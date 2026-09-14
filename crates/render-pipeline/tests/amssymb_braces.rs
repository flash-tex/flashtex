//! amssymb/amsfonts symbols set from the msam/msbm TFMs, and `\overbrace`,
//! `\underbrace`, the amsmath over/under arrows and the growing wide accents
//! (compiler `MathAtom.ams_symbol`, `Frame::OverBrace`.., math-layout `ams`,
//! `Nucleus::Brace`/`OverArrow`/`MeasuredAccent`). Positions against pdfLaTeX
//! are pinned by `crates/compiler/tests/amssymb_corpus/oracle.py`; these tests
//! pin the mechanism.

mod common;

use common::*;
use flashtex_math_layout::ams::{self, AmsFont};
use flashtex_math_layout::FontId;
use flashtex_render_pipeline::display::{GlyphRun, Item};
use flashtex_render_pipeline::Rendered;

fn doc(body: &str) -> String {
    format!("\\documentclass[12pt]{{article}}\\usepackage{{amsmath,amssymb}}\\begin{{document}}{body}\\end{{document}}")
}

fn runs(r: &Rendered) -> Vec<GlyphRun> {
    r.v2.pages[0]
        .to_items()
        .into_iter()
        .filter_map(|i| if let Item::GlyphRun(run) = i { Some(run) } else { None })
        .collect()
}

/// `(cluster text, gid, advance pt, font size pt, cluster index)` of every glyph.
fn glyphs(r: &Rendered) -> Vec<(String, u16, f64, f64, u32)> {
    let mut out = Vec::new();
    for run in runs(r) {
        for g in &run.glyphs {
            let c = &run.clusters[g.cluster as usize];
            out.push((
                run.text[c.text_start_byte..c.text_end_byte].to_string(),
                g.gid,
                g.advance_x.to_bp() * 72.27 / 72.0,
                run.font_size.to_bp() * 72.27 / 72.0,
                g.cluster,
            ));
        }
    }
    out
}

fn problems(r: &Rendered) -> Vec<String> {
    r.v2
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "math_limitation" | "math_glyph_unmapped") || d.message.contains("not supported in math"))
        .map(|d| format!("{}: {}", d.code, d.message))
        .collect()
}

#[test]
fn amssymb_relations_advance_by_their_msam_msbm_widths() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(&doc("$a \\leqslant b \\nleq c \\blacktriangleright d$"));
    assert!(problems(&r).is_empty(), "{:?}", problems(&r));
    let gs = glyphs(&r);
    for (text, font, slot) in [("⩽", AmsFont::Msam, 0x36u8), ("≰", AmsFont::Msbm, 0x02), ("▶", AmsFont::Msam, 0x49)] {
        let g = gs.iter().find(|g| g.0 == text).unwrap_or_else(|| panic!("{text} painted: {gs:?}"));
        let tfm = ams::glyph(font, slot, 'x', g.3, FontId(0)).expect("slot");
        assert!((g.2 - tfm.width).abs() < 0.01, "{text}: advance {} vs {:?} TFM {}", g.2, font, tfm.width);
    }
}

#[test]
fn negated_amssymb_relations_overlay_the_solidus_in_one_cluster() {
    if !lm_available() {
        return;
    }
    let r = render_one(&doc("$a \\nleqslant b$"));
    assert!(problems(&r).is_empty(), "{:?}", problems(&r));
    let gs = glyphs(&r);
    let negated: Vec<_> = gs.iter().filter(|g| g.0 == "⩽\u{0338}").collect();
    assert_eq!(negated.len(), 2, "base and overlay share the cluster: {gs:?}");
    assert_eq!(negated[0].4, negated[1].4);
    assert_ne!(negated[0].1, negated[1].1);
}

#[test]
fn overbrace_paints_assembly_parts_and_leader_rules_with_limits() {
    if !lm_available() {
        return;
    }
    let r = render_one(&doc("$\\overbrace{a+b+c+d}^{n} \\underbrace{x+y}_{k}$"));
    assert!(problems(&r).is_empty(), "{:?}", problems(&r));
    let gs = glyphs(&r);
    // Left end, middle, right end: cmex's second middle piece paints nothing.
    assert_eq!(gs.iter().filter(|g| g.0 == "⏞").count(), 3, "{gs:?}");
    assert_eq!(gs.iter().filter(|g| g.0 == "⏟").count(), 3, "{gs:?}");
    let rules = r.v2.pages[0].to_items().iter().filter(|i| matches!(i, Item::Rule(_))).count();
    assert!(rules >= 4, "two \\leaders\\vrule fills per brace, got {rules}");
    // The limits are script size.
    assert!(gs.iter().any(|g| g.0 == "n" && g.3 < 9.0), "{gs:?}");
}

#[test]
fn widehat_grows_along_the_cmex_chain_and_msbm_past_two_ems() {
    if !lm_available() {
        return;
    }
    let r = render_one(&doc("$\\widehat{x} \\widehat{xyz} \\widehat{abcdefg} \\hat{x}$"));
    assert!(problems(&r).is_empty(), "{:?}", problems(&r));
    let gs = glyphs(&r);
    let hats: Vec<_> = gs.iter().filter(|g| g.0 == "\u{0302}").collect();
    assert_eq!(hats.len(), 3, "{gs:?}");
    assert!(hats[0].2 < hats[1].2 && hats[1].2 < hats[2].2, "wider bodies take wider accents: {hats:?}");
    assert!(gs.iter().any(|g| g.0 == "\u{02C6}"), "\\hat is the roman accent: {gs:?}");
}

#[test]
fn amsmath_over_arrows_set_without_limitations() {
    if !lm_available() {
        return;
    }
    let r = render_one(&doc("$\\overrightarrow{AB} \\overleftarrow{AB} \\underleftrightarrow{ABCD}$"));
    assert!(problems(&r).is_empty(), "{:?}", problems(&r));
    let gs = glyphs(&r);
    assert_eq!(gs.iter().filter(|g| g.0 == "→").count(), 2, "{gs:?}");
    assert_eq!(gs.iter().filter(|g| g.0 == "←").count(), 2, "{gs:?}");
}
