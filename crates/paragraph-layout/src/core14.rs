//! Table-driven [`FontMetricsSource`] adapter for the Adobe Core 14 Times faces.
//!
//! Provenance (all values are public Adobe AFM data, unmodified):
//!
//! * Advance widths (`*_ASCII`, `*_LATIN1`): copied verbatim from
//!   `crates/compiler/src/metrics.rs` at `de1020cd0be7cede11be2691e00e7f5b15cb2224`
//!   (branch `agent/claude/compiler-foundation`), which transcribed the Adobe
//!   Core 14 AFM character widths in 1/1000 em. That file also covers Helvetica
//!   and Courier; only the three Times faces are needed here.
//! * Pair kerns (`*_KERNS`): the `KPX` entries of the Adobe AFMs
//!   `Times-Roman`, `Times-Bold`, `Times-Italic` (Copyright (c) 1985-1990 Adobe
//!   Systems Incorporated, AFM 2.0), as distributed in TeX Live 2026
//!   `fonts/afm/adobe/times/ptm{r,b,ri}8a.afm`, restricted to pairs whose both
//!   glyphs are printable ASCII (272 pairs per face; `quoteright`/`quoteleft`
//!   are mapped to ASCII `'`/`` ` `` as TeX's encodings do).
//! * Vertical metrics: AFM `Ascender`/`Descender` (Times-Roman 683/-217,
//!   Times-Bold 676/-205, Times-Italic 683/-205); `line_gap` is 0 (AFM has none).
//! * Interword glue: `space` = width of `space` (250). Stretch 150, shrink 60
//!   and `extra_space` 60 are the `\fontdimen3/4/7` values of the psnfss
//!   `ptmr8t`/`ptmb8t`/`ptmri8t` TFMs that pdflatex's `times` package uses
//!   (read from the TeX Live 2026 TFM files; see `docs/comparison.md`). They
//!   are what the oracle comparison is calibrated against and are *not* the
//!   plain-TeX 1/2, 1/3 defaults of the trait.
//! * Punctuation outside ISO-8859-1 (`endash`, `emdash`, curly quotes,
//!   `bullet`, `ellipsis`) uses the same AFMs' widths, keyed by Unicode.
//!   Following TeX's OT1/T1 text encodings, ASCII `'` and `` ` `` are
//!   `quoteright`/`quoteleft` (width 333, with their AFM kern pairs), the one
//!   deliberate deviation from de1020c's table (which has `quotesingle` 180).
//! * Ligatures: `fi` (StandardEncoding code 174, width 556/556/500) and `fl`
//!   (code 175, width 556/556/500) are genuine Core 14 glyphs and are reported
//!   through [`FontMetricsSource::ligature`] as U+FB01/U+FB02. pdflatex's T1
//!   Times additionally has `ff`/`ffi`/`ffl` composites built by fontinst; those
//!   are not Core 14 glyphs and are not modelled.
//!
//! Glyph ids: Core 14 AFM fonts have no glyph indices. This adapter reports the
//! Unicode scalar value as the glyph id (U+FB01/U+FB02 for the ligatures) so the
//! value is stable and reversible; consumers that embed a Core 14 font map it
//! through their chosen encoding. The real font engine supplies true GIDs.

use crate::metrics::{FontId, FontMetricsSource, Ligature};

/// The three Times faces the adapter knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimesFace {
    Roman,
    Bold,
    Italic,
}

/// Width used for a character outside the covered repertoire (matches de1020c).
pub const DEFAULT_ADVANCE_UNITS: u16 = 500;

/// Core 14 Times metrics adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Core14Times {
    pub face: TimesFace,
}

impl Core14Times {
    pub const ROMAN: Core14Times = Core14Times {
        face: TimesFace::Roman,
    };
    pub const BOLD: Core14Times = Core14Times {
        face: TimesFace::Bold,
    };
    pub const ITALIC: Core14Times = Core14Times {
        face: TimesFace::Italic,
    };

