//! Latin Modern (GUST Font License) from the local BasicTeX installation:
//! OpenType CFF (`OTTO`) parsing, shaping, whole-program embedding data and
//! the `MATH` table. Tests skip with a message if the files are absent.
//!
//! CoreText reference numbers were produced on macOS 26.3.1 by
//! `swift examples/compare_coretext.swift --size 10 <lmroman10-regular.otf> "<text>"`
//! and are quoted here so the check runs without CoreText.

use std::path::Path;

use flashtex_font_engine::embed::{EmbedPlan, FontFile, parse_to_unicode};
use flashtex_font_engine::shape::{ShapeOptions, shape};
use flashtex_font_engine::subset::subset;
use flashtex_font_engine::{
    Error, Face, GlyphId, KerningSource, Outlines, TrueTypeFace, load_from_path,
};

const LM_DIR: &str = "/usr/local/texlive/2026basic/texmf-dist/fonts/opentype/public/lm";
const LM_ROMAN10: &str =
    "/usr/local/texlive/2026basic/texmf-dist/fonts/opentype/public/lm/lmroman10-regular.otf";
const LM_MATH: &str =
    "/usr/local/texlive/2026basic/texmf-dist/fonts/opentype/public/lm-math/latinmodern-math.otf";

fn load(path: &str) -> Option<TrueTypeFace> {
    if !Path::new(path).is_file() {
        eprintln!("SKIP: {path} not present on this machine");
        return None;
    }
    Some(load_from_path(Path::new(path)).unwrap_or_else(|e| panic!("{path}: {e}")))
}

#[test]
fn lmroman10_parses_as_cff_with_metrics_and_identity() {
    let Some(f) = load(LM_ROMAN10) else { return };
    assert_eq!(f.outlines(), Outlines::Cff);
    assert_eq!(f.units_per_em(), 1000);
    assert_eq!(f.postscript_name(), "LMRoman10-Regular");
    assert_eq!(f.id().family, "LM Roman 10"); // name id 1 as the font declares it
    assert_eq!(f.id().weight, 400);
    assert!(f.vertical_metrics().cap_height_declared);
    assert!(f.vertical_metrics().x_height_declared);
    assert!(!f.is_fixed_pitch());
    // Unsupported note is explicit: CFF outlines are exposed raw only.
    assert!(f.unsupported().iter().any(|u| u.table == "CFF "));
}

#[test]
fn lmroman10_hello_at_10pt_matches_coretext_within_0_01pt() {
    let Some(f) = load(LM_ROMAN10) else { return };
    let s = shape(&f, "Hello", &ShapeOptions::PLAIN).unwrap();
    assert!(s.missing.is_empty());
    // CoreText (CTFontGetAdvancesForGlyphs, same file): 22.5000 pt.
    let w = s.width_pt(10.0);
    assert!((w - 22.5000).abs() < 0.01, "Hello width {w}");
    // Per-glyph: H 750, e 444, l 278, l 278, o 500 (Computer Modern design).
    let adv: Vec<i32> = s.glyphs().map(|g| g.advance).collect();
    assert_eq!(adv, vec![750, 444, 278, 278, 500]);
    // Longer line, also CoreText-exact: 337.2200 pt.
    let line = "The quick brown fox jumps over the lazy dog. AVATAR office, fluffy waffle.";
    let w = shape(&f, line, &ShapeOptions::PLAIN)
        .unwrap()
        .width_pt(10.0);
    assert!((w - 337.2200).abs() < 0.01, "line width {w}");
}

#[test]
fn lmroman10_av_kerning_via_gpos() {
    let Some(f) = load(LM_ROMAN10) else { return };
    assert_eq!(f.kerning_source(), KerningSource::Gpos);
    let (adj, src) = f.kerning(f.glyph_id('A').unwrap(), f.glyph_id('V').unwrap());
    assert_eq!(src, KerningSource::Gpos);
    assert!(adj < 0, "AV {adj}");
    let kerned = shape(&f, "AV", &ShapeOptions::default()).unwrap();
    // CoreText CTLine width for "AV" at 10 pt: 13.8900.
    assert!((kerned.width_pt(10.0) - 13.89).abs() < 0.01);
}

