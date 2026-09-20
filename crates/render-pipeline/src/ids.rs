//! Three distinct identities that must never be cast into one another:
//!
//! * [`EncodingCode`]: an 8-bit slot in a TeX font encoding (T1 here). TFM
//!   metrics and TeX input conventions (`--`, ``` `` ```) are expressed in
//!   these; they are neither Unicode nor glyph indices.
//! * `char`: a Unicode scalar value, the currency of the compiler's parse tree
//!   and of the shaper's input.
//! * [`GlyphId`]: an ORIGINAL glyph index inside one specific font program
//!   (font-engine's type, re-exported). Only meaningful together with the
//!   content hash of that program.
//!
//! The only ways across are the explicit functions below: an encoding slot
//! resolves to a `char` through a declared encoding table, and a `char`
//! resolves to a `GlyphId` through one face's `cmap`. Nothing here is a
//! numeric conversion.

pub use flashtex_font_engine::GlyphId;

use flashtex_font_engine::Face;

/// An 8-bit character code in a TeX font encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EncodingCode(pub u8);

/// The TeX font encodings this pipeline can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Encoding {
    /// Cork (`T1`, `\usepackage[T1]{fontenc}`), the encoding of `ec-lm*` and
    /// `ptm*8t` and the one every fixture selects.
    T1,
}

/// The Cork (T1) table, slot by slot (`t1enc.def` / `ec` fonts). Slots
/// 0x00–0x0C are floating accents with no single Unicode spelling and are
/// left undeclared; 0x17 (compound word mark) and 0x18 (perthousandzero)
/// have no character either. Everything else is declared explicitly.
const T1_TO_UNICODE: &[(u8, char)] = &[
    (0x0D, '\u{201A}'), // quotesinglbase
    (0x0E, '\u{2039}'), // guilsinglleft
    (0x0F, '\u{203A}'), // guilsinglright
    (0x10, '\u{201C}'), // quotedblleft  (``)
    (0x11, '\u{201D}'), // quotedblright ('')
    (0x12, '\u{201E}'), // quotedblbase  (,,)
    (0x13, '\u{00AB}'), // guillemotleft (<<)
    (0x14, '\u{00BB}'), // guillemotright (>>)
    (0x15, '\u{2013}'), // endash  (--)
    (0x16, '\u{2014}'), // emdash  (---)
    (0x19, '\u{0131}'), // dotlessi
    (0x1A, '\u{0237}'), // dotlessj
    (0x1B, '\u{FB00}'), // ff
    (0x1C, '\u{FB01}'), // fi
    (0x1D, '\u{FB02}'), // fl
    (0x1E, '\u{FB03}'), // ffi
    (0x1F, '\u{FB04}'), // ffl
    (0x27, '\u{2019}'), // quoteright (')
    (0x60, '\u{2018}'), // quoteleft  (`)
    (0x7F, '-'),        // hyphenchar (the hyphen TeX inserts; same glyph as 0x2D)
    (0x80, 'Ă'),
    (0x81, 'Ą'),
    (0x82, 'Ć'),
    (0x83, 'Č'),
    (0x84, 'Ď'),
    (0x85, 'Ě'),
    (0x86, 'Ę'),
    (0x87, 'Ğ'),
    (0x88, 'Ĺ'),
    (0x89, 'Ľ'),
    (0x8A, 'Ł'),
    (0x8B, 'Ń'),
    (0x8C, 'Ň'),
    (0x8D, 'Ŋ'),
    (0x8E, 'Ő'),
    (0x8F, 'Ŕ'),
    (0x90, 'Ř'),
    (0x91, 'Ś'),
    (0x92, 'Š'),
    (0x93, 'Ş'),
    (0x94, 'Ť'),
    (0x95, 'Ţ'),
    (0x96, 'Ű'),
    (0x97, 'Ů'),
    (0x98, 'Ÿ'),
    (0x99, 'Ź'),
    (0x9A, 'Ž'),
    (0x9B, 'Ż'),
    (0x9C, 'Ĳ'),
    (0x9D, 'İ'),
    (0x9E, 'đ'),
    (0x9F, '§'),
    (0xA0, 'ă'),
    (0xA1, 'ą'),
    (0xA2, 'ć'),
    (0xA3, 'č'),
    (0xA4, 'ď'),
    (0xA5, 'ě'),
    (0xA6, 'ę'),
    (0xA7, 'ğ'),
    (0xA8, 'ĺ'),
    (0xA9, 'ľ'),
    (0xAA, 'ł'),
    (0xAB, 'ń'),
    (0xAC, 'ň'),
    (0xAD, 'ŋ'),
    (0xAE, 'ő'),
    (0xAF, 'ŕ'),
    (0xB0, 'ř'),
    (0xB1, 'ś'),
    (0xB2, 'š'),
    (0xB3, 'ş'),
    (0xB4, 'ť'),
    (0xB5, 'ţ'),
    (0xB6, 'ű'),
    (0xB7, 'ů'),
    (0xB8, 'ÿ'),
    (0xB9, 'ź'),
    (0xBA, 'ž'),
    (0xBB, 'ż'),
    (0xBC, 'ĳ'),
    (0xBD, '¡'),
    (0xBE, '¿'),
    (0xBF, '£'),
    (0xD7, 'Œ'),
    (0xF7, 'œ'),
    (0xFF, 'ß'),
];

