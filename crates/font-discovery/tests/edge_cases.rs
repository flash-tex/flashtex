//! Edge-case and error-path tests for font discovery. Synthetic cache JSON
//! stands in for real fonts where the branch under test does not read bytes,
//! and tiny hand-built sfnt headers cover the reader's rejections.

use flashtex_font_discovery::json::{self, Value};
use flashtex_font_discovery::sfnt::{self, Outlines};
use flashtex_font_discovery::{env_dirs, scan_dirs, FontIndex};
use std::path::{Path, PathBuf};

static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn tmp(tag: &str) -> Tmp {
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir =
        std::env::temp_dir().join(format!("flashtex-fd-edge-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    Tmp(dir)
}

/// One cache-JSON face object; the index is then built with [`index_of`].
fn face(path: &str, fam: &str, leg: &str, w: u16, wd: u16, it: bool, ps: &str) -> String {
    format!("{{\"path\":{path:?},\"face_index\":0,\"family\":{fam:?},\"legacy_family\":{leg:?},\"subfamily\":\"Regular\",\"full_name\":{fam:?},\"postscript_name\":{ps:?},\"weight\":{w},\"width\":{wd},\"italic\":{it},\"bold\":false,\"math\":false,\"outlines\":\"glyf\",\"units_per_em\":1000,\"revision\":1}}")
}

fn index_of(faces: &[String]) -> FontIndex {
    let text = format!(
        "{{\"version\":1,\"dirs\":[],\"faces\":[{}]}}",
        faces.join(",")
    );
    FontIndex::from_cache_json(&text).expect("synthetic cache json must parse")
}

/// An sfnt header plus table records, no table data.
fn font_bytes(version: u32, tables: &[(&[u8; 4], u32, u32)]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&version.to_be_bytes());
    b.extend_from_slice(&(tables.len() as u16).to_be_bytes());
    b.extend_from_slice(&[0u8; 6]);
    for (tag, off, len) in tables {
        b.extend_from_slice(*tag);
        b.extend_from_slice(&0u32.to_be_bytes());
        b.extend_from_slice(&off.to_be_bytes());
        b.extend_from_slice(&len.to_be_bytes());
    }
    b
}

#[test]
fn display_name_is_stem_plus_collection_index() {
    let idx = index_of(&[face("/fonts/Helvetica.ttc", "H", "H", 400, 5, false, "H")]);
    let mut f = idx.files()[0].clone();
    assert_eq!(f.display_name(), "Helvetica");
    f.face_index = 2;
    assert_eq!(f.display_name(), "Helvetica#2");
    f.path = PathBuf::from("");
    f.face_index = 0;
    assert_eq!(f.display_name(), "");
}

#[test]
fn json_rejects_malformed_input() {
    for bad in [
        "",
        "{",
        "[1,",
        "\"abc",
        "\"a\\",
        "\"\\q\"",
        "\"\\u12\"",
        "\"\\uZZZZ\"",
        "nul",
        "{\"a\" 1}",
        "{\"a\":1,}",
    ] {
        assert!(json::parse(bad).is_err(), "must reject {bad:?}");
    }
}

#[test]
fn json_values_numbers_and_escapes() {
    assert!(Value::Bool(true).get("k").is_none());
    assert!(Value::Num(1.0).as_str().is_none());
    assert!(Value::Str("1".into()).as_u64().is_none());
    assert!(Value::Num(-1.0).as_u64().is_none());
    assert!(Value::Num(1.5).as_u64().is_none());
    assert_eq!(Value::Num(7.0).as_u64(), Some(7));
    assert!(Value::Num(1.0).as_bool().is_none());
    assert!(Value::Null.as_arr().is_none());
    let mut s = String::new();
    json::write(&Value::Num(1.5), &mut s);
    assert_eq!(s, "1.5");
    s.clear();
    json::write(&Value::Num(-2.0), &mut s);
    assert_eq!(s, "-2");
    s.clear();
    json::write(&Value::Str("\u{1}é".into()), &mut s);
    assert_eq!(s, "\"\\u0001é\"");
    assert_eq!(
        json::parse("\"\\u0041\\b\"").unwrap(),
        Value::Str("A\u{8}".into())
    );
    assert_eq!(json::parse("\"\\/\"").unwrap(), Value::Str("/".into()));
}

#[test]
fn cache_json_rejects_wrong_shapes() {
    let good = face("/f.ttf", "Fam", "Fam", 400, 5, false, "Fam");
    let valid = format!("{{\"version\":1,\"dirs\":[],\"faces\":[{good}]}}");
    assert!(FontIndex::from_cache_json(&valid).is_some());
    for bad in [
        "{\"version\":\"1\",\"dirs\":[],\"faces\":[]}".to_string(),
        "{\"version\":1}".to_string(),
        "{\"version\":1,\"dirs\":{},\"faces\":[]}".to_string(),
        "{\"version\":1,\"dirs\":[],\"faces\":{}}".to_string(),
        "{\"version\":1,\"dirs\":[{\"path\":\"/d\"}],\"faces\":[]}".to_string(),
        "{\"version\":1,\"dirs\":[],\"faces\":[{\"path\":\"/f\"}]}".to_string(),
        valid.replace("\"glyf\"", "\"WOFF\""),
        valid.replace("\"weight\":400", "\"weight\":\"400\""),
        valid.replace("\"italic\":false", "\"italic\":0"),
    ] {
        assert!(
            FontIndex::from_cache_json(&bad).is_none(),
            "must reject {bad:?}"
        );
    }
}

#[test]
fn weight_tie_breaks_toward_the_requested_side() {
    let idx = index_of(&[
        face("/a.ttf", "Tie", "Tie", 300, 5, false, "Tie-Light"),
        face("/b.ttf", "Tie", "Tie", 500, 5, false, "Tie-Medium"),
        face("/c.ttf", "Tie", "Tie", 600, 5, false, "Tie-Semi"),
        face("/d.ttf", "Tie", "Tie", 800, 5, false, "Tie-Bold"),
    ]);
    // 400 is equidistant from 300 and 500: at or below 500 the lighter wins.
    assert_eq!(
        idx.find("Tie", 400, false).unwrap().info.postscript_name,
        "Tie-Light"
    );
    // 700 is equidistant from 600 and 800: above 500 the heavier wins.
    assert_eq!(
        idx.find("Tie", 700, false).unwrap().info.postscript_name,
        "Tie-Bold"
    );
}

#[test]
fn matching_prefers_normal_width_and_falls_back_to_legacy_names() {
    let idx = index_of(&[
        face("/narrow.ttf", "Wide", "Wide", 400, 3, false, "Wide-Narrow"),
        face("/normal.ttf", "Wide", "Wide", 400, 5, false, "Wide-Normal"),
        face("/foo.ttf", "Foo", "Foo Light", 300, 5, false, "Foo-Light"),
    ]);
    assert_eq!(
        idx.find("Wide", 400, false).unwrap().info.postscript_name,
        "Wide-Normal"
    );
    // No typographic family "Foo Light": the legacy family still resolves.
    assert_eq!(
        idx.find("Foo Light", 300, false)
            .unwrap()
            .info
            .postscript_name,
        "Foo-Light"
    );
    assert!(idx.find("   ", 400, false).is_none());
}

#[test]
fn families_hides_dot_names_but_find_resolves_them() {
    let idx = index_of(&[
        face("/h.ttf", ".Hidden", ".Hidden", 400, 5, false, "Hidden"),
        face("/v.ttf", "Visible", "Visible", 400, 5, false, "Visible"),
    ]);
    assert_eq!(idx.families(), vec!["Visible"]);
    assert!(idx.find(".Hidden", 400, false).is_some());
    assert!(idx.find(".HIDDEN", 400, false).is_some());
}

#[test]
fn scan_traced_reports_even_a_missing_root() {
    let dir = tmp("traced");
    let missing = dir.0.join("does-not-exist");
    let mut seen = Vec::new();
    let idx = FontIndex::scan_traced(std::slice::from_ref(&missing), &mut |p| {
        seen.push(p.to_path_buf())
    });
    assert_eq!(seen, vec![missing]);
    assert!(idx.files().is_empty() && idx.errors().is_empty());
}

#[test]
fn scan_skips_hidden_accepts_uppercase_and_dedups_roots() {
    let dir = tmp("scan");
    let src =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts/lmroman10-regular.otf");
    std::fs::create_dir_all(dir.0.join("sub")).unwrap();
    std::fs::copy(&src, dir.0.join("UPPER.TTF")).unwrap();
    std::fs::copy(&src, dir.0.join(".hidden.ttf")).unwrap();
    std::fs::copy(&src, dir.0.join("sub").join("lower.otf")).unwrap();
    let a = FontIndex::scan(std::slice::from_ref(&dir.0));
    let names: Vec<String> = a
        .files()
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["UPPER.TTF", "lower.otf"]);
    assert!(a.errors().is_empty());
    // The same root twice still yields each file once.
    let b = FontIndex::scan(&[dir.0.clone(), dir.0.clone()]);
    assert_eq!(b.files(), a.files());
}

