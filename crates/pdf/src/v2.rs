//! rendering-v2 display list → exact export.
//!
//! Consumes the `display_list` envelope written by `flashtex-render --v2`
//! (`crates/render-pipeline`, shaped after `protocol/rendering-v2.schema.json`
//! and `crates/rendering-core`'s model) and builds an [`ExactDocument`]:
//!
//! - `coordinate_unit` must be `bp_2pow20`: signed integer ticks, 2^20 per
//!   PDF point, y downward from the page's top-left corner. Every tick value
//!   is converted to an exact terminating decimal (`ticks / 2^20`, at most
//!   20 fractional digits) after the y flip is done in integer arithmetic,
//!   so no `f64` is involved in a coordinate.
//! - Each `glyph_run` glyph carries its original glyph id and an absolute
//!   origin; the origin is authoritative and advances are never added again
//!   (rendering-v2 proposal). A glyph continues the previous glyph's `TJ`
//!   segment when the gap between its origin and the natural advance is an
//!   exactly representable adjustment in thousandths of text space (zero:
//!   same string; non-zero: a `TJ` number, the pdfTeX shape); otherwise it
//!   gets its own `Tm`. Either way the replayed position is the envelope's
//!   origin exactly (`exact::glyph_positions`, tested).
//! - `/W` widths are the producer's own kern-free advances (the most
//!   frequent `advance_x / font_size` per glyph) wherever they differ from
//!   the font's hmtx. Painting does not change (origins are absolute), but a
//!   viewer's text extraction decides word boundaries from where the pen
//!   lands after each glyph: with hmtx widths, Latin Modern's bold `W`
//!   (TFM 1093/1000 em, hmtx 1189) made PDFKit read "Wednesday," as
//!   "W ednesday ,". The report counts the replaced entries.
//! - Fonts are content-addressed (`font_id` = SHA-256 of the program) and
//!   the envelope carries no path, so the bytes are resolved by hashing
//!   candidate files (`--font-dir`, `FLASHTEX_FONT_DIRS`, `FLASHTEX_LM_DIR`,
//!   the TeX Live Latin Modern directories) whose size matches, then
//!   embedded through [`ExactFont::cid_from_opentype`] (CFF: GID-preserving
//!   CID-keyed subset; TrueType: glyph order kept). `format` is accepted as
//!   `opentype-cff` (the pipeline's documented deviation from the schema
//!   token) or `static-truetype`; `core14-afm` has no bytes and is refused
//!   when a run uses it.
//! - `rule` items become `Op::rule`. Paint must be opaque; black is the
//!   default fill, any other opaque colour is written as an exact `rg`.
//! - Cluster ActualText is reduced to a per-glyph ToUnicode entry (the
//!   cluster's text); a glyph seen with two different texts keeps the first
//!   and the report says so. Marked-content `/ActualText` is outside the
//!   bounded operator set. Only a cluster of *one* glyph names that glyph's
//!   text: the three periods of an ellipsis share the cluster `…`, and
//!   mapping the period glyph to `…` would make every period of the document
//!   extract as `…` (and the ellipsis as `………`). A glyph seen only in
//!   clusters of several glyphs takes the character the font's own `cmap`
//!   maps to it (`.`, as pdfTeX's `. . .` extracts), else the cluster text.
//! - `image` items (`display-list-v2-images`,
//!   `protocol/proposals/display-list-v2-image.md`) need
//!   a project root ([`from_v2_rooted`]): the file is read under it without following
//!   links, its length and SHA-256 must match the item, and PNG/JPEG become
//!   image XObjects and PDF pages form XObjects the way pdfTeX 1.40.29 writes
//!   them (`crate::images`). Placement is `q [a -b c -d e H-f] cm … /ImN Do Q`
//!   with the item's own decimals. One XObject per distinct (bytes, page).
//!
//! Everything unsupported is an error naming the item; nothing is dropped.

use crate::exact::{
    Content, Decimal, ExactDocument, ExactFont, ExactPage, GlyphRun, Op, PlacedGlyph, Ratio,
    SubsetOutcome,
};
use crate::images::{self, Geometry};
use crate::json::{self, Value};
use crate::sha256;
use crate::truetype::TrueTypeFont;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const TICKS_PER_BP: i128 = 1 << 20;
pub const COORDINATE_UNIT: &str = "bp_2pow20";
/// Directories searched for font bytes after `--font-dir`, `FLASHTEX_FONT_DIRS`
/// and `FLASHTEX_LM_DIR` (the same list `flashtex-render` uses).
pub const DEFAULT_FONT_DIRS: [&str; 12] = [
    "/usr/local/texlive/2026/texmf-dist/fonts/opentype/public/lm",
    "/usr/local/texlive/2026/texmf-dist/fonts/opentype/public/lm-math",
    "/usr/local/texlive/2026basic/texmf-dist/fonts/opentype/public/lm",
    "/usr/local/texlive/2026basic/texmf-dist/fonts/opentype/public/lm-math",
    "/usr/local/texlive/2025/texmf-dist/fonts/opentype/public/lm",
    "/usr/local/texlive/2025/texmf-dist/fonts/opentype/public/lm-math",
    "/usr/local/texlive/2025basic/texmf-dist/fonts/opentype/public/lm",
    "/usr/local/texlive/2025basic/texmf-dist/fonts/opentype/public/lm-math",
    "/usr/share/texmf/fonts/opentype/public/lm",
    "/usr/share/texmf/fonts/opentype/public/lm-math",
    "/usr/share/texlive/texmf-dist/fonts/opentype/public/lm",
    "/usr/share/texlive/texmf-dist/fonts/opentype/public/lm-math",
];
/// Largest envelope accepted.
pub const MAX_ENVELOPE_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, Default)]
pub struct V2Options {
    /// Directories probed first, in order.
    pub font_dirs: Vec<PathBuf>,
}

/// One distinct image resource (same bytes and PDF page) in first-use order.
struct ImageRequest {
    first_item: String,
    path: String,
    sha256: String,
    byte_length: u64,
    format: String,
    page: u32,
    pixels: Option<(f64, f64)>,
    pdf_box: Option<[f64; 4]>,
    pdf_rotate: Option<f64>,
}

