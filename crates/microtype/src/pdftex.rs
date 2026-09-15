//! pdfTeX character protrusion and font expansion primitives.
//!
//! Every function here is a transcription of pdfTeX 1.40.27 `pdftex.web`
//! (TeX Live trunk `texk/web2c/pdftexdir/pdftex.web`); the section or
//! procedure name is given on each item. All lengths are scaled points.
//!
//! Only **autoexpand** fonts are modelled (microtype's default with any
//! pdfTeX >= 1.20 in PDF mode, `microtype-pdftex.def` line 1449): expanded
//! instances share the base font's parameters (`auto_expand_font` copies
//! `param_base`), so `\fontdimen6` — and therefore every protrusion amount —
//! is identical in all expanded instances, and all char widths / kerns are
//! `round_xn_over_d(x, 1000+e, 1000)` of the base values.

use crate::arith::{Scaled, divide_scaled, ext_xn_over_d, fix_int, round_xn_over_d};

/// `\pdffontexpand <font> <stretch> <shrink> <step> autoexpand`, after
/// pdfTeX's `read_expand_font` normalisation (limits clipped to 0..1000 /
/// 0..500, rounded down to a multiple of `step`, step clipped to 1..100).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpansionLimits {
    /// Maximum stretch in thousandths (`pdf_font_expand_ratio[pdf_font_stretch[f]]`); 0 = none.
    pub stretch: i32,
    /// Maximum shrink in thousandths, positive; 0 = none.
    pub shrink: i32,
    /// Expansion step in thousandths.
    pub step: i32,
}

impl ExpansionLimits {
    /// Normalise raw `\pdffontexpand` arguments exactly as `read_expand_font` does.
    /// Returns `None` where pdfTeX raises an error (step 0, both limits 0).
    pub fn from_primitive(stretch: i32, shrink: i32, step: i32) -> Option<Self> {
        let stretch = fix_int(stretch, 0, 1000);
        let shrink = fix_int(shrink, 0, 500);
        let step = fix_int(step, 0, 100);
        if step == 0 {
            return None;
        }
        let stretch = (stretch - stretch % step).max(0);
        let shrink = (shrink - shrink % step).max(0);
        if stretch == 0 && shrink == 0 {
            return None;
        }
        Some(ExpansionLimits { stretch, shrink, step })
    }
}

/// The per-font state pdfTeX consults: `\fontdimen6` (quad), `\lpcode`,
/// `\rpcode`, `\efcode` and the `\pdffontexpand` limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontParams {
    /// `quad(f)` = `\fontdimen6`, the protrusion unit (`char_pw`).
    pub quad: Scaled,
    /// `\lpcode`, already clipped to −1000..1000 (`get_lp_code`).
    pub lpcode: [i16; 256],
    /// `\rpcode`, already clipped to −1000..1000.
    pub rpcode: [i16; 256],
    /// `\efcode`, already clipped to 0..1000; pdfTeX's default is 1000 (`init_font_base(1000)`).
    pub efcode: [i16; 256],
    /// `None` when `\pdffontexpand` was never applied (`pdf_font_step[f] = 0`).
    pub expansion: Option<ExpansionLimits>,
}

impl FontParams {
    /// A font pdfTeX has never been told about: no protrusion, `\efcode` 1000, not expandable.
    pub fn plain(quad: Scaled) -> Self {
        FontParams {
            quad,
            lpcode: [0; 256],
            rpcode: [0; 256],
            efcode: [1000; 256],
            expansion: None,
        }
    }

    /// `\lpcode<font><c>=<v>` (value clipped like the primitive).
    pub fn set_lpcode(&mut self, c: u8, v: i32) {
        self.lpcode[c as usize] = fix_int(v, -1000, 1000) as i16;
    }
    /// `\rpcode<font><c>=<v>`.
    pub fn set_rpcode(&mut self, c: u8, v: i32) {
        self.rpcode[c as usize] = fix_int(v, -1000, 1000) as i16;
    }
    /// `\efcode<font><c>=<v>`.
    pub fn set_efcode(&mut self, c: u8, v: i32) {
        self.efcode[c as usize] = fix_int(v, 0, 1000) as i16;
    }

    /// `char_pw(p, left_side)`: how far `c` may protrude into the left margin.
    /// The margin kern pdfTeX inserts is the negation of this.
    pub fn left_protrusion(&self, c: u8) -> Scaled {
        pw(self.quad, self.lpcode[c as usize] as i32)
    }

