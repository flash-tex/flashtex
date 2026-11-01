//! FT-018 rev 3: one metric source behind paragraph layout, PDF and math.

use std::path::Path;

use flashtex_font_engine::adapters::math::OpenTypeMathFace;
use flashtex_font_engine::adapters::paragraph::{FaceMetrics, glyph_run};
use flashtex_font_engine::adapters::pdf::to_pdf_embedded_subset;
use flashtex_font_engine::adapters::preview::{ExportRun, face_metrics_json};
use flashtex_font_engine::core14::{Core14, Core14Face};
use flashtex_font_engine::embed::EmbedPlan;
use flashtex_font_engine::manifest::{BASICTEX_OPENTYPE_ROOT, PinnedFontSet, pinned_latin_modern};
use flashtex_font_engine::shape::{ShapeOptions, shape};
use flashtex_font_engine::{Error, Face, TrueTypeFace, load_from_path};
use flashtex_math_layout::metrics::{FontId as MathFontId, MathFontMetrics, SizeClass};
use flashtex_paragraph_layout::items::shape_run;
use flashtex_paragraph_layout::metrics::FontMetricsSource;
use flashtex_pdf::embed::Program;

const CRATE: &str = env!("CARGO_MANIFEST_DIR");

fn pinned() -> Option<PinnedFontSet> {
    let root = Path::new(BASICTEX_OPENTYPE_ROOT);
    if !root.is_dir() {
        eprintln!("SKIP: {BASICTEX_OPENTYPE_ROOT} not present");
        return None;
    }
    Some(
        PinnedFontSet::load(
            &pinned_latin_modern(),
            root,
            &Path::new(CRATE).join("fonts"),
        )
        .unwrap(),
    )
}

#[test]
fn paragraph_char_route_and_shaped_route_agree_on_core14() {
    let times = Core14Face::new(Core14::TimesRoman);
    let m = FaceMetrics::new(&times);
    assert_eq!(m.font_id().0, times.id().content_sha256);
    assert_eq!(m.units_per_em(), 1000.0);
    assert_eq!(m.advance('H'), 722.0);
    assert_eq!(m.kern('A', 'V'), -135.0);
    assert_eq!(m.glyph_id('H'), u32::from(times.glyph_id('H').unwrap().0));
    assert_eq!(m.space(), 250.0);
    assert_eq!(m.ligature('f', 'i').map(|l| l.result), Some('\u{FB01}'));
    // paragraph-layout's own shaper with this source vs the engine's run.
    let run_a = shape_run(&m, 12.0, "office", 0);
    let (run_b, shaped) = glyph_run(&times, 12.0, "office", 0, &ShapeOptions::default()).unwrap();
    assert!(
        (run_a.width - run_b.width).abs() < 1e-9,
        "{} vs {}",
        run_a.width,
        run_b.width
    );
    assert_eq!(run_a.glyphs.len(), run_b.glyphs.len());
    for (a, b) in run_a.glyphs.iter().zip(&run_b.glyphs) {
        assert_eq!(a.gid, b.gid);
        assert_eq!(a.cluster, b.cluster);
    }
    assert!(shaped.missing.is_empty());
    assert_eq!(run_b.font.0, times.id().content_sha256);
    assert!((run_b.width - shaped.width_pt(12.0)).abs() < 1e-9);
}

