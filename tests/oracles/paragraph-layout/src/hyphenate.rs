//! Hyphenation input. The breaker itself only sees discretionary penalties; a
//! [`Hyphenator`] decides where a word may be split.

use std::fmt;

/// One allowed split inside a word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HyphenationPoint {
    /// Byte offset inside the word text at which the break happens.
    pub offset: usize,
    /// Bytes at `offset` that are a discretionary *marker* (e.g. the two bytes
    /// of `\-`) rather than glyphs. They are skipped when shaping and become the
    /// source range of the hyphen glyph if the line breaks there.
    pub marker_len: usize,
    /// True when the point came from an automatic (dictionary/pattern)
    /// hyphenator rather than an explicit discretionary. TeX ignores automatic
    /// points in its first (pretolerance) pass; explicit `\-` is always legal.
    pub automatic: bool,
}

/// Reports the legal hyphenation points of one word (text without spaces).
///
/// Implemented by callers, so — from this crate's point of view — an
/// implementation is untrusted input: [`crate::items::ParagraphBuilder::word`]
/// guards every call against a panicking or misbehaving implementation and
/// reports [`HyphenatorError`] instead of letting either crash past this
/// crate's public API.
pub trait Hyphenator {
    fn hyphenate(&self, word: &str) -> Vec<HyphenationPoint>;
}

/// Something a caller-supplied [`Hyphenator`] did wrong, caught at the
/// boundary where this crate calls into it
/// ([`crate::items::ParagraphBuilder::word`]) instead of being allowed to
/// panic past this crate's public API.
#[derive(Debug, Clone, PartialEq)]
pub enum HyphenatorError {
    /// The `Hyphenator::hyphenate` call itself panicked. Carries the panic
    /// payload's message, if it was a `&str` or `String`.
    Panicked { word: String, message: String },
    /// `hyphenate` returned normally, but with a [`HyphenationPoint`] whose
    /// `offset` (or `offset + marker_len`) is out of range for `word`, not on
    /// a UTF-8 char boundary, or out of order with an earlier point in the
    /// same call — any of which would panic if used to slice `word` as-is.
    InvalidPoint {
        word: String,
        offset: usize,
        reason: &'static str,
    },
}

impl fmt::Display for HyphenatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HyphenatorError::Panicked { word, message } => {
                write!(f, "Hyphenator::hyphenate panicked on {word:?}: {message}")
            }
            HyphenatorError::InvalidPoint {
                word,
                offset,
                reason,
            } => {
                write!(
                    f,
                    "Hyphenator::hyphenate returned an invalid point for {word:?} at byte {offset}: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for HyphenatorError {}

/// Never hyphenates. Equivalent to `\hyphenpenalty=10000` with no `\-`.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoHyphenation;

impl Hyphenator for NoHyphenation {
    fn hyphenate(&self, _word: &str) -> Vec<HyphenationPoint> {
        Vec::new()
    }
}

/// Honours only explicit `\-` discretionaries written in the word text.
/// The marker bytes are not typeset; they are the source range of the hyphen
/// that appears if the line breaks there.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExplicitDiscretionary;

impl Hyphenator for ExplicitDiscretionary {
    fn hyphenate(&self, word: &str) -> Vec<HyphenationPoint> {
        let mut out = Vec::new();
        let mut from = 0;
        while let Some(i) = word[from..].find("\\-") {
            let offset = from + i;
            out.push(HyphenationPoint {
                offset,
                marker_len: 2,
                automatic: false,
            });
            from = offset + 2;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_marker_positions() {
        let pts = ExplicitDiscretionary.hyphenate("re\\-pro\\-ducible");
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0].offset, 2);
        assert_eq!(pts[1].offset, 7);
        assert!(pts.iter().all(|p| p.marker_len == 2 && !p.automatic));
        assert!(NoHyphenation.hyphenate("re\\-pro").is_empty());
    }
}
