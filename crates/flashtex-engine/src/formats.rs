//! The format cache (DESIGN.md 4.4): `pdflatex.fmt` and friends built from
//! the files the process's resolver sees -- the user's TeX Live, or the
//! bundle -- exactly as TeX Live's `fmtutil` builds them, once, and reused
//! for as long as nothing they were made from changes.
//!
//! **What is built, and how.** fmtutil's own configuration decides: every
//! `fmtutil.cnf` on the search path (`kpsewhich -all fmtutil.cnf`), merged
//! as `fmtutil.pl`'s `read_fmtutil_files` merges them (the first file that
//! mentions a format wins; a `#!` line disables it). The format's line gives
//! the INITEX command line, which is run as `rebuild_one_format` runs it:
//! `pdftex -ini -jobname=FMT -progname=FMT ARGS </dev/null` in an empty
//! directory (for pdflatex: `-translate-file=cp227.tcx *pdflatex.ini`).
//! Hyphenation patterns come from whatever `language.dat`/`language.def` the
//! ini files read, i.e. the user's. Only lines whose engine is `pdftex` are
//! this engine's to build.
//!
//! **What the cache is keyed by.** The INITEX run is made with
//! `FLASHTEX_READ_SET` (system.rs), which lists every lookup it made and
//! every file it opened. The format is stored under the SHA-256 of: the
//! engine build ([`engine_id`], the same for every binary of one engine), the command line, the
//! content hash of every file read, and every lookup's result (so a file
//! that would now shadow another, e.g. a `hyphen.cfg` added to TEXMFHOME,
//! invalidates it too).
//!
//! **What a hit costs.** A slot (one per engine build, resolver and format
//! name) holds a manifest naming the current format file, its read set with
//! each file's `stat` signature, and its lookups. A hit is: read the
//! manifest; `stat` every file (hash only those whose signature changed);
//! repeat the recorded lookups through the process's resolver (only when
//! its program name is the one the format was built with); read the
//! `fmtutil.cnf` files for the command line.
//! See docs/evidence/distribution-2026-09-29/.
//!
//! **Concurrency.** Builds take an exclusive `flock` on the slot, and check
//! again after getting it, so two app windows starting together build once.
//! Formats and manifests are written to temporary names and renamed into
//! place; the previous format file is kept for one more generation, so a
//! process that validated the old manifest can still open what it names.
//!
//! **Where.** `FLASHTEX_FORMAT_CACHE_DIR`, else `~/Library/Caches/FlashTeX/formats`
//! (macOS) or `$XDG_CACHE_HOME/flashtex/formats` (`~/.cache/...`). The
//! cache is off with `FLASHTEX_FORMAT_CACHE=off`.

use crate::resolver::{FileResolver, Format};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Why a format could not be provided.
#[derive(Debug)]
pub enum FormatError {
    /// No enabled `fmtutil.cnf` line builds this format with `pdftex`: the
    /// engine then reports the missing format as pdfTeX does.
    NotInFmtutil(String),
    /// The INITEX run failed or made no format.
    Build(String),
    /// The cache directory could not be used.
    Io(String),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::NotInFmtutil(s) => write!(f, "no fmtutil.cnf line builds {s} with pdftex"),
            FormatError::Build(s) => write!(f, "format build failed: {s}"),
            FormatError::Io(s) => write!(f, "format cache: {s}"),
        }
    }
}

fn io_err(what: &str, p: &Path, e: std::io::Error) -> FormatError {
    FormatError::Io(format!("{what} {}: {e}", p.display()))
}

/// Is the cache on (`FLASHTEX_FORMAT_CACHE` is not `off`, `0` or `no`)?
pub fn cache_enabled() -> bool {
    !matches!(
        std::env::var("FLASHTEX_FORMAT_CACHE").as_deref(),
        Ok("off" | "0" | "no")
    )
}

/// The cache directory (see the module documentation).
pub fn cache_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("FLASHTEX_FORMAT_CACHE_DIR").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(d));
    }
    cache_dir_default_root().map(|r| r.join("formats"))
}

