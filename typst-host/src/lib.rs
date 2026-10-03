//! `flashtex-typst-host`: FlashTeX's Typst engine host (DESIGN.md §15).
//!
//! A separate process, one per open Typst document, that speaks the same
//! socket protocol as the LaTeX engine host (`docs/protocol/display-list-v3.md`
//! §6) and streams `display-list-v3` pages, with the 3.3 additions of
//! DESIGN.md §15.4 for a client that asks for them ([`v33`]).
//!
//! Licence boundary (DESIGN.md §3, §15.2): this crate is MIT and links only
//! the unmodified Apache-2.0 `typst` crates and the MIT
//! `flashtex-display-list`. It never links, calls or shares files with
//! `crates/flashtex-engine`.
//!
//! Phase: T0 (DESIGN.md §15.10 in PR #1264). See `README.md` for what is in
//! and out of scope.

pub mod convert;
pub mod server;
pub mod v33;
pub mod world;

/// The pinned Typst version (Cargo.toml `=0.15.1`; a test checks the lock).
pub const TYPST_VERSION: &str = "0.15.1";

#[cfg(test)]
mod tests {
    #[test]
    fn typst_version_matches_the_lockfile() {
        let lock =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.lock")).unwrap();
        for name in [
            "typst",
            "typst-pdf",
            "typst-layout",
            "typst-library",
            "typst-kit",
        ] {
            let entry = format!("name = \"{name}\"\nversion = \"{}\"", super::TYPST_VERSION);
            assert!(
                lock.contains(&entry),
                "Cargo.lock does not pin {name} {}",
                super::TYPST_VERSION
            );
        }
    }
}
