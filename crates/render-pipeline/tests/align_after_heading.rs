//! An `align` that opens the paragraph after a heading sets no line above
//! itself.
//!
//! TeX §1145: a display whose paragraph's horizontal list is still empty
//! contributes no line. After a heading LaTeX's `\@afterheading` has put
//! `\everypar{{\setbox\z@\lastbox}...}` in place, which takes the
//! `\parindent` box straight back off, so the list really is empty and only
//! `\parskip` precedes the display. After ordinary text there *is* a line —
//! the indent box on its own — and it carries its own interline glue.
//!
//! `build_paragraph`'s `ParaPart::Display` arm already modelled this. Its
//! `ParaPart::Rows` arm — every amsmath alignment — did not, and pushed the
//! opener line unconditionally, so an `align` right after a `\section` sat
//! one phantom empty line too low.
//!
//! ## Oracle
//!
//! pdfTeX 3.141592653-2.6-1.40.27 (TeX Live 2025), `\showoutput`,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, 11 pt article. After ordinary
//! text:
//!
//! ```text
//! ...\hbox(7.54149+2.12863)x469.75502     % "Lead paragraph, ..."
//! ...\glue(\parskip) 0.0 plus 1.0
//! ...\glue(\parskip) 0.0
//! ...\glue(\baselineskip) 11.47137
//! ...\hbox(0.0+0.0)x469.75502, glue set 452.75502fil   % the indent box alone
//! ...\penalty 10000
//! ...\glue(\abovedisplayskip) 11.0 plus 3.0 minus 6.0
//! ...\glue -3.0                            % \vskip -\lineskiplimit (\openup\jot)
//! ...\glue 0.0                             % \vskip \normallineskiplimit
//! ...\glue(\lineskip) 4.0                  % \lineskip + \jot: the row is tall
//! ...\hbox(18.22697+14.49814)x279.67776, display
//! ```
//!
//! and after `\section*{A heading}` the same alignment gets
//!
//! ```text
//! ...\hbox(9.93758+2.7993)x469.75502       % the heading
//! ...\penalty 10000
//! ...\glue 10.84085 plus 0.94266           % \@startsection's after-skip
//! ...\glue(\parskip) 0.0 plus 1.0
//! ...\glue(\parskip) 0.0
//! ...\penalty 10000
//! ...\glue(\abovedisplayskip) 11.0 plus 3.0 minus 6.0
//! ...\glue -3.0
//! ...\glue 0.0
//! ...\glue(\lineskip) 4.0
//! ...\hbox(18.22697+14.49814)x279.67776, display
//! ```
//!
//! — no `\hbox(0.0+0.0)` and no `\glue(\baselineskip)` at all. Heading
//! baseline to first row baseline is therefore
//! `2.7993 + 10.84085 + 11.0 - 3.0 + 0.0 + 4.0 + 18.22697 = 43.86712 pt`.
//! With the phantom line it is one `\baselineskip` minus the heading's depth
//! further down: 13.6 - 2.7993 = 10.8007 pt, or the full 13.6 pt under a
//! heading with no descender.
//!
//! pdflatex is an oracle only and never runs in the product path.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// 1 bp = 1.00375 TeX pt.
const BP: f64 = 1.00375;
const TOL: f64 = 0.5;

/// The same `align*` twice: once after ordinary text, once after a heading.
/// Its rows are the identical two lines, so the two placements are directly
/// comparable and only the material above them differs.
const PROBE: &str = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\usepackage[margin=1in]{geometry}
\usepackage{amsmath,amssymb}
\pagestyle{empty}
\begin{document}
Lead paragraph, then an align after ordinary text.

\begin{align*}
  \sum_{k=0}^{n} k &= \frac{n(n+1)}{2}, \\
  \sum_{k=0}^{\infty} x^k &= \frac{1}{1-x}.
\end{align*}
Middle paragraph between the two alignments.

\section*{A heading}
\begin{align*}
  \sum_{k=0}^{n} k &= \frac{n(n+1)}{2}, \\
  \sum_{k=0}^{\infty} x^k &= \frac{1}{1-x}.
\end{align*}
Tail paragraph.
\end{document}
";

