//! `flashtex-typst-host --socket PATH [--font-path DIR]... [--no-system-fonts]`
//!
//! Loads the fonts, prints one JSON line saying what it found, listens on a
//! Unix-domain stream socket at PATH (mode 0600), prints
//! `flashtex-typst-host: listening on PATH`, and serves one connection at a
//! time (spec §6.1). `--version` prints the host and Typst versions.

use std::path::PathBuf;
use std::process::ExitCode;

use flashtex_typst_host::server::{bind, Host, Verify};
use flashtex_typst_host::world::FontOptions;
use flashtex_typst_host::TYPST_VERSION;

fn usage() -> ExitCode {
    eprintln!(
        "usage: flashtex-typst-host --socket PATH [--font-path DIR]... [--no-system-fonts] \
         [--font-program-budget BYTES] [--seeded on|off] [--verify off|every|idle[:MS]] \
         [--watchdog-secs S] [--watchdog-cold-secs S] [--rss-ceiling-mb MB]"
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
    let mut seeded = true;
    let mut limits = flashtex_typst_host::watchdog::Limits::default();
    let mut verify = Verify::Idle(std::time::Duration::from_millis(1000));
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
            "--seeded" => match args.next().as_deref() {
                Some("on") => seeded = true,
                Some("off") => seeded = false,
                _ => return usage(),
            },
            "--verify" => {
                verify = match args.next().as_deref() {
                    Some("off") => Verify::Off,
                    Some("every") => Verify::Every,
                    Some("idle") => Verify::Idle(std::time::Duration::from_millis(1000)),
                    Some(v) => match v.strip_prefix("idle:").and_then(|m| m.parse().ok()) {
                        Some(ms) => Verify::Idle(std::time::Duration::from_millis(ms)),
                        None => return usage(),
                    },
                    None => return usage(),
                }
            }
            "--watchdog-secs" | "--watchdog-cold-secs" => {
                let Some(s) = args.next().and_then(|v| v.parse::<f64>().ok()) else {
                    return usage();
                };
                let d = std::time::Duration::from_secs_f64(s);
                if a == "--watchdog-secs" {
                    limits.wall = d;
                } else {
                    limits.wall_cold = d;
                }
            }
            "--rss-ceiling-mb" => match args.next().and_then(|v| v.parse::<u64>().ok()) {
                Some(0) => limits.rss_bytes = None,
                Some(mb) => limits.rss_bytes = Some(mb << 20),
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
    let mut host = Host::new(&fonts)
        .with_seeded(seeded)
        .with_verify(verify)
        .with_watchdog(limits);
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
    // Hosts the watchdog (or anyone) killed left their temporary directories.
    flashtex_typst_host::watchdog::sweep_stale_temp_dirs();
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
