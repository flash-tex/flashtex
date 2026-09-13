//! Pseudocode: `algorithm` floats (algorithm.sty on float.sty) and
//! `algorithmic` environments (algorithmic.sty; algpseudocode.sty on
//! algorithmicx.sty), laid out from the compiler's source model
//! (`flashtex_compiler::algorithmic`).
//!
//! Like [`crate::floats`], every environment is found in the exact source
//! bytes and blanked (same byte length) before the compiler parses the
//! document. An `algorithm` float becomes a [`FloatSpec`] placed by
//! `typeset::floatpage`: float.sty's `ruled` box (`\@fs@pre`, the caption,
//! `\@fs@mid`, the body, `\@fs@post`; lines 151-156) or `plain` (body, then
//! `\vspace\abovecaptionskip` and the caption; lines 140-146). A bare
//! `algorithmic` becomes list lines in the text flow
//! ([`adapter::Block::Algorithmic`]). Author material inside statements
//! (conditions, statement text, comments, the caption) is parsed once per
//! environment from a copy of the document blanked except the preamble and
//! those spans, and the items are split back by source position.

use std::collections::HashMap;
use std::rc::Rc;

use flashtex_compiler::algorithmic::{self as alg, Face, FloatItem, FloatStyle, Found, Line, LineKind, Piece};
use flashtex_compiler::parser::SourceDocument;
use flashtex_compiler::{DocumentId, Span};

use crate::adapter::{self, CharSrc, Item as AItem, Labels, ParaPart, Segment, TextStyle, Word};
use crate::display::{Diagnostic, SourceRange};
use crate::floats::{placement_bits, FloatEnv, FloatKind};
use crate::style::Stylesheet;
use crate::typeset::algorithms::{AlgLine, BareAlgorithm, LineGeometry};
use crate::typeset::floatpage::{FloatPart, FloatSpec};
use crate::RenderOptions;

/// float.sty `\@fs@mid` for `plain`: `\vspace\abovecaptionskip` (10pt in
/// size10/11/12.clo).
const ABOVECAPTIONSKIP_PT: f64 = 10.0;

pub struct Scan {
    pub setup: alg::Setup,
    /// Per document, in source order.
    pub found: Vec<Vec<Found>>,
}

impl Scan {
    pub fn is_empty(&self) -> bool {
        self.found.iter().all(Vec::is_empty)
    }
}

