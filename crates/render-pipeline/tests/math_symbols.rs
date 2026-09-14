//! The compiler's control-word symbols (pin `49e6eb43`) through TeX's
//! metrics and Latin Modern Math: composites (`\neq` = `\not=`), the extra
//! cmsy slots (`\perp`), `\cdot`'s class, `\left`/`\right` fences re-derived
//! from the source, and the typed limitation for `\angle`.

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::typeset::{convert_math, fence_before, fence_of, Fence};
use flashtex_math_layout::{AtomClass, Nucleus};

fn doc(body: &str) -> String {
    format!("\\begin{{document}}{body}\\end{{document}}")
}

/// A document that loads `amssymb`, for bodies using the msam/msbm
/// inventory (`\mathbb`, `\varnothing`, `\aleph`, ...).
///
/// Base LaTeX2e has no definition for those names, so the compiler now
/// diagnoses them without the package (`\varnothing requires
/// \usepackage{amssymb}`) and typesets the command literally, exactly as
/// pdflatex refuses them. These fixtures are about which *face* paints the
/// glyph, not about the gating, so they declare the package a real document
/// would have to declare.
fn ams_doc(body: &str) -> String {
    format!("\\usepackage{{amssymb}}\\begin{{document}}{body}\\end{{document}}")
}

/// Math glyph runs of page 1 as `(text, origin x bp)`.
fn math_words(body: &str) -> (Vec<(String, f64)>, Vec<String>) {
    let r = render_one(&doc(body));
    let mut out = Vec::new();
    for item in &r.v2.pages[0].items {
        if let Item::GlyphRun(run) = item {
            out.push((run.text.clone(), run.glyphs[0].origin_x.to_bp()));
        }
    }
    let diags = r.v1_diagnostics_codes();
    (out, diags)
}

trait Codes {
    fn v1_diagnostics_codes(&self) -> Vec<String>;
}

impl Codes for flashtex_render_pipeline::Rendered {
    fn v1_diagnostics_codes(&self) -> Vec<String> {
        self.v2.diagnostics.iter().map(|d| format!("{}:{}", d.code, d.message)).collect()
    }
}

#[test]
fn fences_are_recovered_from_the_source_bytes() {
    assert_eq!(fence_before("$\\left( x", 6), Some(Fence::Left));
    assert_eq!(fence_before("$\\left  ( x", 8), Some(Fence::Left));
    assert_eq!(fence_before("x \\right)", 8), Some(Fence::Right));
    assert_eq!(fence_before("x \\\\left(", 8), None, "an escaped backslash is not a control word");
    assert_eq!(fence_before("( x", 0), None);
    // Compiler pin 87df3e4a: the delimiter span starts at the control word.
    assert_eq!(fence_of("$\\left( x", 1), Some(Fence::Left));
    assert_eq!(fence_of("x \\right)", 2), Some(Fence::Right));
    assert_eq!(fence_of("$\\leftarrow", 1), None, "\\leftarrow is not a fence");
    assert_eq!(fence_of("$\\left( x", 6), Some(Fence::Left), "older pins: span at the delimiter");
}

