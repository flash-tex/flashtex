//! Tall radicals (#690): `\sqrt` over a fraction or an integral picks the
//! smallest radical sign that covers the radicand, the overbar sits on that
//! sign's top edge and spans the radicand, and the painted Latin Modern Math
//! variant's ink fills the cmex box TeX laid out — no sign poking out above
//! its own bar.
//!
//! The reference numbers are pdfTeX 3.141592653 (TeX Live 2026) `\showbox`
//! output for a 10 pt `article`. For `$\sqrt{\frac{a}{b}}$`, for example:
//!
//! ```text
//! \hbox(8.60141+3.79868)x16.73766
//! .\hbox(0.39998+11.60013)x10.00002, shifted -7.80145
//! ..\OMX/cmex/m/n/5 p
//! .\vbox(8.60141+3.44841)x6.73764
//! ..\kern0.39998
//! ..\rule(0.39998+0.0)x*
//! ..\kern0.85025
//! ..\hbox(6.9512+3.44841)x6.73764
//! ```
//!
//! so the sign's box is 0.39998+11.60013 = 12.00011 pt tall, the rule is
//! 0.39998 pt thick with its top on the sign's top, and it is 6.73764 pt
//! wide — the radicand's width.

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::fonts::{Family, FontSet, Role};
use flashtex_render_pipeline::ids::GlyphId;

const BP: f64 = 72.0 / 72.27;
/// The project's gate for a glyph position.
const GLYPH_GATE: f64 = 0.5 * BP;
/// The project's gate for a rule.
const RULE_GATE: f64 = 0.1 * BP;

fn doc(body: &str) -> String {
    format!("\\documentclass[10pt]{{article}}\\begin{{document}}${body}$\\end{{document}}")
}

/// One radical sign in the rendered page: the glyphs painted for it and the
/// box TeX laid it out in, all in TeX pt with y downward from the page top.
struct Surd {
    glyphs: Vec<(u16, f64)>,
    x: f64,
    top: f64,
    width: f64,
    height: f64,
}

struct Rendered {
    surds: Vec<Surd>,
    /// Rules, as `(x, top, width, height)` in TeX pt.
    rules: Vec<(f64, f64, f64, f64)>,
    limitations: Vec<String>,
}

fn radicals(body: &str) -> Rendered {
    let r = render_one(&doc(body));
    let mut surds = Vec::new();
    let mut rules = Vec::new();
    for page in &r.v2.pages {
        for it in page.resident_items() {
            match it {
                Item::GlyphRun(run) => {
                    for (ci, c) in run.clusters.iter().enumerate() {
                        if run.text[c.text_start_byte..c.text_end_byte] != *"\u{221A}" {
                            continue;
                        }
                        surds.push(Surd {
                            glyphs: run
                                .glyphs
                                .iter()
                                .filter(|g| g.cluster as usize == ci)
                                .map(|g| (g.gid, g.baseline_y.to_bp() / BP))
                                .collect(),
                            x: c.hit_rect.x.to_bp() / BP,
                            top: c.hit_rect.top.to_bp() / BP,
                            width: c.hit_rect.width.to_bp() / BP,
                            height: c.hit_rect.height.to_bp() / BP,
                        });
                    }
                }
                Item::Rule(rl) => rules.push((
                    rl.x.to_bp() / BP,
                    rl.top.to_bp() / BP,
                    rl.width.to_bp() / BP,
                    rl.height.to_bp() / BP,
                )),
                _ => {}
            }
        }
    }
    Rendered {
        surds,
        rules,
        limitations: r
            .v2
            .diagnostics
            .iter()
            .filter(|d| d.code == "math_limitation")
            .map(|d| d.message.clone())
            .collect(),
    }
}

