//! mkind.c and mkind.h: the program's state, `main` and the file handling.

use crate::io::{mk_getc, InFile, Out, EOF};
use crate::locale::c_isdigit;
use crate::{host_path, Exit, Host, R};
use std::io::Write;
use std::rc::Rc;

// mkind.h's limits.
pub const ARGUMENT_MAX: usize = 10240;
pub const ARRAY_MAX: usize = 1024;
pub const FIELD_MAX: usize = 3;
pub const LINE_MAX: i32 = 72;
pub const NUMBER_MAX: usize = 99;
pub const PAGEFIELD_MAX: usize = 10;
pub const PAGETYPE_MAX: usize = 5;
pub const ROMAN_MAX: usize = 99;
pub const ARABIC_MAX: usize = 99;
pub const STRING_MAX: usize = 999;

pub const EMPTY: i16 = -9999;
pub const ROML: i16 = 0;
pub const ROMU: i16 = 1;
pub const ARAB: i16 = 2;
pub const ALPL: i16 = 3;
pub const ALPU: i16 = 4;
pub const DUPLICATE: i16 = 9999;

pub const SYMBOL: i32 = -1;
pub const ALPHA: i32 = -2;

pub const TAB: i32 = b'\t' as i32;
pub const LFD: i32 = b'\n' as i32;
pub const SPC: i32 = b' ' as i32;

pub const DOT_MAX: i32 = 1000;
pub const CMP_MAX: i32 = 1500;

const VERSION: &[u8] = b"version 2.18 [TeX Live 2026] (kpathsea + Thai support)";
const INDEX_IDX: &[u8] = b".idx";
const INDEX_ILG: &[u8] = b".ilg";
const INDEX_IND: &[u8] = b".ind";
const INDEX_STY: &[u8] = b".mst";
const INDEX_LOG: &[u8] = b".log";

/// A `char` value as an `int`: `char` is signed.
pub fn sc(b: u8) -> i32 {
    b as i8 as i32
}

/// The C string in `buf`: up to its first NUL.
pub fn cstr(buf: &[u8]) -> &[u8] {
    match buf.iter().position(|&b| b == 0) {
        Some(n) => &buf[..n],
        None => buf,
    }
}

/// `s[i]`, NUL past the end.
pub fn at(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

/// One `printf` argument.
pub enum P<'a> {
    S(&'a [u8]),
    D(i64),
    C(u8),
}

impl<'a> From<&'a [u8]> for P<'a> {
    fn from(s: &'a [u8]) -> P<'a> {
        P::S(s)
    }
}
impl<'a> From<&'a Vec<u8>> for P<'a> {
    fn from(s: &'a Vec<u8>) -> P<'a> {
        P::S(s)
    }
}
impl<'a> From<&'a str> for P<'a> {
    fn from(s: &'a str) -> P<'a> {
        P::S(s.as_bytes())
    }
}
impl<'a, const N: usize> From<&'a [u8; N]> for P<'a> {
    fn from(s: &'a [u8; N]) -> P<'a> {
        P::S(s)
    }
}
impl From<i32> for P<'_> {
    fn from(d: i32) -> Self {
        P::D(d as i64)
    }
}
impl From<i64> for P<'_> {
    fn from(d: i64) -> Self {
        P::D(d)
    }
}
impl From<usize> for P<'_> {
    fn from(d: usize) -> Self {
        P::D(d as i64)
    }
}

/// `sprintf` for the formats makeindex uses: `%s`, `%d`, `%ld`, `%c`.
pub fn fmt(f: &str, args: &[P]) -> Vec<u8> {
    let f = f.as_bytes();
    let mut out = Vec::with_capacity(f.len() + 16);
    let mut k = 0;
    let mut i = 0;
    while i < f.len() {
        if f[i] == b'%' {
            i += 1;
            if f[i] == b'l' {
                i += 1;
            }
            match (f[i], &args[k]) {
                (b's', P::S(s)) => out.extend_from_slice(cstr(s)),
                (b'd', P::D(d)) => out.extend_from_slice(d.to_string().as_bytes()),
                (b'c', P::C(c)) => out.push(*c),
                (b'c', P::D(d)) => out.push(*d as u8),
                _ => panic!("format {:?}", String::from_utf8_lossy(f)),
            }
            k += 1;
        } else {
            out.push(f[i]);
        }
        i += 1;
    }
    out
}

