//! A `\text{...}` box that is italic only because the surrounding text is
//! (amsmath `\text` in an italic theorem body) gets no italic correction.
//! latex.ltx's `\check@icr` belongs to a text font command, and its
//! `\maybe@ic` fires only when the font outside that command is upright.
//!
//! Oracle (measured): TeX Live 2026 pdfTeX, `article` 11pt, T1, amsmath and
//! amsthm. `\showbox` of `\itshape$r\quad\text{and}\quad 0$` gives
//! `\glue 11.1051`, `\hbox(7.54149+0.0)x17.21259` holding just `a n d`, then
//! `\glue 11.1051` straight after the box: no `\kern` for the `d`. In the
//! corpus (lecture-notes p1, `a = qb + r \quad\text{and}\quad 0 \le r < b.`),
//! pdflatex puts `d` at 309.713 bp and `0` at 326.306 bp. With the italic
//! correction, render-pipeline set the `0` about 0.6 bp late.

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;

/// (source text, x, advance) of every glyph on page 1, in paint order, bp.
fn glyphs(src: &str) -> Vec<(String, f64, f64)> {
    let r = render_one(src);
    let mut out = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        if let Item::GlyphRun(run) = item {
            for g in &run.glyphs {
                let c = run
                    .clusters
                    .get(g.cluster as usize)
                    .map(|c| run.text[c.text_start_byte..c.text_end_byte].to_string())
                    .unwrap_or_default();
                out.push((c, g.origin_x.to_bp(), g.advance_x.to_bp()));
            }
        }
    }
    out
}

/// The space between the end of the `d` of `and` and the next glyph.
fn gap_after_and(body: &str) -> f64 {
    let src = format!(
        "\\documentclass[11pt]{{article}}\\usepackage[T1]{{fontenc}}\\usepackage{{amsmath,amsthm}}\\newtheorem{{theorem}}{{Theorem}}\\begin{{document}}{body}\\end{{document}}"
    );
    let gs = glyphs(&src);
    let i = (2..gs.len())
        .find(|&i| gs[i - 2].0 == "a" && gs[i - 1].0 == "n" && gs[i].0 == "d")
        .unwrap_or_else(|| panic!("no `and` in {gs:?}"));
    gs[i + 1].1 - (gs[i].1 + gs[i].2)
}

#[test]
fn an_inherited_italic_text_box_takes_no_italic_correction() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    // The same `\quad` after an upright `\text{and}` is the reference: only
    // the correction pdflatex does not add could make the italic one wider.
    let upright = gap_after_and("Text.\n\\[ a \\quad\\text{and}\\quad 0 \\]");
    let italic = gap_after_and("\\begin{theorem}Text.\n\\[ a \\quad\\text{and}\\quad 0 \\]\\end{theorem}");
    assert!((italic - upright).abs() < 0.1, "gap after the italic box {italic} bp vs upright {upright} bp");
}

#[test]
fn an_explicit_textit_in_upright_text_keeps_its_correction() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    // `\maybe@ic` after `\textit` with an upright outside adds `\/`.
    let plain = gap_after_and("Text.\n\\[ a \\quad\\text{and}\\quad 0 \\]");
    let corrected = gap_after_and("Text.\n\\[ a \\quad\\text{\\textit{and}}\\quad 0 \\]");
    assert!(corrected > plain + 0.3, "\\textit{{and}} keeps its italic correction: {corrected} vs {plain}");
}

/// Falsifier, not yet met: an explicit `\textit` whose *outside* is already
/// italic takes no correction either, because `\maybe@ic` only fires when
/// the font before the command is upright.
///
/// Oracle (measured): TeX Live 2026 pdfTeX, `article` 11pt, T1, amsmath.
/// `\showbox` of `\hbox{\itshape $r\quad\text{\textit{and}}\quad 0$}` gives
/// `\glue 11.1051`, `\hbox(7.54149+0.0)x17.21259` holding `a n d` and no
/// `\kern`, then `\glue 11.1051`: the same box as the inherited
/// `\text{and}` above.
///
/// The render pipeline adds the correction here: `convert_math_classed`'s
/// `TextRun` arm sees the slanting `\textit` in the atom's span but not the
/// face in force *outside* it, which the compiler knows
/// (`MathParser::text_base`, from 2e4aca2af) but does not put on the atom.
/// Meeting this needs that ambient face on `Nucleus::TextRun` (a compiler
/// change and re-pin), not a renderer-side re-read.
#[test]
#[ignore = "needs the ambient text face on the compiler's TextRun atom"]
fn an_explicit_textit_in_italic_text_takes_no_correction() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let upright = gap_after_and("Text.\n\\[ a \\quad\\text{and}\\quad 0 \\]");
    let nested = gap_after_and("\\begin{theorem}Text.\n\\[ a \\quad\\text{\\textit{and}}\\quad 0 \\]\\end{theorem}");
    assert!((nested - upright).abs() < 0.1, "gap after \\textit{{and}} in italic text {nested} bp vs upright {upright} bp");
}

/// The `TextRun` italic correction is decided from the control words inside
/// the atom's span, which the compiler folds into the pieces' style. In
/// italic text `\textit{and}` and `\textmd{and}` come out as the same
/// italic pieces, but only the slanting `\textit` takes the correction
/// here, so a same-length edit between them must not reuse the warm
/// cache's layout: the render cache keys a text atom on its source bytes
/// (`incremental::KeyContext`). Every warm render must equal a cold one,
/// in both directions and inline as well as in a display.
#[test]
fn a_warm_cache_follows_a_font_command_edit_inside_text() {
    use flashtex_compiler::parser::SourceDocument;
    use flashtex_render_pipeline::{render_cached, FontSet, RenderCache, RenderOptions};
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let run = |text: &str, cache: Option<&RenderCache>| -> Vec<(String, i64)> {
        let docs = [SourceDocument { path: "main.tex", text }];
        let r = render_cached(&docs, "main.tex", 1, "slant", &fonts, &RenderOptions::default(), cache);
        let mut out = Vec::new();
        for item in r.v2.pages[0].resident_items() {
            if let Item::GlyphRun(run) = item {
                for g in &run.glyphs {
                    let c = &run.clusters[g.cluster as usize];
                    out.push((run.text[c.text_start_byte..c.text_end_byte].to_string(), (g.origin_x.to_bp() * 1000.0).round() as i64));
                }
            }
        }
        out
    };
    let doc = |cmd: &str| {
        format!(
            "\\documentclass[11pt]{{article}}\\usepackage[T1]{{fontenc}}\\usepackage{{amsmath,amsthm}}\\newtheorem{{theorem}}{{Theorem}}\\begin{{document}}\
             \\begin{{theorem}}Text.\n\\[ a \\quad\\text{{\\{cmd}{{and}}}}\\quad 0 \\]\nand inline $b \\text{{\\{cmd}{{and}}}}\\, c$ too.\\end{{theorem}}\\end{{document}}"
        )
    };
    let (it, md) = (doc("textit"), doc("textmd"));
    assert_eq!(it.len(), md.len(), "the edit must keep every span where it was");
    let (cold_it, cold_md) = (run(&it, None), run(&md, None));
    assert_ne!(cold_it, cold_md, "the correction must differ between the two, or this tests nothing");
    let cache = RenderCache::new();
    for (i, (text, cold)) in [(&it, &cold_it), (&md, &cold_md), (&it, &cold_it)].into_iter().enumerate() {
        assert_eq!(&run(text, Some(&cache)), cold, "step {i}: the warm render must equal a cold one");
    }
}
