//! Typst's laid-out pages to `display-list-v3` pages.
//!
//! The walk mirrors `typst-pdf`'s (convert.rs `handle_frame`): the page fill
//! first, then the frame translated by the bleed, groups composing their
//! transform and clip, text runs as positioned glyphs, shapes as paths. What
//! v3 (and this host's 3.2 draft) cannot express is flagged INCOMPLETE with
//! an UNSUPPORTED entry at the place it was skipped, never approximated
//! (spec §4.7): gradients and tilings (E5 islands), images (E6), alpha and
//! spot colour (E3), stroked text (E4).
//!
//! **Positions.** Typst's frame positions (f64 pt; Typst's `pt` is the PDF
//! point, bp) are carried exactly in ORIGINS_F64 and rounded once to sp for
//! the v3 GLYPH items. typst-pdf writes glyph positions through krilla in
//! f32, so the exported PDF's positions differ from these by up to about
//! 6·10⁻⁵ bp (Track A §5.2); the PDF-derived origins that make the preview
//! pixel-identical are phase T1 (DESIGN.md §15.5, §15.10).

use std::collections::HashMap;

use flashtex_display_list::json::Json;
use flashtex_display_list::page::{
    self, Color, Item, Link, LinkKind, Page, Path, Seg, StreamKind, Stroke,
};
use flashtex_display_list::resource::{Font as FontRes, Sources};
use flashtex_display_list::sha256::sha256;
use typst::layout::{Abs, Frame, FrameItem, GroupItem, Point, Size, Transform};
use typst::model::Destination;
use typst::syntax::{FileId, Span};
use typst::text::{FontInstance, TextItem};
use typst::visualize::{
    Color as TColor, Curve, CurveItem, FillRule, FixedStroke, Geometry, LineCap, LineJoin, Paint,
    ProcessColorSpace, Shape,
};
use typst::{World, WorldExt};
use typst_layout::PagedDocument;

use crate::v32;
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
    /// The negotiated minor version (1 or 2).
    pub minor: u32,
    /// `COMPILE.font_formats` lists `opentype`.
    pub opentype_programs: bool,
    /// The client's HELLO lists [`PROGRAM_REFS`]: it understands a FONT with
    /// an empty program and `program_from`. Without it every FONT that takes
    /// a program carries the whole program (spec §5.1: an empty program
    /// means the client already holds that font).
    pub program_refs: bool,
    /// At most this many font-program bytes per compile (`None`: no limit).
    pub program_budget: Option<u64>,
}

/// Draft 3.2 capability token (typst-host only, for the protocol owner): a
/// client that lists it in its HELLO `capabilities` accepts `program_from`.
pub const PROGRAM_REFS: &str = "font-program-refs";

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
    /// A SOURCES frame this page needs first, if any.
    pub sources: Option<Json>,
    /// The PAGE body.
    pub body: Vec<u8>,
    pub glyphs: usize,
    pub flags: u32,
}

/// Graphics state a SAVE scopes: fill, stroke, text render mode.
type Saved = (Option<Vec<f64>>, Option<Vec<f64>>, u8);

