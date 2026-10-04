//! Typst packages (`#import "@preview/name:1.2.3"`), DESIGN.md §15.2
//! "Packages": where they come from, the FlashTeX package lock, offline
//! mode and consent.
//!
//! A package is looked up, in order:
//!
//! 1. **Vendored** in the project: `<root>/typst-packages/<ns>/<name>/<ver>/`
//!    (the "vendor packages into the project" layout; any namespace). They
//!    are project sources, read like the rest of the project.
//! 2. The host's **package paths** (`--package-path DIR`, any namespace;
//!    typst's `<data>/typst/packages` layout), when the app passes one.
//! 3. The host's **package cache** (`--package-cache DIR`, the app's):
//!    `<cache>/<ns>/<name>/<ver>/`, plus the tarball it was unpacked from in
//!    `<cache>/.tarballs/<ns>/<name>-<ver>.tar.gz`. A cached package is used
//!    only with its tarball, and the tarball's SHA-256 is checked against
//!    the project's lock (recorded there if the lock has no entry yet). The
//!    unpacked tree is checked against the tarball once per process (same
//!    files, same bytes, nothing more); a tree that differs is replaced by
//!    a fresh unpack of the checked tarball.
//! 4. **The network**, for `@preview` only, and only when the client's
//!    `COMPILE` says `"packages": "online"` (the app sends that after its
//!    first-use consent sheet) and the host was not started `--offline`.
//!    The fetch runs on its own thread: the compile that starts it waits at
//!    most [`WAIT`], any other compile not at all; either fails with a
//!    located error ("downloading …"), and the host sends `PACKAGE` (spec
//!    §11.8) when the fetch has finished, so the client compiles again. A
//!    failed fetch is retried only after [`backoff`] (5 s, doubling, at
//!    most 5 minutes); until then a compile reports the failure.
//!
//! **The lock** (`flashtex-typst.lock`, [`crate::lock`]): the tarball's
//! SHA-256 is recorded on the first fetch and checked on every later fetch
//! and every use of the cached copy; a mismatch is an error, never accepted.
//! typst-kit checks nothing but TLS and the Universe index carries no hash
//! (Track A §6, Track C §2.6).
//!
//! **Transport: the system `curl`** (an absolute path, [`system_curl`];
//! `-q` so no `~/.curlrc` applies; at most [`MAX_DOWNLOAD`] bytes, counted
//! while reading). typst-kit's `system-downloader` would
//! add ureq, native-tls, openssl and env_proxy (the `openssl` crate needs
//! OpenSSL headers or a vendored OpenSSL build on macOS) to an MIT host
//! whose licence list we audit (§15.8); `curl` ships with macOS, Windows 10+
//! and every mainstream Linux, uses the platform's TLS and proxy settings,
//! and adds no dependency. The host restricts it to the mirror's scheme
//! (`https` for packages.typst.org, also on redirects), sends a FlashTeX
//! User-Agent, and fetches only the one tarball a compile needs (no index,
//! except to name the latest version after a 404; no prefetch).
//!
//! Every resolved package root is canonical, and a file of the package is
//! read only if it resolves inside that root (no symlink escape), as for
//! the project root.

use std::any::Any;
use std::collections::HashMap;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use flashtex_display_list::json::Json;
use flashtex_display_list::sha256::{hex, sha256};
use typst::diag::{FileError, FileResult, PackageError};
use typst::syntax::package::PackageSpec;
use typst_kit::downloader::Downloader;
use typst_kit::packages::{FsPackages, UniversePackages};

use crate::lock::{Lock, FILE as LOCK_FILE};

/// The vendored-packages directory in the project root.
pub const VENDOR_DIR: &str = "typst-packages";

/// The namespace the Universe serves (and the only one fetched).
pub const UNIVERSE: &str = "preview";

/// The Typst Universe.
pub const DEFAULT_MIRROR: &str = "https://packages.typst.org";

/// How long a compile waits for a fetch it started before failing with
/// "downloading …" (a fast fetch then needs no second compile).
pub const WAIT: Duration = Duration::from_millis(200);

/// The `PACKAGE` message kind (host → client, spec §11.8), sent only to a
/// client whose `HELLO` `accept`s [`CAPABILITY`].
pub const KIND: u8 = flashtex_display_list::kind::PACKAGE;

/// The host capability and `accept` token for `PACKAGE` (spec §11.8).
pub const CAPABILITY: &str = "packages-v1";

