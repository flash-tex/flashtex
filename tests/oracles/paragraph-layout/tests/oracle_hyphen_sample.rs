//! pdflatex oracle comparison for `docs/hyphen-sample.tex`: justified text with
//! TeX's default hyphenation (`\language` english = hyphen.tex), `times`,
//! 12pt, 1in margins, `\parindent 0pt`.
//!
//! Oracle: `/Library/TeX/texbin/pdflatex` = pdfTeX 3.141592653-2.6-1.40.29
//! (TeX Live 2026), compiled once on 2026-09-12 (oracle only; log had no
//! overfull/underfull boxes). Word boxes extracted with PDFKit via the
//! native-verification branch's `oracle_extract.swift` (x = glyph-box left,
//! bottom = glyph-box bottom, PDF points from the page top-left; no DPI).
//! The oracle typesets `engine's` with a curly quoteright; the AFM width of
//! `quoteright` and ASCII `quotesingle` differ (333 vs 180), so the source
//! below uses U+2019 to give the same measure.

use flashtex_paragraph_layout::core14::Core14Times;
use flashtex_paragraph_layout::*;

const PARA1: &str = "Reproducibility and verification are the characteristic responsibilities of any typographical documentation effort, and the international community expects deliberately measured compilation results rather than approximations of representative behaviour. Hyphenation boundaries determine whether an unremarkable paragraph fits comfortably or overflows; consequently the implementation compares its automatically generated discretionary breaks against the reference engine\u{2019}s independently computed positions.";
const PARA2: &str = "Deliberately unbalanced paragraphs demonstrate emergency stretchability: extraordinarily incomprehensible terminology, uncharacteristically interdisciplinary collaboration, and counterproductive institutionalization complicate justification considerably, particularly when consecutive polysyllabic constructions eliminate conventional opportunities for satisfactory interword adjustment throughout the measurement.";

