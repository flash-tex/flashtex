//! Old artifacts are retained; compare semantic identities before new snapshots.
//!
//! The retained OLD one-page PDFs predate two deliberate, reviewed pdf-crate
//! changes, both gated by that crate's own tests: /W widths written from the
//! display list's advances where hmtx differs (4893f3e7f), and coordinates
//! capped at 7 fractional digits (2a9936f3b #729). Exact byte and
//! exact-rational replay equality with the old artifacts can therefore no
//! longer hold by design. What the subsetter itself must preserve — glyph
//! identities, absolute origins within print precision, Unicode maps, /W
//! width tables, glyph programs, and smaller size — is asserted here with
//! tolerance. Exact bytes of the new snapshots are pinned separately by the
//! byte-exact goldens in math_reference and original_reference.
use flashtex_pdf::{cff::CffFont, compare::font_from_dict, exact::*, reader::PdfFile};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
};

/// Origins replayed from 7-fractional-digit tokens differ from the retained
/// 20-digit readings by at most 5e-8; gate two orders of magnitude above that
/// and still six orders below a pixel.
const ORIGIN_EPS: f64 = 1e-6;

/// /W entries are 1000/em units. The one deliberate width change this test
/// tolerates (hmtx -> display-advance /W, 4893f3e7f) moves individual entries
/// by at most 81.8 across all six pairs (display-math/F2 CID 118: 517 ->
/// 435.196), while pinning a shown CID to the default width 1000 moves
/// typical text glyphs by 184+. Gate just above the measured 81.8 so a
/// corruption in the old 81.8-100 window still fails.
const WIDTH_EPS: f64 = 85.0;

/// Bomb guards reinstated from the old compare(): these one-page fixtures are
/// kilobytes, so anything near either cap is not a fixture.
const MAX_FIXTURE_BYTES: usize = 4 * 1024 * 1024;
const MAX_FIXTURE_OPERATORS: usize = 10_000;

struct Snapshot {
    frames: Vec<GlyphPosition>,
    fonts: BTreeMap<String, CidFont>,
}

/// Replay `pdf`'s page against `widths` — the NEW snapshot's /W tables, so
/// the old-vs-new origin comparison isolates coordinate changes instead of
/// baking each side's own width table into its geometry. Entries the new
/// table omits fall back to this PDF's own table so a narrowed subset still
/// reports its precise /W key-set error below instead of a replay failure.
fn load(pdf: &[u8], widths: &BTreeMap<String, CidFont>) -> Result<Snapshot, Box<dyn Error>> {
    if pdf.len() > MAX_FIXTURE_BYTES {
        return Err("fixture byte cap".into());
    }
    let file = PdfFile::parse(pdf)?;
    let pages = file.pages()?;
    if pages.len() != 1 {
        return Err("one-page fixture required".into());
    }
    let fonts: BTreeMap<_, _> = file
        .page_fonts(pages[0])
        .into_iter()
        .map(|(name, dict)| {
            let ExactFont::CidCff(font) = font_from_dict(&file, dict)? else {
                return Err("CID CFF fixture required".into());
            };
            Ok((name, font))
        })
        .collect::<Result<_, Box<dyn Error>>>()?;
    let ops = flashtex_pdf::exact::parse(&file.page_content(pages[0])?)?;
    if ops.len() > MAX_FIXTURE_OPERATORS {
        return Err("fixture operator cap".into());
    }
    let frames = glyph_positions(&ops, &|_| true, &|name, gid| {
        widths
            .get(name)
            .and_then(|f| f.widths.get(&gid))
            .or_else(|| fonts.get(name).and_then(|f| f.widths.get(&gid)))
            .map(Ratio::from_decimal)
    })?;
    Ok(Snapshot { frames, fonts })
}

fn check(old: &[u8], new: &[u8]) -> Result<(), Box<dyn Error>> {
    let b = load(new, &BTreeMap::new())?;
    let a = load(old, &b.fonts)?;
    check_snapshots(&a, &b)?;
    if new.len() >= old.len() {
        return Err("subset must stay smaller than the retained artifact".into());
    }
    Ok(())
}

