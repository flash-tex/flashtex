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

/// The natural width of glue/dimen text in scaled points: the leading
/// dimension with any `plus`/`minus` stretch dropped, since a set box
/// measures the natural width. `None` for text no dimension scan accepts.
fn leading_dimen_sp(text: &str, cx: &crate::text_builtins::DimenContext) -> Option<i64> {
    let mut text = text;
    for keyword in ["plus", "minus"] {
        if let Some(i) = text.find(keyword) {
            text = text[..i].trim_end();
        }
    }
    let dimen = crate::text_builtins::TextDimen::parse(text.trim())?;
    Some(i64::from(dimen.resolve(cx)))
}

/// Byte length of the leading `<dimen>` in `text`: TeX's `scan_dimen`
/// number-plus-unit shape (no `true` prefix, registers or expressions).
/// `None` when the text does not start with a number and a unit. Like
/// `scan_dimen`, the unit is one of the known two-letter units, not a
/// maximal letter run (`\kern1cmb` scans `1cm`, leaving the `b`). The end
/// is always an ASCII boundary, so slicing there is safe.
fn dimen_prefix_len(text: &str) -> Option<usize> {
    const UNITS: [&str; 11] = [
        "pt", "pc", "in", "bp", "cm", "mm", "dd", "cc", "sp", "em", "ex",
    ];
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] == b' ' || b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let num_start = i;
    while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.' || b[i] == b',') {
        i += 1;
    }
    if i == num_start {
        return None;
    }
    while i < b.len() && b[i] == b' ' {
        i += 1;
    }
    // `i` only advanced over ASCII bytes, so it is a char boundary.
    UNITS
        .iter()
        .find_map(|unit| text[i..].strip_prefix(unit).map(|_| i + unit.len()))
}

impl EngineFontMetrics {
    /// Font-relative units in a glue operand resolve against the measured
    /// box's own font.
    fn dimen_context(&self, font: u32) -> crate::text_builtins::DimenContext {
        let (quad, x_height) = self.em_ex_sp(font);
        crate::text_builtins::DimenContext {
            quad: i32::try_from(quad).unwrap_or(i32::MAX),
            x_height: i32::try_from(x_height).unwrap_or(i32::MAX),
            ..Default::default()
        }
    }

    /// Consume `\hspace`'s (the host shim's `flashtexhspacedone`, or a raw
    /// `\hspace`) optional star and `{<dimen>}` group at `tokens[i]`,
    /// adding the dimension to `glue`. The group is always consumed, so its
    /// text can never leak into the glyph run as literal characters.
    fn take_hspace(&self, tokens: &[tex::Token], i: usize, font: u32, glue: &mut i64) -> usize {
        let mut j = i + 1;
        while matches!(
            tokens.get(j).map(|t| &t.kind),
            Some(tex::TokenKind::Char(_, tex::CatCode::Space))
        ) {
            j += 1;
        }
        if matches!(
            tokens.get(j).map(|t| &t.kind),
            Some(tex::TokenKind::Char('*', _))
        ) {
            j += 1;
        }
        while matches!(
            tokens.get(j).map(|t| &t.kind),
            Some(tex::TokenKind::Char(_, tex::CatCode::Space))
        ) {
            j += 1;
        }
        if !matches!(
            tokens.get(j).map(|t| &t.kind),
            Some(tex::TokenKind::Char(_, tex::CatCode::BeginGroup))
        ) {
            return j;
        }
        let mut text = String::new();
        let mut depth = 0i32;
        let mut k = j;
        while let Some(tok) = tokens.get(k) {
            match &tok.kind {
                tex::TokenKind::Char(_, tex::CatCode::BeginGroup) => {
                    depth += 1;
                    // Nested group braces are structural, not dimension
                    // text; the outer pair never reaches this loop.
                }
                tex::TokenKind::Char(_, tex::CatCode::EndGroup) => {
                    depth -= 1;
                    if depth == 0 {
                        k += 1;
                        break;
                    }
                }
                tex::TokenKind::Char(c, _) => text.push(*c),
                _ => {}
            }
            k += 1;
        }
        if let Some(sp) = leading_dimen_sp(&text, &self.dimen_context(font)) {
            *glue += sp;
        }
        k
    }

