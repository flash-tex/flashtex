//! Spans and diagnostics in the shape `docs/contracts/runtime-v1.md` specifies.
//!
//! Offsets are zero-based, end-exclusive UTF-8 byte offsets into the text that
//! was parsed. The `source.path` is supplied by the caller at serialisation time,
//! exactly as `crates/compiler/src/diagnostics.rs` does.

use std::fmt;

/// A half-open UTF-8 byte range `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }

    /// Smallest span covering both.
    pub fn join(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// The text this span selects. Panics only if the span is not on char
    /// boundaries of `src`, which the parser guarantees for spans it produces.
    pub fn slice<'a>(&self, src: &'a str) -> &'a str {
        &src[self.start..self.end]
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    /// `None` where the problem cannot be mapped to source text.
    pub span: Option<Span>,
    /// What was done provisionally instead, or `None` if nothing was recovered.
    pub recovery: Option<String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: Option<Span>, recovery: Option<String>) -> Self {
        Diagnostic {
            severity: Severity::Error,
            message: message.into(),
            span,
            recovery,
        }
    }

    pub fn warning(
        message: impl Into<String>,
        span: Option<Span>,
        recovery: Option<String>,
    ) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            message: message.into(),
            span,
            recovery,
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Serialise as the runtime-v1 diagnostic object:
    /// `{"severity","message","source":{"path","start_byte","end_byte"}|null,"recovery"|null}`.
    pub fn to_json(&self, path: &str) -> String {
        let mut out = String::new();
        out.push_str("{\"severity\":");
        push_json_string(&mut out, self.severity.as_str());
        out.push_str(",\"message\":");
        push_json_string(&mut out, &self.message);
        out.push_str(",\"source\":");
        match self.span {
            Some(s) => {
                out.push_str("{\"path\":");
                push_json_string(&mut out, path);
                out.push_str(&format!(
                    ",\"start_byte\":{},\"end_byte\":{}}}",
                    s.start, s.end
                ));
            }
            None => out.push_str("null"),
        }
        out.push_str(",\"recovery\":");
        match &self.recovery {
            Some(r) => push_json_string(&mut out, r),
            None => out.push_str("null"),
        }
        out.push('}');
        out
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.severity.as_str(), self.message)?;
        if let Some(s) = self.span {
            write!(f, " at bytes {s}")?;
        }
        if let Some(r) = &self.recovery {
            write!(f, " (recovery: {r})")?;
        }
        Ok(())
    }
}

fn push_json_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_shape_matches_runtime_v1() {
        let d = Diagnostic::error(
            "bad \"thing\"",
            Some(Span::new(3, 9)),
            Some("skipped".into()),
        );
        assert_eq!(
            d.to_json("refs.bib"),
            r#"{"severity":"error","message":"bad \"thing\"","source":{"path":"refs.bib","start_byte":3,"end_byte":9},"recovery":"skipped"}"#
        );
        let w = Diagnostic::warning("w", None, None);
        assert_eq!(
            w.to_json("x"),
            r#"{"severity":"warning","message":"w","source":null,"recovery":null}"#
        );
    }
}
