//! TeX Live 2026's ICU 78.2 for the XeTeX port (docs/design/xetex/PLAN.md
//! §3.1): the common library vendored as TeX Live has it in `third_party/icu`,
//! compiled by `build.rs` with TeX Live's configuration, with the items of
//! TeX Live's ICU data that XeTeX's uses need (the converter alias table and
//! the break-iteration data; `third_party/icu/README.md`), and raw
//! `extern "C"` declarations of what `XeTeX_ext.c` calls, under their C
//! names. ICU renames its functions with the version (`ubidi_open` links
//! as `ubidi_open_78`, `unicode/urename.h`), as in TeX Live's build.
//!
//! What XeTeX uses ICU for:
//!
//! * bidirectional analysis of native words (`measure_native_node`:
//!   [`ubidi_open`], [`ubidi_setPara`], [`ubidi_getDirection`],
//!   [`ubidi_countRuns`], [`ubidi_getVisualRun`]), from the Unicode data
//!   compiled into the library;
//! * input encodings other than UTF-8, UTF-16 and bytes
//!   (`\XeTeXinputencoding`: [`ucnv_open`], [`ucnv_toAlgorithmic`]);
//! * `\XeTeXlinebreaklocale` ([`ubrk_open`], [`ubrk_setText`],
//!   [`ubrk_next`]);
//! * the version line ([`u_getVersion`]).
//!
//! XeTeX's input normalisation (`\XeTeXinputnormalization`) is TECkit's,
//! not ICU's (`apply_normalization` in XeTeX_ext.c).
//!
//! The version is pinned: `crates/flashtex-xetex/tests/pinned_libs.rs`
//! fails when the runtime version, the README's pin or any vendored file's
//! sha256 changes.

use std::ffi::{c_char, CStr};

/// ICU's `UErrorCode`; zero or less is success (negative: a warning).
pub type UErrorCode = i32;
pub const U_ZERO_ERROR: UErrorCode = 0;
pub const U_FILE_ACCESS_ERROR: UErrorCode = 4;
pub const U_STRING_NOT_TERMINATED_WARNING: UErrorCode = -124;

pub type UChar = u16;
pub type UBiDiLevel = u8;
pub const UBIDI_DEFAULT_LTR: UBiDiLevel = 0xfe;
pub const UBIDI_DEFAULT_RTL: UBiDiLevel = 0xff;

/// `UBiDiDirection`.
pub type UBiDiDirection = i32;
pub const UBIDI_LTR: UBiDiDirection = 0;
pub const UBIDI_RTL: UBiDiDirection = 1;
pub const UBIDI_MIXED: UBiDiDirection = 2;
pub const UBIDI_NEUTRAL: UBiDiDirection = 3;

/// `UConverterType`.
pub type UConverterType = i32;
pub const UCNV_UTF32_BIG_ENDIAN: UConverterType = 7;
pub const UCNV_UTF32_LITTLE_ENDIAN: UConverterType = 8;
/// XeTeX_ext.c's `UCNV_UTF32_NativeEndian`.
#[cfg(target_endian = "little")]
pub const UCNV_UTF32_NATIVE_ENDIAN: UConverterType = UCNV_UTF32_LITTLE_ENDIAN;
#[cfg(target_endian = "big")]
pub const UCNV_UTF32_NATIVE_ENDIAN: UConverterType = UCNV_UTF32_BIG_ENDIAN;

/// `UBreakIteratorType`.
pub type UBreakIteratorType = i32;
pub const UBRK_LINE: UBreakIteratorType = 2;
pub const UBRK_DONE: i32 = -1;

macro_rules! opaque {
    ($($name:ident),* $(,)?) => {$(
        #[repr(C)]
        pub struct $name {
            _private: [u8; 0],
            _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
        }
    )*};
}
opaque!(UBiDi, UConverter, UBreakIterator);