/// Every pseudocode float and bare environment. The preamble of the entry
/// document decides the packages; an `algorithmic` inside a `figure` or
/// `table` is left to [`crate::floats`] (reported there).
pub fn scan(documents: &[SourceDocument<'_>], entry_index: usize, figures: &[Vec<FloatEnv>]) -> Scan {
    let setup = documents.get(entry_index).map(|d| alg::setup(d.text)).unwrap_or_default();
    let found = documents
        .iter()
        .enumerate()
        .map(|(i, d)| {
            if setup.dialect.is_none() && !setup.float_loaded {
                return Vec::new();
            }
            alg::scan(d.text, DocumentId(i), &setup)
                .into_iter()
                .filter(|f| {
                    let s = f.span();
                    !figures.get(i).is_some_and(|envs| envs.iter().any(|e| e.span.start <= s.start && s.end <= e.span.end))
                })
                .collect()
        })
        .collect();
    Scan { setup, found }
}

/// `text` with every environment of `found` blanked. A bare `algorithmic`
/// ends the paragraph it interrupts (`\@trivlist`'s `\par`), and text that
/// follows `\end{algorithmic}` without a blank line starts a paragraph
/// without indentation (`\@endpetrue`): the blanked bytes carry a blank line
/// and, when needed, `\noindent`.
pub fn mask(text: &str, found: &[Found]) -> String {
    let mut bytes = text.as_bytes().to_vec();
    for f in found {
        let s = f.span();
        for b in &mut bytes[s.start..s.end] {
            *b = b' ';
        }
        if let Found::Bare(_) = f {
            bytes[s.start] = b'\n';
            bytes[s.start + 1] = b'\n';
            let after = &text[s.end..];
            let rest = after.trim_start();
            let gap = &after[..after.len() - rest.len()];
            let blank_line = gap.matches('\n').count() >= 2;
            if !blank_line && !rest.is_empty() && !rest.starts_with("\\end{document}") && !rest.starts_with('%') {
                let tag = b"\\noindent";
                let at = s.end - tag.len() - 1;
                bytes[at..at + tag.len()].copy_from_slice(tag);
            }
        }
    }
    String::from_utf8(bytes).expect("ASCII keeps UTF-8 valid")
}

/// `(float number per found item, label key -> value)`. `\caption` steps
/// the float's counter (float.sty line 110, `\refstepcounter\@captype`); a
/// `\label` after it takes that number.
pub fn number(scan: &Scan) -> (Vec<Vec<u32>>, Vec<(String, String)>) {
    let mut n = 0;
    let mut labels = Vec::new();
    let numbers = scan
        .found
        .iter()
        .map(|doc| {
            doc.iter()
                .map(|f| {
                    let Found::Float(fl) = f else { return 0 };
                    let mut captioned = false;
                    for item in &fl.body {
                        match item {
                            FloatItem::Caption { .. } => {
                                n += 1;
                                captioned = true;
                            }
                            FloatItem::Label { key, .. } if captioned => labels.push((key.clone(), n.to_string())),
                            _ => {}
                        }
                    }
                    n
                })
                .collect()
        })
        .collect();
    (numbers, labels)
}

/// The layout input of every pseudocode float, and the bare environments
/// with the document they belong to.
#[allow(clippy::too_many_arguments)]
pub fn prepare(
    scan: &Scan,
    numbers: &[Vec<u32>],
    documents: &[SourceDocument<'_>],
    entry_index: usize,
    texts: &[&str],
    style: &Stylesheet,
    options: &RenderOptions,
    labels: &Labels,
) -> (Vec<FloatSpec>, Vec<Rc<BareAlgorithm>>, Vec<Diagnostic>) {
    let setup = &scan.setup;
    let class_size = adapter::class_size_of(style.body_size_pt);
    let mut specs = Vec::new();
    let mut bare = Vec::new();
    let mut diags = Vec::new();
    for (d, doc_found) in scan.found.iter().enumerate() {
        let path: Rc<str> = Rc::from(documents[d].path);
        let src = |span: Span| SourceRange { path: path.clone(), start_byte: span.start, end_byte: span.end };
        for (fi, f) in doc_found.iter().enumerate() {
            let mut spans = Vec::new();
            let algorithmics: Vec<&alg::Algorithmic> = match f {
                Found::Float(fl) => fl
                    .body
                    .iter()
                    .filter_map(|i| match i {
                        FloatItem::Caption { arg, .. } => {
                            spans.push(*arg);
                            None
                        }
                        FloatItem::Algorithmic(a) => Some(a),
                        _ => None,
                    })
                    .collect(),
                Found::Bare(a) => vec![a],
            };
            for a in &algorithmics {
                for (span, message) in &a.problems {
                    diags.push(Diagnostic::warning("algorithmic_construct", message.clone(), vec![src(*span)]));
                }
                for line in &a.lines {
                    collect_spans(&line.pieces, &mut spans);
                    if let LineKind::Labelled(label) = &line.kind {
                        collect_spans(label, &mut spans);
                    }
                }
            }
            let items = source_items(d, &spans, documents, entry_index, texts, options, labels);
            let by_span: HashMap<(usize, usize), &Vec<AItem>> = spans.iter().zip(&items).map(|(s, i)| ((s.start, s.end), i)).collect();
            let mut conv = Converter { by_span: &by_span };
            let lines_of = |a: &alg::Algorithmic, conv: &mut Converter| -> Vec<AlgLine> {
                a.lines.iter().map(|line| alg_line(a, line, setup, class_size, conv)).collect()
            };
            match f {
                Found::Bare(a) => {
                    let lines = lines_of(a, &mut conv);
                    bare.push(Rc::new(BareAlgorithm { lines, span: a.span }));
                }
                Found::Float(fl) => {
                    let number = numbers[d][fi];
                    let mut body = Vec::new();
                    let mut caption = None;
                    let mut spec_labels = Vec::new();
                    let mut reported_other = false;
                    for item in &fl.body {
                        match item {
                            FloatItem::Caption { span, arg } => {
                                caption = Some(caption_items(setup, number, *span, *arg, &by_span));
                            }
                            FloatItem::Label { key, .. } => spec_labels.push(key.clone()),
                            FloatItem::Algorithmic(a) => {
                                body.extend(lines_of(a, &mut conv).into_iter().map(|l| FloatPart::AlgLine(Box::new(l))));
                                // `\@endparenv`: `\addvspace\@topsepadd` after the list.
                                body.push(FloatPart::Kern { pt: 0.0, em: alg::TOPSEP_EM });
                            }
                            FloatItem::Declaration { .. } => {}
                            FloatItem::Size { span, .. } => diags.push(Diagnostic::warning(
                                "algorithm_size",
                                "a size declaration inside an algorithm float is not applied yet; its lines are set at the body size",
                                vec![src(*span)],
                            )),
                            FloatItem::Other { span } => {
                                if !reported_other {
                                    reported_other = true;
                                    diags.push(Diagnostic::warning(
                                        "float_content_unsupported",
                                        format!("{} {number}: only \\caption, \\label and algorithmic are typeset inside an algorithm float so far; this material is omitted", setup.float_name),
                                        vec![src(*span)],
                                    ));
                                }
                            }
                        }
                    }
                    let mut parts = Vec::new();
                    let span = fl.span;
                    match setup.float_style {
                        FloatStyle::Ruled | FloatStyle::Boxed => {
                            if setup.float_style == FloatStyle::Boxed {
                                diags.push(Diagnostic::warning("algorithm_style", "the boxed algorithm style is drawn as ruled", vec![src(span)]));
                            }
                            // float.sty 153-155.
                            parts.push(FloatPart::Rule { height: alg::RULED_TOP_RULE_PT, span });
                            parts.push(FloatPart::Kern { pt: alg::RULED_KERN_PT, em: 0.0 });
                            if let Some(items) = caption {
                                parts.push(FloatPart::StyleCaption { items, center_if_fits: false });
                                parts.push(FloatPart::Kern { pt: alg::RULED_KERN_PT, em: 0.0 });
                                parts.push(FloatPart::Rule { height: alg::RULED_RULE_PT, span });
                                parts.push(FloatPart::Kern { pt: alg::RULED_KERN_PT, em: 0.0 });
                            }
                            parts.extend(body);
                            parts.push(FloatPart::Kern { pt: alg::RULED_KERN_PT, em: 0.0 });
                            parts.push(FloatPart::Rule { height: alg::RULED_RULE_PT, span });
                        }
                        FloatStyle::Plain => {
                            parts.extend(body);
                            if let Some(items) = caption {
                                parts.push(FloatPart::Kern { pt: ABOVECAPTIONSKIP_PT, em: 0.0 });
                                parts.push(FloatPart::StyleCaption { items, center_if_fits: true });
                            }
                        }
                    }
                    if setup.within.is_some() {
                        diags.push(Diagnostic::warning(
                            "algorithm_numbering",
                            "algorithm numbers reset within sectioning units are not implemented; numbered consecutively",
                            vec![src(span)],
                        ));
                    }
                    let placement = fl.placement.clone().unwrap_or_else(|| "htbp".to_string());
                    let bits = match placement_bits(Some(&placement)) {
                        Ok(b) => b,
                        Err(msg) => {
                            let fallback = if msg.starts_with("placement H") { 16 | 1 } else { 16 | 8 };
                            diags.push(Diagnostic::warning("float_placement", msg, vec![src(span)]));
                            fallback
                        }
                    };
                    specs.push(FloatSpec { kind: FloatKind::Algorithm, number, bits, span, hmode: fl.hmode, wide: fl.starred, parts, labels: spec_labels });
                }
            }
        }
    }
    (specs, bare, diags)
}

/// Puts every bare environment into the adapted document before the first
/// block that starts after it in the same source document.
pub fn insert_bare(doc: &mut adapter::Doc, bare: &[Rc<BareAlgorithm>]) {
    for b in bare {
        let at = doc
            .blocks
            .iter()
            .position(|block| block_start(block).is_some_and(|(document, start)| document == b.span.document && start > b.span.start))
            .unwrap_or(doc.blocks.len());
        doc.blocks.insert(at, adapter::Block::Algorithmic(b.clone()));
    }
}

fn block_start(block: &adapter::Block) -> Option<(DocumentId, usize)> {
    match block {
        adapter::Block::Paragraph { parts, .. } => parts.iter().find_map(|p| match p {
            ParaPart::Lines(items) => crate::incremental::block_origin(items),
            _ => None,
        }),
        adapter::Block::Heading { span, .. } | adapter::Block::Chapter { span, .. } | adapter::Block::Title { span, .. } => Some((span.document, span.start)),
        adapter::Block::ClearPage { span, .. } | adapter::Block::Chrome { span, .. } | adapter::Block::Rule { span, .. } => Some((span.document, span.start)),
        adapter::Block::Algorithmic(a) => Some((a.span.document, a.span.start)),
        // Blocks without a source origin of their own are skipped.
        _ => None,
    }
}

fn collect_spans(pieces: &[Piece], out: &mut Vec<Span>) {
    for p in pieces {
        match p {
            Piece::Source(span) | Piece::StyledSource { span, .. } => out.push(*span),
            _ => {}
        }
    }
}

struct Converter<'a> {
    by_span: &'a HashMap<(usize, usize), &'a Vec<AItem>>,
}

fn style_of(face: Face) -> TextStyle {
    TextStyle { bold: face == Face::Bold, caps: face == Face::SmallCaps, ..TextStyle::default() }
}

fn word(text: &str, span: Span, style: TextStyle) -> AItem {
    let origin = CharSrc { document: span.document, start: span.start, end: span.end };
    AItem::Word(Word { segments: vec![Segment { text: text.to_string(), chars: text.chars().map(|_| origin).collect(), style }] })
}

/// `\(\triangleright\)` and friends as a one-atom math list at the command.
fn math_symbol(tex: &str, span: Span) -> Option<flashtex_compiler::math::MathList> {
    let tokens = flashtex_compiler::lexer::tokenize_document(tex, span.document);
    let mut diagnostics = Vec::new();
    let list = flashtex_compiler::math::parse_tokens(&tokens, &mut diagnostics);
    diagnostics.is_empty().then(|| flashtex_compiler::math::shift_list(&list, span.start as isize))
}

impl Converter<'_> {
    fn items(&mut self, pieces: &[Piece]) -> Vec<AItem> {
        let mut out = Vec::new();
        for (index, p) in pieces.iter().enumerate() {
            match p {
                Piece::Text { text, face, span } => {
                    out.push(word(text, *span, style_of(*face)));
                    // `\textbf{..}` ends with `\check@icr`: the italic
                    // correction of its last letter (latex.ltx
                    // `\DeclareTextFontCommand`), unless the group goes on.
                    let group_continues = matches!(pieces.get(index + 1), Some(Piece::Space { face: next } | Piece::Text { face: next, .. }) if next == face);
                    if *face != Face::Roman && !group_continues {
                        out.push(AItem::ItalicCorrection);
                    }
                }
                Piece::Source(span) => {
                    if let Some(items) = self.by_span.get(&(span.start, span.end)) {
                        out.extend(items.iter().cloned());
                    }
                }
                Piece::StyledSource { face, span } => {
                    // `\textproc{..}` (`\textsc`, algpseudocode.sty 34) and
                    // `\Call`'s name: the words in small caps.
                    if let Some(items) = self.by_span.get(&(span.start, span.end)) {
                        out.extend(items.iter().cloned().map(|mut item| {
                            if let AItem::Word(w) = &mut item {
                                for s in &mut w.segments {
                                    s.style.bold |= *face == Face::Bold;
                                    s.style.caps |= *face == Face::SmallCaps;
                                }
                            }
                            item
                        }));
                    }
                }
                Piece::Space { face } => {
                    let factor = match out.last() {
                        Some(AItem::Word(w)) => w.text().chars().last().map_or(1000, |c| adapter::space_factor(c, 1000)),
                        _ => 1000,
                    };
                    out.push(AItem::Space { style: style_of(*face), factor, no_break: false });
                }
                Piece::HFill { .. } => out.push(AItem::HFill { fill: true }),
                Piece::MathSymbol { tex, span } => {
                    if let Some(list) = math_symbol(tex, *span) {
                        out.push(AItem::Math { list, span: *span });
                    }
                }
            }
        }
        out
    }
}

