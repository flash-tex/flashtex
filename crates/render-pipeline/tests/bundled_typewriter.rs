//! The bundled font tree (`apps/mac/Fonts`) must answer every typewriter
//! request the font tables can make, from the repository's own files.
//!
//! Before this guard existed the tree carried no `ectt`/`ecst`/`ecit`/`ectc`
//! (T1 `cmtt`), no `ec-lmtt*` (T1 `lmtt`) and no `lmmono*.otf`, so every
//! `\texttt`/`\ttfamily`/verbatim run against the bundle warned
//! `ec_metrics_unavailable` + `font_outline_substituted` and was laid out on
//! *substituted roman* metrics — silently enough that lanes reported
//! typewriter geometry measured against a substitution. A fallback must fail
//! a gate, not pass it, so these tests assert on the absence of the four
//! substitution diagnostics rather than on a rendered number.
//!
//! Nothing here reads `FLASHTEX_*`: the directories are built from
//! `CARGO_MANIFEST_DIR` and handed to [`FontSet::with_dirs`], so the suite
//! behaves the same with and without the packaging environment (the
//! render-pipeline suite deliberately runs with no `FLASHTEX_*` exports).

use std::path::PathBuf;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Severity;
use flashtex_render_pipeline::fonts::{ec_tfm_file, latin_modern_outline, latin_modern_tfm, Discovery, Role};
use flashtex_render_pipeline::nfss::{self, FamilyKind, FontKey, Scheme, Series, Shape};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// Diagnostics that mean "this layout is not the document that was asked
/// for": a face or its metrics were unavailable and something else was used.
/// A gate that reports geometry must see none of these.
const SUBSTITUTION_CODES: [&str; 4] = [
    "font_unavailable",
    "required_metrics_unavailable",
    "ec_metrics_unavailable",
    "font_outline_substituted",
];

fn bundle() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts")
}

/// The bundled tree as the packaged app exposes it: the flat `Fonts`
/// directory for outlines, and the three rooted metric directories.
fn bundled_fonts() -> FontSet {
    let root = bundle();
    let texmf = root.join("texmf/fonts/tfm");
    FontSet::with_dirs(
        vec![root],
        vec![
            texmf.join("public/lm"),
            texmf.join("jknappen/ec"),
            texmf.join("public/amsfonts/symbols"),
        ],
    )
}

/// Every typewriter shape reachable through NFSS after substitution, at the
/// sizes the standard classes and `\small`…`\Huge` produce. `t1cmtt.fd`
/// declares `<5><6><7><8>#50800`, so 5/6/7 pt share the 8 pt file, and
/// `bx/n`/`bx/it` `ssub` to `m/n`/`m/it`, which is why only the medium
/// series appears in the EC table.
const TT_SIZES: [f64; 14] = [
    5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 10.95, 12.0, 14.4, 17.28, 20.74, 24.88, 29.86, 35.83,
];

/// Every typewriter font LaTeX can actually *load*: each `\ttfamily` shape
/// a document can ask for, in each of the four schemes, put through
/// `\selectfont`'s `\wrong@fontshape` substitution ([`nfss::select`]) and
/// then followed through `sub*`/`ssub*` to a real font ([`nfss::terminal`]).
/// Asking for an undeclared shape such as bold small-caps typewriter is not
/// a bundle gap — LaTeX substitutes it before any font is loaded — so the
/// keys that never reach a `\DeclareFontShape` font are not listed here, and
/// keys that substitute out of `Tt` entirely are dropped.
fn loadable_tt_keys() -> Vec<(Scheme, FontKey)> {
    let mut keys = Vec::new();
    for scheme in [Scheme::CmOt1, Scheme::CmT1, Scheme::LmOt1, Scheme::LmT1] {
        for shape in [Shape::N, Shape::It, Shape::Sl, Shape::Sc, Shape::Scsl, Shape::Scit, Shape::Ui] {
            for series in [Series::M, Series::B, Series::Bx, Series::Sbc] {
                let requested = FontKey::new(FamilyKind::Tt, series, shape);
                let (terminal, _) = nfss::terminal(scheme, nfss::select(scheme, requested).key);
                if terminal.family == FamilyKind::Tt && !keys.contains(&(scheme, terminal)) {
                    keys.push((scheme, terminal));
                }
            }
        }
    }
    assert!(!keys.is_empty());
    keys
}

