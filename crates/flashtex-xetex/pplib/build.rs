//! Builds TeX Live 2026's pplib (third_party/pplib, unmodified) as TeX Live's
//! `libs/pplib` builds the `libpplib.a` that XeTeX links
//! (`nodist_libpplib_a_SOURCES` in libs/pplib/Makefile.am), and
//! csrc/flashtex_pdfimage.c, XeTeX's pdfimage.cpp over it.
//!
//! * Sources: the Makefile.am's list without `pptest1.c`, `pptest2.c` and
//!   `pptest3.c`. TeX Live puts those three test programs into the archive
//!   too; each defines `main` and nothing a program refers to, so its
//!   linker never loads them.
//! * Flags: `-I src -I src/util` and zlib's headers (`AM_CPPFLAGS`), and
//!   autoconf's default `-O2` whatever the cargo profile. TeX Live's
//!   `PPLIB_DEFINES` is `-DHAVE_UNISTD_H` plus the large-file macros; no
//!   pplib source reads any of them. No `-DNDEBUG`, as TeX Live gives none
//!   (the library's only `assert`s are in the test programs).
//!   `WARNING_CFLAGS` only select warnings.
//! * zlib: TeX Live's zlib 1.3.2 headers (third_party/zlib) with `Z_PREFIX`,
//!   as crates/flashtex-xetex/teckit compiles it; the objects come from
//!   that crate's archive (Cargo.toml).

use std::path::PathBuf;

/// `nodist_libpplib_a_SOURCES` without the three test programs.
const SOURCES: &[&str] = &[
    "pparray.c",
    "ppcrypt.c",
    "ppdict.c",
    "ppheap.c",
    "ppload.c",
    "ppstream.c",
    "ppxref.c",
    "util/utilbasexx.c",
    "util/utilcrypt.c",
    "util/utilflate.c",
    "util/utilfpred.c",
    "util/utiliof.c",
    "util/utillog.c",
    "util/utillzw.c",
    "util/utilmd5.c",
    "util/utilmem.c",
    "util/utilmemheap.c",
    "util/utilmemheapiof.c",
    "util/utilmeminfo.c",
    "util/utilnumber.c",
    "util/utilsha.c",
];

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let third_party = manifest.join("../../../third_party");
    let src = third_party.join("pplib/pplib-src/src");
    let zlib = third_party.join("zlib/zlib-src");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=csrc");
    println!("cargo:rerun-if-changed={}", src.display());

    // zconf.h as TeX Live's configure makes it for zlib 1.3.2
    // (crates/flashtex-xetex/teckit/build.rs).
    let zconf = out.join("zlib");
    std::fs::create_dir_all(&zconf).unwrap();
    std::fs::copy(zlib.join("zconf.h.in"), zconf.join("zconf.h")).unwrap();

    let mut b = cc::Build::new();
    b.include(&src)
        .include(src.join("util"))
        .include(&zconf)
        .include(&zlib)
        .define("Z_PREFIX", None)
        .opt_level(2)
        .warnings(false)
        .flag_if_supported("-w");
    b.file(manifest.join("csrc/flashtex_pdfimage.c"));
    for s in SOURCES {
        b.file(src.join(s));
    }
    b.compile("flashtex_pplib");
    // pplib's number scanner calls libm (`LIBS = -lm`).
    if std::env::var("CARGO_CFG_UNIX").is_ok() {
        println!("cargo:rustc-link-lib=m");
    }
}
