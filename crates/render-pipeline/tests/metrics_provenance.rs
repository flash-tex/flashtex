//! The metrics a compile actually used are visible in its diagnostics: the
//! pinned Latin Modern 2.004 TFMs load digest-bound through font-resources,
//! and their absence is a blocking `required_metrics_unavailable` error,
//! never a silent switch to OpenType advances.

mod common;

use common::*;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Severity;
use flashtex_render_pipeline::fonts::{Discovery, TfmStatus, REQUIRED_TFMS, REQUIRED_TFM_DIR};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

#[test]
fn required_metrics_load_digest_bound_and_a_clean_compile_means_tex_metrics() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let set = fonts.required_metrics().expect("pinned 12pt metrics load from texmf-dist");
    for (file, sha) in REQUIRED_TFMS {
        let (asset, tfm) = set.get(&format!("fonts/tfm/public/lm/{file}")).unwrap();
        assert_eq!(asset.sha256, sha);
        assert_eq!(tfm.source_sha256, sha, "{file}");
        let t = fonts.tfm(file).unwrap();
        assert_eq!(t.sha256(), sha);
    }
    let docs = [SourceDocument { path: "main.tex", text: "\\begin{document}Body $x^2$ text.\\end{document}" }];
    let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
    // The only diagnostics are the outline-resource profile notes for the
    // math families that have no optical OpenType sibling (lmmi12/lmmi8
    // drawn from Latin Modern Math); metrics are the pinned TFMs.
    let others: Vec<_> = r.v2.diagnostics.iter().filter(|d| d.code != "math_resource_profile").collect();
    assert!(others.is_empty(), "{others:?}");
    assert!(r.v2.diagnostics.iter().any(|d| d.code == "math_resource_profile" && d.message.starts_with("lmmi12")), "{:?}", r.v2.diagnostics);
    let body = fonts.by_name("lmroman12-regular").expect("text face loaded");
    assert_eq!(body.tfm_status, TfmStatus::Loaded);
    assert_eq!(body.tfm.as_ref().unwrap().sha256(), REQUIRED_TFMS[0].1);
}

#[test]
fn missing_required_metrics_are_a_blocking_diagnostic_not_a_silent_fallback() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // The OTFs without their TFM siblings: a font directory whose
    // `fonts/tfm/...` counterpart does not exist.
    let real = FontSet::with_default_dirs(&[]);
    let src_dir = real.dirs().iter().find(|d| d.join("lmroman12-regular.otf").is_file()).unwrap().clone();
    let math_dir = real.dirs().iter().find(|d| d.join("latinmodern-math.otf").is_file()).unwrap().clone();
    let tmp = std::env::temp_dir().join(format!("flashtex-no-tfm-{}", std::process::id()));
    let otf_dir = tmp.join("fonts/opentype/public/lm");
    std::fs::create_dir_all(&otf_dir).unwrap();
    for f in ["lmroman12-regular.otf", "lmroman12-bold.otf", "lmroman12-italic.otf", "lmroman10-regular.otf", "lmroman17-regular.otf"] {
        let _ = std::fs::copy(src_dir.join(f), otf_dir.join(f));
    }
    let _ = std::fs::copy(math_dir.join("latinmodern-math.otf"), otf_dir.join("latinmodern-math.otf"));
    // `with_dirs` with an empty metric list, not `FontSet::new`: `new` reads
    // `FLASHTEX_TFM_DIRS`, so under CI's exported metric trees this control
    // would resolve the very TFMs it exists to prove are missing.
    let fonts = FontSet::with_dirs(vec![otf_dir.clone()], Vec::new());
    assert!(fonts.required_metrics().is_err());
    assert!(matches!(fonts.tfm("ec-lmr12.tfm"), Err(TfmStatus::RequiredUnavailable(_))));
    let docs = [SourceDocument { path: "main.tex", text: "\\begin{document}Body $x^2$ text.\\end{document}" }];
    let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
    let blocking: Vec<_> = r.v2.diagnostics.iter().filter(|d| d.code == "required_metrics_unavailable").collect();
    assert!(blocking.len() >= 2, "text and math roman both report: {:?}", r.v2.diagnostics);
    assert!(blocking.iter().all(|d| d.severity == Severity::Error));
    assert!(blocking[0].message.contains("ec-lmr12.tfm"), "{}", blocking[0].message);
    assert!(!r.v2.pages.is_empty(), "the document is still laid out (OpenType metrics) so the editor shows something");
    let _ = std::fs::remove_dir_all(&tmp);
}

