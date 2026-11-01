use flashtex_font_resources::*;
use std::collections::BTreeMap;

fn be16(data: &mut [u8], at: usize, n: u16) {
    data[at..at + 2].copy_from_slice(&n.to_be_bytes());
}
fn be32(data: &mut [u8], at: usize, n: u32) {
    data[at..at + 4].copy_from_slice(&n.to_be_bytes());
}
fn fixture() -> Vec<u8> {
    let mut tables = BTreeMap::new();
    let mut head = vec![0; 54];
    be32(&mut head, 0, 0x00010000);
    be32(&mut head, 12, 0x5f0f3cf5);
    be16(&mut head, 18, 1000);
    tables.insert(*b"head", head);
    let mut maxp = vec![0; 32];
    be32(&mut maxp, 0, 0x00010000);
    be16(&mut maxp, 4, 3);
    tables.insert(*b"maxp", maxp);
    tables.insert(*b"loca", vec![0; 8]);
    tables.insert(*b"glyf", vec![]);
    let mut hhea = vec![0; 36];
    be32(&mut hhea, 0, 0x00010000);
    be16(&mut hhea, 34, 2);
    tables.insert(*b"hhea", hhea);
    tables.insert(*b"hmtx", vec![0; 10]);
    let mut name = vec![0; 18];
    be16(&mut name, 2, 1);
    be16(&mut name, 4, 18);
    be16(&mut name, 6, 3);
    be16(&mut name, 8, 1);
    be16(&mut name, 10, 0x0409);
    be16(&mut name, 12, 6);
    be16(&mut name, 14, 12);
    for ch in "FTTest".encode_utf16() {
        name.extend_from_slice(&ch.to_be_bytes());
    }
    tables.insert(*b"name", name);
    let mut bytes = vec![0; 12 + tables.len() * 16];
    be32(&mut bytes, 0, 0x00010000);
    be16(&mut bytes, 4, tables.len() as u16);
    for (index, (tag, data)) in tables.into_iter().enumerate() {
        while !bytes.len().is_multiple_of(4) {
            bytes.push(0);
        }
        let start = bytes.len();
        let at = 12 + index * 16;
        bytes[at..at + 4].copy_from_slice(&tag);
        be32(&mut bytes, at + 8, start as u32);
        be32(&mut bytes, at + 12, data.len() as u32);
        bytes.extend_from_slice(&data);
    }
    bytes
}
fn entry(bytes: &[u8]) -> ManifestEntry {
    ManifestEntry {
        font: FontDescriptor {
            font_id: "test.font".into(),
            sha256: sha256(bytes),
            byte_length: bytes.len() as u64,
            format: "static-truetype".into(),
            face_index: 0,
            units_per_em: 1000,
            glyph_count: 3,
            postscript_name: "FTTest".into(),
        },
        path: "font.ttf".into(),
        license: LicenseMetadata {
            identifier: "LicenseRef-Synthetic-Test".into(),
            copyright: "Synthetic fixture only".into(),
            source: "locally generated validation fixture".into(),
            text_path: "LICENSE.txt".into(),
            text_sha256: sha256(b"test license"),
            embedding_permission: EmbeddingPermission::Unknown,
        },
    }
}
fn table_record(bytes: &[u8], tag: &[u8; 4]) -> usize {
    (0..u16::from_be_bytes([bytes[4], bytes[5]]) as usize)
        .map(|i| 12 + i * 16)
        .find(|i| &bytes[*i..*i + 4] == tag)
        .unwrap()
}
fn table_offset(bytes: &[u8], tag: &[u8; 4]) -> usize {
    let at = table_record(bytes, tag) + 8;
    u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}

#[test]
fn immutable_digest_bound_bytes_and_exact_descriptor() {
    let mut bytes = fixture();
    let spec = entry(&bytes);
    let font = FontResource::from_bytes(&spec, &bytes, b"test license").unwrap();
    let copied = font.shared_bytes();
    bytes[0] = 99;
    assert_eq!(font.bytes()[0], 0);
    assert_eq!(&*copied, font.bytes());
    assert_eq!(font.descriptor(), &spec.font);
    assert_eq!(font.table(b"head").unwrap().len(), 54);
    assert_eq!(font.license_text(), b"test license");
    assert_eq!(
        font.license().embedding_permission,
        EmbeddingPermission::Unknown
    );
}
#[test]
fn mismatched_digest_license_and_metadata_rejected() {
    let bytes = fixture();
    let mut spec = entry(&bytes);
    spec.font.sha256 = "0".repeat(64);
    assert_eq!(
        FontResource::from_bytes(&spec, &bytes, b"test license").unwrap_err(),
        Error::DigestMismatch
    );
    let spec = entry(&bytes);
    assert_eq!(
        FontResource::from_bytes(&spec, &bytes, b"changed").unwrap_err(),
        Error::LicenseDigestMismatch
    );
    for field in [
        "units_per_em",
        "glyph_count",
        "postscript_name",
        "byte_length",
    ] {
        let mut spec = entry(&bytes);
        match field {
            "units_per_em" => spec.font.units_per_em = 2048,
            "glyph_count" => spec.font.glyph_count = 4,
            "postscript_name" => spec.font.postscript_name = "Wrong".into(),
            _ => spec.font.byte_length += 1,
        }
        assert_eq!(
            FontResource::from_bytes(&spec, &bytes, b"test license").unwrap_err(),
            Error::MetadataMismatch(field)
        );
    }
}
#[test]
fn face_collection_and_variable_fonts_do_not_fallback() {
    let bytes = fixture();
    let mut spec = entry(&bytes);
    spec.font.face_index = 1;
    assert!(matches!(
        FontResource::from_bytes(&spec, &bytes, b"test license"),
        Err(Error::UnsupportedFont(_))
    ));
    for magic in [b"ttcf", b"OTTO", b"wOFF"] {
        let mut b = fixture();
        b[..4].copy_from_slice(magic);
        assert!(matches!(
            inspect_static_truetype(&b),
            Err(Error::UnsupportedFont(_))
        ));
    }
    let mut b = fixture();
    let at = table_record(&b, b"hmtx");
    b[at..at + 4].copy_from_slice(b"fvar");
    assert!(matches!(
        inspect_static_truetype(&b),
        Err(Error::UnsupportedFont(_))
    ));
}
#[test]
fn directory_bounds_duplicate_and_overlap_rejected() {
    let mut b = fixture();
    be32(&mut b, 20, u32::MAX);
    assert!(inspect_static_truetype(&b).is_err());
    let mut b = fixture();
    let tag = b[12..16].to_vec();
    b[28..32].copy_from_slice(&tag);
    assert!(inspect_static_truetype(&b).is_err());
    let mut b = fixture();
    let head = table_offset(&b, b"head");
    let at = table_record(&b, b"hhea");
    be32(&mut b, at + 8, head as u32);
    assert!(inspect_static_truetype(&b).is_err());
}
#[test]
fn loca_and_horizontal_metrics_bounds_rejected() {
    let mut b = fixture();
    let loca = table_offset(&b, b"loca");
    be16(&mut b, loca + 6, 1);
    assert!(inspect_static_truetype(&b).is_err());
    let mut b = fixture();
    let hhea = table_offset(&b, b"hhea");
    be16(&mut b, hhea + 34, 4);
    assert!(inspect_static_truetype(&b).is_err());
    let mut b = fixture();
    let record = table_record(&b, b"hmtx");
    be32(&mut b, record + 12, 1);
    assert!(inspect_static_truetype(&b).is_err());
}
#[test]
fn units_glyph_count_name_bounds_checked() {
    let mut b = fixture();
    let head = table_offset(&b, b"head");
    be16(&mut b, head + 18, 0);
    assert!(inspect_static_truetype(&b).is_err());
    let mut b = fixture();
    let maxp = table_offset(&b, b"maxp");
    be16(&mut b, maxp + 4, 1);
    assert!(inspect_static_truetype(&b).is_err());
    let mut b = fixture();
    let name = table_offset(&b, b"name");
    be16(&mut b, name + 16, 65000);
    assert!(inspect_static_truetype(&b).is_err());
}
#[test]
fn collection_is_sorted_and_lookup_missing_is_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = fixture();
    std::fs::write(dir.path().join("font.ttf"), &bytes).unwrap();
    std::fs::write(dir.path().join("LICENSE.txt"), b"test license").unwrap();
    let mut a = entry(&bytes);
    a.font.font_id = "a".into();
    let mut z = a.clone();
    z.font.font_id = "z".into();
    let fonts = FontCollection::load(
        dir.path(),
        &Manifest {
            schema_version: 1,
            resources: vec![z, a],
        },
    )
    .unwrap();
    assert_eq!(fonts.ids().collect::<Vec<_>>(), vec!["a", "z"]);
    assert!(matches!(
        fonts.get("fallback"),
        Err(Error::MissingResource(_))
    ));
    std::fs::write(dir.path().join("font.ttf"), b"changed on disk").unwrap();
    assert_eq!(fonts.get("a").unwrap().bytes(), bytes);
}
#[test]
fn missing_files_traversal_and_duplicate_ids_are_errors() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = fixture();
    let spec = entry(&bytes);
    assert!(matches!(
        FontCollection::load(
            dir.path(),
            &Manifest {
                schema_version: 1,
                resources: vec![spec.clone()]
            }
        ),
        Err(Error::MissingResource(_))
    ));
    for path in [
        "../outside.ttf",
        "/absolute.ttf",
        "a//b.ttf",
        "./font.ttf",
        "a\\b.ttf",
    ] {
        let mut s = spec.clone();
        s.path = path.into();
        assert!(matches!(
            FontCollection::load(
                dir.path(),
                &Manifest {
                    schema_version: 1,
                    resources: vec![s]
                }
            ),
            Err(Error::InvalidManifest(_))
        ));
    }
    assert!(matches!(
        FontCollection::load(
            dir.path(),
            &Manifest {
                schema_version: 1,
                resources: vec![spec.clone(), spec]
            }
        ),
        Err(Error::InvalidManifest(_))
    ));
}
#[test]
fn truncated_inputs_never_panic() {
    let bytes = fixture();
    for n in 0..bytes.len() {
        assert!(std::panic::catch_unwind(|| inspect_static_truetype(&bytes[..n])).is_ok());
    }
}
#[test]
fn wire_descriptor_shape_matches_rendering_v2() {
    let value = serde_json::to_value(entry(&fixture()).font).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 8);
    assert_eq!(value["format"], "static-truetype");
    assert_eq!(value["face_index"], 0);
    let mut bad = value;
    bad["platform_fallback"] = serde_json::json!(true);
    assert!(serde_json::from_value::<FontDescriptor>(bad).is_err());
}

