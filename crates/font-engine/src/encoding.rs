//! TeX 8-bit font encodings (OT1, T1) as an explicit, non-coercible layer.
//!
//! Three identities are kept distinct and never interchangeable:
//!
//! * [`EncodingCode`] — an 8-bit TeX encoding value (what a TFM `char`
//!   metric or a `\char` refers to). Meaningful only with an [`Encoding`].
//! * `char` — a Unicode scalar, the shaper's input.
//! * [`crate::GlyphId`] — an index in one font program's ORIGINAL glyph order.
//!
//! Mapping goes one way through explicit functions: `Encoding::to_unicode`
//! then `Face::glyph_id`; there are no `From`/`Into` impls between the three,
//! so mixing them is a type error:
//!
//! ```compile_fail
//! use flashtex_font_engine::encoding::EncodingCode;
//! use flashtex_font_engine::GlyphId;
//! let code = EncodingCode(0x41);
//! let gid: GlyphId = code; // error: mismatched types
//! ```
//!
//! ```compile_fail
//! use flashtex_font_engine::encoding::EncodingCode;
//! let c: char = EncodingCode(0x41); // error: mismatched types
//! ```
//!
//! TFM parsing itself is out of scope here (it belongs to
//! `crates/font-resources`); this module only supplies the code → Unicode
//! tables that a TFM consumer needs to reach a glyph.

/// An 8-bit TeX encoding value. Not a Unicode scalar, not a glyph id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EncodingCode(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Encoding {
    /// Knuth's 7-bit text encoding (Computer Modern `cmr10` layout).
    OT1,
    /// The Cork encoding (`ec`/Latin Modern `T1` layout).
    T1,
}

/// OT1 positions 0..=31 and 127 (33..=126 follow ASCII with exceptions).
const OT1_LOW: [u32; 33] = [
    0x0393, 0x0394, 0x0398, 0x039B, 0x039E, 0x03A0, 0x03A3, 0x03A5, // Γ Δ Θ Λ Ξ Π Σ Υ
    0x03A6, 0x03A8, 0x03A9, 0xFB00, 0xFB01, 0xFB02, 0xFB03,
    0xFB04, // Φ Ψ Ω ff fi fl ffi ffl
    0x0131, 0x0237, 0x0060, 0x00B4, 0x02C7, 0x02D8, 0x00AF, 0x02DA, // ı ȷ ` ´ ˇ ˘ ¯ ˚
    0x00B8, 0x00DF, 0x00E6, 0x0153, 0x00F8, 0x00C6, 0x0152, 0x00D8, // ¸ ß æ œ ø Æ Œ Ø
    0x0141, // 32: suppress (Polish l-slash marker) — mapped to Ł glyph shape
];

/// T1 (Cork) positions 0..=32 and 127..=255.
const T1_LOW: [u32; 33] = [
    0x0060, 0x00B4, 0x02C6, 0x02DC, 0x00A8, 0x02DD, 0x02DA, 0x02C7, // ` ´ ˆ ˜ ¨ ˝ ˚ ˇ
    0x02D8, 0x00AF, 0x02D9, 0x00B8, 0x02DB, 0x201A, 0x2039,
    0x203A, // ˘ ¯ ˙ ¸ ˛ ‚ ‹ ›
    0x201C, 0x201D, 0x201E, 0x00AB, 0x00BB, 0x2013, 0x2014,
    0x200B, // “ ” „ « » – — cwm
    0x2030, 0x0131, 0x0237, 0xFB00, 0xFB01, 0xFB02, 0xFB03,
    0xFB04, // ‰(perthousandzero) ı ȷ ff fi fl ffi ffl
    0x2423, // 32: visible space
];
const T1_HIGH: [u32; 129] = [
    0x00AD, // 127: hyphen (soft hyphen)
    0x0102, 0x0104, 0x0106, 0x010C, 0x010E, 0x011A, 0x0118, 0x011E, // Ă Ą Ć Č Ď Ě Ę Ğ
    0x0139, 0x013D, 0x0141, 0x0143, 0x0147, 0x014A, 0x0150, 0x0154, // Ĺ Ľ Ł Ń Ň Ŋ Ő Ŕ
    0x0158, 0x015A, 0x0160, 0x015E, 0x0164, 0x0162, 0x0170, 0x016E, // Ř Ś Š Ş Ť Ţ Ű Ů
    0x0178, 0x0179, 0x017D, 0x017B, 0x0132, 0x0130, 0x0111, 0x00A7, // Ÿ Ź Ž Ż Ĳ İ đ §
    0x0103, 0x0105, 0x0107, 0x010D, 0x010F, 0x011B, 0x0119, 0x011F, // ă ą ć č ď ě ę ğ
    0x013A, 0x013E, 0x0142, 0x0144, 0x0148, 0x014B, 0x0151, 0x0155, // ĺ ľ ł ń ň ŋ ő ŕ
    0x0159, 0x015B, 0x0161, 0x015F, 0x0165, 0x0163, 0x0171, 0x016F, // ř ś š ş ť ţ ű ů
    0x00FF, 0x017A, 0x017E, 0x017C, 0x0133, 0x00A1, 0x00BF, 0x00A3, // ÿ ź ž ż ĳ ¡ ¿ £
    0x00C0, 0x00C1, 0x00C2, 0x00C3, 0x00C4, 0x00C5, 0x00C6, 0x00C7, // À Á Â Ã Ä Å Æ Ç
    0x00C8, 0x00C9, 0x00CA, 0x00CB, 0x00CC, 0x00CD, 0x00CE, 0x00CF, // È É Ê Ë Ì Í Î Ï
    0x00D0, 0x00D1, 0x00D2, 0x00D3, 0x00D4, 0x00D5, 0x00D6, 0x0152, // Ð Ñ Ò Ó Ô Õ Ö Œ
    0x00D8, 0x00D9, 0x00DA, 0x00DB, 0x00DC, 0x00DD, 0x00DE,
    0x1E9E, // Ø Ù Ú Û Ü Ý Þ ẞ(SS)
    0x00E0, 0x00E1, 0x00E2, 0x00E3, 0x00E4, 0x00E5, 0x00E6, 0x00E7, // à á â ã ä å æ ç
    0x00E8, 0x00E9, 0x00EA, 0x00EB, 0x00EC, 0x00ED, 0x00EE, 0x00EF, // è é ê ë ì í î ï
    0x00F0, 0x00F1, 0x00F2, 0x00F3, 0x00F4, 0x00F5, 0x00F6, 0x0153, // ð ñ ò ó ô õ ö œ
    0x00F8, 0x00F9, 0x00FA, 0x00FB, 0x00FC, 0x00FD, 0x00FE, 0x00DF, // ø ù ú û ü ý þ ß
];

