//! The system-dependent part of the XeTeX port: what web2c's `tex.ch`,
//! TeX Live's `xetexdir/xetex.ch`, `texmfmp.c` and `XeTeX_ext.c` supply.
//!
//! It holds
//!
//!   * Pascal's file model for the three kinds of file the generated code
//!     uses (`AlphaFile` is also XeTeX's `UFILE`, the Unicode input file);
//!   * the routines that open, close and name files (`a_open_in` ...,
//!     `u_open_in`, `dvi_open_out`), with kpathsea through the pdfTeX
//!     engine's resolver;
//!   * `input_ln`, which is `XeTeX_ext.c`'s `input_line`: a line of UTF-8,
//!     UTF-16 or bytes becomes Unicode scalars in `buffer`;
//!   * `init_terminal` (texmfmp.c's `topenin`: the command line, UTF-8
//!     decoded, is the first line);
//!   * the routines `changes/web2c-run.ch`, `changes/virtex.ch` and
//!     `changes/filenames.ch` declare, and the ways out of the program.
//!
//! The run's configuration (the command line and texmf.cnf) is the pdfTeX
//! engine's `flashtex_engine::system::configure`, read with `run()`.

use crate::generated::types::memory_word;
use crate::generated::Globals;
use flashtex_engine::resolver::Format;
pub use flashtex_engine::system::{
    invocation_name, run, setup_bound_var, texmf_var, texmf_yesno, Run, WEB2C_VERSION,
};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;

// ---------------------------------------------------------------------------
// XeTeX's input encodings (xetex.h)
// ---------------------------------------------------------------------------

/// `AUTO`: sniff the encoding when the file is opened.
pub const AUTO: i32 = 0;
pub const UTF8: i32 = 1;
pub const UTF16BE: i32 = 2;
pub const UTF16LE: i32 = 3;
pub const RAW: i32 = 4;
/// An ICU converter; not available in phase S0 (see `set_input_file_encoding`).
pub const ICUMAPPING: i32 = 5;

// ---------------------------------------------------------------------------
// Pascal text files, which are XeTeX's Unicode files when read
// ---------------------------------------------------------------------------

enum TextIn {
    Stdin,
    File(BufReader<File>),
    /// `\input|command` (texmfmp.c's `open_in_or_pipe`).
    Pipe(BufReader<std::process::ChildStdout>),
}

/// `packed file of char`, and XeTeX's `unicode_file` (a `UFILE`): text
/// written byte by byte, or read a line at a time by `input_ln` in the
/// file's encoding.
pub struct AlphaFile {
    /// Pascal's buffer variable `f^` (only the debugging code reads it).
    pub buf: u8,
    input: Option<TextIn>,
    output: Option<BufWriter<Box<dyn Write + Send>>>,
    /// The command of a pipe, waited for when the file is closed.
    child: Option<std::process::Child>,
    to_stdout: bool,
    /// Pascal's `erstat`: 0 means the last open succeeded.
    err: i32,
    /// The path opened.
    path: Option<String>,
    /// `UFILE.encodingMode`.
    pub encoding_mode: i32,
    /// `UFILE.savedChar`: a UTF-16 low half read too early, or -1.
    saved_char: i32,
    /// `UFILE.skipNextLF`: the last line ended with a CR.
    skip_next_lf: bool,
    /// C's `ungetc`: one byte pushed back.
    pushback: Option<u8>,
    /// The end of the file was seen (Pascal's `eof`).
    at_eof: bool,
}

impl Default for AlphaFile {
    fn default() -> AlphaFile {
        AlphaFile {
            buf: b' ',
            input: None,
            output: None,
            child: None,
            to_stdout: false,
            err: 0,
            path: None,
            encoding_mode: UTF8,
            saved_char: -1,
            skip_next_lf: false,
            pushback: None,
            at_eof: true,
        }
    }
}

impl AlphaFile {
    fn put_byte(&mut self, b: u8) {
        if self.to_stdout {
            let _ = std::io::stdout().write_all(&[b]);
        } else if let Some(w) = self.output.as_mut() {
            let _ = w.write_all(&[b]);
        }
    }

    /// C's `getc`: the next byte, or -1 at the end of the file.
    fn getc(&mut self) -> i32 {
        if let Some(b) = self.pushback.take() {
            return b as i32;
        }
        let mut one = [0u8; 1];
        let r = match self.input.as_mut() {
            Some(TextIn::Stdin) => std::io::stdin().lock().read(&mut one),
            Some(TextIn::File(r)) => r.read(&mut one),
            Some(TextIn::Pipe(r)) => r.read(&mut one),
            None => Ok(0),
        };
        match r {
            Ok(1) => one[0] as i32,
            _ => {
                self.at_eof = true;
                -1
            }
        }
    }

    /// C's `ungetc`.
    fn ungetc(&mut self, c: i32) {
        if c >= 0 {
            self.pushback = Some(c as u8);
            self.at_eof = false;
        }
    }

    /// `rewind`: back to the first byte of a file.
    fn rewind(&mut self) {
        use std::io::Seek;
        self.pushback = None;
        if let Some(TextIn::File(r)) = self.input.as_mut() {
            let _ = r.seek(std::io::SeekFrom::Start(0));
            self.at_eof = false;
        }
    }
}

/// `packed file of eight_bits`.
#[derive(Default)]
pub struct ByteFile {
    pub buf: i32,
    input: Option<BufReader<File>>,
    output: Option<BufWriter<File>>,
    at_eof: bool,
    err: i32,
    path: Option<String>,
    /// Bytes written, for `dvi_close`'s error report.
    written: u64,
}

/// `file of memory_word`.
#[derive(Default)]
pub struct WordFile {
    pub buf: memory_word,
    input: Option<BufReader<File>>,
    output: Option<BufWriter<File>>,
    at_eof: bool,
    err: i32,
    path: Option<String>,
}

// ---------------------------------------------------------------------------
// The Pascal built-ins the generated code calls
// ---------------------------------------------------------------------------

pub trait PasFile {
    fn erstat(&self) -> i32;
    fn at_eof(&self) -> bool;
    fn flush(&mut self);
    fn close(&mut self);
}

