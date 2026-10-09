//! OpenType math, ported from XeTeX's `XeTeXOTMath.cpp` (MIT; TeX Live
//! 2026, notices in this crate's `LICENSE`): the `MATH` table's constants
//! as fontdimens, size variants, glyph assemblies, italic corrections, top
//! accent positions and cut-in kerns, read through the vendored HarfBuzz's
//! math API exactly as XeTeX reads them (docs/design/xetex/PLAN.md, S2).
//!
//! Arithmetic follows the C++: `unitsToPoints`/`pointsToUnits` in `float`,
//! C's truncating `float`-to-`int` conversions, and `D2Fix`. A glyph
//! assembly (`GlyphAssembly*` in C) is a handle into the engine's handle
//! table (`changes/ext.ch`), freed by `free_ot_assembly`.

use super::font_inst::{d2fix, fix2d, FontInst};
use crate::fontlibs::hb;
use crate::generated::Globals;
use crate::state::Object;
use std::sync::Arc;

/// `GlyphAssembly`: the parts of a glyph's assembly in one direction.
pub struct Assembly {
    parts: Vec<hb::hb_ot_math_glyph_part_t>,
}

/// `hb_ot_math_constant_t`'s `-1`: no OpenType constant for a fontdimen.
const UNKNOWN: i32 = -1;

// The fontdimens of xetex.web's math symbols font (family 2).
const MATH_QUAD: i32 = 6;
const DELIM1: i32 = 20;
const DELIM2: i32 = 21;

/// `TeX_sym_to_OT_map`: family 2's fontdimens to `MATH` constants.
const TEX_SYM_TO_OT: [i32; 23] = [
    UNKNOWN,
    UNKNOWN,
    UNKNOWN,
    UNKNOWN,
    UNKNOWN,
    hb::HB_OT_MATH_CONSTANT_ACCENT_BASE_HEIGHT as i32, // x-height
    UNKNOWN,                                           // quad
    UNKNOWN,
    hb::HB_OT_MATH_CONSTANT_FRACTION_NUMERATOR_DISPLAY_STYLE_SHIFT_UP as i32,
    hb::HB_OT_MATH_CONSTANT_FRACTION_NUMERATOR_SHIFT_UP as i32,
    hb::HB_OT_MATH_CONSTANT_STACK_TOP_SHIFT_UP as i32,
    hb::HB_OT_MATH_CONSTANT_FRACTION_DENOMINATOR_DISPLAY_STYLE_SHIFT_DOWN as i32,
    hb::HB_OT_MATH_CONSTANT_FRACTION_DENOMINATOR_SHIFT_DOWN as i32,
    hb::HB_OT_MATH_CONSTANT_SUPERSCRIPT_SHIFT_UP as i32,
    hb::HB_OT_MATH_CONSTANT_SUPERSCRIPT_SHIFT_UP as i32,
    hb::HB_OT_MATH_CONSTANT_SUPERSCRIPT_SHIFT_UP_CRAMPED as i32,
    hb::HB_OT_MATH_CONSTANT_SUBSCRIPT_SHIFT_DOWN as i32,
    hb::HB_OT_MATH_CONSTANT_SUBSCRIPT_SHIFT_DOWN as i32,
    hb::HB_OT_MATH_CONSTANT_SUPERSCRIPT_BASELINE_DROP_MAX as i32,
    hb::HB_OT_MATH_CONSTANT_SUBSCRIPT_BASELINE_DROP_MIN as i32,
    hb::HB_OT_MATH_CONSTANT_DELIMITED_SUB_FORMULA_MIN_HEIGHT as i32,
    UNKNOWN, // delim2: 1.5em, clamped to delim1
    hb::HB_OT_MATH_CONSTANT_AXIS_HEIGHT as i32,
];

