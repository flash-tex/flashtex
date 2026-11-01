//! `\usepackage{times}` exports with an embedded program, glyph for glyph
//! where pdfLaTeX puts it.
//!
//! The Times family is laid out with the Adobe Core 14 AFM metrics (the
//! widths and kerns psnfss's `ptm*7t`/`ptm*8t` carry) and used to reach the
//! display list as `core14-afm` fonts, which carry no program: the exact PDF
//! route refused every such document ("font Times-Roman ... is format
//! core14-afm"), 17 arXiv papers of the parity corpus among them. The
//! pipeline now draws those faces with TeX Gyre Termes/Heros/Cursor
//! (`fonts::core14_program_file`, bundled in `apps/mac/Fonts`), where
//! pdfTeX embeds URW's Nimbus Roman/Sans/Mono.
//!
//! `fixtures/times-program/reference.json` is pdfLaTeX's page (MacTeX 2026,
//! made by `fixtures/times-program/oracle.py`, test-only; cargo never runs
//! TeX): every glyph's character, font, x and baseline. Each must be matched
//! by the display list in order, in the corresponding TeX Gyre face, within
//! 0.5 bp on both axes.

use std::path::PathBuf;

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const TOL_BP: f64 = 0.5;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/times-program")
}

fn bundle() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts")
}

/// The bundled tree only: the TeX Gyre programs must come from the
/// repository's own files, not a host TeX.
fn bundled_fonts() -> FontSet {
    let root = bundle();
    let texmf = root.join("texmf/fonts/tfm");
    FontSet::with_dirs(vec![root], vec![texmf.join("public/lm"), texmf.join("jknappen/ec"), texmf.join("public/cm")])
}

/// pdfTeX's font for a glyph as the TeX Gyre face the pipeline draws it in.
fn tex_gyre_for(nimbus: &str) -> &'static str {
    match nimbus {
        "NimbusRomNo9L-Regu" => "TeXGyreTermes-Regular",
        "NimbusRomNo9L-Medi" => "TeXGyreTermes-Bold",
        "NimbusRomNo9L-ReguItal" => "TeXGyreTermes-Italic",
        "NimbusRomNo9L-MediItal" => "TeXGyreTermes-BoldItalic",
        "NimbusSanL-Regu" => "TeXGyreHeros-Regular",
        "NimbusMonL-Regu" => "TeXGyreCursor-Regular",
        other => panic!("unexpected reference font {other}"),
    }
}

/// Cluster text spelled the way the reference reader names the glyph.
fn reference_spelling(text: &str) -> String {
    match text {
        "\u{201C}" => "``".into(),
        "\u{201D}" => "''".into(),
        "\u{FB00}" => "ff".into(),
        "\u{FB01}" => "fi".into(),
        "\u{FB02}" => "fl".into(),
        "\u{FB03}" => "ffi".into(),
        "\u{FB04}" => "ffl".into(),
        t => t.into(),
    }
}

struct Glyph {
    text: String,
    font: String,
    x: f64,
    baseline: f64,
}

#[test]
fn times_page_draws_tex_gyre_where_pdflatex_puts_nimbus() {
    let tex = std::fs::read_to_string(fixture().join("main.tex")).unwrap();
    let reference = json::parse(&std::fs::read_to_string(fixture().join("reference.json")).unwrap()).unwrap();
    let fonts = bundled_fonts();
    let docs = [SourceDocument { path: "main.tex", text: &tex }];
    let r = render(&docs, "main.tex", 1, "times-program", &fonts, &RenderOptions::default());

    for f in &r.v2.fonts {
        assert_ne!(f.format, "core14-afm", "{} still has no program", f.postscript_name);
    }
    let mut ours = Vec::new();
    for page in &r.v2.pages {
        for it in page.resident_items() {
            let Item::GlyphRun(run) = it else { continue };
            let font = r.v2.fonts.iter().find(|f| f.font_id == run.font_id).expect("run font listed");
            assert_eq!(font.format, "opentype-cff", "{}", font.postscript_name);
            let mut seen = std::collections::BTreeSet::new();
            for g in &run.glyphs {
                if !seen.insert(g.cluster) {
                    continue;
                }
                let c = &run.clusters[g.cluster as usize];
                let text = reference_spelling(&run.text[c.text_start_byte..c.text_end_byte]);
                if text.trim().is_empty() {
                    continue;
                }
                ours.push(Glyph { text, font: font.postscript_name.clone(), x: g.origin_x.to_bp(), baseline: g.baseline_y.to_bp() });
            }
        }
    }
    let theirs: Vec<Glyph> = reference
        .get("glyphs")
        .and_then(Value::as_arr)
        .unwrap()
        .iter()
        .map(|g| {
            let s = |k: &str| match g.get(k) {
                Some(Value::Str(s)) => s.clone(),
                _ => panic!("reference glyph without {k}"),
            };
            let n = |k: &str| match g.get(k) {
                Some(Value::Num(n)) => *n,
                _ => panic!("reference glyph without {k}"),
            };
            Glyph { text: s("text"), font: tex_gyre_for(&s("font")).to_string(), x: n("x"), baseline: n("baseline") }
        })
        .collect();
    assert_eq!(ours.len(), theirs.len(), "glyph count vs pdflatex");
    let mut worst = 0.0f64;
    for (i, (a, b)) in ours.iter().zip(&theirs).enumerate() {
        assert_eq!((&a.text, &a.font), (&b.text, &b.font), "glyph {i}");
        let d = (a.x - b.x).abs().max((a.baseline - b.baseline).abs());
        worst = worst.max(d);
        assert!(d <= TOL_BP, "glyph {i} {:?}: ({:.3}, {:.3}) vs pdflatex ({:.3}, {:.3})", a.text, a.x, a.baseline, b.x, b.baseline);
    }
    eprintln!("{} glyphs, worst |d| {worst:.3} bp", theirs.len());

    // The exact route embeds the programs from the bundled directory.
    let out = flashtex_render_pipeline::pdf::write_pdf_exact(&r.v2, &[bundle()], None).expect("times page exports");
    assert_eq!(out.fonts, 6, "{:?}", out.notes);
    for name in ["TeXGyreTermes-Regular", "TeXGyreTermes-BoldItalic", "TeXGyreHeros-Regular", "TeXGyreCursor-Regular"] {
        assert!(out.notes.iter().any(|n| n.contains(name)), "{name} not embedded: {:?}", out.notes);
    }
}

/// A Core 14 face without a program keeps its runs: Symbol has no TeX Gyre
/// counterpart, and a set whose directories hold no TeX Gyre file leaves the
/// Times faces as they were (the export then refuses them, as before).
#[test]
fn core14_program_files() {
    use flashtex_font_engine::core14::Core14;
    use flashtex_render_pipeline::fonts::core14_program_file;
    assert_eq!(core14_program_file(Core14::TimesRoman), Some("texgyretermes-regular.otf"));
    assert_eq!(core14_program_file(Core14::Courier), Some("texgyrecursor-regular.otf"));
    assert_eq!(core14_program_file(Core14::Symbol), None);
    for which in Core14::ALL {
        if let Some(file) = core14_program_file(which) {
            assert!(bundle().join(file).is_file(), "{file} is bundled");
        }
    }
}