impl PasFile for AlphaFile {
    fn erstat(&self) -> i32 {
        self.err
    }
    fn at_eof(&self) -> bool {
        self.at_eof
    }
    fn flush(&mut self) {
        if self.to_stdout {
            let _ = std::io::stdout().flush();
        } else if let Some(w) = self.output.as_mut() {
            let _ = w.flush();
        }
    }
    fn close(&mut self) {
        self.flush();
        self.output = None;
        self.input = None;
        self.path = None;
        self.at_eof = true;
        if let Some(mut c) = self.child.take() {
            let _ = c.wait();
        }
    }
}
impl PasFile for ByteFile {
    fn erstat(&self) -> i32 {
        self.err
    }
    fn at_eof(&self) -> bool {
        self.at_eof
    }
    fn flush(&mut self) {
        if let Some(w) = self.output.as_mut() {
            let _ = w.flush();
        }
    }
    fn close(&mut self) {
        self.flush();
        self.output = None;
        self.input = None;
    }
}
impl PasFile for WordFile {
    fn erstat(&self) -> i32 {
        self.err
    }
    fn at_eof(&self) -> bool {
        self.at_eof
    }
    fn flush(&mut self) {
        if let Some(w) = self.output.as_mut() {
            let _ = w.flush();
        }
    }
    fn close(&mut self) {
        self.flush();
        self.output = None;
        self.input = None;
    }
}

pub fn erstat<F: PasFile>(f: &F) -> i32 {
    f.erstat()
}
pub fn eof<F: PasFile>(f: &F) -> bool {
    f.at_eof()
}
pub fn close<F: PasFile>(f: &mut F) {
    f.close();
}
pub fn break_out<F: PasFile>(f: &mut F) {
    f.flush();
}
pub fn break_in(_f: &mut AlphaFile, _discard: bool) {}

pub fn get_byte(f: &mut ByteFile) {
    let mut b = [0u8; 1];
    match f.input.as_mut().map(|r| r.read_exact(&mut b)) {
        Some(Ok(())) => f.buf = b[0] as i32,
        _ => {
            f.at_eof = true;
            f.buf = 0;
        }
    }
}
pub fn put_byte(f: &mut ByteFile) {
    let v = f.buf;
    write_byte(f, v);
}
pub fn write_byte(f: &mut ByteFile, v: i32) {
    if let Some(w) = f.output.as_mut() {
        if w.write_all(&[v as u8]).is_ok() {
            f.written += 1;
        }
    }
}
pub fn read_byte(f: &mut ByteFile) -> i32 {
    get_byte(f);
    f.buf
}

pub fn get_word(f: &mut WordFile) {
    let mut b = [0u8; 8];
    match f.input.as_mut().map(|r| r.read_exact(&mut b)) {
        Some(Ok(())) => f.buf = memory_word::from_bits(u64::from_le_bytes(b)),
        _ => {
            f.at_eof = true;
            f.buf = memory_word::default();
        }
    }
}
pub fn put_word(f: &mut WordFile) {
    let v = f.buf;
    write_word(f, v);
}
pub fn write_word(f: &mut WordFile, v: memory_word) {
    if let Some(w) = f.output.as_mut() {
        let _ = w.write_all(&v.to_bits().to_le_bytes());
    }
}
pub fn read_word(f: &mut WordFile) -> memory_word {
    get_word(f);
    f.buf
}

pub fn wr_char(f: &mut AlphaFile, c: u8) {
    f.put_byte(c);
}
pub fn wr_str(f: &mut AlphaFile, s: &str) {
    for b in s.bytes() {
        f.put_byte(b);
    }
}
pub fn wr_ln(f: &mut AlphaFile) {
    f.put_byte(b'\n');
    if f.to_stdout {
        f.flush();
    }
}
pub fn wr_bool(f: &mut AlphaFile, v: bool) {
    wr_str(f, if v { "TRUE" } else { "FALSE" });
}
pub fn wr_int(f: &mut AlphaFile, v: i32, width: i32) {
    let s = v.to_string();
    for _ in s.len() as i32..width {
        f.put_byte(b' ');
    }
    wr_str(f, &s);
}
pub fn wr_real(f: &mut AlphaFile, v: f64, width: i32) {
    let s = format!("{v}");
    for _ in s.len() as i32..width {
        f.put_byte(b' ');
    }
    wr_str(f, &s);
}

/// Assign a Pascal string to a `packed array [1..n] of char`, blank-padded.
pub fn copy_str<const N: usize>(dst: &mut [u8; N], s: &str) {
    dst.fill(b' ');
    for (i, b) in s.bytes().take(N).enumerate() {
        dst[i] = b;
    }
}

/// Pascal's `round` as web2c compiles it (`zround`: add 0.5 and truncate,
/// clamped); the pdfTeX engine's `pas_round` explains why.
pub fn pas_round(r: f64) -> i32 {
    if r > 2147483647.0 {
        2147483647
    } else if r < -2147483647.0 {
        -2147483647
    } else if r >= 0.0 {
        (r + 0.5) as i32
    } else {
        (r - 0.5) as i32
    }
}

/// `t_open_out` — `rewrite(term_out,'TTY:','/O')`.
pub fn rewrite_char(f: &mut AlphaFile, _name: &str, _mode: &str) {
    *f = AlphaFile::default();
    f.to_stdout = true;
    f.err = 0;
}

/// What a checkpoint would visit in each file global (the generated
/// `Globals::visit_files`); this crate takes no checkpoints.
pub trait FileVisit {
    fn alpha(&mut self, f: &mut AlphaFile);
    fn byte(&mut self, f: &mut ByteFile);
    fn word(&mut self, f: &mut WordFile);
}

// ---------------------------------------------------------------------------
// The run
// ---------------------------------------------------------------------------

/// The command line after the options, the first line of input
/// (`Host::first_line`).
pub fn set_first_line(g: &mut Globals, line: Vec<u8>) {
    g.host.first_line = Some(line);
}

