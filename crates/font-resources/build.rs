use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries = std::fs::read_dir(dir)
        .expect("font engine sources")
        .map(|e| e.unwrap().path())
        .collect::<Vec<_>>();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            files(&p, out)
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p)
        }
    }
}
fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../font-engine");
    let mut sources = vec![root.join("Cargo.toml")];
    files(&root.join("src"), &mut sources);
    let mut hash = Sha256::new();
    for p in sources {
        println!("cargo:rerun-if-changed={}", p.display());
        let name = p.strip_prefix(&root).unwrap().to_str().unwrap();
        let bytes = std::fs::read(&p).unwrap();
        hash.update((name.len() as u64).to_be_bytes());
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    println!("cargo:rerun-if-changed={}", root.join("src").display());
    println!(
        "cargo:rustc-env=FLASHTEX_FONT_ENGINE_SOURCE_SHA256={:x}",
        hash.finalize()
    );
}
