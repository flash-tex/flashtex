//! `figure`/`table` floats: source scan and masking.
//!
//! Like the adapter, this re-derives structure from the exact source bytes
//! instead of changing the compiler's parse tree (whose `figure` support is
//! a caption paragraph in the text flow). Every float environment in the
//! document body is found here, split into its pieces (`\includegraphics`,
//! `\caption`, `\label`, `\centering`), and then blanked to spaces of the
//! same byte length before the compiler parses the document, so every other
//! span stays exact and the surrounding text flows as if the float were an
//! invisible marker — which is what LaTeX does with it.

use flashtex_compiler::{DocumentId, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatKind {
    Figure,
    Table,
    /// algorithm.sty's float (`crate::algorithms`).
    Algorithm,
}

impl FloatKind {
    pub fn name(self) -> &'static str {
        match self {
            FloatKind::Figure => "Figure",
            FloatKind::Table => "Table",
            FloatKind::Algorithm => "Algorithm",
        }
    }
    /// `\ftype@figure` = 1, `\ftype@table` = 2; float.sty's `\newfloat`
    /// continues from 4 when `figure` and `table` exist (float.sty 24-27).
    pub fn type_bit(self) -> u32 {
        match self {
            FloatKind::Figure => 1,
            FloatKind::Table => 2,
            FloatKind::Algorithm => 4,
        }
    }
}

/// One piece of a float body, in source order.
#[derive(Debug, Clone, PartialEq)]
pub enum Piece {
    /// `\centering`.
    Centering,
    /// `\includegraphics[options]{path}`; `span` covers the whole command.
    Graphic { span: Span, options: String, path: String },
    /// `\caption[...]{...}`: `span` covers the command, `arg` the argument's
    /// inner bytes, `short` those of the optional argument (latex.ltx
    /// `\@caption#1[#2]#3` writes `#2` to the list of figures/tables).
    Caption { span: Span, arg: Span, short: Option<Span> },
    /// `\label{key}`.
    Label { span: Span, key: String },
    /// A blank line or `\par`: ends the current paragraph.
    ParBreak,
    /// Anything else that is not whitespace: not typeset (reported).
    Other { span: Span },
    /// `\begin{minipage}[pos]{width}` .. `\end{minipage}`: `pos` is `b'c'`,
    /// `b't'` or `b'b'`; `body` the inner bytes, split into its own pieces.
    Minipage { span: Span, pos: u8, width: String, body: (usize, usize), pieces: Vec<Piece> },
    /// Horizontal glue beside a graphic or minipage on its line.
    HSkip { span: Span, glue: HGlue },
    /// An interword space after a graphic or minipage (an end of line after
    /// `}`; a control word's trailing blanks are skipped).
    Space,
}

