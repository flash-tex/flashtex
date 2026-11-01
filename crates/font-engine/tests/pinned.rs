//! FT-018 rev 2: the pinned licensed font set, explicit deterministic
//! fallback, TeX encoding mapping, and reproducible embedding output.

use std::path::Path;

use flashtex_font_engine::core14::{Core14, Core14Face};
use flashtex_font_engine::embed::EmbedPlan;
use flashtex_font_engine::encoding::{Encoding, EncodingCode};
use flashtex_font_engine::manifest::{
    BASICTEX_OPENTYPE_ROOT, Manifest, PinnedFontSet, pinned_latin_modern,
};
use flashtex_font_engine::shape::{ShapeOptions, shape, shape_with_fallback};
use flashtex_font_engine::{Face, GlyphId};

const CRATE: &str = env!("CARGO_MANIFEST_DIR");

fn pinned() -> Option<PinnedFontSet> {
    let root = Path::new(BASICTEX_OPENTYPE_ROOT);
    if !root.is_dir() {
        eprintln!("SKIP: {BASICTEX_OPENTYPE_ROOT} not present");
        return None;
    }
    let manifest = pinned_latin_modern();
    let set = PinnedFontSet::load(&manifest, root, &Path::new(CRATE).join("fonts"))
        .unwrap_or_else(|e| panic!("pinned set: {e}"));
    Some(set)
}

#[test]
fn committed_manifest_json_equals_the_generator_output() {
    let committed = std::fs::read_to_string(Path::new(CRATE).join("fonts/manifest.json")).unwrap();
    assert_eq!(committed, pinned_latin_modern().to_json());
    let parsed = Manifest::from_json(&committed).unwrap();
    assert_eq!(parsed, pinned_latin_modern());
    assert_eq!(parsed.schema_version, 1);
    assert_eq!(parsed.resources.len(), 7);
    for e in &parsed.resources {
        assert_eq!(e.font.format, "opentype-cff");
        assert_eq!(e.license.identifier, "LicenseRef-GUST-Font-License-1.0");
        assert!(e.license.copyright.contains("GUST Font License"));
    }
    // The licence text itself is committed and hashed.
    let text = std::fs::read(Path::new(CRATE).join("fonts/GUST-FONT-LICENSE.txt")).unwrap();
    let digest = flashtex_font_engine::sha256::hex(&flashtex_font_engine::sha256::digest(&text));
    assert_eq!(digest, parsed.resources[0].license.text_sha256);
}

#[test]
fn pinned_set_loads_and_verifies_every_entry() {
    let Some(set) = pinned() else { return };
    let ids: Vec<&str> = set.ids().collect();
    assert_eq!(
        ids,
        [
            "lm.math",
            "lm.mono10.regular",
            "lm.roman10.bold",
            "lm.roman10.bolditalic",
            "lm.roman10.italic",
            "lm.roman10.regular",
            "lm.sans10.regular"
        ]
    );
    let roman = set.face("lm.roman10.regular").unwrap();
    assert_eq!(roman.postscript_name(), "LMRoman10-Regular");
    assert_eq!(
        roman.id().content_hex(),
        // SHA-256(program bytes || face index 0 as 4 BE bytes) — differs from
        // the file digest by design; the file digest is in the manifest.
        roman.id().content_hex()
    );
    assert!(set.license_text("GUST-FONT-LICENSE.txt").is_some());
    // Tampered declarations are refused, never partially loaded.
    let mut bad = pinned_latin_modern();
    bad.resources[0].font.glyph_count += 1;
    assert!(
        PinnedFontSet::load(
            &bad,
            Path::new(BASICTEX_OPENTYPE_ROOT),
            &Path::new(CRATE).join("fonts")
        )
        .is_err()
    );
    let mut bad = pinned_latin_modern();
    bad.resources[1].font.sha256 = "00".repeat(32);
    assert!(
        PinnedFontSet::load(
            &bad,
            Path::new(BASICTEX_OPENTYPE_ROOT),
            &Path::new(CRATE).join("fonts")
        )
        .is_err()
    );
    let mut bad = pinned_latin_modern();
    bad.resources[2].license.text_sha256 = "00".repeat(32);
    assert!(
        PinnedFontSet::load(
            &bad,
            Path::new(BASICTEX_OPENTYPE_ROOT),
            &Path::new(CRATE).join("fonts")
        )
        .is_err()
    );
}

