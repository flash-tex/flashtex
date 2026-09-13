//! Contents lists: `\tableofcontents`, `\listoffigures`, `\listoftables`.
//!
//! LaTeX writes one `\contentsline{<level>}{<entry>}{<page>}{}` per
//! numbered heading, `\addcontentsline` and float caption to `.toc`/`.lof`/
//! `.lot` and reads the file back on the next run (latex.ltx `\@starttoc`,
//! `\addcontentsline`, `\contentsline`). The pipeline keeps the same
//! records in memory: the adapter collects them from the parse in one pass
//! and sets every list where its command stands, and the page of each entry
//! comes from the previous layout pass through a synthetic `\label`
//! ([`key`]) — `render` reruns the layout until those pages settle, as it
//! does for `\pageref`.
//!
//! The entry geometry (`\l@section`, `\l@chapter`, `\@dottedtocline`) is
//! [`EntryStyle`]; the lines themselves are set by `typeset::toc`.

use std::collections::BTreeMap;

use crate::adapter::{self, Block, ChromeEvent, Item, Labels, ParaPart};
use crate::floats::{FloatEnv, FloatKind, Piece};
use crate::RenderOptions;
use flashtex_compiler::parser::SourceDocument;
use flashtex_compiler::{DocumentId, Span};

/// Which list a `\contentsline` belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListKind {
    Toc,
    Lof,
    Lot,
}

impl ListKind {
    /// The `\addcontentsline` file extension.
    pub fn from_ext(ext: &str) -> Option<ListKind> {
        match ext {
            "toc" => Some(ListKind::Toc),
            "lof" => Some(ListKind::Lof),
            "lot" => Some(ListKind::Lot),
            _ => None,
        }
    }

    /// The list's heading macro and its article/report/book.cls value.
    pub fn name(self) -> (&'static str, &'static str) {
        match self {
            ListKind::Toc => ("contentsname", "Contents"),
            ListKind::Lof => ("listfigurename", "List of Figures"),
            ListKind::Lot => ("listtablename", "List of Tables"),
        }
    }
}

/// `\l@<level>`'s depth: `\c@tocdepth` hides entries deeper than it.
pub fn level_of(name: &str) -> Option<i8> {
    Some(match name {
        // `\l@part`: `\ifnum \c@tocdepth >-2`.
        "part" => -1,
        "chapter" => 0,
        "section" | "figure" | "table" => 1,
        "subsection" => 2,
        "subsubsection" => 3,
        "paragraph" => 4,
        "subparagraph" => 5,
        _ => return None,
    })
}

/// Prefix of the synthetic label keys that carry entry pages; `\label`
/// keys cannot contain U+0001.
const KEY_PREFIX: &str = "\u{1}toc:";

pub fn is_key(key: &str) -> bool {
    key.starts_with(KEY_PREFIX)
}

/// The label key of the `n`th heading/`\addcontentsline` record.
pub fn key(n: usize) -> String {
    format!("{KEY_PREFIX}{n}")
}

/// The label key of float `index` of document `document` (in `floats::scan`
/// order).
pub fn float_key(document: usize, index: usize) -> String {
    format!("{KEY_PREFIX}float:{document}:{index}")
}

/// One `\contentsline` record.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub list: ListKind,
    pub level: i8,
    /// `\numberline{<number>}` and the bytes that produced it.
    pub number: Option<(String, Span)>,
    pub title: Vec<Item>,
    pub key: String,
}

/// A captioned float: a `\listoffigures`/`\listoftables` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloatEntry {
    pub list: ListKind,
    /// `\begin{figure}` offset, in the float's document.
    pub at: usize,
    /// The caption argument's inner bytes.
    pub caption: Span,
    /// Their text in the unmasked document (the parse sees the float
    /// blanked).
    pub text: String,
    pub key: String,
}

/// Every captioned float in source order; `texts` are the documents before
/// `floats::mask`.
pub fn float_entries(envs: &[Vec<FloatEnv>], texts: &[&str]) -> Vec<FloatEntry> {
    let mut out = Vec::new();
    for (d, doc) in envs.iter().enumerate() {
        let source = texts.get(d).copied().unwrap_or("");
        for (i, f) in doc.iter().enumerate() {
            let Some(caption) = f.pieces.iter().find_map(|p| match p {
                // `\@caption#1[#2]#3`: the list shows `#2` (`\@dblarg`
                // makes it `#3` when absent).
                Piece::Caption { arg, short, .. } => Some(short.unwrap_or(*arg)),
                _ => None,
            }) else {
                continue;
            };
            out.push(FloatEntry {
                list: match f.kind {
                    FloatKind::Figure => ListKind::Lof,
                    FloatKind::Table => ListKind::Lot,
                    // `algorithm` floats (`crate::algorithms`) are not
                    // `floats::scan` results; `\listofalgorithms` is not set.
                    FloatKind::Algorithm => continue,
                },
                at: f.span.start,
                caption,
                text: source.get(caption.start..caption.end).unwrap_or("").to_string(),
                key: float_key(d, i),
            });
        }
    }
    out
}

