//! Primes (GH-278): `f'` is `f^{\prime}`, set in script style from cmsy
//! slot "30 and raised by the ordinary superscript shift, and every `'` of a
//! run -- plus a directly following `^{...}` -- joins one superscript
//! (latex.ltx `\active@math@prime` / `\pr@m@s`).
//!
//! **Oracle.** TeX Live 2026 pdflatex on this machine (pdflatex is an oracle,
//! never in the product path), preamble
//! `\documentclass[12pt]{article}\usepackage[T1]{fontenc}\usepackage{lmodern}`
//! and the same without `[12pt]`. Each formula was boxed twice and read with
//! `\showbox` (`\showboxdepth=100`), which gives TeX's exact box algebra:
//!
//! ```text
//! \def\t#1{\setbox0\hbox{$#1$}\showbox0 \setbox0\hbox{$\displaystyle #1$}\showbox0}
//! \t{f'(x)} \t{f''} \t{f'''} \t{f^{\prime}} \t{f'^2} \t{f_1'} \t{x'_a}
//! ```
//!
//! At 12 pt `$f''$` is
//!
//! ```text
//! \hbox(8.79916+2.33331)x12.18398
//! .\OML/lmm/m/it/12 f
//! .\kern1.28123                                 <- f's italic correction
//! .\hbox(4.44446+0.0)x5.11111, shifted -4.3547  <- one superscript
//! ..\OMS/lmsy/m/n/8 0                           <- \prime, cmsy "30 at 8 pt
//! ..\OMS/lmsy/m/n/8 0
//! ```
//!
//! so `f` is 12.18398 - 1.28123 - 5.11111 = 5.79164 pt wide, the first prime
//! sits at 5.79164 + 1.28123 = 7.07287 pt, each prime advances 2.30556 pt
//! (the box adds `\scriptspace` 0.5 pt) and the superscript is raised
//! 4.3547 pt (4.9547 pt under `\displaystyle`). `f_1'` and `x'_a` set a
//! `\vbox` whose superscript box is 4.3547 pt above the baseline and whose
//! subscript box is 2.9666 pt below it (the kern between them fills the
//! rest). At 10 pt the same readings give `f` 4.89586 pt, correction
//! 1.0764 pt, primes in lmsy7, superscript shift 3.62892 pt (display
//! 4.12892 pt), subscript shift 2.482 pt for `f_1'` inline and 2.47217 pt
//! otherwise. The display rows below were checked with `\[...\]`, whose math
//! list is laid out exactly as the `\displaystyle` box.
//!
//! The **ink** of the prime is pdflatex's glyph bounding box from the Type 1
//! metrics it draws with, `lmsy7.afm` `C 205 ; WX 329.366 ; N prime ; B 48 41
//! 299 559` and `lmsy8.afm` `B 31 41 274 559` (thousandths of the font size),
//! placed at the box position above.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};
use flashtex_render_pipeline::ids::GlyphId;
use flashtex_render_pipeline::FontSet;

const TOL_BP: f64 = 0.5;

/// 1 bp = 1.00375 TeX pt.
fn bp(pt: f64) -> f64 {
    pt / 1.00375
}

const PRE12: &str = "\\documentclass[12pt]{article}\\usepackage[T1]{fontenc}\\usepackage{lmodern}";
const PRE10: &str = "\\documentclass{article}\\usepackage[T1]{fontenc}\\usepackage{lmodern}";

/// One painted math glyph: cluster text, origin x and baseline y (bp, y
/// down), font size (bp), and its ink `[x_min, y_min, x_max, y_max]` in bp
/// relative to the origin (y up).
#[derive(Debug, Clone)]
struct Painted {
    text: String,
    x: f64,
    y: f64,
    size: f64,
    ink: [f64; 4],
}

