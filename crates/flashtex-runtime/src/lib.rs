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
//! * `statsig` — a file's identity (`StatSig`) and the racy window.
//! * `zlib` — TeX Live's zlib (feature `zlib`), for the PDF writers and the
//!   bundle.
//! * `resolver` — the file resolver: kpathsea (feature `kpathsea`), the
//!   working directory, the bundle.
//! * `lookupproof` — what a lookup's answer depends on, checked without
//!   making the lookup again.
//! * `formats` — the format cache (feature `distribution`).
//! * `bundle` — the no-TeX-Live bundle (feature `distribution`).
//! * `trace` — the file trace (`FLASHTEX_FILE_TRACE`).

#[cfg(all(feature = "distribution", not(feature = "tex82")))]
pub mod bundle;
pub mod busy;
#[cfg(feature = "distribution")]
pub mod formats;
#[cfg(all(any(target_os = "linux", target_os = "macos"), not(feature = "tex82")))]
pub mod logalloc;
pub mod lookupproof;
pub mod memstat;
pub mod os;
pub mod persist;
pub mod resolver;
pub mod statsig;
pub mod trace;
pub mod zlib;
