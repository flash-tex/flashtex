//! The font-parameter interface the layout engine depends on.
//!
//! TeX takes its math parameters from `\fontdimen`s of the family-2 (symbol)
//! and family-3 (extension) fonts of the current size (TeXbook Appendix G,
//! "the parameters"). [`MathFontMetrics`] abstracts that so the engine can be
//! driven by the FT-018 font engine when it lands, by the Computer Modern TFM
//! tables shipped in [`crate::cm`], or by the Times approximation in
//! [`crate::times`].

use crate::mathlist::TextStyle;

/// Opaque font identity assigned by the metrics provider.
///
/// The provider maps it to a concrete font (a TFM name for the Computer Modern
/// adapter, a content-addressed font handle once the FT-018 font engine is the
/// provider) via [`MathFontMetrics::font_name`]. The renderer must draw glyph
/// `gid` from exactly this font; the engine never invents fonts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontId(pub u32);

/// The three sizes TeX distinguishes: text, script, and scriptscript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SizeClass {
    Text,
    Script,
    ScriptScript,
}

/// A glyph selected for a symbol, with its metrics already scaled to points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glyph {
    pub font_id: FontId,
    /// Glyph identity inside `font_id` (TFM character code for the CM adapter).
    pub gid: u16,
    /// The symbol this glyph renders; kept for text extraction and fallbacks.
    pub ch: char,
    /// Font size in pt at which the metrics below apply.
    pub size: f64,
    pub width: f64,
    pub height: f64,
    pub depth: f64,
    /// Italic correction (TeXbook Appendix G "δ").
    pub italic: f64,
    /// Accent skew: kern with the font's skew character (Rule 12).
    pub skew: f64,
}

impl Glyph {
    pub fn total_height(&self) -> f64 {
        self.height + self.depth
    }
}

/// Math parameters for one size, in points.
///
/// Names follow TeXbook Appendix G. `x_height`..`axis_height` are family-2
/// fontdimens 5..22 (σ₅..σ₂₂); `default_rule_thickness`..`big_op_spacing5`
/// are family-3 fontdimens 8..13 (ξ₈..ξ₁₃). The remaining fields are TeX
/// primitives (`\scriptspace`, `\nulldelimiterspace`, `\delimiterfactor`,
/// `\delimitershortfall`) that plain.tex/LaTeX set to fixed values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MathParams {
    /// Font size of family 2 at this size class, in pt.
    pub size: f64,
    pub x_height: f64,
    pub quad: f64,
    pub num1: f64,
    pub num2: f64,
    pub num3: f64,
    pub denom1: f64,
    pub denom2: f64,
    pub sup1: f64,
    pub sup2: f64,
    pub sup3: f64,
    pub sub1: f64,
    pub sub2: f64,
    pub sup_drop: f64,
    pub sub_drop: f64,
    pub delim1: f64,
    pub delim2: f64,
    pub axis_height: f64,
    pub default_rule_thickness: f64,
    pub big_op_spacing1: f64,
    pub big_op_spacing2: f64,
    pub big_op_spacing3: f64,
    pub big_op_spacing4: f64,
    pub big_op_spacing5: f64,
    pub script_space: f64,
    pub null_delimiter_space: f64,
    /// `\delimiterfactor` as a fraction (plain: 901/1000).
    pub delimiter_factor: f64,
    pub delimiter_shortfall: f64,
}

impl MathParams {
    /// One math unit: 1/18 of the family-2 quad at this size (TeXbook ch. 18).
    ///
    /// TeX computes `cur_mu = x_over_n(math_quad, 18)` in scaled points with
    /// truncation, which is why `\medmuskip` (4mu) is 2.22217pt in a 10pt
    /// document rather than 2.22222pt; the same truncation is applied here.
    pub fn mu(&self) -> f64 {
        let quad_sp = (self.quad * 65536.0).round();
        (quad_sp / 18.0).trunc() / 65536.0
    }
}

/// An extensible (stackable) glyph recipe: `bot`, `rep`×n, `mid`, `rep`×n,
/// `top`, as in TFM extensible recipes and OpenType `MathVariants` vertical
/// assemblies. Pieces are stacked with no gaps.
#[derive(Debug, Clone, PartialEq)]
pub struct Extensible {
    pub top: Option<Glyph>,
    pub mid: Option<Glyph>,
    pub bot: Option<Glyph>,
    pub rep: Glyph,
}

/// Everything the layout engine needs from a font set.
pub trait MathFontMetrics {
    /// Parameters at a size class.
    fn params(&self, size: SizeClass) -> MathParams;

    /// Human-readable name of a font identity (for reports and renderers).
    fn font_name(&self, font: FontId) -> String;

    /// The glyph for a symbol at a size class, or `None` when unsupported.
    fn glyph(&self, ch: char, size: SizeClass) -> Option<Glyph>;

