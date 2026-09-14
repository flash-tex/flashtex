//! Named math operators: the table of kernel log-like functions, and
//! `\DeclareMathOperator` as the route for everything outside it.
//!
//! The table used to carry a 33rd entry, `\sgn`, that no LaTeX layer defines
//! (probed under TeX Live 2025 with `amsmath` and `amssymb` loaded: it is the
//! only undefined name of the 33). A row like that is the worst divergence
//! this engine can ship — the document renders here and fails in pdflatex —
//! and it was published as `"renders": true` in the generated inventory the
//! Mac editor completes from. These tests pin both directions: a document
//! that declares the operator works, one that does not is told so.

use flashtex_compiler::diagnostics::DiagnosticCode;
use flashtex_compiler::incremental::compile_full;
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::math::{MathList, Nucleus};
use flashtex_compiler::parser::{self, Block, Inline};

fn diagnostics(text: &str) -> Vec<(String, Option<DiagnosticCode>, Option<String>)> {
    compile_full(text, LayoutConstraints::default())
        .diagnostics
        .into_iter()
        .map(|d| (d.message, d.code, d.suggestion))
        .collect()
}

fn document(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{amsmath}}\n{body}\n")
}

/// Every math list in the document, in reading order.
fn math_lists(text: &str) -> Vec<MathList> {
    let mut out = Vec::new();
    for block in parser::parse(text).blocks {
        let inlines = match block {
            Block::Paragraph(inlines) => inlines,
            Block::Styled { content, .. } | Block::ListItem { content, .. } => content,
            _ => continue,
        };
        for inline in inlines {
            if let Inline::Math { list, .. } = inline {
                out.push(list);
            }
        }
    }
    out
}

/// The single `Nucleus::Operator` in `body`, as (operator text, limits).
fn operator(body: &str) -> (String, bool) {
    let lists = math_lists(&document(body));
    let mut found = Vec::new();
    for list in &lists {
        for atom in &list.atoms {
            if let Nucleus::Operator { body, limits } = &atom.nucleus {
                let text: String = body
                    .atoms
                    .iter()
                    .filter_map(|a| match &a.nucleus {
                        Nucleus::Text(t) => Some(t.as_str()),
                        _ => None,
                    })
                    .collect();
                found.push((text, *limits));
            }
        }
    }
    assert_eq!(found.len(), 1, "{body}: {found:?}");
    found.pop().unwrap()
}

/// The kernel's log-like functions, `fontmath.ltx` 455-503. Pinned as a
/// literal list rather than derived from the compiler's own table, so that
/// adding an entry to that table has to change this line too and say which
/// LaTeX file defines the new name.
const FONTMATH_LOG_LIKE: &[&str] = &[
    "arccos", "arcsin", "arctan", "arg", "cos", "cosh", "cot", "coth", "csc", "deg", "det", "dim",
    "exp", "gcd", "hom", "inf", "ker", "lg", "lim", "liminf", "limsup", "ln", "log", "max", "min",
    "Pr", "sec", "sin", "sinh", "sup", "tan", "tanh",
];

#[test]
fn the_operator_table_is_exactly_the_kernels_log_like_functions() {
    let inventory = flashtex_compiler::supported::inventory();
    let mut published: Vec<&str> = inventory
        .commands
        .iter()
        .filter(|c| c.origin == flashtex_compiler::supported::Origin::MathOperator)
        .map(|c| c.name)
        .collect();
    published.sort_unstable();
    let mut expected: Vec<&str> = FONTMATH_LOG_LIKE.to_vec();
    expected.sort_unstable();
    assert_eq!(published, expected);
    assert!(!published.contains(&"sgn"));
}

#[test]
fn an_undeclared_sgn_is_diagnosed_and_offers_the_declaration() {
    let all = diagnostics(&document("\\begin{document}$\\sgn x$\\end{document}"));
    let matching: Vec<_> = all.iter().filter(|(m, ..)| m.contains("\\sgn")).collect();
    assert_eq!(matching.len(), 1, "{all:?}");
    assert_eq!(matching[0].1, Some(DiagnosticCode::UnknownCommand));
    assert_eq!(
        matching[0].2.as_deref(),
        Some("\\DeclareMathOperator{\\sgn}{sgn}"),
        "the edit-distance suggester answers \\sin here, which is wrong advice"
    );
}

