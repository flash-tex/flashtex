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
//!    the project's lock (recorded there if the lock has no entry yet).
//! 4. **The network**, for `@preview` only, and only when the client's
//!    `COMPILE` says `"packages": "online"` (the app sends that after its
//!    first-use consent sheet) and the host was not started `--offline`.
//!    The fetch runs on its own thread, so a compile never waits for it
//!    longer than [`WAIT`]: the compile that needs the package fails at once
//!    with a located error ("downloading …"), and the host sends `PACKAGE`
//!    (spec §11.8) when the fetch has finished, so the client compiles again.
//!
//! **The lock** (`flashtex-typst.lock`, [`crate::lock`]): the tarball's
//! SHA-256 is recorded on the first fetch and checked on every later fetch
//! and every use of the cached copy; a mismatch is an error, never accepted.
//! typst-kit checks nothing but TLS and the Universe index carries no hash
//! (Track A §6, Track C §2.6).
//!
//! **Transport: the system `curl`.** typst-kit's `system-downloader` would
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
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

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
pub const KIND: u8 = 0x50;

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
    /// The `curl` program.
    pub curl: PathBuf,
}

impl Default for PackageOptions {
    fn default() -> Self {
        PackageOptions {
            cache: None,
            paths: vec![],
            mirror: DEFAULT_MIRROR.into(),
            offline: false,
            curl: PathBuf::from("curl"),
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

enum Fetch {
    Running,
    /// Finished with this error; the next compile that needs the package
    /// reports it once, and the one after retries.
    Failed(String),
}

/// The host's packages: shared by every document the process serves.
pub struct Packages {
    opts: PackageOptions,
    fetches: Mutex<HashMap<PackageSpec, Fetch>>,
    done: Condvar,
    listener: Mutex<Option<Listener>>,
    /// Tarball path → (length, mtime, SHA-256): each tarball hashed once.
    hashed: Mutex<HashMap<PathBuf, (u64, std::time::SystemTime, String)>>,
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
    /// (length, mtime) of the file as last read or written.
    seen: Option<(u64, std::time::SystemTime)>,
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

    /// Re-read the lock if the file changed since it was last read or
    /// written (the user may edit it, e.g. to drop an entry).
    pub fn refresh(&self) {
        let md = std::fs::symlink_metadata(self.path())
            .ok()
            .map(|m| (m.len(), m.modified().unwrap_or(std::time::UNIX_EPOCH)));
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

    /// Change the lock with `f` and write it (unless the lock is off or
    /// unreadable, which is never overwritten).
    pub fn update(&self, f: impl FnOnce(&mut Lock) -> bool) -> Result<(), String> {
        let mut st = self.state.lock().unwrap();
        if st.mode == LockMode::Off {
            return Ok(());
        }
        if let Some(e) = &st.error {
            return Err(format!("{e}; it was left as it is"));
        }
        if !f(&mut st.lock) {
            return Ok(());
        }
        st.lock.write(&self.root)?;
        st.seen = std::fs::symlink_metadata(self.path())
            .ok()
            .map(|m| (m.len(), m.modified().unwrap_or(std::time::UNIX_EPOCH)));
        Ok(())
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
    /// when allowed (see the module documentation).
    /// `say_needed`: emit `needed` when offline (once per compile: the
    /// world asks for each package's files separately).
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
        // 3. The cache, with its tarball checked against the lock.
        if let Some(cache) = &self.opts.cache {
            let dir = cache.join(&rel);
            let tarball = tarball_path(cache, spec);
            if dir.is_dir() && tarball.is_file() {
                let sha = self.tarball_sha(&tarball).map_err(err)?;
                self.check_lock(&key, &sha, lock).map_err(err)?;
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
        let mut fetches = self.fetches.lock().unwrap();
        match fetches.get(spec) {
            Some(Fetch::Failed(_)) => {
                // Report the failure once; the next compile retries.
                let Some(Fetch::Failed(m)) = fetches.remove(spec) else {
                    unreachable!()
                };
                return Err(err(m));
            }
            Some(Fetch::Running) => {}
            None => {
                fetches.insert(spec.clone(), Fetch::Running);
                let me = Arc::clone(self);
                let (spec2, lock2) = (spec.clone(), Arc::clone(lock));
                std::thread::spawn(move || me.fetch(spec2, cache, lock2));
            }
        }
        // Give a fast fetch a moment, so it needs no second compile.
        let (mut fetches, _) = self
            .done
            .wait_timeout_while(fetches, WAIT, |f| {
                matches!(f.get(spec), Some(Fetch::Running))
            })
            .unwrap();
        match fetches.get(spec) {
            Some(Fetch::Running) => Err(err(format!(
                "downloading {key} from {} …; the document compiles again when it arrives",
                self.opts.mirror
            ))),
            Some(Fetch::Failed(_)) => {
                let Some(Fetch::Failed(m)) = fetches.remove(spec) else {
                    unreachable!()
                };
                Err(err(m))
            }
            None => {
                drop(fetches);
                // Done: it is in the cache now (checked by the fetch).
                let dir = self
                    .opts
                    .cache
                    .as_ref()
                    .map(|c| c.join(&rel))
                    .ok_or_else(|| err(format!("{key}: no cache")))?;
                dir.canonicalize().map_err(|e| err(format!("{key}: {e}")))
            }
        }
    }

    /// The fetch thread: download, check against the lock, keep the
    /// tarball, unpack atomically into the cache, record the lock entry.
    fn fetch(self: Arc<Self>, spec: PackageSpec, cache: PathBuf, lock: Arc<ProjectLock>) {
        let key = spec_key(&spec);
        self.emit(PackageEvent {
            package: key.clone(),
            event: "fetching",
            message: None,
            sha256: None,
            bytes: None,
        });
        let r = self.fetch_inner(&spec, &cache, &lock);
        let mut fetches = self.fetches.lock().unwrap();
        let ev = match r {
            Ok((sha, bytes)) => {
                fetches.remove(&spec);
                PackageEvent {
                    package: key,
                    event: "ready",
                    message: None,
                    sha256: Some(sha),
                    bytes: Some(bytes),
                }
            }
            Err(m) => {
                fetches.insert(spec.clone(), Fetch::Failed(m.clone()));
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
        let dl = LockedDownloader {
            curl: self.opts.curl.clone(),
            mirror: self.opts.mirror.clone(),
            lock: Arc::clone(lock),
            tarball: tarball_path(cache, spec),
            got: Arc::clone(&got),
        };
        let universe = UniversePackages::with_url(dl, self.opts.mirror.clone());
        let mut archive = universe.package(spec).map_err(|e| format!("{key}: {e}"))?;
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
        self.check_lock(&key, &sha, lock)?;
        Ok((sha, bytes))
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
            None => lock
                .update(|l| {
                    l.packages.insert(key.to_string(), sha.to_string());
                    true
                })
                .map_err(|e| format!("{key}: could not record it in {LOCK_FILE}: {e}")),
        }
    }

    fn tarball_sha(&self, path: &Path) -> Result<String, String> {
        let md = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let stamp = (md.len(), md.modified().unwrap_or(std::time::UNIX_EPOCH));
        if let Some((l, m, s)) = self.hashed.lock().unwrap().get(path) {
            if (*l, *m) == stamp {
                return Ok(s.clone());
            }
        }
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let s = hex(&sha256(&data));
        self.hashed
            .lock()
            .unwrap()
            .insert(path.to_path_buf(), (stamp.0, stamp.1, s.clone()));
        Ok(s)
    }
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

use std::io::Read;

/// GET `url` with the system `curl`, restricted to the mirror's scheme.
/// A 404 (or a missing `file://` file) is `NotFound`.
fn curl_get(curl: &Path, mirror: &str, url: &str) -> io::Result<Vec<u8>> {
    let scheme = mirror.split("://").next().unwrap_or("https");
    if !matches!(scheme, "https" | "http" | "file") || !url.starts_with(&format!("{scheme}://")) {
        return Err(io::Error::other(format!(
            "refusing to fetch {url}: not under the mirror's scheme"
        )));
    }
    let proto = format!("={scheme}");
    let out = Command::new(curl)
        .args([
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
            "268435456",
            "--user-agent",
            &user_agent(),
            "--",
            url,
        ])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| io::Error::other(format!("cannot run {}: {e}", curl.display())))?;
    if out.status.success() {
        return Ok(out.stdout);
    }
    let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
    // curl: 22 = HTTP error (with --fail), 37 = a file:// file not readable.
    let code = out.status.code();
    if (code == Some(22) && msg.contains("404")) || code == Some(37) {
        return Err(io::Error::new(io::ErrorKind::NotFound, msg));
    }
    Err(io::Error::other(if msg.is_empty() {
        format!("curl exited with {:?}", code)
    } else {
        msg
    }))
}
