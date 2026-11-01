//! `flashtex-bibtex [bibtex's arguments]`: the BibTeX port (crates/bibtex)
//! as a command, exactly as `\write18` runs it in-process (src/bibtex.rs):
//! the process's working directory, `.bib` and `.bst` files and texmf.cnf
//! settings through the engine's kpathsea. It is what the oracle comparison
//! in docs/evidence/rusttools-2026-10-09/bibtex/ runs against TeX Live's
//! `bibtex`.
//!
//! Exit status: bibtex's (0, 1 for a usage or file error, 2 after errors,
//! 3 after a fatal error). Where the C program's behaviour is undefined and
//! TeX Live's binary dies of SIGSEGV (`flashtex_bibtex::CRASHED`), this
//! command leaves the same partial output and dies of SIGSEGV too.

fn main() {
    // A crash is reported the C program's way (a signal), not by a message.
    std::panic::set_hook(Box::new(|_| {}));
    let args: Vec<Vec<u8>> = std::env::args_os()
        .skip(1)
        .map(|a| a.into_encoded_bytes())
        .collect();
    let host = flashtex_engine::bibtex::EngineHost {
        stream_stdout: true,
        ..Default::default()
    };
    let o = flashtex_bibtex::run(&args, Box::new(host));
    use std::io::Write;
    let _ = std::io::stderr().write_all(&o.stderr);
    if o.status == flashtex_bibtex::CRASHED {
        #[cfg(unix)]
        {
            extern "C" {
                fn signal(sig: i32, handler: usize) -> usize;
                fn raise(sig: i32) -> i32;
            }
            // SAFETY: SIGSEGV (11 on macOS and Linux) back to its default
            // action (SIG_DFL, 0; Rust's runtime installs a handler), then
            // raised: the process ends as the C program's does.
            unsafe {
                signal(11, 0);
                raise(11);
            }
        }
    }
    std::process::exit(if o.status < 0 {
        128 - o.status
    } else {
        o.status
    });
}