/// `TeX_ext_to_OT_map`: family 3's fontdimens to `MATH` constants.
const TEX_EXT_TO_OT: [i32; 14] = [
    UNKNOWN,
    UNKNOWN,
    UNKNOWN,
    UNKNOWN,
    UNKNOWN,
    hb::HB_OT_MATH_CONSTANT_ACCENT_BASE_HEIGHT as i32, // x-height
    UNKNOWN,                                           // quad
    UNKNOWN,
    hb::HB_OT_MATH_CONSTANT_FRACTION_RULE_THICKNESS as i32, // default_rule_thickness
    hb::HB_OT_MATH_CONSTANT_UPPER_LIMIT_GAP_MIN as i32,     // big_op_spacing1
    hb::HB_OT_MATH_CONSTANT_LOWER_LIMIT_GAP_MIN as i32,     // big_op_spacing2
    hb::HB_OT_MATH_CONSTANT_UPPER_LIMIT_BASELINE_RISE_MIN as i32, // big_op_spacing3
    hb::HB_OT_MATH_CONSTANT_LOWER_LIMIT_BASELINE_DROP_MIN as i32, // big_op_spacing4
    hb::HB_OT_MATH_CONSTANT_STACK_GAP_MIN as i32,           // big_op_spacing5
];

// xetex.web's `sup_cmd` and `sub_cmd`.
const SUP_CMD: i32 = 0;
const SUB_CMD: i32 = 1;

/// The HarfBuzz direction XeTeX asks in: `RTL` for horizontal (any
/// horizontal direction selects the horizontal tables), `TTB` for vertical.
fn direction(horiz: bool) -> hb::hb_direction_t {
    if horiz {
        hb::HB_DIRECTION_RTL
    } else {
        hb::HB_DIRECTION_TTB
    }
}

/// `D2Fix(font->unitsToPoints(units))`.
fn units_fix(font: &FontInst, units: i32) -> i32 {
    d2fix(font.units_to_points(units as f32) as f64)
}

/// `pointsToUnits`, in C's `float`.
fn points_to_units(font: &FontInst, points: f32) -> f32 {
    (points * font.units_per_em as f32) / font.point_size
}

/// `getMathKernAt`: the cut-in kern at `height` (font units).
fn math_kern_at(font: &FontInst, g: i32, height: i32, side: hb::hb_ot_math_kern_t) -> i32 {
    // SAFETY: a live HarfBuzz font.
    unsafe { hb::hb_ot_math_get_glyph_kerning(font.hb_font, g as u32, side, height) }
}

impl Globals {
    /// `get_ot_math_constant`: the `MATH` constant `n`, scaled to the font
    /// size except for the three percentages; 0 for a font that is not an
    /// OpenType font.
    pub fn get_ot_math_constant(&mut self, f: i32, n: i32) -> i32 {
        let Some(e) = self.font_engine(f) else {
            return 0;
        };
        // SAFETY: a live HarfBuzz font.
        let rval = unsafe { hb::hb_ot_math_get_constant(e.font.hb_font, n as u32) };
        match n as u32 {
            hb::HB_OT_MATH_CONSTANT_SCRIPT_PERCENT_SCALE_DOWN
            | hb::HB_OT_MATH_CONSTANT_SCRIPT_SCRIPT_PERCENT_SCALE_DOWN
            | hb::HB_OT_MATH_CONSTANT_RADICAL_DEGREE_BOTTOM_RAISE_PERCENT => rval,
            _ => units_fix(&e.font, rval),
        }
    }

    /// `get_native_mathsy_param`: fontdimen `n` of an OpenType font used
    /// as family 2.
    pub fn get_native_mathsy_param(&mut self, f: i32, n: i32) -> i32 {
        if n == MATH_QUAD {
            self.font_size[f as usize]
        } else if n == DELIM2 {
            // `std::min<int>(1.5 * fontsize[f], ...)`: the double is
            // truncated to int.
            let em = (1.5 * self.font_size[f as usize] as f64) as i32;
            em.min(self.get_native_mathsy_param(f, DELIM1))
        } else {
            match usize::try_from(n).ok().and_then(|i| TEX_SYM_TO_OT.get(i)) {
                Some(&c) if c != UNKNOWN => self.get_ot_math_constant(f, c),
                _ => 0,
            }
        }
    }

    /// `get_native_mathex_param`: fontdimen `n` of an OpenType font used
    /// as family 3.
    pub fn get_native_mathex_param(&mut self, f: i32, n: i32) -> i32 {
        if n == MATH_QUAD {
            self.font_size[f as usize]
        } else {
            match usize::try_from(n).ok().and_then(|i| TEX_EXT_TO_OT.get(i)) {
                Some(&c) if c != UNKNOWN => self.get_ot_math_constant(f, c),
                _ => 0,
            }
        }
    }

