//! `figure`/`table` floats: source scan, masking and body content.
//!
//! Like the adapter, this re-derives the float's *structure* from the exact
//! source bytes instead of changing the compiler's parse tree (whose
//! `figure` support is a caption paragraph in the text flow). Every float
//! environment in the document body is found here and then blanked to
//! spaces of the same byte length before the compiler parses the document,
//! so every other span stays exact and the surrounding text flows as if the
//! float were an invisible marker — which is what LaTeX does with it.
//!
//! Only the commands that are *float* semantics rather than content are
//! read from the bytes: `\caption` (numbering and the list of figures),
//! `\label` (which resolves to the float's number and page), the alignment
//! declarations, and — until `\includegraphics` sets a box in running text
//! — a standalone `\includegraphics`. **Everything else is body material**:
//! its byte range becomes a [`Piece::Content`], which [`prepare`] parses and
//! adapts into ordinary [`adapter::Block`]s the way `caption_items` already
//! does for one caption argument. `typeset::Context::box_blocks` then sets
//! those blocks with the same code the page's own text goes through, so a
//! `tabular`, a list, a display or a paragraph of prose inside a float is
//! typeset, not dropped.

use flashtex_compiler::{DocumentId, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatKind {
    Figure,
    Table,
}

impl FloatKind {
    pub fn name(self) -> &'static str {
        match self {
            FloatKind::Figure => "Figure",
            FloatKind::Table => "Table",
        }
    }
    /// `\ftype@figure` = 1, `\ftype@table` = 2.
    pub fn type_bit(self) -> u32 {
        match self {
            FloatKind::Figure => 1,
            FloatKind::Table => 2,
        }
    }
}

/// An alignment declaration in a float body (latex.ltx `\centering`,
/// `\raggedright`, `\raggedleft`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Center,
    FlushLeft,
    FlushRight,
}

