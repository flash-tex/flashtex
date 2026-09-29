//! The kpathsea resolver over a Tectonic-style bundle: one flat directory, no
//! texmf.cnf, no ls-R. Hermetic, so CI runs it without TeX Live. (Its own test
//! binary, because kpathsea's configuration is process-wide.)
#![cfg(feature = "kpathsea")]

use flashtex_engine::resolver::{FileResolver, Format, KpathseaResolver};

#[test]
fn flat_bundle_lookups_follow_kpathsea_suffix_rules() {
    let d = std::env::temp_dir().join(format!("flashtex-bundle-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    for f in [
        "story.tex",
        "article.cls",
        "size10.clo",
        "cmr10.tfm",
        "plain.fmt",
        "lm-rm.enc",
    ] {
        std::fs::write(d.join(f), b"x").unwrap();
    }
    std::fs::write(d.join("noext"), b"x").unwrap();
    let mut r = KpathseaResolver::for_bundle(&d, "tex", "");
    let name = |p: Option<std::path::PathBuf>| {
        p.map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
    };
    assert_eq!(
        name(r.find("story", Format::Tex)).as_deref(),
        Some("story.tex")
    );
    assert_eq!(
        name(r.find("story.tex", Format::Tex)).as_deref(),
        Some("story.tex")
    );
    assert_eq!(
        name(r.find("article.cls", Format::Tex)).as_deref(),
        Some("article.cls")
    );
    assert_eq!(
        name(r.find("size10.clo", Format::Tex)).as_deref(),
        Some("size10.clo")
    );
    assert_eq!(
        name(r.find("cmr10", Format::Tfm)).as_deref(),
        Some("cmr10.tfm")
    );
    assert_eq!(
        name(r.find("plain", Format::Fmt)).as_deref(),
        Some("plain.fmt")
    );
    assert_eq!(
        name(r.find("lm-rm.enc", Format::Enc)).as_deref(),
        Some("lm-rm.enc")
    );
    assert_eq!(name(r.find("noext", Format::Tex)).as_deref(), Some("noext"));
    assert_eq!(r.find("missing", Format::Tex), None);
    assert_eq!(r.find("story", Format::Tfm), None);
    let _ = std::fs::remove_dir_all(&d);
}
