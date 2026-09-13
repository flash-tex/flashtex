//! The LaTeX kernel's `\DeclareMathDelimiter`s, against pdfTeX's own numbers.
//!
//! `fontmath.ltx` 457-505 declares twelve delimiters this crate's tables did
//! not carry: `\Vert` `\vert` `\backslash` `\updownarrow` `\Updownarrow`
//! `\lgroup` `\rgroup` `\lmoustache` `\rmoustache` `\arrowvert` `\Arrowvert`
//! `\bracevert`. Each is a *pair* of font slots — a small variant set when the
//! delimiter is used as an ordinary math character, and a `largesymbols`
//! growth chain `var_delimiter` walks when it is a `\left`/`\right` fence — so
//! a row is only right if both halves are.
//!
//! Every number below was measured with TeX Live 2025
//! pdfTeX 3.141592653-2.6-1.40.27 at 10pt, not derived from the tables:
//!
//! * `SMALL` is `\showthe\wd0`/`\ht0`/`\dp0` of `\hbox{$\sym$}`, which is the
//!   TFM box of the slot the *small* variant names. A wrong family or slot
//!   moves it.
//! * `GROWN` is the same three of `\hbox{$\left\sym\vrule height 45pt depth
//!   45pt width 0pt\right.$}` minus `\nulldelimiterspace` (1.2pt): a body far
//!   taller than any discrete size, so `var_delimiter` has reached the end of
//!   the chain and the numbers are the *extensible* recipe's, which is what
//!   the growth chain is for.
//! * `CLASS` is read from the `$a\sym b$` box against the `$ab$` control
//!   (9.57755pt at 10pt): 0 for Ord, 2x5mu for Rel. Open and Close add no
//!   space next to an ordinary, so those two come from `fontmath.ltx`'s
//!   declaration rather than from the measurement, which cannot separate them
//!   from Ord.

use flashtex_math_layout::cm::{self, CmMathMetrics, Family};
use flashtex_math_layout::mathlist::{default_class, AtomClass};
use flashtex_math_layout::{MathFontMetrics, SizeClass};

/// command, character, (small family, small slot), large cmex slot, class.
const DELIMITERS: &[(&str, char, (Family, u8), u8, AtomClass)] = &[
    (r"\Vert", '\u{2016}', (Family::Symbol, 0x6B), 0x0D, AtomClass::Ord),
    (r"\vert", '|', (Family::Symbol, 0x6A), 0x0C, AtomClass::Ord),
    (r"\backslash", '\u{2216}', (Family::Symbol, 0x6E), 0x0F, AtomClass::Ord),
    (r"\updownarrow", '\u{2195}', (Family::Symbol, 0x6C), 0x3F, AtomClass::Rel),
    (r"\Updownarrow", '\u{21D5}', (Family::Symbol, 0x6D), 0x77, AtomClass::Rel),
    (r"\lgroup", '\u{27EE}', (Family::Extension, 0x3A), 0x3A, AtomClass::Open),
    (r"\rgroup", '\u{27EF}', (Family::Extension, 0x3B), 0x3B, AtomClass::Close),
    (r"\lmoustache", '\u{23B0}', (Family::Extension, 0x7A), 0x40, AtomClass::Open),
    (r"\rmoustache", '\u{23B1}', (Family::Extension, 0x7B), 0x41, AtomClass::Close),
    (r"\arrowvert", '\u{23D0}', (Family::Symbol, 0x6A), 0x3C, AtomClass::Ord),
    (r"\Arrowvert", cm::ARROWVERT_DOUBLE, (Family::Symbol, 0x6B), 0x3D, AtomClass::Ord),
    (r"\bracevert", '\u{23AA}', (Family::Extension, 0x3E), 0x3E, AtomClass::Ord),
];

/// `\showthe\wd0 \ht0 \dp0` of `\hbox{$\sym$}` at 10pt.
const SMALL: &[(&str, f64, f64, f64)] = &[
    (r"\Vert", 5.00002, 7.50000, 2.50000),
    (r"\vert", 2.77779, 7.50000, 2.50000),
    (r"\backslash", 5.00002, 7.50000, 2.50000),
    (r"\updownarrow", 5.00002, 7.50000, 2.50000),
    (r"\Updownarrow", 6.11111, 7.50000, 2.50000),
    (r"\lgroup", 8.88890, 0.00000, 9.00009),
    (r"\rgroup", 8.88890, 0.00000, 9.00009),
    (r"\lmoustache", 4.50005, 1.19997, 0.00000),
    (r"\rmoustache", 4.50005, 1.19997, 0.00000),
    (r"\arrowvert", 2.77779, 7.50000, 2.50000),
    (r"\Arrowvert", 5.00002, 7.50000, 2.50000),
    (r"\bracevert", 8.88890, 0.00000, 3.00003),
];

