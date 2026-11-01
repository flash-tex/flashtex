use flashtex_font_resources::{pfb::*, sha256, EmbeddingPermission, LicenseMetadata};
#[test]
#[ignore = "set FLASHTEX_LM_TFM_DIR to existing official2.004 directory containing lmr10.pfb and LICENSE"]
fn pinned_official_type1_container_only() {
    let p = std::path::PathBuf::from(std::env::var("FLASHTEX_LM_TFM_DIR").unwrap());
    let bytes = std::fs::read(p.join("lmr10.pfb")).unwrap();
    let license = std::fs::read(p.join("LICENSE")).unwrap();
    let provenance: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/lm-required-metrics-provenance.json"
    ))
    .unwrap();
    assert_eq!(
        sha256(&bytes),
        "84eb01245abb17c0530ca3909427256d73df8bed2d7243b9f2717ca08c010ac8"
    );
    assert_eq!(
        sha256(&license),
        provenance["license"]["sha256"].as_str().unwrap()
    );
    let identity = Identity {
        resource_id: "lmr10-type1-container".into(),
        sha256: sha256(&bytes),
        byte_length: bytes.len() as u64,
        license: LicenseMetadata {
            identifier: "LicenseRef-GUST-Font-License".into(),
            copyright: "see pinned license".into(),
            source: "official GUST lm2.004bas.zip".into(),
            text_path: "LICENSE".into(),
            text_sha256: sha256(&license),
            embedding_permission: EmbeddingPermission::Unknown,
        },
    };
    let resource = Resource::from_bytes(&identity, &bytes, &license).unwrap();
    assert_eq!(resource.segments().len(), 3);
    for s in resource.segments() {
        println!("{:?} {:?}", s.kind, s.payload);
    }
    let inspection =
        flashtex_font_resources::eexec::inspect_binary_eexec(&resource, 112949).unwrap();
    assert_eq!(inspection.random_prefix(), [0; 4]);
    assert_eq!(inspection.plaintext().len(), 112949);
    assert_eq!(
        inspection.plaintext_sha256(),
        "bd88b12233faf829fbf86770638e4aec367847bcdb06e285b75e2381787af9a4"
    );
    assert_eq!(
        inspection.encrypted_sha256(),
        "02262ab31d397263650f1ec77c7bef04d0720419f69aa9e6562a52b2ddc85c62"
    );
    assert_eq!(inspection.resource_identity(), resource.identity());
    let records = flashtex_font_resources::type1_records::extract(&inspection).unwrap();
    assert_eq!(records.len_iv(), 4);
    assert_eq!(records.glyphs().len(), 822);
    assert_eq!(records.subrs().len(), 882);
    assert!(records.opaque_other_subrs_present());
    let a = records.decrypted_glyph("A").unwrap();
    let notdef = records.decrypted_glyph(".notdef").unwrap();
    println!(
        "A {} {} notdef {} {}",
        a.len(),
        sha256(&a),
        notdef.len(),
        sha256(&notdef)
    );
    assert_eq!(
        sha256(&a),
        "13883a7a5915c1d3874a112f50a9e269e7663daf1587f4edbd504e381e16cdec"
    );
    assert_eq!(
        sha256(&notdef),
        "eaa5a748d6652e8857daddbbb79acaa6359e20d1eb90a1b1f0c843b3538a00d8"
    );
    for name in records.glyphs().keys() {
        records.decrypted_glyph(name).unwrap();
    }
    for (index, record) in records.subrs().iter().enumerate() {
        if record.is_some() {
            records.decrypted_subr(index).unwrap();
        }
    }
    let mut outcomes = std::collections::BTreeMap::<String, usize>::new();
    let mut accepted = Vec::new();
    for name in records.glyphs().keys() {
        match flashtex_font_resources::type1_outline::interpret(&records, name) {
            Ok(outline) => {
                *outcomes.entry("accepted".into()).or_default() += 1;
                accepted.push((name.clone(), outline.commands.len()));
                assert_eq!(outline.identity, *resource.identity());
                assert_eq!(outline.commands.len(), outline.sources.len());
            }
            Err(error) => {
                *outcomes.entry(format!("{error:?}")).or_default() += 1;
            }
        }
    }
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/pfb-type1-outline-inventory.json")).unwrap();
    assert_eq!(
        serde_json::to_value(&outcomes).unwrap(),
        expected["outcomes"]
    );
    assert_eq!(
        serde_json::to_value(&accepted).unwrap(),
        expected["accepted"]
    );
    println!("Type1 inventory {:?}; accepted {:?}", outcomes, accepted);
    let mut unhinted = std::collections::BTreeMap::<String, usize>::new();
    let mut stem_count = 0;
    for name in records.glyphs().keys() {
        let policy = flashtex_font_resources::type1_outline::Policy {
            hints: flashtex_font_resources::cff::HintPolicy::Unhinted,
            exact_division: true,
        };
        match flashtex_font_resources::type1_outline::interpret_with_policy(&records, name, policy)
        {
            Ok(out) => {
                *unhinted.entry("accepted".into()).or_default() += 1;
                stem_count += out.stems.len();
                assert_eq!(out.policy, policy)
            }
            Err(e) => {
                *unhinted.entry(format!("{e:?}")).or_default() += 1;
            }
        }
    }
    assert_eq!(
        serde_json::to_value(&unhinted).unwrap(),
        expected["explicit_unhinted_exact_division"]["outcomes"]
    );
    assert_eq!(stem_count, 455);
    println!(
        "Unhinted inventory {:?}, retained stems {}",
        unhinted, stem_count
    );
    let mut rational_outcomes = std::collections::BTreeMap::<String, usize>::new();
    let mut paired = 0;
    let policy = flashtex_font_resources::type1_outline::Policy {
        hints: flashtex_font_resources::cff::HintPolicy::Unhinted,
        exact_division: true,
    };
    for name in records.glyphs().keys() {
        match flashtex_font_resources::type1_outline::interpret_rational(&records, name, policy) {
            Ok(out) => {
                *rational_outcomes.entry("accepted".into()).or_default() += 1;
                if let Ok(old) = flashtex_font_resources::type1_outline::interpret_with_policy(
                    &records, name, policy,
                ) {
                    let converted = out.try_into_dyadic().unwrap();
                    assert_eq!(old.commands, converted.commands);
                    assert_eq!(old.sources, converted.sources);
                    assert_eq!(old.sidebearing, converted.sidebearing);
                    assert_eq!(old.advance, converted.advance);
                    assert_eq!(old.stems, converted.stems);
                    assert_eq!(old.identity, converted.identity);
                    assert_eq!(old.charstring_sha256, converted.charstring_sha256);
                    paired += 1;
                }
            }
            Err(e) => {
                *rational_outcomes.entry(format!("{e:?}")).or_default() += 1;
            }
        }
    }
    assert_eq!(paired, 171);
    assert_eq!(
        serde_json::to_value(&rational_outcomes).unwrap(),
        expected["rational_unhinted_exact_division"]["outcomes"]
    );
    println!(
        "Rational inventory {:?}, paired {}",
        rational_outcomes, paired
    );
    assert!(matches!(
        flashtex_font_resources::type1_matrix::Context::from_resource(&resource),
        Err(flashtex_font_resources::type1_matrix::Error::Header(_))
    ));
    assert_eq!(
        resource.require_outlines(),
        Err(Error::EncryptedOutlinesUnsupported)
    );
}
