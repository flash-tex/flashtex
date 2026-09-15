//! Macro expansion: a compiler-owned pass in front of the parser.
//!
//! `flashtex-tex-expansion` executes the expansion layer of TeX — category
//! codes, `\def`/`\let`, `\newcommand`/`\newenvironment`, conditionals,
//! registers, counters — over every document the entry `\input`s, and this
//! module turns its output back into the parser's [`Token`] vocabulary. The
//! parser therefore never sees a macro call: `\greet{world}` arrives as the
//! words of its replacement text.
//!
//! Contract kept for the parser and its consumers:
//!
//! - **Pass-through.** Every control sequence the engine does not define
//!   (all typesetting commands: `\section`, `\textbf`, `\hspace`, ...) and
//!   every grouping brace reaches the parser unchanged, with its exact source
//!   span. The parser's built-in command names are declared to the engine as
//!   host commands, so `\newcommand` refuses to redefine them and
//!   `\renewcommand` accepts them, as before.
//! - **Spans.** A token read from a source keeps its exact byte span. A token
//!   of a macro's replacement text carries the span of the outermost macro
//!   invocation in the source (the `\name` control word, as before this pass
//!   existed) *and* its definition span: the bytes inside the definition it
//!   was copied from (see [`ExpandedToken::definition`] and
//!   [`crate::parser::Parsed::expansions`]). Tokens substituted for a
//!   macro's arguments keep their own source spans.
//! - **Verbatim.** `\verb` arguments and the bodies of `verbatim`,
//!   `verbatim*` and `lstlisting` are hidden from the engine before it reads
//!   the source (their bytes are blanked in a private copy; offsets do not
//!   move), so no `%`, `\`, `$`, `{`, `}` or macro in them is interpreted.
//!   The parser keeps reading those regions from the original text.
//! - **Environments.** The engine turns `\begin{name}` into `\name` (so a
//!   `\newenvironment` definition runs); an undefined `\name` produced that
//!   way is turned back into `\begin`, `{`, `name`, `}` with the exact spans
//!   of each piece.
//! - **Diagnostics.** Engine diagnostics (TeX's/LaTeX's own messages) become
//!   compiler diagnostics at the engine's span, mapped into the owning
//!   document. Messages the parser already reports itself (unbalanced
//!   environments, unterminated `\verb`/verbatim) and the engine's
//!   "environment passed through to the typesetter" notes are dropped.
//! - **Tables.** `\arraystretch` is read where LaTeX reads it, at
//!   `\begin{tabular}`: a host prelude makes the engine emit its value there
//!   (see [`HOST_PRELUDE`]), recorded in [`Expansion::arraystretch`].

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

use flashtex_tex_expansion::{self as tex, CatCode, Edit, Engine, IncrementalExpander, Limits, TokenKind as TexKind};

use crate::diagnostics::Diagnostic;
use crate::lexer::{tokenize_document, Token, TokenKind};
use crate::parser::{path_is_safe, SourceDocument, BUILT_INS, INCLUDE_DEPTH_LIMIT};
use crate::{DocumentId, Span};

/// One parser input token with its expansion provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct ExpandedToken {
    pub token: Token,
    /// For a token copied from a macro's replacement text: the bytes of the
    /// definition it came from. `token.span` is then the invocation span.
    pub definition: Option<Span>,
    /// True when `token.span` is an invocation span rather than the token's
    /// own source bytes.
    pub maps_to_invocation: bool,
}

impl std::borrow::Borrow<Token> for ExpandedToken {
    fn borrow(&self) -> &Token {
        &self.token
    }
}

/// A replacement-text run: the invocation it was expanded at, and the
/// definition bytes it was copied from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpansionSite {
    pub invocation: Span,
    pub definition: Span,
}

pub struct Expansion {
    /// Shared with the incremental cache, so a keystroke does not copy the
    /// whole stream.
    pub tokens: Rc<Vec<ExpandedToken>>,
    pub diagnostics: Vec<Diagnostic>,
    /// `\arraystretch`'s replacement text in effect at each
    /// `\begin{tabular}`/`tabular*`/`array`, keyed by that `\begin`'s span.
    pub arraystretch: HashMap<(usize, usize), String>,
    /// `\@currentlabel`'s expansion just after each bare `\refstepcounter`
    /// (which the engine runs with no output tokens, so the parser would
    /// otherwise never hear about it), keyed by the pushed
    /// `flashtexcurrentlabel` token's own span.
    pub current_label_by_marker: HashMap<(usize, usize), String>,
}

/// Host definitions run before the document. `\DeclareMathOperator` is
/// amsopn.sty's definition reduced to its effect: `\cmd` becomes
/// `\operatorname{text}` (`\operatorname*{text}` when starred).
/// LaTeX's `\tabular`/`\array` read `\arraystretch` when the environment begins; here they emit a
/// marker plus `\arraystretch`'s current expansion, which the converter
/// turns back into `\begin{<env>}` and records for the parser. Likewise
/// `\refstepcounter` (which the engine runs with no output tokens) emits a
/// marker plus `\@currentlabel`'s current expansion, recorded in
/// [`Expansion::current_label_by_marker`].
///
/// Kernel definitions that would intercept a command the parser typesets
/// itself (`\\label`, `\\verb`, whose argument the pass has already hidden,
/// `\\:`, which latex.ltx only uses while building `\\@ifnextchar`
/// before redefining it as a math space, and `\\fnsymbol`, whose counter
/// the parser resolves against its own `footnote`/`mpfootnote` counters
/// that the engine never defines) are removed, so they pass through.
///
/// `\\setlength`/`\\addtolength` keep the kernel meaning when `#1` is already
/// defined (a `\\newlength` skip, so `\\the` can read it back). An undefined
/// target (`\\textwidth`, `\\parindent`, `\\fboxsep`, ...) is rewritten to a
/// host command the converter maps back, so the parser sees the original
/// name with its argument still a control sequence, not consumed as a
/// skip assignment, which would yield `\\addtolength{\\}`.
pub const HOST_PRELUDE: &str = "\\let\\label\\flashtexundefined
\\let\\verb\\flashtexundefined
\\let\\:\\flashtexundefined
\\let\\counterwithin\\flashtexundefined
\\let\\counterwithout\\flashtexundefined
\\let\\fnsymbol\\flashtexundefined
\\def\\setlength#1#2{\\ifdefined#1#1 #2\\relax\\else\\flashtexsetlength{#1}{#2}\\fi}%
\\def\\addtolength#1#2{\\ifdefined#1\\advance#1 #2\\relax\\else\\flashtexaddtolength{#1}{#2}\\fi}%
\\long\\def\\flashtexdeclaremathop#1#2#3{\\newcommand#2{\\operatorname#1{#3}}}%
\\expandafter\\def\\expandafter\\DeclareMathOperator\\expandafter{\\csname @ifstar\\endcsname{\\flashtexdeclaremathop*}{\\flashtexdeclaremathop{}}}%
\\def\\arraystretch{1}%
\\def\\tabular{\\flashtexbegintabular\\expandafter{\\arraystretch}}%
\\expandafter\\def\\csname tabular*\\endcsname{\\flashtexbegintabularstar\\expandafter{\\arraystretch}}%
\\def\\array{\\flashtexbeginarray\\expandafter{\\arraystretch}}%
\\makeatletter
\\let\\flashtexrealrefstepcounter\\refstepcounter
\\def\\refstepcounter#1{\\flashtexrealrefstepcounter{#1}\\flashtexcurrentlabelmarker\\expandafter{\\@currentlabel}}%
\\makeatother
";

/// Engine diagnostics that duplicate the parser's own reports, or only note
/// a pass-through.
fn parser_reports_itself(message: &str) -> bool {
    (message.starts_with("Environment ") && message.contains("undefined (passed through"))
        || message.contains("ended by \\end{")
        || message.contains("without matching \\begin")
        || message == "Too many }'s."
        || message.contains("\\verb illegal in command argument")
        || message.contains("verbatim illegal in command argument")
        || message.contains("\\verb ended by end of line")
        || message.contains("while scanning text of \\begin{verbatim")
        || message.contains("while scanning text of \\begin{lstlisting")
}

