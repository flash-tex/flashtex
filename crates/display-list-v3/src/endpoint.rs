//! Where a standalone engine writes its display-list frames: the value of
//! `FLASHTEX_DISPLAY_LIST` (spec §6.6), parsed in this one place for the
//! engine and for anything that launches it.
//!
//! | value | meaning |
//! |---|---|
//! | `fd:N` | an inherited, already-open descriptor `N` (Unix; how the host runs an export) |
//! | `socket:PATH` | connect to a listening Unix-domain stream socket at `PATH` |
//! | `pipe:NAME` | connect to the Windows named pipe `\\.\pipe\NAME` (Windows only) |
//! | anything else | a file path, created (or truncated) |
//!
//! The frames need nothing from the transport but a reliable, ordered byte
//! stream (spec §6.1), so the named forms exist for systems without
//! `socketpair` and numbered-descriptor inheritance (Windows): the launcher
//! listens on a name and passes the name, not a descriptor.

use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::PathBuf;

/// The environment variable this module parses.
pub const ENV: &str = "FLASHTEX_DISPLAY_LIST";

/// One parsed `FLASHTEX_DISPLAY_LIST` value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Endpoint {
    /// `fd:N`: an inherited descriptor.
    Fd(i32),
    /// `socket:PATH`: a Unix-domain stream socket to connect to.
    Socket(PathBuf),
    /// `pipe:NAME`: a Windows named pipe (`\\.\pipe\NAME`).
    Pipe(String),
    /// A file to create.
    File(PathBuf),
}

impl Endpoint {
    /// Parse a `FLASHTEX_DISPLAY_LIST` value. A file path that itself begins
    /// with `fd:`, `socket:` or `pipe:` must be written `./fd:…` etc.
    pub fn parse(spec: &OsStr) -> Result<Endpoint, String> {
        if spec.is_empty() {
            return Err(format!("{ENV}: empty value"));
        }
        // The prefixes are ASCII; the rest of a path stays as the OS gave it.
        let bytes = spec.as_encoded_bytes();
        let rest = |n: usize| -> &OsStr {
            // SAFETY: splitting after an ASCII prefix keeps both halves valid
            // encoded OS-string bytes.
            unsafe { OsStr::from_encoded_bytes_unchecked(&bytes[n..]) }
        };
        if let Some(n) = bytes.strip_prefix(b"fd:") {
            let s = std::str::from_utf8(n).map_err(|_| format!("{ENV}: bad descriptor"))?;
            return match s.parse::<i32>() {
                Ok(fd) if fd >= 0 => Ok(Endpoint::Fd(fd)),
                _ => Err(format!("{ENV}: bad descriptor `{s}'")),
            };
        }
        if bytes.starts_with(b"socket:") {
            let p = rest(7);
            if p.is_empty() {
                return Err(format!("{ENV}: socket: needs a path"));
            }
            return Ok(Endpoint::Socket(PathBuf::from(p)));
        }
        if bytes.starts_with(b"pipe:") {
            let n = rest(5)
                .to_str()
                .ok_or_else(|| format!("{ENV}: pipe name is not UTF-8"))?;
            if n.is_empty() || n.contains(['\\', '/']) {
                return Err(format!("{ENV}: bad pipe name `{n}'"));
            }
            return Ok(Endpoint::Pipe(n.to_string()));
        }
        Ok(Endpoint::File(PathBuf::from(spec)))
    }

    /// Read and parse `FLASHTEX_DISPLAY_LIST`: `None` when it is unset.
    pub fn from_env() -> Option<Result<Endpoint, String>> {
        std::env::var_os(ENV).map(|v| Endpoint::parse(&v))
    }

