//! Diagnostics dropped inside parsed section titles (issue #38).
//!
//! A section title is set from a flattened token list
//! (`parser::inlines_from_tokens`), not re-parsed, and its catch-all used to
//! swallow every command it could not set without a word — so
//! `\section*{A \nosuchcommand B}` lost the "not supported" diagnostic the
//! same tokens raise in body text (and pdflatex's own
//! `! Undefined control sequence.` for the same input). These tests pin the
//! title/body parity: unknown commands report in both places, while the
//! commands a title legitimately carries stay silent.

use flashtex_compiler::parser;

fn doc(body: &str) -> String {
    format!("\\documentclass[10pt]{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn messages(source: &str) -> Vec<String> {
    parser::parse(source)
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

/// An unknown command in a section title reports exactly what the same
/// tokens report in body text — one diagnostic, not zero.
#[test]
fn unknown_command_in_a_section_title_reports_like_body_text() {
    let title = doc("\\section*{A \\nosuchcommand B}\nText.");
    let body = doc("A \\nosuchcommand B\n");
    let title_messages = messages(&title);
    assert_eq!(
        title_messages,
        ["\\nosuchcommand is not supported by this compiler version"]
    );
    assert_eq!(title_messages, messages(&body));
}

/// The diagnostic points at the command inside the title, not at the
/// `\section` token, so hover and hit-testing land on the culprit.
#[test]
fn section_title_diagnostic_span_covers_the_command() {
    let source = doc("\\section*{A \\nosuchcommand B}\nText.");
    let parsed = parser::parse(&source);
    assert_eq!(parsed.diagnostics.len(), 1);
    let span = parsed.diagnostics[0].span.expect("a span on the diagnostic");
    assert_eq!(&source[span.start..span.end], "\\nosuchcommand");
}

/// `\hfill` and `\normalfont` are set (fill glue; a face reset), not
/// dropped: pdflatex compiles `\section*{\hfill\normalfont Title}` clean,
/// so no diagnostic may fire for them.
#[test]
fn hfill_and_normalfont_in_a_section_title_stay_silent() {
    let parsed = parser::parse(&doc("\\section*{\\hfill\\normalfont Title}\nText."));
    assert!(
        parsed.diagnostics.is_empty(),
        "handled title commands must not report: {:?}",
        parsed.diagnostics
    );
}

/// `\protect` marks the next command robust and sets nothing, so it stays
/// silent in a title exactly as expansion leaves it silent in body text.
#[test]
fn protect_in_a_section_title_stays_silent() {
    let parsed = parser::parse(&doc("\\section{A\\protect B}\nText."));
    assert!(
        parsed.diagnostics.is_empty(),
        "\\protect must not report: {:?}",
        parsed.diagnostics
    );
}

/// Commands a heading, caption or `\maketitle` field routinely carries and
/// this pass correctly ignores or sets must stay silent, including under
/// `\tableofcontents`, which reads every title a second time; pdflatex
/// compiles this clean.
#[test]
fn routine_title_commands_stay_silent() {
    let source = format!(
        "\\documentclass{{article}}\n\\usepackage{{amsmath}}\n\\title{{T\\thanks{{x}}}}\n\\author{{A \\and B}}\n\\begin{{document}}\n\\maketitle\n\\tableofcontents\n{}\n\\end{{document}}\n",
        "\\section[Short]{Long \\emph{e} \\LaTeX{} \\label{s}\\protect\\footnote{n} \\ref{s} \\S1 \\ldots}\n\\begin{figure}\\caption{Cap \\label{f} \\textbf{b}}\\end{figure}\nText."
    );
    let parsed = parser::parse(&source);
    assert!(
        parsed.diagnostics.iter().all(|d| !d.message.contains("not supported")),
        "routine title commands must not report: {:?}",
        parsed.diagnostics
    );
}

/// The heading is read again for the table of contents; the unknown
/// command still reports once, at its own span.
#[test]
fn a_title_read_twice_reports_once() {
    let source = format!(
        "\\documentclass{{article}}\n\\begin{{document}}\n\\tableofcontents\n{}\n\\end{{document}}\n",
        "\\section{A \\nosuchcommand B}\nText."
    );
    assert_eq!(
        messages(&source),
        ["\\nosuchcommand is not supported by this compiler version"]
    );
}

/// A caption goes through the same pass and reports the same way.
#[test]
fn unknown_command_in_a_caption_reports() {
    let source = doc("\\begin{figure}\\caption{A \\nosuchcommand B}\\end{figure}\nText.");
    assert_eq!(
        messages(&source),
        ["\\nosuchcommand is not supported by this compiler version"]
    );
}

/// `\ensuremath{<x>}` in a title is `$<x>$` (latex.ltx), so `\alpha`
/// inside it is math, not an unsupported text command: one inline formula
/// spanning the call, and no diagnostic (pdflatex sets the alpha).
#[test]
fn ensuremath_in_a_section_title_is_inline_math() {
    let source = doc("\\section{A \\ensuremath{\\alpha^2} B}\nText.");
    let parsed = parser::parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let heading = format!("{:?}", parsed.blocks.first().expect("a heading"));
    assert!(heading.contains("Math"), "the group is set as math: {heading}");
}
