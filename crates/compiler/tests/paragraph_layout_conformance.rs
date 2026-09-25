//! Paragraph-layout conformance harness, slice 1 of #27 (measurement only).
//!
//! The compiler breaks paragraphs greedily (`layout.rs`: "breaking here is
//! greedy": each word wraps with `x + w > right_edge`), while
//! `crates/paragraph-layout` offers TeX-optimal Knuth-Plass total-fit
//! (`layout_paragraph`). Adopting it in production changes every line break,
//! so this slice only MEASURES: it lays out a small fixed corpus of plain
//! prose with the real compiler pipeline (greedy) and with
//! `layout_paragraph` (total-fit), and diffs the chosen break points. It
//! touches no rendering code path and no existing fixture.
//!
//! Metrics parity: the optimal side rebuilds each document's horizontal list
//! with `ParagraphBuilder` over `Core14Times::ROMAN` at 12pt. Those advances
//! were transcribed from the same Adobe AFM data as the compiler's own
//! tables (`core14.rs` provenance note), and both sides apply kerns and the
//! `fi`/`fl` ligatures; hyphenation is off on both sides (`NoHyphenation`;
//! the compiler has none). The corpus avoids apostrophes, the one known
//! deliberate metrics deviation (`quotesingle` 180 vs `quoteright` 333).
//! Measure is the compiler's text column (`PAGE_WIDTH_PT - 2*MARGIN_PT`).

use flashtex_compiler::layout::{
    self as clayout, BODY_SIZE_PT, MARGIN_PT, PAGE_WIDTH_PT, text_width,
};
use flashtex_compiler::layout::Font as CompilerFont;
use flashtex_compiler::parser;
use flashtex_paragraph_layout::core14::Core14Times;
use flashtex_paragraph_layout::{
    FontMetricsSource, Glue, LineBreakParams, Lines, NoHyphenation, ParagraphBuilder,
    layout_paragraph, shape_run,
};

/// Compiler text-column width in pt.
const MEASURE_PT: f64 = PAGE_WIDTH_PT - 2.0 * MARGIN_PT;

/// Small fixed corpus of plain prose (no LaTeX-special characters, so the
/// compiler parses each as one `Block::Paragraph` of `Inline::Text`, exactly
/// what `ParagraphBuilder::text` consumes).
const CORPUS: &[&str] = &[
    "In 1969 the Apollo 11 mission landed the first humans on the Moon, a milestone that reshaped how people imagined the future of exploration, science, and the long, uncertain relationship between ambition and the fragile machinery that carries it forward.",
    "Supercalifragilisticexpialidocious words occasionally appear in otherwise ordinary paragraphs and they change how a line must wrap, forcing the breaker to reconsider every candidate point rather than simply falling back on the nearest convenient space between two shorter words.",
    "The committee reviewed the quarterly report, noted several discrepancies in the projected revenue figures, and requested a full audit before the next meeting, citing concerns that had already been raised twice in prior sessions without any concrete resolution being reached.",
    "The old lighthouse keeper climbed the spiral staircase every evening to light the lamp that guided ships safely past the rocky coastline, a routine he had kept for thirty years without missing a single night, rain or shine, storm or calm.",
];

/// Line-start byte offsets from the real greedy pipeline. A compiler
/// `TextItem` starts a line exactly when its `x_pt` is the left margin (`x`
/// is reset to `MARGIN_PT` only by `newline`).
fn greedy_line_starts(text: &str) -> Vec<usize> {
    let parsed = parser::parse(text);
    let pages = clayout::layout(&parsed.blocks);
    let mut starts = Vec::new();
    for page in &pages {
        for item in &page.items {
            if (item.x_pt - MARGIN_PT).abs() < 1e-9 {
                starts.push(item.span.start);
            }
        }
    }
    starts
}

fn optimal_layout(text: &str) -> Lines {
    let m = Core14Times::ROMAN;
    let h = NoHyphenation;
    let mut b = ParagraphBuilder::new(&h);
    b.text(&m, BODY_SIZE_PT, text, 0).unwrap();
    let items = b.finish(Glue::fil());
    let params = LineBreakParams::article_12pt_letter_1in().with_width(MEASURE_PT);
    layout_paragraph(&items, &params).unwrap()
}

/// Line-start byte offsets from `layout_paragraph` (total-fit): the source
/// offset of the first run of every line.
fn optimal_line_starts(text: &str) -> Vec<usize> {
    optimal_layout(text)
        .lines
        .iter()
        .filter_map(|l| l.runs.first().map(|r| r.source.start))
        .collect()
}

