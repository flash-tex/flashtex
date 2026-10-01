//! Builds the vendored kpathsea (third_party/kpathsea, LGPL-2.1-or-later) with
//! the `cc` crate when the `kpathsea` feature is on (the default), and the
//! shim to the C library's regular expressions when `regex` is.
//!
//! kpathsea's sources say `#include <kpathsea/...>`, and upstream generates
//! three headers at configure time. So the unmodified sources and our three
//! headers (kpathsea-config/kpathsea/{c-auto,paths,kpathsea}.h) are copied
//! into one `$OUT_DIR/include/kpathsea/` and compiled from there. The source
//! list is `libkpathsea_la_SOURCES` from third_party/kpathsea/Makefile.am for
//! a non-Windows host.
//!
//! It also builds TeX Live's zlib (third_party/zlib), libpng
//! (third_party/libpng) and xpdf (third_party/xpdf), all unmodified, for the
//! PDF writer and image inclusion, with the two small C/C++ interfaces of
//! csrc/, except for the `tex82` scratch build, which has no PDF writer.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo::rustc-check-cfg=cfg(flashtex_zlib)");
    println!("cargo::rustc-check-cfg=cfg(flashtex_images)");
    #[cfg(feature = "kpathsea")]
    kpathsea::build();
    // `\pdfmatch`: the C library's regcomp/regexec behind a shim, because
    // regex_t and regmatch_t differ between C libraries.
    #[cfg(feature = "regex")]
    {
        println!("cargo:rerun-if-changed=csrc/flashtex_regex.c");
        cc::Build::new()
            .file("csrc/flashtex_regex.c")
            .compile("flashtex_regex");
    }
    if std::env::var_os("CARGO_FEATURE_TEX82").is_none() {
        zlib::build();
        libpng::build();
        xpdf::build();
        println!("cargo:rustc-cfg=flashtex_images");
    }
}

/// TeX Live's `libs/libpng`: the sources of its `libpng.a`
/// (`nodist_libpng_a_SOURCES` in libs/libpng/Makefile.am, with the ARM NEON
/// files where its `PNG_ARM_NEON` conditional adds them: `arm*`/`aarch64*`
/// hosts), plus csrc/png_shim.c. libpng includes `zlib.h`, which is TeX
/// Live's zlib built above, with the same `Z_PREFIX` so that its calls
/// reach that zlib. `pnglibconf.h` is TeX Live's copy of
/// `scripts/pnglibconf.h.prebuilt` (third_party/libpng/README.md).
mod libpng {
    use std::path::PathBuf;

    const SOURCES: &[&str] = &[
        "png.c",
        "pngerror.c",
        "pngget.c",
        "pngmem.c",
        "pngpread.c",
        "pngread.c",
        "pngrio.c",
        "pngrtran.c",
        "pngrutil.c",
        "pngset.c",
        "pngtrans.c",
        "pngwio.c",
        "pngwrite.c",
        "pngwtran.c",
        "pngwutil.c",
    ];
    const ARM_SOURCES: &[&str] = &[
        "arm/arm_init.c",
        "arm/filter_neon_intrinsics.c",
        "arm/palette_neon_intrinsics.c",
    ];

    pub fn build() {
        let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let src = manifest.join("../../third_party/libpng/libpng-src");
        let zlib_src = manifest.join("../../third_party/zlib/zlib-src");
        let zlib_out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("zlib");
        println!("cargo:rerun-if-changed={}", src.display());
        println!("cargo:rerun-if-changed=csrc/png_shim.c");
        let mut b = cc::Build::new();
        // LIBPNG_DEFINES (libs/libpng/configure.ac); PNG_CONFIGURE_LIBPNG
        // only makes pngpriv.h include configure's config.h, whose checks
        // (headers, `pow`, `memset`) every host we build on passes.
        b.include(&src)
            .include(&zlib_out)
            .include(&zlib_src)
            .define("Z_PREFIX", None)
            .define("PNG_NO_MMX_CODE", None)
            .warnings(false)
            .flag_if_supported("-w");
        for s in SOURCES {
            b.file(src.join(s));
        }
        let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
        if arch == "aarch64" || arch == "arm" {
            for s in ARM_SOURCES {
                b.file(src.join(s));
            }
        } else {
            b.define("PNG_ARM_NEON_OPT", "0");
        }
        b.file(manifest.join("csrc/png_shim.c"));
        b.compile("flashtex_png");
    }
}

/// TeX Live's `libs/xpdf`: the sources of its `libxpdf.a` (goo, fofi and
/// xpdf lists of libs/xpdf/Makefile.am), compiled with its `AM_CPPFLAGS`
/// (`-DPDF_PARSER_ONLY`) and `NO_WARN_CXXFLAGS`, against
/// xpdf-config/aconf.h (what its `configure` writes), plus
/// csrc/xpdf_shim.cc.
mod xpdf {
    use std::path::PathBuf;

