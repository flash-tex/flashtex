//! Golden tests: every expected number is derived by hand in the comments.
//!
//! `TestFont` is a monospace test face: every glyph advances 500 units
//! (5pt at 10pt), space 250 (2.5pt), ascender 700 / descender -300 (7pt / 3pt
//! at 10pt), no kerns, no ligatures, glyph id = Unicode scalar. Its interword
//! stretch/shrink are per test.

use flashtex_paragraph_layout::core14::Core14Times;
use flashtex_paragraph_layout::*;

struct TestFont {
    stretch: f64,
    shrink: f64,
}

impl TestFont {
    /// TeX-Times-like glue: stretch 150, shrink 60 (1.5pt / 0.6pt at 10pt).
    const TIMES_LIKE: TestFont = TestFont {
        stretch: 150.0,
        shrink: 60.0,
    };
}

impl FontMetricsSource for TestFont {
    fn font_id(&self) -> FontId {
        FontId::from_label("test:mono")
    }
    fn units_per_em(&self) -> f64 {
        1000.0
    }
    fn advance(&self, _ch: char) -> f64 {
        500.0
    }
    fn kern(&self, _l: char, _r: char) -> f64 {
        0.0
    }
    fn glyph_id(&self, ch: char) -> u32 {
        ch as u32
    }
    fn ascender(&self) -> f64 {
        700.0
    }
    fn descender(&self) -> f64 {
        -300.0
    }
    fn line_gap(&self) -> f64 {
        0.0
    }
    fn space(&self) -> f64 {
        250.0
    }
    fn space_stretch(&self) -> f64 {
        self.stretch
    }
    fn space_shrink(&self) -> f64 {
        self.shrink
    }
}

fn items(font: &dyn FontMetricsSource, hyph: &dyn Hyphenator, text: &str) -> Vec<Item> {
    let mut b = ParagraphBuilder::new(hyph);
    b.text(font, 10.0, text, 0).unwrap();
    b.finish(Glue::fil())
}

fn params(width: f64) -> LineBreakParams {
    LineBreakParams::article_12pt_letter_1in().with_width(width)
}

