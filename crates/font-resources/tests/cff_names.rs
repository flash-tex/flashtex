use flashtex_font_resources::{cff::*, encoding::GlyphIdentity, sha256};
use sha2::{Digest, Sha256};
#[test]
#[ignore = "requires pinned installed STIX font/OFL; this is name-table evidence, not raster parity"]
fn pinned_stix_original_glyph_names() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/stix-cff.json")).unwrap();
    let font = std::env::var("FLASHTEX_STIX_FONT")
        .unwrap_or_else(|_| fixture["font_path"].as_str().unwrap().into());
    let license = std::env::var("FLASHTEX_STIX_LICENSE")
        .unwrap_or_else(|_| fixture["license_path"].as_str().unwrap().into());
    let bytes = std::fs::read(font).unwrap();
    assert_eq!(sha256(&bytes), fixture["font_sha256"].as_str().unwrap());
    assert_eq!(
        sha256(&std::fs::read(license).unwrap()),
        fixture["license_sha256"].as_str().unwrap()
    );
    let start = fixture["cff_range"][0].as_u64().unwrap() as usize;
    let end = fixture["cff_range"][1].as_u64().unwrap() as usize;
    let cache = CffOutlineCache::from_font_table(
        &bytes,
        0,
        start..end,
        CacheLimits {
            max_entries: 8,
            max_bytes: 1048576,
        },
    )
    .unwrap();
    let names = cache.glyph_names().unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &names,
        &cache.glyph_names().unwrap()
    ));
    let mut hash = Sha256::new();
    hash.update(b"cff-glyph-names-v1\0");
    for gid in 0..names.len() as u16 {
        let name = names.glyph_name(gid).unwrap();
        assert_eq!(
            names.resolve(name).unwrap(),
            if gid == 0 {
                GlyphIdentity::Notdef
            } else {
                GlyphIdentity::Original(gid)
            }
        );
        hash.update(gid.to_be_bytes());
        hash.update((name.len() as u16).to_be_bytes());
        hash.update(name.as_bytes());
    }
    let hash = format!("{:x}", hash.finalize());
    eprintln!(
        "names={} names_sha256={} A={:?} fi={:?} retained_name_bytes={}",
        names.len(),
        hash,
        names.resolve("A").unwrap(),
        names.resolve("fi"),
        names.retained_bytes()
    );
    if std::env::var_os("FLASHTEX_RECORD_CFF_NAMES").is_none() {
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/stix-cff-names.json")).unwrap();
        assert_eq!(hash, expected["glyph_names_sha256"].as_str().unwrap());
    }
}
