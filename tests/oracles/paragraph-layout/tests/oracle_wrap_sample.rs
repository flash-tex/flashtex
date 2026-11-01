//! pdflatex oracle comparison for `tools/native-validation/oracle-samples/wrap-sample.tex`
//! (branch `agent/mac-validation/native-verification`), variant
//! `C-times12-1in-ragged`.
//!
//! Reference data: `tools/native-validation/reports/oracle-20260912T050958Z.json`
//! on that branch, result `wrap-sample` / `C-times12-1in-ragged`, field
//! `comparisons[de1020c].deltas[*].{word, oracle_x, oracle_bottom, oracle_line_start}`.
//! Oracle engine: `/Library/TeX/texbin/pdflatex` = pdfTeX 3.141592653-2.6-1.40.29
//! (TeX Live 2026, BasicTeX), fonts `times` package (psnfss `ptmr8t`/`ptmb8t`/
//! `ptmri8t`, URW Nimbus Roman outlines with Adobe Times metrics), T1 encoding,
//! `[12pt]{article}`, `geometry margin=1in`, `\parindent 0pt`, `\raggedright`,
//! `\hyphenpenalty=\exhyphenpenalty=10000`, `secnumdepth 0`, `\pagestyle{empty}`.
//! Word boxes were extracted with PDFKit (`oracle_extract.swift`): `x` is the
//! glyph-box left edge and `bottom` the glyph-box bottom (baseline + font
//! descent) from the page top, in PDF points (bp, 1/72 in). There is no raster
//! DPI involved: the comparison is on PDF coordinates.
//!
//! Unit conversion: pdflatex works in TeX points (1/72.27 in) and the PDF is in
//! bp, so every oracle coordinate is `72 + tex_pt * 72/72.27` for x and the
//! same scale for y. The layout below runs in TeX points with the oracle's
//! `\hsize` = 469.75499pt and is converted with [`tex_pt_to_bp`] before
//! comparison.

use flashtex_paragraph_layout::core14::Core14Times;
use flashtex_paragraph_layout::*;

/// (word, oracle x, oracle bottom, oracle starts line) — 102 words.
const ORACLE: &[(&str, f64, f64, bool)] = &[
    ("Wrapping", 72.0, 87.553, true),
    ("and", 151.879, 87.553, false),
    ("accents", 183.935, 87.553, false),
    ("A", 72.0, 113.357, true),
    ("naïve", 83.62, 113.357, false),
    ("reader", 112.325, 113.357, false),
    ("at", 145.178, 113.357, false),
    ("the", 156.798, 113.357, false),
    ("café", 174.396, 113.357, false),
    ("expects", 197.29, 113.357, false),
    ("the", 235.954, 113.357, false),
    ("layout", 253.552, 113.357, false),
    ("to", 286.428, 113.357, false),
    ("follow", 298.718, 113.357, false),
    ("the", 332.623, 113.357, false),
    ("source.", 350.221, 113.357, false),
    ("The", 388.119, 113.357, false),
    ("bold", 409.699, 113.357, false),
    ("phrase", 435.283, 113.357, false),
    ("and", 472.81, 113.357, false),
    ("the", 493.062, 113.357, false),
    ("emphasised", 72.0, 127.802, true),
    ("phrase", 130.772, 127.802, false),
    ("keep", 166.123, 127.802, false),
    ("their", 191.564, 127.802, false),
    ("words", 216.467, 127.802, false),
    ("in", 248.554, 127.802, false),
    ("order.", 260.844, 127.802, false),
    ("This", 72.0, 142.248, true),
    ("paragraph", 96.245, 142.248, false),
    ("is", 147.031, 142.248, false),
    ("deliberately", 157.994, 142.248, false),
    ("long", 217.423, 142.248, false),
    ("so", 241.668, 142.248, false),
    ("that", 255.285, 142.248, false),
    ("a", 276.207, 142.248, false),
    ("real", 284.504, 142.248, false),
    ("TeX", 305.413, 142.248, false),
    ("engine", 328.81, 142.248, false),
    ("and", 363.671, 142.248, false),
    ("the", 383.923, 142.248, false),
    ("FlashTeX", 401.521, 142.248, false),
    ("compiler", 450.824, 142.248, false),
    ("both", 496.314, 142.248, false),
    ("have", 72.0, 156.694, true),
    ("to", 97.142, 156.694, false),
    ("break", 109.432, 156.694, false),
    ("it", 138.973, 156.694, false),
    ("into", 148.609, 156.694, false),
    ("several", 170.2, 156.694, false),
    ("lines,", 206.568, 156.694, false),
    ("which", 235.129, 156.694, false),
    ("lets", 267.336, 156.694, false),
    ("the", 286.931, 156.694, false),
    ("comparison", 304.529, 156.694, false),
    ("tool", 363.3, 156.694, false),
    ("report", 384.891, 156.694, false),
    ("where", 416.429, 156.694, false),
    ("each", 448.625, 156.694, false),
    ("line", 473.515, 156.694, false),
    ("starts", 494.437, 156.694, false),
    ("and", 522.663, 156.694, false),
    ("how", 72.0, 171.14, true),
    ("far", 95.277, 171.14, false),
    ("every", 111.416, 171.14, false),
    ("word", 140.479, 171.14, false),
    ("drifts", 167.917, 171.14, false),
    ("from", 196.143, 171.14, false),
    ("the", 222.373, 171.14, false),
    ("oracle", 239.971, 171.14, false),
    ("position.", 272.166, 171.14, false),
    ("It", 317.392, 171.14, false),
    ("keeps", 327.686, 171.14, false),
    ("going", 357.777, 171.14, false),
    ("with", 388.0, 171.14, false),
    ("ordinary", 412.245, 171.14, false),
    ("prose,", 455.738, 171.14, false),
    ("a", 487.611, 171.14, false),
    ("few", 495.907, 171.14, false),
    ("longer", 72.0, 185.586, true),
    ("words", 105.534, 185.586, false),
    ("such", 137.622, 185.586, false),
    ("as", 162.525, 185.586, false),
    ("verification", 175.472, 185.586, false),
    ("and", 232.738, 185.586, false),
    ("reproducibility,", 252.99, 185.586, false),
    ("and", 329.252, 185.586, false),
    ("finally", 349.504, 185.586, false),
    ("ends", 383.05, 185.586, false),
    ("here.", 407.953, 185.586, false),
    ("Short", 72.0, 200.032, true),
    ("last", 100.896, 200.032, false),
    ("paragraph", 120.49, 200.032, false),
    ("with", 171.276, 200.032, false),
    ("fine", 195.521, 200.032, false),
    ("coffee", 216.443, 200.032, false),
    ("and", 248.997, 200.032, false),
    ("an", 269.249, 200.032, false),
    ("em", 283.523, 200.032, false),
    ("dash", 301.121, 200.032, false),
    ("—", 326.024, 200.032, false),
    ("done.", 340.968, 200.032, false),
];

