//! `\vdots` and `\ddots` are built stacks of text-font periods, not a glyph.
//!
//! The LaTeX kernel does not give either command a character. Both are boxes
//! built out of `\hbox{.}` of the *text* font (`fontmath.ltx` 404-409,
//! identical to `plain.tex` 934-937):
//!
//! ```text
//! \vdots {\vbox{\baselineskip4\p@ \lineskiplimit\z@
//!               \kern6\p@\hbox{.}\hbox{.}\hbox{.}}}
//! \ddots {\mathinner{\mkern1mu\raise7\p@\vbox{\kern7\p@\hbox{.}}\mkern2mu
//!                    \raise4\p@\hbox{.}\mkern2mu\raise\p@\hbox{.}\mkern1mu}}
//! ```
//!
//! Everything there except the period itself and the `mu` kerns is an
//! absolute length — `\p@` is 1pt, not an em — so the 6pt kern, the 4pt
//! `\baselineskip` and the 7/4/1pt raises do not move with the body size, and
//! `\hbox` leaves math mode so the periods do not shrink in a script either.
//! Both boxes come out 14pt plus the period's height tall with no depth,
//! which is 15.05554pt in a 10pt document — tall enough that `\topskip`
//! cannot fit the line and the **first baseline of the page** moves down.
//!
//! flashtex mapped both to a single Latin Modern Math codepoint instead
//! (U+22EE, U+22F1), whose advances are 2.18pt and 6.13pt and whose ink box
//! is 5.06pt too short (issue #717). The stacks are now built through the
//! same placeholder seam a nested grid or a `\boxed` frame uses
//! (`mathtext::BuiltBody::Dots`).
//!
//! ## Oracle
//!
//! pdfTeX 3.141592653 (TeX Live 2026); glyph origins extracted with PyMuPDF,
//! in bp from the top-left of the 612x792 bp page. pdflatex is an oracle and
//! is never in the product path.
//!
//! ```tex
//! \documentclass[10pt]{article}\begin{document}$a\vdots b$ and more text here.\end{document}
//! ```
//!
//! ```text
//! \vdots   a 148.7120 139.8010 | . 153.9780 131.8311 | . 153.9780 135.8160
//!                              | . 153.9780 139.8010 | b 156.7476 139.8010
//! \ddots   a 148.7120 139.8010 | . 156.1920 132.8270 | . 160.0670 135.8160
//!                              | . 163.9410 138.8051 | b 168.9220 139.8010
//! \ldots   a 148.7120 134.7650   (the same document with `\ldots`: the
//!                                 undisturbed `\topskip` baseline)
//! ```
//!
//! and `\showbox` of the same formulas, in TeX pt:
//!
//! ```text
//! 10pt  \hbox{.}  (1.05554+0.0)x2.77779
//!       \vdots    \vbox(15.05554+0.0)x2.77779
//!       \ddots    \hbox(15.05554+0.0)x11.66661, thin 1.66663 on each side
//! 12pt  \hbox{.}  (1.16666+0.0)x3.26385
//!       \vdots    \vbox(15.16666+0.0)x3.26385
//!       \ddots    \hbox(15.16666+0.0)x13.7915
//! ```
//!
//! The 12pt row is why the absolute lengths are pinned separately below: the
//! dots stay exactly 4pt apart and the raises exactly 7/4/1pt while the
//! period and the `mu` kerns grow.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};

const BP: f64 = 72.0 / 72.27;
/// The project's gate for a glyph position.
const GLYPH_GATE: f64 = 0.5 * BP;

/// pdfTeX's first baseline with an undisturbed `\topskip`, TeX pt from the
/// page top (the constant `math_symbol_line_height.rs` pins).
const PLAIN_BASELINE: f64 = 135.27038;