/// A document as the engine reads it: verbatim regions blanked.
struct Prepared<'a> {
    text: Cow<'a, str>,
    /// `\verb` tokens, by the byte offset of their backslash.
    verbs: HashMap<usize, Token>,
    /// Offsets of the `\/` end markers written into blanked `\verb`s.
    verb_markers: HashSet<usize>,
    /// enumitem label markers (`\Roman*` in `label=\Roman*.`), renamed to
    /// same-length undefined control words so the engine's counter
    /// primitives do not consume them; by backslash offset.
    renamed: HashMap<usize, &'static str>,
    /// Byte ranges whose engine tokens are not parser input (a `\verb`
    /// argument after its control word; a verbatim-like body), sorted.
    skip: Vec<(usize, usize)>,
    /// `\begin{verbatim}` offset -> offset of its `\end{verbatim}`.
    verbatim_ends: HashMap<usize, usize>,
}

impl Prepared<'_> {
    fn skips(&self, offset: usize) -> bool {
        let i = self.skip.partition_point(|(start, _)| *start <= offset);
        i > 0 && offset < self.skip[i - 1].1
    }
}

/// Counter formats enumitem accepts as `\<format>*` in a `label` template.
const LABEL_FORMATS: &[&str] = &["arabic", "roman", "Roman", "alph", "Alph", "fnsymbol"];

impl<'a> Prepared<'a> {
    /// A document the engine reads unmodified (never scanned).
    fn plain(text: &'a str) -> Self {
        Prepared {
            text: Cow::Borrowed(text),
            verbs: HashMap::new(),
            verb_markers: HashSet::new(),
            renamed: HashMap::new(),
            skip: Vec::new(),
            verbatim_ends: HashMap::new(),
        }
    }
}

fn prepare<'a>(text: &'a str, document: DocumentId) -> Prepared<'a> {
    let mut prepared = Prepared {
        text: Cow::Borrowed(text),
        verbs: HashMap::new(),
        verb_markers: HashSet::new(),
        renamed: HashMap::new(),
        skip: Vec::new(),
        verbatim_ends: HashMap::new(),
    };
    // amsmath `\numberwithin[\alph]{..}{..}`: the format is a name the parser
    // reads, not a `\alph` call for the engine to run on `]`.
    let has_labels = (text.contains('*') && LABEL_FORMATS.iter().any(|f| text.contains(&format!("\\{f}*"))))
        || text.contains("\\numberwithin[");
    let has_urls = text.contains("\\url") || text.contains("\\href") || text.contains("\\nolinkurl");
    if !(has_labels
        || has_urls
        || text.contains("\\verb")
        || text.contains("\\lstinline")
        || text.contains("verbatim")
        || text.contains("lstlisting"))
    {
        return prepared;
    }
    let tokens = tokenize_document(text, document);
    let mut bytes = text.as_bytes().to_vec();
    let mut blanked = false;
    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        match &token.kind {
            TokenKind::Verb { .. } => {
                let (start, end) = (token.span.start, token.span.end);
                prepared.verbs.insert(start, token.clone());
                prepared.skip.push((start + 1, end));
                // listings' `\lstinline[keys]<d>...<d>` is spelled over as
                // `\verb` in the copy the engine reads. The engine then
                // takes the same `\verb` path (the body is all blanks by
                // then, so what it reads is one space-delimited empty
                // argument), and the conversion below maps the position
                // back to *this* token — the one the compiler's own lexer
                // built, which carries the listing's real text. So
                // `\lstinline` needs no expansion primitive of its own and
                // its keys and delimiters never reach the engine.
                bytes[start..start + 5].copy_from_slice(b"\\verb");
                // `\verb` + blanks + `\/`: the engine reads one undefined
                // control word (mapped back to this token) and a control
                // symbol that restores mid-line state, so a space after the
                // argument still counts.
                if end - start >= 7 {
                    for b in &mut bytes[start + 5..end - 2] {
                        *b = b' ';
                    }
                    bytes[end - 2] = b'\\';
                    bytes[end - 1] = b'/';
                    prepared.verb_markers.insert(end - 2);
                } else {
                    for b in &mut bytes[start + 5..end] {
                        *b = b' ';
                    }
                }
                blanked = true;
                i += 1;
            }
            TokenKind::Command(name)
                if LABEL_FORMATS.contains(&name.as_str())
                    && (text.as_bytes().get(token.span.end) == Some(&b'*')
                        || text[..token.span.start].trim_end().ends_with("\\numberwithin[")) =>
            {
                let format = LABEL_FORMATS.iter().find(|f| **f == name.as_str()).copied().unwrap_or("arabic");
                for b in &mut bytes[token.span.start + 1..token.span.end] {
                    *b = b'Z';
                }
                prepared.renamed.insert(token.span.start, format);
                blanked = true;
                i += 1;
            }
            // hyperref reads a URL with `% # ~ _ ^ &` as other characters;
            // the parser reads those bytes raw (`url_argument`), so the engine
            // must not interpret them either.
            TokenKind::Command(name) if matches!(name.as_str(), "url" | "nolinkurl" | "href") => {
                let mut open = i + 1;
                while matches!(tokens.get(open).map(|t| &t.kind), Some(TokenKind::Space)) {
                    open += 1;
                }
                if tokens.get(open).map(|t| &t.kind) != Some(&TokenKind::LBrace) {
                    i += 1;
                    continue;
                }
                let content_start = tokens[open].span.end;
                let close = url_group_close(text, content_start);
                for b in &mut bytes[content_start..close] {
                    *b = b' ';
                }
                blanked = true;
                i = open + 1;
                while i < tokens.len() && tokens[i].span.start < close {
                    i += 1;
                }
            }
            TokenKind::Command(name) if name == "begin" => {
                let Some((env, close)) = braced_word_after(&tokens, i + 1) else {
                    i += 1;
                    continue;
                };
                if !matches!(env.as_str(), "verbatim" | "verbatim*" | "lstlisting") {
                    i += 1;
                    continue;
                }
                let mut content_start = tokens[close].span.end;
                if env == "lstlisting" {
                    content_start = after_bracket_option(text, content_start);
                }
                let end_tag = format!("\\end{{{env}}}");
                let tag_start = text[content_start..]
                    .find(end_tag.as_str())
                    .map_or(text.len(), |offset| content_start + offset);
                for b in &mut bytes[content_start..tag_start] {
                    *b = b' ';
                }
                prepared.skip.push((content_start, tag_start));
                if env != "lstlisting" {
                    prepared.verbatim_ends.insert(token.span.start, tag_start);
                }
                blanked = true;
                i = close + 1;
                while i < tokens.len() && tokens[i].span.start < tag_start {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    prepared.skip.sort_unstable();
    if blanked {
        // Only whole UTF-8 sequences were replaced by ASCII spaces.
        prepared.text = Cow::Owned(String::from_utf8(bytes).expect("blanking keeps UTF-8 valid"));
    }
    prepared
}

/// The offset of the `}` closing a URL group whose content starts at `from`,
/// with the parser's `url_argument` rules (`\{`/`\}` escapes, nested braces
/// balance); the end of the text when unclosed.
fn url_group_close(text: &str, from: usize) -> usize {
    let bytes = text.as_bytes();
    let mut depth = 1usize;
    let mut pos = from;
    while pos < bytes.len() {
        match bytes[pos] {
            b'\\' if matches!(bytes.get(pos + 1), Some(b'{' | b'}')) => pos += 2,
            b'{' => {
                depth += 1;
                pos += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return pos;
                }
                pos += 1;
            }
            _ => pos += 1,
        }
    }
    bytes.len()
}

/// `{ word }` starting at `index` (spaces skipped before the brace): the
/// word and the index of the closing brace.
fn braced_word_after(tokens: &[Token], mut index: usize) -> Option<(String, usize)> {
    while matches!(tokens.get(index)?.kind, TokenKind::Space) {
        index += 1;
    }
    if tokens.get(index)?.kind != TokenKind::LBrace {
        return None;
    }
    let TokenKind::Word(word) = &tokens.get(index + 1)?.kind else {
        return None;
    };
    if tokens.get(index + 2)?.kind != TokenKind::RBrace {
        return None;
    }
    Some((word.clone(), index + 2))
}

/// Mirrors the parser's `optional_bracket_argument` for `lstlisting`: the
/// body starts after a `[...]` that follows the environment name (whitespace
/// without a blank line may precede it).
fn after_bracket_option(text: &str, from: usize) -> usize {
    let bytes = text.as_bytes();
    let mut j = from;
    let mut newlines = 0;
    while j < bytes.len() && (bytes[j] as char).is_ascii_whitespace() {
        if bytes[j] == b'\n' {
            newlines += 1;
        }
        j += 1;
    }
    if newlines >= 2 || bytes.get(j) != Some(&b'[') {
        return from;
    }
    // A `]` inside braces does not close the option
    // (`[caption={[short]long}]`). A backslash takes the next byte with it,
    // as TeX reads a control symbol: `\]` is display-math close, not a
    // bracket, and `\{`/`\}` do not change the brace depth.
    let mut depth = 0usize;
    let mut k = j + 1;
    while k < bytes.len() {
        match bytes[k] {
            b'\\' => k += 1,
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            b']' if depth == 0 => return k + 1,
            _ => {}
        }
        k += 1;
    }
    from
}

struct Converter<'d> {
    documents: &'d [SourceDocument<'d>],
    document_by_path: HashMap<&'d str, usize>,
    /// Engine source id -> document index (`None`: the prelude).
    source_documents: HashMap<u32, Option<usize>>,
    /// Document index of source 0 (the entry), read without the map.
    entry: usize,
    out: Vec<ExpandedToken>,
    diagnostics: Vec<Diagnostic>,
    arraystretch: HashMap<(usize, usize), String>,
    /// Names from a preamble `\includeonly{...}` (`None`: never used).
    /// Replaced wholesale by each call, so the last call wins, and read by
    /// [`include`]; each entry is kept trimmed and, when suffixed, stripped
    /// of one trailing `.tex`, mirroring `\include`'s own lookup.
    includeonly: Option<HashSet<String>>,
    /// Set once the converter emits `\begin{document}` (see [`push`]): what
    /// [`in_preamble`] reads to gate preamble-only `\includeonly`.
    document_begun: bool,
    /// Characters of the word being assembled, with its provenance.
    word: Option<PendingWord>,
    last_span: Span,
    /// Reading the `{<\arraystretch>}` group after a table marker: the key
    /// it is recorded under, brace depth, and the text so far.
    stretch: Option<((usize, usize), usize, String)>,
    /// Reading the `{<\@currentlabel>}` group after a `\refstepcounter`
    /// marker: the marker's span (re-keyed to the pushed
    /// `flashtexcurrentlabel` token's own span when the capture completes),
    /// brace depth, and the text so far.
    current_label: Option<((usize, usize), usize, String)>,
    /// `\@currentlabel`'s expansion just after each bare `\refstepcounter`,
    /// keyed by the pushed `flashtexcurrentlabel` token's own span.
    current_label_by_marker: HashMap<(usize, usize), String>,
    /// Engine token index being converted, and the index of the marker that
    /// opened the current `\arraystretch` capture.
    index: usize,
    stretch_index: usize,
    /// Engine token index of the marker that opened the current
    /// `\@currentlabel` capture.
    current_label_index: usize,
    /// Every `\arraystretch` record in production order, with its marker's
    /// engine token index (what the incremental cache keeps and splices).
    stretch_log: Vec<(usize, (usize, usize), String)>,
    /// Every `\@currentlabel` record in production order, with its marker's
    /// engine token index (kept and spliced like `stretch_log`).
    current_label_log: Vec<(usize, (usize, usize), String)>,
}

