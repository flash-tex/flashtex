//! FlashTeX paragraph layout: an original Rust implementation of TeX-style
//! paragraph line breaking, horizontal/vertical metrics and deterministic page
//! breaking, behind an integration API that consumes font metrics and produces
//! positioned glyph runs.
//!
//! See `README.md` for the API tour, parameter defaults and coordinate
//! conventions, and `docs/comparison.md` for the pdflatex oracle comparison.

pub mod adapter;
pub mod core14;
pub mod document;
pub mod hyphenate;
pub mod items;
pub mod liang;
pub mod linebreak;
pub mod metrics;
pub mod microtype;
pub mod pages;
pub mod runtime_v1;
pub mod style;

pub use adapter::{LayoutError, MAX_DIMEN_PT, MAX_DIMEN_SP, MAX_ITEMS, try_layout_paragraph};
pub use document::{
    DocumentLayout, DocumentSpec, ParagraphLayout, RelayoutStats, layout_document, relayout,
};
pub use hyphenate::{
    ExplicitDiscretionary, HyphenationPoint, Hyphenator, HyphenatorError, NoHyphenation,
};
pub use items::{
    FORCED_BREAK, Glue, GlueOrder, Glyph, GlyphRun, INFINITE_PENALTY, Item, Kern, ParagraphBuilder,
    Penalty, ShapedGlyph, shape_run,
};
pub use liang::LiangHyphenator;
pub use linebreak::{
    Algorithm, BreakMode, BreakPoint, Diagnostic, DiagnosticKind, Fitness, Line, LineBreakParams,
    Lines, Overfull, PositionedGlyph, PositionedRun, Severity, Stats, layout_paragraph,
    layout_paragraph_microtype,
};
pub use microtype::{MathNode, MicroGlyph, MicroItem, MicroLine, MicroMath, MicroRun, Microtype};
pub use metrics::{FontId, FontMetricsSource, Ligature};
pub use pages::{Page, PageOverflow, PageParams, Pages, ParagraphBlock, PlacedLine, layout_pages};

/// Renders a caught panic payload as a message, for typed errors that carry
/// panic diagnostics ([`LayoutError::Internal`], [`hyphenate::HyphenatorError::Panicked`]).
/// Shared so every panic-to-error boundary in this crate reports the same way.
pub(crate) fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

/// 1 TeX point in PostScript/PDF points (big points): 72/72.27.
pub const BP_PER_TEX_PT: f64 = 72.0 / 72.27;

/// Converts a length in TeX points to PDF big points.
pub fn tex_pt_to_bp(pt: f64) -> f64 {
    pt * BP_PER_TEX_PT
}
