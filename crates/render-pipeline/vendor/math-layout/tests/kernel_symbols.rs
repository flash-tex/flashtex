//! LaTeX kernel `\DeclareMathSymbol` rows: slot, family and spacing class.
//!
//! Every expected number is pdfTeX's own `\showthe\wd0` for the same source at
//! 10pt (`\documentclass[10pt]{article}`, no packages), TeX Live 2025. Two
//! boxes per symbol pin the two things a `cm::symbol_slot` +
//! `mathlist::default_class` pair decides:
//!
//! * `\setbox0\hbox{$\sym$}` — the width, which is the TFM width of the slot
//!   the *family* row names, so a wrong family or a wrong slot moves it.
//! * `\setbox0\hbox{$a\sym b$}` — the same width plus the inter-atom glue the
//!   *class* decides (0 for Ord, 2x3mu for Op, 2x4mu for Bin, 2x5mu for Rel),
//!   so a wrong class moves it while the bare width stays right.
//!
//! `$ab$` is 9.57755pt, the control both are read against. Nothing here runs
//! TeX; the oracle output is evidence recorded in the table.
//!
//! The *character* each row is keyed by is the code point that cmsy/cmmi slot
//! carries, as `crates/compiler/tests/amsmath_corpus/oracle.py`'s `OMS_TEXT`
//! already states for family 2 — not always `unicode-math`'s alias for the
//! command. The two differ at cmsy `"0D` (`\bigcirc` is the 1 em `◯` U+25EF,
//! not U+25CB) and cmsy `"0F` (`\bullet` is `∙` U+2219, not U+2022); the
//! widths below are the check, since a slot is 1 em wide or it is not.

use flashtex_math_layout::{Atom, CmMathMetrics, MathList, Style, layout};

fn cm() -> CmMathMetrics {
    CmMathMetrics::latex_10pt()
}

fn f5(v: f64) -> String {
    format!("{:.5}", v)
}

fn width(atoms: Vec<Atom>) -> String {
    f5(layout(&MathList { atoms }, Style::TEXT, &cm()).width)
}

