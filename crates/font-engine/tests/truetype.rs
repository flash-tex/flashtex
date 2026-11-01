//! TrueType parsing, shaping, subsetting and embedding against Apple-supplied
//! system fonts. Each test skips with a message when its font is absent, so
//! the suite is honest on machines without them. Nothing is committed.

use std::path::Path;

use flashtex_font_engine::embed::{EmbedPlan, parse_to_unicode};
use flashtex_font_engine::resolve::FontSearch;
use flashtex_font_engine::shape::{ShapeOptions, shape};
use flashtex_font_engine::subset::{subset, verify_checksums};
use flashtex_font_engine::{
    Face, FontSource, GlyphId, KerningSource, TrueTypeFace, load_from_path,
};

const TIMES_NEW_ROMAN: &str = "/System/Library/Fonts/Supplemental/Times New Roman.ttf";
const ARIAL_UNICODE: &str = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";
const TIMES_TTC: &str = "/System/Library/Fonts/Times.ttc";

fn load(path: &str) -> Option<TrueTypeFace> {
    if !Path::new(path).is_file() {
        eprintln!("SKIP: {path} not present on this machine");
        return None;
    }
    Some(load_from_path(Path::new(path)).unwrap_or_else(|e| panic!("{path}: {e}")))
}

#[test]
fn times_new_roman_metrics_and_identity() {
    let Some(f) = load(TIMES_NEW_ROMAN) else {
        return;
    };
    assert_eq!(f.units_per_em(), 2048);
    assert_eq!(f.id().family, "Times New Roman");
    assert_eq!(f.postscript_name(), "TimesNewRomanPSMT");
    assert!(matches!(
        &f.id().source,
        FontSource::File { face_index: 0, .. }
    ));
    let again = load(TIMES_NEW_ROMAN).unwrap();
    assert_eq!(f.id().content_sha256, again.id().content_sha256);
    assert!(f.glyph_id('H').is_some());
    let vm = f.vertical_metrics();
    assert!(vm.ascender > 0 && vm.descender < 0);
    assert!(vm.cap_height > 0 && vm.x_height > 0);
    // Times New Roman's H is 1341 units tall at 2048/em (0.655 em) — the
    // OS/2 sCapHeight value in Apple's copy.
    assert!(
        (1300..=1400).contains(&vm.cap_height),
        "cap_height {}",
        vm.cap_height
    );
}

#[test]
fn times_new_roman_hello_advances_are_near_core14() {
    let Some(f) = load(TIMES_NEW_ROMAN) else {
        return;
    };
    let s = shape(&f, "Hello", &ShapeOptions::PLAIN).unwrap();
    assert!(s.missing.is_empty());
    // Times New Roman's metrics track Times-Roman closely: 26.664 pt in the
    // AFM; the TrueType face is within 0.05 pt at 12 pt.
    let w = s.width_pt(12.0);
    assert!((w - 26.664).abs() < 0.05, "width {w}");
}

#[test]
fn av_kerning_via_gpos_or_kern_is_negative() {
    let Some(f) = load(TIMES_NEW_ROMAN) else {
        return;
    };
    let a = f.glyph_id('A').unwrap();
    let v = f.glyph_id('V').unwrap();
    let (adj, src) = f.kerning(a, v);
    assert!(adj < 0, "AV kerning {adj} from {src:?}");
    assert!(matches!(
        src,
        KerningSource::Gpos | KerningSource::KernTable
    ));
    let kerned = shape(&f, "AV", &ShapeOptions::default()).unwrap();
    let plain = shape(&f, "AV", &ShapeOptions::PLAIN).unwrap();
    assert_eq!(
        kerned.advance_units(),
        plain.advance_units() + i64::from(adj)
    );
    assert_eq!(kerned.kerning_source, src);
}

#[test]
fn fi_ligature_from_gsub_or_cmap() {
    let Some(f) = load(TIMES_NEW_ROMAN) else {
        return;
    };
    let on = shape(&f, "fi", &ShapeOptions::default()).unwrap();
    let off = shape(
        &f,
        "fi",
        &ShapeOptions {
            ligatures: false,
            ..ShapeOptions::default()
        },
    )
    .unwrap();
    assert_eq!(off.clusters.len(), 2);
    assert_eq!(
        on.clusters.len(),
        1,
        "expected a ligature (GSUB liga or cmap U+FB01)"
    );
    assert_eq!(on.clusters[0].glyphs.len(), 1);
    assert_eq!(on.clusters[0].source_range, 0..2);
    assert_eq!(on.clusters[0].text, "fi");
    assert_ne!(on.clusters[0].glyphs[0].gid, off.clusters[0].glyphs[0].gid);
    assert_ne!(on.clusters[0].glyphs[0].gid, off.clusters[1].glyphs[0].gid);
}

