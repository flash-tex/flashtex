//! `MathFontMetrics` for math-layout, driven by Latin Modern Math through
//! font-engine: Appendix G parameters from the `MATH` constants
//! (`MathParams::from_opentype`, the LuaTeX correspondence), glyph metrics
//! from the face advances and the CFF charstring bounds, italic corrections
//! and top-accent attachment from the `MATH` table, and vertical glyph
//! variants (display-size operators, larger delimiters, radical signs) read
//! from `MathVariants` here because font-engine does not expose them yet
//! (requested API, see docs/proposals/rendering-abi.md).
//!
//! `\usepackage{times}` changes only the text fonts in LaTeX; math stays in
//! Computer Modern, so this provider is used for both families.
//!
//! Blackboard bold is the exception to "one face": pdfLaTeX's `\mathbb`
//! comes from AMS `msbm10`, a serifed double-struck design, while Latin
//! Modern Math's double-struck block is the sans-like open-face design. New
//! Computer Modern Math reproduces the msbm design (and its widths track
//! msbm's), so when `NewCMMath-Regular.otf` is in a font directory every
//! double-struck code point is drawn from it as a secondary face
//! ([`BB_FONT`]); otherwise Latin Modern Math draws it and the typesetter
//! reports the profile difference once.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use flashtex_math_layout::{FontId as MathFontId, Glyph, MathFontMetrics, MathParams, OpenTypeMathConstants, SizeClass};

use crate::cff::u16_at;
use crate::fonts::LoadedFace;
use crate::ids::GlyphId;

/// The three sizes of one math context, points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MathSizes {
    pub text: f64,
    pub script: f64,
    pub script_script: f64,
}

impl MathSizes {
    pub fn at(&self, size: SizeClass) -> f64 {
        match size {
            SizeClass::Text => self.text,
            SizeClass::Script => self.script,
            SizeClass::ScriptScript => self.script_script,
        }
    }
}

/// One vertical variant from `MathVariants`: glyph and its declared advance
/// height (font units).
#[derive(Debug, Clone, Copy)]
struct VertVariant {
    gid: u16,
    #[allow(dead_code)]
    advance: u16,
}

/// One `GlyphPartRecord` of a `MathVariants` glyph assembly, font units.
///
/// `full_advance` is the part's own extent along the assembly axis (its ink
/// height for a vertical part: every Latin Modern Math vertical part draws
/// from its origin up to exactly `fullAdvance`). `start_connector` and
/// `end_connector` are how much of the part may be overlapped by the
/// neighbour before it and after it — the joint between two parts may
/// overlap by at most `min(end of the lower, start of the upper)` and at
/// least [`MathFonts::min_connector_overlap`]. `extender` is `partFlags`
/// bit 0 (`fExtender`): the part may repeat to reach the wanted size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssemblyPart {
    pub gid: u16,
    pub start_connector: u16,
    pub end_connector: u16,
    pub full_advance: u16,
    pub extender: bool,
}

/// One `MathGlyphConstruction`: the variant records and the glyph assembly
/// (empty when the construction has none).
type Construction = (Vec<VertVariant>, Vec<AssemblyPart>);

/// How often one extender may repeat in an assembly. A `\left(` around a
/// page-tall box needs about 10 (Latin Modern Math's paren extender is
/// 0.498 em); the cap only keeps a malformed font from looping.
const MAX_ASSEMBLY_REPEATS: usize = 256;

pub struct MathFonts {
    face: Rc<LoadedFace>,
    /// The double-struck face (New Computer Modern Math), when found.
    bb: Option<Rc<LoadedFace>>,
    /// Why `bb` is absent (the font set's reason), for the profile note.
    bb_status: Option<String>,
    /// Set once a double-struck glyph was served from `face` because `bb`
    /// is absent; drained by the typesetter for its one profile note.
    bb_fallback: RefCell<bool>,
    sizes: MathSizes,
    constants: OpenTypeMathConstants,
    x_height_units: i16,
    /// Vertical constructions from `MathVariants`: each base glyph's
    /// variants (advance heights, smallest first) and its assembly's parts,
    /// bottom to top (larger delimiters and radicals, then the pieces a
    /// delimiter taller than every variant is assembled from).
    vert: BTreeMap<u16, Construction>,
    /// Horizontal constructions from `MathVariants`: each base glyph's
    /// variants (advance widths) and its assembly's parts, left to right
    /// (wide accents, `\overbrace` pieces).
    horiz: BTreeMap<u16, Construction>,
    /// `MathVariants.minConnectorOverlap`, font units: the least a part may
    /// overlap its neighbour in an assembly (20 in Latin Modern Math).
    min_connector_overlap: u16,
    /// Characters with no glyph in the math font, recorded for diagnostics.
    missing: RefCell<Vec<char>>,
}

