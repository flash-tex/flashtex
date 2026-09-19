//! Core 14 metrics and shaping: values are the Adobe AFM widths, which the
//! compiler's metrics.rs on de1020c also embeds.

use flashtex_font_engine::core14::{Core14, Core14Face};
use flashtex_font_engine::shape::{ShapeOptions, shape};
use flashtex_font_engine::{Face, GlyphId, KerningSource};

fn times() -> Core14Face {
    Core14Face::new(Core14::TimesRoman)
}

#[test]
fn hello_times_roman_12pt_is_26_664() {
    // H=722 e=444 l=278 l=278 o=500 => 2222 units => 26.664 pt at 12 pt.
    let s = shape(&times(), "Hello", &ShapeOptions::PLAIN).unwrap();
    assert_eq!(s.advance_units(), 2222);
    assert!((s.width_pt(12.0) - 26.664).abs() < 1e-9);
    assert!(s.missing.is_empty());
    assert_eq!(s.clusters.len(), 5);
    for (i, c) in s.clusters.iter().enumerate() {
        assert_eq!(c.source_range, i..i + 1);
        assert_eq!(c.text, "Hello"[i..i + 1]);
    }
}

#[test]
fn mac_matches_compiler_hand_sum() {
    let s = shape(&times(), "Mac", &ShapeOptions::PLAIN).unwrap();
    assert!((s.width_pt(12.0) - 21.324).abs() < 1e-9);
}

#[test]
fn all_seven_faces_have_ascii_and_latin1_widths() {
    for which in Core14::ALL {
        let f = Core14Face::new(which);
        if which == Core14::Symbol {
            // Symbol has its own repertoire; Greek must be present.
            assert!(f.glyph_id('\u{03B1}').is_some(), "alpha in Symbol");
            assert_eq!(f.width_units('\u{03B1}'), Some(631));
            continue;
        }
        for c in (0x20u8..=0x7E).map(char::from) {
            assert!(f.glyph_id(c).is_some(), "{which:?} lacks {c:?}");
        }
        for c in (0xA0u32..=0xFF).map(|c| char::from_u32(c).unwrap()) {
            assert!(
                f.glyph_id(c).is_some(),
                "{which:?} lacks U+{:04X}",
                c as u32
            );
        }
    }
    assert_eq!(Core14Face::new(Core14::Courier).width_units('W'), Some(600));
    assert_eq!(
        Core14Face::new(Core14::Helvetica).width_units('W'),
        Some(944)
    );
    assert_eq!(
        Core14Face::new(Core14::TimesBoldItalic).width_units('W'),
        Some(889)
    );
}

#[test]
fn unicode_beyond_latin1() {
    let f = times();
    assert_eq!(f.width_units('\u{2014}'), Some(1000)); // em dash
    assert_eq!(f.width_units('\u{2019}'), Some(333)); // right single quote
    assert_eq!(f.width_units('\u{20AC}'), Some(500)); // Euro
    assert_eq!(f.width_units('\u{0394}'), Some(612)); // Greek Delta alias of U+2206
    assert!(f.glyph_id('\u{03A9}').is_none()); // Times has no Omega glyph
    let sym = Core14Face::new(Core14::Symbol);
    assert_eq!(sym.width_units('\u{03A9}'), Some(768)); // Omega alias of U+2126
    assert_eq!(sym.width_units('\u{2211}'), Some(713)); // summation
    assert!(f.glyph_id('\u{4E2D}').is_none()); // CJK: not in Core 14
}

#[test]
fn missing_glyphs_are_reported_not_replaced_by_question_mark() {
    let s = shape(&times(), "a\u{4E2D}b", &ShapeOptions::default()).unwrap();
    assert_eq!(s.missing.len(), 1);
    assert_eq!(s.missing[0].ch, '\u{4E2D}');
    assert_eq!(s.missing[0].byte_offset, 1);
    assert_eq!(s.clusters.len(), 3);
    assert_eq!(s.clusters[1].glyphs[0].gid, GlyphId::NOTDEF);
    assert_eq!(s.clusters[1].source_range, 1..4);
    assert_eq!(s.clusters[1].glyphs[0].advance, 0);
    // The '?' glyph is never substituted silently.
    let q = times().glyph_id('?').unwrap();
    assert!(s.glyphs().all(|g| g.gid != q));
}

