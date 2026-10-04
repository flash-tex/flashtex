//! Unit tests: no TeX Live and no system font needed (synthetic sfnt
//! files in a temp directory), except `system_fonts_resolve`, which skips
//! with a printed reason when the machine has none.

use super::sfnt::{self, Bytes, Face, NameRecord};
use super::*;
use std::path::PathBuf;
use std::sync::Arc;

// ---- building synthetic fonts ---------------------------------------------

fn be16(v: u16) -> [u8; 2] {
    v.to_be_bytes()
}

/// An sfnt file from (tag, data) tables.
fn sfnt_file(tables: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let n = tables.len();
    let mut out = Vec::new();
    out.extend(0x0001_0000u32.to_be_bytes());
    out.extend(be16(n as u16));
    out.extend([0u8; 6]);
    let mut off = 12 + 16 * n;
    let mut body = Vec::new();
    for (tag, data) in tables {
        out.extend(*tag);
        out.extend(0u32.to_be_bytes());
        out.extend((off as u32).to_be_bytes());
        out.extend((data.len() as u32).to_be_bytes());
        body.extend(data);
        while body.len() % 4 != 0 {
            body.push(0);
        }
        off = 12 + 16 * n + body.len();
    }
    out.extend(body);
    out
}

/// A format-0 `name` table: (platform, encoding, language, id, string);
/// platform 1 strings are written as Mac Roman bytes (ASCII here), others
/// as UTF-16BE.
fn name_table(recs: &[(u16, u16, u16, u16, &str)]) -> Vec<u8> {
    let mut storage = Vec::new();
    let mut out = Vec::new();
    out.extend(be16(0));
    out.extend(be16(recs.len() as u16));
    out.extend(be16((6 + 12 * recs.len()) as u16));
    for &(p, e, l, id, s) in recs {
        let bytes: Vec<u8> = if p == 1 {
            s.bytes().collect()
        } else {
            s.encode_utf16().flat_map(u16::to_be_bytes).collect()
        };
        for v in [p, e, l, id, bytes.len() as u16, storage.len() as u16] {
            out.extend(be16(v));
        }
        storage.extend(bytes);
    }
    out.extend(storage);
    out
}

fn head_table(mac_style: u16) -> Vec<u8> {
    let mut h = vec![0u8; 54];
    h[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    h[18..20].copy_from_slice(&be16(1000));
    h[44..46].copy_from_slice(&be16(mac_style));
    h
}

fn os2_table(weight: u16, width: u16, fs_selection: u16) -> Vec<u8> {
    let mut t = vec![0u8; 96];
    t[0..2].copy_from_slice(&be16(4));
    t[4..6].copy_from_slice(&be16(weight));
    t[6..8].copy_from_slice(&be16(width));
    t[62..64].copy_from_slice(&be16(fs_selection));
    t
}

fn post_table(italic_angle_deg: f64) -> Vec<u8> {
    let mut t = vec![0u8; 32];
    t[0..4].copy_from_slice(&0x0003_0000u32.to_be_bytes());
    t[4..8].copy_from_slice(&((italic_angle_deg * 65536.0) as i32).to_be_bytes());
    t
}

/// A GPOS 1.0 with a FeatureList of the given (tag, params) features; a
/// params of `None` has no FeatureParams.
fn gpos_table(features: &[(&[u8; 4], Option<[u16; 5]>)]) -> Vec<u8> {
    let mut t = Vec::new();
    t.extend(0x0001_0000u32.to_be_bytes());
    t.extend(be16(0)); // ScriptList
    t.extend(be16(10)); // FeatureList
    t.extend(be16(0)); // LookupList
                       // FeatureList at 10: count, records; feature tables after.
    let n = features.len();
    let mut list = Vec::new();
    list.extend(be16(n as u16));
    let mut tables = Vec::new();
    let base = 2 + 6 * n;
    for (tag, params) in features {
        list.extend(**tag);
        list.extend(be16((base + tables.len()) as u16));
        // Feature: params offset (from the feature), lookup count 0.
        match params {
            Some(p) => {
                tables.extend(be16(4));
                tables.extend(be16(0));
                for v in p {
                    tables.extend(be16(*v));
                }
            }
            None => {
                tables.extend(be16(0));
                tables.extend(be16(0));
            }
        }
    }
    t.extend(list);
    t.extend(tables);
    t
}

/// A face: family/style (ids 1/2 on Windows and Mac), full name, PS name,
/// weight, width, fsSelection, italic angle, optional size params.
struct Spec<'a> {
    family: &'a str,
    style: &'a str,
    full: &'a str,
    ps: &'a str,
    weight: u16,
    width: u16,
    sel: u16,
    angle: f64,
    size: Option<[u16; 5]>,
}

