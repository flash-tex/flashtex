//! Construction and layout of the deliberately small supported math subset.
//!
//! The constants below approximate classic TeX proportions. TeX normally obtains
//! script shifts, axis height, and fraction gaps from font parameters; FlashTeX
//! does not yet read a real math font and these values are honest approximations.

use crate::diagnostics::Diagnostic;
use crate::lexer::{Token, TokenKind};
use crate::Span;

pub const SCRIPT_SCALE: f64 = 0.7;
pub const SECOND_ORDER_SCRIPT_SCALE: f64 = 0.5;
pub const SUPERSCRIPT_RAISE_EM: f64 = 0.45;
pub const SUBSCRIPT_LOWER_EM: f64 = 0.2;
pub const MATH_AXIS_EM: f64 = 0.25;
pub const FRACTION_GAP_EM: f64 = 0.16;
pub const FRACTION_RULE_EM: f64 = 0.06;
/// Symbol.afm `radical` (C 214): ink right edge 515 and top 917, per 1000 em.
pub const RADICAL_INK_RIGHT_EM: f64 = 0.515;
pub const RADICAL_TOP_EM: f64 = 0.917;
/// Symbol.afm `radicalex` (C 96), the vinculum extender: y 881..917.
pub const RADICALEX_THICKNESS_EM: f64 = 0.036;
pub const MATRIX_COLUMN_GAP_EM: f64 = 1.0;
pub const MATRIX_ROW_GAP_EM: f64 = 0.3;
pub const QUAD_EM: f64 = 1.0;

#[derive(Debug, Clone, PartialEq)]
pub struct MathList {
    pub atoms: Vec<MathAtom>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MathAtom {
    pub nucleus: Nucleus,
    pub span: Span,
    pub superscript: Option<MathList>,
    pub subscript: Option<MathList>,
    /// Forces this atom's TeXbook Chapter 17 spacing class rather than
    /// deriving it from the nucleus (see `atom_class`).
    ///
    /// Needed whenever the same glyph must carry two different classes
    /// depending on which command produced it (`\bot` is Ord where `\perp`'s
    /// identical U+22A5 glyph is Rel; `\bigtriangleup` is Bin where
    /// `\triangle`'s identical U+25B3 glyph is Ord), and by the
    /// `\mathbin`/`\mathrel`/`\mathord`/`\mathop`/`\mathopen`/`\mathclose`/
    /// `\mathpunct` family, which boxes an arbitrary math list as one atom of
    /// the stated class.
    ///
    /// Public because the class cannot be recovered downstream: the render
    /// pipeline re-derives it today by reading the control word back out of
    /// the source at the atom's span (`typeset::class_override_of`), which
    /// cannot tell `\colon`'s two definitions apart -- the kernel's is Punct
    /// and amsmath's is Ord, from the same five characters of source.
    pub class_override: Option<AtomClass>,
    /// Forces a symbol atom's advance, in ems of its size, when the glyph is
    /// shared by commands whose TeX fonts differ (`\varnothing` is msbm10's
    /// 0.777781em where `\emptyset`'s identical U+2205 is cmsy10's).
    pub width_em: Option<f64>,
    /// The amssymb/amsfonts symbol (`crate::amssymb`) this atom sets: its msam/msbm
    /// font slot and math class, which a TFM-driven layout boxes from the
    /// AMS font metrics while `nucleus` keeps the Unicode text.
    pub ams_symbol: Option<&'static crate::amssymb::AmsSymbol>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Nucleus {
    Symbol(String),
    /// `\big(`, `\Bigr]`, ...: a delimiter scaled to `scale` times the current
    /// size (cmex10's 1.2, 1.8, 2.4, 3.0) and centred on the math axis.
    /// `\left`/`\right` (`role` `Left`/`Right`, including the invisible null
    /// delimiter) reuse this nucleus with a continuous `scale` computed from
    /// their enclosed content by TeX's rule 19, rather than one of cmex10's
    /// four fixed steps.
    SizedDelimiter {
        glyph: String,
        scale: f64,
        role: DelimiterRole,
    },
    /// Literal text with explicit Roman intent, distinct from math symbols.
    Text(String),
    /// Explicit TeX math glue. `em` is in quads of the math symbol font
    /// (18 mu: `\,` is 3/18) unless `font_em`, when it is in ems of the
    /// current text font: `\quad` is `\hskip1em` (latex.ltx), and `em` in
    /// math mode is `\fontdimen6\font` of the text font selected outside the
    /// formula, at the text size whatever the math style.
    Space {
        em: f64,
        font_em: bool,
    },
    Fraction {
        numerator: MathList,
        denominator: MathList,
    },
    Radical(MathList),
    /// `\mathbf{...}`: literal text in the bold roman face.
    Bold(String),
    /// `\boxed`, `\overline` and `\underline`: a list with real rules.
    Framed {
        body: MathList,
        frame: Frame,
    },
    /// `\overset`, `\underset`, `\stackrel`: a base with a script-size list
    /// centred directly above or below it.
    Stacked {
        base: MathList,
        over: Option<MathList>,
        under: Option<MathList>,
    },
    /// `array`, `cases` and the amsmath matrix environments: a grid of cells
    /// with per-column alignment (`l`, `c`, `r`) and optional stretched fences.
    Matrix {
        rows: Vec<Vec<MathList>>,
        columns: String,
        left: String,
        right: String,
    },
    /// `\hat`, `\bar`, `\vec`, ..., `\widehat`, `\widetilde`: a mark placed
    /// over `body`. See [`Accent`] for which marks have a real base-14 glyph.
    Accent {
        accent: Accent,
        body: MathList,
    },
    /// `\rule[<raise>]{<width>}{<height>}` in math: latex.ltx's `\@rule`
    /// `\hbox` (see `text_builtins::TextRule`), an Ord box whose `em`/`ex`
    /// are the text font's at the formula's text size.
    Rule(crate::text_builtins::TextRule),
    /// `\mathbin{...}`, `\mathrel{...}`, and the rest of the `\math*` class
    /// family (TeXbook Chapter 17): an arbitrary math list boxed as a single
    /// atom, laid out like a bare `{...}` group. The enclosing [`MathAtom`]'s
    /// `class_override` carries the forced spacing class; this variant only
    /// exists so a multi-atom argument stays one atom for spacing purposes
    /// instead of flattening into the surrounding list.
    Group(MathList),
    /// amsmath `\genfrac{left}{right}{thickness}{style}{num}{den}`
    /// (`amsmath.sty` lines 237-312) and its shorthands: `\dfrac`/`\tfrac`
    /// (no delimiters, display/text style), `\binom`/`\dbinom`/`\tbinom`
    /// (parentheses, zero thickness). `thickness_pt` `None` is the default
    /// rule; empty `left`/`right` are null delimiters; `style` `None` keeps
    /// the current style. amsmath wraps the result in a group (an ordinary
    /// atom), unlike the plain `\frac`'s inner [`Nucleus::Fraction`].
    GenFraction {
        numerator: MathList,
        denominator: MathList,
        thickness_pt: Option<f64>,
        left: String,
        right: String,
        style: Option<MathStyle>,
    },
    /// `\phantom`/`\hphantom`/`\vphantom` (`latex.ltx` `\ph@nt`): an empty box
    /// with `body`'s width (`horizontal`) and/or height and depth (`vertical`).
    Phantom {
        body: MathList,
        horizontal: bool,
        vertical: bool,
    },
    /// `\operatorname{...}`, `\operatorname*{...}` and commands declared by
    /// `\DeclareMathOperator` (`amsopn.sty` `\qopname`): `\mathop{\operator@font
    /// body}` followed by `\limits` (`limits`, the starred forms) or
    /// `\nolimits`. `body` holds upright [`Nucleus::Text`] runs and the math
    /// glue written inside the argument (`arg\,max`).
    Operator {
        body: MathList,
        limits: bool,
    },
    /// amsmath `\substack{a \\ b}` (`subarray{c}`, `amsmath.sty` lines
    /// 1030-1059): rows in `\scriptstyle`, centred, `\vcenter`ed.
    SubArray {
        rows: Vec<MathList>,
        align: char,
    },
    /// amsmath `\xrightarrow[below]{above}`, `\xleftarrow` and mathtools'
    /// `\xleftrightarrow` (`amsmath.sty` 971-979 `\arrowfill@`, 1012-1028
    /// `\ext@arrow`; `mathtools.sty` 323-326): a relation whose arrow is
    /// stretched to fit its labels, `above` set as the upper limit and
    /// `below` (the optional argument, empty when absent) as the lower one.
    ExtArrow {
        arrow: ExtArrow,
        above: MathList,
        below: MathList,
    },
}

/// Which extensible arrow an [`Nucleus::ExtArrow`] draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtArrow {
    /// `\xrightarrow`: `\ext@arrow 0359\rightarrowfill@`.
    Right,
    /// `\xleftarrow`: `\ext@arrow 3095\leftarrowfill@`.
    Left,
    /// mathtools `\xleftrightarrow`: `\ext@arrow 3399`, `\leftarrow\relbar\rightarrow`.
    LeftRight,
}

/// An explicit math style (`\displaystyle` .. `\scriptscriptstyle`, and the
/// `{0..3}` style argument of amsmath's `\genfrac`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathStyle {
    Display,
    Text,
    Script,
    ScriptScript,
}

/// The atom class plain TeX gives a `\big` delimiter: `\bigl` opens, `\bigr`
/// closes, `\bigm` is a relation and bare `\big` is ordinary.
///
/// `Left` and `Right` are `\left`/`\right` specifically (classed as `Open`
/// and `Close` in `atom_class`, same spacing as `\bigl`/`\bigr`), kept as
/// separate variants so the `\left`/`\right` stretch-to-content pairing pass
/// never mistakes a `\bigl`/`\bigr` pair — which stays at its fixed cmex10
/// size — for one of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelimiterRole {
    Ord,
    Open,
    Close,
    Rel,
    Left,
    Right,
}

/// `\hat`..`\grave`, plus `\widehat`/`\widetilde`.
///
/// The compiler renders math with Adobe's Core 14 Symbol/Times-Roman faces,
/// not Computer Modern, so TeX's exact accent geometry is not reproducible.
/// Where a real base-14 glyph exists for the
/// mark, it is used, scaled and centered over `body`; `\check` and `\breve`
/// have no such glyph (no caron or breve character in WinAnsi or the Symbol
/// encoding — see `crate::export`) and are reported rather than faked, the
/// same policy `crate::export::map_char` already applies to every other
/// unrepresentable character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accent {
    Hat,
    Bar,
    Vec,
    Tilde,
    Dot,
    Ddot,
    Check,
    Breve,
    Acute,
    Grave,
    WideHat,
    WideTilde,
}