/// `(command, char, \wd of `$\sym$`, \wd of `$a\sym b$`)`.
const KERNEL_SYMBOLS: &[(&str, char, &str, &str)] = &[
    // symbols family (cmsy10) -- fontmath.ltx 234-310, 320-352, 507-509.
    ("amalg", '\u{2A3F}', "7.50002", "21.52190"),
    ("asymp", '\u{224D}', "7.77780", "22.91077"),
    ("clubsuit", '\u{2663}', "7.77780", "17.35535"),
    ("dagger", '\u{2020}', "4.44444", "18.46632"),
    ("ddagger", '\u{2021}', "4.44444", "18.46632"),
    ("diamondsuit", '\u{2662}', "7.77780", "17.35535"),
    ("heartsuit", '\u{2661}', "7.77780", "17.35535"),
    ("spadesuit", '\u{2660}', "7.77780", "17.35535"),
    ("nearrow", '\u{2197}', "10.00002", "25.13298"),
    ("nwarrow", '\u{2196}', "10.00002", "25.13298"),
    ("searrow", '\u{2198}', "10.00002", "25.13298"),
    ("swarrow", '\u{2199}', "10.00002", "25.13298"),
    ("odot", '\u{2299}', "7.77780", "21.79968"),
    ("ominus", '\u{2296}', "7.77780", "21.79968"),
    ("oslash", '\u{2298}', "7.77780", "21.79968"),
    ("prec", '\u{227A}', "7.77780", "22.91077"),
    ("preceq", '\u{2AAF}', "7.77780", "22.91077"),
    ("succ", '\u{227B}', "7.77780", "22.91077"),
    ("succeq", '\u{2AB0}', "7.77780", "22.91077"),
    ("sqcap", '\u{2293}', "6.66669", "20.68857"),
    ("sqcup", '\u{2294}', "6.66669", "20.68857"),
    ("sqsubseteq", '\u{2291}', "7.77780", "22.91077"),
    ("sqsupseteq", '\u{2292}', "7.77780", "22.91077"),
    ("uplus", '\u{228E}', "6.66669", "20.68857"),
    ("wr", '\u{2240}', "2.77779", "16.79967"),
    ("bullet", '\u{2219}', "5.00002", "19.02190"),
    ("diamond", '\u{22C4}', "5.00002", "19.02190"),
    ("bigcirc", '\u{25EF}', "10.00002", "24.02190"),
    ("surd", '\u{221A}', "8.33336", "17.91090"),
    ("mathparagraph", '\u{00B6}', "6.11111", "15.68866"),
    ("mathsection", '\u{00A7}', "4.44447", "14.02202"),
    // letters family (cmmi10) -- fontmath.ltx 234-236, 264, 265, 299, 347-352.
    ("leftharpoonup", '\u{21BC}', "10.00002", "25.13298"),
    ("leftharpoondown", '\u{21BD}', "10.00002", "25.13298"),
    ("rightharpoonup", '\u{21C0}', "10.00002", "25.13298"),
    ("rightharpoondown", '\u{21C1}', "10.00002", "25.13298"),
    ("triangleleft", '\u{25C1}', "5.00002", "19.02190"),
    ("triangleright", '\u{25B7}', "5.00002", "19.02190"),
    ("star", '\u{22C6}', "5.00002", "19.02190"),
    ("flat", '\u{266D}', "3.88890", "13.46645"),
    ("natural", '\u{266E}', "3.88890", "13.46645"),
    ("sharp", '\u{266F}', "3.88890", "13.46645"),
    ("smile", '\u{2323}', "10.00002", "25.13298"),
    ("frown", '\u{2322}', "10.00002", "25.13298"),
    // operators family (cmr10) -- fontmath.ltx 509.
    ("mathdollar", '$', "5.00002", "14.57756"),
    // largesymbols family (cmex10) -- fontmath.ltx 250, 262.
    ("bigsqcup", '\u{2A06}', "8.33336", "21.24416"),
    ("biguplus", '\u{2A04}', "8.33336", "21.24416"),
    // Already-carried rows re-read here as the control on the two tables:
    // \imath/\jmath (letters "7B/"7C) and \varrho (letters "25).
    ("imath", '\u{1D6A4}', "3.22456", "12.80211"),
    ("jmath", '\u{1D6A5}', "3.84030", "13.41785"),
    ("imath (text dotless i)", '\u{0131}', "3.22456", "12.80211"),
    ("jmath (text dotless j)", '\u{0237}', "3.84030", "13.41785"),
    ("varrho", '\u{03F1}', "5.17015", "14.74770"),
];

#[test]
fn ab_control_is_pdftex_width() {
    assert_eq!(width(vec![Atom::symbol('a'), Atom::symbol('b')]), "9.57755");
}

#[test]
fn bare_symbol_widths_match_pdftex() {
    for (command, ch, bare, _) in KERNEL_SYMBOLS {
        assert_eq!(
            width(vec![Atom::symbol(*ch)]),
            *bare,
            "\\{command} (U+{:04X}) bare width",
            *ch as u32
        );
    }
}

#[test]
fn spacing_classes_match_pdftex() {
    for (command, ch, _, flanked) in KERNEL_SYMBOLS {
        assert_eq!(
            width(vec![
                Atom::symbol('a'),
                Atom::symbol(*ch),
                Atom::symbol('b'),
            ]),
            *flanked,
            "\\{command} (U+{:04X}) in `a\\{command} b`",
            *ch as u32
        );
    }
}

/// `\bigsqcup`/`\biguplus` are `\mathop`: they have a display-size successor in
/// cmex10 like `\sum`, and `\limits` by default (not `\nolimits` like `\int`).
#[test]
fn the_two_large_operators_grow_in_display_style() {
    use flashtex_math_layout::{Limits, MathFontMetrics, SizeClass};
    let m = cm();
    for (ch, text_slot) in [('\u{2A06}', 0x46u16), ('\u{2A04}', 0x55)] {
        let small = m.glyph(ch, SizeClass::Text).unwrap();
        let big = m.large_operator(ch, SizeClass::Text).unwrap();
        assert_eq!(small.gid, text_slot, "U+{:04X} text slot", ch as u32);
        assert!(
            big.total_height() > small.total_height(),
            "U+{:04X} has no larger variant",
            ch as u32
        );
        assert_eq!(Atom::symbol(ch).limits, Limits::DisplayLimits);
    }
}
