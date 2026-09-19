//! Named font families: fontspec's `\setmainfont`/`\setsansfont`/
//! `\setmonofont`/`\newfontfamily`/`\fontspec` and the manifest's `[fonts]`
//! table, set from any installed font through `flashtex-font-discovery`
//! (`docs/proposals/packages-fonts-manifest.md` §S4).
//!
//! Two kinds of test:
//!
//! * **Hermetic**, over a staged copy of the bundled Latin Modern faces
//!   whose `name` tables call them "Latin Modern Roman"/"Latin Modern
//!   Sans": scope, precedence, substitution notes, `Scale=`, the manifest.
//!   They run everywhere the suite runs.
//! * **This Mac's fonts** (`Helvetica`, `Georgia`, `Bodoni Ornaments` from
//!   `/System/Library/Fonts`): advances against the font's own `hmtx`, a
//!   TrueType collection member, and per-glyph fallback. They skip (and say
//!   so) where the font is not installed, since no committed fixture can
//!   carry an Apple font.

mod common;

use std::path::{Path, PathBuf};

use flashtex_compiler::parser::SourceDocument;
use flashtex_font_engine::Face;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, FontSettings, RenderOptions, Rendered};

/// A staged directory of Latin Modern faces (by file name) for the index,
/// so the tests depend on no installed font. The regular Latin Modern
/// search directories (outlines + TFMs for the class font) stay ambient.
struct Staged(PathBuf);

impl Staged {
    fn new(tag: &str, files: &[&str]) -> Staged {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts");
        let dir = std::env::temp_dir().join(format!("flashtex-named-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for f in files {
            std::fs::copy(src.join(f), dir.join(f)).unwrap_or_else(|e| panic!("{f}: {e}"));
        }
        Staged(dir)
    }

    fn fonts(&self) -> FontSet {
        FontSet::with_default_dirs(&[]).with_index_dirs(vec![self.0.clone()])
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const LM_ALL: [&str; 6] = [
    "lmroman10-regular.otf",
    "lmroman10-bold.otf",
    "lmroman10-italic.otf",
    "lmroman10-bolditalic.otf",
    "lmsans10-regular.otf",
    "lmmono10-regular.otf",
];

fn render_with(text: &str, fonts: &FontSet, options: &RenderOptions) -> Rendered {
    let sources = [SourceDocument { path: "main.tex", text }];
    render(&sources, "main.tex", 7, "named-fonts", fonts, options)
}

/// `(text, postscript name, width bp, font size bp)` of every glyph run.
fn runs(r: &Rendered) -> Vec<(String, String, f64, f64)> {
    let mut out = Vec::new();
    for page in &r.v2.pages {
        for it in page.resident_items() {
            if let Item::GlyphRun(run) = it {
                let font = r.v2.fonts.iter().find(|f| f.font_id == run.font_id).expect("run's font is published");
                let (Some(first), Some(last)) = (run.glyphs.first(), run.glyphs.last()) else { continue };
                let width = (last.origin_x.0 + last.advance_x.0 - first.origin_x.0) as f64 / flashtex_render_pipeline::display::TICKS_PER_BP;
                out.push((run.text.clone(), font.postscript_name.clone(), width, run.font_size.to_bp()));
            }
        }
    }
    out
}

fn font_of<'a>(runs: &'a [(String, String, f64, f64)], word: &str) -> &'a str {
    &runs.iter().find(|r| r.0 == word).unwrap_or_else(|| panic!("no run {word:?} in {runs:?}")).1
}

fn codes(r: &Rendered) -> Vec<(String, String)> {
    r.v2.diagnostics.iter().map(|d| (d.code.clone(), d.message.clone())).collect()
}

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{fontspec}}\n{body}\n")
}