    fn ascii(&self) -> &'static [u16; 95] {
        match self.face {
            TimesFace::Roman => &TIMES_ROMAN_ASCII,
            TimesFace::Bold => &TIMES_BOLD_ASCII,
            TimesFace::Italic => &TIMES_ITALIC_ASCII,
        }
    }

    fn latin1(&self) -> &'static [u16; 96] {
        match self.face {
            TimesFace::Roman => &TIMES_ROMAN_LATIN1,
            TimesFace::Bold => &TIMES_BOLD_LATIN1,
            TimesFace::Italic => &TIMES_ITALIC_LATIN1,
        }
    }

    fn kerns(&self) -> &'static [(char, char, i16)] {
        match self.face {
            TimesFace::Roman => TIMES_ROMAN_KERNS,
            TimesFace::Bold => TIMES_BOLD_KERNS,
            TimesFace::Italic => TIMES_ITALIC_KERNS,
        }
    }

    /// Widths of the Core 14 glyphs outside ISO-8859-1 that LaTeX text commonly
    /// produces (`--`, `---`, quotes, `\dots`, `\textbullet`, fi/fl), from the
    /// same Adobe AFMs. Roman / Bold / Italic.
    fn punctuation_units(&self, ch: char) -> Option<u16> {
        let (r, b, i) = match ch {
            '\u{2013}' => (500, 500, 500),              // endash
            '\u{2014}' => (1000, 1000, 889),            // emdash
            '\u{2018}' | '\u{2019}' => (333, 333, 333), // quoteleft / quoteright
            '\u{201C}' => (444, 500, 556),              // quotedblleft
            '\u{201D}' => (444, 500, 556),              // quotedblright
            '\u{2022}' => (350, 350, 350),              // bullet
            '\u{2026}' => (1000, 1000, 889),            // ellipsis
            '\u{FB01}' | '\u{FB02}' => (556, 556, 500), // fi / fl
            _ => return None,
        };
        Some(match self.face {
            TimesFace::Roman => r,
            TimesFace::Bold => b,
            TimesFace::Italic => i,
        })
    }

    /// Width of `ch` in 1/1000 em.
    pub fn width_units(&self, ch: char) -> u16 {
        let code = tex_quote(ch) as u32;
        if code == 0x27 {
            // TeX's OT1/T1 encodings map ASCII `'` to `quoteright` (333), not
            // `quotesingle` (180, which de1020c's table holds).
            return 333;
        }
        if (0x20..=0x7e).contains(&code) {
            self.ascii()[(code - 0x20) as usize]
        } else if (0xa0..=0xff).contains(&code) {
            self.latin1()[(code - 0xa0) as usize]
        } else if let Some(w) = self.punctuation_units(ch) {
            w
        } else {
            DEFAULT_ADVANCE_UNITS
        }
    }
}

/// TeX text conventions: ASCII `'` is `quoteright` and `` ` `` is `quoteleft`,
/// and the Unicode curly quotes are the same glyphs.
fn tex_quote(ch: char) -> char {
    match ch {
        '\u{2019}' => '\'',
        '\u{2018}' => '`',
        c => c,
    }
}

impl FontMetricsSource for Core14Times {
    fn font_id(&self) -> FontId {
        match self.face {
            TimesFace::Roman => FontId::from_label("core14:Times-Roman:afm"),
            TimesFace::Bold => FontId::from_label("core14:Times-Bold:afm"),
            TimesFace::Italic => FontId::from_label("core14:Times-Italic:afm"),
        }
    }

    fn units_per_em(&self) -> f64 {
        1000.0
    }

    fn advance(&self, ch: char) -> f64 {
        f64::from(self.width_units(ch))
    }

    fn kern(&self, left: char, right: char) -> f64 {
        // The table's `'`/`` ` `` rows are the AFM `quoteright`/`quoteleft`
        // pairs (see `tex_quote`), so U+2019/U+2018 share them.
        let (left, right) = (tex_quote(left), tex_quote(right));
        // 272 entries; a linear scan is cheap and keeps the table literal.
        self.kerns()
            .iter()
            .find(|(l, r, _)| *l == left && *r == right)
            .map(|(_, _, v)| f64::from(*v))
            .unwrap_or(0.0)
    }

    fn glyph_id(&self, ch: char) -> u32 {
        ch as u32
    }

    fn ascender(&self) -> f64 {
        match self.face {
            TimesFace::Roman | TimesFace::Italic => 683.0,
            TimesFace::Bold => 676.0,
        }
    }

    fn descender(&self) -> f64 {
        match self.face {
            TimesFace::Roman => -217.0,
            TimesFace::Bold | TimesFace::Italic => -205.0,
        }
    }

    fn line_gap(&self) -> f64 {
        0.0
    }

    fn space(&self) -> f64 {
        250.0
    }

    fn space_stretch(&self) -> f64 {
        150.0
    }

    fn space_shrink(&self) -> f64 {
        60.0
    }

    fn extra_space(&self) -> f64 {
        60.0
    }

    fn ligature(&self, left: char, right: char) -> Option<Ligature> {
        match (left, right) {
            ('f', 'i') => Some(Ligature { result: '\u{FB01}' }),
            ('f', 'l') => Some(Ligature { result: '\u{FB02}' }),
            _ => None,
        }
    }
}

const TIMES_ROMAN_ASCII: [u16; 95] = [
    250, 333, 408, 500, 500, 833, 778, 180, 333, 333, 500, 564, 250, 333, 250, 278, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 278, 278, 564, 564, 564, 444, 921, 722, 667, 667, 722, 611,
    556, 722, 722, 333, 389, 722, 611, 889, 722, 722, 556, 722, 667, 556, 611, 722, 722, 944, 722,
    722, 611, 333, 278, 333, 469, 500, 333, 444, 500, 444, 500, 444, 333, 500, 500, 278, 278, 500,
    278, 778, 500, 500, 500, 500, 333, 389, 278, 500, 500, 722, 500, 500, 444, 480, 200, 480, 541,
];

const TIMES_BOLD_ASCII: [u16; 95] = [
    250, 333, 555, 500, 500, 1000, 833, 278, 333, 333, 500, 570, 250, 333, 250, 278, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 333, 333, 570, 570, 570, 500, 930, 722, 667, 722, 722, 667,
    611, 778, 778, 389, 500, 778, 667, 944, 722, 778, 611, 778, 722, 556, 667, 722, 722, 1000, 722,
    722, 667, 333, 278, 333, 581, 500, 333, 500, 556, 444, 556, 444, 333, 500, 556, 278, 333, 556,
    278, 833, 556, 500, 556, 556, 444, 389, 333, 556, 500, 722, 500, 500, 444, 394, 220, 394, 520,
];