/// Declares ICU functions under their C names, linked by ICU's renamed
/// (`_78`-suffixed) symbols, and a table of their names and addresses for
/// the link test.
macro_rules! icu_functions {
    ($( pub fn $name:ident ( $($arg:ident : $ty:ty),* $(,)? ) $(-> $ret:ty)? ; )*) => {
        extern "C" {
            $(
                #[link_name = concat!(stringify!($name), "_78")]
                pub fn $name($($arg: $ty),*) $(-> $ret)?;
            )*
        }

        /// Every function this crate declares, by C name, with its address.
        #[doc(hidden)]
        pub fn all_functions() -> Vec<(&'static str, usize)> {
            vec![$( (stringify!($name), $name as *const () as usize) ),*]
        }
    };
}

icu_functions! {
    pub fn u_getVersion(version_array: *mut u8);
    pub fn u_errorName(code: UErrorCode) -> *const c_char;

    pub fn ubidi_open() -> *mut UBiDi;
    pub fn ubidi_close(bidi: *mut UBiDi);
    pub fn ubidi_setPara(
        bidi: *mut UBiDi,
        text: *const UChar,
        length: i32,
        para_level: UBiDiLevel,
        embedding_levels: *mut UBiDiLevel,
        error: *mut UErrorCode,
    );
    pub fn ubidi_getDirection(bidi: *const UBiDi) -> UBiDiDirection;
    pub fn ubidi_countRuns(bidi: *mut UBiDi, error: *mut UErrorCode) -> i32;
    pub fn ubidi_getVisualRun(
        bidi: *mut UBiDi,
        run_index: i32,
        logical_start: *mut i32,
        length: *mut i32,
    ) -> UBiDiDirection;

    pub fn ucnv_open(name: *const c_char, error: *mut UErrorCode) -> *mut UConverter;
    pub fn ucnv_close(cnv: *mut UConverter);
    pub fn ucnv_toAlgorithmic(
        algorithmic_type: UConverterType,
        cnv: *mut UConverter,
        target: *mut c_char,
        target_capacity: i32,
        source: *const c_char,
        source_length: i32,
        error: *mut UErrorCode,
    ) -> i32;

    pub fn ubrk_open(
        kind: UBreakIteratorType,
        locale: *const c_char,
        text: *const UChar,
        text_length: i32,
        status: *mut UErrorCode,
    ) -> *mut UBreakIterator;
    pub fn ubrk_close(bi: *mut UBreakIterator);
    pub fn ubrk_setText(
        bi: *mut UBreakIterator,
        text: *const UChar,
        text_length: i32,
        status: *mut UErrorCode,
    );
    pub fn ubrk_next(bi: *mut UBreakIterator) -> i32;
}

/// `U_FAILURE`.
pub fn failure(code: UErrorCode) -> bool {
    code > U_ZERO_ERROR
}

/// ICU's version as the library reports it (`u_getVersion` and
/// `u_versionToString`: trailing zero fields after the second dropped).
pub fn version_string() -> String {
    let mut v = [0u8; 4];
    // SAFETY: u_getVersion writes the four bytes of a UVersionInfo.
    unsafe { u_getVersion(v.as_mut_ptr()) };
    let mut n = 4;
    while n > 2 && v[n - 1] == 0 {
        n -= 1;
    }
    v[..n]
        .iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join(".")
}

/// The name of an error code (`u_errorName`).
pub fn error_name(code: UErrorCode) -> String {
    // SAFETY: u_errorName returns a static string.
    unsafe { CStr::from_ptr(u_errorName(code)) }
        .to_string_lossy()
        .into_owned()
}

/// An open `UConverter`, closed when dropped (`ucnv_close`).
pub struct Converter(*mut UConverter);

impl Converter {
    /// `ucnv_open`: the converter, or the error code.
    pub fn open(name: &CStr) -> Result<Converter, UErrorCode> {
        let mut err = U_ZERO_ERROR;
        // SAFETY: a NUL-terminated name and an error code.
        let cnv = unsafe { ucnv_open(name.as_ptr(), &mut err) };
        if cnv.is_null() {
            Err(err)
        } else {
            Ok(Converter(cnv))
        }
    }