fn face_bytes(s: &Spec) -> Vec<u8> {
    let names = name_table(&[
        (1, 0, 0, 1, s.family),
        (1, 0, 0, 2, s.style),
        (1, 0, 0, 4, s.full),
        (1, 0, 0, 6, s.ps),
        (3, 1, 0x409, 1, s.family),
        (3, 1, 0x409, 2, s.style),
        (3, 1, 0x409, 4, s.full),
        (3, 1, 0x409, 6, s.ps),
    ]);
    let mut tables: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"OS/2", os2_table(s.weight, s.width, s.sel)),
        (b"head", head_table(0)),
        (b"name", names),
        (b"post", post_table(s.angle)),
    ];
    if let Some(p) = s.size {
        tables.insert(0, (b"GPOS", gpos_table(&[(b"size", Some(p))])));
    }
    sfnt_file(&tables)
}

struct TempFonts(PathBuf);

impl TempFonts {
    fn new(tag: &str, specs: &[Spec]) -> TempFonts {
        let dir = std::env::temp_dir().join(format!(
            "flashtex-xetex-fontmgr-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for s in specs {
            std::fs::write(dir.join(format!("{}.otf", s.ps)), face_bytes(s)).unwrap();
        }
        TempFonts(dir)
    }

    fn mgr(&self) -> FontMgr {
        FontMgr::new(Arc::new(FontCatalog::scan(std::slice::from_ref(&self.0))))
    }
}

impl Drop for TempFonts {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const REG: u16 = 1 << 6;
const BOLD: u16 = 1 << 5;
const ITAL: u16 = 1;

fn spec<'a>(
    style: &'a str,
    full: &'a str,
    ps: &'a str,
    weight: u16,
    sel: u16,
    angle: f64,
) -> Spec<'a> {
    Spec {
        family: "Testa",
        style,
        full,
        ps,
        weight,
        width: 5,
        sel,
        angle,
        size: None,
    }
}

fn testa() -> Vec<Spec<'static>> {
    vec![
        spec("Regular", "Testa Regular", "Testa-Regular", 400, REG, 0.0),
        spec("Bold", "Testa Bold", "Testa-Bold", 700, BOLD, 0.0),
        spec("Italic", "Testa Italic", "Testa-Italic", 400, ITAL, -12.0),
        spec(
            "Bold Italic",
            "Testa Bold Italic",
            "Testa-BoldItalic",
            700,
            BOLD | ITAL,
            -12.0,
        ),
        spec("Light", "Testa Light", "Testa-Light", 300, 0, 0.0),
        spec("Black", "Testa Black", "Testa-Black", 900, 0, 0.0),
    ]
}

fn ps_of(m: &FontMgr, loc: &Located) -> String {
    let face = m
        .catalog()
        .faces()
        .iter()
        .position(|f| f.path.to_string_lossy() == loc.path && f.index == loc.face_index)
        .unwrap();
    m.face(face).names.ps_name.clone()
}

fn find(m: &mut FontMgr, name: &str) -> Option<String> {
    m.locate_with(name, 10 << 16, &mut |_, _| None)
        .map(|l| ps_of(m, &l))
}

// ---- splitFontName ----------------------------------------------------------