fn check_snapshots(a: &Snapshot, b: &Snapshot) -> Result<(), Box<dyn Error>> {
    if a.frames.is_empty() || b.frames.is_empty() {
        return Err("no positioned glyphs to compare".into());
    }
    if a.frames.len() != b.frames.len() {
        return Err("positioned glyph count changed".into());
    }
    let ratio = |q: &Ratio| q.num as f64 / q.den as f64;
    for (index, (g, n)) in a.frames.iter().zip(b.frames.iter()).enumerate() {
        if g.font != n.font || g.code != n.code {
            return Err(format!("glyph {index} identity changed").into());
        }
        let dx = (ratio(&g.x) - ratio(&n.x)).abs();
        let dy = (ratio(&g.y) - ratio(&n.y)).abs();
        if dx > ORIGIN_EPS || dy > ORIGIN_EPS {
            return Err(format!("glyph {index} moved by ({dx},{dy})").into());
        }
    }
    let mut used: BTreeMap<String, BTreeSet<u16>> = BTreeMap::new();
    for g in a.frames.iter().chain(&b.frames) {
        used.entry(g.font.clone()).or_default().insert(g.code);
    }
    for (name, gids) in used {
        let (Some(oldfont), Some(newfont)) = (a.fonts.get(&name), b.fonts.get(&name)) else {
            return Err("CID CFF fixture required".into());
        };
        let cmap = |f: &CidFont| -> Result<_, String> {
            match &f.to_unicode_verbatim {
                Some(bytes) => parse_to_unicode(bytes),
                None => Ok(f.to_unicode.clone()),
            }
        };
        if cmap(oldfont)? != cmap(newfont)? {
            return Err("Unicode map changed".into());
        }
        check_widths(&name, oldfont, newfont)?;
        let oldcff = CffFont::parse(oldfont.program.bytes()).map_err(|e| format!("{e:?}"))?;
        let newcff = CffFont::parse(newfont.program.bytes()).map_err(|e| format!("{e:?}"))?;
        for gid in &gids {
            let at = |cff: &CffFont, gid: u16| -> Result<Vec<u8>, String> {
                let index = (0..cff.glyph_count())
                    .find(|i| cff.charset_entry(*i) == Some(gid))
                    .ok_or_else(|| "missing original CID in subset charset".to_string())?;
                cff.expanded_charstring(index)
                    .map_err(|e| format!("{e:?}"))
            };
            if at(&oldcff, *gid)? != at(&newcff, *gid)? {
                return Err("expanded original glyph charstring changed".into());
            }
        }
    }
    Ok(())
}

/// Direct /W comparison. The old blanket `old.widths != new.widths` check
/// broke on the deliberate hmtx -> display-advance change, but asserting
/// nothing lets a corrupted width table (e.g. every shown CID pinned to the
/// default width) pass whenever origins survive — which they do on
/// single-glyph runs, where replay never consults a width. So: the default
/// width and the /W CID set must match exactly (the deliberate change
/// preserves both on all six pairs), and every entry's value must stay within
/// WIDTH_EPS — the measured envelope of that one deliberate change class.
fn check_widths(name: &str, old: &CidFont, new: &CidFont) -> Result<(), Box<dyn Error>> {
    if Ratio::from_decimal(&old.default_width) != Ratio::from_decimal(&new.default_width) {
        return Err(format!("{name}: default width changed").into());
    }
    if old.widths.len() != new.widths.len()
        || old.widths.keys().any(|cid| !new.widths.contains_key(cid))
    {
        return Err(format!("{name}: /W CID set changed").into());
    }
    let num = |d: &Decimal| {
        let r = Ratio::from_decimal(d);
        r.num as f64 / r.den as f64
    };
    for (cid, w_old) in &old.widths {
        let w_new = &new.widths[cid];
        let delta = (num(w_old) - num(w_new)).abs();
        if delta > WIDTH_EPS {
            return Err(format!("{name}: CID {cid} width changed by {delta} (>{WIDTH_EPS})").into());
        }
    }
    Ok(())
}

fn escaped_pair() -> (&'static [u8], &'static [u8]) {
    (
        include_bytes!("fixtures/original-reference/escaped-searchable.pdf"),
        include_bytes!("fixtures/pdf-subset-20e5277/escaped.pdf"),
    )
}

