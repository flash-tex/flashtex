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

/// A script paragraph set at a class size other than `\normalsize`:
/// `cmd` is `\large`, `\small` or `\footnotesize` (12/9/8pt at a 10pt
/// base), so the body runs at `body_size` and the scripts at `sf_size`.
fn sized_doc(cmd: &str) -> String {
    format!(
        "\\documentclass[10pt]{{article}}\n\\begin{{document}}\n{{{cmd} E=mc\\textsuperscript{{2}} and H\\textsubscript{{2}}O end.}}\n\\end{{document}}"
    )
}

/// Baseline geometry of the two script runs against the body run holding
/// `mc`: `(body_baseline, super_lift, sub_drop)`, all in bp.
fn script_geometry(rendered: &flashtex_render_pipeline::Rendered) -> (f64, f64, f64) {
    let found = runs(rendered);
    let body_baseline = found
        .iter()
        .find(|(text, _, _)| text.contains("mc"))
        .map(|(_, _, baseline)| *baseline)
        .expect("body run should be present");
    let twos: Vec<_> = found.iter().filter(|(text, _, _)| text == "2").collect();
    assert_eq!(twos.len(), 2, "both script runs present: {found:?}");
    let super_baseline = twos
        .iter()
        .find(|(_, _, baseline)| *baseline < body_baseline)
        .map(|(_, _, baseline)| *baseline)
        .expect("one run above the baseline");
    let sub_baseline = twos
        .iter()
        .find(|(_, _, baseline)| *baseline > body_baseline)
        .map(|(_, _, baseline)| *baseline)
        .expect("one run below the baseline");
    (
        body_baseline,
        body_baseline - super_baseline,
        sub_baseline - body_baseline,
    )
}

/// Slice-2 finding 2: `{\large ...}` is 12pt, scripts at 8pt. The
/// superscript lifts by sup2 of lmsy10 (4.3547pt) and the subscript drops
/// by sub1 of lmsy10 (1.8pt) — the pdflatex `\showbox` oracle numbers.
#[test]
fn text_scripts_at_large_match_oracle() {
    if !lm_available() {
        return;
    }
    let rendered = render_one(&sized_doc("\\large"));
    assert!(unsupported_related(&rendered).is_empty());
    let found = runs(&rendered);
    let body_size = found
        .iter()
        .find(|(text, _, _)| text.contains("mc"))
        .map(|(_, size, _)| *size)
        .expect("body run should be present");
    assert!(
        (body_size - 12.0).abs() < 0.05,
        "body text is 12pt: {body_size}"
    );
    let script_size = found
        .iter()
        .find(|(text, _, _)| text == "2")
        .map(|(_, size, _)| *size)
        .expect("script run should be present");
    assert!(
        (script_size - 8.0).abs() < 0.05,
        "scripts are set at \\sf@size (8pt): {script_size}"
    );
    let (_, lift, drop) = script_geometry(&rendered);
    assert!(
        (lift - 4.3547).abs() < 0.1,
        "superscript raised by sup2 (~4.3547pt), got {lift}pt"
    );
    assert!(
        (drop - 1.8).abs() < 0.1,
        "subscript lowered by sub1 (1.8pt), got {drop}pt"
    );
}

/// Slice-2 finding 2: `\small` is 9pt, scripts at 6pt. The superscript
/// lifts by sup2 of lmsy9 (3.82329pt) and the subscript drops by sub1 of
/// lmsy9 (1.0pt) — the old fixed 0.15em gave 1.35pt here, 0.35pt too low.
#[test]
fn text_scripts_at_small_match_oracle() {
    if !lm_available() {
        return;
    }
    let rendered = render_one(&sized_doc("\\small"));
    assert!(unsupported_related(&rendered).is_empty());
    let found = runs(&rendered);
    let body_size = found
        .iter()
        .find(|(text, _, _)| text.contains("mc"))
        .map(|(_, size, _)| *size)
        .expect("body run should be present");
    assert!(
        (body_size - 9.0).abs() < 0.05,
        "body text is 9pt: {body_size}"
    );
    let script_size = found
        .iter()
        .find(|(text, _, _)| text == "2")
        .map(|(_, size, _)| *size)
        .expect("script run should be present");
    assert!(
        (script_size - 6.0).abs() < 0.05,
        "scripts are set at \\sf@size (6pt): {script_size}"
    );
    let (_, lift, drop) = script_geometry(&rendered);
    assert!(
        (lift - 3.82329).abs() < 0.1,
        "superscript raised by sup2 (~3.82329pt), got {lift}pt"
    );
    assert!(
        (drop - 1.0).abs() < 0.1,
        "subscript lowered by sub1 (1.0pt), got {drop}pt"
    );
}