/// `-no-pdf`: write the `.xdv` file and run no output driver. Phases S0-S1
/// always write XDV (a driver is phase S2), so this is the default.
pub fn set_no_pdf(g: &mut Globals, on: bool) {
    g.host.no_pdf.0 = on;
}

/// XeTeX's `BANNER` (xetexextra.h), the same text as xetex.web's banner.
pub const BANNER: &str = "This is XeTeX, Version 3.141592653-2.6-0.999998";

/// `-version`.
pub fn version_text() -> String {
    "XeTeX 3.141592653-2.6-0.999998 (FlashTeX XeTeX port, phase S0)\n\
     FlashTeX's XeTeX port is a translation of XeTeX 0.999998's xetex.web\n\
     (TeX Live 2026) by tools/web2rust, with kpathsea from TeX Live.\n\
     Copyright 2026 SIL International, Jonathan Kew and Khaled Hosny.\n\
     There is NO warranty.  Redistribution of this software is\n\
     covered by the terms of the GNU General Public License,\n\
     version 2 or (at your option) any later version.\n"
        .to_string()
}

/// The program's end (tex.ch's `do_final_end`): the exit status says
/// whether there was an error.
pub fn final_end(g: &mut Globals) -> ! {
    g.ready_already = 0;
    let code = if g.history > 1 { 1 } else { 0 };
    exit_process(g, code)
}

/// C's `exit(code)`: every stdio stream is flushed first.
pub fn exit_process(g: &mut Globals, code: i32) -> ! {
    let _ = std::io::stdout().flush();
    g.log_file.flush();
    for f in g.write_file.iter_mut() {
        f.flush();
    }
    g.dvi_file.flush();
    std::process::exit(code)
}

/// `goto end_of_TEX` (tex.web §1332's `final_cleanup` path): close the files
/// and stop.
#[allow(non_snake_case)]
pub fn end_of_TEX(g: &mut Globals) -> ! {
    if !std::mem::replace(&mut g.host.terminating, true) {
        g.close_files_and_terminate();
    }
    final_end(g)
}

/// The string pool web2rust wrote for xetex.web (`xetex.pool`), compiled
/// in: TeX Live's xetex carries its pool too (`tex-binpool.ch`).
const POOL: &str = include_str!("../xetex.pool");

/// The decoding of a UTF-8 byte sequence's first byte: how many bytes follow
/// it (ConvertUTF.c's `bytesFromUTF8`, which XeTeX_ext.c uses).
fn bytes_from_utf8(b: u32) -> u32 {
    match b {
        0x00..=0xBF => 0,
        0xC0..=0xDF => 1,
        0xE0..=0xEF => 2,
        0xF0..=0xF7 => 3,
        0xF8..=0xFB => 4,
        _ => 5,
    }
}

/// ConvertUTF.c's `offsetsFromUTF8`.
const OFFSETS_FROM_UTF8: [u32; 6] = [
    0x00000000, 0x00003080, 0x000E2080, 0x03C82080, 0xFA082080, 0x82082080,
];

/// texmfmp.c's `topenin` and XeTeX_ext.c's `makeutf16name` decode UTF-8 so:
/// no check of the continuation bytes (a missing one is a 0).
fn decode_utf8_lenient(s: &[u8]) -> Vec<u32> {
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let mut rval = s[i] as u32;
        i += 1;
        let extra = bytes_from_utf8(rval);
        for _ in 0..extra {
            rval <<= 6;
            if i < s.len() {
                rval = rval.wrapping_add(s[i] as u32);
                i += 1;
            }
        }
        out.push(rval.wrapping_sub(OFFSETS_FROM_UTF8[extra as usize]));
    }
    out
}

/// xetex.ch [29.519]'s `append_to_name` for one UTF-16 code unit (`k`, the
/// position, is that of the last byte written), used where a string becomes a
/// C string outside the generated code (`runsystem`'s command).
fn utf16_to_name(units: &[i32]) -> Vec<u8> {
    let mut n: Vec<u8> = vec![];
    for &c in units {
        let c = c as i64;
        if c < 0x80 {
            n.push(c as u8);
        } else if c < 0x800 {
            n.push((0xC0 + c / 0x40) as u8);
            n.push((0x80 + c % 0x40) as u8);
        } else if c < 0xD800 {
            n.push((0xE0 + c / 0x1000) as u8);
            n.push((0x80 + (c % 0x1000) / 0x40) as u8);
            n.push((0x80 + c % 0x40) as u8);
        } else if c < 0xDC00 {
            n.push((0xF0 + (c - 0xD7C0) / 0x1000) as u8);
            n.push((0x80 + ((c - 0xD7C0) % 0x1000) / 0x4) as u8);
            n.push((0x80 + (c - 0xD7C0) % 0x4 * 0x10) as u8);
            n.push(0x80);
        } else if c < 0xE000 && n.len() >= 4 {
            let k = n.len();
            n[k - 2] = n[k - 2].wrapping_add(((c - 0xDC00) / 0x40) as u8);
            n[k - 1] = n[k - 1].wrapping_add(((c - 0xDC00) % 0x40) as u8);
        } else if c < 0x10000 {
            n.push((0xE0 + c / 0x1000) as u8);
            n.push((0x80 + (c % 0x1000) / 0x40) as u8);
            n.push((0x80 + c % 0x40) as u8);
        } else {
            n.extend_from_slice(&[0xEF, 0xBF, 0xBD]);
        }
    }
    n
}

/// The run's file lookup: `kpse_find_file(name, format, must_exist)` the way
/// web2c's `open_input` asks it, through the pdfTeX engine's resolver.
fn lookup(name: &str, format: Format, must_exist: bool) -> Option<String> {
    if format == Format::Tex && must_exist {
        flashtex_engine::system::find_input(name)
    } else {
        flashtex_engine::system::find_file(name, format)
    }
}

impl Globals {
    /// `name_of_file[1..name_length]`, the UTF-8 bytes of a file name.
    fn raw_name_bytes(&self) -> Vec<u8> {
        let n = (self.name_length.max(0) as usize).min(self.name_of_file.len());
        self.name_of_file[..n].to_vec()
    }