fn xs(line: &Line) -> Vec<f64> {
    line.runs.iter().map(|r| r.x).collect()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn assert_xs(line: &Line, expected: &[f64]) {
    let got = xs(line);
    assert_eq!(
        got.len(),
        expected.len(),
        "run count on line {}: {:?}",
        line.index,
        got
    );
    for (g, e) in got.iter().zip(expected) {
        assert!(
            close(*g, *e),
            "line {} x: got {got:?}, expected {expected:?}",
            line.index
        );
    }
}

const EIGHT_WORDS: &str = "aaaa bbbb cccc dddd eeee ffff gggg hhhh";

/// Justified, width 91: four 20pt words + three 2.5pt spaces = 87.5pt natural.
/// Shortfall 3.5pt over 3 x 1.5pt stretch -> ratio 0.777..., badness
/// round(100 x 0.7778^3) = 47 (Loose), demerits (10 + 47)^2 = 3249. Five words
/// (110pt) cannot shrink to 91 (4 x 0.6pt), three words are far too loose, so
/// the only feasible layout is 4 + 4. The last line has `\parfillskip`
/// (fil) -> badness 0, demerits 100. Total 3349, found in pass 1 (47 <= 100).
/// Each stretched space is 2.5 + 0.7778 x 1.5 = 3.6667pt.
#[test]
fn justified_stretch_uses_total_fit_numbers() {
    let it = items(&TestFont::TIMES_LIKE, &NoHyphenation, EIGHT_WORDS);
    let out = layout_paragraph(&it, &params(91.0)).unwrap();
    assert_eq!(out.lines.len(), 2);
    assert_eq!(out.stats.pass, 1);
    let l0 = &out.lines[0];
    assert!(close(l0.natural_width, 87.5));
    assert!(close(l0.ratio, 3.5 / 4.5));
    assert_eq!(l0.badness, 47.0);
    assert_eq!(out.breaks[0].fitness, Fitness::Loose);
    assert!(close(out.breaks[0].demerits, 3249.0));
    assert_xs(l0, &[0.0, 23.0 + 2.0 / 3.0, 47.0 + 1.0 / 3.0, 71.0]);
    assert!(close(l0.set_width, 91.0));
    let l1 = &out.lines[1];
    assert_eq!(l1.badness, 0.0);
    assert_xs(l1, &[0.0, 22.5, 45.0, 67.5]);
    assert!(close(out.stats.total_demerits, 3349.0));
    assert!(out.stats.overfull.is_empty() && out.stats.underfull.is_empty());
    // Vertical: first baseline at the line height (7pt); second at
    // 7 + depth 3 + interline glue (14.5 - 3 - 7 = 4.5) + 7 = 21.5.
    assert!(close(l0.baseline_y, 7.0));
    assert!(close(l1.baseline_y, 21.5));
    assert!(close(out.height, 24.5));
}

/// Justified, width 109: five words = 110pt natural, 1pt over. Shrink
/// 4 x 0.6 = 2.4pt -> ratio -0.41667, badness round(100 x 0.41667^3) = 7
/// (Decent), demerits 17^2 = 289. Four words would be 21.5pt short (ratio
/// 4.78, infinite badness), so 5 + 3 is the only feasible layout: total
/// 289 + 100 = 389. Each shrunk space is 2.5 - 0.41667 x 0.6 = 2.25pt.
#[test]
fn justified_shrink_uses_total_fit_numbers() {
    let it = items(&TestFont::TIMES_LIKE, &NoHyphenation, EIGHT_WORDS);
    let out = layout_paragraph(&it, &params(109.0)).unwrap();
    assert_eq!(out.lines.len(), 2);
    let l0 = &out.lines[0];
    assert!(close(l0.natural_width, 110.0));
    assert!(close(l0.ratio, -1.0 / 2.4));
    assert_eq!(l0.badness, 7.0);
    assert_eq!(out.breaks[0].fitness, Fitness::Decent);
    assert!(close(out.breaks[0].demerits, 289.0));
    assert_xs(l0, &[0.0, 22.25, 44.5, 66.75, 89.0]);
    assert!(close(l0.set_width, 109.0));
    assert_eq!(out.lines[1].runs.len(), 3);
    assert!(close(out.stats.total_demerits, 389.0));
}

/// `\raggedright` (rightskip 0pt plus 1fil): spaces stay at their natural
/// 2.5pt, every line has badness 0 and fitness Decent, and the breaks are
/// the same 4 + 4 as the justified case because the fil absorbs the slack.
#[test]
fn ragged_right_keeps_natural_spaces() {
    let it = items(&TestFont::TIMES_LIKE, &NoHyphenation, EIGHT_WORDS);
    let out = layout_paragraph(&it, &params(91.0).ragged()).unwrap();
    assert_eq!(out.lines.len(), 2);
    for (l, b) in out.lines.iter().zip(&out.breaks) {
        assert_eq!(l.badness, 0.0);
        assert_eq!(b.fitness, Fitness::Decent);
        assert_xs(l, &[0.0, 22.5, 45.0, 67.5]);
        assert!(close(l.set_width, 91.0));
    }
    // Two lines x (10 + 0)^2.
    assert!(close(out.stats.total_demerits, 200.0));
}

/// Where first-fit and total-fit differ. Words 30, 30, 10, 30, 30, 30, 30pt,
/// width 75, space 2.5pt plus 10pt (TestFont stretch 1000).
///
/// First-fit fills line 1 with "30 30 10" = 75pt exactly (badness 0), leaving
/// "30 30" = 62.5pt for line 2: 12.5pt short over 10pt stretch, ratio 1.25,
/// badness round(100 x 1.953) = 195 (VeryLoose), then "30 30" last.
///
/// Total-fit charges adj_demerits (10000) whenever adjacent fitness classes
/// differ by more than one, and TeX treats the paragraph start as Decent:
/// * first-fit's layout: 100 + (205^2 + 10000 adj) + (100 + 10000 adj) = 62225
/// * "30 30" / "10 30 30" (= 75pt, badness 0) / "30 30": (205^2 + 10000 adj)
///   + (100 + 10000 adj) + 100 = 62225
///
/// The totals tie. TeX's rule decides: candidate nodes are created in fitness
/// class order (very loose first) and a later active node wins an equal
/// comparison (`<=`), so the layout whose penultimate line is Decent — line 1
/// ending after the second word — is chosen. Pass 1 (pretolerance 100) fails
/// because every layout needs one 195-badness line; pass 2 succeeds.
#[test]
fn first_fit_and_total_fit_differ_on_a_loose_line() {
    let font = TestFont {
        stretch: 1000.0,
        shrink: 100.0,
    };
    let text = "aaaaaa bbbbbb cc dddddd eeeeee ffffff gggggg";
    let it = items(&font, &NoHyphenation, text);
    let greedy = layout_paragraph(&it, &params(75.0).first_fit()).unwrap();
    let counts = |l: &Lines| l.lines.iter().map(|x| x.runs.len()).collect::<Vec<_>>();
    assert_eq!(counts(&greedy), vec![3, 2, 2]);
    assert_eq!(greedy.stats.pass, 0);
    assert_eq!(greedy.lines[1].badness, 195.0);
    let total = layout_paragraph(&it, &params(75.0)).unwrap();
    assert_eq!(counts(&total), vec![2, 3, 2]);
    assert_eq!(total.stats.pass, 2);
    assert_eq!(total.lines[0].badness, 195.0);
    assert_eq!(total.breaks[0].fitness, Fitness::VeryLoose);
    assert_eq!(total.lines[1].badness, 0.0);
    assert!(close(total.breaks[0].demerits, 52025.0));
    assert!(close(total.breaks[1].demerits, 62125.0));
    assert!(close(total.stats.total_demerits, 62225.0));
}

/// `\-` discretionary, ragged, width 35: "xxxx yyyy\-zzzz". "xxxx yyyy" is
/// 42.5pt so line 1 is "xxxx"; "yyyyzzzz" is 40pt > 35 so the word must split:
/// "yyyy-" (20 + 5pt hyphen = 25pt) then "zzzz". Demerits: line 1 100, line 2
/// 10^2 + hyphenpenalty 50^2 = 2600, last line 100 + finalhyphendemerits 5000
/// (the line before it was hyphenated) = 5100; total 7800, pass 1 because an
/// explicit discretionary is legal in the first pass.
#[test]
fn explicit_discretionary_produces_hyphen_run_with_marker_cluster() {
    let text = "xxxx yyyy\\-zzzz";
    let it = items(&TestFont::TIMES_LIKE, &ExplicitDiscretionary, text);
    let out = layout_paragraph(&it, &params(35.0).ragged()).unwrap();
    assert_eq!(out.lines.len(), 3);
    assert_eq!(out.stats.pass, 1);
    assert_eq!(out.stats.hyphenated_lines, 1);
    assert!(out.breaks[1].hyphenated);
    assert!(close(out.stats.total_demerits, 7800.0));
    let l1 = &out.lines[1];
    assert_eq!(l1.runs.len(), 2);
    assert_eq!(l1.runs[0].source, 5..9);
    assert!(!l1.runs[0].is_hyphen);
    let hy = &l1.runs[1];
    assert!(hy.is_hyphen);
    assert!(close(hy.x, 20.0));
    assert_eq!(hy.glyphs.len(), 1);
    assert_eq!(hy.glyphs[0].gid, '-' as u32);
    assert_eq!(
        hy.glyphs[0].cluster,
        9..11,
        "hyphen cluster is the `\\-` marker bytes"
    );
    assert!(close(l1.natural_width, 25.0));
    assert_eq!(out.lines[2].runs[0].source, 11..15);
    // Byte clusters of the fragments flow through unchanged.
    assert_eq!(out.lines[2].runs[0].glyphs[0].cluster, 11..12);
}

/// Without hyphenation the 40pt word cannot fit a 35pt measure. Both passes
/// fail; the final pass accepts the break with artificial demerits and the
/// line is reported overfull by 5pt rather than dropped.
#[test]
fn overfull_line_is_reported_not_dropped() {
    let it = items(&TestFont::TIMES_LIKE, &NoHyphenation, "xxxx yyyyzzzz");
    let out = layout_paragraph(&it, &params(35.0).ragged()).unwrap();
    assert_eq!(out.lines.len(), 2);
    assert_eq!(out.stats.pass, 2);
    assert_eq!(out.stats.overfull.len(), 1);
    assert_eq!(out.stats.overfull[0].line, 1);
    assert!(close(out.stats.overfull[0].excess, 5.0));
    assert_eq!(out.lines[1].runs[0].source, 5..13);
    assert!(close(out.lines[1].set_width, 40.0));
    // First-fit reports the same overfull line.
    let greedy = layout_paragraph(&it, &params(35.0).ragged().first_fit()).unwrap();
    assert_eq!(greedy.stats.overfull, out.stats.overfull);
}

/// A wrapper that hides a font's kern pairs.
struct NoKern<'a>(&'a Core14Times);

