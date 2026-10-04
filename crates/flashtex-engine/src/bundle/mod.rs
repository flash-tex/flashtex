//! The bundle fallback (DESIGN.md 4.4): where there is no TeX Live, files
//! come from a content-addressed bundle of unmodified TeX Live files,
//! fetched by byte range, verified against a pinned SHA-256, and kept in a
//! local cache. The decision record (why Tectonic's format but not
//! Tectonic's bundles) is docs/evidence/distribution-2026-09-29/.
//!
//! * **Format:** TTBv1 (`ttb.rs`), with each TeX Live package's files
//!   contiguous and a small core at the front.
//! * **Pinning:** a bundle is named by its TTBv1 digest, which is the
//!   SHA-256 of its file list with every file's SHA-256. The index is
//!   accepted only if it implies the pinned digest, and every file only if
//!   its content has the SHA-256 the index gives.
//! * **Fetching:** the header and index once, then the core packages in one
//!   request, then on demand: a whole package when it is small
//!   ([`PACKAGE_FETCH_LIMIT`]), else just the file.
//! * **Lookup:** files keep their TeX Live paths (`texmf-dist/tex/latex/base/
//!   article.cls`) under `<cache>/<digest>/tree`, which gets an `ls-R` made
//!   from the index, and lookups are kpathsea's own through that database
//!   ([`KpathseaResolver::for_bundle_tree`]), so its suffix rules hold
//!   exactly. The bundle holds one file per basename (the one kpathsea
//!   chose in the TeX Live it was made from), so search order cannot matter.
//!   kpathsea only returns a database entry that exists on disk, so files
//!   not yet fetched are made as empty placeholders -- only those a lookup
//!   could return, just before it -- and the file kpathsea picks is fetched
//!   before its path is returned. A file is present when its size is the
//!   index's.
//! * **Offline:** with `offline`, nothing is fetched; lookups find only what
//!   is already in the cache.
//! * **Configuration:** `FLASHTEX_BUNDLE_URL` and `FLASHTEX_BUNDLE_DIGEST`,
//!   else a `flashtex-bundle.lock` ([`lock_candidates`], [`parse_lock`]),
//!   so a pinned default ships as data beside the program. There is no
//!   built-in default: where bundles are hosted is the owner's decision.

pub mod build;
pub mod fetch;
pub mod gz;
pub mod serve;
pub mod ttb;

use crate::formats::{hex, write_atomic, write_atomic_cache};
use crate::resolver::{FileResolver, Format, KpathseaResolver};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

/// Packages whose members together are at most this many compressed bytes
/// are fetched whole when one of their files is needed.
pub const PACKAGE_FETCH_LIMIT: u64 = 256 * 1024;

/// Which bundle, and how to reach it.
#[derive(Clone, Debug)]
pub struct BundleSpec {
    /// `https://...`, `http://...`, `file://...` or an absolute path.
    pub url: String,
    /// The pinned TTBv1 digest, hex.
    pub digest: String,
    pub offline: bool,
}

impl BundleSpec {
    /// `FLASHTEX_BUNDLE_URL`, `FLASHTEX_BUNDLE_DIGEST`, `FLASHTEX_BUNDLE_OFFLINE=1`.
    pub fn from_env() -> Option<BundleSpec> {
        let url = std::env::var("FLASHTEX_BUNDLE_URL")
            .ok()
            .filter(|u| !u.is_empty());
        let digest = std::env::var("FLASHTEX_BUNDLE_DIGEST")
            .ok()
            .filter(|d| !d.is_empty())?;
        Some(BundleSpec {
            url: url.unwrap_or_default(),
            digest: digest.to_ascii_lowercase(),
            offline: offline_from_env(),
        })
    }

    /// The configured bundle and where its configuration came from: the
    /// environment ([`BundleSpec::from_env`]) when `FLASHTEX_BUNDLE_DIGEST`
    /// is set, else the first lock file of [`lock_candidates`] that exists
    /// ([`parse_lock`]). `FLASHTEX_BUNDLE_OFFLINE` applies to either. A lock
    /// file that exists but does not parse is an error, not "no bundle".
    ///
    /// **Fetching from a lock file fails closed:** a bundle configured by a
    /// lock file is offline (nothing is fetched; only what is already in
    /// the cache is read) unless `FLASHTEX_BUNDLE_ALLOW_FETCH` is `1` (a
    /// command-line user) or that bundle's digest (what the app passes
    /// once the user agreed to downloading that bundle). A bundle set in
    /// the environment is that user's explicit choice and may fetch (the app
    /// passes `FLASHTEX_BUNDLE_OFFLINE=1` until its user agreed).
    /// `FLASHTEX_BUNDLE_OFFLINE=1` makes either offline, whatever else.
    pub fn configured() -> Option<Result<(BundleSpec, SpecOrigin), String>> {
        Self::configured_with(&|k| std::env::var(k).ok(), &lock_candidates())
    }

