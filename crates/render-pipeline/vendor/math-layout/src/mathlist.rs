//! The math list model: atoms with a class, a nucleus, and optional scripts.

use crate::boxes::{Flex, GlueOrder};
use crate::source::SourceTag;

/// A `plus`/`minus` component of explicit math glue before the style's math
/// unit is known: `mu` math units plus `pt` points for a finite order; for
/// `fil`/`fill`/`filll` the amount is `mu + pt` in fil units, never scaled
/// by the math unit (tex.web §716 multiplies only `normal` components).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MathFlex {
    pub mu: f64,
    pub pt: f64,
    pub order: GlueOrder,
}

impl MathFlex {
    pub const ZERO: MathFlex = MathFlex {
        mu: 0.0,
        pt: 0.0,
        order: GlueOrder::Normal,
    };

    /// A finite component in math units (`\mskip ... plus 2mu`).
    pub fn mu(mu: f64) -> MathFlex {
        MathFlex {
            mu,
            ..MathFlex::ZERO
        }
    }

    /// A finite component in points (`\hskip ... minus 1pt`).
    pub fn pt(pt: f64) -> MathFlex {
        MathFlex {
            pt,
            ..MathFlex::ZERO
        }
    }

    /// An infinite component (`plus 1fill` is `infinite(1.0, Fill)`).
    pub fn infinite(amount: f64, order: GlueOrder) -> MathFlex {
        MathFlex {
            mu: 0.0,
            pt: amount,
            order,
        }
    }

    /// The component in points (fil units for infinite orders) with the
    /// math unit `mu_pt`.
    pub fn resolve(self, mu_pt: f64) -> Flex {
        let amount = match self.order {
            GlueOrder::Normal => self.mu * mu_pt + self.pt,
            _ => self.mu + self.pt,
        };
        Flex {
            amount,
            order: self.order,
        }
    }
}

/// TeX's eight atom classes (TeXbook ch. 17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl AtomClass {
    pub(crate) fn index(self) -> usize {
        match self {
            AtomClass::Ord => 0,
            AtomClass::Op => 1,
            AtomClass::Bin => 2,
            AtomClass::Rel => 3,
            AtomClass::Open => 4,
            AtomClass::Close => 5,
            AtomClass::Punct => 6,
            AtomClass::Inner => 7,
        }
    }
}

/// Limit placement for `Op` atoms (`\limits`, `\nolimits`, `\displaylimits`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Limits {
    /// Limits in display style, scripts otherwise (TeX's default).
    #[default]
    DisplayLimits,
    Limits,
    NoLimits,
}

/// How a `\big`/`\Big`/`\bigg`/`\Bigg` delimiter is sized. The two
/// definitions are genuinely different, not two spellings of one rule, and
/// which is in force depends only on whether `amsmath` is loaded.
///
/// `\showbox` under pdfTeX 3.141592653-2.6-1.40.27 (TeX Live 2025),
/// `\Big[` in an `article`, as the delimiter glyph's own box (height+depth):
///
/// | body size | no `amsmath` | `amsmath` |
/// |---|---|---|
/// | 10pt | 18.00017 (`cmex` `h`) | 18.00017 (`cmex` `h`) |
/// | 11pt | 18.00017 (`cmex` `h`) | 19.71019 (`cmex` `h`) |
/// | 12pt | 18.00017 (`cmex` `h`) | 21.60020 (`cmex` `h`) |
///
/// The kernel column does not move with the body size because its target is
/// an absolute number of points *and* family 3 is `sfixed*cmex10`; the
/// amsmath column moves with both.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BigSizing {
    /// The LaTeX kernel's definition (`fontmath.ltx` 513-520), in force
    /// whenever `amsmath` is **not** loaded:
    /// `\hbox{$\left<delim>\vbox to<pt>{}\right.\n@space$}` with `pt`
    /// **8.5** (`\big`), **11.5** (`\Big`), **14.5** (`\bigg`), **17.5**
    /// (`\Bigg`) — absolute lengths, independent of the body size, the math
    /// size and the fonts.
    ///
    /// The box is a `\vbox`, so its height is `pt` and its depth is zero;
    /// Rule 19's δ is therefore `max(pt − axis, axis)`, not `pt/2`.
    Kernel { pt: f64 },
    /// amsmath's redefinition (`amsmath.sty` 721-738 `\bBigg@`), in force
    /// whenever `amsmath` is loaded:
    /// `\hbox{$\nulldelimiterspace0pt \left<delim>\vcenter to<factor>\big@size{}\right.$}`
    /// with `\big@size` = 1.2 × (height + depth) of `\Mathstrutbox@` (the
    /// text-size roman `(`) and `factor` 1, 1.5, 2, 2.5.
    ///
    /// The box is a `\vcenter`, so it straddles the axis and Rule 19's δ is
    /// exactly half the target.
    Amsmath { factor: f64 },
}