    /// The same as a `String` (a name that is not UTF-8 is read lossily).
    pub(crate) fn raw_file_name(&self) -> String {
        String::from_utf8_lossy(&self.raw_name_bytes()).into_owned()
    }

    /// Replace `name_of_file` by `name` (UTF-8), as web2c does after opening
    /// a file, with a 0 after it (xetex.ch's C string).
    fn set_name_of_file(&mut self, name: &[u8]) {
        let n = name.len();
        if n <= self.name_of_file.len() {
            self.name_of_file.fill(b' ');
            self.name_of_file[..n].copy_from_slice(name);
            if n < self.name_of_file.len() {
                self.name_of_file[n] = 0;
            }
            self.name_length = n as i32;
        }
    }

    /// Where to open an input file (lib/openclose.c's `open_input`): a
    /// relative name in `-output-directory` first, else looked up by the
    /// resolver. The path found is written back into `name_of_file` without
    /// the `./` kpathsea puts in front of a file in the current directory
    /// (unless the name asked for had it), as openclose.c does;
    /// `fullnameoffile` keeps it.
    fn input_path(&mut self, format: Format, must_exist: bool) -> Option<String> {
        self.host.full_name_of_file = None;
        let s = self.raw_file_name();
        let mut found = None;
        if let Some(dir) = run().output_directory {
            if !s.starts_with('/') {
                let p = format!("{dir}/{s}");
                if Path::new(&p).is_file() {
                    found = Some(p);
                }
            }
        }
        let (found, searched) = match found {
            Some(p) => (p, false),
            None => (lookup(&s, format, must_exist)?, true),
        };
        self.host.full_name_of_file = Some(found.clone());
        let shown = match found.strip_prefix("./") {
            Some(rest) if searched && !rest.is_empty() && !s.starts_with("./") => rest,
            _ => found.as_str(),
        };
        let shown = shown.as_bytes().to_vec();
        self.set_name_of_file(&shown);
        Some(found)
    }

    /// lib/openclose.c's `open_output`: a relative name goes into
    /// `-output-directory`; if it cannot be created there, into texmf.cnf's
    /// `TEXMFOUTPUT`. The name opened is written back into `name_of_file`.
    fn open_output_file(&mut self) -> Option<File> {
        let name = self.raw_file_name();
        let absolute = name.starts_with('/');
        let mut fname = name.clone();
        if let Some(dir) = run().output_directory {
            if !absolute {
                fname = format!("{dir}/{name}");
            }
        }
        let mut f = File::create(&fname).ok();
        if f.is_none() && !absolute {
            if let Some(out) = texmf_var("TEXMFOUTPUT").filter(|v| !v.is_empty()) {
                fname = format!("{out}/{name}");
                f = File::create(&fname).ok();
            }
        }
        if f.is_some() && fname != name {
            let b = fname.as_bytes().to_vec();
            self.set_name_of_file(&b);
        }
        f
    }

    // ---- the overrides of tools/web2rust (tex.web §§27-28, 31, 37) -------

    pub fn a_open_in(&mut self, f: &mut AlphaFile) -> bool {
        self.u_open_in(f, crate::generated::consts::kpse_tex_format, RAW, 0)
    }

    pub fn a_open_out(&mut self, f: &mut AlphaFile) -> bool {
        *f = AlphaFile::default();
        f.err = 1;
        match self.open_output_file() {
            Some(h) => {
                f.output = Some(BufWriter::new(Box::new(h)));
                f.path = Some(self.raw_file_name());
                f.err = 0;
                true
            }
            None => false,
        }
    }

    pub fn b_open_in(&mut self, f: &mut ByteFile) -> bool {
        *f = ByteFile::default();
        f.err = 1;
        let Some(name) = self.input_path(Format::Tfm, true) else {
            return false;
        };
        match File::open(&name) {
            Ok(h) => {
                f.input = Some(BufReader::new(h));
                f.path = Some(name);
                f.err = 0;
                // Pascal's `reset` leaves `f^` holding the first component.
                get_byte(f);
                true
            }
            Err(_) => false,
        }
    }

    pub fn b_open_out(&mut self, f: &mut ByteFile) -> bool {
        *f = ByteFile::default();
        match self.open_output_file() {
            Some(h) => {
                f.output = Some(BufWriter::new(h));
                f.path = Some(self.raw_file_name());
                f.err = 0;
                true
            }
            None => {
                f.err = 1;
                false
            }
        }
    }

    pub fn w_open_in(&mut self, f: &mut WordFile) -> bool {
        *f = WordFile::default();
        f.err = 1;
        let Some(name) = self.input_path(Format::Fmt, true) else {
            return false;
        };
        match File::open(&name) {
            Ok(h) => {
                f.input = Some(BufReader::new(h));
                f.path = Some(name);
                f.err = 0;
                get_word(f);
                true
            }
            Err(_) => false,
        }
    }

    pub fn w_open_out(&mut self, f: &mut WordFile) -> bool {
        *f = WordFile::default();
        match self.open_output_file() {
            Some(h) => {
                f.output = Some(BufWriter::new(h));
                f.path = Some(self.raw_file_name());
                f.err = 0;
                true
            }
            None => {
                f.err = 1;
                false
            }
        }
    }

    pub fn a_close(&mut self, f: &mut AlphaFile) {
        f.close();
    }
    pub fn b_close(&mut self, f: &mut ByteFile) {
        f.close();
    }
    pub fn w_close(&mut self, f: &mut WordFile) {
        f.close();
    }

    pub fn a_make_name_string(&mut self, _f: &mut AlphaFile) -> i32 {
        self.make_name_string()
    }
    pub fn b_make_name_string(&mut self, _f: &mut ByteFile) -> i32 {
        self.make_name_string()
    }
    pub fn w_make_name_string(&mut self, _f: &mut WordFile) -> i32 {
        self.make_name_string()
    }

