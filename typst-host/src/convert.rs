//! Typst's laid-out pages to `display-list-v3` pages.
//!
//! The walk mirrors `typst-pdf`'s (convert.rs `handle_frame`): the page fill
//! first, then the frame translated by the bleed, groups composing their
//! transform and clip, text runs as positioned glyphs, shapes as paths. What
//! v3.3 (spec §11) cannot yet express is flagged INCOMPLETE with
//! an UNSUPPORTED entry at the place it was skipped, never approximated
//! (spec §4.7): gradients and tilings (E5 islands), images (E6), alpha and
//! spot colour (E3), stroked text (E4).
//!
//! **Positions** (DESIGN.md §15.5, spec §11.2). typst-pdf writes glyph
//! positions through krilla in f32, so Typst's frame positions miss the
//! exported PDF's by up to about 6·10⁻⁵ bp (Track A §5.2), enough to move
//! pixels. For a client that draws the page, every glyph's origin and glyph
//! matrix, and the page box, are the ones the viewer computes from
//! typst-pdf's export of the page ([`crate::pdfpos`]): ORIGINS carries the
//! origins and GLYPH their sp rounding. The walk consumes the PDF's glyphs
//! in painting order, the glyphs of runs it does not draw included; when
//! the counts disagree, or the export could not be read, the page is
//! INCOMPLETE (the positions are then Typst's, never guessed).

use std::collections::HashMap;

use flashtex_display_list::json::Json;
use flashtex_display_list::page::{
    self, Color, Item, Link, LinkKind, Page, Path, Seg, StreamKind, Stroke,
};
use flashtex_display_list::resource::{Font as FontRes, ImageData, Sources};
use flashtex_display_list::sha256::{hex, sha256};
use typst::layout::{Abs, Frame, FrameItem, FrameKind, GroupItem, Point, Size, Transform};
use typst::model::Destination;
use typst::syntax::{FileId, Span};
use typst::text::{FontInstance, TextItem};
use typst::visualize::{
    Color as TColor, Curve, CurveItem, FillRule, FixedStroke, Geometry, ImageKind, LineCap,
    LineJoin, Paint, ProcessColorSpace, RasterImage, Shape,
};
use typst::{World, WorldExt};
use typst_layout::PagedDocument;

use crate::pdfpos::{self, PagePos, PathOp, PdfColor};
use crate::world::HostWorld;

/// sp per bp (spec §1): 6578176/100.
const SP_PER_BP: f64 = 65_781.76;

fn sp(bp: f64) -> i32 {
    // Round to nearest, halves away from zero (spec §4.2).
    (bp * SP_PER_BP).round() as i32
}

/// What the client can take (from its `HELLO` and `COMPILE`).
#[derive(Clone, Copy, Debug, Default)]
pub struct ClientCaps {
    /// The negotiated minor version (1, 2 or 3).
    pub minor: u32,
    /// `COMPILE.font_formats` lists `opentype`.
    pub opentype_programs: bool,
    /// The client's HELLO `accept` lists [`PROGRAM_REFS`] (spec §11.1): it
    /// understands a FONT with an empty program and `program_from`.
    /// Without it every FONT that takes a program carries the whole program
    /// (spec §5.1: an empty program means the client already holds that
    /// font).
    pub program_refs: bool,
    /// At most this many font-program bytes per compile (`None`: no limit).
    pub program_budget: Option<u64>,
    /// `accept` lists `color-spaces` (spec §11.3): ICCBased and Separation
    /// colours (FILL/STROKE_COLOR_CS) and constant alpha are drawn.
    pub color_spaces: bool,
    /// `accept` lists `line-state` (spec §11.4): stroked glyphs are drawn.
    pub line_state: bool,
    /// `accept` lists `image-data` (spec §11.5): IMAGE with `"data": true`
    /// and its IMAGE_DATA.
    pub image_data: bool,
}

/// What a client's HELLO `accept` lists, of what this host sends (§11.7).
#[derive(Clone, Copy, Debug, Default)]
pub struct Accept {
    pub program_refs: bool,
    pub color_spaces: bool,
    pub line_state: bool,
    pub image_data: bool,
}

/// The 3.3 `accept` token for `program_from` (spec §11.1, §11.7).
pub const PROGRAM_REFS: &str = flashtex_display_list::accept::FONT_PROGRAM_REFS;

/// Default per-compile budget of font-program bytes sent to a client
/// (the host's `--font-program-budget`).
pub const DEFAULT_PROGRAM_BUDGET: u64 = 256 << 20;

/// One font resource (a font *instance*: program + face + variation
/// coordinates) of the connection. The program is not held here: it is
/// shared by every instance of the same file (see [`Program`]).
struct FontEntry {
    key: [u8; 32],
    info: Json,
    program: usize,
}

/// A font program, held once per connection however many variation
/// instances use it, and sent to the client at most once.
struct Program {
    data: typst::foundations::Bytes,
    /// The font id whose FONT frame carried the program, once sent.
    sent_with: Option<u16>,
}

/// Font ids are the u16 `font` of GLYPH items (spec §4.3), so a connection
/// has at most this many font instances.
pub const MAX_FONTS: usize = u16::MAX as usize + 1;

/// Resource and span ids that live as long as the connection keeps them
/// (spec §5, §5.3).
#[derive(Default)]
pub struct Tables {
    fonts: HashMap<FontInstance, u16>,
    font_list: Vec<FontEntry>,
    programs: Vec<Program>,
    /// Program index by content hash (and by font, to hash each file once).
    program_by_sha: HashMap<[u8; 32], usize>,
    program_by_font: HashMap<typst::text::Font, (usize, [u8; 32])>,
    /// At most this many font ids ([`MAX_FONTS`]; lower only in tests).
    font_limit: Option<usize>,
    /// Font-program bytes sent in the current compile (the budget).
    program_bytes: u64,
    /// Font ids the client has been sent (with or without the program).
    fonts_sent: Vec<bool>,
    files: HashMap<FileId, u32>,
    files_sent: Vec<bool>,
    spans: HashMap<Span, u32>,
    /// span id - 1 → (file id, line) last declared to the client.
    span_lines: Vec<Option<(u32, u32)>>,
    /// Per-span resolution cache for this compile.
    span_cache: HashMap<Span, Option<SpanPos>>,
    /// Image ids by key (spec §5.2, §11.5); id - 1 → key.
    images: HashMap<[u8; 32], u32>,
    image_keys: Vec<[u8; 32]>,
    /// Island image ids by the island page's Typst hash (E5).
    islands: HashMap<u128, u32>,
}

/// One frame on the way from the page's frame to the item being walked:
/// its size and kind, where the current item sits in it, and the group
/// that holds it (`None` for the page's own frame). A PDF island rebuilds
/// this chain around its one item, so that typst-pdf composes the same
/// transforms in the same order (E5).
struct Chain {
    size: Size,
    kind: FrameKind,
    pos: Point,
    group: Option<(Transform, Option<Curve>)>,
}

#[derive(Clone, Copy)]
struct SpanPos {
    span_id: u32,
    /// Start byte of the span, and of its first line (and that line's end).
    start: usize,
    line_start: usize,
    line_end: usize,
}

impl Tables {
    pub fn new() -> Tables {
        Tables::default()
    }

    /// Tables with a lower font-id limit than [`MAX_FONTS`] (tests).
    #[doc(hidden)]
    pub fn with_font_limit(limit: usize) -> Tables {
        Tables {
            font_limit: Some(limit.min(MAX_FONTS)),
            ..Tables::default()
        }
    }

    /// Distinct font programs held (tests and diagnostics).
    pub fn program_count(&self) -> usize {
        self.programs.len()
    }

    /// A new compile begins: spans may have moved.
    pub fn begin_compile(&mut self) {
        self.span_cache.clear();
        self.program_bytes = 0;
    }
}

/// One converted page, ready to send.
pub struct PageOut {
    /// FONT frames this page needs first (bodies).
    pub fonts: Vec<Vec<u8>>,
    /// IMAGE and IMAGE_DATA frames this page needs first (bodies).
    pub images: Vec<(Vec<u8>, Vec<u8>)>,
    /// A SOURCES frame this page needs first, if any.
    pub sources: Option<Json>,
    /// The PAGE body.
    pub body: Vec<u8>,
    pub glyphs: usize,
    pub flags: u32,
}

