//! `\vec` paint-origin regression, plus unchanged-pins for every other accent.
//!
//! TeX's `\vec` is a *spacing* accent (cmmi `"7E`), but the pipeline paints
//! it as U+20D7, a Unicode *combining* mark whose Latin Modern Math ink sits
//! entirely left of its own origin (x in [-472, -56]/1000em, centre -264).
//! Painted at the TeX box's origin the arrow landed ~2/3em too far left even
//! though the layout pen was already exact against pdflatex `\showbox`
//! (-0.47847pt at 12pt). `math_items` therefore shifts the painted outline
//! by 0.6675em so the combining ink's centre lands on the cmmi10 arrow ink's
//! centre (403.5/1000em). Layout (pen, advance, hit-testing) is untouched.
//!
//! These tests pin that paint shift and the exactness of the layout pen
//! against the pdflatex oracle, and pin every other accent to paint exactly
//! at its TeX origin so the `\vec` arm can never leak onto them.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole, Tick, BP_PER_TEX_PT};

/// One v2 math glyph: cluster text, painted gid, painted origin, TeX box
/// origin (the cluster hit rect) and advance, all as ticks, plus the run's
/// font size.
#[derive(Debug)]
struct MGlyph {
    text: String,
    gid: u16,
    origin: Tick,
    hit: Tick,
    size: Tick,
}

fn math_glyphs(doc: &str) -> Vec<MGlyph> {
    if !lm_available() {
        return Vec::new();
    }
    let r = render_one(doc);
    let mut out = Vec::new();
    for page in &r.v2.pages {
        for item in page.resident_items() {
            let Item::GlyphRun(run) = item else { continue };
            if run.role != RunRole::Math {
                continue;
            }
            for g in &run.glyphs {
                let c = &run.clusters[g.cluster as usize];
                out.push(MGlyph {
                    text: run.text[c.text_start_byte..c.text_end_byte].to_string(),
                    gid: g.gid,
                    origin: g.origin_x,
                    hit: c.hit_rect.x,
                    size: run.font_size,
                });
            }
        }
    }
    out
}

fn accent_over_v(cmd: &str, text: &str) -> (MGlyph, MGlyph) {
    let doc = format!("\\begin{{document}}${cmd}{{v}}$\\end{{document}}");
    let mut g = math_glyphs(&doc);
    if g.is_empty() {
        panic!("fontless skip");
    }
    let i = g.iter().position(|x| x.text == text).unwrap_or_else(|| panic!("{cmd}: no {text:?} glyph in {g:?}"));
    let j = g.iter().position(|x| x.text == "v").unwrap_or_else(|| panic!("{cmd}: no base glyph in {g:?}"));
    (g.swap_remove(i), g.swap_remove(if j > i { j - 1 } else { j }))
}

#[test]
fn vec_paint_shift_centres_combining_ink_on_cmmi_ink() {
    if !lm_available() {
        return;
    }
    let (acc, base) = accent_over_v("\\vec", "\u{20D7}");
    // Latin Modern Math `uni20D7`, the combining mark `otf_glyph` paints for
    // the cmmi `"7E` accent slot (zero advance, ink left of origin).
    assert_eq!(acc.gid, 1817, "{acc:?}");
    // The default render is at 12pt, the size the pdflatex oracle below was
    // measured at; without this the pt comparisons that follow would silently
    // mis-scale if the default ever changed.
    assert_eq!(acc.size, Tick::from_tex_pt(12.0), "{acc:?}");
    assert_eq!(base.size, Tick::from_tex_pt(12.0), "{base:?}");
    // Layout pen, untouched by the fix: pdflatex `\showbox` (TeX Live 2026)
    // shifts the `\vec` accent box -0.47847pt from the base at 12pt.
    let pen_bp = acc.hit.to_bp() - base.hit.to_bp();
    let oracle_bp = -0.47847 * BP_PER_TEX_PT;
    assert!((pen_bp - oracle_bp).abs() < 2e-5, "pen {pen_bp}bp vs oracle {oracle_bp}bp");
    // Paint: combining ink centre (-264/1000em) onto cmmi10 arrow ink centre
    // (403.5/1000em): (403.5 + 264)/1000 = 0.6675em at size. Tick-exact up
    // to the two independent `from_tex_pt` roundings.
    let shift = acc.origin.0 - acc.hit.0;
    let want = Tick::from_tex_pt(0.6675 * 12.0).0 - Tick::from_tex_pt(0.0).0;
    assert!((shift - want).abs() <= 4, "paint shift {shift} ticks vs {want}");
    // The base letter is untouched: painted where TeX boxed it.
    assert_eq!(base.origin, base.hit, "{base:?}");
}

#[test]
fn other_accents_still_paint_at_their_tex_origin() {
    if !lm_available() {
        return;
    }
    // (command, cluster text, accent-box pen offset from the base in bp as
    // measured on the pre-fix tree: layout the fix must not move).
    let cases = [
        ("\\hat", "\u{02C6}", 0.442775),
        ("\\bar", "\u{00AF}", 0.442775),
        ("\\tilde", "\u{02DC}", 0.442775),
        ("\\dot", "\u{02D9}", 1.743440),
        ("\\ddot", "\u{00A8}", 0.442775),
        ("\\acute", "\u{00B4}", 0.442775),
        ("\\grave", "`", 0.442775),
        ("\\widehat", "\u{0302}", 0.601861),
        ("\\widetilde", "\u{0303}", 0.601861),
    ];
    for (cmd, text, pen_bp) in cases {
        let (acc, base) = accent_over_v(cmd, text);
        // No paint shift: the painted origin is the TeX box origin, exactly.
        assert_eq!(acc.origin, acc.hit, "{cmd}: {acc:?}");
        assert_eq!(base.origin, base.hit, "{cmd}: {base:?}");
        let pen = acc.hit.to_bp() - base.hit.to_bp();
        assert!((pen - pen_bp).abs() < 5e-6, "{cmd}: pen {pen}bp vs {pen_bp}bp");
    }
    // `\hat` additionally against the pdflatex oracle: `\showbox` shifts the
    // hat box +0.44444pt at 12pt, matched to 0.00000bp pre-fix.
    let (hat, base) = accent_over_v("\\hat", "\u{02C6}");
    let pen = hat.hit.to_bp() - base.hit.to_bp();
    let oracle = 0.44444 * BP_PER_TEX_PT;
    assert!((pen - oracle).abs() < 2e-5, "hat pen {pen}bp vs oracle {oracle}bp");
}

#[test]
fn overline_still_paints_a_rule_at_the_tex_origin() {
    if !lm_available() {
        return;
    }
    let r = render_one("\\begin{document}$\\overline{v}$\\end{document}");
    let mut rules = Vec::new();
    let mut base_hit = None;
    for page in &r.v2.pages {
        for item in page.resident_items() {
            match item {
                Item::GlyphRun(run) if run.role == RunRole::Math => {
                    for g in &run.glyphs {
                        let c = &run.clusters[g.cluster as usize];
                        if run.text[c.text_start_byte..c.text_end_byte] == *"v" {
                            base_hit = Some(c.hit_rect.x);
                        }
                    }
                }
                Item::Rule(rule) => rules.push((rule.x, rule.width)),
                _ => {}
            }
        }
    }
    let base_hit = base_hit.expect("base letter");
    assert_eq!(rules.len(), 1, "{rules:?}");
    // The vinculum starts at the TeX origin: no paint arm touches rules.
    assert_eq!(rules[0].0, base_hit, "{rules:?}");
}
