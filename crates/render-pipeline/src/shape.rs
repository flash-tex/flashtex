//! Shaping with a per-face cache. Text faces with a TFM attached (Latin
//! Modern `ec-lm*`) are shaped by the TFM: characters become T1 codes, the
//! ligature/kern program runs, and widths, kerns, heights and depths are
//! the fixwords pdfTeX lays out with (units: 2^20 per em); the glyph ids
//! come from the same face's `cmap`. Text the encoding cannot express, and
//! faces without a TFM, go through font-engine (`shape::shape`: cmap
//! mapping, mark composition, GSUB/AFM ligatures, GPOS/AFM kerning; units:
//! the face's units per em). Glyph extents come from the face outlines
//! either way. Everything cached is size-independent.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

use flashtex_font_engine::{shape as fe_shape, ShapeOptions};

use crate::fonts::LoadedFace;
use crate::ids::{Encoding, EncodingCode, GlyphId};
use crate::tfm::{Tfm, FIX};

#[derive(Debug, Clone, PartialEq)]
pub struct SGlyph {
    pub gid: GlyphId,
    /// Advance in font units, kerning included.
    pub advance: i32,
    /// TFM italic correction (fixwords) of this glyph; 0 without a TFM.
    pub italic: i32,
    pub x_offset: i32,
    pub y_offset: i32,
    /// Extents in font units relative to the glyph origin (0 when empty).
    pub y_max: i32,
    pub y_min: i32,
    pub x_max: i32,
    pub empty: bool,
    /// TFM shaping only: the character code (T1 slot) this glyph sets and
    /// the font kern (fixwords) included in `advance` after it; microtype
    /// protrusion/expansion read them. `None`/0 otherwise.
    pub tfm_code: Option<u8>,
    pub tfm_kern: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SCluster {
    pub glyphs: Vec<SGlyph>,
    /// Byte range into the shaped text.
    pub text_range: Range<usize>,
    pub text: String,
}

impl SCluster {
    pub fn advance_units(&self) -> i64 {
        self.glyphs.iter().map(|g| i64::from(g.advance)).sum()
    }
}

/// A shaped string in one face. Advances, width, height and depth are in
/// `units_per_em` units (the face's for OpenType shaping, 2^20 for TFM
/// shaping); glyph extents (`y_max`, `y_min`, `x_max`) and offsets are
/// always in the face's font units.
#[derive(Clone)]
pub struct Shaped {
    pub face: Rc<LoadedFace>,
    pub text: String,
    pub clusters: Vec<SCluster>,
    /// Units per em of the advances/width/height/depth.
    pub units_per_em: i64,
    /// True when the TFM (TeX's own metrics) produced these advances.
    pub tfm_metrics: bool,
    pub width_units: i64,
    /// Max glyph extent above the baseline, font units.
    pub height_units: i32,
    /// Max glyph extent below the baseline, font units (positive).
    pub depth_units: i32,
    /// Characters with no glyph in this face, with byte offsets into `text`.
    pub missing: Vec<(char, usize)>,
    /// Set when the shaper refused the text (unsupported script); the
    /// clusters are then empty and nothing is typeset for it.
    pub refused: Option<String>,
    /// The TFM interpreter's error for this text, when it had one: the run
    /// was then shaped by the font program instead, and the caller reports it.
    pub tfm_error: Option<String>,
}

impl Shaped {
    fn pt(&self, units: i64, size_pt: f64) -> f64 {
        units as f64 * size_pt / self.units_per_em as f64
    }
    pub fn width_pt(&self, size_pt: f64) -> f64 {
        self.pt(self.width_units, size_pt)
    }
    pub fn height_pt(&self, size_pt: f64) -> f64 {
        self.pt(i64::from(self.height_units), size_pt)
    }
    pub fn depth_pt(&self, size_pt: f64) -> f64 {
        self.pt(i64::from(self.depth_units), size_pt)
    }
}

/// Most shaped words kept; beyond this the cache is cleared (a document
/// edited for hours never grows it without bound).
pub const SHAPER_CACHE_LIMIT: usize = 200_000;

#[derive(Default)]
pub struct Shaper {
    cache: RefCell<HashMap<(Rc<str>, String, bool), Rc<Shaped>>>,
}

impl Shaper {
    pub fn new() -> Shaper {
        Shaper::default()
    }

    /// Shapes `text` in `face` with kerning and ligatures on.
    pub fn shape(&self, face: &Rc<LoadedFace>, text: &str) -> Rc<Shaped> {
        self.shape_with(face, text, false)
    }