/// FlashTeX's cache root: `~/Library/Caches/FlashTeX` on macOS, else
/// `$XDG_CACHE_HOME/flashtex` or `~/.cache/flashtex`.
pub fn cache_dir_default_root() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from);
    if cfg!(target_os = "macos") {
        return home.map(|h| h.join("Library/Caches/FlashTeX"));
    }
    if let Some(x) = std::env::var_os("XDG_CACHE_HOME").filter(|x| !x.is_empty()) {
        let x = PathBuf::from(x);
        if x.is_absolute() {
            return Some(x.join("flashtex"));
        }
    }
    home.map(|h| h.join(".cache/flashtex"))
}

// ---------------------------------------------------------------------------
// fmtutil.cnf
// ---------------------------------------------------------------------------

/// One format line of `fmtutil.cnf`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FmtutilLine {
    pub format: String,
    pub engine: String,
    /// The pattern file field (`language.dat`, `language.def`, `-`), which
    /// fmtutil only uses to decide what to rebuild.
    pub hyphen: String,
    /// The fourth field and on, joined by single spaces as `"@rest"` joins them.
    pub args: String,
    pub enabled: bool,
    pub origin: PathBuf,
}

/// `read_fmtutil_file`: the format lines of one file, in order; a format
/// mentioned twice keeps its first line.
pub fn parse_fmtutil_cnf(text: &str, origin: &Path) -> Vec<FmtutilLine> {
    let mut out: Vec<FmtutilLine> = vec![];
    for line in text.lines() {
        let t = line.trim();
        // Empty, blank or bare-`#` lines; whole-line comments that are not `#!`.
        if t.is_empty() || t == "#" || (t.starts_with('#') && !t.starts_with("#!")) {
            continue;
        }
        // `s/#[^!].*//; s/#$//`: a `#` not followed by `!` starts a comment.
        let mut l = line.to_string();
        let b = l.as_bytes();
        if let Some(i) =
            (0..b.len()).find(|&i| b[i] == b'#' && b.get(i + 1).is_some_and(|&c| c != b'!'))
        {
            l.truncate(i);
        }
        if l.ends_with('#') {
            l.pop();
        }
        let mut w: Vec<&str> = l.split_whitespace().collect();
        let mut enabled = true;
        if w.first() == Some(&"#!") {
            w.remove(0);
            enabled = false;
        }
        if w.len() < 4 {
            continue;
        }
        if out.iter().any(|o| o.format == w[0] && o.engine == w[1]) {
            continue;
        }
        out.push(FmtutilLine {
            format: w[0].into(),
            engine: w[1].into(),
            hyphen: w[2].into(),
            args: w[3..].join(" "),
            enabled,
            origin: origin.to_path_buf(),
        });
    }
    out
}

/// `read_fmtutil_files`: the files in precedence order (as `kpsewhich -all`
/// lists them); the first that mentions a format/engine pair decides it.
pub fn merge_fmtutil(files: &[(PathBuf, String)]) -> HashMap<(String, String), FmtutilLine> {
    let mut m = HashMap::new();
    for (path, text) in files.iter().rev() {
        for l in parse_fmtutil_cnf(text, path) {
            m.insert((l.format.clone(), l.engine.clone()), l);
        }
    }
    m
}

/// `rebuild_one_format`'s command line, after the engine name: `-ini`,
/// `-jobname=FMT`, `-progname=...` unless the line has its own, then the
/// line's arguments split on blanks (it runs through the shell, whose glob of
/// `*pdflatex.ini` in the empty build directory matches nothing).
pub fn fmtutil_command(l: &FmtutilLine) -> Result<Vec<String>, FormatError> {
    if l.args.starts_with("nls=") {
        return Err(FormatError::Build(format!(
            "{}: nls= (localised pool files) is not supported",
            l.format
        )));
    }
    let mut v = vec!["-ini".to_string(), format!("-jobname={}", l.format)];
    if !l.args.contains("-progname=") {
        let prog = match l.format.as_str() {
            "metafun" => "mpost",
            "mptopdf" => "context",
            f if f.len() == 7 && f.starts_with("cont-") => "context",
            f => f,
        };
        v.push(format!("-progname={prog}"));
    }
    v.extend(l.args.split_whitespace().map(str::to_string));
    Ok(v)
}

/// The program name a command line sets (`-progname=`).
fn command_progname(cmd: &[String]) -> Option<&str> {
    cmd.iter().find_map(|a| a.strip_prefix("-progname="))
}

