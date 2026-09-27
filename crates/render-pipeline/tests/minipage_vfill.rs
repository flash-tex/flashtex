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
//! * a beamer frame: a `\vfill` shares the room with the frame's own fills;
//! * `\linewidth` in a list or `quote` (the list's line, not `\textwidth`),
//!   the inner position a height falls back to, and footnotes across a
//!   nested minipage (labels and where LaTeX's global `\@mpfootins` puts
//!   them);
//! * the top of a box: `\vspace`, `\vskip`, `\bigskip`, `\vspace*` are
//!   kept, a list's or environment's `\addvspace` is not;
//! * glue with a natural width and an infinite stretch at a page end
//!   (`\vskip 12pt plus 1fill\newpage`, `\vspace{12pt plus 1fill}`).

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

/// Whether a run reading exactly `text` sits at `(x, y)` on page 1.
fn has_run(words: &[Word], text: &str, x: f64, y: f64) -> bool {
    words.iter().any(|w| w.page == 1 && w.text.trim() == text && (w.x - x).abs() <= TOL_BP && (w.baseline - y).abs() <= TOL_BP)
}

/// `\linewidth` is the list's line: `\textwidth` less every enclosing
/// list's `\leftmargin` (and `quote`'s `\rightmargin`), as `\@listI` and
/// `\list` set it -- so a `minipage{\linewidth}` in an item wraps at the
/// item's width, `0.5\linewidth` in a nested `enumerate` is half the
/// nested line, and `\rule{\linewidth}` in a `quote` or an item is that
/// line's width (293.898 bp = 295pt, 318.804 bp = 320pt at 10pt).
#[test]
fn linewidth_in_a_list_or_quote_is_the_list_line() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass{article}\n\
\\begin{document}\n\
Lead line.\n\
\\begin{itemize}\n\
\\item \\begin{minipage}[t]{\\linewidth}Alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo sierra.\\end{minipage}\n\
\\item Plain.\n\
\\begin{enumerate}\n\
\\item \\begin{minipage}[t]{0.5\\linewidth}Tango uniform victor whiskey xray yankee zulu one two three four five six.\\end{minipage}\\hfill X\\rule{\\linewidth}{0pt}\n\
\\end{enumerate}\n\
\\end{itemize}\n\
\\begin{quote}\n\
\\begin{minipage}{\\linewidth}Seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty.\\end{minipage}\n\
\\end{quote}\n\
\\begin{quote}\n\
Quoted words \\rule{\\linewidth}{1pt}\n\
\\end{quote}\n\
\\begin{itemize}\\item\\rule{\\linewidth}{1pt}\\end{itemize}\n\
\\end{document}\n\
";
    let (words, rules, diags) = render(src);
    no_minipage_or_vfill_diagnostic(&diags);
    check(
        &words,
        &[
            (1, "Lead", 148.712, 134.765),
            (1, "Alpha", 158.676, 154.690),
            (1, "november", 158.675, 166.645),
            (1, "Plain.", 158.676, 184.467),
            (1, "Tango", 180.593, 204.392),
            (1, "yankee", 180.593, 216.348),
            (1, "six.", 180.593, 228.303),
            (1, "X", 329.036, 204.392),
            (1, "Seven", 158.675, 246.180),
            (1, "seventeen", 158.675, 258.135),
            (1, "Quoted", 158.675, 277.950),
        ],
    );
    check_rules(&rules[0], &[(220.471, 276.954, 293.898, 0.996), (158.675, 298.872, 318.804, 0.996)]);
}

