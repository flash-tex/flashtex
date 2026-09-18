//! Light reference scanner: finds `\input`, `\include`, `\bibliography`,
//! `\addbibresource` and `\includegraphics` with UTF-8 byte spans.
//!
//! This is not the compiler. It does no macro expansion; it skips `%`
//! comments, `\verb` and verbatim-like environments, and treats a command
//! name as the maximal run of ASCII letters so `\inputfoo` never matches
//! `\input`. Arguments containing `\` or `#` are reported with
//! `literal == false` because they cannot be resolved without expansion.

/// Zero-based, end-exclusive UTF-8 byte range (runtime-v1 `source` convention).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteSpan {
    pub start: usize,
    pub end: usize,
}

impl ByteSpan {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReferenceKind {
    /// `\input{file}` or `\input file` (implicit `.tex`, then literal name).
    Input,
    /// `\include{file}` (always `file.tex`).
    Include,
    /// `\bibliography{a,b}` (one reference per item, `.bib` then `.bbl` appended).
    Bibliography,
    /// `\addbibresource{file.bib}` (biblatex; literal name, `.bib` if missing).
    AddBibResource,
    /// `\includegraphics[opts]{file}` (asset; extension may be omitted).
    IncludeGraphics,
}

impl ReferenceKind {
    pub fn command(self) -> &'static str {
        match self {
            ReferenceKind::Input => "input",
            ReferenceKind::Include => "include",
            ReferenceKind::Bibliography => "bibliography",
            ReferenceKind::AddBibResource => "addbibresource",
            ReferenceKind::IncludeGraphics => "includegraphics",
        }
    }

    fn from_command(name: &str) -> Option<Self> {
        Some(match name {
            "input" => ReferenceKind::Input,
            "include" => ReferenceKind::Include,
            "bibliography" => ReferenceKind::Bibliography,
            "addbibresource" => ReferenceKind::AddBibResource,
            "includegraphics" => ReferenceKind::IncludeGraphics,
            _ => return None,
        })
    }
}

/// One discovered reference in a source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub kind: ReferenceKind,
    /// The referenced name as written (whitespace-trimmed; one list item for
    /// `\bibliography`).
    pub argument: String,
    /// Span of the whole command from the backslash through the closing brace
    /// (or the end of the bare name for `\input name`).
    pub span: ByteSpan,
    /// Span of `argument` itself, for diagnostics.
    pub argument_span: ByteSpan,
    /// False when the argument contains `\` or `#` and needs expansion.
    pub literal: bool,
}

const VERBATIM_ENVS: &[&str] = &[
    "verbatim",
    "verbatim*",
    "comment",
    "lstlisting",
    "minted",
    "Verbatim",
];

/// Scans `text` and returns references in source order.
pub fn scan_references(text: &str) -> Vec<Reference> {
    Scanner {
        text,
        bytes: text.as_bytes(),
        pos: 0,
        out: Vec::new(),
    }
    .run()
}

struct Scanner<'a> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
    out: Vec<Reference>,
}

