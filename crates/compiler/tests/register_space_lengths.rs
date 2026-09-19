//! Issue #835 sub-bugs 1 and 2, from the `question` environment in the real
//! user document `set0.tex` (`\documentclass[11pt]{article}`):
//!
//! 1. A coefficient times a length register is rejected:
//!    `\setlength{\topsep}{0.6\baselineskip}` errored, although TeX allows
//!    `<factor><length>` anywhere a dimension is expected.
//! 2. `\newlength` registers do not read back as dimensions:
//!    `\setlength{\qnumsep}{0.6em}` then `\hspace{\qnumsep}` reported
//!    "Missing number" + "Illegal unit of measure" from the engine and
//!    "got ''" from the parser.
//!
//! Every oracle value below was measured with live pdflatex (pdfTeX 1.40.29,
//! TeX Live 2026, 1 page each; pages counted with `pdfinfo`, never `mdls`):
//! - `0.6\baselineskip` at 11pt is 8.16008pt; `\parindent` is 17.0pt there
//!   (15.0pt at 10pt) and `2\parindent` is 34.0pt; `-.5\textwidth` is
//!   -180.0pt; `\setlength{\qnumsep}{0.6em}` at 11pt is 6.57007pt;
//!   `2\zerolen` (register set to 0pt) is 0.0pt.
//! - A bare factor with no unit (`\setlength{\topsep}{0.6}`) is still an
//!   error in pdflatex ("Illegal unit of measure"), and `\hspace` with an
//!   undeclared register is still an error ("Undefined control sequence").
use flashtex_compiler::parser::{parse, Block, Inline};

fn messages(text: &str) -> Vec<String> {
    parse(text)
        .diagnostics
        .into_iter()
        .map(|d| d.message)
        .collect()
}

fn no_recognised_dimension_error(text: &str) -> Vec<String> {
    let bad: Vec<String> = messages(text)
        .into_iter()
        .filter(|m| m.contains("recognised dimension"))
        .collect();
    assert!(
        bad.is_empty(),
        "dimension failed to parse: {bad:?} in {text:?}: {:?}",
        parse(text).diagnostics
    );
    messages(text)
}

/// The first horizontal space in the body, in points.
fn hspace_pt(source: &str) -> f64 {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => inlines.iter().find_map(|inline| match inline {
                Inline::HSpace { pt, .. } => Some(*pt),
                _ => None,
            }),
            _ => None,
        })
        .expect("the document has no hspace")
}

/// The first vertical space in the body, in points.
fn vspace_pt(source: &str) -> f64 {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::VSpace { pt, .. } => Some(*pt),
            _ => None,
        })
        .expect("the document has no vspace")
}

#[test]
fn setlength_topsep_factor_baselineskip_parses() {
    // set0.tex line: `\setlength{\topsep}{0.6\baselineskip}` (oracle 8.16008pt).
    // The compiler owns no page-geometry values, so like `\textwidth` this
    // parses (no error) and warns `unsupported length expression`.
    let source = "\\documentclass[11pt]{article}\\begin{document}\\setlength{\\topsep}{0.6\\baselineskip}x\\end{document}";
    let rest = no_recognised_dimension_error(source);
    assert!(
        rest.iter().any(|m| m.contains("unsupported length expression")),
        "{rest:?}"
    );
}

#[test]
fn factor_lengths_parse_anywhere_a_dimension_is_accepted() {
    // Oracle: `2\parindent` at 11pt is 34.0pt; `-.5\textwidth` is -180.0pt.
    // Unvalued references parse to zero (the preamble-length convention).
    let h = "\\documentclass[11pt]{article}\\begin{document}x\\hspace{2\\parindent}y\\end{document}";
    assert_eq!(hspace_pt(h), 0.0);
    // The other edge: spaces around the factor and the name.
    let spaced = "\\documentclass[11pt]{article}\\begin{document}x\\hspace{ 2 \\parindent }y\\end{document}";
    assert_eq!(hspace_pt(spaced), 0.0);
    // A negative factor parses too.
    let neg = "\\documentclass{article}\\begin{document}\\setlength{\\topsep}{-.5\\textwidth}x\\end{document}";
    let rest = no_recognised_dimension_error(neg);
    assert!(
        rest.iter().any(|m| m.contains("unsupported length expression")),
        "{rest:?}"
    );
    // A bare `\baselineskip` (factor 1) parses as well.
    let bare = "\\documentclass{article}\\begin{document}\\setlength{\\topsep}{\\baselineskip}x\\end{document}";
    no_recognised_dimension_error(bare);
}

#[test]
fn bare_factor_with_no_unit_still_errors() {
    // Oracle: `\setlength{\topsep}{0.6}` is "Illegal unit of measure".
    let source = "\\documentclass{article}\\begin{document}\\setlength{\\topsep}{0.6}x\\end{document}";
    let found = messages(source);
    assert!(
        found
            .iter()
            .any(|m| m.contains("requires a recognised dimension")),
        "{found:?}"
    );
}

