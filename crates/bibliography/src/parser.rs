//! `.bib` parser with byte-exact spans and skip-to-next-`@` recovery.
//!
//! Grammar (BibTeX as documented in btxdoc/btxhak, no biblatex extensions):
//!
//! ```text
//! file      := (junk | item)*                 junk: anything up to the next '@'
//! item      := '@' ident ws* ( entry | string | preamble | comment )
//! entry     := open ws* key ws* (',' ws* field)* ','? ws* close
//! field     := ident ws* '=' ws* value
//! string    := open ws* ident ws* '=' ws* value ws* close
//! preamble  := open ws* value ws* close
//! comment   := balanced group after '@comment' if one follows, else the rest of the line
//! value     := part (ws* '#' ws* part)*
//! part      := '{' balanced '}' | '"' text-with-balanced-braces '"' | digits | macro-ident
//! open/close:= '{' '}' | '(' ')'
//! ```
//!
//! Entry types, field names, and macro names are case-insensitive. Keys are
//! compared case-insensitively too (BibTeX warns on case mismatch; here the
//! second spelling is a duplicate). On a syntax error the parser reports one
//! diagnostic whose `recovery` names the byte it resumed at, drops the item,
//! and resumes at the next `@`.

use crate::diagnostics::{Diagnostic, Span};
use crate::model::{Database, Entry, Field, Fields, Macro, Preamble};

/// The month macros every standard `.bst` predefines. A `@string` in the file
/// with the same name overrides them.
pub const MONTH_MACROS: &[(&str, &str)] = &[
    ("jan", "January"),
    ("feb", "February"),
    ("mar", "March"),
    ("apr", "April"),
    ("may", "May"),
    ("jun", "June"),
    ("jul", "July"),
    ("aug", "August"),
    ("sep", "September"),
    ("oct", "October"),
    ("nov", "November"),
    ("dec", "December"),
];

/// Parse a `.bib` source. Syntax diagnostics (including unresolved macros)
/// are attached; semantic validation is [`crate::model::validate`], which
/// [`crate::load`] runs for you.
pub fn parse(src: &str) -> Database {
    let mut p = Parser {
        src,
        bytes: src.as_bytes(),
        pos: 0,
        db: Database::default(),
        macros: MONTH_MACROS
            .iter()
            .map(|(n, v)| ((*n).to_string(), (*v).to_string()))
            .collect(),
    };
    p.parse_file();
    p.db
}

struct Parser<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    db: Database,
    /// Lower-cased macro name → value, latest definition wins.
    macros: Vec<(String, String)>,
}

/// A syntax error at a position; the parser converts it into a diagnostic
/// with a recovery note once it knows where it resumed.
struct SyntaxError {
    message: String,
    span: Span,
}

type PResult<T> = Result<T, SyntaxError>;

fn err<T>(message: impl Into<String>, span: Span) -> PResult<T> {
    Err(SyntaxError {
        message: message.into(),
        span,
    })
}

