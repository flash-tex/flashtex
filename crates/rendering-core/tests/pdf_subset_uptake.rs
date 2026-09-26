//! Old artifacts are retained; compare semantic identities before new snapshots.
//!
//! The retained OLD one-page PDFs predate two deliberate, reviewed pdf-crate
//! changes, both gated by that crate's own tests: /W widths written from the
//! display list's advances where hmtx differs (4893f3e7f), and coordinates
//! capped at 7 fractional digits (2a9936f3b #729). Exact byte and
//! exact-rational replay equality with the old artifacts can therefore no
//! longer hold by design. What the subsetter itself must preserve — glyph
//! identities, absolute origins within print precision, Unicode maps, glyph
//! programs, and smaller size — is asserted here with tolerance. Exact bytes
//! of the new snapshots are pinned separately by the byte-exact goldens in
//! math_reference and original_reference.
use flashtex_pdf::{cff::CffFont, compare::font_from_dict, exact::*, reader::PdfFile};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
};

/// Origins replayed from 7-fractional-digit tokens differ from the retained
/// 20-digit readings by at most 5e-8; gate two orders of magnitude above that
/// and still six orders below a pixel.
const ORIGIN_EPS: f64 = 1e-6;

struct Snapshot {
    frames: Vec<GlyphPosition>,
    fonts: BTreeMap<String, CidFont>,
}

fn load(pdf: &[u8]) -> Result<Snapshot, Box<dyn Error>> {
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
    let frames = glyph_positions(&ops, &|_| true, &|name, gid| {
        fonts
            .get(name)
            .and_then(|f| f.widths.get(&gid).map(Ratio::from_decimal))
    })?;
    Ok(Snapshot { frames, fonts })
}

fn check(old: &[u8], new: &[u8]) -> Result<(), Box<dyn Error>> {
    let a = load(old)?;
    let b = load(new)?;
    check_snapshots(&a, &b)?;
    if new.len() >= old.len() {
        return Err("subset must stay smaller than the retained artifact".into());
    }
    Ok(())
}

fn check_snapshots(a: &Snapshot, b: &Snapshot) -> Result<(), Box<dyn Error>> {
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
    for g in &a.frames {
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
    let a = load(old).unwrap();
    let b = load(new).unwrap();
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
    let a = load(old).unwrap();
    let b = load(new).unwrap();
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
    let a = load(old).unwrap();
    let b = load(new).unwrap();
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
fn oversize_fixture_hits_the_byte_cap() {
    let (_, new) = escaped_pair();
    // 4 MiB cap reinstated from the old compare(); must match MAX_FIXTURE_BYTES.
    let big = vec![0u8; 4 * 1024 * 1024 + 1];
    let err = check(&big, new).unwrap_err();
    assert!(
        err.to_string().contains("fixture byte cap"),
        "unexpected error: {err}"
    );
}