/// The layout an app bundle can ship: one flat directory with the OTFs,
/// the four required TFMs and the GUST licence (`Contents/Resources/Fonts`,
/// or any `FLASHTEX_FONT_DIRS` entry); the required set loads from it
/// digest-bound, exactly like from a texmf tree.
#[test]
fn a_flat_bundle_directory_with_tfms_and_licence_satisfies_the_required_set() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let real = FontSet::with_default_dirs(&[]);
    let src_dir = real.dirs().iter().find(|d| d.join("lmroman12-regular.otf").is_file()).unwrap().clone();
    let math_dir = real.dirs().iter().find(|d| d.join("latinmodern-math.otf").is_file()).unwrap().clone();
    let tfm_dir = real.tfm_dirs().iter().find(|d| d.join("ec-lmr12.tfm").is_file()).unwrap().clone();
    let texmf = tfm_dir.to_string_lossy().trim_end_matches("/fonts/tfm/public/lm").to_string();
    let tmp = std::env::temp_dir().join(format!("flashtex-flat-bundle-{}", std::process::id()));
    let flat = tmp.join("Fonts");
    std::fs::create_dir_all(&flat).unwrap();
    for f in ["lmroman12-regular.otf", "lmroman12-bold.otf", "lmroman12-italic.otf", "lmroman10-regular.otf", "lmroman8-regular.otf", "lmroman6-regular.otf"] {
        let _ = std::fs::copy(src_dir.join(f), flat.join(f));
    }
    let _ = std::fs::copy(math_dir.join("latinmodern-math.otf"), flat.join("latinmodern-math.otf"));
    for (f, _) in REQUIRED_TFMS {
        std::fs::copy(tfm_dir.join(f), flat.join(f)).unwrap();
    }
    std::fs::copy(format!("{texmf}/doc/fonts/lm/GUST-FONT-LICENSE.TXT"), flat.join("GUST-FONT-LICENSE.TXT")).unwrap();
    // The flat directory is its own metric directory, named explicitly:
    // `FontSet::new` would add whatever `FLASHTEX_TFM_DIRS` holds, so this
    // would pass from the ambient texmf trees without proving the flat
    // layout resolves anything.
    let fonts = FontSet::with_dirs(vec![flat.clone()], vec![flat.clone()]);
    fonts.required_metrics().expect("flat layout loads the pinned set");
    assert_eq!(fonts.tfm("ec-lmr12.tfm").unwrap().sha256(), REQUIRED_TFMS[0].1);
    let docs = [SourceDocument { path: "main.tex", text: "\\begin{document}Body $x^2$ text.\\end{document}" }];
    let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
    assert!(r.v2.diagnostics.iter().all(|d| d.code == "math_resource_profile"), "{:?}", r.v2.diagnostics);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Stages an app-bundle layout in a temp dir from the host's Latin Modern
