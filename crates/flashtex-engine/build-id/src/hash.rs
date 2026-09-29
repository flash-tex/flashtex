//! What the engine build id covers, and how it is computed. Shared by the
//! build script (which computes the id once per change) and the library
//! (whose tests recompute it).

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Directories of the engine crate whose every file is hashed: all its Rust
/// sources (the translated `pdftex.web` in src/generated/ and the
/// hand-written system layer, resolver, format cache and pdfTeX C-part
/// ports alike), the change files, and the C shims.
pub const DIRS: &[&str] = &["src", "changes", "csrc", "kpathsea-config"];

/// Single files of the engine crate that are hashed: the string pool, the
/// capacities, the manifest (version, features) and the build script.
pub const FILES: &[&str] = &[
    "pdftex.pool",
    "web2rust-default.args",
    "web2rust-trip.args",
    "web2rust-etrip.args",
    "Cargo.toml",
    "build.rs",
];

fn walk(d: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(d) else { return };
    for e in rd.flatten() {
        if e.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.is_file() {
            out.push(p);
        }
    }
}

/// The files hashed, relative to `engine_dir`, sorted.
pub fn inputs(engine_dir: &Path) -> Vec<PathBuf> {
    let mut v = vec![];
    for d in DIRS {
        walk(&engine_dir.join(d), &mut v);
    }
    for f in FILES {
        let p = engine_dir.join(f);
        if p.is_file() {
            v.push(p);
        }
    }
    let mut v: Vec<PathBuf> = v
        .into_iter()
        .map(|p| {
            p.strip_prefix(engine_dir)
                .map(Path::to_path_buf)
                .unwrap_or(p)
        })
        .collect();
    v.sort();
    v
}

/// The id: SHA-256 over each input's relative path, length and content.
pub fn engine_sources_id(engine_dir: &Path) -> String {
    let mut h = Sha256::new();
    h.update(b"flashtex-engine-build-id 1\n");
    for rel in inputs(engine_dir) {
        let data = std::fs::read(engine_dir.join(&rel)).unwrap_or_default();
        h.update(format!("{} {}\n", rel.display(), data.len()).as_bytes());
        h.update(&data);
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
