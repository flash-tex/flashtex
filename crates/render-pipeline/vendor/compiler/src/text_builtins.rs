//! Text-mode built-ins that pdfLaTeX defines in the kernel rather than as
//! plain characters: the encoding-dependent text symbols (`\AA`, `\ss`, `\S`,
//! ...), the `\TeX`/`\LaTeX`/`\LaTeXe` logos and `\rule`.
//!
//! THIN ADOPTION SHIM (KC-105 / KC-106). Symbol resolution calls
//! `flashtex_tex_text_encoding` (the merged, pdflatex-oracled model of
//! `\DeclareText*` and the `*.dfu` Unicode tables) and dimension arithmetic
//! calls `flashtex_tex_boxes::scaled` (TeX's exact `scan_dimen`). Nothing here
//! re-states their tables. When the compiler adopts tex-text-encoding in full
//! (after tex-expansion), `text_symbol` is subsumed by that typesetter and the
//! logo/rule geometry below by tex-boxes' engine; delete this module then.
//!
//! Geometry is expressed in TeX scaled points against metrics the *layout*
//! supplies ([`LogoMetrics`], [`DimenContext`]), because the compiler's own v1
//! layout (Core 14) and the render pipeline (Latin Modern TFMs) measure fonts
//! differently; the construction itself is latex.ltx's, transcribed once.

use flashtex_tex_boxes::scaled::{self, Scaled, Unit};
use flashtex_tex_text_encoding::encoding::{self, Encoding, Resolution};
use flashtex_tex_text_encoding::generated::UNICODE_DECLARATIONS;

/// Text symbol commands and the text command each one means in text mode.
///
/// Encoding commands (`\AA`, `\ss`, ...) mean themselves. The robust
/// wrappers are latex.ltx (TeX Live 2026) lines 10084-10096:
/// `\P`->`\textparagraph`, `\S`->`\textsection`, `\dag`->`\textdagger`,
/// `\ddag`->`\textdaggerdbl`, `\copyright`->`\textcopyright`,
/// `\pounds`->`\textsterling`, `\dots`->`\textellipsis`, `\let\ldots\dots`.
pub const TEXT_SYMBOLS: &[(&str, &str)] = &[
    ("AA", "\\AA"),
    ("aa", "\\aa"),
    ("AE", "\\AE"),
    ("ae", "\\ae"),
    ("OE", "\\OE"),
    ("oe", "\\oe"),
    ("O", "\\O"),
    ("o", "\\o"),
    ("L", "\\L"),
    ("l", "\\l"),
    ("ss", "\\ss"),
    ("SS", "\\SS"),
    ("TH", "\\TH"),
    ("th", "\\th"),
    ("DH", "\\DH"),
    ("dh", "\\dh"),
    ("DJ", "\\DJ"),
    ("dj", "\\dj"),
    ("NG", "\\NG"),
    ("ng", "\\ng"),
    ("IJ", "\\IJ"),
    ("ij", "\\ij"),
    ("i", "\\i"),
    ("j", "\\j"),
    ("S", "\\textsection"),
    ("P", "\\textparagraph"),
    ("dag", "\\textdagger"),
    ("ddag", "\\textdaggerdbl"),
    ("copyright", "\\textcopyright"),
    ("pounds", "\\textsterling"),
    ("dots", "\\textellipsis"),
    ("ldots", "\\textellipsis"),
    ("textsection", "\\textsection"),
    ("textparagraph", "\\textparagraph"),
    ("textdagger", "\\textdagger"),
    ("textdaggerdbl", "\\textdaggerdbl"),
    ("textcopyright", "\\textcopyright"),
    ("textsterling", "\\textsterling"),
    ("textellipsis", "\\textellipsis"),
    ("textbackslash", "\\textbackslash"),
    ("textasciitilde", "\\textasciitilde"),
    ("textasciicircum", "\\textasciicircum"),
    ("textunderscore", "\\textunderscore"),
    ("textbar", "\\textbar"),
    ("textless", "\\textless"),
    ("textgreater", "\\textgreater"),
    ("textbraceleft", "\\textbraceleft"),
    ("textbraceright", "\\textbraceright"),
];

