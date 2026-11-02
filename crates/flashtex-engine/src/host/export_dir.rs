//! An export's own output directory (Commander's ruling, 2026-11-02; spec
//! §6.3 `export_dir`). An `export: true` compile runs the engine as a child
//! process; in the session's output directory it wrote the `.aux`, the log
//! and the rest again, which the resident engine's checkpoints hold (an
//! export's log is not the preview's: pdfTeX's says the PDF is compressed),
//! so the next keystroke compiled from S₀ or from nothing. It runs instead
//! in a directory of its own, named with [`DIR`] in the client's `export_dir` (the
//! app's cache, never next to the sources), seeded with what the session's
//! runs wrote there (`seed`): it reads the `.aux`, `.toc`, `.bbl`, ... a run
//! in the session's directory reads, and writes only its own copies. The
//! session's files are never touched, for either host; `DONE.pdf` and
//! `DONE.log` name the export's.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The prefix of an export's directory inside the client's `export_dir`.
pub const DIR: &str = "ftx-export";
const MARKER: &str = ".flashtex-export";
static ACTIVE: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();

pub struct Lease(PathBuf);

impl Drop for Lease {
    fn drop(&mut self) {
        ACTIVE.get().unwrap().lock().unwrap().remove(&self.0);
    }
}

/// Refuse a second live export to the same session's destination.
pub fn reserve(base: &Path, session: &Path) -> Result<Lease, String> {
    let dir = destination(base, session)?;
    let dir = dir
        .parent()
        .unwrap()
        .canonicalize()
        .map_err(|e| format!("export_dir {}: {e}", base.display()))?
        .join(dir.file_name().unwrap());
    let mut active = ACTIVE
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap();
    if !active.insert(dir.clone()) {
        return Err(format!("export already running in {}", dir.display()));
    }
    Ok(Lease(dir))
}

fn destination(base: &Path, session: &Path) -> Result<PathBuf, String> {
    if !base.is_absolute() {
        return Err(format!("export_dir {} is not absolute", base.display()));
    }
    std::fs::create_dir_all(base).map_err(|e| format!("export_dir {}: {e}", base.display()))?;
    let session = session
        .canonicalize()
        .map_err(|e| format!("session {}: {e}", session.display()))?;
    let digest = crate::pdftex::md5::digest(session.as_os_str().as_encoded_bytes());
    let suffix: String = digest[..8].iter().map(|b| format!("{b:02x}")).collect();
    Ok(base.join(format!("{DIR}-{suffix}")))
}

/// The session's files an export does not take: what the session's runs
/// make and never read back (the PDF, the log, SyncTeX's), and the
/// resident engine's own (`*.flashtex-removed`, a fatal run's PDF set aside).
fn not_seeded(name: &str) -> bool {
    [
        ".pdf",
        ".log",
        ".synctex",
        ".synctex.gz",
        ".fls",
        ".flashtex-removed",
    ]
    .iter()
    .any(|e| name.ends_with(e))
}

/// A job's own files in the project's directory that are its sources, not
/// what a run wrote ([`Seed::Job`]).
const SOURCES: &[&str] = &[
    ".tex", ".ltx", ".bib", ".sty", ".cls", ".dtx", ".ins", ".png", ".jpg", ".jpeg", ".eps",
    ".svg", ".toml",
];

/// The default `export_dir` of connection `conn` (a client that names
/// none): a temporary directory of this process's.
pub fn default_base(conn: u64) -> PathBuf {
    std::env::temp_dir().join(format!("flashtex-export-{}-{conn}", std::process::id()))
}

/// What of the session's directory an export takes.
#[derive(Clone, Copy, Debug)]
pub enum Seed<'a> {
    /// A directory of the session's own: every file but [`not_seeded`]'s.
    All,
    /// The project's directory itself (a session without an output
    /// directory of its own; the job's name): only what LaTeX writes there
    /// and reads back, the `.aux` files (`\include`'s too) and the job's
    /// own `jobname.*` (`.toc`, `.bbl`, ...), never the sources and figures.
    Job(&'a str),
}

