//! `MathFontMetrics` for math-layout, driven by an OpenType `MATH` face
//! through font-engine: Appendix G parameters from the `MATH` constants
//! (`MathParams::from_opentype`, the LuaTeX correspondence) and the
//! constants beyond them (`OpenTypeExtras`), glyph metrics from the face
//! advances and the CFF charstring bounds, italic corrections, top-accent
//! attachment and cut-in kerns from `MathGlyphInfo`, and the vertical and
//! horizontal variants and glyph assemblies (display-size operators,
//! larger delimiters, radical signs, wide accents) from `MathVariants`, all
//! read by font-engine's `MathTable`.
//!
//! Two callers: `TexMathMetrics` (pdfLaTeX's geometry) paints its TFM boxes
//! with this provider's Latin Modern Math outlines and never asks it to lay
//! anything out, and `MathProvider::Otf` lays the whole formula out from
//! the table -- for a document that selects an OpenType math font
//! (`\setmathfont`, `unicode-math`, the manifest's `[fonts] math`;
//! [`MathFonts::named`]), and as the fallback when the `lm` TFMs are
//! missing. Under `\usepackage{times}` math stays in Computer Modern, so
//! the Latin Modern Math provider serves both text families.
//!
//! Blackboard bold is the exception to "one face" in the TFM route:
//! pdfLaTeX's `\mathbb` comes from AMS `msbm10`, a serifed double-struck
//! design, while Latin Modern Math's double-struck block is the sans-like
//! open-face design. New Computer Modern Math reproduces the msbm design
//! (and its widths track msbm's), so when `NewCMMath-Regular.otf` is in a
//! font directory every double-struck code point is drawn from it as a
//! secondary face ([`BB_FONT`]); otherwise Latin Modern Math draws it and
//! the typesetter reports the profile difference once. A named math font
//! draws its own double-struck block, as unicode-math does.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use flashtex_font_engine::math::{KernCorner as OtfCorner, MathTable};
use flashtex_math_layout::{
    Assembly, AssemblyPart as LayoutPart, FontId as MathFontId, Glyph, KernCorner, MathFontMetrics, MathParams, OpenTypeExtras,
    OpenTypeMathConstants, SizeClass,
};

use crate::cff::u16_at;
use crate::fonts::LoadedFace;
use crate::ids::GlyphId;
use crate::mathalpha::MathAlphabet;

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

    /// The sizes unicode-math declares for a document set in an OpenType
    /// math font (`\__um_declare_math_sizes:`): script and scriptscript at
    /// the face's `ScriptPercentScaleDown`/`ScriptScriptPercentScaleDown`
    /// of the text size, whatever `\DeclareMathSizes` the class has.
    /// Measured on LuaLaTeX + unicode-math in a 12pt article, `f^{2^3}`:
    /// Latin Modern Math (70/50) sets the `2` at 8.4 pt and the `3` at
    /// 6 pt; STIX Two Math (70/55) the `3` at 6.6 pt (its `three.ssty2`,
    /// 561 units, comes out 3.7026 pt wide).
    pub fn unicode_math(text: f64, c: &flashtex_font_engine::math::MathConstants) -> MathSizes {
        let pct = |p: i16| f64::from(p.clamp(1, 100)) / 100.0;
        MathSizes {
            text,
            script: text * pct(c.script_percent_scale_down),
            script_script: text * pct(c.script_script_percent_scale_down),
        }
    }

    fn index(size: SizeClass) -> usize {
        match size {
            SizeClass::Text => 0,
            SizeClass::Script => 1,
            SizeClass::ScriptScript => 2,
        }
    }
}

/// One `GlyphPartRecord` of a `MathVariants` glyph assembly, font units.
///
/// `full_advance` is the part's own extent along the assembly axis (its ink
/// height for a vertical part: every Latin Modern Math vertical part draws
/// from its origin up to exactly `fullAdvance`). `start_connector` and
/// `end_connector` are how much of the part may be overlapped by the
/// neighbour before it and after it -- the joint between two parts may
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