fn tt_keys() -> Vec<FontKey> {
    let mut keys: Vec<FontKey> = loadable_tt_keys().into_iter().map(|(_, k)| k).collect();
    keys.sort_by_key(|k| format!("{k:?}"));
    keys.dedup();
    keys
}

/// Every `ectt`/`ecst`/`ecit`/`ectc` file `ec_tfm_file` can name for a
/// typewriter role is vendored, and loads as a TFM.
#[test]
fn every_t1_cmtt_metric_the_table_can_request_is_vendored() {
    let fonts = bundled_fonts();
    let dir = bundle().join("texmf/fonts/tfm/jknappen/ec");
    let mut seen = Vec::new();
    for key in tt_keys() {
        for size in TT_SIZES {
            let Some(file) = ec_tfm_file(Role::Font(key), size) else { continue };
            assert!(
                dir.join(&file).is_file(),
                "{file} (T1 cmtt, {size} pt) is not vendored in apps/mac/Fonts/texmf/fonts/tfm/jknappen/ec; \
                 \\texttt would fall back to substituted roman metrics and warn ec_metrics_unavailable"
            );
            fonts.tfm(&file).unwrap_or_else(|e| panic!("{file} does not load: {e:?}"));
            seen.push(file);
        }
    }
    seen.sort();
    seen.dedup();
    // ectt/ecst/ecit/ectc, each at the 11 distinct sizes t1cmtt.fd declares.
    assert_eq!(seen.len(), 44, "{seen:?}");
}

/// Every `lmmono*.otf` outline `latin_modern_outline` can name for a
/// typewriter shape is vendored, together with the `ec-lmtt*` metric
/// `latin_modern_tfm` pairs with it.
#[test]
fn every_lmtt_outline_and_metric_the_tables_can_request_are_vendored() {
    let fonts = bundled_fonts();
    let root = bundle();
    let lm = root.join("texmf/fonts/tfm/public/lm");
    let (mut outlines, mut metrics) = (Vec::new(), Vec::new());
    for key in tt_keys() {
        for size in TT_SIZES {
            let (file, note) = latin_modern_outline(key, size);
            assert!(
                file.starts_with("lmmono"),
                "{key:?} at {size} pt resolves to {file} ({note:?}), not a Latin Modern typewriter design"
            );
            assert!(
                root.join(&file).is_file(),
                "{file} ({key:?} at {size} pt) is not vendored in apps/mac/Fonts; the outline would be \
                 substituted and warn font_outline_substituted"
            );
            let stem = file.trim_end_matches(".otf");
            let tfm = latin_modern_tfm(stem).unwrap_or_else(|| panic!("no Latin Modern metric pairs with {file}"));
            assert!(
                lm.join(&tfm).is_file(),
                "{tfm} (the T1 lmtt metric for {file}) is not vendored in \
                 apps/mac/Fonts/texmf/fonts/tfm/public/lm"
            );
            fonts.tfm(&tfm).unwrap_or_else(|e| panic!("{tfm} does not load: {e:?}"));
            outlines.push(file);
            metrics.push(tfm);
        }
    }
    for v in [&mut outlines, &mut metrics] {
        v.sort();
        v.dedup();
    }
    assert_eq!(outlines.len(), 10, "{outlines:?}");
    assert_eq!(metrics.len(), 10, "{metrics:?}");
}

