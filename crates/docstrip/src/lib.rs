//! docstrip for FlashTeX: turns a package that ships only `.ins` and
//! `.dtx` sources into the `.sty`/`.cls`/`.def`/`.cfg` files TeX
//! distributions install (docs/proposals/packages-fonts-manifest.md §S3:
//! "`.ins`/`.dtx` packages are unpacked with the engine's own docstrip
//! subset or skipped with a diagnostic").
//!
//! The reference is `docstrip.dtx` v2.6c (2024-12-23) by Frank
//! Mittelbach, Denys Duchier, Johannes Braams, Marcin Woliński and Mark
//! Wooding, in TeX Live under `source/latex/base/`; the comments cite its
//! line numbers. A batch file is plain TeX, but the part docstrip gives
//! meaning to is small — `\generate{\file{…}{\from{…}{…}}}`, the pre-
//! and postamble declarations, a few switches — and the stripping rules
//! for the sources are a line grammar with a regular guard language. So
//! this is a purpose-built interpreter, not a TeX: no expansion engine of
//! the compiler is linked, and nothing fetched is ever *executed* in the
//! sense that matters (the crate only ever writes bytes it copied from
//! the sources or the batch file's own preamble text). What it does not
//! understand it reports, naming the command and the line, and carries
//! on; a batch file that programs its own TeX (as `lipsum.ins` does to
//! build its `.ltd.tex` files) still yields the files `\generate` names.
//!
//! - [`lexer`]: TeX's tokenizer as far as a batch file needs it;
//! - [`batch`]: the batch-file commands and a minimal macro layer
//!   (`\def`/`\let` with parameters), so `.ins` files that define
//!   helpers like microtype's `\makefile` work;
//! - [`guard`]: the guard expressions `%<…>`;
//! - [`strip`]: the line processor for `.dtx` sources.
//!
//! [`run`] is the entry point: a batch file's bytes plus a [`Sources`]
//! that answers `\from{name}` and `\batchinput{name}`, and the result is
//! every generated file's bytes with the diagnostics and terminal
//! messages the run produced. Output is byte-identical to `tex x.ins`
//! for the constructs implemented (the TeX Live comparison tests in
//! `tests/texlive.rs` are the oracle).

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

pub mod batch;
pub mod guard;
pub mod lexer;
pub mod strip;

/// The docstrip version whose behaviour this crate reproduces
/// (`\fileversion` in docstrip.dtx line 32); written by
/// `\AddGenerationDate` headers.
pub const DOCSTRIP_VERSION: &str = "v2.6c";

/// Where `\from{name}` and `\batchinput{name}` read from: the package's
/// source directory, on disk or in memory.
pub trait Sources {
    /// The bytes of `name`, or `None` when there is no such file.
    fn read(&self, name: &str) -> Option<Vec<u8>>;
}

impl Sources for BTreeMap<String, Vec<u8>> {
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.get(name).cloned()
    }
}

impl<T: Sources + ?Sized> Sources for &T {
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        (**self).read(name)
    }
}

/// A directory of sources. Relative paths below it are served (caption's
/// `\from{fallback/v1/caption.dtx}`); an absolute path, a `..` or a dot
/// component reads nothing, so a batch file cannot reach outside.
pub struct DirSources(pub PathBuf);

impl Sources for DirSources {
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        if name.is_empty() || name.starts_with('/') || name.contains(['\\', '\0']) || name.split('/').any(|c| c.is_empty() || c.starts_with('.')) {
            return None;
        }
        std::fs::read(self.0.join(name)).ok()
    }
}

/// Run-time inputs docstrip would take from TeX.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Today's date (year, month, day) for `\AddGenerationDate` headers,
    /// which write `generated on <year/month/day>` (docstrip.dtx line
    /// 3497). `None` writes `<0/0/0>` with a diagnostic; a batch file
    /// that does not call `\AddGenerationDate` (nearly all) never needs it.
    pub today: Option<(i32, u32, u32)>,
}

/// One file `\generate` wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    /// The name given to `\file{…}`, as written.
    pub name: String,
    pub bytes: Vec<u8>,
    /// The `\from` sources, in reading order, without repeats.
    pub sources: Vec<String>,
    /// The `\usedir{…}` label in force at the `\file`, if any (docstrip
    /// without a `docstrip.cfg` ignores it and writes next to the batch
    /// file; it is recorded for the caller's information).
    pub dir: Option<String>,
}

/// Something the run could not do, or that docstrip itself would have
/// reported: `file:line: message`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The batch file or source the line is in.
    pub file: String,
    /// 1-based; 0 when no line applies.
    pub line: usize,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}: {}", self.file, self.message)
        } else {
            write!(f, "{}:{}: {}", self.file, self.line, self.message)
        }
    }
}

/// What a run produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    /// In generation order; a name generated twice keeps the last.
    pub files: Vec<GeneratedFile>,
    pub diagnostics: Vec<Diagnostic>,
    /// `\Msg` output and docstrip's own progress lines, in order.
    pub messages: Vec<String>,
    /// Whether the batch file ran to `\endbatchfile` or its end (false
    /// when it stopped on something fatal, such as a missing nested batch
    /// file).
    pub completed: bool,
}

impl Outcome {
    pub fn file(&self, name: &str) -> Option<&GeneratedFile> {
        self.files.iter().find(|f| f.name == name)
    }
}

/// Runs the batch file `batch` (named `batch_name`, which is what
/// diagnostics cite and what `\jobname` expands to) against `sources`.
pub fn run(batch_name: &str, batch: &[u8], sources: &dyn Sources) -> Outcome {
    run_with(batch_name, batch, sources, &Options::default())
}

/// [`run`] with explicit [`Options`].
pub fn run_with(batch_name: &str, batch: &[u8], sources: &dyn Sources, options: &Options) -> Outcome {
    batch::Interpreter::new(sources, options).run(batch_name, batch)
}

/// Whether `name` is a docstrip batch file by extension.
pub fn is_batch_file(name: &str) -> bool {
    Path::new(name).extension().is_some_and(|e| e.eq_ignore_ascii_case("ins"))
}

/// Whether `name` is a docstrip source by extension.
pub fn is_source_file(name: &str) -> bool {
    Path::new(name).extension().is_some_and(|e| e.eq_ignore_ascii_case("dtx"))
}