impl BigSizing {
    /// The kernel's `\big`…`\Bigg` for amsmath's 1 / 1.5 / 2 / 2.5, so a
    /// caller that only knows which of the four commands it saw can ask for
    /// either rule. 1 → 8.5pt, 1.5 → 11.5, 2 → 14.5, 2.5 → 17.5.
    pub fn kernel_for_factor(factor: f64) -> BigSizing {
        BigSizing::Kernel {
            pt: 8.5 + 6.0 * (factor - 1.0),
        }
    }
}

/// What sits in the nucleus of an atom.
#[derive(Debug, Clone, PartialEq)]
pub enum Nucleus {
    /// A single symbol resolved to a glyph by the metrics provider.
    Symbol(char),
    /// A braced subformula.
    List(MathList),
    /// `\frac{num}{den}`. `thickness` overrides the default rule thickness
    /// (0 gives `\atop`-style stacking). `left`/`right` are the delimiters of
    /// `\abovewithdelims`/`\atopwithdelims` (amsmath `\genfrac`, `\binom`),
    /// sized to `\delim1`/`\delim2` by Rule 15e; `None` is the null
    /// delimiter (`\nulldelimiterspace`), which is what plain `\over` has.
    Fraction {
        numerator: MathList,
        denominator: MathList,
        thickness: Option<f64>,
        left: Option<char>,
        right: Option<char>,
    },
    /// `\big`/`\Big`/`\bigg`/`\Bigg`, sized by whichever of the two
    /// definitions is in force ([`BigSizing`]). `None` is `\big.`.
    BigDelimiter { delim: Option<char>, sizing: BigSizing },
    /// `\phantom`/`\hphantom`/`\vphantom` (`latex.ltx` `\ph@nt`/`\finph@nt`):
    /// an empty box with the width (`horizontal`) and/or height and depth
    /// (`vertical`) of `body` set in the current (uncramped) style.
    Phantom {
        body: MathList,
        horizontal: bool,
        vertical: bool,
    },
    /// amsmath's `subarray` environment (`\substack` is `subarray{c}`): rows
    /// in `\scriptstyle`, aligned `c` or `l`, stacked at `\baselineskip` =
    /// `\fontdimen10`+`\fontdimen12` of `\scriptfont2` with `\lineskip` =
    /// `\lineskiplimit` = 3 × `\fontdimen8 \scriptfont3`, `\vcenter`ed.
    SubArray { rows: Vec<MathList>, align: char },
    /// Explicit math glue (`\,` `\:` `\;` `\!` `\mskip`, `\quad` `\hskip`):
    /// `mu` math units of the current style plus `pt` points. Like TeX's glue
    /// node it takes no part in inter-atom spacing (it does not change
    /// `r_type`, tex.web §760), so the atom's class is ignored. `stretch`
    /// and `shrink` are its `plus`/`minus` components (`\hfill` is
    /// `plus 1fill`); finite ones are converted from mu with the style's
    /// math unit like the natural size (§716).
    Glue {
        mu: f64,
        pt: f64,
        stretch: MathFlex,
        shrink: MathFlex,
    },
    /// `\sqrt{radicand}` or `\sqrt[degree]{radicand}`.
    Radical {
        radicand: MathList,
        degree: Option<MathList>,
    },
    /// Upright operator text such as `\lim` or `\sin` (`\operator@font`):
    /// each character is a text glyph of the roman font, no italic correction.
    Text(String),
    /// One character of the upright text family (`\fam0`: `\mathrm{K}`,
    /// `\mathop{\operator@font d}`) set as a math character: TeX §1186 turns
    /// a group holding a single ordinary character into that character, so
    /// its scripts follow Rule 18a (`shift_up` starts at 0) and an operator is
    /// centred on the axis. A [`Nucleus::Text`] of several characters is a box.
    TextChar(char),
    /// `\overline{body}`: body under a rule (Rule 9).
    Overline(MathList),
    /// `\underline{body}`: body over a rule (Rule 10).
    Underline(MathList),
    /// `{\displaystyle body}` and friends: an explicit style override for the
    /// body, boxed as an ordinary atom. This is the `\mathchoice`-free way to
    /// force a style; the body's own sub-formulas derive from it as usual.
    Styled {
        style: crate::style::Style,
        body: MathList,
    },
    /// `\hat{base}` and friends; `accent` is the accent symbol.
    Accent { accent: char, base: MathList },
    /// `\left l body \right r`. `None` is a null delimiter.
    Delimited {
        left: Option<char>,
        right: Option<char>,
        body: MathList,
    },
    /// amsmath `\ext@arrow` (`amsmath.sty` 1012-1026) over an `\arrowfill@`
    /// (971-976): `$\displaystyle left\mkern-7mu\cleaders\hbox{$\mkern-2mu
    /// fill\mkern-2mu$}\hfill\mkern-7mu right$` at the text size with every
    /// muskip zero, in an hbox as wide as the widest of its natural width and
    /// `\scriptstyle\mkern kerns[2]mu{label}\mkern kerns[3]mu` for either
    /// label; then `\mathop{..}\limits` with `^{\mkern kerns[0]mu above
    /// \mkern kerns[1]mu}` and `_{..below..}` for the non-empty labels. A
    /// minus piece (`\relbar`, `\mathsm@sh` of the minus) has no height or
    /// depth.
    ExtArrow {
        left: char,
        fill: char,
        right: char,
        kerns: [f64; 4],
        above: MathList,
        below: MathList,
    },
    /// `\overbrace{body}` (`under` false) / `\underbrace{body}` (`fontmath.ltx`
    /// 430-437, 447-456): `body` in `\displaystyle`, centred in an `\ialign`
    /// column with a `\downbracefill` (`\upbracefill`) row as wide as the
    /// column, `\kern3pt` above and below that row, packed as a `\vbox`
    /// (`\vtop`). The brace row is the four cmex pieces `\braceld`
    /// `\braceru` `\bracelu` `\bracerd` (`\bracelu` `\bracerd` `\braceld`
    /// `\braceru`) with `\leaders\vrule` of `\braceld`'s height and no depth
    /// filling two `\hfill`s. The atom carrying it is `\mathop..\limits`.
    Brace { body: MathList, under: bool },
    /// amsmath `\overrightarrow`/`\overleftarrow`/`\overleftrightarrow`
    /// (`amsmath.sty` 983-990, `\overarrow@`) and `\underrightarrow`..
    /// (1000-1006, `\underarrow@`): an `\arrowfill@` row of `left`, `fill`
    /// leaders and `right` in the current style, as wide as `body` set in
    /// that (uncramped) style, stacked directly above `body` in a `\vbox`
    /// (`\nointerlineskip`), or below it after a `gap` pt kern
    /// (`\kern1.3\ex@`) in a `\vtop`.
    OverArrow {
        left: char,
        fill: char,
        right: char,
        body: MathList,
        under: bool,
        gap: f64,
    },
    /// amsfonts `\widehat`/`\widetilde` (`amsfonts.sty` 78-86): `\mathaccent`
    /// `narrow` (the cmex successor chain) unless `body` measured in an
    /// `\hbox{$\textstyle ..$}` is wider than `threshold` pt (`2em` of the
    /// text font), then `wide` (msbm `"5B`/`"5D`). Laid out as that
    /// [`Nucleus::Accent`].
    MeasuredAccent {
        narrow: char,
        wide: char,
        threshold: f64,
        base: MathList,
    },
    /// `{}`: an empty ordinary atom.
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Atom {
    pub class: AtomClass,
    pub nucleus: Nucleus,
    pub superscript: Option<MathList>,
    pub subscript: Option<MathList>,
    pub limits: Limits,
    /// Source provenance copied onto every glyph and rule leaf this atom
    /// produces that no inner atom already tagged (see [`crate::source`]).
    pub tag: SourceTag,
    /// Provenance of the delimiters of a [`Nucleus::Delimited`] (`\left`,
    /// `\right`) or of a delimited [`Nucleus::Fraction`], when they came
    /// from their own commands; an unset field inherits [`Atom::tag`].
    pub delimiter_tags: [SourceTag; 2],
}

impl Atom {
    pub fn new(class: AtomClass, nucleus: Nucleus) -> Atom {
        Atom {
            class,
            nucleus,
            superscript: None,
            subscript: None,
            limits: Limits::default(),
            tag: SourceTag::NONE,
            delimiter_tags: [SourceTag::NONE; 2],
        }
    }