/// Fixed-height boxes with the inner position left to its default: `[t]`,
/// `[b]`, `[c]` pass their own letter on, an unknown `[x]` is centred
/// (`\bm@c`, "Unexpected alignment x"), `[s]` stretches its `\vfill`; and
/// explicit inner `[x]` (centred), `[l]` (as `t`), `[r]` (as `b`).
#[test]
fn the_inner_position_of_a_fixed_height_box_follows_latex() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass{article}\n\
\\begin{document}\n\
\\noindent Ref\\begin{minipage}[t][1in]{50pt}Aa\\end{minipage}%\n\
\\begin{minipage}[b][1in]{50pt}Bb\\end{minipage}%\n\
\\begin{minipage}[c][1in]{50pt}Cc\\end{minipage}%\n\
\\begin{minipage}[x][1in]{50pt}Dd\\end{minipage}%\n\
\\begin{minipage}[s][1in]{50pt}Ee\\par\\vfill Ff\\end{minipage}\n\
\n\
\\bigskip\n\
\\noindent Ref\\begin{minipage}[t][1in][x]{50pt}Gg\\end{minipage}%\n\
\\begin{minipage}[t][1in][l]{50pt}Hh\\end{minipage}%\n\
\\begin{minipage}[t][1in][r]{50pt}Ii\\end{minipage}%\n\
\\begin{minipage}[b][1in][c]{50pt}Jj\\end{minipage}%\n\
\\begin{minipage}[x][1in][b]{50pt}Kk\\end{minipage}\n\
\\end{document}\n\
";
    let (words, _, diags) = render(src);
    assert_eq!(diags.iter().filter(|d| d.contains("Unexpected alignment x")).count(), 2, "{diags:?}");
    check(
        &words,
        &[
            (1, "Bb", 198.386, 196.802),
            (1, "Cc", 248.200, 197.715),
            (1, "Dd", 298.014, 197.771),
            (1, "Ee", 347.827, 165.119),
            (1, "Ff", 347.827, 230.311),
            (1, "Gg", 148.574, 385.381),
            (1, "Hh", 198.387, 346.946),
            (1, "Ii", 248.200, 418.946),
            (1, "Jj", 298.014, 313.381),
            (1, "Kk", 347.827, 380.455),
        ],
    );
}

/// Footnotes across a nested minipage, as pdflatex sets them (it warns
/// "Nested minipage: footnotes may be misplaced"): `\@mpfootins` is global,
/// so the notes the outer box has collected are set under the inner box
/// with the inner box's own (widening it to the outer measure), and the
/// outer count goes on from the inner one (`Dd` is `b`) unless the inner
/// box stepped nothing (`Hh` is `c`). Notes follow each other with no
/// interline glue (each `\@mpfootnotetext` is a fresh `\vbox`).
#[test]
fn footnotes_across_a_nested_minipage_follow_latex() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass{article}\n\
\\begin{document}\n\
\\noindent\\begin{minipage}{3in}\n\
Aa\\footnote{One.} Bb\\footnote{Two.}\n\
\\begin{minipage}{1in}Cc\\footnote{Three.}\\end{minipage}\n\
Dd\\footnote{Four.}\n\
\\end{minipage}\n\
\n\
\\bigskip\n\
\\noindent\\begin{minipage}{3in}\n\
Ee\\footnote{Five.} Ff\\footnote{Six.}\n\
\\begin{minipage}{1in}Gg without notes.\\end{minipage}\n\
Hh\\footnote{Seven.}\n\
\\end{minipage}\n\
\\end{document}\n\
";
    let (words, rules, diags) = render(src);
    assert!(diags.iter().any(|d| d.contains("nested minipage")), "{diags:?}");
    check(
        &words,
        &[
            (1, "Aa", 133.768, 149.546),
            (1, "Cc", 172.241, 131.610),
            (1, "One.", 187.484, 147.201),
            (1, "Two.", 187.484, 157.005),
            (1, "Three.", 187.484, 166.469),
            (1, "Dd", 133.768, 178.763),
            (1, "Four.", 149.011, 194.693),
            (1, "Gg", 167.951, 217.403),
            (1, "Five.", 183.194, 244.949),
            (1, "Six.", 183.194, 254.752),
            (1, "Hh", 133.768, 265.507),
            (1, "Seven.", 149.011, 281.098),
        ],
    );
    for (text, x, y) in [("b", 146.914, 175.148), ("b", 145.259, 191.880), ("c", 146.775, 261.891), ("c", 145.259, 278.285), ("a", 183.380, 163.656)] {
        assert!(has_run(&words, text, x, y), "no mark `{text}` at ({x}, {y})");
    }
    check_rules(
        &rules[0],
        &[(172.241, 137.588, 28.800, 0.398), (133.768, 184.741, 86.399, 0.398), (167.951, 235.336, 28.800, 0.398), (133.768, 271.484, 86.399, 0.398)],
    );
}

