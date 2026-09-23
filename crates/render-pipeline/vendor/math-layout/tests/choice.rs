//! `\mathchoice` (tex.web §731, §1174): the list for the current style
//! replaces the choice and is spliced into the enclosing list.

use flashtex_math_layout::{Atom, AtomClass, CmMathMetrics, MathList, Nucleus, Style, layout};

fn choice(branches: [char; 4]) -> Atom {
    let lists = branches.map(|c| MathList::new(vec![Atom::ord(c)]));
    Atom::new(AtomClass::Ord, Nucleus::Choice(Box::new(lists)))
}

fn width(atoms: Vec<Atom>, style: Style) -> f64 {
    layout(&MathList::new(atoms), style, &CmMathMetrics::latex_10pt()).width
}

#[test]
fn each_style_sets_its_own_branch() {
    // Branch glyphs of clearly different widths in cmmi: m, i, W, l.
    let branches = ['m', 'i', 'W', 'l'];
    for (style, ch) in [
        (Style::DISPLAY, 'm'),
        (Style::TEXT, 'i'),
        (Style::SCRIPT, 'W'),
        (Style::SCRIPT_SCRIPT, 'l'),
        (Style::TEXT.cramped(), 'i'),
        (Style::SCRIPT.cramped(), 'W'),
    ] {
        let got = width(vec![choice(branches)], style);
        let want = width(vec![Atom::ord(ch)], style);
        assert!((got - want).abs() < 1e-9, "{style:?}: {got} vs {ch} {want}");
    }
}

#[test]
fn the_chosen_atoms_are_spliced_and_space_like_their_neighbours() {
    // `a\mathchoice{+}{+}{+}{+}b` is `a+b` with a binary `+` (medium spaces),
    // not `a{+}b`, where the boxed group is an Ord and gets no space.
    let plus = MathList::new(vec![Atom::bin('+')]);
    let spliced = Atom::new(
        AtomClass::Ord,
        Nucleus::Choice(Box::new([plus.clone(), plus.clone(), plus.clone(), plus.clone()])),
    );
    let got = width(vec![Atom::ord('a'), spliced, Atom::ord('b')], Style::TEXT);
    let bin = width(vec![Atom::ord('a'), Atom::bin('+'), Atom::ord('b')], Style::TEXT);
    let boxed = width(
        vec![Atom::ord('a'), Atom::new(AtomClass::Ord, Nucleus::List(plus)), Atom::ord('b')],
        Style::TEXT,
    );
    assert!((got - bin).abs() < 1e-9, "{got} vs {bin}");
    assert!(got > boxed + 1.0, "{got} vs boxed {boxed}");
}

#[test]
fn a_choice_inside_the_chosen_branch_resolves_in_the_same_style() {
    let inner = choice(['m', 'i', 'W', 'l']);
    let outer = Atom::new(
        AtomClass::Ord,
        Nucleus::Choice(Box::new([
            MathList::new(vec![inner.clone()]),
            MathList::new(vec![inner.clone()]),
            MathList::new(vec![inner.clone()]),
            MathList::new(vec![inner]),
        ])),
    );
    assert!((width(vec![outer], Style::DISPLAY) - width(vec![Atom::ord('m')], Style::DISPLAY)).abs() < 1e-9);
}

#[test]
fn a_choice_with_a_script_keeps_the_script() {
    let mut scripted = choice(['m', 'i', 'W', 'l']);
    scripted.superscript = Some(MathList::new(vec![Atom::ord('2')]));
    let bare = width(vec![choice(['m', 'i', 'W', 'l'])], Style::TEXT);
    let got = width(vec![scripted], Style::TEXT);
    assert!(got > bare + 1.0, "the superscript is set: {got} vs {bare}");
}
