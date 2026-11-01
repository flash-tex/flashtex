//! The fonts' catalog kept between runs (lane M3-FAST).
//!
//! A run that looks a font up by name builds [`FontCatalog::system_cached`]:
//! `font-discovery`'s walk of the font directories, which opens every font
//! file for its faces, then each face's names (from [`NameCache`] when the
//! file is unchanged). On ubuntu-latest with TeX Live's font trees that is
//! about 490 M instructions a run, and the opening of every file (run
//! 38202203143, bench b2). Its result depends only on:
//!
//! * the directory list;
//! * which directories the walk lists and what is in each: a directory's
//!   entries are unchanged while its device, inode and modification time
//!   are (adding, removing or renaming an entry, a symbolic link included,
//!   changes the modification time);
//! * each font file the walk visits: unchanged while its device, inode,
//!   size, modification and change times are (the change time is set by
//!   the system on every write and cannot be set back).
//!
//! So the catalog is written down with those identities, and a later run
//! whose identities are all equal takes the catalog from the file instead of
//! walking and reading. Anything else is a new scan, written down anew. A
//! scan is not written down when one of the identities' times is within
//! [`RACY_NS`] of the scan's start or later: something written in the same
//! clock tick as the scan read it could change again without its times
//! changing (git's "racy" rule, as `fmtimage.rs` follows it).

use super::{CatalogFace, FontCatalog, NameCollection, PatternNames};
use std::path::{Path, PathBuf};

/// The file's first line; another version is ignored.
const VERSION: &str = "flashtex-xetex font catalog 1";

/// How close to the scan's start a recorded time may be.
const RACY_NS: i128 = 2_000_000_000;

fn ns(s: i64, n: i64) -> i128 {
    i128::from(s) * 1_000_000_000 + i128::from(n)
}

/// Nanoseconds since the epoch, now.
pub(super) fn now_ns() -> i128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as i128)
}

/// A directory's identity (followed through links, as the walk lists it),
/// and the newest of its times.
pub(super) fn dir_stamp(p: &Path) -> (String, i128) {
    use std::os::unix::fs::MetadataExt;
    match std::fs::metadata(p) {
        Ok(m) if m.is_dir() => (
            format!(
                "d:{}:{}:{}",
                m.dev(),
                m.ino(),
                ns(m.mtime(), m.mtime_nsec())
            ),
            ns(m.mtime(), m.mtime_nsec()),
        ),
        Ok(m) => (
            format!("x:{}:{}", m.dev(), m.ino()),
            ns(m.mtime(), m.mtime_nsec()).max(ns(m.ctime(), m.ctime_nsec())),
        ),
        Err(_) => ("missing".into(), 0),
    }
}

/// A font file's identity (followed through links), and the newest of its
/// times.
pub(super) fn file_stamp(p: &Path) -> (String, i128) {
    use std::os::unix::fs::MetadataExt;
    match std::fs::metadata(p) {
        Ok(m) => {
            let (mt, ct) = (ns(m.mtime(), m.mtime_nsec()), ns(m.ctime(), m.ctime_nsec()));
            (
                format!("f:{}:{}:{}:{mt}:{ct}", m.dev(), m.ino(), m.size()),
                mt.max(ct),
            )
        }
        Err(_) => ("missing".into(), 0),
    }
}

/// `s` with `\`, tab, newline and the unit separator escaped.
fn esc(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{1f}' => out.push_str("\\u"),
            c => out.push(c),
        }
    }
}

fn unesc(s: &str) -> Option<String> {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        out.push(match it.next()? {
            '\\' => '\\',
            't' => '\t',
            'n' => '\n',
            'u' => '\u{1f}',
            _ => return None,
        });
    }
    Some(out)
}

fn esc_list(v: &[String], out: &mut String) {
    out.push_str(&v.len().to_string());
    for s in v {
        out.push('\u{1f}');
        esc(s, out);
    }
}

fn unesc_list(s: &str) -> Option<Vec<String>> {
    let mut parts = s.split('\u{1f}');
    let n: usize = parts.next()?.parse().ok()?;
    let v: Vec<String> = parts.map(unesc).collect::<Option<_>>()?;
    (v.len() == n).then_some(v)
}