/// The User-Agent of every fetch.
pub fn user_agent() -> String {
    format!(
        "FlashTeX-typst-host/{} (Typst {})",
        env!("CARGO_PKG_VERSION"),
        crate::TYPST_VERSION
    )
}

/// The host's package configuration.
#[derive(Clone, Debug)]
pub struct PackageOptions {
    /// Where fetched packages (and their tarballs) are kept; `None`: never
    /// fetch.
    pub cache: Option<PathBuf>,
    /// Read-only package directories (typst's layout), any namespace.
    pub paths: Vec<PathBuf>,
    /// The Universe or a mirror of it (`https://…`, or `file://…`).
    pub mirror: String,
    /// `--offline`: never fetch, whatever a client says.
    pub offline: bool,
    /// The `curl` program, an absolute path ([`system_curl`]).
    pub curl: PathBuf,
}

impl Default for PackageOptions {
    fn default() -> Self {
        PackageOptions {
            cache: None,
            paths: vec![],
            mirror: DEFAULT_MIRROR.into(),
            offline: false,
            curl: system_curl(),
        }
    }
}

/// What the host tells the client about a package (spec §11.8).
#[derive(Clone, Debug, PartialEq)]
pub struct PackageEvent {
    /// `@ns/name:version`.
    pub package: String,
    /// `needed` (offline: the client should ask the user), `fetching`,
    /// `ready` (compile again), `failed`.
    pub event: &'static str,
    pub message: Option<String>,
    /// `ready`: the tarball's SHA-256 (hex) and size.
    pub sha256: Option<String>,
    pub bytes: Option<u64>,
}

impl PackageEvent {
    pub fn to_json(&self) -> Json {
        let mut kv = vec![
            ("package".to_string(), Json::Str(self.package.clone())),
            ("event".into(), Json::Str(self.event.into())),
        ];
        if let Some(m) = &self.message {
            kv.push(("message".into(), Json::Str(m.clone())));
        }
        if let Some(s) = &self.sha256 {
            kv.push(("sha256".into(), Json::Str(s.clone())));
        }
        if let Some(b) = self.bytes {
            kv.push(("bytes".into(), Json::Int(b as i64)));
        }
        Json::Obj(kv)
    }
}

type Listener = Box<dyn Fn(PackageEvent) + Send + Sync>;

/// A fetch, per package and project (a lock mismatch is one project's).
enum Fetch {
    /// `attempts` earlier fetches of it failed.
    Running { attempts: u32 },
    /// The last fetch failed at `at`; until [`backoff`]`(attempts)` has
    /// passed, a compile that needs it reports `message` without fetching.
    Failed {
        message: String,
        at: Instant,
        attempts: u32,
    },
}

/// How long after `attempts` failed fetches a compile fetches again: 5 s,
/// doubling, at most 5 minutes.
pub fn backoff(attempts: u32) -> Duration {
    Duration::from_secs(
        5u64.saturating_mul(1 << attempts.saturating_sub(1).min(6))
            .min(300),
    )
}

type FetchKey = (PackageSpec, PathBuf);

/// A file's identity for the hash caches: device, inode, length, mtime.
type Stamp = (u64, u64, u64, std::time::SystemTime);

fn stamp(path: &Path) -> Result<Stamp, String> {
    let md = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(stamp_of(&md))
}

fn stamp_of(md: &std::fs::Metadata) -> Stamp {
    use std::os::unix::fs::MetadataExt;
    (
        md.dev(),
        md.ino(),
        md.len(),
        md.modified().unwrap_or(std::time::UNIX_EPOCH),
    )
}

/// The host's packages: shared by every document the process serves.
pub struct Packages {
    opts: PackageOptions,
    fetches: Mutex<HashMap<FetchKey, Fetch>>,
    done: Condvar,
    listener: Mutex<Option<Listener>>,
    /// Tarball path → (its stamp, SHA-256): each tarball hashed once.
    hashed: Mutex<HashMap<PathBuf, (Stamp, String)>>,
    /// Cache directories whose tree was checked against their tarball (of
    /// this SHA-256) in this process.
    verified: Mutex<HashMap<PathBuf, String>>,
}

/// How a compile treats the lock (`COMPILE.lock`, spec §11.8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LockMode {
    /// Check what the lock has; record what it lacks.
    #[default]
    Record,
    /// Re-record the fonts as they are now (accept changed fonts); packages
    /// are still checked (a package hash is never replaced).
    Update,
    /// Neither read nor write the lock.
    Off,
}

/// The project's lock, shared by the world and its fetch threads.
pub struct ProjectLock {
    root: PathBuf,
    state: Mutex<LockState>,
}

