//! Computes `FLASHTEX_ENGINE_BUILD_ID` over the engine crate's sources
//! (src/hash.rs says which) and reruns whenever any of them changes. This
//! crate has no C to compile, so a rerun costs milliseconds.

#[path = "src/hash.rs"]
mod hash;

fn main() {
    let engine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let engine = std::fs::canonicalize(&engine).unwrap_or(engine);
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/hash.rs");
    for d in hash::DIRS {
        println!("cargo:rerun-if-changed={}", engine.join(d).display());
    }
    for f in hash::FILES {
        println!("cargo:rerun-if-changed={}", engine.join(f).display());
    }
    println!(
        "cargo:rustc-env=FLASHTEX_ENGINE_BUILD_ID={}",
        hash::engine_sources_id(&engine)
    );
    println!("cargo:rustc-env=FLASHTEX_ENGINE_DIR={}", engine.display());
}