#[test]
fn env_dirs_parses_and_scan_dirs_dedups() {
    let key = "FLASHTEX_FONT_DIRS";
    let saved = std::env::var_os(key);
    let sep = if cfg!(windows) { ";" } else { ":" };
    std::env::set_var(key, format!("/a{sep}{sep}/b"));
    assert_eq!(env_dirs(), vec![PathBuf::from("/a"), PathBuf::from("/b")]);
    std::env::set_var(key, "");
    assert!(env_dirs().is_empty());
    let dup = PathBuf::from("/tmp/fd-dedup");
    std::env::set_var(key, format!("{}{sep}{}", dup.display(), dup.display()));
    assert_eq!(scan_dirs(None).iter().filter(|d| *d == &dup).count(), 1);
    match saved {
        Some(v) => std::env::set_var(key, v),
        None => std::env::remove_var(key),
    }
}

#[test]
fn sfnt_rejects_missing_truncated_and_foreign_files() {
    let dir = tmp("sfnt-err");
    assert!(sfnt::read_faces(&dir.0.join("nope.ttf")).is_err());
    let short = dir.0.join("short.ttf");
    std::fs::write(&short, b"OT").unwrap();
    assert!(sfnt::read_faces(&short).is_err());
    let foreign = dir.0.join("foreign.ttf");
    std::fs::write(&foreign, font_bytes(0x5858_5858, &[])).unwrap();
    let e = sfnt::read_faces(&foreign).unwrap_err();
    assert!(e.contains("not a font"), "{e}");
    let empty = dir.0.join("empty.ttf");
    std::fs::write(&empty, font_bytes(0x0001_0000, &[])).unwrap();
    let e = sfnt::read_faces(&empty).unwrap_err();
    assert!(e.contains("0 tables"), "{e}");
    // Claims two tables but the records are missing.
    let mut cut = font_bytes(0x0001_0000, &[]);
    cut[5] = 2;
    let cutf = dir.0.join("cut.ttf");
    std::fs::write(&cutf, cut).unwrap();
    let e = sfnt::read_faces(&cutf).unwrap_err();
    assert!(e.contains("short read"), "{e}");
    // Truncated collection: two faces promised, no offsets.
    let mut ttc = b"ttcf".to_vec();
    ttc.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    ttc.extend_from_slice(&2u32.to_be_bytes());
    let ttcf = dir.0.join("cut.ttc");
    std::fs::write(&ttcf, ttc).unwrap();
    assert!(sfnt::read_faces(&ttcf).is_err());
}

