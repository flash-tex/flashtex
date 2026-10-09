//! The system-dependent layer: what TeX Live's C supplies to bibtex.ch's
//! Pascal, as the `external` routines `changes/flashtex.ch` declares, and
//! Pascal's text files as web2c implements them.
//!
//! Each routine names the C it reproduces (TeX Live 2026, `texk/web2c/lib`,
//! `texk/web2c/cpascal.h`, `texk/web2c/web2c/cvtbib.sed`,
//! `texk/kpathsea/getopt.c`).

use crate::generated::Globals;
use std::cell::{Cell, RefCell};
use std::io::Write;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// The kpathsea formats BibTeX searches (`kpse_bib_format`,
/// `kpse_bst_format`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Bib,
    Bst,
}

/// What BibTeX asks of its surroundings: kpathsea and the directory relative
/// names are opened in.
pub trait Host {
    /// `kpse_find_file(name, format, true)` for program `bibtex`: the path
    /// as kpathsea returns it, or `None`.
    fn find_file(&mut self, name: &[u8], format: Format) -> Option<Vec<u8>>;
    /// `kpse_var_value(name)` for program `bibtex` (`max_strings`,
    /// `TEXMFOUTPUT`, ...).
    fn var_value(&mut self, name: &str) -> Option<String>;
    /// `kpse_out_name_ok(name)`: `Err(openout_any's value)` when kpathsea
    /// refuses the name (the caller prints kpathsea's message).
    fn out_name_ok(&mut self, _name: &[u8]) -> Result<(), Vec<u8>> {
        Ok(())
    }
    /// The directory relative names are opened in (the child's working
    /// directory); `None` is the process's.
    fn cwd(&self) -> Option<&Path> {
        None
    }
    /// Standard output as the program's own: a stream, and the buffer size
    /// C's stdio has for it (`st_blksize`; 0 for a terminal's line
    /// buffering). `None`: captured into [`Outcome::stdout`].
    fn stdout_stream(&mut self) -> Option<(Box<dyn Write>, usize)> {
        None
    }
}

/// A run's result.
#[derive(Debug, Default)]
pub struct Outcome {
    /// The exit status (`uexit`), [`CRASHED`] or [`TIMED_OUT`].
    pub status: i32,
    /// What reached standard output, when it was captured
    /// ([`Host::stdout_stream`]).
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// Why the run [`CRASHED`]: the port's message, which the C program
    /// does not print (it dies of a signal).
    pub crash: Option<String>,
}

/// The status of a run that stopped where the C program's behaviour is
/// undefined: an array access out of its bounds, which in TeX Live's binary
/// is a write through a NULL or freed pointer and dies of SIGSEGV. Every
/// output then holds what C's stdio had passed on before ([`Stdio`]).
pub const CRASHED: i32 = -11;

/// The status of a run stopped at its deadline ([`run_with_deadline`]),
/// where a child process would have been killed; its files are partial.
pub const TIMED_OUT: i32 = -1000;

/// The program name kpathsea and getopt print (`argv[0]`, `my_name`).
const MY_NAME: &str = "bibtex";

/// Run BibTeX with the arguments after the program name.
pub fn run(args: &[Vec<u8>], host: Box<dyn Host>) -> Outcome {
    run_with_deadline(args, host, None)
}

/// [`run`], stopped with [`TIMED_OUT`] once `deadline` has passed: checked
/// whenever a style-file function starts, which every loop of a `.bst`
/// program does (a style's `while$` may never end).
pub fn run_with_deadline(
    args: &[Vec<u8>],
    host: Box<dyn Host>,
    deadline: Option<std::time::Instant>,
) -> Outcome {
    let mut g = Globals::new();
    let captured = Rc::new(RefCell::new(Vec::new()));
    g.host = State {
        host: Some(host),
        args: args.to_vec(),
        aux_name: vec![],
        captured: captured.clone(),
        stderr: vec![],
        deadline,
        calls: 0,
    };
    let r = catch_unwind(AssertUnwindSafe(|| g.tex_body()));
    let mut crash = None;
    let status = match r {
        Ok(()) => 0,
        Err(e) => match e.downcast::<Jump>() {
            Ok(j) => match *j {
                Jump::Exit(c) => c,
                Jump::Deadline => TIMED_OUT,
                Jump::CloseUpShop | Jump::BstDone => CRASHED,
            },
            Err(e) => {
                let m = e
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                crash = Some(m);
                CRASHED
            }
        },
    };
    // C's exit() flushes every stream; after a signal each keeps only what
    // stdio had passed on.
    for f in g.all_files() {
        match f.output.take() {
            Some(mut o) if status != CRASHED => o.flush(),
            _ => {}
        }
    }
    let stderr = std::mem::take(&mut g.host.stderr);
    drop(g);
    let stdout = std::mem::take(&mut *captured.borrow_mut());
    Outcome {
        status,
        stdout,
        stderr,
        crash,
    }
}