struct Pinned {
    greedy: &'static [usize],
    optimal: &'static [usize],
}

/// Pinned per-document line-start byte offsets for BOTH engines. Exact
/// output, captured by running this harness — the test recomputes them from
/// the two real engines every run and asserts equality against these
/// literals, so a change to either breaker fails here with the new numbers.
const PINNED: [Pinned; 4] = [
    Pinned {
        greedy: &[0, 93, 186],
        optimal: &[0, 97, 194],
    },
    Pinned {
        greedy: &[0, 94, 188],
        optimal: &[0, 98, 195],
    },
    Pinned {
        greedy: &[0, 90, 184],
        optimal: &[0, 98, 197],
    },
    Pinned {
        greedy: &[0, 99, 195],
        optimal: &[0, 99, 202],
    },
];

/// Indices (into [`CORPUS`]) of the documents where greedy and total-fit
/// choose at least one different break point: every document in this corpus.
const DOCS_THAT_DIFFER: &[usize] = &[0, 1, 2, 3];

/// Total count of differing line-index positions across the corpus: docs 0,
/// 1 and 2 disagree at 2 positions each, doc 3 at 1.
const TOTAL_DIFFERING_POSITIONS: usize = 7;

#[test]
fn greedy_vs_total_fit_breaks_pinned() {
    assert_eq!(CORPUS.len(), PINNED.len());
    assert_eq!(MEASURE_PT, 468.0);
    assert_eq!(BODY_SIZE_PT, 12.0);
    let mut docs_differing = Vec::new();
    let mut total_differing_positions = 0usize;

    for (i, text) in CORPUS.iter().enumerate() {
        let g_starts = greedy_line_starts(text);
        let o_starts = optimal_line_starts(text);
        println!("doc {i}: greedy  {g_starts:?}");
        println!("doc {i}: optimal {o_starts:?}");

        assert_eq!(g_starts, PINNED[i].greedy, "doc {i}: greedy breaks changed");
        assert_eq!(
            o_starts, PINNED[i].optimal,
            "doc {i}: total-fit breaks changed"
        );

        let max_len = g_starts.len().max(o_starts.len());
        let diff = (0..max_len)
            .filter(|&idx| g_starts.get(idx) != o_starts.get(idx))
            .count();
        total_differing_positions += diff;
        if diff > 0 {
            docs_differing.push(i);
        }
    }

    println!("docs that differ: {docs_differing:?}");
    println!("differing positions: {total_differing_positions}");
    assert_eq!(docs_differing, DOCS_THAT_DIFFER);
    assert_eq!(total_differing_positions, TOTAL_DIFFERING_POSITIONS);
    assert!(
        !docs_differing.is_empty(),
        "bug #27 no longer reproduces: greedy and total-fit agree everywhere"
    );
}

