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
    let mut g = flashtex_engine::Globals::new();
    g.tex_body();
    // The end of the main program: tex.ch's `do_final_end`, whose exit
    // status says whether there was an error.
    system::final_end(&mut g)
}