    /// `char_pw(p, right_side)`.
    pub fn right_protrusion(&self, c: u8) -> Scaled {
        pw(self.quad, self.rpcode[c as usize] as i32)
    }

    /// `check_expand_pars(f)` is true (the font takes part in expansion).
    pub fn is_expandable(&self) -> bool {
        matches!(self.expansion, Some(l) if l.stretch > 0 || l.shrink > 0)
    }

    /// `char_stretch(f, c)`: extra width `c` gains at the maximum stretch,
    /// weighted by `\efcode`. `width` is `char_width(f)(c)` in the base font.
    pub fn char_stretch(&self, c: u8, width: Scaled) -> Scaled {
        let (Some(l), ef) = (self.expansion, self.efcode[c as usize] as i32) else {
            return 0;
        };
        if l.stretch == 0 || ef <= 0 {
            return 0;
        }
        let dw = expanded_width(width, l.stretch) - width;
        if dw > 0 { round_xn_over_d(dw, ef, 1000) } else { 0 }
    }

    /// `char_shrink(f, c)`.
    pub fn char_shrink(&self, c: u8, width: Scaled) -> Scaled {
        let (Some(l), ef) = (self.expansion, self.efcode[c as usize] as i32) else {
            return 0;
        };
        if l.shrink == 0 || ef <= 0 {
            return 0;
        }
        let dw = width - expanded_width(width, -l.shrink);
        if dw > 0 { round_xn_over_d(dw, ef, 1000) } else { 0 }
    }

    /// `kern_stretch(p)` for a font kern (`subtype normal`) of width `kern`
    /// sitting directly between two characters of this font, the left one
    /// being `left`. pdfTeX looks the pair up again in the maximally
    /// stretched font (`get_kern`); in an autoexpand font that kern is
    /// `round_xn_over_d(kern, 1000+stretch, 1000)`. Callers must return 0
    /// themselves when the neighbours are not both characters of this font.
    pub fn kern_stretch(&self, left: u8, kern: Scaled) -> Scaled {
        let Some(l) = self.expansion else { return 0 };
        if l.stretch == 0 {
            return 0;
        }
        let d = expanded_width(kern, l.stretch);
        round_xn_over_d(d - kern, self.efcode[left as usize] as i32, 1000)
    }

    /// `kern_shrink(p)`.
    pub fn kern_shrink(&self, left: u8, kern: Scaled) -> Scaled {
        let Some(l) = self.expansion else { return 0 };
        if l.shrink == 0 {
            return 0;
        }
        let d = expanded_width(kern, -l.shrink);
        round_xn_over_d(kern - d, self.efcode[left as usize] as i32, 1000)
    }

    /// `do_subst_font(p, ex_ratio)`: the expansion (thousandths, signed) that
    /// character `c` receives on a line whose `font_expand_ratio` is `ratio`
    /// (−1000..1000). 0 means the base font is kept.
    pub fn char_expansion(&self, c: u8, ratio: i32) -> i32 {
        let ef = self.efcode[c as usize] as i32;
        let Some(l) = self.expansion else { return 0 };
        if ef == 0 {
            return 0;
        }
        if l.stretch > 0 && ratio > 0 {
            fix_expand_value(l, ext_xn_over_d(ratio * ef, l.stretch, 1_000_000))
        } else if l.shrink > 0 && ratio < 0 {
            fix_expand_value(l, ext_xn_over_d(ratio * ef, l.shrink, 1_000_000))
        } else {
            0
        }
    }
}

/// `round_xn_over_d(quad(f), code, 1000)` (`char_pw`).
fn pw(quad: Scaled, code: i32) -> Scaled {
    if code == 0 {
        return 0;
    }
    // round_xn_over_d takes a non-negative n; pdfTeX passes the signed code,
    // and the web routine's arithmetic is sign-symmetric in n.
    if code < 0 {
        -round_xn_over_d(quad, -code, 1000)
    } else {
        round_xn_over_d(quad, code, 1000)
    }
}

/// Width of a base-font dimension `w` in the instance expanded by `e`
/// thousandths (`auto_expand_font`: `round_xn_over_d(w, 1000+e, 1000)`).
pub fn expanded_width(w: Scaled, e: i32) -> Scaled {
    if e == 0 { w } else { round_xn_over_d(w, 1000 + e, 1000) }
}