#[test]
fn setmainfont_selects_the_family_for_the_body_and_switches_keep_their_slots() {
    if !common::lm_available() {
        return;
    }
    let s = Staged::new("slots", &LM_ALL);
    let r = render_with(
        &doc("\\setmainfont{Latin Modern Sans}\n\\setmonofont{Latin Modern Roman}\n\\begin{document}\nBody \\textbf{bold} \\textit{italic} \\texttt{mono} \\textsf{sans}.\n\\end{document}"),
        &s.fonts(),
        &RenderOptions::default(),
    );
    let runs = runs(&r);
    // The main font is the named sans; `\textbf`/`\textit` ask for its bold
    // and italic, which the staged index has no sans faces for: the nearest
    // (the upright regular) is used and each substitution is said once.
    assert_eq!(font_of(&runs, "Body"), "LMSans10-Regular");
    assert_eq!(font_of(&runs, "bold"), "LMSans10-Regular");
    assert_eq!(font_of(&runs, "italic"), "LMSans10-Regular");
    // `\texttt` takes the named mono slot (Latin Modern Roman, regular).
    assert_eq!(font_of(&runs, "mono"), "LMRoman10-Regular");
    // `\textsf` has no named sans: the class font's sans as before.
    assert_eq!(font_of(&runs, "sans"), "LMSans10-Regular");
    let notes: Vec<_> = codes(&r).into_iter().filter(|(c, _)| c == "font_face_substituted").collect();
    assert_eq!(notes.len(), 2, "{notes:?}");
    assert!(notes.iter().any(|(_, m)| m.contains("no bold (700) face") && m.contains("LMSans10-Regular") || m.contains("weight 400")), "{notes:?}");
    // The compiler's own diagnostics for the commands are superseded, and
    // the `fontspec` package is no longer "not implemented".
    assert!(!codes(&r).iter().any(|(c, m)| c == "unknown_command" || c == "unsupported_feature" || m.contains("fontspec are recognised")), "{:?}", codes(&r));
}

#[test]
fn fontspec_is_local_to_its_group_and_newfontfamily_defines_a_switch() {
    if !common::lm_available() {
        return;
    }
    let s = Staged::new("scope", &LM_ALL);
    let r = render_with(
        &doc("\\newfontfamily\\sansy{Latin Modern Sans}\n\\begin{document}\nBefore {\\fontspec{Latin Modern Mono} inside} after {\\sansy switched \\textbf{bold}} back.\n\\end{document}"),
        &s.fonts(),
        &RenderOptions::default(),
    );
    let runs = runs(&r);
    assert_eq!(font_of(&runs, "Before"), "LMRoman10-Regular");
    assert_eq!(font_of(&runs, "inside"), "LMMono10-Regular");
    assert_eq!(font_of(&runs, "after"), "LMRoman10-Regular");
    assert_eq!(font_of(&runs, "switched"), "LMSans10-Regular");
    // `\textbf` inside the switch stays in the switch's family.
    assert_eq!(font_of(&runs, "bold"), "LMSans10-Regular");
    assert_eq!(font_of(&runs, "back."), "LMRoman10-Regular");
    // Nothing of the arguments leaked into the page.
    assert!(!runs.iter().any(|r| r.0.contains("Latin") || r.0.contains("Modern") || r.0.contains("Mono")), "{runs:?}");
    assert!(!codes(&r).iter().any(|(c, _)| c == "unknown_command"), "{:?}", codes(&r));
}

#[test]
fn a_missing_family_falls_back_to_latin_modern_with_one_warning() {
    if !common::lm_available() {
        return;
    }
    let s = Staged::new("missing", &LM_ALL[..1]);
    let r = render_with(
        &doc("\\setmainfont{No Such Family 4711}\n\\begin{document}\nText \\textbf{bold} here.\n\\end{document}"),
        &s.fonts(),
        &RenderOptions::default(),
    );
    let runs = runs(&r);
    assert_eq!(font_of(&runs, "Text"), "LMRoman10-Regular");
    assert_eq!(font_of(&runs, "bold"), "LMRoman10-Bold");
    let missing: Vec<_> = codes(&r).into_iter().filter(|(c, _)| c == "font_family_unavailable").collect();
    assert_eq!(missing.len(), 1, "{missing:?}");
    assert!(missing[0].1.contains("No Such Family 4711") && missing[0].1.contains(&s.0.display().to_string()), "{}", missing[0].1);
}

