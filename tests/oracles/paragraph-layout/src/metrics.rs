//! Font metrics input contract.
//!
//! Paragraph layout never opens font files. Everything it knows about a font
//! comes through [`FontMetricsSource`], which the font engine (FT-018)
//! implements. Until that engine lands, [`crate::core14`] ships a table-driven
//! adapter for the Adobe Core 14 Times faces so the breaker is testable.
//!
//! Units: metric methods return *font units* (see [`FontMetricsSource::units_per_em`]);
//! the item builder scales them by `size / units_per_em` into points. The crate
//! is unit-agnostic beyond that: whatever linear unit the caller uses for
//! `size` and line widths is the unit of every output coordinate.

use std::fmt;

/// Content-addressed font identity supplied by the font engine.
///
/// The paragraph layer treats this as an opaque 32-byte token: it is copied
/// into every [`crate::items::GlyphRun`] and [`crate::linebreak::PositionedRun`]
/// unchanged and never interpreted. FT-018 is expected to fill it with a hash
/// of the font file bytes (plus any variation coordinates); the built-in
/// Core 14 adapter fills it with a fixed ASCII label (see
/// [`FontId::from_label`]) because there is no file to hash.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontId(pub [u8; 32]);

impl FontId {
    /// Builds an identity from an ASCII label, zero-padded/truncated to 32 bytes.
    /// Only for table-driven adapters without file bytes; real fonts must use a
    /// content hash.
    pub const fn from_label(label: &str) -> FontId {
        let bytes = label.as_bytes();
        let mut out = [0u8; 32];
        let mut i = 0;
        while i < bytes.len() && i < 32 {
            out[i] = bytes[i];
            i += 1;
        }
        FontId(out)
    }

    /// Lower-case hex of the 32 bytes — the form rendering-v2's
    /// `FontResource.sha256` and manifests use when the identity is a real
    /// content hash.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// The identity as a lossy string (for diagnostics and golden tests).
    pub fn label(&self) -> String {
        let end = self.0.iter().position(|&b| b == 0).unwrap_or(32);
        self.0[..end]
            .iter()
            .map(|&b| if b.is_ascii_graphic() { b as char } else { '.' })
            .collect()
    }
}

impl fmt::Debug for FontId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FontId({})", self.label())
    }
}

/// A ligature substitution reported by a metrics source: `left`+`right`
/// collapse into the single glyph identified by `result`, whose advance and
/// glyph id are then queried through the normal methods.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ligature {
    pub result: char,
}

/// What the paragraph layer needs from a font. All lengths are in font units.
///
/// `space`, `space_stretch`, `space_shrink` and `extra_space` mirror TeX's
/// `\fontdimen2..4` and `\fontdimen7`. The defaults are the plain-TeX ratios the
/// task statement asked for (stretch = 1/2 space, shrink = 1/3 space, no extra
/// space); adapters for real fonts override them with the font's own values.
pub trait FontMetricsSource {
    /// Content-addressed identity, copied through unchanged.
    fn font_id(&self) -> FontId;
    /// Font units per em (1000 for AFM/Type 1, typically 1000 or 2048 for OpenType).
    fn units_per_em(&self) -> f64;
    /// Horizontal advance of `ch`, unkerned.
    fn advance(&self, ch: char) -> f64;
    /// Pair kern to add between `left` and `right` (negative pulls closer). 0 when none.
    fn kern(&self, left: char, right: char) -> f64;
    /// Original glyph id for `ch`. Flows through to the output unchanged so the
    /// PDF/preview back end can address the same glyph the shaper chose.
    fn glyph_id(&self, ch: char) -> u32;
    /// Typographic ascender (positive, above the baseline).
    fn ascender(&self) -> f64;
    /// Typographic descender (negative, below the baseline).
    fn descender(&self) -> f64;
    /// Height of `ch`'s own box above the baseline (TeX's per-character
    /// height, the TFM `charht`). [`crate::items::shape_run`] takes a run's
    /// height as the maximum over its glyphs, which is what `\baselineskip`/
    /// `\lineskiplimit` and page breaking see. Default: the font ascender, i.e.
    /// every glyph as tall as the font; an adapter with real per-glyph boxes
    /// (a TFM, or an OpenType source with glyph bounding boxes) should
    /// override this for a tighter, TeX-accurate result.
    fn glyph_height(&self, _ch: char) -> f64 {
        self.ascender()
    }
    /// Depth of `ch`'s own box below the baseline, positive (TFM `chardp`).
    /// Default: the negated font descender; see [`Self::glyph_height`].
    fn glyph_depth(&self, _ch: char) -> f64 {
        -self.descender()
    }
    /// Extra leading the font recommends between lines (0 for AFM fonts).
    fn line_gap(&self) -> f64;
    /// Natural interword space (`\fontdimen2`).
    fn space(&self) -> f64;
    /// Interword stretch (`\fontdimen3`).
    fn space_stretch(&self) -> f64 {
        self.space() * 0.5
    }
    /// Interword shrink (`\fontdimen4`).
    fn space_shrink(&self) -> f64 {
        self.space() / 3.0
    }
    /// Extra space added after sentence-ending punctuation when the space
    /// factor is >= 2000 (`\fontdimen7`).
    fn extra_space(&self) -> f64 {
        0.0
    }
    /// Optional ligature `left`+`right` -> one glyph. Default: no ligatures.
    fn ligature(&self, _left: char, _right: char) -> Option<Ligature> {
        None
    }
}
