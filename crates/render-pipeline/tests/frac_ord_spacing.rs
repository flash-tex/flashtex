//! `\frac` is an ordinary atom, so it takes no thin space before an Ord.
//!
//! latex.ltx 15742 and amsmath.sty 233 both define `\frac` as
//! `{\begingroup#1\endgroup\over#2}`: the outer braces make the result an Ord
//! atom (TeX §1186), not the Inner atom a bare `\over` builds. The pipeline
//! converted `\frac` through math-layout's `Atom::frac`, which is Inner, and
//! TeX's spacing table gives Inner-Ord a thin space (3mu) in display and text
//! style. So `\frac{2}{5} \quad \text{as }` in
//! `fixtures/real-world/ps-calculus` was 1.82 bp (3mu at 11 pt) too wide, and
//! so was every `\frac` followed by an ordinary atom. `\dfrac` goes through
//! amsmath's `\genfrac`, already Ord, and was exact; it is the control here.
//!
//! Expected numbers are pdflatex's (TeX Live 2025, 11 pt `article`, T1,
//! amsmath), read from its PDF with `tools/visual-oracle/pdftext.py`: the
//! distance from the numerator `a`'s origin to the origin of `as`. (The
//! fraction is painted as one glyph run, `ab`, starting at the numerator.)

mod common;

use common::*;

fn doc(body: &str) -> String {
    format!(
        "\\documentclass[11pt]{{article}}\n\\usepackage[T1]{{fontenc}}\n\\usepackage{{amsmath}}\n\
         \\pagestyle{{empty}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

/// `as`'s origin minus the numerator `a`'s origin, in bp.
fn a_to_as(body: &str) -> f64 {
    let words = words_of(&render_one(&doc(body)));
    let names = || words.iter().map(|w| w.text.clone()).collect::<Vec<_>>();
    let a = words.iter().find(|w| w.text == "ab").unwrap_or_else(|| panic!("no `ab` run in {:?}", names())).x;
    let as_ = words
        .iter()
        .find(|w| w.text.starts_with("as"))
        .unwrap_or_else(|| panic!("no `as` in {:?}", names()))
        .x;
    as_ - a
}

#[test]
fn a_frac_is_ordinary_before_text_and_spaces() {
    if !lm_available() {
        return;
    }
    // (body, pdflatex's a -> as distance). Before the fix each `\frac` case
    // was 1.818 bp wider than this.
    let cases = [
        (r"\[ \frac{a}{b} \quad \text{as } x\to 0 \]", 17.810),
        (r"\[ \frac{a}{b} \qquad \text{as } x\to 0 \]", 28.656),
        (r"\[ \frac{a}{b} \, \text{as } x\to 0 \]", 8.780),
        (r"\[ \frac{a}{b}\text{as } x\to 0 \]", 6.962),
        (r"Text $\frac{a}{b} \quad \text{as } x$ end.", 16.540),
        // Control: `\genfrac` was already Ord.
        (r"\[ \dfrac{a}{b} \quad \text{as } x\to 0 \]", 17.810),
    ];
    let mut bad = Vec::new();
    for (body, want) in cases {
        let got = a_to_as(body);
        if (got - want).abs() > 0.05 {
            bad.push(format!("{body}: a -> as {got:.3} bp, pdflatex has {want:.3}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
