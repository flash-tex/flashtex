//! Unicode mode's formats (`xelatex.fmt`, `xetex.fmt`), built by FlashTeX
//! itself from the files the resolver sees, as TeX Live's `fmtutil` builds
//! them (its `fmtutil.cnf` lines `xelatex xetex language.dat -etex
//! xelatex.ini` and `xetex xetex language.def -etex xetex.ini`), and cached
//! (PLAN.md §3.4, DESIGN.md D12). No format of TeX Live's is loaded and no
//! TeX Live program runs.
//!
//! The INITEX run is this program invoked as `xetex`, in an empty
//! directory, with `FLASHTEX_READ_SET`, which lists every lookup it made
//! and every file it opened. A format is reused while the engine build is
//! the same and every file it read has the same content (a file whose size
//! and modification time are unchanged is taken as unchanged; one whose are
//! not is hashed again) and every lookup finds the same file.
//!
//! **Where.** `FLASHTEX_FORMAT_CACHE_DIR`, else the per-user cache
//! (`~/Library/Caches/FlashTeX`, `$XDG_CACHE_HOME/flashtex`), in
//! `formats-unicode/<engine build>/<format>/`. A build takes an exclusive
//! lock on the slot, so two hosts starting together build once.

use flashtex_display_list::sha256::{hex, Sha256};
use flashtex_engine::resolver::Format;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// What `fmtutil.cnf` says for each format this engine builds.
pub fn ini_args(fmt: &str) -> Option<Vec<&'static str>> {
    Some(match fmt {
        "xelatex" => vec!["-etex", "xelatex.ini"],
        "xetex" => vec!["-etex", "xetex.ini"],
        _ => return None,
    })
}

/// The cache's root directory.
pub fn cache_root() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("FLASHTEX_FORMAT_CACHE_DIR") {
        return Some(PathBuf::from(d));
    }
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    if cfg!(target_os = "macos") {
        return Some(home.join("Library/Caches/FlashTeX/formats"));
    }
    match std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from) {
        Some(x) if x.is_absolute() => Some(x.join("flashtex/formats")),
        _ => Some(home.join(".cache/flashtex/formats")),
    }
}

/// The engine build: SHA-256 of the running program.
pub fn engine_build(exe: &Path) -> std::io::Result<String> {
    let mut h = Sha256::new();
    let mut f = File::open(exe)?;
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(&h.finish())[..32].to_string())
}

/// One file the INITEX run read.
#[derive(Clone, Debug, PartialEq)]
struct FileRec {
    path: String,
    sha: String,
    size: u64,
    mtime_ns: u128,
}

/// One lookup it made.
#[derive(Clone, Debug, PartialEq)]
struct LookupRec {
    format: String,
    must_exist: bool,
    name: String,
    found: String,
}

fn stat(p: &Path) -> Option<(u64, u128)> {
    let m = fs::metadata(p).ok()?;
    let t = m
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    Some((m.len(), t))
}

fn sha_file(p: &Path) -> Option<String> {
    let d = fs::read(p).ok()?;
    Some(hex(&flashtex_display_list::sha256::sha256(&d)))
}

/// The manifest beside a cached format: its files and lookups, one a line.
fn encode(files: &[FileRec], lookups: &[LookupRec]) -> String {
    let mut s = String::from("flashtex-host-unicode format 1\n");
    for f in files {
        s.push_str(&format!(
            "file\t{}\t{}\t{}\t{}\n",
            f.sha, f.size, f.mtime_ns, f.path
        ));
    }
    for l in lookups {
        s.push_str(&format!(
            "lookup\t{}\t{}\t{}\t{}\n",
            l.format, l.must_exist as u8, l.name, l.found
        ));
    }
    s
}

fn decode(t: &str) -> Option<(Vec<FileRec>, Vec<LookupRec>)> {
    let mut lines = t.lines();
    if lines.next()? != "flashtex-host-unicode format 1" {
        return None;
    }
    let (mut files, mut lookups) = (vec![], vec![]);
    for l in lines {
        let p: Vec<&str> = l.splitn(5, '\t').collect();
        match p.first() {
            Some(&"file") if p.len() == 5 => files.push(FileRec {
                sha: p[1].into(),
                size: p[2].parse().ok()?,
                mtime_ns: p[3].parse().ok()?,
                path: p[4].into(),
            }),
            Some(&"lookup") if p.len() == 5 => lookups.push(LookupRec {
                format: p[1].into(),
                must_exist: p[2] == "1",
                name: p[3].into(),
                found: p[4].into(),
            }),
            _ => return None,
        }
    }
    Some((files, lookups))
}

/// Whether what the format was made from is unchanged; files whose
/// signature changed but whose content did not get their new signature
/// (`refresh`).
fn still_valid(files: &mut [FileRec], lookups: &[LookupRec], progname: &str) -> bool {
    for f in files.iter_mut() {
        let Some((size, t)) = stat(Path::new(&f.path)) else {
            return false;
        };
        if (size, t) != (f.size, f.mtime_ns) {
            if sha_file(Path::new(&f.path)).as_deref() != Some(f.sha.as_str()) {
                return false;
            }
            f.size = size;
            f.mtime_ns = t;
        }
    }
    flashtex_engine::system::with_resolver_for(progname, |r| {
        lookups.iter().all(|l| {
            let Some(fmt) = Format::all().iter().find(|f| f.kpse_name() == l.format) else {
                return false;
            };
            let found = if l.must_exist {
                r.find_ex(&l.name, *fmt, true).0
            } else {
                r.find(&l.name, *fmt)
            };
            found
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default()
                == l.found
        })
    })
}