/// `FontId(0)` is the math face (Latin Modern Math).
const MATH_FONT: MathFontId = MathFontId(0);
/// `FontId(1)` is the double-struck face (New Computer Modern Math); glyphs
/// carry it only when that face is loaded.
pub const BB_FONT: MathFontId = MathFontId(1);
/// File name of the double-struck face, looked up in the font directories.
pub const BB_FONT_FILE: &str = "NewCMMath-Regular.otf";

/// The code points `\mathbb` produces (the letter-like symbols and the
/// Mathematical Alphanumeric Symbols double-struck block), drawn from
/// [`BB_FONT`] when it is available.
pub fn is_double_struck(ch: char) -> bool {
    matches!(
        ch,
        '\u{2102}' | '\u{210D}' | '\u{2115}' | '\u{2119}' | '\u{211A}' | '\u{211D}' | '\u{2124}' | '\u{1D538}'..='\u{1D56B}'
    )
}

/// The code points `\mathcal` produces (compiler pin `dbf6ec78`,
/// `newcm_math::script`: the Mathematical Alphanumeric script capitals
/// U+1D49C–U+1D4B5 and the eight capitals Unicode encodes in Letterlike
/// Symbols), which the compiler binds to New Computer Modern Math
/// (`newcm_math::FONT_ID`); the same secondary face draws them here. The
/// set is the compiler's own advance table, so it cannot drift from it.
pub fn is_script_capital(ch: char) -> bool {
    flashtex_compiler::newcm_math::advance(ch).is_some()
}

/// Sentinel the pipeline substitutes for `\varnothing`'s compiler symbol
/// (U+2205, identical to `\emptyset`'s) when `MathAtom.width_em` is `Some`
/// (compiler pin `c583d6d4`: the field is now `pub`, msbm10's 0.777781em).
/// A private-use code point, never a compiler output, so the metrics
/// providers can tell the two U+2205 occurrences apart without re-scanning
/// the source: [`is_secondary_face`] paints it from [`BB_FONT`] like
/// `\mathbb`/`\mathcal` (New Computer Modern Math's advances track msbm's,
/// per the module doc comment), and [`math_char`] maps it straight back to
/// U+2205 everywhere else (glyph lookup, text extraction). Only
/// `typeset::symbol_atoms` introduces it; nothing else compares against it.
pub const VARNOTHING_SENTINEL: char = '\u{F8FF}';

/// Whether `ch` is drawn from the secondary face ([`BB_FONT`]) when it is
/// loaded: `\mathbb` and `\mathcal` letters, and [`VARNOTHING_SENTINEL`].
pub fn is_secondary_face(ch: char) -> bool {
    is_double_struck(ch) || is_script_capital(ch) || ch == VARNOTHING_SENTINEL || ams_of(ch).is_some()
}

/// First code point of the plane-15 private-use range [`ams_sentinel`] maps
/// AMS symbol font slots into (never compiler text).
const AMS_SENTINEL_BASE: u32 = 0xF_0000;

/// The sentinel `typeset` substitutes for an amssymb/amsfonts symbol atom
/// (compiler `MathAtom.ams_symbol`). It carries the msam/msbm font and slot to
/// the metrics providers, which box it from the AMS TFMs (math-layout `ams`)
/// and paint the symbol's `text` from the secondary face (New Computer Modern
/// Math, whose symbol designs track the AMS fonts) when it carries it, else
/// from Latin Modern Math; [`math_char`](MathFonts::math_char) and text
/// extraction map it back to that text.
pub fn ams_sentinel(ams: &flashtex_compiler::amssymb::AmsSymbol) -> char {
    use flashtex_compiler::amssymb::SymbolFont;
    let font = match ams.font {
        SymbolFont::Msam => 0,
        SymbolFont::Msbm => 1,
    };
    char::from_u32(AMS_SENTINEL_BASE + font * 256 + u32::from(ams.slot)).expect("plane-15 private use")
}

/// The amssymb symbol an [`ams_sentinel`] stands for.
pub fn ams_of(ch: char) -> Option<&'static flashtex_compiler::amssymb::AmsSymbol> {
    use flashtex_compiler::amssymb::{by_slot, SymbolFont};
    let v = (ch as u32).checked_sub(AMS_SENTINEL_BASE)?;
    let font = match v >> 8 {
        0 => SymbolFont::Msam,
        1 => SymbolFont::Msbm,
        _ => return None,
    };
    by_slot(font, (v & 0xFF) as u8)
}

/// math-layout's AMS font for a compiler symbol font.
pub fn ams_font(font: flashtex_compiler::amssymb::SymbolFont) -> flashtex_math_layout::ams::AmsFont {
    match font {
        flashtex_compiler::amssymb::SymbolFont::Msam => flashtex_math_layout::ams::AmsFont::Msam,
        flashtex_compiler::amssymb::SymbolFont::Msbm => flashtex_math_layout::ams::AmsFont::Msbm,
    }
}

