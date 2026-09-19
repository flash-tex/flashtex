//! Adobe Core 14 metric faces (no font program): Times-Roman, Times-Bold,
//! Times-Italic, Times-BoldItalic, Helvetica, Courier and Symbol.
//!
//! Widths and `KPX` kerning pairs come from the Adobe AFM files through
//! `tools/gen_tables.py` (see README.md for provenance and digests). The
//! compiler's `metrics.rs` on `de1020c` embeds the same ASCII/Latin-1 widths;
//! this crate keeps its own generated copy so it can stand alone.
//!
//! Glyph ids are synthetic but stable: `GlyphId(i + 1)` is the `i`-th entry
//! of the sorted width table for that face and `GlyphId(0)` is `.notdef`
//! (advance 0). They are consistent across processes because the tables are
//! generated deterministically.

use crate::generated::{self as g, AfmHeader};
use crate::{Error, Face, FontId, FontSource, GlyphId, KerningSource, Style, VerticalMetrics};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Core14 {
    TimesRoman,
    TimesBold,
    TimesItalic,
    TimesBoldItalic,
    Helvetica,
    Courier,
    Symbol,
}

impl Core14 {
    pub const ALL: [Core14; 7] = [
        Core14::TimesRoman,
        Core14::TimesBold,
        Core14::TimesItalic,
        Core14::TimesBoldItalic,
        Core14::Helvetica,
        Core14::Courier,
        Core14::Symbol,
    ];

    /// Resolves an AFM `FontName` such as `"Times-Roman"`.
    pub fn from_name(name: &str) -> Option<Core14> {
        Core14::ALL
            .iter()
            .copied()
            .find(|f| f.header().font_name == name)
    }

    pub fn header(self) -> &'static AfmHeader {
        match self {
            Core14::TimesRoman => &g::TIMES_ROMAN_HEADER,
            Core14::TimesBold => &g::TIMES_BOLD_HEADER,
            Core14::TimesItalic => &g::TIMES_ITALIC_HEADER,
            Core14::TimesBoldItalic => &g::TIMES_BOLD_ITALIC_HEADER,
            Core14::Helvetica => &g::HELVETICA_HEADER,
            Core14::Courier => &g::COURIER_HEADER,
            Core14::Symbol => &g::SYMBOL_HEADER,
        }
    }

    fn widths(self) -> &'static [(u32, u16)] {
        match self {
            Core14::TimesRoman => &g::TIMES_ROMAN_WIDTHS,
            Core14::TimesBold => &g::TIMES_BOLD_WIDTHS,
            Core14::TimesItalic => &g::TIMES_ITALIC_WIDTHS,
            Core14::TimesBoldItalic => &g::TIMES_BOLD_ITALIC_WIDTHS,
            Core14::Helvetica => &g::HELVETICA_WIDTHS,
            Core14::Courier => &g::COURIER_WIDTHS,
            Core14::Symbol => &g::SYMBOL_WIDTHS,
        }
    }

    fn kerns(self) -> &'static [(u16, u16, i16)] {
        match self {
            Core14::TimesRoman => &g::TIMES_ROMAN_KERNS,
            Core14::TimesBold => &g::TIMES_BOLD_KERNS,
            Core14::TimesItalic => &g::TIMES_ITALIC_KERNS,
            Core14::TimesBoldItalic => &g::TIMES_BOLD_ITALIC_KERNS,
            Core14::Helvetica => &g::HELVETICA_KERNS,
            Core14::Courier => &g::COURIER_KERNS,
            Core14::Symbol => &g::SYMBOL_KERNS,
        }
    }

    /// Unicode distinguishes the mathematical DIVIDES relation from the
    /// ASCII vertical bar, while Adobe Symbol exposes both semantics through
    /// its single `verticalbar` glyph at code 0x7C.
    fn afm_char(self, ch: char) -> char {
        match (self, ch) {
            (Core14::Symbol, '\u{2223}') => '|',
            _ => ch,
        }
    }
}

/// A Core 14 face. Cheap to construct; holds only static tables.
#[derive(Debug, Clone)]
pub struct Core14Face {
    which: Core14,
    id: FontId,
}

