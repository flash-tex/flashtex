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
    // \State is algorithmicx's mixed-case bundle (algpseudocode.sty), not
    // the older all-caps algorithmic.sty the environment itself defaults
    // to naming -- the two packages are distinct \usepackage targets.
    assert_eq!(
        help.as_deref(),
        Some("\\State is a algpseudocode command, which this compiler does not implement"),
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

#[test]
fn block_command_prose_arguments_are_never_silently_eaten() {
    // Round 4 review finding: the generic recoverable-argument heuristic
    // treats a single lowercase word as a parameter (matching keywords
    // like `arabic`/`empty`), which is exactly the shape of the prose
    // algpseudocode commands take: \Require{input}, \Function{merge}. That
    // text must still reach the page, not vanish as a "skipped parameter".
    let text = "\\documentclass{article}\n\
        \\usepackage{algpseudocode}\n\
        \\begin{document}\n\
        \\begin{algorithmic}\n\
        \\Require{input}\n\
        \\Function{merge}\n\
        \\State $x \\gets 1$\n\
        \\end{algorithmic}\n\
        \\end{document}\n";
    let (_, help) = only_with_code(text, "\\Require");
    assert_eq!(
        help.as_deref(),
        Some("\\Require is a algpseudocode command, which this compiler does not implement"),
        "{text}"
    );
    let (_, help) = only_with_code(text, "\\Function");
    assert_eq!(
        help.as_deref(),
        Some("\\Function is a algpseudocode command, which this compiler does not implement"),
        "{text}"
    );
    // The load-bearing check: the RECOVERY note (not the help text, which
    // is unaffected either way) must say the argument was typeset as plain
    // text, never that it was skipped as a parameter. This is the only
    // signal that actually distinguishes "input"/"merge" surviving on the
    // page from silently vanishing.
    let full = flashtex_compiler::incremental::compile_full(text, LayoutConstraints::default());
    for name in ["\\Require", "\\Function"] {
        let matching: Vec<_> = full
            .diagnostics
            .iter()
            .filter(|d| d.message.contains(name))
            .collect();
        assert_eq!(matching.len(), 1, "{name}: {:?}", full.diagnostics);
        assert_eq!(
            matching[0].recovery.as_deref(),
            Some("skipped the command; any braced argument was typeset as plain text"),
            "{name} recovery note: {:?}",
            matching[0].recovery
        );
    }
}

#[test]
fn algpseudocode_help_names_the_right_package_not_algorithmic() {
    // Round 4 review finding: mapping \State's help to "algorithmic"
    // (the older, all-caps sibling package) pointed authors at the wrong
    // \usepackage for the mixed-case commands they actually used.
    let text = "\\documentclass{article}\n\
        \\usepackage{algpseudocode}\n\
        \\begin{document}\n\
        \\begin{algorithmic}\n\
        \\If{$x > 0$}\n\
        \\EndIf\n\
        \\end{algorithmic}\n\
        \\end{document}\n";
    let (_, help) = only_with_code(text, "\\If");
    assert_eq!(
        help.as_deref(),
        Some("\\If is a algpseudocode command, which this compiler does not implement"),
        "{text}"
    );
    // algpseudocode itself is recognised as built in, no looked-for-file note.
    let (code, _) = only_with_code(
        text,
        "packages algpseudocode are recognised but not implemented",
    );
    assert_eq!(code, Some(DiagnosticCode::UnsupportedFeature), "{text}");
}