    /// The display-size variant of a large operator (Rule 13), if any.
    fn large_operator(&self, ch: char, size: SizeClass) -> Option<Glyph>;

    /// Delimiter sizes for `ch`, smallest first (Rule 19 / `var_delimiter`).
    fn delimiter_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph>;

    /// Radical sign sizes, smallest first (Rule 11).
    fn radical_sizes(&self, size: SizeClass) -> Vec<Glyph>;

    /// Accent glyph variants, narrowest first (Rule 12).
    fn accent_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph>;

    /// The extensible recipe used once every delimiter size is too small.
    fn delimiter_extensible(&self, _ch: char, _size: SizeClass) -> Option<Extensible> {
        None
    }

    /// The extensible radical-sign recipe.
    fn radical_extensible(&self, _size: SizeClass) -> Option<Extensible> {
        None
    }

    /// A character of upright operator text (`\lim`, `\sin`): the roman
    /// text font at this size. Defaults to [`MathFontMetrics::glyph`].
    fn text_glyph(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        self.glyph(ch, size)
    }

    /// A literal text glyph with the face selected by a mixed text run.
    /// Providers that only expose one upright text family can keep the default.
    fn text_glyph_with_style(
        &self,
        ch: char,
        size: SizeClass,
        _style: TextStyle,
    ) -> Option<Glyph> {
        self.text_glyph(ch, size)
    }

    /// The inter-word space of the text font at this size.
    fn text_space(&self, size: SizeClass) -> f64 {
        self.text_glyph(' ', size).map_or(0.0, |glyph| glyph.width)
    }

    /// Slot `code` of the math extension font (family 3, `largesymbols`) at
    /// size class `size`, for constructions that place its characters
    /// directly rather than through a symbol or delimiter (`fontmath.ltx`'s
    /// `\braceld`..`\braceru` in `\downbracefill`/`\upbracefill`), tagged
    /// `ch` for the renderer. `None` when the provider has no TFM-slotted
    /// extension font.
    ///
    /// `size` matters whenever family 3 is not one fixed font: amsmath and
    /// amsfonts redeclare `OMX/cmex/m/n` without `sfixed`
    /// ([`crate::cm::ExtensionSizing::Designs`]), so `\textfont3` and
    /// `\scriptfont3` are different designs at different sizes (pdfTeX
    /// `\fontname` in an 11 pt article loading amsmath: `cmex10 at 10.95pt`
    /// and `cmex8`). Callers pass the size class the construction sets its
    /// family-3 characters at, which is not always the current style's — see
    /// [`MathFontMetrics::extension_glyph`]'s caller in `make_brace`.
    fn extension_glyph(&self, _code: u8, _ch: char, _size: SizeClass) -> Option<Glyph> {
        None
    }

    /// TeX's `make_ord` (tex.web §752) for an ordinary character `left`
    /// without scripts followed by the character `right` of an Ord..Punct
    /// atom: `None` unless both are in the same math family; otherwise the
    /// kern the family's font program puts between them at `size` and
    /// whether that font is a text font. Providers without lig/kern data
    /// keep the default, which never kerns.
    fn ord_pair(&self, _left: MathChar, _right: MathChar, _size: SizeClass) -> Option<OrdPair> {
        None
    }
}

/// A character nucleus as `make_ord` sees it: a math symbol resolved through
/// its `\mathcode` family ([`Nucleus::Symbol`](crate::Nucleus::Symbol)) or a
/// character of the upright text family
/// ([`Nucleus::TextChar`](crate::Nucleus::TextChar)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathChar {
    Symbol(char),
    Text(char),
}

/// What `make_ord` finds between two adjacent characters of one family.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrdPair {
    /// The font kern appended after the left character, in points; 0 when
    /// the pair has no kern instruction (or a ligature, which is not formed).
    pub kern: f64,
    /// The family's font has a nonzero interword space (fontdimen 2), so
    /// TeX drops the left character's italic correction (§755: "no italic
    /// correction in mid-word of text font"). False for cmmi and cmsy.
    pub text_font: bool,
}

/// The subset of OpenType `MathConstants` (font units) needed to derive TeX's
/// parameters. Field names follow the OpenType specification so a font
/// engine that parses the `MATH` table (FT-018) can fill this directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OpenTypeMathConstants {
    pub units_per_em: u16,
    pub axis_height: i16,
    pub fraction_numerator_display_style_shift_up: i16,
    pub fraction_numerator_shift_up: i16,
    pub stack_top_shift_up: i16,
    pub fraction_denominator_display_style_shift_down: i16,
    pub fraction_denominator_shift_down: i16,
    pub superscript_shift_up: i16,
    pub superscript_shift_up_cramped: i16,
    pub subscript_shift_down: i16,
    pub superscript_baseline_drop_max: i16,
    pub subscript_baseline_drop_min: i16,
    pub fraction_rule_thickness: i16,
    pub upper_limit_gap_min: i16,
    pub lower_limit_gap_min: i16,
    pub upper_limit_baseline_rise_min: i16,
    pub lower_limit_baseline_drop_min: i16,
    pub delimited_sub_formula_min_height: u16,
}

