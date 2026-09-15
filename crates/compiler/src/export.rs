//! Export glyph adapter: what the PDF path can and cannot represent.
//!
//! The compiler lays text out in Unicode. The PDF writer (`crates/pdf`, FT-009)
//! encodes with the base-14 fonts, whose repertoire is much smaller. Before this
//! module existed, the mismatch was discovered only at export time and resolved
//! by writing `?` — a student's exported equation silently lost its symbols.
//! Issue #9 records the reproduction.
//!
//! This table is the shared representation the two crates agree on. It is
//! exported from here so `crates/pdf` can consume it rather than re-deriving the
//! mapping and drifting apart again.
//!
//! What it deliberately does NOT do is invent a substitute. A character with no
//! base-14 glyph is reported as [`Glyph::Unrepresentable`] with a reason, and
//! the compiler raises a diagnostic so the author learns before exporting.

use crate::char_table::CharTable;
use crate::math::FRACTION_RULE_CHAR;

/// One of the 14 standard PDF fonts, or none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFont {
    /// Text faces encoded with WinAnsiEncoding.
    Text,
    /// The `Symbol` face, which carries Greek letters and mathematical operators.
    Symbol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    /// Encodable: use this font and this byte in that font's encoding.
    Encodable { font: ExportFont, code: u8 },
    /// Drawn from the pinned Latin Modern Math resource (`lm.math`), which the
    /// PDF writer embeds. When that font is not installed, the writer reports
    /// its own substitution warning at export time, so the compiler stays quiet.
    LatinModernMath,
    /// No base-14 glyph exists. The reason is shown to the author.
    Unrepresentable { reason: &'static str },
}