#[test]
fn composite_and_extra_symbols_convert_with_texbook_classes() {
    // Compiler pin d416472a: `MathAtom` carries a crate-private
    // `class_override`, so the list is built by the compiler's own math
    // parser from the control words that produce these symbols.
    let tokens = flashtex_compiler::lexer::tokenize("a \\neq b \\cdot c \\perp d \\notin e");
    let mut diagnostics = Vec::new();
    // `MathPackages::default()` is every package absent: \neq, \cdot, \perp
    // and \notin are all LaTeX kernel commands, so they must convert with no
    // package loaded (compiler pin past #230, which gates the
    // amssymb/amsfonts-provided names on their package).
    let list = flashtex_compiler::math::parse_tokens(
        &tokens,
        flashtex_compiler::math::MathPackages::default(),
        &mut diagnostics,
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let list = convert_math(&list);
    let classes: Vec<(AtomClass, Option<char>)> = list
        .atoms
        .iter()
        .map(|a| (a.class, if let Nucleus::Symbol(c) = a.nucleus { Some(c) } else { None }))
        .collect();
    use AtomClass::*;
    assert_eq!(
        classes,
        vec![
            (Ord, Some('a')),
            (Rel, Some('\u{0338}')),
            (Rel, Some('=')),
            (Ord, Some('b')),
            (Bin, Some('\u{22C5}')),
            (Ord, Some('c')),
            (Rel, Some('\u{22A5}')),
            (Ord, Some('d')),
            (Rel, Some('\u{0338}')),
            (Rel, Some('\u{2208}')),
            (Ord, Some('e')),
        ]
    );
}

#[test]
// The vendored compiler needs the integration lane's re-pin before this can run.
#[ignore = "needs the vendor/compiler re-pin past #314"]
fn odot_is_painted_as_a_binary_math_glyph() {
    if !lm_available() {
        return;
    }
    let (words, diags) = math_words(r"$a \odot b$");
    assert!(
        words.iter().any(|(text, _)| text == "⊙"),
        "odot glyph was not drawn: words={words:?}, diagnostics={diags:?}"
    );
    assert!(
        diags.iter().all(|diagnostic| !diagnostic.contains("\\odot")),
        "odot should not produce a compiler diagnostic: {diags:?}"
    );
}

#[test]
fn every_compiler_symbol_typesets_without_a_missing_glyph() {
    if !lm_available() {
        return;
    }
    let mut body = String::from("$");
    for (name, _) in flashtex_compiler::math::COMMAND_GLYPHS {
        body.push_str(&format!("\\{name} "));
    }
    body.push('$');
    let (words, diags) = math_words(&body);
    assert!(!words.is_empty());
    let limitations: Vec<&String> = diags.iter().filter(|d| d.starts_with("math_limitation") || d.starts_with("missing_glyph")).collect();
    // Compiler pin 887bf21 (main) no longer lists \angle (a constructed
    // macro in fontmath.ltx, not a glyph), so every listed symbol has a glyph.
    assert_eq!(limitations.len(), 0, "{limitations:?}");
}

#[test]
fn not_equal_is_a_zero_width_slash_over_the_equals_sign() {
    if !lm_available() {
        return;
    }
    let (eq, _) = math_words("$a = b$");
    let (neq, _) = math_words("$a \\neq b$");
    // The slash is placed (it joins the preceding run as a combining
    // character) and `b` sits where it sits after `=`: the negation slash
    // has no width (cmsy 0x36).
    assert!(neq.iter().any(|(t, _)| t.contains('\u{0338}')), "{neq:?}");
    let b_eq = eq.last().unwrap().1;
    let b_neq = neq.last().unwrap().1;
    assert!((b_eq - b_neq).abs() < 1e-6, "b at {b_neq} vs {b_eq}");
}

#[test]
fn left_right_grows_the_delimiter_to_the_body() {
    if !lm_available() {
        return;
    }
    let plain = render_one(&doc("$( \\frac{a}{b} )$"));
    let fenced = render_one(&doc("$\\left( \\frac{a}{b} \\right)$"));
    let width = |r: &flashtex_render_pipeline::Rendered| -> f64 {
        let mut xs: Vec<f64> = Vec::new();
        for item in &r.v2.pages[0].items {
            if let Item::GlyphRun(run) = item {
                for g in &run.glyphs {
                    xs.push(g.origin_x.to_bp());
                }
            }
        }
        xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min)
    };
    assert!(fenced.v2.diagnostics.iter().all(|d| d.code != "math_limitation"), "{:?}", fenced.v2.diagnostics);
    // A \left( around a text-style fraction selects a larger cmex variant
    // than the 12 pt roman parenthesis, so the closing delimiter moves right.
    assert!(width(&fenced) > width(&plain) + 0.5, "fenced {} vs plain {}", width(&fenced), width(&plain));
}

/// `(gid, ink height + depth in bp, ink top and bottom in bp relative to
/// the first text word's baseline: negative = above)` of every glyph drawn
/// from Latin Modern Math whose cluster text is `text`, page 1.
fn math_face_glyphs(body: &str, text: &str) -> Vec<(u16, f64, f64, f64)> {
    use flashtex_compiler::parser::SourceDocument;
    use flashtex_render_pipeline::display::RunRole;
    use flashtex_render_pipeline::ids::GlyphId;
    use flashtex_render_pipeline::{render, FontSet, RenderOptions};
    let fonts = FontSet::with_default_dirs(&[]);
    let text_doc = doc(body);
    let r = render(&[SourceDocument { path: "main.tex", text: &text_doc }], "main.tex", 7, "test-project", &fonts, &RenderOptions::default());
    let text_baseline = r.v2.pages[0]
        .items
        .iter()
        .find_map(|item| match item {
            Item::GlyphRun(run) if run.role == RunRole::Text => Some(run.glyphs[0].baseline_y.to_bp()),
            _ => None,
        })
        .expect("a text word");
    let mut out = Vec::new();
    for item in &r.v2.pages[0].items {
        if let Item::GlyphRun(run) = item {
            if run.role != RunRole::Math {
                continue;
            }
            let face = fonts.by_font_id(&run.font_id).expect("resource of a drawn run");
            if !face.name.to_ascii_lowercase().contains("math") {
                continue;
            }
            let size = run.font_size.to_bp();
            for g in &run.glyphs {
                let c = &run.clusters[g.cluster as usize];
                if &run.text[c.text_start_byte as usize..c.text_end_byte as usize] != text {
                    continue;
                }
                let b = face.bounds(GlyphId(g.gid), None);
                let (top, bottom) = (face.pt(i64::from(b.y_max), size), face.pt(-i64::from(b.y_min), size));
                let y = g.baseline_y.to_bp() - text_baseline;
                out.push((g.gid, top + bottom, y - top, y + bottom));
            }
        }
    }
    out
}