impl EncodingCode {
    /// T1 slot of the endash produced by `--`.
    pub const T1_ENDASH: EncodingCode = EncodingCode(0x15);
    /// T1 slot of the emdash produced by `---`.
    pub const T1_EMDASH: EncodingCode = EncodingCode(0x16);
    pub const T1_QUOTELEFT: EncodingCode = EncodingCode(0x60);
    pub const T1_QUOTERIGHT: EncodingCode = EncodingCode(0x27);
    pub const T1_QUOTEDBLLEFT: EncodingCode = EncodingCode(0x10);
    pub const T1_QUOTEDBLRIGHT: EncodingCode = EncodingCode(0x11);

    /// The Unicode character an encoding slot denotes, if this pipeline has
    /// declared it. `None` means "unknown slot", never "code point 0xNN".
    pub fn to_char(self, encoding: Encoding) -> Option<char> {
        match encoding {
            Encoding::T1 => {
                if let Some((_, c)) = T1_TO_UNICODE.iter().find(|(code, _)| *code == self.0) {
                    return Some(*c);
                }
                let c = self.0;
                // Printable ASCII occupies its own code points in T1 except
                // the two quote slots declared above.
                if (0x20..0x7F).contains(&c) && c != 0x27 && c != 0x60 {
                    return Some(char::from(c));
                }
                // 0xC0–0xFE follow Latin-1 except Œ/œ (0xD7/0xF7) and "SS" (0xDF).
                if (0xC0..=0xFE).contains(&c) && c != 0xD7 && c != 0xDF && c != 0xF7 {
                    return char::from_u32(u32::from(c));
                }
                None
            }
        }
    }

    /// The slot a character occupies in `encoding`, if any. `None` means
    /// the encoding has no such character (the caller shapes that text by
    /// the font's own `cmap` instead), never a guess.
    pub fn for_char(ch: char, encoding: Encoding) -> Option<EncodingCode> {
        match encoding {
            Encoding::T1 => {
                if let Some((code, _)) = T1_TO_UNICODE.iter().find(|(code, c)| *c == ch && *code != 0x7F) {
                    return Some(EncodingCode(*code));
                }
                let u = u32::from(ch);
                if (0x20..0x7F).contains(&u) && u != 0x27 && u != 0x60 {
                    return Some(EncodingCode(u as u8));
                }
                if (0xC0..=0xFE).contains(&u) && u != 0xD7 && u != 0xDF && u != 0xF7 {
                    return Some(EncodingCode(u as u8));
                }
                // ASCII apostrophe and grave are typed as the curly quotes.
                match ch {
                    '\'' => Some(EncodingCode(0x27)),
                    '`' => Some(EncodingCode(0x60)),
                    _ => None,
                }
            }
        }
    }