impl Accent {
    pub fn command(self) -> &'static str {
        match self {
            Accent::Hat => "hat",
            Accent::Bar => "bar",
            Accent::Vec => "vec",
            Accent::Tilde => "tilde",
            Accent::Dot => "dot",
            Accent::Ddot => "ddot",
            Accent::Check => "check",
            Accent::Breve => "breve",
            Accent::Acute => "acute",
            Accent::Grave => "grave",
            Accent::WideHat => "widehat",
            Accent::WideTilde => "widetilde",
        }
    }

    /// The glyph drawn above `body`, or `None` when no base-14 glyph exists.
    ///
    /// `\widehat`/`\widetilde` reuse the plain `\hat`/`\tilde` glyph: TeX
    /// grows these from a cmex10 successor chain to cover a wide base, and
    /// there is no equivalent stretchy glyph or font-growing mechanism here,
    /// so a multi-atom base gets a diagnostic (see `accent_atom`) rather than
    /// a silently-too-narrow mark.
    pub fn glyph(self) -> Option<char> {
        match self {
            Accent::Hat | Accent::WideHat => Some('\u{2C6}'), // circumflex accent
            Accent::Bar => Some('\u{AF}'),                    // macron
            // TeX's \vec draws a short low arrow; the closest real base-14
            // glyph is the full-size Symbol arrowright. An approximation,
            // not a fabrication: it is a real arrow glyph, just not the
            // exact short accent stroke.
            Accent::Vec => Some('\u{2192}'),
            Accent::Tilde | Accent::WideTilde => Some('\u{2DC}'), // small tilde
            // TeX's \dot is a raised dot above (U+02D9), which has no
            // base-14 glyph either. The Symbol/Times middle dot U+00B7 (the
            // same character already used for \cdot) is the closest real
            // stand-in.
            Accent::Dot => Some('\u{B7}'),
            Accent::Ddot => Some('\u{A8}'),  // diaeresis
            Accent::Acute => Some('\u{B4}'), // acute accent
            Accent::Grave => Some('\u{60}'), // grave accent
            Accent::Check | Accent::Breve => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    Box,
    Over,
    Under,
    /// `\overbrace` (`fontmath.ltx` 430-433): `\mathop{..}\limits`, so its
    /// scripts are limits.
    OverBrace,
    /// `\underbrace` (`fontmath.ltx` 434-437).
    UnderBrace,
    /// amsmath `\overrightarrow` (`amsmath.sty` 985-986).
    OverRightArrow,
    /// amsmath `\overleftarrow` (987-988).
    OverLeftArrow,
    /// amsmath `\overleftrightarrow` (989-990).
    OverLeftRightArrow,
    /// amsmath `\underrightarrow` (1001-1002).
    UnderRightArrow,
    /// amsmath `\underleftarrow` (1003-1004).
    UnderLeftArrow,
    /// amsmath `\underleftrightarrow` (1005-1006).
    UnderLeftRightArrow,
}

impl Frame {
    /// Whether the decoration sits above the body (a rule over it in the
    /// compiler's own layout).
    pub fn is_over(self) -> bool {
        matches!(
            self,
            Frame::Box
                | Frame::Over
                | Frame::OverBrace
                | Frame::OverRightArrow
                | Frame::OverLeftArrow
                | Frame::OverLeftRightArrow
        )
    }

    /// Whether the decoration sits below the body.
    pub fn is_under(self) -> bool {
        matches!(
            self,
            Frame::Box
                | Frame::Under
                | Frame::UnderBrace
                | Frame::UnderRightArrow
                | Frame::UnderLeftArrow
                | Frame::UnderLeftRightArrow
        )
    }

    /// The extensible arrow of an over/under arrow frame.
    pub fn arrow(self) -> Option<ExtArrow> {
        match self {
            Frame::OverRightArrow | Frame::UnderRightArrow => Some(ExtArrow::Right),
            Frame::OverLeftArrow | Frame::UnderLeftArrow => Some(ExtArrow::Left),
            Frame::OverLeftRightArrow | Frame::UnderLeftRightArrow => Some(ExtArrow::LeftRight),
            _ => None,
        }
    }
}

/// Math-mode environments implemented as grids: (name, default column
/// alignment repeated for every column, left fence, right fence).
pub(crate) const GRID_ENVIRONMENTS: &[(&str, char, &str, &str)] = &[
    ("array", 'c', "", ""),
    ("matrix", 'c', "", ""),
    ("smallmatrix", 'c', "", ""),
    ("pmatrix", 'c', "(", ")"),
    ("bmatrix", 'c', "[", "]"),
    ("Bmatrix", 'c', "{", "}"),
    ("vmatrix", 'c', "|", "|"),
    ("Vmatrix", 'c', "‖", "‖"),
    ("cases", 'l', "{", ""),
    // mathtools.sty `\newcases{dcases}`: `cases` with `\displaystyle` cells.
    ("dcases", 'l', "{", ""),
    ("aligned", 'c', "", ""),
    ("alignedat", 'c', "", ""),
    ("split", 'c', "", ""),
    ("gathered", 'c', "", ""),
];

#[derive(Debug, Clone, PartialEq)]
pub struct MathItem {
    /// Explicit font for text nuclei; None retains symbol-driven selection.
    pub font: Option<crate::layout::Font>,
    pub text: String,
    pub x: f64,
    /// Offset from the surrounding text baseline; positive is downward.
    pub baseline: f64,
    pub size: f64,
    pub span: Span,
    /// A real rectangular rule represented alongside the legacy text fallback.
    /// Coordinates are relative to the surrounding math baseline.
    pub rule: Option<MathRule>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MathRule {
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MathBox {
    pub items: Vec<MathItem>,
    pub width: f64,
    pub ascent: f64,
    pub descent: f64,
}

/// The loaded packages that change what a math command *means*.
///
/// Math parsing is otherwise package-blind, which silently picks the LaTeX
/// kernel's definition for every construct a package redefines — the wrong one
/// whenever the document loaded the package, which for amsmath is most
/// documents that use the affected commands. The parser resolves this from the
/// document class and `\usepackage` (`parser::P::math_packages`) and hands it
/// to every entry point below; `MathPackages::KERNEL` is "nothing loaded", the
/// definition in `fontmath.ltx`/`latex.ltx`.
///
/// Deliberately a plain `Copy` value passed down the parse, not a global: the
/// same process compiles many documents, and `siunitx`'s thread-local settings
/// are the mistake this is not repeating.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MathPackages {
    /// `amsmath` is loaded, directly or by a package or class that loads it.
    ///
    /// Currently this only reaches `\colon`, which amsmath redefines
    /// (`amsmath.sty` 409-410) from the kernel's `\mathpunct{:}` to
    /// `\mskip2mu{:}\mskip6mu plus1mu` — 5mu wider in a formula, measured
    /// against pdflatex. Other amsmath redefinitions the audit found are
    /// listed on the pull request; each one that lands reads this same flag.
    pub amsmath: bool,
    /// `amssymb` is loaded, so the whole `AMSa`/`AMSb` (msam/msbm) inventory
    /// of `amssymb.sty` exists.
    ///
    /// Without it base LaTeX2e has **no definition at all** for 203 of the
    /// names in `crate::amssymb` (`\square`, `\nleq`, ... — every one probed
    /// with `\ifcsname` under TeX Live 2025) and pdflatex answers "Undefined
    /// control sequence". The table used to be applied unconditionally, which
    /// made every such document silently diverge.
    pub amssymb: bool,
    /// `amsfonts` is loaded — by itself, or by `amssymb`, which requires it.
    ///
    /// `amsfonts.sty` declares a 22-name subset of the same symbol fonts
    /// (`\ulcorner`, `\square`, `\yen`, the dashed arrows) plus the
    /// `\mathbb`/`\mathfrak` alphabets, so `\usepackage{amsfonts}` alone
    /// provides those and nothing else. Each symbol carries which of the two
    /// files declares it (`amssymb::Provider`).
    pub amsfonts: bool,
}

/// Packages that load amsmath, so that `\usepackage{X}` alone gives amsmath's
/// definitions. Measured, not assumed: each name was confirmed by compiling
/// `\documentclass[10pt]{article}\usepackage{X}` with TeX Live 2025 pdflatex
/// and checking `\hbox{$a a\colon b b$}` against `\hbox{$a a\mathpunct{:}b b$}`
/// (kernel, 23.5995pt) and `\hbox{$a a\mskip2mu{:}\mskip6mu plus1mu b b$}`
/// (amsmath, 26.37721pt).
///
/// `amsthm`, `amssymb`, `amsfonts`, `amsopn`, `amsbsy`, `amscd`, `amstext`,
/// `bm`, `unicode-math`, `siunitx`, `esint`, `breqn`, `cases`, `mathdots`,
/// `thmtools`, `braket`, `cancel`, `tikz-cd` and `diagbox` measured as kernel
/// and are deliberately absent.
const AMSMATH_PACKAGES: &[&str] = &[
    "amsmath",
    "mathtools",
    "empheq",
    "physics",
    "nccmath",
    "aligned-overset",
    "cool",
    "commath",
    "mismath",
    "mhchem",
    "chemformula",
];

/// Document classes that load amsmath before the preamble runs, measured the
/// same way with an empty preamble. `article`, `report`, `book`, `memoir`,
/// `scrartcl`, `scrbook`, `scrreprt`, `revtex4-2`, `elsarticle`, `IEEEtran`,
/// `letter`, `proc`, `slides` and `amsdtx` measured as kernel.
const AMSMATH_CLASSES: &[&str] = &["amsart", "amsbook", "amsproc", "acmart", "beamer"];

/// Packages that make the full `amssymb` inventory exist. Measured the same
/// way: `\documentclass[10pt]{article}\usepackage{X}` compiled with TeX Live
/// 2025 pdflatex, asking `\ifcsname nleq\endcsname` (an `amssymb.sty`-only
/// name) and `\ifcsname ulcorner\endcsname` (also in `amsfonts.sty`).
///
/// The eight non-AMS names are math *font* packages that really do
/// `\RequirePackage{amssymb}`, so the commands exist; they then point the
/// symbol fonts at their own faces, which this compiler does not model — a
/// separate, pre-existing divergence, not one this gate introduces.
///
/// `libertinust1math` is deliberately absent although `\nleq` is defined
/// under it: it defines part of the inventory itself without loading either
/// AMS package (`\ulcorner`, `\yen`, `\mathfrak` and the dashed arrows stay
/// undefined), so neither flag describes it. `fourier` and `siunitx` define
/// `\square` alone by other means and are absent for the same reason.
/// `amsthm`, `amsopn`, `amsbsy`, `amscd`, `amstext`, `bm`, `unicode-math`,
/// `stmaryrd`, `wasysym`, `esint`, `mathrsfs`, `eucal`, `euler`, `mathpazo`,
/// `mathptmx`, `mathdesign`, `concmath`, `dsfont`, `bbm`, `bbold`, `cmll`,
/// `physics`, `mathtools` and 20 more measured as providing neither.
const AMSSYMB_PACKAGES: &[&str] = &[
    "amssymb",
    "MnSymbol",
    "txfonts",
    "pxfonts",
    "newtxmath",
    "newpxmath",
    "mathabx",
    "kpfonts",
    "cool",
];

/// Classes that load `amssymb` before the preamble runs. `amsart`, `amsbook`
/// and `amsproc` load only `amsfonts` and are in `AMSFONTS_CLASSES` instead;
/// `article`, `report`, `book`, `memoir`, `scrartcl`, `scrbook`, `scrreprt`,
/// `revtex4-2`, `elsarticle`, `IEEEtran`, `letter`, `proc`, `slides` and
/// `amsdtx` load neither.
const AMSSYMB_CLASSES: &[&str] = &["acmart", "beamer"];

/// Packages that make the `amsfonts.sty` subset exist without necessarily
/// bringing the rest of `amssymb`. Every name in `AMSSYMB_PACKAGES` also
/// measured as loading `amsfonts` (`amssymb.sty` requires it), so those are
/// folded in by `load_package` rather than repeated here.
const AMSFONTS_PACKAGES: &[&str] = &["amsfonts"];

/// Classes that load `amsfonts`: the AMS classes load it (and `amsmath`) but
/// not `amssymb`, so `\usepackage`-less `amsart` gets `\ulcorner` but not
/// `\nleq` — measured.
const AMSFONTS_CLASSES: &[&str] = &["amsart", "amsbook", "amsproc", "acmart", "beamer"];

impl MathPackages {
    /// Nothing loaded: every command takes its LaTeX kernel definition, and
    /// every package-provided symbol is undefined.
    pub const KERNEL: Self = Self {
        amsmath: false,
        amssymb: false,
        amsfonts: false,
    };

    /// Folds one `\documentclass` name in.
    pub fn load_class(&mut self, class: &str) {
        self.amsmath |= AMSMATH_CLASSES.contains(&class);
        self.amssymb |= AMSSYMB_CLASSES.contains(&class);
        self.amsfonts |= AMSFONTS_CLASSES.contains(&class);
    }

    /// Folds one `\usepackage`/`\RequirePackage` name in. Loading is
    /// cumulative: no package unloads another's redefinitions.
    pub fn load_package(&mut self, package: &str) {
        self.amsmath |= AMSMATH_PACKAGES.contains(&package);
        let amssymb = AMSSYMB_PACKAGES.contains(&package);
        self.amssymb |= amssymb;
        // `amssymb.sty` line 8 is `\RequirePackage{amsfonts}`, so anything
        // that gives the full inventory gives the subset too.
        self.amsfonts |= amssymb || AMSFONTS_PACKAGES.contains(&package);
    }

    /// Whether a symbol of `crate::amssymb` is defined at all: the file that
    /// declares it has to have been loaded.
    pub fn provides(&self, symbol: &crate::amssymb::AmsSymbol) -> bool {
        match symbol.provider {
            crate::amssymb::Provider::Amsfonts => self.amsfonts,
            crate::amssymb::Provider::Amssymb => self.amssymb,
        }
    }
}

pub fn parse_tokens(
    tokens: &[Token],
    packages: MathPackages,
    diagnostics: &mut Vec<Diagnostic>,
) -> MathList {
    let (list, unclosed) = parse_tokens_reporting_unclosed(tokens, packages, diagnostics, false);
    if let Some(open) = unclosed {
        diagnostics.push(Diagnostic::error(
            "math group is missing its closing brace",
            Some(open),
            Some("closed the group at the math delimiter".into()),
        ));
    }
    list
}

/// Parses a math token list and returns, instead of reporting it, the `{`
/// of the innermost braced group left open when the tokens ran out. Every
/// enclosing open group and every argument that could not be read because
/// the tokens ran out is a consequence of that one opener, so none of them
/// is diagnosed separately: the caller emits one primary diagnostic whose
/// wording depends on where the math ended (its closing delimiter, or the
/// end of the paragraph when the math itself is unterminated).
///
/// `cut_off` says the tokens end because the math was unterminated, not at
/// a closing delimiter: an argument missing at the very end is then input
/// not typed yet, covered by the caller's diagnostic, and is not reported.
pub fn parse_tokens_reporting_unclosed(
    tokens: &[Token],
    packages: MathPackages,
    diagnostics: &mut Vec<Diagnostic>,
    cut_off: bool,
) -> (MathList, Option<Span>) {
    let split = split_word_tokens(tokens);
    let mut parser = MathParser {
        tokens: &split,
        i: 0,
        depth: 0,
        diagnostics,
        packages,
        pending: Vec::new(),
        unclosed: None,
        cut_off,
    };
    let list = parser.list(false);
    (list, parser.unclosed)
}

/// Math environments this crate typesets inside math mode. Any other
/// `\begin`/`\end` ends an unterminated math scan (see the parser's
/// paragraph-boundary recovery).
pub fn is_math_environment(name: &str) -> bool {
    GRID_ENVIRONMENTS.iter().any(|(env, ..)| *env == name)
}

/// Maximum nesting of braced math groups, scripts, fractions and radicals.
///
/// The list parser is recursive descent, so a document full of unclosed openers
/// recurses once per opener. Without a bound, a pathological file — or a
/// half-typed one — overflows the stack and kills the worker mid-keystroke.
/// Exceeding the bound is an explicit diagnostic, not a crash.
// Keep ample headroom for the command parser's stack frame on the macOS Swift
// app's worker thread as the supported command set grows. The previous 256
// limit could exhaust that thread before the guard was reached.
pub const MAX_MATH_DEPTH: usize = 128;

struct MathParser<'a> {
    tokens: &'a [Token],
    i: usize,
    depth: usize,
    diagnostics: &'a mut Vec<Diagnostic>,
    /// The packages whose redefinitions apply to this list (`MathPackages`).
    packages: MathPackages,
    /// Atoms produced by the last `atom()` call beyond the one it returned
    /// (a flattened style group, a root index), in order after it.
    pending: Vec<MathAtom>,
    /// The `{` of the first (innermost) group found still open at the end of
    /// the tokens. Once set, the tokens are exhausted, so later "missing
    /// argument" failures at the end are cascades and are not reported.
    unclosed: Option<Span>,
    /// The tokens end where unterminated math was cut off.
    cut_off: bool,
}

impl MathParser<'_> {
    fn list(&mut self, stop_at_brace: bool) -> MathList {
        if self.depth >= MAX_MATH_DEPTH {
            // Consume the rest so the caller cannot loop on the same tokens.
            let span = self.tokens.get(self.i).map(|t| t.span);
            self.diagnostics.push(Diagnostic::error(
                format!("math nesting deeper than {MAX_MATH_DEPTH} levels is not supported"),
                span,
                Some("stopped descending and typeset nothing further in this expression".into()),
            ));
            self.i = self.tokens.len();
            return MathList { atoms: Vec::new() };
        }
        self.depth += 1;
        let result = self.list_inner(stop_at_brace);
        self.depth -= 1;
        result
    }

    /// Whether an argument missing here is only a consequence of the tokens
    /// having run out inside an unclosed group or unterminated math, which
    /// the caller already diagnoses once.
    fn argument_cut_off(&self) -> bool {
        (self.unclosed.is_some() || self.cut_off) && self.i >= self.tokens.len()
    }

    fn list_inner(&mut self, stop_at_brace: bool) -> MathList {
        // Every caller consumes the `{` immediately before a braced list.
        let open = self
            .i
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
            .filter(|token| token.kind == TokenKind::LBrace)
            .map(|token| token.span);
        let mut atoms: Vec<MathAtom> = Vec::new();
        while self.i < self.tokens.len() {
            let token = self.tokens[self.i].clone();
            match token.kind {
                TokenKind::Space | TokenKind::ParBreak | TokenKind::Comment => self.i += 1,
                TokenKind::RBrace if stop_at_brace => {
                    self.i += 1;
                    return MathList { atoms };
                }
                TokenKind::RBrace => {
                    self.i += 1;
                    self.diagnostics.push(Diagnostic::error(
                        "unmatched '}' in math mode",
                        Some(token.span),
                        Some("ignored the stray brace and continued".into()),
                    ));
                }
                TokenKind::LBrace => {
                    self.i += 1;
                    atoms.extend(self.list(true).atoms);
                }
                // Limit-placement switches produce no atom, so a following
                // script still attaches to the operator (`\lim\limits_{x}`).
                TokenKind::Command(ref switch) if switch == "limits" || switch == "nolimits" => {
                    self.i += 1;
                }
                // xcolor in math: `\color[model]{c}` recolours the rest of the
                // group, `\textcolor[model]{c}{body}` its body. The atoms are
                // unchanged; the text parser resolves the colours into
                // `Inline::Math::color_ranges` from the same tokens.
                TokenKind::Command(ref paint) if paint == "color" || paint == "textcolor" => {
                    self.i += 1;
                    self.skip_color_arguments();
                    if paint == "textcolor" {
                        atoms.extend(self.required_group("textcolor", token.span).atoms);
                    }
                }
                TokenKind::Command(ref infix) if infix == "choose" || infix == "over" => {
                    // TeX infix forms: everything before in this group is the
                    // top, everything after (to the group's end) the bottom.
                    self.i += 1;
                    let top = MathList {
                        atoms: std::mem::take(&mut atoms),
                    };
                    let bottom = self.list(stop_at_brace);
                    let nucleus = if infix == "over" {
                        Nucleus::Fraction {
                            numerator: top,
                            denominator: bottom,
                        }
                    } else {
                        Nucleus::Matrix {
                            rows: vec![vec![top], vec![bottom]],
                            columns: "c".into(),
                            left: "(".into(),
                            right: ")".into(),
                        }
                    };
                    return MathList {
                        atoms: vec![MathAtom {
                            nucleus,
                            span: token.span,
                            superscript: None,
                            subscript: None,
                            class_override: None,
                            width_em: None,
                            ams_symbol: None,
                        }],
                    };
                }
                // A bare `&` reaches here only outside a tabular alignment
                // context: `grid_environment` (matrices, `cases`, `array`, …)
                // and the parser's `align`/`gather` row-splitting both consume
                // their own `&` tokens before ever calling into this list, so
                // one seen here is always misplaced.
                TokenKind::Word(ref word) if word == "&" => {
                    self.i += 1;
                    self.diagnostics.push(Diagnostic::error(
                        "misplaced alignment tab character &",
                        Some(token.span),
                        Some("ignored the stray alignment tab and continued".into()),
                    ));
                }
                // latex.ltx 15683-15697: a math `'` is `^\bgroup\prim@s`, which
                // collects every following `'` as another `\prime` and a
                // directly following `^{...}` into the same superscript.
                TokenKind::Word(ref word) if word == "'" => {
                    let mut script = MathList { atoms: Vec::new() };
                    while let Some(t) = self.tokens.get(self.i) {
                        if !matches!(&t.kind, TokenKind::Word(w) if w == "'") {
                            break;
                        }
                        script.atoms.push(symbol("\u{2032}".into(), t.span));
                        self.i += 1;
                    }
                    if matches!(self.tokens.get(self.i).map(|t| &t.kind), Some(TokenKind::Superscript)) {
                        let marker = self.tokens[self.i].span;
                        self.i += 1;
                        script.atoms.extend(self.script_argument(marker).atoms);
                    }
                    if atoms.is_empty() {
                        atoms.push(symbol(String::new(), token.span));
                    }
                    let atom = atoms.last_mut().expect("an atom to carry the primes");
                    if atom.superscript.replace(script).is_some() {
                        self.diagnostics.push(Diagnostic::error(
                            "duplicate script on a math atom",
                            Some(token.span),
                            Some("used the last script and continued".into()),
                        ));
                    }
                }
                TokenKind::Superscript | TokenKind::Subscript => {
                    self.i += 1;
                    let script = self.script_argument(token.span);
                    if let Some(atom) = atoms.last_mut() {
                        let slot = if token.kind == TokenKind::Superscript {
                            &mut atom.superscript
                        } else {
                            &mut atom.subscript
                        };
                        if slot.replace(script).is_some() {
                            self.diagnostics.push(Diagnostic::error(
                                "duplicate script on a math atom",
                                Some(token.span),
                                Some("used the last script and continued".into()),
                            ));
                        }
                    } else {
                        self.diagnostics.push(Diagnostic::error(
                            "script marker has no preceding math atom",
                            Some(token.span),
                            Some("ignored the unattached script".into()),
                        ));
                    }
                }
                _ => {
                    if let Some(atom) = self.atom() {
                        atoms.push(atom);
                        atoms.append(&mut self.pending);
                    }
                }
            }
        }
        if stop_at_brace && self.unclosed.is_none() {
            self.unclosed = open.or_else(|| self.tokens.last().map(|t| t.span));
        }
        MathList { atoms }
    }

    fn script_argument(&mut self, marker: Span) -> MathList {
        while matches!(
            self.tokens.get(self.i).map(|t| &t.kind),
            Some(TokenKind::Space)
        ) {
            self.i += 1;
        }
        if matches!(
            self.tokens.get(self.i).map(|t| &t.kind),
            Some(TokenKind::LBrace)
        ) {
            self.i += 1;
            return self.list(true);
        }
        if let Some(atom) = self.atom() {
            let mut atoms = vec![atom];
            atoms.append(&mut self.pending);
            MathList { atoms }
        } else {
            if self.argument_cut_off() {
                return MathList { atoms: Vec::new() };
            }
            self.diagnostics.push(Diagnostic::error(
                "math script is missing its argument",
                Some(marker),
                Some("used an empty script and continued".into()),
            ));
            MathList { atoms: Vec::new() }
        }
    }

    fn atom(&mut self) -> Option<MathAtom> {
        let token = self.tokens.get(self.i)?.clone();
        self.i += 1;
        match token.kind {
            TokenKind::Word(word) => {
                let mut chars = word.char_indices();
                let (offset, ch) = chars.next()?;
                let end = offset + ch.len_utf8();
                let span = if token.span.end - token.span.start == word.len() {
                    Span::in_document(
                        token.span.document,
                        token.span.start + offset,
                        token.span.start + end,
                    )
                } else {
                    token.span
                };
                // Word runs are split before parsing so a script attaches to
                // one ordinary atom rather than the entire lexer token.
                debug_assert_eq!(
                    end,
                    word.len(),
                    "word tokens must be split before math parsing"
                );
                // The lexer turns the control symbols `\,` `\:` `\;` into a
                // one-character word spanning two source bytes; in math they are
                // thin/medium/thick spaces (3, 4 and 5 mu), not punctuation.
                if token.span.end - token.span.start == 2 {
                    let mu = match ch {
                        ',' => 3.0,
                        ':' | '>' => 4.0,
                        ';' => 5.0,
                        ' ' => 6.0,
                        '!' => -3.0,
                        _ => 0.0,
                    };
                    if mu != 0.0 {
                        return Some(space(mu / 18.0, token.span));
                    }
                    if ch == '|' {
                        return Some(symbol("‖".into(), token.span));
                    }
                }
                Some(symbol(ch.to_string(), span))
            }
            TokenKind::Command(name) => Some(self.command_atom(name, token.span)),
            TokenKind::DisplayMathOpen | TokenKind::DisplayMathClose | TokenKind::MathShift => {
                self.diagnostics.push(Diagnostic::error(
                    "unexpected math delimiter inside math mode",
                    Some(token.span),
                    Some("typeset the delimiter literally and continued".into()),
                ));
                Some(symbol("$".into(), token.span))
            }
            TokenKind::LineBreak => Some(symbol("\\\\".into(), token.span)),
            TokenKind::LBrace => Some(symbol("{".into(), token.span)),
            TokenKind::Verb { .. } => {
                self.diagnostics.push(Diagnostic::error(
                    "\\verb is not supported in math mode",
                    Some(token.span),
                    Some("ignored the \\verb and continued".into()),
                ));
                None
            }
            TokenKind::RBrace
            | TokenKind::Space
            | TokenKind::ParBreak
            | TokenKind::Comment
            | TokenKind::Superscript
            | TokenKind::Subscript => None,
        }
    }

    /// A command whose defining package the document did not load.
    ///
    /// pdflatex's own answer is `! Undefined control sequence`, which typesets
    /// nothing and carries on; this reports the missing `\usepackage` by name
    /// — the actionable half — and falls back to the same literal recovery
    /// every unsupported math command already uses, so the divergence is
    /// visible in the output as well as in the diagnostics.
    fn missing_package(&mut self, name: &str, package: &str, span: Span) -> MathAtom {
        self.diagnostics.push(Diagnostic::command_error(
            name,
            format!("\\{name} requires \\usepackage{{{package}}}"),
            Some(span),
            Some("typeset the command literally and continued".into()),
        ));
        symbol(format!("\\{name}"), span)
    }

    fn command_atom(&mut self, name: String, span: Span) -> MathAtom {
        // The `amsfonts.sty` math alphabets (`\mathbb` 108, `\mathfrak` 106)
        // and its two obsolete spellings exist only once the package is
        // loaded; base LaTeX2e has no definition for any of the four, so
        // pdflatex answers "Undefined control sequence". `\mathbf`, which
        // `\bold` stands for, is the kernel's and stays unconditional — the
        // gate is on the spelling the document wrote, before the alias below.
        if matches!(name.as_str(), "mathbb" | "mathfrak" | "Bbb" | "bold")
            && !self.packages.amsfonts
        {
            return self.missing_package(&name, "amsfonts", span);
        }
        // amsfonts' obsolete `\Bbb` and `\bold` (`amsfonts.sty` 111-116) are
        // `\mathbb` and `\mathbf` after an obsolescence warning.
        let name = match name.as_str() {
            "Bbb" => "mathbb".to_string(),
            "bold" => "mathbf".to_string(),
            _ => name,
        };
        if let Some(operator) = OPERATOR_NAMES.iter().find(|op| **op == name) {
            return text_atom(operator.to_string(), span);
        }
        match name.as_str() {
            // Plain TeX's `\iff` and mathtools's `\implies`/`\impliedby` are
            // macros that expand to a thick space (`\;`, 5mu), the long
            // double arrow, and another thick space — not a bare glyph — so
            // they need their own arms rather than a `COMMAND_GLYPHS` row.
            "iff" | "implies" | "impliedby" => {
                let arrow = match name.as_str() {
                    "iff" => "⟺",
                    "implies" => "⟹",
                    _ => "⟸",
                };
                self.pending.push(symbol(arrow.into(), span));
                self.pending.push(space(5.0 / 18.0, span));
                space(5.0 / 18.0, span)
            }
            // `\colon` sets the same character as a bare `:` but never that
            // character's class. The kernel declares it punctuation
            // (`fontmath.ltx` 400, `\DeclareMathSymbol{\colon}{\mathpunct}
            // {operators}{"3A}`) where `:` itself is a relation (line 385):
            // 0mu before and 3mu after, not 5mu on each side.
            //
            // amsmath renews it (`amsmath.sty` 409-410) to
            // `\nobreak\mskip2mu\mathpunct{}\nonscript\mkern-\thinmuskip{:}%
            // \mskip6mu plus1mu\relax`. The empty punctuation atom's 3mu is
            // exactly cancelled by the negative kern behind it, so what is
            // left is an *ordinary* `:` with 2mu of glue before and 6mu
            // after — 5mu wider than the kernel's, and most documents that
            // write `\colon` load amsmath.
            //
            // Measured at 10pt against TeX Live 2025 pdflatex, with
            // `\hbox{$ab$}` = 9.57755pt as the control: `\hbox{$a\colon b$}`
            // is 14.02196pt without amsmath and 16.79967pt with it. The
            // three-atom form below reproduces amsmath's box to the scaled
            // point in text, display, script and scriptscript style.
            "colon" if self.packages.amsmath => {
                self.pending.push(MathAtom {
                    class_override: Some(AtomClass::Ord),
                    ..symbol(":".into(), span)
                });
                self.pending.push(space(6.0 / 18.0, span));
                space(2.0 / 18.0, span)
            }
            "colon" => MathAtom {
                class_override: Some(AtomClass::Punct),
                ..symbol(":".into(), span)
            },
            // `\bot` renders the exact same Symbol glyph as `\perp`
            // (U+22A5), but is Ord where `\perp` is Rel; `symbol_class` is
            // keyed by glyph, so the class must be forced on the atom instead
            // of invented as a second glyph.
            "bot" => MathAtom {
                class_override: Some(AtomClass::Ord),
                width_em: None,
                ams_symbol: None,
                ..symbol("⊥".into(), span)
            },
            // `\bigtriangleup` renders `\triangle`'s exact glyph (U+25B3) but
            // is Bin where `\triangle` is Ord; same fix as `\bot`/`\perp`.
            "bigtriangleup" => MathAtom {
                class_override: Some(AtomClass::Bin),
                width_em: None,
                ams_symbol: None,
                ..symbol("△".into(), span)
            },
            // TeXbook Chapter 17's `\mathbin`/`\mathrel`/... family: the
            // argument is a full math list, boxed as one atom whose class is
            // forced regardless of what its own contents would imply.
            "mathbin" | "mathrel" | "mathord" | "mathop" | "mathopen" | "mathclose"
            | "mathpunct" => {
                let class = match name.as_str() {
                    "mathbin" => AtomClass::Bin,
                    "mathrel" => AtomClass::Rel,
                    "mathop" => AtomClass::Op,
                    "mathopen" => AtomClass::Open,
                    "mathclose" => AtomClass::Close,
                    "mathpunct" => AtomClass::Punct,
                    _ => AtomClass::Ord,
                };
                let body = self.required_group(&name, span);
                MathAtom {
                    nucleus: Nucleus::Group(body),
                    span,
                    superscript: None,
                    subscript: None,
                    class_override: Some(class),
                    width_em: None,
                    ams_symbol: None,
                }
            }
            // amsopn.sty: `\operatorname` is `\qopname\newmcodes@ o` (`\nolimits`),
            // `\operatorname*` is `\qopname\newmcodes@ m` (`\limits`).
            "operatorname" | "operatornamewithlimits" => {
                let starred = name == "operatornamewithlimits"
                    || matches!(self.tokens.get(self.i).map(|t| &t.kind), Some(TokenKind::Word(w)) if w == "*");
                self.skip_star();
                let body = self.required_group(&name, span);
                MathAtom {
                    nucleus: Nucleus::Operator {
                        body: operator_body(body),
                        limits: starred,
                    },
                    span,
                    superscript: None,
                    subscript: None,
                    class_override: None,
                    width_em: None,
                    ams_symbol: None,
                }
            }
            "rule" => {
                let raise = self.raw_bracket_text();
                let (width, width_span) = self.raw_group_text("rule", span);
                let (height, height_span) = self.raw_group_text("rule", span);
                let full = span.merge(width_span).merge(height_span);
                use crate::text_builtins::{TextDimen, TextRule};
                let dimens = (
                    raise
                        .as_deref()
                        .map_or(Some(TextDimen::zero()), TextDimen::parse),
                    TextDimen::parse(&width),
                    TextDimen::parse(&height),
                );
                match dimens {
                    (Some(raise), Some(width), Some(height)) => MathAtom {
                        nucleus: Nucleus::Rule(TextRule {
                            raise,
                            width,
                            height,
                        }),
                        span: full,
                        superscript: None,
                        subscript: None,
                        class_override: None,
                        width_em: None,
                        ams_symbol: None,
                    },
                    _ => {
                        self.diagnostics.push(Diagnostic::error(
                            format!(
                                "\\rule requires recognised dimensions, got [{}]{{{}}}{{{}}}",
                                raise.unwrap_or_default().trim(),
                                width.trim(),
                                height.trim()
                            ),
                            Some(full),
                            Some("omitted the rule and continued".into()),
                        ));
                        space(0.0, full)
                    }
                }
            }
            "phantom" | "hphantom" | "vphantom" => {
                let body = self.required_group(&name, span);
                MathAtom {
                    nucleus: Nucleus::Phantom {
                        body,
                        horizontal: name != "vphantom",
                        vertical: name != "hphantom",
                    },
                    span,
                    superscript: None,
                    subscript: None,
                    class_override: None,
                    width_em: None,
                    ams_symbol: None,
                }
            }
            "xrightarrow" | "xleftarrow" | "xleftrightarrow" => {
                let below = self
                    .optional_bracket_list()
                    .unwrap_or(MathList { atoms: Vec::new() });
                let above = self.required_group(&name, span);
                let arrow = match name.as_str() {
                    "xleftarrow" => ExtArrow::Left,
                    "xleftrightarrow" => ExtArrow::LeftRight,
                    _ => ExtArrow::Right,
                };
                MathAtom {
                    nucleus: Nucleus::ExtArrow {
                        arrow,
                        above,
                        below,
                    },
                    span,
                    superscript: None,
                    subscript: None,
                    class_override: None,
                    width_em: None,
                    ams_symbol: None,
                }
            }
            "substack" => {
                let rows = self.braced_rows(&name, span);
                MathAtom {
                    nucleus: Nucleus::SubArray { rows, align: 'c' },
                    span,
                    superscript: None,
                    subscript: None,
                    class_override: None,
                    width_em: None,
                    ams_symbol: None,
                }
            }
            // Upright roman is already the math default in this subset, and
            // the other style switches have no distinct face yet: keep the
            // argument's content rather than dropping or garbling it.
            "mathrm" | "mathit" | "mathsf" | "mathtt" | "mathnormal" | "boldsymbol" | "bm"
            | "mbox" | "hbox" | "textrm" | "textit" | "textnormal" => {
                // `\mathsf{AB}`, `\mathtt{T}`, `\mathit{diff}` with a plain
                // argument: the letters of that math alphabet (fontmath.ltx
                // `\DeclareMathAlphabet`: OT1 cmss/m/n, cmtt/m/n, cmr/m/it)
                // as Unicode mathematical alphanumerics in one atom, like
                // `\mathbb`. Any other argument keeps the surrounding math
                // letters.
                if name == "mathrm" && self.plain_text_argument() {
                    // fontmath.ltx: `\mathrm` is the `operators` font (OT1
                    // cmr/m/n), the upright roman `Text` sets, so `\mathrm{K}`
                    // is upright; math ignores the spaces in the argument.
                    let (text, argument_span) = self.required_text_group(&name, span);
                    let span = span.merge(argument_span);
                    let letters: String = text.chars().filter(|c| !c.is_whitespace()).collect();
                    if letters.is_empty() {
                        space(0.0, span)
                    } else {
                        text_atom(letters, span)
                    }
                } else if matches!(&*name, "mathit" | "mathsf" | "mathtt") && self.plain_text_argument() {
                    let (text, argument_span) = self.required_text_group(&name, span);
                    let span = span.merge(argument_span);
                    let glyphs: String = text
                        .chars()
                        .filter(|c| !c.is_whitespace())
                        .map(|c| math_alphabet_char(&name, c))
                        .collect();
                    if glyphs.is_empty() {
                        space(0.0, span)
                    } else {
                        symbol(glyphs, span)
                    }
                } else {
                    let body = self.required_group(&name, span);
                    self.group_atom(body, span)
                }
            }
            "displaystyle" | "textstyle" | "scriptstyle" | "scriptscriptstyle" | "nonumber"
            | "notag" | "middle" => space(0.0, span),
            // `\left`/`\right` stretch to their enclosed content at layout
            // time (`left_right_stretch_scales`); here they just record which
            // role they play so that pairing pass can find them.
            "left" | "right" => {
                let role = if name == "left" {
                    DelimiterRole::Left
                } else {
                    DelimiterRole::Right
                };
                left_right_delimiter(self.take_delimiter(&name, span), role)
            }
            "big" | "Big" | "bigg" | "Bigg" | "bigl" | "Bigl" | "biggl" | "Biggl" | "bigr"
            | "Bigr" | "biggr" | "Biggr" | "bigm" | "Bigm" | "biggm" | "Biggm" => {
                sized_delimiter(self.take_delimiter(&name, span), &name)
            }
            "dots" | "ldots" | "dotsc" | "dotso" => text_atom("...".into(), span),
            "cdots" | "dotsb" | "dotsm" | "dotsi" => symbol("⋅⋅⋅".into(), span),
            // Symbol has no U+222C/U+222D: repeated real integral glyphs.
            "iint" => symbol("∫∫".into(), span),
            "lbrace" => symbol("{".into(), span),
            "rbrace" => symbol("}".into(), span),
            "iiint" => symbol("∫∫∫".into(), span),
            // `\bmod` is not a bare `\mathbin` (`latex.ltx` 15703-15706):
            //
            //   \nonscript\mskip-\medmuskip\mkern5mu
            //   \mathbin{\operator@font mod}\penalty900
            //   \mkern5mu\nonscript\mskip-\medmuskip
            //
            // so the 4mu the Bin class contributes is cancelled and 5mu put in
            // its place — 1mu more on each side than the class alone, and the
            // same under amsmath, which does not touch it. The extra mu is
            // added around the Bin atom rather than replacing it, so TeX's
            // Bin-with-no-left-operand rule still applies and the boundary
            // cases come out right.
            //
            // Measured with TeX Live 2025 pdflatex at 10pt, `\mathrm{mod}`
            // being the same letters as an ordinary atom:
            //
            //   $a\bmod b$     34.29970   $a\mathrm{mod}b$   28.74428  (+10mu)
            //   $\bmod b$      24.56947   $\mathrm{mod}b$    23.45839  (+2mu)
            //   $\bmod$        20.27782   $\mathrm{mod}$     19.16673  (+2mu)
            //
            // and `$a\mkern5mu\mathrm{mod}\mkern5mu b$` is 34.29970 exactly.
            // The rows with 2mu are the Bin degrading to Ord at a boundary,
            // where only the explicit 5mu and the -4mu survive.
            //
            // Residual: the two `\nonscript`s drop the -4mu in script styles,
            // where the Bin class contributes nothing either, so pdflatex
            // keeps the full 5mu there and this keeps 1mu. That needs a
            // style-aware kern, which no atom here carries.
            "bmod" => {
                self.pending.push(text_atom("mod".into(), span));
                self.pending.push(space(BMOD_EXTRA_MU / 18.0, span));
                space(BMOD_EXTRA_MU / 18.0, span)
            }
            // amsmath's `\mod` (`amsmath.sty` 726-728) is a different command
            // with a different kern and no parentheses, and is undefined in
            // base LaTeX2e; it is left exactly as it was, with the rest of the
            // amsmath-provided constructs.
            "mod" => text_atom("mod".into(), span),
            // amsmath.sty lines 237-241: `\dfrac` = `\genfrac{}{}{}0`,
            // `\tfrac` = `\genfrac{}{}{}1`, `\binom` = `\genfrac()\z@{}`,
            // `\dbinom` = `\genfrac(){0pt}0`, `\tbinom` = `\genfrac(){0pt}1`.
            "dfrac" | "tfrac" | "binom" | "dbinom" | "tbinom" => {
                let numerator = self.required_group(&name, span);
                let denominator = self.required_group(&name, span);
                let binom = name.ends_with("binom");
                let style = match name.chars().next() {
                    Some('d') => Some(MathStyle::Display),
                    Some('t') => Some(MathStyle::Text),
                    _ => None,
                };
                gen_fraction(numerator, denominator, binom, style, span)
            }
            "genfrac" => {
                let (left, _) = self.required_text_group("genfrac", span);
                let (right, _) = self.required_text_group("genfrac", span);
                let (thickness, thickness_span) = self.required_text_group("genfrac", span);
                let (style, _) = self.required_text_group("genfrac", span);
                let numerator = self.required_group("genfrac", span);
                let denominator = self.required_group("genfrac", span);
                let thickness = thickness.trim();
                let thickness_pt = if thickness.is_empty() {
                    None
                } else if let Some(pt) = thickness
                    .strip_suffix("pt")
                    .and_then(|v| v.trim().parse::<f64>().ok())
                {
                    Some(pt)
                } else {
                    self.diagnostics.push(Diagnostic::error(
                        format!("\\genfrac thickness {thickness:?} is not a pt dimension"),
                        Some(thickness_span),
                        Some("used the default fraction rule thickness".into()),
                    ));
                    None
                };
                let delimiter = |s: String| match s.trim() {
                    "." => String::new(),
                    "\\{" | "\\lbrace" => "{".into(),
                    "\\}" | "\\rbrace" => "}".into(),
                    "\\langle" => "⟨".into(),
                    "\\rangle" => "⟩".into(),
                    "\\|" => "‖".into(),
                    other => other.to_string(),
                };
                MathAtom {
                    nucleus: Nucleus::GenFraction {
                        numerator,
                        denominator,
                        thickness_pt,
                        left: delimiter(left),
                        right: delimiter(right),
                        style: match style.trim() {
                            "0" => Some(MathStyle::Display),
                            "1" => Some(MathStyle::Text),
                            "2" => Some(MathStyle::Script),
                            "3" => Some(MathStyle::ScriptScript),
                            _ => None,
                        },
                    },
                    span,
                    superscript: None,
                    subscript: None,
                    class_override: None,
                    width_em: None,
                    ams_symbol: None,
                }
            }
            "cfrac" => self.command_atom("frac".into(), span),
            "frac" => {
                let numerator = self.required_group("frac", span);
                let denominator = self.required_group("frac", span);
                MathAtom {
                    nucleus: Nucleus::Fraction {
                        numerator,
                        denominator,
                    },
                    span,
                    superscript: None,
                    subscript: None,
                    class_override: None,
                    width_em: None,
                    ams_symbol: None,
                }
            }
            "begin" => self.grid_environment(span),
            "sqrt" => {
                let index = self.optional_bracket_list();
                let radical = MathAtom {
                    nucleus: Nucleus::Radical(self.required_group("sqrt", span)),
                    span,
                    superscript: None,
                    subscript: None,
                    class_override: None,
                    width_em: None,
                    ams_symbol: None,
                };
                match index {
                    // The root index sits as a raised script ahead of the sign.
                    Some(index) if !index.atoms.is_empty() => {
                        self.pending.push(radical);
                        MathAtom {
                            superscript: Some(index),
                            ..space(0.0, span)
                        }
                    }
                    _ => radical,
                }
            }
            "overset" | "stackrel" | "underset" => {
                let script = self.required_group(&name, span);
                let base = self.required_group(&name, span);
                let (over, under) = if name == "underset" {
                    (None, Some(script))
                } else {
                    (Some(script), None)
                };
                MathAtom {
                    nucleus: Nucleus::Stacked { base, over, under },
                    span,
                    superscript: None,
                    subscript: None,
                    class_override: None,
                    width_em: None,
                    ams_symbol: None,
                }
            }
            "mathbf" | "textbf" => {
                let (text, argument_span) = self.required_text_group(&name, span);
                MathAtom {
                    nucleus: Nucleus::Bold(text),
                    span: span.merge(argument_span),
                    superscript: None,
                    subscript: None,
                    class_override: None,
                    width_em: None,
                    ams_symbol: None,
                }
            }
            "boxed" | "overline" | "underline" | "overbrace" | "underbrace" | "overrightarrow"
            | "overleftarrow" | "overleftrightarrow" | "underrightarrow" | "underleftarrow"
            | "underleftrightarrow" => {
                let body = self.required_group(&name, span);
                let frame = match name.as_str() {
                    "boxed" => Frame::Box,
                    "overline" => Frame::Over,
                    "overbrace" => Frame::OverBrace,
                    "underbrace" => Frame::UnderBrace,
                    "overrightarrow" => Frame::OverRightArrow,
                    "overleftarrow" => Frame::OverLeftArrow,
                    "overleftrightarrow" => Frame::OverLeftRightArrow,
                    "underrightarrow" => Frame::UnderRightArrow,
                    "underleftarrow" => Frame::UnderLeftArrow,
                    "underleftrightarrow" => Frame::UnderLeftRightArrow,
                    _ => Frame::Under,
                };
                MathAtom {
                    nucleus: Nucleus::Framed { body, frame },
                    span,
                    superscript: None,
                    subscript: None,
                    class_override: None,
                    width_em: None,
                    ams_symbol: None,
                }
            }
            "tag" => {
                let starred = matches!(self.tokens.get(self.i).map(|t| &t.kind), Some(TokenKind::Word(w)) if w == "*");
                self.skip_star();
                let (text, argument_span) = self.required_text_group("tag", span);
                let label = if starred { text } else { format!("({text})") };
                self.pending
                    .push(text_atom(label, span.merge(argument_span)));
                text_space(2.0 * QUAD_EM, span)
            }
            // The kernel's `\pmod` (`latex.ltx` 15709) opens with
            // `\mkern18mu`; amsmath renews it through `\pod`, which is
            // `\if@display\mkern18mu\else\mkern8mu\fi` (`amsmath.sty`
            // 719-724), so **inline** it is 10mu narrower while display is
            // byte-identical. Both then set `(mod` + 6mu + the argument + `)`
            // — the kernel as `\,\,`, amsmath as `\mkern6mu`.
            //
            // Measured with TeX Live 2025 pdflatex at 10pt: `\hbox{$a\pmod
            // {y}$}` is 50.82503pt under the kernel and 45.26960pt under
            // amsmath, matching `$a\mkern18mu(\mathrm{mod}\mkern6mu y)$` and
            // `$a\mkern8mu(...)$` exactly; `\hbox{$\pmod{y}$}` alone is
            // 45.53914 against 39.98372, the same 10mu. Setting
            // `\@displaytrue` by hand puts amsmath back on 50.82503, which is
            // what makes the display case identical.
            //
            // This compiler has no display flag on the atom, so the inline
            // definition is the one that moves; every display formula reaches
            // the same 18mu it does today.
            "pmod" => {
                let body = self.required_group("pmod", span);
                self.pending.push(text_atom("(mod".into(), span));
                self.pending.push(space(6.0 / 18.0, span));
                self.pending.extend(body.atoms);
                self.pending.push(text_atom(")".into(), span));
                let opening = if self.packages.amsmath {
                    AMSMATH_POD_MU / 18.0
                } else {
                    QUAD_EM
                };
                space(opening, span)
            }
            // siunitx inside a formula (`crate::siunitx`).
            "num" | "qty" | "unit" | "si" | "SI" | "numlist" | "numrange" | "qtylist"
            | "qtyrange" | "SIlist" | "SIrange" | "ang" => self.siunitx(&name, span),
            "sisetup" => {
                let (keys, argument_span) = self.siunitx_raw_group().unwrap_or((String::new(), span));
                crate::siunitx::sisetup(&keys, span.merge(argument_span), self.diagnostics);
                space(0.0, span)
            }
            "text" => {
                let (text, argument_span) = self.required_text_group("text", span);
                MathAtom {
                    nucleus: Nucleus::Text(text),
                    span: span.merge(argument_span),
                    superscript: None,
                    subscript: None,
                    class_override: None,
                    width_em: None,
                    ams_symbol: None,
                }
            }
            "quad" => text_space(QUAD_EM, span),
            "qquad" => text_space(2.0 * QUAD_EM, span),
            "mathbb" => {
                let (text, argument_span) = self.required_text_group("mathbb", span);
                let span = span.merge(argument_span);
                let letters: String = text.chars().filter(|c| !c.is_whitespace()).collect();
                match letters
                    .chars()
                    .map(crate::lm_math::double_struck)
                    .collect::<Option<String>>()
                {
                    Some(glyphs) if !glyphs.is_empty() => symbol(glyphs, span),
                    _ => {
                        self.diagnostics.push(Diagnostic::error(
                            format!(
                                "\\mathbb supports only capital letters A-Z, not {:?}",
                                letters
                            ),
                            Some(span),
                            Some("typeset the argument without blackboard bold".into()),
                        ));
                        symbol(letters, span)
                    }
                }
            }
            // amsfonts.sty `\DeclareMathAlphabet{\mathfrak}{U}{euf}{m}{n}`:
            // Euler Fraktur letters as Unicode mathematical fraktur; digits
            // and other characters are kept as they are.
            "mathfrak" => {
                let (text, argument_span) = self.required_text_group("mathfrak", span);
                let span = span.merge(argument_span);
                let glyphs: String = text
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .map(|c| math_alphabet_char("mathfrak", c))
                    .collect();
                if glyphs.is_empty() {
                    space(0.0, span)
                } else {
                    symbol(glyphs, span)
                }
            }
            "mathcal" => {
                let (text, argument_span) = self.required_text_group("mathcal", span);
                let span = span.merge(argument_span);
                let letters: String = text.chars().filter(|c| !c.is_whitespace()).collect();
                match letters
                    .chars()
                    .map(crate::newcm_math::script)
                    .collect::<Option<String>>()
                {
                    Some(glyphs) if !glyphs.is_empty() => symbol(glyphs, span),
                    _ => {
                        self.diagnostics.push(Diagnostic::error(
                            format!(
                                "\\mathcal supports only capital letters A-Z, not {:?}",
                                letters
                            ),
                            Some(span),
                            Some("typeset the argument without calligraphic letters".into()),
                        ));
                        symbol(letters, span)
                    }
                }
            }
            // amssymb: msbm10 char "3F, 0.777781em (cmsy10's \emptyset is 0.5em).
            // Its own arm rather than a table row, so it needs its own gate;
            // `\emptyset`, the kernel's cmsy10 "3B, is not affected.
            "varnothing" if !self.packages.amssymb => {
                self.missing_package(&name, "amssymb", span)
            }
            "varnothing" => MathAtom {
                width_em: Some(VARNOTHING_MSBM_EM),
                ..symbol("∅".into(), span)
            },
            "hat" => self.accent_atom(Accent::Hat, span),
            "bar" => self.accent_atom(Accent::Bar, span),
            "vec" => self.accent_atom(Accent::Vec, span),
            "tilde" => self.accent_atom(Accent::Tilde, span),
            "dot" => self.accent_atom(Accent::Dot, span),
            "ddot" => self.accent_atom(Accent::Ddot, span),
            "check" => self.accent_atom(Accent::Check, span),
            "breve" => self.accent_atom(Accent::Breve, span),
            "acute" => self.accent_atom(Accent::Acute, span),
            "grave" => self.accent_atom(Accent::Grave, span),
            "widehat" => self.accent_atom(Accent::WideHat, span),
            "widetilde" => self.accent_atom(Accent::WideTilde, span),
            // The dashed arrows are drawn from msam pieces `amsfonts.sty`
            // declares, so without the package there is nothing to draw with
            // and pdflatex answers "Undefined control sequence".
            "dashrightarrow" | "dasharrow" | "dashleftarrow" if !self.packages.amsfonts => {
                self.missing_package(&name, "amsfonts", span)
            }
            // `\angle` and `\hbar` are the two commands of this inventory that
            // base LaTeX2e *does* define and amsfonts replaces with a single
            // msam/msbm glyph of different metrics, so an unloaded document
            // gets the kernel composite rather than an error. `fontmath.ltx`
            // 243 builds `\angle` from an `\ialign` of rules and 241 sets
            // `\hbar` as `\mathchar'26\mkern-9mu h`; measured with TeX Live
            // 2025 pdflatex at 10pt, `\hbox{$\angle$}` is 6.37344pt against
            // amssymb's 7.22223pt and `\hbox{$\hbar$}` 5.76172pt against
            // 5.40280pt. (The `\hbar` composite reproduces exactly: the
            // macron is 5.00002pt, `\mkern-9mu` is -4.99988pt and math italic
            // `h` is 5.76158pt.) `\rightleftharpoons` is the third such
            // command; this compiler has no row for it at all, so there is
            // nothing to gate yet.
            //
            // Only the advance is forced here: the nearest Latin Modern Math
            // character stays the glyph either way, because the kernel's
            // `\angle` is a rule drawing with no character to name and the
            // barred `h` is what U+210F already shows.
            "angle" | "hbar" if !self.packages.amsfonts => MathAtom {
                width_em: Some(if name == "angle" {
                    KERNEL_ANGLE_EM
                } else {
                    KERNEL_HBAR_EM
                }),
                ..symbol(
                    command_glyph(&name)
                        .expect("\\angle and \\hbar have kernel glyph rows")
                        .into(),
                    span,
                )
            },
            // amsfonts `\dashrightarrow` = `\mathrel{\dabar@\dabar@\mathchar"0\hexnumber@
            // \symAMSa 4B}` (`\dasharrow` its alias) and `\dashleftarrow` with the
            // "4C head first (`amsfonts.sty` 87-95).
            "dashrightarrow" | "dasharrow" | "dashleftarrow" => {
                let piece = |n: &str| {
                    ams_atom(
                        crate::amssymb::piece(n).expect("generated amssymb piece"),
                        span,
                    )
                };
                let atoms = if name == "dashleftarrow" {
                    vec![piece("dashleftarrow@"), piece("dabar@"), piece("dabar@")]
                } else {
                    vec![piece("dabar@"), piece("dabar@"), piece("dashrightarrow@")]
                };
                MathAtom {
                    nucleus: Nucleus::Group(MathList { atoms }),
                    class_override: Some(AtomClass::Rel),
                    ..symbol(String::new(), span)
                }
            }
            // amssymb/amsfonts symbols take precedence over the older glyph
            // rows for the same names (`\square`, `\nleq`, ...): they carry
            // the msam/msbm slot and declared class pdfLaTeX sets — but only
            // once the document has loaded the file that declares them.
            _ => match (crate::amssymb::by_name(&name), command_glyph(&name)) {
                (Some(ams), _) if self.packages.provides(ams) => ams_atom(ams, span),
                // Base LaTeX2e defines none of these 212 names (every one
                // probed with `\ifcsname` under TeX Live 2025), so pdflatex
                // answers "Undefined control sequence" and typesets nothing.
                // A `COMMAND_GLYPHS` row for the same name must not stand in
                // for the package: rendering it anyway is the divergence this
                // gate closes, and it was table-wide.
                (Some(ams), _) => {
                    let package = match ams.provider {
                        crate::amssymb::Provider::Amsfonts => "amsfonts",
                        crate::amssymb::Provider::Amssymb => "amssymb",
                    };
                    self.missing_package(&name, package, span)
                }
                (None, Some(glyph)) => symbol(glyph.into(), span),
                (None, None) => {
                    self.diagnostics.push(Diagnostic::command_error(
                        &name,
                        format!("\\{} is not supported in math mode", name),
                        Some(span),
                        Some("typeset the command literally and continued".into()),
                    ));
                    symbol(format!("\\{}", name), span)
                }
            },
        }
    }

    /// A siunitx command in math (`crate::siunitx::typeset`): the first atom
    /// is returned and the rest queued, so the output joins the formula.
    fn siunitx(&mut self, name: &str, span: Span) -> MathAtom {
        let Some((required, pre_unit_bracket)) = crate::siunitx::arity(name) else {
            return space(0.0, span);
        };
        let options = self.siunitx_raw_bracket();
        let mut full = options.as_ref().map_or(span, |(_, s)| span.merge(*s));
        let mut pre_unit = None;
        let mut args = Vec::with_capacity(required);
        for index in 0..required {
            if pre_unit_bracket && index == 1 {
                if let Some((raw, s)) = self.siunitx_raw_bracket() {
                    full = full.merge(s);
                    pre_unit = Some(raw);
                }
            }
            match self.siunitx_raw_group() {
                Some((raw, s)) => {
                    full = full.merge(s);
                    args.push(raw);
                }
                None => {
                    if !self.argument_cut_off() {
                        self.diagnostics.push(Diagnostic::error(
                            format!("\\{name} requires an argument"),
                            Some(span),
                            Some("used an empty argument and continued".into()),
                        ));
                    }
                    args.push(String::new());
                }
            }
        }
        let mut atoms = crate::siunitx::typeset(
            name,
            options.as_ref().map(|(o, _)| o.as_str()),
            pre_unit.as_deref(),
            &args,
            true,
            self.packages,
            full,
            self.diagnostics,
        )
        .into_iter();
        match atoms.next() {
            Some(first) => {
                self.pending.extend(atoms);
                first
            }
            None => space(0.0, full),
        }
    }

    /// A `[...]` siunitx argument as raw source, braces kept; nothing is
    /// consumed when no bracket follows.
    fn siunitx_raw_bracket(&mut self) -> Option<(String, Span)> {
        let tokens = self.tokens;
        let mut index = self.i;
        while matches!(tokens.get(index).map(|t| &t.kind), Some(TokenKind::Space)) {
            index += 1;
        }
        if !matches!(tokens.get(index).map(|t| &t.kind), Some(TokenKind::Word(w)) if w == "[") {
            return None;
        }
        let start = tokens[index].span;
        let mut depth = 0usize;
        for (offset, token) in tokens[index + 1..].iter().enumerate() {
            match &token.kind {
                TokenKind::Word(w) if w == "]" && depth == 0 => {
                    let inner = &tokens[index + 1..index + 1 + offset];
                    self.i = index + offset + 2;
                    return Some((crate::siunitx::raw_text(inner), start.merge(token.span)));
                }
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        None
    }

    /// A required siunitx argument as raw source: a braced group (outer
    /// braces removed) or a single token.
    fn siunitx_raw_group(&mut self) -> Option<(String, Span)> {
        let tokens = self.tokens;
        while matches!(tokens.get(self.i).map(|t| &t.kind), Some(TokenKind::Space)) {
            self.i += 1;
        }
        let open = tokens.get(self.i)?;
        match &open.kind {
            TokenKind::LBrace => {}
            TokenKind::Word(_) | TokenKind::Command(_) => {
                self.i += 1;
                return Some((crate::siunitx::raw_text([open]), open.span));
            }
            _ => return None,
        }
        let mut depth = 0usize;
        for (offset, token) in tokens[self.i..].iter().enumerate() {
            match &token.kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        let inner = &tokens[self.i + 1..self.i + offset];
                        self.i += offset + 1;
                        return Some((crate::siunitx::raw_text(inner), open.span.merge(token.span)));
                    }
                }
                _ => {}
            }
        }
        self.unclosed.get_or_insert(open.span);
        self.i = tokens.len();
        None
    }

    /// Returns the first atom of `body` and queues the rest, so the group
    /// flattens into the surrounding list exactly like a bare `{...}` group.
    fn group_atom(&mut self, body: MathList, span: Span) -> MathAtom {
        let mut atoms = body.atoms.into_iter();
        match atoms.next() {
            Some(first) => {
                self.pending.extend(atoms);
                first
            }
            None => space(0.0, span),
        }
    }

    /// `\rule`'s optional `[<raise>]` as raw text (control words kept).
    fn raw_bracket_text(&mut self) -> Option<String> {
        let mut cursor = self.i;
        while matches!(
            self.tokens.get(cursor).map(|t| &t.kind),
            Some(TokenKind::Space)
        ) {
            cursor += 1;
        }
        if !matches!(self.tokens.get(cursor).map(|t| &t.kind), Some(TokenKind::Word(w)) if w == "[")
        {
            return None;
        }
        let mut text = String::new();
        let mut end = cursor + 1;
        while let Some(token) = self.tokens.get(end) {
            match &token.kind {
                TokenKind::Word(w) if w == "]" => break,
                TokenKind::Word(w) => text.push_str(w),
                TokenKind::Command(name) => {
                    text.push('\\');
                    text.push_str(name);
                }
                TokenKind::Space => text.push(' '),
                _ => {}
            }
            end += 1;
        }
        if end >= self.tokens.len() {
            return None;
        }
        self.i = end + 1;
        Some(text)
    }

    /// A `\rule` dimension argument as raw text: `{\textwidth}` keeps its
    /// control word instead of being diagnosed as text-group content.
    fn raw_group_text(&mut self, command: &str, span: Span) -> (String, Span) {
        while matches!(
            self.tokens.get(self.i).map(|t| &t.kind),
            Some(TokenKind::Space)
        ) {
            self.i += 1;
        }
        match self.tokens.get(self.i).map(|t| &t.kind) {
            Some(TokenKind::LBrace) => {}
            _ => return self.required_text_group(command, span),
        }
        let open = self.tokens[self.i].span;
        self.i += 1;
        let mut depth = 1usize;
        let mut text = String::new();
        let mut end = open;
        while let Some(token) = self.tokens.get(self.i).cloned() {
            self.i += 1;
            end = token.span;
            match token.kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        return (text, open.merge(end));
                    }
                }
                TokenKind::Word(word) => text.push_str(&word),
                TokenKind::Command(name) => {
                    text.push('\\');
                    text.push_str(&name);
                }
                TokenKind::Space => text.push(' '),
                _ => {}
            }
        }
        self.diagnostics.push(Diagnostic::error(
            format!("\\{command} argument is missing its closing brace"),
            Some(open.merge(end)),
            Some("used the text up to the end of the formula".into()),
        ));
        (text, open.merge(end))
    }

    /// An optional `[...]` math argument, as in `\sqrt[n]{x}`.
    fn optional_bracket_list(&mut self) -> Option<MathList> {
        let mut cursor = self.i;
        while matches!(
            self.tokens.get(cursor).map(|t| &t.kind),
            Some(TokenKind::Space)
        ) {
            cursor += 1;
        }
        if !matches!(self.tokens.get(cursor).map(|t| &t.kind), Some(TokenKind::Word(w)) if w == "[")
        {
            return None;
        }
        let start = cursor + 1;
        let mut depth = 0usize;
        let mut end = start;
        while let Some(token) = self.tokens.get(end) {
            match &token.kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => depth = depth.saturating_sub(1),
                TokenKind::Word(w) if w == "]" && depth == 0 => break,
                _ => {}
            }
            end += 1;
        }
        if end >= self.tokens.len() {
            return None;
        }
        self.i = end + 1;
        Some(self.sub_list(&self.tokens[start..end]))
    }

    /// Parses a delimited sub-list (an optional argument, a grid cell). A
    /// group left open inside it closes at the sub-list's own delimiter.
    fn sub_list(&mut self, tokens: &[Token]) -> MathList {
        let mut parser = MathParser {
            tokens,
            i: 0,
            depth: self.depth,
            diagnostics: self.diagnostics,
            packages: self.packages,
            pending: Vec::new(),
            unclosed: None,
            cut_off: false,
        };
        let list = parser.list(false);
        if let Some(open) = parser.unclosed {
            self.diagnostics.push(Diagnostic::error(
                "math group is missing its closing brace",
                Some(open),
                Some("closed the group at the math delimiter".into()),
            ));
        }
        list
    }

    fn skip_star(&mut self) {
        if matches!(self.tokens.get(self.i).map(|t| &t.kind), Some(TokenKind::Word(w)) if w == "*")
        {
            self.i += 1;
        }
    }

    fn accent_atom(&mut self, accent: Accent, span: Span) -> MathAtom {
        let body = self.required_group(accent.command(), span);
        if accent.glyph().is_none() {
            self.diagnostics.push(Diagnostic::warning(
                format!(
                    "\\{} has no representable accent glyph in the compiler's base-14 fonts",
                    accent.command()
                ),
                Some(span),
                Some("typeset the base without the accent mark and continued".into()),
            ));
        }
        MathAtom {
            nucleus: Nucleus::Accent { accent, body },
            span,
            superscript: None,
            subscript: None,
            class_override: None,
            width_em: None,
            ams_symbol: None,
        }
    }

    fn take_delimiter(&mut self, command: &str, span: Span) -> MathAtom {
        while matches!(
            self.tokens.get(self.i).map(|t| &t.kind),
            Some(TokenKind::Space)
        ) {
            self.i += 1;
        }
        let Some(token) = self.tokens.get(self.i).cloned() else {
            if self.argument_cut_off() {
                return symbol(String::new(), span);
            }
            self.diagnostics.push(Diagnostic::error(
                format!("\\{command} requires a following delimiter"),
                Some(span),
                Some("used an empty delimiter and continued".into()),
            ));
            return symbol(String::new(), span);
        };
        if let TokenKind::Command(name) = &token.kind {
            let glyph = match name.as_str() {
                "lbrace" => Some("{"),
                "rbrace" => Some("}"),
                "vert" => Some("|"),
                "Vert" => Some("‖"),
                other => command_glyph(other).filter(|_| DELIMITER_COMMANDS.contains(&other)),
            };
            if let Some(glyph) = glyph {
                self.i += 1;
                return symbol(glyph.into(), span.merge(token.span));
            }
        }
        let TokenKind::Word(delimiter) = &token.kind else {
            self.diagnostics.push(Diagnostic::error(
                format!("\\{command} requires a following delimiter"),
                Some(span),
                Some("left the following non-delimiter token to be parsed normally".into()),
            ));
            return symbol(String::new(), span);
        };
        if delimiter == "." {
            // The null delimiter: an invisible fence (`\left.` / `\right.`).
            self.i += 1;
            return space(0.0, span.merge(token.span));
        }
        if delimiter == "|" && token.span.end - token.span.start == 2 {
            self.i += 1;
            return symbol("‖".into(), span.merge(token.span));
        }
        if delimiter.chars().count() != 1 || !"()[]{}|./<>".contains(delimiter.as_str()) {
            self.diagnostics.push(Diagnostic::error(
                format!("\\{command} does not support delimiter {delimiter:?}"),
                Some(span.merge(token.span)),
                Some("typeset the delimiter at ordinary size and continued".into()),
            ));
        }
        self.i += 1;
        symbol(delimiter.clone(), span.merge(token.span))
    }

    /// Whether the next argument is plain text: one word token, or a brace
    /// group of words and spaces only (no commands, scripts or groups).
    fn plain_text_argument(&self) -> bool {
        let mut i = self.i;
        while matches!(self.tokens.get(i).map(|t| &t.kind), Some(TokenKind::Space)) {
            i += 1;
        }
        match self.tokens.get(i).map(|t| &t.kind) {
            Some(TokenKind::Word(_)) => true,
            Some(TokenKind::LBrace) => {
                i += 1;
                loop {
                    match self.tokens.get(i).map(|t| &t.kind) {
                        Some(TokenKind::Word(_) | TokenKind::Space) => i += 1,
                        Some(TokenKind::RBrace) => return true,
                        _ => return false,
                    }
                }
            }
            _ => false,
        }
    }

    fn required_text_group(&mut self, command: &str, span: Span) -> (String, Span) {
        while matches!(
            self.tokens.get(self.i).map(|t| &t.kind),
            Some(TokenKind::Space)
        ) {
            self.i += 1;
        }
        let Some(open) = self.tokens.get(self.i).cloned() else {
            self.diagnostics.push(Diagnostic::error(
                format!("\\{command} requires an argument"),
                Some(span),
                Some("used an empty argument and continued".into()),
            ));
            return (String::new(), span);
        };
        if open.kind != TokenKind::LBrace {
            // TeX's undelimited argument: without a brace, the argument is
            // the next single token by itself -- one already-split character
            // (`\mathbb R`, `\mathbf v`) or one whole control sequence --
            // not a full group scan.
            return match open.kind {
                TokenKind::Word(ch) => {
                    self.i += 1;
                    (ch, open.span)
                }
                TokenKind::Command(name) => {
                    self.i += 1;
                    self.diagnostics.push(Diagnostic::error(
                        format!("\\{name} is not supported inside \\{command}"),
                        Some(open.span),
                        Some("typeset the command name literally and continued".into()),
                    ));
                    (format!("\\{name}"), open.span)
                }
                _ => {
                    self.diagnostics.push(Diagnostic::error(
                        format!("\\{command} requires an argument"),
                        Some(span),
                        Some("used an empty argument and continued".into()),
                    ));
                    (String::new(), span)
                }
            };
        }
        self.i += 1;
        let mut depth = 1usize;
        let mut text = String::new();
        let mut end = open.span;
        let mut after_comment = false;
        let mut depth_reported = false;
        while let Some(token) = self.tokens.get(self.i).cloned() {
            self.i += 1;
            end = token.span;
            match token.kind {
                TokenKind::LBrace => {
                    depth += 1;
                    if depth > MAX_MATH_DEPTH && !depth_reported {
                        depth_reported = true;
                        self.diagnostics.push(Diagnostic::error(
                            "text group nesting exceeds the supported math depth",
                            Some(token.span),
                            Some("continued bounded iterative recovery".into()),
                        ));
                    }
                }
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        return (text, open.span.merge(end));
                    }
                }
                TokenKind::Word(word) => text.push_str(&word),
                TokenKind::Space => {
                    if !after_comment {
                        text.push(' ');
                    }
                }
                TokenKind::Comment => {
                    after_comment = true;
                    continue;
                }
                TokenKind::ParBreak => {
                    self.diagnostics.push(Diagnostic::error(
                        format!("paragraph breaks are not supported inside \\{command}"),
                        Some(token.span),
                        Some("collapsed the paragraph break to one space and continued".into()),
                    ));
                    text.push(' ');
                }
                TokenKind::LineBreak => {
                    self.diagnostics.push(Diagnostic::error(
                        format!("line breaks are not supported inside \\{command}"),
                        Some(token.span),
                        Some("typeset the line-break command literally and continued".into()),
                    ));
                    text.push_str("\\\\");
                }
                TokenKind::Command(name) => {
                    self.diagnostics.push(Diagnostic::command_error(
                        &name,
                        format!("\\{name} is not supported inside \\{command}"),
                        Some(token.span),
                        Some("typeset the command name literally and continued".into()),
                    ));
                    text.push('\\');
                    text.push_str(&name);
                }
                TokenKind::Verb { .. } => {
                    self.diagnostics.push(Diagnostic::error(
                        format!("\\verb is not supported inside \\{command}"),
                        Some(token.span),
                        Some("ignored the \\verb and continued".into()),
                    ));
                }
                TokenKind::MathShift
                | TokenKind::DisplayMathOpen
                | TokenKind::DisplayMathClose
                | TokenKind::Superscript
                | TokenKind::Subscript => {
                    self.diagnostics.push(Diagnostic::error(
                        format!("math syntax is not supported inside \\{command}"),
                        Some(token.span),
                        Some("typeset the token literally and continued".into()),
                    ));
                    text.push_str(match token.kind {
                        TokenKind::MathShift => "$",
                        TokenKind::DisplayMathOpen => "\\[",
                        TokenKind::DisplayMathClose => "\\]",
                        TokenKind::Superscript => "^",
                        TokenKind::Subscript => "_",
                        _ => unreachable!(),
                    });
                }
            }
            after_comment = false;
        }
        self.diagnostics.push(Diagnostic::error(
            format!("argument to \\{command} is missing its closing brace"),
            Some(open.span),
            Some("closed the text argument at the math delimiter".into()),
        ));
        (text, open.span.merge(end))
    }

    /// Reads `{name}` after a `\begin` as plain characters.
    fn environment_name(&mut self) -> Option<String> {
        while matches!(
            self.tokens.get(self.i).map(|t| &t.kind),
            Some(TokenKind::Space)
        ) {
            self.i += 1;
        }
        if !matches!(
            self.tokens.get(self.i).map(|t| &t.kind),
            Some(TokenKind::LBrace)
        ) {
            return None;
        }
        let mut cursor = self.i + 1;
        let mut name = String::new();
        loop {
            match self.tokens.get(cursor).map(|t| &t.kind) {
                Some(TokenKind::Word(ch)) => name.push_str(ch),
                Some(TokenKind::RBrace) => break,
                _ => return None,
            }
            cursor += 1;
        }
        self.i = cursor + 1;
        Some(name)
    }

    /// `\begin{env} cell & cell \\ ... \end{env}` for the grid environments.
    fn grid_environment(&mut self, span: Span) -> MathAtom {
        let unsupported = |name: &str| format!("\\begin{{{name}}} is not supported in math mode");
        let Some(name) = self.environment_name() else {
            self.diagnostics.push(Diagnostic::error(
                "\\begin requires a braced environment name",
                Some(span),
                Some("typeset the command literally and continued".into()),
            ));
            return symbol("\\begin".into(), span);
        };
        let Some(&(_, default_align, left, right)) =
            GRID_ENVIRONMENTS.iter().find(|(env, ..)| *env == name)
        else {
            self.diagnostics.push(Diagnostic::environment_error(
                &name,
                unsupported(&name),
                Some(span),
                Some("typeset the environment body inline".into()),
            ));
            return symbol(String::new(), span);
        };
        let mut columns = String::new();
        // `array[t]{cc}`, `aligned[b]`, `gathered[c]`, `alignedat[t]{2}`: the
        // box-position argument (latex.ltx `\@array`, amsmath
        // `\ams@start@box`) comes before the preamble. It is not a cell; the
        // render pipeline reads the letter from the source at `\begin`.
        if matches!(name.as_str(), "array" | "aligned" | "alignedat" | "gathered") {
            let mut cursor = self.i;
            while matches!(self.tokens.get(cursor).map(|t| &t.kind), Some(TokenKind::Space)) {
                cursor += 1;
            }
            if matches!(self.tokens.get(cursor).map(|t| &t.kind), Some(TokenKind::Word(w)) if w == "[") {
                while let Some(t) = self.tokens.get(cursor) {
                    cursor += 1;
                    if matches!(&t.kind, TokenKind::Word(w) if w == "]") {
                        break;
                    }
                }
                self.i = cursor;
            }
        }
        if name == "array" {
            if let Some(TokenKind::LBrace) = self.tokens.get(self.i).map(|t| &t.kind) {
                self.i += 1;
                while let Some(token) = self.tokens.get(self.i) {
                    self.i += 1;
                    match &token.kind {
                        TokenKind::RBrace => break,
                        TokenKind::Word(ch) if matches!(ch.as_str(), "l" | "c" | "r") => {
                            columns.push_str(ch)
                        }
                        _ => {}
                    }
                }
            }
        }
        if name == "alignedat" {
            // The column-pair count argument; the grid sizes itself from cells.
            let _ = self.required_text_group("alignedat", span);
        }
        if matches!(name.as_str(), "aligned" | "alignedat" | "split") {
            columns = "rl".repeat(8);
        }
        let mut rows: Vec<Vec<Vec<Token>>> = vec![vec![Vec::new()]];
        let mut depth = 0usize;
        let mut nesting = 0usize;
        let mut closed = false;
        while let Some(token) = self.tokens.get(self.i).cloned() {
            self.i += 1;
            let top = depth == 0 && nesting == 0;
            match &token.kind {
                TokenKind::Command(command) if command == "begin" => nesting += 1,
                TokenKind::Command(command) if command == "end" => {
                    if nesting == 0 && depth == 0 {
                        let before = self.i;
                        if self.environment_name().as_deref() == Some(name.as_str()) {
                            closed = true;
                            break;
                        }
                        self.i = before;
                    }
                    nesting = nesting.saturating_sub(1);
                }
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => depth = depth.saturating_sub(1),
                _ => {}
            }
            let row = rows.last_mut().expect("at least one row");
            match &token.kind {
                TokenKind::LineBreak if top => {
                    // Skip an optional `[<length>]` row-spacing argument.
                    if matches!(&self.tokens.get(self.i).map(|t| &t.kind), Some(TokenKind::Word(w)) if w == "[")
                    {
                        while let Some(t) = self.tokens.get(self.i) {
                            self.i += 1;
                            if matches!(&t.kind, TokenKind::Word(w) if w == "]") {
                                break;
                            }
                        }
                    }
                    rows.push(vec![Vec::new()]);
                }
                TokenKind::Word(w) if top && w == "&" => row.push(Vec::new()),
                _ => row.last_mut().expect("at least one cell").push(token),
            }
        }
        if !closed {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "\\begin{{{name}}} has no matching \\end{{{name}}} in this math expression"
                ),
                Some(span),
                Some("closed the environment at the math delimiter".into()),
            ));
        }
        if rows.len() > 1
            && rows.last().is_some_and(|cells| {
                cells.iter().flatten().all(|t| {
                    matches!(
                        t.kind,
                        TokenKind::Space | TokenKind::Comment | TokenKind::ParBreak
                    )
                })
            })
        {
            rows.pop();
        }
        let rows = rows
            .into_iter()
            .map(|cells| cells.into_iter().map(|cell| self.sub_list(&cell)).collect())
            .collect::<Vec<Vec<MathList>>>();
        let width = rows.iter().map(Vec::len).max().unwrap_or(0);
        let mut columns: String = columns.chars().take(width).collect();
        while columns.chars().count() < width {
            columns.push(default_align);
        }
        MathAtom {
            nucleus: Nucleus::Matrix {
                rows,
                columns,
                left: left.into(),
                right: right.into(),
            },
            class_override: None,
            width_em: None,
            ams_symbol: None,
            span,
            superscript: None,
            subscript: None,
        }
    }

    /// A TeX "undelimited" math argument: `{...}` groups as a full list, or
    /// -- per TeX's actual grammar for a single argument -- the next token by
    /// itself: one already-split character (`\hat AB` accents only `A`,
    /// `\frac12` is 1 over 2) or one whole control sequence (`\hat\alpha`,
    /// `\vec\nabla`), skipping leading spaces. `self.atom()` is exactly the
    /// "parse one token into one atom" step the top-level list and
    /// `script_argument` already use for this same rule (`x^ab` = `x^a b`).
    /// Skips xcolor's `[model]` and one `{colour}` argument.
    fn skip_color_arguments(&mut self) {
        let space = |p: &Self| matches!(p.tokens.get(p.i).map(|t| &t.kind), Some(TokenKind::Space));
        while space(self) {
            self.i += 1;
        }
        if matches!(self.tokens.get(self.i).map(|t| &t.kind), Some(TokenKind::Word(w)) if w.starts_with('[')) {
            while self.i < self.tokens.len() {
                let closes = matches!(&self.tokens[self.i].kind, TokenKind::Word(w) if w.contains(']'));
                self.i += 1;
                if closes {
                    break;
                }
            }
            while space(self) {
                self.i += 1;
            }
        }
        if matches!(self.tokens.get(self.i).map(|t| &t.kind), Some(TokenKind::LBrace)) {
            let mut depth = 0usize;
            while self.i < self.tokens.len() {
                let kind = self.tokens[self.i].kind.clone();
                self.i += 1;
                match kind {
                    TokenKind::LBrace => depth += 1,
                    TokenKind::RBrace => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn required_group(&mut self, command: &str, span: Span) -> MathList {
        while matches!(
            self.tokens.get(self.i).map(|t| &t.kind),
            Some(TokenKind::Space)
        ) {
            self.i += 1;
        }
        if matches!(
            self.tokens.get(self.i).map(|t| &t.kind),
            Some(TokenKind::LBrace)
        ) {
            self.i += 1;
            return self.list(true);
        }
        if self.argument_cut_off() {
            return MathList { atoms: Vec::new() };
        }
        if let Some(atom) = self.atom() {
            let mut atoms = vec![atom];
            atoms.append(&mut self.pending);
            return MathList { atoms };
        }
        self.diagnostics.push(Diagnostic::error(
            format!("\\{} requires an argument", command),
            Some(span),
            Some("used an empty argument and continued".into()),
        ));
        MathList { atoms: Vec::new() }
    }
}

impl MathParser<'_> {
    /// The rows of a braced `\substack` argument, split at top-level `\\`
    /// (a trailing `\\` adds no row, as `\crcr` does not).
    fn braced_rows(&mut self, command: &str, span: Span) -> Vec<MathList> {
        while matches!(
            self.tokens.get(self.i).map(|t| &t.kind),
            Some(TokenKind::Space)
        ) {
            self.i += 1;
        }
        if !matches!(
            self.tokens.get(self.i).map(|t| &t.kind),
            Some(TokenKind::LBrace)
        ) {
            return vec![self.required_group(command, span)];
        }
        let start = self.i + 1;
        let mut depth = 0usize;
        let mut end = start;
        let mut breaks = Vec::new();
        while let Some(token) = self.tokens.get(end) {
            match &token.kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace if depth == 0 => break,
                TokenKind::RBrace => depth -= 1,
                TokenKind::LineBreak if depth == 0 => breaks.push(end),
                _ => {}
            }
            end += 1;
        }
        if end >= self.tokens.len() {
            self.diagnostics.push(Diagnostic::error(
                format!("\\{command} argument is missing its closing brace"),
                Some(span),
                Some("closed the argument at the end of the formula".into()),
            ));
        }
        self.i = (end + 1).min(self.tokens.len());
        let mut rows = Vec::new();
        let mut from = start;
        for at in breaks.into_iter().chain(std::iter::once(end)) {
            let row = self.sub_list(&self.tokens[from..at.min(self.tokens.len())]);
            rows.push(row);
            from = at + 1;
        }
        if rows.len() > 1 && rows.last().is_some_and(|r| r.atoms.is_empty()) {
            rows.pop();
        }
        rows
    }
}

/// amsmath's `\genfrac` shorthands: `\dfrac`/`\tfrac` (default rule, no
/// delimiters) and `\binom`/`\dbinom`/`\tbinom` (zero rule, parentheses).
fn gen_fraction(
    numerator: MathList,
    denominator: MathList,
    binom: bool,
    style: Option<MathStyle>,
    span: Span,
) -> MathAtom {
    let (thickness_pt, left, right) = if binom {
        (Some(0.0), "(", ")")
    } else {
        (None, "", "")
    };
    MathAtom {
        nucleus: Nucleus::GenFraction {
            numerator,
            denominator,
            thickness_pt,
            left: left.into(),
            right: right.into(),
            style,
        },
        span,
        superscript: None,
        subscript: None,
        class_override: None,
        width_em: None,
        ams_symbol: None,
    }
}

/// The body of `\operatorname{...}` in `\operator@font`: runs of single
/// characters become upright [`Nucleus::Text`]; math glue (`\,`) and any
/// other construct stay as they are.
fn operator_body(list: MathList) -> MathList {
    let mut atoms: Vec<MathAtom> = Vec::new();
    for atom in list.atoms {
        let plain = atom.superscript.is_none() && atom.subscript.is_none();
        match (&atom.nucleus, atoms.last_mut()) {
            (Nucleus::Symbol(s), Some(last))
                if plain
                    && s.chars().count() == 1
                    && matches!(&last.nucleus, Nucleus::Text(_))
                    && last.superscript.is_none()
                    && last.subscript.is_none() =>
            {
                if let Nucleus::Text(text) = &mut last.nucleus {
                    text.push_str(s);
                }
                last.span = last.span.merge(atom.span);
            }
            (Nucleus::Symbol(s), _) if plain && s.chars().count() == 1 => {
                atoms.push(MathAtom {
                    nucleus: Nucleus::Text(s.clone()),
                    ..atom
                });
            }
            _ => atoms.push(atom),
        }
    }
    MathList { atoms }
}

/// `\varnothing`'s advance in ems: msbm10.tfm character "3F (CHARWD R 0.777781).
pub(crate) const VARNOTHING_MSBM_EM: f64 = 0.777781;

/// The mu `\bmod` adds on each side beyond the 4mu of its Bin class: its
/// definition cancels `\medmuskip` and puts an explicit `\mkern5mu` there.
pub(crate) const BMOD_EXTRA_MU: f64 = 1.0;

/// The mu amsmath's `\pod` opens with in a non-display formula, against the
/// kernel's 18mu (`QUAD_EM`).
pub(crate) const AMSMATH_POD_MU: f64 = 8.0;

/// The kernel `\angle`'s advance in ems, without amsfonts: `fontmath.ltx` 243
/// builds it from an `\ialign` of rules, so it has no character and no font —
/// `\showthe\wd` of `\hbox{$\angle$}` is 6.37344pt at every size from 10pt,
/// and the same 6.37344pt in `\scriptstyle`, TeX Live 2025. amsfonts replaces
/// it with msam "5A, 0.722224em.
pub(crate) const KERNEL_ANGLE_EM: f64 = 0.637344;

/// The kernel `\hbar`'s advance in ems, without amsfonts: `fontmath.ltx` 241
/// is `{\mathchar'26\mkern-9mu h}` — the OT1 macron (0.500002em), `\mkern-9mu`
/// (-0.499988em) and math italic `h` (0.576158em), which sum to the 0.576172em
/// `\showthe\wd` of `\hbox{$\hbar$}` reports. amsfonts replaces it with msbm
/// "7E, 0.540280em.
pub(crate) const KERNEL_HBAR_EM: f64 = 0.576172;

/// The Unicode mathematical alphanumeric symbol that stands for `ch` in the
/// math alphabet of `command`: `\mathsf` sans-serif (U+1D5A0, digits
/// U+1D7E2), `\mathtt` monospace (U+1D670, digits U+1D7F6), `\mathit`
/// italic (U+1D434, `h` is U+210E) and `\mathfrak` fraktur (U+1D504, with
/// the Letterlike Symbols `C` U+212D, `H` U+210C, `I` U+2111, `R` U+211C,
/// `Z` U+2128). Characters the alphabet block has no code point for (italic
/// and fraktur digits, punctuation) are returned unchanged. The render
/// pipeline maps these back to the TeX fonts' slots.
pub fn math_alphabet_char(command: &str, ch: char) -> char {
    let offset = |base: u32, first: char| char::from_u32(base + (ch as u32 - first as u32));
    let mapped = match (command, ch) {
        ("mathsf", 'A'..='Z') => offset(0x1D5A0, 'A'),
        ("mathsf", 'a'..='z') => offset(0x1D5BA, 'a'),
        ("mathsf", '0'..='9') => offset(0x1D7E2, '0'),
        ("mathtt", 'A'..='Z') => offset(0x1D670, 'A'),
        ("mathtt", 'a'..='z') => offset(0x1D68A, 'a'),
        ("mathtt", '0'..='9') => offset(0x1D7F6, '0'),
        ("mathit", 'h') => Some('\u{210E}'),
        ("mathit", 'A'..='Z') => offset(0x1D434, 'A'),
        ("mathit", 'a'..='z') => offset(0x1D44E, 'a'),
        ("mathfrak", 'C') => Some('\u{212D}'),
        ("mathfrak", 'H') => Some('\u{210C}'),
        ("mathfrak", 'I') => Some('\u{2111}'),
        ("mathfrak", 'R') => Some('\u{211C}'),
        ("mathfrak", 'Z') => Some('\u{2128}'),
        ("mathfrak", 'A'..='Z') => offset(0x1D504, 'A'),
        ("mathfrak", 'a'..='z') => offset(0x1D51E, 'a'),
        _ => None,
    };
    mapped.unwrap_or(ch)
}

fn symbol(text: String, span: Span) -> MathAtom {
    MathAtom {
        nucleus: Nucleus::Symbol(text),
        span,
        superscript: None,
        subscript: None,
        class_override: None,
        width_em: None,
        ams_symbol: None,
    }
}

/// Scales a delimiter taken by `\big`..`\Biggm`. The null delimiter (a zero
/// space) and an empty recovery glyph are left as they are.
/// An amssymb/amsfonts symbol (`crate::amssymb`): its Unicode text as the
/// nucleus, the declared `\math<class>` forced, msam10/msbm10's character
/// width for this crate's own layout, and the table entry for TFM-driven
/// layouts, which box it from the AMS font metrics at the math size.
pub(crate) fn ams_atom(ams: &'static crate::amssymb::AmsSymbol, span: Span) -> MathAtom {
    use crate::amssymb::SymbolClass as C;
    MathAtom {
        class_override: Some(match ams.class {
            C::Ord => AtomClass::Ord,
            C::Bin => AtomClass::Bin,
            C::Rel => AtomClass::Rel,
            C::Open => AtomClass::Open,
            C::Close => AtomClass::Close,
        }),
        width_em: Some(ams.width_em),
        ams_symbol: Some(ams),
        ..symbol(ams.text.into(), span)
    }
}

fn sized_delimiter(mut atom: MathAtom, command: &str) -> MathAtom {
    let Nucleus::Symbol(glyph) = &atom.nucleus else {
        return atom;
    };
    if glyph.is_empty() {
        return atom;
    }
    let stem = command.trim_end_matches(['l', 'r', 'm']);
    let scale = match stem {
        "big" => 1.2,
        "Big" => 1.8,
        "bigg" => 2.4,
        _ => 3.0,
    };
    let role = match &command[stem.len()..] {
        "l" => DelimiterRole::Open,
        "r" => DelimiterRole::Close,
        "m" => DelimiterRole::Rel,
        _ => DelimiterRole::Ord,
    };
    atom.nucleus = Nucleus::SizedDelimiter {
        glyph: glyph.clone(),
        scale,
        role,
    };
    atom
}

/// Turns the atom `take_delimiter` produced for `\left`/`\right` into a
/// `SizedDelimiter` at ordinary (scale 1) size, ready for
/// `left_right_stretch_scales` to pair up and stretch at layout time.
///
/// `take_delimiter` returns either a `Symbol` (an ordinary glyph) or a
/// `Space { em: 0.0 }` (the null delimiter `\left.`/`\right.`, or the empty
/// recovery glyph after a parse error). Both become a `SizedDelimiter` here —
/// with an empty glyph for the space case — so the null delimiter still
/// participates in pairing and stretches its *partner* correctly, while
/// itself remaining invisible (an empty glyph draws nothing, at zero width).
fn left_right_delimiter(atom: MathAtom, role: DelimiterRole) -> MathAtom {
    let glyph = match &atom.nucleus {
        Nucleus::Symbol(glyph) => glyph.clone(),
        _ => String::new(),
    };
    MathAtom {
        nucleus: Nucleus::SizedDelimiter {
            glyph,
            scale: 1.0,
            role,
        },
        span: atom.span,
        superscript: atom.superscript,
        subscript: atom.subscript,
        class_override: atom.class_override,
        width_em: atom.width_em,
        ams_symbol: atom.ams_symbol,
    }
}

/// `\LaTeXe`'s `$_{\textstyle\varepsilon}$` subscript body as a one-atom
/// math list attributed to `span`, for layouts that set the logo's `ε` with
/// their own math fonts (`text_builtins::layout_logo` gives its position).
pub fn varepsilon_list(span: Span) -> MathList {
    let glyph = COMMAND_GLYPHS
        .iter()
        .find(|(name, _)| *name == "varepsilon")
        .map_or("\u{03B5}", |(_, glyph)| *glyph);
    MathList {
        atoms: vec![MathAtom {
            nucleus: Nucleus::Symbol(glyph.to_string()),
            span,
            superscript: None,
            subscript: None,
            class_override: None,
            width_em: None,
            ams_symbol: None,
        }],
    }
}

/// `\hskip<em>em`: glue in ems of the current text font (`\quad`).
fn text_space(em: f64, span: Span) -> MathAtom {
    MathAtom {
        nucleus: Nucleus::Space { em, font_em: true },
        ..space(0.0, span)
    }
}

fn space(em: f64, span: Span) -> MathAtom {
    MathAtom {
        nucleus: Nucleus::Space { em, font_em: false },
        span,
        superscript: None,
        subscript: None,
        class_override: None,
        width_em: None,
        ams_symbol: None,
    }
}

/// Every named symbol the math layer can emit, as (command, rendered glyph).
///
/// The export adapter in `crate::export` is tested against this exact table, so
/// adding a symbol here without giving it an export mapping fails the build's
/// tests rather than silently producing a glyph the PDF path turns into `?`.
pub const COMMAND_GLYPHS: &[(&str, &str)] = &[
    ("alpha", "α"),
    ("beta", "β"),
    ("gamma", "γ"),
    ("delta", "δ"),
    ("theta", "θ"),
    ("lambda", "λ"),
    ("mu", "μ"),
    ("pi", "π"),
    ("sigma", "σ"),
    ("phi", "φ"),
    ("omega", "ω"),
    // Handwritten-homework coverage (Adobe Symbol encodes every glyph below).
    // fontmath.ltx: `\epsilon` is cmmi "0F (the lunate ϵ, U+03F5) and
    // `\varepsilon` cmmi "22 (the open ε, U+03B5); TFM-driven layouts box
    // them from those slots. The base-14 Symbol export has only the open
    // form (0x65) and draws both with it (`export.rs`).
    ("epsilon", "\u{03F5}"),
    ("varepsilon", "\u{03B5}"),
    ("zeta", "ζ"),
    ("eta", "η"),
    ("vartheta", "ϑ"),
    ("iota", "ι"),
    ("kappa", "κ"),
    ("nu", "ν"),
    ("xi", "ξ"),
    ("varpi", "ϖ"),
    ("rho", "ρ"),
    ("varsigma", "ς"),
    ("tau", "τ"),
    ("upsilon", "υ"),
    ("varphi", "ϕ"),
    ("chi", "χ"),
    ("psi", "ψ"),
    ("Gamma", "Γ"),
    ("Delta", "Δ"),
    ("Theta", "Θ"),
    ("Lambda", "Λ"),
    ("Xi", "Ξ"),
    ("Pi", "Π"),
    ("Sigma", "Σ"),
    ("Upsilon", "Υ"),
    ("Phi", "Φ"),
    ("Psi", "Ψ"),
    ("Omega", "Ω"),
    ("le", "≤"),
    ("ge", "≥"),
    ("ne", "≠"),
    ("equiv", "≡"),
    ("sim", "∼"),
    ("cong", "≅"),
    ("propto", "∝"),
    ("perp", "⊥"),
    ("partial", "∂"),
    ("nabla", "∇"),
    ("prod", "∏"),
    ("ast", "∗"),
    ("prime", "′"),
    ("cup", "∪"),
    ("cap", "∩"),
    // The cmsy square relations are base LaTeX2e kernel symbols, not amssymb:
    // `fontmath.ltx` 279/278 declare `\sqcup`/`\sqcap` `\mathbin` at symbols
    // "74/"75 and 301/302 `\sqsubseteq`/`\sqsupseteq` `\mathrel` at "76/"77.
    // (amsfonts adds only the strict `\sqsubset`/`\sqsupset`, from msam.)
    // Verified with pdfTeX 3.141592653 (TeX Live 2025): `\show` gives
    // \mathchar"2274/"2275/"3276/"3277 both with and without amssymb, and at
    // 10pt against the 9.57755pt `$ab$` control, `$a\sqcup b$` is 20.68857pt
    // (glyph 6.66669pt + 8mu, so Bin) and `$a\sqsubseteq b$` 22.91077pt
    // (glyph 7.7778pt + 10mu, so Rel).
    ("sqcup", "⊔"),
    ("sqcap", "⊓"),
    ("subset", "⊂"),
    ("subseteq", "⊆"),
    ("sqsubseteq", "⊑"),
    ("supset", "⊃"),
    ("supseteq", "⊇"),
    ("sqsupseteq", "⊒"),
    ("notin", "∉"),
    ("ni", "∋"),
    ("emptyset", "∅"),
    ("varnothing", "∅"),
    ("oplus", "⊕"),
    ("otimes", "⊗"),
    ("odot", "⊙"),
    ("wedge", "∧"),
    ("land", "∧"),
    ("lor", "∨"),
    ("to", "→"),
    ("rightarrow", "→"),
    ("leftarrow", "←"),
    ("gets", "←"),
    ("uparrow", "↑"),
    ("downarrow", "↓"),
    ("leftrightarrow", "↔"),
    // `\implies`/`\impliedby`/`\iff` are handled in `command_atom`: they
    // expand to a thick space, a long double arrow, and another thick space
    // (matching mathtools/plain TeX), not a bare glyph, so they are not rows
    // here.
    ("Leftarrow", "⇐"),
    ("Leftrightarrow", "⇔"),
    ("Uparrow", "⇑"),
    ("Downarrow", "⇓"),
    ("therefore", "∴"),
    ("angle", "∠"),
    ("aleph", "ℵ"),
    ("Re", "ℜ"),
    ("Im", "ℑ"),
    ("wp", "℘"),
    ("langle", "〈"),
    ("rangle", "〉"),
    ("lvert", "∣"),
    ("rvert", "∣"),
    // `\|`/`\Vert`/`\lVert`/`\rVert` are U+2016 DOUBLE VERTICAL LINE, a
    // different symbol from `\mid`'s U+2223: plain.tex gives `\Vert` the
    // cmsy `"6B` small variant and the cmex `"0D` extensible recipe, where
    // `\mid` gets cmsy `"6A`/cmex `"0C`. They differ in width at every size
    // (5.00002/5.55557 pt against 2.77779/3.33333 at 10 pt), so spelling one
    // as two of the other is wrong in the box, not only in the ink. The
    // base-14 Symbol face has no double bar, but U+2016 is drawn from the
    // pinned Latin Modern Math resource (`crate::lm_math`) exactly as
    // `\parallel`'s U+2225 already is.
    ("lVert", "‖"),
    ("rVert", "‖"),
    ("times", "×"),
    ("div", "÷"),
    ("pm", "±"),
    ("leq", "≤"),
    ("geq", "≥"),
    ("neq", "≠"),
    ("approx", "≈"),
    ("cdot", "⋅"),
    ("infty", "∞"),
    ("sum", "∑"),
    ("int", "∫"),
    ("in", "∈"),
    ("forall", "∀"),
    ("exists", "∃"),
    ("vee", "∨"),
    ("Rightarrow", "⇒"),
    ("mid", "∣"),
    // Neither glyph exists in Symbol.afm; both are drawn from the pinned
    // Latin Modern Math resource (`crate::lm_math`), not approximated.
    ("setminus", "∖"),
    ("Longrightarrow", "⟹"),
    // amssymb/latexsym symbols below have no base-14 Symbol glyph either;
    // all are drawn from the pinned Latin Modern Math resource (see issue #62).
    ("mp", "∓"),
    ("ll", "≪"),
    ("gg", "≫"),
    ("simeq", "≃"),
    ("vdots", "⋮"),
    ("ddots", "⋱"),
    ("lfloor", "⌊"),
    ("rfloor", "⌋"),
    ("lceil", "⌈"),
    ("rceil", "⌉"),
    ("oint", "∮"),
    ("mapsto", "↦"),
    ("ell", "ℓ"),
    ("hbar", "ℏ"),
    ("circ", "∘"),
    ("parallel", "∥"),
    ("nmid", "∤"),
    ("nleq", "≰"),
    ("ngeq", "≱"),
    ("subsetneq", "⊊"),
    ("supsetneq", "⊋"),
    ("lesssim", "≲"),
    ("gtrsim", "≳"),
    ("triangleq", "≜"),
    ("coloneqq", "≔"),
    ("nexists", "∄"),
    ("complement", "∁"),
    ("rightsquigarrow", "⇝"),
    ("hookrightarrow", "↪"),
    ("leftrightarrows", "⇆"),
    ("models", "⊨"),
    ("vdash", "⊢"),
    ("dashv", "⊣"),
    ("top", "⊤"),
    ("measuredangle", "∡"),
    ("square", "□"),
    ("blacksquare", "■"),
    ("lozenge", "◊"),
    ("checkmark", "✓"),
    // HW2 follow-up (issue #62): the remaining long arrows, drawn from the
    // pinned Latin Modern Math resource like `\Longrightarrow` above.
    ("Longleftrightarrow", "⟺"),
    ("longrightarrow", "⟶"),
    ("longleftarrow", "⟵"),
    ("Longleftarrow", "⟸"),
    ("longleftrightarrow", "⟷"),
    // `\triangle`, also from the pinned Latin Modern Math resource.
    // `\bigtriangleup` shares this exact glyph with a forced Bin class (see
    // `command_atom`), so it is not a second row here.
    ("triangle", "△"),
    ("bigtriangledown", "▽"),
    // `\bot` shares `\perp`'s exact base-14 Symbol glyph above with a forced
    // Ord class (see `command_atom`), so it is not a second row here.
];

/// Named operators typeset as upright roman words (`\sin x`, `\lim_{x\to 0}`).
pub(crate) const OPERATOR_NAMES: &[&str] = &[
    "sin", "cos", "tan", "cot", "sec", "csc", "arcsin", "arccos", "arctan", "sinh", "cosh", "tanh",
    "coth", "log", "ln", "lg", "exp", "lim", "liminf", "limsup", "max", "min", "sup", "inf", "det",
    "gcd", "deg", "dim", "ker", "arg", "hom", "Pr", "sgn",
];

/// Named commands that `\left`, `\right` and `\big...` accept as fences.
pub(crate) const DELIMITER_COMMANDS: &[&str] = &[
    "langle",
    "rangle",
    "lvert",
    "rvert",
    "lVert",
    "rVert",
    "lbrace",
    "rbrace",
    "uparrow",
    "downarrow",
    "Uparrow",
    "Downarrow",
    "lfloor",
    "rfloor",
    "lceil",
    "rceil",
];

fn text_atom(text: String, span: Span) -> MathAtom {
    MathAtom {
        nucleus: Nucleus::Text(text),
        span,
        superscript: None,
        subscript: None,
        class_override: None,
        width_em: None,
        ams_symbol: None,
    }
}

/// The rule character used to draw fraction bars.
///
/// This is a stand-in, not a real glyph: runtime-v1 has no rule item type yet
/// (see issue #9). No font contains it, so it is deliberately unrepresentable in
/// the export adapter and is reported rather than silently substituted.
pub const FRACTION_RULE_CHAR: char = '\u{2500}';

/// The glyph a math-mode ASCII `-` renders as (U+2212, Symbol `minus`).
pub const MINUS_SIGN: &str = "\u{2212}";

fn command_glyph(name: &str) -> Option<&'static str> {
    COMMAND_GLYPHS
        .iter()
        .find(|(command, _)| *command == name)
        .map(|(_, glyph)| *glyph)
}

