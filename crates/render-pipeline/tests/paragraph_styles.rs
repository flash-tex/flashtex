//! `center`/`flushleft`/`flushright`/`quote` paragraphs and `\hrule` in the
//! page builder: the line-setting skips LaTeX uses (`\centering` =
//! `\leftskip`/`\rightskip` `0pt plus 1fil`, `quote` = a level-1 list with
//! both margins at `\leftmargini`), the `\topsep`(+`\partopsep`) glue around
//! the environment, and a full-measure 0.4pt rule with no interline glue.
//! Every number is checked against pdflatex's article.cls values.

mod common;

use common::*;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::v1::{Capabilities, V1Item, V1Payload};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

#[derive(Debug, Clone)]
struct Word {
    page: u32,
    text: String,
    x: f64,
    baseline: f64,
    width: f64,
}

fn layout(text: &str) -> (V1Payload, Vec<Word>) {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
    let v1 = v1_of(&r, Capabilities { rules: true, font_hints: true, ..Capabilities::default() });
    assert_ne!(v1.status, "failed", "{:?}", v1.diagnostics);
    // Word widths from the v2 display list's runs (v1 text items carry none).
    let mut words = Vec::new();
    for page in &r.v2.pages {
        for it in page.resident_items() {
            if let flashtex_render_pipeline::display::Item::GlyphRun(run) = it {
                let Some(first) = run.glyphs.first() else { continue };
                let last = run.glyphs.last().expect("non-empty");
                words.push(Word {
                    page: page.number,
                    text: run.text.clone(),
                    x: first.origin_x.to_bp(),
                    baseline: first.baseline_y.to_bp(),
                    width: (last.origin_x.0 + last.advance_x.0 - first.origin_x.0) as f64 / flashtex_render_pipeline::display::TICKS_PER_BP,
                });
            }
        }
    }
    (v1, words)
}

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words.iter().find(|w| w.text == text).unwrap_or_else(|| panic!("no word {text:?} in {words:?}"))
}

/// TeX points to PDF points, the unit of every v1/v2 coordinate.
fn bp(pt: f64) -> f64 {
    pt * 72.0 / 72.27
}

/// 11pt article, US Letter, no geometry: the text area's left edge and
/// `\textwidth` (360pt) in bp, from the stylesheet the renderer uses;
/// `\baselineskip` 13.6pt, `\topsep` 9pt, `\partopsep` 3pt and
/// `\leftmargini` 2.5em = 27.5pt are article.cls's size11.clo values.
fn measure() -> (f64, f64) {
    let s = flashtex_render_pipeline::Stylesheet::article(11, flashtex_render_pipeline::fonts::Family::LatinModern, None);
    assert_eq!(s.text_width_pt, 360.0);
    (bp(s.text_x_pt), bp(s.text_width_pt))
}

/// `\leftmargini` = 2.5em of the 11pt body font (quad 10.95003pt).
fn leftmargini() -> f64 {
    let s = flashtex_render_pipeline::Stylesheet::article(11, flashtex_render_pipeline::fonts::Family::LatinModern, None);
    assert!((s.leftmargini_pt - 2.5 * 10.95003).abs() < 1e-6, "{}", s.leftmargini_pt);
    bp(s.leftmargini_pt)
}