struct PendingWord {
    text: String,
    span: Span,
    definition: Option<Span>,
    maps: bool,
}

/// Where one engine token belongs in the parser's input.
#[derive(Clone, Copy)]
struct Placement {
    span: Span,
    definition: Option<Span>,
    maps: bool,
    /// The token's own source bytes (definition bytes for replacement text),
    /// when it has any.
    real: Option<Span>,
}

impl<'d> Converter<'d> {
    fn span(&self, span: tex::Span) -> Option<Span> {
        if span.is_synthetic() {
            return None;
        }
        let document = if span.source_id == 0 {
            self.entry
        } else {
            (*self.source_documents.get(&span.source_id)?)?
        };
        let text = self.documents[document].text;
        let (start, end) = (span.start as usize, span.end as usize);
        (end <= text.len() && start <= end).then(|| Span::in_document(DocumentId(document), start, end))
    }

    fn place(&self, token: &tex::Token, origin: Option<tex::Span>) -> Placement {
        let own = self.span(token.span);
        match origin {
            Some(invocation) => {
                let direct = !token.span.is_synthetic()
                    && token.span.source_id == invocation.source_id
                    && token.span.start >= invocation.end;
                match (direct, own, self.span(invocation)) {
                    (true, Some(own), _) => Placement { span: own, definition: None, maps: false, real: Some(own) },
                    (_, own, Some(at)) => Placement { span: at, definition: own, maps: true, real: own },
                    (_, own, None) => Placement {
                        span: own.unwrap_or(self.last_span),
                        definition: None,
                        maps: own.is_none(),
                        real: own,
                    },
                }
            }
            None => match own {
                Some(own) => Placement { span: own, definition: None, maps: false, real: Some(own) },
                None => Placement { span: self.last_span, definition: None, maps: true, real: None },
            },
        }
    }

    fn source_text(&self, span: Span) -> &'d str {
        self.documents[span.document.0].text.get(span.start..span.end).unwrap_or("")
    }

    fn flush_word(&mut self) {
        if let Some(word) = self.word.take() {
            self.out.push(ExpandedToken {
                token: Token { kind: TokenKind::Word(word.text), span: word.span },
                definition: word.definition,
                maps_to_invocation: word.maps,
            });
        }
    }

    /// True when the output tail is `\begin{document` awaiting its closing
    /// brace — `Command("begin")`, `LBrace`, `Word("document")`, skipping
    /// spaces — the same shape the parser's own `document_begin_end`
    /// matches. This covers both the pass-through token sequence and
    /// [`push_environment`]'s fused emission, which funnels its closing
    /// brace through [`push`].
    fn closes_document_begin(&self) -> bool {
        let mut tail = self
            .out
            .iter()
            .rev()
            .filter(|t| !matches!(t.token.kind, TokenKind::Space))
            .map(|t| &t.token.kind);
        matches!(tail.next(), Some(TokenKind::Word(name)) if name == "document")
            && matches!(tail.next(), Some(TokenKind::LBrace))
            && matches!(tail.next(), Some(TokenKind::Command(name)) if name == "begin")
    }

    fn push(&mut self, kind: TokenKind, at: Placement) {
        self.flush_word();
        if matches!(kind, TokenKind::RBrace) && self.closes_document_begin() {
            self.document_begun = true;
        }
        self.last_span = at.span;
        // Whitespace runs collapse the way the parser's own tokenizer
        // produces them: one `Space`, or one `ParBreak` if the run holds a
        // paragraph break.
        match (&kind, self.out.last().map(|t| &t.token.kind)) {
            (TokenKind::Space, Some(TokenKind::Space | TokenKind::ParBreak)) => return,
            (TokenKind::ParBreak, Some(TokenKind::ParBreak)) => return,
            (TokenKind::ParBreak, Some(TokenKind::Space)) => {
                self.out.pop();
            }
            _ => {}
        }
        self.out.push(ExpandedToken {
            token: Token { kind, span: at.span },
            definition: at.definition,
            maps_to_invocation: at.maps,
        });
    }

    fn push_char(&mut self, c: char, at: Placement) {
        self.last_span = at.span;
        if let Some(word) = &mut self.word {
            let contiguous = word.maps == at.maps
                && if at.maps {
                    word.span == at.span
                        && match (word.definition, at.definition) {
                            (Some(a), Some(b)) => a.document == b.document && a.end == b.start,
                            (None, None) => true,
                            _ => false,
                        }
                } else {
                    word.span.document == at.span.document && word.span.end == at.span.start
                };
            if contiguous {
                word.text.push(c);
                if at.maps {
                    if let (Some(d), Some(b)) = (word.definition, at.definition) {
                        word.definition = Some(d.merge(b));
                    }
                } else {
                    word.span = word.span.merge(at.span);
                }
                return;
            }
        }
        self.flush_word();
        self.word = Some(PendingWord {
            text: c.to_string(),
            span: at.span,
            definition: at.definition,
            maps: at.maps,
        });
    }

    /// `\begin`/`\end`, `{`, `name`, `}` for an environment the engine left
    /// to the parser. Piece spans are exact when the bytes after the
    /// control word spell `{name}`; otherwise every piece gets `at`.
    fn push_environment(&mut self, command: &str, name: &str, at: Placement) {
        let pieces = at.real.and_then(|real| {
            let text = self.documents[real.document.0].text;
            let bytes = text.as_bytes();
            let mut open = real.end;
            while open < bytes.len() && (bytes[open] as char).is_ascii_whitespace() {
                open += 1;
            }
            if bytes.get(open) != Some(&b'{') {
                return None;
            }
            let close = open + 1 + text[open + 1..].find('}')?;
            (text[open + 1..close].trim() == name).then_some((open, close))
        });
        let piece = |start: usize, end: usize| -> Placement {
            match (pieces, at.real) {
                (Some(_), Some(real)) => {
                    let exact = Span::in_document(real.document, start, end);
                    if at.maps {
                        Placement { span: at.span, definition: Some(exact), maps: true, real: Some(exact) }
                    } else {
                        Placement { span: exact, definition: None, maps: false, real: Some(exact) }
                    }
                }
                _ => at,
            }
        };
        let (command_at, open_at, word_at, close_at) = match (pieces, at.real) {
            (Some((open, close)), Some(real)) => (
                piece(real.start, real.end),
                piece(open, open + 1),
                piece(open + 1, close),
                piece(close, close + 1),
            ),
            _ => (at, at, at, at),
        };
        self.push(TokenKind::Command(command.to_string()), command_at);
        self.push(TokenKind::LBrace, open_at);
        self.flush_word();
        self.out.push(ExpandedToken {
            token: Token { kind: TokenKind::Word(name.to_string()), span: word_at.span },
            definition: word_at.definition,
            maps_to_invocation: word_at.maps,
        });
        self.push(TokenKind::RBrace, close_at);
    }
}