/// Text symbols for printable ASCII characters. The `*.dfu` tables declare
/// only non-ASCII input, so the character is the ASCII one the command names
/// (T1 slots `"5C` `"7E` `"5E` `"5F` `"7C` `"3C` `"3E` `"7B` `"7D`). In OT1
/// they resolve to kernel defaults in other encodings (`\textbackslash`,
/// `\textbar`, `\textbraceleft/right` from OMS; `\textless`/`\textgreater`
/// from OML), which `encoding::resolve` reports as available.
const ASCII_TEXT_SYMBOLS: &[(&str, char)] = &[
    ("\\textbackslash", '\\'),
    ("\\textasciitilde", '~'),
    ("\\textasciicircum", '^'),
    ("\\textunderscore", '_'),
    ("\\textbar", '|'),
    ("\\textless", '<'),
    ("\\textgreater", '>'),
    ("\\textbraceleft", '{'),
    ("\\textbraceright", '}'),
];

/// What a text symbol command typesets under the current encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolOutcome {
    /// The character, as the Unicode code point whose `*.dfu` declaration
    /// expands to this text command (so text extraction round-trips).
    Char(char),
    /// Literal letters from a kernel command default (`\SS` in OT1).
    Text(String),
    /// `LaTeX Error: Command \cmd unavailable in encoding E.`; pdfLaTeX
    /// typesets nothing for it.
    Unavailable(String),
}

/// `None` when `name` is not a text symbol command.
pub fn text_symbol(name: &str, enc: Encoding) -> Option<SymbolOutcome> {
    let (_, command) = TEXT_SYMBOLS.iter().find(|(n, _)| *n == name)?;
    // latex.ltx 564-565 define `\aa`/`\AA` as `\r a`/`\r A` (the ring accent,
    // or its composite slot in T1), exactly as tex-text-encoding's typesetter
    // expands them: availability is the accent's, and the dfu tables declare
    // U+00E5/U+00C5 with that expansion.
    let (resolved, dfu_key) = match *command {
        "\\aa" => ("\\r", "\\r a"),
        "\\AA" => ("\\r", "\\r A"),
        other => (other, other),
    };
    match encoding::resolve(enc, resolved) {
        Resolution::Unavailable => {
            return Some(SymbolOutcome::Unavailable(encoding::unavailable_message(
                enc, command,
            )))
        }
        // A kernel default that is literal letters (`\DeclareTextCommandDefault
        // {\SS}{SS}`, latex.ltx 10078) typesets those letters.
        Resolution::Default(encoding::Default::Command(body))
            if !body.is_empty() && body.bytes().all(|b| b.is_ascii_alphabetic()) =>
        {
            return Some(SymbolOutcome::Text(body.to_string()))
        }
        _ => {}
    }
    if let Some((_, ch)) = ASCII_TEXT_SYMBOLS.iter().find(|(c, _)| c == command) {
        return Some(SymbolOutcome::Char(*ch));
    }
    let ch = UNICODE_DECLARATIONS
        .iter()
        .find(|(_, expansion, _)| *expansion == dfu_key)
        .and_then(|(cp, ..)| char::from_u32(*cp))?;
    Some(SymbolOutcome::Char(ch))
}

/// The encoding `\usepackage[<options>]{fontenc}` leaves current: fontenc
/// loads every listed encoding and selects the last one (`fontenc.sty`,
/// `\fontencoding` of the last option). Unknown encodings are ignored.
pub fn fontenc_encoding(options: &str) -> Option<Encoding> {
    options
        .split(',')
        .map(str::trim)
        .filter(|option| !option.is_empty())
        .last()
        .and_then(Encoding::from_name)
}

// ---------------------------------------------------------------------------
// Text-mode kerns (latex.ltx lines 9432, 15669-15681)
// ---------------------------------------------------------------------------

/// Control symbols that are `\tmspace` kerns in text mode.
pub const KERN_CONTROL_SYMBOLS: &[char] = &[',', '!', ':', '>', ';'];

/// Control words that are text-mode kerns.
pub const KERN_COMMANDS: &[&str] = &[
    "thinspace",
    "negthinspace",
    "medspace",
    "negmedspace",
    "thickspace",
    "negthickspace",
    "enspace",
];