#[test]
fn split_font_name_cases() {
    let s = |n: &str| {
        let x = split_font_name(n);
        (x.name, x.variant, x.features, x.index)
    };
    let o = |v: &str| Some(v.to_string());
    assert_eq!(s("Times"), ("Times".into(), None, None, 0));
    assert_eq!(s("Times/B"), ("Times".into(), o("B"), None, 0));
    assert_eq!(
        s("Times/BI:+smcp"),
        ("Times".into(), o("BI"), o("+smcp"), 0)
    );
    assert_eq!(
        s("Times:mapping=tex-text;+onum"),
        ("Times".into(), None, o("mapping=tex-text;+onum"), 0)
    );
    // A '/' after the ':' belongs to the features; the first ':' wins.
    assert_eq!(s("Times:a/b:c"), ("Times".into(), None, o("a/b:c"), 0));
    assert_eq!(
        s("Times/B/S=12:x"),
        ("Times".into(), o("B/S=12"), o("x"), 0)
    );
    assert_eq!(s("Times/"), ("Times".into(), o(""), None, 0));
    assert_eq!(s("Times/:"), ("Times".into(), o(""), o(""), 0));
    // The file form: the `]` starts the variant, so even a bare `[file]`
    // has an (empty) one.
    assert_eq!(
        s("[lmroman10-regular.otf]"),
        ("[lmroman10-regular.otf".into(), o(""), None, 0)
    );
    assert_eq!(
        s("[lmroman10-regular.otf]/OT"),
        ("[lmroman10-regular.otf".into(), o("/OT"), None, 0)
    );
    assert_eq!(s("[a.otf]:+smcp"), ("[a.otf".into(), o(""), o("+smcp"), 0));
    assert_eq!(
        s("[a.otf]/GR:x:y"),
        ("[a.otf".into(), o("/GR"), o("x:y"), 0)
    );
    assert_eq!(
        s("[Helvetica.ttc:3]"),
        ("[Helvetica.ttc".into(), o("3]"), None, 3)
    );
    assert_eq!(
        s("[Helvetica.ttc:12]/OT:+kern"),
        ("[Helvetica.ttc".into(), o("12]/OT"), o("+kern"), 12)
    );
    // Inside the brackets a second ':' and a '/' are part of the file name.
    assert_eq!(
        s("[dir/a:b.ttc:1]"),
        ("[dir/a".into(), o("b.ttc:1]"), None, 0)
    );
    assert_eq!(s("[no-close"), ("[no-close".into(), None, None, 0));
    assert!(split_font_name("[a.otf]").is_file() && !split_font_name("a").is_file());
    // UTF-8 names split on the ASCII delimiters.
    assert_eq!(
        s("ヒラギノ角ゴシック W3/B"),
        ("ヒラギノ角ゴシック W3".into(), o("B"), None, 0)
    );
}

// ---- sfnt reading ----------------------------------------------------------

#[test]
fn size_feature_parser_follows_harfbuzz() {
    let gpos = |f: &[(&[u8; 4], Option<[u16; 5]>)]| gpos_table(f);
    let parse = |t: Vec<u8>| sfnt::size_params(&t, 0, t.len() as u64);
    // A design size with a range.
    let p = parse(gpos(&[
        (b"kern", None),
        (b"size", Some([100, 1, 256, 80, 140])),
    ]))
    .unwrap();
    assert_eq!(
        (
            p.design_size,
            p.subfamily_id,
            p.subfamily_name_id,
            p.range_start,
            p.range_end
        ),
        (100, 1, 256, 80, 140)
    );
    // A design size with no range at all is valid.
    assert_eq!(
        parse(gpos(&[(b"size", Some([120, 0, 0, 0, 0]))]))
            .unwrap()
            .design_size,
        120
    );
    // Invalid params (menu name id < 256, size outside the range, design
    // size 0) are neutered: the next 'size' feature is used.
    for bad in [
        [100, 1, 255, 80, 140],
        [100, 1, 256, 110, 140],
        [100, 1, 256, 80, 90],
        [0, 0, 0, 0, 0],
        [100, 1, 40000, 80, 140],
    ] {
        assert_eq!(parse(gpos(&[(b"size", Some(bad))])), None, "{bad:?}");
        let p = parse(gpos(&[
            (b"size", Some(bad)),
            (b"size", Some([90, 0, 0, 0, 0])),
        ]))
        .unwrap();
        assert_eq!(p.design_size, 90);
    }
    // No params, no 'size' feature, another version, truncated tables.
    assert_eq!(parse(gpos(&[(b"size", None)])), None);
    assert_eq!(parse(gpos(&[(b"liga", Some([100, 0, 0, 0, 0]))])), None);
    let mut v2 = gpos(&[(b"size", Some([100, 0, 0, 0, 0]))]);
    v2[0..2].copy_from_slice(&be16(2));
    assert_eq!(parse(v2), None);
    let full = gpos(&[(b"size", Some([100, 0, 0, 0, 0]))]);
    for cut in [8, 13, 20, full.len() - 1] {
        assert_eq!(parse(full[..cut].to_vec()), None, "cut at {cut}");
    }
    // Decipoints to TeX points.
    assert!((decipoints_to_tex(100) - 10.0375).abs() < 1e-12);
}