#[test]
fn center_lines_are_centred_in_the_measure_and_carry_topsep() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let src = "\\documentclass[11pt]{article}\\begin{document}\nBefore.\n\n\\begin{center}\nShort line\\\\\nOne\n\\end{center}\n\nAfter.\n\\end{document}";
    let (v1, words) = layout(src);
    let (text_x, text_w) = measure();
    assert!(
        !v1.diagnostics.iter().any(|d| d.code == "unsupported_block"),
        "center is set, not reported: {:?}",
        v1.diagnostics
    );
    let short = word(&words, "Short");
    let line_w = word(&words, "line").x + word(&words, "line").width - short.x;
    let centre = short.x + line_w / 2.0;
    assert!((centre - (text_x + text_w / 2.0)).abs() < 0.05, "first line centred: {centre} vs {}", text_x + text_w / 2.0);
    // `\\` inside `center` is `\@centercr` (a `\par`): the line after it is
    // centred too, not pushed left by `\hfil` glue.
    let one = word(&words, "One");
    let one_centre = one.x + one.width / 2.0;
    assert!((one_centre - (text_x + text_w / 2.0)).abs() < 0.05, "line after \\\\ centred: {one_centre}");
    // \topsep 9pt + \partopsep 3pt (the environment opens in vertical mode
    // after a blank line) above and below, on top of \baselineskip 13.6pt.
    let before = word(&words, "Before.");
    let after = word(&words, "After.");
    assert!((short.baseline - before.baseline - bp(13.6 + 12.0)).abs() < 0.05, "gap above: {}", short.baseline - before.baseline);
    assert!((after.baseline - one.baseline - bp(13.6 + 12.0)).abs() < 0.05, "gap below: {}", after.baseline - one.baseline);
}

#[test]
fn flushright_and_flushleft_set_ragged_lines() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let src = "\\documentclass[11pt]{article}\\begin{document}\n\\begin{flushright}\nRight edge\n\\end{flushright}\n\\begin{flushleft}\nLeft edge\n\\end{flushleft}\n\\end{document}";
    let (_v1, words) = layout(src);
    let (text_x, text_w) = measure();
    let edge = word(&words, "edge");
    assert!((edge.x + edge.width - (text_x + text_w)).abs() < 0.05, "flushright ends at the right margin: {}", edge.x + edge.width);
    let left = word(&words, "Left");
    assert!((left.x - text_x).abs() < 0.05, "flushleft starts at the left margin: {}", left.x);
}

#[test]
fn quote_indents_both_margins_by_leftmargini() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // A quote long enough to wrap: every line starts at \leftmargini
    // (2.5em = 27.375pt at 11pt) and no line passes the right margin less
    // that; the paragraph after it returns to the full measure.
    let body = "quoted words that go on and on ".repeat(8);
    let src = format!(
        "\\documentclass[11pt]{{article}}\\begin{{document}}\nIntro text:\n\\begin{{quote}}\n{body}\n\\end{{quote}}\nAfter the quote.\n\\end{{document}}"
    );
    let (v1, words) = layout(&src);
    let (text_x, text_w) = measure();
    assert!(!v1.diagnostics.iter().any(|d| d.code == "unsupported_block"), "{:?}", v1.diagnostics);
    let intro = word(&words, "Intro");
    let after = word(&words, "After");
    // The quote's lines lie between the intro and the closing paragraph.
    let mut baselines: Vec<f64> = words.iter().filter(|w| w.baseline > intro.baseline && w.baseline < after.baseline).map(|w| w.baseline).collect();
    baselines.sort_by(f64::total_cmp);
    baselines.dedup();
    assert!(baselines.len() >= 2, "the quote wraps: {baselines:?}");
    for b in &baselines {
        let line: Vec<&Word> = words.iter().filter(|w| w.baseline == *b).collect();
        let left = line.iter().map(|w| w.x).fold(f64::INFINITY, f64::min);
        let right = line.iter().map(|w| w.x + w.width).fold(0.0, f64::max);
        assert!((left - (text_x + leftmargini())).abs() < 0.05, "quote line starts at \\leftmargini: {left} ({line:?})");
        assert!(right <= text_x + text_w - leftmargini() + 0.05, "quote line ends before the right margin less \\leftmargini: {right}");
    }
    // The environment opened in horizontal mode (no blank line before
    // `\begin{quote}`): \topsep 9pt only, no \partopsep.
    assert!((baselines[0] - intro.baseline - bp(13.6 + 9.0)).abs() < 0.05, "gap above quote: {}", baselines[0] - intro.baseline);
    assert!((after.x - text_x).abs() < 0.05, "the paragraph after the quote is not indented: {}", after.x);
}

