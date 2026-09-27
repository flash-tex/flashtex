//! A `\ ` that ends a macro body does not swallow the blank after the
//! invocation. State S (TeX §347, skip blanks after a control space) belongs
//! to the tokenizer reading one line of text: the newline after
//! `\begin{solution}` is read from the document as a space token of its own,
//! whatever the replacement text ended with. pdflatex (TeX Live 2026, oracle
//! only), `\newenvironment{solution}{\par\noindent\textit{Solution.}\ }{..}`
//! then `\begin{solution}` + newline + `Positivity`: `\showlists` gives
//! `\glue 3.33333 plus 1.66666 minus 1.11111` (the `\ `) and then
//! `\glue 4.44444 plus 4.99997 minus 0.37036` (the newline, a sentence space
//! after `Solution.`). The parser dropped the second one, so `Positivity`
//! sat 4.43 bp left of pdflatex (fixtures/proof-corpus/problem-set-solution-env).
use flashtex_compiler::parser::{parse, Block, Inline};

fn glue_before(source: &str, word: &str) -> bool {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => inlines.iter().find_map(|inline| match inline {
                Inline::Text { text, glue_before, .. } if text == word => Some(glue_before.is_some()),
                _ => None,
            }),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no {word:?}"))
}

fn doc(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\newenvironment{{solution}}{{\\par\\noindent\\textit{{Solution.}}\\ }}{{\\par}}\n\\newcommand\\sol{{\\textit{{Solution.}}\\ }}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

#[test]
fn the_blank_after_an_invocation_whose_body_ends_in_a_control_space_is_glue() {
    assert!(glue_before(&doc("\\begin{solution}\nPositivity holds.\n\\end{solution}"), "Positivity"));
    assert!(glue_before(&doc("\\begin{solution} Positivity holds.\n\\end{solution}"), "Positivity"));
    assert!(glue_before(&doc("\\sol{} Positivity holds."), "Positivity"));
}

#[test]
fn a_blank_right_after_a_typed_control_space_is_still_skipped() {
    assert!(!glue_before(&doc("\\textit{Solution.}\\ Positivity holds."), "Positivity"));
    assert!(!glue_before(&doc("\\textit{Solution.}\\  Positivity holds."), "Positivity"));
    // A control word eats the blank after it, macro or not.
    assert!(!glue_before(&doc("\\sol Positivity holds."), "Positivity"));
}