impl Core14Face {
    pub fn new(which: Core14) -> Core14Face {
        let h = which.header();
        let weight = match h.weight {
            "Bold" => 700,
            "Medium" => 500,
            _ => 400,
        };
        let style = if h.italic_angle != 0.0 {
            Style::Italic
        } else {
            Style::Upright
        };
        let mut seed = Vec::new();
        seed.extend_from_slice(b"flashtex-core14:");
        seed.extend_from_slice(h.font_name.as_bytes());
        for (cp, w) in which.widths() {
            seed.extend_from_slice(&cp.to_be_bytes());
            seed.extend_from_slice(&w.to_be_bytes());
        }
        Core14Face {
            which,
            id: FontId {
                family: h.family_name.to_string(),
                weight,
                style,
                source: FontSource::Core14 {
                    afm_name: h.font_name,
                },
                content_sha256: crate::sha256::digest(&seed),
            },
        }
    }

    pub fn which(&self) -> Core14 {
        self.which
    }

    /// The code point a synthetic glyph id stands for.
    pub fn char_for(&self, gid: GlyphId) -> Option<char> {
        if gid.0 == 0 {
            return None;
        }
        self.which
            .widths()
            .get(usize::from(gid.0) - 1)
            .and_then(|(cp, _)| char::from_u32(*cp))
    }

    /// Unkerned advance of `ch` in 1/1000 em, `None` if the face lacks it.
    pub fn width_units(&self, ch: char) -> Option<u16> {
        let ch = self.which.afm_char(ch);
        let t = self.which.widths();
        t.binary_search_by_key(&(ch as u32), |(cp, _)| *cp)
            .ok()
            .map(|i| t[i].1)
    }
}

impl Face for Core14Face {
    fn id(&self) -> &FontId {
        &self.id
    }

    fn units_per_em(&self) -> u16 {
        1000
    }

    fn num_glyphs(&self) -> u16 {
        self.which.widths().len() as u16 + 1
    }

    fn vertical_metrics(&self) -> VerticalMetrics {
        let h = self.which.header();
        // Symbol declares no Ascender/Descender/CapHeight/XHeight; the AFM
        // bounding box stands in for the first two.
        let declared = h.ascender != 0 || h.descender != 0;
        VerticalMetrics {
            ascender: if declared { h.ascender } else { h.bbox[3] },
            descender: if declared { h.descender } else { h.bbox[1] },
            line_gap: 0,
            cap_height: h.cap_height,
            x_height: h.x_height,
            cap_height_declared: h.cap_height != 0,
            x_height_declared: h.x_height != 0,
        }
    }

    fn bbox(&self) -> [i16; 4] {
        self.which.header().bbox
    }

    fn italic_angle(&self) -> f64 {
        self.which.header().italic_angle
    }

    fn advance(&self, gid: GlyphId) -> Result<u16, Error> {
        if gid.0 == 0 {
            return Ok(0);
        }
        self.which
            .widths()
            .get(usize::from(gid.0) - 1)
            .map(|(_, w)| *w)
            .ok_or(Error::GlyphOutOfRange(gid.0))
    }

    fn glyph_id(&self, ch: char) -> Option<GlyphId> {
        let ch = self.which.afm_char(ch);
        self.which
            .widths()
            .binary_search_by_key(&(ch as u32), |(cp, _)| *cp)
            .ok()
            .map(|i| GlyphId(i as u16 + 1))
    }

    fn kerning(&self, left: GlyphId, right: GlyphId) -> (i16, KerningSource) {
        let (Some(l), Some(r)) = (self.char_for(left), self.char_for(right)) else {
            return (0, KerningSource::None);
        };
        let (l, r) = (l as u32, r as u32);
        if l > 0xFFFF || r > 0xFFFF {
            return (0, KerningSource::None);
        }
        let t = self.which.kerns();
        match t.binary_search_by_key(&(l as u16, r as u16), |(a, b, _)| (*a, *b)) {
            Ok(i) => (t[i].2, KerningSource::Afm),
            Err(_) => (0, KerningSource::None),
        }
    }