#[test]
fn name_records_and_postscript_name_follow_freetype() {
    let mut t = name_table(&[
        (3, 1, 0x407, 6, "Ger-Name"),
        (3, 1, 0x409, 6, "Good Name%(x)"),
        (1, 0, 0, 6, "MacName"),
        (3, 1, 0x409, 4, ""),
        (3, 1, 0x409, 1, "Gone"),
    ]);
    // A record pointing outside the storage is dropped: move the last
    // record's string offset past the table.
    let at = 6 + 12 * 4 + 10;
    t[at..at + 2].copy_from_slice(&be16(0x7000));
    let file = sfnt_file(&[(b"head", head_table(0)), (b"name", t)]);
    let face = Face::open(&file, 0).unwrap();
    let recs = face.name_records();
    // The empty record is dropped (FreeType keeps no zero-length string).
    assert_eq!(recs.len(), 3);
    // Windows US English wins over the first Windows record; the space,
    // '%' and parentheses are not PostScript name characters.
    assert_eq!(sfnt::postscript_name(&recs).as_deref(), Some("GoodNamex"));
    // Without a Windows record the Mac one is used.
    let mac: Vec<NameRecord> = recs.iter().filter(|r| r.platform == 1).cloned().collect();
    assert_eq!(sfnt::postscript_name(&mac).as_deref(), Some("MacName"));
    // A string of invalid characters only is no name.
    let bad = vec![NameRecord {
        platform: 3,
        encoding: 1,
        language: 0x409,
        name_id: 6,
        bytes: vec![0, b' ', 0x30, 0x42],
    }];
    assert_eq!(sfnt::postscript_name(&bad), None);
    // A face index past a single font, or a non-font, does not open.
    assert!(Face::open(&file, 1).is_none());
    assert!(Face::open(&b"not a font at all".to_vec(), 0).is_none());
    assert_eq!(sfnt::face_count(&file), Some(1));
    assert_eq!(Bytes::size(&file), file.len() as u64);
}

