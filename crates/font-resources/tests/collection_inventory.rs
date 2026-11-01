use flashtex_font_resources::sha256;
#[test]
#[ignore = "requires exact installed licensed Noto CJK collection; unsupported inventory only"]
fn installed_variable_collection_refusal() {
    let bytes =
        std::fs::read("/usr/share/fonts/google-noto-sans-cjk-vf-fonts/NotoSansCJK-VF.ttc").unwrap();
    let license =
        std::fs::read("/usr/share/licenses/google-noto-sans-cjk-vf-fonts/LICENSE").unwrap();
    assert_eq!(
        sha256(&bytes),
        "d3d8256cdec8dbcb3552284bc6b20c734dd60c2ee9df83b5758e34807c4bac32"
    );
    assert_eq!(
        sha256(&license),
        "6a73f9541c2de74158c0e7cf6b0a58ef774f5a780bf191f2d7ec9cc53efe2bf2"
    );
    let outcome = flashtex_font_engine::TrueTypeFace::parse_with_source(
        bytes,
        flashtex_font_engine::FontSource::Memory { face_index: 0 },
    );
    if let Err(e) = &outcome {
        println!("peer collection outcome {e:?}");
    }
    assert!(
        matches!(outcome,Err(flashtex_font_engine::Error::MissingTable(ref message)) if message == "CFF ")
    );
}