impl<'a> Scanner<'a> {
    fn run(mut self) -> Vec<Reference> {
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b'%' => self.skip_line(),
                b'\\' => self.command(),
                _ => self.pos += 1,
            }
        }
        self.out
    }

    fn skip_line(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos] != b'\n' {
            self.pos += 1;
        }
    }

    fn char_len_at(&self, i: usize) -> usize {
        self.text[i..].chars().next().map_or(1, char::len_utf8)
    }

    fn command(&mut self) {
        let start = self.pos;
        self.pos += 1;
        let name_start = self.pos;
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_alphabetic() {
            self.pos += 1;
        }
        if self.pos == name_start {
            // Control symbol such as `\%`, `\\`, `\{`: skip the symbol character.
            if self.pos < self.bytes.len() {
                self.pos += self.char_len_at(self.pos);
            }
            return;
        }
        let name = &self.text[name_start..self.pos];
        match name {
            "verb" => self.skip_verb(),
            "begin" => self.maybe_skip_verbatim_env(),
            _ => {
                if let Some(kind) = ReferenceKind::from_command(name) {
                    self.reference(kind, start);
                }
            }
        }
    }

    fn skip_verb(&mut self) {
        // `\verb*?<delim>...<delim>`
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'*' {
            self.pos += 1;
        }
        if self.pos >= self.bytes.len() {
            return;
        }
        let delim_len = self.char_len_at(self.pos);
        let delim = &self.text[self.pos..self.pos + delim_len];
        self.pos += delim_len;
        match self.text[self.pos..].find(delim) {
            Some(off) => self.pos += off + delim_len,
            None => self.pos = self.bytes.len(),
        }
    }

    fn maybe_skip_verbatim_env(&mut self) {
        let save = self.pos;
        self.skip_ws();
        let Some(inner) = self.braced_span() else {
            self.pos = save;
            return;
        };
        let env = self.text[inner.start..inner.end].trim();
        if !VERBATIM_ENVS.contains(&env) {
            self.pos = save;
            return;
        }
        let end_marker = format!("\\end{{{env}}}");
        match self.text[self.pos..].find(&end_marker) {
            Some(off) => self.pos += off + end_marker.len(),
            None => self.pos = self.bytes.len(),
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    /// If the next byte is `{`, consumes through the matching `}` and returns
    /// the inner span. Braces nest; a backslash escapes the following char.
    fn braced_span(&mut self) -> Option<ByteSpan> {
        if self.pos >= self.bytes.len() || self.bytes[self.pos] != b'{' {
            return None;
        }
        let inner_start = self.pos + 1;
        let mut depth = 0usize;
        let mut i = self.pos;
        while i < self.bytes.len() {
            match self.bytes[i] {
                b'\\' => {
                    i += 1;
                    if i < self.bytes.len() {
                        i += self.char_len_at(i);
                    }
                    continue;
                }
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        self.pos = i + 1;
                        return Some(ByteSpan::new(inner_start, i));
                    }
                }
                _ => {}
            }
            i += 1;
        }
        // Unbalanced: consume to end so we do not loop forever.
        self.pos = self.bytes.len();
        None
    }

    fn skip_optional_arg(&mut self) {
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'[' {
            let mut i = self.pos;
            while i < self.bytes.len() && self.bytes[i] != b']' && self.bytes[i] != b'\n' {
                i += 1;
            }
            if i < self.bytes.len() && self.bytes[i] == b']' {
                self.pos = i + 1;
            }
        }
    }

    fn reference(&mut self, kind: ReferenceKind, start: usize) {
        let after_name = self.pos;
        self.skip_ws();
        self.skip_optional_arg();
        self.skip_ws();
        if let Some(inner) = self.braced_span() {
            let end = self.pos;
            match kind {
                ReferenceKind::Bibliography => {
                    let mut item_start = inner.start;
                    let inner_text = &self.text[inner.start..inner.end];
                    for item in inner_text.split(',') {
                        let item_end = item_start + item.len();
                        self.push(kind, item_start, item_end, start, end);
                        item_start = item_end + 1;
                    }
                }
                _ => self.push(kind, inner.start, inner.end, start, end),
            }
            return;
        }
        // Bare `\input name` (TeX primitive form). Other commands require braces.
        if kind == ReferenceKind::Input {
            let name_start = self.pos;
            while self.pos < self.bytes.len() {
                let b = self.bytes[self.pos];
                if b.is_ascii_whitespace() || matches!(b, b'\\' | b'%' | b'{' | b'}') {
                    break;
                }
                self.pos += self.char_len_at(self.pos);
            }
            if self.pos > name_start {
                let end = self.pos;
                self.push(kind, name_start, end, start, end);
                return;
            }
        }
        self.pos = after_name;
    }

    fn push(
        &mut self,
        kind: ReferenceKind,
        arg_start: usize,
        arg_end: usize,
        start: usize,
        end: usize,
    ) {
        let raw = &self.text[arg_start..arg_end];
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return;
        }
        let lead = raw.len() - raw.trim_start().len();
        let arg_span = ByteSpan::new(arg_start + lead, arg_start + lead + trimmed.len());
        self.out.push(Reference {
            kind,
            argument: trimmed.to_string(),
            span: ByteSpan::new(start, end),
            argument_span: arg_span,
            literal: !trimmed.contains(['\\', '#']),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &str) -> Vec<(ReferenceKind, String)> {
        scan_references(text)
            .into_iter()
            .map(|r| (r.kind, r.argument))
            .collect()
    }

    #[test]
    fn finds_basic_references() {
        let text = "\\input{a} \\include{ch/b} \\bibliography{x, y} \\includegraphics[width=2cm]{fig} \\addbibresource{r.bib}";
        assert_eq!(
            args(text),
            vec![
                (ReferenceKind::Input, "a".into()),
                (ReferenceKind::Include, "ch/b".into()),
                (ReferenceKind::Bibliography, "x".into()),
                (ReferenceKind::Bibliography, "y".into()),
                (ReferenceKind::IncludeGraphics, "fig".into()),
                (ReferenceKind::AddBibResource, "r.bib".into()),
            ]
        );
    }

    #[test]
    fn spans_are_utf8_bytes() {
        let text = "café \\input{naïve}";
        let refs = scan_references(text);
        assert_eq!(refs.len(), 1);
        let r = &refs[0];
        assert_eq!(&text[r.span.start..r.span.end], "\\input{naïve}");
        assert_eq!(&text[r.argument_span.start..r.argument_span.end], "naïve");
        assert_eq!(r.span.start, "café ".len());
    }

    #[test]
    fn skips_comments_verb_and_verbatim() {
        let text = "% \\input{no}\n\\verb|\\input{no}| \\begin{verbatim}\\input{no}\\end{verbatim} \\input{yes} \\% \\input{also}";
        assert_eq!(
            args(text),
            vec![
                (ReferenceKind::Input, "yes".into()),
                (ReferenceKind::Input, "also".into())
            ]
        );
    }

    #[test]
    fn command_name_boundary_and_bare_input() {
        assert_eq!(
            args("\\inputfoo{x} \\input bare.tex\\relax"),
            vec![(ReferenceKind::Input, "bare.tex".into())]
        );
        assert!(args("\\include").is_empty());
    }

    #[test]
    fn non_literal_argument_flagged() {
        let refs = scan_references("\\input{\\jobname}");
        assert_eq!(refs.len(), 1);
        assert!(!refs[0].literal);
    }

    #[test]
    fn bibliography_item_spans() {
        let text = "\\bibliography{ one ,two}";
        let refs = scan_references(text);
        assert_eq!(
            &text[refs[0].argument_span.start..refs[0].argument_span.end],
            "one"
        );
        assert_eq!(
            &text[refs[1].argument_span.start..refs[1].argument_span.end],
            "two"
        );
        assert_eq!(&text[refs[1].span.start..refs[1].span.end], text);
    }
}