#[derive(Default)]
struct LockState {
    lock: Lock,
    /// The file's stamp as last read or written.
    seen: Option<Stamp>,
    /// The lock could not be read: reported, and never overwritten.
    error: Option<String>,
    loaded: bool,
    mode: LockMode,
}

impl ProjectLock {
    pub fn new(root: &Path) -> Arc<ProjectLock> {
        let l = Arc::new(ProjectLock {
            root: root.to_path_buf(),
            state: Mutex::new(LockState::default()),
        });
        l.refresh();
        l
    }

    pub fn path(&self) -> PathBuf {
        self.root.join(LOCK_FILE)
    }

    pub fn set_mode(&self, mode: LockMode) {
        self.state.lock().unwrap().mode = mode;
    }

    pub fn mode(&self) -> LockMode {
        self.state.lock().unwrap().mode
    }

    fn file_stamp(&self) -> Option<Stamp> {
        use std::os::unix::fs::MetadataExt;
        std::fs::symlink_metadata(self.path()).ok().map(|m| {
            (
                m.dev(),
                m.ino(),
                m.len(),
                m.modified().unwrap_or(std::time::UNIX_EPOCH),
            )
        })
    }

    /// Re-read the lock if the file changed since it was last read or
    /// written (the user, or another host, may have changed it).
    pub fn refresh(&self) {
        let md = self.file_stamp();
        let mut st = self.state.lock().unwrap();
        if st.loaded && md == st.seen {
            return;
        }
        st.loaded = true;
        st.seen = md;
        match Lock::read(&self.root) {
            Ok(l) => {
                st.lock = l.unwrap_or_default();
                st.error = None;
            }
            Err(e) => {
                st.lock = Lock::default();
                st.error = Some(e);
            }
        }
    }

    /// The lock as last read (for checks), and its read error, if any.
    pub fn snapshot(&self) -> (Lock, Option<String>) {
        let st = self.state.lock().unwrap();
        (st.lock.clone(), st.error.clone())
    }

    fn package(&self, key: &str) -> Option<String> {
        self.state.lock().unwrap().lock.packages.get(key).cloned()
    }

    /// Change the lock with `f` and write it ([`crate::lock::update`]:
    /// locked, re-read, atomic), unless the lock is off. An unreadable lock
    /// is never overwritten.
    pub fn update(&self, f: impl FnOnce(&mut Lock) -> bool) -> Result<(), String> {
        let mut st = self.state.lock().unwrap();
        if st.mode == LockMode::Off {
            return Ok(());
        }
        let force = st.mode == LockMode::Update;
        match crate::lock::update(&self.root, force, f) {
            Ok((l, written)) => {
                st.lock = l;
                st.error = None;
                match written {
                    // The file we renamed into place: anyone's later change
                    // is another stamp, so the next refresh re-reads.
                    Some(md) => {
                        st.loaded = true;
                        st.seen = Some(stamp_of(&md));
                    }
                    // Nothing written: re-read at the next refresh.
                    None => st.loaded = false,
                }
                Ok(())
            }
            Err(e) => {
                st.error = Some(e.clone());
                Err(format!("{e}; it was left as it is"))
            }
        }
    }
}

/// `@ns/name:version`.
pub fn spec_key(spec: &PackageSpec) -> String {
    format!("@{}/{}:{}", spec.namespace, spec.name, spec.version)
}

fn err(msg: String) -> FileError {
    FileError::Package(PackageError::Other(Some(msg.into())))
}

impl Packages {
    pub fn new(opts: PackageOptions) -> Packages {
        Packages {
            opts,
            fetches: Mutex::new(HashMap::new()),
            done: Condvar::new(),
            listener: Mutex::new(None),
            hashed: Mutex::new(HashMap::new()),
            verified: Mutex::new(HashMap::new()),
        }
    }

    pub fn options(&self) -> &PackageOptions {
        &self.opts
    }

    /// Where `PACKAGE` events go (the current connection), or nowhere.
    pub fn set_listener(&self, l: Option<Listener>) {
        *self.listener.lock().unwrap() = l;
    }

    fn emit(&self, ev: PackageEvent) {
        if let Some(l) = &*self.listener.lock().unwrap() {
            l(ev);
        }
    }