    const SOURCES: &[&str] = &[
        "goo/FixedPoint.cc",
        "goo/GHash.cc",
        "goo/GList.cc",
        "goo/GString.cc",
        "goo/Trace.cc",
        "goo/gfile.cc",
        "goo/gmem.cc",
        "goo/gmempp.cc",
        "fofi/FoFiBase.cc",
        "fofi/FoFiEncodings.cc",
        "fofi/FoFiIdentifier.cc",
        "fofi/FoFiTrueType.cc",
        "fofi/FoFiType1.cc",
        "fofi/FoFiType1C.cc",
        "xpdf/AcroForm.cc",
        "xpdf/Annot.cc",
        "xpdf/Array.cc",
        "xpdf/BuiltinFont.cc",
        "xpdf/BuiltinFontTables.cc",
        "xpdf/CMap.cc",
        "xpdf/Catalog.cc",
        "xpdf/CharCodeToUnicode.cc",
        "xpdf/Decrypt.cc",
        "xpdf/Dict.cc",
        "xpdf/Error.cc",
        "xpdf/FontEncodingTables.cc",
        "xpdf/Function.cc",
        "xpdf/Gfx.cc",
        "xpdf/GfxFont.cc",
        "xpdf/GfxState.cc",
        "xpdf/GlobalParams.cc",
        "xpdf/JArithmeticDecoder.cc",
        "xpdf/JBIG2Stream.cc",
        "xpdf/JPXStream.cc",
        "xpdf/Lexer.cc",
        "xpdf/Link.cc",
        "xpdf/NameToCharCode.cc",
        "xpdf/Object.cc",
        "xpdf/OptionalContent.cc",
        "xpdf/Outline.cc",
        "xpdf/OutputDev.cc",
        "xpdf/PDF417Barcode.cc",
        "xpdf/PDFDoc.cc",
        "xpdf/PDFDocEncoding.cc",
        "xpdf/PSTokenizer.cc",
        "xpdf/Page.cc",
        "xpdf/Parser.cc",
        "xpdf/SecurityHandler.cc",
        "xpdf/Stream.cc",
        "xpdf/TextString.cc",
        "xpdf/UnicodeMap.cc",
        "xpdf/UnicodeRemapping.cc",
        "xpdf/UTF8.cc",
        "xpdf/XFAScanner.cc",
        "xpdf/XRef.cc",
        "xpdf/Zoox.cc",
    ];

    pub fn build() {
        let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let src = manifest.join("../../third_party/xpdf/xpdf-src");
        println!("cargo:rerun-if-changed={}", src.display());
        println!("cargo:rerun-if-changed=xpdf-config");
        println!("cargo:rerun-if-changed=csrc/xpdf_shim.cc");
        let mut b = cc::Build::new();
        b.cpp(true)
            .include(manifest.join("xpdf-config"))
            .include(src.join("goo"))
            .include(src.join("fofi"))
            .include(src.join("splash"))
            .include(src.join("xpdf"))
            .define("PDF_PARSER_ONLY", None)
            .warnings(false)
            .flag_if_supported("-w")
            .flag_if_supported("-Wno-write-strings");
        for s in SOURCES {
            b.file(src.join(s));
        }
        b.file(manifest.join("csrc/xpdf_shim.cc"));
        b.compile("flashtex_xpdf");
        rpath_cxx_runtime(&b);
    }

    /// Where the dynamic loader finds the C++ runtime that xpdf needs.
    ///
    /// The `cc` crate links `-lstdc++` on Linux, and the linker finds it in
    /// the compiler's own library directory. On an FHS system that directory
    /// is also on the loader's search path; on NixOS (and any toolchain
    /// outside /usr) it is not, so every binary of this crate -- and every
    /// test that runs one, starting with tests/initex.rs -- died with
    /// "libstdc++.so.6: cannot open shared object file" before printing a
    /// byte. When the compiler's libstdc++ lives outside /usr and /lib, record
    /// its directory as an rpath of this crate's binaries and tests. On macOS
    /// (libc++ from the SDK) and on FHS Linux this adds nothing, so release
    /// binaries built there are unchanged.
    fn rpath_cxx_runtime(b: &cc::Build) {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux") {
            return;
        }
        let compiler = b.get_compiler();
        let Ok(out) = std::process::Command::new(compiler.path())
            .args(compiler.args())
            .arg("-print-file-name=libstdc++.so")
            .output()
        else {
            return;
        };
        let printed = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        let lib = PathBuf::from(&printed);
        // A bare name back means the compiler does not know the file.
        if !out.status.success() || !lib.is_absolute() {
            return;
        }
        if ["/usr/", "/lib/", "/lib64/"]
            .iter()
            .any(|p| printed.starts_with(p))
        {
            return;
        }
        if let Some(dir) = lib.parent() {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", dir.display());
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
        // !MINGW32
        "getopt.c",
        "getopt1.c",
        // !WIN32
        "xfseeko.c",
        "xftello.c",
    ];

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
        for s in SOURCES {
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