/// The top of a minipage: `\@setminipage` makes `\addvspace` do nothing
/// until the first paragraph starts, so a list's or `center`'s `\@topsep`
/// is dropped -- but `\vspace`, `\vskip`, `\bigskip` and `\vspace*` are
/// `\vskip`s and stay. In a `[t]` box that glue is the list's first item, so
/// the box's height is 0 and the first baseline sits the skip plus its
/// height below the line (pdflatex: `\vspace{10pt}` 16.83pt, `\vskip 5pt`
/// 11.83pt, `\bigskip` 18.83pt, `\vspace*{7pt}` 13.83pt).
#[test]
fn a_skip_at_the_top_of_a_minipage_stays_and_an_addvspace_goes() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass{article}\n\
\\begin{document}\n\
\\noindent Ref \\begin{minipage}[t]{80pt}\\vspace{10pt}Aa\\end{minipage}\n\
\\begin{minipage}[t]{80pt}\\vskip 5pt Bb\\end{minipage}\n\
\\begin{minipage}[t]{80pt}\\begin{itemize}\\item Cc\\end{itemize}\\end{minipage}\n\
\\begin{minipage}[t]{80pt}\\addvspace{20pt}Dd\\end{minipage}\n\
\n\
\\bigskip\n\
\\noindent Ref \\begin{minipage}[t]{80pt}Ee\\par\\vspace{10pt}Ff\\end{minipage}\n\
\\begin{minipage}[t]{80pt}\\bigskip Gg\\end{minipage}\n\
\\begin{minipage}[t]{80pt}\\begin{center}Hh\\end{center}\\end{minipage}\n\
\\begin{minipage}[t]{80pt}\\vspace*{7pt}Ii\\end{minipage}\n\
\\end{document}\n\
";
    let (words, _, _) = render(src);
    check(
        &words,
        &[
            (1, "Ref@1", 133.768, 134.765),
            (1, "Aa", 151.099, 151.535),
            (1, "Bb", 233.326, 146.664),
            (1, "Cc", 340.459, 134.765),
            (1, "Dd", 397.779, 134.765),
            (1, "Ref@2", 133.768, 171.405),
            (1, "Ee", 151.104, 171.405),
            (1, "Ff", 151.099, 193.323),
            (1, "Gg", 233.326, 190.168),
            (1, "Hh", 348.899, 171.405),
            (1, "Ii", 397.778, 185.187),
        ],
    );
}

/// Glue with a natural width and an infinite stretch is one node, as TeX
/// keeps it: `\vskip 12pt plus 1fill\newpage` on a page with less than
/// 12pt left, `\vspace{12pt plus 1fill}` where the page breaks and where
/// it does not, and `30pt plus 1fill` beside `20pt plus 2fill` sharing a
/// page after their natural widths (the second is `filll`: see below).
#[test]
fn finite_plus_infinite_glue_at_a_page_end_follows_latex() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass{article}\n\
\\setlength{\\parindent}{0pt}\n\
\\begin{document}\n\
Top one.\\par\n\
\\rule{1pt}{520pt}\\par\n\
Last one.\\par\n\
\\vskip 12pt plus 1fill\n\
\\newpage\n\
Top two.\\par\n\
\\rule{1pt}{520pt}\\par\n\
Last two.\\par\n\
\\vspace{12pt plus 1fill}\n\
After two.\\par\n\
\\rule{1pt}{500pt}\\par\n\
Last three.\\par\n\
\\vspace{12pt plus 1fill}\n\
After three.\n\
\\newpage\n\
Top four.\\par\n\
\\vspace{30pt plus 1fill}\n\
Mid four.\\par\n\
\\vskip 20pt plus 2fill\n\
Last four.\n\
\\end{document}\n\
";
    let (words, _, diags) = render(src);
    no_minipage_or_vfill_diagnostic(&diags);
    check(
        &words,
        &[
            (1, "Top", 133.768, 134.765),
            (1, "Last", 133.768, 667.711),
            (2, "Top", 133.768, 134.765),
            (2, "Last", 133.768, 667.711),
            (3, "After@1", 133.768, 134.765),
            (3, "Last", 133.768, 645.848),
            (3, "After@2", 133.768, 672.747),
            (4, "Top", 133.768, 134.765),
            (4, "Mid", 133.768, 176.608),
            // `\vskip 20pt plus 2fill` then `Last`: TeX's `fil` loop reads
            // the `L` as a fourth `l` (a `filll` glue), in pdflatex as here.
            (4, "ast", 133.768, 672.747),
        ],
    );
}