    /// [`BundleSpec::configured`] over the variables `var` gives and the
    /// lock files `candidates` (testable without the process environment).
    pub fn configured_with(
        var: &dyn Fn(&str) -> Option<String>,
        candidates: &[PathBuf],
    ) -> Option<Result<(BundleSpec, SpecOrigin), String>> {
        let set = |k: &str| var(k).filter(|v| !v.is_empty());
        let flag = |k: &str| matches!(set(k).as_deref(), Some("1" | "yes" | "true"));
        let offline = flag("FLASHTEX_BUNDLE_OFFLINE");
        if let Some(digest) = set("FLASHTEX_BUNDLE_DIGEST") {
            let spec = BundleSpec {
                url: set("FLASHTEX_BUNDLE_URL").unwrap_or_default(),
                digest: digest.to_ascii_lowercase(),
                offline,
            };
            return Some(Ok((spec, SpecOrigin::Environment)));
        }
        let lock = candidates.iter().find(|p| p.is_file())?.clone();
        let allow = set("FLASHTEX_BUNDLE_ALLOW_FETCH").map(|a| a.to_ascii_lowercase());
        Some(
            std::fs::read_to_string(&lock)
                .map_err(|e| e.to_string())
                .and_then(|t| parse_lock(&t, lock.parent().unwrap_or(Path::new("."))))
                .map_err(|e| format!("{}: {e}", lock.display()))
                .map(|(url, digest)| {
                    // `1` (a command-line user), or the very digest the
                    // user agreed to (the app), so a lock that changed
                    // after the app read it is not fetched either.
                    let allow_fetch = matches!(allow.as_deref(), Some("1" | "yes" | "true"))
                        || allow.as_deref() == Some(digest.as_str());
                    let spec = BundleSpec {
                        url,
                        digest,
                        offline: offline || !allow_fetch,
                    };
                    (spec, SpecOrigin::LockFile(lock))
                }),
        )
    }
}

fn offline_from_env() -> bool {
    matches!(
        std::env::var("FLASHTEX_BUNDLE_OFFLINE").as_deref(),
        Ok("1" | "yes" | "true")
    )
}

/// Where a [`BundleSpec`] came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpecOrigin {
    /// `FLASHTEX_BUNDLE_URL` and `FLASHTEX_BUNDLE_DIGEST`.
    Environment,
    /// A `flashtex-bundle.lock` ([`lock_candidates`]).
    LockFile(PathBuf),
}

impl SpecOrigin {
    pub fn describe(&self) -> String {
        match self {
            SpecOrigin::Environment => "environment".into(),
            SpecOrigin::LockFile(p) => p.display().to_string(),
        }
    }
}

/// The lock file's name.
pub const LOCK_FILE: &str = "flashtex-bundle.lock";

/// Where a bundle lock file is looked for, first first:
///
/// 1. `FLASHTEX_BUNDLE_LOCK`, when set (then nothing else);
/// 2. the user's own, in the per-user configuration directory:
///    `~/Library/Application Support/FlashTeX/` (macOS),
///    `%APPDATA%\FlashTeX\` (Windows), `$XDG_CONFIG_HOME/flashtex/` or
///    `~/.config/flashtex/` (elsewhere);
/// 3. the one shipped with the program: beside the executable, then the
///    app bundle's `Contents/Resources/engine/` (the executable in
///    `Contents/Helpers` or `Contents/MacOS`).
///
/// So a pinned default ships as data next to the program, and a user's
/// own file (or the environment) overrides it.
pub fn lock_candidates() -> Vec<PathBuf> {
    if let Some(p) = std::env::var_os("FLASHTEX_BUNDLE_LOCK").filter(|p| !p.is_empty()) {
        return vec![PathBuf::from(p)];
    }
    let mut v = vec![];
    if let Some(d) = user_config_dir(crate::formats::HOST_OS, |k| std::env::var_os(k)) {
        v.push(d.join(LOCK_FILE));
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        v.push(dir.join(LOCK_FILE));
        v.push(dir.join("../Resources/engine").join(LOCK_FILE));
    }
    v
}