/// [`FontCatalog::system_cached`]'s catalog of `dirs` (names through the
/// name cache at `name_cache`): the one kept beside the name cache when it
/// is still valid, else a new scan, kept.
pub(super) fn system(name_cache: &Path, dirs: &[PathBuf]) -> FontCatalog {
    let path = name_cache.with_file_name("xetex-font-catalog.txt");
    if let Some(c) = load(&path, dirs) {
        return c;
    }
    let start = now_ns();
    let mut walked = Vec::new();
    let (catalog, index) = FontCatalog::scan_named(name_cache, dirs, &mut |d| {
        walked.push((d.to_path_buf(), dir_stamp(d)));
    });
    // every font file the walk visited: those with faces and those it could
    // not read
    let mut files: Vec<PathBuf> = Vec::new();
    for p in index
        .files()
        .iter()
        .map(|f| &f.path)
        .chain(index.errors().iter().map(|(p, _)| p))
    {
        if files.last() != Some(p) {
            files.push(p.clone());
        }
    }
    save(&path, dirs, &walked, &files, &catalog, start);
    catalog
}

/// The catalog written down at `path` for directory list `dirs`, if every
/// identity it was written with is unchanged.
pub(super) fn load(path: &Path, dirs: &[PathBuf]) -> Option<FontCatalog> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut lines = text.lines();
    if lines.next()? != VERSION {
        return None;
    }
    let mut roots = Vec::new();
    let mut faces = Vec::new();
    for l in lines {
        let f: Vec<&str> = l.split('\t').collect();
        match f.as_slice() {
            ["root", p] => roots.push(PathBuf::from(unesc(p)?)),
            ["dir", p, st] => {
                if dir_stamp(Path::new(&unesc(p)?)).0 != *st {
                    return None;
                }
            }
            ["file", p, st] => {
                if file_stamp(Path::new(&unesc(p)?)).0 != *st {
                    return None;
                }
            }
            ["face", p, i, pf, pa, ps, fa, st, fu, psn] => {
                let pattern = PatternNames {
                    full_names: unesc_list(pf)?,
                    families: unesc_list(pa)?,
                    styles: unesc_list(ps)?,
                };
                let names = NameCollection {
                    family_names: unesc_list(fa)?,
                    style_names: unesc_list(st)?,
                    full_names: unesc_list(fu)?,
                    ps_name: unesc(psn)?,
                };
                faces.push(CatalogFace::from_cache(
                    PathBuf::from(unesc(p)?),
                    i.parse().ok()?,
                    pattern,
                    names,
                ));
            }
            ["end"] => {
                return (roots == dirs).then(|| FontCatalog::from_faces(faces));
            }
            _ => return None,
        }
    }
    // no `end` line: a file cut short
    None
}