/// Executable metric-parity check: the optimal side builds its `Item`s from
/// `Core14Times::ROMAN`, while the greedy side measures through the real
/// compiler pipeline (`layout::text_width` over the font engine's Core 14
/// tables, with kerning and ligatures applied). This test compares the two
/// paths' actual shaped widths word by word, so a metrics mismatch fails
/// here rather than hiding inside the break-point diff above. Only the
/// public test-visible API is used; no production code was touched.
///
/// Both sides scale integer font units by `BODY_SIZE_PT / 1000` and sum in a
/// different order, so the comparison allows 1e-9 pt of float rounding. A
/// one-unit (0.012 pt) perturbation on either side still fails loudly.
#[test]
fn metric_parity_compiler_vs_core14_roman() {
    const TIMES: Core14Times = Core14Times::ROMAN;
    const TOL_PT: f64 = 1e-9;

    // 1. Every distinct corpus character (plus space): unkerned advances.
    let mut chars: Vec<char> = CORPUS
        .iter()
        .flat_map(|t| t.chars())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    chars.sort_unstable();
    assert!(!chars.is_empty());
    println!("distinct corpus chars ({}): {chars:?}", chars.len());
    for ch in &chars {
        let expected_pt = TIMES.advance(*ch) * BODY_SIZE_PT / TIMES.units_per_em();
        let actual_pt = text_width(
            &ch.to_string(),
            BODY_SIZE_PT,
            CompilerFont::TimesRoman,
        );
        assert!(
            (actual_pt - expected_pt).abs() <= TOL_PT,
            "advance mismatch for {ch:?} (U+{:04X}): compiler {actual_pt} pt vs Core14Times {expected_pt} pt",
            *ch as u32,
        );
    }

    // 2. Every distinct corpus word through BOTH real shaping paths: the
    // optimal side's own `shape_run` (advances + kerns + fi/fl ligatures)
    // against the compiler's `text_width`. Words carry no spaces, so no
    // glue/space-model difference can intrude.
    let mut words: Vec<&str> = CORPUS
        .iter()
        .flat_map(|t| t.split_whitespace())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    words.sort_unstable();
    let mut kerned_runs = 0usize;
    let mut ligatured_runs = 0usize;
    let mut max_diff = 0.0f64;
    for w in &words {
        let run = shape_run(&TIMES, BODY_SIZE_PT, w, 0);
        let actual_pt = text_width(w, BODY_SIZE_PT, CompilerFont::TimesRoman);
        let diff = (actual_pt - run.width).abs();
        max_diff = max_diff.max(diff);
        assert!(
            diff <= TOL_PT,
            "shaped-width mismatch for {w:?}: compiler {actual_pt} pt vs shape_run {} pt",
            run.width,
        );
        if run.glyphs.iter().any(|g| g.kern != 0.0) {
            kerned_runs += 1;
        }
        if run.glyphs.len() < w.chars().count() {
            ligatured_runs += 1;
        }
    }
    println!(
        "words checked: {}; with nonzero kerns: {kerned_runs}; with ligatures: {ligatured_runs}; max diff: {max_diff:.3e} pt",
        words.len(),
    );
    // Without at least one kerned and one ligatured word the loop above
    // would pass even with a kern- or ligature-table mismatch. "first" and
    // "figures" carry the `fi` ligature on both sides (U+FB01, 556 units).
    assert!(
        kerned_runs > 0,
        "corpus exercises no kern pairs; add a kerned probe word"
    );
    assert!(
        ligatured_runs > 0,
        "corpus exercises no ligatures; add an fi/fl probe word"
    );
    for probe in ["fi", "fl"] {
        let run = shape_run(&TIMES, BODY_SIZE_PT, probe, 0);
        let actual_pt = text_width(probe, BODY_SIZE_PT, CompilerFont::TimesRoman);
        assert_eq!(
            run.glyphs.len(),
            1,
            "{probe:?} should form one ligature glyph on the optimal side"
        );
        assert!(
            (actual_pt - run.width).abs() <= TOL_PT,
            "ligature mismatch for {probe:?}: compiler {actual_pt} pt vs {} pt",
            run.width,
        );
    }

    // 3. The one KNOWN deliberate deviation, pinned so it cannot drift
    // silently: ASCII `'` is `quotesingle` (180 units) in the compiler's
    // tables but `quoteright` (333 units, with its AFM kern pairs) on the
    // optimal side, which follows TeX's OT1/T1 encoding. The corpus avoids
    // apostrophes, so the break diff above is unaffected — but the parity
    // check must show its work here rather than pass by accident.
    let compiler_apost = text_width("'", BODY_SIZE_PT, CompilerFont::TimesRoman);
    let optimal_apost = TIMES.advance('\'') * BODY_SIZE_PT / TIMES.units_per_em();
    assert!(
        (compiler_apost - 180.0 * BODY_SIZE_PT / 1000.0).abs() <= TOL_PT,
        "compiler `'` width changed: {compiler_apost} pt"
    );
    assert!(
        (optimal_apost - 333.0 * BODY_SIZE_PT / 1000.0).abs() <= TOL_PT,
        "optimal `'` width changed: {optimal_apost} pt"
    );
    assert!(
        (compiler_apost - optimal_apost).abs() > 1.0,
        "the known quotesingle/quoteright deviation closed; update the corpus note and this pin"
    );
}