/// Adobe Symbol encoding, for the symbols this compiler emits.
///
/// Values are the code points in the Symbol font's own encoding, not Unicode.
pub(crate) const SYMBOL_ENCODING: &[(char, u8)] = &[
    ('\u{3B1}', 0x61),  // alpha
    ('\u{3B2}', 0x62),  // beta
    ('\u{3B3}', 0x67),  // gamma
    ('\u{3B4}', 0x64),  // delta
    ('\u{3B8}', 0x71),  // theta
    ('\u{3BB}', 0x6C),  // lambda
    ('\u{3BC}', 0x6D),  // mu
    ('\u{3C0}', 0x70),  // pi
    ('\u{3C3}', 0x73),  // sigma
    ('\u{3C6}', 0x66),  // phi
    ('\u{3C9}', 0x77),  // omega
    ('\u{D7}', 0xB4),   // multiply
    ('\u{F7}', 0xB8),   // divide
    ('\u{B1}', 0xB1),   // plusminus
    ('\u{2212}', 0x2D), // minus (math-mode `-`)
    ('\u{2264}', 0xA3), // lessequal
    ('\u{2265}', 0xB3), // greaterequal
    ('\u{2260}', 0xB9), // notequal
    ('\u{2248}', 0xBB), // approxequal
    ('\u{B7}', 0xD7),   // dotmath
    ('\u{221E}', 0xA5), // infinity
    ('\u{2211}', 0xE5), // summation
    ('\u{222B}', 0xF2), // integral
    ('\u{221A}', 0xD6), // radical
    ('\u{2208}', 0xCE), // element of (\in)
    ('\u{2200}', 0x22), // for all (\forall)
    ('\u{2203}', 0x24), // there exists (\exists)
    ('\u{2228}', 0xDA), // logical or (\vee)
    ('\u{21D2}', 0xDE), // double right arrow (\Rightarrow)
    ('\u{2223}', 0x7C), // verticalbar (\mid)
    ('\u{3B5}', 0x65),  // epsilon (\varepsilon)
    ('\u{3F5}', 0x65),  // lunate epsilon (\epsilon): Symbol has only the open form
    ('\u{3B6}', 0x7A),  // zeta
    ('\u{3B7}', 0x68),  // eta
    ('\u{3D1}', 0x4A),  // theta1 (\vartheta)
    ('\u{3B9}', 0x69),  // iota
    ('\u{3BA}', 0x6B),  // kappa
    ('\u{3BD}', 0x6E),  // nu
    ('\u{3BE}', 0x78),  // xi
    ('\u{3D6}', 0x76),  // omega1 (\varpi)
    ('\u{3C1}', 0x72),  // rho
    ('\u{3C2}', 0x56),  // sigma1 (\varsigma)
    ('\u{3C4}', 0x74),  // tau
    ('\u{3C5}', 0x75),  // upsilon
    ('\u{3D5}', 0x6A),  // phi1 (\varphi)
    ('\u{3C7}', 0x63),  // chi
    ('\u{3C8}', 0x79),  // psi
    ('\u{393}', 0x47),  // Gamma
    ('\u{394}', 0x44),  // Delta
    ('\u{398}', 0x51),  // Theta
    ('\u{39B}', 0x4C),  // Lambda
    ('\u{39E}', 0x58),  // Xi
    ('\u{3A0}', 0x50),  // Pi
    ('\u{3A3}', 0x53),  // Sigma
    ('\u{3A5}', 0x55),  // Upsilon
    ('\u{3A6}', 0x46),  // Phi
    ('\u{3A8}', 0x59),  // Psi
    ('\u{3A9}', 0x57),  // Omega
    ('\u{2261}', 0xBA), // equivalence
    ('\u{223C}', 0x7E), // similar
    ('\u{2245}', 0x40), // congruent
    ('\u{221D}', 0xB5), // proportional
    ('\u{22A5}', 0x5E), // perpendicular
    ('\u{2202}', 0xB6), // partialdiff
    ('\u{2207}', 0xD1), // gradient
    ('\u{220F}', 0xD5), // product
    ('\u{2217}', 0x2A), // asteriskmath
    ('\u{2032}', 0xA2), // minute (\prime)
    ('\u{222A}', 0xC8), // union
    ('\u{2229}', 0xC7), // intersection
    ('\u{22C5}', 0xD7), // dotmath (\cdots)
    ('\u{2282}', 0xCC), // propersubset
    ('\u{2286}', 0xCD), // reflexsubset
    ('\u{2283}', 0xC9), // propersuperset
    ('\u{2287}', 0xCA), // reflexsuperset
    ('\u{2209}', 0xCF), // notelement
    ('\u{220B}', 0x27), // suchthat (\ni)
    ('\u{2205}', 0xC6), // emptyset
    ('\u{2295}', 0xC5), // circleplus
    ('\u{2297}', 0xC4), // circlemultiply
    ('\u{2227}', 0xD9), // logicaland
    ('\u{2192}', 0xAE), // arrowright
    ('\u{2190}', 0xAC), // arrowleft
    ('\u{2191}', 0xAD), // arrowup
    ('\u{2193}', 0xAF), // arrowdown
    ('\u{2194}', 0xAB), // arrowboth
    ('\u{21D0}', 0xDC), // arrowdblleft
    ('\u{21D4}', 0xDB), // arrowdblboth
    ('\u{21D1}', 0xDD), // arrowdblup
    ('\u{21D3}', 0xDF), // arrowdbldown
    ('\u{2234}', 0x5C), // therefore
    ('\u{2220}', 0xD0), // angle
    ('\u{2135}', 0xC0), // aleph
    ('\u{211C}', 0xC2), // Rfraktur
    ('\u{2111}', 0xC1), // Ifraktur
    ('\u{2118}', 0xC3), // weierstrass
    ('\u{2329}', 0xE1), // angleleft
    ('\u{232A}', 0xF1), // angleright
    ('\u{27E8}', 0xE1), // mathematical angle bracket (\langle): Symbol's angleleft
    ('\u{27E9}', 0xF1), // mathematical angle bracket (\rangle): Symbol's angleright
];

