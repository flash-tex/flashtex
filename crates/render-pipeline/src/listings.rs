//! `listings.sty`: the source scan, the option list, and the character
//! machine that decides which output box every code character lands in.
//!
//! The compiler lowers `lstlisting` to a plain [`Block::Verbatim`] and
//! reports its option list as unimplemented (`parser::verbatim_environment`),
//! so everything listings actually *does* with those options is re-derived
//! here from the source bytes -- the arrangement PR #188 anticipated: "the
//! renderer scans `\lstset`/`\lstdefinestyle`/the local list from the
//! source". [`crate::typeset`] turns the boxes this module plans into
//! glyphs, because only it knows the face NFSS selected and its TFM.
//!
//! Everything below is transcribed from TeX Live 2025's `listings.sty`
//! (v1.10b) and `lstmisc.sty`, and measured against pdfLaTeX. The pieces
//! that are easy to get wrong:
//!
//! * **A listing is not a `\trivlist`.** `\lst@Init` is `\par\penalty-50
//!   \vspace\lst@aboveskip ... \normalbaselines` and `\lst@DeInit` is
//!   `\par\removelastskip ... \penalty-50\vspace\lst@belowskip`; there is
//!   no `\topsep`, no `\partopsep` and no `\@totalleftmargin`. `\vspace`
//!   is the unstarred LaTeX one, a plain `\vskip` that does not
//!   `\addvspace`-merge, so two adjacent listings *add* both skips.
//! * **One box per token, not per character.** `\lst@OutputToken` sets
//!   `\hbox to <N>\lst@width{\lst@lefthss <c1>\hss <c2>\hss ... <cN>
//!   \lst@righthss}` for a maximal run of letters or of "other"
//!   characters. `\lst@width` is fixed once at `InitVars` from
//!   `basicstyle` (`fontadjust` is false by default), so a
//!   `keywordstyle`/`commentstyle` switch never changes the cell grid --
//!   only the glyph positions *inside* a cell, because `[c]` centring
//!   divides the slack left by the natural character widths.
//! * **`\@noligs` characters add a zero-width item to the token.**
//!   `\lst@SelectCharTable` runs `\let\do\lst@do@noligs \verbatim@nolig@list`,
//!   which redefines `` ` ``, `'`, `,`, `-`, `<` and `>` as
//!   `\lst@NoLig <original meaning>`, and `\lst@NoLig` is
//!   `\advance\lst@length\m@ne \lst@Append\lst@nolig`: it appends an empty
//!   box to whatever token is *currently open* before the character's own
//!   processing decides to flush it. `\lst@length` is unchanged, so the
//!   box is still N cells wide -- but it now holds N+1 items, so
//!   `\lst@FillFixed` puts N+2 `\hss` in it under `[c]` instead of N+1 and
//!   every glyph in that token moves. This is the "comma anomaly" #145
//!   measured and could not explain; it is `,` and `-` (the two
//!   `\@noligs` characters that occur in the fixtures), not `,` alone.
//!   Measured on this host: `abcdefghijkl` followed by each of
//!   `! " ( ) + . / : ; = ? [ ] * 0 Z` keeps the N+1 divisor and only `,`
//!   and `-` change it.
//! * **listings suppresses ligatures and kerns unconditionally**, roman
//!   `basicstyle` included -- every character is its own box here, which
//!   is why. `\verb`/`\@verbatim` are different: there `\@noligs` really
//!   is the whole mechanism and [`crate::shape::Shaper::shape_with`]'s
//!   split is right. The two constructs are deliberately not unified.

use std::ops::Range;

/// `\verbatim@nolig@list` (latex.ltx): the characters `\@noligs` makes
/// active, and which listings turns into `\lst@NoLig`.
pub const NOLIG: [char; 6] = ['`', '<', '>', ',', '\'', '-'];

/// `\lst@ProcessOther` characters that are *not* `\lst@ProcessLetter` or
/// `\lst@ProcessDigit` (listings.sty `\lst@CCPut`/`\lst@CCPutMacro`).
/// Everything printable that is not here and not a digit is a letter;
/// note `_`, `$` and `@` are letters, which is what makes `argument_two`
/// one token.
fn is_letter(c: char) -> bool {
    c.is_ascii_alphabetic() || matches!(c, '@' | '$' | '_')
}

fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

/// Which `\lst@Process...` a character takes (digits join whatever token
/// is open, so they have no class of their own here).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Class {
    Letter,
    Other,
    Digit,
}

fn class(c: char) -> Class {
    if is_letter(c) {
        Class::Letter
    } else if is_digit(c) {
        Class::Digit
    } else {
        Class::Other
    }
}

/// The style a piece is set in. `Basic` is `basicstyle` (listings calls it
/// `\lst@identifierstyle`, empty by default, on top of `basicstyle`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TokStyle {
    #[default]
    Basic,
    /// `keywordstyle`, `\bfseries` by default.
    Keyword,
    /// `commentstyle`; declared empty but the `EmptyStyle` hook sets it to
    /// `\itshape` (listings.sty `\lst@AddToHook{EmptyStyle}`).
    Comment,
    /// `stringstyle`, empty by default.
    Str,
}

/// One item of a token's `\hbox`: a character, or the zero-width
/// `\lst@nolig` (`\leavevmode\kern\z@`).
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    /// `None` for `\lst@nolig`.
    pub ch: Option<char>,
    pub style: TokStyle,
    /// Source bytes of the character (empty range for `\lst@nolig`, which
    /// points at the character that produced it).
    pub at: Range<usize>,
    /// The character is set as `\lst@visiblespace` (`showstringspaces`).
    pub visible_space: bool,
}

/// One thing `\lst@ProcessX` does to the horizontal list, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// `\lst@ProcessSpace`'s lost-space branch: `\lst@lostspace` grows by
    /// one `\lst@width` and nothing is set. Leading whitespace and every
    /// space after the first of a run take it.
    LostSpace,
    /// `\lst@OutputToken`: `\lst@length` = `cells`, the box's items.
    Box { cells: usize, pieces: Vec<Piece> },
    /// `\lst@GotoTabStop` away from the start of a line: a box exactly one
    /// `\lst@outputspace` wide declared `cells` cells long, so the
    /// remainder becomes lost space around it.
    TabBox { cells: usize, at: Range<usize> },
    /// `\lst@GotoTabStop` at the start of a line: pure lost space.
    TabLost { cells: usize },
}