    fn kerning_source(&self) -> KerningSource {
        if self.which.kerns().is_empty() {
            KerningSource::None
        } else {
            KerningSource::Afm
        }
    }

    fn ligature(&self, components: &[GlyphId]) -> Option<GlyphId> {
        let chars: Option<String> = components.iter().map(|g| self.char_for(*g)).collect();
        let lig = match chars?.as_str() {
            "fi" => '\u{FB01}',
            "fl" => '\u{FB02}',
            "ff" => '\u{FB00}',
            "ffi" => '\u{FB03}',
            "ffl" => '\u{FB04}',
            _ => return None,
        };
        self.glyph_id(lig)
    }

    fn mark_attachment(&self, _base: GlyphId, _mark: GlyphId) -> Option<(i16, i16)> {
        None
    }

    fn ligature_passes(&self) -> usize {
        1
    }

    fn longest_ligature(&self, _pass: usize, glyphs: &[GlyphId]) -> Option<(GlyphId, usize)> {
        for len in (2..=3.min(glyphs.len())).rev() {
            if let Some(g) = self.ligature(&glyphs[..len]) {
                return Some((g, len));
            }
        }
        None
    }

    fn unsupported(&self) -> &[crate::Unsupported] {
        &[]
    }

    fn postscript_name(&self) -> &str {
        self.which.header().font_name
    }

    fn is_fixed_pitch(&self) -> bool {
        self.which.header().is_fixed_pitch
    }
}

/// AFM glyph names that had no Adobe Glyph List mapping, per face (empty
/// for the shipped tables; kept so a regenerated table cannot hide gaps).
pub fn unmapped_glyph_names() -> &'static [(&'static str, &'static [&'static str])] {
    g::UNMAPPED_GLYPH_NAMES
}

/// Unicode version of the composition/mark tables in this build.
pub fn unicode_version() -> &'static str {
    g::UNICODE_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_roman_matches_compiler_metrics_rs_on_de1020c() {
        let f = Core14Face::new(Core14::TimesRoman);
        // Same values the compiler embeds (metrics.rs, de1020c).
        assert_eq!(f.width_units('M'), Some(889));
        assert_eq!(f.width_units(' '), Some(250));
        assert_eq!(f.width_units('\u{A0}'), Some(250));
        assert_eq!(f.width_units('\u{FF}'), Some(500));
        assert_eq!(f.width_units('\u{FB01}'), Some(556));
    }

    #[test]
    fn synthetic_gids_round_trip() {
        for which in Core14::ALL {
            let f = Core14Face::new(which);
            for gid in 1..f.num_glyphs() {
                let c = f.char_for(GlyphId(gid)).unwrap();
                assert_eq!(f.glyph_id(c), Some(GlyphId(gid)));
            }
            assert_eq!(f.advance(GlyphId(0)).unwrap(), 0);
            assert!(f.advance(GlyphId(f.num_glyphs())).is_err());
        }
    }

    #[test]
    fn afm_kerning_pairs() {
        let f = Core14Face::new(Core14::TimesRoman);
        let a = f.glyph_id('A').unwrap();
        let v = f.glyph_id('V').unwrap();
        assert_eq!(f.kerning(a, v), (-135, KerningSource::Afm));
        assert_eq!(f.kerning(v, v), (0, KerningSource::None));
    }

    #[test]
    fn identity_is_stable_and_distinct() {
        let a = Core14Face::new(Core14::TimesRoman);
        let b = Core14Face::new(Core14::TimesRoman);
        let c = Core14Face::new(Core14::TimesBold);
        assert_eq!(a.id(), b.id());
        assert_ne!(a.id().content_sha256, c.id().content_sha256);
        assert_eq!(a.id().weight, 400);
        assert_eq!(c.id().weight, 700);
        assert_eq!(
            Core14Face::new(Core14::TimesItalic).id().style,
            Style::Italic
        );
        assert_eq!(Core14::from_name("Symbol"), Some(Core14::Symbol));
    }
}
