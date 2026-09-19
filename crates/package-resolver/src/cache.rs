//! The per-user package cache: `<root>/<name>/<version>/<files…>` plus
//! `<root>/<name>/<version>/manifest.json` recording where the files came
//! from, when, and their digests.
//!
//! ```json
//! {"name":"cancel","version":"2.2",
//!  "source_url":"https://mirrors.ctan.org/macros/latex/contrib/cancel/",
//!  "fetched_utc":"2026-09-19T10:11:12Z",
//!  "files":[{"name":"cancel.sty","sha256":"…","bytes":1234}]}
//! ```
//!
//! A store is atomic per version: the files land in a temporary sibling
//! directory and are renamed into place last, so a crash mid-fetch leaves no
//! half-written version, and a version directory without a `manifest.json`
//! is treated as absent. Nothing is ever executed from the cache.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{display, is_package_file, is_valid_name, iso_utc, ResolvedFile};

/// The cache manifest's file name inside a version directory.
pub const MANIFEST_NAME: &str = "manifest.json";

/// One cached version of one package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub version: String,
    /// `<root>/<name>/<version>`.
    pub dir: PathBuf,
    pub source_url: String,
    pub fetched_utc: String,
    /// File names with their recorded digests, sorted by name.
    pub files: Vec<CachedFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedFile {
    pub name: String,
    pub sha256: String,
    pub bytes: u64,
}

impl Entry {
    /// Reads every recorded file as text, verifying each digest: a file
    /// that changed under the cache (or vanished) is an error naming it,
    /// never silently different source.
    pub fn read(&self) -> Result<Vec<ResolvedFile>, String> {
        let mut out = Vec::with_capacity(self.files.len());
        for f in &self.files {
            let path = self.dir.join(&f.name);
            let bytes = fs::read(&path).map_err(|e| format!("cannot read cached {}: {e}", display(&path)))?;
            let digest = sha256_hex(&bytes);
            if digest != f.sha256 {
                return Err(format!(
                    "cached {} does not match its recorded digest (the cache was modified; `flashtex packages clear {}` and fetch again)",
                    display(&path),
                    self.name
                ));
            }
            let text = String::from_utf8(bytes).map_err(|_| format!("cached {} is not UTF-8", display(&path)))?;
            out.push(ResolvedFile { name: f.name.clone(), path, text });
        }
        Ok(out)
    }
}

