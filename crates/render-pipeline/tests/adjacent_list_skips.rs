//! The skip between two lists that sit next to each other (#706).
//!
//! `\end{<list>}` is `\endtrivlist` -> `\@endparenv`, which does
//! `\addvspace\@topsepadd`; the `\begin{<list>}` that follows is `\@trivlist`,
//! which does `\addvspace\@topsep`. Both are `\addvspace`, so the two keep the
//! *larger* natural skip — they are never summed. And `\@endparenv` leaves TeX
//! in vertical mode, so the second list's `\begin` is read in vertical mode and
//! takes `\partopsep` even though no blank line separates the two.
//!
//! The pipeline got both halves wrong: it summed the closing `\@topsepadd`
//! onto the opening `\@topsep`, and it read the second `\begin` as horizontal
//! mode (no `\partopsep`). At 10 pt that is 10 pt + 8 pt = 18 pt where pdflatex
//! puts max(10 pt, 10 pt) = 10 pt, so every list after the first was 8 pt low
//! and the error grew by a further 6 pt at each later boundary: for four
//! adjacent lists the baselines were +7.971, +13.948 and +19.925 bp off.
//!
//! `itemize`, `enumerate` and `description` are all `\list` with the same
//! `\@listI` skips, so none of this depends on the environment — the bug only
//! looked itemize-specific in #706 because `itemize` came first there.
//!
//! ## Oracle
//!
//! pdfTeX 1.40 (MacTeX 2026), 10 pt T1 Latin Modern, `\pagestyle{empty}`,
//! PyMuPDF glyph origins, in bp. Every one of the 28 probe documents (the six
//! orderings of the three lists, all nine ordered pairs, four of a kind, and
//! each list alone / after a paragraph / after a blank line) puts the first
//! baselines at exactly
//!
//! * 134.765, then 156.682, 178.600, 200.518 for each further adjacent list —
//!   a constant 21.918 bp step, the same for every environment and every
//!   order: `\baselineskip` 12 pt + max(`\topsep` 8 pt + `\partopsep` 2 pt,
//!   the same) = 22 pt;
//! * a list after a paragraph with no blank line: 154.690, i.e. 19.925 bp =
//!   12 pt + `\topsep` 8 pt, with **no** `\partopsep` (horizontal mode);
//! * a list after a blank line: 156.682, i.e. 21.918 bp = 12 pt + 8 pt + 2 pt.
//!
//! The paragraph pair is what separates `\topsep` from `\partopsep`: the 2 pt
//! between them is `\partopsep` on its own.
//!
//! pdflatex is an oracle only and never runs in the product path.

mod common;

/// Word x/baseline gate for this project.
const WORD_TOL_BP: f64 = 0.5;

const HEAD: &str = "\\documentclass[10pt]{article}\n\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}\n\\pagestyle{empty}\n\\begin{document}\n";

/// pdflatex's first baseline on an empty `article` page.
const FIRST: f64 = 134.765;
/// pdflatex's baseline step from one list to the next one beside it.
const ADJACENT: f64 = 21.9175;

fn words(body: &str) -> Vec<common::Word> {
    common::words_of(&common::render_one(&format!("{HEAD}{body}\\end{{document}}\n")))
}

fn at<'a>(words: &'a [common::Word], text: &str) -> &'a common::Word {
    words
        .iter()
        .find(|w| w.text.trim() == text)
        .unwrap_or_else(|| panic!("no run `{text}` in {:?}", words.iter().map(|w| &w.text).collect::<Vec<_>>()))
}

fn check(label: &str, got: f64, expect: f64) {
    assert!(
        (got - expect).abs() <= WORD_TOL_BP,
        "{label}: {got:.3} bp, pdflatex {expect:.3} bp ({:+.3})",
        got - expect
    );
}

