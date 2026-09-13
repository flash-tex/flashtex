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
            }
            b'\n' => {
                let j = skip_ws(text, i, end);
                if text[i..j].matches('\n').count() >= 2 {
                    out.push(Piece::ParBreak);
                }
                i = j.max(i + 1);
            }
            b' ' | b'\t' | b'\r' => i += 1,
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
    out
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

/// `(float numbers per document in scan order, label key -> value)`.
pub fn number(envs: &[Vec<FloatEnv>]) -> (Vec<Vec<u32>>, Vec<(String, String)>) {
    let (mut figures, mut tables) = (0u32, 0u32);
    let mut numbers = Vec::new();
    let mut labels = Vec::new();
    for doc in envs {
        let mut nums = Vec::new();
        for f in doc {
            let has_caption = f.pieces.iter().any(|p| matches!(p, Piece::Caption { .. }));
            let counter = match f.kind {
                FloatKind::Figure => &mut figures,
                FloatKind::Table => &mut tables,
            };
            if has_caption {
                *counter += 1;
            }
            nums.push(*counter);
            // `\label` after `\caption` takes its number (`\@currentlabel`).
            let mut seen_caption = false;
            for p in &f.pieces {
                match p {
                    Piece::Caption { .. } => seen_caption = true,
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
    let env = LengthEnv { text_width: style.text_width_pt, text_height: style.text_height_pt, paper_width: style.page_width_pt, paper_height: style.page_height_pt, em, ex };
    for (d, doc_envs) in envs.iter().enumerate() {
        let path: Rc<str> = Rc::from(documents[d].path);
        let src = |span: Span| SourceRange { path: path.clone(), start_byte: span.start, end_byte: span.end };
        for (fi, f) in doc_envs.iter().enumerate() {
            let number = numbers[d][fi];
            let bits = match placement_bits(f.placement.as_deref()) {
                Ok(b) => b,
                Err(msg) => {
                    let fallback = if msg.starts_with("placement H") { 16 | 1 } else { 16 | 8 };
                    diags.push(Diagnostic::warning("float_placement", msg, vec![src(f.span)]));
                    fallback
                }
            };
            let mut parts = Vec::new();
            let mut spec_labels = Vec::new();
            let mut reported_other = false;
            for piece in &f.pieces {
                match piece {
                    Piece::Centering => parts.push(FloatPart::Centering),
                    Piece::ParBreak => parts.push(FloatPart::ParBreak),
                    Piece::Label { key, .. } => spec_labels.push(key.clone()),
                    Piece::Other { span } => {
                        if !reported_other {
                            reported_other = true;
                            diags.push(Diagnostic::warning(
                                "float_content_unsupported",
                                format!("{} {number}: only \\includegraphics, \\caption, \\label and \\centering are typeset inside a float so far; this material is omitted", f.kind.name()),
                                vec![src(*span)],
                            ));
                        }
                    }
                    Piece::Graphic { span, options: opts, path: file } => {
                        let (keys, problems) = graphics::parse_keys(opts, &env);
                        for p in problems {
                            diags.push(Diagnostic::warning("graphics_option", p, vec![src(*span)]));
                        }
                        for k in &keys {
                            if let GKey::Unsupported(name) = k {
                                diags.push(Diagnostic::warning("graphics_option", format!("\\includegraphics key '{name}' is not honoured yet"), vec![src(*span)]));
                            }
                        }
                        let page = keys.iter().find_map(|k| if let GKey::Page(p) = k { Some(*p) } else { None }).unwrap_or(1);
                        match images.load(options, file, page) {
                            Ok((resource, info)) => {
                                let gbox = graphics::size_box(info.width_bp / graphics::BP_PER_PT, info.height_bp / graphics::BP_PER_PT, &keys);
                                parts.push(FloatPart::Graphic(PreparedGraphic { gbox, resource: Some(resource), span: *span }));
                            }
                            Err(msg) => {
                                let w = keys.iter().rev().find_map(|k| if let GKey::Width(v) = k { Some(*v) } else { None });
                                let h = keys.iter().rev().find_map(|k| if let GKey::Height(v) | GKey::TotalHeight(v) = k { Some(*v) } else { None });
                                match (w, h) {
                                    (Some(w), Some(h)) => {
                                        diags.push(Diagnostic::error("image_unavailable", format!("{msg} (its requested size is kept empty)"), vec![src(*span)]));
                                        let gbox = graphics::GraphicBox { width: w, height: h, depth: 0.0, matrix: [w, 0.0, 0.0, h, 0.0, 0.0] };
                                        parts.push(FloatPart::Graphic(PreparedGraphic { gbox, resource: None, span: *span }));
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
            specs.push(FloatSpec { kind: f.kind, number, bits, span: f.span, hmode: f.hmode, parts, labels: spec_labels });
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
    fn scans_pieces_and_masks_in_place() {
        let src = "\\begin{document}\nText before.\n\n\\begin{figure}[ht]\n  \\centering\n  \\includegraphics[width=2in]{a.png}\n  \\caption{A {nested} cap.}\\label{fig:a}\n\\end{figure}\n\nAfter.\n\\end{document}\n";
        let f = scan(src, DocumentId(0));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].placement.as_deref(), Some("ht"));
        assert!(!f[0].hmode);
        assert!(matches!(f[0].pieces[0], Piece::Centering));
        match &f[0].pieces[1] {
            Piece::Graphic { options, path, .. } => assert_eq!((options.as_str(), path.as_str()), ("width=2in", "a.png")),
            other => panic!("{other:?}"),
        }
        match &f[0].pieces[2] {
            Piece::Caption { arg, .. } => assert_eq!(&src[arg.start..arg.end], "A {nested} cap."),
            other => panic!("{other:?}"),
        }
        assert!(matches!(&f[0].pieces[3], Piece::Label { key, .. } if key == "fig:a"));
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
