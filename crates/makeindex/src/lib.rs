//! makeindex 2.18 as TeX Live 2026 builds it (`texk/makeindexk`, with
//! kpathsea), ported to Rust so the engine can run it in-process.
//!
//! **Source of record.** TeX Live's source tree, GitHub mirror
//! <https://github.com/TeX-Live/texlive-source>, tag `texlive-2026.1`,
//! commit `6a300188053b8f2ded89dbd52293732a706b9c0e`, directory
//! `texk/makeindexk`: `mkind.c`/`mkind.h` (this file and `mkind.rs`),
//! `scanid.c`/`scanid.h` (`scanid.rs`), `scanst.c`/`scanst.h`
//! (`scanst.rs`), `sortid.c` (`sortid.rs`), `qsort.c` (`qsort.rs`) and
//! `genind.c`/`genind.h` (`genind.rs`). Each function keeps its C name and
//! its statements their order, so the two read side by side. The build
//! settings are TeX Live's: `USE_KPATHSEA`, `HAVE_SETLOCALE`, `UNIX`.
//!
//! **Licence.** A port is a modified version under the MakeIndex
//! Distribution Notice, so this crate carries that notice (`LICENSE`), not
//! MIT or GPL; only the GPL engine links it (DESIGN.md §3).
//!
//! **What the C program did through the system** goes through [`Host`]:
//! kpathsea's `kpse_find_file(name, kpse_ist_format, 1)` and its
//! `kpse_in_name_ok`/`kpse_out_name_ok`, the standard streams, and the
//! directory relative names are opened in (the child's working directory).
//!
//! **Behaviour the C code leaves undefined** is reproduced where it is
//! deterministic and visible: `char` is signed (as on every platform TeX
//! Live builds for except Linux on ARM), so a delimiter byte >= 0x80 never
//! equals a character read with `getc`; the static `key` buffer keeps
//! earlier entries' bytes, which a quote before an embedded NUL reads; and
//! `mk_getc`'s lookahead is shared by every stream. Two cases cannot be
//! reproduced and are documented in the evidence README: a style file that
//! ends inside a string (the C program prints an unterminated stack buffer)
//! and an empty `page_precedence` (it indexes an array with an
//! uninitialised value).

mod genind;
mod io;
mod locale;
mod mkind;
mod qsort;
mod scanid;
mod scanst;
mod sortid;

use std::io::Write;
use std::path::{Path, PathBuf};

/// What makeindex asks of its surroundings.
pub trait Host {
    /// `kpse_find_file(name, kpse_ist_format, 1)`: the style file's path,
    /// as kpathsea returns it (it is printed in the transcript).
    fn find_ist(&mut self, name: &[u8]) -> Option<Vec<u8>>;
    /// `kpse_in_name_ok(name)` (TeX Live 2026 allows any name for
    /// reading).
    fn in_name_ok(&mut self, _name: &[u8]) -> bool {
        true
    }
    /// `kpse_out_name_ok(name)`, reporting a refusal on standard error as
    /// kpathsea does, with the invocation name `makeindex`.
    fn out_name_ok(&mut self, _name: &[u8]) -> bool {
        true
    }
    fn stdout(&mut self) -> &mut dyn Write;
    fn stderr(&mut self) -> &mut dyn Write;
    /// Everything on standard input (`-i`, or no input file).
    fn read_stdin(&mut self) -> Vec<u8>;
    /// The directory relative file names are opened in; `None` is the
    /// process's working directory.
    fn cwd(&self) -> Option<&Path> {
        None
    }
}

/// `kpathsea`'s message for a refused output name
/// (`kpathsea_name_ok`, tex-file.c), for hosts that check silently.
pub fn not_writing_message(name: &[u8], openout_any: &[u8]) -> Vec<u8> {
    let mut m = b"\nmakeindex: Not writing to ".to_vec();
    m.extend_from_slice(name);
    m.extend_from_slice(b" (openout_any = ");
    m.extend_from_slice(openout_any);
    m.extend_from_slice(b"; no extended check).\n");
    m
}

/// The result of [`run`] when the C program would have died of SIGSEGV
/// (genind.c dereferences a NULL `prev` when the first sorted entry is a
/// duplicate); its output files then hold only the stdio blocks written
/// before.
pub const KILLED_BY_SIGSEGV: i32 = -11;