    /// Shapes `text` with ligatures and kerns **off**: verbatim's regime
    /// (`crate::tfm::literal_run`). A separate cache entry, because the same
    /// face and text shape differently under it.
    pub fn shape_literal(&self, face: &Rc<LoadedFace>, text: &str) -> Rc<Shaped> {
        self.shape_with(face, text, true)
    }

    fn shape_with(&self, face: &Rc<LoadedFace>, text: &str, literal: bool) -> Rc<Shaped> {
        // Keyed by the face's metrics identity, not its wire `font_id`: one
        // OpenType program is laid out with different TFMs (`ec-lmr10` for
        // `lmodern`, `ecrm1095`/`ecrm1000` for T1 `cmr`).
        let key = (face.shape_key.clone(), text.to_string(), literal);
        if let Some(hit) = self.cache.borrow().get(&key) {
            return hit.clone();
        }
        let shaped = Rc::new(shape_uncached(face, text, literal));
        let mut cache = self.cache.borrow_mut();
        if cache.len() >= SHAPER_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, shaped.clone());
        shaped
    }

    pub fn len(&self) -> usize {
        self.cache.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.borrow().is_empty()
    }
}

fn shape_uncached(face: &Rc<LoadedFace>, text: &str, literal: bool) -> Shaped {
    if let Some(tfm) = &face.tfm {
        match shape_tfm(face, tfm, text, literal) {
            Ok(Some(s)) => return s,
            Ok(None) => {}
            Err(e) => {
                let mut s = shape_otf(face, text, literal);
                s.tfm_error = Some(e.to_string());
                return s;
            }
        }
    }
    shape_otf(face, text, literal)
}

/// TFM shaping; `Ok(None)` when a character has no T1 slot (the caller then
/// shapes through the font program and its own metrics); `Err` propagates
/// the shared interpreter's errors (malformed program, run budget).
fn shape_tfm(face: &Rc<LoadedFace>, tfm: &Tfm, text: &str, literal: bool) -> Result<Option<Shaped>, crate::tfm::TfmError> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut codes = Vec::with_capacity(chars.len());
    for (_, c) in &chars {
        let Some(code) = EncodingCode::for_char(*c, Encoding::T1) else {
            return Ok(None);
        };
        codes.push(code.0);
    }
    let end_of = |i: usize| -> usize { chars.get(i).map_or(text.len(), |(b, _)| *b) };
    let mut clusters = Vec::new();
    let mut missing = Vec::new();
    let mut y_max = 0i32;
    let mut y_min = 0i32;
    let mut height = 0i32;
    let mut depth = 0i32;
    // `\@noligs`: verbatim runs no ligature/kern program at all.
    let run = if literal { crate::tfm::literal_run(&codes) } else { tfm.ligkern(&codes)? };
    if run.leading_kern != 0 {
        // A left-boundary kern: an explicit advance before the first
        // character, attributed to an empty range at the text start.
        clusters.push(SCluster {
            glyphs: vec![SGlyph {
                gid: GlyphId(0),
                advance: run.leading_kern,
                italic: 0,
                x_offset: 0,
                y_offset: 0,
                y_max: 0,
                y_min: 0,
                x_max: 0,
                empty: true,
                tfm_code: None,
                tfm_kern: 0,
            }],
            text_range: 0..0,
            text: String::new(),
        });
    }
    for g in run.glyphs {
        let Some(m) = tfm.metrics(g.code) else {
            return Err(crate::tfm::TfmError(format!("code {:#04x} has no metrics", g.code)));
        };
        let range = end_of(g.input.0)..end_of(g.input.1);
        let ctext = text[range.clone()].to_string();
        let Some(ch) = EncodingCode(g.code).to_char(Encoding::T1) else {
            return Err(crate::tfm::TfmError(format!("ligature program produced undeclared T1 slot {:#04x}", g.code)));
        };
        let gid = match face.face().glyph_id(ch) {
            Some(gid) => gid,
            None => {
                missing.push((ch, range.start));
                GlyphId(0)
            }
        };
        let b = face.bounds(gid, Some(ch));
        if !b.empty && gid.0 != 0 {
            y_max = y_max.max(b.y_max);
            y_min = y_min.min(b.y_min);
        }
        height = height.max(m.height);
        depth = depth.max(m.depth);
        clusters.push(SCluster {
            glyphs: vec![SGlyph {
                gid,
                advance: m.width + g.kern_after,
                italic: m.italic,
                x_offset: 0,
                y_offset: 0,
                y_max: if b.empty { 0 } else { b.y_max },
                y_min: if b.empty { 0 } else { b.y_min },
                x_max: if b.empty { 0 } else { b.x_max },
                empty: b.empty || gid.0 == 0,
                tfm_code: Some(g.code),
                tfm_kern: g.kern_after,
            }],
            text_range: range,
            text: ctext,
        });
    }
    let _ = (y_max, y_min);
    let width_units: i64 = clusters.iter().map(SCluster::advance_units).sum();
    Ok(Some(Shaped {
        face: face.clone(),
        text: text.to_string(),
        clusters,
        units_per_em: FIX,
        tfm_metrics: true,
        width_units,
        height_units: height,
        depth_units: depth,
        missing,
        refused: None,
        tfm_error: None,
    }))
}