#[test]
fn scale_and_explicit_faces_are_honoured_and_unknown_keys_are_noted_once() {
    if !common::lm_available() {
        return;
    }
    let s = Staged::new("scale", &LM_ALL);
    let plain = render_with(&doc("\\setmainfont{Latin Modern Roman}\n\\begin{document}\nWord\n\\end{document}"), &s.fonts(), &RenderOptions::default());
    let scaled = render_with(
        &doc("\\setmainfont[Scale=1.5,BoldFont={LMRoman10-Italic},Color=FF0000]{Latin Modern Roman}\n\\begin{document}\nWord \\textbf{bold}\n\\end{document}"),
        &s.fonts(),
        &RenderOptions::default(),
    );
    let (a, b) = (runs(&plain), runs(&scaled));
    let w = |r: &[(String, String, f64, f64)]| r.iter().find(|x| x.0 == "Word").map(|x| (x.2, x.3)).unwrap();
    let ((w1, s1), (w2, s2)) = (w(&a), w(&b));
    assert!((s2 / s1 - 1.5).abs() < 1e-6 && (w2 / w1 - 1.5).abs() < 1e-6, "{w1}@{s1} vs {w2}@{s2}");
    // `BoldFont=` names a face by its PostScript name.
    assert_eq!(font_of(&b, "bold"), "LMRoman10-Italic");
    let noted: Vec<_> = codes(&scaled).into_iter().filter(|(c, _)| c == "fontspec_feature_ignored").collect();
    assert_eq!(noted.len(), 1, "{noted:?}");
    assert!(noted[0].1.contains("Color=FF0000"), "{}", noted[0].1);
    // The named face carries no TFM by design: no `tfm_missing` warning.
    assert!(!codes(&scaled).iter().any(|(c, _)| c == "tfm_missing"), "{:?}", codes(&scaled));
}

#[test]
fn the_manifest_sets_the_slots_and_the_document_overrides_it() {
    if !common::lm_available() {
        return;
    }
    let s = Staged::new("manifest", &LM_ALL);
    let options = RenderOptions {
        fonts: Some(FontSettings { text: Some("Latin Modern Sans".into()), mono: Some("Latin Modern Roman".into()), ..FontSettings::default() }),
        ..RenderOptions::default()
    };
    let from_manifest = render_with(&doc("\\begin{document}\nBody \\texttt{mono}\n\\end{document}"), &s.fonts(), &options);
    let r = runs(&from_manifest);
    assert_eq!(font_of(&r, "Body"), "LMSans10-Regular");
    assert_eq!(font_of(&r, "mono"), "LMRoman10-Regular");
    let overridden = render_with(&doc("\\setmainfont{Latin Modern Mono}\n\\begin{document}\nBody \\texttt{mono}\n\\end{document}"), &s.fonts(), &options);
    let r = runs(&overridden);
    assert_eq!(font_of(&r, "Body"), "LMMono10-Regular");
    assert_eq!(font_of(&r, "mono"), "LMRoman10-Regular");
}