/// One output line of a listing (one source line unless `breaklines`
/// splits it, which [`crate::typeset`] does once it has the metrics).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CodeLine {
    pub ops: Vec<Op>,
    /// `\thelstnumber` when `numbers` puts one on this line.
    pub number: Option<String>,
    /// The source bytes of the line (for diagnostics and `\label`-free
    /// glyph provenance).
    pub at: Range<usize>,
}

// ---------------------------------------------------------------- options

/// A listings dimension that may be in ems of the `basicstyle` font.
/// `xleftmargin=2em` is expanded *inside* the listing, after `basicstyle`
/// has been applied, so 2em of cmtt8 (whose quad is 1.062515, not
/// 1.049991) is 17.00024pt and not 16.8pt.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Dimen {
    pub pt: f64,
    pub em: f64,
    pub ex: f64,
}

impl Dimen {
    pub const fn pt(pt: f64) -> Dimen {
        Dimen { pt, em: 0.0, ex: 0.0 }
    }
    pub fn resolve(self, em: f64, ex: f64) -> f64 {
        self.pt + self.em * em + self.ex * ex
    }
}

/// `columns=<mode>`: `\lst@column@fixed`, `...@flexible`,
/// `...@fullflexible`, `...@spaceflexible`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Columns {
    #[default]
    Fixed,
    Flexible,
    FullFlexible,
    SpaceFlexible,
}

impl Columns {
    pub fn flexible(self) -> bool {
        !matches!(self, Columns::Fixed)
    }
}

/// The optional argument of `columns`, `\lst@outputpos`: which `\hss`
/// the box gets on its outside.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Pos {
    /// `[l]`: no `\lst@lefthss`.
    Left,
    /// `[c]`, the default: `\hss` on both sides.
    #[default]
    Center,
    /// `[r]`: no `\lst@righthss`.
    Right,
}

/// `frame=`'s four sides. `frame=single` is `tblr`, `frame=lines` is `tb`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Frame {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

impl Frame {
    pub fn any(self) -> bool {
        self.top || self.bottom || self.left || self.right
    }
    /// `\lst@frameInit`/`\lst@frameExit` only change the vertical list
    /// when there is a horizontal rule; an `l`/`r`-only frame adds nothing.
    pub fn horizontal(self) -> bool {
        self.top || self.bottom
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NumberSide {
    #[default]
    None,
    Left,
    Right,
}

/// A font declaration list (`basicstyle=\ttfamily\small`,
/// `numberstyle=\tiny`, `keywordstyle=\bfseries`, ...) as far as it
/// changes geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct StyleSpec {
    pub family: Option<crate::nfss::FamilyKind>,
    /// `\tiny`..`\Huge` as the compiler's level, `None` for `\normalsize`
    /// or no size declaration.
    pub size: Option<flashtex_compiler::parser::FontSizeLevel>,
    pub bold: bool,
    pub italic: bool,
    pub slanted: bool,
}

impl StyleSpec {
    /// Reads the declarations of one style value. Only the commands that
    /// reach the metrics are modelled; anything else (a colour, a user
    /// macro) is ignored and reported by the caller.
    pub fn parse(value: &str) -> StyleSpec {
        use flashtex_compiler::parser::FontSizeLevel as L;
        use crate::nfss::FamilyKind;
        let mut s = StyleSpec::default();
        for (name, _) in commands(value) {
            match name {
                "ttfamily" | "tt" => s.family = Some(FamilyKind::Tt),
                "rmfamily" | "rm" => s.family = Some(FamilyKind::Rm),
                "sffamily" | "sf" => s.family = Some(FamilyKind::Sf),
                "bfseries" | "bf" => s.bold = true,
                "mdseries" => s.bold = false,
                "itshape" | "it" => s.italic = true,
                "slshape" | "sl" => s.slanted = true,
                "upshape" => {
                    s.italic = false;
                    s.slanted = false;
                }
                "normalfont" => s = StyleSpec { size: s.size, ..StyleSpec::default() },
                "tiny" => s.size = Some(L::Tiny),
                "scriptsize" => s.size = Some(L::ScriptSize),
                "footnotesize" => s.size = Some(L::FootnoteSize),
                "small" => s.size = Some(L::Small),
                "normalsize" => s.size = None,
                "large" => s.size = Some(L::Large1),
                "Large" => s.size = Some(L::Large2),
                "LARGE" => s.size = Some(L::Large3),
                "huge" => s.size = Some(L::Huge1),
                "Huge" => s.size = Some(L::Huge2),
                _ => {}
            }
        }
        s
    }

    /// The declarations this value carries that the pipeline does not
    /// model, for the `unsupported_block` note.
    pub fn unmodelled(value: &str) -> Vec<String> {
        const KNOWN: [&str; 22] = [
            "ttfamily", "tt", "rmfamily", "rm", "sffamily", "sf", "bfseries", "bf", "mdseries", "itshape", "it", "slshape", "sl", "upshape",
            "normalfont", "tiny", "scriptsize", "footnotesize", "small", "normalsize", "large", "Large",
        ];
        const KNOWN2: [&str; 4] = ["LARGE", "huge", "Huge", "relax"];
        commands(value)
            .into_iter()
            .filter(|(n, _)| !KNOWN.contains(n) && !KNOWN2.contains(n))
            .map(|(n, _)| format!("\\{n}"))
            .collect()
    }
}

/// The control words of a style value, with the byte offset of each.
fn commands(value: &str) -> Vec<(&str, usize)> {
    let bytes = value.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                j += 1;
            }
            if j > start {
                out.push((&value[start..j], i));
            }
            i = j.max(i + 2);
        } else {
            i += 1;
        }
    }
    out
}

