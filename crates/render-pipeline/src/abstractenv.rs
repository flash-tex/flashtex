//! The `abstract` environment (article.cls 366-388, report.cls 441-463,
//! v1.4n / TeX Live 2025).
//!
//! ```tex
//! \newenvironment{abstract}{%
//!     \if@twocolumn
//!       \section*{\abstractname}%
//!     \else
//!       \small
//!       \begin{center}%
//!         {\bfseries \abstractname\vspace{-.5em}\vspace{\z@}}%
//!       \end{center}%
//!       \quotation
//!     \fi}
//!     {\if@twocolumn\else\endquotation\fi}
//! ```
//!
//! with `\quotation` (article.cls 356-364)
//!
//! ```tex
//! \newenvironment{quotation}
//!     {\list{}{\listparindent 1.5em
//!              \itemindent    \listparindent
//!              \rightmargin   \leftmargin
//!              \parsep        \z@ \@plus\p@}%
//!      \item\relax}
//!     {\endlist}
//! ```
//!
//! The compiler has no model for the environment (it reports it and sets
//! the body as plain text), so the pipeline reads it from the source bytes,
//! the way it already reads `tikzpicture`, a list's `\begin` keys and a
//! paragraph's `\newpage`. What the class file says, in order, checked
//! against pdfTeX's own vertical list for `fixtures/divergence-probes/
//! min-abstract` (`\showoutput`, TeX Live 2025, `article`, 10pt):
//!
//! ```text
//! \penalty -51                      \@beginparpenalty of center's \@item
//! \glue 10.0 plus 4.0 minus 5.0     \@topsep: \normalsize \topsep 8pt+2-4,
//!                                   \partopsep 2pt+1-1, \parskip 0pt+1.
//!                                   \small only \def s \@listi, it never
//!                                   executes it, so \topsep is still
//!                                   \normalsize's here.
//! \glue(\parskip) 0.0 plus 1.0      the head paragraph starts
//! \glue(\baselineskip) 4.78902      11pt: \small's own \baselineskip
//! \hbox(6.21098+0.0)x469.75502      \small\bfseries \abstractname, centred
//!                                   at the FULL measure (center is a
//!                                   \trivlist: \leftmargin is 0)
//! \glue -5.3237                     \vspace{-.5em}: half a \bfseries quad
//!                                   at \small (cmbx9), through \vadjust
//! \glue 0.0  \glue 0.0  \glue 0.0   \vspace{\z@} and \@vspace's \z@skip s
//! \penalty -51                      \@endparpenalty of \end{center}
//! \glue 10.0 plus 3.0 minus 5.0     \@endparenv's \@topsepadd (no \parskip)
//! \glue -10.0 ... / \penalty -51 / \glue 10.0 ...
//!                                   \addpenalty of quotation's \@item,
//!                                   which lifts the skip, adds the penalty
//!                                   and puts the skip back
//!                                   (\addvspace\@topsep then adds nothing:
//!                                   quotation's 6pt loses to the 10pt
//!                                   already there)
//! \glue(\parskip) 0.0 plus 1.0      \list set \parskip\parsep
//! \glue(\baselineskip) 4.25165      still 11pt
//! \hbox(...)x419.75496, shifted 25.00003        \leftmargini both sides
//! .\hbox(0.0+0.0)x13.87161                      \itemindent = 1.5em of
//!                                               \small (cmr9)
//! ...
//! \penalty -51
//! \glue 6.0 plus 3.0 minus 3.0      \endlist's \@topsepadd, now from the
//!                                   \@listi \small redefined: \topsep 4pt
//!                                   plus \partopsep 2pt, NOT 8pt+2pt
//! ```
//!
//! The `\if@twocolumn` branch (article.cls 378-379) is set too, as what it
//! is: `\section*{\abstractname}`, an unnumbered `\Large\bfseries` head at
//! the *column* measure with the body as ordinary `\normalsize` paragraphs.
//! Its reference is `fixtures/real-world/conf-paper`, where pdfLaTeX's head
//! is `SFBX1440` at the column's left edge and the body `SFRM1000`.
//!
//! The `titlepage` branch (report's and book's default, or article's
//! `titlepage` option) — a page of its own between `\null\vfil`s — is not
//! set; the compiler's own limitation stands. `book.cls` defines no
//! `abstract` environment at all, so its `\begin{abstract}` keeps the
//! compiler's "not implemented" warning.
//!
//! One thing this environment made visible that is not specific to it: the
//! opening `\addvspace{\@topsep}` of every `\list`/`\trivlist` obeys
//! `\@xaddvskip`, and right after a heading `\@nbitem` replaces it
//! altogether. `\@maketitle`'s trailing `\vskip 1.5em` is larger than
//! `\topsep + \partopsep` at every class size, so an `abstract` opened
//! right after `\maketitle` contributes no skip of its own. See
//! `tests/abstract_env.rs`.