/// Entry documents at least this large keep a thread-local incremental
/// cache (see [`expand_project_cached`]); smaller ones re-expand from
/// scratch. Measured on HW1/HW2 (5 KB): a cached keystroke re-expands in
/// ~0.17-0.21 ms against ~0.23-0.27 ms from scratch, so real documents use
/// the cache while unit-test-sized snippets stay on the full path.
pub const INCREMENTAL_MIN_BYTES: usize = 4 * 1024;

fn limits_for(bytes: usize) -> Limits {
    Limits {
        max_expansion_steps: 2_000_000 + 32 * bytes as u64,
        max_output_tokens: 2_000_000 + 8 * bytes as u64,
        ..Limits::default()
    }
}

/// Host setup shared by the full and the incremental path. Everything it
/// sets is part of the engine's checkpointed state.
fn configure(engine: &mut Engine) {
    engine.run_host_prelude(HOST_PRELUDE);
    engine.set_emit_unbalanced_close(true);
    for name in BUILT_INS {
        engine.declare_host_command(name);
    }
    engine.declare_host_command("include");
    engine.declare_host_command("flashtexsetlength");
    engine.declare_host_command("flashtexaddtolength");
}

fn has_includes(text: &str) -> bool {
    text.contains("\\input") || text.contains("\\include")
}

fn step_limit_hit(diagnostics: &[tex::Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.message.contains("expansion step limit exceeded"))
}

/// What the caller must do after one converted token.
enum Flow {
    Next,
    /// A source-level `\input`/`\include`: its braced path is still to be
    /// read.
    Include(String, Placement),
    /// A source-level `\includeonly`: its braced list is still to be read.
    IncludeOnly(Placement),
}