const TIMES_ITALIC_ASCII: [u16; 95] = [
    250, 333, 420, 500, 500, 833, 778, 214, 333, 333, 500, 675, 250, 333, 250, 278, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 333, 333, 675, 675, 675, 500, 920, 611, 611, 667, 722, 611,
    611, 722, 722, 333, 444, 667, 556, 833, 667, 722, 611, 722, 611, 500, 556, 722, 611, 833, 611,
    556, 556, 389, 278, 389, 422, 500, 333, 500, 500, 444, 500, 444, 278, 500, 500, 278, 278, 444,
    278, 722, 500, 500, 500, 500, 389, 389, 278, 500, 444, 667, 444, 444, 389, 400, 275, 400, 541,
];

const TIMES_ROMAN_LATIN1: [u16; 96] = [
    250, 333, 500, 500, 500, 500, 200, 500, 333, 760, 276, 500, 564, 333, 760, 500, 400, 564, 300,
    300, 333, 500, 453, 250, 333, 300, 310, 500, 750, 750, 750, 444, 722, 722, 722, 722, 722, 722,
    889, 667, 611, 611, 611, 611, 333, 333, 333, 333, 722, 722, 722, 722, 722, 722, 722, 564, 722,
    722, 722, 722, 722, 722, 556, 500, 444, 444, 444, 444, 444, 444, 667, 444, 444, 444, 444, 444,
    278, 278, 278, 278, 500, 500, 500, 500, 500, 500, 500, 564, 500, 500, 500, 500, 500, 500, 500,
    500,
];

const TIMES_BOLD_LATIN1: [u16; 96] = [
    250, 333, 500, 500, 500, 500, 220, 500, 333, 747, 300, 500, 570, 333, 747, 500, 400, 570, 300,
    300, 333, 556, 540, 250, 333, 300, 330, 500, 750, 750, 750, 500, 722, 722, 722, 722, 722, 722,
    1000, 722, 667, 667, 667, 667, 389, 389, 389, 389, 722, 722, 778, 778, 778, 778, 778, 570, 778,
    722, 722, 722, 722, 722, 611, 556, 500, 500, 500, 500, 500, 500, 722, 444, 444, 444, 444, 444,
    278, 278, 278, 278, 500, 556, 500, 500, 500, 500, 500, 570, 500, 556, 556, 556, 556, 500, 556,
    500,
];