    /// `ucnv_toAlgorithmic(UCNV_UTF32_NativeEndian, cnv, target,
    /// target.len(), source, ...)`: the number of bytes written and the
    /// error code, which XeTeX treats as a failure unless it is zero.
    pub fn to_utf32_native(&mut self, target: &mut [u8], source: &[u8]) -> (i32, UErrorCode) {
        let mut err = U_ZERO_ERROR;
        // SAFETY: both buffers are valid for the lengths passed.
        let n = unsafe {
            ucnv_toAlgorithmic(
                UCNV_UTF32_NATIVE_ENDIAN,
                self.0,
                target.as_mut_ptr().cast(),
                i32::try_from(target.len()).unwrap_or(i32::MAX),
                source.as_ptr().cast(),
                i32::try_from(source.len()).unwrap_or(i32::MAX),
                &mut err,
            )
        };
        (n, err)
    }
}

impl Drop for Converter {
    fn drop(&mut self) {
        // SAFETY: the converter was opened by `open` and is closed once.
        unsafe { ucnv_close(self.0) }
    }
}

/// A line-break iterator (`ubrk_open(UBRK_LINE, ...)`), closed when
/// dropped, with the text it iterates over (`ubrk_setText` does not copy).
pub struct LineBreaker {
    bi: *mut UBreakIterator,
    text: Vec<UChar>,
}

impl LineBreaker {
    /// `ubrk_open(UBRK_LINE, locale, NULL, 0, &status)`: the iterator, or
    /// the failing status. A warning (such as `U_USING_DEFAULT_WARNING`)
    /// is not a failure.
    pub fn open(locale: &CStr) -> Result<LineBreaker, UErrorCode> {
        let mut status = U_ZERO_ERROR;
        // SAFETY: a NUL-terminated locale, no text yet.
        let bi = unsafe { ubrk_open(UBRK_LINE, locale.as_ptr(), std::ptr::null(), 0, &mut status) };
        if failure(status) {
            if !bi.is_null() {
                // SAFETY: as returned by ubrk_open.
                unsafe { ubrk_close(bi) };
            }
            return Err(status);
        }
        if bi.is_null() {
            return Err(status);
        }
        Ok(LineBreaker { bi, text: vec![] })
    }

    /// `ubrk_setText`, keeping a copy of the text for the iterator.
    pub fn set_text(&mut self, text: &[UChar]) -> UErrorCode {
        self.text = text.to_vec();
        let mut status = U_ZERO_ERROR;
        // SAFETY: the text lives in `self` until the next set_text or drop.
        unsafe {
            ubrk_setText(
                self.bi,
                self.text.as_ptr(),
                self.text.len() as i32,
                &mut status,
            )
        };
        status
    }

    /// `ubrk_next`: the next boundary, or [`UBRK_DONE`].
    pub fn next_boundary(&mut self) -> i32 {
        // SAFETY: an open iterator over `self.text`.
        unsafe { ubrk_next(self.bi) }
    }
}

impl Drop for LineBreaker {
    fn drop(&mut self) {
        // SAFETY: opened by `open`, closed once.
        unsafe { ubrk_close(self.bi) }
    }
}

/// A `UBiDi` object, closed when dropped (`ubidi_close`), with the text of
/// its paragraph (`ubidi_setPara` does not copy).
pub struct Bidi {
    bidi: *mut UBiDi,
    text: Vec<UChar>,
}

impl Bidi {
    /// `ubidi_open` then `ubidi_setPara(text, para_level, NULL)`; the
    /// error code of `ubidi_setPara` is returned with it.
    pub fn new(text: &[UChar], para_level: UBiDiLevel) -> (Bidi, UErrorCode) {
        // SAFETY: ubidi_open allocates; a null result is checked by
        // ubidi_setPara (U_ILLEGAL_ARGUMENT_ERROR).
        let bidi = unsafe { ubidi_open() };
        assert!(!bidi.is_null(), "ubidi_open: out of memory");
        let b = Bidi {
            bidi,
            text: text.to_vec(),
        };
        let mut err = U_ZERO_ERROR;
        // SAFETY: the text lives in `b` as long as the object.
        unsafe {
            ubidi_setPara(
                b.bidi,
                b.text.as_ptr(),
                b.text.len() as i32,
                para_level,
                std::ptr::null_mut(),
                &mut err,
            )
        };
        (b, err)
    }

