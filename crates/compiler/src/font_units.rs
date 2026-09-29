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

    /// The height of one run's text in its own font and size: Core 14
    /// carries no per-glyph boxes, so each character contributes its
    /// class's face-declared metric — capitals and lining figures reach
    /// the cap height, lower-case ascenders the ascender, every other
    /// graphic character the x-height; whitespace has no height.
    fn run_height_sp(&self, run_font: u32, text: &str) -> i64 {
        let (face, size_pt) = self.face_and_size(run_font);
        let face_ref = crate::layout::face(face);
        let m = face_ref.vertical_metrics();
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

    /// The depth of one run's text in its own font and size: the face's
    /// own descender when the run holds a descender glyph (the same
    /// descender-glyph test layout underlines by), else 0.
    fn run_depth_sp(&self, run_font: u32, text: &str) -> i64 {
        if !text
            .chars()
            .any(|ch| TEXT_DESCENDER_GLYPHS.contains(&ch))
        {
            return 0;
        }
        let (face, size_pt) = self.face_and_size(run_font);
        let face_ref = crate::layout::face(face);
        let descender = -i32::from(face_ref.vertical_metrics().descender);
        units_to_sp(descender.max(0), face_ref.units_per_em(), size_pt)
    }
}

/// A host font command's effect on the engine's font selector, mirroring
/// the table the engine was given ([`font_switches`]): `None` for content
/// the engine left for the host that carries no font change.
fn lookup_switch(name: &str) -> Option<tex::FontSwitch> {
    font_switches()
        .into_iter()
        .find_map(|(command, switch)| (command == name).then_some(switch))
}

/// The box content as `(font selector, text)` runs. The engine emits host
/// font commands ([`Engine::declare_font_switch`]) into the content stream
/// it hands the measurer, so a declaration (`\bfseries`, `\large`) switches
/// the running font from that token on, an argument command (`\textbf`,
/// `\texttt`) switches the brace group that follows it, and groups (`{..}`
/// and `\begingroup..\endgroup`) restore the font they entered with — the
/// same save-stack discipline the engine applies while producing the
/// stream, including an argument switch waiting out spaces (`\textbf {..}`)
/// and dying on any other content. Runs with no ink are dropped, and text
/// that never sees a switch stays one run, exactly as before. Width sums
/// the runs; height and depth take the maximum across them.
fn split_font_runs(start: u32, tokens: &[tex::Token]) -> Vec<(u32, String)> {
    let mut runs: Vec<(u32, String)> = Vec::new();
    let mut current = start;
    let mut stack: Vec<u32> = Vec::new();
    let mut pending: Option<tex::FontSwitch> = None;
    let mut text = String::new();
    for tok in tokens {
        match &tok.kind {
            tex::TokenKind::Char(_, tex::CatCode::BeginGroup) => {
                if !text.is_empty() {
                    runs.push((current, std::mem::take(&mut text)));
                }
                stack.push(current);
                if let Some(switch) = pending.take() {
                    current = switch.apply(current);
                }
            }
            tex::TokenKind::Char(_, tex::CatCode::EndGroup) => {
                if let Some(restored) = stack.pop() {
                    if restored != current && !text.is_empty() {
                        runs.push((current, std::mem::take(&mut text)));
                    }
                    current = restored;
                }
                pending = None;
            }
            // Spaces keep a pending argument switch alive; any other
            // content token means there was no brace group for it.
            tex::TokenKind::Char(ch, tex::CatCode::Space) => text.push(*ch),
            tex::TokenKind::Char(ch, tex::CatCode::Letter)
            | tex::TokenKind::Char(ch, tex::CatCode::Other) => {
                pending = None;
                text.push(*ch);
            }
            tex::TokenKind::Char(..) => pending = None,
            tex::TokenKind::ControlSequence(name) => {
                if let Some(switch) = lookup_switch(name) {
                    if switch.argument {
                        pending = Some(switch);
                    } else {
                        if !text.is_empty() {
                            runs.push((current, std::mem::take(&mut text)));
                        }
                        pending = None;
                        current = switch.apply(current);
                    }
                } else if name == "begingroup" {
                    stack.push(current);
                    pending = None;
                } else if name == "endgroup" {
                    if let Some(restored) = stack.pop() {
                        if restored != current && !text.is_empty() {
                            runs.push((current, std::mem::take(&mut text)));
                        }
                        current = restored;
                    }
                    pending = None;
                } else {
                    pending = None;
                }
            }
            tex::TokenKind::ActiveChar(ch) => {
                pending = None;
                text.push(*ch);
            }
            tex::TokenKind::Param(_) | tex::TokenKind::Eof => {}
        }
    }
    if !text.is_empty() {
        runs.push((current, std::mem::take(&mut text)));
    }
    runs
}