#[test]
fn lmroman10_ligatures_via_gsub_including_two_step_ffi() {
    let Some(f) = load(LM_ROMAN10) else { return };
    let no_fallback = ShapeOptions {
        cmap_ligature_fallback: false,
        ..ShapeOptions::default()
    };
    let fi = shape(&f, "fi", &no_fallback).unwrap();
    assert_eq!(fi.clusters.len(), 1);
    assert_eq!(fi.clusters[0].text, "fi");
    assert_eq!(fi.clusters[0].source_range, 0..2);
    let f_gid = f.glyph_id('f').unwrap();
    let i_gid = f.glyph_id('i').unwrap();
    assert_eq!(
        f.ligature(&[f_gid, i_gid]),
        Some(fi.clusters[0].glyphs[0].gid)
    );
    // ffi is built as f+f -> ff (lookup 8) then ff+i -> ffi (lookup 9); the
    // result must be the single 833-unit ffi glyph, as CoreText produces
    // (8.3300 pt at 10 pt), not ff + i (861 units).
    let ffi = shape(&f, "ffi", &no_fallback).unwrap();
    assert_eq!(ffi.clusters.len(), 1, "{:?}", ffi.clusters);
    assert_eq!(ffi.clusters[0].glyphs.len(), 1);
    assert_eq!(ffi.advance_units(), 833);
    assert_eq!(ffi.clusters[0].source_range, 0..3);
    assert_eq!(
        f.ligature(&[f_gid, f_gid, i_gid]),
        Some(ffi.clusters[0].glyphs[0].gid)
    );
    // Whole line with CoreText's default shaping: 54.9900 pt.
    let line = shape(&f, "AV fi fl ffi ffl", &ShapeOptions::default()).unwrap();
    assert!(
        (line.width_pt(10.0) - 54.99).abs() < 0.01,
        "{}",
        line.width_pt(10.0)
    );
    let off = shape(
        &f,
        "ffi",
        &ShapeOptions {
            ligatures: false,
            ..ShapeOptions::default()
        },
    )
    .unwrap();
    assert_eq!(off.clusters.len(), 3);
}

#[test]
fn lmroman10_cmap_covers_e_acute_and_em_dash() {
    let Some(f) = load(LM_ROMAN10) else { return };
    let s = shape(&f, "\u{00E9}\u{2014}", &ShapeOptions::default()).unwrap();
    assert!(s.missing.is_empty());
    assert_eq!(s.clusters.len(), 2);
    assert_eq!(s.clusters[0].source_range, 0..2);
    assert_eq!(s.clusters[1].source_range, 2..5);
    assert_eq!(s.clusters[1].glyphs[0].advance, 1000, "em dash is 1 em");
    // e + U+0301 composes to the same glyph as é.
    let composed = shape(&f, "e\u{0301}", &ShapeOptions::default()).unwrap();
    assert_eq!(
        composed.clusters[0].glyphs[0].gid,
        s.clusters[0].glyphs[0].gid
    );
}

