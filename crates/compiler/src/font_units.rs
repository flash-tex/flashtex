//! TeX's font-relative units for the text font a style selects.
//!
//! `em` is the current font's `\fontdimen6` (quad) and `ex` its
//! `\fontdimen5` (x-height), not the point size: cmr12's quad is 11.74988pt,
//! so `3em` under `\documentclass[12pt]` is 35.24963pt, and `\small`,
//! `\bfseries`, `\sffamily`, `\ttfamily`, italics, slanted, small-caps,
//! `fontenc` T1 and `lmodern` each load another TFM with other values.
//! [`crate::text_fontdimens`] records
//! them per NFSS font from pdflatex. This is independent of the compiler's own
//! Core14 layout faces: these are the values `\the`/`\showthe` report.

use crate::parser::{FontSizeLevel, TextFamily, TextStyle, TEXT_DESCENDER_GLYPHS};
use crate::text_fontdimens::{row, FONTDIMENS, SIZES_PT};
use flashtex_font_engine::Face as _;
use flashtex_tex_expansion as tex;

/// The document-wide inputs to NFSS text-font selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FontSetup {
    /// `\documentclass[10pt|11pt|12pt]`; the standard classes default to 10pt.
    pub class_pt: f64,
    /// `\usepackage[T1]{fontenc}` is in force (EC fonts).
    pub t1: bool,
    /// `\usepackage{lmodern}`'s families are the ones selected.
    pub latin_modern: bool,
}

impl FontSetup {
    pub(crate) fn new(class_pt: Option<f64>, t1: bool, latin_modern: bool) -> Self {
        FontSetup {
            class_pt: class_pt.unwrap_or(10.0),
            t1,
            latin_modern,
        }
    }

    /// The point size of `\normalsize` (11pt is `\@xipt`, 10.95pt) or of a
    /// size declaration.
    pub(crate) fn size_pt(self, level: Option<FontSizeLevel>) -> f64 {
        match level {
            Some(level) => crate::layout::size_declaration_pt(level, self.class_pt),
            None if self.class_pt > 11.5 => 12.0,
            None if self.class_pt > 10.5 => 10.95,
            None => 10.0,
        }
    }

    /// `(em, ex)` in scaled points for `style`.
    pub(crate) fn em_ex_sp(self, style: TextStyle) -> (i64, i64) {
        let family = match style.family {
            TextFamily::Roman => 0,
            TextFamily::Sans => 1,
            TextFamily::Mono => 2,
        };
        let shape = if style.small_caps {
            3
        } else if style.slanted {
            2
        } else if style.italic {
            1
        } else {
            0
        };
        let cells = &FONTDIMENS[row(self.latin_modern, self.t1, family, style.bold, shape)];
        let size = self.size_pt(style.size);
        let (index, nearest) = SIZES_PT
            .iter()
            .enumerate()
            .min_by(|a, b| (a.1 - size).abs().total_cmp(&(b.1 - size).abs()))
            .expect("sizes are not empty");
        let (quad, x_height) = cells[index];
        if (nearest - size).abs() < 0.005 {
            (i64::from(quad), i64::from(x_height))
        } else {
            // Not a standard LaTeX size: scale the nearest design's values.
            let scale = size / nearest;
            (
                (f64::from(quad) * scale).round() as i64,
                (f64::from(x_height) * scale).round() as i64,
            )
        }
    }
}

// The expansion engine's font selector (`tex::FontSwitch`): the size
// declaration, series, shape and family of `TextStyle`, plus whether
// `\begin{document}` has run (`lmodern` takes effect there).
const SIZE: u32 = 0xF;
const BOLD: u32 = 1 << 4;
const ITALIC: u32 = 1 << 5;
const SLANTED: u32 = 1 << 9;
const SMALL_CAPS: u32 = 1 << 10;
const SHAPE: u32 = ITALIC | SLANTED | SMALL_CAPS;
const FAMILY: u32 = 3 << 6;
const SANS: u32 = 1 << 6;
const MONO: u32 = 2 << 6;
const BODY: u32 = 1 << 8;

