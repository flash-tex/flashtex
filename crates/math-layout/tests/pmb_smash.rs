//! amsbsy `\pmb` and latex.ltx/amsmath `\smash` as math-layout atoms.

use flashtex_math_layout::{Atom, AtomClass, CmMathMetrics, MathList, Nucleus, Style, layout};

fn body() -> MathList {
    MathList::new(vec![Atom::ord('x')])
}

fn boxed(n: Nucleus, style: Style) -> flashtex_math_layout::MathBox {
    layout(&MathList::new(vec![Atom::new(AtomClass::Ord, n)]), style, &CmMathMetrics::latex_10pt())
}

#[test]
fn pmb_keeps_the_width_and_raises_the_height_by_half_a_mu() {
    let m = CmMathMetrics::latex_10pt();
    for style in [Style::DISPLAY, Style::TEXT, Style::SCRIPT, Style::SCRIPT_SCRIPT] {
        let plain = boxed(Nucleus::List(body()), style);
        let pmb = boxed(Nucleus::Pmb(body()), style);
        let mu = flashtex_math_layout::MathFontMetrics::params(&m, style.size_class()).mu();
        assert!((pmb.width - plain.width).abs() < 1e-9, "{style:?}");
        assert!((pmb.height - (plain.height + 0.5 * mu)).abs() < 1e-9, "{style:?}");
        assert!((pmb.depth - plain.depth).abs() < 1e-9, "{style:?}");
    }
}

#[test]
fn smash_zeroes_the_requested_dimensions_only() {
    let deep = MathList::new(vec![Atom::ord('y')]);
    let plain = boxed(Nucleus::List(deep.clone()), Style::TEXT);
    assert!(plain.height > 0.0 && plain.depth > 0.0);
    for (top, bottom) in [(true, true), (true, false), (false, true), (false, false)] {
        let b = boxed(Nucleus::Smash { body: deep.clone(), top, bottom }, Style::TEXT);
        assert!((b.width - plain.width).abs() < 1e-9);
        assert_eq!(b.height, if top { 0.0 } else { plain.height }, "{top} {bottom}");
        assert_eq!(b.depth, if bottom { 0.0 } else { plain.depth }, "{top} {bottom}");
    }
}
