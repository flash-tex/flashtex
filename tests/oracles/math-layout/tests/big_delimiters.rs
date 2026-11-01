//! `\big`/`\Big`/`\bigg`/`\Bigg` under the two definitions that exist.
//!
//! The LaTeX kernel (`fontmath.ltx` 513-520) sets them with `\vbox to` an
//! **absolute** 8.5 / 11.5 / 14.5 / 17.5 pt; `amsmath` (`amsmath.sty`
//! 721-738 `\bBigg@`) redefines them as `\vcenter to` 1 / 1.5 / 2 / 2.5
//! times `\big@size`, which is 1.2 × the math strut and so follows the math
//! size. The two agree at 10 pt and diverge at every other body size, so a
//! 10 pt-only comparison cannot tell them apart.
//!
//! Reference numbers are `\showbox` under pdfTeX 3.141592653-2.6-1.40.27
//! (TeX Live 2025) for `\setbox0=\hbox{$\displaystyle \Big[$}` and friends
//! in an `article`, as the box's own height and depth in TeX pt. The oracle
//! `pdflatex` never runs here; the numbers are transcribed.

use flashtex_math_layout::cm::ExtensionSizing;
use flashtex_math_layout::{
    Atom, AtomClass, BigSizing, CmMathMetrics, MathList, Style, layout,
};

/// `(command, amsmath factor, kernel \vbox to <pt>)`.
const COMMANDS: [(&str, f64, f64); 4] = [
    ("\\big", 1.0, 8.5),
    ("\\Big", 1.5, 11.5),
    ("\\bigg", 2.0, 14.5),
    ("\\Bigg", 2.5, 17.5),
];

/// `pdflatex` `\showbox` height and depth of `\hbox{$\displaystyle <cmd>[$}`.
///
/// The first column is the **math text size** the class declares, which is
/// what `\DeclareMathSizes` sets: 10pt and 12pt articles use their body
/// size, an 11pt article uses `\@xipt` = 10.95pt (`size11.clo`).
///
/// `(math text pt, [big, Big, bigg, Bigg] as (height, depth))`
const KERNEL: [(f64, [(f64, f64); 4]); 3] = [
    (10.0, [(8.50005, 3.50006), (11.50008, 6.50009), (14.50010, 9.50012), (17.50014, 12.50015)]),
    (10.95, [(8.50000, 2.73750), (11.73756, 6.26260), (14.73760, 9.26263), (17.73763, 12.26266)]),
    (12.0, [(9.00000, 3.00000), (12.00008, 6.00009), (15.00010, 9.00012), (18.00014, 12.00015)]),
];

const AMSMATH: [(f64, [(f64, f64); 4]); 3] = [
    (10.0, [(8.50005, 3.50006), (11.50008, 6.50009), (14.50010, 9.50012), (17.50014, 12.50015)]),
    (10.95, [(9.30754, 3.83258), (12.59258, 7.11761), (15.87761, 10.40265), (19.16264, 13.68767)]),
    (12.0, [(10.20006, 4.20007), (13.80010, 7.80011), (17.40013, 11.40015), (21.00017, 15.00018)]),
];

/// pdfTeX rounds to 5 decimals and scales through 2^-16 pt, so agreement to
/// 1e-4 pt is exact agreement.
const TOLERANCE_PT: f64 = 1e-4;

fn box_of(atom: Atom, text_pt: f64, amsmath: bool) -> (f64, f64) {
    let mut m = CmMathMetrics::for_text_size(text_pt);
    if amsmath {
        // amsfonts/amsmath redeclare `OMX/cmex/m/n` without `sfixed`.
        m = m.with_extension(ExtensionSizing::Designs);
    }
    let b = layout(&MathList::new(vec![atom]), Style::TEXT, &m);
    (b.height, b.depth)
}

/// Without `amsmath` the target is an absolute length, so the delimiter does
/// not grow with the body size: `\Big[` is pdfTeX's `cmex` `h` (18.00017 pt
/// of height plus depth) at 10, 11 and 12 pt alike.
#[test]
fn kernel_big_delimiters_match_pdftex_at_every_body_size() {
    for (text_pt, expected) in KERNEL {
        for (i, (name, _, pt)) in COMMANDS.iter().enumerate() {
            let atom = Atom::big_delimiter_kernel(AtomClass::Open, Some('['), *pt);
            let (h, d) = box_of(atom, text_pt, false);
            let (eh, ed) = expected[i];
            assert!(
                (h - eh).abs() < TOLERANCE_PT && (d - ed).abs() < TOLERANCE_PT,
                "{name}[ at {text_pt}pt: ({h}+{d}), pdfTeX ({eh}+{ed})"
            );
        }
    }
}

/// With `amsmath` the target is 1.2 × the math strut, so it does.
#[test]
fn amsmath_big_delimiters_match_pdftex_at_every_body_size() {
    for (text_pt, expected) in AMSMATH {
        for (i, (name, factor, _)) in COMMANDS.iter().enumerate() {
            let atom = Atom::big_delimiter(AtomClass::Open, Some('['), *factor);
            let (h, d) = box_of(atom, text_pt, true);
            let (eh, ed) = expected[i];
            assert!(
                (h - eh).abs() < TOLERANCE_PT && (d - ed).abs() < TOLERANCE_PT,
                "{name}[ at {text_pt}pt: ({h}+{d}), pdfTeX ({eh}+{ed})"
            );
        }
    }
}

/// The two rules genuinely differ: at 11 and 12 pt every one of the four
/// commands lands on a different box, which is the error this models. (At
/// 10 pt they agree exactly, which is why it hid.)
#[test]
fn the_two_rules_agree_at_ten_point_and_differ_above_it() {
    for (text_pt, _) in KERNEL {
        for (name, factor, pt) in COMMANDS {
            let k = box_of(Atom::big_delimiter_kernel(AtomClass::Open, Some('['), pt), text_pt, false);
            let a = box_of(Atom::big_delimiter(AtomClass::Open, Some('['), factor), text_pt, true);
            let same = (k.0 - a.0).abs() < TOLERANCE_PT && (k.1 - a.1).abs() < TOLERANCE_PT;
            assert_eq!(
                same,
                text_pt == 10.0,
                "{name} at {text_pt}pt: kernel {k:?}, amsmath {a:?}"
            );
        }
    }
}

/// `BigSizing::kernel_for_factor` is the mapping between the two spellings
/// of the same four commands.
#[test]
fn kernel_lengths_map_from_the_amsmath_factors() {
    for (name, factor, pt) in COMMANDS {
        match BigSizing::kernel_for_factor(factor) {
            BigSizing::Kernel { pt: got } => {
                assert!((got - pt).abs() < 1e-9, "{name}: {got}pt, expected {pt}pt")
            }
            other => panic!("{name}: {other:?}"),
        }
    }
}
