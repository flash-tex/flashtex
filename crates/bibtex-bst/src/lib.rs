//! FlashTeX BibTeX engine: an original Rust port of BibTeX 0.99e
//! (`bibtex.web` with the TeX Live change file `bibtex.ch`).
//!
//! It reads an `.aux` file, the `.bst` style and the `.bib` databases through
//! a [`FileSource`] and produces the `.bbl` and `.blg` bytes that TeX Live's
//! `bibtex` would write. Nothing here runs an external program.
//!
//! ```no_run
//! use flashtex_bibtex_bst::{run, DirSource, Options};
//! let src = DirSource::new("build").with_bst_dir("styles").with_bib_dir("refs");
//! let out = run("paper", &src, &Options::default()).unwrap();
//! std::fs::write("build/paper.bbl", &out.bbl).unwrap();
//! ```

// Clippy debt in frozen old-engine code (D13): allowed, not refactored.
#![allow(clippy::collapsible_if, clippy::manual_range_patterns, clippy::needless_range_loop, clippy::same_item_push, reason = "frozen old engine (D13): fixes only; clippy debt is allowed rather than refactored")]

mod bib;
mod bst;
mod chars;
mod engine;
mod exec;
mod names;

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Which kind of file BibTeX is asking for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileKind {
    /// The top-level `.aux` (name includes `.aux`) or an `\@input` file.
    Aux,
    /// A style, named as in `\bibstyle` (no `.bst` added).
    Bst,
    /// A database, named as in `\bibdata` (no `.bib` added).
    Bib,
}

/// Supplies file contents. Implementations decide the search path
/// (TeX Live uses kpathsea `BSTINPUTS`/`BIBINPUTS`).
pub trait FileSource {
    /// Returns the bytes of the file, or `None` if it cannot be opened.
    /// For [`FileKind::Bst`] and [`FileKind::Bib`] the name is exactly the
    /// `\bibstyle`/`\bibdata` argument; the implementation adds the
    /// extension the way kpathsea does.
    fn read(&self, kind: FileKind, name: &str) -> Option<Vec<u8>>;
}

/// A [`FileSource`] backed by directories, searched in order.
#[derive(Clone, Debug, Default)]
pub struct DirSource {
    pub aux_dirs: Vec<PathBuf>,
    pub bst_dirs: Vec<PathBuf>,
    pub bib_dirs: Vec<PathBuf>,
}

impl DirSource {
    /// All three searches start in `dir` (like running `bibtex` there).
    pub fn new(dir: impl AsRef<Path>) -> Self {
        let d = dir.as_ref().to_path_buf();
        DirSource { aux_dirs: vec![d.clone()], bst_dirs: vec![d.clone()], bib_dirs: vec![d] }
    }
    pub fn with_bst_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.bst_dirs.push(dir.as_ref().to_path_buf());
        self
    }
    pub fn with_bib_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.bib_dirs.push(dir.as_ref().to_path_buf());
        self
    }
}

impl FileSource for DirSource {
    fn read(&self, kind: FileKind, name: &str) -> Option<Vec<u8>> {
        let (dirs, ext) = match kind {
            FileKind::Aux => (&self.aux_dirs, ""),
            FileKind::Bst => (&self.bst_dirs, ".bst"),
            FileKind::Bib => (&self.bib_dirs, ".bib"),
        };
        let mut candidates = vec![format!("{name}{ext}")];
        if !ext.is_empty() && name.ends_with(ext) {
            candidates.insert(0, name.to_string());
        }
        for dir in dirs {
            for c in &candidates {
                let p = if Path::new(c).is_absolute() { PathBuf::from(c) } else { dir.join(c) };
                if let Ok(b) = std::fs::read(&p) {
                    return Some(b);
                }
            }
        }
        None
    }
}

