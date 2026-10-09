//! The project's FlashTeX Typst lock, `flashtex-typst.lock` in the project
//! root (DESIGN.md §15.2): what the document was set with, so another
//! machine (or a later session) notices when it differs.
//!
//! * **`[packages]`**: `"@ns/name:version" = "sha256:<hex>"`, the SHA-256 of
//!   the package's tarball as it was first fetched. Every later fetch, and
//!   every use of a cached copy, is checked against it; a mismatch is an
//!   error, never silently accepted (typst-kit verifies nothing but TLS, and
//!   the Universe index has no hash field: Track A §6, Track C §2.6).
//! * **`[fonts]`**: `"family|style|weight|stretch" = "sha256:<hex>"`, the
//!   SHA-256 of each font file the document's text uses, recorded on its
//!   first successful compile. A missing or different file is a prominent
//!   warning: Typst itself only warns about an unknown family, and a changed
//!   file silently reflows (Track C §2.5).
//!
//! **Why a separate file in the project root.** It is the project's, like a
//! `Cargo.lock`: committed with the sources so collaborators and CI get the
//! same packages and are told about different fonts; the host writes it
//! (never `flashtex.toml`, which the user edits), and one file keeps both
//! lists in step. The format is a strict TOML subset (one `key = "value"`
//! per line, two tables), readable by any TOML parser and diffable, written
//! sorted so it changes only where an entry does; a later version's tables
//! and keys are kept verbatim. It is read through the project root's
//! confined paths and written atomically: under an exclusive `flock` on the
//! project root (so two hosts, e.g. two documents of one project, never lose
//! each other's entries), re-read, changed, written to a new file created
//! `O_EXCL | O_NOFOLLOW` beside it and `renameat` over it ([`update`]).
//!
//! **The user's edits win.** The host's first line is a stamp, the SHA-256 of
//! the rest of the file as it wrote it. A lock whose stamp is missing or does
//! not match was changed outside FlashTeX since a host last wrote it: it is
//! never overwritten (a `lock` note says so) unless the client asks for
//! `"lock": "update"`. A project the user cannot write (a read-only
//! directory or lock) is not written at all.

use std::collections::BTreeMap;
use std::path::Path;

/// The lock's file name, in the project root.
pub const FILE: &str = "flashtex-typst.lock";

/// The first line of a lock a host wrote, before the SHA-256 (hex) of the
/// rest of the file.
const STAMP: &str = "# flashtex-typst-host: sha256 of what follows: ";

/// `text` with the host's stamp line in front.
pub fn stamped(text: &str) -> String {
    let h = flashtex_display_list::sha256::sha256(text.as_bytes());
    format!("{STAMP}{}\n{text}", flashtex_display_list::sha256::hex(&h))
}

/// The file is exactly as a host wrote it (its stamp matches the rest).
pub fn host_written(text: &str) -> bool {
    match text.split_once('\n') {
        Some((first, rest)) => first.strip_prefix(STAMP).is_some_and(|h| {
            h == flashtex_display_list::sha256::hex(&flashtex_display_list::sha256::sha256(
                rest.as_bytes(),
            ))
        }),
        None => false,
    }
}

/// The lock's contents.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lock {
    /// `@ns/name:version` → tarball SHA-256 (hex).
    pub packages: BTreeMap<String, String>,
    /// `family|style|weight|stretch` → (font file SHA-256 (hex), file name).
    pub fonts: BTreeMap<String, (String, String)>,
    /// Top-level lines other than `version` (a later version's), verbatim.
    pub other_top: Vec<String>,
    /// Tables other than `packages` and `fonts`, verbatim (header included).
    pub other_tables: Vec<String>,
}