/// The state outside the program's globals (web2rust's `--host-state`).
pub struct State {
    host: Option<Box<dyn Host>>,
    args: Vec<Vec<u8>>,
    /// The command line's file argument (`cmdline(optind)`).
    aux_name: Vec<u8>,
    /// Standard output, when the host does not give a stream.
    captured: Rc<RefCell<Vec<u8>>>,
    stderr: Vec<u8>,
    deadline: Option<std::time::Instant>,
    /// Function calls so far (the clock is read every 1024).
    calls: u32,
}

impl Default for State {
    fn default() -> State {
        State {
            host: None,
            args: vec![],
            aux_name: vec![],
            captured: Rc::new(RefCell::new(Vec::new())),
            stderr: vec![],
            deadline: None,
            calls: 0,
        }
    }
}

impl State {
    fn host(&mut self) -> &mut dyn Host {
        self.host.as_deref_mut().expect("bibtex: no host")
    }
}

/// The ways out of a routine that C takes with `longjmp` or `exit`.
#[derive(Debug)]
enum Jump {
    /// `uexit(code)`.
    Exit(i32),
    /// `longjmp(jmp9998,1)`: `goto close_up_shop`.
    CloseUpShop,
    /// `longjmp(jmp32,1)`: `goto bst_done`.
    BstDone,
    /// The host's time limit passed.
    Deadline,
}

fn jump(j: Jump) -> ! {
    // resume_unwind does not run the panic hook: nothing is printed.
    resume_unwind(Box::new(j))
}

// ---------------------------------------------------------------------------
// Pascal's text files, as web2c's C has them (a FILE*)
// ---------------------------------------------------------------------------

/// `alpha_file`: a C `FILE*`, for reading (the whole file, read when it is
/// opened) or writing ([`Stdio`]).
#[derive(Default)]
pub struct AlphaFile {
    input: Option<Vec<u8>>,
    pos: Cell<usize>,
    /// C's end-of-file indicator (`feof`), set by a read that found none.
    at_eof: Cell<bool>,
    output: Option<Stdio>,
}

/// Only closed files are copied (web2rust's `vec![Default::default(); n]`
/// and `resize` for the `.bib` file array).
impl Clone for AlphaFile {
    fn clone(&self) -> AlphaFile {
        assert!(
            self.input.is_none() && self.output.is_none(),
            "bibtex: an open file copied"
        );
        AlphaFile::default()
    }
}

/// A C stdio output stream, passing on to its file exactly what stdio
/// would by now: a fully buffered stream (a file, a pipe) writes its buffer
/// of `block` bytes (`st_blksize`) when a write finds it full or too small,
/// a terminal (block 0) each line. So a run that stops early (a crash, a
/// kill) leaves what TeX Live's program would leave; `flush` is `fclose`
/// or `exit`. Every write BibTeX makes is smaller than a buffer, for which
/// this is stdio's rule (FreeBSD's and macOS's `__sfvwrite`, glibc's).
pub(crate) struct Stdio {
    sink: Box<dyn Write>,
    pending: Vec<u8>,
    block: usize,
}

impl Stdio {
    fn new(sink: Box<dyn Write>, block: usize) -> Stdio {
        Stdio {
            sink,
            pending: vec![],
            block,
        }
    }

    fn put(&mut self, b: &[u8]) {
        self.pending.extend_from_slice(b);
        let n = if self.block == 0 {
            self.pending
                .iter()
                .rposition(|&c| c == b'\n')
                .map_or(0, |i| i + 1)
        } else if self.pending.len() > self.block {
            (self.pending.len() - 1) / self.block * self.block
        } else {
            0
        };
        if n > 0 {
            let _ = self.sink.write_all(&self.pending[..n]);
            self.pending.drain(..n);
        }
    }

    fn flush(&mut self) {
        let _ = self.sink.write_all(&self.pending);
        let _ = self.sink.flush();
        self.pending.clear();
    }
}

