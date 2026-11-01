//! `flashtex_paragraph_layout` adapter.
//!
//! Two routes, both reading the same [`Face`]:
//!
//! 1. [`FaceMetrics`] implements `FontMetricsSource` so paragraph-layout's
//!    own char-based `shape_run` can measure with the engine's advances,
//!    kerns and glyph ids. Its `ligature` contract is char → char, so only
//!    ligatures that have a Unicode code point in the cmap (the f-ligatures
//!    U+FB00..U+FB04) can be reported; glyph-only ligatures and multi-step
//!    ones (Latin Modern's `ffi` built through `ff`) cannot. Missing glyphs
//!    come back as `.notdef` (gid 0, notdef advance) because the trait has no
//!    error channel — use route 2 when that matters.
//! 2. [`glyph_run`] shapes with [`crate::shape::shape`] (full GSUB/GPOS,
//!    clusters, explicit `missing`, fail-closed scripts) and hands the result
//!    to `GlyphRun::from_shaped`, so the paragraph box carries exactly the
//!    engine's glyph ids, kerned advances and source clusters.

use std::ops::Range;

use flashtex_paragraph_layout::items::{GlyphRun, ShapedGlyph};
use flashtex_paragraph_layout::metrics::{FontId, FontMetricsSource, Ligature};

use crate::shape::{ShapeOptions, Shaped, shape};
use crate::{Error, Face, GlyphId};

/// A `Face` viewed as a paragraph-layout metrics source.
pub struct FaceMetrics<'a> {
    pub face: &'a dyn Face,
}

impl<'a> FaceMetrics<'a> {
    pub fn new(face: &'a dyn Face) -> Self {
        FaceMetrics { face }
    }

    fn gid(&self, ch: char) -> GlyphId {
        self.face.glyph_id(ch).unwrap_or(GlyphId::NOTDEF)
    }
}

/// The engine's content-addressed identity as paragraph-layout's 32-byte token.
pub fn font_id(face: &dyn Face) -> FontId {
    FontId(face.id().content_sha256)
}

impl FontMetricsSource for FaceMetrics<'_> {
    fn font_id(&self) -> FontId {
        font_id(self.face)
    }

    fn units_per_em(&self) -> f64 {
        f64::from(self.face.units_per_em())
    }

    fn advance(&self, ch: char) -> f64 {
        f64::from(self.face.advance(self.gid(ch)).unwrap_or(0))
    }

    fn kern(&self, left: char, right: char) -> f64 {
        let (l, r) = (self.face.glyph_id(left), self.face.glyph_id(right));
        match (l, r) {
            (Some(l), Some(r)) => f64::from(self.face.kerning(l, r).0),
            _ => 0.0,
        }
    }

    fn glyph_id(&self, ch: char) -> u32 {
        u32::from(self.gid(ch).0)
    }

    fn ascender(&self) -> f64 {
        f64::from(self.face.vertical_metrics().ascender)
    }

    fn descender(&self) -> f64 {
        f64::from(self.face.vertical_metrics().descender)
    }

    fn line_gap(&self) -> f64 {
        f64::from(self.face.vertical_metrics().line_gap)
    }

    fn space(&self) -> f64 {
        self.advance(' ')
    }

    fn ligature(&self, left: char, right: char) -> Option<Ligature> {
        if self.face.is_fixed_pitch() {
            return None;
        }
        let (l, r) = (self.face.glyph_id(left)?, self.face.glyph_id(right)?);
        let lig = self.face.ligature(&[l, r])?;
        // Report only ligatures addressable by a code point in the cmap.
        for c in ['\u{FB00}', '\u{FB01}', '\u{FB02}', '\u{FB03}', '\u{FB04}'] {
            if self.face.glyph_id(c) == Some(lig) {
                return Some(Ligature { result: c });
            }
        }
        // Core 14 faces map f-ligature characters directly.
        match (left, right) {
            ('f', 'i') if self.face.glyph_id('\u{FB01}').is_some() => {
                Some(Ligature { result: '\u{FB01}' })
            }
            ('f', 'l') if self.face.glyph_id('\u{FB02}').is_some() => {
                Some(Ligature { result: '\u{FB02}' })
            }
            _ => None,
        }
    }
}

/// Converts a shaping result into paragraph-layout's shaped-glyph input.
/// Byte ranges are offset by `source_start`.
pub fn shaped_glyphs(shaped: &Shaped, source_start: usize) -> Vec<ShapedGlyph> {
    let mut out = Vec::with_capacity(shaped.clusters.len());
    for cluster in &shaped.clusters {
        for g in &cluster.glyphs {
            out.push(ShapedGlyph {
                gid: u32::from(g.gid.0),
                advance_units: i64::from(g.advance),
                cluster: cluster.source_range.start + source_start
                    ..cluster.source_range.end + source_start,
            });
        }
    }
    out
}

/// Shapes `text` with the engine and builds a `GlyphRun` from it: identical
/// glyph ids, kerned advances and clusters to what the PDF and preview
/// receive. Returns the engine's errors (unsupported script) unchanged and
/// the shaping result alongside so callers can inspect `missing`.
pub fn glyph_run(
    face: &dyn Face,
    size: f64,
    text: &str,
    source_start: usize,
    opts: &ShapeOptions,
) -> Result<(GlyphRun, Shaped), Error> {
    let shaped = shape(face, text, opts)?;
    let vm = face.vertical_metrics();
    let glyphs = shaped_glyphs(&shaped, source_start);
    let source: Range<usize> = source_start..source_start + text.len();
    let run = GlyphRun::from_shaped(
        font_id(face),
        size,
        f64::from(face.units_per_em()),
        f64::from(vm.ascender),
        f64::from(vm.descender),
        &glyphs,
        source,
    );
    Ok((run, shaped))
}