fn alg_line(a: &alg::Algorithmic, line: &Line, setup: &alg::Setup, class_size: u32, conv: &mut Converter) -> AlgLine {
    let label = match &line.kind {
        LineKind::Numbered if line.show_number => {
            let n = &setup.line_numbers;
            let style = TextStyle { bold: n.bold, size_cpt: adapter::declared_size(n.size, class_size), ..TextStyle::default() };
            vec![word(&format!("{}{}{}", n.prefix, line.number, n.suffix), line.span, style)]
        }
        LineKind::Labelled(pieces) => conv.items(pieces),
        _ => Vec::new(),
    };
    AlgLine {
        items: conv.items(&line.pieces),
        label,
        geometry: LineGeometry {
            dialect: a.dialect,
            label_width_em: a.label_width_em(),
            depth: line.depth,
            indent: setup.indent,
            hskip_tlm: line.hskip_tlm,
        },
        no_text: line.kind == LineKind::NoText,
        span: line.span,
    }
}

/// The caption paragraph: float.sty `\floatc@ruled` (`{\bfseries Algorithm
/// N} text`, line 151) or `\floatc@plain` (`Algorithm N: text`, line 140).
fn caption_items(setup: &alg::Setup, number: u32, span: Span, arg: Span, by_span: &HashMap<(usize, usize), &Vec<AItem>>) -> Vec<AItem> {
    let bold = setup.float_style != FloatStyle::Plain;
    let head = TextStyle { bold, ..TextStyle::default() };
    let mut items = Vec::new();
    // `\fname@algorithm{} \thealgorithm`: the space is in the caption font.
    for (k, w) in setup.float_name.split(' ').enumerate() {
        if k > 0 {
            items.push(AItem::Space { style: head, factor: 1000, no_break: false });
        }
        items.push(word(w, span, head));
    }
    items.push(AItem::Space { style: head, factor: 1000, no_break: false });
    let tail = if bold { number.to_string() } else { format!("{number}:") };
    let last = tail.chars().last().unwrap_or('0');
    items.push(word(&tail, span, head));
    if let Some(body) = by_span.get(&(arg.start, arg.end)).filter(|b| !b.is_empty()) {
        items.push(AItem::Space { style: TextStyle::default(), factor: adapter::space_factor(last, 1000), no_break: false });
        items.extend(body.iter().cloned());
    }
    items
}