/// Every listings option this pipeline reads, at its package default.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub basic: StyleSpec,
    pub keyword: StyleSpec,
    pub comment: StyleSpec,
    pub string: StyleSpec,
    pub number_style: StyleSpec,
    pub columns: Columns,
    pub pos: Pos,
    /// `basewidth={0.6em,0.45em}`.
    pub basewidth_fixed: Dimen,
    pub basewidth_flexible: Dimen,
    pub fontadjust: bool,
    pub numbers: NumberSide,
    pub numbersep: Dimen,
    pub stepnumber: i64,
    pub firstnumber: i64,
    pub number_blank_lines: bool,
    pub frame: Frame,
    pub framerule: Dimen,
    pub framesep: Dimen,
    pub xleftmargin: Dimen,
    pub xrightmargin: Dimen,
    pub tabsize: usize,
    pub gobble: usize,
    pub breaklines: bool,
    pub breakindent: Dimen,
    pub breakautoindent: bool,
    pub breakatwhitespace: bool,
    pub showstringspaces: bool,
    pub showspaces: bool,
    pub showtabs: bool,
    pub keepspaces: bool,
    pub aboveskip: Dimen,
    pub belowskip: Dimen,
    /// `(stretch, shrink)` of the two skips. Both default to
    /// `\medskipamount` = `6pt plus 2pt minus 2pt`; an explicit
    /// `aboveskip=<dimen>` is rigid.
    pub aboveskip_rubber: (f64, f64),
    pub belowskip_rubber: (f64, f64),
    pub language: Option<Language>,
    /// Option keys seen but not modelled, for the pipeline's note.
    pub unsupported: Vec<String>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            basic: StyleSpec::default(),
            // `keywordstyle` default `\bfseries`; `commentstyle` is
            // declared empty but the `EmptyStyle` hook makes it
            // `\itshape`; `stringstyle` and `numberstyle` stay empty.
            keyword: StyleSpec { bold: true, ..StyleSpec::default() },
            comment: StyleSpec { italic: true, ..StyleSpec::default() },
            string: StyleSpec::default(),
            number_style: StyleSpec::default(),
            columns: Columns::Fixed,
            pos: Pos::Center,
            basewidth_fixed: Dimen { pt: 0.0, em: 0.6, ex: 0.0 },
            basewidth_flexible: Dimen { pt: 0.0, em: 0.45, ex: 0.0 },
            fontadjust: false,
            numbers: NumberSide::None,
            numbersep: Dimen::pt(10.0),
            stepnumber: 1,
            firstnumber: 1,
            number_blank_lines: true,
            frame: Frame::default(),
            framerule: Dimen::pt(0.4),
            framesep: Dimen::pt(3.0),
            xleftmargin: Dimen::default(),
            xrightmargin: Dimen::default(),
            tabsize: 8,
            gobble: 0,
            breaklines: false,
            breakindent: Dimen::pt(20.0),
            breakautoindent: true,
            breakatwhitespace: false,
            showstringspaces: true,
            showspaces: false,
            showtabs: false,
            keepspaces: false,
            aboveskip: Dimen::pt(6.0),
            belowskip: Dimen::pt(6.0),
            aboveskip_rubber: (2.0, 2.0),
            belowskip_rubber: (2.0, 2.0),
            language: None,
            unsupported: Vec::new(),
        }
    }
}

/// Keys whose effect on geometry is nil, so ignoring them silently is
/// correct rather than a gap.
const HARMLESS: [&str; 16] = [
    "identifierstyle", "directivestyle", "emphstyle", "captionpos", "caption", "label", "title", "float", "belowcaptionskip", "abovecaptionskip",
    "rulecolor", "backgroundcolor", "fillcolor", "rulesepcolor", "upquote", "extendedchars",
];

impl Options {
    /// Applies one `key=value` list (`\lstset`'s argument or the
    /// environment's optional one). Unknown keys are recorded.
    pub fn apply(&mut self, list: &str) {
        for (key, value) in key_values(list) {
            self.apply_one(&key, value.as_deref());
        }
    }

    fn apply_one(&mut self, key: &str, value: Option<&str>) {
        let v = value.unwrap_or("");
        let flag = |v: &str| !matches!(v.trim(), "false" | "f");
        match key {
            "basicstyle" => self.basic = StyleSpec::parse(v),
            "keywordstyle" => self.keyword = StyleSpec::parse(v),
            "commentstyle" => self.comment = StyleSpec::parse(v),
            "stringstyle" => self.string = StyleSpec::parse(v),
            "numberstyle" => self.number_style = StyleSpec::parse(v),
            "columns" => {
                // `columns=[l]fixed`: `\lstKV@OptArg` splits the leading
                // bracket group off as `\lst@outputpos`'s argument. In an
                // environment's own option list the value has to be braced
                // (`columns={[l]fixed}`), or LaTeX's optional-argument
                // scanner ends the list at the inner `]`.
                let rest = match strip_braces(v).trim().strip_prefix('[') {
                    Some(r) => match r.split_once(']') {
                        Some((p, rest)) => {
                            self.pos = match p.trim() {
                                "l" => Pos::Left,
                                "r" => Pos::Right,
                                _ => Pos::Center,
                            };
                            rest
                        }
                        None => r,
                    },
                    None => v,
                };
                self.columns = match rest.trim() {
                    "flexible" => Columns::Flexible,
                    "fullflexible" => Columns::FullFlexible,
                    "spaceflexible" => Columns::SpaceFlexible,
                    _ => Columns::Fixed,
                };
            }
            "flexiblecolumns" => {
                self.columns = if flag(v) { Columns::Flexible } else { Columns::Fixed };
            }
            "outputpos" => {
                self.pos = match v.trim() {
                    "l" => Pos::Left,
                    "r" => Pos::Right,
                    _ => Pos::Center,
                }
            }
            "basewidth" => {
                let inner = strip_braces(v);
                let mut parts = inner.splitn(2, ',');
                if let Some(a) = parts.next().and_then(dimen) {
                    self.basewidth_fixed = a;
                    self.basewidth_flexible = a;
                }
                if let Some(b) = parts.next().filter(|b| !b.trim().is_empty()).and_then(dimen) {
                    self.basewidth_flexible = b;
                }
            }
            "fontadjust" => self.fontadjust = flag(v),
            "numbers" => {
                self.numbers = match v.trim() {
                    "left" => NumberSide::Left,
                    "right" => NumberSide::Right,
                    _ => NumberSide::None,
                }
            }
            "numbersep" => self.numbersep = dimen(v).unwrap_or(self.numbersep),
            "stepnumber" => self.stepnumber = v.trim().parse().unwrap_or(self.stepnumber),
            "firstnumber" => self.firstnumber = v.trim().parse().unwrap_or(self.firstnumber),
            "numberblanklines" => self.number_blank_lines = flag(v),
            "frame" => self.frame = frame(v.trim()),
            "frameround" | "frameshape" => self.unsupported.push(key.to_string()),
            "framerule" => self.framerule = dimen(v).unwrap_or(self.framerule),
            "framesep" | "frametextsep" => self.framesep = dimen(v).unwrap_or(self.framesep),
            "xleftmargin" => self.xleftmargin = dimen(v).unwrap_or_default(),
            "xrightmargin" => self.xrightmargin = dimen(v).unwrap_or_default(),
            "tabsize" => self.tabsize = v.trim().parse().unwrap_or(self.tabsize).max(1),
            "gobble" => self.gobble = v.trim().parse().unwrap_or(self.gobble),
            "breaklines" => self.breaklines = flag(v),
            "breakindent" => self.breakindent = dimen(v).unwrap_or(self.breakindent),
            "breakautoindent" => self.breakautoindent = flag(v),
            "breakatwhitespace" => self.breakatwhitespace = flag(v),
            "showstringspaces" => self.showstringspaces = flag(v),
            "showspaces" => {
                self.showspaces = flag(v);
                if self.showspaces {
                    self.keepspaces = true;
                }
            }
            "showtabs" => self.showtabs = flag(v),
            "keepspaces" => self.keepspaces = flag(v),
            "aboveskip" => {
                self.aboveskip = dimen(v).unwrap_or(self.aboveskip);
                self.aboveskip_rubber = (0.0, 0.0);
            }
            "belowskip" => {
                self.belowskip = dimen(v).unwrap_or(self.belowskip);
                self.belowskip_rubber = (0.0, 0.0);
            }
            "language" => {
                let name = strip_braces(v);
                // `language=[dialect]Name`.
                let name = match name.trim().strip_prefix('[') {
                    Some(r) => r.split_once(']').map_or(r, |(_, n)| n),
                    None => name,
                };
                self.language = Language::named(name.trim());
                if self.language.is_none() && !name.trim().is_empty() {
                    self.unsupported.push(format!("language={}", name.trim()));
                }
            }
            "" => {}
            other if HARMLESS.contains(&other) => {}
            other => self.unsupported.push(other.to_string()),
        }
    }
}

