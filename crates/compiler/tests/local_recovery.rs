//! Half-typed input stays local (issue #77): an unclosed group, argument,
//! math span or display environment is closed at the end of its paragraph,
//! reports one primary diagnostic at its opening token, and leaves every page
//! item after the paragraph exactly where the unedited document puts it.

use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::incremental::{compile_full, CompileOutput, Session};
use flashtex_compiler::layout::{LayoutConstraints, TextItem};

const HW1: &str = include_str!("../../../fixtures/real-world/hw1/HW1.tex");

/// Inserted after this text, inside a paragraph that ends at a blank line
/// and is followed by paragraphs full of inline and display math.
const ANCHOR: &str = "Your solution should";

/// (inserted text, the opening token the one new error must point at)
const EDITS: &[(&str, &str)] = &[
    (r" $x^2 + \frac{a", "{"),
    (r" $x^2 + \frac{a}{", "{a}{"),
    (r" $\sqrt{", "{"),
    (r" \textbf{abc", "{"),
    (r" \begin{align}", r"\begin"),
    (r" $x", "$"),
    (r" $\left(", "$"),
    (r" \[ x+\frac{a", "{"),
];

fn opener_offset(inserted: &str, opener: &str) -> usize {
    // `{a}{` names the second brace of `\frac{a}{`; everything else is the
    // first occurrence of the opener.
    match opener {
        "{a}{" => inserted.find(opener).unwrap() + 3,
        _ => inserted.find(opener).unwrap(),
    }
}

fn items_from(output: &CompileOutput, from: usize, shift: usize) -> Vec<(u32, TextItem)> {
    output
        .pages
        .iter()
        .flat_map(|page| page.items.iter().map(move |item| (page.number, item)))
        .filter(|(_, item)| item.span.start >= from + shift)
        .map(|(number, item)| {
            let mut item = item.clone();
            item.span.start -= shift;
            item.span.end -= shift;
            (number, item)
        })
        .collect()
}

fn errors(output: &CompileOutput) -> Vec<&flashtex_compiler::diagnostics::Diagnostic> {
    output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .collect()
}

#[test]
fn half_typed_input_never_changes_layout_after_its_paragraph() {
    let at = HW1.find(ANCHOR).unwrap() + ANCHOR.len();
    let paragraph_end = at + HW1[at..].find("\n\n").unwrap();
    let base = compile_full(HW1, LayoutConstraints::default());
    let base_after = items_from(&base, paragraph_end, 0);
    assert!(base_after.len() > 500, "the fixture must have a long tail");
    let base_errors = errors(&base).len();

    for &(inserted, opener) in EDITS {
        let edited = format!("{}{inserted}{}", &HW1[..at], &HW1[at..]);
        let output = compile_full(&edited, LayoutConstraints::default());
        assert_eq!(
            output.pages.len(),
            base.pages.len(),
            "{inserted}: page count"
        );

        let new_errors = errors(&output);
        assert_eq!(
            new_errors.len(),
            base_errors + 1,
            "{inserted}: expected exactly one new error, got {:#?}",
            new_errors
        );
        let expected_start = at + opener_offset(inserted, opener);
        assert!(
            new_errors
                .iter()
                .any(|diagnostic| diagnostic.span.map(|span| span.start) == Some(expected_start)),
            "{inserted}: no error located at the opening token (byte {expected_start}): {:#?}",
            new_errors
        );

        let after = items_from(&output, paragraph_end, inserted.len());
        assert_eq!(after.len(), base_after.len(), "{inserted}: item count");
        let first_page = base_after[0].0;
        let shift = after[0].1.baseline_y_pt - base_after[0].1.baseline_y_pt;
        for ((page, item), (base_page, base_item)) in after.iter().zip(&base_after) {
            assert_eq!(page, base_page, "{inserted}: {} changed page", item.text);
            // The changed paragraph's height may move the rest of its own page
            // by one constant amount; later pages are untouched.
            let y_shift = if *page == first_page { shift } else { 0.0 };
            let mut expected = base_item.clone();
            expected.baseline_y_pt += y_shift;
            if let Some(rule) = expected.rule.as_mut() {
                rule.y_pt += y_shift;
            }
            // Baselines are stored rounded to 0.01pt, so `base + shift` can
            // land one rounding step away from the re-laid item: compare y
            // within one step and everything else exactly.
            let strip = |item: &TextItem| {
                let mut item = item.clone();
                item.baseline_y_pt = 0.0;
                if let Some(rule) = item.rule.as_mut() {
                    rule.y_pt = 0.0;
                }
                format!("{item:?}")
            };
            assert_eq!(strip(item), strip(&expected), "{inserted}: item moved");
            assert!(
                (item.baseline_y_pt - expected.baseline_y_pt).abs() <= 0.0101,
                "{inserted}: item moved: {item:?} vs {expected:?}"
            );
            if let (Some(a), Some(b)) = (item.rule.as_ref(), expected.rule.as_ref()) {
                assert!((a.y_pt - b.y_pt).abs() <= 0.0101, "{inserted}: rule moved: {item:?} vs {expected:?}");
            }
        }
    }
}

#[test]
fn typing_an_unclosed_construct_incrementally_matches_fresh_compiles() {
    let at = HW1.find(ANCHOR).unwrap() + ANCHOR.len();
    let fresh = |text: &str| format!("{:#?}", compile_full(text, LayoutConstraints::default()));
    let mut session = Session::new();
    let mut check = |text: &str| {
        let incremental = session.compile(text, LayoutConstraints::default());
        assert_eq!(
            format!("{:#?}", incremental.output),
            fresh(text),
            "incremental output differs from a fresh compile after typing {:?}",
            &text[at..text.len().min(at + 24)]
        );
    };
    check(HW1);
    // Keystroke by keystroke into the paragraph, then deleting it again.
    let typed = r" $x^2 + \frac{a}{b}$";
    for end in (1..=typed.len()).chain((0..typed.len()).rev()) {
        check(&format!("{}{}{}", &HW1[..at], &typed[..end], &HW1[at..]));
    }
    for &(inserted, _) in EDITS {
        check(&format!("{}{inserted}{}", &HW1[..at], &HW1[at..]));
        check(HW1);
    }
}
