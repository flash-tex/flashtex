//! Source provenance carried through layout.
//!
//! An [`Atom`](crate::Atom) may carry a [`SourceTag`]: the byte range of the
//! source that produced it and an optional caller-defined attribute id (a
//! colour, a hyperlink, ...). Layout never reads either for geometry; it only
//! copies them onto the glyph and rule leaves the atom produces, so the
//! flattened [`PositionedGlyph`](crate::PositionedGlyph)s and
//! [`PositionedRule`](crate::PositionedRule)s can be mapped back to source
//! (click-to-source) and painted per attribute.
//!
//! Inheritance is innermost-first and per field: a leaf keeps the span and
//! the attribute of the nearest enclosing atom that sets each one. So in
//! `\frac{a}{b}` the `a` glyph maps to `a`'s bytes while the fraction rule,
//! which no inner atom produced, maps to the `\frac` atom; a colour set on an
//! outer group reaches every leaf whose own atoms leave the attribute unset.

/// A byte range in one source document. `document` is an opaque caller id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SourceSpan {
    pub document: u32,
    pub start: usize,
    pub end: usize,
}

impl SourceSpan {
    pub fn new(document: u32, start: usize, end: usize) -> SourceSpan {
        SourceSpan {
            document,
            start,
            end,
        }
    }
}

/// Provenance of an atom or of a laid-out leaf box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SourceTag {
    /// The source bytes the atom (or the leaf's nearest spanned atom) came from.
    pub span: Option<SourceSpan>,
    /// A caller-defined attribute id (colour, link, ...), inherited like `span`.
    pub attr: Option<u32>,
}

impl SourceTag {
    /// No provenance.
    pub const NONE: SourceTag = SourceTag {
        span: None,
        attr: None,
    };

    pub fn span(span: SourceSpan) -> SourceTag {
        SourceTag {
            span: Some(span),
            attr: None,
        }
    }

    pub fn with_attr(mut self, attr: Option<u32>) -> SourceTag {
        self.attr = attr;
        self
    }

    pub fn is_none(&self) -> bool {
        self.span.is_none() && self.attr.is_none()
    }

    /// Fills whichever fields are unset from `outer`.
    pub fn inherit(&mut self, outer: SourceTag) {
        if self.span.is_none() {
            self.span = outer.span;
        }
        if self.attr.is_none() {
            self.attr = outer.attr;
        }
    }
}
