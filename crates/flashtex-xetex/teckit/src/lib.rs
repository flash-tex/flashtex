//! TeX Live 2026's TECkit 2.5.13 conversion engine for the XeTeX port
//! (docs/design/xetex/PLAN.md §3.1): the library vendored unmodified in
//! `third_party/teckit`, compiled by `build.rs` as TeX Live compiles the
//! `libTECkit.a` that XeTeX links, with TeX Live's zlib for compressed
//! mappings, and raw `extern "C"` declarations of what XeTeX_ext.c calls,
//! under their C names (`source/Public-headers/TECkit_Engine.h`,
//! `TECkit_Common.h`).
//!
//! XeTeX applies a font's `mapping=` (a `.tec` file, such as `tex-text.tec`)
//! with this engine, so its output depends on it; it is pinned:
//! `crates/flashtex-xetex/tests/pinned_libs.rs` fails when the README's pin,
//! any vendored file's sha256 or the engine's reported version changes.
//!
//! Licence: TECkit is CPL-0.5-or-later OR LGPL-2.1-or-later; FlashTeX uses
//! it under the LGPL-2.1-or-later, and links it only into the GPL engine.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

use std::os::raw::c_long;

pub type Byte = u8;
pub type UInt16 = u16;
pub type UInt32 = u32;

/// `typedef long TECkit_Status` (TECkit_Common.h): 64 bits on LP64 hosts,
/// 32 on Windows, as C's `long`.
pub type TECkit_Status = c_long;

/// `struct Opaque_TECkit_Converter`: never dereferenced.
#[repr(C)]
pub struct Opaque_TECkit_Converter {
    _private: [u8; 0],
}

/// `typedef struct Opaque_TECkit_Converter* TECkit_Converter`.
pub type TECkit_Converter = *mut Opaque_TECkit_Converter;

// TECkit_Common.h: status values.
pub const kStatus_NoError: TECkit_Status = 0;
pub const kStatus_OutputBufferFull: TECkit_Status = 1;
pub const kStatus_NeedMoreInput: TECkit_Status = 2;
pub const kStatus_InvalidForm: TECkit_Status = -1;
pub const kStatus_InvalidConverter: TECkit_Status = -3;
pub const kStatus_InvalidMapping: TECkit_Status = -4;
pub const kStatus_BadMappingVersion: TECkit_Status = -5;

// TECkit_Common.h: encoding forms.
pub const kForm_Bytes: UInt16 = 1;
pub const kForm_UTF8: UInt16 = 2;
pub const kForm_UTF16BE: UInt16 = 3;
pub const kForm_UTF16LE: UInt16 = 4;
pub const kForm_UTF32BE: UInt16 = 5;
pub const kForm_UTF32LE: UInt16 = 6;
// TECkit_Common.h: normalization forms, or'ed into an encoding form.
pub const kForm_NFC: UInt16 = 0x0100;
pub const kForm_NFD: UInt16 = 0x0200;

/// XeTeX_ext.c's `UTF16_NATIVE`: UTF-16 in the host's byte order
/// (`kForm_UTF16BE` under `WORDS_BIGENDIAN`).
#[cfg(target_endian = "little")]
pub const UTF16_NATIVE: UInt16 = kForm_UTF16LE;
#[cfg(target_endian = "big")]
pub const UTF16_NATIVE: UInt16 = kForm_UTF16BE;

/// XeTeX_ext.c's `NATIVE_UTF32`: UTF-32 in the host's byte order.
#[cfg(target_endian = "little")]
pub const NATIVE_UTF32: UInt16 = kForm_UTF32LE;
#[cfg(target_endian = "big")]
pub const NATIVE_UTF32: UInt16 = kForm_UTF32BE;

/// TECkit_Common.h's `kCurrentTECkitVersion` (16.16), which
/// `TECkit_GetVersion` returns: the engine's API version, 2.4, unchanged
/// since 2006 and the same in release 2.5.13.
pub const kCurrentTECkitVersion: UInt32 = 0x0002_0004;

/// Declares `extern "C"` functions and, for the link test, a table of their
/// names and addresses (which forces every declared symbol to resolve).
macro_rules! ffi_functions {
    ($( pub fn $name:ident ( $($arg:ident : $ty:ty),* $(,)? ) $(-> $ret:ty)? ; )*) => {
        extern "C" {
            $( pub fn $name($($arg: $ty),*) $(-> $ret)?; )*
        }

        /// Every function this crate declares, by name, with its address.
        #[doc(hidden)]
        pub fn all_functions() -> Vec<(&'static str, usize)> {
            vec![$( (stringify!($name), $name as *const () as usize) ),*]
        }
    };
}

ffi_functions! {
    pub fn TECkit_CreateConverter(
        mapping: *mut Byte,
        mappingSize: UInt32,
        mapForward: Byte,
        sourceForm: UInt16,
        targetForm: UInt16,
        converter: *mut TECkit_Converter,
    ) -> TECkit_Status;
    pub fn TECkit_DisposeConverter(converter: TECkit_Converter) -> TECkit_Status;
    pub fn TECkit_ResetConverter(converter: TECkit_Converter) -> TECkit_Status;
    pub fn TECkit_ConvertBuffer(
        converter: TECkit_Converter,
        inBuffer: *const Byte,
        inLength: UInt32,
        inUsed: *mut UInt32,
        outBuffer: *mut Byte,
        outLength: UInt32,
        outUsed: *mut UInt32,
        inputIsComplete: Byte,
    ) -> TECkit_Status;
    pub fn TECkit_GetVersion() -> UInt32;
}