impl Lock {
    /// Parse the lock's text. Unknown tables and top-level keys (a later
    /// version's) are kept verbatim; a malformed line of a known table is an
    /// error with its line number.
    pub fn parse(text: &str) -> Result<Lock, String> {
        let mut lock = Lock::default();
        let mut table = String::new();
        let mut comment_file = String::new();
        let known = |t: &str| matches!(t, "" | "packages" | "fonts");
        for (n, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if let Some(t) = line.strip_prefix('[') {
                table = t
                    .strip_suffix(']')
                    .ok_or_else(|| format!("{FILE}:{}: unclosed table header", n + 1))?
                    .trim()
                    .to_string();
                if !known(&table) {
                    lock.other_tables.push(raw.to_string());
                }
                continue;
            }
            if !known(&table) {
                lock.other_tables.push(raw.to_string());
                continue;
            }
            if line.is_empty() {
                continue;
            }
            if let Some(c) = line.strip_prefix('#') {
                // `# file: NAME` before a font entry names its file.
                comment_file = c
                    .trim()
                    .strip_prefix("file:")
                    .map(|f| f.trim().to_string())
                    .unwrap_or_default();
                continue;
            }
            let (key, rest) = if line.starts_with('"') {
                let (k, used) =
                    unquote(line).ok_or_else(|| format!("{FILE}:{}: a malformed key", n + 1))?;
                (k, line[used..].trim_start())
            } else {
                let at = line
                    .find('=')
                    .ok_or_else(|| format!("{FILE}:{}: expected `key = value`", n + 1))?;
                (line[..at].trim().to_string(), &line[at..])
            };
            let value = rest
                .strip_prefix('=')
                .ok_or_else(|| format!("{FILE}:{}: expected `=`", n + 1))?
                .trim();
            if table.is_empty() && key == "version" {
                if value != "1" {
                    return Err(format!(
                        "{FILE}:{}: lock version {value} is newer than this host reads (1)",
                        n + 1
                    ));
                }
                continue;
            }
            if table.is_empty() {
                lock.other_top.push(raw.to_string());
                continue;
            }
            let (value, used) =
                unquote(value).ok_or_else(|| format!("{FILE}:{}: a malformed value", n + 1))?;
            let _ = used;
            let sha = |v: &str| -> Result<String, String> {
                let h = v
                    .strip_prefix("sha256:")
                    .filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
                    .ok_or_else(|| format!("{FILE}:{}: expected \"sha256:<64 hex>\"", n + 1))?;
                Ok(h.to_ascii_lowercase())
            };
            match table.as_str() {
                "packages" => {
                    lock.packages.insert(key, sha(&value)?);
                }
                "fonts" => {
                    lock.fonts
                        .insert(key, (sha(&value)?, std::mem::take(&mut comment_file)));
                }
                _ => {}
            }
        }
        Ok(lock)
    }

    /// The lock's text, sorted (so it changes only where an entry does).
    pub fn to_text(&self) -> String {
        let mut o = String::from(
            "# flashtex-typst.lock: written by FlashTeX's Typst host (DESIGN.md §15.2).\n\
             # Commit it with the project. [packages]: the SHA-256 of each package's\n\
             # tarball, checked on every later fetch. [fonts]: the font files the\n\
             # document was set with; a missing or different file is reported.\n\
             version = 1\n",
        );
        for l in &self.other_top {
            o.push_str(l);
            o.push('\n');
        }
        o.push_str("\n[packages]\n");
        for (k, v) in &self.packages {
            o.push_str(&format!("{} = \"sha256:{v}\"\n", quote(k)));
        }
        o.push_str("\n[fonts]\n");
        for (k, (v, file)) in &self.fonts {
            if !file.is_empty() {
                o.push_str(&format!("# file: {}\n", file.replace('\n', " ")));
            }
            o.push_str(&format!("{} = \"sha256:{v}\"\n", quote(k)));
        }
        if !self.other_tables.is_empty() {
            o.push('\n');
            for l in &self.other_tables {
                o.push_str(l);
                o.push('\n');
            }
        }
        o
    }

    /// Read the lock of the project at `root` (canonical): `None` when there
    /// is none. A lock that is a symlink, or that leaves the root, is refused.
    pub fn read(root: &Path) -> Result<Option<Lock>, String> {
        match read_text(root)? {
            None => Ok(None),
            Some(text) => Lock::parse(&text).map(Some),
        }
    }
}

/// The lock's text, `None` when there is none; a symlink, or a path that
/// leaves the root, is refused.
fn read_text(root: &Path) -> Result<Option<String>, String> {
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    // One open that never follows a symlink (no check-then-read race); the
    // root is canonical.
    let mut f = match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(root.join(FILE))
    {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) if e.raw_os_error() == Some(libc::ELOOP) => {
            return Err(format!("{FILE} is refused: it is a symlink"))
        }
        Err(e) => return Err(format!("{FILE}: {e}")),
    };
    if !f.metadata().map_err(|e| format!("{FILE}: {e}"))?.is_file() {
        return Err(format!("{FILE} is refused: it is not a regular file"));
    }
    let mut text = String::new();
    f.read_to_string(&mut text)
        .map_err(|e| format!("{FILE}: {e}"))?;
    Ok(Some(text))
}