/// Write `catalog` down at `path`: the scan of `dirs`, which started at
/// `start` (nanoseconds) and listed `walked` (each directory with its
/// identity taken just before it was listed) and visited `files`. Not
/// written when an identity is racy or a path is not UTF-8.
pub(super) fn save(
    path: &Path,
    dirs: &[PathBuf],
    walked: &[(PathBuf, (String, i128))],
    files: &[PathBuf],
    catalog: &FontCatalog,
    start: i128,
) {
    let racy = |t: i128| t >= start - RACY_NS;
    let mut out = String::from(VERSION);
    out.push('\n');
    let put = |kind: &str, fields: &[&str], out: &mut String| {
        out.push_str(kind);
        for f in fields {
            out.push('\t');
            out.push_str(f);
        }
        out.push('\n');
    };
    let e = |p: &Path| -> Option<String> {
        let mut s = String::new();
        esc(p.to_str()?, &mut s);
        Some(s)
    };
    for d in dirs {
        let Some(p) = e(d) else { return };
        put("root", &[&p], &mut out);
    }
    for (d, (st, t)) in walked {
        let Some(p) = e(d) else { return };
        if racy(*t) {
            return;
        }
        put("dir", &[&p, st], &mut out);
    }
    for f in files {
        let (st, t) = file_stamp(f);
        let Some(p) = e(f) else { return };
        if racy(t) {
            return;
        }
        put("file", &[&p, &st], &mut out);
    }
    for face in catalog.faces() {
        let Some(p) = e(&face.path) else { return };
        let list = |v: &[String]| {
            let mut s = String::new();
            esc_list(v, &mut s);
            s
        };
        let mut psn = String::new();
        esc(&face.names.ps_name, &mut psn);
        put(
            "face",
            &[
                &p,
                &face.index.to_string(),
                &list(&face.pattern.full_names),
                &list(&face.pattern.families),
                &list(&face.pattern.styles),
                &list(&face.names.family_names),
                &list(&face.names.style_names),
                &list(&face.names.full_names),
                &psn,
            ],
            &mut out,
        );
    }
    put("end", &[], &mut out);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    if std::fs::write(&tmp, out).is_ok() && std::fs::rename(&tmp, path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(a: &FontCatalog, b: &FontCatalog) {
        assert_eq!(a.len(), b.len());
        for (x, y) in a.faces().iter().zip(b.faces()) {
            assert_eq!((&x.path, x.index), (&y.path, y.index));
            assert_eq!(x.names, y.names);
            assert_eq!(x.pattern, y.pattern);
        }
    }

    /// A kept catalog is the scan's; it is not used once a font file or a
    /// directory the scan listed changes, or for another directory list;
    /// a racy scan is not kept.
    #[test]
    fn a_kept_catalog_is_the_scan_until_something_changes() {
        let fonts: Vec<PathBuf> = [
            "/System/Library/Fonts/Helvetica.ttc",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
        ]
        .iter()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect();
        if fonts.is_empty() {
            eprintln!("skipped: no system fonts to copy");
            return;
        }
        let dir = std::env::temp_dir().join(format!("xetex-kept-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("fonts");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        for (i, f) in fonts.iter().enumerate() {
            let to = root.join(if i % 2 == 0 { "sub" } else { "" });
            std::fs::copy(f, to.join(f.file_name().unwrap())).unwrap();
        }
        std::fs::write(root.join("sub/broken.ttf"), b"not a font").unwrap();
        let names = dir.join("names.json");
        let kept = dir.join("catalog.txt");
        let dirs = vec![root.clone(), dir.join("absent")];
        let scan = |kept_at: i128| {
            let mut walked = Vec::new();
            let (c, index) = FontCatalog::scan_named(&names, &dirs, &mut |d| {
                walked.push((d.to_path_buf(), dir_stamp(d)));
            });
            let mut files: Vec<PathBuf> = index.files().iter().map(|f| f.path.clone()).collect();
            files.extend(index.errors().iter().map(|(p, _)| p.clone()));
            files.dedup();
            save(&kept, &dirs, &walked, &files, &c, kept_at);
            c
        };
        // just copied: racy, not kept
        let first = scan(now_ns());
        assert!(!kept.exists());
        assert!(first.len() >= fonts.len());
        // a scan whose start is well after every time: kept, and loads as it
        let c = scan(now_ns() + 10 * RACY_NS);
        same(&first, &c);
        same(&c, &load(&kept, &dirs).expect("kept"));
        // another directory list
        assert!(load(&kept, &dirs[..1]).is_none());
        // a font file written again (its change time moves)
        let f = root.join("sub").join(fonts[0].file_name().unwrap());
        let bytes = std::fs::read(&f).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&f, &bytes).unwrap();
        assert!(load(&kept, &dirs).is_none());
        let c = scan(now_ns() + 10 * RACY_NS);
        same(&first, &c);
        assert!(load(&kept, &dirs).is_some());
        // a file added to a directory the scan listed
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(root.join("sub/new.txt"), b"x").unwrap();
        assert!(load(&kept, &dirs).is_none());
        // a directory that was missing appears
        let c = scan(now_ns() + 10 * RACY_NS);
        assert!(load(&kept, &dirs).is_some());
        std::fs::create_dir_all(dir.join("absent")).unwrap();
        assert!(load(&kept, &dirs).is_none());
        same(&first, &c);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn escapes_round_trip() {
        for s in ["", "a", "a\tb\nc\\d\u{1f}e", "\\t", "Ünïcödé"] {
            let mut e = String::new();
            esc(s, &mut e);
            assert!(!e.contains(['\t', '\n', '\u{1f}']));
            assert_eq!(unesc(&e).as_deref(), Some(s));
        }
        for v in [
            vec![],
            vec!["".to_string()],
            vec!["a\u{1f}".into(), "b".into()],
        ] {
            let mut e = String::new();
            esc_list(&v, &mut e);
            assert_eq!(unesc_list(&e), Some(v));
        }
    }
}
