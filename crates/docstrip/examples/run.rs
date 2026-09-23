//! `cargo run --example run -- <batch.ins> <out-dir>`: runs a batch file
//! against the sources next to it and writes every generated file into
//! `out-dir`, printing docstrip's messages and the diagnostics. Handy for
//! diffing against a TeX Live installation.

use std::path::PathBuf;

use flashtex_docstrip::{run, DirSources};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [ins, out] = args.as_slice() else {
        eprintln!("usage: run <batch.ins> <out-dir>");
        std::process::exit(2);
    };
    let ins = PathBuf::from(ins);
    let dir = ins.parent().map(PathBuf::from).unwrap_or_default();
    let name = ins.file_name().unwrap().to_string_lossy().into_owned();
    let bytes = std::fs::read(&ins).unwrap_or_else(|e| {
        eprintln!("{}: {e}", ins.display());
        std::process::exit(1)
    });
    let outcome = run(&name, &bytes, &DirSources(dir));
    for m in &outcome.messages {
        println!("{m}");
    }
    for d in &outcome.diagnostics {
        eprintln!("note: {d}");
    }
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    for f in &outcome.files {
        let path = out.join(f.name.rsplit('/').next().unwrap_or(&f.name));
        std::fs::write(&path, &f.bytes).unwrap();
        println!("wrote {} ({} bytes, from {})", path.display(), f.bytes.len(), f.sources.join(" "));
    }
    if !outcome.completed {
        std::process::exit(1);
    }
}