    /// The HELLO's `typst.packages` description.
    pub fn describe(&self) -> Json {
        let p = |o: &Option<PathBuf>| {
            o.as_ref()
                .map(|p| Json::Str(p.to_string_lossy().into_owned()))
                .unwrap_or(Json::Null)
        };
        Json::Obj(vec![
            ("lock".into(), Json::Str(LOCK_FILE.into())),
            ("vendor".into(), Json::Str(VENDOR_DIR.into())),
            ("cache".into(), p(&self.opts.cache)),
            ("mirror".into(), Json::Str(self.opts.mirror.clone())),
            ("offline".into(), Json::Bool(self.opts.offline)),
        ])
    }

    /// The canonical root of `spec`'s files, fetching it in the background
    /// when allowed (see the module documentation). `say_needed`: emit
    /// `needed` when offline (once per compile: the world asks for each
    /// package's files separately).
    pub fn resolve(
        self: &Arc<Self>,
        spec: &PackageSpec,
        project: &Path,
        lock: &Arc<ProjectLock>,
        online: bool,
        say_needed: bool,
    ) -> FileResult<PathBuf> {
        let rel = format!("{}/{}/{}", spec.namespace, spec.name, spec.version);
        let key = spec_key(spec);

        // 1. Vendored in the project (confined to the project root).
        let vendored = Path::new(VENDOR_DIR).join(&rel);
        if project.join(&vendored).is_dir() {
            return crate::world::confine(project, &vendored).map_err(err);
        }
        // 2. The host's read-only package paths.
        for dir in &self.opts.paths {
            let d = dir.join(&rel);
            if d.is_dir() {
                return d.canonicalize().map_err(|e| err(format!("{key}: {e}")));
            }
        }
        // 3. The cache: its tarball checked against the lock, its tree
        //    against the tarball.
        if let Some(cache) = &self.opts.cache {
            let dir = cache.join(&rel);
            let tarball = tarball_path(cache, spec);
            if dir.is_dir() && tarball.is_file() {
                let sha = self.tarball_sha(&tarball).map_err(err)?;
                self.check_lock(&key, &sha, lock).map_err(err)?;
                self.verify_cached(spec, cache, &tarball, &sha)
                    .map_err(err)?;
                return dir.canonicalize().map_err(|e| err(format!("{key}: {e}")));
            }
        }
        if spec.namespace != UNIVERSE {
            return Err(err(format!(
                "{key} is not installed: only @{UNIVERSE} packages are downloaded; \
                 put it in the project as {VENDOR_DIR}/{rel}/"
            )));
        }
        // 4. The network, with consent.
        if self.opts.offline || !online {
            if say_needed {
                self.emit(PackageEvent {
                    package: key.clone(),
                    event: "needed",
                    message: None,
                    sha256: None,
                    bytes: None,
                });
            }
            return Err(err(format!(
                "{key} is not downloaded and FlashTeX is offline: allow package \
                 downloads (packages.typst.org) and compile again, or vendor it into \
                 {VENDOR_DIR}/{rel}/"
            )));
        }
        let Some(cache) = self.opts.cache.clone() else {
            return Err(err(format!(
                "{key} cannot be downloaded: this host has no package cache (--package-cache)"
            )));
        };
        let fk: FetchKey = (spec.clone(), project.to_path_buf());
        let downloading = || {
            err(format!(
                "downloading {key} from {} …; the document compiles again when it arrives",
                self.opts.mirror
            ))
        };
        let mut fetches = self.fetches.lock().unwrap();
        let attempts = match fetches.get(&fk) {
            // Another compile started it: only that one waits.
            Some(Fetch::Running { .. }) => return Err(downloading()),
            Some(Fetch::Failed {
                message,
                at,
                attempts,
            }) => {
                let wait = backoff(*attempts);
                let since = at.elapsed();
                if since < wait {
                    return Err(err(format!(
                        "{message} (FlashTeX tries again in {} s)",
                        (wait - since).as_secs().max(1)
                    )));
                }
                *attempts
            }
            None => 0,
        };
        fetches.insert(fk.clone(), Fetch::Running { attempts });
        let me = Arc::clone(self);
        let (fk2, lock2, cache2) = (fk.clone(), Arc::clone(lock), cache.clone());
        std::thread::spawn(move || me.fetch(fk2, cache2, lock2));
        // The compile that started the fetch gives a fast one a moment, so
        // it needs no second compile.
        let (fetches, _) = self
            .done
            .wait_timeout_while(fetches, WAIT, |f| {
                matches!(f.get(&fk), Some(Fetch::Running { .. }))
            })
            .unwrap();
        match fetches.get(&fk) {
            Some(Fetch::Running { .. }) => Err(downloading()),
            Some(Fetch::Failed { message, .. }) => Err(err(message.clone())),
            None => {
                drop(fetches);
                // Done: unpacked from a checked tarball and verified.
                cache
                    .join(&rel)
                    .canonicalize()
                    .map_err(|e| err(format!("{key}: {e}")))
            }
        }
    }