/// One single-item list of `kind`, labelled `word`.
fn list(kind: char, word: &str) -> String {
    match kind {
        'i' => format!("\\begin{{itemize}}\\item {word}\\end{{itemize}}\n"),
        'e' => format!("\\begin{{enumerate}}\\item {word}\\end{{enumerate}}\n"),
        'd' => format!("\\begin{{description}}\\item[Term] {word}\\end{{description}}\n"),
        _ => unreachable!(),
    }
}

const PROBES: [&str; 4] = ["alpha", "bravo", "charlie", "delta"];

fn chain(kinds: &str) -> Vec<common::Word> {
    words(&kinds.chars().enumerate().map(|(k, c)| list(c, PROBES[k])).collect::<String>())
}

/// The three environments adjacent, in every order: the baselines are the same
/// 134.765 / 156.682 / 178.600 whichever list comes first, because all three
/// are `\list` with `\@listI`'s skips.
#[test]
fn three_adjacent_lists_step_by_topsep_plus_partopsep_in_every_order() {
    if !common::lm_available() {
        eprintln!("SKIP adjacent_list_skips: Latin Modern not installed");
        return;
    }
    for order in ["ied", "eid", "die", "ide", "edi", "dei"] {
        let w = chain(order);
        for (k, probe) in PROBES[..3].iter().enumerate() {
            check(&format!("{order}: {probe}"), at(&w, probe).baseline, FIRST + ADJACENT * k as f64);
        }
    }
}

/// Every ordered pair, including two lists of the same kind: one boundary is
/// one `\topsep` + `\partopsep`, never two.
#[test]
fn every_ordered_pair_of_lists_has_a_single_boundary_skip() {
    if !common::lm_available() {
        eprintln!("SKIP adjacent_list_skips: Latin Modern not installed");
        return;
    }
    for a in ['i', 'e', 'd'] {
        for b in ['i', 'e', 'd'] {
            let pair = format!("{a}{b}");
            let w = chain(&pair);
            check(&format!("{pair}: alpha"), at(&w, "alpha").baseline, FIRST);
            check(&format!("{pair}: bravo"), at(&w, "bravo").baseline, FIRST + ADJACENT);
        }
    }
}

/// Four lists in a row: the boundary skip does not accumulate. This is the
/// case #706 asks about — the old code was +7.971, +13.948, +19.925 bp low.
#[test]
fn a_chain_of_four_lists_does_not_accumulate_error() {
    if !common::lm_available() {
        eprintln!("SKIP adjacent_list_skips: Latin Modern not installed");
        return;
    }
    for chained in ["iiii", "eeee", "dddd", "iedi"] {
        let w = chain(chained);
        for (k, probe) in PROBES.iter().enumerate() {
            check(&format!("{chained}: {probe}"), at(&w, probe).baseline, FIRST + ADJACENT * k as f64);
        }
    }
}

/// `\topsep` alone versus `\topsep` + `\partopsep`: a list whose `\begin` is
/// read in horizontal mode (no blank line after the paragraph) is 2 pt higher
/// than one read in vertical mode.
#[test]
fn partopsep_applies_only_when_the_begin_is_read_in_vertical_mode() {
    if !common::lm_available() {
        eprintln!("SKIP adjacent_list_skips: Latin Modern not installed");
        return;
    }
    for kind in ['i', 'e', 'd'] {
        let body = list(kind, "alpha");
        // `\baselineskip` 12 pt + `\topsep` 8 pt = 19.925 bp.
        let w = words(&format!("Preceding text paragraph.\n{body}"));
        check(&format!("{kind} after a paragraph"), at(&w, "alpha").baseline - at(&w, "Preceding").baseline, 19.925);
        // + `\partopsep` 2 pt = 21.918 bp.
        let w = words(&format!("Preceding text paragraph.\n\n{body}"));
        check(&format!("{kind} after a blank line"), at(&w, "alpha").baseline - at(&w, "Preceding").baseline, ADJACENT);
        // Alone on the page, all three start at the same first baseline.
        let w = words(&body);
        check(&format!("{kind} alone"), at(&w, "alpha").baseline, FIRST);
    }
}