/// The text-mode kern a spacing command inserts, in ems of the current font:
/// `\DeclareRobustCommand\tmspace[3]{\ifmmode\mskip#1#2\else
/// \leavevmode@ifvmode\kern#1#3\fi\relax}` with
/// `\,`=`\thinspace` `+.16667em`, `\!`=`\negthinspace` `-.16667em`,
/// `\:`=`\>`=`\medspace` `+.2222em`, `\negmedspace` `-.2222em`,
/// `\;`=`\thickspace` `+.2777em`, `\negthickspace` `-.2777em`
/// (latex.ltx 15669-15681) and `\enspace` = `\kern.5em` (latex.ltx 9432).
/// `name` is a control word, or the one-character name of a control symbol.
///
/// `amsmath` says whether the document loaded amsmath, which renews
/// `\thinspace` and `\negthinspace` to `.1667em` (`amsmath.sty` 476-477) —
/// one digit shorter than the kernel's `.16667em`, and a real difference
/// because `\tmspace` scales the factor rather than rounding it: measured
/// with TeX Live 2025 pdflatex, `\hbox{a\,b}` is 12.2223pt under the kernel
/// and 12.22261pt under amsmath, **20sp** apart at 10pt (24sp at 12pt), and
/// `\hbox{a\!b}` 8.88887pt against 8.88857pt. `\:`/`\;` and their negatives
/// are byte-identical under both and do not read the flag.
///
/// In *math* mode these are `\mskip\thinmuskip`, which amsmath leaves alone:
/// `$a\,b$` measured identical under both. Only the text-mode kern moves.
pub fn text_kern(name: &str, amsmath: bool) -> Option<TextDimen> {
    let factor = match name {
        "," | "thinspace" if amsmath => ".1667em",
        "!" | "negthinspace" if amsmath => "-.1667em",
        "," | "thinspace" => ".16667em",
        "!" | "negthinspace" => "-.16667em",
        ":" | ">" | "medspace" => ".2222em",
        "negmedspace" => "-.2222em",
        ";" | "thickspace" => ".2777em",
        "negthickspace" => "-.2777em",
        "enspace" => ".5em",
        _ => return None,
    };
    TextDimen::parse(factor)
}

// ---------------------------------------------------------------------------
// Logos (latex.ltx lines 9447-9460, `ltlogos.dtx`)
// ---------------------------------------------------------------------------

/// `\TeX`, `\LaTeX` and `\LaTeXe`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextLogo {
    TeX,
    LaTeX,
    LaTeXe,
}

impl TextLogo {
    pub fn from_command(name: &str) -> Option<TextLogo> {
        Some(match name {
            "TeX" => TextLogo::TeX,
            "LaTeX" => TextLogo::LaTeX,
            "LaTeXe" => TextLogo::LaTeXe,
            _ => return None,
        })
    }

    pub fn command(self) -> &'static str {
        match self {
            TextLogo::TeX => "TeX",
            TextLogo::LaTeX => "LaTeX",
            TextLogo::LaTeXe => "LaTeXe",
        }
    }

    /// The characters of the logo in reading order (text extraction).
    pub fn text(self) -> &'static str {
        match self {
            TextLogo::TeX => "TeX",
            TextLogo::LaTeX => "LaTeX",
            TextLogo::LaTeXe => "LaTeX2\u{03B5}",
        }
    }
}

/// Which font a logo glyph is set in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogoFont {
    /// The current text font.
    Current,
    /// The current family/series/shape at `\sf@size` (`\LaTeX`'s `A`).
    ScriptSize,
    /// The math italic family at the current size (`\LaTeXe`'s `\varepsilon`,
    /// bold math italic under `\boldmath`).
    MathItalic,
}

/// One character box's TFM dimensions, in sp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CharBox {
    pub width: Scaled,
    pub height: Scaled,
    pub depth: Scaled,
    pub italic: Scaled,
}

