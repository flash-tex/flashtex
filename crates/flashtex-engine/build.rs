//! Builds the vendored kpathsea (third_party/kpathsea, LGPL-2.1-or-later) with
//! the `cc` crate when the `kpathsea` feature is on (the default).
//!
//! kpathsea's sources say `#include <kpathsea/...>`, and upstream generates
//! three headers at configure time. So the unmodified sources and our three
//! headers (kpathsea-config/kpathsea/{c-auto,paths,kpathsea}.h) are copied
//! into one `$OUT_DIR/include/kpathsea/` and compiled from there. The source
//! list is `libkpathsea_la_SOURCES` from third_party/kpathsea/Makefile.am for
//! a non-Windows host.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(feature = "kpathsea")]
    kpathsea::build();
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
