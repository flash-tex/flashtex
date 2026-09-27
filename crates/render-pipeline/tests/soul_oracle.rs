//! GH-828: soul `\so` and `\hl` against pdfTeX.
//!
//! Oracle: MacTeX 2026 pdflatex (`SOURCE_DATE_EPOCH=0`, two runs), 12pt
//! article; glyph origins from `tools/visual-oracle/pdftext.py` and fill
//! rectangles replayed from the reference content stream (overlapping
//! fills on one baseline merged into bands). Committed evidence; pdflatex
//! never runs here.
//!
//! - `\so` keeps the font's lig/kern effect between two letters before its
//!   `.25em` letterskip (`\SOUL@getkern`): `V` 10.401 bp after `A`, not
//!   the 11.702 bp of the letterskip alone.
//! - `\so` words break at their hyphenation points (soul's explicit
//!   discretionaries), and soul's own glue stretches (`.55em plus.275em
//!   minus.183em` outside): a long letterspaced word near a line's end was
//!   an overfull line, and a justified line's slack went to the wrong glue.
//! - `\hl` fills 1.75ex above the baseline, 0.25pt past each end and
//!   through the interword spaces of one command within a line (one band,
//!   not a patch per word), and breaks words at hyphenation points with the
//!   hyphen highlighted.
//! - `\noindent` survives a paragraph that starts with `\so`, `\mbox` or a
//!   footnote (the nested parse used to spend it: 17.559 bp of indent).

mod common;

use common::{assert_pdftex_glyphs, lm_available, render_one, rules_of};

const TOL: f64 = 0.02;