    /// The TS1 (text companion) slot of a character that T1 has no slot for
    /// and that the kernel sets from the companion font instead: `©` is
    /// `\textcopyright`, declared `\DeclareTextSymbolDefault{..}{TS1}`
    /// (latex.ltx 14425, slot 169 of `ts1enc.def`), likewise `®`, `™`, `°`,
    /// `×`, `€`… The tables are the `*.dfu`/`ts1enc.def`
    /// declarations `flashtex-tex-text-encoding` carries. `None` for a T1
    /// character (never consulted for one), for ASCII, for a character
    /// nobody declares, and for one whose T1 meaning is not a TS1 symbol
    /// (an accent, a macro such as `\textellipsis`).
    pub fn ts1_symbol(ch: char) -> Option<EncodingCode> {
        use flashtex_tex_text_encoding::encoding::{self, Declared, Default, Encoding as E, Resolution};
        if ch.is_ascii() || EncodingCode::for_char(ch, Encoding::T1).is_some() {
            return None;
        }
        let (expansion, _) = flashtex_tex_text_encoding::unicode::lookup_declared(ch)?;
        let (cmd, arg) = crate::inputenc::command_of(expansion)?;
        if !arg.is_empty() {
            return None;
        }
        let from_companion = match encoding::resolve(E::T1, &cmd) {
            Resolution::Default(Default::Symbol(E::TS1)) => true,
            // `\CheckEncodingSubset\UseTextSymbol{TS1}<fake>{n}\cmd` and
            // `\tc@check@symbol{n}\cmd` (latex.ltx 10430): the TS1 glyph is
            // used when `n` exceeds the family's `\DeclareEncodingSubset`
            // (`cmr` 0, `lmr` 1); `\texteuro` is `{8}`, `\textcelsius` `{9}`.
            Resolution::Default(Default::Command(body)) => {
                (body.starts_with("\\CheckEncodingSubset\\UseTextSymbol{TS1}") || body.starts_with("\\tc@check@symbol{"))
                    && body
                        .rfind('{')
                        .and_then(|at| body[at + 1..].split('}').next())
                        .and_then(|n| n.parse::<u8>().ok())
                        .is_some_and(|n| n >= 2)
            }
            _ => false,
        };
        if !from_companion {
            return None;
        }
        match encoding::declared(E::TS1, &cmd)? {
            Declared::Symbol(slot) => Some(EncodingCode(slot)),
            _ => None,
        }
    }
}

/// Resolves a Unicode character to an original glyph id of `face` through
/// its character map. `None` is a missing glyph, to be reported; it is never
/// silently `.notdef`.
pub fn glyph_for_char(face: &dyn Face, ch: char) -> Option<GlyphId> {
    face.glyph_id(ch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1_slots_are_declared_not_cast() {
        assert_eq!(EncodingCode::T1_ENDASH.to_char(Encoding::T1), Some('\u{2013}'));
        assert_eq!(EncodingCode(0x41).to_char(Encoding::T1), Some('A'));
        // 0x15 is NOT U+0015 and an undeclared slot is unknown.
        assert_ne!(EncodingCode(0x15).to_char(Encoding::T1), Some('\u{15}'));
        assert_eq!(EncodingCode(0x00).to_char(Encoding::T1), None);
        // Cork: `` -> 0x10, '' -> 0x11, ` -> 0x60, ' -> 0x27, é -> 0xE9, Œ -> 0xD7.
        assert_eq!(EncodingCode::for_char('\u{201C}', Encoding::T1), Some(EncodingCode(0x10)));
        assert_eq!(EncodingCode::for_char('\u{2019}', Encoding::T1), Some(EncodingCode(0x27)));
        assert_eq!(EncodingCode::for_char('\u{2018}', Encoding::T1), Some(EncodingCode(0x60)));
        assert_eq!(EncodingCode::for_char('é', Encoding::T1), Some(EncodingCode(0xE9)));
        assert_eq!(EncodingCode::for_char('Œ', Encoding::T1), Some(EncodingCode(0xD7)));
        assert_eq!(EncodingCode::for_char('×', Encoding::T1), None);
        assert_eq!(EncodingCode::for_char('ǅ', Encoding::T1), None);
        // `ts1enc.def` slots of the kernel's TS1-default symbols.
        assert_eq!(EncodingCode::ts1_symbol('©'), Some(EncodingCode(169)));
        assert_eq!(EncodingCode::ts1_symbol('®'), Some(EncodingCode(174)));
        assert_eq!(EncodingCode::ts1_symbol('°'), Some(EncodingCode(176)));
        assert_eq!(EncodingCode::ts1_symbol('™'), Some(EncodingCode(151)));
        assert_eq!(EncodingCode::ts1_symbol('€'), Some(EncodingCode(191)));
        assert_eq!(EncodingCode::ts1_symbol('×'), Some(EncodingCode(214)));
        // A T1 character, ASCII, a kernel macro (`\textellipsis`) and an
        // undeclared character are not companion symbols.
        assert_eq!(EncodingCode::ts1_symbol('é'), None);
        assert_eq!(EncodingCode::ts1_symbol('£'), None);
        assert_eq!(EncodingCode::ts1_symbol('a'), None);
        assert_eq!(EncodingCode::ts1_symbol('…'), None);
        assert_eq!(EncodingCode::ts1_symbol('ǅ'), None);
        for code in 0x20u8..=0xFF {
            if let Some(c) = EncodingCode(code).to_char(Encoding::T1) {
                assert_eq!(EncodingCode::for_char(c, Encoding::T1), Some(EncodingCode(if code == 0x7F { 0x2D } else { code })), "slot {code:#x}");
            }
        }
    }
}