/// The captured standard output.
struct Captured(Rc<RefCell<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl AlphaFile {
    fn put(&mut self, b: &[u8]) {
        if let Some(o) = &mut self.output {
            o.put(b);
        } // else C would write to a NULL FILE*; never happens
    }

    /// `getc`: the next byte, or -1 (EOF, which sets the indicator).
    fn getc(&self) -> i32 {
        let Some(d) = &self.input else {
            self.at_eof.set(true);
            return -1;
        };
        let p = self.pos.get();
        if p < d.len() {
            self.pos.set(p + 1);
            d[p] as i32
        } else {
            self.at_eof.set(true);
            -1
        }
    }

    /// `ungetc` of the byte just read.
    fn ungetc(&self) {
        self.pos.set(self.pos.get() - 1);
    }
}

/// An open file's `st_blksize` (stdio's buffer size for it).
fn block_size(f: &std::fs::File) -> usize {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let Ok(m) = f.metadata() {
            if m.blksize() > 0 {
                return m.blksize() as usize;
            }
        }
    }
    let _ = f;
    1024 // BUFSIZ
}

/// web2c's `eof` (lib/eofeoln.c).
pub fn eof(f: &AlphaFile) -> bool {
    if f.input.is_none() && f.output.is_none() {
        return true; // a NULL FILE*
    }
    if f.at_eof.get() {
        return true;
    }
    if f.getc() == -1 {
        return true;
    }
    f.ungetc();
    false
}

/// web2c's `eoln` (lib/eofeoln.c): CR and LF both end a line.
pub fn eoln(f: &AlphaFile) -> bool {
    if f.at_eof.get() {
        return true;
    }
    let c = f.getc();
    if c != -1 {
        f.ungetc();
    }
    c == b'\n' as i32 || c == b'\r' as i32 || c == -1
}

pub fn wr_char(f: &mut AlphaFile, c: u8) {
    f.put(&[c]);
}

pub fn wr_str(f: &mut AlphaFile, s: &str) {
    // WEB's strings are 8-bit text; each char is one byte (Latin-1).
    let b: Vec<u8> = s.chars().map(|c| c as u32 as u8).collect();
    f.put(&b);
}

pub fn wr_ln(f: &mut AlphaFile) {
    f.put(b"\n");
}

/// `%ld` (fixwrites), right-aligned in `width` as Pascal's `x:width`.
pub fn wr_int(f: &mut AlphaFile, v: i32, width: i32) {
    let s = v.to_string();
    for _ in s.len() as i32..width {
        f.put(b" ");
    }
    f.put(s.as_bytes());
}

/// `goto close_up_shop` from inside a routine: cvtbib.sed's
/// `longjmp(jmp9998,1)` (web2rust emits this name for label 9998).
#[allow(non_snake_case)]
pub fn end_of_TEX(_g: &mut Globals) -> ! {
    jump(Jump::CloseUpShop)
}

/// web2rust's file visitor (the engines' checkpoints); unused here.
pub trait FileVisit {
    fn alpha(&mut self, f: &mut AlphaFile);
}

// ---------------------------------------------------------------------------
// The external routines of changes/flashtex.ch
// ---------------------------------------------------------------------------

/// `name_of_file+1` as C sees it: the bytes up to the first NUL.
fn c_name(g: &Globals) -> Vec<u8> {
    let mut v = vec![];
    let mut i = 1;
    while i < g.name_of_file.len() && g.name_of_file[i] != 0 {
        v.push(g.name_of_file[i]);
        i += 1;
    }
    v
}

/// Set `name_of_file+1` to `s` (NUL-terminated), as openclose.c does after
/// it found a file elsewhere, and `name_length` to its length.
fn set_c_name(g: &mut Globals, s: &[u8]) {
    g.name_of_file.alloc_len(s.len() + 2);
    for (i, &b) in s.iter().enumerate() {
        g.name_of_file[i + 1] = b;
    }
    g.name_of_file[s.len() + 1] = 0;
    g.name_length = s.len() as i32;
}

#[cfg(unix)]
fn bytes_path(name: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(std::ffi::OsStr::from_bytes(name))
}

#[cfg(not(unix))]
fn bytes_path(name: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(name).into_owned())
}

