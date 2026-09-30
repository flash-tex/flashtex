//! `utils.c` and the pdfTeX routines of web2c's `lib/texmfmp.c`, ported.
//!
//! Each routine names its C original. The string results go onto the string
//! pool at `pool_ptr`, exactly as the C code writes `strpool[poolptr++]`, and
//! an overflowing result stops at `pool_size` and leaves the error to
//! `str_room`, as there.

use super::{md5, with_state};
use crate::generated::Globals;

/// The C globals of `utils.c` and `texmfmp.c`.
#[derive(Default, Clone)]
pub struct State {
    /// `start_time`, once `init_start_time` has run.
    start_time: Option<i64>,
    source_date_epoch: bool,
    /// `start_time_str`: the creation date in PDF form.
    start_time_str: Vec<u8>,
    job_id: Option<String>,
    colstacks: Vec<ColStack>,
    /// `page_mode`: shipping a page (not a form).
    page_mode: bool,
    matrix_stack: Vec<[f64; 6]>,
    pos_stack: Vec<(i32, i32, usize)>,
    ret: [i32; 4],
    last: [i32; 4],
    /// `sub_match_count`, `pmatch`, `match_string` and
    /// `last_match_succeeded` of `\pdfmatch`.
    match_count: i32,
    match_spans: Vec<(i64, i64)>,
    match_string: Option<Vec<u8>>,
    last_match_succeeded: bool,
}

// Checkpoint registration (crate::checkpoint): the state is cloned at a
// checkpoint and persisted with a snapshot.
crate::codec_struct!(State {
    start_time,
    source_date_epoch,
    start_time_str,
    job_id,
    colstacks,
    page_mode,
    matrix_stack,
    pos_stack,
    ret,
    last,
    match_count,
    match_spans,
    match_string,
    last_match_succeeded
});

/// The C library's `regcomp` + `regexec` (csrc/flashtex_regex.c): whether
/// `pattern` matches `text`, and the first `n` subexpression spans; or
/// `regerror`'s message.
#[cfg(feature = "regex")]
fn regex_match(
    pattern: &[u8],
    text: &[u8],
    icase: bool,
    n: i32,
) -> Result<(bool, Vec<(i64, i64)>), String> {
    use std::ffi::{c_char, c_int, CString};
    extern "C" {
        fn flashtex_regex_match(
            pattern: *const c_char,
            text: *const c_char,
            icase: c_int,
            nmatch: c_int,
            so: *mut i64,
            eo: *mut i64,
            errbuf: *mut c_char,
            errlen: usize,
        ) -> c_int;
    }
    // Neither contains a NUL (`c_string`).
    let p = CString::new(pattern).unwrap_or_default();
    let t = CString::new(text).unwrap_or_default();
    let n = n.max(0) as usize;
    let (mut so, mut eo) = (vec![-1i64; n], vec![-1i64; n]);
    let mut err = vec![0u8; 512];
    // SAFETY: the buffers have the lengths passed, the strings are NUL-terminated.
    let r = unsafe {
        flashtex_regex_match(
            p.as_ptr(),
            t.as_ptr(),
            icase as c_int,
            n as c_int,
            so.as_mut_ptr(),
            eo.as_mut_ptr(),
            err.as_mut_ptr() as *mut c_char,
            err.len(),
        )
    };
    if r < 0 {
        let len = err.iter().position(|&c| c == 0).unwrap_or(err.len());
        return Err(String::from_utf8_lossy(&err[..len]).into_owned());
    }
    Ok((r == 1, so.into_iter().zip(eo).collect()))
}

/// Without the `regex` feature there is no regular-expression engine.
#[cfg(not(feature = "regex"))]
fn regex_match(_: &[u8], _: &[u8], _: bool, _: i32) -> Result<(bool, Vec<(i64, i64)>), String> {
    Err("regular expressions are not available in this build".into())
}

#[derive(Clone)]
struct ColStack {
    page_stack: Vec<Option<Vec<u8>>>,
    form_stack: Vec<Option<Vec<u8>>>,
    page_current: Option<Vec<u8>>,
    form_current: Option<Vec<u8>>,
    /// Kept as C keeps it, for `colorstackpagestart`, which never runs
    /// (see `pdfshipoutbegin`).
    #[allow(dead_code)]
    form_init: Option<Vec<u8>>,
    literal_mode: i32,
    page_start: bool,
}
crate::codec_struct!(ColStack {
    page_stack,
    form_stack,
    page_current,
    form_current,
    form_init,
    literal_mode,
    page_start
});