/// Horizontal glue on a line of boxes.
#[derive(Debug, Clone, PartialEq)]
pub enum HGlue {
    /// `\hfil` (order 1) or `\hfill` (order 2).
    Infinite(u8),
    /// `\hspace{<dimen>}` as written.
    Dimen(String),
    /// `\quad` (1), `\qquad` (2), `\enskip` (0.5): ems of the font.
    Em(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FloatEnv {
    pub kind: FloatKind,
    /// The placement letters as written (`None` = the class default `tbp`).
    pub placement: Option<String>,
    /// `\begin{...}` through `\end{...}`.
    pub span: Span,
    /// Whether the float started in horizontal mode (text of the same
    /// paragraph precedes it): `\vadjust` after the line instead of a
    /// vertical-mode marker.
    pub hmode: bool,
    /// `figure*`/`table*`.
    pub wide: bool,
    /// The body's byte range (after `\begin{...}[...]`, before `\end`).
    pub body: (usize, usize),
    pub pieces: Vec<Piece>,
}

/// LaTeX's `\@xfloat` placement bits: 1 = h, 2 = t, 4 = b, 8 = p, 16 = not
/// `!`. An empty or `!`-only specifier adds the class default (`tbp`).
pub fn placement_bits(placement: Option<&str>) -> Result<u32, String> {
    let mut fps = placement.unwrap_or("tbp").to_string();
    if fps.is_empty() || fps == "!" {
        fps.push_str("tbp");
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
pub fn scan(text: &str, document: DocumentId) -> Vec<FloatEnv> {
    let body_start = find_uncommented(text, "\\begin{document}", 0).map_or(0, |p| p + "\\begin{document}".len());
    let mut out = Vec::new();
    let mut at = body_start;
    while let Some(pos) = find_uncommented(text, "\\begin{", at) {
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
        let Some(end) = find_uncommented(text, &end_tag, cursor) else { break };
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
        let hmode = !before.trim().is_empty() && !preceded_by_blank_line(before);
        let pieces = pieces(text, cursor, end, document);
        out.push(FloatEnv {
            kind,
            placement,
            span: Span::in_document(document, pos, end + end_tag.len()),
            hmode,
            wide: name.ends_with('*'),
            body: (cursor, end),
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
    let mut at = from;
    while let Some(rel) = text.get(at..)?.find(needle) {
        let pos = at + rel;
        let line_start = text[..pos].rfind('\n').map_or(0, |i| i + 1);
        if !is_commented(&text[line_start..pos]) {
            return Some(pos);
        }
        at = pos + needle.len();
    }
    None
}

fn is_commented(line_prefix: &str) -> bool {
    let b = line_prefix.as_bytes();
    (0..b.len()).any(|i| b[i] == b'%' && (i == 0 || b[i - 1] != b'\\'))
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

fn pieces(text: &str, start: usize, end: usize, document: DocumentId) -> Vec<Piece> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = start;
    let span = |s: usize, e: usize| Span::in_document(document, s, e);
    while i < end {
        match b[i] {
            b'%' => {
                i = text[i..end].find('\n').map_or(end, |n| i + n + 1);
                // The next line's leading blanks are skipped (state N).
                while i < end && matches!(b[i], b' ' | b'\t') {
                    i += 1;
                }
            }
            b'\n' | b' ' | b'\t' | b'\r' => {
                let j = skip_ws(text, i, end);
                if text[i..j].matches('\n').count() >= 2 {
                    out.push(Piece::ParBreak);
                } else if !after_control_word(&text[start..i]) && matches!(out.last(), Some(Piece::Graphic { .. } | Piece::Minipage { .. } | Piece::HSkip { glue: HGlue::Dimen(_), .. })) {
                    out.push(Piece::Space);
                }
                i = j.max(i + 1);
            }
            b'\\' => {
                let name_end = text[i + 1..end].find(|c: char| !c.is_ascii_alphabetic()).map_or(end, |n| i + 1 + n);
                let name = &text[i + 1..name_end];
                match name {
                    "centering" => {
                        out.push(Piece::Centering);
                        i = name_end;
                    }
                    "par" => {
                        out.push(Piece::ParBreak);
                        i = name_end;
                    }
                    "hfill" | "hfil" | "quad" | "qquad" | "enskip" => {
                        let glue = match name {
                            "hfill" => HGlue::Infinite(2),
                            "hfil" => HGlue::Infinite(1),
                            "quad" => HGlue::Em(1.0),
                            "qquad" => HGlue::Em(2.0),
                            _ => HGlue::Em(0.5),
                        };
                        out.push(Piece::HSkip { span: span(i, name_end), glue });
                        i = name_end;
                    }
                    "hspace" => {
                        let mut j = skip_ws(text, name_end, end);
                        if b.get(j) == Some(&b'*') {
                            j = skip_ws(text, j + 1, end);
                        }
                        match group(text, j).filter(|(_, e)| *e < end) {
                            Some((s, e)) => {
                                out.push(Piece::HSkip { span: span(i, e + 1), glue: HGlue::Dimen(text[s..e].trim().to_string()) });
                                i = e + 1;
                            }
                            None => {
                                out.push(Piece::Other { span: span(i, name_end) });
                                i = name_end;
                            }
                        }
                    }
                    "begin" if text[name_end..end].starts_with("{minipage}") => match minipage(text, i, name_end + "{minipage}".len(), end, document) {
                        Some((piece, next)) => {
                            out.push(piece);
                            i = next;
                        }
                        None => {
                            out.push(Piece::Other { span: span(i, name_end) });
                            i = name_end;
                        }
                    },
                    "includegraphics" => {
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
                                out.push(Piece::Graphic { span: span(i, e + 1), options, path: text[s..e].trim().to_string() });
                                i = e + 1;
                            }
                            None => {
                                out.push(Piece::Other { span: span(i, name_end) });
                                i = name_end;
                            }
                        }
                    }
                    "caption" | "label" => {
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
                                out.push(if name == "caption" {
                                    Piece::Caption { span: span(i, e + 1), arg: span(s, e), short }
                                } else {
                                    Piece::Label { span: span(i, e + 1), key: text[s..e].trim().to_string() }
                                });
                                i = e + 1;
                            }
                            None => {
                                out.push(Piece::Other { span: span(i, name_end) });
                                i = name_end;
                            }
                        }
                    }
                    _ => {
                        let stop = name_end.max(i + 2).min(end);
                        out.push(Piece::Other { span: span(i, stop) });
                        i = stop;
                    }
                }
            }
            _ => {
                let s = i;
                while i < end && !matches!(b[i], b'\\' | b'\n' | b'%') {
                    i += 1;
                }
                if !text[s..i].trim().is_empty() {
                    out.push(Piece::Other { span: span(s, i) });
                }
            }
        }
    }
    // Glue is set on a line of boxes only beside a graphic or minipage;
    // anywhere else it is text material the adapter sets.
    let keep: Vec<bool> = (0..out.len()).map(|k| !matches!(out[k], Piece::HSkip { .. }) || box_beside(&out, k, false) || box_beside(&out, k, true)).collect();
    out.into_iter()
        .zip(keep)
        .map(|(p, keep)| match p {
            Piece::HSkip { span, .. } if !keep => Piece::Other { span },
            p => p,
        })
        .collect()
}

/// Whether the nearest piece before (`forward` false) or after `k`, past
/// glue and spaces, is a graphic or a minipage.
fn box_beside(out: &[Piece], k: usize, forward: bool) -> bool {
    let mut m = k;
    loop {
        if forward {
            m += 1;
            if m >= out.len() {
                return false;
            }
        } else {
            if m == 0 {
                return false;
            }
            m -= 1;
        }
        match out[m] {
            Piece::HSkip { .. } | Piece::Space => {}
            Piece::Graphic { .. } | Piece::Minipage { .. } => return true,
            _ => return false,
        }
    }
}

/// `s` ends with a control word (whose trailing blanks TeX skips).
fn after_control_word(s: &str) -> bool {
    let t = s.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    t.len() < s.len() && t.ends_with('\\') && !t[..t.len() - 1].ends_with('\\')
}

/// `\begin{minipage}[pos][height][inner-pos]{width}` .. `\end{minipage}`
/// starting at `at` (`after` is just past `{minipage}`): the piece and the
/// byte after `\end{minipage}`.
fn minipage(text: &str, at: usize, after: usize, end: usize, document: DocumentId) -> Option<(Piece, usize)> {
    const END: &str = "\\end{minipage}";
    let b = text.as_bytes();
    let mut k = after;
    let mut pos = None;
    loop {
        let j = skip_ws(text, k, end);
        if b.get(j) != Some(&b'[') {
            break;
        }
        let c = text[j..end].find(']')?;
        pos.get_or_insert_with(|| text[j + 1..j + c].trim().bytes().next().unwrap_or(b'c'));
        k = j + c + 1;
    }
    let (ws, we) = group(text, skip_ws(text, k, end)).filter(|(_, e)| *e < end)?;
    let body_start = we + 1;
    let (mut depth, mut from) = (1, body_start);
    let close = loop {
        let next_end = find_uncommented(text, END, from).filter(|p| *p < end)?;
        match find_uncommented(text, "\\begin{minipage}", from).filter(|p| *p < next_end) {
            Some(p) => {
                depth += 1;
                from = p + 1;
            }
            None => {
                depth -= 1;
                if depth == 0 {
                    break next_end;
                }
                from = next_end + 1;
            }
        }
    };
    let stop = close + END.len();
    let pos = match pos {
        Some(p @ (b't' | b'b')) => p,
        _ => b'c',
    };
    let piece = Piece::Minipage {
        span: Span::in_document(document, at, stop),
        pos,
        width: text[ws..we].trim().to_string(),
        body: (body_start, close),
        pieces: pieces(text, body_start, close, document),
    };
    Some((piece, stop))
}

/// `text` with every float environment replaced by spaces (byte length and
/// every other byte offset preserved).
pub fn mask(text: &str, floats: &[FloatEnv]) -> String {
    let mut bytes = text.as_bytes().to_vec();
    for f in floats {
        for b in &mut bytes[f.span.start..f.span.end] {
            *b = b' ';
        }
    }
    String::from_utf8(bytes).expect("ASCII spaces keep UTF-8 valid")
}

/// `text` blanked except the preamble (through `\begin{document}`) and
/// `keep`: the input for parsing one caption argument in place.
pub fn isolate(text: &str, keep: Span) -> String {
    let preamble_end = find_uncommented(text, "\\begin{document}", 0).map_or(0, |p| p + "\\begin{document}".len());
    let mut bytes = text.as_bytes().to_vec();
    for (i, b) in bytes.iter_mut().enumerate() {
        if i >= preamble_end && !(keep.start..keep.end).contains(&i) {
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
use crate::typeset::floatpage::{FloatPart, FloatSpec, PreparedGraphic};
use crate::RenderOptions;

/// Largest image file read (bytes).
pub const MAX_IMAGE_BYTES: u64 = 64 * 1024 * 1024;

/// `\caption`s and `\label`s of a body in source order, minipages included.
fn captions_and_labels<'p>(pieces: &'p [Piece], out: &mut Vec<&'p Piece>) {
    for p in pieces {
        match p {
            Piece::Caption { .. } | Piece::Label { .. } => out.push(p),
            Piece::Minipage { pieces, .. } => captions_and_labels(pieces, out),
            _ => {}
        }
    }
}

/// `(float numbers per document in scan order, label key -> value)`. A
/// float's number is its first caption's (the counter itself when it has
/// none); every `\caption` steps the counter.
pub fn number(envs: &[Vec<FloatEnv>]) -> (Vec<Vec<u32>>, Vec<(String, String)>) {
    // `algorithm` floats are numbered by `crate::algorithms::number`; `scan`
    // never yields them.
    let (mut figures, mut tables, mut algorithms) = (0u32, 0u32, 0u32);
    let mut numbers = Vec::new();
    let mut labels = Vec::new();
    for doc in envs {
        let mut nums = Vec::new();
        for f in doc {
            let counter = match f.kind {
                FloatKind::Figure => &mut figures,
                FloatKind::Table => &mut tables,
                FloatKind::Algorithm => &mut algorithms,
            };
            let mut seq = Vec::new();
            captions_and_labels(&f.pieces, &mut seq);
            nums.push(*counter + u32::from(seq.iter().any(|p| matches!(p, Piece::Caption { .. }))));
            // `\label` after `\caption` takes its number (`\@currentlabel`).
            let mut seen_caption = false;
            for p in seq {
                match p {
                    Piece::Caption { .. } => {
                        *counter += 1;
                        seen_caption = true;
                    }
                    Piece::Label { key, .. } => labels.push((key.clone(), if seen_caption { counter.to_string() } else { String::new() })),
                    _ => {}
                }
            }
        }
        numbers.push(nums);
    }
    (numbers, labels)
}

type Loaded = Result<(Rc<ImageResource>, ImageInfo), String>;

/// Reads and probes image files through the project root, once per path.
#[derive(Default)]
pub struct ImageCache {
    root: Option<Result<ProjectRoot, String>>,
    entries: HashMap<(String, u32), Loaded>,
    /// `\graphicspath` directories, tried after the name as given (LaTeX's
    /// `\IfFileExists` searches `\input@path`, which graphics.sty sets to
    /// `\Ginput@path`), for each extension in turn.
    paths: Vec<String>,
}

impl ImageCache {
    pub fn set_search_path(&mut self, paths: Vec<String>) {
        if self.paths != paths {
            self.paths = paths;
            self.entries.clear();
        }
    }

    pub(crate) fn load(&mut self, options: &RenderOptions, raw: &str, page: u32) -> Loaded {
        if let Some(hit) = self.entries.get(&(raw.to_string(), page)) {
            return hit.clone();
        }
        let prefixes: Vec<String> = std::iter::once(String::new()).chain(self.paths.iter().cloned()).collect();
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
                let candidates: Vec<String> = candidates.iter().flat_map(|c| prefixes.iter().map(move |p| format!("{p}{c}"))).collect();
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

/// `\tiny` .. `\Huge` as the compiler's size levels (`None` = `\normalsize`).
fn size_command(name: &str) -> Option<Option<flashtex_compiler::parser::FontSizeLevel>> {
    use flashtex_compiler::parser::FontSizeLevel as L;
    Some(Some(match name {
        "tiny" => L::Tiny,
        "scriptsize" => L::ScriptSize,
        "footnotesize" => L::FootnoteSize,
        "small" => L::Small,
        "large" => L::Large1,
        "Large" => L::Large2,
        "LARGE" => L::Large3,
        "huge" => L::Huge1,
        "Huge" => L::Huge2,
        "normalsize" => return Some(None),
        _ => return None,
    }))
}

/// The first source byte an item list was set from.
fn first_byte(items: &[AItem]) -> Option<usize> {
    items.iter().find_map(|i| match i {
        AItem::Word(w) => w.segments.iter().find_map(|s| s.chars.first()).map(|c| c.start),
        AItem::Math { span, .. } => Some(span.start),
        AItem::Table(t) => Some(t.span.start),
        _ => None,
    })
}

/// The material of a float body, or of a minipage in it (`range`,
/// `pieces`), as the main flow would set it: those bytes alone (preamble
/// kept), with `\caption`/`\includegraphics`/`minipage` replaced by `\par`
/// and `\label` and box glue by spaces (byte offsets preserved), parsed and
/// adapted. Returns each block's first source byte and part, in source
/// order: text paragraphs (`tabular`s included) as [`FloatPart::Text`];
/// lists, displays, headings, pictures and rules as [`FloatPart::Flow`].
#[allow(clippy::too_many_arguments)]
fn body_parts(
    range: (usize, usize),
    pieces: &[Piece],
    f: &FloatEnv,
    number: u32,
    d: usize,
    documents: &[SourceDocument<'_>],
    entry_index: usize,
    texts: &[&str],
    options: &RenderOptions,
    labels: &Labels,
    diags: &mut Vec<Diagnostic>,
) -> Vec<(usize, FloatPart)> {
    let text = documents[d].text;
    let mut isolated = isolate(text, Span::in_document(f.span.document, range.0, range.1)).into_bytes();
    for p in pieces {
        let (span, par) = match p {
            Piece::Caption { span, .. } | Piece::Graphic { span, .. } | Piece::Minipage { span, .. } => (*span, true),
            Piece::Label { span, .. } | Piece::HSkip { span, .. } => (*span, false),
            _ => continue,
        };
        for b in &mut isolated[span.start..span.end] {
            *b = b' ';
        }
        if par && span.end - span.start >= 4 {
            isolated[span.start..span.start + 4].copy_from_slice(b"\\par");
        }
    }
    let isolated = String::from_utf8(isolated).unwrap_or_default();
    let mut texts2: Vec<&str> = texts.to_vec();
    texts2[d] = &isolated;
    let docs2: Vec<SourceDocument<'_>> = documents.iter().zip(&texts2).map(|(doc, t)| SourceDocument { path: doc.path, text: t }).collect();
    let parsed = flashtex_compiler::parser::parse_project(&docs2, documents[d].path);
    let doc = adapter::adapt(&texts2, entry_index, &parsed, options, labels);
    let path: Rc<str> = Rc::from(documents[d].path);
    let inside = |s: &Span| s.document == f.span.document && s.start >= range.0 && s.start < range.1;
    // The compiler's reports about TikZ commands are superseded by the
    // picture reader's (as in the main flow).
    let pictures: Vec<(usize, usize)> = flashtex_vector_graphics::tikz::find_pictures(&isolated).into_iter().map(|p| (p.start, p.end)).collect();
    let reported = |s: &Span| inside(s) && !pictures.iter().any(|(a, b)| s.start >= *a && s.start < *b);
    let paths: Vec<&str> = documents.iter().map(|x| x.path).collect();
    for cd in &parsed.diagnostics {
        if cd.span.as_ref().is_some_and(reported) {
            diags.push(Diagnostic::from_compiler(cd, &paths));
        }
    }
    for (code, span, message) in &doc.limitations {
        if reported(span) {
            diags.push(Diagnostic::warning(code, message.clone(), vec![SourceRange { path: path.clone(), start_byte: span.start, end_byte: span.end }]));
        }
    }
    let mut out: Vec<(usize, usize, FloatPart)> = Vec::new();
    let unsupported = |what: &str, diags: &mut Vec<Diagnostic>| {
        diags.push(Diagnostic::warning(
            "float_content_unsupported",
            format!("{} {number}: {what} inside a float is not typeset yet; it is omitted", f.kind.name()),
            vec![SourceRange { path: path.clone(), start_byte: f.span.start, end_byte: f.span.end }],
        ));
    };
    let flow = |at: Option<usize>, end: Option<usize>, block: &adapter::Block, out: &mut Vec<(usize, usize, FloatPart)>| {
        if let Some(at) = at {
            out.push((at, end.unwrap_or(at).max(at), FloatPart::Flow(vec![block.clone()])));
        }
    };
    for block in &doc.blocks {
        match block {
            // A bare `algorithmic` is never inside a `figure`/`table` body:
            // `algorithms::scan` leaves those to this module, which reports
            // them, and `insert_bare` only inserts at document level.
            adapter::Block::Algorithmic(_) => {}
            adapter::Block::Paragraph { parts, style, env_open, env_close, vspace_before, addvspace_before, list: None, .. } if parts.iter().all(|p| matches!(p, ParaPart::Lines(_))) => {
                let n = parts.len();
                for (pi, part) in parts.iter().enumerate() {
                    let ParaPart::Lines(items) = part else { continue };
                    let Some(at) = first_byte(items) else { continue };
                    out.push((
                        at,
                        last_byte(items).unwrap_or(at),
                        FloatPart::Text {
                            items: items.clone(),
                            style: *style,
                            env_open: if pi == 0 { env_open.map(|e| e.vmode) } else { None },
                            env_close: *env_close && pi + 1 == n,
                            vspace_before: if pi == 0 { *vspace_before } else { 0.0 },
                            addvspace_before: if pi == 0 { *addvspace_before } else { 0.0 },
                        },
                    ));
                }
            }
            adapter::Block::Paragraph { parts, .. } => {
                let bounds = |p: &ParaPart| match p {
                    ParaPart::Lines(items) => first_byte(items).map(|a| (a, last_byte(items).unwrap_or(a))),
                    ParaPart::Display { span, .. } | ParaPart::Rows { span, .. } => Some((span.start, span.end)),
                };
                flow(parts.iter().find_map(bounds).map(|b| b.0), parts.iter().rev().find_map(bounds).map(|b| b.1), block, &mut out);
            }
            adapter::Block::Heading { span, .. } | adapter::Block::Rule { span, .. } => flow(Some(span.start), Some(span.end), block, &mut out),
            adapter::Block::Picture { picture, .. } => flow(Some(picture.start), Some(picture.end), block, &mut out),
            adapter::Block::Chapter { .. } => unsupported("a chapter heading", diags),
            // Like a chapter: a page-level heading or a contents line has no
            // meaning inside a float box.
            adapter::Block::Part { .. } => unsupported("a \\part heading", diags),
            adapter::Block::TocEntry(_) => unsupported("a table-of-contents entry", diags),
            adapter::Block::Chrome { .. } | adapter::Block::ClearPage { .. } => {}
            adapter::Block::Title { .. } => unsupported("a \\maketitle title", diags),
        }
    }
    // The adapter finds an environment's `\begin`/`\end` in the bytes around
    // a paragraph, which in the isolated body reach into the preamble
    // (`\begin{document}`); keep its skips only where the body itself opens
    // or closes a paragraph-shape environment there.
    const ENVS: [&str; 6] = ["center", "flushleft", "flushright", "quote", "quotation", "verse"];
    let has = |gap: &str, cmd: &str| ENVS.iter().any(|e| gap.contains(&format!("\\{cmd}{{{e}}}")));
    let spans: Vec<(usize, usize)> = out.iter().map(|(a, b, _)| (*a, *b)).collect();
    out.into_iter()
        .enumerate()
        .map(|(i, (at, end, mut part))| {
            let before = text.get(if i == 0 { range.0 } else { spans[i - 1].1.min(at) }..at).unwrap_or("");
            let after = text.get(end..spans.get(i + 1).map_or(range.1, |s| s.0.max(end))).unwrap_or("");
            let (opens, closes) = (has(before, "begin"), has(after, "end"));
            match &mut part {
                FloatPart::Text { env_open, env_close, .. } => {
                    if !opens {
                        *env_open = None;
                    }
                    if !closes {
                        *env_close = false;
                    }
                }
                FloatPart::Flow(blocks) => {
                    for b in blocks {
                        if let adapter::Block::Paragraph { env_open, env_close, .. } = b {
                            if !opens {
                                *env_open = None;
                            }
                            if !closes {
                                *env_close = false;
                            }
                        }
                    }
                }
                _ => {}
            }
            (at, part)
        })
        .collect()
}

/// The source byte just past the last character an item list was set from.
fn last_byte(items: &[AItem]) -> Option<usize> {
    items
        .iter()
        .filter_map(|i| match i {
            AItem::Word(w) => w.segments.iter().filter_map(|s| s.chars.last()).map(|c| c.end).max(),
            AItem::Math { span, .. } => Some(span.end),
            AItem::Table(t) => Some(t.span.end),
            _ => None,
        })
        .max()
}

/// `\usepackage[..,demo,..]{graphicx}` (or `graphics`) in the preamble:
/// `\includegraphics` draws a black `\rule` of the requested size and reads
/// no file.
pub fn graphics_demo(text: &str) -> bool {
    let preamble = text.find("\\begin{document}").map_or(text, |e| &text[..e]);
    let mut from = 0;
    while let Some(at) = preamble[from..].find("\\usepackage") {
        let pos = from + at;
        from = pos + 1;
        let line_start = preamble[..pos].rfind('\n').map_or(0, |i| i + 1);
        if preamble[line_start..pos].contains('%') {
            continue;
        }
        let rest = preamble[pos + "\\usepackage".len()..].trim_start();
        let Some(o) = rest.strip_prefix('[') else { continue };
        let Some(close) = o.find(']') else { continue };
        let (opts, after) = (&o[..close], o[close + 1..].trim_start());
        let Some(names) = after.strip_prefix('{').and_then(|a| a.find('}').map(|c| &a[..c])) else { continue };
        if names.split(',').any(|n| matches!(n.trim(), "graphicx" | "graphics")) && opts.split(',').any(|o| o.trim() == "demo") {
            return true;
        }
    }
    false
}

/// Walks a float body, and every `minipage` in it, into layout parts.
struct Prep<'x> {
    f: &'x FloatEnv,
    d: usize,
    documents: &'x [SourceDocument<'x>],
    entry_index: usize,
    texts: &'x [&'x str],
    options: &'x RenderOptions,
    labels: &'x Labels,
    images: &'x mut ImageCache,
    diags: &'x mut Vec<Diagnostic>,
    demo: bool,
    body_size: f64,
    text_height: f64,
    paper: (f64, f64),
    em: f64,
    ex: f64,
    /// The float's number (its first caption's) and the next caption's.
    number: u32,
    next_caption: u32,
    labels_out: Vec<String>,
}

impl Prep<'_> {
    fn env(&self, text_width: f64, line_width: f64) -> LengthEnv {
        LengthEnv { text_width, line_width, text_height: self.text_height, paper_width: self.paper.0, paper_height: self.paper.1, em: self.em, ex: self.ex }
    }

    /// The parts of `range` (split into `pieces`) set in a box whose
    /// `\linewidth` is `hsize`, where `\textwidth` is `textwidth`.
    fn parts(&mut self, range: (usize, usize), pieces: &[Piece], hsize: f64, textwidth: f64) -> Vec<FloatPart> {
        let env = self.env(textwidth, hsize);
        let d = self.d;
        let source = self.documents[d].text;
        let path: Rc<str> = Rc::from(self.documents[d].path);
        let src = |span: Span| SourceRange { path: path.clone(), start_byte: span.start, end_byte: span.end };
        let mut parts = Vec::new();
        // Paragraphs, lists, displays, ... of the body, merged with the
        // pieces below in source order.
        let body = body_parts(range, pieces, self.f, self.number, d, self.documents, self.entry_index, self.texts, self.options, self.labels, &mut *self.diags);
        let mut text = body.into_iter().peekable();
        let class_size = adapter::class_size_of(self.body_size);
        for piece in pieces {
            let at = match piece {
                Piece::Graphic { span, .. } | Piece::Caption { span, .. } | Piece::Label { span, .. } | Piece::Other { span } | Piece::Minipage { span, .. } | Piece::HSkip { span, .. } => Some(span.start),
                Piece::Centering | Piece::ParBreak | Piece::Space => None,
            };
            if let Some(at) = at {
                while let Some((_, part)) = text.next_if(|(b, _)| *b < at) {
                    parts.push(part);
                }
            }
            match piece {
                Piece::Centering => parts.push(FloatPart::Centering),
                Piece::ParBreak => parts.push(FloatPart::ParBreak),
                Piece::Space => parts.push(FloatPart::Space),
                Piece::Label { key, .. } => self.labels_out.push(key.clone()),
                Piece::Other { span } => {
                    // A size declaration at the box's top level sets the
                    // `\baselineskip` of what follows (the paragraphs' own
                    // words carry their size from the compiler).
                    if let Some(level) = source[span.start..span.end].strip_prefix('\\').and_then(size_command) {
                        parts.push(FloatPart::Size(adapter::declared_size(level, class_size)));
                    }
                }
                Piece::HSkip { span, glue } => {
                    let (width, order) = match glue {
                        HGlue::Infinite(o) => (0.0, *o),
                        HGlue::Em(e) => (e * env.em, 0),
                        HGlue::Dimen(raw) => match graphics::parse_dimen(raw, &env) {
                            Some(w) => (w, 0),
                            None => {
                                self.diags.push(Diagnostic::warning("float_content_unsupported", format!("\\hspace{{{raw}}}: the length is not understood; no space is set"), vec![src(*span)]));
                                (0.0, 0)
                            }
                        },
                    };
                    parts.push(FloatPart::HSkip { width, order });
                }
                Piece::Minipage { span, pos, width, body, pieces: inner } => {
                    let w = match graphics::parse_dimen(width, &env) {
                        Some(w) => w,
                        None => {
                            self.diags.push(Diagnostic::warning("float_content_unsupported", format!("minipage width `{width}` is not understood; \\linewidth is used"), vec![src(*span)]));
                            hsize
                        }
                    };
                    // `\@iiiminipage`: `\hsize`, `\textwidth` and `\columnwidth`
                    // are the box width inside it.
                    let inner_parts = self.parts(*body, inner, w, w);
                    parts.push(FloatPart::Minipage { pos: *pos, width: w, parts: inner_parts, span: *span });
                }
                Piece::Graphic { span, options: opts, path: file } => {
                    let (keys, problems) = graphics::parse_keys(opts, &env);
                    for p in problems {
                        self.diags.push(Diagnostic::warning("graphics_option", p, vec![src(*span)]));
                    }
                    for k in &keys {
                        if let GKey::Unsupported(name) = k {
                            self.diags.push(Diagnostic::warning("graphics_option", format!("\\includegraphics key '{name}' is not honoured yet"), vec![src(*span)]));
                        }
                    }
                    if self.demo {
                        // graphicx `demo`: `\rule{\Gin@@ewidth}{\Gin@@eheight}`,
                        // 150pt by 100pt unless requested; no file is read.
                        let w = keys.iter().rev().find_map(|k| if let GKey::Width(v) = k { Some(*v) } else { None }).unwrap_or(150.0);
                        let h = keys.iter().rev().find_map(|k| if let GKey::Height(v) | GKey::TotalHeight(v) = k { Some(*v) } else { None }).unwrap_or(100.0);
                        let gbox = graphics::GraphicBox { width: w, height: h, depth: 0.0, matrix: [w, 0.0, 0.0, h, 0.0, 0.0] };
                        parts.push(FloatPart::Graphic(PreparedGraphic { gbox, resource: None, clip: None, span: *span, demo: true }));
                        continue;
                    }
                    let page = keys.iter().find_map(|k| if let GKey::Page(p) = k { Some(*p) } else { None }).unwrap_or(1);
                    match self.images.load(self.options, file, page) {
                        Ok((resource, info)) => {
                            // pdftex.def `\Ginclude@@pdftex`: `trim`/`viewport`
                            // with `clip` show only part of the image.
                            let placed = graphics::place_image(info.width_bp / graphics::BP_PER_PT, info.height_bp / graphics::BP_PER_PT, &keys, false, false, &env);
                            parts.push(FloatPart::Graphic(PreparedGraphic { gbox: placed.gbox, resource: Some(resource), clip: placed.clip, span: *span, demo: false }));
                        }
                        Err(msg) => {
                            let w = keys.iter().rev().find_map(|k| if let GKey::Width(v) = k { Some(*v) } else { None });
                            let h = keys.iter().rev().find_map(|k| if let GKey::Height(v) | GKey::TotalHeight(v) = k { Some(*v) } else { None });
                            match (w, h) {
                                (Some(w), Some(h)) => {
                                    self.diags.push(Diagnostic::error("image_unavailable", format!("{msg} (its requested size is kept empty)"), vec![src(*span)]));
                                    let gbox = graphics::GraphicBox { width: w, height: h, depth: 0.0, matrix: [w, 0.0, 0.0, h, 0.0, 0.0] };
                                    parts.push(FloatPart::Graphic(PreparedGraphic { gbox, resource: None, clip: None, span: *span, demo: false }));
                                }
                                _ => self.diags.push(Diagnostic::error("image_unavailable", msg, vec![src(*span)])),
                            }
                        }
                    }
                }
                // `short` (the `\caption[..]` optional argument) is written to
                // the list of figures/tables, not to the float body set here.
                Piece::Caption { span, arg, .. } => {
                    let items = caption_items(self.f.kind, self.next_caption, *span, *arg, d, self.documents, self.entry_index, self.texts, self.options, self.labels);
                    self.next_caption += 1;
                    parts.push(FloatPart::Caption { items });
                }
            }
        }
        parts.extend(text.map(|(_, part)| part));
        parts
    }
}

/// Builds the layout input of every float. `texts` are the masked texts
/// the main parse ran on.
#[allow(clippy::too_many_arguments)]
pub fn prepare(
    envs: &[Vec<FloatEnv>],
    numbers: &[Vec<u32>],
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
    let demo = documents.get(entry_index).is_some_and(|d| graphics_demo(d.text));
    // `\textwidth`: the text block (a two-column document's `\columnwidth`
    // is narrower).
    let textwidth = style.class_geometry.as_deref().map_or(style.text_width_pt, |g| crate::style::frame_pt(g.frame.text_width));
    for (d, doc_envs) in envs.iter().enumerate() {
        let path: Rc<str> = Rc::from(documents[d].path);
        for (fi, f) in doc_envs.iter().enumerate() {
            let number = numbers[d][fi];
            let bits = match placement_bits(f.placement.as_deref()) {
                Ok(b) => b,
                Err(msg) => {
                    let fallback = if msg.starts_with("placement H") { 16 | 1 } else { 16 | 8 };
                    diags.push(Diagnostic::warning("float_placement", msg, vec![SourceRange { path: path.clone(), start_byte: f.span.start, end_byte: f.span.end }]));
                    fallback
                }
            };
            // `\@xdblfloat`: `\hsize\textwidth`.
            let hsize = if f.wide { textwidth } else { style.text_width_pt };
            let mut prep = Prep {
                f,
                d,
                documents,
                entry_index,
                texts,
                options,
                labels,
                images: &mut *images,
                diags: &mut diags,
                demo,
                body_size: style.body_size_pt,
                text_height: style.text_height_pt,
                paper: (style.page_width_pt, style.page_height_pt),
                em,
                ex,
                number,
                next_caption: number,
                labels_out: Vec::new(),
            };
            let parts = prep.parts(f.body, &f.pieces, hsize, textwidth);
            let spec_labels = std::mem::take(&mut prep.labels_out);
            specs.push(FloatSpec { kind: f.kind, number, bits, span: f.span, hmode: f.hmode, wide: f.wide, parts, labels: spec_labels });
        }
    }
    (specs, diags)
}

#[allow(clippy::too_many_arguments)]
fn caption_items(
    kind: FloatKind,
    number: u32,
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
    fn graphicx_demo_option() {
        assert!(graphics_demo("\\documentclass{article}\n\\usepackage[demo]{graphicx}\n\\begin{document}\n"));
        assert!(graphics_demo("\\usepackage[draft, demo]{graphics,xcolor}\n\\begin{document}"));
        assert!(!graphics_demo("% \\usepackage[demo]{graphicx}\n\\usepackage{graphicx}\n\\begin{document}\\usepackage[demo]{graphicx}"));
    }

    #[test]
    fn star_floats_are_wide_and_body_excludes_the_placement() {
        let src = "\\begin{document}\n\\begin{table*}[t]\\caption{C}x\\end{table*}\n";
        let f = &scan(src, DocumentId(0))[0];
        assert!(f.wide);
        assert_eq!(&src[f.body.0..f.body.1], "\\caption{C}x");
    }

    #[test]
    fn scans_pieces_and_masks_in_place() {
        let src = "\\begin{document}\nText before.\n\n\\begin{figure}[ht]\n  \\centering\n  \\includegraphics[width=2in]{a.png}\n  \\caption{A {nested} cap.}\\label{fig:a}\n\\end{figure}\n\nAfter.\n\\end{document}\n";
        let f = scan(src, DocumentId(0));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].placement.as_deref(), Some("ht"));
        assert!(!f[0].hmode);
        // The end of the line after `\includegraphics{...}` is a space
        // (dropped again where the paragraph ends).
        assert!(matches!(f[0].pieces[2], Piece::Space));
        let pieces: Vec<&Piece> = f[0].pieces.iter().filter(|p| !matches!(p, Piece::Space)).collect();
        assert!(matches!(pieces[0], Piece::Centering));
        match pieces[1] {
            Piece::Graphic { options, path, .. } => assert_eq!((options.as_str(), path.as_str()), ("width=2in", "a.png")),
            other => panic!("{other:?}"),
        }
        match pieces[2] {
            Piece::Caption { arg, .. } => assert_eq!(&src[arg.start..arg.end], "A {nested} cap."),
            other => panic!("{other:?}"),
        }
        assert!(matches!(pieces[3], Piece::Label { key, .. } if key == "fig:a"));
        let masked = mask(src, &f);
        assert_eq!(masked.len(), src.len());
        assert!(!masked.contains("figure") && masked.contains("After."));
        assert_eq!(placement_bits(Some("ht")), Ok(16 | 1 | 2));
        // `\@fpsadddefault`: a bare `!` becomes `!tbp`, and `!` clears 16.
        assert_eq!(placement_bits(Some("!")), Ok(2 | 4 | 8));
        assert_eq!(placement_bits(Some("!h")), Ok(1));
    }

    #[test]
    fn a_float_inside_a_paragraph_is_horizontal_mode() {
        let src = "Some text\n\\begin{figure}\\caption{x}\\end{figure} more.";
        assert!(scan(src, DocumentId(0))[0].hmode);
    }
}