/// `(text, x, baseline y)` of every glyph on page 1, in paint order, in TeX
/// pt from the top-left of the page. `math` keeps only the math run glyphs.
fn glyphs(body: &str, math: bool) -> Vec<(String, f64, f64)> {
    let src = format!("\\documentclass[10pt]{{article}}\\begin{{document}}{body}\\end{{document}}");
    let r = render_one(&src);
    let mut out = Vec::new();
    for it in r.v2.pages[0].resident_items() {
        if let Item::GlyphRun(run) = it {
            if math && run.role != RunRole::Math {
                continue;
            }
            let mut chars = run
                .clusters
                .iter()
                .map(|c| run.text[c.text_start_byte as usize..c.text_end_byte as usize].to_string());
            for g in &run.glyphs {
                out.push((
                    chars.next().unwrap_or_default(),
                    g.origin_x.to_bp() / BP,
                    g.baseline_y.to_bp() / BP,
                ));
            }
        }
    }
    out
}

/// The `a`, the three dots and the `b` of `$a<cmd> b$`, as `(x, y)` in TeX pt.
fn dots_of(cmd: &str) -> ((f64, f64), Vec<(f64, f64)>, (f64, f64)) {
    let g = glyphs(&format!("$a{cmd} b$ and more text here."), true);
    let find = |t: &str| {
        let m = g.iter().find(|g| g.0 == t).unwrap_or_else(|| panic!("no `{t}` among {g:?}"));
        (m.1, m.2)
    };
    let dots: Vec<(f64, f64)> = g.iter().filter(|g| g.0 == ".").map(|g| (g.1, g.2)).collect();
    assert_eq!(dots.len(), 3, "{cmd}: three periods expected, got {dots:?} in {g:?}");
    (find("a"), dots, find("b"))
}

fn close(what: &str, got: f64, want: f64) {
    assert!(
        (got - want).abs() <= GLYPH_GATE,
        "{what}: {got:.4} pt, pdflatex {want:.4} pt (off by {:+.4}, gate {GLYPH_GATE:.4})",
        got - want
    );
}

// pdflatex's measured positions, converted from the bp of the module header
// to TeX pt (`/ BP`). `A` is the `a`'s origin, which every other coordinate
// below is measured against so the page geometry does not enter.
const A_X: f64 = 148.7120 / BP;
const A_Y: f64 = 139.8010 / BP;

/// `\vdots`: one column of periods at the `a`'s advance, the bottom one on
/// the formula's own baseline, and the `b` one period-width further on with
/// no space of any kind (a `\vbox` in math is an ordinary atom, TeX §1076).
#[test]
fn vdots_is_a_vbox_of_three_text_periods() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let (a, dots, b) = dots_of("\\vdots");
    close("\\vdots a x", a.0, A_X);
    close("\\vdots baseline", a.1, A_Y);
    for (i, want) in [131.8311, 135.8160, 139.8010].into_iter().enumerate() {
        close(&format!("\\vdots dot {i} x"), dots[i].0, 153.9780 / BP);
        close(&format!("\\vdots dot {i} y"), dots[i].1, want / BP);
    }
    close("\\vdots b x", b.0, 156.7476 / BP);
    close("\\vdots b y", b.1, A_Y);
}

/// `\ddots`: three periods stepping down to the right, inside a `\mathinner`,
/// so there is a thin space between the `a` and the group and between the
/// group and the `b` on top of the 1mu kerns of the definition.
#[test]
fn ddots_is_a_mathinner_of_three_raised_text_periods() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let (a, dots, b) = dots_of("\\ddots");
    close("\\ddots a x", a.0, A_X);
    close("\\ddots baseline", a.1, A_Y);
    for (i, (wx, wy)) in [(156.1920, 132.8270), (160.0670, 135.8160), (163.9410, 138.8051)]
        .into_iter()
        .enumerate()
    {
        close(&format!("\\ddots dot {i} x"), dots[i].0, wx / BP);
        close(&format!("\\ddots dot {i} y"), dots[i].1, wy / BP);
    }
    close("\\ddots b x", b.0, 168.9220 / BP);
    close("\\ddots b y", b.1, A_Y);
}