const SIZE_LEVELS: [FontSizeLevel; 9] = [
    FontSizeLevel::Tiny,
    FontSizeLevel::ScriptSize,
    FontSizeLevel::FootnoteSize,
    FontSizeLevel::Small,
    FontSizeLevel::Large1,
    FontSizeLevel::Large2,
    FontSizeLevel::Large3,
    FontSizeLevel::Huge1,
    FontSizeLevel::Huge2,
];

const SIZE_NAMES: [&str; 9] = [
    "tiny",
    "scriptsize",
    "footnotesize",
    "small",
    "large",
    "Large",
    "LARGE",
    "huge",
    "Huge",
];

fn switch(clear: u32, set: u32) -> tex::FontSwitch {
    tex::FontSwitch {
        clear,
        set,
        toggle: 0,
        argument: false,
    }
}

fn argument(mut switch: tex::FontSwitch) -> tex::FontSwitch {
    switch.argument = true;
    switch
}

/// The font commands the engine tracks, mirroring LaTeX's declarations (and
/// `parser::apply_style` for the attributes `TextStyle` models).
pub(crate) fn font_switches() -> Vec<(&'static str, tex::FontSwitch)> {
    let emph = tex::FontSwitch {
        clear: SLANTED | SMALL_CAPS,
        set: 0,
        toggle: ITALIC,
        argument: false,
    };
    let mut switches = vec![
        ("bfseries", switch(BOLD, BOLD)),
        ("textbf", argument(switch(BOLD, BOLD))),
        ("mdseries", switch(BOLD, 0)),
        ("textmd", argument(switch(BOLD, 0))),
        ("itshape", switch(SHAPE, ITALIC)),
        ("slshape", switch(SHAPE, ITALIC | SLANTED)),
        ("textit", argument(switch(SHAPE, ITALIC))),
        ("textsl", argument(switch(SHAPE, ITALIC | SLANTED))),
        ("scshape", switch(SHAPE, SMALL_CAPS)),
        ("textsc", argument(switch(SHAPE, SMALL_CAPS))),
        ("upshape", switch(SHAPE, 0)),
        ("textup", argument(switch(SHAPE, 0))),
        ("em", emph),
        ("emph", argument(emph)),
        ("rmfamily", switch(FAMILY, 0)),
        ("textrm", argument(switch(FAMILY, 0))),
        ("sffamily", switch(FAMILY, SANS)),
        ("textsf", argument(switch(FAMILY, SANS))),
        ("ttfamily", switch(FAMILY, MONO)),
        ("texttt", argument(switch(FAMILY, MONO))),
        // `\normalfont` keeps the size.
        ("normalfont", switch(BOLD | SHAPE | FAMILY, 0)),
        ("textnormal", argument(switch(BOLD | SHAPE | FAMILY, 0))),
        // LaTeX 2.09 forms are `\normalfont` plus one attribute.
        ("rm", switch(BOLD | SHAPE | FAMILY, 0)),
        ("sf", switch(BOLD | SHAPE | FAMILY, SANS)),
        ("tt", switch(BOLD | SHAPE | FAMILY, MONO)),
        ("bf", switch(BOLD | SHAPE | FAMILY, BOLD)),
        ("it", switch(BOLD | SHAPE | FAMILY, ITALIC)),
        ("sl", switch(BOLD | SHAPE | FAMILY, ITALIC | SLANTED)),
        ("sc", switch(BOLD | SHAPE | FAMILY, SMALL_CAPS)),
        ("normalsize", switch(SIZE, 0)),
        ("document", switch(BODY, BODY)),
    ];
    for (code, name) in SIZE_NAMES.iter().enumerate() {
        switches.push((name, switch(SIZE, code as u32 + 1)));
    }
    switches
}

