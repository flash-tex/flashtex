//! Math alphabets: the Unicode mathematical alphanumeric symbols the
//! compiler emits for `\mathsf`, `\mathtt`, `\mathit`, `\mathfrak` (and the
//! pipeline's own `\mathbf` text), mapped back to the TeX font LaTeX sets
//! them in and the character's slot there.
//!
//! `fontmath.ltx` declares `\mathbf` OT1/cmr/bx/n, `\mathsf` OT1/cmss/m/n,
//! `\mathit` OT1/cmr/m/it and `\mathtt` OT1/cmtt/m/n (`lmodern.sty`: the
//! same shapes of lmr/lmss/lmtt); `amsfonts.sty` declares `\mathfrak`
//! U/euf/m/n (`ueuf.fd`: `eufm5` below 6 pt, `eufm7` below 8 pt, else
//! `eufm10`). The four text alphabets are text fonts: a run of their
//! characters is kerned and ligatured by the TFM (TeX §752 `make_ord`) with
//! no italic correction between characters, which the math text sink
//! reproduces; fraktur is a math font set character by character.

use crate::nfss::{FamilyKind, FontKey, Series, Shape};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MathAlphabet {
    Bold,
    Sans,
    Italic,
    Mono,
    Fraktur,
}

/// The four text-font alphabets.
pub const TEXT_ALPHABETS: [MathAlphabet; 4] = [MathAlphabet::Bold, MathAlphabet::Sans, MathAlphabet::Italic, MathAlphabet::Mono];

impl MathAlphabet {
    /// Dense index (0..5), for font ids.
    pub fn index(self) -> usize {
        self as usize
    }

    /// The LaTeX command selecting the alphabet.
    pub fn command(self) -> &'static str {
        match self {
            MathAlphabet::Bold => "\\mathbf",
            MathAlphabet::Sans => "\\mathsf",
            MathAlphabet::Italic => "\\mathit",
            MathAlphabet::Mono => "\\mathtt",
            MathAlphabet::Fraktur => "\\mathfrak",
        }
    }

    /// The NFSS shape of a text-font alphabet; `None` for fraktur.
    pub fn text_key(self) -> Option<FontKey> {
        match self {
            MathAlphabet::Bold => Some(FontKey::new(FamilyKind::Rm, Series::Bx, Shape::N)),
            MathAlphabet::Sans => Some(FontKey::new(FamilyKind::Sf, Series::M, Shape::N)),
            MathAlphabet::Italic => Some(FontKey::new(FamilyKind::Rm, Series::M, Shape::It)),
            MathAlphabet::Mono => Some(FontKey::new(FamilyKind::Tt, Series::M, Shape::N)),
            MathAlphabet::Fraktur => None,
        }
    }
}

/// The alphabet and ASCII letter/digit a mathematical alphanumeric symbol
/// stands for, `None` for every other character.
pub fn classify(ch: char) -> Option<(MathAlphabet, char)> {
    use MathAlphabet::*;
    let c = ch as u32;
    let letter = |base: u32| -> Option<char> {
        let k = c - base;
        char::from_u32(if k < 26 { 'A' as u32 + k } else { 'a' as u32 + k - 26 })
    };
    let digit = |base: u32| char::from_u32('0' as u32 + (c - base));
    Some(match c {
        0x1D400..=0x1D433 => (Bold, letter(0x1D400)?),
        0x1D434..=0x1D467 if c != 0x1D455 => (Italic, letter(0x1D434)?),
        0x210E => (Italic, 'h'),
        0x1D5A0..=0x1D5D3 => (Sans, letter(0x1D5A0)?),
        0x1D670..=0x1D6A3 => (Mono, letter(0x1D670)?),
        0x1D504..=0x1D537 if !matches!(c, 0x1D506 | 0x1D50B | 0x1D50C | 0x1D515 | 0x1D51D) => (Fraktur, letter(0x1D504)?),
        0x212D => (Fraktur, 'C'),
        0x210C => (Fraktur, 'H'),
        0x2111 => (Fraktur, 'I'),
        0x211C => (Fraktur, 'R'),
        0x2128 => (Fraktur, 'Z'),
        0x1D7CE..=0x1D7D7 => (Bold, digit(0x1D7CE)?),
        0x1D7E2..=0x1D7EB => (Sans, digit(0x1D7E2)?),
        0x1D7F6..=0x1D7FF => (Mono, digit(0x1D7F6)?),
        _ => return None,
    })
}

