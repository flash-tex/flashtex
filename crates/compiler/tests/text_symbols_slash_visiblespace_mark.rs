//! Text-mode `\slash`, `\textvisiblespace` and `\textcompwordmark`.
//!
//! Stated pdflatex oracle (TeX Live 2026, `\documentclass{article}` at
//! 10pt, exit code 0, zero `!` errors) for
//! `a\slash b and c\textvisiblespace d and shelf\textcompwordmark ful
//! and f\textcompwordmark i next.`:
//!
//! - `\slash` is `/` followed by `\penalty 50` (`\showbox` shows
//!   `....\OT1/cmr/m/n/10 /` then `....\penalty 50`; latex.ltx 604
//!   defines it as `/\penalty\exhyphenpenalty`, a break point after the
//!   slash, not a hyphenation point).
//! - `\textvisiblespace` in OT1 is the kernel's rule construction, not a
//!   glyph (`\showbox` of `c\textvisiblespace d`: a kern of 0.59998pt, a
//!   0.4pt-by-1.29167pt post, a 3.00003pt-by-0.4pt bar, a second post —
//!   4.4pt wide at 10pt, no glue between the boxes). In T1 it is the real
//!   slot-32 glyph (U+2423).
//! - `\textcompwordmark` is T1/cmr slot 23 (`\showbox` shows
//!   `....\T1/cmr/m/n/10 ^^W` between the letters), a zero-width node that
//!   breaks the ligature/kern program: `shelf\textcompwordmark ful` sets
//!   `shelf` + `ful` with no `ff` ligature, `f\textcompwordmark i` with no
//!   `fi` ligature.
//!
//! The oracle pins `and` at x 167.531, 204.571 and 258.339 and `next.` at
//! 286.852 (glyph origins in bp from the page's top-left). Those positions
//! need Computer Modern metrics, which only the render pipeline sets; the
//! compiler's own layout uses Core 14 Times and cannot reproduce them to
//! 0.1bp. What is pinned here is the compiler's contract: zero
//! diagnostics for the oracle source, and the exact inline structure the
//! pipeline consumes (`/` + penalty 50; kern + three rules totalling
//! 4.4pt at 10pt; `boundary_before` on the run after the mark, which the
//! pipeline shapes as its own segment so no ligature forms).
use flashtex_compiler::incremental::{compile_full, LayoutConstraints};
use flashtex_compiler::parser::{Block, Inline, parse};
use flashtex_compiler::text_builtins::{self as tb, TextDimen};