#[test]
fn sfnt_requires_name_and_head() {
    let dir = tmp("sfnt-tab");
    let no_name = dir.0.join("noname.ttf");
    std::fs::write(&no_name, font_bytes(0x0001_0000, &[(b"head", 28, 54)])).unwrap();
    assert!(sfnt::read_faces(&no_name).unwrap_err().contains("name"));
    let no_head = dir.0.join("nohead.ttf");
    std::fs::write(&no_head, font_bytes(0x0001_0000, &[(b"name", 28, 6)])).unwrap();
    assert!(sfnt::read_faces(&no_head).unwrap_err().contains("head"));
    let short_head = dir.0.join("shorthead.ttf");
    std::fs::write(
        &short_head,
        font_bytes(0x0001_0000, &[(b"name", 44, 6), (b"head", 50, 10)]),
    )
    .unwrap();
    assert!(sfnt::read_faces(&short_head).unwrap_err().contains("head"));
}

#[test]
fn sfnt_falls_back_to_mac_style_without_os2() {
    let dir = tmp("sfnt-fb");
    // name table: one Windows Unicode US-English record, id 1 = "Test".
    let mut name = vec![0, 0, 0, 1, 0, 18];
    name.extend_from_slice(&[0, 3, 0, 1, 4, 9, 0, 1, 0, 8, 0, 0]);
    for u in "Test".encode_utf16() {
        name.extend_from_slice(&u.to_be_bytes());
    }
    let mut head = vec![0u8; 54];
    head[18] = 8; // units_per_em 2048
    head[45] = 3; // macStyle bold + italic
    let name_len = name.len() as u32;
    let mut file = font_bytes(
        0x0001_0000,
        &[(b"name", 44, name_len), (b"head", 44 + name_len, 54)],
    );
    file.extend_from_slice(&name);
    file.extend_from_slice(&head);
    let p = dir.0.join("mini.ttf");
    std::fs::write(&p, file).unwrap();
    let faces = sfnt::read_faces(&p).unwrap();
    assert_eq!(faces.len(), 1);
    let f = &faces[0];
    assert_eq!(f.family, "Test");
    assert_eq!((f.weight, f.width, f.italic, f.bold), (700, 5, true, true));
    assert_eq!(f.units_per_em, 2048);
    assert_eq!(f.outlines, Outlines::Glyf);
}
