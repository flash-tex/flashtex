//! Source-span provenance. Every token carries a byte range into the
//! original source string it came from, so the IDE can map expanded
//! output back to exact source characters (TeXbook ch. 20 does not need
//! this, but our IDE consumer does).

/// A half-open byte range `[start, end)` into a source buffer, plus an id
/// identifying which buffer (file/string) it belongs to. Tokens produced
/// purely by expansion (e.g. characters coming out of `\romannumeral` or a
/// macro body with no corresponding source bytes) use `Span::synthetic`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub source_id: u32,
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub const fn new(source_id: u32, start: u32, end: u32) -> Self {
        Span { source_id, start, end }
    }

    /// A span with no real source backing (synthesized during expansion).
    pub const fn synthetic() -> Self {
        Span { source_id: u32::MAX, start: 0, end: 0 }
    }

    pub fn is_synthetic(&self) -> bool {
        self.source_id == u32::MAX
    }

    /// Merge two spans from the same source into their enclosing range.
    /// If either is synthetic, or they come from different sources, the
    /// first non-synthetic one wins (falls back to synthetic).
    pub fn join(a: Span, b: Span) -> Span {
        if a.is_synthetic() {
            return b;
        }
        if b.is_synthetic() || a.source_id != b.source_id {
            return a;
        }
        Span { source_id: a.source_id, start: a.start.min(b.start), end: a.end.max(b.end) }
    }
}