const COLOR_DEFAULT: &[u8] = b"0 g 0 G";
const MAX_COLORSTACKS: usize = 32768;
/// `DIRECT_ALWAYS` (pdftex.web's `direct_always`).
const DIRECT_ALWAYS: i32 = 2;

fn empty_to_none(b: Vec<u8>) -> Option<Vec<u8>> {
    if b.is_empty() {
        None
    } else {
        Some(b)
    }
}

// ---------------------------------------------------------------------------
// Dates (texmfmp.c: init_start_time, makepdftime, initstarttime)
// ---------------------------------------------------------------------------

/// `struct tm` as far as POSIX fixes it, followed by the BSD/glibc
/// extensions macOS and Linux both have.
#[repr(C)]
struct Tm {
    tm_sec: i32,
    tm_min: i32,
    tm_hour: i32,
    tm_mday: i32,
    tm_mon: i32,
    tm_year: i32,
    tm_wday: i32,
    tm_yday: i32,
    tm_isdst: i32,
    tm_gmtoff: std::ffi::c_long,
    tm_zone: *const std::ffi::c_char,
}

extern "C" {
    fn localtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
    fn gmtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
}

fn broken_down(t: i64, utc: bool) -> Tm {
    let mut tm = Tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null(),
    };
    // SAFETY: both functions only write the `struct tm` they are given.
    unsafe {
        if utc {
            gmtime_r(&t, &mut tm);
        } else {
            localtime_r(&t, &mut tm);
        }
    }
    tm
}

/// `makepdftime`: `D:YYYYmmddHHMMSS` plus `Z` or `+HH'MM'`.
pub fn make_pdf_time(t: i64, utc: bool) -> Vec<u8> {
    let lt = broken_down(t, utc);
    let mut s = format!(
        "D:{:04}{:02}{:02}{:02}{:02}{:02}",
        lt.tm_year + 1900,
        lt.tm_mon + 1,
        lt.tm_mday,
        lt.tm_hour,
        lt.tm_min,
        lt.tm_sec.min(59)
    );
    let gmt = broken_down(t, true);
    let mut off = 60 * (lt.tm_hour - gmt.tm_hour) + lt.tm_min - gmt.tm_min;
    if lt.tm_year != gmt.tm_year {
        off += if lt.tm_year > gmt.tm_year {
            1440
        } else {
            -1440
        };
    } else if lt.tm_yday != gmt.tm_yday {
        off += if lt.tm_yday > gmt.tm_yday {
            1440
        } else {
            -1440
        };
    }
    if off == 0 {
        s.push('Z');
    } else {
        let h = off / 60;
        let m = (off - h * 60).abs();
        s.push_str(&format!("{h:+03}'{m:02}'"));
    }
    s.into_bytes()
}

