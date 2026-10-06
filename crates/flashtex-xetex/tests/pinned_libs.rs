//! The shaping and metrics libraries are pinned to TeX Live 2026's versions
//! (docs/design/xetex/PLAN.md §3.1 and §5: XeTeX's output depends on the
//! exact HarfBuzz and FreeType). These tests fail when:
//!
//! * the linked HarfBuzz does not report 12.3.2 or FreeType 2.14.1;
//! * either `third_party/<lib>/README.md` pins another version;
//! * any vendored file differs from its `SHA256SUMS` entry, or a file of the
//!   vendored tree is missing from `SHA256SUMS`;
//! * a declared FFI function does not link.
//!
//! An upgrade changes all of these together, in a lane of its own with a
//! lockstep run against the TeX Live that ships the new versions.
//!
//! The smoke test shapes "office" in Latin Modern Roman the way XeTeX does
//! (an `hb_face_t` built from `FT_Load_Sfnt_Table`, XeTeXFontInst.cpp). It
//! reads the font from TeX Live, an oracle only, and skips when TeX Live is
//! absent.

use std::collections::BTreeSet;
use std::ffi::{CStr, CString};
use std::os::raw::{c_uint, c_void};
use std::path::{Path, PathBuf};

use flashtex_xetex_fontlibs::{ft, hb};
use sha2::{Digest, Sha256};

const HARFBUZZ: &str = "12.3.2";
const FREETYPE: (i32, i32, i32) = (2, 14, 1);

fn third_party() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../third_party")
}

#[test]
fn harfbuzz_runtime_version_is_pinned() {
    assert_eq!(flashtex_xetex_fontlibs::hb_version_string(), HARFBUZZ);
    let (mut major, mut minor, mut micro) = (0, 0, 0);
    unsafe { hb::hb_version(&mut major, &mut minor, &mut micro) };
    assert_eq!(format!("{major}.{minor}.{micro}"), HARFBUZZ);
}

#[test]
fn freetype_runtime_version_is_pinned() {
    assert_eq!(
        flashtex_xetex_fontlibs::ft_library_version(),
        Some(FREETYPE)
    );
    let header = (ft::FREETYPE_MAJOR, ft::FREETYPE_MINOR, ft::FREETYPE_PATCH);
    assert_eq!(header, FREETYPE, "the FFI's FREETYPE_* constants");
}

/// The README's pin table row `| Version | <version> ...`.
fn readme_version(lib: &str) -> String {
    let readme = std::fs::read_to_string(third_party().join(lib).join("README.md")).unwrap();
    let row = readme
        .lines()
        .find(|l| l.starts_with("| Version |"))
        .unwrap_or_else(|| panic!("third_party/{lib}/README.md has no `| Version |` row"));
    row.split('|')
        .nth(2)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .trim_matches('`')
        .to_string()
}

#[test]
fn readmes_pin_the_same_versions() {
    assert_eq!(readme_version("harfbuzz"), HARFBUZZ);
    let (a, b, c) = FREETYPE;
    assert_eq!(readme_version("freetype"), format!("{a}.{b}.{c}"));
}

