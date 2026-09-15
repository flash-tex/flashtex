//! `Parsed::block_par_leading` has exactly one entry per top-level block
//! (the render pipeline pairs them by index and debug-asserts the lengths).
//! A table entry or box argument that parsed blocks and folded them into
//! inlines truncated `block_dependencies` but left their leading entries
//! behind, shifting every later paragraph's `\baselineskip`. Found by
//! `crates/render-pipeline/examples/fuzz_render.rs`
//! (`adapter.rs: assertion left == right failed`).

use flashtex_compiler::parser::{parse_project, SourceDocument};

fn assert_one_leading_per_block(text: &str) {
    let docs = [SourceDocument { path: "main.tex", text }];
    let parsed = parse_project(&docs, "main.tex");
    assert_eq!(
        parsed.block_par_leading.len(),
        parsed.blocks.len(),
        "block_par_leading out of step with blocks for {text:?}"
    );
}

#[test]
fn fuzz_minimised_longtable_in_usepackage() {
    assert_one_leading_per_block("\\usepackage{longtable\\begin{longtable}\\begin{longtable");
}

#[test]
fn table_entry_with_blocks() {
    assert_one_leading_per_block(
        "\\documentclass{article}\\begin{document}\\begin{tabular}{c}a\\section{s}b\\\\\\end{tabular}\n\npara\n\n\\begin{itemize}\\item x\\end{itemize}\\end{document}",
    );
    assert_one_leading_per_block(
        "\\documentclass{article}\\begin{document}\\begin{tabular}{c}\\begin{itemize}\\item x\\end{itemize}\\end{tabular}\n\nafter\\end{document}",
    );
}

#[test]
fn box_argument_with_blocks() {
    assert_one_leading_per_block(
        "\\documentclass{article}\\begin{document}\\colorbox{red}{a\\section{s}b}\n\npara\\end{document}",
    );
    assert_one_leading_per_block(
        "\\documentclass{article}\\begin{document}\\fbox{\\begin{itemize}\\item x\\end{itemize}}\n\nafter\\end{document}",
    );
}
