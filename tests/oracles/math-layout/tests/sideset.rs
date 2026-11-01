//! amsmath `\sideset` (`Atom::left_scripts`) against pdfTeX.
//!
//! Every number is from pdfTeX 1.40 (TeX Live, `/Library/TeX/texbin/pdflatex`)
//! `\showbox` of `\hbox{$..$}` in a 10pt `article` loading `amsmath`, with the
//! `\displaystyle` variant measured separately. pdfTeX's structure is
//! `\hbox to\dimen@{}`, `\thinmuskip` 1.66663, then `\mathop{\kern-\dimen@
//! \box4\box6}`: box 4 is `\vbox(10.50006+5.50006)x0` (the display `\sum`,
//! cmex10 "58 14.44447 wide, `shifted -9.50006`) carrying the left scripts,
//! box 6 the display `\sum\nolimits` with the right ones. Positions are pt,
//! x rightward from the formula's origin and `y` downward from its baseline.
//! The tolerance is far inside the 0.5bp (0.50187pt) acceptance bound.

use flashtex_math_layout::{
    Atom, AtomClass, CmMathMetrics, MathList, Nucleus, PositionedGlyph, PositionedRuns, Style,
    layout, positioned_runs,
};

const TOL: f64 = 1e-3;

fn run(list: &MathList, style: Style) -> (PositionedRuns, f64) {
    let b = layout(list, style, &CmMathMetrics::latex_10pt());
    if list.atoms.len() == 2 {
        // Every `\sideset{..}{..}\sum` measured here is 12.88896+6.00005.
        close("height", b.height, 12.88896);
        close("depth", b.depth, 6.00005);
    }
    // `positioned_runs` puts the origin at the box's top edge.
    let r = positioned_runs(&b, (0.0, -b.height));
    (r, b.width)
}

fn letters(text: &str) -> MathList {
    MathList::new(text.chars().map(Atom::ord).collect())
}

/// `\sideset{left}{right}\sum` as a builder writes it: the ordinary
/// `\hbox to\dimen@{}` atom, then the operator carrying both pairs.
fn sideset(left: (Option<&str>, Option<&str>), right: (Option<&str>, Option<&str>)) -> Vec<Atom> {
    let mut op = Atom::op('\u{2211}').with_left_scripts(left.0.map(letters), left.1.map(letters));
    op.superscript = right.0.map(letters);
    op.subscript = right.1.map(letters);
    vec![Atom::new(AtomClass::Ord, Nucleus::Empty), op]
}

fn nth(r: &PositionedRuns, ch: char, n: usize) -> &PositionedGlyph {
    r.glyphs
        .iter()
        .filter(|g| g.ch == ch)
        .nth(n)
        .unwrap_or_else(|| panic!("no glyph #{n} {ch:?} in {:?}", r.glyphs))
}

fn close(what: &str, got: f64, want: f64) {
    assert!(
        (got - want).abs() <= TOL,
        "{what}: got {got:.5}, pdfTeX {want:.5}"
    );
}

#[test]
fn sideset_ab_cd_sum_matches_pdftex_inline_and_display() {
    // \showbox: \hbox(12.88896+6.00005)x25.61162 in both styles. Left pair
    // at 0.17477 - 0.17477 + 1.66663; b `\vbox(18.889+0.0)x4.83765, shifted
    // 6.00005` puts b's baseline 18.889 - 6.00005 - 4.8611 = 8.02785 up and
    // a's 6.00005 down; the display sum follows box 4 (4.83765 wide), and
    // box 6's scripts follow the 14.44447 glyph.
    let list = MathList::new(sideset((Some("b"), Some("a")), (Some("d"), Some("c"))));
    for style in [Style::TEXT, Style::DISPLAY] {
        let (r, width) = run(&list, style);
        let sum = nth(&r, '\u{2211}', 0);
        close("width", width, 25.61162);
        close("b x", nth(&r, 'b', 0).x, 1.66663);
        close("b y", nth(&r, 'b', 0).baseline_y, -8.02785);
        close("a x", nth(&r, 'a', 0).x, 1.66663);
        close("a y", nth(&r, 'a', 0).baseline_y, 6.00005);
        close("sum x", sum.x, 6.50428);
        close("sum y", sum.baseline_y, -9.50006);
        close("sum width", sum.width, 14.44447);
        assert_eq!(sum.size, 10.0, "{style:?}");
        close("d x", nth(&r, 'd', 0).x, 20.94875);
        close("d y", nth(&r, 'd', 0).baseline_y, -8.02785);
        close("c x", nth(&r, 'c', 0).x, 20.94875);
        close("c y", nth(&r, 'c', 0).baseline_y, 6.00005);
        assert_eq!(nth(&r, 'b', 0).size, 7.0);
    }
}

#[test]
fn unequal_left_scripts_are_left_aligned() {
    // `\sideset{_{ab}^b}{}\sum`: \hbox(12.88896+6.00005)x24.46541; box 4 is
    // 8.35431 wide (ab + \scriptspace) and both scripts start at its left
    // edge, 1.66663; the sum is at 10.02094 and box 6 has no scripts.
    let list = MathList::new(sideset((Some("b"), Some("ab")), (None, None)));
    for style in [Style::TEXT, Style::DISPLAY] {
        let (r, width) = run(&list, style);
        close("width", width, 24.46541);
        close("sup b x", nth(&r, 'b', 0).x, 1.66663);
        close("sup b y", nth(&r, 'b', 0).baseline_y, -8.02785);
        close("sub a x", nth(&r, 'a', 0).x, 1.66663);
        close("sub a y", nth(&r, 'a', 0).baseline_y, 6.00005);
        close("sub b x", nth(&r, 'b', 1).x, 1.66663 + 4.33765);
        close("sum x", nth(&r, '\u{2211}', 0).x, 10.02094);
        close("sum width", nth(&r, '\u{2211}', 0).width, 14.44447);
    }
}

#[test]
fn prime_only_sideset_spaces_like_ord_then_op() {
    // `x\sideset{}{'}\sum x`: \hbox(11.91676+5.50006)x32.01382. The leading
    // `\hbox to\dimen@{}` is ordinary, so x gets no space before it and only
    // the thin space before the \mathop; box 4 is empty and 0 wide; the
    // prime is `shifted -8.02786` (10.50006 - sup_drop of cmsy7).
    let mut atoms = vec![Atom::ord('x')];
    let mut side = sideset((None, None), (None, None));
    side[1].superscript = Some(MathList::new(vec![Atom::ord('\u{2032}')]));
    atoms.extend(side);
    atoms.push(Atom::ord('x'));
    let (r, width) = run(&MathList::new(atoms), Style::TEXT);
    close("width", width, 32.01382);
    close("sum x", nth(&r, '\u{2211}', 0).x, 7.38190);
    close("prime x", nth(&r, '\u{2032}', 0).x, 21.82637);
    close("prime y", nth(&r, '\u{2032}', 0).baseline_y, -8.02786);
    close("last x", nth(&r, 'x', 1).x, 26.29855);
}
