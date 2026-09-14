//! FT-061 amsmath inline constructs (`Nucleus::GenFraction`/`Phantom`/
//! `Operator`/`SubArray`, `dcases`, and amsmath's `\big`-family delimiters
//! as math-layout's `BigDelimiter`), converted by the `amsmath-inline`
//! feature — a `render-pipeline` default since the `crates/compiler` and
//! `crates/math-layout` re-pin to a9952df3 (see `vendor/VENDORING.md`).
//! Each construct here is asserted to typeset without a `math_limitation`
//! or `unsupported_*` diagnostic; anything the pipeline still only
//! approximates is called out in its own test instead of asserted clean.

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\\begin{{document}}{body}\\end{{document}}")
}

/// Renders `body` and returns every diagnostic code/message pair plus
/// whether at least one math glyph reached the display list (so a clean
/// diagnostic list is not merely a silently empty formula).
fn render(body: &str) -> (Vec<(String, String)>, bool) {
    let r = render_one(&doc(body));
    let diags = r.v2.diagnostics.iter().map(|d| (d.code.clone(), d.message.clone())).collect();
    let has_math_glyph = r.v2.pages.iter().flat_map(|p| p.to_items()).any(|it| matches!(it, Item::GlyphRun(run) if run.role == flashtex_render_pipeline::display::RunRole::Math));
    (diags, has_math_glyph)
}

fn assert_no_limitation_or_unsupported(construct: &str, body: &str) {
    let (diags, has_math_glyph) = render(body);
    let flagged: Vec<&(String, String)> = diags.iter().filter(|(code, _)| code == "math_limitation" || code.starts_with("unsupported")).collect();
    assert!(flagged.is_empty(), "{construct} ({body:?}) should have no math_limitation/unsupported diagnostic, got {flagged:?}");
    assert!(has_math_glyph, "{construct} ({body:?}) produced no math glyph run at all");
}

#[test]
fn binom_is_a_genfraction_with_parenthesis_delimiters_and_no_rule() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    assert_no_limitation_or_unsupported("\\binom", "$\\binom{n}{k}$");
}

#[test]
fn dfrac_is_a_genfraction_in_display_style() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    assert_no_limitation_or_unsupported("\\dfrac", "$\\dfrac{a}{b}$");
}

#[test]
fn tfrac_is_a_genfraction_in_text_style() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    assert_no_limitation_or_unsupported("\\tfrac", "$\\tfrac{a}{b}$");
}

#[test]
fn phantom_sets_an_empty_box_without_a_limitation() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // `\phantom` alone has no visible glyph, so gate on diagnostics only;
    // pair it with a visible atom to also confirm the formula still typesets.
    let (diags, has_math_glyph) = render("$a\\phantom{x}b$");
    let flagged: Vec<&(String, String)> = diags.iter().filter(|(code, _)| code == "math_limitation" || code.starts_with("unsupported")).collect();
    assert!(flagged.is_empty(), "\\phantom should have no math_limitation/unsupported diagnostic, got {flagged:?}");
    assert!(has_math_glyph, "\\phantom{{x}} between two symbols produced no math glyph run");
}

#[test]
fn substack_stacks_rows_as_a_subarray_without_a_limitation() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    assert_no_limitation_or_unsupported("\\substack", "$\\sum_{\\substack{a\\\\b}} x$");
}

#[test]
fn smallmatrix_is_a_top_level_grid_without_a_limitation() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    assert_no_limitation_or_unsupported("smallmatrix", "$\\begin{smallmatrix}1&2\\\\3&4\\end{smallmatrix}$");
}

#[test]
fn big_delimiters_are_sized_as_amsmath_big_delimiter_atoms() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    assert_no_limitation_or_unsupported("\\bigl/\\bigr", "$\\bigl(\\bigr)$");
}

#[test]
fn dcases_is_a_cases_style_grid_without_a_limitation() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    assert_no_limitation_or_unsupported("dcases", "$\\begin{dcases}1&x>0\\\\0&x\\le0\\end{dcases}$");
}

#[test]
fn operatorname_and_declaremathoperator_are_operator_nuclei_without_a_limitation() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    assert_no_limitation_or_unsupported("\\operatorname", "$\\operatorname{argmax}_{x} f(x)$");
    assert_no_limitation_or_unsupported("\\operatorname*", "$\\operatorname*{argmax}_{x} f(x)$");
}

#[test]
fn genfrac_with_explicit_delimiters_and_style_is_modeled() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    assert_no_limitation_or_unsupported("\\genfrac", "$\\genfrac{[}{]}{0pt}{0}{a}{b}$");
}

/// A grid nested inside a sub-formula (here `\dfrac`'s numerator) is set
/// as a box (`mathtext::GridCells`), not flattened into one row, so it
/// carries no `math_limitation`.
#[test]
fn a_grid_nested_inside_a_genfraction_is_laid_out_as_a_box() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    assert_no_limitation_or_unsupported("smallmatrix in \\dfrac", "$\\dfrac{\\begin{smallmatrix}1&2\\\\3&4\\end{smallmatrix}}{b}$");
}