#[test]
fn missing_license_is_distinguished_from_missing_font() {
    let root = tempfile::tempdir().unwrap();
    let bytes = fixture();
    std::fs::write(root.path().join("font.ttf"), &bytes).unwrap();
    let manifest = Manifest {
        schema_version: 1,
        resources: vec![entry(&bytes)],
    };
    assert!(matches!(
        FontCollection::load(root.path(), &manifest),
        Err(Error::MissingLicense(_))
    ));
}

#[cfg(unix)]
#[test]
fn symlink_outside_resource_root_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let bytes = fixture();
    std::fs::write(outside.path().join("font.ttf"), &bytes).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("font.ttf"),
        root.path().join("font.ttf"),
    )
    .unwrap();
    let manifest = Manifest {
        schema_version: 1,
        resources: vec![entry(&bytes)],
    };
    assert!(FontCollection::load(root.path(), &manifest).is_err());
}

#[test]
fn horizontal_metrics_keep_original_units_and_trailing_bearings() {
    let mut bytes = fixture();
    let at = table_record(&bytes, b"hmtx");
    let start = u32::from_be_bytes(bytes[at + 8..at + 12].try_into().unwrap()) as usize;
    be16(&mut bytes, start, 500);
    be16(&mut bytes, start + 4, 700);
    be16(&mut bytes, start + 8, (-30i16) as u16);
    let resource = FontResource::from_bytes(&entry(&bytes), &bytes, b"test license").unwrap();
    assert_eq!(
        resource.horizontal_metrics(2).unwrap(),
        HorizontalMetrics {
            advance_width: 700,
            left_side_bearing: -30
        }
    );
    assert!(resource.horizontal_metrics(3).is_err());
    assert!(matches!(
        resource.glyph_id('A'),
        Err(Error::UnsupportedFont(_))
    ));
}