/// The cmex chain step math-layout selects (`\Big(` = cmex 0x10, 18 pt, for
/// a `\left(` around a text-style fraction; `\bigg(` = 0x12, 24 pt, in
/// display — pdflatex sets `lmex10` codes 0x10/0x12 there in the oracle
/// PDFs) must be painted with the Latin Modern Math variant of that size.
/// The face lists seven parenthesis sizes against cmex's four, so a
/// same-index pick draws 11.9 pt / 14.4 pt glyphs where TeX sets 18 / 24.
#[test]
fn left_right_paints_the_variant_of_the_selected_cmex_size() {
    if !lm_available() {
        return;
    }
    let inline = math_face_glyphs("Inline $\\left( \\frac{a}{b} \\right)$ text.", "(");
    let display = math_face_glyphs("Display \\[ \\left( \\frac{a}{b} \\right) \\] after.", "(");
    assert_eq!(inline.len(), 1, "{inline:?}");
    assert_eq!(display.len(), 1, "{display:?}");
    let (gid_t, h_t, top_t, bottom_t) = inline[0];
    let (gid_d, h_d, _, _) = display[0];
    // cmex10 is `sfixed` at 10 pt: the \Big box is 18 pt, the \bigg box 24 pt.
    assert!((h_t - 18.0).abs() < 0.3, "inline \\left( draws gid {gid_t}, {h_t:.2} pt tall; TeX's \\Big( box is 18 pt");
    assert!((h_d - 24.0).abs() < 0.3, "display \\left( draws gid {gid_d}, {h_d:.2} pt tall; TeX's \\bigg( box is 24 pt");
    assert_ne!(gid_t, gid_d);
    // Vertical placement: pdflatex sets the \Big( origin 11.557 pt above the
    // text baseline (oracle content stream, 08-delimiters), and the cmex
    // outline hangs from it (0.4 pt above, 17.6 below): ink from 11.96 pt
    // above the baseline to 6.04 pt below. The OpenType variant is centred
    // on the axis relative to its own origin, so painting it at the cmex
    // origin would lift it ~9 pt; the painter re-centres it on the TFM box.
    assert!((top_t + 11.96).abs() < 0.3, "inline \\left( ink top {top_t:.2} pt from the baseline; TeX: -11.96");
    assert!((bottom_t - 6.04).abs() < 0.3, "inline \\left( ink bottom {bottom_t:.2} pt from the baseline; TeX: +6.04");
    // The radical chain lists exactly the cmex sizes: \sqrt over a text-style
    // fraction takes cmex 0x71 (18 pt) and paints the 18 pt sign.
    let sqrt = math_face_glyphs("Inline $\\sqrt{\\frac{a}{b}}$ text.", "\u{221A}");
    assert_eq!(sqrt.len(), 1, "{sqrt:?}");
    assert!((sqrt[0].1 - 18.0).abs() < 0.3, "\\sqrt sign gid {} is {:.2} pt tall; cmex 0x71 is 18 pt", sqrt[0].0, sqrt[0].1);
    // Text-size \sum is a cmex glyph too (0x50, hanging 10 pt below its
    // origin); centred on the axis (3 pt at 12 pt) its ink spans 8 pt above
    // to 2 pt below the baseline, as pdflatex draws lmex10.
    let sum = math_face_glyphs("Inline $\\sum_{i=1}^{n} i$ text.", "\u{2211}");
    assert_eq!(sum.len(), 1, "{sum:?}");
    let (_, _, top_s, bottom_s) = sum[0];
    assert!((top_s + 8.0).abs() < 0.4 && (bottom_s - 2.0).abs() < 0.4, "text \\sum ink {top_s:.2}..{bottom_s:.2} pt from the baseline; TeX: -8.0..+2.0");
}