/// The line that builds `fmt` with this engine (`pdftex`), from the
/// resolver's `fmtutil.cnf` files, and its command line.
pub fn fmtutil_line_for(
    fmt: &str,
    r: &mut dyn FileResolver,
) -> Result<(FmtutilLine, Vec<String>), FormatError> {
    let mut files = vec![];
    for p in r.find_all("fmtutil.cnf", Format::Cnf) {
        if let Ok(t) = fs::read_to_string(&p) {
            files.push((p, t));
        }
    }
    let merged = merge_fmtutil(&files);
    let line = merged
        .get(&(fmt.to_string(), "pdftex".to_string()))
        .filter(|l| l.enabled)
        .cloned()
        .ok_or_else(|| FormatError::NotInFmtutil(format!("{fmt}.fmt")))?;
    let cmd = fmtutil_command(&line)?;
    Ok((line, cmd))
}

// ---------------------------------------------------------------------------
// Hashes and stat signatures
// ---------------------------------------------------------------------------

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// SHA-256 of a file's content, hex.
pub fn sha256_file(p: &Path) -> std::io::Result<String> {
    let mut f = File::open(p)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(&h.finalize()))
}

/// What `stat` says about a file, enough to tell that it has not been
/// rewritten: size, modification and change times (ns), inode and device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatSig {
    pub size: u64,
    pub mtime_ns: i128,
    pub ctime_ns: i128,
    pub ino: u64,
    pub dev: u64,
}

impl StatSig {
    pub fn of(p: &Path) -> Option<StatSig> {
        use std::os::unix::fs::MetadataExt;
        let m = fs::metadata(p).ok()?;
        if !m.is_file() {
            return None;
        }
        Some(StatSig {
            size: m.size(),
            mtime_ns: m.mtime() as i128 * 1_000_000_000 + m.mtime_nsec() as i128,
            ctime_ns: m.ctime() as i128 * 1_000_000_000 + m.ctime_nsec() as i128,
            ino: m.ino(),
            dev: m.dev(),
        })
    }
    fn encode(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}",
            self.size, self.mtime_ns, self.ctime_ns, self.ino, self.dev
        )
    }
    fn decode(f: &[&str]) -> Option<StatSig> {
        Some(StatSig {
            size: f.first()?.parse().ok()?,
            mtime_ns: f.get(1)?.parse().ok()?,
            ctime_ns: f.get(2)?.parse().ok()?,
            ino: f.get(3)?.parse().ok()?,
            dev: f.get(4)?.parse().ok()?,
        })
    }
}

/// The engine's build id (crates/flashtex-engine/build-id): the SHA-256 of
/// every source of this crate that can affect what a format holds -- all of
/// src/ (generated and hand-written), changes/, the C shims, the string
/// pool, the capacities, Cargo.toml and build.rs. Every binary built from
/// the same engine (flashtex-initex, flashtex-dist, the host) has the same
/// id, so a format prepared by one is a cache hit for the others; any edit
/// to the engine makes new formats. (Hashing the running executable, as
/// this did first, gave each binary its own formats.)
pub fn engine_id() -> &'static str {
    flashtex_engine_build_id::ENGINE_BUILD_ID
}

/// Write `data` to a temporary file beside `p` and rename it into place.
/// The data reaches the disk before the rename (`fsync`), so after a crash
/// `p` is either the old file or the whole new one.
pub fn write_atomic(p: &Path, data: &[u8]) -> std::io::Result<()> {
    write_atomic_with(p, data, true)
}

/// `write_atomic` without the `fsync`, for files that are a cache of
/// content verified when it was fetched (bundle members): an `fsync` per
/// file costs milliseconds on macOS (`F_FULLFSYNC`), which for the ~270
/// files of a bundle's core was a second of a cold start.
pub fn write_atomic_cache(p: &Path, data: &[u8]) -> std::io::Result<()> {
    write_atomic_with(p, data, false)
}