/// Slice-2 finding 2, height-term arm: `\footnotesize` is 8pt, where sub1
/// of lmsy8 (1.0pt) loses to the box-height term, so the drop is 1.1111pt.
/// The old fixed 0.15em gave 1.2pt — only 0.09pt off, so this case uses a
/// tighter tolerance than the 0.1pt of the cases above.
#[test]
fn text_subscript_at_footnotesize_uses_height_term() {
    if !lm_available() {
        return;
    }
    let rendered = render_one(&sized_doc("\\footnotesize"));
    assert!(unsupported_related(&rendered).is_empty());
    let found = runs(&rendered);
    let body_size = found
        .iter()
        .find(|(text, _, _)| text.contains("mc"))
        .map(|(_, size, _)| *size)
        .expect("body run should be present");
    assert!(
        (body_size - 8.0).abs() < 0.05,
        "body text is 8pt: {body_size}"
    );
    let (_, _, drop) = script_geometry(&rendered);
    assert!(
        (drop - 1.11111).abs() < 0.05,
        "subscript lowered by the height term (1.1111pt), got {drop}pt"
    );
}

/// Slice-2 finding 3: TeX adds `\scriptspace` (0.5pt) to every script
/// box's width. With no glue between the script and the next glyph, the
/// next run's left edge sits exactly content-plus-0.5pt past the script's
/// left edge; `words_of` measures the content width from the glyphs, so
/// the residual is the scriptspace itself (0.0pt before the fix).
#[test]
fn script_space_widens_script_box() {
    if !lm_available() {
        return;
    }
    let rendered = render_one(
        r"\documentclass[10pt]{article}
\begin{document}
A\textsuperscript{2}B
\end{document}",
    );
    assert!(unsupported_related(&rendered).is_empty());
    let words = words_of(&rendered);
    let body = words
        .iter()
        .find(|w| w.text == "A")
        .expect("body run before the script: {words:?}");
    let script = words
        .iter()
        .find(|w| w.text == "2")
        .expect("script run: {words:?}");
    assert!(
        script.baseline < body.baseline,
        "script is raised: {words:?}"
    );
    let after = words
        .iter()
        .filter(|w| w.x > script.x && (w.baseline - body.baseline).abs() < 0.01)
        .min_by(|a, b| a.x.partial_cmp(&b.x).expect("finite"))
        .expect("run after the script: {words:?}");
    let residual = after.x - script.x - script.width;
    assert!(
        (residual - 0.5).abs() < 0.05,
        "next glyph starts content + scriptspace (0.5pt) past the script, residual {residual}pt"
    );
}

/// Slice-2 finding 2 touched the shared subscript formula in the footnotes
/// module: footnote marks (the superscript side of that module) must be
/// unaffected — the mark is still set at `\sf@size` and raised by sup2.
#[test]
fn footnote_mark_still_set_as_superscript() {
    if !lm_available() {
        return;
    }
    let rendered = render_one(
        r"\documentclass[10pt]{article}
\begin{document}
Word\footnote{Note} end.
\end{document}",
    );
    assert!(unsupported_related(&rendered).is_empty());
    let found = runs(&rendered);
    let body_baseline = found
        .iter()
        .find(|(text, _, _)| text.contains("Word"))
        .map(|(_, _, baseline)| *baseline)
        .expect("body run should be present");
    let (mark_size, mark_baseline) = found
        .iter()
        .find(|(text, size, _)| text == "1" && (size - 7.0).abs() < 0.05)
        .map(|(_, size, baseline)| (*size, *baseline))
        .expect("footnote mark at sf@size (7pt): {found:?}");
    assert!(
        (mark_size - 7.0).abs() < 0.05,
        "mark is set at \\sf@size (7pt): {mark_size}"
    );
    let lift = body_baseline - mark_baseline;
    assert!(
        (lift - 3.62892).abs() < 0.1,
        "mark raised by sup2 (~3.63pt), got {lift}pt"
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