/// The mathematical alphanumeric symbol of `ch` (an ASCII letter or digit)
/// in `alphabet`, when Unicode has one.
pub fn alphanumeric(alphabet: MathAlphabet, ch: char) -> Option<char> {
    use MathAlphabet::*;
    let off = |base: u32, first: char| char::from_u32(base + (ch as u32 - first as u32));
    match (alphabet, ch) {
        (Bold, 'A'..='Z') => off(0x1D400, 'A'),
        (Bold, 'a'..='z') => off(0x1D41A, 'a'),
        (Bold, '0'..='9') => off(0x1D7CE, '0'),
        (Sans, 'A'..='Z') => off(0x1D5A0, 'A'),
        (Sans, 'a'..='z') => off(0x1D5BA, 'a'),
        (Sans, '0'..='9') => off(0x1D7E2, '0'),
        (Mono, 'A'..='Z') => off(0x1D670, 'A'),
        (Mono, 'a'..='z') => off(0x1D68A, 'a'),
        (Mono, '0'..='9') => off(0x1D7F6, '0'),
        (Italic, 'h') => Some('\u{210E}'),
        (Italic, 'A'..='Z') => off(0x1D434, 'A'),
        (Italic, 'a'..='z') => off(0x1D44E, 'a'),
        (Fraktur, 'C') => Some('\u{212D}'),
        (Fraktur, 'H') => Some('\u{210C}'),
        (Fraktur, 'I') => Some('\u{2111}'),
        (Fraktur, 'R') => Some('\u{211C}'),
        (Fraktur, 'Z') => Some('\u{2128}'),
        (Fraktur, 'A'..='Z') => off(0x1D504, 'A'),
        (Fraktur, 'a'..='z') => off(0x1D51E, 'a'),
        _ => None,
    }
}

/// The TeX font of a character set in the bold math version
/// (`\mathversion{bold}`, which `\boldsymbol` and bm's `\bm` select).
/// fontmath.ltx 197-236: `letters` is OML/cmm/b/it (`cmmib`), `symbols`
/// OMS/cmsy/b/n (`cmbsy`) and `operators` OT1/cmr/bx/n (`cmbx`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoldMathFont {
    /// Bold math italic: Latin letters, lowercase and italic-capital Greek,
    /// `\partial`.
    Letters,
    /// Bold symbols: `\nabla`.
    Symbols,
    /// Bold roman: the upright Greek capitals (mathchar "7000, family 0).
    Operators,
}

impl BoldMathFont {
    pub const ALL: [BoldMathFont; 3] = [BoldMathFont::Letters, BoldMathFont::Symbols, BoldMathFont::Operators];

    /// Dense index (0..3), for font ids.
    pub fn index(self) -> usize {
        self as usize
    }

    /// The TFM the bold math version loads at `size_pt`: `OMLcmm.fd`
    /// (`b/it`), `OMScmsy.fd` (`b/n`) and `OT1cmr.fd` (`bx/n`) take the
    /// 5-9pt designs at their own size and the 10pt design above, except
    /// that `cmbx12` serves 12pt and up.
    pub fn tfm(self, size_pt: f64) -> String {
        let stem = match self {
            BoldMathFont::Letters => "cmmib",
            BoldMathFont::Symbols => "cmbsy",
            BoldMathFont::Operators => "cmbx",
        };
        let design = match self {
            BoldMathFont::Operators if size_pt >= 12.0 => 12,
            _ if size_pt < 10.0 => (size_pt.floor() as i64).clamp(5, 9),
            _ => 10,
        };
        format!("{stem}{design}")
    }
}

/// The bold math font and the plain math character (the one the regular
/// math version sets, which carries the TeX slot) of a Unicode bold
/// mathematical symbol the compiler emits for `\boldsymbol`/`\bm`: bold
/// italic Latin (U+1D468) and Greek (U+1D71C, U+1D736 and the variant forms
/// after them), the bold capital Greek of the upright capitals (U+1D6A8)
/// the bold nabla (U+1D6C1) and the bold digits (U+1D7CE, cmbx like
/// `\mathbf`'s, which boxes them from its text font when the document uses
/// `\mathbf` at all). `None` for every other character; upright bold Latin
/// is `\mathbf`'s alone ([`classify`]).
pub fn bold_math(ch: char) -> Option<(BoldMathFont, char)> {
    use BoldMathFont::*;
    let c = ch as u32;
    let base = |from: u32, to: u32| char::from_u32(to + (c - from));
    let (font, plain) = match c {
        0x1D468..=0x1D481 => (Letters, base(0x1D468, 'A' as u32)?),
        0x1D482..=0x1D49B => (Letters, base(0x1D482, 'a' as u32)?),
        // Italic capital Greek (`\varGamma`, U+1D6E2..).
        0x1D71C..=0x1D734 => (Letters, base(0x1D71C, 0x1D6E2)?),
        0x1D736..=0x1D74E => (Letters, base(0x1D736, 0x03B1)?),
        0x1D74F => (Letters, '\u{2202}'),
        0x1D750 => (Letters, '\u{03F5}'),
        0x1D751 => (Letters, '\u{03D1}'),
        0x1D753 => (Letters, '\u{03D5}'),
        0x1D754 => (Letters, '\u{03F1}'),
        0x1D755 => (Letters, '\u{03D6}'),
        0x1D6A8..=0x1D6C0 if c != 0x1D6B9 => (Operators, base(0x1D6A8, 0x0391)?),
        0x1D6C1 => (Symbols, '\u{2207}'),
        0x1D7CE..=0x1D7D7 => (Operators, base(0x1D7CE, '0' as u32)?),
        _ => return None,
    };
    Some((font, plain))
}

