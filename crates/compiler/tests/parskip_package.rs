//! `\usepackage{parskip}` applies the package's own no-option defaults:
//! `\parindent` 0pt and `\parskip` of half the class `\baselineskip`.
use flashtex_compiler::parser::parse;

fn messages(src: &str) -> Vec<String> {
    parse(src)
        .diagnostics
        .into_iter()
        .map(|d| d.message)
        .collect()
}

#[test]
fn parskip_package_sets_the_documented_default_skip() {
    // parskip.sty v2.0h (TeX Live 2026): with no options `\parskip` becomes
    // `.5\baselineskip plus 2pt` and `\parindent` becomes `0pt`. The engine
    // never indents paragraphs, so only the skip needs storing; it is rigid
    // here (`parskip_pt` carries no stretch).
    let src =
        "\\documentclass{article}\\usepackage{parskip}\\begin{document}Hi\\end{document}";
    let parsed = parse(src);
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
    assert_eq!(parsed.parskip_pt, Some(6.0));
    assert!(parsed.packages.iter().any(|p| p == "parskip"));
}

#[test]
fn parskip_package_default_follows_the_class_size() {
    // `\the\parskip` under pdflatex (TeX Live 2026, article): 6.0pt at 10pt
    // (0.5 x 12pt `\baselineskip`), 6.8pt at 11pt (0.5 x 13.6pt), 7.25pt at
    // 12pt (0.5 x 14.5pt). The `plus 2pt` stretch is not modelled.
    for (option, expected) in [("10pt", 6.0), ("11pt", 6.8), ("12pt", 7.25)] {
        let src = format!(
            "\\documentclass[{option}]{{article}}\\usepackage{{parskip}}\
             \\begin{{document}}Hi\\end{{document}}"
        );
        let parsed = parse(&src);
        assert!(
            parsed.diagnostics.is_empty(),
            "{option}: {:?}",
            parsed.diagnostics
        );
        assert_eq!(parsed.parskip_pt, Some(expected), "{option}");
    }
}

#[test]
fn manual_setlength_after_parskip_still_wins() {
    // Packages set defaults, not permanent values: a later `\setlength`
    // overwrites the package default through the same state field, exactly
    // as in real LaTeX.
    let src = "\\documentclass{article}\\usepackage{parskip}\
         \\setlength{\\parskip}{12pt}\\begin{document}Hi\\end{document}";
    let parsed = parse(src);
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
    assert_eq!(parsed.parskip_pt, Some(12.0));
}

#[test]
fn parskip_package_load_after_setlength_takes_effect() {
    // Order semantics run both ways: the package load is an assignment like
    // any other, so loading it after a manual `\setlength` restores the
    // package default, as real LaTeX does.
    let src = "\\documentclass{article}\\setlength{\\parskip}{12pt}\
         \\usepackage{parskip}\\begin{document}Hi\\end{document}";
    let parsed = parse(src);
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
    assert_eq!(parsed.parskip_pt, Some(6.0));
}

#[test]
fn parskip_package_options_keep_a_diagnostic() {
    // Only the no-option default is implemented (`skip=`, `indent=`,
    // `parfill=`, `tocskip=` are not honoured), so options must warn rather
    // than apply silently.
    let src = "\\documentclass{article}\\usepackage[skip=10pt]{parskip}\
         \\begin{document}Hi\\end{document}";
    let found = messages(src);
    assert!(
        found
            .iter()
            .any(|m| m.contains("recognised but not implemented")),
        "{found:?}"
    );
}