#[test]
fn hrule_is_a_full_measure_rule_with_no_interline_glue() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let src = "\\documentclass[11pt]{article}\\begin{document}\nAbove, typography.\n\n\\hrule\n\nBelow the rule.\n\\end{document}";
    let (v1, words) = layout(src);
    let (text_x, text_w) = measure();
    assert!(!v1.diagnostics.iter().any(|d| d.code == "unsupported_block"), "{:?}", v1.diagnostics);
    let rules: Vec<(f64, f64, f64, f64)> = v1
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .filter_map(|it| match it {
            V1Item::Rule {
                x_pt,
                y_pt,
                width_pt,
                height_pt,
                ..
            } => Some((*x_pt, *y_pt, *width_pt, *height_pt)),
            _ => None,
        })
        .collect();
    assert_eq!(rules.len(), 1, "{rules:?}");
    let (x, y, w, h) = rules[0];
    assert!((x - text_x).abs() < 0.05 && (w - text_w).abs() < 0.05, "full measure: {rules:?}");
    assert!((h - bp(0.4)).abs() < 0.01, "default \\hrule height 0.4pt: {h}");
    // TeX §1056: no interline glue on either side. The rule's top is the
    // previous line's bottom (baseline + the depth of "typography", the
    // descender of lm 11pt: about 2.1pt) and the next line is appended with
    // prev_depth = ignore_depth, so its top touches the rule's bottom:
    // baseline = rule bottom + the line's height (the cap height of "B",
    // about 7.5pt), well under a \baselineskip.
    let above = word(&words, "Above,");
    let below = word(&words, "Below");
    let gap_above = y - above.baseline;
    assert!(gap_above > bp(1.5) && gap_above < bp(3.0), "rule top on the previous line's depth: {gap_above}bp");
    let gap_below = below.baseline - (y + h);
    assert!(gap_below > bp(6.0) && gap_below < bp(9.0), "next line's top on the rule's bottom: {gap_below}bp");
    // Nothing else is reported for the rule.
    assert!(!v1.diagnostics.iter().any(|d| d.message.contains("hrule")), "{:?}", v1.diagnostics);
}

#[test]
fn preamble_parskip_and_group_size_declarations_are_read_from_the_source() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // `\setlength{\parskip}{0.65em}` at 11pt = 7.1175pt between paragraphs
    // (on top of \baselineskip 13.6pt); `{\Large\bfseries ...}` sets its
    // group at 14.4pt bold, `\LARGE` at 17.28pt, back to 10.95pt after it.
    let src = "\\documentclass[11pt]{article}\\setlength{\\parskip}{0.65em}\\begin{document}\nFirst para.\n\nSecond para.\n\n{\\Large\\bfseries Big title} then {\\LARGE Bigger} and normal.\n\\end{document}";
    let (v1, words) = layout(src);
    let first = word(&words, "First");
    let second = word(&words, "Second");
    assert!((second.baseline - first.baseline - bp(13.6 + 7.1175)).abs() < 0.05, "parskip: {}", second.baseline - first.baseline);
    let sizes: Vec<(String, f64, bool)> = v1
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .filter_map(|it| match it {
            V1Item::Text { text, font_size_pt, font, .. } => Some((text.clone(), *font_size_pt, font.as_ref().is_some_and(|f| f.weight == "bold"))),
            _ => None,
        })
        .collect();
    let of = |t: &str| sizes.iter().find(|s| s.0 == t).unwrap_or_else(|| panic!("no {t:?} in {sizes:?}"));
    assert!((of("Big").1 - bp(14.4)).abs() < 0.01, "\\Large: {:?}", of("Big"));
    assert!(of("Big").2 && of("title").2, "\\bfseries in the group: {sizes:?}");
    assert!((of("title").1 - bp(14.4)).abs() < 0.01);
    assert!((of("then").1 - bp(10.95)).abs() < 0.01 && !of("then").2, "back to normalsize/medium after the group: {:?}", of("then"));
    assert!((of("Bigger").1 - bp(17.28)).abs() < 0.01, "\\LARGE: {:?}", of("Bigger"));
    assert!((of("normal.").1 - bp(10.95)).abs() < 0.01);
}
