//! Text symbols inside `\texttt{...}` (and the other text families) against
//! pdflatex's `\showbox`, in OT1 (LaTeX's default) and under
//! `\usepackage[T1]{fontenc}`.
//!
//! * `\textbackslash`, `\{`, `\}`, `\textbar`, `\textless`, `\textgreater`:
//!   T1 declares them in the text font (`t1enc.def` 143-156, slots 92, 123,
//!   125, 124, 60, 62), so `\texttt{\textbackslash}` is the typewriter
//!   font's own glyph. OT1 has no slot for them, and the kernel default
//!   (`latex.ltx` 10046-10059) is `\UseTextSymbol{OMS}`/`{OML}`: the same
//!   family and series in the *math symbol* encoding, which `omscmr.fd`
//!   resolves to `cmsy` (`cmbsy` for `bx`) and every other family to the
//!   `OMS/cmsy/m/n` default after a "Font shape undefined" warning. `cmsy`'s
//!   `\` and braces are 0.5 em, `|` 0.2778 em, all `(0.75+0.25)` em tall;
//!   `cmtt`'s are 0.525 em. The pipeline used to set the typewriter glyph
//!   under both encodings.
//! * `\ldots`/`\dots`/`\textellipsis` in text: `.\kern\fontdimen3\font`
//!   three times in the current font (`latex.ltx` 10071), so three
//!   typewriter periods with zero kerns in `\texttt` (`cmtt` has no
//!   interword stretch), and three roman periods 1.66666 pt apart in `cmr`.
//!
//! Oracle: MacTeX 2026 `pdflatex` (TeX Live 2026), `\showbox` of `\hbox{...}`
//! at 10 pt in `article`, transcribed below in TeX points. pdflatex is never
//! in the product path.
//!
//! ```text
//! OT1                                     T1
//! \hbox{\texttt{a\textbackslash b}}       \hbox{\texttt{a\textbackslash b}}
//! \hbox(7.5+2.5)x15.49992                 \hbox(6.94275+0.83313)x15.74615
//! .\OT1/cmtt/m/n/10 a                     .\T1/cmtt/m/n/10 a
//! .\OMS/cmsy/m/n/10 n                     .\T1/cmtt/m/n/10 \
//! .\OT1/cmtt/m/n/10 b                     .\T1/cmtt/m/n/10 b
//!
//! \hbox{a\textbackslash b}                \hbox{a\textbackslash b}
//! \hbox(7.5+2.5)x15.5556                  \hbox(7.49817+2.49939)x15.55176
//! .\OT1/cmr/m/n/10 a                      .\T1/cmr/m/n/10 a
//! .\OMS/cmsy/m/n/10 n                     .\T1/cmr/m/n/10 \
//! .\OT1/cmr/m/n/10 b                      .\T1/cmr/m/n/10 b
//!
//! \hbox{\texttt{x\ldots y}}               \hbox{\texttt{x\ldots y}}
//! \hbox(4.30554+2.22223)x26.24977         \hbox(4.3045+2.22168)x26.24359
//! .\OT1/cmtt/m/n/10 x                     (x . . . y, \kern 0.0 after each .)
//! .\OT1/cmtt/m/n/10 .  \kern 0.0  (x3)
//! .\OT1/cmtt/m/n/10 y
//!
//! \hbox{x\ldots y}                        \hbox{x\ldots y}
//! \hbox(4.30554+1.94444)x23.88893         \hbox(4.3045+1.94397)x23.88306
//! .\OT1/cmr/m/n/10 x                      (\kern 1.66626 after each .)
//! .\OT1/cmr/m/n/10 .  \kern 1.66666 (x3)
//! .\OT1/cmr/m/n/10 y
//!
//! \hbox{\textbf{x\ldots y}}               \hbox{\textbf{x\ldots y}}
//! \hbox(4.44444+1.94444)x27.6318          \hbox(4.44336+1.94397)x27.6252
//! .\OT1/cmr/bx/n/10 x                     (\kern 1.9162 after each .)
//! .\OT1/cmr/bx/n/10 .  \kern 1.91666 (x3)
//! .\OT1/cmr/bx/n/10 y  \kern 0.15973
//!
//! OT1 only:
//! \hbox{\textbf{a\textbackslash b}}                   \hbox(7.5+2.5)x17.72905  (OMS/cmsy/b/n = cmbsy10, 5.75)
//! \hbox{\textsf{\textbf{a\textbackslash b}}}          \hbox(7.5+2.5)x15.86118  (OMS/cmss/bx/n undefined -> cmsy10, 5.0)
//! \hbox{\footnotesize a\textbackslash b}              \hbox(6.0+2.0)x13.22241  (cmsy8, 4.25)
//! \hbox{\texttt{\{a\}\textbar|<>\textless\textgreater}} \hbox(7.5+2.5)x49.33324
//!   OMS f, OT1/cmtt a, OMS g, OMS j, OT1/cmtt | < >, OML/cmm < >
//!   = 5 + 5.24995 + 5 + 2.77779 + 3 x 5.24995 + 2 x 7.77781
//! \hbox{\texttt{a\textbackslash{} b}}                  \hbox(7.5+2.5)x20.74988  (\glue 5.24995 after the OMS n)
//! ```

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};

