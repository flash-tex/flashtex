//! Diagnostics in the shape `docs/contracts/runtime-v1.md` specifies.

use crate::json::{str_, Value};
use crate::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

/// Machine-readable category, serialized as the optional runtime-v1 `code`.
///
/// Consumers classify by this rather than by message wording; messages stay
/// human prose and may change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    /// A command no LaTeX layer this compiler knows of defines (most often a
    /// typo); may carry a did-you-mean `suggestion`.
    UnknownCommand,
    /// Real LaTeX (a known command, environment, package or key) that this
    /// compiler does not implement yet.
    UnsupportedFeature,
    /// Malformed input: unbalanced braces, unterminated environments,
    /// misplaced `&`, missing arguments.
    SyntaxError,
    /// The preview is right but the exported PDF cannot reproduce it.
    ExportLimitation,
    /// Rendered, but visibly different from what pdfLaTeX would produce.
    FidelityNote,
    /// Input the author must fix (missing include, undefined reference,
    /// duplicate label) that the compiler worked around.
    RecoveredInput,
}

impl DiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticCode::UnknownCommand => "unknown_command",
            DiagnosticCode::UnsupportedFeature => "unsupported_feature",
            DiagnosticCode::SyntaxError => "syntax_error",
            DiagnosticCode::ExportLimitation => "export_limitation",
            DiagnosticCode::FidelityNote => "fidelity_note",
            DiagnosticCode::RecoveredInput => "recovered_input",
        }
    }
}

/// One underlined span in a rustc-style report (`labels[]` on the wire).
///
/// `span` is serialized as a runtime-v1 `source` object
/// (`{path, start_byte, end_byte}`). Exactly one label in a non-empty list
/// should have `primary: true`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel {
    pub span: Span,
    pub text: String,
    pub primary: bool,
}

/// Optional mechanical edit for `help.replacement` (issue #277).
///
/// On the wire this is `{source: {path, start_byte, end_byte}, text}` —
/// the same `source` object as `labels[].source` — so a replacement can
/// target a different document than the diagnostic (e.g. an included file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticReplacement {
    pub span: Span,
    pub text: String,
}

/// Suggested fix (`help` on the wire). `replacement` is omitted when the
/// help is advice only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticHelp {
    pub message: String,
    pub replacement: Option<DiagnosticReplacement>,
}