impl Encoding {
    /// The Unicode scalar an encoding code stands for, or `None` for codes
    /// the encoding leaves undefined (OT1 128..=255).
    pub fn to_unicode(self, code: EncodingCode) -> Option<char> {
        let c = u32::from(code.0);
        let cp = match self {
            Encoding::OT1 => match c {
                0..=32 => OT1_LOW[c as usize],
                34 => 0x201D,  // quotedblright
                39 => 0x2019,  // quoteright
                60 => 0x00A1,  // exclamdown
                62 => 0x00BF,  // questiondown
                92 => 0x201C,  // quotedblleft
                94 => 0x02C6,  // circumflex accent
                95 => 0x02D9,  // dot accent
                96 => 0x2018,  // quoteleft
                123 => 0x2013, // endash
                124 => 0x2014, // emdash
                125 => 0x02DD, // hungarumlaut
                126 => 0x02DC, // tilde accent
                127 => 0x00A8, // dieresis
                33..=126 => c,
                _ => return None,
            },
            Encoding::T1 => match c {
                0..=32 => T1_LOW[c as usize],
                39 => 0x2019, // quoteright
                96 => 0x2018, // quoteleft
                33..=126 => c,
                127..=255 => T1_HIGH[(c - 127) as usize],
                _ => unreachable!(),
            },
        };
        char::from_u32(cp)
    }

    /// Inverse lookup (first code that maps to `ch`), for callers that hold
    /// Unicode and need a TFM position.
    pub fn from_unicode(self, ch: char) -> Option<EncodingCode> {
        (0..=255u8)
            .map(EncodingCode)
            .find(|code| self.to_unicode(*code) == Some(ch))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ot1_and_t1_agree_on_ascii_letters_and_differ_on_quotes() {
        for c in b'A'..=b'Z' {
            assert_eq!(Encoding::OT1.to_unicode(EncodingCode(c)), Some(c as char));
            assert_eq!(Encoding::T1.to_unicode(EncodingCode(c)), Some(c as char));
        }
        assert_eq!(Encoding::OT1.to_unicode(EncodingCode(34)), Some('\u{201D}'));
        assert_eq!(Encoding::T1.to_unicode(EncodingCode(34)), Some('"'));
        assert_eq!(Encoding::OT1.to_unicode(EncodingCode(12)), Some('\u{FB01}'));
        assert_eq!(Encoding::T1.to_unicode(EncodingCode(28)), Some('\u{FB01}'));
        assert_eq!(Encoding::OT1.to_unicode(EncodingCode(200)), None);
        assert_eq!(Encoding::T1.to_unicode(EncodingCode(0xE9)), Some('é'));
        assert_eq!(Encoding::T1.to_unicode(EncodingCode(0xFF)), Some('ß'));
        assert_eq!(Encoding::T1.from_unicode('é'), Some(EncodingCode(0xE9)));
    }
}
