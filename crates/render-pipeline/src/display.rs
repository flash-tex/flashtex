//! Display list v2: the pipeline's authoritative output, shaped after
//! `protocol/rendering-v2.schema.json` and `crates/rendering-core`'s model.
//!
//! Coordinates are `bp_2pow20` ticks (integer, 1 048 576 per PDF point,
//! y downward from the page's top-left corner). Every glyph carries its
//! ORIGINAL glyph id and a cluster index; every cluster carries its byte
//! range inside the run's `text` (the ActualText), exact hit rectangles
//! and source ranges with the document path. Rules are explicit rectangles.
//! Deviations from the schema are listed in `docs/proposals/rendering-abi.md`
//! and in the README, never hidden: this pipeline emits
//! `format: "opentype-cff"` for Latin Modern and `core14-afm` (no bytes) for
//! Times, and `gid: 0` never appears (missing glyphs are diagnostics).

use flashtex_compiler::json::{self, Value};

pub const PROTOCOL_VERSION: i64 = 2;
pub const TICKS_PER_BP: f64 = 1_048_576.0;
/// 1 TeX point in PDF points (big points): 72/72.27.
pub const BP_PER_TEX_PT: f64 = 72.0 / 72.27;

/// Integer ticks, 2^20 per PDF point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Tick(pub i64);

impl Tick {
    /// Ticks from a length in TeX points (72.27/in). The single rounding
    /// point of the pipeline: layout is f64 TeX points, output is exact
    /// integers, so equal inputs give equal outputs.
    pub fn from_tex_pt(pt: f64) -> Tick {
        Tick((pt * BP_PER_TEX_PT * TICKS_PER_BP).round() as i64)
    }
    /// Ticks from PDF points (used for page sizes, which LaTeX declares in
    /// TeX points but consumers expect as 612x792 bp for US Letter).
    pub fn from_bp(bp: f64) -> Tick {
        Tick((bp * TICKS_PER_BP).round() as i64)
    }
    pub fn to_bp(self) -> f64 {
        self.0 as f64 / TICKS_PER_BP
    }
}

/// A byte range in one source document. `path` is shared: a page carries
/// one range per cluster, so the string is reference-counted rather than
/// copied a hundred thousand times per compile.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceRange {
    pub path: std::rc::Rc<str>,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Paint {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
    /// PROPOSAL (`protocol/proposals/display-list-v2-device-color.md`): the
    /// colour exactly as pdfTeX writes it (`k`, `rg`, `g` operands); `r`,
    /// `g`, `b` are then its naive sRGB preview. `None`: the default colour.
    pub device: Option<flashtex_compiler::color::DeviceColor>,
}

impl Paint {
    pub const BLACK: Paint = Paint {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
        device: None,
    };

    /// The paint of a compiler colour (`None`: black, the default).
    pub fn of(color: Option<flashtex_compiler::color::DeviceColor>) -> Paint {
        match color {
            None => Paint::BLACK,
            Some(device) => {
                let (r, g, b) = device.to_rgb();
                Paint { r, g, b, a: 1.0, device: Some(device) }
            }
        }
    }
}

/// Negotiated display-list proposals: image items (FT-063) and device
/// colours (`display-list-v2-device-color`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Wire {
    pub images: bool,
    pub device_color: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: Tick,
    pub top: Tick,
    pub width: Tick,
    pub height: Tick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caret {
    pub text_byte: usize,
    pub x: Tick,
    pub top: Tick,
    pub height: Tick,
}

/// Where a cluster's bytes came from: exact source ranges, or a stated
/// reason when the pipeline synthesised it.
#[derive(Debug, Clone, PartialEq)]
pub enum Provenance {
    /// One source range (the common case; no allocation per cluster).
    Source(SourceRange),
    /// Several ranges (macro expansion); reserved, unused today.
    Sources(Vec<SourceRange>),
    Synthetic(String),
}

impl Provenance {
    /// The source ranges, in order (empty for synthetic content).
    pub fn sources(&self) -> &[SourceRange] {
        match self {
            Provenance::Source(s) => std::slice::from_ref(s),
            Provenance::Sources(v) => v,
            Provenance::Synthetic(_) => &[],
        }
    }
}

/// One or two carets per cluster (its start, and the run end on the last
/// cluster), stored inline: a page carries a caret pair per cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Carets {
    pub first: Caret,
    pub last: Option<Caret>,
}

impl Carets {
    pub fn iter(&self) -> impl Iterator<Item = &Caret> {
        std::iter::once(&self.first).chain(self.last.iter())
    }
    pub fn len(&self) -> usize {
        1 + usize::from(self.last.is_some())
    }
    pub fn is_empty(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    pub text_start_byte: usize,
    pub text_end_byte: usize,
    /// The cluster's hit rectangle (the wire format is a list; this
    /// pipeline emits exactly one per cluster).
    pub hit_rect: Rect,
    pub carets: Carets,
    pub provenance: Provenance,
}

impl Cluster {
    pub fn hit_rects(&self) -> &[Rect] {
        std::slice::from_ref(&self.hit_rect)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph {
    /// Original glyph id in the run's font. Never 0.
    pub gid: u16,
    pub origin_x: Tick,
    pub baseline_y: Tick,
    pub advance_x: Tick,
    pub advance_y: Tick,
    pub cluster: u32,
}

/// What a run is, for the v1 fallback (not on the v2 wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunRole {
    /// A word segment: one v1 text item.
    Text,
    /// Math glyphs with individual positions: one v1 text item per glyph.
    Math,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlyphRun {
    /// Content-addressed font id (SHA-256 hex of the program).
    pub font_id: std::rc::Rc<str>,
    pub font_size: Tick,
    /// ActualText of the whole run; clusters partition it.
    pub text: String,
    pub glyphs: Vec<Glyph>,
    pub clusters: Vec<Cluster>,
    pub paint: Paint,
    pub role: RunRole,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub x: Tick,
    pub top: Tick,
    pub width: Tick,
    pub height: Tick,
    pub paint: Paint,
    pub provenance: Provenance,
}

/// One vector path command, in ticks (top-left origin, y down). Proposal
/// `path-v0` (`docs/proposals/display-list-paths.md`), not in the frozen
/// rendering-v2 schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathCmd {
    Move(Tick, Tick),
    Line(Tick, Tick),
    /// Cubic Bézier: first control, second control, end.
    Cubic(Tick, Tick, Tick, Tick, Tick, Tick),
    Close,
}

impl PathCmd {
    /// The command with every y coordinate mapped by `f`.
    pub fn map_x(self, f: &dyn Fn(Tick) -> Tick) -> PathCmd {
        match self {
            PathCmd::Move(x, y) => PathCmd::Move(f(x), y),
            PathCmd::Line(x, y) => PathCmd::Line(f(x), y),
            PathCmd::Cubic(a, b, c, d, e, g) => PathCmd::Cubic(f(a), b, f(c), d, f(e), g),
            PathCmd::Close => PathCmd::Close,
        }
    }

