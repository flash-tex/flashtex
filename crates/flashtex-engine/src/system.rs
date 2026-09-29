//! The system-dependent part: everything web2c supplies through `tex.ch`.
//!
//! Deliberately small (DESIGN.md §4.1: "the system-dependent behaviour web2c
//! adds ... is re-specified by us"). It holds
//!
//!   * Pascal's lazy text-file and binary-file model (`f^`, `get`, `put`,
//!     `eof`, `eoln`, `read`, `read_ln`, `write`, `break`),
//!   * the fourteen routines of `tex.web` §§27-28, 31 and 37 that open, close
//!     and read files by name, and
//!   * the two non-local `goto`s out of the main program.
//!
//! The date and time come from `date_and_time` (src/pdftex/utils.rs), which
//! follows web2c: `SOURCE_DATE_EPOCH` with `FORCE_SOURCE_DATE=1`, else the
//! clock. (The trip test's tex.web build keeps tex.web §241's own pinned
//! date.)

use crate::generated::types::memory_word;
use crate::generated::Globals;
use crate::resolver::{FileResolver, Format};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Pascal text files
// ---------------------------------------------------------------------------

enum TextIn {
    Stdin,
    File(BufReader<File>),
    /// Lines handed to the engine before stdin is consulted, i.e. the command
    /// line, which is how INITEX is normally driven.
    Pre(Vec<Vec<u8>>, usize),
}

/// `packed file of char`.
#[derive(Default)]
pub struct AlphaFile {
    /// Pascal's buffer variable `f^`.
    pub buf: u8,
    input: Option<TextIn>,
    output: Option<BufWriter<File>>,
    to_stdout: bool,
    line: Vec<u8>,
    pos: usize,
    have_line: bool,
    /// Pascal's `erstat`: 0 means the last open succeeded.
    err: i32,
}

