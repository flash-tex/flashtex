//! Builds TeX Live 2026's ICU 78.2 common library (third_party/icu,
//! unmodified) with the configuration TeX Live's build uses, and links a
//! subset of TeX Live's ICU data in the way TeX Live links all of it.
//!
//! * Library: TeX Live's `libs/icu` runs ICU's own `source/configure` with
//!   `--enable-static --disable-shared --disable-extras --disable-samples
//!   --disable-tests --disable-dyload --disable-layout`
//!   (`libs/icu/configure.ac`) and builds `libicuuc.a` from the 202 files of
//!   `source/common/sources.txt` with `$(DEFS) $(CPPFLAGS) $(CXXFLAGS)`:
//!   `-DU_ALL_IMPLEMENTATION -DU_ATTRIBUTE_DEPRECATED= -DU_ENABLE_DYLOAD=0
//!   -I<common> -DU_COMMON_IMPLEMENTATION
//!   -DDEFAULT_ICU_PLUGINS="/usr/local/lib/icu"` and `-O2 -std=c++17`, plus
//!   `-fvisibility=hidden -fno-common` on macOS (`config/mh-darwin`). That
//!   configure, run unmodified on macOS on 2026-10-04, writes no header (ICU
//!   78's `platform.h` and `uconfig.h` are static), so nothing is generated.
//!   ICU's version-suffix renaming (`urename.h`: `ubidi_open` is
//!   `ubidi_open_78`) is on, as there.
//! * Data: TeX Live links `libicudata.a`, its trimmed
//!   `source/data/in/icudt78l.dat` turned into an object by ICU's `pkgdata`
//!   (`genccode`: a C array named `icudt78_dat`, `U_ICUDATA_ENTRY_POINT`).
//!   FlashTeX links a package of the items of that .dat it uses, extracted
//!   unchanged into `third_party/icu/icudt78l/` (README there), repacked in
//!   the same common-data format and written out the way `genccode` writes
//!   its C (see [`data`]). Items not in the package are absent to ICU, as a
//!   missing data file is.
//!
//! Both are compiled at `-O2` whatever the cargo profile, as TeX Live does.

use std::path::{Path, PathBuf};

const ICU_DATA_NAME: &str = "icudt78l";
const ICU_ENTRY_POINT: &str = "icudt78_dat";

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let icu = manifest.join("../../../third_party/icu");
    let common = icu.join("icu-src/source/common");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=csrc");
    println!("cargo:rerun-if-changed={}", common.display());
    println!(
        "cargo:rerun-if-changed={}",
        icu.join(ICU_DATA_NAME).display()
    );
    assert_eq!(
        std::env::var("CARGO_CFG_TARGET_ENDIAN").as_deref(),
        Ok("little"),
        "TeX Live's ICU data (icudt78l) is little-endian"
    );

    let sources: Vec<PathBuf> = std::fs::read_to_string(common.join("sources.txt"))
        .unwrap()
        .split_whitespace()
        .map(|s| common.join(s))
        .collect();
    assert_eq!(sources.len(), 202, "source/common/sources.txt changed");

    // Compile the 202 files in NUM_JOBS threads, then archive them with the
    // data object into one library.
    let jobs: usize = std::env::var("NUM_JOBS")
        .ok()
        .and_then(|j| j.parse().ok())
        .unwrap_or(4)
        .clamp(1, 16);
    let chunk = sources.len().div_ceil(jobs);
    let threads: Vec<_> = sources
        .chunks(chunk)
        .map(|files| {
            let (files, common) = (files.to_vec(), common.clone());
            std::thread::spawn(move || {
                let mut b = cxx_build(&common);
                b.files(&files);
                b.compile_intermediates()
            })
        })
        .collect();
    let mut objects = vec![];
    for t in threads {
        objects.extend(t.join().expect("ICU compile thread panicked"));
    }

    let package = data::package(&icu.join(ICU_DATA_NAME));
    let data_c = out.join(format!("{ICU_DATA_NAME}_dat.c"));
    data::write_c_if_changed(&data_c, &package);

    let mut lib = cc::Build::new();
    lib.include(&common)
        .file(&data_c)
        .objects(&objects)
        .opt_level(2)
        .debug(false)
        .warnings(false)
        .flag_if_supported("-w");
    lib.compile("flashtex_icu");

    // csrc/probe.c: the constants and sizes the FFI declares, read by the
    // crate's tests.
    let mut probe = cc::Build::new();
    probe
        .include(&common)
        .file(manifest.join("csrc/probe.c"))
        .opt_level(2)
        .debug(false)
        .warnings(false);
    probe.compile("flashtex_icu_probe");

    // libicuuc uses the C++ standard library (as TeX Live links xetex with
    // the C++ compiler); `cc` links it for the C++ objects above only when
    // a `cpp(true)` build is compiled, which `cxx_build`'s are not here.
    let target = std::env::var("TARGET").unwrap();
    if target.contains("apple") {
        println!("cargo:rustc-link-lib=c++");
    } else if !target.contains("msvc") {
        println!("cargo:rustc-link-lib=stdc++");
    }
}