use flashtex_compiler::Span;

use crate::adapter::{Block, CharSrc, EnvOpen, Item, ParaPart, ParaStyle, Segment, SizedPara, TextStyle, Word};
use crate::style::Stylesheet;

/// `\listparindent` of `quotation`, in `em` of the size in force
/// (article.cls 358).
const LISTPARINDENT_EM: f64 = 1.5;

/// `\vspace{-.5em}` after `\abstractname` (article.cls 383).
const HEAD_VSPACE_EM: f64 = -0.5;

/// `\abstractname` (article.cls 447, report.cls 722).
const ABSTRACTNAME: &str = "Abstract";

/// One `\begin{abstract}...\end{abstract}` in one document's source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    /// Byte range of the `\begin{abstract}` command itself.
    pub begin: (usize, usize),
    /// Byte range of the body between the two commands.
    pub body: (usize, usize),
    /// Byte range of the `\end{abstract}` command itself.
    pub end: (usize, usize),
}

/// Every `abstract` environment of `text`, in order. Nested or unclosed
/// environments are skipped: the environment takes no argument, so a
/// `\begin` whose `\end` is missing has no body to set.
pub fn ranges(text: &str) -> Vec<Range> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(at) = crate::adapter::find_command(&text[from..], "begin") {
        let begin = from + at;
        let after = begin + "\\begin".len();
        from = begin + 1;
        let Some(open) = argument_at(text, after, "abstract") else { continue };
        let mut scan = open;
        let mut close = None;
        while let Some(rel) = crate::adapter::find_command(&text[scan..], "end") {
            let end = scan + rel;
            scan = end + 1;
            if let Some(after_end) = argument_at(text, end + "\\end".len(), "abstract") {
                close = Some((end, after_end));
                break;
            }
        }
        let Some((end_start, end_end)) = close else { continue };
        out.push(Range {
            begin: (begin, open),
            body: (open, end_start),
            end: (end_start, end_end),
        });
        from = end_end;
    }
    out
}

/// The end byte of `{name}` when that is exactly what follows byte `at`
/// (spaces allowed before the brace, as `\begin`'s argument scan does).
fn argument_at(text: &str, at: usize, name: &str) -> Option<usize> {
    let rest = text.get(at..)?;
    let trimmed = rest.trim_start();
    let skipped = rest.len() - trimmed.len();
    let inner = trimmed.strip_prefix('{')?;
    let close = inner.find('}')?;
    (inner[..close].trim() == name).then_some(at + skipped + 1 + close + 1)
}

/// Which of the environment's three forms this document takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Branch {
    /// article.cls 380-386: `\small`, a centred head, a `quotation`.
    OneColumn,
    /// article.cls 378-379: `\section*{\abstractname}` — an unnumbered
    /// `\Large\bfseries` head at `\normalsize` and the *column* measure,
    /// and the body as ordinary paragraphs. A different shape, not a
    /// narrower one: no `\small`, no `quotation`, no centring.
    TwoColumn,
    /// article.cls 367-375 (report's and book's default, or article's
    /// `titlepage` option): a page of its own between `\null\vfil`s,
    /// inside `\titlepage`. Not set here; the compiler's own limitation
    /// stands.
    TitlePage,
}

fn branch(style: &Stylesheet) -> Branch {
    let Some(g) = style.class_geometry.as_ref() else { return Branch::OneColumn };
    if g.options.titlepage {
        // `\if@titlepage` is tested first (article.cls 366).
        return Branch::TitlePage;
    }
    if g.options.twocolumn {
        return Branch::TwoColumn;
    }
    Branch::OneColumn
}

