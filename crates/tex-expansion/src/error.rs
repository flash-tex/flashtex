use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Diagnostic { severity: Severity::Error, message: message.into(), span }
    }
    pub fn warning(message: impl Into<String>, span: Span) -> Self {
        Diagnostic { severity: Severity::Warning, message: message.into(), span }
    }
}

/// The diagnostic for a run that goes past `Limits::max_output_tokens`,
/// in TeX's capacity wording. Unlike TeX's other capacity stops, nothing
/// typesets the rest of the document afterwards, and the message says so.
pub fn output_limit_message(limit: u64) -> String {
    format!(
        "{OUTPUT_LIMIT_PREFIX}{limit}]; expansion stopped here and the rest of the document was not typeset."
    )
}

const OUTPUT_LIMIT_PREFIX: &str = "TeX capacity exceeded, sorry [output token limit=";

/// `message` is [`output_limit_message`]'s.
pub fn is_output_limit(message: &str) -> bool {
    message.starts_with(OUTPUT_LIMIT_PREFIX)
}

/// Resource limits so a mid-edit infinite macro loop (very common while
/// typing) degrades to a diagnostic instead of hanging or OOMing the IDE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_expansion_steps: u64,
    pub max_output_tokens: u64,
    pub max_conditional_depth: u32,
    pub max_group_depth: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_expansion_steps: 2_000_000,
            max_output_tokens: 2_000_000,
            max_conditional_depth: 10_000,
            max_group_depth: 10_000,
        }
    }
}
