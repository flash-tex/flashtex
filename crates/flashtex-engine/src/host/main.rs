//! `flashtex-host`: the engine host (DESIGN.md §3, §5). One program:
//!
//! * `flashtex-host --socket PATH ...` serves `display-list-v3` over a Unix
//!   socket (docs/protocol/display-list-v3.md §6) from a resident,
//!   incremental engine: [`flashtex_engine::host::server`].
//! * `flashtex-host serve|iserve|bench|open|selftest|layout ...` are the
//!   resident engine's line protocols and measurements:
//!   [`flashtex_engine::host::tools`].
//! * Invoked as `pdftex` (argv[0]), it is the engine itself, as
//!   `flashtex-initex` is: the format cache builds formats by running the
//!   current program as `pdftex -ini ...`, and an export (`"export": true`)
//!   runs it as a child with pdflatex's command line.
//!
//! GPL-2.0-or-later like the engine. The app never links this program; it
//! runs it and talks to its socket.

fn main() {
    let argv: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let invoked_as = argv
        .first()
        .and_then(|a| std::path::Path::new(a).file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if invoked_as == "pdftex" {
        engine(&argv);
    }
    let code = match argv.get(1).map(String::as_str) {
        Some("serve" | "iserve" | "bench" | "open" | "selftest" | "layout") => {
            flashtex_engine::host::tools::main(argv)
        }
        _ => flashtex_engine::host::server::main(argv),
    };
    std::process::exit(code)
}

/// The engine's driver, as `flashtex-initex`'s (src/main.rs).
fn engine(argv: &[String]) -> ! {
    use flashtex_engine::system;
    let o = flashtex_engine::cli::parse(argv);
    system::configure(o);
    flashtex_engine::host::pin_clock_from_env();
    if std::env::var_os("FLASHTEX_PREVIEW").is_some_and(|v| v == "1") {
        flashtex_engine::pdftex::set_preview(true);
    }
    flashtex_engine::displaylist::init_from_env();
    let mut g = flashtex_engine::Globals::new();
    g.tex_body();
    flashtex_engine::displaylist::finish();
    system::final_end(&mut g)
}