#[test]
fn combining_mark_composes_via_cmap() {
    let Some(f) = load(TIMES_NEW_ROMAN) else {
        return;
    };
    let composed = shape(&f, "e\u{0301}", &ShapeOptions::default()).unwrap();
    let pre = shape(&f, "\u{00E9}", &ShapeOptions::default()).unwrap();
    assert_eq!(composed.clusters.len(), 1);
    assert_eq!(composed.clusters[0].glyphs.len(), 1);
    assert_eq!(
        composed.clusters[0].glyphs[0].gid,
        pre.clusters[0].glyphs[0].gid
    );
    assert_eq!(composed.clusters[0].source_range, 0..3);
    assert!(composed.missing.is_empty());
}

#[test]
fn unicode_repertoire_in_arial_unicode() {
    let Some(f) = load(ARIAL_UNICODE) else { return };
    let s = shape(&f, "\u{00E9}\u{2014}\u{4E2D}", &ShapeOptions::default()).unwrap();
    assert!(s.missing.is_empty(), "missing {:?}", s.missing);
    assert_eq!(s.clusters.len(), 3);
    assert_eq!(s.clusters[2].source_range, 5..8); // é is 2 bytes, — is 3
    let zhong = s.clusters[2].glyphs[0];
    assert_eq!(
        i64::from(zhong.advance),
        i64::from(f.units_per_em()),
        "ideograph is 1 em"
    );
}

#[test]
fn missing_glyph_in_truetype_is_notdef_and_listed() {
    let Some(f) = load(TIMES_NEW_ROMAN) else {
        return;
    };
    let s = shape(&f, "a\u{4E2D}", &ShapeOptions::default()).unwrap();
    assert_eq!(s.missing.len(), 1);
    assert_eq!(s.clusters[1].glyphs[0].gid, GlyphId::NOTDEF);
    assert_eq!(
        s.clusters[1].glyphs[0].advance,
        i32::from(f.advance(GlyphId::NOTDEF).unwrap())
    );
}

#[test]
fn subsetting_is_deterministic_and_checksummed() {
    let Some(f) = load(TIMES_NEW_ROMAN) else {
        return;
    };
    let text = "Hello, \u{00E9} fi AV";
    let shaped = shape(&f, text, &ShapeOptions::default()).unwrap();
    let gids: Vec<GlyphId> = shaped.glyphs().map(|g| g.gid).collect();
    let a = subset(&f, &gids).unwrap();
    let mut reversed = gids.clone();
    reversed.reverse();
    reversed.extend_from_slice(&gids); // duplicates and order must not matter
    let b = subset(&f, &reversed).unwrap();
    assert_eq!(a.program, b.program);
    assert_eq!(a.tag, b.tag);
    assert_eq!(a.old_to_new, b.old_to_new);
    verify_checksums(&a.program).unwrap();
    assert!(a.program.len() < f.program().len() / 10);
    // The subset parses again with our own parser and keeps advances.
    let sub = TrueTypeFace::parse(a.program.clone()).unwrap();
    for (&old, &new) in &a.old_to_new {
        assert_eq!(
            sub.advance(GlyphId(new)).unwrap(),
            f.advance(GlyphId(old)).unwrap()
        );
    }
    assert_eq!(sub.num_glyphs(), a.num_glyphs());
    assert_eq!(a.new_to_old[0], 0);
    // Different glyph sets give different tags.
    let c = subset(&f, &[f.glyph_id('Z').unwrap()]).unwrap();
    assert_ne!(a.tag, c.tag);
}

#[test]
fn to_unicode_round_trip_including_ligature_text() {
    let Some(f) = load(TIMES_NEW_ROMAN) else {
        return;
    };
    let text = "fi e\u{0301} AV";
    let shaped = shape(&f, text, &ShapeOptions::default()).unwrap();
    let mut plan = EmbedPlan::new();
    plan.add_shaped(&shaped);
    let pdf = plan.finish(&f).unwrap();
    assert!(pdf.base_font.ends_with("+TimesNewRomanPSMT"));
    assert_eq!(pdf.base_font.len(), 7 + "TimesNewRomanPSMT".len());
    let parsed = parse_to_unicode(&pdf.to_unicode_cmap).unwrap();
    assert_eq!(parsed, pdf.to_unicode);
    // Every shaped glyph has a CID, and the ligature maps back to "fi".
    for cluster in &shaped.clusters {
        for g in &cluster.glyphs {
            let cid = pdf.cid(g.gid).expect("every shaped glyph is in the subset");
            assert!(pdf.width(cid).is_some());
        }
    }
    let lig_cid = pdf.cid(shaped.clusters[0].glyphs[0].gid).unwrap();
    assert_eq!(parsed[&lig_cid], "fi");
    let acute_cid = pdf.cid(shaped.clusters[2].glyphs[0].gid).unwrap();
    assert_eq!(parsed[&acute_cid], "e\u{0301}");
    assert!(pdf.w_array().starts_with("[ 0 [ "), "{}", pdf.w_array());
    assert_eq!(
        pdf.width(0),
        Some(f.to_pdf_units(i64::from(f.advance(GlyphId::NOTDEF).unwrap())))
    );
    assert_eq!(pdf.cid_font_subtype, "CIDFontType2");
    assert!(matches!(
        pdf.font_file,
        flashtex_font_engine::embed::FontFile::TrueTypeSubset(_)
    ));
    assert_eq!(
        pdf.font_file.bytes(),
        &pdf.subset.as_ref().unwrap().program[..]
    );
    assert!(pdf.descriptor.ascent > 0 && pdf.descriptor.descent < 0);
}