/// mkind.h's `FIELD`: one index entry.
#[derive(Clone)]
pub struct Field {
    pub sf: [Vec<u8>; FIELD_MAX],
    pub af: [Vec<u8>; FIELD_MAX],
    pub group: i32,
    pub lpg: Vec<u8>,
    pub npg: [i32; PAGEFIELD_MAX],
    pub count: i16,
    pub type_: i16,
    pub encap: Vec<u8>,
    pub fn_: Rc<Vec<u8>>,
    pub lc: i32,
}

/// The program's global and static variables.
pub struct Mk<'h> {
    pub host: &'h mut dyn Host,

    // mkind.c
    pub letter_ordering: bool,
    pub compress_blanks: bool,
    pub merge_page: bool,
    pub init_page: bool,
    pub even_odd: i32,
    pub verbose: bool,
    pub german_sort: bool,
    pub thai_sort: bool,
    pub locale_sort: bool,
    pub fn_no: i32,
    pub idx_dot: bool,
    pub idx_tt: i32,
    pub idx_et: i32,
    pub idx_gt: i32,

    pub idx_key: Vec<usize>,
    pub log_fp: Option<InFile>,
    pub sty_fp: Option<InFile>,
    pub idx_fp: Option<InFile>,
    pub ind_fp: Option<Out>,
    pub ilg_fp: Option<Out>,

    pub pgm_fn: Vec<u8>,
    pub sty_fn: Vec<u8>,
    pub idx_fn: Rc<Vec<u8>>,
    pub ind_fn: Vec<u8>,
    pub ilg_fn: Vec<u8>,
    pub pageno: Vec<u8>,
    pub log_fn: Vec<u8>,
    pub base: Vec<u8>,
    pub need_version: bool,
    /// `mk_getc`'s static lookahead.
    pub lookahead: i32,

    // scanst.c: the style
    /// `char idx_keyword[ARRAY_MAX]`, the whole array: `type_guess[10]`
    /// (scanid.c) is its first four bytes in TeX Live's binary.
    pub idx_keyword: Vec<u8>,
    /// `scan_string`'s stack buffer `clone[ARRAY_MAX]`, which keeps the
    /// bytes of earlier calls (printed by "No closing delimiter").
    pub sty_clone: Vec<u8>,
    pub idx_aopen: u8,
    pub idx_aclose: u8,
    pub idx_level: u8,
    pub idx_ropen: u8,
    pub idx_rclose: u8,
    pub idx_quote: u8,
    pub idx_actual: u8,
    pub idx_encap: u8,
    pub idx_escape: u8,
    pub preamble: Vec<u8>,
    pub postamble: Vec<u8>,
    pub prelen: i32,
    pub postlen: i32,
    pub setpage_open: Vec<u8>,
    pub setpage_close: Vec<u8>,
    pub setpagelen: i32,
    pub group_skip: Vec<u8>,
    pub skiplen: i32,
    pub headings_flag: i32,
    pub heading_pre: Vec<u8>,
    pub heading_suf: Vec<u8>,
    pub headprelen: i32,
    pub headsuflen: i32,
    pub symhead_pos: Vec<u8>,
    pub symhead_neg: Vec<u8>,
    pub numhead_pos: Vec<u8>,
    pub numhead_neg: Vec<u8>,
    pub item_r: [Vec<u8>; FIELD_MAX],
    pub item_u: [Vec<u8>; FIELD_MAX],
    pub item_x: [Vec<u8>; FIELD_MAX],
    pub ilen_r: [i32; FIELD_MAX],
    pub ilen_u: [i32; FIELD_MAX],
    pub ilen_x: [i32; FIELD_MAX],
    pub delim_p: [Vec<u8>; FIELD_MAX],
    pub delim_n: Vec<u8>,
    pub delim_r: Vec<u8>,
    pub delim_t: Vec<u8>,
    pub suffix_2p: Vec<u8>,
    pub suffix_3p: Vec<u8>,
    pub suffix_mp: Vec<u8>,
    pub encap_p: Vec<u8>,
    pub encap_i: Vec<u8>,
    pub encap_s: Vec<u8>,
    pub linemax: i32,
    pub indent_length: i32,
    pub indent_space: Vec<u8>,
    pub page_comp: Vec<u8>,
    pub page_offset: [i32; PAGETYPE_MAX],
    pub page_prec: Vec<u8>,
    pub sty_lc: i32,
    pub sty_tc: i32,
    pub sty_ec: i32,
    pub put_dot: bool,
    /// `scan_sty`'s local `tmp`, which `fscanf` may leave unset.
    pub sty_tmp: i32,

    // scanid.c
    pub idx_lc: i32,
    pub idx_tc: i32,
    pub idx_ec: i32,
    pub idx_dc: i32,
    pub comp_len: usize,
    /// The static `key[ARGUMENT_MAX]` (one byte more: `scan_arg1` may
    /// write one past its end).
    pub key: Vec<u8>,
    /// The static `no[NUMBER_MAX]`.
    pub no: Vec<u8>,
    pub nodes: Vec<Field>,
    /// `scan_no`'s static `int type_guess[PAGEFIELD_MAX]`. It is indexed
    /// by a field count that may be PAGEFIELD_MAX; that element lies past
    /// the array, and in TeX Live 2026's macOS binary it is the first four
    /// bytes of `idx_keyword` (scanst.c's data follows scanid.c's): a page
    /// number of 11 fields overwrites the keyword with a little-endian
    /// `int`, and every later `\indexentry` is then an unknown keyword.
    /// [`Mk::tg_get`] and [`Mk::tg_set`] reproduce that.
    pub type_guess: [i32; PAGEFIELD_MAX],

    // sortid.c
    pub idx_gc: i64,
    pub collate: Option<crate::locale::Env>,

    // genind.c
    pub curr: Option<usize>,
    pub prev: Option<usize>,
    pub begin: Option<usize>,
    pub the_end: Option<usize>,
    pub range_ptr: Option<usize>,
    pub level: usize,
    pub prev_level: usize,
    pub encap: Vec<u8>,
    pub prev_encap: Option<Vec<u8>>,
    pub in_range: bool,
    pub encap_range: bool,
    pub buff: Vec<u8>,
    pub line: Vec<u8>,
    pub ind_lc: i32,
    pub ind_ec: i32,
    pub ind_indent: i32,
    pub ctype: Option<crate::locale::Env>,
}