#[test]
fn a_document_naming_no_font_is_byte_identical_whatever_the_index_holds() {
    if !common::lm_available() {
        return;
    }
    let text = "\\documentclass{article}\n\\usepackage{lmodern}\n\\begin{document}\nA plain paragraph with \\textbf{bold}, \\textit{italic} and \\texttt{mono} text, and $x^2$.\n\\end{document}\n";
    let ambient = render_with(text, &FontSet::with_default_dirs(&[]), &RenderOptions::default());
    let s = Staged::new("untouched", &LM_ALL);
    let staged = render_with(text, &s.fonts(), &RenderOptions::default());
    let empty = render_with(text, &FontSet::with_default_dirs(&[]).with_index_dirs(vec![PathBuf::from("/nonexistent/flashtex-fonts")]), &RenderOptions::default());
    let wire = flashtex_render_pipeline::display::Wire { images: false, device_color: false, diagnostics: true, links: false };
    let a = ambient.v2.write_json_wire("x", wire);
    assert_eq!(a, staged.v2.write_json_wire("x", wire));
    assert_eq!(a, empty.v2.write_json_wire("x", wire));
    // And the index was never built for it.
    assert!(!codes(&ambient).iter().any(|(c, _)| c.starts_with("font_")), "{:?}", codes(&ambient));
}

/// The installed face of `family` at 400/upright, when this machine has it.
fn installed(family: &str) -> Option<(PathBuf, u32)> {
    let fonts = FontSet::with_default_dirs(&[]);
    let index = fonts.index();
    let f = index.find(family, 400, false)?;
    Some((f.path.clone(), f.face_index))
}

#[test]
fn helvetica_advances_match_the_fonts_hmtx() {
    if !common::lm_available() {
        return;
    }
    let Some((path, face_index)) = installed("Helvetica") else {
        eprintln!("skipping: Helvetica is not installed on this machine");
        return;
    };
    let fonts = FontSet::with_default_dirs(&[]);
    let r = render_with(&doc("\\setmainfont{Helvetica}\n\\begin{document}\nHello Typography, AVAST!\n\\end{document}"), &fonts, &RenderOptions::default());
    let runs = runs(&r);
    let face = flashtex_font_engine::load_from_path_index(&path, face_index).unwrap();
    assert_eq!(font_of(&runs, "Hello"), face.postscript_name());
    // A TrueType collection member is published with its face index and
    // as static TrueType (glyf) outlines.
    let published = r.v2.fonts.iter().find(|f| f.postscript_name == face.postscript_name()).unwrap();
    assert_eq!(published.face_index, face_index);
    assert_eq!(published.format, "static-truetype");
    for word in ["Hello", "Typography,", "AVAST!"] {
        let (_, _, width, size) = runs.iter().find(|x| x.0 == word).unwrap();
        // The word's width is the sum of the glyph advances plus the pair
        // kerns (GPOS `kern` or the `kern` table), at 10pt = 9.963 bp.
        let gids: Vec<_> = word.chars().map(|c| face.glyph_id(c).unwrap()).collect();
        let mut units: i64 = gids.iter().map(|g| i64::from(face.advance(*g).unwrap())).sum();
        for pair in gids.windows(2) {
            units += i64::from(face.kerning(pair[0], pair[1]).0);
        }
        let expected = units as f64 * size / f64::from(face.units_per_em());
        assert!((width - expected).abs() < 0.01, "{word}: {width} vs hmtx {expected}");
    }
    // The interword glue is XeTeX's: the space glyph's advance.
    let space = f64::from(face.advance(face.glyph_id(' ').unwrap()).unwrap()) * 9.963 / f64::from(face.units_per_em());
    let hello = runs.iter().find(|x| x.0 == "Hello").unwrap();
    let typo = runs.iter().find(|x| x.0 == "Typography,").unwrap();
    let page = &r.v2.pages[0];
    let x_of = |text: &str| {
        page.resident_items()
            .iter()
            .find_map(|it| match it {
                Item::GlyphRun(run) if run.text == text => Some(run.glyphs[0].origin_x.to_bp()),
                _ => None,
            })
            .unwrap()
    };
    let gap = x_of("Typography,") - (x_of("Hello") + hello.2);
    let _ = typo;
    // The line is not justified (a one-line paragraph ends with \parfillskip), so the gap is the natural space.
    assert!((gap - space).abs() < 0.01, "gap {gap} vs space {space}");
}

