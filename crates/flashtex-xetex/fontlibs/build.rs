//! Builds TeX Live 2026's FreeType 2.14.1 (third_party/freetype) and HarfBuzz
//! 12.3.2 (third_party/harfbuzz), both unmodified, with the configuration TeX
//! Live's build uses (libs/freetype2/Makefile.am, libs/harfbuzz/Makefile.am
//! and configure.ac at texlive-2026.1). Every define and flag is listed, with
//! its source, in the two READMEs; the summary is:
//!
//! * FreeType: TeX Live runs FreeType's own `builds/unix` configure with
//!   `--disable-shared --without-bzip2 --without-brotli --without-harfbuzz
//!   --without-png --without-zlib`. That configure leaves `ftoption.h`
//!   unchanged, writes `ftconfig.h` from `builds/unix/ftconfig.h.in` (defining
//!   `HAVE_UNISTD_H` and `HAVE_FCNTL_H`) and `ftmodule.h` from `modules.cfg`,
//!   and compiles the 43 files of [`freetype::SOURCES`] with `-DFT2_BUILD_LIBRARY
//!   -DFT_CONFIG_CONFIG_H=<ftconfig.h> -DFT_CONFIG_MODULES_H=<ftmodule.h>
//!   -DFT_CONFIG_OPTIONS_H=<ftoption.h>` (plus `-DDARWIN_NO_CARBON` on macOS),
//!   `-std=c99 -O2 -fvisibility=hidden`.
//! * HarfBuzz: TeX Live compiles `src/harfbuzz.cc` (the amalgamation) and the
//!   subsetter sources with `-DHAVE_CONFIG_H -DHB_NO_MT -DHAVE_FALLBACK=1`
//!   (here without `HB_NO_MT`: see `harfbuzz::build`),
//!   `-O2 -fno-rtti -fno-exceptions -fvisibility=hidden
//!   -fvisibility-inlines-hidden`, against configure's `config.h`
//!   (config/harfbuzz/config.h here) and `hb-version.h` (generated below from
//!   `src/hb-version.h.in`, as `config.status` does). The subsetter is
//!   vendored but not compiled: XeTeX never calls it.
//!
//! Both are compiled at `-O2` whatever the cargo profile, as TeX Live does.
//! Graphite2 (which TeX Live links into HarfBuzz) comes with phase S2.

use std::path::{Path, PathBuf};

/// HarfBuzz's version, from TeX Live's own `libs/harfbuzz/version.ac`
/// (vendored as third_party/harfbuzz/version.ac and sha-pinned with the
/// sources), as TeX Live's configure takes it.
fn hb_version(third_party: &Path) -> (String, String, String) {
    let ac = third_party.join("harfbuzz/version.ac");
    println!("cargo:rerun-if-changed={}", ac.display());
    let text = std::fs::read_to_string(&ac).unwrap();
    let v = text
        .lines()
        .find_map(|l| l.strip_prefix("m4_define([harfbuzz_version], ["))
        .and_then(|r| r.strip_suffix("])"))
        .expect("version.ac defines harfbuzz_version");
    let mut it = v.split('.').map(str::to_string);
    let (a, b, c) = (it.next(), it.next(), it.next());
    (a.unwrap(), b.unwrap(), c.unwrap())
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let third_party = manifest.join("../../../third_party");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=config");
    println!("cargo:rerun-if-changed=csrc");

    let ft_src = third_party.join("freetype/freetype-src");
    let hb_src = third_party.join("harfbuzz/harfbuzz-src");
    let ft_conf = out.join("freetype-config");
    let hb_conf = out.join("harfbuzz-config");
    freetype::configure(&ft_src, &ft_conf);
    harfbuzz::configure(&manifest, &hb_src, &hb_conf);

    // HarfBuzz's one large translation unit dominates; FreeType's 43 files
    // compile meanwhile.
    let hb = {
        let (hb_src, hb_conf) = (hb_src.clone(), hb_conf.clone());
        std::thread::spawn(move || harfbuzz::build(&hb_src, &hb_conf))
    };
    freetype::build(&ft_src, &ft_conf);
    hb.join().expect("HarfBuzz build thread panicked");

    // The layout probe (csrc/layout_probe.c): sizeof/offsetof of every
    // struct the FFI declares, read by the layout test.
    let mut probe = cc::Build::new();
    freetype::defines(&mut probe, &ft_src, &ft_conf);
    probe
        .include(hb_src.join("src"))
        .include(&hb_conf)
        .file(manifest.join("csrc/layout_probe.c"))
        .opt_level(2)
        .debug(false)
        .warnings(false);
    probe.compile("flashtex_fontlibs_probe");
}

fn is_target(var: &str, value: &str) -> bool {
    std::env::var(var).as_deref() == Ok(value)
}

mod freetype {
    use super::*;