    /// XeTeX_ext.c's `get_uni_c`: the next character of a Unicode file in
    /// its encoding, or -1 at the end of the file. A bad UTF-8 sequence is
    /// reported (`bad_utf8_warning`) and read as U+FFFD.
    fn get_uni_c(&mut self, f: &mut AlphaFile) -> i32 {
        if f.saved_char != -1 {
            let r = f.saved_char;
            f.saved_char = -1;
            return r;
        }
        match f.encoding_mode {
            UTF8 => {
                let first = f.getc();
                if first == -1 {
                    return -1;
                }
                let mut rval = first as u32;
                let extra = bytes_from_utf8(rval);
                if extra >= 4 {
                    self.bad_utf8_warning();
                    return 0xFFFD;
                }
                for _ in 0..extra {
                    let c = f.getc();
                    if !(0x80..0xC0).contains(&c) {
                        if c != -1 {
                            f.ungetc(c);
                        }
                        self.bad_utf8_warning();
                        return 0xFFFD;
                    }
                    rval = (rval << 6).wrapping_add(c as u32);
                }
                let r = rval.wrapping_sub(OFFSETS_FROM_UTF8[extra as usize]) as i32;
                if !(0..=0x10FFFF).contains(&r) {
                    self.bad_utf8_warning();
                    return 0xFFFD;
                }
                r
            }
            UTF16BE | UTF16LE => {
                let be = f.encoding_mode == UTF16BE;
                let unit = |f: &mut AlphaFile| -> i32 {
                    let a = f.getc();
                    if a == -1 {
                        return -1;
                    }
                    let b = f.getc();
                    // C adds getc's -1 at the end of the file, as here.
                    if be {
                        (a << 8) + b
                    } else {
                        a + (b << 8)
                    }
                };
                let mut rval = unit(f);
                if rval == -1 {
                    return -1;
                }
                if (0xD800..=0xDBFF).contains(&rval) {
                    let lo = unit(f);
                    if (0xDC00..=0xDFFF).contains(&lo) {
                        rval = 0x10000 + (rval - 0xD800) * 0x400 + (lo - 0xDC00);
                    } else {
                        rval = 0xFFFD;
                        f.saved_char = lo;
                    }
                } else if (0xDC00..=0xDFFF).contains(&rval) {
                    rval = 0xFFFD;
                }
                rval
            }
            // RAW, and (never in S0) ICUMAPPING
            _ => f.getc(),
        }
    }

    /// `tex.web` §31's `input_ln` as XeTeX_ext.c's `input_line` does it: a
    /// line of the file, up to a LF or a CR (a LF right after a CR belongs
    /// to the line before), into `buffer[first..last-1]` as Unicode scalars,
    /// with trailing spaces (and CRs and LFs) removed; false at the end of
    /// the file. `bypass_eoln` is not needed: lines are read whole.
    pub fn input_ln(&mut self, f: &mut AlphaFile, _bypass_eoln: bool) -> bool {
        let buf_size = crate::generated::consts::buf_size;
        self.last = self.first;
        let mut i = self.get_uni_c(f);
        if f.skip_next_lf {
            f.skip_next_lf = false;
            if i == b'\n' as i32 {
                i = self.get_uni_c(f);
            }
        }
        if self.last < buf_size && i != -1 && i != b'\n' as i32 && i != b'\r' as i32 {
            self.buffer[self.last as usize] = i;
            self.last += 1;
        }
        if i != -1 && i != b'\n' as i32 && i != b'\r' as i32 {
            while self.last < buf_size {
                i = self.get_uni_c(f);
                if i == -1 || i == b'\n' as i32 || i == b'\r' as i32 {
                    break;
                }
                self.buffer[self.last as usize] = i;
                self.last += 1;
            }
        }
        if i == -1 && self.last == self.first {
            return false;
        }
        if i != -1 && i != b'\n' as i32 && i != b'\r' as i32 {
            // XeTeX_ext.c's `buffer_overflow`.
            eprintln!("! Unable to read an entire line---bufsize={buf_size}.");
            eprintln!("Please increase buf_size in texmf.cnf.");
            exit_process(self, 1);
        }
        if i == b'\r' as i32 {
            f.skip_next_lf = true;
        }
        self.buffer[self.last as usize] = b' ' as i32;
        if self.last >= self.max_buf_stack {
            self.max_buf_stack = self.last;
        }
        while self.last > self.first {
            let c = self.buffer[(self.last - 1) as usize];
            if c == b' ' as i32 || c == b'\r' as i32 || c == b'\n' as i32 {
                self.last -= 1;
            } else {
                break;
            }
        }
        true
    }

    /// `tex.web` §37 with texmfmp.c's `topenin`: the command line, decoded
    /// from UTF-8, is the first line; else the terminal is asked.
    pub fn init_terminal(&mut self) -> bool {
        let mut term = AlphaFile {
            input: Some(TextIn::Stdin),
            at_eof: false,
            encoding_mode: UTF8,
            ..Default::default()
        };
        self.buffer[self.first as usize] = 0;
        if let Some(line) = self.host.first_line.take() {
            let mut k = self.first;
            for c in decode_utf8_lenient(&line) {
                self.buffer[k as usize] = c as i32;
                k += 1;
            }
            self.buffer[k as usize] = 0;
        }
        self.last = self.first;
        while self.buffer[self.last as usize] != 0 {
            self.last += 1;
        }
        // One past the last non-space character, ignoring line terminators.
        while self.last > self.first {
            let c = self.buffer[(self.last - 1) as usize];
            if c == b' ' as i32 || c == b'\r' as i32 || c == b'\n' as i32 {
                self.last -= 1;
            } else {
                break;
            }
        }
        let r = loop {
            if self.last > self.first {
                self.cur_input.loc_field = self.first;
                while self.cur_input.loc_field < self.last
                    && self.buffer[self.cur_input.loc_field as usize] == b' ' as i32
                {
                    self.cur_input.loc_field += 1;
                }
                if self.cur_input.loc_field < self.last {
                    break true;
                }
            }
            print!("**");
            let _ = std::io::stdout().flush();
            if !self.input_ln(&mut term, true) {
                println!();
                println!("! End of file on the terminal... why?");
                break false;
            }
            self.cur_input.loc_field = self.first;
            while self.cur_input.loc_field < self.last
                && self.buffer[self.cur_input.loc_field as usize] == b' ' as i32
            {
                self.cur_input.loc_field += 1;
            }
            if self.cur_input.loc_field < self.last {
                break true;
            }
            println!("Please type the name of your input file.");
            self.last = self.first;
        };
        self.term_in = term;
        r
    }