fn math_glyphs(src: &str) -> Vec<Painted> {
    let fonts = FontSet::with_default_dirs(&[]);
    let r = render_one_with(src, &fonts);
    let mut out = Vec::new();
    for item in &r.v2.pages[0].items {
        let Item::GlyphRun(run) = item else { continue };
        if run.role != RunRole::Math {
            continue;
        }
        let face = fonts.by_font_id(&run.font_id).expect("resource of a drawn run");
        let size = run.font_size.to_bp();
        for g in &run.glyphs {
            let c = &run.clusters[g.cluster as usize];
            let b = face.bounds(GlyphId(g.gid), None);
            let s = |u| face.pt(i64::from(u), size);
            out.push(Painted {
                text: run.text[c.text_start_byte as usize..c.text_end_byte as usize].to_string(),
                x: g.origin_x.to_bp(),
                y: g.baseline_y.to_bp(),
                size,
                ink: [s(b.x_min), s(b.y_min), s(b.x_max), s(b.y_max)],
            });
        }
    }
    out.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    out
}

/// `(text, dx pt, rise pt, size pt)`: the next unmatched glyph with that
/// text (in x order, the base excluded) relative to the base glyph's origin,
/// rise positive = up.
type Want<'a> = &'a [(&'a str, f64, f64, f64)];

fn check(what: &str, src: &str, base: &str, want: Want<'_>) {
    let glyphs = math_glyphs(src);
    let bi = glyphs.iter().position(|g| g.text == base).unwrap_or_else(|| panic!("{what}: no base {base:?} in {glyphs:?}"));
    let (bx, by) = (glyphs[bi].x, glyphs[bi].y);
    let mut used = vec![false; glyphs.len()];
    used[bi] = true;
    let mut fails = Vec::new();
    for (text, dx, rise, size) in want {
        let Some(i) = (0..glyphs.len()).find(|&i| !used[i] && glyphs[i].text == *text) else {
            fails.push(format!("{what}: no {text:?} in {glyphs:?}"));
            continue;
        };
        used[i] = true;
        let g = &glyphs[i];
        let got = (g.x - bx, by - g.y, g.size);
        let exp = (bp(*dx), bp(*rise), bp(*size));
        if (got.0 - exp.0).abs() > TOL_BP || (got.1 - exp.1).abs() > TOL_BP || (got.2 - exp.2).abs() > TOL_BP {
            fails.push(format!(
                "{what} {text:?}: dx {:.4} rise {:.4} size {:.3} bp, pdflatex dx {:.4} rise {:.4} size {:.3} bp",
                got.0, got.1, got.2, exp.0, exp.1, exp.2
            ));
        }
    }
    assert!(fails.is_empty(), "{}", fails.join("\n"));
}

fn doc(pre: &str, math: &str) -> String {
    format!("{pre}\\begin{{document}}{math}\\end{{document}}")
}

/// One class size of the oracle, every length in pt from the `\showbox`
/// readings above.
struct Size {
    pre: &'static str,
    label: &'static str,
    /// Width of the math-italic `f`, and its italic correction.
    f: f64,
    kern: f64,
    /// Width of the math-italic `x`.
    x: f64,
    text: f64,
    script: f64,
    /// Advance of one lmsy prime at the script size.
    prime: f64,
    /// `(open, close, style, superscript shift, f_1' subscript shift, x'_a
    /// subscript shift)`.
    styles: [(&'static str, &'static str, &'static str, f64, f64, f64); 2],
}

const SIZES: [Size; 2] = [
    Size {
        pre: PRE12,
        label: "12pt",
        f: 5.79164,
        kern: 1.28123,
        x: 6.67703,
        text: 12.0,
        script: 8.0,
        prime: 2.30556,
        styles: [("$", "$", "inline", 4.3547, 2.9666, 2.9666), ("\\[", "\\]", "display", 4.9547, 2.9666, 2.9666)],
    },
    Size {
        pre: PRE10,
        label: "10pt",
        f: 4.89586,
        kern: 1.0764,
        x: 5.71527,
        text: 10.0,
        script: 7.0,
        prime: 2.30556,
        styles: [("$", "$", "inline", 3.62892, 2.482, 2.47217), ("\\[", "\\]", "display", 4.12892, 2.47217, 2.47217)],
    },
];

/// Superscript and subscript placement, glyph sizes and the advance of the
/// whole superscript box (the glyph after it in `f'(x)` and `f'^2`), inline
/// and in display, at 10 pt and 12 pt. Every offset was already within
/// 0.001 bp of pdflatex on main: the compiler and math-layout lay primes out
/// as TeX does, and this pins it.
#[test]
fn primes_are_one_script_size_superscript() {
    if !lm_available() {
        return;
    }
    for s in &SIZES {
        let px = s.f + s.kern;
        for (open, close, style, sup, sub_f1, sub_xa) in s.styles {
            let d = |m: &str| doc(s.pre, &format!("{open}{m}{close}"));
            let w = |m: &str| format!("{} {style} {m}", s.label);
            let p = |n: f64| ("′", px + n * s.prime, sup, s.script);
            check(&w("f'(x)"), &d("f'(x)"), "f", &[p(0.0), ("(", px + s.prime + 0.5, 0.0, s.text)]);
            check(&w("f''"), &d("f''"), "f", &[p(0.0), p(1.0)]);
            check(&w("f'''"), &d("f'''"), "f", &[p(0.0), p(1.0), p(2.0)]);
            check(&w("f^{\\prime}"), &d("f^{\\prime}"), "f", &[p(0.0)]);
            check(&w("f'^2"), &d("f'^2"), "f", &[p(0.0), ("2", px + s.prime, sup, s.script)]);
            check(&w("f_1'"), &d("f_1'"), "f", &[p(0.0), ("1", s.f, -sub_f1, s.script)]);
            check(&w("x'_a"), &d("x'_a"), "x", &[("′", s.x, sup, s.script), ("a", s.x, -sub_xa, s.script)]);
        }
    }
}

/// pdflatex's prime ink, thousandths of the font size: `lmsy<size>.afm`
/// `N prime ; B ...`.
fn lmsy_prime_bbox(script: f64) -> [f64; 4] {
    match script as i64 {
        7 => [48.0, 41.0, 299.0, 559.0],
        8 => [31.0, 41.0, 274.0, 559.0],
        other => panic!("no lmsy{other} reading"),
    }
}

/// The painted prime's ink relative to the base glyph's origin: the bottom
/// and top of the stroke and its horizontal centre within 0.5 bp of the lmsy
/// glyph pdflatex draws.
///
/// This is the GH-278 owner report (a prime "sitting high"): Latin Modern
/// Math's cmap glyph for U+2032 (`minute`, ink 430..748 thousandths of an
/// em) is the *pre-raised* prime that Unicode math sets without a script, so
/// painting it at the superscript position raised it a second time. On main
/// this test failed with, among others:
///
/// ```text
/// 10pt inline f': prime ink bottom 6.6143 bp, pdflatex 3.9012 bp
/// 10pt inline f': prime ink top 8.8319 bp, pdflatex 7.5139 bp
/// ```
///
/// cmsy's "30 is the unraised prime, which the face carries as the `ssty`
/// alternate `minute.st` (ink 96..549).
#[test]
fn prime_ink_is_the_unraised_script_prime() {
    if !lm_available() {
        return;
    }
    let mut fails = Vec::new();
    for s in &SIZES {
        let px = s.f + s.kern;
        for (open, close, style, sup, _, _) in s.styles {
            for m in ["f'", "f''", "f^{\\prime}", "f'^2", "f_1'"] {
                let src = doc(s.pre, &format!("{open}{m}{close}"));
                let glyphs = math_glyphs(&src);
                let f = glyphs.iter().find(|g| g.text == "f").expect("the base f");
                let prime = glyphs.iter().find(|g| g.text == "′").unwrap_or_else(|| panic!("{src}: no prime in {glyphs:?}"));
                let (dx, rise) = (prime.x - f.x, f.y - prime.y);
                let got = [dx + prime.ink[0], rise + prime.ink[1], dx + prime.ink[2], rise + prime.ink[3]];
                let b = lmsy_prime_bbox(s.script);
                let em = s.script / 1000.0;
                let want = [bp(px + b[0] * em), bp(sup + b[1] * em), bp(px + b[2] * em), bp(sup + b[3] * em)];
                let centre = |i: &[f64; 4]| (i[0] + i[2]) / 2.0;
                let what = format!("{} {style} {m}", s.label);
                for (name, g, w) in [("bottom", got[1], want[1]), ("top", got[3], want[3]), ("centre", centre(&got), centre(&want))] {
                    if (g - w).abs() > TOL_BP {
                        fails.push(format!("{what}: prime ink {name} {g:.4} bp, pdflatex {w:.4} bp"));
                    }
                }
            }
        }
    }
    assert!(fails.is_empty(), "{}", fails.join("\n"));
}