/// kpathsea's `kpse_absolute_p(name, relative_ok)` on Unix.
fn absolute_p(name: &[u8], relative_ok: bool) -> bool {
    let is_sep = |c: u8| c == b'/' || (cfg!(windows) && c == b'\\');
    let absolute = !name.is_empty() && is_sep(name[0]);
    #[cfg(windows)]
    let absolute = absolute || (name.len() >= 2 && name[1] == b':');
    let explicit_relative = relative_ok
        && name.first() == Some(&b'.')
        && (name.get(1).is_some_and(|&c| is_sep(c))
            || (name.get(1) == Some(&b'.') && name.get(2).is_some_and(|&c| is_sep(c))));
    absolute || explicit_relative
}

impl Globals {
    fn all_files(&mut self) -> Vec<&mut AlphaFile> {
        let mut v: Vec<&mut AlphaFile> = vec![
            &mut self.standard_output,
            &mut self.log_file,
            &mut self.bbl_file,
        ];
        v.extend(self.aux_file.iter_mut());
        v.extend(self.bib_file.iter_mut());
        v
    }

    /// A path for `name` as the child process would open it.
    fn host_path(&mut self, name: &[u8]) -> PathBuf {
        let p = bytes_path(name);
        match self.host.host().cwd() {
            Some(d) if !p.is_absolute() => d.join(p),
            _ => p,
        }
    }

    /// `fopen(name, "rb")`.
    fn fopen_read(&mut self, name: &[u8]) -> Option<Vec<u8>> {
        let p = self.host_path(name);
        match std::fs::read(&p) {
            Ok(d) => Some(d),
            // fopen of a directory succeeds; reading it gives EOF at once
            Err(_) if p.is_dir() => Some(vec![]),
            Err(_) => None,
        }
    }

    /// `uexit(code)` (lib/uexit.c): exit with that status.
    pub fn uexit(&mut self, code: i32) {
        jump(Jump::Exit(code))
    }

    /// `standard_input := stdin; standard_output := stdout`.
    pub fn open_standard_files(&mut self) {
        let o = match self.host.host().stdout_stream() {
            Some((w, block)) => Stdio::new(w, block),
            // captured as a pipe's (st_blksize 16384 on macOS)
            None => Stdio::new(Box::new(Captured(self.host.captured.clone())), 16384),
        };
        self.standard_output.output = Some(o);
    }

    /// `parse_arguments` (bibtex.ch §467) over kpathsea's
    /// `getopt_long_only` (kpathsea/getopt.c), options `terse`,
    /// `min-crossrefs` (with an argument), `help` and `version`.
    pub fn parse_arguments(&mut self) {
        self.init_option_variables();
        let args = self.host.args.clone();
        let posixly_correct = std::env::var_os("POSIXLY_CORRECT").is_some();
        let names: [(&str, bool); 4] = [
            ("terse", false),
            ("min-crossrefs", true),
            ("help", false),
            ("version", false),
        ];
        let mut nonopts: Vec<Vec<u8>> = vec![];
        let mut i = 0;
        while i < args.len() {
            let a = &args[i];
            if a.as_slice() == b"--" {
                nonopts.extend(args[i + 1..].iter().cloned());
                break;
            }
            if a.first() != Some(&b'-') || a.len() == 1 {
                if posixly_correct {
                    nonopts.extend(args[i..].iter().cloned());
                    break;
                }
                nonopts.push(a.clone());
                i += 1;
                continue;
            }
            let dashes = if a.get(1) == Some(&b'-') { 2 } else { 1 };
            let body = &a[dashes..];
            let nameend = body.iter().position(|&c| c == b'=').unwrap_or(body.len());
            let name = &body[..nameend];
            let mut found: Option<usize> = None;
            let mut exact = false;
            let mut ambig = false;
            for (k, (n, _)) in names.iter().enumerate() {
                if n.as_bytes().starts_with(name) {
                    if name.len() == n.len() {
                        found = Some(k);
                        exact = true;
                        break;
                    } else if found.is_none() {
                        found = Some(k);
                    } else {
                        ambig = true;
                    }
                }
            }
            let arg0 = String::from_utf8_lossy(a).into_owned();
            if ambig && !exact {
                let _ = writeln!(self.host.stderr, "{MY_NAME}: option `{arg0}' is ambiguous");
                self.usage();
            }
            let Some(k) = found else {
                let rest = String::from_utf8_lossy(body);
                if dashes == 2 {
                    let _ = writeln!(
                        self.host.stderr,
                        "{MY_NAME}: unrecognized option `--{rest}'"
                    );
                } else {
                    let _ = writeln!(self.host.stderr, "{MY_NAME}: unrecognized option `-{rest}'");
                }
                self.usage();
            };
            let (opt, has_arg) = names[k];
            i += 1;
            let mut optarg: Option<Vec<u8>> = None;
            if nameend < body.len() {
                if has_arg {
                    optarg = Some(body[nameend + 1..].to_vec());
                } else {
                    if dashes == 2 {
                        let _ = writeln!(
                            self.host.stderr,
                            "{MY_NAME}: option `--{opt}' doesn't allow an argument"
                        );
                    } else {
                        let _ = writeln!(
                            self.host.stderr,
                            "{MY_NAME}: option `-{opt}' doesn't allow an argument"
                        );
                    }
                    self.usage();
                }
            } else if has_arg {
                if i < args.len() {
                    optarg = Some(args[i].clone());
                    i += 1;
                } else {
                    let _ = writeln!(
                        self.host.stderr,
                        "{MY_NAME}: option `{arg0}' requires an argument"
                    );
                    self.usage();
                }
            }
            match opt {
                "terse" => self.verbose = false,
                "min-crossrefs" => self.min_crossrefs = atoi(&optarg.unwrap_or_default()),
                "help" => self.usage_help(),
                _ => self.print_version_and_exit(),
            }
        }
        if nonopts.len() != 1 {
            let _ = writeln!(
                self.host.stderr,
                "{MY_NAME}: Need exactly one file argument."
            );
            self.usage();
        }
        self.host.aux_name = nonopts.remove(0);
    }

