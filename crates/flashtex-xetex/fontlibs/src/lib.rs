//! TeX Live 2026's HarfBuzz 12.3.2 and FreeType 2.14.1 for the XeTeX port
//! (docs/design/xetex/PLAN.md §3.1): the libraries vendored unmodified in
//! `third_party/harfbuzz` and `third_party/freetype`, compiled by `build.rs`
//! with TeX Live's configuration, and raw `extern "C"` declarations of what
//! XeTeX's C/C++ layer calls ([`hb`], [`ft`]), under their C names.
//!
//! XeTeX's output depends on these exact versions (shaping and glyph
//! metrics), so they are pinned: `crates/flashtex-xetex/tests/pinned_libs.rs`
//! fails when the runtime versions, the READMEs' pins or any vendored file's
//! sha256 change. Upgrading is a lane of its own, with a lockstep run against
//! the TeX Live that ships the new versions.
//!
//! Licences: HarfBuzz is MIT ("Old MIT"); FreeType is used under its
//! GPL-2.0-or-later option; this crate is GPL-2.0-or-later. See the two
//! READMEs under `third_party/`.

use std::ffi::CStr;
use std::os::raw::{c_char, c_longlong};

/// Declares `extern "C"` functions and, for the link test, a table of their
/// names and addresses (which forces every declared symbol to resolve).
#[macro_export]
#[doc(hidden)]
macro_rules! ffi_functions {
    ($( pub fn $name:ident ( $($arg:ident : $ty:ty),* $(,)? ) $(-> $ret:ty)? ; )*) => {
        extern "C" {
            $( pub fn $name($($arg: $ty),*) $(-> $ret)?; )*
        }

        /// Every function this module declares, by name, with its address.
        #[doc(hidden)]
        pub fn all_functions() -> Vec<(&'static str, usize)> {
            vec![$( (stringify!($name), $name as *const () as usize) ),*]
        }
    };
}

pub mod ft;
pub mod hb;

/// HarfBuzz's version as the library reports it (`hb_version_string`).
pub fn hb_version_string() -> String {
    // SAFETY: hb_version_string returns a static NUL-terminated string.
    unsafe { CStr::from_ptr(hb::hb_version_string()) }
        .to_string_lossy()
        .into_owned()
}

/// FreeType's version as `FT_Library_Version` reports it, from a library
/// created and destroyed for the call. `None` if FreeType fails to start.
pub fn ft_library_version() -> Option<(i32, i32, i32)> {
    let mut lib: ft::FT_Library = std::ptr::null_mut();
    // SAFETY: plain FreeType calls on a library this function owns.
    unsafe {
        if ft::FT_Init_FreeType(&mut lib) != 0 {
            return None;
        }
        let (mut major, mut minor, mut patch) = (0, 0, 0);
        ft::FT_Library_Version(lib, &mut major, &mut minor, &mut patch);
        ft::FT_Done_FreeType(lib);
        Some((major, minor, patch))
    }
}

#[repr(C)]
struct ProbeEntry {
    name: *const c_char,
    value: c_longlong,
}

extern "C" {
    fn flashtex_fontlibs_layout_probe(count: *mut usize) -> *const ProbeEntry;
}

/// What the C compiler says about the structs and constants the FFI
/// declares (`csrc/layout_probe.c`): `("sizeof T", n)`, `("alignof T", n)`,
/// `("T.field", offset)` and `("CONSTANT", value)`.
#[doc(hidden)]
pub fn layout_probe() -> Vec<(String, i64)> {
    let mut n = 0usize;
    // SAFETY: the probe returns a static array of `n` entries whose names
    // are static NUL-terminated strings.
    unsafe {
        let entries = flashtex_fontlibs_layout_probe(&mut n);
        std::slice::from_raw_parts(entries, n)
            .iter()
            .map(|e| {
                (
                    CStr::from_ptr(e.name).to_string_lossy().into_owned(),
                    e.value,
                )
            })
            .collect()
    }
}