impl From<&flashtex_font_engine::math::GlyphPart> for AssemblyPart {
    fn from(p: &flashtex_font_engine::math::GlyphPart) -> AssemblyPart {
        AssemblyPart {
            gid: p.gid.0,
            start_connector: p.start_connector,
            end_connector: p.end_connector,
            full_advance: p.full_advance,
            extender: p.extender,
        }
    }
}

/// How often one extender may repeat in an assembly. A `\left(` around a
/// page-tall box needs about 10 (Latin Modern Math's paren extender is
/// 0.498 em); the cap only keeps a malformed font from looping.
const MAX_ASSEMBLY_REPEATS: usize = 256;

/// The text faces a math alphabet draws from ([`MathFonts::with_text_alphabets`]):
/// fontspec's `\mathbf`/`\mathsf`/`\mathit`/`\mathtt` and `\mathrm`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlphabet {
    Bold,
    Sans,
    Italic,
    Mono,
    /// The upright roman face: `\mathrm{K}` and the operator text.
    Roman,
}

impl TextAlphabet {
    fn index(self) -> u32 {
        match self {
            TextAlphabet::Bold => 0,
            TextAlphabet::Sans => 1,
            TextAlphabet::Italic => 2,
            TextAlphabet::Mono => 3,
            TextAlphabet::Roman => 4,
        }
    }

    /// The alphabet a math alphanumeric symbol belongs to, for the four
    /// that are text fonts.
    fn of(alphabet: MathAlphabet) -> Option<TextAlphabet> {
        match alphabet {
            MathAlphabet::Bold => Some(TextAlphabet::Bold),
            MathAlphabet::Sans => Some(TextAlphabet::Sans),
            MathAlphabet::Italic => Some(TextAlphabet::Italic),
            MathAlphabet::Mono => Some(TextAlphabet::Mono),
            MathAlphabet::Fraktur => None,
        }
    }

    /// The NFSS shape of the alphabet's text face.
    pub fn key(self) -> crate::nfss::FontKey {
        use crate::nfss::{FamilyKind, FontKey, Series, Shape};
        match self {
            TextAlphabet::Bold => FontKey::new(FamilyKind::Rm, Series::Bx, Shape::N),
            TextAlphabet::Sans => FontKey::new(FamilyKind::Sf, Series::M, Shape::N),
            TextAlphabet::Italic => FontKey::new(FamilyKind::Rm, Series::M, Shape::It),
            TextAlphabet::Mono => FontKey::new(FamilyKind::Tt, Series::M, Shape::N),
            TextAlphabet::Roman => FontKey::new(FamilyKind::Rm, Series::M, Shape::N),
        }
    }

    pub const ALL: [TextAlphabet; 5] = [TextAlphabet::Bold, TextAlphabet::Sans, TextAlphabet::Italic, TextAlphabet::Mono, TextAlphabet::Roman];
}

pub struct MathFonts {
    face: Rc<LoadedFace>,
    /// The double-struck face (New Computer Modern Math), when found.
    bb: Option<Rc<LoadedFace>>,
    /// Why `bb` is absent (the font set's reason), for the profile note.
    bb_status: Option<String>,
    /// Set once a double-struck glyph was served from `face` because `bb`
    /// is absent; drained by the typesetter for its one profile note.
    bb_fallback: RefCell<bool>,
    /// The document's own math font (`\setmathfont`, `unicode-math`, the
    /// manifest): every glyph comes from `face` (or a text alphabet face),
    /// never from the secondary face, and no profile note is owed.
    named: bool,
    sizes: MathSizes,
    constants: OpenTypeMathConstants,
    x_height_units: i16,
    /// The `ssty` (script-style) alternates of each glyph the face's
    /// `GSUB` lists them for, in the font's order: Latin Modern Math's
    /// `minute` -> [`minute.st`, `minute.sts`].
    script_alternates: BTreeMap<u16, Vec<u16>>,
    /// The text faces of the math alphabets unicode-math sets from the text
    /// fonts, per alphabet and size index (`with_text_alphabets`).
    alphabets: Vec<(TextAlphabet, usize, Rc<LoadedFace>)>,
    /// Characters with no glyph in the math font, recorded for diagnostics.
    missing: RefCell<Vec<char>>,
}

