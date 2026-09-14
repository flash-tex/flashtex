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

/// `\usepackage{amsmath}`, because these are amsmath's constructs.
///
/// This helper used to write a bare `\documentclass{article}` preamble, which
/// made nine of the eleven fixtures below documents real LaTeX rejects. Checked
/// against pdflatex (TeX Live 2025) as an oracle: without the package,
/// `\binom`, `\dfrac`, `\tfrac`, `\substack`, `\operatorname`, `\operatorname*`
/// and `\genfrac` are "Undefined control sequence" and `smallmatrix` is
/// "Environment smallmatrix undefined". Only `\bigl`/`\bigr` and `\phantom`
/// are kernel commands that compile bare. An engine whose contract is matching
/// pdflatex cannot claim a construct "typesets cleanly" from a document
/// pdflatex refuses to typeset at all.
const AMSMATH: &str = "\\usepackage{amsmath}";
/// `dcases` is mathtools', not amsmath's: pdflatex still reports "Environment
/// dcases undefined" with amsmath alone.
const MATHTOOLS: &str = "\\usepackage{amsmath}\\usepackage{mathtools}";

fn doc_with(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}{preamble}\\begin{{document}}{body}\\end{{document}}")
}

/// Renders `body` under `AMSMATH` and returns every diagnostic code/message
/// pair plus whether at least one math glyph reached the display list (so a
/// clean diagnostic list is not merely a silently empty formula).
fn render(body: &str) -> (Vec<(String, String)>, bool) {
    let (diags, has_math_glyph) = render_preamble(AMSMATH, body);
    (diags.into_iter().map(|(c, m, _)| (c, m)).collect(), has_math_glyph)
}

/// As `render`, but each diagnostic also carries whether any of its source
/// ranges overlaps `body` (as opposed to the preamble).
fn render_preamble(preamble: &str, body: &str) -> (Vec<(String, String, bool)>, bool) {
    let text = doc_with(preamble, body);
    let start = text.find(body).expect("the body is written into the document verbatim");
    let end = start + body.len();
    let r = render_one(&text);
    let diags = r
        .v2
        .diagnostics
        .iter()
        .map(|d| {
            let in_body = d.sources.iter().any(|s| s.start_byte < end && s.end_byte > start);
            (d.code.clone(), d.message.clone(), in_body)
        })
        .collect();
    let has_math_glyph = r.v2.pages.iter().flat_map(|p| p.items.iter()).any(|it| matches!(it, Item::GlyphRun(run) if run.role == flashtex_render_pipeline::display::RunRole::Math));
    (diags, has_math_glyph)
}

/// Codes meaning the construct did not typeset as itself.
///
/// `math_limitation` and the pipeline's own `unsupported_*` codes were always
/// reachable here. The three compiler codes were not: until
/// `display::Diagnostic::from_compiler` stopped rewriting every compiler code
/// to the literal `"compiler"`, no compiler diagnostic could match any
/// predicate in this file, so this gate was structurally unable to fail on
/// one. `unknown_command` and `syntax_error` are spelled out because they do
/// not share the `unsupported` prefix and are exactly how an unrecognised
/// construct arrives.
fn is_failure_code(code: &str) -> bool {
    code == "math_limitation"
        || code.starts_with("unsupported")
        || code == "unknown_command"
        || code == "syntax_error"
}

fn assert_no_limitation_or_unsupported(construct: &str, body: &str) {
    assert_clean(construct, AMSMATH, body);
}

/// Asserts `body` typesets with no diagnostic saying the construct itself did
/// not work.
///
/// Scoped to diagnostics whose source range overlaps `body`. The preamble
/// raises its own `unsupported_feature` warnings -- "packages amsmath are
/// recognised but not implemented", spanning the `\usepackage` line -- which
/// are about package machinery, not about whether `\binom` set correctly, and
/// would otherwise make every fixture here unassertable the moment it declares
/// the package it needs. A construct-level `unsupported_feature` still fails.
fn assert_clean(construct: &str, preamble: &str, body: &str) {
    let (diags, has_math_glyph) = render_preamble(preamble, body);
    let flagged: Vec<&(String, String, bool)> = diags.iter().filter(|(code, _, in_body)| *in_body && is_failure_code(code)).collect();
    assert!(flagged.is_empty(), "{construct} ({body:?}) should have no math_limitation/unsupported/unknown/syntax diagnostic on the body, got {flagged:?}");
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
    let (diags, has_math_glyph) = render_preamble(AMSMATH, "$a\\phantom{x}b$");
    let flagged: Vec<&(String, String, bool)> = diags.iter().filter(|(code, _, in_body)| *in_body && is_failure_code(code)).collect();
    assert!(flagged.is_empty(), "\\phantom should have no math_limitation/unsupported/unknown/syntax diagnostic on the body, got {flagged:?}");
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
    // mathtools, not amsmath: pdflatex reports "Environment dcases undefined"
    // with amsmath alone.
    assert_clean("dcases", MATHTOOLS, "$\\begin{dcases}1&x>0\\\\0&x\\le0\\end{dcases}$");
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