/// Symbols with no Computer Modern slot (`\mathbb`, `\setminus`,
/// `\Longrightarrow`, `\aleph`) are drawn from Latin Modern Math through
/// `OTF_FALLBACK_FONT`, whose id lies above the `\text` run range: the
/// painter must not look them up as run glyphs (they were silently
/// dropped). The math minus is U+2212, not the text hyphen.
#[test]
fn cm_less_symbols_are_painted_from_latin_modern_math() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // A control for the *fallback* face, so it must not see
    // `NewCMMath-Regular.otf`: with NewCM in `FLASHTEX_FONT_DIRS` the
    // double-struck glyphs paint from it instead and `ℤℵ` splits into two
    // runs, which is the documented NewCM behaviour, not the drop this test
    // guards against. `mathbb_paints_from_new_computer_modern_when_bundled`
    // covers the bundled case.
    let staged = stage_faces_without("cm_less_symbols", &["NewCMMath-Regular.otf"]);
    let fonts = staged.font_set(&[], ambient_tfm_dirs());
    let r = render_one_with(&ams_doc("A $\\mathbb{Z}\\aleph$ $\\mathbb{R}\\setminus\\mathbb{Q}$ $x\\Longrightarrow y$ $10 - x$ B"), &fonts);
    let runs: Vec<(String, u16)> = r.v2.pages[0]
        .items
        .iter()
        .filter_map(|it| match it {
            Item::GlyphRun(run) => Some((run.text.clone(), run.glyphs[0].gid)),
            _ => None,
        })
        .collect();
    let texts: Vec<&str> = runs.iter().map(|(t, _)| t.as_str()).collect();
    assert!(texts.contains(&"ℤℵ"), "\\mathbb{{Z}}\\aleph dropped: {texts:?}");
    assert!(texts.contains(&"ℝ∖ℚ"), "\\mathbb{{R}}\\setminus\\mathbb{{Q}} dropped: {texts:?}");
    assert!(texts.contains(&"x⟹y"), "\\Longrightarrow dropped: {texts:?}");
    // The run text keeps the source's ASCII hyphen; the painted glyph is
    // Latin Modern Math's U+2212 (gid 2615 in the pinned font 6075562b…),
    // not its text hyphen (gid 14).
    let minus = runs.iter().find(|(t, _)| t.starts_with('-')).expect("the minus run");
    assert_eq!(minus.1, 2615, "math minus should paint U+2212: {runs:?}");
}