#[test]
fn a_glyph_the_named_font_lacks_is_set_in_latin_modern_not_dropped() {
    if !common::lm_available() {
        return;
    }
    // Chalkduster has the Latin letters but not `ő` (U+0151) or `đ`
    // (U+0111); Latin Modern has both. The word is set in three runs --
    // `Gy` and `r` in Chalkduster, `ő` in Latin Modern -- joined as one
    // word, with one note per (font, character).
    let Some((path, face_index)) = installed("Chalkduster") else {
        eprintln!("skipping: Chalkduster is not installed on this machine");
        return;
    };
    let face = flashtex_font_engine::load_from_path_index(&path, face_index).unwrap();
    if face.glyph_id('ő').is_some() || face.glyph_id('G').is_none() {
        eprintln!("skipping: this Chalkduster's coverage differs from the one the test was written against");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let r = render_with(&doc("\\setmainfont{Chalkduster}\n\\begin{document}\nHello Győr and Đông.\n\\end{document}"), &fonts, &RenderOptions::default());
    let runs = runs(&r);
    assert_eq!(font_of(&runs, "Hello"), face.postscript_name());
    assert_eq!(font_of(&runs, "Gy"), face.postscript_name(), "{runs:?}");
    assert_eq!(font_of(&runs, "ő"), "LMRoman10-Regular");
    assert_eq!(font_of(&runs, "r"), face.postscript_name());
    assert_eq!(font_of(&runs, "ông."), face.postscript_name());
    // The runs of one word abut: `ő` starts where `Gy` ends.
    let page = &r.v2.pages[0];
    let run = |text: &str| {
        page.resident_items()
            .iter()
            .find_map(|it| match it {
                Item::GlyphRun(run) if run.text == text => Some(run),
                _ => None,
            })
            .unwrap()
    };
    let gy = run("Gy");
    let end = (gy.glyphs.last().unwrap().origin_x.0 + gy.glyphs.last().unwrap().advance_x.0) as f64 / flashtex_render_pipeline::display::TICKS_PER_BP;
    assert!((run("ő").glyphs[0].origin_x.to_bp() - end).abs() < 1e-6);
    let missing: Vec<_> = codes(&r).into_iter().filter(|(c, _)| c == "missing_glyph").collect();
    assert_eq!(missing.len(), 2, "{missing:?}");
    assert!(missing.iter().all(|(_, m)| m.contains("set in lmroman10-regular instead")), "{missing:?}");
}

/// The glyph ids of every glyph of the run whose text is `word`.
fn gids(r: &Rendered, word: &str) -> Vec<u16> {
    r.v2.pages[0]
        .resident_items()
        .iter()
        .find_map(|it| match it {
            Item::GlyphRun(run) if run.text == word => Some(run.glyphs.iter().map(|g| g.gid).collect()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no run {word:?}"))
}

#[test]
fn numbers_oldstyle_applies_the_faces_onum_substitutions() {
    if !common::lm_available() {
        return;
    }
    // Latin Modern Roman's `onum` maps the ten figures to `*.oldstyle`
    // glyphs with their own advances (`1.oldstyle` is narrower than `1`).
    let s = Staged::new("onum", &LM_ALL);
    let lining = render_with(&doc("\\setmainfont{Latin Modern Roman}\n\\begin{document}\n1234567890\n\\end{document}"), &s.fonts(), &RenderOptions::default());
    let oldstyle = render_with(&doc("\\setmainfont[Numbers=OldStyle]{Latin Modern Roman}\n\\begin{document}\n1234567890\n\\end{document}"), &s.fonts(), &RenderOptions::default());
    let (a, b) = (gids(&lining, "1234567890"), gids(&oldstyle, "1234567890"));
    assert_eq!(a.len(), 10);
    assert!(a.iter().zip(&b).all(|(x, y)| x != y), "every figure substituted: {a:?} vs {b:?}");
    let face = flashtex_font_engine::load_from_path(&s.0.join("lmroman10-regular.otf")).unwrap();
    let map = FontSet::with_default_dirs(&[]).load_file(&s.fonts().index().find("Latin Modern Roman", 400, false).unwrap().clone()).unwrap().feature_map(b"onum").unwrap();
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(map.get(x).copied(), Some(*y));
    }
    // The run's width is the substitutes' advances.
    let expected: f64 = b.iter().map(|g| f64::from(face.advance(flashtex_font_engine::GlyphId(*g)).unwrap())).sum::<f64>() * 9.963 / 1000.0;
    let width = runs(&oldstyle).iter().find(|r| r.0 == "1234567890").unwrap().2;
    assert!((width - expected).abs() < 0.01, "{width} vs {expected}");
    // No "not applied" note for it.
    assert!(!codes(&oldstyle).iter().any(|(c, _)| c == "fontspec_feature_ignored"), "{:?}", codes(&oldstyle));
}

#[test]
fn scshape_in_a_named_family_uses_its_smcp_or_says_it_has_none() {
    if !common::lm_available() {
        return;
    }
    // A face without `smcp` (Latin Modern Roman has none in the OTF; the
    // small-caps design is a separate family) sets the letters as they are
    // and says so once.
    let s = Staged::new("smcp", &LM_ALL);
    let r = render_with(&doc("\\setmainfont{Latin Modern Roman}\n\\begin{document}\nPlain \\textsc{Caps}\n\\end{document}"), &s.fonts(), &RenderOptions::default());
    assert_eq!(gids(&r, "Caps")[1], gids(&r, "Plain")[2], "the `a` of Caps is the lowercase a");
    let notes: Vec<_> = codes(&r).into_iter().filter(|(c, _)| c == "font_face_substituted").collect();
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].1.contains("no GSUB `smcp`"), "{}", notes[0].1);
    // A face with `smcp` (Iowan Old Style ships with macOS): the lowercase
    // letters become the small capitals, with their advances.
    let Some((path, face_index)) = installed("Iowan Old Style") else {
        eprintln!("skipping the applied half: Iowan Old Style is not installed on this machine");
        return;
    };
    let fonts = FontSet::with_default_dirs(&[]);
    let r = render_with(&doc("\\setmainfont{Iowan Old Style}\n\\begin{document}\nplain \\textsc{plain}\n\\end{document}"), &fonts, &RenderOptions::default());
    let items = r.v2.pages[0].resident_items();
    let plain_runs: Vec<&flashtex_render_pipeline::display::GlyphRun> = items
        .iter()
        .filter_map(|it| match it {
            Item::GlyphRun(run) if run.text == "plain" => Some(run),
            _ => None,
        })
        .collect();
    assert_eq!(plain_runs.len(), 2);
    let face = flashtex_font_engine::load_from_path_index(&path, face_index).unwrap();
    let map = fonts.load_file(fonts.index().find("Iowan Old Style", 400, false).unwrap()).unwrap().feature_map(b"smcp").expect("Iowan Old Style has smcp");
    let (lower, caps) = (&plain_runs[0], &plain_runs[1]);
    for (l, c) in lower.glyphs.iter().zip(&caps.glyphs) {
        assert_eq!(map.get(&l.gid).copied(), Some(c.gid), "{} -> {}", l.gid, c.gid);
        let expected = f64::from(face.advance(flashtex_font_engine::GlyphId(c.gid)).unwrap()) * 9.963 / f64::from(face.units_per_em());
        let kern = c.advance_x.to_bp() - expected;
        // The advance is the small capital's own, plus the base pair's kern.
        assert!(kern.abs() < 0.5, "{}: {} vs hmtx {expected}", c.gid, c.advance_x.to_bp());
    }
    assert!(!codes(&r).iter().any(|(c, _)| c == "font_face_substituted"), "{:?}", codes(&r));
}
