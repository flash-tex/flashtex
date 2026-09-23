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
//!   `verbatim*`, `lstlisting` and `comment` are hidden from the engine
//!   before it reads the source (their bytes are blanked in a private
//!   copy; offsets do not move), so no `%`, `\`, `$`, `{`, `}` or macro
//!   in them is interpreted. The parser keeps reading the verbatim-like
//!   regions from the original text; `comment` bodies are discarded there
//!   instead (see `parser`'s `comment` environment handling).
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
//! - **Packages and classes.** `\usepackage`/`\RequirePackage`/
//!   `\documentclass`/`\LoadClass` read project `.sty`/`.cls` documents
//!   through the engine's package reader ([`crate::packages::reader`]);
//!   their tokens carry the file's own [`DocumentId`], and the loading
//!   command's span is recorded in [`Expansion::package_files`]. Names the
//!   reader declines (built-in models, missing files) reach the parser as
//!   before.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

use flashtex_tex_expansion::{self as tex, CatCode, Edit, Engine, IncrementalExpander, Limits, OpenedFile, PackageReader, TokenKind as TexKind};

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
    /// Every project `.sty`/`.cls` the engine read, with the span of the
    /// `\usepackage`/`\documentclass`/... that loaded it, in loading order.
    pub package_files: Vec<(DocumentId, Span)>,
    /// The same files with what each defined (`crate::package_definitions`),
    /// parallel to `package_files`.
    pub package_records: Vec<crate::package_definitions::PackageRecord>,
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
/// `\\setlength`/`\\addtolength` are the engine's (`Primitive::SetLength`):
/// every kernel length is a register there, set from the class's measured
/// defaults ([`class_prelude`]), so the assignment executes with TeX's own
/// `<glue>` grammar (`\\p@`, `\\@plus`, `0.5\\textwidth`, `\\dimexpr`) and
/// `\\the`/`\\ifdim`/`\\advance` read it back. The lengths this parser keeps
/// a copy of ([`crate::parser::OBSERVED_LENGTHS`]) are observed: the engine
/// emits a `\\flashtexlengthset{\\name}{<value>}` marker after each
/// assignment, which the converter hands the parser as the marker command
/// (see its `length_marker` arm). A target that is no register (a package
/// length the parser models, `\\LTleft`, `\\headrulewidth`) comes back as
/// `\\flashtexsetlength{#1}{#2}`, mapped to the original command, so the
/// parser sees the name with its argument still a control sequence.
///
/// `\\AtBeginDocument` keeps the kernel queuing behavior, but wraps each
/// queued chunk in `\\flashtexatbeginstart...\\flashtexatbeginend` markers
/// (left undefined, so the engine passes them through): the converter holds
/// marked output back and re-emits it after `\\begin{document}` closes, so
/// the hook typesets ahead of the body instead of being dropped as
/// preamble. Like the kernel's `\\g@addto@macro`, the append routes through
/// the `\\toks@` register so the chunk is stored unexpanded and only
/// resolves when the hook runs at `\\begin{document}` (a bare `\\xdef` would
/// bake preamble definitions in eagerly). Calls after `\\begin{document}`
/// bypass the wrapper entirely (`\\AtBeginDocument` is `\\let` to
/// `\\@firstofone` by then) and run in the body, as in LaTeX.
///
/// `\\setlist` routes through a host primitive that absorbs `[<names>]`
/// and `{<options>}` with one expansion pass but no execution, then pushes
/// the reconstructed command back for the main loop. Without this, a bare
/// length register in a value (`leftmargin=\mylen`) reaches the stomach
/// as a register assignment, whose dimension scan reports "Missing number"
/// and "Illegal unit of measure" on whatever follows it; real enumitem
/// stores the keyval text unexecuted, where the bare register is already a
/// complete dimension. The star is preserved for the parser.
///
/// `etoolbox`'s toggle booleans (`\newtoggle`/`\providetoggle` declare a
/// false toggle, `\toggletrue`/`\togglefalse` set it,
/// `\iftoggle{name}{true}{false}` selects a branch) mirror etoolbox.sty's
/// own representation: `etb@tgl@<name>` `\let` to `\@firstoftwo` (true) or
/// `\@secondoftwo` (false), tested with the kernel's `\@ifundefined` under
/// `\makeatletter` (so a `\relax`-valued name still counts as undefined,
/// and `@` tokenizes as a letter). The document engine has no `\errmessage`
/// (it exists only on the INITEX probe path), so a duplicate `\newtoggle`
/// and any use of an undefined toggle expand to a never-defined marker
/// (`\etb@err@toggledefined` / `\etb@err@notoggle`, the latter named after
/// etoolbox.sty's own error site): the parser reports it as an
/// `unknown_command` error at the use span while existing state is left
/// alone, matching the package's error-and-continue recovery. Like the
/// package, `\iftoggle` takes only the name: the two branches stay braced
/// in the input so `\@firstoftwo`/`\@secondoftwo` select whole groups (a
/// three-argument form would strip the braces and select single tokens).
/// The definitions are `\protected`, as the package's `\newrobustcmd*`
/// ones are, and always installed, exactly like the `ifthen` primitives.
/// Engine identity (`iftex.sty` under pdfTeX): this compiler is
/// pdflatex-equivalent, so `\ifxetex`/`\ifluatex` are defined false here --
/// exactly as `iftex.sty` leaves them when neither `\XeTeXrevision` nor
/// `\directlua` exists -- with `\ifXeTeX`/`\ifLuaTeX` let to the same
/// switches as that package does. The `.sty` files themselves are never
/// executed (`\usepackage{iftex}` and the legacy `ifxetex`/`ifluatex` are
/// silent layout-neutral loads), so a guarded block
/// (`\ifxetex\usepackage{fontspec}...\fi`) skips with no diagnostic, matching
/// pdflatex's exit-0 behavior on the same input.
/// `\hspace`/`\vspace` route through host primitives the same way (the
/// star and `{<dimen>}` are absorbed, a bare or factored register is
/// spliced to its current value text); real LaTeX absorbs those arguments
/// unexpanded as macro parameters.
pub const HOST_PRELUDE: &str = "\\let\\label\\flashtexundefined
\\let\\verb\\flashtexundefined
\\let\\:\\flashtexundefined
\\let\\counterwithin\\flashtexundefined
\\let\\counterwithout\\flashtexundefined
\\let\\fnsymbol\\flashtexundefined
\\newif\\ifxetex\\xetexfalse
\\newif\\ifluatex\\luatexfalse
\\let\\ifXeTeX\\ifxetex
\\let\\ifLuaTeX\\ifluatex
\\def\\setlist{\\flashtexsetlist}%
\\makeatletter
\\protected\\def\\newtoggle#1{\\@ifundefined{etb@tgl@#1}{\\expandafter\\let\\csname etb@tgl@#1\\endcsname\\@secondoftwo}{\\etb@err@toggledefined}}%
\\protected\\def\\providetoggle#1{\\@ifundefined{etb@tgl@#1}{\\expandafter\\let\\csname etb@tgl@#1\\endcsname\\@secondoftwo}{}}%
\\protected\\def\\toggletrue#1{\\@ifundefined{etb@tgl@#1}{\\etb@err@notoggle}{\\expandafter\\let\\csname etb@tgl@#1\\endcsname\\@firstoftwo}}%
\\protected\\def\\togglefalse#1{\\@ifundefined{etb@tgl@#1}{\\etb@err@notoggle}{\\expandafter\\let\\csname etb@tgl@#1\\endcsname\\@secondoftwo}}%
\\protected\\def\\iftoggle#1{\\@ifundefined{etb@tgl@#1}{\\etb@err@notoggle\\@gobbletwo}{\\csname etb@tgl@#1\\endcsname}}%
\\makeatother
\\def\\hspace{\\flashtexhspace}%
\\def\\vspace{\\flashtexvspace}%
\\long\\def\\flashtexdeclaremathop#1#2#3{\\newcommand#2{\\operatorname#1{#3}}}%
\\expandafter\\def\\expandafter\\DeclareMathOperator\\expandafter{\\csname @ifstar\\endcsname{\\flashtexdeclaremathop*}{\\flashtexdeclaremathop{}}}%
\\def\\arraystretch{1}%
\\def\\tabular{\\flashtexbegintabular\\expandafter{\\arraystretch}}%
\\expandafter\\def\\csname tabular*\\endcsname{\\flashtexbegintabularstar\\expandafter{\\arraystretch}}%
\\def\\array{\\flashtexbeginarray\\expandafter{\\arraystretch}}%
\\begingroup\\catcode32=13\\relax
\\gdef\\flashtexallttspaceinit{\\catcode32=13\\relax\\def {\\flashtexallttspace}}%
\\endgroup
\\begingroup\\catcode13=13\\relax
\\gdef\\flashtexallttlineinit{\\catcode13=13\\relax\\def^^M{\\flashtexallttnewline}}%
\\endgroup
\\def\\alltt{\\flashtexbeginalltt\\catcode37=12\\relax\\catcode35=12\\relax\\catcode36=12\\relax\\catcode38=12\\relax\\catcode94=12\\relax\\catcode95=12\\relax\\catcode126=12\\relax\\flashtexallttspaceinit\\flashtexallttlineinit}%
\\def\\endalltt{\\flashtexendalltt}%
\\long\\def\\flashtexaddtobeginhook#1#2{\\begingroup\\csname toks@\\endcsname\\expandafter{#1\\flashtexatbeginstart#2\\flashtexatbeginend}\\xdef#1{\\the\\csname toks@\\endcsname}\\endgroup}%
\\long\\def\\AtBeginDocument#1{\\expandafter\\flashtexaddtobeginhook\\csname @begindocumenthook\\endcsname{#1}}%
\\makeatletter
\\let\\flashtexrealrefstepcounter\\refstepcounter
\\def\\refstepcounter#1{\\flashtexrealrefstepcounter{#1}\\flashtexcurrentlabelmarker\\expandafter{\\@currentlabel}}%
\\def\\@startsection#1#2#3#4#5#6{\\par\\@tempskipa #4\\relax\\@afterindenttrue\\ifdim \\@tempskipa <\\z@ \\@tempskipa -\\@tempskipa \\@afterindentfalse\\fi\\@ifstar{\\@ssect{#3}{#4}{#5}{#6}}{\\@dblarg{\\@sect{#1}{#2}{#3}{#4}{#5}{#6}}}}%
\\def\\@sect#1#2#3#4#5#6[#7]#8{\\@tempdima #3\\relax\\@tempskipa #4\\relax\\@tempskipb #5\\relax\\flashtexsect{#1}{#2}{\\ifnum #2>\\c@secnumdepth 0\\else 1\\fi}{\\the\\@tempdima}{\\the\\@tempskipa}{\\the\\@tempskipb}{#6}{#7}{#8}}%
\\def\\@ssect#1#2#3#4#5{\\@tempdima #1\\relax\\@tempskipa #2\\relax\\@tempskipb #3\\relax\\flashtexsect{}{0}{0}{\\the\\@tempdima}{\\the\\@tempskipa}{\\the\\@tempskipb}{#4}{}{#5}}%
\\def\\@xsect#1{\\@tempskipa #1\\relax\\ifdim \\@tempskipa>\\z@ \\par\\nobreak\\vskip \\@tempskipa\\fi\\ignorespaces}%
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
        || text.contains("lstlisting")
        || text.contains("comment"))
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
                if !matches!(env.as_str(), "verbatim" | "verbatim*" | "lstlisting" | "comment") {
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
                if !matches!(env.as_str(), "lstlisting" | "comment") {
                    // `comment` needs no entry: unlike `verbatim`, the
                    // LaTeX kernel defines no `comment` environment, so
                    // the engine can never read one of its bodies itself
                    // and close it with a frozen `\endcomment`.
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
    /// Some project document literally loads biblatex (`\usepackage` or
    /// `\RequirePackage` naming it): gates the `\printbibliography` `.bbl`
    /// fallback. A raw-token scan like [`document_fonts`]; a
    /// macro-generated `\usepackage` is missed, like there.
    biblatex: bool,
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
    /// While true, converted tokens are `\AtBeginDocument` hook output that
    /// must typeset after `\begin{document}`: they accumulate in
    /// `atbegin_buffer` instead of `out`. Set by the
    /// `\flashtexatbeginstart` marker the host prelude wraps each queued
    /// hook chunk in; cleared by its end marker, and forcibly by the real
    /// `\begin{document}` re-emission (the end marker can be swallowed when
    /// a hook chunk ends in an argument-taking macro).
    atbegin_capturing: bool,
    /// Hook output held back while `atbegin_capturing` (possibly across
    /// several `\AtBeginDocument` chunks), flushed into `out` right after
    /// the real `\begin{document}` closes.
    atbegin_buffer: Vec<ExpandedToken>,
    /// Every `\@currentlabel` record in production order, with its marker's
    /// engine token index (kept and spliced like `stretch_log`).
    current_label_log: Vec<(usize, (usize, usize), String)>,
    /// Package files mapped so far (see [`Expansion::package_files`]).
    package_files: Vec<(DocumentId, Span)>,
    /// The same, with each file's engine source id: what
    /// [`Converter::package_records`] pairs with the engine's final records.
    package_sites: Vec<(u32, DocumentId, Span)>,
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
            // Assembled by `push_char` from ordinary characters only —
            // control symbols bypass the pending word via `push_marked` —
            // so this is never an escaped character.
            self.emit(ExpandedToken {
                token: Token {
                    kind: TokenKind::Word(word.text),
                    span: word.span,
                    control_symbol: false,
                },
                definition: word.definition,
                maps_to_invocation: word.maps,
            });
        }
    }

    /// Push one finished token to the active sink: the held-back hook buffer
    /// while capturing `\AtBeginDocument` output, the parser stream
    /// otherwise.
    fn emit(&mut self, token: ExpandedToken) {
        if self.atbegin_capturing {
            self.atbegin_buffer.push(token);
        } else {
            self.out.push(token);
        }
    }

    /// End of the `\AtBeginDocument` window (the real `\begin{document}`
    /// close, or end of input as a safety net): re-emit the held-back hook
    /// run after the marker, so it typesets ahead of the body. The pending
    /// word is flushed first so it keeps engine order — it belongs to the
    /// hook when capture is still on, to the stream once it is off.
    fn drain_atbegin(&mut self) {
        self.flush_word();
        self.atbegin_capturing = false;
        if !self.atbegin_buffer.is_empty() {
            let buffered = std::mem::take(&mut self.atbegin_buffer);
            self.out.extend(buffered);
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
        self.push_marked(kind, at, false);
    }

    /// Push one finished token, recording whether it was lexed from a
    /// backslash control symbol (see [`Token::control_symbol`]). Only the
    /// single-character control-sequence arm below passes `true`; every
    /// other converter output is an ordinary token.
    fn push_marked(&mut self, kind: TokenKind, at: Placement, control_symbol: bool) {
        self.flush_word();
        if matches!(kind, TokenKind::RBrace) && self.closes_document_begin() {
            self.document_begun = true;
        }
        self.last_span = at.span;
        // Whitespace runs collapse the way the parser's own tokenizer
        // produces them: one `Space`, or one `ParBreak` if the run holds a
        // paragraph break. While capturing `\AtBeginDocument` output the
        // run collapses against the held-back buffer, never the frozen
        // stream behind it.
        let sink = if self.atbegin_capturing {
            &mut self.atbegin_buffer
        } else {
            &mut self.out
        };
        match (&kind, sink.last().map(|t| &t.token.kind)) {
            (TokenKind::Space, Some(TokenKind::Space | TokenKind::ParBreak)) => return,
            (TokenKind::ParBreak, Some(TokenKind::ParBreak)) => return,
            (TokenKind::ParBreak, Some(TokenKind::Space)) => {
                sink.pop();
            }
            _ => {}
        }
        sink.push(ExpandedToken {
            token: Token { kind, span: at.span, control_symbol },
            definition: at.definition,
            maps_to_invocation: at.maps,
        });
    }

    fn push_char(&mut self, c: char, at: Placement) {
        self.last_span = at.span;
        if let Some(word) = &mut self.word {
            // Characters the engine synthesized from one token (`\the`'s
            // digits, the canonical operand of `\hskip`/`\kern`/`\penalty`,
            // a length marker's value) all carry that token's span: an
            // identical span is contiguous too, so `12.0pt` reaches the
            // parser as one word, as the source text `12pt` would.
            let contiguous = word.maps == at.maps
                && if at.maps {
                    word.span == at.span
                        && match (word.definition, at.definition) {
                            (Some(a), Some(b)) => a.document == b.document && (a.end == b.start || a == b),
                            (None, None) => true,
                            _ => false,
                        }
                } else {
                    word.span.document == at.span.document
                        && (word.span.end == at.span.start || word.span == at.span)
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
        self.emit(ExpandedToken {
            token: Token {
                kind: TokenKind::Word(name.to_string()),
                span: word_at.span,
                control_symbol: false,
            },
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
        match package_of_built_in(name) {
            // A package's or a class's command exists only once that file is
            // loaded (`\usepackage{siunitx}`, `\documentclass{letter}`); before
            // that a document's own `\newcommand{\si}`/`\newcommand{\cc}` is
            // free, as in LaTeX (parity 2026-09-23 cause 4).
            Some(file) => engine.declare_host_command_after(name, file),
            None => engine.declare_host_command(name),
        }
    }
    engine.declare_host_command("include");
    for name in KERNEL_ENVIRONMENTS {
        engine.declare_host_command(name);
        engine.declare_host_command(&format!("end{name}"));
    }
    // `\global\setlength{\parskip}{..}` is valid LaTeX: `\setlength` is a
    // macro, so TeX applies the prefix to the register assignment.
    engine.declare_host_assignment("flashtexsetlength");
    engine.declare_host_assignment("flashtexaddtolength");
    engine.declare_host_command("flashtexsetlistdone");
    for name in [
        "flashtexbeginalltt",
        "flashtexendalltt",
        "flashtexallttspace",
        "flashtexallttnewline",
    ] {
        engine.declare_host_command(name);
    }
    engine.declare_host_command("flashtexhspacedone");
    engine.declare_host_command("flashtexvspacedone");
    engine.declare_host_command("flashtexsect");
    // NFSS `\fontsize`/`\selectfont` run in the engine (`\set@fontsize`
    // records `\f@size`/`\f@baselineskip`, `\size@update` sets
    // `\baselineskip`), then hand the command back under these names so
    // the parser sees `\fontsize{<f@size>}{<f@baselineskip>}` and
    // `\selectfont` exactly as it did.
    engine.declare_host_command("flashtexfontsizedone");
    engine.declare_host_command("flashtexselectfontdone");
}

/// The file that provides a `BUILT_INS` name when it is not the LaTeX
/// kernel's or every standard class's: siunitx's commands and letter.cls's
/// (`\cc`, `\ps`, `\address`, ...). Such a name is declared to the engine
/// only once that file is loaded (`Engine::declare_host_command_after`).
fn package_of_built_in(name: &str) -> Option<&'static str> {
    match name {
        "num" | "qty" | "unit" | "si" | "SI" | "numlist" | "numrange" | "qtylist" | "qtyrange" | "SIlist"
        | "SIrange" | "ang" | "sisetup" | "DeclareSIUnit" => Some("siunitx.sty"),
        "address" | "signature" | "name" | "location" | "telephone" | "opening" | "closing" | "cc" | "encl"
        | "ps" | "startbreaks" | "stopbreaks" | "stopletter" | "makelabels" => Some("letter.cls"),
        _ => None,
    }
}

/// The expansion engine's `em`/`ex` come from the text font its tracked font
/// commands select ([`crate::font_units`]); its `\usepackage` files from
/// the project's package reader. `soul` is [`uses_soul`]: soul.sty owns
/// `\so`/`\hl`, so with soul loaded they become host commands — still
/// emitted unchanged for the parser's soul arms, but counting as defined,
/// so `\newcommand` refuses them and `\renewcommand` accepts them, as in
/// LaTeX. There is no per-package load hook in the engine to do this at
/// the `\usepackage` itself (no built-in package claims names on load;
/// even `appendix` registers nothing), so the claim happens here at
/// configure time, from the same raw-token preamble scan `document_fonts`
/// and [`uses_biblatex`] already use. Package presence is therefore
/// order-independent, like the parser's own `self.packages` gate: a
/// `\newcommand{\hl}` *before* `\usepackage{soul}` is refused at the
/// `\newcommand` rather than at the load, where real pdflatex refuses the
/// redefinition the other way round (soul.sty's own `\newcommand`).
fn configure_with_fonts(
    engine: &mut Engine,
    fonts: DocumentFonts,
    soul: bool,
    reader: PackageReader,
) {
    configure(engine);
    if soul {
        engine.declare_host_command("so");
        engine.declare_host_command("hl");
    }
    engine.set_package_reader(reader);
    for (name, switch) in crate::font_units::font_switches() {
        engine.declare_font_switch(name, switch);
    }
    engine.set_font_metrics(Rc::new(crate::font_units::EngineFontMetrics {
        setup: fonts.setup,
        preamble_latin_modern: fonts.preamble_latin_modern,
    }));
    // The class's measured lengths, then the names whose assignments come
    // back as markers for the parser (see `HOST_PRELUDE`).
    engine.run_host_prelude(&class_prelude(&fonts.class));
    for name in crate::parser::OBSERVED_LENGTHS.iter().chain(crate::parser::OBSERVED_COUNTERS) {
        engine.observe_register(name);
    }
}

/// The document-wide font inputs the engine needs before it executes any
/// `\setlength`: the class size option, `fontenc` and `lmodern`, and the
/// class itself (whose measured lengths the engine's registers start from).
#[derive(Debug, Clone, PartialEq)]
struct DocumentFonts {
    setup: crate::font_units::FontSetup,
    /// `lmodern` was loaded before `\usepackage[T1]{fontenc}`, whose
    /// `\selectfont` then switches the preamble to Latin Modern already.
    preamble_latin_modern: bool,
    class: ClassSetup,
}

/// The `\documentclass` the entry names, with its options, and the
/// `\LoadClass` a project class file makes (see [`class_prelude`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ClassSetup {
    pub name: String,
    pub options: String,
    pub loaded: Option<(String, String)>,
}

/// The engine's starting register values for a document: the kernel
/// lengths, TeX parameters, float counters and float-fraction macros as
/// pdflatex has them at `\begin{document}` under the document's class,
/// size and layout options (`crate::kernel_lengths`, measured by
/// `scripts/gen_kernel_lengths.py`), as TeX text for a host prelude.
///
/// The class is the entry's `\documentclass` when the table has it, else
/// the class a project `.cls` `\LoadClass`es (the project file then sets its
/// own values on top, exactly as it does in LaTeX), else `article` for the
/// kernel-level values only: such a class allocates its own caption skips
/// and float fractions (or none), so none are declared for it. A size the
/// table lacks takes the nearest measured one; `a4paper`/`letterpaper`,
/// `twocolumn`/`onecolumn` and `twoside`/`oneside` select the measured
/// combination, the class's defaults filling in what the document leaves
/// unsaid.
pub(crate) fn class_prelude(class: &ClassSetup) -> String {
    use crate::kernel_lengths::{ClassDefaults, CLASSES};
    let by_name = |name: &str| CLASSES.iter().find(|c| c.class == name.trim());
    let (defaults, exact, options): (&ClassDefaults, bool, String) = match by_name(&class.name) {
        Some(c) => (c, true, class.options.clone()),
        None => match class.loaded.as_ref().and_then(|(name, opts)| by_name(name).map(|c| (c, opts))) {
            Some((c, opts)) => (c, true, format!("{},{}", class.options, opts)),
            None => (by_name("article").expect("article is measured"), false, class.options.clone()),
        },
    };
    let mut size: Option<f64> = None;
    let mut a4 = None;
    let mut twocolumn = None;
    let mut twoside = None;
    for option in options.split(',').map(str::trim) {
        let option = option.strip_prefix("fontsize=").unwrap_or(option);
        match option {
            "a4paper" => a4 = Some(true),
            "letterpaper" => a4 = Some(false),
            "twocolumn" => twocolumn = Some(true),
            "onecolumn" => twocolumn = Some(false),
            "twoside" => twoside = Some(true),
            "oneside" => twoside = Some(false),
            _ => {
                if let Some(pt) = option.strip_suffix("pt").and_then(|v| v.parse::<f64>().ok()) {
                    size.get_or_insert(pt);
                }
            }
        }
    }
    let pt_of = |s: &str| s.strip_suffix("pt").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let wanted = size.unwrap_or_else(|| pt_of(defaults.size));
    let nearest = defaults
        .rows
        .iter()
        .map(|r| r.size)
        .min_by(|a, b| (pt_of(a) - wanted).abs().partial_cmp(&(pt_of(b) - wanted).abs()).unwrap())
        .unwrap_or(defaults.size);
    let want = |given: Option<bool>, default: Option<bool>| given.or(default);
    let (a4, twocolumn, twoside) = (
        want(a4, defaults.a4paper),
        want(twocolumn, defaults.twocolumn),
        want(twoside, defaults.twoside),
    );
    let row = defaults.rows.iter().find(|r| {
        r.size == nearest
            && (r.a4paper.is_none() || r.a4paper == a4)
            && (r.twocolumn.is_none() || r.twocolumn == twocolumn)
            && (r.twoside.is_none() || r.twoside == twoside)
    });
    let mut text = String::from("\\makeatletter\n");
    let declared: HashSet<&str> = if exact { defaults.class_lengths.iter().copied().collect() } else { HashSet::new() };
    for name in &declared {
        text.push_str(&format!("\\newskip\\{name}\n"));
    }
    let mut registers: Vec<(&str, &str)> = defaults.registers.to_vec();
    let mut macros: Vec<(&str, &str)> = if exact { defaults.macros.to_vec() } else { Vec::new() };
    if let Some(row) = row {
        for (key, value) in row.delta {
            match key.strip_prefix('\\') {
                Some(macro_name) => {
                    if let Some(entry) = macros.iter_mut().find(|(n, _)| *n == macro_name) {
                        entry.1 = value;
                    }
                }
                None => {
                    if let Some(entry) = registers.iter_mut().find(|(n, _)| n == key) {
                        entry.1 = value;
                    }
                }
            }
        }
    }
    for (name, value) in registers {
        let class_only = defaults.class_lengths.contains(&name);
        if class_only && !declared.contains(name) {
            continue;
        }
        text.push_str(&format!("\\{name}={value}\n"));
    }
    for (name, value) in macros {
        text.push_str(&format!("\\def\\{name}{{{value}}}\n"));
    }
    // The kernel switches the standard classes set from their options
    // (`\@twosidetrue`, `\@twocolumntrue`, `\@titlepagetrue`, `\@openrighttrue`
    // in classes.dtx), so a project's `.cls`/`.sty` that tests
    // `\if@twoside`/`\if@titlepage` sees the class's answer. The switches
    // themselves live in the engine prelude; `\if@titlepage` and
    // `\if@openright` are class-level `\newif`s.
    let is_report_like = matches!(defaults.class.trim(), "report" | "book");
    text.push_str("\\newif\\if@titlepage\n\\newif\\if@openright\n");
    let titlepage = options.split(',').map(str::trim).fold(is_report_like, |acc, option| match option {
        "titlepage" => true,
        "notitlepage" => false,
        _ => acc,
    });
    let openright = options.split(',').map(str::trim).fold(is_report_like, |acc, option| match option {
        "openright" => true,
        "openany" => false,
        _ => acc,
    });
    for (flag, on) in [("twoside", twoside == Some(true)), ("twocolumn", twocolumn == Some(true)), ("titlepage", titlepage), ("openright", openright)] {
        if on {
            text.push_str(&format!("\\@{flag}true\n"));
        }
    }
    // NFSS's record of `\normalsize` after the class's size option
    // (`size1x.clo`: `\@setfontsize\normalsize\@xpt\@xiipt` etc.), which
    // `\fontsize`/`\@setfontsize` then update: `\f@size` 10/10.95/12 and
    // `\f@baselineskip` 12/13.6/14.5pt (pdflatex `\typeout` at
    // `\begin{document}` for the three options).
    let (f_size, f_baselineskip) = options.split(',').map(str::trim).fold(("10", "12.0pt"), |acc, option| match option {
        "10pt" => ("10", "12.0pt"),
        "11pt" => ("10.95", "13.6pt"),
        "12pt" => ("12", "14.5pt"),
        _ => acc,
    });
    text.push_str(&format!("\\def\\f@size{{{f_size}}}\\def\\f@baselineskip{{{f_baselineskip}}}\n"));
    text.push_str("\\makeatother\n");
    text
}

/// Environments `latex.ltx` and the standard classes define in TeX, which
/// this parser sets itself: declared to the engine as host commands
/// (`\name`/`\endname`), so `\renewenvironment{abstract}` in a project's
/// `.sty` redefines them, as in LaTeX, instead of reporting "Environment
/// abstract undefined". Package environments (`proof`, `align`,
/// `lstlisting`, ...) are not here: without their package a document's own
/// `\newenvironment{proof}` must succeed, exactly as in real LaTeX.
const KERNEL_ENVIRONMENTS: &[&str] = &[
    "document", "abstract", "titlepage", "array", "center", "flushleft", "flushright",
    "description", "displaymath", "enumerate", "eqnarray", "eqnarray*", "equation", "figure", "figure*",
    "filecontents", "filecontents*", "itemize", "list", "lrbox", "math", "minipage", "picture", "quotation",
    "quote", "samepage", "sloppypar", "tabbing", "table", "table*", "tabular", "tabular*", "thebibliography",
    "theindex", "trivlist", "verbatim", "verbatim*", "verse",
];

/// The words of `tokens` from `index` up to the next `{`, and the words of
/// that brace group.
fn option_and_group_words(tokens: &[Token], index: usize) -> (String, String) {
    let mut options = String::new();
    let mut group = String::new();
    let mut in_group = false;
    for token in &tokens[index..] {
        match &token.kind {
            TokenKind::LBrace if !in_group => in_group = true,
            TokenKind::RBrace if in_group => break,
            TokenKind::Word(word) if in_group => group.push_str(word),
            TokenKind::Word(word) => options.push_str(word),
            TokenKind::Space => {}
            _ if in_group => break,
            _ => {}
        }
    }
    (options.trim_matches(|c| matches!(c, '[' | ']')).to_string(), group)
}

/// `entry` is scanned first: its `\documentclass` size option wins over a
/// project class file's `\LoadClass[11pt]{article}` (as article.cls's
/// declaration order makes the later size win), whatever the document
/// order. Package and class files of built-in names are never loaded, so
/// they are not scanned.
fn document_fonts(documents: &[SourceDocument<'_>], entry: usize) -> DocumentFonts {
    let mut class_pt = None;
    let mut t1 = false;
    let mut latin_modern = false;
    let mut preamble_latin_modern = false;
    let mut class = ClassSetup::default();
    let order = std::iter::once(entry).chain((0..documents.len()).filter(|i| *i != entry));
    for document_index in order {
        let Some(document) = documents.get(document_index) else {
            continue;
        };
        if let Some((stem, ext)) = document.path.rsplit_once('.').filter(|(_, ext)| matches!(*ext, "sty" | "cls")) {
            let name = stem.rsplit('/').next().unwrap_or(stem);
            if crate::packages::is_built_in(name, ext) {
                continue;
            }
        }
        let tokens = tokenize_document(document.text, DocumentId(document_index));
        for (index, token) in tokens.iter().enumerate() {
            let TokenKind::Command(name) = &token.kind else {
                continue;
            };
            match name.as_str() {
                "documentclass" | "LoadClass" => {
                    let (options, group) = option_and_group_words(&tokens, index + 1);
                    // The first `\documentclass` names the class (with its
                    // options); a project class's first `\LoadClass` the one
                    // its values build on (see `class_prelude`).
                    if name == "documentclass" {
                        if class.name.is_empty() {
                            class.name = group.trim().to_string();
                            class.options = options.clone();
                        }
                    } else if class.loaded.is_none() {
                        class.loaded = Some((group.trim().to_string(), options.clone()));
                    }
                    if class_pt.is_some() {
                        continue;
                    }
                    class_pt = options.split(',').find_map(|option| match option.trim() {
                        "10pt" => Some(10.0),
                        "11pt" => Some(11.0),
                        "12pt" => Some(12.0),
                        _ => None,
                    });
                    // KOMA classes default to 11pt and take `fontsize=11pt`;
                    // the legacy `10pt`/`11pt`/`12pt` names above already won
                    // when present.
                    if class_pt.is_none()
                        && matches!(
                            group.trim(),
                            "scrartcl" | "scrarticle" | "scrreprt" | "scrbook"
                        )
                    {
                        class_pt = options
                            .split(',')
                            .filter_map(|option| {
                                option.trim().strip_prefix("fontsize=").map(str::trim)
                            })
                            .filter_map(|v| {
                                v.strip_suffix("pt").unwrap_or(v).parse::<f64>().ok()
                            })
                            .next()
                            .or(Some(11.0));
                    }
                }
                "usepackage" | "RequirePackage" => {
                    let (options, group) = option_and_group_words(&tokens, index + 1);
                    for package in group.split(',').map(str::trim) {
                        match package {
                            "lmodern" => latin_modern = true,
                            "fontenc" => {
                                if let Some(encoding) = crate::text_builtins::fontenc_encoding(&options) {
                                    t1 = encoding == flashtex_tex_text_encoding::encoding::Encoding::T1;
                                    preamble_latin_modern = latin_modern;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }
    DocumentFonts {
        setup: crate::font_units::FontSetup::new(class_pt, t1, latin_modern),
        preamble_latin_modern: preamble_latin_modern && t1,
        class,
    }
}

/// The engine's reader for the project's `.sty`/`.cls` documents, serving
/// the prepared text (see [`Prepared`]) of each, as `include` does.
fn package_reader(documents: &[SourceDocument<'_>], prepared: &[Prepared<'_>]) -> PackageReader {
    let files = documents
        .iter()
        .zip(prepared)
        .filter(|(d, _)| crate::packages::is_package_file(d.path))
        .map(|(d, p)| (d.path.to_string(), p.text.to_string()))
        .collect();
    crate::packages::reader(files)
}

fn has_includes(text: &str) -> bool {
    // `\bibliography` also matches `\bibliographystyle` (and `\include`
    // already matches `\includeonly`/`\includegraphics`): the superset only
    // sends more documents down the full path, never the cache, which is the
    // safe direction — a `.bbl` input makes expansion depend on another
    // project document, exactly like `\input` does.
    text.contains("\\input")
        || text.contains("\\include")
        || text.contains("\\bibliography")
        || text.contains("\\printbibliography")
}

/// The engine stopped on the step limit or on TeX's "capacity exceeded"
/// (input stack, main memory): the rest of the document is then typeset
/// unexpanded from where the engine stood. A stop on the output token limit
/// is not resumed (see [`tex::output_limit_message`]).
fn step_limit_hit(diagnostics: &[tex::Diagnostic]) -> bool {
    diagnostics.iter().any(|d| is_stop_limit(&d.message) && !tex::is_output_limit(&d.message))
}

/// Any stop on a resource limit, the output token limit included.
pub(crate) fn is_stop_limit(message: &str) -> bool {
    message.contains("expansion step limit exceeded") || message.starts_with("TeX capacity exceeded, sorry [")
}

/// What the caller must do after one converted token.
enum Flow {
    Next,
    /// A source-level `\input`/`\include`: its braced path is still to be
    /// read.
    Include(String, Placement),
    /// A source-level `\includeonly`: its braced list is still to be read.
    IncludeOnly(Placement),
    /// A source-level `\bibliography`: its braced database list is still to
    /// be read; the main loop turns it into `.bbl` inputs (see
    /// [`bibliography`]) or passes the command back for the parser's
    /// missing-bibliography diagnostic.
    Bibliography(Placement),
    /// A source-level `\printbibliography`: its optional bracket argument is
    /// still to be read; the main loop inputs the job's `.bbl` when biblatex
    /// is loaded and the project carries it.
    PrintBibliography(Placement),
}

impl<'d> Converter<'d> {
    fn new(documents: &'d [SourceDocument<'d>], entry: usize) -> Self {
        Converter {
            documents,
            document_by_path: documents.iter().enumerate().map(|(i, d)| (d.path, i)).collect(),
            includeonly: None,
            biblatex: uses_biblatex(documents),
            document_begun: false,
            source_documents: HashMap::from([(0, Some(entry))]),
            entry,
            out: Vec::new(),
            diagnostics: Vec::new(),
            arraystretch: HashMap::new(),
            word: None,
            last_span: Span::in_document(DocumentId(entry), 0, 0),
            atbegin_capturing: false,
            atbegin_buffer: Vec::new(),
            stretch: None,
            current_label: None,
            current_label_by_marker: HashMap::new(),
            index: 0,
            stretch_index: 0,
            current_label_index: 0,
            stretch_log: Vec::new(),
            current_label_log: Vec::new(),
            package_files: Vec::new(),
            package_sites: Vec::new(),
        }
    }

    /// Give the tokens of a `.sty`/`.cls` the engine opened their document:
    /// the file is resolved by the same rule the reader used
    /// (`crate::packages::resolve`), so the name the engine reports maps to
    /// the document whose text it read. The loading command's span is kept
    /// for the parser's "loaded here" label.
    fn map_opened(&mut self, file: &OpenedFile) {
        if self.source_documents.contains_key(&file.source_id) {
            return;
        }
        let (name, ext) = file.name.rsplit_once('.').unwrap_or((&file.name, ""));
        let index = crate::packages::resolve(self.documents, name, ext);
        self.source_documents.insert(file.source_id, index);
        if let Some(index) = index {
            let at = self.span(file.loaded_at).unwrap_or(self.last_span);
            self.package_files.push((DocumentId(index), at));
            self.package_sites.push((file.source_id, DocumentId(index), at));
        }
    }

    /// [`Expansion::package_records`] from the engine's opened files once
    /// the run is over: a file is mapped when it opens
    /// ([`Converter::map_opened`]), but what it defined is complete only
    /// when it has been read to the end.
    fn package_records<'f>(&self, files: impl Iterator<Item = &'f OpenedFile>) -> Vec<crate::package_definitions::PackageRecord> {
        let files: HashMap<u32, &OpenedFile> = files.map(|file| (file.source_id, file)).collect();
        self.package_sites
            .iter()
            .filter_map(|(source_id, document, at)| {
                let file = files.get(source_id)?;
                let path = self.documents[document.0].path;
                Some(crate::package_definitions::PackageRecord::from_opened(file, *document, *at, path, &|span| self.span(span)))
            })
            .collect()
    }

    /// No partially built word, `\arraystretch` capture, `\@currentlabel`
    /// capture, or held-back `\AtBeginDocument` output: the output so far
    /// does not depend on tokens still to come. (The incremental cache only
    /// records marks and splices while clean, so a capture window is always
    /// re-converted from an earlier mark with a fresh converter.)
    fn clean(&self) -> bool {
        self.word.is_none()
            && self.stretch.is_none()
            && self.current_label.is_none()
            && !self.atbegin_capturing
            && self.atbegin_buffer.is_empty()
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
                    // `\relax` produces nothing for the parser. Group
                    // boundaries open and close a parser group, so
                    // declarations stay scoped to it: `{...}` already
                    // arrives as braces, and `\begingroup`/`\endgroup`
                    // — literal ones as well as the pair the engine emits
                    // around every `\begin{...}`/`\end{...}` — arrive
                    // here.
                    "relax" => {}
                    "begingroup" => conv.push(TokenKind::LBrace, at),
                    "endgroup" => conv.push(TokenKind::RBrace, at),
                    // `\newblock`, the block separator every `.bst` style
                    // emits between an entry's author/title/journal blocks
                    // (the standard classes define it as a small horizontal
                    // space): an interword space. Mapped here, not in the
                    // parser, so it never reaches the supported-command
                    // inventory — it is spacing, not a feature. A package
                    // that redefines `\newblock` is expanded by the engine
                    // first, so its definition still wins.
                    "newblock" => conv.push(TokenKind::Space, at),
                    "flashtexsetlength" => conv.push(TokenKind::Command("setlength".to_string()), at),
                    "flashtexaddtolength" => {
                        conv.push(TokenKind::Command("addtolength".to_string()), at)
                    }
                    // The engine's observed-register markers
                    // (`Engine::observe_register`): `{\name}{<\the text>}`
                    // follows, read by the parser's `length_marker` arm.
                    "flashtexlengthset" | "flashtexlengthadd" | "flashtexlengthassign" => {
                        conv.push(TokenKind::Command(name.clone()), at)
                    }
                    // The host prelude's `\@sect`/`\@ssect`: the evaluated
                    // `\@startsection` parameters and the title, read by the
                    // parser's `startsection_marker`.
                    "flashtexsect" => conv.push(TokenKind::Command(name.clone()), at),
                    // Ends the operand of an engine-scanned `\hskip`/
                    // `\vskip`/`\kern`/`\penalty` (`Engine::emit_with_operand`):
                    // the pending word closes with no space after it, as
                    // TeX consumed the one optional space itself.
                    "flashtexwordbreak" => conv.flush_word(),
                    // `do_flashtex_setlist`'s absorbed-and-spliced command.
                    "flashtexsetlistdone" => conv.push(TokenKind::Command("setlist".to_string()), at),
                    // `do_flashtex_space`'s absorbed-and-spliced commands.
                    "flashtexhspacedone" => conv.push(TokenKind::Command("hspace".to_string()), at),
                    "flashtexvspacedone" => conv.push(TokenKind::Command("vspace".to_string()), at),
                    "flashtexfontsizedone" => conv.push(TokenKind::Command("fontsize".to_string()), at),
                    "flashtexselectfontdone" => conv.push(TokenKind::Command("selectfont".to_string()), at),
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
                    "flashtexbeginalltt" => conv.push_environment("begin", "alltt", at),
                    "flashtexendalltt" => conv.push_environment("end", "alltt", at),
                    "flashtexallttspace" => conv.push(TokenKind::Word(" ".to_string()), at),
                    "flashtexallttnewline" => conv.push(TokenKind::LineBreak, at),
                    // `\AtBeginDocument` hook output: the host prelude wraps
                    // every chunk queued before `\begin{document}` in these
                    // markers. The engine runs the hook ahead of the real
                    // `\begin{document}` re-emission, while the parser still
                    // drops pre-marker content as preamble — so hold the
                    // marked tokens back and re-emit them once the real
                    // `\begin{document}` closes (see the `document` arm
                    // below). The markers themselves are invisible.
                    "flashtexatbeginstart" => {
                        conv.flush_word();
                        conv.atbegin_capturing = true;
                    }
                    "flashtexatbeginend" => {
                        conv.flush_word();
                        conv.atbegin_capturing = false;
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
                    // A blank line's `\par` is a paragraph break, not the
                    // control word: its own bytes never start with a
                    // backslash. A file with no project document (a
                    // vendored real package, `crate::packages::APPENDIX_STY`)
                    // has no readable bytes, so its `real_text` is empty --
                    // still not a backslash, so its blank lines fold here
                    // too instead of reaching the parser as `\par` (an
                    // error in the preamble). A literal `\par` spelled in
                    // such a file folds the same way; in the body that
                    // typesets identically (`flush_paragraph` either way).
                    "par" if !real_text.starts_with('\\') => conv.push(TokenKind::ParBreak, at),
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
                    "bibliography" if origin.is_none() && real_text == "\\bibliography" => {
                        return Flow::Bibliography(at);
                    }
                    "printbibliography" if origin.is_none() && real_text == "\\printbibliography" => {
                        return Flow::PrintBibliography(at);
                    }
                    "includeonly" if origin.is_none() && real_text == "\\includeonly" => {
                        return Flow::IncludeOnly(at);
                    }
                    // `\-` (the discretionary hyphen) stays a command: it is
                    // not the character it looks like.
                    "-" => conv.push(TokenKind::Command(name.clone()), at),
                    _ if name.chars().count() == 1 && !name.chars().all(char::is_alphabetic) => {
                        conv.flush_word();
                        // A backslash control symbol (`\,`, `\%`, …): the
                        // same `Word` variant also carries ordinary literal
                        // characters, so the escaped identity is recorded in
                        // the token mark, never inferred from span length —
                        // expansion rebinds the span to the invocation while
                        // the mark (like the kind) travels with the token.
                        conv.push_marked(TokenKind::Word(name.clone()), at, true);
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
                        if name == "document" {
                            // The engine's real `\begin{document}`: the
                            // user's literal never reaches the converter
                            // (the engine intercepts it), so this frozen
                            // `\document` re-emission carrying the source
                            // `\begin` span is the one true marker. Anything
                            // captured above ran ahead of it as hook output;
                            // the pending word belongs to the hook too while
                            // capture is still on (a swallowed end marker),
                            // so flush before releasing the capture, then
                            // re-emit the held-back run right after the
                            // marker closes, ahead of the body.
                            conv.flush_word();
                            conv.atbegin_capturing = false;
                            conv.push_environment("begin", name, at);
                            conv.drain_atbegin();
                        } else {
                            conv.push_environment("begin", name, at);
                        }
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
            self.emit(ExpandedToken {
                token: Token { kind: token.kind, span, control_symbol: token.control_symbol },
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
    let limits = limits_for(total_bytes);
    let mut engine = Engine::with_limits(entry_text, limits);
    configure_with_fonts(
        &mut engine,
        document_fonts(documents, entry),
        uses_soul(documents),
        package_reader(documents, &prepared),
    );

    let mut conv = Converter::new(documents, entry);
    let mut lookahead: VecDeque<(tex::Token, Option<tex::Span>)> = VecDeque::new();
    // Package files the engine has opened so far (`map_opened` is cheap,
    // so every pull checks the count).
    let mut opened = 0usize;
    // Engine tokens taken so far. The output token limit is the incremental
    // expander's (`IncrementalExpander`'s run loop): the token that goes past
    // it is still converted, then the run stops with the same diagnostic and
    // nothing after it is typeset.
    let mut pulled: u64 = 0;
    loop {
        let next = match lookahead.pop_front() {
            Some(t) => Some(t),
            None if pulled > limits.max_output_tokens => {
                let message = tex::output_limit_message(limits.max_output_tokens);
                engine.push_diagnostic(tex::Diagnostic::error(message, tex::Span::synthetic()));
                break;
            }
            None => {
                pulled += 1;
                engine.next_content_token_with_origin()
            }
        };
        let Some((token, origin)) = next else { break };
        if engine.opened_package_files().len() > opened {
            for file in &engine.opened_package_files()[opened..] {
                conv.map_opened(file);
            }
            opened = engine.opened_package_files().len();
        }
        match conv.convert_token(&prepared, &token, origin) {
            Flow::Next => continue,
            Flow::Include(name, at) => {
                // Read the braced path through the engine.
                let (taken, path, ok) = read_braced_argument(&mut engine);
                pulled += taken.len() as u64;
                if !ok {
                    conv.push(TokenKind::Command(name), at);
                    lookahead.extend(taken);
                    continue;
                }
                include(&mut conv, &mut engine, &prepared, &name, path.trim(), at.span);
            }
            Flow::IncludeOnly(at) => {
                let (taken, path, ok) = read_braced_argument(&mut engine);
                pulled += taken.len() as u64;
                if !ok {
                    conv.push(TokenKind::Command("includeonly".to_string()), at);
                    lookahead.extend(taken);
                    continue;
                }
                record_includeonly(&mut conv, path.trim(), at.span);
            }
            Flow::Bibliography(at) => {
                // Read the braced database list through the engine.
                let (taken, arg, ok) = read_braced_argument(&mut engine);
                pulled += taken.len() as u64;
                if !ok {
                    conv.push(TokenKind::Command("bibliography".to_string()), at);
                    lookahead.extend(taken);
                    continue;
                }
                let entry = conv.entry;
                if !bibliography(&mut conv, &mut engine, &prepared, entry, arg.trim(), at.span) {
                    // No `.bbl` for any name: hand the whole command back so
                    // the parser's missing-bibliography diagnostic fires
                    // exactly as before.
                    conv.push(TokenKind::Command("bibliography".to_string()), at);
                    lookahead.extend(taken);
                }
            }
            Flow::PrintBibliography(at) => {
                if !print_bibliography(&mut conv, &mut engine, &prepared, &mut pulled, at.span) {
                    conv.push(TokenKind::Command("printbibliography".to_string()), at);
                }
            }
        }
    }
    conv.drain_atbegin();

    if step_limit_hit(engine.diagnostics()) {
        if let Some((source, offset)) = engine.input_position() {
            if let Some(Some(document)) = conv.source_documents.get(&source).copied() {
                conv.resume_unexpanded(document, offset);
            }
        }
    }
    for file in &engine.opened_package_files()[opened..] {
        conv.map_opened(file);
    }
    let diagnostics = engine.diagnostics().to_vec();
    conv.map_diagnostics(&diagnostics);
    let package_records = conv.package_records(engine.opened_package_files().iter());
    Expansion {
        tokens: Rc::new(conv.out),
        diagnostics: conv.diagnostics,
        arraystretch: conv.arraystretch,
        current_label_by_marker: conv.current_label_by_marker,
        package_files: conv.package_files,
        package_records,
    }
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
    /// The class size and font packages change the engine's `em`/`ex`.
    fonts: DocumentFonts,
    /// Whether soul's names were reserved as host commands at configure
    /// time ([`uses_soul`]): toggling `\usepackage{soul}` rebuilds the
    /// cache, since restored checkpoints would otherwise keep the old
    /// reservation either way.
    soul: bool,
    /// The project's `.sty`/`.cls` texts the expander read: an edit to one
    /// of them is not an edit of the entry, so the cache is rebuilt instead.
    package_texts: Vec<(String, String)>,
    /// Tokens at the end of `out` typeset unexpanded after the engine
    /// stopped (see [`Converter::resume_unexpanded`]); no marks cover them.
    recovered: usize,
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
    let masked: &str = prepared[entry].text.as_ref();
    let fonts = document_fonts(documents, entry);
    let soul = uses_soul(documents);
    // The same limits as `expand_project`, which the expander applies to
    // every edit (`IncrementalExpander::edit_with_limits`).
    let limits = limits_for(documents.iter().map(|d| d.text.len()).sum());
    let reusable = cache.as_ref().is_some_and(|c| {
        !c.lent
            && c.entry_path == document.path
            && c.fonts == fonts
            && c.soul == soul
            && masked.len() <= 2 * c.created_bytes.max(INCREMENTAL_MIN_BYTES)
            && c.package_texts == crate::packages::package_texts(documents)
    });
    let expansion = if reusable {
        update_cache(cache.as_mut().expect("checked"), documents, entry, &prepared, limits)
    } else {
        let (fresh, expansion) = build_cache(documents, entry, &prepared, limits);
        *cache = Some(fresh);
        expansion
    };
    // The incremental expander equals a full run across stops, so the
    // unexpanded recovery after one is the full path's too, and so is a
    // stop on the output token limit. Debug builds check that on every
    // stopped run.
    #[cfg(debug_assertions)]
    if cache.as_ref().expect("cache kept").expander.diagnostics().iter().any(|d| is_stop_limit(&d.message)) {
        let full = expand_project(documents, entry);
        debug_assert!(
            *full.tokens == *expansion.tokens && full.diagnostics == expansion.diagnostics && full.arraystretch == expansion.arraystretch,
            "cached expansion of a stopped run differs from a full expansion"
        );
    }
    expansion
}

fn build_cache(documents: &[SourceDocument<'_>], entry: usize, prepared: &[Prepared<'_>], limits: Limits) -> (ExpansionCache, Expansion) {
    let masked: &str = prepared[entry].text.as_ref();
    let fonts = document_fonts(documents, entry);
    let reader = package_reader(documents, prepared);
    let init_fonts = fonts.clone();
    let soul = uses_soul(documents);
    let init: Rc<dyn Fn(&mut Engine)> = Rc::new(move |engine| {
        configure_with_fonts(engine, init_fonts.clone(), soul, reader.clone());
    });
    let expander = IncrementalExpander::with_host(masked, limits, CHECKPOINT_INTERVAL, init);
    let mut conv = Converter::new(documents, entry);
    for file in expander.opened_package_files() {
        conv.map_opened(file);
    }
    let mut marks = vec![Mark { index: 0, out_len: 0, last_span: conv.last_span }];
    // One allocation for the converted stream: growing it by doubling frees
    // a chain of blocks as large as the stream (hundreds of MB on a runaway
    // document), which the allocator keeps resident through the rest of the
    // compile. The stream is rarely longer than the engine's tokens plus the
    // bytes of the unexpanded rest; `finish` trims what is left over.
    let rest = expander.input_position().map_or(0, |(_, offset)| masked.len().saturating_sub(offset));
    conv.out.reserve_exact(expander.tokens().len() + rest);
    convert_range(&mut conv, prepared, &expander, 0, &mut marks, None);
    conv.drain_atbegin();
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
        fonts,
        soul,
        package_texts: crate::packages::package_texts(documents),
        recovered: 0,
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
        // documents through the full expansion above, and it covers
        // `\bibliography`/`\printbibliography` for the same reason); pass
        // the commands through untouched so the parser, not the cache,
        // reports them.
        match conv.convert_token(prepared, &tokens[k], origins[k]) {
            Flow::Include(name, at) => conv.push(TokenKind::Command(name), at),
            Flow::IncludeOnly(at) => conv.push(TokenKind::Command("includeonly".to_string()), at),
            Flow::Bibliography(at) => conv.push(TokenKind::Command("bibliography".to_string()), at),
            Flow::PrintBibliography(at) => {
                conv.push(TokenKind::Command("printbibliography".to_string()), at);
            }
            Flow::Next => {}
        }
    }
    None
}

fn shifted(token: &ExpandedToken, shift: &dyn Fn(Span) -> Span) -> ExpandedToken {
    ExpandedToken {
        token: Token {
            kind: token.token.kind.clone(),
            span: shift(token.token.span),
            control_symbol: token.token.control_symbol,
        },
        definition: token.definition.map(shift),
        maps_to_invocation: token.maps_to_invocation,
    }
}

fn update_cache(
    cache: &mut ExpansionCache,
    documents: &[SourceDocument<'_>],
    entry: usize,
    prepared: &[Prepared<'_>],
    limits: Limits,
) -> Expansion {
    let masked: &str = prepared[entry].text.as_ref();
    let mut changes = crate::incremental::changed_bytes(&cache.masked, masked);
    if changes.old.is_empty() && changes.new.is_empty() && cache.masked.len() == masked.len() {
        if cache.expander.limits() == limits {
            let mut conv = Converter::new(documents, entry);
            for file in cache.expander.opened_package_files() {
                conv.map_opened(file);
            }
            conv.out = Vec::new();
            conv.last_span = cache.last_span;
            conv.stretch_log = cache.stretch_log.clone();
            conv.arraystretch = stretch_map(&conv.stretch_log);
            conv.current_label_log = cache.current_label_log.clone();
            conv.current_label_by_marker = stretch_map(&conv.current_label_log);
            let tokens = cache.out.clone();
            let mut expansion = finish_diagnostics(cache, conv);
            expansion.tokens = tokens;
            return expansion;
        }
        // Only the limits changed (another project document's size): an
        // empty edit at the end re-runs as little as they allow.
        changes.old = masked.len()..masked.len();
        changes.new = masked.len()..masked.len();
    }
    let stats = cache.expander.edit_with_limits(
        &Edit { start: changes.old.start, end: changes.old.end, replacement: masked[changes.new.clone()].to_string() },
        limits,
    );
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
    out.truncate(out.len() - std::mem::take(&mut cache.recovered));
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
    for file in cache.expander.opened_package_files() {
        conv.map_opened(file);
    }
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
        None => conv.drain_atbegin(),
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
/// After a stop the rest of the entry is typeset unexpanded, as
/// [`expand_project`] does.
fn finish(cache: &mut ExpansionCache, conv: Converter<'_>) -> Expansion {
    let mut conv = conv;
    if step_limit_hit(cache.expander.diagnostics()) {
        if let Some((source, offset)) = cache.expander.input_position() {
            if let Some(Some(document)) = conv.source_documents.get(&source).copied() {
                let before = conv.out.len();
                conv.resume_unexpanded(document, offset);
                cache.recovered = conv.out.len() - before;
            }
        }
    }
    let mut out = std::mem::take(&mut conv.out);
    // The cache keeps the stream between revisions: at most an eighth of it
    // spare, so an edit that adds a few tokens still extends it in place.
    if out.capacity() > out.len() + out.len() / 4 {
        out.shrink_to(out.len() + out.len() / 8);
    }
    cache.stretch_log = conv.stretch_log.clone();
    cache.current_label_log = conv.current_label_log.clone();
    cache.last_span = conv.last_span;
    cache.old_engine_tokens = cache.expander.tokens().len();
    let tokens = Rc::new(out);
    cache.out = tokens.clone();
    let mut expansion = finish_diagnostics(cache, conv);
    expansion.tokens = tokens;
    expansion
}

fn stretch_map(log: &[(usize, (usize, usize), String)]) -> HashMap<(usize, usize), String> {
    log.iter().map(|(_, key, text)| (*key, text.clone())).collect()
}

fn finish_diagnostics(cache: &ExpansionCache, mut conv: Converter<'_>) -> Expansion {
    conv.last_span = cache.last_span;
    conv.map_diagnostics(cache.expander.diagnostics());
    let package_records = conv.package_records(cache.expander.opened_package_files());
    Expansion {
        tokens: Rc::new(Vec::new()),
        diagnostics: conv.diagnostics,
        arraystretch: conv.arraystretch,
        current_label_by_marker: conv.current_label_by_marker,
        package_files: conv.package_files,
        package_records,
    }
}

fn recovery_for(message: &str) -> &'static str {
    if message.contains("LaTeX Error: Command") && message.contains("already defined") {
        "kept the existing command definition"
    } else if message.contains("LaTeX Error: Command") && message.contains("undefined") {
        "defined the command anyway"
    } else if tex::is_output_limit(message) {
        "stopped expanding; the rest of the document was not typeset"
    } else if message == "group nesting limit exceeded" {
        // The engine drops the `{` without opening a group and goes on.
        "the extra group was ignored and expansion continued"
    } else if message == "conditional nesting limit exceeded" {
        // The engine drops the `\if...` token; its test is read as text.
        "the extra conditional was ignored without evaluating its test, and expansion continued"
    } else if is_stop_limit(message) {
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

/// True when any project document literally loads biblatex: a raw-token
/// scan for `\usepackage`/`\RequirePackage` naming it, mirroring how
/// [`document_fonts`] reads the preamble (a macro-generated `\usepackage`
/// is missed, like there).
fn uses_biblatex(documents: &[SourceDocument<'_>]) -> bool {
    for (index, document) in documents.iter().enumerate() {
        let tokens = tokenize_document(document.text, DocumentId(index));
        let mut i = 0;
        while i < tokens.len() {
            let TokenKind::Command(name) = &tokens[i].kind else {
                i += 1;
                continue;
            };
            if name != "usepackage" && name != "RequirePackage" {
                i += 1;
                continue;
            }
            let mut cursor = i + 1;
            if let Some((_, after)) = crate::bib::optional_bracket_text(&tokens, cursor) {
                cursor = after;
            }
            match crate::bib::group_text(&tokens, cursor) {
                Some((packages, after)) => {
                    if packages
                        .split(',')
                        .map(str::trim)
                        .any(|package| package == "biblatex")
                    {
                        return true;
                    }
                    i = after;
                }
                None => i = cursor,
            }
        }
    }
    false
}

/// True when any project document literally loads the built-in soul model:
/// a raw-token scan for `\usepackage`/`\RequirePackage` naming `soul`,
/// exactly like [`uses_biblatex`] (a macro-generated `\usepackage` is
/// missed, like there). When true the engine reserves soul's names (see
/// [`configure_with_fonts`]): real soul.sty defines `\so`/`\hl`, so a
/// later `\newcommand` on either errors there, and it must error here too
/// (GH-828 item 3). Without soul the names stay undefined so a user's own
/// `\newcommand{\hl}`/`\newcommand{\so}` wins, as in real LaTeX.
fn uses_soul(documents: &[SourceDocument<'_>]) -> bool {
    for (index, document) in documents.iter().enumerate() {
        let tokens = tokenize_document(document.text, DocumentId(index));
        let mut i = 0;
        while i < tokens.len() {
            let TokenKind::Command(name) = &tokens[i].kind else {
                i += 1;
                continue;
            };
            if name != "usepackage" && name != "RequirePackage" {
                i += 1;
                continue;
            }
            let mut cursor = i + 1;
            if let Some((_, after)) = crate::bib::optional_bracket_text(&tokens, cursor) {
                cursor = after;
            }
            match crate::bib::group_text(&tokens, cursor) {
                Some((packages, after)) => {
                    if packages
                        .split(',')
                        .map(str::trim)
                        .any(|package| package == "soul")
                    {
                        return true;
                    }
                    i = after;
                }
                None => i = cursor,
            }
        }
    }
    false
}

/// The job's own `.bbl` next to the entry document (`main.tex` →
/// `main.bbl`): biber names its output after the job, and real LaTeX's
/// `\bibliography` inputs exactly that file whatever its argument says
/// (latex.ltx `\@input@{\jobname.bbl}`). `None` for a pathless entry.
fn job_bbl_path(entry_path: &str) -> Option<String> {
    let (dir, base) = match entry_path.rfind('/') {
        Some(i) => (&entry_path[..=i], &entry_path[i + 1..]),
        None => ("", entry_path),
    };
    let stem = match base.rfind('.') {
        Some(i) => &base[..i],
        None => base,
    };
    if stem.is_empty() {
        return None;
    }
    Some(format!("{dir}{stem}.bbl"))
}

/// One `\bibliography` name against the project's `.bbl` documents:
/// `{name}.bbl` first, then the literal name — the same two-way match
/// [`include`] performs with `.tex`. Unsafe paths never resolve, and a
/// `.bib` database never resolves either: inputting one as TeX would
/// typeset its `@article` records as body text (seen on a real document
/// writing `\bibliography{main.bib}` next to `main.bib`).
fn resolve_bbl(conv: &Converter<'_>, name: &str) -> Option<usize> {
    if !path_is_safe(name) {
        return None;
    }
    if name.ends_with(".bbl") {
        return conv.document_by_path.get(name).copied();
    }
    // `\bibliography{main.bib}` names the database, not the prebuilt file:
    // look next to it, never *at* it.
    let base = name.strip_suffix(".bib").unwrap_or(name);
    let appended = format!("{base}.bbl");
    if let Some(index) = conv.document_by_path.get(appended.as_str()).copied() {
        return Some(index);
    }
    // The literal name, unless it is a `.bib` database.
    if base == name {
        if let Some(index) = conv.document_by_path.get(name).copied() {
            let is_bib = conv
                .documents
                .get(index)
                .map(|document| document.path.ends_with(".bib"))
                .unwrap_or(true);
            if !is_bib {
                return Some(index);
            }
        }
    }
    None
}

/// Consume `\bibliography{databases}` by inputting the project's pre-built
/// `.bbl` files through [`include`], so the `thebibliography` they carry
/// reaches the parser macro-expanded and `bib::prescan` resolves `\cite`
/// against its `\bibitem`s. Every comma-separated name must resolve to a
/// `.bbl` of its own; otherwise the job's own `.bbl` is tried (see
/// [`job_bbl_path`]). Returns false when neither rule finds a file, and the
/// caller hands the command back for the parser's missing-bibliography
/// diagnostic, unchanged.
fn bibliography(
    conv: &mut Converter<'_>,
    engine: &mut Engine,
    prepared: &[Prepared<'_>],
    entry: usize,
    arg: &str,
    span: Span,
) -> bool {
    let names: Vec<&str> = arg
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect();
    let mut resolved = Vec::with_capacity(names.len());
    let mut all = !names.is_empty();
    for name in names {
        match resolve_bbl(conv, name) {
            Some(index) => resolved.push(index),
            None => {
                all = false;
                break;
            }
        }
    }
    if all {
        for index in resolved {
            let path = conv.documents[index].path.to_string();
            include(conv, engine, prepared, "bibliography", &path, span);
        }
        return true;
    }
    let entry_path = conv.documents.get(entry).map(|d| d.path).unwrap_or("");
    if let Some(path) = job_bbl_path(entry_path) {
        if conv.document_by_path.contains_key(path.as_str()) {
            include(conv, engine, prepared, "bibliography", &path, span);
            return true;
        }
    }
    false
}

/// Whether `\printbibliography`'s optional `[...]` literally follows the
/// command in its own source: spaces and `%` comments, then `[`, read from
/// the command token's own span — no engine pull, so nothing executes (a
/// peek through the engine would run whatever follows, e.g. an end-of-file
/// `\end{document}`, before the `.bbl` is input). A bracket donated past an
/// `\input` boundary, or by a macro, is missed and the parser typesets it
/// as text; both are degenerate placements real documents never use.
fn bracket_follows(prepared: &[Prepared<'_>], span: Span) -> bool {
    let Some(text) = prepared.get(span.document.0).map(|p| p.text.as_ref()) else {
        return false;
    };
    let Some(mut rest) = text.get(span.end..) else {
        return false;
    };
    loop {
        rest = rest.trim_start_matches([' ', '\t', '\n', '\r']);
        if let Some(after) = rest.strip_prefix('%') {
            rest = match after.find('\n') {
                Some(i) => &after[i + 1..],
                None => return false,
            };
        } else {
            break;
        }
    }
    rest.starts_with('[')
}

/// Consume one `[...]` through the engine, dropping it: call only after
/// [`bracket_follows`] saw the `[`, so the pulls start at the bracket and
/// nothing else executes. A `]` inside a brace group does not end the scan,
/// mirroring the parser's bracket argument.
fn consume_bracket(engine: &mut Engine, pulled: &mut u64) {
    let mut depth = 0usize;
    while let Some((token, _)) = engine.next_content_token_with_origin() {
        *pulled += 1;
        match &token.kind {
            TexKind::Char(_, CatCode::BeginGroup) => depth += 1,
            TexKind::Char(_, CatCode::EndGroup) if depth > 0 => depth -= 1,
            TexKind::Char(']', _) | TexKind::ActiveChar(']') if depth == 0 => break,
            _ => {}
        }
    }
}

/// Consume `\printbibliography` by inputting the job's `.bbl` (see
/// [`job_bbl_path`]) when the project loads biblatex and carries that file:
/// real biblatex typesets exactly biber's output, and a BibTeX-style `.bbl`
/// is the `thebibliography` the parser already renders — its own
/// `References` heading replaces biblatex's, so the swallowed command's
/// optional `[title=...]` is read and dropped here. Without biblatex, or
/// without the file, returns false and the command passes through to the
/// parser's own diagnostic. A resolvable `.bib` database keeps working
/// through the existing biblatex path whenever no `.bbl` is present.
fn print_bibliography(
    conv: &mut Converter<'_>,
    engine: &mut Engine,
    prepared: &[Prepared<'_>],
    pulled: &mut u64,
    span: Span,
) -> bool {
    if !conv.biblatex {
        return false;
    }
    let entry = conv.entry;
    let entry_path = conv.documents.get(entry).map(|d| d.path).unwrap_or("");
    let Some(path) = job_bbl_path(entry_path) else {
        return false;
    };
    if !conv.document_by_path.contains_key(path.as_str()) {
        return false;
    }
    if bracket_follows(prepared, span) {
        consume_bracket(engine, pulled);
    }
    include(conv, engine, prepared, "printbibliography", &path, span);
    true
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
    // `\input{glyphtounicode}` (pdfTeX's glyph-to-Unicode table): the
    // ~2,700-line system file is pure `\pdfglyphtounicode` metadata with zero
    // visible effect (measured against pdflatex, TeX Live 2026), so it is a
    // silent no-op. Matched by exact target name -- never a general
    // kpathsea/system-file fallback. (The parser's own `include` carries the
    // same exemption for the tokens that reach it.)
    let target = requested.trim();
    if target == "glyphtounicode" || target == "glyphtounicode.tex" {
        return;
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
    use super::document_fonts;
    use crate::parser::SourceDocument;

    fn class_pt_of(preamble: &str) -> f64 {
        let doc = SourceDocument {
            path: "main.tex",
            text: preamble,
        };
        document_fonts(std::slice::from_ref(&doc), 0).setup.class_pt
    }

    /// KOMA classes default to 11pt and honour `fontsize=`; the legacy
    /// size names keep working and keep winning when present.
    #[test]
    fn koma_document_font_size() {
        assert_eq!(class_pt_of("\\documentclass{scrartcl}\n\\begin{document}"), 11.0);
        assert_eq!(class_pt_of("\\documentclass{scrreprt}\n\\begin{document}"), 11.0);
        assert_eq!(class_pt_of("\\documentclass{scrbook}\n\\begin{document}"), 11.0);
        assert_eq!(
            class_pt_of("\\documentclass[fontsize=12pt]{scrartcl}\n\\begin{document}"),
            12.0
        );
        assert_eq!(
            class_pt_of("\\documentclass[10pt]{scrartcl}\n\\begin{document}"),
            10.0
        );
        assert_eq!(class_pt_of("\\documentclass{article}\n\\begin{document}"), 10.0);
        assert_eq!(
            class_pt_of("\\documentclass[11pt]{article}\n\\begin{document}"),
            11.0
        );
    }

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