/// The oracle source as a full document (default OT1).
fn document(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn oracle_source() -> String {
    document("a\\slash b and c\\textvisiblespace d and shelf\\textcompwordmark ful and f\\textcompwordmark i next.")
}

/// The body paragraph's inlines.
fn paragraph(source: &str) -> Vec<Inline> {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    parsed
        .blocks
        .into_iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .expect("one paragraph")
}

/// The oracle source compiles with zero diagnostics, through the full
/// layout as well as the parser.
#[test]
fn oracle_source_has_zero_diagnostics() {
    let source = oracle_source();
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let laid_out = compile_full(&source, LayoutConstraints::default());
    assert!(laid_out.diagnostics.is_empty(), "{:?}", laid_out.diagnostics);
    let words: Vec<&str> = laid_out
        .pages
        .iter()
        .flat_map(|page| page.items.iter())
        .map(|item| item.text.as_str())
        .collect();
    // The slash, the three rules (drawn with the rule character) and no
    // ligated `ff`/`fi`: `shelf` and `ful` stay separate runs in this
    // layout, which shapes each run on its own.
    assert!(words.contains(&"a"), "{words:?}");
    assert!(words.contains(&"/"), "{words:?}");
    assert!(words.contains(&"shelf"), "{words:?}");
    assert!(words.contains(&"ful"), "{words:?}");
    assert!(!words.iter().any(|w| w.contains("␣") || *w == "@"), "{words:?}");
}

/// `\slash` is `/` plus a penalty-50 break: latex.ltx 604, confirmed by
/// `\showbox` (`/` then `\penalty 50`).
#[test]
fn slash_is_slash_plus_exhyphenpenalty() {
    let inlines = paragraph(&document("a\\slash b next."));
    let (before, after) = (&inlines[..3], &inlines[3..]);
    assert!(matches!(&before[0], Inline::Text { text, .. } if text == "a"), "{before:?}");
    assert!(matches!(&before[1], Inline::Text { text, space_before: false, .. } if text == "/"), "{before:?}");
    assert!(
        matches!(&before[2], Inline::Penalty { value: 50, unskip: false, .. }),
        "{before:?}"
    );
    assert!(matches!(&after[0], Inline::Text { text, space_before: false, .. } if text == "b"), "{after:?}");
}

/// OT1 `\textvisiblespace` is the kern plus the three rules of the
/// kernel's construction (latex.ltx `\DeclareTextCommandDefault`).
#[test]
fn textvisiblespace_is_kern_plus_three_rules() {
    let inlines = paragraph(&document("c\\textvisiblespace d next."));
    assert!(matches!(&inlines[0], Inline::Text { text, .. } if text == "c"), "{inlines:?}");
    let (kern, post, bar, post2) = (&inlines[1], &inlines[2], &inlines[3], &inlines[4]);
    let Inline::Kern { amount, .. } = kern else { panic!("kern: {kern:?}") };
    assert_eq!(amount, &TextDimen::parse(".06em").unwrap(), "{kern:?}");
    for (rule, width, height) in [
        (post, ".4pt", ".3ex"),
        (bar, ".3em", ".4pt"),
        (post2, ".4pt", ".3ex"),
    ] {
        let Inline::Rule { rule, space_before: false, .. } = rule else {
            panic!("rule: {rule:?}")
        };
        assert_eq!(rule.width, TextDimen::parse(width).unwrap(), "{rule:?}");
        assert_eq!(rule.height, TextDimen::parse(height).unwrap(), "{rule:?}");
        assert_eq!(rule.raise, TextDimen::zero(), "{rule:?}");
    }
    assert!(matches!(&inlines[5], Inline::Text { text, space_before: false, .. } if text == "d"), "{inlines:?}");
}

/// The construction is 4.4pt wide at 10pt (0.6 + 0.4 + 3.0 + 0.4), the
/// width the oracle's `and` positions after it depend on.
#[test]
fn textvisiblespace_totals_4_4pt_at_10pt() {
    let inlines = paragraph(&document("c\\textvisiblespace d"));
    let cx = tb::DimenContext {
        quad: tb::pt_to_sp(10.0),
        x_height: tb::pt_to_sp(4.30554),
        ..Default::default()
    };
    let mut width = 0;
    for inline in &inlines[1..5] {
        match inline {
            Inline::Kern { amount, .. } => width += amount.resolve(&cx),
            Inline::Rule { rule, .. } => width += rule.resolve(&cx).width,
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(width, tb::pt_to_sp(4.4), "{inlines:?}");
}

/// T1 `\textvisiblespace` is the real slot-32 glyph, U+2423.
#[test]
fn textvisiblespace_in_t1_is_the_visible_space_glyph() {
    let source =
        "\\documentclass{article}\n\\usepackage[T1]{fontenc}\n\\begin{document}\nc\\textvisiblespace d\n\\end{document}\n";
    let inlines = paragraph(source);
    assert!(matches!(&inlines[1], Inline::Text { text, .. } if text == "\u{2423}"), "{inlines:?}");
}

/// `\textcompwordmark` sets nothing itself but marks the run after it as a
/// ligature boundary, so the pipeline shapes `f` and `i` apart (no `fi`
/// ligature) and `shelf` and `ful` apart (no `ff` ligature).
#[test]
fn textcompwordmark_bounds_the_following_run() {
    let inlines = paragraph(&document("f\\textcompwordmark i next."));
    assert!(matches!(&inlines[0], Inline::Text { text, .. } if text == "f"), "{inlines:?}");
    assert!(
        matches!(&inlines[1], Inline::Text { text, boundary_before: true, .. } if text == "i"),
        "{inlines:?}"
    );
    let inlines = paragraph(&document("shelf\\textcompwordmark ful next."));
    assert!(matches!(&inlines[0], Inline::Text { text, .. } if text == "shelf"), "{inlines:?}");
    assert!(
        matches!(&inlines[1], Inline::Text { text, boundary_before: true, .. } if text == "ful"),
        "{inlines:?}"
    );
    // Without the mark there is no boundary: `fi` ligates as usual.
    let inlines = paragraph(&document("fi next."));
    assert!(
        matches!(&inlines[0], Inline::Text { text, boundary_before: false, .. } if text == "fi"),
        "{inlines:?}"
    );
}

/// The mark works wrapped in a group too (`shelf{\textcompwordmark}ful`):
/// the node still sits between the letters on the list.
#[test]
fn textcompwordmark_inside_a_group_still_bounds() {
    let inlines = paragraph(&document("shelf{\\textcompwordmark}ful next."));
    assert!(
        matches!(&inlines[1], Inline::Text { text, boundary_before: true, .. } if text == "ful"),
        "{inlines:?}"
    );
}