#[test]
fn ttc_faces_are_addressable_by_index() {
    if !Path::new(TIMES_TTC).is_file() {
        eprintln!("SKIP: {TIMES_TTC} not present");
        return;
    }
    let f0 = flashtex_font_engine::load_from_path_index(Path::new(TIMES_TTC), 0).unwrap();
    let f1 = flashtex_font_engine::load_from_path_index(Path::new(TIMES_TTC), 1).unwrap();
    assert_ne!(f0.id().content_sha256, f1.id().content_sha256);
    assert_ne!(f0.postscript_name(), f1.postscript_name());
    assert!(flashtex_font_engine::load_from_path_index(Path::new(TIMES_TTC), 99).is_err());
}

#[test]
fn bounded_resolution_never_scans() {
    let search = FontSearch::new()
        .with_dir("/nonexistent/dir")
        .with_macos_system_dirs();
    assert!(search.find("../etc/passwd").is_none());
    assert!(search.find("definitely-not-a-font.ttf").is_none());
    if Path::new(TIMES_NEW_ROMAN).is_file() {
        assert_eq!(
            search.find("Times New Roman.ttf").unwrap(),
            Path::new(TIMES_NEW_ROMAN)
        );
        let f = search.load("Times New Roman.ttf", 0).unwrap();
        assert_eq!(f.postscript_name(), "TimesNewRomanPSMT");
    }
}

#[test]
fn malformed_and_unsupported_inputs_are_errors_not_panics() {
    assert!(TrueTypeFace::parse(vec![]).is_err());
    assert!(TrueTypeFace::parse(vec![0; 64]).is_err());
    let mut otto = vec![0u8; 64];
    otto[..4].copy_from_slice(b"OTTO");
    // An OTTO header with no tables is malformed/missing, never a panic.
    assert!(TrueTypeFace::parse(otto).is_err());
}

const ARIAL: &str = "/System/Library/Fonts/Supplemental/Arial.ttf";
const IOWAN: &str = "/System/Library/Fonts/Supplemental/Iowan Old Style.ttc";
const TNR_ITALIC: &str = "/System/Library/Fonts/Supplemental/Times New Roman Italic.ttf";

#[test]
fn gpos_pairpos_kerning_in_arial_agrees_with_legacy_kern_table() {
    let Some(f) = load(ARIAL) else { return };
    assert_eq!(f.kerning_source(), KerningSource::Gpos);
    let a = f.glyph_id('A').unwrap();
    let v = f.glyph_id('V').unwrap();
    let (adj, src) = f.kerning(a, v);
    assert_eq!(src, KerningSource::Gpos);
    assert!(adj < 0, "AV via GPOS {adj}");
    // Apple's Arial carries the same pairs in the legacy table.
    assert_eq!(f.legacy_kern_table_kerning(a, v), Some(adj));
    let (none, _) = f.kerning(f.glyph_id('x').unwrap(), f.glyph_id('x').unwrap());
    assert_eq!(none, 0);
}

#[test]
fn gpos_kerning_in_times_new_roman_italic() {
    let Some(f) = load(TNR_ITALIC) else { return };
    assert_eq!(f.kerning_source(), KerningSource::Gpos);
    let (adj, _) = f.kerning(f.glyph_id('T').unwrap(), f.glyph_id('o').unwrap());
    assert!(adj < 0, "To via GPOS {adj}");
}

#[test]
fn gpos_extension_lookups_are_unwrapped() {
    if !Path::new(IOWAN).is_file() {
        eprintln!("SKIP: {IOWAN} not present");
        return;
    }
    let f = flashtex_font_engine::load_from_path_index(Path::new(IOWAN), 0).unwrap();
    assert_eq!(
        f.kerning_source(),
        KerningSource::Gpos,
        "Iowan uses Extension (type 9 -> 2)"
    );
    let (adj, src) = f.kerning(f.glyph_id('A').unwrap(), f.glyph_id('V').unwrap());
    assert_eq!(src, KerningSource::Gpos);
    assert!(adj < 0, "AV via Extension PairPos {adj}");
}