    // ---- XeTeX_ext.c's Unicode files (changes/ext.ch) ---------------------

    /// `u_open_in`: open the file named in `name_of_file` as a TeX input
    /// and settle its encoding: `mode` (`AUTO` sniffs a byte order mark or
    /// a zero byte, else UTF-8).
    pub fn u_open_in(
        &mut self,
        f: &mut AlphaFile,
        _filefmt: i32,
        in_mode: i32,
        encoding_data: i32,
    ) -> bool {
        *f = AlphaFile::default();
        f.err = 1;
        let s = self.raw_file_name();
        if let Some(cmd) = s.strip_prefix('|') {
            if !run().shell_enabled {
                return false;
            }
            self.host.full_name_of_file = Some(s.clone());
            let Ok(mut child) = std::process::Command::new("/bin/sh")
                .arg("-c")
                .arg(cmd)
                .stdout(std::process::Stdio::piped())
                .spawn()
            else {
                return false;
            };
            let Some(out) = child.stdout.take() else {
                return false;
            };
            f.input = Some(TextIn::Pipe(BufReader::new(out)));
            f.child = Some(child);
        } else {
            let must_exist = self.host.tex_input_type;
            let Some(name) = self.input_path(Format::Tex, must_exist) else {
                return false;
            };
            let Ok(h) = File::open(&name) else {
                return false;
            };
            f.input = Some(TextIn::File(BufReader::new(h)));
            f.path = Some(name);
        }
        f.err = 0;
        f.at_eof = false;
        let mut mode = in_mode;
        if mode == AUTO {
            let b1 = f.getc();
            let b2 = f.getc();
            if b1 == 0xfe && b2 == 0xff {
                mode = UTF16BE;
            } else if b2 == 0xfe && b1 == 0xff {
                mode = UTF16LE;
            } else if b1 == 0 && b2 != 0 {
                mode = UTF16BE;
                f.rewind();
            } else if b2 == 0 && b1 != 0 {
                mode = UTF16LE;
                f.rewind();
            } else if b1 == 0xef && b2 == 0xbb {
                let b3 = f.getc();
                if b3 == 0xbf {
                    mode = UTF8;
                }
            }
            if mode == AUTO {
                f.rewind();
                mode = UTF8;
            }
        }
        self.set_input_file_encoding(f, mode, encoding_data);
        true
    }

    /// `u_close_inout`.
    pub fn u_close(&mut self, f: &mut AlphaFile) {
        f.close();
    }

    /// `setinputfileencoding`. An ICU converter (`ICUMAPPING`) is not
    /// available in phase S0: `get_encoding_mode_and_info` never returns
    /// it.
    pub fn set_input_file_encoding(&mut self, f: &mut AlphaFile, in_mode: i32, _data: i32) {
        match in_mode {
            UTF8 | UTF16BE | UTF16LE | RAW => f.encoding_mode = in_mode,
            _ => f.encoding_mode = RAW,
        }
    }

    /// `getencodingmodeandinfo`: the encoding named in `name_of_file`
    /// (`\XeTeXinputencoding`, `\XeTeXdefaultencoding`). Names other than
    /// XeTeX's built-in ones are ICU converters in TeX Live; phase S0 has
    /// none, and reports such a name as TeX Live does a name ICU does not
    /// know.
    pub fn get_encoding_mode_and_info(&mut self, enc_info: &mut i32) -> i32 {
        *enc_info = 0;
        let name = self.raw_file_name();
        let n = name.to_ascii_lowercase();
        match n.as_str() {
            "auto" => return AUTO,
            "utf8" => return UTF8,
            "utf16" => {
                return if cfg!(target_endian = "big") {
                    UTF16BE
                } else {
                    UTF16LE
                }
            }
            "utf16be" => return UTF16BE,
            "utf16le" => return UTF16LE,
            "bytes" => return RAW,
            _ => {}
        }
        self.begin_diagnostic();
        self.print_nl(b'U' as i32);
        for b in "nknown encoding `".bytes() {
            self.print_char(b as i32);
        }
        for b in name.bytes() {
            self.print_char(b as i32);
        }
        for b in "'; reading as raw bytes".bytes() {
            self.print_char(b as i32);
        }
        self.end_diagnostic(true);
        RAW
    }

    /// `makeutf16name`: `name_of_file16[0..name_length16-1]` from
    /// `name_of_file`.
    pub fn make_utf16_name(&mut self) {
        let bytes = self.raw_name_bytes();
        let mut t = 0usize;
        for c in decode_utf8_lenient(&bytes) {
            if c > 0xffff {
                let r = c - 0x10000;
                self.name_of_file16[t] = (0xd800 + r / 0x400) as i32;
                t += 1;
                self.name_of_file16[t] = (0xdc00 + r % 0x400) as i32;
                t += 1;
            } else {
                self.name_of_file16[t] = c as i32;
                t += 1;
            }
        }
        self.name_length16 = t as i32;
    }

    /// `printcstring(nameoffile+1)`: the bytes of `name_of_file`, each
    /// printed as a character.
    pub fn print_name_of_file_c(&mut self) {
        for b in self.raw_name_bytes() {
            if b == 0 {
                break;
            }
            self.print_char(b as i32);
        }
    }

    /// texmfmp.c's `loadpoolstrings` (tex-binpool.ch): the pool's strings,
    /// byte by byte, after the 65536 single-character strings; the last
    /// string made, or 0 if they would take more than `spare_size`.
    pub fn load_pool_strings(&mut self, spare_size: i32) -> i32 {
        let mut g = 0;
        let mut total: i64 = 0;
        for line in POOL.lines() {
            let b = line.as_bytes();
            if b.first() == Some(&b'*') {
                break;
            }
            if b.len() < 2 {
                continue;
            }
            let s = &b[2..];
            total += s.len() as i64;
            if total >= spare_size as i64 {
                return 0;
            }
            for &c in s {
                self.str_pool[self.pool_ptr as usize] = c as i32;
                self.pool_ptr += 1;
            }
            g = self.make_string();
        }
        g
    }

