//! `\mathcal` capitals drawn from New Computer Modern Math, not the base-14 fonts,
//! plus `\Diamond`'s U+25C7 (issue #591): Latin Modern Math has no such glyph,
//! so this is the only bundled face that carries it.
//!
//! The Mac app bundles `apps/mac/Fonts/NewCMMath-Regular.otf` (New Computer
//! Modern Math 7.1.1, copied byte-for-byte from TeX Live / MacTeX 2026 and
//! pinned in `apps/mac/Fonts/SUPPLEMENTARY-FACES.json`). The compiler emits the
//! real Unicode script capitals (Mathematical Alphanumeric U+1D49C–U+1D4B5,
//! with the eight letters Unicode encodes in Letterlike Symbols) and binds them
//! to that font program. Advance widths come from its `hmtx` table;
//! `tests/newcm_math_binding.rs` re-reads the file and fails if the digest or
//! any advance drifts. This follows the `crate::lm_math` binding pattern.
//!
//! Fidelity limitation: pdfLaTeX draws `\mathcal` from `cmsy10`. The default
//! script capitals in New Computer Modern Math are not that calligraphic
//! design, and a few advances differ from cmsy10's width plus italic
//! correction by more than 0.5pt at 10pt (H, J, K, O, Q, Y, among others; see
//! the pdfLaTeX oracle table in the binding test). `\mathcal{P}`, the letter
//! HW2 uses, is within 0.15pt.

/// Resource id for items drawn from this font (not yet in the font-engine
/// manifest; the Mac producer resolves it by family/PostScript name).
pub const FONT_ID: &str = "newcm.math";
/// SHA-256 of the exact font program the advances below were read from.
pub const SHA256: &str = "60394d357348f68cd301764fe61cc502a5858e1c4ff21b948a1d14d82586a7a2";
/// `font-hints-v1` family for items drawn from this resource.
pub const FAMILY: &str = "New Computer Modern Math";
pub const UNITS_PER_EM: f64 = 1000.0;

/// Every glyph the compiler emits from this resource, with its advance width
/// in font units.
pub const ADVANCES: &[(char, u16)] = &[
    ('\u{1D49C}', 857),  // \mathcal{A}
    ('\u{212C}', 778),   // \mathcal{B}
    ('\u{1D49E}', 654),  // \mathcal{C}
    ('\u{1D49F}', 871),  // \mathcal{D}
    ('\u{2130}', 613),   // \mathcal{E}
    ('\u{2131}', 904),   // \mathcal{F}
    ('\u{1D4A2}', 685),  // \mathcal{G}
    ('\u{210B}', 1065),  // \mathcal{H}
    ('\u{2110}', 620),   // \mathcal{I}
    ('\u{1D4A5}', 698),  // \mathcal{J}
    ('\u{1D4A6}', 989),  // \mathcal{K}
    ('\u{2112}', 770),   // \mathcal{L}
    ('\u{2133}', 1149),  // \mathcal{M}
    ('\u{1D4A9}', 1007), // \mathcal{N}
    ('\u{1D4AA}', 699),  // \mathcal{O}
    ('\u{1D4AB}', 763),  // \mathcal{P}
    ('\u{1D4AC}', 716),  // \mathcal{Q}
    ('\u{211B}', 818),   // \mathcal{R}
    ('\u{1D4AE}', 625),  // \mathcal{S}
    ('\u{1D4AF}', 776),  // \mathcal{T}
    ('\u{1D4B0}', 744),  // \mathcal{U}
    ('\u{1D4B1}', 710),  // \mathcal{V}
    ('\u{1D4B2}', 1028), // \mathcal{W}
    ('\u{1D4B3}', 870),  // \mathcal{X}
    ('\u{1D4B4}', 628),  // \mathcal{Y}
    ('\u{1D4B5}', 726),  // \mathcal{Z}
    // `\Diamond` (issue #591): latexsym's `\mathord` (lasy "33, U+25C7), a
    // different glyph from the kernel `\diamond` (U+22C4). Latin Modern Math
    // carries no U+25C7 (`tools/kernel-math-gap/fontprobe.py`), so this is
    // the only bundled face that can draw it; the advance here is this
    // program's own (1025/1000 em), while `\Diamond` atoms lay out at the
    // real lasy advance (`crate::math::DIAMOND_LASY_EM`) — the same
    // TFM-width plus bundled-ink split the amssymb table uses.
    ('\u{25C7}', 1025), // \Diamond
];

/// The script code point for `\mathcal{letter}`: the Mathematical
/// Alphanumeric block, except the eight capitals Unicode encodes in
/// Letterlike Symbols (whose Alphanumeric slots are reserved).
pub fn script(letter: char) -> Option<char> {
    let exception = match letter {
        'B' => Some('\u{212C}'),
        'E' => Some('\u{2130}'),
        'F' => Some('\u{2131}'),
        'H' => Some('\u{210B}'),
        'I' => Some('\u{2110}'),
        'L' => Some('\u{2112}'),
        'M' => Some('\u{2133}'),
        'R' => Some('\u{211B}'),
        _ => None,
    };
    match letter {
        'A'..='Z' => exception.or_else(|| char::from_u32(0x1D49C + (letter as u32 - 'A' as u32))),
        _ => None,
    }
}

pub fn advance(c: char) -> Option<u16> {
    ADVANCES
        .iter()
        .find(|(glyph, _)| *glyph == c)
        .map(|(_, advance)| *advance)
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
    fn every_capital_has_exactly_one_script_glyph_in_the_table() {
        let mut seen = Vec::new();
        for letter in 'A'..='Z' {
            let glyph = script(letter).expect("capital letter");
            assert!(advance(glyph).is_some(), "{letter} -> {glyph:?} missing");
            assert!(
                crate::lm_math::advance(glyph).is_none(),
                "{glyph:?} bound twice"
            );
            assert!(!seen.contains(&glyph));
            seen.push(glyph);
        }
        assert_eq!(script('P'), Some('\u{1D4AB}'));
        assert_eq!(script('B'), Some('ℬ'));
        assert_eq!(script('p'), None);
        // 26 script capitals plus `\Diamond`'s U+25C7 (issue #591), the one
        // non-`\mathcal` entry: the only bundled face carrying that glyph.
        assert_eq!(ADVANCES.len(), 27);
    }

    #[test]
    fn widths_scale_from_font_units() {
        assert_eq!(width_pt("\u{1D4AB}", 10.0), Some(7.63));
        assert_eq!(width_pt("\u{1D4AB}x", 10.0), None);
    }
}