fn files_under(dir: &Path, base: &Path, out: &mut BTreeSet<String>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            files_under(&p, base, out);
        } else {
            out.insert(
                p.strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
}

fn check_sums(lib: &str, tree: &str) {
    let base = third_party().join(lib);
    let sums = std::fs::read_to_string(base.join("SHA256SUMS")).unwrap();
    let mut listed = BTreeSet::new();
    let mut bad = Vec::new();
    for line in sums.lines().filter(|l| !l.is_empty()) {
        let (hash, path) = line
            .split_once("  ")
            .unwrap_or_else(|| panic!("bad SHA256SUMS line: {line}"));
        let bytes = std::fs::read(base.join(path)).unwrap_or_else(|e| panic!("{lib}/{path}: {e}"));
        let got: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if got != hash {
            bad.push(format!("{path}: SHA256SUMS {hash}, file {got}"));
        }
        listed.insert(path.to_string());
    }
    assert!(
        bad.is_empty(),
        "third_party/{lib}: files differ from SHA256SUMS:\n{}",
        bad.join("\n")
    );
    let mut on_disk = BTreeSet::new();
    files_under(&base.join(tree), &base, &mut on_disk);
    let unlisted: Vec<_> = on_disk.difference(&listed).collect();
    assert!(
        unlisted.is_empty(),
        "third_party/{lib}: files not in SHA256SUMS: {unlisted:?}"
    );
    assert!(
        listed.len() > 300,
        "third_party/{lib}: only {} files listed",
        listed.len()
    );
}

#[test]
fn vendored_harfbuzz_matches_sha256sums() {
    check_sums("harfbuzz", "harfbuzz-src");
}

#[test]
fn vendored_freetype_matches_sha256sums() {
    check_sums("freetype", "freetype-src");
}

#[test]
fn every_declared_function_links() {
    let all: Vec<_> = flashtex_xetex_fontlibs::hb::all_functions()
        .into_iter()
        .chain(flashtex_xetex_fontlibs::ft::all_functions())
        .collect();
    assert!(all.len() > 90, "{} functions", all.len());
    for (name, addr) in all {
        assert_ne!(addr, 0, "{name}");
    }
}

// --- smoke test: shape "office" as XeTeXFontInst does --------------------

/// XeTeXFontInst.cpp's `_get_table`: a table of the FreeType face as a blob.
unsafe extern "C" fn get_table(
    _face: *mut hb::hb_face_t,
    tag: hb::hb_tag_t,
    user_data: *mut c_void,
) -> *mut hb::hb_blob_t {
    let face = user_data as ft::FT_Face;
    let mut length: ft::FT_ULong = 0;
    if ft::FT_Load_Sfnt_Table(face, tag.into(), 0, std::ptr::null_mut(), &mut length) != 0 {
        return std::ptr::null_mut();
    }
    let mut table = Box::new(vec![0u8; length as usize]);
    if ft::FT_Load_Sfnt_Table(face, tag.into(), 0, table.as_mut_ptr(), &mut length) != 0 {
        return std::ptr::null_mut();
    }
    let (data, len) = (table.as_ptr(), table.len());
    let owner = Box::into_raw(table);
    hb::hb_blob_create(
        data.cast(),
        len as c_uint,
        hb::HB_MEMORY_MODE_WRITABLE,
        owner.cast(),
        Some(free_table),
    )
}

unsafe extern "C" fn free_table(owner: *mut c_void) {
    drop(Box::from_raw(owner.cast::<Vec<u8>>()));
}

fn find_font(name: &str) -> Option<PathBuf> {
    let out = std::process::Command::new("kpsewhich")
        .arg(name)
        .output()
        .ok()?;
    let path = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (out.status.success() && !path.is_empty()).then(|| PathBuf::from(path))
}

#[test]
fn shapes_office_with_the_ffi_ligature() {
    let Some(font) = find_font("lmroman10-regular.otf") else {
        println!("skipped: `kpsewhich lmroman10-regular.otf` found no TeX Live font (TeX Live is an oracle only)");
        return;
    };
    unsafe {
        let mut lib: ft::FT_Library = std::ptr::null_mut();
        assert_eq!(ft::FT_Init_FreeType(&mut lib), 0);
        let mut face: ft::FT_Face = std::ptr::null_mut();
        let path = CString::new(font.to_str().unwrap()).unwrap();
        assert_eq!(
            ft::FT_New_Face(lib, path.as_ptr(), 0, &mut face),
            0,
            "{}",
            font.display()
        );
        assert!(ft::FT_IS_SFNT(face) && ft::FT_IS_SCALABLE(face));
        let upem = (*face).units_per_EM;
        assert_eq!(upem, 1000);
        let name = CStr::from_ptr(ft::FT_Get_Postscript_Name(face))
            .to_str()
            .unwrap();
        assert_eq!(name, "LMRoman10-Regular");
        let os2 = ft::FT_Get_Sfnt_Table(face, ft::FT_SFNT_OS2) as *const ft::TT_OS2;
        assert!(!os2.is_null());
        assert_eq!((*os2).usWeightClass, 400);

        let hb_face = hb::hb_face_create_for_tables(Some(get_table), face as *mut c_void, None);
        hb::hb_face_set_index(hb_face, 0);
        hb::hb_face_set_upem(hb_face, upem.into());
        let hb_font = hb::hb_font_create(hb_face);
        hb::hb_face_destroy(hb_face);
        hb::hb_font_set_scale(hb_font, upem.into(), upem.into());
        hb::hb_font_set_ppem(hb_font, 0, 0);

        let text: Vec<u16> = "office".encode_utf16().collect();
        let buf = hb::hb_buffer_create();
        hb::hb_buffer_add_utf16(buf, text.as_ptr(), text.len() as i32, 0, text.len() as i32);
        hb::hb_buffer_set_direction(buf, hb::HB_DIRECTION_LTR);
        hb::hb_buffer_set_script(buf, hb::HB_SCRIPT_LATIN);
        hb::hb_buffer_guess_segment_properties(buf);
        let mut props = std::mem::zeroed::<hb::hb_segment_properties_t>();
        hb::hb_buffer_get_segment_properties(buf, &mut props);
        let ot = CString::new("ot").unwrap();
        let shapers = [ot.as_ptr(), std::ptr::null()];
        let plan = hb::hb_shape_plan_create_cached(
            hb::hb_font_get_face(hb_font),
            &props,
            std::ptr::null(),
            0,
            shapers.as_ptr(),
        );
        assert_ne!(
            hb::hb_shape_plan_execute(plan, hb_font, buf, std::ptr::null(), 0),
            0
        );
        let shaper = CStr::from_ptr(hb::hb_shape_plan_get_shaper(plan))
            .to_str()
            .unwrap()
            .to_string();
        hb::hb_shape_plan_destroy(plan);
        assert_eq!(shaper, "ot");

        let n = hb::hb_buffer_get_length(buf);
        let infos = std::slice::from_raw_parts(
            hb::hb_buffer_get_glyph_infos(buf, std::ptr::null_mut()),
            n as usize,
        );
        let glyphs: Vec<u32> = infos.iter().map(|i| i.codepoint).collect();
        let clusters: Vec<u32> = infos.iter().map(|i| i.cluster).collect();
        let ffi = ft::FT_Get_Name_Index(face, c"f_f_i".as_ptr());
        println!("office -> glyphs {glyphs:?} clusters {clusters:?} (f_f_i = {ffi})");
        assert!(n < 6, "no ligature: {n} glyphs");
        assert_ne!(ffi, 0, "the font has an f_f_i glyph");
        assert_eq!(glyphs[1], ffi, "o, ffi, c, e");
        assert_eq!(clusters, [0, 1, 4, 5]);

        hb::hb_buffer_destroy(buf);
        hb::hb_font_destroy(hb_font);
        ft::FT_Done_Face(face);
        ft::FT_Done_FreeType(lib);
    }
}
