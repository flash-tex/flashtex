//! Builds TeX Live's zlib (third_party/zlib, zlib licence) with the `cc`
//! crate when the `zlib` feature is on (flashtex-engine sets it), moved
//! unchanged from crates/flashtex-engine/build.rs (docs/design/xetex/PLAN.md
//! §3.3). Its include directories go to the crates that build C against it
//! (the engine's libpng) as `DEP_FLASHTEX_ZLIB_INCLUDE` (the generated
//! `zconf.h`) and `DEP_FLASHTEX_ZLIB_SRC` (`zlib.h`), through `links`.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo::rustc-check-cfg=cfg(flashtex_zlib)");
    if std::env::var_os("CARGO_FEATURE_ZLIB").is_some() {
        zlib::build();
    }
}

/// TeX Live's `libs/zlib`: the sources of its `libz.a`
/// (`nodist_libz_a_SOURCES` in libs/zlib/Makefile.am) except the `gz*.c`
/// file-I/O layer, which the engine never calls, with `zconf.h` made
/// from `zconf.h.in` as its `configure` makes it (a copy, for zlib 1.3.2;
/// see third_party/zlib/README.md). `Z_PREFIX`, zconf.h's own switch,
/// renames the exported symbols to `z_*` so that they can never be confused
/// with a system libz in the same process; it changes no code.
mod zlib {
    use std::path::PathBuf;

    const SOURCES: &[&str] = &[
        "adler32.c",
        "compress.c",
        "crc32.c",
        "deflate.c",
        "infback.c",
        "inffast.c",
        "inflate.c",
        "inftrees.c",
        "trees.c",
        "uncompr.c",
        "zutil.c",
    ];

    pub fn build() {
        let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let src = manifest.join("../../third_party/zlib/zlib-src");
        let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("zlib");
        std::fs::create_dir_all(&out).unwrap();
        println!("cargo:rerun-if-changed={}", src.display());
        std::fs::copy(src.join("zconf.h.in"), out.join("zconf.h")).unwrap();
        let mut b = cc::Build::new();
        b.include(&out)
            .include(&src)
            .define("Z_PREFIX", None)
            .warnings(false)
            .flag_if_supported("-w");
        for s in SOURCES {
            b.file(src.join(s));
        }
        b.compile("flashtex_zlib");
        println!("cargo:rustc-cfg=flashtex_zlib");
        // (`links = "flashtex_zlib"`: the dependents' build scripts see
        // these as DEP_FLASHTEX_ZLIB_INCLUDE and DEP_FLASHTEX_ZLIB_SRC)
        println!("cargo:include={}", out.display());
        println!("cargo:src={}", src.display());
    }
}