/// How one `\l@<level>` sets its entry. Lengths in `em` are the body
/// font's; skips are `(em, stretch pt)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntryStyle {
    /// `\bfseries` title, number and page (`\l@section`, `\l@chapter`).
    pub bold: bool,
    /// `\@dottedtocline`'s `#2` (the entry's indent; 0 for the bold forms).
    pub indent_em: f64,
    /// `\@tempdima`: the `\numberline` box width.
    pub numwidth_em: f64,
    /// `\leaders` dots, `\rightskip\@tocrmarg`; otherwise `\hfil` and
    /// `\rightskip\@pnumwidth`.
    pub dotted: bool,
    /// `\addpenalty` before the entry (skipped right after a heading).
    pub penalty_before: Option<i32>,
    pub skip_before: (f64, f64),
    /// `\addvspace` (only the excess over the previous skip) rather than
    /// `\vskip`.
    pub addvspace: bool,
    pub penalty_after: Option<i32>,
    /// `\l@part`: `{\leavevmode \large \bfseries #1\hfil \hb@xt@\@pnumwidth
    /// {\hss #2}}\par` — the title and page at `\large`, the number set
    /// inline (`\thepart\hspace{1em}`) rather than in a `\numberline` box.
    pub part: bool,
    /// report/book `\l@part`'s `\global\@nobreaktrue`: the next entry's
    /// `\addpenalty` does nothing.
    pub nobreak_after: bool,
}

/// `\@secpenalty` (article.cls) and `\@highpenalty` (latex.ltx).
const SECPENALTY: i32 = -300;
const HIGHPENALTY: i32 = 301;

/// The `\l@<level>` of article.cls (`chapters == false`) or report.cls /
/// book.cls (identical here).
pub fn entry_style(level: i8, list: ListKind, chapters: bool) -> Option<EntryStyle> {
    // `\@dottedtocline{#1}{#2}{#3}`: `\vskip 0pt plus .2pt`.
    let dotted = |indent_em: f64, numwidth_em: f64| EntryStyle {
        bold: false,
        indent_em,
        numwidth_em,
        dotted: true,
        penalty_before: None,
        skip_before: (0.0, 0.2),
        addvspace: false,
        penalty_after: None,
        part: false,
        nobreak_after: false,
    };
    if list != ListKind::Toc {
        // `\l@figure{\@dottedtocline{1}{1.5em}{2.3em}}`, `\let\l@table\l@figure`.
        return Some(dotted(1.5, 2.3));
    }
    Some(match (chapters, level) {
        // `\l@part` (article.cls lines 509-527, report.cls 595-611,
        // book.cls 601-617): `\addpenalty\@secpenalty` (article) or
        // `{-\@highpenalty}`, `\addvspace{2.25em \@plus\p@}`, `\rightskip
        // \@pnumwidth`, `\parfillskip -\@pnumwidth`, the line, `\nobreak`.
        (_, -1) => EntryStyle {
            bold: true,
            indent_em: 0.0,
            numwidth_em: 0.0,
            dotted: false,
            penalty_before: Some(if chapters { -HIGHPENALTY } else { SECPENALTY }),
            skip_before: (2.25, 1.0),
            addvspace: true,
            penalty_after: Some(10_000),
            part: true,
            nobreak_after: chapters,
        },
        // report.cls/book.cls `\l@chapter`: `\addpenalty{-\@highpenalty}`,
        // `\vskip 1.0em \@plus\p@`, ..., `\penalty\@highpenalty`.
        (true, 0) => EntryStyle {
            bold: true,
            indent_em: 0.0,
            numwidth_em: 1.5,
            dotted: false,
            penalty_before: Some(-HIGHPENALTY),
            skip_before: (1.0, 1.0),
            addvspace: false,
            penalty_after: Some(HIGHPENALTY),
            part: false,
            nobreak_after: false,
        },
        (true, 1) => dotted(1.5, 2.3),
        (true, 2) => dotted(3.8, 3.2),
        (true, 3) => dotted(7.0, 4.1),
        (true, 4) => dotted(10.0, 5.0),
        (true, 5) => dotted(12.0, 6.0),
        // article.cls `\l@section`: `\addpenalty\@secpenalty`,
        // `\addvspace{1.0em \@plus\p@}`.
        (false, 1) => EntryStyle {
            bold: true,
            indent_em: 0.0,
            numwidth_em: 1.5,
            dotted: false,
            penalty_before: Some(SECPENALTY),
            skip_before: (1.0, 1.0),
            addvspace: true,
            penalty_after: None,
            part: false,
            nobreak_after: false,
        },
        (false, 2) => dotted(1.5, 2.3),
        (false, 3) => dotted(3.8, 3.2),
        (false, 4) => dotted(7.0, 4.1),
        (false, 5) => dotted(10.0, 5.0),
        _ => return None,
    })
}

