//! The runtime FlashTeX's engines share (docs/design/xetex/PLAN.md §3.3):
//! what the pdfTeX engine (`crates/flashtex-engine`) and the XeTeX-derived
//! one (`crates/flashtex-xetex`) both use, moved here unchanged so a fix
//! lands once for both programs. `flashtex-engine` re-exports every module
//! under its old path (`flashtex_engine::os`, ...), so nothing that names
//! one changes.
//!
//! GPL-2.0-or-later (see `LICENSE` in this directory); per
//! `docs/design/engine-v2/DESIGN.md` §3 nothing MIT-licensed may link it.
//!
//! * `persist` — the snapshot codec (`Codec`, `codec_struct!`, `codec_enum!`).
//! * `os` — the operating-system and clock calls.
//! * `busy` — where the engine thread's time and instructions go.
//! * `memstat` — the heap and resident-memory accounting.
//! * `logalloc` — the host's allocator (undo logs in mappings of their own).

pub mod busy;
#[cfg(all(any(target_os = "linux", target_os = "macos"), not(feature = "tex82")))]
pub mod logalloc;
pub mod memstat;
pub mod os;
pub mod persist;