const TIMES_ITALIC_LATIN1: [u16; 96] = [
    250, 389, 500, 500, 500, 500, 275, 500, 333, 760, 276, 500, 675, 333, 760, 500, 400, 675, 300,
    300, 333, 500, 523, 250, 333, 300, 310, 500, 750, 750, 750, 500, 611, 611, 611, 611, 611, 611,
    889, 667, 611, 611, 611, 611, 333, 333, 333, 333, 722, 667, 722, 722, 722, 722, 722, 675, 722,
    722, 722, 722, 722, 556, 611, 500, 500, 500, 500, 500, 500, 500, 667, 444, 444, 444, 444, 444,
    278, 278, 278, 278, 500, 500, 500, 500, 500, 500, 500, 675, 500, 500, 500, 500, 500, 444, 500,
    444,
];
const TIMES_ROMAN_KERNS: &[(char, char, i16)] = &[
    ('A', 'y', -92),
    ('A', 'w', -92),
    ('A', 'v', -74),
    ('A', 'u', 0),
    ('A', '\'', -111),
    ('A', 'p', 0),
    ('A', 'Y', -105),
    ('A', 'W', -90),
    ('A', 'V', -135),
    ('A', 'U', -55),
    ('A', 'T', -111),
    ('A', 'Q', -55),
    ('A', 'O', -55),
    ('A', 'G', -40),
    ('A', 'C', -40),
    ('B', '.', 0),
    ('B', ',', 0),
    ('B', 'U', -10),
    ('B', 'A', -35),
    ('D', '.', 0),
    ('D', ',', 0),
    ('D', 'Y', -55),
    ('D', 'W', -30),
    ('D', 'V', -40),
    ('D', 'A', -40),
    ('F', 'r', 0),
    ('F', '.', -80),
    ('F', 'o', -15),
    ('F', 'i', 0),
    ('F', 'e', 0),
    ('F', ',', -80),
    ('F', 'a', -15),
    ('F', 'A', -74),
    ('G', '.', 0),
    ('G', ',', 0),
    ('J', 'u', 0),
    ('J', '.', 0),
    ('J', 'o', 0),
    ('J', 'e', 0),
    ('J', ',', 0),
    ('J', 'a', 0),
    ('J', 'A', -60),
    ('K', 'y', -25),
    ('K', 'u', -15),
    ('K', 'o', -35),
    ('K', 'e', -25),
    ('K', 'O', -30),
    ('L', 'y', -55),
    ('L', '\'', -92),
    ('L', 'Y', -100),
    ('L', 'W', -74),
    ('L', 'V', -100),
    ('L', 'T', -92),
    ('N', '.', 0),
    ('N', ',', 0),
    ('N', 'A', -35),
    ('O', '.', 0),
    ('O', ',', 0),
    ('O', 'Y', -50),
    ('O', 'X', -40),
    ('O', 'W', -35),
    ('O', 'V', -50),
    ('O', 'T', -40),
    ('O', 'A', -35),
    ('P', '.', -111),
    ('P', 'o', 0),
    ('P', 'e', 0),
    ('P', ',', -111),
    ('P', 'a', -15),
    ('P', 'A', -92),
    ('Q', '.', 0),
    ('Q', ',', 0),
    ('Q', 'U', -10),
    ('R', 'Y', -65),
    ('R', 'W', -55),
    ('R', 'V', -80),
    ('R', 'U', -40),
    ('R', 'T', -60),
    ('R', 'O', -40),
    ('S', '.', 0),
    ('S', ',', 0),
    ('T', 'y', -80),
    ('T', 'w', -80),
    ('T', 'u', -45),
    ('T', ';', -55),
    ('T', 'r', -35),
    ('T', '.', -74),
    ('T', 'o', -80),
    ('T', 'i', -35),
    ('T', '-', -92),
    ('T', 'h', 0),
    ('T', 'e', -70),
    ('T', ',', -74),
    ('T', ':', -50),
    ('T', 'a', -80),
    ('T', 'O', -18),
    ('T', 'A', -93),
    ('U', '.', 0),
    ('U', ',', 0),
    ('U', 'A', -40),
    ('V', 'u', -75),
    ('V', ';', -74),
    ('V', '.', -129),
    ('V', 'o', -129),
    ('V', 'i', -60),
    ('V', '-', -100),
    ('V', 'e', -111),
    ('V', ',', -129),
    ('V', ':', -74),
    ('V', 'a', -111),
    ('V', 'O', -40),
    ('V', 'G', -15),
    ('V', 'A', -135),
    ('W', 'y', -73),
    ('W', 'u', -50),
    ('W', ';', -37),
    ('W', '.', -92),
    ('W', 'o', -80),
    ('W', 'i', -40),
    ('W', '-', -65),
    ('W', 'h', 0),
    ('W', 'e', -80),
    ('W', ',', -92),
    ('W', ':', -37),
    ('W', 'a', -80),
    ('W', 'O', -10),
    ('W', 'A', -120),
    ('Y', 'u', -111),
    ('Y', ';', -92),
    ('Y', '.', -129),
    ('Y', 'o', -110),
    ('Y', 'i', -55),
    ('Y', '-', -111),
    ('Y', 'e', -100),
    ('Y', ',', -129),
    ('Y', ':', -92),
    ('Y', 'a', -100),
    ('Y', 'O', -30),
    ('Y', 'A', -120),
    ('a', 'y', 0),
    ('a', 'w', -15),
    ('a', 'v', -20),
    ('a', 't', 0),
    ('a', 'p', 0),
    ('a', 'g', 0),
    ('a', 'b', 0),
    ('b', 'y', 0),
    ('b', 'v', -15),
    ('b', 'u', -20),
    ('b', '.', -40),
    ('b', 'l', 0),
    ('b', ',', 0),
    ('b', 'b', 0),
    ('c', 'y', -15),
    ('c', '.', 0),
    ('c', 'l', 0),
    ('c', 'k', 0),
    ('c', 'h', 0),
    ('c', ',', 0),
    (':', ' ', 0),
    (',', ' ', 0),
    (',', '\'', -70),
    ('d', 'y', 0),
    ('d', 'w', 0),
    ('d', 'v', 0),
    ('d', '.', 0),
    ('d', 'd', 0),
    ('d', ',', 0),
    ('e', 'y', -15),
    ('e', 'x', -15),
    ('e', 'w', -25),
    ('e', 'v', -25),
    ('e', '.', 0),
    ('e', 'p', 0),
    ('e', 'g', -15),
    ('e', ',', 0),
    ('e', 'b', 0),
    ('f', '\'', 55),
    ('f', '.', 0),
    ('f', 'o', 0),
    ('f', 'l', 0),
    ('f', 'i', -20),
    ('f', 'f', -25),
    ('f', 'e', 0),
    ('f', ',', 0),
    ('f', 'a', -10),
    ('g', 'y', 0),
    ('g', 'r', 0),
    ('g', '.', 0),
    ('g', 'o', 0),
    ('g', 'i', 0),
    ('g', 'g', 0),
    ('g', 'e', 0),
    ('g', ',', 0),
    ('g', 'a', -5),
    ('h', 'y', -5),
    ('i', 'v', -25),
    ('k', 'y', -15),
    ('k', 'o', -10),
    ('k', 'e', -10),
    ('l', 'y', 0),
    ('l', 'w', -10),
    ('m', 'y', 0),
    ('m', 'u', 0),
    ('n', 'y', -15),
    ('n', 'v', -40),
    ('n', 'u', 0),
    ('o', 'y', -10),
    ('o', 'x', 0),
    ('o', 'w', -25),
    ('o', 'v', -15),
    ('o', 'g', 0),
    ('p', 'y', -10),
    ('.', '\'', -70),
    ('`', '`', -74),
    ('`', 'A', -80),
    ('\'', 'v', -50),
    ('\'', 't', -18),
    ('\'', ' ', -74),
    ('\'', 's', -55),
    ('\'', 'r', -50),
    ('\'', '\'', -74),
    ('\'', 'l', -10),
    ('\'', 'd', -50),
    ('r', 'y', 0),
    ('r', 'v', 0),
    ('r', 'u', 0),
    ('r', 't', 0),
    ('r', 's', 0),
    ('r', 'r', 0),
    ('r', 'q', 0),
    ('r', '.', -55),
    ('r', 'p', 0),
    ('r', 'o', 0),
    ('r', 'n', 0),
    ('r', 'm', 0),
    ('r', 'l', 0),
    ('r', 'k', 0),
    ('r', 'i', 0),
    ('r', '-', -20),
    ('r', 'g', -18),
    ('r', 'e', 0),
    ('r', 'd', 0),
    ('r', ',', -40),
    ('r', 'c', 0),
    ('r', 'a', 0),
    ('s', 'w', 0),
    (' ', '`', 0),
    (' ', 'Y', -90),
    (' ', 'W', -30),
    (' ', 'V', -50),
    (' ', 'T', -18),
    (' ', 'A', -55),
    ('v', '.', -65),
    ('v', 'o', -20),
    ('v', 'e', -15),
    ('v', ',', -65),
    ('v', 'a', -25),
    ('w', '.', -65),
    ('w', 'o', -10),
    ('w', 'h', 0),
    ('w', 'e', 0),
    ('w', ',', -65),
    ('w', 'a', -10),
    ('x', 'e', -15),
    ('y', '.', -65),
    ('y', 'o', 0),
    ('y', 'e', 0),
    ('y', ',', -65),
    ('y', 'a', 0),
    ('z', 'o', 0),
    ('z', 'e', 0),
];

