//! gzip through TeX Live's zlib, which the engine already links for the PDF
//! writer (third_party/zlib, `Z_PREFIX`; src/pdftex/zlib.rs): the bundle's
//! members are gzip streams (Tectonic's TTBv1 format), and a second
//! compression library would add nothing.

use crate::pdftex::zlib::{ZStream, Z_FINISH, Z_NO_FLUSH, Z_OK, Z_STREAM_END};
use std::ffi::{c_char, c_int, c_uint};

const ZLIB_VERSION: &std::ffi::CStr = c"1.3.2";
const Z_BUF_ERROR: c_int = -5;

extern "C" {
    #[allow(clippy::too_many_arguments)]
    fn z_deflateInit2_(
        strm: *mut ZStream,
        level: c_int,
        method: c_int,
        window_bits: c_int,
        mem_level: c_int,
        strategy: c_int,
        version: *const c_char,
        size: c_int,
    ) -> c_int;
    fn z_deflate(strm: *mut ZStream, flush: c_int) -> c_int;
    fn z_deflateEnd(strm: *mut ZStream) -> c_int;
    fn z_inflateInit2_(
        strm: *mut ZStream,
        window_bits: c_int,
        version: *const c_char,
        size: c_int,
    ) -> c_int;
    fn z_inflate(strm: *mut ZStream, flush: c_int) -> c_int;
    fn z_inflateEnd(strm: *mut ZStream) -> c_int;
}

/// One gzip member (RFC 1952) of `data`, as `gzip -LEVEL` makes it:
/// `deflateInit2` with window bits 15 + 16.
pub fn gzip(data: &[u8], level: i32) -> Vec<u8> {
    let mut z = ZStream::new();
    let mut out = Vec::with_capacity(data.len() / 2 + 64);
    let mut buf = vec![0u8; 1 << 16];
    // SAFETY: `z` is a boxed z_stream that does not move; `data` and `buf`
    // outlive every call that points at them.
    unsafe {
        let r = z_deflateInit2_(
            &mut *z,
            level,
            8,
            15 + 16,
            8,
            0,
            ZLIB_VERSION.as_ptr(),
            std::mem::size_of::<ZStream>() as c_int,
        );
        assert_eq!(r, Z_OK, "deflateInit2");
        z.next_in = data.as_ptr();
        z.avail_in = data.len() as c_uint;
        loop {
            z.next_out = buf.as_mut_ptr();
            z.avail_out = buf.len() as c_uint;
            let r = z_deflate(&mut *z, Z_FINISH);
            out.extend_from_slice(&buf[..buf.len() - z.avail_out as usize]);
            if r == Z_STREAM_END {
                break;
            }
            assert!(r == Z_OK || r == Z_BUF_ERROR, "deflate: {r}");
        }
        z_deflateEnd(&mut *z);
    }
    out
}

/// Decompress one gzip member (window bits 15 + 32 also accept a zlib
/// stream). `size_hint` is the expected size.
pub fn gunzip(data: &[u8], size_hint: usize) -> Result<Vec<u8>, String> {
    let mut z = ZStream::new();
    let mut out = Vec::with_capacity(size_hint.max(64));
    let mut buf = vec![0u8; 1 << 16];
    // SAFETY: as in `gzip`.
    unsafe {
        let r = z_inflateInit2_(
            &mut *z,
            15 + 32,
            ZLIB_VERSION.as_ptr(),
            std::mem::size_of::<ZStream>() as c_int,
        );
        if r != Z_OK {
            return Err(format!("inflateInit2: {r}"));
        }
        z.next_in = data.as_ptr();
        z.avail_in = data.len() as c_uint;
        let res = loop {
            z.next_out = buf.as_mut_ptr();
            z.avail_out = buf.len() as c_uint;
            let r = z_inflate(&mut *z, Z_NO_FLUSH);
            let n = buf.len() - z.avail_out as usize;
            out.extend_from_slice(&buf[..n]);
            if r == Z_STREAM_END {
                break Ok(());
            }
            if r != Z_OK || (n == 0 && z.avail_in == 0) {
                break Err(format!("inflate: {r} (truncated or corrupt data)"));
            }
        };
        z_inflateEnd(&mut *z);
        res.map(|_| out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gzip_round_trip() {
        let input: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        let gz = gzip(&input, 9);
        assert_eq!(&gz[..3], &[0x1f, 0x8b, 8]);
        assert_eq!(gunzip(&gz, input.len()).unwrap(), input);
        assert!(gunzip(&gz[..gz.len() / 2], 0).is_err());
        assert_eq!(gunzip(&gzip(b"", 6), 0).unwrap(), b"");
    }
}
