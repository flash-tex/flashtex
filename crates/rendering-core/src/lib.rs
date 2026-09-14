//! Original Rust model/validator for the experimental rendering-v2 proposal.
//! No runtime-v1 producer or renderer is changed. Successful validation does not
//! establish font outline safety, shaping correctness, visual parity or permission
//! to paint. The caller must negotiate/integrate actual consumers separately.

pub mod batch;
pub mod cache;
pub mod font_adapter;
pub mod glyph_cache;
pub mod hit_test;
pub mod outlines;
pub mod transform;
pub mod wire;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const MAX_MESSAGE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_EXACT_INTEGER: i64 = (1i64 << 53) - 1;
pub const TICKS_PER_BP: i64 = 1 << 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError(pub String);
impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ValidationError {}
pub type Result<T> = std::result::Result<T, ValidationError>;
fn require(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(ValidationError(message.into()))
    }
}
fn bounded_len(length: usize, minimum: usize, maximum: usize, label: &str) -> Result<()> {
    require((minimum..=maximum).contains(&length), label)
}
fn text(value: &str, min: usize, max: usize, label: &str) -> Result<()> {
    bounded_len(value.chars().count(), min, max, label)
}
fn id(value: &str) -> Result<()> {
    require(
        !value.is_empty()
            && value.len() <= 128
            && value.as_bytes()[0].is_ascii_alphanumeric()
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b)),
        "invalid identifier",
    )
}
fn path(value: &str) -> Result<()> {
    text(value, 1, 4096, "invalid path length")?;
    require(
        !value.contains(['\\', ':', '\0'])
            && value.split('/').all(|p| !matches!(p, "" | "." | "..")),
        "invalid project path",
    )
}
fn exact_unsigned(value: u64) -> Result<()> {
    require(
        value <= MAX_EXACT_INTEGER as u64,
        "integer exceeds JSON exact range",
    )
}
fn hash(value: &str) -> Result<()> {
    require(
        value.len() == 64
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "invalid SHA256",
    )
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct Tick(pub i64);
impl Tick {
    pub fn validate(self) -> Result<()> {
        require(
            (-MAX_EXACT_INTEGER..=MAX_EXACT_INTEGER).contains(&self.0),
            "tick outside exact range",
        )
    }
    pub fn positive(self) -> Result<()> {
        self.validate()?;
        require(self.0 > 0, "dimension must be positive")
    }
    pub fn nonnegative(self) -> Result<()> {
        self.validate()?;
        require(self.0 >= 0, "dimension must be nonnegative")
    }
    pub fn checked_add(self, other: Self) -> Result<Self> {
        let sum = self
            .0
            .checked_add(other.0)
            .ok_or_else(|| ValidationError("tick overflow".into()))?;
        let tick = Self(sum);
        tick.validate()?;
        Ok(tick)
    }
    /// Exact conversion of TeX scaled points to canonical ticks, ties to even.
    pub fn from_tex_sp(sp: i64) -> Result<Self> {
        let numerator = i128::from(sp) * 7200 * i128::from(TICKS_PER_BP);
        let denominator = 7227i128 * 65536;
        let magnitude = numerator.abs();
        let mut quotient = magnitude / denominator;
        let remainder = magnitude % denominator;
        if remainder * 2 > denominator || (remainder * 2 == denominator && quotient % 2 != 0) {
            quotient += 1;
        }
        let signed = if numerator < 0 { -quotient } else { quotient };
        let tick = Self(
            i64::try_from(signed).map_err(|_| ValidationError("TeX conversion overflow".into()))?,
        );
        tick.validate()?;
        Ok(tick)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    GlyphRun,
    Rule,
    #[serde(rename = "static-truetype")]
    StaticTrueType,
    #[serde(rename = "rgba-srgb")]
    RgbaSrgb,
    #[serde(rename = "cluster-actualtext")]
    ClusterActualText,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RenderFormat {
    #[serde(rename = "runtime-v1")]
    RuntimeV1,
    #[serde(rename = "display-list-v2")]
    DisplayListV2,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    pub render_formats: Vec<RenderFormat>,
    pub features: Vec<Feature>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub render_format: RenderFormat,
    pub required_features: Vec<Feature>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionCode {
    UnsupportedFormat,
    UnsupportedFeature,
    InvalidResource,
    InvalidDisplayList,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rejection {
    pub code: RejectionCode,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRange {
    pub path: String,
    pub start_byte: u64,
    pub end_byte: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentResource {
    pub path: String,
    pub revision: u64,
    pub sha256: String,
    pub byte_length: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontResource {
    pub font_id: String,
    pub sha256: String,
    pub byte_length: u64,
    pub format: String,
    pub face_index: u32,
    pub units_per_em: u32,
    pub glyph_count: u32,
    pub postscript_name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Paint {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}
impl Paint {
    fn validate(&self) -> Result<()> {
        require(
            [self.r, self.g, self.b, self.a]
                .iter()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v)),
            "invalid RGBA paint",
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HitRect {
    pub x: Tick,
    pub top: Tick,
    pub width: Tick,
    pub height: Tick,
}
impl HitRect {
    fn validate(&self) -> Result<()> {
        self.x.validate()?;
        self.top.validate()?;
        self.width.nonnegative()?;
        self.height.nonnegative()?;
        self.x.checked_add(self.width)?;
        self.top.checked_add(self.height)?;
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Caret {
    pub text_byte: u64,
    pub x: Tick,
    pub top: Tick,
    pub height: Tick,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cluster {
    pub text_start_byte: u64,
    pub text_end_byte: u64,
    pub hit_rects: Vec<HitRect>,
    pub carets: Vec<Caret>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<SourceRange>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synthetic_reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Glyph {
    pub gid: u32,
    pub origin_x: Tick,
    pub baseline_y: Tick,
    pub advance_x: Tick,
    pub advance_y: Tick,
    pub cluster: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlyphRun {
    pub font_id: String,
    pub font_size: Tick,
    pub text: String,
    pub glyphs: Vec<Glyph>,
    pub clusters: Vec<Cluster>,
    pub paint: Paint,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub x: Tick,
    pub top: Tick,
    pub width: Tick,
    pub height: Tick,
    pub paint: Paint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<SourceRange>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synthetic_reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Item {
    #[serde(rename = "glyph_run")]
    GlyphRun(GlyphRun),
    #[serde(rename = "rule")]
    Rule(Rule),
}
// PROPOSAL display-list-v2-window: a page outside the negotiated window is
// elided — it decodes/encodes with no `items` key and `resident: false`
// instead. `Page` keeps the pre-proposal `items` accessor working (empty for
// an elided page) by hand-rolling (de)serialization around a private wire
// shape rather than deriving it, so every existing resident page round-trips
// byte-identically (no `resident` key ever appears for it).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PageWireDe {
    number: u32,
    width: Tick,
    height: Tick,
    #[serde(default)]
    items: Option<Vec<Item>>,
    #[serde(default)]
    resident: Option<bool>,
}
#[derive(Serialize)]
struct PageWireSer<'a> {
    number: u32,
    width: Tick,
    height: Tick,
    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<&'a [Item]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resident: Option<bool>,
}
#[derive(Debug, Clone)]
pub struct Page {
    pub number: u32,
    pub width: Tick,
    pub height: Tick,
    /// Empty for an elided (non-resident) page.
    pub items: Vec<Item>,
    resident: bool,
}
impl Page {
    /// PROPOSAL display-list-v2-window: false only for a page elided from a
    /// windowed display list (no `items`, wire `resident: false`).
    pub fn is_resident(&self) -> bool {
        self.resident
    }
}
impl<'de> Deserialize<'de> for Page {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = PageWireDe::deserialize(deserializer)?;
        match (wire.items, wire.resident) {
            (Some(items), None) => Ok(Page {
                number: wire.number,
                width: wire.width,
                height: wire.height,
                items,
                resident: true,
            }),
            (None, Some(false)) => Ok(Page {
                number: wire.number,
                width: wire.width,
                height: wire.height,
                items: Vec::new(),
                resident: false,
            }),
            // Fail closed: a consumer that never learned about windows must
            // be refused, not shown a blank page. This rejects `items`
            // absent without an explicit `resident: false`, and any other
            // combination (both present, or an explicit `resident: true`).
            _ => Err(serde::de::Error::custom(
                "page must have `items`, or an explicit `resident: false` and no `items`",
            )),
        }
    }
}
impl Serialize for Page {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let wire = if self.resident {
            PageWireSer {
                number: self.number,
                width: self.width,
                height: self.height,
                items: Some(&self.items),
                resident: None,
            }
        } else {
            PageWireSer {
                number: self.number,
                width: self.width,
                height: self.height,
                items: None,
                resident: Some(false),
            }
        };
        wire.serialize(serializer)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Error,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub severity: Severity,
    pub sources: Vec<SourceRange>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisplayList {
    pub render_format: RenderFormat,
    pub coordinate_unit: String,
    pub color_space: String,
    pub text_extraction: String,
    pub project_id: String,
    pub revision: u64,
    pub required_features: Vec<Feature>,
    pub documents: Vec<DocumentResource>,
    pub fonts: Vec<FontResource>,
    pub pages: Vec<Page>,
    pub diagnostics: Vec<Diagnostic>,
    /// PROPOSAL display-list-v2-window: present only when the list was
    /// produced under a negotiated page window. Omitted (not `null`) when
    /// absent so an unwindowed list serializes byte-identically to before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<PageWindow>,
}
/// PROPOSAL display-list-v2-window: the negotiated window, echoed verbatim.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PageWindow {
    pub first_page: u32,
    pub page_count: u32,
    pub document_page_count: u32,
}
#[derive(Debug, Clone)]
pub enum Message {
    Offer(Capabilities),
    Selected(Selection),
    Rejected(Rejection),
    DisplayList(DisplayList),
}
#[derive(Debug, Clone)]
pub struct Envelope {
    pub id: String,
    pub message: Message,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEnvelope {
    protocol_version: u8,
    id: String,
    #[serde(rename = "type")]
    kind: String,
    payload: Box<serde_json::value::RawValue>,
}
fn decode<T: serde::de::DeserializeOwned>(value: &serde_json::value::RawValue) -> Result<T> {
    serde_json::from_str(value.get())
        .map_err(|error| ValidationError(format!("invalid typed payload: {error}")))
}
/// Parse bounded JSON. No unknown item or message is silently skipped.
pub fn parse(bytes: &[u8]) -> Result<Envelope> {
    require(bytes.len() <= MAX_MESSAGE_BYTES, "message exceeds 32 MiB")?;
    let raw: RawEnvelope = serde_json::from_slice(bytes)
        .map_err(|error| ValidationError(format!("invalid envelope: {error}")))?;
    require(
        raw.protocol_version == 2,
        "experimental protocol version 2 required",
    )?;
    id(&raw.id)?;
    let message = match raw.kind.as_str() {
        "render_capabilities" => Message::Offer(decode(&raw.payload)?),
        "render_format_selected" => Message::Selected(decode(&raw.payload)?),
        "render_format_rejected" => Message::Rejected(decode(&raw.payload)?),
        "display_list" => Message::DisplayList(decode(&raw.payload)?),
        _ => return Err(ValidationError("unsupported message type".into())),
    };
    Ok(Envelope {
        id: raw.id,
        message,
    })
}

fn features(values: &[Feature], min: usize) -> Result<BTreeSet<Feature>> {
    bounded_len(values.len(), min, 5, "invalid feature list")?;
    let set: BTreeSet<_> = values.iter().cloned().collect();
    require(set.len() == values.len(), "duplicate feature")?;
    Ok(set)
}
impl Capabilities {
    pub fn validate(&self) -> Result<()> {
        bounded_len(self.render_formats.len(), 1, 2, "invalid formats")?;
        require(
            self.render_formats.len() != 2 || self.render_formats[0] != self.render_formats[1],
            "duplicate format",
        )?;
        features(&self.features, 0)?;
        Ok(())
    }
    fn accepts(&self, required: &[Feature]) -> Result<()> {
        self.validate()?;
        require(
            self.render_formats.contains(&RenderFormat::DisplayListV2),
            "v2 not offered",
        )?;
        require(
            features(required, 1)?.is_subset(&features(&self.features, 0)?),
            "unsupported required capability",
        )
    }
}
impl Envelope {
    /// Selection correlates to offer ID; display request IDs remain separate.
    pub fn validate(&self, offer: Option<&Envelope>) -> Result<()> {
        id(&self.id)?;
        match &self.message {
            Message::Offer(capabilities) => capabilities.validate(),
            Message::Rejected(reason) => {
                text(&reason.message, 1, 4096, "invalid rejection message")
            }
            Message::Selected(selection) => {
                let (offer_id, capabilities) = offered(offer)?;
                require(self.id == offer_id, "handshake correlation mismatch")?;
                require(
                    selection.render_format == RenderFormat::DisplayListV2,
                    "v2 selection required",
                )?;
                capabilities.accepts(&selection.required_features)
            }
            Message::DisplayList(list) => {
                let (_, capabilities) = offered(offer)?;
                list.validate(capabilities)
            }
        }
    }
}
fn offered(offer: Option<&Envelope>) -> Result<(&str, &Capabilities)> {
    match offer {
        Some(Envelope {
            id: offer_id,
            message: Message::Offer(capabilities),
        }) => {
            id(offer_id)?;
            Ok((offer_id, capabilities))
        }
        _ => Err(ValidationError("explicit capability offer required".into())),
    }
}

impl DisplayList {
    pub fn validate(&self, capabilities: &Capabilities) -> Result<()> {
        self.validate_profile(capabilities, false)
    }
    pub(crate) fn validate_profile(&self, capabilities: &Capabilities, cff: bool) -> Result<()> {
        capabilities.accepts(&self.required_features)?;
        require(
            self.render_format == RenderFormat::DisplayListV2
                && self.coordinate_unit == "bp_2pow20"
                && self.color_space == "srgb"
                && self.text_extraction == "cluster-actualtext",
            "unsupported display configuration",
        )?;
        id(&self.project_id)?;
        exact_unsigned(self.revision)?;
        bounded_len(self.documents.len(), 1, 4096, "document count")?;
        bounded_len(self.fonts.len(), 0, 256, "font count")?;
        bounded_len(self.pages.len(), 0, 10000, "page count")?;
        bounded_len(self.diagnostics.len(), 0, 10000, "diagnostic count")?;
        let mut docs = BTreeMap::new();
        for document in &self.documents {
            path(&document.path)?;
            exact_unsigned(document.revision)?;
            hash(&document.sha256)?;
            require(document.byte_length <= 8388608, "source exceeds 8 MiB")?;
            require(
                docs.insert(document.path.as_str(), document).is_none(),
                "duplicate source path",
            )?;
        }
        let mut fonts = BTreeMap::new();
        for font in &self.fonts {
            id(&font.font_id)?;
            hash(&font.sha256)?;
            require(
                (1..=67108864).contains(&font.byte_length),
                "font byte length",
            )?;
            require(
                (font.format == "static-truetype" || (cff && font.format == "opentype-cff"))
                    && font.face_index == 0,
                "unsupported font profile",
            )?;
            require(
                (16..=16384).contains(&font.units_per_em)
                    && (2..=65536).contains(&font.glyph_count),
                "font metrics out of range",
            )?;
            text(&font.postscript_name, 1, 256, "PostScript name")?;
            require(
                fonts.insert(font.font_id.as_str(), font).is_none(),
                "duplicate font ID",
            )?;
        }
        let mut used = BTreeSet::from([Feature::RgbaSrgb, Feature::ClusterActualText]);
        for (index, page) in self.pages.iter().enumerate() {
            require(
                page.number as usize == index + 1,
                "page numbers must be contiguous",
            )?;
            page.width.positive()?;
            page.height.positive()?;
            // PROPOSAL display-list-v2-window: an elided page carries no
            // items (enforced by `Page`'s custom `Deserialize`), so skip
            // item-level checks for it entirely; residency itself is
            // checked against `self.window` below.
            if !page.is_resident() {
                continue;
            }
            bounded_len(page.items.len(), 0, 100000, "page item count")?;
            for item in &page.items {
                match item {
                    Item::Rule(rule) => {
                        used.insert(Feature::Rule);
                        rule.x.validate()?;
                        rule.top.validate()?;
                        rule.width.positive()?;
                        rule.height.positive()?;
                        rule.x.checked_add(rule.width)?;
                        rule.top.checked_add(rule.height)?;
                        page.height
                            .checked_add(Tick(-rule.top.0))?
                            .checked_add(Tick(-rule.height.0))?;
                        rule.paint.validate()?;
                        provenance(
                            rule.sources.as_deref(),
                            rule.synthetic_reason.as_deref(),
                            &docs,
                        )?;
                    }
                    Item::GlyphRun(run) => {
                        used.insert(Feature::GlyphRun);

                        run.font_size.positive()?;
                        run.paint.validate()?;
                        id(&run.font_id)?;
                        let font = fonts
                            .get(run.font_id.as_str())
                            .ok_or_else(|| ValidationError("unknown font ID".into()))?;
                        if font.format == "static-truetype" {
                            used.insert(Feature::StaticTrueType);
                        }
                        text(&run.text, 1, 1048576, "run text length")?;
                        bounded_len(run.glyphs.len(), 1, 65536, "glyph count")?;
                        bounded_len(run.clusters.len(), 1, 65536, "cluster count")?;
                        let mut cursor = 0u64;
                        for cluster in &run.clusters {
                            require(
                                cluster.text_start_byte == cursor && cursor < cluster.text_end_byte,
                                "clusters must partition logical text",
                            )?;
                            boundary(&run.text, cluster.text_start_byte)?;
                            boundary(&run.text, cluster.text_end_byte)?;
                            cursor = cluster.text_end_byte;
                            provenance(
                                cluster.sources.as_deref(),
                                cluster.synthetic_reason.as_deref(),
                                &docs,
                            )?;
                            bounded_len(cluster.hit_rects.len(), 1, 128, "hit rect count")?;
                            bounded_len(cluster.carets.len(), 0, 128, "caret count")?;
                            for hit in &cluster.hit_rects {
                                hit.validate()?;
                            }
                            for caret in &cluster.carets {
                                require(
                                    (cluster.text_start_byte..=cluster.text_end_byte)
                                        .contains(&caret.text_byte),
                                    "caret outside cluster",
                                )?;
                                boundary(&run.text, caret.text_byte)?;
                                caret.x.validate()?;
                                caret.top.validate()?;
                                caret.height.positive()?;
                                caret.top.checked_add(caret.height)?;
                            }
                        }
                        require(
                            cursor == run.text.len() as u64,
                            "clusters omit logical text",
                        )?;
                        let mut referenced = BTreeSet::new();
                        for glyph in &run.glyphs {
                            require(
                                glyph.gid > 0 && glyph.gid < font.glyph_count,
                                "missing or out-of-range glyph",
                            )?;
                            require(
                                (glyph.cluster as usize) < run.clusters.len(),
                                "unknown glyph cluster",
                            )?;
                            referenced.insert(glyph.cluster);
                            glyph.origin_x.validate()?;
                            glyph.baseline_y.validate()?;
                            glyph.advance_x.validate()?;
                            glyph.advance_y.validate()?;
                            glyph.origin_x.checked_add(glyph.advance_x)?;
                            glyph.baseline_y.checked_add(glyph.advance_y)?;
                            page.height.checked_add(Tick(-glyph.baseline_y.0))?;
                        }
                        require(
                            referenced.len() == run.clusters.len(),
                            "cluster without glyph mapping",
                        )?;
                    }
                }
            }
        }
        // PROPOSAL display-list-v2-window: residency must agree exactly with
        // `self.window`. Without a window every page must be resident (an
        // elided page with no window is a refusal, not a blank paint).
        match &self.window {
            Some(window) => {
                require(
                    window.first_page > 0 && window.page_count > 0,
                    "window range must be positive",
                )?;
                require(
                    window.document_page_count as usize == self.pages.len(),
                    "window document page count mismatch",
                )?;
                let start = u64::from(window.first_page);
                let end = start.saturating_add(u64::from(window.page_count));
                for page in &self.pages {
                    require(
                        page.is_resident() == (start..end).contains(&u64::from(page.number)),
                        "page residency disagrees with window",
                    )?;
                }
            }
            None => {
                require(
                    self.pages.iter().all(Page::is_resident),
                    "elided page requires a window",
                )?;
            }
        }
        for diagnostic in &self.diagnostics {
            id(&diagnostic.code)?;
            text(&diagnostic.message, 1, 4096, "diagnostic message")?;
            bounded_len(diagnostic.sources.len(), 0, 128, "diagnostic sources")?;
            for range in &diagnostic.sources {
                source(range, &docs)?;
            }
        }
        require(
            used.is_subset(&features(&self.required_features, 1)?),
            "undeclared rendering feature",
        )
    }

    /// Validate exact source bytes and font identity through a caller-owned loader.
    /// No paths are opened here and font URLs/system-name fallbacks do not exist.
    pub fn validate_resources<V: FontValidator>(
        &self,
        capabilities: &Capabilities,
        documents: &BTreeMap<String, SourceSnapshot>,
        fonts: &BTreeMap<String, Vec<u8>>,
        verifier: &V,
    ) -> Result<ResourceEvidence> {
        self.validate(capabilities)?;
        for document in &self.documents {
            let snapshot = documents
                .get(&document.path)
                .ok_or_else(|| ValidationError("missing source snapshot".into()))?;
            require(
                snapshot.revision == document.revision,
                "source revision mismatch",
            )?;
            require(
                snapshot.text.len() as u64 == document.byte_length
                    && digest(snapshot.text.as_bytes()) == document.sha256,
                "source digest mismatch",
            )?;
        }
        for font in &self.fonts {
            let bytes = fonts
                .get(&font.font_id)
                .ok_or_else(|| ValidationError("missing font bytes".into()))?;
            require(
                bytes.len() as u64 == font.byte_length && digest(bytes) == font.sha256,
                "font digest mismatch",
            )?;
            let metadata = verifier.validate_static_truetype(bytes)?;
            require(
                metadata.units_per_em == font.units_per_em
                    && metadata.glyph_count == font.glyph_count,
                "font metadata mismatch",
            )?;
        }
        for page in &self.pages {
            for item in &page.items {
                match item {
                    Item::Rule(rule) => {
                        if let Some(ranges) = &rule.sources {
                            validate_source_bytes(ranges, documents)?;
                        }
                    }
                    Item::GlyphRun(run) => {
                        for cluster in &run.clusters {
                            if let Some(ranges) = &cluster.sources {
                                validate_source_bytes(ranges, documents)?;
                            }
                        }
                    }
                }
            }
        }
        for diagnostic in &self.diagnostics {
            validate_source_bytes(&diagnostic.sources, documents)?;
        }
        Ok(ResourceEvidence {
            source_snapshots_verified: true,
            font_resources_verified: true,
            paintable: false,
        })
    }
}

fn boundary(text: &str, offset: u64) -> Result<()> {
    require(
        offset <= text.len() as u64 && text.is_char_boundary(offset as usize),
        "range splits UTF-8 or exceeds text",
    )
}
fn source(range: &SourceRange, docs: &BTreeMap<&str, &DocumentResource>) -> Result<()> {
    path(&range.path)?;
    let document = docs
        .get(range.path.as_str())
        .ok_or_else(|| ValidationError("unknown source path".into()))?;
    require(
        range.start_byte <= range.end_byte && range.end_byte <= document.byte_length,
        "source range outside document",
    )
}
fn provenance(
    ranges: Option<&[SourceRange]>,
    reason: Option<&str>,
    docs: &BTreeMap<&str, &DocumentResource>,
) -> Result<()> {
    match (ranges, reason) {
        (Some(ranges), None) => {
            bounded_len(ranges.len(), 1, 128, "source range count")?;
            for range in ranges {
                source(range, docs)?;
            }
            Ok(())
        }
        (None, Some(reason)) => text(reason, 1, 1024, "synthetic reason"),
        _ => Err(ValidationError(
            "exactly one source or synthetic provenance required".into(),
        )),
    }
}
fn validate_source_bytes(
    ranges: &[SourceRange],
    docs: &BTreeMap<String, SourceSnapshot>,
) -> Result<()> {
    for range in ranges {
        let snapshot = docs
            .get(&range.path)
            .ok_or_else(|| ValidationError("missing source snapshot".into()))?;
        boundary(&snapshot.text, range.start_byte)?;
        boundary(&snapshot.text, range.end_byte)?;
    }
    Ok(())
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
#[derive(Debug, Clone)]
pub struct SourceSnapshot {
    pub revision: u64,
    pub text: String,
}
#[derive(Debug, Clone)]
pub struct FontMetadata {
    pub units_per_em: u32,
    pub glyph_count: u32,
}
/// Adapter hook for the separately owned original `font-resources` loader.
/// Implementations must reject collections, variable/CFF fonts and malformed
/// outlines under the current static-TrueType face-zero experimental profile.
pub trait FontValidator {
    fn validate_static_truetype(&self, bytes: &[u8]) -> Result<FontMetadata>;
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceEvidence {
    pub source_snapshots_verified: bool,
    pub font_resources_verified: bool,
    /// Always false: renderer acceptance/outline painting is a separate gate.
    pub paintable: bool,
}

pub mod tex_adapter;

pub mod graph_cache;

pub mod cubic;

pub mod mixed;

pub mod mixed_replay;

pub mod residency;

pub mod cff_run;

pub mod geometry_diff;

pub mod device_grid;

pub mod shaped_run;

pub mod shaped_replay;

pub mod registry_binding;

pub mod pdf_compare;
pub mod pdf_export;
pub mod pdf_stream;

pub mod pipeline_cff;

pub mod pipeline_frame;

pub mod helper_candidate;
