//! GH-EXAM-CLASS: exam.cls head/foot declarations in the preamble consume
//! their arguments silently in the `exam` class (the page chrome itself is
//! class-geometry's `ExamChrome`), and stay unknown elsewhere.

use flashtex_compiler::parser;

const PREAMBLE: &str = "\\pagestyle{head}\n\\headrule\n\\firstpagefootrule\n\
    \\header{\\textbf{12-345}}{\\textbf{Name}}{\\textbf{Homework 0}}\n\
    \\footer{}{Page \\thepage}{}\n\\cfoot[]{Page \\thepage}\n\\lhead{L}\n";

fn diagnostics(class: &str) -> Vec<String> {
    let src = format!("\\documentclass{{{class}}}\n{PREAMBLE}\\begin{{document}}\nBody.\n\\end{{document}}\n");
    parser::parse(&src).diagnostics.into_iter().map(|d| d.message).collect()
}

#[test]
fn exam_head_and_foot_declarations_are_accepted_in_the_preamble() {
    let diags = diagnostics("exam");
    assert!(
        !diags.iter().any(|d| ["header", "footer", "headrule", "footrule", "cfoot", "lhead"].iter().any(|c| d.contains(&format!("\\{c}")))),
        "{diags:?}"
    );
}

#[test]
fn their_arguments_are_not_typeset() {
    let src = format!("\\documentclass{{exam}}\n{PREAMBLE}\\begin{{document}}\nBody.\n\\end{{document}}\n");
    let parsed = parser::parse(&src);
    let text = format!("{:?}", parsed.blocks);
    assert!(!text.contains("Homework") && !text.contains("12-345"), "{text}");
}

#[test]
fn outside_exam_they_are_still_unsupported() {
    let diags = diagnostics("article");
    assert!(diags.iter().any(|d| d.contains("\\headrule")), "{diags:?}");
}
