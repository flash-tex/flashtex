//! The kpathsea resolver against an installed TeX Live, checked name by name
//! against `kpsewhich`. Skips (passes vacuously, with a note) where there is
//! no TeX Live, e.g. on CI runners. The full ≥ 500-name comparison is
//! `examples/resolver_corpus.rs`; see docs/evidence/file-resolver-2026-09-29/.
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::{find_texlive_bin, FileResolver, Format, KpathseaResolver};
use std::process::Command;

#[test]
fn agrees_with_kpsewhich() {
    let Some(bin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let mut r = KpathseaResolver::for_texlive(&bin, "pdflatex", "pdftex");
    let cases = [
        ("article.cls", Format::Tex),
        ("latex.ltx", Format::Tex),
        ("plain", Format::Tex),
        ("amsmath.sty", Format::Tex),
        ("cmr10", Format::Tfm),
        ("cmr10.tfm", Format::Tfm),
        ("pdflatex.fmt", Format::Fmt),
        ("cmr10.pfb", Format::Type1),
        ("pdftex.map", Format::Map),
        ("no-such-file-anywhere", Format::Tex),
    ];
    for (name, f) in cases {
        let out = Command::new(bin.join("kpsewhich"))
            .args(["-progname=pdflatex", "-engine=pdftex"])
            .arg(format!("-format={}", f.kpse_name()))
            .arg(name)
            .output()
            .unwrap();
        let want = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let got = r
            .find(name, f)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        assert_eq!(got, want, "{name} ({})", f.kpse_name());
    }
}
