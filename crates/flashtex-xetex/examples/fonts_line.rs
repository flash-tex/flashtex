//! The `[fonts]` code Unicode mode's host puts ahead of `\input{main.tex}`
//! on the engine's first line for a project (`host::fonts::setup`,
//! docs/design/modes/PROPOSAL.md §4.5): the document's *effective source*,
//! which `tools/unicode-m3/corpus.py` hands to both `xelatex` and the port so
//! that the `[fonts]` fixtures are compared against the oracle on the same
//! line (P-T1 and XDV, the M3 gate).
//!
//! ```sh
//! cargo run --release --example fonts_line -- PROJECT_DIR
//! ```
//!
//! Prints the code (nothing when `[fonts]` sets no role) on stdout, and each
//! note (`info` per role applied, `warning` per role refused) on stderr.

fn main() {
    let Some(root) = std::env::args().nth(1) else {
        eprintln!("usage: fonts_line PROJECT_DIR");
        std::process::exit(2);
    };
    let s = flashtex_xetex::host::fonts::setup(std::path::Path::new(&root), "xelatex");
    for (severity, message) in &s.notes {
        eprintln!("{severity}: {message}");
    }
    print!("{}", s.code);
}