fn write_atomic_with(p: &Path, data: &[u8], durable: bool) -> std::io::Result<()> {
    let tmp = p.with_file_name(format!(
        ".{}.tmp-{}-{}",
        p.file_name().and_then(|n| n.to_str()).unwrap_or("f"),
        std::process::id(),
        nanos()
    ));
    {
        let mut f = File::create(&tmp)?;
        f.write_all(data)?;
        if durable {
            f.sync_all()?;
        }
    }
    fs::rename(&tmp, p).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// The manifest
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRec {
    pub path: PathBuf,
    pub sha256: String,
    pub sig: StatSig,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lookup {
    /// kpathsea's format name (`Format::kpse_name`).
    pub format: String,
    pub must_exist: bool,
    pub name: String,
    /// The path found; empty for none.
    pub found: String,
}

/// A slot's manifest: what its current format was built from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub key: String,
    /// The format file's name in the slot (`<key>.fmt`).
    pub fmt_file: String,
    pub engine: String,
    pub resolver: String,
    pub command: Vec<String>,
    pub files: Vec<FileRec>,
    pub lookups: Vec<Lookup>,
}

const MANIFEST_MAGIC: &str = "flashtex-format-cache 1";

impl Manifest {
    pub fn encode(&self) -> String {
        let mut s = format!("{MANIFEST_MAGIC}\n");
        s += &format!(
            "key\t{}\nfmt\t{}\nengine\t{}\n",
            self.key, self.fmt_file, self.engine
        );
        s += &format!("resolver\t{}\n", self.resolver);
        for a in &self.command {
            s += &format!("arg\t{a}\n");
        }
        for f in &self.files {
            s += &format!(
                "file\t{}\t{}\t{}\n",
                f.sha256,
                f.sig.encode(),
                f.path.display()
            );
        }
        for l in &self.lookups {
            s += &format!(
                "lookup\t{}\t{}\t{}\t{}\n",
                l.format, l.must_exist as u8, l.name, l.found
            );
        }
        s
    }

    pub fn decode(t: &str) -> Option<Manifest> {
        let mut lines = t.lines();
        if lines.next()? != MANIFEST_MAGIC {
            return None;
        }
        let mut m = Manifest {
            key: String::new(),
            fmt_file: String::new(),
            engine: String::new(),
            resolver: String::new(),
            command: vec![],
            files: vec![],
            lookups: vec![],
        };
        for l in lines {
            let (tag, rest) = l.split_once('\t')?;
            match tag {
                "key" => m.key = rest.into(),
                "fmt" => m.fmt_file = rest.into(),
                "engine" => m.engine = rest.into(),
                "resolver" => m.resolver = rest.into(),
                "arg" => m.command.push(rest.into()),
                "file" => {
                    let f: Vec<&str> = rest.splitn(7, '\t').collect();
                    if f.len() != 7 {
                        return None;
                    }
                    m.files.push(FileRec {
                        sha256: f[0].into(),
                        sig: StatSig::decode(&f[1..6])?,
                        path: PathBuf::from(f[6]),
                    });
                }
                "lookup" => {
                    let f: Vec<&str> = rest.splitn(4, '\t').collect();
                    if f.len() != 4 {
                        return None;
                    }
                    m.lookups.push(Lookup {
                        format: f[0].into(),
                        must_exist: f[1] == "1",
                        name: f[2].into(),
                        found: f[3].into(),
                    });
                }
                _ => return None,
            }
        }
        (!m.key.is_empty() && !m.fmt_file.is_empty()).then_some(m)
    }

    /// The cache key: the SHA-256 of everything the format depends on.
    fn compute_key(
        engine: &str,
        command: &[String],
        files: &[FileRec],
        lookups: &[Lookup],
    ) -> String {
        let mut h = Sha256::new();
        h.update(b"flashtex-format-key 1\n");
        h.update(format!("engine\t{engine}\n").as_bytes());
        for a in command {
            h.update(format!("arg\t{a}\n").as_bytes());
        }
        let mut fl: Vec<String> = files
            .iter()
            .map(|f| format!("file\t{}\t{}\n", f.sha256, f.path.display()))
            .collect();
        fl.sort();
        let mut ll: Vec<String> = lookups
            .iter()
            .map(|l| {
                format!(
                    "lookup\t{}\t{}\t{}\t{}\n",
                    l.format, l.must_exist as u8, l.name, l.found
                )
            })
            .collect();
        ll.sort();
        for s in fl.iter().chain(ll.iter()) {
            h.update(s.as_bytes());
        }
        hex(&h.finalize())
    }
}

// ---------------------------------------------------------------------------
// The cache
// ---------------------------------------------------------------------------

/// What a validation found.
#[derive(Debug, PartialEq, Eq)]
pub enum Validity {
    /// Valid; `refresh` if some file's stat signature changed but its content did not.
    Valid {
        refresh: bool,
    },
    Missing,
    Stale(String),
}

/// Statistics of the last `ensure_format` call, for measurements.
#[derive(Clone, Debug, Default)]
pub struct EnsureStats {
    pub built: bool,
    pub files_checked: usize,
    pub files_rehashed: usize,
    pub lookups_checked: usize,
    /// Time spent re-running the lookups (ms).
    pub lookups_ms: f64,
    pub stale_reason: Option<String>,
}

/// One format cache directory.
pub struct FormatCache {
    pub dir: PathBuf,
    /// The engine executable the INITEX runs use (default: this one). It
    /// must be built from the same engine as the caller: formats are keyed
    /// by the caller's [`engine_id`].
    pub engine_exe: Option<PathBuf>,
    /// Environment for the INITEX runs, beyond the inherited one.
    pub env: Vec<(String, String)>,
    pub last: EnsureStats,
}

impl FormatCache {
    pub fn new(dir: PathBuf) -> FormatCache {
        FormatCache {
            dir,
            engine_exe: None,
            env: vec![],
            last: EnsureStats::default(),
        }
    }

    /// The slot for a format under one engine build and resolver.
    pub fn slot(&self, engine: &str, resolver: &str, fmt: &str) -> PathBuf {
        let mut h = Sha256::new();
        h.update(format!("slot\t{engine}\t{resolver}\t{fmt}").as_bytes());
        self.dir
            .join(format!("{fmt}-{}", &hex(&h.finalize())[..20]))
    }

    /// Check a slot's manifest against the files as they are now.
    pub fn validate(
        &mut self,
        slot: &Path,
        engine: &str,
        command: &[String],
        progname: &str,
        r: &mut dyn FileResolver,
    ) -> (Validity, Option<Manifest>) {
        let Ok(t) = fs::read_to_string(slot.join("manifest")) else {
            return (Validity::Missing, None);
        };
        let Some(m) = Manifest::decode(&t) else {
            return (Validity::Stale("unreadable manifest".into()), None);
        };
        if m.engine != engine {
            return (Validity::Stale("engine build changed".into()), Some(m));
        }
        if m.command != command {
            return (
                Validity::Stale("fmtutil.cnf command line changed".into()),
                Some(m),
            );
        }
        if !slot.join(&m.fmt_file).is_file() {
            return (Validity::Stale("format file missing".into()), Some(m));
        }
        let mut refresh = false;
        for f in &m.files {
            self.last.files_checked += 1;
            let now = StatSig::of(&f.path);
            if now == Some(f.sig) {
                continue;
            }
            if now.is_none() {
                return (
                    Validity::Stale(format!("{} is gone", f.path.display())),
                    Some(m),
                );
            }
            self.last.files_rehashed += 1;
            match sha256_file(&f.path) {
                Ok(h) if h == f.sha256 => refresh = true,
                _ => {
                    return (
                        Validity::Stale(format!("{} changed", f.path.display())),
                        Some(m),
                    )
                }
            }
        }
        // The lookups, where this process searches as the build did.
        let t_lookups = std::time::Instant::now();
        if command_progname(command) == Some(progname) {
            for l in &m.lookups {
                let Some(format) = Format::from_kpse_name(&l.format) else {
                    continue;
                };
                self.last.lookups_checked += 1;
                let found = if l.must_exist {
                    r.find_ex(&l.name, format, true).0
                } else {
                    r.find(&l.name, format)
                };
                let found = found
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();
                // A file in the working directory hides what the TeX trees
                // hold; the build ran in an empty directory. Such a lookup
                // says nothing about the trees, so it is not compared.
                if found.starts_with("./") || (!found.is_empty() && !found.starts_with('/')) {
                    continue;
                }
                if found != l.found {
                    return (
                        Validity::Stale(format!(
                            "lookup of {} ({}) now finds {:?}, not {:?}",
                            l.name, l.format, found, l.found
                        )),
                        Some(m),
                    );
                }
            }
        }
        self.last.lookups_ms += t_lookups.elapsed().as_secs_f64() * 1e3;
        (Validity::Valid { refresh }, Some(m))
    }

    /// The format `fmt` (e.g. `pdflatex`), from the cache or built now.
    /// `progname` is the running program's kpathsea name; `r` the process's
    /// resolver, whose `fmtutil.cnf` files and TeX trees the format is made from.
    pub fn ensure(
        &mut self,
        fmt: &str,
        progname: &str,
        r: &mut dyn FileResolver,
    ) -> Result<PathBuf, FormatError> {
        self.last = EnsureStats::default();
        let (_line, command) = fmtutil_line_for(fmt, r)?;
        fs::create_dir_all(&self.dir).map_err(|e| io_err("create", &self.dir, e))?;
        let engine = engine_id().to_string();
        let resolver = r.describe();
        let slot = self.slot(&engine, &resolver, fmt);
        let (v, m) = self.validate(&slot, &engine, &command, progname, r);
        if let (Validity::Valid { refresh }, Some(m)) = (&v, &m) {
            if *refresh {
                self.refresh_sigs(&slot, m);
            }
            return Ok(slot.join(&m.fmt_file));
        }
        fs::create_dir_all(&slot).map_err(|e| io_err("create", &slot, e))?;
        let lock_path = slot.join("lock");
        let lock = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)
            .map_err(|e| io_err("open", &lock_path, e))?;
        lock.lock().map_err(|e| io_err("lock", &lock_path, e))?;
        // Another process may have built it while we waited.
        self.last = EnsureStats::default();
        let (v, m) = self.validate(&slot, &engine, &command, progname, r);
        if let (Validity::Valid { .. }, Some(m)) = (&v, &m) {
            return Ok(slot.join(&m.fmt_file));
        }
        if let Validity::Stale(s) = v {
            self.last.stale_reason = Some(s);
        }
        let previous = m.map(|m| m.fmt_file);
        let out = self.build(
            &slot,
            fmt,
            &engine,
            &resolver,
            &command,
            previous.as_deref(),
        );
        let _ = lock.unlock();
        self.last.built = true;
        out
    }

    /// Rewrite the manifest with current stat signatures (content unchanged),
    /// if nobody holds the lock; otherwise leave it for next time.
    fn refresh_sigs(&self, slot: &Path, m: &Manifest) {
        let Ok(lock) = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(slot.join("lock"))
        else {
            return;
        };
        if lock.try_lock().is_err() {
            return;
        }
        let mut m2 = m.clone();
        for f in &mut m2.files {
            if let Some(s) = StatSig::of(&f.path) {
                f.sig = s;
            }
        }
        // Only if the manifest is still the one validated.
        if fs::read_to_string(slot.join("manifest")).ok().as_deref() == Some(m.encode().as_str()) {
            let _ = write_atomic(&slot.join("manifest"), m2.encode().as_bytes());
        }
        let _ = lock.unlock();
    }

    fn build(
        &mut self,
        slot: &Path,
        fmt: &str,
        engine: &str,
        resolver: &str,
        command: &[String],
        previous: Option<&str>,
    ) -> Result<PathBuf, FormatError> {
        // Leftovers of builds that died (builds only run under the lock).
        if let Ok(rd) = fs::read_dir(slot) {
            for e in rd.flatten() {
                if e.file_name().to_string_lossy().starts_with("build-") {
                    let _ = fs::remove_dir_all(e.path());
                }
            }
        }
        let work = slot.join(format!("build-{}-{}", std::process::id(), nanos()));
        fs::create_dir_all(&work).map_err(|e| io_err("create", &work, e))?;
        let result = self.build_in(&work, slot, fmt, engine, resolver, command, previous);
        let _ = fs::remove_dir_all(&work);
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn build_in(
        &mut self,
        work: &Path,
        slot: &Path,
        fmt: &str,
        engine: &str,
        resolver: &str,
        command: &[String],
        previous: Option<&str>,
    ) -> Result<PathBuf, FormatError> {
        use std::os::unix::process::CommandExt;
        // Outside the build directory, so that nothing the run does can read it.
        let read_set = slot.join(format!(".readset-{}", std::process::id()));
        let _ = fs::remove_file(&read_set);
        let exe = match &self.engine_exe {
            Some(p) => p.clone(),
            None => {
                std::env::current_exe().map_err(|e| FormatError::Io(format!("current_exe: {e}")))?
            }
        };
        let out_path = work.join(format!("{fmt}.out"));
        let out = File::create(&out_path).map_err(|e| io_err("create", &out_path, e))?;
        let err = out.try_clone().map_err(|e| io_err("dup", &out_path, e))?;
        let mut c = Command::new(&exe);
        // fmtutil runs the engine as `pdftex`.
        c.arg0("pdftex")
            .args(command)
            .current_dir(work)
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(err)
            .env("FLASHTEX_READ_SET", &read_set)
            .env("FLASHTEX_FORMAT_CACHE", "off")
            // When the build is made from inside a compile (system.rs
            // `find_format`), that compile's -output-directory is in
            // TEXMF_OUTPUT_DIRECTORY and would send the format there, and
            // its display list (FLASHTEX_DISPLAY_LIST=fd:3) is not the
            // format build's: fmtutil runs INITEX with neither.
            .env_remove("TEXMF_OUTPUT_DIRECTORY")
            .env_remove("FLASHTEX_DISPLAY_LIST")
            .env_remove("FLASHTEX_DISPLAY_LIST_HAVE_FONTS");
        for (k, v) in &self.env {
            c.env(k, v);
        }
        let status = c
            .status()
            .map_err(|e| FormatError::Build(format!("cannot run {}: {e}", exe.display())))?;
        let fmt_made = work.join(format!("{fmt}.fmt"));
        let log = work.join(format!("{fmt}.log"));
        let log_text = fs::read_to_string(&log).unwrap_or_default();
        let _ = fs::copy(&log, slot.join(format!("{fmt}.log")));
        if fs::metadata(&fmt_made).map(|m| m.len()).unwrap_or(0) == 0 {
            let _ = fs::remove_file(&read_set);
            let tail: Vec<&str> = log_text.lines().rev().take(12).collect();
            let tail: Vec<&str> = tail.into_iter().rev().collect();
            return Err(FormatError::Build(format!(
                "`pdftex {}` ({status}) made no {fmt}.fmt; log ends:\n{}",
                command.join(" "),
                tail.join("\n")
            )));
        }
        let rs = fs::read_to_string(&read_set).unwrap_or_default();
        let _ = fs::remove_file(&read_set);
        let (files, lookups) = read_set_records(&rs, work)?;
        let key = Manifest::compute_key(engine, command, &files, &lookups);
        let fmt_file = format!("{key}.fmt");
        let dest = slot.join(&fmt_file);
        // On disk before the manifest names it.
        File::open(&fmt_made)
            .and_then(|f| f.sync_all())
            .map_err(|e| io_err("sync", &fmt_made, e))?;
        fs::rename(&fmt_made, &dest).map_err(|e| io_err("rename to", &dest, e))?;
        let m = Manifest {
            key,
            fmt_file: fmt_file.clone(),
            engine: engine.into(),
            resolver: resolver.into(),
            command: command.to_vec(),
            files,
            lookups,
        };
        write_atomic(&slot.join("manifest"), m.encode().as_bytes())
            .map_err(|e| io_err("write", &slot.join("manifest"), e))?;
        // Keep the current and the previous format; remove older ones.
        if let Ok(rd) = fs::read_dir(slot) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if n.ends_with(".fmt") && n != fmt_file && Some(n.as_str()) != previous {
                    let _ = fs::remove_file(e.path());
                }
            }
        }
        Ok(dest)
    }
}