/// Both vendored pins cover exactly what is on disk for the files this lane
/// added, so `bundle-texmf.py check` and `make-app.sh` refuse a drifted or
/// unpinned typewriter file.
#[test]
fn the_vendored_typewriter_files_are_pinned() {
    let root = bundle();
    let metrics = std::fs::read_to_string(root.join("texmf/SUPPLEMENTARY-METRICS.json")).unwrap();
    let faces = std::fs::read_to_string(root.join("SUPPLEMENTARY-FACES.json")).unwrap();
    for key in tt_keys() {
        for size in TT_SIZES {
            if let Some(file) = ec_tfm_file(Role::Font(key), size) {
                let path = format!("fonts/tfm/jknappen/ec/{file}");
                assert!(metrics.contains(&path), "{path} is on disk but not in SUPPLEMENTARY-METRICS.json");
            }
            let (otf, _) = latin_modern_outline(key, size);
            assert!(faces.contains(&otf), "{otf} is on disk but not in SUPPLEMENTARY-FACES.json");
            let tfm = latin_modern_tfm(otf.trim_end_matches(".otf")).unwrap();
            let path = format!("fonts/tfm/public/lm/{tfm}");
            assert!(metrics.contains(&path), "{path} is on disk but not in SUPPLEMENTARY-METRICS.json");
        }
    }
}