impl FontMetricsSource for NoKern<'_> {
    fn font_id(&self) -> FontId {
        FontId::from_label("test:times-unkerned")
    }
    fn units_per_em(&self) -> f64 {
        self.0.units_per_em()
    }
    fn advance(&self, ch: char) -> f64 {
        self.0.advance(ch)
    }
    fn kern(&self, _l: char, _r: char) -> f64 {
        0.0
    }
    fn glyph_id(&self, ch: char) -> u32 {
        self.0.glyph_id(ch)
    }
    fn ascender(&self) -> f64 {
        self.0.ascender()
    }
    fn descender(&self) -> f64 {
        self.0.descender()
    }
    fn line_gap(&self) -> f64 {
        0.0
    }
    fn space(&self) -> f64 {
        self.0.space()
    }
    fn space_stretch(&self) -> f64 {
        self.0.space_stretch()
    }
    fn space_shrink(&self) -> f64 {
        self.0.space_shrink()
    }
}

/// Kerning decides a break. Times-Roman 10pt: A and V are 7.22pt each, the
/// AFM pairs A-V and V-A are both -1.35pt. "AVAVAVAV" is 8 x 7.22 = 57.76pt
/// unkerned but 57.76 - 7 x 1.35 = 48.31pt kerned. Two such words with a
/// 2.5pt space: kerned 99.12pt fits a 100pt measure on one line; unkerned
/// 118.02pt needs two lines. This is the issue #10 finding ("onto" wrapping
/// early without kerns) in miniature.
#[test]
fn kerning_changes_the_break() {
    let text = "AVAVAVAV AVAVAVAV";
    assert_eq!(Core14Times::ROMAN.kern('A', 'V'), -135.0);
    assert_eq!(Core14Times::ROMAN.kern('V', 'A'), -135.0);
    let kerned = items(&Core14Times::ROMAN, &NoHyphenation, text);
    let out = layout_paragraph(&kerned, &params(100.0).ragged()).unwrap();
    assert_eq!(out.lines.len(), 1);
    assert!(close(out.lines[0].runs[0].width, 48.31));
    assert!(close(out.lines[0].natural_width, 99.12));
    assert!(close(out.lines[0].runs[1].x, 50.81));
    // Kern is carried per glyph and folded into the pen advance.
    let g = &out.lines[0].runs[0].glyphs;
    assert!(close(g[1].x_offset, 7.22 - 1.35));
    assert!(close(g[0].advance, 5.87));
    let unkerned = items(&NoKern(&Core14Times::ROMAN), &NoHyphenation, text);
    let out = layout_paragraph(&unkerned, &params(100.0).ragged()).unwrap();
    assert_eq!(out.lines.len(), 2);
    assert!(close(out.lines[0].runs[0].width, 57.76));
}

/// An explicit kern followed by glue is a legal break point and vanishes when
/// the line breaks there (TeX rule); otherwise it displaces the next box.
#[test]
fn explicit_kern_is_discarded_at_a_break() {
    let font = TestFont::TIMES_LIKE;
    let h = NoHyphenation;
    let mut b = ParagraphBuilder::new(&h);
    b.word(&font, 10.0, "aaaa", 0).unwrap();
    b.kern(4.0);
    b.space(&font, 10.0, 4..5);
    b.word(&font, 10.0, "bbbb", 5).unwrap();
    let it = b.finish(Glue::fil());
    // Wide measure: kern kept, "bbbb" at 20 + 4 + 2.5 = 26.5.
    let out = layout_paragraph(&it, &params(100.0).ragged()).unwrap();
    assert_eq!(out.lines.len(), 1);
    assert_xs(&out.lines[0], &[0.0, 26.5]);
    // Narrow measure: break at the kern; line 1 is exactly "aaaa" = 20pt.
    let out = layout_paragraph(&it, &params(21.0).ragged()).unwrap();
    assert_eq!(out.lines.len(), 2);
    assert!(close(out.lines[0].natural_width, 20.0));
    assert_xs(&out.lines[1], &[0.0]);
}

/// Pages for the vertical tests: 200 x 100pt, no margins, topskip 10,
/// maxdepth 5, baselineskip 10, parskip 0. TestFont lines are 7pt high and 3pt
/// deep, so interline glue is 10 - 3 - 7 = 0 and baselines sit at 10, 20, ...,
/// 100: exactly ten lines per page.
fn small_page() -> PageParams {
    PageParams {
        page_width: 200.0,
        page_height: 100.0,
        margin_top: 0.0,
        margin_bottom: 0.0,
        margin_left: 0.0,
        margin_right: 0.0,
        topskip: 10.0,
        max_depth: 5.0,
        parskip: Glue::fixed(0.0),
        baselineskip: 10.0,
        lineskip: 1.0,
        lineskiplimit: 0.0,
        baseline_grid: None,
        club_lines: 2,
        widow_lines: 2,
    }
}

/// A paragraph of exactly `n` one-word lines (forced breaks).
fn lines_para(n: usize, size: f64) -> Lines {
    let font = TestFont::TIMES_LIKE;
    let h = NoHyphenation;
    let mut b = ParagraphBuilder::new(&h);
    for i in 0..n {
        b.word(&font, size, "w", i).unwrap();
        if i + 1 < n {
            b.line_break();
        }
    }
    let it = b.finish(Glue::fil());
    let out = layout_paragraph(&it, &params(200.0)).unwrap();
    assert_eq!(out.lines.len(), n);
    out
}

fn page_line_counts(p: &Pages) -> Vec<usize> {
    p.pages.iter().map(|pg| pg.lines.len()).collect()
}