    /// This atom with source provenance `tag`.
    pub fn with_tag(mut self, tag: SourceTag) -> Atom {
        self.tag = tag;
        self
    }

    /// This atom with the provenance of its left and right delimiters.
    pub fn with_delimiter_tags(mut self, left: SourceTag, right: SourceTag) -> Atom {
        self.delimiter_tags = [left, right];
        self
    }

    /// A symbol atom whose class comes from the default classification table.
    pub fn symbol(ch: char) -> Atom {
        let (class, limits) = default_class(ch);
        Atom {
            limits,
            ..Atom::new(class, Nucleus::Symbol(ch))
        }
    }

    pub fn ord(ch: char) -> Atom {
        Atom::new(AtomClass::Ord, Nucleus::Symbol(ch))
    }

    pub fn op(ch: char) -> Atom {
        Atom::new(AtomClass::Op, Nucleus::Symbol(ch))
    }

    pub fn bin(ch: char) -> Atom {
        Atom::new(AtomClass::Bin, Nucleus::Symbol(ch))
    }

    pub fn rel(ch: char) -> Atom {
        Atom::new(AtomClass::Rel, Nucleus::Symbol(ch))
    }

    pub fn open(ch: char) -> Atom {
        Atom::new(AtomClass::Open, Nucleus::Symbol(ch))
    }