/// `fix_expand_value(f, e)`: clamp to the font's limit, otherwise round to
/// the nearest multiple of `step`.
pub fn fix_expand_value(limits: ExpansionLimits, e: i32) -> i32 {
    if e == 0 {
        return 0;
    }
    let (neg, mut e, max) = if e < 0 {
        (true, -e, limits.shrink)
    } else {
        (false, e, limits.stretch)
    };
    if e > max {
        e = max;
    } else if e % limits.step > 0 {
        e = limits.step * round_xn_over_d(e, 1, limits.step);
    }
    if neg { -e } else { e }
}

/// Paragraph-wide expansion parameters pdfTeX records while scanning
/// (`check_expand_pars`: `max_stretch_ratio`, `max_shrink_ratio`,
/// `cur_font_step`). pdfTeX errors out if expandable fonts in one paragraph
/// disagree, so one set suffices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ParagraphExpansion {
    pub max_stretch: i32,
    pub max_shrink: i32,
    pub step: i32,
}

impl ParagraphExpansion {
    /// Record a font as `check_expand_pars` does. Returns `Err` for the two
    /// "using fonts with different ... in one paragraph is not allowed" errors.
    pub fn note_font(&mut self, font: &FontParams) -> Result<bool, &'static str> {
        let Some(l) = font.expansion else { return Ok(false) };
        if l.stretch == 0 && l.shrink == 0 {
            return Ok(false);
        }
        if self.step == 0 {
            self.step = l.step;
        } else if self.step != l.step {
            return Err("using fonts with different step of expansion in one paragraph is not allowed");
        }
        if l.stretch > 0 {
            if self.max_stretch == 0 {
                self.max_stretch = l.stretch;
            } else if self.max_stretch != l.stretch {
                return Err("using fonts with different limit of expansion in one paragraph is not allowed");
            }
        }
        if l.shrink > 0 {
            if self.max_shrink == 0 {
                self.max_shrink = l.shrink;
            } else if self.max_shrink != l.shrink {
                return Err("using fonts with different limit of expansion in one paragraph is not allowed");
            }
        }
        Ok(true)
    }
}

/// `try_break`, "Consider the demerits for a line from r to cur_p" (pdfTeX
/// additions only): the shortfall TeX's badness is computed from, given
/// `shortfall = line_width - natural width` **after** adding the line's total
/// protrusion (`total_pw`) when `\pdfprotrudechars > 1`.
///
/// `font_stretch`/`font_shrink` are `cur_active_width[7]`/`[8]` (sums of
/// `char_stretch`/`kern_stretch` over the line); `margin_*` are the
/// "variations of marginal kerns" (always 0 for autoexpand fonts).
/// Only call with `\pdfadjustspacing > 1`.
pub fn adjust_shortfall(
    shortfall: Scaled,
    font_stretch: Scaled,
    font_shrink: Scaled,
    margin_stretch: Scaled,
    margin_shrink: Scaled,
    par: ParagraphExpansion,
) -> Scaled {
    if shortfall == 0 {
        return 0;
    }
    // In i64: a caller that clamps an out-of-range line to `i32::MIN` must
    // not overflow `-shortfall` or the stretch sums. In-range values give
    // exactly the i32 results.
    let shortfall = i64::from(shortfall);
    let st = i64::from(font_stretch) + i64::from(margin_stretch);
    let sh = i64::from(font_shrink) + i64::from(margin_shrink);
    let ratio = |max: i32| i64::from(max / par.step.max(1)).max(1);
    let adjusted = if shortfall > 0 && st > 0 {
        if st > shortfall {
            (st / ratio(par.max_stretch)) / 2
        } else {
            shortfall - st
        }
    } else if shortfall < 0 && sh > 0 {
        if sh > -shortfall {
            -((sh / ratio(par.max_shrink)) / 2)
        } else {
            shortfall + sh
        }
    } else {
        shortfall
    };
    adjusted.clamp(i64::from(Scaled::MIN), i64::from(Scaled::MAX)) as Scaled
}