/// Font metrics a layout supplies to build a logo.
pub trait LogoMetrics {
    /// Dimensions of `ch` in `font`. For [`LogoFont::MathItalic`] `ch` is
    /// `'\u{03B5}'` (`\varepsilon`, cmmi/lmmi slot `"22`).
    fn char_box(&self, font: LogoFont, ch: char) -> CharBox;
    /// `\fontdimen6` (quad, `1em`) of the current text font.
    fn quad(&self) -> Scaled;
    /// `\fontdimen5` (x-height, `1ex`) of the current text font.
    fn x_height(&self) -> Scaled;
    /// `\LaTeXe` only: `sub1` (`\fontdimen16`) and `math_x_height`
    /// (`\fontdimen5`) of `\textfont2`, and `sub_drop` (`\fontdimen19`) of
    /// `\scriptfont2`, at the current size.
    fn math_sub_params(&self) -> MathSubParams;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MathSubParams {
    pub sub1: Scaled,
    pub math_x_height: Scaled,
    pub script_sub_drop: Scaled,
}

/// One placed logo glyph: `x` from the logo's left edge, `raise` above the
/// baseline (negative lowers), both in sp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogoGlyph {
    pub ch: char,
    pub font: LogoFont,
    pub x: Scaled,
    pub raise: Scaled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogoLayout {
    pub glyphs: Vec<LogoGlyph>,
    /// Natural width of the whole construction, in sp.
    pub width: Scaled,
}

/// `\scriptspace` (latex.ltx: `\scriptspace=.5pt`).
pub const SCRIPT_SPACE: Scaled = 32768;

/// `<factor><unit>` for a literal factor such as `.1667` (§455).
fn em_factor(integer: i32, frac: &[u8], v: Scaled, negative: bool) -> Scaled {
    scaled::scale_internal(negative, integer, frac, v).unwrap_or(0)
}

/// Builds `logo` exactly as latex.ltx expands it into TeX primitives.
pub fn layout_logo(logo: TextLogo, m: &dyn LogoMetrics) -> LogoLayout {
    let quad = m.quad();
    let mut glyphs = Vec::new();
    let mut x: Scaled = 0;
    let put = |glyphs: &mut Vec<LogoGlyph>, x: &mut Scaled, ch, font, raise| {
        glyphs.push(LogoGlyph {
            ch,
            font,
            x: *x,
            raise,
        });
        *x += m.char_box(font, ch).width;
    };
    if matches!(logo, TextLogo::LaTeX | TextLogo::LaTeXe) {
        // \DeclareRobustCommand{\LaTeX}{L\kern-.36em%
        //   {\sbox\z@ T\vbox to\ht\z@{\hbox{\check@mathfonts
        //      \fontsize\sf@size\z@\math@fontsfalse\selectfont A}\vss}}%
        //   \kern-.15em\TeX}                          (latex.ltx 9448-9457)
        put(&mut glyphs, &mut x, 'L', LogoFont::Current, 0);
        x += em_factor(0, &[3, 6], quad, true);
        // The \vbox is `\ht T` tall with the \hbox{A} at its top and depth 0
        // (its last item is \vss glue), so A's baseline sits ht(T) - ht(A)
        // above the surrounding baseline; its width is A's.
        let t = m.char_box(LogoFont::Current, 'T');
        let a = m.char_box(LogoFont::ScriptSize, 'A');
        put(
            &mut glyphs,
            &mut x,
            'A',
            LogoFont::ScriptSize,
            t.height - a.height,
        );
        x += em_factor(0, &[1, 5], quad, true);
    }
    // \DeclareRobustCommand\TeX{T\kern-.1667em\lower.5ex\hbox{E}\kern-.125emX\@}
    //                                                       (latex.ltx 9447)
    put(&mut glyphs, &mut x, 'T', LogoFont::Current, 0);
    x += em_factor(0, &[1, 6, 6, 7], quad, true);
    let lower = em_factor(0, &[5], m.x_height(), false);
    put(&mut glyphs, &mut x, 'E', LogoFont::Current, -lower);
    x += em_factor(0, &[1, 2, 5], quad, true);
    put(&mut glyphs, &mut x, 'X', LogoFont::Current, 0);
    if logo == TextLogo::LaTeXe {
        // \DeclareRobustCommand{\LaTeXe}{\mbox{\m@th
        //   \if b\expandafter\@car\f@series\@nil\boldmath\fi
        //   \LaTeX\kern.15em2$_{\textstyle\varepsilon}$}}  (latex.ltx 9458-9460)
        x += em_factor(0, &[1, 5], quad, false);
        put(&mut glyphs, &mut x, '2', LogoFont::Current, 0);
        // `$_{\textstyle\varepsilon}$`: an Ord noad with an empty nucleus and
        // a subscript. make_scripts (§756-757): the empty nucleus hpacks to
        // a zero box, so v = sub_drop(script size); the subscript box x is
        // the text-style ε (plus its italic correction kern, §755) widened by
        // \scriptspace, shifted down max(v, sub1, height(x) - 4/5 x-height).
        let eps = m.char_box(LogoFont::MathItalic, '\u{03B5}');
        let p = m.math_sub_params();
        let shift = p
            .script_sub_drop
            .max(p.sub1)
            .max(eps.height - (p.math_x_height.abs() * 4) / 5);
        glyphs.push(LogoGlyph {
            ch: '\u{03B5}',
            font: LogoFont::MathItalic,
            x,
            raise: -shift,
        });
        x += eps.width + eps.italic + SCRIPT_SPACE;
    }
    LogoLayout { glyphs, width: x }
}

/// `\sf@size` for a font size in sp: fontmath.ltx lines 75-86's
/// `\DeclareMathSizes` table, else `\calculate@math@sizes` (latex.ltx
/// 10742-10753): `\defaultscriptratio` (.7) times the size.
pub fn sf_size(size: Scaled) -> Scaled {
    let pt = |integer: i32, frac: &[u8]| {
        scaled::dimen_from_parts(false, integer, frac, Unit::Pt).unwrap_or(0)
    };
    // (\f@size, \sf@size): \@xpt=10, \@xipt=10.95, \@xiipt=12, \@xivpt=14.4,
    // \@xviipt=17.28, \@xxpt=20.74, \@xxvpt=24.88.
    let table = [
        (pt(5, &[]), pt(5, &[])),
        (pt(6, &[]), pt(5, &[])),
        (pt(7, &[]), pt(5, &[])),
        (pt(8, &[]), pt(6, &[])),
        (pt(9, &[]), pt(6, &[])),
        (pt(10, &[]), pt(7, &[])),
        (pt(10, &[9, 5]), pt(8, &[])),
        (pt(12, &[]), pt(8, &[])),
        (pt(14, &[4]), pt(10, &[])),
        (pt(17, &[2, 8]), pt(12, &[])),
        (pt(20, &[7, 4]), pt(14, &[4])),
        (pt(24, &[8, 8]), pt(20, &[7, 4])),
    ];
    if let Some(&(_, sf)) = table.iter().find(|(declared, _)| *declared == size) {
        return sf;
    }
    // \@tempdimb \defaultscriptratio \dimen@, then \strip@pt: the printed
    // decimal is re-read as a size, which is the same scaled value.
    scaled::scale_internal(false, 0, &[7], size).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// \rule (latex.ltx lines 16359-16367)
// ---------------------------------------------------------------------------

/// A dimension as `\setlength` would scan it: an optional sign, a decimal
/// factor and a unit, which may be font-relative or an internal length.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextDimen {
    pub negative: bool,
    pub integer: i32,
    /// Fraction digits (values 0-9), as `round_decimals` reads them.
    pub frac: Vec<u8>,
    pub unit: DimenUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DimenUnit {
    Physical(PhysicalUnit),
    Em,
    Ex,
    TextWidth,
    LineWidth,
    ColumnWidth,
}

/// `flashtex_tex_boxes::scaled::Unit`, restated only so `TextDimen` can
/// derive `Hash` (the library enum does not).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhysicalUnit {
    Pt,
    In,
    Pc,
    Cm,
    Mm,
    Bp,
    Dd,
    Cc,
    Sp,
}

impl PhysicalUnit {
    fn unit(self) -> Unit {
        match self {
            PhysicalUnit::Pt => Unit::Pt,
            PhysicalUnit::In => Unit::In,
            PhysicalUnit::Pc => Unit::Pc,
            PhysicalUnit::Cm => Unit::Cm,
            PhysicalUnit::Mm => Unit::Mm,
            PhysicalUnit::Bp => Unit::Bp,
            PhysicalUnit::Dd => Unit::Dd,
            PhysicalUnit::Cc => Unit::Cc,
            PhysicalUnit::Sp => Unit::Sp,
        }
    }