#[test]
fn lmroman10_raw_cff_is_exposed_with_the_directory_length() {
    let Some(f) = load(LM_ROMAN10) else { return };
    let cff = f.cff_table().expect("CFF table");
    // Independent check of the length from the sfnt directory.
    let data = std::fs::read(LM_ROMAN10).unwrap();
    let n = u16::from_be_bytes([data[4], data[5]]) as usize;
    let mut expected = None;
    for i in 0..n {
        let rec = 12 + 16 * i;
        if &data[rec..rec + 4] == b"CFF " {
            let off = u32::from_be_bytes(data[rec + 8..rec + 12].try_into().unwrap()) as usize;
            let len = u32::from_be_bytes(data[rec + 12..rec + 16].try_into().unwrap()) as usize;
            expected = Some(&data[off..off + len]);
        }
    }
    assert_eq!(cff, expected.expect("CFF in directory"));
    assert_eq!(cff[0], 1, "CFF header major version 1");
    // glyf-only operations are refused, not faked.
    assert!(matches!(
        f.glyph_data(GlyphId(1)),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        subset(&f, &[GlyphId(1)]),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn lmroman10_embeds_as_bare_cff_with_identity_cids() {
    let Some(f) = load(LM_ROMAN10) else { return };
    let shaped = shape(&f, "ffi AV \u{00E9}", &ShapeOptions::default()).unwrap();
    let mut plan = EmbedPlan::new();
    plan.add_shaped(&shaped);
    let pdf = plan.finish(&f).unwrap();
    assert_eq!(pdf.cid_font_subtype, "CIDFontType0");
    assert_eq!(pdf.base_font, "LMRoman10-Regular");
    assert!(pdf.subset.is_none());
    match &pdf.font_file {
        FontFile::Cff(bytes) => {
            assert_eq!(&bytes[..], f.cff_table().unwrap());
            assert_eq!(
                pdf.font_file.stream_key(),
                ("FontFile3", Some("CIDFontType0C"))
            );
        }
        other => panic!("expected bare CFF, got {other:?}"),
    }
    for g in shaped.glyphs() {
        assert_eq!(pdf.cid(g.gid), Some(g.gid.0), "identity CIDs for CFF");
        // /W widths are unkerned glyph advances.
        let unkerned = i32::from(f.advance(g.gid).unwrap());
        assert_eq!(
            pdf.width(g.gid.0),
            Some(unkerned),
            "width of gid {}",
            g.gid.0
        );
    }
    let parsed = parse_to_unicode(&pdf.to_unicode_cmap).unwrap();
    let ffi_cid = pdf.cid(shaped.clusters[0].glyphs[0].gid).unwrap();
    assert_eq!(parsed[&ffi_cid], "ffi");
    assert!(pdf.w_array().starts_with("[ 0 [ "));
    assert!(!pdf.embedding_restricted());
}

#[test]
fn latin_modern_math_constants_match_the_file() {
    let Some(f) = load(LM_MATH) else { return };
    assert_eq!(f.outlines(), Outlines::Cff);
    let m = f.math().expect("MATH table");
    let c = &m.constants;
    // Values read independently from the file with a Python parser
    // (tools are not committed; the numbers are what the table stores).
    assert_eq!(c.script_percent_scale_down, 70);
    assert_eq!(c.script_script_percent_scale_down, 50);
    assert_eq!(c.delimited_sub_formula_min_height, 1300);
    assert_eq!(c.display_operator_min_height, 1300);
    assert_eq!(c.math_leading, 154);
    assert_eq!(c.axis_height, 250);
    assert_eq!(c.accent_base_height, 450);
    assert_eq!(c.subscript_shift_down, 247);
    assert_eq!(c.superscript_shift_up, 363);
    assert_eq!(c.superscript_shift_up_cramped, 289);
    assert_eq!(c.sub_superscript_gap_min, 160);
    assert_eq!(c.fraction_numerator_shift_up, 394);
    assert_eq!(c.fraction_numerator_display_style_shift_up, 677);
    assert_eq!(c.fraction_denominator_shift_down, 345);
    assert_eq!(c.fraction_denominator_display_style_shift_down, 686);
    assert_eq!(c.fraction_rule_thickness, 40);
    assert_eq!(c.fraction_numerator_gap_min, 40);
    assert_eq!(c.fraction_num_display_style_gap_min, 120);
    assert_eq!(c.radical_vertical_gap, 50);
    assert_eq!(c.radical_display_style_vertical_gap, 148);
    assert_eq!(c.radical_rule_thickness, 40);
    assert_eq!(c.radical_extra_ascender, 40);
    assert_eq!(c.radical_kern_before_degree, 278);
    assert_eq!(c.radical_kern_after_degree, -556);
    assert_eq!(c.radical_degree_bottom_raise_percent, 60);
    // Italics correction: text f = 79, math italic f (U+1D453) = 90,
    // integral (U+222B) = 332; summation lists none.
    assert_eq!(m.italics_correction(f.glyph_id('f').unwrap()), 79);
    assert_eq!(m.italics_correction(f.glyph_id('\u{1D453}').unwrap()), 90);
    assert_eq!(m.italics_correction(f.glyph_id('\u{222B}').unwrap()), 332);
    assert_eq!(m.italics_correction(f.glyph_id('\u{2211}').unwrap()), 0);
    assert!(
        m.top_accent_attachment(f.glyph_id('\u{1D453}').unwrap())
            .is_some()
    );
}

#[test]
fn every_latin_modern_text_face_parses() {
    let dir = Path::new(LM_DIR);
    if !dir.is_dir() {
        eprintln!("SKIP: {LM_DIR} not present");
        return;
    }
    // Bounded: the explicit file list, not a directory scan in library code.
    let names = [
        "lmroman10-regular.otf",
        "lmroman10-bold.otf",
        "lmroman10-italic.otf",
        "lmroman10-bolditalic.otf",
        "lmroman12-regular.otf",
        "lmsans10-regular.otf",
        "lmmono10-regular.otf",
    ];
    for name in names {
        let path = dir.join(name);
        if !path.is_file() {
            eprintln!("SKIP: {} not present", path.display());
            continue;
        }
        let f = load_from_path(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(f.outlines(), Outlines::Cff, "{name}");
        let s = shape(&f, "Hello, world! fi AV", &ShapeOptions::default()).unwrap();
        assert!(s.missing.is_empty(), "{name}: {:?}", s.missing);
        if name.starts_with("lmmono") {
            assert!(f.is_fixed_pitch(), "{name}");
            assert_eq!(s.ligatures_applied, 0, "{name} must not ligate");
        }
    }
}
