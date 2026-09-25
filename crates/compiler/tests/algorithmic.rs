//! algorithm/algorithmic pseudocode: one named `unsupported_feature`
//! diagnostic per construct, not generic unknown-command noise per line.
//!
//! The packages are built in (`crate::packages::BUILT_IN_PACKAGES`, so no
//! project `.sty` is read and the `\usepackage` warning carries no
//! looked-for-file note), the `algorithmic`/`algorithm` environments are
//! known-unimplemented vocabulary (so `\begin` warns `unsupported_feature`
//! with help naming the package), and the block commands (`\State`, ...)
//! are known-unimplemented commands with package help.

use flashtex_compiler::diagnostics::DiagnosticCode;
use flashtex_compiler::incremental::compile_full;
use flashtex_compiler::layout::LayoutConstraints;

fn diagnostics(text: &str) -> Vec<(String, Option<DiagnosticCode>, Option<String>)> {
    compile_full(text, LayoutConstraints::default())
        .diagnostics
        .into_iter()
        .map(|d| (d.message, d.code, d.help.map(|h| h.message)))
        .collect()
}

fn only_with_code(text: &str, needle: &str) -> (Option<DiagnosticCode>, Option<String>) {
    let all = diagnostics(text);
    let matching: Vec<_> = all.iter().filter(|(m, ..)| m.contains(needle)).collect();
    assert_eq!(matching.len(), 1, "{text}: {all:?}");
    (matching[0].1, matching[0].2.clone())
}

#[test]
fn algorithmic_environment_is_a_named_unsupported_feature() {
    let text = "\\documentclass{article}\n\
        \\usepackage{algorithmic}\n\
        \\begin{document}\n\
        \\begin{algorithmic}\n\
        \\State $x \\gets 1$\n\
        \\end{algorithmic}\n\
        \\end{document}\n";
    let (code, help) = only_with_code(text, "environment 'algorithmic'");
    assert_eq!(code, Some(DiagnosticCode::UnsupportedFeature), "{text}");
    assert_eq!(
        help.as_deref(),
        Some("environment 'algorithmic' needs \\usepackage{algorithmic}"),
        "{text}"
    );
    // The `\usepackage` itself stays a named warning, without a
    // looked-for-file note (the package is built in).
    let (code, _) = only_with_code(
        text,
        "packages algorithmic are recognised but not implemented",
    );
    assert_eq!(code, Some(DiagnosticCode::UnsupportedFeature), "{text}");
}

#[test]
fn state_lines_are_unsupported_features_not_unknown_commands() {
    let text = "\\documentclass{article}\n\
        \\usepackage{algorithmic}\n\
        \\begin{document}\n\
        \\begin{algorithmic}\n\
        \\State $x \\gets 1$\n\
        \\end{algorithmic}\n\
        \\end{document}\n";
    let (code, help) = only_with_code(text, "\\State");
    assert_eq!(code, Some(DiagnosticCode::UnsupportedFeature), "{text}");
    assert_eq!(
        help.as_deref(),
        Some("\\State is a algorithmic command, which this compiler does not implement"),
        "{text}"
    );
    // No generic per-line noise anywhere in the document.
    let all = diagnostics(text);
    assert!(
        all.iter()
            .all(|(_, code, _)| *code != Some(DiagnosticCode::UnknownCommand)),
        "{all:?}"
    );
}

#[test]
fn algorithm_float_environment_is_named_too() {
    // Only the `\caption` inside `algorithm` is modelled (the Algorithm
    // counter and label); the float wrapper itself reports by name.
    let text = "\\documentclass{article}\n\
        \\usepackage{algorithm}\n\
        \\begin{document}\n\
        \\begin{algorithm}\n\
        \\caption{Euclid}\n\
        \\end{algorithm}\n\
        \\end{document}\n";
    let (code, help) = only_with_code(text, "environment 'algorithm'");
    assert_eq!(code, Some(DiagnosticCode::UnsupportedFeature), "{text}");
    assert_eq!(
        help.as_deref(),
        Some("environment 'algorithm' needs \\usepackage{algorithm}"),
        "{text}"
    );
    let all = diagnostics(text);
    assert!(
        all.iter()
            .all(|(_, code, _)| *code != Some(DiagnosticCode::UnknownCommand)),
        "{all:?}"
    );
}