/// The delimiter's own width in `\hbox{$\left\sym<45pt rule>\right.$}` at
/// 10pt, less `\nulldelimiterspace`. At this height every one of the twelve
/// has reached its extensible recipe, so this is the repeated piece's width.
const GROWN_WIDTH: &[(&str, f64)] = &[
    (r"\Vert", 5.55557),
    (r"\vert", 3.33333),
    // The only one of the twelve with no extensible recipe: cmex "0F's chain
    // ends at the largest discrete size ("2D, 1.27778 em), which is why a
    // very tall `\left\backslash` is the one that stays too small.
    (r"\backslash", 12.77780),
    (r"\updownarrow", 6.66668),
    (r"\Updownarrow", 7.77780),
    (r"\lgroup", 8.88890),
    (r"\rgroup", 8.88890),
    (r"\lmoustache", 8.88890),
    (r"\rmoustache", 8.88890),
    (r"\arrowvert", 6.66668),
    (r"\Arrowvert", 7.77780),
    (r"\bracevert", 8.88890),
];

/// cmex extensible recipes (`TOP`, `MID`, `BOT`, `REP`; 0 = no piece), read
/// from `cmex10.tfm` with `tftopl`. `var_delimiter` builds the tall forms out
/// of these, so the growth chain has to end on the right one.
const RECIPE: &[(&str, [u8; 4])] = &[
    (r"\Vert", [0, 0, 0, 0x0D]),
    (r"\vert", [0, 0, 0, 0x0C]),
    (r"\updownarrow", [0x78, 0, 0x79, 0x3F]),
    (r"\Updownarrow", [0x7E, 0, 0x7F, 0x77]),
    (r"\lgroup", [0x38, 0, 0x3A, 0x3E]),
    (r"\rgroup", [0x39, 0, 0x3B, 0x3E]),
    (r"\lmoustache", [0x38, 0, 0x3B, 0x3E]),
    (r"\rmoustache", [0x39, 0, 0x3A, 0x3E]),
    (r"\arrowvert", [0, 0, 0, 0x3F]),
    (r"\Arrowvert", [0, 0, 0, 0x77]),
    (r"\bracevert", [0, 0, 0, 0x3E]),
];

fn near(a: f64, b: f64, what: &str) {
    assert!(
        (a - b).abs() < 0.0002,
        "{what}: {a:.5} != pdfTeX's {b:.5}"
    );
}

#[test]
fn every_kernel_delimiter_has_both_halves_of_its_pair() {
    for (command, ch, small, large, _) in DELIMITERS {
        let slot = cm::delimiter_slot(*ch)
            .unwrap_or_else(|| panic!("{command} has no delimiter pair"));
        assert_eq!(slot, (*small, *large), "{command} pair");
        // The small variant is also the character's ordinary slot, so
        // `$\sym$` outside `\left`/`\right` sets the same glyph.
        assert_eq!(
            cm::symbol_slot(*ch),
            Some(*small),
            "{command} ordinary slot"
        );
    }
}

#[test]
fn small_variant_boxes_are_pdftexs() {
    let m = CmMathMetrics::latex_10pt();
    for (command, ch, ..) in DELIMITERS {
        let (_, w, h, d) = SMALL
            .iter()
            .find(|(c, ..)| c == command)
            .unwrap_or_else(|| panic!("no pdfTeX box for {command}"));
        let g = m
            .glyph(*ch, SizeClass::Text)
            .unwrap_or_else(|| panic!("{command} has no glyph"));
        near(g.width, *w, &format!("{command} width"));
        near(g.height, *h, &format!("{command} height"));
        near(g.depth, *d, &format!("{command} depth"));
        // The first size `var_delimiter` considers is that same glyph.
        let sizes = m.delimiter_sizes(*ch, SizeClass::Text);
        assert!(!sizes.is_empty(), "{command} has no delimiter sizes");
        near(sizes[0].width, *w, &format!("{command} first size width"));
    }
}

