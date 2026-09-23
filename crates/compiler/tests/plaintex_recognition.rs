//! Recognition (not support) for non-LaTeX2e formats (issue #907).
//!
//! Ten of the hundred famous arXiv papers are not LaTeX2e at all — six are
//! plain TeX (`harvmac` / `epsf` / `\magnification`) and four are LaTeX 2.09
//! (`\documentstyle`) — and the compiler answered with a flood of
//! unknown-command errors that never named the real cause. These tests pin
//! the recognition diagnostics: exactly one early diagnostic naming the
//! format, and no downstream unknown-command flood.

use flashtex_compiler::diagnostics::DiagnosticCode;
use flashtex_compiler::parser::{parse, parse_project, SourceDocument};

fn messages(text: &str) -> Vec<(String, Option<DiagnosticCode>)> {
    parse(text)
        .diagnostics
        .iter()
        .map(|d| (d.message.clone(), d.code))
        .collect()
}

fn project_messages(main: &str, extra: &[(&str, &str)]) -> Vec<(String, Option<DiagnosticCode>)> {
    let owned: Vec<String> = extra.iter().map(|(_, t)| t.to_string()).collect();
    let main_owned = main.to_string();
    let mut docs: Vec<SourceDocument<'_>> = Vec::new();
    docs.push(SourceDocument {
        path: "main.tex",
        text: &main_owned,
    });
    for (i, (path, _)) in extra.iter().enumerate() {
        docs.push(SourceDocument {
            path,
            text: &owned[i],
        });
    }
    parse_project(&docs, "main.tex")
        .diagnostics
        .iter()
        .map(|d| (d.message.clone(), d.code))
        .collect()
}

#[test]
fn latex_209_document_gets_one_early_diagnostic_and_no_flood() {
    let diags = messages(
        "\\documentstyle[11pt]{article}\n\\begin{document}\nHello, world.\n\\section{One}\nText.\n\\end{document}\n",
    );
    let legacy: Vec<_> = diags
        .iter()
        .filter(|(m, _)| m.contains("LaTeX 2.09"))
        .collect();
    assert_eq!(legacy.len(), 1, "{diags:?}");
    assert!(legacy[0].0.contains("\\documentstyle"), "{diags:?}");
    assert!(
        diags
            .iter()
            .all(|(_, c)| *c != Some(DiagnosticCode::UnknownCommand)),
        "{diags:?}"
    );
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].0, legacy[0].0.clone());
}

#[test]
fn plain_tex_document_gets_one_early_diagnostic_and_no_flood() {
    let diags = project_messages(
        "\\magnification\\magstep1\n\\input harvmac\n\\centerline{Hello, world.}\n\\bye\n",
        &[("harvmac.tex", "% harvmac stub\n")],
    );
    let legacy: Vec<_> = diags
        .iter()
        .filter(|(m, _)| m.contains("plain TeX"))
        .collect();
    assert_eq!(legacy.len(), 1, "{diags:?}");
    assert!(
        diags
            .iter()
            .all(|(_, c)| *c != Some(DiagnosticCode::UnknownCommand)),
        "{diags:?}"
    );
    assert_eq!(diags.len(), 1, "{diags:?}");
}

#[test]
fn latex2e_document_using_input_is_unaffected() {
    let diags = project_messages(
        "\\documentclass{article}\n\\begin{document}\nHello.\n\\input{foo}\nMore.\n\\end{document}\n",
        &[("foo.tex", "included text.\n")],
    );
    assert!(
        diags
            .iter()
            .all(|(m, _)| !m.contains("plain TeX") && !m.contains("LaTeX 2.09")),
        "{diags:?}"
    );
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn latex2e_document_using_input_epsf_is_unaffected() {
    // Mirrors phys-kitaev-anyons-ftqc/anyons.tex: a real LaTeX2e paper that
    // genuinely `\input`s epsf. Recognition must stay silent.
    let diags = project_messages(
        "\\documentclass[12pt]{article}\n\\input{epsf.tex}\n\\begin{document}\nHello.\n\\end{document}\n",
        &[("epsf.tex", "% epsf stub\n")],
    );
    assert!(
        diags
            .iter()
            .all(|(m, _)| !m.contains("plain TeX") && !m.contains("LaTeX 2.09")),
        "{diags:?}"
    );
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn typo_fragment_without_documentclass_is_not_plain_tex() {
    // A bare fragment with a typo must keep its unknown-command diagnostic:
    // the absence of `\documentclass` alone never identifies plain TeX.
    let diags = messages("Text \\alpah here.");
    assert!(
        diags
            .iter()
            .all(|(m, _)| !m.contains("plain TeX") && !m.contains("LaTeX 2.09")),
        "{diags:?}"
    );
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].1, Some(DiagnosticCode::UnknownCommand));
}

#[test]
fn documentclass_alongside_documentstyle_never_fires() {
    // A LaTeX2e manual that merely mentions `\documentstyle` stays silent.
    let diags = messages(
        "\\documentclass{article}\n\\begin{document}\nSince \\verb|\\documentstyle| was replaced by \\verb|\\documentclass|.\n\\end{document}\n",
    );
    assert!(
        diags
            .iter()
            .all(|(m, _)| !m.contains("plain TeX") && !m.contains("LaTeX 2.09")),
        "{diags:?}"
    );
    assert!(diags.is_empty(), "{diags:?}");
}
