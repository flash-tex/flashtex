use flashtex_font_resources::{enc_file::*, encoding::EncodingManifest, sha256};
#[test]
fn published_latin_modern_literal_encoding() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../font-engine/fixtures/tfm");
    let bytes = std::fs::read(root.join("lm-ec.enc")).unwrap();
    let license = std::fs::read(root.join("GUST-FONT-LICENSE.txt")).unwrap();
    assert_eq!(
        sha256(&license),
        "49ea6cb9257bbee0a3979c48a774cd221550ac1c20c95549efe45fc99cc18050"
    );
    let enc = EncFile::parse(
        &bytes,
        "7f9932c402d22a937b853406cfdf4166b80260e3ff21a03fe9a4c05105a2918c",
    )
    .unwrap();
    assert_eq!(enc.source().encoding_name, "enclmec");
    assert_eq!(enc.slots().len(), 256);
    let expected: EncodingManifest =
        serde_json::from_slice(&std::fs::read(root.join("ec-lmr10.encoding.json")).unwrap())
            .unwrap();
    assert_eq!(expected.encoding.len(), 253);
    for slot in expected.encoding {
        assert_eq!(enc.slots()[slot.code as usize].glyph_name, slot.glyph_name);
    }
    let temp = tempfile::tempdir().unwrap();
    let rooted = flashtex_project_files::ProjectRoot::open(temp.path()).unwrap();
    std::fs::write(temp.path().join("font.enc"), &bytes).unwrap();
    let loaded = EncFile::load(&rooted, "font.enc", &sha256(&bytes)).unwrap();
    assert_eq!(loaded.source().project_path.as_deref(), Some("font.enc"));
    assert!(EncFile::load(&rooted, "../font.enc", &sha256(&bytes)).is_err());
    std::fs::write(temp.path().join("font.enc"), b"changed").unwrap();
    assert!(matches!(
        EncFile::load(&rooted, "font.enc", &sha256(&bytes)),
        Err(EncError::DigestMismatch)
    ));
    assert_eq!(loaded.slots().len(), 256);
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join("lm-ec.enc"), temp.path().join("linked.enc")).unwrap();
        assert!(EncFile::load(&rooted, "linked.enc", &sha256(&bytes)).is_err());
    }
}