/// `FontId(0)` is the math face (Latin Modern Math).
const MATH_FONT: MathFontId = MathFontId(0);
/// `FontId(1)` is the double-struck face (New Computer Modern Math); glyphs
/// carry it only when that face is loaded.
pub const BB_FONT: MathFontId = MathFontId(1);
/// `FontId(0x20 + 3·alphabet + size)`: a text alphabet face at a size
/// index ([`TextAlphabet`]).
const ALPHABET_FONT_BASE: u32 = 0x20;
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
        let c = table.constants();
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
            fraction_numerator_gap_min: c.fraction_numerator_gap_min,
            fraction_num_display_style_gap_min: c.fraction_num_display_style_gap_min,
            fraction_denominator_gap_min: c.fraction_denominator_gap_min,
            fraction_denom_display_style_gap_min: c.fraction_denom_display_style_gap_min,
            stack_gap_min: c.stack_gap_min,
            stack_display_style_gap_min: c.stack_display_style_gap_min,
            stack_top_display_style_shift_up: c.stack_top_display_style_shift_up,
            stack_bottom_shift_down: c.stack_bottom_shift_down,
            stack_bottom_display_style_shift_down: c.stack_bottom_display_style_shift_down,
            sub_superscript_gap_min: c.sub_superscript_gap_min,
            superscript_bottom_max_with_subscript: c.superscript_bottom_max_with_subscript,
            subscript_top_max: c.subscript_top_max,
            superscript_bottom_min: c.superscript_bottom_min,
            space_after_script: c.space_after_script,
            radical_rule_thickness: c.radical_rule_thickness,
            radical_vertical_gap: c.radical_vertical_gap,
            radical_display_style_vertical_gap: c.radical_display_style_vertical_gap,
            radical_extra_ascender: c.radical_extra_ascender,
            radical_kern_before_degree: c.radical_kern_before_degree,
            radical_kern_after_degree: c.radical_kern_after_degree,
            radical_degree_bottom_raise_percent: c.radical_degree_bottom_raise_percent,
            accent_base_height: c.accent_base_height,
            flattened_accent_base_height: c.flattened_accent_base_height,
            overbar_vertical_gap: c.overbar_vertical_gap,
            overbar_rule_thickness: c.overbar_rule_thickness,
            overbar_extra_ascender: c.overbar_extra_ascender,
            underbar_vertical_gap: c.underbar_vertical_gap,
            underbar_rule_thickness: c.underbar_rule_thickness,
            // font-engine names the specification's `UnderbarExtraDescender`
            // after its neighbour; it is the descender.
            underbar_extra_descender: c.underbar_extra_ascender,
            display_operator_min_height: c.display_operator_min_height,
        };
        let vm = face.face().vertical_metrics();
        let x_height_units = if vm.x_height_declared { vm.x_height } else { 431 };
        let script_alternates = face
            .otf()
            .and_then(|f| f.table(b"GSUB"))
            .and_then(|t| alternate_substitutions(t, b"ssty").ok())
            .unwrap_or_default();
        Some(MathFonts {
            face,
            bb: None,
            bb_status: None,
            bb_fallback: RefCell::new(false),
            named: false,
            sizes,
            constants,
            x_height_units,
            script_alternates,
            alphabets: Vec::new(),
            missing: RefCell::new(Vec::new()),
        })
    }

    /// The document's own math font (`\setmathfont{..}`, `unicode-math`,
    /// the manifest's `[fonts] math`): [`MathFonts::new`] on `face`, drawing
    /// everything from it -- no secondary double-struck face, no profile
    /// note when `\mathbb` is set from it. `None` when `face` has no `MATH`
    /// table.
    pub fn named(face: Rc<LoadedFace>, sizes: MathSizes) -> Option<MathFonts> {
        let mut m = MathFonts::new(face, sizes)?;
        m.named = true;
        Some(m)
    }

    /// Whether this is a document's own math font ([`MathFonts::named`]).
    pub fn is_named(&self) -> bool {
        self.named
    }

    /// Attaches the text faces unicode-math sets the text math alphabets
    /// from: `\mathbf`/`\mathsf`/`\mathit`/`\mathtt` are the text fonts'
    /// bold, sans, italic and typewriter faces (LuaLaTeX + unicode-math,
    /// `\setmathfont{Latin Modern Math}`: `\mathbf{x}` is LMRoman10-Bold's
    /// `x`, `\mathsf{y}` LMSans10-Regular's, `\mathtt{z}` LMMono10-Regular's,
    /// `\mathit{d}` LMRoman10-Italic's), while `\mathcal`, `\mathfrak` and
    /// `\mathbb` come from the math font's own alphanumeric blocks. `faces`
    /// is one entry per alphabet and size index (0 text, 1 script, 2
    /// scriptscript); an alphabet without a face falls back to the math
    /// font's own block for it.
    pub fn with_text_alphabets(mut self, faces: Vec<(TextAlphabet, usize, Rc<LoadedFace>)>) -> MathFonts {
        self.alphabets = faces;
        self
    }

    /// The `MATH` table of the face (present: `new` checked it).
    fn table(&self) -> &MathTable {
        self.face.math().expect("MathFonts::new checked the MATH table")
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
        self.table().vertical_variants(GlyphId(base)).iter().filter(|v| v.gid.0 != base).nth(k - 1).map(|v| v.gid.0)
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
        self.table()
            .vertical_variants(GlyphId(base))
            .iter()
            .filter(|v| v.gid.0 != base)
            .map(|v| (v.gid.0, (self.glyph_for(v.gid.0, drawn, size_pt).total_height() - wanted).abs()))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(gid, _)| gid)
    }

    /// The horizontal variant of `ch` (base glyph included) whose advance at
    /// `size_pt` is nearest `wanted` pt: what paints a cmex/msbm wide accent
    /// laid out at its TFM width.
    pub fn hvariant_nearest(&self, ch: char, size_pt: f64, wanted: f64) -> Option<u16> {
        let base = self.face.face().glyph_id(ch)?;
        let variants = self.table().horizontal_variants(base);
        if variants.is_empty() {
            return None;
        }
        let upem = f64::from(self.face.units_per_em);
        variants
            .iter()
            .map(|v| (v.gid.0, (f64::from(v.advance) * size_pt / upem - wanted).abs()))
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
            .and_then(|g| self.table().horizontal_assembly(g))
            .map(|a| a.parts.iter().map(|p| p.gid.0).collect())
            .unwrap_or_default()
    }

    /// The parts of `ch`'s vertical glyph assembly, bottom to top (Latin
    /// Modern Math's `(`: bottom hook, extender, top hook; `{`: bottom,
    /// extender, middle, extender, top); empty when it has none.
    pub fn vassembly_parts(&self, ch: char) -> Vec<AssemblyPart> {
        self.face
            .face()
            .glyph_id(ch)
            .and_then(|g| self.table().vertical_assembly(g))
            .map(|a| a.parts.iter().map(AssemblyPart::from).collect())
            .unwrap_or_default()
    }

    /// Whether `gid` is a part of the vertical assembly of `ch`: the
    /// painter joins a run of them into one cluster.
    pub fn is_assembly_part(&self, ch: char, gid: u16) -> bool {
        self.vassembly_parts(ch).iter().any(|p| p.gid == gid)
    }

    /// The face's first `ssty` alternate of `gid`: the design for script
    /// style. `None` when the face lists none.
    pub fn script_alternate(&self, gid: u16) -> Option<u16> {
        self.script_alternates.get(&gid).and_then(|a| a.first().copied())
    }

    /// The glyph the document's own math font sets `gid` with at `size`:
    /// its `ssty` script form at script size and scriptscript form at
    /// scriptscript size, as unicode-math loads `\scriptfont` with `ssty=1`
    /// and `\scriptscriptfont` with `ssty=2` (LuaLaTeX, Latin Modern Math
    /// `\frac{a}{b}`: the 7 pt `a` is `u1D44E.st`, 4.34 pt wide, not the
    /// text form's 3.70). The base glyph when the face lists no alternate.
    fn script_gid(&self, gid: u16, size: SizeClass) -> u16 {
        let Some(alternates) = self.script_alternates.get(&gid) else {
            return gid;
        };
        match size {
            SizeClass::Text => gid,
            SizeClass::Script => alternates.first().copied().unwrap_or(gid),
            SizeClass::ScriptScript => alternates.get(1).or(alternates.first()).copied().unwrap_or(gid),
        }
    }

    /// `MathVariants.minConnectorOverlap` in font units.
    pub fn min_connector_overlap(&self) -> u16 {
        self.table().min_connector_overlap()
    }

    /// The face's units per em, for scaling assembly measurements.
    pub fn units_per_em(&self) -> f64 {
        f64::from(self.face.units_per_em)
    }

    /// The face that draws a placed glyph's `font_id`: the math face, the
    /// secondary double-struck face, or a text alphabet face.
    pub fn face_of(&self, font: MathFontId) -> Option<Rc<LoadedFace>> {
        if font == MATH_FONT {
            return Some(self.face.clone());
        }
        if font == BB_FONT {
            return self.bb.clone();
        }
        self.alphabets.iter().find(|(a, i, _)| Self::alphabet_font(*a, *i) == font).map(|(_, _, f)| f.clone())
    }

    /// Font id of a text alphabet face at size index `i`.
    fn alphabet_font(alphabet: TextAlphabet, i: usize) -> MathFontId {
        MathFontId(ALPHABET_FONT_BASE + 3 * alphabet.index() + i as u32)
    }

    /// `letter` from the text face of `alphabet` at `size`, tagged `ch`
    /// (the math alphanumeric the compiler emitted), when the face is
    /// attached ([`MathFonts::with_text_alphabets`]).
    fn alphabet_glyph(&self, alphabet: TextAlphabet, letter: char, ch: char, size: SizeClass) -> Option<Glyph> {
        let i = MathSizes::index(size);
        let (_, _, face) = self.alphabets.iter().find(|(a, j, _)| *a == alphabet && *j == i)?;
        let gid = face.face().glyph_id(letter)?;
        let mut g = Self::glyph_from(face, Self::alphabet_font(alphabet, i), gid.0, ch, self.sizes.at(size));
        g.height = g.height.max(0.0);
        g.depth = g.depth.max(0.0);
        // A text face has no MATH table: LuaTeX (luaotfload) gives an
        // italic face's glyphs the italic correction by which their ink
        // overhangs the advance (`\mathit{d}\mathrm{d}` in Latin Modern:
        // 0.69 pt between the two `d`s).
        if alphabet == TextAlphabet::Italic {
            let b = face.bounds(GlyphId(gid.0), Some(letter));
            if !b.empty {
                g.italic = (face.pt(i64::from(b.x_max), g.size) - g.width).max(0.0);
            }
        }
        Some(g)
    }

    /// `ch`'s vertical glyph assembly at `size` for math-layout, when the
    /// face has one: every part as a glyph box plus its connectors and
    /// full advance in points.
    fn assembly(&self, ch: char, size: SizeClass) -> Option<Assembly> {
        let drawn = Self::math_char(ch);
        let gid = self.face.face().glyph_id(drawn).or_else(|| self.face.face().glyph_id(ch))?;
        let a = self.table().vertical_assembly(gid)?;
        if a.parts.is_empty() {
            return None;
        }
        let size_pt = self.sizes.at(size);
        let pt = |u: u16| self.face.pt(i64::from(u), size_pt);
        Some(Assembly {
            parts: a
                .parts
                .iter()
                .map(|p| LayoutPart {
                    glyph: self.glyph_for(p.gid.0, drawn, size_pt),
                    start_connector: pt(p.start_connector),
                    end_connector: pt(p.end_connector),
                    full_advance: pt(p.full_advance),
                    extender: p.extender,
                })
                .collect(),
            min_overlap: pt(self.table().min_connector_overlap()),
        })
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
        let parts = parts.as_slice();
        let upem = self.units_per_em();
        let want = span * upem / size_pt;
        let min_overlap = f64::from(self.min_connector_overlap());
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
        let mut g = Self::glyph_from(&self.face, MATH_FONT, gid, ch, size_pt);
        if self.named {
            // A box has no negative height or depth: LuaTeX's `char_height`/
            // `char_depth` of an OpenType glyph whose ink lies entirely
            // above the baseline (a combining accent, an arrow) are its
            // bbox top and 0. The TFM route keeps the raw values it has
            // always laid its fallback glyphs out with.
            g.height = g.height.max(0.0);
            g.depth = g.depth.max(0.0);
        }
        g
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

    /// The character a math symbol the compiler spells in ASCII is set
    /// as when this provider lays the formula out (unicode-math's
    /// `\mathcode`s): the hyphen-minus is the minus sign, `*` the asterisk
    /// operator; everything else is [`MathFonts::math_char`]. The TeX
    /// provider never comes through here for these two (their cmsy slots
    /// are mapped in `TexMathMetrics::otf_gid`).
    fn laid_out_char(ch: char) -> char {
        match ch {
            '-' => '\u{2212}',
            '*' => '\u{2217}',
            // `typeset::accent_char` spells the accents as the spacing
            // modifiers TeX's `\mathaccent` slots hold; unicode-math sets
            // the combining marks, whose zero advance and top-accent anchor
            // are what Rule 12 reads here (`\widehat`/`\widetilde`/`\vec`
            // are combining already).
            '\u{02C6}' => '\u{0302}', // \hat
            '\u{00AF}' => '\u{0304}', // \bar
            '\u{02DC}' => '\u{0303}', // \tilde
            '\u{02D9}' => '\u{0307}', // \dot
            '\u{00A8}' => '\u{0308}', // \ddot
            '\u{02C7}' => '\u{030C}', // \check
            '\u{02D8}' => '\u{0306}', // \breve
            '\u{00B4}' => '\u{0301}', // \acute
            '`' => '\u{0300}',        // \grave
            '\u{02DA}' => '\u{030A}', // \mathring
            _ => Self::math_char(ch),
        }
    }

    fn base_gid(&self, ch: char) -> Option<u16> {
        let drawn = Self::laid_out_char(ch);
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
        for v in self.table().vertical_variants(GlyphId(base)) {
            if v.gid.0 != base {
                out.push(self.glyph_for(v.gid.0, drawn, size_pt));
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
            _ => self.face_of(font).map_or_else(|| self.face.name.clone(), |f| f.name.clone()),
        }
    }

    fn glyph(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        if !self.named && is_secondary_face(ch) {
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
        // A one-letter text alphabet (`\mathbf{x}`, `\mathsf{y}`, ...,
        // arriving as its math alphanumeric): the text face, as unicode-math
        // sets it, when one is attached; else the math font's own block.
        if let Some((alphabet, letter)) = crate::mathalpha::classify(ch) {
            if let Some(g) = TextAlphabet::of(alphabet).and_then(|text| self.alphabet_glyph(text, letter, ch, size)) {
                return Some(g);
            }
        }
        let gid = self.base_gid(ch)?;
        let gid = if self.named { self.script_gid(gid, size) } else { gid };
        Some(self.glyph_for(gid, Self::laid_out_char(ch), self.sizes.at(size)))
    }

    /// Rule 13 in display style: the first variant at least
    /// `DisplayOperatorMinHeight` tall (LuaTeX `Umathoperatorsize`; the
    /// manual's note 6), the largest when none is, and none at all when
    /// the face lists no variant.
    fn large_operator(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        let v = self.variants(ch, size);
        if v.len() < 2 {
            return None;
        }
        let min = self.opentype_extras(size).map_or(0.0, |e| e.display_operator_min_height);
        let base_h = v[0].total_height();
        v.iter()
            .find(|g| g.total_height() >= min && g.total_height() > base_h + 1e-6)
            .or(v.last())
            .copied()
    }

    fn delimiter_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        self.variants(ch, size)
    }

    fn radical_sizes(&self, size: SizeClass) -> Vec<Glyph> {
        self.variants('\u{221A}', size)
    }

    /// The accent and, for the wide accents (`\widehat`/`\widetilde`, the
    /// combining marks `typeset::accent_char` keeps as such), its horizontal
    /// variants, narrowest first: the base combining mark has no advance,
    /// so Rule 12 keeps it over a single character and steps up to the
    /// variant no wider than a wider base. unicode-math's `\hat`, `\bar`,
    /// `\vec`, ... are fixed accents that never stretch (LuaLaTeX sets
    /// `\hat{A}` with the 0-advance hat although Latin Modern Math lists a
    /// 6.4 pt variant that would fit 𝐴's 7.5 pt), so those get the base
    /// glyph only.
    fn accent_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        let Some(base) = self.glyph(ch, size) else {
            return Vec::new();
        };
        let mut out = vec![base];
        let wide = matches!(ch, '\u{0302}' | '\u{0303}');
        if wide && base.font_id == MATH_FONT {
            let size_pt = self.sizes.at(size);
            for v in self.table().horizontal_variants(GlyphId(base.gid)) {
                if v.gid.0 != base.gid {
                    out.push(self.glyph_for(v.gid.0, base.ch, size_pt));
                }
            }
            out.sort_by(|a, b| a.width.partial_cmp(&b.width).unwrap_or(std::cmp::Ordering::Equal));
        }
        out
    }

    /// Upright text in math (`\mathrm{K}`, the operator words): the roman
    /// text face when attached, else the math font's own upright glyph.
    fn text_glyph(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        if let Some(g) = self.alphabet_glyph(TextAlphabet::Roman, ch, ch, size) {
            return Some(g);
        }
        let gid = self.face.face().glyph_id(ch)?;
        let gid = if self.named { self.script_gid(gid.0, size) } else { gid.0 };
        Some(self.glyph_for(gid, ch, self.sizes.at(size)))
    }

    fn opentype_extras(&self, size: SizeClass) -> Option<OpenTypeExtras> {
        Some(OpenTypeExtras::from_opentype(&self.constants, self.sizes.at(size)))
    }

    /// `MathKernInfo` of the math face, the height converted to the glyph's
    /// own font units and the kern back to points at its size.
    fn math_kern(&self, glyph: &Glyph, corner: KernCorner, height: f64) -> f64 {
        if glyph.font_id != MATH_FONT || glyph.size <= 0.0 {
            return 0.0;
        }
        let upem = self.units_per_em();
        let units = (height * upem / glyph.size).round().clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16;
        let corner = match corner {
            KernCorner::TopRight => OtfCorner::TopRight,
            KernCorner::TopLeft => OtfCorner::TopLeft,
            KernCorner::BottomRight => OtfCorner::BottomRight,
            KernCorner::BottomLeft => OtfCorner::BottomLeft,
        };
        self.face.pt(i64::from(self.table().kern(GlyphId(glyph.gid), corner, units)), glyph.size)
    }

    fn delimiter_assembly(&self, ch: char, size: SizeClass) -> Option<Assembly> {
        self.assembly(ch, size)
    }

    fn radical_assembly(&self, size: SizeClass) -> Option<Assembly> {
        self.assembly('\u{221A}', size)
    }
}

