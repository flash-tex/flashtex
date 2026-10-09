//! Builds TeX Live 2026's TECkit 2.5.13 conversion engine
//! (third_party/teckit, unmodified) as TeX Live's `libs/teckit` builds the
//! `libTECkit.a` that XeTeX links (`nodist_libTECkit_a_SOURCES` in
//! libs/teckit/Makefile.am: `source/Engine.cpp` only, which `#include`s
//! `NormalizationData.c`), and TeX Live's zlib 1.3.2 (third_party/zlib), which
//! Engine.cpp calls (`uncompress`) for compressed `.tec` mappings.
//!
//! * TECkit: `-DHAVE_CONFIG_H` (automake's `DEFS`) with configure's
//!   `config.h` (config/config.h, see its header), `-DNDEBUG` and
//!   `-I source/Public-headers` (`AM_CPPFLAGS`), zlib's headers
//!   (`ZLIB_INCLUDES`), and autoconf's default `-O2` whatever the cargo
//!   profile. TeX Live's `WARNING_CXXFLAGS` only select warnings.
//! * zlib: compiled as crates/flashtex-engine/build.rs compiles it: the
//!   sources of TeX Live's `libz.a` except the `gz*.c` file layer, `zconf.h`
//!   copied from `zconf.h.in` (what TeX Live's configure makes of it for zlib
//!   1.3.2), and `Z_PREFIX`, zconf.h's own switch, which names every symbol
//!   `z_*` so that it can never be confused with a system libz. TeX Live's
//!   TECkit includes TeX Live's own zlib.h the same way (`ZLIB_INCLUDES`),
//!   without `Z_PREFIX`, which renames symbols only.
//!
//! The XeTeX port also links crates/flashtex-engine, whose build compiles
//! the same zlib files with the same switch into its own archive. The two
//! copies are object for object the same source, so the linker, which loads
//! an archive member only for a symbol still undefined, takes each member
//! from one archive or the other and never both: no symbol is defined twice,
//! and whichever copy serves `z_uncompress` is TeX Live's zlib 1.3.2.

use std::path::PathBuf;

/// The sources of TeX Live's `libz.a` (`nodist_libz_a_SOURCES`) without
/// `gz*.c`, as crates/flashtex-engine/build.rs lists them.
const ZLIB_SOURCES: &[&str] = &[
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

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let third_party = manifest.join("../../../third_party");
    let teckit = third_party.join("teckit/TECkit-src/source");
    let zlib = third_party.join("zlib/zlib-src");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=config");
    println!("cargo:rerun-if-changed={}", teckit.display());
    println!("cargo:rerun-if-changed={}", zlib.display());

    let zconf = out.join("zlib");
    std::fs::create_dir_all(&zconf).unwrap();
    std::fs::copy(zlib.join("zconf.h.in"), zconf.join("zconf.h")).unwrap();

    let mut t = cc::Build::new();
    t.cpp(true)
        .file(teckit.join("Engine.cpp"))
        .define("HAVE_CONFIG_H", None)
        .define("NDEBUG", None)
        .define("Z_PREFIX", None)
        .include(manifest.join("config"))
        .include(teckit.join("Public-headers"))
        .include(&zconf)
        .include(&zlib)
        .opt_level(2)
        .warnings(false)
        .flag_if_supported("-w");
    t.compile("flashtex_teckit");

    // After TECkit, which calls it: GNU ld resolves left to right.
    let mut z = cc::Build::new();
    z.include(&zconf)
        .include(&zlib)
        .define("Z_PREFIX", None)
        .opt_level(2)
        .warnings(false)
        .flag_if_supported("-w");
    for s in ZLIB_SOURCES {
        z.file(zlib.join(s));
    }
    z.compile("flashtex_teckit_zlib");
}
