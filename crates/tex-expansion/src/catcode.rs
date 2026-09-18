//! TeX category codes (TeXbook ch. 7) and the per-scope catcode table.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CatCode {
    Escape = 0,
    BeginGroup = 1,
    EndGroup = 2,
    MathShift = 3,
    AlignTab = 4,
    EndLine = 5,
    Param = 6,
    Superscript = 7,
    Subscript = 8,
    Ignored = 9,
    Space = 10,
    Letter = 11,
    Other = 12,
    Active = 13,
    Comment = 14,
    Invalid = 15,
}

impl CatCode {
    pub fn from_u8(n: u8) -> Option<CatCode> {
        use CatCode::*;
        Some(match n {
            0 => Escape,
            1 => BeginGroup,
            2 => EndGroup,
            3 => MathShift,
            4 => AlignTab,
            5 => EndLine,
            6 => Param,
            7 => Superscript,
            8 => Subscript,
            9 => Ignored,
            10 => Space,
            11 => Letter,
            12 => Other,
            13 => Active,
            14 => Comment,
            15 => Invalid,
            _ => return None,
        })
    }
}

/// Category codes are a groupable ("local") assignment, plus support
/// `\global\catcode`. We keep a flat table of 256 entries (plain TeX/LaTeX
/// only assign catcodes to ASCII/Latin-1 code points in practice) with a
/// save stack of `(char, old_value)` pairs pushed on group entry, mirroring
/// how the compiler will eventually restore all other local assignments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatCodeTable {
    table: [CatCode; 256],
}

impl CatCodeTable {
    /// The standard *initial* TeX catcode table (TeXbook p. 341), before
    /// plain.tex/latex.ltx run `\catcode`` assignments such as making `@`
    /// a letter under `\makeatletter`.
    pub fn plain_initial() -> Self {
        let mut table = [CatCode::Other; 256];
        table[b'\\' as usize] = CatCode::Escape;
        table[b'%' as usize] = CatCode::Comment;
        table[b' ' as usize] = CatCode::Space;
        // U+0009 TAB is whitespace (catcode 10), exactly like a space,
        // in both text and math mode (TeXbook p. 341; plain.tex sets
        // `\catcode`\^^I=10`). Without this a tab lexes as a printable
        // "other" character and reaches shaping, which has no glyph.
        table[b'\t' as usize] = CatCode::Space;
        // ^^M is the end-of-line character. ^^J (`\n`) is an ordinary
        // "other" character, as in INITEX: physical line breaks are
        // recognised by the lexer itself and stand for \endlinechar.
        table[b'\r' as usize] = CatCode::EndLine;
        table[0] = CatCode::Ignored; // null
        table[127] = CatCode::Invalid; // delete
        for c in b'a'..=b'z' {
            table[c as usize] = CatCode::Letter;
        }
        for c in b'A'..=b'Z' {
            table[c as usize] = CatCode::Letter;
        }
        table[b'{' as usize] = CatCode::BeginGroup;
        table[b'}' as usize] = CatCode::EndGroup;
        table[b'$' as usize] = CatCode::MathShift;
        table[b'&' as usize] = CatCode::AlignTab;
        table[b'#' as usize] = CatCode::Param;
        table[b'^' as usize] = CatCode::Superscript;
        table[b'_' as usize] = CatCode::Subscript;
        table[b'~' as usize] = CatCode::Active;
        CatCodeTable { table }
    }

    /// LaTeX's actual starting table additionally has `@` as Other until
    /// `\makeatletter` is in effect. We model `\makeatletter`/`\makeatother`
    /// as plain catcode assignments on `@`, so no special-casing is needed
    /// beyond starting from `plain_initial`.
    pub fn latex_initial() -> Self {
        Self::plain_initial()
    }

    pub fn get(&self, ch: char) -> CatCode {
        if (ch as u32) < 256 {
            self.table[ch as u32 as usize]
        } else {
            // Non-Latin-1 characters default to "Other" in classic TeX;
            // engines with Unicode input (XeTeX/LuaTeX) differ, out of
            // scope here.
            CatCode::Other
        }
    }

    pub fn set(&mut self, ch: char, cat: CatCode) {
        if (ch as u32) < 256 {
            self.table[ch as u32 as usize] = cat;
        }
    }
}

impl Default for CatCodeTable {
    fn default() -> Self {
        Self::latex_initial()
    }
}