impl MathFonts {
    /// `face` must carry a `MATH` table (Latin Modern Math); `None` otherwise.
    pub fn new(face: Rc<LoadedFace>, sizes: MathSizes) -> Option<MathFonts> {
        let table = face.math()?;
        let c = &table.constants;
        let constants = OpenTypeMathConstants {
            units_per_em: face.units_per_em as u16,
            axis_height: c.axis_height,
            fraction_numerator_display_style_shift_up: c.fraction_numerator_display_style_shift_up,
            fraction_numerator_shift_up: c.fraction_numerator_shift_up,
            stack_top_shift_up: c.stack_top_shift_up,
            fraction_denominator_display_style_shift_down: c.fraction_denominator_display_style_shift_down,
            fraction_denominator_shift_down: c.fraction_denominator_shift_down,
            superscript_shift_up: c.superscript_shift_up,
            superscript_shift_up_cramped: c.superscript_shift_up_cramped,
            subscript_shift_down: c.subscript_shift_down,
            superscript_baseline_drop_max: c.superscript_baseline_drop_max,
            subscript_baseline_drop_min: c.subscript_baseline_drop_min,
            fraction_rule_thickness: c.fraction_rule_thickness,
            upper_limit_gap_min: c.upper_limit_gap_min,
            lower_limit_gap_min: c.lower_limit_gap_min,
            upper_limit_baseline_rise_min: c.upper_limit_baseline_rise_min,
            lower_limit_baseline_drop_min: c.lower_limit_baseline_drop_min,
            delimited_sub_formula_min_height: c.delimited_sub_formula_min_height,
        };
        let vm = face.face().vertical_metrics();
        let x_height_units = if vm.x_height_declared { vm.x_height } else { 431 };
        let (min_connector_overlap, vert, horiz) = face
            .otf()
            .and_then(|f| f.table(b"MATH"))
            .and_then(|t| parse_variants(t).ok())
            .unwrap_or_default();
        Some(MathFonts {
            face,
            bb: None,
            bb_status: None,
            bb_fallback: RefCell::new(false),
            sizes,
            constants,
            x_height_units,
            vert,
            horiz,
            min_connector_overlap,
            missing: RefCell::new(Vec::new()),
        })
    }

    /// Attaches the double-struck face (`Ok`) or records why it is absent
    /// (`Err`, the font set's reason). A face without a `cmap` entry for
    /// U+2124 is refused as not double-struck.
    pub fn with_double_struck(mut self, bb: Result<Rc<LoadedFace>, String>) -> MathFonts {
        match bb {
            Ok(f) if f.face().glyph_id('\u{2124}').is_some() => {
                self.bb = Some(f);
                self.bb_status = None;
            }
            Ok(f) => self.bb_status = Some(format!("{} has no double-struck glyphs", f.name)),
            Err(reason) => self.bb_status = Some(reason),
        }
        self
    }

    pub fn face(&self) -> &Rc<LoadedFace> {
        &self.face
    }

    /// The face that draws [`BB_FONT`] glyphs, when loaded.
    pub fn bb_face(&self) -> Option<&Rc<LoadedFace>> {
        self.bb.as_ref()
    }

    /// Why no double-struck face is attached, when none is.
    pub fn bb_status(&self) -> Option<&str> {
        self.bb_status.as_deref()
    }

    /// Whether a double-struck glyph was drawn from the primary face since
    /// the last call (the secondary face being absent).
    pub fn take_bb_fallback(&self) -> bool {
        std::mem::take(&mut *self.bb_fallback.borrow_mut())
    }

    pub fn take_missing(&self) -> Vec<char> {
        std::mem::take(&mut *self.missing.borrow_mut())
    }

    /// The `k`-th (1-based) vertical variant of `base` from `MathVariants`,
    /// in the table's increasing-size order. Latin Modern Math lists the
    /// base glyph itself as the first entry of every `MathGlyphConstruction`
    /// (`∑`: `[3060, 3074]`), so entries equal to `base` are skipped: `k = 1`
    /// is the first glyph that is actually larger (cmex's `next_larger`).
    pub fn variant_gid(&self, base: u16, k: usize) -> Option<u16> {
        if k == 0 {
            return Some(base);
        }
        self.vert.get(&base)?.0.iter().filter(|v| v.gid != base).nth(k - 1).map(|v| v.gid)
    }

