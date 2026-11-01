//! A Times-based approximation so the engine can run before a real math font
//! (FT-018) is available.
//!
//! Honest scope: glyph advance widths are the Adobe core-14 `Times-Roman`,
//! `Times-Italic` and `Symbol` AFM widths for the characters listed (1/1000 em),
//! transcribed by hand; heights and depths are category estimates
//! (x-height 0.45 em, cap/ascender 0.68 em, descender 0.22 em). The TeX math
//! parameters (σ/ξ) have no Times counterpart, so the Computer Modern ratios
//! from [`crate::cm`] are applied to the requested sizes. There is no size
//! chain for delimiters, big operators or the radical sign: the base glyph is
//! always used and the engine reports `DelimiterTooSmall`/`RadicalTooSmall`
//! when something taller was wanted. Glyph ids are Unicode scalar values
//! because no glyph program is known.

use crate::cm::CmMathMetrics;
use crate::metrics::{FontId, Glyph, MathFontMetrics, MathParams, SizeClass};

pub const TIMES_ROMAN: FontId = FontId(100);
pub const TIMES_ITALIC: FontId = FontId(101);
pub const SYMBOL: FontId = FontId(102);

#[derive(Debug, Clone)]
pub struct TimesApproxMetrics {
    /// Font sizes for text, script, scriptscript.
    pub sizes: [f64; 3],
}

impl TimesApproxMetrics {
    pub fn new(base: f64) -> TimesApproxMetrics {
        TimesApproxMetrics {
            sizes: [base, base * 0.7, base * 0.5],
        }
    }

    fn size_pt(&self, size: SizeClass) -> f64 {
        match size {
            SizeClass::Text => self.sizes[0],
            SizeClass::Script => self.sizes[1],
            SizeClass::ScriptScript => self.sizes[2],
        }
    }
}

fn roman_width(ch: char) -> Option<f64> {
    Some(match ch {
        '0'..='9' => 500.0,
        '(' | ')' | '[' | ']' => 333.0,
        ',' | '.' => 250.0,
        ';' | ':' => 278.0,
        '!' => 333.0,
        '?' => 444.0,
        '/' => 278.0,
        '+' | '=' | '<' | '>' => 564.0,
        '{' | '}' => 480.0,
        '|' => 200.0,
        '^' => 469.0,
        '~' => 541.0,
        '\u{00AF}' => 333.0,
        '\u{00A8}' | '\u{02D9}' | '\u{00B4}' | '`' | '\u{02D8}' | '\u{02C7}' => 333.0,
        _ => return None,
    })
}

fn italic_width(ch: char) -> Option<f64> {
    Some(match ch {
        'a' => 500.0,
        'b' => 500.0,
        'c' => 444.0,
        'd' => 500.0,
        'e' => 444.0,
        'f' => 278.0,
        'g' => 500.0,
        'h' => 500.0,
        'i' => 278.0,
        'j' => 278.0,
        'k' => 444.0,
        'l' => 278.0,
        'm' => 722.0,
        'n' => 500.0,
        'o' => 500.0,
        'p' => 500.0,
        'q' => 500.0,
        'r' => 389.0,
        's' => 389.0,
        't' => 278.0,
        'u' => 500.0,
        'v' => 444.0,
        'w' => 667.0,
        'x' => 444.0,
        'y' => 444.0,
        'z' => 389.0,
        'A' => 611.0,
        'B' => 611.0,
        'C' => 667.0,
        'D' => 722.0,
        'E' => 611.0,
        'F' => 611.0,
        'G' => 722.0,
        'H' => 722.0,
        'I' => 333.0,
        'J' => 444.0,
        'K' => 667.0,
        'L' => 556.0,
        'M' => 833.0,
        'N' => 667.0,
        'O' => 722.0,
        'P' => 611.0,
        'Q' => 722.0,
        'R' => 611.0,
        'S' => 500.0,
        'T' => 556.0,
        'U' => 722.0,
        'V' => 611.0,
        'W' => 833.0,
        'X' => 611.0,
        'Y' => 556.0,
        'Z' => 556.0,
        _ => return None,
    })
}

