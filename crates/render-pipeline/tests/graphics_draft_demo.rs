//! graphicx `draft` and `demo`: the space a graphic reserves when its file
//! is never read, end to end through a `figure`.
//!
//! Every target below was measured with `pdflatex` (pdfTeX 3.141592653-2.6-
//! 1.40.27, TeX Live 2025) by setting the same `\includegraphics` in an
//! `\hbox` under the same preamble and reporting `\wd`/`\ht`/`\dp`. pdflatex
//! is an oracle only; nothing here runs TeX.
//!
//! In an `article` with `\usepackage[margin=1in]{geometry}` on US Letter,
//! `\linewidth` = `\textwidth` = `\columnwidth` = 469.75502pt.

mod common;

use flashtex_render_pipeline::display::{Item, Rule, Severity};

const TW: f64 = 469.75502;
/// The rules are exact to well under a hundredth of a point; the slack is
/// for `\Gscale@div`'s fixed-point rounding in the oracle, not for ours.
const TOL: f64 = 0.02;

fn bp(pt: f64) -> f64 {
    pt * 72.0 / 72.27
}

fn doc(preamble: &str, graphic: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\usepackage[margin=1in]{{geometry}}\n{preamble}\n\
         \\begin{{document}}\nText before the float.\n\
         \\begin{{figure}}[t]\n\\centering\n{graphic}\n\\caption{{A caption.}}\n\\end{{figure}}\n\
         More text.\n\\end{{document}}\n"
    )
}

/// Every `Rule` on page 1, as (x, top, width, height) in TeX points.
fn rules(source: &str) -> Vec<(f64, f64, f64, f64)> {
    let r = common::render_docs(&[("main.tex", source)], "main.tex");
    assert!(
        !r.v2.diagnostics.iter().any(|d| d.severity == Severity::Error),
        "unexpected error diagnostics: {:?}",
        r.v2.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| &d.message).collect::<Vec<_>>()
    );
    let mut out: Vec<(f64, f64, f64, f64)> = r.v2.pages[0]
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Rule(Rule { x, top, width, height, .. }) => Some((x.to_bp(), top.to_bp(), width.to_bp(), height.to_bp())),
            _ => None,
        })
        .collect();
    out.sort_by(|a, b| a.partial_cmp(b).unwrap());
    out
}

fn near(got: f64, want_pt: f64, what: &str) {
    assert!((got - bp(want_pt)).abs() < TOL, "{what}: {got} bp, expected {} bp ({want_pt} pt)", bp(want_pt));
}

/// `\includegraphics[draft,...]{missing.png}`: `pdftex.def` cannot find the
/// file, leaves the bounding box at `0 0 72 72` and -- because `draft` is on
/// -- warns instead of raising its package error. The graphic therefore
/// still reserves one inch square, scaled by the keys, and `\Gin@setfile`
/// draws a frame of default-thickness rules around it.
#[test]
fn draft_reserves_the_fallback_bounding_box_and_frames_it() {
    if !common::lm_available() {
        return;
    }
    // Measured: \includegraphics[draft,width=0.8\linewidth]{plot.png} is
    // 375.80402pt wide and 375.82394pt tall (\Gscale@div rounding), depth 0.
    let side = 0.8 * TW;
    let got = rules(&doc("\\usepackage{graphicx}", "\\includegraphics[draft,width=0.8\\linewidth]{plot.png}"));
    assert_eq!(got.len(), 4, "a frame is four rules: {got:?}");
    let (widths, heights): (Vec<f64>, Vec<f64>) = (got.iter().map(|r| r.2).collect(), got.iter().map(|r| r.3).collect());
    // Two \hrules the full width, two \vrules the full height, all 0.4pt.
    let mut spans = widths.clone();
    spans.extend(heights.iter().copied());
    assert_eq!(spans.iter().filter(|v| (**v - bp(0.4)).abs() < 1e-6).count(), 4, "four 0.4pt rules: {got:?}");
    for w in widths.iter().filter(|w| **w > bp(1.0)) {
        near(*w, side, "frame width");
    }
    for h in heights.iter().filter(|h| **h > bp(1.0)) {
        near(*h, side, "frame height");
    }
    // The frame is square and closed: left/right edges span the same rows,
    // top/bottom the same columns.
    let (l, r) = (got[0].0, got.iter().map(|r| r.0 + r.2).fold(f64::MIN, f64::max));
    near(r - l, side, "frame extent");
}

