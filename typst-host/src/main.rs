//! `flashtex-typst-host --socket PATH [--font-path DIR]... [--no-system-fonts]`
//!
//! Loads the fonts, prints one JSON line saying what it found, listens on a
//! Unix-domain stream socket at PATH (mode 0600), prints
//! `flashtex-typst-host: listening on PATH`, and serves one connection at a
//! time (spec §6.1). `--version` prints the host and Typst versions.

use std::path::PathBuf;
use std::process::ExitCode;

use flashtex_typst_host::server::{bind, Host};
use flashtex_typst_host::world::FontOptions;
use flashtex_typst_host::TYPST_VERSION;

fn usage() -> ExitCode {
    eprintln!(
        "usage: flashtex-typst-host --socket PATH [--font-path DIR]... [--no-system-fonts] \
         [--font-program-budget BYTES]"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let mut socket: Option<PathBuf> = None;
    let mut fonts = FontOptions {
        paths: vec![],
        system: true,
    };
    let mut budget: Option<u64> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--socket" => match args.next() {
                Some(p) => socket = Some(p.into()),
                None => return usage(),
            },
            "--font-path" => match args.next() {
                Some(p) => fonts.paths.push(p.into()),
                None => return usage(),
            },
            "--no-system-fonts" => fonts.system = false,
            "--font-program-budget" => match args.next().and_then(|b| b.parse().ok()) {
                Some(b) => budget = Some(b),
                None => return usage(),
            },
            "--version" => {
                println!(
                    "flashtex-typst-host {} (Typst {TYPST_VERSION})",
                    env!("CARGO_PKG_VERSION")
                );
                return ExitCode::SUCCESS;
            }
            _ => return usage(),
        }
    }
    let Some(socket) = socket else { return usage() };
    let mut host = Host::new(&fonts);
    if let Some(b) = budget {
        host = host.with_program_budget(b);
    }
    let paths: Vec<String> = fonts
        .paths
        .iter()
        .map(|p| format!("{:?}", p.display().to_string()))
        .collect();
    println!(
        "{{\"typst\":\"{TYPST_VERSION}\",\"fonts\":{},\"font_paths\":[{}],\"system_fonts\":{}}}",
        host.font_count(),
        paths.join(","),
        fonts.system
    );
    let listener = match bind(&socket) {
        Ok(l) => l,
        Err(e) => {
            eprintln!(
                "flashtex-typst-host: cannot listen on {}: {e}",
                socket.display()
            );
            return ExitCode::FAILURE;
        }
    };
    println!("flashtex-typst-host: listening on {}", socket.display());
    use std::io::Write;
    let _ = std::io::stdout().flush();
    match host.serve(listener) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("flashtex-typst-host: {e}");
            ExitCode::FAILURE
        }
    }
}