/// One entry line of a list, ready to set.
#[derive(Debug, Clone, PartialEq)]
pub struct TocEntry {
    pub style: EntryStyle,
    pub number: Option<(String, Span)>,
    pub title: Vec<Item>,
    /// `\thepage` of the entry in the previous pass (empty on the first).
    pub page: String,
    /// The list command: the bytes of the leader dots and page numbers.
    pub list_span: Span,
    /// Set across the full `\textwidth` of a two-column document: report/
    /// book `\tableofcontents` (etc.) run `\onecolumn` around the list.
    pub wide: bool,
}

/// Document settings the lists read from the source.
#[derive(Debug, Clone)]
pub struct Settings {
    pub chapters: bool,
    pub tocdepth: i8,
    pub names: [String; 3],
    /// `\pagestyle{headings}`: `\@mkboth` sets both marks.
    pub marks: bool,
    /// report.cls/book.cls lists in a `twocolumn` document: `\if@twocolumn
    /// \@restonecoltrue\onecolumn ... \if@restonecol\twocolumn\fi` (report.cls
    /// lines 583-594, 634-658; book.cls 589-600, 640-664).
    pub onecolumn_lists: bool,
}

/// Whether `\documentclass[<options>]` lists `twocolumn`.
pub fn twocolumn_class(source: &str) -> bool {
    let Some(at) = source.find("\\documentclass") else { return false };
    let rest = source[at + "\\documentclass".len()..].trim_start();
    rest.strip_prefix('[').and_then(|r| r.split(']').next()).is_some_and(|opts| opts.split(',').any(|o| o.trim() == "twocolumn"))
}

impl Settings {
    pub fn read(source: &str, chapters: bool) -> Settings {
        let name = |kind: ListKind| {
            let (macro_name, default) = kind.name();
            renewed_name(source, macro_name).unwrap_or_else(|| default.to_string())
        };
        Settings {
            onecolumn_lists: chapters && twocolumn_class(source),
            chapters,
            // article.cls `\setcounter{tocdepth}{3}`, report/book `{2}`.
            tocdepth: signed_counter(source, "tocdepth").unwrap_or(if chapters { 2 } else { 3 }),
            names: [name(ListKind::Toc), name(ListKind::Lof), name(ListKind::Lot)],
            marks: source.contains("\\pagestyle{headings}"),
        }
    }

    fn name_of(&self, kind: ListKind) -> &str {
        &self.names[kind as usize]
    }
}

/// The last `\setcounter{<name>}{<n>}` in the source (negative allowed).
fn signed_counter(source: &str, name: &str) -> Option<i8> {
    let pattern = format!("\\setcounter{{{name}}}");
    let mut value = None;
    let mut from = 0;
    while let Some(at) = source[from..].find(&pattern) {
        let rest = source[from + at + pattern.len()..].trim_start();
        if let Some(r) = rest.strip_prefix('{') {
            if let Some(end) = r.find('}') {
                value = r[..end].trim().parse::<i8>().ok().or(value);
            }
        }
        from += at + pattern.len();
    }
    value
}