/// Shift the first glyph's replayed origin by an exact rational dx. The first
/// glyph in the content has no width advance preceding it, so its replayed
/// origin IS its Tm translation verbatim: this models a Tm perturbation.
fn shifted_first_origin(snapshot: &Snapshot, dx: Ratio) -> Snapshot {
    let mut out = Snapshot {
        frames: snapshot.frames.clone(),
        fonts: snapshot.fonts.clone(),
    };
    let first = out
        .frames
        .first_mut()
        .expect("fixture has positioned glyphs");
    first.x = Ratio::new(first.x.num, first.x.den) + dx;
    out
}

/// Corrupt a snapshot the way a broken /W writer would: every entry pinned to
/// the font's default width, keys and defaults untouched.
fn pinned_widths_to_default(snapshot: &Snapshot) -> Snapshot {
    let mut out = Snapshot {
        frames: snapshot.frames.clone(),
        fonts: snapshot.fonts.clone(),
    };
    for font in out.fonts.values_mut() {
        let def = font.default_width.clone();
        for w in font.widths.values_mut() {
            *w = def.clone();
        }
    }
    out
}

/// Shift one /W entry by `delta` em-units, modeling a corruption that used to
/// sit inside the old WIDTH_EPS = 100 blanket gate.
fn shifted_first_width(snapshot: &Snapshot, delta: i128) -> Snapshot {
    let mut out = Snapshot {
        frames: snapshot.frames.clone(),
        fonts: snapshot.fonts.clone(),
    };
    let font = out.fonts.values_mut().next().expect("fixture has fonts");
    let (cid, w) = font
        .widths
        .iter()
        .next()
        .map(|(cid, w)| (*cid, w.clone()))
        .expect("fixture has widths");
    let r = Ratio::from_decimal(&w);
    font.widths.insert(
        cid,
        Decimal::from_ratio(r.num + delta * r.den, r.den as u128, 32).unwrap(),
    );
    out
}

/// Respell every default width without changing its value (`1000` -> `1000.0`),
/// modeling a producer that writes a different-but-equal token.
fn respelled_default_widths(snapshot: &Snapshot) -> Snapshot {
    let mut out = Snapshot {
        frames: snapshot.frames.clone(),
        fonts: snapshot.fonts.clone(),
    };
    for font in out.fonts.values_mut() {
        let s = font.default_width.as_str().to_string();
        let respelled = if s.contains('.') { format!("{s}0") } else { format!("{s}.0") };
        font.default_width = Decimal::new(&respelled).unwrap();
    }
    out
}

/// Minimal one-page PDF whose content stream is `content` verbatim. The reader
/// scans object headers (no xref needed) and falls back to the /Type /Catalog
/// object, so direct lengths suffice.
fn minimal_one_page_pdf(content: &str) -> Vec<u8> {
    let mut pdf = b"%PDF-1.7\n".to_vec();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>\nendobj\n",
    );
    let tail = format!("4 0 obj\n<< /Length {} >>\nstream\n{content}endstream\nendobj\n", content.len());
    pdf.extend_from_slice(tail.as_bytes());
    pdf
}

#[test]
fn exact_subset_geometry_text_and_programs_match_all_existing_candidates() {
    let escaped = escaped_pair();
    let cases: &[(&[u8], &[u8])] = &[
        (
            include_bytes!("fixtures/original-reference/65dbe7d-clean-searchable.pdf"),
            include_bytes!("fixtures/pdf-subset-20e5277/plain.pdf"),
        ),
        (
            include_bytes!("fixtures/math-reference/original.pdf"),
            include_bytes!("fixtures/pdf-subset-20e5277/inline-math.pdf"),
        ),
        (
            include_bytes!("fixtures/display-math-reference/original.pdf"),
            include_bytes!("fixtures/pdf-subset-20e5277/display-math.pdf"),
        ),
        (
            include_bytes!("fixtures/wrapping-reference/original.pdf"),
            include_bytes!("fixtures/pdf-subset-20e5277/wrapping.pdf"),
        ),
        (
            include_bytes!("fixtures/ligatures-reference/original.pdf"),
            include_bytes!("fixtures/pdf-subset-20e5277/ligatures.pdf"),
        ),
        (escaped.0, escaped.1),
    ];
    for (old, new) in cases {
        check(old, new).unwrap();
    }
    assert!(check(b"not a PDF", cases[0].1).is_err());
    assert!(check(cases[0].0, b"not a PDF").is_err());
}

