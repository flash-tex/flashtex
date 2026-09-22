//! A nonzero preamble `\parindent` indents the first line of paragraphs.
//!
//! The render pipeline is the source of truth for `\parindent`: it reads the
//! preamble assignment from the source (`apply_preamble_lengths`, including
//! `\addtolength` and the class default) into `style.parindent_pt`, and the
//! line breaker opens each ordinary paragraph with that width. This pins the
//! effect end to end: the same body renders with its paragraphs' first words
//! exactly `2em` further right under `\setlength{\parindent}{2em}` than under
//! `\setlength{\parindent}{0pt}`.
//!
//! The compiler-side half (no "recognised but ... not implemented" warning
//! for a nonzero value) is covered against the live compiler in
//! `crates/compiler/tests/preamble_lengths.rs`; this crate still builds
//! against the pinned `vendor/compiler`, so diagnostics are not asserted here.

mod common;

use common::*;
use flashtex_compiler::parser::parse;
use flashtex_render_pipeline::adapter::{self, Labels};
use flashtex_render_pipeline::RenderOptions;

const BODY: &str = "\\begin{document}\nFirst paragraph of body text with enough words to wrap onto a second line.\n\nSecond paragraph of body text with enough words to wrap onto a second line.\n\\end{document}";

fn first_words(src: &str) -> (f64, f64) {
    let r = render_one(src);
    let words = words_of(&r);
    let x = |text: &str| {
        words
            .iter()
            .find(|w| w.text == text)
            .unwrap_or_else(|| {
                panic!("no word {text:?} in {:?}", words.iter().map(|w| &w.text).collect::<Vec<_>>())
            })
            .x
    };
    (x("First"), x("Second"))
}

/// TeX points to PDF points, the unit of every v2 coordinate.
fn bp(pt: f64) -> f64 {
    pt * 72.0 / 72.27
}

#[test]
fn nonzero_parindent_indents_first_lines_by_its_width() {
    if !lm_available() {
        return;
    }
    let zero = format!("\\documentclass{{article}}\n\\setlength{{\\parindent}}{{0pt}}\n{BODY}");
    let two_em = format!("\\documentclass{{article}}\n\\setlength{{\\parindent}}{{2em}}\n{BODY}");
    let (zero_first, zero_second) = first_words(&zero);
    let (ind_first, ind_second) = first_words(&two_em);
    // 2em at the 10pt base is 20pt; both paragraphs open that much further in.
    let want = bp(20.0);
    assert!(
        (ind_first - zero_first - want).abs() < 0.1,
        "first paragraph indented by 2em: {} vs {} + {want}",
        ind_first,
        zero_first
    );
    assert!(
        (ind_second - zero_second - want).abs() < 0.1,
        "second paragraph indented by 2em: {} vs {} + {want}",
        ind_second,
        zero_second
    );
    // The value the line breaker actually used.
    let parsed = parse(&two_em);
    let doc = adapter::adapt(&[two_em.as_str()], 0, &parsed, &RenderOptions::default(), &Labels::default());
    assert!(
        (doc.style.parindent_pt - 20.0).abs() < 1e-6,
        "style parindent is 20pt, got {}",
        doc.style.parindent_pt
    );
}
