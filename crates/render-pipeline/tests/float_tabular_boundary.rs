//! Where a float or a `tabular` hands the vertical list back to the body,
//! measured against pdflatex at 10, 11 and 12 pt.
//!
//! This is the gate for the corpus finding filed as **F12** ("float /
//! `tabular` boundaries", `docs/evidence/corpus-fidelity-2026-09-16T1130Z`),
//! and it is here because that finding turned out **not** to be about floats
//! or about `tabular`. Every skip the boundary actually spends —
//! `\intextsep`, `\textfloatsep`, `\abovecaptionskip`, the `\vcenter` split
//! of the `tabular` box into height and depth, and the float `\vbox`'s own
//! height — is exact. What the corpus measured is the *following line*.
//!
//! ## Why a deep box exposes a glyph height
//!
//! TeX's `append_to_vlist` (§679) puts a box's baseline at `prev + prevdepth
//! + g + height(box)`, where `g` is `\baselineskip - prevdepth -
//! height(box)` unless that falls below `\lineskiplimit`, in which case `g`
//! is `\lineskip`. On the normal branch the `height` cancels and the
//! baseline lands `\baselineskip` below the last one, whatever the glyphs
//! are. A `tabular` is `\vcenter`ed, so a four-row one is 22 pt deep, `g`
//! goes negative, `\lineskip` is used instead — and the next baseline now
//! carries `height(box)` directly. The same thing happens under any deep
//! box: `figure-rule-*` here uses no `tabular` at all.
//!
//! ## What the residual is
//!
//! A line's height is its tallest glyph's `charht`. pdfLaTeX without
//! `lmodern` sets text from OT1 `cmr*`; this pipeline measures with T1
//! `ec-lmr*` (`fonts::latin_modern_tfm`). Latin Modern is metric-compatible
//! with Computer Modern in *widths* only — its heights differ, so the two
//! disagree exactly where `\lineskip` applies (`tftopl`, TeX Live 2026):
//!
//! | glyph | `cmr10` | `ec-lmr10` | `cmr12` | `ec-lmr12` |
//! |---|---|---|---|---|
//! | `h`, `l` (ascender) | 0.694445 | 0.688875 | 0.694444 | 0.688874 |
//! | digits | 0.644444 | 0.629724 | 0.644444 | 0.629729 |
//! | `x` | 0.430555 | 0.430550 | 0.430556 | 0.430556 |
//! | `(` | 0.750000 | 0.750000 | 0.750000 | 0.750000 |
//!
//! So `tabular-follower-xheight` and `tabular-follower-paren` are exact and
//! `tabular-follower-ascender` / `-digits` are not, by the difference above
//! times the body size. That is the same defect #754 fixed for math family 0
//! (`rm-lmr*` where the kernel declares `cmr*`), in the text path; it is
//! **not** fixed here. The expected residuals are computed from the table,
//! not from this engine's output, so the day the text metrics are corrected
//! this test fails and says so instead of quietly following.
//!
//! Every other fixture asserts 0 within the 0.1 bp rule gate.
//!
//! Fixtures: `fixtures/float-tabular-boundary/*.tex`, expected data in
//! `expected/*.txt` — every baseline pdfTeX set on the page, read from its
//! own PDF's content stream. **reference_engine: pdfTeX 1.40.29 (TeX Live
//! 2026)**, `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, two passes. No TeX
//! runs here.

mod common;

use common::lm_available;
use flashtex_render_pipeline::display::Item;

/// One expected baseline: pdfTeX's position, the residual this engine is
/// expected to sit at, and the tolerance.
struct Row {
    baseline: f64,
    residual: f64,
    tol: f64,
    word: String,
}

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/float-tabular-boundary")
}

fn expected(name: &str) -> Vec<Row> {
    let data = std::fs::read_to_string(dir().join("expected").join(format!("{name}.txt"))).unwrap();
    data.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.splitn(4, ' ').collect();
            Row {
                baseline: f[0].parse().unwrap(),
                residual: f[1].parse().unwrap(),
                tol: f[2].parse().unwrap(),
                word: f[3].to_string(),
            }
        })
        .collect()
}

/// Every distinct glyph baseline on the page, in order. A word never spans
/// two baselines, so grouping by baseline is the same partition the oracle
/// made on the reference side.
fn baselines(r: &flashtex_render_pipeline::Rendered) -> Vec<f64> {
    let mut ys: Vec<f64> = Vec::new();
    for page in &r.v2.pages {
        for it in page.resident_items() {
            let Item::GlyphRun(run) = it else { continue };
            let Some(first) = run.glyphs.first() else { continue };
            let y = first.baseline_y.to_bp();
            if !ys.iter().any(|v| (v - y).abs() < 0.001) {
                ys.push(y);
            }
        }
    }
    ys.sort_by(f64::total_cmp);
    ys
}

fn fixtures() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir())
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".tex"))
        .map(|n| n.trim_end_matches(".tex").to_string())
        .collect();
    names.sort();
    names
}

#[test]
fn a_float_or_tabular_boundary_lands_where_pdflatex_lands_it() {
    if !lm_available() {
        return;
    }
    let names = fixtures();
    assert_eq!(names.len(), 24, "8 probes x 3 body sizes: {names:?}");
    let mut fails: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for name in &names {
        let tex = std::fs::read_to_string(dir().join(format!("{name}.tex"))).unwrap();
        let rendered = common::render_one(&tex);
        for diag in &rendered.v2.diagnostics {
            if matches!(
                diag.code.as_ref(),
                "font_unavailable"
                    | "required_metrics_unavailable"
                    | "ec_metrics_unavailable"
                    | "font_outline_substituted"
                    | "unsupported_block"
                    | "float_content_unsupported"
                    | "float_too_large"
            ) {
                fails.push(format!("{name}: {}: {}", diag.code, diag.message));
            }
        }
        if rendered.v2.pages.len() != 1 {
            fails.push(format!("{name}: {} pages, pdflatex set 1", rendered.v2.pages.len()));
            continue;
        }
        let want = expected(name);
        let got = baselines(&rendered);
        if got.len() != want.len() {
            fails.push(format!("{name}: {} baselines, pdflatex set {}", got.len(), want.len()));
            continue;
        }
        for (row, y) in want.iter().zip(&got) {
            checked += 1;
            let off = y - (row.baseline + row.residual);
            if off.abs() > row.tol {
                fails.push(format!(
                    "{name} {:?}: pdflatex {:.5}, expected {:+.5} off it, ours {:.5} ({:+.5}), \
                     over the {} bp gate by {:.5}",
                    row.word,
                    row.baseline,
                    row.residual,
                    y,
                    y - row.baseline,
                    row.tol,
                    off.abs() - row.tol
                ));
            }
        }
    }
    assert!(checked >= 180, "only {checked} baselines compared");
    assert!(fails.is_empty(), "{} baseline(s) off:\n{}", fails.len(), fails.join("\n"));
}