pub fn layout(list: &MathList, size: f64, diagnostics: &mut Vec<Diagnostic>) -> MathBox {
    layout_list(list, size, size, 0, diagnostics)
}

/// Display-style layout: scripts on `\lim`-like operators, `\sum` and `\prod`
/// at the top level stack centred above and below the operator, as in TeX.
pub fn layout_display(list: &MathList, size: f64, diagnostics: &mut Vec<Diagnostic>) -> MathBox {
    layout_list_with(list, size, size, 0, true, diagnostics)
}

/// Operators whose display-style scripts become limits.
fn takes_display_limits(nucleus: &Nucleus) -> bool {
    match nucleus {
        Nucleus::Text(name) => matches!(
            name.as_str(),
            "lim" | "liminf" | "limsup" | "max" | "min" | "sup" | "inf" | "det" | "gcd" | "Pr"
        ),
        Nucleus::Symbol(glyph) => matches!(glyph.as_str(), "∑" | "∏"),
        _ => false,
    }
}

/// TeX's atom classes (TeXbook Chapter 17), which drive inter-atom spacing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomClass {
    Ord,
    Op,
    Bin,
    Rel,
    Open,
    Close,
    Punct,
    Inner,
}

/// The class of `atom`, or `None` for explicit glue (`\,`, `\quad`), which
/// TeX skips when pairing atoms for spacing.
///
/// The parser does not keep TeX's class through `\left`/`\right` or
/// `\operatorname`, so a fence is classified by its glyph (open/close, not
/// inner) — including the invisible null delimiter, which is still an
/// `Open`/`Close` atom for spacing purposes even though it draws nothing —
/// and `\operatorname{...}` text is ordinary. `atom.class_override` (set by
/// `\bot`, `\bigtriangleup`, and the `\mathbin`-family commands) wins over
/// any of that, including the explicit-glue case above, since an atom with
/// a forced class is never the invisible glue those commands produce.
fn atom_class(atom: &MathAtom) -> Option<AtomClass> {
    use AtomClass::*;
    if let Some(class) = atom.class_override {
        return Some(class);
    }
    Some(match &atom.nucleus {
        Nucleus::Space { .. } if atom.superscript.is_none() && atom.subscript.is_none() => {
            return None
        }
        Nucleus::Symbol(glyph) => symbol_class(glyph),
        Nucleus::SizedDelimiter { role, .. } => match role {
            DelimiterRole::Ord => Ord,
            DelimiterRole::Open | DelimiterRole::Left => Open,
            DelimiterRole::Close | DelimiterRole::Right => Close,
            DelimiterRole::Rel => Rel,
        },
        Nucleus::Text(text) if OPERATOR_NAMES.contains(&text.as_str()) => Op,
        Nucleus::Text(text) if text == "mod" => Bin,
        Nucleus::Text(text) if text == "..." => Inner,
        Nucleus::Fraction { .. } => Inner,
        Nucleus::Operator { .. } => Op,
        // `\ext@arrow` is `\mathrel{\mathop{...}\limits...}`.
        Nucleus::ExtArrow { .. } => Rel,
        // `\overbrace`/`\underbrace` are `\mathop{..}\limits` (`fontmath.ltx` 430-437).
        Nucleus::Framed {
            frame: Frame::OverBrace | Frame::UnderBrace,
            ..
        } => Op,
        Nucleus::Matrix { left, right, .. } if !left.is_empty() || !right.is_empty() => Inner,
        // amsmath's `\overset`/`\stackrel` keep a relation or binary base's class.
        Nucleus::Stacked { base, .. } if base.atoms.len() == 1 => {
            match atom_class(&base.atoms[0]) {
                Some(class @ (Rel | Bin)) => class,
                _ => Ord,
            }
        }
        _ => Ord,
    })
}

