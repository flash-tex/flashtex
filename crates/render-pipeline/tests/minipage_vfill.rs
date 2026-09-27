//! `minipage` (latex.ltx `\@iiiminipage`, `\endminipage`, `\@iiiparbox`) and
//! vertical glue of infinite stretch (`\vfill`, `\vfil`, `\vspace{\fill}`)
//! on the page and inside a fixed-height box.
//!
//! Every expected number is a word origin or a rule read off the PDF that the
//! quoted probe produces under pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 pdflatex`, then
//! `tools/visual-oracle/pdftext.py` for the words and the content stream's
//! `re f`/`m l S` operators for the rules), in bp from the page's top-left
//! corner. pdflatex is an oracle only and never runs in the product path.
//!
//! What the probes pin:
//!
//! * the 21-242 proof-practice problem box: `\noindent\begin{minipage}[t]
//!   [2in][t]{\linewidth}` whose `\vfill` puts the ruled writing lines at the
//!   box's foot, two boxes a page with `\par\vspace{14pt}` between them, and
//!   a `\vfill{\small ...}\newpage` footer line on the text block's last
//!   baseline;
//! * `[t]`, `[b]` and `[c]` boxes side by side (`\hfill`), a box that starts
//!   an indented paragraph, one inside `center`, and one between words with
//!   a `\footnote` set at the box's own foot;
//! * the inner positions of a fixed-height box, `[t]`/`[b]`/`[c]`/`[s]`, and
//!   two `\vfil`s sharing its room;
//! * the page: `\vspace{\fill}` before `\newpage` takes the room from the
//!   `\newpage`'s own `\vfil`; a `\vfil` shares it with that `\vfil` (the
//!   line lands half-way); `\vfill` beats `\vfil` and two `\vfill`s share;
//! * a beamer frame: a `\vfill` shares the room with the frame's own fills.

mod common;

use common::*;

/// Positions agree with pdfTeX's to within rounding of the display list.
const TOL_BP: f64 = 0.02;

fn render(src: &str) -> (Vec<Word>, Vec<Vec<Rule>>, Vec<String>) {
    let r = render_one(src);
    let diags = r.v2.diagnostics.iter().map(|d| format!("{}: {}", d.code, d.message)).collect();
    (words_of(&r), rules_of(&r), diags)
}

/// `(x, baseline)` of the first run on `page` whose text starts with `text`
/// (`text@n`: the `n`th such run, top to bottom).
fn at(words: &[Word], page: u32, text: &str) -> (f64, f64) {
    let (text, nth) = match text.split_once('@') {
        Some((t, n)) => (t, n.parse::<usize>().expect("an occurrence number")),
        None => (text, 1),
    };
    let mut found: Vec<&Word> = words.iter().filter(|w| w.page == page && w.text.trim_start().starts_with(text)).collect();
    found.sort_by(|a, b| a.baseline.total_cmp(&b.baseline));
    found
        .get(nth - 1)
        .map(|w| (w.x, w.baseline))
        .unwrap_or_else(|| panic!("no run `{text}` #{nth} on page {page} in {:?}", words.iter().map(|w| (w.page, &w.text)).collect::<Vec<_>>()))
}