/// files: `Contents/MacOS` (the executable's directory) next to
/// `Contents/Resources/texmf/{fonts/opentype/public/{lm,lm-math},
/// fonts/tfm/public/lm, doc/fonts/lm/GUST-FONT-LICENSE.TXT}`. Returns the
/// `MacOS` directory. The host installation is only the copy source; the
/// font sets built on it never list a host directory.
fn stage_bundle(tag: &str) -> Option<std::path::PathBuf> {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return None;
    }
    let real = FontSet::with_default_dirs(&[]);
    let src_dir = real.dirs().iter().find(|d| d.join("lmroman12-regular.otf").is_file()).unwrap().clone();
    let math_dir = real.dirs().iter().find(|d| d.join("latinmodern-math.otf").is_file()).unwrap().clone();
    let tfm_dir = real.tfm_dirs().iter().find(|d| d.join("ec-lmr12.tfm").is_file()).unwrap().clone();
    let texmf = tfm_dir.to_string_lossy().trim_end_matches("/fonts/tfm/public/lm").to_string();
    let contents = std::env::temp_dir().join(format!("flashtex-bundle-{tag}-{}", std::process::id())).join("Contents");
    let _ = std::fs::remove_dir_all(contents.parent().unwrap());
    let macos = contents.join("MacOS");
    let root = contents.join("Resources/texmf");
    let lm = root.join("fonts/opentype/public/lm");
    let lm_math = root.join("fonts/opentype/public/lm-math");
    let tfms = root.join(REQUIRED_TFM_DIR);
    let doc = root.join("doc/fonts/lm");
    for d in [&macos, &lm, &lm_math, &tfms, &doc] {
        std::fs::create_dir_all(d).unwrap();
    }
    for f in ["lmroman12-regular.otf", "lmroman12-bold.otf", "lmroman12-italic.otf", "lmroman10-regular.otf", "lmroman8-regular.otf", "lmroman6-regular.otf"] {
        let _ = std::fs::copy(src_dir.join(f), lm.join(f));
    }
    std::fs::copy(math_dir.join("latinmodern-math.otf"), lm_math.join("latinmodern-math.otf")).unwrap();
    for (f, _) in REQUIRED_TFMS {
        std::fs::copy(tfm_dir.join(f), tfms.join(f)).unwrap();
    }
    std::fs::copy(format!("{texmf}/doc/fonts/lm/GUST-FONT-LICENSE.TXT"), doc.join("GUST-FONT-LICENSE.TXT")).unwrap();
    Some(macos)
}

/// GH36: the packaged helper finds the rooted LM 2.004 metrics and licence
/// under `<exe>/../Resources/texmf` with no host TeX in its search list,
/// and explicit `FLASHTEX_TFM_DIRS` / `FLASHTEX_FONT_DIRS` entries keep
/// precedence over the bundle.
#[test]
fn bundle_rooted_metrics_are_discovered_relative_to_the_executable_without_host_tex() {
    let Some(macos) = stage_bundle("rooted") else { return };
    let contents = macos.parent().unwrap().to_path_buf();
    let d = Discovery { exe_dir: Some(macos.clone()), ..Discovery::default() };
    let bundled_tfm = macos.join("../Resources/texmf").join(REQUIRED_TFM_DIR);
    // Discovery order: the bundle's TFM directory is the first candidate
    // when no override is set, and every entry before the host list is
    // bundle-relative.
    let font_dirs = d.font_dirs();
    let tfm_dirs = d.tfm_dirs_for(&font_dirs);
    assert_eq!(tfm_dirs[0], bundled_tfm);
    assert_eq!(font_dirs[0], macos.join("../Resources/texmf/fonts/opentype/public/lm"));
    // Overrides first: an explicit TFM directory and font directory precede
    // the bundle, which stays second.
    let over = Discovery {
        tfm_dirs: Some("/override/tfm".into()),
        font_dirs: Some("/override/fonts:/override/more".into()),
        ..d.clone()
    };
    let over_fonts = over.font_dirs();
    assert_eq!(&over_fonts[..2], &[std::path::PathBuf::from("/override/fonts"), "/override/more".into()]);
    let over_tfms = over.tfm_dirs_for(&over_fonts);
    assert_eq!(over_tfms[0], std::path::PathBuf::from("/override/tfm"));
    assert_eq!(over_tfms[1], bundled_tfm);
    // A font set restricted to what the bundle provides (host directories
    // are absent from the temp tree, so they resolve nothing) loads the
    // pinned set digest-bound and compiles with TeX metrics.
    let only_bundle = Discovery { exe_dir: Some(macos.clone()), ..Discovery::default() };
    let bundle_font_dirs: Vec<_> = only_bundle.font_dirs().into_iter().filter(|p| p.starts_with(&contents)).collect();
    let bundle_tfm_dirs: Vec<_> = only_bundle.tfm_dirs_for(&bundle_font_dirs).into_iter().filter(|p| p.starts_with(&contents)).collect();
    assert_eq!(bundle_tfm_dirs[0], bundled_tfm);
    let fonts = FontSet::with_dirs(bundle_font_dirs, bundle_tfm_dirs);
    assert!(fonts.dirs().iter().all(|p| p.starts_with(&contents)), "{:?}", fonts.dirs());
    assert!(fonts.tfm_dirs().iter().all(|p| p.starts_with(&contents)), "{:?}", fonts.tfm_dirs());
    let set = fonts.required_metrics().expect("bundle-rooted set loads");
    for (file, sha) in REQUIRED_TFMS {
        let (asset, tfm) = set.get(&format!("{REQUIRED_TFM_DIR}/{file}")).unwrap();
        assert_eq!(asset.sha256, sha);
        assert_eq!(tfm.source_sha256, sha);
        assert_eq!(fonts.tfm(file).unwrap().sha256(), sha);
    }
    let docs = [SourceDocument { path: "main.tex", text: "\\begin{document}Body $x^2$ text.\\end{document}" }];
    let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
    let others: Vec<_> = r.v2.diagnostics.iter().filter(|d| d.code != "math_resource_profile").collect();
    assert!(others.is_empty(), "{others:?}");
    assert_eq!(fonts.by_name("lmroman12-regular").unwrap().tfm_status, TfmStatus::Loaded);
    let _ = std::fs::remove_dir_all(contents.parent().unwrap());
}