/// The directory for an export of a job whose output directory is
/// `session`, in `base`: `base/ftx-export-<session hash>`, remade and
/// seeded with `session`'s files
/// (`what`, with its subdirectories, which `\include` needs; links are not
/// followed). pdfTeX and XeTeX look for a file in `-output-directory`
/// before the current directory (kpathsea's `open_input`), so the export
/// reads these copies.
pub fn prepare(base: &Path, session: &Path, what: Seed) -> Result<PathBuf, String> {
    let dir = destination(base, session)?;
    let base_canonical = base
        .canonicalize()
        .map_err(|e| format!("export_dir {}: {e}", base.display()))?;
    let session = session
        .canonicalize()
        .map_err(|e| format!("session {}: {e}", session.display()))?;
    let canonical_dir = dir
        .canonicalize()
        .unwrap_or_else(|_| base_canonical.join(dir.file_name().unwrap()));
    if canonical_dir.starts_with(&session)
        || session.starts_with(&canonical_dir)
        || base_canonical.starts_with(&session)
    {
        return Err(format!(
            "export_dir {} overlaps session {}",
            base.display(),
            session.display()
        ));
    }
    match std::fs::symlink_metadata(&dir) {
        Ok(m) if m.is_dir() && dir.join(MARKER).is_file() => {
            std::fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?
        }
        Ok(_) => {
            return Err(format!(
                "{} exists and is not an export directory",
                dir.display()
            ))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("{}: {e}", dir.display())),
    }
    std::fs::create_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    std::fs::write(dir.join(MARKER), b"").map_err(|e| format!("{}: {e}", dir.display()))?;
    let skip = [base_canonical.as_path(), dir.as_path()];
    seed(&session, &dir, what, true, &skip)
        .map_err(|e| format!("seeding {}: {e}", dir.display()))?;
    Ok(dir)
}