/// The expansion engine's `em`/`ex`: [`FontSetup`] for the selector the
/// engine tracked. `lmodern` only replaces the preamble's already-selected
/// Computer Modern when a later `\selectfont` (fontenc) ran.
///
/// This is also the engine's `\settowidth`/`\settoheight`/`\settodepth`
/// measurer ([`tex::BoxMeasurer`]): the same selector decodes to the Core 14
/// face layout would set ([`crate::layout::style_font`]) at the
/// declaration's point size, and the content is measured with the
/// font engine's real AFM advances and vertical metrics.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EngineFontMetrics {
    pub setup: FontSetup,
    pub preamble_latin_modern: bool,
}

impl EngineFontMetrics {
    /// The NFSS style the engine's font selector addresses, with the
    /// effective setup (see the struct docs for the `lmodern` rule).
    fn style_and_setup(&self, font: u32) -> (TextStyle, FontSetup) {
        let level = (font & SIZE) as usize;
        let style = TextStyle {
            bold: font & BOLD != 0,
            italic: font & ITALIC != 0,
            slanted: font & SLANTED != 0,
            small_caps: font & SMALL_CAPS != 0,
            family: match font & FAMILY {
                SANS => TextFamily::Sans,
                MONO => TextFamily::Mono,
                _ => TextFamily::Roman,
            },
            size: level
                .checked_sub(1)
                .and_then(|i| SIZE_LEVELS.get(i).copied()),
            // The expansion engine's size codes address only the nine
            // document-visible levels, never the AMS `\Tiny` rung (GH-824).
            ams_tiny: false,
            color: None,
            cjk: None,
            ..TextStyle::default()
        };
        let latin_modern = if font & BODY != 0 {
            self.setup.latin_modern
        } else {
            self.preamble_latin_modern
        };
        (
            style,
            FontSetup {
                latin_modern,
                ..self.setup
            },
        )
    }

    fn em_ex_sp(&self, font: u32) -> (i64, i64) {
        let (style, setup) = self.style_and_setup(font);
        setup.em_ex_sp(style)
    }

    /// The Core 14 face and point size the selector's style sets in.
    fn face_and_size(&self, font: u32) -> (crate::layout::Font, f64) {
        let (style, setup) = self.style_and_setup(font);
        (crate::layout::style_font(style), setup.size_pt(style.size))
    }
}

/// The measurable characters of `\settowidth`-style box content: letters,
/// digits, punctuation and spaces. Group braces, math shifts and other
/// structural tokens carry no ink; control sequences (spacing and font
/// commands the engine left for the host, `\hskip` glue, `\\`) have no
/// glyph advance the shaper could measure, so they contribute nothing.
fn measurable_text(tokens: &[tex::Token]) -> String {
    let mut out = String::new();
    for tok in tokens {
        match &tok.kind {
            tex::TokenKind::Char(ch, cat) => match cat {
                tex::CatCode::Letter | tex::CatCode::Other | tex::CatCode::Space => out.push(*ch),
                _ => {}
            },
            tex::TokenKind::ActiveChar(ch) => out.push(*ch),
            _ => {}
        }
    }
    out
}

/// Font units to scaled points at `size_pt`, rounding to the nearest
/// integer like a measured (not scanned) TeX dimension.
fn units_to_sp(units: i32, units_per_em: u16, size_pt: f64) -> i64 {
    (f64::from(units) * size_pt / f64::from(units_per_em) * 65536.0).round() as i64
}

/// Punctuation whose glyphs rise well above the x-height toward the
/// ascender line: parens, brackets, the slash and the bar. `{`/`}` are
/// included although escaped `\{`/`\}` does not reach this path today
/// (probed: `\settoheight{\h}{\{}` measures 0.0pt with and without T1
/// `fontenc` — the escape never becomes measurable ink): a grouping brace
/// arrives as `BeginGroup`/`EndGroup` and carries no ink, so listing the
/// characters here is harmless now and stays correct if brace ink ever
/// arrives as `Other` (a future `\chardef` definition, or
/// `\catcode`\{=12`).
fn is_tall_punctuation(ch: char) -> bool {
    matches!(ch, '(' | ')' | '[' | ']' | '/' | '|' | '{' | '}')
}