fn tfm_for_encoding() -> flashtex_font_resources::tfm::Tfm {
    flashtex_font_resources::tfm::Tfm::parse(&tfm_bytes_for_encoding()).unwrap()
}
fn tfm_bytes_for_encoding() -> Vec<u8> {
    let mut bytes = [16u16, 2, 65, 66, 2, 1, 1, 1, 0, 0, 0, 1]
        .into_iter()
        .flat_map(u16::to_be_bytes)
        .collect::<Vec<_>>();
    for word in [
        0u32,
        10 << 20,
        0x01000000,
        0x01000000,
        0,
        1 << 19,
        0,
        0,
        0,
        0,
    ] {
        bytes.extend(word.to_be_bytes());
    }
    bytes
}
fn encoding_manifest(
    font: &FontResource,
    tfm: &flashtex_font_resources::tfm::Tfm,
) -> flashtex_font_resources::encoding::EncodingManifest {
    use flashtex_font_resources::encoding::*;
    EncodingManifest {
        font_sha256: font.descriptor().sha256.clone(),
        tfm_sha256: tfm.source_sha256.clone(),
        face_index: 0,
        encoding: vec![
            EncodingEntry {
                code: 65,
                glyph_name: "A.alt".into(),
            },
            EncodingEntry {
                code: 66,
                glyph_name: ".notdef".into(),
            },
        ],
        declared_glyphs: vec![NamedGlyph {
            glyph_name: "A.alt".into(),
            glyph_id: 2,
        }],
    }
}
#[test]
fn explicit_encoding_never_casts_codes_and_adapter_preserves_metrics() {
    use flashtex_font_resources::encoding::*;
    let bytes = fixture();
    let font = FontResource::from_bytes(&entry(&bytes), &bytes, b"test license").unwrap();
    let tfm = tfm_for_encoding();
    let manifest = encoding_manifest(&font, &tfm);
    let bound = BoundTfmFont::new(&tfm, &font, &manifest).unwrap();
    assert_eq!(bound.map_code(65).unwrap().0, GlyphIdentity::Original(2));
    assert_eq!(bound.map_code(66).unwrap().0, GlyphIdentity::Notdef);
    assert!(bound.map_code(67).is_err());
    assert!(matches!(
        &bound.map_run(b"AB").unwrap()[0],
        MappedItem::Glyph {
            identity: GlyphIdentity::Original(2),
            input_start: 0,
            input_end: 1,
            ..
        }
    ));
}
#[test]
fn encoding_duplicate_missing_and_hash_binding_fail() {
    use flashtex_font_resources::encoding::*;
    let bytes = fixture();
    let font = FontResource::from_bytes(&entry(&bytes), &bytes, b"test license").unwrap();
    let tfm = tfm_for_encoding();
    let original = encoding_manifest(&font, &tfm);
    for mode in 0..7 {
        let mut m = original.clone();
        match mode {
            0 => m.encoding.push(m.encoding[0].clone()),
            1 => m.declared_glyphs.push(m.declared_glyphs[0].clone()),
            2 => m.declared_glyphs.clear(),
            3 => m.font_sha256 = "0".repeat(64),
            4 => m.tfm_sha256 = "0".repeat(64),
            5 => m.face_index = 1,
            _ => m.declared_glyphs[0].glyph_id = 0,
        };
        assert!(EncodingMap::bind(&m, &font, &tfm).is_err(), "{mode}");
    }
}
#[test]
fn typed_json_escaped_names_decode_exactly_without_postscript_guessing() {
    use flashtex_font_resources::encoding::*;
    let bytes = fixture();
    let font = FontResource::from_bytes(&entry(&bytes), &bytes, b"test license").unwrap();
    let tfm = tfm_for_encoding();
    let mut manifest = encoding_manifest(&font, &tfm);
    manifest.encoding[0] =
        serde_json::from_str(r#"{"code":65,"glyph_name":"A\u002ealt"}"#).unwrap();
    assert_eq!(
        EncodingMap::bind(&manifest, &font, &tfm)
            .unwrap()
            .resolve(65)
            .unwrap(),
        GlyphIdentity::Original(2)
    );
    manifest.encoding[0].glyph_name = "A#2Ealt".into();
    assert!(EncodingMap::bind(&manifest, &font, &tfm).is_err());
    manifest.declared_glyphs.push(NamedGlyph {
        glyph_name: ".notdef".into(),
        glyph_id: 1,
    });
    assert!(EncodingMap::bind(&manifest, &font, &tfm).is_err());
}

fn virtual_font(commands: &[u8]) -> flashtex_font_resources::vf::VirtualFont {
    let mut b = vec![247, 202, 0];
    for n in [0u32, 10 << 20] {
        b.extend(n.to_be_bytes());
    }
    b.extend([243, 0]);
    for n in [0u32, 1 << 20, 10 << 20] {
        b.extend(n.to_be_bytes());
    }
    b.extend([0, 1, b'f', 242]);
    for n in [commands.len() as u32, 65, 1 << 19] {
        b.extend(n.to_be_bytes());
    }
    b.extend(commands);
    b.push(248);
    flashtex_font_resources::vf::VirtualFont::parse(&b).unwrap()
}
#[test]
fn virtual_packet_expands_exact_bound_glyphs_and_restores_position() {
    use flashtex_font_resources::{encoding::*, vf::Placement};
    let bytes = fixture();
    let font = FontResource::from_bytes(&entry(&bytes), &bytes, b"test license").unwrap();
    let tfm = tfm_for_encoding();
    let manifest = encoding_manifest(&font, &tfm);
    let binding = BoundTfmFont::new(&tfm, &font, &manifest).unwrap();
    let resources = BTreeMap::from([(0, binding)]);
    let vf = virtual_font(&[65, 141, 146, 0, 8, 0, 0, 65, 142, 65]);
    let out = vf.expand_packet(65, &tfm, &resources).unwrap();
    let xs = out
        .placements
        .iter()
        .map(|p| match p {
            Placement::Glyph { x, glyph_id, .. } => {
                assert_eq!(*glyph_id, 2);
                (x.numerator(), x.shift())
            }
            _ => panic!("unexpected rule"),
        })
        .collect::<Vec<_>>();
    assert_eq!(xs, vec![(0, 0), (1, 0), (1, 1)]);
}
#[test]
fn virtual_packet_missing_resources_specials_notdef_and_width_fail() {
    use flashtex_font_resources::encoding::*;
    let bytes = fixture();
    let font = FontResource::from_bytes(&entry(&bytes), &bytes, b"test license").unwrap();
    let tfm = tfm_for_encoding();
    let manifest = encoding_manifest(&font, &tfm);
    let resources = BTreeMap::from([(0, BoundTfmFont::new(&tfm, &font, &manifest).unwrap())]);
    assert!(virtual_font(&[65])
        .expand_packet(65, &tfm, &BTreeMap::new())
        .is_err());
    assert!(virtual_font(&[66])
        .expand_packet(65, &tfm, &resources)
        .is_err());
    assert!(matches!(
        virtual_font(&[239, 1, 0]).expand_packet(65, &tfm, &resources),
        Err(Error::UnsupportedFont(_))
    ));
    let mut mismatch = tfm.clone();
    mismatch.design_size = flashtex_font_resources::tfm::FixWord(11 << 20);
    assert!(virtual_font(&[65])
        .expand_packet(65, &mismatch, &resources)
        .is_err());
}

#[test]
fn virtual_packet_put_rule_and_reused_register_have_exact_units() {
    use flashtex_font_resources::{encoding::*, vf::Placement};
    let bytes = fixture();
    let font = FontResource::from_bytes(&entry(&bytes), &bytes, b"test license").unwrap();
    let tfm = tfm_for_encoding();
    let manifest = encoding_manifest(&font, &tfm);
    let resources = BTreeMap::from([(0, BoundTfmFont::new(&tfm, &font, &manifest).unwrap())]);
    let out = virtual_font(&[
        133, 65, 151, 0, 8, 0, 0, 65, 147, 132, 0, 4, 0, 0, 0, 8, 0, 0, 65,
    ])
    .expand_packet(65, &tfm, &resources)
    .unwrap();
    assert_eq!(out.placements.len(), 4);
    match &out.placements[2] {
        Placement::Rule {
            x, width, height, ..
        } => {
            assert_eq!((x.numerator(), x.shift()), (3, 1));
            assert_eq!((width.numerator(), width.shift()), (1, 1));
            assert_eq!((height.numerator(), height.shift()), (1, 2));
        }
        _ => panic!("expected rule"),
    }
    match &out.placements[3] {
        Placement::Glyph { x, .. } => assert_eq!((x.numerator(), x.shift()), (2, 0)),
        _ => panic!("expected glyph"),
    }
}

fn graph_vf(commands: &[u8], scale: u32, comment: u8) -> flashtex_font_resources::vf::VirtualFont {
    flashtex_font_resources::vf::VirtualFont::parse(&graph_vf_bytes(commands, scale, comment))
        .unwrap()
}
fn graph_vf_bytes(commands: &[u8], scale: u32, comment: u8) -> Vec<u8> {
    let mut b = vec![247, 202, 1, comment];
    for n in [0u32, 10 << 20] {
        b.extend(n.to_be_bytes());
    }
    b.extend([243, 0]);
    for n in [0u32, scale, 10 << 20] {
        b.extend(n.to_be_bytes());
    }
    b.extend([0, 1, b'f', 242]);
    for n in [commands.len() as u32, 65, 1 << 19] {
        b.extend(n.to_be_bytes());
    }
    b.extend(commands);
    b.push(248);
    b
}
#[test]
fn nested_and_flat_vf_geometry_are_exactly_equivalent_with_distinct_provenance() {
    use flashtex_font_resources::{encoding::*, vf_graph::*};
    let bytes = fixture();
    let font = FontResource::from_bytes(&entry(&bytes), &bytes, b"test license").unwrap();
    let tfm = tfm_for_encoding();
    let manifest = encoding_manifest(&font, &tfm);
    let binding = BoundTfmFont::new(&tfm, &font, &manifest).unwrap();
    let child = graph_vf(
        &[146, 0, 4, 0, 0, 65, 137, 0, 8, 0, 0, 0, 4, 0, 0],
        1 << 19,
        b'c',
    );
    let root = graph_vf(&[146, 0, 8, 0, 0, 65], 1 << 19, b'r');
    let flat = graph_vf(
        &[146, 0, 10, 0, 0, 65, 137, 0, 4, 0, 0, 0, 2, 0, 0],
        1 << 18,
        b'f',
    );
    let mut graph = ResourceGraph::new();
    let physical = graph.insert(Resource::Physical(&binding)).unwrap();
    let child_key = graph
        .insert(Resource::Virtual {
            vf: &child,
            tfm: &tfm,
            fonts: BTreeMap::from([(0, physical.clone())]),
        })
        .unwrap();
    let root_key = graph
        .insert(Resource::Virtual {
            vf: &root,
            tfm: &tfm,
            fonts: BTreeMap::from([(0, child_key)]),
        })
        .unwrap();
    let flat_key = graph
        .insert(Resource::Virtual {
            vf: &flat,
            tfm: &tfm,
            fonts: BTreeMap::from([(0, physical)]),
        })
        .unwrap();
    let nested = graph.expand(&root_key, 65).unwrap();
    let direct = graph.expand(&flat_key, 65).unwrap();
    let geometry = |p: &NestedPlacement| match p {
        NestedPlacement::Glyph {
            resource,
            glyph_id,
            x,
            y,
            scale,
            ..
        } => (resource.clone(), *glyph_id, *x, *y, *scale),
        _ => panic!("glyph expected"),
    };
    assert_eq!(
        geometry(&nested.placements[0]),
        geometry(&direct.placements[0])
    );
    let rule_geometry = |p: &NestedPlacement| match p {
        NestedPlacement::Rule {
            x,
            y,
            width,
            height,
            ..
        } => (*x, *y, *width, *height),
        _ => panic!("expected rule"),
    };
    assert_eq!(
        rule_geometry(&nested.placements[1]),
        rule_geometry(&direct.placements[1])
    );
    match &nested.placements[0] {
        NestedPlacement::Glyph {
            x, scale, source, ..
        } => {
            assert_eq!((x.numerator(), x.shift()), (5, 3));
            assert_eq!((scale.numerator(), scale.shift()), (1, 2));
            assert_eq!(source.len(), 3);
            assert_eq!(source[0].command_index, Some(1));
        }
        _ => unreachable!(),
    }
}
#[test]
fn nested_vf_cycles_missing_keys_and_specials_fail() {
    use flashtex_font_resources::{encoding::*, vf_graph::*};
    let bytes = fixture();
    let font = FontResource::from_bytes(&entry(&bytes), &bytes, b"test license").unwrap();
    let tfm = tfm_for_encoding();
    let manifest = encoding_manifest(&font, &tfm);
    let binding = BoundTfmFont::new(&tfm, &font, &manifest).unwrap();
    let root = graph_vf(&[65], 1 << 20, b'r');
    let key = ResourceKey::Virtual {
        vf_sha256: root.source_sha256.clone(),
        tfm_sha256: tfm.source_sha256.clone(),
    };
    let mut graph = ResourceGraph::new();
    graph
        .insert(Resource::Virtual {
            vf: &root,
            tfm: &tfm,
            fonts: BTreeMap::from([(0, key.clone())]),
        })
        .unwrap();
    assert!(graph.expand(&key, 65).is_err());
    let mut graph = ResourceGraph::new();
    assert!(graph.expand(&key, 65).is_err());
    let physical = graph.insert(Resource::Physical(&binding)).unwrap();
    assert!(graph.insert(Resource::Physical(&binding)).is_err());
    let special = graph_vf(&[239, 1, 0], 1 << 20, b's');
    let key = graph
        .insert(Resource::Virtual {
            vf: &special,
            tfm: &tfm,
            fonts: BTreeMap::from([(0, physical)]),
        })
        .unwrap();
    assert!(matches!(
        graph.expand(&key, 65),
        Err(Error::UnsupportedFont(_))
    ));
}
#[test]
fn nested_vf_depth_and_node_caps_fail_without_partial_output() {
    use flashtex_font_resources::{encoding::*, vf_graph::*};
    let bytes = fixture();
    let font = FontResource::from_bytes(&entry(&bytes), &bytes, b"test license").unwrap();
    let tfm = tfm_for_encoding();
    let manifest = encoding_manifest(&font, &tfm);
    let binding = BoundTfmFont::new(&tfm, &font, &manifest).unwrap();
    for branch in [false, true] {
        let fonts = (0..34)
            .map(|i| graph_vf(if branch { &[65, 65] } else { &[65] }, 1 << 20, i))
            .collect::<Vec<_>>();
        let mut graph = ResourceGraph::new();
        let mut key = graph.insert(Resource::Physical(&binding)).unwrap();
        for vf in &fonts {
            key = graph
                .insert(Resource::Virtual {
                    vf,
                    tfm: &tfm,
                    fonts: BTreeMap::from([(0, key)]),
                })
                .unwrap();
            if branch && vf.comment[0] == 12 {
                break;
            }
        }
        assert!(graph.expand(&key, 65).is_err());
    }
}

#[test]
fn nested_vf_output_cap_is_global_across_packets() {
    use flashtex_font_resources::{encoding::*, vf_graph::*};
    let bytes = fixture();
    let font = FontResource::from_bytes(&entry(&bytes), &bytes, b"test license").unwrap();
    let tfm = tfm_for_encoding();
    let manifest = encoding_manifest(&font, &tfm);
    let binding = BoundTfmFont::new(&tfm, &font, &manifest).unwrap();
    let rule = [137, 0, 0, 0, 1, 0, 0, 0, 1];
    let commands = rule.repeat(60000);
    let child = graph_vf(&commands, 1 << 20, b'c');
    let root = graph_vf(&[65, 65], 1 << 20, b'r');
    let mut graph = ResourceGraph::new();
    let physical = graph.insert(Resource::Physical(&binding)).unwrap();
    let child = graph
        .insert(Resource::Virtual {
            vf: &child,
            tfm: &tfm,
            fonts: BTreeMap::from([(0, physical)]),
        })
        .unwrap();
    let root = graph
        .insert(Resource::Virtual {
            vf: &root,
            tfm: &tfm,
            fonts: BTreeMap::from([(0, child)]),
        })
        .unwrap();
    assert!(graph
        .expand(&root, 65)
        .unwrap_err()
        .to_string()
        .contains("output budget"));
}

#[test]
fn cff_encoding_binds_actual_names_and_resource_hashes() {
    use flashtex_font_resources::cff::*;
    // Header/name/top/string/global indexes plus a two-entry CharStrings INDEX;
    // ISOAdobe SID1 is space, not a Unicode or TFM-code cast.
    let mut bytes = b"OTTOxxxx".to_vec();
    bytes.extend([
        1, 0, 4, 4, 0, 1, 1, 1, 2, b'F', 0, 1, 1, 1, 3, 160, 17, 0, 0, 0, 0, 0, 2, 1, 1, 2, 3, 14,
        14,
    ]);
    let cache = CffOutlineCache::from_font_table(
        &bytes,
        0,
        8..bytes.len(),
        CacheLimits {
            max_entries: 2,
            max_bytes: 10000,
        },
    )
    .unwrap();
    let tfm = tfm_for_encoding();
    let mut enc_names = vec!["/.notdef"; 256];
    enc_names[65] = "/space";
    let enc_source = format!("/Literal[{}]def", enc_names.join(" "));
    let enc =
        enc_file::EncFile::parse(enc_source.as_bytes(), &sha256(enc_source.as_bytes())).unwrap();
    let encoded = enc.bind_cff(&tfm, &cache).unwrap();
    assert_eq!(
        encoded.font().map_code(65).unwrap().0,
        encoding::GlyphIdentity::Original(1)
    );
    assert_eq!(
        encoded.font().map_code(66).unwrap().0,
        encoding::GlyphIdentity::Notdef
    );
    assert_eq!(encoded.source().file_sha256, sha256(enc_source.as_bytes()));
    let mut aliases_names = vec!["/.notdef"; 256];
    aliases_names[65] = "/alias";
    aliases_names[66] = "/absent";
    let alias_source = format!("/Declared[{}]def", aliases_names.join(" "));
    let enc = enc_file::EncFile::parse(alias_source.as_bytes(), &sha256(alias_source.as_bytes()))
        .unwrap();
    assert!(enc.bind_cff(&tfm, &cache).is_err());
    let declarations = enc_file::CffMappingDeclarations {
        encoding_file_sha256: sha256(alias_source.as_bytes()),
        font_sha256: cache.identity().font_sha256.clone(),
        cff_sha256: cache.identity().cff_sha256.clone(),
        face_index: 0,
        aliases: vec![enc_file::GlyphNameAlias {
            literal_name: "alias".into(),
            font_name: "space".into(),
            original_gid: 1,
        }],
        unavailable_slots: vec![enc_file::UnavailableSlot {
            code: 66,
            literal_name: "absent".into(),
        }],
    };
    let bound_alias = enc.bind_cff_declared(&tfm, &cache, &declarations).unwrap();
    assert_eq!(
        bound_alias.font().map_code(65).unwrap().0,
        encoding::GlyphIdentity::Original(1)
    );
    assert_eq!(bound_alias.font().encoding().glyph_name(65), Some("alias"));
    assert!(bound_alias.font().map_code(66).is_err());
    assert!(bound_alias.declarations().is_some());
    for fault in 0..6 {
        let mut bad = declarations.clone();
        match fault {
            0 => bad.font_sha256 = "0".repeat(64),
            1 => bad.aliases[0].original_gid = 0,
            2 => bad.aliases.push(bad.aliases[0].clone()),
            3 => bad.unavailable_slots.push(bad.unavailable_slots[0].clone()),
            4 => bad.unavailable_slots[0].code = 65,
            _ => bad.encoding_file_sha256 = "0".repeat(64),
        }
        assert!(enc.bind_cff_declared(&tfm, &cache, &bad).is_err());
    }
    let identity = cache.identity();
    let manifest = CffEncodingManifest {
        font_sha256: identity.font_sha256.clone(),
        cff_sha256: identity.cff_sha256.clone(),
        tfm_sha256: tfm.source_sha256.clone(),
        face_index: 0,
        encoding: vec![flashtex_font_resources::encoding::EncodingEntry {
            code: 65,
            glyph_name: "space".into(),
        }],
    };
    let bound = BoundCffTfmFont::new(&tfm, &cache, &manifest).unwrap();
    assert_eq!(
        bound.map_code(65).unwrap().0,
        flashtex_font_resources::encoding::GlyphIdentity::Original(1)
    );
    assert!(bound.map_code(66).is_err());
    assert!(bound.validate_cache(&cache).is_ok());
    let mut bad = manifest.clone();
    bad.encoding[0].glyph_name = "A".into();
    assert!(BoundCffTfmFont::new(&tfm, &cache, &bad).is_err());
    bad = manifest.clone();
    bad.cff_sha256 = "0".repeat(64);
    assert!(BoundCffTfmFont::new(&tfm, &cache, &bad).is_err());
    bad = manifest.clone();
    bad.encoding.push(bad.encoding[0].clone());
    assert!(BoundCffTfmFont::new(&tfm, &cache, &bad).is_err());
}

#[test]
fn cff_encoding_cache_identity_canonicalization_eviction_and_bounds() {
    use flashtex_font_resources::{cff::*, encoding::EncodingEntry};
    use std::sync::Arc;
    let mut bytes = b"OTTOxxxx".to_vec();
    bytes.extend([
        1, 0, 4, 4, 0, 1, 1, 1, 2, b'F', 0, 1, 1, 1, 3, 160, 17, 0, 0, 0, 0, 0, 2, 1, 1, 2, 3, 14,
        14,
    ]);
    let outline = CffOutlineCache::from_font_table(
        &bytes,
        0,
        8..bytes.len(),
        CacheLimits {
            max_entries: 0,
            max_bytes: 0,
        },
    )
    .unwrap();
    let tfm = tfm_for_encoding();
    let manifest = CffEncodingManifest {
        font_sha256: outline.identity().font_sha256.clone(),
        cff_sha256: outline.identity().cff_sha256.clone(),
        face_index: 0,
        tfm_sha256: tfm.source_sha256.clone(),
        encoding: vec![
            EncodingEntry {
                code: 65,
                glyph_name: "space".into(),
            },
            EncodingEntry {
                code: 66,
                glyph_name: ".notdef".into(),
            },
        ],
    };
    let mut cache = CffEncodingCache::new(
        &outline,
        CacheLimits {
            max_entries: 2,
            max_bytes: 10000,
        },
    )
    .unwrap();
    let first = cache.lookup(&tfm, &manifest).unwrap();
    assert_eq!(first.status, CacheStatus::Stored);
    let mut reordered = manifest.clone();
    reordered.encoding.reverse();
    let hit = cache.lookup(&tfm, &reordered).unwrap();
    assert_eq!(hit.status, CacheStatus::Hit);
    assert!(Arc::ptr_eq(&first.encoding, &hit.encoding));
    BoundCffTfmFont::from_resolved(&tfm, hit.encoding)
        .unwrap()
        .validate_cache(&outline)
        .unwrap();
    for field in 0..4 {
        let mut wrong = manifest.clone();
        match field {
            0 => wrong.font_sha256 = "0".repeat(64),
            1 => wrong.cff_sha256 = "0".repeat(64),
            2 => wrong.tfm_sha256 = "0".repeat(64),
            _ => wrong.face_index = 1,
        }
        assert!(cache.lookup(&tfm, &wrong).is_err());
    }
    let mut bad = manifest.clone();
    bad.encoding.push(bad.encoding[0].clone());
    assert!(cache.lookup(&tfm, &bad).is_err());
    bad = manifest.clone();
    bad.encoding[0].glyph_name = "absent".into();
    assert!(cache.lookup(&tfm, &bad).is_err());
    assert_eq!(cache.len(), 1);
    let mut second = manifest.clone();
    second.encoding[0].glyph_name = ".notdef".into();
    let other = cache.lookup(&tfm, &second).unwrap();
    assert_ne!(
        first.encoding.encoding_sha256(),
        other.encoding.encoding_sha256()
    );
    assert_eq!(
        other.encoding.resolve(65).unwrap(),
        flashtex_font_resources::encoding::GlyphIdentity::Notdef
    );
    assert_eq!(
        cache.lookup(&tfm, &manifest).unwrap().status,
        CacheStatus::Hit
    );
    let mut third = manifest.clone();
    third.encoding.pop();
    cache.lookup(&tfm, &third).unwrap(); // second is least recently used
    assert_eq!(
        cache.lookup(&tfm, &manifest).unwrap().status,
        CacheStatus::Hit
    );
    assert_eq!(
        cache.lookup(&tfm, &second).unwrap().status,
        CacheStatus::Stored
    );
    assert_eq!(cache.len(), 2);
    assert!(cache.retained_bytes() <= 10000);
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.retained_bytes(), 0);
    for limits in [
        CacheLimits {
            max_entries: 0,
            max_bytes: 10000,
        },
        CacheLimits {
            max_entries: 2,
            max_bytes: 1,
        },
    ] {
        let mut tiny = CffEncodingCache::new(&outline, limits).unwrap();
        assert_eq!(
            tiny.lookup(&tfm, &manifest).unwrap().status,
            CacheStatus::BypassedOversize
        );
        assert!(tiny.is_empty());
        assert_eq!(tiny.retained_bytes(), 0);
    }
    assert!(CffEncodingCache::new(
        &outline,
        CacheLimits {
            max_entries: 129,
            max_bytes: 1
        }
    )
    .is_err());
}