    /// `ubidi_getDirection`.
    pub fn direction(&self) -> UBiDiDirection {
        // SAFETY: an open object.
        unsafe { ubidi_getDirection(self.bidi) }
    }

    /// `ubidi_countRuns`, with its error code passed through.
    pub fn count_runs(&mut self, err: &mut UErrorCode) -> i32 {
        // SAFETY: an open object.
        unsafe { ubidi_countRuns(self.bidi, err) }
    }

    /// `ubidi_getVisualRun`: `(direction, logical start, length)`.
    pub fn visual_run(&mut self, run: i32) -> (UBiDiDirection, i32, i32) {
        let (mut start, mut length) = (0, 0);
        // SAFETY: an open object; out-parameters are local.
        let dir = unsafe { ubidi_getVisualRun(self.bidi, run, &mut start, &mut length) };
        (dir, start, length)
    }
}

impl Drop for Bidi {
    fn drop(&mut self) {
        // SAFETY: opened by `new`, closed once.
        unsafe { ubidi_close(self.bidi) }
    }
}

#[repr(C)]
struct ProbeEntry {
    name: *const c_char,
    value: std::ffi::c_longlong,
}

extern "C" {
    fn flashtex_icu_probe(count: *mut usize) -> *const ProbeEntry;
    fn flashtex_icu_renamed_ubidi_open() -> *const c_char;
}

