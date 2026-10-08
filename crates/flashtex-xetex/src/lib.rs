//! FlashTeX's XeTeX port, phase S0 — a Rust translation of TeX Live 2026's
//! `xetex.web` (XeTeX 0.999998, which includes e-TeX), TFM fonts only.
//!
//! GPL-2.0-or-later (see `LICENSE` in this directory). Per
//! `docs/design/engine-v2/DESIGN.md` §3 nothing MIT-licensed may link this
//! crate. The plan and the phases are `docs/design/xetex/PLAN.md`.
//!
//! * `generated/` is produced by `tools/web2rust` from
//!   `third_party/xetex/xetex.web` and the change files in `changes/`
//!   (`web2rust-default.args`), and is committed as-is. Never edit it.
//! * `system.rs` is the system-dependent layer: Pascal's file model, the
//!   routines web2c's `tex.ch` and `xetex.ch` supply, and XeTeX's Unicode
//!   input files (`XeTeX_ext.c`'s `u_open_in` and `input_line`).
//! * `xetex_ext.rs` holds XeTeX's other C parts behind the interface
//!   `changes/ext.ch` declares. In S0 the native-font, graphics and TECkit
//!   routines are stubs: no installed font is ever found.
//! * `state.rs` is the engine's state outside the word space (`Globals::host`,
//!   web2rust's `--host-state`): the handle tables of `changes/ext.ch`, saved
//!   with the word space at a checkpoint, and what TeX Live keeps in C
//!   globals. This crate has no process-wide mutable state; the runtime it
//!   shares with the pdfTeX engine still has some, which moves to per-engine
//!   state in phase S3 (docs/design/xetex/PLAN.md §3.3).
//! * `fontmgr/` is XeTeX's font lookup (`XeTeXFontMgr`, `splitFontName`,
//!   `findnativefont` up to loading), platform-free over
//!   `crates/font-discovery`'s index. Not yet called by `xetex_ext.rs`.
//! * The word space (`arena`), the checked array index (`ix`), the command
//!   line, the run's configuration and the file resolver (kpathsea) are the
//!   pdfTeX engine's (`crates/flashtex-engine`), used through its public
//!   interface.

pub use flashtex_engine::arena;
pub use flashtex_engine::ix;
/// TeX Live's HarfBuzz and FreeType (phase S1's native fonts,
/// docs/design/xetex/PLAN.md §3.1): `fontlibs::hb`, `fontlibs::ft`.
pub use flashtex_xetex_fontlibs as fontlibs;

pub mod fontmgr;
pub mod generated;
pub mod state;
pub mod system;
pub mod xetex_ext;

pub use generated::Globals;
