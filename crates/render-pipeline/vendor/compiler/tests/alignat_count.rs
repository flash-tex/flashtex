//! amsmath `alignat`/`alignat*` column-pair count is a TeX undelimited
//! argument, so `\begin{alignat}3` parses exactly like
//! `\begin{alignat}{3}`: no diagnostic, no stray `3` in the output, and
//! identical glyph origins.
//!
//! Oracle (all values measured, not assumed): TeX Live 2026 pdflatex
//! (`/Library/TeX/texbin/pdflatex -interaction=nonstopmode t.tex`, with
//! `article.cls` and `amsmath.sty` found via `kpsewhich`) reports 0 errors
//! for both forms on the body
//! `\begin{alignat*}X a&=b, &\quad c&=d\\ e&=f, & g&=h\end{alignat*}`
//! (X = `3` vs `{3}` on separate pages), and per-glyph x positions from
//! `mutool draw -F stext t.pdf` are identical on both pages:
//! a at x=275.618bp, c at x=312.30027bp, e at x=276.246bp, g at x=311.50367bp.
//! The committed assertions below check the same property on FlashTeX's side
//! (braced and unbraced lay out identically, origins within 0.1bp); absolute
//! bp equality with pdflatex is not asserted because FlashTeX's math glyph
//! metrics already differ from Computer Modern by ~1bp per glyph on plain
//! `$a=b$` (pre-existing, outside this slice).
use flashtex_compiler::layout::layout;
use flashtex_compiler::parser::parse;

fn doc(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\\usepackage{{amsmath}}\\begin{{document}}{body}\\end{{document}}"
    )
}

/// Laid-out `(text, x_pt)` items; asserts the body reports no diagnostics.
fn laid_out(body: &str) -> Vec<(String, f64)> {
    let parsed = parse(&doc(body));
    let diags: Vec<String> = parsed
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    assert!(diags.is_empty(), "{body}: {diags:?}");
    let items: Vec<(String, f64)> = layout(&parsed.blocks)
        .iter()
        .flat_map(|page| page.items.iter())
        .map(|item| (item.text.clone(), item.x_pt))
        .collect();
    assert!(!items.is_empty(), "{body} laid out nothing");
    items
}

fn diagnostics(body: &str) -> Vec<String> {
    parse(&doc(body))
        .diagnostics
        .into_iter()
        .map(|d| d.message)
        .collect()
}

const ROWS: &str = "a&=b, &\\quad c&=d\\\\ e&=f, & g&=h";

/// pdflatex typesets `{3}` and `3` identically, so FlashTeX must too: same
/// glyph stream and every origin within 0.1bp (in practice exactly equal).
#[test]
fn unbraced_count_matches_braced() {
    for env in ["alignat", "alignat*"] {
        let braced = laid_out(&format!("\\begin{{{env}}} {{3}} {ROWS} \\end{{{env}}}"));
        let unbraced = laid_out(&format!("\\begin{{{env}}}3 {ROWS} \\end{{{env}}}"));
        assert_eq!(
            braced.len(),
            unbraced.len(),
            "{env}: item counts differ: {braced:?} vs {unbraced:?}"
        );
        let mut worst = 0.0f64;
        for ((b_text, b_x), (u_text, u_x)) in braced.iter().zip(unbraced.iter()) {
            assert_eq!(b_text, u_text, "{env}: glyph streams differ");
            worst = worst.max((b_x - u_x).abs());
        }
        assert!(
            worst <= 0.1,
            "{env}: origins drift by {worst}bp (> 0.1bp): {braced:?} vs {unbraced:?}"
        );
    }
}

/// The unbraced count is consumed as the argument: no diagnostic and no
/// stray `3` typeset before the first cell.
#[test]
fn unbraced_count_leaves_no_stray_digit() {
    let body = format!("\\begin{{alignat*}}3 {ROWS} \\end{{alignat*}}");
    assert_eq!(diagnostics(&body), Vec::<String>::new(), "{body}");
    let texts: String = laid_out(&body).into_iter().map(|(text, _)| text).collect();
    assert!(
        !texts.contains('3'),
        "stray count typeset in {texts:?} for {body}"
    );
}

/// A genuinely missing count keeps the old recovery: the braced-argument
/// diagnostic fires and the environment still closes on its own `\end`
/// (the `\end` must not be swallowed as the argument).
#[test]
fn missing_count_still_reports_braced_argument() {
    for body in [
        "\\begin{alignat*}\\end{alignat*}",
        "\\begin{alignat*} \\end{alignat*}",
        "\\begin{alignat}\\end{alignat}",
    ] {
        let found = diagnostics(body);
        assert!(
            found
                .iter()
                .any(|m| m == "\\alignat requires a braced argument"),
            "{body}: expected the braced-argument error, got {found:?}"
        );
    }
}