    /// What `make -n` in TeX Live's FreeType build compiles into
    /// `libfreetype.a` (`builds/unix` with the default `modules.cfg`), in its
    /// order. On Windows, where `builds/unix/ftsystem.c` (mmap) does not
    /// apply, `src/base/ftsystem.c` (ANSI stdio) replaces it.
    pub const SOURCES: &[&str] = &[
        "builds/unix/ftsystem.c",
        "src/base/ftdebug.c",
        "src/base/ftinit.c",
        "src/base/ftbase.c",
        "src/base/ftbbox.c",
        "src/base/ftbdf.c",
        "src/base/ftbitmap.c",
        "src/base/ftcid.c",
        "src/base/ftfstype.c",
        "src/base/ftgasp.c",
        "src/base/ftglyph.c",
        "src/base/ftgxval.c",
        "src/base/ftmm.c",
        "src/base/ftotval.c",
        "src/base/ftpatent.c",
        "src/base/ftpfr.c",
        "src/base/ftstroke.c",
        "src/base/ftsynth.c",
        "src/base/fttype1.c",
        "src/base/ftwinfnt.c",
        "src/truetype/truetype.c",
        "src/type1/type1.c",
        "src/cff/cff.c",
        "src/cid/type1cid.c",
        "src/pfr/pfr.c",
        "src/type42/type42.c",
        "src/winfonts/winfnt.c",
        "src/pcf/pcf.c",
        "src/bdf/bdf.c",
        "src/sfnt/sfnt.c",
        "src/autofit/autofit.c",
        "src/pshinter/pshinter.c",
        "src/smooth/smooth.c",
        "src/raster/raster.c",
        "src/svg/svg.c",
        "src/sdf/sdf.c",
        "src/cache/ftcache.c",
        "src/gzip/ftgzip.c",
        "src/lzw/ftlzw.c",
        "src/bzip2/ftbzip2.c",
        "src/psaux/psaux.c",
        "src/psnames/psnames.c",
        "src/dlg/dlgwrap.c",
    ];

    /// `ftmodule.h` as `builds/unix` generates it from the default
    /// `modules.cfg` (its order, which is the order FT_Init_FreeType adds
    /// the modules in).
    const FTMODULE_H: &str = "/* This is a generated file. */
FT_USE_MODULE( FT_Driver_ClassRec, tt_driver_class )
FT_USE_MODULE( FT_Driver_ClassRec, t1_driver_class )
FT_USE_MODULE( FT_Driver_ClassRec, cff_driver_class )
FT_USE_MODULE( FT_Driver_ClassRec, t1cid_driver_class )
FT_USE_MODULE( FT_Driver_ClassRec, pfr_driver_class )
FT_USE_MODULE( FT_Driver_ClassRec, t42_driver_class )
FT_USE_MODULE( FT_Driver_ClassRec, winfnt_driver_class )
FT_USE_MODULE( FT_Driver_ClassRec, pcf_driver_class )
FT_USE_MODULE( FT_Driver_ClassRec, bdf_driver_class )
FT_USE_MODULE( FT_Module_Class, sfnt_module_class )
FT_USE_MODULE( FT_Module_Class, autofit_module_class )
FT_USE_MODULE( FT_Module_Class, pshinter_module_class )
FT_USE_MODULE( FT_Renderer_Class, ft_smooth_renderer_class )
FT_USE_MODULE( FT_Renderer_Class, ft_raster1_renderer_class )
FT_USE_MODULE( FT_Renderer_Class, ft_svg_renderer_class )
FT_USE_MODULE( FT_Renderer_Class, ft_sdf_renderer_class )
FT_USE_MODULE( FT_Renderer_Class, ft_bitmap_sdf_renderer_class )
FT_USE_MODULE( FT_Module_Class, psaux_module_class )
FT_USE_MODULE( FT_Module_Class, psnames_module_class )
/* EOF */
";

    /// Writes `ftconfig.h` and `ftmodule.h` into `conf`, as FreeType's
    /// `builds/unix/configure` does into its build directory. Its
    /// `ftoption.h` is the unchanged `include/freetype/config/ftoption.h`
    /// (configure only edits it for the external libraries TeX Live turns
    /// off), so that file is used in place.
    pub fn configure(src: &Path, conf: &Path) {
        std::fs::create_dir_all(conf).unwrap();
        let input = std::fs::read_to_string(src.join("builds/unix/ftconfig.h.in")).unwrap();
        let mut ftconfig =
            String::from("/* ftconfig.h.  Generated from ftconfig.h.in by configure.  */\n");
        let unix = std::env::var("CARGO_CFG_UNIX").is_ok();
        let mut replaced = 0;
        for line in input.lines() {
            match line {
                "#undef HAVE_UNISTD_H" if unix => {
                    ftconfig.push_str("#define HAVE_UNISTD_H 1\n");
                    replaced += 1;
                }
                "#undef HAVE_FCNTL_H" if unix => {
                    ftconfig.push_str("#define HAVE_FCNTL_H 1\n");
                    replaced += 1;
                }
                _ => {
                    ftconfig.push_str(line);
                    ftconfig.push('\n');
                }
            }
        }
        assert!(
            !unix || replaced == 2,
            "builds/unix/ftconfig.h.in changed shape"
        );
        write_if_changed(&conf.join("ftconfig.h"), &ftconfig);
        write_if_changed(&conf.join("ftmodule.h"), FTMODULE_H);
    }

