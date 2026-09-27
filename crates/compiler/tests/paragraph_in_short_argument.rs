//! A blank line or `\par` inside a short (non-`\long`) argument is an error:
//! pdflatex reports "Paragraph ended before \text@command was complete."
//! for `\textbf`/`\emph` (both are `\DeclareTextFontCommand`, read through
//! the kernel's short `\text@command`), while a `\long` argument (such as a
//! plain `\newcommand` macro's) accepts the paragraph break. FlashTeX used
//! to read the balanced-brace argument as an ordinary group and report no
//! diagnostic at all.
use flashtex_compiler::parser::parse;

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn errors(src: &str) -> Vec<String> {
    parse(src)
        .diagnostics
        .iter()
        .filter(|d| d.severity == flashtex_compiler::diagnostics::Severity::Error)
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn blank_line_in_textbf_reports_paragraph_ended() {
    let messages = errors(&doc("Before \\textbf{a\n\nb} after."));
    assert!(
        messages
            .iter()
            .any(|m| m.contains(r"Paragraph ended before \text@command was complete")),
        "expected the pdflatex paragraph-ended error, got {messages:?}"
    );
}

#[test]
fn par_in_textbf_reports_paragraph_ended() {
    let messages = errors(&doc(r"Before \textbf{a\par b} after."));
    assert!(
        messages
            .iter()
            .any(|m| m.contains(r"Paragraph ended before \text@command was complete")),
        "expected the pdflatex paragraph-ended error, got {messages:?}"
    );
}

#[test]
fn blank_line_in_emph_reports_paragraph_ended() {
    let messages = errors(&doc("Before \\emph{a\n\nb} after."));
    assert!(
        messages
            .iter()
            .any(|m| m.contains(r"Paragraph ended before \text@command was complete")),
        "expected the pdflatex paragraph-ended error, got {messages:?}"
    );
}

#[test]
fn par_nested_in_braces_still_reports_paragraph_ended() {
    let messages = errors(&doc(r"Before \textbf{a{\par}b} after."));
    assert!(
        messages
            .iter()
            .any(|m| m.contains(r"Paragraph ended before \text@command was complete")),
        "expected the pdflatex paragraph-ended error, got {messages:?}"
    );
}

#[test]
fn blank_line_in_section_title_is_an_error() {
    let messages = errors(&doc("Before.\n\n\\section{A\n\nB}\n\nAfter."));
    assert!(
        !messages.is_empty(),
        "a blank line inside \\section's short argument must report an error"
    );
}

#[test]
fn blank_line_in_title_is_an_error() {
    let src = "\\documentclass{article}\n\\title{A\n\nB}\n\\begin{document}\nBody.\n\\end{document}\n";
    let messages = errors(src);
    assert!(
        !messages.is_empty(),
        "a blank line inside \\title's argument must report an error"
    );
}

#[test]
fn blank_line_in_long_newcommand_argument_is_accepted() {
    let src = doc("\\newcommand{\\foo}[1]{[#1]}\nBefore \\foo{a\n\nb} after.");
    let parsed = parse(&src);
    assert!(
        !parsed
            .diagnostics
            .iter()
            .any(|d| d.severity == flashtex_compiler::diagnostics::Severity::Error),
        "a \\long macro argument must accept a blank line, got {:?}",
        parsed.diagnostics
    );
}