/// The result of [`run`] when the C program would never finish: with
/// `-p odd`/`even`/`any`, `find_pageno` scans the `.log` backwards and,
/// on some logs (one ending in CR LF), returns to the same state forever.
/// The port proves the repetition and stops instead; nothing has been
/// written to the (created) output files by then, as in the C program.
pub const LOOPS_FOREVER: i32 = -1000;

/// Run makeindex with the arguments after the program name; the result is
/// its exit status (0, or 1 after a fatal error), minus the signal that
/// would have killed it ([`KILLED_BY_SIGSEGV`]), or [`LOOPS_FOREVER`].
///
/// A panic inside the port (a bug: it reads past an array where the C
/// program's behaviour is undefined) is caught here and reported as
/// [`KILLED_BY_SIGSEGV`], so that a host running it in-process (the engine
/// host's tools thread) carries on.
pub fn run(args: &[Vec<u8>], host: &mut dyn Host) -> i32 {
    let mut mk = mkind::Mk::new(host);
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| mk.main(args)));
    let status = match r {
        Ok(Ok(())) => 0,
        Ok(Err(Exit(code))) => code,
        Err(_) => KILLED_BY_SIGSEGV,
    };
    mk.finish(status == KILLED_BY_SIGSEGV || status == LOOPS_FOREVER);
    status
}

/// The largest input file makeindex reads: 64 MiB, far more than any index
/// (Infinite Descent's four `.idx` files are 60 KB), and few enough bytes
/// that a host keeps running.
pub const MAX_INPUT: u64 = 64 << 20;

/// An input file's bytes, if `path` opens (`None`: it does not, as
/// `fopen` fails), or why it is refused: a file that is not a regular file
/// after following links (a FIFO, a device such as `/dev/zero`, a socket)
/// or one larger than [`MAX_INPUT`]. A directory opens and reads as empty,
/// as `fopen` and `getc` do with one.
pub fn read_input(path: &Path) -> Option<Result<Vec<u8>, String>> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.is_dir() {
        std::fs::File::open(path).ok()?;
        return Some(Ok(vec![]));
    }
    if !meta.is_file() {
        return Some(Err("not a regular file".into()));
    }
    if meta.len() > MAX_INPUT {
        return Some(Err(format!(
            "{} bytes, more than FlashTeX's limit of {MAX_INPUT}",
            meta.len()
        )));
    }
    let f = std::fs::File::open(path).ok()?;
    Some(read_capped(f))
}

/// At most [`MAX_INPUT`] bytes of `r` (standard input, a file that grew).
pub fn read_capped(r: impl std::io::Read) -> Result<Vec<u8>, String> {
    let mut d = vec![];
    let _ = std::io::Read::read_to_end(&mut r.take(MAX_INPUT + 1), &mut d);
    if d.len() as u64 > MAX_INPUT {
        return Err(format!("more than FlashTeX's limit of {MAX_INPUT} bytes"));
    }
    Ok(d)
}

/// `exit(code)`, from anywhere.
#[derive(Debug)]
pub(crate) struct Exit(pub i32);

pub(crate) type R<T> = Result<T, Exit>;

/// A file name as the C program has it, joined to the host's directory.
pub(crate) fn host_path(cwd: Option<&Path>, name: &[u8]) -> PathBuf {
    let p = bytes_path(name);
    match cwd {
        Some(d) if !p.is_absolute() => d.join(p),
        _ => p,
    }
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

/// A host on the process's own streams and working directory, with a
/// caller-supplied style-file lookup.
pub struct ProcessHost<F: FnMut(&[u8]) -> Option<Vec<u8>>> {
    pub find_ist: F,
    pub stdout: std::io::Stdout,
    pub stderr: std::io::Stderr,
}

impl<F: FnMut(&[u8]) -> Option<Vec<u8>>> ProcessHost<F> {
    pub fn new(find_ist: F) -> Self {
        ProcessHost {
            find_ist,
            stdout: std::io::stdout(),
            stderr: std::io::stderr(),
        }
    }
}

impl<F: FnMut(&[u8]) -> Option<Vec<u8>>> Host for ProcessHost<F> {
    fn find_ist(&mut self, name: &[u8]) -> Option<Vec<u8>> {
        (self.find_ist)(name)
    }
    fn stdout(&mut self) -> &mut dyn Write {
        &mut self.stdout
    }
    fn stderr(&mut self) -> &mut dyn Write {
        &mut self.stderr
    }
    fn read_stdin(&mut self) -> Vec<u8> {
        read_capped(std::io::stdin()).unwrap_or_default()
    }
}