/// Every discovery route a shipped binary can use must reach the bundled
/// typewriter files, not only an explicit `FLASHTEX_TFM_DIRS`.
///
/// `Discovery::bundle_texmf_roots` probes `<exe>/../Resources/texmf`,
/// `<exe>/texmf` and `<exe>/../share/flashtex/texmf`; `Discovery::font_dirs`
/// probes the matching `Fonts` directories. `FontSet::texmf_roots` in turn
/// derives a texmf root from any `FLASHTEX_TFM_DIRS` entry spelled
/// `<root>/fonts/tfm/public/lm`, which is why the environment route works
/// **only** with that exact suffix — pointing the variable at
/// `apps/mac/Fonts` itself finds no root and no flat root, and every metric
/// falls back. That is the whole 0/59-vs-59/59 difference on the amsmath
/// corpus, and it is a spelling trap, not a missing code path.
#[test]
fn the_bundled_typewriter_files_are_reachable_by_every_discovery_route() {
    let root = bundle();
    let texmf = root.join("texmf/fonts/tfm");
    let rooted = |exe_dir: PathBuf| FontSet::from_discovery(&Discovery { exe_dir: Some(exe_dir), ..Discovery::default() });

    // The two bundle layouts, staged as a shipped binary would see them.
    let stage = std::env::temp_dir().join(format!("flashtex-tt-roots-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&stage);
    for (label, exe_rel, texmf_rel, fonts_rel) in [
        ("app bundle", "Contents/MacOS", "Contents/Resources/texmf", "Contents/Resources/Fonts"),
        ("tarball", "bin", "bin/texmf", "bin/Fonts"),
    ] {
        let here = stage.join(label.replace(' ', "-"));
        std::fs::create_dir_all(here.join(exe_rel)).unwrap();
        copy_tree(&root.join("texmf"), &here.join(texmf_rel));
        std::fs::create_dir_all(here.join(fonts_rel)).unwrap();
        for f in std::fs::read_dir(&root).unwrap().flatten() {
            let name = f.file_name();
            if name.to_string_lossy().ends_with(".otf") {
                std::fs::copy(f.path(), here.join(fonts_rel).join(&name)).unwrap();
            }
        }
        let fonts = rooted(here.join(exe_rel));
        assert!(fonts.required_metrics().is_ok(), "{label}: {:?}", fonts.required_metrics().err());
        assert_no_substitution(&fonts, label);
    }
    let _ = std::fs::remove_dir_all(&stage);

    // The environment route, spelled correctly.
    let env = FontSet::with_dirs(
        vec![root.clone()],
        vec![texmf.join("public/lm"), texmf.join("jknappen/ec"), texmf.join("public/amsfonts/symbols")],
    );
    assert!(env.required_metrics().is_ok(), "{:?}", env.required_metrics().err());
    assert_no_substitution(&env, "FLASHTEX_TFM_DIRS = the three rooted metric directories");

    // The trap: the same files, named by a directory no root can be derived
    // from. This must FAIL, or a fallback run would pass a gate.
    let trap = FontSet::with_dirs(vec![root.clone()], vec![root.clone()]);
    assert!(trap.required_metrics().is_err(), "apps/mac/Fonts is not a texmf root and holds no flat required set");
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if src.is_dir() {
            copy_tree(&src, &dst);
        } else {
            std::fs::copy(&src, &dst).unwrap();
        }
    }
}

/// Renders the typewriter probe documents through `fonts` and asserts that
/// none of the four substitution diagnostics appears.
fn assert_no_substitution(fonts: &FontSet, label: &str) {
    for (scheme, preamble) in PROBES {
        let text = probe(preamble);
        let docs = [SourceDocument { path: "main.tex", text: &text }];
        let r = render(&docs, "main.tex", 1, "p", fonts, &RenderOptions::default());
        let bad: Vec<_> = r.v2.diagnostics.iter().filter(|d| SUBSTITUTION_CODES.contains(&d.code.as_str())).collect();
        assert!(bad.is_empty(), "{label} / {scheme}: the bundle substituted a typewriter face or its metrics: {bad:?}");
        assert!(!r.v2.pages.is_empty(), "{label} / {scheme}: nothing was laid out");
    }
}

const PROBES: [(&str, &str); 4] = [
    ("OT1", "\\documentclass{article}"),
    ("T1", "\\documentclass{article}\\usepackage[T1]{fontenc}"),
    ("lmodern", "\\documentclass{article}\\usepackage[T1]{fontenc}\\usepackage{lmodern}"),
    ("12pt T1", "\\documentclass[12pt]{article}\\usepackage[T1]{fontenc}"),
];

fn probe(preamble: &str) -> String {
    format!(
        "{preamble}\\begin{{document}}\
         Body text, then \\texttt{{typewriter 0123}} and {{\\ttfamily more}}.\n\n\
         {{\\small\\texttt{{small}}}} {{\\large\\texttt{{large}}}} {{\\Huge\\texttt{{huge}}}}\n\n\
         \\textbf{{\\texttt{{bold}}}} \\textit{{\\texttt{{italic}}}} \\textsl{{\\texttt{{slanted}}}}\n\
         \\end{{document}}"
    )
}

/// The end-to-end property the corpus measures: a `\texttt` document laid
/// out against the bundled tree substitutes nothing.
#[test]
fn a_texttt_document_lays_out_on_the_bundle_with_no_substitution() {
    let fonts = bundled_fonts();
    for (label, preamble) in [
        ("OT1", "\\documentclass{article}"),
        ("T1", "\\documentclass{article}\\usepackage[T1]{fontenc}"),
        ("lmodern", "\\documentclass{article}\\usepackage[T1]{fontenc}\\usepackage{lmodern}"),
        ("12pt T1", "\\documentclass[12pt]{article}\\usepackage[T1]{fontenc}"),
    ] {
        let text = format!(
            "{preamble}\\begin{{document}}\
             Body text, then \\texttt{{typewriter 0123}} and {{\\ttfamily more}}.\n\n\
             {{\\small\\texttt{{small}}}} {{\\large\\texttt{{large}}}} {{\\Huge\\texttt{{huge}}}}\n\n\
             \\textbf{{\\texttt{{bold}}}} \\textit{{\\texttt{{italic}}}} \\textsl{{\\texttt{{slanted}}}}\n\
             \\end{{document}}"
        );
        let docs = [SourceDocument { path: "main.tex", text: &text }];
        let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
        let bad: Vec<_> = r
            .v2
            .diagnostics
            .iter()
            .filter(|d| SUBSTITUTION_CODES.contains(&d.code.as_str()))
            .collect();
        assert!(bad.is_empty(), "{label}: the bundle substituted a typewriter face or its metrics: {bad:?}");
        assert!(
            r.v2.diagnostics.iter().all(|d| d.severity != Severity::Error),
            "{label}: {:?}",
            r.v2.diagnostics
        );
        assert!(!r.v2.pages.is_empty(), "{label}: nothing was laid out");
    }
}