/// TeX points to PostScript points (1 pt = 72/72.27 bp).
const BP: f64 = 72.0 / 72.27;

fn doc(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}{preamble}\\begin{{document}}\\noindent {body}\\end{{document}}")
}

/// `(origin x in bp, advance in bp, cluster text)` of every text glyph on
/// page 1 except the page number, in order.
fn glyphs(preamble: &str, body: &str) -> Vec<(f64, f64, String)> {
    let r = render_one(&doc(preamble, body));
    let mut out = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        if let Item::GlyphRun(run) = item {
            if run.role != RunRole::Text {
                continue;
            }
            for g in &run.glyphs {
                let c = &run.clusters[g.cluster as usize];
                out.push((g.origin_x.to_bp(), g.advance_x.to_bp(), run.text[c.text_start_byte as usize..c.text_end_byte as usize].to_string()));
            }
        }
    }
    // The page number sits on its own baseline far below; it is the last
    // run and always a single digit.
    out.truncate(out.len() - 1);
    out
}

/// Asserts the glyph advances (in TeX pt) of a body: every glyph the box
/// lists, in order, plus the box width as their sum.
fn assert_advances(preamble: &str, body: &str, want_pt: &[f64]) {
    let g = glyphs(preamble, body);
    let texts: Vec<&str> = g.iter().map(|(_, _, t)| t.as_str()).collect();
    assert_eq!(g.len(), want_pt.len(), "{preamble:?} {body}: glyphs {texts:?}");
    for (i, ((_, adv, t), want)) in g.iter().zip(want_pt).enumerate() {
        let got = adv / BP;
        assert!((got - want).abs() < 0.01, "{preamble:?} {body}: glyph {i} {t:?} advance {got:.5} pt, pdflatex {want:.5} pt");
    }
    // Consecutive origins: each glyph starts where the previous advance
    // ends (no stray kern or offset between the pieces of the word).
    for w in g.windows(2) {
        let (x0, adv0, _) = &w[0];
        let (x1, _, _) = &w[1];
        assert!(((x0 + adv0) - x1).abs() < 0.01, "{preamble:?} {body}: gap between glyphs: {:?}", g);
    }
    let width: f64 = g.iter().map(|(_, a, _)| a).sum::<f64>() / BP;
    let want_width: f64 = want_pt.iter().sum();
    assert!((width - want_width).abs() < 0.02, "{preamble:?} {body}: width {width:.5} pt, pdflatex {want_width:.5} pt");
}

const T1: &str = "\\usepackage[T1]{fontenc}";
const CMTT: f64 = 5.24995;
const ECTT: f64 = 5.24872;

#[test]
fn textbackslash_in_texttt_is_cmsy_under_ot1_and_cmtt_under_t1() {
    if !lm_available() {
        return;
    }
    // OT1: \OT1/cmtt a, \OMS/cmsy n (0.500002 em), \OT1/cmtt b = 15.49992 pt.
    assert_advances("", "\\texttt{a\\textbackslash b}", &[CMTT, 5.00002, CMTT]);
    // T1: three ectt1000 characters = 15.74615 pt.
    assert_advances(T1, "\\texttt{a\\textbackslash b}", &[ECTT, ECTT, ECTT]);
}

#[test]
fn textbackslash_in_roman() {
    if !lm_available() {
        return;
    }
    // OT1: cmr a 5.00002, cmsy 5.00002, cmr b 5.55557 = 15.5556.
    assert_advances("", "a\\textbackslash b", &[5.00002, 5.00002, 5.55557]);
    // T1: ecrm1000 a 4.99878, \ 4.99878, b 5.5542 = 15.55176.
    assert_advances(T1, "a\\textbackslash b", &[4.99878, 4.99878, 5.5542]);
}

