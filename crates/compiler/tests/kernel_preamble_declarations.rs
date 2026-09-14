//! `\NeedsTeXFormat`, `\ProvidesClass`, `\ProvidesPackage` and
//! `\ProvidesFile` are `.cls`/`.sty` declarations (or inert metadata) with
//! no visible output, so this compiler accepts them as silent no-ops,
//! including their optional `[release info]` date. `\DocumentMetadata`
//! (LaTeX2e 2022+) is accepted before `\documentclass` with a warning
//! naming the ignored keys, and is a real error after `\documentclass`,
//! exactly like real LaTeX.

use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;

fn compile(text: &str) -> CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

fn words(out: &CompileOutput) -> Vec<String> {
    out.pages
        .iter()
        .flat_map(|p| p.items.iter())
        .map(|i| i.text.clone())
        .collect()
}

const BODY: &str = "\\begin{document}\nOne paragraph of body text.\n\\end{document}\n";

fn bare() -> CompileOutput {
    compile(&format!("\\documentclass{{article}}\n{BODY}"))
}

#[test]
fn silent_preamble_declarations_are_accepted_no_ops() {
    let bare_words = words(&bare());
    for preamble in [
        "\\NeedsTeXFormat{LaTeX2e}",
        "\\NeedsTeXFormat{LaTeX2e}[2022/06/01]",
        "\\ProvidesClass{article}",
        "\\ProvidesClass{article}[2022/07/02 v1.4n]",
        "\\ProvidesPackage{hyperref}",
        "\\ProvidesPackage{hyperref}[2023-11-26 v7.01d]",
        "\\ProvidesFile{foo.cfg}",
        "\\ProvidesFile{foo.cfg}[2024/01/01]",
    ] {
        let out = compile(&format!("\\documentclass{{article}}\n{preamble}\n{BODY}"));
        assert!(
            out.diagnostics.is_empty(),
            "{preamble}: {:?}",
            out.diagnostics
        );
        assert_eq!(words(&out), bare_words, "{preamble}: no visible effect");
    }
}

#[test]
fn needstexformat_before_documentclass_is_silent() {
    let bare_words = words(&bare());
    let out = compile(&format!(
        "\\NeedsTeXFormat{{LaTeX2e}}[2022/06/01]\n\\documentclass{{article}}\n{BODY}"
    ));
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert_eq!(words(&out), bare_words);
}

#[test]
fn missing_required_argument_is_a_parse_error() {
    for command in [
        "NeedsTeXFormat",
        "ProvidesClass",
        "ProvidesPackage",
        "ProvidesFile",
    ] {
        let out = compile(&format!("\\{command}\n{BODY}"));
        let errors: Vec<_> = out
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(
            errors.iter().any(|d| d.message.contains(command)),
            "\\{command} without {{...}} should be a real error: {:?}",
            out.diagnostics
        );
    }
}

#[test]
fn document_metadata_before_documentclass_warns_naming_keys() {
    let bare_words = words(&bare());
    let out = compile(&format!(
        "\\DocumentMetadata{{lang=en-US}}\n\\documentclass{{article}}\n{BODY}"
    ));
    assert_eq!(
        out.diagnostics.len(),
        1,
        "exactly one diagnostic: {:?}",
        out.diagnostics
    );
    let diagnostic = &out.diagnostics[0];
    assert_eq!(diagnostic.severity, Severity::Warning);
    assert!(
        diagnostic.message.contains("lang"),
        "warning names the ignored key: {:?}",
        diagnostic.message
    );
    assert_eq!(words(&out), bare_words, "no stray output text");
}

#[test]
fn document_metadata_names_every_ignored_key() {
    let out = compile(&format!(
        "\\DocumentMetadata{{lang=en-US,pdfversion=1.7}}\n\\documentclass{{article}}\n{BODY}"
    ));
    assert_eq!(
        out.diagnostics.len(),
        1,
        "exactly one diagnostic: {:?}",
        out.diagnostics
    );
    assert_eq!(out.diagnostics[0].severity, Severity::Warning);
    assert!(
        out.diagnostics[0].message.contains("lang"),
        "warning names lang: {:?}",
        out.diagnostics[0].message
    );
    assert!(
        out.diagnostics[0].message.contains("pdfversion"),
        "warning names pdfversion: {:?}",
        out.diagnostics[0].message
    );
}

#[test]
fn document_metadata_after_documentclass_is_an_error() {
    let bare_words = words(&bare());
    let out = compile(&format!(
        "\\documentclass{{article}}\n\\DocumentMetadata{{lang=en-US}}\n{BODY}"
    ));
    let errors: Vec<_> = out
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("DocumentMetadata")),
        "a real error, not a warning: {:?}",
        out.diagnostics
    );
    assert_eq!(words(&out), bare_words, "no stray output text");
}
