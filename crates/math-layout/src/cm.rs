//! Computer Modern metrics adapter.
//!
//! Provides TeX's own math parameters and glyph metrics from the embedded TFM
//! tables ([`crate::cm_tfm`]), so the engine reproduces plain TeX / LaTeX
//! geometry when the renderer draws the same Computer Modern fonts.
//!
//! Provenance of the parameters: `\fontdimen` 5–22 of `cmsy10/7/5.tfm`
//! (family 2, the TeXbook's σ parameters) and `\fontdimen` 8–13 of
//! `cmex10.tfm` (family 3, the ξ parameters); see TeXbook Appendix G and
//! `plain.tex` (`\textfont2=\tensy`, `\scriptfont2=\sevensy`,
//! `\scriptscriptfont2=\fivesy`, `\textfont3=\tenex`). Fixed TeX registers use
//! the plain.tex / LaTeX values: `\scriptspace=0.5pt`,
//! `\nulldelimiterspace=1.2pt`, `\delimiterfactor=901`,
//! `\delimitershortfall=5pt`.
//!
//! Unicode symbols are mapped to CM families and codes following the
//! `\mathcode`/`\mathchardef`/`\delcode` assignments of `plain.tex`
//! (LaTeX's `fontmath.ltx` uses the same slots for these symbols).

use crate::cm_tfm::*;
use crate::metrics::{Extensible, FontId, Glyph, MathFontMetrics, MathParams, SizeClass};
use crate::tfm::{TfmChar, TfmFont, scale};

/// Family 0: roman (`cmr`), 1: math italic (`cmmi`), 2: symbols (`cmsy`),
/// 3: extension (`cmex`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Roman,
    Italic,
    Symbol,
    Extension,
}

/// How family 3 (`cmex`) is sized at script sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionSizing {
    /// plain.tex (`\scriptfont3=\tenex`) and LaTeX (`omxcmex.fd` declares
    /// `<->sfixed*cmex10`): family 3 stays at its 10pt design size in every
    /// style, so rule thickness is 0.4pt even inside scripts. Verified
    /// against pdfTeX `\showbox` output (see `docs/comparison.md`).
    Fixed,
    /// `cmex10` scaled to the script sizes (7pt, 5pt), so rule thickness and
    /// big-operator spacing shrink with the style. Approximates `amsfonts`,
    /// which loads real `cmex7`/`cmex8`/`cmex9` fonts at smaller sizes; see
    /// [`ExtensionSizing::Designs`] for the exact declaration.
    Scaled,
    /// `amsmath.sty` (default `cmex7` option, lines 109-114:
    /// `<-8>cmex7<8>cmex8<9>cmex9<10><10.95>..cmex10`) and `amsfonts.sty`
    /// (lines 36-41: `<-7.5>cmex7<7.5-8.5>cmex8<8.5-9.5>cmex9<9.5->cmex10`)
    /// redeclare `OMX/cmex/m/n` without `sfixed`: family 3 is loaded at the
    /// math size itself, in the design [`extension_design`] names. A
    /// Computer Modern document loading amsmath or amssymb gets this (pdfTeX
    /// `\fontname\textfont3` in an 11pt article: `cmex10 at 10.95pt`,
    /// `\scriptfont3` `cmex8`, `\scriptscriptfont3` `cmex7 at 6.0pt`).
    /// `lmodern` keeps `omxlmex.fd`'s `sfixed*lmex10` ([`ExtensionSizing::Fixed`]).
    Designs,
}

/// Every embedded font, indexed by `FontId`.
///
/// New designs are appended so that existing ids stay stable.
pub static ALL_FONTS: [&TfmFont; 25] = [
    &CMR10, &CMR7, &CMR5, &CMMI10, &CMMI7, &CMMI5, &CMSY10, &CMSY7, &CMSY5, &CMEX10, &CMR12, &CMR8,
    &CMR6, &CMMI12, &CMMI8, &CMMI6, &CMSY8, &CMSY6, &CMR9, &CMR17, &CMMI9, &CMSY9, &CMEX7, &CMEX8,
    &CMEX9,
];

/// The embedded TFM called `name` (`"cmmi9"`), if any.
pub fn tfm_by_name(name: &str) -> Option<&'static TfmFont> {
    ALL_FONTS.iter().copied().find(|f| f.name == name)
}