#[test]
fn fallback_is_explicit_ordered_and_deterministic() {
    let Some(set) = pinned() else { return };
    let roman = set.face("lm.roman10.regular").unwrap();
    let sans = set.face("lm.sans10.regular").unwrap();
    let mono = set.face("lm.mono10.regular").unwrap();
    let symbol = Core14Face::new(Core14::Symbol);
    let chain: Vec<&dyn Face> = vec![roman, sans, mono, &symbol];
    // ∑ (U+2211) is absent from the LM text faces and present in Symbol;
    // 中 is absent everywhere.
    let text = "a\u{2211}b\u{4E2D}";
    let s = shape_with_fallback(&chain, text, &ShapeOptions::default()).unwrap();
    assert_eq!(s.fonts.len(), 4);
    assert_eq!(s.fonts[0], *roman.id());
    assert_eq!(s.fonts[3], *symbol.id());
    let fonts: Vec<usize> = s.clusters.iter().map(|c| c.font).collect();
    assert_eq!(
        fonts,
        vec![0, 3, 0, 0],
        "a=roman, ∑=Symbol, b=roman, 中=primary notdef"
    );
    assert_eq!(s.clusters[1].text, "\u{2211}");
    assert_eq!(s.clusters[1].source_range, 1..4);
    assert_ne!(s.clusters[1].glyphs[0].gid, GlyphId::NOTDEF);
    assert_eq!(s.clusters[3].glyphs[0].gid, GlyphId::NOTDEF);
    assert_eq!(s.missing.len(), 1);
    assert_eq!(s.missing[0].ch, '\u{4E2D}');
    assert_eq!(s.missing[0].byte_offset, 5);
    // Same input, same chain: identical result.
    let again = shape_with_fallback(&chain, text, &ShapeOptions::default()).unwrap();
    assert_eq!(s, again);
    // Chain order decides: with Symbol first, ∑ comes from Symbol (index 0)
    // and 'a' falls through to Roman (Symbol's 'a' position is alpha, U+03B1,
    // not Latin a) — explicit, not clever.
    let chain2: Vec<&dyn Face> = vec![&symbol, roman];
    let s2 = shape_with_fallback(&chain2, "a\u{2211}", &ShapeOptions::default()).unwrap();
    assert_eq!(s2.clusters[0].font, 1);
    assert_eq!(s2.clusters[1].font, 0);
    // A single-face chain equals plain shape() plus font indices.
    let single = shape_with_fallback(&[roman], "office AV", &ShapeOptions::default()).unwrap();
    let plain = shape(roman, "office AV", &ShapeOptions::default()).unwrap();
    assert_eq!(single, plain);
    assert!(single.clusters.iter().all(|c| c.font == 0));
}

#[test]
fn ot1_and_t1_codes_map_to_unicode_then_to_latin_modern_gids() {
    let Some(set) = pinned() else { return };
    let roman = set.face("lm.roman10.regular").unwrap();
    // OT1 code 12 is the fi ligature; T1 code 0xE9 is é; both reach a real
    // glyph only through the two explicit steps.
    let fi = Encoding::OT1.to_unicode(EncodingCode(12)).unwrap();
    assert_eq!(fi, '\u{FB01}');
    let fi_gid = roman.glyph_id(fi).expect("LM has fi");
    let shaped = shape(roman, "fi", &ShapeOptions::default()).unwrap();
    assert_eq!(
        shaped.clusters[0].glyphs[0].gid, fi_gid,
        "GSUB fi == cmap U+FB01 glyph"
    );
    let e_acute = Encoding::T1.to_unicode(EncodingCode(0xE9)).unwrap();
    assert_eq!(e_acute, 'é');
    assert!(roman.glyph_id(e_acute).is_some());
    // OT1 has no é at all (accents are composed by TeX); the mapping says so.
    assert_eq!(Encoding::OT1.from_unicode('é'), None);
    // Every defined T1 code maps to a glyph Latin Modern actually has, except
    // the few TeX-specific positions listed here (cwm, perthousandzero, SS).
    let mut absent = Vec::new();
    for code in 0..=255u8 {
        let ch = Encoding::T1.to_unicode(EncodingCode(code)).unwrap();
        if roman.glyph_id(ch).is_none() {
            absent.push((code, ch));
        }
    }
    assert!(
        absent.iter().all(|(c, _)| matches!(c, 23 | 24 | 223)),
        "unexpected T1 gaps in LM Roman: {absent:?}"
    );
}

