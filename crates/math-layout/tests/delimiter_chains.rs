//! The growth chain of every delimiter, pinned against pdfTeX's.
//!
//! `\left<d>` picks the first variant in a chain — the small character from
//! the text/symbol family, then `cmex`'s `\nextlarger` successors, then the
//! extensible recipe (tex.web §713) — whose height plus depth reaches Rule
//! 19's target. Three separate reports have described a chain step here as
//! wrong after measuring the *painted Latin Modern Math outline's ink*
//! rather than the TeX box: `\Bigg(`'s ink is 29.90 pt inside its correct
//! 30.00029 pt box because the OpenType paren's round ends do not touch the
//! box edge, and one cmex extension piece's ink is 12.02 pt inside its
//! correct 6.00006 pt box because the OpenType bar is drawn to be overlapped.
//! Neither is a layout error. This file pins the boxes, which are what every
//! surrounding atom is positioned from, so the distinction stays measurable.
//!
//! Reference numbers are `\showbox` under pdfTeX 3.141592653-2.6-1.40.27
//! (TeX Live 2025): `\setbox0=\hbox{$\displaystyle\left<d>\vcenter to
//! <n>pt{}\right.$}` swept over `n` = 1..40 pt in an `article`, reading the
//! chosen delimiter box's own height, depth and width. The oracle `pdflatex`
//! never runs here; the numbers are transcribed. Each row is `(character,
//! [(height+depth, width)] for the fixed chain, the extensible recipe's
//! repeated piece width)`.
//!
//! The three `cmex` sizes are the three a document actually gets: 10 pt is a
//! 10 pt `article` (the kernel loads `OMX/cmex` `sfixed`, so cmex10 at its
//! design size), and 10.95/12 pt are 11 and 12 pt `article`s loading
//! `amsmath`, which redeclares the shape without `sfixed`.

use flashtex_math_layout::cm::ExtensionSizing;
use flashtex_math_layout::metrics::{MathFontMetrics, SizeClass};
use flashtex_math_layout::CmMathMetrics;

type Chain = (char, &'static [(f64, f64)], Option<f64>);

const PDFTEX: [(f64, &[Chain]); 3] = [
    // cmex at 10 pt
    (10.0, &[
        ('(', &[(10.0, 3.8889), (12.00011, 4.58336), (18.00017, 5.97223), (24.00023, 7.36115), (30.00029, 7.91669)], Some(8.75002)),
        ('[', &[(10.0, 2.77779), (12.00011, 4.16669), (18.00017, 4.72223), (24.00023, 5.2778), (30.00029, 5.83336)], Some(6.66669)),
        ('{', &[(10.0, 5.00002), (12.00011, 5.83336), (18.00017, 6.66669), (24.00023, 7.50002), (30.00029, 8.05559)], Some(8.8889)),
        ('|', &[(10.0, 2.77779)], Some(3.33333)),
        ('\u{2016}', &[(10.0, 5.00002)], Some(5.55557)),
        ('\u{2308}', &[(10.0, 4.44444), (12.00011, 4.72223), (18.00017, 5.2778), (24.00023, 5.83336), (30.00029, 6.3889)], Some(6.66669)),
        ('\u{230A}', &[(10.0, 4.44444), (12.00011, 4.72223), (18.00017, 5.2778), (24.00023, 5.83336), (30.00029, 6.3889)], Some(6.66669)),
        ('\u{27E8}', &[(10.0, 3.8889), (12.00011, 4.72223), (18.00017, 6.11111), (24.00023, 7.50002), (30.00029, 8.05559)], None),
    ]),
    // cmex at 10.95 pt
    (10.95, &[
        ('(', &[(10.94999, 4.25835), (13.14012, 5.01877), (19.71019, 6.5396), (26.28026, 8.06044), (32.85031, 8.66878)], Some(9.58127)),
        ('[', &[(10.94999, 3.04167), (13.14012, 4.56252), (19.71019, 5.17085), (26.28026, 5.77919), (32.85031, 6.38751)], Some(7.30002)),
        ('{', &[(10.94999, 5.475), (13.14012, 6.38751), (19.71019, 7.30002), (26.28026, 8.21251), (32.85031, 8.82088)], Some(9.73335)),
        ('|', &[(10.94999, 3.04167)], Some(3.65)),
        ('\u{2016}', &[(10.94999, 5.475)], Some(6.08334)),
        ('\u{2308}', &[(10.94999, 4.86667), (13.14012, 5.17085), (19.71019, 5.77919), (26.28026, 6.38751), (32.85031, 6.99585)], Some(7.30002)),
        ('\u{230A}', &[(10.94999, 4.86667), (13.14012, 5.17085), (19.71019, 5.77919), (26.28026, 6.38751), (32.85031, 6.99585)], Some(7.30002)),
        ('\u{27E8}', &[(10.94999, 4.25835), (13.14012, 5.17085), (19.71019, 6.69168), (26.28026, 8.21251), (32.85031, 8.82088)], None),
    ]),
    // cmex at 12 pt
    (12.0, &[
        ('(', &[(12.0, 4.5694), (14.40013, 5.50003), (21.6002, 7.16669), (28.80028, 8.83337), (36.00035, 9.50003)], Some(10.50003)),
        ('[', &[(12.0, 3.26385), (14.40013, 5.00002), (21.6002, 5.66669), (28.80028, 6.33336), (36.00035, 7.00003)], Some(8.00002)),
        ('{', &[(12.0, 6.00002), (14.40013, 7.00003), (21.6002, 8.00002), (28.80028, 9.00002), (36.00035, 9.66672)], Some(10.66669)),
        ('|', &[(12.0, 3.33334)], Some(4.0)),
        ('\u{2016}', &[(12.0, 6.00002)], Some(6.66669)),
        ('\u{2308}', &[(12.0, 5.33334), (14.40013, 5.66669), (21.6002, 6.33336), (28.80028, 7.00003), (36.00035, 7.66669)], Some(8.00002)),
        ('\u{230A}', &[(12.0, 5.33334), (14.40013, 5.66669), (21.6002, 6.33336), (28.80028, 7.00003), (36.00035, 7.66669)], Some(8.00002)),
        ('\u{27E8}', &[(12.0, 4.66667), (14.40013, 5.66669), (21.6002, 7.33334), (28.80028, 9.00002), (36.00035, 9.66672)], None),
    ]),
];

/// pdfTeX prints 5 decimals and scales through 2^-16 pt, so agreement to
/// 1e-4 pt is exact agreement.
const TOLERANCE_PT: f64 = 1e-4;

fn metrics(text_pt: f64) -> CmMathMetrics {
    CmMathMetrics::for_text_size(text_pt).with_extension(ExtensionSizing::Designs)
}

fn near(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < TOLERANCE_PT,
        "{what}: {actual}, pdfTeX {expected}"
    );
}

