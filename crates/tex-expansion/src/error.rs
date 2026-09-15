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

/// Resource limits so a mid-edit infinite macro loop (very common while
/// typing) degrades to a diagnostic instead of hanging or OOMing the IDE.
#[derive(Debug, Clone, Copy)]
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