fn strip_braces(v: &str) -> &str {
    let t = v.trim();
    t.strip_prefix('{').and_then(|r| r.strip_suffix('}')).unwrap_or(t)
}

/// `frame=` values: the letters `tblrTBLR` (single rules lower case,
/// double upper case, which is drawn as a single rule here), or the
/// named shorthands.
fn frame(v: &str) -> Frame {
    match v {
        "none" | "" => Frame::default(),
        "leftline" => Frame { left: true, ..Frame::default() },
        "topline" => Frame { top: true, ..Frame::default() },
        "bottomline" => Frame { bottom: true, ..Frame::default() },
        "rightline" => Frame { right: true, ..Frame::default() },
        "lines" => Frame { top: true, bottom: true, ..Frame::default() },
        "single" | "shadowbox" => Frame { top: true, bottom: true, left: true, right: true },
        letters => {
            let mut f = Frame::default();
            for c in letters.chars() {
                match c.to_ascii_lowercase() {
                    't' => f.top = true,
                    'b' => f.bottom = true,
                    'l' => f.left = true,
                    'r' => f.right = true,
                    _ => {}
                }
            }
            f
        }
    }
}

/// A TeX dimension with the units listings' options actually carry.
pub fn dimen(v: &str) -> Option<Dimen> {
    let t = strip_braces(v).trim();
    if t.is_empty() {
        return None;
    }
    let (num, unit) = t.split_at(t.len() - t.chars().rev().take_while(|c| c.is_ascii_alphabetic()).count());
    let n: f64 = num.trim().parse().ok().or_else(|| if num.trim() == "-" { Some(-1.0) } else { None })?;
    Some(match unit {
        "pt" => Dimen::pt(n),
        "bp" => Dimen::pt(n * 72.27 / 72.0),
        "mm" => Dimen::pt(n * 72.27 / 25.4),
        "cm" => Dimen::pt(n * 72.27 / 2.54),
        "in" => Dimen::pt(n * 72.27),
        "em" => Dimen { pt: 0.0, em: n, ex: 0.0 },
        "ex" => Dimen { pt: 0.0, em: 0.0, ex: n },
        _ => return None,
    })
}

/// Splits a `key=value,key=value` list at top-level commas, keeping brace
/// and bracket groups whole.
pub fn key_values(list: &str) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    let (mut depth, mut bracket) = (0i32, 0i32);
    let mut start = 0usize;
    let bytes = list.as_bytes();
    let mut i = 0;
    let mut pieces: Vec<&str> = Vec::new();
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b'[' if depth == 0 => bracket += 1,
            b']' if depth == 0 => bracket -= 1,
            b',' if depth == 0 && bracket == 0 => {
                pieces.push(&list[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    pieces.push(&list[start..]);
    for piece in pieces {
        let piece = piece.trim();
        if piece.is_empty() {
            continue;
        }
        // The first top-level `=` splits key from value.
        let (mut depth, mut bracket) = (0i32, 0i32);
        let b = piece.as_bytes();
        let mut at = None;
        let mut i = 0;
        while i < b.len() {
            match b[i] {
                b'\\' => i += 1,
                b'{' => depth += 1,
                b'}' => depth -= 1,
                b'[' if depth == 0 => bracket += 1,
                b']' if depth == 0 => bracket -= 1,
                b'=' if depth == 0 && bracket == 0 => {
                    at = Some(i);
                    break;
                }
                _ => {}
            }
            i += 1;
        }
        match at {
            Some(i) => out.push((piece[..i].trim().to_string(), Some(piece[i + 1..].trim().to_string()))),
            None => out.push((piece.trim().to_string(), None)),
        }
    }
    out
}

// --------------------------------------------------------------- language

/// The part of a `\lst@definelanguage` that decides styles: the keyword
/// list, the comment delimiters and the string delimiters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Language {
    pub name: &'static str,
    pub keywords: &'static [&'static str],
    /// `morecomment=[l]...`.
    pub line_comment: &'static [&'static str],
    /// `morecomment=[s]{..}{..}`.
    pub block_comment: &'static [(&'static str, &'static str)],
    /// `morestring=[b]<c>`: a delimiter with backslash escapes.
    pub strings: &'static [char],
    /// `morestring=[s]{...}{...}` (Python's triple quotes).
    pub long_strings: &'static [(&'static str, &'static str)],
}

/// `lstlang1.sty`'s `[ANSI]{C}` (the default C dialect).
const C_KEYWORDS: &[&str] = &[
    "auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else", "enum", "extern", "float", "for", "goto", "if", "int",
    "long", "register", "return", "short", "signed", "sizeof", "static", "struct", "switch", "typedef", "union", "unsigned", "void", "volatile",
    "while",
];