/// pdfLaTeX's `\mathbb` is AMS `msbm10`'s serifed double-struck design;
/// Latin Modern Math's is the sans-like open face. With
/// `NewCMMath-Regular.otf` (New Computer Modern Math reproduces the msbm
/// design) in a font directory, every double-struck run paints from that
/// secondary face — its own `fonts` entry by raw-byte sha256 — and no
/// profile note is emitted; without it, Latin Modern Math paints as before
/// and exactly one typed `math_resource_profile` note names msbm10.
#[test]
fn mathbb_paints_from_new_computer_modern_when_bundled() {
    use flashtex_compiler::parser::SourceDocument;
    use flashtex_render_pipeline::{render, FontSet, RenderOptions};
    use std::path::PathBuf;

    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    const NEWCM: &str = "NewCMMath-Regular.otf";
    const CONTROL_TAG: &str = "mathbb";
    const NEWCM_SHA: &str = "60394d357348f68cd301764fe61cc502a5858e1c4ff21b948a1d14d82586a7a2";
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts"),
        PathBuf::from("/usr/local/texlive/2026/texmf-dist/fonts/opentype/public/newcomputermodern"),
    ];
    let text = ams_doc("A $\\mathbb{Z}_{>0}$ and $\\mathbb{R}\\setminus\\mathbb{Q}$ and $x \\in \\mathbb{N}$.");
    let sources = [SourceDocument { path: "main.tex", text: &text }];
    // Hermetic, not `FontSet::with_default_dirs(extra)`: that reads
    // `FLASHTEX_FONT_DIRS`, and `apps/mac/Fonts` -- exactly what CI exports --
    // ships `NewCMMath-Regular.otf` beside the Latin Modern faces. The
    // `without` half is a control that asserts NewCM is absent, so under a
    // real font environment it saw the very face it exists to rule out. Stage
    // the faces explicitly, omitting NewCM, and let the bundled half add it
    // back through `extra`.
    let staged = stage_faces_without(CONTROL_TAG, &[NEWCM]);
    let tfm_dirs = ambient_tfm_dirs();
    let render_with = |extra: &[PathBuf]| {
        let fonts = staged.font_set(extra, tfm_dirs.clone());
        render(&sources, "main.tex", 1, "bb", &fonts, &RenderOptions::default())
    };
    // The face each double-struck run paints from, by its `fonts` entry.
    let bb_faces = |r: &flashtex_render_pipeline::Rendered| -> Vec<(String, String, String)> {
        r.v2.pages[0]
            .items
            .iter()
            .filter_map(|it| match it {
                Item::GlyphRun(run) if run.text.chars().any(|c| matches!(c, 'ℤ' | 'ℝ' | 'ℚ' | 'ℕ')) => {
                    let f = r.v2.fonts.iter().find(|f| f.font_id == run.font_id).expect("run font is a fonts entry");
                    Some((run.text.clone(), f.postscript_name.clone(), f.sha256.clone()))
                }
                _ => None,
            })
            .collect()
    };
    let bb_notes = |r: &flashtex_render_pipeline::Rendered| -> Vec<String> {
        r.v2.diagnostics
            .iter()
            .filter(|d| d.code == "math_resource_profile" && d.message.starts_with("msbm10"))
            .map(|d| d.message.clone())
            .collect()
    };

    // Without the secondary face: Latin Modern Math, one profile note.
    let without = render_with(&[]);
    let faces = bb_faces(&without);
    assert!(faces.len() >= 3, "double-struck runs: {faces:?}");
    for (text, ps, _) in &faces {
        assert_eq!(ps, "LatinModernMath-Regular", "{text} without NewCM");
    }
    let notes = bb_notes(&without);
    assert_eq!(notes.len(), 1, "one msbm10 profile note without NewCM: {notes:?}");
    assert!(notes[0].contains(NEWCM), "{}", notes[0]);
    assert!(!without.v2.fonts.iter().any(|f| f.postscript_name == "NewCMMath-Regular"));

    let Some(dir) = candidates.iter().find(|d| d.join(NEWCM).is_file()) else {
        eprintln!("skipping the bundled half: {NEWCM} not found");
        return;
    };
    let with = render_with(std::slice::from_ref(dir));
    let faces = bb_faces(&with);
    assert!(faces.len() >= 3, "double-struck runs: {faces:?}");
    for (text, ps, sha) in &faces {
        assert_eq!(ps, "NewCMMath-Regular", "{text} with NewCM");
        assert_eq!(sha, NEWCM_SHA, "{text} raw-byte identity");
    }
    let newcm = with.v2.fonts.iter().find(|f| f.postscript_name == "NewCMMath-Regular").expect("NewCM fonts entry");
    assert_eq!(newcm.sha256, NEWCM_SHA);
    assert_eq!(newcm.byte_length, 1_187_476);
    assert!(with.v2.fonts.iter().any(|f| f.postscript_name == "LatinModernMath-Regular"), "LM Math still draws the rest");
    assert!(bb_notes(&with).is_empty(), "no msbm10 note with NewCM: {:?}", bb_notes(&with));
    // The rest of the formula is unchanged: every non-double-struck glyph
    // keeps its face and glyph id (runs split differently because `ℝ∖ℚ`
    // now alternates faces, and x positions move with NewCM's msbm-like
    // advances, which is the point).
    let others = |r: &flashtex_render_pipeline::Rendered| -> Vec<(String, String, u16)> {
        let mut out = Vec::new();
        for it in &r.v2.pages[0].items {
            let Item::GlyphRun(run) = it else { continue };
            let ps = r.v2.fonts.iter().find(|f| f.font_id == run.font_id).map(|f| f.postscript_name.clone()).unwrap_or_default();
            for g in &run.glyphs {
                let c = &run.clusters[g.cluster as usize];
                let text = run.text[c.text_start_byte..c.text_end_byte].to_string();
                if text.chars().any(|c| matches!(c, 'ℤ' | 'ℝ' | 'ℚ' | 'ℕ')) {
                    continue;
                }
                out.push((text, ps.clone(), g.gid));
            }
        }
        out
    };
    assert_eq!(others(&with), others(&without));
}