fn symbol_class(glyph: &str) -> AtomClass {
    use AtomClass::*;
    match glyph {
        "=" | "<" | ">" | ":" | "≤" | "≥" | "≠" | "≈" | "≡" | "∼" | "≅" | "∝" | "⊥" | "∈" | "∉"
        | "∋" | "⊂" | "⊆" | "⊃" | "⊇" | "∣" | "→" | "←" | "↔" | "⇒" | "⇐" | "⇔" | "⟹" | "↑"
        | "↓" | "⇑" | "⇓" | "∴"
        // amssymb/latexsym relations, all drawn from the pinned Latin Modern
        // Math resource (`crate::lm_math`).
        | "≪" | "≫" | "≃" | "↦" | "∥" | "∤" | "≰" | "≱" | "⊊" | "⊋" | "≲" | "≳" | "≜" | "≔"
        | "⇝" | "↪" | "⇆" | "⊨" | "⊢" | "⊣"
        // HW2 follow-up: the remaining long arrows (issue #62), also from the
        // pinned Latin Modern Math resource. `⊥` above is `\perp`'s glyph;
        // `\bot` shares it but overrides the class to Ord (see `command_atom`).
        | "⟺" | "⟶" | "⟵" | "⟸" | "⟷"
        // fontmath.ltx 301-302: `\sqsubseteq`/`\sqsupseteq`, `\mathrel` at
        // cmsy "76/"77 (kernel, not amssymb).
        | "⊑" | "⊒" => Rel,
        "+" | "-" | "−" | "*" | "±" | "×" | "÷" | "⋅" | "·" | "∗" | "∪" | "∩" | "∨" | "∧" | "⊕"
        | "⊗" | "⊙" | "∖" | "∓" | "∘"
        // fontmath.ltx 278-279: `\sqcap`/`\sqcup`, `\mathbin` at cmsy "75/"74.
        | "⊓" | "⊔"
        // `\bigtriangledown`; `\bigtriangleup` shares `\triangle`'s glyph
        // (Ord by default here) and overrides its class to Bin instead.
        | "▽" => Bin,
        "(" | "[" | "{" | "〈" | "⟨" | "⌊" | "⌈" => Open,
        ")" | "]" | "}" | "〉" | "⟩" | "!" | "?" | "⌋" | "⌉" => Close,
        "," | ";" => Punct,
        "∑" | "∏" | "∫" | "∫∫" | "∫∫∫" | "∮" => Op,
        "⋅⋅⋅" => Inner,
        _ => Ord,
    }
}