fn v(s: &[u8]) -> Vec<u8> {
    s.to_vec()
}

impl<'h> Mk<'h> {
    pub fn new(host: &'h mut dyn Host) -> Mk<'h> {
        Mk {
            host,
            letter_ordering: false,
            compress_blanks: false,
            merge_page: true,
            init_page: false,
            even_odd: -1,
            verbose: true,
            german_sort: false,
            thai_sort: false,
            locale_sort: false,
            fn_no: -1,
            idx_dot: true,
            idx_tt: 0,
            idx_et: 0,
            idx_gt: 0,
            idx_key: vec![],
            log_fp: None,
            sty_fp: None,
            idx_fp: None,
            ind_fp: None,
            ilg_fp: None,
            // kpse_set_program_name (argv[0], "makeindex")
            pgm_fn: v(b"makeindex"),
            sty_fn: vec![],
            idx_fn: Rc::new(vec![]),
            ind_fn: vec![],
            ilg_fn: vec![],
            pageno: vec![],
            log_fn: vec![],
            base: vec![],
            need_version: true,
            lookahead: -2,
            idx_keyword: {
                let mut k = vec![0u8; ARRAY_MAX];
                k[..11].copy_from_slice(b"\\indexentry");
                k
            },
            sty_clone: vec![0u8; ARRAY_MAX + 1],
            idx_aopen: b'{',
            idx_aclose: b'}',
            idx_level: b'!',
            idx_ropen: b'(',
            idx_rclose: b')',
            idx_quote: b'"',
            idx_actual: b'@',
            idx_encap: b'|',
            idx_escape: b'\\',
            preamble: v(b"\\begin{theindex}\n"),
            postamble: v(b"\n\n\\end{theindex}\n"),
            prelen: 1,
            postlen: 3,
            setpage_open: v(b"\n  \\setcounter{page}{"),
            setpage_close: v(b"}\n"),
            setpagelen: 2,
            group_skip: v(b"\n\n  \\indexspace\n"),
            skiplen: 3,
            headings_flag: 0,
            heading_pre: vec![],
            heading_suf: vec![],
            headprelen: 0,
            headsuflen: 0,
            symhead_pos: v(b"Symbols"),
            symhead_neg: v(b"symbols"),
            numhead_pos: v(b"Numbers"),
            numhead_neg: v(b"numbers"),
            item_r: [
                v(b"\n  \\item "),
                v(b"\n    \\subitem "),
                v(b"\n      \\subsubitem "),
            ],
            item_u: [vec![], v(b"\n    \\subitem "), v(b"\n      \\subsubitem ")],
            item_x: [vec![], v(b"\n    \\subitem "), v(b"\n      \\subsubitem ")],
            ilen_r: [1, 1, 1],
            ilen_u: [0, 1, 1],
            ilen_x: [0, 1, 1],
            delim_p: [v(b", "), v(b", "), v(b", ")],
            delim_n: v(b", "),
            delim_r: v(b"--"),
            delim_t: vec![],
            suffix_2p: vec![],
            suffix_3p: vec![],
            suffix_mp: vec![],
            encap_p: v(b"\\"),
            encap_i: v(b"{"),
            encap_s: v(b"}"),
            linemax: LINE_MAX,
            indent_length: 16,
            indent_space: v(b"\t\t"),
            page_comp: v(b"-"),
            page_offset: [0, 10000, 20000, 20026, 30026],
            page_prec: v(b"rnaRA"),
            sty_lc: 0,
            sty_tc: 0,
            sty_ec: 0,
            put_dot: false,
            sty_tmp: 0,
            idx_lc: 0,
            idx_tc: 0,
            idx_ec: 0,
            idx_dc: 0,
            comp_len: 0,
            key: vec![0; ARGUMENT_MAX + 2],
            no: vec![0; NUMBER_MAX + 1],
            nodes: vec![],
            type_guess: [EMPTY as i32; PAGEFIELD_MAX],
            idx_gc: 0,
            collate: None,
            curr: None,
            prev: None,
            begin: None,
            the_end: None,
            range_ptr: None,
            level: 0,
            prev_level: 0,
            encap: vec![],
            prev_encap: None,
            in_range: false,
            encap_range: false,
            buff: vec![],
            line: vec![],
            ind_lc: 0,
            ind_ec: 0,
            ind_indent: 0,
            ctype: None,
        }
    }

    /// `type_guess[k]`, k <= PAGEFIELD_MAX (see the field).
    pub fn tg_get(&self, k: usize) -> i32 {
        if k < PAGEFIELD_MAX {
            self.type_guess[k]
        } else {
            let b = &self.idx_keyword;
            i32::from_le_bytes([b[0], b[1], b[2], b[3]])
        }
    }

    /// `type_guess[k] = v`.
    pub fn tg_set(&mut self, k: usize, v: i32) {
        if k < PAGEFIELD_MAX {
            self.type_guess[k] = v;
        } else {
            self.idx_keyword[..4].copy_from_slice(&v.to_le_bytes());
        }
    }

    // ---- output -------------------------------------------------------

    pub fn err(&mut self, b: &[u8]) {
        let _ = self.host.stderr().write_all(b);
    }

    fn put(host: &mut dyn Host, o: &mut Option<Out>, b: &[u8]) {
        let _ = match o {
            Some(Out::File(_, buf)) => {
                buf.extend_from_slice(b);
                Ok(())
            }
            Some(Out::Stdout) => host.stdout().write_all(b),
            Some(Out::Stderr) => host.stderr().write_all(b),
            None => Ok(()),
        };
    }

    pub fn ilg(&mut self, b: &[u8]) {
        Self::put(self.host, &mut self.ilg_fp, b);
    }

    pub fn ind(&mut self, b: &[u8]) {
        Self::put(self.host, &mut self.ind_fp, b);
    }

    /// `MESSAGE`/`MESSAGE1`.
    pub fn message(&mut self, b: &[u8]) {
        if self.verbose {
            self.err(b);
        }
        self.ilg(b);
    }

    /// `FATAL`/`FATAL1`/`FATAL2`: the message and the usage on standard
    /// error, then `exit(1)`.
    pub fn fatal(&mut self, msg: &[u8]) -> Exit {
        self.err(msg);
        let u = fmt(
            "Usage: %s [-ilqrcgLT] [-s sty] [-o ind] [-t log] [-p num] [idx0 idx1 ...]\n",
            &[P::S(&self.pgm_fn.clone())],
        );
        self.err(&u);
        Exit(1)
    }

    /// `IDX_DOT(MAX)`.
    pub fn idx_dot_(&mut self, max: i32) {
        self.idx_dot = true;
        let first = self.idx_dc == 0;
        self.idx_dc += 1;
        if first {
            if self.verbose {
                self.err(b".");
            }
            self.ilg(b".");
        }
        if self.idx_dc == max {
            self.idx_dc = 0;
        }
    }

    /// `DONE(A, B, C, D)`.
    pub fn done(&mut self, a: i32, b: &str, c: i32, d: &str) {
        let m = fmt(
            "done (%d %s, %d %s).\n",
            &[a.into(), b.into(), c.into(), d.into()],
        );
        self.message(&m);
    }

    /// What `exit` and the final `fclose`s do to the streams.
    /// After a signal (`killed`), only the whole `BUFSIZ` blocks stdio had
    /// written out reach the files.
    pub fn finish(&mut self, killed: bool) {
        const BUFSIZ: usize = 4096;
        for o in [&mut self.ind_fp, &mut self.ilg_fp] {
            if let Some(Out::File(f, buf)) = o {
                let n = if killed {
                    buf.len() / BUFSIZ * BUFSIZ
                } else {
                    buf.len()
                };
                let _ = f.write_all(&buf[..n]);
            }
        }
        self.ind_fp = None;
        self.ilg_fp = None;
        let _ = self.host.stdout().flush();
        let _ = self.host.stderr().flush();
    }

    // ---- files ----------------------------------------------------------

    fn path(&self, name: &[u8]) -> std::path::PathBuf {
        host_path(self.host.cwd(), name)
    }

    /// `fopen(name, "rb")`.
    pub fn open_in(&self, name: &[u8]) -> Option<InFile> {
        let mut f = std::fs::File::open(self.path(name)).ok()?;
        let mut d = vec![];
        let _ = std::io::Read::read_to_end(&mut f, &mut d);
        Some(InFile::new(d))
    }

    /// `fopen(name, "wb")`.
    fn open_out(&self, name: &[u8]) -> Option<Out> {
        std::fs::File::create(self.path(name))
            .ok()
            .map(|f| Out::File(f, Vec::new()))
    }

    /// `access(name, R_OK) == 0`.
    fn access_r(&self, name: &[u8]) -> bool {
        let p = self.path(name);
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let Ok(c) = std::ffi::CString::new(p.as_os_str().as_bytes()) else {
                return false;
            };
            // SAFETY: a NUL-terminated path.
            unsafe { libc::access(c.as_ptr(), libc::R_OK) == 0 }
        }
        #[cfg(not(unix))]
        {
            std::fs::File::open(p).is_ok()
        }
    }