/// `lstlang1.sty`'s `{Java}`.
const JAVA_KEYWORDS: &[&str] = &[
    "abstract", "boolean", "break", "byte", "case", "catch", "char", "class", "const", "continue", "default", "do", "double", "else", "extends",
    "false", "final", "finally", "float", "for", "goto", "if", "implements", "import", "instanceof", "int", "interface", "label", "long", "native",
    "new", "null", "package", "private", "protected", "public", "return", "short", "static", "super", "switch", "synchronized", "this", "throw",
    "throws", "transient", "true", "try", "void", "volatile", "while",
];

/// `lstlang1.sty`'s `[3]{Python}` keyword list (`[2]` plus the additions,
/// less the deletions); the built-in list `morekeywords=[2]` is a second
/// class whose style is empty by default, so it changes nothing.
const PYTHON_KEYWORDS: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "case", "class", "continue", "def", "del", "elif", "else", "except", "False", "finally",
    "for", "from", "global", "if", "import", "in", "is", "lambda", "match", "None", "nonlocal", "not", "or", "pass", "raise", "return", "True",
    "try", "while", "with", "yield",
];

impl Language {
    pub fn named(name: &str) -> Option<Language> {
        let c = Language {
            name: "C",
            keywords: C_KEYWORDS,
            line_comment: &["//"],
            block_comment: &[("/*", "*/")],
            strings: &['"', '\''],
            long_strings: &[],
        };
        Some(match name {
            "C" => c,
            "C++" => Language { name: "C++", ..c },
            "Java" => Language { name: "Java", keywords: JAVA_KEYWORDS, ..c },
            "Python" => Language {
                name: "Python",
                keywords: PYTHON_KEYWORDS,
                line_comment: &["#"],
                block_comment: &[],
                strings: &['"', '\''],
                long_strings: &[("'''", "'''"), ("\"\"\"", "\"\"\"")],
            },
            _ => return None,
        })
    }
}

// ------------------------------------------------------- the source scan

/// One `lstlisting` environment as the source has it.
#[derive(Clone, Debug, PartialEq)]
pub struct Environment {
    /// `\begin{lstlisting}` through the end of `\end{lstlisting}`.
    pub span: Range<usize>,
    /// The optional argument's text (without the brackets).
    pub options: String,
    /// The body, excluding the newline after `\begin` and before `\end`.
    pub body: Range<usize>,
}

const BEGIN: &str = "\\begin{lstlisting}";
const END: &str = "\\end{lstlisting}";

/// Every `lstlisting` environment of a document, in source order. The
/// scan is the same finicky literal one LaTeX's verbatim scanner and the
/// compiler's `verbatim_environment` perform, so the two agree on where a
/// body starts and ends.
pub fn find_environments(text: &str) -> Vec<Environment> {
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(rel) = text[from..].find(BEGIN) {
        let begin = from + rel;
        let mut i = begin + BEGIN.len();
        let mut options = String::new();
        if text.as_bytes().get(i) == Some(&b'[') {
            if let Some(close) = balanced_bracket(text, i) {
                options = text[i + 1..close].to_string();
                i = close + 1;
            }
        }
        if text.as_bytes().get(i) == Some(&b'\n') {
            i += 1;
        }
        let body_start = i;
        let (body_end, span_end) = match text[body_start..].find(END) {
            Some(off) => (body_start + off, body_start + off + END.len()),
            None => (text.len(), text.len()),
        };
        let mut trimmed = body_end;
        if trimmed > body_start && text.as_bytes()[trimmed - 1] == b'\n' {
            trimmed -= 1;
        }
        out.push(Environment {
            span: begin..span_end,
            options,
            body: body_start..trimmed,
        });
        from = span_end;
    }
    out
}

/// The spans of every `\lstset{...}` and `\lstdefinestyle{...}` in a
/// document, with the argument text of the `\lstset`s.
///
/// The pipeline needs both: the options, and the bytes to keep out of the
/// body text. The compiler has no `\lstset`, so it typesets the argument
/// as prose (11 stray glyphs in `28-lstinline`); the adapter drops every
/// inline whose span falls inside one of these.
pub fn find_lstset(text: &str) -> Vec<(Range<usize>, String)> {
    let mut out = Vec::new();
    for command in ["\\lstset", "\\lstdefinestyle", "\\lstdefinelanguage"] {
        let mut from = 0usize;
        while let Some(rel) = text[from..].find(command) {
            let at = from + rel;
            // `\lstsetfoo` is a different control word.
            let after = at + command.len();
            if text[after..].chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
                from = after;
                continue;
            }
            let mut i = after;
            while text.as_bytes().get(i).is_some_and(|b| b.is_ascii_whitespace()) {
                i += 1;
            }
            match (text.as_bytes().get(i), balanced_brace(text, i)) {
                (Some(b'{'), Some(close)) => {
                    let body = text[i + 1..close].to_string();
                    out.push((at..close + 1, if command == "\\lstset" { body } else { String::new() }));
                    from = close + 1;
                }
                _ => from = after,
            }
        }
    }
    out.sort_by_key(|(r, _)| r.start);
    out
}

fn balanced_brace(text: &str, at: usize) -> Option<usize> {
    balanced(text, at, b'{', b'}')
}

fn balanced_bracket(text: &str, at: usize) -> Option<usize> {
    balanced(text, at, b'[', b']')
}

fn balanced(text: &str, at: usize, open: u8, close: u8) -> Option<usize> {
    let b = text.as_bytes();
    if b.get(at) != Some(&open) {
        return None;
    }
    let mut depth = 0i32;
    let mut i = at;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1,
            c if c == open => depth += 1,
            c if c == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The options in force at `at`: the package defaults, then every
/// `\lstset` before that offset, then the environment's own list.
pub fn options_at(text: &str, at: usize, local: &str) -> Options {
    let mut opts = Options::default();
    for (span, body) in find_lstset(text) {
        if span.start < at {
            opts.apply(&body);
        }
    }
    opts.apply(local);
    opts
}

// ------------------------------------------------------ the character machine

/// `morecomment=[l]`: a comment that runs to the end of the line, kept
/// as a block comment whose closing delimiter never matches.
const LINE_COMMENT: usize = usize::MAX;

/// The state `\lst@ProcessX` keeps across a line (and, for a block
/// comment, across lines).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Code,
    /// Inside `morecomment=[s]{a}{b}`; the index into `block_comment`.
    BlockComment(usize),
    /// Inside `morestring=[b]<c>`.
    String(char),
    /// Inside `morestring=[s]{a}{b}`; the index into `long_strings`.
    LongString(usize),
}