/// One embedded font, for the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontNote {
    pub resource: String,
    pub font_id: String,
    pub postscript_name: String,
    pub path: PathBuf,
    pub hash_form: HashForm,
    pub glyphs: usize,
    pub outcome: SubsetOutcome,
    pub program_bytes: usize,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct V2Report {
    pub fonts: Vec<FontNote>,
    pub pages: usize,
    pub glyphs: usize,
    pub runs: usize,
    pub rules: usize,
    /// Image items placed (`q … cm /ImN Do Q`).
    pub images: usize,
    /// Distinct image XObjects embedded.
    pub image_resources: usize,
    /// Glyphs that continued the previous string at the natural advance.
    pub joined_glyphs: usize,
    /// Glyphs that continued the previous segment with an exact `TJ`
    /// adjustment.
    pub kerned_glyphs: usize,
    /// `/W` entries written from the display list's own advances because
    /// they differ from the font's hmtx (the producer's TFM metrics; a
    /// viewer's text extraction keeps word boundaries only when the pen
    /// after a glyph lands where the next glyph actually starts).
    pub display_widths: usize,
    /// Gaps between consecutive same-baseline glyphs (the next origin minus
    /// the pen after the previous glyph's written `/W` width) of at least
    /// [`WORD_GAP_EM`] of the previous glyph's font size: the word boundaries
    /// the export relies on extractors to read from geometry
    /// (`docs/proposals/pdf-searchable-text.md`).
    pub word_gaps: usize,
    /// Gaps in `[`[`CHAR_GAP_EM`]`, `[`WORD_GAP_EM`]`)`: extractor-dependent
    /// (measured on mac-m1max-a: PDFKit/Spotlight break a word from 100/1000
    /// em, Ghostscript `txtwrite` from 250, Poppler `-raw` from its
    /// `minWordSpacing` 150); typically a math italic correction. The first
    /// few are named in `notes`.
    pub ambiguous_gaps: usize,
    pub diagnostics: Vec<String>,
    pub notes: Vec<String>,
}

/// Gap, in thousandths of the font size, that the export promises as a word
/// boundary for PDFKit/Spotlight (measured: from 100) and Poppler `-raw`
/// (its `minWordSpacing`, 150). A Computer Modern word space shrunk to its
/// TeX minimum is still above it (cmr12: 326 - 109 = 217). Ghostscript
/// `txtwrite` needs about 250 and is not promised.
pub const WORD_GAP_EM: i128 = 150;
/// Gap, in thousandths of the font size, below which no measured extractor
/// breaks a word (Poppler's `maxCharSpacing`; PDFKit measured to keep `ab`
/// at 80); kerns and rounding live here.
pub const CHAR_GAP_EM: i128 = 30;

fn f(v: Option<&Value>, what: &str) -> Result<f64, String> {
    v.and_then(Value::as_f64)
        .ok_or_else(|| format!("{what}: expected a number"))
}

fn ticks(v: Option<&Value>, what: &str) -> Result<i128, String> {
    let n = f(v, what)?;
    if !n.is_finite() || n.fract() != 0.0 || n.abs() > 9.0e15 {
        return Err(format!("{what}: {n} is not an integer tick value"));
    }
    Ok(n as i128)
}

fn s<'a>(v: Option<&'a Value>, what: &str) -> Result<&'a str, String> {
    v.and_then(Value::as_str)
        .ok_or_else(|| format!("{what}: expected a string"))
}

fn arr<'a>(v: Option<&'a Value>, what: &str) -> Result<&'a [Value], String> {
    v.and_then(Value::as_array)
        .ok_or_else(|| format!("{what}: expected an array"))
}

/// Exact decimal of `ticks / 2^20`.
pub fn bp(t: i128) -> Result<Decimal, String> {
    Decimal::from_ratio(t, TICKS_PER_BP as u128, 20)
        .ok_or_else(|| format!("tick value {t} has no terminating decimal (impossible for 2^20)"))
}

/// Exact decimal of a finite `f64` in `[0, 1]` (colour component).
fn exact_unit(v: f64, what: &str) -> Result<Decimal, String> {
    if !v.is_finite() || !(0.0..=1.0).contains(&v) {
        return Err(format!("{what}: {v} is not in [0, 1]"));
    }
    if v == 0.0 {
        return Ok(Decimal::from_i64(0));
    }
    if v == 1.0 {
        return Ok(Decimal::from_i64(1));
    }
    let bits = v.to_bits();
    let exponent = ((bits >> 52) & 2047) as i32;
    let mantissa = (bits & ((1u64 << 52) - 1)) | (1u64 << 52);
    let shift = 1075 - exponent;
    if exponent == 0 || !(0..64).contains(&shift) {
        return Err(format!(
            "{what}: {v} is not exactly representable within the decimal bound"
        ));
    }
    Decimal::from_ratio(mantissa as i128, 1u128 << shift, 20)
        .ok_or_else(|| format!("{what}: {v} needs more than 20 decimal digits"))
}

struct Paint {
    /// Fill and stroke operators to set before painting (`None`: black,
    /// the page default). A `device_color` (proposal
    /// `display-list-v2-device-color`) is written exactly as pdfTeX does,
    /// fill then stroke (`pdftex.def`: `r g b rg r g b RG`); an sRGB paint
    /// becomes an exact `rg`.
    ops: Option<Vec<Op>>,
}

/// `paint.device_color`: `{"space": "rgb"|"cmyk"|"gray", "values": [..]}`
/// with decimal strings, copied verbatim into the operators.
fn device_color(v: &Value, what: &str) -> Result<Vec<Op>, String> {
    let space = s(v.get("space"), &format!("{what}.device_color.space"))?;
    let values = arr(v.get("values"), &format!("{what}.device_color.values"))?
        .iter()
        .enumerate()
        .map(|(i, x)| {
            let w = format!("{what}.device_color.values[{i}]");
            let text = x.as_str().ok_or_else(|| format!("{w}: expected a decimal string"))?;
            let d = Decimal::new(text).map_err(|e| format!("{w}: {e}"))?;
            if d.approx() < 0.0 || d.approx() > 1.0 {
                return Err(format!("{w}: {text} is not in [0, 1]"));
            }
            Ok(d)
        })
        .collect::<Result<Vec<Decimal>, String>>()?;
    let n = |k: usize| -> Result<(), String> {
        if values.len() == k {
            Ok(())
        } else {
            Err(format!("{what}.device_color: {space} takes {k} values, found {}", values.len()))
        }
    };
    let v = &values;
    Ok(match space {
        "gray" => {
            n(1)?;
            vec![Op::FillGray(v[0].clone()), Op::StrokeGray(v[0].clone())]
        }
        "rgb" => {
            n(3)?;
            let c = [v[0].clone(), v[1].clone(), v[2].clone()];
            vec![Op::FillRgb(c.clone()), Op::StrokeRgb(c)]
        }
        "cmyk" => {
            n(4)?;
            let c = [v[0].clone(), v[1].clone(), v[2].clone(), v[3].clone()];
            vec![Op::FillCmyk(c.clone()), Op::StrokeCmyk(c)]
        }
        other => return Err(format!("{what}.device_color.space: unknown colour space {other}")),
    })
}