#[test]
fn a_declared_sgn_sets_as_an_operator_with_no_diagnostic() {
    let body = "\\DeclareMathOperator{\\sgn}{sgn}\n\\begin{document}$\\sgn x$\\end{document}";
    let mentions: Vec<_> = diagnostics(&document(body))
        .into_iter()
        .filter(|(m, ..)| m.contains("\\sgn") || m.contains("DeclareMathOperator"))
        .collect();
    assert!(mentions.is_empty(), "{mentions:?}");
    assert_eq!(operator(body), ("sgn".to_string(), false));
}

/// amsopn.sty: `\DeclareMathOperator` is `\qopname\newmcodes@ o` and
/// `\operatorname` is the same `\qopname` — one `\mathop{\operator@font ...}`
/// with `\nolimits`. The starred forms are the `m` (`\limits`) variant.
#[test]
fn declared_operators_and_operatorname_are_the_same_atom() {
    let declared = "\\DeclareMathOperator{\\sgn}{sgn}\n\\begin{document}$\\sgn x$\\end{document}";
    let spelled_out = "\\begin{document}$\\operatorname{sgn} x$\\end{document}";
    assert_eq!(operator(declared), operator(spelled_out));

    let declared_star =
        "\\DeclareMathOperator*{\\rank}{rank}\n\\begin{document}$\\rank_{A} x$\\end{document}";
    let spelled_out_star = "\\begin{document}$\\operatorname*{rank}_{A} x$\\end{document}";
    assert_eq!(operator(declared_star), ("rank".to_string(), true));
    assert_eq!(operator(declared_star), operator(spelled_out_star));
}

/// `\DeclareMathOperator{\argmax}{arg\,max}` keeps the thin space inside the
/// operator's body, so the operator sets as `arg max`, not `argmax`.
#[test]
fn a_declared_operator_keeps_the_math_glue_written_in_its_text() {
    let body =
        "\\DeclareMathOperator*{\\argmax}{arg\\,max}\n\\begin{document}$\\argmax_y f(y)$\\end{document}";
    assert_eq!(operator(body), ("argmax".to_string(), true));
    let lists = math_lists(&document(body));
    let spaces = lists
        .iter()
        .flat_map(|l| l.atoms.iter())
        .filter_map(|a| match &a.nucleus {
            Nucleus::Operator { body, .. } => Some(body),
            _ => None,
        })
        .flat_map(|body| body.atoms.iter())
        .filter(|a| matches!(a.nucleus, Nucleus::Space { .. }))
        .count();
    assert_eq!(spaces, 1, "the \\, between `arg` and `max` must survive");
}

/// The defect that operator exposed is not specific to operators: the lexer
/// encodes a control symbol's escape in its span *width*, and a token copied
/// out of a macro body carries the invocation's span instead. Every `\,`
/// `\;` `\!` `\:` in every `\newcommand` body decayed into the bare
/// character. Both spellings must produce the same atoms.
#[test]
fn control_symbols_survive_a_macro_body() {
    for symbol in ["\\,", "\\;", "\\!", "\\:"] {
        let body = format!(
            "\\newcommand{{\\gap}}{{a{symbol}b}}\n\\begin{{document}}$\\gap$\\end{{document}}"
        );
        let direct = format!("\\begin{{document}}$a{symbol}b$\\end{{document}}");
        let kinds = |text: &str| -> Vec<String> {
            math_lists(&document(text))
                .iter()
                .flat_map(|l| l.atoms.iter())
                .map(|a| match &a.nucleus {
                    Nucleus::Symbol(s) => format!("symbol {s}"),
                    Nucleus::Space { .. } => "space".to_string(),
                    other => format!("{other:?}"),
                })
                .collect()
        };
        let through_macro = kinds(&body);
        assert_eq!(through_macro, kinds(&direct), "{symbol} through a macro body");
        assert!(
            through_macro.contains(&"space".to_string()),
            "{symbol}: {through_macro:?}"
        );
    }
}