/// The rule whose top is nearest `top` (the overbar of that radical; nested
/// radicals and fractions contribute rules of their own).
fn bar_at(r: &Rendered, top: f64) -> (f64, f64, f64, f64) {
    *r.rules
        .iter()
        .min_by(|a, b| (a.1 - top).abs().partial_cmp(&(b.1 - top).abs()).expect("finite"))
        .expect("a rule")
}

/// Every radicand gets a sign that covers it, at exactly the size pdfTeX
/// chose, with the bar on the sign's top edge and spanning the radicand.
#[test]
fn radical_signs_and_bars_match_pdftex() {
    if !lm_available() {
        return;
    }
    // (source, pdfTeX's sign box height+depth, pdfTeX's rule width)
    let cases: [(&str, f64, f64); 6] = [
        // \hbox(0.39998+9.6)x8.33336 — cmsy10's own sign, no larger variant.
        ("\\sqrt{x}", 9.99998, 5.71527),
        // \OMX/cmex ^^p: \hbox(0.39998+11.60013)x10.00002.
        ("\\sqrt{\\frac{a}{b}}", 12.00011, 6.73764),
        // \OMX/cmex ^^q: \hbox(0.39998+17.60019)x10.00002.
        ("\\sqrt{\\frac{\\frac{a}{b}}{c}}", 18.00017, 8.67213),
        // \OMX/cmex ^^s: \hbox(0.39998+29.60031)x10.00002.
        ("\\displaystyle\\sqrt{\\int_0^1 f\\,dx}", 30.00029, 34.71179),
        // #690's own two, in display style. ^^r: \hbox(0.39998+23.60025).
        // The bar is 6.56668 pt in pdfTeX; Latin Modern's `\ell` is 0.09331
        // pt wider than cmmi10's, which is a font-metric divergence of its
        // own, so only the sign is pinned against the oracle here.
        ("\\displaystyle\\sqrt{\\frac{\\ell}{\\ell}}", 24.00023, f64::NAN),
        ("\\displaystyle\\sqrt{\\int_0^\\infty f(x)\\,dx}", 30.00029, 52.19098),
    ];
    for (body, sign, bar_width) in cases {
        let r = radicals(body);
        assert!(r.limitations.is_empty(), "{body}: {:?}", r.limitations);
        assert_eq!(r.surds.len(), 1, "{body}: one radical");
        let s = &r.surds[0];
        assert!(
            (s.height - sign).abs() < GLYPH_GATE,
            "{body}: sign box {} pt, pdfTeX {sign} pt",
            s.height
        );
        let (bx, btop, bw, bh) = bar_at(&r, s.top);
        // The bar's top is the sign's top (tex.web §737 raises the sign so
        // the two meet) and it is one default rule thickness thick.
        assert!((btop - s.top).abs() < RULE_GATE, "{body}: bar top {btop} pt, sign top {} pt", s.top);
        assert!((bh - 0.39998).abs() < RULE_GATE, "{body}: bar {bh} pt thick");
        // It starts where the sign's box ends, so the two touch.
        assert!(
            (bx - (s.x + s.width)).abs() < RULE_GATE,
            "{body}: bar starts at {bx} pt, sign ends at {} pt",
            s.x + s.width
        );
        if !bar_width.is_nan() {
            assert!((bw - bar_width).abs() < RULE_GATE, "{body}: bar {bw} pt wide, pdfTeX {bar_width} pt");
        }
    }
}

