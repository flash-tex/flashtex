//! A file's identity as the file system reports it (moved from
//! flashtex-engine's system.rs, which re-exports it): the engines' read
//! sets, format caches and the bundle's cache compare files by it.

use crate::persist::Codec;

/// A file's identity as the file system reports it: a cheap test for
/// "unchanged" before its content is hashed again.
///
/// **Racy signatures** (git's "racy clean" rule). A file system keeps
/// modification times at some granularity: 1 s on HFS+, 2 s on FAT, a
/// kernel tick on ext4 before Linux 6.13. A file changed again within the
/// tick in which its signature was taken, to the same length, keeps the
/// same signature. So a signature taken within [`RACY_NS`] of the file's
/// modification time (`racy`) proves nothing: it equals no signature, not
/// even itself, and every "unchanged?" test that meets one compares the
/// content instead (and may then keep the fresh signature, which is not
/// racy once the tick has passed).
#[derive(Clone, Copy, Debug, Default)]
pub struct StatSig {
    pub len: u64,
    pub mtime_ns: i128,
    pub ino: u64,
    /// Taken within [`RACY_NS`] of `mtime_ns`, either side.
    pub racy: bool,
}

/// The widest modification-time granularity of a supported file system
/// (FAT's 2 s; HFS+ 1 s, ext4 a kernel tick), for [`StatSig::racy`]
/// (`FLASHTEX_RACY_MS` changes it, for the tests).
pub const RACY_NS: i128 = 2_000_000_000;

#[inline]
pub fn racy_ns() -> i128 {
    static R: std::sync::OnceLock<i128> = std::sync::OnceLock::new();
    *R.get_or_init(|| {
        std::env::var("FLASHTEX_RACY_MS")
            .ok()
            .and_then(|v| v.parse::<i128>().ok())
            .map_or(RACY_NS, |ms| ms * 1_000_000)
    })
}

impl PartialEq for StatSig {
    #[inline]
    fn eq(&self, o: &StatSig) -> bool {
        !self.racy && !o.racy && self.same_fields(o)
    }
}

impl StatSig {
    #[inline]
    pub fn of(path: &str) -> Option<StatSig> {
        let m = std::fs::metadata(path).ok()?;
        let mtime_ns = m
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos() as i128);
        let ino = crate::os::file_id(&m);
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as i128);
        Some(StatSig {
            len: m.len(),
            mtime_ns,
            ino,
            // Within the tick on either side: a write after now lands in
            // the same tick only then. (A modification time far in the
            // future -- a file from a clock ahead -- is not racy: any write
            // from now on gets an earlier time.)
            racy: (now_ns - mtime_ns).abs() < racy_ns(),
        })
    }

    /// The same length, time and identity, racy or not (only where a racy
    /// equality is harmless: see the callers).
    #[inline]
    pub fn same_fields(&self, o: &StatSig) -> bool {
        (self.len, self.mtime_ns, self.ino) == (o.len, o.mtime_ns, o.ino)
    }
}

/// (The S₀ cache persists signatures.)
impl Codec for StatSig {
    fn enc(&self, w: &mut Vec<u8>) {
        self.len.enc(w);
        (self.mtime_ns as i64).enc(w);
        ((self.mtime_ns >> 64) as i64).enc(w);
        self.ino.enc(w);
        (self.racy as u64).enc(w);
    }
    fn dec(r: &mut crate::persist::Reader) -> Result<Self, String> {
        let len = u64::dec(r)?;
        let lo = i64::dec(r)? as u64 as i128;
        let hi = i64::dec(r)? as i128;
        Ok(StatSig {
            len,
            mtime_ns: (hi << 64) | lo,
            ino: u64::dec(r)?,
            racy: u64::dec(r)? != 0,
        })
    }
}