/// Resolves each atom's class for spacing: TeX turns a binary operator with
/// no left operand (list start, or after Bin/Op/Rel/Open/Punct) into Ord, and
/// likewise one directly followed by Rel/Close/Punct or ending the list.
fn spacing_classes(list: &MathList) -> Vec<Option<AtomClass>> {
    use AtomClass::*;
    let mut classes: Vec<Option<AtomClass>> = list.atoms.iter().map(atom_class).collect();
    let mut previous: Option<usize> = None;
    for i in 0..classes.len() {
        let Some(class) = classes[i] else { continue };
        let before = previous.and_then(|p| classes[p]);
        match class {
            Bin if matches!(before, None | Some(Bin | Op | Rel | Open | Punct)) => {
                classes[i] = Some(Ord)
            }
            Rel | Close | Punct if before == Some(Bin) => classes[previous.unwrap()] = Some(Ord),
            _ => {}
        }
        previous = Some(i);
    }
    if let Some(last) = previous {
        if classes[last] == Some(Bin) {
            classes[last] = Some(Ord);
        }
    }
    classes
}

/// The TeXbook Chapter 18 spacing table, in mu (thin 3, medium 4, thick 5).
/// Entries TeX parenthesises apply only in display and text styles.
fn inter_atom_mu(left: AtomClass, right: AtomClass, script: bool) -> f64 {
    use AtomClass::*;
    let (mu, text_styles_only) = match (left, right) {
        (Ord | Close, Op) | (Op, Ord | Op) | (Inner, Op) => (3.0, false),
        (Ord | Op | Close | Inner, Bin) | (Bin, Ord | Op | Open | Inner) => (4.0, true),
        (Ord | Op | Close | Inner, Rel) | (Rel, Ord | Op | Open | Inner) => (5.0, true),
        (Ord | Op | Close, Inner) | (Inner, Ord | Open | Punct | Inner) => (3.0, true),
        (Punct, Ord | Op | Rel | Open | Close | Punct | Inner) => (3.0, true),
        _ => (0.0, false),
    };
    if script && text_styles_only {
        0.0
    } else {
        mu
    }
}

