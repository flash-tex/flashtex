//! NFSS text font selection. The tables and rules live in
//! `flashtex_font_resources::nfss` (moved there in PLAN3 S1, so the TFM file
//! tables that complete them sit in one crate both the compiler and the
//! render pipeline depend on); this path is kept for existing callers.

pub use flashtex_font_resources::nfss::*;