/// The `eufm` design `ueuf.fd` loads at `size_pt`.
pub fn fraktur_tfm(size_pt: f64) -> &'static str {
    if size_pt < 6.0 {
        "eufm5"
    } else if size_pt < 8.0 {
        "eufm7"
    } else {
        "eufm10"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alphanumerics_map_back_to_their_letters() {
        assert_eq!(classify('\u{1D5A0}'), Some((MathAlphabet::Sans, 'A')));
        assert_eq!(classify('\u{1D5BB}'), Some((MathAlphabet::Sans, 'b')));
        assert_eq!(classify('\u{1D7E3}'), Some((MathAlphabet::Sans, '1')));
        assert_eq!(classify('\u{1D683}'), Some((MathAlphabet::Mono, 'T')));
        assert_eq!(classify('\u{1D451}'), Some((MathAlphabet::Italic, 'd')));
        assert_eq!(classify('\u{210E}'), Some((MathAlphabet::Italic, 'h')));
        assert_eq!(classify('\u{1D524}'), Some((MathAlphabet::Fraktur, 'g')));
        assert_eq!(classify('\u{211C}'), Some((MathAlphabet::Fraktur, 'R')));
        assert_eq!(classify('\u{1D41A}'), Some((MathAlphabet::Bold, 'a')));
        assert_eq!(classify('x'), None);
        // Double-struck and script capitals stay with \mathbb/\mathcal.
        assert_eq!(classify('\u{211D}'), None);
        assert_eq!(classify('\u{212C}'), None);
        assert_eq!((fraktur_tfm(10.95), fraktur_tfm(8.0), fraktur_tfm(6.0), fraktur_tfm(5.0)), ("eufm10", "eufm10", "eufm7", "eufm5"));
        // The bold math version's symbols (`\bm`, `\boldsymbol`) map back to
        // the regular math character and the bold TeX font.
        assert_eq!(bold_math('\u{1D499}'), Some((BoldMathFont::Letters, 'x')));
        assert_eq!(bold_math('\u{1D468}'), Some((BoldMathFont::Letters, 'A')));
        assert_eq!(bold_math('\u{1D736}'), Some((BoldMathFont::Letters, '\u{03B1}')));
        assert_eq!(bold_math('\u{1D71E}'), Some((BoldMathFont::Letters, '\u{1D6E4}')));
        assert_eq!(bold_math('\u{1D74F}'), Some((BoldMathFont::Letters, '\u{2202}')));
        assert_eq!(bold_math('\u{1D6AA}'), Some((BoldMathFont::Operators, '\u{0393}')));
        assert_eq!(bold_math('\u{1D6C1}'), Some((BoldMathFont::Symbols, '\u{2207}')));
        assert_eq!(bold_math('\u{1D7D0}'), Some((BoldMathFont::Operators, '2')));
        assert_eq!(bold_math('\u{1D41A}'), None, "upright bold Latin is \\mathbf's");
        assert_eq!(bold_math('x'), None);
        assert_eq!(
            (BoldMathFont::Letters.tfm(10.95), BoldMathFont::Letters.tfm(8.0), BoldMathFont::Symbols.tfm(6.0), BoldMathFont::Operators.tfm(5.0), BoldMathFont::Letters.tfm(12.0), BoldMathFont::Operators.tfm(12.0)),
            ("cmmib10".into(), "cmmib8".into(), "cmbsy6".into(), "cmbx5".into(), "cmmib10".into(), "cmbx12".into())
        );
        // `alphanumeric` inverts `classify` for every letter and digit.
        for al in [MathAlphabet::Bold, MathAlphabet::Sans, MathAlphabet::Italic, MathAlphabet::Mono, MathAlphabet::Fraktur] {
            for ch in ('A'..='Z').chain('a'..='z').chain('0'..='9') {
                if let Some(c) = alphanumeric(al, ch) {
                    assert_eq!(classify(c), Some((al, ch)), "{al:?} {ch}");
                }
            }
        }
    }
}