/// Default code for a diagnostic from the compiler's own message conventions.
///
/// This lets every existing construction site carry a code without being
/// touched. Sites whose category needs more than wording (unknown versus
/// known-unsupported commands) set it explicitly via
/// [`Diagnostic::command_error`] / [`Diagnostic::with_code`]. New sites should
/// either follow these wordings or set a code explicitly; `None` (for example
/// request-validation errors) omits the field from JSON.
pub fn default_code(message: &str) -> Option<DiagnosticCode> {
    let has = |needles: &[&str]| needles.iter().any(|n| message.contains(n));
    if has(&["PDF export", "page-unit bounds"]) {
        Some(DiagnosticCode::ExportLimitation)
    } else if has(&[
        "has no glyph",
        "could not shape",
        "differ from pdfLaTeX",
        "accent glyph",
        "does not stretch",
        "did not converge",
    ]) {
        Some(DiagnosticCode::FidelityNote)
    } else if has(&[
        "undefined reference",
        "Reference `",
        "duplicate \\label",
        "was given an empty",
        "included file not found",
        "include path",
        "include cycle",
        "include depth",
        "project-relative path",
        "recursion limit",
        "received an empty required argument",
    ]) {
        Some(DiagnosticCode::RecoveredInput)
    } else if has(&[
        "not supported",
        "not implemented",
        "unsupported",
        "supports only",
        "does not support",
        "exceeds the supported",
        "recognised dimension",
    ]) {
        Some(DiagnosticCode::UnsupportedFeature)
    } else if has(&[
        "unmatched",
        "unterminated",
        "missing its",
        "misplaced",
        "stray",
        "duplicate script",
        "script marker",
        "unexpected math delimiter",
        "does not match",
        "no matching",
        "requires a",
        "requires math mode",
        "only supported inside",
        "argument count",
        "cannot redefine",
        "references #",
    ]) {
        Some(DiagnosticCode::SyntaxError)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    /// `None` where the compiler genuinely cannot map the problem to source.
    pub span: Option<Span>,
    /// What was rendered provisionally instead, or `None` if nothing was recovered.
    pub recovery: Option<String>,
    /// Serialized as `code`; `None` omits the field.
    pub code: Option<DiagnosticCode>,
    /// Replacement text for the source range (e.g. `\alpha` for `\alpah`),
    /// serialized as `suggestion`; `None` omits the field.
    pub suggestion: Option<String>,
    /// Extra underlined spans; omitted from JSON when empty.
    pub labels: Vec<DiagnosticLabel>,
    /// Extra `= note:` strings; omitted from JSON when empty.
    pub notes: Vec<String>,
    /// Suggested fix; omitted from JSON when `None`.
    pub help: Option<DiagnosticHelp>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: Option<Span>, recovery: Option<String>) -> Self {
        let message = message.into();
        Diagnostic {
            severity: Severity::Error,
            code: default_code(&message),
            message,
            span,
            recovery,
            suggestion: None,
            labels: Vec::new(),
            notes: Vec::new(),
            help: None,
        }
    }

    pub fn warning(
        message: impl Into<String>,
        span: Option<Span>,
        recovery: Option<String>,
    ) -> Self {
        let message = message.into();
        Diagnostic {
            severity: Severity::Warning,
            code: default_code(&message),
            message,
            span,
            recovery,
            suggestion: None,
            labels: Vec::new(),
            notes: Vec::new(),
            help: None,
        }
    }

    pub fn with_code(mut self, code: DiagnosticCode) -> Self {
        self.code = Some(code);
        self
    }

    pub fn with_label(mut self, span: Span, text: impl Into<String>, primary: bool) -> Self {
        self.labels.push(DiagnosticLabel {
            span,
            text: text.into(),
            primary,
        });
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_help(mut self, message: impl Into<String>) -> Self {
        self.help = Some(DiagnosticHelp {
            message: message.into(),
            replacement: None,
        });
        self
    }

    /// Like [`with_help`], or a no-op when there is nothing useful to say.
    /// Does not replace help that is already set (so `command_error` can
    /// attach a did-you-mean replacement and callers can still chain this).
    pub fn with_optional_help<S: Into<String>>(self, message: Option<S>) -> Self {
        if self.help.is_some() {
            return self;
        }
        match message {
            Some(message) => self.with_help(message),
            None => self,
        }
    }

    /// Attach a byte-range edit to an already-set `help`. No-op when help is
    /// absent, so callers can chain `with_help(...).with_replacement(...)`.
    pub fn with_replacement(mut self, span: Span, text: impl Into<String>) -> Self {
        if let Some(help) = &mut self.help {
            help.replacement = Some(DiagnosticReplacement {
                span,
                text: text.into(),
            });
        }
        self
    }

    /// An error about command `name` (without the backslash):
    /// `unknown_command` with a did-you-mean suggestion when no known LaTeX
    /// layer defines it, otherwise `unsupported_feature`.
    pub fn command_error(
        name: &str,
        message: impl Into<String>,
        span: Option<Span>,
        recovery: Option<String>,
    ) -> Self {
        let mut diagnostic = Diagnostic::error(message, span, recovery);
        if crate::vocabulary::is_known_command(name) {
            diagnostic.code = Some(DiagnosticCode::UnsupportedFeature);
        } else {
            diagnostic.code = Some(DiagnosticCode::UnknownCommand);
            // Only a *unique* closest match becomes a mechanical edit. The
            // editor applies `suggestion`/`help.replacement` on Tab without the
            // author re-reading it, so a tie broken by list order would be a
            // silent wrong rewrite. Ambiguous cases still get prose help naming
            // every candidate, and the author picks.
            match crate::vocabulary::closest_commands(name).as_slice() {
                [] => {}
                [known] => {
                    let text = format!("\\{known}");
                    diagnostic.suggestion = Some(text.clone());
                    diagnostic = diagnostic.with_help(format!("did you mean \\{known}?"));
                    if let Some(span) = span {
                        diagnostic = diagnostic.with_replacement(span, text);
                    }
                }
                candidates => {
                    let list = candidates
                        .iter()
                        .map(|c| format!("\\{c}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    diagnostic = diagnostic.with_help(format!(
                        "did you mean one of {list}? \
                         (no automatic fix: they are equally close)"
                    ));
                }
            }
        }
        diagnostic
    }

    /// An error about environment `name`, classified like
    /// [`Diagnostic::command_error`]. No suggestion: the span covers `\begin`,
    /// not the name.
    pub fn environment_error(
        name: &str,
        message: impl Into<String>,
        span: Option<Span>,
        recovery: Option<String>,
    ) -> Self {
        Diagnostic::error(message, span, recovery).with_environment_code(name)
    }

    /// Warning counterpart of [`Diagnostic::environment_error`].
    pub fn environment_warning(
        name: &str,
        message: impl Into<String>,
        span: Option<Span>,
        recovery: Option<String>,
    ) -> Self {
        Diagnostic::warning(message, span, recovery).with_environment_code(name)
    }

    fn with_environment_code(self, name: &str) -> Self {
        self.with_code(if crate::vocabulary::is_known_environment(name) {
            DiagnosticCode::UnsupportedFeature
        } else {
            DiagnosticCode::UnknownCommand
        })
    }

    pub fn to_json(&self, path: &str) -> Value {
        self.to_json_with_paths(&[path])
    }

    /// Serialize with the path belonging to the document carried by the span.
    pub fn to_json_with_paths(&self, paths: &[&str]) -> Value {
        let mut v = Value::obj();
        v.set("severity", str_(self.severity.as_str()));
        v.set("message", str_(self.message.clone()));
        if let Some(code) = self.code {
            v.set("code", str_(code.as_str()));
        }
        if let Some(suggestion) = &self.suggestion {
            v.set("suggestion", str_(suggestion.clone()));
        }
        if !self.labels.is_empty() {
            v.set(
                "labels",
                Value::Arr(
                    self.labels
                        .iter()
                        .map(|label| {
                            let mut o = Value::obj();
                            o.set("source", source_json(label.span, paths));
                            o.set("text", str_(label.text.clone()));
                            o.set("primary", Value::Bool(label.primary));
                            o
                        })
                        .collect(),
                ),
            );
        }
        if !self.notes.is_empty() {
            v.set(
                "notes",
                Value::Arr(self.notes.iter().cloned().map(str_).collect()),
            );
        }
        if let Some(help) = &self.help {
            let mut h = Value::obj();
            h.set("message", str_(help.message.clone()));
            if let Some(repl) = &help.replacement {
                let mut r = Value::obj();
                r.set("source", source_json(repl.span, paths));
                r.set("text", str_(repl.text.clone()));
                h.set("replacement", r);
            }
            v.set("help", h);
        }
        v.set(
            "source",
            match self.span {
                Some(s) => source_json(s, paths),
                None => Value::Null,
            },
        );
        v.set(
            "recovery",
            match &self.recovery {
                Some(r) => str_(r.clone()),
                None => Value::Null,
            },
        );
        v
    }
}

fn source_json(span: Span, paths: &[&str]) -> Value {
    let mut src = Value::obj();
    src.set("path", str_(paths.get(span.document.0).copied().unwrap_or("")));
    src.set("start_byte", Value::Num(span.start as f64));
    src.set("end_byte", Value::Num(span.end as f64));
    src
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_codes_follow_the_compiler_message_conventions() {
        use DiagnosticCode::*;
        for (message, code) in [
            (
                "'─' (U+2500) will not survive PDF export: stand-in",
                Some(ExportLimitation),
            ),
            (
                "fraction rule geometry exceeds the runtime-v1 page-unit bounds",
                Some(ExportLimitation),
            ),
            (
                "Times-Roman has no glyph for 'x' (U+0078)",
                Some(FidelityNote),
            ),
            (
                "widths differ from pdfLaTeX's msbm10/cmsy10",
                Some(FidelityNote),
            ),
            (
                "\\hat has no representable accent glyph in the compiler's base-14 fonts",
                Some(FidelityNote),
            ),
            ("undefined reference 'k'", Some(RecoveredInput)),
            (
                "duplicate \\label{k}; the second definition wins",
                Some(RecoveredInput),
            ),
            (
                "included file not found: looked for 'a' and 'a.tex'",
                Some(RecoveredInput),
            ),
            (
                "packages tikz are recognised but not implemented",
                Some(UnsupportedFeature),
            ),
            (
                "\\setlist keys x are recognised but not implemented",
                Some(UnsupportedFeature),
            ),
            (
                "\\includegraphics is unsupported; image loading is not implemented",
                Some(UnsupportedFeature),
            ),
            (
                "\\mathbb supports only capital letters A-Z, not \"ab\"",
                Some(UnsupportedFeature),
            ),
            (
                "\\hspace requires a recognised dimension, got 'x'",
                Some(UnsupportedFeature),
            ),
            ("unmatched '{' — group never closed", Some(SyntaxError)),
            (
                "unterminated environment 'x' — no matching \\end",
                Some(SyntaxError),
            ),
            ("misplaced alignment tab character &", Some(SyntaxError)),
            ("inline math is missing its closing '$'", Some(SyntaxError)),
            ("\\end{a} does not match \\begin{b}", Some(SyntaxError)),
            ("\\textbf requires a braced argument", Some(SyntaxError)),
            (
                "\\item is only supported inside itemize or enumerate",
                Some(SyntaxError),
            ),
            ("layout_capabilities must be a list", None),
        ] {
            assert_eq!(default_code(message), code, "{message}");
        }
    }

    #[test]
    fn code_and_suggestion_are_omitted_from_json_when_absent() {
        let plain = Diagnostic::error("layout_capabilities must be a list", None, None);
        let json = crate::json::write(&plain.to_json(""));
        assert!(
            !json.contains("\"code\"") && !json.contains("suggestion"),
            "{json}"
        );
        assert!(
            !json.contains("\"labels\"") && !json.contains("\"notes\"") && !json.contains("\"help\""),
            "{json}"
        );
        let typo = Diagnostic::command_error("alpah", "\\alpah is not supported", None, None);
        let json = crate::json::write(&typo.to_json(""));
        assert!(json.contains(r#""code":"unknown_command""#), "{json}");
        assert!(json.contains(r#""suggestion":"\\alpha""#), "{json}");
    }

    #[test]
    fn unknown_command_did_you_mean_emits_help_replacement() {
        let span = Span::new(5, 11);
        let typo = Diagnostic::command_error("alpah", "\\alpah is not supported", Some(span), None);
        assert_eq!(typo.suggestion.as_deref(), Some("\\alpha"));
        let help = typo.help.as_ref().expect("help");
        assert_eq!(help.message, "did you mean \\alpha?");
        let repl = help.replacement.as_ref().expect("replacement");
        assert_eq!(repl.span, span);
        assert_eq!(repl.text, "\\alpha");
        let json = crate::json::write(&typo.to_json("main.tex"));
        assert!(json.contains(r#""suggestion":"\\alpha""#), "{json}");
        assert!(
            json.contains(r#""help":{"message":"did you mean \\alpha?","replacement":{"source":{"end_byte":11,"path":"main.tex","start_byte":5},"text":"\\alpha"}}"#),
            "{json}"
        );
    }

    #[test]
    fn labels_notes_and_help_are_emitted_only_when_set() {
        let span = Span::new(0, 6);
        let with = Diagnostic::error("\\tilde is not supported", Some(span), Some("skipped".into()))
            .with_label(span, "this command", true)
            .with_note("\\tilde is a math accent")
            .with_help("wrap it in math: \\(\\tilde{c}\\)")
            .with_replacement(span, "\\(\\tilde{c}\\)");
        let json = crate::json::write(&with.to_json("notes.tex"));
        assert!(json.contains(r#""labels":[{"primary":true,"source":{"end_byte":6,"path":"notes.tex","start_byte":0},"text":"this command"}]"#), "{json}");
        assert!(json.contains(r#""notes":["\\tilde is a math accent"]"#), "{json}");
        assert!(json.contains(r#""help":{"message":"wrap it in math: \\(\\tilde{c}\\)","replacement":{"source":{"end_byte":6,"path":"notes.tex","start_byte":0},"text":"\\(\\tilde{c}\\)"}}"#), "{json}");
        let without = Diagnostic::error("\\tilde is not supported", Some(span), Some("skipped".into()));
        let json = crate::json::write(&without.to_json("notes.tex"));
        assert!(!json.contains("\"labels\"") && !json.contains("\"notes\"") && !json.contains("\"help\""), "{json}");
    }
}