    // ---- main -------------------------------------------------------------

    pub fn main(&mut self, args: &[Vec<u8>]) -> R<()> {
        let mut fns: Vec<Vec<u8>> = vec![];
        let mut use_stdin = false;
        let mut sty_given = false;
        let mut ind_given = false;
        let mut ilg_given = false;
        let mut log_given = false;

        // process command line options
        let mut argc = args.len() as i64 + 1;
        let mut av = 0usize;
        loop {
            argc -= 1;
            if argc <= 0 {
                break;
            }
            av += 1;
            let cur = args[av - 1].clone();
            if cur.first() == Some(&b'-') {
                if cur.len() == 1 {
                    break;
                }
                for &ap in &cur[1..] {
                    match ap {
                        b'i' => use_stdin = true,
                        b'l' => self.letter_ordering = true,
                        b'r' => self.merge_page = false,
                        b'q' => self.verbose = false,
                        b'c' => self.compress_blanks = true,
                        b's' => {
                            argc -= 1;
                            if argc <= 0 {
                                return Err(self.fatal(b"Expected -s <stylefile>\n"));
                            }
                            av += 1;
                            let a = args[av - 1].clone();
                            self.open_sty(&a)?;
                            sty_given = true;
                        }
                        b'o' => {
                            argc -= 1;
                            if argc <= 0 {
                                return Err(self.fatal(b"Expected -o <ind>\n"));
                            }
                            av += 1;
                            self.ind_fn = args[av - 1].clone();
                            ind_given = true;
                        }
                        b't' => {
                            argc -= 1;
                            if argc <= 0 {
                                return Err(self.fatal(b"Expected -t <logfile>\n"));
                            }
                            av += 1;
                            self.ilg_fn = args[av - 1].clone();
                            ilg_given = true;
                        }
                        b'p' => {
                            argc -= 1;
                            if argc <= 0 {
                                return Err(self.fatal(b"Expected -p <num>\n"));
                            }
                            av += 1;
                            if args[av - 1].len() >= NUMBER_MAX {
                                return Err(self.fatal(b"Page number too high\n"));
                            }
                            self.pageno = args[av - 1].clone();
                            self.init_page = true;
                            if self.pageno == b"even" {
                                log_given = true;
                                self.even_odd = 2;
                            } else if self.pageno == b"odd" {
                                log_given = true;
                                self.even_odd = 1;
                            } else if self.pageno == b"any" {
                                log_given = true;
                                self.even_odd = 0;
                            }
                        }
                        b'g' => self.german_sort = true,
                        b'L' => self.locale_sort = true,
                        b'T' => {
                            self.thai_sort = true;
                            self.locale_sort = true;
                        }
                        _ => {
                            let m = fmt("Unknown option -%c.\n", &[P::C(ap)]);
                            return Err(self.fatal(&m));
                        }
                    }
                }
            } else if self.fn_no < ARRAY_MAX as i32 {
                self.check_idx(&cur, false)?;
                self.fn_no += 1;
                fns.push(cur);
            } else {
                let m = fmt("Too many input files (max %d).\n", &[ARRAY_MAX.into()]);
                return Err(self.fatal(&m));
            }
        }

        if self.fn_no == 0 && !sty_given {
            // base set by last call to check_idx
            let mut tmp = self.base.clone();
            tmp.extend_from_slice(INDEX_STY);
            tmp.truncate(STRING_MAX + 4);
            if self.access_r(&tmp) {
                self.open_sty(&tmp)?;
                sty_given = true;
            }
        }

        self.process_idx(&fns, use_stdin, sty_given, ind_given, ilg_given, log_given)?;
        self.idx_gt = self.idx_tt - self.idx_et;
        // ALL_DONE
        if self.fn_no > 0 {
            let m = fmt(
                "Overall %d files read (%d entries accepted, %d rejected).\n",
                &[
                    (self.fn_no + 1).into(),
                    self.idx_gt.into(),
                    self.idx_et.into(),
                ],
            );
            if self.verbose {
                self.err(&m);
            }
            self.ilg(&m);
        }
        if self.idx_gt > 0 {
            self.prepare_idx()?;
            self.sort_idx();
            self.gen_ind()?;
            let m = fmt("Output written in %s.\n", &[P::S(&self.ind_fn.clone())]);
            self.message(&m);
        } else {
            let m = fmt("Nothing written in %s.\n", &[P::S(&self.ind_fn.clone())]);
            self.message(&m);
        }
        let m = fmt("Transcript written in %s.\n", &[P::S(&self.ilg_fn.clone())]);
        self.message(&m);
        Ok(())
    }