    pub fn close(ch: char) -> Atom {
        Atom::new(AtomClass::Close, Nucleus::Symbol(ch))
    }

    pub fn punct(ch: char) -> Atom {
        Atom::new(AtomClass::Punct, Nucleus::Symbol(ch))
    }

    pub fn group(list: MathList) -> Atom {
        Atom::new(AtomClass::Ord, Nucleus::List(list))
    }

    pub fn frac(numerator: MathList, denominator: MathList) -> Atom {
        Atom::new(
            AtomClass::Inner,
            Nucleus::Fraction {
                numerator,
                denominator,
                thickness: None,
                left: None,
                right: None,
            },
        )
    }

    /// `\abovewithdelims` / amsmath `\genfrac{left}{right}{thickness}{}`:
    /// a fraction with Rule 15e delimiters. amsmath wraps the result in a
    /// group, so the atom is ordinary rather than inner.
    pub fn genfrac(
        numerator: MathList,
        denominator: MathList,
        thickness: Option<f64>,
        left: Option<char>,
        right: Option<char>,
    ) -> Atom {
        Atom::new(
            AtomClass::Ord,
            Nucleus::Fraction {
                numerator,
                denominator,
                thickness,
                left,
                right,
            },
        )
    }

    /// amsmath `\big(` (`factor` 1), `\Big` 1.5, `\bigg` 2, `\Bigg` 2.5, as
    /// an ordinary atom; `\bigl`/`\bigr`/`\bigm` change `class`.
    ///
    /// This is the amsmath rule. A document that does not load `amsmath`
    /// gets the kernel's fixed lengths instead — see
    /// [`Atom::big_delimiter_kernel`] and [`BigSizing`].
    pub fn big_delimiter(class: AtomClass, delim: Option<char>, factor: f64) -> Atom {
        Atom::new(
            class,
            Nucleus::BigDelimiter {
                delim,
                sizing: BigSizing::Amsmath { factor },
            },
        )
    }