    /// Consume a `\hskip`/`\kern` operand at `tokens[i]`: the dimension
    /// characters up to the engine's `flashtexwordbreak` terminator (which
    /// `Engine::emit_with_operand` appends to the canonical re-emission),
    /// or up to the first non-dimension token. Adds the natural width to
    /// `glue`; consumed text never shapes as glyphs. Without a terminator
    /// only the leading dimension is consumed (as `scan_dimen` would scan
    /// it) and the rest stays for the glyph run; with no dimension at all
    /// nothing is consumed, exactly as the old flattener behaved.
    fn take_skip_or_kern(
        &self,
        tokens: &[tex::Token],
        i: usize,
        font: u32,
        glue: &mut i64,
    ) -> usize {
        let mut text = String::new();
        let mut j = i + 1;
        let mut terminated = false;
        while let Some(tok) = tokens.get(j) {
            match &tok.kind {
                tex::TokenKind::ControlSequence(name) if name == "flashtexwordbreak" => {
                    j += 1;
                    terminated = true;
                    break;
                }
                tex::TokenKind::Char(c, cat) => match cat {
                    tex::CatCode::Letter | tex::CatCode::Other | tex::CatCode::Space => {
                        text.push(*c);
                        j += 1;
                    }
                    _ => break,
                },
                _ => break,
            }
        }
        let cx = self.dimen_context(font);
        if terminated {
            if let Some(sp) = leading_dimen_sp(&text, &cx) {
                *glue += sp;
            }
            return j;
        }
        if let Some(len) = dimen_prefix_len(&text) {
            if let Some(sp) = leading_dimen_sp(&text[..len], &cx) {
                *glue += sp;
                // The prefix is pure ASCII, so one token per char.
                return i + 1 + text[..len].chars().count();
            }
        }
        i + 1
    }

    /// The width parts of box content: the glyph text to shape, plus the
    /// glue/kern widths in scaled points. This is [`measurable_text`] plus
    /// the spacing commands the flattener drops:
    /// - `\quad`/`\qquad` are 1em/2em of the current font: the same `em`s
    ///   the parser lays out for body text (`parser.rs` pushes `TextGlue`
    ///   with `math::QUAD_EM`, resolved against this font's quad here).
    /// - `\hspace{<dimen>}`, `\hskip<dimen>` and `\kern<dimen>` contribute
    ///   the literal dimension.
    /// - `~` is TeX's tie: non-breaking glue with the width of a normal
    ///   interword space, never the shape of a `~` glyph.
    /// - `\\` ends the measured line, contributing no width to it.
    fn width_parts(&self, font: u32, tokens: &[tex::Token]) -> (String, i64) {
        let mut text = String::new();
        let mut glue: i64 = 0;
        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i].kind {
                tex::TokenKind::Char(ch, cat) => {
                    match cat {
                        tex::CatCode::Letter | tex::CatCode::Other | tex::CatCode::Space => {
                            text.push(*ch);
                        }
                        _ => {}
                    }
                    i += 1;
                }
                tex::TokenKind::ActiveChar(ch) => {
                    text.push(if *ch == '~' { ' ' } else { *ch });
                    i += 1;
                }
                tex::TokenKind::ControlSequence(name) => match name.as_str() {
                    "quad" => {
                        glue += (crate::math::QUAD_EM * self.em_ex_sp(font).0 as f64).round() as i64;
                        i += 1;
                    }
                    "qquad" => {
                        glue += (2.0 * crate::math::QUAD_EM * self.em_ex_sp(font).0 as f64).round()
                            as i64;
                        i += 1;
                    }
                    "hspace" | "flashtexhspacedone" => {
                        i = self.take_hspace(tokens, i, font, &mut glue);
                    }
                    "hskip" | "kern" => {
                        i = self.take_skip_or_kern(tokens, i, font, &mut glue);
                    }
                    // `\\` ends the measured line: no width on this line.
                    // (A following `[<dimen>]` optional argument is the
                    // line-breaking model's, left for the parser.)
                    "\\" => {
                        i += 1;
                    }
                    _ => {
                        i += 1;
                    }
                },
                _ => {
                    i += 1;
                }
            }
        }
        (text, glue)
    }
}

impl tex::BoxMeasurer for EngineFontMetrics {
    fn width(&self, font: u32, tokens: &[tex::Token]) -> i64 {
        let (text, glue) = self.width_parts(font, tokens);
        let shaped = if text.is_empty() {
            0
        } else {
            let (face, size_pt) = self.face_and_size(font);
            // The same memoised shaping layout measures body text with, so a
            // kerned pair or ligature measures exactly as it typesets. Unshapable
            // text (an unsupported script) measures 0, as it lays out.
            (crate::layout::text_width(&text, size_pt, face) * 65536.0).round() as i64
        };
        shaped + glue
    }

