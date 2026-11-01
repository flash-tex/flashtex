//! Records the C++ runtime's directory as an rpath of this crate's binary and
//! tests, on Linux toolchains outside /usr and /lib (NixOS).
//!
//! HarfBuzz (fontlibs/) is C++, so the `cc` crate links `-lstdc++`; that link
//! library reaches this crate, but a dependency's `rustc-link-arg` does not.
//! fontlibs/build.rs therefore finds the directory (`rpath_cxx_runtime`, a
//! copy of the one #1232 added to crates/flashtex-engine/build.rs) and passes
//! it here as `links` metadata. It is unset on macOS, Windows and FHS Linux,
//! where this adds nothing.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if let Ok(dir) = std::env::var("DEP_FLASHTEX_XETEX_FONTLIBS_CXX_RPATH") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
    }
}
