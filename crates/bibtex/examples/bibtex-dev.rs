//! A development driver: the port on the process's streams, with kpathsea's
//! answers from `kpsewhich -progname=bibtex` (the engine's `flashtex-bibtex`
//! binary uses the engine's own kpathsea instead).

use std::process::Command;

struct Kpsewhich;

fn kpsewhich(args: &[&str]) -> Option<Vec<u8>> {
    let out = Command::new("kpsewhich").args(args).output().ok()?;
    let mut p = out.stdout;
    while p.last() == Some(&b'\n') {
        p.pop();
    }
    (!p.is_empty()).then_some(p)
}

impl flashtex_bibtex::Host for Kpsewhich {
    fn find_file(&mut self, name: &[u8], format: flashtex_bibtex::Format) -> Option<Vec<u8>> {
        let f = match format {
            flashtex_bibtex::Format::Bib => "-format=bib",
            flashtex_bibtex::Format::Bst => "-format=bst",
        };
        kpsewhich(&[
            "-progname=bibtex",
            "-must-exist",
            f,
            String::from_utf8_lossy(name).as_ref(),
        ])
    }
    fn var_value(&mut self, name: &str) -> Option<String> {
        let v = kpsewhich(&["-progname=bibtex", &format!("-var-value={name}")])?;
        Some(String::from_utf8_lossy(&v).into_owned())
    }
}

fn main() {
    let args: Vec<Vec<u8>> = std::env::args_os()
        .skip(1)
        .map(|a| a.into_encoded_bytes())
        .collect();
    let o = flashtex_bibtex::run(&args, Box::new(Kpsewhich));
    use std::io::Write;
    let _ = std::io::stdout().write_all(&o.stdout);
    let _ = std::io::stderr().write_all(&o.stderr);
    std::process::exit(if o.status < 0 {
        128 - o.status
    } else {
        o.status
    });
}