    /// The vertical variant of `ch` (drawn at `size_pt`) whose ink
    /// height + depth is nearest `wanted`, the base glyph excluded.
    ///
    /// The cmex size chains are indexed by TeX's `next_larger` steps
    /// (`\big` 12 pt, `\Big` 18, `\bigg` 24, `\Bigg` 30 for delimiters),
    /// but Latin Modern Math's `MathVariants` for the delimiters carry
    /// seven sizes (≈ 11, 12, 14.4, 18, 21, 24, 30 pt at 10 pt), so the
    /// same-index variant is the wrong size from the second step on; the
    /// radical and the operators list exactly the cmex sizes and match
    /// either way. `None` when the face lists no larger variant.
    pub fn variant_nearest(&self, ch: char, size_pt: f64, wanted: f64) -> Option<u16> {
        let base = self.base_gid(ch)?;
        let drawn = Self::math_char(ch);
        self.vert
            .get(&base)?
            .0
            .iter()
            .filter(|v| v.gid != base)
            .map(|v| (v.gid, (self.glyph_for(v.gid, drawn, size_pt).total_height() - wanted).abs()))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(gid, _)| gid)
    }

    /// The horizontal variant of `ch` (base glyph included) whose advance at
    /// `size_pt` is nearest `wanted` pt: what paints a cmex/msbm wide accent
    /// laid out at its TFM width.
    pub fn hvariant_nearest(&self, ch: char, size_pt: f64, wanted: f64) -> Option<u16> {
        let base = self.face.face().glyph_id(ch)?.0;
        let (variants, _) = self.horiz.get(&base)?;
        let upem = f64::from(self.face.units_per_em);
        variants
            .iter()
            .map(|v| (v.gid, (f64::from(v.advance) * size_pt / upem - wanted).abs()))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(gid, _)| gid)
    }

    /// The assembly parts of `ch`'s horizontal construction, left to right
    /// (Latin Modern Math's U+23DE: left end, extender, middle, extender,
    /// right end); empty when it has none.
    pub fn hassembly_parts(&self, ch: char) -> Vec<u16> {
        self.face
            .face()
            .glyph_id(ch)
            .and_then(|g| self.horiz.get(&g.0))
            .map(|(_, parts)| parts.iter().map(|p| p.gid).collect())
            .unwrap_or_default()
    }

    /// The parts of `ch`'s vertical glyph assembly, bottom to top (Latin
    /// Modern Math's `(`: bottom hook, extender, top hook; `{`: bottom,
    /// extender, middle, extender, top); empty when it has none.
    pub fn vassembly_parts(&self, ch: char) -> &[AssemblyPart] {
        self.face
            .face()
            .glyph_id(ch)
            .and_then(|g| self.vert.get(&g.0))
            .map(|(_, parts)| parts.as_slice())
            .unwrap_or(&[])
    }

    /// `MathVariants.minConnectorOverlap` in font units.
    pub fn min_connector_overlap(&self) -> u16 {
        self.min_connector_overlap
    }

    /// The face's units per em, for scaling assembly measurements.
    pub fn units_per_em(&self) -> f64 {
        f64::from(self.face.units_per_em)
    }

    /// Paints `ch`'s vertical glyph assembly to exactly `span` pt at
    /// `size_pt`: `(glyph id, the height of its ink bottom above the bottom
    /// of the assembly)`, bottom to top.
    ///
    /// The OpenType assembly algorithm: the non-extender parts appear once,
    /// each extender repeats the same number of times, and consecutive parts
    /// overlap. The overlap is the same at every joint (what LuaTeX and
    /// HarfBuzz do), at least `minConnectorOverlap` and at most the joint's
    /// own `min(endConnectorLength of the lower part, startConnectorLength
    /// of the upper)`; the repeat count is the smallest that can cover
    /// `span` without going below the minimum overlap. Every Latin Modern
    /// Math vertical part draws from its origin up to exactly its
    /// `fullAdvance`, so an assembly laid out this way has ink from 0 to
    /// `span` and no seam. `None` when `ch` has no assembly.
    pub fn vertical_assembly(&self, ch: char, span: f64, size_pt: f64) -> Option<Vec<(u16, f64)>> {
        let parts = self.vassembly_parts(ch);
        if parts.is_empty() || size_pt <= 0.0 {
            return None;
        }
        let upem = self.units_per_em();
        let want = span * upem / size_pt;
        let min_overlap = f64::from(self.min_connector_overlap);
        let fixed: f64 = parts.iter().filter(|p| !p.extender).map(|p| f64::from(p.full_advance)).sum();
        let stretch: f64 = parts.iter().filter(|p| p.extender).map(|p| f64::from(p.full_advance)).sum();
        let fixed_n = parts.iter().filter(|p| !p.extender).count();
        let ext_n = parts.iter().filter(|p| p.extender).count();
        // The smallest repeat count whose parts still cover `want` when they
        // overlap by the least the font allows.
        let mut repeats = 0usize;
        loop {
            let n = fixed_n + ext_n * repeats;
            if n == 0 {
                if ext_n == 0 {
                    return None;
                }
                repeats += 1;
                continue;
            }
            let longest = fixed + stretch * repeats as f64 - (n - 1) as f64 * min_overlap;
            if longest >= want || repeats >= MAX_ASSEMBLY_REPEATS {
                break;
            }
            repeats += 1;
        }
        let mut seq: Vec<&AssemblyPart> = Vec::new();
        for p in parts {
            if p.extender {
                seq.extend(std::iter::repeat_n(p, repeats));
            } else {
                seq.push(p);
            }
        }
        if seq.is_empty() {
            return None;
        }
        let total: f64 = seq.iter().map(|p| f64::from(p.full_advance)).sum();
        let joints = seq.len().saturating_sub(1);
        // The uniform overlap that makes the assembly exactly `want` long,
        // held inside the font's limits. It can only hit the upper clamp
        // when even one fewer repeat would leave the parts below the minimum
        // overlap, which no Latin Modern Math delimiter does.
        let max_overlap = seq
            .windows(2)
            .map(|w| f64::from(w[0].end_connector.min(w[1].start_connector)))
            .fold(f64::INFINITY, f64::min)
            .max(min_overlap);
        let overlap = if joints == 0 {
            0.0
        } else {
            ((total - want) / joints as f64).clamp(min_overlap, max_overlap)
        };
        let mut out = Vec::with_capacity(seq.len());
        let mut rise = 0.0;
        for p in &seq {
            out.push((p.gid, rise * size_pt / upem));
            rise += f64::from(p.full_advance) - overlap;
        }
        Some(out)
    }

    /// The character actually drawn for a math symbol: letters and lower-case
    /// Greek go to the Unicode mathematical-italic block (what `cmmi` is to
    /// `cmr`), everything else is itself.
    pub fn math_char(ch: char) -> char {
        match ch {
            'h' => '\u{210E}',
            'a'..='z' => char::from_u32(0x1D44E + (ch as u32 - 'a' as u32)).unwrap_or(ch),
            'A'..='Z' => char::from_u32(0x1D434 + (ch as u32 - 'A' as u32)).unwrap_or(ch),
            '\u{3B1}'..='\u{3C9}' => char::from_u32(0x1D6FC + (ch as u32 - 0x3B1)).unwrap_or(ch),
            '\u{2202}' => '\u{1D715}', // partial
            '\u{3F5}' => '\u{1D716}',  // epsilon
            '\u{3D1}' => '\u{1D717}',  // theta variant
            '\u{3F0}' => '\u{1D718}',  // kappa variant
            '\u{3D5}' => '\u{1D719}',  // phi variant
            '\u{3F1}' => '\u{1D71A}',  // rho variant
            '\u{3D6}' => '\u{1D71B}',  // pi variant
            VARNOTHING_SENTINEL => '\u{2205}',
            _ => match ams_of(ch) {
                Some(ams) => ams.text.chars().next().unwrap_or(ch),
                None => ch,
            },
        }
    }

    /// Semantic Unicode corrections for math glyph text. This is separate
    /// from [`math_char`], which selects the painted OpenType glyph. The
    /// default pdfTeX cmex/cmsy maps are font-slot artefacts (for example,
    /// `\sum` maps to `P`), so they are not the extraction oracle here.
    pub fn extraction_text(ch: char) -> Option<&'static str> {
        match ch {
            '-' => Some("\u{2212}"),
            '*' => Some("\u{2217}"),
            '\u{03C6}' => Some("\u{03D5}"),
            '\u{03D5}' => Some("\u{03C6}"),
            _ => None,
        }
    }

    fn glyph_for(&self, gid: u16, ch: char, size_pt: f64) -> Glyph {
        Self::glyph_from(&self.face, MATH_FONT, gid, ch, size_pt)
    }

    /// `gid` of `face` as a math glyph tagged `font_id`: advance, ink box
    /// and (when the face has a `MATH` table) italic correction and accent
    /// attachment, at `size_pt`.
    fn glyph_from(face: &Rc<LoadedFace>, font_id: MathFontId, gid: u16, ch: char, size_pt: f64) -> Glyph {
        let g = GlyphId(gid);
        let adv = i64::from(face.face().advance(g).unwrap_or(0));
        let b = face.bounds(g, Some(ch));
        let (height, depth) = if b.empty {
            (0.0, 0.0)
        } else {
            (face.pt(i64::from(b.y_max), size_pt), face.pt(-i64::from(b.y_min), size_pt))
        };
        let width = face.pt(adv, size_pt);
        let (italic, skew) = match face.math() {
            Some(m) => {
                let ic = face.pt(i64::from(m.italics_correction(g)), size_pt);
                let skew = m
                    .top_accent_attachment(g)
                    .map(|t| face.pt(i64::from(t), size_pt) - width / 2.0)
                    .unwrap_or(0.0);
                (ic, skew)
            }
            None => (0.0, 0.0),
        };
        Glyph {
            font_id,
            gid,
            ch,
            size: size_pt,
            width,
            height,
            depth,
            italic,
            skew,
        }
    }

    fn base_gid(&self, ch: char) -> Option<u16> {
        let drawn = Self::math_char(ch);
        let g = self.face.face().glyph_id(drawn).or_else(|| self.face.face().glyph_id(ch));
        if g.is_none() {
            self.missing.borrow_mut().push(ch);
        }
        g.map(|g| g.0)
    }

    /// Base glyph followed by its vertical variants, in increasing size.
    fn variants(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        let Some(base) = self.base_gid(ch) else {
            return Vec::new();
        };
        let size_pt = self.sizes.at(size);
        let drawn = Self::math_char(ch);
        let mut out = vec![self.glyph_for(base, drawn, size_pt)];
        if let Some((vs, _)) = self.vert.get(&base) {
            for v in vs {
                if v.gid != base {
                    out.push(self.glyph_for(v.gid, drawn, size_pt));
                }
            }
        }
        out.sort_by(|a, b| a.total_height().partial_cmp(&b.total_height()).unwrap_or(std::cmp::Ordering::Equal));
        out
    }
}