fn layout_list(
    list: &MathList,
    size: f64,
    root_size: f64,
    level: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> MathBox {
    layout_list_with(list, size, root_size, level, false, diagnostics)
}

fn layout_list_with(
    list: &MathList,
    size: f64,
    root_size: f64,
    level: usize,
    display: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> MathBox {
    let mut out = MathBox {
        items: Vec::new(),
        width: 0.0,
        ascent: size,
        descent: 0.2 * size,
    };
    let delimiter_scales = left_right_stretch_scales(list, size, root_size, level, display);
    let classes = spacing_classes(list);
    let mut previous_class = None;
    for (index, (atom, class)) in list.atoms.iter().zip(classes).enumerate() {
        if let Some(class) = class {
            if let Some(previous) = previous_class {
                // Scripts and fraction parts are the only lists laid out
                // below level 0, so `level > 0` is TeX's script style.
                out.width += inter_atom_mu(previous, class, level > 0) / 18.0 * size;
            }
            previous_class = Some(class);
        }
        // A matched `\left`/`\right` gets its computed stretch substituted
        // in for this nucleus only; everything else about the atom (its
        // scripts, its class) is untouched.
        let stretched;
        let atom_for_nucleus = match delimiter_scales[index] {
            Some(scale) => {
                stretched = with_delimiter_scale(atom, scale);
                &stretched
            }
            None => atom,
        };
        let mut nucleus = layout_nucleus(atom_for_nucleus, size, root_size, level, diagnostics);
        if display
            && level == 0
            && (atom.superscript.is_some() || atom.subscript.is_some())
            && takes_display_limits(&atom.nucleus)
        {
            let script_size = root_size * SCRIPT_SCALE;
            let sup = atom
                .superscript
                .as_ref()
                .map(|l| layout_list(l, script_size, root_size, level + 1, diagnostics));
            let sub = atom
                .subscript
                .as_ref()
                .map(|l| layout_list(l, script_size, root_size, level + 1, diagnostics));
            let width = [Some(&nucleus), sup.as_ref(), sub.as_ref()]
                .into_iter()
                .flatten()
                .map(|b| b.width)
                .fold(0.0, f64::max);
            offset_items(
                &mut nucleus.items,
                out.width + (width - nucleus.width) / 2.0,
                0.0,
            );
            out.ascent = out.ascent.max(nucleus.ascent);
            out.descent = out.descent.max(nucleus.descent);
            out.items.extend(nucleus.items);
            // Limit baselines clear the operator's cap height / descender by
            // a small gap; box ascents include font-size headroom.
            if let Some(mut b) = sup {
                let dy = -0.8 * size - 0.12 * size - b.descent;
                offset_items(&mut b.items, out.width + (width - b.width) / 2.0, dy);
                out.ascent = out.ascent.max(b.ascent - dy);
                out.items.extend(b.items);
            }
            if let Some(mut b) = sub {
                let dy = 0.2 * size + 0.12 * size + 0.75 * script_size;
                offset_items(&mut b.items, out.width + (width - b.width) / 2.0, dy);
                out.descent = out.descent.max(b.descent + dy);
                out.items.extend(b.items);
            }
            out.width += width;
            continue;
        }
        let nucleus_width = nucleus.width;
        offset_items(&mut nucleus.items, out.width, 0.0);
        out.ascent = out.ascent.max(nucleus.ascent);
        out.descent = out.descent.max(nucleus.descent);
        out.items.extend(nucleus.items);

        let script_size = if level == 0 {
            root_size * SCRIPT_SCALE
        } else {
            root_size * SECOND_ORDER_SCRIPT_SCALE
        };
        let mut script_width: f64 = 0.0;
        if let Some(sup) = &atom.superscript {
            let mut b = layout_list(sup, script_size, root_size, level + 1, diagnostics);
            let dy = -SUPERSCRIPT_RAISE_EM * size;
            offset_items(&mut b.items, out.width + nucleus_width, dy);
            out.ascent = out.ascent.max(b.ascent - dy);
            script_width = script_width.max(b.width);
            out.items.extend(b.items);
        }
        if let Some(sub) = &atom.subscript {
            let mut b = layout_list(sub, script_size, root_size, level + 1, diagnostics);
            let dy = SUBSCRIPT_LOWER_EM * size;
            offset_items(&mut b.items, out.width + nucleus_width, dy);
            out.descent = out.descent.max(b.descent + dy);
            script_width = script_width.max(b.width);
            out.items.extend(b.items);
        }
        out.width += nucleus_width + script_width;
    }
    out
}

/// Pairs each `\left` in `list` with the `\right` at the same nesting depth
/// (a stack of open indices, like matching parentheses) and computes TeX's
/// rule-19 stretch scale for every matched pair from the atoms strictly
/// between them, laid out at the same size/root size/level/display as `list`
/// itself so nested `\left`/`\right` and display-style limits measure the
/// same way they will actually render.
///
/// The returned vector has one slot per atom in `list`; a matched delimiter's
/// slot holds its scale, everything else is `None`. An unmatched `\left` or
/// `\right` — a stray opener, or one half of a pair split across `&`/rows
/// before it ever reaches this list — is left `None` and stays at its parsed
/// scale of 1, the same as plain TeX leaves a runaway fence alone rather than
/// guessing a size for it.
fn left_right_stretch_scales(
    list: &MathList,
    size: f64,
    root_size: f64,
    level: usize,
    display: bool,
) -> Vec<Option<f64>> {
    let mut scales = vec![None; list.atoms.len()];
    let mut open: Vec<usize> = Vec::new();
    for (index, atom) in list.atoms.iter().enumerate() {
        let Nucleus::SizedDelimiter { role, .. } = &atom.nucleus else {
            continue;
        };
        match role {
            DelimiterRole::Left => open.push(index),
            DelimiterRole::Right => {
                let Some(left) = open.pop() else { continue };
                let content = MathList {
                    atoms: list.atoms[left + 1..index].to_vec(),
                };
                // Discard: the real diagnostics for these atoms are emitted
                // once, by the atom-by-atom pass below that actually lays
                // this list out.
                let mut scratch = Vec::new();
                let content_box =
                    layout_list_with(&content, size, root_size, level, display, &mut scratch);
                let scale = delimiter_stretch_scale(&content_box, size);
                scales[left] = Some(scale);
                scales[index] = Some(scale);
            }
            DelimiterRole::Ord
            | DelimiterRole::Open
            | DelimiterRole::Close
            | DelimiterRole::Rel => {}
        }
    }
    scales
}

/// TeX's rule for sizing a `\left`/`\right` pair to its content (TeXbook
/// Appendix G, rule 19): given the enclosed material's ascent `a` and depth
/// `d`, let `h = 2 * max(a - axis, d + axis)` — twice the larger of the two
/// half-heights measured from the math axis — and stretch the delimiter to
/// at least `max(delimiterfactor * h, h - delimitershortfall)`, TeX's
/// `\delimiterfactor` (901/1000) and `\delimitershortfall` (5pt by default).
///
/// This compiler has no absolute-length glue yet, so shortfall is taken as
/// `0.5 * size` instead of a fixed 5pt, keeping the same proportion at
/// ordinary text sizes.
///
/// `a` is corrected by `a_eff = a - 0.3 * size` before that: this compiler's
/// ordinary boxes carry far more headroom above their ink than cmex10's (a
/// bare letter/symbol box is `ascent == size, descent == 0.2 * size`), so
/// using `a` directly would stretch even `\left( x \right)`. For that
/// baseline case (`a == size`, `d == 0.2 * size`) `a_eff` makes both
/// half-heights equal (`0.45 * size`) and `needed` comes out just under
/// `size`, so `scale` is exactly 1 — the calibration this constant is
/// chosen for.
fn delimiter_stretch_scale(content: &MathBox, size: f64) -> f64 {
    if size <= 0.0 {
        return 1.0;
    }
    let axis = MATH_AXIS_EM * size;
    let a_eff = content.ascent - 0.3 * size;
    let h = 2.0 * (a_eff - axis).max(content.descent + axis);
    let needed = (h * 0.901).max(h - 0.5 * size);
    (needed / size).max(1.0)
}

/// Substitutes `scale` into a `SizedDelimiter` nucleus, keeping its glyph,
/// role, span and scripts. Used only for a matched `\left`/`\right` pair, and
/// only for laying out those two atoms; the atoms stored in the `MathList`
/// itself are never mutated.
fn with_delimiter_scale(atom: &MathAtom, scale: f64) -> MathAtom {
    let (glyph, role) = match &atom.nucleus {
        Nucleus::SizedDelimiter { glyph, role, .. } => (glyph.clone(), *role),
        _ => return atom.clone(),
    };
    MathAtom {
        nucleus: Nucleus::SizedDelimiter { glyph, scale, role },
        span: atom.span,
        superscript: atom.superscript.clone(),
        subscript: atom.subscript.clone(),
        class_override: atom.class_override,
        width_em: atom.width_em,
        ams_symbol: atom.ams_symbol,
    }
}

fn layout_nucleus(
    atom: &MathAtom,
    size: f64,
    root_size: f64,
    level: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> MathBox {
    match &atom.nucleus {
        // TeX's math `-` is the minus sign (Symbol `minus`), not a hyphen.
        Nucleus::Symbol(text) if text == "-" => layout_nucleus(
            &MathAtom {
                nucleus: Nucleus::Symbol(MINUS_SIGN.into()),
                span: atom.span,
                superscript: None,
                subscript: None,
                class_override: atom.class_override,
                width_em: atom.width_em,
                ams_symbol: atom.ams_symbol,
            },
            size,
            root_size,
            level,
            diagnostics,
        ),
        Nucleus::Symbol(text) | Nucleus::Text(text) => MathBox {
            items: vec![MathItem {
                font: matches!(atom.nucleus, Nucleus::Text(_))
                    .then_some(crate::layout::Font::TimesRoman),
                text: text.clone(),
                x: 0.0,
                baseline: 0.0,
                size,
                span: atom.span,
                rule: None,
            }],
            width: match (
                &atom.nucleus,
                atom.width_em.map(|em| em * size).or_else(|| {
                    crate::lm_math::width_pt(text, size)
                        .or_else(|| crate::newcm_math::width_pt(text, size))
                }),
            ) {
                (Nucleus::Symbol(_), Some(width)) => width,
                _ => {
                    crate::layout::shaped_width(
                        text,
                        size,
                        if matches!(atom.nucleus, Nucleus::Text(_)) {
                            crate::layout::Font::TimesRoman
                        } else {
                            crate::layout::math_font(text)
                        },
                        atom.span,
                        diagnostics,
                    )
                    .0
                }
            },
            ascent: size,
            descent: 0.2 * size,
        },
        Nucleus::SizedDelimiter { glyph, scale, .. } => {
            let glyph_size = size * scale;
            let width = match crate::lm_math::width_pt(glyph, glyph_size) {
                Some(width) => width,
                None => {
                    crate::layout::shaped_width(
                        glyph,
                        glyph_size,
                        crate::layout::math_font(glyph),
                        atom.span,
                        diagnostics,
                    )
                    .0
                }
            };
            // An ordinary delimiter's centre already sits on the axis, so the
            // scaled glyph is lowered by the growth of that centre height.
            // The box spans `scale` ems centred on the axis, as cmex10's do:
            // `\big` at 10pt is 8.5pt high and 3.5pt deep.
            let half = 0.5 * glyph_size;
            MathBox {
                items: vec![MathItem {
                    font: None,
                    text: glyph.clone(),
                    x: 0.0,
                    baseline: MATH_AXIS_EM * (glyph_size - size),
                    size: glyph_size,
                    span: atom.span,
                    rule: None,
                }],
                width,
                ascent: (MATH_AXIS_EM * size + half).max(size),
                descent: (half - MATH_AXIS_EM * size).max(0.2 * size),
            }
        }
        Nucleus::Bold(text) => MathBox {
            items: vec![MathItem {
                font: Some(crate::layout::Font::TimesBold),
                text: text.clone(),
                x: 0.0,
                baseline: 0.0,
                size,
                span: atom.span,
                rule: None,
            }],
            width: crate::layout::shaped_width(
                text,
                size,
                crate::layout::Font::TimesBold,
                atom.span,
                diagnostics,
            )
            .0,
            ascent: size,
            descent: 0.2 * size,
        },
        Nucleus::Stacked { base, over, under } => {
            let script_size = if level == 0 {
                root_size * SCRIPT_SCALE
            } else {
                root_size * SECOND_ORDER_SCRIPT_SCALE
            };
            let mut b = layout_list(base, size, root_size, level, diagnostics);
            let over = over
                .as_ref()
                .map(|l| layout_list(l, script_size, root_size, level + 1, diagnostics));
            let under = under
                .as_ref()
                .map(|l| layout_list(l, script_size, root_size, level + 1, diagnostics));
            let width = [Some(&b), over.as_ref(), under.as_ref()]
                .into_iter()
                .flatten()
                .map(|m| m.width)
                .fold(0.0, f64::max);
            offset_items(&mut b.items, (width - b.width) / 2.0, 0.0);
            let mut out = MathBox {
                items: b.items,
                width,
                ascent: b.ascent,
                descent: b.descent,
            };
            if let Some(mut m) = over {
                let dy = -0.75 * size - 0.1 * size - m.descent;
                offset_items(&mut m.items, (width - m.width) / 2.0, dy);
                out.ascent = out.ascent.max(m.ascent - dy);
                out.items.extend(m.items);
            }
            if let Some(mut m) = under {
                let dy = 0.2 * size + 0.1 * size + 0.75 * script_size;
                offset_items(&mut m.items, (width - m.width) / 2.0, dy);
                out.descent = out.descent.max(m.descent + dy);
                out.items.extend(m.items);
            }
            out
        }
        Nucleus::Framed { body, frame } => {
            let mut b = layout_list(body, size, root_size, level, diagnostics);
            let rule = FRACTION_RULE_EM * size;
            let pad = if *frame == Frame::Box {
                0.25 * size
            } else {
                0.0
            };
            offset_items(&mut b.items, pad, 0.0);
            let width = b.width + 2.0 * pad;
            // Content extents: ascent/descent carry font-size headroom, so the
            // rules sit a small gap outside the nominal glyph box.
            let top = -(0.75 * size) - 0.15 * size - pad * 0.4;
            let bottom = 0.2 * size + 0.1 * size + pad * 0.4;
            let rule_item = |x: f64, y: f64, w: f64, h: f64| MathItem {
                font: None,
                text: FRACTION_RULE_CHAR.to_string(),
                x,
                baseline: y + h,
                size,
                span: atom.span,
                rule: Some(MathRule {
                    y,
                    width: w,
                    height: h,
                }),
            };
            let mut rules = Vec::new();
            if frame.is_over() {
                rules.push(rule_item(0.0, top - rule, width, rule));
            }
            if frame.is_under() {
                rules.push(rule_item(0.0, bottom, width, rule));
            }
            if *frame == Frame::Box {
                let height = bottom - top + 2.0 * rule;
                rules.push(rule_item(0.0, top - rule, rule, height));
                rules.push(rule_item(width - rule, top - rule, rule, height));
            }
            b.items.extend(rules);
            MathBox {
                items: b.items,
                width,
                ascent: b.ascent.max(-(top - rule)),
                descent: b.descent.max(bottom + rule),
            }
        }
        Nucleus::Space { em, .. } => MathBox {
            items: Vec::new(),
            width: em * size,
            ascent: size,
            descent: 0.2 * size,
        },
        Nucleus::Radical(body) => {
            let mut b = layout_list(body, size, root_size, level, diagnostics);
            let radical_width = crate::layout::shaped_width(
                "√",
                size,
                crate::layout::math_font("√"),
                atom.span,
                diagnostics,
            )
            .0;
            offset_items(&mut b.items, radical_width, 0.0);
            b.items.insert(
                0,
                MathItem {
                    font: None,
                    text: "√".into(),
                    x: 0.0,
                    baseline: 0.0,
                    size,
                    span: atom.span,
                    rule: None,
                },
            );
            // The vinculum, as Symbol's own `radicalex` extender draws it: from
            // the radical's ink edge over the whole body, top-aligned with the
            // radical glyph. Neither the sign nor the bar grows for tall bodies.
            let vinculum_x = RADICAL_INK_RIGHT_EM * size;
            let vinculum_height = RADICALEX_THICKNESS_EM * size;
            let vinculum_y = -RADICAL_TOP_EM * size;
            b.items.push(MathItem {
                font: None,
                text: FRACTION_RULE_CHAR.to_string(),
                x: vinculum_x,
                baseline: vinculum_y + vinculum_height,
                size,
                span: atom.span,
                rule: Some(MathRule {
                    y: vinculum_y,
                    width: radical_width - vinculum_x + b.width,
                    height: vinculum_height,
                }),
            });
            b.width += radical_width;
            b.ascent = b.ascent.max(RADICAL_TOP_EM * size);
            b
        }
        Nucleus::Fraction {
            numerator,
            denominator,
        } => {
            let child_size = if level == 0 {
                root_size * SCRIPT_SCALE
            } else {
                root_size * SECOND_ORDER_SCRIPT_SCALE
            };
            let mut num = layout_list(numerator, child_size, root_size, level + 1, diagnostics);
            let mut den = layout_list(denominator, child_size, root_size, level + 1, diagnostics);
            let pad = 0.12 * size;
            let natural_width = num.width.max(den.width) + 2.0 * pad;
            let axis = -MATH_AXIS_EM * size;
            let rule = FRACTION_RULE_EM * size;
            // This legacy string is only a paint fallback. Its geometry is the
            // real rule width and does not pretend U+2500 exists in a Core 14 face.
            let rule_text = "─".to_string();
            let width = natural_width;
            let num_dy = axis - FRACTION_GAP_EM * size - rule / 2.0 - num.descent;
            let den_dy = axis + FRACTION_GAP_EM * size + rule / 2.0 + den.ascent;
            let num_x = (width - num.width) / 2.0;
            let den_x = (width - den.width) / 2.0;
            offset_items(&mut num.items, num_x, num_dy);
            offset_items(&mut den.items, den_x, den_dy);
            let mut items = num.items;
            items.push(MathItem {
                font: None,
                text: rule_text,
                x: 0.0,
                baseline: axis + rule / 2.0,
                size: child_size,
                span: atom.span,
                rule: Some(MathRule {
                    y: axis - rule / 2.0,
                    width,
                    height: rule,
                }),
            });
            items.extend(den.items);
            MathBox {
                items,
                width,
                ascent: (num.ascent - num_dy).max(size * 0.5),
                descent: (den.descent + den_dy).max(size * 0.2),
            }
        }
        Nucleus::Matrix {
            rows,
            columns,
            left,
            right,
        } => layout_matrix(
            atom,
            rows,
            columns,
            (left, right),
            size,
            root_size,
            level,
            diagnostics,
        ),
        Nucleus::Accent { accent, body } => {
            layout_accent(atom, *accent, body, size, root_size, level, diagnostics)
        }
        // `\mathbin{...}` and kin: laid out exactly like a bare `{...}`
        // group; only the enclosing atom's forced class differs.
        Nucleus::Group(body) => layout_list(body, size, root_size, level, diagnostics),
        // The compiler's own (base-14) layout has no delimiter sizing or
        // style changes: a delimited `\genfrac` is set like the grid `\binom`
        // used to be, an undelimited one like `\frac`.
        Nucleus::GenFraction {
            numerator,
            denominator,
            left,
            right,
            ..
        } => {
            let nucleus = if left.is_empty() && right.is_empty() {
                Nucleus::Fraction {
                    numerator: numerator.clone(),
                    denominator: denominator.clone(),
                }
            } else {
                Nucleus::Matrix {
                    rows: vec![vec![numerator.clone()], vec![denominator.clone()]],
                    columns: "c".into(),
                    left: left.clone(),
                    right: right.clone(),
                }
            };
            layout_nucleus(
                &MathAtom {
                    nucleus,
                    ..atom.clone()
                },
                size,
                root_size,
                level,
                diagnostics,
            )
        }
        Nucleus::Rule(rule) => {
            use crate::text_builtins::{self as tb, DimenContext};
            // `\rule` is an `\hbox` built with the current text font, which in
            // a formula is the text size (`root_size`), not the script size.
            let measure = crate::layout::LayoutConstraints::default().measure_pt;
            let cx = DimenContext {
                quad: tb::pt_to_sp(root_size),
                x_height: tb::pt_to_sp(crate::layout::x_height_pt(
                    crate::layout::Font::TimesRoman,
                    root_size,
                )),
                text_width: tb::pt_to_sp(measure),
                line_width: tb::pt_to_sp(measure),
                column_width: tb::pt_to_sp(measure),
            };
            let b = rule.resolve(&cx);
            let width = tb::sp_to_pt(b.width);
            let mut items = Vec::new();
            if b.painted() {
                items.push(MathItem {
                    font: None,
                    text: FRACTION_RULE_CHAR.to_string(),
                    x: 0.0,
                    baseline: 0.0,
                    size,
                    span: atom.span,
                    rule: Some(MathRule {
                        y: -tb::sp_to_pt(b.rule_top),
                        width,
                        height: tb::sp_to_pt(b.rule_top - b.rule_bottom),
                    }),
                });
            }
            MathBox {
                items,
                width,
                ascent: tb::sp_to_pt(b.height),
                descent: tb::sp_to_pt(b.depth),
            }
        }
        Nucleus::Phantom {
            body,
            horizontal,
            vertical,
        } => {
            let mut b = layout_list(body, size, root_size, level, diagnostics);
            b.items.clear();
            if !horizontal {
                b.width = 0.0;
            }
            if !vertical {
                b.ascent = 0.0;
                b.descent = 0.0;
            }
            b
        }
        Nucleus::Operator { body, .. } => layout_list(body, size, root_size, level, diagnostics),
        // Approximated as the arrow glyph with its labels stacked over and
        // under it (render-pipeline builds amsmath's stretched arrow).
        Nucleus::ExtArrow {
            arrow,
            above,
            below,
        } => {
            let glyph = match arrow {
                ExtArrow::Right => "→",
                ExtArrow::Left => "←",
                ExtArrow::LeftRight => "↔",
            };
            let stacked = MathAtom {
                nucleus: Nucleus::Stacked {
                    base: MathList {
                        atoms: vec![symbol(glyph.into(), atom.span)],
                    },
                    over: (!above.atoms.is_empty()).then(|| above.clone()),
                    under: (!below.atoms.is_empty()).then(|| below.clone()),
                },
                ..atom.clone()
            };
            layout_nucleus(&stacked, size, root_size, level, diagnostics)
        }
        Nucleus::SubArray { rows, .. } => {
            let rows: Vec<Vec<MathList>> = rows.iter().map(|r| vec![r.clone()]).collect();
            layout_matrix(
                atom,
                &rows,
                "c",
                ("", ""),
                size,
                root_size,
                level,
                diagnostics,
            )
        }
    }
}

/// Places `accent`'s mark over `body`.
///
/// Horizontal: symmetric centering, `(body.width - glyph.width) / 2`, plus a
/// skew term when `body` is a single italic Latin letter (a math variable —
/// `crate::layout::math_font` puts those in Times-Italic). TeX shifts an
/// accent right in that case by the base character's TFM skewchar kern
/// (TeXbook Appendix G, rule 12); Adobe Core 14 AFM metrics have no skewchar
/// concept to borrow that from, so `crate::layout::italic_skew_pt` derives an
/// equivalent shift from Times-Italic's real `ItalicAngle` (-15.5 degrees)
/// instead — see that function for the reasoning. Upright bodies (digits,
/// multi-letter names, Symbol-font Greek) keep plain symmetric centering.
///
/// Vertical: TeX's real rule, `raise = min(nucleus_height, accent font's
/// x-height)`, using the real Times-Roman x-height
/// (`crate::layout::x_height_pt`) rather than a guessed constant.
fn layout_accent(
    atom: &MathAtom,
    accent: Accent,
    body: &MathList,
    size: f64,
    root_size: f64,
    level: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> MathBox {
    let mut b = layout_list(body, size, root_size, level, diagnostics);
    let Some(glyph) = accent.glyph() else {
        // \check / \breve: no base-14 glyph. Already diagnosed at parse time
        // (`accent_atom`); typeset the body alone rather than draw nothing
        // and also fabricate a plausible-looking substitute mark.
        return b;
    };
    let glyph_text = glyph.to_string();
    let accent_font = crate::layout::math_font(&glyph_text);
    let accent_width =
        crate::layout::shaped_width(&glyph_text, size, accent_font, atom.span, diagnostics).0;
    let raise = b.ascent.min(crate::layout::x_height_pt(accent_font, size));
    // Skew only a single italic Latin letter (a math variable, per
    // `crate::layout::math_font`): a digit, a multi-letter name and
    // Symbol-font Greek are all upright and keep plain symmetric centering.
    let skew = match body.atoms.as_slice() {
        [MathAtom {
            nucleus: Nucleus::Symbol(text),
            superscript: None,
            subscript: None,
            ..
        }] if crate::layout::math_font(text) == crate::layout::Font::TimesItalic => {
            crate::layout::italic_skew_pt(crate::layout::Font::TimesItalic, raise)
        }
        _ => 0.0,
    };
    let dx = (b.width - accent_width) / 2.0 + skew;
    // A thin mark, not a full-height glyph: ~0.15em is enough for a
    // circumflex/tilde/dot/acute stroke without inflating every accented
    // atom's box to a full line height.
    let accent_ascent = 0.15 * size;
    b.items.push(MathItem {
        font: Some(accent_font),
        text: glyph_text,
        x: dx,
        baseline: -raise,
        size,
        span: atom.span,
        rule: None,
    });
    b.ascent = b.ascent.max(raise + accent_ascent);
    b
}

/// Lays out a grid centred on the math axis, with fences scaled to its height.
#[allow(clippy::too_many_arguments)]
fn layout_matrix(
    atom: &MathAtom,
    rows: &[Vec<MathList>],
    columns: &str,
    fences: (&str, &str),
    size: f64,
    root_size: f64,
    level: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> MathBox {
    let boxes: Vec<Vec<MathBox>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| layout_list(cell, size, root_size, level, diagnostics))
                .collect()
        })
        .collect();
    let aligns: Vec<char> = columns.chars().collect();
    let mut widths = vec![0.0f64; aligns.len()];
    for row in &boxes {
        for (column, b) in row.iter().enumerate() {
            widths[column] = widths[column].max(b.width);
        }
    }
    let column_gap = MATRIX_COLUMN_GAP_EM * size;
    let row_gap = MATRIX_ROW_GAP_EM * size;
    // Row baselines relative to the first row's baseline.
    let mut baselines = Vec::with_capacity(boxes.len());
    let mut y = 0.0;
    for (index, row) in boxes.iter().enumerate() {
        let ascent = row.iter().map(|b| b.ascent).fold(size * 0.7, f64::max);
        if index > 0 {
            y += ascent + row_gap;
        }
        baselines.push(y);
        y += row.iter().map(|b| b.descent).fold(size * 0.2, f64::max);
    }
    let first_ascent = boxes.first().map_or(size * 0.7, |row| {
        row.iter().map(|b| b.ascent).fold(size * 0.7, f64::max)
    });
    let height = first_ascent + y;
    // Centre the grid on the math axis.
    let shift = -MATH_AXIS_EM * size - height / 2.0 + first_ascent;
    let (left, right) = fences;
    let fence_size = height.max(size);
    let fence_width = |text: &str, diagnostics: &mut Vec<Diagnostic>| {
        if text.is_empty() {
            0.0
        } else {
            crate::layout::shaped_width(
                text,
                fence_size,
                crate::layout::math_font(text),
                atom.span,
                diagnostics,
            )
            .0
        }
    };
    let left_width = fence_width(left, diagnostics);
    let mut items = Vec::new();
    // A fence glyph's visual centre sits roughly 0.3em above its baseline.
    let fence_baseline = -MATH_AXIS_EM * size + 0.3 * fence_size;
    if !left.is_empty() {
        items.push(MathItem {
            font: None,
            text: left.into(),
            x: 0.0,
            baseline: fence_baseline,
            size: fence_size,
            span: atom.span,
            rule: None,
        });
    }
    let pad = if left.is_empty() { 0.0 } else { 0.15 * size };
    let mut grid_width = 0.0;
    for (row, baseline) in boxes.into_iter().zip(&baselines) {
        let mut x = left_width + pad;
        for (column, mut b) in row.into_iter().enumerate() {
            let dx = match aligns[column] {
                'r' => widths[column] - b.width,
                'c' => (widths[column] - b.width) / 2.0,
                _ => 0.0,
            };
            offset_items(&mut b.items, x + dx, baseline + shift);
            items.extend(b.items);
            x += widths[column] + column_gap;
        }
    }
    if !widths.is_empty() {
        grid_width = widths.iter().sum::<f64>() + column_gap * (widths.len() - 1) as f64;
    }
    let mut width = left_width + pad + grid_width;
    if !right.is_empty() {
        width += 0.15 * size;
        items.push(MathItem {
            font: None,
            text: right.into(),
            x: width,
            baseline: fence_baseline,
            size: fence_size,
            span: atom.span,
            rule: None,
        });
        width += fence_width(right, diagnostics);
    }
    MathBox {
        items,
        width,
        ascent: (first_ascent - shift).max(size),
        descent: (y + shift).max(0.2 * size),
    }
}

fn offset_items(items: &mut [MathItem], dx: f64, dy: f64) {
    for item in items {
        item.x += dx;
        item.baseline += dy;
    }
}

/// Split lexer word runs into one token per Unicode scalar for atom attachment.
fn split_word_tokens(tokens: &[Token]) -> Vec<Token> {
    let mut out = Vec::new();
    for token in tokens {
        if let TokenKind::Word(word) = &token.kind {
            let source_matches_word = token.span.end - token.span.start == word.len();
            for (offset, ch) in word.char_indices() {
                out.push(Token {
                    kind: TokenKind::Word(ch.to_string()),
                    span: if source_matches_word {
                        Span::in_document(
                            token.span.document,
                            token.span.start + offset,
                            token.span.start + offset + ch.len_utf8(),
                        )
                    } else {
                        // Macro replacement text has no byte range of its own.
                        // Preserve the invocation attribution for every atom
                        // instead of fabricating per-glyph provenance.
                        token.span
                    },
                });
            }
        } else {
            out.push(token.clone());
        }
    }
    out
}

/// Shifts every span in a math list by `delta` bytes.
///
/// Incremental reuse moves unchanged blocks when earlier text grows or shrinks.
/// A math list nests — scripts, fractions and radicals each hold their own list —
/// so shifting only the outer span would leave every inner span pointing at the
/// previous revision's bytes, and source navigation would land in the wrong place.
pub fn shift_list(list: &MathList, delta: isize) -> MathList {
    MathList {
        atoms: list.atoms.iter().map(|a| shift_atom(a, delta)).collect(),
    }
}

fn shift_atom(atom: &MathAtom, delta: isize) -> MathAtom {
    MathAtom {
        nucleus: match &atom.nucleus {
            Nucleus::Symbol(s) => Nucleus::Symbol(s.clone()),
            Nucleus::SizedDelimiter { .. } => atom.nucleus.clone(),
            Nucleus::Text(s) => Nucleus::Text(s.clone()),
            Nucleus::Space { em, font_em } => Nucleus::Space {
                em: *em,
                font_em: *font_em,
            },
            Nucleus::Fraction {
                numerator,
                denominator,
            } => Nucleus::Fraction {
                numerator: shift_list(numerator, delta),
                denominator: shift_list(denominator, delta),
            },
            Nucleus::Radical(inner) => Nucleus::Radical(shift_list(inner, delta)),
            Nucleus::Bold(s) => Nucleus::Bold(s.clone()),
            Nucleus::Framed { body, frame } => Nucleus::Framed {
                body: shift_list(body, delta),
                frame: *frame,
            },
            Nucleus::Stacked { base, over, under } => Nucleus::Stacked {
                base: shift_list(base, delta),
                over: over.as_ref().map(|l| shift_list(l, delta)),
                under: under.as_ref().map(|l| shift_list(l, delta)),
            },
            Nucleus::Matrix {
                rows,
                columns,
                left,
                right,
            } => Nucleus::Matrix {
                rows: rows
                    .iter()
                    .map(|row| row.iter().map(|cell| shift_list(cell, delta)).collect())
                    .collect(),
                columns: columns.clone(),
                left: left.clone(),
                right: right.clone(),
            },
            Nucleus::Accent { accent, body } => Nucleus::Accent {
                accent: *accent,
                body: shift_list(body, delta),
            },
            Nucleus::Group(inner) => Nucleus::Group(shift_list(inner, delta)),
            Nucleus::GenFraction {
                numerator,
                denominator,
                thickness_pt,
                left,
                right,
                style,
            } => Nucleus::GenFraction {
                numerator: shift_list(numerator, delta),
                denominator: shift_list(denominator, delta),
                thickness_pt: *thickness_pt,
                left: left.clone(),
                right: right.clone(),
                style: *style,
            },
            Nucleus::Phantom {
                body,
                horizontal,
                vertical,
            } => Nucleus::Phantom {
                body: shift_list(body, delta),
                horizontal: *horizontal,
                vertical: *vertical,
            },
            Nucleus::Operator { body, limits } => Nucleus::Operator {
                body: shift_list(body, delta),
                limits: *limits,
            },
            Nucleus::Rule(rule) => Nucleus::Rule(rule.clone()),
            Nucleus::ExtArrow {
                arrow,
                above,
                below,
            } => Nucleus::ExtArrow {
                arrow: *arrow,
                above: shift_list(above, delta),
                below: shift_list(below, delta),
            },
            Nucleus::SubArray { rows, align } => Nucleus::SubArray {
                rows: rows.iter().map(|r| shift_list(r, delta)).collect(),
                align: *align,
            },
        },
        span: shift(atom.span, delta),
        superscript: atom.superscript.as_ref().map(|l| shift_list(l, delta)),
        subscript: atom.subscript.as_ref().map(|l| shift_list(l, delta)),
        class_override: atom.class_override,
        width_em: atom.width_em,
        ams_symbol: atom.ams_symbol,
    }
}

fn shift(span: Span, delta: isize) -> Span {
    let apply = |v: usize| -> usize {
        if delta >= 0 {
            v.saturating_add(delta as usize)
        } else {
            v.saturating_sub(delta.unsigned_abs())
        }
    };
    Span::in_document(span.document, apply(span.start), apply(span.end))
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    #[test]
    fn primes_mathrm_and_epsilons_follow_latex() {
        let parse = |src: &str| {
            let mut diagnostics = Vec::new();
            let list = parse_tokens(
                &crate::lexer::tokenize(src),
                MathPackages::KERNEL,
                &mut diagnostics,
            );
            assert!(diagnostics.is_empty(), "{src}: {diagnostics:?}");
            list
        };
        let symbols = |list: &MathList| -> Vec<String> {
            list.atoms
                .iter()
                .map(|a| match &a.nucleus {
                    Nucleus::Symbol(s) => s.clone(),
                    Nucleus::Text(t) => format!("text:{t}"),
                    other => format!("{other:?}"),
                })
                .collect()
        };
        // latex.ltx `\active@math@prime`: `f''` is `f^{\prime\prime}` and a
        // following `^` joins the same superscript.
        let list = parse(r"f''(x) g'^2");
        assert_eq!(symbols(&list), ["f", "(", "x", ")", "g"]);
        assert_eq!(symbols(list.atoms[0].superscript.as_ref().unwrap()), ["\u{2032}", "\u{2032}"]);
        assert_eq!(symbols(list.atoms[4].superscript.as_ref().unwrap()), ["\u{2032}", "2"]);
        // `\mathrm` sets its letters upright (fontmath.ltx `operators`).
        let list = parse(r"\mathrm{K}^{-1} \mathrm{k g}");
        assert_eq!(symbols(&list), ["text:K", "text:kg"]);
        assert!(list.atoms[0].superscript.is_some());
        // cmmi "0F is `\epsilon` (lunate), "22 `\varepsilon`.
        assert_eq!(symbols(&parse(r"\epsilon\varepsilon")), ["\u{03F5}", "\u{03B5}"]);
    }

    #[test]
    fn grid_position_argument_is_not_a_cell() {
        for (src, want_columns) in [
            (r"\begin{aligned}[t] a &= b \end{aligned}", None),
            (r"\begin{array}[b]{cc} a & b \end{array}", Some("cc")),
            (r"\begin{gathered} [c] a \end{gathered}", None),
            (r"\begin{alignedat}[t]{1} a &= b \end{alignedat}", None),
        ] {
            let mut diagnostics = Vec::new();
            let tokens = crate::lexer::tokenize(src);
            let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
            assert!(diagnostics.is_empty(), "{src}: {diagnostics:?}");
            let Nucleus::Matrix { rows, columns, .. } = &list.atoms[0].nucleus else {
                panic!("{src}: not a grid: {:?}", list.atoms)
            };
            let brackets = rows[0][0]
                .atoms
                .iter()
                .any(|a| matches!(&a.nucleus, Nucleus::Symbol(s) if s == "[" || s == "]"));
            assert!(!brackets, "{src}: first cell {:?}", rows[0][0]);
            assert_eq!(rows[0][0].atoms.len(), 1, "{src}: first cell is just `a`");
            if let Some(want) = want_columns {
                assert_eq!(columns, want, "{src}");
            }
        }
    }

    #[test]
    fn big_delimiters_scale_like_cmex_and_keep_tex_classes() {
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"\bigl(x\bigr) \Bigm| \bigg[ \Biggr] \big.");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let sized: Vec<(&str, f64, DelimiterRole)> = list
            .atoms
            .iter()
            .filter_map(|atom| match &atom.nucleus {
                Nucleus::SizedDelimiter { glyph, scale, role } => {
                    Some((glyph.as_str(), *scale, *role))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            sized,
            [
                ("(", 1.2, DelimiterRole::Open),
                (")", 1.2, DelimiterRole::Close),
                ("|", 1.8, DelimiterRole::Rel),
                ("[", 2.4, DelimiterRole::Ord),
                ("]", 3.0, DelimiterRole::Close),
            ]
        );
        // `\big.` stays the invisible null delimiter.
        assert!(matches!(
            list.atoms.last().map(|a| &a.nucleus),
            Some(Nucleus::Space { em, .. }) if *em == 0.0
        ));

        let boxed = layout(
            &parse_tokens(
                &crate::lexer::tokenize(r"\bigl("),
                MathPackages::KERNEL,
                &mut diagnostics,
            ),
            10.0,
            &mut diagnostics,
        );
        let paren = &boxed.items[0];
        assert_eq!(paren.size, 12.0);
        // Lowered so its centre stays on the axis; 8.5pt high, 3.5pt deep.
        assert!((paren.baseline - 0.5).abs() < 1e-9, "{}", paren.baseline);
        assert!((boxed.descent - 3.5).abs() < 1e-9, "{}", boxed.descent);
    }

    #[test]
    fn left_right_hug_ordinary_content_at_scale_one() {
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"\left( x \right)");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let sized: Vec<(&str, f64, DelimiterRole)> = list
            .atoms
            .iter()
            .filter_map(|atom| match &atom.nucleus {
                Nucleus::SizedDelimiter { glyph, scale, role } => {
                    Some((glyph.as_str(), *scale, *role))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            sized,
            [
                ("(", 1.0, DelimiterRole::Left),
                (")", 1.0, DelimiterRole::Right),
            ],
            "parsed at ordinary size before the layout pass stretches them"
        );

        let size = 10.0;
        let boxed = layout(&list, size, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let open = boxed.items.iter().find(|i| i.text == "(").unwrap();
        let close = boxed.items.iter().find(|i| i.text == ")").unwrap();
        assert!((open.size - size).abs() < 1e-9, "{}", open.size);
        assert!((close.size - size).abs() < 1e-9, "{}", close.size);
    }

    #[test]
    fn left_right_stretch_around_a_fraction_and_agree_on_scale() {
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"\left( \frac{a}{b} \right)");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let size = 10.0;
        let boxed = layout(&list, size, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let open = boxed.items.iter().find(|i| i.text == "(").unwrap();
        let close = boxed.items.iter().find(|i| i.text == ")").unwrap();
        assert!(open.size > size, "expected growth, got {}", open.size);
        assert!(
            (open.size - close.size).abs() < 1e-9,
            "both fences of a pair must share one scale: {} vs {}",
            open.size,
            close.size
        );
    }

    #[test]
    fn nested_left_right_pairs_each_hug_their_own_content() {
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"\left[ \left( \frac{a}{b} \right) \right]");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let size = 10.0;
        let boxed = layout(&list, size, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let bracket_open = boxed.items.iter().find(|i| i.text == "[").unwrap();
        let bracket_close = boxed.items.iter().find(|i| i.text == "]").unwrap();
        let paren_open = boxed.items.iter().find(|i| i.text == "(").unwrap();
        let paren_close = boxed.items.iter().find(|i| i.text == ")").unwrap();
        assert!((bracket_open.size - bracket_close.size).abs() < 1e-9);
        assert!((paren_open.size - paren_close.size).abs() < 1e-9);
        assert!(bracket_open.size > size, "{}", bracket_open.size);
        assert!(paren_open.size > size, "{}", paren_open.size);
        // The outer pair encloses the (already stretched) inner pair, so it
        // can never need to be smaller than it.
        assert!(
            bracket_open.size >= paren_open.size - 1e-9,
            "outer {} < inner {}",
            bracket_open.size,
            paren_open.size
        );
    }

    #[test]
    fn null_left_delimiter_stretches_invisibly_with_its_paired_fence() {
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"\left. \frac{a}{b} \right|");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let size = 10.0;
        let boxed = layout(&list, size, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let bar = boxed.items.iter().find(|i| i.text == "|").unwrap();
        assert!(bar.size > size, "expected the bar to stretch: {}", bar.size);
        let null = boxed
            .items
            .iter()
            .find(|i| i.text.is_empty())
            .expect("the null delimiter still emits an (invisible) item");
        assert!(
            (null.size - bar.size).abs() < 1e-9,
            "the null delimiter's partner sets its scale too: {} vs {}",
            null.size,
            bar.size
        );
    }

    #[test]
    fn unmatched_left_delimiter_does_not_panic_and_stays_at_scale_one() {
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"\left( x");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        let _ = layout(&list, 10.0, &mut diagnostics);
        let open = list
            .atoms
            .iter()
            .find_map(|atom| match &atom.nucleus {
                Nucleus::SizedDelimiter { glyph, scale, role } if glyph == "(" => {
                    Some((*scale, *role))
                }
                _ => None,
            })
            .expect("the lone \\left( is still a SizedDelimiter");
        assert_eq!(open, (1.0, DelimiterRole::Left));
    }

    #[test]
    fn logical_commands_are_real_exportable_symbol_atoms() {
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"\in\forall\exists\vee\Rightarrow\mid");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let glyphs: Vec<&str> = list
            .atoms
            .iter()
            .map(|atom| match &atom.nucleus {
                Nucleus::Symbol(text) => text.as_str(),
                other => panic!("expected symbol, got {other:?}"),
            })
            .collect();
        assert_eq!(glyphs, ["∈", "∀", "∃", "∨", "⇒", "∣"]);
        let _ = layout(&list, 12.0, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn display_limits_stack_under_lim_but_stay_beside_inline_and_on_integrals() {
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"\lim_{x\to 0} f \int_0^1 g");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        let x_of = |b: &MathBox, text: &str| {
            b.items
                .iter()
                .find(|item| item.text == text)
                .map(|item| (item.x, item.baseline))
                .unwrap()
        };
        let display = layout_display(&list, 12.0, &mut diagnostics);
        let inline = layout(&list, 12.0, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let (lim_x, _) = x_of(&display, "lim");
        let (sub_x, sub_y) = x_of(&display, "x");
        // Stacked: the limit starts under the operator, not after it.
        assert!(
            sub_x <= lim_x + 1.0 && sub_y > 0.5 * 12.0,
            "{lim_x} {sub_x} {sub_y}"
        );
        let (_, inline_sub_y) = x_of(&inline, "x");
        assert!(inline_sub_y < sub_y);
        // Integrals keep side scripts in display style.
        let (int_x, _) = x_of(&display, "∫");
        let zero_x = display
            .items
            .iter()
            .rev()
            .find(|item| item.text == "0")
            .unwrap()
            .x;
        assert!(zero_x > int_x);
        assert!(display.width < inline.width);
    }

    #[test]
    fn stacked_scripts_and_infix_choose_over_build_real_atoms() {
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(
            r"\overset{?}{=} \underset{x}{\min} {n \choose k} {a \over b} \lim\limits_{x}",
        );
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let nuclei: Vec<&Nucleus> = list.atoms.iter().map(|atom| &atom.nucleus).collect();
        assert_eq!(nuclei.len(), 5, "{nuclei:?}");
        assert!(
            list.atoms[4].subscript.is_some(),
            "limits keeps the script on lim"
        );
        assert!(matches!(
            nuclei[0],
            Nucleus::Stacked {
                over: Some(_),
                under: None,
                ..
            }
        ));
        assert!(matches!(
            nuclei[1],
            Nucleus::Stacked {
                over: None,
                under: Some(_),
                ..
            }
        ));
        assert!(matches!(nuclei[2], Nucleus::Matrix { rows, .. } if rows.len() == 2));
        assert!(matches!(nuclei[3], Nucleus::Fraction { .. }));
        let laid = layout(&list, 12.0, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let question = laid.items.iter().find(|item| item.text == "?").unwrap();
        let equals = laid.items.iter().find(|item| item.text == "=").unwrap();
        assert!(question.baseline < equals.baseline - 6.0, "? sits above =");
    }

    #[test]
    fn structural_homework_commands_build_real_atoms() {
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(
            r"\binom{n}{k} \sqrt[3]{8} \mathbf{F} \boxed{x=4} \overline{AB} a \pmod{n} \tag{2}",
        );
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let nuclei: Vec<&Nucleus> = list.atoms.iter().map(|atom| &atom.nucleus).collect();
        assert!(matches!(
            nuclei[0],
            Nucleus::GenFraction { left, right, thickness_pt: Some(t), style: None, .. }
                if left == "(" && right == ")" && *t == 0.0
        ));
        assert!(
            list.atoms[1].superscript.is_some(),
            "root index is a raised script"
        );
        assert!(matches!(nuclei[2], Nucleus::Radical(_)));
        assert_eq!(nuclei[3], &Nucleus::Bold("F".into()));
        assert!(matches!(
            nuclei[4],
            Nucleus::Framed {
                frame: Frame::Box,
                ..
            }
        ));
        assert!(matches!(
            nuclei[5],
            Nucleus::Framed {
                frame: Frame::Over,
                ..
            }
        ));
        assert!(nuclei.contains(&&Nucleus::Text("(2)".into())));
        let laid = layout(&list, 12.0, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        // The box contributes four real rules, the overline one and the
        // radical's vinculum one, beside the binomial's none.
        assert_eq!(
            laid.items.iter().filter(|item| item.rule.is_some()).count(),
            6
        );
    }

    #[test]
    fn sqrt_draws_its_vinculum_over_the_whole_body() {
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"\sqrt{10-x}");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        let size = 10.0;
        let laid = layout(&list, size, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let rules: Vec<_> = laid.items.iter().filter(|i| i.rule.is_some()).collect();
        assert_eq!(rules.len(), 1, "exactly one vinculum");
        let bar = rules[0].rule.unwrap();
        let x_item = laid.items.iter().find(|i| i.text == "x").unwrap();
        // Starts at the radical's ink edge and reaches the end of the body.
        assert!((rules[0].x - RADICAL_INK_RIGHT_EM * size).abs() < 1e-9);
        assert!((rules[0].x + bar.width - laid.width).abs() < 1e-9);
        assert!(
            rules[0].x + bar.width > x_item.x,
            "covers the last body glyph"
        );
        // Top-aligned with the radical glyph at Symbol's radicalex thickness.
        assert!((bar.y + RADICAL_TOP_EM * size).abs() < 1e-9);
        assert!((bar.height - RADICALEX_THICKNESS_EM * size).abs() < 1e-9);
    }

    #[test]
    fn handwritten_homework_constructs_parse_and_shape_without_diagnostics() {
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(
            r"\lim_{n\to\infty}\left(1+\frac{1}{n}\right)^n \sin\theta \operatorname*{rank}(A)
              \mathrm{d}x \Gamma\Delta\partial\nabla\equiv\propto\cup\subseteq\notin\emptyset
              \iff\langle u\rangle \big\{ \bigr\} \left. \right| \dfrac{1}{2} a\!b\cdots\dots",
        );
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let _ = layout(&list, 12.0, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert!(list
            .atoms
            .iter()
            .any(|atom| atom.nucleus == Nucleus::Text("lim".into()) && atom.subscript.is_some()));
        // amsopn: `\operatorname*{rank}` is `\mathop{\operator@font rank}\limits`.
        assert!(list.atoms.iter().any(|atom| matches!(
            &atom.nucleus,
            Nucleus::Operator { body, limits: true }
                if body.atoms.len() == 1 && body.atoms[0].nucleus == Nucleus::Text("rank".into())
        )));
    }

    #[test]
    fn delimiter_sizes_consume_the_source_delimiter_once() {
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"\bigl(x\bigr)");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let glyphs: Vec<&str> = list
            .atoms
            .iter()
            .map(|atom| match &atom.nucleus {
                Nucleus::Symbol(text) | Nucleus::SizedDelimiter { glyph: text, .. } => {
                    text.as_str()
                }
                other => panic!("expected symbol, got {other:?}"),
            })
            .collect();
        assert_eq!(glyphs, ["(", "x", ")"]);
    }

    #[test]
    fn bare_ampersand_outside_alignment_is_diagnosed() {
        // Reference-corpus negative fixture `error-extra-math-align`: a `&`
        // in ordinary (non-tabular) math used to pass through silently as a
        // literal symbol. `grid_environment` (matrices, `cases`, `array`)
        // and the parser's align/gather row-splitting both consume their own
        // `&` before it ever reaches this list, so one seen here is always a
        // misplaced alignment tab.
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize("a & b");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(diagnostics[0].message.contains("misplaced alignment tab"));
        let glyphs: Vec<&str> = list
            .atoms
            .iter()
            .map(|atom| match &atom.nucleus {
                Nucleus::Symbol(text) => text.as_str(),
                other => panic!("expected symbol, got {other:?}"),
            })
            .collect();
        assert_eq!(glyphs, ["a", "b"], "the stray & is dropped, not typeset");
    }

    #[test]
    fn quad_is_text_font_em_and_thin_space_is_math_units() {
        let tokens = crate::lexer::tokenize(r"a\quad b\,c");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut Vec::new());
        let spaces: Vec<(f64, bool)> = list
            .atoms
            .iter()
            .filter_map(|a| match a.nucleus {
                Nucleus::Space { em, font_em } => Some((em, font_em)),
                _ => None,
            })
            .collect();
        assert_eq!(spaces, [(1.0, true), (3.0 / 18.0, false)]);
    }

    #[test]
    fn xrightarrow_takes_optional_below_and_required_above() {
        let tokens = crate::lexer::tokenize(r"A \xrightarrow{f} B \xleftarrow[g]{h} C");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut Vec::new());
        let arrows: Vec<_> = list
            .atoms
            .iter()
            .filter_map(|a| match &a.nucleus {
                Nucleus::ExtArrow {
                    arrow,
                    above,
                    below,
                } => Some((*arrow, above.atoms.len(), below.atoms.len(), atom_class(a))),
                _ => None,
            })
            .collect();
        assert_eq!(
            arrows,
            [
                (ExtArrow::Right, 1, 0, Some(AtomClass::Rel)),
                (ExtArrow::Left, 1, 1, Some(AtomClass::Rel)),
            ]
        );
    }

    #[test]
    fn quad_text_and_qquad_have_distinct_semantics() {
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"\quad\text{two words}\qquad");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert!(matches!(list.atoms[0].nucleus, Nucleus::Space { em, .. } if em == 1.0));
        assert!(matches!(&list.atoms[1].nucleus, Nucleus::Text(text) if text == "two words"));
        assert!(matches!(list.atoms[2].nucleus, Nucleus::Space { em, .. } if em == 2.0));

        let laid_out = layout(&list, 12.0, &mut diagnostics);
        assert_eq!(laid_out.items.len(), 1, "spacing must not emit fake glyphs");
        assert_eq!(
            laid_out.items[0].font,
            Some(crate::layout::Font::TimesRoman),
            "text nuclei must retain explicit Roman intent"
        );
        let text_width =
            crate::layout::text_width("two words", 12.0, crate::layout::Font::TimesRoman);
        assert!((laid_out.width - (text_width + 36.0)).abs() < 0.001);
    }

    #[test]
    fn malformed_delimiter_and_text_arguments_remain_diagnostic() {
        // `\bigl` truly has no following delimiter at all; `\text` truly has
        // no following token at all. Neither is TeX's "undelimited argument"
        // case -- that requires a token to take as the argument.
        for source in [r"\bigl", r"\text"] {
            let mut diagnostics = Vec::new();
            let tokens = crate::lexer::tokenize(source);
            let _ = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
            assert!(!diagnostics.is_empty(), "{source:?} must remain diagnostic");
        }

        // TeX's undelimited argument: without braces, `\text` takes just the
        // next single token ("u"), leaving the rest ("nbraced") to parse as
        // ordinary math symbols rather than erroring or being swallowed.
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"\text x");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(list.atoms.len(), 1, "{:?}", list.atoms);
        assert!(matches!(list.atoms[0].nucleus, Nucleus::Text(ref text) if text == "x"));
    }
}

/// TeX's rule for an undelimited argument: without a `{...}` group, the
/// argument is exactly the next token -- one already-split character, or one
/// whole control sequence -- skipping leading spaces. See issue #78:
/// `$\hat A$` was accenting nothing because `\hat` demanded a brace.
#[cfg(test)]
mod unbraced_argument_tests {
    use super::*;

    #[test]
    fn unbraced_accent_takes_only_the_next_character() {
        // `\hat AB`: the argument is just "A"; "B" is an ordinary atom after
        // it, exactly like real TeX (and unlike the pre-fix empty-body bug).
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"\hat AB");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(list.atoms.len(), 2, "{:?}", list.atoms);
        match &list.atoms[0].nucleus {
            Nucleus::Accent { accent, body } => {
                assert_eq!(*accent, Accent::Hat);
                assert_eq!(body.atoms.len(), 1, "{:?}", body.atoms);
                assert_eq!(body.atoms[0].nucleus, Nucleus::Symbol("A".into()));
                // The body keeps its own real one-byte source span: "A" sits
                // at byte 5 in `\hat AB` (`\hat ` is 5 bytes).
                assert_eq!(body.atoms[0].span.start, 5);
                assert_eq!(body.atoms[0].span.end, 6);
            }
            other => panic!("expected an accent, got {other:?}"),
        }
        assert_eq!(list.atoms[1].nucleus, Nucleus::Symbol("B".into()));
        assert_eq!(list.atoms[1].span.start, 6);
        assert_eq!(list.atoms[1].span.end, 7);
    }

    #[test]
    fn unbraced_accent_argument_may_be_one_control_sequence() {
        for (source, accent, glyph) in [
            (r"\hat\alpha", Accent::Hat, "α"),
            (r"\vec\nabla", Accent::Vec, "∇"),
        ] {
            let mut diagnostics = Vec::new();
            let tokens = crate::lexer::tokenize(source);
            let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
            assert!(diagnostics.is_empty(), "{source}: {diagnostics:?}");
            assert_eq!(list.atoms.len(), 1, "{source}: {:?}", list.atoms);
            match &list.atoms[0].nucleus {
                Nucleus::Accent { accent: a, body } => {
                    assert_eq!(*a, accent, "{source}");
                    assert_eq!(body.atoms.len(), 1, "{source}: {:?}", body.atoms);
                    assert_eq!(body.atoms[0].nucleus, Nucleus::Symbol(glyph.into()));
                }
                other => panic!("{source}: expected an accent, got {other:?}"),
            }
        }
    }

    #[test]
    fn every_accent_family_accepts_an_unbraced_argument() {
        for command in [
            "hat",
            "bar",
            "vec",
            "tilde",
            "dot",
            "ddot",
            "check",
            "breve",
            "acute",
            "grave",
            "widehat",
            "widetilde",
        ] {
            let mut diagnostics = Vec::new();
            let source = format!(r"\{command} x");
            let tokens = crate::lexer::tokenize(&source);
            let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
            assert!(
                diagnostics
                    .iter()
                    .all(|d| !d.message.contains("requires an argument")),
                "{source}: {diagnostics:?}"
            );
            assert_eq!(list.atoms.len(), 1, "{source}: {:?}", list.atoms);
            match &list.atoms[0].nucleus {
                Nucleus::Accent { body, .. } => {
                    assert_eq!(body.atoms.len(), 1, "{source}: {:?}", body.atoms);
                    assert_eq!(body.atoms[0].nucleus, Nucleus::Symbol("x".into()));
                }
                other => panic!("{source}: expected an accent, got {other:?}"),
            }
        }
    }

    #[test]
    fn unbraced_sqrt_roots_only_the_next_token() {
        // `\sqrt 2x`: roots only "2"; "x" is outside the radical.
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"\sqrt 2x");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(list.atoms.len(), 2, "{:?}", list.atoms);
        match &list.atoms[0].nucleus {
            Nucleus::Radical(body) => {
                assert_eq!(body.atoms.len(), 1, "{:?}", body.atoms);
                assert_eq!(body.atoms[0].nucleus, Nucleus::Symbol("2".into()));
                assert_eq!(body.atoms[0].span.start, 6);
                assert_eq!(body.atoms[0].span.end, 7);
            }
            other => panic!("expected a radical, got {other:?}"),
        }
        assert_eq!(list.atoms[1].nucleus, Nucleus::Symbol("x".into()));
        assert_eq!(list.atoms[1].span.start, 7);
        assert_eq!(list.atoms[1].span.end, 8);
    }

    #[test]
    fn unbraced_frac_dfrac_tfrac_take_one_token_each() {
        // The task brief's headline case: `\frac12` is 1 over 2, not an
        // error.
        for command in ["frac", "dfrac", "tfrac"] {
            let mut diagnostics = Vec::new();
            let source = format!(r"\{command}12");
            let tokens = crate::lexer::tokenize(&source);
            let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
            assert!(diagnostics.is_empty(), "{source}: {diagnostics:?}");
            assert_eq!(list.atoms.len(), 1, "{source}: {:?}", list.atoms);
            match &list.atoms[0].nucleus {
                Nucleus::Fraction {
                    numerator,
                    denominator,
                }
                | Nucleus::GenFraction {
                    numerator,
                    denominator,
                    ..
                } => {
                    assert_eq!(numerator.atoms.len(), 1, "{source}: {:?}", numerator.atoms);
                    assert_eq!(numerator.atoms[0].nucleus, Nucleus::Symbol("1".into()));
                    assert_eq!(
                        denominator.atoms.len(),
                        1,
                        "{source}: {:?}",
                        denominator.atoms
                    );
                    assert_eq!(denominator.atoms[0].nucleus, Nucleus::Symbol("2".into()));
                    // Each half keeps its own exact one-byte source span.
                    let n = &numerator.atoms[0].span;
                    let d = &denominator.atoms[0].span;
                    assert_eq!(n.end - n.start, 1, "{source}");
                    assert_eq!(d.end - d.start, 1, "{source}");
                    assert_eq!(d.start, n.end, "{source}: the digits are adjacent bytes");
                }
                other => panic!("{source}: expected a fraction, got {other:?}"),
            }
        }
    }

    #[test]
    fn unbraced_binom_takes_one_token_per_argument() {
        for command in ["binom", "dbinom", "tbinom"] {
            let mut diagnostics = Vec::new();
            let source = format!(r"\{command} nk");
            let tokens = crate::lexer::tokenize(&source);
            let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
            assert!(diagnostics.is_empty(), "{source}: {diagnostics:?}");
            assert_eq!(list.atoms.len(), 1, "{source}: {:?}", list.atoms);
            match &list.atoms[0].nucleus {
                Nucleus::GenFraction {
                    numerator,
                    denominator,
                    left,
                    right,
                    style,
                    ..
                } => {
                    assert_eq!(numerator.atoms[0].nucleus, Nucleus::Symbol("n".into()));
                    assert_eq!(denominator.atoms[0].nucleus, Nucleus::Symbol("k".into()));
                    assert_eq!((left.as_str(), right.as_str()), ("(", ")"), "{source}");
                    let expected = match command {
                        "dbinom" => Some(MathStyle::Display),
                        "tbinom" => Some(MathStyle::Text),
                        _ => None,
                    };
                    assert_eq!(*style, expected, "{source}");
                }
                other => panic!("{source}: expected a generalized fraction, got {other:?}"),
            }
        }
    }

    /// A document that loaded `amsfonts`: its `\mathbb`/`\mathfrak` alphabets
    /// exist. Base LaTeX2e defines neither.
    const AMSFONTS: MathPackages = MathPackages {
        amsmath: false,
        amssymb: false,
        amsfonts: true,
    };

    #[test]
    fn math_alphabets_map_plain_letters_to_unicode_alphanumerics() {
        for (source, expected) in [
            (r"\mathsf{Ab1}", "\u{1D5A0}\u{1D5BB}\u{1D7E3}"),
            (r"\mathtt{T}", "\u{1D683}"),
            (r"\mathit{diff}", "\u{1D451}\u{1D456}\u{1D453}\u{1D453}"),
            (r"\mathit{h}", "\u{210E}"),
            (r"\mathfrak{gRA}", "\u{1D524}\u{211C}\u{1D504}"),
            (r"\mathfrak{g1}", "\u{1D524}1"),
        ] {
            let mut diagnostics = Vec::new();
            let list = parse_tokens(
                &crate::lexer::tokenize(source),
                AMSFONTS,
                &mut diagnostics,
            );
            assert!(diagnostics.is_empty(), "{source}: {diagnostics:?}");
            assert_eq!(list.atoms.len(), 1, "{source}: {:?}", list.atoms);
            assert_eq!(list.atoms[0].nucleus, Nucleus::Symbol(expected.into()), "{source}");
        }
        // Any other argument keeps the surrounding math letters.
        let mut diagnostics = Vec::new();
        let list = parse_tokens(
            &crate::lexer::tokenize(r"\mathsf{x^2}"),
            MathPackages::KERNEL,
            &mut diagnostics,
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(list.atoms[0].nucleus, Nucleus::Symbol("x".into()));
    }

    #[test]
    fn unbraced_mathbb_and_mathbf_take_one_letter() {
        // The issue's own examples: `\mathbb R` and `\mathbf v`.
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"\mathbb RS");
        let list = parse_tokens(&tokens, AMSFONTS, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(list.atoms.len(), 2, "{:?}", list.atoms);
        assert_eq!(
            list.atoms[0].nucleus,
            Nucleus::Symbol(crate::lm_math::double_struck('R').unwrap().to_string())
        );
        // The atom's span is the command merged with its argument (matching
        // the pre-existing braced-form convention); its end lands exactly at
        // "R", proving "S" was not swallowed along with it.
        assert_eq!(list.atoms[0].span.end, 9);
        assert_eq!(list.atoms[1].nucleus, Nucleus::Symbol("S".into()));
        assert_eq!(list.atoms[1].span.start, 9);
        assert_eq!(list.atoms[1].span.end, 10);

        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"\mathbf vw");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(list.atoms.len(), 2, "{:?}", list.atoms);
        assert_eq!(list.atoms[0].nucleus, Nucleus::Bold("v".into()));
        assert_eq!(list.atoms[0].span.end, 9);
        assert_eq!(list.atoms[1].nucleus, Nucleus::Symbol("w".into()));
        assert_eq!(list.atoms[1].span.start, 9);
        assert_eq!(list.atoms[1].span.end, 10);
    }

    #[test]
    fn unbraced_style_switches_match_their_braced_form() {
        // `\mathrm`, `\mathit`, `\mathsf`, `\mathtt`, `\boldsymbol` and `\bm`
        // flatten a group's first atom into the surrounding list; the
        // unbraced form must produce the identical atom.
        for command in ["mathrm", "mathit", "mathsf", "mathtt", "boldsymbol", "bm"] {
            let mut braced_diagnostics = Vec::new();
            let braced = crate::lexer::tokenize(&format!(r"\{command}{{d}}x"));
            let braced_list = parse_tokens(&braced, MathPackages::KERNEL, &mut braced_diagnostics);

            let mut unbraced_diagnostics = Vec::new();
            let unbraced = crate::lexer::tokenize(&format!(r"\{command} dx"));
            let unbraced_list = parse_tokens(
                &unbraced,
                MathPackages::KERNEL,
                &mut unbraced_diagnostics,
            );

            assert!(
                braced_diagnostics.is_empty() && unbraced_diagnostics.is_empty(),
                "{command}: {braced_diagnostics:?} {unbraced_diagnostics:?}"
            );
            assert_eq!(
                braced_list.atoms.len(),
                2,
                "{command}: {:?}",
                braced_list.atoms
            );
            assert_eq!(
                unbraced_list.atoms.len(),
                2,
                "{command}: {:?}",
                unbraced_list.atoms
            );
            assert_eq!(
                braced_list.atoms[0].nucleus, unbraced_list.atoms[0].nucleus,
                "{command}"
            );
            assert_eq!(
                braced_list.atoms[1].nucleus, unbraced_list.atoms[1].nucleus,
                "{command}"
            );
        }
    }

    #[test]
    fn unbraced_overline_underline_boxed_take_one_token() {
        for (command, frame) in [
            ("overline", Frame::Over),
            ("underline", Frame::Under),
            ("boxed", Frame::Box),
        ] {
            let mut diagnostics = Vec::new();
            let source = format!(r"\{command} xy");
            let tokens = crate::lexer::tokenize(&source);
            let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
            assert!(diagnostics.is_empty(), "{source}: {diagnostics:?}");
            assert_eq!(list.atoms.len(), 2, "{source}: {:?}", list.atoms);
            match &list.atoms[0].nucleus {
                Nucleus::Framed { body, frame: got } => {
                    assert_eq!(*got, frame, "{source}");
                    assert_eq!(body.atoms.len(), 1, "{source}: {:?}", body.atoms);
                    assert_eq!(body.atoms[0].nucleus, Nucleus::Symbol("x".into()));
                }
                other => panic!("{source}: expected a framed nucleus, got {other:?}"),
            }
            assert_eq!(list.atoms[1].nucleus, Nucleus::Symbol("y".into()));
        }
    }

    #[test]
    fn unbraced_text_takes_one_character_leaving_the_rest_as_math() {
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"\text nR");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(list.atoms.len(), 2, "{:?}", list.atoms);
        assert!(matches!(&list.atoms[0].nucleus, Nucleus::Text(text) if text == "n"));
        // Merged with the command's own span (see the `mathbb`/`mathbf`
        // test above); its end lands exactly at "n", not swallowing "R".
        assert_eq!(list.atoms[0].span.end, 7);
        assert_eq!(list.atoms[1].nucleus, Nucleus::Symbol("R".into()));
        assert_eq!(list.atoms[1].span.start, 7);
        assert_eq!(list.atoms[1].span.end, 8);
    }

    #[test]
    fn unbraced_and_braced_control_sequence_argument_diagnose_the_same_way() {
        // A control sequence isn't literal text: both the unbraced and
        // braced forms report it and keep its name literally, consistently.
        for source in [r"\mathbf\alpha", r"\mathbf{\alpha}"] {
            let mut diagnostics = Vec::new();
            let tokens = crate::lexer::tokenize(source);
            let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
            assert_eq!(diagnostics.len(), 1, "{source}: {diagnostics:?}");
            assert!(
                diagnostics[0]
                    .message
                    .contains("is not supported inside \\mathbf"),
                "{source}: {diagnostics:?}"
            );
            assert_eq!(list.atoms[0].nucleus, Nucleus::Bold("\\alpha".into()));
        }
    }

    #[test]
    fn superscript_without_braces_still_takes_only_the_next_token() {
        // Pre-existing `script_argument` behavior, verified here as the
        // reference the other undelimited arguments above now match:
        // `x^ab` is `x^a` followed by an ordinary "b", not `x^{ab}`.
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(r"x^ab");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(list.atoms.len(), 2, "{:?}", list.atoms);
        assert_eq!(list.atoms[0].nucleus, Nucleus::Symbol("x".into()));
        let sup = list.atoms[0].superscript.as_ref().expect("superscript");
        assert_eq!(sup.atoms.len(), 1, "{:?}", sup.atoms);
        assert_eq!(sup.atoms[0].nucleus, Nucleus::Symbol("a".into()));
        assert_eq!(list.atoms[1].nucleus, Nucleus::Symbol("b".into()));
    }
}