impl AlphaFile {
    fn refresh(&mut self) {
        self.buf = if !self.have_line {
            b' '
        } else if self.pos < self.line.len() {
            self.line[self.pos]
        } else {
            b' ' // reading the line separator yields a blank
        };
    }
    fn next_line(&mut self) -> bool {
        self.line.clear();
        self.pos = 0;
        match self.input.as_mut() {
            Some(TextIn::Pre(lines, i)) => {
                if *i < lines.len() {
                    self.line = lines[*i].clone();
                    *i += 1;
                    return true;
                }
                self.input = Some(TextIn::Stdin);
                self.next_line()
            }
            Some(TextIn::Stdin) => {
                let mut s = String::new();
                match std::io::stdin().read_line(&mut s) {
                    Ok(0) | Err(_) => false,
                    Ok(_) => {
                        self.line = s.trim_end_matches(['\n', '\r']).as_bytes().to_vec();
                        true
                    }
                }
            }
            Some(TextIn::File(r)) => {
                let mut v = Vec::new();
                match r.read_until(b'\n', &mut v) {
                    Ok(0) | Err(_) => false,
                    Ok(_) => {
                        while matches!(v.last(), Some(b'\n') | Some(b'\r')) {
                            v.pop();
                        }
                        self.line = v;
                        true
                    }
                }
            }
            None => false,
        }
    }
    fn put_byte(&mut self, b: u8) {
        if self.to_stdout {
            let _ = std::io::stdout().write_all(&[b]);
        } else if let Some(w) = self.output.as_mut() {
            let _ = w.write_all(&[b]);
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
}

/// `file of memory_word`.
#[derive(Default)]
pub struct WordFile {
    pub buf: memory_word,
    input: Option<BufReader<File>>,
    output: Option<BufWriter<File>>,
    at_eof: bool,
    err: i32,
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
        !self.have_line
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
        self.have_line = false;
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

pub fn eoln(f: &AlphaFile) -> bool {
    f.have_line && f.pos >= f.line.len()
}

pub fn get_char(f: &mut AlphaFile) {
    if !f.have_line {
        return;
    }
    if f.pos < f.line.len() {
        f.pos += 1;
    } else {
        f.have_line = f.next_line();
    }
    f.refresh();
}

pub fn read_char(f: &mut AlphaFile) -> u8 {
    let c = f.buf;
    get_char(f);
    c
}

pub fn read_ln(f: &mut AlphaFile) {
    if f.have_line {
        f.have_line = f.next_line();
    }
    f.refresh();
}

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
    let v = f.buf as u8;
    write_byte(f, v as i32);
}
pub fn write_byte(f: &mut ByteFile, v: i32) {
    if let Some(w) = f.output.as_mut() {
        let _ = w.write_all(&[v as u8]);
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

/// Pascal's `round`: half away from zero.
pub fn pas_round(x: f64) -> i32 {
    let r = x.round();
    if r >= 2147483647.0 {
        i32::MAX
    } else if r <= -2147483648.0 {
        i32::MIN
    } else {
        r as i32
    }
}

/// `t_open_out` — `rewrite(term_out,'TTY:','/O')`.
pub fn rewrite_char(f: &mut AlphaFile, _name: &str, _mode: &str) {
    *f = AlphaFile::default();
    f.to_stdout = true;
    f.err = 0;
}

// ---------------------------------------------------------------------------
// Opening files by name
// ---------------------------------------------------------------------------

/// The extra lines INITEX should see before stdin, i.e. the command line.
static FIRST_LINES: Mutex<Option<Vec<Vec<u8>>>> = Mutex::new(None);
/// Guards `close_files_and_terminate` against being re-entered by a second
/// `goto end_of_TEX` raised from inside it.
static TERMINATING: AtomicBool = AtomicBool::new(false);

/// Called by the binary before `tex_body`.
pub fn set_command_line(lines: Vec<Vec<u8>>) {
    *FIRST_LINES.lock().unwrap() = Some(lines);
}

fn take_command_line() -> Option<Vec<Vec<u8>>> {
    FIRST_LINES.lock().unwrap().take()
}

/// The process's file resolver (see `resolver.rs`). kpathsea keeps its state
/// in the environment, so there is one per process; `set_resolver` replaces
/// the default chosen on first use.
static RESOLVER: Mutex<Option<Box<dyn FileResolver>>> = Mutex::new(None);

pub fn set_resolver(r: Box<dyn FileResolver>) {
    *RESOLVER.lock().unwrap() = Some(r);
}

/// kpathsea's `kpse_invocation_name`, which pdfTeX's C parts print in
/// their warnings: `FLASHTEX_PROGNAME`, else `pdftex`.
pub fn invocation_name() -> String {
    std::env::var("FLASHTEX_PROGNAME").unwrap_or_else(|_| "pdftex".into())
}

/// `kpse_find_tex(name)`, for the C parts' `find_input_file`.
pub fn find_input(name: &str) -> Option<String> {
    resolve(name, Format::Tex)
}

/// C's `getc` on a binary file opened by `tex_b_openin` or `vf_b_open_in`:
/// the next byte, or -1 at the end. As for every `ByteFile`, `f.buf` holds
/// the byte Pascal's `f^` would show and `eof(f)` is true after the last one.
pub fn getc(f: &mut ByteFile) -> i32 {
    if f.at_eof {
        return -1;
    }
    let c = f.buf;
    get_byte(f);
    c
}

fn resolve(name: &str, format: Format) -> Option<String> {
    let mut g = RESOLVER.lock().unwrap();
    let r = g.get_or_insert_with(|| crate::resolver::default_resolver("tex", ""));
    let found = r
        .find(name, format)
        .map(|p| p.to_string_lossy().into_owned());
    if std::env::var_os("FLASHTEX_DEBUG_FILES").is_some() {
        eprintln!(
            "[resolve] {} {:?} {:?} -> {:?}",
            r.describe(),
            format,
            name,
            found
        );
    }
    found
}

impl Globals {
    /// `name_of_file`, trimmed. `name_length` is authoritative when set, but
    /// §51 opens the pool file without setting it.
    fn raw_file_name(&self) -> String {
        let raw: &[u8] = if self.name_length > 0 {
            &self.name_of_file[..self.name_length as usize]
        } else {
            &self.name_of_file[..]
        };
        raw.iter()
            .map(|&b| b as char)
            .collect::<String>()
            .trim()
            .to_string()
    }

    /// Split off a `tex.web` device name (§§514, 520: `TeXinputs:`,
    /// `TeXfonts:`, `TeXformats:`); never a drive letter or a URL.
    fn split_area(s: &str) -> (&str, &str) {
        match s.find(':') {
            Some(i) if i > 1 && s[..i].chars().all(|c| c.is_ascii_alphabetic()) => {
                (&s[..i], &s[i + 1..])
            }
            _ => ("", s),
        }
    }

    /// Where to open an input file, looked up by the resolver in the format
    /// its device name implies. As web2c does, the path found is written back
    /// into `name_of_file`, so `a_make_name_string` -- and therefore the `(`
    /// line in the log -- shows `./story.tex` exactly as pdfTeX's does.
    fn input_path(&mut self, default: Format) -> Option<String> {
        let s = self.raw_file_name();
        let (area, base) = Self::split_area(&s);
        if base.eq_ignore_ascii_case("TEX.POOL") {
            return Some(pool_path());
        }
        let format = match area {
            "TeXfonts" => Format::Tfm,
            "TeXformats" => Format::Fmt,
            "TeXinputs" => Format::Tex,
            _ => default,
        };
        let found = resolve(base, format)?;
        let n = found.len();
        if n <= self.name_of_file.len() {
            self.name_of_file.fill(b' ');
            self.name_of_file[..n].copy_from_slice(found.as_bytes());
            self.name_length = n as i32;
        }
        Some(found)
    }

    /// Output files are created where they are named, device name dropped.
    fn output_path(&self) -> String {
        let s = self.raw_file_name();
        Self::split_area(&s).1.to_string()
    }

    pub fn a_open_in(&mut self, f: &mut AlphaFile) -> bool {
        *f = AlphaFile::default();
        f.err = 1;
        let Some(name) = self.input_path(Format::Tex) else {
            return false;
        };
        match File::open(&name) {
            Ok(h) => {
                f.input = Some(TextIn::File(BufReader::new(h)));
                f.have_line = f.next_line();
                f.refresh();
                f.err = 0;
                true
            }
            Err(_) => false,
        }
    }

    pub fn a_open_out(&mut self, f: &mut AlphaFile) -> bool {
        let name = self.output_path();
        *f = AlphaFile::default();
        match File::create(&name) {
            Ok(h) => {
                f.output = Some(BufWriter::new(h));
                f.err = 0;
                true
            }
            Err(_) => {
                f.err = 1;
                false
            }
        }
    }

    pub fn b_open_in(&mut self, f: &mut ByteFile) -> bool {
        *f = ByteFile::default();
        f.err = 1;
        let Some(name) = self.input_path(Format::Tfm) else {
            return false;
        };
        match File::open(&name) {
            Ok(h) => {
                f.input = Some(BufReader::new(h));
                f.err = 0;
                // Pascal's `reset` leaves `f^` holding the first component:
                // `read_sixteen` (§565) reads `fbyte` before its first `fget`.
                get_byte(f);
                true
            }
            Err(_) => false,
        }
    }

    /// `open_input(&f, kpse_tex_format, FOPEN_RBIN_MODE)` (pdftex.h's
    /// `texbopenin`): a TeX input file read as bytes (`\pdfobj file`).
    pub fn tex_b_openin(&mut self, f: &mut ByteFile) -> bool {
        self.byte_open_in(f, Format::Tex)
    }

    /// `open_input(&f, kpse_vf_format, FOPEN_RBIN_MODE)` (pdftex.h's
    /// `vfbopenin`).
    pub fn vf_b_open_in(&mut self, f: &mut ByteFile) -> bool {
        self.byte_open_in(f, Format::Vf)
    }

    fn byte_open_in(&mut self, f: &mut ByteFile, format: Format) -> bool {
        *f = ByteFile::default();
        f.err = 1;
        let Some(name) = self.input_path(format) else {
            return false;
        };
        match File::open(&name) {
            Ok(h) => {
                f.input = Some(BufReader::new(h));
                f.err = 0;
                get_byte(f);
                true
            }
            Err(_) => false,
        }
    }

    pub fn b_open_out(&mut self, f: &mut ByteFile) -> bool {
        let name = self.output_path();
        *f = ByteFile::default();
        match File::create(&name) {
            Ok(h) => {
                f.output = Some(BufWriter::new(h));
                f.err = 0;
                true
            }
            Err(_) => {
                f.err = 1;
                false
            }
        }
    }

    pub fn w_open_in(&mut self, f: &mut WordFile) -> bool {
        *f = WordFile::default();
        f.err = 1;
        let Some(name) = self.input_path(Format::Fmt) else {
            return false;
        };
        match File::open(&name) {
            Ok(h) => {
                f.input = Some(BufReader::new(h));
                f.err = 0;
                // As for `b_open_in`: §1307 reads `fmt_file^.int` before the
                // first `undump_wd`, which itself starts with a `get`.
                get_word(f);
                true
            }
            Err(_) => false,
        }
    }

    pub fn w_open_out(&mut self, f: &mut WordFile) -> bool {
        let name = self.output_path();
        *f = WordFile::default();
        match File::create(&name) {
            Ok(h) => {
                f.output = Some(BufWriter::new(h));
                f.err = 0;
                true
            }
            Err(_) => {
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

    /// `tex.web` §31, with the system-dependent lookahead done by `AlphaFile`.
    pub fn input_ln(&mut self, f: &mut AlphaFile, bypass_eoln: bool) -> bool {
        if bypass_eoln && !eof(f) {
            get_char(f);
        }
        self.last = self.first;
        if eof(f) {
            return false;
        }
        let mut last_nonblank = self.first;
        while !eoln(f) {
            if self.last >= self.max_buf_stack {
                self.max_buf_stack = self.last + 1;
                if self.max_buf_stack == crate::generated::consts::buf_size {
                    self.buffer_overflow();
                }
            }
            let c = f.buf;
            self.buffer[self.last as usize] = self.xord[c as usize];
            get_char(f);
            self.last += 1;
            if self.buffer[(self.last - 1) as usize] != b' ' as i32 {
                last_nonblank = self.last;
            }
        }
        self.last = last_nonblank;
        true
    }

    /// The number of the pool string TANGLE wrote for `text` (the system
    /// layer is not tangled, so it cannot write `"buffer size"` itself).
    fn pool_string_number(&self, text: &[u8]) -> i32 {
        for s in 256..self.str_ptr {
            let (a, b) = (
                self.str_start[s as usize] as usize,
                self.str_start[s as usize + 1] as usize,
            );
            if self.str_pool[a..b]
                .iter()
                .map(|&c| c as u8)
                .eq(text.iter().copied())
            {
                return s;
            }
        }
        0
    }

    /// `tex.web` §35, "Report overflow of the input buffer, and abort".
    fn buffer_overflow(&mut self) {
        if self.format_ident == 0 {
            eprintln!("Buffer size exceeded!");
            final_end(self);
        } else {
            self.cur_input.loc_field = self.first;
            self.cur_input.limit_field = self.last - 1;
            // `overflow("buffer size", buf_size)`.
            let s = self.pool_string_number(b"buffer size");
            self.overflow(s, crate::generated::consts::buf_size);
        }
    }

    /// `tex.web` §37 (`function init_terminal:boolean`). The command line, if
    /// any, stands in for the first `**` line, which is how INITEX is driven in
    /// practice.
    pub fn init_terminal(&mut self) -> bool {
        let mut term = ::core::mem::take(&mut self.term_in);
        let r = self.init_terminal_inner(&mut term);
        self.term_in = term;
        r
    }

    fn init_terminal_inner(&mut self, f: &mut AlphaFile) -> bool {
        *f = AlphaFile::default();
        f.input = Some(match take_command_line() {
            Some(lines) if !lines.is_empty() => TextIn::Pre(lines, 0),
            _ => TextIn::Stdin,
        });
        f.have_line = f.next_line();
        f.refresh();
        loop {
            if !matches!(f.input, Some(TextIn::Pre(..))) {
                print!("**");
                let _ = std::io::stdout().flush();
            }
            // `input_ln(term_in, true)` would skip the line we just loaded.
            self.last = self.first;
            if eof(f) {
                println!("\n! End of file on the terminal... why?");
                return false;
            }
            let mut last_nonblank = self.first;
            while !eoln(f) {
                let c = f.buf;
                self.buffer[self.last as usize] = self.xord[c as usize];
                get_char(f);
                self.last += 1;
                if self.buffer[(self.last - 1) as usize] != b' ' as i32 {
                    last_nonblank = self.last;
                }
            }
            self.last = last_nonblank;
            self.cur_input.loc_field = self.first;
            while self.cur_input.loc_field < self.last
                && self.buffer[self.cur_input.loc_field as usize] == b' ' as i32
            {
                self.cur_input.loc_field += 1;
            }
            if self.cur_input.loc_field < self.last {
                return true;
            }
            println!("Please type the name of your input file.");
            read_ln(f);
        }
    }
}

// ---------------------------------------------------------------------------
// The non-local gotos of the main program
// ---------------------------------------------------------------------------

/// `goto final_end` (label 9999): `ready_already:=0; end.`
pub fn final_end(g: &mut Globals) -> ! {
    g.ready_already = 0;
    let code = if g.history > 1 { 1 } else { 0 };
    std::process::exit(code)
}

/// `goto end_of_TEX` (label 9998), reached from `jump_out` (§81): run
/// `close_files_and_terminate` and then fall into `final_end`.
#[allow(non_snake_case)] // WEB's label name, kept on purpose
pub fn end_of_TEX(g: &mut Globals) -> ! {
    if !TERMINATING.swap(true, Ordering::SeqCst) {
        g.close_files_and_terminate();
    }
    final_end(g)
}

/// The string pool web2rust wrote (crates/flashtex-engine/pdftex.pool):
/// `FLASHTEX_POOL`, else `pdftex.pool` beside the executable, else in the
/// working directory. It is ours, not TeX Live's, so it never goes through the
/// resolver.
fn pool_path() -> String {
    if let Ok(p) = std::env::var("FLASHTEX_POOL") {
        return p;
    }
    if let Ok(exe) = std::env::current_exe() {
        let c = exe.with_file_name("pdftex.pool");
        if c.exists() {
            return c.to_string_lossy().into_owned();
        }
    }
    "pdftex.pool".into()
}