/// `\hsize` of the oracle: 8.5in - 2in = 6.5in = 469.75499pt.
const HSIZE: f64 = 469.75499;
/// `\normalsize` in the 12pt article.
const BODY: f64 = 12.0;
/// `\Large` in the 12pt article (`\section` heading font, bold).
const LARGE: f64 = 17.28;

struct Seg(Core14Times, f64, &'static str);

fn build(segs: &[Seg]) -> Vec<Item> {
    let h = NoHyphenation;
    let mut b = ParagraphBuilder::new(&h);
    let mut off = 0;
    for Seg(font, size, text) in segs {
        b.text(font, *size, text, off).unwrap();
        off += text.len();
    }
    b.finish(Glue::fil())
}

fn paragraphs() -> Vec<Vec<Item>> {
    vec![
        build(&[Seg(Core14Times::BOLD, LARGE, "Wrapping and accents")]),
        build(&[
            Seg(
                Core14Times::ROMAN,
                BODY,
                "A naïve reader at the café expects the layout to follow the source.\nThe ",
            ),
            Seg(Core14Times::BOLD, BODY, "bold phrase"),
            Seg(Core14Times::ROMAN, BODY, " and the "),
            Seg(Core14Times::ITALIC, BODY, "emphasised phrase"),
            Seg(Core14Times::ROMAN, BODY, " keep their words in order."),
        ]),
        build(&[Seg(
            Core14Times::ROMAN,
            BODY,
            "This paragraph is deliberately long so that a real TeX engine and the FlashTeX compiler both have to break it into several lines, which lets the comparison tool report where each line starts and how far every word drifts from the oracle position. It keeps going with ordinary prose, a few longer words such as verification and reproducibility, and finally ends here.",
        )]),
        build(&[Seg(
            Core14Times::ROMAN,
            BODY,
            "Short last paragraph with fine coffee and an em dash \u{2014} done.",
        )]),
    ]
}

struct Placed {
    x_bp: f64,
    bottom_bp: f64,
    line_start: bool,
}

fn run(algorithm: Algorithm) -> (Vec<Placed>, Vec<Lines>) {
    let params = LineBreakParams::article_12pt_letter_1in()
        .with_width(HSIZE)
        .ragged();
    let params = LineBreakParams {
        algorithm,
        ..params
    };
    let paras = paragraphs();
    let mut blocks = Vec::new();
    let mut all_lines = Vec::new();
    for (i, items) in paras.iter().enumerate() {
        let lines = layout_paragraph(items, &params).unwrap();
        all_lines.push(lines.clone());
        blocks.push(if i == 0 {
            ParagraphBlock::section_heading_12pt(lines)
        } else {
            ParagraphBlock::body(lines)
        });
    }
    let page_params = PageParams::article_12pt_letter_1in_tex_pt();
    let pages = layout_pages(&blocks, &page_params);
    assert_eq!(
        pages.pages.len(),
        1,
        "wrap-sample fits on one page in pdflatex"
    );
    assert!(pages.overflow.is_empty());
    let mut placed = Vec::new();
    let mut last_line: Option<f64> = None;
    for r in &pages.pages[0].runs {
        // Descent used by PDFKit's glyph box = font descender at the run size.
        let desc = if r.font == Core14Times::BOLD.font_id() {
            205.0
        } else {
            217.0
        } * r.size
            / 1000.0;
        let line_start = last_line != Some(r.baseline_y);
        last_line = Some(r.baseline_y);
        placed.push(Placed {
            x_bp: tex_pt_to_bp(r.x),
            bottom_bp: tex_pt_to_bp(r.baseline_y + desc),
            line_start,
        });
    }
    (placed, all_lines)
}

struct Summary {
    line_start_matches: usize,
    dx_mean: f64,
    dx_max: f64,
    dy_mean: f64,
    dy_max: f64,
}

fn summarize(placed: &[Placed], verbose: bool) -> Summary {
    assert_eq!(
        placed.len(),
        ORACLE.len(),
        "word count must equal the oracle's 102 words"
    );
    let mut s = Summary {
        line_start_matches: 0,
        dx_mean: 0.0,
        dx_max: 0.0,
        dy_mean: 0.0,
        dy_max: 0.0,
    };
    if verbose {
        println!(
            "| word | oracle x | ours x | dx | oracle bottom | ours bottom | dy | line start o/u |"
        );
        println!("|---|---|---|---|---|---|---|---|");
    }
    for (p, (word, ox, ob, ostart)) in placed.iter().zip(ORACLE) {
        let dx = p.x_bp - ox;
        let dy = p.bottom_bp - ob;
        if p.line_start == *ostart {
            s.line_start_matches += 1;
        }
        s.dx_mean += dx.abs();
        s.dy_mean += dy.abs();
        s.dx_max = s.dx_max.max(dx.abs());
        s.dy_max = s.dy_max.max(dy.abs());
        if verbose {
            println!(
                "| {word} | {ox:.3} | {:.3} | {dx:+.3} | {ob:.3} | {:.3} | {dy:+.3} | {}/{} |",
                p.x_bp,
                p.bottom_bp,
                if *ostart { "yes" } else { "no" },
                if p.line_start { "yes" } else { "no" },
            );
        }
    }
    s.dx_mean /= placed.len() as f64;
    s.dy_mean /= placed.len() as f64;
    if verbose {
        println!();
        println!(
            "line starts matched: {}/{}; mean|dx| {:.3} bp; max|dx| {:.3} bp; mean|dy| {:.3} bp; max|dy| {:.3} bp",
            s.line_start_matches,
            placed.len(),
            s.dx_mean,
            s.dx_max,
            s.dy_mean,
            s.dy_max
        );
    }
    s
}

/// Golden: total-fit ragged reproduces every one of pdflatex's 8 line starts
/// (102/102 words agree) and every word is within 0.01 bp horizontally and
/// 0.15 bp vertically. The numbers are explained in `docs/comparison.md`.
#[test]
fn total_fit_ragged_matches_pdflatex_line_starts() {
    let (placed, lines) = run(Algorithm::TotalFit);
    let s = summarize(&placed, true);
    // Lines per paragraph: heading 1, para 1: 2, para 2: 4, para 3: 1 (oracle: 8 line starts).
    let counts: Vec<usize> = lines.iter().map(|l| l.lines.len()).collect();
    assert_eq!(counts, vec![1, 2, 4, 1]);
    assert_eq!(s.line_start_matches, 102);
    assert!(s.dx_max < 0.01, "max|dx| {} bp", s.dx_max);
    assert!(s.dy_max < 0.15, "max|dy| {} bp", s.dy_max);
    // Every line has zero badness in ragged mode (fil rightskip): TeX's
    // total-fit therefore minimises the line count and ties like first-fit.
    for l in &lines {
        assert_eq!(l.stats.pass, 1);
        assert!(l.lines.iter().all(|line| line.badness == 0.0));
    }
}

/// First-fit gives the same breaks on this sample (see comparison.md for why
/// the two coincide here and when they would not).
#[test]
fn first_fit_ragged_matches_total_fit_on_this_sample() {
    let (a, _) = run(Algorithm::TotalFit);
    let (b, _) = run(Algorithm::FirstFit);
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.line_start, y.line_start);
        assert!((x.x_bp - y.x_bp).abs() < 1e-9);
    }
}