impl<'d> Converter<'d> {
    fn new(documents: &'d [SourceDocument<'d>], entry: usize) -> Self {
        Converter {
            documents,
            document_by_path: documents.iter().enumerate().map(|(i, d)| (d.path, i)).collect(),
            includeonly: None,
            document_begun: false,
            source_documents: HashMap::from([(0, Some(entry))]),
            entry,
            out: Vec::new(),
            diagnostics: Vec::new(),
            arraystretch: HashMap::new(),
            word: None,
            last_span: Span::in_document(DocumentId(entry), 0, 0),
            stretch: None,
            current_label: None,
            current_label_by_marker: HashMap::new(),
            index: 0,
            stretch_index: 0,
            current_label_index: 0,
            stretch_log: Vec::new(),
            current_label_log: Vec::new(),
        }
    }

    /// No partially built word, `\arraystretch` capture or `\@currentlabel`
    /// capture: the output so far does not depend on tokens still to come.
    fn clean(&self) -> bool {
        self.word.is_none() && self.stretch.is_none() && self.current_label.is_none()
    }

    fn convert_token(&mut self, prepared: &[Prepared<'_>], token: &tex::Token, origin: Option<tex::Span>) -> Flow {
        let conv = self;
        let at = conv.place(token, origin);
        if let Some((key, depth, mut text)) = conv.stretch.take() {
            match &token.kind {
                TexKind::Char(_, CatCode::BeginGroup) => {
                    if depth > 0 {
                        text.push('{');
                    }
                    conv.stretch = Some((key, depth + 1, text));
                }
                TexKind::Char(_, CatCode::EndGroup) if depth <= 1 => {
                    conv.stretch_log.push((conv.stretch_index, key, text.clone()));
                    conv.arraystretch.insert(key, text);
                }
                TexKind::Char(_, CatCode::EndGroup) => {
                    text.push('}');
                    conv.stretch = Some((key, depth - 1, text));
                }
                TexKind::Char(c, _) | TexKind::ActiveChar(c) => {
                    text.push(*c);
                    conv.stretch = Some((key, depth, text));
                }
                TexKind::ControlSequence(cs) => {
                    text.push('\\');
                    text.push_str(cs);
                    conv.stretch = Some((key, depth, text));
                }
                _ => conv.stretch = Some((key, depth, text)),
            }
            return Flow::Next;
        }
        // `\@currentlabel` capture after a `\refstepcounter` marker, mirroring
        // the `\arraystretch` capture above. Unlike a table marker there is
        // no enclosing environment to key by: when the group closes, a
        // literal `flashtexcurrentlabel` command token is pushed (exactly
        // what the default arm below would have produced for that name) and
        // the text is keyed by that pushed token's own span.
        if let Some((key, depth, mut text)) = conv.current_label.take() {
            match &token.kind {
                TexKind::Char(_, CatCode::BeginGroup) => {
                    if depth > 0 {
                        text.push('{');
                    }
                    conv.current_label = Some((key, depth + 1, text));
                }
                TexKind::Char(_, CatCode::EndGroup) if depth <= 1 => {
                    conv.push(TokenKind::Command("flashtexcurrentlabel".into()), at);
                    let pushed = (conv.last_span.document.0, conv.last_span.start);
                    conv.current_label_log.push((conv.current_label_index, pushed, text.clone()));
                    conv.current_label_by_marker.insert(pushed, text);
                }
                TexKind::Char(_, CatCode::EndGroup) => {
                    text.push('}');
                    conv.current_label = Some((key, depth - 1, text));
                }
                TexKind::Char(c, _) | TexKind::ActiveChar(c) => {
                    text.push(*c);
                    conv.current_label = Some((key, depth, text));
                }
                TexKind::ControlSequence(cs) => {
                    text.push('\\');
                    text.push_str(cs);
                    conv.current_label = Some((key, depth, text));
                }
                _ => conv.current_label = Some((key, depth, text)),
            }
            return Flow::Next;
        }
        if !at.maps && at.real.is_some_and(|real| prepared[real.document.0].skips(real.start)) {
            return Flow::Next;
        }
        match &token.kind {
            TexKind::Char(c, cat) => match cat {
                CatCode::BeginGroup => conv.push(TokenKind::LBrace, at),
                CatCode::EndGroup => conv.push(TokenKind::RBrace, at),
                CatCode::MathShift => conv.push(TokenKind::MathShift, at),
                CatCode::Superscript => conv.push(TokenKind::Superscript, at),
                CatCode::Subscript => conv.push(TokenKind::Subscript, at),
                CatCode::Space => conv.push(TokenKind::Space, at),
                _ => conv.push_char(*c, at),
            },
            TexKind::ActiveChar(c) => conv.push_char(*c, at),
            TexKind::Param(n) => {
                conv.push_char('#', at);
                conv.push_char(char::from(b'0' + n.min(&9)), at);
            }
            TexKind::Eof => {}
            TexKind::ControlSequence(name) => {
                let real_text = at.real.map_or("", |real| conv.source_text(real));
                match name.as_str() {
                    // Grouping bookkeeping and `\relax` produce nothing for the
                    // parser (LaTeX's environment groups included).
                    "begingroup" | "endgroup" | "relax" => {}
                    "flashtexsetlength" => conv.push(TokenKind::Command("setlength".to_string()), at),
                    "flashtexaddtolength" => {
                        conv.push(TokenKind::Command("addtolength".to_string()), at)
                    }
                    "flashtexbegintabular" | "flashtexbegintabularstar" | "flashtexbeginarray" => {
                        let env = match name.as_str() {
                            "flashtexbegintabular" => "tabular",
                            "flashtexbegintabularstar" => "tabular*",
                            _ => "array",
                        };
                        let begin = origin
                            .and_then(|o| conv.span(o))
                            .filter(|b| conv.source_text(*b) == "\\begin")
                            .map(|b| Placement { span: b, definition: None, maps: false, real: Some(b) })
                            .unwrap_or(at);
                        conv.stretch_index = conv.index;
                        conv.stretch = Some(((begin.span.document.0, begin.span.start), 0, String::new()));
                        conv.push_environment("begin", env, begin);
                    }
                    // The `{<\@currentlabel>}` group after this marker is
                    // captured above and re-emitted as a literal
                    // `flashtexcurrentlabel` token carrying no output of its
                    // own — exactly like the real `\refstepcounter`, which
                    // the engine runs with no output tokens.
                    "flashtexcurrentlabelmarker" => {
                        conv.current_label_index = conv.index;
                        conv.current_label =
                            Some(((at.span.document.0, at.span.start), 0, String::new()));
                    }
                    "\\" => conv.push(TokenKind::LineBreak, at),
                    "[" => conv.push(TokenKind::DisplayMathOpen, at),
                    "]" => conv.push(TokenKind::DisplayMathClose, at),
                    "(" => conv.push(TokenKind::InlineMathOpen, at),
                    ")" => conv.push(TokenKind::InlineMathClose, at),
                    "par" if !real_text.starts_with('\\') && at.real.is_some() => conv.push(TokenKind::ParBreak, at),
                    "verb" | "verb*" => {
                        let verb = at
                            .real
                            .and_then(|real| prepared[real.document.0].verbs.get(&real.start).cloned());
                        match verb {
                            Some(verb) => conv.push(verb.kind, at),
                            None => conv.push(TokenKind::Command(name.clone()), at),
                        }
                    }
                    _ if name.bytes().all(|b| b == b'Z')
                        && at
                            .real
                            .and_then(|real| prepared[real.document.0].renamed.get(&real.start))
                            .is_some() =>
                    {
                        let original = at
                            .real
                            .and_then(|real| prepared[real.document.0].renamed.get(&real.start))
                            .copied()
                            .unwrap_or("arabic");
                        conv.push(TokenKind::Command(original.to_string()), at);
                    }
                    "/" if at
                        .real
                        .is_some_and(|real| prepared[real.document.0].verb_markers.contains(&real.start)) => {}
                    "input" | "include" if origin.is_none() && real_text == format!("\\{name}") => {
                        return Flow::Include(name.clone(), at);
                    }
                    "includeonly" if origin.is_none() && real_text == "\\includeonly" => {
                        return Flow::IncludeOnly(at);
                    }
                    _ if name.chars().count() == 1 && !name.chars().all(char::is_alphabetic) => {
                        conv.flush_word();
                        conv.push(TokenKind::Word(name.clone()), at);
                    }
                    // r2 reads a verbatim body itself and ends it with a frozen
                    // `\end<name>` carrying the `\begin` span.
                    _ if real_text == "\\begin"
                        && name.starts_with("endverbatim")
                        && at.real.is_some_and(|real| prepared[real.document.0].verbatim_ends.contains_key(&real.start)) =>
                    {
                        let real = at.real.expect("checked above");
                        let tag = prepared[real.document.0].verbatim_ends[&real.start];
                        let end = Span::in_document(real.document, tag, tag + 4);
                        conv.push_environment(
                            "end",
                            &name[3..],
                            Placement { span: end, definition: None, maps: false, real: Some(end) },
                        );
                    }
                    _ if real_text == "\\begin" && name != "begin" => {
                        conv.push_environment("begin", name, at);
                    }
                    _ if real_text == "\\end" && name.len() > 3 && name.starts_with("end") => {
                        conv.push_environment("end", &name[3..], at);
                    }
                    _ => conv.push(TokenKind::Command(name.clone()), at),
                }
            }
        }
        Flow::Next
    }

    /// A runaway expansion (`\def\x{\x}\x`, common mid-edit) stops the engine
    /// for good. Keep the rest of the document visible: its remaining bytes
    /// are typeset from the parser's own tokenizer, without macro expansion.
    fn resume_unexpanded(&mut self, document: usize, offset: usize) {
        let text = self.documents[document].text;
        let offset = offset.min(text.len());
        if !text.is_char_boundary(offset) {
            return;
        }
        for token in tokenize_document(&text[offset..], DocumentId(document)) {
            let span = Span::in_document(DocumentId(document), token.span.start + offset, token.span.end + offset);
            self.out.push(ExpandedToken {
                token: Token { kind: token.kind, span },
                definition: None,
                maps_to_invocation: false,
            });
        }
    }

    fn map_diagnostics(&mut self, diagnostics: &[tex::Diagnostic]) {
        let fallback = self.last_span;
        for diagnostic in diagnostics {
            if parser_reports_itself(&diagnostic.message) {
                continue;
            }
            let span = self
                .span(diagnostic.span)
                .or(if diagnostic.span.is_synthetic() { Some(fallback) } else { None });
            let recovery = Some(recovery_for(&diagnostic.message).to_string());
            self.diagnostics.push(match diagnostic.severity {
                tex::Severity::Error => Diagnostic::error(diagnostic.message.clone(), span, recovery),
                tex::Severity::Warning => Diagnostic::warning(diagnostic.message.clone(), span, recovery),
            });
        }
    }
}