/// `\mathcal` (compiler pin `dbf6ec78`): the compiler emits the Unicode
/// script capitals and binds them to New Computer Modern Math. The
/// pipeline sets each at cmsy10's slot (`fontmath.ltx`: `\mathcal` is the
/// `symbols` alphabet), so the advance is cmsy10's width plus italic
/// correction — `\mathcal{P}`: 0.6955595 + 0.082222 em, the distance
/// pdfLaTeX leaves before the following `(` — and paints it from
/// `NewCMMath-Regular.otf` when that face is in a font directory (its own
/// `fonts` entry by raw-byte sha256, one typed profile note naming it),
/// else from Latin Modern Math. No compiler error, no dropped glyph, and
/// the layout is the same either way because the metrics are the TFM's.
#[test]
fn mathcal_sets_at_cmsy_metrics_and_paints_from_new_computer_modern_when_bundled() {
    use flashtex_compiler::parser::SourceDocument;
    use flashtex_render_pipeline::display::{Severity, Tick};
    use flashtex_render_pipeline::{render, FontSet, RenderOptions};
    use std::path::PathBuf;

    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    const NEWCM: &str = "NewCMMath-Regular.otf";
    const CONTROL_TAG: &str = "mathcal";
    const NEWCM_SHA: &str = "60394d357348f68cd301764fe61cc502a5858e1c4ff21b948a1d14d82586a7a2";
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts"),
        PathBuf::from("/usr/local/texlive/2026/texmf-dist/fonts/opentype/public/newcomputermodern"),
    ];
    let text = doc("power set $\\mathcal{P}(T)$ and $\\mathcal{A}\\cup\\mathcal{B}$.");
    let sources = [SourceDocument { path: "main.tex", text: &text }];
    // Hermetic, not `FontSet::with_default_dirs(extra)`: that reads
    // `FLASHTEX_FONT_DIRS`, and `apps/mac/Fonts` -- exactly what CI exports --
    // ships `NewCMMath-Regular.otf` beside the Latin Modern faces. The
    // `without` half is a control that asserts NewCM is absent, so under a
    // real font environment it saw the very face it exists to rule out. Stage
    // the faces explicitly, omitting NewCM, and let the bundled half add it
    // back through `extra`.
    let staged = stage_faces_without(CONTROL_TAG, &[NEWCM]);
    let tfm_dirs = ambient_tfm_dirs();
    let render_with = |extra: &[PathBuf]| {
        let fonts = staged.font_set(extra, tfm_dirs.clone());
        render(&sources, "main.tex", 1, "cal", &fonts, &RenderOptions::default())
    };
    let is_cal = |c: char| flashtex_compiler::newcm_math::advance(c).is_some();
    // Every calligraphic glyph: text, face, raw-byte identity, origin x,
    // size (runs split differently once the faces alternate).
    let cal_runs = |r: &flashtex_render_pipeline::Rendered| -> Vec<(String, String, String, Tick, Tick)> {
        let mut out = Vec::new();
        for it in &r.v2.pages[0].items {
            let Item::GlyphRun(run) = it else { continue };
            let f = r.v2.fonts.iter().find(|f| f.font_id == run.font_id).expect("run font is a fonts entry");
            for g in &run.glyphs {
                let c = &run.clusters[g.cluster as usize];
                let text = run.text[c.text_start_byte..c.text_end_byte].to_string();
                if text.chars().any(is_cal) {
                    out.push((text, f.postscript_name.clone(), f.sha256.clone(), g.origin_x, run.font_size));
                }
            }
        }
        out
    };
    // The `(` right after `\mathcal{P}`.
    let paren_x = |r: &flashtex_render_pipeline::Rendered| -> Tick {
        r.v2.pages[0]
            .items
            .iter()
            .find_map(|it| match it {
                Item::GlyphRun(run) if run.text == "(" => Some(run.glyphs[0].origin_x),
                _ => None,
            })
            .expect("the ( after \\mathcal{P}")
    };

    let without = render_with(&[]);
    assert!(!without.v2.diagnostics.iter().any(|d| d.severity == Severity::Error), "{:?}", without.v2.diagnostics);
    let runs = cal_runs(&without);
    assert_eq!(runs.iter().map(|r| r.0.as_str()).collect::<Vec<_>>(), ["𝒫", "𝒜", "ℬ"], "{runs:?}");
    for (text, ps, _, _, _) in &runs {
        assert_eq!(ps, "LatinModernMath-Regular", "{text} without NewCM");
    }
    // cmsy10 slot 0x50: width 0.6955595 em + italic correction 0.082222 em.
    let (p_x, size) = (runs[0].3 .0 as f64, runs[0].4 .0 as f64);
    let expected = (0.6955595 + 0.082222) * size;
    let actual = paren_x(&without).0 as f64 - p_x;
    assert!((actual - expected).abs() < 0.002 * size, "P advance {actual} ticks vs cmsy10 {expected}");

    let Some(dir) = candidates.iter().find(|d| d.join(NEWCM).is_file()) else {
        eprintln!("skipping the bundled half: {NEWCM} not found");
        return;
    };
    let with = render_with(std::slice::from_ref(dir));
    let runs_with = cal_runs(&with);
    assert_eq!(runs_with.len(), 3, "{runs_with:?}");
    for (text, ps, sha, _, _) in &runs_with {
        assert_eq!(ps, "NewCMMath-Regular", "{text} with NewCM");
        assert_eq!(sha, NEWCM_SHA, "{text} raw-byte identity");
    }
    for (a, b) in runs.iter().zip(&runs_with) {
        assert_eq!(a.0, b.0);
        assert_eq!(a.3, b.3, "{} origin moves with the painting face", a.0);
    }
    assert_eq!(paren_x(&with), paren_x(&without));
    let notes: Vec<_> = with
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.code == "math_resource_profile" && d.message.contains("\\mathcal"))
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(notes.len(), 1, "one \\mathcal profile note with NewCM: {notes:?}");
    assert!(notes[0].contains("NewCMMath-Regular"), "{}", notes[0]);
}