thread_local! {
    /// The clock pinned for an editing session (DESIGN.md §4.5): `\time`,
    /// `\day`, `\month`, `\year`, the PDF creation date and -- through
    /// the first `get_seconds_and_micros` of a run -- the random seed.
    static PINNED: std::cell::Cell<Option<(i64, i32)>> = const { std::cell::Cell::new(None) };
    /// The next `get_seconds_and_micros` returns the pinned time (once per
    /// run: later calls time `\pdfelapsedtime`, which stays real).
    static PINNED_SEED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Pin the clock at `(seconds, microseconds)` since the epoch, or unpin it.
/// Takes effect at the next run (`arm_pinned_seed` before each).
pub fn pin_clock(t: Option<(i64, i32)>) {
    PINNED.with(|p| p.set(t));
}

/// The pinned clock, if any.
pub fn pinned_clock() -> Option<(i64, i32)> {
    PINNED.with(|p| p.get())
}

/// A new run is starting: its seed comes from the pinned clock.
pub fn arm_pinned_seed() {
    PINNED_SEED.with(|p| p.set(pinned_clock().is_some()));
}

fn now_secs() -> i64 {
    match pinned_clock() {
        Some((s, _)) => s,
        None => std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
    }
}

/// `init_start_time` (texmfmp.c): `SOURCE_DATE_EPOCH`, else the clock.
fn start_time(st: &mut State) -> i64 {
    if let Some(t) = st.start_time {
        return t;
    }
    let t = match std::env::var("SOURCE_DATE_EPOCH") {
        Ok(v) => {
            st.source_date_epoch = true;
            v.trim().parse::<i64>().unwrap_or(0)
        }
        Err(_) => now_secs(),
    };
    st.start_time = Some(t);
    st.start_time_str = make_pdf_time(t, st.source_date_epoch);
    t
}

pub(super) fn start_time_str() -> Vec<u8> {
    with_state(|s| {
        start_time(&mut s.utils);
        s.utils.start_time_str.clone()
    })
}

/// Whether `FORCE_SOURCE_DATE` and `SOURCE_DATE_EPOCH` are both set.
fn force_source_date() -> bool {
    std::env::var("FORCE_SOURCE_DATE").map(|v| v == "1") == Ok(true)
        && std::env::var_os("SOURCE_DATE_EPOCH").is_some()
}

impl Globals {
    /// `assert` (pdftex.h's `pdfassert`).
    pub fn pdfassert(&mut self, b: bool) {
        assert!(b, "pdfTeX assertion failed");
    }

    /// `extxnoverd` (utils.c): `x*n/d` in double precision, rounded.
    pub fn ext_xn_over_d(&mut self, x: i32, n: i32, d: i32) -> i32 {
        let mut r = (x as f64) * (n as f64) / (d as f64);
        if r > f64::EPSILON {
            r += 0.5;
        } else {
            r -= 0.5;
        }
        if r >= i32::MAX as f64 || r <= -(i32::MAX as f64) {
            self.pdftex_warn("arithmetic: number too big");
        }
        r as i32
    }

    /// `init_start_time` (texmfmp.c).
    pub fn init_start_time(&mut self) {
        with_state(|s| {
            start_time(&mut s.utils);
        });
    }

    /// `get_seconds_and_micros` (texmfmp.c).
    pub fn seconds_and_micros(&mut self, s: &mut i32, m: &mut i32) {
        if PINNED_SEED.with(|p| p.replace(false)) {
            if let Some((ps, pm)) = pinned_clock() {
                *s = ps as i32;
                *m = pm;
                return;
            }
        }
        // `\pdfelapsedtime` or `\pdfresettimer` (the run's first call is
        // its start time): the clock as it is, which no re-run reproduces --
        // a barrier (DESIGN.md §5.3)
        if self.ready_already == 314159 {
            crate::system::note_nondeterministic("elapsedtime");
        }
        let d = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        *s = d.as_secs() as i32;
        *m = d.subsec_micros() as i32;
    }

    /// `get_date_and_time` (texmfmp.c), tex.ch's `date_and_time`: the
    /// time in UTC of `SOURCE_DATE_EPOCH` (or of now) if
    /// `FORCE_SOURCE_DATE=1`, else the local time now.
    pub fn date_and_time(&mut self, t: &mut i32, d: &mut i32, m: &mut i32, y: &mut i32) {
        let forced = std::env::var("FORCE_SOURCE_DATE").map(|v| v == "1") == Ok(true);
        let tm = if forced {
            let st = with_state(|s| start_time(&mut s.utils));
            broken_down(st, true)
        } else {
            broken_down(now_secs(), false)
        };
        *t = tm.tm_hour * 60 + tm.tm_min;
        *d = tm.tm_mday;
        *m = tm.tm_mon + 1;
        *y = tm.tm_year + 1900;
    }

    /// `getcreationdate` (texmfmp.c).
    pub fn getcreationdate(&mut self) {
        let s = start_time_str();
        if self.pool_ptr as usize + s.len() >= crate::generated::consts::pool_size as usize {
            self.pool_ptr = crate::generated::consts::pool_size;
            return;
        }
        self.pool_append(&s);
    }

    /// `find_input_file` (texmfmp.c): the file named by string `s`, quotes
    /// removed, found as a TeX input.
    pub(crate) fn find_input_file(&mut self, s: i32) -> Option<String> {
        let name: Vec<u8> = self
            .str_bytes(s)
            .into_iter()
            .filter(|&c| c != b'"')
            .collect();
        let name = String::from_utf8_lossy(&name).into_owned();
        crate::system::find_input(&name)
    }

    /// `getfilemoddate` (texmfmp.c).
    pub fn getfilemoddate(&mut self, s: i32) {
        let Some(path) = self.find_input_file(s) else {
            return;
        };
        let Ok(meta) = std::fs::metadata(&path) else {
            return;
        };
        let t = meta
            .modified()
            .ok()
            .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let s = make_pdf_time(t, force_source_date());
        if self.pool_ptr as usize + s.len() >= crate::generated::consts::pool_size as usize {
            self.pool_ptr = crate::generated::consts::pool_size;
        } else {
            self.pool_append(&s);
        }
    }

    /// `getfilesize` (texmfmp.c).
    pub fn getfilesize(&mut self, s: i32) {
        let Some(path) = self.find_input_file(s) else {
            return;
        };
        let Ok(meta) = std::fs::metadata(&path) else {
            return;
        };
        let b = meta.len().to_string().into_bytes();
        if self.pool_ptr as usize + b.len() >= crate::generated::consts::pool_size as usize {
            self.pool_ptr = crate::generated::consts::pool_size;
        } else {
            self.pool_append(&b);
        }
    }

    /// `getfiledump` (texmfmp.c): `len` bytes from `offset`, in hex.
    pub fn getfiledump(&mut self, s: i32, offset: i32, len: i32) {
        if len == 0 {
            return;
        }
        let size = crate::generated::consts::pool_size;
        if self.pool_ptr + 2 * len + 1 >= size {
            self.pool_ptr = size;
            return;
        }
        let Some(path) = self.find_input_file(s) else {
            return;
        };
        let Ok(data) = std::fs::read(&path) else {
            return;
        };
        if offset < 0 {
            return;
        }
        let from = (offset as usize).min(data.len());
        let to = (from + len as usize).min(data.len());
        let hex: Vec<u8> = data[from..to]
            .iter()
            .flat_map(|b| format!("{b:02X}").into_bytes())
            .collect();
        self.pool_append(&hex);
    }

    /// `getmd5sum` (texmfmp.c).
    pub fn getmd5sum(&mut self, s: i32, is_file: bool) {
        let digest = if is_file {
            let Some(path) = self.find_input_file(s) else {
                return;
            };
            let Ok(data) = std::fs::read(&path) else {
                return;
            };
            md5::digest(&data)
        } else {
            md5::digest(&self.str_bytes(s))
        };
        if self.pool_ptr + 32 >= crate::generated::consts::pool_size {
            return;
        }
        let hex: Vec<u8> = digest
            .iter()
            .flat_map(|b| format!("{b:02X}").into_bytes())
            .collect();
        self.pool_append(&hex);
    }

    /// `escapestring` (utils.c).
    pub fn escapestring(&mut self, p: i32) {
        let size = crate::generated::consts::pool_size;
        let out = self.pool_ptr;
        let mut i = p;
        while i < out {
            if self.pool_ptr + 4 >= size {
                self.pool_ptr = size;
                return;
            }
            let ch = self.str_pool[i as usize] as u8;
            i += 1;
            if !(b'!'..=b'~').contains(&ch) {
                self.pool_append(format!("\\{ch:03o}").as_bytes());
                continue;
            }
            if ch == b'(' || ch == b')' || ch == b'\\' {
                self.pool_append(b"\\");
            }
            self.pool_append(&[ch]);
        }
    }

    /// `escapename` (utils.c).
    pub fn escapename(&mut self, p: i32) {
        let size = crate::generated::consts::pool_size;
        let out = self.pool_ptr;
        let mut i = p;
        while i < out {
            if self.pool_ptr + 3 >= size {
                self.pool_ptr = size;
                return;
            }
            let ch = self.str_pool[i as usize] as u8;
            i += 1;
            match ch {
                0 => {}
                1..=32 | 127..=255 | 35 | 37 | 40 | 41 | 47 | 60 | 62 | 91 | 93 | 123 | 125 => {
                    self.pool_append(format!("#{ch:02X}").as_bytes())
                }
                _ => self.pool_append(&[ch]),
            }
        }
    }

    /// `escapehex` (utils.c).
    pub fn escapehex(&mut self, p: i32) {
        let size = crate::generated::consts::pool_size;
        let out = self.pool_ptr;
        let mut i = p;
        while i < out {
            if self.pool_ptr + 2 >= size {
                self.pool_ptr = size;
                return;
            }
            let ch = self.str_pool[i as usize] as u8;
            i += 1;
            self.pool_append(format!("{ch:02X}").as_bytes());
        }
    }

    /// `unescapehex` (utils.c).
    pub fn unescapehex(&mut self, p: i32) {
        let size = crate::generated::consts::pool_size;
        let out = self.pool_ptr;
        let mut i = p;
        let mut a = 0u8;
        let mut first = true;
        while i < out {
            if self.pool_ptr + 1 >= size {
                self.pool_ptr = size;
                return;
            }
            let ch = self.str_pool[i as usize] as u8;
            i += 1;
            let v = match ch {
                b'0'..=b'9' => ch - b'0',
                b'A'..=b'F' => ch - b'A' + 10,
                b'a'..=b'f' => ch - b'a' + 10,
                _ => continue,
            };
            if first {
                a = v << 4;
                first = false;
                continue;
            }
            self.pool_append(&[a.wrapping_add(v)]);
            first = true;
        }
        if !first {
            self.pool_append(&[a]);
        }
    }

    /// `matchstrings` (utils.c), `\pdfmatch`: string `s` as a POSIX
    /// extended regular expression (case-blind with `icase`), matched
    /// against string `t` by the C library's `regcomp`/`regexec`, as
    /// pdfTeX does everywhere but Windows. `1` or `0` goes onto the pool; a
    /// pattern that does not compile gives `-1` and `regerror`'s message as
    /// a warning. The searched string and up to `subcount` (default 10)
    /// subexpression positions are kept for `\pdflastmatch`.
    pub fn matchstrings(&mut self, s: i32, t: i32, subcount: i32, icase: bool) {
        let size = crate::generated::consts::pool_size;
        if self.pool_ptr + 10 >= size {
            self.pool_ptr = size;
            return;
        }
        let pattern = self.c_string(s);
        let n = if subcount < 0 { 10 } else { subcount };
        let text = self.c_string(t);
        match regex_match(&pattern, &text, icase, n) {
            Err(msg) => {
                self.pdftex_warn(&format!("\\pdfmatch: {msg}"));
                self.pool_append(b"-1");
            }
            Ok((matched, spans)) => {
                with_state(|st| {
                    st.utils.match_count = n;
                    st.utils.match_spans = spans;
                    st.utils.match_string = Some(text);
                    st.utils.last_match_succeeded = matched;
                });
                self.pool_append(if matched { b"1" } else { b"0" });
            }
        }
    }

    /// `getmatch` (utils.c), `\pdflastmatch`: `position->text` of
    /// subexpression `i` of the last `\pdfmatch`, else `-1->`.
    pub fn getmatch(&mut self, i: i32) {
        let size = crate::generated::consts::pool_size;
        let found = with_state(|st| {
            let u = &st.utils;
            let (so, eo) = *u.match_spans.get(i as usize)?;
            let text = u.match_string.as_ref()?;
            (i >= 0 && i < u.match_count && u.last_match_succeeded && so >= 0 && eo >= so)
                .then(|| (so, text[so as usize..eo as usize].to_vec()))
        });
        let need = match &found {
            Some((_, t)) => 20 + t.len() as i32,
            None => 4,
        };
        if self.pool_ptr + need >= size {
            self.pool_ptr = size;
            return;
        }
        match found {
            Some((so, t)) => {
                self.pool_append(format!("{so}->").as_bytes());
                self.pool_append(&t);
            }
            None => self.pool_append(b"-1->"),
        }
    }

    /// `setjobid` (utils.c), without the web2c and kpathsea version strings.
    pub fn set_job_id(&mut self, y: i32, m: i32, d: i32, t: i32) {
        let name = String::from_utf8_lossy(&self.str_bytes(self.job_name)).into_owned();
        let fmt = String::from_utf8_lossy(&self.str_bytes(self.format_ident)).into_owned();
        with_state(|s| {
            if s.utils.job_id.is_none() {
                s.utils.job_id = Some(format!(
                    "{y:04}/{m:02}/{d:02} {:02}:{:02} {name} {fmt}",
                    t / 60,
                    t % 60
                ));
            }
        });
    }

    /// `getresnameprefix` (utils.c): six characters from the CRC-32 of the
    /// job id.
    pub fn get_resname_prefix(&mut self) -> i32 {
        const NAME: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        let id = with_state(|s| s.utils.job_id.clone()).unwrap_or_default();
        let mut crc = crc32(id.as_bytes()) as u64;
        let mut prefix = Vec::new();
        for _ in 0..6 {
            prefix.push(NAME[(crc % NAME.len() as u64) as usize]);
            crc /= NAME.len() as u64;
        }
        self.make_tex_string(&prefix)
    }

    // printcreationdate, printmoddate, printID, printIDalt,
    // writestreamlength, removepdffile and libpdffinish write the PDF file:
    // they are in output.rs.

    /// `garbagewarning` (utils.c).
    pub fn garbage_warning(&mut self) {
        self.pdftex_warn("dangling objects discarded, no output file produced.");
        self.remove_pdffile();
    }

    /// `allocvffnts` (utils.c): grow `vf_e_fnts`/`vf_i_fnts` so that entry
    /// `vf_nf` exists.
    pub fn allocvffnts(&mut self) {
        let need = self.vf_nf as usize + 1;
        if self.vf_e_fnts.len() < need {
            // The word space reserves `font_max + 1` entries (web2rust's
            // `--arena-cap` inference), which `vf_nf <= font_max` never exceeds.
            let n = need.max(self.vf_e_fnts.len() + crate::generated::consts::font_max as usize);
            let n = n.min(self.vf_e_fnts.capacity());
            self.vf_e_fnts.resize_len(n);
            self.vf_i_fnts.resize_len(n);
        }
    }

    // ---- colour stacks (utils.c) --------------------------------------------

    fn colstacks_init(st: &mut State) {
        if st.colstacks.is_empty() {
            st.colstacks.push(ColStack {
                page_stack: vec![],
                form_stack: vec![],
                page_current: Some(COLOR_DEFAULT.to_vec()),
                form_current: Some(COLOR_DEFAULT.to_vec()),
                form_init: Some(COLOR_DEFAULT.to_vec()),
                literal_mode: DIRECT_ALWAYS,
                page_start: true,
            });
        }
    }

    /// `colorstackused` (utils.c).
    pub fn colorstackused(&mut self) -> i32 {
        with_state(|s| {
            Self::colstacks_init(&mut s.utils);
            s.utils.colstacks.len() as i32
        })
    }

    /// `newcolorstack` (utils.c): the new stack's number, or -1.
    pub fn newcolorstack(&mut self, s: i32, literal_mode: i32, page_start: bool) -> i32 {
        let init = empty_to_none(self.str_bytes(s));
        with_state(|st| {
            Self::colstacks_init(&mut st.utils);
            if st.utils.colstacks.len() == MAX_COLORSTACKS {
                return -1;
            }
            st.utils.colstacks.push(ColStack {
                page_stack: vec![],
                form_stack: vec![],
                page_current: init.clone(),
                form_current: init.clone(),
                form_init: init,
                literal_mode,
                page_start,
            });
            st.utils.colstacks.len() as i32 - 1
        })
    }

    /// `put_cstring_on_strpool` (utils.c).
    fn put_on_pool(&mut self, v: Option<Vec<u8>>) {
        if let Some(b) = v {
            let size = crate::generated::consts::pool_size;
            if self.pool_ptr + b.len() as i32 >= size {
                self.pool_ptr = size;
                return;
            }
            self.pool_append(&b);
        }
    }

    /// `colorstackset` (utils.c).
    pub fn colorstackset(&mut self, n: i32, s: i32) -> i32 {
        let v = Some(self.str_bytes(s));
        with_state(|st| {
            let page = st.utils.page_mode;
            let c = &mut st.utils.colstacks[n as usize];
            if page {
                c.page_current = v;
            } else {
                c.form_current = v;
            }
            c.literal_mode
        })
    }

    /// `colorstackcurrent` (utils.c).
    pub fn colorstackcurrent(&mut self, n: i32) -> i32 {
        let (v, mode) = with_state(|st| {
            let page = st.utils.page_mode;
            let c = &st.utils.colstacks[n as usize];
            (
                if page {
                    c.page_current.clone()
                } else {
                    c.form_current.clone()
                },
                c.literal_mode,
            )
        });
        self.put_on_pool(v);
        mode
    }

    /// `colorstackpush` (utils.c).
    pub fn colorstackpush(&mut self, n: i32, s: i32) -> i32 {
        let v = empty_to_none(self.str_bytes(s));
        with_state(|st| {
            let page = st.utils.page_mode;
            let c = &mut st.utils.colstacks[n as usize];
            if page {
                let old = std::mem::replace(&mut c.page_current, v);
                c.page_stack.push(old);
            } else {
                let old = std::mem::replace(&mut c.form_current, v);
                c.form_stack.push(old);
            }
            c.literal_mode
        })
    }

    /// `colorstackpop` (utils.c).
    pub fn colorstackpop(&mut self, n: i32) -> i32 {
        let r = with_state(|st| {
            let page = st.utils.page_mode;
            let c = &mut st.utils.colstacks[n as usize];
            let stack = if page {
                &mut c.page_stack
            } else {
                &mut c.form_stack
            };
            match stack.pop() {
                None => Err((page, c.literal_mode)),
                Some(v) => {
                    if page {
                        c.page_current = v.clone();
                    } else {
                        c.form_current = v.clone();
                    }
                    Ok((v, c.literal_mode))
                }
            }
        });
        match r {
            Err((page, mode)) => {
                let what = if page { "page" } else { "form" };
                self.pdftex_warn(&format!("pop empty color {what} stack {n}"));
                mode
            }
            Ok((v, mode)) => {
                self.put_on_pool(v);
                mode
            }
        }
    }

    /// `colorstackskippagestart` (utils.c).
    pub fn colorstackskippagestart(&mut self, n: i32) -> i32 {
        with_state(|st| {
            let c = &st.utils.colstacks[n as usize];
            if !c.page_start {
                1
            } else if c.page_current.is_none() {
                0
            } else if c.page_current.as_deref() == Some(COLOR_DEFAULT) {
                2
            } else {
                0
            }
        })
    }

    // ---- \pdfsave, \pdfrestore, \pdfsetmatrix (utils.c) ----------------------

    /// `checkpdfsave` (utils.c).
    pub fn checkpdfsave(&mut self, cur_h: i32, cur_v: i32) {
        with_state(|st| {
            let m = if st.utils.page_mode {
                st.utils.matrix_stack.len()
            } else {
                0
            };
            st.utils.pos_stack.push((cur_h, cur_v, m));
        });
    }

    /// `checkpdfrestore` (utils.c).
    pub fn checkpdfrestore(&mut self, cur_h: i32, cur_v: i32) {
        let top = with_state(|st| st.utils.pos_stack.pop());
        let Some((h, v, m)) = top else {
            self.pdftex_warn("\\pdfrestore: missing \\pdfsave");
            return;
        };
        let (dh, dv) = (cur_h.wrapping_sub(h), cur_v.wrapping_sub(v));
        if dh != 0 || dv != 0 {
            // C prints the differences with %u.
            self.pdftex_warn(&format!(
                "Misplaced \\pdfrestore by ({}sp, {}sp)",
                dh as u32, dv as u32
            ));
        }
        with_state(|st| {
            if st.utils.page_mode {
                st.utils.matrix_stack.truncate(m);
            }
        });
    }

    /// `pdfshipoutbegin` (utils.c).
    pub fn pdfshipoutbegin(&mut self, shipping_page: bool) {
        // C then calls `colorstackpagestart` when `shipping_page`, which
        // returns at once because `page_mode` is then set: it never resets
        // anything, and neither does this.
        with_state(|st| {
            st.utils.pos_stack.clear();
            st.utils.page_mode = shipping_page;
        });
        // The content stream starts here (after the magnification's `cm`):
        // the display list reads it from this point (src/displaylist/).
        self.dl_shipout_begin(shipping_page);
    }

    /// `pdfshipoutend` (utils.c).
    pub fn pdfshipoutend(&mut self, shipping_page: bool) {
        // `pdf_end_text` has run: the stream is complete.
        self.dl_shipout_end(shipping_page);
        let n = with_state(|st| st.utils.pos_stack.len());
        if n > 0 {
            let what = if shipping_page { "page" } else { "form" };
            self.pdftex_fail(&format!("{n} unmatched \\pdfsave after {what} shipout"));
        }
    }

    /// `pdfsetmatrix` (utils.c): 1 on success, 0 if the argument is not four
    /// numbers.
    pub fn pdfsetmatrix(&mut self, p: i32, cur_h: i32, cur_v: i32) -> i32 {
        let text = self.pool_bytes_from(p);
        let text = String::from_utf8_lossy(&text);
        with_state(|st| {
            if !st.utils.page_mode {
                return 1;
            }
            let nums: Vec<&str> = text.split_whitespace().collect();
            if nums.len() != 4 {
                return 0;
            }
            let mut v = [0f64; 4];
            for (i, n) in nums.iter().enumerate() {
                match n.parse::<f64>() {
                    Ok(x) => v[i] = x,
                    Err(_) => return 0,
                }
            }
            let [a, b, c, d] = v;
            let e = cur_h as f64 * (1.0 - a) - cur_v as f64 * c;
            let f = cur_v as f64 * (1.0 - d) - cur_h as f64 * b;
            let z = match st.utils.matrix_stack.last() {
                Some(y) => [
                    a * y[0] + b * y[2],
                    a * y[1] + b * y[3],
                    c * y[0] + d * y[2],
                    c * y[1] + d * y[3],
                    e * y[0] + f * y[2] + y[4],
                    e * y[1] + f * y[3] + y[5],
                ],
                None => [a, b, c, d, e, f],
            };
            st.utils.matrix_stack.push(z);
            1
        })
    }

    /// `matrixused` (utils.c).
    pub fn matrixused(&mut self) -> bool {
        let used = with_state(|st| !st.utils.matrix_stack.is_empty());
        if used {
            super::set_matrix_uses(super::matrix_uses() + 1);
        }
        used
    }

    /// `matrixtransformrect` (utils.c).
    pub fn matrixtransformrect(&mut self, llx: i32, lly: i32, urx: i32, ury: i32) {
        with_state(|st| {
            let u = &mut st.utils;
            match (u.page_mode, u.matrix_stack.last()) {
                (true, Some(m)) => {
                    u.last = [llx, lly, urx, ury];
                    let tr = |x: i32, y: i32| {
                        let round = |v: f64| if v > 0.0 { v + 0.5 } else { v - 0.5 } as i32;
                        let (x, y) = (x as f64, y as f64);
                        (
                            round(x * m[0] + y * m[2] + m[4]),
                            round(x * m[1] + y * m[3] + m[5]),
                        )
                    };
                    let p = [tr(llx, lly), tr(llx, ury), tr(urx, lly), tr(urx, ury)];
                    u.ret = [
                        p.iter().map(|q| q.0).min().unwrap(),
                        p.iter().map(|q| q.1).min().unwrap(),
                        p.iter().map(|q| q.0).max().unwrap(),
                        p.iter().map(|q| q.1).max().unwrap(),
                    ];
                }
                _ => u.ret = [llx, lly, urx, ury],
            }
        });
    }

    /// `matrixrecalculate` (utils.c).
    pub fn matrixrecalculate(&mut self, urx: i32) {
        let l = with_state(|st| st.utils.last);
        self.matrixtransformrect(l[0], l[1], urx, l[3]);
    }

    /// `getllx` (utils.c).
    pub fn getllx(&mut self) -> i32 {
        with_state(|st| st.utils.ret[0])
    }
    /// `getlly` (utils.c).
    pub fn getlly(&mut self) -> i32 {
        with_state(|st| st.utils.ret[1])
    }
    /// `geturx` (utils.c).
    pub fn geturx(&mut self) -> i32 {
        with_state(|st| st.utils.ret[2])
    }
    /// `getury` (utils.c).
    pub fn getury(&mut self) -> i32 {
        with_state(|st| st.utils.ret[3])
    }

    /// `pdftex_fail` (utils.c): the same layout as pdftex.web's `pdf_error`,
    /// then the run ends.
    pub fn pdftex_fail(&mut self, msg: &str) -> ! {
        // safe_print: `print` of each character code
        fn safe_print(g: &mut Globals, s: &[u8]) {
            for &c in s {
                g.print(c as i32);
            }
        }
        self.print_ln();
        safe_print(self, b"!pdfTeX error: ");
        let name = crate::system::invocation_name();
        safe_print(self, name.as_bytes());
        if let Some(f) = super::output::cur_file_name() {
            safe_print(self, b" (file ");
            safe_print(self, &f);
            safe_print(self, b")");
        }
        safe_print(self, b": ");
        safe_print(self, super::output::printf_cut(msg.as_bytes()));
        self.print_ln();
        self.remove_pdffile();
        safe_print(
            self,
            b" ==> Fatal error occurred, no output PDF file produced!",
        );
        self.print_ln();
        // exit(EXIT_FAILURE), which flushes C's buffered files (all of them:
        // the log, the terminal, the \write files)
        crate::system::exit_process(self, 1)
    }
}

/// zlib's CRC-32 (`crc32(0, ...)`), for `getresnameprefix`.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    #[test]
    fn crc32_matches_zlib() {
        assert_eq!(super::crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn pdf_time_in_utc() {
        assert_eq!(super::make_pdf_time(0, true), b"D:19700101000000Z");
    }
}