/// `GSUB` lookups of feature `tag` -> each covered glyph's substitute:
/// the `ssty=1` form for the math face's script alternates, the small
/// capital for `smcp`, the old-style figure for `onum` (named text
/// families, `crate::fonts::LoadedFace::feature_map`). Single
/// substitutions (type 1), the first alternate of AlternateSubst (type 3,
/// Latin Modern Math's `ssty`) and Extension wrappers (type 7) are read;
/// any other lookup type is skipped. Script and language systems are not
/// consulted: these features are the same under all of them in the fonts
/// this serves, and a language-specific `smcp` (Turkish `i`) is out of
/// scope. `Ok` with an empty map when the table has no such feature.
pub(crate) fn single_substitutions(g: &[u8], tag: &[u8; 4]) -> Result<BTreeMap<u16, u16>, flashtex_font_engine::Error> {
    Ok(alternate_substitutions(g, tag)?
        .into_iter()
        .filter_map(|(gid, alternates)| alternates.first().map(|a| (gid, *a)))
        .collect())
}

/// [`single_substitutions`] keeping every alternate of an AlternateSubst
/// in the font's order: Latin Modern Math's `ssty` lists the script form
/// (`.st`, what LuaTeX's `ssty=1` selects for `\scriptfont`) and then the
/// scriptscript form (`.sts`, `ssty=2`).
pub(crate) fn alternate_substitutions(g: &[u8], tag: &[u8; 4]) -> Result<BTreeMap<u16, Vec<u16>>, flashtex_font_engine::Error> {
    let malformed = |what: &str| flashtex_font_engine::Error::Malformed(format!("GSUB {what}"));
    let mut out: BTreeMap<u16, Vec<u16>> = BTreeMap::new();
    let features = usize::from(u16_at(g, 6)?);
    let lookups = usize::from(u16_at(g, 8)?);
    let mut indices = Vec::new();
    for i in 0..usize::from(u16_at(g, features)?) {
        let rec = features + 2 + 6 * i;
        if g.get(rec..rec + 4) != Some(&tag[..]) {
            continue;
        }
        let feature = features + usize::from(u16_at(g, rec + 4)?);
        for j in 0..usize::from(u16_at(g, feature + 2)?) {
            indices.push(u16_at(g, feature + 4 + 2 * j)?);
        }
    }
    let lookup_count = u16_at(g, lookups)?;
    for index in indices {
        if index >= lookup_count {
            return Err(malformed("lookup index"));
        }
        let lookup = lookups + usize::from(u16_at(g, lookups + 2 + 2 * usize::from(index))?);
        let kind = u16_at(g, lookup)?;
        for k in 0..usize::from(u16_at(g, lookup + 4)?) {
            let mut sub = lookup + usize::from(u16_at(g, lookup + 6 + 2 * k)?);
            let mut kind = kind;
            if kind == 7 {
                kind = u16_at(g, sub + 2)?;
                let hi = u32::from(u16_at(g, sub + 4)?);
                let lo = u32::from(u16_at(g, sub + 6)?);
                sub += usize::try_from((hi << 16) | lo).map_err(|_| malformed("extension offset"))?;
            }
            let format = u16_at(g, sub)?;
            let coverage = parse_coverage(g, sub + usize::from(u16_at(g, sub + 2)?))?;
            for (c, gid) in coverage.into_iter().enumerate() {
                let substitutes = match (kind, format) {
                    (1, 1) => vec![gid.wrapping_add(u16_at(g, sub + 4)?)],
                    (1, 2) => vec![u16_at(g, sub + 6 + 2 * c)?],
                    (3, 1) => {
                        let set = sub + usize::from(u16_at(g, sub + 6 + 2 * c)?);
                        let n = usize::from(u16_at(g, set)?);
                        if n == 0 {
                            continue;
                        }
                        (0..n).map(|a| u16_at(g, set + 2 + 2 * a)).collect::<Result<Vec<u16>, _>>()?
                    }
                    _ => break,
                };
                out.entry(gid).or_insert(substitutes);
            }
        }
    }
    Ok(out)
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
