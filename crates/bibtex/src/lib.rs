//! BibTeX 0.99e as TeX Live 2026 builds it, translated to Rust so the engine
//! host can run it in-process (lane RUST-TOOLS, DESIGN.md §4.5).
//!
//! **Source of record.** `third_party/bibtex/bibtex.web` (Oren Patashnik's
//! BibTeX, version 0.99e) and TeX Live's change file
//! `third_party/bibtex/bibtex.ch`, both unmodified, from the TeX Live source
//! tree at tag `texlive-2026.1` (commit
//! `6a300188053b8f2ded89dbd52293732a706b9c0e`, `texk/web2c`).
//!
//! **How.** `src/generated/` is `tools/web2rust`'s translation of
//! `bibtex.web` with `bibtex.ch` and then `changes/flashtex.ch`
//! (`web2rust.args`); it is committed and never edited by hand (the drift
//! test regenerates it). `changes/flashtex.ch` restates bibtex.ch's C
//! constructs in the Pascal web2rust reads: what web2c's `convert`,
//! `cvtbib.sed`, `fixwrites` and TeX Live's `lib/` do with them becomes
//! `external` routines, written in [`system`] as the C does them (kpathsea's
//! file lookup through [`Host`], `eof`/`eoln`/`getc`, BIB_XRETALLOC's log
//! lines, getopt, the nonlocal gotos).
//!
//! **Licence.** BibTeX is "available under the same terms as Donald Knuth's
//! TeX program" (bibtex.web's header): copying is unrestricted, and a changed
//! version may not be called `bibtex.web`; changes go in change files, as
//! here. See `LICENSE` in this directory.

pub mod arena;
pub mod generated;
pub mod system;

pub use system::{run, run_with_deadline, Format, Host, Outcome, CRASHED, TIMED_OUT};