    /// lib/usage.c `usage`.
    fn usage(&mut self) -> ! {
        let _ = writeln!(
            self.host.stderr,
            "Try `{MY_NAME} --help' for more information."
        );
        jump(Jump::Exit(1))
    }

    /// lib/usage.c `usagehelp(BIBTEXHELP, NULL)` (help.h).
    fn usage_help(&mut self) -> ! {
        let text = "Usage: bibtex [OPTION]... AUXFILE[.aux]\n  Write bibliography for entries in AUXFILE to AUXFILE.bbl,\n  along with a log file AUXFILE.blg.\n-min-crossrefs=NUMBER  include item after NUMBER cross-refs; default 2\n-terse                 do not print progress reports\n-help                  display this help and exit\n-version               output version information and exit\n\nEmail bug reports to tex-k@tug.org (https://lists.tug.org/tex-k).\n";
        self.standard_output.put(text.as_bytes());
        jump(Jump::Exit(0))
    }

    /// lib/printversion.c `print_version_and_exit(banner, "Oren
    /// Patashnik", NULL, NULL)`, with TeX Live 2026's kpathsea version.
    fn print_version_and_exit(&mut self) -> ! {
        let text = "BibTeX 0.99e (TeX Live 2026)\nkpathsea version 6.4.2\nCopyright 2026 Oren Patashnik.\nThere is NO warranty.  Redistribution of this software is\ncovered by the terms of both the BibTeX copyright and\nthe Lesser GNU General Public License.\nFor more information about these matters, see the file\nnamed COPYING and the BibTeX source.\nPrimary author of BibTeX: Oren Patashnik.\n";
        self.standard_output.put(text.as_bytes());
        jump(Jump::Exit(0))
    }

    /// lib/setupvar.c `setupboundvariable`: the value of `name` in
    /// texmf.cnf (or the environment), else `dflt`.
    pub fn setup_bound_value(&mut self, name: &str, dflt: i32) -> i32 {
        let Some(exp) = self.host.host().var_value(name) else {
            return dflt;
        };
        let conf_val = atoi(exp.as_bytes());
        if conf_val < 0 || (conf_val == 0 && dflt > 0) {
            let _ = writeln!(
                self.host.stderr,
                "{MY_NAME}: Bad value ({conf_val}) in environment or texmf.cnf for {name}, keeping {dflt}."
            );
            dflt
        } else {
            conf_val
        }
    }

    /// cpascal.h's BIBXRETALLOC log line.
    pub fn log_realloc(&mut self, name: &str, elt_size: i32, new_size: i32, old_size: i32) {
        let s = format!(
            "Reallocated {name} (elt_size={elt_size}) to {new_size} items from {old_size}.\n"
        );
        self.log_file.put(s.as_bytes());
    }