/// The cache at one root directory.
#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Store {
        Store { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The cached version of `name` that satisfies `pin` (that exact
    /// version), or without a pin the most recently fetched one (ties go
    /// to the greater version string). `Ok(None)` when nothing usable is
    /// cached; a corrupt manifest is an error naming it.
    pub fn lookup(&self, name: &str, pin: Option<&str>) -> Result<Option<Entry>, String> {
        if !is_valid_name(name) {
            return Ok(None);
        }
        if let Some(pin) = pin {
            let dir = self.root.join(name).join(pin);
            return match read_manifest(&dir) {
                Ok(Some(entry)) => Ok(Some(entry)),
                Ok(None) => Ok(None),
                Err(e) => Err(e),
            };
        }
        let mut best: Option<Entry> = None;
        for entry in self.versions(name)? {
            let better = match &best {
                None => true,
                Some(b) => (entry.fetched_utc.as_str(), entry.version.as_str()) > (b.fetched_utc.as_str(), b.version.as_str()),
            };
            if better {
                best = Some(entry);
            }
        }
        Ok(best)
    }

    /// Every cached version of `name`, sorted by version string.
    pub fn versions(&self, name: &str) -> Result<Vec<Entry>, String> {
        let dir = self.root.join(name);
        let rd = match fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(format!("cannot list {}: {e}", display(&dir))),
        };
        let mut out = Vec::new();
        for e in rd {
            let e = e.map_err(|e| format!("cannot list {}: {e}", display(&dir)))?;
            let path = e.path();
            // A version directory still being written (`.tmp-…`) is not a version.
            if !path.is_dir() || e.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if let Some(entry) = read_manifest(&path)? {
                out.push(entry);
            }
        }
        out.sort_by(|a, b| a.version.cmp(&b.version));
        Ok(out)
    }

    /// Every cached package, by name then version.
    pub fn list(&self) -> Result<Vec<Entry>, String> {
        let rd = match fs::read_dir(&self.root) {
            Ok(rd) => rd,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(format!("cannot list {}: {e}", display(&self.root))),
        };
        let mut names: Vec<String> = rd
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| is_valid_name(n))
            .collect();
        names.sort();
        let mut out = Vec::new();
        for n in names {
            out.extend(self.versions(&n)?);
        }
        Ok(out)
    }

    /// Writes `files` as `<root>/<name>/<version>/` with its manifest,
    /// replacing an existing copy of that version. Only package files
    /// ([`is_package_file`]) are accepted; a name with a separator is refused.
    pub fn store(&self, name: &str, version: &str, source_url: &str, files: &[(String, Vec<u8>)]) -> Result<Entry, String> {
        if !is_valid_name(name) {
            return Err(format!("{name:?} is not a package name"));
        }
        if version.is_empty() || version.starts_with('.') || version.contains(['/', '\\', '\0']) {
            return Err(format!("{version:?} is not a version string a directory can be named after"));
        }
        for (file, _) in files {
            if !is_package_file(file) || file.contains(['/', '\\', '\0']) || file == MANIFEST_NAME {
                return Err(format!("{file:?} is not a package file"));
            }
        }
        let package_dir = self.root.join(name);
        fs::create_dir_all(&package_dir).map_err(|e| format!("cannot create {}: {e}", display(&package_dir)))?;
        let staging = package_dir.join(format!(".tmp-{version}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&staging);
        fs::create_dir(&staging).map_err(|e| format!("cannot create {}: {e}", display(&staging)))?;
        let mut recorded: Vec<CachedFile> = Vec::with_capacity(files.len());
        for (file, bytes) in files {
            let path = staging.join(file);
            fs::write(&path, bytes).map_err(|e| format!("cannot write {}: {e}", display(&path)))?;
            recorded.push(CachedFile { name: file.clone(), sha256: sha256_hex(bytes), bytes: bytes.len() as u64 });
        }
        recorded.sort_by(|a, b| a.name.cmp(&b.name));
        let fetched_utc = iso_utc(std::time::SystemTime::now());
        let manifest = json!({
            "name": name,
            "version": version,
            "source_url": source_url,
            "fetched_utc": fetched_utc,
            "files": recorded.iter().map(|f| json!({"name": f.name, "sha256": f.sha256, "bytes": f.bytes})).collect::<Vec<_>>(),
        });
        let manifest_path = staging.join(MANIFEST_NAME);
        fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).expect("json") + "\n")
            .map_err(|e| format!("cannot write {}: {e}", display(&manifest_path)))?;
        let final_dir = package_dir.join(version);
        if final_dir.exists() {
            fs::remove_dir_all(&final_dir).map_err(|e| format!("cannot replace {}: {e}", display(&final_dir)))?;
        }
        fs::rename(&staging, &final_dir).map_err(|e| format!("cannot move {} into place: {e}", display(&final_dir)))?;
        Ok(Entry { name: name.into(), version: version.into(), dir: final_dir, source_url: source_url.into(), fetched_utc, files: recorded })
    }

    /// Removes one package (every version) or, with `None`, the whole
    /// cache. Answers how many version directories went.
    pub fn clear(&self, name: Option<&str>) -> Result<usize, String> {
        let entries = match name {
            Some(n) => self.versions(n)?,
            None => self.list()?,
        };
        let count = entries.len();
        let target = match name {
            Some(n) if is_valid_name(n) => self.root.join(n),
            Some(n) => return Err(format!("{n:?} is not a package name")),
            None => self.root.clone(),
        };
        match fs::remove_dir_all(&target) {
            Ok(()) => Ok(count),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(0),
            Err(e) => Err(format!("cannot remove {}: {e}", display(&target))),
        }
    }
}