/// Run the expansion pass over the entry document (and everything it
/// includes) from scratch.
pub fn expand_project(documents: &[SourceDocument<'_>], entry: usize) -> Expansion {
    let prepared: Vec<Prepared<'_>> = documents
        .iter()
        .enumerate()
        .map(|(index, document)| prepare(document.text, DocumentId(index)))
        .collect();
    let total_bytes: usize = documents.iter().map(|d| d.text.len()).sum();
    let entry_text: &str = prepared.get(entry).map_or("", |p| p.text.as_ref());
    let mut engine = Engine::with_limits(entry_text, limits_for(total_bytes));
    configure(&mut engine);

    let mut conv = Converter::new(documents, entry);
    let mut lookahead: VecDeque<(tex::Token, Option<tex::Span>)> = VecDeque::new();
    loop {
        let next = match lookahead.pop_front() {
            Some(t) => Some(t),
            None => engine.next_content_token_with_origin(),
        };
        let Some((token, origin)) = next else { break };
        match conv.convert_token(&prepared, &token, origin) {
            Flow::Next => continue,
            Flow::Include(name, at) => {
                // Read the braced path through the engine.
                let (taken, path, ok) = read_braced_argument(&mut engine);
                if !ok {
                    conv.push(TokenKind::Command(name), at);
                    lookahead.extend(taken);
                    continue;
                }
                include(&mut conv, &mut engine, &prepared, &name, path.trim(), at.span);
            }
            Flow::IncludeOnly(at) => {
                let (taken, path, ok) = read_braced_argument(&mut engine);
                if !ok {
                    conv.push(TokenKind::Command("includeonly".to_string()), at);
                    lookahead.extend(taken);
                    continue;
                }
                record_includeonly(&mut conv, path.trim(), at.span);
            }
        }
    }
    conv.flush_word();

    if step_limit_hit(engine.diagnostics()) {
        if let Some((source, offset)) = engine.input_position() {
            if let Some(Some(document)) = conv.source_documents.get(&source).copied() {
                conv.resume_unexpanded(document, offset);
            }
        }
    }
    let diagnostics = engine.diagnostics().to_vec();
    conv.map_diagnostics(&diagnostics);
    Expansion { tokens: Rc::new(conv.out), diagnostics: conv.diagnostics, arraystretch: conv.arraystretch, current_label_by_marker: conv.current_label_by_marker }
}

/// A converter state with nothing pending, recorded while converting: after
/// `index` engine tokens the converted output had `out_len` tokens.
#[derive(Debug, Clone, Copy)]
struct Mark {
    index: usize,
    out_len: usize,
    last_span: Span,
}

/// Engine tokens between two recorded marks.
const MARK_EVERY: usize = 64;

/// Incremental expansion state for one entry document: the engine's
/// checkpoints (`IncrementalExpander`) and the converted stream with marks
/// where it can be cut and resumed.
pub struct ExpansionCache {
    entry_path: String,
    /// The blanked entry text the expander holds.
    masked: String,
    created_bytes: usize,
    expander: IncrementalExpander,
    out: Rc<Vec<ExpandedToken>>,
    marks: Vec<Mark>,
    stretch_log: Vec<(usize, (usize, usize), String)>,
    current_label_log: Vec<(usize, (usize, usize), String)>,
    last_span: Span,
    /// Engine tokens the previous run produced.
    old_engine_tokens: usize,
    /// The last revision ran into the step limit: re-expand from scratch
    /// until it no longer does (an incremental run would hit the same limit
    /// and still need the full run for its recovery).
    halted: bool,
    /// `out` is lent to a parser ([`lend_cached_tokens`]). A cache whose
    /// stream never came back is rebuilt instead of reused.
    lent: bool,
}

thread_local! {
    static CACHES: RefCell<Vec<ExpansionCache>> = const { RefCell::new(Vec::new()) };
}

/// Documents kept warm per thread.
const MAX_CACHES: usize = 4;

/// Source bytes between engine checkpoints. Engine state is copy-on-write, so
/// a checkpoint costs a few reference-count bumps and a convergence check
/// compares only what the two runs assigned since they diverged; a keystroke
/// then re-expands at most about two intervals of source.
const CHECKPOINT_INTERVAL: usize = 512;

/// [`expand_project`], re-expanding incrementally when the entry document
/// was expanded before on this thread (same path). Output is identical to
/// [`expand_project`] (see `tests/expansion_incremental.rs`). Small entries
/// and projects that `\input` files use the full path.
pub fn expand_project_cached(documents: &[SourceDocument<'_>], entry: usize) -> Expansion {
    let Some(document) = documents.get(entry) else {
        return expand_project(documents, entry);
    };
    if document.text.len() < INCREMENTAL_MIN_BYTES || has_includes(document.text) {
        return expand_project(documents, entry);
    }
    CACHES.with(|caches| {
        let mut caches = caches.borrow_mut();
        let mut slot = caches
            .iter()
            .position(|cache| cache.entry_path == document.path)
            .map(|index| caches.remove(index));
        let expansion = expand_project_with_cache(documents, entry, &mut slot);
        if let Some(cache) = slot {
            caches.insert(0, cache);
            caches.truncate(MAX_CACHES);
        }
        expansion
    })
}

/// Let the parser edit `tokens` in place: when this thread's cache for
/// `entry_path` holds the same stream, it gives up its reference until
/// [`return_cached_tokens`], so the parser's edit does not copy the stream.
/// The parser must undo its edits before returning it.
pub(crate) fn lend_cached_tokens(entry_path: &str, tokens: &Rc<Vec<ExpandedToken>>) -> bool {
    CACHES.with(|caches| {
        let mut caches = caches.borrow_mut();
        match caches.iter_mut().find(|c| c.entry_path == entry_path && !c.lent && Rc::ptr_eq(&c.out, tokens)) {
            Some(cache) => {
                cache.out = Rc::new(Vec::new());
                cache.lent = true;
                true
            }
            None => false,
        }
    })
}

/// Give a lent stream, with every parser edit undone, back to the cache.
pub(crate) fn return_cached_tokens(entry_path: &str, tokens: Rc<Vec<ExpandedToken>>) {
    CACHES.with(|caches| {
        let mut caches = caches.borrow_mut();
        if let Some(cache) = caches.iter_mut().find(|c| c.entry_path == entry_path && c.lent) {
            cache.out = tokens;
            cache.lent = false;
        }
    })
}

/// [`expand_project`] through a caller-held cache, with no size threshold.
pub fn expand_project_with_cache(
    documents: &[SourceDocument<'_>],
    entry: usize,
    cache: &mut Option<ExpansionCache>,
) -> Expansion {
    let Some(document) = documents.get(entry).copied() else {
        *cache = None;
        return expand_project(documents, entry);
    };
    if has_includes(document.text) {
        *cache = None;
        return expand_project(documents, entry);
    }
    let prepared: Vec<Prepared<'_>> = documents
        .iter()
        .enumerate()
        .map(|(index, d)| if index == entry { prepare(d.text, DocumentId(index)) } else { Prepared::plain(d.text) })
        .collect();
    if cache.as_ref().is_some_and(|c| c.halted && c.entry_path == document.path) {
        let full = expand_project(documents, entry);
        if full.diagnostics.iter().any(|d| d.message.contains("expansion step limit exceeded")) {
            return full;
        }
        *cache = None;
    }
    let masked: &str = prepared[entry].text.as_ref();
    let reusable = cache.as_ref().is_some_and(|c| {
        !c.lent && c.entry_path == document.path && masked.len() <= 2 * c.created_bytes.max(INCREMENTAL_MIN_BYTES)
    });
    let expansion = if reusable {
        update_cache(cache.as_mut().expect("checked"), documents, entry, &prepared)
    } else {
        let (fresh, expansion) = build_cache(documents, entry, &prepared);
        *cache = Some(fresh);
        expansion
    };
    match expansion {
        Some(expansion) => expansion,
        None => {
            // Runaway expansion: the full path's recovery reads the engine's
            // own stop position, which the cache does not keep.
            if let Some(cache) = cache.as_mut() {
                cache.halted = true;
            }
            expand_project(documents, entry)
        }
    }
}

fn build_cache(documents: &[SourceDocument<'_>], entry: usize, prepared: &[Prepared<'_>]) -> (ExpansionCache, Option<Expansion>) {
    let masked: &str = prepared[entry].text.as_ref();
    let init: Rc<dyn Fn(&mut Engine)> = Rc::new(configure);
    let expander = IncrementalExpander::with_host(masked, limits_for(masked.len()), CHECKPOINT_INTERVAL, init);
    let mut conv = Converter::new(documents, entry);
    let mut marks = vec![Mark { index: 0, out_len: 0, last_span: conv.last_span }];
    convert_range(&mut conv, prepared, &expander, 0, &mut marks, None);
    conv.flush_word();
    let mut cache = ExpansionCache {
        entry_path: documents[entry].path.to_string(),
        masked: masked.to_string(),
        created_bytes: masked.len(),
        expander,
        out: Rc::new(Vec::new()),
        marks,
        stretch_log: Vec::new(),
        current_label_log: Vec::new(),
        last_span: conv.last_span,
        old_engine_tokens: 0,
        halted: false,
        lent: false,
    };
    let expansion = finish(&mut cache, conv);
    (cache, expansion)
}

/// The reusable old suffix while re-converting after an edit.
struct Join<'a> {
    /// First new engine index whose token comes from the old run.
    from: usize,
    /// new index - old index for those tokens.
    offset: isize,
    old_marks: &'a [Mark],
    /// Old converted tokens from `base` on (old coordinates).
    old_tail: &'a mut Vec<ExpandedToken>,
    base: usize,
    shift: &'a dyn Fn(Span) -> Span,
}

/// Convert engine tokens `from..` of `expander` into `conv`, recording marks.
/// With `join`, stop as soon as the old run's converted suffix can be spliced
/// in; returns the old mark it was spliced at.
fn convert_range(
    conv: &mut Converter<'_>,
    prepared: &[Prepared<'_>],
    expander: &IncrementalExpander,
    from: usize,
    marks: &mut Vec<Mark>,
    mut join: Option<Join<'_>>,
) -> Option<Mark> {
    let tokens = expander.tokens();
    let origins = expander.origins();
    for k in from..tokens.len() {
        if k > from && conv.clean() {
            if let Some(j) = join.as_mut() {
                if k >= j.from {
                    let old_k = k as isize - j.offset;
                    if let Ok(m) = j.old_marks.binary_search_by_key(&old_k, |mark| mark.index as isize) {
                        let old = j.old_marks[m];
                        if old.out_len >= j.base {
                            // The converter's next decision depends only on
                            // the last converted token; it must agree.
                            let old_last = if old.out_len == j.base {
                                None
                            } else {
                                j.old_tail.get(old.out_len - 1 - j.base).map(|t| shifted(t, j.shift))
                            };
                            let agrees = if old.out_len == j.base {
                                conv.out.len() == j.base
                            } else {
                                conv.out.last() == old_last.as_ref()
                            };
                            if agrees {
                                let out_offset = conv.out.len() as isize - old.out_len as isize;
                                let shift = j.shift;
                                conv.out.extend(j.old_tail.drain(old.out_len - j.base..).map(|t| shifted(&t, shift)));
                                marks.extend(j.old_marks[m..].iter().map(|mark| Mark {
                                    index: (mark.index as isize + j.offset) as usize,
                                    out_len: (mark.out_len as isize + out_offset) as usize,
                                    last_span: shift(mark.last_span),
                                }));
                                return Some(old);
                            }
                        }
                    }
                }
            }
            if marks.last().map_or(true, |mark| k - mark.index >= MARK_EVERY) {
                marks.push(Mark { index: k, out_len: conv.out.len(), last_span: conv.last_span });
            }
        }
        conv.index = k;
        // Includes never reach this path (`has_includes` sends their
        // documents through the full expansion above); pass both commands
        // through untouched so the parser, not the cache, reports them.
        match conv.convert_token(prepared, &tokens[k], origins[k]) {
            Flow::Include(name, at) => conv.push(TokenKind::Command(name), at),
            Flow::IncludeOnly(at) => conv.push(TokenKind::Command("includeonly".to_string()), at),
            Flow::Next => {}
        }
    }
    None
}

fn shifted(token: &ExpandedToken, shift: &dyn Fn(Span) -> Span) -> ExpandedToken {
    ExpandedToken {
        token: Token { kind: token.token.kind.clone(), span: shift(token.token.span) },
        definition: token.definition.map(shift),
        maps_to_invocation: token.maps_to_invocation,
    }
}

fn update_cache(
    cache: &mut ExpansionCache,
    documents: &[SourceDocument<'_>],
    entry: usize,
    prepared: &[Prepared<'_>],
) -> Option<Expansion> {
    let masked: &str = prepared[entry].text.as_ref();
    let changes = crate::incremental::changed_bytes(&cache.masked, masked);
    if changes.old.is_empty() && changes.new.is_empty() && cache.masked.len() == masked.len() {
        let mut conv = Converter::new(documents, entry);
        conv.out = Vec::new();
        conv.last_span = cache.last_span;
        conv.stretch_log = cache.stretch_log.clone();
        conv.arraystretch = stretch_map(&conv.stretch_log);
        conv.current_label_log = cache.current_label_log.clone();
        conv.current_label_by_marker = stretch_map(&conv.current_label_log);
        let tokens = cache.out.clone();
        let mut expansion = finish_diagnostics(cache, conv)?;
        expansion.tokens = tokens;
        return Some(expansion);
    }
    let stats = cache.expander.edit(&Edit {
        start: changes.old.start,
        end: changes.old.end,
        replacement: masked[changes.new.clone()].to_string(),
    });
    let delta = masked.len() as isize - cache.masked.len() as isize;
    cache.masked.clear();
    cache.masked.push_str(masked);

    let n_new = cache.expander.tokens().len();
    let prefix = stats.prefix_reused.min(n_new);
    let suffix = if stats.converged_at.is_some() { stats.suffix_reused } else { 0 };
    let old_marks = std::mem::take(&mut cache.marks);
    let restart_at = old_marks.partition_point(|mark| mark.index <= prefix).saturating_sub(1);
    let restart = old_marks[restart_at];
    let mut out = Rc::try_unwrap(std::mem::replace(&mut cache.out, Rc::new(Vec::new()))).unwrap_or_else(|shared| (*shared).clone());
    let mut old_tail = out.split_off(restart.out_len);
    let old_log = std::mem::take(&mut cache.stretch_log);
    let old_label_log = std::mem::take(&mut cache.current_label_log);
    let edit_start = changes.old.start;
    let old_edit_end = changes.old.end;
    let document = DocumentId(entry);
    let shift = move |sp: Span| -> Span {
        if sp.document == document && sp.start >= old_edit_end {
            Span::in_document(sp.document, (sp.start as isize + delta) as usize, (sp.end as isize + delta) as usize)
        } else {
            sp
        }
    };

    let mut conv = Converter::new(documents, entry);
    conv.out = out;
    conv.last_span = restart.last_span;
    // Records whose marker precedes the restart point are unchanged; later
    // ones are regenerated or come back with the spliced suffix.
    conv.stretch_log = old_log.iter().filter(|(index, _, _)| *index < restart.index).cloned().collect();
    conv.arraystretch = stretch_map(&conv.stretch_log);
    conv.current_label_log =
        old_label_log.iter().filter(|(index, _, _)| *index < restart.index).cloned().collect();
    conv.current_label_by_marker = stretch_map(&conv.current_label_log);
    let _ = edit_start;
    let mut marks: Vec<Mark> = old_marks[..=restart_at].to_vec();
    let old_count = cache.engine_token_count(n_new, &stats);
    let token_offset = n_new as isize - old_count as isize;
    let join = (suffix > 0).then(|| Join {
        from: n_new - suffix,
        offset: n_new as isize - old_count as isize,
        old_marks: &old_marks,
        old_tail: &mut old_tail,
        base: restart.out_len,
        shift: &shift,
    });
    let spliced = convert_range(&mut conv, prepared, &cache.expander, restart.index, &mut marks, join);
    match spliced {
        Some(old_mark) => {
            for (index, key, text) in old_log.into_iter().filter(|(index, _, _)| *index >= old_mark.index) {
                let key = if key.0 == entry && key.1 >= old_edit_end {
                    (key.0, (key.1 as isize + delta) as usize)
                } else {
                    key
                };
                conv.stretch_log.push(((index as isize + token_offset) as usize, key, text.clone()));
                conv.arraystretch.insert(key, text);
            }
            for (index, key, text) in
                old_label_log.into_iter().filter(|(index, _, _)| *index >= old_mark.index)
            {
                let key = if key.0 == entry && key.1 >= old_edit_end {
                    (key.0, (key.1 as isize + delta) as usize)
                } else {
                    key
                };
                conv.current_label_log
                    .push(((index as isize + token_offset) as usize, key, text.clone()));
                conv.current_label_by_marker.insert(key, text);
            }
            conv.last_span = shift(cache.last_span);
        }
        None => conv.flush_word(),
    }
    cache.marks = marks;
    cache.last_span = conv.last_span;
    finish(cache, conv)
}

impl ExpansionCache {
    /// Engine token count of the previous run, from this edit's statistics.
    fn engine_token_count(&self, n_new: usize, stats: &tex::EditStats) -> usize {
        // new = prefix + expanded + suffix; old = prefix + (old middle) + suffix,
        // and the old middle is what the last recorded mark index tells.
        let _ = (n_new, stats);
        self.old_engine_tokens
    }
}

/// Store the converted stream in the cache and map the engine diagnostics.
/// `None` when expansion ran away (the caller falls back to the full path).
fn finish(cache: &mut ExpansionCache, conv: Converter<'_>) -> Option<Expansion> {
    let mut conv = conv;
    let out = std::mem::take(&mut conv.out);
    cache.stretch_log = conv.stretch_log.clone();
    cache.current_label_log = conv.current_label_log.clone();
    cache.last_span = conv.last_span;
    cache.old_engine_tokens = cache.expander.tokens().len();
    let tokens = Rc::new(out);
    cache.out = tokens.clone();
    let mut expansion = finish_diagnostics(cache, conv)?;
    expansion.tokens = tokens;
    Some(expansion)
}

fn stretch_map(log: &[(usize, (usize, usize), String)]) -> HashMap<(usize, usize), String> {
    log.iter().map(|(_, key, text)| (*key, text.clone())).collect()
}

fn finish_diagnostics(cache: &ExpansionCache, mut conv: Converter<'_>) -> Option<Expansion> {
    if step_limit_hit(cache.expander.diagnostics()) {
        return None;
    }
    conv.last_span = cache.last_span;
    conv.map_diagnostics(cache.expander.diagnostics());
    Some(Expansion {
        tokens: Rc::new(Vec::new()),
        diagnostics: conv.diagnostics,
        arraystretch: conv.arraystretch,
        current_label_by_marker: conv.current_label_by_marker,
    })
}

fn recovery_for(message: &str) -> &'static str {
    if message.contains("LaTeX Error: Command") && message.contains("already defined") {
        "kept the existing command definition"
    } else if message.contains("LaTeX Error: Command") && message.contains("undefined") {
        "defined the command anyway"
    } else if message.contains("limit exceeded") {
        "stopped expanding; the rest of the document was typeset without macro expansion"
    } else {
        "continued expanding after the problem"
    }
}

/// Read a `{...}` group through the engine (macro-expanding its content),
/// the way `\input`/`\include`/`\includeonly` arguments are read: the taken
/// engine tokens (for re-queueing when no group follows), the group text,
/// and whether a group closed.
fn read_braced_argument(engine: &mut Engine) -> (Vec<(tex::Token, Option<tex::Span>)>, String, bool) {
    let mut taken = Vec::new();
    let mut path = String::new();
    let mut ok = false;
    let mut depth = 0usize;
    while let Some((t, o)) = engine.next_content_token_with_origin() {
        let kind = t.kind.clone();
        taken.push((t, o));
        match kind {
            TexKind::Char(_, CatCode::Space) if depth == 0 => {}
            TexKind::Char(_, CatCode::BeginGroup) => {
                depth += 1;
                if depth > 1 {
                    path.push('{');
                }
            }
            TexKind::Char(_, CatCode::EndGroup) if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    ok = true;
                    break;
                }
                path.push('}');
            }
            _ if depth == 0 => break,
            TexKind::Char(c, _) | TexKind::ActiveChar(c) => path.push(c),
            TexKind::ControlSequence(cs) => {
                path.push('\\');
                path.push_str(&cs);
            }
            _ => {}
        }
    }
    (taken, path, ok)
}

/// True while `\begin{document}` has not been emitted yet: `\includeonly`
/// is a preamble-only command (real LaTeX's `\@onlypreamble`), and
/// expansion runs in document order, so the converter's own flag is the
/// boundary — no raw-text scan. Fragments without a document environment
/// never set the flag, so every position counts as preamble there. An
/// `\includeonly` in an `\input`-ed preamble config file counts (it is
/// processed before the boundary), and a commented `% \begin{document}`
/// emits nothing, so it cannot end the preamble early.
fn in_preamble(conv: &Converter<'_>) -> bool {
    !conv.document_begun
}

/// Record a preamble `\includeonly{...}` list for [`include`]: comma-split
/// and trimmed (real LaTeX allows spaces after commas), each entry kept both
/// as written and stripped of one trailing `.tex`, mirroring [`include`]'s
/// exact-then-`+.tex` lookup. Like real LaTeX's `\let\@partlist\@empty`
/// reset, each call completely replaces the recorded set, so the last call
/// wins. A post-preamble use warns and is ignored (`diagnostics.rs` carries
/// no preamble-only pattern to reuse).
fn record_includeonly(conv: &mut Converter<'_>, list: &str, span: Span) {
    if !in_preamble(conv) {
        conv.diagnostics.push(Diagnostic::warning(
            "\\includeonly must appear in the preamble; ignored this use and continued",
            Some(span),
            Some("ignored the misplaced \\includeonly and continued".into()),
        ));
        return;
    }
    let mut set = HashSet::new();
    for name in list.split(',') {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        set.insert(name.to_string());
        if let Some(stripped) = name.strip_suffix(".tex") {
            set.insert(stripped.to_string());
        }
    }
    conv.includeonly = Some(set);
}

/// True when `requested` names a file in the recorded `\includeonly` set:
/// the trimmed name itself, the name minus one `.tex`, or the name plus
/// `.tex` — the same two-way match [`include`] performs against the project
/// documents.
fn include_allowed(allowed: &HashSet<String>, requested: &str) -> bool {
    let requested = requested.trim();
    if allowed.contains(requested) {
        return true;
    }
    match requested.strip_suffix(".tex") {
        Some(stripped) => allowed.contains(stripped),
        None => allowed.contains(&format!("{requested}.tex")),
    }
}

fn include(
    conv: &mut Converter<'_>,
    engine: &mut Engine,
    prepared: &[Prepared<'_>],
    command: &str,
    requested: &str,
    span: Span,
) {
    let skip = |conv: &mut Converter<'_>, message: String, recovery: &str| {
        conv.diagnostics.push(Diagnostic::error(message, Some(span), Some(recovery.into())));
    };
    if requested.is_empty() {
        return skip(
            conv,
            format!("\\{command} requires a non-empty project-relative path"),
            "skipped the empty include and continued",
        );
    }
    if !path_is_safe(requested) {
        return skip(
            conv,
            format!("rejected include path '{requested}': paths must be project-relative with no parent traversal"),
            "skipped the unsafe include and continued",
        );
    }
    // A recorded, non-empty `\includeonly` list selects which `\include`d
    // files are read: any other file is a pure no-op. (`\include` has no
    // page-break or paragraph-flush side effect of its own, so there is
    // nothing to replay for the skipped file.) `\input` never consults the
    // list — real LaTeX tests `\@partlist` only in `\@include`. With no
    // `\includeonly` at all (`None`), every `\include` behaves exactly as
    // before; a recorded list — even an empty one from `\includeonly{}`,
    // which switches `\@partsw` on with an empty `\@partlist` — selects.
    if command == "include" {
        if let Some(allowed) = conv.includeonly.as_ref() {
            if !include_allowed(allowed, requested) {
                return;
            }
        }
    }
    let appended = format!("{requested}.tex");
    let Some(index) = conv
        .document_by_path
        .get(requested)
        .copied()
        .or_else(|| conv.document_by_path.get(appended.as_str()).copied())
    else {
        return {
            conv.diagnostics.push(Diagnostic::error(
                format!("included file not found: looked for '{requested}' and '{appended}'"),
                Some(span),
                Some("skipped the missing include and continued".into()),
            )
            .with_help(format!(
                "add '{requested}' or '{appended}' to the project documents, or fix the \\input path"
            )));
        };
    };
    let open: Vec<usize> = engine
        .open_input_ids()
        .into_iter()
        .filter_map(|id| conv.source_documents.get(&id).copied().flatten())
        .collect();
    if let Some(cycle_start) = open.iter().position(|active| *active == index) {
        let mut cycle: Vec<&str> = open[cycle_start..].iter().map(|i| conv.documents[*i].path).collect();
        cycle.push(conv.documents[index].path);
        return skip(
            conv,
            format!("include cycle detected: {}", cycle.join(" -> ")),
            "skipped the cyclic include and continued",
        );
    }
    if open.len() > INCLUDE_DEPTH_LIMIT {
        return skip(
            conv,
            format!(
                "include depth exceeds the limit of {INCLUDE_DEPTH_LIMIT} while loading '{}'",
                conv.documents[index].path
            ),
            "skipped the too-deep include and continued",
        );
    }
    let id = engine.push_input(prepared[index].text.as_ref());
    conv.source_documents.insert(id, Some(index));
}

#[cfg(test)]
mod tests {
    use super::after_bracket_option;

    /// The byte index just past the options that `after_bracket_option`
    /// finds in `text` (whose `[` follows `\begin{lstlisting}` at index 0).
    fn options_of(text: &str) -> &str {
        &text[..after_bracket_option(text, 0)]
    }

    #[test]
    fn bracket_option_ends_at_the_first_unbraced_bracket() {
        assert_eq!(options_of("[language=C]\nx]"), "[language=C]");
        assert_eq!(options_of("[caption={[Short]Long}]\nx]"), "[caption={[Short]Long}]");
    }

    /// A backslash takes the next byte with it: `\]` does not close the
    /// options, and `\{` / `\}` do not change the brace depth.
    #[test]
    fn bracket_option_skips_escaped_bytes() {
        assert_eq!(options_of("[caption=Has a \\] mark]\nx]"), "[caption=Has a \\] mark]");
        assert_eq!(options_of("[caption={Open \\{ only}]\nx]"), "[caption={Open \\{ only}]");
        assert_eq!(options_of("[caption=Close \\} only]\nx]"), "[caption=Close \\} only]");
        assert_eq!(options_of("[caption=Two \\\\]\nx]"), "[caption=Two \\\\]");
    }

    #[test]
    fn bracket_option_without_a_close_leaves_the_body_start() {
        assert_eq!(after_bracket_option("[caption={open]", 0), 0);
        assert_eq!(after_bracket_option("\n\n[language=C]", 0), 0);
    }
}