/// Precomposed Latin-1 Supplement / Latin Extended-A capitals whose
/// diacritic sits above the base letter (É, Ä, Ñ, Ā, Š, Ż, ...), so the
/// glyph reaches past the cap height.
///
/// An explicit table rather than a Unicode property: `char` offers no
/// "diacritic position" property, and pulling in a normalization crate for
/// one height bucket would be disproportionate next to the hardcoded
/// buckets this measurer already uses. Ranges may admit interleaved
/// lowercase code points (ć between Ć and Ĉ); the `is_uppercase` guard
/// filters those back out, keeping lowercase accents in their old bucket
/// (out of scope for this slice). Letters whose mark sits below or through
/// the base (Ç, Ą, Ę, Ø, Ł, ...) are deliberately excluded: their height
/// is the cap height they already classify into.
fn is_accented_capital(ch: char) -> bool {
    ch.is_uppercase()
        && matches!(ch,
            // Latin-1 Supplement: À–Å, È–Ë, Ì–Ï, Ñ, Ò–Ö, Ù–Ü, Ý
            // (Æ ligature, Ç cedilla-below, Ð eth, Ø stroke excluded).
            '\u{C0}'..='\u{C5}'
            | '\u{C8}'..='\u{CB}'
            | '\u{CC}'..='\u{CF}'
            | '\u{D1}'
            | '\u{D2}'..='\u{D6}'
            | '\u{D9}'..='\u{DC}'
            | '\u{DD}'
            // Latin Extended-A capitals with an above-diacritic; cedilla /
            // ogonek / stroke / ligature letters (Ą Đ Ę Ģ Ħ Į Ĳ Ķ Ļ Ł Ņ Ŗ
            // Ş Ţ Ŧ Ų ...) are excluded and keep the cap-height bucket.
            | '\u{100}'..='\u{102}'
            | '\u{106}'..='\u{10C}'
            | '\u{10E}'
            | '\u{112}'..='\u{116}'
            | '\u{11A}'..='\u{120}'
            | '\u{124}'
            | '\u{128}'..='\u{12C}'
            | '\u{130}'
            | '\u{134}'
            | '\u{139}'
            | '\u{13D}'
            | '\u{143}'
            | '\u{147}'
            | '\u{14C}'..='\u{150}'
            | '\u{154}'
            | '\u{158}'
            | '\u{15A}'..='\u{15C}'
            | '\u{160}'
            | '\u{164}'
            | '\u{168}'..='\u{170}'
            | '\u{174}'
            | '\u{176}'
            | '\u{178}'..='\u{17D}')
}

impl tex::BoxMeasurer for EngineFontMetrics {
    fn width(&self, font: u32, tokens: &[tex::Token]) -> i64 {
        let text = measurable_text(tokens);
        if text.is_empty() {
            return 0;
        }
        let (face, size_pt) = self.face_and_size(font);
        // The same memoised shaping layout measures body text with, so a
        // kerned pair or ligature measures exactly as it typesets. Unshapable
        // text (an unsupported script) measures 0, as it lays out.
        let pt = crate::layout::text_width(&text, size_pt, face);
        (pt * 65536.0).round() as i64
    }