/// `\renewcommand{\<name>}{<text>}` / `\renewcommand\<name>{<text>}` (also
/// `*` and `\def\<name>{<text>}`): the last one, whitespace collapsed.
fn renewed_name(source: &str, name: &str) -> Option<String> {
    let bytes = source.as_bytes();
    let mut found = None;
    for opener in ["\\renewcommand", "\\def"] {
        let mut from = 0;
        while let Some(at) = source[from..].find(opener) {
            let mut k = from + at + opener.len();
            from = k;
            if bytes.get(k) == Some(&b'*') {
                k += 1;
            }
            let rest = &source[k..];
            let trimmed = rest.trim_start();
            k += rest.len() - trimmed.len();
            let target = format!("\\{name}");
            let braced = format!("{{{target}}}");
            let after = if trimmed.starts_with(&braced) {
                k + braced.len()
            } else if trimmed.starts_with(&target) && !trimmed[target.len()..].starts_with(|c: char| c.is_ascii_alphabetic()) {
                k + target.len()
            } else {
                continue;
            };
            let rest = &source[after..];
            let open = after + rest.len() - rest.trim_start().len();
            if bytes.get(open) != Some(&b'{') {
                continue;
            }
            if let Some(close) = matching_brace(bytes, open) {
                found = Some((open, source[open + 1..close].split_whitespace().collect::<Vec<_>>().join(" ")));
            }
        }
    }
    found.map(|(_, text)| text)
}

/// Byte offsets of the commands this module handles that the compiler
/// reports as errors (`\listoffigures`, `\listoftables`, `\addcontentsline`,
/// `\appendix`, the list-name redefinitions and `\setcounter{tocdepth}`):
/// those diagnostics are superseded when the document has contents lists or
/// an appendix.
pub fn superseded_commands(source: &str) -> Vec<usize> {
    let mut out: Vec<usize> = adapter::body_commands(source, false, false)
        .iter()
        .filter(|c| matches!(c.kind, adapter::BodyKind::ContentsList(_) | adapter::BodyKind::AddContentsLine { .. } | adapter::BodyKind::Appendix | adapter::BodyKind::Part { .. }))
        .map(|c| c.start)
        .collect();
    for name in ["contentsname", "listfigurename", "listtablename"] {
        for opener in ["\\renewcommand{\\", "\\renewcommand\\", "\\renewcommand*{\\", "\\renewcommand*\\"] {
            let pattern = format!("{opener}{name}");
            out.extend(source.match_indices(&pattern).map(|(i, _)| i));
        }
    }
    out.extend(source.match_indices("\\setcounter{tocdepth}").map(|(i, _)| i));
    out.sort_unstable();
    out
}

/// The `}` closing the group opened at `open`, skipping `\{`/`\}`.
fn matching_brace(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Whether the entry document sets a contents list.
pub fn has_lists(source: &str) -> bool {
    adapter::body_commands(source, false, false).iter().any(|c| matches!(c.kind, adapter::BodyKind::ContentsList(_)))
}

/// `\addcontentsline`'s entry text `source[start..end]`: an optional
/// leading `[\protect]\numberline{<number>}` (the number and the bytes
/// through its `}`), then the title's range.
fn contentsline_parts(source: &str, start: usize, end: usize) -> (Option<(String, usize, usize)>, (usize, usize)) {
    let text = &source[start..end];
    let mut k = start + text.len() - text.trim_start().len();
    if source[k..end].starts_with("\\protect") {
        k += "\\protect".len();
        let rest = &source[k..end];
        k += rest.len() - rest.trim_start().len();
    }
    if source[k..end].starts_with("\\numberline") {
        let n = k + "\\numberline".len();
        let rest = &source[n..end];
        let open = n + rest.len() - rest.trim_start().len();
        if source.as_bytes().get(open) == Some(&b'{') {
            if let Some(close) = matching_brace(source.as_bytes(), open).filter(|c| *c < end) {
                let number = source[open + 1..close].split_whitespace().collect::<Vec<_>>().join(" ");
                return (Some((number, k, close + 1)), (close + 1, end));
            }
        }
    }
    (None, (start, end))
}

/// `\addcontentsline`'s entry: its `\numberline` number and the title,
/// set like body text when [`entry_items`] has it.
pub fn contentsline_text(source: &str, document: DocumentId, start: usize, end: usize, entry: &EntryItems) -> (Option<(String, Span)>, Vec<Item>) {
    let (number, (s, e)) = contentsline_parts(source, start, end);
    let title = entry.get(document, s, e).unwrap_or_else(|| adapter::words_from_source(source, document, s, e));
    (number.map(|(n, a, b)| (n, Span::in_document(document, a, b))), title)
}

/// Entry titles set as body text: `(document, start, end)` of the source
/// range → the items the adapter makes of it. The contents files hold the
/// arguments unexpanded and `\l@<level>` typesets them, so macros, math and
/// font commands come out as they would in a paragraph.
#[derive(Debug, Clone, Default)]
pub struct EntryItems(pub BTreeMap<(usize, usize, usize), Vec<Item>>);

/// Derived from the sources once per render: equal keys mean equal items.
impl PartialEq for EntryItems {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len() && self.0.keys().eq(other.0.keys())
    }
}

