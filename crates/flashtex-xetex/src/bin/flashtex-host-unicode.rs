//! `flashtex-host-unicode`: Unicode mode's engine host
//! (`flashtex_xetex::host`). GPL-2.0-or-later like the engine; the app never
//! links it, it runs it and talks to its socket.

fn main() {
    let argv: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    std::process::exit(flashtex_xetex::host::main(argv))
}
