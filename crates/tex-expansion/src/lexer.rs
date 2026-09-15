//! TeX's tokenizer (TeXbook ch. 8, "The eyes and the mouth of TeX"):
//! turns raw source bytes into tokens, respecting the current catcode
//! table, the three lexer states (N = new line, M = mid line, S =
//! skipping blanks), `^^` notation, and comments.

use std::cell::Cell;
use std::rc::Rc;

use crate::catcode::{CatCode, CatCodeTable};
use crate::span::Span;
use crate::token::{Token, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Start of line: an end-of-line character here produces `\par`.
    NewLine,
    /// Middle of a line: an end-of-line character here produces a space
    /// token (catcode 10 with a single space character), unless the
    /// line's catcode-10-equivalent trailing char was already consumed.
    MidLine,
    /// Skipping blanks: spaces (and end-of-line) are swallowed until a
    /// non-space is found. Entered right after a control word or an
    /// explicit space token.
    SkipBlanks,
}

/// The lexer owns its buffer (shared via `Rc` so that cloning a lexer for
/// an incremental checkpoint is O(1)); `\scantokens` pseudo-files get
/// their own buffer and `source_id`.
#[derive(Debug, Clone)]
pub struct Lexer {
    src: Rc<str>,
    pos: usize,
    source_id: u32,
    state: State,
    /// A byte offset known to hold the first non-space character of the
    /// current physical line's remaining run of spaces (see
    /// [`Lexer::rest_of_line_blank`]), so a run of `k` spaces is scanned
    /// once, not `k` times.
    nonblank_at: Cell<Option<usize>>,
}

impl Lexer {
    pub fn new(src: Rc<str>, source_id: u32) -> Self {
        Lexer { src, pos: 0, source_id, state: State::NewLine, nonblank_at: Cell::new(None) }
    }