fn is_ident_byte(b: u8) -> bool {
    // BibTeX: anything but whitespace and "#%'(),={}"; also exclude '"'.
    !(b.is_ascii_whitespace() || b"\"#%'(),={}".contains(&b)) && b != b'@'
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn at_end(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b) if b.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }

    /// Span of the byte at `pos` (or an empty span at end of input).
    fn here(&self) -> Span {
        let end = self.char_end(self.pos);
        Span::new(self.pos, end)
    }

    /// End byte of the UTF-8 character starting at `at`.
    fn char_end(&self, at: usize) -> usize {
        if at >= self.bytes.len() {
            return at;
        }
        let mut end = at + 1;
        while end < self.bytes.len() && (self.bytes[end] & 0xC0) == 0x80 {
            end += 1;
        }
        end
    }

    fn describe_here(&self) -> String {
        match self.peek() {
            None => "end of input".to_string(),
            Some(_) => {
                let s = self.here();
                format!("'{}'", s.slice(self.src).escape_debug())
            }
        }
    }

    fn parse_file(&mut self) {
        loop {
            // Skip junk to the next '@'.
            while let Some(b) = self.peek() {
                if b == b'@' {
                    break;
                }
                self.pos += 1;
            }
            if self.at_end() {
                break;
            }
            let item_start = self.pos;
            match self.parse_item(item_start) {
                Ok(()) => {}
                Err(e) => {
                    // Resume at the next '@' strictly after the error position.
                    let resume_from = e.span.start.max(item_start) + 1;
                    let next_at = self.bytes[resume_from.min(self.bytes.len())..]
                        .iter()
                        .position(|&b| b == b'@')
                        .map(|i| i + resume_from);
                    self.pos = next_at.unwrap_or(self.bytes.len());
                    let recovery = match next_at {
                        Some(at) => format!(
                            "dropped the item starting at byte {item_start} and resumed at the next '@' (byte {at})"
                        ),
                        None => format!(
                            "dropped the item starting at byte {item_start}; no later '@' to resume at"
                        ),
                    };
                    self.db.diagnostics.push(Diagnostic::error(
                        e.message,
                        Some(e.span),
                        Some(recovery),
                    ));
                }
            }
        }
    }

    fn parse_item(&mut self, item_start: usize) -> PResult<()> {
        debug_assert_eq!(self.peek(), Some(b'@'));
        self.pos += 1;
        self.skip_ws();
        let type_span = self.read_ident();
        if type_span.is_empty() {
            return err(
                format!(
                    "expected an entry type after '@', found {}",
                    self.describe_here()
                ),
                self.here(),
            );
        }
        let type_name = type_span.slice(self.src).to_ascii_lowercase();
        self.skip_ws();
        match type_name.as_str() {
            "comment" => {
                self.parse_comment();
                Ok(())
            }
            "preamble" => self.parse_preamble(item_start),
            "string" => self.parse_string(item_start),
            _ => self.parse_entry(item_start, type_name, type_span),
        }
    }

    fn read_ident(&mut self) -> Span {
        let start = self.pos;
        while matches!(self.peek(), Some(b) if is_ident_byte(b)) {
            self.pos += 1;
        }
        Span::new(start, self.pos)
    }

    /// Returns the matching close delimiter after consuming the open one.
    fn open_delim(&mut self, what: &str) -> PResult<u8> {
        match self.peek() {
            Some(b'{') => {
                self.pos += 1;
                Ok(b'}')
            }
            Some(b'(') => {
                self.pos += 1;
                Ok(b')')
            }
            _ => err(
                format!(
                    "expected '{{' or '(' to open {what}, found {}",
                    self.describe_here()
                ),
                self.here(),
            ),
        }
    }

    fn expect_close(&mut self, close: u8, what: &str) -> PResult<()> {
        self.skip_ws();
        if self.peek() == Some(close) {
            self.pos += 1;
            Ok(())
        } else {
            err(
                format!(
                    "expected '{}' to close {what}, found {}",
                    close as char,
                    self.describe_here()
                ),
                self.here(),
            )
        }
    }

    fn parse_comment(&mut self) {
        match self.peek() {
            Some(b'{') | Some(b'(') => {
                let open = self.peek().unwrap_or(b'{');
                let close = if open == b'{' { b'}' } else { b')' };
                self.pos += 1;
                let mut depth = 1usize;
                while let Some(b) = self.peek() {
                    self.pos += 1;
                    if b == open {
                        depth += 1;
                    } else if b == close {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                }
            }
            _ => {
                while let Some(b) = self.peek() {
                    self.pos += 1;
                    if b == b'\n' {
                        break;
                    }
                }
            }
        }
    }

    fn parse_preamble(&mut self, item_start: usize) -> PResult<()> {
        let close = self.open_delim("@preamble")?;
        self.skip_ws();
        let (value, _) = self.parse_value()?;
        self.expect_close(close, "@preamble")?;
        self.db.preambles.push(Preamble {
            value,
            span: Span::new(item_start, self.pos),
        });
        Ok(())
    }

    fn parse_string(&mut self, item_start: usize) -> PResult<()> {
        let close = self.open_delim("@string")?;
        self.skip_ws();
        let name_span = self.read_ident();
        if name_span.is_empty() {
            return err(
                format!(
                    "expected a macro name in @string, found {}",
                    self.describe_here()
                ),
                self.here(),
            );
        }
        self.skip_ws();
        if self.peek() != Some(b'=') {
            return err(
                format!(
                    "expected '=' after macro name, found {}",
                    self.describe_here()
                ),
                self.here(),
            );
        }
        self.pos += 1;
        self.skip_ws();
        let (value, _) = self.parse_value()?;
        self.expect_close(close, "@string")?;
        let name = name_span.slice(self.src).to_ascii_lowercase();
        self.define_macro(&name, &value);
        self.db.macros.push(Macro {
            name,
            value,
            span: Span::new(item_start, self.pos),
        });
        Ok(())
    }

    fn define_macro(&mut self, name: &str, value: &str) {
        if let Some(slot) = self.macros.iter_mut().find(|(n, _)| n == name) {
            slot.1 = value.to_string();
        } else {
            self.macros.push((name.to_string(), value.to_string()));
        }
    }

    fn lookup_macro(&self, name: &str) -> Option<&str> {
        self.macros
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    fn parse_entry(
        &mut self,
        item_start: usize,
        entry_type: String,
        type_span: Span,
    ) -> PResult<()> {
        let close = self.open_delim(&format!("@{entry_type}"))?;
        self.skip_ws();
        let key_start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_whitespace() || b == b',' || b == close || b == b'{' || b == b'}' {
                break;
            }
            self.pos += 1;
        }
        let key_span = Span::new(key_start, self.pos);
        if key_span.is_empty() {
            return err(
                format!(
                    "expected a citation key after '@{entry_type}{{', found {}",
                    self.describe_here()
                ),
                self.here(),
            );
        }
        let key = key_span.slice(self.src).to_string();
        let mut fields = Fields::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b) if b == close => {
                    self.pos += 1;
                    break;
                }
                Some(b',') => {
                    self.pos += 1;
                }
                _ => {
                    return err(
                        format!(
                            "expected ',' or '{}' in entry '{key}', found {}",
                            close as char,
                            self.describe_here()
                        ),
                        self.here(),
                    );
                }
            }
            self.skip_ws();
            if self.peek() == Some(close) {
                self.pos += 1;
                break;
            }
            let name_span = self.read_ident();
            if name_span.is_empty() {
                return err(
                    format!(
                        "expected a field name in entry '{key}', found {}",
                        self.describe_here()
                    ),
                    self.here(),
                );
            }
            self.skip_ws();
            if self.peek() != Some(b'=') {
                return err(
                    format!(
                        "expected '=' after field '{}' in entry '{key}', found {}",
                        name_span.slice(self.src),
                        self.describe_here()
                    ),
                    self.here(),
                );
            }
            self.pos += 1;
            self.skip_ws();
            let (value, value_span) = self.parse_value()?;
            let name = name_span.slice(self.src).to_ascii_lowercase();
            let field = Field {
                name: name.clone(),
                value,
                span: name_span.join(value_span),
                name_span,
                value_span,
            };
            if !fields.insert(field) {
                self.db.diagnostics.push(Diagnostic::warning(
                    format!("repeated field '{name}' in entry '{key}'"),
                    Some(name_span.join(value_span)),
                    Some("ignored the repeated field and kept the first".into()),
                ));
            }
        }
        self.db.entries.push(Entry {
            key,
            key_span,
            entry_type,
            type_span,
            fields,
            span: Span::new(item_start, self.pos),
        });
        Ok(())
    }

    /// Parses `part (# part)*`; returns the resolved, whitespace-collapsed
    /// value and the span from the first part to the end of the last.
    fn parse_value(&mut self) -> PResult<(String, Span)> {
        let start = self.pos;
        let mut raw = String::new();
        loop {
            self.parse_part(&mut raw)?;
            let after_part = self.pos;
            self.skip_ws();
            if self.peek() == Some(b'#') {
                self.pos += 1;
                self.skip_ws();
                continue;
            }
            self.pos = after_part;
            break;
        }
        Ok((collapse_whitespace(&raw), Span::new(start, self.pos)))
    }

    fn parse_part(&mut self, out: &mut String) -> PResult<()> {
        match self.peek() {
            Some(b'{') => {
                let start = self.pos;
                self.pos += 1;
                let mut depth = 1usize;
                while let Some(b) = self.peek() {
                    match b {
                        b'{' => depth += 1,
                        b'}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    self.pos += 1;
                }
                if self.at_end() {
                    return err(
                        "unterminated '{' value (no matching '}')",
                        Span::new(start, start + 1),
                    );
                }
                out.push_str(&self.src[start + 1..self.pos]);
                self.pos += 1;
                Ok(())
            }
            Some(b'"') => {
                let start = self.pos;
                self.pos += 1;
                let mut depth = 0usize;
                while let Some(b) = self.peek() {
                    match b {
                        b'{' => depth += 1,
                        b'}' => depth = depth.saturating_sub(1),
                        b'"' if depth == 0 => break,
                        _ => {}
                    }
                    self.pos += 1;
                }
                if self.at_end() {
                    return err(
                        "unterminated '\"' value (no closing quote at brace depth 0)",
                        Span::new(start, start + 1),
                    );
                }
                out.push_str(&self.src[start + 1..self.pos]);
                self.pos += 1;
                Ok(())
            }
            Some(b) if b.is_ascii_digit() => {
                let start = self.pos;
                while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
                    self.pos += 1;
                }
                out.push_str(&self.src[start..self.pos]);
                Ok(())
            }
            Some(b) if is_ident_byte(b) => {
                let span = self.read_ident();
                let name = span.slice(self.src);
                match self.lookup_macro(name) {
                    Some(v) => out.push_str(v),
                    None => self.db.diagnostics.push(Diagnostic::warning(
                        format!("undefined macro '{name}'"),
                        Some(span),
                        Some("substituted the empty string".into()),
                    )),
                }
                Ok(())
            }
            _ => err(
                format!(
                    "expected a field value ('{{', '\"', a number, or a macro name), found {}",
                    self.describe_here()
                ),
                self.here(),
            ),
        }
    }
}

/// Collapse whitespace runs to one space and trim the ends, as BibTeX does
/// when it reads a field value.
fn collapse_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_ws = false;
    for c in s.chars() {
        if c.is_whitespace() {
            in_ws = true;
        } else {
            if in_ws && !out.is_empty() {
                out.push(' ');
            }
            in_ws = false;
            out.push(c);
        }
    }
    out
}
