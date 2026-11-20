//! TeX Live 2026's pplib for the XeTeX port's PDF pictures
//! (docs/design/xetex/PLAN.md, S2): the library vendored unmodified in
//! `third_party/pplib`, compiled by `build.rs` as TeX Live compiles the
//! `libpplib.a` that XeTeX links, and XeTeX's `pdfimage.cpp` over it
//! (csrc/flashtex_pdfimage.c): [`count_pages`] and [`get_rect`].
//!
//! XeTeX asks pplib, not xpdf, whether a PDF file can be read at all:
//! `ppdoc_load` refuses a file whose `startxref` does not point at its
//! cross-reference table, a file with bytes before `%PDF-`, and others that
//! xpdf repairs (flash-tex/flashtex#2494). So `\XeTeXpdffile` and
//! `\XeTeXpdfpagecount` depend on this library; it is pinned:
//! `crates/flashtex-xetex/tests/pinned_libs.rs` fails when the README's pin,
//! any vendored file's sha256 or the library's version string changes.
//!
//! pplib keeps process-wide state (its filter heaps and its logger's
//! buffer, `util/utiliof.c` and `util/utillog.c`), and the port may run
//! several engines in one process, so every call is made under one lock.
//!
//! Licence: third_party/pplib/README.md; used under GPL-2.0-or-later.

use flashtex_xetex_teckit as _; // TeX Live's zlib, which pplib's flate filter calls

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int};
use std::sync::Mutex;

extern "C" {
    fn flashtex_pdf_get_rect(
        filename: *const c_char,
        page_num: c_int,
        pdf_box: c_int,
        r#box: *mut c_double,
    ) -> c_int;
    fn flashtex_pdf_count_pages(filename: *const c_char) -> c_int;
    fn flashtex_pplib_version() -> *const c_char;
}

static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    // a panic cannot happen while the lock is held (the calls are C)
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// `pdf_count_pages` (pdfimage.cpp): the document's page count, 0 when
/// pplib cannot load the file.
pub fn count_pages(path: &[u8]) -> i32 {
    let Ok(c) = CString::new(path) else {
        return 0;
    };
    let _g = lock();
    // SAFETY: `c` is a NUL-terminated string that outlives the call.
    unsafe { flashtex_pdf_count_pages(c.as_ptr()) }
}

/// `pdf_get_rect` (pdfimage.cpp): box `pdf_box` (XeTeX_ext.h's `pdfbox_*`)
/// of page `page_num` (clamped to the document; negative counts from the
/// end) in TeX points, as `[x, y, wd, ht]`. None for the C's -1.
pub fn get_rect(path: &[u8], page_num: i32, pdf_box: i32) -> Option<[f64; 4]> {
    let c = CString::new(path).ok()?;
    let mut r = [0.0f64; 4];
    let _g = lock();
    // SAFETY: `c` is NUL-terminated and `r` holds the four doubles the C writes.
    let rc = unsafe { flashtex_pdf_get_rect(c.as_ptr(), page_num, pdf_box, r.as_mut_ptr()) };
    (rc == 0).then_some(r)
}

/// ppapi.h's `pplib_version` of the linked library.
pub fn version() -> String {
    // SAFETY: the C returns a pointer to a string literal.
    unsafe { CStr::from_ptr(flashtex_pplib_version()) }
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// tools/xetex-lockstep/pictures: the files of lockstep case
    /// p007-pdffile-damaged (and p003's p-pages.pdf), where TeX Live's XeTeX
    /// is the oracle for what each of them gives.
    fn picture(name: &str) -> Vec<u8> {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tools/xetex-lockstep/pictures")
            .join(name);
        assert!(p.is_file(), "{} is missing", p.display());
        p.to_string_lossy().into_owned().into_bytes()
    }

    /// A sound file. XeTeX 0.999998 (TeX Live 2026): `\XeTeXpdfpagecount`
    /// 1; the page (CropBox [10 10 190 290], Rotate 90) is a box of
    /// 281.05pt x 180.675pt, its media box one of 301.125pt x 200.75pt.
    #[test]
    fn a_sound_file_gives_its_pages_and_boxes() {
        let p = picture("p-sound.pdf");
        assert_eq!(count_pages(&p), 1);
        let k = 72.27 / 72.0;
        assert_eq!(
            get_rect(&p, 1, 1),
            Some([k * 10.0, k * 10.0, k * 280.0, k * 180.0])
        );
        // the media box, of a page number past the end (clamped)
        assert_eq!(get_rect(&p, 9, 2), Some([0.0, 0.0, k * 300.0, k * 200.0]));
        assert_eq!(count_pages(&picture("p-pages.pdf")), 5);
    }

    /// flash-tex/flashtex#2494: files that xpdf repairs and pplib refuses.
    /// XeTeX 0.999998 (TeX Live 2026) reports `\XeTeXpdfpagecount` 0 and
    /// "Unable to load picture or PDF file" for each.
    #[test]
    fn pplib_refuses_what_xpdf_repairs() {
        for name in [
            "p-badstartxref.pdf",
            "p-nostartxref.pdf",
            "p-junkhead.pdf",
            "p-trailing.pdf",
        ] {
            let p = picture(name);
            assert_eq!(count_pages(&p), 0, "{name}");
            for pdf_box in 0..=5 {
                assert_eq!(get_rect(&p, 1, pdf_box), None, "{name}, box {pdf_box}");
            }
        }
        assert_eq!(count_pages(b"/nonexistent/flashtex-pplib.pdf"), 0);
        assert_eq!(get_rect(b"/nonexistent/flashtex-pplib.pdf", 1, 1), None);
    }

    #[test]
    fn the_linked_library_is_pplib_v2_2() {
        assert_eq!(version(), "v2.2");
    }
}
