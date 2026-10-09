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

/// Linux and macOS: undo logs in mappings of their own, everything else
/// jemalloc's (Linux) or libmalloc's (macOS) (`flashtex_engine::logalloc`;
/// lane P4-MEMORY-BUDGET). A `mem-stats` build's counting allocator does
/// the same underneath.
#[cfg(all(
    any(target_os = "linux", target_os = "macos"),
    not(feature = "mem-stats")
))]
#[global_allocator]
static ALLOC: flashtex_engine::logalloc::HostAlloc = flashtex_engine::logalloc::HostAlloc;

fn main() {
    let mut argv: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    // `argv[0]`'s file name, or on Windows (which cannot set `argv[0]`)
    // what `flashtex_engine::os::engine_command` put in the environment.
    let invoked_as = flashtex_engine::os::invoked_as(argv.first().map_or("", String::as_str));
    if invoked_as == "pdftex" {
        if let Some(a0) = argv.first_mut() {
            if cfg!(windows) {
                // As on Unix, where the engine sees `argv[0]` = `pdftex`.
                *a0 = invoked_as;
            }
        }
        engine(&argv);
    }
    // Low Memory: the allocator that returns freed pages (macOS; re-runs
    // this program once, before any thread starts).
    #[cfg(target_os = "macos")]
    flashtex_engine::profile::space_efficient_startup();
    flashtex_engine::host::crash::install();
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    flashtex_engine::logalloc::set_enabled(std::env::var_os("FLASHTEX_NO_LOG_MAPS").is_none());
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
    if std::env::var_os("FLASHTEX_DIAGNOSTICS").is_some_and(|v| v == "1") {
        flashtex_engine::diag::set_enabled(true);
    }
    let mut g = flashtex_engine::Globals::new();
    g.tex_body();
    flashtex_engine::displaylist::finish();
    system::final_end(&mut g)
}
