//! BibTeX in-process (lane RUST-TOOLS): TeX Live 2026's bibtex.web and
//! bibtex.ch translated to Rust (`crates/bibtex`), run instead of the user's
//! TeX Live binary where the engine would start one:
//!
//! * the host's external tools (`host::external`, DESIGN.md §4.5, §5.5):
//!   latexmk's bibtex rule;
//! * restricted `\write18` (`system::runsystem`): a command texmfmp.c's
//!   `shell_cmd_is_allowed` passed whose program is `bibtex`.
//!
//! The port finds `.bib` and `.bst` files and reads its texmf.cnf settings
//! (`max_strings.bibtex`, `openout_any`, ...) through the engine's own
//! kpathsea, which in bundle mode is the bundle's (DESIGN.md §4.4), so
//! neither needs a TeX Live binary. `FLASHTEX_BIBTEX=external` restores the
//! child process (A/B measurement, and the oracle comparison).

use crate::resolver::Format;
use std::path::{Path, PathBuf};

/// Whether bibtex runs in-process (the default) or as TeX Live's program
/// (`FLASHTEX_BIBTEX=external`).
pub fn in_process() -> bool {
    std::env::var("FLASHTEX_BIBTEX").map_or(true, |v| v != "external")
}

/// The port's view of the engine: kpathsea, and the directory the child
/// process would have run in.
#[derive(Clone, Debug, Default)]
pub struct EngineHost {
    /// The child's working directory: relative names are opened there and
    /// kpathsea's `.` is there. `None`: the process's.
    pub cwd: Option<PathBuf>,
    /// Directories searched before kpathsea's path for `.bib` and `.bst`
    /// files: latexmk's `BIBINPUTS`/`BSTINPUTS` fudge (the project and
    /// output directories, `host::external`).
    pub first: Vec<PathBuf>,
    /// Standard output is the process's own (a command, `\write18`), written
    /// as C's stdio would; otherwise it is captured.
    pub stream_stdout: bool,
}

impl EngineHost {
    /// kpathsea's candidates for `name` in one directory: the name with the
    /// format's suffix first (unless it has it), then as given.
    fn in_dir(d: &Path, name: &str, suffix: &str) -> Option<PathBuf> {
        let mut tries = vec![];
        if !name.ends_with(suffix) {
            tries.push(format!("{name}{suffix}"));
        }
        tries.push(name.to_string());
        tries.into_iter().map(|n| d.join(n)).find(|p| p.is_file())
    }
}

impl flashtex_bibtex::Host for EngineHost {
    fn find_file(&mut self, name: &[u8], format: flashtex_bibtex::Format) -> Option<Vec<u8>> {
        let name = String::from_utf8_lossy(name).into_owned();
        let (fmt, suffix) = match format {
            flashtex_bibtex::Format::Bib => (Format::Bib, ".bib"),
            flashtex_bibtex::Format::Bst => (Format::Bst, ".bst"),
        };
        // Confined reads: the name the document gave obeys the name rule (no
        // absolute name, `~`, `$` or `..`), as `\input`'s does.
        if !crate::system::input_name_confined_ok(&name) {
            return None;
        }
        if Path::new(&name).is_absolute() {
            return Self::in_dir(Path::new(""), &name, suffix).map(|p| path_bytes(&p));
        }
        let mut dirs: Vec<PathBuf> = self.first.clone();
        if let Some(d) = &self.cwd {
            // kpathsea's `.`, which is the child's directory
            dirs.push(d.clone());
        }
        for d in &dirs {
            if let Some(p) = Self::in_dir(d, &name, suffix) {
                return Some(path_bytes(&p));
            }
        }
        let found = crate::system::with_resolver(|r| r.find_ex(&name, fmt, true).0)?;
        if self.cwd.is_some() && !found.is_absolute() {
            // Found in the host process's own directory, which is not the
            // child's `.` (that one was searched above).
            return None;
        }
        // A regular file only, as the host's rule finds them (a FIFO or a
        // device is not a database; the port refuses it anyway).
        if !found.is_file() {
            return None;
        }
        Some(path_bytes(&found))
    }

    fn var_value(&mut self, name: &str) -> Option<String> {
        crate::system::with_resolver(|r| r.config_var_for(name, "bibtex"))
    }

    fn out_name_ok(&mut self, name: &[u8]) -> Result<(), Vec<u8>> {
        let n = String::from_utf8_lossy(name).into_owned();
        crate::system::with_resolver(|r| {
            if r.name_ok_silent(&n, true) {
                Ok(())
            } else {
                Err(r
                    .config_var_for("openout_any", "bibtex")
                    .unwrap_or_else(|| "p".into())
                    .into_bytes())
            }
        })
    }

    fn cwd(&self) -> Option<&Path> {
        self.cwd.as_deref()
    }

    fn read_ok(&mut self, name: &[u8], path: &Path, searched: bool) -> bool {
        // Confined reads (FLASHTEX_CONFINE_READS, Live Share): the engine's
        // own confinement (system::tool_read_ok); nothing otherwise.
        let name = String::from_utf8_lossy(name).into_owned();
        crate::system::tool_read_ok(&name, path, searched, self.cwd.as_deref())
    }

    fn stdout_stream(&mut self) -> Option<(Box<dyn std::io::Write>, usize)> {
        if !self.stream_stdout {
            return None;
        }
        Some(process_stdout())
    }
}

/// The process's standard output, unbuffered, and the buffer C's stdio would
/// give it: `st_blksize` for a file or pipe, 0 (line buffered) for a
/// terminal.
fn process_stdout() -> (Box<dyn std::io::Write>, usize) {
    #[cfg(unix)]
    {
        use std::os::fd::AsFd;
        use std::os::unix::fs::{FileTypeExt, MetadataExt};
        if let Ok(fd) = std::io::stdout().as_fd().try_clone_to_owned() {
            let f = std::fs::File::from(fd);
            let block = match f.metadata() {
                Ok(m) if m.file_type().is_char_device() => 0,
                Ok(m) if m.blksize() > 0 => m.blksize() as usize,
                _ => 1024,
            };
            return (Box::new(f), block);
        }
    }
    (Box::new(std::io::stdout()), 0)
}

#[cfg(unix)]
fn path_bytes(p: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    p.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn path_bytes(p: &Path) -> Vec<u8> {
    p.to_string_lossy().as_bytes().to_vec()
}

/// The arguments of `cmd` after the program name, if `cmd` runs bibtex and
/// nothing else (the rules of `crate::makeindex::command_args`).
pub fn command_args(cmd: &[u8], restricted: bool) -> Option<Vec<Vec<u8>>> {
    crate::os::tool_command_args(cmd, restricted, b"bibtex")
}

/// Run bibtex on the process's streams and working directory, as the child
/// of `\write18` would run: its exit status, or -1 where the C program's
/// behaviour is undefined or it ran out of time (what the shell route
/// reports for a child killed by a signal).
pub fn run_in_process(args: &[Vec<u8>]) -> i32 {
    let host = EngineHost {
        stream_stdout: true,
        ..EngineHost::default()
    };
    let o = flashtex_bibtex::run(args, Box::new(host));
    use std::io::Write;
    let _ = std::io::stderr().write_all(&o.stderr);
    if o.status < 0 {
        -1
    } else {
        o.status
    }
}