#[test]
fn combining_acute_composes_to_precomposed_e_acute() {
    let f = times();
    let composed = shape(&f, "e\u{0301}", &ShapeOptions::default()).unwrap();
    let precomposed = shape(&f, "\u{00E9}", &ShapeOptions::default()).unwrap();
    assert_eq!(composed.clusters.len(), 1);
    assert_eq!(composed.clusters[0].glyphs.len(), 1);
    assert_eq!(
        composed.clusters[0].glyphs[0].gid,
        precomposed.clusters[0].glyphs[0].gid
    );
    assert_eq!(composed.advance_units(), precomposed.advance_units());
    assert_eq!(composed.clusters[0].source_range, 0..3);
    assert_eq!(composed.clusters[0].text, "e\u{0301}");
    assert!(composed.missing.is_empty());
}

#[test]
fn unknown_mark_with_no_precomposed_form_stays_in_cluster_and_is_missing() {
    // U+0327 combining cedilla after 'x' has no precomposed form; Core 14 has
    // no combining glyph, so it's a missing glyph inside the base's cluster.
    let s = shape(&times(), "x\u{0327}y", &ShapeOptions::default()).unwrap();
    assert_eq!(s.clusters.len(), 2);
    assert_eq!(s.clusters[0].glyphs.len(), 2);
    assert_eq!(s.clusters[0].glyphs[1].advance, 0);
    assert_eq!(s.missing.len(), 1);
    assert_eq!(s.missing[0].ch, '\u{0327}');
    assert_eq!(s.clusters[0].source_range, 0..3);
}

#[test]
fn fi_ligature_on_and_off_share_source_bytes() {
    let f = times();
    let on = shape(&f, "office", &ShapeOptions::default()).unwrap();
    let off = shape(
        &f,
        "office",
        &ShapeOptions {
            ligatures: false,
            ..ShapeOptions::default()
        },
    )
    .unwrap();
    // o f fi c e  => 5 clusters; "fi" cluster covers bytes 2..4.
    assert_eq!(on.clusters.len(), 5);
    assert_eq!(on.ligatures_applied, 1);
    let lig = &on.clusters[2];
    assert_eq!(lig.text, "fi");
    assert_eq!(lig.source_range, 2..4);
    assert_eq!(lig.glyphs.len(), 1);
    assert_eq!(lig.glyphs[0].gid, f.glyph_id('\u{FB01}').unwrap());
    assert_eq!(lig.glyphs[0].advance, 556);
    assert_eq!(off.clusters.len(), 6);
    assert_eq!(off.ligatures_applied, 0);
    assert_eq!(off.clusters[2].source_range, 2..3);
    assert_eq!(off.clusters[3].source_range, 3..4);
    // Same bytes reachable either way.
    assert_eq!(on.cluster_at_byte(3).unwrap().source_range, 2..4);
    assert_eq!(off.cluster_at_byte(3).unwrap().source_range, 3..4);
}

#[test]
fn av_kerning_is_negative_and_optional() {
    let f = times();
    let kerned = shape(&f, "AV", &ShapeOptions::default()).unwrap();
    let plain = shape(&f, "AV", &ShapeOptions::PLAIN).unwrap();
    assert_eq!(plain.advance_units(), 722 + 722);
    assert_eq!(kerned.advance_units(), 722 + 722 - 135);
    assert_eq!(kerned.kerning_source, KerningSource::Afm);
    assert_eq!(kerned.clusters[0].glyphs[0].advance, 722 - 135);
}

#[test]
fn unsupported_scripts_fail_closed() {
    let err = shape(&times(), "ab\u{05D0}", &ShapeOptions::default()).unwrap_err();
    match err {
        flashtex_font_engine::Error::UnsupportedScript {
            ch, byte_offset, ..
        } => {
            assert_eq!(ch, '\u{05D0}');
            assert_eq!(byte_offset, 2);
        }
        other => panic!("expected UnsupportedScript, got {other:?}"),
    }
    assert!(shape(&times(), "a\u{0627}", &ShapeOptions::default()).is_err());
    assert!(shape(&times(), "a\u{202E}b", &ShapeOptions::default()).is_err());
    assert!(shape(&times(), "\u{0915}", &ShapeOptions::default()).is_err());
}

