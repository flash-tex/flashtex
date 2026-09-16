//! The tie as the paragraph builder sees it, against pdflatex.
//!
//! `~` is catcode 13 and expands to `\nobreakspace` = `\leavevmode\nobreak\ `
//! (latex.ltx 9411-9418), and `inputenc` maps a typed U+00A0 onto the same
//! command. Both are therefore an interword space of the font in force with
//! **no legal breakpoint at it** — two claims, and the second is the one a
//! width comparison cannot make.
//!
//! Oracle (pdflatex, TeX Live 2025; no TeX runs here). The paragraph below
//! is engineered so the break wants to fall exactly at the tie:
//!
//! ```text
//! xx xx xx xx xx aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll Figure 7 shows the rest of it.
//! ```
//!
//! | written | pdflatex's first line ends |
//! |---|---|
//! | a plain space | `... llll Figure` |
//! | `~` | `... llll Figure 7` |
//! | a typed U+00A0 | `... llll Figure 7` |
//!
//! 0 overfull and 0 underfull lines in all three pdflatex runs. Article's
//! interword space at 10pt is 3.3209bp, and the tie keeps that width even
//! after a sentence period, because `\ ` ignores the space factor.

mod common;

use common::*;

const NO_BREAK_SPACE: char = '\u{00A0}';

/// The words of each line, in reading order.
fn lines(text: &str) -> Vec<String> {
    let rendered = render_one(text);
    let mut words = words_of(&rendered);
    words.sort_by(|a, b| {
        (a.page, (a.baseline * 100.0) as i64, (a.x * 100.0) as i64)
            .cmp(&(b.page, (b.baseline * 100.0) as i64, (b.x * 100.0) as i64))
    });
    let mut out: Vec<(i64, Vec<String>)> = Vec::new();
    for word in words {
        let key = (word.baseline * 100.0) as i64;
        match out.last_mut() {
            Some((k, line)) if *k == key => line.push(word.text),
            _ => out.push((key, vec![word.text])),
        }
    }
    out.into_iter().map(|(_, line)| line.join(" ")).collect()
}

fn doc(body: &str) -> String {
    format!(r"\documentclass{{article}}\pagestyle{{empty}}\begin{{document}}\noindent {body}\end{{document}}")
}

/// The oracle paragraph, with `sep` between `Figure` and `7`.
fn engineered(sep: &str) -> String {
    doc(&format!(
        "xx xx xx xx xx aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll Figure{sep}7 shows the rest of it."
    ))
}

/// The control: with an ordinary space, pdflatex breaks after `Figure` and
/// leaves `7` to start the second line. If this stops being true the
/// paragraph is no longer engineered and the two tests below prove nothing.
#[test]
fn the_control_paragraph_breaks_between_figure_and_seven() {
    if !lm_available() {
        return;
    }
    let lines = lines(&engineered(" "));
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].ends_with("llll Figure"), "{:?}", lines[0]);
    assert!(lines[1].starts_with("7 shows"), "{:?}", lines[1]);
}

#[test]
fn a_tie_is_not_a_legal_breakpoint() {
    if !lm_available() {
        return;
    }
    let lines = lines(&engineered("~"));
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].ends_with("llll Figure 7"), "{:?}", lines[0]);
    assert!(lines[1].starts_with("shows"), "{:?}", lines[1]);
}

/// A non-breaking space typed straight into the source is the same tie.
/// It used to be set as a *glyph* inside the word, so the paragraph both
/// measured wrong and could not break at all in the neighbourhood.
#[test]
fn a_typed_no_break_space_is_the_same_tie() {
    if !lm_available() {
        return;
    }
    let lines = lines(&engineered(&NO_BREAK_SPACE.to_string()));
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].ends_with("llll Figure 7"), "{:?}", lines[0]);
    assert!(lines[1].starts_with("shows"), "{:?}", lines[1]);
}

/// Width, not just breakability: the tie sets the font's own interword
/// space, and the `\ ` it expands to ignores the space factor, so a
/// preceding sentence period must not widen it.
#[test]
fn a_tie_is_an_interword_space_unaffected_by_the_space_factor() {
    if !lm_available() {
        return;
    }
    let gap = |text: &str| {
        let rendered = render_one(&doc(text));
        let words = words_of(&rendered);
        let left = &words[0];
        let right = &words[1];
        right.x - (left.x + left.width)
    };
    let interword = gap("Figure 7");
    assert!((interword - 3.3209).abs() < 0.01, "article 10pt interword space: {interword}");
    for tied in ["Figure~7", &format!("Figure{NO_BREAK_SPACE}7")] {
        assert!((gap(tied) - interword).abs() < 0.001, "{tied:?} sets {} not {interword}", gap(tied));
    }
    // `Dr.` sets \spacefactor 3000, which stretches an ordinary space but
    // not `\ `.
    let after_period = gap("Dr.~Smith");
    assert!((after_period - interword).abs() < 0.001, "after a period the tie is {after_period}");
}

/// `\textasciitilde` is the only way to ask for the character. Recognising
/// the tie by the character it sets rather than by looking the source
/// back up must not make this one a space.
#[test]
fn textasciitilde_is_still_a_tilde() {
    if !lm_available() {
        return;
    }
    let rendered = render_one(&doc(r"A\textasciitilde B"));
    let joined: String = words_of(&rendered).iter().map(|w| w.text.clone()).collect();
    assert!(joined.contains('~'), "{joined:?}");
    assert!(!joined.contains(NO_BREAK_SPACE), "{joined:?}");
}