/// The 15.05554pt box is what moves the first baseline: `\topskip` is 10pt in
/// a 10pt document, so a box taller than that pushes the line down by the
/// excess. Both commands move it by the same 5.0553pt, because both boxes are
/// 14pt plus the period's height tall. This is the headline number of #717,
/// and the reason `\vdots`/`\ddots` were held out of
/// `math_symbol_line_height.rs`'s list.
#[test]
fn both_stacks_push_the_first_baseline_down_by_the_box_height() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    // `\showbox`: \vbox(15.05554+0.0), against a 10pt \topskip.
    let want = PLAIN_BASELINE + (15.05554 - 10.0);
    for cmd in ["\\vdots", "\\ddots"] {
        let (a, _, _) = dots_of(cmd);
        close(&format!("{cmd} first baseline"), a.1, want);
    }
    // The control: the same document with a symbol short enough to leave
    // `\topskip` alone is still at the plain baseline.
    let g = glyphs("$a\\ldots b$ and more text here.", true);
    close("\\ldots first baseline", g[0].2, PLAIN_BASELINE);
}

/// The lengths in the definitions are absolute, so they must not scale with
/// the body size: at 12pt pdflatex still sets the periods exactly 4pt apart
/// and raises them exactly 7/4/1pt, while the period and the `mu` kerns grow
/// (`\showbox`, `\vbox(15.16666+0.0)x3.26385` against 10pt's
/// `\vbox(15.05554+0.0)x2.77779`).
#[test]
fn the_kerns_and_raises_are_absolute_lengths() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    // 0.1 bp: these are differences of two measured positions, where the
    // page geometry cancels, so they are pinned harder than a position.
    let gate = 0.1 * BP;
    for size in ["10pt", "12pt"] {
        for (cmd, want) in [("\\vdots", [8.0, 4.0, 0.0]), ("\\ddots", [7.0, 4.0, 1.0])] {
            let src = format!(
                "\\documentclass[{size}]{{article}}\\begin{{document}}$a{cmd} b$ and more text here.\\end{{document}}"
            );
            let r = render_one(&src);
            let mut dots: Vec<(f64, f64)> = Vec::new();
            let mut base = f64::NAN;
            for it in r.v2.pages[0].resident_items() {
                if let Item::GlyphRun(run) = it {
                    if run.role != RunRole::Math {
                        continue;
                    }
                    let mut chars = run.clusters.iter().map(|c| {
                        run.text[c.text_start_byte as usize..c.text_end_byte as usize].to_string()
                    });
                    for g in &run.glyphs {
                        let (t, x, y) = (
                            chars.next().unwrap_or_default(),
                            g.origin_x.to_bp() / BP,
                            g.baseline_y.to_bp() / BP,
                        );
                        if t == "a" {
                            base = y;
                        }
                        if t == "." {
                            dots.push((x, y));
                        }
                    }
                }
            }
            assert_eq!(dots.len(), 3, "{size} {cmd}: {dots:?}");
            for (i, w) in want.into_iter().enumerate() {
                let got = base - dots[i].1;
                assert!(
                    (got - w).abs() <= gate,
                    "{size} {cmd}: dot {i} raised {got:.4} pt, TeX {w:.1} pt (gate {gate:.4})"
                );
            }
        }
    }
}

/// The single-glyph fallback is gone: neither U+22EE nor U+22F1 is painted
/// any more. (This replaces `math_ellipsis.rs`'s
/// `vdots_and_ddots_are_left_alone`, which pinned the opposite.)
#[test]
fn neither_unicode_dots_glyph_is_painted() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    for (body, ch) in [("$a\\vdots b$", "\u{22EE}"), ("$a\\ddots b$", "\u{22F1}")] {
        let g = glyphs(body, false);
        assert!(
            !g.iter().any(|g| g.0 == ch),
            "{body}: {ch:?} still painted as one glyph: {g:?}"
        );
    }
}