struct Walker<'a, 'w> {
    world: &'a HostWorld<'w>,
    doc: &'a PagedDocument,
    tables: &'a mut Tables,
    caps: ClientCaps,
    have_fonts: &'a [String],
    /// The first error that stops the compile (the font-id limit).
    error: Option<String>,
    /// Page height in bp (stream space's y flip).
    h: f64,
    page: Page,
    matrices: HashMap<[u64; 6], u32>,
    unsupported: HashMap<String, u32>,
    origins: Vec<(f64, f64)>,
    fonts_out: Vec<Vec<u8>>,
    sources_out: Sources,
    fill: Option<Vec<f64>>,
    stroke: Option<Vec<f64>>,
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
) -> Result<PageOut, String> {
    let tp = &doc.pages()[index];
    let size = tp.frame.size() + tp.bleed.sum_by_axis();
    let (w, h) = (size.x.to_pt(), size.y.to_pt());
    let mut p = Page::new(StreamKind::Page, index as u32);
    p.width = sp(w);
    p.height = sp(h);
    p.pdf_box = [0.0, 0.0, w, h];
    let mut wk = Walker {
        world,
        doc,
        tables,
        caps,
        have_fonts,
        h,
        page: p,
        matrices: HashMap::new(),
        unsupported: HashMap::new(),
        origins: Vec::new(),
        fonts_out: Vec::new(),
        sources_out: Sources::default(),
        fill: None,
        stroke: None,
        text_render: 0,
        glyph_matrix: None,
        span: 0,
        stack: Vec::new(),
        error: None,
    };
    if let Some(fill) = tp.fill_or_transparent() {
        let shape = Geometry::Rect(size).filled(fill);
        wk.shape(&shape, Span::detached(), Transform::identity());
    }
    let ts = Transform::translate(tp.bleed.left, tp.bleed.top);
    wk.frame(&tp.frame, ts);

    // A 3.1 client cannot draw OpenType glyph ids (v3.1 refuses `opentype`
    // without the capability): it gets the page INCOMPLETE and renders
    // DONE.pdf, as DESIGN.md §15.4 specifies.
    let glyphs = wk.origins.len();
    if glyphs > 0 && (wk.caps.minor < 2 || !wk.caps.opentype_programs) {
        wk.unsupported("opentype glyphs (display-list-v3.2 E1 and font_formats opentype)");
    }

    let mut page = wk.page;
    let font_list = &wk.tables.font_list;
    let v3_hash = page.content_hash(
        &|id| font_list.get(id as usize).map(|f| f.key).unwrap_or([0; 32]),
        &|_| [0; 32],
    );
    let mut extra = Vec::new();
    if wk.caps.minor >= 2 {
        extra.push((v32::tag::ORIGINS_F64, v32::encode_origins(&wk.origins)));
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
        extra.push((v32::tag::PAGE_META, meta.to_string().into_bytes()));
        page.hash = v32::extended_hash(v3_hash, &extra);
    } else {
        page.hash = v3_hash;
    }
    let mut body = page.encode();
    v32::append_sections(&mut body, &extra);
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
        sources,
        body,
        glyphs,
        flags: page.flags,
    })
}

impl Walker<'_, '_> {
    fn frame(&mut self, frame: &Frame, ts: Transform) {
        for (pos, item) in frame.items() {
            let ts = ts.pre_concat(Transform::translate(pos.x, pos.y));
            match item {
                FrameItem::Group(g) => self.group(g, ts),
                FrameItem::Text(t) => self.text(t, ts),
                FrameItem::Shape(s, span) => self.shape(s, *span, ts),
                FrameItem::Image(_, _, span) => {
                    self.set_span(*span, None);
                    self.unsupported("image (display-list-v3.2 E6)");
                }
                FrameItem::Link(dest, size) => self.link(dest, *size, ts),
                FrameItem::Tag(_) => {}
            }
        }
    }

    fn group(&mut self, g: &GroupItem, ts: Transform) {
        let ts = ts.pre_concat(g.transform);
        if let Some(clip) = &g.clip {
            self.save();
            let m = self.ctm(ts);
            let n = self.push_path(Path {
                paint: page::paint::CLIP,
                matrix: m,
                stroke: None,
                segs: curve(clip),
            });
            self.page.items.push(Item::Clip(n));
            self.frame(&g.frame, ts);
            self.restore();
        } else {
            self.frame(&g.frame, ts);
        }
    }

    fn text(&mut self, t: &TextItem, ts: Transform) {
        let Some(fill) = self.paint(&t.fill) else {
            return;
        };
        let Some(font) = self.font(&t.font) else {
            return;
        };
        if let Some(s) = &t.stroke {
            // v3 has no text line width (E4): the fill is exact, the stroke is not drawn.
            let _ = s;
            self.unsupported("stroked text (display-list-v3.2 E4)");
        }
        self.set_fill(fill);
        if self.text_render != 0 {
            self.text_render = 0;
            self.page.items.push(Item::TextRender(0));
        }
        let s = t.size.to_pt();
        let (sx, ky, kx, sy) = (ts.sx.get(), ts.ky.get(), ts.kx.get(), ts.sy.get());
        let m = self.matrix([s * sx, -s * ky, -s * kx, s * sy, 0.0, 0.0]);
        if self.glyph_matrix != Some(m) {
            self.glyph_matrix = Some(m);
            self.page.items.push(Item::Matrix(m));
        }
        let (mut x, mut y) = (Abs::zero(), Abs::zero());
        for g in &t.glyphs {
            let gx = x + g.x_offset.at(t.size);
            let gy = y - g.y_offset.at(t.size);
            let p = Point::new(gx, gy).transform(ts);
            let col = self.set_span(g.span.0, Some(g.span.1));
            let (px, py) = (p.x.to_pt(), p.y.to_pt());
            self.origins.push((px, self.h - py));
            self.page.items.push(Item::Glyph {
                font,
                code: g.id,
                x: sp(px),
                y: sp(py),
                col,
            });
            x += g.x_advance.at(t.size);
            y -= g.y_advance.at(t.size);
        }
    }