#[test]
fn paragraph_shaped_route_carries_latin_modern_two_step_ligature() {
    let Some(set) = pinned() else { return };
    let roman = set.face("lm.roman10.regular").unwrap();
    let m = FaceMetrics::new(roman);
    // Char route: f+f -> ff is reportable (U+FB00 in cmap), but ff+i -> ffi
    // is a glyph-level step the char contract cannot express, so the char
    // route measures "ff"+"i" (861) while the engine route measures the
    // ffi glyph (833) — the documented reason to prefer glyph_run.
    let char_route = shape_run(&m, 10.0, "ffi", 0);
    let (engine_route, _) = glyph_run(roman, 10.0, "ffi", 0, &ShapeOptions::default()).unwrap();
    assert!(
        (engine_route.width - 8.33).abs() < 1e-9,
        "{}",
        engine_route.width
    );
    assert!(
        char_route.width >= engine_route.width,
        "{} vs {}",
        char_route.width,
        engine_route.width
    );
    assert_eq!(engine_route.glyphs.len(), 1);
    assert_eq!(engine_route.glyphs[0].cluster, 0..3);
    // Plain words agree exactly on both routes (kerning included).
    let a = shape_run(&m, 10.0, "AVATAR", 0);
    let (b, _) = glyph_run(roman, 10.0, "AVATAR", 0, &ShapeOptions::default()).unwrap();
    assert!(
        (a.width - b.width).abs() < 1e-9,
        "{} vs {}",
        a.width,
        b.width
    );
}

#[test]
fn pdf_adapter_hands_the_writer_this_crates_program() {
    let tnr = Path::new("/System/Library/Fonts/Supplemental/Times New Roman.ttf");
    if tnr.is_file() {
        let f = load_from_path(tnr).unwrap();
        let s = shape(&f, "office AV", &ShapeOptions::default()).unwrap();
        let mut plan = EmbedPlan::new();
        plan.add_shaped(&s);
        let program = plan.finish(&f).unwrap();
        let e = to_pdf_embedded_subset(&program);
        assert_eq!(e.base_font, program.base_font);
        assert_eq!(e.units_per_em, 2048);
        assert_eq!(e.program_bytes(), program.font_file.bytes());
        match &e.program {
            Program::TrueType(sub) => {
                assert_eq!(sub.glyph_map, program.subset.as_ref().unwrap().old_to_new);
                assert_eq!(sub.advances.len(), program.cid_widths.len());
            }
            other => panic!("expected TrueType program, got {other:?}"),
        }
        // Single-char clusters are addressable through `chars`; the ligature
        // glyph is in the program and /W but has no char entry (documented).
        assert_eq!(
            e.chars.get(&'A'),
            program.cid(f.glyph_id('A').unwrap()).as_ref()
        );
        assert!(e.widths_array().starts_with("[ 0 [ "));
        assert_eq!(e.widths_array(), program.w_array());
    } else {
        eprintln!("SKIP: Times New Roman absent");
    }
    let Some(set) = pinned() else { return };
    let roman = set.face("lm.roman10.regular").unwrap();
    let s = shape(roman, "office AV", &ShapeOptions::default()).unwrap();
    let mut plan = EmbedPlan::new();
    plan.add_shaped(&s);
    let program = plan.finish(roman).unwrap();
    let e = to_pdf_embedded_subset(&program);
    match &e.program {
        Program::Cff {
            bytes,
            used_advances,
        } => {
            assert_eq!(&bytes[..], roman.cff_table().unwrap());
            assert_eq!(used_advances.len(), program.cid_widths.len());
            let a = roman.glyph_id('A').unwrap().0;
            assert_eq!(
                used_advances[&a],
                roman.advance(roman.glyph_id('A').unwrap()).unwrap()
            );
        }
        other => panic!("expected CFF program, got {other:?}"),
    }
    assert_eq!(e.units_per_em, 1000);
}

