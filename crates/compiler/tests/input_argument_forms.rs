//! Unbraced and quoted `\input` argument forms.
//!
//! TeX's `\input` (unlike LaTeX's usual `\input{name}`) also accepts the
//! classic unbraced form `\input part` (the filename runs to the next space)
//! and LaTeX's quoted form `\input{"part"}` for names containing spaces
//! (the surrounding quotes are stripped, not part of the filename).
//!
//! Ground truth, pdflatex (TeX Live 2026) on `main.tex` holding
//! `A \input part B \input{"part"} C next.` with `part.tex` holding
//! `(part text)`: the log shows `(part text)` twice (once per include),
//! with `B` at x 209.633 and `C` at x 273.454.
use flashtex_compiler::incremental::compile_full_project;
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{parse_project, Block, Inline, SourceDocument};

const MAIN_MIXED: &str =
    "\\documentclass{article}\n\\begin{document}\nA \\input part B \\input{\"part\"} C next.\n\\end{document}\n";
const MAIN_BRACED: &str =
    "\\documentclass{article}\n\\begin{document}\nA \\input{part} B \\input{part} C next.\n\\end{document}\n";
const PART: &str = "(part text)\n";

fn docs<'a>(main: &'a str) -> [SourceDocument<'a>; 2] {
    [
        SourceDocument { path: "main.tex", text: main },
        SourceDocument { path: "part.tex", text: PART },
    ]
}

fn paragraph_text(blocks: &[Block]) -> String {
    let mut out = String::new();
    for block in blocks {
        let Block::Paragraph(inlines) = block else {
            continue;
        };
        for inline in inlines {
            let Inline::Text { text, .. } = inline else {
                continue;
            };
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(text);
        }
    }
    out
}

fn flat_items(out: &flashtex_compiler::incremental::CompileOutput) -> Vec<(String, f64)> {
    out.pages
        .iter()
        .flat_map(|p| p.items.iter().map(|i| (i.text.clone(), i.x_pt)))
        .collect()
}

#[test]
fn unbraced_and_quoted_input_include_with_zero_errors() {
    let docs = docs(MAIN_MIXED);
    let parsed = parse_project(&docs, "main.tex");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let text = paragraph_text(&parsed.blocks);
    assert_eq!(text.matches("(part text)").count(), 2, "{text:?}");
    let a = text.find('A').expect("A present");
    let first = text.find("(part text)").expect("first include");
    let b = text.find('B').expect("B present");
    let second = text.match_indices("(part text)").nth(1).map(|(i, _)| i).expect("second include");
    let c = text.find('C').expect("C present");
    assert!(a < first && first < b && b < second && second < c, "{text:?}");
}

#[test]
fn mixed_spellings_match_braced_layout_exactly() {
    // The three spellings must typeset identically: same words at the same
    // positions. This pins the new forms to whatever the already-supported
    // braced form produces, so any future metrics change carries over.
    //
    // Absolute pdflatex positions are infeasible here (verified 2026-09-27:
    // pymupdf on real pdflatex output gives `B` at x 209.633 and `C` at x
    // 273.454, while this compiler lays the all-braced source out with `B`
    // at x 134.140 and `C` at x 195.610 — a pre-existing font-metrics gap
    // shared by the already-supported form, not by these spellings).
    let mixed = docs(MAIN_MIXED);
    let braced = docs(MAIN_BRACED);
    let out_mixed = compile_full_project(&mixed, "main.tex", LayoutConstraints::default());
    let out_braced = compile_full_project(&braced, "main.tex", LayoutConstraints::default());
    assert!(out_mixed.diagnostics.is_empty(), "{:?}", out_mixed.diagnostics);
    assert!(out_braced.diagnostics.is_empty(), "{:?}", out_braced.diagnostics);
    assert_eq!(flat_items(&out_mixed), flat_items(&out_braced));
}