struct Machine<'a> {
    opts: &'a Options,
    mode: Mode,
    letter: bool,
    token: Vec<Piece>,
    length: usize,
    col: usize,
    /// `\lst@newlines`: no box has been set on this line yet.
    at_bol: bool,
    /// `\lst@ifwhitespace`.
    whitespace: bool,
    ops: Vec<Op>,
}

impl Machine<'_> {
    /// `\lst@PrintToken`: sets the open token unless it is empty. A token
    /// that holds only `\lst@nolig` has length 0 and is *not* set --
    /// listings leaves it in `\lst@token` to join the next one, which is
    /// why `ab,,cd` puts both commas in one two-cell box.
    fn flush(&mut self) {
        if self.length == 0 {
            return;
        }
        let pieces = std::mem::take(&mut self.token);
        let mut pieces = pieces;
        // `\lst@Output`'s keyword hook looks the token text up; a token
        // that swallowed a `\lst@nolig` cannot match, which is faithful.
        if let Some(lang) = self.opts.language {
            if self.letter && matches!(self.mode, Mode::Code) && pieces.iter().all(|p| p.ch.is_some()) {
                let text: String = pieces.iter().filter_map(|p| p.ch).collect();
                if lang.keywords.contains(&text.as_str()) {
                    for p in &mut pieces {
                        p.style = TokStyle::Keyword;
                    }
                }
            }
        }
        self.ops.push(Op::Box {
            cells: self.length,
            pieces,
        });
        self.length = 0;
        self.at_bol = false;
    }

    fn style(&self) -> TokStyle {
        match self.mode {
            Mode::Code => TokStyle::Basic,
            Mode::BlockComment(_) => TokStyle::Comment,
            Mode::String(_) | Mode::LongString(_) => TokStyle::Str,
        }
    }

    fn push(&mut self, ch: char, at: Range<usize>, visible_space: bool) {
        self.token.push(Piece {
            ch: Some(ch),
            style: self.style(),
            at,
            visible_space,
        });
        self.length += 1;
        self.col += 1;
    }

    /// `\lst@NoLig`: one zero-width item, no cell.
    fn push_nolig(&mut self, at: Range<usize>) {
        self.token.push(Piece {
            ch: None,
            style: self.style(),
            at,
            visible_space: false,
        });
    }

    fn append(&mut self, ch: char, at: Range<usize>) {
        if NOLIG.contains(&ch) {
            self.push_nolig(at.start..at.start);
        }
        match class(ch) {
            Class::Letter => {
                if !self.letter {
                    self.flush();
                    self.letter = true;
                }
            }
            Class::Other => {
                if self.letter {
                    self.flush();
                    self.letter = false;
                }
            }
            Class::Digit => {}
        }
        self.whitespace = false;
        self.push(ch, at, false);
        // `breaklines` redefines `)` as `\lst@breakProcessOther`
        // (`\lst@ProcessOther )\lst@OutputOther`), which flushes the token
        // there and then so a break can follow it. `);` is therefore two
        // one-cell boxes under `breaklines` and one two-cell box without.
        if self.opts.breaklines && ch == ')' {
            self.flush();
            self.letter = false;
        }
    }

    /// A delimiter (`/*`, `"`, `#`, ...) is its own token: `\lst@Delim`
    /// flushes what is open, sets the delimiter in the new style, and
    /// flushes again.
    fn delimiter(&mut self, text: &str, at: usize, style_after: Mode) {
        self.flush();
        self.letter = false;
        self.mode = style_after;
        let mut off = at;
        for ch in text.chars() {
            let len = ch.len_utf8();
            if NOLIG.contains(&ch) {
                self.push_nolig(off..off);
            }
            self.push(ch, off..off + len, false);
            off += len;
        }
        self.flush();
        self.whitespace = false;
    }

    /// `\lst@ProcessSpace`.
    fn space(&mut self, at: Range<usize>) {
        let keep = self.opts.keepspaces || (self.opts.showstringspaces && matches!(self.mode, Mode::String(_) | Mode::LongString(_)));
        let visible = self.opts.showspaces || (self.opts.showstringspaces && matches!(self.mode, Mode::String(_) | Mode::LongString(_)));
        if keep {
            self.flush();
            self.letter = false;
            self.whitespace = true;
            self.push(' ', at, visible);
            self.flush();
            return;
        }
        if self.at_bol && self.length == 0 {
            // `\ifnum\lst@newlines=\z@ \else \ifnum\lst@length=\z@`:
            // leading whitespace is lost space, not a space box.
            self.ops.push(Op::LostSpace);
            self.col += 1;
            self.whitespace = true;
            return;
        }
        // `\lst@AppendSpecialSpace`.
        if self.whitespace {
            self.flush();
            self.letter = false;
            self.ops.push(Op::LostSpace);
            self.col += 1;
        } else {
            self.flush();
            self.letter = false;
            self.whitespace = true;
            self.push(' ', at, false);
            self.flush();
        }
    }

    /// `\lst@ProcessTabulator`: `\lst@pos` runs to the next multiple of
    /// `tabsize`, which is an *absolute* tab stop -- unlike `\@verbatim`,
    /// where a tab is one ordinary space (PR #235).
    fn tab(&mut self, at: Range<usize>) {
        self.flush();
        self.letter = false;
        self.whitespace = true;
        let size = self.opts.tabsize.max(1);
        let cells = size - (self.col % size);
        if self.at_bol {
            self.ops.push(Op::TabLost { cells });
        } else {
            self.ops.push(Op::TabBox { cells, at });
        }
        self.col += cells;
    }
}