/// Where a page's glyph positions come from.
#[derive(Clone, Copy)]
pub enum Positions<'a> {
    /// Typst's frame (a client that cannot draw the page: it is INCOMPLETE
    /// anyway).
    Frame,
    /// The viewer's, from typst-pdf's export of the page.
    Pdf(&'a PagePos),
    /// The export could not be read: the page is INCOMPLETE, with why.
    Failed(&'a str),
}

/// How far (bp) a run's frame positions may be from the PDF's when the walk
/// looks for the run past an image's inline text: krilla writes f32, about
/// 6·10⁻⁵ bp off at page sizes; anything near 0.01 bp is another glyph.
const ALIGN_BP: f64 = 0.01;

/// Graphics state a SAVE scopes: fill, stroke, text render mode.
type Saved = (Option<Item>, Option<Item>, u8, f64, f64, Option<Stroke>);

struct Walker<'a, 'w> {
    world: &'a HostWorld<'w>,
    doc: &'a PagedDocument,
    tables: &'a mut Tables,
    /// The Typst page being converted (its fill, bleed, frame).
    tpage: &'a typst_layout::Page,
    chain: Vec<Chain>,
    pending_group: Option<(Transform, Option<Curve>)>,
    caps: ClientCaps,
    have_fonts: &'a [String],
    /// The first error that stops the compile (the font-id limit).
    error: Option<String>,
    /// Page height in bp (stream space's y flip).
    h: f64,
    /// The PDF's glyphs, and how many of them the walk has consumed.
    pdf: Option<&'a PagePos>,
    pdf_at: usize,
    /// An image since the last run: the PDF may show its SVG text first.
    image_gap: bool,
    /// The PDF's paths consumed, and the same image allowance for them
    /// (an SVG image's paths are drawn inline too).
    path_at: usize,
    path_gap: bool,
    /// Shapes and clips not found where the PDF paints them.
    path_misaligned: usize,
    /// Runs not found where the PDF should show them.
    misaligned: usize,
    /// The PDF's XObjects (`Do`) consumed, and an allowance after an SVG or
    /// PDF image (whose own raster images the PDF draws inline).
    image_at: usize,
    image_op_gap: bool,
    images_out: Vec<(Vec<u8>, Vec<u8>)>,
    page: Page,
    matrices: HashMap<[u64; 6], u32>,
    unsupported: HashMap<String, u32>,
    origins: Vec<(f64, f64)>,
    fonts_out: Vec<Vec<u8>>,
    sources_out: Sources,
    /// The fill and stroke colour items in effect (FILL_COLOR or
    /// FILL_COLOR_CS, ...), the alphas and the line state (3.3).
    fill: Option<Item>,
    stroke: Option<Item>,
    fill_alpha: f64,
    stroke_alpha: f64,
    line: Option<Stroke>,
    text_render: u8,
    glyph_matrix: Option<u32>,
    span: u32,
    /// (fill, stroke, text_render) saved by each open SAVE.
    stack: Vec<Saved>,
}

/// Convert page `index` of `doc`.
pub fn page(
    world: &HostWorld,
    doc: &PagedDocument,
    index: usize,
    tables: &mut Tables,
    caps: ClientCaps,
    have_fonts: &[String],
    positions: Positions,
) -> Result<PageOut, String> {
    let tp = &doc.pages()[index];
    let size = tp.frame.size() + tp.bleed.sum_by_axis();
    let (w, h) = (size.x.to_pt(), size.y.to_pt());
    let mut p = Page::new(StreamKind::Page, index as u32);
    let pdf = match positions {
        Positions::Pdf(pp) => Some(pp),
        _ => None,
    };
    // The box as the PDF writes it (f32 numbers), the y flip of every
    // stream-space coordinate (spec §4.1, §4.2).
    p.pdf_box = match pdf {
        Some(pp) => pp.media_box,
        None => [0.0, 0.0, w, h],
    };
    let h = p.pdf_box[3];
    p.width = sp(p.pdf_box[2] - p.pdf_box[0]);
    p.height = sp(p.pdf_box[3] - p.pdf_box[1]);
    let mut wk = Walker {
        world,
        doc,
        tables,
        tpage: tp,
        chain: Vec::new(),
        pending_group: None,
        caps,
        have_fonts,
        h,
        pdf,
        pdf_at: 0,
        image_gap: false,
        path_at: 0,
        path_gap: false,
        path_misaligned: 0,
        misaligned: 0,
        page: p,
        matrices: HashMap::new(),
        unsupported: HashMap::new(),
        origins: Vec::new(),
        fonts_out: Vec::new(),
        image_at: 0,
        image_op_gap: false,
        images_out: Vec::new(),
        sources_out: Sources::default(),
        fill: None,
        stroke: None,
        fill_alpha: 1.0,
        stroke_alpha: 1.0,
        line: None,
        text_render: 0,
        glyph_matrix: None,
        span: 0,
        stack: Vec::new(),
        error: None,
    };
    if let Some(fill) = tp.fill_or_transparent() {
        let shape = Geometry::Rect(size).filled(fill);
        wk.shape(&shape, Span::detached(), Transform::identity(), None);
    }
    let ts = Transform::translate(tp.bleed.left, tp.bleed.top);
    wk.frame(&tp.frame, ts);

    // A 3.1 or 3.2 client cannot draw OpenType glyph ids (it refuses `opentype`
    // without the capability): it gets the page INCOMPLETE and renders
    // DONE.pdf, as DESIGN.md §15.4 specifies.
    let glyphs = wk.origins.len();
    if glyphs > 0 && (wk.caps.minor < 3 || !wk.caps.opentype_programs) {
        wk.unsupported("opentype glyphs (display-list-v3.3 E1 and font_formats opentype)");
    }
    match positions {
        // Glyphs left over are allowed only after a last image (its SVG text).
        Positions::Pdf(pp)
            if wk.misaligned > 0 || (wk.pdf_at != pp.glyphs.len() && !wk.image_gap) =>
        {
            let m = format!(
                "glyph positions: {} runs not where the PDF shows them; the PDF shows {} glyphs, the walk reached {}",
                wk.misaligned,
                pp.glyphs.len(),
                wk.pdf_at
            );
            wk.unsupported(&m);
        }
        Positions::Failed(e) => {
            let m = format!("glyph positions: {e}");
            wk.unsupported(&m);
        }
        _ => {}
    }
    if let Positions::Pdf(pp) = positions {
        if wk.path_misaligned > 0 || (wk.path_at != pp.paths.len() && !wk.path_gap) {
            let m = format!(
                "paths: {} shapes or clips not where the PDF paints them; the PDF paints {} paths, the walk reached {}",
                wk.path_misaligned,
                pp.paths.len(),
                wk.path_at
            );
            wk.unsupported(&m);
        }
    }

    let mut page = wk.page;
    if wk.caps.minor >= 3 {
        // 3.3 (spec §11.2): ORIGINS and PAGE_META; both enter the hash.
        page.origins = wk.origins.iter().map(|&(x, y)| [x, y]).collect();
        let bleed = &tp.bleed;
        let meta = Json::Obj(vec![
            ("engine".into(), Json::Str("typst".into())),
            ("number".into(), Json::Int(tp.number as i64)),
            (
                "bleed".into(),
                Json::Arr(
                    [bleed.left, bleed.top, bleed.right, bleed.bottom]
                        .iter()
                        .map(|a| Json::Num(a.to_pt()))
                        .collect(),
                ),
            ),
        ]);
        page.meta = Some(meta.to_string());
    }
    let font_list = &wk.tables.font_list;
    let image_keys = &wk.tables.image_keys;
    page.hash = page.content_hash(
        &|id| font_list.get(id as usize).map(|f| f.key).unwrap_or([0; 32]),
        &|id| {
            image_keys
                .get((id as usize).wrapping_sub(1))
                .copied()
                .unwrap_or([0; 32])
        },
    );
    let body = page.encode();
    let sources = if wk.sources_out.files.is_empty() && wk.sources_out.spans.is_empty() {
        None
    } else {
        Some(wk.sources_out.to_json())
    };
    if let Some(e) = wk.error {
        return Err(e);
    }
    Ok(PageOut {
        fonts: wk.fonts_out,
        images: wk.images_out,
        sources,
        body,
        glyphs,
        flags: page.flags,
    })
}

impl<'a> Walker<'a, '_> {
    fn frame(&mut self, frame: &Frame, ts: Transform) {
        let group = self.pending_group.take();
        self.chain.push(Chain {
            size: frame.size(),
            kind: frame.kind(),
            pos: Point::zero(),
            group,
        });
        for (pos, item) in frame.items() {
            if let Some(c) = self.chain.last_mut() {
                c.pos = *pos;
            }
            let ts = ts.pre_concat(Transform::translate(pos.x, pos.y));
            match item {
                FrameItem::Group(g) => self.group(g, ts),
                FrameItem::Text(t) => self.text(t, ts, item),
                FrameItem::Shape(s, span) => self.shape(s, *span, ts, Some(item)),
                FrameItem::Image(image, size, span) => {
                    // An SVG image's text is drawn inline in the PDF: the
                    // next run may start after glyphs of the image's own.
                    self.image_gap = true;
                    self.path_gap = true;
                    self.set_span(*span, None);
                    match image.kind() {
                        ImageKind::Raster(r) => self.raster(r, *size, ts),
                        ImageKind::Svg(_) => {
                            self.image_op_gap = true;
                            self.island(Some(item), "SVG image");
                        }
                        ImageKind::Pdf(_) => {
                            self.image_op_gap = true;
                            self.island(Some(item), "PDF image");
                        }
                    }
                }
                FrameItem::Link(dest, size) => self.link(dest, *size, ts),
                FrameItem::Tag(_) => {}
            }
        }
        self.chain.pop();
    }

    /// The client draws PDF islands (`image-data`, spec §11.5).
    fn islands(&self) -> bool {
        self.caps.minor >= 3 && self.caps.image_data
    }

    /// A PDF island (E5, spec §11.5) for `item`, or, with `None`, for the
    /// page's fill: a one-page PDF of the page's size that typst-pdf
    /// exports with that item alone, inside the same chain of frames,
    /// groups, transforms and clips as on the page, so that its content
    /// stream carries the page's own numbers; drawn by an IMAGE item with
    /// the identity matrix (the island's page space is the page's).
    /// Without `image-data` the page is INCOMPLETE. Returns whether it is
    /// drawn.
    fn island(&mut self, item: Option<&FrameItem>, what: &str) -> bool {
        if !self.islands() {
            let m = format!("{what} (display-list-v3.3 E5 island: accept image-data)");
            self.unsupported(&m);
            return false;
        }
        let tp = self.tpage;
        let page = match item {
            None => typst_layout::Page {
                frame: Frame::new(tp.frame.size(), tp.frame.kind()),
                bleed: tp.bleed,
                fill: tp.fill.clone(),
                numbering: None,
                supplement: Default::default(),
                number: tp.number,
            },
            Some(item) => {
                let mut child = item.clone();
                let mut page_frame = None;
                for lvl in self.chain.iter().rev() {
                    let mut f = Frame::new(lvl.size, lvl.kind);
                    f.push(lvl.pos, child);
                    match &lvl.group {
                        Some((t, clip)) => {
                            let mut g = GroupItem::new(f);
                            g.transform = *t;
                            g.clip = clip.clone();
                            child = FrameItem::Group(g);
                        }
                        None => {
                            page_frame = Some(f);
                            break;
                        }
                    }
                }
                let Some(frame) = page_frame else {
                    let m = format!("{what}: no page frame for its island");
                    self.unsupported(&m);
                    return false;
                };
                typst_layout::Page {
                    frame,
                    bleed: tp.bleed,
                    fill: typst::foundations::Smart::Custom(None),
                    numbering: None,
                    supplement: Default::default(),
                    number: tp.number,
                }
            }
        };
        let h = typst::utils::hash128(&page);
        if let Some(&id) = self.tables.islands.get(&h) {
            self.page.items.push(Item::Image { id, matrix: 0 });
            return true;
        }
        let doc = PagedDocument::new(
            typst::ecow::eco_vec![page],
            typst::model::DocumentInfo::default(),
        );
        let opts = typst_pdf::PdfOptions {
            tagged: false,
            ..Default::default()
        };
        let bytes = match typst_pdf::pdf(&doc, &opts) {
            Ok(b) => b,
            Err(errs) => {
                let m: Vec<String> = errs.iter().map(|e| e.message.to_string()).collect();
                let m = format!("{what}: its island cannot be exported: {}", m.join("; "));
                self.unsupported(&m);
                return false;
            }
        };
        let b = self.page.pdf_box;
        let info = vec![
            ("type".to_string(), Json::Str("pdf".into())),
            ("island".into(), Json::Bool(true)),
            ("data".into(), Json::Bool(true)),
            ("page".into(), Json::Int(1)),
            ("page_box".into(), Json::Str("media".into())),
            ("width".into(), Json::Num(b[2] - b[0])),
            ("height".into(), Json::Num(b[3] - b[1])),
            ("orig_x".into(), Json::Num(b[0])),
            ("orig_y".into(), Json::Num(b[1])),
        ];
        let id = self.register_image(info, vec![bytes]);
        self.tables.islands.insert(h, id);
        self.page.items.push(Item::Image { id, matrix: 0 });
        true
    }

    /// The id of the image `info` with `parts` (spec §11.5), queueing its
    /// IMAGE and IMAGE_DATA when the connection does not hold it yet.
    fn register_image(&mut self, mut info: Vec<(String, Json)>, parts: Vec<Vec<u8>>) -> u32 {
        let mut h = flashtex_display_list::sha256::Sha256::new();
        h.update(b"display-list-v3 image\0");
        h.update(Json::Obj(info.clone()).to_string().as_bytes());
        for p in &parts {
            h.update(&sha256(p));
        }
        let key = h.finish();
        if let Some(&id) = self.tables.images.get(&key) {
            return id;
        }
        let id = self.tables.image_keys.len() as u32 + 1;
        self.tables.image_keys.push(key);
        self.tables.images.insert(key, id);
        let mut kv = vec![
            ("id".to_string(), Json::Int(id as i64)),
            ("key".into(), Json::Str(hex(&key))),
        ];
        kv.append(&mut info);
        let body = ImageData { id, parts }.encode();
        self.images_out
            .push((Json::Obj(kv).to_string().into_bytes(), body));
        id
    }

    fn group(&mut self, g: &GroupItem, ts: Transform) {
        let ts = ts.pre_concat(g.transform);
        if let Some(clip) = &g.clip {
            self.save();
            let segs = curve(clip);
            let m6 = self.ctm6(ts);
            let clip_bits = page::paint::CLIP | page::paint::CLIP_EVEN_ODD;
            let ops = self.take_paths(&segs, m6, clip_bits, clip_bits, segs.is_empty());
            match ops {
                Some(ops) => {
                    for op in ops {
                        let n = self.pdf_path(op, op.paint & clip_bits);
                        self.page.items.push(Item::Clip(n));
                    }
                }
                None => {
                    let m = self.matrix(m6);
                    let n = self.push_path(Path {
                        paint: page::paint::CLIP,
                        matrix: m,
                        stroke: None,
                        segs,
                    });
                    self.page.items.push(Item::Clip(n));
                }
            }
            self.pending_group = Some((g.transform, g.clip.clone()));
            self.frame(&g.frame, ts);
            self.restore();
        } else {
            self.pending_group = Some((g.transform, g.clip.clone()));
            self.frame(&g.frame, ts);
        }
    }

    /// A raster image (spec §5.2, §11.5): the PDF's image XObject and the
    /// CTM it is drawn with, found where the frame puts the image (the unit
    /// square's corners within [`ALIGN_BP`]); the pixels are the PDF's
    /// (`raw` samples after typst-pdf's conversion, or the JPEG it passes
    /// through), sent once per connection.
    fn raster(&mut self, raster: &RasterImage, size: Size, ts: Transform) {
        // typst-pdf draws a JPEG with its EXIF orientation as a transform,
        // and nothing for an image krilla's size cannot hold (zero, say).
        let (exif, size) = exif_transform(raster, size);
        let ts = ts.pre_concat(exif);
        let (wf, hf) = (size.x.to_pt() as f32, size.y.to_pt() as f32);
        if !(wf.is_finite() && hf.is_finite() && wf > 0.0 && hf > 0.0) {
            return;
        }
        if self.caps.minor < 3 || !self.caps.image_data {
            self.unsupported("image (display-list-v3.3 E6: accept image-data)");
            return;
        }
        let Some(pp) = self.pdf else {
            self.unsupported("image without the PDF's positions");
            return;
        };
        let m = self.ctm6(ts);
        let pt = |x: f64, y: f64| [x * m[0] + y * m[2] + m[4], x * m[1] + y * m[3] + m[5]];
        let (w, h) = (size.x.to_pt(), size.y.to_pt());
        // The unit square's (0,0), (1,0), (0,1): the image's bottom-left,
        // bottom-right and top-left corners.
        let want = [pt(0.0, h), pt(w, h), pt(0.0, 0.0)];
        let fits = |op: &pdfpos::ImageOp| {
            let c = op.ctm;
            let got = [
                [c[4], c[5]],
                [c[0] + c[4], c[1] + c[5]],
                [c[2] + c[4], c[3] + c[5]],
            ];
            !op.form
                && got.iter().zip(&want).all(|(g, w)| {
                    (g[0] - w[0]).abs() <= ALIGN_BP && (g[1] - w[1]).abs() <= ALIGN_BP
                })
        };
        let start = self.image_at;
        let found = if pp.images.get(start).is_some_and(fits) {
            Some(start)
        } else if self.image_op_gap {
            (start + 1..pp.images.len()).find(|&k| fits(&pp.images[k]))
        } else {
            None
        };
        self.image_op_gap = false;
        let Some(k) = found else {
            self.image_at = start + 1;
            self.unsupported("image: not where the PDF draws it");
            return;
        };
        self.image_at = k + 1;
        let op = &pp.images[k];
        if op.fill_alpha != 1.0 {
            self.unsupported("image with alpha (display-list-v3.3 E3)");
            return;
        }
        let img = match &op.image {
            Ok(i) => i.clone(),
            Err(e) => {
                let m = format!("image: {e}");
                self.unsupported(&m);
                return;
            }
        };
        let Some(id) = self.image_id(&img) else {
            return;
        };
        let n = self.matrix(op.ctm);
        self.page.items.push(Item::Image { id, matrix: n });
    }

    /// The connection's id for an image, queueing its IMAGE and IMAGE_DATA
    /// the first time a page uses it (`None`, and the page INCOMPLETE,
    /// when its data cannot be decoded).
    fn image_id(&mut self, img: &pdfpos::PdfImage) -> Option<u32> {
        let jpeg = img.encoding == pdfpos::ImageEncoding::Jpeg;
        let mut info = vec![
            (
                "type".to_string(),
                Json::Str(if jpeg { "jpeg" } else { "raw" }.into()),
            ),
            ("data".into(), Json::Bool(true)),
            ("width".into(), Json::Int(img.width as i64)),
            ("height".into(), Json::Int(img.height as i64)),
            ("components".into(), Json::Int(img.components as i64)),
            ("bits".into(), Json::Int(img.bits as i64)),
            ("interpolate".into(), Json::Bool(img.interpolate)),
            ("smask".into(), Json::Bool(img.mask.is_some())),
            ("icc".into(), Json::Bool(img.icc.is_some())),
        ];
        // The key covers the encoded streams: an image the connection holds
        // is not decoded again.
        let mut h = flashtex_display_list::sha256::Sha256::new();
        h.update(b"display-list-v3 image\0");
        h.update(Json::Obj(info.clone()).to_string().as_bytes());
        for part in [
            Some(img.data.as_slice()),
            img.mask.as_ref().map(|m| m.1.as_slice()),
            img.icc.as_ref().map(|p| p.as_slice()),
        ]
        .into_iter()
        .flatten()
        {
            h.update(&sha256(part));
        }
        let key = h.finish();
        if let Some(&id) = self.tables.images.get(&key) {
            return Some(id);
        }
        let parts = (|| -> Result<Vec<Vec<u8>>, String> {
            let mut p = vec![img.data_part()?];
            if let Some(m) = img.mask_part()? {
                p.push(m);
            }
            if let Some(icc) = &img.icc {
                p.push(icc.as_ref().clone());
            }
            Ok(p)
        })();
        let parts = match parts {
            Ok(p) => p,
            Err(e) => {
                let m = format!("image: {e}");
                self.unsupported(&m);
                return None;
            }
        };
        let id = self.tables.image_keys.len() as u32 + 1;
        self.tables.image_keys.push(key);
        self.tables.images.insert(key, id);
        let mut kv = vec![
            ("id".to_string(), Json::Int(id as i64)),
            ("key".into(), Json::Str(hex(&key))),
        ];
        kv.append(&mut info);
        let body = ImageData { id, parts }.encode();
        self.images_out
            .push((Json::Obj(kv).to_string().into_bytes(), body));
        Some(id)
    }

    /// Frame origins of a run's glyphs, stream space (y up).
    fn frame_origins(&self, t: &TextItem, ts: Transform) -> Vec<[f64; 2]> {
        let (mut x, mut y) = (Abs::zero(), Abs::zero());
        t.glyphs
            .iter()
            .map(|g| {
                let p =
                    Point::new(x + g.x_offset.at(t.size), y - g.y_offset.at(t.size)).transform(ts);
                x += g.x_advance.at(t.size);
                y -= g.y_advance.at(t.size);
                [p.x.to_pt(), self.h - p.y.to_pt()]
            })
            .collect()
    }

    /// Where run `t`'s glyphs are in the PDF's glyph sequence: right after
    /// the previous run's (every glyph of every run is shown, drawn here or
    /// not), or, after an image (whose SVG text the PDF shows inline), at
    /// the first place further on where they all fall within
    /// [`ALIGN_BP`] of the frame's positions. `None` (and the page
    /// INCOMPLETE) when the PDF does not show the run where it should.
    fn align(&mut self, frame: &[[f64; 2]]) -> Option<usize> {
        let pp = self.pdf?;
        let n = frame.len();
        let fits = |k: usize| {
            k + n <= pp.glyphs.len()
                && (0..n).all(|j| {
                    let (a, b) = (pp.glyphs[k + j].origin, frame[j]);
                    (a[0] - b[0]).abs() <= ALIGN_BP && (a[1] - b[1]).abs() <= ALIGN_BP
                })
        };
        let start = self.pdf_at;
        let found = if fits(start) {
            Some(start)
        } else if self.image_gap {
            (start + 1..=pp.glyphs.len().saturating_sub(n)).find(|&k| fits(k))
        } else {
            None
        };
        self.image_gap = false;
        match found {
            Some(k) => {
                self.pdf_at = k + n;
                Some(k)
            }
            None => {
                self.misaligned += 1;
                self.pdf_at = start + n;
                None
            }
        }
    }

    fn text(&mut self, t: &TextItem, ts: Transform, item: &FrameItem) {
        let frame = self.frame_origins(t, ts);
        let first = self.align(&frame);
        // Gradient or tiling text: a PDF island (E5) when the client draws
        // them.
        if self.islands()
            && (is_pattern(&t.fill) || t.stroke.as_ref().is_some_and(|s| is_pattern(&s.paint)))
        {
            if let Some(g) = t.glyphs.first() {
                self.set_span(g.span.0, Some(g.span.1));
            }
            self.island(Some(item), "gradient or tiling text");
            self.image_op_gap = true;
            return;
        }
        let Some(fill) = self.paint(&t.fill) else {
            return;
        };
        let Some(font) = self.font(&t.font) else {
            return;
        };
        let from_pdf = match (self.pdf, first) {
            (Some(pp), Some(k)) => Some(&pp.glyphs[k..k + t.glyphs.len()]),
            _ => None,
        };
        // The paint state the PDF draws the run with (its first glyph's).
        let pdf_paint = match (self.pdf, from_pdf) {
            (Some(pp), Some(pg)) => pg.first().map(|g| &pp.paints[g.paint as usize]),
            _ => None,
        };
        match pdf_paint {
            Some(p) => {
                if !self.pdf_fill(p) {
                    return;
                }
            }
            None => self.set_fill(fill),
        }
        let mut render = 0;
        if let Some(st) = &t.stroke {
            // Stroked glyphs (E4): the PDF's text render mode, line state
            // and stroke colour, for a client that accepts `line-state`.
            match pdf_paint {
                Some(p) if self.caps.line_state && self.pdf_stroke(p) => {
                    render = p.render;
                    if self.line.as_ref() != Some(&p.line) {
                        self.line = Some(p.line.clone());
                        self.page.items.push(Item::LineState(p.line.clone()));
                    }
                }
                _ => {
                    let _ = st;
                    self.unsupported("stroked text (display-list-v3.3 E4)");
                }
            }
        }
        if self.text_render != render {
            self.text_render = render;
            self.page.items.push(Item::TextRender(render));
        }
        let s = t.size.to_pt();
        let (sx, ky, kx, sy) = (ts.sx.get(), ts.ky.get(), ts.kx.get(), ts.sy.get());
        let frame_matrix = [s * sx, -s * ky, -s * kx, s * sy];
        for (i, g) in t.glyphs.iter().enumerate() {
            // Origin (stream space, y up) and glyph matrix: the PDF's, else
            // the frame's.
            let (origin, lin) = match from_pdf {
                Some(pg) => (pg[i].origin, pg[i].matrix),
                None => (frame[i], frame_matrix),
            };
            let m = self.matrix([lin[0], lin[1], lin[2], lin[3], 0.0, 0.0]);
            if self.glyph_matrix != Some(m) {
                self.glyph_matrix = Some(m);
                self.page.items.push(Item::Matrix(m));
            }
            let col = self.set_span(g.span.0, Some(g.span.1));
            self.origins.push((origin[0], origin[1]));
            self.page.items.push(Item::Glyph {
                font,
                code: g.id,
                x: sp(origin[0]),
                y: sp(self.h - origin[1]),
                col,
            });
        }
    }

    fn shape(&mut self, s: &Shape, span: Span, ts: Transform, item: Option<&FrameItem>) {
        let segs = match &s.geometry {
            Geometry::Line(p) => vec![Seg::Move(0.0, 0.0), Seg::Line(p.x.to_pt(), p.y.to_pt())],
            Geometry::Rect(size) => {
                let (w, h) = (size.x.to_pt(), size.y.to_pt());
                vec![
                    Seg::Move(0.0, 0.0),
                    Seg::Line(w, 0.0),
                    Seg::Line(w, h),
                    Seg::Line(0.0, h),
                    Seg::Close,
                ]
            }
            Geometry::Curve(c) => curve(c),
        };
        // What typst-pdf (krilla) paints for the shape, drawable here or not:
        // the path(s) to take from the PDF.
        let stroke = s.stroke.as_ref().filter(|st| st.thickness.to_pt() > 0.0);
        let mut want = 0;
        if s.fill.is_some() {
            want |= page::paint::FILL | page::paint::FILL_EVEN_ODD;
        }
        if stroke.is_some() {
            want |= page::paint::STROKE;
        }
        let m6 = self.ctm6(ts);
        let ops = if want == 0 {
            None
        } else {
            self.take_paths(&segs, m6, want, 0, krilla_skips(&segs, stroke.is_some()))
        };
        // A gradient or tiling fill or stroke: the whole shape as a PDF
        // island (E5) when the client draws them, its PDF paths consumed.
        if self.islands()
            && (s.fill.as_ref().is_some_and(is_pattern)
                || stroke.is_some_and(|st| is_pattern(&st.paint)))
        {
            if ops.as_ref().is_some_and(|o| o.is_empty()) {
                return;
            }
            self.set_span(span, None);
            self.island(item, "gradient or tiling");
            self.image_op_gap = true;
            return;
        }
        let fill = s.fill.as_ref().map(|p| self.paint(p));
        let stroke_paint = stroke.map(|st| self.paint(&st.paint));
        let fill = fill.flatten();
        let stroke_color = stroke_paint.flatten();
        let mut paint = 0;
        let from_pdf = ops.is_some();
        if let Some(c) = fill {
            if !from_pdf {
                self.set_fill(c);
            }
            paint |= match s.fill_rule {
                FillRule::NonZero => page::paint::FILL,
                FillRule::EvenOdd => page::paint::FILL_EVEN_ODD,
            };
        }
        let mut st = None;
        if let (Some(c), Some(fs)) = (stroke_color, stroke) {
            if !from_pdf {
                self.set_stroke(c);
            }
            paint |= page::paint::STROKE;
            st = Some(stroke_params(fs));
        }
        if paint == 0 {
            return;
        }
        self.set_span(span, None);
        match ops {
            // The PDF's paths, with what is drawable here of their paint.
            Some(ops) => {
                for op in ops {
                    let mut bits = op.paint & paint;
                    if op.paint & page::paint::FILL_EVEN_ODD != 0 && paint & page::paint::FILL != 0
                    {
                        bits |= page::paint::FILL_EVEN_ODD;
                    }
                    if op.paint & page::paint::FILL != 0 && paint & page::paint::FILL_EVEN_ODD != 0
                    {
                        bits |= page::paint::FILL;
                    }
                    // The PDF's colours and alphas for what is painted.
                    let ps = &self.pdf.expect("PDF paths").paints[op.paint_state as usize];
                    if bits & (page::paint::FILL | page::paint::FILL_EVEN_ODD) != 0
                        && !self.pdf_fill(ps)
                    {
                        bits &= !(page::paint::FILL | page::paint::FILL_EVEN_ODD);
                    }
                    if bits & page::paint::STROKE != 0 && !self.pdf_stroke(ps) {
                        bits &= !page::paint::STROKE;
                    }
                    if bits != 0 {
                        let n = self.pdf_path(op, bits);
                        self.page.items.push(Item::Path(n));
                    }
                }
            }
            None if self.pdf.is_some() => {
                // typst-pdf paints nothing for it (zero-size geometry):
                // neither does the page.
            }
            None => {
                let m = self.matrix(m6);
                let n = self.push_path(Path {
                    paint,
                    matrix: m,
                    stroke: st,
                    segs,
                });
                self.page.items.push(Item::Path(n));
            }
        }
    }

    /// The PDF's path(s) for a shape or clip whose frame geometry is `segs`
    /// under the stream-space matrix `m6`: the next path(s) the PDF paints,
    /// when their first point is within [`ALIGN_BP`] of the frame's and
    /// their paint is part of `want`; after an image, the first such path
    /// further on. Two paths when krilla fills and strokes separately.
    /// `Some(empty)` when krilla paints nothing for it (`skips`) and the PDF
    /// agrees; `None` with frame positions (no PDF), and also, counted as
    /// misaligned, when the PDF does not paint it where it should.
    fn take_paths(
        &mut self,
        segs: &[Seg],
        m6: [f64; 6],
        want: u8,
        clip: u8,
        skips: bool,
    ) -> Option<Vec<&'a PathOp>> {
        let pp = self.pdf?;
        // Where the path lies on the page: the stream-space bounding box of
        // its points (krilla normalises a rectangle of negative size, so the
        // first point may differ while the box does not).
        let bbox = |segs: &[Seg], m: &[f64; 6]| {
            let mut b: Option<[f64; 4]> = None;
            let mut add = |x: f64, y: f64| {
                let p = [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]];
                b = Some(match b {
                    None => [p[0], p[1], p[0], p[1]],
                    Some(b) => [
                        b[0].min(p[0]),
                        b[1].min(p[1]),
                        b[2].max(p[0]),
                        b[3].max(p[1]),
                    ],
                });
            };
            for s in segs {
                match *s {
                    Seg::Move(x, y) | Seg::Line(x, y) => add(x, y),
                    Seg::Curve(a, b2, c, d, e, f) => {
                        add(a, b2);
                        add(c, d);
                        add(e, f);
                    }
                    Seg::Close => {}
                }
            }
            b
        };
        let want_box = bbox(segs, &m6);
        let fits = |op: &PathOp| {
            let bits = op.paint & !clip;
            let paint_ok = if clip != 0 {
                op.paint & clip != 0
            } else {
                bits != 0
                    && bits & !want == 0
                    && op.paint & (page::paint::CLIP | page::paint::CLIP_EVEN_ODD) == 0
            };
            paint_ok
                && match (want_box, bbox(&op.segs, &op.ctm)) {
                    (Some(a), Some(b)) => a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= ALIGN_BP),
                    (None, None) => true,
                    _ => false,
                }
        };
        let start = self.path_at;
        let found = if pp.paths.get(start).is_some_and(fits) {
            Some(start)
        } else if self.path_gap {
            (start + 1..pp.paths.len()).find(|&k| fits(&pp.paths[k]))
        } else {
            None
        };
        let Some(k) = found else {
            if skips {
                return Some(vec![]);
            }
            self.path_misaligned += 1;
            return None;
        };
        self.path_gap = false;
        let mut out = vec![&pp.paths[k]];
        // Filled, then stroked separately (krilla does so for a stroke
        // with alpha or gradients with alpha).
        if clip == 0 {
            let got = pp.paths[k].paint;
            if let Some(next) = pp.paths.get(k + 1) {
                if got & page::paint::STROKE == 0
                    && want & page::paint::STROKE != 0
                    && next.paint == page::paint::STROKE
                    && next.segs == pp.paths[k].segs
                    && next.ctm == pp.paths[k].ctm
                {
                    out.push(next);
                }
            }
        }
        self.path_at = k + out.len();
        Some(out)
    }

    /// A PATH from the PDF's path, painted with `paint`.
    fn pdf_path(&mut self, op: &PathOp, paint: u8) -> u32 {
        let m = self.matrix(op.ctm);
        let stroke = if paint & page::paint::STROKE != 0 {
            op.stroke.clone()
        } else {
            None
        };
        self.push_path(Path {
            paint,
            matrix: m,
            stroke,
            segs: op.segs.clone(),
        })
    }

    fn link(&mut self, dest: &Destination, size: Size, ts: Transform) {
        let corners = [
            Point::zero(),
            Point::new(size.x, Abs::zero()),
            Point::new(Abs::zero(), size.y),
            size.to_point(),
        ];
        let pts: Vec<(f64, f64)> = corners
            .iter()
            .map(|c| c.transform(ts))
            .map(|p| (p.x.to_pt(), p.y.to_pt()))
            .collect();
        let fold = |f: fn(f64, f64) -> f64, i: usize| {
            pts.iter()
                .map(|p| if i == 0 { p.0 } else { p.1 })
                .reduce(f)
                .unwrap()
        };
        let rect = [
            sp(fold(f64::min, 0)),
            sp(fold(f64::min, 1)),
            sp(fold(f64::max, 0)),
            sp(fold(f64::max, 1)),
        ];
        let position = match dest {
            Destination::Url(u) => {
                self.page.links.push(Link {
                    rect,
                    span: 0,
                    kind: LinkKind::Uri,
                    file: vec![],
                    data: u.as_str().as_bytes().to_vec(),
                });
                return;
            }
            Destination::Position(p) => Some(*p),
            Destination::Location(loc) => self.doc.introspector().position(*loc),
        };
        let Some(pos) = position else { return };
        let n = pos.page.get();
        let Some(target) = self.doc.pages().get(n - 1) else {
            return;
        };
        let th = (target.frame.size() + target.bleed.sum_by_axis()).y.to_pt();
        let x = pos.point.x.to_pt() + target.bleed.left.to_pt();
        let y = th - (pos.point.y.to_pt() + target.bleed.top.to_pt());
        let data = format!("{n} /XYZ {} {} null", fmt(x), fmt(y));
        self.page.links.push(Link {
            rect,
            span: 0,
            kind: LinkKind::GotoPage,
            file: vec![],
            data: data.into_bytes(),
        });
    }

    /// The CTM mapping a frame's local (y-down, pt) space to stream space
    /// (y-up, bp): the frame transform, then the page's flip.
    fn ctm6(&self, ts: Transform) -> [f64; 6] {
        [
            ts.sx.get(),
            -ts.ky.get(),
            ts.kx.get(),
            -ts.sy.get(),
            ts.tx.to_pt(),
            self.h - ts.ty.to_pt(),
        ]
    }

    fn matrix(&mut self, m: [f64; 6]) -> u32 {
        if m == [1.0, 0.0, 0.0, 1.0, 0.0, 0.0] {
            return 0;
        }
        let key = m.map(f64::to_bits);
        if let Some(&n) = self.matrices.get(&key) {
            return n;
        }
        self.page.matrices.push(m);
        let n = self.page.matrices.len() as u32;
        self.matrices.insert(key, n);
        n
    }

    fn push_path(&mut self, p: Path) -> u32 {
        self.page.paths.push(p);
        self.page.paths.len() as u32 - 1
    }

    fn unsupported(&mut self, what: &str) {
        self.page.flags |= 1;
        let n = match self.unsupported.get(what) {
            Some(&n) => n,
            None => {
                self.page.unsupported.push(what.to_string());
                let n = self.page.unsupported.len() as u32 - 1;
                self.unsupported.insert(what.to_string(), n);
                n
            }
        };
        self.page.items.push(Item::Unsupported(n));
    }

    /// A solid paint's components as the PDF writes them (typst-pdf
    /// paint.rs: luma → gray, CMYK → CMYK, everything else → sRGB, each
    /// quantised to u8), or `None` (and an UNSUPPORTED entry) for what v3
    /// cannot draw.
    fn paint(&mut self, p: &Paint) -> Option<Vec<f64>> {
        match p {
            Paint::Solid(TColor::Process(c)) => {
                let (comps, alpha): (Vec<u8>, u8) = match c.space() {
                    ProcessColorSpace::D65Gray => {
                        let v = c.to_space(ProcessColorSpace::D65Gray).to_vec4_u8();
                        (vec![v[0]], v[3])
                    }
                    ProcessColorSpace::Cmyk => (
                        c.to_space(ProcessColorSpace::Cmyk).to_vec4_u8().to_vec(),
                        255,
                    ),
                    _ => {
                        let v = c.to_space(ProcessColorSpace::Srgb).to_vec4_u8();
                        (v[..3].to_vec(), v[3])
                    }
                };
                if alpha != 255 && !self.draws_e3() {
                    self.unsupported("alpha (display-list-v3.3 E3)");
                }
                Some(comps.iter().map(|&b| b as f64 / 255.0).collect())
            }
            // The PDF's Separation colour replaces this (spec §11.3).
            Paint::Solid(TColor::Spot(_)) if self.draws_e3() => Some(vec![]),
            Paint::Solid(TColor::Spot(_)) => {
                self.unsupported("separation colour (display-list-v3.3 E3)");
                None
            }
            Paint::Gradient(_) => {
                self.unsupported("gradient (display-list-v3.3 E5 island)");
                None
            }
            Paint::Tiling(_) => {
                self.unsupported("tiling (display-list-v3.3 E5 island)");
                None
            }
        }
    }

    fn set_fill(&mut self, c: Vec<f64>) {
        self.set_fill_item(Item::FillColor(Color(c)));
    }

    fn set_stroke(&mut self, c: Vec<f64>) {
        self.set_stroke_item(Item::StrokeColor(Color(c)));
    }

    fn set_fill_item(&mut self, it: Item) {
        if self.fill.as_ref() != Some(&it) {
            self.page.items.push(it.clone());
            self.fill = Some(it);
        }
    }

    fn set_stroke_item(&mut self, it: Item) {
        if self.stroke.as_ref() != Some(&it) {
            self.page.items.push(it.clone());
            self.stroke = Some(it);
        }
    }

    /// Colour spaces and alpha are drawn: the client accepts `color-spaces`
    /// and the colours come from the PDF.
    fn draws_e3(&self) -> bool {
        self.caps.color_spaces && self.pdf.is_some()
    }

    /// Set the fill colour and alpha the PDF paints with (spec §11.3);
    /// `false` (and an UNSUPPORTED entry) when the client cannot draw them.
    fn pdf_fill(&mut self, p: &pdfpos::Paint) -> bool {
        let Some(it) = self.color_item(&p.fill, false) else {
            return false;
        };
        self.set_fill_item(it);
        self.set_alpha(p.fill_alpha, false)
    }

    /// The same for the stroke colour and alpha.
    fn pdf_stroke(&mut self, p: &pdfpos::Paint) -> bool {
        let Some(it) = self.color_item(&p.stroke, true) else {
            return false;
        };
        self.set_stroke_item(it);
        self.set_alpha(p.stroke_alpha, true)
    }

    fn set_alpha(&mut self, a: f64, stroke: bool) -> bool {
        let cur = if stroke {
            self.stroke_alpha
        } else {
            self.fill_alpha
        };
        if a == cur {
            return true;
        }
        if !self.caps.color_spaces {
            self.unsupported("alpha (display-list-v3.3 E3)");
            return true;
        }
        if stroke {
            self.stroke_alpha = a;
            self.page.items.push(Item::StrokeAlpha(a));
        } else {
            self.fill_alpha = a;
            self.page.items.push(Item::FillAlpha(a));
        }
        true
    }

    /// The colour item for a PDF colour: the Device spaces as FILL_COLOR;
    /// ICCBased as FILL_COLOR_CS for a client that accepts `color-spaces`,
    /// else as the Device space of its component count (spec §11.3);
    /// Separation only as FILL_COLOR_CS; patterns not at all (`None`, with
    /// an UNSUPPORTED entry).
    fn color_item(&mut self, c: &PdfColor, stroke: bool) -> Option<Item> {
        let device = |comps: &[f64]| {
            let col = Color(comps.to_vec());
            if stroke {
                Item::StrokeColor(col)
            } else {
                Item::FillColor(col)
            }
        };
        let in_space = |cs: u32, comps: &[f64]| {
            let color = Color(comps.to_vec());
            if stroke {
                Item::StrokeColorCs { cs, color }
            } else {
                Item::FillColorCs { cs, color }
            }
        };
        match &c.space {
            pdfpos::Space::Gray | pdfpos::Space::Rgb | pdfpos::Space::Cmyk
                if matches!(c.comps.len(), 1 | 3 | 4) =>
            {
                Some(device(&c.comps))
            }
            pdfpos::Space::Icc { n, .. } if c.comps.len() == *n as usize => {
                if self.caps.color_spaces {
                    let cs = self.intern_space(&c.space)?;
                    Some(in_space(cs, &c.comps))
                } else {
                    Some(device(&c.comps))
                }
            }
            pdfpos::Space::Separation { .. } if self.caps.color_spaces && c.comps.len() == 1 => {
                match self.intern_space(&c.space) {
                    Some(cs) => Some(in_space(cs, &c.comps)),
                    None => {
                        self.unsupported("separation colour (display-list-v3.3 E3)");
                        None
                    }
                }
            }
            pdfpos::Space::Separation { .. } => {
                self.unsupported("separation colour (display-list-v3.3 E3)");
                None
            }
            pdfpos::Space::Pattern => {
                self.unsupported("pattern (display-list-v3.3 E5 island)");
                None
            }
            other => {
                let m = format!("colour space {other:?}");
                self.unsupported(&m);
                None
            }
        }
    }

    /// The page's COLORSPACES number for a space (1-based; spec §11.3).
    fn intern_space(&mut self, sp: &pdfpos::Space) -> Option<u32> {
        let cs = match sp {
            pdfpos::Space::Icc { n, profile } => page::ColorSpace::Icc {
                n: *n,
                profile: profile.as_ref().clone(),
            },
            pdfpos::Space::Separation {
                name,
                alternate,
                c0,
                c1,
                e,
            } => {
                let alt = match alternate.as_ref() {
                    pdfpos::Space::Gray => page::alternate::DEVICE_GRAY,
                    pdfpos::Space::Rgb => page::alternate::DEVICE_RGB,
                    pdfpos::Space::Cmyk => page::alternate::DEVICE_CMYK,
                    a @ pdfpos::Space::Icc { .. } => self.intern_space(a)?,
                    _ => return None,
                };
                if c0.len() != c1.len() {
                    return None;
                }
                page::ColorSpace::Separation {
                    name: name.clone(),
                    alternate: alt,
                    c0: c0.clone(),
                    c1: c1.clone(),
                    e: *e,
                }
            }
            _ => return None,
        };
        if let Some(i) = self.page.colorspaces.iter().position(|x| *x == cs) {
            return Some(i as u32 + 1);
        }
        self.page.colorspaces.push(cs);
        Some(self.page.colorspaces.len() as u32)
    }

    fn save(&mut self) {
        self.stack.push((
            self.fill.clone(),
            self.stroke.clone(),
            self.text_render,
            self.fill_alpha,
            self.stroke_alpha,
            self.line.clone(),
        ));
        self.page.items.push(Item::Save);
    }

    fn restore(&mut self) {
        let (f, s, t, fa, sa, l) = self.stack.pop().expect("balanced save/restore");
        self.fill = f;
        self.stroke = s;
        self.text_render = t;
        self.fill_alpha = fa;
        self.stroke_alpha = sa;
        self.line = l;
        self.page.items.push(Item::Restore);
    }

    /// Emit a SPAN item when the span changes; declare it in SOURCES when the
    /// client does not know it (or it moved). Returns the glyph column of
    /// byte `offset` into the span (0xFFFF when unknown).
    fn set_span(&mut self, span: Span, offset: Option<u16>) -> u16 {
        let pos = self.resolve(span);
        let id = pos.map(|p| p.span_id).unwrap_or(0);
        if id != self.span {
            self.span = id;
            self.page.items.push(Item::Span(id));
        }
        let (Some(p), Some(off)) = (pos, offset) else {
            return page::NO_COLUMN;
        };
        // The 0-based byte column (spec §5.3). A glyph made from a span that
        // runs past its first line (rare in text) has no column.
        let byte = p.start + off as usize;
        match byte.checked_sub(p.line_start) {
            Some(c) if byte < p.line_end && c < page::NO_COLUMN as usize => c as u16,
            _ => page::NO_COLUMN,
        }
    }

    fn resolve(&mut self, span: Span) -> Option<SpanPos> {
        if span.is_detached() {
            return None;
        }
        if let Some(c) = self.tables.span_cache.get(&span) {
            return *c;
        }
        let r = self.resolve_uncached(span);
        self.tables.span_cache.insert(span, r);
        r
    }

    fn resolve_uncached(&mut self, span: Span) -> Option<SpanPos> {
        let fid = span.id()?;
        let path = self.world.path_of(fid)?;
        let range = self.world.range(span)?;
        let src = self.world.source(fid).ok()?;
        let line0 = src.lines().byte_to_line(range.start)?;
        let line_range = src.lines().line_to_range(line0)?;
        let line = line0 as u32 + 1;
        let t = &mut *self.tables;
        let file = match t.files.get(&fid) {
            Some(&f) => f,
            None => {
                let f = t.files.len() as u32 + 1;
                t.files.insert(fid, f);
                t.files_sent.push(false);
                f
            }
        };
        if !t.files_sent[file as usize - 1] {
            t.files_sent[file as usize - 1] = true;
            self.sources_out
                .files
                .push((file, path.to_string_lossy().into_owned()));
        }
        let span_id = match t.spans.get(&span) {
            Some(&s) => s,
            None => {
                let s = t.spans.len() as u32 + 1;
                t.spans.insert(span, s);
                t.span_lines.push(None);
                s
            }
        };
        let slot = &mut t.span_lines[span_id as usize - 1];
        if *slot != Some((file, line)) {
            *slot = Some((file, line));
            self.sources_out.spans.push((span_id, file, line));
        }
        Some(SpanPos {
            span_id,
            start: range.start,
            line_start: line_range.start,
            line_end: line_range.end,
        })
    }

    /// The connection's id for a font instance, queueing its FONT frame the
    /// first time a page uses it. Each font *program* is hashed, held and
    /// sent once per connection: a later instance of the same program (other
    /// variation coordinates, DESIGN.md §15.4 E1) gets a FONT frame with an
    /// empty program and `program_from`, the id whose frame carried it.
    /// `None` (and [`Walker::error`]) past [`MAX_FONTS`] ids: a u16 id must
    /// never wrap onto another font.
    fn font(&mut self, fi: &FontInstance) -> Option<u16> {
        let t = &mut *self.tables;
        let id = match t.fonts.get(fi) {
            Some(&id) => id,
            None => {
                let limit = t.font_limit.unwrap_or(MAX_FONTS);
                let id = match u16::try_from(t.font_list.len()) {
                    Ok(id) if (id as usize) < limit => id,
                    _ => {
                        if self.error.is_none() {
                            self.error = Some(format!(
                                "this document uses more than {limit} font instances (fonts times \
                                 variation coordinates), the most one display-list connection can \
                                 address; use fewer distinct `text(variations: ..)` or weight values"
                            ));
                        }
                        return None;
                    }
                };
                let font = fi.font();
                let (program, program_sha) = match t.program_by_font.get(font) {
                    Some(&p) => p,
                    None => {
                        let sha = sha256(font.data().as_slice());
                        let programs = &mut t.programs;
                        let idx = *t.program_by_sha.entry(sha).or_insert_with(|| {
                            programs.push(Program {
                                data: font.data().clone(),
                                sent_with: None,
                            });
                            programs.len() - 1
                        });
                        t.program_by_font.insert(font.clone(), (idx, sha));
                        (idx, sha)
                    }
                };
                let program_len = t.programs[program].data.len();
                let vars: Vec<([u8; 4], f32)> = fi
                    .variations()
                    .0
                    .iter()
                    .map(|(tag, v)| (tag.to_bytes(), v.0))
                    .collect();
                let key = flashtex_display_list::resource::opentype_font_key(
                    &program_sha,
                    font.index(),
                    &vars,
                );
                let file = self
                    .world
                    .font_file(font)
                    .map(|(p, _)| p.to_string_lossy().into_owned());
                let upem = fi.units_per_em();
                let outlines =
                    if fi.ttf().tables().cff.is_some() || fi.ttf().tables().cff2.is_some() {
                        "cff"
                    } else {
                        "truetype"
                    };
                let info = Json::Obj(vec![
                    ("format".into(), Json::Str("opentype".into())),
                    (
                        "ps_name".into(),
                        font.post_script_name().map(Json::Str).unwrap_or(Json::Null),
                    ),
                    ("family".into(), Json::Str(font.info().family.clone())),
                    ("file".into(), file.map(Json::Str).unwrap_or(Json::Null)),
                    ("face_index".into(), Json::Int(font.index() as i64)),
                    ("units_per_em".into(), Json::Num(upem)),
                    (
                        "font_matrix".into(),
                        Json::Str(format!("{} 0 0 {} 0 0", fmt(1.0 / upem), fmt(1.0 / upem))),
                    ),
                    ("outlines".into(), Json::Str(outlines.into())),
                    ("glyph_ids".into(), Json::Bool(true)),
                    (
                        "variations".into(),
                        Json::Arr(
                            vars.iter()
                                .map(|(tag, v)| {
                                    Json::Arr(vec![
                                        Json::Str(String::from_utf8_lossy(tag).into_owned()),
                                        Json::Num(*v as f64),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                    (
                        "program_sha256".into(),
                        Json::Str(flashtex_display_list::sha256::hex(&program_sha)),
                    ),
                    ("program_bytes".into(), Json::Int(program_len as i64)),
                ]);
                t.font_list.push(FontEntry { key, info, program });
                t.fonts_sent.push(false);
                t.fonts.insert(fi.clone(), id);
                id
            }
        };
        if !t.fonts_sent[id as usize] {
            t.fonts_sent[id as usize] = true;
            let e = &t.font_list[id as usize];
            let hex = flashtex_display_list::sha256::hex(&e.key);
            let takes_programs = self.caps.minor >= 3 && self.caps.opentype_programs;
            let prog = &mut t.programs[e.program];
            let mut info = e.info.clone();
            let program = if !takes_programs || self.have_fonts.contains(&hex) {
                vec![]
            } else if let (true, Some(from)) = (self.caps.program_refs, prog.sent_with) {
                // 3.3, accepted (spec §11.1): the program is the one FONT `from` carried.
                if let Json::Obj(kv) = &mut info {
                    kv.push(("program_from".into(), Json::Int(from as i64)));
                }
                vec![]
            } else {
                // The whole program: the first time, or for every instance
                // to a client without PROGRAM_REFS -- bounded per compile.
                let len = prog.data.len() as u64;
                if let Some(budget) = self.caps.program_budget {
                    if t.program_bytes + len > budget {
                        if self.error.is_none() {
                            self.error = Some(format!(
                                "the font programs of this compile exceed {budget} bytes \
                                 (one per font instance: {} instances so far); a client that \
                                 lists `{PROGRAM_REFS}` in its HELLO receives each program once",
                                t.font_list.len()
                            ));
                        }
                        return None;
                    }
                }
                t.program_bytes += len;
                prog.sent_with.get_or_insert(id);
                prog.data.as_slice().to_vec()
            };
            let res = FontRes {
                id,
                key: e.key,
                info,
                program,
            };
            self.fonts_out.push(res.encode());
        }
        Some(id)
    }
}

/// krilla paints nothing for this geometry (content.rs `draw_path`): a
/// zero-size path; or, unstroked, a path of zero width or height, or a
/// single line (its fill would be empty).
fn krilla_skips(segs: &[Seg], stroked: bool) -> bool {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    let mut pts = |x: f64, y: f64| {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    };
    for s in segs {
        match *s {
            Seg::Move(x, y) | Seg::Line(x, y) => pts(x, y),
            Seg::Curve(a, b, c, d, e, f) => {
                pts(a, b);
                pts(c, d);
                pts(e, f);
            }
            Seg::Close => {}
        }
    }
    if segs.is_empty() || x0 > x1 {
        return true;
    }
    let (w, h) = ((x1 - x0) as f32, (y1 - y0) as f32);
    let is_line = segs.len() == 2 && matches!(segs, [Seg::Move(..), Seg::Line(..)]);
    (w == 0.0 && h == 0.0) || (!stroked && (w == 0.0 || h == 0.0 || is_line))
}

fn curve(c: &Curve) -> Vec<Seg> {
    c.0.iter()
        .map(|it| match it {
            CurveItem::Move(p) => Seg::Move(p.x.to_pt(), p.y.to_pt()),
            CurveItem::Line(p) => Seg::Line(p.x.to_pt(), p.y.to_pt()),
            CurveItem::Cubic(a, b, c) => Seg::Curve(
                a.x.to_pt(),
                a.y.to_pt(),
                b.x.to_pt(),
                b.y.to_pt(),
                c.x.to_pt(),
                c.y.to_pt(),
            ),
            CurveItem::Close => Seg::Close,
        })
        .collect()
}

fn stroke_params(s: &FixedStroke) -> Stroke {
    let (dash, phase) = match &s.dash {
        Some(d) => (d.array.iter().map(|a| a.to_pt()).collect(), d.phase.to_pt()),
        None => (vec![], 0.0),
    };
    Stroke {
        width: s.thickness.to_pt(),
        cap: match s.cap {
            LineCap::Butt => 0,
            LineCap::Round => 1,
            LineCap::Square => 2,
        },
        join: match s.join {
            LineJoin::Miter => 0,
            LineJoin::Round => 1,
            LineJoin::Bevel => 2,
        },
        miter: s.miter_limit.get(),
        dash,
        phase,
    }
}

/// A number for PDF-like text fields: shortest round-trip form.
fn fmt(v: f64) -> String {
    let s = format!("{v}");
    if s == "-0" {
        "0".into()
    } else {
        s
    }
}

/// A gradient or a tiling: what v3 draws only as a PDF island (E5).
fn is_pattern(p: &Paint) -> bool {
    matches!(p, Paint::Gradient(_) | Paint::Tiling(_))
}

/// typst-pdf's `exif_transform` (typst-pdf 0.15.1 `image.rs`, Apache-2.0):
/// a JPEG is not re-encoded, so its EXIF orientation is drawn as a
/// transform of the image's box, which may swap its sides.
fn exif_transform(image: &RasterImage, size: Size) -> (Transform, Size) {
    use typst::layout::{Angle, Ratio};
    use typst::visualize::{ExchangeFormat, RasterFormat};
    if image.format() != RasterFormat::Exchange(ExchangeFormat::Jpg) {
        return (Transform::identity(), size);
    }
    let base = |hp: bool, vp: bool, mut base_ts: Transform, size: Size| {
        if hp {
            base_ts = base_ts.pre_concat(
                Transform::scale(-Ratio::one(), Ratio::one())
                    .pre_concat(Transform::translate(-size.x, Abs::zero())),
            )
        }
        if vp {
            base_ts = base_ts.pre_concat(
                Transform::scale(Ratio::one(), -Ratio::one())
                    .pre_concat(Transform::translate(Abs::zero(), -size.y)),
            )
        }
        base_ts
    };
    let no_flipping = |hp: bool, vp: bool| (base(hp, vp, Transform::identity(), size), size);
    let with_flipping = |hp: bool, vp: bool| {
        let base_ts = Transform::rotate_at(Angle::deg(90.0), Abs::zero(), Abs::zero())
            .pre_concat(Transform::scale(Ratio::one(), -Ratio::one()));
        let inv_size = Size::new(size.y, size.x);
        (base(hp, vp, base_ts, inv_size), inv_size)
    };
    match image.exif_rotation() {
        Some(2) => no_flipping(true, false),
        Some(3) => no_flipping(true, true),
        Some(4) => no_flipping(false, true),
        Some(5) => with_flipping(false, false),
        Some(6) => with_flipping(false, true),
        Some(7) => with_flipping(true, true),
        Some(8) => with_flipping(true, false),
        _ => no_flipping(false, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{FontOptions, Fonts};

    /// Review fix: past the font-id limit the compile fails with a clear
    /// error instead of wrapping a u16 id onto another font. The real limit
    /// is 65,536 ids; the test lowers it to reach it with three instances.
    #[test]
    fn font_ids_past_the_limit_fail_instead_of_wrapping() {
        assert_eq!(MAX_FONTS, 65_536);
        let dir = std::env::temp_dir().join(format!("ftth-fontlimit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("fonts")).unwrap();
        for (i, data) in typst_assets::fonts().enumerate() {
            std::fs::write(dir.join("fonts").join(format!("f{i:02}.otf")), data).unwrap();
        }
        std::fs::write(
            dir.join("main.typ"),
            "#set text(font: \"Libertinus Serif\")\n#for w in (300, 500, 700) [#text(variations: (wght: w))[a] ]\n",
        )
        .unwrap();
        let fonts = Fonts::load(&FontOptions {
            paths: vec![dir.join("fonts")],
            system: false,
        });
        let world = HostWorld::new(&dir, "main.typ", &fonts).unwrap();
        let doc = typst::compile::<PagedDocument>(&world).output.unwrap();
        let caps = ClientCaps {
            minor: 2,
            opentype_programs: true,
            program_refs: true,
            program_budget: None,
            ..Default::default()
        };

        // Unlimited: every instance gets an id, and they share one program.
        let mut t = Tables::new();
        let out = page(&world, &doc, 0, &mut t, caps, &[], Positions::Frame).expect("fits");
        let n = out.fonts.len();
        assert!(
            n >= 3,
            "three wght instances (and the spaces' default): {n}"
        );
        assert_eq!(t.program_count(), 1, "{n} instances, one program");

        // One id short: a clear error, never a wrapped id.
        let mut t = Tables::with_font_limit(n - 1);
        let err = page(&world, &doc, 0, &mut t, caps, &[], Positions::Frame)
            .err()
            .expect("the limit is reached");
        assert!(
            err.contains(&format!("more than {} font instances", n - 1)),
            "{err}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
