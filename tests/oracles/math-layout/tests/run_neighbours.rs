//! A run cut out of a longer formula at top-level glue (`\dots\,+b` is
//! `\dots`, a thin space, then `+b`) keeps the classes the whole formula
//! gives its edge atoms: TeX classifies the formula at once and glue is not
//! a noad, so Rule 5 judges the `+` against the Inner before the glue and
//! it stays Bin, with `\medmuskip` on both sides (pdfTeX: `+` at 163.666 bp,
//! `b` 2.211 bp after it, in a 10 pt article).

use flashtex_math_layout::layout::{effective_classes, effective_classes_in};
use flashtex_math_layout::{
    Atom, AtomClass, CmMathMetrics, MathList, Neighbours, Style, layout_in_context,
    layout_with_report,
};

fn before(class: AtomClass) -> Neighbours {
    Neighbours { before: Some(class), after: None }
}

fn after(class: AtomClass) -> Neighbours {
    Neighbours { before: None, after: Some(class) }
}

#[test]
fn a_leading_bin_is_judged_against_the_noad_before_the_run() {
    use AtomClass::*;
    let run = [Atom::bin('+'), Atom::ord('b')];
    assert_eq!(effective_classes(&run), [Ord, Ord]);
    assert_eq!(effective_classes_in(&run, Neighbours::default()), [Ord, Ord]);
    assert_eq!(effective_classes_in(&run, before(Inner)), [Bin, Ord]);
    assert_eq!(effective_classes_in(&run, before(Ord)), [Bin, Ord]);
    assert_eq!(effective_classes_in(&run, before(Close)), [Bin, Ord]);
    for demoting in [Bin, Op, Rel, Open, Punct] {
        assert_eq!(effective_classes_in(&run, before(demoting)), [Ord, Ord], "{demoting:?}");
    }
}

#[test]
fn a_trailing_bin_is_judged_against_the_noad_after_the_run() {
    use AtomClass::*;
    let run = [Atom::ord('a'), Atom::bin('+')];
    assert_eq!(effective_classes(&run), [Ord, Ord]);
    assert_eq!(effective_classes_in(&run, after(Ord)), [Ord, Bin]);
    assert_eq!(effective_classes_in(&run, after(Open)), [Ord, Bin]);
    for demoting in [Rel, Close, Punct] {
        assert_eq!(effective_classes_in(&run, after(demoting)), [Ord, Ord], "{demoting:?}");
    }
}

#[test]
fn glue_inside_the_run_does_not_hide_the_neighbour() {
    use AtomClass::*;
    let run = [Atom::glue(3.0, 0.0), Atom::bin('+'), Atom::ord('b')];
    assert_eq!(effective_classes_in(&run, before(Inner))[1..], [Bin, Ord]);
}

#[test]
fn a_bin_kept_by_its_neighbour_is_spaced_inside_the_run() {
    let m = CmMathMetrics::latex_10pt();
    let run = MathList::new(vec![Atom::bin('+'), Atom::ord('b')]);
    let alone = layout_with_report(&run, Style::TEXT, &m).root.width;
    let cut = layout_in_context(&run, Style::TEXT, &m, before(AtomClass::Inner)).root.width;
    // `\medmuskip` 4mu at 10 pt: 4/18 of cmsy10's quad (1.000003 em as a
    // TFM fixword, scaled).
    assert!((cut - alone - 40.0 / 18.0).abs() < 1e-3, "{alone} -> {cut}");
    assert_eq!(layout_in_context(&run, Style::TEXT, &m, Neighbours::default()), layout_with_report(&run, Style::TEXT, &m));
}