#[cfg(test)]
mod accent_tests {
    use super::*;

    fn laid_out(source: &str, size: f64) -> (MathBox, Vec<Diagnostic>) {
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize(source);
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        let b = layout(&list, size, &mut diagnostics);
        (b, diagnostics)
    }

    /// The measurement the task brief asked for: `\hat{A}`'s horizontal
    /// offset in this compiler. The math variable "A" is Times-Italic, so
    /// the accent now carries an italic-angle skew on top of symmetric
    /// centering — see `layout_accent` and `crate::layout::italic_skew_pt`
    /// for the derivation (TeX's real skewchar-kern rule has no equivalent
    /// in Adobe Core 14 AFM metrics, so this uses the font's real
    /// `ItalicAngle` instead).
    #[test]
    fn hat_a_skews_right_by_the_italic_angle_at_the_accent_height() {
        let size = 10.0;
        let (b, diagnostics) = laid_out(r"\hat{A}", size);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let a_item = b.items.iter().find(|i| i.text == "A").unwrap();
        let accent_item = b.items.iter().find(|i| i.text == "\u{2C6}").unwrap();

        let mut d = Vec::new();
        let a_width = crate::layout::shaped_width(
            "A",
            size,
            crate::layout::Font::TimesItalic,
            a_item.span,
            &mut d,
        )
        .0;
        let accent_width = crate::layout::shaped_width(
            "\u{2C6}",
            size,
            crate::layout::Font::TimesRoman,
            accent_item.span,
            &mut d,
        )
        .0;
        let center_dx = (a_width - accent_width) / 2.0;
        let raise = crate::layout::x_height_pt(crate::layout::Font::TimesRoman, size);
        let skew = crate::layout::italic_skew_pt(crate::layout::Font::TimesItalic, raise);
        let expected_dx = center_dx + skew;

        assert!(
            (accent_item.x - a_item.x - expected_dx).abs() < 1e-9,
            "expected skewed centering dx {expected_dx}, got {}",
            accent_item.x - a_item.x
        );
        // At 10pt: symmetric centering alone gives ~1.39pt (Times-Italic A
        // is 611 units wide, the circumflex 333). Adding the italic-angle
        // skew at the accent's raise height (Times-Roman's x-height, 450
        // units) brings the total to ~2.64pt — pdflatex measures cmmi10's
        // real skewchar kern for `\hat A` at 2.639pt.
        assert!(
            (expected_dx - 2.64).abs() < 0.01,
            "expected ~2.64pt at 10pt, got {expected_dx}"
        );
    }

    /// Upright bodies never get the italic skew: a digit, a multi-letter
    /// name, and Symbol-font Greek all keep plain symmetric centering.
    #[test]
    fn upright_accent_bodies_keep_symmetric_centering() {
        let size = 10.0;
        for source in [r"\hat{5}", r"\hat{AB}", r"\hat{\alpha}"] {
            let (b, diagnostics) = laid_out(source, size);
            assert!(diagnostics.is_empty(), "{source}: {diagnostics:?}");
            let accent_item = b.items.iter().find(|i| i.text == "\u{2C6}").unwrap();
            let mut d = Vec::new();
            let accent_width = crate::layout::shaped_width(
                "\u{2C6}",
                size,
                crate::layout::Font::TimesRoman,
                accent_item.span,
                &mut d,
            )
            .0;
            let expected_dx = (b.width - accent_width) / 2.0;
            assert!(
                (accent_item.x - expected_dx).abs() < 1e-9,
                "{source}: expected symmetric centering dx {expected_dx}, got {}",
                accent_item.x
            );
        }
    }

    /// Issue #78: `$\hat A$` (no braces) used to accent an empty body, giving
    /// a dx of 0 instead of the same centering `\hat{A}` produces.
    #[test]
    fn unbraced_hat_a_has_the_same_accent_dx_as_braced_hat_a() {
        let size = 10.0;
        let dx = |b: &MathBox| {
            let a_item = b.items.iter().find(|i| i.text == "A").unwrap();
            let accent_item = b.items.iter().find(|i| i.text == "\u{2C6}").unwrap();
            accent_item.x - a_item.x
        };
        let (braced, d1) = laid_out(r"\hat{A}", size);
        let (unbraced, d2) = laid_out(r"\hat A", size);
        assert!(d1.is_empty(), "{d1:?}");
        assert!(d2.is_empty(), "{d2:?}");
        assert!((dx(&braced) - dx(&unbraced)).abs() < 1e-9);
    }

    #[test]
    fn accent_vertical_raise_is_capped_at_the_accent_fonts_x_height() {
        let size = 10.0;
        let (b, diagnostics) = laid_out(r"\hat{A}", size);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let accent_item = b.items.iter().find(|i| i.text == "\u{2C6}").unwrap();
        let xheight = crate::layout::x_height_pt(crate::layout::Font::TimesRoman, size);
        assert!((accent_item.baseline - (-xheight)).abs() < 1e-9);
    }