    fn height(&self, font: u32, tokens: &[tex::Token]) -> i64 {
        let text = measurable_text(tokens);
        let (face, size_pt) = self.face_and_size(font);
        let face_ref = crate::layout::face(face);
        let m = face_ref.vertical_metrics();
        // Core 14 carries no per-glyph boxes, so each character contributes
        // its class's face-declared metric: capitals and lining figures reach
        // the cap height, lower-case ascenders the ascender, every other
        // graphic character the x-height; whitespace has no height.
        let mut need_cap = false;
        let mut need_ascender = false;
        let mut need_x = false;
        for ch in text.chars() {
            if ch.is_uppercase() || ch.is_ascii_digit() {
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
        if !text.chars().any(|ch| TEXT_DESCENDER_GLYPHS.contains(&ch)) {
            return 0;
        }
        let (face, size_pt) = self.face_and_size(font);
        let face_ref = crate::layout::face(face);
        let descender = -i32::from(face_ref.vertical_metrics().descender);
        units_to_sp(descender.max(0), face_ref.units_per_em(), size_pt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flashtex_tex_expansion::BoxMeasurer as _;
    use flashtex_tex_expansion::FontMetrics as _;

    fn metrics() -> EngineFontMetrics {
        EngineFontMetrics {
            setup: FontSetup::new(None, false, false),
            preamble_latin_modern: false,
        }
    }

    fn cs(name: &str) -> tex::Token {
        tex::Token::synthetic(tex::TokenKind::ControlSequence(name.to_string()))
    }

    fn ch(c: char) -> tex::Token {
        let cat = if c == ' ' {
            tex::CatCode::Space
        } else {
            tex::CatCode::Other
        };
        tex::Token::synthetic(tex::TokenKind::Char(c, cat))
    }

    fn text(s: &str) -> Vec<tex::Token> {
        s.chars().map(ch).collect()
    }

    fn braced(dimen: &str) -> Vec<tex::Token> {
        let mut out = vec![
            tex::Token::synthetic(tex::TokenKind::Char('{', tex::CatCode::BeginGroup)),
        ];
        out.extend(text(dimen));
        out.push(tex::Token::synthetic(tex::TokenKind::Char(
            '}',
            tex::CatCode::EndGroup,
        )));
        out
    }

    /// `\hspace{<dimen>}` as the expansion engine delivers it: the host
    /// shim's `flashtexhspacedone` with the absorbed `{<dimen>}` group.
    fn hspace_tokens(dimen: &str) -> Vec<tex::Token> {
        let mut out = vec![cs("flashtexhspacedone")];
        out.extend(braced(dimen));
        out
    }

    /// `\hskip`/`\kern` as the engine delivers them: the primitive scans its
    /// operand and re-emits the command, the canonical `<dimen>` text and a
    /// `flashtexwordbreak` terminator (`Engine::emit_with_operand`).
    fn skip_tokens(command: &str, operand: &str) -> Vec<tex::Token> {
        let mut out = vec![cs(command)];
        out.extend(text(operand));
        out.push(cs("flashtexwordbreak"));
        out
    }

    fn width_of(tokens: &[tex::Token]) -> i64 {
        metrics().width(0, tokens)
    }

    /// 1cm in scaled points, TeX's own `scan_dimen` value
    /// (`7227/254` of a point, tex.web section 458).
    const ONE_CM_SP: i64 = 1_864_679;

    #[test]
    fn quad_adds_exactly_one_em() {
        let plain = width_of(&text("ab"));
        let mut spaced = text("a");
        spaced.push(cs("quad"));
        spaced.push(ch('b'));
        let quad = metrics().quad_sp_in(0);
        assert!(quad > 600_000 && quad < 700_000, "sanity: 1em near 10pt: {quad}");
        assert_eq!(width_of(&spaced) - plain, quad);
    }

    #[test]
    fn qquad_adds_exactly_two_ems() {
        let plain = width_of(&text("ab"));
        let mut spaced = text("a");
        spaced.push(cs("qquad"));
        spaced.push(ch('b'));
        assert_eq!(width_of(&spaced) - plain, 2 * metrics().quad_sp_in(0));
    }

    #[test]
    fn glue_only_box_measures_without_glyphs() {
        assert_eq!(width_of(&[cs("quad")]), metrics().quad_sp_in(0));
    }

    #[test]
    fn hspace_cm_adds_exactly_one_cm() {
        let plain = width_of(&text("ab"));
        let mut spaced = text("a");
        spaced.extend(hspace_tokens("1cm"));
        spaced.push(ch('b'));
        // Exactly the dimension: the `1cm` argument must not also leak in
        // as the glyphs "1cm".
        assert_eq!(width_of(&spaced) - plain, ONE_CM_SP);
    }

    #[test]
    fn hspace_star_form_adds_the_dimen() {
        let plain = width_of(&text("ab"));
        let mut spaced = text("a");
        spaced.push(cs("flashtexhspacedone"));
        spaced.push(ch('*'));
        spaced.extend(braced("1cm"));
        spaced.push(ch('b'));
        assert_eq!(width_of(&spaced) - plain, ONE_CM_SP);
    }

    #[test]
    fn hskip_and_kern_add_the_literal_dimen() {
        let plain = width_of(&text("ab"));
        for command in ["hskip", "kern"] {
            let mut spaced = text("a");
            // The canonical re-emission (`\the`-style `pt` text).
            spaced.extend(skip_tokens(command, "28.45274pt"));
            spaced.push(ch('b'));
            assert_eq!(width_of(&spaced) - plain, ONE_CM_SP, "{command}");
        }
        // A raw dimension without the terminator parses the same way.
        let mut spaced = text("a");
        spaced.push(cs("kern"));
        spaced.extend(text("1cm"));
        spaced.push(ch('b'));
        assert_eq!(width_of(&spaced) - plain, ONE_CM_SP);
    }

    #[test]
    fn tilde_measures_as_a_normal_interword_space() {
        let mut with_tilde = text("a");
        with_tilde.push(tex::Token::synthetic(tex::TokenKind::ActiveChar('~')));
        with_tilde.push(ch('b'));
        let with_space = text("a b");
        assert!(width_of(&with_space) > 0);
        assert_eq!(width_of(&with_tilde), width_of(&with_space));
    }

    #[test]
    fn linebreak_adds_no_width() {
        let plain = width_of(&text("ab"));
        let mut broken = text("a");
        broken.push(cs("\\"));
        broken.push(ch('b'));
        assert_eq!(width_of(&broken), plain);
    }

    /// End to end through the real expansion pipeline: the engine's
    /// `expand_fully` delivers the token shapes above to the measurer.
    fn rendered_dim(source: &str) -> f64 {
        let output = crate::incremental::compile_full(
            source,
            crate::layout::LayoutConstraints::default(),
        );
        assert!(
            !output
                .diagnostics
                .iter()
                .any(|d| d.message.contains("not supported")),
            "unsupported: {:?}",
            output.diagnostics
        );
        let rendered: String = output
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .map(|item| item.text.clone())
            .collect();
        assert!(rendered.ends_with("pt"), "a dimension renders: {rendered:?}");
        rendered.trim_end_matches("pt").parse().expect("numeric dimension")
    }

    #[test]
    fn end_to_end_quad_and_hspace_grow_the_box() {
        let plain = rendered_dim(r"\newlength{\mywidth}\settowidth{\mywidth}{ab}\the\mywidth");
        let quad = rendered_dim(r"\newlength{\mywidth}\settowidth{\mywidth}{a\quad b}\the\mywidth");
        assert!(quad > plain, "a quad widens the box: {quad} vs {plain}");
        let hspace = rendered_dim(r"\newlength{\mywidth}\settowidth{\mywidth}{a\hspace{1cm}b}\the\mywidth");
        assert!(hspace > plain, "1cm of space widens the box: {hspace} vs {plain}");
        // 1cm prints as 28.45274pt; print rounding may move the last digit.
        assert!((hspace - plain - 28.45274).abs() < 0.0001, "{hspace} vs {plain}");
    }

    #[test]
    fn end_to_end_tilde_equals_a_space() {
        let space = rendered_dim(r"\newlength{\mywidth}\settowidth{\mywidth}{a b}\the\mywidth");
        let tilde = rendered_dim(r"\newlength{\mywidth}\settowidth{\mywidth}{a~b}\the\mywidth");
        assert_eq!(space, tilde);
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