const TIMES_BOLD_KERNS: &[(char, char, i16)] = &[
    ('A', 'y', -74),
    ('A', 'w', -90),
    ('A', 'v', -100),
    ('A', 'u', -50),
    ('A', '\'', -74),
    ('A', 'p', -25),
    ('A', 'Y', -100),
    ('A', 'W', -130),
    ('A', 'V', -145),
    ('A', 'U', -50),
    ('A', 'T', -95),
    ('A', 'Q', -45),
    ('A', 'O', -45),
    ('A', 'G', -55),
    ('A', 'C', -55),
    ('B', '.', 0),
    ('B', ',', 0),
    ('B', 'U', -10),
    ('B', 'A', -30),
    ('D', '.', -20),
    ('D', ',', 0),
    ('D', 'Y', -40),
    ('D', 'W', -40),
    ('D', 'V', -40),
    ('D', 'A', -35),
    ('F', 'r', 0),
    ('F', '.', -110),
    ('F', 'o', -25),
    ('F', 'i', 0),
    ('F', 'e', -25),
    ('F', ',', -92),
    ('F', 'a', -25),
    ('F', 'A', -90),
    ('G', '.', 0),
    ('G', ',', 0),
    ('J', 'u', -15),
    ('J', '.', -20),
    ('J', 'o', -15),
    ('J', 'e', -15),
    ('J', ',', 0),
    ('J', 'a', -15),
    ('J', 'A', -30),
    ('K', 'y', -45),
    ('K', 'u', -15),
    ('K', 'o', -25),
    ('K', 'e', -25),
    ('K', 'O', -30),
    ('L', 'y', -55),
    ('L', '\'', -110),
    ('L', 'Y', -92),
    ('L', 'W', -92),
    ('L', 'V', -92),
    ('L', 'T', -92),
    ('N', '.', 0),
    ('N', ',', 0),
    ('N', 'A', -20),
    ('O', '.', 0),
    ('O', ',', 0),
    ('O', 'Y', -50),
    ('O', 'X', -40),
    ('O', 'W', -50),
    ('O', 'V', -50),
    ('O', 'T', -40),
    ('O', 'A', -40),
    ('P', '.', -110),
    ('P', 'o', -20),
    ('P', 'e', -20),
    ('P', ',', -92),
    ('P', 'a', -10),
    ('P', 'A', -74),
    ('Q', '.', -20),
    ('Q', ',', 0),
    ('Q', 'U', -10),
    ('R', 'Y', -35),
    ('R', 'W', -35),
    ('R', 'V', -55),
    ('R', 'U', -30),
    ('R', 'T', -40),
    ('R', 'O', -30),
    ('S', '.', 0),
    ('S', ',', 0),
    ('T', 'y', -74),
    ('T', 'w', -74),
    ('T', 'u', -92),
    ('T', ';', -74),
    ('T', 'r', -74),
    ('T', '.', -90),
    ('T', 'o', -92),
    ('T', 'i', -18),
    ('T', '-', -92),
    ('T', 'h', 0),
    ('T', 'e', -92),
    ('T', ',', -74),
    ('T', ':', -74),
    ('T', 'a', -92),
    ('T', 'O', -18),
    ('T', 'A', -90),
    ('U', '.', -50),
    ('U', ',', -50),
    ('U', 'A', -60),
    ('V', 'u', -92),
    ('V', ';', -92),
    ('V', '.', -145),
    ('V', 'o', -100),
    ('V', 'i', -37),
    ('V', '-', -74),
    ('V', 'e', -100),
    ('V', ',', -129),
    ('V', ':', -92),
    ('V', 'a', -92),
    ('V', 'O', -45),
    ('V', 'G', -30),
    ('V', 'A', -135),
    ('W', 'y', -60),
    ('W', 'u', -50),
    ('W', ';', -55),
    ('W', '.', -92),
    ('W', 'o', -75),
    ('W', 'i', -18),
    ('W', '-', -37),
    ('W', 'h', 0),
    ('W', 'e', -65),
    ('W', ',', -92),
    ('W', ':', -55),
    ('W', 'a', -65),
    ('W', 'O', -10),
    ('W', 'A', -120),
    ('Y', 'u', -92),
    ('Y', ';', -92),
    ('Y', '.', -92),
    ('Y', 'o', -111),
    ('Y', 'i', -37),
    ('Y', '-', -92),
    ('Y', 'e', -111),
    ('Y', ',', -92),
    ('Y', ':', -92),
    ('Y', 'a', -85),
    ('Y', 'O', -35),
    ('Y', 'A', -110),
    ('a', 'y', 0),
    ('a', 'w', 0),
    ('a', 'v', -25),
    ('a', 't', 0),
    ('a', 'p', 0),
    ('a', 'g', 0),
    ('a', 'b', 0),
    ('b', 'y', 0),
    ('b', 'v', -15),
    ('b', 'u', -20),
    ('b', '.', -40),
    ('b', 'l', 0),
    ('b', ',', 0),
    ('b', 'b', -10),
    ('c', 'y', 0),
    ('c', '.', 0),
    ('c', 'l', 0),
    ('c', 'k', 0),
    ('c', 'h', 0),
    ('c', ',', 0),
    (':', ' ', 0),
    (',', ' ', 0),
    (',', '\'', -55),
    ('d', 'y', 0),
    ('d', 'w', -15),
    ('d', 'v', 0),
    ('d', '.', 0),
    ('d', 'd', 0),
    ('d', ',', 0),
    ('e', 'y', 0),
    ('e', 'x', 0),
    ('e', 'w', 0),
    ('e', 'v', -15),
    ('e', '.', 0),
    ('e', 'p', 0),
    ('e', 'g', 0),
    ('e', ',', 0),
    ('e', 'b', 0),
    ('f', '\'', 55),
    ('f', '.', -15),
    ('f', 'o', -25),
    ('f', 'l', 0),
    ('f', 'i', -25),
    ('f', 'f', 0),
    ('f', 'e', 0),
    ('f', ',', -15),
    ('f', 'a', 0),
    ('g', 'y', 0),
    ('g', 'r', 0),
    ('g', '.', -15),
    ('g', 'o', 0),
    ('g', 'i', 0),
    ('g', 'g', 0),
    ('g', 'e', 0),
    ('g', ',', 0),
    ('g', 'a', 0),
    ('h', 'y', -15),
    ('i', 'v', -10),
    ('k', 'y', -15),
    ('k', 'o', -15),
    ('k', 'e', -10),
    ('l', 'y', 0),
    ('l', 'w', 0),
    ('m', 'y', 0),
    ('m', 'u', 0),
    ('n', 'y', 0),
    ('n', 'v', -40),
    ('n', 'u', 0),
    ('o', 'y', 0),
    ('o', 'x', 0),
    ('o', 'w', -10),
    ('o', 'v', -10),
    ('o', 'g', 0),
    ('p', 'y', 0),
    ('.', '\'', -55),
    ('`', '`', -63),
    ('`', 'A', -10),
    ('\'', 'v', -20),
    ('\'', 't', 0),
    ('\'', ' ', -74),
    ('\'', 's', -37),
    ('\'', 'r', -20),
    ('\'', '\'', -63),
    ('\'', 'l', 0),
    ('\'', 'd', -20),
    ('r', 'y', 0),
    ('r', 'v', -10),
    ('r', 'u', 0),
    ('r', 't', 0),
    ('r', 's', 0),
    ('r', 'r', 0),
    ('r', 'q', -18),
    ('r', '.', -100),
    ('r', 'p', -10),
    ('r', 'o', -18),
    ('r', 'n', -15),
    ('r', 'm', 0),
    ('r', 'l', 0),
    ('r', 'k', 0),
    ('r', 'i', 0),
    ('r', '-', -37),
    ('r', 'g', -10),
    ('r', 'e', -18),
    ('r', 'd', 0),
    ('r', ',', -92),
    ('r', 'c', -18),
    ('r', 'a', 0),
    ('s', 'w', 0),
    (' ', '`', 0),
    (' ', 'Y', -55),
    (' ', 'W', -30),
    (' ', 'V', -45),
    (' ', 'T', -30),
    (' ', 'A', -55),
    ('v', '.', -70),
    ('v', 'o', -10),
    ('v', 'e', -10),
    ('v', ',', -55),
    ('v', 'a', -10),
    ('w', '.', -70),
    ('w', 'o', -10),
    ('w', 'h', 0),
    ('w', 'e', 0),
    ('w', ',', -55),
    ('w', 'a', 0),
    ('x', 'e', 0),
    ('y', '.', -70),
    ('y', 'o', -25),
    ('y', 'e', -10),
    ('y', ',', -55),
    ('y', 'a', 0),
    ('z', 'o', 0),
    ('z', 'e', 0),
];