#[test]
fn times_new_roman_latin_ligatures_come_from_cmap_not_its_arabic_only_liga() {
    let Some(f) = load(TIMES_NEW_ROMAN) else {
        return;
    };
    // TNR's only `liga` records live under the `arab` script; the default
    // (DFLT/latn) language system selects none of them.
    assert!(!f.has_gsub_ligatures(), "no Latin liga lookups selected");
    let fi = [f.glyph_id('f').unwrap(), f.glyph_id('i').unwrap()];
    assert_eq!(f.ligature(&fi), None, "no Latin fi in GSUB liga");
    let with = shape(&f, "fi", &ShapeOptions::default()).unwrap();
    assert_eq!(with.ligatures_applied, 1);
    assert_eq!(
        with.clusters[0].glyphs[0].gid,
        f.glyph_id('\u{FB01}').unwrap()
    );
    let without = shape(
        &f,
        "fi",
        &ShapeOptions {
            cmap_ligature_fallback: false,
            ..ShapeOptions::default()
        },
    )
    .unwrap();
    assert_eq!(without.ligatures_applied, 0);
    assert_eq!(without.clusters.len(), 2);
}

#[test]
fn gsub_latin_liga_from_a_font_that_declares_it() {
    let candidates = [
        "/System/Library/Fonts/Supplemental/Iowan Old Style.ttc",
        "/System/Library/Fonts/Supplemental/Charter.ttc",
        "/System/Library/Fonts/Supplemental/PTSerif.ttc",
        "/System/Library/Fonts/Supplemental/Cochin.ttc",
        "/System/Library/Fonts/Supplemental/Futura.ttc",
        "/System/Library/Fonts/Supplemental/Hoefler Text.ttc",
        "/System/Library/Fonts/Supplemental/Baskerville.ttc",
        "/System/Library/Fonts/Supplemental/Georgia.ttf",
        "/System/Library/Fonts/Palatino.ttc",
        "/System/Library/Fonts/Avenir.ttc",
    ];
    for path in candidates {
        if !Path::new(path).is_file() {
            continue;
        }
        let f = flashtex_font_engine::load_from_path_index(Path::new(path), 0).unwrap();
        let fi = [f.glyph_id('f').unwrap(), f.glyph_id('i').unwrap()];
        let Some(lig) = f.ligature(&fi) else { continue };
        let s = shape(
            &f,
            "fix",
            &ShapeOptions {
                cmap_ligature_fallback: false,
                ..ShapeOptions::default()
            },
        )
        .unwrap();
        assert_eq!(s.ligatures_applied, 1, "{path}");
        assert_eq!(s.clusters[0].glyphs[0].gid, lig);
        assert_eq!(s.clusters[0].source_range, 0..2);
        assert_eq!(s.clusters[0].text, "fi");
        assert_eq!(s.clusters[1].source_range, 2..3);
        eprintln!("GSUB liga verified with {path} (fi -> gid {})", lig.0);
        return;
    }
    eprintln!("SKIP: no candidate font with a Latin GSUB liga present");
}

#[test]
fn gpos_mark_to_base_positions_an_unattached_combining_mark() {
    let Some(f) = load(TIMES_NEW_ROMAN) else {
        return;
    };
    assert!(
        f.has_mark_attachment(),
        "Times New Roman carries GPOS mark/mkmk"
    );
    // x + U+0301 has no precomposed form, so the mark stays a separate glyph
    // and MarkToBase places it above the x.
    let s = shape(&f, "x\u{0301}", &ShapeOptions::default()).unwrap();
    assert_eq!(s.clusters.len(), 1);
    assert_eq!(s.clusters[0].glyphs.len(), 2);
    assert!(s.missing.is_empty());
    let base = s.clusters[0].glyphs[0];
    let mark = s.clusters[0].glyphs[1];
    let x = f.glyph_id('x').unwrap();
    let acute = f.glyph_id('\u{0301}').unwrap();
    let (dx, dy) = f.mark_attachment(x, acute).expect("anchor pair");
    assert_eq!(mark.x_offset, i32::from(dx) - base.advance);
    assert_eq!(mark.y_offset, i32::from(dy));
    assert_eq!(mark.advance, 0);
    // The mark lands over the base, not after it: its origin is inside the
    // base's advance box horizontally, and at or above the baseline.
    let mark_x = base.advance + mark.x_offset;
    assert!(
        mark_x > -base.advance && mark_x < base.advance,
        "mark x {mark_x} vs base advance {}",
        base.advance
    );
    // TNR draws the combining acute at cap height and anchors it down to
    // the x-height of 'x' (dy = -310 at 2048/em); the exact value is the
    // font's, the test only requires a real, bounded attachment.
    assert!(
        dy != 0 && i32::from(dy).abs() < i32::from(f.units_per_em()),
        "dy {dy}"
    );
}