fn shape_otf(face: &Rc<LoadedFace>, text: &str, literal: bool) -> Shaped {
    let f = face.face();
    // Verbatim: no GSUB `liga`, no f-ligature cmap fallback, no pair
    // kerning. Mark composition stays on — it is not a ligature, and a
    // verbatim run reaching this path at all means the text left T1.
    let opts = ShapeOptions {
        ligatures: !literal,
        kerning: !literal,
        cmap_ligature_fallback: !literal,
        ..ShapeOptions::default()
    };
    let (clusters, missing, refused) = match fe_shape::shape(f, text, &opts) {
        Ok(s) => {
            let clusters = s
                .clusters
                .iter()
                .map(|c| SCluster {
                    glyphs: c
                        .glyphs
                        .iter()
                        .map(|g| {
                            let ch = c.text.chars().next();
                            let b = face.bounds(g.gid, ch);
                            SGlyph {
                                gid: g.gid,
                                advance: g.advance,
                                italic: 0,
                                x_offset: g.x_offset,
                                y_offset: g.y_offset,
                                y_max: if b.empty { 0 } else { b.y_max },
                                y_min: if b.empty { 0 } else { b.y_min },
                                x_max: if b.empty { g.advance } else { b.x_max },
                                empty: b.empty,
                                tfm_code: None,
                                tfm_kern: 0,
                            }
                        })
                        .collect(),
                    text_range: c.source_range.clone(),
                    text: c.text.clone(),
                })
                .collect();
            (clusters, s.missing.iter().map(|m| (m.ch, m.byte_offset)).collect(), None)
        }
        Err(e) => (Vec::new(), Vec::new(), Some(e.to_string())),
    };
    let clusters: Vec<SCluster> = clusters;
    let width_units: i64 = clusters.iter().map(SCluster::advance_units).sum();
    let mut y_max = 0i32;
    let mut y_min = 0i32;
    for c in &clusters {
        for g in &c.glyphs {
            if g.empty {
                continue;
            }
            y_max = y_max.max(g.y_max + g.y_offset);
            y_min = y_min.min(g.y_min + g.y_offset);
        }
    }
    Shaped {
        face: face.clone(),
        text: text.to_string(),
        clusters,
        units_per_em: i64::from(face.units_per_em),
        tfm_metrics: false,
        width_units,
        height_units: y_max,
        depth_units: -y_min,
        missing,
        refused,
        tfm_error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::{Family, FontSet, Role, DEFAULT_FONT_DIRS};

    #[test]
    fn latin_modern_shaping_applies_ligatures_and_kerning() {
        if !DEFAULT_FONT_DIRS.iter().any(|d| std::path::Path::new(d).join("lmroman10-regular.otf").is_file()) {
            eprintln!("skipping: Latin Modern not installed");
            return;
        }
        let fonts = FontSet::with_default_dirs(&[]);
        let face = fonts.resolve(Family::LatinModern, Role::Text { bold: false, italic: false }, 10.0).face;
        let shaper = Shaper::new();
        // The OpenType path (GSUB/GPOS), bypassing the TFM.
        let s = Rc::new(shape_otf(&face, "office", false));
        // "ffi" is one cluster covering bytes 1..4.
        let lig = s.clusters.iter().find(|c| c.text == "ffi").expect("ffi ligature cluster");
        assert_eq!(lig.text_range, 1..4);
        assert_eq!(lig.glyphs.len(), 1);
        let av = Rc::new(shape_otf(&face, "AV", false));
        let plain: i64 = av.clusters.iter().flat_map(|c| c.glyphs.iter()).map(|g| i64::from(g.advance)).sum();
        // font-engine README: "AV" shaped at 10pt is 13.89pt -> 1389 units (kerned).
        assert_eq!(plain, 1389);
        assert!(s.missing.is_empty());
        // Round letters overshoot the baseline by 11 units in Latin Modern.
        assert!(s.height_units > 600 && s.depth_units <= 15, "{} {}", s.height_units, s.depth_units);
    }
}