#[test]
fn orphan_line_moves_to_next_page() {
    // 9 + 3 lines: B's first line would be the tenth line of page 1, alone
    // (club_lines 2), so the whole of B moves to page 2.
    let blocks = vec![
        ParagraphBlock::body(lines_para(9, 10.0)),
        ParagraphBlock::body(lines_para(3, 10.0)),
    ];
    let p = layout_pages(&blocks, &small_page());
    assert_eq!(page_line_counts(&p), vec![9, 3]);
    assert!(p.overflow.is_empty());
    let last = p.pages[0].lines.last().unwrap();
    assert!(close(last.baseline_y, 90.0));
    assert!(close(p.pages[1].lines[0].baseline_y, 10.0));
    assert_eq!(p.pages[1].lines[0].paragraph, 1);
    // Runs are translated onto the page: x = margin 0 + 0, baseline absolute.
    assert!(close(p.pages[1].runs[0].baseline_y, 10.0));
}

#[test]
fn widow_line_pulls_a_line_or_the_paragraph() {
    // 8 + 3: two lines of B fit (90, 100) leaving one widow; pulling one back
    // would leave one orphan, so B moves entirely.
    let blocks = vec![
        ParagraphBlock::body(lines_para(8, 10.0)),
        ParagraphBlock::body(lines_para(3, 10.0)),
    ];
    assert_eq!(
        page_line_counts(&layout_pages(&blocks, &small_page())),
        vec![8, 3]
    );
    // 7 + 4: three lines of B fit (80, 90, 100) leaving one widow; keep two,
    // carry two.
    let blocks = vec![
        ParagraphBlock::body(lines_para(7, 10.0)),
        ParagraphBlock::body(lines_para(4, 10.0)),
    ];
    let p = layout_pages(&blocks, &small_page());
    assert_eq!(page_line_counts(&p), vec![9, 2]);
    assert_eq!(p.pages[1].lines[0].line, 2);
}

#[test]
fn keep_with_next_moves_heading_with_its_body() {
    // 9 body lines, then a one-line heading that would land at 100 with no
    // room for two lines of the following paragraph: heading goes to page 2.
    let blocks = vec![
        ParagraphBlock::body(lines_para(9, 10.0)),
        ParagraphBlock {
            keep_with_next: true,
            ..ParagraphBlock::body(lines_para(1, 10.0))
        },
        ParagraphBlock::body(lines_para(3, 10.0)),
    ];
    let p = layout_pages(&blocks, &small_page());
    assert_eq!(page_line_counts(&p), vec![9, 4]);
    assert_eq!(p.pages[1].lines[0].paragraph, 1);
    assert!(close(p.pages[1].lines[1].baseline_y, 20.0));
}

#[test]
fn long_paragraph_breaks_deterministically_without_loss() {
    let blocks = vec![ParagraphBlock::body(lines_para(25, 10.0))];
    let p = layout_pages(&blocks, &small_page());
    assert_eq!(page_line_counts(&p), vec![10, 10, 5]);
    assert!(p.overflow.is_empty());
    let total: usize = p.pages.iter().map(|pg| pg.runs.len()).sum();
    assert_eq!(total, 25);
}

/// A 300pt line (210pt high, 90pt deep) cannot fit a 100pt page. It is placed
/// at the top of an empty page and reported: bottom 300 vs limit 100.
#[test]
fn page_overflow_is_reported_and_content_kept() {
    let blocks = vec![
        ParagraphBlock::body(lines_para(2, 10.0)),
        ParagraphBlock::body(lines_para(1, 300.0)),
    ];
    let p = layout_pages(&blocks, &small_page());
    assert_eq!(page_line_counts(&p), vec![2, 1]);
    assert_eq!(p.overflow.len(), 1);
    let o = &p.overflow[0];
    assert_eq!((o.page, o.paragraph, o.line), (2, 1, 0));
    assert!(close(o.bottom, 300.0));
    assert!(close(o.limit, 100.0));
    assert_eq!(p.pages[1].runs.len(), 1);
}

/// Baseline grid of 14.5pt anchored at the first baseline (topskip). With the
/// 12pt-article page: heading (Times-Bold 17.28pt: height 11.68, depth 3.54)
/// at 72.27 + 12 = 84.27; the body line after `\section`'s 12.42pt afterskip
/// would naturally sit at 84.27 + 3.54 + 12.42 + (14.5 - 3.54 - 8.196) +
/// 8.196 = 111.19, which the grid rounds up to 84.27 + 2 x 14.5 = 113.27; the
/// next body lines follow at +14.5 (already on the grid).
#[test]
fn baseline_grid_snaps_every_baseline() {
    let h = NoHyphenation;
    let mut b = ParagraphBuilder::new(&h);
    b.text(&Core14Times::BOLD, 17.28, "Heading", 0).unwrap();
    let heading = layout_paragraph(
        &b.finish(Glue::fil()),
        &LineBreakParams::article_12pt_letter_1in(),
    )
    .unwrap();
    // Three body lines via explicit line breaks.
    let mut b2 = ParagraphBuilder::new(&h);
    for (i, w) in ["one", "two", "three"].iter().enumerate() {
        b2.word(&Core14Times::ROMAN, 12.0, w, i * 6).unwrap();
        if i < 2 {
            b2.line_break();
        }
    }
    let body = layout_paragraph(
        &b2.finish(Glue::fil()),
        &LineBreakParams::article_12pt_letter_1in(),
    )
    .unwrap();
    let blocks = vec![
        ParagraphBlock::section_heading_12pt(heading),
        ParagraphBlock::body(body),
    ];
    let mut pp = PageParams::article_12pt_letter_1in_tex_pt();
    let free = layout_pages(&blocks, &pp);
    let ys: Vec<f64> = free.pages[0].lines.iter().map(|l| l.baseline_y).collect();
    assert!(close(ys[0], 84.27));
    assert!(close(
        ys[1],
        84.27 + 3.5424 + 12.42 + (14.5 - 3.5424 - 8.196) + 8.196
    ));
    assert!(close(ys[1], 111.19));
    pp.baseline_grid = Some(14.5);
    let grid = layout_pages(&blocks, &pp);
    let ys: Vec<f64> = grid.pages[0].lines.iter().map(|l| l.baseline_y).collect();
    assert_eq!(ys.len(), 4);
    assert!(close(ys[0], 84.27));
    assert!(close(ys[1], 84.27 + 29.0));
    assert!(close(ys[2], 84.27 + 43.5));
    assert!(close(ys[3], 84.27 + 58.0));
    for y in ys {
        let k = (y - 84.27) / 14.5;
        assert!(close(k, k.round()), "baseline {y} is off the grid");
    }
}