    /// `get_ot_math_variant`: size variant `v` of glyph `g` (the glyph
    /// itself when there is none), its advance in `adv` (-1 when none).
    pub fn get_ot_math_variant(
        &mut self,
        f: i32,
        g: i32,
        v: i32,
        adv: &mut i32,
        horiz: i32,
    ) -> i32 {
        *adv = -1;
        let Some(e) = self.font_engine(f) else {
            return g;
        };
        let mut variant = hb::hb_ot_math_glyph_variant_t::default();
        let mut count: u32 = 1;
        // SAFETY: a live HarfBuzz font; room for one variant.
        unsafe {
            hb::hb_ot_math_get_glyph_variants(
                e.font.hb_font,
                g as u32,
                direction(horiz != 0),
                v as u32,
                &mut count,
                &mut variant,
            );
        }
        if count > 0 {
            *adv = units_fix(&e.font, variant.advance);
            variant.glyph as i32
        } else {
            g
        }
    }

    /// `get_ot_assembly_ptr`: glyph `g`'s assembly, as a handle (0 if it
    /// has none).
    pub fn get_ot_assembly_ptr(&mut self, f: i32, g: i32, horiz: i32) -> i32 {
        let Some(e) = self.font_engine(f) else {
            return 0;
        };
        let dir = direction(horiz != 0);
        let font = e.font.hb_font;
        // SAFETY: a live HarfBuzz font; the second call writes at most
        // `count` parts into a buffer of that size.
        let parts = unsafe {
            let count = hb::hb_ot_math_get_glyph_assembly(
                font,
                g as u32,
                dir,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            if count == 0 {
                return 0;
            }
            let mut parts = vec![hb::hb_ot_math_glyph_part_t::default(); count as usize];
            let mut n = count;
            hb::hb_ot_math_get_glyph_assembly(
                font,
                g as u32,
                dir,
                0,
                &mut n,
                parts.as_mut_ptr(),
                std::ptr::null_mut(),
            );
            parts.truncate(n as usize);
            parts
        };
        self.host
            .handles
            .alloc(Object::Other(Arc::new(Assembly { parts })))
    }

    /// `free_ot_assembly`.
    pub fn free_ot_assembly(&mut self, a: i32) {
        self.host.handles.free(a);
    }

    /// The assembly of handle `a`. xetex.web asks only for the parts of an
    /// assembly it holds, so a bad handle is an internal error.
    fn assembly(&self, a: i32) -> Arc<Assembly> {
        if let Some(Object::Other(o)) = self.host.handles.get(a) {
            if let Ok(asm) = o.clone().downcast::<Assembly>() {
                return asm;
            }
        }
        eprintln!("\n! Internal error: bad OpenType assembly handle {a}");
        std::process::exit(3);
    }

    /// `get_ot_math_ital_corr`.
    pub fn get_ot_math_ital_corr(&mut self, f: i32, g: i32) -> i32 {
        let Some(e) = self.font_engine(f) else {
            return 0;
        };
        // SAFETY: a live HarfBuzz font.
        let r = unsafe { hb::hb_ot_math_get_glyph_italics_correction(e.font.hb_font, g as u32) };
        units_fix(&e.font, r)
    }

    /// `get_ot_math_accent_pos`: the top accent attachment (0x7fffffff for
    /// a font that is not an OpenType font).
    pub fn get_ot_math_accent_pos(&mut self, f: i32, g: i32) -> i32 {
        let Some(e) = self.font_engine(f) else {
            return 0x7fff_ffff;
        };
        // SAFETY: a live HarfBuzz font.
        let r = unsafe { hb::hb_ot_math_get_glyph_top_accent_attachment(e.font.hb_font, g as u32) };
        units_fix(&e.font, r)
    }

    /// `ot_min_connector_overlap`.
    pub fn ot_min_connector_overlap(&mut self, f: i32) -> i32 {
        let Some(e) = self.font_engine(f) else {
            return 0;
        };
        // SAFETY: a live HarfBuzz font.
        let r = unsafe {
            hb::hb_ot_math_get_min_connector_overlap(e.font.hb_font, hb::HB_DIRECTION_RTL)
        };
        units_fix(&e.font, r)
    }

    /// `get_ot_math_kern`: the cut-in kern between base glyph `g` of font
    /// `f` and the first glyph `sg` of a sub- or superscript in font `sf`,
    /// shifted by `shift_scaled`.
    pub fn get_ot_math_kern(
        &mut self,
        f: i32,
        g: i32,
        sf: i32,
        sg: i32,
        cmd: i32,
        shift_scaled: i32,
    ) -> i32 {
        let (Some(e), Some(se)) = (self.font_engine(f), self.font_engine(sf)) else {
            return 0;
        };
        let (font, sfont) = (&e.font, &se.font);
        // `glyph_height`/`glyph_depth` in points, then in font units,
        // truncated to int as C assigns a float to an int.
        let (h, d) = font.glyph_height_depth(g as u32);
        let (sh, sd) = sfont.glyph_height_depth(sg as u32);
        let g_height = points_to_units(font, h) as i32;
        let g_depth = points_to_units(font, d) as i32;
        let sg_height = points_to_units(sfont, sh) as i32;
        let sg_depth = points_to_units(sfont, sd) as i32;
        // `Fix2D` is a double, `pointsToUnits` takes a float.
        let shift = points_to_units(font, fix2d(shift_scaled) as f32) as i32;
        let scale = sfont.point_size / font.point_size;
        let (base_side, script_side) = match cmd {
            SUP_CMD => (
                hb::HB_OT_MATH_KERN_TOP_RIGHT,
                hb::HB_OT_MATH_KERN_BOTTOM_LEFT,
            ),
            SUB_CMD => (
                hb::HB_OT_MATH_KERN_BOTTOM_RIGHT,
                hb::HB_OT_MATH_KERN_TOP_LEFT,
            ),
            _ => {
                eprintln!("\n! Internal error: get_ot_math_kern with cmd {cmd}");
                std::process::exit(3);
            }
        };
        // The two heights at which the kerns are summed, in base glyph
        // units (for the base) and script glyph units (for the script):
        // C computes each in float and truncates it to int.
        let (k1_base, k1_script, k2_base, k2_script) = if cmd == SUP_CMD {
            (
                (shift as f32 - scale * sg_depth as f32) as i32,
                -sg_depth,
                g_height,
                ((g_height - shift) as f32 / scale) as i32,
            )
        } else {
            (
                (scale * sg_height as f32 - shift as f32) as i32,
                sg_height,
                -g_depth,
                ((shift - g_depth) as f32 / scale) as i32,
            )
        };
        let kern = math_kern_at(font, g, k1_base, base_side);
        let skern = math_kern_at(sfont, sg, k1_script, script_side);
        let top_kern = (kern as f32 + scale * skern as f32) as i32;
        let kern = math_kern_at(font, g, k2_base, base_side);
        let skern = math_kern_at(sfont, sg, k2_script, script_side);
        let bot_kern = (kern as f32 + scale * skern as f32) as i32;
        units_fix(font, top_kern.max(bot_kern))
    }

    /// `ot_part_count`.
    pub fn ot_part_count(&mut self, a: i32) -> i32 {
        self.assembly(a).parts.len() as i32
    }

    /// `ot_part_glyph`.
    pub fn ot_part_glyph(&mut self, a: i32, i: i32) -> i32 {
        self.assembly(a).parts[i as usize].glyph as i32
    }

    /// `ot_part_is_extender`.
    pub fn ot_part_is_extender(&mut self, a: i32, i: i32) -> bool {
        self.assembly(a).parts[i as usize].flags & hb::HB_OT_MATH_GLYPH_PART_FLAG_EXTENDER != 0
    }

    /// The length `part` reads from part `i` of assembly `a`, in scaled
    /// points of font `f` (0 for a font that is not an OpenType font).
    fn part_length(
        &self,
        f: i32,
        a: i32,
        i: i32,
        part: fn(&hb::hb_ot_math_glyph_part_t) -> i32,
    ) -> i32 {
        let Some(e) = self.font_engine(f) else {
            return 0;
        };
        units_fix(&e.font, part(&self.assembly(a).parts[i as usize]))
    }

    /// `ot_part_start_connector`.
    pub fn ot_part_start_connector(&mut self, f: i32, a: i32, i: i32) -> i32 {
        self.part_length(f, a, i, |p| p.start_connector_length)
    }

    /// `ot_part_end_connector`.
    pub fn ot_part_end_connector(&mut self, f: i32, a: i32, i: i32) -> i32 {
        self.part_length(f, a, i, |p| p.end_connector_length)
    }

    /// `ot_part_full_advance`.
    pub fn ot_part_full_advance(&mut self, f: i32, a: i32, i: i32) -> i32 {
        self.part_length(f, a, i, |p| p.full_advance)
    }
}