    /// A C string literal's byte `i` (`ucharcast(pds[i])`).
    pub fn c_char(&mut self, s: &str, i: i32) -> i32 {
        s.as_bytes()[i as usize] as i32
    }

    /// `getc(f)`.
    pub fn getc(&mut self, f: &mut AlphaFile) -> i32 {
        f.getc()
    }

    /// `vgetc(f)`: a byte read and thrown away.
    pub fn vgetc(&mut self, f: &mut AlphaFile) {
        f.getc();
    }

    /// cpascal.h's `aopenin(f, path)`: openclose.c's `open_input(&f,
    /// path, "rb")` on `name_of_file+1`.
    pub fn a_open_in(&mut self, f: &mut AlphaFile, path: i32) -> bool {
        *f = AlphaFile::default();
        let name = c_name(self);
        if path < 0 {
            // no_file_path: fopen as given
            match self.fopen_read(&name) {
                Some(d) => {
                    f.input = Some(d);
                    true
                }
                None => false,
            }
        } else {
            let format = if path == crate::generated::consts::kpse_bib_format {
                Format::Bib
            } else {
                Format::Bst
            };
            let Some(mut found) = self.host.host().find_file(&name, format) else {
                return false;
            };
            // Drop a `./' kpathsea put in front, unless the name had it.
            if found.len() >= 2
                && found[0] == b'.'
                && found[1] == b'/'
                && !(name.first() == Some(&b'.') && name.get(1) == Some(&b'/'))
            {
                found.drain(..2);
            }
            // "This fopen is not allowed to fail" (xfopen).
            let Some(d) = self.fopen_read(&found) else {
                let _ = writeln!(
                    self.host.stderr,
                    "{MY_NAME}: fopen({}) failed",
                    String::from_utf8_lossy(&found)
                );
                jump(Jump::Exit(1))
            };
            f.input = Some(d);
            set_c_name(self, &found);
            true
        }
    }

    /// cpascal.h's `aopeninwithdirname`: openclose.c's
    /// `open_input_with_dirname`, the directory of the top-level `.aux`
    /// name `top` before `name_of_file+1`.
    pub fn a_open_in_with_dirname(&mut self, f: &mut AlphaFile, path: i32, top: i32) -> bool {
        let s = self.str_start[top as usize] as usize;
        let e = self.str_start[(top + 1) as usize] as usize;
        let fname: Vec<u8> = (s..e).map(|k| self.str_pool[k] as u8).collect();
        // kpathsea's xdirname
        let top_dir = xdirname(&fname);
        let name = c_name(self);
        if !top_dir.is_empty() && top_dir != b"." && !absolute_p(&name, true) {
            let mut newname = top_dir;
            newname.push(b'/');
            newname.extend_from_slice(&name);
            set_c_name(self, &newname);
            return self.a_open_in(f, path);
        }
        false
    }

    /// cpascal.h's `aopenout(f)`: openclose.c's `open_output(&f, "w")` on
    /// `name_of_file+1`, then `$TEXMFOUTPUT/name` for a relative name.
    pub fn a_open_out(&mut self, f: &mut AlphaFile) -> bool {
        *f = AlphaFile::default();
        let name = c_name(self);
        let absolute = absolute_p(&name, false);
        let mut fname = name.clone();
        let mut file = std::fs::File::create(self.host_path(&fname));
        if file.is_err() && !absolute {
            let out = self.host.host().var_value("TEXMFOUTPUT");
            if let Some(o) = out.filter(|o| !o.is_empty()) {
                fname = o.into_bytes();
                fname.push(b'/');
                fname.extend_from_slice(&name);
                file = std::fs::File::create(self.host_path(&fname));
            }
        }
        match file {
            Ok(file) => {
                if fname != name {
                    set_c_name(self, &fname);
                }
                let block = block_size(&file);
                f.output = Some(Stdio::new(Box::new(file), block));
                true
            }
            Err(_) => false,
        }
    }

    /// cpascal.h's `aclose`: openclose.c's `close_file`.
    pub fn a_close(&mut self, f: &mut AlphaFile) {
        if let Some(mut o) = f.output.take() {
            o.flush();
        }
        *f = AlphaFile::default();
    }

    /// `kpse_in_name_ok(name_of_file+1)`: TeX Live 2026's kpathsea allows
    /// every name for reading (tex-file.c, `kpathsea_name_ok`).
    pub fn name_in_ok(&mut self) -> bool {
        true
    }

