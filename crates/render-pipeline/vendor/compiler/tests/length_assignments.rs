//! The document's page and paragraph length assignments and its
//! `\setcounter{secnumdepth}` are on `Parsed` (PLAN1 sites 33 and 34), as
//! the expansion engine ran them: a macro that runs one counts, a
//! definition that never runs does not, and a grouped one is local.
use flashtex_compiler::parser::parse;

fn doc(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}\n{preamble}\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn lengths(source: &str) -> Vec<(String, String, bool)> {
    parse(source).length_assignments.into_iter().map(|a| (a.name, a.value, a.preamble)).collect()
}

#[test]
fn a_macro_that_runs_counts_and_an_uncalled_one_does_not() {
    let direct = lengths(&doc("\\setlength{\\parindent}{0pt}\n", "x"));
    assert_eq!(direct, [("parindent".to_string(), "0.0pt".to_string(), true)]);
    assert_eq!(lengths(&doc("\\newcommand\\np{\\setlength{\\parindent}{0pt}}\\np\n", "x")), direct);
    assert!(lengths(&doc("\\newcommand\\np{\\setlength{\\parindent}{0pt}}\n", "x")).is_empty());
}

#[test]
fn values_are_resolved_and_grouped_assignments_are_local() {
    let source = doc("\\setlength{\\parskip}{6pt plus 2pt}\\addtolength{\\parskip}{1pt}\n{\\setlength{\\textwidth}{5in}}\n", "\\parindent=1em x");
    assert_eq!(
        lengths(&source),
        [
            ("parskip".to_string(), "6.0pt plus 2.0pt".to_string(), true),
            ("parskip".to_string(), "7.0pt plus 2.0pt".to_string(), true),
            ("parindent".to_string(), "10.00002pt".to_string(), false),
        ]
    );
}

#[test]
fn secnumdepth_is_the_last_setcounter_that_ran() {
    assert_eq!(parse(&doc("", "x")).secnumdepth, None);
    assert_eq!(parse(&doc("\\setcounter{secnumdepth}{-1}\n", "x")).secnumdepth, Some(-1));
    assert_eq!(parse(&doc("\\newcommand\\scn{\\setcounter{secnumdepth}{0}}\n", "x")).secnumdepth, None);
    assert_eq!(parse(&doc("\\newcommand\\scn{\\setcounter{secnumdepth}{0}}\\scn\\addtocounter{secnumdepth}{2}\n", "x")).secnumdepth, Some(2));
}