/// A dimension in `pt`, from text of the form `6.57007pt`.
fn pt(text: &str) -> f64 {
    text.strip_suffix("pt").expect("pt suffix").parse().expect("a number")
}

/// The body text of the first paragraph (for `\the` probes).
fn paragraph_text(source: &str) -> String {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(
                inlines
                    .iter()
                    .filter_map(|inline| match inline {
                        Inline::Text { text, .. } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .expect("the document has no paragraph")
}

#[test]
fn newlength_register_reads_back_in_hspace() {
    // set0.tex: `\setlength{\qnumsep}{0.6em}` (oracle 6.57007pt at 11pt),
    // then `\hspace{\qnumsep}`. The shim splices the register's `\the`
    // text, so the space is exactly `\the\qnumsep` parsed — the oracle
    // number itself, not the float product the `0.6em` literal takes.
    let preamble = "\\documentclass[11pt]{article}\\newlength{\\qnumsep}\\setlength{\\qnumsep}{0.6em}";
    let the = paragraph_text(&format!(
        "{preamble}\\begin{{document}}\\the\\qnumsep\\end{{document}}"
    ));
    assert_eq!(the, "6.57007pt", "oracle: 6.57007pt");
    let via_register =
        format!("{preamble}\\begin{{document}}x\\hspace{{\\qnumsep}}y\\end{{document}}");
    assert_eq!(hspace_pt(&via_register), pt(&the));
    // The literal is the same space up to the float-vs-fixed last digit.
    let via_literal =
        format!("{preamble}\\begin{{document}}x\\hspace{{0.6em}}y\\end{{document}}");
    assert!((hspace_pt(&via_register) - hspace_pt(&via_literal)).abs() < 1e-4);
}

#[test]
fn newlength_register_reads_back_in_vspace() {
    let preamble = "\\documentclass[11pt]{article}\\newlength{\\qnumsep}\\setlength{\\qnumsep}{0.6em}";
    let the = paragraph_text(&format!(
        "{preamble}\\begin{{document}}\\the\\qnumsep\\end{{document}}"
    ));
    assert_eq!(the, "6.57007pt", "oracle: 6.57007pt");
    let via_register =
        format!("{preamble}\\begin{{document}}x\\vspace{{\\qnumsep}}y\\end{{document}}");
    assert_eq!(vspace_pt(&via_register), pt(&the));
}

#[test]
fn factor_times_register_in_space_commands() {
    // Oracle: `2\zerolen` (register set to 0pt) is 0.0pt.
    let preamble = "\\documentclass{article}\\newlength{\\mylen}\\setlength{\\mylen}{1em}\\newlength{\\zerolen}\\setlength{\\zerolen}{0pt}";
    let zero = format!("{preamble}\\begin{{document}}x\\hspace{{2\\zerolen}}y\\end{{document}}");
    assert_eq!(hspace_pt(&zero), 0.0);
    // `2\mylen` is the fixed-point product, exactly `\the\dimexpr2\mylen`
    // parsed: TeX's `<factor><internal dimen>`, not the float double of the
    // bare space (the two differ in the last digit). Spaces at either edge
    // agree exactly.
    let the_doubled = paragraph_text(&format!(
        "{preamble}\\begin{{document}}\\the\\dimexpr2\\mylen\\relax\\end{{document}}"
    ));
    let doubled = format!("{preamble}\\begin{{document}}x\\hspace{{2\\mylen}}y\\end{{document}}");
    let bare = format!("{preamble}\\begin{{document}}x\\hspace{{\\mylen}}y\\end{{document}}");
    let spaced = format!("{preamble}\\begin{{document}}x\\hspace{{ 2 \\mylen }}y\\end{{document}}");
    assert_eq!(hspace_pt(&doubled), pt(&the_doubled));
    assert!((hspace_pt(&doubled) - 2.0 * hspace_pt(&bare)).abs() < 1e-4);
    assert_eq!(hspace_pt(&spaced), hspace_pt(&doubled));
    // A negative factor negates.
    let neg = format!("{preamble}\\begin{{document}}x\\hspace{{-\\mylen}}y\\end{{document}}");
    assert_eq!(hspace_pt(&neg), -hspace_pt(&bare));
    // The empty argument still errors (the empty case).
    let empty = format!("{preamble}\\begin{{document}}x\\hspace{{}}y\\end{{document}}");
    assert!(
        messages(&empty)
            .iter()
            .any(|m| m.contains("requires a recognised dimension")),
        "{:?}",
        messages(&empty)
    );
}

#[test]
fn hspace_with_undeclared_register_still_errors_naming_it() {
    // Oracle: `\hspace{\nosuchlen}` is "Undefined control sequence".
    let source =
        "\\documentclass{article}\\begin{document}x\\hspace{\\nosuchlen}y\\end{document}";
    let found = messages(source);
    assert!(
        found
            .iter()
            .any(|m| m.contains("requires a recognised dimension") && m.contains("nosuchlen")),
        "{found:?}"
    );
}