#[test]
fn origin_shift_of_0_01_fails_but_print_drift_passes() {
    let (old, new) = escaped_pair();
    let b = load(new, &BTreeMap::new()).unwrap();
    let a = load(old, &b.fonts).unwrap();
    check_snapshots(&a, &b).unwrap();
    // Real 7-digit print drift is <= 5e-8; that must keep passing.
    check_snapshots(&a, &shifted_first_origin(&b, Ratio::new(5, 100_000_000))).unwrap();
    // A +0.01 Tm perturbation is four orders above the gate; it must fail.
    let err = check_snapshots(&a, &shifted_first_origin(&b, Ratio::new(1, 100))).unwrap_err();
    assert!(err.to_string().contains("moved"), "unexpected error: {err}");
}

#[test]
fn pinned_width_table_fails_the_width_check() {
    let (old, new) = escaped_pair();
    let b = load(new, &BTreeMap::new()).unwrap();
    let a = load(old, &b.fonts).unwrap();
    check_snapshots(&a, &b).unwrap();
    let bad = pinned_widths_to_default(&b);
    let err = check_snapshots(&a, &bad).unwrap_err();
    assert!(
        err.to_string().contains("width"),
        "unexpected error: {err}"
    );
}

#[test]
fn empty_or_mismatched_glyph_counts_fail() {
    let (old, new) = escaped_pair();
    let b = load(new, &BTreeMap::new()).unwrap();
    let a = load(old, &b.fonts).unwrap();
    let empty = Snapshot {
        frames: Vec::new(),
        fonts: BTreeMap::new(),
    };
    // Zero glyphs on both sides would otherwise pass every per-glyph check
    // vacuously; zero on one side must fail too, not just a bare count delta.
    assert!(check_snapshots(&empty, &empty).is_err());
    assert!(check_snapshots(&empty, &b).is_err());
    assert!(check_snapshots(&a, &empty).is_err());
    let mut fewer = Snapshot {
        frames: a.frames.clone(),
        fonts: BTreeMap::new(),
    };
    fewer.frames.pop();
    assert!(check_snapshots(&fewer, &b).is_err());
}

#[test]
fn default_width_equal_value_different_spelling_passes() {
    let (old, new) = escaped_pair();
    let b = load(new, &BTreeMap::new()).unwrap();
    let a = load(old, &b.fonts).unwrap();
    // A lexical default-width comparison would fail `1000` vs `1000.0`;
    // the numeric gate must keep passing.
    check_snapshots(&a, &respelled_default_widths(&b)).unwrap();
}

#[test]
fn width_shift_inside_old_tolerance_still_fails() {
    let (old, new) = escaped_pair();
    let b = load(new, &BTreeMap::new()).unwrap();
    let a = load(old, &b.fonts).unwrap();
    // +90 sat inside the old WIDTH_EPS = 100 blanket gate and passed
    // silently; past the tightened 85 gate it must fail.
    let bad = shifted_first_width(&b, 90);
    let err = check_snapshots(&a, &bad).unwrap_err();
    assert!(err.to_string().contains("width"), "unexpected error: {err}");
}

#[test]
fn oversize_operator_count_hits_the_operator_cap() {
    let content = "q\n".repeat(MAX_FIXTURE_OPERATORS + 1);
    let pdf = minimal_one_page_pdf(&content);
    let err = load(&pdf, &BTreeMap::new()).err().expect("operator cap did not fire");
    assert!(
        err.to_string().contains("fixture operator cap"),
        "unexpected error: {err}"
    );
}

#[test]
fn oversize_fixture_hits_the_byte_cap() {
    let (_, new) = escaped_pair();
    let big = vec![0u8; MAX_FIXTURE_BYTES + 1];
    let err = check(&big, new).unwrap_err();
    assert!(
        err.to_string().contains("fixture byte cap"),
        "unexpected error: {err}"
    );
}