#[test]
fn default_ignorables_keep_their_bytes_without_glyphs() {
    let s = shape(&times(), "a\u{200B}b", &ShapeOptions::default()).unwrap();
    assert_eq!(s.clusters.len(), 3);
    assert!(s.clusters[1].glyphs.is_empty());
    assert_eq!(s.clusters[1].source_range, 1..4);
    assert!(s.missing.is_empty());
    assert_eq!(s.advance_units(), 444 + 500);
}

#[test]
fn shaping_empty_text_yields_no_clusters() {
    // A bounded/degenerate input: zero scalars in, zero clusters out, never
    // an error or a panic.
    let s = shape(&times(), "", &ShapeOptions::default()).unwrap();
    assert!(s.clusters.is_empty());
    assert!(s.missing.is_empty());
    assert_eq!(s.advance_units(), 0);
}

#[test]
fn leading_combining_mark_without_a_base_is_reported_missing() {
    // U+0301 COMBINING ACUTE ACCENT with nothing before it: mark composition
    // only fires against a preceding cluster, so this must fall through to
    // ordinary mapping rather than panic on an empty `clusters` list. Times
    // Roman's AFM has no standalone glyph for it, so it becomes .notdef and
    // is listed as missing, still occupying its own one-glyph cluster.
    let s = shape(&times(), "\u{0301}", &ShapeOptions::default()).unwrap();
    assert_eq!(s.clusters.len(), 1);
    assert_eq!(
        s.missing,
        vec![flashtex_font_engine::shape::MissingGlyph {
            ch: '\u{0301}',
            byte_offset: 0,
        }]
    );
    assert_eq!(s.clusters[0].glyphs.len(), 1);
    assert_eq!(s.clusters[0].glyphs[0].gid, GlyphId::NOTDEF);
    assert_eq!(s.clusters[0].glyphs[0].advance, 0);
    assert_eq!(s.clusters[0].source_range, 0..2); // U+0301 is 2 UTF-8 bytes.

    // A mark with no base never retroactively attaches to what follows it
    // either: it stays its own cluster, and the base after it starts fresh.
    let s2 = shape(&times(), "\u{0301}a", &ShapeOptions::default()).unwrap();
    assert_eq!(s2.clusters.len(), 2);
    assert_eq!(s2.clusters[1].text, "a");
}

#[test]
fn cluster_hit_testing() {
    let s = shape(&times(), "AV", &ShapeOptions::default()).unwrap();
    assert_eq!(s.cluster_at_x(0).unwrap().text, "A");
    assert_eq!(s.cluster_at_x(722 - 135).unwrap().text, "V");
    assert!(s.cluster_at_x(722 - 135 + 722).is_none());
}

#[test]
fn vertical_metrics_come_from_afm_headers() {
    let f = times();
    let vm = f.vertical_metrics();
    assert_eq!(vm.ascender, 683);
    assert_eq!(vm.descender, -217);
    assert_eq!(vm.cap_height, 662);
    assert_eq!(vm.x_height, 450);
    assert_eq!(f.bbox(), [-168, -218, 1000, 898]);
    assert_eq!(f.italic_angle(), 0.0);
    assert_eq!(Core14Face::new(Core14::TimesItalic).italic_angle(), -15.5);
    assert_eq!(f.postscript_name(), "Times-Roman");
    assert!(Core14Face::new(Core14::Courier).is_fixed_pitch());
}

#[test]
fn courier_never_ligates_even_though_its_afm_has_fi() {
    let f = Core14Face::new(Core14::Courier);
    assert!(f.glyph_id('\u{FB01}').is_some());
    let s = shape(&f, "fi fl", &ShapeOptions::default()).unwrap();
    assert_eq!(s.ligatures_applied, 0);
    assert_eq!(s.advance_units(), 5 * 600);
}
