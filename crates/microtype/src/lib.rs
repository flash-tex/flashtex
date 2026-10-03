//! FlashTeX model of pdfTeX's microtypographic extensions as driven by the
//! LaTeX `microtype` package: character protrusion (`\pdfprotrudechars`,
//! `\lpcode`, `\rpcode`) and font expansion (`\pdfadjustspacing`,
//! `\pdffontexpand`, `\efcode`).
//!
//! * [`config`] parses microtype `.cfg` files and resolves, per NFSS font, the
//!   exact integer codes and expansion limits microtype hands to pdfTeX.
//! * [`pdftex`] holds the pdfTeX primitives the line breaker and the line
//!   packer need (protrusion amounts, char/kern stretch and shrink, the
//!   shortfall rule used for badness, the per-line expansion ratio and the
//!   per-glyph expansion it implies).
//! * [`arith`] is pdfTeX's integer arithmetic (to the scaled point).
//!
//! See `CONTRACT.md` for the proposed hooks into `crates/paragraph-layout`
//! and `crates/render-pipeline`. The crate has no dependencies and never runs
//! TeX; `tests/oracle/` holds pdflatex-generated expectations.

// Clippy debt in frozen old-engine code (D13): allowed, not refactored.
#![allow(
    clippy::collapsible_if,
    reason = "frozen old engine (D13): fixes only; clippy debt is allowed rather than refactored"
)]

pub mod arith;
pub mod config;
pub mod pdftex;

pub use arith::Scaled;
pub use config::{FontMetrics, MicrotypeConfig, NfssDefaults, NfssFont, Options, ResolvedFont};
pub use pdftex::{
    ExpansionLimits, FontParams, ParagraphExpansion, adjust_shortfall, expanded_width,
    fix_expand_value, line_expand_ratio,
};