    /// `kpse_out_name_ok(name_of_file+1)`, with kpathsea's message.
    pub fn name_out_ok(&mut self) -> bool {
        let name = c_name(self);
        match self.host.host().out_name_ok(&name) {
            Ok(()) => true,
            Err(choice) => {
                let mut m = format!("\n{MY_NAME}: Not writing to ").into_bytes();
                m.extend_from_slice(&name);
                m.extend_from_slice(b" (openout_any = ");
                m.extend_from_slice(&choice);
                m.extend_from_slice(b"; no extended check).\n");
                self.host.stderr.extend_from_slice(&m);
                false
            }
        }
    }

    /// bibtex.ch §100: `name_of_file := xmalloc_array(ASCII_code,
    /// strlen(cmdline(optind)) + 5)`, the argument copied to
    /// `name_of_file+1`, `aux_name_length` its length.
    pub fn set_aux_name_from_command_line(&mut self) {
        let a = self.host.aux_name.clone();
        self.name_of_file.alloc_len(a.len() + 5 + 1);
        for (i, &b) in a.iter().enumerate() {
            self.name_of_file[i + 1] = b;
        }
        self.name_of_file[a.len() + 1] = 0;
        // strlen: up to the first NUL (an argument has none)
        self.aux_name_length = a.len() as i32;
    }

    /// The main program from `hack0` to `close_up_shop:`, which a fatal
    /// error leaves with `longjmp(jmp9998,1)`.
    pub fn catch_close_up_shop(&mut self) {
        let r = catch_unwind(AssertUnwindSafe(|| self.main_part()));
        if let Err(e) = r {
            match e.downcast::<Jump>() {
                Ok(j) if matches!(*j, Jump::CloseUpShop) => {}
                Ok(j) => resume_unwind(j),
                Err(e) => resume_unwind(e),
            }
        }
    }

    /// cvtbib.sed's `if(setjmp(jmp32)==0)for(;;){...}`: the `.bst` loop,
    /// left by a `break` (a return from `bst_loop`) or `longjmp(jmp32,1)`.
    pub fn catch_bst_done(&mut self) {
        let r = catch_unwind(AssertUnwindSafe(|| self.bst_loop()));
        if let Err(e) = r {
            match e.downcast::<Jump>() {
                Ok(j) if matches!(*j, Jump::BstDone) => {}
                Ok(j) => resume_unwind(j),
                Err(e) => resume_unwind(e),
            }
        }
    }

    /// The host's time limit (not TeX Live's: a child process would be
    /// killed instead), read at the first function call and every 1024th.
    pub fn check_deadline(&mut self) {
        let Some(d) = self.host.deadline else { return };
        let n = self.host.calls;
        self.host.calls = n.wrapping_add(1);
        if n.is_multiple_of(1024) && std::time::Instant::now() >= d {
            jump(Jump::Deadline)
        }
    }

    /// `goto bst_done` from `bst_err_print_and_look_for_blank_line`.
    pub fn jump_to_bst_done(&mut self) {
        jump(Jump::BstDone)
    }
}

/// C's `atoi`: optional blanks, a sign, digits (wrapping like the 32-bit
/// result TeX Live's `integer` gets).
fn atoi(s: &[u8]) -> i32 {
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let mut v: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        v = v.wrapping_mul(10).wrapping_add((s[i] - b'0') as i64);
        i += 1;
    }
    (if neg { -v } else { v }) as i32
}

/// kpathsea's `xdirname` (Unix): the name up to its last `/`, without
/// trailing slashes; `.` when there is none.
fn xdirname(name: &[u8]) -> Vec<u8> {
    let Some(mut loc) = name.iter().rposition(|&c| c == b'/') else {
        return b".".to_vec();
    };
    if loc == 0 {
        return b"/".to_vec();
    }
    while loc > 1 && name[loc - 1] == b'/' {
        loc -= 1;
    }
    name[..loc].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_atoi() {
        assert_eq!(atoi(b"  42x"), 42);
        assert_eq!(atoi(b"-3"), -3);
        assert_eq!(atoi(b""), 0);
    }

    #[test]
    fn dirname() {
        assert_eq!(xdirname(b"a/b"), b"a");
        assert_eq!(xdirname(b"b"), b".");
        assert_eq!(xdirname(b"/b"), b"/");
        assert_eq!(xdirname(b"a//b"), b"a");
    }
}
