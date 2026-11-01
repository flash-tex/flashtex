//! `\index` and `\glossary` are accepted no-ops (GH#566). This compiler has
//! no indexing or glossary backend, and neither command typesets anything in
//! real LaTeX either, so the entry is parsed and discarded with no
//! diagnostic -- like `\graphicspath` and `\pagestyle`, not `unsupported`.

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

const PRE: &str = "\\documentclass{article}\n\\begin{document}\n";
const POST: &str = "\\end{document}\n";

#[test]
fn index_and_glossary_are_silent_no_ops() {
    let bare = compile(&format!("{PRE}Alpha beta gamma.\n{POST}"));
    assert!(bare.diagnostics.is_empty(), "{:?}", bare.diagnostics);
    for entry in [
        "\\index{term}",
        "\\index{term|textbf}",
        "\\index{term|see{other}}",
        "\\index{sortas@term}",
        "\\index{term!subentry}",
        "\\glossary{term: definition}",
    ] {
        let text = format!("{PRE}Alpha {entry} beta gamma.\n{POST}");
        let out = compile(&text);
        assert!(
            out.diagnostics.is_empty(),
            "{entry}: {:?}",
            out.diagnostics
        );
        assert_eq!(words(&out), words(&bare), "{entry}: no visible effect");
    }
}

#[test]
fn index_entries_do_not_leak_their_text_onto_the_page() {
    let text = format!("{PRE}Before\\index{{hidden term|textbf}}after.\n{POST}");
    let out = compile(&text);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let rendered: String = words(&out).iter().map(|(w, _, _)| w.clone()).collect();
    assert!(
        !rendered.contains("hidden"),
        "index entry leaked: {rendered:?}"
    );
}