#[test]
fn read_names_puts_mac_english_first_and_prefers_typographic_names() {
    let mk = |p, e, l, id, s: &str| NameRecord {
        platform: p,
        encoding: e,
        language: l,
        name_id: id,
        bytes: if p == 1 {
            s.bytes().collect()
        } else {
            s.encode_utf16().flat_map(u16::to_be_bytes).collect()
        },
    };
    let recs = vec![
        mk(0, 3, 0, 1, "Uni Family"),
        mk(1, 0, 0, 1, "Mac Family"),
        mk(1, 0, 11, 1, "Mac Japanese"), // Mac, not English: ignored
        mk(3, 1, 0x404, 4, "繁體 全名"),
        mk(3, 1, 0x409, 4, "Full Name"),
        mk(3, 1, 0x409, 1, "Uni Family"), // a duplicate: kept once
        mk(3, 1, 0x409, 2, "Bold"),
        mk(3, 1, 0x409, 17, "Bold Display"),
        mk(3, 1, 0x409, 16, "Typo Family"),
        mk(3, 10, 0x409, 16, "Typo Family"),
        mk(2, 1, 0, 1, "ISO"), // platform 2: ignored
    ];
    let n = catalog::read_names(&recs, Some("PS-Name".into()));
    assert_eq!(n.ps_name, "PS-Name");
    assert_eq!(n.family_names, vec!["Typo Family"]);
    assert_eq!(n.style_names, vec!["Bold Display"]);
    // Table order: the Traditional Chinese record (0x404) before English.
    assert_eq!(n.full_names, vec!["繁體 全名", "Full Name"]);
    let legacy: Vec<NameRecord> = recs
        .iter()
        .filter(|r| r.name_id != 16 && r.name_id != 17)
        .cloned()
        .collect();
    let n = catalog::read_names(&legacy, Some("P".into()));
    assert_eq!(n.family_names, vec!["Mac Family", "Uni Family"]);
    assert_eq!(n.style_names, vec!["Bold"]);
    assert_eq!(catalog::read_names(&recs, None), NameCollection::default());
    let p = catalog::pattern_names(&recs);
    assert!(
        p.families.contains(&"Typo Family".into()) && p.families.contains(&"Mac Japanese".into())
    );
    assert_eq!(
        sfnt::decode_mac_roman(&[b'C', 0x8E, b'a', 0xDB, 0, b'x']),
        "Céa€"
    );
    assert_eq!(
        sfnt::decode_utf16be(&[0, b'A', 0xD8, 0x00, 0]),
        "A\u{FFFD}\u{FFFD}"
    );
}

// ---- the matching rules ----------------------------------------------------

#[test]
fn find_font_tries_full_hyphen_postscript_then_family() {
    let fonts = TempFonts::new("rules", &testa());
    let mut m = fonts.mgr();
    assert_eq!(find(&mut m, "Testa Bold").as_deref(), Some("Testa-Bold")); // full name
    assert_eq!(
        find(&mut m, "Testa-Bold Italic").as_deref(),
        Some("Testa-BoldItalic")
    ); // Family-Style
    assert_eq!(
        find(&mut m, "Testa-BoldItalic").as_deref(),
        Some("Testa-BoldItalic")
    ); // PostScript name
    assert_eq!(find(&mut m, "Testa").as_deref(), Some("Testa-Regular")); // family: the OS/2 regular bit
    assert_eq!(find(&mut m, "Testa Light").as_deref(), Some("Testa-Light"));
    assert_eq!(find(&mut m, "testa"), None); // the maps are case-sensitive
    assert_eq!(find(&mut m, "Nope"), None);
    // The name of file, full name and engine.
    let l = m
        .locate_with("Testa/B/OT:+smcp", -1000, &mut |_, _| None)
        .unwrap();
    assert_eq!(l.full_name.as_deref(), Some("Testa Bold"));
    assert_eq!(l.name_of_file.as_deref(), Some("Testa Bold/OT:+smcp"));
    assert_eq!(
        (l.req_engine, l.variant.as_deref(), l.features.as_deref()),
        (b'O', Some("OT"), Some("+smcp"))
    );
    assert_eq!(
        (l.design_size, l.loaded_font_design_size, l.scaled_size),
        (10.0, 655360, 655360)
    );
    assert!(!l.by_file);
    // `scaled 1200` of a 10pt design size.
    assert_eq!(
        m.locate_with("Testa", -1200, &mut |_, _| None)
            .unwrap()
            .scaled_size,
        786432
    );
    // The variant keeps only engine options; ICU is written OT.
    let v = |m: &mut FontMgr, n: &str| {
        let l = m.locate_with(n, 10 << 16, &mut |_, _| None).unwrap();
        (l.variant, l.req_engine, l.name_of_file)
    };
    assert_eq!(
        v(&mut m, "Testa/ICU/B"),
        (Some("OT".into()), b'O', Some("Testa Bold/OT".into()))
    );
    assert_eq!(
        v(&mut m, "Testa/AAT"),
        (Some("AAT".into()), b'A', Some("Testa Regular/AAT".into()))
    );
    assert_eq!(
        v(&mut m, "Testa/GR/S=9"),
        (Some("GR".into()), b'G', Some("Testa Regular/GR".into()))
    );
    assert_eq!(
        v(&mut m, "Testa/I"),
        (Some(String::new()), 0, Some("Testa Italic".into()))
    );
    assert_eq!(v(&mut m, "Testa"), (None, 0, Some("Testa Regular".into())));
}

