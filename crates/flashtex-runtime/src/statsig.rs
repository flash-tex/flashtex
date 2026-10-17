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
///
/// **The change time.** A modification time can be put back: `cp -p` onto
/// an existing file, `rsync -t --inplace` and `touch -r` rewrite a file in
/// place and then restore its old time, which with the same length kept
/// the same signature. The status-change time (`ctime_ns`: Unix's
/// `st_ctime`, Windows' `ChangeTime`) moves with every such write and
/// cannot be set, so the signature holds it too, and a signature taken
/// within [`RACY_NS`] of it is racy as well (its granularity is the
/// modification time's). Where the system gives no change time (WASI, or
/// a Windows file that cannot be opened for its attributes) it is 0 and
/// the signature rests on the other fields, as before.
#[derive(Clone, Copy, Debug, Default)]
pub struct StatSig {
    pub len: u64,
    pub mtime_ns: i128,
    /// The status-change time in ns since the epoch (0: none known).
    pub ctime_ns: i128,
    pub ino: u64,
    /// Taken within [`RACY_NS`] of `mtime_ns` or `ctime_ns`, either side.
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

/// Whether a signature with times `mtime_ns` and `ctime_ns`, taken at
/// `now_ns`, is racy: either time within `window` of now, on either side.
/// A write after now lands in the same tick only then. (A modification
/// time far in the future -- a file from a clock ahead -- is not racy: any
/// write from now on gets an earlier time.) The change time likewise: a
/// write with the modification time put back, within its tick, would
/// leave the change time as it was too. (0, no change time known, is never
/// within the window.)
#[inline]
pub fn racy_at(now_ns: i128, mtime_ns: i128, ctime_ns: i128, window: i128) -> bool {
    (now_ns - mtime_ns).abs() < window || (now_ns - ctime_ns).abs() < window
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
        StatSig::of_within(path, racy_ns())
    }

    /// [`StatSig::of`] with a racy window of `window` ns in place of
    /// [`racy_ns`] (the tests: 0, nothing is racy).
    pub fn of_within(path: &str, window: i128) -> Option<StatSig> {
        let (m, ctime_ns) = crate::os::metadata_and_change_time(path)?;
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
            ctime_ns,
            ino,
            racy: racy_at(now_ns, mtime_ns, ctime_ns, window),
        })
    }

    /// The same length, times and identity, racy or not (only where a racy
    /// equality is harmless: see the callers).
    #[inline]
    pub fn same_fields(&self, o: &StatSig) -> bool {
        (self.len, self.mtime_ns, self.ctime_ns, self.ino) == (o.len, o.mtime_ns, o.ctime_ns, o.ino)
    }
}

/// (The S₀ caches persist signatures: a change to this encoding bumps
/// their magic, flashtex-engine's `host::MAGIC` and flashtex-xetex's
/// `s0file::MAGIC`, so that an older file is refused, not misread.)
impl Codec for StatSig {
    fn enc(&self, w: &mut Vec<u8>) {
        self.len.enc(w);
        (self.mtime_ns as i64).enc(w);
        ((self.mtime_ns >> 64) as i64).enc(w);
        (self.ctime_ns as i64).enc(w);
        ((self.ctime_ns >> 64) as i64).enc(w);
        self.ino.enc(w);
        (self.racy as u64).enc(w);
    }
    fn dec(r: &mut crate::persist::Reader) -> Result<Self, String> {
        let len = u64::dec(r)?;
        let lo = i64::dec(r)? as u64 as i128;
        let hi = i64::dec(r)? as i128;
        let clo = i64::dec(r)? as u64 as i128;
        let chi = i64::dec(r)? as i128;
        Ok(StatSig {
            len,
            mtime_ns: (hi << 64) | lo,
            ctime_ns: (chi << 64) | clo,
            ino: u64::dec(r)?,
            racy: u64::dec(r)? != 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn set_mtime(p: &std::path::Path, t: SystemTime) {
        std::fs::File::options()
            .write(true)
            .open(p)
            .unwrap()
            .set_modified(t)
            .unwrap();
    }

    /// A file rewritten in place to the same length with its modification
    /// time put back (`cp -p` onto it, `rsync -t --inplace`, `touch -r`)
    /// has another signature: its change time moved. And a signature taken
    /// within the racy window of the change time is racy, whatever the
    /// modification time.
    #[test]
    fn an_in_place_rewrite_with_the_time_put_back_is_seen() {
        let d = std::env::temp_dir().join(format!("flashtex-rt-statsig-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("f.tex");
        let p = f.to_str().unwrap();
        let old = SystemTime::now() - Duration::from_secs(3600);
        std::fs::write(&f, "abcd").unwrap();
        set_mtime(&f, old);
        let before = StatSig::of_within(p, 0).unwrap();
        assert!(!before.racy);
        // where the system keeps no change time this cannot be seen
        if before.ctime_ns == 0 {
            let _ = std::fs::remove_dir_all(&d);
            return;
        }
        // the change time has just moved, the modification time has not
        assert!(StatSig::of_within(p, RACY_NS).unwrap().racy);
        // Rewrite in place until the change time's tick has passed (a
        // nanosecond on APFS and ext4, a second or two at worst).
        let mut after = before;
        for i in 0..300 {
            std::thread::sleep(Duration::from_millis(10));
            let mut w = std::fs::File::options().write(true).open(&f).unwrap();
            let bytes: &[u8] = if i % 2 == 0 { b"wxyz" } else { b"abcd" };
            std::io::Write::write_all(&mut w, bytes).unwrap();
            drop(w);
            set_mtime(&f, old);
            after = StatSig::of_within(p, 0).unwrap();
            if after.ctime_ns != before.ctime_ns {
                break;
            }
        }
        assert_eq!(
            (after.len, after.mtime_ns, after.ino),
            (before.len, before.mtime_ns, before.ino)
        );
        assert_ne!(
            after.ctime_ns, before.ctime_ns,
            "the change time never moved"
        );
        assert!(!after.same_fields(&before));
        assert!(after != before);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Racy: either time within the window, on either side; a time far
    /// ahead, or no change time (0), is not.
    #[test]
    fn racy_on_either_time() {
        let (now, w) = (1_000_000_000_000_000_000i128, RACY_NS);
        let hour = 3_600_000_000_000i128;
        assert!(!racy_at(now, now - hour, now - hour, w));
        assert!(racy_at(now, now - w / 2, now - hour, w));
        assert!(racy_at(now, now - hour, now - w / 2, w));
        assert!(racy_at(now, now - hour, now + w / 2, w));
        assert!(!racy_at(now, now + hour, now - hour, w));
        assert!(!racy_at(now, now - hour, 0, w));
        assert!(!racy_at(now, now, now, 0));
    }

    /// The codec keeps every field, the change time's high half too.
    #[test]
    fn the_codec_keeps_the_change_time() {
        let s = StatSig {
            len: 7,
            mtime_ns: -5,
            ctime_ns: (1i128 << 70) + 3,
            ino: 9,
            racy: true,
        };
        let mut w = Vec::new();
        s.enc(&mut w);
        let t = StatSig::dec(&mut crate::persist::Reader::new(&w)).unwrap();
        assert!(t.same_fields(&s) && t.racy);
    }
}