/// The per-user configuration directory of [`lock_candidates`] (2.).
pub fn user_config_dir(
    os: crate::formats::CacheOs,
    var: impl Fn(&str) -> Option<std::ffi::OsString>,
) -> Option<PathBuf> {
    use crate::formats::CacheOs;
    let set = |k: &str| var(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    match os {
        CacheOs::MacOs => Some(set("HOME")?.join("Library/Application Support/FlashTeX")),
        CacheOs::Windows => Some(set("APPDATA")?.join("FlashTeX")),
        CacheOs::Xdg => match set("XDG_CONFIG_HOME").filter(|p| p.is_absolute()) {
            Some(d) => Some(d.join("flashtex")),
            None => Some(set("HOME")?.join(".config/flashtex")),
        },
    }
}

/// A bundle lock file: `key = value` lines (a TOML subset; values may be
/// quoted), `#` comments, unknown keys ignored, so later versions can add
/// keys:
///
/// ```text
/// # The bundle the engine reads when there is no TeX Live.
/// url = "https://example.org/texlive-2026-core.ttb"
/// digest = "26c4b1e6f5a4248ae5fc3427078cadabdbb1a87ddeca345a4d548435fe9e5d30"
/// ```
///
/// `digest` (the TTBv1 digest, 64 hex digits) and `url` are required. A
/// `url` with no scheme that is not absolute is relative to the lock
/// file's directory (`dir`), for a bundle shipped beside it. Lines end in
/// LF or CRLF; a quoted value may be followed by a comment. The app's
/// reader (apps/mac EngineV3Bundle.parseLock) is held to the same answers
/// by the shared vectors in docs/contracts/bundle-lock-vectors.json.
pub fn parse_lock(text: &str, dir: &Path) -> Result<(String, String), String> {
    let mut url = None;
    let mut digest = None;
    for (n, line) in text.lines().enumerate() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let (k, v) = l
            .split_once('=')
            .ok_or_else(|| format!("line {}: expected key = value", n + 1))?;
        let v = v.trim();
        let v = match v.strip_prefix('"') {
            // `"value"`, then nothing or a comment.
            Some(rest) => {
                let (inner, after) = rest
                    .split_once('"')
                    .ok_or_else(|| format!("line {}: unterminated string", n + 1))?;
                let after = after.trim();
                if !after.is_empty() && !after.starts_with('#') {
                    return Err(format!("line {}: text after the string", n + 1));
                }
                inner
            }
            None => v.split('#').next().unwrap_or("").trim(),
        };
        match k.trim() {
            "url" => url = Some(v.to_string()),
            "digest" => digest = Some(v.to_ascii_lowercase()),
            _ => {}
        }
    }
    let digest = digest.ok_or("no digest")?;
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("digest {digest:?} is not a SHA-256"));
    }
    let url = url.filter(|u| !u.is_empty()).ok_or("no url")?;
    let url = if url.contains("://") || Path::new(&url).is_absolute() {
        url
    } else {
        dir.join(&url).display().to_string()
    };
    Ok((url, digest))
}

/// What a fetch is doing, for a progress display (`set_progress`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    /// `index` (header and index), `core` (the core packages, in chunks)
    /// or `file` (a file or package on demand).
    pub what: &'static str,
    /// The file or package fetched (`file`), else the digest.
    pub name: String,
    pub done: u64,
    pub total: u64,
}

type ProgressFn = dyn Fn(&Progress) + Send + Sync;
static PROGRESS: std::sync::OnceLock<Box<ProgressFn>> = std::sync::OnceLock::new();

/// Report every fetch's progress to `f` (once per process: the host's).
/// It is called before each request and after the last one of a step.
pub fn set_progress(f: impl Fn(&Progress) + Send + Sync + 'static) {
    let _ = PROGRESS.set(Box::new(f));
}

fn progress(what: &'static str, name: &str, done: u64, total: u64) {
    if let Some(f) = PROGRESS.get() {
        f(&Progress {
            what,
            name: name.to_string(),
            done,
            total,
        });
    }
}

/// The core is fetched in requests of at most this many bytes, so that
/// its progress can be shown (a few requests more on a cold start).
pub const CORE_CHUNK: u64 = 4 * 1024 * 1024;

