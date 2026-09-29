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
            offline: matches!(
                std::env::var("FLASHTEX_BUNDLE_OFFLINE").as_deref(),
                Ok("1" | "yes" | "true")
            ),
        })
    }
}

/// `FLASHTEX_BUNDLE_CACHE_DIR`, else `~/Library/Caches/FlashTeX/bundles`
/// (macOS) or `$XDG_CACHE_HOME/flashtex/bundles` (`~/.cache/...`).
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
            if e.path.starts_with('/')
                || !e.path.contains('/')
                || e.path
                    .split('/')
                    .any(|c| c.is_empty() || c == "." || c == "..")
                || e.basename() == "ls-R"
            {
                return Err(format!(
                    "bundle path {:?} is not a plain relative path",
                    e.path
                ));
            }
            by_name.entry(e.basename().to_string()).or_default().push(i);
        }
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

    /// `<root>/<tree>/ls-R` for each tree (`texmf-dist`, `texmf-var`), as
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

    /// The core packages, which the bundle stores first: one request.
    fn fetch_core(&mut self) -> Result<(), String> {
        let core: Vec<usize> = (0..self.index.packages.len())
            .filter(|&i| self.index.core.contains(&self.index.packages[i].name))
            .collect();
        let (Some(&first), Some(&last)) = (core.first(), core.last()) else {
            return Ok(());
        };
        let start = self.index.packages[first].start;
        let end = self.index.packages[last].start + self.index.packages[last].len;
        let data = self.read_range(start, end - start)?;
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
                let data = self.read_range(p.start, p.len)?;
                let members: Vec<usize> = (0..self.index.files.len())
                    .filter(|&j| self.package_of[j] == self.package_of[i])
                    .collect();
                for j in members {
                    self.store_member(j, &data, p.start)?;
                }
            }
            _ => {
                let e = self.index.files[i].clone();
                let data = self.read_range(e.start, e.gzip_len as u64)?;
                self.store_member(i, &data, e.start)?;
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

    /// The bundle's cache (from the environment), for `default_resolver`.
    pub fn from_env(progname: &str, engine: &str) -> Option<Result<BundleResolver, String>> {
        let spec = BundleSpec::from_env()?;
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
                if !self.failed.contains_key(&name) {
                    eprintln!("flashtex: bundle file {name} is not available: {e}");
                    self.failed.insert(name, e);
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