    fn height(&self, font: u32, tokens: &[tex::Token]) -> i64 {
        let text = measurable_text(tokens);
        let (face, size_pt) = self.face_and_size(font);
        let face_ref = crate::layout::face(face);
        let m = face_ref.vertical_metrics();
        // Core 14 carries no per-glyph boxes (the `Face` trait exposes only
        // face-wide `vertical_metrics`/`bbox`, advances and glyph ids), so
        // each character contributes its class's face-declared metric:
        // capitals and lining figures reach the cap height, lower-case
        // ascenders the ascender, every other graphic character the
        // x-height; whitespace has no height. Two classes rise past their
        // naive bucket and join the ascender one instead: tall punctuation
        // (parens/brackets/slash/bar reach near the ascender line, well
        // above x-height) and precomposed capitals with an above-diacritic
        // (É reaches past the cap height). Real parens overshoot even the
        // ascender toward the face bbox top, but with no per-glyph metric
        // the ascender is the closest grounded face value.
        let mut need_cap = false;
        let mut need_ascender = false;
        let mut need_x = false;
        for ch in text.chars() {
            if is_tall_punctuation(ch) || is_accented_capital(ch) {
                need_ascender = true;
            } else if ch.is_uppercase() || ch.is_ascii_digit() {
                need_cap = true;
            } else if matches!(ch, 'b' | 'd' | 'f' | 'h' | 'i' | 'k' | 'l' | 't') {
                need_ascender = true;
            } else if !ch.is_whitespace() && !ch.is_control() {
                need_x = true;
            }
        }
        let mut units: i16 = 0;
        if need_x {
            units = units.max(m.x_height);
        }
        if need_cap {
            units = units.max(m.cap_height);
        }
        if need_ascender {
            units = units.max(m.ascender);
        }
        units_to_sp(i32::from(units.max(0)), face_ref.units_per_em(), size_pt)
    }

    fn depth(&self, font: u32, tokens: &[tex::Token]) -> i64 {
        let text = measurable_text(tokens);
        // The same descender-glyph test layout underlines by: only that
        // content reaches below the baseline, by the face's own descender.
        // Comma and period are additive on top of that set (which is
        // untouched, so `TEXT_DESCENDER_GLYPHS` itself is not modified): the
        // comma tail drops nearly to the descender line, so it contributes
        // the full descender, while the round period merely overshoots the
        // baseline and gets a quarter of it. Both fractions are conventional
        // approximations, documented as such — Core 14 exposes no per-glyph
        // depth to ground them in — but their order (comma deeper than
        // period) matches the glyphs.
        let full = text.chars().any(|ch| TEXT_DESCENDER_GLYPHS.contains(&ch));
        let comma = text.contains(',');
        if !full && !comma && !text.contains('.') {
            return 0;
        }
        let (face, size_pt) = self.face_and_size(font);
        let face_ref = crate::layout::face(face);
        let descender = (-i32::from(face_ref.vertical_metrics().descender)).max(0);
        let units = if full || comma { descender } else { descender / 4 };
        units_to_sp(units, face_ref.units_per_em(), size_pt)
    }
}

impl tex::FontMetrics for EngineFontMetrics {
    fn quad_sp(&self) -> i64 {
        self.quad_sp_in(0)
    }

    fn x_height_sp(&self) -> i64 {
        self.x_height_sp_in(0)
    }

    fn quad_sp_in(&self, font: u32) -> i64 {
        self.em_ex_sp(font).0
    }

