//! FlashTeX engine core — a Rust translation of Knuth's `tex.web`.
//!
//! GPL-2.0-or-later (see `LICENSE` in this directory). Per
//! `docs/design/engine-v2/DESIGN.md` §3 nothing MIT-licensed may link this
//! crate; the app talks to it over the `display-list-v3` socket protocol.
//!
//! * `generated/` is produced by `tools/web2rust` from
//!   `third_party/knuth/tex.web` and is committed as-is. Never edit it; change
//!   the translator and regenerate (see `tools/web2rust/README.md`).
//! * `system.rs` is the hand-written system-dependent layer, i.e. the part
//!   web2c gets from `tex.ch`.

pub mod generated;
pub mod system;

pub use generated::Globals;