/// Font units to scaled points at `size_pt`, rounding to the nearest
/// integer like a measured (not scanned) TeX dimension.
fn units_to_sp(units: i32, units_per_em: u16, size_pt: f64) -> i64 {
    (f64::from(units) * size_pt / f64::from(units_per_em) * 65536.0).round() as i64
}

impl tex::BoxMeasurer for EngineFontMetrics {
    fn width(&self, font: u32, tokens: &[tex::Token]) -> i64 {
        // The content is split into runs at font/size switches (see
        // `split_font_runs`) so each run measures in its own font; the
        // run widths sum. Text that never sees a switch stays one run.
        split_font_runs(font, tokens)
            .iter()
            .map(|(run_font, text)| {
                let (face, size_pt) = self.face_and_size(*run_font);
                // The same memoised shaping layout measures body text with,
                // so a kerned pair or ligature measures exactly as it
                // typesets. Unshapable text (an unsupported script)
                // measures 0, as it lays out.
                //
                // Note this is an approximation of what pdflatex reports:
                // the runs shape with Core 14 (Times/Helvetica/Courier)
                // AFM advances while pdflatex sets article text in the TFM
                // fonts (Computer Modern), so absolute values are typically
                // a few percent off even with correct run splitting.
                // Closing that gap needs the shared TFM metric source
                // tracked for issue #1063; this measurer only fixes which
                // font each run measures in.
                let pt = crate::layout::text_width(text, size_pt, face);
                (pt * 65536.0).round() as i64
            })
            .sum()
    }

    fn height(&self, font: u32, tokens: &[tex::Token]) -> i64 {
        // Like `width`, the content is split into font runs so each run
        // measures in its own font and size; the box height is the tallest
        // run's. Measuring in the outer font only understated a size
        // switch: `\settoheight{\x}{\Large A}` gave 6.62pt (cap height at
        // 10pt) where pdflatex reports 9.84pt.
        split_font_runs(font, tokens)
            .iter()
            .map(|(run_font, text)| self.run_height_sp(*run_font, text))
            .max()
            .unwrap_or(0)
    }