/// Plans every output line of one listing: which boxes, how many cells
/// each, and which style every character is in.
pub fn plan(body: &str, base: usize, opts: &Options) -> Vec<CodeLine> {
    let mut machine = Machine {
        opts,
        mode: Mode::Code,
        letter: false,
        token: Vec::new(),
        length: 0,
        col: 0,
        at_bol: true,
        whitespace: true,
        ops: Vec::new(),
    };
    let mut out: Vec<CodeLine> = Vec::new();
    let mut number = opts.firstnumber;
    let mut line_start = base;
    for raw in body.split('\n') {
        let start = line_start;
        line_start += raw.len() + 1;
        // `gobble=N` drops N characters from the front of the line before
        // anything else, column counting included.
        let mut chars: Vec<(usize, char)> = raw.char_indices().map(|(i, c)| (start + i, c)).collect();
        if opts.gobble > 0 {
            chars.drain(..opts.gobble.min(chars.len()));
        }
        machine.ops = Vec::new();
        machine.col = 0;
        machine.at_bol = true;
        machine.whitespace = true;
        machine.letter = false;
        machine.token.clear();
        machine.length = 0;
        if matches!(machine.mode, Mode::String(_)) {
            // `morestring=[b]` delimiters do not span lines.
            machine.mode = Mode::Code;
        }
        let mut i = 0usize;
        while i < chars.len() {
            let (at, ch) = chars[i];
            let rest: String = chars[i..].iter().map(|(_, c)| *c).collect();
            if let Some(step) = machine.try_delimiters(&rest, at) {
                i += step;
                continue;
            }
            match ch {
                ' ' => machine.space(at..at + 1),
                '\t' => machine.tab(at..at + 1),
                _ => machine.append(ch, at..at + ch.len_utf8()),
            }
            i += 1;
        }
        // `\lst@XPrintToken` at end of line.
        machine.flush();
        machine.letter = false;
        if matches!(machine.mode, Mode::String(_) | Mode::BlockComment(LINE_COMMENT)) {
            machine.mode = Mode::Code;
        }
        let blank = machine.ops.iter().all(|o| matches!(o, Op::LostSpace | Op::TabLost { .. }));
        let numbered = matches!(opts.numbers, NumberSide::Left | NumberSide::Right)
            && opts.stepnumber > 0
            && (number % opts.stepnumber == 0)
            && (opts.number_blank_lines || !blank);
        out.push(CodeLine {
            ops: std::mem::take(&mut machine.ops),
            number: numbered.then(|| number.to_string()),
            at: start..start + raw.len(),
        });
        number += 1;
    }
    out
}