/// Rewrites the plain paragraphs the compiler produced for each `abstract`
/// body into the class's own shape and inserts its head before them.
/// Returns the source spans whose compiler diagnostic the pipeline now
/// supersedes — one per environment actually set, so a `titlepage`
/// document's warning (and `book`'s) survives untouched.
pub fn apply(texts: &[&str], blocks: &mut Vec<Block>, style: &Stylesheet) -> Vec<Span> {
    let mut superseded = Vec::new();
    // `book.cls` has no `abstract`; the compiler's warning is the truth.
    if style.class_geometry.as_ref().is_some_and(|g| g.options.kind == flashtex_class_geometry::ClassKind::Book) {
        return superseded;
    }
    let small = style.small();
    // Innermost-last so the inserted head never moves a range not yet done.
    let mut found: Vec<(usize, Range)> = Vec::new();
    for (d, text) in texts.iter().enumerate() {
        found.extend(ranges(text).into_iter().map(|r| (d, r)));
    }
    let form = branch(style);
    for (document, range) in found.into_iter().rev() {
        let span = Span::in_document(flashtex_compiler::DocumentId(document), range.begin.0, range.begin.1);
        if form == Branch::TitlePage {
            continue;
        }
        let inside = |b: &Block| -> bool {
            block_span(b).is_some_and(|s| s.document.0 == document && s.start >= range.body.0 && s.start < range.body.1)
        };
        let Some(first) = blocks.iter().position(inside) else {
            // An empty `abstract`: nothing was set, so nothing to fix.
            continue;
        };
        let last = blocks.iter().rposition(inside).unwrap_or(first);
        if form == Branch::TwoColumn {
            // `\section*`'s before-skip is negative, so `\@startsection`
            // leaves `\@afterindentfalse`: the first paragraph of the body
            // is not indented. Everything else the class sets for this
            // branch is what the compiler already produced — `\normalsize`
            // paragraphs at the column measure.
            if let Some(Block::Paragraph { indent, .. }) = blocks.get_mut(first) {
                *indent = false;
            }
            blocks.insert(first, section_head_block(texts, document, range));
            superseded.push(span);
            continue;
        }
        for block in &mut blocks[first..=last] {
            let Block::Paragraph { indent, style: para_style, env_open, list, sized, .. } = block else { continue };
            // `\list` sets `\parindent\listparindent`, and `\@item`'s
            // `\everypar` drops the `\parindent` box of the first paragraph
            // only to re-add the same width as `\itemindent`: every
            // paragraph of a `quotation` is indented by `\listparindent`.
            *indent = true;
            *para_style = ParaStyle::Quote;
            // The `\list`'s own `\addvspace\@topsep` is absorbed whole by
            // the `\end{center}` skip already on the vertical list: at
            // every class size `\normalsize`'s `\topsep + \partopsep`
            // (10/12/13pt) is larger than `\small`'s plus `\parskip`
            // (6/9/12pt), and `\@xaddvskip` adds nothing to a larger
            // `\lastskip`.
            *env_open = None;
            *list = None;
            *sized = Some(SizedPara {
                size_pt: small.size_pt,
                baselineskip_pt: small.baselineskip_pt,
                parindent_em: Some(LISTPARINDENT_EM),
                vspace_after_em: 0.0,
                close_skip: Some(small.topsepadd()),
            });
        }
        blocks.insert(first, head_block(texts, document, range, &small));
        superseded.push(span);
    }
    superseded
}