/// `FLASHTEX_BUNDLE_CACHE_DIR`, else `bundles` under
/// [`crate::formats::cache_dir_default_root`]: `~/Library/Caches/FlashTeX`
/// (macOS), `%LOCALAPPDATA%\FlashTeX` (Windows) or `$XDG_CACHE_HOME/flashtex`
/// (`~/.cache/...`).
pub fn default_cache_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("FLASHTEX_BUNDLE_CACHE_DIR").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(d));
    }
    let formats = crate::formats::cache_dir_default_root()?;
    Some(formats.join("bundles"))
}

/// Requests and bytes fetched, for measurements.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FetchStats {
    pub requests: u64,
    pub bytes: u64,
    pub files_materialized: u64,
}

/// One bundle, opened against its local cache.
pub struct Bundle {
    pub spec: BundleSpec,
    /// `<cache>/<digest>`.
    pub dir: PathBuf,
    /// `<cache>/<digest>/tree`: the files at their TeX Live paths, and `ls-R`.
    pub files_dir: PathBuf,
    pub index: ttb::Index,
    by_name: BTreeMap<String, Vec<usize>>,
    package_of: Vec<Option<usize>>,
    source: Option<Box<dyn fetch::RangeSource>>,
    pub stats: FetchStats,
}

impl Bundle {
    /// Open the bundle: from the cache if its verified index is there,
    /// else (online) by fetching the header and index and then the core.
    pub fn open(spec: BundleSpec, cache: &Path) -> Result<Bundle, String> {
        if spec.digest.len() != 64 || !spec.digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("bundle digest {:?} is not a SHA-256", spec.digest));
        }
        let dir = cache.join(&spec.digest);
        let files_dir = dir.join("tree");
        let index_path = dir.join("index.gz");
        let mut source = None;
        let mut stats = FetchStats::default();
        let (index, fresh) = match fs::read(&index_path) {
            Ok(gzd) => (Self::verify_index(&gzd, 0, &spec.digest)?, false),
            Err(_) if spec.offline => {
                return Err(format!(
                    "bundle {} is not in the cache ({}) and offline mode is on",
                    spec.digest,
                    cache.display()
                ))
            }
            Err(_) => {
                progress("index", &spec.digest, 0, 0);
                let mut src = fetch::open(&spec.url)?;
                let hb = src.read_range(0, ttb::HEADER_SIZE)?;
                let h = ttb::Header::parse(&hb)?;
                if hex(&h.digest) != spec.digest {
                    return Err(format!(
                        "{} is bundle {}, not the pinned {}",
                        spec.url,
                        hex(&h.digest),
                        spec.digest
                    ));
                }
                let gzd = src.read_range(h.index_start, h.index_gzip_len as u64)?;
                stats.requests += 2;
                stats.bytes += ttb::HEADER_SIZE + h.index_gzip_len as u64;
                let ix = Self::verify_index(&gzd, h.index_real_len as usize, &spec.digest)?;
                fs::create_dir_all(&files_dir)
                    .map_err(|e| format!("{}: {e}", files_dir.display()))?;
                write_atomic(&index_path, &gzd)
                    .map_err(|e| format!("{}: {e}", index_path.display()))?;
                source = Some(src);
                (ix, true)
            }
        };
        fs::create_dir_all(&files_dir).map_err(|e| format!("{}: {e}", files_dir.display()))?;
        // kpathsea returns paths under the canonical directory
        // (`for_bundle_tree` canonicalises it), and `entry_of` compares with it.
        let files_dir = fs::canonicalize(&files_dir).unwrap_or(files_dir);
        let mut by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, e) in index.files.iter().enumerate() {
            if ttb::META_FILES.contains(&e.path.as_str()) || e.sha256.is_none() {
                continue;
            }
            // `<tree>/<path>`: a plain relative path inside a tree.
            ttb::check_member_path(&e.path)?;
            by_name.entry(e.basename().to_string()).or_default().push(i);
        }
        ttb::check_case_collisions(
            by_name
                .values()
                .flatten()
                .map(|&i| index.files[i].path.as_str()),
        )?;
        Self::write_ls_r(&files_dir, &index, &by_name)?;
        // Each file's package: the range that contains it (binary search
        // over the ranges ordered by offset).
        let mut ranges: Vec<(u64, u64, usize)> = index
            .packages
            .iter()
            .enumerate()
            .map(|(pi, p)| (p.start, p.start + p.len, pi))
            .collect();
        ranges.sort();
        let package_of: Vec<Option<usize>> = index
            .files
            .iter()
            .map(|e| {
                let k = ranges.partition_point(|r| r.0 <= e.start);
                let &(s, end, pi) = ranges.get(k.checked_sub(1)?)?;
                (e.start >= s && e.start + e.gzip_len as u64 <= end).then_some(pi)
            })
            .collect();
        let mut b = Bundle {
            spec,
            dir,
            files_dir,
            index,
            by_name,
            package_of,
            source,
            stats,
        };
        if fresh {
            b.fetch_core()?;
        }
        Ok(b)
    }

    /// `<root>/<tree>/ls-R` for each tree (`texmf-dist`, `texmf-var`, `texmf-config`), as
    /// `mktexlsr` would write it for every file of the bundle in that tree,
    /// fetched or not; once per bundle.
    fn write_ls_r(
        root: &Path,
        index: &ttb::Index,
        by_name: &BTreeMap<String, Vec<usize>>,
    ) -> Result<(), String> {
        // tree -> directory (relative to the tree) -> entries
        let mut trees: BTreeMap<&str, BTreeMap<&str, Vec<&str>>> = BTreeMap::new();
        for &i in by_name.values().flatten() {
            let (tree, rel) = index.files[i].path.split_once('/').unwrap();
            let dirs = trees.entry(tree).or_default();
            let (d, f) = rel.rsplit_once('/').unwrap_or(("", rel));
            dirs.entry(d).or_default().push(f);
            // Every ancestor is listed too, so kpathsea sees the directories.
            let mut a = d;
            while let Some((parent, child)) = a.rsplit_once('/') {
                dirs.entry(parent).or_default().push(child);
                a = parent;
            }
            if !a.is_empty() {
                dirs.entry("").or_default().push(a);
            }
        }
        for (tree, dirs) in trees {
            let p = root.join(tree).join("ls-R");
            if p.is_file() {
                continue;
            }
            let mut s = String::from(
                "% ls-R -- filename database for kpathsea; do not change this line.\n",
            );
            for (d, mut fs) in dirs {
                fs.sort();
                fs.dedup();
                s += &format!(
                    "{}:\n",
                    if d.is_empty() {
                        "./".to_string()
                    } else {
                        format!("./{d}")
                    }
                );
                for f in fs {
                    s += f;
                    s.push('\n');
                }
                s.push('\n');
            }
            fs::create_dir_all(root.join(tree)).map_err(|e| format!("{tree}: {e}"))?;
            write_atomic(&p, s.as_bytes()).map_err(|e| format!("{}: {e}", p.display()))?;
        }
        Ok(())
    }

    fn verify_index(gzd: &[u8], real_len: usize, digest: &str) -> Result<ttb::Index, String> {
        let text = gz::gunzip(gzd, real_len)?;
        let text = String::from_utf8(text).map_err(|_| "bundle index is not UTF-8".to_string())?;
        let ix = ttb::Index::parse(&text)?;
        if hex(&ix.digest()) != digest {
            return Err(format!(
                "bundle index does not match the pinned digest {digest}"
            ));
        }
        Ok(ix)
    }

    fn source(&mut self) -> Result<&mut Box<dyn fetch::RangeSource>, String> {
        if self.spec.offline {
            return Err("offline".into());
        }
        if self.source.is_none() {
            self.source = Some(fetch::open(&self.spec.url)?);
        }
        Ok(self.source.as_mut().unwrap())
    }

    fn read_range(&mut self, start: u64, len: u64) -> Result<Vec<u8>, String> {
        let v = self.source()?.read_range(start, len)?;
        self.stats.requests += 1;
        self.stats.bytes += len;
        Ok(v)
    }

    /// The core packages, which the bundle stores first: one byte range,
    /// fetched in requests of at most [`CORE_CHUNK`] bytes.
    fn fetch_core(&mut self) -> Result<(), String> {
        let core: Vec<usize> = (0..self.index.packages.len())
            .filter(|&i| self.index.core.contains(&self.index.packages[i].name))
            .collect();
        let (Some(&first), Some(&last)) = (core.first(), core.last()) else {
            return Ok(());
        };
        let start = self.index.packages[first].start;
        let end = self.index.packages[last].start + self.index.packages[last].len;
        let total = end - start;
        let mut data = Vec::with_capacity(total as usize);
        while (data.len() as u64) < total {
            let done = data.len() as u64;
            progress("core", &self.spec.digest, done, total);
            let n = CORE_CHUNK.min(total - done);
            data.extend_from_slice(&self.read_range(start + done, n)?);
        }
        progress("core", &self.spec.digest, total, total);
        let members: Vec<usize> = (0..self.index.files.len())
            .filter(|&i| self.package_of[i].is_some_and(|p| core.contains(&p)))
            .collect();
        for i in members {
            self.store_member(i, &data, start)?;
        }
        Ok(())
    }

    fn local_path(&self, i: usize) -> PathBuf {
        self.files_dir.join(&self.index.files[i].path)
    }

    /// Is file `i` in the cache (its size is the index's)?
    pub fn is_present(&self, i: usize) -> bool {
        fs::metadata(self.local_path(i))
            .is_ok_and(|m| m.len() == self.index.files[i].real_len as u64)
    }

    /// Verify and store member `i`, whose gzip bytes are in `data`, which
    /// starts at bundle offset `base`.
    fn store_member(&mut self, i: usize, data: &[u8], base: u64) -> Result<(), String> {
        if self.is_present(i) {
            return Ok(());
        }
        let e = &self.index.files[i];
        let off = (e.start - base) as usize;
        let gzd = data
            .get(off..off + e.gzip_len as usize)
            .ok_or_else(|| format!("{}: outside the fetched range", e.path))?;
        let content =
            gz::gunzip(gzd, e.real_len as usize).map_err(|m| format!("{}: {m}", e.path))?;
        let want = e.sha256.as_deref().unwrap_or("");
        if content.len() != e.real_len as usize || hex(&Sha256::digest(&content)) != want {
            return Err(format!(
                "{}: content does not match its SHA-256 in the bundle",
                e.path
            ));
        }
        let p = self.local_path(i);
        if let Some(d) = p.parent() {
            fs::create_dir_all(d).map_err(|m| format!("{}: {m}", d.display()))?;
        }
        write_atomic_cache(&p, &content).map_err(|m| format!("{}: {m}", p.display()))?;
        self.stats.files_materialized += 1;
        Ok(())
    }

    /// Fetch file `i` if it is not in the cache: with its whole package
    /// when that is small, else alone.
    pub fn materialize(&mut self, i: usize) -> Result<PathBuf, String> {
        if self.is_present(i) {
            return Ok(self.local_path(i));
        }
        match self.package_of[i].map(|p| self.index.packages[p].clone()) {
            Some(p) if p.len <= PACKAGE_FETCH_LIMIT => {
                progress("file", &p.name, 0, p.len);
                let data = self.read_range(p.start, p.len)?;
                let members: Vec<usize> = (0..self.index.files.len())
                    .filter(|&j| self.package_of[j] == self.package_of[i])
                    .collect();
                for j in members {
                    self.store_member(j, &data, p.start)?;
                }
                progress("file", &p.name, p.len, p.len);
            }
            _ => {
                let e = self.index.files[i].clone();
                progress("file", e.basename(), 0, e.gzip_len as u64);
                let data = self.read_range(e.start, e.gzip_len as u64)?;
                self.store_member(i, &data, e.start)?;
                progress("file", e.basename(), e.gzip_len as u64, e.gzip_len as u64);
            }
        }
        Ok(self.local_path(i))
    }

    /// Make an empty placeholder for every file a lookup of `name` could
    /// return (`name` itself, or `name` plus a suffix) that is not there yet.
    pub fn prepare_lookup(&self, name: &str) {
        if name.contains('/') || name.is_empty() {
            return;
        }
        let dotted = format!("{name}.");
        // Every basename that starts with `name` sorts in one run from it.
        for (b, is) in self.by_name.range(name.to_string()..) {
            if !b.starts_with(name) {
                break;
            }
            if b != name && !b.starts_with(&dotted) {
                continue;
            }
            for &i in is {
                let p = self.local_path(i);
                if !p.exists() {
                    if let Some(d) = p.parent() {
                        let _ = fs::create_dir_all(d);
                    }
                    // create_new: never truncate a file another process wrote.
                    let _ = fs::OpenOptions::new().write(true).create_new(true).open(&p);
                }
            }
        }
    }

    /// The index entry of a path kpathsea returned from the tree.
    pub fn entry_of(&self, p: &Path) -> Option<usize> {
        self.by_name
            .get(p.file_name()?.to_str()?)?
            .iter()
            .copied()
            .find(|&i| self.local_path(i) == p)
    }

    /// The entry at a bundle path (`texmf-dist/web2c/texmf.cnf`).
    pub fn entry_at(&self, path: &str) -> Option<usize> {
        let base = path.rsplit('/').next()?;
        self.by_name
            .get(base)?
            .iter()
            .copied()
            .find(|&i| self.index.files[i].path == path)
    }

    /// Every file's basename, for tests and tools.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.by_name.keys().map(String::as_str)
    }
}