/// Remove the lock's temporary files that hosts which no longer run left in
/// the project root (`.flashtex-typst.lock.<pid>.<n>.tmp`, this user's
/// only); called before this host writes the lock.
fn sweep_stale_temps(root: &Path) {
    use std::os::unix::fs::MetadataExt;
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let prefix = format!(".{FILE}.");
    for e in entries.flatten() {
        let name = e.file_name();
        let Some(pid) = name
            .to_str()
            .and_then(|n| n.strip_prefix(prefix.as_str()))
            .and_then(|r| r.strip_suffix(".tmp"))
            .and_then(|r| r.split('.').next())
            .and_then(|p| p.parse::<i32>().ok())
        else {
            continue;
        };
        if let Ok(md) = std::fs::symlink_metadata(e.path()) {
            if md.is_file()
                && md.uid() == crate::watchdog::uid()
                && !crate::watchdog::pid_alive(pid)
            {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

/// What [`update`] did.
#[derive(Debug)]
pub struct Updated {
    /// The lock as it now is (written, or only in memory).
    pub lock: Lock,
    /// The written file's metadata (from its descriptor, before the rename,
    /// so a later change by anyone else shows as a different file).
    pub written: Option<std::fs::Metadata>,
    /// `f` changed the lock but it could not be written (a read-only
    /// project, or the project root could not be locked in [`FLOCK_WAIT`]):
    /// why. The caller keeps the change in memory.
    pub not_written: Option<String>,
}

/// How long [`update`] tries to lock the project root before it keeps its
/// change in memory instead.
pub const FLOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(2);

/// Change the lock of the project at `root` (canonical) with `f` and write
/// it if `f` says it changed: under an exclusive `flock` on the root
/// directory (tried for [`FLOCK_WAIT`], never blocking longer), the lock is
/// re-read from disk (what another host wrote since is kept), changed,
/// written to a fresh file in the root (`openat` `O_CREAT | O_EXCL |
/// O_NOFOLLOW`) and renamed over the lock (`renameat`, which replaces the
/// name and never follows it). A lock that cannot be read (a symlink, a
/// newer version, malformed) is an error and is left as it is.
///
/// A lock changed outside FlashTeX since a host wrote it (its stamp does
/// not match) is an error and is left as it is, unless `force` (the
/// client's `"lock": "update"`). A project that is not writable, or whose
/// root cannot be locked in time, is not written: [`Updated::not_written`].
pub fn update(
    root: &Path,
    force: bool,
    f: impl FnOnce(&mut Lock) -> bool,
) -> Result<Updated, String> {
    use std::ffi::CString;
    use std::io::Write;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::ffi::OsStrExt;

    let io = |what: &str| format!("{FILE}: {what}: {}", std::io::Error::last_os_error());
    let root_c = CString::new(root.as_os_str().as_bytes()).map_err(|_| "a NUL in the root")?;
    // SAFETY: a NUL-terminated path; the descriptor is owned exactly once.
    let dir = unsafe {
        libc::open(
            root_c.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    if dir < 0 {
        return Err(io("open the project root"));
    }
    let dir = unsafe { OwnedFd::from_raw_fd(dir) };
    // Released when `dir` is closed, also on every early return. Never
    // blocks: another host holding it for longer than FLOCK_WAIT (or a
    // file system without flock) leaves the change in memory.
    let start = std::time::Instant::now();
    let locked = loop {
        if unsafe { libc::flock(dir.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            break Ok(());
        }
        let e = std::io::Error::last_os_error();
        if e.raw_os_error() != Some(libc::EWOULDBLOCK) {
            break Err(format!("the project root cannot be locked ({e})"));
        }
        if start.elapsed() >= FLOCK_WAIT {
            break Err(format!(
                "another FlashTeX host held the project's lock for {} s",
                FLOCK_WAIT.as_secs()
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let text = read_text(root)?;
    let mut lock = match &text {
        Some(t) => Lock::parse(t)?,
        None => Lock::default(),
    };
    if let Some(t) = &text {
        if !force && !host_written(t) {
            return Err(format!(
                "{FILE} was changed outside FlashTeX since it was last written: it is left as \
                 it is (compile with \"lock\": \"update\" to accept it and record again)"
            ));
        }
    }
    if !f(&mut lock) {
        return Ok(Updated {
            lock,
            written: None,
            not_written: None,
        });
    }
    if let Err(why) = locked {
        return Ok(Updated {
            lock,
            written: None,
            not_written: Some(why),
        });
    }
    // A read-only project (directory or lock) is not written.
    let file_c = CString::new(FILE).unwrap();
    let dot = CString::new(".").unwrap();
    // SAFETY: a directory descriptor and NUL-terminated names.
    let writable = unsafe { libc::faccessat(dir.as_raw_fd(), dot.as_ptr(), libc::W_OK, 0) } == 0
        && (text.is_none()
            || unsafe { libc::faccessat(dir.as_raw_fd(), file_c.as_ptr(), libc::W_OK, 0) } == 0);
    if !writable {
        return Ok(Updated {
            lock,
            written: None,
            not_written: Some("the project is read-only".into()),
        });
    }
    sweep_stale_temps(root);
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let tmp = format!(
        ".{FILE}.{}.{}.tmp",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let tmp_c = CString::new(tmp.as_bytes()).unwrap();
    // SAFETY: a directory descriptor and NUL-terminated names.
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            tmp_c.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o644 as libc::c_uint,
        )
    };
    if fd < 0 {
        return Err(io("create a temporary file"));
    }
    let mut file = std::fs::File::from(unsafe { OwnedFd::from_raw_fd(fd) });
    let written = file
        .write_all(stamped(&lock.to_text()).as_bytes())
        .and_then(|_| file.sync_all())
        .and_then(|_| file.metadata());
    drop(file);
    let renamed = written.is_ok()
        && unsafe {
            libc::renameat(
                dir.as_raw_fd(),
                tmp_c.as_ptr(),
                dir.as_raw_fd(),
                file_c.as_ptr(),
            )
        } == 0;
    if !renamed {
        let e = written
            .as_ref()
            .err()
            .map(|e| format!("{FILE}: {e}"))
            .unwrap_or_else(|| io("rename"));
        unsafe { libc::unlinkat(dir.as_raw_fd(), tmp_c.as_ptr(), 0) };
        return Err(e);
    }
    Ok(Updated {
        lock,
        written: written.ok(),
        not_written: None,
    })
}

/// A TOML basic string.
fn quote(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => o.push_str(&format!("\\u{:04X}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Read a TOML basic string at the start of `s`: (text, bytes used).
fn unquote(s: &str) -> Option<(String, usize)> {
    let mut chars = s.char_indices();
    if chars.next()?.1 != '"' {
        return None;
    }
    let mut o = String::new();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Some((o, i + 1)),
            '\\' => match chars.next()?.1 {
                '"' => o.push('"'),
                '\\' => o.push('\\'),
                'n' => o.push('\n'),
                't' => o.push('\t'),
                'u' => {
                    let hex: String = (0..4).filter_map(|_| chars.next().map(|x| x.1)).collect();
                    o.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                }
                _ => return None,
            },
            c => o.push(c),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_strictness() {
        let mut l = Lock::default();
        l.packages
            .insert("@preview/cetz:0.3.1".into(), "ab".repeat(32));
        l.fonts.insert(
            "Weird \"Family\"\\|Normal|400|1000".into(),
            ("cd".repeat(32), "W.otf".into()),
        );
        let t = l.to_text();
        assert_eq!(Lock::parse(&t).unwrap(), l);
        assert!(Lock::parse("version = 2\n").is_err());
        assert!(Lock::parse("[packages]\n\"x\" = \"sha256:12\"\n").is_err());
        assert!(Lock::parse("[packages]\n\"x\" = \"md5:00\"\n").is_err());
        // A later version's tables and top-level keys are kept verbatim.
        let later = t.replace("version = 1\n", "version = 1\nnew_key = 3\n")
            + "\n[other]\n# kept\nk = { a = 1 }\n";
        let p = Lock::parse(&later).unwrap();
        assert_eq!((&p.packages, &p.fonts), (&l.packages, &l.fonts));
        let again = p.to_text();
        assert!(again.contains("version = 1\nnew_key = 3\n"), "{again}");
        assert!(
            again.ends_with("[other]\n# kept\nk = { a = 1 }\n"),
            "{again}"
        );
        assert_eq!(Lock::parse(&again).unwrap(), p);
    }

    /// The host's own file is rewritten; a file the user changed is not,
    /// unless forced; a read-only project is not written.
    #[test]
    fn user_edits_and_read_only_projects_are_respected() {
        let dir = std::env::temp_dir().join(format!("ftth-lockstamp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let root = dir.canonicalize().unwrap();
        let add = |k: &'static str| {
            move |l: &mut Lock| {
                l.packages.insert(k.into(), "ab".repeat(32));
                true
            }
        };
        update(&root, false, add("@preview/a:1.0.0")).unwrap();
        let text = std::fs::read_to_string(root.join(FILE)).unwrap();
        assert!(host_written(&text), "{text}");
        update(&root, false, add("@preview/b:1.0.0")).unwrap();
        // The user edits it: never overwritten without `force`.
        let edited = std::fs::read_to_string(root.join(FILE)).unwrap() + "# mine\n";
        std::fs::write(root.join(FILE), &edited).unwrap();
        let err = update(&root, false, add("@preview/c:1.0.0")).unwrap_err();
        assert!(err.contains("changed outside FlashTeX"), "{err}");
        assert_eq!(std::fs::read_to_string(root.join(FILE)).unwrap(), edited);
        let u = update(&root, true, add("@preview/c:1.0.0")).unwrap();
        assert!(u.written.is_some() && u.lock.packages.len() == 3);
        assert!(host_written(
            &std::fs::read_to_string(root.join(FILE)).unwrap()
        ));
        // Read-only: nothing is written, nothing fails.
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(root.join(FILE), std::fs::Permissions::from_mode(0o444)).unwrap();
        let before = std::fs::read_to_string(root.join(FILE)).unwrap();
        let u = update(&root, false, add("@preview/d:1.0.0")).unwrap();
        assert!(u.written.is_none());
        assert_eq!(u.not_written.as_deref(), Some("the project is read-only"));
        assert!(u.lock.packages.contains_key("@preview/d:1.0.0"));
        assert_eq!(std::fs::read_to_string(root.join(FILE)).unwrap(), before);
        std::fs::set_permissions(root.join(FILE), std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Another host holding the project root's flock: `update` gives up
    /// after FLOCK_WAIT and returns the change unwritten, never blocking.
    #[test]
    fn a_held_flock_times_out_into_memory() {
        use std::os::fd::AsRawFd;
        let dir = std::env::temp_dir().join(format!("ftth-flock-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let root = dir.canonicalize().unwrap();
        let holder = std::fs::File::open(&root).unwrap();
        assert_eq!(unsafe { libc::flock(holder.as_raw_fd(), libc::LOCK_EX) }, 0);
        let t = std::time::Instant::now();
        let u = update(&root, false, |l| {
            l.packages
                .insert("@preview/x:1.0.0".into(), "cd".repeat(32));
            true
        })
        .unwrap();
        let took = t.elapsed();
        assert!(took >= FLOCK_WAIT && took < FLOCK_WAIT * 3, "{took:?}");
        assert!(u.written.is_none());
        assert!(u.not_written.unwrap().contains("held the project's lock"));
        assert!(u.lock.packages.contains_key("@preview/x:1.0.0"));
        assert!(!root.join(FILE).exists());
        drop(holder);
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Temporary lock files of dead hosts are swept before a write; a live
    /// host's, and anything not named like ours, is left.
    #[test]
    fn stale_temporary_lock_files_are_swept() {
        let dir = std::env::temp_dir().join(format!("ftth-locksweep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let root = dir.canonicalize().unwrap();
        // pid 2^22 + 7 is beyond every OS's default pid range.
        let dead = root.join(format!(".{FILE}.{}.0.tmp", (1 << 22) + 7));
        let live = root.join(format!(".{FILE}.{}.0.tmp", std::process::id()));
        let other = root.join("notes.tmp");
        for f in [&dead, &live, &other] {
            std::fs::write(f, "x").unwrap();
        }
        sweep_stale_temps(&root);
        assert!(!dead.exists() && live.exists() && other.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
