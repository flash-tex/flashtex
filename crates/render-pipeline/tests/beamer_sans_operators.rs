//! beamer's sans-serif operator names (owner report 2026-10-02): under the
//! default font theme `beamerbasefont.sty` sets the `operators` symbol font
//! to `OT1/cmss/m/n` (`\SetSymbolFont{operators}{normal}{OT1}{cmss}...`), so
//! the body of `\operatorname{...}` and of a `\DeclareMathOperator` command
//! is sans like `\ker`, `\sin` and `\log` -- while `\mathrm` keeps
//! `\rmdefault` (`\SetMathAlphabet{\mathrm}{normal}{..}{\rmdefault}`).
//! `professionalfonts` and the `serif` font theme leave the kernel's roman
//! `operators` font in place.
//!
//! Oracle: pdflatex 3.141592653-2.6-1.40.29 (TeX Live 2026) on the decks
//! below, read back with `tools/visual-oracle/pdftext.py` (bp from the
//! paper's top-left corner; glyph origins). The same constructs are the
//! corpus fixtures `fixtures/real-world/beamer-sans-operators{,-professional,
//! -serif}`. On `origin/main` the sans deck set `rref` and `tr` in
//! LMRoman10 and every operator body lacked its last character's italic
//! correction (`\operatorname{rref}(` 1.08 bp short in sans, 0.85 bp in
//! roman: pdfTeX's box dump has `f \kern0.7604` / `f \kern0.85167`).
//!
//! Needs the bundled Latin Modern faces and their metrics
//! (`FLASHTEX_FONT_DIRS=apps/mac/Fonts`, `FLASHTEX_TFM_DIRS` at TeX Live's
//! `lm`/`ec`/`amsfonts/symbols` TFMs), like every oracle test in this crate.

mod common;

use common::{lm_available, render_one};
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::Rendered;

const BODY: &str = concat!(
    "\\DeclareMathOperator{\\rref}{rref}\n",
    "\\begin{document}\n\\begin{frame}\n",
    "$\\ker(\\operatorname{rref}(A)) = \\ker(\\rref(A))$ and $\\operatorname{tr}(B)$ and ",
    "$\\mathrm{d}x$ and $\\sin x + \\log 2$ and $\\operatorname*{arg\\,max}_x f$.\n",
    "\\end{frame}\n\\end{document}\n",
);

fn deck(theme: &str) -> String {
    format!("\\documentclass{{beamer}}\n{theme}{BODY}")
}

/// Every non-space glyph on page 1 in drawing order: (char, PostScript name
/// of its face, origin x in bp).
fn glyphs(r: &Rendered) -> Vec<(char, String, f64)> {
    let mut out = Vec::new();
    let page = r.v2.pages.iter().find(|p| p.number == 1).expect("page 1");
    for it in page.resident_items() {
        if let Item::GlyphRun(run) = it {
            let face =
                r.v2.fonts
                    .iter()
                    .find(|f| *f.font_id == *run.font_id)
                    .map(|f| f.postscript_name.clone())
                    .unwrap_or_default();
            let chars: Vec<char> = run.text.chars().filter(|c| !c.is_whitespace()).collect();
            assert_eq!(
                chars.len(),
                run.glyphs.len(),
                "one glyph per character in {:?}",
                run.text
            );
            for (c, g) in chars.into_iter().zip(&run.glyphs) {
                out.push((c, face.clone(), g.origin_x.to_bp()));
            }
        }
    }
    out
}

/// The oracle's 62 glyphs: `ker(rref(A))=ker(rref(A))andtr(B)anddxandsinx+log2and`
/// on the first line, `argmaxxf.` on the second.
const TEXT: &str = "ker(rref(A))=ker(rref(A))andtr(B)anddxandsinx+log2andargmaxxf.";

/// Asserts the face of glyphs `range` and that glyph `i` sits at `x` (bp).
fn check(g: &[(char, String, f64)], faces: &[(std::ops::Range<usize>, &str)], xs: &[(usize, f64)]) {
    let text: String = g.iter().map(|(c, _, _)| c).collect();
    assert_eq!(text, TEXT);
    for (range, face) in faces {
        for i in range.clone() {
            assert_eq!(
                g[i].1,
                *face,
                "glyph {i} {:?} of {:?}",
                g[i].0,
                &TEXT[range.clone()]
            );
        }
    }
    for &(i, x) in xs {
        assert!(
            (g[i].2 - x).abs() <= 0.05,
            "glyph {i} {:?}: ours {:.3}, pdflatex {x:.3}",
            g[i].0,
            g[i].2
        );
    }
}

#[test]
fn default_theme_sets_operator_names_sans_and_mathrm_roman() {
    if !lm_available() {
        return;
    }
    let g = glyphs(&render_one(&deck("")));
    check(
        &g,
        &[
            (0..9, "LMSans10-Regular"),    // ker(rref(  -- \ker, \operatorname
            (9..10, "LMSans10-Oblique"),   // A
            (13..21, "LMSans10-Regular"),  // ker(rref   -- \DeclareMathOperator
            (28..31, "LMSans10-Regular"),  // tr(
            (36..37, "LMRoman10-Regular"), // d          -- \mathrm stays \rmdefault
            (41..44, "LMSans10-Regular"),  // sin
            (46..49, "LMSans10-Regular"),  // log
            (53..59, "LMSans10-Regular"),  // argmax     -- \operatorname*
        ],
        // The `(` after each operator body: its last character's italic
        // correction (pdfTeX `\kern0.7604` after `rref`).
        &[
            (8, 62.746),
            (21, 131.675),
            (30, 183.288),
            (55, 37.011),
            (56, 44.440),
        ],
    );
}