    #[test]
    fn ddot_acute_grave_bar_use_exact_base14_glyphs() {
        for (source, glyph) in [
            (r"\ddot{x}", "\u{A8}"),
            (r"\acute{x}", "\u{B4}"),
            (r"\grave{x}", "\u{60}"),
            (r"\bar{x}", "\u{AF}"),
            (r"\tilde{x}", "\u{2DC}"),
        ] {
            let (b, diagnostics) = laid_out(source, 10.0);
            assert!(diagnostics.is_empty(), "{source}: {diagnostics:?}");
            assert!(
                b.items.iter().any(|i| i.text == glyph),
                "{source}: expected glyph {glyph:?} in {:?}",
                b.items.iter().map(|i| &i.text).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn check_and_breve_are_diagnosed_and_typeset_without_a_mark() {
        for source in [r"\check{x}", r"\breve{x}"] {
            let (b, diagnostics) = laid_out(source, 10.0);
            assert_eq!(diagnostics.len(), 1, "{source}: {diagnostics:?}");
            assert!(diagnostics[0]
                .message
                .contains("no representable accent glyph"));
            // Only the base "x", no extra accent glyph item.
            assert_eq!(b.items.len(), 1);
            assert_eq!(b.items[0].text, "x");
        }
    }

    #[test]
    fn widehat_is_silent_over_one_or_more_symbols() {
        // The accent grows with its body (cmex successor chain, msbm "5B past
        // 2em) in TFM-driven layouts, so a wide body is no longer diagnosed.
        let (_, one) = laid_out(r"\widehat{A}", 10.0);
        assert!(one.is_empty(), "{one:?}");
        let (_, many) = laid_out(r"\widetilde{ABC}", 10.0);
        assert!(many.is_empty(), "{many:?}");
    }

    #[test]
    fn overline_and_underline_draw_a_rule_spanning_the_body() {
        let size = 10.0;
        let (over, d1) = laid_out(r"\overline{x}", size);
        assert!(d1.is_empty(), "{d1:?}");
        let over_rule = over
            .items
            .iter()
            .find_map(|i| i.rule)
            .expect("overline rule");
        assert!(over_rule.width > 0.0);
        assert!(over_rule.height > 0.0);
        // Drawn above the body: strictly negative (upward) y.
        assert!(over_rule.y < 0.0);

        let (under, d2) = laid_out(r"\underline{x}", size);
        assert!(d2.is_empty(), "{d2:?}");
        let under_rule = under
            .items
            .iter()
            .find_map(|i| i.rule)
            .expect("underline rule");
        assert!(under_rule.width > 0.0);
        assert!(under_rule.height > 0.0);
        // Drawn below the body: strictly positive (downward) y.
        assert!(under_rule.y > 0.0);
    }
}

#[cfg(test)]
mod spacing_tests {
    use super::*;

    const SIZE: f64 = 18.0; // 1mu = 1pt

    fn laid_out(source: &str, size: f64) -> MathBox {
        laid_out_with(source, size, MathPackages::KERNEL)
    }

    fn laid_out_with(source: &str, size: f64, packages: MathPackages) -> MathBox {
        let mut diagnostics = Vec::new();
        let list = parse_tokens(&crate::lexer::tokenize(source), packages, &mut diagnostics);
        let b = layout(&list, size, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{source}: {diagnostics:?}");
        b
    }

    fn width(source: &str, size: f64) -> f64 {
        laid_out(source, size).width
    }

    /// A document that loaded `amssymb`, which requires `amsfonts`: the whole
    /// msam/msbm inventory exists. Base LaTeX2e defines none of it, so the
    /// tests below that use those commands have to say so — see
    /// `amssymb_commands_are_diagnosed_when_the_package_is_not_loaded`.
    const AMSSYMB: MathPackages = MathPackages {
        amsmath: false,
        amssymb: true,
        amsfonts: true,
    };

    fn width_with(source: &str, size: f64, packages: MathPackages) -> f64 {
        laid_out_with(source, size, packages).width
    }

    fn x(b: &MathBox, text: &str) -> f64 {
        b.items.iter().find(|i| i.text == text).unwrap().x
    }

    fn close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
    }

    #[test]
    fn relations_get_thick_space_on_both_sides() {
        let b = laid_out("a=b", SIZE);
        close(x(&b, "="), width("a", SIZE) + 5.0);
        close(x(&b, "b"), x(&b, "=") + width("=", SIZE) + 5.0);
    }

    #[test]
    fn binary_operators_get_medium_space() {
        let b = laid_out("a+b", SIZE);
        close(x(&b, "+"), width("a", SIZE) + 4.0);
        close(x(&b, "b"), x(&b, "+") + width("+", SIZE) + 4.0);
    }

    #[test]
    fn odot_is_a_binary_operator() {
        let b = laid_out(r"a\odot b", SIZE);
        close(x(&b, "⊙"), width("a", SIZE) + 4.0);
        close(x(&b, "b"), x(&b, "⊙") + width("⊙", SIZE) + 4.0);
    }

    #[test]
    fn a_leading_or_post_relation_minus_is_ordinary_and_a_real_minus_sign() {
        let b = laid_out("-x", SIZE);
        assert!(b.items.iter().all(|i| i.text != "-"), "{:?}", b.items);
        close(x(&b, "x"), width("-", SIZE));
        close(b.width, width("-", SIZE) + width("x", SIZE));
        // After a relation: thick space before the minus, none after it.
        let b = laid_out("a=-x", SIZE);
        close(x(&b, MINUS_SIGN), x(&b, "=") + width("=", SIZE) + 5.0);
        close(x(&b, "x"), x(&b, MINUS_SIGN) + width("-", SIZE));
    }

    #[test]
    fn script_style_drops_relation_space() {
        let script = SIZE * SCRIPT_SCALE;
        let b = laid_out("a_{i=1}", SIZE);
        close(
            b.width,
            width("a", SIZE) + width("i", script) + width("=", script) + width("1", script),
        );
    }

    #[test]
    fn math_punctuation_gets_thin_space_after_only() {
        let b = laid_out("f(x),y", SIZE);
        close(x(&b, "("), width("f", SIZE));
        close(x(&b, "y"), x(&b, ",") + width(",", SIZE) + 3.0);
        close(b.width, width("f(x),y", SIZE));
        close(
            b.width,
            ["f", "(", "x", ")", ",", "y"]
                .iter()
                .map(|s| width(s, SIZE))
                .sum::<f64>()
                + 3.0,
        );
    }

    #[test]
    fn explicit_glue_adds_to_the_table_spacing() {
        close(width(r"a\,=b", SIZE), width("a=b", SIZE) + 3.0);
        close(
            width(r"\sin x", SIZE),
            width(r"\sin", SIZE) + width("x", SIZE) + 3.0,
        );
    }

    #[test]
    fn new_relations_get_thick_space_like_other_relations() {
        for command in [
            "ll",
            "gg",
            "simeq",
            "mapsto",
            "parallel",
            "nmid",
            "nleq",
            "ngeq",
            "subsetneq",
            "supsetneq",
            "lesssim",
            "gtrsim",
            "triangleq",
            "coloneqq",
            "rightsquigarrow",
            "hookrightarrow",
            "leftrightarrows",
            "models",
            "vdash",
            "dashv",
        ] {
            // amssymb commands set their own table text at msam/msbm widths.
            let glyph = crate::amssymb::by_name(command)
                .map(|s| s.text)
                .unwrap_or_else(|| command_glyph(command).unwrap());
            // A space after a control word is swallowed by the lexer (like
            // real TeX), so it safely separates the command from `b`.
            let b = laid_out_with(&format!(r"a\{command} b"), SIZE, AMSSYMB);
            close(x(&b, glyph), width("a", SIZE) + 5.0);
            let own = width_with(&format!(r"\{command}"), SIZE, AMSSYMB);
            close(x(&b, "b"), x(&b, glyph) + own + 5.0);
        }
    }

    #[test]
    fn mp_and_circ_get_medium_space_like_other_binary_operators() {
        for command in ["mp", "circ"] {
            let glyph = command_glyph(command).unwrap();
            let b = laid_out(&format!(r"a\{command} b"), SIZE);
            close(x(&b, glyph), width("a", SIZE) + 4.0);
            close(x(&b, "b"), x(&b, glyph) + width(glyph, SIZE) + 4.0);
        }
    }

    /// The kernel cmsy square relations, whose classes come from
    /// `fontmath.ltx` 278-279 (`\sqcap`/`\sqcup`, `\mathbin`) and 301-302
    /// (`\sqsubseteq`/`\sqsupseteq`, `\mathrel`). Measured with pdfTeX
    /// 3.141592653 (TeX Live 2025) at 10pt against the 9.57755pt `$ab$`
    /// control: `$a\sqcup b$` and `$a\sqcap b$` are 20.68857pt (glyph
    /// 6.66669pt plus 8mu, so 4mu a side), `$a\sqsubseteq b$` and
    /// `$a\sqsupseteq b$` 22.91077pt (glyph 7.7778pt plus 10mu, so 5mu).
    #[test]
    fn square_relations_take_their_kernel_classes() {
        for (command, space) in [
            ("sqcup", 4.0),
            ("sqcap", 4.0),
            ("sqsubseteq", 5.0),
            ("sqsupseteq", 5.0),
        ] {
            let glyph = command_glyph(command).unwrap();
            let b = laid_out(&format!(r"a\{command} b"), SIZE);
            close(x(&b, glyph), width("a", SIZE) + space);
            close(x(&b, "b"), x(&b, glyph) + width(glyph, SIZE) + space);
        }
    }

    #[test]
    fn floor_and_ceiling_are_open_and_close_fences() {
        // Open fences get no leading space; close fences get no trailing space.
        let b = laid_out(r"a=\lfloor x\rfloor", SIZE);
        close(x(&b, "⌊"), x(&b, "=") + width("=", SIZE) + 5.0);
        close(x(&b, "x"), x(&b, "⌊") + width("⌊", SIZE));
        close(b.width, x(&b, "⌋") + width("⌋", SIZE));

        let b = laid_out(r"a=\lceil x\rceil", SIZE);
        close(x(&b, "⌈"), x(&b, "=") + width("=", SIZE) + 5.0);
        close(x(&b, "x"), x(&b, "⌈") + width("⌈", SIZE));
        close(b.width, x(&b, "⌉") + width("⌉", SIZE));
    }

    #[test]
    fn left_right_floor_and_ceiling_are_accepted_as_delimiters() {
        laid_out(r"\left\lfloor x \right\rfloor", SIZE);
        laid_out(r"\left\lceil x \right\rceil", SIZE);
    }

    #[test]
    fn oint_is_an_op_like_int_and_oint() {
        let b = laid_out(r"\oint_C f", SIZE);
        // Op class before an ordinary atom gets a thin space (3mu), same as \int.
        let int = laid_out(r"\int_C f", SIZE);
        close(b.width - width("∮", SIZE), int.width - width("∫", SIZE));
    }

    #[test]
    fn every_new_amssymb_command_renders_with_no_diagnostics() {
        for command in [
            "mp",
            "ll",
            "gg",
            "simeq",
            "vdots",
            "ddots",
            "lfloor",
            "rfloor",
            "lceil",
            "rceil",
            "oint",
            "mapsto",
            "ell",
            "hbar",
            "circ",
            "parallel",
            "nmid",
            "nleq",
            "ngeq",
            "subsetneq",
            "supsetneq",
            "lesssim",
            "gtrsim",
            "triangleq",
            "coloneqq",
            "nexists",
            "complement",
            "rightsquigarrow",
            "hookrightarrow",
            "leftrightarrows",
            "models",
            "vdash",
            "dashv",
            "top",
            "measuredangle",
            "square",
            "blacksquare",
            "lozenge",
            "checkmark",
        ] {
            laid_out_with(&format!(r"\{command}"), SIZE, AMSSYMB);
        }
    }

    /// Issue #62 HW2 follow-up: the remaining long arrows are Rel, same as
    /// the existing short arrows and `\Longrightarrow`.
    #[test]
    fn long_arrows_get_thick_space_like_other_relations() {
        for command in [
            "Longleftrightarrow",
            "longrightarrow",
            "longleftarrow",
            "Longleftarrow",
            "longleftrightarrow",
        ] {
            let glyph = command_glyph(command).unwrap();
            let b = laid_out(&format!(r"a\{command} b"), SIZE);
            close(x(&b, glyph), width("a", SIZE) + 5.0);
            close(x(&b, "b"), x(&b, glyph) + width(glyph, SIZE) + 5.0);
        }
    }

    /// `\iff`/`\implies`/`\impliedby` expand to a thick space, the long
    /// double arrow, and another thick space (mathtools/plain TeX) — deliberate
    /// extra room on top of the automatic Rel spacing the arrow already gets,
    /// exactly like plain TeX's real `\def\iff{\;\Longleftrightarrow\;}`.
    #[test]
    fn iff_implies_impliedby_expand_to_a_spaced_long_arrow() {
        for (command, arrow) in [("iff", "⟺"), ("implies", "⟹"), ("impliedby", "⟸")] {
            let b = laid_out(&format!(r"a\{command} b"), SIZE);
            close(x(&b, arrow), width("a", SIZE) + 10.0);
            close(x(&b, "b"), x(&b, arrow) + width(arrow, SIZE) + 10.0);
        }
    }

    /// `\triangle` is Ord (no space against an adjacent ordinary atom);
    /// `\bigtriangleup` renders the identical glyph but is Bin.
    #[test]
    fn triangle_is_ord_and_bigtriangleup_is_bin_on_the_same_glyph() {
        let ord = laid_out(r"a\triangle b", SIZE);
        close(x(&ord, "△"), width("a", SIZE));
        close(x(&ord, "b"), x(&ord, "△") + width("△", SIZE));

        let bin = laid_out(r"a\bigtriangleup b", SIZE);
        close(x(&bin, "△"), width("a", SIZE) + 4.0);
        close(x(&bin, "b"), x(&bin, "△") + width("△", SIZE) + 4.0);
    }

    #[test]
    fn bigtriangledown_is_a_distinct_bin_glyph() {
        let b = laid_out(r"a\bigtriangledown b", SIZE);
        close(x(&b, "▽"), width("a", SIZE) + 4.0);
        close(x(&b, "b"), x(&b, "▽") + width("▽", SIZE) + 4.0);
    }

    /// `\bot` and `\perp` render the exact same U+22A5 glyph but must space
    /// differently: `\bot` is Ord (no relation space), `\perp` is Rel (thick
    /// space on both sides).
    #[test]
    fn bot_and_perp_render_the_same_glyph_with_different_spacing() {
        let bot = laid_out(r"a\bot b", SIZE);
        close(x(&bot, "⊥"), width("a", SIZE));
        close(x(&bot, "b"), x(&bot, "⊥") + width("⊥", SIZE));

        let perp = laid_out(r"a\perp b", SIZE);
        close(x(&perp, "⊥"), width("a", SIZE) + 5.0);
        close(x(&perp, "b"), x(&perp, "⊥") + width("⊥", SIZE) + 5.0);

        assert!(bot.width < perp.width);
    }

    /// TeXbook Chapter 17's `\mathbin`/`\mathrel`/`\mathord`/`\mathop`/
    /// `\mathopen`/`\mathclose`/`\mathpunct`: the class is forced regardless
    /// of what the argument's own atoms would otherwise imply.
    #[test]
    fn math_class_family_forces_spacing_around_an_arbitrary_argument() {
        // HW2 uses `\mathbin{\triangle}` for symmetric difference: it must
        // get Bin (medium) spacing, unlike bare `\triangle` above.
        let b = laid_out(r"A\mathbin{\triangle}B", SIZE);
        close(x(&b, "△"), width("A", SIZE) + 4.0);
        close(x(&b, "B"), x(&b, "△") + width("△", SIZE) + 4.0);

        // `\mathord{=}` strips the relation spacing a bare `=` would get.
        let ord = laid_out(r"a\mathord{=}b", SIZE);
        close(x(&ord, "="), width("a", SIZE));
        close(x(&ord, "b"), x(&ord, "=") + width("=", SIZE));

        // `\mathrel{+}` adds relation (thick) spacing a bare `+` would not get.
        let rel = laid_out(r"a\mathrel{+}b", SIZE);
        close(x(&rel, "+"), width("a", SIZE) + 5.0);
        close(x(&rel, "b"), x(&rel, "+") + width("+", SIZE) + 5.0);

        // A multi-atom argument is boxed as one atom: `\mathbin{ab}` spaces
        // like a single Bin atom around the whole two-letter group, not like
        // two separate ordinary atoms with no internal gap removed.
        let group = laid_out(r"A\mathbin{ab}B", SIZE);
        close(x(&group, "a"), width("A", SIZE) + 4.0);
        close(x(&group, "b"), x(&group, "a") + width("a", SIZE));
        close(x(&group, "B"), x(&group, "b") + width("b", SIZE) + 4.0);
    }

    /// `\colon` sets the same `:` as a bare colon, but its class and its
    /// spacing come from whichever definition is in force.
    ///
    /// The kernel declares it punctuation (`fontmath.ltx` 400) where `:`
    /// itself is a relation (line 385), so it loses the relation's 5mu on
    /// each side and gains punctuation's 3mu after it only. amsmath renews it
    /// (`amsmath.sty` 409-410) to an *ordinary* `:` with explicit 2mu and 6mu
    /// glue around it.
    ///
    /// Measured against TeX Live 2025 pdflatex at 10pt, control
    /// `\hbox{$ab$}` = 9.57755pt:
    ///
    /// | box | no amsmath | with amsmath |
    /// | --- | ---: | ---: |
    /// | `\hbox{$\colon$}` | 2.77779pt | 7.22212pt (+8mu) |
    /// | `\hbox{$a\colon b$}` | 14.02196pt | 16.79967pt (+5mu) |
    /// | `\hbox{$a\colon=b$}` | 24.57747pt | 30.13289pt (+10mu) |
    #[test]
    fn colon_is_kernel_punctuation_until_amsmath_renews_it() {
        const AMS: MathPackages = MathPackages {
            amsmath: true,
            ..MathPackages::KERNEL
        };

        // Kernel: no space before, punctuation's thin space after — exactly
        // what `\mathpunct{:}` gives, and 5mu narrower than a bare `:`.
        let kernel = laid_out(r"a\colon b", SIZE);
        close(x(&kernel, ":"), width("a", SIZE));
        close(x(&kernel, "b"), x(&kernel, ":") + width(":", SIZE) + 3.0);
        close(kernel.width, width(r"a\mathpunct{:}b", SIZE));
        close(width("a:b", SIZE), kernel.width + 7.0);

        // amsmath: 2mu of glue, an ordinary `:`, then 6mu — no punctuation
        // space anywhere, so the `=` below still gets its own relation space.
        let ams = laid_out_with(r"a\colon b", SIZE, AMS);
        close(x(&ams, ":"), width("a", SIZE) + 2.0);
        close(x(&ams, "b"), x(&ams, ":") + width(":", SIZE) + 6.0);
        close(ams.width, kernel.width + 5.0);

        // Before a relation the gap widens to 10mu, and before a binary
        // operator to 13mu, because the kernel's punctuation atom changes
        // what comes *after* it and amsmath's ordinary one does not: the
        // kernel turns the `+` below into an Ord (TeXbook Chapter 17's
        // Bin-after-Punct rule) where amsmath leaves it a Bin.
        // pdflatex, 10pt: `a\colon=b` 24.57747pt -> 30.13289pt (+10mu),
        // `a\colon+b` 21.79976pt -> 29.0218pt (+13mu).
        close(
            laid_out_with(r"a\colon=b", SIZE, AMS).width,
            width(r"a\colon=b", SIZE) + 10.0,
        );
        close(
            laid_out_with(r"a\colon+b", SIZE, AMS).width,
            width(r"a\colon+b", SIZE) + 13.0,
        );

        // On its own the explicit glue survives on both sides, where the
        // kernel's punctuation space has no right neighbour to apply to:
        // 2.77779pt -> 7.22212pt, +8mu.
        close(
            laid_out_with(r"\colon", SIZE, AMS).width,
            width(r"\colon", SIZE) + 8.0,
        );

        // Neither definition is a bare `:`, which is a relation (5mu each
        // side) in both.
        assert_ne!(kernel.width, width("a:b", SIZE));
        assert_ne!(ams.width, width("a:b", SIZE));
    }

    /// The packages that carry amsmath's definitions in, and the ones that
    /// measured as leaving the kernel's alone. Each name below was checked by
    /// compiling `\documentclass[10pt]{article}\usepackage{X}` with TeX Live
    /// 2025 pdflatex and comparing `\hbox{$a a\colon b b$}` (23.5995pt with
    /// the kernel's definition, 26.37721pt with amsmath's) against the two
    /// hand-written expansions in the same document.
    #[test]
    fn amsmath_arrives_through_the_packages_and_classes_that_load_it() {
        for name in [
            "amsmath",
            "mathtools",
            "empheq",
            "physics",
            "nccmath",
            "mhchem",
        ] {
            let mut packages = MathPackages::KERNEL;
            packages.load_package(name);
            assert!(packages.amsmath, "{name} loads amsmath");
        }
        for name in [
            "amsthm",
            "amssymb",
            "amsfonts",
            "amsopn",
            "bm",
            "siunitx",
            "breqn",
            "unicode-math",
        ] {
            let mut packages = MathPackages::KERNEL;
            packages.load_package(name);
            assert!(!packages.amsmath, "{name} does not load amsmath");
        }
        for class in ["amsart", "amsbook", "amsproc", "acmart", "beamer"] {
            let mut packages = MathPackages::KERNEL;
            packages.load_class(class);
            assert!(packages.amsmath, "{class} loads amsmath");
        }
        for class in ["article", "report", "book", "memoir", "IEEEtran"] {
            let mut packages = MathPackages::KERNEL;
            packages.load_class(class);
            assert!(!packages.amsmath, "{class} does not load amsmath");
        }
    }

    #[test]
    fn qed_glyph_is_in_the_pinned_font_and_carries_no_export_loss() {
        assert!(crate::lm_math::advance('\u{220E}').is_some());
        assert!(matches!(
            crate::export::map_char('\u{220E}'),
            crate::export::Glyph::LatinModernMath
        ));
    }
}

#[cfg(test)]
mod shift_tests {
    use super::*;

    #[test]
    fn shifting_reaches_nested_spans() {
        let mut diagnostics = Vec::new();
        let tokens = crate::lexer::tokenize("\\frac{a^2}{b}");
        let list = parse_tokens(&tokens, MathPackages::KERNEL, &mut diagnostics);
        let shifted = shift_list(&list, 10);

        fn min_start(list: &MathList) -> usize {
            list.atoms
                .iter()
                .map(|a| {
                    let nested = match &a.nucleus {
                        Nucleus::Symbol(_) | Nucleus::SizedDelimiter { .. } => usize::MAX,
                        Nucleus::Text(_) => usize::MAX,
                        Nucleus::Space { .. } | Nucleus::Rule(_) => usize::MAX,
                        Nucleus::Fraction {
                            numerator,
                            denominator,
                        } => min_start(numerator).min(min_start(denominator)),
                        Nucleus::Radical(inner) => min_start(inner),
                        Nucleus::Bold(_) => usize::MAX,
                        Nucleus::Framed { body, .. } => min_start(body),
                        Nucleus::Stacked { base, over, under } => {
                            [Some(base), over.as_ref(), under.as_ref()]
                                .into_iter()
                                .flatten()
                                .map(min_start)
                                .min()
                                .unwrap_or(usize::MAX)
                        }
                        Nucleus::Matrix { rows, .. } => rows
                            .iter()
                            .flatten()
                            .map(min_start)
                            .min()
                            .unwrap_or(usize::MAX),
                        Nucleus::Accent { body, .. } => min_start(body),
                        Nucleus::Group(body)
                        | Nucleus::Phantom { body, .. }
                        | Nucleus::Operator { body, .. } => min_start(body),
                        Nucleus::GenFraction {
                            numerator,
                            denominator,
                            ..
                        } => min_start(numerator).min(min_start(denominator)),
                        Nucleus::ExtArrow { above, below, .. } => {
                            min_start(above).min(min_start(below))
                        }
                        Nucleus::SubArray { rows, .. } => {
                            rows.iter().map(min_start).min().unwrap_or(usize::MAX)
                        }
                    };
                    let scripts = a
                        .superscript
                        .as_ref()
                        .map(min_start)
                        .unwrap_or(usize::MAX)
                        .min(a.subscript.as_ref().map(min_start).unwrap_or(usize::MAX));
                    a.span.start.min(nested).min(scripts)
                })
                .min()
                .unwrap_or(usize::MAX)
        }

        assert_eq!(min_start(&shifted), min_start(&list) + 10);
    }
}

/// The packages a document loads decide which math commands exist at all.
///
/// Every expectation here was probed against TeX Live 2025 pdflatex: each
/// command's `\meaning` under an empty preamble, under `\usepackage{amssymb}`,
/// under `\usepackage{amsfonts}` and under `\usepackage{amsmath}`, and the box
/// dimensions with `\showthe\wd`/`\ht` of `\hbox{$...$}`.
#[cfg(test)]
mod package_gating_tests {
    use super::*;
    use crate::amssymb::Provider;

    /// 1mu = 1pt, so a measured mu reads straight off a coordinate.
    const SIZE: f64 = 18.0;

    const AMSSYMB: MathPackages = MathPackages {
        amsmath: false,
        amssymb: true,
        amsfonts: true,
    };
    const AMSFONTS: MathPackages = MathPackages {
        amsmath: false,
        amssymb: false,
        amsfonts: true,
    };
    const AMSMATH: MathPackages = MathPackages {
        amsmath: true,
        amssymb: false,
        amsfonts: false,
    };

    fn parsed(source: &str, packages: MathPackages) -> (MathList, Vec<Diagnostic>) {
        let mut diagnostics = Vec::new();
        let list = parse_tokens(&crate::lexer::tokenize(source), packages, &mut diagnostics);
        (list, diagnostics)
    }

    fn laid_out(source: &str, packages: MathPackages) -> MathBox {
        let (list, diagnostics) = parsed(source, packages);
        assert!(diagnostics.is_empty(), "{source}: {diagnostics:?}");
        layout(&list, SIZE, &mut Vec::new())
    }

    fn x(b: &MathBox, text: &str) -> f64 {
        b.items
            .iter()
            .find(|i| i.text == text)
            .unwrap_or_else(|| panic!("{text:?} not in {:?}", b.items))
            .x
    }

    fn close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
    }

    /// Every command of the table needs its declaring package, and gets it
    /// from `amssymb` — which requires `amsfonts`, so the whole inventory is
    /// available under either flag combination a real document produces.
    ///
    /// This is the divergence the lane exists for: base LaTeX2e has no
    /// definition for any of these 212 names, so pdflatex answers "Undefined
    /// control sequence" where this compiler used to render the msam/msbm
    /// glyph regardless of what the document loaded.
    #[test]
    fn every_table_command_is_diagnosed_without_its_package() {
        let mut checked = 0;
        for name in crate::amssymb::command_names() {
            let source = format!("\\{name}");
            let (_, kernel) = parsed(&source, MathPackages::KERNEL);
            assert_eq!(kernel.len(), 1, "\\{name} with nothing loaded: {kernel:?}");
            let package = match crate::amssymb::by_name(name).expect("named symbol").provider {
                Provider::Amsfonts => "amsfonts",
                Provider::Amssymb => "amssymb",
            };
            assert_eq!(
                kernel[0].message,
                format!("\\{name} requires \\usepackage{{{package}}}"),
                "\\{name}"
            );

            let (_, loaded) = parsed(&source, AMSSYMB);
            assert!(loaded.is_empty(), "\\{name} under amssymb: {loaded:?}");
            checked += 1;
        }
        // 212 declarations plus the 6 `\global\let` aliases.
        assert_eq!(checked, 218, "table command count");
    }

    /// `amsfonts.sty` alone declares a 15-command subset of the same symbol
    /// fonts (plus the pieces the dashed arrows and wide accents are built
    /// from). `\usepackage{amsfonts}` provides exactly those, and `\nleq` and
    /// the other 197 stay undefined — probed name by name with `\ifcsname`.
    #[test]
    fn amsfonts_alone_provides_only_its_own_subset() {
        let subset: Vec<&str> = crate::amssymb::command_names()
            .filter(|n| {
                crate::amssymb::by_name(n).expect("named symbol").provider == Provider::Amsfonts
            })
            .collect();
        assert_eq!(
            subset,
            [
                "square",
                "lozenge",
                "rightsquigarrow",
                "vartriangleright",
                "vartriangleleft",
                "trianglerighteq",
                "trianglelefteq",
                "ulcorner",
                "urcorner",
                "llcorner",
                "lrcorner",
                "yen",
                "checkmark",
                "circledR",
                "maltese",
            ],
            "the amsfonts.sty declarations (141-147, 74-77, 64-73)"
        );
        for name in &subset {
            let (_, diagnostics) = parsed(&format!("\\{name}"), AMSFONTS);
            assert!(
                diagnostics.is_empty(),
                "\\{name} under amsfonts: {diagnostics:?}"
            );
        }
        for name in ["nleq", "boxdot", "circlearrowright"] {
            let (_, diagnostics) = parsed(&format!("\\{name}"), AMSFONTS);
            assert_eq!(diagnostics.len(), 1, "\\{name} under amsfonts");
        }
    }

    /// The `\global\let` aliases are all `amssymb.sty`'s, and each resolves to
    /// a symbol `amssymb.sty` declares, so gating the alias on its target's
    /// provider gates it on `amssymb` — which is what pdflatex does.
    #[test]
    fn every_alias_resolves_to_an_amssymb_declaration() {
        for (alias, target) in crate::amssymb::ALIASES {
            let symbol = crate::amssymb::by_name(alias).expect("alias resolves");
            assert_eq!(symbol.name, *target);
            assert_eq!(symbol.provider, Provider::Amssymb, "\\{alias}");
        }
    }

    /// The two math alphabets and the dashed arrows are `amsfonts.sty`'s too,
    /// and are gated the same way. `\mathbf`, which the obsolete `\bold`
    /// stands for, is the kernel's and stays available.
    #[test]
    fn amsfonts_alphabets_and_dashed_arrows_need_the_package() {
        for name in ["mathbb", "mathfrak", "Bbb", "bold"] {
            let (_, diagnostics) = parsed(&format!("\\{name}{{R}}"), MathPackages::KERNEL);
            assert_eq!(
                diagnostics.first().map(|d| d.message.as_str()),
                Some(format!("\\{name} requires \\usepackage{{amsfonts}}").as_str()),
                "\\{name}"
            );
            let (_, loaded) = parsed(&format!("\\{name}{{R}}"), AMSFONTS);
            assert!(loaded.is_empty(), "\\{name} under amsfonts: {loaded:?}");
        }
        for name in ["dashrightarrow", "dasharrow", "dashleftarrow"] {
            let (_, diagnostics) = parsed(&format!("\\{name}"), MathPackages::KERNEL);
            assert_eq!(diagnostics.len(), 1, "\\{name}");
            let (_, loaded) = parsed(&format!("\\{name}"), AMSFONTS);
            assert!(loaded.is_empty(), "\\{name} under amsfonts: {loaded:?}");
        }
        // The kernel alphabet the obsolete spelling redirects to.
        let (_, kernel) = parsed(r"\mathbf{v}", MathPackages::KERNEL);
        assert!(kernel.is_empty(), "{kernel:?}");
    }

    /// `\angle` and `\hbar` are the two commands here that base LaTeX2e does
    /// define, as composites amsfonts replaces with one glyph. Measured with
    /// TeX Live 2025 pdflatex at 10pt: `\hbox{$\angle$}` 6.37344pt without the
    /// package against 7.22223pt with it, `\hbox{$\hbar$}` 5.76172pt against
    /// 5.40280pt. Neither changes atom class: `$a\angle b$` grows by exactly
    /// the width difference (15.95099 -> 16.79977), and `$a\hbar b$` shrinks
    /// by it (15.33926 -> 14.98035).
    #[test]
    fn angle_and_hbar_keep_the_kernel_composite_without_amsfonts() {
        for (name, em) in [("angle", KERNEL_ANGLE_EM), ("hbar", KERNEL_HBAR_EM)] {
            let (kernel, diagnostics) = parsed(&format!("\\{name}"), MathPackages::KERNEL);
            assert!(diagnostics.is_empty(), "\\{name}: {diagnostics:?}");
            assert_eq!(kernel.atoms[0].width_em, Some(em), "\\{name}");

            // With the package the compiler's glyph row applies unchanged, so
            // nothing forces the kernel advance any more.
            let (loaded, diagnostics) = parsed(&format!("\\{name}"), AMSSYMB);
            assert!(diagnostics.is_empty(), "\\{name}: {diagnostics:?}");
            assert_eq!(loaded.atoms[0].width_em, None, "\\{name}");
            assert_eq!(loaded.atoms[0].nucleus, kernel.atoms[0].nucleus, "\\{name}");
        }
        // pdflatex's widths at 10pt, which the two constants reproduce.
        close(KERNEL_ANGLE_EM * 10.0, 6.37344);
        close(KERNEL_HBAR_EM * 10.0, 5.76172);
    }

    /// `\bmod` cancels `\medmuskip` and puts an explicit `\mkern5mu` in its
    /// place, so it is 1mu wider on each side than its Bin class alone — under
    /// every package, amsmath included. Measured at 10pt: `$a\bmod b$`
    /// 34.29970pt against `$a\mathrm{mod}b$` 28.74428pt (10mu), and
    /// `$\bmod b$` 24.56947 against `$\mathrm{mod}b$` 23.45839 (2mu), the
    /// second being the Bin degrading to Ord with no left operand.
    #[test]
    fn bmod_is_five_mu_on_each_side_not_the_four_of_its_class() {
        for packages in [MathPackages::KERNEL, AMSMATH, AMSSYMB] {
            let own = laid_out(r"\bmod", packages).width - 2.0 * BMOD_EXTRA_MU;

            let mid = laid_out(r"a\bmod b", packages);
            let a = laid_out("a", packages).width;
            close(x(&mid, "mod"), a + 5.0);
            close(x(&mid, "b"), x(&mid, "mod") + own + 5.0);

            // No left operand: the Bin becomes Ord and only the explicit kern
            // and the cancelled medmuskip are left.
            let lead = laid_out(r"\bmod b", packages);
            close(x(&lead, "mod"), BMOD_EXTRA_MU);
            close(x(&lead, "b"), x(&lead, "mod") + own + BMOD_EXTRA_MU);
        }
    }

    /// The kernel's `\pmod` opens with `\mkern18mu`; amsmath's `\pod` uses
    /// `\mkern8mu` outside display. Measured at 10pt: `$a\pmod{y}$` is
    /// 50.82503pt under the kernel and 45.26960pt under amsmath, matching
    /// `$a\mkern18mu(\mathrm{mod}\mkern6mu y)$` and the 8mu form exactly; the
    /// 6mu between `mod` and the argument is the same in both. Setting
    /// `\@displaytrue` by hand puts amsmath back on the kernel's 50.82503,
    /// which is why display formulas are left alone.
    #[test]
    fn pmod_opens_with_eight_mu_under_amsmath_and_eighteen_without() {
        for (packages, opening) in [
            (MathPackages::KERNEL, 18.0),
            (AMSSYMB, 18.0),
            (AMSMATH, AMSMATH_POD_MU),
        ] {
            let b = laid_out(r"a\pmod{y}", packages);
            let a = laid_out("a", packages).width;
            close(x(&b, "(mod"), a + opening);
            // The 6mu before the argument does not move.
            close(x(&b, "y") - x(&b, "(mod"), {
                let label = laid_out(r"\pmod{y}", MathPackages::KERNEL);
                x(&label, "y") - x(&label, "(mod")
            });
        }
    }

    /// Which `\usepackage` and `\documentclass` names set which flag, measured
    /// by compiling each one and asking `\ifcsname nleq\endcsname` (amssymb
    /// only), `\ifcsname ulcorner\endcsname` (also amsfonts) and
    /// `\ifcsname binom\endcsname` (amsmath) under TeX Live 2025.
    #[test]
    fn loaders_match_the_measured_packages_and_classes() {
        let package = |name: &str| {
            let mut p = MathPackages::KERNEL;
            p.load_package(name);
            p
        };
        let class = |name: &str| {
            let mut p = MathPackages::KERNEL;
            p.load_class(name);
            p
        };
        // amssymb requires amsfonts, so it sets both; amsfonts sets only its own.
        assert_eq!(package("amssymb"), AMSSYMB);
        assert_eq!(package("amsfonts"), AMSFONTS);
        assert_eq!(package("amsmath"), AMSMATH);
        // A math font package that really does load amssymb.
        assert_eq!(package("txfonts"), AMSSYMB);
        // Defines part of the inventory itself without either AMS package, so
        // neither flag describes it.
        assert_eq!(package("libertinust1math"), MathPackages::KERNEL);
        for neither in ["amsthm", "bm", "stmaryrd", "siunitx", "fourier", "unicode-math"] {
            let p = package(neither);
            assert!(!p.amssymb && !p.amsfonts, "{neither}");
        }
        // The AMS classes load amsfonts and amsmath, but not amssymb.
        assert_eq!(
            class("amsart"),
            MathPackages {
                amsmath: true,
                amssymb: false,
                amsfonts: true
            }
        );
        assert_eq!(
            class("beamer"),
            MathPackages {
                amsmath: true,
                amssymb: true,
                amsfonts: true
            }
        );
        assert_eq!(class("article"), MathPackages::KERNEL);
    }
}

/// `\|`/`\Vert` against `\mid`: two different symbols, not one spelled twice.
#[cfg(test)]
mod double_bar_tests {
    use super::*;

    /// `\lVert`/`\rVert`/`\lvert`/`\rvert` are amsmath's, so they need it
    /// loaded to exist at all (`package_gating_tests`).
    const AMSMATH: MathPackages = MathPackages {
        amsmath: true,
        amssymb: false,
        amsfonts: false,
    };

    /// The glyph texts a formula lays out, in order.
    fn texts(source: &str, packages: MathPackages) -> Vec<String> {
        let mut diagnostics = Vec::new();
        let list = parse_tokens(&crate::lexer::tokenize(source), packages, &mut diagnostics);
        let b = layout(&list, 10.0, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{source}: {diagnostics:?}");
        b.items.iter().map(|i| i.text.clone()).collect()
    }

    /// plain.tex gives `\Vert` cmsy `"6B` and the cmex `"0D` recipe, `\mid`
    /// cmsy `"6A` and cmex `"0C`. Spelling the first as two of the second put
    /// a 2.77779 pt bar where a 5.00002 pt one belongs at text size, and the
    /// 3.33333 pt single-bar extension where the 5.55557 pt double-bar one
    /// belongs once the delimiter grows.
    #[test]
    fn every_spelling_of_the_double_bar_is_one_u2016() {
        // `\Vert` on its own is still an unsupported command here (it is
        // only a fence name, `DELIMITER_COMMANDS`); `\|` is its spelling
        // that parses everywhere.
        assert_eq!(texts(r"\|", MathPackages::KERNEL), vec!["\u{2016}"]);
        for source in [r"\lVert", r"\rVert"] {
            assert_eq!(texts(source, AMSMATH), vec!["\u{2016}"], "{source}");
        }
        for source in [r"\left\| x \right\|", r"\left\Vert x \right\Vert"] {
            let t = texts(source, MathPackages::KERNEL);
            assert!(
                t.iter().filter(|s| *s == "\u{2016}").count() == 2
                    && !t.iter().any(|s| s.contains('\u{2223}')),
                "{source}: {t:?}"
            );
        }
    }

    /// `\mid` and `\vert` keep the single bar they always had.
    #[test]
    fn the_single_bar_commands_are_unchanged() {
        assert_eq!(texts(r"\mid", MathPackages::KERNEL), vec!["\u{2223}"]);
        assert_eq!(texts(r"\lvert", AMSMATH), vec!["\u{2223}"]);
        assert_eq!(texts(r"\rvert", AMSMATH), vec!["\u{2223}"]);
    }

    /// U+2016 is bound to the same pinned Latin Modern Math resource that
    /// already carries `\parallel`, so it stays exportable.
    #[test]
    fn the_double_bar_is_bound_to_latin_modern_math() {
        assert!(crate::lm_math::advance('\u{2016}').is_some());
        assert!(crate::export::unrepresentable("\u{2016}").is_empty());
    }
}
