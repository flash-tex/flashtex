//! `flashtex-tex-boxes`: TeX's box machinery, faithful to *The TeX Program*
//! (tex.web parts 10, 12, 32, 33 and 47) and pdfTeX's shipout, plus the
//! LaTeX box commands of `latex.ltx` built on top of it.
//!
//! * [`scaled`] — scaled-point arithmetic and unit conversion (§§99–109, 453–458).
//! * [`node`] — hlist/vlist node model (§§133–161).
//! * [`pack`] — `badness`, `hpack`, `vpackage`, over/underfull reports (§§108, 644–678).
//! * [`display`] — `show_box` / `short_display` transcript text (§§173–198).
//! * [`shipout`] — `hlist_out`/`vlist_out` positions with accumulated glue rounding (§§619–637).
//! * [`engine`] — the semantic nest, grouping and box primitives (§§211–283, 1055–1110, 1167–1196).
//! * [`latex`] — `\mbox`, `\makebox`, `\fbox`, `\parbox`, `minipage`, `\raisebox`, … (latex.ltx).
//!
//! See `CONTRACT.md` for the adoption plan.

// Clippy debt in frozen old-engine code (D13): allowed, not refactored.
#![allow(clippy::collapsible_if, clippy::manual_clamp, reason = "frozen old engine (D13): fixes only; clippy debt is allowed rather than refactored")]

pub mod display;
pub mod engine;
pub mod latex;
pub mod node;
pub mod pack;
pub mod scaled;
pub mod shipout;

pub use display::{Printer, ShowLimits, show_box_string};
pub use engine::{BoxContext, BoxDim, BoxEngine, BoxError, BoxKind, BrokenLine, LineBreaker, Mode, PageBuilder};
pub use node::{BoxNode, CharMetrics, GlueOrder, GlueSign, GlueSpec, ListKind, NoChars, Node, Rule};
pub use pack::{PackOrigin, PackOutcome, PackParams, PackReport, PackSpec, ReportKind, badness, hpack, vpack, vpackage};
pub use scaled::{Scaled, parse_dimen, print_scaled};
pub use shipout::{ShipEvent, ship_out};