#[test]
fn bold_and_italic_requests_move_within_the_family() {
    let fonts = TempFonts::new("bi", &testa());
    let mut m = fonts.mgr();
    assert_eq!(find(&mut m, "Testa/B").as_deref(), Some("Testa-Bold"));
    assert_eq!(find(&mut m, "Testa/I").as_deref(), Some("Testa-Italic"));
    assert_eq!(
        find(&mut m, "Testa/BI").as_deref(),
        Some("Testa-BoldItalic")
    );
    assert_eq!(
        find(&mut m, "Testa/IB").as_deref(),
        Some("Testa-BoldItalic")
    );
    assert_eq!(
        find(&mut m, "Testa Bold/I").as_deref(),
        Some("Testa-BoldItalic")
    );
    // "Times-Italic/I": already slanted, the least slant is upright.
    assert_eq!(
        find(&mut m, "Testa Italic/I").as_deref(),
        Some("Testa-Regular")
    );
    // Bold from Light: weight 300 + (900-300)/2 + 1 = 601, nearest is Bold.
    assert_eq!(find(&mut m, "Testa Light/B").as_deref(), Some("Testa-Bold"));
    // Bold from Bold: 700 + 301 = 1001, nearest is Black.
    assert_eq!(find(&mut m, "Testa Bold/B").as_deref(), Some("Testa-Black"));
    // Black is the heaviest but not flagged bold: the face with the bold
    // flag and the same slant is taken.
    assert_eq!(find(&mut m, "Testa Black/B").as_deref(), Some("Testa-Bold"));
}

#[test]
fn family_lookup_falls_back_to_style_names_then_nearest() {
    // Two faces flagged regular (an ornament font): the "Regular" style wins.
    let mut specs = testa();
    specs.push(spec(
        "Ornaments",
        "Testa Ornaments",
        "Testa-Ornaments",
        400,
        REG,
        0.0,
    ));
    let fonts = TempFonts::new("orn", &specs);
    let mut m = fonts.mgr();
    assert_eq!(find(&mut m, "Testa").as_deref(), Some("Testa-Regular"));
    // No regular bit and no known style name: nearest to (80, 100, 0) by
    // styleDiff, i.e. the lightest upright face.
    let fonts = TempFonts::new(
        "nr",
        &[
            spec("Book", "Testa Book", "Testa-Book", 350, 0, 0.0),
            spec("Heavy", "Testa Heavy", "Testa-Heavy", 800, 0, 0.0),
        ],
    );
    let mut m = fonts.mgr();
    assert_eq!(find(&mut m, "Testa").as_deref(), Some("Testa-Book"));
    // "Plain" when nothing is flagged regular.
    let fonts = TempFonts::new(
        "plain",
        &[
            spec("Plain", "Testa Plain", "Testa-Plain", 500, 0, 0.0),
            spec("Book", "Testa Book", "Testa-Book", 350, 0, 0.0),
        ],
    );
    let mut m = fonts.mgr();
    assert_eq!(find(&mut m, "Testa").as_deref(), Some("Testa-Plain"));
}