#[test]
fn original_engine_adapter_resource_identity_and_request_gates() {
    use flashtex_font_engine::ShapeOptions;
    use flashtex_font_resources::engine_adapter::*;
    let bytes = fixture();
    let resource = FontResource::from_bytes(&entry(&bytes), &bytes, b"test license").unwrap();
    let adapter = EngineFontAdapter::from_resource(&resource).unwrap();
    assert_eq!(adapter.identity().font_sha256, sha256(&bytes));
    let mut engine_bytes = bytes.clone();
    engine_bytes.extend(0u32.to_be_bytes());
    assert_eq!(
        adapter.identity().engine_font_id.content_hex(),
        sha256(&engine_bytes)
    );
    let source = "";
    let hash = sha256(source.as_bytes());
    let request = || ShapeRequest {
        source,
        source_sha256: &hash,
        path: "main.tex",
        revision: 1,
        range: 0..0,
        font_sha256: &resource.descriptor().sha256,
        face_index: 0,
        encoding: InputEncoding::Unicode,
        variation_coordinates: &[],
        options: ShapeOptions::PLAIN,
    };
    let empty = adapter.shape(request()).unwrap();
    assert!(empty.shaped().clusters.is_empty());
    let mut wrong = request();
    wrong.face_index = 1;
    assert!(adapter.shape(wrong).is_err());
    let mut wrong = request();
    wrong.encoding = InputEncoding::TexEightBit;
    assert!(adapter.shape(wrong).is_err());
    let mut wrong = request();
    let variations = [(*b"wght", 400)];
    wrong.variation_coordinates = &variations;
    assert!(adapter.shape(wrong).is_err());
    let mut wrong = request();
    wrong.range = 1..2;
    assert!(adapter.shape(wrong).is_err());
    let mut wrong = request();
    wrong.source_sha256 = "bad";
    assert!(adapter.shape(wrong).is_err());
    let mut other = request();
    other.revision = 2;
    assert_ne!(adapter.shape(other).unwrap().cache_key(), empty.cache_key());
}

