//! `\quad`/`\qquad`/`\,`/`\:`/`\;`/`\!` in math: the pinned math-layout has
//! no kern atom, so `math_box` splits a formula at its top-level spaces into
//! runs joined by kerns of the requested width plus the inter-atom spacing
//! TeX keeps across glue (§760: glue does not reset `r_type`). Glue inside
//! `\left...\right` or a sub-formula stays a typed `math_limitation`.

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\\begin{{document}}{body}\\end{{document}}")
}

/// Every math glyph of page 1 as `(char, origin x, advance)` in TeX points,
/// plus the diagnostics.
fn glyphs(body: &str) -> (Vec<(char, f64, f64)>, Vec<String>) {
    let r = render_one(&doc(body));
    let mut out = Vec::new();
    for item in &r.v2.pages[0].to_items() {
        if let Item::GlyphRun(run) = item {
            if run.role != flashtex_render_pipeline::display::RunRole::Math {
                continue;
            }
            let chars: Vec<char> = run.text.chars().collect();
            for (g, c) in run.glyphs.iter().zip(chars) {
                out.push((c, g.origin_x.to_bp() * 72.27 / 72.0, g.advance_x.to_bp() * 72.27 / 72.0));
            }
        }
    }
    let diags = r.v2.diagnostics.iter().map(|d| format!("{}:{}", d.code, d.message)).collect();
    (out, diags)
}

fn gap(glyphs: &[(char, f64, f64)], left: char, right: char) -> f64 {
    let l = glyphs.iter().find(|g| g.0 == left).unwrap_or_else(|| panic!("no {left:?} in {glyphs:?}"));
    let r = glyphs.iter().find(|g| g.0 == right).unwrap_or_else(|| panic!("no {right:?} in {glyphs:?}"));
    r.1 - (l.1 + l.2)
}

#[test]
fn quad_and_qquad_separate_runs_by_whole_ems() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // 10pt body: 1em = 10pt (the family-2 quad), 2em = 20pt. `a` and `b`
    // are Ord: no inter-atom spacing across the glue. `a`'s italic
    // correction is not in play (Ord followed by glue), so the gap is the
    // kern alone.
    let (plain, _) = glyphs("$ab$");
    let (quad, diags) = glyphs("$a\\quad b$");
    let (qquad, _) = glyphs("$a\\qquad b$");
    let base = gap(&plain, 'a', 'b');
    assert!((gap(&quad, 'a', 'b') - base - 10.0).abs() < 0.02, "\\quad adds 1em: {}", gap(&quad, 'a', 'b') - base);
    assert!((gap(&qquad, 'a', 'b') - base - 20.0).abs() < 0.02, "\\qquad adds 2em: {}", gap(&qquad, 'a', 'b') - base);
    assert!(!diags.iter().any(|d| d.starts_with("math_limitation")), "top-level glue is set, not reported: {diags:?}");
}

#[test]
fn thin_medium_thick_and_negative_spaces_are_mu_fractions() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // 1mu = quad/18 with TeX's scaled-point truncation (10pt: 0.55554pt).
    let mu = ((10.0f64 * 65536.0).round() / 18.0).trunc() / 65536.0;
    let (plain, _) = glyphs("$ab$");
    let base = gap(&plain, 'a', 'b');
    for (cmd, mus) in [("\\,", 3.0), ("\\:", 4.0), ("\\;", 5.0), ("\\!", -3.0)] {
        let (g, diags) = glyphs(&format!("$a{cmd} b$"));
        let got = gap(&g, 'a', 'b') - base;
        assert!((got - mus * mu).abs() < 0.02, "{cmd}: {got} vs {}", mus * mu);
        assert!(!diags.iter().any(|d| d.starts_with("math_limitation")), "{cmd}: {diags:?}");
    }
}

#[test]
fn inter_atom_spacing_survives_across_the_glue() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // `a \quad = b`: TeX inserts \thickmuskip (5mu) between the Ord and the
    // Rel as well as the quad, since glue does not reset r_type.
    let mu = ((10.0f64 * 65536.0).round() / 18.0).trunc() / 65536.0;
    let (rel, _) = glyphs("$a=b$");
    let (spaced, _) = glyphs("$a\\quad=b$");
    let got = gap(&spaced, 'a', '=') - gap(&rel, 'a', '=');
    assert!((got - 10.0).abs() < 0.02, "the thick space is kept on both sides, the quad added once: {got}");
    // `a\quad b` vs `a=b`: the plain formula already has 2 x 5mu.
    assert!((gap(&rel, 'a', '=') - (gap(&glyphs("$ab$").0, 'a', 'b') + 5.0 * mu)).abs() < 0.3, "sanity: Ord-Rel spacing is one thick space");
}

/// With `amsmath-inline`, glue inside a fence pair or a sub-formula is a
/// math-layout `Glue` atom: set at its width, never reported.
#[cfg(feature = "amsmath-inline")]
#[test]
fn glue_inside_a_fence_pair_is_set() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let (plain, _) = glyphs("$\\left( a b \\right)$");
    let (g, diags) = glyphs("$\\left( a \\quad b \\right)$");
    assert!(!diags.iter().any(|d| d.starts_with("math_limitation")), "{diags:?}");
    // `\quad` is 18mu of the 10pt family-2 quad (10pt).
    let got = gap(&g, 'a', 'b') - gap(&plain, 'a', 'b');
    assert!((got - 10.0).abs() < 0.02, "quad inside the fences: {got}");
    let (_, diags) = glyphs("$\\frac{a\\quad b}{c}$");
    assert!(!diags.iter().any(|d| d.contains("sub-formula")), "{diags:?}");
}

#[cfg(not(feature = "amsmath-inline"))]
#[test]
fn glue_inside_a_fence_pair_stays_a_typed_limitation() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let (_, diags) = glyphs("$\\left( a \\quad b \\right)$");
    assert!(
        diags.iter().any(|d| d.starts_with("math_limitation:") && d.contains("inside \\left...\\right or a sub-formula")),
        "{diags:?}"
    );
    let (_, diags) = glyphs("$\\frac{a\\quad b}{c}$");
    assert!(diags.iter().any(|d| d.contains("sub-formula")), "{diags:?}");
    let (_, diags) = glyphs("$\\left( a \\right) \\quad b$");
    assert!(!diags.iter().any(|d| d.starts_with("math_limitation")), "glue after a closed fence pair is top-level: {diags:?}");
}