#[test]
fn growth_ends_on_the_recipe_pdftex_uses() {
    let m = CmMathMetrics::latex_10pt();
    for (command, ch, ..) in DELIMITERS {
        let (_, grown) = GROWN_WIDTH
            .iter()
            .find(|(c, _)| c == command)
            .unwrap_or_else(|| panic!("no pdfTeX grown width for {command}"));
        match RECIPE.iter().find(|(c, _)| c == command) {
            Some((_, pieces)) => {
                let ext = m
                    .delimiter_extensible(*ch, SizeClass::Text)
                    .unwrap_or_else(|| panic!("{command} has no extensible recipe"));
                let got = [
                    ext.top.as_ref().map_or(0, |g| g.gid as u8),
                    ext.mid.as_ref().map_or(0, |g| g.gid as u8),
                    ext.bot.as_ref().map_or(0, |g| g.gid as u8),
                    ext.rep.gid as u8,
                ];
                assert_eq!(got, *pieces, "{command} extensible pieces");
                // Every piece of a vertical recipe is as wide as the whole.
                near(ext.rep.width, *grown, &format!("{command} grown width"));
            }
            // `\backslash` has no recipe: the chain ends at a discrete size,
            // and that largest size is what a tall fence gets.
            None => {
                assert!(
                    m.delimiter_extensible(*ch, SizeClass::Text).is_none(),
                    "{command} was not expected to be extensible"
                );
                let last = m
                    .delimiter_sizes(*ch, SizeClass::Text)
                    .pop()
                    .unwrap_or_else(|| panic!("{command} has no sizes"));
                near(last.width, *grown, &format!("{command} largest size width"));
            }
        }
    }
}

#[test]
fn classes_are_fontmaths() {
    for (command, ch, _, _, declared) in DELIMITERS {
        // `default_class` answers with the class a *bare* occurrence of the
        // character takes. Open and Close add no space beside an ordinary, so
        // the `$a\sym b$` measurement cannot separate them from Ord; Rel and
        // Ord it can, and every one of the twelve is asserted against
        // `fontmath.ltx`'s declaration either way.
        let got = default_class(*ch).0;
        if *ch == '\u{2216}' {
            // `\backslash` is the one of the twelve whose character is not
            // its own: `fontmath.ltx` 483 puts it on cmsy "6E, the slot
            // `\setminus` (147, `\mathbin`) already owns, and Unicode has one
            // code point for the two. The table keeps U+2216 a binary, which
            // is what a bare `\setminus` must stay, and the compiler forces
            // Ord at `\backslash` the way it does for `\bot`/`\perp`.
            assert_eq!(got, AtomClass::Bin, "{command} shares \\setminus's row");
            continue;
        }
        assert_eq!(got, *declared, "{command} class");
    }
    // The cmsy slots `\Vert`/`\vert`/`\backslash` share with `\parallel`,
    // `\mid` and `\setminus`, which are *not* ordinaries: the delimiters have
    // to be distinct characters, which is what these three pin.
    assert_eq!(default_class('\u{2225}').0, AtomClass::Rel, "\\parallel");
    assert_eq!(default_class('\u{2223}').0, AtomClass::Rel, "\\mid");
    assert_eq!(default_class('\u{2216}').0, AtomClass::Bin, "\\setminus");
}

#[test]
fn arrowvert_and_vert_grow_differently_on_the_same_cmsy_bar() {
    // The reason `\Arrowvert` cannot simply be U+2016: it sets the same small
    // glyph as `\Vert` but grows through cmex's extension recipe "3D, whose
    // repeated piece is cmex "77, not "0D.
    let m = CmMathMetrics::latex_10pt();
    let vert = m.delimiter_extensible('\u{2016}', SizeClass::Text).expect("\\Vert");
    let arrow = m
        .delimiter_extensible(cm::ARROWVERT_DOUBLE, SizeClass::Text)
        .expect("\\Arrowvert");
    assert_eq!(vert.rep.gid, 0x0D);
    assert_eq!(arrow.rep.gid, 0x77);
    assert_ne!(vert.rep.width, arrow.rep.width);
    // Same story for the single bar (`\vert` "0C vs `\arrowvert` "3F).
    let bar = m.delimiter_extensible('|', SizeClass::Text).expect("\\vert");
    let abar = m.delimiter_extensible('\u{23D0}', SizeClass::Text).expect("\\arrowvert");
    assert_eq!(bar.rep.gid, 0x0C);
    assert_eq!(abar.rep.gid, 0x3F);
}