/// `skip`: the client's `export_dir` and the export's directory, never
/// copied into themselves when they are inside the session's directory.
fn seed(from: &Path, to: &Path, what: Seed, top: bool, skip: &[&Path]) -> std::io::Result<()> {
    let Ok(rd) = std::fs::read_dir(from) else {
        return Ok(());
    };
    for e in rd.flatten() {
        let name = e.file_name();
        let n = name.to_string_lossy();
        let t = e.file_type()?;
        if skip.contains(&e.path().as_path()) {
            continue;
        }
        if t.is_dir() {
            // (made even when empty: TeX writes an `\include`'s `.aux` into
            // its directory and does not make it)
            if matches!(what, Seed::All) || !n.starts_with('.') {
                std::fs::create_dir_all(to.join(&name))?;
                seed(&e.path(), &to.join(&name), what, false, skip)?;
            }
        } else if t.is_file() && !not_seeded(&n) {
            let take = match what {
                Seed::All => true,
                Seed::Job(job) => {
                    n.ends_with(".aux")
                        || (top
                            && n.strip_prefix(job).is_some_and(|r| {
                                r.starts_with('.') && !SOURCES.iter().any(|s| r == *s)
                            }))
                }
            };
            if take {
                std::fs::copy(e.path(), to.join(&name))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The export's directory holds the session's files but its PDF and
    /// log, with their subdirectories; a second export starts from them
    /// again in the same directory; nothing else in `base`, and
    /// nothing of the session's, is touched.
    #[test]
    fn an_export_directory_is_the_sessions_files_again() {
        let d = std::env::temp_dir().join(format!("flashtex-export-dir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let (session, base) = (d.join("out"), d.join("cache"));
        std::fs::create_dir_all(session.join("chapters")).unwrap();
        for (f, t) in [
            ("main.aux", "aux"),
            ("main.toc", "toc"),
            ("main.pdf", "pdf"),
            ("main.log", "log"),
            ("chapters/one.aux", "one"),
        ] {
            std::fs::write(session.join(f), t).unwrap();
        }
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("keep.txt"), "mine").unwrap();
        let dir = prepare(&base, &session, Seed::All).unwrap();
        assert!(dir.starts_with(&base));
        assert_eq!(
            std::fs::read_to_string(dir.join("main.aux")).unwrap(),
            "aux"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("main.toc")).unwrap(),
            "toc"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("chapters/one.aux")).unwrap(),
            "one"
        );
        assert!(!dir.join("main.pdf").exists() && !dir.join("main.log").exists());
        std::fs::write(dir.join("main.pdf"), "exported").unwrap();
        std::fs::write(dir.join("stale.toc"), "x").unwrap();
        let again = prepare(&base, &session, Seed::All).unwrap();
        assert_eq!(again, dir);
        assert!(!again.join("main.pdf").exists() && !again.join("stale.toc").exists());
        assert_eq!(
            std::fs::read_to_string(base.join("keep.txt")).unwrap(),
            "mine"
        );
        assert_eq!(
            std::fs::read_to_string(session.join("main.pdf")).unwrap(),
            "pdf"
        );
        assert!(prepare(Path::new("relative"), &session, Seed::All).is_err());
        // the project's own directory: its `.aux` files and the job's, not
        // its sources
        for f in ["main.tex", "fig.png", "notes.txt", "chapters/one.tex"] {
            std::fs::write(session.join(f), "src").unwrap();
        }
        let dir = prepare(&base, &session, Seed::Job("main")).unwrap();
        for f in ["main.aux", "main.toc", "chapters/one.aux"] {
            assert!(dir.join(f).is_file(), "{f}");
        }
        for f in [
            "main.tex",
            "fig.png",
            "notes.txt",
            "chapters/one.tex",
            "main.log",
        ] {
            assert!(!dir.join(f).exists(), "{f}");
        }
        // an `export_dir` inside the session's directory cannot hold an export
        let inner = session.join("export");
        assert!(prepare(&inner, &session, Seed::All).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn prepare_does_not_remove_the_session_or_a_clients_directory() {
        let d =
            std::env::temp_dir().join(format!("flashtex-export-overlap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let session = d.join(DIR);
        std::fs::create_dir_all(&session).unwrap();
        std::fs::write(session.join("main.aux"), "session").unwrap();
        let other_dir = prepare(&d, &session, Seed::All).unwrap();
        assert_ne!(other_dir, session);
        let nested = session.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        assert_ne!(prepare(&d, &nested, Seed::All).unwrap(), session);
        assert!(prepare(&nested, &session, Seed::All).is_err());
        #[cfg(unix)]
        {
            let alias = d.join("alias");
            std::os::unix::fs::symlink(&d, &alias).unwrap();
            assert_eq!(
                prepare(&alias, &session, Seed::All)
                    .unwrap()
                    .canonicalize()
                    .unwrap(),
                other_dir.canonicalize().unwrap()
            );
        }
        assert_eq!(
            std::fs::read_to_string(session.join("main.aux")).unwrap(),
            "session"
        );

        let other = d.join("other");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("main.aux"), "other").unwrap();
        let dir = prepare(&d, &other, Seed::All).unwrap();
        assert_ne!(dir, session);
        assert_eq!(
            std::fs::read_to_string(session.join("main.aux")).unwrap(),
            "session"
        );
        let unmarked = destination(&d, &other).unwrap();
        std::fs::remove_dir_all(&unmarked).unwrap();
        std::fs::create_dir(&unmarked).unwrap();
        std::fs::write(unmarked.join("mine.txt"), "keep").unwrap();
        assert!(prepare(&d, &other, Seed::All).is_err());
        assert_eq!(
            std::fs::read_to_string(unmarked.join("mine.txt")).unwrap(),
            "keep"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn exports_sharing_a_base_keep_their_own_files() {
        let d =
            std::env::temp_dir().join(format!("flashtex-export-concurrent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let (base, first, second) = (d.join("cache"), d.join("one"), d.join("two"));
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(first.join("main.aux"), "one").unwrap();
        std::fs::write(second.join("main.aux"), "two").unwrap();
        let one = prepare(&base, &first, Seed::All).unwrap();
        std::fs::write(one.join("main.pdf"), "first PDF").unwrap();
        let two = prepare(&base, &second, Seed::All).unwrap();
        assert_ne!(one, two);
        assert_eq!(
            std::fs::read_to_string(one.join("main.pdf")).unwrap(),
            "first PDF"
        );
        assert_eq!(
            std::fs::read_to_string(one.join("main.aux")).unwrap(),
            "one"
        );
        assert_eq!(
            std::fs::read_to_string(two.join("main.aux")).unwrap(),
            "two"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_active_session_export_is_refused_until_it_finishes() {
        let d = std::env::temp_dir().join(format!("flashtex-export-lease-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let session = d.join("session");
        std::fs::create_dir_all(&session).unwrap();
        let lease = reserve(&d, &session).unwrap();
        assert!(reserve(&d, &session)
            .err()
            .unwrap()
            .contains("already running"));
        #[cfg(unix)]
        {
            let alias = d.join("alias");
            std::os::unix::fs::symlink(&session, &alias).unwrap();
            assert!(reserve(&d, &alias)
                .err()
                .unwrap()
                .contains("already running"));
        }
        drop(lease);
        assert!(reserve(&d, &session).is_ok());
        let _ = std::fs::remove_dir_all(&d);
    }
}
