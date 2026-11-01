//! Matching font bytes are opt-in; TFM/encoding/license already live in the peer fixture.
use flashtex_font_engine::{Face, Outlines, TrueTypeFace};
use flashtex_font_resources::{
    encoding::MappedItem,
    registry::{vf_project::*, *},
    *,
};
#[test]
#[ignore = "set FLASHTEX_LM_FONT to exact published lmroman10-regular.otf; no substitute font"]
fn matching_latin_modern_rooted_encoding_run() {
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../font-engine/fixtures/tfm");
    let font_bytes =
        std::fs::read(std::env::var("FLASHTEX_LM_FONT").expect("exact matching font path"))
            .unwrap();
    assert_eq!(
        sha256(&font_bytes),
        "1aa18cfefa58132c52ce5de70db1fd1154201c19cd2b2cdaffba4906a33e6852"
    );
    let tfm = std::fs::read(fixtures.join("ec-lmr10.tfm")).unwrap();
    assert_eq!(
        sha256(&tfm),
        "cd13479f463b9a575d053dd7bf0884daa46bfdeffe4b7f537c193861652ac9e5"
    );
    let enc = std::fs::read(fixtures.join("lm-ec.enc")).unwrap();
    assert_eq!(
        sha256(&enc),
        "7f9932c402d22a937b853406cfdf4166b80260e3ff21a03fe9a4c05105a2918c"
    );
    let license = std::fs::read(fixtures.join("GUST-FONT-LICENSE.txt")).unwrap();
    assert_eq!(
        sha256(&license),
        "49ea6cb9257bbee0a3979c48a774cd221550ac1c20c95549efe45fc99cc18050"
    );
    let dir = tempfile::tempdir().unwrap();
    for (path, bytes) in [
        ("font.otf", font_bytes.as_slice()),
        ("font.tfm", tfm.as_slice()),
        ("font.enc", enc.as_slice()),
        ("LICENSE", license.as_slice()),
    ] {
        std::fs::write(dir.path().join(path), bytes).unwrap();
    }
    let face = TrueTypeFace::parse(font_bytes.clone()).unwrap();
    let metadata = LicenseMetadata {
        identifier: "LicenseRef-GUST-Font-License".into(),
        copyright: "see pinned license".into(),
        source: "published Latin Modern fixture".into(),
        text_path: "LICENSE".into(),
        text_sha256: sha256(&license),
        embedding_permission: EmbeddingPermission::Allowed,
    };
    let resource = ManifestEntry {
        path: "font.otf".into(),
        license: metadata.clone(),
        font: FontDescriptor {
            font_id: "latin-modern-roman10".into(),
            sha256: sha256(&font_bytes),
            byte_length: font_bytes.len() as u64,
            format: match face.outlines() {
                Outlines::Glyf => "static-truetype",
                Outlines::Cff => "static-cff",
            }
            .into(),
            face_index: 0,
            units_per_em: face.units_per_em().into(),
            glyph_count: face.num_glyphs().into(),
            postscript_name: face.postscript_name().into(),
        },
    };
    let binding = StyleBinding {
        family: "Explicit LM".into(),
        weight: 400,
        style: FontStyle::Upright,
    };
    std::fs::write(
        dir.path().join("fonts.json"),
        serde_json::to_vec(&RegistryManifest {
            schema_version: 1,
            entries: vec![RegistryEntry {
                binding: binding.clone(),
                resource,
            }],
        })
        .unwrap(),
    )
    .unwrap();
    let root = flashtex_project_files::ProjectRoot::open(dir.path()).unwrap();
    let registry = ProjectFontRegistry::load(&root, "fonts.json", Default::default()).unwrap();
    let tfm_asset = Asset {
        path: "font.tfm".into(),
        sha256: sha256(&tfm),
        license: metadata.clone(),
    };
    let enc_asset = Asset {
        path: "font.enc".into(),
        sha256: sha256(&enc),
        license: metadata,
    };
    let known: encoding::EncodingManifest =
        serde_json::from_slice(&std::fs::read(fixtures.join("ec-lmr10.encoding.json")).unwrap())
            .unwrap();
    let cff_identity = match registry.resource(&binding).unwrap() {
        RegistryResource::Cff(r) => Some(r.identity().clone()),
        _ => None,
    };
    let declarations = cff_identity.map(|identity| enc_file::CffMappingDeclarations {
        encoding_file_sha256: sha256(&enc),
        font_sha256: identity.font_sha256,
        cff_sha256: identity.cff_sha256,
        face_index: 0,
        aliases: [
            ("ff", "f_f"),
            ("fi", "f_i"),
            ("fl", "f_l"),
            ("ffi", "f_f_i"),
            ("ffl", "f_f_l"),
        ]
        .into_iter()
        .map(|(literal, target)| enc_file::GlyphNameAlias {
            literal_name: literal.into(),
            font_name: target.into(),
            original_gid: known
                .declared_glyphs
                .iter()
                .find(|g| g.glyph_name == literal)
                .unwrap()
                .glyph_id,
        })
        .collect(),
        unavailable_slots: [(156, "IJ"), (188, "ij"), (223, "Germandbls")]
            .into_iter()
            .map(|(code, name)| enc_file::UnavailableSlot {
                code,
                literal_name: name.into(),
            })
            .collect(),
    });
    let node = match face.outlines() {
        Outlines::Cff => Node::CffPhysicalEncodingAsset {
            declarations,
            id: "lm".into(),
            tfm: tfm_asset,
            binding,
            encoding_asset: enc_asset,
        },
        Outlines::Glyf => {
            let mapped: encoding::EncodingManifest = serde_json::from_slice(
                &std::fs::read(fixtures.join("ec-lmr10.encoding.json")).unwrap(),
            )
            .unwrap();
            Node::PhysicalEncodingAsset {
                id: "lm".into(),
                tfm: tfm_asset,
                binding,
                encoding_asset: enc_asset,
                declared_glyphs: mapped.declared_glyphs,
            }
        }
    };
    let manifest = DependencyManifest {
        schema_version: 2,
        registry_generation: registry.generation().into(),
        root: "lm".into(),
        nodes: vec![node],
    };
    let write = |m: &DependencyManifest| {
        std::fs::write(dir.path().join("deps.json"), serde_json::to_vec(m).unwrap()).unwrap()
    };
    let mut strict = manifest.clone();
    if let Node::CffPhysicalEncodingAsset { declarations, .. } = &mut strict.nodes[0] {
        *declarations = None;
        write(&strict);
        assert!(
            ResolvedVfProject::load(&root, "deps.json", &registry, Default::default()).is_err()
        );
    }
    write(&manifest);
    let project =
        ResolvedVfProject::load(&root, "deps.json", &registry, Default::default()).unwrap();
    let fi = project
        .physical_run("lm", b"fi", registry.generation())
        .unwrap();
    assert!(matches!(
        fi.items.as_slice(),
        [MappedItem::Glyph {
            tfm_code: 28,
            identity: encoding::GlyphIdentity::Original(125),
            input_start: 0,
            input_end: 2,
            ..
        }]
    ));
    let av = project
        .physical_run("lm", b"AV", registry.generation())
        .unwrap();
    assert!(matches!(
        av.items.as_slice(),
        [
            MappedItem::Glyph {
                input_start: 0,
                input_end: 1,
                ..
            },
            MappedItem::Kern(tfm::FixWord(-116509)),
            MappedItem::Glyph {
                input_start: 1,
                input_end: 2,
                ..
            }
        ]
    ));
    for slot in [156, 188, 223] {
        assert!(project
            .physical_run("lm", &[slot], registry.generation())
            .is_err());
    }
    let cases = [
        b"fi".as_slice(),
        b"ff",
        b"fl",
        b"ffi",
        b"ffl",
        b"AV",
        b"To",
        b"WA",
    ];
    let mut runs = Vec::new();
    for input in cases {
        let run = project
            .physical_run("lm", input, registry.generation())
            .unwrap();
        let repeat = project
            .physical_run("lm", input, registry.generation())
            .unwrap();
        assert_eq!(run.items, repeat.items);
        let items: Vec<_> = run
            .items
            .iter()
            .map(|item| match item {
                MappedItem::Glyph {
                    tfm_code,
                    identity,
                    metrics,
                    input_start,
                    input_end,
                } => {
                    let gid = match identity {
                        encoding::GlyphIdentity::Original(gid) => *gid,
                        _ => panic!("no substitute glyph permitted in replay"),
                    };
                    serde_json::json!({"kind":"glyph", "tfm_code":tfm_code, "original_gid":gid,
                    "input_range":[input_start,input_end], "width_fixword":metrics.width.0,
                    "height_fixword":metrics.height.0,"depth_fixword":metrics.depth.0,
                    "italic_fixword":metrics.italic.0})
                }
                MappedItem::Kern(value) => serde_json::json!({"kind":"kern","fixword":value.0}),
            })
            .collect();
        runs.push(
            serde_json::json!({"input_bytes":input,"input_sha256":sha256(input),"items":items}),
        );
    }
    let failures: Vec<_> = [156, 188, 223]
        .into_iter()
        .map(|slot| {
            let error = project
                .physical_run("lm", &[slot], registry.generation())
                .err()
                .expect("unavailable slot must reject");
            serde_json::json!({"slot":slot,"outcome":"rejected","error":error.to_string()})
        })
        .collect();
    let evidence = serde_json::json!({
        "schema_version":1,"scope":"exact TFM metrics and explicit encoding mapping; no visual or PDF claim",
        "units":"signed TFM fix_word integers, denominator 1048576; input ranges are 8-bit code intervals",
        "font_sha256":sha256(&font_bytes),"face_index":0,
        "tfm_sha256":sha256(&tfm),"encoding_sha256":sha256(&enc),"license_sha256":sha256(&license),
        "peer_mapping_sha256":sha256(&std::fs::read(fixtures.join("ec-lmr10.encoding.json")).unwrap()),
        "registry_generation":registry.generation(),"project_generation":project.generation(),
        "dependency_manifest":manifest,"runs":runs,"unavailable":failures,
        "strict_undeclared_binding":"rejected","repeat_run":"identical","stale_registry":"rejected"
    });
    if let Ok(path) = std::env::var("FLASHTEX_LM_REPLAY_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    } else {
        let expected: serde_json::Value = serde_json::from_slice(
            &std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("fixtures/rooted-lm-replay.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(evidence, expected);
    }
    assert_eq!(av.project_generation, project.generation());
    assert!(project.physical_run("lm", b"fi", "stale").is_err());
}