/// The baseline of every `=` on page 1, in bp from the page top, top first.
///
/// Eight of them: each alignment row's relation sign, on the row's own
/// baseline, and the `k=0` under each row's `\sum`, one line lower. Sorted
/// top-first they interleave as row 1, its limit, row 2, its limit — so
/// `[0]` and `[4]` are the first row of each alignment, which is what the
/// placement above the alignment decides. Reading a row baseline this way
/// avoids the sum's *upper* limit, which is raised and would otherwise be
/// the topmost thing in the block.
fn equals_baselines(text: &str) -> Vec<f64> {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "align-after-heading", &fonts, &RenderOptions::default());
    assert_eq!(r.v2.pages.len(), 1, "expected a one-page document");
    let mut ys = Vec::new();
    for item in &r.v2.pages[0].items {
        let Item::GlyphRun(run) = item else { continue };
        let mut chars = run.clusters.iter().map(|c| {
            run.text[c.text_start_byte as usize..c.text_end_byte as usize].chars().next()
        });
        for g in &run.glyphs {
            if chars.next().flatten() == Some('=') {
                ys.push(g.baseline_y.to_bp());
            }
        }
    }
    ys.sort_by(f64::total_cmp);
    ys
}

/// The baseline of the first glyph of the run that starts with `word`.
fn baseline_of(text: &str, word: &str) -> f64 {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "align-after-heading", &fonts, &RenderOptions::default());
    for item in &r.v2.pages[0].items {
        let Item::GlyphRun(run) = item else { continue };
        if run.text.trim_start().starts_with(word) {
            if let Some(g) = run.glyphs.first() {
                return g.baseline_y.to_bp();
            }
        }
    }
    panic!("no glyph run starting `{word}` on page 1");
}

#[test]
fn an_alignment_after_a_heading_has_no_line_above_it() {
    if !common::lm_available() {
        eprintln!("SKIP an_alignment_after_a_heading_has_no_line_above_it: Latin Modern not installed");
        return;
    }
    let rows = equals_baselines(PROBE);
    assert_eq!(rows.len(), 8, "expected two `=` per alignment row, got {rows:?}");
    let heading = baseline_of(PROBE, "A");
    let row = rows[4];
    // 2.7993 (the heading's depth: `A heading` has a descender)
    //  + 10.84085 (\@startsection's after-skip)
    //  + 11.0 - 3.0 + 0.0 (\abovedisplayskip, \openup\jot's lineskiplimit)
    //  + 4.0 (\lineskip + \jot)
    //  + 18.22697 (the row's height)
    let expected = (2.7993 + 10.84085 + 11.0 - 3.0 + 0.0 + 4.0 + 18.22697) / BP;
    let got = row - heading;
    assert!(
        (got - expected).abs() <= TOL,
        "heading to first alignment row: {got:.4} bp, pdflatex {expected:.4} bp (off by {:+.4} bp). \
         A whole phantom line is {:.3} bp here (\\baselineskip 13.6 less the heading's 2.7993 pt \
         of depth), so a miss of that size means the alignment still gets an opener line that \
         `\\@afterheading` had already emptied.",
        got - expected,
        (13.6 - 2.7993) / BP
    );
}

/// The control: after ordinary text the line *is* there, and this placement
/// was already right — the fix must not take it away.
#[test]
fn an_alignment_after_ordinary_text_keeps_its_line() {
    if !common::lm_available() {
        eprintln!("SKIP an_alignment_after_ordinary_text_keeps_its_line: Latin Modern not installed");
        return;
    }
    let rows = equals_baselines(PROBE);
    assert_eq!(rows.len(), 8, "expected two `=` per alignment row, got {rows:?}");
    let lead = baseline_of(PROBE, "Lead");
    let row = rows[0];
    // 2.12863 (the lead line's depth)
    //  + 0.0 + 0.0 (\parskip)
    //  + 11.47137 (\baselineskip onto the empty indent-box line, height 0)
    //  + 0.0 (that line's own height and depth)
    //  + 11.0 - 3.0 + 0.0 + 4.0 + 18.22697
    let expected = (2.12863 + 11.47137 + 11.0 - 3.0 + 0.0 + 4.0 + 18.22697) / BP;
    let got = row - lead;
    assert!(
        (got - expected).abs() <= TOL,
        "ordinary text to first alignment row: {got:.4} bp, pdflatex {expected:.4} bp \
         (off by {:+.4} bp)",
        got - expected
    );
}