    pub fn map_y(self, f: &dyn Fn(Tick) -> Tick) -> PathCmd {
        match self {
            PathCmd::Move(x, y) => PathCmd::Move(x, f(y)),
            PathCmd::Line(x, y) => PathCmd::Line(x, f(y)),
            PathCmd::Cubic(a, b, c, d, e, g) => PathCmd::Cubic(a, f(b), c, f(d), e, f(g)),
            PathCmd::Close => PathCmd::Close,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub width: Tick,
    pub cap: LineCap,
    pub join: LineJoin,
    pub miter_limit: f64,
    /// Alternating on/off lengths; empty for a solid line.
    pub dash: Vec<Tick>,
    pub dash_phase: Tick,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PathPaintOp {
    Fill { even_odd: bool },
    Stroke(Stroke),
}

/// A clip in page space; an item is visible only inside every clip.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipPath {
    pub commands: Vec<PathCmd>,
    pub even_odd: bool,
}

/// A filled or stroked vector path (TikZ pictures).
#[derive(Debug, Clone, PartialEq)]
pub struct PathItem {
    pub op: PathPaintOp,
    pub commands: Vec<PathCmd>,
    pub clips: Vec<ClipPath>,
    pub paint: Paint,
    pub provenance: Provenance,
}

/// PROPOSAL (FT-063, `protocol/proposals/display-list-v2-image.md`): one
/// image file referenced by content hash; the consumer fetches the bytes
/// through project-files by `path` and verifies `sha256`/`byte_length`.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageResource {
    /// SHA-256 hex of the file bytes (also the image id).
    pub sha256: std::rc::Rc<str>,
    pub byte_length: u64,
    /// `png`, `jpeg` or `pdf`.
    pub format: &'static str,
    /// Project-relative path the bytes were read from.
    pub path: String,
    pub pixels: Option<(u32, u32)>,
    /// PDF: 1-based page, the clip box in the page's own space, `/Rotate`.
    pub pdf_page: u32,
    pub pdf_box: Option<[f64; 4]>,
    pub pdf_rotate: i32,
}

/// PROPOSAL (FT-063): a placed image. `x/top/width/height` is the bounding
/// box; `transform` maps the image's unit square (u right, v up, (0,0) at
/// the image's lower-left) to page points, y down:
/// `page = (e + a*u + c*v, f + b*u + d*v)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub x: Tick,
    pub top: Tick,
    pub width: Tick,
    pub height: Tick,
    pub transform: [f64; 6],
    pub resource: std::rc::Rc<ImageResource>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    GlyphRun(GlyphRun),
    Rule(Rule),
    Path(PathItem),
    /// Only serialised when the request negotiated `display-list-v2-images`
    /// (see [`DisplayList::to_json_with`]).
    Image(Image),
}

/// Moves every x of `item` by `dx` (an even page's left margin, the second
/// column).
pub fn shift_x(item: &mut Item, dx: Tick) {
    let add = |t: Tick| Tick(t.0 + dx.0);
    match item {
        Item::GlyphRun(r) => {
            for g in &mut r.glyphs {
                g.origin_x = add(g.origin_x);
            }
            for c in &mut r.clusters {
                c.hit_rect.x = add(c.hit_rect.x);
                c.carets.first.x = add(c.carets.first.x);
                if let Some(l) = &mut c.carets.last {
                    l.x = add(l.x);
                }
            }
        }
        Item::Rule(rule) => rule.x = add(rule.x),
        Item::Image(image) => {
            image.x = add(image.x);
            // The transform's translation is in PDF points.
            image.transform[4] += dx.to_bp();
        }
        Item::Path(p) => {
            for c in &mut p.commands {
                *c = c.map_x(&add);
            }
            for clip in &mut p.clips {
                for c in &mut clip.commands {
                    *c = c.map_x(&add);
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub number: u32,
    pub width: Tick,
    pub height: Tick,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontResource {
    pub font_id: std::rc::Rc<str>,
    pub sha256: String,
    pub byte_length: u64,
    /// `opentype-cff`, `static-truetype` or `core14-afm`.
    pub format: String,
    pub face_index: u32,
    pub units_per_em: u32,
    pub glyph_count: u32,
    pub postscript_name: String,
    /// Not on the wire; where the bytes came from, for diagnostics/PDF.
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentResource {
    pub path: String,
    pub revision: u64,
    pub sha256: String,
    pub byte_length: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub severity: Severity,
    pub sources: Vec<SourceRange>,
    /// The compiler's recovery note, when it produced this diagnostic.
    pub recovery: Option<String>,
    /// Replacement text for the source range (runtime-v1 `suggestion`).
    /// Never serialised on display-list-v2 (`additionalProperties: false`).
    pub suggestion: Option<String>,
}

impl Diagnostic {
    pub fn error(code: &str, message: impl Into<String>, sources: Vec<SourceRange>) -> Diagnostic {
        Diagnostic {
            code: code.into(),
            message: message.into(),
            severity: Severity::Error,
            sources,
            recovery: None,
            suggestion: None,
        }
    }
    pub fn warning(code: &str, message: impl Into<String>, sources: Vec<SourceRange>) -> Diagnostic {
        Diagnostic {
            code: code.into(),
            message: message.into(),
            severity: Severity::Warning,
            sources,
            recovery: None,
            suggestion: None,
        }
    }

    /// Converts a compiler diagnostic; `paths` is indexed by `DocumentId`.
    pub fn from_compiler(d: &flashtex_compiler::diagnostics::Diagnostic, paths: &[&str]) -> Diagnostic {
        use flashtex_compiler::diagnostics::Severity as S;
        Diagnostic {
            code: "compiler".into(),
            message: d.message.clone(),
            severity: match d.severity {
                S::Error => Severity::Error,
                _ => Severity::Warning,
            },
            sources: d
                .span
                .map(|s| {
                    vec![SourceRange {
                        path: std::rc::Rc::from(paths.get(s.document.0).copied().unwrap_or("")),
                        start_byte: s.start,
                        end_byte: s.end,
                    }]
                })
                .unwrap_or_default(),
            recovery: d.recovery.clone(),
            suggestion: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayList {
    pub project_id: String,
    pub revision: u64,
    pub documents: Vec<DocumentResource>,
    pub fonts: Vec<FontResource>,
    pub pages: Vec<Page>,
    pub diagnostics: Vec<Diagnostic>,
}

impl DisplayList {
    /// An upper-bound estimate of the serialised envelope size, so a
    /// producer can decline `display-list-v2` for a request without first
    /// serialising a line it would then throw away (the exact check still
    /// runs on the serialised line when the estimate is under the limit).
    pub fn estimated_json_bytes(&self) -> usize {
        let mut n = 512 + self.fonts.len() * 400 + self.documents.len() * 200;
        for d in &self.diagnostics {
            n += 160 + d.message.len() + d.sources.len() * 80;
        }
        for p in &self.pages {
            n += 64;
            for it in &p.items {
                n += match it {
                    Item::GlyphRun(r) => 220 + 2 * r.text.len() + 120 * r.glyphs.len() + 280 * r.clusters.len(),
                    Item::Rule(_) => 240,
                    Item::Path(p) => 240 + 64 * (p.commands.len() + p.clips.iter().map(|c| c.commands.len()).sum::<usize>()),
                    Item::Image(i) => 520 + 2 * i.resource.path.len(),
                };
            }
        }
        n
    }

    pub fn required_features(&self) -> Vec<&'static str> {
        self.required_features_with(false)
    }

    /// `images`: whether image items are serialised (adds `image`).
    pub fn required_features_with(&self, images: bool) -> Vec<&'static str> {
        self.required_features_wire(Wire { images, device_color: false })
    }

    /// `device-color` is listed when negotiated and some paint carries one.
    pub fn required_features_wire(&self, wire: Wire) -> Vec<&'static str> {
        let images = wire.images;
        let mut f = vec!["glyph_run", "rgba-srgb", "cluster-actualtext"];
        if self.pages.iter().any(|p| p.items.iter().any(|i| matches!(i, Item::Rule(_)))) {
            f.insert(1, "rule");
        }
        let paths = || self.pages.iter().flat_map(|p| p.items.iter()).filter_map(|i| if let Item::Path(p) = i { Some(p) } else { None });
        if paths().any(|p| matches!(p.op, PathPaintOp::Fill { .. })) {
            f.push("path_fill");
        }
        if paths().any(|p| matches!(p.op, PathPaintOp::Stroke(_))) {
            f.push("path_stroke");
        }
        if paths().any(|p| !p.clips.is_empty()) {
            f.push("clip");
        }
        if self.fonts.iter().any(|r| r.format == "static-truetype") {
            f.push("static-truetype");
        }
        if images && self.pages.iter().any(|p| p.items.iter().any(|i| matches!(i, Item::Image(_)))) {
            f.push("image");
        }
        let device = |it: &Item| match it {
            Item::GlyphRun(r) => r.paint.device.is_some(),
            Item::Rule(r) => r.paint.device.is_some(),
            Item::Path(p) => p.paint.device.is_some(),
            Item::Image(_) => false,
        };
        if wire.device_color && self.pages.iter().any(|p| p.items.iter().any(device)) {
            f.push("device-color");
        }
        f
    }

    /// Whether any page carries an image item.
    pub fn has_images(&self) -> bool {
        self.pages.iter().any(|p| p.items.iter().any(|i| matches!(i, Item::Image(_))))
    }

    /// The `display_list` envelope of rendering-v2 as a JSON value, exactly
    /// as frozen: image items (an FT-063 proposal) are not serialised.
    pub fn to_json(&self, id: &str) -> Value {
        self.to_json_with(id, false)
    }

    /// [`to_json`](Self::to_json); `images` also serialises image items
    /// and lists the `image` feature (negotiated `display-list-v2-images`).
    pub fn to_json_with(&self, id: &str, images: bool) -> Value {
        self.to_json_wire(id, Wire { images, device_color: false })
    }

    /// [`to_json_with`](Self::to_json_with) with every negotiated proposal.
    pub fn to_json_wire(&self, id: &str, wire: Wire) -> Value {
        let mut payload = Value::obj();
        payload.set("render_format", json::str_("display-list-v2"));
        payload.set("coordinate_unit", json::str_("bp_2pow20"));
        payload.set("color_space", json::str_("srgb"));
        payload.set("text_extraction", json::str_("cluster-actualtext"));
        payload.set("project_id", json::str_(self.project_id.clone()));
        payload.set("revision", json::num(self.revision as f64));
        payload.set(
            "required_features",
            Value::Arr(self.required_features_wire(wire).into_iter().map(json::str_).collect()),
        );
        payload.set(
            "documents",
            Value::Arr(
                self.documents
                    .iter()
                    .map(|d| {
                        let mut o = Value::obj();
                        o.set("path", json::str_(d.path.clone()));
                        o.set("revision", json::num(d.revision as f64));
                        o.set("sha256", json::str_(d.sha256.clone()));
                        o.set("byte_length", json::num(d.byte_length as f64));
                        o
                    })
                    .collect(),
            ),
        );
        payload.set(
            "fonts",
            Value::Arr(
                self.fonts
                    .iter()
                    .map(|f| {
                        let mut o = Value::obj();
                        o.set("font_id", json::str_(f.font_id.to_string()));
                        o.set("sha256", json::str_(f.sha256.clone()));
                        o.set("byte_length", json::num(f.byte_length as f64));
                        o.set("format", json::str_(f.format.clone()));
                        o.set("face_index", json::num(f64::from(f.face_index)));
                        o.set("units_per_em", json::num(f64::from(f.units_per_em)));
                        o.set("glyph_count", json::num(f64::from(f.glyph_count)));
                        o.set("postscript_name", json::str_(f.postscript_name.clone()));
                        o
                    })
                    .collect(),
            ),
        );
        payload.set("pages", Value::Arr(self.pages.iter().map(|p| page_json(p, wire)).collect()));
        payload.set(
            "diagnostics",
            Value::Arr(self.diagnostics.iter().map(diagnostic_json).collect()),
        );
        let mut v = Value::obj();
        v.set("protocol_version", json::num(PROTOCOL_VERSION as f64));
        v.set("id", json::str_(id));
        v.set("type", json::str_("display_list"));
        v.set("payload", payload);
        v
    }

    /// `json::write(&self.to_json(id))` without building the `Value` tree
    /// (FT-065: at HW1 size the tree of per-glyph maps with owned keys cost
    /// more than layout). Keys are written in the `BTreeMap` order the tree
    /// serialises in and numbers/strings go through the same json writers,
    /// so the bytes are identical (`write_json_matches_the_value_tree`).
    pub fn write_json(&self, id: &str) -> String {
        self.write_json_with(id, false)
    }

    /// `json::write(&self.to_json_with(id, images))` written directly:
    /// `images` also serialises image items and lists the `image` feature.
    pub fn write_json_with(&self, id: &str, images: bool) -> String {
        self.write_json_wire(id, Wire { images, device_color: false })
    }

    /// [`write_json_with`](Self::write_json_with) with every negotiated proposal.
    pub fn write_json_wire(&self, id: &str, wire: Wire) -> String {
        self.write_envelope(id, wire, |o, _, p| write_page(o, p, wire))
    }

    /// [`write_json_wire`](Self::write_json_wire), also recording the exact
    /// byte length of every page object as written (`display-list-v2-delta`
    /// `page_bytes`).
    pub fn write_json_wire_measured(&self, id: &str, wire: Wire, page_bytes: &mut Vec<usize>) -> String {
        page_bytes.clear();
        self.write_envelope(id, wire, |o, _, p| {
            let start = o.len();
            write_page(o, p, wire);
            page_bytes.push(o.len() - start);
        })
    }

    /// The full line with each page object supplied as text (a consumer's
    /// reconstruction from base + delta; the producer gate compares it with
    /// [`write_json_wire`](Self::write_json_wire)). `pages` has one entry per
    /// page of `self`.
    pub fn write_json_wire_with_page_objects(&self, id: &str, wire: Wire, pages: &[String]) -> String {
        self.write_envelope(id, wire, |o, i, _| o.push_str(&pages[i]))
    }

    fn write_envelope(&self, id: &str, wire: Wire, mut page: impl FnMut(&mut String, usize, &Page)) -> String {
        let mut o = String::with_capacity(self.estimated_json_bytes());
        o.push_str("{\"id\":");
        json::write_string_into(id, &mut o);
        o.push_str(",\"payload\":{\"color_space\":\"srgb\",\"coordinate_unit\":\"bp_2pow20\",\"diagnostics\":");
        write_diagnostics(&mut o, &self.diagnostics);
        o.push_str(",\"documents\":");
        write_documents(&mut o, &self.documents);
        o.push_str(",\"fonts\":");
        write_fonts(&mut o, &self.fonts);
        o.push_str(",\"pages\":[");
        for (i, p) in self.pages.iter().enumerate() {
            sep(&mut o, i);
            page(&mut o, i, p);
        }
        o.push_str("],\"project_id\":");
        json::write_string_into(&self.project_id, &mut o);
        o.push_str(",\"render_format\":\"display-list-v2\",\"required_features\":");
        write_features(&mut o, self, wire);
        o.push_str(",\"revision\":");
        num(&mut o, self.revision as f64);
        o.push_str(",\"text_extraction\":\"cluster-actualtext\"},\"protocol_version\":");
        num(&mut o, PROTOCOL_VERSION as f64);
        o.push_str(",\"type\":\"display_list\"}");
        o
    }
}

/// Bytes of the full `display_list` line that are neither a page object, a
/// page separator, nor one of the measured parts (`id`, `diagnostics`,
/// `documents`, `fonts`, `project_id`, `required_features`, `revision`):
/// the fixed framing of [`DisplayList::write_json_wire`], which the delta
/// consumer's exact size formula (`DisplayListDelta.fullLineBytes`) assumes.
pub const FULL_LINE_FRAME_BYTES: usize = "{\"id\":".len()
    + ",\"payload\":{\"color_space\":\"srgb\",\"coordinate_unit\":\"bp_2pow20\",\"diagnostics\":".len()
    + ",\"documents\":".len()
    + ",\"fonts\":".len()
    + ",\"pages\":[".len()
    + "],\"project_id\":".len()
    + ",\"render_format\":\"display-list-v2\",\"required_features\":".len()
    + ",\"revision\":".len()
    + ",\"text_extraction\":\"cluster-actualtext\"},\"protocol_version\":2,\"type\":\"display_list\"}".len();

/// The `diagnostics` array of the full line (also carried complete by a delta).
pub(crate) fn write_diagnostics(o: &mut String, diagnostics: &[Diagnostic]) {
    o.push('[');
    for (i, d) in diagnostics.iter().enumerate() {
        sep(o, i);
        o.push_str("{\"code\":");
        json::write_string_into(&d.code, o);
        o.push_str(",\"message\":");
        json::write_string_into(&d.message, o);
        o.push_str(",\"severity\":");
        o.push_str(match d.severity {
            Severity::Warning => "\"warning\"",
            Severity::Error => "\"error\"",
        });
        o.push_str(",\"sources\":");
        write_sources(o, &d.sources);
        o.push('}');
    }
    o.push(']');
}

pub(crate) fn write_documents(o: &mut String, documents: &[DocumentResource]) {
    o.push('[');
    for (i, d) in documents.iter().enumerate() {
        sep(o, i);
        o.push_str("{\"byte_length\":");
        num(o, d.byte_length as f64);
        o.push_str(",\"path\":");
        json::write_string_into(&d.path, o);
        o.push_str(",\"revision\":");
        num(o, d.revision as f64);
        o.push_str(",\"sha256\":");
        json::write_string_into(&d.sha256, o);
        o.push('}');
    }
    o.push(']');
}

pub(crate) fn write_fonts(o: &mut String, fonts: &[FontResource]) {
    o.push('[');
    for (i, f) in fonts.iter().enumerate() {
        sep(o, i);
        o.push_str("{\"byte_length\":");
        num(o, f.byte_length as f64);
        o.push_str(",\"face_index\":");
        num(o, f64::from(f.face_index));
        o.push_str(",\"font_id\":");
        json::write_string_into(&f.font_id, o);
        o.push_str(",\"format\":");
        json::write_string_into(&f.format, o);
        o.push_str(",\"glyph_count\":");
        num(o, f64::from(f.glyph_count));
        o.push_str(",\"postscript_name\":");
        json::write_string_into(&f.postscript_name, o);
        o.push_str(",\"sha256\":");
        json::write_string_into(&f.sha256, o);
        o.push_str(",\"units_per_em\":");
        num(o, f64::from(f.units_per_em));
        o.push('}');
    }
    o.push(']');
}

/// The `required_features` array of `list` under `wire`.
pub(crate) fn write_features(o: &mut String, list: &DisplayList, wire: Wire) {
    o.push('[');
    for (i, f) in list.required_features_wire(wire).into_iter().enumerate() {
        sep(o, i);
        json::write_string_into(f, o);
    }
    o.push(']');
}

fn sep(o: &mut String, i: usize) {
    if i > 0 {
        o.push(',');
    }
}

fn num(o: &mut String, n: f64) {
    json::write_number_into(n, o);
}

fn write_tick(o: &mut String, t: Tick) {
    num(o, t.0 as f64);
}

fn write_sources(o: &mut String, sources: &[SourceRange]) {
    o.push('[');
    for (i, s) in sources.iter().enumerate() {
        sep(o, i);
        o.push_str("{\"end_byte\":");
        num(o, s.end_byte as f64);
        o.push_str(",\"path\":");
        json::write_string_into(&s.path, o);
        o.push_str(",\"start_byte\":");
        num(o, s.start_byte as f64);
        o.push('}');
    }
    o.push(']');
}

/// `sources` or `synthetic_reason`, preceded by a comma (both sort after
/// every key written before them and before every key written after).
fn write_provenance(o: &mut String, p: &Provenance) {
    match p {
        Provenance::Source(_) | Provenance::Sources(_) => {
            o.push_str(",\"sources\":");
            write_sources(o, p.sources());
        }
        Provenance::Synthetic(reason) => {
            o.push_str(",\"synthetic_reason\":");
            json::write_string_into(reason, o);
        }
    }
}

fn write_paint(o: &mut String, p: &Paint, device: bool) {
    o.push_str("{\"a\":");
    num(o, p.a);
    o.push_str(",\"b\":");
    num(o, p.b);
    if let Some(d) = p.device.filter(|_| device) {
        o.push_str(",\"device_color\":{\"space\":");
        json::write_string_into(device_space(&d), o);
        o.push_str(",\"values\":[");
        for (i, v) in d.operands().split(' ').enumerate() {
            sep(o, i);
            json::write_string_into(v, o);
        }
        o.push_str("]}");
    }
    o.push_str(",\"g\":");
    num(o, p.g);
    o.push_str(",\"r\":");
    num(o, p.r);
    o.push('}');
}

/// [`path_json`] written directly.
fn write_path(o: &mut String, cmds: &[PathCmd]) {
    o.push('[');
    for (i, c) in cmds.iter().enumerate() {
        sep(o, i);
        let (op, ts): (&str, &[Tick]) = match c {
            PathCmd::Move(x, y) => ("[\"m\"", &[*x, *y]),
            PathCmd::Line(x, y) => ("[\"l\"", &[*x, *y]),
            PathCmd::Cubic(a, b, cc, d, e, f) => ("[\"c\"", &[*a, *b, *cc, *d, *e, *f]),
            PathCmd::Close => ("[\"z\"", &[]),
        };
        o.push_str(op);
        for t in ts {
            o.push(',');
            write_tick(o, *t);
        }
        o.push(']');
    }
    o.push(']');
}

/// [`image_json`] written directly. BTreeMap key order: height, image
/// (byte_length, format, image_id, path, pdf_box, pdf_page, pdf_rotate,
/// pixel_height, pixel_width, sha256), kind, sources/synthetic_reason, top,
/// transform, width, x.
fn write_image(o: &mut String, i: &Image) {
    let r = &i.resource;
    o.push_str("{\"height\":");
    write_tick(o, i.height);
    o.push_str(",\"image\":{\"byte_length\":");
    num(o, r.byte_length as f64);
    o.push_str(",\"format\":");
    json::write_string_into(r.format, o);
    o.push_str(",\"image_id\":");
    json::write_string_into(&r.sha256, o);
    o.push_str(",\"path\":");
    json::write_string_into(&r.path, o);
    if let Some(b) = r.pdf_box {
        o.push_str(",\"pdf_box\":[");
        for (j, v) in b.iter().enumerate() {
            sep(o, j);
            num(o, *v);
        }
        o.push_str("],\"pdf_page\":");
        num(o, f64::from(r.pdf_page));
        o.push_str(",\"pdf_rotate\":");
        num(o, f64::from(r.pdf_rotate));
    }
    if let Some((w, h)) = r.pixels {
        o.push_str(",\"pixel_height\":");
        num(o, f64::from(h));
        o.push_str(",\"pixel_width\":");
        num(o, f64::from(w));
    }
    o.push_str(",\"sha256\":");
    json::write_string_into(&r.sha256, o);
    o.push_str("},\"kind\":\"image\"");
    write_provenance(o, &i.provenance);
    o.push_str(",\"top\":");
    write_tick(o, i.top);
    o.push_str(",\"transform\":[");
    for (j, v) in i.transform.iter().enumerate() {
        sep(o, j);
        num(o, (v * 1000.0).round() / 1000.0 + 0.0);
    }
    o.push_str("],\"width\":");
    write_tick(o, i.width);
    o.push_str(",\"x\":");
    write_tick(o, i.x);
    o.push('}');
}

fn device_space(d: &flashtex_compiler::color::DeviceColor) -> &'static str {
    match d.space {
        flashtex_compiler::color::ColorSpace::Gray => "gray",
        flashtex_compiler::color::ColorSpace::Rgb => "rgb",
        flashtex_compiler::color::ColorSpace::Cmyk => "cmyk",
    }
}

/// One page object exactly as it sits inside the full line's `pages` array
/// (and inside a delta's `changed_pages`).
pub fn write_page(o: &mut String, p: &Page, wire: Wire) {
    let images = wire.images;
    o.push_str("{\"height\":");
    write_tick(o, p.height);
    o.push_str(",\"items\":[");
    for (i, it) in p.items.iter().filter(|it| images || !matches!(it, Item::Image(_))).enumerate() {
        sep(o, i);
        match it {
            Item::Image(img) => write_image(o, img),
            Item::GlyphRun(r) => {
                o.push_str("{\"clusters\":[");
                for (j, c) in r.clusters.iter().enumerate() {
                    sep(o, j);
                    o.push_str("{\"carets\":[");
                    for (k, caret) in c.carets.iter().enumerate() {
                        sep(o, k);
                        o.push_str("{\"height\":");
                        write_tick(o, caret.height);
                        o.push_str(",\"text_byte\":");
                        num(o, caret.text_byte as f64);
                        o.push_str(",\"top\":");
                        write_tick(o, caret.top);
                        o.push_str(",\"x\":");
                        write_tick(o, caret.x);
                        o.push('}');
                    }
                    o.push_str("],\"hit_rects\":[");
                    for (k, rect) in c.hit_rects().iter().enumerate() {
                        sep(o, k);
                        o.push_str("{\"height\":");
                        write_tick(o, rect.height);
                        o.push_str(",\"top\":");
                        write_tick(o, rect.top);
                        o.push_str(",\"width\":");
                        write_tick(o, rect.width);
                        o.push_str(",\"x\":");
                        write_tick(o, rect.x);
                        o.push('}');
                    }
                    o.push(']');
                    write_provenance(o, &c.provenance);
                    o.push_str(",\"text_end_byte\":");
                    num(o, c.text_end_byte as f64);
                    o.push_str(",\"text_start_byte\":");
                    num(o, c.text_start_byte as f64);
                    o.push('}');
                }
                o.push_str("],\"font_id\":");
                json::write_string_into(&r.font_id, o);
                o.push_str(",\"font_size\":");
                write_tick(o, r.font_size);
                o.push_str(",\"glyphs\":[");
                for (j, g) in r.glyphs.iter().enumerate() {
                    sep(o, j);
                    o.push_str("{\"advance_x\":");
                    write_tick(o, g.advance_x);
                    o.push_str(",\"advance_y\":");
                    write_tick(o, g.advance_y);
                    o.push_str(",\"baseline_y\":");
                    write_tick(o, g.baseline_y);
                    o.push_str(",\"cluster\":");
                    num(o, f64::from(g.cluster));
                    o.push_str(",\"gid\":");
                    num(o, f64::from(g.gid));
                    o.push_str(",\"origin_x\":");
                    write_tick(o, g.origin_x);
                    o.push('}');
                }
                o.push_str("],\"kind\":\"glyph_run\",\"paint\":");
                write_paint(o, &r.paint, wire.device_color);
                o.push_str(",\"text\":");
                json::write_string_into(&r.text, o);
                o.push('}');
            }
            Item::Rule(r) => {
                o.push_str("{\"height\":");
                write_tick(o, r.height);
                o.push_str(",\"kind\":\"rule\",\"paint\":");
                write_paint(o, &r.paint, wire.device_color);
                write_provenance(o, &r.provenance);
                o.push_str(",\"top\":");
                write_tick(o, r.top);
                o.push_str(",\"width\":");
                write_tick(o, r.width);
                o.push_str(",\"x\":");
                write_tick(o, r.x);
                o.push('}');
            }
            Item::Path(p) => {
                // BTreeMap key order: clips, fill_rule, kind, paint, path,
                // sources, stroke, synthetic_reason.
                o.push('{');
                if !p.clips.is_empty() {
                    o.push_str("\"clips\":[");
                    for (j, c) in p.clips.iter().enumerate() {
                        sep(o, j);
                        o.push_str("{\"fill_rule\":");
                        o.push_str(if c.even_odd { "\"evenodd\"" } else { "\"nonzero\"" });
                        o.push_str(",\"kind\":\"path\",\"path\":");
                        write_path(o, &c.commands);
                        o.push('}');
                    }
                    o.push_str("],");
                }
                let stroke = match &p.op {
                    PathPaintOp::Fill { even_odd } => {
                        o.push_str("\"fill_rule\":");
                        o.push_str(if *even_odd { "\"evenodd\"" } else { "\"nonzero\"" });
                        o.push_str(",\"kind\":\"path_fill\"");
                        None
                    }
                    PathPaintOp::Stroke(s) => {
                        o.push_str("\"kind\":\"path_stroke\"");
                        Some(s)
                    }
                };
                o.push_str(",\"paint\":");
                write_paint(o, &p.paint, wire.device_color);
                o.push_str(",\"path\":");
                write_path(o, &p.commands);
                let synthetic = matches!(p.provenance, Provenance::Synthetic(_));
                if !synthetic {
                    write_provenance(o, &p.provenance);
                }
                if let Some(s) = stroke {
                    o.push_str(",\"stroke\":{\"cap\":");
                    o.push_str(match s.cap {
                        LineCap::Butt => "\"butt\"",
                        LineCap::Round => "\"round\"",
                        LineCap::Square => "\"square\"",
                    });
                    if !s.dash.is_empty() {
                        o.push_str(",\"dash\":{\"array\":[");
                        for (j, t) in s.dash.iter().enumerate() {
                            sep(o, j);
                            write_tick(o, *t);
                        }
                        o.push_str("],\"phase\":");
                        write_tick(o, s.dash_phase);
                        o.push('}');
                    }
                    o.push_str(",\"join\":");
                    o.push_str(match s.join {
                        LineJoin::Miter => "\"miter\"",
                        LineJoin::Round => "\"round\"",
                        LineJoin::Bevel => "\"bevel\"",
                    });
                    o.push_str(",\"miter_limit\":");
                    num(o, s.miter_limit);
                    o.push_str(",\"width\":");
                    write_tick(o, s.width);
                    o.push('}');
                }
                if synthetic {
                    write_provenance(o, &p.provenance);
                }
                o.push('}');
            }
        }
    }
    o.push_str("],\"number\":");
    num(o, f64::from(p.number));
    o.push_str(",\"width\":");
    write_tick(o, p.width);
    o.push('}');
}

fn tick(t: Tick) -> Value {
    json::num(t.0 as f64)
}

fn source_json(s: &SourceRange) -> Value {
    let mut o = Value::obj();
    o.set("path", json::str_(s.path.to_string()));
    o.set("start_byte", json::num(s.start_byte as f64));
    o.set("end_byte", json::num(s.end_byte as f64));
    o
}

fn provenance_into(o: &mut Value, p: &Provenance) {
    match p {
        Provenance::Source(_) | Provenance::Sources(_) => o.set("sources", Value::Arr(p.sources().iter().map(source_json).collect())),
        Provenance::Synthetic(reason) => o.set("synthetic_reason", json::str_(reason.clone())),
    }
}

fn paint_json(p: &Paint, device: bool) -> Value {
    let mut o = Value::obj();
    if let Some(d) = p.device.filter(|_| device) {
        let mut dc = Value::obj();
        dc.set("space", json::str_(device_space(&d)));
        dc.set("values", Value::Arr(d.operands().split(' ').map(|v| json::str_(v.to_string())).collect()));
        o.set("device_color", dc);
    }
    o.set("r", json::num(p.r));
    o.set("g", json::num(p.g));
    o.set("b", json::num(p.b));
    o.set("a", json::num(p.a));
    o
}

/// `[["m",x,y],["l",x,y],["c",x1,y1,x2,y2,x,y],["z"]]` in ticks.
fn path_json(cmds: &[PathCmd]) -> Value {
    Value::Arr(
        cmds.iter()
            .map(|c| {
                Value::Arr(match *c {
                    PathCmd::Move(x, y) => vec![json::str_("m"), tick(x), tick(y)],
                    PathCmd::Line(x, y) => vec![json::str_("l"), tick(x), tick(y)],
                    PathCmd::Cubic(a, b, cc, d, e, f) => vec![json::str_("c"), tick(a), tick(b), tick(cc), tick(d), tick(e), tick(f)],
                    PathCmd::Close => vec![json::str_("z")],
                })
            })
            .collect(),
    )
}

fn rect_json(r: &Rect) -> Value {
    let mut o = Value::obj();
    o.set("x", tick(r.x));
    o.set("top", tick(r.top));
    o.set("width", tick(r.width));
    o.set("height", tick(r.height));
    o
}

pub fn diagnostic_json(d: &Diagnostic) -> Value {
    let mut o = Value::obj();
    o.set("code", json::str_(d.code.clone()));
    o.set("message", json::str_(d.message.clone()));
    o.set(
        "severity",
        json::str_(match d.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        }),
    );
    o.set("sources", Value::Arr(d.sources.iter().map(source_json).collect()));
    o
}

fn page_json(p: &Page, wire: Wire) -> Value {
    let images = wire.images;
    let mut o = Value::obj();
    o.set("number", json::num(f64::from(p.number)));
    o.set("width", tick(p.width));
    o.set("height", tick(p.height));
    o.set(
        "items",
        Value::Arr(
            p.items
                .iter()
                .filter(|it| images || !matches!(it, Item::Image(_)))
                .map(|it| match it {
                    Item::GlyphRun(r) => {
                        let mut o = Value::obj();
                        o.set("kind", json::str_("glyph_run"));
                        o.set("font_id", json::str_(r.font_id.to_string()));
                        o.set("font_size", tick(r.font_size));
                        o.set("text", json::str_(r.text.clone()));
                        o.set(
                            "glyphs",
                            Value::Arr(
                                r.glyphs
                                    .iter()
                                    .map(|g| {
                                        let mut o = Value::obj();
                                        o.set("gid", json::num(f64::from(g.gid)));
                                        o.set("origin_x", tick(g.origin_x));
                                        o.set("baseline_y", tick(g.baseline_y));
                                        o.set("advance_x", tick(g.advance_x));
                                        o.set("advance_y", tick(g.advance_y));
                                        o.set("cluster", json::num(f64::from(g.cluster)));
                                        o
                                    })
                                    .collect(),
                            ),
                        );
                        o.set(
                            "clusters",
                            Value::Arr(
                                r.clusters
                                    .iter()
                                    .map(|c| {
                                        let mut o = Value::obj();
                                        o.set("text_start_byte", json::num(c.text_start_byte as f64));
                                        o.set("text_end_byte", json::num(c.text_end_byte as f64));
                                        o.set("hit_rects", Value::Arr(c.hit_rects().iter().map(rect_json).collect()));
                                        o.set(
                                            "carets",
                                            Value::Arr(
                                                c.carets
                                                    .iter()
                                                    .map(|k| {
                                                        let mut o = Value::obj();
                                                        o.set("text_byte", json::num(k.text_byte as f64));
                                                        o.set("x", tick(k.x));
                                                        o.set("top", tick(k.top));
                                                        o.set("height", tick(k.height));
                                                        o
                                                    })
                                                    .collect(),
                                            ),
                                        );
                                        provenance_into(&mut o, &c.provenance);
                                        o
                                    })
                                    .collect(),
                            ),
                        );
                        o.set("paint", paint_json(&r.paint, wire.device_color));
                        o
                    }
                    Item::Path(p) => {
                        let mut o = Value::obj();
                        match &p.op {
                            PathPaintOp::Fill { even_odd } => {
                                o.set("kind", json::str_("path_fill"));
                                o.set("fill_rule", json::str_(if *even_odd { "evenodd" } else { "nonzero" }));
                            }
                            PathPaintOp::Stroke(s) => {
                                o.set("kind", json::str_("path_stroke"));
                                let mut so = Value::obj();
                                so.set("width", tick(s.width));
                                so.set(
                                    "cap",
                                    json::str_(match s.cap {
                                        LineCap::Butt => "butt",
                                        LineCap::Round => "round",
                                        LineCap::Square => "square",
                                    }),
                                );
                                so.set(
                                    "join",
                                    json::str_(match s.join {
                                        LineJoin::Miter => "miter",
                                        LineJoin::Round => "round",
                                        LineJoin::Bevel => "bevel",
                                    }),
                                );
                                so.set("miter_limit", json::num(s.miter_limit));
                                if !s.dash.is_empty() {
                                    let mut d = Value::obj();
                                    d.set("array", Value::Arr(s.dash.iter().map(|t| tick(*t)).collect()));
                                    d.set("phase", tick(s.dash_phase));
                                    so.set("dash", d);
                                }
                                o.set("stroke", so);
                            }
                        }
                        o.set("path", path_json(&p.commands));
                        if !p.clips.is_empty() {
                            o.set(
                                "clips",
                                Value::Arr(
                                    p.clips
                                        .iter()
                                        .map(|c| {
                                            let mut co = Value::obj();
                                            co.set("kind", json::str_("path"));
                                            co.set("path", path_json(&c.commands));
                                            co.set("fill_rule", json::str_(if c.even_odd { "evenodd" } else { "nonzero" }));
                                            co
                                        })
                                        .collect(),
                                ),
                            );
                        }
                        o.set("paint", paint_json(&p.paint, wire.device_color));
                        provenance_into(&mut o, &p.provenance);
                        o
                    }
                    Item::Rule(r) => {
                        let mut o = Value::obj();
                        o.set("kind", json::str_("rule"));
                        o.set("x", tick(r.x));
                        o.set("top", tick(r.top));
                        o.set("width", tick(r.width));
                        o.set("height", tick(r.height));
                        o.set("paint", paint_json(&r.paint, wire.device_color));
                        provenance_into(&mut o, &r.provenance);
                        o
                    }
                    Item::Image(i) => image_json(i),
                })
                .collect(),
        ),
    );
    o
}

fn image_json(i: &Image) -> Value {
    let mut o = Value::obj();
    o.set("kind", json::str_("image"));
    o.set("x", tick(i.x));
    o.set("top", tick(i.top));
    o.set("width", tick(i.width));
    o.set("height", tick(i.height));
    // Points with 1/1000 pt resolution: the transform is a paint hint, the
    // bounding box above is the exact geometry.
    o.set("transform", Value::Arr(i.transform.iter().map(|v| json::num((v * 1000.0).round() / 1000.0 + 0.0)).collect()));
    let r = &i.resource;
    let mut res = Value::obj();
    res.set("image_id", json::str_(r.sha256.to_string()));
    res.set("sha256", json::str_(r.sha256.to_string()));
    res.set("byte_length", json::num(r.byte_length as f64));
    res.set("format", json::str_(r.format));
    res.set("path", json::str_(r.path.clone()));
    if let Some((w, h)) = r.pixels {
        res.set("pixel_width", json::num(f64::from(w)));
        res.set("pixel_height", json::num(f64::from(h)));
    }
    if let Some(b) = r.pdf_box {
        res.set("pdf_page", json::num(f64::from(r.pdf_page)));
        res.set("pdf_box", Value::Arr(b.iter().map(|v| json::num(*v)).collect()));
        res.set("pdf_rotate", json::num(f64::from(r.pdf_rotate)));
    }
    o.set("image", res);
    provenance_into(&mut o, &i.provenance);
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_round_once_from_tex_points() {
        // 72.27 TeX pt = 72 bp = 72 * 2^20 ticks exactly.
        assert_eq!(Tick::from_tex_pt(72.27), Tick(72 * 1_048_576));
        assert_eq!(Tick::from_bp(612.0).0, 612 * 1_048_576);
        assert_eq!(Tick::from_tex_pt(0.0), Tick(0));
    }

    #[test]
    fn from_compiler_forwards_code_and_suggestion() {
        use flashtex_compiler::diagnostics::{Diagnostic as C, DiagnosticCode, Severity as CS};
        let unknown = C {
            severity: CS::Error,
            message: r"\alpah is not supported by this compiler version".into(),
            span: None,
            recovery: None,
            code: Some(DiagnosticCode::UnknownCommand),
            suggestion: Some(r"\alpha".into()),
        };
        let out = Diagnostic::from_compiler(&unknown, &[]);
        assert_eq!(out.code, "unknown_command");
        assert_eq!(out.suggestion.as_deref(), Some(r"\alpha"));

        let no_explicit = C {
            severity: CS::Error,
            message: r"\tikz is not supported by this compiler version".into(),
            span: None,
            recovery: None,
            code: None,
            suggestion: None,
        };
        assert_eq!(Diagnostic::from_compiler(&no_explicit, &[]).code, "unsupported_feature");

        let none = C {
            severity: CS::Error,
            message: "layout_capabilities must be a list".into(),
            span: None,
            recovery: None,
            code: None,
            suggestion: None,
        };
        assert_eq!(Diagnostic::from_compiler(&none, &[]).code, "compiler");
        assert_eq!(Diagnostic::from_compiler(&none, &[]).suggestion, None);
    }

    #[test]
    fn write_json_matches_the_value_tree() {
        let src = |a, b| SourceRange {
            path: std::rc::Rc::from("dir/ma\"in.tex"),
            start_byte: a,
            end_byte: b,
        };
        let caret = |x| Caret {
            text_byte: 3,
            x: Tick(x),
            top: Tick(-7),
            height: Tick(1 << 40),
        };
        let cluster = |provenance| Cluster {
            text_start_byte: 0,
            text_end_byte: 4,
            hit_rect: Rect {
                x: Tick(1),
                top: Tick(-2),
                width: Tick(3),
                height: Tick(4),
            },
            carets: Carets {
                first: caret(5),
                last: Some(caret(9)),
            },
            provenance,
        };
        let run = Item::GlyphRun(GlyphRun {
            font_id: std::rc::Rc::from("abc"),
            font_size: Tick(12 << 20),
            text: "ﬁ \"q\"\\\n\t\u{1}é".into(),
            glyphs: vec![
                Glyph {
                    gid: 65535,
                    origin_x: Tick(-1),
                    baseline_y: Tick(2),
                    advance_x: Tick(3),
                    advance_y: Tick(0),
                    cluster: 1,
                };
                2
            ],
            clusters: vec![
                cluster(Provenance::Source(src(1, 2))),
                cluster(Provenance::Sources(vec![src(3, 4), src(5, 6)])),
                cluster(Provenance::Synthetic("heading number".into())),
            ],
            paint: Paint {
                r: 0.25,
                g: 0.1,
                b: 1.0 / 3.0,
                a: 1.0,
                device: None,
            },
            role: RunRole::Text,
        });
        let rule = |provenance| {
            Item::Rule(Rule {
                x: Tick(10),
                top: Tick(20),
                width: Tick(30),
                height: Tick(40),
                paint: Paint::BLACK,
                provenance,
            })
        };
        let cmds = || {
            vec![
                PathCmd::Move(Tick(1), Tick(-2)),
                PathCmd::Line(Tick(3), Tick(4)),
                PathCmd::Cubic(Tick(5), Tick(6), Tick(7), Tick(8), Tick(9), Tick(1 << 40)),
                PathCmd::Close,
            ]
        };
        let clips = || {
            vec![
                ClipPath { commands: cmds(), even_odd: false },
                ClipPath { commands: Vec::new(), even_odd: true },
            ]
        };
        let stroke = |dash: Vec<Tick>| Stroke {
            width: Tick(1 << 19),
            cap: LineCap::Round,
            join: LineJoin::Bevel,
            miter_limit: 10.5,
            dash,
            dash_phase: Tick(2),
        };
        let path = |op, clips, provenance| {
            Item::Path(PathItem {
                op,
                commands: cmds(),
                clips,
                paint: Paint { r: 0.5, g: 0.0, b: 1.0, a: 0.25, device: None },
                provenance,
            })
        };
        let png = || {
            std::rc::Rc::new(ImageResource {
                sha256: std::rc::Rc::from("beef"),
                byte_length: 1 << 34,
                format: "png",
                path: "img/r\"ed.png".into(),
                pixels: Some((96, 48)),
                pdf_page: 0,
                pdf_box: None,
                pdf_rotate: 0,
            })
        };
        let pdf = || {
            std::rc::Rc::new(ImageResource {
                sha256: std::rc::Rc::from("cafe"),
                byte_length: 777,
                format: "pdf",
                path: "box.pdf".into(),
                pixels: None,
                pdf_page: 2,
                pdf_box: Some([0.5, -1.0, 612.0, 792.25]),
                pdf_rotate: -90,
            })
        };
        let image = |resource, provenance| {
            Item::Image(Image {
                x: Tick(11),
                top: Tick(-12),
                width: Tick(1 << 30),
                height: Tick(14),
                transform: [1.0 / 3.0, -0.0, 0.00049, 2.5, -72.0004, 1e9],
                resource,
                provenance,
            })
        };
        let list = DisplayList {
            project_id: "p\\1".into(),
            revision: 42,
            documents: vec![DocumentResource {
                path: "main.tex".into(),
                revision: 42,
                sha256: "00ff".into(),
                byte_length: 5126,
            }],
            fonts: vec![FontResource {
                font_id: std::rc::Rc::from("abc"),
                sha256: "abc".into(),
                byte_length: 1 << 33,
                format: "opentype-cff".into(),
                face_index: 0,
                units_per_em: 1000,
                glyph_count: 821,
                postscript_name: "LMRoman12-Regular".into(),
                path: Some("/x".into()),
            }],
            pages: vec![
                Page {
                    number: 1,
                    width: Tick(612 << 20),
                    height: Tick(792 << 20),
                    items: vec![run, rule(Provenance::Source(src(7, 8))), rule(Provenance::Synthetic("frac".into()))],
                },
                Page {
                    number: 3,
                    width: Tick(612 << 20),
                    height: Tick(792 << 20),
                    items: vec![
                        path(PathPaintOp::Fill { even_odd: true }, Vec::new(), Provenance::Source(src(1, 3))),
                        path(PathPaintOp::Fill { even_odd: false }, clips(), Provenance::Synthetic("tikz".into())),
                        path(PathPaintOp::Stroke(stroke(Vec::new())), Vec::new(), Provenance::Synthetic("tikz".into())),
                        image(png(), Provenance::Source(src(4, 7))),
                        path(PathPaintOp::Stroke(stroke(vec![Tick(3), Tick(-4)])), clips(), Provenance::Sources(vec![src(2, 5), src(6, 9)])),
                        image(pdf(), Provenance::Synthetic("float".into())),
                    ],
                },
                Page {
                    number: 2,
                    width: Tick(1),
                    height: Tick(2),
                    items: Vec::new(),
                },
            ],
            diagnostics: vec![
                Diagnostic::warning("overfull_hbox", "line \"3\" is 1.5pt too wide", vec![src(1, 9)]),
                Diagnostic::error("compiler", "x", Vec::new()),
            ],
        };
        assert_eq!(list.write_json("id\"1"), json::write(&list.to_json("id\"1")));
        for images in [false, true] {
            assert_eq!(list.write_json_with("id\"1", images), json::write(&list.to_json_with("id\"1", images)));
        }
        assert!(!list.write_json("i").contains("\"image\""));
        assert!(list.write_json_with("i", true).contains("\"kind\":\"image\""));
        let empty = DisplayList {
            project_id: String::new(),
            revision: 0,
            documents: Vec::new(),
            fonts: Vec::new(),
            pages: Vec::new(),
            diagnostics: Vec::new(),
        };
        assert_eq!(empty.write_json(""), json::write(&empty.to_json("")));
        assert_eq!(empty.write_json_with("", true), json::write(&empty.to_json_with("", true)));
    }
}