/// `<dir>/manifest.json` as an [`Entry`]; `Ok(None)` when there is none.
fn read_manifest(dir: &Path) -> Result<Option<Entry>, String> {
    let path = dir.join(MANIFEST_NAME);
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("cannot read {}: {e}", display(&path))),
    };
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("{} is not JSON: {e}", display(&path)))?;
    let str_of = |key: &str| v.get(key).and_then(Value::as_str).map(str::to_string).ok_or_else(|| format!("{} lacks {key:?}", display(&path)));
    let files = v
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{} lacks \"files\"", display(&path)))?
        .iter()
        .map(|f| {
            Some(CachedFile {
                name: f.get("name")?.as_str()?.to_string(),
                sha256: f.get("sha256")?.as_str()?.to_string(),
                bytes: f.get("bytes")?.as_u64()?,
            })
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| format!("{} has a malformed \"files\" entry", display(&path)))?;
    if files.iter().any(|f| !is_package_file(&f.name) || f.name.contains(['/', '\\'])) {
        return Err(format!("{} names a file that is not a package file", display(&path)));
    }
    Ok(Some(Entry {
        name: str_of("name")?,
        version: str_of("version")?,
        dir: dir.to_path_buf(),
        source_url: str_of("source_url")?,
        fetched_utc: str_of("fetched_utc")?,
        files,
    }))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// The version a registry that states none gets: the first 12 hex digits
/// of the SHA-256 over the files (sorted by name; name, NUL, bytes, NUL),
/// so the same content always has the same version and a pin means
/// something.
pub fn content_version(files: &[(String, Vec<u8>)]) -> String {
    let mut sorted: Vec<&(String, Vec<u8>)> = files.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut h = Sha256::new();
    for (name, bytes) in sorted {
        h.update(name.as_bytes());
        h.update([0]);
        h.update(bytes);
        h.update([0]);
    }
    let digest = h.finalize();
    digest.iter().take(6).map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::tmp;

    fn file(name: &str, text: &str) -> (String, Vec<u8>) {
        (name.into(), text.as_bytes().to_vec())
    }

    #[test]
    fn store_lookup_list_and_clear() {
        let root = tmp("cache");
        let store = Store::new(&root);
        assert_eq!(store.lookup("cancel", None).unwrap(), None);
        assert!(store.list().unwrap().is_empty(), "no root yet is an empty cache");
        let e = store.store("cancel", "2.2", "https://m/cancel/", &[file("cancel.sty", "a"), file("cancel.cfg", "b")]).unwrap();
        assert_eq!(e.dir, root.join("cancel/2.2"));
        assert_eq!(e.files.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["cancel.cfg", "cancel.sty"]);
        assert_eq!(e.files[1].sha256, sha256_hex(b"a"));
        assert!(root.join("cancel/2.2/manifest.json").is_file());
        assert!(!root.join("cancel").join(format!(".tmp-2.2-{}", std::process::id())).exists(), "staging renamed away");
        let read = e.read().unwrap();
        assert_eq!(read[0].text, "b");
        assert_eq!(read[1].path, root.join("cancel/2.2/cancel.sty"));

        // A second version: the pinned lookup is exact, the free one is the newest fetch.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        store.store("cancel", "2.1", "https://m/cancel/", &[file("cancel.sty", "old")]).unwrap();
        assert_eq!(store.lookup("cancel", Some("2.2")).unwrap().unwrap().version, "2.2");
        assert_eq!(store.lookup("cancel", Some("2.1")).unwrap().unwrap().version, "2.1");
        assert_eq!(store.lookup("cancel", Some("3.0")).unwrap(), None);
        assert_eq!(store.lookup("cancel", None).unwrap().unwrap().version, "2.1", "fetched later wins without a pin");
        assert_eq!(store.versions("cancel").unwrap().iter().map(|e| e.version.as_str()).collect::<Vec<_>>(), ["2.1", "2.2"]);
        store.store("other", "1", "u", &[file("other.cls", "c")]).unwrap();
        assert_eq!(store.list().unwrap().iter().map(|e| format!("{}@{}", e.name, e.version)).collect::<Vec<_>>(), ["cancel@2.1", "cancel@2.2", "other@1"]);

        // Tampering is detected on read, never served.
        std::fs::write(root.join("cancel/2.2/cancel.sty"), "changed").unwrap();
        let err = store.lookup("cancel", Some("2.2")).unwrap().unwrap().read().unwrap_err();
        assert!(err.contains("digest"), "{err}");
        // A version directory without a manifest is not a version.
        std::fs::create_dir_all(root.join("cancel/9.9")).unwrap();
        assert_eq!(store.lookup("cancel", Some("9.9")).unwrap(), None);

        assert_eq!(store.clear(Some("cancel")).unwrap(), 2);
        assert!(!root.join("cancel").exists());
        assert_eq!(store.clear(None).unwrap(), 1);
        assert!(!root.exists());
        assert_eq!(store.clear(None).unwrap(), 0);
    }

    #[test]
    fn refuses_non_package_files_and_bad_names() {
        let root = tmp("cache-refuse");
        let store = Store::new(&root);
        assert!(store.store("x", "1", "u", &[file("x.pdf", "")]).is_err());
        assert!(store.store("x", "1", "u", &[file("manifest.json", "")]).is_err());
        assert!(store.store("x", "1", "u", &[file("../x.sty", "")]).is_err());
        assert!(store.store("../x", "1", "u", &[file("x.sty", "")]).is_err());
        assert!(store.store("x", "../1", "u", &[file("x.sty", "")]).is_err());
        assert!(store.store("x", "", "u", &[file("x.sty", "")]).is_err());
        assert!(!root.join("x").exists() || store.list().unwrap().is_empty());
        assert_eq!(content_version(&[file("b.sty", "1"), file("a.sty", "2")]), content_version(&[file("a.sty", "2"), file("b.sty", "1")]));
        assert_ne!(content_version(&[file("a.sty", "1")]), content_version(&[file("a.sty", "2")]));
        let _ = std::fs::remove_dir_all(&root);
    }
}