    /// The preprocessor flags of TeX Live's FreeType build, shared by the
    /// library and by the layout probe (which must see the same headers).
    pub fn defines(b: &mut cc::Build, src: &Path, conf: &Path) {
        b.include(conf)
            .include(src.join("builds/unix"))
            .include(src.join("include"))
            .define("FT_CONFIG_CONFIG_H", "<ftconfig.h>")
            .define("FT_CONFIG_MODULES_H", "<ftmodule.h>")
            .define("FT_CONFIG_OPTIONS_H", "<freetype/config/ftoption.h>");
        if is_target("CARGO_CFG_TARGET_OS", "macos") || is_target("CARGO_CFG_TARGET_OS", "ios") {
            b.define("DARWIN_NO_CARBON", None);
        }
    }

    pub fn build(src: &Path, conf: &Path) {
        println!("cargo:rerun-if-changed={}", src.display());
        let mut b = cc::Build::new();
        defines(&mut b, src, conf);
        b.define("FT2_BUILD_LIBRARY", None)
            .flag_if_supported("-std=c99")
            .flag_if_supported("-fvisibility=hidden")
            .opt_level(2)
            .debug(false)
            .warnings(false)
            .flag_if_supported("-w");
        let windows = is_target("CARGO_CFG_TARGET_OS", "windows");
        for s in SOURCES {
            let s = if windows && *s == "builds/unix/ftsystem.c" {
                "src/base/ftsystem.c"
            } else {
                s
            };
            b.file(src.join(s));
        }
        b.compile("flashtex_freetype");
    }
}

mod harfbuzz {
    use super::*;

    /// `libharfbuzz_a_SOURCES` of libs/harfbuzz/Makefile.am that XeTeX
    /// uses: the amalgamation, which includes every `hb-*.cc` of the shaper,
    /// the OpenType layout, math and font functions. The other 17 sources of
    /// that list are the subsetter (`hb-subset*.cc`, `graph/`), vendored and
    /// not compiled.
    const SOURCES: &[&str] = &["src/harfbuzz.cc"];

    /// `config.h` (config/harfbuzz/config.h) and `hb-version.h`, generated
    /// from `src/hb-version.h.in` exactly as TeX Live's `config.status` does
    /// (`@HB_VERSION_MAJOR@` etc. from libs/harfbuzz/version.ac).
    pub fn configure(manifest: &Path, src: &Path, conf: &Path) {
        std::fs::create_dir_all(conf).unwrap();
        let config = std::fs::read_to_string(manifest.join("config/harfbuzz/config.h")).unwrap();
        write_if_changed(&conf.join("config.h"), &config);
        let (major, minor, micro) = hb_version(&manifest.join("../../../third_party"));
        let template = std::fs::read_to_string(src.join("src/hb-version.h.in")).unwrap();
        let version = template
            .replace("@HB_VERSION_MAJOR@", &major)
            .replace("@HB_VERSION_MINOR@", &minor)
            .replace("@HB_VERSION_MICRO@", &micro)
            .replace("@HB_VERSION@", &format!("{major}.{minor}.{micro}"));
        assert!(
            !version.contains("@HB_"),
            "src/hb-version.h.in has an unknown substitution"
        );
        write_if_changed(&conf.join("hb-version.h"), &version);
    }

    pub fn build(src: &Path, conf: &Path) {
        println!("cargo:rerun-if-changed={}", src.display());
        let mut b = cc::Build::new();
        // DEFS, DEFAULT_INCLUDES and AM_CPPFLAGS of libs/harfbuzz/Makefile:
        // -DHAVE_CONFIG_H -I<build dir> -DHB_NO_MT -DHAVE_FALLBACK=1
        // -I<harfbuzz-src/src>, except HB_NO_MT: an engine may move to
        // another thread (Globals is Send), so HarfBuzz keeps its locking and
        // atomic lazy globals. Locking changes no shaping result. The compiler's default C++ dialect, as there
        // (configure: "g++ supports C++11 features by default... yes").
        b.cpp(true)
            .include(conf)
            .include(src.join("src"))
            .define("HAVE_CONFIG_H", None)
            .define("HAVE_FALLBACK", "1")
            .flag_if_supported("-fno-rtti")
            .flag_if_supported("-fno-exceptions")
            .flag_if_supported("-fvisibility=hidden")
            .flag_if_supported("-fvisibility-inlines-hidden")
            .opt_level(2)
            .debug(false)
            .warnings(false)
            .flag_if_supported("-w");
        for s in SOURCES {
            b.file(src.join(s));
        }
        b.compile("flashtex_harfbuzz");
    }
}

fn write_if_changed(path: &Path, contents: &str) {
    if std::fs::read_to_string(path).ok().as_deref() != Some(contents) {
        std::fs::write(path, contents).unwrap();
    }
}