    /// The LaTeX kernel's `\big`…`\Bigg` (`fontmath.ltx`), for a document
    /// that does not load `amsmath`: `pt` is the absolute `\vbox to` length,
    /// 8.5 / 11.5 / 14.5 / 17.5.
    pub fn big_delimiter_kernel(class: AtomClass, delim: Option<char>, pt: f64) -> Atom {
        Atom::new(
            class,
            Nucleus::BigDelimiter {
                delim,
                sizing: BigSizing::Kernel { pt },
            },
        )
    }

    /// `\phantom{body}` (both), `\hphantom` (horizontal), `\vphantom` (vertical).
    pub fn phantom(body: MathList, horizontal: bool, vertical: bool) -> Atom {
        Atom::new(
            AtomClass::Ord,
            Nucleus::Phantom {
                body,
                horizontal,
                vertical,
            },
        )
    }

    /// Explicit rigid math glue of `mu` math units plus `pt` points.
    pub fn glue(mu: f64, pt: f64) -> Atom {
        Atom::glue_flex(mu, pt, MathFlex::ZERO, MathFlex::ZERO)
    }

    /// Explicit math glue with `plus`/`minus` components:
    /// `\mskip 4mu plus 2mu minus 4mu` is
    /// `glue_flex(4.0, 0.0, MathFlex::mu(2.0), MathFlex::mu(4.0))`,
    /// `\hskip 2pt plus 3pt minus 1pt` is
    /// `glue_flex(0.0, 2.0, MathFlex::pt(3.0), MathFlex::pt(1.0))`.
    pub fn glue_flex(mu: f64, pt: f64, stretch: MathFlex, shrink: MathFlex) -> Atom {
        Atom::new(
            AtomClass::Ord,
            Nucleus::Glue {
                mu,
                pt,
                stretch,
                shrink,
            },
        )
    }

    /// `\hfill` in math: `\hskip 0pt plus 1fill`.
    pub fn hfill() -> Atom {
        Atom::glue_flex(
            0.0,
            0.0,
            MathFlex::infinite(1.0, GlueOrder::Fill),
            MathFlex::ZERO,
        )
    }

    /// `\hfil` in math: `\hskip 0pt plus 1fil`.
    pub fn hfil() -> Atom {
        Atom::glue_flex(
            0.0,
            0.0,
            MathFlex::infinite(1.0, GlueOrder::Fil),
            MathFlex::ZERO,
        )
    }

    /// amsmath `\substack` (`align` `c`) / `subarray{l}`.
    pub fn subarray(rows: Vec<MathList>, align: char) -> Atom {
        Atom::new(AtomClass::Ord, Nucleus::SubArray { rows, align })
    }

    /// A relation holding an amsmath extensible arrow ([`Nucleus::ExtArrow`]):
    /// `pieces` are the left piece, the leader fill and the right piece.
    pub fn ext_arrow(pieces: [char; 3], kerns: [f64; 4], above: MathList, below: MathList) -> Atom {
        Atom::new(
            AtomClass::Rel,
            Nucleus::ExtArrow {
                left: pieces[0],
                fill: pieces[1],
                right: pieces[2],
                kerns,
                above,
                below,
            },
        )
    }

