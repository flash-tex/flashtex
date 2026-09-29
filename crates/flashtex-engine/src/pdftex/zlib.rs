//! TeX Live's zlib 1.3.2 (third_party/zlib), linked unmodified: the few
//! calls pdfTeX's `writezip.c` makes.
//!
//! build.rs compiles zlib with `Z_PREFIX`, so its symbols are `z_*`, and
//! sets `cfg(flashtex_zlib)` when it has.

use std::ffi::{c_char, c_int, c_uint, c_ulong, c_void};

pub const Z_OK: c_int = 0;
pub const Z_STREAM_END: c_int = 1;
pub const Z_NO_FLUSH: c_int = 0;
pub const Z_FINISH: c_int = 4;
const ZLIB_VERSION: &std::ffi::CStr = c"1.3.2";

/// zlib.h's `z_stream`.
#[repr(C)]
pub struct ZStream {
    pub next_in: *const u8,
    pub avail_in: c_uint,
    pub total_in: c_ulong,
    pub next_out: *mut u8,
    pub avail_out: c_uint,
    pub total_out: c_ulong,
    pub msg: *const c_char,
    state: *mut c_void,
    zalloc: *const c_void,
    zfree: *const c_void,
    opaque: *mut c_void,
    pub data_type: c_int,
    pub adler: c_ulong,
    reserved: c_ulong,
}

#[cfg(flashtex_zlib)]
extern "C" {
    fn z_deflateInit_(
        strm: *mut ZStream,
        level: c_int,
        version: *const c_char,
        size: c_int,
    ) -> c_int;
    fn z_deflate(strm: *mut ZStream, flush: c_int) -> c_int;
    fn z_deflateReset(strm: *mut ZStream) -> c_int;
    fn z_deflateEnd(strm: *mut ZStream) -> c_int;
    fn z_zlibVersion() -> *const c_char;
}

/// Without build.rs (scripts/flashtex-etrip.sh builds the engine as a scratch
/// package in DVI mode, where nothing is compressed) zlib is not linked, and
/// these stand in for it.
#[cfg(not(flashtex_zlib))]
#[allow(non_snake_case)]
mod unlinked {
    use super::*;
    fn missing() -> ! {
        panic!("TeX Live's zlib is not linked into this build (see build.rs)")
    }
    pub unsafe fn z_deflateInit_(_: *mut ZStream, _: c_int, _: *const c_char, _: c_int) -> c_int {
        missing()
    }
    pub unsafe fn z_deflate(_: *mut ZStream, _: c_int) -> c_int {
        missing()
    }
    pub unsafe fn z_deflateReset(_: *mut ZStream) -> c_int {
        missing()
    }
    pub unsafe fn z_deflateEnd(_: *mut ZStream) -> c_int {
        missing()
    }
    pub unsafe fn z_zlibVersion() -> *const c_char {
        missing()
    }
}
#[cfg(not(flashtex_zlib))]
use unlinked::*;

impl ZStream {
    /// A stream with `zalloc`, `zfree` and `opaque` null (zlib's defaults),
    /// as `writezip` sets them before `deflateInit`. Boxed: zlib keeps a
    /// pointer back to it, so it must not move.
    pub fn new() -> Box<ZStream> {
        Box::new(ZStream {
            next_in: std::ptr::null(),
            avail_in: 0,
            total_in: 0,
            next_out: std::ptr::null_mut(),
            avail_out: 0,
            total_out: 0,
            msg: std::ptr::null(),
            state: std::ptr::null_mut(),
            zalloc: std::ptr::null(),
            zfree: std::ptr::null(),
            opaque: std::ptr::null_mut(),
            data_type: 0,
            adler: 0,
            reserved: 0,
        })
    }

    /// `deflateInit(strm, level)`.
    pub fn deflate_init(&mut self, level: i32) -> c_int {
        self.zalloc = std::ptr::null();
        self.zfree = std::ptr::null();
        self.opaque = std::ptr::null_mut();
        // SAFETY: `self` is a boxed, pinned z_stream laid out as zlib.h's.
        unsafe {
            z_deflateInit_(
                self,
                level,
                ZLIB_VERSION.as_ptr(),
                std::mem::size_of::<ZStream>() as c_int,
            )
        }
    }

    /// `deflate(strm, flush)`. The caller has set `next_in`/`next_out` to
    /// buffers that stay valid for the call.
    pub fn deflate(&mut self, flush: c_int) -> c_int {
        // SAFETY: see above.
        unsafe { z_deflate(self, flush) }
    }

    /// `deflateReset(strm)`.
    pub fn deflate_reset(&mut self) -> c_int {
        // SAFETY: an initialised stream.
        unsafe { z_deflateReset(self) }
    }

    /// `deflateEnd(strm)`.
    pub fn deflate_end(&mut self) -> c_int {
        // SAFETY: an initialised stream.
        unsafe { z_deflateEnd(self) }
    }
}

/// `zlibVersion()`: the version of the linked library.
pub fn version() -> String {
    // SAFETY: zlib returns a static NUL-terminated string.
    unsafe { std::ffi::CStr::from_ptr(z_zlibVersion()) }
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linked_zlib_is_tex_lives() {
        assert_eq!(version(), "1.3.2");
    }

    #[test]
    fn deflate_round_trip_size() {
        let input = b"hello hello hello hello hello hello";
        let mut out = vec![0u8; 256];
        let mut z = ZStream::new();
        assert_eq!(z.deflate_init(9), Z_OK);
        z.next_in = input.as_ptr();
        z.avail_in = input.len() as c_uint;
        z.next_out = out.as_mut_ptr();
        z.avail_out = out.len() as c_uint;
        assert_eq!(z.deflate(Z_FINISH), Z_STREAM_END);
        let n = z.total_out as usize;
        assert_eq!(z.deflate_end(), Z_OK);
        // zlib header for level 9: 0x78 0xDA.
        assert_eq!(&out[..2], &[0x78, 0xDA]);
        assert!(n < input.len());
    }
}