/// Every fixed step of every chain — the small variant and each `cmex`
/// successor — has pdfTeX's height plus depth and pdfTeX's width.
#[test]
fn fixed_delimiter_chains_match_pdftex_at_every_cmex_size() {
    for (text_pt, chains) in PDFTEX {
        for (ch, expected, _) in chains {
            let got = metrics(text_pt).delimiter_sizes(*ch, SizeClass::Text);
            assert_eq!(
                got.len(),
                expected.len(),
                "{ch:?} at {text_pt}pt: {} sizes, pdfTeX {}",
                got.len(),
                expected.len()
            );
            for (i, (total, width)) in expected.iter().enumerate() {
                let g = &got[i];
                near(g.height + g.depth, *total, &format!("{ch:?} size {i} at {text_pt}pt"));
                near(g.width, *width, &format!("{ch:?} size {i} width at {text_pt}pt"));
            }
        }
    }
}

/// Past the fixed chain, the recipe's repeated piece is the one pdfTeX
/// stacks, at pdfTeX's width. `\langle` has no recipe in either.
#[test]
fn extensible_recipes_repeat_the_piece_pdftex_stacks() {
    for (text_pt, chains) in PDFTEX {
        for (ch, _, rep_width) in chains {
            let got = metrics(text_pt).delimiter_extensible(*ch, SizeClass::Text);
            match (got, rep_width) {
                (Some(r), Some(w)) => {
                    near(r.rep.width, *w, &format!("{ch:?} rep width at {text_pt}pt"))
                }
                (None, None) => {}
                (a, b) => panic!("{ch:?} at {text_pt}pt: recipe {:?}, pdfTeX {b:?}", a.is_some()),
            }
        }
    }
}

/// `\Vert` (U+2016) and `\mid` (U+2223) are different symbols with different
/// chains: plain.tex gives them cmsy `"6B`/`"6A` and cmex `"0D`/`"0C`. This
/// is the distinction `crates/compiler` erased by spelling `\|` as two
/// `\mid`s — 5.55557 pt of extensible width against 3.33333 at 10 pt.
#[test]
fn the_double_bar_and_the_single_bar_have_different_chains() {
    for (text_pt, _) in PDFTEX {
        let m = metrics(text_pt);
        for single in ['|', '\u{2223}'] {
            for double in ['\u{2016}', '\u{2225}'] {
                let s = m.delimiter_extensible(single, SizeClass::Text).expect("single bar recipe");
                let d = m.delimiter_extensible(double, SizeClass::Text).expect("double bar recipe");
                assert!(
                    d.rep.width > s.rep.width + TOLERANCE_PT,
                    "{text_pt}pt: {double:?} rep {} is not wider than {single:?} rep {}",
                    d.rep.width,
                    s.rep.width
                );
            }
        }
    }
}
