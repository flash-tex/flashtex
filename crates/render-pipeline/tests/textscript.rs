//! GH-497 (GH-TEXTSUPERSCRIPT): `\textsuperscript{text}` and
//! `\textsubscript{text}` errored `unsupported_feature` in text mode.
//! Both are kernel text commands: the argument is set at the `\sf@size`
//! of the current size (the `\DeclareMathSizes` table, like footnote
//! marks), raised or lowered like a math script of an empty nucleus.
mod common;

use common::*;
use flashtex_render_pipeline::display::Item;

fn script_doc() -> String {
    r"\documentclass[10pt]{article}
\begin{document}
E=mc\textsuperscript{2} and H\textsubscript{2}O end.
\end{document}"
        .to_string()
}

fn unsupported_related(rendered: &flashtex_render_pipeline::Rendered) -> Vec<String> {
    rendered
        .v2
        .diagnostics
        .iter()
        .filter(|d| {
            d.code.contains("unsupported")
                || d.message.contains("textsuperscript")
                || d.message.contains("textsubscript")
        })
        .map(|d| format!("[{}] {}", d.code, d.message))
        .collect()
}

fn runs(rendered: &flashtex_render_pipeline::Rendered) -> Vec<(String, f64, f64)> {
    rendered.v2.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::GlyphRun(run) => {
                let baseline = run
                    .glyphs
                    .first()
                    .map(|g| g.baseline_y.to_bp())
                    .unwrap_or(f64::NAN);
                Some((run.text.clone(), run.font_size.to_bp(), baseline))
            }
            _ => None,
        })
        .collect()
}

/// Issue #497: both commands must typeset with no diagnostics.
#[test]
fn text_scripts_typeset_with_no_diagnostics() {
    if !lm_available() {
        return;
    }
    let rendered = render_one(&script_doc());
    let bad = unsupported_related(&rendered);
    assert!(
        bad.is_empty(),
        "text scripts should be implemented: {bad:?} (all: {:?})",
        rendered
            .v2
            .diagnostics
            .iter()
            .map(|d| format!("[{}] {}", d.code, d.message))
            .collect::<Vec<_>>()
    );
}

/// Geometry, asserted numerically: the script runs are set smaller than
/// the body, the superscript sits above the body baseline and the
/// subscript below it.
#[test]
fn text_scripts_set_smaller_and_shift_baselines() {
    if !lm_available() {
        return;
    }
    let rendered = render_one(&script_doc());
    assert!(
        unsupported_related(&rendered).is_empty(),
        "no diagnostics expected: {:?}",
        rendered.v2.diagnostics
    );
    let found = runs(&rendered);
    let body_baseline = found
        .iter()
        .find(|(text, _, _)| text.contains("mc"))
        .map(|(_, _, baseline)| *baseline)
        .expect("body run should be present");
    let body_size = found
        .iter()
        .find(|(text, _, _)| text.contains("mc"))
        .map(|(_, size, _)| *size)
        .expect("body run should be present");
    assert!(
        (body_size - 10.0).abs() < 0.05,
        "body text is 10pt: {body_size}"
    );
    // The superscript `2`: `\sf@size` of 10pt is 7pt, raised by `sup2`
    // (~3.63pt at 10pt for an empty nucleus and a digit with no depth).
    let twos: Vec<_> = found.iter().filter(|(text, _, _)| text == "2").collect();
    assert_eq!(twos.len(), 2, "both script runs present: {found:?}");
    let (_, super_size, super_baseline) = twos
        .iter()
        .find(|(_, _, baseline)| *baseline < body_baseline)
        .expect("one run above the baseline");
    assert!(
        (super_size - 7.0).abs() < 0.05,
        "superscript is set at \\sf@size (7pt): {super_size}"
    );
    let super_lift = body_baseline - super_baseline;
    assert!(
        (super_lift - 3.63).abs() < 0.3,
        "superscript raised by sup2 (~3.63pt), got {super_lift}pt"
    );
    // The subscript `2`: same size, lowered (sub1 = 1.5pt at 10pt wins
    // over the digit's own height term).
    let (_, sub_size, sub_baseline) = twos
        .iter()
        .find(|(_, _, baseline)| *baseline > body_baseline)
        .expect("one run below the baseline");
    assert!(
        (sub_size - 7.0).abs() < 0.05,
        "subscript is set at \\sf@size (7pt): {sub_size}"
    );
    let sub_drop = sub_baseline - body_baseline;
    assert!(
        (sub_drop - 1.5).abs() < 0.3,
        "subscript lowered by sub1 (1.5pt), got {sub_drop}pt"
    );
}

/// The baseline returns to normal immediately after the script: the word
/// following `\textsuperscript` sits on the body baseline at body size.
#[test]
fn baseline_returns_to_normal_after_text_script() {
    if !lm_available() {
        return;
    }
    let rendered = render_one(&script_doc());
    assert!(unsupported_related(&rendered).is_empty());
    let found = runs(&rendered);
    let body_baseline = found
        .iter()
        .find(|(text, _, _)| text.contains("mc"))
        .map(|(_, _, baseline)| *baseline)
        .expect("body run should be present");
    let after = found
        .iter()
        .find(|(text, _, _)| text.contains("and"))
        .expect("word after the superscript should be present");
    assert!(
        (after.2 - body_baseline).abs() < 0.01,
        "text after the script is back on the baseline: {after:?}"
    );
    assert!(
        (after.1 - 10.0).abs() < 0.05,
        "text after the script is back at body size: {after:?}"
    );
}

/// Math-mode scripts are untouched: `$x^2$` still typesets with no
/// diagnostics alongside the new text commands.
#[test]
fn math_mode_scripts_are_unaffected() {
    if !lm_available() {
        return;
    }
    let rendered = render_one(
        r"\documentclass[10pt]{article}
\begin{document}
$x^2_3$ and \textsuperscript{2} end.
\end{document}",
    );
    assert!(
        unsupported_related(&rendered).is_empty(),
        "no diagnostics expected: {:?}",
        rendered.v2.diagnostics
    );
    let found = runs(&rendered);
    assert!(
        found.iter().any(|(text, _, _)| text.contains('x')),
        "math nucleus still present: {found:?}"
    );
    assert!(
        found.iter().any(|(text, _, _)| text == "2"),
        "text superscript still present: {found:?}"
    );
}