impl MathParams {
    /// Derives TeX's parameters from OpenType `MATH` constants at `size` pt,
    /// using the correspondence LuaTeX documents in its manual ("Math
    /// parameters", the OpenType-to-TeX table): axis_height ← AxisHeight;
    /// num1/num2/num3 ← FractionNumeratorDisplayStyleShiftUp /
    /// FractionNumeratorShiftUp / StackTopShiftUp; denom1/denom2 ←
    /// FractionDenominatorDisplayStyleShiftDown / FractionDenominatorShiftDown;
    /// sup1 = sup2 ← SuperscriptShiftUp, sup3 ← SuperscriptShiftUpCramped;
    /// sub1 = sub2 ← SubscriptShiftDown; sup_drop ← SuperscriptBaselineDropMax;
    /// sub_drop ← SubscriptBaselineDropMin; default_rule_thickness ←
    /// FractionRuleThickness; big_op_spacing1..4 ← UpperLimitGapMin,
    /// LowerLimitGapMin, UpperLimitBaselineRiseMin, LowerLimitBaselineDropMin;
    /// big_op_spacing5 = 0; delim1 = delim2 ← DelimitedSubFormulaMinHeight.
    /// `x_height` and `quad` come from the text font, in font units. Fixed
    /// registers keep the plain.tex values.
    pub fn from_opentype(
        c: &OpenTypeMathConstants,
        x_height_units: i16,
        quad_units: u16,
        size: f64,
    ) -> MathParams {
        let upem = f64::from(c.units_per_em.max(1));
        let u = |v: i16| f64::from(v) * size / upem;
        let delim = f64::from(c.delimited_sub_formula_min_height) * size / upem;
        MathParams {
            size,
            x_height: u(x_height_units),
            quad: f64::from(quad_units) * size / upem,
            num1: u(c.fraction_numerator_display_style_shift_up),
            num2: u(c.fraction_numerator_shift_up),
            num3: u(c.stack_top_shift_up),
            denom1: u(c.fraction_denominator_display_style_shift_down),
            denom2: u(c.fraction_denominator_shift_down),
            sup1: u(c.superscript_shift_up),
            sup2: u(c.superscript_shift_up),
            sup3: u(c.superscript_shift_up_cramped),
            sub1: u(c.subscript_shift_down),
            sub2: u(c.subscript_shift_down),
            sup_drop: u(c.superscript_baseline_drop_max),
            sub_drop: u(c.subscript_baseline_drop_min),
            delim1: delim,
            delim2: delim,
            axis_height: u(c.axis_height),
            default_rule_thickness: u(c.fraction_rule_thickness),
            big_op_spacing1: u(c.upper_limit_gap_min),
            big_op_spacing2: u(c.lower_limit_gap_min),
            big_op_spacing3: u(c.upper_limit_baseline_rise_min),
            big_op_spacing4: u(c.lower_limit_baseline_drop_min),
            big_op_spacing5: 0.0,
            script_space: 0.5,
            null_delimiter_space: 1.2,
            delimiter_factor: 0.901,
            delimiter_shortfall: 5.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opentype_constants_map_to_tex_parameters() {
        // Illustrative constants in a 1000-upem font chosen to equal Computer
        // Modern's fontdimens (the values Latin Modern Math was designed to
        // reproduce); the test checks the mapping, not any particular font.
        let c = OpenTypeMathConstants {
            units_per_em: 1000,
            axis_height: 250,
            fraction_numerator_display_style_shift_up: 677,
            fraction_numerator_shift_up: 394,
            stack_top_shift_up: 444,
            fraction_denominator_display_style_shift_down: 686,
            fraction_denominator_shift_down: 345,
            superscript_shift_up: 363,
            superscript_shift_up_cramped: 289,
            subscript_shift_down: 247,
            superscript_baseline_drop_max: 386,
            subscript_baseline_drop_min: 50,
            fraction_rule_thickness: 40,
            upper_limit_gap_min: 111,
            lower_limit_gap_min: 167,
            upper_limit_baseline_rise_min: 200,
            lower_limit_baseline_drop_min: 600,
            delimited_sub_formula_min_height: 1300,
        };
        let p = MathParams::from_opentype(&c, 431, 1000, 10.0);
        assert!((p.axis_height - 2.5).abs() < 1e-9);
        assert!((p.num1 - 6.77).abs() < 1e-9);
        assert!((p.sup3 - 2.89).abs() < 1e-9);
        assert!((p.default_rule_thickness - 0.4).abs() < 1e-9);
        assert!((p.big_op_spacing4 - 6.0).abs() < 1e-9);
        assert_eq!(p.big_op_spacing5, 0.0);
        assert!((p.mu() - 10.0 / 18.0).abs() < 1e-4);
    }
}
