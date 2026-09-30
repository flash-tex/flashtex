//! The engine's driver: pdfTeX's command line (parsed by
//! `flashtex_engine::cli`, which documents the options), then one run of
//! the program, then the process exit status.

use flashtex_engine::system;

fn main() {
    // Arguments that are not UTF-8 are read with replacement characters.
    let argv: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let o = flashtex_engine::cli::parse(&argv);
    system::configure(o);
    #[cfg(not(feature = "tex82"))]
    flashtex_engine::host::pin_clock_from_env();
    // FLASHTEX_PREVIEW=1: the preview mode of `flashtex_engine::incr` (PDF
    // streams stored, not compressed), for comparing with the resident host.
    #[cfg(not(feature = "tex82"))]
    if std::env::var_os("FLASHTEX_PREVIEW").is_some_and(|v| v == "1") {
        flashtex_engine::pdftex::set_preview(true);
    }
    // The preview's display list, when the engine host asked for one.
    #[cfg(not(feature = "tex82"))]
    flashtex_engine::displaylist::init_from_env();
    // FLASHTEX_DIAGNOSTICS=1: the diagnostics side channel's hooks
    // (`flashtex_engine::diag`) run as in the host, to show that they
    // change nothing a run writes (the parity gates with it on).
    #[cfg(not(feature = "tex82"))]
    if std::env::var_os("FLASHTEX_DIAGNOSTICS").is_some_and(|v| v == "1") {
        flashtex_engine::diag::set_enabled(true);
    }
    let mut g = flashtex_engine::Globals::new();
    // FLASHTEX_MACRO_PROFILE=FILE: the macro-level profiler (src/macroprof.rs).
    #[cfg(not(feature = "tex82"))]
    flashtex_engine::macroprof::start_from_env(&mut g);
    g.tex_body();
    #[cfg(not(feature = "tex82"))]
    flashtex_engine::displaylist::finish();
    // The end of the main program: tex.ch's `do_final_end`, whose exit
    // status says whether there was an error.
    system::final_end(&mut g)
}