    // ---- the XDV file -----------------------------------------------------

    /// XeTeX_ext.c's `open_dvi_output`: with `-no-pdf`, `open_output`; the
    /// pipe to the output driver is phase 3.
    pub fn dvi_open_out(&mut self, f: &mut ByteFile) -> bool {
        self.b_open_out(f)
    }

    /// `dviclose`: 0, or the error the close reported (errno).
    pub fn dvi_close(&mut self, f: &mut ByteFile) -> i32 {
        let r = match f.output.as_mut() {
            Some(w) => match w.flush() {
                Ok(()) => 0,
                Err(e) => e.raw_os_error().unwrap_or(5),
            },
            None => 0,
        };
        f.close();
        r
    }

    /// C's `fflush` on the output file (the driver's pipe, phase 3).
    pub fn fflush(&mut self, f: &mut ByteFile) {
        f.flush();
    }

    /// `print_c_string(strerror(k))`.
    pub fn print_strerror(&mut self, k: i32) {
        let msg = std::io::Error::from_raw_os_error(k).to_string();
        // Rust appends " (os error N)" to the C message.
        let msg = msg.split(" (os error").next().unwrap_or("").to_string();
        for b in msg.bytes() {
            self.print_char(b as i32);
        }
    }

    // ---- changes/virtex.ch ------------------------------------------------

    /// Is this INITEX?
    pub fn ini_version(&mut self) -> bool {
        run().ini
    }

    /// Was `-etex` given?
    pub fn etex_p(&mut self) -> bool {
        run().etex
    }

    /// The default format's name, on the terminal.
    pub fn wterm_dump_name(&mut self) {
        let d = run().dump_name;
        wr_str(&mut self.term_out, &d);
    }

    /// The default format's file name into `name_of_file`.
    pub fn pack_default_format_name(&mut self) {
        let name = default_format_file();
        let b = name.as_bytes().to_vec();
        self.set_name_of_file(&b);
    }

    /// `fputs(name_of_file + 1, stdout)`.
    pub fn wterm_name_of_file(&mut self) {
        let n = self.raw_file_name();
        wr_str(&mut self.term_out, &n);
    }

    /// `fputs(TEX_format_default + 1, stdout)`.
    pub fn wterm_format_default(&mut self) {
        wr_str(&mut self.term_out, &default_format_file());
    }

    // ---- changes/filenames.ch ---------------------------------------------

    /// tex.ch's `texmf_yesno('log_openout')`.
    pub fn texmf_yesno_log_openout(&mut self) -> bool {
        texmf_yesno("log_openout")
    }

    /// tex.ch [29.530]'s `print_c_string(prompt_file_name_help_msg)`.
    pub fn print_prompt_file_name_help_msg(&mut self) {
        let msg: &[u8] = if cfg!(windows) {
            b"(Press Enter to retry, or Control-Z to exit"
        } else {
            b"(Press Enter to retry, or Control-D to exit"
        };
        for &c in msg {
            self.print_char(c as i32);
        }
    }

    /// texmfmp.c's `makefullnamestring`: the full name of the file opened
    /// last, or the empty string.
    pub fn make_full_name_string(&mut self) -> i32 {
        let full = self.host.full_name_of_file.clone().unwrap_or_default();
        self.make_string_utf8(full.as_bytes())
    }

    /// texmfmp.c's `maketexstring` for XeTeX: a C string (UTF-8) as a new
    /// pool string of UTF-16 code units; `""` for an empty one.
    pub fn make_string_utf8(&mut self, s: &[u8]) -> i32 {
        if s.is_empty() {
            return self.empty_string();
        }
        for c in decode_utf8_lenient(s) {
            if c > 0xffff {
                let r = c - 0x10000;
                self.str_pool[self.pool_ptr as usize] = (0xd800 + r / 0x400) as i32;
                self.pool_ptr += 1;
                self.str_pool[self.pool_ptr as usize] = (0xdc00 + r % 0x400) as i32;
            } else {
                self.str_pool[self.pool_ptr as usize] = c as i32;
            }
            self.pool_ptr += 1;
        }
        self.make_string()
    }

    /// The pool's empty string `""` (`getnullstr`): the first string after
    /// the single-character ones whose length is 0.
    fn empty_string(&self) -> i32 {
        let base = 65536;
        let mut s = base;
        while s < self.str_ptr {
            let k = (s - base) as usize;
            if self.str_start[k + 1] == self.str_start[k] {
                return s;
            }
            s += 1;
        }
        s
    }

    // ---- changes/web2c-run.ch ---------------------------------------------

