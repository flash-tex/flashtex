//! `\pagestyle`, `\thispagestyle` and `\pagenumbering` are ordinary preamble
//! material (GH#321, #258 ranked finding 11, probe
//! `fixtures/divergence-probes/min2-pagestyle`). With no header or footer
//! rendering they are no-ops, in the preamble exactly as in the body.

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;

fn compile(text: &str) -> CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

fn words(out: &CompileOutput) -> Vec<(String, i64, i64)> {
    out.pages
        .iter()
        .flat_map(|p| p.items.iter())
        .map(|i| {
            (
                i.text.clone(),
                (i.x_pt * 100.0).round() as i64,
                (i.baseline_y_pt * 100.0).round() as i64,
            )
        })
        .collect()
}

const BODY: &str = "\\begin{document}\nOne paragraph of body text.\n\\end{document}\n";

#[test]
fn page_style_commands_in_the_preamble_are_accepted_no_ops() {
    let bare = compile(&format!("\\documentclass{{article}}\n{BODY}"));
    for preamble in [
        "\\pagestyle{empty}",
        "\\thispagestyle{plain}",
        "\\pagenumbering{roman}",
        "\\pagestyle{headings}\\pagenumbering{arabic}",
    ] {
        let out = compile(&format!("\\documentclass{{article}}\n{preamble}\n{BODY}"));
        assert!(
            out.diagnostics.is_empty(),
            "{preamble}: {:?}",
            out.diagnostics
        );
        assert_eq!(words(&out), words(&bare), "{preamble}: no visible effect");
    }
}

#[test]
fn the_min2_pagestyle_probe_compiles_without_errors() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/divergence-probes/min2-pagestyle/main.tex"
    ))
    .expect("probe is committed");
    let out = compile(&text);
    let errors: Vec<_> = out
        .diagnostics
        .iter()
        .filter(|d| d.severity == flashtex_compiler::diagnostics::Severity::Error)
        .map(|d| d.message.as_str())
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
}
