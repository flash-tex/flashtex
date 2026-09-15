//! Glyphs drawn from the pinned Latin Modern Math resource, not the base-14 fonts.
//!
//! Blackboard bold, the cmsy circled operators, `\setminus`, the long `\Longrightarrow` arrow, a
//! further set of common amssymb/latexsym symbols (issue #62: `\mp`, `\ll`,
//! `\gg`, `\simeq`, `\vdots`, `\ddots`, the floor/ceiling fences, `\oint`,
//! `\mapsto`, `\ell`, `\hbar`, `\circ`, `\parallel`, and the relations/order
//! symbols below), and the HW2 follow-up (the remaining long double/single
//! arrows, `\triangle`/`\bigtriangleup`/`\bigtriangledown`, and the proof QED
//! mark), and the base LaTeX2e cmsy square relations
//! (`\sqcup`/`\sqcap`/`\sqsubseteq`/`\sqsupseteq`), have no glyph in the
//! base-14 Symbol face. Rather than
//! substitute a look-alike, the compiler emits their real Unicode code points
//! and binds them to the `lm.math` resource the font-engine manifest already
//! pins (the same file the Mac app bundles as
//! `apps/mac/Fonts/latinmodern-math.otf`). Advance widths come from that exact
//! font program; `tests/lm_math_binding.rs` re-reads the file and fails if the
//! digest or any advance drifts.
//!
//! Fidelity limitation: this is the `unicode-math` design of Latin Modern
//! Math. pdfLaTeX draws `\mathbb` from `msbm10` and `\setminus` from `cmsy10`,
//! whose widths differ (for example R is 0.83pt narrower here at 10pt), so
//! output using these glyphs does not have pixel parity with pdfLaTeX.

/// Font-engine manifest id of the resource.
pub const FONT_ID: &str = "lm.math";
/// SHA-256 of the exact font program the advances below were read from.
pub const SHA256: &str = "6075562b771f8b82f0c179e363389684f2dd09de30038269e2628e504bd7be0f";
/// `font-hints-v1` family for items drawn from this resource.
pub const FAMILY: &str = "Latin Modern Math";
pub const UNITS_PER_EM: f64 = 1000.0;

/// Every glyph the compiler emits from this resource, with its advance width
/// in font units.
pub const ADVANCES: &[(char, u16)] = &[
    ('\u{1D538}', 611), // \mathbb{A}
    ('\u{1D539}', 639), // \mathbb{B}
    ('\u{2102}', 667),  // \mathbb{C}
    ('\u{1D53B}', 694), // \mathbb{D}
    ('\u{1D53C}', 611), // \mathbb{E}
    ('\u{1D53D}', 611), // \mathbb{F}
    ('\u{1D53E}', 667), // \mathbb{G}
    ('\u{210D}', 722),  // \mathbb{H}
    ('\u{1D540}', 334), // \mathbb{I}
    ('\u{1D541}', 639), // \mathbb{J}
    ('\u{1D542}', 639), // \mathbb{K}
    ('\u{1D543}', 611), // \mathbb{L}
    ('\u{1D544}', 722), // \mathbb{M}
    ('\u{2115}', 722),  // \mathbb{N}
    ('\u{1D546}', 667), // \mathbb{O}
    ('\u{2119}', 639),  // \mathbb{P}
    ('\u{211A}', 667),  // \mathbb{Q}
    ('\u{211D}', 639),  // \mathbb{R}
    ('\u{1D54A}', 611), // \mathbb{S}
    ('\u{1D54B}', 611), // \mathbb{T}
    ('\u{1D54C}', 722), // \mathbb{U}
    ('\u{1D54D}', 611), // \mathbb{V}
    ('\u{1D54E}', 833), // \mathbb{W}
    ('\u{1D54F}', 667), // \mathbb{X}
    ('\u{1D550}', 611), // \mathbb{Y}
    ('\u{2124}', 667),  // \mathbb{Z}
    ('\u{2216}', 568),  // \setminus
    ('\u{27F9}', 1457), // \Longrightarrow
    // amssymb/latexsym symbols with no base-14 Symbol glyph (issue #62).
    ('\u{2213}', 778),  // \mp
    ('\u{226A}', 1000), // \ll
    ('\u{226B}', 1000), // \gg
    ('\u{2243}', 778),  // \simeq
    ('\u{22EE}', 218),  // \vdots
    ('\u{22F1}', 613),  // \ddots
    ('\u{230A}', 444),  // \lfloor
    ('\u{230B}', 444),  // \rfloor
    ('\u{2308}', 444),  // \lceil
    ('\u{2309}', 444),  // \rceil
    ('\u{222E}', 665),  // \oint
    ('\u{21A6}', 977),  // \mapsto
    ('\u{2113}', 417),  // \ell
    ('\u{210F}', 576),  // \hbar
    ('\u{2218}', 412),  // \circ
    ('\u{2225}', 500),  // \parallel
    ('\u{2016}', 398),  // \| / \Vert / \lVert / \rVert
    ('\u{2224}', 388),  // \nmid
    ('\u{2270}', 778),  // \nleq
    ('\u{2271}', 778),  // \ngeq
    ('\u{228A}', 778),  // \subsetneq
    ('\u{228B}', 778),  // \supsetneq
    // The kernel cmsy square relations (fontmath.ltx 278-279, 301-302). The
    // base-14 Symbol face has no square cup/cap or square subset, so they are
    // bound to this resource like the rest above. These advances match what
    // pdfLaTeX sets from cmsy10 at 10pt (measured: `\sqcup` 6.66669pt against
    // 667/1000 em, `\sqsubseteq` 7.7778pt against 778/1000 em).
    ('\u{2294}', 667), // \sqcup
    ('\u{2293}', 667), // \sqcap
    ('\u{2291}', 778), // \sqsubseteq
    ('\u{2292}', 778), // \sqsupseteq
    // Kernel cmsy10 circled operators; advances measured with hb-shape from
    // apps/mac/Fonts/latinmodern-math.otf (font units).
    ('\u{2296}', 778),  // \ominus, cmsy10 "09
    ('\u{2298}', 778),  // \oslash, cmsy10 "0B
    ('\u{2299}', 778),  // \odot, cmsy10 "0C
    ('\u{25EF}', 1013), // \bigcirc, cmsy10 "0D
    ('\u{2272}', 776),  // \lesssim
    ('\u{2273}', 776),  // \gtrsim
    ('\u{225C}', 778),  // \triangleq
    ('\u{2254}', 906),  // \coloneqq
    ('\u{2204}', 556),  // \nexists
    ('\u{2201}', 556),  // \complement
    ('\u{21DD}', 997),  // \rightsquigarrow
    ('\u{21AA}', 997),  // \hookrightarrow
    ('\u{21C6}', 1018), // \leftrightarrows
    ('\u{22A8}', 612),  // \models
    ('\u{22A2}', 611),  // \vdash
    ('\u{22A3}', 611),  // \dashv
    ('\u{22A4}', 778),  // \top
    ('\u{2221}', 778),  // \measuredangle
    ('\u{25A1}', 778),  // \square
    ('\u{25A0}', 778),  // \blacksquare
    ('\u{25CA}', 572),  // \lozenge
    ('\u{2713}', 833),  // \checkmark
    // HW2 coverage (issue #62 follow-up): long arrows, \triangle family, \bot,
    // and the amsthm QED mark, all drawn from the same pinned resource.
    ('\u{27FA}', 1534), // \Longleftrightarrow, and \iff (\;\Longleftrightarrow\;)
    ('\u{27F6}', 1463), // \longrightarrow
    ('\u{27F5}', 1463), // \longleftarrow
    ('\u{27F8}', 1457), // \Longleftarrow, and \impliedby (\;\Longleftarrow\;)
    ('\u{27F7}', 1442), // \longleftrightarrow
    // \triangle (Ord) and \bigtriangleup (Bin) share this one real glyph;
    // \bigtriangleup gets a per-atom class override rather than a second glyph.
    ('\u{25B3}', 968), // \triangle, \bigtriangleup
    ('\u{25BD}', 968), // \bigtriangledown (a distinct glyph, no override needed)
    // \bot shares \perp's exact U+22A5 glyph; only its class differs (Ord vs
    // Rel), handled with a per-atom class override, not a second glyph.
    // Proof QED mark (U+220E): not emitted by any command here yet, but
    // pinning its advance stops it being reported as an unrepresentable/lossy
    // export if it reaches an item's text (e.g. typed literally by an
    // amsthm-style proof ending).
    ('\u{220E}', 666), // ∎ QED
];