#[test]
fn the_oms_symbol_keeps_the_family_and_series_only_for_roman_bold() {
    if !lm_available() {
        return;
    }
    // \textbf: OMS/cmr/bx/n is cmbsy10 (0.575 em): 5.59023 + 5.74997 + 6.38885 = 17.72905.
    assert_advances("", "\\textbf{a\\textbackslash b}", &[5.59023, 5.74997, 6.38885]);
    // \textsf{\textbf{..}}: OMS/cmss/bx/n is undefined, so cmsy10's 5.0:
    // 5.25003 + 5.00002 + 5.61113 = 15.86118.
    assert_advances("", "\\textsf{\\textbf{a\\textbackslash b}}", &[5.25003, 5.00002, 5.61113]);
    // \footnotesize: cmsy8 (0.531258 em at 8 pt): 4.25006 + 4.25006 + 4.72229 = 13.22241.
    assert_advances("", "{\\footnotesize a\\textbackslash b}", &[4.25006, 4.25006, 4.72229]);
}

#[test]
fn braces_bar_less_and_greater_in_texttt_under_ot1() {
    if !lm_available() {
        return;
    }
    // \OMS f (5.00002), \OT1/cmtt a, \OMS g (5.00002), \OMS j (2.77779), then the
    // typed | < > are cmtt's own ASCII slots (5.24995 each), and
    // \textless/\textgreater are \OML/cmm (cmmi10, 7.77781): 49.33324.
    assert_advances(
        "",
        "\\texttt{\\{a\\}\\textbar|<>\\textless\\textgreater}",
        &[5.00002, CMTT, 5.00002, 2.77779, CMTT, CMTT, CMTT, 7.77781, 7.77781],
    );
    // T1: every one of them is an ectt1000 character (47.23846 pt).
    assert_advances(T1, "\\texttt{\\{a\\}\\textbar|<>\\textless\\textgreater}", &[ECTT; 9]);
}

#[test]
fn the_space_after_textbackslash_is_the_typewriter_interword_glue() {
    if !lm_available() {
        return;
    }
    // a, OMS n (5.00002), \glue 5.24995 (cmtt's \fontdimen2, no stretch), b = 20.74988.
    let g = glyphs("", "\\texttt{a\\textbackslash{} b}");
    let texts: Vec<&str> = g.iter().map(|(_, _, t)| t.as_str()).collect();
    assert_eq!(texts, ["a", "\\", "b"], "{g:?}");
    let b_offset = (g[2].0 - g[0].0) / BP;
    assert!((b_offset - (CMTT + 5.00002 + CMTT)).abs() < 0.01, "b at {b_offset:.5} pt from a, pdflatex {:.5}", CMTT + 5.00002 + CMTT);
}

#[test]
fn ldots_in_texttt_is_three_typewriter_periods_with_zero_kerns() {
    if !lm_available() {
        return;
    }
    // OT1: x . . . y in cmtt, \kern 0.0 after each period = 26.24977.
    assert_advances("", "\\texttt{x\\ldots y}", &[CMTT; 5]);
    // T1: the same in ectt1000 = 26.24359.
    assert_advances(T1, "\\texttt{x\\ldots y}", &[ECTT; 5]);
}

#[test]
fn ldots_in_roman_and_bold_carries_fontdimen3_kerns() {
    if !lm_available() {
        return;
    }
    // cmr10: x 5.27779, . 2.77779 + \kern 1.66666 (x3), y 5.27779 = 23.88893.
    assert_advances("", "x\\ldots y", &[5.27779, 4.44445, 4.44445, 4.44445, 5.27779]);
    // ecrm1000: x 5.27649, . 2.7771 + \kern 1.66626, y 5.27649 = 23.88306.
    assert_advances(T1, "x\\ldots y", &[5.27649, 4.44336, 4.44336, 4.44336, 5.27649]);
    // cmbx10: x 6.06944, . 3.19446 + \kern 1.91666, y 6.06944 (the box's
    // trailing \kern 0.15973 is \textbf's italic correction, not a glyph).
    assert_advances("", "\\textbf{x\\ldots y}", &[6.06944, 5.11112, 5.11112, 5.11112, 6.06944]);
    // ecbx1000: x 6.06796, . 3.19366 + \kern 1.9162, y 6.06796 (+ \kern 0.15968).
    assert_advances(T1, "\\textbf{x\\ldots y}", &[6.06796, 5.10986, 5.10986, 5.10986, 6.06796]);
}

#[test]
fn verbatim_backslash_and_braces_stay_typewriter_characters() {
    if !lm_available() {
        return;
    }
    // \verb|\{a}| under OT1: \@noligs typewriter characters, no OMS symbol
    // (pdflatex: four \OT1/cmtt/m/n/10 characters, 20.99982 pt).
    assert_advances("", "\\verb|\\{a}|", &[CMTT; 4]);
}
