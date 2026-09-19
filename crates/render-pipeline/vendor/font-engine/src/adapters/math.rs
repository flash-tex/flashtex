//! `flashtex_math_layout` adapter: `MathFontMetrics` driven by an OpenType
//! `MATH` face (Latin Modern Math) through this crate's parser.
//!
//! What is exact: every `MathParams` value (`MathParams::from_opentype` from
//! the parsed `MathConstants`), glyph widths (`hmtx`), italics corrections
//! (`MathGlyphInfo`), glyph ids, and the script/scriptscript scale factors
//! (`scriptPercentScaleDown` / `scriptScriptPercentScaleDown`).
//!
//! What is declared approximate or absent:
//! * **height/depth** of a glyph: exact from the `glyf` bounding box for
//!   TrueType faces; for CFF faces (Latin Modern Math is CFF) charstrings are
//!   not parsed, so height/depth are estimated from the face's cap height,
//!   x-height, ascender and descender by character class — the same kind of
//!   approximation `math_layout::times` uses, flagged by
//!   [`OpenTypeMathFace::heights_are_approximate`].
//! * `large_operator`, `delimiter_sizes`, `radical_sizes`, `accent_sizes`:
//!   `MathVariants` is not parsed yet, so only the base glyph is offered (one
//!   size); stretchy construction is a follow-up and is not faked.
//! * `skew` (accent skew kern) is 0: OpenType fonts express it through
//!   `MathTopAccentAttachment`, which the layout crate does not consume yet.

use flashtex_math_layout::metrics::{
    FontId, Glyph, MathFontMetrics, MathParams, OpenTypeMathConstants, SizeClass,
};

use crate::math::MathConstants;
use crate::truetype::{Outlines, TrueTypeFace};
use crate::{Face, GlyphId};

/// A `MATH` face plus the text-size it is used at.
pub struct OpenTypeMathFace<'a> {
    pub face: &'a TrueTypeFace,
    /// Text size in points; script sizes derive from the MATH percentages.
    pub text_size_pt: f64,
    /// The opaque id handed to the layout crate for this face.
    pub font_id: FontId,
}

impl<'a> OpenTypeMathFace<'a> {
    /// Fails (returns `None`) when the face has no `MATH` table.
    pub fn new(face: &'a TrueTypeFace, text_size_pt: f64, font_id: FontId) -> Option<Self> {
        face.math()?;
        Some(OpenTypeMathFace {
            face,
            text_size_pt,
            font_id,
        })
    }

    fn constants(&self) -> &MathConstants {
        &self.face.math().expect("checked in new").constants
    }

    pub fn size_pt(&self, size: SizeClass) -> f64 {
        let c = self.constants();
        let pct = match size {
            SizeClass::Text => 100,
            SizeClass::Script => c.script_percent_scale_down,
            SizeClass::ScriptScript => c.script_script_percent_scale_down,
        };
        self.text_size_pt * f64::from(pct.max(1)) / 100.0
    }

    pub fn opentype_constants(&self) -> OpenTypeMathConstants {
        let c = self.constants();
        OpenTypeMathConstants {
            units_per_em: self.face.units_per_em(),
            axis_height: c.axis_height,
            fraction_numerator_display_style_shift_up: c.fraction_numerator_display_style_shift_up,
            fraction_numerator_shift_up: c.fraction_numerator_shift_up,
            stack_top_shift_up: c.stack_top_shift_up,
            fraction_denominator_display_style_shift_down: c
                .fraction_denominator_display_style_shift_down,
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
            underbar_extra_descender: c.underbar_extra_ascender,
            display_operator_min_height: c.display_operator_min_height,
        }
    }

    /// True for CFF faces, whose glyph heights/depths are estimated.
    pub fn heights_are_approximate(&self) -> bool {
        self.face.outlines() == Outlines::Cff
    }

    /// (height, depth) in font units, exact for glyf, estimated for CFF.
    fn extents_units(&self, gid: GlyphId, ch: char) -> (f64, f64) {
        if self.face.outlines() == Outlines::Glyf
            && let Ok(data) = self.face.glyph_data(gid)
            && data.len() >= 10
        {
            let y_min = i16::from_be_bytes([data[4], data[5]]);
            let y_max = i16::from_be_bytes([data[8], data[9]]);
            return (f64::from(y_max.max(0)), f64::from((-y_min).max(0)));
        }
        let vm = self.face.vertical_metrics();
        let cap = f64::from(vm.cap_height);
        let xh = f64::from(vm.x_height);
        let asc = f64::from(vm.ascender);
        let desc = f64::from(-vm.descender);
        let height = if ch.is_uppercase() || ch.is_ascii_digit() {
            cap
        } else if ch.is_lowercase() {
            if matches!(ch, 'b' | 'd' | 'f' | 'h' | 'k' | 'l' | 't' | 'i' | 'j') {
                asc
            } else {
                xh
            }
        } else {
            match ch {
                '+' | '-' | '=' | '<' | '>' | '\u{2212}' | '\u{00B1}' | '\u{00D7}' | '\u{2264}'
                | '\u{2265}' => f64::from(self.constants().axis_height) * 2.0,
                ',' | '.' => xh * 0.25,
                _ => asc,
            }
        };
        let depth = if matches!(
            ch,
            'g' | 'j' | 'p' | 'q' | 'y' | ',' | '(' | ')' | '[' | ']' | 'Q' | 'J'
        ) || matches!(ch, '\u{2211}' | '\u{220F}' | '\u{222B}')
        {
            desc
        } else {
            0.0
        };
        (height, depth)
    }

    fn glyph_at(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        let gid = self.face.glyph_id(ch)?;
        let at = self.size_pt(size);
        let scale = at / f64::from(self.face.units_per_em());
        let width = f64::from(self.face.advance(gid).ok()?) * scale;
        let italic = f64::from(self.face.math()?.italics_correction(gid)) * scale;
        let (h, d) = self.extents_units(gid, ch);
        Some(Glyph {
            font_id: self.font_id,
            gid: gid.0,
            ch,
            size: at,
            width,
            height: h * scale,
            depth: d * scale,
            italic,
            skew: 0.0,
        })
    }
}

impl MathFontMetrics for OpenTypeMathFace<'_> {
    fn params(&self, size: SizeClass) -> MathParams {
        let vm = self.face.vertical_metrics();
        MathParams::from_opentype(
            &self.opentype_constants(),
            vm.x_height,
            self.face.units_per_em(),
            self.size_pt(size),
        )
    }

    fn font_name(&self, _font: FontId) -> String {
        self.face.postscript_name().to_string()
    }

    fn glyph(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        self.glyph_at(ch, size)
    }

    fn large_operator(&self, _ch: char, _size: SizeClass) -> Option<Glyph> {
        // MathVariants not parsed: no display-size variant is offered.
        None
    }

    fn delimiter_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        self.glyph_at(ch, size).into_iter().collect()
    }

    fn radical_sizes(&self, size: SizeClass) -> Vec<Glyph> {
        self.glyph_at('\u{221A}', size).into_iter().collect()
    }

    fn accent_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        self.glyph_at(ch, size).into_iter().collect()
    }
}
