//! Inter-atom spacing (TeXbook chapter 18, the 8×8 table; Appendix G Rule 20).

use crate::mathlist::AtomClass;
use crate::style::Style;

/// The amount of space between two adjacent atoms.
///
/// The muskip values are plain.tex's (TeX Live 2026
/// `tex/plain/base/plain.tex` lines 373–375), which LaTeX keeps:
/// `\thinmuskip=3mu`, `\medmuskip=4mu plus 2mu minus 4mu`,
/// `\thickmuskip=5mu plus 5mu`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    None,
    /// `\thinmuskip` = 3mu.
    Thin,
    /// `\medmuskip` = 4mu plus 2mu minus 4mu.
    Medium,
    /// `\thickmuskip` = 5mu plus 5mu.
    Thick,
}

impl Space {
    /// The natural width in mu.
    pub fn mu(self) -> f64 {
        match self {
            Space::None => 0.0,
            Space::Thin => 3.0,
            Space::Medium => 4.0,
            Space::Thick => 5.0,
        }
    }

    /// The stretch in mu.
    pub fn stretch_mu(self) -> f64 {
        match self {
            Space::Medium => 2.0,
            Space::Thick => 5.0,
            Space::None | Space::Thin => 0.0,
        }
    }

    /// The shrink in mu.
    pub fn shrink_mu(self) -> f64 {
        match self {
            Space::Medium => 4.0,
            Space::None | Space::Thin | Space::Thick => 0.0,
        }
    }
}

// Table entries: 0 = none, 1 = thin, 2 = medium, 3 = thick; the second element
// says whether the space is suppressed in script and scriptscript styles (the
// parenthesised entries of the TeXbook table). `*` (impossible) is modelled as
// none because Bin atoms are converted to Ord before spacing (Rules 5, 6).
//
const T: bool = true;
const F: bool = false;
//                  Ord     Op      Bin     Rel     Open    Close   Punct   Inner
#[rustfmt::skip]
const TABLE: [[(u8, bool); 8]; 8] = [
    /* Ord   */ [(0, F), (1, F), (2, T), (3, T), (0, F), (0, F), (0, F), (1, T)],
    /* Op    */ [(1, F), (1, F), (0, F), (3, T), (0, F), (0, F), (0, F), (1, T)],
    /* Bin   */ [(2, T), (2, T), (0, F), (0, F), (2, T), (0, F), (0, F), (2, T)],
    /* Rel   */ [(3, T), (3, T), (0, F), (0, F), (3, T), (0, F), (0, F), (3, T)],
    /* Open  */ [(0, F), (0, F), (0, F), (0, F), (0, F), (0, F), (0, F), (0, F)],
    /* Close */ [(0, F), (1, F), (2, T), (3, T), (0, F), (0, F), (0, F), (1, T)],
    /* Punct */ [(1, T), (1, T), (0, F), (1, T), (1, T), (1, T), (1, T), (1, T)],
    /* Inner */ [(1, T), (1, F), (2, T), (3, T), (1, T), (0, F), (1, T), (1, T)],
];

/// Space between a `left` atom and a `right` atom in `style`.
pub fn between(left: AtomClass, right: AtomClass, style: Style) -> Space {
    let (amount, only_uncompressed) = TABLE[left.index()][right.index()];
    if only_uncompressed && !style.is_uncompressed() {
        return Space::None;
    }
    match amount {
        1 => Space::Thin,
        2 => Space::Medium,
        3 => Space::Thick,
        _ => Space::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use AtomClass::*;

    #[test]
    fn texbook_table_spot_checks() {
        assert_eq!(between(Ord, Bin, Style::TEXT), Space::Medium);
        assert_eq!(between(Ord, Bin, Style::SCRIPT), Space::None);
        assert_eq!(between(Ord, Rel, Style::DISPLAY), Space::Thick);
        assert_eq!(between(Ord, Op, Style::SCRIPT_SCRIPT), Space::Thin);
        assert_eq!(between(Op, Ord, Style::SCRIPT), Space::Thin);
        assert_eq!(between(Ord, Open, Style::TEXT), Space::None);
        assert_eq!(between(Close, Open, Style::TEXT), Space::None);
        assert_eq!(between(Punct, Ord, Style::TEXT), Space::Thin);
        assert_eq!(between(Punct, Ord, Style::SCRIPT), Space::None);
        assert_eq!(between(Inner, Op, Style::SCRIPT), Space::Thin);
        assert_eq!(between(Inner, Close, Style::TEXT), Space::None);
    }
}
