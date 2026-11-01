use flashtex_font_engine::{Face, TrueTypeFace};
use flashtex_font_resources::{registry::*, *};
use flashtex_project_files::ProjectRoot;
fn declaration(bytes: &[u8], license: &[u8], id: &str, format: &str) -> ManifestEntry {
    let face = TrueTypeFace::parse(bytes.to_vec()).unwrap();
    ManifestEntry {
        font: FontDescriptor {
            font_id: id.into(),
            sha256: sha256(bytes),
            byte_length: bytes.len() as u64,
            format: format.into(),
            face_index: 0,
            units_per_em: face.units_per_em() as u32,
            glyph_count: face.num_glyphs() as u32,
            postscript_name: face.postscript_name().into(),
        },
        path: format!("{id}.font"),
        license: LicenseMetadata {
            identifier: "OFL-1.1".into(),
            copyright: "see exact pinned license".into(),
            source: "installed licensed resource".into(),
            text_path: format!("{id}.license"),
            text_sha256: sha256(license),
            embedding_permission: EmbeddingPermission::Allowed,
        },
    }
}
#[test]
#[ignore = "requires pinned installed STIX/Liberation and licenses; temporary project only"]
fn mixed_registry_identity_generation_and_immutable_cff() {
    let dir = tempfile::tempdir().unwrap();
    let root = ProjectRoot::open(dir.path()).unwrap();
    let mut entries = Vec::new();
    for (id, format, font, license, font_hash, license_hash) in [
        (
            "stix",
            "static-cff",
            "/usr/share/fonts/stix-fonts/STIXTwoText-Regular.otf",
            "/usr/share/licenses/stix-fonts/OFL.txt",
            "c4864ca6ec071c2d31d0d8309001faa1ee3517fffb53a31a405a697b71f52ca1",
            "0c8825913b60d858aacdb33c4ca6660a7d64b0d6464702efbb19313f5765861a",
        ),
        (
            "liberation",
            "static-truetype",
            "/usr/share/fonts/liberation-sans-fonts/LiberationSans-Regular.ttf",
            "/usr/share/licenses/liberation-sans-fonts/LICENSE",
            "76d04c18ea243f426b7de1f3ad208e927008f961dc5945e5aad352d0dfde8ee8",
            "93fed46019c38bbe566b479d22148e2e8a1e85ada614accb0211c37b2c61c19b",
        ),
    ] {
        let bytes = std::fs::read(font).unwrap();
        let license = std::fs::read(license).unwrap();
        assert_eq!(sha256(&bytes), font_hash);
        assert_eq!(sha256(&license), license_hash);
        let resource = declaration(&bytes, &license, id, format);
        std::fs::write(dir.path().join(&resource.path), bytes).unwrap();
        std::fs::write(dir.path().join(&resource.license.text_path), license).unwrap();
        entries.push(RegistryEntry {
            binding: StyleBinding {
                family: id.into(),
                weight: 400,
                style: FontStyle::Upright,
            },
            resource,
        });
    }
    let mut manifest = RegistryManifest {
        schema_version: 1,
        entries,
    };
    let write = |m: &RegistryManifest| {
        std::fs::write(
            dir.path().join("fonts.json"),
            serde_json::to_vec(m).unwrap(),
        )
        .unwrap()
    };
    write(&manifest);
    let load = || ProjectFontRegistry::load(&root, "fonts.json", RegistryLimits::default());
    let first = load().unwrap();
    assert_eq!(first.files_read(), 5);
    let export = first.export_json(1024 * 1024).unwrap();
    std::fs::write(dir.path().join("fonts.json"), &export).unwrap();
    assert_eq!(load().unwrap().export_json(1024 * 1024).unwrap(), export);
    let page = first
        .enumerate(first.generation(), MetadataFilter::default(), 0, 64)
        .unwrap();
    assert_eq!(
        page.entries
            .iter()
            .filter(|e| e.cff_table.is_some())
            .count(),
        1
    );
    let stix = manifest.entries[0].binding.clone();
    let RegistryResource::Cff(held) = first.resource(&stix).unwrap() else {
        panic!("CFF backend")
    };
    assert!(first.get(&stix).is_err());
    assert!(first.get(&manifest.entries[1].binding).is_ok());
    assert_eq!(held.identity().table_range, 5644..177326);
    assert_eq!(
        held.identity().cff_sha256,
        "c5d11bab6a95e75a568e1b72fd30fdd5e4c95abe68a72f02c0c4329ee948b532"
    );
    let mut cache = held
        .outline_cache(cff::CacheLimits {
            max_entries: 2,
            max_bytes: 1_000_000,
        })
        .unwrap();
    let glyph = cache.lookup(3, cff::HintPolicy::Unhinted).result.unwrap();
    assert_eq!(glyph.cff_sha256, held.identity().cff_sha256);
    assert_eq!(
        cache.lookup(3, cff::HintPolicy::Unhinted).status,
        cff::CacheStatus::Hit
    );
    assert_eq!(
        held.shape_adapter().unwrap().identity().font_sha256,
        held.identity().font_sha256
    );
    manifest.entries.reverse();
    write(&manifest);
    assert_eq!(load().unwrap().generation(), first.generation());
    let stix_index = manifest
        .entries
        .iter()
        .position(|e| e.binding == stix)
        .unwrap();
    manifest.entries[stix_index].resource.license.source = "updated declared provenance".into();
    write(&manifest);
    let updated = load().unwrap();
    assert!(updated.require_generation(first.generation()).is_err());
    let mut wrong = manifest.clone();
    wrong.entries[stix_index].resource.font.face_index = 1;
    write(&wrong);
    assert!(load().is_err());
    wrong = manifest.clone();
    wrong.entries[stix_index].resource.font.glyph_count -= 1;
    write(&wrong);
    assert!(load().is_err());
    wrong = manifest.clone();
    wrong.entries[stix_index].resource.font.format = "static-truetype".into();
    write(&wrong);
    assert!(load().is_err());
    write(&manifest);
    std::fs::write(dir.path().join("stix.font"), b"changed").unwrap();
    assert!(matches!(
        load(),
        Err(RegistryError::ResourceMismatch { .. })
    ));
    assert_eq!(sha256(held.bytes()), held.identity().font_sha256);
    assert_eq!(
        held.outline_cache(cff::CacheLimits {
            max_entries: 0,
            max_bytes: 0
        })
        .unwrap()
        .lookup(3, cff::HintPolicy::Unhinted)
        .result
        .unwrap(),
        glyph
    );
    eprintln!("mixed registry initial={} updated={} sourcebytes={} files={} exact CFF identity/immutable replay verified",first.generation(),updated.generation(),first.loaded_bytes(),first.files_read());
}
