//! FlashTeX engine core — a Rust translation of TeX Live's `pdftex.web`
//! (pdfTeX 1.40.29, which includes e-TeX).
//!
//! GPL-2.0-or-later (see `LICENSE` in this directory). Per
//! `docs/design/engine-v2/DESIGN.md` §3 nothing MIT-licensed may link this
//! crate; the app talks to it over the `display-list-v3` socket protocol.
//!
//! * `generated/` is produced by `tools/web2rust` from
//!   `third_party/pdftex/pdftex.web` and the change files in `changes/`, and
//!   is committed as-is. Never edit it; change the translator or a change file
//!   and regenerate (see `tools/web2rust/README.md`).
//! * `system.rs` is the hand-written system-dependent layer: Pascal's file
//!   model and the routines web2c's `tex.ch` supplies in C.
//! * `pdftex/` holds pdfTeX's C parts (`utils.c`, `vfpacket.c`, the font and
//!   image writers, ...) behind the interface `changes/ext.ch` declares.
//! * `resolver.rs` finds input files: TeX Live's kpathsea (vendored in
//!   third_party/kpathsea, feature `kpathsea`) behind the `FileResolver`
//!   trait; see docs/evidence/file-resolver-2026-09-29/.
//! * `formats.rs` (feature `distribution`) builds and caches formats from
//!   the TeX Live in use, as fmtutil would (DESIGN.md 4.4); `bundle/` is the
//!   content-addressed bundle for machines without TeX Live; see
//!   docs/evidence/distribution-2026-09-29/.
//! * The capacities the generated code was built with are in
//!   `web2rust-default.args` (TeX Live 2026's texmf.cnf values for pdflatex).
//!
//! Feature `tex82` is for the trip test's scratch package only
//! (`scripts/flashtex-trip.sh`), which builds the same system layer against a
//! translation of Knuth's `tex.web` and so leaves out `pdftex/`.

pub mod arena;
#[cfg(not(feature = "tex82"))]
pub mod checkpoint;
pub mod cli;
#[cfg(all(feature = "distribution", not(feature = "tex82")))]
pub mod bundle;
#[cfg(feature = "distribution")]
pub mod formats;
pub mod generated;
#[cfg(not(feature = "tex82"))]
pub mod host;
#[cfg(not(feature = "tex82"))]
pub mod pdftex;
pub mod persist;
pub mod resolver;
pub mod system;

pub use generated::Globals;