    /// The fetch thread: download, check against the lock, keep the
    /// tarball, unpack atomically into the cache, verify the tree, record
    /// the lock entry.
    fn fetch(self: Arc<Self>, fk: FetchKey, cache: PathBuf, lock: Arc<ProjectLock>) {
        let key = spec_key(&fk.0);
        self.emit(PackageEvent {
            package: key.clone(),
            event: "fetching",
            message: None,
            sha256: None,
            bytes: None,
        });
        let r = self.fetch_inner(&fk.0, &cache, &lock);
        let mut fetches = self.fetches.lock().unwrap();
        let attempts = match fetches.get(&fk) {
            Some(Fetch::Running { attempts }) => *attempts,
            _ => 0,
        };
        let ev = match r {
            Ok((sha, bytes)) => {
                fetches.remove(&fk);
                PackageEvent {
                    package: key,
                    event: "ready",
                    message: None,
                    sha256: Some(sha),
                    bytes: Some(bytes),
                }
            }
            Err(m) => {
                fetches.insert(
                    fk,
                    Fetch::Failed {
                        message: m.clone(),
                        at: Instant::now(),
                        attempts: attempts + 1,
                    },
                );
                PackageEvent {
                    package: key,
                    event: "failed",
                    message: Some(m),
                    sha256: None,
                    bytes: None,
                }
            }
        };
        drop(fetches);
        self.done.notify_all();
        self.emit(ev);
    }

    fn fetch_inner(
        &self,
        spec: &PackageSpec,
        cache: &Path,
        lock: &Arc<ProjectLock>,
    ) -> Result<(String, u64), String> {
        let key = spec_key(spec);
        let got: Arc<Mutex<Option<(String, u64)>>> = Arc::default();
        let tarball = tarball_path(cache, spec);
        let dl = LockedDownloader {
            curl: self.opts.curl.clone(),
            mirror: self.opts.mirror.clone(),
            lock: Arc::clone(lock),
            tarball: tarball.clone(),
            got: Arc::clone(&got),
        };
        let universe = UniversePackages::with_url(dl, self.opts.mirror.clone());
        let mut archive = universe.package(spec).map_err(|e| format!("{key}: {e}"))?;
        // A tree left by an earlier, failed unpack would win the store's
        // rename; it is replaced below if it does not match.
        FsPackages::new(cache)
            .store(spec, |tmp| {
                archive
                    .unpack(tmp)
                    .map_err(|e| PackageError::MalformedArchive(Some(e.to_string().into())))
            })
            .map_err(|e| format!("{key}: {e}"))?;
        let (sha, bytes) = got
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| format!("{key}: the download was not recorded"))?;
        self.verify_cached(spec, cache, &tarball, &sha)?;
        self.check_lock(&key, &sha, lock)?;
        Ok((sha, bytes))
    }

    /// Check a cached package's unpacked tree against its tarball (whose
    /// SHA-256 is `sha`, already checked against the lock), once per
    /// process: every file and symlink of the tarball is in the tree with
    /// the same bytes, and the tree has nothing else. A tree that differs
    /// (edited, half unpacked, added to) is replaced by a fresh unpack of
    /// the tarball, which is then checked again.
    fn verify_cached(
        &self,
        spec: &PackageSpec,
        cache: &Path,
        tarball: &Path,
        sha: &str,
    ) -> Result<(), String> {
        let key = spec_key(spec);
        let dir = cache.join(format!("{}/{}/{}", spec.namespace, spec.name, spec.version));
        if self.verified.lock().unwrap().get(&dir).map(String::as_str) == Some(sha) {
            return Ok(());
        }
        let data = std::fs::read(tarball).map_err(|e| format!("{key}: {e}"))?;
        if hex(&sha256(&data)) != sha {
            return Err(format!("{key}: its cached tarball changed while in use"));
        }
        if let Err(why) = tree_matches(&data, &dir) {
            // Move the bad tree aside (rename is atomic for readers), drop
            // it, and unpack the verified tarball again.
            let aside = dir.with_file_name(format!(".bad-{}-{}", spec.version, std::process::id()));
            let _ = std::fs::remove_dir_all(&aside);
            std::fs::rename(&dir, &aside).map_err(|e| format!("{key}: {e}"))?;
            let _ = std::fs::remove_dir_all(&aside);
            FsPackages::new(cache)
                .store(spec, |tmp| {
                    tar::Archive::new(flate2::read::GzDecoder::new(&data[..]))
                        .unpack(tmp)
                        .map_err(|e| PackageError::MalformedArchive(Some(e.to_string().into())))
                })
                .map_err(|e| format!("{key}: {e}"))?;
            tree_matches(&data, &dir).map_err(|again| {
                format!("{key}: the cached copy differed from its tarball ({why}) and could not be restored ({again})")
            })?;
        }
        self.verified.lock().unwrap().insert(dir, sha.to_string());
        Ok(())
    }

    /// Check `sha` against the lock's entry for `key`; record it when the
    /// lock has none (the project's first fetch of it).
    fn check_lock(&self, key: &str, sha: &str, lock: &Arc<ProjectLock>) -> Result<(), String> {
        if lock.mode() == LockMode::Off {
            return Ok(());
        }
        match lock.package(key) {
            Some(want) if want == sha => Ok(()),
            Some(want) => Err(mismatch(key, &want, sha)),
            None => {
                let mut clash = None;
                let r = lock.update(|l| match l.packages.get(key) {
                    // Another host recorded it meanwhile: that entry rules.
                    Some(want) if want != sha => {
                        clash = Some(want.clone());
                        false
                    }
                    Some(_) => false,
                    None => {
                        l.packages.insert(key.to_string(), sha.to_string());
                        true
                    }
                });
                r.map_err(|e| format!("{key}: could not record it in {LOCK_FILE}: {e}"))?;
                match clash {
                    Some(want) => Err(mismatch(key, &want, sha)),
                    None => Ok(()),
                }
            }
        }
    }

    fn tarball_sha(&self, path: &Path) -> Result<String, String> {
        let st = stamp(path)?;
        if let Some((s0, sha)) = self.hashed.lock().unwrap().get(path) {
            if *s0 == st {
                return Ok(sha.clone());
            }
        }
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let sha = hex(&sha256(&data));
        self.hashed
            .lock()
            .unwrap()
            .insert(path.to_path_buf(), (st, sha.clone()));
        Ok(sha)
    }
}