/// A metrics source with real per-glyph boxes (TFM-style `charht`/`chardp`):
/// same shape as `TestFont` (500 advance, 700 ascender / -300 descender,
/// 250 space), except 'T' has an unusually tall box (900) and 'y' an
/// unusually deep one (500); every other glyph keeps the font default.
struct BoxTestFont;

impl FontMetricsSource for BoxTestFont {
    fn font_id(&self) -> FontId {
        FontId::from_label("test:mono-boxes")
    }
    fn units_per_em(&self) -> f64 {
        1000.0
    }
    fn advance(&self, _ch: char) -> f64 {
        500.0
    }
    fn kern(&self, _l: char, _r: char) -> f64 {
        0.0
    }
    fn glyph_id(&self, ch: char) -> u32 {
        ch as u32
    }
    fn ascender(&self) -> f64 {
        700.0
    }
    fn descender(&self) -> f64 {
        -300.0
    }
    fn line_gap(&self) -> f64 {
        0.0
    }
    fn space(&self) -> f64 {
        250.0
    }
    fn glyph_height(&self, ch: char) -> f64 {
        if ch == 'T' { 900.0 } else { self.ascender() }
    }
    fn glyph_depth(&self, ch: char) -> f64 {
        if ch == 'y' { 500.0 } else { -self.descender() }
    }
}

/// A run's height/depth is the tallest/deepest glyph box in it
/// ([`FontMetricsSource::glyph_height`]/[`glyph_depth`]), and that box feeds
/// straight into `\baselineskip` placement of the *next* line — exactly like
/// real TeX metrics (a `y` or a tall capital changes where the following
/// baseline lands, not just how the current line looks).
///
/// "Ty aaaa" at 10pt, width 10pt: the only legal break is the interior space
/// (the whole line is 32.5pt natural, nowhere near 10pt, so a single line is
/// infeasible), giving "Ty" then "aaaa" as the two lines. "Ty" is one
/// unbreakable box: natural width (500+500)*0.01 = 10pt = target, so ratio 0,
/// badness 0 -- a clean fit with no interior glue to muddy the height/depth
/// numbers. Its height is max(glyph_height('T')=900, glyph_height('y')=700)
/// *0.01 = 9pt; its depth is max(glyph_depth('T')=300, glyph_depth('y')=500)
/// *0.01 = 5pt. "aaaa" has no overridden glyphs: height 7pt, depth 3pt (the
/// plain `TestFont` numbers, confirmed against
/// `justified_stretch_uses_total_fit_numbers` below).
#[test]
fn glyph_height_and_depth_change_baseline_placement() {
    let it = items(&BoxTestFont, &NoHyphenation, "Ty aaaa");
    let out = layout_paragraph(&it, &params(10.0)).unwrap();
    assert_eq!(out.lines.len(), 2);
    let l0 = &out.lines[0];
    assert!(close(l0.natural_width, 10.0));
    assert_eq!(l0.badness, 0.0);
    assert!(close(l0.height, 9.0));
    assert!(close(l0.depth, 5.0));
    // First baseline sits at the line's own height (TeX: no glue above line 1).
    assert!(close(l0.baseline_y, 9.0));
    let l1 = &out.lines[1];
    assert!(close(l1.height, 7.0));
    assert!(close(l1.depth, 3.0));
    // baselineskip 14.5 - prev_depth 5 - height 7 = 2.5 >= lineskiplimit 0, so
    // the ordinary interline glue applies: y = 9 + 5 + 2.5 + 7 = 23.5. Without
    // the box overrides (prev_depth 3, height 7) this would be 21.5, as in
    // `justified_stretch_uses_total_fit_numbers`'s second baseline -- the 2pt
    // difference is exactly the extra depth 'y' contributed to line 0.
    assert!(close(l1.baseline_y, 23.5));
    assert!(close(out.height, 23.5 + 3.0));
}

/// Same input, same output: the whole `Lines`/`Pages` structures compare equal
/// across two runs (no hash-map iteration, no randomness, no timing).
#[test]
fn layout_is_deterministic() {
    let it = items(
        &Core14Times::ROMAN,
        &ExplicitDiscretionary,
        "The quick brown fox jumps over the lazy dog and keeps re\\-run\\-ning until the para\\-graph wraps several times.",
    );
    let p = params(120.0);
    let a = layout_paragraph(&it, &p).unwrap();
    let b = layout_paragraph(&it, &p).unwrap();
    assert_eq!(a, b);
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
    let pa = layout_pages(&[ParagraphBlock::body(a.clone())], &small_page());
    let pb = layout_pages(&[ParagraphBlock::body(b)], &small_page());
    assert_eq!(pa, pb);
}

