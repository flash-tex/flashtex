//! Issue #495: `\addvspace` takes the MAX of its argument and the vertical
//! skip already pending at that point — never the sum.
//!
//! KNOWN DEVIATION (slice 2, pdflatex `\vbox`-height oracle, TeX Live 2026):
//! real LaTeX max-merges ONLY against a nonzero pending `\lastskip`, and
//! `\@vspace` appends `\vskip\z@skip`, so `\vspace{20pt}\addvspace{10pt}`
//! sets 30pt (addition), as does `\vspace{10pt}\addvspace{20pt}`. This
//! crate's arm max-merges against a trailing `\vspace` block too, so the
//! two `\vspace`-then-`\addvspace` cases below pin the IMPLEMENTATION
//! (20pt), not real LaTeX (30pt) — see the `KNOWN DEVIATION` comment on
//! the `vertical_command` arm. Consecutive-`\addvspace` max and
//! lone-`\addvspace` add below DO match the oracle.
use flashtex_compiler::incremental::compile_full_project;
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{parse, Block, SourceDocument};

/// Baseline distance between the single-line "Above" and "Below" paragraphs
/// around `between`. Asserts the compile is diagnostic-free: `\addvspace`
/// must no longer report `unsupported_feature`.
///
/// Compare only docs with identical paragraph structure (both with a skip
/// block between the paragraphs): a skip block carries paragraph-gap
/// padding on each side, so a bare `gap("")` baseline is NOT `pt` below
/// `gap("\\vspace{<pt>}")`. Relative comparisons (`addvspace` combo vs the
/// equivalent `\vspace`-only doc) isolate exactly the max semantics.
fn gap(between: &str) -> f64 {
    let text = format!("Above\n\n{between}\nBelow\n");
    let out = compile_full_project(
        &[SourceDocument {
            path: "main.tex",
            text: &text,
        }],
        "main.tex",
        LayoutConstraints::default(),
    );
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let items = &out.pages[0].items;
    let above = items.iter().find(|i| i.text == "Above").unwrap();
    let below = items.iter().find(|i| i.text == "Below").unwrap();
    below.baseline_y_pt - above.baseline_y_pt
}

/// Natural lengths of every `Block::VSpace` for a body fragment, asserting a
/// diagnostic-free parse.
fn vspace_pts(body: &str) -> Vec<f64> {
    let parsed = parse(body);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::VSpace { pt, .. } => Some(*pt),
            _ => None,
        })
        .collect()
}

#[test]
fn addvspace_keeps_the_pending_skip_when_it_is_larger() {
    // The max-semantics case: 20pt already pending, 10pt asked — the 10pt
    // must vanish, not add. A plain-addition implementation would leave
    // two blocks (or one 30pt block) and set 30pt.
    assert_eq!(vspace_pts("A\n\n\\vspace{20pt}\n\\addvspace{10pt}\nB\n"), vec![20.0]);
    let reference = gap("\\vspace{20pt}");
    // Max, not addition: an additive implementation would sit 10pt lower.
    assert!(
        (gap("\\vspace{20pt}\n\\addvspace{10pt}") - reference).abs() < 0.02,
        "max, not addition"
    );
}

#[test]
fn addvspace_replaces_the_pending_skip_when_it_is_larger() {
    assert_eq!(vspace_pts("A\n\n\\vspace{10pt}\n\\addvspace{20pt}\nB\n"), vec![20.0]);
    // Consecutive `\addvspace` calls fold the same way: the maximum wins.
    assert_eq!(
        vspace_pts("A\n\n\\addvspace{10pt}\n\\addvspace{20pt}\nB\n"),
        vec![20.0]
    );
    let reference = gap("\\vspace{20pt}");
    assert!((gap("\\vspace{10pt}\n\\addvspace{20pt}") - reference).abs() < 0.02);
    assert!((gap("\\addvspace{10pt}\n\\addvspace{20pt}") - reference).abs() < 0.02);
}

#[test]
fn addvspace_without_a_pending_skip_adds_its_length() {
    assert_eq!(vspace_pts("A\n\n\\addvspace{15pt}\nB\n"), vec![15.0]);
    let reference = gap("\\vspace{15pt}");
    // Guards against an implementation that silently drops `\addvspace`.
    assert!((gap("\\addvspace{15pt}") - reference).abs() < 0.02);
    assert!(reference - gap("") > 10.0, "reference skip must apply");
}