    /// Continue lexing `src` from byte `pos` in lexer state `state` (used
    /// when re-expanding from an incremental checkpoint over an edited
    /// buffer: everything before `pos` is unchanged by construction).
    pub fn resume(src: Rc<str>, source_id: u32, pos: usize, state: State) -> Self {
        Lexer { src, pos, source_id, state, nonblank_at: Cell::new(None) }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn src(&self) -> &str {
        &self.src
    }

    pub fn at_end(&self) -> bool {
        self.pos >= self.src.len()
    }

    /// 1-based line number of byte offset `pos` (for TeX-style "after
    /// line N" diagnostics; only computed on the error path).
    pub fn line_of(&self, pos: usize) -> usize {
        let end = pos.min(self.src.len());
        1 + self.src.as_bytes()[..end].iter().filter(|&&b| b == b'\n').count()
    }

    fn peek_char(&self) -> Option<(char, usize)> {
        if self.pos >= self.src.len() {
            return None;
        }
        let rest = &self.src[self.pos..];
        rest.chars().next().map(|c| (c, c.len_utf8()))
    }

    /// Apply TeX's `^^` notation (TeXbook p. 45): `^^X` for printable X in
    /// a restricted range, and `^^xy` for two lowercase hex digits. Returns
    /// the resulting single character and how many source bytes it
    /// consumed, if a `^^` sequence was recognized at `pos`.
    fn try_superscript_notation(&self, cat_table: &CatCodeTable, pos: usize) -> Option<(char, usize)> {
        let rest = self.src.get(pos..)?;
        let mut chars = rest.char_indices();
        let (_, c0) = chars.next()?;
        if cat_table.get(c0) != CatCode::Superscript {
            return None;
        }
        let (_i1, c1) = chars.next()?;
        if c1 != c0 {
            return None;
        }
        let (i2, c2) = chars.next()?;
        // two-hex-digit form
        if let Some((i3, c3)) = chars.clone().next() {
            if c2.is_ascii_hexdigit() && c2.is_ascii_lowercase_hexish() && c3.is_ascii_hexdigit() && c3.is_ascii_lowercase_hexish()
            {
                let byte = u8::from_str_radix(&format!("{c2}{c3}"), 16).ok()?;
                // Indices are relative to `rest`, so `end` is the length.
                let end = i3 + c3.len_utf8();
                return Some((byte as char, end));
            }
        }
        // single-char form: char code c2 XOR 64 if < 128, else + 64
        if (c2 as u32) < 128 {
            let code = (c2 as u32) ^ 64;
            let ch = char::from_u32(code)?;
            let end = i2 + c2.len_utf8();
            return Some((ch, end));
        }
        None
    }

    fn skip_comment_to_eol(&mut self) {
        while let Some((c, len)) = self.peek_char() {
            if c == '\n' {
                break;
            }
            self.pos += len;
        }
    }

    /// Read the rest of a control sequence name after the escape char has
    /// been consumed. Per TeXbook: a control word is a maximal run of
    /// catcode-11 (letter) characters; a control symbol is exactly one
    /// non-letter character (which may itself be a space, ending as a
    /// control-symbol named " "). An empty following (immediate EOL) is
    /// the "null control sequence" `\csname` builds sometimes; treated
    /// as an empty-name control word.
    fn read_cs_name(&mut self, cat_table: &CatCodeTable, endlinechar: i64) -> (String, State) {
        let mut name = String::new();
        // `^^` notation is resolved inside names too (`\^^M`, `\^^J`).
        let peek = |me: &Self| me.try_superscript_notation(cat_table, me.pos).or_else(|| me.peek_char());
        // A *physical* line break right after the escape character (not a
        // `^^J`/`^^M` written in notation, which is an ordinary symbol).
        let physical_break = matches!(self.peek_char(), Some(('\n' | '\r', _)));
        match peek(self) {
            None => (name, State::MidLine),
            Some((c @ ('\n' | '\r'), len)) if physical_break => {
                // `\` at the end of a line: the name is the \endlinechar
                // (control symbol `\^^M`), or empty when there is none.
                self.pos += len;
                if c == '\r' && self.src.as_bytes().get(self.pos) == Some(&b'\n') {
                    self.pos += 1;
                }
                if let Some(e) = char::from_u32(endlinechar as u32).filter(|_| (0..256).contains(&endlinechar)) {
                    name.push(e);
                }
                (name, State::NewLine)
            }
            Some((c, len)) => {
                let cat = cat_table.get(c);
                if cat == CatCode::Letter {
                    while let Some((c, len)) = peek(self) {
                        if cat_table.get(c) == CatCode::Letter {
                            name.push(c);
                            self.pos += len;
                        } else {
                            break;
                        }
                    }
                    (name, State::SkipBlanks)
                } else {
                    name.push(c);
                    self.pos += len;
                    let next_state = if cat == CatCode::Space { State::SkipBlanks } else { State::MidLine };
                    (name, next_state)
                }
            }
        }
    }

    /// Are the bytes from the current position to the end of the physical
    /// line all spaces? (TeX's `input_ln` strips trailing spaces.)
    ///
    /// Called at every space, so the answer is remembered: when the scan
    /// from `pos` stops at a non-space byte `j`, every later position up to
    /// `j` sees the same spaces and then `j`, and is not blank either.
    /// Without that, `k` spaces before a non-space cost `k²/2` byte
    /// compares (a blanked float body in the render pipeline puts a whole
    /// document's worth of spaces on one line).
    fn rest_of_line_blank(&self) -> bool {
        if self.nonblank_at.get().is_some_and(|j| self.pos <= j) {
            return false;
        }
        let bytes = self.src.as_bytes();
        match bytes[self.pos..].iter().position(|&b| b != b' ') {
            Some(offset) if !matches!(bytes[self.pos + offset], b'\n' | b'\r') => {
                self.nonblank_at.set(Some(self.pos + offset));
                false
            }
            _ => true,
        }
    }

    /// Skip the rest of the physical line including its line break
    /// (`\n`, `\r\n` or `\r`).
    fn discard_rest_of_line(&mut self) {
        self.skip_comment_to_eol();
        match self.peek_char() {
            Some(('\r', _)) => {
                self.pos += 1;
                if self.src.as_bytes().get(self.pos) == Some(&b'\n') {
                    self.pos += 1;
                }
            }
            Some(('\n', _)) => self.pos += 1,
            _ => {}
        }
    }

    /// Produce the next token, given the current catcode table (owned by
    /// the caller so `\catcode` assignments made mid-stream take effect
    /// immediately, as real TeX requires) and `\endlinechar`.
    ///
    /// Each physical line break (`\n`, `\r\n` or `\r`) stands for the
    /// `\endlinechar` TeX appends to every line -- processed with *its*
    /// catcode (normally `^^M`, end of line) -- or for nothing when
    /// `\endlinechar` is outside 0..255. Trailing spaces of a line are
    /// ignored, as TeX's `input_ln` strips them.
    pub fn next_token(&mut self, cat_table: &CatCodeTable, endlinechar: i64) -> Option<Token> {
        loop {
            let start = self.pos;
            let prev_state = self.state;
            let mut line_end = false;
            // Resolve `^^` notation into an effective character + length
            // before catcode lookup, so `^^41` etc. behave like the literal
            // character for catcode purposes.
            let (ch, raw_len) = match self.peek_char() {
                None => return None,
                Some((c @ ('\n' | '\r'), len)) => {
                    let n = if c == '\r' && self.src.as_bytes().get(self.pos + 1) == Some(&b'\n') { len + 1 } else { len };
                    match char::from_u32(endlinechar as u32).filter(|_| (0..256).contains(&endlinechar)) {
                        Some(e) => {
                            line_end = true;
                            (e, n)
                        }
                        None => {
                            self.pos += n;
                            self.state = State::NewLine;
                            continue;
                        }
                    }
                }
                Some((' ', _)) if self.rest_of_line_blank() => {
                    while self.src.as_bytes().get(self.pos) == Some(&b' ') {
                        self.pos += 1;
                    }
                    continue;
                }
                Some((c, len)) => {
                    if let Some((rc, rlen)) = self.try_superscript_notation(cat_table, self.pos) {
                        (rc, rlen)
                    } else {
                        (c, len)
                    }
                }
            };
            let cat = cat_table.get(ch);
            if line_end && cat != CatCode::EndLine {
                // An \endlinechar with a non-end-of-line catcode (e.g.
                // `\catcode`\^^M=13` under \obeylines) is an ordinary
                // character ending the line; the next line starts in N.
                self.pos += raw_len;
                self.state = State::NewLine;
                let span = Span::new(self.source_id, start as u32, self.pos as u32);
                match cat {
                    CatCode::Escape => return Some(Token::new(TokenKind::ControlSequence(String::new()), span)),
                    CatCode::Space if prev_state == State::MidLine => {
                        return Some(Token::new(TokenKind::Char(' ', CatCode::Space), span))
                    }
                    CatCode::Space | CatCode::Comment | CatCode::Ignored | CatCode::Invalid => continue,
                    CatCode::Active => return Some(Token::new(TokenKind::ActiveChar(ch), span)),
                    other => return Some(Token::new(TokenKind::Char(ch, other), span)),
                }
            }
            match cat {
                CatCode::Escape => {
                    self.pos += raw_len;
                    let (name, next_state) = self.read_cs_name(cat_table, endlinechar);
                    self.state = next_state;
                    let span = Span::new(self.source_id, start as u32, self.pos as u32);
                    return Some(Token::new(TokenKind::ControlSequence(name), span));
                }
                CatCode::EndLine => {
                    self.pos += raw_len;
                    if !line_end {
                        // A catcode-5 character inside a line (`^^M`
                        // typed as such) ends it: the rest is discarded.
                        self.discard_rest_of_line();
                    }
                    let span = Span::new(self.source_id, start as u32, self.pos as u32);
                    let out = match self.state {
                        State::NewLine => {
                            self.state = State::NewLine;
                            Some(Token::new(TokenKind::ControlSequence("par".to_string()), span))
                        }
                        State::MidLine => {
                            self.state = State::NewLine;
                            Some(Token::new(TokenKind::Char(' ', CatCode::Space), span))
                        }
                        State::SkipBlanks => {
                            self.state = State::NewLine;
                            continue;
                        }
                    };
                    return out;
                }
                CatCode::Space => {
                    self.pos += raw_len;
                    match self.state {
                        State::MidLine => {
                            self.state = State::SkipBlanks;
                            let span = Span::new(self.source_id, start as u32, self.pos as u32);
                            return Some(Token::new(TokenKind::Char(' ', CatCode::Space), span));
                        }
                        State::NewLine | State::SkipBlanks => continue,
                    }
                }
                CatCode::Comment => {
                    self.pos += raw_len;
                    self.skip_comment_to_eol();
                    // A comment discards the rest of the line *including*
                    // its end-of-line character (TeXbook p. 47: "the rest
                    // of the line is thrown away"), so no space or `\par`
                    // is produced for it; the next line starts in state N.
                    self.discard_rest_of_line();
                    self.state = State::NewLine;
                    continue;
                }
                CatCode::Ignored => {
                    self.pos += raw_len;
                    continue;
                }
                CatCode::Invalid => {
                    self.pos += raw_len;
                    // Real TeX raises "Text line contains an invalid
                    // character"; we skip it rather than panicking, the
                    // caller surfaces this as a diagnostic upstream.
                    continue;
                }
                CatCode::Active => {
                    self.pos += raw_len;
                    self.state = State::MidLine;
                    let span = Span::new(self.source_id, start as u32, self.pos as u32);
                    return Some(Token::new(TokenKind::ActiveChar(ch), span));
                }
                other => {
                    self.pos += raw_len;
                    self.state = State::MidLine;
                    let span = Span::new(self.source_id, start as u32, self.pos as u32);
                    return Some(Token::new(TokenKind::Char(ch, other), span));
                }
            }
        }
    }

    pub fn source_id(&self) -> u32 {
        self.source_id
    }

    pub fn byte_pos(&self) -> usize {
        self.pos
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    /// `\endinput`: stop reading this buffer.
    pub fn finish(&mut self) {
        self.pos = self.src.len();
    }

    /// Read raw characters (ignoring catcodes) up to and excluding the
    /// first occurrence of `delim`, consuming the delimiter. Used for
    /// `\verb`. Returns `None` (consuming nothing further) if the line
    /// ends first, matching LaTeX's "\verb ended by end of line" error.
    pub fn read_verb_until(&mut self, delim: char) -> Option<String> {
        let rest = &self.src[self.pos..];
        let mut out = String::new();
        for (i, c) in rest.char_indices() {
            if c == delim {
                self.pos += i + c.len_utf8();
                self.state = State::MidLine;
                return Some(out);
            }
            if c == '\n' {
                self.pos += i;
                return None;
            }
            out.push(c);
        }
        self.pos = self.src.len();
        None
    }

    /// Read raw text up to (excluding) the first occurrence of `end`,
    /// consuming it. Used for verbatim environments. Returns `None` if
    /// `end` never occurs (everything to EOF is consumed).
    pub fn read_raw_until_str(&mut self, end: &str) -> Option<String> {
        let rest = &self.src[self.pos..];
        match rest.find(end) {
            Some(i) => {
                let text = rest[..i].to_string();
                self.pos += i + end.len();
                self.state = State::MidLine;
                Some(text)
            }
            None => {
                self.pos = self.src.len();
                None
            }
        }
    }

    /// The first raw character at the current position (for `\verb`'s
    /// delimiter), consumed.
    pub fn read_raw_char(&mut self) -> Option<char> {
        let (c, len) = self.peek_char()?;
        self.pos += len;
        self.state = State::MidLine;
        Some(c)
    }
}

// Small local helper trait to keep the hex-digit check readable without
// pulling in extra deps; `is_ascii_hexdigit` already covers upper+lower,
// but TeX's `^^xy` form only recognizes lowercase hex digits.
trait LowercaseHexish {
    fn is_ascii_lowercase_hexish(&self) -> bool;
}
impl LowercaseHexish for char {
    fn is_ascii_lowercase_hexish(&self) -> bool {
        self.is_ascii_digit() || ('a'..='f').contains(self)
    }
}
