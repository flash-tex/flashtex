//! GH-EXAM-CLASS: exam.cls's page frame and running head/foot.
//!
//! exam.cls (TeX Live 2026) loads article, then sets its own frame (lines
//! 747-760: 1in side margins, `\textheight = \paperheight - 2.2in`,
//! `\headheight`/`\headsep` 15pt, `\footskip` 29pt) and draws head and foot
//! as three full-width overlapping parboxes with optional rules. Before
//! this, `exam` fell back to article's frame with no chrome at all.
//!
//! ## Oracle
//!
//! Every number is a word origin (first glyph x, baseline y from the page
//! top, bp) read by `tools/visual-oracle/pdftext.py`, or a rule edge from
//! `mutool draw -F trace`, from pdfLaTeX's output for the same document:
//! pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026). pdflatex is an oracle
//! only, never in the product path.

mod common;

use flashtex_render_pipeline::display::Item;

/// The corpus harness's exact-route tolerance.
const TOL: f64 = 0.01;

/// Per page: every glyph run's (text, x, baseline) and every rule's
/// (x, top, width, height), in bp.
#[allow(clippy::type_complexity)]
fn render(text: &str) -> Vec<(Vec<(String, f64, f64)>, Vec<(f64, f64, f64, f64)>)> {
    let r = common::render_one(text);
    r.v2.pages
        .iter()
        .map(|page| {
            let (mut runs, mut rules) = (Vec::new(), Vec::new());
            for item in page.resident_items() {
                match item {
                    Item::GlyphRun(run) => {
                        if let Some(g) = run.glyphs.first() {
                            runs.push((run.text.trim().to_string(), g.origin_x.to_bp(), g.baseline_y.to_bp()));
                        }
                    }
                    Item::Rule(rule) => rules.push((rule.x.to_bp(), rule.top.to_bp(), rule.width.to_bp(), rule.height.to_bp())),
                    _ => {}
                }
            }
            (runs, rules)
        })
        .collect()
}

fn check(runs: &[(String, f64, f64)], want: &[(&str, f64, f64)]) {
    for (text, x, y) in want {
        let found = runs
            .iter()
            .find(|(t, rx, ry)| t.starts_with(text) && (rx - x).abs() <= TOL && (ry - y).abs() <= TOL);
        assert!(found.is_some(), "{text} at ({x}, {y}) not in {runs:?}");
    }
}

#[test]
fn head_with_rule_and_plain_first_page() {
    if !common::lm_available() {
        return;
    }
    let pages = render(
        "\\documentclass[11pt]{exam}\n\\pagestyle{head}\n\\headrule\n\
         \\header{\\textbf{Left}}{\\textbf{Mid}}{\\textbf{Right}}\n\
         \\begin{document}\n\\thispagestyle{plain}\nBody one.\n\\newpage\nBody two.\n\\end{document}\n",
    );
    assert_eq!(pages.len(), 2);
    // Page 1 (`plain`): the body at exam's frame, the folio at \footskip.
    check(&pages[0].0, &[("Body", 88.936, 82.959), ("one.", 117.875, 82.959), ("1", 303.273, 734.492)]);
    assert_eq!(pages[0].0.len(), 3, "{:?}", pages[0].0);
    assert!(pages[0].1.is_empty(), "{:?}", pages[0].1);
    // Page 2 (`head`): the centre slot is centred on the line, not in the
    // gap between left and right.
    check(
        &pages[1].0,
        &[("Left", 72.0, 52.593), ("Mid", 294.823, 52.593), ("Right", 509.339, 52.593), ("Body", 88.936, 82.959)],
    );
    // pdflatex strokes the rule at y 56.857 with width .398: top 56.658.
    let (x, top, width, height) = pages[1].1[0];
    assert!((x - 72.0).abs() <= TOL && (top - 56.658).abs() <= TOL && (width - 468.0).abs() <= TOL && (height - 0.398).abs() <= TOL, "{:?}", pages[1].1);
}

#[test]
fn default_headandfoot_numbers_every_page_but_the_first() {
    if !common::lm_available() {
        return;
    }
    let pages = render("\\documentclass{exam}\n\\begin{document}\nBody one.\n\\newpage\nBody two.\n\\end{document}\n");
    check(&pages[0].0, &[("Body", 86.944, 81.963)]);
    assert_eq!(pages[0].0.len(), 2, "no foot on page 1: {:?}", pages[0].0);
    check(&pages[1].0, &[("Body", 86.944, 81.963), ("Page", 291.402, 744.687), ("2", 315.621, 744.687)]);
}

#[test]
fn first_page_and_running_heads_macros_and_foot_rule() {
    if !common::lm_available() {
        return;
    }
    let pages = render(
        "\\documentclass[12pt]{exam}\n\\newcommand{\\nm}{Ann}\n\\firstpageheader{First \\nm}{}{}\n\
         \\runningheader{Run}{\\thepage}{}\n\\footrule\n\\begin{document}\nOne.\n\\newpage\nTwo.\n\\end{document}\n",
    );
    check(&pages[0].0, &[("First", 72.0, 52.324), ("Ann", 100.511, 52.324), ("One.", 89.559, 83.955)]);
    check(
        &pages[1].0,
        &[("Run", 72.0, 52.324), ("2", 303.077, 52.324), ("Two.", 89.559, 83.955), ("Page", 288.849, 746.049), ("2", 317.298, 746.049)],
    );
    // `\footrule` on both pages: stroked at 734.691, width .398: top 734.492.
    for (page, _) in pages.iter().enumerate() {
        let (x, top, width, _) = pages[page].1[0];
        assert!((x - 72.0).abs() <= TOL && (top - 734.492).abs() <= TOL && (width - 468.0).abs() <= TOL, "{:?}", pages[page].1);
    }
}
