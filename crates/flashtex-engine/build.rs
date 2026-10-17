//! Builds the shim to the C library's regular expressions with the `cc`
//! crate when the `regex` feature is on. The vendored kpathsea (feature
//! `kpathsea`) is built by crates/flashtex-runtime, with the resolver that
//! calls it (docs/design/xetex/PLAN.md §3.3).
//!
//! It also builds TeX Live's libpng (third_party/libpng) and xpdf
//! (third_party/xpdf), both unmodified, for the PDF writer and image
//! inclusion, with the two small C/C++ interfaces of csrc/, except for the
//! `tex82` scratch build, which has no PDF writer. TeX Live's zlib, which
//! they and the PDF writer use, is built by crates/flashtex-runtime
//! (feature `zlib`; docs/design/xetex/PLAN.md §3.3).

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo::rustc-check-cfg=cfg(flashtex_images)");
    println!("cargo::rustc-check-cfg=cfg(flashtex_regex)");
    // `\pdfmatch`: regcomp/regexec behind a shim (csrc/flashtex_regex.c),
    // because regex_t and regmatch_t differ between C libraries. The
    // regcomp is pdfTeX's own: the C library's on Unix; on Windows, whose C
    // runtimes have no <regex.h>, pdfTeX's copy of glibc 2.5's,
    // pdftexdir/regex, which TeX Live compiles `if MINGW32`
    // (pdftexdir/am/libpdftex.am) and this build compiles for Windows
    // (third_party/pdftex-regex, LGPL-2.1-or-later).
    #[cfg(feature = "regex")]
    {
        println!("cargo:rerun-if-changed=csrc/flashtex_regex.c");
        let mut b = cc::Build::new();
        b.file("csrc/flashtex_regex.c");
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
            let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
            let dir = manifest.join("../../third_party/pdftex-regex");
            println!("cargo:rerun-if-changed={}", dir.display());
            // regex.c says `#include <regex/regex.h>` and includes the
            // other three .c files; the shim's <regex.h> is that header
            // (libpdftex.am's REGEX_INCLUDES).
            // glibc 2.5's C: `typedef enum { false, true } bool;` without
            // HAVE_STDBOOL_H (TeX Live defines no HAVE_* for this file),
            // which C23, GCC 15's default, rejects. The C17 dialect of the
            // compilers TeX Live builds it with.
            b.file(dir.join("regex/regex.c"))
                .include(&dir)
                .include(dir.join("regex"))
                .flag_if_supported("-std=gnu17")
                .warnings(false);
        }
        b.compile("flashtex_regex");
        println!("cargo:rustc-cfg=flashtex_regex");
    }
    if std::env::var_os("CARGO_FEATURE_TEX82").is_none() {
        libpng::build();
        xpdf::build();
        println!("cargo:rustc-cfg=flashtex_images");
    }
    // The Win32 libraries the vendored C calls: kpathsea's knj.c
    // (CommandLineToArgvW, shell32; built by flashtex-runtime, which links
    // shell32 after it too) and xpdf's GlobalParams.cc
    // (SHGetFolderPathA, shell32; the registry, advapi32). Last, after every
    // static library above: GNU ld resolves left to right.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=shell32");
        println!("cargo:rustc-link-lib=advapi32");
    }
}

/// TeX Live's `libs/libpng`: the sources of its `libpng.a`
/// (`nodist_libpng_a_SOURCES` in libs/libpng/Makefile.am, with the ARM NEON
/// files where its `PNG_ARM_NEON` conditional adds them: `arm*`/`aarch64*`
/// hosts), plus csrc/png_shim.c. libpng includes `zlib.h`, which is TeX
/// Live's zlib built by crates/flashtex-runtime (its include directories
/// through `links`: DEP_FLASHTEX_ZLIB_INCLUDE, the generated `zconf.h`, and
/// DEP_FLASHTEX_ZLIB_SRC, `zlib.h`), with the same `Z_PREFIX` so that its
/// calls reach that zlib. `pnglibconf.h` is TeX Live's copy of
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
        let zlib_dir = |k: &str| {
            PathBuf::from(std::env::var(k).unwrap_or_else(|_| {
                panic!("{k}: flashtex-runtime's `zlib` feature builds TeX Live's zlib")
            }))
        };
        let zlib_out = zlib_dir("DEP_FLASHTEX_ZLIB_INCLUDE");
        let zlib_src = zlib_dir("DEP_FLASHTEX_ZLIB_SRC");
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
        // MinGW: the C++ runtime linked statically, so that the engine is
        // one .exe with no libstdc++-6.dll beside it.
        let mingw = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
            && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu");
        if mingw {
            b.cpp_link_stdlib(None);
        }
        b.compile("flashtex_xpdf");
        if mingw {
            let compiler = b.get_compiler();
            let lib = std::process::Command::new(compiler.path())
                .arg("-print-file-name=libstdc++.a")
                .output()
                .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
                .expect("the MinGW C++ compiler");
            if let Some(dir) = lib.parent().filter(|_| lib.is_absolute()) {
                println!("cargo:rustc-link-search=native={}", dir.display());
            }
            println!("cargo:rustc-link-lib=static=stdc++");
        }
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