    fn prepare_idx(&mut self) -> R<()> {
        if self.nodes.is_empty() {
            return Err(self.fatal(b"No valid index entries collected.\n"));
        }
        let n = (self.idx_gt.max(0) as usize).min(self.nodes.len());
        self.idx_key = (0..n).collect();
        Ok(())
    }

    fn put_version(&mut self) {
        let m = fmt("This is %s, ", &[P::S(&self.pgm_fn.clone())]);
        self.message(&m);
        let m = fmt("%s.\n", &[P::S(VERSION)]);
        self.message(&m);
        self.need_version = false;
    }

    fn process_idx(
        &mut self,
        fns: &[Vec<u8>],
        mut use_stdin: bool,
        sty_given: bool,
        mut ind_given: bool,
        mut ilg_given: bool,
        log_given: bool,
    ) -> R<()> {
        if self.fn_no == -1 {
            // use stdin if no input files specified
            use_stdin = true;
        } else {
            self.check_all(&fns[0], ind_given, ilg_given, log_given)?;
            self.put_version();
            if sty_given {
                self.scan_sty();
            }
            if self.german_sort && self.idx_quote == b'"' {
                let m = fmt(
                    "Option -g invalid, quote character must be different from '%c'.\n",
                    &[P::C(b'"')],
                );
                return Err(self.fatal(&m));
            }
            self.scan_idx();
            ind_given = true;
            ilg_given = true;
            for f in fns.iter().take(self.fn_no as usize + 1).skip(1) {
                self.check_idx(f, true)?;
                self.scan_idx();
            }
        }

        if use_stdin {
            self.idx_fn = Rc::new(b"stdin".to_vec());
            let d = self.host.read_stdin();
            self.idx_fp = Some(InFile::new(d));

            if ind_given {
                let name = self.ind_fn.clone();
                let ok = self.host.out_name_ok(&name);
                if !ok
                    || (self.ind_fp.is_none() && {
                        self.ind_fp = self.open_out(&name);
                        self.ind_fp.is_none()
                    })
                {
                    let m = fmt("Can't create output index file %s.\n", &[P::S(&name)]);
                    return Err(self.fatal(&m));
                }
            } else {
                self.ind_fn = b"stdout".to_vec();
                self.ind_fp = Some(Out::Stdout);
            }

            if ilg_given {
                let name = self.ilg_fn.clone();
                let ok = self.host.out_name_ok(&name);
                if !ok
                    || (self.ilg_fp.is_none() && {
                        self.ilg_fp = self.open_out(&name);
                        self.ilg_fp.is_none()
                    })
                {
                    let m = fmt("Can't create transcript file %s.\n", &[P::S(&name)]);
                    return Err(self.fatal(&m));
                }
            } else {
                self.ilg_fn = b"stderr".to_vec();
                self.ilg_fp = Some(Out::Stderr);
                // This is already equivalent to verbose mode...
                self.verbose = false;
            }

            if self.fn_no == -1 && sty_given {
                self.scan_sty();
            }
            if self.german_sort && self.idx_quote == b'"' {
                let m = fmt(
                    "Option -g ignored, quote character must be different from '%c'.\n",
                    &[P::C(b'"')],
                );
                return Err(self.fatal(&m));
            }

            if self.need_version {
                self.put_version();
            }
            self.scan_idx();
            self.fn_no += 1;
        }
        Ok(())
    }