/// `\emergencystretch`: width 55, words 20pt, Times-like glue. "aaaa bbbb" is
/// 12.5pt short over 1.5pt stretch (infinite badness) and "aaaa bbbb cccc" is
/// 10pt over with 1.2pt shrink, so passes 1 and 2 fail. With
/// `emergency_stretch = 10`, pass 3 sees 11.5pt of stretch: ratio 1.087,
/// TeX badness (tex.web 108) r = 819200sp x 297 / 753664sp = 322,
/// (322^3 + 2^17) / 2^18 = 127 <= 200 (100 x 1.284 would round to 128), and
/// the paragraph is 2 + 2 words.
/// The set line still only has its real 1.5pt of stretch, so TeX's hpack sets
/// it to the full measure at ratio 12.5/1.5 and it is reported "Underfull \hbox (badness 10000)" citing both boxes.
#[test]
fn emergency_stretch_enables_a_third_pass_and_reports_underfull() {
    let it = items(&TestFont::TIMES_LIKE, &NoHyphenation, "aaaa bbbb cccc dddd");
    let without = layout_paragraph(&it, &params(55.0)).unwrap();
    assert_eq!(without.stats.pass, 2);
    assert!(!without.stats.overfull.is_empty() || !without.stats.underfull.is_empty());
    let p = LineBreakParams {
        emergency_stretch: 10.0,
        ..params(55.0)
    };
    let with = layout_paragraph(&it, &p).unwrap();
    assert_eq!(with.stats.pass, 3);
    assert!(with.stats.emergency_pass_used);
    assert_eq!(with.lines.len(), 2);
    assert_eq!(with.lines[0].runs.len(), 2);
    assert!(close(with.breaks[0].ratio, 12.5 / 11.5));
    assert_eq!(with.breaks[0].badness, 127.0);
    // hpack sets the chosen line with its real 1.5pt of stretch: glue_set =
    // 12.5/1.5, a 15pt space, and the line is exactly the 55pt measure.
    assert!(close(with.lines[0].runs[1].x, 20.0 + 2.5 + 12.5));
    assert!(close(with.lines[0].ratio, 12.5 / 1.5));
    assert!(close(with.lines[0].set_width, 55.0));
    assert_eq!(with.lines[0].badness, 10000.0);
    assert!(with.stats.overfull.is_empty());
    assert_eq!(with.diagnostics.len(), 1);
    let d = &with.diagnostics[0];
    assert_eq!(d.severity, Severity::Warning);
    assert_eq!(d.kind, DiagnosticKind::Underfull { badness: 10000.0 });
    assert_eq!(d.line, 0);
    assert_eq!(d.boxes, vec![0..4, 5..9]);
    assert_eq!(d.source, Some(0..9));
    assert!(
        d.message
            .starts_with("Underfull \\hbox (badness 10000) in paragraph, line 1")
    );
    assert!(d.recovery.is_some());
}

/// Overfull diagnostics obey `\hfuzz` and cite the exact box: the 40pt word
/// "yyyyzzzz" (source bytes 5..13) on a 35pt measure is 5pt too wide.
#[test]
fn overfull_diagnostic_cites_the_offending_box_and_respects_hfuzz() {
    let it = items(&TestFont::TIMES_LIKE, &NoHyphenation, "xxxx yyyyzzzz");
    let out = layout_paragraph(&it, &params(35.0).ragged()).unwrap();
    assert_eq!(out.diagnostics.len(), 1);
    let d = &out.diagnostics[0];
    assert_eq!(d.kind, DiagnosticKind::Overfull { excess: 5.0 });
    assert_eq!(d.line, 1);
    assert_eq!(d.boxes, vec![5..13]);
    assert_eq!(d.source, Some(5..13));
    assert!(
        d.message
            .starts_with("Overfull \\hbox (5.000pt too wide) in paragraph, line 2")
    );
    // A generous \hfuzz silences it; the overfull stat remains.
    let p = LineBreakParams {
        hfuzz: 6.0,
        ..params(35.0).ragged()
    };
    let out = layout_paragraph(&it, &p).unwrap();
    assert!(out.diagnostics.is_empty());
    assert_eq!(out.stats.overfull.len(), 1);
}

/// `\hbadness`: with `\tolerance 10000` (sloppy) a 12.5pt-short line over
/// 1.5pt of stretch is accepted (badness 10000) and reported underfull; raising
/// `\hbadness` to 10000 silences the report.
#[test]
fn underfull_diagnostic_respects_hbadness() {
    let it = items(&TestFont::TIMES_LIKE, &NoHyphenation, "aaaa bbbb cccc dddd");
    let p = LineBreakParams {
        pretolerance: -1.0,
        tolerance: 10000.0,
        ..params(55.0)
    };
    let out = layout_paragraph(&it, &p).unwrap();
    assert_eq!(out.lines.len(), 2);
    assert_eq!(out.diagnostics.len(), 1);
    assert_eq!(
        out.diagnostics[0].kind,
        DiagnosticKind::Underfull { badness: 10000.0 }
    );
    assert_eq!(out.diagnostics[0].boxes, vec![0..4, 5..9]);
    let p = LineBreakParams {
        hbadness: 10000.0,
        ..p
    };
    assert!(layout_paragraph(&it, &p).unwrap().diagnostics.is_empty());
}

/// Automatic hyphenation with penalties: only a word after glue gets
/// automatic points (TeX never hyphenates a paragraph's first word). Justified
/// on an 85pt measure, "documentation" (59.44pt at 10pt Times) alone on a line
/// has no glue to stretch, so pass 1 fails and pass 2 breaks at "docu-":
/// 59.44 + 2.5 + 19.44 + 3.33 = 84.71pt, 0.29pt short (badness 1). With
/// `\hyphenpenalty 10000` no discretionary is legal; a lone "documentation"
/// line has infinite badness (no glue) and is never feasible below
/// `\tolerance 10000`, so — exactly as TeX does — the final pass sets both
/// words on one overfull line (121.38pt natural, set at full 0.6pt shrink:
/// 35.78pt too wide) and reports it.
#[test]
fn hyphen_penalty_and_first_word_rule() {
    let hyph = LiangHyphenator::en_us_subset();
    let text = "documentation documentation";
    let mut b = ParagraphBuilder::new(&hyph);
    b.text(&Core14Times::ROMAN, 10.0, text, 0).unwrap();
    let it = b.finish(Glue::fil());
    let hyphen_points = it
        .iter()
        .filter(|i| matches!(i, Item::Penalty(p) if p.flagged))
        .count();
    assert_eq!(
        hyphen_points, 4,
        "only the second word gets doc-u-men-ta-tion"
    );
    let out = layout_paragraph(&it, &params(85.0)).unwrap();
    assert_eq!(out.stats.pass, 2);
    assert_eq!(out.lines.len(), 2);
    assert!(out.lines[0].hyphenated);
    assert!(out.lines[0].runs.last().unwrap().is_hyphen);
    assert!(close(out.lines[0].natural_width, 84.71));
    assert_eq!(out.breaks[0].badness, 1.0);
    // The hyphen's cluster is empty (automatic point) at the break offset.
    assert_eq!(out.lines[0].runs.last().unwrap().glyphs[0].cluster, 18..18);
    let mut b = ParagraphBuilder::new(&hyph);
    b.hyphen_penalty = 10000;
    b.text(&Core14Times::ROMAN, 10.0, text, 0).unwrap();
    let it = b.finish(Glue::fil());
    let out = layout_paragraph(&it, &params(85.0)).unwrap();
    assert_eq!(out.stats.hyphenated_lines, 0);
    assert_eq!(out.lines.len(), 1);
    assert_eq!(out.diagnostics.len(), 1);
    match out.diagnostics[0].kind {
        DiagnosticKind::Overfull { excess } => assert!(close(excess, 121.38 - 0.6 - 85.0)),
        ref k => panic!("expected overfull, got {k:?}"),
    }
    // Boxes are the shaped fragments: the second word is split at its four
    // (forbidden) discretionaries, so its provenance is five ranges.
    assert_eq!(
        out.diagnostics[0].boxes,
        vec![0..13, 14..17, 17..18, 18..21, 21..23, 23..27]
    );
}

