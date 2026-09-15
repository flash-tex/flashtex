//! TeX's font-relative units for the text font a style selects.
//!
//! `em` is the current font's `\fontdimen6` (quad) and `ex` its
//! `\fontdimen5` (x-height), not the point size: cmr12's quad is 11.74988pt,
//! so `3em` under `\documentclass[12pt]` is 35.24963pt, and `\small`,
//! `\bfseries`, `\sffamily`, `\ttfamily`, italics, `fontenc` T1 and `lmodern`
//! each load another TFM with other values. [`crate::text_fontdimens`] records
//! them per NFSS font from pdflatex. This is independent of the compiler's own
//! Core14 layout faces: these are the values `\the`/`\showthe` report.

use crate::parser::{FontSizeLevel, TextFamily, TextStyle};
use crate::text_fontdimens::{row, FONTDIMENS, SIZES_PT};
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
        let cells = &FONTDIMENS[row(self.latin_modern, self.t1, family, style.bold, style.italic)];
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
        clear: 0,
        set: 0,
        toggle: ITALIC,
        argument: false,
    };
    let mut switches = vec![
        ("bfseries", switch(BOLD, BOLD)),
        ("textbf", argument(switch(BOLD, BOLD))),
        ("mdseries", switch(BOLD, 0)),
        ("textmd", argument(switch(BOLD, 0))),
        ("itshape", switch(ITALIC, ITALIC)),
        ("slshape", switch(ITALIC, ITALIC)),
        ("textit", argument(switch(ITALIC, ITALIC))),
        ("textsl", argument(switch(ITALIC, ITALIC))),
        ("upshape", switch(ITALIC, 0)),
        ("textup", argument(switch(ITALIC, 0))),
        ("em", emph),
        ("emph", argument(emph)),
        ("rmfamily", switch(FAMILY, 0)),
        ("textrm", argument(switch(FAMILY, 0))),
        ("sffamily", switch(FAMILY, SANS)),
        ("textsf", argument(switch(FAMILY, SANS))),
        ("ttfamily", switch(FAMILY, MONO)),
        ("texttt", argument(switch(FAMILY, MONO))),
        // `\normalfont` keeps the size.
        ("normalfont", switch(BOLD | ITALIC | FAMILY, 0)),
        ("textnormal", argument(switch(BOLD | ITALIC | FAMILY, 0))),
        // LaTeX 2.09 forms are `\normalfont` plus one attribute.
        ("rm", switch(BOLD | ITALIC | FAMILY, 0)),
        ("sf", switch(BOLD | ITALIC | FAMILY, SANS)),
        ("tt", switch(BOLD | ITALIC | FAMILY, MONO)),
        ("bf", switch(BOLD | ITALIC | FAMILY, BOLD)),
        ("it", switch(BOLD | ITALIC | FAMILY, ITALIC)),
        ("sl", switch(BOLD | ITALIC | FAMILY, ITALIC)),
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
pub(crate) struct EngineFontMetrics {
    pub setup: FontSetup,
    pub preamble_latin_modern: bool,
}

impl EngineFontMetrics {
    fn em_ex_sp(&self, font: u32) -> (i64, i64) {
        let level = (font & SIZE) as usize;
        let style = TextStyle {
            bold: font & BOLD != 0,
            italic: font & ITALIC != 0,
            family: match font & FAMILY {
                SANS => TextFamily::Sans,
                MONO => TextFamily::Mono,
                _ => TextFamily::Roman,
            },
            size: level
                .checked_sub(1)
                .and_then(|i| SIZE_LEVELS.get(i).copied()),
            color: None,
        };
        let latin_modern = if font & BODY != 0 {
            self.setup.latin_modern
        } else {
            self.preamble_latin_modern
        };
        FontSetup {
            latin_modern,
            ..self.setup
        }
        .em_ex_sp(style)
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