fn paint(v: Option<&Value>, what: &str) -> Result<Paint, String> {
    let Some(p) = v else {
        return Ok(Paint { ops: None });
    };
    let a = f(p.get("a"), &format!("{what}.paint.a"))?;
    if a != 1.0 {
        return Err(format!(
            "{what}: paint alpha {a} is not 1; alpha needs an ExtGState, which is outside the bounded operator set"
        ));
    }
    if let Some(dc) = p.get("device_color") {
        return Ok(Paint { ops: Some(device_color(dc, &format!("{what}.paint"))?) });
    }
    let r = f(p.get("r"), &format!("{what}.paint.r"))?;
    let g = f(p.get("g"), &format!("{what}.paint.g"))?;
    let b = f(p.get("b"), &format!("{what}.paint.b"))?;
    if r == 0.0 && g == 0.0 && b == 0.0 {
        return Ok(Paint { ops: None });
    }
    Ok(Paint {
        ops: Some(vec![Op::FillRgb([
            exact_unit(r, what)?,
            exact_unit(g, what)?,
            exact_unit(b, what)?,
        ])]),
    })
}

struct FontEntry {
    font_id: String,
    sha256: String,
    byte_length: u64,
    format: String,
    face_index: u64,
    units_per_em: u64,
    glyph_count: u64,
    postscript_name: String,
    resource: String,
}

/// How a font file matched the envelope's `sha256`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashForm {
    /// SHA-256 over the file bytes (rendering-core's validator, the contract).
    Bytes,
    /// SHA-256 over the file bytes followed by the big-endian `u32` face index
    /// (font-engine's `content_sha256`, what `flashtex-render` emits today).
    BytesAndFaceIndex,
}

/// Finds a file with the given SHA-256 among the candidate directories,
/// hashing only files whose size matches. Both hash forms are tried.
pub fn resolve_font(
    dirs: &[PathBuf],
    sha: &str,
    byte_length: u64,
    face_index: u32,
) -> Option<(PathBuf, HashForm)> {
    let mut seen = BTreeSet::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        for p in paths {
            if !seen.insert(p.clone()) {
                continue;
            }
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase());
            if !matches!(ext.as_deref(), Some("otf" | "ttf")) {
                continue;
            }
            let Ok(meta) = std::fs::metadata(&p) else {
                continue;
            };
            if meta.len() != byte_length {
                continue;
            }
            if let Ok(mut bytes) = std::fs::read(&p) {
                if sha256::hex(&bytes) == sha {
                    return Some((p, HashForm::Bytes));
                }
                bytes.extend_from_slice(&face_index.to_be_bytes());
                if sha256::hex(&bytes) == sha {
                    return Some((p, HashForm::BytesAndFaceIndex));
                }
            }
        }
    }
    None
}

/// The font directories to probe: `options.font_dirs`, then the environment
/// and the defaults.
pub fn font_dirs(options: &V2Options) -> Vec<PathBuf> {
    let mut dirs = options.font_dirs.clone();
    if let Ok(v) = std::env::var("FLASHTEX_FONT_DIRS") {
        dirs.extend(v.split(':').filter(|x| !x.is_empty()).map(PathBuf::from));
    }
    if let Ok(v) = std::env::var("FLASHTEX_LM_DIR")
        && !v.is_empty()
    {
        dirs.push(PathBuf::from(v));
    }
    dirs.extend(DEFAULT_FONT_DIRS.iter().map(PathBuf::from));
    dirs
}

/// Builds the exact document from a rendering-v2 `display_list` envelope.
pub fn from_v2(envelope: &str, options: &V2Options) -> Result<(ExactDocument, V2Report), String> {
    from_v2_rooted(envelope, options, None)
}