/// (word as PDFKit reports it, oracle x, oracle bottom, starts a line) — 92 words.
const ORACLE: &[(&str, f64, f64, bool)] = &[
    ("Reproducibility", 72.000, 86.537, true),
    ("and", 151.263, 86.537, false),
    ("verification", 172.723, 86.537, false),
    ("are", 231.207, 86.537, false),
    ("the", 250.013, 86.537, false),
    ("characteristic", 268.818, 86.537, false),
    ("responsibilities", 336.760, 86.537, false),
    ("of", 413.357, 86.537, false),
    ("any", 427.524, 86.537, false),
    ("typographical", 448.816, 86.537, false),
    ("doc-", 518.754, 86.537, false),
    ("umentation", 72.000, 100.983, true),
    ("effort,", 130.545, 100.983, false),
    ("and", 164.964, 100.983, false),
    ("the", 186.973, 100.983, false),
    ("international", 206.329, 100.983, false),
    ("community", 271.508, 100.983, false),
    ("expects", 330.722, 100.983, false),
    ("deliberately", 371.143, 100.983, false),
    ("measured", 432.318, 100.983, false),
    ("compilation", 482.876, 100.983, false),
    ("results", 72.000, 115.429, true),
    ("rather", 105.941, 115.429, false),
    ("than", 136.558, 115.429, false),
    ("approximations", 159.871, 115.429, false),
    ("of", 236.994, 115.429, false),
    ("representative", 249.678, 115.429, false),
    ("behaviour.", 318.994, 115.429, false),
    ("Hyphenation", 372.506, 115.429, false),
    ("boundaries", 437.004, 115.429, false),
    ("determine", 492.189, 115.429, false),
    ("whether", 72.000, 129.875, true),
    ("an", 114.680, 129.875, false),
    ("unremarkable", 130.150, 129.875, false),
    ("paragraph", 200.052, 129.875, false),
    ("fits", 252.021, 129.875, false),
    ("comfortably", 270.815, 129.875, false),
    ("or", 333.436, 129.875, false),
    ("overflows;", 347.567, 129.875, false),
    ("consequently", 402.155, 129.875, false),
    ("the", 469.415, 129.875, false),
    ("implemen-", 488.196, 129.875, false),
    ("tation", 72.000, 144.320, true),
    ("compares", 103.813, 144.320, false),
    ("its", 154.192, 144.320, false),
    ("automatically", 170.069, 144.320, false),
    ("generated", 239.731, 144.320, false),
    ("discretionary", 290.780, 144.320, false),
    ("breaks", 357.765, 144.320, false),
    ("against", 393.547, 144.320, false),
    ("the", 431.935, 144.320, false),
    ("reference", 451.123, 144.320, false),
    ("engine’s", 500.152, 144.320, false),
    ("independently", 72.000, 158.766, true),
    ("computed", 142.727, 158.766, false),
    ("positions.", 192.867, 158.766, false),
    ("Deliberately", 72.000, 173.212, true),
    ("unbalanced", 133.988, 173.212, false),
    ("paragraphs", 191.325, 173.212, false),
    ("demonstrate", 246.677, 173.212, false),
    ("emergency", 308.008, 173.212, false),
    ("stretchability:", 362.954, 173.212, false),
    ("extraordinarily", 432.366, 173.212, false),
    ("incom-", 506.129, 173.212, false),
    ("prehensible", 72.000, 187.658, true),
    ("terminology,", 131.011, 187.658, false),
    ("uncharacteristically", 195.808, 187.658, false),
    ("interdisciplinary", 293.327, 187.658, false),
    ("collaboration,", 375.590, 187.658, false),
    ("and", 445.803, 187.658, false),
    ("counterproduc-", 466.964, 187.658, false),
    ("tive", 72.000, 202.104, true),
    ("institutionalization", 92.754, 202.104, false),
    ("complicate", 185.718, 202.104, false),
    ("justification", 241.477, 202.104, false),
    ("considerably,", 301.229, 202.104, false),
    ("particularly", 367.903, 202.104, false),
    ("when", 426.316, 202.104, false),
    ("consecutive", 455.511, 202.104, false),
    ("poly-", 514.761, 202.104, false),
    ("syllabic", 72.000, 216.550, true),
    ("constructions", 113.018, 216.550, false),
    ("eliminate", 180.601, 216.550, false),
    ("conventional", 228.912, 216.550, false),
    ("opportunities", 293.841, 216.550, false),
    ("for", 360.766, 216.550, false),
    ("satisfactory", 378.531, 216.550, false),
    ("interword", 437.339, 216.550, false),
    ("adjustment", 487.527, 216.550, false),
    ("throughout", 72.000, 230.995, true),
    ("the", 127.460, 230.995, false),
    ("measurement.", 145.058, 230.995, false),
];

struct Word {
    text: String,
    x_bp: f64,
    bottom_bp: f64,
    line_start: bool,
    hyphenated: bool,
}

/// Lays out both paragraphs and flattens the page into PDFKit-style words:
/// consecutive runs on a line with no glue between them merge (a hyphenated
/// fragment plus its `-`), so the sequence is directly comparable.
fn ours() -> (Vec<Word>, Vec<Lines>) {
    let hyph = LiangHyphenator::en_us_subset();
    let params = LineBreakParams::article_12pt_letter_1in();
    let mut blocks = Vec::new();
    let mut all = Vec::new();
    let mut offset = 0;
    for text in [PARA1, PARA2] {
        let mut b = ParagraphBuilder::new(&hyph);
        b.text(&Core14Times::ROMAN, 12.0, text, offset).unwrap();
        let items = b.finish(Glue::fil());
        let lines = layout_paragraph(&items, &params).unwrap();
        all.push(lines.clone());
        blocks.push(ParagraphBlock::body(lines));
        offset += text.len() + 2;
    }
    let pages = layout_pages(&blocks, &PageParams::article_12pt_letter_1in_tex_pt());
    assert_eq!(pages.pages.len(), 1);
    let source = format!("{PARA1}\n\n{PARA2}");
    let mut words: Vec<Word> = Vec::new();
    let mut last_line: Option<f64> = None;
    let mut last_end: Option<(usize, f64)> = None; // (source end, x end) of previous run
    for r in &pages.pages[0].runs {
        let line_start = last_line != Some(r.baseline_y);
        let text = if r.is_hyphen {
            "-".to_string()
        } else {
            source[r.source.clone()].to_string()
        };
        // Source-contiguous runs are one word (a kern may sit between them).
        let glued = !line_start && last_end.is_some_and(|(e, _)| e == r.source.start);
        if (glued || r.is_hyphen) && !line_start {
            let w = words.last_mut().unwrap();
            w.text.push_str(&text);
            w.hyphenated |= r.is_hyphen;
        } else {
            words.push(Word {
                text,
                x_bp: tex_pt_to_bp(r.x),
                bottom_bp: tex_pt_to_bp(r.baseline_y + 217.0 * r.size / 1000.0),
                line_start,
                hyphenated: r.is_hyphen,
            });
        }
        last_line = Some(r.baseline_y);
        last_end = Some((r.source.end, r.x + r.width));
    }
    (words, all)
}