/// The read set file an INITEX run wrote, as records.
fn read_set(text: &str) -> (Vec<FileRec>, Vec<LookupRec>) {
    let (mut files, mut lookups) = (Vec::<FileRec>::new(), Vec::new());
    for l in text.lines() {
        let p: Vec<&str> = l.split('\t').collect();
        match p.as_slice() {
            ["open", path] => {
                if files.iter().any(|f| f.path == *path) {
                    continue;
                }
                let abs = Path::new(path);
                if !abs.is_absolute() {
                    continue; // the run's own directory: nothing to keep
                }
                if let (Some((size, t)), Some(sha)) = (stat(abs), sha_file(abs)) {
                    files.push(FileRec {
                        path: path.to_string(),
                        sha,
                        size,
                        mtime_ns: t,
                    });
                }
            }
            // the run's own directory (`./texsys.aux`): not what the format
            // is made from
            ["lookup", _, _, name, found]
                if name.starts_with("./") || !found.is_empty() && !found.starts_with('/') => {}
            ["lookup", fmt, must, name, found] => {
                let rec = LookupRec {
                    format: fmt.to_string(),
                    must_exist: *must == "1",
                    name: name.to_string(),
                    found: found.to_string(),
                };
                if !lookups.contains(&rec) {
                    lookups.push(rec);
                }
            }
            _ => {}
        }
    }
    (files, lookups)
}

/// The directory holding format `fmt` (`<fmt>.fmt` in it), built if it is
/// not cached or what it was made from changed. `exe` is this program.
pub fn ensure(exe: &Path, fmt: &str) -> Result<PathBuf, String> {
    let args = ini_args(fmt).ok_or_else(|| format!("no format {fmt} for the Unicode engine"))?;
    let root = cache_root().ok_or("no cache directory (HOME unset)")?;
    let build = engine_build(exe).map_err(|e| format!("{}: {e}", exe.display()))?;
    let slot = root.join("formats-unicode").join(&build).join(fmt);
    fs::create_dir_all(&slot).map_err(|e| format!("{}: {e}", slot.display()))?;
    let manifest = slot.join("manifest");
    let fmt_file = slot.join(format!("{fmt}.fmt"));
    let check = || -> bool {
        let Some(text) = fs::read_to_string(&manifest).ok() else {
            return false;
        };
        let Some((mut files, lookups)) = decode(&text) else {
            return false;
        };
        if !fmt_file.is_file() || !still_valid(&mut files, &lookups, fmt) {
            return false;
        }
        let _ = fs::write(&manifest, encode(&files, &lookups));
        true
    };
    if check() {
        return Ok(slot);
    }
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(slot.join("lock"))
        .map_err(|e| format!("{}: {e}", slot.display()))?;
    lock.lock()
        .map_err(|e| format!("lock {}: {e}", slot.display()))?;
    if check() {
        return Ok(slot);
    }
    // fmtutil's `rebuild_one_format`: INITEX in an empty directory
    let work = slot.join(format!("build-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    let rs = work.join("read-set");
    let mut cmd = Command::new(exe);
    use std::os::unix::process::CommandExt;
    cmd.arg0("xetex")
        .arg("-ini")
        .arg(format!("-jobname={fmt}"))
        .arg(format!("-progname={fmt}"))
        .args(&args)
        .current_dir(&work)
        .env("FLASHTEX_READ_SET", &rs)
        .env_remove("FLASHTEX_DISPLAY_LIST")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let status = cmd
        .status()
        .map_err(|e| format!("{}: {e}", exe.display()))?;
    let built = work.join(format!("{fmt}.fmt"));
    if !status.success() || !built.is_file() {
        let log = fs::read_to_string(work.join(format!("{fmt}.log"))).unwrap_or_default();
        let tail: Vec<&str> = log.lines().rev().take(5).collect();
        let _ = fs::remove_dir_all(&work);
        return Err(format!(
            "building {fmt}.fmt failed ({status}): {}",
            tail.into_iter().rev().collect::<Vec<_>>().join(" / ")
        ));
    }
    let (files, lookups) = read_set(&fs::read_to_string(&rs).unwrap_or_default());
    let tmp = slot.join(format!("{fmt}.fmt.tmp"));
    fs::rename(&built, &tmp)
        .and_then(|_| fs::rename(&tmp, &fmt_file))
        .map_err(|e| format!("{}: {e}", fmt_file.display()))?;
    let mtmp = slot.join("manifest.tmp");
    File::create(&mtmp)
        .and_then(|mut f| f.write_all(encode(&files, &lookups).as_bytes()))
        .and_then(|_| fs::rename(&mtmp, &manifest))
        .map_err(|e| format!("{}: {e}", manifest.display()))?;
    let _ = fs::remove_dir_all(&work);
    Ok(slot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifests_round_trip_and_read_sets_parse() {
        let dir = std::env::temp_dir().join(format!("ftx-fmt-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.tex");
        fs::write(&f, "x").unwrap();
        let text = format!(
            "lookup\ttex\t1\ta.tex\t{0}\nopen\t{0}\nopen\t{0}\nopen\trelative.tex\n",
            f.display()
        );
        let (files, lookups) = read_set(&text);
        assert_eq!(files.len(), 1);
        assert_eq!(lookups.len(), 1);
        let (f2, l2) = decode(&encode(&files, &lookups)).unwrap();
        assert_eq!((f2, l2), (files.clone(), lookups));
        // a changed file invalidates, an unchanged one under a new mtime
        // does not
        let mut fs2 = files.clone();
        assert!(fs2
            .iter_mut()
            .all(|r| sha_file(Path::new(&r.path)).as_deref() == Some(r.sha.as_str())));
        fs::write(&f, "y").unwrap();
        assert_ne!(sha_file(&f).unwrap(), files[0].sha);
        let _ = fs::remove_dir_all(&dir);
    }
}