#[test]
fn math_adapter_derives_tex_parameters_from_latin_modern_math() {
    let Some(set) = pinned() else { return };
    let math = set.face("lm.math").unwrap();
    let m = OpenTypeMathFace::new(math, 10.0, MathFontId(2)).expect("MATH table");
    let p = m.params(SizeClass::Text);
    assert!((p.axis_height - 2.5).abs() < 1e-9, "{}", p.axis_height);
    assert!((p.default_rule_thickness - 0.4).abs() < 1e-9);
    assert!((p.num1 - 6.77).abs() < 1e-9);
    assert!((p.num2 - 3.94).abs() < 1e-9);
    assert!((p.denom1 - 6.86).abs() < 1e-9);
    assert!((p.sup1 - 3.63).abs() < 1e-9);
    assert!((p.sub1 - 2.47).abs() < 1e-9);
    assert!((p.quad - 10.0).abs() < 1e-9);
    let ps = m.params(SizeClass::Script);
    assert!(
        (ps.size - 7.0).abs() < 1e-9,
        "script = 70% of text: {}",
        ps.size
    );
    assert!((m.params(SizeClass::ScriptScript).size - 5.0).abs() < 1e-9);
    let f = m.glyph('f', SizeClass::Text).unwrap();
    assert_eq!(f.font_id, MathFontId(2));
    assert_eq!(f.gid, math.glyph_id('f').unwrap().0);
    assert!(
        (f.italic - 0.79).abs() < 1e-9,
        "italics correction 79 units: {}",
        f.italic
    );
    assert!(f.width > 0.0 && f.height > 0.0);
    assert!(
        m.heights_are_approximate(),
        "CFF: heights are estimates and say so"
    );
    assert!(
        m.large_operator('\u{2211}', SizeClass::Text).is_none(),
        "no MathVariants yet"
    );
    assert_eq!(m.radical_sizes(SizeClass::Text).len(), 1);
    assert_eq!(m.font_name(MathFontId(2)), "LatinModernMath-Regular");
    let roman = set.face("lm.roman10.regular").unwrap();
    assert!(
        OpenTypeMathFace::new(roman, 10.0, MathFontId(1)).is_none(),
        "text face has no MATH"
    );
}

#[test]
fn preview_export_is_deterministic_and_carries_positions() {
    let times = Core14Face::new(Core14::TimesRoman);
    let s = shape(&times, "AV fi", &ShapeOptions::default()).unwrap();
    let runs = [ExportRun {
        text: "AV fi",
        shaped: &s,
    }];
    let a = face_metrics_json(&times, 12.0, &runs);
    let b = face_metrics_json(&times, 12.0, &runs);
    assert_eq!(a, b);
    assert!(a.contains("\"schema_version\": 1"));
    assert!(a.contains(&format!("\"font_id\": \"{}\"", times.id().content_hex())));
    assert!(a.contains("\"postscript_name\": \"Times-Roman\""));
    // V sits at A's kerned advance: (722 - 135) * 12 / 1000 = 7.044 pt.
    assert!(a.contains("\"x_pt\": 7.0440"), "{a}");
    assert!(a.contains("\"text\": \"fi\""));
    assert!(a.contains("\"width_pt\":"));
}

#[test]
fn unsupported_lookups_in_a_requested_feature_fail_explicitly() {
    let arial_unicode = Path::new("/System/Library/Fonts/Supplemental/Arial Unicode.ttf");
    if !arial_unicode.is_file() {
        eprintln!("SKIP: Arial Unicode absent");
        return;
    }
    let f: TrueTypeFace = load_from_path(arial_unicode).unwrap();
    assert!(
        f.unsupported().iter().any(|u| u.feature == "mark"),
        "MarkToLigature present"
    );
    // Plain text never exercises `mark`: fine.
    assert!(shape(&f, "abc", &ShapeOptions::default()).is_ok());
    // An unattached mark does: explicit error, not an approximation.
    match shape(&f, "x\u{0301}", &ShapeOptions::default()) {
        Err(Error::UnsupportedFeature { table, feature, .. }) => {
            assert_eq!((table, feature), ("GPOS", "mark"));
        }
        other => panic!("expected UnsupportedFeature, got {other:?}"),
    }
    // Opting out keeps the approximation and the note.
    let lenient = ShapeOptions {
        fail_on_unsupported_lookups: false,
        ..ShapeOptions::default()
    };
    let s = shape(&f, "x\u{0301}", &lenient).unwrap();
    assert!(s.unsupported.iter().any(|u| u.feature == "mark"));
}