/// A [`FileResolver`] over a [`Bundle`].
pub struct BundleResolver {
    pub bundle: Bundle,
    kpse: KpathseaResolver,
    /// Files that could not be fetched, reported once each.
    failed: HashMap<String, String>,
}

impl BundleResolver {
    pub fn open(
        spec: BundleSpec,
        cache: &Path,
        progname: &str,
        engine: &str,
    ) -> Result<BundleResolver, String> {
        let mut bundle = Bundle::open(spec, cache)?;
        // TeX Live's texmf.cnf, if the bundle has it, must be on disk before
        // kpathsea starts: it is what configures kpathsea (see
        // `KpathseaResolver::for_bundle_tree`).
        if let Some(i) = bundle.entry_at("texmf-dist/web2c/texmf.cnf") {
            bundle
                .materialize(i)
                .map_err(|e| format!("the bundle's texmf.cnf: {e}"))?;
        }
        let kpse = KpathseaResolver::for_bundle_tree(&bundle.files_dir, progname, engine);
        Ok(BundleResolver {
            bundle,
            kpse,
            failed: HashMap::new(),
        })
    }

    /// The configured bundle ([`BundleSpec::configured`]: the environment,
    /// else a lock file) in its cache, for `default_resolver`.
    pub fn from_env(progname: &str, engine: &str) -> Option<Result<BundleResolver, String>> {
        let spec = match BundleSpec::configured()? {
            Ok((spec, _)) => spec,
            Err(e) => return Some(Err(e)),
        };
        let cache = match default_cache_dir() {
            Some(c) => c,
            None => return Some(Err("no bundle cache directory (HOME unset)".into())),
        };
        Some(Self::open(spec, &cache, progname, engine))
    }