fn soul_doc(packages: &str, body: &str) -> String {
    format!("\\documentclass[12pt]{{article}}\n\\usepackage{{{packages}}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

#[test]
fn so_keeps_the_font_kern_before_the_letterskip() {
    if !lm_available() {
        return;
    }
    let source = soul_doc("soul", "\\noindent AVATAR WAVY To\\par\n\\noindent\\so{AVATAR WAVY To}\\par\n\\noindent\\so{office} x\\par");
    assert_pdftex_glyphs(
        &source,
        &[
            ("A", "CMR12", 110.854, 152.199), ("V", "CMR12", 121.255, 152.199), ("A", "CMR12", 131.656, 152.199), ("T", "CMR12", 142.380, 152.199),
            ("A", "CMR12", 152.783, 152.199), ("R", "CMR12", 164.487, 152.199), ("W", "CMR12", 180.718, 152.199), ("A", "CMR12", 194.371, 152.199),
            ("V", "CMR12", 204.772, 152.199), ("Y", "CMR12", 216.464, 152.199), ("T", "CMR12", 232.854, 152.199), ("o", "CMR12", 243.258, 152.199),
            // `ff` and `fi` are not ligatures in `\so`, but the ligature's
            // width difference counts as the kern (`\wd\hbox{ff}` minus
            // `\wd\hbox{f\null f}`), so the second `f` overlaps the first.
            ("o", "CMR12", 110.854, 166.645), ("f", "CMR12", 119.636, 166.645), ("f", "CMR12", 125.808, 166.645), ("i", "CMR12", 131.991, 166.645),
            ("c", "CMR12", 138.172, 166.645), ("e", "CMR12", 146.292, 166.645), ("x", "CMR12", 157.938, 166.645),
        ],
        TOL,
    );
}

#[test]
fn so_word_breaks_at_a_hyphenation_point() {
    if !lm_available() {
        return;
    }
    let source = soul_doc("soul", "the quick brown  and then the letterspaced \\so{incomprehensibilities} arrives here now at last.");
    assert_pdftex_glyphs(
        &source,
        &[
            ("b", "CMR12", 467.537, 137.753), ("i", "CMR12", 476.957, 137.753), ("l", "CMR12", 483.138, 137.753), ("i", "CMR12", 489.319, 137.753),
            ("-", "CMR12", 495.500, 137.753),
            ("t", "CMR12", 110.854, 152.199), ("i", "CMR12", 118.336, 152.199), ("e", "CMR12", 124.516, 152.199), ("s", "CMR12", 132.636, 152.199),
            ("a", "CMR12", 143.697, 152.199), ("r", "CMR12", 149.551, 152.199), ("r", "CMR12", 154.103, 152.199), ("i", "CMR12", 158.656, 152.199),
        ],
        TOL,
    );
}

#[test]
fn so_edge_space_stretches_like_soul() {
    if !lm_available() {
        return;
    }
    let source = soul_doc(
        "soul",
        "This paragraph has ordinary words before the letterspaced phrase so that\n\\so{internationalization} lands near the end of a line and must break.",
    );
    assert_pdftex_glyphs(
        &source,
        &[
            ("i", "CMR12", 110.854, 152.199), ("z", "CMR12", 220.440, 152.199), ("n", "CMR12", 259.775, 152.199),
            ("l", "CMR12", 272.603, 152.199), ("a", "CMR12", 275.855, 152.199), ("n", "CMR12", 281.708, 152.199), ("d", "CMR12", 288.212, 152.199),
            ("s", "CMR12", 294.715, 152.199), ("n", "CMR12", 303.170, 152.199), ("r", "CMR12", 320.730, 152.199), ("t", "CMR12", 329.108, 152.199),
        ],
        TOL,
    );
}

#[test]
fn noindent_reaches_a_paragraph_starting_with_a_box() {
    if !lm_available() {
        return;
    }
    let source = soul_doc(
        "color,soul",
        "\\noindent\\so{AB} x\\par\n\\noindent \\so{AB} x\\par\n\\noindent\\mbox{AB} x\\par\n\\noindent\\hl{AB} x\\par\n\\noindent AB\\footnote{x} x\\par",
    );
    let r = render_one(&source);
    let mut firsts: Vec<(f64, f64)> = Vec::new();
    for (text, x, y) in common::painted_glyphs(&r) {
        if y > 500.0 || text == "1" {
            continue; // the footnote, its mark and the page number
        }
        match firsts.iter_mut().find(|(fy, _)| (*fy - y).abs() < 0.05) {
            Some((_, fx)) => *fx = fx.min(x),
            None => firsts.push((y, x)),
        }
    }
    assert_eq!(firsts.len(), 5, "{firsts:?}");
    for (y, x) in firsts {
        assert!((x - 110.854).abs() < TOL, "line at {y:.3} starts at {x:.3}, pdflatex 110.854 (no indent)");
    }
}

/// The page's rules merged into bands: overlapping or touching rules with
/// the same top join, as the eye (and pdfTeX's overlapping leaders) sees
/// them. `(x0, top, x1, bottom)` in bp.
fn bands(r: &flashtex_render_pipeline::Rendered) -> Vec<(f64, f64, f64, f64)> {
    let mut rules: Vec<(f64, f64, f64, f64)> = rules_of(r)[0].iter().map(|&(x, top, w, h)| (top, x, x + w, top + h)).collect();
    rules.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut out: Vec<(f64, f64, f64, f64)> = Vec::new();
    for (top, x0, x1, bottom) in rules {
        match out.last_mut() {
            Some(band) if (band.1 - top).abs() < 0.01 && x0 <= band.2 + 0.01 => band.2 = band.2.max(x1),
            _ => out.push((x0, top, x1, bottom)),
        }
    }
    out
}

fn assert_bands(source: &str, expected: &[(f64, f64, f64, f64)]) {
    let r = render_one(source);
    let got = bands(&r);
    let close = |a: &(f64, f64, f64, f64), b: &(f64, f64, f64, f64)| {
        (a.0 - b.0).abs() <= TOL && (a.1 - b.1).abs() <= TOL && (a.2 - b.2).abs() <= TOL && (a.3 - b.3).abs() <= TOL
    };
    assert!(
        got.len() == expected.len() && got.iter().zip(expected).all(|(g, e)| close(g, e)),
        "highlight bands\n  got      {got:?}\n  pdflatex {expected:?}"
    );
}

#[test]
fn hl_fills_one_band_through_the_interword_spaces() {
    if !lm_available() {
        return;
    }
    assert_bands(
        &soul_doc("color,soul", "Some text before \\hl{highlighted words here now} and after."),
        &[(218.886, 128.746, 359.596, 141.614)],
    );
    assert_bands(
        &soul_doc(
            "color,soul",
            "Here is a sentence that runs for a while before the highlight starts so\n\\hl{this highlighted phrase must wrap onto the next line of text} and\ncontinues after it.\n\nTwo separate \\hl{alpha} \\hl{beta} commands and \\hl{gamma}\\hl{delta} here.",
        ),
        &[
            (480.224, 128.746, 499.647, 141.614),
            (110.605, 143.192, 397.373, 156.060),
            // Two commands with a space between: two bands.
            (200.741, 172.083, 229.204, 184.951),
            (232.608, 172.083, 255.543, 184.951),
            (339.652, 172.083, 402.582, 184.951),
        ],
    );
}

#[test]
fn hl_word_breaks_at_a_hyphenation_point() {
    if !lm_available() {
        return;
    }
    // soul's discretionaries are explicit, so TeX's first pass already
    // breaks at `i-ties` (demerits 7700) instead of setting the whole word
    // shrunk (b=83, 8749); a plain word would take the latter.
    let source = soul_doc(
        "color,soul",
        "the quick brown fox jumps  and then the letterspaced \\hl{incomprehensibilities} arrives here now at last.",
    );
    assert_pdftex_glyphs(
        &source,
        &[
            ("b", "CMR12", 479.237, 137.753), ("i", "CMR12", 485.741, 137.753), ("l", "CMR12", 488.992, 137.753), ("i", "CMR12", 492.244, 137.753),
            ("-", "CMR12", 495.495, 137.753),
            ("t", "CMR12", 110.854, 152.199), ("i", "CMR12", 115.407, 152.199), ("e", "CMR12", 118.658, 152.199), ("s", "CMR12", 123.861, 152.199),
            ("a", "CMR12", 132.376, 152.199), ("r", "CMR12", 138.229, 152.199), ("r", "CMR12", 142.782, 152.199), ("i", "CMR12", 147.334, 152.199),
        ],
        TOL,
    );
    // The hyphen is highlighted too, and the fill stops at the line's end.
    assert_bands(&source, &[(406.086, 128.746, 499.646, 141.614), (110.605, 143.192, 128.727, 156.060)]);
}

#[test]
fn hl_takes_its_space_and_extents_from_the_highlighted_face() {
    if !lm_available() {
        return;
    }
    // 10pt: the space inside `\hl` is the bold/small/Large face's, and the
    // fill is 1.75ex above and 0.75ex below in that face (the body face at
    // the paragraph's size made `\small`'s fill 0.32 bp too deep and moved
    // every later line).
    let source = "\\documentclass[10pt]{article}\n\\usepackage{color,soul}\n\\begin{document}\n\\noindent A {\\bfseries\\hl{Wavy To}} x\\par\n\\noindent A {\\small\\hl{Wavy To}} x\\par\n\\noindent A {\\Large\\hl{Wavy To}} x\\par\n\\noindent A \\textbf{\\hl{Wavy To}} x\\par\n\\end{document}\n";
    assert_pdftex_glyphs(
        source,
        &[
            ("T", "CMBX10", 176.614, 0.0), ("x", "CMR10", 192.684, 0.0),
            ("T", "CMR9", 170.414, 0.0), ("x", "CMR10", 184.234, 0.0),
            ("T", "CMR17", 183.966, 0.0), ("x", "CMR10", 203.287, 0.0),
        ],
        TOL,
    );
    assert_bands(
        source,
        &[
            (144.312, 127.016, 189.607, 138.086),
            (144.312, 139.964, 181.158, 149.615),
            (144.312, 150.612, 200.213, 166.054),
            (144.312, 167.049, 189.607, 178.119),
        ],
    );
}
