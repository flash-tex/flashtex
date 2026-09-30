//! C's `sprintf("%g")` and `sscanf("%f"/"%g")`, from the C library itself.
//!
//! pdfTeX's font code reads numbers from map lines and Type 1 fonts with
//! `sscanf` and writes some back with `sprintf("%g")` (a slanted or extended
//! `/FontMatrix`, `/ItalicAngle`). Rust's own float parsing and printing
//! differ from C's in the edge cases (which prefixes `sscanf` accepts, how
//! `%g` picks its form), so these call the same C library functions pdfTeX
//! calls, and the bytes agree by construction.

use std::ffi::{c_char, c_int};

extern "C" {
    fn snprintf(buf: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    fn sscanf(s: *const c_char, fmt: *const c_char, ...) -> c_int;
}

/// A NUL-terminated copy of `s`, cut at its first NUL as C would see it.
fn c_string(s: &[u8]) -> Vec<u8> {
    let end = s.iter().position(|&b| b == 0).unwrap_or(s.len());
    let mut v = s[..end].to_vec();
    v.push(0);
    v
}

/// `sprintf(buf, "%g", x)`.
pub fn fmt_g(x: f64) -> Vec<u8> {
    let mut buf = [0u8; 64];
    // SAFETY: `buf` is 64 bytes and `snprintf` writes at most that many,
    // NUL included; the format takes exactly one double.
    let n = unsafe {
        snprintf(
            buf.as_mut_ptr() as *mut c_char,
            buf.len(),
            c"%g".as_ptr(),
            x,
        )
    };
    buf[..(n.max(0) as usize).min(buf.len() - 1)].to_vec()
}

/// `sprintf(buf, "%.<prec>f", x)` (pdftoepdf.cc's `%.1f` and `%.8f`).
pub fn fmt_f(prec: i32, x: f64) -> Vec<u8> {
    let mut buf = vec![0u8; 512];
    // SAFETY: `buf` is 512 bytes and `snprintf` writes at most that many,
    // NUL included; the format takes an int and a double.
    let n = unsafe {
        snprintf(
            buf.as_mut_ptr() as *mut c_char,
            buf.len(),
            c"%.*f".as_ptr(),
            prec as c_int,
            x,
        )
    };
    buf.truncate((n.max(0) as usize).min(buf.len() - 1));
    buf
}

/// `sscanf(s, "%g", &f)` into a C `float`: the number, or `None` if the
/// conversion failed.
pub fn scan_float(s: &[u8]) -> Option<f32> {
    let cs = c_string(s);
    let mut f: f32 = 0.0;
    // SAFETY: `cs` is NUL-terminated; `%g` stores one float into `f`.
    let r = unsafe {
        sscanf(
            cs.as_ptr() as *const c_char,
            c"%g".as_ptr(),
            &mut f as *mut f32,
        )
    };
    (r == 1).then_some(f)
}

/// `sscanf(s, "%f %n", &d, &j)`: the float and the bytes consumed
/// (trailing blanks included), or `None` if no number was read.
pub fn scan_float_n(s: &[u8]) -> Option<(f32, usize)> {
    let cs = c_string(s);
    let mut f: f32 = 0.0;
    let mut n: c_int = 0;
    // SAFETY: as above; `%n` stores one int into `n`.
    let r = unsafe {
        sscanf(
            cs.as_ptr() as *const c_char,
            c"%f %n".as_ptr(),
            &mut f as *mut f32,
            &mut n as *mut c_int,
        )
    };
    (r > 0).then_some((f, n as usize))
}

/// `sscanf(s, fmt, &a, &b, &c)` with up to three `int` conversions
/// (`%i`, `%n`) and no others: the assignment count and the values.
pub fn scan_ints(s: &[u8], fmt: &std::ffi::CStr) -> (i32, [i32; 3]) {
    let cs = c_string(s);
    let mut v: [c_int; 3] = [0; 3];
    // SAFETY: callers pass formats with at most three `int` conversions
    // (`%i`/`%n`); each gets its own `int`.
    let r = unsafe {
        sscanf(
            cs.as_ptr() as *const c_char,
            fmt.as_ptr(),
            &mut v[0] as *mut c_int,
            &mut v[1] as *mut c_int,
            &mut v[2] as *mut c_int,
        )
    };
    (r, v)
}

/// `sscanf(s, fmt, ...)` with exactly eight `int` conversions (writet3.c's
/// `\pdfglyph` preamble): the assignment count and the values.
pub fn scan_ints8(s: &[u8], fmt: &std::ffi::CStr) -> (i32, [i32; 8]) {
    let cs = c_string(s);
    let mut v: [c_int; 8] = [0; 8];
    let p = v.as_mut_ptr();
    // SAFETY: callers pass formats with at most eight `int` conversions;
    // each gets its own `int` of `v`.
    let r = unsafe {
        sscanf(
            cs.as_ptr() as *const c_char,
            fmt.as_ptr(),
            p,
            p.add(1),
            p.add(2),
            p.add(3),
            p.add(4),
            p.add(5),
            p.add(6),
            p.add(7),
        )
    };
    (r, v)
}

/// `n = -1; sscanf(s, fmt, &index, &n)` with one `int` conversion and a
/// `%n` (writettf.c's `uni%X%n` and `index%i%n`): `(index, n)`, `n` still
/// -1 when the conversion failed.
pub fn scan_prefixed(s: &[u8], fmt: &std::ffi::CStr) -> (i32, i32) {
    let cs = c_string(s);
    let (mut index, mut n): (c_int, c_int) = (0, -1);
    // SAFETY: the format has one int conversion and one %n, each given
    // its own int.
    unsafe {
        sscanf(
            cs.as_ptr() as *const c_char,
            fmt.as_ptr(),
            &mut index as *mut c_int,
            &mut n as *mut c_int,
        )
    };
    (index, n)
}

/// `sscanf(s, " %li %n", &i, &n)` (subfont.c): the return value; `i` and
/// `n` keep their values where nothing is assigned, as in C.
pub fn scan_long_n(s: &[u8], i: &mut i64, n: &mut i32) -> i32 {
    let cs = c_string(s);
    let mut l: std::ffi::c_long = *i as std::ffi::c_long;
    let mut m: c_int = *n;
    // SAFETY: one long and one int (%n) conversion.
    let r = unsafe {
        sscanf(
            cs.as_ptr() as *const c_char,
            c" %li %n".as_ptr(),
            &mut l as *mut std::ffi::c_long,
            &mut m as *mut c_int,
        )
    };
    *i = l as i64;
    *n = m;
    r
}

/// `sscanf(s, "%s %n", buf, &n)` into a 256-byte buffer (subfont.c): the
/// word and `n` (unchanged if not assigned).
pub fn scan_word_n(s: &[u8], n: &mut i32) -> Vec<u8> {
    let cs = c_string(s);
    // (sfd_line is at most 256 bytes, so the word fits, as in C)
    let mut buf = vec![0u8; cs.len() + 1];
    let mut m: c_int = *n;
    // SAFETY: `buf` is longer than the input, so `%s` cannot overflow it.
    unsafe {
        sscanf(
            cs.as_ptr() as *const c_char,
            c"%s %n".as_ptr(),
            buf.as_mut_ptr() as *mut c_char,
            &mut m as *mut c_int,
        )
    };
    *n = m;
    let end = buf.iter().position(|&b| b == 0).unwrap_or(0);
    buf[..end].to_vec()
}

/// `sscanf(s, "dup %i%255s put", &i, buf)`: the assignment count, `i` and
/// `buf`.
pub fn scan_dup_put(s: &[u8]) -> (i32, i32, Vec<u8>) {
    let cs = c_string(s);
    let mut i: c_int = 0;
    let mut buf = [0u8; 257];
    // SAFETY: one int and one string of at most 255 bytes plus NUL into a
    // 257-byte buffer.
    let r = unsafe {
        sscanf(
            cs.as_ptr() as *const c_char,
            c"dup %i%255s put".as_ptr(),
            &mut i as *mut c_int,
            buf.as_mut_ptr() as *mut c_char,
        )
    };
    let end = buf.iter().position(|&b| b == 0).unwrap_or(0);
    (r, i, buf[..end].to_vec())
}

/// `sscanf(s, "%i", &i)`: whether a number follows.
pub fn scan_one_int(s: &[u8]) -> Option<i32> {
    let cs = c_string(s);
    let mut i: c_int = 0;
    // SAFETY: one int conversion into `i`.
    let r = unsafe {
        sscanf(
            cs.as_ptr() as *const c_char,
            c"%i".as_ptr(),
            &mut i as *mut c_int,
        )
    };
    (r == 1).then_some(i)
}

/// `sscanf(s, "%lX", &v)`.
pub fn scan_hex(s: &[u8]) -> Option<u64> {
    let cs = c_string(s);
    let mut v: std::ffi::c_ulong = 0;
    // SAFETY: one unsigned long conversion into `v`.
    let r = unsafe {
        sscanf(
            cs.as_ptr() as *const c_char,
            c"%lX".as_ptr(),
            &mut v as *mut std::ffi::c_ulong,
        )
    };
    (r == 1).then_some(v as u64)
}

/// `sscanf(s, "%4lX", &v)`.
pub fn scan_hex4(s: &[u8]) -> Option<u64> {
    let cs = c_string(s);
    let mut v: std::ffi::c_ulong = 0;
    // SAFETY: one unsigned long conversion into `v`.
    let r = unsafe {
        sscanf(
            cs.as_ptr() as *const c_char,
            c"%4lX".as_ptr(),
            &mut v as *mut std::ffi::c_ulong,
        )
    };
    (r == 1).then_some(v as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g_is_cs() {
        assert_eq!(fmt_g(0.001), b"0.001");
        assert_eq!(fmt_g(-9.46), b"-9.46");
        assert_eq!(fmt_g(1e-5), b"1e-05");
        assert_eq!(fmt_g(0.1669999957084656), b"0.167");
    }

    #[test]
    fn scans() {
        assert_eq!(scan_float_n(b"0.167 SlantFont"), Some((0.167, 6)));
        assert_eq!(scan_float(b" -1.5 def"), Some(-1.5));
        assert_eq!(scan_float(b"x"), None);
        assert_eq!(
            scan_dup_put(b"dup 32 /space put"),
            (2, 32, b"/space".to_vec())
        );
        let (r, v) = scan_ints(b"PidEid=3, 1 x", c"PidEid=%i, %i %n");
        assert_eq!((r, v), (2, [3, 1, 12]));
    }
}