    fn check_idx(&mut self, fn_: &[u8], open_fn: bool) -> R<()> {
        let ext = fn_.iter().rposition(|&b| b == b'.');
        let is_delim = |b: &u8| *b == b'/' || (cfg!(windows) && *b == b'\\');
        let with_ext = match ext {
            Some(e) => e != 0 && !fn_[e + 1..].iter().any(is_delim),
            None => false,
        };
        let stem = if with_ext { &fn_[..ext.unwrap()] } else { fn_ };
        if stem.len() < STRING_MAX {
            self.base = stem.to_vec();
        } else {
            self.base = stem[..STRING_MAX].to_vec();
            let m = fmt(
                "Index file name %s too long (max %d).\n",
                &[P::S(&self.base.clone()), STRING_MAX.into()],
            );
            return Err(self.fatal(&m));
        }

        self.idx_fn = Rc::new(fn_.to_vec());

        if self.idx_fails(fn_, open_fn) {
            if with_ext {
                let m = fmt("Input index file %s not found.\n", &[P::S(fn_)]);
                return Err(self.fatal(&m));
            } else {
                let mut tmp = self.base.clone();
                tmp.extend_from_slice(INDEX_IDX);
                self.idx_fn = Rc::new(tmp.clone());
                if self.idx_fails(&tmp, open_fn) {
                    let m = fmt(
                        "Couldn't find input index file %s nor %s.\n",
                        &[P::S(&self.base.clone()), P::S(&tmp)],
                    );
                    return Err(self.fatal(&m));
                }
            }
        }
        Ok(())
    }