    pub fn sqrt(radicand: MathList) -> Atom {
        Atom::new(
            AtomClass::Ord,
            Nucleus::Radical {
                radicand,
                degree: None,
            },
        )
    }

    /// `\sqrt[degree]{radicand}`.
    pub fn root(degree: MathList, radicand: MathList) -> Atom {
        Atom::new(
            AtomClass::Ord,
            Nucleus::Radical {
                radicand,
                degree: Some(degree),
            },
        )
    }

    /// `\lim`, `\sin`, …: an `Op` atom whose nucleus is upright text.
    /// `\lim`-style operators take limits in display style by default; pass
    /// `Limits::NoLimits` (as LaTeX does for `\sin`) with [`Atom::with_limits`].
    pub fn text_op(text: &str) -> Atom {
        Atom::new(AtomClass::Op, Nucleus::Text(text.to_string()))
    }

    /// `\overbrace{body}` / `\underbrace{body}`: a `\mathop` with `\limits`.
    pub fn brace(body: MathList, under: bool) -> Atom {
        Atom::new(AtomClass::Op, Nucleus::Brace { body, under }).with_limits(Limits::Limits)
    }

    /// amsmath `\overrightarrow`-family arrow over (or under) `body`.
    pub fn over_arrow(pieces: [char; 3], body: MathList, under: bool, gap: f64) -> Atom {
        Atom::new(
            AtomClass::Ord,
            Nucleus::OverArrow {
                left: pieces[0],
                fill: pieces[1],
                right: pieces[2],
                body,
                under,
                gap,
            },
        )
    }

    /// amsfonts' measured `\widehat`/`\widetilde`.
    pub fn measured_accent(narrow: char, wide: char, threshold: f64, base: MathList) -> Atom {
        Atom::new(
            AtomClass::Ord,
            Nucleus::MeasuredAccent {
                narrow,
                wide,
                threshold,
                base,
            },
        )
    }

    pub fn overline(body: MathList) -> Atom {
        Atom::new(AtomClass::Ord, Nucleus::Overline(body))
    }

    pub fn underline(body: MathList) -> Atom {
        Atom::new(AtomClass::Ord, Nucleus::Underline(body))
    }

    /// `{\displaystyle body}` etc.
    pub fn styled(style: crate::style::Style, body: MathList) -> Atom {
        Atom::new(AtomClass::Ord, Nucleus::Styled { style, body })
    }

    pub fn accent(accent: char, base: MathList) -> Atom {
        Atom::new(AtomClass::Ord, Nucleus::Accent { accent, base })
    }

    pub fn left_right(left: Option<char>, right: Option<char>, body: MathList) -> Atom {
        Atom::new(AtomClass::Inner, Nucleus::Delimited { left, right, body })
    }

    pub fn with_sup(mut self, sup: MathList) -> Atom {
        self.superscript = Some(sup);
        self
    }

    pub fn with_sub(mut self, sub: MathList) -> Atom {
        self.subscript = Some(sub);
        self
    }