    fn shape(&mut self, s: &Shape, span: Span, ts: Transform) {
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
        let fill = s.fill.as_ref().map(|p| self.paint(p));
        let stroke = s.stroke.as_ref().filter(|st| st.thickness.to_pt() > 0.0);
        let stroke_paint = stroke.map(|st| self.paint(&st.paint));
        let fill = fill.flatten();
        let stroke_color = stroke_paint.flatten();
        let mut paint = 0;
        if let Some(c) = fill {
            self.set_fill(c);
            paint |= match s.fill_rule {
                FillRule::NonZero => page::paint::FILL,
                FillRule::EvenOdd => page::paint::FILL_EVEN_ODD,
            };
        }
        let mut st = None;
        if let (Some(c), Some(fs)) = (stroke_color, stroke) {
            self.set_stroke(c);
            paint |= page::paint::STROKE;
            st = Some(stroke_params(fs));
        }
        if paint == 0 {
            return;
        }
        self.set_span(span, None);
        let m = self.ctm(ts);
        let n = self.push_path(Path {
            paint,
            matrix: m,
            stroke: st,
            segs,
        });
        self.page.items.push(Item::Path(n));
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
    fn ctm(&mut self, ts: Transform) -> u32 {
        self.matrix([
            ts.sx.get(),
            -ts.ky.get(),
            ts.kx.get(),
            -ts.sy.get(),
            ts.tx.to_pt(),
            self.h - ts.ty.to_pt(),
        ])
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
                if alpha != 255 {
                    self.unsupported("alpha (display-list-v3.2 E3)");
                }
                Some(comps.iter().map(|&b| b as f64 / 255.0).collect())
            }
            Paint::Solid(TColor::Spot(_)) => {
                self.unsupported("separation colour (display-list-v3.2 E3)");
                None
            }
            Paint::Gradient(_) => {
                self.unsupported("gradient (display-list-v3.2 E5 island)");
                None
            }
            Paint::Tiling(_) => {
                self.unsupported("tiling (display-list-v3.2 E5 island)");
                None
            }
        }
    }

    fn set_fill(&mut self, c: Vec<f64>) {
        if self.fill.as_ref() != Some(&c) {
            self.page.items.push(Item::FillColor(Color(c.clone())));
            self.fill = Some(c);
        }
    }

    fn set_stroke(&mut self, c: Vec<f64>) {
        if self.stroke.as_ref() != Some(&c) {
            self.page.items.push(Item::StrokeColor(Color(c.clone())));
            self.stroke = Some(c);
        }
    }

    fn save(&mut self) {
        self.stack
            .push((self.fill.clone(), self.stroke.clone(), self.text_render));
        self.page.items.push(Item::Save);
    }

    fn restore(&mut self) {
        let (f, s, t) = self.stack.pop().expect("balanced save/restore");
        self.fill = f;
        self.stroke = s;
        self.text_render = t;
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
                let vars: Vec<(String, f32)> = fi
                    .variations()
                    .0
                    .iter()
                    .map(|(tag, v)| (String::from_utf8_lossy(&tag.to_bytes()).into_owned(), v.0))
                    .collect();
                let key = v32::opentype_font_key(&program_sha, font.index(), &vars);
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
                                    Json::Arr(vec![Json::Str(tag.clone()), Json::Num(*v as f64)])
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
            let takes_programs = self.caps.minor >= 2 && self.caps.opentype_programs;
            let prog = &mut t.programs[e.program];
            let mut info = e.info.clone();
            let program = if !takes_programs || self.have_fonts.contains(&hex) {
                vec![]
            } else if let (true, Some(from)) = (self.caps.program_refs, prog.sent_with) {
                // 3.2 draft, opted in: the program is the one FONT `from` carried.
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
        };

        // Unlimited: every instance gets an id, and they share one program.
        let mut t = Tables::new();
        let out = page(&world, &doc, 0, &mut t, caps, &[]).expect("fits");
        let n = out.fonts.len();
        assert!(
            n >= 3,
            "three wght instances (and the spaces' default): {n}"
        );
        assert_eq!(t.program_count(), 1, "{n} instances, one program");

        // One id short: a clear error, never a wrapped id.
        let mut t = Tables::with_font_limit(n - 1);
        let err = page(&world, &doc, 0, &mut t, caps, &[])
            .err()
            .expect("the limit is reached");
        assert!(
            err.contains(&format!("more than {} font instances", n - 1)),
            "{err}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
