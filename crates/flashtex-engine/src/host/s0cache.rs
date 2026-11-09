//! Pruning the S₀/S₁ cache (`--s0-cache`, lane COLD-OPEN).
//!
//! The resident host stores each document's S₀ (`HASH.s0`, tens of
//! megabytes) and S₁ (`HASH.s1`) and nothing removed them, so the cache grew
//! with every document ever opened. A host now prunes it once, in the
//! background, after it has opened its first document:
//!
//! * **What.** Only the cache's own names: 32 lowercase hex digits and
//!   `.s0` or `.s1`. A document's pair is one entry, last used when the
//!   newer of the two was written or touched; nothing else in the directory
//!   is looked at (a writer's temporary file is `host::S0Image::write`'s).
//! * **Order and cap.** Least recently used first, until the entries'
//!   bytes on disk are within the performance mode's cap ([`cap`]: Low
//!   Memory 512 MB, Balanced 2 GB, High Performance 8 GB;
//!   `FLASHTEX_S0_CACHE_CAP=BYTES` for the tests).
//! * **Never.** A file of a document this process has open ([`hold`]: the
//!   current project, held before its stored S₀ is looked at), nor one
//!   written or touched in the last ten minutes (another host's open
//!   project, or a save just made). The check and the removal are made under
//!   the lock [`hold`] takes, so a document opened meanwhile is held first or
//!   finds its files gone (and compiles from the format, as without a cache).
//! * **Last use.** A host that opens a document touches its stored files
//!   ([`hold`]), so a document reopened every day stays however old its S₀.

use crate::profile::Mode;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, SystemTime};

/// A file written or touched this recently is never pruned.
const RECENT: Duration = Duration::from_secs(600);

/// How long the pruning waits for the host's first document.
const WAIT_FOR_DOCUMENT: Duration = Duration::from_secs(120);

/// The files of documents this process has open, and whether one was held.
static HELD: Mutex<(Vec<PathBuf>, bool)> = Mutex::new((Vec::new(), false));
static FIRST: Condvar = Condvar::new();

/// The cache's bytes on disk the performance mode allows.
pub fn cap(mode: Mode) -> u64 {
    if let Some(b) = std::env::var("FLASHTEX_S0_CACHE_CAP")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        return b;
    }
    const MB: u64 = 1 << 20;
    match mode {
        Mode::LowMemory => 512 * MB,
        Mode::Balanced => 2048 * MB,
        Mode::HighPerformance => 8192 * MB,
    }
}

/// The document whose stored files are `paths` (its S₀'s and S₁'s) is open
/// in this process: they are never pruned, and are touched (their last use).
pub fn hold(paths: &[&Path]) {
    let mut h = HELD.lock().unwrap_or_else(|p| p.into_inner());
    for p in paths {
        if let Ok(f) = std::fs::File::options().write(true).open(p) {
            let _ = f.set_modified(SystemTime::now());
        }
        if !h.0.iter().any(|x| x.as_path() == *p) {
            h.0.push(p.to_path_buf());
        }
    }
    h.1 = true;
    FIRST.notify_all();
}

/// The cache's name for `p`: its document's hash, if it is `HASH.s0` or
/// `HASH.s1`.
fn entry_of(p: &Path) -> Option<String> {
    let ext = p.extension()?.to_str()?;
    let stem = p.file_stem()?.to_str()?;
    let ok = matches!(ext, "s0" | "s1")
        && stem.len() == 32
        && stem.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    ok.then(|| stem.to_string())
}

fn on_disk(m: &std::fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        std::os::unix::fs::MetadataExt::blocks(m) * 512
    }
    #[cfg(not(unix))]
    {
        m.len()
    }
}

/// What a pruning removed: entries, files, bytes on disk.
#[derive(Debug, Default, PartialEq)]
pub struct Pruned {
    pub entries: usize,
    pub files: usize,
    pub bytes: u64,
}