    pub fn with_limits(mut self, limits: Limits) -> Atom {
        self.limits = limits;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MathList {
    pub atoms: Vec<Atom>,
}

impl MathList {
    pub fn new(atoms: Vec<Atom>) -> MathList {
        MathList { atoms }
    }

    /// Builds a list from symbols with the default classification.
    pub fn symbols(text: &str) -> MathList {
        MathList {
            atoms: text.chars().map(Atom::symbol).collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.atoms.is_empty()
    }
}

impl From<Atom> for MathList {
    fn from(atom: Atom) -> MathList {
        MathList { atoms: vec![atom] }
    }
}

/// Default class and limit behaviour of a symbol, after plain.tex's
/// `\mathcode`/`\mathchardef` assignments for the symbols this crate knows.
pub fn default_class(ch: char) -> (AtomClass, Limits) {
    use AtomClass::*;
    let class = match ch {
        '+' | '-' | '\u{2212}' | '\u{22C5}' | '\u{00D7}' | '\u{00F7}' | '\u{00B1}' | '\u{2213}'
        | '\u{2217}' | '\u{2218}' | '\u{2229}' | '\u{222A}' | '\u{2228}' | '\u{2227}'
        | '\u{2295}' | '\u{2297}' | '\u{2216}'
        // LaTeX kernel \DeclareMathSymbol{...}{\mathbin} rows (fontmath.ltx
        // 264, 265, 276-284, 286, 287, 289, 294, 299).
        | '\u{2A3F}' | '\u{2020}' | '\u{2021}' | '\u{2299}' | '\u{2296}' | '\u{2298}'
        | '\u{2293}' | '\u{2294}' | '\u{228E}' | '\u{2240}' | '\u{22C6}' | '\u{25C1}'
        | '\u{25B7}' | '\u{2219}' | '\u{22C4}' | '\u{25EF}' => Bin,
        '=' | '<' | '>' | ':' | '\u{2264}' | '\u{2265}' | '\u{2261}' | '\u{2248}' | '\u{2260}'
        | '\u{223C}' | '\u{2282}' | '\u{2283}' | '\u{2286}' | '\u{2287}' | '\u{2208}'
        | '\u{220B}' | '\u{2190}' | '\u{2192}' | '\u{2194}' | '\u{21D0}' | '\u{21D2}'
        | '\u{21D4}' | '\u{2225}' | '\u{22A5}' | '\u{2223}'
        // amsmath/plain long arrows (\Longrightarrow etc.) are \mathrel.
        | '\u{27F5}' | '\u{27F6}' | '\u{27F7}' | '\u{27F8}' | '\u{27F9}' | '\u{27FA}'
        | '\u{27FC}'
        // LaTeX kernel \DeclareMathSymbol{...}{\mathrel} rows (fontmath.ltx
        // 301, 302, 307-310, 320, 321, 323, 324, 346-352).
        | '\u{224D}' | '\u{2322}' | '\u{2323}' | '\u{21BC}' | '\u{21BD}' | '\u{21C0}'
        | '\u{21C1}' | '\u{2197}' | '\u{2196}' | '\u{2198}' | '\u{2199}' | '\u{227A}'
        | '\u{2AAF}' | '\u{227B}' | '\u{2AB0}' | '\u{2291}' | '\u{2292}' => Rel,
        '(' | '[' | '{' | '\u{27E8}' | '\u{2308}' | '\u{230A}' => Open,
        ')' | ']' | '}' | '\u{27E9}' | '\u{2309}' | '\u{230B}' => Close,
        ',' | ';' => Punct,
        '\u{2211}' | '\u{220F}' | '\u{2210}' | '\u{222B}' | '\u{222E}' | '\u{22C2}'
        | '\u{22C3}' | '\u{2A01}' | '\u{2A02}' | '\u{2A00}' | '\u{22C1}' | '\u{22C0}'
        // \bigsqcup, \biguplus (fontmath.ltx 262, 250).
        | '\u{2A06}' | '\u{2A04}' => Op,
        _ => Ord,
    };
    // plain.tex: \int and \oint are \intop\nolimits.
    let limits = match ch {
        '\u{222B}' | '\u{222E}' => Limits::NoLimits,
        _ => Limits::DisplayLimits,
    };
    (class, limits)
}

#[cfg(test)]
mod long_arrow_tests {
    use super::*;

    #[test]
    fn long_arrows_are_relations() {
        // \Longrightarrow, \Longleftarrow, \Longleftrightarrow, \longrightarrow,
        // \longleftarrow, \longleftrightarrow, \longmapsto.
        for ch in [
            '\u{27F9}', '\u{27F8}', '\u{27FA}', '\u{27F6}', '\u{27F5}', '\u{27F7}', '\u{27FC}',
        ] {
            assert_eq!(default_class(ch).0, AtomClass::Rel, "U+{:04X}", ch as u32);
        }
    }
}
