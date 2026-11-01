//! `flashtex-host-unicode`: Unicode mode's engine host
//! (`flashtex_xetex::host`). GPL-2.0-or-later like the engine; the app never
//! links it, it runs it and talks to its socket.

/// Linux and macOS: the pdfTeX host's allocator (`flashtex_engine::logalloc`):
/// undo logs, and on macOS every large block, in mappings of their own, so
/// that what a run frees leaves the footprint (with the system allocator,
/// a resident Unicode host grew by about 10 MB a keystroke on a 100-page
/// document: freed large blocks stayed resident).
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[global_allocator]
static ALLOC: flashtex_engine::logalloc::HostAlloc = flashtex_engine::logalloc::HostAlloc;

fn main() {
    let argv: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    // (the host only: invoked as the engine -- a format's INITEX run, a
    // compile in a process of its own -- it keeps the system's way, as the
    // pdfTeX engine's do)
    let engine = argv.first().is_some_and(|a| {
        matches!(
            flashtex_engine::system::program_name_from_argv0(a).as_str(),
            "xetex" | "xelatex"
        )
    });
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    if !engine {
        flashtex_engine::logalloc::set_enabled(std::env::var_os("FLASHTEX_NO_LOG_MAPS").is_none());
        // (not under Low Memory, whose allocator returns freed pages itself)
        flashtex_engine::logalloc::set_large_blocks(
            !flashtex_engine::profile::allocator_returns_free_pages(),
        );
    }
    std::process::exit(flashtex_xetex::host::main(argv))
}