/// WinAnsiEncoding's 0x80..0x9F block, which is NOT Latin-1.
///
/// This block is where WinAnsi puts typographic punctuation: em and en dashes,
/// curly quotes, the ellipsis and the bullet. Treating WinAnsi as plain Latin-1
/// wrongly reports all of them as unexportable, and they are among the most
/// common non-ASCII characters in ordinary prose — an em dash in a sentence
/// would have warned the author that their PDF was lossy when it was not.
pub(crate) const WINANSI_HIGH: &[(char, u8)] = &[
    ('\u{20AC}', 0x80), // Euro
    ('\u{201A}', 0x82), // single low quote
    ('\u{192}', 0x83),  // florin
    ('\u{201E}', 0x84), // double low quote
    ('\u{2026}', 0x85), // ellipsis
    ('\u{2020}', 0x86), // dagger
    ('\u{2021}', 0x87), // double dagger
    ('\u{2C6}', 0x88),  // circumflex
    ('\u{2030}', 0x89), // per mille
    ('\u{160}', 0x8A),  // S caron
    ('\u{2039}', 0x8B), // single left guillemet
    ('\u{152}', 0x8C),  // OE
    ('\u{17D}', 0x8E),  // Z caron
    ('\u{2018}', 0x91), // left single quote
    ('\u{2019}', 0x92), // right single quote
    ('\u{201C}', 0x93), // left double quote
    ('\u{201D}', 0x94), // right double quote
    ('\u{2022}', 0x95), // bullet
    ('\u{2013}', 0x96), // en dash
    ('\u{2014}', 0x97), // em dash
    ('\u{2DC}', 0x98),  // small tilde
    ('\u{2122}', 0x99), // trademark
    ('\u{161}', 0x9A),  // s caron
    ('\u{203A}', 0x9B), // single right guillemet
    ('\u{153}', 0x9C),  // oe
    ('\u{17E}', 0x9E),  // z caron
    ('\u{178}', 0x9F),  // Y dieresis
];

/// How a single character would be exported.
pub fn map_char(c: char) -> Glyph {
    if c == FRACTION_RULE_CHAR {
        return Glyph::Unrepresentable {
            reason: "fraction rules are drawn with a box-drawing character as a stand-in; \
                     runtime-v1 has no rule item type yet, so they cannot be exported faithfully",
        };
    }
    if crate::lm_math::advance(c).is_some() {
        return Glyph::LatinModernMath;
    }
    if crate::amssymb::newcm_advance(c).is_some() {
        return Glyph::Unrepresentable {
            reason: "this amssymb symbol is drawn from New Computer Modern Math (newcm.math), \
                     which the Mac producer bundles but the base-14 PDF writer does not embed",
        };
    }
    if crate::newcm_math::advance(c).is_some() {
        return Glyph::Unrepresentable {
            reason: "\\mathcal letters are drawn from New Computer Modern Math (newcm.math), \
                     which the Mac producer bundles but the base-14 PDF writer does not embed",
        };
    }
    static SYMBOL_INDEX: CharTable<u8> = CharTable::new(SYMBOL_ENCODING);
    static WINANSI_HIGH_INDEX: CharTable<u8> = CharTable::new(WINANSI_HIGH);
    if let Some(code) = SYMBOL_INDEX.get(c) {
        return Glyph::Encodable {
            font: ExportFont::Symbol,
            code,
        };
    }
    if let Some(code) = WINANSI_HIGH_INDEX.get(c) {
        return Glyph::Encodable {
            font: ExportFont::Text,
            code,
        };
    }
    // Below U+0100 WinAnsi agrees with Latin-1, EXCEPT the 0x80..0x9F block
    // handled above, which Latin-1 leaves as control codes.
    if (c as u32) < 0x100 && c != '\u{7F}' && (c as u32) >= 0xA0
        || (0x20..0x7F).contains(&(c as u32))
    {
        return Glyph::Encodable {
            font: ExportFont::Text,
            code: c as u32 as u8,
        };
    }
    Glyph::Unrepresentable {
        reason: "no glyph for this character exists in the base-14 PDF fonts",
    }
}

