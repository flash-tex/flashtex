//! The compiler's runtime-v1 diagnostic code has to survive the display list.
//!
//! `docs/contracts/runtime-v1.md` tells consumers to "classify by `code`, not
//! by `message` wording". That is only possible if the code the compiler set
//! is the code they receive: the conversion used to replace every one of them
//! with the literal string `"compiler"`, so anything the compiler produced was
//! indistinguishable from anything else it produced.

use flashtex_compiler::diagnostics::{Diagnostic as CompilerDiagnostic, DiagnosticCode};
use flashtex_render_pipeline::display::Diagnostic;

fn code_of(d: CompilerDiagnostic) -> String {
    Diagnostic::from_compiler(&d, &["main.tex"]).code
}

#[test]
fn every_compiler_code_reaches_the_display_list_intact() {
    let codes = [
        DiagnosticCode::UnknownCommand,
        DiagnosticCode::UnsupportedFeature,
        DiagnosticCode::SyntaxError,
        DiagnosticCode::ExportLimitation,
        DiagnosticCode::FidelityNote,
        DiagnosticCode::RecoveredInput,
    ];
    for code in codes {
        let d = CompilerDiagnostic::error("anything at all", None, None).with_code(code);
        assert_eq!(code_of(d), code.as_str());
    }
}

#[test]
fn a_syntax_error_is_distinguishable_from_an_unsupported_feature() {
    // The distinction the compiler-side work in #365 exists to make: LaTeX
    // pdflatex would refuse, versus real LaTeX this compiler has not
    // implemented. Both used to arrive as `"compiler"`.
    let syntax = CompilerDiagnostic::error("misplaced alignment tab character &", None, None)
        .with_code(DiagnosticCode::SyntaxError);
    let unsupported = CompilerDiagnostic::error("\\footnote is not supported in math mode", None, None)
        .with_code(DiagnosticCode::UnsupportedFeature);
    assert_ne!(code_of(syntax), code_of(unsupported));
}

#[test]
fn a_codeless_diagnostic_still_names_its_producer() {
    // A few compiler diagnostics (request validation) deliberately carry no
    // code. They must not arrive with an empty one.
    let mut d = CompilerDiagnostic::error("no code here", None, None);
    d.code = None;
    assert_eq!(code_of(d), "compiler");
}

#[test]
fn severity_and_recovery_are_carried_through_unchanged() {
    let d = CompilerDiagnostic::warning("a note", None, Some("did something instead".into()))
        .with_code(DiagnosticCode::FidelityNote);
    let converted = Diagnostic::from_compiler(&d, &["main.tex"]);
    assert_eq!(converted.severity, flashtex_render_pipeline::display::Severity::Warning);
    assert_eq!(converted.recovery.as_deref(), Some("did something instead"));
}