impl Eq for EntryItems {}

impl EntryItems {
    pub fn get(&self, document: DocumentId, start: usize, end: usize) -> Option<Vec<Item>> {
        self.0.get(&(document.0, start, end)).filter(|items| !items.is_empty()).cloned()
    }
}

/// The source ranges of the entry titles the lists need from source bytes:
/// `\addcontentsline` titles, `\chapter`/`\part` titles (the optional
/// argument when given) in the entry document, and float captions.
pub fn entry_spans(source: &str, entry: DocumentId, floats: &[FloatEntry]) -> Vec<Span> {
    let mut out = Vec::new();
    for c in adapter::body_commands(source, true, false) {
        let range = match c.kind {
            adapter::BodyKind::AddContentsLine { text, .. } => contentsline_parts(source, text.0, text.1).1,
            adapter::BodyKind::Chapter { title, .. } => title,
            adapter::BodyKind::Part { short, title, .. } => short.unwrap_or(title),
            _ => continue,
        };
        out.push(Span::in_document(entry, range.0, range.1));
    }
    out.extend(floats.iter().map(|f| f.caption));
    out
}

/// Parses every range of `spans` as body text (the rest of its document
/// blanked, the preamble kept for its macro definitions, a paragraph break
/// after each range) and keeps each range's paragraph items.
pub fn entry_items(documents: &[SourceDocument<'_>], entry_index: usize, texts: &[&str], options: &RenderOptions, labels: &Labels, spans: &[Span]) -> EntryItems {
    let mut out = EntryItems::default();
    let mut by_doc: BTreeMap<usize, Vec<Span>> = BTreeMap::new();
    for s in spans {
        if s.start < s.end {
            by_doc.entry(s.document.0).or_default().push(*s);
        }
    }
    for (d, doc_spans) in by_doc {
        let Some(document) = documents.get(d) else { continue };
        let text = document.text;
        let body = text.find("\\begin{document}").map_or(0, |p| p + "\\begin{document}".len());
        let mut bytes: Vec<u8> = Vec::with_capacity(text.len());
        for (i, c) in text.char_indices() {
            if i < body || doc_spans.iter().any(|s| (s.start..s.end).contains(&i)) {
                bytes.extend_from_slice(&text.as_bytes()[i..i + c.len_utf8()]);
            } else {
                bytes.extend(std::iter::repeat_n(if c == '\n' { b'\n' } else { b' ' }, c.len_utf8()));
            }
        }
        for s in &doc_spans {
            // A paragraph break after the range when two blanked bytes follow.
            if s.end + 2 <= bytes.len() && !doc_spans.iter().any(|o| (o.start..o.end).contains(&s.end) || (o.start..o.end).contains(&(s.end + 1))) {
                bytes[s.end] = b'\n';
                bytes[s.end + 1] = b'\n';
            }
        }
        // Blanking keeps multi-byte characters whole only inside kept ranges;
        // replace any broken sequence rather than failing.
        let isolated = String::from_utf8_lossy(&bytes).into_owned();
        if isolated.len() != text.len() {
            continue;
        }
        let mut texts2: Vec<&str> = texts.to_vec();
        if d >= texts2.len() {
            continue;
        }
        texts2[d] = &isolated;
        let docs2: Vec<SourceDocument<'_>> = documents.iter().zip(&texts2).map(|(doc, t)| SourceDocument { path: doc.path, text: t }).collect();
        let entry_path = documents.get(entry_index).map_or("", |doc| doc.path);
        let parsed = flashtex_compiler::parser::parse_project(&docs2, entry_path);
        let doc = adapter::adapt(&texts2, entry_index, &parsed, options, labels);
        for block in &doc.blocks {
            let adapter::Block::Paragraph { parts, .. } = block else { continue };
            let items: Vec<Item> = parts
                .iter()
                .flat_map(|p| match p {
                    ParaPart::Lines(items) => items.clone(),
                    _ => Vec::new(),
                })
                .collect();
            let position = items.iter().find_map(|i| match i {
                Item::Word(w) => Some(w.span()),
                Item::Math { span, .. } => Some(*span),
                _ => None,
            });
            let Some(at) = position.filter(|p| p.document.0 == d) else { continue };
            let Some(s) = doc_spans.iter().find(|s| (s.start..s.end).contains(&at.start)) else { continue };
            let slot = out.0.entry((d, s.start, s.end)).or_default();
            if !slot.is_empty() {
                slot.push(Item::Space { style: adapter::TextStyle::default(), factor: 1000, no_break: false });
            }
            slot.extend(items.into_iter().filter(|i| !matches!(i, Item::Label { .. })));
        }
    }
    out
}

/// The blocks of one list: its heading (`\section*` in article,
/// `\chapter*` in report/book, `\@mkboth` under `headings`) and every entry
/// within `tocdepth`, figures and tables numbered `\thefigure` /
/// `\thetable` (`<chapter>.<n>` with chapters).
#[allow(clippy::too_many_arguments)]
pub fn list_blocks(
    kind: ListKind,
    span: Span,
    eject_before: bool,
    settings: &Settings,
    records: &[Record],
    labels: &Labels,
    chapter_starts: &[(usize, String)],
) -> Vec<Block> {
    let name = settings.name_of(kind).to_string();
    let mut out = Vec::new();
    let items = adapter::command_words(&name, span);
    if settings.chapters {
        // `\chapter*` issues no `\chaptermark`; `\@mkboth` below sets both.
        out.push(Block::Chapter {
            number: None,
            appendix: false,
            items,
            title: name.clone(),
            span,
            mark: false,
        });
    } else {
        out.push(Block::Heading {
            level: 1,
            items,
            eject_before,
            vspace_before: 0.0,
            number: String::new(),
            title: name.clone(),
            span,
        });
    }
    if settings.marks {
        let upper = name.to_uppercase();
        out.push(Block::Chrome {
            event: ChromeEvent::MarkBoth(upper.clone(), upper),
            span,
        });
    }
    let page = |key: &str| labels.toc_pages.get(key).cloned().unwrap_or_default();
    let mut push = |level: i8, number: Option<(String, Span)>, title: Vec<Item>, key: &str| {
        if level > settings.tocdepth {
            return;
        }
        let Some(style) = entry_style(level, kind, settings.chapters) else { return };
        out.push(Block::TocEntry(Box::new(TocEntry {
            style,
            number,
            title,
            page: page(key),
            list_span: span,
            wide: settings.onecolumn_lists,
        })));
    };
    // Float captions and `\addcontentsline{lof}` records are merged in
    // source order: captions by their float's position, records by the
    // position of their first title byte.
    let record_at = |r: &Record| -> usize {
        r.number.as_ref().map(|(_, s)| s.start).or_else(|| r.title.iter().find_map(|i| match i {
            Item::Word(w) => Some(w.span().start),
            _ => None,
        })).unwrap_or(0)
    };
    let mut floats: Vec<(usize, Option<&FloatEntry>, Option<&Record>)> = Vec::new();
    for r in records.iter().filter(|r| r.list == kind) {
        floats.push((record_at(r), None, Some(r)));
    }
    if kind != ListKind::Toc {
        for f in labels.floats.iter().filter(|f| f.list == kind) {
            floats.push((f.at, Some(f), None));
        }
        floats.sort_by_key(|(at, ..)| *at);
    }
    let mut per_chapter: Vec<(usize, u32)> = Vec::new();
    for (at, float, record) in floats {
        match (float, record) {
            (_, Some(r)) => push(r.level, r.number.clone(), r.title.clone(), &r.key),
            (Some(f), None) => {
                let doc = f.caption.document;
                // `\thefigure`: `\@arabic\c@figure`, `\thechapter.` first
                // with chapters (`\@addtoreset{figure}{chapter}`).
                let chapter = chapter_starts.iter().rposition(|(start, _)| *start < at);
                let n = match per_chapter.iter_mut().find(|(c, _)| *c == chapter.unwrap_or(usize::MAX)) {
                    Some((_, n)) => {
                        *n += 1;
                        *n
                    }
                    None => {
                        per_chapter.push((chapter.unwrap_or(usize::MAX), 1));
                        1
                    }
                };
                let number = if settings.chapters {
                    format!("{}.{n}", chapter.map_or("0", |c| chapter_starts[c].1.as_str()))
                } else {
                    n.to_string()
                };
                let title = labels.entry_items.get(doc, f.caption.start, f.caption.end).unwrap_or_else(|| adapter::words_at(&f.text, doc, f.caption.start));
                push(1, Some((number, f.caption)), title, &f.key);
            }
            (None, None) => {}
        }
    }
    out
}