/// Parse a `FLASHTEX_READ_SET` file into hashed file records and lookups.
/// Relative paths are the build directory's; files inside it (none, unless
/// the run wrote and read back its own output) are not dependencies.
pub fn read_set_records(
    text: &str,
    work: &Path,
) -> Result<(Vec<FileRec>, Vec<Lookup>), FormatError> {
    let mut seen = HashSet::new();
    let mut files = vec![];
    let mut seen_l = HashSet::new();
    let mut lookups = vec![];
    for l in text.lines() {
        let f: Vec<&str> = l.split('\t').collect();
        match f.as_slice() {
            ["open", p] => {
                let path = if p.starts_with('/') {
                    PathBuf::from(p)
                } else {
                    work.join(p)
                };
                if path.starts_with(work) || !seen.insert(path.clone()) {
                    continue;
                }
                // Devices (`/dev/null`, which texsys.cfg's probes may open)
                // have no content to depend on.
                if fs::metadata(&path).is_ok_and(|m| !m.is_file()) {
                    continue;
                }
                let sig = StatSig::of(&path)
                    .ok_or_else(|| FormatError::Io(format!("stat {}", path.display())))?;
                let sha256 = sha256_file(&path).map_err(|e| io_err("hash", &path, e))?;
                files.push(FileRec { path, sha256, sig });
            }
            ["lookup", format, must, name, found] => {
                let k = (format.to_string(), must.to_string(), name.to_string());
                if !seen_l.insert(k) {
                    continue;
                }
                // A find in the build directory is `./name`; it cannot happen
                // (the directory is empty) but must not be compared later.
                let found = if found.starts_with("./") {
                    String::new()
                } else {
                    found.to_string()
                };
                lookups.push(Lookup {
                    format: format.to_string(),
                    must_exist: *must == "1",
                    name: name.to_string(),
                    found,
                });
            }
            _ => {}
        }
    }
    Ok((files, lookups))
}