impl MathFontMetrics for MathFonts {
    fn params(&self, size: SizeClass) -> MathParams {
        let upem = self.face.units_per_em as u16;
        MathParams::from_opentype(&self.constants, self.x_height_units, upem, self.sizes.at(size))
    }

    fn font_name(&self, font: MathFontId) -> String {
        match (&self.bb, font) {
            (Some(bb), BB_FONT) => bb.name.clone(),
            _ => self.face.name.clone(),
        }
    }

    fn glyph(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        if is_secondary_face(ch) {
            // `VARNOTHING_SENTINEL` has no glyph of its own in either face;
            // it stands for U+2205, which both faces do carry.
            let drawn = Self::math_char(ch);
            match &self.bb {
                Some(bb) => {
                    if let Some(gid) = bb.face().glyph_id(drawn) {
                        return Some(Self::glyph_from(bb, BB_FONT, gid.0, drawn, self.sizes.at(size)));
                    }
                }
                None => *self.bb_fallback.borrow_mut() = true,
            }
        }
        let gid = self.base_gid(ch)?;
        Some(self.glyph_for(gid, Self::math_char(ch), self.sizes.at(size)))
    }

    fn large_operator(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        let v = self.variants(ch, size);
        // The first variant strictly taller than the text-size glyph.
        let base_h = v.first()?.total_height();
        v.into_iter().find(|g| g.total_height() > base_h + 1e-6)
    }

