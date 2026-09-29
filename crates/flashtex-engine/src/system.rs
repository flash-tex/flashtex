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
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Pascal text files
// ---------------------------------------------------------------------------

enum TextIn {
    Stdin,
    File(BufReader<File>),
    /// `\input|command` (texmfmp.c's `open_in_or_pipe`).
    Pipe(BufReader<std::process::ChildStdout>),
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
    output: Option<BufWriter<Box<dyn Write + Send>>>,
    /// The command of a pipe, waited for when the file is closed (`pclose`).
    child: Option<std::process::Child>,
    to_stdout: bool,
    line: Vec<u8>,
    pos: usize,
    have_line: bool,
    /// Pascal's `erstat`: 0 means the last open succeeded.
    err: i32,
    /// The file opened, for a checkpoint's host-state record.
    path: Option<String>,
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
            Some(TextIn::Stdin) => read_tex_line(&mut std::io::stdin().lock(), &mut self.line),
            Some(TextIn::File(r)) => read_tex_line(r, &mut self.line),
            Some(TextIn::Pipe(r)) => read_tex_line(r, &mut self.line),
            None => false,
        }
    }
    fn put_byte(&mut self, b: u8) {
        if self.to_stdout {
            if !terminal_capture_byte(b) {
                let _ = std::io::stdout().write_all(&[b]);
            }
        } else if let Some(w) = self.output.as_mut() {
            let _ = w.write_all(&[b]);
        }
    }
}