// ---------------------------------------------------------------------------
// FT-019 rev 4 (b): the demerit parameters, one minimal case each. In every
// case the alternative layouts are enumerated by hand in the comment; the
// parameter under test is the only thing that changes between the two runs.
// `TestFont` glyphs are 5pt, spaces 2.5pt; a `-` hyphen is 5pt.
// ---------------------------------------------------------------------------

/// Words on each line as text, a discretionary hyphen shown as `-`.
fn line_words(text: &str, line: &Line) -> String {
    let mut s = String::new();
    let mut last_end = None;
    for r in &line.runs {
        if r.is_hyphen {
            s.push('-');
        } else {
            // A gap in the source is a space unless it is only a `\-` marker.
            if let Some(e) = last_end
                && text[e..r.source.start].contains(char::is_whitespace)
            {
                s.push(' ');
            }
            s.push_str(&text[r.source.clone()]);
        }
        last_end = Some(r.source.end);
    }
    s
}

fn line_texts(text: &str, out: &Lines) -> Vec<String> {
    out.lines.iter().map(|l| line_words(text, l)).collect()
}

/// `\linepenalty`. Words A=20 B=20 C=10 D=20 E=20 F=5 pt, measure 52.5pt,
/// interword stretch 150pt (so any line up to 25pt short has badness 0) and
/// shrink 2.7pt.
///
/// Feasible lines: "A B" (42.5, b 0), "A B C" (55: 2.5pt over 5.4pt shrink,
/// ratio -0.46296, 100·r³ = 9.92 -> b 10, Decent), "C D" (32.5, b 0),
/// "C D E" (55, b 10), "D E" (42.5, b 0), last lines "E F" 27.5 / "D E F" 50 /
/// "F" (fil, b 0). A single non-last word has no glue (infinite badness);
/// "A B C D" (77.5) and "C D E F" (62.5) exceed the shrink.
///
/// Layouts and demerits, l = \linepenalty:
///   X  "A B" / "C D" / "E F"        3·l²
///   Y  "A B C" / "D E F"            (l+10)² + l²
///   Z  "A B" / "C D E" / "F"        l² + (l+10)² + l²   (and "A B C"/"D E"/"F", same)
/// l = 10:  X 300 < Y 500 < Z 600  -> three lines.
/// l = 100: Y 22100 < X 30000 < Z 32100 -> two lines: the higher line penalty
/// buys the shrunk line to save a line. All lines Decent, so \adjdemerits is
/// never charged; pass 1 both times (badness <= 100).
#[test]
fn linepenalty_trades_a_line_for_badness() {
    let font = TestFont {
        stretch: 15_000.0,
        shrink: 270.0,
    };
    let text = "aaaa bbbb cc dddd eeee f";
    let it = items(&font, &NoHyphenation, text);
    let out = layout_paragraph(&it, &params(52.5)).unwrap();
    assert_eq!(out.stats.pass, 1);
    assert_eq!(line_texts(text, &out), ["aaaa bbbb", "cc dddd", "eeee f"]);
    assert!(close(out.stats.total_demerits, 300.0));
    assert_eq!(out.lines[0].badness, 0.0);
    assert_eq!(out.lines[1].badness, 0.0);

    let mut p = params(52.5);
    p.line_penalty = 100.0;
    let out = layout_paragraph(&it, &p).unwrap();
    assert_eq!(out.stats.pass, 1);
    assert_eq!(line_texts(text, &out), ["aaaa bbbb cc", "dddd eeee f"]);
    assert!(close(out.lines[0].ratio, -2.5 / 5.4));
    assert_eq!(out.lines[0].badness, 10.0);
    assert_eq!(out.breaks[0].fitness, Fitness::Decent);
    assert!(close(out.breaks[0].demerits, 110.0 * 110.0));
    assert!(close(out.stats.total_demerits, 22_100.0));
    // The shrunk spaces: 2.5 - 0.46296 x 2.7 = 1.25pt each.
    assert_xs(&out.lines[0], &[0.0, 21.25, 42.5]);
}

/// `\adjdemerits`. Words A=15 B=15 C=20 D=10 E=25 F=20 pt, measure 51.55pt,
/// stretch 24pt, shrink 2.95pt per space.
///
/// Feasible lines: "A B" 32.5 (19.05 short / 24: r 0.79375, b 50, Loose),
/// "A B C" 55 (3.45 over 5.9: r -0.58475, b 20, Tight), "C D" 32.5 (b 50,
/// Loose), "D E" 37.5 (14.05 / 24: r 0.58542, b 20, Loose), last lines
/// "E F" 47.5 and "F" (b 0, Decent). Infeasible: "A B C D" 67.5, "C D E" 60
/// and "D E F" 60 (even as a last line: 8.45 over 5.9pt shrink), any lone
/// non-last word.
///
/// Layouts, a = \adjdemerits:
///   P  "A B" / "C D" / "E F"     60² + 60² + 10²          = 7300  (Loose, Loose, Decent)
///   Q  "A B C" / "D E" / "F"     30² + 30² + 10² + a      = 1900 + a  (Tight -> Loose: classes differ by 2)
/// a = 10000: P 7300 < Q 11900. a = 0: Q 1900 < P 7300.
#[test]
fn adjdemerits_avoids_a_tight_loose_pair() {
    let font = TestFont {
        stretch: 2_400.0,
        shrink: 295.0,
    };
    let text = "aaa bbb cccc dd eeeee ffff";
    let it = items(&font, &NoHyphenation, text);
    let out = layout_paragraph(&it, &params(51.55)).unwrap();
    assert_eq!(out.stats.pass, 1);
    assert_eq!(line_texts(text, &out), ["aaa bbb", "cccc dd", "eeeee ffff"]);
    assert_eq!(out.lines[0].badness, 50.0);
    assert_eq!(out.breaks[0].fitness, Fitness::Loose);
    assert_eq!(out.breaks[1].fitness, Fitness::Loose);
    assert!(close(out.stats.total_demerits, 7300.0));

    let mut p = params(51.55);
    p.adj_demerits = 0.0;
    let out = layout_paragraph(&it, &p).unwrap();
    assert_eq!(line_texts(text, &out), ["aaa bbb cccc", "dd eeeee", "ffff"]);
    assert_eq!(out.lines[0].badness, 20.0);
    assert_eq!(out.breaks[0].fitness, Fitness::Tight);
    assert_eq!(out.lines[1].badness, 20.0);
    assert_eq!(out.breaks[1].fitness, Fitness::Loose);
    assert!(close(out.stats.total_demerits, 1900.0));
    // With the default \adjdemerits the same layout would cost 11900: the
    // cumulative demerits through the second break say so.
    assert!(close(out.breaks[1].demerits, 1800.0));
}

