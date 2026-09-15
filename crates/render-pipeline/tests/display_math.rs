//! `\[...\]` is typeset in TeX's display style (`\displaystyle`): the large
//! cmex-size operator variant, limits above/below (Rule 13a), and the
//! display fraction shifts (Rule 15b, `\sigma_8`/`\sigma_11`); `$...$` stays
//! in text style with limits as scripts and the smaller variant.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};

fn doc(body: &str) -> String {
    format!("\\begin{{document}}{body}\\end{{document}}")
}

type G = (String, u16, f64, f64, f64);

/// `(text, gid, x bp, baseline y bp, font size bp)` of every math glyph on page 1.
fn math_glyphs(body: &str) -> Vec<G> {
    let r = render_one(&doc(body));
    let mut out = Vec::new();
    for item in &r.v2.pages[0].items {
        if let Item::GlyphRun(run) = item {
            if run.role != RunRole::Math {
                continue;
            }
            let mut chars = run.clusters.iter().map(|c| run.text[c.text_start_byte as usize..c.text_end_byte as usize].to_string());
            for g in &run.glyphs {
                let t = chars.next().unwrap_or_default();
                out.push((t, g.gid, g.origin_x.to_bp(), g.baseline_y.to_bp(), run.font_size.to_bp()));
            }
        }
    }
    out
}

fn find<'a>(gs: &'a [G], text: &str, nth: usize) -> &'a G {
    gs.iter().filter(|g| g.0 == text).nth(nth).unwrap_or_else(|| panic!("no {nth}th {text:?} in {gs:?}"))
}

#[test]
fn bracket_display_uses_display_style_and_inline_stays_text_style() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let inline = math_glyphs("Inline $\\sum_{i=1}^{n} \\frac{a_i}{b}$ here.");
    let display = math_glyphs("Before.\n\n\\[ \\sum_{i=1}^{n} \\frac{a_i}{b} = \\int_0^1 f(x)\\,dx \\]");
    eprintln!("inline:  {inline:?}");
    eprintln!("display: {display:?}");
    let sum_t = find(&inline, "P", 0);
    let sum_d = find(&display, "P", 0);
    // Rule 13: the display variant is a different (taller) glyph.
    assert_ne!(sum_t.1, sum_d.1, "display \\sum must use the large operator variant");
    // Rule 13a: limits above and below in display style; the subscript
    // `i=1` sits below the operator's baseline, the superscript `n` above,
    // and both are centred on the operator rather than trailing it.
    let n_d = find(&display, "n", 0);
    let i_d = find(&display, "i", 0);
    assert!(i_d.3 > sum_d.3, "display subscript below the operator: i y {} vs sum y {}", i_d.3, sum_d.3);
    assert!(n_d.3 < sum_d.3, "display superscript above the operator: n y {} vs sum y {}", n_d.3, sum_d.3);
    assert!(n_d.2 < sum_d.2 + 6.0, "display limits are centred on the operator, not scripts to its right");
    // Text style: scripts to the right of the operator.
    let n_t = find(&inline, "n", 0);
    assert!(n_t.2 > sum_t.2 + 5.0, "inline superscript trails the operator");
    // Rule 15b: the display numerator is shifted up by \sigma_8 (larger
    // than the text-style \sigma_9); the fraction is set in display size
    // (numerator in text style, not script style).
    let a_t = find(&inline, "a", 0);
    let a_d = find(&display, "a", 0);
    let b_t = find(&inline, "b", 0);
    let b_d = find(&display, "b", 0);
    let num_shift_t = sum_t.3 - a_t.3;
    let num_shift_d = sum_d.3 - a_d.3;
    assert!(num_shift_d > num_shift_t + 1.0, "display numerator shift {num_shift_d} vs text {num_shift_t}");
    assert!(b_d.3 - sum_d.3 > b_t.3 - sum_t.3 + 1.0, "display denominator shift");
    // The display fraction's numerator is text size; inline it is script size.
    assert!(a_d.4 > a_t.4 + 1.0, "display numerator font size {} vs inline {}", a_d.4, a_t.4);
    // \int is \nolimits: its scripts stay at the corners even in display.
    let int_d = find(&display, "R", 0);
    let one_d = find(&display, "1", 1);
    assert!(one_d.2 > int_d.2 + 4.0, "\\int limits at the corners");
}