/// The math sizes `[text, script, scriptscript]` LaTeX uses at a text size.
///
/// `fontmath.ltx` lines 75-86: `\DeclareMathSizes{5}{5}{5}{5}`, `{6}{6}{5}{5}`,
/// `{7}{7}{5}{5}`, `{8}{8}{6}{5}`, `{9}{9}{6}{5}`, `{10}{10}{7}{5}`,
/// `{10.95}{10.95}{8}{6}`, `{12}{12}{8}{6}`, `{14.4}{14.4}{10}{7}`,
/// `{17.28}{17.28}{12}{10}`, `{20.74}{20.74}{14.4}{12}`,
/// `{24.88}{24.88}{20.74}{17.28}`. `size10.clo`/`size11.clo`/`size12.clo`
/// only choose which of these a size command selects (10pt class:
/// `\footnotesize` 8, `\small` 9, `\Large` 14.4; 11pt: 9, 10, 14.4; 12pt:
/// 10, 10.95, 17.28). Any other size gets `\calculate@math@sizes`
/// (latex.ltx 10742-10754): `\defaultscriptratio` .7 and
/// `\defaultscriptscriptratio` .5 of the text size, multiplied as TeX does
/// (the factor in 2^-16 units, truncated to the scaled point).
pub fn declare_math_sizes(text_pt: f64) -> [f64; 3] {
    const TABLE: [[f64; 3]; 12] = [
        [5.0, 5.0, 5.0],
        [6.0, 5.0, 5.0],
        [7.0, 5.0, 5.0],
        [8.0, 6.0, 5.0],
        [9.0, 6.0, 5.0],
        [10.0, 7.0, 5.0],
        [10.95, 8.0, 6.0],
        [12.0, 8.0, 6.0],
        [14.4, 10.0, 7.0],
        [17.28, 12.0, 10.0],
        [20.74, 14.4, 12.0],
        [24.88, 20.74, 17.28],
    ];
    if let Some(row) = TABLE.iter().find(|row| (row[0] - text_pt).abs() < 0.005) {
        return *row;
    }
    let sp = (text_pt * 65536.0).round() as i64;
    let scaled = |f: i64| ((sp * f) >> 16) as f64 / 65536.0;
    [text_pt, scaled(45875), scaled(32768)]
}

/// The design of `family` that LaTeX loads at `size_pt`.
///
/// Computer Modern (`ot1cmr.fd`, `omlcmm.fd`, `omscmsy.fd`) lists exact
/// sizes: cmr `<5>..<9><10><12>gen*cmr <10.95>cmr10 <14.4>cmr12
/// <17.28><20.74><24.88>cmr17`, cmmi `<5>..<9>gen*cmmi <10><10.95>cmmi10
/// <12>..<24.88>cmmi12`, cmsy `<5>..<10>gen*cmsy <10.95>..<24.88>cmsy10`.
/// Latin Modern (`ot1lmr.fd`, `omllmm.fd`, `omslmsy.fd`) uses ranges that
/// select the same design at every one of those sizes and are used here for
/// the rest: roman `<-5.5>5 <5.5-6.5>6 <6.5-7.5>7 <7.5-8.5>8 <8.5-9.5>9
/// <9.5-11>10 <11-15>12 <15->17`, math italic the same up to `<9.5-11>10
/// <11->12`, symbols up to `<9.5->10` (`lmr`/`lmmi`/`lmsy` are metric
/// copies of the CM designs). Family 3 is [`extension_design`]'s.
pub fn design_for(family: Family, size_pt: f64) -> &'static TfmFont {
    let s = size_pt;
    match family {
        Family::Roman => match s {
            s if s < 5.5 => &CMR5,
            s if s < 6.5 => &CMR6,
            s if s < 7.5 => &CMR7,
            s if s < 8.5 => &CMR8,
            s if s < 9.5 => &CMR9,
            s if s < 11.0 => &CMR10,
            s if s < 15.0 => &CMR12,
            _ => &CMR17,
        },
        Family::Italic => match s {
            s if s < 5.5 => &CMMI5,
            s if s < 6.5 => &CMMI6,
            s if s < 7.5 => &CMMI7,
            s if s < 8.5 => &CMMI8,
            s if s < 9.5 => &CMMI9,
            s if s < 11.0 => &CMMI10,
            _ => &CMMI12,
        },
        Family::Symbol => match s {
            s if s < 5.5 => &CMSY5,
            s if s < 6.5 => &CMSY6,
            s if s < 7.5 => &CMSY7,
            s if s < 8.5 => &CMSY8,
            s if s < 9.5 => &CMSY9,
            _ => &CMSY10,
        },
        Family::Extension => extension_design(s),
    }
}

/// The `cmex` design amsmath/amsfonts load at `size_pt`
/// ([`ExtensionSizing::Designs`]): cmex7 below 7.5pt, cmex8 below 8.5pt,
/// cmex9 below 9.5pt, else cmex10 (amsfonts' ranges; amsmath's exact sizes
/// agree at every declared size).
pub fn extension_design(size_pt: f64) -> &'static TfmFont {
    match size_pt {
        s if s < 7.5 => &CMEX7,
        s if s < 8.5 => &CMEX8,
        s if s < 9.5 => &CMEX9,
        _ => &CMEX10,
    }
}

fn font_id_of(font: &'static TfmFont) -> FontId {
    let i = ALL_FONTS
        .iter()
        .position(|f| std::ptr::eq(*f, font))
        .expect("font is in ALL_FONTS");
    FontId(i as u32)
}

