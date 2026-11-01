//! The report states the build profile it was measured under, and a
//! hand-written constant drifts away from the manifest. These come from
//! cargo itself, so `meta.build` cannot lie about the binary it describes.

fn main() {
    let v = |k: &str| std::env::var(k).unwrap_or_default();
    println!("cargo:rustc-env=FT_PROFILE={}", v("PROFILE"));
    println!("cargo:rustc-env=FT_OPT_LEVEL={}", v("OPT_LEVEL"));
    println!("cargo:rustc-env=FT_DEBUG={}", v("DEBUG"));
    println!("cargo:rustc-env=FT_TARGET={}", v("TARGET"));
    // Encoded flags are \x1f separated; a space-joined copy is enough to see
    // that two runs used the same ones.
    println!("cargo:rustc-env=FT_RUSTFLAGS={}", v("CARGO_ENCODED_RUSTFLAGS").replace('\u{1f}', " "));
    println!("cargo:rerun-if-changed=build.rs");
}