/// Runtime parameters (TeX Live's `texmf.cnf` values by default).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Options {
    /// `--min-crossrefs` (default 2).
    pub min_crossrefs: i32,
    /// `max_print_line` (default 79).
    pub max_print_line: usize,
    /// `ent_str_size` (default 500).
    pub ent_str_size: usize,
    /// `glob_str_size` (default 200000).
    pub glob_str_size: usize,
    /// Appended to the `.blg` banner (default `" (TeX Live 2026)"`).
    pub version_suffix: String,
    /// The `Capacity:` line of the `.blg`.
    pub capacity_line: String,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            min_crossrefs: 2,
            max_print_line: 79,
            ent_str_size: 500,
            glob_str_size: 200000,
            version_suffix: " (TeX Live 2026)".into(),
            capacity_line: "Capacity: max_strings=200000, hash_size=200000, hash_prime=170003"
                .into(),
        }
    }
}

/// BibTeX's `history` (§18).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum History {
    Spotless,
    Warnings,
    Errors,
    Fatal,
}

/// The result of a run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    /// The `.bbl` bytes.
    pub bbl: Vec<u8>,
    /// The `.blg` bytes.
    pub blg: Vec<u8>,
    pub history: History,
    /// Number of warnings or error messages counted for `history`.
    pub err_count: i32,
}

impl Output {
    /// The diagnostic part of the log: every `.blg` line except the banner,
    /// the `Capacity:` line, the progress lines (`The top-level auxiliary
    /// file:`, `The style file:`, `Database file #`, `A level-`), the usage
    /// statistics and the final history line.
    pub fn diagnostics(&self) -> Vec<String> {
        let text = String::from_utf8_lossy(&self.blg);
        let mut out = Vec::new();
        let mut in_stats = false;
        for (i, line) in text.lines().enumerate() {
            if i < 2 {
                continue;
            }
            if line.starts_with("You've used ") {
                in_stats = true;
                continue;
            }
            if in_stats {
                if line.starts_with("(There w") || line.starts_with("(That was") {
                    in_stats = false;
                }
                continue;
            }
            if line.starts_with("The top-level auxiliary file: ")
                || line.starts_with("The style file: ")
                || line.starts_with("Database file #")
                || line.starts_with("A level-")
            {
                continue;
            }
            out.push(line.to_string());
        }
        out
    }

    /// Lines of [`Output::diagnostics`] that start a warning (`Warning--`).
    pub fn warnings(&self) -> Vec<String> {
        self.diagnostics().into_iter().filter(|l| l.starts_with("Warning--")).collect()
    }
}

/// Why a run could not start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunError {
    /// The top-level `.aux` file could not be read (BibTeX prints
    /// ``I couldn't open file name `x.aux'`` and exits with status 1).
    AuxNotFound(String),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::AuxNotFound(n) => write!(f, "I couldn't open file name `{n}'"),
        }
    }
}

impl std::error::Error for RunError {}

/// Runs BibTeX on `aux_name` (with or without the `.aux` extension).
pub fn run(aux_name: &str, fs: &dyn FileSource, opts: &Options) -> Result<Output, RunError> {
    let mut e = engine::Engine::new(fs, opts.clone());
    e.pre_define();
    if !e.open_top_level_aux(aux_name) {
        let n = if aux_name.ends_with(".aux") { aux_name.to_string() } else { format!("{aux_name}.aux") };
        return Err(RunError::AuxNotFound(n));
    }
    e.log.extend_from_slice(b"This is BibTeX, Version 0.99e");
    e.log.extend_from_slice(opts.version_suffix.as_bytes());
    e.log.push(b'\n');
    e.log.extend_from_slice(opts.capacity_line.as_bytes());
    e.log.push(b'\n');
    let r = e.read_aux().and_then(|_| e.read_bst());
    let _ = r; // `Flow::Fatal` jumps to `close_up_shop`, as does normal completion.
    e.clean_up();
    let history = match e.history {
        engine::SPOTLESS => History::Spotless,
        engine::WARNING_MESSAGE => History::Warnings,
        engine::ERROR_MESSAGE => History::Errors,
        _ => History::Fatal,
    };
    Ok(Output { bbl: e.bbl, blg: e.log, history, err_count: e.err_count })
}

/// A [`FileSource`] wrapper that records every lookup and its result.
struct Recorder<'a> {
    inner: &'a dyn FileSource,
    log: RefCell<Vec<(FileKind, String, Option<u64>)>>,
}