#[test]
fn optical_size_selects_within_the_subfamily() {
    let opt = |style: &'static str, ps: &'static str, range: [u16; 5]| Spec {
        family: "Opti",
        style,
        full: style,
        ps,
        weight: 400,
        width: 5,
        sel: if ps.ends_with("Regular") { REG } else { 0 },
        angle: 0.0,
        size: Some(range),
    };
    let specs = vec![
        opt("Caption", "Opti-Caption", [80, 1, 256, 60, 95]),
        opt("Regular", "Opti-Regular", [110, 1, 257, 95, 140]),
        opt("Display", "Opti-Display", [240, 1, 258, 140, 720]),
    ];
    let fonts = TempFonts::new("opsz", &specs);
    let mut m = fonts.mgr();
    let at = |m: &mut FontMgr, name: &str, size: i32| {
        let l = m.locate_with(name, size, &mut |_, _| None).unwrap();
        (ps_of(m, &l), l.loaded_font_design_size)
    };
    let ds = |dp: u16| d2fix(decipoints_to_tex(dp));
    assert_eq!(
        at(&mut m, "Opti", 10 << 16),
        ("Opti-Regular".into(), ds(110))
    );
    assert_eq!(at(&mut m, "Opti", 6 << 16), ("Opti-Caption".into(), ds(80)));
    assert_eq!(
        at(&mut m, "Opti", 24 << 16),
        ("Opti-Display".into(), ds(240))
    );
    // /S= overrides the size; scaled (negative) uses the design size.
    assert_eq!(
        at(&mut m, "Opti/S=20", 6 << 16),
        ("Opti-Display".into(), ds(240))
    );
    assert_eq!(
        at(&mut m, "Opti/S=7.5", 10 << 16),
        ("Opti-Caption".into(), ds(80))
    );
    assert_eq!(at(&mut m, "Opti", -1000).0, "Opti-Regular");
    assert_eq!(at(&mut m, "Opti-Display", -1000).0, "Opti-Display");
    // design_size_of_file reads the 'size' feature directly.
    let l = m.locate_with("Opti/S=20", 0, &mut |_, _| None).unwrap();
    assert_eq!(l.design_size, decipoints_to_tex(240));
}

#[test]
fn the_platform_search_is_lazy_and_falls_back_to_every_font() {
    let mut specs = testa();
    specs.push(Spec {
        family: "Other",
        style: "Regular",
        full: "Other Regular",
        ps: "OtherRoman",
        weight: 400,
        width: 5,
        sel: REG,
        angle: 0.0,
        size: None,
    });
    let fonts = TempFonts::new("lazy", &specs);
    let mut m = fonts.mgr();
    // A family search adds the family only.
    assert!(find(&mut m, "Testa").is_some());
    assert!(m.family("Testa").is_some() && m.family("Other").is_none());
    // A name the search does not find (a PostScript name without a
    // hyphen) makes it add every face, after which the second pass finds it.
    assert_eq!(find(&mut m, "OtherRoman").as_deref(), Some("OtherRoman"));
    assert!(m.family("Other").is_some());
    // A hyphenated name searches the family before the hyphen.
    let mut m = fonts.mgr();
    assert_eq!(find(&mut m, "Testa-Black").as_deref(), Some("Testa-Black"));
    assert!(m.family("Other").is_none());
    assert_eq!(m.family("Testa").unwrap().styles.len(), 6);
    // A second file with an earlier face's PostScript name is skipped, so
    // its own full name finds nothing.
    let dup = spec("Bold", "Testa Bold Copy", "Testa-Bold", 700, BOLD, 0.0);
    std::fs::write(fonts.0.join("dup.otf"), face_bytes(&dup)).unwrap();
    let mut m = fonts.mgr();
    assert_eq!(find(&mut m, "Testa").as_deref(), Some("Testa-Regular"));
    assert_eq!(find(&mut m, "Testa Bold Copy"), None);
    assert_eq!(find(&mut m, "Testa Bold").as_deref(), Some("Testa-Bold"));
}