fn check(words: &[Word], expected: &[(u32, &str, f64, f64)]) {
    let mut bad = Vec::new();
    for &(page, text, x, y) in expected {
        let (gx, gy) = at(words, page, text);
        if (gx - x).abs() > TOL_BP || (gy - y).abs() > TOL_BP {
            bad.push(format!("p{page} `{text}`: ({gx:.3}, {gy:.3}), pdflatex ({x:.3}, {y:.3})"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

fn check_rules(got: &[Rule], want: &[Rule]) {
    let m = match_rules(want, got, TOL_BP);
    assert!(m.missing.is_empty() && m.extra.is_empty(), "missing {:?}, extra {:?} (got {got:?})", m.missing, m.extra);
}

fn no_minipage_or_vfill_diagnostic(diags: &[String]) {
    let bad: Vec<&String> = diags.iter().filter(|d| d.contains("minipage") || d.contains("vfill") || d.contains("linewidth") || d.contains("stretchable")).collect();
    assert!(bad.is_empty(), "{bad:?}");
}

/// The proof-practice problem box: `\vfill` in a `[t][2in][t]` box puts the
/// rules at its foot (the box is `2in` from its first line's top whatever
/// the body holds), the second box follows `\par\vspace{14pt}` and
/// `\parskip` below the first one's foot, and `\vfill{\small ...}\newpage`
/// sets the footer line on the text block's last baseline.
#[test]
fn a_fixed_height_top_box_puts_its_rules_at_the_foot() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass[11pt]{article}\n\\setlength{\\parindent}{0pt}\\setlength{\\parskip}{6pt}\n\\begin{document}\n\
Heading line.\\par\\vspace{9pt}\n\
\\noindent\\begin{minipage}[t][2in][t]{\\linewidth}\\textbf{Alpha} first line of the box.\\par Second paragraph of the box.\\par\\vfill\\noindent\\rule{\\linewidth}{.25pt}\\par\\noindent\\rule{\\linewidth}{.25pt}\\par\\end{minipage}\n\
\\par\\vspace{14pt}\n\
\\noindent\\begin{minipage}[t][2in][t]{\\linewidth}\\textbf{Bravo} next box.\\par\\vfill\\noindent\\rule{\\linewidth}{.25pt}\\par\\end{minipage}\n\
\\vfill{\\small Charlie footer line.}\n\\newpage\nDelta on page two.\n\\end{document}\n";
    let (words, rules, diags) = render(src);
    no_minipage_or_vfill_diagnostic(&diags);
    check(
        &words,
        &[
            (1, "Heading", 125.798, 140.742),
            (1, "Alpha", 125.798, 169.235),
            (1, "Second", 125.798, 182.785),
            (1, "Bravo", 125.798, 334.157),
            (1, "Charlie", 125.798, 669.161),
            (2, "Delta", 125.798, 140.742),
        ],
    );
    // `\rule{\linewidth}{.25pt}`: the box's `\linewidth`, i.e. `\textwidth`
    // here; pdfTeX strokes it as a 0.249bp line.
    check_rules(
        &rules[0],
        &[(125.798, 291.861, 358.655, 0.249), (125.798, 305.411, 358.655, 0.249), (125.798, 470.332, 358.655, 0.249)],
    );
}

/// `[t]` boxes align their first baselines, `[b]` its last, `[c]` centres on
/// the math axis; a box that starts a paragraph without `\noindent` is
/// indented; `center` centres the box; and a box between words keeps the
/// words on its line, its `\footnote` set at the box's foot under a
/// `\footnoterule` of the box (`\@mpfootnotetext`, the mark in italic `a`).
#[test]
fn boxes_on_a_line_align_by_their_position() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass{article}\n\\begin{document}\nBefore text line.\n\n\
\\noindent\\begin{minipage}[t]{0.45\\linewidth}\nAlpha bravo charlie delta echo foxtrot golf hotel india juliet.\n\\end{minipage}\\hfill\n\
\\begin{minipage}[t]{0.45\\linewidth}\nKilo lima.\n\nMike november.\n\\end{minipage}\n\nOscar papa.\n\n\
\\noindent\\begin{minipage}[b]{0.3\\linewidth}\nQuebec romeo sierra tango uniform victor whiskey.\n\\end{minipage}\\hfill\n\
\\begin{minipage}{0.3\\linewidth}\nXray yankee zulu one two three four five six seven eight nine.\n\\end{minipage}\\hfill\n\
\\begin{minipage}[t]{0.3\\linewidth}\nTen eleven.\n\\end{minipage}\n\n\
\\begin{minipage}{200pt}\nTwelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen.\n\\end{minipage}\n\n\
\\begin{center}\n\\begin{minipage}{100pt}\nTwenty twentyone twentytwo.\n\\end{minipage}\n\\end{center}\nFinal line.\n\n\
Text before \\begin{minipage}[t]{120pt}\nInside\\footnote{Noted.} words here and more.\n\\end{minipage} text after.\n\\end{document}\n";
    let (words, rules, diags) = render(src);
    no_minipage_or_vfill_diagnostic(&diags);
    check(
        &words,
        &[
            (1, "Before", 148.712, 134.765),
            (1, "Alpha", 133.768, 146.720),
            (1, "trot", 133.768, 158.675),
            (1, "Kilo", 322.810, 146.720),
            (1, "Mike", 322.810, 158.675),
            (1, "Oscar", 148.712, 168.416),
            (1, "Quebec", 133.768, 178.268),
            (1, "whiskey.", 133.768, 202.178),
            (1, "Xray", 254.067, 190.223),
            (1, "seven", 254.067, 214.134),
            (1, "Ten", 374.365, 202.178),
            (1, "Twelve", 148.712, 223.986),
            (1, "teen", 148.712, 235.941),
            (1, "Twenty", 255.811, 255.645),
            (1, "twentytwo.", 255.811, 267.600),
            (1, "Final", 133.768, 287.414),
            (1, "Text", 148.712, 299.370),
            (1, "Inside", 201.872, 299.370),
            (1, "more.", 201.874, 311.325),
            (1, "Noted.", 217.117, 326.916),
            (1, "text@2", 324.747, 299.370),
        ],
    );
    check_rules(&rules[0], &[(201.874, 317.303, 47.820, 0.398)]);
}

/// A fixed-height box's inner position: `[t]` (`\\vss` below the body) with
/// a `\\vfill` that beats it, `[b]` and `[c]`, `[s]` whose own `\\vfill`
/// stretches, `[c][h]` (the inner position defaults to the outer), and two
/// `\\vfil`s sharing a `[t]` box's room with its `\\vss`. Then the page:
/// `\\vspace{\\fill}` before `\\newpage` takes all of the page's room from
/// the `\\newpage`'s `\\vfil`, a `\\vfil` shares it with that `\\vfil`, two
/// `\\vfill`s share it and leave a `\\vfil` nothing, and `\\stretch{1}` and
/// `\\stretch{2}` share it one to two.
#[test]
fn fixed_height_boxes_and_pages_share_their_room_by_glue_order() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass{article}\n\
\\begin{document}\n\
\\noindent\\begin{minipage}[t][1.5in][t]{0.3\\linewidth}\n\
Alpha top.\n\
\\vfill\n\
Alpha bottom.\n\
\\end{minipage}\\hfill\n\
\\begin{minipage}[t][1.5in][b]{0.3\\linewidth}\n\
Bravo text.\n\
\\end{minipage}\\hfill\n\
\\begin{minipage}[t][1.5in][c]{0.3\\linewidth}\n\
Charlie text.\n\
\\end{minipage}\n\
\\par\\vspace{14pt}\n\
\\noindent\\begin{minipage}[b][1in][s]{0.3\\linewidth}\n\
Delta top.\n\
\\vfill\n\
Delta bottom.\n\
\\end{minipage}\\hfill\n\
\\begin{minipage}[c][1in]{0.3\\linewidth}\n\
Echo text.\n\
\\end{minipage}\\hfill\n\
\\begin{minipage}[t][1in][t]{0.3\\linewidth}\n\
Foxtrot top.\\par\n\
\\vfil\n\
Foxtrot mid.\\par\n\
\\vfil\n\
Foxtrot end.\n\
\\end{minipage}\n\
\\par\n\
After the boxes.\\par\n\
\\vspace{\\fill}\n\
Bottom of page one.\n\
\\newpage\n\
Page two top.\n\
\\vfil\n\
Golf middle.\n\
\\newpage\n\
Page three.\n\
\\vfill\n\
\\vfill\n\
Hotel thirds.\n\
\\vfil\n\
India unmoved.\n\
\\newpage\n\
Page four.\\par\n\
\\vspace{\\stretch{1}}\n\
Juliet third.\\par\n\
\\vspace{\\stretch{2}}\n\
Kilo last.\n\
\\end{document}\n\
";
    let (words, _, diags) = render(src);
    no_minipage_or_vfill_diagnostic(&diags);
    check(
        &words,
        &[
            (1, "Alpha@1", 133.768, 134.765),
            (1, "Alpha@2", 133.768, 233.909),
            (1, "Bravo", 254.067, 242.765),
            (1, "Charlie", 374.365, 192.224),
            (1, "Delta@1", 133.768, 264.627),
            (1, "Delta@2", 133.768, 329.709),
            (1, "Echo", 254.067, 330.677),
            (1, "Foxtrot@1", 374.365, 329.709),
            (1, "Foxtrot@2", 374.365, 355.424),
            (1, "Foxtrot@3", 374.365, 381.140),
            (1, "After", 148.712, 402.816),
            (1, "Bottom", 148.712, 672.747),
            (2, "Page", 148.712, 134.765),
            (2, "Golf", 148.712, 409.719),
            (3, "Hotel", 148.712, 660.792),
            (3, "India", 148.712, 672.747),
            (4, "Juliet", 148.712, 318.077),
            (4, "Kilo", 148.712, 672.747),
        ],
    );
}

/// A `\\vfill` in a beamer frame joins the frame's own `plus 1fill` glue:
/// in a `[c]` frame it takes a third of the room (the frame's top and
/// bottom fills take the rest), in a `[t]` frame it shares the room with
/// the frame's bottom fill and a trailing `\\vfill`, and a trailing
/// `\\vfill` alone moves the body up by sharing with the bottom fill.
#[test]
fn a_vfill_in_a_beamer_frame_shares_the_frame_fills() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass{beamer}\n\\begin{document}\n\\begin{frame}{Title one}\nAlpha top.\n\n\\vfill\nBravo bottom.\n\\end{frame}\n\\begin{frame}[t]{Title two}\nCharlie top.\n\\vfill\nDelta.\n\\vfill\n\\end{frame}\n\\begin{frame}{Title three}\nEcho only.\n\\vfill\n\\end{frame}\n\\end{document}\n";
    let (words, _, diags) = render(src);
    no_minipage_or_vfill_diagnostic(&diags);
    check(
        &words,
        &[
            (1, "Alpha", 28.346, 98.696),
            (1, "Bravo", 28.346, 174.603),
            (2, "Charlie", 28.346, 42.006),
            (2, "Delta.", 28.346, 126.417),
            (3, "Echo", 28.346, 101.961),
        ],
    );
}