fn hash_bytes(b: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    b.hash(&mut h);
    h.finish()
}

impl FileSource for Recorder<'_> {
    fn read(&self, kind: FileKind, name: &str) -> Option<Vec<u8>> {
        let r = self.inner.read(kind, name);
        self.log.borrow_mut().push((kind, name.to_string(), r.as_deref().map(hash_bytes)));
        r
    }
}

/// An IDE session that reuses the previous result when nothing BibTeX
/// consulted has changed.
///
/// A run's dependencies are the exact list of files it looked up (the
/// `.aux` chain, the style and every database, including failed lookups)
/// with content hashes. [`Session::run`] re-hashes those files; if all match
/// (and the options and `.aux` name are the same) it returns the cached
/// [`Output`] without executing the style. When only citations change the
/// `.aux` hash changes and the engine re-runs in full, which is what makes
/// the output byte-identical (entry order, labels and `crossref` inclusion
/// all depend on the whole citation list).
#[derive(Default)]
pub struct Session {
    cache: Option<CacheEntry>,
    /// How many [`Session::run`] calls were answered from the cache.
    pub hits: u64,
    /// How many ran the engine.
    pub misses: u64,
}

struct CacheEntry {
    aux: String,
    opts: Options,
    deps: Vec<(FileKind, String, Option<u64>)>,
    out: Output,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(
        &mut self,
        aux_name: &str,
        fs: &dyn FileSource,
        opts: &Options,
    ) -> Result<Output, RunError> {
        if let Some(c) = &self.cache {
            if c.aux == aux_name
                && &c.opts == opts
                && c.deps
                    .iter()
                    .all(|(k, n, h)| fs.read(*k, n).as_deref().map(hash_bytes) == *h)
            {
                self.hits += 1;
                return Ok(c.out.clone());
            }
        }
        self.misses += 1;
        let rec = Recorder { inner: fs, log: RefCell::new(Vec::new()) };
        let out = run(aux_name, &rec, opts)?;
        self.cache = Some(CacheEntry {
            aux: aux_name.to_string(),
            opts: opts.clone(),
            deps: rec.log.into_inner(),
            out: out.clone(),
        });
        Ok(out)
    }

    /// Drops the cached result.
    pub fn invalidate(&mut self) {
        self.cache = None;
    }
}

#[cfg(test)]
mod session_tests {
    use super::*;
    use std::cell::RefCell as StdRefCell;
    use std::collections::HashMap;

    /// An in-memory [`FileSource`] for tests, keyed as `"<kind>:<name>"`.
    #[derive(Default)]
    struct MemSource {
        files: StdRefCell<HashMap<(FileKind, String), Vec<u8>>>,
    }

    impl MemSource {
        fn new() -> Self {
            Self::default()
        }
        fn set(&self, kind: FileKind, name: &str, content: &str) {
            self.files.borrow_mut().insert((kind, name.to_string()), content.as_bytes().to_vec());
        }
    }

    impl FileSource for MemSource {
        fn read(&self, kind: FileKind, name: &str) -> Option<Vec<u8>> {
            self.files.borrow().get(&(kind, name.to_string())).cloned()
        }
    }

    /// A minimal, always-successful `.aux`/`.bst`/`.bib` trio: one entry,
    /// cited explicitly, formatted by a trivial style.
    fn basic_fixture() -> MemSource {
        let fs = MemSource::new();
        fs.set(
            FileKind::Aux,
            "job.aux",
            "\\citation{k1}\n\\bibdata{refs}\n\\bibstyle{sty}\n",
        );
        fs.set(
            FileKind::Bib,
            "refs",
            "@article{k1, author={A B}, title={T}, journal={J}, year={2000}}\n",
        );
        fs.set(
            FileKind::Bst,
            "sty",
            "ENTRY { title } { } { }\n\
             FUNCTION {article} { title write$ newline$ }\n\
             READ\n\
             ITERATE {call.type$}\n",
        );
        fs
    }

