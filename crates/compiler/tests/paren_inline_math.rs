//! `\(...\)` is LaTeX's inline math (`$...$` is TeX's): the same content,
//! the same layout, the same recovery when it is left open.

use flashtex_compiler::incremental::compile_full;
use flashtex_compiler::layout::LayoutConstraints;

fn compile(text: &str) -> flashtex_compiler::incremental::CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

#[test]
fn paren_math_is_inline_math() {
    let dollar = compile("Input \\(x_t\\) at step \\(t\\), rate \\(\\alpha\\vartheta_j(t)\\).\n");
    let paren = compile("Input $x_t$ at step $t$, rate $\\alpha\\vartheta_j(t)$.\n");
    assert!(dollar.diagnostics.is_empty(), "{:?}", dollar.diagnostics);
    assert_eq!(dollar.blocks.len(), paren.blocks.len());
    // Spans differ by the delimiter width; the typeset output does not.
    let words = |o: &flashtex_compiler::incremental::CompileOutput| {
        o.pages
            .iter()
            .flat_map(|p| p.items.iter())
            .map(|i| {
                (
                    i.text.clone(),
                    (i.x_pt * 100.0).round(),
                    (i.baseline_y_pt * 100.0).round(),
                    (i.font_size_pt * 100.0).round(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        words(&dollar),
        words(&paren),
        "\\(..\\) and $..$ lay out identically"
    );
}

#[test]
fn paren_math_inside_a_list_and_a_macro() {
    let out = compile(
        "\\newcommand{\\oname}[1]{\\operatorname{#1}}\n\\begin{itemize}\\item \\(\\oname{net}_i(t)\\) is output \\(i\\).\\end{itemize}\n",
    );
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
}

#[test]
fn unterminated_paren_math_ends_with_its_paragraph() {
    let out = compile("Visible \\(x+1\n\nTail \\(y\\).\n");
    let messages: Vec<&str> = out.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(
        messages,
        ["inline math is missing its closing '$'"],
        "{messages:?}"
    );
}

#[test]
fn stray_paren_close_is_reported_and_dropped() {
    let out = compile("Visible \\) Tail.\n");
    let messages: Vec<&str> = out.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(messages, ["stray \\) has no matching \\("], "{messages:?}");
}