    fn depth(&self, font: u32, tokens: &[tex::Token]) -> i64 {
        // As `height`: each run measures in its own font and size and the
        // box depth is the deepest run's.
        split_font_runs(font, tokens)
            .iter()
            .map(|(run_font, text)| self.run_depth_sp(*run_font, text))
            .max()
            .unwrap_or(0)
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
    use crate::layout::{text_width, Font};
    use flashtex_tex_expansion::BoxMeasurer as _;

    /// The article-10pt measurer: the selector starts at 0 (no switches)
    /// and `\normalsize` is 10pt.
    fn metrics() -> EngineFontMetrics {
        EngineFontMetrics {
            setup: FontSetup::new(Some(10.0), false, false),
            preamble_latin_modern: false,
        }
    }

    fn sp(pt: f64) -> i64 {
        (pt * 65536.0).round() as i64
    }

    fn expected(text: &str, size_pt: f64, font: Font) -> i64 {
        sp(text_width(text, size_pt, font))
    }

    fn letter(ch: char) -> tex::Token {
        let cat = if ch == ' ' {
            tex::CatCode::Space
        } else {
            tex::CatCode::Letter
        };
        tex::Token::synthetic(tex::TokenKind::Char(ch, cat))
    }

    fn cs(name: &str) -> tex::Token {
        tex::Token::synthetic(tex::TokenKind::ControlSequence(name.to_string()))
    }

    fn begin() -> tex::Token {
        tex::Token::synthetic(tex::TokenKind::Char('{', tex::CatCode::BeginGroup))
    }

    fn end() -> tex::Token {
        tex::Token::synthetic(tex::TokenKind::Char('}', tex::CatCode::EndGroup))
    }

    fn word(text: &str) -> Vec<tex::Token> {
        text.chars().map(letter).collect()
    }

    /// The six GH #1105 item-1 cases as the engine emits them: host font
    /// commands stay in the stream as control sequences around the group
    /// braces (see `Engine::declare_font_switch`).
    fn case_tokens(content: &str) -> Vec<tex::Token> {
        match content {
            "plain" => word("Hi"),
            "bfseries" => [vec![begin(), cs("bfseries")], word("Hi"), vec![end()]].concat(),
            "textbf" => {
                [vec![cs("textbf"), begin()], word("Hi"), vec![end()], word(" x")].concat()
            }
            "itshape" => [vec![begin(), cs("itshape")], word("f"), vec![end()]].concat(),
            "large" => [vec![cs("large")], word("Hi")].concat(),
            "texttt" => [vec![cs("texttt"), begin()], word("Hi"), vec![end()]].concat(),
            _ => unreachable!("unknown case"),
        }
    }

    /// `\settowidth` splits the box at font/size switches and measures
    /// each run in its own font: the declaration cases differ from the
    /// plain outer-font measure, and the argument cases pick up the inner
    /// font plus the trailing outer-font text. Before the fix every case
    /// measured the whole content in the outer font, so `bfseries` equalled
    /// `plain` (10.0pt vs pdflatex's 12.19438pt).
    #[test]
    fn settowidth_splits_runs_at_font_switches() {
        let m = metrics();
        let outer: u32 = 0;
        let cases: &[(&str, i64)] = &[
            ("plain", expected("Hi", 10.0, Font::TimesRoman)),
            ("bfseries", expected("Hi", 10.0, Font::TimesBold)),
            (
                "textbf",
                expected("Hi", 10.0, Font::TimesBold) + expected(" x", 10.0, Font::TimesRoman),
            ),
            ("itshape", expected("f", 10.0, Font::TimesItalic)),
            // `\large` under article 10pt is 12pt (size10.clo `\@xiipt`).
            ("large", expected("Hi", 12.0, Font::TimesRoman)),
            ("texttt", expected("Hi", 10.0, Font::Courier)),
        ];
        for (name, want) in cases {
            let got = m.width(outer, &case_tokens(name));
            assert_eq!(got, *want, "case {name}");
        }
        // The fixed bug, stated directly: the bold run must not measure
        // as the outer font.
        assert_ne!(
            m.width(outer, &case_tokens("bfseries")),
            m.width(outer, &case_tokens("plain")),
        );
    }

    /// Measure through the real expansion engine, so the token shapes the
    /// unit test above assumes (emitted `\textbf`, braces, chars) are what
    /// `\settowidth` actually hands `width()`.
    fn expand_settowidth_pt(content_latex: &str) -> f64 {
        let m = metrics();
        let source = format!("\\newdimen\\x\\settowidth\\x{{{content_latex}}}\\the\\x");
        let mut engine = tex::Engine::new(&source);
        for (name, switch) in font_switches() {
            engine.declare_font_switch(name, switch);
        }
        engine.set_font_metrics(std::rc::Rc::new(m));
        engine.set_box_measurer(std::rc::Rc::new(m));
        let tokens = engine.run();
        assert!(
            engine.take_diagnostics().is_empty(),
            "expanding {source} must be diagnostic-free"
        );
        let printed: String = tokens
            .iter()
            .filter_map(|tok| match &tok.kind {
                tex::TokenKind::Char(ch, _) => Some(*ch),
                _ => None,
            })
            .collect();
        printed
            .strip_suffix("pt")
            .unwrap_or_else(|| panic!("`\\the\\x` must print a dimen, got {printed:?}"))
            .parse()
            .unwrap_or_else(|_| panic!("unparsable dimen {printed:?}"))
    }

    #[test]
    fn settowidth_end_to_end_matches_split_runs() {
        let m = metrics();
        let outer: u32 = 0;
        let cases = [
            ("plain", "Hi"),
            ("bfseries", "{\\bfseries Hi}"),
            ("textbf", "\\textbf{Hi} x"),
            ("itshape", "{\\itshape f}"),
            ("large", "\\large Hi"),
            ("texttt", "\\texttt{Hi}"),
        ];
        for (name, latex) in cases {
            let direct_pt = (m.width(outer, &case_tokens(name)) as f64) / 65536.0;
            let engine_pt = expand_settowidth_pt(latex);
            // `\the` prints the register to 5 decimal places at most, so
            // allow a couple of scaled points of print rounding.
            assert!(
                (engine_pt - direct_pt).abs() < 1e-4,
                "case {name}: engine {engine_pt}pt vs direct {direct_pt}pt"
            );
        }
    }

    /// The pdflatex oracle for the six cases (TeX Live 2026, article 10pt,
    /// `\showthe\wd`/`\the` in pt). Our runs shape with Core 14 AFM
    /// advances while pdflatex uses the TFM fonts, so this asserts a loose
    /// approximation bound documenting that gap (issue #1063), not
    /// exactness: the exact per-run pin is `settowidth_splits_runs_at_font_switches`.
    #[test]
    fn settowidth_approximates_pdflatex_oracle() {
        let m = metrics();
        let outer: u32 = 0;
        let oracle: &[(&str, f64)] = &[
            ("plain", 10.2778),
            ("bfseries", 12.19438),
            ("textbf", 20.80551),
            ("itshape", 3.06665),
            ("large", 12.0721),
            ("texttt", 10.49991),
        ];
        for (name, oracle_pt) in oracle {
            let ours_pt = (m.width(outer, &case_tokens(name)) as f64) / 65536.0;
            let rel = (ours_pt - oracle_pt).abs() / oracle_pt;
            assert!(
                rel < 0.20,
                "case {name}: ours {ours_pt}pt vs pdflatex {oracle_pt}pt"
            );
        }
    }

    /// The article-12pt measurer: `\normalsize` is 12pt.
    fn metrics_12() -> EngineFontMetrics {
        EngineFontMetrics {
            setup: FontSetup::new(Some(12.0), false, false),
            preamble_latin_modern: false,
        }
    }

    /// The height/depth run-splitting cases as the engine emits them: a
    /// bare declaration switches the running font from that token on, and
    /// `A{\Large A}` wraps the switched run in a group (cf. `case_tokens`).
    fn vertical_tokens(content: &str) -> Vec<tex::Token> {
        match content {
            "cap" => word("A"),
            "large-cap" => [vec![cs("Large")], word("A")].concat(),
            "mixed-cap" => {
                [word("A"), vec![begin(), cs("Large")], word("A"), vec![end()]].concat()
            }
            "bf-cap" => [vec![cs("textbf"), begin()], word("H"), vec![end()]].concat(),
            "bf-hi" => [vec![cs("textbf"), begin()], word("Hi"), vec![end()]].concat(),
            "bf-gy" => [vec![cs("textbf"), begin()], word("gy"), vec![end()]].concat(),
            "ag" => word("Ag"),
            "x-only" => word("a"),
            "ascender" => word("b"),
            "desc" => word("g"),
            "large-desc" => [vec![cs("Large")], word("g")].concat(),
            "mixed-desc" => {
                [word("g"), vec![begin(), cs("Large")], word("g"), vec![end()]].concat()
            }
            _ => unreachable!("unknown case"),
        }
    }

    /// `\settoheight` splits the box into font runs like `\settowidth`
    /// does and takes the tallest run's height in its own font and size.
    /// Expected values pin the Core 14 AFM declarations (Times-Roman cap
    /// 662 / x 450 / ascender 683, Times-Bold cap 676, all at units/1000):
    /// `\Large` under article 10pt is 14.4pt, under 12pt it is 17.28pt.
    /// Before the fix every case measured in the outer font, so
    /// `large-cap` gave cap height at 10pt (6.62pt) where pdflatex reports
    /// 9.84pt.
    #[test]
    fn settoheight_splits_runs_at_size_switches() {
        let m = metrics();
        let outer: u32 = 0;
        let cases: &[(&str, i64)] = &[
            ("cap", sp(0.662 * 10.0)),
            ("large-cap", sp(0.662 * 14.4)),
            ("mixed-cap", sp(0.662 * 14.4)),
            // Bold `H` reaches the bold face's own cap height (676);
            // bold `Hi` is taller still via `i`'s ascender class (683).
            ("bf-cap", sp(0.676 * 10.0)),
            ("bf-hi", sp(0.683 * 10.0)),
            ("ag", sp(0.662 * 10.0)),
            // Lower-case classes in the outer font: x-height, ascender.
            ("x-only", sp(0.450 * 10.0)),
            ("ascender", sp(0.683 * 10.0)),
        ];
        for (name, want) in cases {
            let got = m.height(outer, &vertical_tokens(name));
            assert_eq!(got, *want, "case {name}");
        }
        // The fixed bug, stated directly: the `\Large` run must not
        // measure as the outer font, and the mixed box takes the max.
        assert_ne!(
            m.height(outer, &vertical_tokens("large-cap")),
            m.height(outer, &vertical_tokens("cap")),
        );
        assert_eq!(
            m.height(outer, &vertical_tokens("mixed-cap")),
            m.height(outer, &vertical_tokens("large-cap")),
        );
    }

    /// `\settodepth` splits the box into font runs and takes the deepest
    /// run's descender in its own font and size (Times descender is
    /// 217/1000). Content with no descender glyph has no depth.
    #[test]
    fn settodepth_splits_runs_at_size_switches() {
        let m = metrics();
        let outer: u32 = 0;
        let cases: &[(&str, i64)] = &[
            ("desc", sp(0.217 * 10.0)),
            ("large-desc", sp(0.217 * 14.4)),
            ("mixed-desc", sp(0.217 * 14.4)),
            ("bf-gy", sp(0.217 * 10.0)),
            ("ag", sp(0.217 * 10.0)),
            ("cap", 0),
        ];
        for (name, want) in cases {
            let got = m.depth(outer, &vertical_tokens(name));
            assert_eq!(got, *want, "case {name}");
        }
        assert_ne!(
            m.depth(outer, &vertical_tokens("large-desc")),
            m.depth(outer, &vertical_tokens("desc")),
        );
        assert_eq!(
            m.depth(outer, &vertical_tokens("mixed-desc")),
            m.depth(outer, &vertical_tokens("large-desc")),
        );
    }

    /// The 12pt-class pins: `\normalsize` is 12pt and `\Large` is 17.28pt
    /// (`size12.clo`), so the runs measure at those sizes.
    #[test]
    fn settoheight_depth_split_runs_at_12pt() {
        let m = metrics_12();
        let outer: u32 = 0;
        assert_eq!(m.height(outer, &vertical_tokens("cap")), sp(0.662 * 12.0));
        assert_eq!(
            m.height(outer, &vertical_tokens("large-cap")),
            sp(0.662 * 17.28)
        );
        assert_eq!(
            m.height(outer, &vertical_tokens("mixed-cap")),
            m.height(outer, &vertical_tokens("large-cap")),
        );
        assert_eq!(
            m.depth(outer, &vertical_tokens("mixed-desc")),
            sp(0.217 * 17.28)
        );
    }

    /// The pdflatex oracles for the new cases (TeX Live 2026, article,
    /// `\the` of the length register after `\settoheight`/`\settodepth`):
    /// 10pt `{\Large A}` height 9.84pt, `A{\Large A}` height 9.84pt, `Ag`
    /// height 6.83331pt, `\textbf{Hi}` height 6.94444pt, `{\Large g}`
    /// depth 2.79999pt, `g{\Large g}` depth 2.79999pt, `Ag` depth
    /// 1.94444pt, `\textbf{gy}` depth 1.94444pt; 12pt `{\Large A}` height
    /// 11.80556pt, `Ag` height 8.2pt, `a` height 5.16667pt, `{\Large g}`
    /// depth 3.3611pt, `Ag` depth 2.33331pt. Each case carries its own
    /// bound from its measured Core-14-vs-Computer-Modern gap: heights
    /// agree within 5%, depths within 15% (the Core 14 descender runs
    /// deeper than Computer Modern's).
    #[test]
    fn settoheight_depth_approximate_pdflatex_oracle() {
        let m = metrics();
        let m12 = metrics_12();
        let outer: u32 = 0;
        // (label, 12pt measurer, case tokens, oracle pt, relative bound).
        let oracle: &[(&str, bool, &str, f64, f64)] = &[
            ("10pt height", false, "large-cap", 9.84, 0.05),
            ("10pt height", false, "mixed-cap", 9.84, 0.05),
            ("10pt height", false, "ag", 6.83331, 0.05),
            ("10pt height", false, "bf-hi", 6.94444, 0.05),
            ("10pt depth", false, "large-desc", 2.79999, 0.15),
            ("10pt depth", false, "mixed-desc", 2.79999, 0.15),
            ("10pt depth", false, "ag", 1.94444, 0.15),
            ("10pt depth", false, "bf-gy", 1.94444, 0.15),
            ("12pt height", true, "large-cap", 11.80556, 0.05),
            ("12pt height", true, "ag", 8.2, 0.05),
            ("12pt height", true, "x-only", 5.16667, 0.06),
            ("12pt depth", true, "large-desc", 3.3611, 0.15),
            ("12pt depth", true, "ag", 2.33331, 0.15),
        ];
        for (label, twelve, name, oracle_pt, bound) in oracle {
            let measurer = if *twelve { &m12 } else { &m };
            let ours_sp = if label.ends_with("height") {
                measurer.height(outer, &vertical_tokens(name))
            } else {
                measurer.depth(outer, &vertical_tokens(name))
            };
            let ours_pt = (ours_sp as f64) / 65536.0;
            let rel = (ours_pt - oracle_pt).abs() / oracle_pt;
            assert!(
                rel < *bound,
                "{label} case {name}: ours {ours_pt}pt vs pdflatex {oracle_pt}pt (bound {bound})"
            );
        }
        // The headline case from the report: `\settoheight{\x}{\Large A}`
        // at 10pt must read ~9.84pt, not the old 6.62pt.
        let large_pt = (m.height(outer, &vertical_tokens("large-cap")) as f64) / 65536.0;
        assert!(
            (large_pt - 9.84).abs() < 0.5,
            "headline case drifted: {large_pt}pt vs pdflatex 9.84pt"
        );
    }

    /// Measure `\settoheight` through the real expansion engine, so the
    /// token shapes `vertical_tokens` assumes are what the primitive
    /// actually hands `height()`.
    fn expand_settoheight_pt(content_latex: &str) -> f64 {
        let m = metrics();
        let source = format!("\\newdimen\\x\\settoheight\\x{{{content_latex}}}\\the\\x");
        let mut engine = tex::Engine::new(&source);
        for (name, switch) in font_switches() {
            engine.declare_font_switch(name, switch);
        }
        engine.set_font_metrics(std::rc::Rc::new(m));
        engine.set_box_measurer(std::rc::Rc::new(m));
        let tokens = engine.run();
        assert!(
            engine.take_diagnostics().is_empty(),
            "expanding {source} must be diagnostic-free"
        );
        let printed: String = tokens
            .iter()
            .filter_map(|tok| match &tok.kind {
                tex::TokenKind::Char(ch, _) => Some(*ch),
                _ => None,
            })
            .collect();
        printed
            .strip_suffix("pt")
            .unwrap_or_else(|| panic!("`\\the\\x` must print a dimen, got {printed:?}"))
            .parse()
            .unwrap_or_else(|_| panic!("unparsable dimen {printed:?}"))
    }

    #[test]
    fn settoheight_end_to_end_matches_split_runs() {
        let m = metrics();
        let outer: u32 = 0;
        let cases = [
            ("large-cap", "\\Large A"),
            ("mixed-cap", "A{\\Large A}"),
        ];
        for (name, latex) in cases {
            let direct_pt = (m.height(outer, &vertical_tokens(name)) as f64) / 65536.0;
            let engine_pt = expand_settoheight_pt(latex);
            assert!(
                (engine_pt - direct_pt).abs() < 1e-4,
                "case {name}: engine {engine_pt}pt vs direct {direct_pt}pt"
            );
        }
    }
}