/// One piece of a float body, in source order.
#[derive(Debug, Clone, PartialEq)]
pub enum Piece {
    /// `\centering`/`\raggedright`/`\raggedleft`. The declaration's own
    /// bytes stay inside the [`Piece::Content`] run that holds them, and are
    /// carried into every later run of the same float (its scope is the rest
    /// of the float box, which a `\caption` between them must not end).
    Align { span: Span, align: Align },
    /// A size declaration (`\tiny` ... `\Huge`, `\normalsize`) at the body's
    /// own level. Like [`Piece::Align`] its bytes stay in their run and are
    /// carried into every later run: `\@caption` sets its box inside
    /// `\begingroup ... \normalsize ... \endgroup` (latex.ltx), so a
    /// `\small` before a `\caption` still sets the `tabular` after it.
    Size { span: Span },
    /// `\includegraphics[options]{path}`; `span` covers the whole command.
    Graphic { span: Span, options: String, path: String },
    /// `\caption[...]{...}`: `span` covers the command, `arg` the argument's
    /// inner bytes, `short` those of the optional argument (latex.ltx
    /// `\@caption#1[#2]#3` writes `#2` to the list of figures/tables).
    Caption { span: Span, arg: Span, short: Option<Span> },
    /// `\label{key}`.
    Label { span: Span, key: String },
    /// A blank line or `\par` between two graphics: ends their line.
    ParBreak,
    /// Body material: everything that is not one of the above, as one
    /// maximal byte range, typeset through the ordinary block path.
    Content { span: Span },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FloatEnv {
    pub kind: FloatKind,
    /// The starred form (`figure*`/`table*`). In a two-column document
    /// `\@floatc@...`'s `\@dblarg` route makes it `\@dbflt`
    /// (latex.ltx 17419: `\if@twocolumn\let\reserved@a\@dbflt\else
    /// \let\reserved@a\@float\fi`), which sets the box at
    /// `\hsize\textwidth \linewidth\textwidth` (`\@xdblfloat`, 17555)
    /// and places it in the page's spanning top area. In a one-column
    /// document the star does nothing at all.
    pub starred: bool,
    /// The placement letters as written (`None` = the class default,
    /// `tbp` for `\@float` and `tp` for `\@dbflt`).
    pub placement: Option<String>,
    /// `\begin{...}` through `\end{...}`.
    pub span: Span,
    /// Whether the float started in horizontal mode (text of the same
    /// paragraph precedes it): `\vadjust` after the line instead of a
    /// vertical-mode marker.
    pub hmode: bool,
    pub pieces: Vec<Piece>,
}

/// LaTeX's `\@xfloat` placement bits: 1 = h, 2 = t, 4 = b, 8 = p, 16 = not
/// `!`. An empty or `!`-only specifier adds the class default (`tbp`).
pub fn placement_bits(placement: Option<&str>, starred: bool) -> Result<u32, String> {
    // `\@dbflt` defaults to `[tp]`, `\@float` to `[tbp]` (latex.ltx 17554,
    // 17538); a full-width float has no bottom area to go to.
    let default = if starred { "tp" } else { "tbp" };
    let mut fps = placement.unwrap_or(default).to_string();
    if fps.is_empty() || fps == "!" {
        fps.push_str(default);
    }
    let mut bits = 16u32;
    for c in fps.chars() {
        match c {
            'h' => bits |= 1,
            't' => bits |= 2,
            'b' => bits |= 4,
            'p' => bits |= 8,
            '!' => bits &= !16,
            'H' => return Err("placement H (float package) is not supported; using h".into()),
            other => return Err(format!("unknown float placement '{other}'; `p` used")),
        }
    }
    Ok(bits)
}

/// Finds every `figure`/`table` environment in the body of `text`.
///
/// Markup that only *spells* a float is skipped: a `%` comment, and the
/// body of `verbatim`, `lstlisting`, `minted`, `comment`, `\verb|...|` or
/// `\lstinline` (`adapter::opaque_regions`). pdflatex never reads those
/// bytes as `\begin{figure}`, and `mask` must not blank them.
pub fn scan(text: &str, document: DocumentId) -> Vec<FloatEnv> {
    let regions = crate::adapter::opaque_regions(text);
    let find_uncommented = |needle: &str, from: usize| find_markup(text, needle, from, &regions);
    let body_start = find_uncommented("\\begin{document}", 0).map_or(0, |p| p + "\\begin{document}".len());
    let mut out = Vec::new();
    let mut at = body_start;
    while let Some(pos) = find_uncommented("\\begin{", at) {
        let name_start = pos + "\\begin{".len();
        let Some(close) = text[name_start..].find('}') else { break };
        let name = &text[name_start..name_start + close];
        let kind = match name {
            "figure" | "figure*" => FloatKind::Figure,
            "table" | "table*" => FloatKind::Table,
            _ => {
                at = name_start;
                continue;
            }
        };
        let mut cursor = name_start + close + 1;
        let end_tag = format!("\\end{{{name}}}");
        let Some(end) = find_uncommented(&end_tag, cursor) else { break };
        let mut placement = None;
        let rest = &text[cursor..end];
        let lead = rest.len() - rest.trim_start().len();
        if rest[lead..].starts_with('[') {
            if let Some(c) = rest[lead..].find(']') {
                placement = Some(rest[lead + 1..lead + c].trim().to_string());
                cursor += lead + c + 1;
            }
        }
        let before = &text[body_start..pos];
        // A float right after another float (only spaces and one newline
        // between) is in the same mode: the `\end{figure}` before it is no
        // paragraph end, unlike a display environment's.
        let after_float = out.last().filter(|p: &&FloatEnv| {
            let gap = &text[p.span.end..pos];
            gap.trim().is_empty() && gap.matches('\n').count() < 2
        });
        let hmode = match after_float {
            Some(prev) => prev.hmode,
            None => !before.trim().is_empty() && !preceded_by_blank_line(before),
        };
        let pieces = pieces(text, cursor, end, document);
        out.push(FloatEnv {
            kind,
            starred: name.ends_with('*'),
            placement,
            span: Span::in_document(document, pos, end + end_tag.len()),
            hmode,
            pieces,
        });
        at = end + end_tag.len();
    }
    out
}

fn preceded_by_blank_line(before: &str) -> bool {
    let trimmed = before.trim_end();
    let gap = &before[trimmed.len()..];
    gap.matches('\n').count() >= 2 || trimmed.ends_with("\\par") || trimmed.ends_with('}') && ends_with_env_end(trimmed)
}

fn ends_with_env_end(s: &str) -> bool {
    s.rfind("\\end{").is_some_and(|p| !s[p..].contains('\n') && s[p..].ends_with('}'))
}

/// `needle` at or after `from`, skipping `%` comments.
fn find_uncommented(text: &str, needle: &str, from: usize) -> Option<usize> {
    find_markup(text, needle, from, &[])
}

/// `needle` at or after `from` that TeX reads as markup: not in a `%`
/// comment and not inside any of the sorted, disjoint `opaque` byte ranges
/// (whose own `%` characters start no comment).
fn find_markup(text: &str, needle: &str, from: usize, opaque: &[(usize, usize)]) -> Option<usize> {
    let region = |i: usize| opaque.get(opaque.partition_point(|r| r.1 <= i)).filter(|r| r.0 <= i);
    let inside = |i: usize| region(i).is_some();
    let mut at = from;
    while let Some(rel) = text.get(at..)?.find(needle) {
        let pos = at + rel;
        if let Some(&(_, end)) = region(pos) {
            at = end;
            continue;
        }
        let line_start = text[..pos].rfind('\n').map_or(0, |i| i + 1);
        let b = text.as_bytes();
        let commented = (line_start..pos).any(|i| {
            b[i] == b'%' && !inside(i) && b[line_start..i].iter().rev().take_while(|&&c| c == b'\\').count() % 2 == 0
        });
        if !commented {
            return Some(pos);
        }
        at = pos + needle.len();
    }
    None
}

/// The inner byte range of the balanced `{...}` group starting at `open`.
fn group(text: &str, open: usize) -> Option<(usize, usize)> {
    let b = text.as_bytes();
    if b.get(open) != Some(&b'{') {
        return None;
    }
    let mut depth = 0i32;
    let mut i = open;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((open + 1, i));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The `]` closing the optional argument opened at `open`, outside braces.
fn optional_end(text: &str, open: usize, end: usize) -> Option<usize> {
    let b = text.as_bytes();
    let mut depth = 0usize;
    let mut i = open + 1;
    while i < end {
        match b[i] {
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            b']' if depth == 0 => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

fn skip_ws(text: &str, mut i: usize, end: usize) -> usize {
    let b = text.as_bytes();
    while i < end && (b[i] == b' ' || b[i] == b'\t' || b[i] == b'\n' || b[i] == b'\r') {
        i += 1;
    }
    i
}

/// The float body's pieces in source order: the float-level commands, and
/// everything else as maximal [`Piece::Content`] runs (see the module docs).
fn pieces(text: &str, start: usize, end: usize, document: DocumentId) -> Vec<Piece> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = start;
    let span = |s: usize, e: usize| Span::in_document(document, s, e);
    // The open body-material run, as `(first byte, last byte + 1)`. Only a
    // float-level command ends it, so a blank line inside it stays in it and
    // the compiler makes the paragraph break.
    let mut run: Option<(usize, usize)> = None;
    // A float-level command is only float-level at the body's own level: an
    // `\includegraphics` in a `tabular` cell or a `\label` inside a group
    // belongs to the material around it, and cutting the run there would
    // hand the compiler three fragments of a table instead of a table.
    let (mut braces, mut envs) = (0usize, 0usize);
    let outer = |braces: usize, envs: usize| braces == 0 && envs == 0;
    macro_rules! flush {
        () => {
            if let Some((s, e)) = run.take() {
                out.push(Piece::Content { span: span(s, e) });
            }
        };
    }
    macro_rules! body {
        ($s:expr, $e:expr) => {{
            let (s, e) = ($s, $e);
            run = Some(match run {
                Some((was, _)) => (was, e),
                None => (s, e),
            });
        }};
    }
    while i < end {
        match b[i] {
            b'%' => {
                i = text[i..end].find('\n').map_or(end, |n| i + n + 1);
            }
            b'\n' => {
                let j = skip_ws(text, i, end);
                if run.is_none() && text[i..j].matches('\n').count() >= 2 {
                    out.push(Piece::ParBreak);
                }
                i = j.max(i + 1);
            }
            b' ' | b'\t' | b'\r' => i += 1,
            b'\\' => {
                let name_end = text[i + 1..end].find(|c: char| !c.is_ascii_alphabetic()).map_or(end, |n| i + 1 + n);
                let name = &text[i + 1..name_end];
                match name {
                    "centering" | "raggedright" | "raggedleft" if outer(braces, envs) => {
                        // The declaration is body material too: its bytes stay
                        // in the run so the compiler sets the paragraph style,
                        // and the piece records it for the graphics line.
                        let align = match name {
                            "centering" => Align::Center,
                            "raggedright" => Align::FlushLeft,
                            _ => Align::FlushRight,
                        };
                        out.push(Piece::Align { span: span(i, name_end), align });
                        body!(i, name_end);
                        i = name_end;
                    }
                    "tiny" | "scriptsize" | "footnotesize" | "small" | "normalsize" | "large" | "Large" | "LARGE" | "huge" | "Huge"
                        if outer(braces, envs) =>
                    {
                        out.push(Piece::Size { span: span(i, name_end) });
                        body!(i, name_end);
                        i = name_end;
                    }
                    "par" if run.is_none() => {
                        out.push(Piece::ParBreak);
                        i = name_end;
                    }
                    "includegraphics" if outer(braces, envs) => {
                        let mut j = skip_ws(text, name_end, end);
                        let mut options = String::new();
                        if b.get(j) == Some(&b'[') {
                            if let Some(c) = text[j..end].find(']') {
                                options = text[j + 1..j + c].to_string();
                                j = skip_ws(text, j + c + 1, end);
                            }
                        }
                        match group(text, j).filter(|(_, e)| *e < end) {
                            Some((s, e)) => {
                                flush!();
                                out.push(Piece::Graphic { span: span(i, e + 1), options, path: text[s..e].trim().to_string() });
                                i = e + 1;
                            }
                            None => {
                                body!(i, name_end);
                                i = name_end;
                            }
                        }
                    }
                    "caption" | "label" if outer(braces, envs) => {
                        let mut j = skip_ws(text, name_end, end);
                        let mut short = None;
                        if name == "caption" && b.get(j) == Some(&b'[') {
                            if let Some(close) = optional_end(text, j, end) {
                                short = Some(span(j + 1, close));
                                j = skip_ws(text, close + 1, end);
                            }
                        }
                        match group(text, j).filter(|(_, e)| *e < end) {
                            Some((s, e)) => {
                                flush!();
                                out.push(if name == "caption" {
                                    Piece::Caption { span: span(i, e + 1), arg: span(s, e), short }
                                } else {
                                    Piece::Label { span: span(i, e + 1), key: text[s..e].trim().to_string() }
                                });
                                i = e + 1;
                            }
                            None => {
                                body!(i, name_end);
                                i = name_end;
                            }
                        }
                    }
                    other => {
                        match other {
                            "begin" => envs += 1,
                            "end" => envs = envs.saturating_sub(1),
                            _ => {}
                        }
                        let stop = name_end.max(i + 2).min(end);
                        body!(i, stop);
                        i = stop;
                    }
                }
            }
            _ => {
                let s = i;
                while i < end && !matches!(b[i], b'\\' | b'\n' | b'%') {
                    match b[i] {
                        b'{' => braces += 1,
                        b'}' => braces = braces.saturating_sub(1),
                        _ => {}
                    }
                    i += 1;
                }
                if !text[s..i].trim().is_empty() {
                    body!(s, i);
                }
            }
        }
    }
    flush!();
    out
}

/// `text` with every float environment replaced by spaces (byte length and
/// every other byte offset preserved).
///
/// A float alone on its line(s) would leave a line of spaces, which TeX
/// reads as a blank line (`\par`). The float itself is no paragraph end: in
/// `First.\n<figure>\nSecond.` pdflatex keeps one paragraph (the newline
/// before the float is the space, `\@esphack` ignores the one after). So
/// such a span starts with `%` instead, which comments out the rest of its
/// line exactly as the float's own bytes left no blank line behind.
pub fn mask(text: &str, floats: &[FloatEnv]) -> String {
    let mut bytes = text.as_bytes().to_vec();
    for f in floats {
        let (start, end) = (f.span.start, f.span.end);
        for b in &mut bytes[start..end] {
            *b = b' ';
        }
        let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
        let line_end = text[end..].find('\n').map_or(text.len(), |i| end + i);
        let blank = |s: &[u8]| s.iter().all(|b| b.is_ascii_whitespace());
        let rest = &bytes[end..line_end];
        let rest_is_empty = blank(rest) || rest.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'%');
        // float.sty's `[H]` is no float: `\float@endH` ends the paragraph
        // with `\par`, so its blank line stays.
        let here_box = !f.starred && f.placement.as_deref() == Some("H");
        // A float at the very tail of a file (only blanks and comments
        // follow, so no `\end{document}`: an `\input`/`\include`d file)
        // keeps the blank-line mask. `\include`'s closing `\clearpage`
        // ends the paragraph in pdflatex, and a `%` here would leave it
        // open across the file boundary, joining it with the entry
        // document's next text (tests/include_float_lists.rs).
        let file_tail = text[end..].lines().all(|l| {
            let t = l.trim_start();
            t.is_empty() || t.starts_with('%')
        });
        if start < end && !here_box && !file_tail && blank(&bytes[line_start..start]) && rest_is_empty {
            bytes[start] = b'%';
        }
    }
    String::from_utf8(bytes).expect("ASCII spaces keep UTF-8 valid")
}

/// `text` blanked except the preamble (through `\begin{document}`) and
/// `keep`: the input for parsing one caption argument in place.
pub fn isolate(text: &str, keep: Span) -> String {
    isolate_all(text, &[keep])
}

/// [`isolate`] keeping several ranges: a float body's content run together
/// with the alignment declarations whose scope reaches it.
pub fn isolate_all(text: &str, keep: &[Span]) -> String {
    let preamble_end = find_uncommented(text, "\\begin{document}", 0).map_or(0, |p| p + "\\begin{document}".len());
    let mut bytes = text.as_bytes().to_vec();
    for (i, b) in bytes.iter_mut().enumerate() {
        if i >= preamble_end && !keep.iter().any(|k| (k.start..k.end).contains(&i)) {
            *b = b' ';
        }
    }
    // Multi-byte characters are kept or blanked whole: `keep` lies on
    // character boundaries and the preamble end is ASCII.
    String::from_utf8(bytes).unwrap_or_default()
}


// ---------------------------------------------------------------------------
// Preparation: numbering, captions, graphics.

use std::collections::HashMap;
use std::rc::Rc;

use flashtex_compiler::parser::SourceDocument;
use flashtex_project_files::path::ProjectPath;
use flashtex_project_files::save::ProjectRoot;

use crate::adapter::{self, CharSrc, Item as AItem, Labels, ParaPart, Segment, TextStyle, Word};
use crate::display::{Diagnostic, ImageResource, SourceRange};
use crate::graphics::{self, GKey, ImageInfo, LengthEnv};
use crate::style::Stylesheet;
use crate::typeset::floatpage::{FloatPart, FloatSpec, Placeholder, PreparedGraphic};
use crate::RenderOptions;

/// Largest image file read (bytes).
pub const MAX_IMAGE_BYTES: u64 = 64 * 1024 * 1024;

/// `(float numbers per document in scan order, label key -> value)`.
///
/// `chapters` is `Some(book)` for report.cls/book.cls, whose `\thefigure`
/// and `\thetable` are `\ifnum\c@chapter>\z@\thechapter.\fi\@arabic\c@figure`
/// and whose `\@addtoreset{figure}{chapter}` restarts both counters at every
/// `\refstepcounter{chapter}`. `\chapter*`, and book's `\chapter` outside
/// `\mainmatter`, step nothing; `\appendix` sets the chapter counter to zero
/// and `\thechapter` to `\@Alph`. The chapter commands are read from `texts`
/// (the masked sources). Floats and commands are taken in `order`
/// ([`adapter::reading_order`]), so a float in the entry file after an
/// `\include` whose file has its own `\chapter` is in that chapter. A
/// float of a document never read (an `\includeonly`-excluded file) has no
/// number and no label value.
pub fn number(envs: &[Vec<FloatEnv>], texts: &[&str], order: &[Span], chapters: Option<bool>) -> (Vec<Vec<Option<String>>>, Vec<(String, String)>) {
    let mut counters = Counters { figures: 0, tables: 0, chapter: 0, appendix: false, mainmatter: true };
    let mut numbers: Vec<Vec<Option<String>>> = envs.iter().map(|doc| vec![None; doc.len()]).collect();
    let mut labels = Vec::new();
    let commands: Vec<Vec<adapter::BodyCommand>> = texts
        .iter()
        .enumerate()
        .map(|(d, text)| match chapters {
            Some(book) if order.iter().any(|s| s.document.0 == d) => adapter::body_commands(text, true, book),
            _ => Vec::new(),
        })
        .collect();
    for segment in order {
        let d = segment.document.0;
        let inside = |at: usize| (segment.start..segment.end).contains(&at);
        let mut cmds = commands.get(d).into_iter().flatten().filter(|c| inside(c.start)).peekable();
        for (fi, f) in envs.get(d).into_iter().flatten().enumerate().filter(|(_, f)| inside(f.span.start)) {
            while let Some(cmd) = cmds.next_if(|c| c.start < f.span.start) {
                counters.step(cmd);
            }
            let has_caption = f.pieces.iter().any(|p| matches!(p, Piece::Caption { .. }));
            let counter = match f.kind {
                FloatKind::Figure => &mut counters.figures,
                FloatKind::Table => &mut counters.tables,
            };
            if has_caption {
                *counter += 1;
            }
            let counter = *counter;
            let value = if counters.chapter > 0 {
                let the_chapter = if counters.appendix { flashtex_class_geometry::Numbering::UpperAlph.format(i64::from(counters.chapter)) } else { counters.chapter.to_string() };
                format!("{the_chapter}.{counter}")
            } else {
                counter.to_string()
            };
            // `\label` after `\caption` takes its number (`\@currentlabel`).
            let mut seen_caption = false;
            for p in &f.pieces {
                match p {
                    Piece::Caption { .. } => seen_caption = true,
                    Piece::Label { key, .. } => labels.push((key.clone(), if seen_caption { value.clone() } else { String::new() })),
                    _ => {}
                }
            }
            numbers[d][fi] = Some(value);
        }
        // The commands after the segment's last float reach the next one.
        cmds.for_each(|cmd| counters.step(cmd));
    }
    (numbers, labels)
}

/// report/book's float counters as [`number`] steps them.
struct Counters {
    figures: u32,
    tables: u32,
    chapter: u32,
    appendix: bool,
    mainmatter: bool,
}

impl Counters {
    fn step(&mut self, cmd: &adapter::BodyCommand) {
        use crate::adapter::{BodyKind, Matter};
        match cmd.kind {
            BodyKind::Chapter { starred: false, .. } if self.mainmatter => {
                self.chapter += 1;
                self.figures = 0;
                self.tables = 0;
            }
            BodyKind::Appendix => {
                self.chapter = 0;
                self.appendix = true;
            }
            BodyKind::Matter(m) => self.mainmatter = m == Matter::Main,
            _ => {}
        }
    }
}

type Loaded = Result<(Rc<ImageResource>, ImageInfo), String>;

/// Reads and probes image files through the project root, once per path.
#[derive(Default)]
pub struct ImageCache {
    root: Option<Result<ProjectRoot, String>>,
    entries: HashMap<(String, u32), Loaded>,
}

impl ImageCache {
    fn load(&mut self, options: &RenderOptions, raw: &str, page: u32) -> Loaded {
        if let Some(hit) = self.entries.get(&(raw.to_string(), page)) {
            return hit.clone();
        }
        let root = self.root.get_or_insert_with(|| match &options.project_root {
            Some(dir) => ProjectRoot::open(dir).map_err(|e| format!("project root {} cannot be opened: {e:?}", dir.display())),
            None => Err("no project root was supplied with the request, so image files cannot be read".into()),
        });
        let result = match root {
            Err(e) => Err(e.clone()),
            Ok(root) => {
                let has_ext = raw.rsplit('/').next().is_some_and(|name| name.contains('.'));
                let mut candidates = Vec::new();
                if has_ext {
                    candidates.push(raw.to_string());
                }
                candidates.extend(graphics::EXTENSIONS.iter().map(|e| format!("{raw}{e}")));
                let mut found: Loaded = Err(format!("image file '{raw}' not found (tried {})", candidates.join(", ")));
                for c in candidates {
                    let Ok(path) = ProjectPath::normalize(&c) else {
                        found = Err(format!("image path '{raw}' is not a project-relative path"));
                        break;
                    };
                    match root.read(&path, MAX_IMAGE_BYTES) {
                        Ok(Some(read)) => {
                            found = graphics::probe(&read.bytes, page)
                                .map(|info| {
                                    let resource = ImageResource {
                                        sha256: Rc::from(flashtex_project_files::sha256::hex(&read.sha256)),
                                        byte_length: read.bytes.len() as u64,
                                        format: info.format.wire_name(),
                                        path: c.clone(),
                                        pixels: info.pixels,
                                        pdf_page: info.pdf_page,
                                        pdf_box: info.pdf_box,
                                        pdf_rotate: info.pdf_rotate,
                                    };
                                    (Rc::new(resource), info)
                                })
                                .map_err(|e| format!("{c}: {e}"));
                            break;
                        }
                        Ok(None) => {}
                        Err(e) => {
                            found = Err(format!("{c}: {e:?}"));
                            break;
                        }
                    }
                }
                found
            }
        };
        self.entries.insert((raw.to_string(), page), result.clone());
        result
    }
}

/// `em`/`ex` of Latin Modern Roman at the body size (quad and x-height).
fn em_ex(body: f64) -> (f64, f64) {
    if body >= 11.5 {
        (11.74988, 5.16654)
    } else if body >= 10.5 {
        (10.95, 4.71438)
    } else {
        (10.0, 4.30554)
    }
}

/// Builds the layout input of every float. `texts` are the masked texts
/// the main parse ran on.
#[allow(clippy::too_many_arguments)]
pub fn prepare(
    envs: &[Vec<FloatEnv>],
    numbers: &[Vec<Option<String>>],
    documents: &[SourceDocument<'_>],
    entry_index: usize,
    texts: &[&str],
    style: &Stylesheet,
    options: &RenderOptions,
    labels: &Labels,
    images: &mut ImageCache,
) -> (Vec<FloatSpec>, Vec<Diagnostic>) {
    let mut specs = Vec::new();
    let mut diags = Vec::new();
    let (em, ex) = em_ex(style.body_size_pt);
    // `\textwidth` is the whole text block even in a two-column document,
    // where `style.text_width_pt` is `\columnwidth` (style.rs: the frame's
    // first column). Inside a `figure*`/`table*` of a two-column document
    // `\@xdblfloat` sets `\hsize\textwidth \linewidth\textwidth`, so
    // `\linewidth` there is the whole block too.
    let full_text_width = style.class_geometry.as_deref().map_or(style.text_width_pt, |g| crate::style::frame_pt(g.frame.text_width));
    let twocolumn = style.class_geometry.as_deref().is_some_and(|g| g.frame.columns.len() > 1);
    let env = LengthEnv { text_width: full_text_width, line_width: style.text_width_pt, text_height: style.text_height_pt, paper_width: style.page_width_pt, paper_height: style.page_height_pt, em, ex };
    let wide_env = LengthEnv { line_width: full_text_width, ..env };
    // `draft`/`demo` are per document, from the class options and every
    // `\usepackage` of `graphics`/`graphicx` in the entry file.
    let gmode = graphics::mode(texts.get(entry_index).copied().unwrap_or_default());
    let float_package = adapter::package_options(texts.get(entry_index).copied().unwrap_or_default(), "float").is_some();
    for (d, doc_envs) in envs.iter().enumerate() {
        let path: Rc<str> = Rc::from(documents[d].path);
        let src = |span: Span| SourceRange { path: path.clone(), start_byte: span.start, end_byte: span.end };
        for (fi, f) in doc_envs.iter().enumerate() {
            // A float of a document never read is not set (`number`).
            let Some(number) = numbers[d][fi].as_deref() else { continue };
            // `\figure*` is `\@dbflt` only `\if@twocolumn` (latex.ltx
            // 17419); in a one-column document the star does nothing.
            let wide = f.starred && twocolumn;
            let env = if wide { &wide_env } else { &env };
            // float.sty `\@xfloat#1[{\@ifnextchar{H}...`: exactly `[H]`, in
            // vertical mode (a blank line before the environment). In the
            // middle of a paragraph `\float@endH`'s `\vskip` would end it
            // there, which the text flow here cannot do yet.
            let exact_here = f.placement.as_deref() == Some("H") && float_package && !wide && !f.hmode;
            let bits = match placement_bits(f.placement.as_deref(), wide) {
                _ if exact_here => 16 | 1,
                Ok(b) => b,
                Err(msg) if msg.starts_with("placement H") && float_package => {
                    let why = if f.hmode { "in the middle of a paragraph (no blank line before the environment)" } else { "on a full-width float" };
                    diags.push(Diagnostic::warning("float_placement", format!("placement H {why} is not supported yet; using h"), vec![src(f.span)]));
                    16 | 1
                }
                Err(msg) => {
                    let fallback = if msg.starts_with("placement H") { 16 | 1 } else { 16 | 8 };
                    diags.push(Diagnostic::warning("float_placement", msg, vec![src(f.span)]));
                    fallback
                }
            };
            let mut parts = Vec::new();
            let mut spec_labels = Vec::new();
            // `\centering`, `\small` and friends stay in force for the rest
            // of the float box, so every later content run is parsed with them.
            let mut aligns: Vec<Span> = Vec::new();
            for piece in &f.pieces {
                match piece {
                    Piece::Align { span, align } => {
                        aligns.push(*span);
                        parts.push(FloatPart::Align(*align));
                    }
                    Piece::Size { span } => aligns.push(*span),
                    Piece::ParBreak => parts.push(FloatPart::ParBreak),
                    Piece::Label { key, .. } => spec_labels.push(key.clone()),
                    Piece::Content { span } => {
                        let mut keep = aligns.clone();
                        keep.push(*span);
                        let (blocks, problems) = body_blocks(&keep, *span, d, documents, entry_index, texts, options, labels);
                        diags.extend(problems);
                        // A `\includegraphics` the scan left in the run is
                        // nested in a group or an environment, where the
                        // adapter drops it like every other running-text
                        // graphic. Losing it silently is the bug this file
                        // is fixing, so it is reported.
                        if documents[d].text[span.start..span.end].contains("\\includegraphics") {
                            diags.push(Diagnostic::warning(
                                "float_content_unsupported",
                                format!(
                                    "{} {number}: an \\includegraphics inside a group or an environment (a `tabular` cell, say) is not set yet and takes no space",
                                    f.kind.name()
                                ),
                                vec![src(*span)],
                            ));
                        }
                        if !blocks.is_empty() {
                            let end_skip = adapter::list_end_skip(documents[d].text, &(span.start..span.end), style.body_size_pt, style);
                            parts.push(FloatPart::Content { blocks, end_skip });
                        }
                    }
                    Piece::Graphic { span, options: opts, path: file } => {
                        let (keys, problems) = graphics::parse_keys(opts, env);
                        for p in problems {
                            diags.push(Diagnostic::warning("graphics_option", p, vec![src(*span)]));
                        }
                        for k in &keys {
                            if let GKey::Unsupported(name) = k {
                                diags.push(Diagnostic::warning("graphics_option", format!("\\includegraphics key '{name}' is not honoured yet"), vec![src(*span)]));
                            }
                        }
                        let page = keys.iter().find_map(|k| if let GKey::Page(p) = k { Some(*p) } else { None }).unwrap_or(1);
                        // `demo` replaced `\Ginclude@graphics` with a rule,
                        // so no file is looked up and the per-image `draft`
                        // key never reaches `\Gin@setfile`'s draft branch.
                        if gmode.demo {
                            let gbox = graphics::demo_box(&keys);
                            parts.push(FloatPart::Graphic(PreparedGraphic { gbox, resource: None, placeholder: Some(Placeholder::DemoRule), span: *span }));
                            continue;
                        }
                        let draft = keys.iter().rev().find_map(|k| if let GKey::Draft(v) = k { Some(*v) } else { None }).unwrap_or(gmode.draft);
                        match images.load(options, file, page) {
                            Ok((resource, info)) => {
                                let gbox = graphics::size_box(info.width_bp / graphics::BP_PER_PT, info.height_bp / graphics::BP_PER_PT, &keys);
                                // Under `draft` the box is sized from the
                                // file and the file is not embedded: the
                                // space is the same and the ink is the
                                // frame `\Gin@setfile` draws instead.
                                let (resource, placeholder) = if draft { (None, Some(Placeholder::DraftFrame)) } else { (Some(resource), None) };
                                parts.push(FloatPart::Graphic(PreparedGraphic { gbox, resource, placeholder, span: *span }));
                            }
                            // `pdftex.def`'s `\Gread@pdftex` leaves a file
                            // it cannot find at the bounding box `0 0 72
                            // 72` and, under `draft`, warns instead of
                            // raising its package error -- so the graphic
                            // still takes one inch square of space, scaled
                            // by whatever the keys ask for.
                            Err(msg) if draft => {
                                let nat = graphics::MISSING_NATURAL_BP / graphics::BP_PER_PT;
                                let gbox = graphics::size_box(nat, nat, &keys);
                                diags.push(Diagnostic::warning(
                                    "image_unavailable",
                                    format!("{msg}; the `draft` option keeps its 1 in natural size, as pdfTeX does"),
                                    vec![src(*span)],
                                ));
                                parts.push(FloatPart::Graphic(PreparedGraphic { gbox, resource: None, placeholder: Some(Placeholder::DraftFrame), span: *span }));
                            }
                            Err(msg) => {
                                let w = keys.iter().rev().find_map(|k| if let GKey::Width(v) = k { Some(*v) } else { None });
                                let h = keys.iter().rev().find_map(|k| if let GKey::Height(v) | GKey::TotalHeight(v) = k { Some(*v) } else { None });
                                match (w, h) {
                                    (Some(w), Some(h)) => {
                                        diags.push(Diagnostic::error("image_unavailable", format!("{msg} (its requested size is kept empty)"), vec![src(*span)]));
                                        let gbox = graphics::GraphicBox { width: w, height: h, depth: 0.0, matrix: [w, 0.0, 0.0, h, 0.0, 0.0] };
                                        parts.push(FloatPart::Graphic(PreparedGraphic { gbox, resource: None, placeholder: None, span: *span }));
                                    }
                                    _ => diags.push(Diagnostic::error("image_unavailable", msg, vec![src(*span)])),
                                }
                            }
                        }
                    }
                    Piece::Caption { span, arg, .. } => {
                        let items = caption_items(f.kind, number, *span, *arg, d, documents, entry_index, texts, options, labels);
                        parts.push(FloatPart::Caption { items });
                    }
                }
            }
            specs.push(FloatSpec { kind: f.kind, number: number.to_string(), wide, bits, span: f.span, hmode: f.hmode, exact_here, parts, labels: spec_labels });
        }
    }
    (specs, diags)
}

/// The blocks of one content run of a float body: the document with
/// everything but `keep` blanked is parsed and adapted, exactly as
/// [`caption_items`] does for a caption argument, so the run's `tabular`,
/// list, display or prose becomes ordinary [`adapter::Block`]s. Only the
/// diagnostics the run itself raised are returned (the rest of the
/// document, and the preamble, are reported by the main parse).
#[allow(clippy::too_many_arguments)]
fn body_blocks(
    keep: &[Span],
    run: Span,
    d: usize,
    documents: &[SourceDocument<'_>],
    entry_index: usize,
    texts: &[&str],
    options: &RenderOptions,
    labels: &Labels,
) -> (Vec<adapter::Block>, Vec<Diagnostic>) {
    let isolated = isolate_all(documents[d].text, keep);
    let mut texts2: Vec<&str> = texts.to_vec();
    texts2[d] = &isolated;
    let docs2: Vec<SourceDocument<'_>> = documents.iter().zip(&texts2).map(|(doc, t)| SourceDocument { path: doc.path, text: t }).collect();
    let parsed = flashtex_compiler::parser::parse_project(&docs2, documents[d].path);
    let doc = adapter::adapt(&texts2, entry_index, &parsed, options, labels);
    let path = documents[d].path;
    let mine = |dg: &Diagnostic| {
        dg.sources.iter().any(|s| s.path.as_ref() == path && s.start_byte >= run.start && s.start_byte < run.end)
    };
    let mut blocks = doc.blocks;
    // `\centering` is a declaration, not `\begin{center}`: it adds no
    // `\topsep`/`\partopsep` and no closing `\@endparenv` skip. The
    // adapter decides that from the bytes before the block, which here are
    // the preamble the isolation kept -- so the run's leading
    // `\begin{document}` reads as an environment opening. Every block
    // before the run's first paragraph-shape environment (all of them when
    // it has none) therefore has no environment to open or close.
    let styled_at = styled_env_at(&documents[d].text[run.start..run.end]).map_or(usize::MAX, |at| run.start + at);
    for block in &mut blocks {
        if let adapter::Block::Paragraph { parts, env_open, env_close, .. } = block {
            if paragraph_start(parts).is_none_or(|start| start < styled_at) {
                *env_open = None;
                *env_close = false;
            }
        }
    }
    (blocks, doc.diagnostics.into_iter().filter(mine).collect())
}

/// Where `run` first opens one of the compiler's paragraph-shape
/// environments (`parser::paragraph_style`), whose `\trivlist` really does
/// add the `\topsep` glue around it.
fn styled_env_at(run: &str) -> Option<usize> {
    ["center", "flushright", "flushleft", "quote", "quotation", "verse"]
        .iter()
        .filter_map(|name| run.find(&format!("\\begin{{{name}}}")))
        .min()
}

/// The first source byte a paragraph block was set from.
fn paragraph_start(parts: &[adapter::ParaPart]) -> Option<usize> {
    parts.iter().find_map(|part| match part {
        adapter::ParaPart::Lines(items) => crate::incremental::block_origin(items).map(|(_, start)| start),
        adapter::ParaPart::Display { span, .. } | adapter::ParaPart::Rows { span, .. } => Some(span.start),
    })
}

#[allow(clippy::too_many_arguments)]
fn caption_items(
    kind: FloatKind,
    number: &str,
    span: Span,
    arg: Span,
    d: usize,
    documents: &[SourceDocument<'_>],
    entry_index: usize,
    texts: &[&str],
    options: &RenderOptions,
    labels: &Labels,
) -> Vec<AItem> {
    let isolated = isolate(documents[d].text, arg);
    let mut texts2: Vec<&str> = texts.to_vec();
    texts2[d] = &isolated;
    let docs2: Vec<SourceDocument<'_>> = documents.iter().zip(&texts2).map(|(doc, t)| SourceDocument { path: doc.path, text: t }).collect();
    let parsed = flashtex_compiler::parser::parse_project(&docs2, documents[d].path);
    let doc = adapter::adapt(&texts2, entry_index, &parsed, options, labels);
    let origin = CharSrc { document: span.document, start: span.start, end: span.start + "\\caption".len() };
    let word = |t: &str| AItem::Word(Word { segments: vec![Segment { text: t.to_string(), chars: t.chars().map(|_| origin).collect(), style: TextStyle::default() }] });
    let mut items = vec![word(kind.name()), AItem::Space { style: TextStyle::default(), factor: 1000, no_break: true }, word(&format!("{number}:"))];
    let mut body = Vec::new();
    for block in &doc.blocks {
        if let adapter::Block::Paragraph { parts, .. } = block {
            for part in parts {
                if let ParaPart::Lines(lines) = part {
                    if !body.is_empty() {
                        body.push(AItem::Space { style: TextStyle::default(), factor: 1000, no_break: false });
                    }
                    body.extend(lines.iter().cloned());
                }
            }
        }
    }
    if !body.is_empty() {
        items.push(AItem::Space { style: TextStyle::default(), factor: adapter::space_factor(':', 1000), no_break: false });
        items.extend(body);
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_pieces_and_masks_in_place() {
        let src = "\\begin{document}\nText before.\n\n\\begin{figure}[ht]\n  \\centering\n  \\includegraphics[width=2in]{a.png}\n  \\caption{A {nested} cap.}\\label{fig:a}\n\\end{figure}\n\nAfter.\n\\end{document}\n";
        let f = scan(src, DocumentId(0));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].placement.as_deref(), Some("ht"));
        assert!(!f[0].hmode);
        assert!(matches!(f[0].pieces[0], Piece::Align { align: Align::Center, .. }));
        // `\centering`'s own bytes stay in a content run (the compiler reads
        // the declaration), and the graphic ends that run.
        assert!(matches!(&f[0].pieces[1], Piece::Content { span } if &src[span.start..span.end] == "\\centering"));
        match &f[0].pieces[2] {
            Piece::Graphic { options, path, .. } => assert_eq!((options.as_str(), path.as_str()), ("width=2in", "a.png")),
            other => panic!("{other:?}"),
        }
        match &f[0].pieces[3] {
            Piece::Caption { arg, .. } => assert_eq!(&src[arg.start..arg.end], "A {nested} cap."),
            other => panic!("{other:?}"),
        }
        assert!(matches!(&f[0].pieces[4], Piece::Label { key, .. } if key == "fig:a"));
        let masked = mask(src, &f);
        assert_eq!(masked.len(), src.len());
        assert!(!masked.contains("figure") && masked.contains("After."));
        assert_eq!(placement_bits(Some("ht"), false), Ok(16 | 1 | 2));
        // `\@fpsadddefault`: a bare `!` becomes `!tbp`, and `!` clears 16.
        assert_eq!(placement_bits(Some("!"), false), Ok(2 | 4 | 8));
        assert_eq!(placement_bits(Some("!h"), false), Ok(1));
    }

    #[test]
    fn body_material_is_one_content_run_across_blank_lines() {
        let src = "\\begin{document}\n\\begin{table}\n\\centering\n\\caption{C}\n\\begin{tabular}{ll}\na & b \\\\\n\\end{tabular}\n\nAnd a note.\n\\end{table}\n\\end{document}\n";
        let f = scan(src, DocumentId(0));
        let runs: Vec<&str> = f[0]
            .pieces
            .iter()
            .filter_map(|p| match p {
                Piece::Content { span } => Some(&src[span.start..span.end]),
                _ => None,
            })
            .collect();
        // `\centering` before the caption, then one run holding the whole
        // tabular *and* the paragraph after the blank line.
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0], "\\centering");
        assert!(runs[1].starts_with("\\begin{tabular}") && runs[1].ends_with("And a note."));
        assert!(matches!(f[0].pieces[2], Piece::Caption { .. }));
    }

    #[test]
    fn a_size_declaration_is_carried_past_the_caption() {
        let src = "\\begin{document}\n\\begin{table}\n\\centering\\small\n\\caption{C}\n\\begin{tabular}{l}\na\n\\end{tabular}\n{\\large x}\n\\end{table}\n\\end{document}\n";
        let f = scan(src, DocumentId(0));
        let sizes: Vec<&str> = f[0]
            .pieces
            .iter()
            .filter_map(|p| match p {
                Piece::Size { span } => Some(&src[span.start..span.end]),
                _ => None,
            })
            .collect();
        // `\small` is float-level; the `\large` inside a group is not.
        assert_eq!(sizes, ["\\small"]);
        assert!(matches!(&f[0].pieces[2], Piece::Content { span } if &src[span.start..span.end] == "\\centering\\small"));
    }

    #[test]
    fn a_command_inside_a_cell_does_not_cut_the_run() {
        let src = "\\begin{document}\n\\begin{table}\n\\centering\n\\begin{tabular}{ll}\nA\\label{r}& \\includegraphics{p.png}\\\\\n\\end{tabular}\n\\caption{C}\n\\end{table}\n\\end{document}\n";
        let f = scan(src, DocumentId(0));
        // One run holding the whole tabular: the `\\label` and the
        // `\\includegraphics` in its cells are the table's, not the float's.
        let runs: Vec<&str> = f[0]
            .pieces
            .iter()
            .filter_map(|p| match p {
                Piece::Content { span } => Some(&src[span.start..span.end]),
                _ => None,
            })
            .collect();
        assert_eq!(runs.len(), 1, "{runs:?}");
        assert!(runs[0].starts_with("\\centering") && runs[0].ends_with("\\end{tabular}"));
        assert!(!f[0].pieces.iter().any(|p| matches!(p, Piece::Graphic { .. } | Piece::Label { .. })));
        assert!(f[0].pieces.iter().any(|p| matches!(p, Piece::Caption { .. })));
    }

    #[test]
    fn a_float_inside_a_paragraph_is_horizontal_mode() {
        let src = "Some text\n\\begin{figure}\\caption{x}\\end{figure} more.";
        assert!(scan(src, DocumentId(0))[0].hmode);
    }

    #[test]
    fn a_float_on_its_own_lines_is_masked_without_a_blank_line() {
        let fig = "\\begin{figure}[h]\n\\caption{x}\n\\end{figure}";
        let masked = |src: &str| mask(src, &scan(src, DocumentId(0)));
        // Inside a paragraph: a comment line, not a line of spaces.
        let src = format!("\\begin{{document}}\nFirst.\n  {fig}  \nSecond.\n");
        let m = masked(&src);
        assert_eq!(m.len(), src.len());
        assert_eq!(m, format!("\\begin{{document}}\nFirst.\n  %{}  \nSecond.\n", " ".repeat(fig.len() - 1)));
        // Text before or after on the same line keeps the line non-blank,
        // so nothing may be commented out.
        for src in [format!("\\begin{{document}}\nFirst. {fig}\nSecond.\n"), format!("\\begin{{document}}\nFirst.\n{fig} Second.\n")] {
            let m = masked(&src);
            assert!(!m.contains('%') && m.contains("Second.") && m.contains("First."), "{m:?}");
        }
        // Blank lines around the float are the document's own: still there.
        let src = format!("\\begin{{document}}\nFirst.\n\n{fig}\n\nSecond.\n");
        assert!(masked(&src).contains("First.\n\n%"));
        // float.sty's `[H]` ends the paragraph in pdflatex: no `%`.
        let src = format!("\\begin{{document}}\nFirst.\n{}\nSecond.\n", fig.replace("[h]", "[H]"));
        assert!(!masked(&src).contains('%'));
        // Two floats in a row inside a paragraph are both horizontal mode;
        // after a blank line, both vertical.
        let two = scan(&format!("\\begin{{document}}\nFirst.\n{fig}\n{fig}\nSecond.\n"), DocumentId(0));
        assert!(two[0].hmode && two[1].hmode);
        let two = scan(&format!("\\begin{{document}}\nFirst.\n\n{fig}\n{fig}\nSecond.\n"), DocumentId(0));
        assert!(!two[0].hmode && !two[1].hmode);
        // `wrapfigure` is not a float here: its bytes reach the compiler.
        let src = "\\begin{document}\nFirst.\n\\begin{wrapfigure}{r}{1in}\nx\n\\end{wrapfigure}\nSecond.\n";
        assert!(scan(src, DocumentId(0)).is_empty());
    }

    #[test]
    fn markup_that_only_spells_a_float_is_not_one() {
        let fig = "\\begin{figure}[h]\n\\caption{x}\n\\end{figure}";
        let lookalikes = [
            format!("\\begin{{verbatim}}\n{fig}\n\\end{{verbatim}}"),
            format!("\\begin{{verbatim*}}\n{fig}\n\\end{{verbatim*}}"),
            format!("\\begin{{lstlisting}}[language=TeX]\n{fig}\n\\end{{lstlisting}}"),
            format!("\\begin{{minted}}{{latex}}\n{fig}\n\\end{{minted}}"),
            format!("\\begin{{comment}}\n{fig}\n\\end{{comment}}"),
            "Write \\verb|\\begin{figure}| and \\verb+\\end{figure}+.".to_string(),
            "Write \\lstinline!\\begin{figure}\\end{figure}! here.".to_string(),
            "% \\begin{figure}\\caption{x}\\end{figure}".to_string(),
            "Text. % \\begin{figure}\n% \\end{figure}".to_string(),
        ];
        for body in lookalikes {
            let src = format!("\\begin{{document}}\nFirst.\n{body}\nSecond.\n");
            assert!(scan(&src, DocumentId(0)).is_empty(), "{src:?}");
        }
        // A real float right after a verbatim block that holds a lookalike:
        // found once, vertical mode (after `\end{verbatim}`), and the
        // verbatim bytes survive `mask` untouched.
        let verbatim = format!("\\begin{{verbatim}}\n{fig}\n\\end{{verbatim}}");
        let src = format!("\\begin{{document}}\nFirst.\n{verbatim}\n{fig}\nSecond.\n");
        let found = scan(&src, DocumentId(0));
        assert_eq!(found.len(), 1);
        let real = src.rfind("\\begin{figure}").unwrap();
        assert_eq!((found[0].span.start, found[0].hmode), (real, false));
        assert!(mask(&src, &found).contains(&verbatim));
        // A `%` inside `\verb` starts no comment, and `\\%` is a comment.
        let src = format!("\\begin{{document}}\nFirst \\verb|%| {fig}\nSecond.\n");
        assert_eq!(scan(&src, DocumentId(0)).len(), 1);
        let src = format!("\\begin{{document}}\nFirst \\\\% {fig}\nSecond.\n");
        assert!(scan(&src, DocumentId(0)).is_empty());
    }
}