/// A tarball entry's path inside the package: its normal components only;
/// `None` for one that would leave the package (`..`, absolute).
fn entry_path(p: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::Normal(n) => out.push(n),
            std::path::Component::CurDir => {}
            _ => return None,
        }
    }
    Some(out)
}

/// Whether the tree at `dir` is exactly what the gzipped tarball `data`
/// unpacks to (regular files by content, symlinks by target, nothing more).
pub fn tree_matches(data: &[u8], dir: &Path) -> Result<(), String> {
    use std::collections::HashSet;
    use tar::EntryType;
    let mut ar = tar::Archive::new(flate2::read::GzDecoder::new(data));
    let mut expected: HashSet<PathBuf> = HashSet::new();
    for e in ar.entries().map_err(|e| e.to_string())? {
        let mut e = e.map_err(|e| e.to_string())?;
        let raw = e.path().map_err(|e| e.to_string())?.into_owned();
        let Some(rel) = entry_path(&raw) else {
            continue; // tar's unpack skips these too
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        let at = dir.join(&rel);
        let md =
            std::fs::symlink_metadata(&at).map_err(|_| format!("{} is missing", rel.display()))?;
        match e.header().entry_type() {
            EntryType::Regular | EntryType::Continuous => {
                if !md.file_type().is_file() {
                    return Err(format!("{} is not a file", rel.display()));
                }
                let mut want = Vec::with_capacity(e.size() as usize);
                e.read_to_end(&mut want).map_err(|e| e.to_string())?;
                let have = std::fs::read(&at).map_err(|e| e.to_string())?;
                if have != want {
                    return Err(format!("{} differs", rel.display()));
                }
            }
            EntryType::Directory => {
                if !md.file_type().is_dir() {
                    return Err(format!("{} is not a directory", rel.display()));
                }
            }
            EntryType::Symlink => {
                let want = e
                    .link_name()
                    .map_err(|e| e.to_string())?
                    .map(|l| l.into_owned());
                let have = std::fs::read_link(&at).ok();
                if !md.file_type().is_symlink() || have != want {
                    return Err(format!("{} is not the archive's symlink", rel.display()));
                }
            }
            _ => continue,
        }
        expected.insert(rel);
    }
    // Nothing more than the archive has.
    fn walk(
        base: &Path,
        at: &Path,
        expected: &std::collections::HashSet<PathBuf>,
    ) -> Result<(), String> {
        for ent in std::fs::read_dir(at).map_err(|e| e.to_string())? {
            let ent = ent.map_err(|e| e.to_string())?;
            let p = ent.path();
            let rel = p.strip_prefix(base).unwrap().to_path_buf();
            let ft = ent.file_type().map_err(|e| e.to_string())?;
            if ft.is_dir() {
                walk(base, &p, expected)?;
            } else if !expected.contains(&rel) {
                return Err(format!("{} is not in the archive", rel.display()));
            }
        }
        Ok(())
    }
    walk(dir, dir, &expected)
}

fn mismatch(key: &str, want: &str, got: &str) -> String {
    format!(
        "{key} does not match {LOCK_FILE}: its tarball's SHA-256 is {got}, the lock says {want}. \
         The package changed since this project first used it; FlashTeX does not use it. \
         If you trust the new version, delete its line from {LOCK_FILE}"
    )
}

fn tarball_path(cache: &Path, spec: &PackageSpec) -> PathBuf {
    cache
        .join(".tarballs")
        .join(spec.namespace.as_str())
        .join(format!("{}-{}.tar.gz", spec.name, spec.version))
}

/// typst-kit's `Downloader`, over `curl`, that checks a package tarball
/// against the lock before typst-kit unpacks it and keeps the tarball.
struct LockedDownloader {
    curl: PathBuf,
    mirror: String,
    lock: Arc<ProjectLock>,
    tarball: PathBuf,
    got: Arc<Mutex<Option<(String, u64)>>>,
}

impl Downloader for LockedDownloader {
    fn stream(&self, key: &dyn Any, url: &str) -> io::Result<(Option<usize>, Box<dyn Read>)> {
        let data = self.download(key, url)?;
        Ok((Some(data.len()), Box::new(io::Cursor::new(data))))
    }

    fn download(&self, key: &dyn Any, url: &str) -> io::Result<Vec<u8>> {
        let data = curl_get(&self.curl, &self.mirror, url)?;
        if let Some(spec) = key.downcast_ref::<PackageSpec>() {
            let k = spec_key(spec);
            let sha = hex(&sha256(&data));
            if self.lock.mode() != LockMode::Off {
                if let Some(want) = self.lock.package(&k) {
                    if want != sha {
                        return Err(io::Error::other(mismatch(&k, &want, &sha)));
                    }
                }
            }
            // Keep the tarball (atomically) so later uses can be checked.
            if let Some(dir) = self.tarball.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let tmp = self
                .tarball
                .with_extension(format!("tmp-{}", std::process::id()));
            std::fs::write(&tmp, &data)?;
            std::fs::rename(&tmp, &self.tarball)?;
            *self.got.lock().unwrap() = Some((sha, data.len() as u64));
        }
        Ok(data)
    }
}

/// The largest download accepted (a Universe package is a few MB).
pub const MAX_DOWNLOAD: u64 = 64 << 20;

/// The absolute path of the system `curl`, resolved once at start:
/// `/usr/bin/curl` on macOS (part of the OS); elsewhere the first `curl`
/// on `PATH` (Windows 10+ ships `curl.exe`), else plain `curl`.
pub fn system_curl() -> PathBuf {
    if cfg!(target_os = "macos") && Path::new("/usr/bin/curl").is_file() {
        return PathBuf::from("/usr/bin/curl");
    }
    let name = if cfg!(windows) { "curl.exe" } else { "curl" };
    std::env::var_os("PATH")
        .and_then(|p| {
            std::env::split_paths(&p)
                .map(|d| d.join(name))
                .find(|c| c.is_absolute() && c.is_file())
        })
        .unwrap_or_else(|| PathBuf::from(name))
}

/// GET `url` with the system `curl`, restricted to the mirror's scheme,
/// ignoring the user's `.curlrc` (`-q`), at most [`MAX_DOWNLOAD`] bytes
/// (enforced while reading: `--max-filesize` cannot cap a chunked body).
/// A 404 (or a missing `file://` file) is `NotFound`.
fn curl_get(curl: &Path, mirror: &str, url: &str) -> io::Result<Vec<u8>> {
    let scheme = mirror.split("://").next().unwrap_or("https");
    if !matches!(scheme, "https" | "http" | "file") || !url.starts_with(&format!("{scheme}://")) {
        return Err(io::Error::other(format!(
            "refusing to fetch {url}: not under the mirror's scheme"
        )));
    }
    let proto = format!("={scheme}");
    let max = MAX_DOWNLOAD.to_string();
    let mut child = Command::new(curl)
        .args([
            // First: do not read ~/.curlrc (it could add options or a proxy).
            "-q",
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--max-redirs",
            "5",
            "--proto",
            &proto,
            "--proto-redir",
            &proto,
            "--connect-timeout",
            "20",
            "--max-time",
            "300",
            "--max-filesize",
            &max,
            "--user-agent",
            &user_agent(),
            "--",
            url,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| io::Error::other(format!("cannot run {}: {e}", curl.display())))?;
    let mut data = Vec::new();
    let read = child
        .stdout
        .take()
        .expect("piped")
        .take(MAX_DOWNLOAD + 1)
        .read_to_end(&mut data);
    if read.is_err() || data.len() as u64 > MAX_DOWNLOAD {
        let _ = child.kill();
        let _ = child.wait();
        return Err(io::Error::other(match read {
            Err(e) => format!("reading {url}: {e}"),
            Ok(_) => format!("{url} is larger than {MAX_DOWNLOAD} bytes"),
        }));
    }
    let mut stderr = String::new();
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_string(&mut stderr);
    }
    let status = child.wait()?;
    if status.success() {
        return Ok(data);
    }
    let msg = stderr.trim().to_string();
    // curl: 22 = HTTP error (with --fail), 37 = a file:// file not readable.
    let code = status.code();
    if (code == Some(22) && msg.contains("404")) || code == Some(37) {
        return Err(io::Error::new(io::ErrorKind::NotFound, msg));
    }
    Err(io::Error::other(if msg.is_empty() {
        format!("curl exited with {:?}", code)
    } else {
        msg
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The size cap holds while reading (a sparse file over a `file://`
    /// "mirror"), and `curl` is an absolute path.
    #[test]
    fn downloads_are_capped_while_reading() {
        let curl = system_curl();
        assert!(curl.is_absolute(), "{}", curl.display());
        let dir = std::env::temp_dir().join(format!("ftth-cap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let big = dir.join("big.tar.gz");
        std::fs::File::create(&big)
            .unwrap()
            .set_len(MAX_DOWNLOAD + 4096)
            .unwrap();
        let small = dir.join("small.tar.gz");
        std::fs::write(&small, b"abc").unwrap();
        let mirror = format!("file://{}", dir.display());
        let e = curl_get(&curl, &mirror, &format!("{mirror}/big.tar.gz")).unwrap_err();
        // A known length trips curl's own --max-filesize; a chunked body
        // (no length) trips the count while reading. Either way: refused.
        let m = e.to_string();
        assert!(
            m.contains("larger than") || m.contains("maximum allowed file size"),
            "{m}"
        );
        assert_eq!(
            curl_get(&curl, &mirror, &format!("{mirror}/small.tar.gz")).unwrap(),
            b"abc"
        );
        let e = curl_get(&curl, &mirror, &format!("{mirror}/missing.tar.gz")).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::NotFound);
        assert!(curl_get(&curl, &mirror, "https://example.invalid/x").is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn backoff_doubles_and_is_capped() {
        assert_eq!(backoff(1), Duration::from_secs(5));
        assert_eq!(backoff(2), Duration::from_secs(10));
        assert_eq!(backoff(4), Duration::from_secs(40));
        assert_eq!(backoff(30), Duration::from_secs(300));
    }

    /// Two hosts (separate locks, separate descriptors) recording entries at
    /// once lose none: each update is locked, re-read and renamed in place.
    #[test]
    fn concurrent_lock_writers_lose_nothing() {
        let dir = std::env::temp_dir().join(format!("ftth-lockrace-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let root = dir.canonicalize().unwrap();
        let threads: Vec<_> = (0..4)
            .map(|t| {
                let root = root.clone();
                std::thread::spawn(move || {
                    let l = ProjectLock::new(&root);
                    for i in 0..25 {
                        l.update(|lock| {
                            lock.packages
                                .insert(format!("@preview/p{t}x{i}:1.0.0"), "ab".repeat(32));
                            true
                        })
                        .unwrap();
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        let lock = Lock::read(&root).unwrap().unwrap();
        assert_eq!(lock.packages.len(), 100);
        // No temporary file is left behind.
        let left: Vec<_> = std::fs::read_dir(&root)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left.len(), 1, "{left:?}");
        std::fs::remove_dir_all(&root).unwrap();
    }
}
