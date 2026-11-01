//! Content hashes of input files, kept for the process's lifetime (lane
//! COLD-FIXED).
//!
//! Every run hashes each file it opens (`system::note_file`: the read-set of
//! S₀'s key and of the incremental journal, DESIGN.md §5.1): the 15 MB
//! format, the classes and packages, the font files. A resident host
//! compiles the same document again and again, and a cold compile runs up
//! to three passes, so the same unchanged files were hashed again at every
//! pass, every preamble edit and every page that loads a font (23 M of a
//! preamble keystroke's 410 M instructions on a one-page article).
//!
//! A hash is kept per path with the file's identity when it was taken
//! (`os::file_stat`: size, modification and status-change times, inode,
//! device) and reused while the identity is the same. The status-change
//! time is in it because it cannot be set back: a file rewritten with its
//! size and modification time preserved (`cp -p`, `rsync -t`, `touch -r`)
//! still gets a new one.
//!
//! **Same-tick edits.** A file system keeps times at some granularity
//! (FAT 2 s, HFS+ 1 s, ext4 a kernel tick), so a file changed twice within
//! one tick, to the same length, keeps the same identity. As for
//! `system::StatSig::racy` (git's "racy clean" rule, kpathsea's `ls-R`
//! staleness), an identity whose modification or status-change time is
//! within `system::racy_ns()` (2 s) of now, either side, proves nothing: such
//! a file is hashed afresh every time and its hash is not kept. Once the tick
//! has passed, any further change moves the times.
//!
//! **A file changed while it is hashed.** The identity is taken before and
//! after the read; the hash is kept only when both are the same, so a kept
//! hash is always of the bytes the identity stands for.
//!
//! **Local volumes only.** A network file system's times come from another
//! machine's clock and its attributes from a cache, so another client's
//! rewrite may keep a signature here: a file on a volume that is not local
//! (`statfs`: macOS's `MNT_LOCAL`; on Linux not NFS, SMB/CIFS, FUSE, AFS,
//! Ceph, 9P, Coda) is hashed every time. The answer is kept per device.
//!
//! **Unix only.** Elsewhere `os::file_stat` has no inode and no
//! status-change time (Windows gives the creation time, which NTFS carries
//! over to a file re-created under the same name), so a rewrite to the same
//! size with its modification time put back would keep its identity: there
//! every file is hashed, as before.

use std::collections::HashMap;
use std::sync::Mutex;

/// What a kept hash stands for (`os::file_stat`'s fields).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Ident {
    size: u64,
    mtime_ns: i128,
    ctime_ns: i128,
    ino: u64,
    dev: u64,
}

impl Ident {
    fn of(path: &str) -> std::io::Result<Ident> {
        Ok(Ident::of_metadata(&std::fs::metadata(path)?))
    }

    fn of_metadata(m: &std::fs::Metadata) -> Ident {
        let s = crate::os::file_stat(m);
        Ident {
            size: s.size,
            mtime_ns: s.mtime_ns,
            ctime_ns: s.ctime_ns,
            ino: s.ino,
            dev: s.dev,
        }
    }

    /// Within `window` ns of now, either side (a time far in the future,
    /// from a clock ahead, is not: any write from now on gets an earlier
    /// one).
    fn racy(&self, window: i128) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as i128);
        (now - self.mtime_ns).abs() < window || (now - self.ctime_ns).abs() < window
    }
}