/// The double-struck code point for `\mathbb{letter}`: the Mathematical
/// Alphanumeric block, except the seven letters Unicode encodes in
/// Letterlike Symbols (whose Alphanumeric slots are reserved).
pub fn double_struck(letter: char) -> Option<char> {
    let exception = match letter {
        'C' => Some('\u{2102}'),
        'H' => Some('\u{210D}'),
        'N' => Some('\u{2115}'),
        'P' => Some('\u{2119}'),
        'Q' => Some('\u{211A}'),
        'R' => Some('\u{211D}'),
        'Z' => Some('\u{2124}'),
        _ => None,
    };
    match letter {
        'A'..='Z' => exception.or_else(|| char::from_u32(0x1D538 + (letter as u32 - 'A' as u32))),
        _ => None,
    }
}

pub fn advance(c: char) -> Option<u16> {
    static INDEX: crate::char_table::CharTable<u16> = crate::char_table::CharTable::new(ADVANCES);
    INDEX
        .get(c)
        // amssymb/amsfonts symbols bound to the same resource
        // (`crate::amssymb::LM_ADVANCES`, generated from this font program).
        .or_else(|| crate::amssymb::lm_advance(c))
}

/// True when `text` is non-empty and every character comes from this resource.
pub fn covers(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| advance(c).is_some())
}

/// Width of `text` at `size` points, when every character is covered.
pub fn width_pt(text: &str, size: f64) -> Option<f64> {
    if !covers(text) {
        return None;
    }
    Some(
        text.chars()
            .filter_map(advance)
            .map(|units| f64::from(units) * size / UNITS_PER_EM)
            .sum(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_capital_has_exactly_one_double_struck_glyph_in_the_table() {
        let mut seen = Vec::new();
        for letter in 'A'..='Z' {
            let glyph = double_struck(letter).expect("capital letter");
            assert!(advance(glyph).is_some(), "{letter} -> {glyph:?} missing");
            assert!(!seen.contains(&glyph));
            seen.push(glyph);
        }
        assert_eq!(double_struck('R'), Some('ℝ'));
        assert_eq!(double_struck('A'), Some('\u{1D538}'));
        assert_eq!(double_struck('a'), None);
        assert_eq!(double_struck('1'), None);
        assert_eq!(ADVANCES.len(), 84);
    }

    #[test]
    fn widths_scale_from_font_units() {
        assert_eq!(width_pt("ℝ", 10.0), Some(6.39));
        assert_eq!(width_pt("ℝx", 10.0), None);
        assert_eq!(width_pt("", 10.0), None);
    }
}