    pub fn setup_bound_vars(&mut self) {
        self.error_line = setup_bound_var("error_line", 79);
        self.half_error_line = setup_bound_var("half_error_line", 50);
        self.max_print_line = setup_bound_var("max_print_line", 79);
        self.expand_depth = setup_bound_var("expand_depth", 10000);
    }
    pub fn web2c_interaction_option(&mut self) -> i32 {
        run().interaction_option
    }
    pub fn web2c_file_line_error_style_p(&mut self) -> bool {
        run().file_line_error
    }
    pub fn web2c_halt_on_error_p(&mut self) -> bool {
        run().halt_on_error
    }
    pub fn web2c_parse_first_line_p(&mut self) -> bool {
        run().parse_first_line
    }
    pub fn web2c_dump_line(&mut self) -> bool {
        run().dump_line
    }
    pub fn web2c_eight_bit_p(&mut self) -> bool {
        run().eight_bit
    }
    pub fn web2c_translate_filename_p(&mut self) -> bool {
        run().translate_filename.is_some()
    }
    pub fn web2c_shellenabledp(&mut self) -> bool {
        run().shell_enabled
    }
    pub fn web2c_restrictedshell(&mut self) -> bool {
        run().restricted_shell
    }
    pub fn web2c_no_pdf_output(&mut self) -> bool {
        self.host.no_pdf.0
    }
    pub fn wterm_version_string(&mut self) {
        wr_str(&mut self.term_out, WEB2C_VERSION);
    }
    pub fn wlog_version_string(&mut self) {
        wr_str(&mut self.log_file, WEB2C_VERSION);
    }
    /// The TCX file's name, which XeTeX ignores (`-translate-file`).
    pub fn wlog_translate_filename(&mut self) {
        if let Some(t) = run().translate_filename {
            wr_str(&mut self.log_file, &t);
        }
    }
    pub fn wterm_translate_filename(&mut self) {
        if let Some(t) = run().translate_filename {
            wr_str(&mut self.term_out, &t);
        }
    }
    /// tex.ch's `do_final_end`.
    pub fn do_final_end(&mut self) {
        final_end(self)
    }
    /// texmfmp.c's `getjobname`: `-jobname`, else `s`.
    pub fn get_job_name(&mut self, s: i32) -> i32 {
        match run().job_name {
            Some(j) => self.make_string_utf8(j.as_bytes()),
            None => s,
        }
    }
    /// `recorder_change_filename`: `-recorder` is not supported in phase S0.
    pub fn recorder_change_filename(&mut self) {}
    /// tex.ch's `tex_input_type`.
    pub fn set_tex_input_type(&mut self, input: bool) {
        self.host.tex_input_type = input;
    }
    /// kpathsea's `kpse_in_name_ok` (texmf.cnf's `openin_any`).
    pub fn kpse_in_name_ok(&mut self) -> bool {
        let n = self.raw_file_name();
        name_ok(&n, "openin_any", 'a')
    }
    /// kpathsea's `kpse_out_name_ok` (texmf.cnf's `openout_any`).
    pub fn kpse_out_name_ok(&mut self) -> bool {
        let n = self.raw_file_name();
        name_ok(&n, "openout_any", 'p')
    }
    /// `-output-comment` (texmfmp.c truncates it to 255 bytes), or -1.
    pub fn output_comment_length(&mut self) -> i32 {
        match run().output_comment {
            Some(c) => c.len().min(255) as i32,
            None => -1,
        }
    }
    pub fn output_comment_byte(&mut self, k: i32) -> i32 {
        run()
            .output_comment
            .as_ref()
            .and_then(|c| c.get(k as usize).copied())
            .unwrap_or(0) as i32
    }
    /// texmfmp.c's `runsystem` on `str_pool[s..s+l-1]`, made UTF-8 as
    /// xetex.ch's `append_to_name` makes it.
    pub fn runsystem(&mut self, s: i32, l: i32) -> i32 {
        let units: Vec<i32> = (s..s + l).map(|k| self.str_pool[k as usize]).collect();
        let cmd = utf16_to_name(&units);
        flashtex_engine::system::runsystem(&cmd)
    }
    /// texmfmp.c's `calledit`: the editor is not run in phase S0; the
    /// program ends as TeX Live's does after the editor.
    pub fn call_edit(&mut self, _s: i32, _l: i32, _n: i32) {
        exit_process(self, 1)
    }
}

/// `TEX_format_default`: `dump_name`, with `.fmt` added unless it is there.
fn default_format_file() -> String {
    let d = run().dump_name;
    if d.len() > 4 && d[d.len() - 4..].eq_ignore_ascii_case(".fmt") {
        d
    } else {
        format!("{d}.fmt")
    }
}

/// kpathsea's `kpathsea_name_ok`: `a` (any) allows everything; `r`
/// (restricted) refuses dot files; `p` (paranoid) also refuses absolute
/// names and `..` components, except an absolute name inside `TEXMFOUTPUT`.
fn name_ok(name: &str, var: &str, default: char) -> bool {
    let level = texmf_var(var)
        .and_then(|v| v.chars().next())
        .unwrap_or(default);
    if matches!(level, 'a' | 'y' | '1') {
        return true;
    }
    let base = name.rsplit('/').next().unwrap_or(name);
    if base.starts_with('.') && base != "." && base != ".." && !name.ends_with(".tex") {
        return false;
    }
    if matches!(level, 'p') {
        if name.starts_with('/') {
            if let Some(out) = texmf_var("TEXMFOUTPUT").filter(|v| !v.is_empty()) {
                if name.starts_with(&out) {
                    return true;
                }
            }
            return false;
        }
        if name.split('/').any(|c| c == "..") {
            return false;
        }
    }
    true
}

/// The UTF-8 command line as texmfmp.c's `topenin` reads it: the
/// arguments, each followed by a space, trailing spaces, CRs and LFs
/// removed by `init_terminal`.
pub fn command_line(args: &[String]) -> Vec<u8> {
    let mut line: Vec<u8> = vec![];
    for a in args {
        line.extend_from_slice(a.as_bytes());
        line.push(b' ');
    }
    line
}

/// The bytes of pool string `s` as UTF-8 (texmfmp.c's XeTeX
/// `gettexstring`): a string below 65536 is empty.
pub fn tex_string_utf8(g: &Globals, s: i32) -> Vec<u8> {
    if s < 65536 {
        return vec![];
    }
    let k = (s - 65536) as usize;
    let (a, b) = (g.str_start[k] as usize, g.str_start[k + 1] as usize);
    let mut out = vec![];
    let mut i = a;
    while i < b {
        let mut c = g.str_pool[i] as u32;
        if (0xD800..=0xDBFF).contains(&c) {
            i += 1;
            let lo = if i < b { g.str_pool[i] as u32 } else { 0 };
            if (0xDC00..=0xDFFF).contains(&lo) {
                c = 0x10000 + (c - 0xD800) * 0x400 + lo - 0xDC00;
            } else {
                c = 0xFFFD;
            }
        }
        let ch = char::from_u32(c).unwrap_or('\u{FFFD}');
        let mut tmp = [0u8; 4];
        out.extend_from_slice(ch.encode_utf8(&mut tmp).as_bytes());
        i += 1;
    }
    out
}

/// The routine reading lines of a file for `input_ln`, kept for the
/// readers that are not XeTeX's (none in phase S0).
#[allow(dead_code)]
fn read_line(r: &mut impl BufRead, line: &mut Vec<u8>) -> bool {
    line.clear();
    r.read_until(b'\n', line).map(|n| n > 0).unwrap_or(false)
}