#[test]
fn file_names_go_through_kpathsea_in_xetex_order() {
    let fonts = TempFonts::new("file", &testa());
    let path = fonts
        .0
        .join("Testa-Bold.otf")
        .to_string_lossy()
        .into_owned();
    let mut asked = Vec::new();
    let mut m = fonts.mgr();
    let l = m
        .locate_with("[Testa-Bold.otf]/AAT:+kern", -1000, &mut |name, fmt| {
            asked.push((name.to_string(), fmt));
            (fmt == Format::Type1).then(|| path.clone())
        })
        .unwrap();
    assert_eq!(
        asked,
        vec![
            ("Testa-Bold.otf".into(), Format::OpenType),
            ("Testa-Bold.otf".into(), Format::TrueType),
            ("Testa-Bold.otf".into(), Format::Type1)
        ]
    );
    assert_eq!(
        (l.path.as_str(), l.face_index, l.by_file, l.req_engine),
        (path.as_str(), 0, true, b'A')
    );
    assert_eq!(
        (
            l.variant.as_deref(),
            l.features.as_deref(),
            l.name_of_file.as_ref()
        ),
        (Some("/AAT"), Some("+kern"), None)
    );
    assert_eq!(l.scaled_size, 655360);
    // `[file:index]` keeps "1]" as its variant, so no engine is requested;
    // a face index past the file's faces finds nothing.
    assert!(m
        .locate_with("[Testa-Bold.otf:1]/OT", 0, &mut |_, _| Some(path.clone()))
        .is_none());
    let l = m
        .locate_with("[Testa-Bold.otf:0]/OT", 0, &mut |_, _| Some(path.clone()))
        .unwrap();
    assert_eq!((l.req_engine, l.variant.as_deref()), (0, Some("0]/OT")));
    assert!(m
        .locate_with("[missing.otf]", 0, &mut |_, _| None)
        .is_none());
}

#[test]
fn arithmetic_helpers_match_c() {
    assert_eq!(d2fix(10.0375), 657818);
    assert_eq!(fix2d(-1000), -1000.0 / 65536.0);
    assert_eq!(xn_over_d(655360, 1200, 1000), 786432);
    assert_eq!(xn_over_d(-655361, 1, 3), -218453);
}

/// A symbolic link to a font file is the face of its target, listed once
/// under the target's path (as Core Text reports it).
#[cfg(unix)]
#[test]
fn a_linked_font_file_is_its_target() {
    let fonts = TempFonts::new("link", &testa()[..1]);
    let target = fonts.0.join("Testa-Regular.otf");
    let sub = fonts.0.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::os::unix::fs::symlink(&target, sub.join("Alias.otf")).unwrap();
    let catalog = FontCatalog::from_files(vec![(sub.join("Alias.otf"), 0), (target.clone(), 0)]);
    assert_eq!(catalog.len(), 1);
    assert_eq!(
        catalog.faces()[0].path,
        std::fs::canonicalize(&target).unwrap()
    );
    let mut m = FontMgr::new(Arc::new(catalog));
    let l = m.locate_with("Testa", 10 << 16, &mut |_, _| None).unwrap();
    assert_eq!(
        std::path::PathBuf::from(l.path),
        std::fs::canonicalize(&target).unwrap()
    );
    // os_extra_dirs only names directories of the system's font service.
    assert!(catalog::os_extra_dirs()
        .iter()
        .all(|d| d.starts_with("/System/Library/AssetsV2")));
}

/// The installed fonts, if any: a smoke test that the catalog reads real
/// files. Skips (with a reason) on a machine without fonts.
#[test]
fn system_fonts_resolve() {
    let catalog = FontCatalog::system(None);
    if catalog.is_empty() {
        eprintln!(
            "skipped: no installed fonts in {:?}",
            flashtex_font_discovery::scan_dirs(None)
        );
        return;
    }
    let catalog = Arc::new(catalog);
    let probe = catalog
        .faces()
        .iter()
        .find(|f| !f.names.full_names.is_empty() && !f.names.ps_name.is_empty())
        .unwrap();
    let full = probe.names.full_names[0].clone();
    let mut m = FontMgr::new(Arc::clone(&catalog));
    let l = m
        .locate_with(&full, 10 << 16, &mut |_, _| None)
        .unwrap_or_else(|| panic!("{full} not found"));
    assert!(std::path::Path::new(&l.path).is_file());
    if cfg!(target_os = "macos")
        && std::path::Path::new("/System/Library/Fonts/Helvetica.ttc").is_file()
    {
        let mut m = FontMgr::new(catalog);
        let l = m
            .locate_with("Helvetica/B", 10 << 16, &mut |_, _| None)
            .unwrap();
        assert_eq!(
            (l.path.as_str(), l.full_name.as_deref()),
            (
                "/System/Library/Fonts/Helvetica.ttc",
                Some("Helvetica Bold")
            )
        );
    }
}