    /// The test of `check_idx`: with `open_fn`, open the file as
    /// `idx_fp`; without, `access` it.
    fn idx_fails(&mut self, name: &[u8], open_fn: bool) -> bool {
        if open_fn {
            if !self.host.in_name_ok(name) {
                return true;
            }
            self.idx_fp = self.open_in(name);
            self.idx_fp.is_none()
        } else {
            !self.access_r(name)
        }
    }

    fn check_all(
        &mut self,
        fn_: &[u8],
        ind_given: bool,
        ilg_given: bool,
        log_given: bool,
    ) -> R<()> {
        self.check_idx(fn_, true)?;

        // index output file
        if !ind_given {
            self.ind_fn = self.base.clone();
            self.ind_fn.extend_from_slice(INDEX_IND);
            self.ind_fn.truncate(STRING_MAX - 1);
        }
        let name = self.ind_fn.clone();
        if !self.host.out_name_ok(&name) || {
            self.ind_fp = self.open_out(&name);
            self.ind_fp.is_none()
        } {
            let m = fmt("Can't create output index file %s.\n", &[P::S(&name)]);
            return Err(self.fatal(&m));
        }

        // index transcript file
        if !ilg_given {
            self.ilg_fn = self.base.clone();
            self.ilg_fn.extend_from_slice(INDEX_ILG);
            self.ilg_fn.truncate(STRING_MAX - 1);
        }
        let name = self.ilg_fn.clone();
        if !self.host.out_name_ok(&name) || {
            self.ilg_fp = self.open_out(&name);
            self.ilg_fp.is_none()
        } {
            let m = fmt("Can't create transcript file %s.\n", &[P::S(&name)]);
            return Err(self.fatal(&m));
        }

        if log_given {
            self.log_fn = self.base.clone();
            self.log_fn.extend_from_slice(INDEX_LOG);
            self.log_fn.truncate(STRING_MAX - 1);
            let name = self.log_fn.clone();
            if !self.host.in_name_ok(&name) || {
                self.log_fp = self.open_in(&name);
                self.log_fp.is_none()
            } {
                let m = fmt("Source log file %s not found.\n", &[P::S(&name)]);
                return Err(self.fatal(&m));
            } else {
                self.find_pageno()?;
                self.log_fp = None;
            }
        }
        Ok(())
    }

