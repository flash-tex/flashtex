//! FlashTeX compiler foundation.
//!
//! This is an original implementation. No existing TeX engine is invoked, linked,
//! or shelled out to. It implements a deliberately finite, documented subset of
//! LaTeX (see `README.md`); anything outside that subset produces an explicit
//! diagnostic rather than silently rendering or silently vanishing.
//!
//! Byte offsets are the contract's currency: every span is a zero-based,
//! end-exclusive UTF-8 byte range into the exact input text of the stated
//! revision, per `docs/contracts/runtime-v1.md`.

pub mod amssymb;
pub mod bib;
pub mod color;
mod color_names;
pub mod date;
pub mod diagnostics;
pub mod export;
mod class_lengths;
pub mod expansion;
pub mod graphics;
pub mod incremental;
pub mod json;
pub mod layout;
pub mod lexer;
pub mod lm_math;
pub mod math;
pub mod natbib;
pub mod newcm_math;
pub mod parser;
pub mod protocol;
pub mod supported;
pub mod tabular;
pub mod siunitx;
pub mod text_builtins;
pub mod theorems;
pub mod vocabulary;
pub mod xref;

/// Stable identity of one document in a compile request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocumentId(pub usize);

/// A zero-based, end-exclusive UTF-8 byte range into a source document.
///
/// Invariant: `start <= end`, both land on UTF-8 character boundaries of the
/// document they refer to, so `&text[start..end]` never panics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub document: DocumentId,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self::in_document(DocumentId::default(), start, end)
    }

    pub fn in_document(document: DocumentId, start: usize, end: usize) -> Self {
        debug_assert!(start <= end, "span start must not exceed end");
        Span {
            document,
            start,
            end,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Smallest span covering both inputs.
    pub fn merge(self, other: Span) -> Span {
        debug_assert_eq!(
            self.document, other.document,
            "cannot merge spans from different documents"
        );
        Span::in_document(
            self.document,
            self.start.min(other.start),
            self.end.max(other.end),
        )
    }
}