/// ICU's flags for `source/common` (see the module comment).
fn cxx_build(common: &Path) -> cc::Build {
    let mut b = cc::Build::new();
    b.cpp(true)
        .cpp_link_stdlib(None)
        .include(common)
        .define("U_ALL_IMPLEMENTATION", None)
        .define("U_ATTRIBUTE_DEPRECATED", "")
        .define("U_ENABLE_DYLOAD", "0")
        .define("U_COMMON_IMPLEMENTATION", None)
        .define("DEFAULT_ICU_PLUGINS", "\"/usr/local/lib/icu\"")
        .flag_if_supported("-std=c++17")
        .flag_if_supported("/std:c++17")
        .flag_if_supported("-fvisibility=hidden")
        .flag_if_supported("-fno-common")
        .opt_level(2)
        .debug(false)
        .warnings(false)
        .flag_if_supported("-w");
    b
}

/// ICU's common-data package (`ucmndata.h`: a `MappedData` header and a
/// `UDataInfo` of format `CmnD` 1.0, then a table of contents of
/// `(name offset, data offset)` pairs sorted by name, then the items, each
/// at a 16-byte boundary), as `icupkg` writes one, and the C `genccode`
/// writes for it.
mod data {
    use std::fmt::Write as _;
    use std::path::Path;

    fn items(dir: &Path, rel: &str, out: &mut Vec<(String, Vec<u8>)>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            let name = p.file_name().unwrap().to_str().unwrap();
            let rel = format!("{rel}/{name}");
            if p.is_dir() {
                items(&p, &rel, out);
            } else {
                out.push((rel, std::fs::read(&p).unwrap()));
            }
        }
    }

    /// The package of every file under `dir` (named `icudt78l/<path>`).
    pub fn package(dir: &Path) -> Vec<u8> {
        let mut entries = vec![];
        items(dir, super::ICU_DATA_NAME, &mut entries);
        entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        assert!(entries.len() > 30, "{} data items", entries.len());
        // ICU knows an item's length only from where the next item starts:
        // the last item of a package has none (`offsetTOCLookupFn` gives
        // -1), and an item without a length fails `ucnv_io`'s check of the
        // alias table. TeX Live's last item is `zu_ZA.res`, which XeTeX
        // never loads; here a sentinel entry with no data, sorting after
        // every item name, ends the last item. No lookup can name it.
        entries.push((format!("{}/~", super::ICU_DATA_NAME), vec![]));

        // MappedData + UDataInfo: headerSize 32, magic 0xda27; UDataInfo
        // size 20, little-endian, ASCII, sizeof(UChar) 2, "CmnD", format
        // 1.0.0.0, data version 3.0.0.0 (the values of TeX Live's .dat).
        let mut pkg = vec![0u8; 32];
        pkg[0..2].copy_from_slice(&32u16.to_le_bytes());
        pkg[2] = 0xda;
        pkg[3] = 0x27;
        pkg[4..6].copy_from_slice(&20u16.to_le_bytes());
        pkg[10] = 2;
        pkg[12..16].copy_from_slice(b"CmnD");
        pkg[16] = 1;
        pkg[20] = 3;
        let toc = pkg.len();
        let n = entries.len();
        let names_start = 4 + 8 * n;
        let mut names = vec![];
        let mut name_offsets = vec![];
        for (name, _) in &entries {
            name_offsets.push((names_start + names.len()) as u32);
            names.extend_from_slice(name.as_bytes());
            names.push(0);
        }
        let mut body_start = toc + names_start + names.len();
        body_start = body_start.div_ceil(16) * 16;
        let mut data_offsets = vec![];
        let mut body = vec![];
        for (_, bytes) in &entries {
            data_offsets.push((body_start + body.len() - toc) as u32);
            body.extend_from_slice(bytes);
            body.resize(body.len().div_ceil(16) * 16, 0);
        }
        pkg.extend_from_slice(&(n as u32).to_le_bytes());
        for i in 0..n {
            pkg.extend_from_slice(&name_offsets[i].to_le_bytes());
            pkg.extend_from_slice(&data_offsets[i].to_le_bytes());
        }
        pkg.extend_from_slice(&names);
        pkg.resize(body_start, 0);
        pkg.extend_from_slice(&body);
        pkg
    }

    /// `genccode`'s C (`tools/toolutil/pkg_genc.cpp`, `writeCCode`): a
    /// `const struct { double bogus; uint8_t bytes[N]; }` named
    /// `icudt78_dat`; ICU skips the alignment-forcing double
    /// (`UDataMemory_normalizeDataPointer`).
    pub fn write_c_if_changed(path: &Path, pkg: &[u8]) {
        let mut c = String::with_capacity(pkg.len() * 5 + 512);
        write!(
            c,
            "#ifndef IN_GENERATED_CCODE\n\
             #define IN_GENERATED_CCODE\n\
             #define U_DISABLE_RENAMING 1\n\
             #include \"unicode/umachine.h\"\n\
             #endif\n\
             U_CDECL_BEGIN\n\
             const struct {{\n    double bogus;\n    uint8_t bytes[{}]; \n}} {}={{ 0.0, {{\n",
            pkg.len(),
            super::ICU_ENTRY_POINT
        )
        .unwrap();
        for (i, b) in pkg.iter().enumerate() {
            write!(c, "{b},").unwrap();
            if i % 32 == 31 {
                c.push('\n');
            }
        }
        c.push_str("\n}\n};\nU_CDECL_END\n");
        if std::fs::read_to_string(path).ok().as_deref() != Some(&c) {
            std::fs::write(path, c).unwrap();
        }
    }
}