#[test]
fn project_registry_snapshots_generation_and_budgets() {
    use flashtex_font_resources::registry::*;
    use flashtex_project_files::ProjectRoot;
    let dir = tempfile::tempdir().unwrap();
    let root = ProjectRoot::open(dir.path()).unwrap();
    let bytes = fixture();
    let mut resource = entry(&bytes);
    resource.path = "font.ttf".into();
    resource.license.text_path = "LICENSE".into();
    std::fs::write(dir.path().join("font.ttf"), &bytes).unwrap();
    std::fs::write(dir.path().join("LICENSE"), b"test license").unwrap();
    let binding = StyleBinding {
        family: "Demo".into(),
        weight: 400,
        style: FontStyle::Upright,
    };
    let mut manifest = RegistryManifest {
        schema_version: 1,
        entries: vec![RegistryEntry {
            binding: binding.clone(),
            resource,
        }],
    };
    let write = |m: &RegistryManifest| {
        std::fs::write(
            dir.path().join("fonts.json"),
            serde_json::to_vec(m).unwrap(),
        )
        .unwrap()
    };
    write(&manifest);
    let first = ProjectFontRegistry::load(&root, "fonts.json", RegistryLimits::default()).unwrap();
    let held = first.get(&binding).unwrap();
    assert_eq!(held.bytes(), bytes);
    assert_eq!(first.files_read(), 3);
    assert!(std::sync::Arc::ptr_eq(&held, &first.get(&binding).unwrap()));
    assert_eq!(first.discovery()[0].binding, binding);
    let load = || ProjectFontRegistry::load(&root, "fonts.json", RegistryLimits::default());
    assert_eq!(load().unwrap().generation(), first.generation());
    std::fs::write(
        dir.path().join("fonts.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    assert_eq!(load().unwrap().generation(), first.generation());
    let mut absent = binding.clone();
    absent.weight = 700;
    assert!(matches!(
        first.get(&absent),
        Err(RegistryError::MissingBinding(_))
    ));
    let mut alias = manifest.entries[0].clone();
    alias.binding.weight = 600;
    manifest.entries.push(alias);
    write(&manifest);
    let two = load().unwrap();
    let page = two
        .enumerate(two.generation(), MetadataFilter::default(), 0, 1)
        .unwrap();
    assert_eq!(page.total_matches, 2);
    assert_eq!(page.next_offset, Some(1));
    let last = two
        .enumerate(two.generation(), MetadataFilter::default(), 1, 1)
        .unwrap();
    assert_eq!(last.next_offset, None);
    let filtered = two
        .enumerate(
            two.generation(),
            MetadataFilter {
                weight: Some(600),
                ..Default::default()
            },
            0,
            64,
        )
        .unwrap();
    assert_eq!(filtered.entries.len(), 1);
    assert!(two
        .enumerate(first.generation(), MetadataFilter::default(), 0, 1)
        .is_err());
    assert!(two
        .enumerate(two.generation(), MetadataFilter::default(), 0, 65)
        .is_err());
    assert!(two
        .enumerate(two.generation(), MetadataFilter::default(), 129, 1)
        .is_err());
    let exported = two.export_json(1024 * 1024).unwrap();
    std::fs::write(dir.path().join("fonts.json"), &exported).unwrap();
    assert_eq!(load().unwrap().export_json(1024 * 1024).unwrap(), exported);
    assert!(two.export_json(1).is_err());
    for malformed in [
        String::from_utf8(exported.clone()).unwrap().replacen(
            "\"schema_version\":2",
            "\"schema_version\":2,\"schema_version\":2",
            1,
        ),
        String::from_utf8(exported.clone()).unwrap().replacen(
            "\"weight\":400",
            "\"weight\":400,\"weight\":400",
            1,
        ),
        String::from_utf8(exported.clone())
            .unwrap()
            .replacen("{", "{\"unknown\":true,", 1),
    ] {
        std::fs::write(dir.path().join("fonts.json"), malformed).unwrap();
        assert!(load().is_err());
    }
    manifest.entries.reverse();
    write(&manifest);
    assert_eq!(load().unwrap().generation(), two.generation());
    manifest.entries.retain(|e| e.binding.weight == 400);
    manifest.entries[0].binding.weight = 500;
    write(&manifest);
    let next = load().unwrap();
    assert_ne!(first.generation(), next.generation());
    assert!(next.require_generation(first.generation()).is_err());
    manifest.entries.push(manifest.entries[0].clone());
    write(&manifest);
    assert!(matches!(load(), Err(RegistryError::AmbiguousBinding(_))));
    manifest.entries.pop();
    write(&manifest);
    for limits in [
        RegistryLimits {
            max_entries: 0,
            ..Default::default()
        },
        RegistryLimits {
            max_files: 2,
            ..Default::default()
        },
        RegistryLimits {
            max_total_bytes: 1,
            ..Default::default()
        },
        RegistryLimits {
            max_manifest_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(ProjectFontRegistry::load(&root, "fonts.json", limits).is_err());
    }
    std::fs::write(dir.path().join("font.ttf"), b"changed").unwrap();
    assert!(matches!(
        load(),
        Err(RegistryError::ResourceMismatch { .. })
    ));
    assert_eq!(held.bytes(), bytes);
    std::fs::remove_file(dir.path().join("font.ttf")).unwrap();
    assert!(matches!(load(), Err(RegistryError::Missing { .. })));
    assert!(ProjectFontRegistry::load(&root, "../fonts.json", RegistryLimits::default()).is_err());
}

#[cfg(unix)]
#[test]
fn project_registry_refuses_rooted_symlinks_and_parent_paths() {
    use flashtex_font_resources::registry::*;
    use flashtex_project_files::ProjectRoot;
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let root = ProjectRoot::open(dir.path()).unwrap();
    let bytes = fixture();
    std::fs::write(outside.path().join("font.ttf"), &bytes).unwrap();
    std::fs::write(dir.path().join("LICENSE"), b"test license").unwrap();
    let mut resource = entry(&bytes);
    resource.path = "font.ttf".into();
    resource.license.text_path = "LICENSE".into();
    let mut manifest = RegistryManifest {
        schema_version: 1,
        entries: vec![RegistryEntry {
            binding: StyleBinding {
                family: "Demo".into(),
                weight: 400,
                style: FontStyle::Upright,
            },
            resource,
        }],
    };
    let write = |m: &RegistryManifest| {
        std::fs::write(
            dir.path().join("fonts.json"),
            serde_json::to_vec(m).unwrap(),
        )
        .unwrap()
    };
    symlink(outside.path().join("font.ttf"), dir.path().join("font.ttf")).unwrap();
    write(&manifest);
    assert!(matches!(
        ProjectFontRegistry::load(&root, "fonts.json", RegistryLimits::default()),
        Err(RegistryError::RootedRead { .. })
    ));
    symlink(outside.path(), dir.path().join("linked")).unwrap();
    manifest.entries[0].resource.path = "linked/font.ttf".into();
    write(&manifest);
    assert!(matches!(
        ProjectFontRegistry::load(&root, "fonts.json", RegistryLimits::default()),
        Err(RegistryError::RootedRead { .. })
    ));
    manifest.entries[0].resource.path = "linked/../font.ttf".into();
    write(&manifest);
    assert!(matches!(
        ProjectFontRegistry::load(&root, "fonts.json", RegistryLimits::default()),
        Err(RegistryError::InvalidManifest(_))
    ));
}

#[test]
fn synthetic_cff_registry_reuses_peer_parser_and_validates_declared_identity() {
    use flashtex_font_resources::registry::*;
    use flashtex_project_files::ProjectRoot;
    let mut bytes = fixture();
    bytes[..4].copy_from_slice(b"OTTO");
    let record = table_record(&bytes, b"glyf");
    let offset = bytes.len();
    let cff = [
        1, 0, 4, 4, 0, 1, 1, 1, 2, b'F', 0, 1, 1, 1, 3, 160, 17, 0, 0, 0, 0, 0, 3, 1, 1, 2, 3, 4,
        14, 14, 14,
    ];
    bytes[record..record + 4].copy_from_slice(b"CFF ");
    be32(&mut bytes, record + 8, offset as u32);
    be32(&mut bytes, record + 12, cff.len() as u32);
    bytes.extend(cff);
    let mut resource = entry(&bytes);
    resource.font.format = "static-cff".into();
    let binding = StyleBinding {
        family: "CFF fixture".into(),
        weight: 400,
        style: FontStyle::Upright,
    };
    let mut manifest = RegistryManifest {
        schema_version: 1,
        entries: vec![RegistryEntry {
            binding: binding.clone(),
            resource: resource.clone(),
        }],
    };
    let dir = tempfile::tempdir().unwrap();
    let root = ProjectRoot::open(dir.path()).unwrap();
    std::fs::write(dir.path().join(&resource.path), &bytes).unwrap();
    std::fs::write(
        dir.path().join(&resource.license.text_path),
        b"test license",
    )
    .unwrap();
    let write = |m: &RegistryManifest| {
        std::fs::write(
            dir.path().join("fonts.json"),
            serde_json::to_vec(m).unwrap(),
        )
        .unwrap()
    };
    write(&manifest);
    let load = || ProjectFontRegistry::load(&root, "fonts.json", RegistryLimits::default());
    let registry = load().unwrap();
    let RegistryResource::Cff(font) = registry.resource(&binding).unwrap() else {
        panic!("CFF resource")
    };
    assert_eq!(font.identity().font_sha256, sha256(&bytes));
    assert_eq!(font.identity().cff_sha256, sha256(&cff));
    assert_eq!(font.identity().table_range, offset..bytes.len());
    assert_eq!(
        font.outline_cache(cff::CacheLimits {
            max_entries: 0,
            max_bytes: 0
        })
        .unwrap()
        .glyph_count(),
        3
    );
    assert!(font.shape_adapter().is_ok());
    assert!(registry.get(&binding).is_err());
    {
        use flashtex_font_resources::registry::vf_project::*;
        let tfmbytes = tfm_bytes_for_encoding();
        let vfbytes = graph_vf_bytes(&[65], 1 << 20, b'c');
        std::fs::write(dir.path().join("cff.tfm"), &tfmbytes).unwrap();
        std::fs::write(dir.path().join("cff.vf"), &vfbytes).unwrap();
        let tfm_asset = Asset {
            path: "cff.tfm".into(),
            sha256: sha256(&tfmbytes),
            license: resource.license.clone(),
        };
        let encoding = cff::CffEncodingManifest {
            font_sha256: font.identity().font_sha256.clone(),
            cff_sha256: font.identity().cff_sha256.clone(),
            tfm_sha256: sha256(&tfmbytes),
            face_index: 0,
            encoding: vec![encoding::EncodingEntry {
                code: 65,
                glyph_name: "space".into(),
            }],
        };
        let mut dependencies = DependencyManifest {
            schema_version: 1,
            registry_generation: registry.generation().into(),
            root: "root".into(),
            nodes: vec![
                Node::CffPhysical {
                    id: "cff".into(),
                    tfm: tfm_asset.clone(),
                    binding: binding.clone(),
                    encoding,
                },
                Node::Virtual {
                    id: "root".into(),
                    tfm: tfm_asset,
                    vf: Asset {
                        path: "cff.vf".into(),
                        sha256: sha256(&vfbytes),
                        license: resource.license.clone(),
                    },
                    fonts: vec![LocalFont {
                        id: 0,
                        target: "cff".into(),
                    }],
                },
            ],
        };
        let write = |m: &DependencyManifest| {
            std::fs::write(
                dir.path().join("cff-deps.json"),
                serde_json::to_vec(m).unwrap(),
            )
            .unwrap()
        };
        write(&dependencies);
        let project =
            ResolvedVfProject::load(&root, "cff-deps.json", &registry, Default::default()).unwrap();
        let result = project.expand(65, registry.generation()).unwrap();
        assert!(
            matches!(&result.placements[0],vf_graph::NestedPlacement::Glyph{resource:vf_graph::ResourceKey::CffPhysical{..},glyph_id:1,source,..} if source.len()==2)
        );
        let mut names = vec!["/.notdef"; 256];
        names[65] = "/space";
        let enc = format!("/Cff[{}]def", names.join(" "));
        std::fs::write(dir.path().join("cff.enc"), enc.as_bytes()).unwrap();
        let mut asset_deps = dependencies.clone();
        asset_deps.schema_version = 2;
        let Node::CffPhysical {
            id, tfm, binding, ..
        } = asset_deps.nodes[0].clone()
        else {
            panic!()
        };
        asset_deps.nodes[0] = Node::CffPhysicalEncodingAsset {
            declarations: None,
            id,
            tfm,
            binding,
            encoding_asset: Asset {
                path: "cff.enc".into(),
                sha256: sha256(enc.as_bytes()),
                license: resource.license.clone(),
            },
        };
        write(&asset_deps);
        let from_asset =
            ResolvedVfProject::load(&root, "cff-deps.json", &registry, Default::default()).unwrap();
        let expanded = from_asset.expand(65, registry.generation()).unwrap();
        assert!(
            matches!(&expanded.placements[0],vf_graph::NestedPlacement::Glyph{glyph_id:1,source,..} if source.len()==2)
        );
        assert!(matches!(
            from_asset
                .physical_run("cff", b"A", registry.generation())
                .unwrap()
                .items
                .as_slice(),
            [encoding::MappedItem::Glyph {
                input_start: 0,
                input_end: 1,
                ..
            }]
        ));
        let Node::CffPhysical { encoding, .. } = &mut dependencies.nodes[0] else {
            panic!()
        };
        encoding.cff_sha256 = "0".repeat(64);
        write(&dependencies);
        assert!(
            ResolvedVfProject::load(&root, "cff-deps.json", &registry, Default::default()).is_err()
        );
    }
    let exported = registry.export_json(1024 * 1024).unwrap();
    std::fs::write(dir.path().join("fonts.json"), &exported).unwrap();
    assert_eq!(load().unwrap().export_json(1024 * 1024).unwrap(), exported);
    let mut wrong = registry.export_manifest();
    wrong.entries[0].cff_table.as_mut().unwrap().offset += 1;
    std::fs::write(
        dir.path().join("fonts.json"),
        serde_json::to_vec(&wrong).unwrap(),
    )
    .unwrap();
    assert!(load().is_err());
    wrong = registry.export_manifest();
    wrong.entries[0].cff_table = None;
    std::fs::write(
        dir.path().join("fonts.json"),
        serde_json::to_vec(&wrong).unwrap(),
    )
    .unwrap();
    assert!(load().is_err());
    wrong = registry.export_manifest();
    wrong.generation = "0".repeat(64);
    std::fs::write(
        dir.path().join("fonts.json"),
        serde_json::to_vec(&wrong).unwrap(),
    )
    .unwrap();
    assert!(matches!(load(), Err(RegistryError::StaleGeneration { .. })));
    for case in 0..5 {
        manifest.entries[0].resource = resource.clone();
        let item = &mut manifest.entries[0].resource;
        match case {
            0 => item.font.face_index = 1,
            1 => item.font.glyph_count = 2,
            2 => item.font.postscript_name = "wrong".into(),
            3 => item.license.text_sha256 = "0".repeat(64),
            _ => item.font.sha256 = "0".repeat(64),
        }
        write(&manifest);
        assert!(matches!(
            load(),
            Err(RegistryError::ResourceMismatch { .. })
        ));
    }
}

#[test]
fn tfm_boundary_run_maps_explicit_glyphs_and_input_intervals() {
    use flashtex_font_resources::{encoding::*, tfm::*};
    let mut data = [20u16, 2, 65, 66, 2, 1, 1, 1, 4, 1, 0, 0]
        .into_iter()
        .flat_map(u16::to_be_bytes)
        .collect::<Vec<_>>();
    for word in [
        0u32,
        10 << 20,
        0x01000101,
        0x01000000,
        0,
        1 << 19,
        0,
        0,
        0,
        0xfffa0000,
        0x80fa8000,
        0x80410042,
        0xff000002,
        (-131072i32) as u32,
    ] {
        data.extend(word.to_be_bytes());
    }
    let tfm = Tfm::parse(&data).unwrap();
    let bytes = fixture();
    let font = FontResource::from_bytes(&entry(&bytes), &bytes, b"test license").unwrap();
    let manifest = encoding_manifest(&font, &tfm);
    let bound = BoundTfmFont::new(&tfm, &font, &manifest).unwrap();
    let run = bound.map_run(b"A").unwrap();
    assert!(matches!(
        run.as_slice(),
        [MappedItem::Glyph {
            tfm_code: 66,
            identity: GlyphIdentity::Notdef,
            input_start: 0,
            input_end: 1,
            ..
        }]
    ));
    let suppressed = bound
        .map_run_with_boundaries(
            b"A",
            BoundaryOptions {
                left: false,
                right: true,
            },
        )
        .unwrap();
    assert!(matches!(
        suppressed.as_slice(),
        [
            MappedItem::Glyph {
                tfm_code: 65,
                identity: GlyphIdentity::Original(2),
                input_start: 0,
                input_end: 1,
                ..
            },
            MappedItem::Kern(FixWord(-131072))
        ]
    ));
    let mut incomplete = manifest.clone();
    incomplete.encoding.retain(|entry| entry.code != 66);
    assert!(BoundTfmFont::new(&tfm, &font, &incomplete)
        .unwrap()
        .map_run(b"A")
        .is_err());
}

#[test]
fn rooted_vf_dependencies_bind_licenses_graph_identity_and_reject_specials() {
    use flashtex_font_resources::{
        registry::{vf_project::*, *},
        vf_graph::*,
    };
    use flashtex_project_files::ProjectRoot;
    let dir = tempfile::tempdir().unwrap();
    let root = ProjectRoot::open(dir.path()).unwrap();
    let bytes = fixture();
    let resource = entry(&bytes);
    let font = FontResource::from_bytes(&resource, &bytes, b"test license").unwrap();
    std::fs::write(dir.path().join(&resource.path), &bytes).unwrap();
    std::fs::write(
        dir.path().join(&resource.license.text_path),
        b"test license",
    )
    .unwrap();
    let binding = StyleBinding {
        family: "Synthetic".into(),
        weight: 400,
        style: FontStyle::Upright,
    };
    let registry_manifest = RegistryManifest {
        schema_version: 1,
        entries: vec![RegistryEntry {
            binding: binding.clone(),
            resource: resource.clone(),
        }],
    };
    std::fs::write(
        dir.path().join("fonts.json"),
        serde_json::to_vec(&registry_manifest).unwrap(),
    )
    .unwrap();
    let registry = ProjectFontRegistry::load(&root, "fonts.json", Default::default()).unwrap();
    let tfm_bytes = tfm_bytes_for_encoding();
    let tfm = tfm_for_encoding();
    let vf_bytes = graph_vf_bytes(&[65], 1 << 20, b'r');
    std::fs::write(dir.path().join("test.tfm"), &tfm_bytes).unwrap();
    std::fs::write(dir.path().join("test.vf"), &vf_bytes).unwrap();
    let tfm_asset = Asset {
        path: "test.tfm".into(),
        sha256: sha256(&tfm_bytes),
        license: resource.license.clone(),
    };
    let vf_asset = Asset {
        path: "test.vf".into(),
        sha256: sha256(&vf_bytes),
        license: resource.license.clone(),
    };
    let mut manifest = DependencyManifest {
        schema_version: 1,
        registry_generation: registry.generation().into(),
        root: "root".into(),
        nodes: vec![
            Node::Physical {
                id: "physical".into(),
                tfm: tfm_asset.clone(),
                binding,
                encoding: encoding_manifest(&font, &tfm),
            },
            Node::Virtual {
                id: "root".into(),
                tfm: tfm_asset,
                vf: vf_asset,
                fonts: vec![LocalFont {
                    id: 0,
                    target: "physical".into(),
                }],
            },
        ],
    };
    let write = |m: &DependencyManifest| {
        std::fs::write(dir.path().join("vf.json"), serde_json::to_vec(m).unwrap()).unwrap()
    };
    write(&manifest);
    let load = || ResolvedVfProject::load(&root, "vf.json", &registry, Default::default());
    let resolved = load().unwrap();
    let packet = resolved.expand(65, registry.generation()).unwrap();
    assert_eq!(packet.placements.len(), 1);
    match &packet.placements[0] {
        NestedPlacement::Glyph {
            glyph_id, source, ..
        } => {
            assert_eq!(*glyph_id, 2);
            assert_eq!(source.len(), 2)
        }
        _ => panic!("glyph"),
    }
    assert_eq!(resolved.license_texts().len(), 3);
    assert!(resolved.expand(65, &"0".repeat(64)).is_err());
    let mut literal_names = vec!["/.notdef"; 256];
    literal_names[65] = "/A.alt";
    let literal = format!("/Project[{}]def", literal_names.join(" "));
    std::fs::write(dir.path().join("project.enc"), literal.as_bytes()).unwrap();
    let mut v2 = manifest.clone();
    v2.schema_version = 2;
    let Node::Physical {
        id,
        tfm,
        binding,
        encoding,
    } = v2.nodes[0].clone()
    else {
        panic!()
    };
    v2.nodes[0] = Node::PhysicalEncodingAsset {
        id,
        tfm,
        binding,
        encoding_asset: Asset {
            path: "project.enc".into(),
            sha256: sha256(literal.as_bytes()),
            license: resource.license.clone(),
        },
        declared_glyphs: encoding.declared_glyphs,
    };
    write(&v2);
    let from_asset = load().unwrap();
    assert_eq!(
        from_asset.expand(65, registry.generation()).unwrap(),
        packet
    );
    assert_ne!(from_asset.generation(), resolved.generation());
    let run = from_asset
        .physical_run("physical", b"A", registry.generation())
        .unwrap();
    assert_eq!(run.project_generation, from_asset.generation());
    assert!(matches!(
        run.items.as_slice(),
        [encoding::MappedItem::Glyph {
            input_start: 0,
            input_end: 1,
            ..
        }]
    ));
    let mut wrong_version = v2.clone();
    wrong_version.schema_version = 1;
    write(&wrong_version);
    assert!(load().is_err());
    write(&v2);
    std::fs::write(dir.path().join("project.enc"), b"changed").unwrap();
    assert!(load().is_err());
    std::fs::write(dir.path().join("project.enc"), literal.as_bytes()).unwrap();
    write(&manifest);
    manifest.nodes.reverse();
    write(&manifest);
    assert_eq!(load().unwrap().generation(), resolved.generation());
    for target in ["missing", "root"] {
        let mut wrong = manifest.clone();
        let Node::Virtual { fonts, .. } = &mut wrong.nodes[0] else {
            panic!()
        };
        fonts[0].target = target.into();
        write(&wrong);
        assert!(load().is_err());
    }
    let mut wrong = manifest.clone();
    let Node::Virtual { fonts, .. } = &mut wrong.nodes[0] else {
        panic!()
    };
    fonts.push(fonts[0].clone());
    write(&wrong);
    assert!(load().is_err());
    write(&manifest);
    assert!(ResolvedVfProject::load(
        &root,
        "vf.json",
        &registry,
        RegistryLimits {
            max_files: 2,
            ..Default::default()
        }
    )
    .is_err());
    let special = graph_vf_bytes(&[239, 2, b'p', b's'], 1 << 20, b's');
    std::fs::write(dir.path().join("test.vf"), &special).unwrap();
    let Node::Virtual { vf, .. } = &mut manifest.nodes[0] else {
        panic!()
    };
    vf.sha256 = sha256(&special);
    write(&manifest);
    assert!(matches!(
        load().unwrap().expand(65, registry.generation()),
        Err(RegistryError::ResourceMismatch {
            error: Error::UnsupportedFont(_),
            ..
        })
    ));
    assert_eq!(resolved.expand(65, registry.generation()).unwrap(), packet);
    std::fs::write(dir.path().join("test.tfm"), b"changed").unwrap();
    assert!(matches!(
        load(),
        Err(RegistryError::ResourceMismatch {
            error: Error::DigestMismatch,
            ..
        })
    ));
    std::fs::write(dir.path().join("test.tfm"), &tfm_bytes).unwrap();
    std::fs::remove_file(dir.path().join(&resource.license.text_path)).unwrap();
    assert!(matches!(load(), Err(RegistryError::Missing { .. })));
    #[cfg(unix)]
    {
        std::fs::write(
            dir.path().join(&resource.license.text_path),
            b"test license",
        )
        .unwrap();
        std::fs::remove_file(dir.path().join("test.tfm")).unwrap();
        std::os::unix::fs::symlink("test.vf", dir.path().join("test.tfm")).unwrap();
        assert!(matches!(load(), Err(RegistryError::RootedRead { .. })));
    }
}

#[test]
fn cff_vf_flat_nested_and_mixed_original_identity() {
    use flashtex_font_resources::{cff::*, encoding::*, vf_graph::*};
    let mut cff_bytes = b"OTTOxxxx".to_vec();
    cff_bytes.extend([
        1, 0, 4, 4, 0, 1, 1, 1, 2, b'F', 0, 1, 1, 1, 3, 160, 17, 0, 0, 0, 0, 0, 2, 1, 1, 2, 3, 14,
        14,
    ]);
    let cache = CffOutlineCache::from_font_table(
        &cff_bytes,
        0,
        8..cff_bytes.len(),
        CacheLimits {
            max_entries: 0,
            max_bytes: 0,
        },
    )
    .unwrap();
    let tfm = tfm_for_encoding();
    let manifest = CffEncodingManifest {
        font_sha256: cache.identity().font_sha256.clone(),
        cff_sha256: cache.identity().cff_sha256.clone(),
        tfm_sha256: tfm.source_sha256.clone(),
        face_index: 0,
        encoding: vec![EncodingEntry {
            code: 65,
            glyph_name: "space".into(),
        }],
    };
    let bound = BoundCffTfmFont::new(&tfm, &cache, &manifest).unwrap();
    let child = graph_vf(&[65], 1 << 19, b'c');
    let root = graph_vf(&[65], 1 << 19, b'r');
    let flat = graph_vf(&[65], 1 << 18, b'f');
    let mut graph = ResourceGraph::new();
    let physical = graph.insert(Resource::CffPhysical(&bound)).unwrap();
    assert!(
        matches!(&physical,ResourceKey::CffPhysical{cff_sha256,encoding_sha256,..} if cff_sha256==&cache.identity().cff_sha256 && encoding_sha256==bound.encoding().encoding_sha256())
    );
    let childkey = graph
        .insert(Resource::Virtual {
            vf: &child,
            tfm: &tfm,
            fonts: BTreeMap::from([(0, physical.clone())]),
        })
        .unwrap();
    let rootkey = graph
        .insert(Resource::Virtual {
            vf: &root,
            tfm: &tfm,
            fonts: BTreeMap::from([(0, childkey)]),
        })
        .unwrap();
    let flatkey = graph
        .insert(Resource::Virtual {
            vf: &flat,
            tfm: &tfm,
            fonts: BTreeMap::from([(0, physical.clone())]),
        })
        .unwrap();
    let nested = graph.expand(&rootkey, 65).unwrap();
    let flat = graph.expand(&flatkey, 65).unwrap();
    match (&nested.placements[0], &flat.placements[0]) {
        (
            NestedPlacement::Glyph {
                resource: a,
                glyph_id: ag,
                x: ax,
                y: ay,
                scale: as_,
                source: ap,
                ..
            },
            NestedPlacement::Glyph {
                resource: b,
                glyph_id: bg,
                x: bx,
                y: by,
                scale: bs,
                source: bp,
                ..
            },
        ) => {
            assert_eq!((a, ag, ax, ay, as_), (b, bg, bx, by, bs));
            assert_eq!(*ag, 1);
            assert_eq!((ap.len(), bp.len()), (3, 2));
        }
        _ => panic!("glyph"),
    }
    let ttbytes = fixture();
    let tt = FontResource::from_bytes(&entry(&ttbytes), &ttbytes, b"test license").unwrap();
    let ttmanifest = encoding_manifest(&tt, &tfm);
    let ttbound = BoundTfmFont::new(&tfm, &tt, &ttmanifest).unwrap();
    let ttkey = graph.insert(Resource::Physical(&ttbound)).unwrap();
    let mut mixedbytes = graph_vf_bytes(&[65, 172, 65], 1 << 20, b'm');
    let mut second = vec![243, 1];
    for n in [0u32, 1 << 20, 10 << 20] {
        second.extend(n.to_be_bytes());
    }
    second.extend([0, 1, b'g']);
    mixedbytes.splice(29..29, second);
    let mixed = flashtex_font_resources::vf::VirtualFont::parse(&mixedbytes).unwrap();
    let mixedkey = graph
        .insert(Resource::Virtual {
            vf: &mixed,
            tfm: &tfm,
            fonts: BTreeMap::from([(0, physical), (1, ttkey)]),
        })
        .unwrap();
    let result = graph.expand(&mixedkey, 65).unwrap();
    assert_eq!(result.placements.len(), 2);
    assert!(matches!(
        result.placements[0],
        NestedPlacement::Glyph {
            resource: ResourceKey::CffPhysical { .. },
            glyph_id: 1,
            ..
        }
    ));
    assert!(matches!(
        result.placements[1],
        NestedPlacement::Glyph {
            resource: ResourceKey::Physical { .. },
            glyph_id: 2,
            ..
        }
    ));
    for name in [Some(".notdef"), None] {
        let mut bad = manifest.clone();
        bad.encoding = name
            .into_iter()
            .map(|n| EncodingEntry {
                code: 65,
                glyph_name: n.into(),
            })
            .collect();
        let binding = BoundCffTfmFont::new(&tfm, &cache, &bad).unwrap();
        let mut g = ResourceGraph::new();
        let key = g.insert(Resource::CffPhysical(&binding)).unwrap();
        assert!(g.expand(&key, 65).is_err());
    }
}

#[test]
fn math_binding_synthetic_limits_and_missing_table() {
    use math_adapter::*;
    use registry::*;
    let dir = tempfile::tempdir().unwrap();
    let root = flashtex_project_files::ProjectRoot::open(dir.path()).unwrap();
    let binding = StyleBinding {
        family: "Synthetic".into(),
        weight: 400,
        style: FontStyle::Upright,
    };
    let load = |bytes: &[u8]| {
        let resource = entry(bytes);
        std::fs::write(dir.path().join(&resource.path), bytes).unwrap();
        std::fs::write(
            dir.path().join(&resource.license.text_path),
            b"test license",
        )
        .unwrap();
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
        ProjectFontRegistry::load(&root, "fonts.json", RegistryLimits::default()).unwrap()
    };
    let original = fixture();
    let absent = load(&original);
    assert!(matches!(
        BoundMathFont::from_registry(
            &absent,
            &binding,
            absent.generation(),
            MathPolicy::UnhintedDesignUnits
        ),
        Err(MathError::MissingMathTable)
    ));
    // Insert one directory record, relocating existing table offsets by16.
    let n = u16::from_be_bytes(original[4..6].try_into().unwrap()) as usize;
    let end = 12 + n * 16;
    let mut bytes = original[..end].to_vec();
    bytes.extend([0; 16]);
    bytes.extend_from_slice(&original[end..]);
    be16(&mut bytes, 4, (n + 1) as u16);
    for i in 0..n {
        let at = 12 + i * 16 + 8;
        let old = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
        be32(&mut bytes, at, old + 16);
    }
    let mut math = vec![0; 224];
    be32(&mut math, 0, 0x00010000);
    be16(&mut math, 4, 10);
    be16(&mut math, 10, 80);
    be16(&mut math, 14, u16::MAX);
    be16(&mut math, 22, i16::MIN as u16); // axis-height signed boundary
    while !bytes.len().is_multiple_of(4) {
        bytes.push(0);
    }
    let off = bytes.len();
    bytes.extend(&math);
    bytes[end..end + 4].copy_from_slice(b"MATH");
    be32(&mut bytes, end + 8, off as u32);
    be32(&mut bytes, end + 12, math.len() as u32);
    let registry = load(&bytes);
    let bound = BoundMathFont::from_registry(
        &registry,
        &binding,
        registry.generation(),
        MathPolicy::UnhintedDesignUnits,
    )
    .unwrap();
    use math_cache::{CacheError, CacheStatus, Limits, MathQueryCache, Query};
    use math_device::{ConstantDeviceRecord, DeviceContext};
    let mut cache = MathQueryCache::new(
        &bound,
        Limits {
            entries: 1,
            query_bytes: 8192,
            parsed_bytes: 0,
        },
    )
    .unwrap();
    let query = Query::Constant {
        record: ConstantDeviceRecord::new(1).unwrap(),
        context: DeviceContext::new(12).unwrap(),
    };
    let first = cache.query(bound.identity(), query.clone()).unwrap();
    assert_eq!(first.status, CacheStatus::Stored);
    let again = cache.query(bound.identity(), query.clone()).unwrap();
    assert_eq!(again.status, CacheStatus::Hit);
    assert!(std::sync::Arc::ptr_eq(&first.outcome, &again.outcome));
    let mut stale = bound.identity().clone();
    stale.registry_generation.push('x');
    assert!(matches!(
        cache.query(&stale, query.clone()),
        Err(CacheError::StaleIdentity)
    ));
    let negative = Query::Fit {
        glyph_id: 0,
        direction: math_variants::Direction::Vertical,
        target: cff::Rational::new(1, 1).unwrap(),
        strategy: math_fit::FitStrategy::EqualExtendersProportionalConnectorFlexibility,
        limits: math_fit::FitLimits::default(),
    };
    let failure = cache.query(bound.identity(), negative.clone()).unwrap();
    assert!(failure.outcome.is_err());
    let hit = cache.query(bound.identity(), negative).unwrap();
    assert_eq!(hit.status, CacheStatus::Hit);
    assert!(std::sync::Arc::ptr_eq(&failure.outcome, &hit.outcome));
    assert_eq!(
        cache.query(bound.identity(), query.clone()).unwrap().status,
        CacheStatus::Stored
    );
    assert!(cache.stats().query_bytes <= 8192);
    assert_eq!(cache.stats().parsed_bytes, 0);
    let mut bypass = MathQueryCache::new(
        &bound,
        Limits {
            entries: 0,
            query_bytes: 0,
            parsed_bytes: 0,
        },
    )
    .unwrap();
    assert_eq!(
        bypass.query(bound.identity(), query).unwrap().status,
        CacheStatus::BypassedOversize
    );
    let mut variation_bytes = bytes.clone();
    be16(&mut variation_bytes, off + 24, 214);
    variation_bytes.extend([0; 8]);
    be16(&mut variation_bytes, off + 224 + 4, 0x8000);
    be32(&mut variation_bytes, end + 12, 232);
    let variation_registry = load(&variation_bytes);
    let variation_font = BoundMathFont::from_registry(
        &variation_registry,
        &binding,
        variation_registry.generation(),
        MathPolicy::UnhintedDesignUnits,
    )
    .unwrap();
    let mut variation_cache = MathQueryCache::new(&variation_font, Limits::default()).unwrap();
    let variation_query = Query::Constant {
        record: ConstantDeviceRecord::new(1).unwrap(),
        context: DeviceContext::new(12).unwrap(),
    };
    let rejected = variation_cache
        .query(variation_font.identity(), variation_query.clone())
        .unwrap();
    assert!(matches!(
        rejected.outcome.as_ref(),
        Err(math_cache::QueryError::Device(
            math_device::DeviceError::UnsupportedVariationIndex
        ))
    ));
    let repeated = variation_cache
        .query(variation_font.identity(), variation_query)
        .unwrap();
    assert_eq!(repeated.status, CacheStatus::Hit);
    assert!(std::sync::Arc::ptr_eq(&rejected.outcome, &repeated.outcome));
    assert_eq!(bound.constants().axis_height, i16::MIN);
    assert_eq!(bound.constants().delimited_sub_formula_min_height, u16::MAX);
    assert_eq!(
        bound.glyphs(&[0, 2]).unwrap()[1].top_accent_attachment,
        None
    );
    assert!(matches!(
        bound.glyphs(&[3]),
        Err(MathError::GlyphOutOfBounds(3))
    ));
    assert!(matches!(
        bound.glyphs(&[0; 257]),
        Err(MathError::LookupBudget)
    ));
}

#[test]
fn explicit_collection_resolver_registry_identity_and_bounds() {
    use flashtex_font_engine::Face;
    use registry::{collections::*, *};
    struct SyntheticResolver(CollectionLayout);
    impl VerifiedCollectionResolver for SyntheticResolver {
        fn implementation_identity(&self) -> &str {
            "synthetic-complete-layout-v1-test-only"
        }
        fn complete_layout(&self, _: &[u8]) -> Result<CollectionLayout> {
            Ok(self.0.clone())
        }
    }
    let original = fixture();
    let n = u16::from_be_bytes(original[4..6].try_into().unwrap()) as usize;
    let mut bytes = vec![0; 20];
    bytes[..4].copy_from_slice(b"ttcf");
    be32(&mut bytes, 4, 0x00010000);
    be32(&mut bytes, 8, 2);
    let mut layout = CollectionLayout {
        header: 0..20,
        faces: Vec::new(),
    };
    for index in 0..2 {
        while !bytes.len().is_multiple_of(4) {
            bytes.push(0)
        }
        let base = bytes.len();
        be32(&mut bytes, 12 + index * 4, base as u32);
        let mut font = original.clone();
        let mut tables = Vec::new();
        for i in 0..n {
            let pos = 12 + i * 16;
            let start =
                u32::from_be_bytes(font[pos + 8..pos + 12].try_into().unwrap()) as usize + base;
            let len = u32::from_be_bytes(font[pos + 12..pos + 16].try_into().unwrap()) as usize;
            be32(&mut font, pos + 8, start as u32);
            tables.push(TableRange {
                tag: font[pos..pos + 4].try_into().unwrap(),
                range: start..start + len,
            });
        }
        layout.faces.push(FaceLayout {
            directory: base..base + 12 + n * 16,
            tables,
        });
        bytes.extend(font);
    }
    let resolver = SyntheticResolver(layout.clone());
    let dir = tempfile::tempdir().unwrap();
    let root = flashtex_project_files::ProjectRoot::open(dir.path()).unwrap();
    let mut resource = entry(&bytes);
    resource.font.format = "collection-truetype".into();
    resource.font.face_index = 1;
    let binding = StyleBinding {
        family: "Explicit Collection".into(),
        weight: 400,
        style: FontStyle::Upright,
    };
    let write = |resource: ManifestEntry| {
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
    };
    std::fs::write(dir.path().join(&resource.path), &bytes).unwrap();
    std::fs::write(
        dir.path().join(&resource.license.text_path),
        b"test license",
    )
    .unwrap();
    write(resource.clone());
    let first = CollectionRegistry::load(&root, "fonts.json", RegistryLimits::default(), &resolver)
        .unwrap();
    let held = first.get(&binding, first.generation()).unwrap();
    assert_eq!(held.declaration().font.face_index, 1);
    assert_eq!(held.bytes(), bytes);
    assert_eq!(held.face().num_glyphs(), 3);
    assert_eq!(held.tables().len(), n);
    resource.font.face_index = 0;
    write(resource.clone());
    let second =
        CollectionRegistry::load(&root, "fonts.json", RegistryLimits::default(), &resolver)
            .unwrap();
    assert_ne!(first.generation(), second.generation());
    assert!(second.get(&binding, first.generation()).is_err());
    assert_ne!(
        held.face().id(),
        second
            .get(&binding, second.generation())
            .unwrap()
            .face()
            .id()
    );
    resource.font.face_index = 2;
    write(resource.clone());
    assert!(
        CollectionRegistry::load(&root, "fonts.json", RegistryLimits::default(), &resolver)
            .is_err()
    );
    resource.font.face_index = 0;
    write(resource);
    let mut overlap = layout.clone();
    overlap.faces[1].directory = overlap.faces[0].directory.clone();
    assert!(CollectionRegistry::load(
        &root,
        "fonts.json",
        RegistryLimits::default(),
        &SyntheticResolver(overlap)
    )
    .is_err());
    let mut mismatch = layout;
    mismatch.faces[0].tables[0].range.start += 4;
    assert!(CollectionRegistry::load(
        &root,
        "fonts.json",
        RegistryLimits::default(),
        &SyntheticResolver(mismatch)
    )
    .is_err());
    std::fs::write(dir.path().join("font.ttf"), b"changed").unwrap();
    assert!(
        CollectionRegistry::load(&root, "fonts.json", RegistryLimits::default(), &resolver)
            .is_err()
    );
    assert_eq!(held.bytes(), bytes);
}

#[test]
fn literal_encoding_feeds_existing_exact_ttf_binding() {
    use enc_file::*;
    let bytes = fixture();
    let font = FontResource::from_bytes(&entry(&bytes), &bytes, b"test license").unwrap();
    let tfm = tfm_for_encoding();
    let mut names = vec!["/.notdef"; 256];
    names[65] = "/A.alt";
    let source = format!("/Explicit[{}]def", names.join(" "));
    let file = EncFile::parse(source.as_bytes(), &sha256(source.as_bytes())).unwrap();
    let bound = file
        .bind_truetype(
            &tfm,
            &font,
            &[encoding::NamedGlyph {
                glyph_name: "A.alt".into(),
                glyph_id: 2,
            }],
        )
        .unwrap();
    assert_eq!(bound.source().file_sha256, sha256(source.as_bytes()));
    assert_eq!(
        bound.font().map_code(65).unwrap().0,
        encoding::GlyphIdentity::Original(2)
    );
    assert_eq!(
        bound.font().map_code(66).unwrap().0,
        encoding::GlyphIdentity::Notdef
    );
    assert!(file.bind_truetype(&tfm, &font, &[]).is_err());
    assert!(file
        .bind_truetype(
            &tfm,
            &font,
            &[encoding::NamedGlyph {
                glyph_name: "A.alt".into(),
                glyph_id: 0
            }]
        )
        .is_err());
}