fn symbol_width(ch: char) -> Option<f64> {
    Some(match ch {
        '\u{2212}' | '-' | '\u{00B1}' | '\u{00D7}' | '\u{00F7}' | '\u{2264}' | '\u{2265}'
        | '\u{2260}' | '\u{2248}' | '\u{2261}' => 549.0,
        '\u{22C5}' | '\u{00B7}' => 250.0,
        '\u{221E}' => 713.0,
        '\u{2211}' | '\u{220F}' => 713.0,
        '\u{222B}' => 274.0,
        '\u{221A}' => 549.0,
        '\u{2192}' | '\u{2190}' | '\u{21D2}' => 987.0,
        '\u{2208}' => 713.0,
        '\u{03B1}'..='\u{03C9}' => 550.0,
        _ => return None,
    })
}

fn is_ascender(ch: char) -> bool {
    matches!(ch, 'b' | 'd' | 'f' | 'h' | 'i' | 'k' | 'l' | 't' | 'A'..='Z' | '0'..='9' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '/' | '!' | '?' | '\u{221E}' | '\u{221A}')
}

fn is_descender(ch: char) -> bool {
    matches!(
        ch,
        'g' | 'j'
            | 'p'
            | 'q'
            | 'y'
            | 'f'
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '|'
            | '/'
            | ','
            | ';'
            | '\u{222B}'
    )
}

impl MathFontMetrics for TimesApproxMetrics {
    fn params(&self, size: SizeClass) -> MathParams {
        // Computer Modern ratios at the requested size (see module docs).
        let cm = CmMathMetrics {
            sizes: self.sizes,
            extension: crate::cm::ExtensionSizing::Scaled,
            ..CmMathMetrics::latex_10pt()
        };
        cm.params(size)
    }

    fn font_name(&self, font: FontId) -> String {
        match font {
            TIMES_ROMAN => "Times-Roman".to_string(),
            TIMES_ITALIC => "Times-Italic".to_string(),
            SYMBOL => "Symbol".to_string(),
            _ => "unknown".to_string(),
        }
    }

    fn glyph(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        let at = self.size_pt(size);
        let (font_id, w, italic) = if let Some(w) = italic_width(ch) {
            (TIMES_ITALIC, w, 30.0)
        } else if let Some(w) = roman_width(ch) {
            (TIMES_ROMAN, w, 0.0)
        } else {
            let w = symbol_width(ch)?;
            (SYMBOL, w, 0.0)
        };
        let em = at / 1000.0;
        let height = match ch {
            '+' | '\u{2212}' | '-' | '\u{00B1}' | '\u{00D7}' => 580.0,
            '=' | '<' | '>' | '\u{2264}' | '\u{2265}' | '\u{2248}' => 500.0,
            ',' | '.' => 100.0,
            '^' | '~' | '\u{00AF}' | '\u{00A8}' | '\u{02D9}' | '\u{00B4}' | '`' | '\u{02D8}'
            | '\u{02C7}' => 680.0,
            '\u{2211}' | '\u{220F}' | '\u{222B}' => 0.0,
            _ if is_ascender(ch) => 680.0,
            _ => 450.0,
        };
        let depth = match ch {
            '\u{2211}' | '\u{220F}' => 1000.0,
            '\u{222B}' => 1100.0,
            '=' => -130.0,
            _ if is_descender(ch) => 220.0,
            _ => 0.0,
        };
        Some(Glyph {
            font_id,
            gid: ch as u16,
            ch,
            size: at,
            width: w * em,
            height: height * em,
            depth: depth * em,
            italic: italic * em,
            skew: 0.0,
        })
    }

    fn large_operator(&self, _ch: char, _size: SizeClass) -> Option<Glyph> {
        None
    }

    fn delimiter_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        self.glyph(ch, size).into_iter().collect()
    }

    fn radical_sizes(&self, size: SizeClass) -> Vec<Glyph> {
        self.glyph('\u{221A}', size).into_iter().collect()
    }

    fn accent_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        self.glyph(ch, size).into_iter().collect()
    }
}