    fn parse(s: &str) -> Option<PhysicalUnit> {
        Some(match Unit::parse(s)? {
            Unit::Pt => PhysicalUnit::Pt,
            Unit::In => PhysicalUnit::In,
            Unit::Pc => PhysicalUnit::Pc,
            Unit::Cm => PhysicalUnit::Cm,
            Unit::Mm => PhysicalUnit::Mm,
            Unit::Bp => PhysicalUnit::Bp,
            Unit::Dd => PhysicalUnit::Dd,
            Unit::Cc => PhysicalUnit::Cc,
            Unit::Sp => PhysicalUnit::Sp,
        })
    }
}

/// What a layout knows when it resolves a [`TextDimen`], in sp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DimenContext {
    pub quad: Scaled,
    pub x_height: Scaled,
    pub text_width: Scaled,
    pub line_width: Scaled,
    pub column_width: Scaled,
}

impl TextDimen {
    pub fn zero() -> TextDimen {
        TextDimen {
            negative: false,
            integer: 0,
            frac: Vec::new(),
            unit: DimenUnit::Physical(PhysicalUnit::Pt),
        }
    }

    /// Parses one `\rule` argument. `None` for anything `\setlength` would
    /// not accept as a plain dimension (calc expressions, unknown lengths).
    pub fn parse(text: &str) -> Option<TextDimen> {
        let (negative, rest) = scaled::strip_signs(text.trim());
        let rest = rest.trim();
        let (integer, frac, unit_text) = if rest.starts_with('\\') {
            (1, Vec::new(), rest)
        } else {
            scaled::split_number(rest)?
        };
        let unit = match unit_text.trim() {
            "em" => DimenUnit::Em,
            "ex" => DimenUnit::Ex,
            "\\textwidth" => DimenUnit::TextWidth,
            "\\linewidth" => DimenUnit::LineWidth,
            "\\columnwidth" => DimenUnit::ColumnWidth,
            // `\z@` is latex.ltx's `0pt` (`\rule\z@\footnotesep`).
            "\\z@" => {
                return Some(TextDimen {
                    negative,
                    integer: 0,
                    frac: Vec::new(),
                    unit: DimenUnit::Physical(PhysicalUnit::Pt),
                })
            }
            other => DimenUnit::Physical(PhysicalUnit::parse(other)?),
        };
        Some(TextDimen {
            negative,
            integer,
            frac,
            unit,
        })
    }