    fn x_height_sp_in(&self, font: u32) -> i64 {
        self.em_ex_sp(font).1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Box content as the expansion engine delivers it after `expand_fully`:
    /// letters lex as `Letter`, everything else printable as `Other`; a
    /// grouping brace stays `BeginGroup`/`EndGroup` and carries no ink.
    fn toks(s: &str) -> Vec<tex::Token> {
        s.chars()
            .map(|ch| {
                let cat = if ch == ' ' {
                    tex::CatCode::Space
                } else if ch == '{' {
                    tex::CatCode::BeginGroup
                } else if ch == '}' {
                    tex::CatCode::EndGroup
                } else if ch.is_ascii_alphabetic() {
                    tex::CatCode::Letter
                } else {
                    tex::CatCode::Other
                };
                tex::Token::synthetic(tex::TokenKind::Char(ch, cat))
            })
            .collect()
    }

    /// A brace arriving as `Other` (as a future `\chardef`-defined `\{`
    /// would, or a literal brace after `\catcode`\{=12`): measurable ink
    /// that should classify as tall punctuation.
    fn escaped_brace_toks(s: &str) -> Vec<tex::Token> {
        s.chars()
            .map(|ch| tex::Token::synthetic(tex::TokenKind::Char(ch, tex::CatCode::Other)))
            .collect()
    }

    fn metrics() -> EngineFontMetrics {
        EngineFontMetrics {
            setup: FontSetup::new(None, false, false),
            preamble_latin_modern: false,
        }
    }

    fn height_of(m: &EngineFontMetrics, s: &str) -> i64 {
        tex::BoxMeasurer::height(m, 0, &toks(s))
    }

    fn depth_of(m: &EngineFontMetrics, s: &str) -> i64 {
        tex::BoxMeasurer::depth(m, 0, &toks(s))
    }

    #[test]
    fn tall_punctuation_reaches_past_x_height_to_cap_height() {
        let m = metrics();
        let paren = height_of(&m, "(");
        assert!(
            paren > height_of(&m, "x"),
            "paren should exceed the x-height bucket: {paren}"
        );
        assert!(
            paren >= height_of(&m, "A"),
            "paren should reach at least cap height: {paren}"
        );
        for glyph in [")", "[", "]", "/", "|"] {
            assert_eq!(
                height_of(&m, glyph),
                paren,
                "{glyph:?} should share the paren bucket"
            );
        }
        assert_eq!(
            tex::BoxMeasurer::height(&m, 0, &escaped_brace_toks("{")),
            paren,
            "an escaped brace is tall punctuation too"
        );
    }

    #[test]
    fn grouping_braces_carry_no_height() {
        let m = metrics();
        assert_eq!(height_of(&m, "{}"), 0);
    }

    #[test]
    fn comma_and_period_have_positive_depth_with_comma_deeper() {
        let m = metrics();
        let comma = depth_of(&m, ",");
        let period = depth_of(&m, ".");
        assert!(comma > 0, "comma should have positive depth");
        assert!(period > 0, "period should have positive depth");
        assert!(
            comma > period,
            "comma should reach further below the baseline than a period"
        );
    }

    #[test]
    fn existing_descender_glyphs_keep_full_descender_depth() {
        let m = metrics();
        let full = depth_of(&m, "g");
        assert!(full > 0);
        for glyph in ["j", "p", "q", "y", "Q"] {
            assert_eq!(depth_of(&m, glyph), full, "{glyph:?} keeps full depth");
        }
        assert_eq!(depth_of(&m, ","), full, "comma reaches the descender line");
        assert_eq!(depth_of(&m, "g,"), full, "mixed content takes the max");
        assert_eq!(depth_of(&m, "a"), 0, "descender-free text stays depthless");
    }

    #[test]
    fn accented_capitals_reach_past_cap_height() {
        let m = metrics();
        let cap = height_of(&m, "E");
        assert!(cap > 0);
        for glyph in ["É", "Ä", "À", "Ñ", "Ü", "Ā", "Š", "Ż"] {
            let h = height_of(&m, glyph);
            assert!(h >= cap, "{glyph:?} should reach at least cap height");
            assert!(
                h > cap,
                "{glyph:?} should exceed cap height via the ascender bucket"
            );
        }
    }

    #[test]
    fn below_base_accents_and_lowercase_stay_in_their_buckets() {
        let m = metrics();
        // Cedilla/ogonek sit below the base letter: no extra height.
        assert_eq!(height_of(&m, "Ç"), height_of(&m, "E"));
        // Lowercase accents are out of scope and keep the old buckets.
        assert_eq!(height_of(&m, "é"), height_of(&m, "x"));
    }

    #[test]
    fn old_buckets_unchanged() {
        let m = metrics();
        assert_eq!(height_of(&m, "Ag"), height_of(&m, "A"));
        assert!(height_of(&m, "b") > height_of(&m, "A"));
        assert_eq!(height_of(&m, " "), 0);
    }
}