    fn resolve(&mut self, found: Option<PathBuf>) -> Option<PathBuf> {
        let p = found?;
        let Some(i) = self.bundle.entry_of(&p) else {
            // The working directory, or a file outside the bundle.
            return Some(p);
        };
        match self.bundle.materialize(i) {
            Ok(p) => Some(p),
            Err(e) => {
                let name = self.bundle.index.files[i].path.clone();
                if let std::collections::hash_map::Entry::Vacant(v) = self.failed.entry(name) {
                    eprintln!("flashtex: bundle file {} is not available: {e}", v.key());
                    v.insert(e);
                }
                None
            }
        }
    }
}

impl FileResolver for BundleResolver {
    fn find(&mut self, name: &str, format: Format) -> Option<PathBuf> {
        self.bundle.prepare_lookup(name);
        let found = self.kpse.find(name, format);
        self.resolve(found)
    }
    fn find_ex(&mut self, name: &str, format: Format, must_exist: bool) -> (Option<PathBuf>, bool) {
        self.bundle.prepare_lookup(name);
        let (found, _) = self.kpse.find_ex(name, format, must_exist);
        (self.resolve(found), false)
    }
    fn find_all(&mut self, name: &str, format: Format) -> Vec<PathBuf> {
        self.bundle.prepare_lookup(name);
        let all = self.kpse.find_all(name, format);
        all.into_iter()
            .filter_map(|p| self.resolve(Some(p)))
            .collect()
    }
    fn describe(&self) -> String {
        // The digest names the content; where it is fetched from does not
        // matter (the format cache keys its slots by this).
        format!("bundle {}", self.bundle.spec.digest)
    }
    fn config_var(&mut self, var: &str) -> Option<String> {
        self.kpse.config_var(var)
    }
    fn name_ok(&mut self, name: &str, write: bool) -> bool {
        self.kpse.name_ok(name, write)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::CacheOs;

    const D: &str = "26c4b1e6f5a4248ae5fc3427078cadabdbb1a87ddeca345a4d548435fe9e5d30";

    /// The vectors the app's parser is held to as well
    /// (docs/contracts/bundle-lock-vectors.json).
    #[cfg(unix)]
    #[test]
    fn lock_file_vectors() {
        use flashtex_display_list::json::Json;
        let v = Json::parse(include_str!(
            "../../../../docs/contracts/bundle-lock-vectors.json"
        ))
        .unwrap();
        let dir = Path::new(v.str_field("dir").unwrap());
        let cases = v.get("cases").and_then(Json::as_array).unwrap();
        assert!(cases.len() >= 10);
        for c in cases {
            let name = c.str_field("name").unwrap();
            let got = parse_lock(c.str_field("text").unwrap(), dir).ok();
            let want = c
                .str_field("url")
                .map(|u| (u.to_string(), c.str_field("digest").unwrap().to_string()));
            assert_eq!(got, want, "{name}");
        }
    }

    /// A lock file's bundle fetches nothing unless FLASHTEX_BUNDLE_ALLOW_FETCH=1
    /// (the app's consent); FLASHTEX_BUNDLE_OFFLINE wins over it; an
    /// environment bundle may fetch.
    #[test]
    fn a_lock_files_bundle_fails_closed() {
        let base = std::env::temp_dir().join(format!(
            "flashtex-lock-closed-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();
        let lock = base.join(LOCK_FILE);
        std::fs::write(&lock, format!("url = \"b.ttb\"\r\ndigest = \"{D}\"\r\n")).unwrap();
        let missing = base.join("none").join(LOCK_FILE);
        let cands = [missing.clone(), lock.clone()];
        let run = |vars: &[(&str, &str)]| {
            let vars: Vec<(String, String)> = vars
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            BundleSpec::configured_with(
                &move |k| vars.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone()),
                &cands,
            )
        };
        let (spec, origin) = run(&[]).unwrap().unwrap();
        assert_eq!(origin, SpecOrigin::LockFile(lock.clone()));
        assert_eq!(spec.digest, D);
        assert_eq!(spec.url, base.join("b.ttb").display().to_string());
        assert!(spec.offline, "no consent: offline");
        let (spec, _) = run(&[("FLASHTEX_BUNDLE_ALLOW_FETCH", "1")])
            .unwrap()
            .unwrap();
        assert!(!spec.offline, "consent given");
        let (spec, _) = run(&[("FLASHTEX_BUNDLE_ALLOW_FETCH", D)]).unwrap().unwrap();
        assert!(!spec.offline, "consent given for this digest");
        let other = "00".repeat(32);
        let (spec, _) = run(&[("FLASHTEX_BUNDLE_ALLOW_FETCH", other.as_str())])
            .unwrap()
            .unwrap();
        assert!(spec.offline, "consent was for another bundle");
        let both = [
            ("FLASHTEX_BUNDLE_ALLOW_FETCH", "1"),
            ("FLASHTEX_BUNDLE_OFFLINE", "1"),
        ];
        assert!(run(&both).unwrap().unwrap().0.offline, "offline wins");
        let env = [
            ("FLASHTEX_BUNDLE_DIGEST", D),
            ("FLASHTEX_BUNDLE_URL", "file:///x"),
        ];
        let (spec, origin) = run(&env).unwrap().unwrap();
        assert_eq!(origin, SpecOrigin::Environment);
        assert!(!spec.offline, "the environment's own bundle");
        // A lock that does not parse is an error, never "no bundle".
        std::fs::write(&lock, "url = \"b.ttb\n").unwrap();
        assert!(run(&[]).unwrap().is_err());
        // No lock at all.
        assert!(BundleSpec::configured_with(&|_| None, &[missing]).is_none());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn user_config_dir_per_os() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |k: &str| {
                pairs
                    .iter()
                    .find(|(n, _)| *n == k)
                    .map(|(_, v)| std::ffi::OsString::from(v))
            }
        };
        assert_eq!(
            user_config_dir(CacheOs::MacOs, env(&[("HOME", "/h")])),
            Some(PathBuf::from("/h/Library/Application Support/FlashTeX"))
        );
        assert_eq!(
            user_config_dir(CacheOs::Xdg, env(&[("HOME", "/h")])),
            Some(PathBuf::from("/h/.config/flashtex"))
        );
        assert_eq!(
            user_config_dir(CacheOs::Windows, env(&[("APPDATA", "A")])),
            Some(PathBuf::from("A").join("FlashTeX"))
        );
        assert_eq!(user_config_dir(CacheOs::MacOs, env(&[])), None);
    }
}