/// What the C compiler says about the types and constants the FFI declares
/// (`csrc/probe.c`), and the renamed name of `ubidi_open`.
#[doc(hidden)]
pub fn probe() -> (Vec<(String, i64)>, String) {
    let mut n = 0usize;
    // SAFETY: the probe returns a static array of `n` entries and a static
    // string.
    unsafe {
        let p = flashtex_icu_probe(&mut n);
        let entries = std::slice::from_raw_parts(p, n)
            .iter()
            .map(|e| {
                (
                    CStr::from_ptr(e.name).to_string_lossy().into_owned(),
                    e.value,
                )
            })
            .collect();
        let renamed = CStr::from_ptr(flashtex_icu_renamed_ubidi_open())
            .to_string_lossy()
            .into_owned();
        (entries, renamed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_constants_match_the_headers() {
        let (entries, renamed) = probe();
        let get = |k: &str| entries.iter().find(|e| e.0 == k).unwrap().1;
        assert_eq!(renamed, "ubidi_open_78");
        assert_eq!(get("sizeof UChar"), 2);
        assert_eq!(get("sizeof UErrorCode"), 4);
        assert_eq!(get("sizeof UBiDiLevel"), 1);
        assert_eq!(get("sizeof UBiDiDirection"), 4);
        assert_eq!(get("sizeof UConverterType"), 4);
        assert_eq!(get("sizeof UBreakIteratorType"), 4);
        assert_eq!(get("sizeof UVersionInfo"), 4);
        assert_eq!(get("U_ZERO_ERROR"), U_ZERO_ERROR as i64);
        assert_eq!(get("U_FILE_ACCESS_ERROR"), U_FILE_ACCESS_ERROR as i64);
        assert_eq!(
            get("U_STRING_NOT_TERMINATED_WARNING"),
            U_STRING_NOT_TERMINATED_WARNING as i64
        );
        assert_eq!(get("UBIDI_DEFAULT_LTR"), UBIDI_DEFAULT_LTR as i64);
        assert_eq!(get("UBIDI_DEFAULT_RTL"), UBIDI_DEFAULT_RTL as i64);
        assert_eq!(get("UBIDI_LTR"), UBIDI_LTR as i64);
        assert_eq!(get("UBIDI_RTL"), UBIDI_RTL as i64);
        assert_eq!(get("UBIDI_MIXED"), UBIDI_MIXED as i64);
        assert_eq!(get("UBIDI_NEUTRAL"), UBIDI_NEUTRAL as i64);
        assert_eq!(get("UCNV_UTF32_BigEndian"), UCNV_UTF32_BIG_ENDIAN as i64);
        assert_eq!(
            get("UCNV_UTF32_LittleEndian"),
            UCNV_UTF32_LITTLE_ENDIAN as i64
        );
        assert_eq!(get("UBRK_LINE"), UBRK_LINE as i64);
        assert_eq!(get("UBRK_DONE"), UBRK_DONE as i64);
        let version = format!(
            "{}.{}",
            get("U_ICU_VERSION_MAJOR_NUM"),
            get("U_ICU_VERSION_MINOR_NUM")
        );
        assert_eq!(version, "78.2");
    }

    #[test]
    fn bidi_of_mixed_text() {
        // "abc " then two Hebrew letters: an LTR run and an RTL run.
        let text: Vec<u16> = "abc \u{05D0}\u{05D1}".encode_utf16().collect();
        let (mut b, err) = Bidi::new(&text, UBIDI_DEFAULT_LTR);
        assert_eq!(err, U_ZERO_ERROR);
        assert_eq!(b.direction(), UBIDI_MIXED);
        let mut err = U_ZERO_ERROR;
        assert_eq!(b.count_runs(&mut err), 2);
        assert_eq!(b.visual_run(0), (UBIDI_LTR, 0, 4));
        assert_eq!(b.visual_run(1), (UBIDI_RTL, 4, 2));
        let (b, _) = Bidi::new(&text[4..], UBIDI_DEFAULT_LTR);
        assert_eq!(b.direction(), UBIDI_RTL);
    }

    #[test]
    fn algorithmic_converters_by_alias() {
        for name in [
            c"latin1",
            c"iso-8859-1",
            c"ISO_8859-1",
            c"ascii",
            c"US-ASCII",
            c"utf-8",
        ] {
            let mut cnv = Converter::open(name).unwrap_or_else(|e| {
                panic!("{name:?}: {}", error_name(e));
            });
            let mut out = [0u8; 16];
            let (n, err) = cnv.to_utf32_native(&mut out, b"Az");
            assert_eq!((n, err), (8, U_ZERO_ERROR), "{name:?}");
        }
        let mut latin1 = Converter::open(c"latin1").unwrap();
        let mut out = [0u8; 8];
        let (n, err) = latin1.to_utf32_native(&mut out, b"\xe9");
        assert_eq!((n, err), (4, U_ZERO_ERROR));
        assert_eq!(u32::from_ne_bytes(out[..4].try_into().unwrap()), 0xe9);
        // Output that fills the target exactly leaves no room for the
        // terminating NUL: a warning, which XeTeX treats as an error.
        let mut out = [0u8; 4];
        let (n, err) = latin1.to_utf32_native(&mut out, b"\xe9");
        assert_eq!((n, err), (4, U_STRING_NOT_TERMINATED_WARNING));
        assert_eq!(u32::from_ne_bytes(out[..4].try_into().unwrap()), 0xe9);
    }

    #[test]
    fn data_converters_are_not_in_the_package() {
        // The alias resolves (cnvalias.icu), the .cnv table does not load.
        for name in [c"latin2", c"cp1252", c"macintosh"] {
            assert_eq!(
                Converter::open(name).err(),
                Some(U_FILE_ACCESS_ERROR),
                "{name:?}"
            );
        }
        assert!(Converter::open(c"no-such-encoding").is_err());
    }

    #[test]
    fn thai_line_breaks_use_the_dictionary() {
        // Two Thai words, "ภาษาไทย" ("Thai language") then "ง่าย" ("easy"):
        // a break between them needs thaidict.dict.
        let text: Vec<u16> = "ภาษาไทยง่าย".encode_utf16().collect();
        let mut lb = LineBreaker::open(c"th").unwrap();
        assert_eq!(lb.set_text(&text), U_ZERO_ERROR);
        let mut breaks = vec![];
        loop {
            let b = lb.next_boundary();
            if b == UBRK_DONE {
                break;
            }
            breaks.push(b);
        }
        assert!(breaks.len() >= 2, "{breaks:?}");
        assert_eq!(*breaks.last().unwrap(), text.len() as i32);
    }

    #[test]
    fn version_is_78_2() {
        assert_eq!(version_string(), "78.2");
    }
}
