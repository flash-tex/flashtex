//! `flashtex-host`: the engine host (DESIGN.md §3, §5). One program:
//!
//! * `flashtex-host --socket PATH ...` serves `display-list-v3` over a Unix
//!   socket (docs/protocol/display-list-v3.md §6): [`flashtex_engine::host::server`].
//! * `flashtex-host serve|iserve|bench|open|selftest|layout ...` are the
//!   resident engine's line protocols and measurements:
//!   [`flashtex_engine::host::tools`].
//!
//! GPL-2.0-or-later like the engine. The app never links this program; it
//! runs it and talks to its socket.

fn main() {
    let argv: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let code = match argv.get(1).map(String::as_str) {
        Some("serve" | "iserve" | "bench" | "open" | "selftest" | "layout") => {
            flashtex_engine::host::tools::main(argv)
        }
        _ => flashtex_engine::host::server::main(argv),
    };
    std::process::exit(code)
}