/// One line as texmfmp.c's `input_line` reads it: up to a LF, a CR or a CR
/// LF; false at the end of the file when nothing was read.
fn read_tex_line(r: &mut impl BufRead, line: &mut Vec<u8>) -> bool {
    line.clear();
    let mut got_any = false;
    loop {
        let buf = match r.fill_buf() {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return got_any,
        };
        if buf.is_empty() {
            return got_any;
        }
        got_any = true;
        match buf.iter().position(|&c| c == b'\n' || c == b'\r') {
            Some(i) => {
                let cr = buf[i] == b'\r';
                line.extend_from_slice(&buf[..i]);
                r.consume(i + 1);
                if cr {
                    // If the next character is the LF of a CR LF, read it.
                    if let Ok(b) = r.fill_buf() {
                        if b.first() == Some(&b'\n') {
                            r.consume(1);
                        }
                    }
                }
                return true;
            }
            None => {
                let n = buf.len();
                line.extend_from_slice(buf);
                r.consume(n);
            }
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
    /// The file opened, for a checkpoint's host-state record.
    path: Option<String>,
}

/// `file of memory_word`.
#[derive(Default)]
pub struct WordFile {
    pub buf: memory_word,
    input: Option<BufReader<File>>,
    output: Option<BufWriter<File>>,
    at_eof: bool,
    err: i32,
    /// The file opened, for a checkpoint's host-state record.
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
        !self.have_line
    }
    fn flush(&mut self) {
        if self.to_stdout {
            if !terminal_captured() {
                let _ = std::io::stdout().flush();
            }
        } else if let Some(w) = self.output.as_mut() {
            let _ = w.flush();
        }
    }
    fn close(&mut self) {
        self.flush();
        self.output = None;
        self.input = None;
        self.have_line = false;
        self.path = None;
        // `pclose`: the command has its end of file, and is waited for.
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

impl ByteFile {
    /// C's `fwrite` on a binary output file (the PDF writer, `writepdf` and
    /// `writezip`): whether all bytes were written.
    pub fn write_bytes(&mut self, bytes: &[u8]) -> bool {
        match self.output.as_mut() {
            Some(w) => w.write_all(bytes).is_ok(),
            None => false,
        }
    }

    /// C's `fseeko(f, offset, SEEK_SET)` on a binary output file
    /// (`writestreamlength` patches a stream's `/Length`).
    pub fn seek_to(&mut self, offset: u64) -> bool {
        use std::io::Seek;
        match self.output.as_mut() {
            Some(w) => w.seek(std::io::SeekFrom::Start(offset)).is_ok(),
            None => false,
        }
    }
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

/// The command line as TeX's first line (texmfmp.c's `topenin`), for a
/// caller that drives the engine without `configure`.
pub fn set_command_line(lines: Vec<Vec<u8>>) {
    *FIRST_LINES.lock().unwrap() = Some(lines);
}

fn take_command_line() -> Option<Vec<Vec<u8>>> {
    FIRST_LINES.lock().unwrap().take()
}

// ---------------------------------------------------------------------------
// The run's options: texmfmp.c's `parse_options` and `maininit`
// ---------------------------------------------------------------------------

/// `-interaction`, numbered as tex.ch numbers the modes (`unspecified_mode`
/// is 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interaction {
    Batch = 0,
    Nonstop = 1,
    Scroll = 2,
    ErrorStop = 3,
    Unspecified = 4,
}

/// The shell-escape switches of the command line (texmfmp.c's
/// `shellenabledp` before `init_shell_escape`: 0, 1 or -1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shell {
    /// No option given: texmf.cnf's `shell_escape` decides.
    Unset,
    /// `-no-shell-escape`.
    Off,
    /// `-shell-escape`.
    On,
    /// `-shell-restricted`: only texmf.cnf's `shell_escape_commands`.
    Restricted,
}

/// What the command line says, before texmf.cnf is consulted.
#[derive(Clone, Debug)]
pub struct RunOptions {
    /// `kpse_invocation_name`: `argv[0]` as given, which the C parts print
    /// in their messages (`pdfTeX warning: /path/pdftex: ...`).
    pub invocation_name: String,
    /// The program name `argv[0]` implies (see `program_name_from_argv0`).
    pub argv0_program: String,
    pub ini: bool,
    pub etex: bool,
    /// `-fmt`.
    pub dump_name: Option<String>,
    /// `-progname`.
    pub user_progname: Option<String>,
    pub interaction: Interaction,
    pub halt_on_error: bool,
    /// 1 on, -1 off, 0 texmf.cnf's `file_line_error_style`.
    pub file_line_error: i32,
    /// 1 on, -1 off, 0 texmf.cnf's `parse_first_line`.
    pub parse_first_line: i32,
    pub job_name: Option<String>,
    pub output_directory: Option<String>,
    pub translate_filename: Option<String>,
    pub default_translate_filename: Option<String>,
    pub eight_bit: bool,
    pub shell: Shell,
    pub recorder: bool,
    /// `-output-format`: `\pdfoutput` 0 (dvi) or 2 (pdf).
    pub output_format: Option<i32>,
    pub draftmode: bool,
    pub output_comment: Option<String>,
    pub cnf_lines: Vec<String>,
    /// The arguments after the options (`argv[optind..]`).
    pub args: Vec<String>,
    /// `argv[1]`, which texmfmp.c checks for `&fmt`.
    pub first_arg: Option<String>,
}

impl RunOptions {
    pub fn new(argv0: &str) -> RunOptions {
        RunOptions {
            invocation_name: argv0.to_string(),
            argv0_program: program_name_from_argv0(argv0),
            ini: false,
            etex: false,
            dump_name: None,
            user_progname: None,
            interaction: Interaction::Unspecified,
            halt_on_error: false,
            file_line_error: 0,
            parse_first_line: 0,
            job_name: None,
            output_directory: None,
            translate_filename: None,
            default_translate_filename: None,
            eight_bit: false,
            shell: Shell::Unset,
            recorder: false,
            output_format: None,
            draftmode: false,
            output_comment: None,
            cnf_lines: vec![],
            args: vec![],
            first_arg: None,
        }
    }
}

/// The run as configured: the command line plus texmf.cnf, the state the
/// generated code reads through `changes/web2c-run.ch`.
#[derive(Clone, Debug)]
pub struct Run {
    pub invocation_name: String,
    /// `kpse_program_name`: selects the search paths.
    pub program_name: String,
    pub ini: bool,
    pub etex: bool,
    /// `dump_name`: the default format.
    pub dump_name: String,
    pub interaction_option: i32,
    pub halt_on_error: bool,
    pub file_line_error: bool,
    pub parse_first_line: bool,
    /// A `%&format` first line named the format.
    pub dump_line: bool,
    pub job_name: Option<String>,
    pub output_directory: Option<String>,
    pub translate_filename: Option<String>,
    pub eight_bit: bool,
    pub shell_enabled: bool,
    pub restricted_shell: bool,
    /// texmf.cnf's `shell_escape_commands`, for restricted shell escape.
    pub shell_commands: Vec<String>,
    pub recorder: bool,
    pub output_format: Option<i32>,
    pub draftmode: bool,
    pub output_comment: Option<String>,
}

impl Default for Run {
    /// A process that drives the engine without `configure` (the tests of
    /// the library, the trip test): INITEX, program name `pdftex` (or
    /// `FLASHTEX_PROGNAME`), nothing from texmf.cnf.
    fn default() -> Run {
        let prog = std::env::var("FLASHTEX_PROGNAME").unwrap_or_else(|_| "pdftex".into());
        Run {
            invocation_name: "pdftex".into(),
            program_name: prog.clone(),
            ini: true,
            etex: false,
            dump_name: prog,
            interaction_option: Interaction::Unspecified as i32,
            halt_on_error: false,
            file_line_error: false,
            parse_first_line: false,
            dump_line: false,
            job_name: None,
            output_directory: None,
            translate_filename: None,
            eight_bit: false,
            shell_enabled: false,
            restricted_shell: false,
            shell_commands: vec![],
            recorder: false,
            output_format: None,
            draftmode: false,
            output_comment: None,
        }
    }
}

static RUN: Mutex<Option<Run>> = Mutex::new(None);

/// The run's configuration (the default one until `configure` is called).
pub fn run() -> Run {
    RUN.lock().unwrap().get_or_insert_with(Run::default).clone()
}

fn with_run<T>(f: impl FnOnce(&mut Run) -> T) -> T {
    f(RUN.lock().unwrap().get_or_insert_with(Run::default))
}

/// The program name web2c derives from `argv[0]`. Invoked under one of
/// pdfTeX's names (`pdflatex`, `pdfinitex`, ... through a link), the engine
/// behaves as pdfTeX invoked under it; under its own name it is `pdftex`.
pub fn program_name_from_argv0(argv0: &str) -> String {
    let base = argv0.rsplit('/').next().unwrap_or(argv0);
    let base = base.strip_suffix(".exe").unwrap_or(base);
    if base.is_empty() || base.starts_with("flashtex") {
        "pdftex".into()
    } else {
        base.to_string()
    }
}

/// texmfmp.c's `normalize_quotes`: quote a name only if it has a space, and
/// then as `"name"`; `None` for unbalanced quotes.
pub fn normalize_quotes(name: &str) -> Option<String> {
    let must_quote = name.contains(' ');
    let mut quoted = false;
    let mut out = String::with_capacity(name.len() + 2);
    if must_quote {
        out.push('"');
    }
    for c in name.chars() {
        if c == '"' {
            quoted = !quoted;
        } else {
            out.push(c);
        }
    }
    if must_quote {
        out.push('"');
    }
    (!quoted).then_some(out)
}

/// kpathsea's `kpathsea_cnf_line_env_progname` (cnf.c `do_line` with
/// `env_progname`): a `-cnf-line` sets `VAR` and `VAR_progname` (or the
/// `.prog` it names) in the environment, where kpathsea looks first.
fn cnf_line_env_progname(line: &str, program_name: &str, invocation: &str) {
    let line = line.trim_start_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    if line.is_empty() || line.starts_with('%') || line.starts_with('#') {
        return;
    }
    // A trailing comment is a % or # preceded by whitespace.
    let bytes = line.as_bytes();
    let mut end = bytes.len();
    let mut i = bytes.len();
    while i > 1 {
        i -= 1;
        if (bytes[i] == b'%' || bytes[i] == b'#') && bytes[i - 1].is_ascii_whitespace() {
            end = i;
        }
    }
    let line = line[..end].trim_end();
    let is_space = |c: char| c.is_ascii_whitespace();
    let var_end = line
        .find(|c: char| is_space(c) || c == '=' || c == '.')
        .unwrap_or(line.len());
    let var = &line[..var_end];
    let warn = |msg: &str| {
        eprintln!("warning: {invocation}: command line (kpathsea): {msg} in argument: {line}");
    };
    if var.is_empty() {
        return warn("No cnf variable name");
    }
    let mut rest = line[var_end..].trim_start_matches(is_space);
    let mut prog: Option<&str> = None;
    if let Some(r) = rest.strip_prefix('.') {
        let r = r.trim_start_matches(is_space);
        let n = r.find(|c: char| is_space(c) || c == '=').unwrap_or(r.len());
        let p = &r[..n];
        if p.is_empty() {
            return warn("Empty program name qualifier");
        }
        if let Some(c) = p
            .chars()
            .find(|&c| matches!(c, '$' | '{' | '}' | ':' | ';'))
        {
            return warn(&format!("Unlikely character {c} in program name"));
        }
        prog = Some(p);
        rest = &r[n..];
    }
    let rest = rest.trim_start_matches(is_space);
    let rest = rest
        .strip_prefix('=')
        .unwrap_or(rest)
        .trim_start_matches(is_space);
    let value = rest.trim_end_matches(is_space);
    if value.is_empty() {
        return warn("No cnf value");
    }
    // Unix separators: `;` in a value means `:`.
    let value = value.replace(';', ":");
    std::env::set_var(var, &value);
    std::env::set_var(format!("{var}_{}", prog.unwrap_or(program_name)), &value);
}

/// texmfmp.c's `maininit` after `parse_options`: settle the program name,
/// the format, the first line and everything texmf.cnf decides, in its
/// order. Called by the binary before `tex_body`.
pub fn configure(mut o: RunOptions) {
    // parse_options: -output-directory is exported for \write18's children,
    // and TEXMF_OUTPUT_DIRECTORY stands in for it.
    if let Some(d) = &o.output_directory {
        std::env::set_var("TEXMF_OUTPUT_DIRECTORY", d);
    } else if let Some(d) = std::env::var("TEXMF_OUTPUT_DIRECTORY")
        .ok()
        .filter(|d| !d.is_empty())
    {
        o.output_directory = Some(d);
    }
    // "If -progname was not specified, default to the dump name", else the
    // name we were invoked under.
    let mut program_name = o
        .user_progname
        .clone()
        .or_else(|| o.dump_name.clone())
        .unwrap_or_else(|| o.argv0_program.clone());
    for l in &o.cnf_lines {
        cnf_line_env_progname(l, &program_name, &o.invocation_name);
    }
    *RUN.lock().unwrap() = Some(Run {
        invocation_name: o.invocation_name.clone(),
        program_name: program_name.clone(),
        ..Run::default()
    });
    reset_resolver(&program_name, o.cnf_lines.is_empty());

    // get_input_file_name: a plain file name as the first argument.
    let mut main_input_file = None;
    if let Some(a) = o.args.first().cloned() {
        if !a.starts_with('&') && !a.starts_with('\\') {
            let Some(name) = normalize_quotes(&a) else {
                eprintln!("! Unbalanced quotes in argument {a}");
                std::process::exit(1);
            };
            let bare = name
                .strip_prefix('"')
                .and_then(|n| n.strip_suffix('"'))
                .unwrap_or(&name);
            main_input_file = resolve(bare, Format::Tex);
            o.args[0] = name;
        }
    }

    // file:line:error and %&-line parsing: the command line, else texmf.cnf.
    let file_line_error = match o.file_line_error {
        n if n < 0 => false,
        0 => texmf_yesno("file_line_error_style"),
        _ => true,
    };
    let parse_first_line = match o.parse_first_line {
        n if n < 0 => false,
        0 => texmf_yesno("parse_first_line"),
        _ => true,
    };
    let mut dump_line = false;
    if parse_first_line && (o.dump_name.is_none() || o.translate_filename.is_none()) {
        if let Some(f) = &main_input_file {
            parse_first_line_of(
                f,
                &mut o.dump_name,
                &mut o.translate_filename,
                &mut program_name,
                &mut dump_line,
            );
        }
    }
    if o.translate_filename.is_none() {
        o.translate_filename = o.default_translate_filename.clone();
    }
    // The program name can make the run INITEX, or name the format.
    let mut ini = o.ini;
    let mut vir = false;
    if program_name.eq_ignore_ascii_case("pdfinitex") || program_name.eq_ignore_ascii_case("initex")
    {
        ini = true;
    } else if program_name.eq_ignore_ascii_case("pdfvirtex")
        || program_name.eq_ignore_ascii_case("virtex")
    {
        vir = true;
    }
    if main_input_file.is_none() {
        if let Some(f) = o.first_arg.as_deref().and_then(|a| a.strip_prefix('&')) {
            o.dump_name = Some(f.to_string());
        }
    }
    let dump_name = o.dump_name.clone().unwrap_or_else(|| {
        if vir {
            "plain".into()
        } else {
            program_name.clone()
        }
    });
    if !ini && o.etex {
        eprintln!("-etex only works with -ini");
    }
    // The trip test's tex.web build is always INITEX, as tex.web itself is.
    if cfg!(feature = "tex82") && o.dump_name.is_none() {
        ini = true;
    }

    // init_shell_escape. With no option texmf.cnf's `shell_escape` decides,
    // as in web2c: TeX Live ships `p`, restricted (DESIGN.md 4.5), and the
    // bundle resolver carries TeX Live's values (resolver.rs).
    let (shell_enabled, restricted_shell) = match o.shell {
        Shell::Off => (false, false),
        Shell::On => (true, false),
        Shell::Restricted => (true, true),
        Shell::Unset => match texmf_var("shell_escape").and_then(|v| v.bytes().next()) {
            Some(b't' | b'y' | b'1') => (true, false),
            Some(b'p') => (true, true),
            _ => (false, false),
        },
    };
    let mut shell_commands = vec![];
    if shell_enabled && restricted_shell {
        if let Some(v) = texmf_var("shell_escape_commands") {
            // mk_shellcmdlist: comma-separated, an empty last item dropped.
            let mut items: Vec<String> = v.split(',').map(str::to_string).collect();
            if items.last().is_some_and(|s| s.is_empty()) {
                items.pop();
            }
            shell_commands = items;
        }
    }
    let output_comment = o
        .output_comment
        .clone()
        .or_else(|| texmf_var("output_comment"));

    // topenin: the arguments, each followed by a space, trailing spaces,
    // CRs and LFs removed.
    if !o.args.is_empty() {
        let mut line: Vec<u8> = vec![];
        for a in &o.args {
            line.extend_from_slice(a.as_bytes());
            line.push(b' ');
        }
        while matches!(line.last(), Some(b' ' | b'\r' | b'\n')) {
            line.pop();
        }
        set_command_line(vec![line]);
    }

    let program_changed = program_name != run().program_name;
    let prog = program_name.clone();
    *RUN.lock().unwrap() = Some(Run {
        invocation_name: o.invocation_name.clone(),
        program_name,
        ini,
        etex: o.etex,
        dump_name,
        interaction_option: o.interaction as i32,
        halt_on_error: o.halt_on_error,
        file_line_error,
        parse_first_line,
        dump_line,
        job_name: o.job_name.clone(),
        output_directory: o.output_directory.clone(),
        translate_filename: o.translate_filename.clone(),
        eight_bit: o.eight_bit,
        shell_enabled,
        restricted_shell,
        shell_commands,
        recorder: o.recorder,
        output_format: o.output_format,
        draftmode: o.draftmode,
        output_comment,
    });
    if program_changed {
        reset_resolver(&prog, false);
    }
}

/// texmfmp.c's `parse_first_line`: a first line `%&fmt --translate-file=tcx`
/// names the format (if it can be found, and none was given) and the TCX
/// file (if none was given).
fn parse_first_line_of(
    path: &str,
    dump_name: &mut Option<String>,
    translate_filename: &mut Option<String>,
    program_name: &mut String,
    dump_line: &mut bool,
) {
    let Ok(f) = File::open(path) else { return };
    let mut first = vec![];
    if BufReader::new(f).read_until(b'\n', &mut first).is_err() {
        return;
    }
    while matches!(first.last(), Some(b'\n' | b'\r')) {
        first.pop();
    }
    let Some(rest) = first.strip_prefix(b"%&") else {
        return;
    };
    let rest = String::from_utf8_lossy(rest).into_owned();
    let rest = rest.trim_start_matches([' ', '\t']);
    // At most three space-separated parts.
    let mut parts: Vec<&str> = vec![];
    let mut s = rest;
    while !s.is_empty() && parts.len() != 3 {
        let n = s.find(' ').unwrap_or(s.len());
        parts.push(&s[..n]);
        s = s[n..].trim_start_matches(' ');
    }
    let mut parse = parts.as_slice();
    if let Some(first) = parse.first() {
        if !first.starts_with('-') {
            if dump_name.is_none() {
                let f_name = format!("{first}.fmt");
                if find_format(&f_name).is_some() {
                    *dump_name = Some(first.to_string());
                    *program_name = first.to_string();
                    with_run(|r| r.program_name = program_name.clone());
                    reset_resolver(program_name, false);
                    *dump_line = true;
                }
            }
            parse = &parse[1..];
        }
    }
    if let Some(p) = parse.first() {
        if translate_filename.is_some() {
            return;
        }
        let s = if *p == "--translate-file" || *p == "-translate-file" {
            parse.get(1).copied()
        } else {
            p.strip_prefix("--translate-file=")
                .or_else(|| p.strip_prefix("-translate-file="))
        };
        if let Some(s) = s.filter(|s| !s.is_empty()) {
            *translate_filename = Some(s.to_string());
        }
    }
}

/// pdfTeX's `BANNER` (pdftexextra.h), the same text as pdftex.web's
/// `banner`.
pub const BANNER: &str = "This is pdfTeX, Version 3.141592653-2.6-1.40.29";

/// web2c's `versionstring` (`WEB2CVERSION`) for the TeX Live 2026 sources
/// this engine is translated from and links (third_party/pdftex,
/// third_party/kpathsea).
pub const WEB2C_VERSION: &str = " (TeX Live 2026)";

/// `-help` (texmfmp-help.h's PDFTEXHELP).
pub const HELP: &str = "\
Usage: pdftex [OPTION]... [TEXNAME[.tex]] [COMMANDS]
   or: pdftex [OPTION]... \\FIRST-LINE
   or: pdftex [OPTION]... &FMT ARGS
  Run pdfTeX on TEXNAME, usually creating TEXNAME.pdf.
  Any remaining COMMANDS are processed as pdfTeX input, after TEXNAME is read.
  If the first line of TEXNAME is %&FMT, and FMT is an existing .fmt file,
  use it.  Else use `NAME.fmt', where NAME is the program invocation name,
  most commonly `pdftex'.

  Alternatively, if the first non-option argument begins with a backslash,
  interpret all non-option arguments as a line of pdfTeX input.

  Alternatively, if the first non-option argument begins with a &, the
  next word is taken as the FMT to read, overriding all else.  Any
  remaining arguments are processed as above.

  If no arguments or options are specified, prompt for input.

-cnf-line=STRING        parse STRING as a configuration file line
-draftmode              switch on draft mode (generates no output PDF)
-etex                   enable e-TeX extensions
[-no]-file-line-error   disable/enable file:line:error style messages
-fmt=FMTNAME            use FMTNAME instead of program name or a %& line
-halt-on-error          stop processing at the first error
-ini                    be pdfinitex, for dumping formats; this is implicitly
                          true if the program name is `pdfinitex'
-interaction=STRING     set interaction mode (STRING=batchmode/nonstopmode/
                          scrollmode/errorstopmode)
-jobname=STRING         set the job name to STRING
-output-comment=STRING  use STRING for DVI file comment instead of date
                          (no effect for PDF)
-output-directory=DIR   use existing DIR as the directory to write files in
-output-format=FORMAT   use FORMAT for job output; FORMAT is `dvi' or `pdf'
[-no]-parse-first-line  disable/enable parsing of first line of input file
-progname=STRING        set program (and fmt) name to STRING
-recorder               enable filename recorder
[-no]-shell-escape      disable/enable \\write18{SHELL COMMAND}
-shell-restricted       enable restricted \\write18
-translate-file=TCXNAME use the TCX file TCXNAME
-8bit                   make all characters printable by default
-help                   display this help and exit
-version                output version information and exit

Not implemented by this engine yet: -enc, -ipc, -ipc-start, -kpathsea-debug,
-mktex, -no-mktex, -mltex, -src-specials, -synctex.
";

/// `-version` (printversion.c's layout): pdfTeX's version, and what this
/// program is.
pub fn version_text() -> String {
    "pdfTeX 3.141592653-2.6-1.40.29 (FlashTeX engine)\n\
     FlashTeX's engine is a translation of pdfTeX 1.40.29's pdftex.web\n\
     (TeX Live 2026) by tools/web2rust, with kpathsea from TeX Live.\n\
     Copyright 2026 Han The Thanh (pdfTeX) et al.\n\
     There is NO warranty.  Redistribution of this software is\n\
     covered by the terms of the GNU General Public License,\n\
     version 2 or (at your option) any later version.\n"
        .to_string()
}

/// `TEX_format_default` (maininit's `DUMP_VAR`): `dump_name`, with `.fmt`
/// added unless it is already there.
fn default_format_file() -> String {
    let d = run().dump_name;
    if d.len() > 4 && d[d.len() - 4..].eq_ignore_ascii_case(".fmt") {
        d
    } else {
        format!("{d}.fmt")
    }
}

/// The process's file resolver (see `resolver.rs`). kpathsea keeps its state
/// in the environment, so there is one per process; `set_resolver` replaces
/// the default chosen on first use.
static RESOLVER: Mutex<Option<Box<dyn FileResolver>>> = Mutex::new(None);

pub fn set_resolver(r: Box<dyn FileResolver>) {
    *RESOLVER.lock().unwrap() = Some(r);
}

/// The program name the current default resolver was made for.
static RESOLVER_PROG: Mutex<Option<String>> = Mutex::new(None);

/// kpathsea's `kpse_reset_program_name`: the next lookup starts a resolver
/// for the current program name. A resident host that configures the
/// process again for the same program name (and no `-cnf-line`) keeps the
/// resolver it has: kpathsea's start-up (texmf.cnf, the `ls-R` databases)
/// is most of a fresh process's time to a first page (DESIGN.md §1.2's
/// reopen target).
fn reset_resolver(prog: &str, keep_allowed: bool) {
    let mut p = RESOLVER_PROG.lock().unwrap();
    if keep_allowed && p.as_deref() == Some(prog) && RESOLVER.lock().unwrap().is_some() {
        return;
    }
    *p = Some(prog.to_string());
    *RESOLVER.lock().unwrap() = None;
}

/// The engine name kpathsea sees (`$engine` in texmf.cnf), which selects the
/// format directory `web2c/$engine`. Formats of this engine are not pdfTeX's,
/// so they live apart from pdfTeX's `web2c/pdftex`.
pub const ENGINE_NAME: &str = "flashtex";

fn with_resolver<T>(f: impl FnOnce(&mut dyn FileResolver) -> T) -> T {
    let prog = run().program_name;
    let mut g = RESOLVER.lock().unwrap();
    let r = g.get_or_insert_with(|| crate::resolver::default_resolver(&prog, ENGINE_NAME));
    f(r.as_mut())
}

/// `kpse_invocation_name`: what pdfTeX's C parts name the program in their
/// warnings, `argv[0]` as the program was invoked (a path, if it was run by
/// its path).
pub fn invocation_name() -> String {
    run().invocation_name
}

/// `kpse_var_value`: a texmf.cnf variable, the environment first. Where
/// the resolver has no texmf.cnf (the trip tests), only the environment,
/// as kpathsea reads it (`VAR_progname`, then `VAR`).
pub fn texmf_var(var: &str) -> Option<String> {
    if let Some(v) = with_resolver(|r| r.config_var(var)) {
        return Some(v);
    }
    let prog = run().program_name;
    std::env::var(format!("{var}_{prog}"))
        .or_else(|_| std::env::var(var))
        .ok()
}

/// texmf.cnf's yes/no settings as web2c's `texmf_yesno` reads them: true
/// if the value starts with `1`, `y` or `t`.
pub fn texmf_yesno(var: &str) -> bool {
    matches!(
        texmf_var(var).and_then(|v| v.bytes().next()),
        Some(b'1' | b'y' | b't')
    )
}

/// tex.ch's `setup_bound_var` (lib/setupvar.c): texmf.cnf's value for
/// `name`, else `default`; a negative value, or 0 where the default is
/// positive, is refused with a warning.
pub fn setup_bound_var(name: &str, default: i32) -> i32 {
    let Some(v) = texmf_var(name) else {
        return default;
    };
    // atoi: leading blanks, a sign, digits.
    let t = v.trim_start();
    let (neg, t) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    let mut n: i64 = digits.parse().unwrap_or(0);
    if neg {
        n = -n;
    }
    let n = n.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    if n < 0 || (n == 0 && default > 0) {
        eprintln!(
            "{}: Bad value ({n}) in environment or texmf.cnf for {name}, keeping {default}.",
            invocation_name()
        );
        return default;
    }
    n
}

/// `kpse_find_tex(name)`, for the C parts' `find_input_file`
/// (`kpse_find_file(name, kpse_tex_format, true)`).
pub fn find_input(name: &str) -> Option<String> {
    let p = resolve_ex(name, Format::Tex, true);
    if let Some(p) = &p {
        read_set_open(p);
    }
    p
}

/// `kpse_find_file(name, format)`, for the C parts (font map files, ...).
pub fn find_file(name: &str, format: Format) -> Option<String> {
    let p = resolve(name, format);
    if let Some(p) = &p {
        read_set_open(p);
    }
    p
}

/// A format file: the resolver's search path first (`TEXFORMATS`, which
/// starts with the current directory, so a format a build made for itself
/// wins, as with pdfTeX), then `FLASHTEX_FORMATS` (a colon-separated list
/// of directories where a caller keeps this engine's formats, standing in
/// for `$TEXMF/web2c/flashtex`).
///
/// Where neither has it, the format cache (`crate::formats`, DESIGN.md
/// 4.4) builds it from the files the resolver sees, as TeX Live's mktexfmt
/// would build it with fmtutil, unless `FLASHTEX_FORMAT_CACHE=off`.
fn find_format(name: &str) -> Option<String> {
    if let Some(p) = resolve(name, Format::Fmt) {
        return Some(p);
    }
    if name.contains('/') {
        return None;
    }
    if let Ok(dirs) = std::env::var("FLASHTEX_FORMATS") {
        if let Some(p) = dirs
            .split(':')
            .filter(|d| !d.is_empty())
            .map(|d| Path::new(d).join(name))
            .find(|p| p.is_file())
        {
            return Some(p.to_string_lossy().into_owned());
        }
    }
    #[cfg(feature = "distribution")]
    if !run().ini && crate::formats::cache_enabled() {
        let fmt = name.strip_suffix(".fmt")?;
        let prog = run().program_name;
        return match with_resolver(|r| crate::formats::ensure_format(fmt, &prog, r)) {
            Ok(p) => Some(p.to_string_lossy().into_owned()),
            Err(crate::formats::FormatError::NotInFmtutil(_)) => None,
            Err(e) => {
                eprintln!("{}: {e}", invocation_name());
                None
            }
        };
    }
    None
}

// ---------------------------------------------------------------------------
// The read set, for the format cache (`crate::formats`)
// ---------------------------------------------------------------------------

/// With `FLASHTEX_READ_SET=<file>`, every lookup the engine makes and every
/// file it opens for reading are appended to that file, one per line:
/// `lookup\t<format>\t<name>\t<path, or nothing>` and `open\t<path>`. The
/// format cache runs its INITEX builds with it to learn exactly what a
/// format was made from. Without the variable it costs one check per file.
static READ_SET: Mutex<Option<Option<File>>> = Mutex::new(None);

fn read_set_note(line: &str) {
    let mut g = READ_SET.lock().unwrap();
    let f = g.get_or_insert_with(|| {
        std::env::var_os("FLASHTEX_READ_SET").and_then(|p| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(p)
                .ok()
        })
    });
    if let Some(f) = f.as_mut() {
        let _ = f.write_all(format!("{line}\n").as_bytes());
    }
}

fn read_set_lookup(name: &str, format: Format, must_exist: bool, found: Option<&str>) {
    read_set_note(&format!(
        "lookup\t{}\t{}\t{name}\t{}",
        format.kpse_name(),
        must_exist as u8,
        found.unwrap_or("")
    ));
}

fn read_set_open(path: &str) {
    read_set_note(&format!("open\t{path}"));
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

/// `kpse_find_file(name, format, must_exist)` as web2c's `open_input` asks
/// it; a file an mktex script made is recorded as an external effect.
fn resolve_ex(name: &str, format: Format, must_exist: bool) -> Option<String> {
    let (found, made) = with_resolver(|r| r.find_ex(name, format, must_exist));
    let found = found.map(|p| p.to_string_lossy().into_owned());
    read_set_lookup(name, format, must_exist, found.as_deref());
    if made {
        record_effect("mktex", name.as_bytes());
    }
    note_lookup(name, format, Some(must_exist), found.as_deref());
    found
}

/// tex.ch's `tex_input_type`: 1 while `\input` opens a file, 0 for
/// `\openin`; `open_input` asks kpathsea with `must_exist` for the first.
static TEX_INPUT_TYPE: AtomicBool = AtomicBool::new(false);

fn resolve(name: &str, format: Format) -> Option<String> {
    // kpathsea's program name selects the search paths (`TEXINPUTS.pdflatex`
    // finds latex.ltx, `TEXINPUTS.pdftex` does not); the engine name selects
    // the format directory.
    let found = with_resolver(|r| {
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
        read_set_lookup(name, format, false, found.as_deref());
        found
    });
    note_lookup(name, format, None, found.as_deref());
    found
}

// ---------------------------------------------------------------------------
// The recorder (lib/openclose.c) and the shell (texmfmp.c)
// ---------------------------------------------------------------------------

struct Recorder {
    name: String,
    file: File,
}

static RECORDER: Mutex<Option<Recorder>> = Mutex::new(None);

/// `recorder_record_input`/`recorder_record_output`: with `-recorder`, each
/// file opened is listed in `<program><pid>.fls` (renamed to
/// `<jobname>.fls` when the log file opens), after a `PWD` line.
fn record_file(prefix: &str, name: &str) {
    if prefix == "INPUT" {
        note_file(name);
    } else {
        note_output(name);
    }
    let r = run();
    if !r.recorder {
        return;
    }
    let mut g = RECORDER.lock().unwrap();
    if g.is_none() {
        let mut fname = format!("{}{}.fls", r.program_name, std::process::id());
        if let Some(d) = &r.output_directory {
            fname = format!("{d}/{fname}");
        }
        let Ok(mut file) = File::create(&fname) else {
            eprintln!("{}: fopen({fname}) failed", r.invocation_name);
            std::process::exit(1);
        };
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let _ = writeln!(file, "PWD {cwd}");
        *g = Some(Recorder { name: fname, file });
    }
    if let Some(rec) = g.as_mut() {
        let _ = writeln!(rec.file, "{prefix} {name}");
        let _ = rec.file.flush();
    }
}

/// `recorder_change_filename`.
fn recorder_change_filename(new_name: &str) {
    let mut g = RECORDER.lock().unwrap();
    let Some(rec) = g.as_mut() else { return };
    let mut new_name = new_name.to_string();
    if let Some(d) = run().output_directory {
        new_name = format!("{d}/{new_name}");
    }
    if std::fs::rename(&rec.name, &new_name).is_ok() {
        rec.name = new_name;
    }
}

/// texmfmp.c's `shell_cmd_is_allowed` for restricted shell escape: -1 for
/// a quoting error, 0 if the command is not in `shell_escape_commands`, 2
/// with the command re-quoted (every argument in `'...'`) if it is.
fn shell_cmd_is_allowed(cmd: &[u8], commands: &[String]) -> (i32, Vec<u8>) {
    let is_space = |c: u8| c == b' ' || c == b'\t';
    let start = cmd.iter().position(|&c| !is_space(c)).unwrap_or(cmd.len());
    let end = cmd[start..]
        .iter()
        .position(|&c| is_space(c))
        .map_or(cmd.len(), |n| start + n);
    let cmdname = &cmd[start..end];
    if !commands.iter().any(|c| c.as_bytes() == cmdname) {
        return (0, vec![]);
    }
    const QUOTE: u8 = b'\'';
    let mut d: Vec<u8> = cmdname.to_vec();
    let mut s = end;
    let mut pre = true;
    while s < cmd.len() {
        let c = cmd[s];
        if c == b'\'' {
            return (-1, vec![]);
        }
        if c == b'"' {
            if !pre {
                d.push(QUOTE);
            }
            pre = false;
            d.push(QUOTE);
            s += 1;
            while s < cmd.len() && cmd[s] != b'"' {
                if cmd[s] == b'\'' {
                    return (-1, vec![]);
                }
                d.push(cmd[s]);
                s += 1;
            }
            if s >= cmd.len() {
                return (-1, vec![]);
            }
            s += 1;
            if s < cmd.len() && !is_space(cmd[s]) {
                return (-1, vec![]);
            }
        } else if pre && !is_space(c) {
            pre = false;
            d.push(QUOTE);
            d.push(c);
            s += 1;
        } else if !pre && is_space(c) {
            pre = true;
            d.push(QUOTE);
            d.push(c);
            s += 1;
        } else {
            d.push(c);
            s += 1;
        }
    }
    if !pre {
        d.push(QUOTE);
    }
    (2, d)
}

fn shell_command(cmd: &[u8]) -> std::process::Command {
    use std::os::unix::ffi::OsStrExt;
    let mut c = std::process::Command::new("/bin/sh");
    c.arg("-c").arg(std::ffi::OsStr::from_bytes(cmd));
    c
}

/// A command the run executed: `\write18` (`runsystem`), or the command
/// behind `\input|cmd` or an `\openout` to `|cmd` (`runpopen`). Each is an
/// effect outside the engine's state, which an incremental rerun cannot
/// replay from a snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalEffect {
    /// `write18`, `pipe-in`, `pipe-out`, or `mktex` (a file an mktex script
    /// such as `mktextfm` made; `command` is then the file's name).
    pub kind: &'static str,
    /// The command as executed (after restricted-mode quoting).
    pub command: Vec<u8>,
}

static EXTERNAL_EFFECTS: Mutex<Vec<ExternalEffect>> = Mutex::new(Vec::new());

/// Every command this run has executed, in order.
pub fn external_effects() -> Vec<ExternalEffect> {
    EXTERNAL_EFFECTS.lock().unwrap().clone()
}

/// Record an executed command; with `FLASHTEX_EXTERNAL_EFFECTS=<file>` also
/// append it there as one line, `<kind> <command>`, for a caller outside
/// the process.
fn record_effect(kind: &'static str, command: &[u8]) {
    note_barrier(kind);
    EXTERNAL_EFFECTS.lock().unwrap().push(ExternalEffect {
        kind,
        command: command.to_vec(),
    });
    if let Some(p) = std::env::var_os("FLASHTEX_EXTERNAL_EFFECTS") {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
        {
            let mut line = kind.as_bytes().to_vec();
            line.push(b' ');
            line.extend_from_slice(command);
            line.push(b'\n');
            let _ = f.write_all(&line);
        }
    }
}

/// texmfmp.c's `runsystem`, for `\write18`: 0 if not allowed, 1 if run
/// (any command allowed), 2 if run as an allowed restricted command, -1
/// for a quoting error. The command's exit status is only reported.
pub fn runsystem(cmd: &[u8]) -> i32 {
    let r = run();
    if !r.shell_enabled {
        return 0;
    }
    let (allow, safecmd) = if r.restricted_shell {
        shell_cmd_is_allowed(cmd, &r.shell_commands)
    } else {
        (1, cmd.to_vec())
    };
    if allow == 2 && safecmd.contains(&b'|') {
        return 0;
    }
    if allow == 1 || allow == 2 {
        let _ = std::io::stdout().flush();
        record_effect("write18", &safecmd);
        let status = shell_command(&safecmd)
            .status()
            .map(|s| s.code().unwrap_or(-1))
            .unwrap_or(127);
        if status != 0 {
            // system(3)'s status is the wait status: the code times 256.
            eprintln!("system returned with code {}", status * 256);
        }
    }
    allow
}

/// texmfmp.c's `runpopen`: the command behind `\input|cmd` (reading) or
/// `\openout` to `|cmd` (writing), subject to the same restrictions as
/// `\write18`.
fn run_popen(cmd: &str, read: bool) -> Option<std::process::Child> {
    let r = run();
    let (allow, safecmd) = if r.restricted_shell {
        shell_cmd_is_allowed(cmd.as_bytes(), &r.shell_commands)
    } else {
        (1, cmd.as_bytes().to_vec())
    };
    match allow {
        1 | 2 => {
            record_effect(if read { "pipe-in" } else { "pipe-out" }, &safecmd);
            let mut c = shell_command(&safecmd);
            if read {
                c.stdout(std::process::Stdio::piped());
            } else {
                c.stdin(std::process::Stdio::piped());
            }
            c.spawn().ok()
        }
        -1 => {
            eprintln!("\nrunpopen quotation error in command line: {cmd}");
            None
        }
        _ => {
            let name = cmd.split([' ', '\t']).find(|w| !w.is_empty()).unwrap_or("");
            eprintln!("\nrunpopen command not allowed: {name}");
            None
        }
    }
}

impl Globals {
    /// `name_of_file`, trimmed. `name_length` is authoritative when set, but
    /// §51 opens the pool file without setting it.
    /// Its bytes are the file system's (UTF-8 names come out as they went
    /// in); a name that is not UTF-8 is read with replacement characters.
    fn raw_file_name(&self) -> String {
        let raw: &[u8] = if self.name_length > 0 {
            &self.name_of_file[..self.name_length as usize]
        } else {
            &self.name_of_file[..]
        };
        String::from_utf8_lossy(raw).trim().to_string()
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

    /// Where to open an input file (lib/openclose.c's `open_input`): a
    /// relative name in `-output-directory` first, as it is, else looked up
    /// by the resolver in the format its device name implies. As web2c
    /// does, the path found is written back into `name_of_file`, so
    /// `a_make_name_string` -- and therefore the `(` line in the log --
    /// shows `./story.tex` exactly as pdfTeX's does.
    fn input_path(&mut self, default: Format, must_exist: bool) -> Option<String> {
        let s = self.raw_file_name();
        let (area, base) = Self::split_area(&s);
        if base.eq_ignore_ascii_case("TEX.POOL") {
            let p = pool_path();
            read_set_open(&p);
            return Some(p);
        }
        let format = match area {
            "TeXfonts" => Format::Tfm,
            "TeXformats" => Format::Fmt,
            "TeXinputs" => Format::Tex,
            _ => default,
        };
        let mut found = None;
        if let Some(dir) = run().output_directory {
            if !base.starts_with('/') {
                let p = format!("{dir}/{base}");
                if Path::new(&p).is_file() {
                    found = Some(p);
                }
            }
        }
        let found = match found {
            Some(p) => p,
            None if format == Format::Fmt => find_format(base)?,
            None => resolve_ex(base, format, must_exist)?,
        };
        self.set_name_of_file(&found);
        record_file("INPUT", &found);
        read_set_open(&found);
        Some(found)
    }

    /// Replace `name_of_file` by `name`, as web2c does after opening a file.
    fn set_name_of_file(&mut self, name: &str) {
        let n = name.len();
        if n <= self.name_of_file.len() {
            self.name_of_file.fill(b' ');
            self.name_of_file[..n].copy_from_slice(name.as_bytes());
            self.name_length = n as i32;
        }
    }

    /// lib/openclose.c's `open_output`: a relative name goes into
    /// `-output-directory`; if it cannot be created there, into texmf.cnf's
    /// `TEXMFOUTPUT`. The name opened is written back into `name_of_file`.
    fn open_output_file(&mut self) -> Option<(File, String)> {
        let s = self.raw_file_name();
        let name = Self::split_area(&s).1.to_string();
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
        if f.is_some() {
            if fname != s {
                self.set_name_of_file(&fname);
            }
            record_file("OUTPUT", &fname);
        }
        f.map(|f| (f, fname))
    }

    /// A pipe instead of a file (texmfmp.c's `open_in_or_pipe`): with shell
    /// escape enabled, a name `|command` reads the command's output.
    fn open_in_pipe(&mut self, f: &mut AlphaFile) -> Option<bool> {
        let s = self.raw_file_name();
        let cmd = s.strip_prefix('|')?;
        if !run().shell_enabled {
            return None;
        }
        record_file("INPUT", cmd);
        let child = run_popen(cmd, true)?;
        f.input = Some(TextIn::Pipe(BufReader::new(child.stdout?)));
        f.have_line = f.next_line();
        f.refresh();
        f.err = 0;
        Some(true)
    }

    pub fn a_open_in(&mut self, f: &mut AlphaFile) -> bool {
        *f = AlphaFile::default();
        f.err = 1;
        if let Some(ok) = self.open_in_pipe(f) {
            return ok;
        }
        // open_input: must_exist unless this is \openin.
        let must_exist = TEX_INPUT_TYPE.load(Ordering::SeqCst);
        let Some(name) = self.input_path(Format::Tex, must_exist) else {
            return false;
        };
        match File::open(&name) {
            Ok(h) => {
                #[cfg(not(feature = "tex82"))]
                if self.arena.extra.is_some() && name.ends_with(".aux") {
                    self.note_aux_open();
                }
                f.input = Some(TextIn::File(BufReader::new(h)));
                f.path = Some(name);
                f.have_line = f.next_line();
                f.refresh();
                f.err = 0;
                true
            }
            Err(_) => false,
        }
    }

    pub fn a_open_out(&mut self, f: &mut AlphaFile) -> bool {
        *f = AlphaFile::default();
        f.err = 1;
        // texmfmp.c's `open_out_or_pipe`: `|command` writes to the command;
        // a `.tex` TeX added is dropped when the command is one word.
        let s = self.raw_file_name();
        if let Some(cmd) = s.strip_prefix('|').filter(|_| run().shell_enabled) {
            let cmd = if !cmd.contains(' ') && !cmd.contains('>') {
                cmd.strip_suffix(".tex").unwrap_or(cmd)
            } else {
                cmd
            };
            record_file("OUTPUT", cmd);
            let Some(mut child) = run_popen(cmd, false) else {
                return false;
            };
            let Some(stdin) = child.stdin.take() else {
                return false;
            };
            f.output = Some(BufWriter::new(Box::new(stdin)));
            f.child = Some(child);
            f.err = 0;
            return true;
        }
        match self.open_output_file() {
            Some((h, name)) => {
                f.output = Some(BufWriter::new(Box::new(h)));
                f.path = Some(name);
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
                // Pascal's `reset` leaves `f^` holding the first component:
                // `read_sixteen` (§565) reads `fbyte` before its first `fget`.
                get_byte(f);
                true
            }
            Err(_) => false,
        }
    }

    /// Is this INITEX? (changes/virtex.ch)
    pub fn ini_version(&mut self) -> bool {
        run().ini
    }

    /// Was `-etex` given? (changes/virtex.ch)
    pub fn etex_p(&mut self) -> bool {
        run().etex
    }

    /// The default format's name, on the terminal (tex.ch's banner).
    pub fn wterm_dump_name(&mut self) {
        wr_str(&mut self.term_out, &run().dump_name);
    }

    /// The default format's file name into `name_of_file` (tex.ch's
    /// `TEX_format_default`).
    pub fn pack_default_format_name(&mut self) {
        let name = default_format_file();
        let n = name.len().min(self.name_of_file.len());
        self.name_of_file.fill(b' ');
        self.name_of_file[..n].copy_from_slice(&name.as_bytes()[..n]);
        self.name_length = n as i32;
    }

    /// `fputs(name_of_file + 1, stdout)` (tex.ch [29.524]).
    pub fn wterm_name_of_file(&mut self) {
        let n = self.raw_file_name();
        wr_str(&mut self.term_out, &n);
    }

    /// `fputs(TEX_format_default + 1, stdout)` (tex.ch [29.524]).
    pub fn wterm_format_default(&mut self) {
        wr_str(&mut self.term_out, &default_format_file());
    }

    /// tex.ch's `texmf_yesno('log_openout')`: is each `\openout` logged?
    pub fn texmf_yesno_log_openout(&mut self) -> bool {
        texmf_yesno("log_openout")
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
        // open_input: must_exist except for VF files and \openin.
        let must_exist = format != Format::Vf
            && (format != Format::Tex || TEX_INPUT_TYPE.load(Ordering::SeqCst));
        let Some(name) = self.input_path(format, must_exist) else {
            return false;
        };
        match File::open(&name) {
            Ok(h) => {
                f.input = Some(BufReader::new(h));
                f.path = Some(name);
                f.err = 0;
                get_byte(f);
                true
            }
            Err(_) => false,
        }
    }

    pub fn b_open_out(&mut self, f: &mut ByteFile) -> bool {
        *f = ByteFile::default();
        match self.open_output_file() {
            Some((h, name)) => {
                f.output = Some(BufWriter::new(h));
                f.path = Some(name);
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
                // As for `b_open_in`: §1307 reads `fmt_file^.int` before the
                // first `undump_wd`, which itself starts with a `get`.
                get_word(f);
                true
            }
            Err(_) => false,
        }
    }

    pub fn w_open_out(&mut self, f: &mut WordFile) -> bool {
        *f = WordFile::default();
        match self.open_output_file() {
            Some((h, name)) => {
                f.output = Some(BufWriter::new(h));
                f.path = Some(name);
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
        if let Some(TextIn::File(r)) = f.input.as_mut() {
            if let Ok((p, off)) = in_offset(r, &f.path) {
                note_close(&p, off);
            }
        }
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
        #[cfg(not(feature = "tex82"))]
        if self.arena.extra.is_some() {
            self.maybe_request_timed_checkpoint();
        }
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
// The routines changes/web2c-run.ch declares
// ---------------------------------------------------------------------------

/// texmfmp.c's `find_suffix`: the extension of the last path component.
fn has_suffix(name: &str) -> bool {
    let base = name.rsplit('/').next().unwrap_or(name);
    base.contains('.')
}

/// texmfmp.c's `tcx_get_num`: `strtol(start, post, 0)`, i.e. leading
/// blanks, a sign, then hexadecimal after `0x`, octal after `0`, else
/// decimal; -1 (with a warning unless the rest is blank) when there is no
/// number or it is out of `0..=upb`.
fn tcx_get_num(upb: i64, line_no: usize, s: &[u8], file: &str) -> (i64, usize) {
    let mut i = 0;
    while i < s.len() && s[i].is_ascii_whitespace() {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let (radix, mut j) = if s.len() > i + 1
        && s[i] == b'0'
        && (s[i + 1] == b'x' || s[i + 1] == b'X')
        && s.get(i + 2).is_some_and(|c| c.is_ascii_hexdigit())
    {
        (16, i + 2)
    } else if s.get(i) == Some(&b'0') {
        (8, i)
    } else {
        (10, i)
    };
    let digits_start = j;
    let mut v: i64 = 0;
    while j < s.len() && (s[j] as char).is_digit(radix) {
        v = v
            .saturating_mul(radix as i64)
            .saturating_add((s[j] as char).to_digit(radix).unwrap() as i64);
        j += 1;
    }
    if j == digits_start {
        // Could not get a number. If blank line, fine. Else complain.
        let rest = s;
        if rest.iter().any(|c| !c.is_ascii_whitespace()) {
            eprintln!(
                "{file}:{line_no}: Expected numeric constant, not `{}'.",
                String::from_utf8_lossy(rest)
            );
        }
        return (-1, 0);
    }
    if neg {
        v = -v;
    }
    if v < 0 || v > upb {
        eprintln!("{file}:{line_no}: Destination charcode {v} <0 or >{upb}.");
        return (-1, j);
    }
    (v, j)
}

#[cfg(not(feature = "tex82"))]
impl Globals {
    /// tex.ch's `setup_bound_var` calls for the values that are variables
    /// here (their defaults are tex.ch's).
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
    /// texmfmp.c's `pdfoutputoption`/`pdfoutputvalue` (`-output-format`)
    /// and `pdfdraftmodeoption`/`pdfdraftmodevalue` (`-draftmode`).
    pub fn web2c_pdf_options(
        &mut self,
        o_opt: &mut i32,
        o_val: &mut i32,
        d_opt: &mut i32,
        d_val: &mut i32,
    ) {
        let r = run();
        (*o_opt, *o_val) = match r.output_format {
            Some(v) => (1, v),
            None => (0, 0),
        };
        (*d_opt, *d_val) = if r.draftmode { (1, 1) } else { (0, 0) };
    }

    /// web2c's `versionstring` after the banner, on the terminal.
    pub fn wterm_version_string(&mut self) {
        wr_str(&mut self.term_out, WEB2C_VERSION);
    }

    /// The same in the log.
    pub fn wlog_version_string(&mut self) {
        wr_str(&mut self.log_file, WEB2C_VERSION);
    }

    /// utils.c's `makepdftexbanner`: `pdftex_banner` becomes the string
    /// `BANNER versionstring kpathsea_version_string`, once per run. (C's
    /// `static boolean pdftexbanner_init` lives as long as the process,
    /// which is one run; a resident host makes many runs in one process, so
    /// the flag is the engine's own `pdftex_banner`, nonzero once made.)
    pub fn make_pdftex_banner(&mut self) {
        if self.pdftex_banner != 0 {
            return;
        }
        let s = format!(
            "{BANNER}{WEB2C_VERSION} {}",
            crate::resolver::kpathsea_version()
        );
        for b in s.bytes() {
            self.str_pool[self.pool_ptr as usize] = b as _;
            self.pool_ptr += 1;
        }
        self.pdftex_banner = self.make_string();
    }

    /// The TCX file's name, as found, into the log (`fputs` in tex.ch).
    pub fn wlog_translate_filename(&mut self) {
        if let Some(t) = run().translate_filename {
            for b in t.bytes() {
                wr_char(&mut self.log_file, b);
            }
        }
    }

    /// The same on the terminal (tex.ch [5.61]).
    pub fn wterm_translate_filename(&mut self) {
        if let Some(t) = run().translate_filename {
            wr_str(&mut self.term_out, &t);
        }
    }

    /// texmfmp.c's `readtcxfile`: the TCX file named by `-translate-file`
    /// (`.tcx` added if it has no extension), found along the web2c path,
    /// updates `xord`, `xchr` and `xprn`. Its full name replaces the one
    /// given, and is what the log shows.
    pub fn read_tcx_file(&mut self) {
        let Some(mut name) = run().translate_filename else {
            return;
        };
        if !has_suffix(&name) {
            name.push_str(".tcx");
        }
        let Some(found) = resolve(&name, Format::Web2c) else {
            with_run(|r| r.translate_filename = Some(name.clone()));
            eprintln!(
                "{}: warning: Could not open char translation file `{name}'.",
                invocation_name()
            );
            return;
        };
        with_run(|r| r.translate_filename = Some(found.clone()));
        read_set_open(&found);
        let Ok(f) = File::open(&found) else {
            eprintln!("{}: fopen({found}) failed", invocation_name());
            std::process::exit(1);
        };
        let mut r = BufReader::new(f);
        let mut line = vec![];
        let mut line_no = 0;
        while read_tex_line(&mut r, &mut line) {
            if let Some(i) = line.iter().position(|&c| c == b'%') {
                line.truncate(i);
            }
            line_no += 1;
            let (first, p1) = tcx_get_num(255, line_no, &line, &found);
            if first < 0 {
                continue;
            }
            let rest = &line[p1..];
            let (second, p2) = tcx_get_num(255, line_no, rest, &found);
            let (second, printable) = if second >= 0 {
                self.xord[first as usize] = second as _;
                self.xchr[second as usize] = first as _;
                let (mut printable, _) = tcx_get_num(1, line_no, &rest[p2..], &found);
                if printable == -1 {
                    printable = 1;
                }
                if (32..=126).contains(&second) {
                    printable = 1;
                }
                (second, printable)
            } else {
                (first, 1)
            };
            self.xprn[second as usize] = printable != 0;
        }
    }

    /// tex.ch's `do_final_end`.
    pub fn do_final_end(&mut self) {
        final_end(self)
    }

    /// texmfmp.c's `getjobname`: `-jobname`, else `s`.
    pub fn get_job_name(&mut self, s: i32) -> i32 {
        match run().job_name {
            Some(j) => {
                for b in j.bytes() {
                    self.str_pool[self.pool_ptr as usize] = b as _;
                    self.pool_ptr += 1;
                }
                self.make_string()
            }
            None => s,
        }
    }

    /// `recorder_change_filename(name_of_file)`.
    pub fn recorder_change_filename(&mut self) {
        let n = self.raw_file_name();
        recorder_change_filename(&n);
    }

    /// tex.ch's `tex_input_type:=1` (`\input`) or `:=0` (`\openin`).
    pub fn set_tex_input_type(&mut self, input: bool) {
        TEX_INPUT_TYPE.store(input, Ordering::SeqCst);
    }

    pub fn kpse_in_name_ok(&mut self) -> bool {
        let n = self.raw_file_name();
        with_resolver(|r| r.name_ok(&n, false))
    }

    pub fn kpse_out_name_ok(&mut self) -> bool {
        let n = self.raw_file_name();
        with_resolver(|r| r.name_ok(&n, true))
    }

    /// texmfmp.c's `runsystem` on `str_pool[s..s+l-1]`.
    pub fn runsystem(&mut self, s: i32, l: i32) -> i32 {
        let cmd: Vec<u8> = (s..s + l)
            .map(|k| self.str_pool[k as usize] as u8)
            .collect();
        runsystem(&cmd)
    }
}

// ---------------------------------------------------------------------------
// The non-local gotos of the main program
// ---------------------------------------------------------------------------

/// `goto final_end` (label 9999): `ready_already:=0; end.`
pub fn final_end(g: &mut Globals) -> ! {
    g.ready_already = 0;
    let code = if g.history > 1 { 1 } else { 0 };
    exit_process(g, code)
}

/// C's `exit(code)`, which every way out of web2c's pdfTeX ends in: all
/// stdio streams are flushed first -- the terminal (tex.ch's
/// `update_terminal`), the log, the `\write` files and the DVI or PDF file
/// -- so a run that stops early (`pdftex_fail`, `-halt-on-error`) leaves
/// complete files behind.
pub fn exit_process(g: &mut Globals, code: i32) -> ! {
    let _ = std::io::stdout().flush();
    g.log_file.flush();
    for f in g.write_file.iter_mut() {
        f.flush();
    }
    g.dvi_file.flush();
    #[cfg(not(feature = "tex82"))]
    g.pdf_file.flush();
    #[cfg(feature = "bench-count-writes")]
    {
        let v = g.arena.write_counts_by_region();
        let total: u64 = v.iter().map(|x| x.1).sum();
        eprintln!(
            "writes: {total}; same chunk as the array's previous write: {}",
            crate::arena::SAME_CHUNK_AS_LAST.load(std::sync::atomic::Ordering::Relaxed)
        );
        for (k, n) in v.iter().take(25) {
            eprintln!("  {k:24} {n:12} {:5.1}%", *n as f64 * 100.0 / total as f64);
        }
    }
    if RESIDENT.with(|r| r.get()) {
        // A resident engine (src/host/) outlives the run: unwind to the
        // host instead of ending the process. `resume_unwind` does not call
        // the panic hook, so nothing is printed.
        TERMINATING.store(false, Ordering::SeqCst);
        std::panic::resume_unwind(Box::new(EngineExit(code)));
    }
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

// ---------------------------------------------------------------------------
// Resident runs and checkpoints (src/checkpoint.rs, src/host/)
// ---------------------------------------------------------------------------

thread_local! {
    /// The engine runs inside a host that outlives the run.
    static RESIDENT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// The terminal, captured instead of written to stdout.
    static TERMINAL: std::cell::RefCell<Option<Vec<u8>>> = const { std::cell::RefCell::new(None) };
    /// The run's read-set, while one is being recorded.
    static READS: std::cell::RefCell<Option<ReadLog>> = const { std::cell::RefCell::new(None) };
}

/// How a resident run ends: the exit status `final_end` would have given
/// the process.
pub struct EngineExit(pub i32);

/// Make the ends of a run unwind to the caller (`EngineExit`) instead of
/// exiting the process.
pub fn set_resident(on: bool) {
    RESIDENT.with(|r| r.set(on));
}

/// Forget that a run was terminating (a new run in the same process).
pub fn reset_run_flags() {
    TERMINATING.store(false, Ordering::SeqCst);
}

/// Capture the terminal into memory from now on (`Some`) or write it to
/// stdout again (`None`); returns what was captured.
pub fn capture_terminal(on: bool) -> Option<Vec<u8>> {
    TERMINAL.with(|t| std::mem::replace(&mut *t.borrow_mut(), on.then(Vec::new)))
}

fn terminal_captured() -> bool {
    TERMINAL.with(|t| t.borrow().is_some())
}

fn terminal_capture_byte(b: u8) -> bool {
    TERMINAL.with(|t| match t.borrow_mut().as_mut() {
        Some(v) => {
            v.push(b);
            true
        }
        None => false,
    })
}

/// Bytes captured from the terminal so far.
pub fn terminal_len() -> usize {
    TERMINAL.with(|t| t.borrow().as_ref().map_or(0, |v| v.len()))
}

/// The captured terminal.
pub fn terminal_bytes() -> Vec<u8> {
    TERMINAL.with(|t| t.borrow().clone().unwrap_or_default())
}

/// Cut the captured terminal back to `n` bytes (restoring a checkpoint).
pub fn truncate_terminal(n: usize) {
    TERMINAL.with(|t| {
        if let Some(v) = t.borrow_mut().as_mut() {
            v.truncate(n);
        }
    })
}

/// Append to the captured terminal (restoring a persisted snapshot).
pub fn append_terminal(b: &[u8]) {
    TERMINAL.with(|t| {
        if let Some(v) = t.borrow_mut().as_mut() {
            v.extend_from_slice(b);
        }
    })
}

/// Number of external effects (`\write18`, pipes, mktex) so far.
pub fn external_effects_len() -> usize {
    EXTERNAL_EFFECTS.lock().unwrap().len()
}

/// Forget the external effects after the first `n` (restoring).
pub fn truncate_external_effects(n: usize) {
    EXTERNAL_EFFECTS.lock().unwrap().truncate(n);
}

pub fn tex_input_type() -> bool {
    TEX_INPUT_TYPE.load(Ordering::SeqCst)
}

pub fn set_tex_input_type_flag(v: bool) {
    TEX_INPUT_TYPE.store(v, Ordering::SeqCst);
}

// ---- the read-set ----------------------------------------------------------

/// A file's identity as the file system reports it: a cheap test for
/// "unchanged" before its content is hashed again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct StatSig {
    pub len: u64,
    pub mtime_ns: i128,
    pub ino: u64,
}

impl StatSig {
    pub fn of(path: &str) -> Option<StatSig> {
        let m = std::fs::metadata(path).ok()?;
        let mtime_ns = m
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos() as i128);
        #[cfg(unix)]
        let ino = std::os::unix::fs::MetadataExt::ino(&m);
        #[cfg(not(unix))]
        let ino = 0;
        Some(StatSig {
            len: m.len(),
            mtime_ns,
            ino,
        })
    }
}

/// A file the run read, with its content hash when it was opened.
#[derive(Clone, Debug)]
pub struct FileRead {
    pub path: String,
    pub hash: [u64; 2],
    pub stat: StatSig,
    /// The content when it was opened, for the user's files (a relative
    /// path or one under the working directory): an edit is located by
    /// comparing it with the file now (`crate::incr`).
    pub content: Option<std::sync::Arc<Vec<u8>>>,
    /// `Some(n)`: not a read but the close of one (an incremental journal
    /// lists those for the user's files), after which the run had consumed
    /// the file's first `n` bytes (up to its lookahead). An earlier read
    /// of a file, now closed, may have seen an edit a restart point is
    /// before (`crate::incr`).
    pub closed_at: Option<u64>,
}

/// Whether `path` is one of the user's files rather than the TeX
/// distribution's.
pub fn is_user_file(path: &str) -> bool {
    if !path.starts_with('/') {
        return true;
    }
    std::env::current_dir()
        .ok()
        .is_some_and(|d| std::path::Path::new(path).starts_with(d))
}

/// A lookup the run made: the name, and the file it found or `None`. A
/// file that did not exist then and exists now changes the run as surely
/// as a changed file does.
#[derive(Clone, Debug, PartialEq)]
pub struct Lookup {
    pub name: String,
    pub format: Format,
    /// `find_ex`'s flag, or `None` for a plain `find`.
    pub must_exist: Option<bool>,
    pub found: Option<String>,
}

/// Everything a run read from outside the engine (DESIGN.md §5.1): the
/// files, the lookups, and whether it did anything whose result no key can
/// capture (a shell command, a pipe, a font made by mktex).
#[derive(Clone, Debug, Default)]
pub struct ReadLog {
    pub files: Vec<FileRead>,
    pub lookups: Vec<Lookup>,
    pub barriers: Vec<String>,
    /// Files opened for output, in order (a file the preamble writes and
    /// closes is part of what S₀ stands for).
    pub outputs: Vec<String>,
    seen: std::collections::HashSet<String>,
    /// Keep the content of the user's files read (`FileRead::content`).
    pub keep_content: bool,
    /// The directories lookups depended on, each with its stat signature
    /// the first time (`host::Key::dirs`).
    pub dirs: Vec<(String, StatSig)>,
}

impl ReadLog {
    /// A log that keeps the content of the user's files it notes.
    pub fn keeping_content() -> ReadLog {
        let mut l = ReadLog {
            keep_content: true,
            ..ReadLog::default()
        };
        l.note_cwd();
        l
    }

    /// Note the working directory's signature now (before the run can add
    /// a file to it).
    pub fn note_cwd(&mut self) {
        if !self.dirs.iter().any(|(d, _)| d == ".") {
            self.dirs
                .push((".".into(), StatSig::of(".").unwrap_or_default()));
        }
    }

    /// Mark `path` as already noted (a journal carried over from an earlier
    /// run segment, `crate::incr`).
    pub fn mark_seen(&mut self, path: &str) {
        self.seen.insert(path.to_string());
    }
}

/// Start recording into `log` (a run that continues an earlier one's
/// journal), returning the log that was being recorded.
pub fn record_reads_into(log: Option<ReadLog>) -> Option<ReadLog> {
    READS.with(|r| std::mem::replace(&mut *r.borrow_mut(), log))
}

/// Make `dst` a copy of `src` sharing its blocks (APFS `clonefile`, O(1));
/// false where the file system cannot (the caller copies instead).
pub fn clone_file(src: &str, dst: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        extern "C" {
            fn clonefile(
                src: *const std::ffi::c_char,
                dst: *const std::ffi::c_char,
                flags: u32,
            ) -> i32;
        }
        let (Ok(a), Ok(b)) = (std::ffi::CString::new(src), std::ffi::CString::new(dst)) else {
            return false;
        };
        let _ = std::fs::remove_file(dst);
        // SAFETY: two NUL-terminated paths.
        unsafe { clonefile(a.as_ptr(), b.as_ptr(), 0) == 0 }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (src, dst);
        false
    }
}

/// The files opened for output after the first `n` the log lists.
pub fn outputs_since(n: usize) -> Vec<String> {
    READS.with(|r| {
        r.borrow()
            .as_ref()
            .map(|l| l.outputs.get(n..).unwrap_or(&[]).to_vec())
            .unwrap_or_default()
    })
}

/// How many files, lookups and outputs the log holds so far.
pub fn reads_len() -> (usize, usize, usize) {
    READS.with(|r| {
        r.borrow().as_ref().map_or((0, 0, 0), |l| {
            (l.files.len(), l.lookups.len(), l.outputs.len())
        })
    })
}

/// Start recording the read-set (or stop, returning it).
pub fn record_reads(on: bool) -> Option<ReadLog> {
    READS.with(|r| {
        std::mem::replace(
            &mut *r.borrow_mut(),
            on.then(|| {
                let mut l = ReadLog::default();
                l.note_cwd();
                l
            }),
        )
    })
}

/// A copy of the read-set recorded so far.
pub fn reads_so_far() -> Option<ReadLog> {
    READS.with(|r| r.borrow().clone())
}

fn note_lookup(name: &str, format: Format, must_exist: Option<bool>, found: Option<&str>) {
    READS.with(|r| {
        if let Some(log) = r.borrow_mut().as_mut() {
            let l = Lookup {
                name: name.to_string(),
                format,
                must_exist,
                found: found.map(str::to_string),
            };
            if !log.lookups.contains(&l) {
                log.lookups.push(l);
            }
            // The directory whose listing decides this lookup, as it was
            // the first time one depended on it (`host::Key::dirs`): the
            // found user file's, or the working directory's.
            let dir = match found {
                Some(p) if is_user_file(p) => Some(
                    std::path::Path::new(p)
                        .parent()
                        .map(|d| d.to_string_lossy().into_owned())
                        .filter(|d| !d.is_empty())
                        .unwrap_or_else(|| ".".into()),
                ),
                Some(_) => None,
                None => Some(
                    std::path::Path::new(name)
                        .parent()
                        .map(|d| d.to_string_lossy().into_owned())
                        .filter(|d| !d.is_empty() && !d.starts_with('/'))
                        .unwrap_or_else(|| ".".into()),
                ),
            };
            if let Some(d) = dir {
                if !log.dirs.iter().any(|(x, _)| *x == d) {
                    let sig = StatSig::of(&d).unwrap_or_default();
                    log.dirs.push((d, sig));
                }
            }
        }
    });
    if let Some(p) = found {
        note_file(p);
    }
}

fn note_file(path: &str) {
    READS.with(|r| {
        let mut b = r.borrow_mut();
        let Some(log) = b.as_mut() else { return };
        if !log.seen.insert(path.to_string()) {
            // Read again (a `.toc` at every \tableofcontents, a file \input
            // twice): an incremental journal lists every read, since a run
            // that converges keeps the old run's later reads (`crate::incr`
            // must know that the old future reads a file that changed).
            if log.keep_content {
                if let Some(first) = log.files.iter().find(|f| f.path == path).cloned() {
                    log.files.push(first);
                }
            }
            return;
        }
        let stat = StatSig::of(path).unwrap_or_default();
        let data = std::fs::read(path).ok();
        let hash = data
            .as_deref()
            .map(crate::persist::hash128)
            .unwrap_or([0, 0]);
        let content = data
            .filter(|_| log.keep_content && is_user_file(path))
            .map(std::sync::Arc::new);
        log.files.push(FileRead {
            path: path.to_string(),
            hash,
            stat,
            content,
            closed_at: None,
        });
    })
}

/// A user's input file closed after its first `consumed` bytes were read
/// (see `FileRead::closed_at`).
fn note_close(path: &str, consumed: u64) {
    READS.with(|r| {
        let mut b = r.borrow_mut();
        let Some(log) = b.as_mut() else { return };
        if !log.keep_content || !is_user_file(path) {
            return;
        }
        let Some(first) = log.files.iter().find(|f| f.path == path) else {
            return;
        };
        let mut e = first.clone();
        e.closed_at = Some(consumed);
        log.files.push(e);
    })
}

fn note_output(path: &str) {
    READS.with(|r| {
        if let Some(log) = r.borrow_mut().as_mut() {
            if !log.outputs.iter().any(|p| p == path) {
                log.outputs.push(path.to_string());
            }
        }
    })
}

fn note_barrier(kind: &str) {
    READS.with(|r| {
        if let Some(log) = r.borrow_mut().as_mut() {
            log.barriers.push(kind.to_string());
        }
    })
}

/// Look `name` up again, exactly as the run did.
pub fn lookup_again(l: &Lookup) -> Option<String> {
    let found = with_resolver(|r| match l.must_exist {
        Some(m) => r.find_ex(&l.name, l.format, m).0,
        None => r.find(&l.name, l.format),
    });
    found.map(|p| p.to_string_lossy().into_owned())
}

// ---- files in a checkpoint ---------------------------------------------------

/// What the generated `Globals::visit_files` calls for each file global.
pub trait FileVisit {
    fn alpha(&mut self, f: &mut AlphaFile);
    fn byte(&mut self, f: &mut ByteFile);
    fn word(&mut self, f: &mut WordFile);
}

/// Where a file global's stream is, at a checkpoint.
#[derive(Clone, Debug, PartialEq)]
pub enum Stream {
    None,
    Stdin,
    /// The command line standing in for the terminal's first lines.
    Pre(Vec<Vec<u8>>, usize),
    /// Reading `path`; `offset` is the next byte the engine has not read.
    In {
        path: String,
        offset: u64,
    },
    /// Writing `path`, which holds `len` bytes.
    Out {
        path: String,
        len: u64,
    },
    /// The terminal (stdout, or the host's capture).
    Terminal,
}

/// A file global at a checkpoint: its Pascal state and its stream.
#[derive(Clone, Debug, PartialEq)]
pub struct FileSnap {
    pub buf: u64,
    pub line: Vec<u8>,
    pub pos: usize,
    pub have_line: bool,
    pub at_eof: bool,
    pub err: i32,
    pub stream: Stream,
}

fn out_len(w: &mut dyn Write, path: &str) -> Result<u64, String> {
    w.flush().map_err(|e| format!("{path}: {e}"))?;
    std::fs::metadata(path)
        .map(|m| m.len())
        .map_err(|e| format!("{path}: {e}"))
}

fn in_offset<R: std::io::Seek>(r: &mut R, path: &Option<String>) -> Result<(String, u64), String> {
    let p = path
        .clone()
        .ok_or("an input file without a name cannot be checkpointed")?;
    let off = r.stream_position().map_err(|e| format!("{p}: {e}"))?;
    Ok((p, off))
}

fn reopen_out(path: &str, len: u64) -> Result<File, String> {
    use std::io::Seek;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| format!("{path}: {e}"))?;
    f.set_len(len).map_err(|e| format!("{path}: {e}"))?;
    f.seek(std::io::SeekFrom::Start(len))
        .map_err(|e| format!("{path}: {e}"))?;
    Ok(f)
}

fn reopen_in(path: &str, offset: u64) -> Result<BufReader<File>, String> {
    use std::io::Seek;
    let mut f = File::open(path).map_err(|e| format!("{path}: {e}"))?;
    f.seek(std::io::SeekFrom::Start(offset))
        .map_err(|e| format!("{path}: {e}"))?;
    Ok(BufReader::new(f))
}

impl AlphaFile {
    /// This file at a checkpoint. Pipes cannot be checkpointed.
    pub fn snapshot(&mut self) -> Result<FileSnap, String> {
        if self.child.is_some() {
            return Err("a pipe is open (\\input|, \\openout|)".into());
        }
        let stream = if self.to_stdout {
            Stream::Terminal
        } else if let Some(w) = self.output.as_mut() {
            let path = self.path.clone().ok_or("an output file without a name")?;
            let len = out_len(w, &path)?;
            Stream::Out { path, len }
        } else {
            match self.input.as_mut() {
                None => Stream::None,
                Some(TextIn::Stdin) => Stream::Stdin,
                Some(TextIn::Pre(lines, i)) => Stream::Pre(lines.clone(), *i),
                Some(TextIn::File(r)) => {
                    let (path, offset) = in_offset(r, &self.path)?;
                    Stream::In { path, offset }
                }
                Some(TextIn::Pipe(_)) => return Err("a pipe is open (\\input|)".into()),
            }
        };
        Ok(FileSnap {
            buf: self.buf as u64,
            line: self.line.clone(),
            pos: self.pos,
            have_line: self.have_line,
            at_eof: false,
            err: self.err,
            stream,
        })
    }

    /// Put this file back as `s` recorded it: an output file is cut back to
    /// its length then, an input file reopened at its offset.
    pub fn restore(&mut self, s: &FileSnap) -> Result<(), String> {
        PasFile::close(self);
        let mut f = AlphaFile {
            buf: s.buf as u8,
            line: s.line.clone(),
            pos: s.pos,
            have_line: s.have_line,
            err: s.err,
            ..AlphaFile::default()
        };
        match &s.stream {
            Stream::None => {}
            Stream::Terminal => f.to_stdout = true,
            Stream::Stdin => f.input = Some(TextIn::Stdin),
            Stream::Pre(lines, i) => f.input = Some(TextIn::Pre(lines.clone(), *i)),
            Stream::In { path, offset } => {
                f.input = Some(TextIn::File(reopen_in(path, *offset)?));
                f.path = Some(path.clone());
            }
            Stream::Out { path, len } => {
                f.output = Some(BufWriter::new(Box::new(reopen_out(path, *len)?)));
                f.path = Some(path.clone());
            }
        }
        *self = f;
        Ok(())
    }
}

impl ByteFile {
    pub fn snapshot(&mut self) -> Result<FileSnap, String> {
        let stream = if let Some(w) = self.output.as_mut() {
            let path = self.path.clone().ok_or("an output file without a name")?;
            let len = out_len(w, &path)?;
            Stream::Out { path, len }
        } else if let Some(r) = self.input.as_mut() {
            let (path, offset) = in_offset(r, &self.path)?;
            Stream::In { path, offset }
        } else {
            Stream::None
        };
        Ok(FileSnap {
            buf: self.buf as u32 as u64,
            line: vec![],
            pos: 0,
            have_line: false,
            at_eof: self.at_eof,
            err: self.err,
            stream,
        })
    }

    pub fn restore(&mut self, s: &FileSnap) -> Result<(), String> {
        PasFile::close(self);
        let mut f = ByteFile {
            buf: s.buf as u32 as i32,
            at_eof: s.at_eof,
            err: s.err,
            ..ByteFile::default()
        };
        match &s.stream {
            Stream::None => {}
            Stream::In { path, offset } => {
                f.input = Some(reopen_in(path, *offset)?);
                f.path = Some(path.clone());
            }
            Stream::Out { path, len } => {
                f.output = Some(BufWriter::new(reopen_out(path, *len)?));
                f.path = Some(path.clone());
            }
            other => return Err(format!("a binary file cannot be {other:?}")),
        }
        *self = f;
        Ok(())
    }
}

impl WordFile {
    pub fn snapshot(&mut self) -> Result<FileSnap, String> {
        if self.output.is_some() || self.input.is_some() {
            return Err("the format file is open".into());
        }
        Ok(FileSnap {
            buf: self.buf.to_bits(),
            line: vec![],
            pos: 0,
            have_line: false,
            at_eof: self.at_eof,
            err: self.err,
            stream: Stream::None,
        })
    }

    pub fn restore(&mut self, s: &FileSnap) -> Result<(), String> {
        PasFile::close(self);
        *self = WordFile {
            buf: memory_word::from_bits(s.buf),
            at_eof: s.at_eof,
            err: s.err,
            ..WordFile::default()
        };
        Ok(())
    }
}

impl crate::persist::Codec for Stream {
    fn enc(&self, w: &mut Vec<u8>) {
        match self {
            Stream::None => w.push(0),
            Stream::Stdin => w.push(1),
            Stream::Pre(l, i) => {
                w.push(2);
                l.enc(w);
                i.enc(w);
            }
            Stream::In { path, offset } => {
                w.push(3);
                path.enc(w);
                offset.enc(w);
            }
            Stream::Out { path, len } => {
                w.push(4);
                path.enc(w);
                len.enc(w);
            }
            Stream::Terminal => w.push(5),
        }
    }
    fn dec(r: &mut crate::persist::Reader) -> Result<Self, String> {
        use crate::persist::Codec;
        Ok(match r.take(1)?[0] {
            0 => Stream::None,
            1 => Stream::Stdin,
            2 => Stream::Pre(Codec::dec(r)?, Codec::dec(r)?),
            3 => Stream::In {
                path: Codec::dec(r)?,
                offset: Codec::dec(r)?,
            },
            4 => Stream::Out {
                path: Codec::dec(r)?,
                len: Codec::dec(r)?,
            },
            5 => Stream::Terminal,
            t => return Err(format!("bad stream tag {t}")),
        })
    }
}

crate::codec_struct!(FileSnap {
    buf,
    line,
    pos,
    have_line,
    at_eof,
    err,
    stream
});
