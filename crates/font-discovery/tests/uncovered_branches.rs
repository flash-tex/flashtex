//! Cover the branches the other suites miss: a `ttcf` collection with
//! several faces, `OS/2` weights written 1..9 and out-of-range values,
//! `scan_cached` seeing changed mtimes under an unchanged directory list,
//! and the env-missing arms of `default_dirs`/`env_dirs`. All fonts are
//! synthetic sfnt headers, so nothing here touches a system font.

use flashtex_font_discovery::{default_dirs, sfnt, FontIndex};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static N: AtomicU64 = AtomicU64::new(0);

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn tmp(tag: &str) -> Tmp {
    let n = N.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("flashtex-fd-br-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    Tmp(dir)
}

/// Restores the variable on drop, so a failed assert cannot leak edited
/// env into another test running in this binary.
struct EnvGuard {
    key: &'static str,
    saved: Option<OsString>,
}

impl EnvGuard {
    fn capture(key: &'static str) -> EnvGuard {
        EnvGuard {
            key,
            saved: std::env::var_os(key),
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.saved {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}

/// One synthetic face: an sfnt directory with `name` + `head` and, unless
/// `os2` is `None`, an `OS/2` table of `os2_len` bytes carrying
/// (usWeightClass, usWidthClass, fsSelection).
struct FaceSpec {
    family: &'static str,
    mac_style: u16,
    os2: Option<(u16, u16, u16)>,
    os2_len: usize,
}

fn spec(family: &'static str, os2: Option<(u16, u16, u16)>) -> FaceSpec {
    FaceSpec {
        family,
        mac_style: 0,
        os2,
        os2_len: 64,
    }
}

fn name_table(family: &str) -> Vec<u8> {
    let units: Vec<u16> = family.encode_utf16().collect();
    let mut b = vec![0, 0, 0, 1, 0, 18];
    b.extend_from_slice(&[0, 3, 0, 1, 4, 9, 0, 1]);
    b.extend_from_slice(&((units.len() * 2) as u16).to_be_bytes());
    b.extend_from_slice(&[0, 0]);
    for u in units {
        b.extend_from_slice(&u.to_be_bytes());
    }
    b
}

fn head_table(mac_style: u16) -> Vec<u8> {
    let mut h = vec![0u8; 54];
    h[18..20].copy_from_slice(&1000u16.to_be_bytes());
    h[44..46].copy_from_slice(&mac_style.to_be_bytes());
    h
}

/// A single-face font for a one-element slice, else a `ttcf` collection
/// with one member per spec, in order.
fn build(specs: &[FaceSpec]) -> Vec<u8> {
    let ttc = specs.len() > 1;
    let mut out = Vec::new();
    if ttc {
        out.extend_from_slice(b"ttcf");
        out.extend_from_slice(&0x0001_0000u32.to_be_bytes());
        out.extend_from_slice(&(specs.len() as u32).to_be_bytes());
        out.resize(12 + 4 * specs.len(), 0);
    }
    let mut offsets = Vec::new();
    for s in specs {
        let mut tables: Vec<([u8; 4], Vec<u8>)> = vec![
            (*b"name", name_table(s.family)),
            (*b"head", head_table(s.mac_style)),
        ];
        if let Some((w, wd, sel)) = s.os2 {
            let mut t = vec![0u8; s.os2_len];
            if s.os2_len >= 64 {
                t[4..6].copy_from_slice(&w.to_be_bytes());
                t[6..8].copy_from_slice(&wd.to_be_bytes());
                t[62..64].copy_from_slice(&sel.to_be_bytes());
            }
            tables.push((*b"OS/2", t));
        }
        if ttc {
            offsets.push(out.len() as u32);
        }
        out.extend_from_slice(&0x0001_0000u32.to_be_bytes());
        out.extend_from_slice(&(tables.len() as u16).to_be_bytes());
        out.extend_from_slice(&[0u8; 6]);
        let recs = out.len();
        out.resize(recs + 16 * tables.len(), 0);
        let mut at = out.len() as u32;
        for (i, (tag, payload)) in tables.iter().enumerate() {
            let r = recs + 16 * i;
            out[r..r + 4].copy_from_slice(tag);
            out[r + 8..r + 12].copy_from_slice(&at.to_be_bytes());
            out[r + 12..r + 16].copy_from_slice(&(payload.len() as u32).to_be_bytes());
            out.extend_from_slice(payload);
            at += payload.len() as u32;
        }
    }
    for (i, off) in offsets.iter().enumerate() {
        out[12 + 4 * i..16 + 4 * i].copy_from_slice(&off.to_be_bytes());
    }
    out
}

#[test]
fn ttc_collection_yields_each_face_in_order() {
    let dir = tmp("ttc");
    let path = dir.0.join("two.ttc");
    let bytes = build(&[
        spec("Alpha", Some((400, 5, 0))),
        spec("Beta", Some((700, 5, 0x0021))),
    ]);
    std::fs::write(&path, bytes).unwrap();
    let faces = sfnt::read_faces(&path).unwrap();
    assert_eq!(faces.len(), 2);
    assert_eq!(faces[0].family, "Alpha");
    assert_eq!(
        (faces[0].weight, faces[0].italic, faces[0].bold),
        (400, false, false)
    );
    assert_eq!(faces[1].family, "Beta");
    assert_eq!(
        (faces[1].weight, faces[1].italic, faces[1].bold),
        (700, true, true)
    );
    let index = FontIndex::scan(std::slice::from_ref(&dir.0));
    assert_eq!(index.files().len(), 2);
    assert_eq!(
        (index.files()[0].face_index, index.files()[1].face_index),
        (0, 1)
    );
    assert_eq!(index.find("Beta", 700, true).unwrap().face_index, 1);
}

#[test]
fn os2_weight_scales_1_to_9_and_clamps() {
    let dir = tmp("os2");
    // (raw weight, raw width, expected weight, expected width)
    let cases = [
        (1, 1, 100, 1),
        (5, 5, 500, 5),
        (9, 9, 900, 9),
        (0, 0, 1, 1),
        (10, 10, 10, 9),
        (400, 3, 400, 3),
        (2000, 20, 1000, 9),
    ];
    for (i, (w, wd, ew, ewd)) in cases.iter().enumerate() {
        let p = dir.0.join(format!("w{i}.ttf"));
        std::fs::write(&p, build(&[spec("W", Some((*w, *wd, 0)))])).unwrap();
        let f = &sfnt::read_faces(&p).unwrap()[0];
        assert_eq!(
            (f.weight, f.width),
            (*ew, *ewd),
            "raw weight {w} width {wd}"
        );
    }
    // OBLIQUE alone reads as italic; BOLD as bold.
    let p = dir.0.join("sel.ttf");
    std::fs::write(&p, build(&[spec("S", Some((400, 5, 0x0200)))])).unwrap();
    let f = &sfnt::read_faces(&p).unwrap()[0];
    assert!(f.italic && !f.bold);
    // macStyle is ignored while a full OS/2 table is present ...
    let p = dir.0.join("mac.ttf");
    std::fs::write(
        &p,
        build(&[FaceSpec {
            mac_style: 3,
            ..spec("M", Some((400, 5, 0)))
        }]),
    )
    .unwrap();
    let f = &sfnt::read_faces(&p).unwrap()[0];
    assert_eq!((f.weight, f.italic, f.bold), (400, false, false));
    // ... but a short OS/2 table falls back to it.
    let p = dir.0.join("short.ttf");
    std::fs::write(
        &p,
        build(&[FaceSpec {
            mac_style: 3,
            os2_len: 10,
            ..spec("T", Some((400, 5, 0)))
        }]),
    )
    .unwrap();
    let f = &sfnt::read_faces(&p).unwrap()[0];
    assert_eq!((f.weight, f.width, f.italic, f.bold), (700, 5, true, true));
}

#[test]
fn scan_cached_rescans_when_mtimes_change() {
    let dir = tmp("mtime");
    let cache_dir = tmp("mtime-cache");
    std::fs::write(
        dir.0.join("good.ttf"),
        build(&[spec("Cached", Some((400, 5, 0)))]),
    )
    .unwrap();
    std::fs::write(dir.0.join("broken.otf"), "OTTO but truncated").unwrap();
    // The cache lives outside the scanned dir, so writing it never moves
    // the stamps the second call compares against.
    let cache = cache_dir.0.join("index.json");
    let a = FontIndex::scan_cached(std::slice::from_ref(&dir.0), &cache);
    assert_eq!(a.files().len(), 1);
    assert_eq!(a.errors().len(), 1);
    // Same stamps: a cache hit, and hits carry no errors.
    let b = FontIndex::scan_cached(std::slice::from_ref(&dir.0), &cache);
    assert_eq!(b.files(), a.files());
    assert!(b.errors().is_empty());
    // Same directory list, stale mtime: rescanned (the errors return) and
    // the cache rewritten, so the call after hits again.
    let text = std::fs::read_to_string(&cache).unwrap();
    let key = "\"mtime\":";
    let at = text.find(key).unwrap() + key.len();
    let end = at + text[at..].find(|c: char| !c.is_ascii_digit()).unwrap();
    let stamp: u64 = text[at..end].parse().unwrap();
    let mut tampered = text.clone();
    tampered.replace_range(at..end, &(stamp + 1).to_string());
    std::fs::write(&cache, tampered).unwrap();
    let c = FontIndex::scan_cached(std::slice::from_ref(&dir.0), &cache);
    assert_eq!(c.files(), a.files());
    assert_eq!(c.errors().len(), 1);
    let d = FontIndex::scan_cached(std::slice::from_ref(&dir.0), &cache);
    assert!(d.errors().is_empty());
}

#[test]
fn default_dirs_drops_entries_with_missing_env() {
    let _home = EnvGuard::capture("HOME");
    let _windir = EnvGuard::capture("WINDIR");
    let _local = EnvGuard::capture("LOCALAPPDATA");
    let home = PathBuf::from("/nonexistent-flashtex-home-sentinel");
    let windir = PathBuf::from("/nonexistent-flashtex-windir-sentinel");
    let local = PathBuf::from("/nonexistent-flashtex-local-sentinel");
    std::env::set_var("HOME", &home);
    std::env::set_var("WINDIR", &windir);
    std::env::set_var("LOCALAPPDATA", &local);
    let with = default_dirs();
    let under =
        |d: &PathBuf| d.starts_with(&home) || d.starts_with(&windir) || d.starts_with(&local);
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    assert!(
        with.iter().any(|d| d.starts_with(&home)),
        "HOME should contribute: {with:?}"
    );
    #[cfg(target_os = "windows")]
    assert!(
        with.iter().any(|d| d.starts_with(&windir)) && with.iter().any(|d| d.starts_with(&local)),
        "WINDIR/LOCALAPPDATA should contribute: {with:?}"
    );
    // Unset: only the fixed system entries survive, in order.
    std::env::remove_var("HOME");
    std::env::remove_var("WINDIR");
    std::env::remove_var("LOCALAPPDATA");
    let without = default_dirs();
    assert!(without.iter().all(|d| !under(d)), "{without:?}");
    let system: Vec<PathBuf> = with.iter().filter(|d| !under(d)).cloned().collect();
    assert_eq!(without, system);
    // Empty counts as missing (the `filter(!is_empty)` arm).
    std::env::set_var("HOME", "");
    std::env::set_var("WINDIR", "");
    std::env::set_var("LOCALAPPDATA", "");
    assert_eq!(default_dirs(), without);
}
