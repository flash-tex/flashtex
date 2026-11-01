//! NFSS text font selection. The model (LaTeX2e's `\fontfamily`/
//! `\fontseries`/`\fontshape` + `\selectfont` over the Computer Modern and
//! Latin Modern font definition files) lives in the compiler
//! (`flashtex_compiler::nfss`), which applies every font command as it
//! reads it and hands the state in force to each text node
//! (`parser::TextStyle::font`). This module re-exports it so the pipeline's
//! face selection (`typeset::Context::text_role`) and the compiler agree on
//! one table.

pub use flashtex_compiler::nfss::*;