/// The items of every span, parsed together from `documents[d]` blanked
/// except its preamble and the spans, then split by source position. An
/// item without a position (interword glue) belongs to a span only when the
/// positioned items on both sides do.
fn source_items(d: usize, spans: &[Span], documents: &[SourceDocument<'_>], entry_index: usize, texts: &[&str], options: &RenderOptions, labels: &Labels) -> Vec<Vec<AItem>> {
    let mut out = vec![Vec::new(); spans.len()];
    if spans.is_empty() {
        return out;
    }
    let text = documents[d].text;
    let mut bytes = crate::floats::isolate(text, Span::in_document(DocumentId(d), 0, 0)).into_bytes();
    for s in spans {
        bytes[s.start..s.end].copy_from_slice(&text.as_bytes()[s.start..s.end]);
    }
    let Ok(isolated) = String::from_utf8(bytes) else { return out };
    let mut texts2: Vec<&str> = texts.to_vec();
    texts2[d] = &isolated;
    let docs2: Vec<SourceDocument<'_>> = documents.iter().zip(&texts2).map(|(doc, t)| SourceDocument { path: doc.path, text: t }).collect();
    let parsed = flashtex_compiler::parser::parse_project(&docs2, documents[d].path);
    let doc = adapter::adapt(&texts2, entry_index, &parsed, options, labels);
    let mut all = Vec::new();
    for block in &doc.blocks {
        if let adapter::Block::Paragraph { parts, .. } = block {
            for part in parts {
                if let ParaPart::Lines(lines) = part {
                    all.extend(lines.iter().cloned());
                }
            }
        }
    }
    let position = |item: &AItem| -> Option<usize> {
        match item {
            AItem::Word(w) => w.segments.iter().find_map(|s| s.chars.first()).filter(|c| c.document.0 == d).map(|c| c.start),
            AItem::Math { span, .. } if span.document.0 == d => Some(span.start),
            _ => None,
        }
    };
    let owner: Vec<Option<Option<usize>>> = all
        .iter()
        .map(|item| position(item).map(|p| spans.iter().position(|s| s.start <= p && p < s.end)))
        .collect();
    for (i, item) in all.iter().enumerate() {
        let k = match owner[i] {
            Some(k) => k,
            None => {
                let prev = owner[..i].iter().rev().find_map(|o| *o).flatten();
                let next = owner[i + 1..].iter().find_map(|o| *o).flatten();
                prev.filter(|p| Some(*p) == next)
            }
        };
        if let Some(k) = k {
            out[k].push(item.clone());
        }
    }
    out
}