/// pdflatex oracle cross-check, reproducible without any scratch directory.
///
/// The original slice-1 check-in measured corpus doc 0 with real pdflatex
/// but kept the fixture in `/tmp`, so nobody could re-execute it. The
/// fixture is therefore generated here, byte-for-byte, from `CORPUS[0]`,
/// and the exact oracle output observed is committed below as
/// `ORACLE_SHIPPED_HBOXES`.
///
/// To reproduce (TeX Live 2026, `/Library/TeX/texbin/pdflatex` =
/// pdfTeX 3.141592653-2.6-1.40.29):
///
/// ```sh
/// # 1. Emit the fixture (the test prints the oracle_tex() bytes):
/// cargo test -p flashtex-compiler --test paragraph_layout_conformance \
///     pdflatex_oracle -- --nocapture
/// # 2. Save the \documentclass..\end{document} block as oracle.tex, then:
/// mkdir -p /tmp/plc-oracle && cd /tmp/plc-oracle
/// /Library/TeX/texbin/pdflatex -interaction=nonstopmode oracle.tex
/// # expect: Output written on oracle.pdf (1 page, 14213 bytes)
/// grep -E '^\.\.\.\\hbox\(8\.18385' oracle.log
/// # expect: the three ORACLE_SHIPPED_HBOXES lines below
/// ```
///
/// Expected: `Output written on oracle.pdf (1 page, 14213 bytes)` and the
/// three content boxes of `ORACLE_SHIPPED_HBOXES`, whose first/last words
/// (`In`->`how`, `people`->`between`, `ambition`->`forward.`) put pdflatex's
/// line starts at byte offsets [0, 97, 194] — exactly the optimal breaks
/// pinned in `PINNED[0]`, while greedy breaks one word early twice
/// ([0, 93, 186]). Word membership is exact; no tolerance is involved.
/// Re-verified 2026-09-25 against the same engine; the committed bytes and
/// log lines matched the original measurement character-for-character.
fn oracle_tex() -> String {
    format!(
        "\\documentclass[12pt]{{article}}\n\
         \\usepackage{{times}}\n\
         \\usepackage[textwidth=468pt]{{geometry}}\n\
         \\parindent=0pt\n\
         \\showoutput\n\
         \\begin{{document}}\n\
         {}\n\
         \\end{{document}}\n",
        CORPUS[0],
    )
}

/// The three shipped-out content `\hbox` lines from `oracle.log`
/// (`Completed box being shipped out [1]`; the fourth `...\\hbox` in the log
/// is the page number). Each is `...\\hbox(8.18385+2.5979)x468.0` with the
/// line's glue set: shrunk, stretched, then the `parfillskip fil` last line.
const ORACLE_SHIPPED_HBOXES: &[&str] = &[
    "...\\hbox(8.18385+2.5979)x468.0, glue set - 0.68916",
    "...\\hbox(8.18385+2.5979)x468.0, glue set 0.37509",
    "...\\hbox(8.18385+2.5979)x468.0, glue set 189.85518fil",
];

/// First and last word of each oracle line, in order, as they appear in
/// `oracle.log`'s glyph runs.
const ORACLE_LINE_WORDS: &[(&str, &str)] = &[
    ("In", "how"),
    ("people", "between"),
    ("ambition", "forward."),
];

/// pdflatex's line-start byte offsets in `CORPUS[0]`, from the word
/// membership above.
const ORACLE_LINE_STARTS: &[usize] = &[0, 97, 194];

#[test]
fn pdflatex_oracle_fixture_matches_pinned_optimal() {
    // The generated fixture embeds the corpus paragraph verbatim: if doc 0
    // ever changes, this fails and the oracle measurement must be re-run.
    let tex = oracle_tex();
    println!("{tex}");
    assert!(
        tex.contains(CORPUS[0]),
        "oracle fixture drifted from CORPUS[0]; regenerate and re-run pdflatex"
    );

    // Word membership of each oracle line, checked against the corpus bytes.
    assert_eq!(ORACLE_LINE_WORDS.len(), ORACLE_SHIPPED_HBOXES.len());
    assert_eq!(ORACLE_LINE_STARTS.len(), ORACLE_LINE_WORDS.len());
    for (i, ((first, last), start)) in ORACLE_LINE_WORDS
        .iter()
        .zip(ORACLE_LINE_STARTS.iter())
        .enumerate()
    {
        let rest = &CORPUS[0][*start..];
        assert!(
            rest.starts_with(first)
                && (rest.len() == first.len() || rest[first.len()..].starts_with(' ')),
            "oracle line {i} does not start with {first:?} at byte {start}"
        );
        assert!(
            rest.contains(last),
            "oracle line {i} word {last:?} not found after byte {start}"
        );
    }
    // The oracle's last line ends the paragraph: nothing follows "forward.".
    assert!(CORPUS[0].ends_with("forward."));

    // pdflatex agrees with total-fit, not greedy, on doc 0.
    assert_eq!(
        ORACLE_LINE_STARTS,
        PINNED[0].optimal,
        "pdflatex line starts no longer match the pinned optimal breaks"
    );
    assert_ne!(
        ORACLE_LINE_STARTS,
        PINNED[0].greedy,
        "greedy now agrees with pdflatex on doc 0; bug #27 may be fixed — repin"
    );
}