#[derive(Debug, Clone)]
pub struct CmMathMetrics {
    /// Font sizes for text, script, scriptscript.
    pub sizes: [f64; 3],
    pub extension: ExtensionSizing,
    /// Fonts for families 0–2 (roman, math italic, symbols) at the three sizes.
    pub families: [[&'static TfmFont; 3]; 3],
}

impl CmMathMetrics {
    /// LaTeX 10pt (and plain TeX): 10/7/5 pt (`\DeclareMathSizes{10}{10}{7}{5}`,
    /// fonts cmr/cmmi/cmsy 10/7/5) with family 3 fixed at 10pt.
    pub fn latex_10pt() -> CmMathMetrics {
        CmMathMetrics {
            sizes: [10.0, 7.0, 5.0],
            extension: ExtensionSizing::Fixed,
            families: [
                [&CMR10, &CMR7, &CMR5],
                [&CMMI10, &CMMI7, &CMMI5],
                [&CMSY10, &CMSY7, &CMSY5],
            ],
        }
    }

    /// LaTeX 12pt: `\DeclareMathSizes{12}{12}{8}{6}` with the fonts the
    /// standard `.fd` files load — cmr12/cmmi12 and cmsy10 scaled to 12pt for
    /// text, cmr8/cmmi8/cmsy8 for scripts, cmr6/cmmi6/cmsy6 for scriptscripts,
    /// and cmex10 fixed at 10pt (`omxcmex.fd`). `lmodern` uses the same
    /// metrics under the Latin Modern names.
    pub fn latex_12pt() -> CmMathMetrics {
        CmMathMetrics {
            sizes: [12.0, 8.0, 6.0],
            extension: ExtensionSizing::Fixed,
            families: [
                [&CMR12, &CMR8, &CMR6],
                [&CMMI12, &CMMI8, &CMMI6],
                [&CMSY10, &CMSY8, &CMSY6],
            ],
        }
    }

    /// The math fonts LaTeX selects when the current text size is `text_pt`
    /// (`\normalsize` of a class, `\footnotesize` in a footnote, `\Large` in
    /// a `\section` title): [`declare_math_sizes`] with [`design_for`] each
    /// family at each size, and family 3 fixed at cmex10 (the kernel's and
    /// lmodern's `sfixed`). A Computer Modern document with amsmath or
    /// amsfonts uses [`CmMathMetrics::with_extension`]`(Designs)`.
    /// `for_text_size(10.0)` and `for_text_size(12.0)` equal
    /// [`CmMathMetrics::latex_10pt`] and [`CmMathMetrics::latex_12pt`].
    pub fn for_text_size(text_pt: f64) -> CmMathMetrics {
        let sizes = declare_math_sizes(text_pt);
        let row = |family: Family| sizes.map(|s| design_for(family, s));
        CmMathMetrics {
            sizes,
            extension: ExtensionSizing::Fixed,
            families: [row(Family::Roman), row(Family::Italic), row(Family::Symbol)],
        }
    }

    /// `self` with family 3 sized as `extension`.
    pub fn with_extension(self, extension: ExtensionSizing) -> CmMathMetrics {
        CmMathMetrics { extension, ..self }
    }

    /// Alias of [`CmMathMetrics::latex_10pt`]: plain TeX uses the same sizes.
    pub fn plain() -> CmMathMetrics {
        CmMathMetrics::latex_10pt()
    }

    /// Scales the 10/7/5 pt design proportionally to another base size.
    /// This is an approximation: real LaTeX uses `cmr8`/`cmr6` at 11–12pt.
    /// Prefer [`CmMathMetrics::latex_12pt`] for 12pt documents.
    pub fn scaled(base: f64) -> CmMathMetrics {
        let k = base / 10.0;
        CmMathMetrics {
            sizes: [10.0 * k, 7.0 * k, 5.0 * k],
            ..CmMathMetrics::latex_10pt()
        }
    }

    fn size_index(size: SizeClass) -> usize {
        match size {
            SizeClass::Text => 0,
            SizeClass::Script => 1,
            SizeClass::ScriptScript => 2,
        }
    }

    fn size_pt(&self, size: SizeClass) -> f64 {
        self.sizes[Self::size_index(size)]
    }

    fn font(&self, family: Family, size: SizeClass) -> (&'static TfmFont, FontId, f64) {
        let i = Self::size_index(size);
        let fam = match family {
            Family::Roman => 0,
            Family::Italic => 1,
            Family::Symbol => 2,
            Family::Extension => {
                let (font, at) = match self.extension {
                    // cmex10 is `sfixed` at its 10pt design size in LaTeX.
                    ExtensionSizing::Fixed => (&CMEX10, CMEX10.design_size),
                    ExtensionSizing::Scaled => (&CMEX10, self.sizes[i]),
                    ExtensionSizing::Designs => (extension_design(self.sizes[i]), self.sizes[i]),
                };
                return (font, font_id_of(font), at);
            }
        };
        let font = self.families[fam][i];
        (font, font_id_of(font), self.sizes[i])
    }

    /// The roman text font at text size with its interword space (fontdimen
    /// 2), for setting words of a mixed text/math line.
    pub fn text_space(&self) -> f64 {
        let (font, _, at) = self.font(Family::Roman, SizeClass::Text);
        font.fontdimen(2, at)
    }

    fn make_glyph(&self, family: Family, code: u8, ch: char, size: SizeClass) -> Option<Glyph> {
        let (font, font_id, at) = self.font(family, size);
        let c = font.char(code)?;
        Some(glyph_from(c, font_id, ch, at))
    }

    /// The extensible recipe at the end of the family-3 chain from `code`.
    fn extension_recipe(&self, code: u8, ch: char, size: SizeClass) -> Option<Extensible> {
        let (font, font_id, at) = self.font(Family::Extension, size);
        let mut cur = font.char(code)?;
        let mut guard = 0;
        while !font.is_extensible(cur) {
            cur = font.next_larger(cur)?;
            guard += 1;
            if guard > 8 {
                return None;
            }
        }
        // TFM order: top, mid, bot, rep; 0 means "no piece".
        let piece = |c: u8| -> Option<Glyph> {
            if c == 0 || c == u8::MAX {
                None
            } else {
                font.char(c).map(|p| glyph_from(p, font_id, ch, at))
            }
        };
        let [top, mid, bot, rep] = cur.extensible;
        Some(Extensible {
            top: piece(top),
            mid: piece(mid),
            bot: piece(bot),
            rep: piece(rep)?,
        })
    }

    /// Follows the `next_larger` chain in family 3 starting at `code`,
    /// stopping before the extensible recipe.
    fn extension_chain(&self, code: u8, ch: char, size: SizeClass, out: &mut Vec<Glyph>) {
        let (font, font_id, at) = self.font(Family::Extension, size);
        let mut cur = font.char(code);
        let mut guard = 0;
        while let Some(c) = cur {
            if font.is_extensible(c) {
                break;
            }
            out.push(glyph_from(c, font_id, ch, at));
            cur = font.next_larger(c);
            guard += 1;
            if guard > 8 {
                break;
            }
        }
    }
}

fn glyph_from(c: &TfmChar, font_id: FontId, ch: char, at: f64) -> Glyph {
    Glyph {
        font_id,
        gid: c.code as u16,
        ch,
        size: at,
        width: scale(c.width, at),
        height: scale(c.height, at),
        depth: scale(c.depth, at),
        italic: scale(c.italic, at),
        skew: scale(c.skew_kern, at),
    }
}

/// plain.tex `\mathcode` / `\mathchardef` slot of a symbol.
pub fn symbol_slot(ch: char) -> Option<(Family, u8)> {
    use Family::*;
    Some(match ch {
        'a'..='z' | 'A'..='Z' => (Italic, ch as u8),
        '0'..='9' => (Roman, ch as u8),
        '(' | ')' | '[' | ']' | '!' | '?' | ';' | ':' | '+' | '=' | '@' => (Roman, ch as u8),
        // plain.tex: \mathcode`\,="613B, `\.="013A, `\/="013D, `\<="313C, `\>="313E.
        ',' => (Italic, 0x3B),
        '.' => (Italic, 0x3A),
        '/' => (Italic, 0x3D),
        '<' | '>' => (Italic, ch as u8),
        '-' | '\u{2212}' => (Symbol, 0x00),
        '\u{22C5}' | '\u{00B7}' => (Symbol, 0x01),
        '\u{00D7}' => (Symbol, 0x02),
        '\u{2217}' => (Symbol, 0x03),
        '\u{00F7}' => (Symbol, 0x04),
        '\u{00B1}' => (Symbol, 0x06),
        '\u{2213}' => (Symbol, 0x07),
        '\u{2218}' => (Symbol, 0x0E),
        '\u{2261}' => (Symbol, 0x11),
        '\u{2286}' => (Symbol, 0x12),
        '\u{2287}' => (Symbol, 0x13),
        '\u{2264}' => (Symbol, 0x14),
        '\u{2265}' => (Symbol, 0x15),
        '\u{223C}' => (Symbol, 0x18),
        '\u{2248}' => (Symbol, 0x19),
        '\u{2282}' => (Symbol, 0x1A),
        '\u{2283}' => (Symbol, 0x1B),
        '\u{2190}' => (Symbol, 0x20),
        '\u{2192}' => (Symbol, 0x21),
        '\u{2191}' => (Symbol, 0x22),
        '\u{2193}' => (Symbol, 0x23),
        '\u{2194}' => (Symbol, 0x24),
        '\u{21D0}' => (Symbol, 0x28),
        '\u{21D2}' => (Symbol, 0x29),
        '\u{21D4}' => (Symbol, 0x2C),
        '\u{221E}' => (Symbol, 0x31),
        '\u{2208}' => (Symbol, 0x32),
        '\u{220B}' => (Symbol, 0x33),
        '\u{2200}' => (Symbol, 0x38),
        '\u{2203}' => (Symbol, 0x39),
        '\u{00AC}' => (Symbol, 0x3A),
        '\u{2205}' => (Symbol, 0x3B),
        '\u{2207}' => (Symbol, 0x72),
        // fontmath.ltx 224: `\DeclareMathSymbol{\prime}{\mathord}{symbols}{"30}`,
        // the large prime that `'` (`^\prime`, latex.ltx 15683) sets in scripts.
        '\u{2032}' => (Symbol, 0x30),
        '\u{2202}' => (Italic, 0x40),
        '\u{2227}' => (Symbol, 0x5E),
        '\u{2228}' => (Symbol, 0x5F),
        '\u{2229}' => (Symbol, 0x5C),
        '\u{222A}' => (Symbol, 0x5B),
        // The square relations, cmsy "74-"77. These are base LaTeX2e kernel
        // symbols, not amssymb: `fontmath.ltx` 279/278 declare `\sqcup`/
        // `\sqcap` `\mathbin` at symbols "74/"75 and 301/302 `\sqsubseteq`/
        // `\sqsupseteq` `\mathrel` at "76/"77. Confirmed with pdfTeX
        // 3.141592653 (TeX Live 2025), where `\show` gives \mathchar"2274,
        // "2275, "3276 and "3277 with and without amssymb loaded; at 10pt the
        // glyphs measure 6.66669pt (cmsy10 "74/"75) and 7.7778pt ("76/"77).
        // (amsfonts' strict `\sqsubset`/`\sqsupset` are msam, not cmsy, and
        // are not reachable through this table.)
        '\u{2294}' => (Symbol, 0x74),
        '\u{2293}' => (Symbol, 0x75),
        '\u{2291}' => (Symbol, 0x76),
        '\u{2292}' => (Symbol, 0x77),
        '\u{2295}' => (Symbol, 0x08),
        '\u{2297}' => (Symbol, 0x0A),
        '\u{2216}' => (Symbol, 0x6E),
        '\u{2223}' | '|' => (Symbol, 0x6A),
        '\u{2225}' | '\u{2016}' => (Symbol, 0x6B),
        '{' => (Symbol, 0x66),
        '}' => (Symbol, 0x67),
        '\u{27E8}' => (Symbol, 0x68),
        '\u{27E9}' => (Symbol, 0x69),
        '\u{230A}' => (Symbol, 0x62),
        '\u{230B}' => (Symbol, 0x63),
        '\u{2308}' => (Symbol, 0x64),
        '\u{2309}' => (Symbol, 0x65),
        '\\' => (Symbol, 0x6E),
        // Greek lower case: cmmi 0x0B..0x21 in the \alpha..\omega order.
        '\u{03B1}' => (Italic, 0x0B),
        '\u{03B2}' => (Italic, 0x0C),
        '\u{03B3}' => (Italic, 0x0D),
        '\u{03B4}' => (Italic, 0x0E),
        // TeX \varepsilon (cmmi "22, the open ε); \epsilon is the lunate U+03F5 below.
        '\u{03B5}' => (Italic, 0x22),
        '\u{03B6}' => (Italic, 0x10),
        '\u{03B7}' => (Italic, 0x11),
        '\u{03B8}' => (Italic, 0x12),
        '\u{03B9}' => (Italic, 0x13),
        '\u{03BA}' => (Italic, 0x14),
        '\u{03BB}' => (Italic, 0x15),
        '\u{03BC}' => (Italic, 0x16),
        '\u{03BD}' => (Italic, 0x17),
        '\u{03BE}' => (Italic, 0x18),
        '\u{03C0}' => (Italic, 0x19),
        '\u{03C1}' => (Italic, 0x1A),
        '\u{03C3}' => (Italic, 0x1B),
        '\u{03C4}' => (Italic, 0x1C),
        '\u{03C5}' => (Italic, 0x1D),
        '\u{03C6}' => (Italic, 0x1E),
        '\u{03C7}' => (Italic, 0x1F),
        '\u{03C8}' => (Italic, 0x20),
        '\u{03C9}' => (Italic, 0x21),
        '\u{03D1}' => (Italic, 0x23),
        '\u{03D6}' => (Italic, 0x24),
        '\u{03F1}' => (Italic, 0x25),
        '\u{03C2}' => (Italic, 0x26),
        '\u{03D5}' => (Italic, 0x27),
        '\u{03F5}' => (Italic, 0x0F),
        // Greek upper case: cmr 0x00..0x0A.
        '\u{0393}' => (Roman, 0x00),
        '\u{0394}' => (Roman, 0x01),
        '\u{0398}' => (Roman, 0x02),
        '\u{039B}' => (Roman, 0x03),
        '\u{039E}' => (Roman, 0x04),
        '\u{03A0}' => (Roman, 0x05),
        '\u{03A3}' => (Roman, 0x06),
        '\u{03A5}' => (Roman, 0x07),
        '\u{03A6}' => (Roman, 0x08),
        '\u{03A8}' => (Roman, 0x09),
        '\u{03A9}' => (Roman, 0x0A),
        // Large operators: text-size slots in cmex.
        '\u{2211}' => (Extension, 0x50),
        '\u{220F}' => (Extension, 0x51),
        '\u{222B}' => (Extension, 0x52),
        '\u{222E}' => (Extension, 0x48),
        '\u{22C3}' => (Extension, 0x53),
        '\u{22C2}' => (Extension, 0x54),
        '\u{2A01}' => (Extension, 0x4C),
        '\u{2A02}' => (Extension, 0x4E),
        '\u{2A00}' => (Extension, 0x4A),
        '\u{22C1}' => (Extension, 0x57),
        '\u{22C0}' => (Extension, 0x56),
        '\u{2210}' => (Extension, 0x60),
        // fontmath.ltx 262/250: \bigsqcup and \biguplus, the two
        // `largesymbols` \mathop operators the table was missing.
        '\u{2A06}' => (Extension, 0x46),
        '\u{2A04}' => (Extension, 0x55),
        // LaTeX kernel \DeclareMathSymbol rows from the `symbols` family
        // (cmsy), fontmath.ltx line in the comment.
        '\u{2296}' => (Symbol, 0x09), // \ominus 289
        '\u{2298}' => (Symbol, 0x0B), // \oslash 287
        '\u{2299}' => (Symbol, 0x0C), // \odot 286
        '\u{25EF}' => (Symbol, 0x0D), // \bigcirc 294
        '\u{2219}' => (Symbol, 0x0F), // \bullet 283
        '\u{22C4}' => (Symbol, 0x05), // \diamond 282
        '\u{224D}' => (Symbol, 0x10), // \asymp 346
        '\u{2AAF}' => (Symbol, 0x16), // \preceq 324
        '\u{2AB0}' => (Symbol, 0x17), // \succeq 323
        '\u{227A}' => (Symbol, 0x1E), // \prec 321
        '\u{227B}' => (Symbol, 0x1F), // \succ 320
        '\u{2197}' => (Symbol, 0x25), // \nearrow 307
        '\u{2198}' => (Symbol, 0x26), // \searrow 308
        '\u{2196}' => (Symbol, 0x2D), // \nwarrow 309
        '\u{2199}' => (Symbol, 0x2E), // \swarrow 310
        '\u{228E}' => (Symbol, 0x5D), // \uplus 280
        '\u{2240}' => (Symbol, 0x6F), // \wr 284
        '\u{221A}' => (Symbol, 0x70), // \surd 242 (\mathchar"1270, braced -> Ord)
        '\u{2A3F}' => (Symbol, 0x71), // \amalg 281
        '\u{2294}' => (Symbol, 0x74), // \sqcup 279
        '\u{2293}' => (Symbol, 0x75), // \sqcap 278
        '\u{2291}' => (Symbol, 0x76), // \sqsubseteq 301
        '\u{2292}' => (Symbol, 0x77), // \sqsupseteq 302
        '\u{00A7}' => (Symbol, 0x78), // \mathsection 508
        '\u{2020}' => (Symbol, 0x79), // \dagger 277
        '\u{2021}' => (Symbol, 0x7A), // \ddagger 276
        '\u{00B6}' => (Symbol, 0x7B), // \mathparagraph 507
        '\u{2663}' => (Symbol, 0x7C), // \clubsuit 237
        '\u{2662}' => (Symbol, 0x7D), // \diamondsuit 238
        '\u{2661}' => (Symbol, 0x7E), // \heartsuit 239
        '\u{2660}' => (Symbol, 0x7F), // \spadesuit 240
        // Kernel rows from the `letters` family (cmmi).
        '\u{21BC}' => (Italic, 0x28), // \leftharpoonup 349
        '\u{21BD}' => (Italic, 0x29), // \leftharpoondown 350
        '\u{21C0}' => (Italic, 0x2A), // \rightharpoonup 351
        '\u{21C1}' => (Italic, 0x2B), // \rightharpoondown 352
        '\u{25B7}' => (Italic, 0x2E), // \triangleright 265
        '\u{25C1}' => (Italic, 0x2F), // \triangleleft 264
        '\u{22C6}' => (Italic, 0x3F), // \star 299
        '\u{266D}' => (Italic, 0x5B), // \flat 234
        '\u{266E}' => (Italic, 0x5C), // \natural 235
        '\u{266F}' => (Italic, 0x5D), // \sharp 236
        '\u{2323}' => (Italic, 0x5E), // \smile 347
        '\u{2322}' => (Italic, 0x5F), // \frown 348
        // Kernel row from the `operators` family (cmr): fontmath.ltx 509.
        '$' => (Roman, 0x24), // \mathdollar
        // Accents (plain.tex \mathaccent slots).
        '^' | '\u{02C6}' => (Roman, 0x5E),
        '~' | '\u{02DC}' => (Roman, 0x7E),
        '\u{00AF}' => (Roman, 0x16),
        '\u{02D9}' => (Roman, 0x5F),
        '\u{00A8}' => (Roman, 0x7F),
        '\u{00B4}' => (Roman, 0x13),
        '`' => (Roman, 0x12),
        '\u{02D8}' => (Roman, 0x15),
        '\u{02C7}' => (Roman, 0x14),
        '\u{20D7}' => (Italic, 0x7E),
        // \imath, \jmath
        // Both the text dotless pair and the Mathematical Alphanumeric pair
        // `unicode-math` names for these two commands, which is what the
        // compiler emits (it is 0.322456 em / 0.384030 em wide in Latin Modern
        // Math, cmmi10's width; the text pair is 14% and 20% narrow there).
        '\u{0131}' | '\u{1D6A4}' => (Italic, 0x7B),
        '\u{0237}' | '\u{1D6A5}' => (Italic, 0x7C),
        _ => return None,
    })
}

/// plain.tex `\delcode`: small (family, code) and the large `cmex` code.
pub fn delimiter_slot(ch: char) -> Option<((Family, u8), u8)> {
    use Family::*;
    Some(match ch {
        '(' => ((Roman, 0x28), 0x00),
        ')' => ((Roman, 0x29), 0x01),
        '[' => ((Roman, 0x5B), 0x02),
        ']' => ((Roman, 0x5D), 0x03),
        '\u{230A}' => ((Symbol, 0x62), 0x04),
        '\u{230B}' => ((Symbol, 0x63), 0x05),
        '\u{2308}' => ((Symbol, 0x64), 0x06),
        '\u{2309}' => ((Symbol, 0x65), 0x07),
        '{' => ((Symbol, 0x66), 0x08),
        '}' => ((Symbol, 0x67), 0x09),
        '\u{27E8}' => ((Symbol, 0x68), 0x0A),
        '\u{27E9}' => ((Symbol, 0x69), 0x0B),
        '|' | '\u{2223}' => ((Symbol, 0x6A), 0x0C),
        '\u{2016}' | '\u{2225}' => ((Symbol, 0x6B), 0x0D),
        '/' => ((Roman, 0x2F), 0x0E),
        '\\' | '\u{2216}' => ((Symbol, 0x6E), 0x0F),
        '\u{2191}' => ((Symbol, 0x22), 0x78),
        '\u{2193}' => ((Symbol, 0x23), 0x79),
        '\u{2195}' => ((Symbol, 0x6C), 0x3F),
        _ => return None,
    })
}

impl MathFontMetrics for CmMathMetrics {
    fn params(&self, size: SizeClass) -> MathParams {
        let (sy, _, at) = self.font(Family::Symbol, size);
        let (ex, _, ex_at) = self.font(Family::Extension, size);
        let s = |n: usize| sy.fontdimen(n, at);
        let x = |n: usize| ex.fontdimen(n, ex_at);
        MathParams {
            size: at,
            x_height: s(5),
            quad: s(6),
            num1: s(8),
            num2: s(9),
            num3: s(10),
            denom1: s(11),
            denom2: s(12),
            sup1: s(13),
            sup2: s(14),
            sup3: s(15),
            sub1: s(16),
            sub2: s(17),
            sup_drop: s(18),
            sub_drop: s(19),
            delim1: s(20),
            delim2: s(21),
            axis_height: s(22),
            default_rule_thickness: x(8),
            big_op_spacing1: x(9),
            big_op_spacing2: x(10),
            big_op_spacing3: x(11),
            big_op_spacing4: x(12),
            big_op_spacing5: x(13),
            script_space: 0.5,
            // plain.tex 1.2pt as TeX stores it: 78643sp (the usual 1sp truncation).
            null_delimiter_space: 78643.0 / 65536.0,
            delimiter_factor: 0.901,
            delimiter_shortfall: 5.0,
        }
    }

    fn font_name(&self, font: FontId) -> String {
        ALL_FONTS
            .get(font.0 as usize)
            .map(|f| f.name.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn glyph(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        let (family, code) = symbol_slot(ch)?;
        self.make_glyph(family, code, ch, size)
    }

    fn large_operator(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        let (family, code) = symbol_slot(ch)?;
        let (font, font_id, at) = self.font(family, size);
        let c = font.char(code)?;
        let larger = font.next_larger(c)?;
        Some(glyph_from(larger, font_id, ch, at))
    }

    fn delimiter_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        let mut out = Vec::new();
        let Some(((family, code), large)) = delimiter_slot(ch) else {
            return out;
        };
        if let Some(g) = self.make_glyph(family, code, ch, size) {
            out.push(g);
        }
        self.extension_chain(large, ch, size, &mut out);
        out
    }

    fn radical_sizes(&self, size: SizeClass) -> Vec<Glyph> {
        let mut out = Vec::new();
        if let Some(g) = self.make_glyph(Family::Symbol, 0x70, '\u{221A}', size) {
            out.push(g);
        }
        self.extension_chain(0x70, '\u{221A}', size, &mut out);
        out
    }

    fn delimiter_extensible(&self, ch: char, size: SizeClass) -> Option<Extensible> {
        let (_, large) = delimiter_slot(ch)?;
        self.extension_recipe(large, ch, size)
    }

    fn radical_extensible(&self, size: SizeClass) -> Option<Extensible> {
        self.extension_recipe(0x70, '\u{221A}', size)
    }

    /// `\operator@font` is the roman family (cmr) at the current size.
    fn extension_glyph(&self, code: u8, ch: char) -> Option<Glyph> {
        self.make_glyph(Family::Extension, code, ch, SizeClass::Text)
    }

    fn text_glyph(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        let code = if ch.is_ascii() { ch as u8 } else { return None };
        self.make_glyph(Family::Roman, code, ch, size)
    }

    fn accent_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        let mut out = Vec::new();
        // \widehat and \widetilde live in cmex and grow with the base.
        match ch {
            '\u{0302}' => self.extension_chain(0x62, ch, size, &mut out),
            '\u{0303}' => self.extension_chain(0x65, ch, size, &mut out),
            _ => {
                let Some((family, code)) = symbol_slot(ch) else {
                    return out;
                };
                let (font, font_id, at) = self.font(family, size);
                let mut cur = font.char(code);
                while let Some(c) = cur {
                    out.push(glyph_from(c, font_id, ch, at));
                    cur = font.next_larger(c);
                    if out.len() > 8 {
                        break;
                    }
                }
            }
        }
        out
    }
}

/// Size in pt of a size class under these metrics (for tests and reports).
pub fn size_pt(m: &CmMathMetrics, size: SizeClass) -> f64 {
    m.size_pt(size)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `fontmath.ltx` 278-279 and 301-302 put the square relations in the
    /// `symbols` (cmsy) family, so they box from the cmsy TFM exactly as
    /// pdfLaTeX sets them. pdfTeX 3.141592653 (TeX Live 2025) `\show` gives
    /// \mathchar"2274, "2275, "3276 and "3277 -- family 2, slots "74-"77 --
    /// both with and without amssymb loaded, and the glyphs measure 6.66669pt
    /// ("74/"75) and 7.7778pt ("76/"77) at 10pt.
    #[test]
    fn square_relations_are_cmsy_74_through_77() {
        assert_eq!(symbol_slot('\u{2294}'), Some((Family::Symbol, 0x74))); // \sqcup
        assert_eq!(symbol_slot('\u{2293}'), Some((Family::Symbol, 0x75))); // \sqcap
        assert_eq!(symbol_slot('\u{2291}'), Some((Family::Symbol, 0x76))); // \sqsubseteq
        assert_eq!(symbol_slot('\u{2292}'), Some((Family::Symbol, 0x77))); // \sqsupseteq
        // The widths the cmsy TFM actually carries at the 10pt text size.
        let m = CmMathMetrics::latex_10pt();
        let five = |c: char| format!("{:.5}", m.glyph(c, SizeClass::Text).expect("glyph").width);
        assert_eq!(five('\u{2294}'), "6.66669");
        assert_eq!(five('\u{2293}'), "6.66669");
        assert_eq!(five('\u{2291}'), "7.77780");
        assert_eq!(five('\u{2292}'), "7.77780");
    }

    #[test]
    fn parameters_match_plain_tex_fontdimens() {
        let m = CmMathMetrics::latex_10pt();
        let p = m.params(SizeClass::Text);
        // Values as pdfTeX prints them (\showbox, 5 decimals) for cmsy10/cmex10.
        let five = |v: f64| format!("{:.5}", v);
        assert_eq!(five(p.axis_height), "2.50000");
        assert_eq!(five(p.x_height), "4.30554");
        assert_eq!(five(p.num1), "6.76508");
        assert_eq!(five(p.num2), "3.93732");
        assert_eq!(five(p.denom2), "3.44841");
        assert_eq!(five(p.sup2), "3.62892");
        assert_eq!(five(p.sub2), "2.47217");
        assert_eq!(five(p.sub1), "1.49998");
        assert_eq!(five(p.default_rule_thickness), "0.39998");
        assert_eq!(five(p.big_op_spacing1), "1.11111");
        assert_eq!(five(p.big_op_spacing3), "1.99998");
        assert_eq!(five(p.quad), "10.00002");
        // 655361sp / 18 truncates to 36408sp: 4mu = 2.22217pt as pdfTeX prints.
        assert_eq!(five(4.0 * p.mu()), "2.22217");
        let s = m.params(SizeClass::Script);
        assert_eq!(five(s.sup_drop), "2.47220");
        assert_eq!(five(s.sub_drop), "0.49998");
        assert_eq!(five(s.axis_height), "1.75000");
        // Family 3 stays at 10pt in LaTeX/plain: the inner rule of a nested
        // fraction is still 0.39998pt (pdfTeX \showbox evidence).
        assert_eq!(five(s.default_rule_thickness), "0.39998");
        let scaled = CmMathMetrics {
            extension: ExtensionSizing::Scaled,
            ..CmMathMetrics::latex_10pt()
        };
        assert_eq!(
            five(scaled.params(SizeClass::Script).default_rule_thickness),
            "0.27998"
        );
    }

    #[test]
    fn delimiter_chain_for_parenthesis() {
        let m = CmMathMetrics::latex_10pt();
        let sizes = m.delimiter_sizes('(', SizeClass::Text);
        let totals: Vec<f64> = sizes
            .iter()
            .map(|g| (g.total_height() * 100.0).round() / 100.0)
            .collect();
        assert_eq!(totals, vec![10.0, 12.0, 18.0, 24.0, 30.0]);
        assert_eq!(sizes[0].font_id, FontId(0));
        assert_eq!(sizes[1].font_id, FontId(9));
        assert_eq!(sizes[1].gid, 0x00);
    }

    #[test]
    fn sum_has_a_display_variant() {
        let m = CmMathMetrics::latex_10pt();
        let small = m.glyph('\u{2211}', SizeClass::Text).unwrap();
        let big = m.large_operator('\u{2211}', SizeClass::Text).unwrap();
        assert_eq!(small.gid, 0x50);
        assert_eq!(big.gid, 0x58);
        assert!(big.total_height() > small.total_height());
    }
}