/// `hpack(p, w, cal_expand_ratio)`: the line's `font_expand_ratio`
/// (−1000..1000, 0 = no expansion). `excess = w − natural width` (margin
/// kerns included); `finite_glue_order` is true when the dominating
/// stretch (resp. shrink) order of the line's glue is `normal`.
pub fn line_expand_ratio(
    excess: Scaled,
    font_stretch: Scaled,
    font_shrink: Scaled,
    finite_glue_order: bool,
) -> i32 {
    let r = if excess > 0 && finite_glue_order && font_stretch > 0 {
        divide_scaled(excess, font_stretch, 3)
    } else if excess < 0 && finite_glue_order && font_shrink > 0 {
        divide_scaled(excess, font_shrink, 3)
    } else {
        0
    };
    fix_int(r, -1000, 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjust_shortfall_is_total_at_the_scaled_limits() {
        // Fuzz finding (render-pipeline fuzz_render, `pdftex.rs:284 attempt
        // to negate with overflow`): paragraph-layout clamps an overfull
        // line's shortfall to `i32::MIN` before calling this.
        let par = ParagraphExpansion { max_stretch: 20, max_shrink: 20, step: 1 };
        assert_eq!(adjust_shortfall(i32::MIN, 0, 1, 0, 0, par), i32::MIN + 1);
        assert_eq!(adjust_shortfall(i32::MIN, 0, i32::MAX, 0, i32::MAX, par), -107_374_182);
        assert_eq!(adjust_shortfall(i32::MAX, i32::MAX, 0, i32::MAX, 0, par), 107_374_182);
        // A ratio of zero (max below the step) no longer divides by zero.
        let odd = ParagraphExpansion { max_stretch: 1, max_shrink: 1, step: 5 };
        assert_eq!(adjust_shortfall(10, 100, 0, 0, 0, odd), 50);
        // In range, unchanged: pdfTeX's `divide(font_stretch, ratio)`.
        assert_eq!(adjust_shortfall(1000, 400, 0, 0, 0, par), 600);
        assert_eq!(adjust_shortfall(100, 400, 0, 0, 0, par), 10);
        assert_eq!(adjust_shortfall(-100, 0, 400, 0, 0, par), -10);
    }

    fn font() -> FontParams {
        let mut f = FontParams::plain(655_200);
        f.expansion = ExpansionLimits::from_primitive(20, 20, 1);
        f.set_rpcode(b'.', 296);
        f
    }

    #[test]
    fn primitive_normalisation() {
        assert_eq!(
            ExpansionLimits::from_primitive(30, 20, 10),
            Some(ExpansionLimits { stretch: 30, shrink: 20, step: 10 })
        );
        assert_eq!(
            ExpansionLimits::from_primitive(25, 900, 10),
            Some(ExpansionLimits { stretch: 20, shrink: 500, step: 10 })
        );
        assert_eq!(ExpansionLimits::from_primitive(20, 20, 0), None);
    }

    #[test]
    fn protrusion_and_stretch() {
        let f = font();
        assert_eq!(f.right_protrusion(b'.'), round_xn_over_d(655_200, 296, 1000));
        assert_eq!(f.left_protrusion(b'.'), 0);
        // 20/1000 of a 5.00002pt glyph, rounded twice as pdfTeX does
        let w = 327_681;
        assert_eq!(f.char_stretch(b'a', w), round_xn_over_d(w, 1020, 1000) - w);
        assert_eq!(f.char_shrink(b'a', w), w - round_xn_over_d(w, 980, 1000));
    }

    #[test]
    fn expansion_value_rounding() {
        let f = font();
        assert_eq!(f.char_expansion(b'a', 1000), 20);
        assert_eq!(f.char_expansion(b'a', -1000), -20);
        assert_eq!(f.char_expansion(b'a', 300), 6);
        assert_eq!(f.char_expansion(b'a', 275), 6); // 5.5 -> 6 (away from zero)
        assert_eq!(f.char_expansion(b'a', -275), -6);
        let mut g = f.clone();
        g.set_efcode(b'a', 0);
        assert_eq!(g.char_expansion(b'a', 1000), 0);
    }

    #[test]
    fn shortfall_rule() {
        let par = ParagraphExpansion { max_stretch: 20, max_shrink: 20, step: 1 };
        assert_eq!(adjust_shortfall(100, 4000, 0, 0, 0, par), 4000 / 20 / 2);
        assert_eq!(adjust_shortfall(5000, 4000, 0, 0, 0, par), 1000);
        assert_eq!(adjust_shortfall(-100, 0, 4000, 0, 0, par), -(4000 / 20 / 2));
        assert_eq!(adjust_shortfall(-100, 4000, 0, 0, 0, par), -100);
    }
}