/// `\begin{center}{\bfseries \abstractname\vspace{-.5em}\vspace{\z@}}
/// \end{center}` at `\small`: one centred bold word at the full measure,
/// its characters pointing at the `\begin{abstract}` command.
fn head_block(texts: &[&str], document: usize, range: Range, small: &crate::style::SmallSize) -> Block {
    let name = abstract_name(texts).unwrap_or_else(|| ABSTRACTNAME.to_string());
    let src = CharSrc {
        document: flashtex_compiler::DocumentId(document),
        start: range.begin.0,
        end: range.begin.1,
    };
    let word = Word {
        segments: vec![Segment {
            chars: name.chars().map(|_| src).collect(),
            text: name,
            style: TextStyle { bold: true, ..TextStyle::default() },
        }],
    };
    Block::Paragraph {
        parts: vec![ParaPart::Lines(vec![Item::Word(word)])],
        indent: false,
        style: ParaStyle::Center,
        // `\begin{abstract}` is always read in vertical mode: the compiler
        // only emits a body block for it after `\par`, and `\@trivlist`
        // takes `\partopsep` whenever it is.
        env_open: Some(EnvOpen { vmode: true, skips: None }),
        env_close: true,
        eject_before: false,
        vspace_before: 0.0,
        addvspace_before: 0.0,
        addvspace_flex: (0.0, 0.0),
        vspace_flex: (0.0, 0.0),
        endlist_adjust: 0.0,
        list: None,
        sized: Some(SizedPara {
            size_pt: small.size_pt,
            baselineskip_pt: small.baselineskip_pt,
            parindent_em: None,
            vspace_after_em: HEAD_VSPACE_EM,
            // `\end{center}` is a `\trivlist`: `\@topsepadd` is whatever
            // `\topsep`/`\partopsep` hold, which `\small` has not touched.
            close_skip: None,
        }),
        // The head's leading travels on its `SizedPara`, which resizes the
        // whole paragraph; nothing here is a compiler-observed `\par`.
        leading_pt: None,
    }
}

/// `\section*{\abstractname}` (article.cls 379), the `\if@twocolumn`
/// branch: an unnumbered level-1 heading, so `\Large\bfseries` at the
/// column measure with `\@startsection`'s own skips. Starred, so `number`
/// is empty and no `\sectionmark` is issued.
fn section_head_block(texts: &[&str], document: usize, range: Range) -> Block {
    let name = abstract_name(texts).unwrap_or_else(|| ABSTRACTNAME.to_string());
    let src = CharSrc {
        document: flashtex_compiler::DocumentId(document),
        start: range.begin.0,
        end: range.begin.1,
    };
    let word = Word {
        segments: vec![Segment {
            chars: name.chars().map(|_| src).collect(),
            text: name.clone(),
            style: TextStyle::default(),
        }],
    };
    Block::Heading {
        level: 1,
        items: vec![Item::Word(word)],
        eject_before: false,
        vspace_before: 0.0,
        number: String::new(),
        title: name,
        span: Span::in_document(flashtex_compiler::DocumentId(document), range.begin.0, range.begin.1),
    }
}

/// `\renewcommand{\abstractname}{...}` (or the unbraced form) in any of
/// the documents' source; the last one wins, as the last definition does.
fn abstract_name(texts: &[&str]) -> Option<String> {
    let mut name = None;
    for text in texts {
        let mut from = 0;
        while let Some(at) = crate::adapter::find_command(&text[from..], "renewcommand") {
            let abs = from + at;
            from = abs + 1;
            let rest = text[abs + "\\renewcommand".len()..].trim_start();
            let rest = match rest.strip_prefix('{') {
                Some(r) => match r.trim_start().strip_prefix("\\abstractname").map(|r| r.trim_start().strip_prefix('}')) {
                    Some(Some(r)) => r,
                    _ => continue,
                },
                None => match rest.strip_prefix("\\abstractname") {
                    Some(r) => r,
                    None => continue,
                },
            };
            let Some(inner) = rest.trim_start().strip_prefix('{') else { continue };
            let Some(close) = inner.find('}') else { continue };
            name = Some(inner[..close].to_string());
        }
    }
    name
}

/// The first source position a block sets material at.
pub(crate) fn block_span(block: &Block) -> Option<Span> {
    match block {
        Block::Paragraph { parts, .. } => parts.iter().find_map(part_span),
        Block::Heading { span, .. } | Block::Chapter { span, .. } | Block::Part { span, .. } | Block::Title { span, .. } | Block::Rule { span, .. } => Some(*span),
        _ => None,
    }
}

fn part_span(part: &ParaPart) -> Option<Span> {
    match part {
        ParaPart::Lines(items) => items.iter().find_map(|i| match i {
            Item::Word(w) => Some(w.span()),
            Item::Math { span, .. } | Item::Logo { span, .. } | Item::Rule { span, .. } | Item::Footnote { span, .. } | Item::Marginpar { span, .. } => Some(*span),
            _ => None,
        }),
        ParaPart::Display { span, .. } | ParaPart::Rows { span, .. } => Some(*span),
    }
}