/// Golden: every one of pdflatex's 11 line starts and all 5 hyphenated line
/// ends (doc-umentation, implemen-tation, incom-prehensible,
/// counterproduc-tive, poly-syllabic) are reproduced; word x positions agree
/// within PDF rounding. Both paragraphs need TeX's second pass (hyphenation).
#[test]
fn justified_hyphenation_matches_pdflatex() {
    let (words, lines) = ours();
    assert_eq!(
        words.len(),
        ORACLE.len(),
        "word count: {:?}",
        words.iter().map(|w| &w.text).collect::<Vec<_>>()
    );
    let mut starts_ok = 0;
    let mut hyph_oracle = 0;
    let mut hyph_ok = 0;
    let mut dx_max: f64 = 0.0;
    let mut dx_sum = 0.0;
    let mut dy_max: f64 = 0.0;
    println!(
        "| word | oracle x | ours x | dx | oracle bottom | ours bottom | dy | line start o/u |"
    );
    println!("|---|---|---|---|---|---|---|---|");
    for (w, (ow, ox, ob, ostart)) in words.iter().zip(ORACLE) {
        assert_eq!(w.text, *ow, "word sequence diverges at {ow:?}");
        if w.line_start == *ostart {
            starts_ok += 1;
        }
        if ow.ends_with('-') {
            hyph_oracle += 1;
            if w.hyphenated {
                hyph_ok += 1;
            }
        }
        let dx = w.x_bp - ox;
        let dy = w.bottom_bp - ob;
        dx_max = dx_max.max(dx.abs());
        dx_sum += dx.abs();
        dy_max = dy_max.max(dy.abs());
        println!(
            "| {ow} | {ox:.3} | {:.3} | {dx:+.3} | {ob:.3} | {:.3} | {dy:+.3} | {}/{} |",
            w.x_bp,
            w.bottom_bp,
            if *ostart { "yes" } else { "no" },
            if w.line_start { "yes" } else { "no" }
        );
    }
    let ours_hyph = words.iter().filter(|w| w.hyphenated).count();
    println!();
    println!(
        "line starts matched: {starts_ok}/{}; hyphenated line ends: oracle {hyph_oracle}, ours {ours_hyph}, matching {hyph_ok}; mean|dx| {:.3} bp; max|dx| {dx_max:.3} bp; max|dy| {dy_max:.3} bp",
        words.len(),
        dx_sum / words.len() as f64
    );
    for l in &lines {
        println!(
            "paragraph: {} lines, pass {}, total demerits {}, hyphenated lines {}, diagnostics {}",
            l.lines.len(),
            l.stats.pass,
            l.stats.total_demerits,
            l.stats.hyphenated_lines,
            l.diagnostics.len()
        );
    }
    assert_eq!(starts_ok, 92);
    assert_eq!((hyph_oracle, ours_hyph, hyph_ok), (5, 5, 5));
    assert!(dx_max < 0.05, "max|dx| {dx_max}");
    assert!(dy_max < 0.05, "max|dy| {dy_max}");
    assert_eq!(
        lines.iter().map(|l| l.lines.len()).collect::<Vec<_>>(),
        vec![6, 5]
    );
    assert!(lines.iter().all(|l| l.stats.pass == 2));
    assert!(lines.iter().all(|l| l.diagnostics.is_empty()));
}