/// Characters in `text` that cannot be exported, in order, without duplicates.
pub fn unrepresentable(text: &str) -> Vec<char> {
    let mut out: Vec<char> = Vec::new();
    for c in text.chars() {
        // Printable ASCII is always WinAnsi-encodable (pinned by the
        // `printable_ascii_is_always_exportable` test). Skipping the linear
        // table scans here keeps the per-keystroke export check cheap: at
        // 500 KB it was ~10 ms of every warm compile_result (issue #65).
        if (' '..='~').contains(&c) {
            continue;
        }
        if matches!(map_char(c), Glyph::Unrepresentable { .. }) && !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

/// The reason a character cannot be exported, for use in a diagnostic.
pub fn reason(c: char) -> Option<&'static str> {
    match map_char(c) {
        Glyph::Unrepresentable { reason } => Some(reason),
        Glyph::Encodable { .. } | Glyph::LatinModernMath => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::COMMAND_GLYPHS;

    /// The guard that keeps this table honest: every symbol the math layer can
    /// emit must have a decided export outcome. Adding a symbol to
    /// `COMMAND_GLYPHS` without considering export fails here.
    #[test]
    fn every_math_symbol_has_a_decided_export_outcome() {
        let minus = [("-", crate::math::MINUS_SIGN)];
        for (command, glyph) in COMMAND_GLYPHS.iter().chain(&minus) {
            for c in glyph.chars() {
                match map_char(c) {
                    Glyph::Encodable { font, .. } => {
                        assert_eq!(
                            font,
                            ExportFont::Symbol,
                            "\\{command} renders {c:?}, which should come from the Symbol font"
                        );
                    }
                    // The only decided non-base-14 outcome: a glyph bound to
                    // the pinned Latin Modern Math resource, which is embedded.
                    Glyph::LatinModernMath => {}
                    Glyph::Unrepresentable { reason } => {
                        panic!("\\{command} renders {c:?} which cannot be exported: {reason}");
                    }
                }
            }
        }
        // amssymb symbols: Latin Modern Math, or the decided New Computer
        // Modern Math outcome (bundled by the Mac producer, not embedded by
        // the base-14 writer) for the few Latin Modern Math lacks.
        for symbol in crate::amssymb::SYMBOLS {
            for c in symbol.text.chars() {
                match map_char(c) {
                    Glyph::LatinModernMath => {}
                    Glyph::Unrepresentable { .. } if crate::amssymb::newcm_advance(c).is_some() => {
                    }
                    other => panic!("\\{} renders {c:?}: {other:?}", symbol.name),
                }
            }
        }
    }

    #[test]
    fn the_fraction_rule_is_reported_not_substituted() {
        let g = map_char(FRACTION_RULE_CHAR);
        let Glyph::Unrepresentable { reason } = g else {
            panic!("the fraction rule stand-in must not claim to be encodable");
        };
        assert!(
            reason.contains("rule item type"),
            "reason should name the contract gap: {reason}"
        );
    }

    #[test]
    fn mathematical_angle_brackets_encode_as_symbol_angles() {
        for (math, symbol) in [('\u{27E8}', '\u{2329}'), ('\u{27E9}', '\u{232A}')] {
            assert_eq!(map_char(math), map_char(symbol));
            assert!(matches!(
                map_char(math),
                Glyph::Encodable {
                    font: ExportFont::Symbol,
                    ..
                }
            ));
        }
    }

    #[test]
    fn ordinary_text_and_latin1_encode_as_text() {
        for c in ['A', 'z', '0', ' ', '.', 'é', 'ü', '±'] {
            assert!(
                matches!(map_char(c), Glyph::Encodable { .. }),
                "{c:?} should be encodable"
            );
        }
        assert_eq!(
            map_char('A'),
            Glyph::Encodable {
                font: ExportFont::Text,
                code: 0x41
            }
        );
        // A character that genuinely has no base-14 glyph.
        assert!(matches!(map_char('日'), Glyph::Unrepresentable { .. }));
    }

    #[test]
    fn unrepresentable_lists_each_offender_once() {
        let text = format!("a{0}b{0}c日日", FRACTION_RULE_CHAR);
        assert_eq!(unrepresentable(&text), vec![FRACTION_RULE_CHAR, '日']);
    }
}

#[cfg(test)]
mod winansi_tests {
    use super::*;

    #[test]
    fn typographic_punctuation_is_exportable() {
        // These live in WinAnsi's 0x80..0x9F block, not in Latin-1. Reporting an
        // em dash as unexportable would tell an author their PDF is lossy when
        // it is not.
        for (c, expected) in [
            ('\u{2014}', 0x97u8), // em dash
            ('\u{2013}', 0x96),   // en dash
            ('\u{2018}', 0x91),   // left single quote
            ('\u{2019}', 0x92),   // right single quote
            ('\u{201C}', 0x93),   // left double quote
            ('\u{201D}', 0x94),   // right double quote
            ('\u{2026}', 0x85),   // ellipsis
            ('\u{2022}', 0x95),   // bullet
        ] {
            assert_eq!(
                map_char(c),
                Glyph::Encodable {
                    font: ExportFont::Text,
                    code: expected
                },
                "{c:?} should encode as WinAnsi 0x{expected:02X}"
            );
        }
    }

    #[test]
    fn characters_with_genuinely_no_base14_glyph_still_report() {
        // CJK has no glyph in any base-14 face. This must stay honest.
        for c in ['東', '京', '\u{1F600}'] {
            assert!(
                matches!(map_char(c), Glyph::Unrepresentable { .. }),
                "{c:?} has no base-14 glyph and must be reported"
            );
        }
    }
}