    fn delimiter_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        self.variants(ch, size)
    }

    fn radical_sizes(&self, size: SizeClass) -> Vec<Glyph> {
        self.variants('\u{221A}', size)
    }

    fn accent_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        self.glyph(ch, size).into_iter().collect()
    }
}

/// `MathVariants` in full: `minConnectorOverlap`, the vertical
/// constructions and the horizontal ones.
///
/// OpenType `MathVariants` is `minConnectorOverlap`, `vertGlyphCoverage`,
/// `horizGlyphCoverage`, `vertGlyphCount`, `horizGlyphCount`, then the
/// vertical and horizontal construction offsets; a `MathGlyphConstruction`
/// is `glyphAssemblyOffset`, `variantCount` and 4-byte `(variantGlyph,
/// advanceMeasurement)` records; a `GlyphAssembly` is a 4-byte
/// `italicsCorrection` `MathValueRecord`, `partCount` and 10-byte
/// `GlyphPartRecord`s (`glyphID`, `startConnectorLength`,
/// `endConnectorLength`, `fullAdvance`, `partFlags`).
///
/// Both axes read the assembly: a delimiter or brace taller than the
/// largest variant is built from its vertical parts
/// ([`MathFonts::vertical_assembly`]), a `\overbrace`/`\underbrace` from
/// its horizontal ones.
fn parse_variants(m: &[u8]) -> Result<(u16, BTreeMap<u16, Construction>, BTreeMap<u16, Construction>), flashtex_font_engine::Error> {
    let (mut vert, mut horiz) = (BTreeMap::new(), BTreeMap::new());
    let v = usize::from(u16_at(m, 8)?);
    if v == 0 {
        return Ok((0, vert, horiz));
    }
    let min_overlap = u16_at(m, v)?;
    let vert_cov = usize::from(u16_at(m, v + 2)?);
    let horiz_cov = usize::from(u16_at(m, v + 4)?);
    let vert_count = usize::from(u16_at(m, v + 6)?);
    let horiz_count = usize::from(u16_at(m, v + 8)?);
    // The construction offsets are one array of `vertGlyphCount` vertical
    // entries followed by `horizGlyphCount` horizontal ones.
    if vert_cov != 0 {
        for (i, gid) in parse_coverage(m, v + vert_cov)?.iter().enumerate().take(vert_count) {
            vert.insert(*gid, parse_construction(m, v + usize::from(u16_at(m, v + 10 + 2 * i)?))?);
        }
    }
    if horiz_cov != 0 {
        for (i, gid) in parse_coverage(m, v + horiz_cov)?.iter().enumerate().take(horiz_count) {
            let at = v + 10 + 2 * vert_count + 2 * i;
            horiz.insert(*gid, parse_construction(m, v + usize::from(u16_at(m, at)?))?);
        }
    }
    Ok((min_overlap, vert, horiz))
}

