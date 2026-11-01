//! float.sty's `\newfloat`: the declared environment is a real float.
//!
//! Oracle: pdflatex (TeX Live 2026) on
//! `\usepackage{float}\newfloat{program}{h}{lop}\floatname{program}{Program}`
//! with `\begin{program}[H]code\caption{P}\end{program}` sets `code` then
//! `Program 1: P`, with no `[H]` text; placements `[h]`, `[H]`, `[htbp]`
//! are all accepted, while a bare `t` (no brackets) stays body text. Under
//! plain article `\newfloat` is `! Undefined control sequence.`
use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::incremental::{compile_full, LayoutConstraints};

const PREAMBLE: &str =
    r"\usepackage{float}\newfloat{program}{h}{lop}\floatname{program}{Program}";

fn texts(source: &str) -> Vec<String> {
    compile_full(source, LayoutConstraints::default())
        .pages
        .into_iter()
        .flat_map(|page| page.items.into_iter().map(|item| item.text))
        .collect()
}

fn errors(source: &str) -> Vec<String> {
    compile_full(source, LayoutConstraints::default())
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message)
        .collect()
}

fn warnings(source: &str) -> Vec<String> {
    compile_full(source, LayoutConstraints::default())
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::Warning)
        .map(|d| d.message)
        .collect()
}

#[test]
fn newfloat_environment_sets_body_and_numbered_caption() {
    // The slice repro: no "not implemented" diagnostic, no `[H]` text.
    let source = format!("{PREAMBLE}\\begin{{program}}[H]code\\caption{{P}}\\end{{program}}");
    assert_eq!(texts(&source), vec!["code", "Program 1:", "P"]);
    assert!(errors(&source).is_empty(), "{:?}", errors(&source));
    assert!(
        !warnings(&source)
            .iter()
            .any(|m| m.contains("environment 'program'")),
        "{:?}",
        warnings(&source)
    );
}

#[test]
fn newfloat_counter_is_own_and_increments() {
    let source = format!(
        "{PREAMBLE}\\begin{{program}}a\\caption{{A}}\\end{{program}}\
         \\begin{{program}}b\\caption{{B}}\\end{{program}}\
         \\begin{{figure}}f\\caption{{F}}\\end{{figure}}"
    );
    let output = texts(&source);
    let labels: Vec<&String> = output
        .iter()
        .filter(|t| *t == "Program 1:" || *t == "Program 2:" || *t == "Figure 1:")
        .collect();
    assert_eq!(labels, ["Program 1:", "Program 2:", "Figure 1:"]);
    assert!(errors(&source).is_empty(), "{:?}", errors(&source));
}

#[test]
fn float_placements_are_accepted() {
    for placement in ["H", "h", "t", "b", "p", "htbp", "!htbp"] {
        let source = format!("{PREAMBLE}\\begin{{program}}[{placement}]b\\caption{{C}}\\end{{program}}");
        let output = texts(&source);
        assert!(
            !output.iter().any(|t| t.contains('[')),
            "{placement}: placement leaked into output: {output:?}"
        );
        assert!(output.contains(&"b".to_string()), "{placement}: {output:?}");
        assert!(
            output.contains(&"Program 1:".to_string()),
            "{placement}: {output:?}"
        );
        assert!(errors(&source).is_empty(), "{placement}: {:?}", errors(&source));
    }
    // No placement at all is the same float.
    let bare = format!("{PREAMBLE}\\begin{{program}}b\\caption{{C}}\\end{{program}}");
    assert_eq!(texts(&bare), vec!["b", "Program 1:", "C"]);
    // A bare `t` without brackets is body text, as pdflatex sets it.
    let leading_t = format!("{PREAMBLE}\\begin{{program}}t\\caption{{D}}\\end{{program}}");
    assert_eq!(texts(&leading_t), vec!["t", "Program 1:", "D"]);
}

#[test]
fn figure_placement_is_consumed() {
    let output = texts(r"\begin{figure}[htbp]code\caption{P}\end{figure}");
    assert_eq!(output, vec!["code", "Figure 1:", "P"]);
}

#[test]
fn float_commands_need_the_float_package() {
    // pdflatex under plain article: `! Undefined control sequence.`
    for (command, args) in [
        ("newfloat", "{program}{h}{lop}"),
        ("floatname", "{program}{Program}"),
        ("floatstyle", "{ruled}"),
        ("floatplacement", "{figure}{tbp}"),
    ] {
        let source = format!("\\{command}{args}Body.");
        let diagnostics = errors(&source);
        assert!(
            diagnostics
                .iter()
                .any(|m| m.contains(&format!("\\{command}")) && m.contains(r"\usepackage{float}")),
            "{command}: {diagnostics:?}"
        );
    }
    // A rejected `\newfloat` registers nothing: the environment stays unknown.
    let source = r"\newfloat{program}{h}{lop}\begin{program}x\end{program}";
    assert!(
        warnings(source)
            .iter()
            .any(|m| m.contains("environment 'program'")),
        "{:?}",
        warnings(source)
    );
}