    /// The value in sp, exactly as `scan_dimen` computes it (§448-§458).
    pub fn resolve(&self, cx: &DimenContext) -> Scaled {
        let internal = |v: Scaled| {
            scaled::scale_internal(self.negative, self.integer, &self.frac, v).unwrap_or(0)
        };
        match self.unit {
            DimenUnit::Physical(unit) => {
                scaled::dimen_from_parts(self.negative, self.integer, &self.frac, unit.unit())
                    .unwrap_or(0)
            }
            DimenUnit::Em => internal(cx.quad),
            DimenUnit::Ex => internal(cx.x_height),
            DimenUnit::TextWidth => internal(cx.text_width),
            DimenUnit::LineWidth => internal(cx.line_width),
            DimenUnit::ColumnWidth => internal(cx.column_width),
        }
    }

    /// Whether the value needs no font or page context.
    pub fn is_absolute(&self) -> bool {
        matches!(self.unit, DimenUnit::Physical(_))
    }
}

/// `\rule[<raise>]{<width>}{<height>}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextRule {
    pub raise: TextDimen,
    pub width: TextDimen,
    pub height: TextDimen,
}

/// The `\hbox` a `\rule` builds, in sp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleBox {
    /// Box width (`\@tempdimb`); the rule is painted only when positive.
    pub width: Scaled,
    /// Box height and depth: hpack takes the maxima with 0 (§653, §656).
    pub height: Scaled,
    pub depth: Scaled,
    /// The painted rule's top above the baseline (`\@tempdimc`) and bottom
    /// above the baseline (`\@tempdima`); painted only when top > bottom.
    pub rule_top: Scaled,
    pub rule_bottom: Scaled,
}

impl RuleBox {
    pub fn painted(&self) -> bool {
        self.width > 0 && self.rule_top > self.rule_bottom
    }
}

impl TextRule {
    /// latex.ltx 16360-16367:
    /// `\setlength\@tempdima{#1}\setlength\@tempdimb{#2}\setlength\@tempdimc{#3}`
    /// `\advance\@tempdimc\@tempdima`
    /// `\vrule\@width\@tempdimb\@height\@tempdimc\@depth-\@tempdima` in an `\hbox`.
    pub fn resolve(&self, cx: &DimenContext) -> RuleBox {
        let a = self.raise.resolve(cx);
        let b = self.width.resolve(cx);
        let c = self.height.resolve(cx).saturating_add(a);
        RuleBox {
            width: b,
            height: c.max(0),
            depth: (-a).max(0),
            rule_top: c,
            rule_bottom: a,
        }
    }
}

