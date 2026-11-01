//! Builds TeX Live's zlib (third_party/zlib, zlib licence) with the `cc`
//! crate when the `zlib` feature is on (flashtex-engine sets it), moved
//! unchanged from crates/flashtex-engine/build.rs (docs/design/xetex/PLAN.md
//! §3.3). Its include directories go to the crates that build C against it
//! (the engine's libpng) as `DEP_FLASHTEX_ZLIB_INCLUDE` (the generated
//! `zconf.h`) and `DEP_FLASHTEX_ZLIB_SRC` (`zlib.h`), through `links`.
//!
//! And the vendored kpathsea (third_party/kpathsea, LGPL-2.1-or-later) when
//! the `kpathsea` feature is on, moved unchanged from the same build script
//! with the resolver that calls it. kpathsea's sources say
//! `#include <kpathsea/...>`, and upstream generates three headers at
//! configure time. So the unmodified sources and our three headers
//! (kpathsea-config/kpathsea/{c-auto,paths,kpathsea}.h) are copied into one
//! `$OUT_DIR/include/kpathsea/` and compiled from there. The source list is
//! `libkpathsea_la_SOURCES` from third_party/kpathsea/Makefile.am for a
//! non-Windows host.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo::rustc-check-cfg=cfg(flashtex_zlib)");
    if std::env::var_os("CARGO_FEATURE_ZLIB").is_some() {
        zlib::build();
    }
    #[cfg(feature = "kpathsea")]
    {
        kpathsea::build();
        // kpathsea's knj.c calls CommandLineToArgvW (shell32): after the
        // static library, since GNU ld resolves left to right.
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
            println!("cargo:rustc-link-lib=shell32");
        }
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

#[cfg(feature = "kpathsea")]
mod kpathsea {
    use std::path::{Path, PathBuf};

    const SOURCES: &[&str] = &[
        "tex-file.c",
        "absolute.c",
        "atou.c",
        "cnf.c",
        "concat.c",
        "concat3.c",
        "concatn.c",
        "db.c",
        "debug.c",
        "dir.c",
        "elt-dirs.c",
        "expand.c",
        "extend-fname.c",
        "file-p.c",
        "find-suffix.c",
        "fn.c",
        "fontmap.c",
        "hash.c",
        "kdefault.c",
        "kpathsea.c",
        "line.c",
        "magstep.c",
        "make-suffix.c",
        "path-elt.c",
        "pathsearch.c",
        "proginit.c",
        "progname.c",
        "readable.c",
        "rm-suffix.c",
        "str-list.c",
        "str-llist.c",
        "tex-glyph.c",
        "tex-hush.c",
        "tex-make.c",
        "tilde.c",
        "uppercasify.c",
        "variable.c",
        "version.c",
        "xbasename.c",
        "xcalloc.c",
        "xdirname.c",
        "xfopen.c",
        "xfseek.c",
        "xftell.c",
        "xgetcwd.c",
        "xmalloc.c",
        "xopendir.c",
        "xputenv.c",
        "xrealloc.c",
        "xstat.c",
        "xstrdup.c",
    ];

    /// Makefile.am's conditional sources for the target: `!MINGW32`
    /// getopt, then `WIN32` (MinGW: mingw32.c, xfseeko.c, xftello.c; MSVC:
    /// win32lib.c; both: knj.c) or `!WIN32` (xfseeko.c, xftello.c).
    fn conditional_sources() -> &'static [&'static str] {
        let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
        let gnu = std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu");
        match (windows, gnu) {
            (true, true) => &["mingw32.c", "xfseeko.c", "xftello.c", "knj.c"],
            (true, false) => &["getopt.c", "getopt1.c", "win32lib.c", "knj.c"],
            (false, _) => &["getopt.c", "getopt1.c", "xfseeko.c", "xftello.c"],
        }
    }

    pub fn build() {
        let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let vendored = manifest.join("../../third_party/kpathsea");
        let config = manifest.join("kpathsea-config");
        let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let inc = out.join("include");
        let dst = inc.join("kpathsea");
        std::fs::create_dir_all(&dst).unwrap();

        println!("cargo:rerun-if-changed={}", vendored.display());
        println!("cargo:rerun-if-changed={}", config.display());

        copy_matching(&vendored, &dst, |n| n.ends_with(".c") || n.ends_with(".h"));
        copy_matching(&config.join("kpathsea"), &dst, |n| n.ends_with(".h"));

        let mut b = cc::Build::new();
        // libkpathsea_la_CPPFLAGS in Makefile.am: MAKE_KPSE_DLL exposes the
        // library-internal declarations.
        // DEFS/DEFAULT_INCLUDES: -DHAVE_CONFIG_H -I. (getopt.c includes <config.h>).
        b.include(&inc)
            .include(&dst)
            .define("MAKE_KPSE_DLL", None)
            .define("HAVE_CONFIG_H", None)
            .warnings(false)
            .flag_if_supported("-w");
        for s in SOURCES.iter().chain(conditional_sources()) {
            b.file(dst.join(s));
        }
        b.file(config.join("flashtex_kpse.c"));
        b.compile("flashtex_kpathsea");
    }

    fn copy_matching(from: &Path, to: &Path, keep: impl Fn(&str) -> bool) {
        for e in std::fs::read_dir(from).unwrap_or_else(|e| panic!("{}: {e}", from.display())) {
            let e = e.unwrap();
            let name = e.file_name().to_string_lossy().into_owned();
            if keep(&name) {
                std::fs::copy(e.path(), to.join(&name)).unwrap();
            }
        }
    }
}
