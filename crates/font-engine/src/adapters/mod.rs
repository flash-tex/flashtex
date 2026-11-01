//! Adapters that let the three consumers — compiler/paragraph layout, native
//! preview and PDF — read the SAME identity, advances, clusters and glyph
//! mapping from one [`crate::Face`] instead of measuring independently.
//!
//! * [`paragraph`] — `flashtex_paragraph_layout::FontMetricsSource` for any
//!   `Face`, plus `glyph_run` that feeds already-shaped clusters into
//!   `GlyphRun::from_shaped` (feature `paragraph`).
//! * [`pdf`] — converts [`crate::embed::PdfFontProgram`] into
//!   `flashtex_pdf::embed::EmbeddedSubset` so the PDF writer embeds this
//!   crate's subset / CFF, `/W` and `ToUnicode` without re-parsing (feature
//!   `pdf`).
//! * [`math`] — `flashtex_math_layout::metrics::MathFontMetrics` from an
//!   OpenType `MATH` face (feature `math`).
//! * [`preview`] — the JSON metrics export (`face_metrics.json`) the Swift
//!   preview positions glyphs from; always available.
//!
//! Every adapter is a thin, lossless view: no rounding beyond what the target
//! contract's units force, and every limitation is stated on the item.

#[cfg(feature = "math")]
pub mod math;
#[cfg(feature = "paragraph")]
pub mod paragraph;
#[cfg(feature = "pdf")]
pub mod pdf;
pub mod preview;