/// [`from_v2`] with the absolute project directory `image` item paths
/// resolve under (`display-list-v2-images`); `None` refuses every image
/// item. A separate entry point keeps [`V2Options`] (built with a struct
/// literal by `crates/rendering-core`) unchanged.
pub fn from_v2_rooted(
    envelope: &str,
    options: &V2Options,
    project_root: Option<&Path>,
) -> Result<(ExactDocument, V2Report), String> {
    if envelope.len() > MAX_ENVELOPE_BYTES {
        return Err("envelope larger than 256 MiB".into());
    }
    let root = json::parse(envelope)?;
    let version = f(root.get("protocol_version"), "protocol_version")?;
    if version != 2.0 {
        return Err(format!("protocol_version {version} is not 2"));
    }
    let kind = s(root.get("type"), "type")?;
    if kind != "display_list" {
        return Err(format!("type {kind:?} is not display_list"));
    }
    let p = root.get("payload").ok_or("missing payload")?;
    let render_format = s(p.get("render_format"), "payload.render_format")?;
    if render_format != "display-list-v2" {
        return Err(format!(
            "render_format {render_format:?} is not display-list-v2"
        ));
    }
    let unit = s(p.get("coordinate_unit"), "payload.coordinate_unit")?;
    if unit != COORDINATE_UNIT {
        return Err(format!("coordinate_unit {unit:?} is not {COORDINATE_UNIT}"));
    }
    let color_space = s(p.get("color_space"), "payload.color_space")?;
    if color_space != "srgb" {
        return Err(format!("color_space {color_space:?} is not srgb"));
    }
    let mut report = V2Report::default();
    for d in arr(p.get("diagnostics"), "payload.diagnostics").unwrap_or(&[]) {
        report.diagnostics.push(format!(
            "{}: {}: {}",
            s(d.get("severity"), "severity").unwrap_or("?"),
            s(d.get("code"), "code").unwrap_or("?"),
            s(d.get("message"), "message").unwrap_or("?")
        ));
    }

    // Fonts, in envelope order; resource names follow that order.
    let mut fonts: BTreeMap<String, FontEntry> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    for (i, fv) in arr(p.get("fonts"), "payload.fonts")?.iter().enumerate() {
        let what = format!("payload.fonts[{i}]");
        let font_id = s(fv.get("font_id"), &format!("{what}.font_id"))?.to_string();
        let entry = FontEntry {
            sha256: s(fv.get("sha256"), &format!("{what}.sha256"))?.to_string(),
            byte_length: f(fv.get("byte_length"), &format!("{what}.byte_length"))? as u64,
            format: s(fv.get("format"), &format!("{what}.format"))?.to_string(),
            face_index: f(fv.get("face_index"), &format!("{what}.face_index"))? as u64,
            units_per_em: f(fv.get("units_per_em"), &format!("{what}.units_per_em"))? as u64,
            glyph_count: f(fv.get("glyph_count"), &format!("{what}.glyph_count"))? as u64,
            postscript_name: s(
                fv.get("postscript_name"),
                &format!("{what}.postscript_name"),
            )?
            .to_string(),
            resource: format!("F{}", i + 1),
            font_id: font_id.clone(),
        };
        if fonts.insert(font_id.clone(), entry).is_some() {
            return Err(format!("{what}: duplicate font_id {font_id}"));
        }
        order.push(font_id);
    }

    // Pass 1: walk pages, collect glyph ids and cluster texts per font, and
    // build operators with resource names.
    struct Used {
        gids: BTreeSet<u16>,
        to_unicode: BTreeMap<u16, String>,
        /// The first text of a cluster of several glyphs each glyph was
        /// seen in; used only for glyphs with no one-glyph cluster and no
        /// `cmap` character.
        shared: BTreeMap<u16, String>,
        conflicts: usize,
        /// Observed `advance_x` per glyph as a reduced ratio in 1000/em
        /// (`advance_x * 1000 / font_size`), with occurrence counts. The
        /// most frequent value is the producer's kern-free width.
        advances: BTreeMap<u16, BTreeMap<(i128, i128), usize>>,
    }
    let mut used: BTreeMap<String, Used> = BTreeMap::new();
    // A pending glyph run before joining decisions (needs font metrics).
    struct Glyph {
        gid: u16,
        origin_x: i128,
        y_pdf: i128,
        advance_y: i128,
        text: String,
    }
    enum Pending {
        Run {
            resource: String,
            size_ticks: i128,
            color: Option<Vec<Op>>,
            glyphs: Vec<Glyph>,
        },
        Ops(Vec<Op>),
        Image {
            index: usize,
            transform: [f64; 6],
            height_ticks: i128,
        },
    }
    let mut image_requests: Vec<ImageRequest> = Vec::new();
    let mut image_index: BTreeMap<(String, u32), usize> = BTreeMap::new();
    let mut pages_ops: Vec<(Decimal, Decimal, Vec<Pending>, BTreeSet<String>)> = Vec::new();
    for (pi, pv) in arr(p.get("pages"), "payload.pages")?.iter().enumerate() {
        let what = format!("payload.pages[{pi}]");
        let width = ticks(pv.get("width"), &format!("{what}.width"))?;
        let height = ticks(pv.get("height"), &format!("{what}.height"))?;
        if width <= 0 || height <= 0 {
            return Err(format!(
                "{what}: page size {width}x{height} ticks is not positive"
            ));
        }
        let mut items: Vec<Pending> = Vec::new();
        let mut page_fonts = BTreeSet::new();
        for (ii, iv) in arr(pv.get("items"), &format!("{what}.items"))?
            .iter()
            .enumerate()
        {
            let iw = format!("{what}.items[{ii}]");
            let kind = s(iv.get("kind"), &format!("{iw}.kind"))?;
            let pt = paint(iv.get("paint"), &iw)?;
            match kind {
                "glyph_run" => {
                    let font_id = s(iv.get("font_id"), &format!("{iw}.font_id"))?;
                    let entry = fonts.get(font_id).ok_or_else(|| {
                        format!("{iw}: font_id {font_id} is not in payload.fonts")
                    })?;
                    if entry.format == "core14-afm" {
                        return Err(format!(
                            "{iw}: font {} ({font_id}) is format core14-afm, which carries no program bytes; the exact route needs the font program (render-pipeline: ship bytes or emit a real font resource)",
                            entry.postscript_name
                        ));
                    }
                    let size = ticks(iv.get("font_size"), &format!("{iw}.font_size"))?;
                    if size <= 0 {
                        return Err(format!("{iw}: font_size {size} ticks is not positive"));
                    }
                    let text = s(iv.get("text"), &format!("{iw}.text"))?;
                    let clusters = arr(iv.get("clusters"), &format!("{iw}.clusters"))?;
                    let u = used.entry(font_id.to_string()).or_insert_with(|| Used {
                        gids: BTreeSet::new(),
                        to_unicode: BTreeMap::new(),
                        shared: BTreeMap::new(),
                        conflicts: 0,
                        advances: BTreeMap::new(),
                    });
                    let mut glyphs = Vec::new();
                    let glyph_values = arr(iv.get("glyphs"), &format!("{iw}.glyphs"))?;
                    // Glyphs per cluster index (out-of-range indices are
                    // refused below, glyph by glyph).
                    let mut cluster_glyphs: BTreeMap<usize, usize> = BTreeMap::new();
                    for gv in glyph_values.iter() {
                        if let Some(c) = gv.get("cluster").and_then(|c| c.as_f64()) {
                            *cluster_glyphs.entry(c as usize).or_default() += 1;
                        }
                    }
                    for (gi, gv) in glyph_values.iter().enumerate() {
                        let gw = format!("{iw}.glyphs[{gi}]");
                        let gid = f(gv.get("gid"), &format!("{gw}.gid"))?;
                        if gid.fract() != 0.0 || !(0.0..=65535.0).contains(&gid) {
                            return Err(format!("{gw}: gid {gid} is not a glyph id"));
                        }
                        let gid = gid as u16;
                        if gid == 0 {
                            return Err(format!(
                                "{gw}: gid 0 (missing glyph) must not be exported"
                            ));
                        }
                        if u64::from(gid) >= entry.glyph_count {
                            return Err(format!(
                                "{gw}: gid {gid} is outside the font's {} glyphs",
                                entry.glyph_count
                            ));
                        }
                        let origin_x = ticks(gv.get("origin_x"), &format!("{gw}.origin_x"))?;
                        let baseline_y = ticks(gv.get("baseline_y"), &format!("{gw}.baseline_y"))?;
                        let advance_x = ticks(gv.get("advance_x"), &format!("{gw}.advance_x"))?;
                        let advance_y = ticks(gv.get("advance_y"), &format!("{gw}.advance_y"))?;
                        let cluster = f(gv.get("cluster"), &format!("{gw}.cluster"))? as usize;
                        let cv = clusters
                            .get(cluster)
                            .ok_or_else(|| format!("{gw}: cluster {cluster} is out of range"))?;
                        let a = f(cv.get("text_start_byte"), "cluster.text_start_byte")? as usize;
                        let b = f(cv.get("text_end_byte"), "cluster.text_end_byte")? as usize;
                        let cluster_text = text.get(a..b).unwrap_or("").to_string();
                        if let Some(t) = text.get(a..b) {
                            if cluster_glyphs.get(&cluster).copied().unwrap_or(0) > 1 {
                                u.shared.entry(gid).or_insert_with(|| t.to_string());
                            } else {
                                match u.to_unicode.get(&gid) {
                                    Some(prev) if prev != t => u.conflicts += 1,
                                    Some(_) => {}
                                    None => {
                                        u.to_unicode.insert(gid, t.to_string());
                                    }
                                }
                            }
                        }
                        u.gids.insert(gid);
                        // Absolute origins are authoritative and advances are never
                        // added to them; advance_x only informs the /W width (below).
                        if advance_x >= 0 {
                            let r = Ratio::new(advance_x * 1000, size);
                            *u.advances
                                .entry(gid)
                                .or_default()
                                .entry((r.num, r.den))
                                .or_default() += 1;
                        }
                        glyphs.push(Glyph {
                            gid,
                            origin_x,
                            y_pdf: height - baseline_y,
                            advance_y,
                            text: cluster_text,
                        });
                        report.glyphs += 1;
                    }
                    if glyphs.is_empty() {
                        return Err(format!("{iw}: glyph run without glyphs"));
                    }
                    report.runs += 1;
                    page_fonts.insert(entry.resource.clone());
                    items.push(Pending::Run {
                        resource: entry.resource.clone(),
                        size_ticks: size,
                        color: pt.ops.clone(),
                        glyphs,
                    });
                }
                "rule" => {
                    let x = ticks(iv.get("x"), &format!("{iw}.x"))?;
                    let top = ticks(iv.get("top"), &format!("{iw}.top"))?;
                    let w = ticks(iv.get("width"), &format!("{iw}.width"))?;
                    let h = ticks(iv.get("height"), &format!("{iw}.height"))?;
                    if w <= 0 || h <= 0 {
                        return Err(format!("{iw}: rule {w}x{h} ticks is not positive"));
                    }
                    let mut ops = Vec::new();
                    if let Some(color) = &pt.ops {
                        ops.push(Op::Save);
                        ops.extend(color.iter().cloned());
                    }
                    ops.extend(Op::rule(bp(x)?, bp(height - top - h)?, bp(w)?, bp(h)?));
                    if pt.ops.is_some() {
                        ops.push(Op::Restore);
                    }
                    report.rules += 1;
                    items.push(Pending::Ops(ops));
                }
                "image" => {
                    let w = ticks(iv.get("width"), &format!("{iw}.width"))?;
                    let h = ticks(iv.get("height"), &format!("{iw}.height"))?;
                    if w <= 0 || h <= 0 {
                        return Err(format!("{iw}: image box {w}x{h} ticks is not positive"));
                    }
                    let t = arr(iv.get("transform"), &format!("{iw}.transform"))?;
                    if t.len() != 6 {
                        return Err(format!(
                            "{iw}.transform: expected 6 numbers, found {}",
                            t.len()
                        ));
                    }
                    let mut transform = [0.0; 6];
                    for (k, v) in t.iter().enumerate() {
                        let n = f(Some(v), &format!("{iw}.transform[{k}]"))?;
                        if !n.is_finite() || n.abs() > 1.0e7 {
                            return Err(format!("{iw}.transform[{k}]: {n} is out of range"));
                        }
                        transform[k] = n;
                    }
                    let im = iv
                        .get("image")
                        .ok_or_else(|| format!("{iw}: image item without an image resource"))?;
                    let what = format!("{iw}.image");
                    let sha = s(im.get("sha256"), &format!("{what}.sha256"))?;
                    if sha.len() != 64
                        || !sha.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
                    {
                        return Err(format!(
                            "{what}.sha256: {sha:?} is not 64 lowercase hex digits"
                        ));
                    }
                    if let Some(id) = im.get("image_id").and_then(Value::as_str)
                        && id != sha
                    {
                        return Err(format!("{what}: image_id {id} differs from sha256 {sha}"));
                    }
                    let byte_length = f(im.get("byte_length"), &format!("{what}.byte_length"))?;
                    if byte_length.fract() != 0.0 || !(0.0..=9.0e15).contains(&byte_length) {
                        return Err(format!(
                            "{what}.byte_length: {byte_length} is not a byte count"
                        ));
                    }
                    let format = s(im.get("format"), &format!("{what}.format"))?;
                    if !matches!(format, "png" | "jpeg" | "pdf") {
                        return Err(format!("{what}.format: {format:?} is not png, jpeg or pdf"));
                    }
                    let path = s(im.get("path"), &format!("{what}.path"))?;
                    let page = if format == "pdf" {
                        let n = f(im.get("pdf_page"), &format!("{what}.pdf_page"))?;
                        if n.fract() != 0.0 || !(1.0..=1.0e6).contains(&n) {
                            return Err(format!("{what}.pdf_page: {n} is not a page number"));
                        }
                        n as u32
                    } else {
                        0
                    };
                    let key = (sha.to_string(), page);
                    let index = match image_index.get(&key) {
                        Some(&i) => i,
                        None => {
                            let num = |k: &str| im.get(k).and_then(Value::as_f64);
                            let pdf_box = match im.get("pdf_box").and_then(Value::as_array) {
                                Some(b) if b.len() == 4 => {
                                    let mut v = [0.0; 4];
                                    for (k, x) in b.iter().enumerate() {
                                        v[k] = f(Some(x), &format!("{what}.pdf_box[{k}]"))?;
                                    }
                                    Some(v)
                                }
                                Some(_) => {
                                    return Err(format!("{what}.pdf_box: expected 4 numbers"));
                                }
                                None => None,
                            };
                            image_requests.push(ImageRequest {
                                first_item: iw.clone(),
                                path: path.to_string(),
                                sha256: sha.to_string(),
                                byte_length: byte_length as u64,
                                format: format.to_string(),
                                page,
                                pixels: num("pixel_width").zip(num("pixel_height")),
                                pdf_box,
                                pdf_rotate: num("pdf_rotate"),
                            });
                            image_index.insert(key, image_requests.len() - 1);
                            image_requests.len() - 1
                        }
                    };
                    items.push(Pending::Image {
                        index,
                        transform,
                        height_ticks: height,
                    });
                }
                other => {
                    return Err(format!(
                        "{iw}: item kind {other:?} is not supported by the exact route (glyph_run, rule and image only)"
                    ));
                }
            }
        }
        pages_ops.push((bp(width)?, bp(height)?, items, page_fonts));
        report.pages += 1;
    }

    // Pass 2: resolve and embed the fonts that are used.
    let dirs = font_dirs(options);
    let mut exact_fonts: BTreeMap<String, ExactFont> = BTreeMap::new();
    for font_id in &order {
        let Some(u) = used.get(font_id) else {
            continue;
        };
        let entry = &fonts[font_id];
        if entry.face_index != 0 {
            return Err(format!(
                "font {}: face_index {} is not 0 (collections are not supported)",
                entry.postscript_name, entry.face_index
            ));
        }
        if !matches!(entry.format.as_str(), "opentype-cff" | "static-truetype") {
            return Err(format!(
                "font {}: format {:?} is not opentype-cff or static-truetype",
                entry.postscript_name, entry.format
            ));
        }
        if entry.sha256 != entry.font_id {
            report.notes.push(format!(
                "font {}: font_id differs from sha256; resolved by sha256",
                entry.postscript_name
            ));
        }
        let (path, hash_form) = resolve_font(
            &dirs,
            &entry.sha256,
            entry.byte_length,
            entry.face_index as u32,
        )
        .ok_or_else(|| {
            format!(
                "font {} (sha256 {}, {} bytes) was not found in {} director{}: {}",
                entry.postscript_name,
                entry.sha256,
                entry.byte_length,
                dirs.len(),
                if dirs.len() == 1 { "y" } else { "ies" },
                dirs.iter()
                    .map(|d| d.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
        if hash_form == HashForm::BytesAndFaceIndex {
            report.notes.push(format!(
                "font {}: sha256 matched SHA-256(bytes || face_index), font-engine's content hash, not SHA-256(bytes) as rendering-core's validator requires (render-pipeline deviation)",
                entry.postscript_name
            ));
        }
        let font = TrueTypeFont::load(&path)?;
        if u64::from(font.num_glyphs()) != entry.glyph_count
            || u64::from(font.units_per_em) != entry.units_per_em
        {
            return Err(format!(
                "font {}: envelope says {} glyphs at {} units/em but {} has {} at {}",
                entry.postscript_name,
                entry.glyph_count,
                entry.units_per_em,
                path.display(),
                font.num_glyphs(),
                font.units_per_em
            ));
        }
        let expected_format = match font.outlines {
            crate::truetype::Outlines::Cff => "opentype-cff",
            crate::truetype::Outlines::TrueType => "static-truetype",
        };
        if entry.format != expected_format {
            return Err(format!(
                "font {}: envelope format {:?} but the file has {expected_format} outlines",
                entry.postscript_name, entry.format
            ));
        }
        let mut to_unicode = u.to_unicode.clone();
        for (gid, text) in &u.shared {
            to_unicode
                .entry(*gid)
                .or_insert_with(|| font.char_for_glyph(*gid).map_or_else(|| text.clone(), String::from));
        }
        let (mut exact, outcome, note) =
            ExactFont::cid_from_opentype(&font, &u.gids, to_unicode)
                .map_err(|e| e.to_string())?;
        let replaced = apply_display_widths(&mut exact, &u.advances);
        if replaced > 0 {
            report.display_widths += replaced;
            report.notes.push(format!(
                "font {}: {replaced} /W width(s) taken from the display list's advances where they differ from hmtx (producer metrics; keeps word boundaries in text extraction)",
                entry.postscript_name
            ));
        }
        if u.conflicts > 0 {
            report.notes.push(format!(
                "font {}: {} glyph occurrence(s) with a different cluster text than the first; ToUnicode keeps the first mapping (cluster ActualText needs marked content, outside the bounded set)",
                entry.postscript_name, u.conflicts
            ));
        }
        report.fonts.push(FontNote {
            resource: entry.resource.clone(),
            font_id: font_id.clone(),
            postscript_name: entry.postscript_name.clone(),
            path: path.clone(),
            hash_form,
            glyphs: u.gids.len(),
            outcome,
            program_bytes: exact.program_identity().map_or(0, |(n, _)| n),
            note,
        });
        exact_fonts.insert(entry.resource.clone(), exact);
    }

    // Pass 2b: read, verify and convert the images (first-use order: Im1, Im2, …).
    let mut exact_images: BTreeMap<String, images::ImageXObject> = BTreeMap::new();
    let mut image_names: Vec<String> = Vec::with_capacity(image_requests.len());
    if !image_requests.is_empty() {
        let root = project_root.ok_or_else(|| {
            format!(
                "{}: image {:?} needs a project root (from_v2_rooted, --project-root DIR); images are read only under it",
                image_requests[0].first_item, image_requests[0].path
            )
        })?;
        for (n, r) in image_requests.iter().enumerate() {
            let name = format!("Im{}", n + 1);
            let fail = |e: String| format!("{}: image {:?}: {e}", r.first_item, r.path);
            let bytes =
                images::read_verified(root, &r.path, r.byte_length, &r.sha256).map_err(fail)?;
            let x = match r.format.as_str() {
                "png" => images::from_png(&bytes),
                "jpeg" => images::from_jpeg(&bytes),
                _ => images::from_pdf_page(&bytes, r.page),
            }
            .map_err(fail)?;
            match &x.geometry {
                Geometry::Raster { width, height } => {
                    if let Some((pw, ph)) = r.pixels
                        && (pw != f64::from(*width) || ph != f64::from(*height))
                    {
                        return Err(fail(format!(
                            "decoded {width}x{height} px, the display list sized {pw}x{ph} px"
                        )));
                    }
                }
                Geometry::Form { bbox, rotate } => {
                    if let Some(b) = r.pdf_box {
                        for (k, t) in bbox.iter().enumerate() {
                            let v: f64 = t.parse().unwrap_or(f64::NAN);
                            if !((v - b[k]).abs() <= 1.0e-3) {
                                return Err(fail(format!(
                                    "page box is [{}], the display list sized [{} {} {} {}]",
                                    bbox.join(" "),
                                    b[0],
                                    b[1],
                                    b[2],
                                    b[3]
                                )));
                            }
                        }
                    }
                    if let Some(rot) = r.pdf_rotate
                        && rot != f64::from(*rotate)
                    {
                        return Err(fail(format!(
                            "page /Rotate is {rotate}, the display list says {rot}"
                        )));
                    }
                }
            }
            report
                .notes
                .push(format!("image /{name} {}: {}", r.path, x.summary));
            exact_images.insert(name.clone(), x);
            image_names.push(name);
        }
        report.image_resources = image_names.len();
    }

    // Pass 3: finish the glyph runs now that advances are known.
    let mut pages = Vec::with_capacity(pages_ops.len());
    for (pi, (width, height, items, page_fonts)) in pages_ops.into_iter().enumerate() {
        let mut ops = Vec::new();
        // The pen after the last glyph placed on this page (ticks), its
        // baseline, size and text: gaps across run boundaries are the word
        // boundaries an extractor reads from geometry.
        let mut last: Option<(i128, i128, i128, String)> = None;
        for item in items {
            let (resource, size_ticks, color, glyphs) = match item {
                Pending::Ops(o) => {
                    ops.extend(o);
                    continue;
                }
                Pending::Image {
                    index,
                    transform,
                    height_ticks,
                } => {
                    let name = &image_names[index];
                    let unit = unit_to_pdf(&transform, height_ticks)?;
                    ops.extend(exact_images[name].placement(name, unit)?);
                    report.images += 1;
                    continue;
                }
                Pending::Run {
                    resource,
                    size_ticks,
                    color,
                    glyphs,
                } => (resource, size_ticks, color, glyphs),
            };
            let widths = cid_widths(&exact_fonts[&resource]);
            let mut placed = Vec::with_capacity(glyphs.len());
            let mut prev: Option<&Glyph> = None;
            for g in &glyphs {
                if let Some((pen, y, size, ref ptext)) = last
                    && y == g.y_pdf
                {
                    // gap / size in thousandths, compared as integers.
                    let gap = (g.origin_x - pen) * 1000;
                    if gap >= WORD_GAP_EM * size {
                        report.word_gaps += 1;
                    } else if gap >= CHAR_GAP_EM * size {
                        report.ambiguous_gaps += 1;
                        if report.ambiguous_gaps <= 8 {
                            report.notes.push(format!(
                                "page {}: gap of {}/1000 em between {ptext:?} and {:?} is between {CHAR_GAP_EM} and {WORD_GAP_EM}/1000 em; extractors disagree on a word boundary there (geometry is the producer's and is kept)",
                                pi + 1,
                                gap / size,
                                g.text
                            ));
                        }
                    }
                }
                let w = widths
                    .get(&g.gid)
                    .map_or(Ratio::int(1000), Ratio::from_decimal);
                // pen = origin + w/1000 * size, rounded down to a tick.
                let pen = g.origin_x + w.num * size_ticks / (1000 * w.den);
                last = Some((pen, g.y_pdf, size_ticks, g.text.clone()));
                // Continue the previous glyph's segment when the gap between
                // the envelope origin and the natural advance is an exactly
                // representable TJ adjustment: with `w` the written /W width
                // (wn/wd in 1000/em) the viewer places this glyph at
                // prev + (w - n)/1000 * size, so n = w - 1000*delta/size.
                let adjust: Option<Option<Decimal>> = prev.and_then(|p| {
                    if p.y_pdf != g.y_pdf || p.advance_y != 0 {
                        return None;
                    }
                    let w = widths
                        .get(&p.gid)
                        .map_or(Ratio::int(1000), Ratio::from_decimal);
                    let delta = g.origin_x - p.origin_x;
                    // n = (wn*size - 1000*delta*wd) / (wd*size)
                    let num = w.num * size_ticks - 1000 * delta * w.den;
                    let den = (w.den * size_ticks) as u128;
                    if num == 0 {
                        return Some(None);
                    }
                    Decimal::from_ratio(num, den, 20).map(Some)
                });
                match adjust {
                    Some(None) => report.joined_glyphs += 1,
                    Some(Some(_)) => report.kerned_glyphs += 1,
                    None => {}
                }
                placed.push(PlacedGlyph {
                    gid: g.gid,
                    origin: if adjust.is_some() {
                        None
                    } else {
                        Some((bp(g.origin_x)?, bp(g.y_pdf)?))
                    },
                    adjust: adjust.flatten(),
                });
                prev = Some(g);
            }
            let run = GlyphRun {
                font: resource,
                size: bp(size_ticks)?,
                glyphs: placed,
            };
            let run_ops = run.to_ops().map_err(|e| e.to_string())?;
            if let Some(color) = color {
                ops.push(Op::Save);
                ops.extend(color);
                ops.extend(run_ops);
                ops.push(Op::Restore);
            } else {
                ops.extend(run_ops);
            }
        }
        pages.push(ExactPage {
            width,
            height,
            content: Content::Ops(ops),
            fonts: Some(page_fonts.into_iter().collect()),
        });
    }
    if pages.is_empty() {
        return Err("display list has no pages".into());
    }
    Ok((
        ExactDocument {
            pages,
            fonts: exact_fonts,
            images: exact_images,
        },
        report,
    ))
}

/// The display list's number as a PDF token: Rust's shortest round-trip
/// decimal (never an exponent), e.g. `148.712`.
fn json_decimal(v: f64) -> Result<Decimal, String> {
    let s = format!("{}", v + 0.0);
    Decimal::new(&s).map_err(|e| format!("transform value {v}: {e}"))
}

fn negate(d: &Decimal) -> Decimal {
    let t = d.as_str();
    let s = match t.strip_prefix('-') {
        Some(rest) => rest.to_string(),
        None if Ratio::from_decimal(d) == Ratio::int(0) => "0".to_string(),
        None => format!("-{t}"),
    };
    Decimal::new(&s).expect("negated decimal token")
}

/// `ticks / 2^20 - f`, exactly: `f` has at most 18 fractional digits, so
/// the result terminates within `20 + 18` digits.
fn ticks_minus(ticks: i128, f: &Decimal) -> Result<Decimal, String> {
    let t = f.as_str();
    let (neg, body) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let (int, frac) = body.split_once('.').unwrap_or((body, ""));
    if frac.len() > 18 || int.len() > 18 {
        return Err(format!("transform value {t} has too many digits"));
    }
    let digits = format!("{int}{frac}");
    let mut m: i128 = if digits.is_empty() {
        0
    } else {
        digits.parse().map_err(|_| format!("bad number {t}"))?
    };
    if neg {
        m = -m;
    }
    let scale = 10i128.pow(frac.len() as u32);
    let num = ticks * scale - m * TICKS_PER_BP;
    Decimal::from_ratio(num, (TICKS_PER_BP * scale) as u128, 20 + frac.len())
        .ok_or_else(|| format!("page height minus {t} does not terminate"))
}

/// The contract's transform (`page_x = e + a·u + c·v`, `page_y = f + b·u +
/// d·v`, y down) as a `cm` in PDF's y-up page space: `[a, -b, c, -d, e, H - f]`
/// (proposal §5.4). The operands are the display list's own decimals.
pub fn unit_to_pdf(t: &[f64; 6], page_height_ticks: i128) -> Result<[Decimal; 6], String> {
    let a = json_decimal(t[0])?;
    let b = json_decimal(t[1])?;
    let c = json_decimal(t[2])?;
    let d = json_decimal(t[3])?;
    let e = json_decimal(t[4])?;
    let f = json_decimal(t[5])?;
    Ok([
        a,
        negate(&b),
        c,
        negate(&d),
        e,
        ticks_minus(page_height_ticks, &f)?,
    ])
}

/// The `/W` map of a CID font (empty for simple fonts).
fn cid_widths(font: &ExactFont) -> BTreeMap<u16, Decimal> {
    match font {
        ExactFont::CidCff(c) | ExactFont::CidTrueType(c) => c.widths.clone(),
        ExactFont::Simple(_) => BTreeMap::new(),
    }
}

/// Writes the producer's own kern-free advance as the `/W` width of every
/// glyph whose observed advance differs from the hmtx value. Positions are
/// unaffected (every glyph is placed by its envelope origin); what changes
/// is where a viewer believes the pen is after the glyph, which is what
/// text extraction uses to decide word boundaries: Latin Modern's OpenType
/// hmtx gives `W` in the bold face 1189/1000 em while the TFM metrics the
/// producer laid out with give 1093, so with hmtx widths PDFKit read
/// "Wednesday," as "W ednesday ,". The most frequent observed ratio per
/// glyph is taken (kerns to a following glyph are folded into `advance_x`
/// by the producer and are the minority); ties go to the larger width.
/// Returns how many entries were replaced.
fn apply_display_widths(
    font: &mut ExactFont,
    advances: &BTreeMap<u16, BTreeMap<(i128, i128), usize>>,
) -> usize {
    let cid = match font {
        ExactFont::CidCff(c) | ExactFont::CidTrueType(c) => c,
        ExactFont::Simple(_) => return 0,
    };
    let mut replaced = 0;
    for (gid, seen) in advances {
        let Some(((num, den), _)) = seen.iter().max_by(|((an, ad), ac), ((bn, bd), bc)| {
            ac.cmp(bc).then_with(|| (an * bd).cmp(&(bn * ad)))
        }) else {
            continue;
        };
        let value = match Decimal::from_ratio(*num, *den as u128, 12) {
            Some(d) => d,
            None => {
                let rounded = (*num as f64 / *den as f64 * 10000.0).round() / 10000.0;
                match Decimal::new(&format!("{rounded}")) {
                    Ok(d) => d,
                    Err(_) => continue,
                }
            }
        };
        match cid.widths.get(gid) {
            Some(current) if Ratio::from_decimal(current) == Ratio::from_decimal(&value) => {}
            Some(_) => {
                cid.widths.insert(*gid, value);
                replaced += 1;
            }
            None => {}
        }
    }
    replaced
}

/// Convenience for callers with a path.
pub fn from_v2_file(path: &Path, options: &V2Options) -> Result<(ExactDocument, V2Report), String> {
    from_v2_file_rooted(path, options, None)
}

/// [`from_v2_rooted`] for callers with a path.
pub fn from_v2_file_rooted(
    path: &Path,
    options: &V2Options,
    project_root: Option<&Path>,
) -> Result<(ExactDocument, V2Report), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    from_v2_rooted(&text, options, project_root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_to_bp_is_exact() {
        assert_eq!(bp(75_497_472).unwrap().as_str(), "72");
        assert_eq!(bp(1).unwrap().as_str(), "0.00000095367431640625");
        assert_eq!(bp(-524_288).unwrap().as_str(), "-0.5");
        assert_eq!(bp(88_033_374).unwrap().as_str(), "83.9551677703857421875");
    }

    #[test]
    fn colour_components_are_exact_or_refused() {
        assert_eq!(exact_unit(0.5, "c").unwrap().as_str(), "0.5");
        assert_eq!(exact_unit(0.125, "c").unwrap().as_str(), "0.125");
        assert!(
            exact_unit(0.1, "c").is_err(),
            "0.1 is not a short binary fraction"
        );
        assert!(exact_unit(1.5, "c").is_err());
    }

    #[test]
    fn rejects_wrong_envelopes() {
        let e = from_v2(
            r#"{"protocol_version":1,"type":"compile","payload":{}}"#,
            &V2Options::default(),
        )
        .unwrap_err();
        assert!(e.contains("protocol_version 1"), "{e}");
        let e = from_v2(
            r#"{"protocol_version":2,"type":"display_list","payload":{"render_format":"display-list-v2","coordinate_unit":"pt","color_space":"srgb","fonts":[],"pages":[]}}"#,
            &V2Options::default(),
        )
        .unwrap_err();
        assert!(e.contains("coordinate_unit"), "{e}");
        let e = from_v2(
            r#"{"protocol_version":2,"type":"display_list","payload":{"render_format":"display-list-v2","coordinate_unit":"bp_2pow20","color_space":"srgb","fonts":[{"font_id":"x","sha256":"x","byte_length":1,"format":"core14-afm","face_index":0,"units_per_em":1000,"glyph_count":300,"postscript_name":"Times-Roman"}],"pages":[{"number":1,"width":1048576,"height":1048576,"items":[{"kind":"glyph_run","font_id":"x","font_size":1048576,"text":"a","paint":{"r":0,"g":0,"b":0,"a":1},"glyphs":[{"gid":5,"origin_x":0,"baseline_y":0,"advance_x":0,"advance_y":0,"cluster":0}],"clusters":[{"text_start_byte":0,"text_end_byte":1}]}]}]}}"#,
            &V2Options::default(),
        )
        .unwrap_err();
        assert!(e.contains("core14-afm"), "{e}");
    }
}
