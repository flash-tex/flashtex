//! FlashTeX model of pdfLaTeX text encodings (KC-105).
//!
//! * [`unicode`] — UTF-8 input → LaTeX commands (the `*.dfu` declarations a
//!   default pdflatex format loads).
//! * [`encoding`] — OT1/T1/TS1/OMS/OML `\DeclareText*` declarations, composites,
//!   kernel defaults and "unavailable in encoding" errors.
//! * [`fonts`] — NFSS font selection (cmr/lmr `.fd` files), glyph names from
//!   the pdfTeX map/`.enc` files, and the bundled Latin Modern OTF glyphs.
//! * [`tfm`], [`ligkern`], [`accent`], [`sfcode`] — exact transcriptions of the
//!   relevant tex.web parts (font loading, main-loop ligatures/kerns, `\accent`,
//!   space factor and interword glue).
//! * [`typeset`] — typesets one line into an `\hbox` and reports errors, with
//!   [`layout`] providing the box model and glyph positions.
//!
//! Tables in [`generated`] are extracted from TeX Live 2026 by
//! `tools/extract_tables.py`; expected results come from pdflatex via
//! `tools/generate_oracle.py`. No TeX is run by this crate or its tests.

pub mod accent;
pub mod encoding;
pub mod fonts;
#[rustfmt::skip]
pub mod generated;
pub mod layout;
pub mod ligkern;
pub mod sfcode;
pub mod tfm;
pub mod typeset;
pub mod unicode;