/// GH36: a bundle whose rooted asset is missing or does not match the
/// pinned digest is reported through `required_metrics_unavailable`,
/// never used silently and never replaced by a host TeX lookup.
#[test]
fn a_missing_or_mismatched_bundle_asset_is_the_blocking_diagnostic_never_a_silent_fallback() {
    let Some(macos) = stage_bundle("broken") else { return };
    let contents = macos.parent().unwrap().to_path_buf();
    let d = Discovery { exe_dir: Some(macos.clone()), ..Discovery::default() };
    let font_dirs: Vec<_> = d.font_dirs().into_iter().filter(|p| p.starts_with(&contents)).collect();
    let tfm_dirs: Vec<_> = d.tfm_dirs_for(&font_dirs).into_iter().filter(|p| p.starts_with(&contents)).collect();
    let tfm_path = macos.join("../Resources/texmf").join(REQUIRED_TFM_DIR).join("ec-lmr12.tfm");
    let good = std::fs::read(&tfm_path).unwrap();
    for case in ["mismatched", "missing"] {
        if case == "mismatched" {
            let mut bad = good.clone();
            let i = bad.len() / 2;
            bad[i] ^= 0x5a;
            std::fs::write(&tfm_path, &bad).unwrap();
        } else {
            std::fs::remove_file(&tfm_path).unwrap();
        }
        let fonts = FontSet::with_dirs(font_dirs.clone(), tfm_dirs.clone());
        let err = fonts.required_metrics().err().unwrap_or_else(|| panic!("{case}: loaded"));
        assert!(err.contains("ec-lmr12.tfm"), "{case}: {err}");
        assert!(matches!(fonts.tfm("ec-lmr12.tfm"), Err(TfmStatus::RequiredUnavailable(_))), "{case}");
        let docs = [SourceDocument { path: "main.tex", text: "\\begin{document}Body $x^2$ text.\\end{document}" }];
        let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
        let blocking: Vec<_> = r.v2.diagnostics.iter().filter(|d| d.code == "required_metrics_unavailable").collect();
        assert!(!blocking.is_empty(), "{case}: {:?}", r.v2.diagnostics);
        assert!(blocking.iter().all(|d| d.severity == Severity::Error), "{case}");
        assert!(blocking[0].message.contains("ec-lmr12.tfm"), "{case}: {}", blocking[0].message);
        assert_ne!(fonts.by_name("lmroman12-regular").unwrap().tfm_status, TfmStatus::Loaded, "{case}");
        assert!(!r.v2.pages.is_empty(), "{case}: still laid out so the editor shows something");
    }
    let _ = std::fs::remove_dir_all(contents.parent().unwrap());
}