    #[test]
    fn cache_hit_when_inputs_unchanged() {
        let fs = basic_fixture();
        let opts = Options::default();
        let mut session = Session::new();

        let first = session.run("job.aux", &fs, &opts).expect("first run");
        assert_eq!(session.hits, 0);
        assert_eq!(session.misses, 1);

        let second = session.run("job.aux", &fs, &opts).expect("second run");
        assert_eq!(session.hits, 1);
        assert_eq!(session.misses, 1);
        assert_eq!(first.bbl, second.bbl);
        assert_eq!(first.blg, second.blg);

        // A third call, still nothing changed, is also a hit.
        let third = session.run("job.aux", &fs, &opts).expect("third run");
        assert_eq!(session.hits, 2);
        assert_eq!(session.misses, 1);
        assert_eq!(first.bbl, third.bbl);
    }

    #[test]
    fn reruns_when_aux_changes() {
        let fs = basic_fixture();
        let opts = Options::default();
        let mut session = Session::new();
        let first = session.run("job.aux", &fs, &opts).expect("first run");

        // Add a second entry and cite it: this rewrites the .aux (and thus
        // its hash), which must force a full re-run rather than a stale hit.
        fs.set(
            FileKind::Bib,
            "refs",
            "@article{k1, author={A B}, title={T}, journal={J}, year={2000}}\n\
             @article{k2, author={C D}, title={T2}, journal={J}, year={2001}}\n",
        );
        fs.set(
            FileKind::Aux,
            "job.aux",
            "\\citation{k1}\n\\citation{k2}\n\\bibdata{refs}\n\\bibstyle{sty}\n",
        );

        let second = session.run("job.aux", &fs, &opts).expect("second run");
        assert_eq!(session.hits, 0);
        assert_eq!(session.misses, 2);
        assert_ne!(first.bbl, second.bbl);
        assert!(second.bbl.windows(2).count() > 0);
    }

    #[test]
    fn reruns_when_bib_changes_but_aux_does_not() {
        let fs = basic_fixture();
        let opts = Options::default();
        let mut session = Session::new();
        let first = session.run("job.aux", &fs, &opts).expect("first run");

        // Change the cited entry's data without touching the .aux or .bst.
        fs.set(
            FileKind::Bib,
            "refs",
            "@article{k1, author={Z Y}, title={Changed}, journal={J}, year={2000}}\n",
        );

        let second = session.run("job.aux", &fs, &opts).expect("second run");
        assert_eq!(session.hits, 0);
        assert_eq!(session.misses, 2);
        assert_ne!(first.bbl, second.bbl);
    }

    #[test]
    fn reruns_when_bst_changes_but_aux_does_not() {
        let fs = basic_fixture();
        let opts = Options::default();
        let mut session = Session::new();
        session.run("job.aux", &fs, &opts).expect("first run");

        fs.set(
            FileKind::Bst,
            "sty",
            "ENTRY { title } { } { }\n\
             FUNCTION {article} { \"changed \" write$ title write$ newline$ }\n\
             READ\n\
             ITERATE {call.type$}\n",
        );

        session.run("job.aux", &fs, &opts).expect("second run");
        assert_eq!(session.hits, 0);
        assert_eq!(session.misses, 2);
    }

    #[test]
    fn invalidate_forces_a_rerun() {
        let fs = basic_fixture();
        let opts = Options::default();
        let mut session = Session::new();
        session.run("job.aux", &fs, &opts).expect("first run");
        session.invalidate();
        session.run("job.aux", &fs, &opts).expect("second run");
        assert_eq!(session.hits, 0);
        assert_eq!(session.misses, 2);
    }

    #[test]
    fn different_options_are_not_cached_together() {
        let fs = basic_fixture();
        let mut session = Session::new();
        let opts_a = Options::default();
        let opts_b = Options { min_crossrefs: 5, ..Options::default() };

        session.run("job.aux", &fs, &opts_a).expect("run a");
        session.run("job.aux", &fs, &opts_b).expect("run b");
        assert_eq!(session.hits, 0);
        assert_eq!(session.misses, 2);

        // Back to the first options: still a fresh cache entry (the last
        // run cached opts_b), so this is a miss too.
        session.run("job.aux", &fs, &opts_a).expect("run a again");
        assert_eq!(session.hits, 0);
        assert_eq!(session.misses, 3);
    }
}