#[test]
fn professionalfonts_keeps_roman_operator_names() {
    if !lm_available() {
        return;
    }
    let g = glyphs(&render_one(&deck("\\usefonttheme{professionalfonts}\n")));
    check(
        &g,
        &[
            (0..9, "LMRoman10-Regular"),
            (13..21, "LMRoman10-Regular"),
            (28..31, "LMRoman10-Regular"),
            (53..59, "LMRoman10-Regular"),
        ],
        // `\kern0.85167` after the roman `rref`.
        &[
            (8, 64.741),
            (21, 136.585),
            (30, 189.803),
            (59, 85.953),
            (60, 93.036),
        ],
    );
}

#[test]
fn serif_theme_keeps_roman_operator_names() {
    if !lm_available() {
        return;
    }
    let g = glyphs(&render_one(&deck("\\usefonttheme{serif}\n")));
    check(
        &g,
        &[
            (0..9, "LMRoman10-Regular"),
            (13..21, "LMRoman10-Regular"),
            (25..28, "LMRoman10-Regular"),
            (53..59, "LMRoman10-Regular"),
        ],
        &[
            (8, 64.741),
            (21, 136.585),
            (30, 190.863),
            (59, 87.013),
            (60, 94.096),
        ],
    );
}

/// What an `\operatorname` argument already holds is not an operator-font
/// run: `\text{...}` and `\mbox{...}` are hboxes (no italic correction),
/// `\mathrm{...}` is the `\rmdefault` alphabet (roman even under beamer's
/// sans math). `\operatorname{f-f}`'s hyphen is an `operators` character
/// (amsopn `\newmcodes@`), so the whole run is sans under beamer.
const PROBES: [&str; 7] = [
    "\\operatorname{\\text{abf}}",
    "\\operatorname{a\\text{bf}}",
    "\\operatorname{\\mbox{abf}}",
    "\\operatorname{\\mathrm{rref}}",
    "\\operatorname{\\mathrm{d}}",
    "\\operatorname{f-f}",
    "\\operatorname{rref}",
];

/// x (bp) of each probe line's `Z`, set 20pt after the formula.
fn probe_zs(head: &str, tail: &str) -> Vec<(f64, Vec<(char, String, f64)>)> {
    let lines: String = PROBES
        .iter()
        .map(|p| format!("\\noindent${p}$\\hspace{{20pt}}Z\\par\n"))
        .collect();
    let g = glyphs(&render_one(&format!("{head}{lines}{tail}")));
    let mut out = Vec::new();
    let mut line = Vec::new();
    for gl in g {
        if gl.0 == 'Z' {
            out.push((gl.2, std::mem::take(&mut line)));
        } else {
            line.push(gl);
        }
    }
    out
}

/// Each `(probe index, Z x in bp, face of the formula's glyphs)` against
/// the render; every mismatch is reported at once.
fn check_probes(zs: &[(f64, Vec<(char, String, f64)>)], expect: &[(usize, f64, &str)]) {
    assert_eq!(zs.len(), PROBES.len());
    let mut bad = Vec::new();
    for &(i, x, face) in expect {
        let (z, line) = &zs[i];
        let fs: Vec<&str> = line.iter().map(|g| g.1.as_str()).collect();
        if (z - x).abs() > 0.05 || !fs.iter().all(|f| *f == face) {
            bad.push(format!(
                "{}: Z ours {z:.3}, expected {x:.3}; faces {fs:?}, expected {face}",
                PROBES[i]
            ));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn article_operator_arguments_keep_their_own_treatment() {
    if !lm_available() {
        return;
    }
    let zs = probe_zs(
        "\\documentclass{article}\n\\usepackage{amsmath}\n\\pagestyle{empty}\n\\begin{document}\n",
        "\\end{document}\n",
    );
    // pdflatex, every one.
    let oracle = [167.254, 167.254, 167.254, 169.747, 159.228, 163.88, 169.747];
    let expect: Vec<(usize, f64, &str)> = oracle
        .iter()
        .enumerate()
        .map(|(i, &x)| (i, x, "LMRoman10-Regular"))
        .collect();
    check_probes(&zs, &expect);
}

#[test]
fn beamer_operator_arguments_keep_their_own_treatment() {
    if !lm_available() {
        return;
    }
    let zs = probe_zs(
        "\\documentclass{beamer}\n\\setbeamertemplate{navigation symbols}{}\n\\begin{document}\n\\begin{frame}\n",
        "\\end{frame}\n\\end{document}\n",
    );
    check_probes(
        &zs,
        &[
            // pdflatex.
            (3, 65.845, "LMRoman10-Regular"),
            (4, 54.327, "LMRoman10-Regular"),
            (5, 59.333, "LMSans10-Regular"),
            (6, 64.667, "LMSans10-Regular"),
            // Not pdflatex's (62.479, CMSS10): a known gap older than this
            // test, in which math `\text`/`\mbox` under beamer take the roman
            // family-0 font instead of the sans text font. Pinned so the hbox
            // never gains an operator run's italic correction here.
            (0, 63.120, "LMRoman10-Regular"),
            (2, 63.120, "LMRoman10-Regular"),
        ],
    );
}