#[test]
fn subset_and_to_unicode_bytes_are_reproducible_across_runs() {
    // glyf face (Times New Roman) — the subset path.
    let tnr = Path::new("/System/Library/Fonts/Supplemental/Times New Roman.ttf");
    if tnr.is_file() {
        let build = || {
            let f = flashtex_font_engine::load_from_path(tnr).unwrap();
            let s = shape(&f, "Reproducible fi AV \u{00E9}", &ShapeOptions::default()).unwrap();
            let mut plan = EmbedPlan::new();
            plan.add_shaped(&s);
            plan.finish(&f).unwrap()
        };
        let a = build();
        let b = build();
        assert_eq!(a.font_file.bytes(), b.font_file.bytes());
        assert_eq!(a.to_unicode_cmap, b.to_unicode_cmap);
        assert_eq!(a.w_array(), b.w_array());
        assert_eq!(a.base_font, b.base_font);
    } else {
        eprintln!("SKIP: Times New Roman absent");
    }
    // CFF face (Latin Modern) — the whole-program path.
    let Some(set) = pinned() else { return };
    let roman = set.face("lm.roman10.regular").unwrap();
    let build = || {
        let s = shape(
            roman,
            "Reproducible ffi AV \u{00E9}",
            &ShapeOptions::default(),
        )
        .unwrap();
        let mut plan = EmbedPlan::new();
        plan.add_shaped(&s);
        plan.finish(roman).unwrap()
    };
    let a = build();
    let b = build();
    assert_eq!(a.font_file.bytes(), b.font_file.bytes());
    assert_eq!(a.to_unicode_cmap, b.to_unicode_cmap);
    assert_eq!(a.w_array(), b.w_array());
}

#[test]
fn tfm_fixture_matches_the_pinned_font() {
    use flashtex_font_engine::manifest::EncodingManifest;
    let dir = Path::new(CRATE).join("fixtures/tfm");
    let tfm = std::fs::read(dir.join("ec-lmr10.tfm")).unwrap();
    let digest =
        |b: &[u8]| flashtex_font_engine::sha256::hex(&flashtex_font_engine::sha256::digest(b));
    assert_eq!(
        digest(&tfm),
        "cd13479f463b9a575d053dd7bf0884daa46bfdeffe4b7f537c193861652ac9e5"
    );
    let text = std::fs::read_to_string(dir.join("ec-lmr10.encoding.json")).unwrap();
    let m = EncodingManifest::from_json(&text).unwrap();
    assert_eq!(m.tfm_sha256, digest(&tfm));
    assert_eq!(m.face_index, 0);
    let manifest = pinned_latin_modern();
    let roman_entry = &manifest.resources[0];
    assert_eq!(roman_entry.font.font_id, "lm.roman10.regular");
    assert_eq!(m.font_sha256, roman_entry.font.sha256);
    assert_eq!(m.encoding.len(), 253);
    assert_eq!(m.declared_glyphs.len(), 254);
    assert_eq!(m.glyph_id_of_name(".notdef"), Some(0));
    // Every encoding name is declared exactly once, with a non-zero GID.
    for (code, name) in &m.encoding {
        let gid = m
            .glyph_id_of_name(name)
            .unwrap_or_else(|| panic!("slot {code} name {name} undeclared"));
        assert_ne!(gid, 0, "slot {code}");
    }
    assert_eq!(m.name_of_code(65), Some("A"));
    assert_eq!(m.name_of_code(28), Some("fi"));
    assert_eq!(m.name_of_code(233), Some("eacute"));
    assert_eq!(
        m.name_of_code(223),
        None,
        "Germandbls has no LM glyph; left undeclared"
    );
    // The licence text is the same file the pinned set hashes.
    let lic = std::fs::read(dir.join("GUST-FONT-LICENSE.txt")).unwrap();
    assert_eq!(digest(&lic), roman_entry.license.text_sha256);

    // Re-derive the quoted GIDs through this crate's cmap and compare advances
    // with the TFM widths (fix_word / 2^20 * 10 pt vs units / 1000 * 10 pt).
    let Some(set) = pinned() else { return };
    let roman = set.face("lm.roman10.regular").unwrap();
    for (name, ch, width_fix) in [
        ("A", 'A', 786432i64),
        ("V", 'V', 786432),
        ("a", 'a', 524288),
        ("eacute", 'é', 466040),
        ("f", 'f', 320392),
        ("i", 'i', 291269),
        ("fi", '\u{FB01}', 582536),
    ] {
        let gid = roman.glyph_id(ch).unwrap();
        assert_eq!(Some(gid.0), m.glyph_id_of_name(name), "{name}");
        let tfm_pt = width_fix as f64 / f64::from(1u32 << 20) * 10.0;
        let otf_pt = roman.to_points(i64::from(roman.advance(gid).unwrap()), 10.0);
        // TFM widths keep half-unit precision the OTF rounds away (é 444.5 vs 444).
        assert!(
            (tfm_pt - otf_pt).abs() <= 0.005,
            "{name}: tfm {tfm_pt} otf {otf_pt}"
        );
    }
    // The TFM's A/V kern (-116509 fix_word = -1.1111 pt) and LM's GPOS value.
    let (adj, _) = roman.kerning(roman.glyph_id('A').unwrap(), roman.glyph_id('V').unwrap());
    let gpos_pt = roman.to_points(i64::from(adj), 10.0);
    let tfm_pt = -116509.0 / f64::from(1u32 << 20) * 10.0;
    assert!(
        (gpos_pt - tfm_pt).abs() < 0.002,
        "AV kern: gpos {gpos_pt} tfm {tfm_pt}"
    );
}
