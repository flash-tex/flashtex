use flashtex_font_engine::{ShapeOptions, TrueTypeFace};
use flashtex_font_resources::{cff::*, engine_adapter::*, sha256};
#[test]
#[ignore = "requires exact pinned installed STIX and OFL; direct peer equivalence, not visual oracle"]
fn pinned_stix_engine_adapter_equivalence_and_cache_identity() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/stix-cff.json")).unwrap();
    let bytes = std::fs::read(fixture["font_path"].as_str().unwrap()).unwrap();
    assert_eq!(sha256(&bytes), fixture["font_sha256"].as_str().unwrap());
    assert_eq!(
        sha256(&std::fs::read(fixture["license_path"].as_str().unwrap()).unwrap()),
        fixture["license_sha256"].as_str().unwrap()
    );
    let cache = CffOutlineCache::from_font_table(
        &bytes,
        0,
        5644..177326,
        CacheLimits {
            max_entries: 0,
            max_bytes: 0,
        },
    )
    .unwrap();
    let adapter = EngineFontAdapter::from_cff(&bytes, &cache).unwrap();
    let direct = TrueTypeFace::parse(bytes.clone()).unwrap();
    let source = "東京 prefix: AV office e\u{301} — end";
    let start = source.find("AV").unwrap();
    let hash = sha256(source.as_bytes());
    let request = || ShapeRequest {
        source,
        source_sha256: &hash,
        path: "chapters/test.tex",
        revision: 7,
        range: start..source.len(),
        font_sha256: &cache.identity().font_sha256,
        face_index: 0,
        encoding: InputEncoding::Unicode,
        variation_coordinates: &[],
        options: ShapeOptions::PLAIN,
    };
    let mut strict = request();
    strict.options = ShapeOptions::default();
    assert!(matches!(
        adapter.shape(strict),
        Err(flashtex_font_resources::Error::UnsupportedFont(_))
    ));
    let result = adapter.shape(request()).unwrap();
    let expected =
        flashtex_font_engine::shape::shape(&direct, &source[start..], &ShapeOptions::PLAIN)
            .unwrap();
    assert_eq!(result.shaped(), &expected);
    assert_eq!(adapter.shape(request()).unwrap(), result);
    for (i, cluster) in result.shaped().clusters.iter().enumerate() {
        assert_eq!(
            &source[result.absolute_cluster_range(i).unwrap()],
            cluster.text
        );
    }
    assert_eq!(result.shaped().ligatures_applied, 0);
    assert_ne!(
        result.identity().font_sha256,
        result.identity().engine_font_id.content_hex()
    );
    for choice in 0..4 {
        let mut request = request();
        match choice {
            0 => request.revision += 1,
            1 => request.path = "other.tex",
            2 => request.options.fail_on_unsupported_lookups = true,
            _ => request.range.end -= 1,
        };
        assert_ne!(
            adapter.shape(request).unwrap().cache_key(),
            result.cache_key()
        );
    }
    let mut wrong = request();
    wrong.range.start = 1;
    assert!(adapter.shape(wrong).is_err());
    let mut wrong = request();
    wrong.font_sha256 = "bad";
    assert!(adapter.shape(wrong).is_err());
    let mut mutated = bytes.clone();
    mutated[0] ^= 1;
    assert!(EngineFontAdapter::from_cff(&mutated, &cache).is_err());
    for text in ["A\u{FE0F}", "\u{10FFFF}"] {
        let hash = sha256(text.as_bytes());
        let mut r = request();
        r.source = text;
        r.source_sha256 = &hash;
        r.range = 0..text.len();
        assert!(adapter.shape(r).is_err());
    }
    eprintln!(
        "peer={} fontSHA={} engineSHA={} clusters={} glyphs={} ligatures={} advance={} cacheKey={}",
        ENGINE_PEER_REVISION,
        result.identity().font_sha256,
        result.identity().engine_font_id.content_hex(),
        result.shaped().clusters.len(),
        result.shaped().glyphs().count(),
        result.shaped().ligatures_applied,
        result.shaped().advance_units(),
        result.cache_key()
    );
}

#[test]
#[ignore = "requires pinned installed LiberationSans and license; peer equivalence only"]
fn pinned_liberation_default_shaper_equivalence() {
    use flashtex_font_engine::Face;
    use flashtex_font_resources::*;
    let bytes =
        std::fs::read("/usr/share/fonts/liberation-sans-fonts/LiberationSans-Regular.ttf").unwrap();
    assert_eq!(
        sha256(&bytes),
        "76d04c18ea243f426b7de1f3ad208e927008f961dc5945e5aad352d0dfde8ee8"
    );
    let license = std::fs::read("/usr/share/licenses/liberation-sans-fonts/LICENSE").unwrap();
    assert_eq!(
        sha256(&license),
        "93fed46019c38bbe566b479d22148e2e8a1e85ada614accb0211c37b2c61c19b"
    );
    let direct = TrueTypeFace::parse(bytes.clone()).unwrap();
    let entry = ManifestEntry {
        font: FontDescriptor {
            font_id: "liberation".into(),
            sha256: sha256(&bytes),
            byte_length: bytes.len() as u64,
            format: "static-truetype".into(),
            face_index: 0,
            units_per_em: direct.units_per_em() as u32,
            glyph_count: direct.num_glyphs() as u32,
            postscript_name: direct.postscript_name().into(),
        },
        path: "font.ttf".into(),
        license: LicenseMetadata {
            identifier: "OFL-1.1".into(),
            copyright: "Liberation font authors; see pinned license".into(),
            source: "installed liberation-sans-fonts".into(),
            text_path: "LICENSE".into(),
            text_sha256: sha256(&license),
            embedding_permission: EmbeddingPermission::Allowed,
        },
    };
    let resource = FontResource::from_bytes(&entry, &bytes, &license).unwrap();
    let adapter = EngineFontAdapter::from_resource(&resource).unwrap();
    let source = "AV office e\u{301}";
    let hash = sha256(source.as_bytes());
    let options = ShapeOptions::default();
    let run = adapter
        .shape(ShapeRequest {
            source,
            source_sha256: &hash,
            path: "main.tex",
            revision: 2,
            range: 0..source.len(),
            font_sha256: &entry.font.sha256,
            face_index: 0,
            encoding: InputEncoding::Unicode,
            variation_coordinates: &[],
            options,
        })
        .unwrap();
    assert_eq!(
        run.shaped(),
        &flashtex_font_engine::shape::shape(&direct, source, &options).unwrap()
    );
    assert!(run
        .shaped()
        .clusters
        .iter()
        .any(|c| c.text == "e\u{301}" && c.glyphs.len() == 1));
    eprintln!(
        "Liberation default clusters={} glyphs={} ligatures={} advance={} peer_source={}",
        run.shaped().clusters.len(),
        run.shaped().glyphs().count(),
        run.shaped().ligatures_applied,
        run.shaped().advance_units(),
        ENGINE_SOURCE_SHA256
    );
}