const TIMES_ITALIC_KERNS: &[(char, char, i16)] = &[
    ('A', 'y', -55),
    ('A', 'w', -55),
    ('A', 'v', -55),
    ('A', 'u', -20),
    ('A', '\'', -37),
    ('A', 'p', 0),
    ('A', 'Y', -55),
    ('A', 'W', -95),
    ('A', 'V', -105),
    ('A', 'U', -50),
    ('A', 'T', -37),
    ('A', 'Q', -40),
    ('A', 'O', -40),
    ('A', 'G', -35),
    ('A', 'C', -30),
    ('B', '.', 0),
    ('B', ',', 0),
    ('B', 'U', -10),
    ('B', 'A', -25),
    ('D', '.', 0),
    ('D', ',', 0),
    ('D', 'Y', -40),
    ('D', 'W', -40),
    ('D', 'V', -40),
    ('D', 'A', -35),
    ('F', 'r', -55),
    ('F', '.', -135),
    ('F', 'o', -105),
    ('F', 'i', -45),
    ('F', 'e', -75),
    ('F', ',', -135),
    ('F', 'a', -75),
    ('F', 'A', -115),
    ('G', '.', 0),
    ('G', ',', 0),
    ('J', 'u', -35),
    ('J', '.', -25),
    ('J', 'o', -25),
    ('J', 'e', -25),
    ('J', ',', -25),
    ('J', 'a', -35),
    ('J', 'A', -40),
    ('K', 'y', -40),
    ('K', 'u', -40),
    ('K', 'o', -40),
    ('K', 'e', -35),
    ('K', 'O', -50),
    ('L', 'y', -30),
    ('L', '\'', -37),
    ('L', 'Y', -20),
    ('L', 'W', -55),
    ('L', 'V', -55),
    ('L', 'T', -20),
    ('N', '.', 0),
    ('N', ',', 0),
    ('N', 'A', -27),
    ('O', '.', 0),
    ('O', ',', 0),
    ('O', 'Y', -50),
    ('O', 'X', -40),
    ('O', 'W', -50),
    ('O', 'V', -50),
    ('O', 'T', -40),
    ('O', 'A', -55),
    ('P', '.', -135),
    ('P', 'o', -80),
    ('P', 'e', -80),
    ('P', ',', -135),
    ('P', 'a', -80),
    ('P', 'A', -90),
    ('Q', '.', 0),
    ('Q', ',', 0),
    ('Q', 'U', -10),
    ('R', 'Y', -18),
    ('R', 'W', -18),
    ('R', 'V', -18),
    ('R', 'U', -40),
    ('R', 'T', 0),
    ('R', 'O', -40),
    ('S', '.', 0),
    ('S', ',', 0),
    ('T', 'y', -74),
    ('T', 'w', -74),
    ('T', 'u', -55),
    ('T', ';', -65),
    ('T', 'r', -55),
    ('T', '.', -74),
    ('T', 'o', -92),
    ('T', 'i', -55),
    ('T', '-', -74),
    ('T', 'h', 0),
    ('T', 'e', -92),
    ('T', ',', -74),
    ('T', ':', -55),
    ('T', 'a', -92),
    ('T', 'O', -18),
    ('T', 'A', -50),
    ('U', '.', -25),
    ('U', ',', -25),
    ('U', 'A', -40),
    ('V', 'u', -74),
    ('V', ';', -74),
    ('V', '.', -129),
    ('V', 'o', -111),
    ('V', 'i', -74),
    ('V', '-', -55),
    ('V', 'e', -111),
    ('V', ',', -129),
    ('V', ':', -65),
    ('V', 'a', -111),
    ('V', 'O', -30),
    ('V', 'G', 0),
    ('V', 'A', -60),
    ('W', 'y', -70),
    ('W', 'u', -55),
    ('W', ';', -65),
    ('W', '.', -92),
    ('W', 'o', -92),
    ('W', 'i', -55),
    ('W', '-', -37),
    ('W', 'h', 0),
    ('W', 'e', -92),
    ('W', ',', -92),
    ('W', ':', -65),
    ('W', 'a', -92),
    ('W', 'O', -25),
    ('W', 'A', -60),
    ('Y', 'u', -92),
    ('Y', ';', -65),
    ('Y', '.', -92),
    ('Y', 'o', -92),
    ('Y', 'i', -74),
    ('Y', '-', -74),
    ('Y', 'e', -92),
    ('Y', ',', -92),
    ('Y', ':', -65),
    ('Y', 'a', -92),
    ('Y', 'O', -15),
    ('Y', 'A', -50),
    ('a', 'y', 0),
    ('a', 'w', 0),
    ('a', 'v', 0),
    ('a', 't', 0),
    ('a', 'p', 0),
    ('a', 'g', -10),
    ('a', 'b', 0),
    ('b', 'y', 0),
    ('b', 'v', 0),
    ('b', 'u', -20),
    ('b', '.', -40),
    ('b', 'l', 0),
    ('b', ',', 0),
    ('b', 'b', 0),
    ('c', 'y', 0),
    ('c', '.', 0),
    ('c', 'l', 0),
    ('c', 'k', -20),
    ('c', 'h', -15),
    ('c', ',', 0),
    (':', ' ', 0),
    (',', ' ', 0),
    (',', '\'', -140),
    ('d', 'y', 0),
    ('d', 'w', 0),
    ('d', 'v', 0),
    ('d', '.', 0),
    ('d', 'd', 0),
    ('d', ',', 0),
    ('e', 'y', -30),
    ('e', 'x', -20),
    ('e', 'w', -15),
    ('e', 'v', -15),
    ('e', '.', -15),
    ('e', 'p', 0),
    ('e', 'g', -40),
    ('e', ',', -10),
    ('e', 'b', 0),
    ('f', '\'', 92),
    ('f', '.', -15),
    ('f', 'o', 0),
    ('f', 'l', 0),
    ('f', 'i', -20),
    ('f', 'f', -18),
    ('f', 'e', 0),
    ('f', ',', -10),
    ('f', 'a', 0),
    ('g', 'y', 0),
    ('g', 'r', 0),
    ('g', '.', -15),
    ('g', 'o', 0),
    ('g', 'i', 0),
    ('g', 'g', -10),
    ('g', 'e', -10),
    ('g', ',', -10),
    ('g', 'a', 0),
    ('h', 'y', 0),
    ('i', 'v', 0),
    ('k', 'y', -10),
    ('k', 'o', -10),
    ('k', 'e', -10),
    ('l', 'y', 0),
    ('l', 'w', 0),
    ('m', 'y', 0),
    ('m', 'u', 0),
    ('n', 'y', 0),
    ('n', 'v', -40),
    ('n', 'u', 0),
    ('o', 'y', 0),
    ('o', 'x', 0),
    ('o', 'w', 0),
    ('o', 'v', -10),
    ('o', 'g', -10),
    ('p', 'y', 0),
    ('.', '\'', -140),
    ('`', '`', -111),
    ('`', 'A', 0),
    ('\'', 'v', -10),
    ('\'', 't', -30),
    ('\'', ' ', -111),
    ('\'', 's', -40),
    ('\'', 'r', -25),
    ('\'', '\'', -111),
    ('\'', 'l', 0),
    ('\'', 'd', -25),
    ('r', 'y', 0),
    ('r', 'v', 0),
    ('r', 'u', 0),
    ('r', 't', 0),
    ('r', 's', -10),
    ('r', 'r', 0),
    ('r', 'q', -37),
    ('r', '.', -111),
    ('r', 'p', 0),
    ('r', 'o', -45),
    ('r', 'n', 0),
    ('r', 'm', 0),
    ('r', 'l', 0),
    ('r', 'k', 0),
    ('r', 'i', 0),
    ('r', '-', -20),
    ('r', 'g', -37),
    ('r', 'e', -37),
    ('r', 'd', -37),
    ('r', ',', -111),
    ('r', 'c', -37),
    ('r', 'a', -15),
    ('s', 'w', 0),
    (' ', '`', 0),
    (' ', 'Y', -75),
    (' ', 'W', -40),
    (' ', 'V', -35),
    (' ', 'T', -18),
    (' ', 'A', -18),
    ('v', '.', -74),
    ('v', 'o', 0),
    ('v', 'e', 0),
    ('v', ',', -74),
    ('v', 'a', 0),
    ('w', '.', -74),
    ('w', 'o', 0),
    ('w', 'h', 0),
    ('w', 'e', 0),
    ('w', ',', -74),
    ('w', 'a', 0),
    ('x', 'e', 0),
    ('y', '.', -55),
    ('y', 'o', 0),
    ('y', 'e', 0),
    ('y', ',', -55),
    ('y', 'a', 0),
    ('z', 'o', 0),
    ('z', 'e', 0),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_match_afm_units() {
        assert_eq!(Core14Times::ROMAN.advance('M'), 889.0);
        assert_eq!(Core14Times::ROMAN.advance(' '), 250.0);
        assert_eq!(Core14Times::ROMAN.advance('é'), 444.0);
        assert_eq!(Core14Times::BOLD.advance('W'), 1000.0);
        assert_eq!(Core14Times::ROMAN.advance('\u{FB01}'), 556.0);
        assert_eq!(Core14Times::ROMAN.advance('🦀'), 500.0);
    }

    #[test]
    fn kerns_match_afm_pairs() {
        // Times-Roman AFM: KPX A V -135, KPX T o -80, KPX r period -55.
        assert_eq!(Core14Times::ROMAN.kern('A', 'V'), -135.0);
        assert_eq!(Core14Times::ROMAN.kern('T', 'o'), -80.0);
        assert_eq!(Core14Times::ROMAN.kern('r', '.'), -55.0);
        assert_eq!(Core14Times::ROMAN.kern('a', 'b'), 0.0);
        // Times-Bold AFM: KPX A V -145.
        assert_eq!(Core14Times::BOLD.kern('A', 'V'), -145.0);
    }

    #[test]
    fn identities_are_distinct_and_labelled() {
        assert_ne!(Core14Times::ROMAN.font_id(), Core14Times::BOLD.font_id());
        assert_eq!(
            Core14Times::ROMAN.font_id().label(),
            "core14:Times-Roman:afm"
        );
    }
}