/// Whether the file system holding `path` (device `dev`) is local, once per
/// device.
fn local(path: &str, dev: u64) -> bool {
    static LOCAL: Mutex<Option<HashMap<u64, bool>>> = Mutex::new(None);
    if let Some(&l) = LOCAL
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|m| m.get(&dev))
    {
        return l;
    }
    let l = statfs_local(path);
    LOCAL
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_or_insert_with(HashMap::new)
        .insert(dev, l);
    l
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn statfs_local(path: &str) -> bool {
    let Ok(c) = std::ffi::CString::new(path) else {
        return false;
    };
    let mut s: libc::statfs = unsafe { std::mem::zeroed() };
    // SAFETY: a NUL-terminated path and a buffer of the type statfs fills.
    if unsafe { libc::statfs(c.as_ptr(), &mut s) } != 0 {
        return false;
    }
    #[cfg(target_os = "macos")]
    {
        s.f_flags & libc::MNT_LOCAL as u32 != 0
    }
    #[cfg(target_os = "linux")]
    {
        // statfs(2)'s magic numbers of the network and user-space file
        // systems: NFS, SMB, CIFS, SMB2, FUSE, AFS, Ceph, 9P, Coda, NCP.
        const REMOTE: [i64; 10] = [
            0x6969,
            0x517B,
            0xFF53_4D42u32 as i64,
            0xFE53_4D42u32 as i64,
            0x6573_5546,
            0x5346_414F,
            0x00C3_6400,
            0x0102_1997,
            0x7375_7245,
            0x564C,
        ];
        !REMOTE.contains(&(s.f_type as i64))
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn statfs_local(_path: &str) -> bool {
    false
}

/// More entries than any document reads; past it the cache starts afresh.
const MAX_ENTRIES: usize = 1 << 16;

/// Path -> (identity, hash, and the bytes of a file whose bytes are kept:
/// `keeps_bytes`).
type Kept = HashMap<String, (Ident, [u64; 2], Option<std::sync::Arc<[u8]>>)>;

static CACHE: Mutex<Option<Kept>> = Mutex::new(None);

/// How many files were read to be hashed (the tests).
#[cfg(test)]
static READS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Whether a file's bytes are kept with its hash: a font metric file
/// (lane T7-FULL10). Every compile that restarts before a document's first
/// math loads its 20-30 TFM files again, and each open, read and close was
/// three of the system calls that made the Mac's first page 2-3 ms slower
/// than the PC's. They are small (1-20 KB), and the same few dozen per
/// document.
fn keeps_bytes(path: &str, size: u64) -> bool {
    size <= 1 << 20
        && path
            .rsplit_once('.')
            .is_some_and(|(_, e)| e.eq_ignore_ascii_case("tfm"))
}

/// `persist::hash128_file(path, None)`'s hash, from the cache while the
/// file's identity is the one it was taken with and is not racy.
pub fn hash_file(path: &str) -> std::io::Result<[u64; 2]> {
    hash_file_within(path, crate::system::racy_ns())
}

/// [`hash_file`], with the file's identity taken from `m`, a `stat` of
/// `path` just made (`system::note_file` takes one for the file's
/// signature too).
pub fn hash_file_from(path: &str, m: &std::fs::Metadata) -> std::io::Result<[u64; 2]> {
    hash_ident(path, crate::system::racy_ns(), Ident::of_metadata(m))
}

fn hash_file_within(path: &str, window: i128) -> std::io::Result<[u64; 2]> {
    hash_ident(path, window, Ident::of(path)?)
}

fn hash_ident(path: &str, window: i128, before: Ident) -> std::io::Result<[u64; 2]> {
    let keep = cfg!(unix) && !before.racy(window) && local(path, before.dev);
    if keep {
        let c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((id, h, _)) = c.as_ref().and_then(|m| m.get(path)) {
            if *id == before {
                return Ok(*h);
            }
        }
    }
    #[cfg(test)]
    READS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let (h, n, bytes) = if keep && keeps_bytes(path, before.size) {
        let d = std::fs::read(path)?;
        (crate::persist::hash128(&d), d.len() as u64, Some(d.into()))
    } else {
        let (h, n) = crate::persist::hash128_file(path, None)?;
        (h, n, None)
    };
    if keep && n == before.size && Ident::of(path).ok() == Some(before) {
        let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        let m = c.get_or_insert_with(HashMap::new);
        if m.len() >= MAX_ENTRIES {
            m.clear();
        }
        m.insert(path.to_string(), (before, h, bytes));
    }
    Ok(h)
}

/// `path`'s bytes as the cache keeps them (`keeps_bytes`), when its
/// identity now -- `m`, a `stat` of it just made, or one taken here -- is
/// the one they were read under and is not racy: what reading the file
/// would give, as `hash_file`'s answer is its hash. `None`: read the file.
pub fn kept_bytes(path: &str, m: Option<&std::fs::Metadata>) -> Option<std::sync::Arc<[u8]>> {
    kept_bytes_within(path, m, crate::system::racy_ns())
}

fn kept_bytes_within(
    path: &str,
    m: Option<&std::fs::Metadata>,
    window: i128,
) -> Option<std::sync::Arc<[u8]>> {
    if !cfg!(unix) || crate::system::logical_len(path).is_some() {
        return None;
    }
    let now = match m {
        Some(m) => Ident::of_metadata(m),
        None => Ident::of(path).ok()?,
    };
    if !keeps_bytes(path, now.size) || now.racy(window) || !local(path, now.dev) {
        return None;
    }
    let c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    match c.as_ref().and_then(|m| m.get(path)) {
        Some((id, _, Some(b))) if *id == now => Some(b.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;
    use std::time::{Duration, SystemTime};

    /// A font metric file's bytes are kept with its hash and given back
    /// while its identity is the one they were read under: never after a
    /// change, and never for another kind of file.
    #[test]
    #[cfg(unix)]
    fn a_tfm_files_bytes_are_kept_while_it_is_unchanged() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let d = dir("kept");
        let (f, g) = (d.join("font.tfm"), d.join("font.sty"));
        std::fs::write(&f, b"tfm one").unwrap();
        std::fs::write(&g, b"sty one").unwrap();
        let (p, q) = (f.to_str().unwrap(), g.to_str().unwrap());
        assert_eq!(
            kept_bytes_within(p, None, 0),
            None,
            "kept before it was read"
        );
        assert_eq!(hash_file_within(p, 0).unwrap(), hash_of(&f));
        hash_file_within(q, 0).unwrap();
        assert_eq!(
            kept_bytes_within(p, None, 0).as_deref(),
            Some(&b"tfm one"[..])
        );
        let m = std::fs::metadata(&f).unwrap();
        assert_eq!(
            kept_bytes_within(p, Some(&m), 0).as_deref(),
            Some(&b"tfm one"[..])
        );
        assert_eq!(
            kept_bytes_within(q, None, 0),
            None,
            "a style file's bytes kept"
        );
        // within the racy window nothing is given back
        assert_eq!(
            kept_bytes_within(p, None, i128::MAX / 4),
            None,
            "racy, kept"
        );
        std::fs::write(&f, b"tfm two!").unwrap();
        assert_eq!(kept_bytes_within(p, None, 0), None, "kept after a change");
        assert_eq!(hash_file_within(p, 0).unwrap(), hash_of(&f));
        assert_eq!(
            kept_bytes_within(p, None, 0).as_deref(),
            Some(&b"tfm two!"[..])
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    // The tests share READS: one at a time.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn dir(name: &str) -> std::path::PathBuf {
        let d =
            std::env::temp_dir().join(format!("flashtex-hashcache-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn reads() -> u64 {
        READS.load(Ordering::Relaxed)
    }

    fn set_mtime(p: &std::path::Path, t: SystemTime) {
        std::fs::File::options()
            .write(true)
            .open(p)
            .unwrap()
            .set_modified(t)
            .unwrap();
    }

    fn hash_of(p: &std::path::Path) -> [u64; 2] {
        crate::persist::hash128(&std::fs::read(p).unwrap())
    }

    /// Without inodes and status-change times every lookup reads the file.
    #[test]
    #[cfg(not(unix))]
    fn without_unix_identities_every_file_is_read() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let f = dir("nounix").join("a.sty");
        std::fs::write(&f, "\\def\\a{1}").unwrap();
        set_mtime(&f, SystemTime::now() - Duration::from_secs(3600));
        let p = f.to_str().unwrap();
        let r0 = reads();
        assert_eq!(hash_file_within(p, 0).unwrap(), hash_of(&f));
        assert_eq!(hash_file_within(p, 0).unwrap(), hash_of(&f));
        assert_eq!(reads() - r0, 2);
    }

    /// An unchanged file is read once; its hash is the file's.
    #[test]
    #[cfg(unix)]
    fn an_unchanged_file_is_hashed_once() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let f = dir("once").join("a.sty");
        std::fs::write(&f, "\\def\\a{1}").unwrap();
        let p = f.to_str().unwrap();
        let r0 = reads();
        let h = hash_file_within(p, 0).unwrap();
        assert_eq!(h, hash_of(&f));
        assert_eq!(hash_file_within(p, 0).unwrap(), h);
        assert_eq!(hash_file_within(p, 0).unwrap(), h);
        assert_eq!(reads() - r0, 1);
    }

    /// A change of size, or a rewrite to the same size whose modification
    /// time is put back (`cp -p`), is hashed again: the status-change time
    /// moved.
    #[test]
    #[cfg(unix)]
    fn a_changed_file_is_hashed_again() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let f = dir("changed").join("b.sty");
        let old = SystemTime::now() - Duration::from_secs(3600);
        std::fs::write(&f, "\\def\\b{1}").unwrap();
        set_mtime(&f, old);
        let p = f.to_str().unwrap();
        let h1 = hash_file_within(p, 0).unwrap();
        // longer
        std::fs::write(&f, "\\def\\b{12}").unwrap();
        set_mtime(&f, old);
        let h2 = hash_file_within(p, 0).unwrap();
        assert_eq!(h2, hash_of(&f));
        assert_ne!(h1, h2);
        // the same size and modification time; only the status-change
        // time differs (on Windows the creation time stands for it and does
        // not move: there the size changes too)
        std::thread::sleep(Duration::from_millis(20));
        let same_len = if cfg!(unix) {
            "\\def\\b{34}"
        } else {
            "\\def\\b{345}"
        };
        std::fs::write(&f, same_len).unwrap();
        set_mtime(&f, old);
        let h3 = hash_file_within(p, 0).unwrap();
        assert_eq!(h3, hash_of(&f));
        assert_ne!(h2, h3);
    }

    /// A file within the racy window is read every time and not kept, so
    /// a second write within the same tick, to the same length and time,
    /// is seen.
    #[test]
    #[cfg(unix)]
    fn a_racy_file_is_never_kept() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let f = dir("racy").join("c.aux");
        std::fs::write(&f, "\\relax 1").unwrap();
        let t = std::fs::metadata(&f).unwrap().modified().unwrap();
        let p = f.to_str().unwrap();
        let window = 3_600_000_000_000; // an hour: everything is racy
        let r0 = reads();
        let h1 = hash_file_within(p, window).unwrap();
        assert_eq!(hash_file_within(p, window).unwrap(), h1);
        assert_eq!(reads() - r0, 2);
        std::fs::write(&f, "\\relax 2").unwrap();
        set_mtime(&f, t);
        let h2 = hash_file_within(p, window).unwrap();
        assert_eq!(h2, hash_of(&f));
        assert_ne!(h1, h2);
        // ... and nothing racy was kept for a later, non-racy lookup
        let r1 = reads();
        assert_eq!(hash_file_within(p, 0).unwrap(), h2);
        assert_eq!(reads() - r1, 1);
    }

    /// Another file at the path (a new inode) is hashed again.
    #[test]
    #[cfg(unix)]
    fn a_replaced_file_is_hashed_again() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let d = dir("replaced");
        let f = d.join("d.cls");
        let old = SystemTime::now() - Duration::from_secs(3600);
        std::fs::write(&f, "one").unwrap();
        set_mtime(&f, old);
        let p = f.to_str().unwrap();
        let h1 = hash_file_within(p, 0).unwrap();
        let g = d.join("d.cls.new");
        std::fs::write(&g, "two").unwrap();
        set_mtime(&g, old);
        std::fs::rename(&g, &f).unwrap();
        let h2 = hash_file_within(p, 0).unwrap();
        assert_eq!(h2, hash_of(&f));
        assert_ne!(h1, h2);
    }
}