/// The `draft` *package* option -- and the class option LaTeX passes to
/// every package -- does the same thing, and a per-image `draft=false` or a
/// `final` turns it off again, which brings back the package error for a
/// file that is not there.
#[test]
fn the_draft_package_option_and_the_per_image_override() {
    if !common::lm_available() {
        return;
    }
    let side = 0.8 * TW;
    let got = rules(&doc("\\usepackage[draft]{graphicx}", "\\includegraphics[width=0.8\\linewidth]{plot.png}"));
    assert_eq!(got.len(), 4, "{got:?}");
    for w in got.iter().map(|r| r.2).filter(|w| *w > bp(1.0)) {
        near(w, side, "package-option draft width");
    }
    // `\documentclass[draft]` reaches graphicx the same way.
    let class = doc("\\usepackage{graphicx}", "\\includegraphics[width=0.8\\linewidth]{plot.png}")
        .replace("\\documentclass{article}", "\\documentclass[draft]{article}");
    assert_eq!(rules(&class).len(), 4, "class option draft");
    // A per-image `draft=false` turns it off again for that graphic, and
    // so does a `final` anywhere in the options -- and then a file that is
    // not there is `pdftex.def`'s package error once more.
    for src in [
        doc("\\usepackage[draft]{graphicx}", "\\includegraphics[draft=false,width=0.8\\linewidth]{plot.png}"),
        doc("\\usepackage[draft,final]{graphicx}", "\\includegraphics[width=0.8\\linewidth]{plot.png}"),
        doc("\\usepackage[final,draft]{graphicx}", "\\includegraphics[width=0.8\\linewidth]{plot.png}"),
    ] {
        let r = common::render_docs(&[("main.tex", &src)], "main.tex");
        assert!(
            r.v2.diagnostics.iter().any(|d| d.severity == Severity::Error && d.code == "image_unavailable"),
            "expected the missing file to be an error again: {src}"
        );
    }
}

/// `demo` replaces the whole of `\Ginclude@graphics` with
/// `\rule{\Gin@@ewidth or 150pt}{\Gin@@eheight or 100pt}`: no file is looked
/// up, the rule is solid, and the height stays at 100pt however wide the
/// graphic is asked to be.
#[test]
fn demo_sets_a_solid_rule_whose_height_is_pinned() {
    if !common::lm_available() {
        return;
    }
    for (opts, want_w, want_h) in [
        ("", 150.0, 100.0),
        ("width=0.6\\textwidth", 0.6 * TW, 100.0),
        ("width=100pt,height=40pt", 100.0, 40.0),
        // `keepaspectratio` never reaches `\Gin@req@sizes`, so it does not
        // shrink the rule the way it would shrink a real image.
        ("keepaspectratio,width=100pt,height=40pt", 100.0, 40.0),
        // Nor does a leading `scale`.
        ("scale=2", 150.0, 100.0),
        // `demo` beats a per-image `draft`: `\Gin@setfile` is never reached.
        ("draft,width=0.6\\textwidth", 0.6 * TW, 100.0),
    ] {
        let g = if opts.is_empty() {
            "\\includegraphics{pipeline-diagram}".to_string()
        } else {
            format!("\\includegraphics[{opts}]{{pipeline-diagram}}")
        };
        // The file has no extension and does not exist; `demo` never looks.
        let got = rules(&doc("\\usepackage[demo]{graphicx}", &g));
        assert_eq!(got.len(), 1, "[{opts}] is one solid rule: {got:?}");
        near(got[0].2, want_w, &format!("[{opts}] width"));
        near(got[0].3, want_h, &format!("[{opts}] height"));
    }
}