/// A radical inside a radical: each sign is sized for its own radicand.
#[test]
fn nested_radicals_each_get_their_own_size() {
    if !lm_available() {
        return;
    }
    // \showbox: \hbox(12.2514+6.14874)x26.73767, the outer sign
    // \OMX/cmex ^^q (0.39998+17.60019) over a 16.73766 pt radicand, the
    // inner \OMX/cmex ^^p (0.39998+11.60013) over a 6.73764 pt one.
    let r = radicals("\\sqrt{\\sqrt{\\frac{a}{b}}}");
    assert!(r.limitations.is_empty(), "{:?}", r.limitations);
    assert_eq!(r.surds.len(), 2, "two radicals");
    let mut sizes: Vec<(f64, f64)> = r.surds.iter().map(|s| (s.height, bar_at(&r, s.top).2)).collect();
    sizes.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("finite"));
    for ((got_sign, got_bar), (sign, bar)) in sizes.iter().zip([(12.00011, 6.73764), (18.00017, 16.73766)]) {
        assert!((got_sign - sign).abs() < GLYPH_GATE, "sign {got_sign} pt, pdfTeX {sign} pt");
        assert!((got_bar - bar).abs() < RULE_GATE, "bar {got_bar} pt, pdfTeX {bar} pt");
    }
}

/// Latin Modern Math's radical sign and its variants, in ink above and below
/// their own origin at a 10 pt size. Every variant is centred on the axis
/// (2.5 pt) and 6 pt taller than the one before it — `radical` 0.4/9.6 is
/// cmsy10's own sign, then 8.5/3.5, 11.5/6.5, 14.5/9.5, 17.5/12.5.
///
/// Read out of `latinmodern-math.otf`'s CFF charstrings with an independent
/// Type 2 interpreter, and confirmed against the ink Ghostscript traces in
/// the PDF FlashTeX writes (`gs -sDEVICE=bbox`, agreeing to 0.02 pt).
const VARIANT_INK: [(&str, f64, f64); 5] = [
    ("\\sqrt{x}", 0.4, 9.6),
    ("\\sqrt{\\frac{a}{b}}", 8.5, 3.5),
    ("\\sqrt{\\frac{\\frac{a}{b}}{c}}", 11.5, 6.5),
    ("\\displaystyle\\sqrt{\\frac{\\ell}{\\ell}}", 14.5, 9.5),
    ("\\displaystyle\\sqrt{\\int_0^\\infty f(x)\\,dx}", 17.5, 12.5),
];

/// The painted variant's ink fills the cmex box it stands for, top and
/// bottom, so the sign meets its own bar. Before #690 the ink box was traced
/// with the charstring's optional width argument still on the stack, which
/// shifted the glyph's first `rmoveto` by one argument: every Latin Modern
/// Math radical variant that declares a width (`radical.v2` and up) came out
/// 1.1, 4.1 and 7.1 pt too low, and the sign was then painted that far above
/// its bar.
#[test]
fn a_radical_variants_ink_fills_its_box() {
    if !lm_available() {
        return;
    }
    for (body, h, d) in VARIANT_INK {
        let r = radicals(body);
        let s = &r.surds[0];
        assert_eq!(s.glyphs.len(), 1, "{body}: one variant, not an assembly");
        let baseline = s.glyphs[0].1;
        assert!(
            (baseline - h - s.top).abs() < GLYPH_GATE,
            "{body}: ink top {} pt, box top {} pt",
            baseline - h,
            s.top
        );
        assert!(
            (baseline + d - (s.top + s.height)).abs() < GLYPH_GATE,
            "{body}: ink bottom {} pt, box bottom {} pt",
            baseline + d,
            s.top + s.height
        );
    }
}

/// The charstring bounds themselves, at the seam the bug was in.
#[test]
fn radical_variant_bounds_are_read_past_the_charstring_width() {
    if !lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let face = fonts.resolve(Family::LatinModern, Role::Math, 10.0).face;
    for (body, want_h, want_d) in VARIANT_INK {
        let gid = radicals(body).surds[0].glyphs[0].0;
        let b = face.bounds(GlyphId(gid), Some('\u{221A}'));
        let (h, d) = (face.pt(i64::from(b.y_max), 10.0), face.pt(-i64::from(b.y_min), 10.0));
        assert!(
            (h - want_h).abs() < 1e-9 && (d - want_d).abs() < 1e-9,
            "{body}: gid {gid} ({h}, {d}), want ({want_h}, {want_d})"
        );
    }
}