    /// Open the endpoint for writing frames (buffered).
    pub fn open_writer(&self) -> io::Result<Box<dyn Write + Send>> {
        const CAP: usize = 1 << 16;
        let buf = |w: Box<dyn Write + Send>| -> Box<dyn Write + Send> {
            Box::new(io::BufWriter::with_capacity(CAP, w))
        };
        match self {
            Endpoint::File(p) => Ok(buf(Box::new(std::fs::File::create(p)?))),
            Endpoint::Fd(fd) => open_fd(*fd).map(buf),
            Endpoint::Socket(p) => open_socket(p).map(buf),
            Endpoint::Pipe(n) => open_pipe(n).map(buf),
        }
    }
}

impl std::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Endpoint::Fd(n) => write!(f, "fd:{n}"),
            Endpoint::Socket(p) => write!(f, "socket:{}", p.display()),
            Endpoint::Pipe(n) => write!(f, "pipe:{n}"),
            Endpoint::File(p) => write!(f, "{}", p.display()),
        }
    }
}

#[cfg(unix)]
fn open_fd(fd: i32) -> io::Result<Box<dyn Write + Send>> {
    use std::os::fd::FromRawFd;
    // SAFETY: the descriptor was inherited for exactly this, and nothing
    // else in the process owns it.
    Ok(Box::new(unsafe { std::fs::File::from_raw_fd(fd) }))
}

#[cfg(not(unix))]
fn open_fd(_fd: i32) -> io::Result<Box<dyn Write + Send>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "fd:N (an inherited descriptor) is unsupported on this OS; use pipe:NAME",
    ))
}

#[cfg(unix)]
fn open_socket(path: &std::path::Path) -> io::Result<Box<dyn Write + Send>> {
    let s = std::os::unix::net::UnixStream::connect(path)?;
    crate::widen_socket_buffers(&s);
    Ok(Box::new(s))
}

#[cfg(not(unix))]
fn open_socket(_path: &std::path::Path) -> io::Result<Box<dyn Write + Send>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "socket:PATH is not implemented on this OS yet; use pipe:NAME",
    ))
}

#[cfg(windows)]
fn open_pipe(name: &str) -> io::Result<Box<dyn Write + Send>> {
    // A named pipe's client end opens like a file.
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(format!(r"\\.\pipe\{name}"))?;
    Ok(Box::new(f))
}

#[cfg(not(windows))]
fn open_pipe(_name: &str) -> io::Result<Box<dyn Write + Send>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "pipe:NAME (a Windows named pipe) is unsupported on this OS; use socket:PATH or fd:N",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Result<Endpoint, String> {
        Endpoint::parse(OsStr::new(s))
    }

    #[test]
    fn parses_every_form() {
        assert_eq!(p("fd:3"), Ok(Endpoint::Fd(3)));
        assert_eq!(
            p("socket:/tmp/a b.sock"),
            Ok(Endpoint::Socket("/tmp/a b.sock".into()))
        );
        assert_eq!(
            p("pipe:flashtex-1"),
            Ok(Endpoint::Pipe("flashtex-1".into()))
        );
        assert_eq!(p("out.dl3"), Ok(Endpoint::File("out.dl3".into())));
        assert_eq!(p("./fd:3"), Ok(Endpoint::File("./fd:3".into())));
        assert_eq!(
            p("/abs/socket:x"),
            Ok(Endpoint::File("/abs/socket:x".into()))
        );
    }

    #[test]
    fn rejects_malformed_values() {
        for bad in [
            "",
            "fd:",
            "fd:x",
            "fd:-1",
            "socket:",
            "pipe:",
            "pipe:a\\b",
            "pipe:a/b",
        ] {
            assert!(p(bad).is_err(), "{bad:?} should be refused");
        }
    }

    #[test]
    fn display_round_trips() {
        for s in ["fd:3", "socket:/tmp/x.sock", "pipe:n", "a/b.dl3"] {
            assert_eq!(p(s).unwrap().to_string(), s);
        }
    }

    #[cfg(unix)]
    #[test]
    fn pipe_is_a_clear_error_on_unix() {
        let e = p("pipe:x").unwrap().open_writer().err().unwrap();
        assert_eq!(e.kind(), io::ErrorKind::Unsupported);
        assert!(e.to_string().contains("unsupported on this OS"), "{e}");
    }
}