/// `\varnothing` (compiler pin `c583d6d4`: `MathAtom.width_em` is now
/// `pub`, `Some(0.777781)` — msbm10's advance — while `\emptyset`'s
/// identical U+2205 glyph carries `None` and keeps cmsy10's narrower
/// 0.5em). The pipeline reads the field directly at the atom
/// (`typeset::symbol_atoms`) rather than re-scanning the source for the
/// control word: both commands still set cmsy10's `\emptyset` slot 0x3B,
/// but `\varnothing`'s advance is forced to msbm10's width regardless of
/// which face paints it, and the outline itself comes from
/// `NewCMMath-Regular.otf` when that face is in a font directory (its
/// design and advances track msbm's, like `\mathbb`/`\mathcal`), else from
/// Latin Modern Math with the forced width unchanged.
#[test]
fn varnothing_sets_at_msbm_width_and_paints_from_new_computer_modern_when_bundled() {
    use flashtex_compiler::parser::SourceDocument;
    use flashtex_render_pipeline::display::{Severity, Tick};
    use flashtex_render_pipeline::{render, FontSet, RenderOptions};
    use std::path::PathBuf;

    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    const NEWCM: &str = "NewCMMath-Regular.otf";
    const CONTROL_TAG: &str = "varnothing";
    const NEWCM_SHA: &str = "60394d357348f68cd301764fe61cc502a5858e1c4ff21b948a1d14d82586a7a2";
    const VARNOTHING_EM: f64 = 0.777781;
    const EMPTYSET_EM: f64 = 0.5;
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts"),
        PathBuf::from("/usr/local/texlive/2026/texmf-dist/fonts/opentype/public/newcomputermodern"),
    ];
    let text = ams_doc("$\\varnothing X$ and $\\emptyset X$.");
    let sources = [SourceDocument { path: "main.tex", text: &text }];
    // Hermetic, not `FontSet::with_default_dirs(extra)`: that reads
    // `FLASHTEX_FONT_DIRS`, and `apps/mac/Fonts` -- exactly what CI exports --
    // ships `NewCMMath-Regular.otf` beside the Latin Modern faces. The
    // `without` half is a control that asserts NewCM is absent, so under a
    // real font environment it saw the very face it exists to rule out. Stage
    // the faces explicitly, omitting NewCM, and let the bundled half add it
    // back through `extra`.
    let staged = stage_faces_without(CONTROL_TAG, &[NEWCM]);
    let tfm_dirs = ambient_tfm_dirs();
    let render_with = |extra: &[PathBuf]| {
        let fonts = staged.font_set(extra, tfm_dirs.clone());
        render(&sources, "main.tex", 1, "varnothing", &fonts, &RenderOptions::default())
    };
    // Every glyph of page 1 as `(text, painting face, sha256, origin x, size)`.
    let glyphs = |r: &flashtex_render_pipeline::Rendered| -> Vec<(String, String, String, Tick, Tick)> {
        let mut out = Vec::new();
        for it in &r.v2.pages[0].items {
            let Item::GlyphRun(run) = it else { continue };
            let f = r.v2.fonts.iter().find(|f| f.font_id == run.font_id).expect("run font is a fonts entry");
            for g in &run.glyphs {
                let c = &run.clusters[g.cluster as usize];
                let t = run.text[c.text_start_byte..c.text_end_byte].to_string();
                out.push((t, f.postscript_name.clone(), f.sha256.clone(), g.origin_x, run.font_size));
            }
        }
        out
    };
    // `∅` occurs twice (`\varnothing` then `\emptyset`); each is immediately
    // followed by `X`, so `x(X) - x(∅)` is the advance regardless of face
    // or run-splitting differences between the two renders.
    let advances = |gs: &[(String, String, String, Tick, Tick)]| -> Vec<(String, f64, f64)> {
        let empties: Vec<usize> = gs.iter().enumerate().filter(|(_, g)| g.0 == "∅").map(|(i, _)| i).collect();
        assert_eq!(empties.len(), 2, "\\varnothing then \\emptyset: {gs:?}");
        empties
            .into_iter()
            .map(|i| {
                let x = gs[i].3 .0 as f64;
                let size = gs[i].4 .0 as f64;
                let next_x = gs[i + 1..].iter().find(|g| g.0 == "X").unwrap_or_else(|| panic!("no X after ∅ at {i}: {gs:?}")).3 .0 as f64;
                (gs[i].1.clone(), next_x - x, size)
            })
            .collect()
    };

    let without = render_with(&[]);
    assert!(!without.v2.diagnostics.iter().any(|d| d.severity == Severity::Error), "{:?}", without.v2.diagnostics);
    let gs_without = glyphs(&without);
    let adv_without = advances(&gs_without);
    let (varnothing_face, varnothing_advance, size) = &adv_without[0];
    let (emptyset_face, emptyset_advance, _) = &adv_without[1];
    assert_eq!(varnothing_face, "LatinModernMath-Regular", "\\varnothing without NewCM");
    assert_eq!(emptyset_face, "LatinModernMath-Regular", "\\emptyset without NewCM");
    assert!((varnothing_advance - VARNOTHING_EM * size).abs() < 0.002 * size, "\\varnothing advance {varnothing_advance} vs msbm10 {}", VARNOTHING_EM * size);
    assert!((emptyset_advance - EMPTYSET_EM * size).abs() < 0.002 * size, "\\emptyset advance {emptyset_advance} vs cmsy10 {}", EMPTYSET_EM * size);
    assert!((varnothing_advance - emptyset_advance).abs() > 0.1 * size, "the two must differ: {varnothing_advance} vs {emptyset_advance}");

    let Some(dir) = candidates.iter().find(|d| d.join(NEWCM).is_file()) else {
        eprintln!("skipping the bundled half: {NEWCM} not found");
        return;
    };
    let with = render_with(std::slice::from_ref(dir));
    let gs_with = glyphs(&with);
    let adv_with = advances(&gs_with);
    let (varnothing_face_with, varnothing_advance_with, _) = &adv_with[0];
    let (emptyset_face_with, emptyset_advance_with, _) = &adv_with[1];
    assert_eq!(varnothing_face_with, "NewCMMath-Regular", "\\varnothing with NewCM");
    let varnothing_sha = gs_with.iter().find(|g| g.0 == "∅" && g.1 == "NewCMMath-Regular").map(|g| g.2.clone()).expect("a NewCM ∅ run");
    assert_eq!(varnothing_sha, NEWCM_SHA, "\\varnothing raw-byte identity");
    // \emptyset is untouched: still Latin Modern Math, same advance.
    assert_eq!(emptyset_face_with, "LatinModernMath-Regular", "\\emptyset stays put with NewCM");
    assert_eq!(emptyset_advance_with, emptyset_advance, "\\emptyset advance unchanged");
    // \varnothing's advance is the compiler's forced width either way: only
    // the painting face moves, exactly like \mathbb/\mathcal.
    assert_eq!(varnothing_advance_with, varnothing_advance, "\\varnothing advance unchanged by the painting face");

    let notes: Vec<_> = with
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.code == "math_resource_profile" && d.message.contains("\\varnothing"))
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(notes.len(), 1, "one \\varnothing profile note with NewCM: {notes:?}");
    assert!(notes[0].contains("NewCMMath-Regular"), "{}", notes[0]);
}