/// Points from sp (`65536 sp = 1 pt`).
pub fn sp_to_pt(sp: Scaled) -> f64 {
    f64::from(sp) / 65536.0
}

/// sp from points, rounded like TeX's `\the` round trip.
pub fn pt_to_sp(pt: f64) -> Scaled {
    (pt * 65536.0).round() as Scaled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_symbol_arms_match_the_builtin_table() {
        let parser = include_str!("parser.rs");
        let begin = parser
            .find("// Kernel text symbols (`text_builtins::TEXT_SYMBOLS`")
            .expect("symbol arm marker moved");
        let end = begin
            + parser[begin..]
                .find("self.text_symbol(name, span, para)")
                .expect("symbol arm body moved");
        let arm = &parser[begin..end];
        let arm_names: Vec<&str> = arm.split('"').skip(1).step_by(2).collect();
        let table: Vec<&str> = TEXT_SYMBOLS.iter().map(|(n, _)| *n).collect();
        assert_eq!(arm_names, table);
    }

    #[test]
    fn every_text_symbol_resolves_in_t1() {
        for (name, command) in TEXT_SYMBOLS {
            match text_symbol(name, Encoding::T1) {
                Some(SymbolOutcome::Char(_)) => {}
                other => panic!("\\{name} ({command}) in T1: {other:?}"),
            }
            match text_symbol(name, Encoding::OT1) {
                Some(_) => {}
                None => panic!("\\{name} ({command}) in OT1 did not resolve"),
            }
        }
    }

    #[test]
    fn symbols_map_to_their_dfu_code_points() {
        let t1 = |n| text_symbol(n, Encoding::T1);
        assert_eq!(t1("AA"), Some(SymbolOutcome::Char('\u{00C5}')));
        assert_eq!(t1("L"), Some(SymbolOutcome::Char('\u{0141}')));
        assert_eq!(t1("S"), Some(SymbolOutcome::Char('\u{00A7}')));
        assert_eq!(t1("dots"), Some(SymbolOutcome::Char('\u{2026}')));
        assert_eq!(t1("i"), Some(SymbolOutcome::Char('\u{0131}')));
        assert_eq!(t1("pounds"), Some(SymbolOutcome::Char('\u{00A3}')));
        assert_eq!(t1("textbackslash"), Some(SymbolOutcome::Char('\\')));
        assert_eq!(t1("textless"), Some(SymbolOutcome::Char('<')));
        for name in [
            "textbackslash",
            "textless",
            "textgreater",
            "textbar",
            "textbraceleft",
        ] {
            assert!(
                matches!(
                    text_symbol(name, Encoding::OT1),
                    Some(SymbolOutcome::Char(_))
                ),
                "\\{name} must be available in OT1 through its kernel default"
            );
        }
    }

    #[test]
    fn ot1_lacks_the_t1_only_letters() {
        for name in ["DH", "dh", "DJ", "dj", "NG", "ng", "TH", "th"] {
            match text_symbol(name, Encoding::OT1) {
                Some(SymbolOutcome::Unavailable(message)) => {
                    assert!(message.contains("unavailable in encoding OT1"), "{message}")
                }
                other => panic!("\\{name} in OT1: {other:?}"),
            }
        }
        assert_eq!(
            text_symbol("AA", Encoding::OT1),
            Some(SymbolOutcome::Char('\u{00C5}'))
        );
        assert_eq!(
            text_symbol("SS", Encoding::OT1),
            Some(SymbolOutcome::Text("SS".into()))
        );
        assert_eq!(
            text_symbol("SS", Encoding::T1),
            Some(SymbolOutcome::Char('\u{1E9E}'))
        );
    }

    #[test]
    fn fontenc_selects_the_last_listed_encoding() {
        assert_eq!(fontenc_encoding("T1"), Some(Encoding::T1));
        assert_eq!(fontenc_encoding("OT1, T1"), Some(Encoding::T1));
        assert_eq!(fontenc_encoding("T1,OT1"), Some(Encoding::OT1));
        assert_eq!(fontenc_encoding("T2A"), None);
    }

    #[test]
    fn dimensions_scan_like_tex() {
        let cx = DimenContext {
            quad: 10 * 65536,
            x_height: 282_168,
            text_width: 345 * 65536,
            line_width: 300 * 65536,
            column_width: 345 * 65536,
        };
        let v = |s: &str| TextDimen::parse(s).unwrap().resolve(&cx);
        assert_eq!(v("0.6pt"), 39_322);
        assert_eq!(v("1em"), 10 * 65536);
        assert_eq!(v("-.5ex"), -141_084);
        assert_eq!(v("\\textwidth"), 345 * 65536);
        assert_eq!(v(".5\\linewidth"), 150 * 65536);
        assert_eq!(v("1in"), 4_736_286);
        assert!(TextDimen::parse("\\foo").is_none());
        assert!(TextDimen::parse("wide").is_none());
    }

    #[test]
    fn text_kerns_scan_their_latex_factors() {
        let cx = DimenContext {
            quad: 10 * 65536,
            ..DimenContext::default()
        };
        let em = |n: &str| text_kern(n, false).unwrap().resolve(&cx);
        // .16667 * 10pt through scale_internal (TeX §455).
        assert_eq!(em(","), 109_230);
        assert_eq!(em("negthinspace"), -109_230);
        assert_eq!(em(">"), em("medspace"));
        assert_eq!(em("enspace"), 5 * 65536);
        assert!(text_kern("quad", false).is_none());
        assert!(text_kern("quad", true).is_none());
    }

    /// amsmath renews `\thinspace`/`\negthinspace` to `.1667em`
    /// (`amsmath.sty` 476-477), which is 20sp wider at 10pt and 24sp at
    /// 12pt than the kernel's `.16667em`. Both sizes measured with TeX Live
    /// 2025 pdflatex: `\hbox{a\,b}` is 12.22230pt against 12.22261pt at 10pt,
    /// and `\hbox{a\!b}` 8.88887pt against 8.88857pt. Everything else in the
    /// table is byte-identical under amsmath, `\:`/`\;` included.
    #[test]
    fn amsmath_renews_only_the_thin_spaces() {
        let at = |quad: i32| DimenContext {
            quad: quad * 65536,
            ..DimenContext::default()
        };
        let kern = |n: &str, amsmath: bool, cx: &DimenContext| {
            text_kern(n, amsmath).unwrap().resolve(cx)
        };
        for (quad, delta) in [(10, 20), (12, 24)] {
            let cx = at(quad);
            assert_eq!(
                kern(",", true, &cx) - kern(",", false, &cx),
                delta,
                "\\, at {quad}pt"
            );
            assert_eq!(
                kern("!", true, &cx) - kern("!", false, &cx),
                -delta,
                "\\! at {quad}pt"
            );
            assert_eq!(kern("thinspace", true, &cx), kern(",", true, &cx));
            assert_eq!(kern("negthinspace", true, &cx), kern("!", true, &cx));
            for unchanged in [":", ">", ";", "medspace", "thickspace", "enspace"] {
                assert_eq!(
                    kern(unchanged, true, &cx),
                    kern(unchanged, false, &cx),
                    "\\{unchanged} at {quad}pt"
                );
            }
        }
    }

    #[test]
    fn rule_box_follows_hpack() {
        let cx = DimenContext::default();
        let rule = TextRule {
            raise: TextDimen::parse("-2pt").unwrap(),
            width: TextDimen::parse("3pt").unwrap(),
            height: TextDimen::parse("1pt").unwrap(),
        };
        let b = rule.resolve(&cx);
        assert_eq!((b.width, b.height, b.depth), (3 * 65536, 0, 2 * 65536));
        assert_eq!((b.rule_top, b.rule_bottom), (-65536, -2 * 65536));
        assert!(b.painted());
    }

    #[test]
    fn sf_size_uses_the_declared_table_then_the_ratio() {
        let pt = |p: i32| p * 65536;
        assert_eq!(sf_size(pt(10)), pt(7));
        assert_eq!(sf_size(pt(12)), pt(8));
        let xipt = scaled::dimen_from_parts(false, 10, &[9, 5], Unit::Pt).unwrap();
        assert_eq!(sf_size(xipt), pt(8));
        assert_eq!(
            sf_size(pt(11)),
            scaled::scale_internal(false, 0, &[7], pt(11)).unwrap()
        );
    }
}