impl Machine<'_> {
    /// Comment, string and line-comment delimiters at the head of `rest`.
    /// Returns how many characters were consumed.
    fn try_delimiters(&mut self, rest: &str, at: usize) -> Option<usize> {
        let lang = self.opts.language?;
        match self.mode {
            Mode::Code => {
                for (i, (open, _)) in lang.block_comment.iter().enumerate() {
                    if rest.starts_with(open) {
                        self.delimiter(open, at, Mode::BlockComment(i));
                        return Some(open.chars().count());
                    }
                }
                for open in lang.line_comment {
                    if rest.starts_with(open) {
                        // `morecomment=[l]`: the delimiter opens a comment
                        // that runs to the end of the line. It is a token
                        // of its own, so flush what is open and let the
                        // delimiter's own characters be appended in the new
                        // style like any others.
                        self.flush();
                        self.letter = false;
                        self.mode = Mode::BlockComment(LINE_COMMENT);
                        return None;
                    }
                }
                for (i, (open, _)) in lang.long_strings.iter().enumerate() {
                    if rest.starts_with(open) {
                        self.delimiter(open, at, Mode::LongString(i));
                        return Some(open.chars().count());
                    }
                }
                for d in lang.strings {
                    if rest.starts_with(*d) {
                        self.delimiter(&d.to_string(), at, Mode::String(*d));
                        return Some(1);
                    }
                }
                None
            }
            Mode::BlockComment(LINE_COMMENT) => None,
            Mode::BlockComment(i) => {
                let close = lang.block_comment.get(i)?.1;
                if rest.starts_with(close) {
                    self.flush();
                    self.letter = false;
                    let mut off = at;
                    for ch in close.chars() {
                        let len = ch.len_utf8();
                        if NOLIG.contains(&ch) {
                            self.push_nolig(off..off);
                        }
                        self.push(ch, off..off + len, false);
                        off += len;
                    }
                    self.flush();
                    self.mode = Mode::Code;
                    self.whitespace = false;
                    return Some(close.chars().count());
                }
                None
            }
            Mode::String(d) => {
                if rest.starts_with('\\') {
                    // `morestring=[b]`: a backslash escapes the next
                    // character, delimiter included.
                    let mut n = 1;
                    let mut it = rest.chars();
                    it.next();
                    let mut off = at + 1;
                    self.append('\\', at..at + 1);
                    if let Some(c) = it.next() {
                        self.append(c, off..off + c.len_utf8());
                        off += c.len_utf8();
                        n += 1;
                    }
                    let _ = off;
                    return Some(n);
                }
                if rest.starts_with(d) {
                    self.flush();
                    self.letter = false;
                    // `'` is itself a `\@noligs` character: measured, the
                    // zero-width item lands in the delimiter's own box (the
                    // string's last token keeps N items), so a closing `'`
                    // sits at 2(W-w)/3 into its cell and not at (W-w)/2.
                    if NOLIG.contains(&d) {
                        self.push_nolig(at..at);
                    }
                    self.push(d, at..at + d.len_utf8(), false);
                    self.flush();
                    self.mode = Mode::Code;
                    self.whitespace = false;
                    return Some(1);
                }
                None
            }
            Mode::LongString(i) => {
                let close = lang.long_strings.get(i)?.1;
                if rest.starts_with(close) {
                    self.flush();
                    self.letter = false;
                    let mut off = at;
                    for ch in close.chars() {
                        let len = ch.len_utf8();
                        if NOLIG.contains(&ch) {
                            self.push_nolig(off..off);
                        }
                        self.push(ch, off..off + len, false);
                        off += len;
                    }
                    self.flush();
                    self.mode = Mode::Code;
                    self.whitespace = false;
                    return Some(close.chars().count());
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_lists_split_at_top_level_commas() {
        let kv = key_values("basicstyle=\\ttfamily\\small,columns=[l]fixed,basewidth={0.6em,0.45em},frame=single");
        assert_eq!(kv[0], ("basicstyle".into(), Some("\\ttfamily\\small".into())));
        assert_eq!(kv[1], ("columns".into(), Some("[l]fixed".into())));
        assert_eq!(kv[2], ("basewidth".into(), Some("{0.6em,0.45em}".into())));
        assert_eq!(kv[3], ("frame".into(), Some("single".into())));
    }

    #[test]
    fn columns_reads_the_optional_alignment() {
        let mut o = Options::default();
        o.apply("columns=[l]fixed");
        assert_eq!((o.columns, o.pos), (Columns::Fixed, Pos::Left));
        let mut o = Options::default();
        o.apply("columns=fullflexible");
        assert_eq!((o.columns, o.pos), (Columns::FullFlexible, Pos::Center));
    }

    #[test]
    fn xleftmargin_keeps_its_em() {
        let mut o = Options::default();
        o.apply("xleftmargin=2em");
        assert_eq!(o.xleftmargin, Dimen { pt: 0.0, em: 2.0, ex: 0.0 });
        // cmtt8's quad is 1.062515, not 1.049991: 2em is 17.00024pt.
        assert!((o.xleftmargin.resolve(8.50012, 0.0) - 17.00024).abs() < 1e-6);
    }

    #[test]
    fn a_nolig_character_adds_an_item_without_a_cell() {
        let opts = Options::default();
        let lines = plan("abc,def", 0, &opts);
        let Op::Box { cells, pieces } = &lines[0].ops[0] else { panic!() };
        assert_eq!(*cells, 3);
        // `a`, `b`, `c`, `\lst@nolig`: four items, so `[c]` divides the
        // slack into five and not four.
        assert_eq!(pieces.len(), 4);
        assert!(pieces[3].ch.is_none());
    }

    #[test]
    fn two_commas_share_one_box() {
        let opts = Options::default();
        let lines = plan("ab,,cd", 0, &opts);
        let boxes: Vec<_> = lines[0]
            .ops
            .iter()
            .filter_map(|o| match o {
                Op::Box { cells, pieces } => Some((*cells, pieces.len())),
                _ => None,
            })
            .collect();
        // `ab` + nolig (3 items, 2 cells), then `,` + nolig + `,`
        // (3 items, 2 cells), then `cd`.
        assert_eq!(boxes, vec![(2, 3), (2, 3), (2, 2)]);
    }

    #[test]
    fn leading_space_is_lost_space_and_a_mid_line_space_is_a_box() {
        let opts = Options::default();
        let lines = plan("  ab cd", 0, &opts);
        let kinds: Vec<&str> = lines[0]
            .ops
            .iter()
            .map(|o| match o {
                Op::LostSpace => "lost",
                Op::Box { .. } => "box",
                Op::TabBox { .. } => "tabbox",
                Op::TabLost { .. } => "tablost",
            })
            .collect();
        assert_eq!(kinds, vec!["lost", "lost", "box", "box", "box"]);
    }

    #[test]
    fn tab_stops_are_absolute() {
        let mut opts = Options::default();
        opts.apply("tabsize=4,gobble=2");
        let lines = plan("  a\tb\n  \tc\n    d", 0, &opts);
        // `a` then a tab: three cells to column 4.
        match &lines[0].ops[1] {
            Op::TabBox { cells, .. } => assert_eq!(*cells, 3),
            other => panic!("{other:?}"),
        }
        // A tab that opens a line is pure lost space, four cells of it.
        match &lines[1].ops[0] {
            Op::TabLost { cells } => assert_eq!(*cells, 4),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn comments_and_strings_take_their_style() {
        let mut opts = Options::default();
        opts.apply("language=C");
        let lines = plan("int x; /* c */\nputs(\"a b\");", 0, &opts);
        let styles: Vec<TokStyle> = lines[0]
            .ops
            .iter()
            .filter_map(|o| match o {
                Op::Box { pieces, .. } => pieces.first().map(|p| p.style),
                _ => None,
            })
            .collect();
        assert_eq!(styles[0], TokStyle::Keyword, "`int` is a C keyword");
        assert!(styles.last().is_some_and(|s| *s == TokStyle::Comment));
        let strings: Vec<bool> = lines[1]
            .ops
            .iter()
            .filter_map(|o| match o {
                Op::Box { pieces, .. } => pieces.first().map(|p| p.style == TokStyle::Str),
                _ => None,
            })
            .collect();
        assert!(strings.iter().any(|s| *s));
    }

    #[test]
    fn showstringspaces_keeps_every_space_in_a_string() {
        let mut opts = Options::default();
        opts.apply("language=C");
        let lines = plan("puts(\"two  spaces\");", 0, &opts);
        let visible = lines[0]
            .ops
            .iter()
            .filter(|o| matches!(o, Op::Box { pieces, .. } if pieces.iter().any(|p| p.visible_space)))
            .count();
        assert_eq!(visible, 2, "both spaces are set as \\lst@visiblespace");
        opts.apply("showstringspaces=false");
        let lines = plan("puts(\"two  spaces\");", 0, &opts);
        let lost = lines[0].ops.iter().filter(|o| matches!(o, Op::LostSpace)).count();
        assert_eq!(lost, 1, "the second space of the run is lost space");
    }

    #[test]
    fn breaklines_makes_a_closing_paren_end_its_token() {
        // `\lst@breakProcessOther` (lstmisc.sty 1349) is
        // `\lst@ProcessOther )\lst@OutputOther`: `);` is two one-cell boxes
        // under `breaklines` where it is one two-cell box without, and in
        // `[c]fixed` that moves both glyphs.
        let cells = |src: &str, on: bool| -> Vec<usize> {
            let mut o = Options::default();
            if on {
                o.apply("breaklines=true");
            }
            plan(src, 0, &o)[0]
                .ops
                .iter()
                .filter_map(|op| match op {
                    Op::Box { cells, .. } => Some(*cells),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(cells("f(x);", false), vec![1, 1, 1, 2]);
        assert_eq!(cells("f(x);", true), vec![1, 1, 1, 1, 1]);
    }

    #[test]
    fn a_closing_string_delimiter_carries_its_own_nolig() {
        // `'` is in `\verbatim@nolig@list`. Measured on 24-lst-python-keywords:
        // the string's last token keeps N items and the closing `'` box holds
        // two, so it sits at 2(W-w)/3 into its cell.
        let mut o = Options::default();
        o.apply("language=Python");
        let lines = plan("s = 'abc'", 0, &o);
        let last = lines[0]
            .ops
            .iter()
            .filter_map(|op| match op {
                Op::Box { cells, pieces } => Some((*cells, pieces.len())),
                _ => None,
            })
            .last()
            .unwrap();
        assert_eq!(last, (1, 2), "the closing quote's box is one cell and two items");
    }

    #[test]
    fn environments_and_lstset_are_found_in_the_source() {
        let src = "\\lstset{basicstyle=\\ttfamily}\nx\n\\begin{lstlisting}[frame=single]\ncode\n\\end{lstlisting}\n";
        let sets = find_lstset(src);
        assert_eq!(sets.len(), 1);
        assert_eq!(&src[sets[0].0.clone()], "\\lstset{basicstyle=\\ttfamily}");
        let envs = find_environments(src);
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0].options, "frame=single");
        assert_eq!(&src[envs[0].body.clone()], "code");
        let opts = options_at(src, envs[0].span.start, &envs[0].options);
        assert_eq!(opts.basic.family, Some(crate::nfss::FamilyKind::Tt));
        assert!(opts.frame.top && opts.frame.left);
    }
}