    fn log_getc(&mut self) -> i32 {
        mk_getc(&mut self.lookahead, self.log_fp.as_mut().unwrap())
    }

    fn find_pageno(&mut self) -> R<()> {
        let mut pageno: Vec<u8> = vec![];
        self.log_fp.as_mut().unwrap().fseek(-1, 2);
        let mut p = self.log_getc();
        self.log_fp.as_mut().unwrap().fseek(-2, 1);
        let mut c;
        // This backward scan never ends on some logs (one ending in CR LF:
        // reading the CR also reads the LF, and the seek back returns to the
        // CR). Its state is (position, lookahead, p), so a repeated state
        // proves it; the C program then runs forever and the port stops.
        let mut seen = std::collections::HashSet::new();
        loop {
            if !seen.insert((self.log_fp.as_ref().unwrap().pos(), self.lookahead, p)) {
                return Err(Exit(crate::LOOPS_FOREVER));
            }
            c = p;
            p = self.log_getc();
            if (p == b'[' as i32 && c_isdigit(c)) || self.log_fp.as_mut().unwrap().fseek(-2, 1) != 0
            {
                break;
            }
        }
        if p == b'[' as i32 {
            loop {
                c = self.log_getc();
                if c != SPC {
                    break;
                }
            }
            loop {
                pageno.push(c as u8);
                c = self.log_getc();
                if !c_isdigit(c) {
                    break;
                }
            }
            self.pageno = pageno;
        } else {
            let m = fmt(
                "Couldn't find any page number in %s...ignored\n",
                &[P::S(&self.log_fn.clone())],
            );
            self.ilg(&m);
            self.init_page = false;
        }
        Ok(())
    }

    fn open_sty(&mut self, fn_: &[u8]) -> R<()> {
        match self.host.find_ist(fn_) {
            None => {
                let m = fmt("Index style file %s not found.\n", &[P::S(fn_)]);
                Err(self.fatal(&m))
            }
            Some(found) => {
                if found.len() >= STRING_MAX {
                    let m = fmt("Style file name %s too long.\n", &[P::S(&found)]);
                    return Err(self.fatal(&m));
                }
                self.sty_fn = found.clone();
                if !self.host.in_name_ok(&found) || {
                    self.sty_fp = self.open_in(&found);
                    self.sty_fp.is_none()
                } {
                    let m = fmt("Could not open style file %s.\n", &[P::S(&found)]);
                    return Err(self.fatal(&m));
                }
                Ok(())
            }
        }
    }
}

/// `strtoint` (mkind.c), with C's wrap-around.
pub fn strtoint(s: &[u8]) -> i32 {
    let mut val: i32 = 0;
    for &b in cstr(s) {
        val = val.wrapping_mul(10).wrapping_add(b as i8 as i32 - 48);
    }
    val
}

#[allow(dead_code)]
const _EOF: i32 = EOF;