/// One `MathGlyphConstruction` at `cons`: its variant records and, when
/// `glyphAssemblyOffset` is non-zero, its assembly parts in order.
fn parse_construction(m: &[u8], cons: usize) -> Result<Construction, flashtex_font_engine::Error> {
    let assembly = usize::from(u16_at(m, cons)?);
    let n = usize::from(u16_at(m, cons + 2)?);
    let mut variants = Vec::with_capacity(n);
    for j in 0..n {
        let rec = cons + 4 + 4 * j;
        variants.push(VertVariant {
            gid: u16_at(m, rec)?,
            advance: u16_at(m, rec + 2)?,
        });
    }
    let mut parts = Vec::new();
    if assembly != 0 {
        let a = cons + assembly;
        let count = usize::from(u16_at(m, a + 4)?);
        parts.reserve(count);
        for k in 0..count {
            let p = a + 6 + 10 * k;
            parts.push(AssemblyPart {
                gid: u16_at(m, p)?,
                start_connector: u16_at(m, p + 2)?,
                end_connector: u16_at(m, p + 4)?,
                full_advance: u16_at(m, p + 6)?,
                extender: u16_at(m, p + 8)? & 1 != 0,
            });
        }
    }
    Ok((variants, parts))
}

/// OpenType coverage table -> glyph ids in coverage-index order.
fn parse_coverage(b: &[u8], at: usize) -> Result<Vec<u16>, flashtex_font_engine::Error> {
    let format = u16_at(b, at)?;
    let mut gids = Vec::new();
    match format {
        1 => {
            let n = usize::from(u16_at(b, at + 2)?);
            for i in 0..n {
                gids.push(u16_at(b, at + 4 + 2 * i)?);
            }
        }
        2 => {
            let n = usize::from(u16_at(b, at + 2)?);
            for i in 0..n {
                let rec = at + 4 + 6 * i;
                let start = u16_at(b, rec)?;
                let end = u16_at(b, rec + 2)?;
                if start > end {
                    return Err(flashtex_font_engine::Error::Malformed("coverage range".into()));
                }
                gids.extend(start..=end);
            }
        }
        other => {
            return Err(flashtex_font_engine::Error::Unsupported(format!("coverage format {other}")));
        }
    }
    Ok(gids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::{Family, FontSet, Role, DEFAULT_FONT_DIRS};
    use flashtex_math_layout::{layout_with_report, Atom, BoxKind, MathList, Style};

    fn lm_math() -> Option<MathFonts> {
        if !DEFAULT_FONT_DIRS.iter().any(|d| std::path::Path::new(d).join("latinmodern-math.otf").is_file()) {
            eprintln!("skipping: Latin Modern Math not installed");
            return None;
        }
        let fonts = FontSet::with_default_dirs(&[]);
        let face = fonts.resolve(Family::LatinModern, Role::Math, 10.0).face;
        MathFonts::new(
            face,
            MathSizes {
                text: 10.0,
                script: 7.0,
                script_script: 5.0,
            },
        )
    }

    #[test]
    fn parameters_come_from_the_math_table() {
        let Some(m) = lm_math() else { return };
        let p = m.params(SizeClass::Text);
        // Latin Modern Math AxisHeight = 250, FractionRuleThickness = 40.
        assert!((p.axis_height - 2.5).abs() < 1e-9, "{}", p.axis_height);
        assert!((p.default_rule_thickness - 0.4).abs() < 1e-9);
        assert!((p.x_height - 4.31).abs() < 0.02, "{}", p.x_height);
        assert!(p.num1 > p.num2 && p.denom1 > p.denom2);
    }

    #[test]
    fn vertical_assemblies_are_read_from_math_variants() {
        let Some(m) = lm_math() else { return };
        assert_eq!(m.min_connector_overlap(), 20);
        // Latin Modern Math's `(`: bottom hook, extender, top hook, bottom to
        // top, the ends connecting over half the extender's length.
        let paren = m.vassembly_parts('(');
        assert_eq!(
            paren,
            [
                AssemblyPart { gid: 2503, start_connector: 0, end_connector: 249, full_advance: 1495, extender: false },
                AssemblyPart { gid: 2504, start_connector: 498, end_connector: 498, full_advance: 498, extender: true },
                AssemblyPart { gid: 2505, start_connector: 249, end_connector: 0, full_advance: 1495, extender: false },
            ]
        );
        // `{` has a middle part between two extenders; `\lceil` has no bottom
        // and `\lfloor` no top, exactly as cmex's recipes do.
        let brace = m.vassembly_parts('{');
        assert_eq!(brace.len(), 5);
        assert_eq!(brace.iter().filter(|p| p.extender).count(), 2);
        assert_eq!(brace[2].full_advance, 1500);
        assert!(m.vassembly_parts('\u{2308}')[0].extender, "\\lceil starts with its extender");
        assert!(m.vassembly_parts('\u{230A}').last().expect("parts").extender, "\\lfloor ends with its extender");
        // The extender of `|` is longer than cmex's 0.6 em repeat, which is
        // why a piece cannot be painted on its own.
        assert_eq!(m.vassembly_parts('|')[1].full_advance, 1202);
        assert!(m.vassembly_parts('x').is_empty());
    }

    #[test]
    fn an_assembly_is_built_to_the_wanted_span() {
        let Some(m) = lm_math() else { return };
        // pdfTeX stacks `\left[` around a six-row array to 72.00072 pt; the
        // font needs its bottom, five extenders and its top for that.
        let parts = m.vertical_assembly('[', 72.00072, 10.0).expect("assembly");
        assert_eq!(parts.len(), 7);
        assert_eq!(parts.first().expect("parts").1, 0.0, "the first part starts at the bottom");
        let (top_gid, top_rise) = *parts.last().expect("parts");
        let full = |gid: u16| {
            f64::from(m.vassembly_parts('[').iter().find(|p| p.gid == gid).expect("part").full_advance) / 100.0
        };
        assert!((top_rise + full(top_gid) - 72.00072).abs() < 1e-6, "the last part ends at the top");
        for pair in parts.windows(2) {
            let overlap = full(pair[0].0) - (pair[1].1 - pair[0].1);
            assert!((0.2..=5.0).contains(&overlap), "overlap {overlap} pt");
        }
        // A span the fixed parts alone already cover needs no extender pass.
        let short = m.vertical_assembly('[', 30.0, 10.0).expect("assembly");
        assert!(short.len() >= 2 && short.len() < 7, "{}", short.len());
    }

    #[test]
    fn letters_are_math_italic_and_operators_have_display_variants() {
        let Some(m) = lm_math() else { return };
        let x = m.glyph('x', SizeClass::Text).unwrap();
        assert_eq!(x.ch, '\u{1D465}');
        assert!(x.width > 4.0 && x.width < 7.0);
        assert!(x.height > 4.0 && x.depth < 0.2, "{} {}", x.height, x.depth);
        let sum = m.glyph('\u{2211}', SizeClass::Text).unwrap();
        let big = m.large_operator('\u{2211}', SizeClass::Text).expect("display-size sum");
        assert!(big.total_height() > sum.total_height());
        let parens = m.delimiter_sizes('(', SizeClass::Text);
        assert!(parens.len() >= 3, "{} paren sizes", parens.len());
        assert!(parens.windows(2).all(|w| w[0].total_height() <= w[1].total_height()));
        assert!(m.radical_sizes(SizeClass::Text).len() >= 2);
    }

    #[test]
    fn extraction_text_uses_semantic_math_unicode() {
        let expected = [
            ('-', "−"),
            ('*', "∗"),
            ('\u{03C6}', "ϕ"),
            ('\u{03D5}', "φ"),
        ];
        for (ch, text) in expected {
            assert_eq!(MathFonts::extraction_text(ch), Some(text), "{ch:?}");
        }
        // pdfTeX's default cmex/cmsy mappings make copy-paste worse by
        // exposing font-slot artefacts such as `\sum` -> `P`; semantic
        // Unicode, not that output, is the contract.
        for ch in [
            '\u{2217}',
            '\u{2218}',
            '\u{22C5}',
            '\u{2216}',
            '\u{0338}',
            '\u{21A6}',
            '\u{27F9}',
            '\u{27F6}',
            '\u{21AA}',
            '\u{03BC}',
            '\u{0394}',
            '\u{03A9}',
            '\u{2211}',
            '\u{220F}',
            '\u{222B}',
            '\u{222E}',
        ] {
            assert_eq!(MathFonts::extraction_text(ch), None, "{ch:?}");
        }
    }

    #[test]
    fn fraction_bar_is_an_explicit_rule() {
        let Some(m) = lm_math() else { return };
        let list = MathList::new(vec![Atom::frac(MathList::symbols("1"), MathList::symbols("2"))]);
        let l = layout_with_report(&list, Style::TEXT, &m);
        assert!(l.limitations.is_empty(), "{:?}", l.limitations);
        let runs = flashtex_math_layout::positioned_runs(&l.root, (0.0, 0.0));
        assert_eq!(runs.rules.len(), 1);
        assert!((runs.rules[0].h - 0.4).abs() < 1e-9);
        assert_eq!(runs.glyphs.len(), 2);
        assert!(matches!(l.root.kind, BoxKind::HBox(_)));
    }
}