/// `\doublehyphendemerits`. Text "aa bb\-bb cc\-cc dd ee ff", measure 30pt,
/// stretch 100pt (short lines have badness 0), shrink 2.96pt, default
/// `\hyphenpenalty` 50.
///
/// Feasible lines: "aa bb-" 27.5 (b 0), "bb cc-" 27.5 (b 0), "bb cccc" 32.5
/// (2.5 over 2.96: r -0.84459, 100·r³ = 60.2 -> b 60, Tight), "cc dd" 22.5,
/// "cc dd ee" 35 (5 over 5.92, b 60), "dd ee" 22.5, "aa bbbb" 32.5 (b 60),
/// "cccc dd" 32.5 (b 60); last lines "ee ff" 22.5 / "ff" (b 0) and
/// "dd ee ff" 35 (shrunk, b 60). "cccc dd ee" 45 and any lone non-last word
/// are infeasible.
///
/// Layouts, h = 50² per hyphenated line, d = \doublehyphendemerits:
///   H  "aa bb-" / "bb cc-" / "cc dd" / "ee ff"   (10²+h) + (10²+h+d) + 10² + 10² = 5400 + d
///   N  "aa bb-" / "bb cccc" / "dd ee" / "ff"     (10²+h) + 70² + 10² + 10²      = 7700
///   N' "aa bb-" / "bb cccc" / "dd ee ff"         (10²+h) + 70² + 70²            = 12400
///   O  "aa bbbb" / "cccc dd" / "ee ff"           70² + 70² + 10²                = 9900
///   H' "aa bb-" / "bb cc-" / "cc dd ee" / "ff"   2600 + 2600 + d + 4900 + 100   = 10200 + d
/// d = 10000: N 7700 wins. d = 0: H 5400 wins. No layout has a hyphen on its
/// penultimate line, so \finalhyphendemerits never applies.
#[test]
fn doublehyphendemerits_avoids_consecutive_hyphens() {
    let font = TestFont {
        stretch: 10_000.0,
        shrink: 296.0,
    };
    let text = "aa bb\\-bb cc\\-cc dd ee ff";
    let it = items(&font, &ExplicitDiscretionary, text);
    let out = layout_paragraph(&it, &params(30.0)).unwrap();
    assert_eq!(out.stats.pass, 1);
    assert_eq!(line_texts(text, &out), ["aa bb-", "bb cccc", "dd ee", "ff"]);
    assert_eq!(out.stats.hyphenated_lines, 1);
    assert_eq!(out.lines[1].badness, 60.0);
    assert!(close(out.stats.total_demerits, 7700.0));

    let mut p = params(30.0);
    p.double_hyphen_demerits = 0.0;
    let out = layout_paragraph(&it, &p).unwrap();
    assert_eq!(
        line_texts(text, &out),
        ["aa bb-", "bb cc-", "cc dd", "ee ff"]
    );
    assert_eq!(out.stats.hyphenated_lines, 2);
    assert!(out.lines[0].hyphenated && out.lines[1].hyphenated);
    assert!(close(out.stats.total_demerits, 5400.0));
}

/// `\finalhyphendemerits`. Text "aa bb cc\-cc dd", measure 40pt, stretch
/// 22pt, shrink 1pt per space, default `\hyphenpenalty` 50.
///
/// Feasible lines: "aa bb cc-" 40 (exact, b 0), "aa bb" 22.5 (17.5 / 22:
/// r 0.79545, 100·r³ = 50.3 -> b 50, Loose); last lines "cc dd" 22.5 and
/// "cccc dd" 32.5 (b 0). "aa bb cccc" 45 exceeds the 2pt shrink; a lone
/// "cc-" line has no glue.
///
/// Layouts, f = \finalhyphendemerits:
///   F  "aa bb cc-" / "cc dd"     (10² + 50² + f) + 10²  = 2700 + f
///   G  "aa bb" / "cccc dd"       60² + 10²              = 3700
/// f = 5000: G 3700 < F 7700. f = 0: F 2700 < G 3700.
#[test]
fn finalhyphendemerits_avoids_a_hyphen_before_the_last_line() {
    let font = TestFont {
        stretch: 2_200.0,
        shrink: 100.0,
    };
    let text = "aa bb cc\\-cc dd";
    let it = items(&font, &ExplicitDiscretionary, text);
    let out = layout_paragraph(&it, &params(40.0)).unwrap();
    assert_eq!(out.stats.pass, 1);
    assert_eq!(line_texts(text, &out), ["aa bb", "cccc dd"]);
    assert_eq!(out.lines[0].badness, 50.0);
    assert!(close(out.lines[0].ratio, 17.5 / 22.0));
    assert!(close(out.stats.total_demerits, 3700.0));

    let mut p = params(40.0);
    p.final_hyphen_demerits = 0.0;
    let out = layout_paragraph(&it, &p).unwrap();
    assert_eq!(line_texts(text, &out), ["aa bb cc-", "cc dd"]);
    assert!(out.lines[0].hyphenated);
    assert_eq!(out.lines[0].badness, 0.0);
    assert!(close(out.lines[0].natural_width, 40.0));
    assert!(
        close(out.stats.total_demerits, 2700.0),
        "total {} breaks {:?}",
        out.stats.total_demerits,
        out.breaks
    );
}