/// The engine's entry point (system.rs `find_format`): the format `fmt`
/// from the default cache directory.
pub fn ensure_format(
    fmt: &str,
    progname: &str,
    r: &mut dyn FileResolver,
) -> Result<PathBuf, FormatError> {
    let dir =
        cache_dir().ok_or_else(|| FormatError::Io("no cache directory (HOME unset)".into()))?;
    let mut c = FormatCache::new(dir);
    let t = std::time::Instant::now();
    let p = c.ensure(fmt, progname, r)?;
    if std::env::var_os("FLASHTEX_DEBUG_FORMATS").is_some() {
        eprintln!(
            "[formats] {fmt}: {} in {:.1} ms ({} files checked, {} rehashed, {} lookups{})",
            if c.last.built { "built" } else { "cache hit" },
            t.elapsed().as_secs_f64() * 1e3,
            c.last.files_checked,
            c.last.files_rehashed,
            c.last.lookups_checked,
            c.last
                .stale_reason
                .as_deref()
                .map(|s| format!("; was stale: {s}"))
                .unwrap_or_default()
        );
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmtutil_lines_parse_and_merge_as_fmtutil_pl_does() {
        let sys = "# comment\n\
                   pdflatex pdftex language.dat -translate-file=cp227.tcx *pdflatex.ini\n\
                   pdftex pdftex language.def -translate-file=cp227.tcx *pdfetex.ini # trailing\n\
                   #! mptopdf pdftex - -translate-file=cp227.tcx mptopdf.tex\n\
                   short pdftex -\n";
        let local = "#! pdftex pdftex language.def -translate-file=cp227.tcx *pdfetex.ini\n";
        let m = merge_fmtutil(&[
            (PathBuf::from("/local/fmtutil.cnf"), local.into()),
            (PathBuf::from("/sys/fmtutil.cnf"), sys.into()),
        ]);
        let pl = &m[&("pdflatex".to_string(), "pdftex".to_string())];
        assert!(pl.enabled);
        assert_eq!(pl.args, "-translate-file=cp227.tcx *pdflatex.ini");
        assert_eq!(
            fmtutil_command(pl).unwrap(),
            [
                "-ini",
                "-jobname=pdflatex",
                "-progname=pdflatex",
                "-translate-file=cp227.tcx",
                "*pdflatex.ini"
            ]
        );
        // The first file listed wins: TEXMFLOCAL's `#!` disables pdftex.
        assert!(!m[&("pdftex".to_string(), "pdftex".to_string())].enabled);
        let mp = &m[&("mptopdf".to_string(), "pdftex".to_string())];
        assert!(!mp.enabled);
        assert_eq!(fmtutil_command(mp).unwrap()[2], "-progname=context");
        assert!(!m.contains_key(&("short".to_string(), "pdftex".to_string())));
    }

    #[test]
    fn manifest_round_trips() {
        let m = Manifest {
            key: "k".into(),
            fmt_file: "k.fmt".into(),
            engine: "e".into(),
            resolver: "kpathsea (/x)".into(),
            command: vec!["-ini".into(), "*pdflatex.ini".into()],
            files: vec![FileRec {
                path: PathBuf::from("/a b/latex.ltx"),
                sha256: "00".into(),
                sig: StatSig {
                    size: 1,
                    mtime_ns: -2,
                    ctime_ns: 3,
                    ino: 4,
                    dev: 5,
                },
            }],
            lookups: vec![Lookup {
                format: "tex".into(),
                must_exist: true,
                name: "hyphen.cfg".into(),
                found: String::new(),
            }],
        };
        assert_eq!(Manifest::decode(&m.encode()), Some(m));
    }
}