/// Prune `dir` to `cap` bytes on disk (see the module's documentation).
pub fn prune(dir: &Path, cap: u64) -> Pruned {
    let now = SystemTime::now();
    let mut entries: HashMap<String, (SystemTime, u64, Vec<PathBuf>)> = HashMap::new();
    let Ok(list) = std::fs::read_dir(dir) else {
        return Pruned::default();
    };
    for e in list.flatten() {
        let p = e.path();
        let Some(k) = entry_of(&p) else {
            continue;
        };
        let Ok(m) = std::fs::symlink_metadata(&p) else {
            continue;
        };
        if !m.file_type().is_file() {
            continue;
        }
        let t = m.modified().unwrap_or(now);
        let x = entries.entry(k).or_insert((t, 0, vec![]));
        x.0 = x.0.max(t);
        x.1 += on_disk(&m);
        x.2.push(p);
    }
    let mut total: u64 = entries.values().map(|e| e.1).sum();
    let mut order: Vec<(SystemTime, u64, Vec<PathBuf>)> = entries.into_values().collect();
    order.sort_by_key(|e| e.0);
    let mut r = Pruned::default();
    for (last, bytes, files) in order {
        if total <= cap {
            break;
        }
        if now.duration_since(last).unwrap_or_default() < RECENT {
            continue;
        }
        let h = HELD.lock().unwrap_or_else(|p| p.into_inner());
        if files.iter().any(|f| h.0.contains(f)) {
            continue;
        }
        let mut removed = 0;
        for f in &files {
            if std::fs::remove_file(f).is_ok() {
                removed += 1;
            }
        }
        drop(h);
        if removed > 0 {
            r.entries += 1;
            r.files += removed;
            r.bytes += bytes;
            total = total.saturating_sub(bytes);
        }
    }
    r
}

/// Prune `dir` on a thread of its own once this process has opened its
/// first document (or after two minutes without one), and say what went.
pub fn prune_in_background(dir: PathBuf, cap: u64) {
    let _ = std::thread::Builder::new()
        .name("s0-prune".into())
        .spawn(move || {
            {
                let h = HELD.lock().unwrap_or_else(|p| p.into_inner());
                let _ = FIRST
                    .wait_timeout_while(h, WAIT_FOR_DOCUMENT, |h| !h.1)
                    .unwrap_or_else(|p| p.into_inner());
            }
            let r = prune(&dir, cap);
            if r.files > 0 {
                super::server::say(&format!(
                    "flashtex-host: {{\"pruned_s0\":{},\"files\":{},\"bytes\":{}}}",
                    r.entries, r.files, r.bytes
                ));
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("flashtex-s0cache-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn file(d: &Path, name: &str, kb: usize, age_s: u64) -> PathBuf {
        let p = d.join(name);
        std::fs::write(&p, vec![1u8; kb << 10]).unwrap();
        std::fs::File::options()
            .write(true)
            .open(&p)
            .unwrap()
            .set_modified(SystemTime::now() - Duration::from_secs(age_s))
            .unwrap();
        p
    }

    /// Oldest entries first, a pair together, to the cap; recent, held and
    /// foreign files stay.
    #[test]
    fn prunes_the_least_recently_used_to_the_cap() {
        let d = dir("lru");
        let h = |c: char| c.to_string().repeat(32);
        let day = 86_400;
        let a = file(&d, &format!("{}.s0", h('a')), 100, 5 * day);
        // (a pair: last used when its newer file was)
        let b0 = file(&d, &format!("{}.s0", h('b')), 100, 6 * day);
        let b1 = file(&d, &format!("{}.s1", h('b')), 100, 4 * day);
        let held = file(&d, &format!("{}.s0", h('c')), 100, 7 * day);
        let recent = file(&d, &format!("{}.s0", h('d')), 100, 60);
        let kept = file(&d, &format!("{}.s0", h('e')), 100, 3 * day);
        let foreign = [
            file(&d, "x.s0", 100, 9 * day),
            file(&d, &format!("{}.s0", "g".repeat(32)), 100, 9 * day),
            file(&d, &format!("{}.s0.1.2.0.tmp", h('f')), 100, 9 * day),
        ];
        hold(&[&held]);
        // (touched by `hold`: put back old, to show the hold alone keeps it)
        std::fs::File::options()
            .write(true)
            .open(&held)
            .unwrap()
            .set_modified(SystemTime::now() - Duration::from_secs(7 * day))
            .unwrap();
        let size = |p: &Path| on_disk(&std::fs::metadata(p).unwrap());
        // the cap leaves room for the held, the recent and the newest only
        let cap = size(&held) + size(&recent) + size(&kept);
        let r = prune(&d, cap);
        assert!(!a.exists() && !b0.exists() && !b1.exists());
        assert!(held.exists() && recent.exists() && kept.exists());
        for f in &foreign {
            assert!(f.exists(), "{}", f.display());
        }
        assert_eq!((r.entries, r.files), (2, 3));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn caps_by_mode() {
        if std::env::var_os("FLASHTEX_S0_CACHE_CAP").is_some() {
            return;
        }
        assert_eq!(cap(Mode::LowMemory), 512 << 20);
        assert_eq!(cap(Mode::Balanced), 2 << 30);
        assert_eq!(cap(Mode::HighPerformance), 8 << 30);
    }
}
