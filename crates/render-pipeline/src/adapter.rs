//! Adapter: compiler parse tree -> styled block model.
//!
//! The compiler's `parser::Parsed` (blocks of `Inline::Text` words with exact
//! byte spans, `Inline::Math` lists, `Inline::LineBreak`) drops information
//! this pipeline needs: which words were inside `\textbf`/`\emph`/`\textit`,
//! whether whitespace separated two words, the class options, `\parindent`,
//! and TeX's input conventions (`---`, quotes, `\'e`). Every one of those is
//! re-derived here from the exact source bytes the spans point into, which
//! is possible because the spans are exact. What the compiler lead is asked
//! to expose instead is listed in docs/proposals/rendering-abi.md
//! ("Requested compiler API").

use std::collections::{BTreeMap, HashMap};

use flashtex_compiler::math::MathList;
use flashtex_compiler::parser::{Block as CBlock, FillLeader, Inline, Parsed, UnderlineGeom};
use flashtex_compiler::text_builtins::{TextDimen, TextLogo, TextRule};
use flashtex_compiler::{DocumentId, Span};

use flashtex_class_geometry::{
    ClassKind, DocumentSetup, GeometryInput, Glue, PageFrame, PageParams, PageStyle, ResolvedDocument,
    Sp,
};

use crate::display::Diagnostic;
use flashtex_compiler::color::DeviceColor;
use crate::style::Stylesheet;
use crate::RenderOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TextStyle {
    pub bold: bool,
    pub italic: bool,
    /// Font size set by a size declaration (`\Large`, ...) in force, in
    /// hundredths of a point; 0 keeps the paragraph's size.
    pub size_cpt: u16,
    /// `\normalfont`/`\mdseries` in force inside a heading: the block's
    /// own weight (`\bfseries` from `\@startsection`) is not applied.
    pub medium: bool,
    /// Shape `sl` (`\slshape`, running heads); `scsl` with `caps`.
    pub slanted: bool,
    /// The compiler's text colour (`TextStyle::color`): the glyph run's paint.
    pub color: Option<DeviceColor>,
    /// Small caps (`\scshape`): shape `sc`, or `scit`/`scsl` with
    /// `italic`/`slanted`.
    pub caps: bool,
    /// `\rmfamily`/`\sffamily`/`\ttfamily`.
    pub family: crate::nfss::FamilyKind,
    /// Verbatim text: `\verb`/`\verb*`, the `verbatim`/`verbatim*` and
    /// `lstlisting` environments, and `\lstinline`.
    ///
    /// TeX typesets these with every ligature and kern suppressed
    /// (`\@noligs`) and with each blank a rigid `\fontdimen2` rather than
    /// interword glue, so the run is *not* the same as `\texttt` over the
    /// same characters: measured against pdflatex at 12 pt T1,
    /// `\verb|x--y|` is 24.69397 pt (4 characters of `ectt1200`) while
    /// `\ttfamily x--y` is 18.52048 pt (3, the `--` having ligated). The
    /// family alone therefore cannot carry this; it is a separate property
    /// of the *text*, and `\texttt` must keep its ligatures.
    pub literal: bool,
    /// The shape LaTeX reported undefined on the way to this style
    /// (`\wrong@fontshape`); the typesetter reports it once.
    pub undefined: Option<crate::nfss::FontKey>,
}

impl TextStyle {
    /// The NFSS shape of this style: series `bx` when bold, and `italic`
    /// wins over `slanted` when a merge with an enclosing style set both.
    pub fn key(self) -> crate::nfss::FontKey {
        use crate::nfss::{FontKey, Series, Shape};
        let shape = match (self.caps, self.italic, self.slanted) {
            (true, true, _) => Shape::Scit,
            (true, false, true) => Shape::Scsl,
            (true, false, false) => Shape::Sc,
            (false, true, _) => Shape::It,
            (false, false, true) => Shape::Sl,
            (false, false, false) => Shape::N,
        };
        FontKey::new(self.family, if self.bold { Series::Bx } else { Series::M }, shape)
    }

    /// This style with its family, series and shape replaced by `key`'s.
    pub fn with_key(self, key: crate::nfss::FontKey) -> TextStyle {
        use crate::nfss::Shape;
        TextStyle {
            bold: key.bold(),
            italic: matches!(key.shape, Shape::It | Shape::Scit),
            slanted: matches!(key.shape, Shape::Sl | Shape::Scsl),
            caps: matches!(key.shape, Shape::Sc | Shape::Scit | Shape::Scsl),
            family: key.family,
            ..self
        }
    }

    /// The size to shape at, given the paragraph's `size`.
    pub fn size_or(self, size: f64) -> f64 {
        if self.size_cpt == 0 {
            size
        } else {
            f64::from(self.size_cpt) / 100.0
        }
    }
}

/// One output character and the source bytes it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharSrc {
    pub document: DocumentId,
    pub start: usize,
    pub end: usize,
}

impl CharSrc {
    pub fn span(&self) -> Span {
        Span::in_document(self.document, self.start, self.end)
    }
}

/// A maximal run of characters in one style with no interword space.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub text: String,
    /// One entry per `char` of `text`, in order.
    pub chars: Vec<CharSrc>,
    pub style: TextStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    pub segments: Vec<Segment>,
}

impl Word {
    /// Smallest span covering every character of the word, in the document
    /// of its first character (a word never straddles two documents).
    pub fn span(&self) -> Span {
        let document = self.segments.iter().flat_map(|s| s.chars.iter()).map(|c| c.document).next().unwrap_or_default();
        let start = self.segments.iter().flat_map(|s| s.chars.iter()).map(|c| c.start).min().unwrap_or(0);
        let end = self.segments.iter().flat_map(|s| s.chars.iter()).map(|c| c.end).max().unwrap_or(0);
        Span::in_document(document, start, end)
    }
    pub fn text(&self) -> String {
        self.segments.iter().map(|s| s.text.as_str()).collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Word(Word),
    /// Interword glue. `factor` is TeX's space factor (1000 normal, 3000
    /// after sentence-ending punctuation, 999 after an uppercase letter).
    Space { style: TextStyle, factor: u32, no_break: bool },
    Math { list: MathList, span: Span },
    /// `\\`; `skip_pt` is the optional `[<dimen>]` (LaTeX `\@xnewline`:
    /// `\vadjust{\vskip <dimen>}` after the line, or `\vskip` after the
    /// paragraph under `\@centercr`).
    LineBreak { skip_pt: f64 },
    /// Fixed horizontal glue of `em` ems of the current font (`\quad`
    /// after a section number).
    Quad { em: f64 },
    /// `\label{key}`: no material; records where the key's page is.
    Label { key: String },
    /// `\/` after a `\textit`/`\emph`/`\textbf` argument (LaTeX's
    /// `\text@command` adds it unless `.` or `,` follows).
    ItalicCorrection,
    /// A paragraph break inside a footnote's text (the compiler attributes
    /// it to the `\footnote` command's span): `\par`, then the next
    /// paragraph's `\indent` box (`\@makefntext`'s `\parindent` 1em).
    NoteParBreak,
    /// `\hfill`/`\hfil` (compiler `Inline::HFill`): infinitely stretchable
    /// glue; a legal break point that is discarded at a line break. `fill`
    /// is the `\hfill` order (it beats `\parfillskip`'s `fil`); the
    /// compiler does not distinguish the two, so the order is re-read from
    /// the source bytes (`\hfill` when they are not `\hfil`).
    HFill { fill: bool, leader: FillLeader },
    /// Explicit horizontal glue in points: `\hspace{<dimen>}` (compiler
    /// `Inline::HSpace`, rigid) or an amsthm theorem head's own separator
    /// (`\hskip\thm@headsep`, `5pt plus 1pt minus 1pt`; `crate::amsthm`).
    HSpace { pt: f64, stretch_pt: f64, shrink_pt: f64 },
    /// `tabular`/`tabular*` (compiler `Inline::Tabular`): one box in the
    /// paragraph, laid out by `table.rs`.
    Table(Box<crate::table::TableItem>),
    /// `\TeX`/`\LaTeX`/`\LaTeXe` (compiler `Inline::Logo`): latex.ltx's
    /// construction, set by `typeset` from the face's TFM metrics.
    Logo { logo: TextLogo, style: TextStyle, span: Span },
    /// `\rule[<raise>]{<width>}{<height>}` (compiler `Inline::Rule`).
    Rule { rule: TextRule, style: TextStyle, span: Span },
    /// A text-mode kern (`\,`, `\thinspace`, `\enspace`, ...; compiler
    /// `Inline::Kern`), in ems of the current face.
    Kern { amount: TextDimen, style: TextStyle },
    /// `\footnote`, `\footnotemark` or `\footnotetext` (compiler
    /// `Inline::Footnote`). `number` is `\@thefnmark`; `mark` sets
    /// `\@makefnmark` here (false for `\footnotetext`); `text` is the note's
    /// items, set in `\footnotesize` at the foot of the column by
    /// `typeset::footnotes` (`None` for `\footnotemark`). `span` is the
    /// command token.
    Footnote { number: String, mark: bool, span: Span, text: Option<Vec<Item>> },
    /// `\colorbox`/`\fcolorbox` (compiler `Inline::ColorBox`).
    ColorBox(Box<ColorBoxItem>),
    /// LaTeX's `\llap{...}`: `items` set at their natural width and then
    /// pulled back by exactly that width, so the line's reference point does
    /// not move and the material hangs in the left margin.
    ///
    /// The first thing emitted for it is an empty `\hbox` (the same
    /// undiscardable anchor [`Item::LeaveVmode`] is), because the pull-back
    /// is a kern and a kern at the head of a line is discarded (TeX §879) —
    /// which is exactly where `listings` puts one, on every numbered line.
    Lap { items: Vec<Item> },
    /// ulem `\uline`/`\sout` or kernel text `\underline` (compiler
    /// `Inline::Underline`).
    Underline(Box<UnderlineItem>),
    /// LaTeX's `\leavevmode`: an empty zero-width `\hbox`.
    ///
    /// Emitted only in front of a verbatim blank that would otherwise open
    /// a line, and there for the reason TeX has it. `verbatim` sets
    /// `\obeylines` and makes the blank `\@xobeysp` = `\leavevmode\penalty
    /// \@M\ `, so an indented line starts *box, glue* rather than *glue* —
    /// and only glue at the head of a horizontal list is discarded
    /// (TeX §879, `pl::Item::is_discardable`). Without the box, every
    /// leading space of every listing is dropped and the indentation of a
    /// code block disappears.
    ///
    /// pdflatex shows it: `\showbox` of `\verb*"a b-c"` opens with
    /// `.\hbox(0.0+0.0)x0.0`.
    LeaveVmode,
}

/// A `\colorbox`/`\fcolorbox`: `items` set as an `\hbox` on a `fill`
/// rectangle `sep_pt` larger on every side, inside a `rule_pt` frame of
/// colour `frame` for `\fcolorbox`.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorBoxItem {
    pub fill: DeviceColor,
    pub frame: Option<DeviceColor>,
    pub sep_pt: f64,
    pub rule_pt: f64,
    pub items: Vec<Item>,
    pub span: Span,
}

/// A `\uline`/`\sout`/`\underline`: `items` set as an `\hbox`, with a
/// `thickness_pt` rule placed by `geom` (ulem descender, TeXbook Rule 10,
/// or a 0.55ex strike).
#[derive(Debug, Clone, PartialEq)]
pub struct UnderlineItem {
    pub thickness_pt: f64,
    pub geom: UnderlineGeom,
    pub items: Vec<Item>,
    pub span: Span,
}

/// Which amsmath display alignment a [`ParaPart::Rows`] is (read from the
/// environment name at the display's first byte; the compiler keeps only
/// whether cells alternate right/left).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RowsEnv {
    /// `align`/`align*`: column pairs spread evenly (`\xatlevel@` 1).
    Align,
    /// `alignat`/`alignat*`: column pairs set with no added space (level 0).
    AlignAt,
    /// `flalign`/`flalign*`: column pairs pushed to the margins (level 2).
    FlAlign,
    /// `gather`/`gather*`: every row centred on its own.
    Gather,
    /// `multline`/`multline*`: first row left, last row right, others centred.
    Multline,
}

impl RowsEnv {
    /// The environment opening at the start of `rest` (`\begin{align*}...`).
    pub fn at(rest: &str) -> RowsEnv {
        let name = rest.strip_prefix("\\begin{").and_then(|r| r.split('}').next()).unwrap_or("");
        match name.trim_end_matches('*') {
            "alignat" => RowsEnv::AlignAt,
            "flalign" => RowsEnv::FlAlign,
            "gather" => RowsEnv::Gather,
            "multline" => RowsEnv::Multline,
            _ => RowsEnv::Align,
        }
    }
}

/// One row of a [`ParaPart::Rows`] display: its `&`-separated cells, its
/// equation number (or `\tag` text) and the row's source span.
#[derive(Debug, Clone, PartialEq)]
pub struct RowPart {
    pub cells: Vec<MathList>,
    pub number: Option<(String, Span)>,
    pub span: Span,
    /// `\intertext` paragraphs set before this row (feature
    /// `amsmath-inline`; always empty otherwise).
    pub intertext: Vec<IntertextPart>,
}

/// One `\intertext{..}`/`\shortintertext{..}` of a [`RowPart`].
#[derive(Debug, Clone, PartialEq)]
pub struct IntertextPart {
    pub items: Vec<Item>,
    pub short: bool,
    /// mathtools is loaded: its `\MT_intertext:`/`\MT_shortintertext:n`
    /// replace amsmath's `\intertext@` (`original-intertext=false`).
    pub mathtools: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParaPart {
    /// An amsmath multi-row display laid out as one alignment (row pitch
    /// `\baselineskip + \jot`, cells at amsmath's column positions, numbers
    /// flush right) instead of one centred display per row.
    Rows {
        env: RowsEnv,
        rows: Vec<RowPart>,
        span: Span,
        bracket: bool,
    },
    Lines(Vec<Item>),
    /// A display; `number` is the `equation` counter text and the
    /// environment's source span (`\eqno` at the right margin).
    /// `bracket` marks LaTeX's `\[`/`displaymath`, which in vertical mode
    /// first sets an empty `.6\linewidth` box with `\nointerlineskip`.
    /// Under amsmath `\[` is `\begin{equation*}`, whose `\mathdisplay`
    /// is a bare `$$` (no box, no `\nointerlineskip`), so it is `false`
    /// there.
    Display {
        list: MathList,
        span: Span,
        number: Option<(String, Span)>,
        bracket: bool,
    },
}

/// LaTeX paragraph-shape environments (compiler `Block::Styled`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ParaStyle {
    #[default]
    Plain,
    /// `center`: `\centering` (`\leftskip`/`\rightskip` `0pt plus 1fil`,
    /// `\parfillskip 0pt`).
    Center,
    /// `flushleft`: `\raggedright`.
    FlushLeft,
    /// `flushright`: `\raggedleft`.
    FlushRight,
    /// `quote`/`quotation`: a level-1 list with `\rightmargin=\leftmargin`.
    Quote,
}

impl ParaStyle {
    fn of(style: flashtex_compiler::parser::ParagraphStyle) -> ParaStyle {
        use flashtex_compiler::parser::ParagraphStyle as P;
        match style {
            P::Center => ParaStyle::Center,
            P::FlushLeft => ParaStyle::FlushLeft,
            P::FlushRight => ParaStyle::FlushRight,
            P::Quote => ParaStyle::Quote,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Paragraph {
        parts: Vec<ParaPart>,
        indent: bool,
        style: ParaStyle,
        /// The paragraph opens its `center`/`quote`/... environment
        /// (`\trivlist`/`\list`: `\topsep` glue before it, plus
        /// `\partopsep` when the environment began in vertical mode) and/or
        /// closes it (`\@endparenv`: the same glue after it).
        env_open: Option<EnvOpen>,
        env_close: bool,
        /// `\newpage`/`\clearpage`/`\pagebreak` stood between the previous
        /// block and this one (the compiler reports and drops the command;
        /// the break is recovered from the source bytes).
        eject_before: bool,
        /// `\vspace{<dimen>}` blocks between the previous block and this
        /// one (compiler `Block::VSpace`), summed in points; `\addvspace`
        /// glue added before the block.
        vspace_before: f64,
        /// LaTeX `\addvspace` glue before the block (`\@item`'s `\topsep`/
        /// `\itemsep`, `\@endparenv`'s `\@topsepadd`), in points: only
        /// its excess over the previous block's trailing skip (a display's
        /// `\belowdisplayskip`) is added.
        addvspace_before: f64,
        /// The stretch and shrink of `addvspace_before` and of the plain
        /// `\vskip` part of `vspace_before`, in points. LaTeX's list skips
        /// are glue; only their natural width fits in the two scalars above,
        /// and a page that loses their `\@plus`/`\@minus` breaks in a
        /// different place from pdfTeX's.
        addvspace_flex: (f64, f64),
        vspace_flex: (f64, f64),
        /// `\endtrivlist` of the list(s) closed between the previous block
        /// and this one: when the previous block left a positive trailing
        /// skip (a display's `\belowdisplayskip`), each closing list
        /// changes it by its `\parsep` minus the `\parskip` outside it,
        /// in points; summed innermost first. Nothing when there was no
        /// trailing skip.
        endlist_adjust: f64,
        /// The paragraph is (part of) an `itemize`/`enumerate` `\item`
        /// (compiler `Block::ListItem`): LaTeX's `\list` geometry applies.
        list: Option<ListGeom>,
        /// The paragraph is set at a size other than `\normalsize`
        /// (`abstract`'s `\small`); see [`SizedPara`].
        sized: Option<SizedPara>,
        /// `\baselineskip` for every line of this paragraph and for the glue
        /// above its first one, when the `\par` that ended it ran under a
        /// size declaration ([`ParLeading`]). `None` is the body's. Set
        /// *instead of* the whole-paragraph resize [`SizedPara`] carries:
        /// the runs keep their own sizes, only the leading moves.
        leading_pt: Option<f64>,
    },
    Heading {
        level: u8,
        items: Vec<Item>,
        eject_before: bool,
        vspace_before: f64,
        /// The displayed number (`""` for a starred heading) and the title
        /// as plain source text, for the `\sectionmark` running head.
        number: String,
        title: String,
        span: Span,
    },
    /// `\chapter` in report/book (the compiler reports the command and sets
    /// its argument as body text, which is dropped): `\clearpage`,
    /// `\thispagestyle{plain}`, `\chaptermark` and `\@makechapterhead`.
    /// `number` is `None` for `\chapter*`.
    Chapter {
        number: Option<String>,
        /// After `\appendix`: `\@chapapp` is `\appendixname`.
        appendix: bool,
        items: Vec<Item>,
        title: String,
        span: Span,
        /// `\chaptermark` is issued (unstarred `\chapter`, numbered or, in
        /// book's `\frontmatter`/`\backmatter`, not: `\if@mainmatter` only
        /// drops the `Chapter <n>.` prefix).
        mark: bool,
    },
    /// `\part` (the compiler reports the command and sets its argument as
    /// body text, which is dropped): article.cls lines 268-301 in the
    /// flow, report.cls 278-328 / book.cls 299-349 on a page of its own.
    /// `number` is `\thepart` (`None` for `\part*`).
    /// `eject_before`: a page-break command stands right before it
    /// (`clear_before`: `\clearpage`/`\cleardoublepage`).
    Part { number: Option<String>, items: Vec<Item>, span: Span, eject_before: bool, clear_before: bool },
    /// `\maketitle` (article.cls lines 169-251, report.cls 175-257,
    /// book.cls 181-263): the compiler's title, the `\and`-separated
    /// authors (each a `tabular` whose rows are split at `\\`) and the date
    /// (`None` for `\date{}`). The class decides between `\@maketitle` and
    /// the `titlepage` form.
    Title {
        title: Vec<Item>,
        authors: Vec<Vec<Vec<Item>>>,
        date: Option<Vec<Item>>,
        span: Span,
    },
    /// A class command's `\clearpage` (`double`: `\cleardoublepage`), from
    /// book.cls `\frontmatter`/`\mainmatter`/`\backmatter` (lines 284-298):
    /// the next material starts a new page (an odd one when two-sided).
    ClearPage { double: bool, span: Span },
    /// A page-style or mark command in the body, attached to the material
    /// that follows it.
    Chrome { event: ChromeEvent, span: Span },
    /// One `\tableofcontents`/`\listoffigures`/`\listoftables` entry line
    /// (`crate::toc`).
    TocEntry(Box<crate::toc::TocEntry>),
    /// A `tikzpicture`, found from the source bytes (the compiler reports the
    /// environment as unknown and sets its body as text, which is dropped
    /// here): its bounding box as one box on a line of its own, flush left
    /// (centred inside `center`).
    Picture {
        document: flashtex_compiler::DocumentId,
        picture: flashtex_vector_graphics::tikz::PictureSource,
        centered: bool,
        eject_before: bool,
        vspace_before: f64,
    },
    /// `\hrule` in vertical mode: a full-measure rule 0.4pt high with no
    /// interline glue on either side (TeX §1056 sets `prev_depth` to
    /// `ignore_depth`).
    Rule {
        span: Span,
        eject_before: bool,
        vspace_before: f64,
    },
    /// A `longtable` (`crate::longtable`). Unlike `tabular` this is not a
    /// box inside a paragraph: `\LT@array` contributes every row straight
    /// to the page's vertical list so the page builder can break between
    /// them, repeating `\LT@head` and `\LT@foot`.
    LongTable {
        table: Box<crate::table::TableItem>,
        eject_before: bool,
        vspace_before: f64,
        /// The package's own skips and dimensions, as `\setlength` left
        /// them: `\LTpre`/`\LTpost` (`\bigskipamount` by default),
        /// `\LTleft`/`\LTright` (`\fill`) and `\LTcapwidth` (4in).
        lengths: LongtableLengths,
        /// `\label` keys inside the table, so `\caption`'s number can be
        /// referenced.
        labels: Vec<String>,
    },
}

/// longtable.sty 61-67: the lengths a document may `\setlength`. `None`
/// keeps the package default.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LongtableLengths {
    pub pre: Option<f64>,
    pub post: Option<f64>,
    pub left: Option<f64>,
    pub right: Option<f64>,
    pub capwidth: Option<f64>,
}

impl LongtableLengths {
    /// Reads each one through a `\setlength` lookup.
    pub fn read(mut value: impl FnMut(&str) -> Option<f64>) -> LongtableLengths {
        LongtableLengths {
            pre: value("LTpre"),
            post: value("LTpost"),
            left: value("LTleft"),
            right: value("LTright"),
            capwidth: value("LTcapwidth"),
        }
    }
}

/// LaTeX `\list` geometry of one `\item` paragraph (see
/// [`Block::Paragraph::list`]). `\list` sets `\parshape` so every line of
/// the item starts `\@totalleftmargin` (the sum of the enclosing lists'
/// `\leftmargin`s) in from the left margin, and `\@item` sets the label
/// right-aligned in `\hbox to\labelwidth{\hss <label>}\hskip\labelsep`
/// before the first line, so its right edge ends `\labelsep` before the
/// text (article's `\makelabel` is `\hss\llap{#1}`, so a wider label
/// simply extends further left).
#[derive(Debug, Clone, PartialEq)]
pub struct ListGeom {
    /// Nesting level (1 = outermost).
    pub level: u8,
    /// `\leftmargin` of every enclosing list, outermost first; the hanging
    /// indent is their sum.
    pub margins: Vec<ListMargin>,
    /// The `\item` marker text and the command's span; `None` for a later
    /// paragraph of the same item (a blank line inside the item's text).
    pub label: Option<(String, Span)>,
    /// The innermost list's `\parsep` (`\list` sets `\parskip\parsep`):
    /// the glue every paragraph of the item adds. Article's `\@list<i>`
    /// value for the nesting level, or an enumitem `parsep=` key.
    pub parsep: crate::style::Skip,
    /// The innermost list's `\itemindent`, in `em` of the body font: the
    /// first line of an item starts `\leftmargin + \itemindent` in. Zero for
    /// every list the classes set; natbib's author-year `\thebibliography`
    /// is the one that is not — `\NAT@bibsetup` (natbib.sty line 642) sets
    /// `\leftmargin\bibhang` (1 em) and `\itemindent-\leftmargin`, so each
    /// entry's first line is flush at the margin and its continuation lines
    /// hang 1 em in.
    pub itemindent_em: f64,
    /// The innermost list is a `description` (article.cls: `\list{}{%
    /// \labelwidth\z@ \itemindent-\leftmargin
    /// \let\makelabel\descriptionlabel}`). Three things follow, all of them
    /// the typesetter's: the item's first line starts flush at the margin
    /// (`\itemindent` cancels `\leftmargin`, so only the continuation lines
    /// hang in), the label is never padded to a `\labelwidth` because that
    /// is zero, and `\descriptionlabel` sets it as `\hspace\labelsep
    /// \normalfont\bfseries <label>`.
    pub description: bool,
}

/// One list level's `\leftmargin`.
#[derive(Debug, Clone, PartialEq)]
pub enum ListMargin {
    /// article's `\leftmargin<i>` (or an explicit enumitem
    /// `leftmargin=<dimen>`), in points.
    Fixed(f64),
    /// enumitem `leftmargin=*`: `\labelwidth` + `\labelsep`, where
    /// `\labelwidth` is the width of this label — the widest one the list
    /// can produce (enumitem's `widest` default: `m`/`M`/`viii`/`VIII`/`0`
    /// for `\alph`/`\Alph`/`\roman`/`\Roman`/`\arabic`), set in the
    /// body font.
    Widest(String),
    /// A `\leftmargin` stated in `em` of the body font, which is where
    /// natbib's `\bibhang` (`1em`, natbib.sty line 638) comes from. Kept as
    /// `em` rather than points so it is resolved against the font's own
    /// `\fontdimen6` at typeset time (cmr10 at 11 pt: 10.95003 pt), which is
    /// what `\setlength{\bibhang}{1em}` measured.
    Em(f64),
}

/// Body commands that decide the header and footer (latex.ltx
/// `\pagestyle`/`\thispagestyle`/`\markboth`/`\markright`). Mark text is
/// the argument's source with whitespace collapsed; a `\quad` inside a
/// class-generated mark is U+2003.
#[derive(Debug, Clone, PartialEq)]
pub enum ChromeEvent {
    PageStyle(PageStyle),
    ThisPageStyle(PageStyle),
    MarkBoth(String, String),
    MarkRight(String),
    /// `\pagenumbering{style}`: `\thepage` style and `\c@page` reset to 1.
    PageNumbering(flashtex_class_geometry::Numbering),
    /// `\setcounter{page}{n}`.
    SetPage(i64),
}

/// A paragraph set at a size other than `\normalsize`, with everything
/// `\@setfontsize` changes for it: the size itself, *that size's own*
/// `\baselineskip`, and any length the environment resolves in the new
/// size's `em` (`\fontdimen6` of the face its own words are set in, which
/// only the typesetter can measure).
///
/// The one producer today is `abstract` (article.cls 377-387): the centred
/// `\small\bfseries` head and the `\small` `quotation` body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizedPara {
    /// `\@setfontsize`'s first argument, in points.
    pub size_pt: f64,
    /// The `\baselineskip` that size selects (`size1x.clo`'s table), in
    /// force for every line of this paragraph.
    pub baselineskip_pt: f64,
    /// `\parindent` in `em` of this size, replacing the class's
    /// (`quotation`'s `\listparindent 1.5em`, which `\list` copies into
    /// `\parindent` and `\@item` re-adds as `\itemindent` on the first
    /// line). `None` keeps the class's `\parindent`.
    pub parindent_em: Option<f64>,
    /// `\vspace` after the paragraph, in `em` of the font its *last word*
    /// is set in — the abstract head's `\vspace{-.5em}`, which sits inside
    /// the `{\bfseries ...}` group, so it is half a `\bfseries` quad.
    /// Added to whatever `\@endparenv` puts there (`\addvspace` cannot
    /// absorb it: it is emitted through `\vadjust`, before the penalty and
    /// the closing skip).
    pub vspace_after_em: f64,
    /// The closing `\@endparenv` skip of the environment this paragraph
    /// ends, when the size redefined `\@list i` (`\small`'s own `\topsep`,
    /// 4pt at a 10pt base rather than `\normalsize`'s 8pt). `None` keeps
    /// the class's.
    pub close_skip: Option<crate::style::Skip>,
}

/// The size declaration in force when a paragraph's `\par` ran, which is the
/// `\baselineskip` TeX reads in `append_to_vlist` (§679) for every one of its
/// lines — the compiler's `parser::ParLeading`, mirrored here so the pipeline
/// builds against a `vendor/compiler` that predates the name.
pub type ParLeading = Option<flashtex_compiler::parser::FontSizeLevel>;

/// How a paragraph-shape environment began (see [`Block::Paragraph`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvOpen {
    /// `\begin{...}` was read in vertical mode (after a blank line, a
    /// heading, a rule or at the document start): `\partopsep` is added.
    pub vmode: bool,
    /// The environment's own `\@topsep`/`\@topsepadd`, when the package
    /// assigns them outright instead of letting `\@trivlist` derive them
    /// from `\topsep`, `\partopsep` and `\parskip` (see [`EnvSkips`]).
    /// `None` keeps the `\@trivlist` derivation, which is what `center`,
    /// `quote` and `abstract` get.
    pub skips: Option<EnvSkips>,
}

/// An environment that sets `\@topsep` (the opening `\addvspace` in
/// `\@item`) and `\@topsepadd` (the closing one in `\@endparenv`) itself,
/// so neither is the `\@trivlist` computation.
///
/// amsthm does this for every theorem-like environment: `\@thm` assigns
/// `\@topsep\thm@preskip` and `\@topsepadd\thm@postskip`, and
/// `\thm@space@setup` sets both of those to `\topsep`. That is why a
/// theorem never picks up `\partopsep` or `\parskip`, however it was
/// entered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvSkips {
    /// `\@topsep`: the skip before the environment's first line.
    pub open: crate::style::Skip,
    /// `\@topsepadd`: the skip after its last.
    pub close: crate::style::Skip,
}

#[derive(Debug)]
pub struct Doc {
    pub style: Stylesheet,
    pub blocks: Vec<Block>,
    pub diagnostics: Vec<Diagnostic>,
    /// Compiler constructs this pipeline has no exact block for and set
    /// approximately or dropped: `(code, source span, message)`, reported
    /// as warnings against the document paths by the caller.
    pub limitations: Vec<(&'static str, Span, String)>,
    /// `secnumdepth` in force (numbers in running heads).
    pub secnumdepth: u8,
    /// `\pagecolor` (compiler `Parsed::page_color`).
    pub page_color: Option<DeviceColor>,
    /// The default text colour (compiler `Parsed::default_color`).
    pub default_color: Option<DeviceColor>,
    /// The colour of every formula set in one, by `(document, start, end)`.
    pub math_colors: std::collections::HashMap<(usize, usize, usize), DeviceColor>,
    /// Indices of the blocks after a `\clearpage`/`\cleardoublepage` (see
    /// `clear_page_blocks`).
    pub page_starts: Vec<usize>,
    /// `\begin` commands the compiler reported as unimplemented that the
    /// pipeline sets itself (`abstract`): its diagnostic is dropped, the
    /// way `toc::superseded_commands` drops the contents-list ones.
    pub superseded: Vec<Span>,
}

/// Label values (`\ref`) and the pages they fell on in a previous layout
/// pass (`\pageref`); a key absent from `pages` renders as `??`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Labels {
    pub values: BTreeMap<String, String>,
    pub pages: BTreeMap<String, u32>,
    /// `\thepage` of every contents entry (`toc::key`) in the previous pass;
    /// kept apart from `pages` so the paragraph cache key is unaffected.
    pub toc_pages: BTreeMap<String, String>,
    /// Captioned floats: the `\listoffigures`/`\listoftables` entries.
    pub floats: Vec<crate::toc::FloatEntry>,
    /// Entry titles taken from source bytes, set as body text.
    pub entry_items: crate::toc::EntryItems,
    /// cleveref's label type per key (`section`, `equation`, `figure`, ...),
    /// from the compiler's `Inline::Label::kind`. Only `\cref` and friends
    /// read it; `\ref` needs the value alone.
    pub kinds: BTreeMap<String, String>,
    /// The document's cleveref naming options and `\crefname` overrides
    /// (`Parsed::cleveref`), so `\cref` can name the type it refers to.
    pub cleveref: flashtex_compiler::xref::CleverefConfig,
}

fn inlines_of(block: &CBlock) -> &[Inline] {
    match block {
        CBlock::Paragraph(i) => i,
        CBlock::ListItem { content, .. } | CBlock::Heading { content, .. } | CBlock::FigureCaption { content } | CBlock::Styled { content, .. } => content,
        // `\maketitle`'s parts are lowered to `Styled` paragraphs before
        // the block walk (`lower_blocks`); only the title is visible here.
        CBlock::TitleBlock { title, .. } => title,
        // `LetterBlock` holds `Vec<Vec<Inline>>`, not one flat slice, and
        // `lower_blocks` turns it into ordinary paragraphs before the block
        // walk reaches here.
        CBlock::VSpace { .. } | CBlock::Rule { .. } | CBlock::PageBreak | CBlock::Verbatim { .. } | CBlock::TableOfContents { .. } | CBlock::VFill | CBlock::LetterBlock { .. } => &[],
    }
}

/// Rewrites the compiler blocks the pipeline has no layout for (pin
/// `d416472a`: `Verbatim`, `TableOfContents`, `TitleBlock`, `VFill`) into
/// the plain blocks it does set, with one typed `unsupported_block`
/// limitation each, so their content is never dropped:
///
/// - `verbatim`/`lstlisting`: a `flushleft` paragraph, one `Text` per
///   source line (the compiler's tab-expanded text, `Mono` family) joined
///   by `\\`; the pipeline sets it in the body face, justified off, so
///   the lines keep their order but not Courier's fixed pitch.
/// - `\tableofcontents`: kept; `crate::toc` sets the list.
/// - `\maketitle`: three centred paragraphs (title at `\LARGE`, authors
///   and date at `\large`, the sizes `\@maketitle` declares) carried by the
///   compiler's `TextStyle::size`, which the pipeline already reads; the
///   exact `\vskip`s and `\thanks` are not.
/// - `\vfill`: dropped (the page builder has no stretchable vertical
///   glue), reported on the next block.
fn lower_blocks(texts: &[&str], blocks: &[(CBlock, ParLeading)], stash_titles: bool, parskip_pt: f64) -> (Vec<(CBlock, ParLeading)>, Vec<(&'static str, Span, String)>, Vec<StashedTitle>, Vec<Span>) {
    use flashtex_compiler::parser::{FontSizeLevel, LetterPart, ParagraphStyle, TextFamily, TextStyle as CStyle};
    let mut out: Vec<(CBlock, ParLeading)> = Vec::with_capacity(blocks.len());
    let mut limitations: Vec<(&'static str, Span, String)> = Vec::new();
    let mut titles: Vec<StashedTitle> = Vec::new();
    // The source spans of the `\opening`/`\closing` blocks lowered below.
    // Their `\raggedleft`/`\raggedright` is a *declaration*, not a
    // `flushright`/`flushleft` environment, so the `env_close` pass must not
    // give them `\@endparenv`'s `\@topsepadd` glue.
    let mut letter_spans: Vec<Span> = Vec::new();
    let mut pending_vfill = 0usize;
    let sized = |inlines: &[Inline], size: FontSizeLevel| -> Vec<Inline> {
        inlines
            .iter()
            .map(|i| match i {
                Inline::Text { text, span, style, space_before } => Inline::Text {
                    text: text.clone(),
                    span: *span,
                    style: CStyle {
                        size: Some(style.size.unwrap_or(size)),
                        ..*style
                    },
                    space_before: *space_before,
                },
                other => other.clone(),
            })
            .collect()
    };
    for (block, par_leading) in blocks {
        let par_leading = *par_leading;
        let first = match block {
            CBlock::Heading { number_span, .. } => Some(*number_span),
            CBlock::Verbatim { span, .. } | CBlock::TableOfContents { span } | CBlock::Rule { span } => Some(*span),
            _ => inlines_of(block).iter().map(inline_span).next(),
        };
        if pending_vfill > 0 {
            if let Some(at) = first {
                limitations.push((
                    "unsupported_block",
                    at,
                    format!("\\vfill ({pending_vfill} before this block) dropped: the page builder has no stretchable vertical glue"),
                ));
                pending_vfill = 0;
            }
        }
        match block {
            CBlock::Verbatim { lines, span: _ } => {
                let mut content: Vec<Inline> = Vec::with_capacity(lines.len() * 2);
                for (i, line) in lines.iter().enumerate() {
                    if i > 0 {
                        // The break owns the bytes between the lines so no
                        // interword space is read across it.
                        let prev = lines[i - 1].span;
                        content.push(line_break_inline(Span {
                            document: line.span.document,
                            start: prev.end.min(line.span.start),
                            end: line.span.start,
                        }));
                    }
                    content.push(Inline::Text {
                        text: line.text.clone(),
                        span: line.span,
                        style: CStyle {
                            family: TextFamily::Mono,
                            ..CStyle::default()
                        },
                        space_before: true,
                    });
                }
                // The text itself is now typewriter and literal (see
                // `style_intervals`/`TextStyle::literal`), so the old
                // "no monospaced face" limitation no longer applies, and an
                // `lstlisting` gets its limitation from `crate::listings` —
                // the pass that knows which keys it applied and which it did
                // not. A blanket "the listings keys are not applied" here
                // would now be false.
                out.push((
                    CBlock::Styled {
                        style: ParagraphStyle::FlushLeft,
                        content,
                        lists: Vec::new(),
                        line_break_before: None,
                    },
                    // No `leading_pt`: a listing's leading comes from its
                    // `basicstyle` through `crate::listings`, which sets the
                    // paragraph's whole `SizedPara` — size and that size's
                    // own `\baselineskip` together — rather than a leading
                    // on its own.
                    None,
                ));
            }
            // Set by `crate::toc` from the source command; the block stays
            // as the position a following `\clearpage` is measured from.
            CBlock::TableOfContents { .. } => out.push((block.clone(), par_leading)),
            CBlock::TitleBlock { title, authors, date } if stash_titles => titles.push((title.clone(), authors.clone(), date.clone())),
            CBlock::TitleBlock { title, authors, date } => {
                if let Some(at) = first {
                    limitations.push((
                        "unsupported_block",
                        at,
                        "\\maketitle set as centred paragraphs (title \\LARGE, authors/date \\large): article's exact \\@maketitle skips and \\thanks are not applied".to_string(),
                    ));
                }
                // `\@maketitle` (article.cls 172-186) puts a `\par` inside
                // the title's and the authors' groups but *not* the date's:
                // `{\LARGE \@title \par}`, `{\large ... \par}`, then
                // `{\large \@date}` and only then `\end{center}`. So the
                // date's `\par` runs after `}` has restored `\baselineskip`
                // and its lines are the body's 13.6 pt apart, not `\large`'s
                // 14 — measured on a wrapping `\date` under pdfTeX
                // 3.141592653-2.6-1.40.27: title 21.918 bp, date 13.549 bp.
                for (part, size, leading) in [
                    (Some(title), FontSizeLevel::Large3, Some(FontSizeLevel::Large3)),
                    (Some(authors), FontSizeLevel::Large1, Some(FontSizeLevel::Large1)),
                    (date.as_ref(), FontSizeLevel::Large1, None),
                ] {
                    let Some(part) = part else { continue };
                    if part.is_empty() {
                        continue;
                    }
                    out.push((
                        CBlock::Styled {
                            style: ParagraphStyle::Center,
                            content: sized(part, size),
                            lists: Vec::new(),
                            line_break_before: None,
                        },
                        leading,
                    ));
                }
            }
            // letter.cls's three positioned blocks. The pipeline has no
            // layout for any of them yet (`\raggedleft` boxes, a fixed
            // `\longindentation` offset and the class's own inter-block
            // skips), so each line is set as an ordinary paragraph in the
            // nearest alignment the pipeline does have and the geometry that
            // is lost is named once per block. Nothing is dropped: every
            // line of every part reaches the page in source order.
            // letter.cls's three positioned blocks (`\opening`'s return
            // address + date and recipient, `\closing`'s closing +
            // signature). The compiler resolved the class's own skips into
            // `gap_before_pt`/`gap_after_pt`/`extra_gap_after_pt`, all of
            // them multiples of letter.cls's `\parskip` (line 91,
            // `0.7em` = 7.66498pt at 11pt), so the pipeline's job here is to
            // spend them, not to recompute them.
            //
            // Each run of lines with no extra gap between them becomes **one**
            // paragraph whose lines are joined by `Inline::LineBreak` -- not
            // one paragraph per line. That distinction is the whole vertical
            // structure: a `\\` inside a paragraph costs `\baselineskip`,
            // while a new paragraph costs `\baselineskip` *plus* `\parskip`,
            // and letter.cls sets the address and the recipient as single
            // `\\`-separated paragraphs (a `tabular{l@{}}` and a
            // `{\raggedright ...\par}` group).
            //
            // `extra_gap_after_pt` (the `\\*[2\parskip]` between
            // `\fromaddress` and `\@date`) does split the paragraph, so the
            // `\vspace` that carries it has the following paragraph's own
            // `\parskip` taken out of it: the two together must add up to the
            // gap the class asked for, once.
            CBlock::LetterBlock { part, lines, extra_gap_after_pt, gap_before_pt, gap_after_pt, indent_pt, span } => {
                let para_style = match part {
                    // `\opening`'s `{\raggedleft ...}`. See the limitation
                    // below: this is the *nearest* style, not the class's.
                    LetterPart::ReturnAddress => Some(ParagraphStyle::FlushRight),
                    // `{\raggedright ...}` and `\parbox{...}{\raggedright ...}`
                    // are *declarations*, not `flushleft`/`flushright`
                    // environments, so they carry none of `\trivlist`'s
                    // `\topsep`/`\partopsep` glue. A `Styled` block here does
                    // carry it (9pt + 3pt at 11pt), which put the recipient
                    // and the closing 12pt too low. These blocks are short,
                    // unwrapped lines, where `\raggedright` and justification
                    // set identical text, so a plain paragraph is both the
                    // right vertical answer and the same horizontal one.
                    LetterPart::Recipient | LetterPart::Closing => None,
                };
                if *gap_before_pt != 0.0 {
                    out.push((CBlock::VSpace { pt: *gap_before_pt }, None));
                }
                let mut group: Vec<Inline> = Vec::new();
                let mut prev_end: Option<Span> = None;
                for (i, line) in lines.iter().enumerate() {
                    let first = line.iter().map(inline_span).next();
                    if !group.is_empty() {
                        // The break owns the bytes between the two lines, so
                        // no interword space is read across it (as the
                        // `verbatim` lowering above does).
                        if let (Some(prev), Some(at)) = (prev_end, first) {
                            group.push(Inline::LineBreak {
                                span: Span {
                                    document: at.document,
                                    start: prev.end.min(at.start),
                                    end: at.start,
                                },
                            });
                        }
                    }
                    group.extend(line.iter().cloned());
                    prev_end = line.iter().map(inline_span).last().or(prev_end);
                    let extra = extra_gap_after_pt.get(i).copied().unwrap_or(0.0);
                    if extra == 0.0 && i + 1 != lines.len() {
                        continue;
                    }
                    if !group.is_empty() {
                        let content = std::mem::take(&mut group);
                        // Keyed by the first inline's span, not the block's:
                        // `\address`'s text comes from the *preamble*, so it
                        // lies outside `\opening`'s own span entirely.
                        if let Some(at) = content.iter().map(inline_span).next() {
                            letter_spans.push(at);
                        }
                        out.push((
                            match para_style {
                                Some(style) => CBlock::Styled { style, content, lists: Vec::new(), line_break_before: None },
                                None => CBlock::Paragraph(content),
                            },
                            par_leading,
                        ));
                    }
                    if extra != 0.0 {
                        out.push((CBlock::VSpace { pt: extra - parskip_pt }, None));
                    }
                }
                if *gap_after_pt != 0.0 {
                    out.push((CBlock::VSpace { pt: *gap_after_pt }, None));
                }
                // What is still approximate is horizontal, and only
                // horizontal: the pipeline has no per-paragraph left offset
                // or measure, so neither `\longindentation` nor the
                // `\raggedleft` *box* can be expressed yet. Reported once per
                // block rather than silently produced.
                let mut lost: Vec<String> = Vec::new();
                if *indent_pt != 0.0 {
                    lost.push(format!(
                        "its {indent_pt} pt \\longindentation offset (it is set at the left margin instead)"
                    ));
                }
                if para_style.is_some() {
                    lost.push(
                        "the \\raggedleft box, whose lines share a *left* edge at the right margin \
                         (flushright aligns their right edges instead, so lines of unequal length differ)"
                            .to_string(),
                    );
                }
                if !lost.is_empty() {
                    limitations.push((
                        "unsupported_block",
                        *span,
                        format!(
                            "{}: the class's vertical skips are applied exactly; {} {} not",
                            match part {
                                LetterPart::ReturnAddress => "\\opening's return address and date",
                                LetterPart::Recipient => "\\opening's recipient",
                                LetterPart::Closing => "\\closing and signature",
                            },
                            lost.join(" and "),
                            if lost.len() == 1 { "is" } else { "are" },
                        ),
                    ));
                }
            }
            CBlock::VFill => pending_vfill += 1,
            other => out.push((other.clone(), par_leading)),
        }
    }
    if pending_vfill > 0 {
        let at = out.iter().rev().flat_map(|(b, _)| inlines_of(b).iter().map(inline_span).last()).next().unwrap_or(Span {
            document: DocumentId(0),
            start: 0,
            end: 0,
        });
        let _ = texts;
        limitations.push((
            "unsupported_block",
            at,
            format!("\\vfill ({pending_vfill} at the end of the document) dropped: the page builder has no stretchable vertical glue"),
        ));
    }
    (out, limitations, titles, letter_spans)
}

/// A compiler `TitleBlock`'s title, authors and date, set aside for the
/// `\maketitle` command it came from (see [`Block::Title`]).
type StashedTitle = (Vec<Inline>, Vec<Inline>, Option<Vec<Inline>>);

/// Splits the compiler's author inlines into `\and` groups and each group
/// into `tabular` rows at `\\`. The compiler joins `\and` groups with a
/// `LineBreak` spanning the whole `\author{...}` command (a `\\` carries
/// its own two bytes), so the two are told apart by the span's source.
fn author_groups(texts: &[&str], authors: &[Inline]) -> Vec<Vec<Inline>> {
    let mut groups: Vec<Vec<Inline>> = vec![Vec::new()];
    for inline in authors {
        if let Inline::LineBreak { span, .. } = inline {
            let at = texts.get(span.document.0).and_then(|t| t.get(span.start..)).unwrap_or("");
            if at.starts_with("\\author") {
                groups.push(Vec::new());
                continue;
            }
        }
        groups.last_mut().expect("at least one group").push(inline.clone());
    }
    groups.retain(|g| !g.is_empty());
    groups
}

/// A tabular cell's rows: `items` split at `\\`.
fn tabular_rows(items: Vec<Item>) -> Vec<Vec<Item>> {
    let mut rows = vec![Vec::new()];
    for item in items {
        match item {
            Item::LineBreak { .. } => rows.push(Vec::new()),
            other => rows.last_mut().expect("at least one row").push(other),
        }
    }
    // A cell starts with `\ignorespaces` and ends with `\unskip`.
    for row in &mut rows {
        while matches!(row.first(), Some(Item::Space { .. })) {
            row.remove(0);
        }
        while matches!(row.last(), Some(Item::Space { .. })) {
            row.pop();
        }
    }
    // `\\` at the end of the last row adds no row (`\@tabularcr` then
    // `\end{tabular}`: an empty last row of zero height is not set).
    if rows.len() > 1 && rows.last().is_some_and(|r| r.is_empty()) {
        rows.pop();
    }
    rows
}

impl Labels {
    /// The `\ref` values of every `\label` in the parse (known before layout).
    pub fn from_parsed(parsed: &Parsed) -> Labels {
        let mut values = BTreeMap::new();
        let mut kinds = BTreeMap::new();
        for inline in parsed.blocks.iter().flat_map(inlines_of) {
            if let Inline::Label { key, value, kind, .. } = inline {
                values.insert(key.clone(), value.clone());
                kinds.insert(key.clone(), kind.clone());
            }
        }
        Labels {
            values,
            kinds,
            cleveref: parsed.cleveref.clone(),
            ..Labels::default()
        }
    }

    /// Whether any `\pageref` in the parse needs a page number.
    pub fn needs_pages(parsed: &Parsed) -> bool {
        parsed
            .blocks
            .iter()
            .flat_map(inlines_of)
            .any(|i| {
                matches!(
                    i,
                    Inline::Reference { page: true, .. } | Inline::CleverReference { page: true, .. }
                )
            })
    }
}

/// Builds the block model from the compiler's parse result. `texts` is
/// indexed by `DocumentId`; `entry` is the root document's index.
pub fn adapt(texts: &[&str], entry: usize, parsed: &Parsed, options: &RenderOptions, labels: &Labels) -> Doc {
    adapt_cached(texts, entry, parsed, options, labels, None)
}

/// [`adapt`] with the cross-request cache: a compiler block whose inlines,
/// source bytes, enclosing style and label table match an earlier request
/// reuses its items (offsets relocated).
pub fn adapt_cached(
    texts: &[&str],
    entry: usize,
    parsed: &Parsed,
    options: &RenderOptions,
    labels: &Labels,
    cache: Option<&crate::incremental::RenderCache>,
) -> Doc {
    let _macro_defs = MacroDefsScope::enter(texts);
    let source = texts.get(entry).copied().unwrap_or("");
    let explicit_class = class_options(source);
    let class_options = explicit_class.clone().unwrap_or_else(|| options.default_class_options.clone());
    let size = class_size(&class_options);
    // LaTeX's own \parindent (size1x.clo) applies when the document declares a
    // class; body-only input keeps the compiler's implicit 0pt.
    let family = Stylesheet::family_for(&parsed.packages, t1_encoding(source));
    let setup = document_setup(
        source,
        explicit_class.is_some(),
        &class_options,
    );
    let mut resolved = flashtex_class_geometry::resolve(&setup);
    let assigned = apply_preamble_lengths(source, &mut resolved, size, family, setup.geometry.is_some());
    let mut style = Stylesheet::from_resolved(&resolved, family);
    // apply_preamble_lengths is the source of truth for `\parindent` /
    // `\parskip` (source order, including `\addtolength` and body
    // assignments). The older `setlength_in` scan only saw `\setlength`
    // and overwrote the accumulated value.
    if explicit_class.is_none() && !assigned.parindent {
        style.parindent_pt = options.default_parindent_pt;
    }
    if let Some(pt) = setlength(source, "columnseprule", size) {
        style.columnseprule_pt = pt;
    }
    style.microtype = microtype_setup(source);
    if document_sloppy(source) {
        // `\sloppy`: `\tolerance 9999 \emergencystretch 3em \hfuzz .5pt
        // \vfuzz\hfuzz` (latex.ltx), as the class does for two columns.
        style.tolerance = 9999.0;
        style.emergency_stretch_pt = 3.0 * style.body_size_pt;
    }
    // amsmath makes `\[` a plain `$$` (see [`ParaPart::Display::bracket`]).
    let amsmath = parsed.packages.iter().any(|p| p == "amsmath");
    // amsmath's `leqno`/`fleqn` options (global class options reach it too).
    // Without amsmath, `leqno.clo`/`fleqn.clo` build displays differently
    // (a zero-width `\eqno`, a `trivlist`), which is not modelled.
    let mut amsmath_cmex10 = false;
    if amsmath {
        let package = package_options(source, "amsmath").unwrap_or_default();
        let has = |name: &str| class_options.split(',').chain(package.split(',')).any(|o| o.trim() == name);
        style.leqno = has("leqno");
        style.fleqn = has("fleqn");
        // `\usepackage[cmex10]{amsmath}` keeps the kernel's `sfixed*cmex10`.
        amsmath_cmex10 = package.split(',').any(|o| o.trim() == "cmex10");
    }
    style.cmex_designs = crate::style::cmex_designs(&parsed.packages, amsmath_cmex10);
    #[cfg(feature = "amsmath-inline")]
    let mathtools = parsed.packages.iter().any(|p| p == "mathtools");
    // `\parskip` from apply_preamble_lengths: `\addtolength` keeps class
    // stretch; a `\setlength` with plus/minus keeps those; a plain value
    // is a fixed skip.
    if assigned.parskip {
        let g = resolved.params.parskip;
        style.parskip = crate::style::Skip::new(
            crate::style::frame_pt(g.natural),
            crate::style::frame_pt(g.stretch),
            crate::style::frame_pt(g.shrink),
        );
    }
    // `\c@secnumdepth`. LaTeX has exactly one such counter and `\@sect` reads
    // it twice: `\ifnum #2>\c@secnumdepth` suppresses the printed number, and
    // the same test suppresses the `\numberline` written to the contents
    // list. Its value is the class's own (`article.cls` line 255
    // `\setcounter{secnumdepth}{3}`; `report.cls`/`book.cls` 2) unless the
    // document sets the counter itself.
    //
    // This used to be a flat `options.default_secnumdepth` (2), so every
    // `\subsubsection` in an `article` came out unnumbered while the contents
    // list — which already derived the class default below — wrote `1.1.1`
    // for the same heading. `\documentclass`-less input (the visual-oracle
    // harness and the Mac app send body-only documents, and `resolve` hands
    // those article geometry regardless) keeps the caller's default.
    let secnumdepth = counter(source, "secnumdepth").unwrap_or_else(|| {
        match (&style.class_geometry, explicit_class.is_some()) {
            (Some(d), true) => d.secnumdepth.clamp(0, i32::from(u8::MAX)) as u8,
            _ => options.default_secnumdepth,
        }
    });
    style.nfss = crate::nfss::Scheme::for_document(&parsed.packages, t1_encoding(source));
    let styles: Vec<Styles> = texts.iter().map(|t| Styles::new(t, style_intervals(t), style.nfss)).collect();
    let labels_fp = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for (k, v) in &labels.values {
            k.hash(&mut h);
            v.hash(&mut h);
        }
        for (k, v) in &labels.pages {
            k.hash(&mut h);
            v.hash(&mut h);
        }
        h.finish()
    };
    let items_for = |inlines: &[Inline], heading: bool| -> Vec<Item> { items_cached(texts, inlines, &styles, labels, labels_fp, size, heading, false, cache) };
    let items_for_weighted = |inlines: &[Inline], compiler_weight: bool| -> Vec<Item> { items_cached(texts, inlines, &styles, labels, labels_fp, size, false, compiler_weight, cache) };
    let mut blocks = Vec::new();
    // Page-style, mark, `\chapter` and `\noindent` commands in the entry
    // document's body, read from the source: the compiler accepts the first
    // two as no-ops, sets the arguments of marks and `\chapter` as body text
    // (dropped here) and ignores `\noindent`.
    let entry_doc = DocumentId(entry);
    let has_chapters = style.class_geometry.as_ref().is_some_and(|d| d.chapter.is_some());
    let book = style.class_geometry.as_ref().is_some_and(|d| d.options.kind == flashtex_class_geometry::ClassKind::Book);
    // article's `\maketitle` (no `titlepage`) issues `\thispagestyle{plain}`.
    let maketitle_plain = style.class_geometry.as_ref().is_some_and(|d| !d.options.titlepage);
    let commands = body_commands(source, has_chapters, book);
    // Every compiler `TitleBlock` is laid out at its `\maketitle` command
    // (the entry document's, in order) when the two correspond one to one;
    // otherwise (a `\maketitle` the compiler rejected, or one in an
    // `\input` file) they stay centred paragraphs.
    let maketitles = commands.iter().filter(|c| matches!(c.kind, BodyKind::MakeTitle)).count();
    let title_blocks = parsed.blocks.iter().filter(|b| matches!(b, CBlock::TitleBlock { .. })).count();
    let stash_titles = style.class_geometry.is_some() && maketitles == title_blocks && maketitles > 0;
    // `Parsed::block_par_leading` is one entry per block, in `blocks` order
    // (the compiler pushes both from the same place). Without the
    // `par-leading` feature the pinned `vendor/compiler` has no such field
    // and every paragraph keeps the body's `\baselineskip`, which is what
    // the pipeline did before this existed.
    #[cfg(feature = "par-leading")]
    let leadings: Vec<ParLeading> = parsed.block_par_leading.clone();
    #[cfg(not(feature = "par-leading"))]
    let leadings: Vec<ParLeading> = vec![None; parsed.blocks.len()];
    debug_assert_eq!(leadings.len(), parsed.blocks.len());
    let paired: Vec<(CBlock, ParLeading)> = parsed
        .blocks
        .iter()
        .cloned()
        .zip(leadings.into_iter().chain(std::iter::repeat(None)))
        .collect();
    let (mut lowered, mut limitations, stashed, letter_spans) = lower_blocks(texts, &paired, stash_titles, style.parskip.natural);
    let mut stashed = stashed.into_iter();
    let title_of = |(title, authors, date): StashedTitle, span: Span| Block::Title {
        title: items_for(&title, false),
        authors: author_groups(texts, &authors).iter().map(|g| tabular_rows(items_for(g, false))).collect(),
        date: date.map(|d| items_for(&d, false)),
        span,
    };
    // book.cls `\if@mainmatter` (true until `\frontmatter`).
    let mut mainmatter = true;
    strip_command_text(&mut lowered, entry_doc, &commands);
    let mut next_command = 0usize;
    let mut noindent_at: Option<usize> = None;
    // The `\input`/`\include`d document whose units are being laid out.
    let mut input_doc: Option<DocumentId> = None;
    // report/book: `\thesection` is `\thechapter.\arabic{section}`.
    let (mut chapter_no, mut section_nos) = (0u32, [0u32; 3]);
    // `\appendix`: `\thesection` (article) or `\thechapter` (report/book)
    // becomes `\@Alph`; `chapter_label` is `\thechapter`.
    let mut appendix = false;
    let mut chapter_label = String::new();
    let mut part_no = 0u32;
    // Contents lists (`crate::toc`): every `\contentsline` record, where
    // each list stands, and the label keys of records whose page is that of
    // the next block. Nothing is collected without a list.
    let toc_active = commands.iter().any(|c| matches!(c.kind, BodyKind::ContentsList(_)));
    let toc_settings = crate::toc::Settings::read(source, has_chapters);
    // `\@sect` writes `\numberline` up to the same `\c@secnumdepth` that
    // decides the printed number; the two are one counter, resolved above.
    let toc_secnumdepth = secnumdepth;
    let mut toc_records: Vec<crate::toc::Record> = Vec::new();
    let mut toc_lists: Vec<(usize, crate::toc::ListKind, Span, bool)> = Vec::new();
    let mut toc_pending: Vec<String> = Vec::new();
    let mut chapter_starts: Vec<(usize, String)> = Vec::new();
    let mut after_heading = false;
    // The block that is, so far, the last one inside an open theorem-like
    // environment. `\endtrivlist`'s `\@endparenv` puts `\@topsepadd` after
    // the *last* paragraph of the environment, and only the next unit says
    // whether there is one: a block still inside the same environment
    // continues the run, anything else closes it. The flag is set on the
    // block itself, so the `toc_lists` splice below cannot shift it.
    let mut open_theorem: Option<usize> = None;
    // Last source span of the previous paragraph block (None after a
    // heading or rule), for rejoining a display with its paragraph.
    let mut prev_para_end: Option<Span> = None;
    for unit in split_at_page_breaks(texts, &lowered, size, &style) {
        // Does this unit continue the theorem-like environment that the
        // previous block left open? Only a paragraph inside it that is not
        // itself a fresh `\item` does.
        let continues_theorem = matches!(
            unit.kind,
            UnitKind::Paragraph { in_theorem: true, theorem_item: false, .. }
        );
        if !continues_theorem {
            if let Some(at) = open_theorem.take() {
                if let Some(Block::Paragraph { env_close, .. }) = blocks.get_mut(at) {
                    *env_close = true;
                }
            }
        }
        let mut eject_before = unit.eject_before;
        let vspace_before = unit.vspace_before;
        limitations.extend(unit.limitations);
        let unit_start = match &unit.kind {
            UnitKind::Heading { number_span, .. } => Some(*number_span),
            UnitKind::Paragraph { inlines, .. } => inlines.iter().map(inline_span).next(),
            UnitKind::Rule { span } => Some(*span),
            UnitKind::Picture { document, picture, .. } => Some(Span::in_document(*document, picture.start, picture.end)),
        };
        // Entry-document commands are laid out before the first unit that
        // follows them in the entry source. A unit of an `\input`/`\include`d
        // document follows the `\input` command that read it, so everything
        // before that command (a `\maketitle` ahead of `\input{intro}`)
        // precedes the file's first unit; later units of the same file flush
        // nothing until the entry document resumes.
        let flush_before = match unit_start {
            Some(at) if at.document == entry_doc => {
                input_doc = None;
                Some(at.start)
            }
            Some(at) if input_doc != Some(at.document) => {
                input_doc = Some(at.document);
                commands[next_command..].iter().find(|c| matches!(c.kind, BodyKind::Input)).map(|c| c.start)
            }
            _ => None,
        };
        if let Some(at) = flush_before {
            while let Some(cmd) = commands.get(next_command).filter(|c| c.start < at) {
                next_command += 1;
                match &cmd.kind {
                    BodyKind::Input => {}
                    BodyKind::Event(event) => blocks.push(Block::Chrome {
                        event: event.clone(),
                        span: Span::in_document(entry_doc, cmd.start, cmd.end),
                    }),
                    BodyKind::NoIndent => noindent_at = Some(cmd.end),
                    BodyKind::Chapter { starred, title } => {
                        // `\@chapter`: `\refstepcounter{chapter}` only
                        // `\if@mainmatter` (book.cls line 356).
                        let number = (!*starred && mainmatter).then(|| {
                            chapter_no += 1;
                            section_nos = [0; 3];
                            chapter_label = if appendix {
                                flashtex_class_geometry::Numbering::UpperAlph.format(i64::from(chapter_no))
                            } else {
                                chapter_no.to_string()
                            };
                            chapter_starts.push((cmd.start, chapter_label.clone()));
                            chapter_label.clone()
                        });
                        let span = Span::in_document(entry_doc, cmd.start, cmd.end);
                        let mut items = words_from_source(source, entry_doc, title.0, title.1);
                        if toc_active {
                            if let Some(n) = &number {
                                // report.cls `\@chapter`: `\addcontentsline{toc}{chapter}{\protect\numberline{\thechapter}#1}`.
                                let key = crate::toc::key(toc_records.len());
                                toc_records.push(crate::toc::Record {
                                    list: crate::toc::ListKind::Toc,
                                    level: 0,
                                    number: Some((n.clone(), span)),
                                    title: labels.entry_items.get(entry_doc, title.0, title.1).unwrap_or_else(|| items.clone()),
                                    key: key.clone(),
                                });
                                toc_pending.push(key);
                            }
                            items.splice(0..0, toc_pending.drain(..).map(|key| Item::Label { key }));
                        }
                        blocks.push(Block::Chapter {
                            number,
                            appendix,
                            items,
                            title: plain_text(&source[title.0..title.1]),
                            span,
                            mark: !*starred,
                        });
                        after_heading = true;
                        prev_para_end = None;
                    }
                    BodyKind::MakeTitle => {
                        if let Some(t) = stashed.next() {
                            let span = Span::in_document(entry_doc, cmd.start, cmd.end);
                            // `\@maketitle` is followed by `\thispagestyle{plain}`;
                            // the `titlepage` form sets `empty` on its own page.
                            if maketitle_plain {
                                blocks.push(Block::Chrome {
                                    event: ChromeEvent::ThisPageStyle(PageStyle::Plain),
                                    span,
                                });
                            }
                            blocks.push(title_of(t, span));
                            after_heading = false;
                            prev_para_end = None;
                        } else if maketitle_plain {
                            blocks.push(Block::Chrome {
                                event: ChromeEvent::ThisPageStyle(PageStyle::Plain),
                                span: Span::in_document(entry_doc, cmd.start, cmd.end),
                            });
                        }
                    }
                    BodyKind::Matter(matter) => {
                        let span = Span::in_document(entry_doc, cmd.start, cmd.end);
                        let openright = style.class_geometry.as_ref().is_some_and(|d| d.options.openright);
                        let (double, numbering, main) = match matter {
                            Matter::Front => (true, Some(flashtex_class_geometry::Numbering::Roman), false),
                            Matter::Main => (true, Some(flashtex_class_geometry::Numbering::Arabic), true),
                            Matter::Back => (openright, None, false),
                        };
                        mainmatter = main;
                        blocks.push(Block::ClearPage { double, span });
                        if let Some(n) = numbering {
                            blocks.push(Block::Chrome {
                                event: ChromeEvent::PageNumbering(n),
                                span,
                            });
                        }
                        prev_para_end = None;
                    }
                    BodyKind::ContentsList(kind) => {
                        // `\newpage` (etc.) right before the command breaks
                        // before the list's heading.
                        let before = source[..cmd.start].trim_end();
                        let eject = ["\\newpage", "\\clearpage", "\\cleardoublepage", "\\pagebreak"].iter().any(|c| before.ends_with(c));
                        toc_lists.push((blocks.len(), *kind, Span::in_document(entry_doc, cmd.start, cmd.end), eject));
                    }
                    BodyKind::AddContentsLine { list, level, text } => {
                        if let (true, Some(level)) = (toc_active, crate::toc::level_of(level)) {
                            let (number, title) = crate::toc::contentsline_text(source, entry_doc, text.0, text.1, &labels.entry_items);
                            let key = crate::toc::key(toc_records.len());
                            toc_records.push(crate::toc::Record {
                                list: *list,
                                level,
                                number,
                                title,
                                key: key.clone(),
                            });
                            // The write lands on the page of the heading just
                            // set, or of the next block.
                            match blocks.last_mut() {
                                Some(Block::Heading { items, .. } | Block::Chapter { items, .. } | Block::Part { items, .. }) if after_heading => items.push(Item::Label { key }),
                                _ => toc_pending.push(key),
                            }
                        }
                    }
                    BodyKind::Appendix => {
                        // article.cls/report.cls `\appendix`: the section
                        // (and chapter) counters restart.
                        appendix = true;
                        chapter_no = 0;
                        section_nos = [0; 3];
                    }
                    BodyKind::Part { starred, short, title } => {
                        // `\@part`: `\refstepcounter{part}` (`\thepart` is
                        // `\@Roman\c@part`) and `\addcontentsline{toc}{part}
                        // {\thepart\hspace{1em}#1}`; `\@spart` writes nothing.
                        let number = (!*starred).then(|| {
                            part_no += 1;
                            flashtex_class_geometry::Numbering::UpperRoman.format(i64::from(part_no))
                        });
                        let span = Span::in_document(entry_doc, cmd.start, cmd.end);
                        let mut items = words_from_source(source, entry_doc, title.0, title.1);
                        if toc_active {
                            if let Some(n) = &number {
                                let (s, e) = short.unwrap_or(*title);
                                let key = crate::toc::key(toc_records.len());
                                toc_records.push(crate::toc::Record {
                                    list: crate::toc::ListKind::Toc,
                                    level: -1,
                                    number: Some((n.clone(), span)),
                                    title: labels.entry_items.get(entry_doc, s, e).unwrap_or_else(|| words_from_source(source, entry_doc, s, e)),
                                    key: key.clone(),
                                });
                                toc_pending.push(key);
                            }
                            items.splice(0..0, toc_pending.drain(..).map(|key| Item::Label { key }));
                        }
                        // The compiler attaches a `\clearpage` before `\part`
                        // to the next unit; it belongs to the part.
                        let before = source[..cmd.start].trim_end();
                        let clear_before = ["\\clearpage", "\\cleardoublepage"].iter().any(|c| before.ends_with(c));
                        let part_eject = clear_before || ["\\newpage", "\\pagebreak"].iter().any(|c| before.ends_with(c));
                        if part_eject {
                            eject_before = false;
                        }
                        blocks.push(Block::Part {
                            number,
                            items,
                            span,
                            eject_before: part_eject,
                            clear_before,
                        });
                        after_heading = true;
                        prev_para_end = None;
                    }
                }
            }
            // The `\input` command that read this unit's document is spent.
            if input_doc.is_some() && commands.get(next_command).is_some_and(|c| c.start == at && matches!(c.kind, BodyKind::Input)) {
                next_command += 1;
            }
        }
        match unit.kind {
            UnitKind::Heading {
                level,
                number,
                number_span,
                content,
            } => {
                let number: String = if (has_chapters || appendix) && !number.is_empty() && (1..=3).contains(&level) {
                    let l = usize::from(level) - 1;
                    section_nos[l] += 1;
                    for n in &mut section_nos[l + 1..] {
                        *n = 0;
                    }
                    let mut parts: Vec<String> = section_nos[..=l].iter().map(|n| n.to_string()).collect();
                    if has_chapters {
                        parts.insert(0, if chapter_no == 0 { "0".to_string() } else { chapter_label.clone() });
                    } else {
                        // article.cls `\appendix`: `\thesection` is `\@Alph\c@section`.
                        parts[0] = flashtex_class_geometry::Numbering::UpperAlph.format(i64::from(section_nos[0]));
                    }
                    parts.join(".")
                } else {
                    number.to_string()
                };
                let title = match (content.first(), content.last()) {
                    (Some(a), Some(b)) => {
                        let (a, b) = (inline_span(a), inline_span(b));
                        texts.get(a.document.0).and_then(|t| t.get(a.start..b.end)).map(plain_text).unwrap_or_default()
                    }
                    _ => String::new(),
                };
                // LaTeX `\@seccntformat`: the counter, then `\quad`, then the
                // title; the number's bytes are the `\section` command's.
                let mut items = Vec::new();
                if !number.is_empty() && level <= secnumdepth {
                    let chars = number
                        .chars()
                        .map(|_| CharSrc {
                            document: number_span.document,
                            start: number_span.start,
                            end: number_span.end,
                        })
                        .collect();
                    push_segment(&mut items, number.to_string(), chars, TextStyle::default());
                    items.push(Item::Quad { em: 1.0 });
                }
                let content_items = items_for(content, true);
                if toc_active {
                    // `\@sect`: `\addcontentsline{toc}{<level>}{\numberline{<number>}<title>}`
                    // for an unstarred heading (no `\numberline` past `secnumdepth`).
                    if !number.is_empty() {
                        let key = crate::toc::key(toc_records.len());
                        toc_records.push(crate::toc::Record {
                            list: crate::toc::ListKind::Toc,
                            level: level as i8,
                            number: (level <= toc_secnumdepth).then(|| (number.clone(), number_span)),
                            title: content_items.iter().filter(|i| !matches!(i, Item::Label { .. })).cloned().collect(),
                            key: key.clone(),
                        });
                        toc_pending.push(key);
                    }
                    items.splice(0..0, toc_pending.drain(..).map(|key| Item::Label { key }));
                }
                items.extend(content_items);
                // report.cls/book.cls open `thebibliography` with
                // `\chapter*{\bibname\@mkboth{...}}`, not article.cls's
                // `\section*{\refname}`: a `\clearpage`, the `\@makeschapterhead`
                // drop and the name `Bibliography`. The compiler synthesises one
                // unnumbered level-1 heading reading `References` for every class
                // (`parser.rs`, `thebibliography`), so a `report` bibliography was
                // set in the flow of the preceding page under the wrong name —
                // which is why `fixtures/real-world/thesis-chapter` came out 4
                // pages against pdflatex's 5.
                if has_chapters && bibliography_heading(texts, level, &number, number_span) {
                    // Keep whatever `\label`s the contents-list machinery put
                    // in front of the title; replace the compiler's
                    // `References` text with `\bibname`.
                    let mut head: Vec<Item> = items.into_iter().take_while(|i| matches!(i, Item::Label { .. })).collect();
                    head.extend(command_words(BIBNAME, number_span));
                    blocks.push(Block::Chapter {
                        number: None,
                        appendix,
                        items: head,
                        title: BIBNAME.to_string(),
                        span: number_span,
                        // `\chapter*` issues no `\chaptermark`; `thebibliography`'s
                        // own `\@mkboth` sets both marks, which only a `headings`
                        // page style would show (report/book default to `plain`).
                        mark: false,
                    });
                } else {
                    blocks.push(Block::Heading {
                        level,
                        items,
                        eject_before,
                        vspace_before,
                        number,
                        title,
                        span: number_span,
                    });
                }
                after_heading = true;
                prev_para_end = None;
            }
            UnitKind::Rule { span } => {
                blocks.push(Block::Rule {
                    span,
                    eject_before,
                    vspace_before,
                });
                after_heading = false;
                prev_para_end = None;
            }
            UnitKind::Picture { document, picture, centered } => {
                blocks.push(Block::Picture {
                    document,
                    picture,
                    centered,
                    eject_before,
                    vspace_before,
                });
                after_heading = false;
            }
            UnitKind::Paragraph {
                inlines,
                caption,
                styled,
                env_open,
                after_env,
                theorem_item,
                in_theorem,
                list,
                run_in,
                par_leading,
            } => {
                for inline in inlines {
                    unsupported_inlines(inline, &mut limitations);
                }
                // amsthm sets the head bold (italic for `remark`/`proof`)
                // and, for the `plain` style, the body italic. None of that
                // is in the source bytes at the head's span (which is the
                // `\begin` command), so the weights come from the compiler's
                // own scoping inside a theorem-like environment.
                let mut items = items_for_weighted(inlines, in_theorem);
                // `\paragraph`/`\subparagraph`: `{\normalfont\normalsize
                // \bfseries <title>}` then `\hskip 1em`, run into this
                // paragraph's first line. The compiler set the title as
                // plain body text, so the weight and the `em` are applied
                // here, over exactly the items whose bytes are the title's.
                if let Some(run_in) = run_in {
                    let h = style.heading(run_in.level);
                    apply_run_in_heading(&mut items, &run_in, h.run_in_after_em.unwrap_or(1.0), h.bold);
                }
                items.splice(0..0, toc_pending.drain(..).map(|key| Item::Label { key }));
                let mut parts = Vec::new();
                let mut current = Vec::new();
                for item in items {
                    match item {
                        Item::Math { list, span } if is_display(inlines, span) => {
                            if !current.is_empty() {
                                parts.push(ParaPart::Lines(std::mem::take(&mut current)));
                            }
                            // amsmath rows: the environment's rows become one
                            // alignment; `\tag{..}` replaces a row's number.
                            if let Some((rows_span, row)) = math_row_of(inlines, span) {
                                let rows_rest = texts.get(rows_span.document.0).and_then(|t| t.get(rows_span.start..)).unwrap_or("");
                                let mut tag = None;
                                let cells: Vec<MathList> = row.cells.iter().map(|c| strip_tag(texts, c, &mut tag)).collect();
                                let number = match tag {
                                    Some(t) => Some((t, row.span)),
                                    None => row.number.clone().map(|n| (format!("({n})"), row.span)),
                                };
                                #[cfg(feature = "amsmath-inline")]
                                let intertext = row
                                    .intertext
                                    .iter()
                                    .map(|t| IntertextPart {
                                        items: items_for(&t.content, false),
                                        short: t.short,
                                        mathtools,
                                        span: t.span,
                                    })
                                    .collect();
                                #[cfg(not(feature = "amsmath-inline"))]
                                let intertext = Vec::new();
                                let part = RowPart { cells, number, span: row.span, intertext };
                                match parts.last_mut() {
                                    Some(ParaPart::Rows { span: s, rows, .. }) if *s == rows_span => rows.push(part),
                                    _ => parts.push(ParaPart::Rows {
                                        env: RowsEnv::at(rows_rest),
                                        rows: vec![part],
                                        span: rows_span,
                                        bracket: false,
                                    }),
                                }
                                continue;
                            }
                            // The compiler counts every closed display; LaTeX
                            // numbers only the `equation` environment (or a
                            // `\tag` in any display).
                            let rest = texts.get(span.document.0).and_then(|t| t.get(span.start..)).unwrap_or("");
                            let mut tag = None;
                            let list = strip_tag(texts, &list, &mut tag);
                            let (list, eqno) = strip_eqno(texts, list, span);
                            let number = match (tag, eqno) {
                                (Some(t), _) => Some((t, span)),
                                (None, Some(n)) => Some(n),
                                (None, None) => display_number(inlines, span)
                                    .filter(|_| rest.starts_with("\\begin{equation}"))
                                    .map(|(n, s)| (format!("({n})"), s)),
                            };
                            let bracket = !amsmath && (rest.starts_with("\\[") || rest.starts_with("\\begin{displaymath}"));
                            parts.push(ParaPart::Display {
                                list,
                                span,
                                number,
                                bracket,
                            });
                        }
                        other => current.push(other),
                    }
                }
                if !current.is_empty() {
                    parts.push(ParaPart::Lines(current));
                }
                let only_labels = parts
                    .iter()
                    .all(|p| matches!(p, ParaPart::Lines(items) if items.iter().all(|i| matches!(i, Item::Label { .. }))));
                if parts.is_empty() {
                    continue;
                }
                // `\longtable` begins with `\par` and `\endlongtable`
                // ends with one, so the compiler always gives it a
                // paragraph of its own: it becomes a block the page
                // builder can break inside rather than a box on a line.
                if let Some(table) = lone_longtable(&mut parts) {
                    let src = texts.get(table.span.document.0).copied().unwrap_or("");
                    blocks.push(Block::LongTable {
                        lengths: LongtableLengths::read(|name| setlength(src, name, size)),
                        labels: Vec::new(),
                        table,
                        eject_before,
                        vspace_before,
                    });
                    prev_para_end = inlines.iter().map(inline_span).last();
                    after_heading = false;
                    continue;
                }
                // A display environment inside a paragraph (no blank line or
                // `\par` around it) continues that paragraph, as in LaTeX: the
                // compiler flushes its paragraph at `\begin{equation}`/
                // `\begin{align}` and again at `\end`, so the pieces are
                // rejoined here (short display skips, no second `\parskip`, no
                // empty opener line, no indent after the display).
                let first_span = inlines.iter().map(inline_span).next();
                let starts_display = matches!(parts.first(), Some(ParaPart::Display { .. } | ParaPart::Rows { .. }));
                if let (Some(Block::Paragraph { parts: prev_parts, style: prev_style, list: prev_list, .. }), Some(f), Some(p)) = (blocks.last_mut(), first_span, prev_para_end) {
                    // Labels only, or a `label_line` (labels then one space).
                    let label_only = |p: &ParaPart| matches!(p, ParaPart::Lines(items) if items.iter().all(|i| matches!(i, Item::Label { .. } | Item::Space { .. })));
                    // A display's `\label` is flushed after it as a part of
                    // labels only.
                    let prev_ends_display = matches!(prev_parts.iter().rev().find(|p| !label_only(p)), Some(ParaPart::Display { .. } | ParaPart::Rows { .. }));
                    // Inside `quote` and friends or a list item, the same
                    // environment (and item) continues.
                    let same_list = match (list.as_ref(), prev_list.as_ref()) {
                        (None, None) => true,
                        (Some(g), Some(pg)) => g.label.is_none() && g.level == pg.level && g.margins == pg.margins,
                        _ => false,
                    };
                    let same_flow = !eject_before
                        && vspace_before == 0.0
                        && unit.addvspace_before == 0.0
                        && styled.unwrap_or_default() == *prev_style
                        && same_list
                        && !caption
                        && env_open.is_none();
                    if same_flow && (starts_display || prev_ends_display) && gap_continues(texts, p, f) {
                        if only_labels {
                            // A `\label` outside the display, still in the
                            // paragraph's horizontal mode (`\begin{subequations}
                            // \label{..}`): a whatsit TeX sets on a line of its
                            // own when a display or the paragraph end follows.
                            // Kept as a `label_line` (labels, then one space)
                            // that text following it absorbs below.
                            let mut items: Vec<Item> = parts
                                .into_iter()
                                .flat_map(|p| match p {
                                    ParaPart::Lines(items) => items,
                                    _ => Vec::new(),
                                })
                                .collect();
                            items.push(Item::Space { style: TextStyle::default(), factor: 1000, no_break: false });
                            prev_parts.push(ParaPart::Lines(items));
                            prev_para_end = inlines.iter().map(inline_span).last().or(prev_para_end);
                            continue;
                        }
                        if matches!(parts.first(), Some(ParaPart::Lines(_))) && prev_parts.last().is_some_and(label_only) {
                            if let (Some(ParaPart::Lines(labels)), Some(ParaPart::Lines(head))) = (prev_parts.pop(), parts.first_mut()) {
                                let at = usize::from(matches!(head.first(), Some(Item::Space { .. })));
                                head.splice(at..at, labels.into_iter().filter(|i| matches!(i, Item::Label { .. })));
                            }
                        }
                        prev_parts.extend(parts);
                        prev_para_end = inlines.iter().map(inline_span).last().or(prev_para_end);
                        after_heading = false;
                        continue;
                    }
                }
                if only_labels {
                    continue;
                }
                prev_para_end = inlines.iter().map(inline_span).last();
                // `\noindent` right before the paragraph's first material.
                let noindent = noindent_at.take().is_some_and(|end| {
                    first_span.is_some_and(|f| f.document == entry_doc && source.get(end..f.start).is_some_and(|gap| gap.trim().is_empty()))
                });
                // `\centering` sets `\parindent 0pt`; a list item's first
                // paragraph carries no indent and `\list` sets
                // `\parindent\listparindent` (0pt in article) for the
                // ones after it, `quote` likewise.
                // `\@xsect`'s run-in branch discards the `\parindent` box and
                // sets `\hskip #3` instead: 0 for `\paragraph`, `\parindent`
                // for `\subparagraph` (article.cls `indent_parindent`).
                let run_in_indent = run_in.map(|r| flashtex_document_style::section_spec(r.level).is_some_and(|s| s.indent_parindent));
                // `\@startsection`'s `\addpenalty\@secpenalty \addvspace{#4}`
                // above the head: `3.25ex \@plus1ex \@minus.2ex` of the body
                // font for both levels. `\addvspace` keeps whichever of the
                // new skip and `\lastskip` is larger (`\@xaddvskip`), so
                // when the head follows something that already contributed
                // one — `\endtrivlist`'s `\addvspace\@topsepadd` after a
                // list, which is how every `\paragraph{Solution.}` in
                // `fixtures/real-world/ps-calculus` is reached — the two do
                // not add up.
                // `\@xaddvskip` keeps whichever glue is the larger, whole:
                // its stretch and shrink come with it, and #405 made these
                // skips real glue rather than rigid kerns.
                let run_in_skip = run_in.map(|r| style.heading(r.level).before);
                let (addvspace_before, addvspace_flex) = match run_in_skip {
                    Some(s) if s.natural > unit.addvspace_before => (s.natural, (s.stretch, s.shrink)),
                    _ => (unit.addvspace_before, unit.addvspace_flex),
                };
                blocks.push(Block::Paragraph {
                    parts,
                    indent: match run_in_indent {
                        Some(indent) => indent,
                        None => !after_heading && !caption && styled.is_none() && !after_env && !theorem_item && list.is_none() && !noindent,
                    },
                    style: styled.unwrap_or_default(),
                    env_open,
                    env_close: false,
                    eject_before,
                    vspace_before,
                    addvspace_before,
                    addvspace_flex,
                    vspace_flex: unit.vspace_flex,
                    endlist_adjust: unit.endlist_adjust,
                    list,
                    sized: None,
                    leading_pt: par_leading_pt(par_leading, style.base),
                });
                if in_theorem {
                    open_theorem = Some(blocks.len() - 1);
                }
                after_heading = false;
            }
        }
    }
    // A theorem-like environment that runs to the end of the document still
    // closes: `\end{document}` is not what ended it.
    if let Some(at) = open_theorem.take() {
        if let Some(Block::Paragraph { env_close, .. }) = blocks.get_mut(at) {
            *env_close = true;
        }
    }
    // The contents lists, now that every record is known.
    for (at, kind, span, eject) in toc_lists.into_iter().rev() {
        let list = crate::toc::list_blocks(kind, span, eject, &toc_settings, &toc_records, labels, &chapter_starts);
        blocks.splice(at..at, list);
    }
    // `abstract`: the compiler sets its body as plain text, so the class's
    // own shape (the centred `\small\bfseries` head and the `\small`
    // `quotation`) is read from the source bytes here, before the
    // `env_close` pass below derives the closing skips from the styles.
    let mut superseded = crate::abstractenv::apply(texts, &mut blocks, &style);
    // `\end{...}`: the last paragraph of a run of same-style paragraphs
    // closes the environment (two adjacent environments of one style are
    // read as one; the compiler does not mark the boundary).
    let styles: Vec<ParaStyle> = blocks
        .iter()
        .map(|b| match b {
            Block::Paragraph { style, .. } => *style,
            _ => ParaStyle::Plain,
        })
        .collect();
    let from_letter = |parts: &[ParaPart]| {
        parts
            .iter()
            .find_map(|p| match p {
                ParaPart::Lines(items) => items.iter().find_map(|i| match i {
                    Item::Word(w) => Some(w.span()),
                    Item::Math { span, .. } => Some(*span),
                    _ => None,
                }),
                _ => None,
            })
            .is_some_and(|at| {
                letter_spans
                    .iter()
                    .any(|s| s.document == at.document && s.start == at.start)
            })
    };
    for (i, block) in blocks.iter_mut().enumerate() {
        if let Block::Paragraph { style, env_close, parts, .. } = block {
            // `ParaStyle::Plain` includes every theorem-like environment,
            // whose `env_close` the unit loop above has already set from the
            // `\end{<theorem>}` that actually closed it.
            //
            // `letter.cls` positions `\opening`'s address with a
            // `{\raggedleft ...\par}` *group*. That is a declaration, not a
            // `flushright` environment, so it closes no `\trivlist` and adds
            // no `\@topsepadd` after itself -- 9pt at 11pt, which is exactly
            // how much too far down the recipient block used to start.
            //
            // Both guards are independent and both are needed: the first keeps
            // a theorem's own closing skip, the second keeps a letter's
            // declaration group from claiming one it never opened.
            if *style != ParaStyle::Plain && !from_letter(parts) {
                *env_close = styles.get(i + 1).is_none_or(|next| *next != *style);
            }
        }
    }
    // `listings`: the compiler sets an `lstlisting` body as literal
    // typewriter lines and reports its `[...]` options, so `\lstset` and the
    // environment's keys are read from the source bytes here
    // (`crate::listings`). It runs *after* the `env_close` pass above: an
    // `lstlisting` is not a `\trivlist` — listings sets the body as a plain
    // paragraph under a `\parshape` — so the listing paragraph must keep
    // neither the opening nor the closing `\topsep`, and that pass would
    // otherwise put the closing one back.
    let (listing_superseded, listing_limitations) = crate::listings::apply(texts, &mut blocks, &style, labels);
    superseded.extend(listing_superseded);
    superseded.extend(crate::listings::lstset_spans(texts));
    limitations.extend(listing_limitations);
    // Page-style and mark commands (and a `\maketitle`) after the last
    // material.
    for cmd in &commands[next_command..] {
        let span = Span::in_document(entry_doc, cmd.start, cmd.end);
        match &cmd.kind {
            BodyKind::Event(event) => blocks.push(Block::Chrome { event: event.clone(), span }),
            BodyKind::MakeTitle => {
                if maketitle_plain {
                    blocks.push(Block::Chrome {
                        event: ChromeEvent::ThisPageStyle(PageStyle::Plain),
                        span,
                    });
                }
                if let Some(t) = stashed.next() {
                    blocks.push(title_of(t, span));
                }
            }
            _ => {}
        }
    }
    let page_starts = clear_page_blocks(texts, &blocks);
    Doc {
        style,
        blocks,
        diagnostics: Vec::new(),
        limitations,
        secnumdepth,
        page_color: parsed.page_color,
        default_color: parsed.default_color,
        math_colors: math_colors(&parsed.blocks),
        page_starts,
        superseded,
    }
}

/// `(document, start, end)` of every formula with a colour of its own
/// (`Inline::Math::color`); colours changed inside a formula
/// (`color_ranges`) are not painted: placed math glyphs carry no spans.
fn math_colors(blocks: &[flashtex_compiler::parser::Block]) -> std::collections::HashMap<(usize, usize, usize), DeviceColor> {
    use flashtex_compiler::parser::Block as CBlock;
    fn walk(inlines: &[Inline], out: &mut std::collections::HashMap<(usize, usize, usize), DeviceColor>) {
        for inline in inlines {
            match inline {
                Inline::Math { color: Some(c), span, .. } => {
                    out.insert((span.document.0, span.start, span.end), *c);
                }
                Inline::Footnote { text: Some(text), .. } => walk(text, out),
                Inline::Tabular(t) => {
                    for list in t.inline_lists() {
                        walk(list, out);
                    }
                }
                Inline::ColorBox(b) => walk(&b.content, out),
                Inline::Underline(u) => walk(&u.content, out),
                _ => {}
            }
        }
    }
    let mut out = std::collections::HashMap::new();
    for block in blocks {
        match block {
            CBlock::Paragraph(content)
            | CBlock::FigureCaption { content }
            | CBlock::Styled { content, .. }
            | CBlock::ListItem { content, .. }
            | CBlock::Heading { content, .. } => walk(content, &mut out),
            CBlock::TitleBlock { title, authors, date } => {
                walk(title, &mut out);
                walk(authors, &mut out);
                walk(date.as_deref().unwrap_or(&[]), &mut out);
            }
            _ => {}
        }
    }
    out
}

/// Blocks whose `eject_before` comes from `\clearpage`/`\cleardoublepage`
/// (the page-break command nearest before the block): in two-column mode
/// those end the page, `\newpage`/`\pagebreak` only the column (latex.ltx
/// `\clearpage` flushes with `\vbox{}\penalty-\@Mi`, `\@outputdblcol` ships
/// the page).
fn clear_page_blocks(texts: &[&str], blocks: &[Block]) -> Vec<usize> {
    let first_span = |items: &[Item]| {
        items.iter().find_map(|i| match i {
            Item::Word(w) => Some(w.span()),
            Item::Math { span, .. } => Some(*span),
            _ => None,
        })
    };
    blocks
        .iter()
        .enumerate()
        .filter_map(|(i, b)| {
            let at = match b {
                Block::Paragraph { eject_before: true, parts, .. } => parts.iter().find_map(|p| match p {
                    ParaPart::Lines(items) => first_span(items),
                    _ => None,
                }),
                Block::Heading { eject_before: true, span, .. } | Block::Rule { eject_before: true, span, .. } => Some(*span),
                Block::Picture { eject_before: true, document, picture, .. } => Some(Span::in_document(*document, picture.start, picture.end)),
                _ => None,
            }?;
            let text = texts.get(at.document.0)?.get(..at.start)?;
            let clear = text.rfind("\\clearpage").max(text.rfind("\\cleardoublepage"));
            let column = text.rfind("\\newpage").max(text.rfind("\\pagebreak"));
            (clear.is_some() && clear > column).then_some(i)
        })
        .collect()
}

fn inline_span(i: &Inline) -> Span {
    match i {
        Inline::Text { span, .. }
        | Inline::LineBreak { span, .. }
        | Inline::Math { span, .. }
        | Inline::MathRows { span, .. }
        | Inline::Label { span, .. }
        | Inline::Reference { span, .. }
        | Inline::CleverReference { span, .. }
        | Inline::HFill { span, .. }
        | Inline::HSpace { span, .. }
        | Inline::Footnote { span, .. }
        | Inline::Verbatim { span, .. }
        | Inline::TextGlue { span, .. }
        | Inline::Logo { span, .. }
        | Inline::Rule { span, .. }
        | Inline::Kern { span, .. } => *span,
        Inline::Tabular(t) => t.span,
        Inline::ColorBox(b) => b.span,
        Inline::Underline(u) => u.span,
        Inline::Graphic(g) => g.span,
        Inline::Transform(t) => t.span,
    }
}

/// The compiler inlines (pin `d416472a`) the pipeline sets only as plain
/// text, as `unsupported_block` limitations: `\footnote` (mark and text
/// inline, no page-bottom note), `tabular` (cells in reading order, no
/// columns or rules) and `\verb` (body face). Footnote text is scanned too.
fn unsupported_inlines(inline: &Inline, out: &mut Vec<(&'static str, Span, String)>) {
    match inline {
        Inline::Footnote { text, .. } => {
            // Set by `typeset::footnotes`; contexts it does not reach
            // (headings, captions, floats) are diagnosed there.
            for i in text.iter().flatten() {
                unsupported_inlines(i, out);
            }
        }
        Inline::Tabular(t) => {
            // Laid out by `table.rs`; only nested constructs are reported.
            for list in t.inline_lists() {
                for i in list {
                    unsupported_inlines(i, out);
                }
            }
        }
        Inline::Verbatim { .. } => {
            // `\verb`/`\verb*`/`\lstinline` are typeset in the typewriter
            // family with ligatures, kerns and stretchable blanks
            // suppressed (`style_intervals`, `TextStyle::literal`), so
            // there is nothing to report. Measured against pdflatex at
            // 12 pt T1: `\verb"ftxc --version"` 86.4289 pt, the oracle's
            // 86.4289 pt.
            //
            // `\verb*`'s visible-space glyph is the one remaining
            // difference, and it is not a geometry one: the compiler's
            // `Inline::Verbatim` does not record the star, and the blank
            // is `\fontdimen2` wide either way.
        }
        #[cfg(feature = "amsmath-inline")]
        Inline::MathRows { rows, .. } => {
            for i in rows.iter().flat_map(|r| &r.intertext).flat_map(|t| &t.content) {
                unsupported_inlines(i, out);
            }
        }
        _ => {}
    }
}

/// Lowers one compiler inline into the shapes `items_from_inlines` sets:
/// `\ref`/`\pageref`/`\eqref` become text from the label table; a
/// footnote becomes its mark (plain text) followed by its note text; a
/// tabular becomes its cells' inlines in reading order with `\\` between
/// rows; `\verb` becomes `Mono` text. Everything else is borrowed.
fn lower_inline<'a>(inline: &'a Inline, labels: &Labels, reference_spans: &mut Vec<Span>, out: &mut Vec<std::borrow::Cow<'a, Inline>>) {
    use flashtex_compiler::parser::{TextFamily, TextStyle as CStyle};
    match inline {
        Inline::Reference { key, page, equation, span, .. } => {
            let text = if *page {
                labels.pages.get(key).map(|p| p.to_string())
            } else {
                labels.values.get(key).cloned()
            }
            .unwrap_or_else(|| "??".to_string());
            // amsmath `\eqref`: the value in parentheses (compiler's flag).
            let text = if *equation { format!("({text})") } else { text };
            reference_spans.push(*span);
            out.push(std::borrow::Cow::Owned(Inline::Text {
                text,
                span: *span,
                style: Default::default(),
                // Interword gaps are read from the source bytes between
                // spans here, never from the compiler's flag.
                space_before: true,
            }));
        }
        Inline::CleverReference { keys, page, range, label_only, capitalise, span, .. } => {
            let text = clever_reference_text(keys, labels, *page, *range, *label_only, *capitalise);
            reference_spans.push(*span);
            out.push(std::borrow::Cow::Owned(Inline::Text {
                text,
                span: *span,
                style: Default::default(),
                // As for `Reference`: the gap comes from the source bytes
                // between spans, not the compiler's flag.
                space_before: true,
            }));
        }
        Inline::Verbatim { text, span, space_before } => {
            reference_spans.push(*span);
            out.push(std::borrow::Cow::Owned(Inline::Text {
                text: text.clone(),
                span: *span,
                style: CStyle {
                    family: TextFamily::Mono,
                    ..CStyle::default()
                },
                space_before: *space_before,
            }));
        }
        other => out.push(std::borrow::Cow::Borrowed(other)),
    }
}

/// One label a `\cref` group refers to, resolved from [`Labels`].
struct CleverItem {
    number: String,
    page: u32,
    /// The cleveref *class* (the compiler's `xref::cleveref_kind`): the kind
    /// several raw kinds collapse onto for grouping.
    kind: String,
    /// The label's own kind, which names the reference.
    raw_kind: String,
}

/// The text of a `cleveref` reference (`\cref`, `\Cref`, `\crefrange`,
/// `\cpageref`, `\labelcref` and their starred forms).
///
/// This mirrors `flashtex_compiler::layout`'s private `clever_reference_text`
/// (grouping by kind, consecutive-number ranges, `and`/`, and` joining,
/// parenthesised equation numbers) because the compiler's own resolver is not
/// public and the pipeline, not the compiler, lays this document out. The
/// naming itself is *not* duplicated: `xref::cleveref_name`/`cleveref_kind`
/// are public and are called here, so `\crefname` overrides and the
/// `capitalise`/`noabbrev` package options stay owned by the compiler.
///
/// An unresolved key contributes `??`, exactly as `\ref` does.
fn clever_reference_text(
    keys: &[String],
    labels: &Labels,
    page: bool,
    range: bool,
    label_only: bool,
    capitalise: bool,
) -> String {
    use flashtex_compiler::xref::{cleveref_kind, cleveref_name};
    let config = &labels.cleveref;
    let mut items: Vec<CleverItem> = Vec::with_capacity(keys.len());
    let mut unresolved = false;
    for key in keys.iter().filter(|k| !k.is_empty()) {
        match labels.values.get(key) {
            Some(number) => {
                let raw_kind = labels.kinds.get(key).cloned().unwrap_or_default();
                items.push(CleverItem {
                    number: number.clone(),
                    // A key whose page is unknown (no previous pass) sorts
                    // first and prints `??`, as `\pageref` does.
                    page: labels.pages.get(key).copied().unwrap_or(0),
                    kind: cleveref_kind(&raw_kind).to_string(),
                    raw_kind,
                });
            }
            None => unresolved = true,
        }
    }
    if items.is_empty() {
        return "??".into();
    }
    let with_unresolved = |text: String| {
        if unresolved {
            format!("{text} and ??")
        } else {
            text
        }
    };
    if range {
        // `\crefrange` needs exactly two labels of one kind; anything else is
        // what cleveref itself reports as an error.
        if items.len() != 2 || items[0].kind != items[1].kind {
            return "??".into();
        }
        let name = cleveref_name(config, &items[0].raw_kind, true, capitalise);
        return with_unresolved(format!(
            "{name} {} to {}",
            clever_number(&items[0]),
            clever_number(&items[1])
        ));
    }
    if page {
        items.sort_by_key(|item| item.page);
        let name = cleveref_name(config, "page", items.len() != 1, capitalise);
        return with_unresolved(format!("{name} {}", format_clever_pages(&items)));
    }
    if label_only {
        items.sort_by(compare_clever_items);
        return with_unresolved(format_clever_numbers(&items));
    }
    let mut groups: Vec<(String, Vec<CleverItem>)> = Vec::new();
    for item in items {
        if let Some((_, group)) = groups.iter_mut().find(|(kind, _)| kind == &item.kind) {
            group.push(item);
        } else {
            groups.push((item.kind.clone(), vec![item]));
        }
    }
    let parts = groups
        .iter_mut()
        .map(|(_, group)| {
            group.sort_by(compare_clever_items);
            let name = cleveref_name(config, &group[0].raw_kind, group.len() != 1, capitalise);
            format!("{name} {}", format_clever_numbers(group))
        })
        .collect::<Vec<_>>();
    // Groups take the Oxford comma (`A 1, B 2, and C 3`); the numbers inside
    // one group do not (`Sections 1, 2 and 3`), as cleveref sets them.
    with_unresolved(join_clever(&parts, true))
}

/// `\ref` numbers, collapsing three or more consecutive ones into a range.
fn format_clever_numbers(items: &[CleverItem]) -> String {
    let mut parts = Vec::new();
    let mut start = 0;
    while start < items.len() {
        let mut end = start;
        while end + 1 < items.len() && clever_consecutive(&items[end], &items[end + 1]) {
            end += 1;
        }
        if end - start >= 2 {
            parts.push(format!(
                "{} to {}",
                clever_number(&items[start]),
                clever_number(&items[end])
            ));
        } else {
            parts.extend(items[start..=end].iter().map(clever_number));
        }
        start = end + 1;
    }
    join_clever(&parts, false)
}

/// The same collapsing for `\cpageref`'s page numbers.
fn format_clever_pages(items: &[CleverItem]) -> String {
    let page_text = |item: &CleverItem| {
        if item.page == 0 {
            "??".to_string()
        } else {
            item.page.to_string()
        }
    };
    let mut parts = Vec::new();
    let mut start = 0;
    while start < items.len() {
        let mut end = start;
        while end + 1 < items.len()
            && items[end].page != 0
            && items[end].page + 1 == items[end + 1].page
        {
            end += 1;
        }
        if end - start >= 2 {
            parts.push(format!(
                "{} to {}",
                page_text(&items[start]),
                page_text(&items[end])
            ));
        } else {
            parts.extend(items[start..=end].iter().map(page_text));
        }
        start = end + 1;
    }
    join_clever(&parts, false)
}

/// Whether `second`'s number is `first`'s plus one, comparing only the last
/// dotted component and requiring the same prefix (`2.3` then `2.4`, never
/// `2.9` then `3.1`).
fn clever_consecutive(first: &CleverItem, second: &CleverItem) -> bool {
    let next_of = |text: &str| text.parse::<u32>().ok().and_then(|v| v.checked_add(1));
    match (first.number.rsplit_once('.'), second.number.rsplit_once('.')) {
        (None, None) => {
            let next = next_of(&first.number);
            next.is_some() && next == second.number.parse::<u32>().ok()
        }
        (Some((prefix, value)), Some((second_prefix, second_value))) => {
            let next = next_of(value);
            prefix == second_prefix && next.is_some() && next == second_value.parse::<u32>().ok()
        }
        _ => false,
    }
}

/// Dotted numbers sort component-wise (`1.9` before `1.10`); anything not
/// all-numeric falls back to a plain string compare.
fn compare_clever_items(first: &CleverItem, second: &CleverItem) -> std::cmp::Ordering {
    let parts =
        |text: &str| text.split('.').map(str::parse::<u32>).collect::<Result<Vec<_>, _>>();
    match (parts(&first.number), parts(&second.number)) {
        (Ok(a), Ok(b)) => a.cmp(&b),
        _ => first.number.cmp(&second.number),
    }
}

/// Equation numbers are parenthesised; every other kind is bare.
fn clever_number(item: &CleverItem) -> String {
    if item.kind == "equation" {
        format!("({})", item.number)
    } else {
        item.number.clone()
    }
}

fn join_clever(parts: &[String], oxford: bool) -> String {
    match parts {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let last = &parts[parts.len() - 1];
            let head = parts[..parts.len() - 1].join(", ");
            if oxford {
                format!("{head}, and {last}")
            } else {
                format!("{head} and {last}")
            }
        }
    }
}

/// A compiler block, or the piece of a paragraph between page-break
/// commands (`\newpage` ends the paragraph in LaTeX; the compiler keeps the
/// text in one block and reports the command as unsupported).
struct Unit<'p> {
    kind: UnitKind<'p>,
    eject_before: bool,
    /// Summed `\vspace` points from compiler `VSpace` blocks before this unit.
    vspace_before: f64,
    /// `\addvspace` glue before this unit (list skips; paragraphs only).
    addvspace_before: f64,
    /// See [`Block::Paragraph::addvspace_flex`].
    addvspace_flex: (f64, f64),
    vspace_flex: (f64, f64),
    /// See [`Block::Paragraph::endlist_adjust`].
    endlist_adjust: f64,
    /// Constructs before this unit the pipeline set approximately.
    limitations: Vec<(&'static str, Span, String)>,
}

enum UnitKind<'p> {
    Heading {
        level: u8,
        number: &'p str,
        number_span: Span,
        content: &'p [Inline],
    },
    Paragraph {
        inlines: &'p [Inline],
        caption: bool,
        /// A compiler `Styled` paragraph (`center`, `quote`, ...).
        styled: Option<ParaStyle>,
        /// The unit is the first paragraph of its environment (the gap
        /// before it holds `\begin{...}`); see [`EnvOpen`].
        env_open: Option<EnvOpen>,
        /// LaTeX's `\@endpe`: text that follows `\end{center}`/... without
        /// a blank line continues in the same paragraph, unindented.
        after_env: bool,
        /// The unit is the `\item` of an amsthm theorem-like environment
        /// (`\trivlist`, `\itemindent\z@`), so its first line is not
        /// indented; see [`opens_theorem_item`].
        theorem_item: bool,
        /// The unit's words are inside a theorem-like environment, so their
        /// weight comes from the compiler; see [`in_theorem_environment`].
        in_theorem: bool,
        /// A compiler `ListItem` paragraph: its `\list` geometry.
        list: Option<ListGeom>,
        /// The paragraph opens with a run-in heading (`\paragraph`,
        /// `\subparagraph`); see [`RunIn`] and [`run_in_heading_at`].
        run_in: Option<RunIn>,
        /// The leading this paragraph's `\par` selected ([`ParLeading`]).
        par_leading: ParLeading,
    },
    Rule {
        span: Span,
    },
    Picture {
        document: flashtex_compiler::DocumentId,
        picture: flashtex_vector_graphics::tikz::PictureSource,
        centered: bool,
    },
}

const PAGE_BREAKS: [&str; 3] = ["newpage", "clearpage", "pagebreak"];

/// The character the tie occupies in a compiler text run.
///
/// Declared here rather than imported from `flashtex_compiler::lexer`
/// because `vendor/compiler` predates that constant; the two are the same
/// code point and a re-pin can replace this with the import.
const NO_BREAK_SPACE: char = '\u{00A0}';

/// Whether the source between `prev` and `next` (same document, in order)
/// holds a page-break command.
fn gap_has_page_break(texts: &[&str], prev: Span, next: Span) -> bool {
    if prev.document != next.document || prev.end > next.start {
        return false;
    }
    let gap = texts.get(next.document.0).and_then(|t| t.get(prev.end..next.start)).unwrap_or("");
    PAGE_BREAKS.iter().any(|c| find_command(gap, c).is_some())
}

fn split_at_page_breaks<'p>(texts: &[&str], blocks: &'p [(CBlock, ParLeading)], size: u32, style: &Stylesheet) -> Vec<Unit<'p>> {
    let theorem_envs = theorem_environments(texts);
    let mut units = Vec::new();
    let mut prev_end: Option<Span> = None;
    // Carried from the compiler's own `PageBreak`/`VSpace`/`Rule` blocks
    // to the next unit that holds material.
    let mut pending_eject = false;
    let mut pending_vspace = 0.0f64;
    let mut pending_limitations: Vec<(&'static str, Span, String)> = Vec::new();
    // The previous unit left TeX in vertical mode (a heading or a rule).
    let mut prev_vmode = false;
    let mut prev_styled = false;
    // The previous unit was an `\item` paragraph, and whether its list's
    // `\begin` was read in vertical mode (`\@topsepadd` keeps `\partopsep`
    // for the closing skip too).
    let mut prev_list = false;
    let mut list_vmode = false;
    // `tikzpicture` environments per document, and those already emitted.
    let pictures: Vec<Vec<flashtex_vector_graphics::tikz::PictureSource>> = texts.iter().map(|t| flashtex_vector_graphics::tikz::find_pictures(t)).collect();
    let mut emitted_pictures: std::collections::BTreeSet<(usize, usize)> = std::collections::BTreeSet::new();
    for (block, par_leading) in blocks {
        let par_leading = *par_leading;
        match block {
            CBlock::PageBreak => {
                pending_eject = true;
                continue;
            }
            CBlock::VSpace { pt } => {
                pending_vspace += pt;
                continue;
            }
            CBlock::TableOfContents { span } => {
                prev_end = Some(*span);
                prev_vmode = true;
                continue;
            }
            CBlock::Rule { span } => {
                let eject = std::mem::take(&mut pending_eject) || prev_end.is_some_and(|p| gap_has_page_break(texts, p, *span));
                units.push(Unit {
                    kind: UnitKind::Rule { span: *span },
                    eject_before: eject,
                    vspace_before: std::mem::take(&mut pending_vspace),
                    addvspace_before: 0.0,
                    addvspace_flex: (0.0, 0.0),
                    vspace_flex: (0.0, 0.0),
                    endlist_adjust: 0.0,
                    limitations: std::mem::take(&mut pending_limitations),
                });
                prev_end = Some(*span);
                prev_vmode = true;
                continue;
            }
            _ => {}
        }
        let first = match block {
            CBlock::Heading { number_span, .. } => Some(*number_span),
            _ => inlines_of(block).iter().map(inline_span).next(),
        };
        let mut eject = std::mem::take(&mut pending_eject) || matches!((prev_end, first), (Some(p), Some(f)) if gap_has_page_break(texts, p, f));
        let mut vspace_before = std::mem::take(&mut pending_vspace);
        // The compiler evaluates `em`/`ex` in `\vspace` at a fixed 12pt;
        // LaTeX uses the class's `\normalsize`. Re-read the commands in
        // the gap before this unit when they are all there.
        if vspace_before != 0.0 {
            if let Some(f) = first {
                let gap = match prev_end {
                    Some(p) if p.document == f.document && p.end <= f.start => texts.get(f.document.0).and_then(|t| t.get(p.end..f.start)),
                    Some(_) => None,
                    None => texts.get(f.document.0).and_then(|t| t.get(..f.start)),
                };
                if let Some(pt) = gap.and_then(|g| vspace_in_gap(g, size)) {
                    vspace_before = pt;
                }
            }
        }
        // The compiler's list model (pin `42557b09`): every `\item`
        // paragraph is a `ListItem` with its nesting level and, for the
        // item's first paragraph, the marker text. Its `\setlist`
        // itemsep/topsep gaps are attached to whichever paragraph the
        // *next* `\item`/`\end` flushes, so an item holding a display
        // (which ends the paragraph early) carries them on the wrong
        // block, and `em` in them is the compiler's fixed 12pt body; the
        // pipeline sets the list's vertical glue from the source instead:
        //
        // `\@item` of the first item: `\addvspace\@topsep` (`\topsep` +
        // the outer `\parskip`, + `\partopsep` when `\begin` was read in
        // vertical mode) then `\addvspace{-\parskip}` with `\parskip` now
        // `\parsep`; the item paragraph then adds `\parsep`
        // (`paragraph_block`). The first `\addvspace` only tops up the
        // skip the previous block left (a display's `\belowdisplayskip`),
        // while the negative one always takes `\parsep` off whatever is
        // there, so it goes into `vspace_before`. Right after a heading (`\@nobreak`)
        // `\@nbitem`'s skip is absorbed by the heading's after-skip, so
        // only `\parsep` remains. Later items: `\addvspace\itemsep`. A
        // later paragraph of one item (no label) adds nothing but
        // `\parsep`. `\end{...}`: `\@endparenv` adds `\@topsepadd`, absorbed
        // by a following heading's larger before-skip (`\addvspace`).
        // The hanging indent and the label box are the pipeline's too
        // (`list_margins`): the compiler reports `leftmargin` as
        // unimplemented.
        let gap_before = |f: Span| -> Option<&str> {
            match prev_end {
                Some(p) if p.document == f.document && p.end <= f.start => texts.get(f.document.0).and_then(|t| t.get(p.end..f.start)),
                Some(_) => None,
                None => texts.get(f.document.0).and_then(|t| t.get(..f.start)),
            }
        };
        let is_heading = matches!(block, CBlock::Heading { .. });
        let mut addvspace_before = 0.0;
        let mut addvspace_flex = (0.0f64, 0.0f64);
        let mut vspace_flex = (0.0f64, 0.0f64);
        let mut endlist_adjust = 0.0;
        if prev_list && !is_heading {
            if let Some(gap) = first.and_then(gap_before) {
                if let Some(env) = gap_has_list_end(gap) {
                    let src = texts.get(prev_end.map_or(0, |p| p.document.0)).copied().unwrap_or("");
                    let stack = prev_end.map(|p| list_stack_at(src, p.end)).unwrap_or_default();
                    let begin_keys = stack.last().map_or("", |(e, keys)| if *e == env && *e != "thebibliography" { keys } else { "" });
                    let seps = list_seps_with(src, env, 1, size, style, begin_keys);
                    addvspace_before += seps.topsep + if list_vmode { seps.partopsep } else { 0.0 };
                    addvspace_flex.0 += seps.topsep_skip.stretch + if list_vmode { seps.partopsep_skip.stretch } else { 0.0 };
                    addvspace_flex.1 += seps.topsep_skip.shrink + if list_vmode { seps.partopsep_skip.shrink } else { 0.0 };
                    if let Some(p) = prev_end {
                        endlist_adjust = list_end_adjust(src, p.end, gap, size, style);
                    }
                }
            }
        }
        let mut list = None;
        if let CBlock::ListItem { level, label, .. } = block {
            let anchor = label.as_ref().map(|(_, span)| *span).or(first);
            if let Some(at) = anchor {
                let src = texts.get(at.document.0).copied().unwrap_or("");
                let stack = list_stack_at(src, at.start);
                let (env, begin_keys) = stack.last().map_or(("enumerate", ""), |(env, keys)| (env, if *env == "thebibliography" { "" } else { keys }));
                let seps = list_seps_with(src, env, stack.len().max(1), size, style, begin_keys);
                // `\@outerparskip`: the `\parskip` in force when `\begin`
                // was read — the enclosing list's `\parsep` when nested.
                let outer_parskip_skip = match stack.len() {
                    n if n > 1 => list_seps(src, stack[n - 2].0, n - 1, size, style).parsep_skip,
                    _ => style.parskip,
                };
                let outer_parskip = outer_parskip_skip.natural;
                if label.is_some() {
                    let opens = gap_before(at).and_then(|g| rfind_command(g, "begin").map(|b| (g, b))).or_else(|| {
                        // `\begin{thebibliography}{<widest>}` is the span of
                        // the compiler's own `References` heading, so the
                        // gap after that heading holds no `\begin`: look
                        // from the heading's start (`\@nbitem` follows).
                        let p = prev_end.filter(|p| prev_vmode && p.document == at.document && p.start < at.start)?;
                        let g = texts.get(p.document.0)?.get(p.start..at.start)?;
                        let b = rfind_command(g, "begin")?;
                        g[b..].strip_prefix("\\begin").is_some_and(|r| r.trim_start().starts_with("{thebibliography}")).then_some((g, b))
                    });
                    match opens {
                        Some((g, b)) if list_env_after_begin(&g[b..]) => {
                            let before = &g[..b];
                            list_vmode = prev_vmode || prev_end.is_none() || has_blank_line(before) || find_command(before, "par").is_some();
                            if prev_vmode {
                                // `\@nbitem`: `\addvspace{\@outerparskip - \parskip}`.
                                // A negative `\addvspace` is never absorbed:
                                // `\@xaddvskip`'s else branch adds it to a
                                // non-negative `\lastskip` (the heading's
                                // after-skip), so `\parsep` comes off it and
                                // the item paragraph's own `\parskip` (=
                                // `\parsep`) restores the heading's gap.
                                let nb = outer_parskip - seps.parsep;
                                let flex = (outer_parskip_skip.stretch - seps.parsep_skip.stretch, outer_parskip_skip.shrink - seps.parsep_skip.shrink);
                                if nb < 0.0 {
                                    vspace_before += nb;
                                    vspace_flex.0 += flex.0;
                                    vspace_flex.1 += flex.1;
                                } else {
                                    addvspace_before += nb;
                                    addvspace_flex.0 += flex.0;
                                    addvspace_flex.1 += flex.1;
                                }
                            } else {
                                addvspace_before += seps.topsep + outer_parskip + if list_vmode { seps.partopsep } else { 0.0 };
                                addvspace_flex.0 += seps.topsep_skip.stretch + outer_parskip_skip.stretch + if list_vmode { seps.partopsep_skip.stretch } else { 0.0 };
                                addvspace_flex.1 += seps.topsep_skip.shrink + outer_parskip_skip.shrink + if list_vmode { seps.partopsep_skip.shrink } else { 0.0 };
                                vspace_before -= seps.parsep;
                                vspace_flex.0 -= seps.parsep_skip.stretch;
                                vspace_flex.1 -= seps.parsep_skip.shrink;
                            }
                        }
                        _ => {
                            addvspace_before += seps.itemsep;
                            addvspace_flex.0 += seps.itemsep_skip.stretch;
                            addvspace_flex.1 += seps.itemsep_skip.shrink;
                        }
                    }
                }
                // natbib's author-year `thebibliography`, and only when the
                // compiler really did drop the entry's marker (`\@biblabel`
                // is `\hfill`): a build whose compiler still numbers the
                // entries keeps the class's label-width geometry, so this
                // never draws a `[1]` on top of the hanging indent.
                let natbib_bib = env == "thebibliography"
                    && natbib_author_year(src)
                    && label.as_ref().is_none_or(|(text, _)| text.is_empty());
                list = Some(ListGeom {
                    level: *level,
                    margins: list_margins(src, at.start, size, natbib_bib),
                    label: label.clone(),
                    description: env == "description",
                    parsep: seps.parsep_skip,
                    // `\NAT@bibsetup`: `\itemindent-\leftmargin`, so the
                    // entry's first line is flush at the margin and the rest
                    // of the entry hangs `\bibhang` in.
                    itemindent_em: if natbib_bib { -1.0 } else { 0.0 },
                });
            }
        }
        prev_list = list.is_some();
        let limitations = std::mem::take(&mut pending_limitations);
        let styled = match block {
            CBlock::Styled { style, .. } => Some(ParaStyle::of(*style)),
            _ => None,
        };
        // The environment opens here when the gap before the block holds
        // its `\begin`; `\partopsep` applies when that `\begin` was read in
        // vertical mode (nothing before it, or a blank line / `\par` between
        // the previous material and it).
        let env_open = styled.and_then(|_| {
            let f = first?;
            let gap = match prev_end {
                Some(p) if p.document == f.document && p.end <= f.start => texts.get(f.document.0).and_then(|t| t.get(p.end..f.start))?,
                Some(_) => return None,
                None => texts.get(f.document.0).and_then(|t| t.get(..f.start))?,
            };
            let begin = rfind_command(gap, "begin")?;
            let before = &gap[..begin];
            let vmode = prev_vmode || prev_end.is_none() || has_blank_line(before) || find_command(before, "par").is_some();
            Some(EnvOpen { vmode, skips: None })
        });
        // `\@endpe`: a plain paragraph right after `\end{...}` (no blank line
        // or `\par` between them) is not indented.
        let after_env = styled.is_none()
            && prev_styled
            && first.zip(prev_end).is_some_and(|(f, p)| {
                p.document == f.document
                    && p.end <= f.start
                    && texts.get(f.document.0).and_then(|t| t.get(p.end..f.start)).is_some_and(|gap| {
                        rfind_command(gap, "end").is_some_and(|end| {
                            let after = gap[end..].split_once('}').map_or("", |(_, rest)| rest);
                            !has_blank_line(after) && find_command(after, "par").is_none()
                        })
                    })
            });
        // The `\item` of an amsthm theorem-like environment: the gap before
        // this block holds its `\begin{...}` (only the environment's first
        // paragraph, so later ones keep the ambient `\parindent`).
        // `Some(is_proof)` when this block is the `\item` that opens a
        // theorem-like environment; `proof` is told apart because its closing
        // `\@topsepadd` is not `\topsep` (see [`theorem_skips`]).
        let theorem_open: Option<bool> = (list.is_none() && styled.is_none())
            .then(|| {
                let f = first?;
                let gap_start = match prev_end {
                    Some(p) if p.document == f.document && p.end <= f.start => Some(p.end),
                    Some(_) => None,
                    None => Some(0),
                };
                let t = texts.get(f.document.0)?;
                opens_theorem_item(t, gap_start?, f.start, &theorem_envs).map(|name| name == "proof")
            })
            .flatten();
        let theorem_item = theorem_open.is_some();
        // amsthm's `\@item` opens the `\trivlist` with `\addvspace\@topsep`
        // exactly as `center`/`quote` do, so the theorem reuses the
        // environment machinery rather than a second one beside it.
        let env_open = env_open.or_else(|| {
            theorem_open.map(|proof| EnvOpen {
                vmode: false,
                skips: Some(theorem_skips(style, proof)),
            })
        });
        let in_theorem = theorem_item
            || first.is_some_and(|f| texts.get(f.document.0).is_some_and(|t| in_theorem_environment(t, f.start, &theorem_envs)));
        // `\paragraph{...}`/`\subparagraph{...}`: the compiler set the title
        // as body text at the front of this very paragraph, so the head is
        // recognised from the bytes immediately before its first word.
        let mut run_in = (list.is_none() && styled.is_none())
            .then(|| first.and_then(|f| texts.get(f.document.0).and_then(|t| run_in_heading_at(t, f.start))))
            .flatten();
        prev_styled = styled.is_some();
        prev_vmode = matches!(block, CBlock::Heading { .. });
        match block {
            CBlock::Heading {
                level,
                number,
                number_span,
                content,
            } => {
                units.push(Unit {
                    kind: UnitKind::Heading {
                        level: *level,
                        number,
                        number_span: *number_span,
                        content,
                    },
                    eject_before: eject,
                    vspace_before,
                    addvspace_before,
                    addvspace_flex,
                    vspace_flex,
                    endlist_adjust: 0.0,
                    limitations,
                });
            }
            CBlock::Paragraph(inlines) | CBlock::ListItem { content: inlines, .. } | CBlock::FigureCaption { content: inlines } | CBlock::Styled { content: inlines, .. } => {
                let caption = matches!(block, CBlock::FigureCaption { .. });
                let mut env_open = env_open;
                // Only the environment's first unit carries the `\item`.
                let mut theorem_item = theorem_item;
                let mut vspace_before = vspace_before;
                let mut limitations = limitations;
                let centered = matches!(block, CBlock::Styled { style: flashtex_compiler::parser::ParagraphStyle::Center, .. });
                // Runs of inlines outside / inside one `tikzpicture`.
                let picture_of = |i: &Inline| -> Option<usize> {
                    let s = inline_span(i);
                    pictures.get(s.document.0)?.iter().position(|p| s.start >= p.start && s.start < p.end)
                };
                let mut segments: Vec<(usize, usize, Option<usize>)> = Vec::new();
                for (i, inline) in inlines.iter().enumerate() {
                    let pic = picture_of(inline);
                    match segments.last_mut() {
                        Some(last) if last.2 == pic => last.1 = i + 1,
                        _ => segments.push((i, i + 1, pic)),
                    }
                }
                if segments.is_empty() {
                    segments.push((0, 0, None));
                }
                for (seg_start, seg_end, pic) in segments {
                    if let Some(k) = pic {
                        let document = inline_span(&inlines[seg_start]).document;
                        if emitted_pictures.insert((document.0, k)) {
                            units.push(Unit {
                                kind: UnitKind::Picture {
                                    document,
                                    picture: pictures[document.0][k].clone(),
                                    centered,
                                },
                                eject_before: eject,
                                vspace_before: std::mem::take(&mut vspace_before),
                                addvspace_before: std::mem::take(&mut addvspace_before),
                                addvspace_flex: std::mem::take(&mut addvspace_flex),
                                vspace_flex: std::mem::take(&mut vspace_flex),
                                endlist_adjust: std::mem::take(&mut endlist_adjust),
                                limitations: std::mem::take(&mut limitations),
                            });
                            eject = false;
                        }
                        continue;
                    }
                    let seg = &inlines[seg_start..seg_end];
                    let mut start = 0usize;
                    for i in 1..seg.len() {
                        if gap_has_page_break(texts, inline_span(&seg[i - 1]), inline_span(&seg[i])) {
                            units.push(Unit {
                                kind: UnitKind::Paragraph {
                                    inlines: &seg[start..i],
                                    caption,
                                    styled,
                                    env_open: env_open.take(),
                                    after_env,
                                    theorem_item: std::mem::take(&mut theorem_item),
                                    in_theorem,
                                    list: list.clone(),
                                    run_in: std::mem::take(&mut run_in),
                                    par_leading,
                                },
                                eject_before: eject,
                                vspace_before: std::mem::take(&mut vspace_before),
                                addvspace_before: std::mem::take(&mut addvspace_before),
                                addvspace_flex: std::mem::take(&mut addvspace_flex),
                                vspace_flex: std::mem::take(&mut vspace_flex),
                                endlist_adjust: std::mem::take(&mut endlist_adjust),
                                limitations: std::mem::take(&mut limitations),
                            });
                            eject = true;
                            start = i;
                        }
                    }
                    units.push(Unit {
                        kind: UnitKind::Paragraph {
                            inlines: &seg[start..],
                            caption,
                            styled,
                            env_open: env_open.take(),
                            after_env,
                            theorem_item: std::mem::take(&mut theorem_item),
                            in_theorem,
                            list: list.clone(),
                            run_in: std::mem::take(&mut run_in),
                            par_leading,
                        },
                        eject_before: eject,
                        vspace_before: std::mem::take(&mut vspace_before),
                        addvspace_before: std::mem::take(&mut addvspace_before),
                        addvspace_flex: std::mem::take(&mut addvspace_flex),
                        vspace_flex: std::mem::take(&mut vspace_flex),
                        endlist_adjust: std::mem::take(&mut endlist_adjust),
                        limitations: std::mem::take(&mut limitations),
                    });
                    eject = false;
                }
            }
            CBlock::VSpace { .. } | CBlock::Rule { .. } | CBlock::PageBreak => unreachable!("handled above"),
            CBlock::Verbatim { .. } | CBlock::TableOfContents { .. } | CBlock::TitleBlock { .. } | CBlock::VFill | CBlock::LetterBlock { .. } => unreachable!("lowered by lower_blocks"),
        }
        if let Some(last) = inlines_of(block).iter().map(inline_span).last() {
            prev_end = Some(last);
        }
    }
    units
}

fn is_display(inlines: &[Inline], span: Span) -> bool {
    inlines.iter().any(|i| match i {
        Inline::Math { display: true, span: s, .. } => *s == span,
        Inline::MathRows { rows, .. } => rows.iter().any(|r| r.span == span),
        _ => false,
    })
}

/// The row of an amsmath multi-row display whose span is `span`, if any:
/// `Some(number)` where `number` is the row's own equation number.
/// The amsmath row whose span is `span`, with its environment's span.
fn math_row_of(inlines: &[Inline], span: Span) -> Option<(Span, &flashtex_compiler::parser::MathRow)> {
    inlines.iter().find_map(|i| match i {
        Inline::MathRows { rows, span: env, .. } => rows.iter().find(|r| r.span == span).map(|r| (*env, r)),
        _ => None,
    })
}

/// `list` without the atoms the compiler makes of `\tag{..}`/`\tag*{..}` (the
/// label text and the `2\quad` glue it inserts, both spanning the command);
/// the label as set goes to `tag`: `\tagform@`'s parentheses for `\tag`,
/// none for `\tag*`.
fn strip_tag(texts: &[&str], list: &MathList, tag: &mut Option<String>) -> MathList {
    use flashtex_compiler::math::Nucleus;
    let is_tag = |span: Span| texts.get(span.document.0).and_then(|t| t.get(span.start..)).is_some_and(|r| r.starts_with("\\tag"));
    let mut atoms = Vec::with_capacity(list.atoms.len());
    for a in &list.atoms {
        if is_tag(a.span) {
            if let Nucleus::Text(s) | Nucleus::Symbol(s) = &a.nucleus {
                *tag = Some(s.clone());
            }
            continue;
        }
        atoms.push(a.clone());
    }
    MathList { atoms }
}

/// `$$ ... \eqno <number> $$` (or `\leqno`): the compiler reads the
/// primitive as text, so the atoms from it on are dropped and the source
/// after the command (before the closing `$$`) becomes the number, set as
/// is; its span starts at the command, which tells the typesetter the side.
fn strip_eqno(texts: &[&str], list: MathList, display: Span) -> (MathList, Option<(String, Span)>) {
    let src = texts.get(display.document.0).copied().unwrap_or("");
    let is_eqno = |start: usize| src.get(start..).is_some_and(|r| r.starts_with("\\eqno") || r.starts_with("\\leqno"));
    let Some(i) = list.atoms.iter().position(|a| a.span.document == display.document && is_eqno(a.span.start)) else {
        return (list, None);
    };
    let start = list.atoms[i].span.start;
    let end = display.end.min(src.len()).max(start);
    let body = &src[start..end];
    let body = body.strip_prefix("\\leqno").or_else(|| body.strip_prefix("\\eqno")).unwrap_or(body).trim_end();
    let body = body.strip_suffix("$$").unwrap_or(body).trim();
    let mut atoms = list.atoms;
    atoms.truncate(i);
    (MathList { atoms }, Some((body.to_string(), Span::in_document(display.document, start, end))))
}

/// Whether the source between two consecutive pieces of material keeps TeX
/// in the same paragraph (no blank line, no `\par`).
fn gap_continues(texts: &[&str], prev: Span, next: Span) -> bool {
    prev.document == next.document
        && prev.end <= next.start
        && texts
            .get(next.document.0)
            .and_then(|t| t.get(prev.end..next.start))
            .is_some_and(|gap| !has_blank_line(gap) && find_command(gap, "par").is_none())
}

#[allow(dead_code)]
fn math_row_number(inlines: &[Inline], span: Span) -> Option<Option<(String, Span)>> {
    inlines.iter().find_map(|i| match i {
        Inline::MathRows { rows, .. } => rows.iter().find(|r| r.span == span).map(|r| r.number.clone().map(|n| (n, r.span))),
        _ => None,
    })
}

/// One row of an amsmath display as a single math list: the `&`-separated
/// cells concatenated in order (the alignment points are reported by
/// `adapt` as a `math_limitation`).
fn math_row_list(row: &flashtex_compiler::parser::MathRow) -> MathList {
    MathList {
        atoms: row.cells.iter().flat_map(|c| c.atoms.iter().cloned()).collect(),
    }
}

fn display_number(inlines: &[Inline], span: Span) -> Option<(String, Span)> {
    inlines.iter().find_map(|i| match i {
        Inline::Math {
            display: true,
            span: s,
            number: Some(n),
            number_span,
            ..
        } if *s == span => Some((n.clone(), number_span.unwrap_or(span))),
        _ => None,
    })
}

/// Whether `\usepackage[...]{fontenc}` makes T1 the text encoding: the last
/// encoding option becomes `\encodingdefault` (`[OT1,T1]` → T1).
pub fn t1_encoding(source: &str) -> bool {
    package_options(source, "fontenc")
        .is_some_and(|opts| opts.split(',').map(str::trim).filter(|o| !o.is_empty()).last() == Some("T1"))
}

/// `\usepackage[<options>]{microtype}` under pdfTeX in PDF mode: protrusion
/// and expansion both on by default (`\pdfprotrudechars=2`,
/// `\pdfadjustspacing=2`, stretch/shrink 20, step 1, autoexpand);
/// `protrusion=`/`expansion=` take `true`, `false`, `compatibility` (level 1),
/// `nocompatibility` or a font set name; `disable` (or `disable=true`) switches
/// both off, `disable=ifdraft` only under `draft` (a package or class option;
/// `draft` alone changes nothing, `final` is a no-op in microtype.sty v3.2);
/// `factor`, `stretch`, `shrink`, `step`, `selected` and `auto` map
/// to [`flashtex_microtype::Options`]. Other options (`tracking`, `kerning`,
/// `spacing`, `letterspace`, ...) do not affect pdfTeX's protrusion or
/// expansion and are ignored, as is `\microtypesetup` (not read).
pub fn microtype_setup(source: &str) -> Option<crate::style::MicrotypeSetup> {
    let opts = package_options(source, "microtype")?;
    let mut o = flashtex_microtype::Options::default();
    let (mut protrude, mut adjust) = (2, 2);
    let mut draft = class_options(source).is_some_and(|c| c.split(',').any(|o| o.trim() == "draft"));
    let (mut disable, mut disable_ifdraft) = (false, false);
    let level = |v: Option<&str>| match v {
        Some("false") => 0,
        Some("compatibility") => 1,
        _ => 2,
    };
    let int = |v: Option<&str>, d: i32| v.and_then(|v| v.parse::<i32>().ok()).unwrap_or(d);
    let named = |v: Option<&str>| {
        v.filter(|v| !matches!(*v, "true" | "false" | "compatibility" | "nocompatibility"))
            .map(str::to_string)
    };
    for kv in opts.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (k, v) = match kv.split_once('=') {
            Some((k, v)) => (k.trim(), Some(v.trim().trim_matches(|c| c == '{' || c == '}').trim())),
            None => (kv, None),
        };
        match k {
            "draft" => draft = v != Some("false"),
            "disable" => match v {
                None | Some("true") => disable = true,
                Some("ifdraft") => disable_ifdraft = true,
                _ => {}
            },
            "protrusion" => {
                protrude = level(v);
                if let Some(set) = named(v) {
                    o.protrusion_set = Some(set);
                }
            }
            "expansion" => {
                adjust = level(v);
                if let Some(set) = named(v) {
                    o.expansion_set = Some(set);
                }
            }
            "factor" => o.protrusion_factor = int(v, o.protrusion_factor),
            "stretch" => o.stretch = int(v, o.stretch),
            "shrink" => o.shrink = int(v, o.shrink),
            "step" => o.step = int(v, o.step),
            "selected" => o.selected = v != Some("false"),
            "auto" => o.auto_expand = v != Some("false"),
            _ => {}
        }
    }
    if disable || (disable_ifdraft && draft) {
        (protrude, adjust) = (0, 0);
    }
    o.protrusion = protrude > 0;
    o.expansion = adjust > 0;
    Some(crate::style::MicrotypeSetup { options: o, protrude_chars: protrude, adjust_spacing: adjust })
}

/// Options of `\usepackage[opts]{name}`, if the package is loaded.
pub fn package_options(source: &str, name: &str) -> Option<String> {
    let mut from = 0;
    while let Some(at) = find_command(&source[from..], "usepackage") {
        let abs = from + at;
        let rest = source[abs + "\\usepackage".len()..].trim_start();
        let (opts, rest) = match rest.strip_prefix('[') {
            Some(inner) => {
                let end = inner.find(']')?;
                (inner[..end].to_string(), inner[end + 1..].trim_start())
            }
            None => (String::new(), rest),
        };
        if let Some(arg) = rest.strip_prefix('{') {
            if let Some(end) = arg.find('}') {
                if arg[..end].split(',').any(|p| p.trim() == name) {
                    return Some(opts);
                }
            }
        }
        from = abs + 1;
    }
    None
}

/// The preamble facts that decide the page frame, read by
/// `flashtex_class_geometry::DocumentSetup::from_preamble` (standard class,
/// every `\usepackage[..]{geometry}` option, `\geometry{..}` calls,
/// `\pagestyle`). Body-only input (`has_class == false`) inherits the
/// compiler's implicit preamble: article with `class_options` and
/// `\usepackage[margin=1in]{geometry}`. A declared non-standard class
/// (`amsart`, ...) keeps the previous behaviour: article geometry with the
/// class options and the `geometry` package options, if loaded.
pub fn document_setup(source: &str, has_class: bool, class_options: &str) -> DocumentSetup {
    if has_class {
        if let Some(setup) = DocumentSetup::from_preamble(source) {
            return setup;
        }
    }
    let mut setup = DocumentSetup::new(ClassKind::Article, class_options);
    // No header or footer: the compiler's implicit preamble (and a class
    // this crate does not model) never had page chrome.
    setup.pagestyle = Some(PageStyle::Empty);
    setup.geometry = match package_options(source, "geometry") {
        Some(opts) => Some(GeometryInput {
            package_options: opts,
            calls: Vec::new(),
        }),
        None if !has_class => Some(GeometryInput {
            package_options: "margin=1in".into(),
            calls: Vec::new(),
        }),
        None => None,
    };
    setup
}

/// Lengths the geometry package overwrites. An earlier `\setlength` of one
/// of these is ignored when `geometry` runs later, matching LaTeX.
const GEOMETRY_LENGTHS: &[&str] = &[
    "paperwidth",
    "paperheight",
    "textwidth",
    "textheight",
    "oddsidemargin",
    "evensidemargin",
    "topmargin",
    "headheight",
    "headsep",
    "footskip",
    "marginparwidth",
    "marginparsep",
    "columnsep",
];

const PREAMBLE_LENGTHS: &[&str] = &[
    "paperwidth",
    "paperheight",
    "textwidth",
    "textheight",
    "oddsidemargin",
    "evensidemargin",
    "topmargin",
    "headheight",
    "headsep",
    "footskip",
    "marginparwidth",
    "marginparsep",
    "columnsep",
    "parindent",
    "parskip",
    "columnseprule",
];

struct LengthAssigns {
    parindent: bool,
    parskip: bool,
}

/// Apply preamble `\setlength` / `\addtolength` / `\len=<dimen>` after the
/// class defaults and the geometry package, in source order.
///
/// Known limits (see ignored tests): `\input`/`\include` files are not in
/// `source`, so their assignments are missed; `\makeatletter` `\@setlength`
/// is missed because [`next_command`] only collects ASCII letters.
fn apply_preamble_lengths(
    source: &str,
    doc: &mut ResolvedDocument,
    size: u32,
    family: crate::fonts::Family,
    geometry: bool,
) -> LengthAssigns {
    let preamble_end = document_begin_offset(source).unwrap_or(source.len());
    let last_geometry = last_geometry_offset(source, preamble_end);
    let em_ex = ec_em_ex(size, family);
    let mut assigned = LengthAssigns { parindent: false, parskip: false };
    let mut params = doc.params;
    let mut scan = CmdScan::new(source);
    while let Some((at, name, depth)) = scan.next() {
        if depth != 0 {
            continue;
        }
        let after_name = at + 1 + name.len();
        if matches!(name, "newcommand" | "renewcommand" | "providecommand" | "def" | "gdef" | "edef" | "xdef") {
            scan.skip_to(skip_macro_definition(source, name, after_name));
            continue;
        }
        if name == "setlength" || name == "addtolength" {
            if let Some((target, raw)) = setlength_args(source, after_name) {
                let page = GEOMETRY_LENGTHS.contains(&target.as_str());
                if page && at >= preamble_end {
                    continue;
                }
                if page && last_geometry.is_some_and(|g| at < g) {
                    continue;
                }
                if let Some(v) = parse_assignment_glue(&raw, &params, size, em_ex) {
                    assign_param(&mut params, &target, v, name == "addtolength");
                    assigned.parindent |= target == "parindent";
                    assigned.parskip |= target == "parskip";
                }
            }
            continue;
        }
        if !PREAMBLE_LENGTHS.contains(&name) {
            continue;
        }
        let page = GEOMETRY_LENGTHS.contains(&name);
        if page && at >= preamble_end {
            continue;
        }
        if page && last_geometry.is_some_and(|g| at < g) {
            continue;
        }
        if let Some(raw) = read_assignment_dimen(source, after_name) {
            if let Some(v) = parse_assignment_glue(&raw, &params, size, em_ex) {
                assign_param(&mut params, name, v, false);
                assigned.parindent |= name == "parindent";
                assigned.parskip |= name == "parskip";
            }
        }
    }
    if assigned.parindent {
        // \@startsection records \parindent into the heading spec at
        // definition time; re-resolve after preamble assignments so
        // \subparagraph sees the final indent. class-geometry is unchanged.
        doc.headings = flashtex_class_geometry::sections::headings(
            doc.options.kind,
            &params,
            doc.font,
            doc.secnumdepth,
        );
    }
    // geometry's pdftex driver copies \paperwidth/\paperheight into the
    // MediaBox at \begin{document}. Without geometry, pdfTeX keeps the
    // engine default even after a later \setlength of those registers.
    let media = if geometry {
        (params.paperwidth, params.paperheight)
    } else {
        (doc.frame.pdf_page_width, doc.frame.pdf_page_height)
    };
    doc.params = params;
    doc.frame = PageFrame::new(&params, doc.flags, media);
    assigned
}

fn last_geometry_offset(source: &str, preamble_end: usize) -> Option<usize> {
    let mut last = None;
    let mut from = 0;
    while let Some((at, name)) = next_command(&source[..preamble_end], from) {
        from = at + 1;
        match name {
            "geometry" => last = Some(at),
            "usepackage" | "RequirePackage" => {
                if let Some((_, arg)) = usepackage_arg(&source[..preamble_end], at + 1 + name.len()) {
                    if arg.split(',').any(|p| p.trim() == "geometry") {
                        last = Some(at);
                    }
                }
            }
            _ => {}
        }
    }
    last
}

/// One linear pass over `source`: comments, escaped bytes, and `{`/`}` depth.
struct CmdScan<'a> {
    source: &'a str,
    i: usize,
    depth: i64,
    comment: bool,
}

impl<'a> CmdScan<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            i: 0,
            depth: 0,
            comment: false,
        }
    }

    fn skip_to(&mut self, pos: usize) {
        if pos > self.i {
            self.i = pos;
        }
        self.comment = false;
    }

    /// Next alphabetic control word and the brace depth at its backslash.
    fn next(&mut self) -> Option<(usize, &'a str, i64)> {
        let bytes = self.source.as_bytes();
        while self.i < bytes.len() {
            let c = bytes[self.i];
            if self.comment {
                if c == b'\n' {
                    self.comment = false;
                }
                self.i += 1;
                continue;
            }
            match c {
                b'%' => {
                    self.comment = true;
                    self.i += 1;
                }
                b'\\' => {
                    let start = self.i + 1;
                    let mut j = start;
                    while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                        j += 1;
                    }
                    if j > start {
                        let at = self.i;
                        let depth = self.depth;
                        self.i = j;
                        return Some((at, &self.source[start..j], depth));
                    }
                    self.i += 2;
                }
                b'{' => {
                    self.depth += 1;
                    self.i += 1;
                }
                b'}' => {
                    self.depth = (self.depth - 1).max(0);
                    self.i += 1;
                }
                _ => self.i += 1,
            }
        }
        None
    }
}

fn next_command(source: &str, from: usize) -> Option<(usize, &str)> {
    // ASCII letters only: `\@setlength` after `\makeatletter` is a known
    // limit (ignored test `preamble_scan_does_not_see_at_setlength`).
    let bytes = source.as_bytes();
    let mut i = from;
    let mut in_comment = false;
    while i < bytes.len() {
        let c = bytes[i];
        if in_comment {
            if c == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        if c == b'%' {
            in_comment = true;
            i += 1;
            continue;
        }
        if c == b'\\' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                j += 1;
            }
            if j > start {
                return Some((i, &source[start..j]));
            }
            i += 2;
            continue;
        }
        i += 1;
    }
    None
}

fn skip_ws(source: &str, mut i: usize) -> usize {
    let b = source.as_bytes();
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// Inner of a `{...}` group, comments stripped, escapes kept.
///
/// Not [`matching_brace`]: that helper returns a close index and does not
/// skip `%` comments, so `{6in%\n}` would count a `}` inside the comment and
/// break [`tests::preamble_scan_strips_comments_inside_dimension_groups`].
/// It also does not skip leading whitespace or yield the inner bytes.
fn read_group(source: &str, i: &mut usize) -> Option<String> {
    *i = skip_ws(source, *i);
    let b = source.as_bytes();
    if b.get(*i) != Some(&b'{') {
        return None;
    }
    *i += 1;
    let mut out = String::new();
    let mut depth = 1i32;
    let mut comment = false;
    while *i < b.len() {
        let c = b[*i];
        if comment {
            if c == b'\n' {
                comment = false;
            }
            *i += 1;
            continue;
        }
        match c {
            b'%' => {
                comment = true;
                *i += 1;
            }
            b'\\' => {
                out.push('\\');
                *i += 1;
                if *i < b.len() {
                    out.push(b[*i] as char);
                    *i += 1;
                }
            }
            b'{' => {
                depth += 1;
                out.push('{');
                *i += 1;
            }
            b'}' => {
                depth -= 1;
                *i += 1;
                if depth == 0 {
                    return Some(out);
                }
                out.push('}');
            }
            _ => {
                out.push(c as char);
                *i += 1;
            }
        }
    }
    None
}

/// Offset of `\begin{document}` / `\begin {document}`. Comments, brace
/// groups, and `\newcommand`/`\def` bodies are skipped the same way as
/// [`apply_preamble_lengths`].
fn document_begin_offset(source: &str) -> Option<usize> {
    let mut scan = CmdScan::new(source);
    while let Some((at, name, depth)) = scan.next() {
        if depth != 0 {
            continue;
        }
        if matches!(
            name,
            "newcommand" | "renewcommand" | "providecommand" | "def" | "gdef" | "edef" | "xdef"
        ) {
            scan.skip_to(skip_macro_definition(source, name, at + 1 + name.len()));
            continue;
        }
        if name != "begin" {
            continue;
        }
        let mut i = skip_ws(source, at + "\\begin".len());
        if let Some(env) = read_group(source, &mut i) {
            if env.trim() == "document" {
                return Some(at);
            }
        }
    }
    None
}

fn skip_macro_definition(source: &str, name: &str, mut i: usize) -> usize {
    let b = source.as_bytes();
    i = skip_ws(source, i);
    if matches!(name, "newcommand" | "renewcommand" | "providecommand") {
        if b.get(i) == Some(&b'*') {
            i += 1;
        }
        i = skip_ws(source, i);
        while b.get(i) == Some(&b'[') {
            i += 1;
            while i < b.len() && b[i] != b']' {
                i += 1;
            }
            if i < b.len() {
                i += 1;
            }
            i = skip_ws(source, i);
        }
        if b.get(i) == Some(&b'{') {
            let _ = read_group(source, &mut i);
        } else if b.get(i) == Some(&b'\\') {
            i += 1;
            while i < b.len() && b[i].is_ascii_alphabetic() {
                i += 1;
            }
        }
        i = skip_ws(source, i);
        if b.get(i) == Some(&b'{') {
            let _ = read_group(source, &mut i);
        }
        return i;
    }
    if b.get(i) == Some(&b'\\') {
        i += 1;
        while i < b.len() && b[i].is_ascii_alphabetic() {
            i += 1;
        }
    }
    while i < b.len() && b[i] != b'{' {
        i += 1;
    }
    if b.get(i) == Some(&b'{') {
        let _ = read_group(source, &mut i);
    }
    i
}

fn setlength_args(source: &str, mut i: usize) -> Option<(String, String)> {
    i = skip_ws(source, i);
    let b = source.as_bytes();
    let target = if b.get(i) == Some(&b'{') {
        read_group(source, &mut i)?
            .trim()
            .trim_start_matches('\\')
            .to_string()
    } else if b.get(i) == Some(&b'\\') {
        i += 1;
        let start = i;
        while i < b.len() && b[i].is_ascii_alphabetic() {
            i += 1;
        }
        source[start..i].to_string()
    } else {
        return None;
    };
    let value = read_group(source, &mut i)?;
    Some((target, value))
}

fn usepackage_arg(source: &str, mut i: usize) -> Option<(String, String)> {
    i = skip_ws(source, i);
    let b = source.as_bytes();
    let opts = if b.get(i) == Some(&b'[') {
        i += 1;
        let start = i;
        while i < b.len() && b[i] != b']' {
            i += 1;
        }
        let o = source[start..i].to_string();
        if i < b.len() {
            i += 1;
        }
        o
    } else {
        String::new()
    };
    let arg = read_group(source, &mut i)?;
    Some((opts, arg))
}

fn read_assignment_dimen(source: &str, mut i: usize) -> Option<String> {
    i = skip_ws(source, i);
    let b = source.as_bytes();
    let start = i;
    if b.get(i) == Some(&b'=') {
        i += 1;
        i = skip_ws(source, i);
    }
    i = read_one_dimen(source, i)?;
    loop {
        let j = skip_ws(source, i);
        if let Some(rest) = keyword_at(source, j, "plus").or_else(|| keyword_at(source, j, "minus")) {
            i = read_one_dimen(source, skip_ws(source, rest))?;
        } else {
            break;
        }
    }
    Some(source[start..i].to_string())
}

fn read_one_dimen(source: &str, mut i: usize) -> Option<usize> {
    let b = source.as_bytes();
    if i < b.len() && (b[i] == b'-' || b[i] == b'+') {
        i += 1;
    }
    if b.get(i) == Some(&b'\\') {
        i += 1;
        while i < b.len() && b[i].is_ascii_alphabetic() {
            i += 1;
        }
        return Some(i);
    }
    let num = i;
    while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
        i += 1;
    }
    if i == num {
        return None;
    }
    if b.get(i) == Some(&b'\\') {
        i += 1;
        while i < b.len() && b[i].is_ascii_alphabetic() {
            i += 1;
        }
        return Some(i);
    }
    let unit = i;
    while i < b.len() && b[i].is_ascii_alphabetic() {
        i += 1;
    }
    if i == unit {
        return None;
    }
    Some(i)
}

fn keyword_at(source: &str, i: usize, kw: &str) -> Option<usize> {
    if source[i..].starts_with(kw) {
        let after = i + kw.len();
        let b = source.as_bytes();
        if after == b.len() || b[after].is_ascii_whitespace() || matches!(b[after], b'-' | b'+' | b'.' | b'\\') || b[after].is_ascii_digit()
        {
            return Some(after);
        }
    }
    None
}

fn parse_assignment_glue(
    raw: &str,
    params: &PageParams,
    size: u32,
    em_ex: Option<(f64, f64)>,
) -> Option<Glue> {
    let s = raw.trim().trim_start_matches('=').trim();
    let (natural_s, stretch_s, shrink_s) = split_skip_spec(s);
    let natural = parse_assignment_dimen(natural_s, params, size, em_ex)?;
    let mut g = Glue::fixed(natural);
    if let Some(p) = stretch_s {
        g.stretch = parse_assignment_dimen(p, params, size, em_ex)?;
    }
    if let Some(m) = shrink_s {
        g.shrink = parse_assignment_dimen(m, params, size, em_ex)?;
    }
    Some(g)
}

fn split_skip_spec(s: &str) -> (&str, Option<&str>, Option<&str>) {
    let plus = skip_keyword_index(s, "plus");
    let minus = skip_keyword_index(s, "minus");
    let (natural_end, stretch, shrink) = match (plus, minus) {
        (Some(p), Some(m)) if p < m => (p, Some(s[p + 4..m].trim()), Some(s[m + 5..].trim())),
        (Some(p), Some(m)) => (m, Some(s[p + 4..].trim()), Some(s[m + 5..p].trim())),
        (Some(p), None) => (p, Some(s[p + 4..].trim()), None),
        (None, Some(m)) => (m, None, Some(s[m + 5..].trim())),
        (None, None) => return (s, None, None),
    };
    (s[..natural_end].trim(), stretch.filter(|t| !t.is_empty()), shrink.filter(|t| !t.is_empty()))
}

fn skip_keyword_index(s: &str, kw: &str) -> Option<usize> {
    let b = s.as_bytes();
    let mut i = 0;
    while i + kw.len() <= b.len() {
        if s[i..].starts_with(kw) {
            let before = i == 0 || b[i - 1].is_ascii_whitespace();
            let after = i + kw.len();
            let after_ok = after == b.len()
                || b[after].is_ascii_whitespace()
                || matches!(b[after], b'-' | b'+' | b'.' | b'\\')
                || b[after].is_ascii_digit();
            if before && after_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn parse_assignment_dimen(
    raw: &str,
    params: &PageParams,
    size: u32,
    em_ex: Option<(f64, f64)>,
) -> Option<Sp> {
    let s = raw.trim().trim_start_matches('=').trim();
    if let Some(bs) = s.find('\\') {
        let (factor, rest) = s.split_at(bs);
        let name = rest[1..].trim();
        let base = param_length(params, name)?;
        let f = factor.trim();
        if f.is_empty() || f == "+" {
            return Some(base);
        }
        if f == "-" {
            return Some(-base);
        }
        return base.scaled(f);
    }
    if let Some(v) = param_length(params, s.trim_start_matches('\\')) {
        return Some(v);
    }
    if s.ends_with("em") || s.ends_with("ex") {
        let pt = parse_dimen_in(s, size, em_ex)?;
        return Some(Sp((pt * 65536.0).round() as i64));
    }
    Sp::parse(s)
}

fn param_length(p: &PageParams, name: &str) -> Option<Sp> {
    Some(match name {
        "paperwidth" => p.paperwidth,
        "paperheight" => p.paperheight,
        "textwidth" | "linewidth" | "columnwidth" | "hsize" => p.textwidth,
        "textheight" => p.textheight,
        "oddsidemargin" => p.oddsidemargin,
        "evensidemargin" => p.evensidemargin,
        "topmargin" => p.topmargin,
        "headheight" => p.headheight,
        "headsep" => p.headsep,
        "footskip" => p.footskip,
        "marginparwidth" => p.marginparwidth,
        "marginparsep" => p.marginparsep,
        "columnsep" => p.columnsep,
        "parindent" => p.parindent,
        "parskip" => p.parskip.natural,
        "columnseprule" => p.columnseprule,
        _ => return None,
    })
}

fn assign_param(p: &mut PageParams, name: &str, v: Glue, add: bool) {
    if name == "parskip" {
        if add {
            p.parskip.natural += v.natural;
            p.parskip.stretch += v.stretch;
            p.parskip.shrink += v.shrink;
        } else {
            p.parskip = v;
        }
        return;
    }
    let slot = match name {
        "paperwidth" => &mut p.paperwidth,
        "paperheight" => &mut p.paperheight,
        "textwidth" => &mut p.textwidth,
        "textheight" => &mut p.textheight,
        "oddsidemargin" => &mut p.oddsidemargin,
        "evensidemargin" => &mut p.evensidemargin,
        "topmargin" => &mut p.topmargin,
        "headheight" => &mut p.headheight,
        "headsep" => &mut p.headsep,
        "footskip" => &mut p.footskip,
        "marginparwidth" => &mut p.marginparwidth,
        "marginparsep" => &mut p.marginparsep,
        "columnsep" => &mut p.columnsep,
        "parindent" => &mut p.parindent,
        "columnseprule" => &mut p.columnseprule,
        _ => return,
    };
    if add {
        *slot += v.natural;
    } else {
        *slot = v.natural;
    }
}

/// `\documentclass[opts]{...}` options, if the source has a class line.
pub fn class_options(source: &str) -> Option<String> {
    let at = find_command(source, "documentclass")?;
    let rest = &source[at + "\\documentclass".len()..];
    let rest = rest.trim_start();
    if let Some(inner) = rest.strip_prefix('[') {
        let end = inner.find(']')?;
        Some(inner[..end].to_string())
    } else {
        Some(String::new())
    }
}

pub fn class_size(options: &str) -> u32 {
    options
        .split(',')
        .filter_map(|o| o.trim().strip_suffix("pt"))
        .filter_map(|n| n.parse::<u32>().ok())
        .find(|n| matches!(n, 10 | 11 | 12))
        .unwrap_or(10)
}

/// `\setcounter{<name>}{<n>}`, the last one in the source.
pub fn counter(source: &str, name: &str) -> Option<u8> {
    let mut from = 0;
    let mut value = None;
    while let Some(at) = find_command(&source[from..], "setcounter") {
        let abs = from + at;
        let rest = source[abs + "\\setcounter".len()..].trim_start();
        if let Some(r) = rest.strip_prefix('{').and_then(|r| r.strip_prefix(name)).and_then(|r| r.strip_prefix('}')) {
            if let Some(r) = r.trim_start().strip_prefix('{') {
                if let Some(end) = r.find('}') {
                    value = r[..end].trim().parse::<u8>().ok().or(value);
                }
            }
        }
        from = abs + 1;
    }
    value
}

/// `\setlength{\parindent}{<dim>}` in points; `em` is resolved against the
/// body size.
pub fn parindent(source: &str, size: u32) -> Option<f64> {
    setlength(source, "parindent", size)
}

/// `\setlength{\parskip}{<dimen>}` in the source, in points (the compiler
/// reports the preamble command and drops it; LaTeX evaluates `em`/`ex`
/// in the class's `\normalsize`).
pub fn parskip(source: &str, size: u32) -> Option<f64> {
    setlength(source, "parskip", size)
}

/// The last `\setlength{\<name>}{<dimen>}` of the source, in points.
/// A paragraph that holds nothing but one `longtable` (and `\label`s):
/// takes the table out, leaving the labels behind for the caller.
fn lone_longtable(parts: &mut Vec<ParaPart>) -> Option<Box<crate::table::TableItem>> {
    let [ParaPart::Lines(items)] = &parts[..] else { return None };
    let mut table = None;
    for item in items {
        match item {
            Item::Table(t) if t.longtable.is_some() && table.is_none() => table = Some(t.clone()),
            Item::Label { .. } => {}
            // A space either side of the box is the paragraph's own
            // `\parskip`/`\par` material, which the block replaces.
            Item::Space { .. } => {}
            _ => return None,
        }
    }
    table
}

fn setlength(source: &str, name: &str, size: u32) -> Option<f64> {
    setlength_in(source, name, size, None)
}

/// [`setlength`] with the document's own `em`/`ex` ([`ec_em_ex`]).
fn setlength_in(source: &str, name: &str, size: u32, em_ex: Option<(f64, f64)>) -> Option<f64> {
    let needle = format!("{{\\{name}}}");
    let mut from = 0;
    let mut found = None;
    while let Some(at) = find_command(&source[from..], "setlength") {
        let abs = from + at;
        let rest = source[abs + "\\setlength".len()..].trim_start();
        if let Some(r) = rest.strip_prefix(needle.as_str()) {
            if let Some(r) = r.trim_start().strip_prefix('{') {
                if let Some(end) = r.find('}') {
                    found = parse_dimen_in(&r[..end], size, em_ex).or(found);
                }
            }
        }
        from = abs + 1;
    }
    found
}

/// The class size (`10`/`11`/`12`) whose `\normalsize` is `body_pt`.
pub fn class_size_of(body_pt: f64) -> u32 {
    if body_pt >= 11.9 {
        12
    } else if body_pt >= 10.9 {
        11
    } else {
        10
    }
}

/// A TeX `<dimen>` in points; `em`/`ex` are those of the class's
/// `\normalsize` (`size` is the class size).
pub fn parse_dimen_pt(s: &str, size: u32) -> Option<f64> {
    parse_dimen(s, size)
}

fn parse_dimen(s: &str, size: u32) -> Option<f64> {
    parse_dimen_in(s, size, None)
}

/// `em`/`ex` of the body font a document's own preamble and `\setlist`
/// keys are evaluated in, when it differs from the class size.
/// `\usepackage[T1]{fontenc}` without `lmodern` selects `t1cmr.fd`'s EC
/// fonts ([`Family::ComputerModern`](crate::fonts::Family)) before the
/// user's `\setlength`s run, and TeX's `em`/`ex` are that font's
/// `\fontdimen6`/`\fontdimen5`. `tftopl` (TeX Live 2026): ecrm1000 QUAD
/// 0.999756 XHEIGHT 0.43045, ecrm1095 QUAD 0.994328 XHEIGHT 0.4304495,
/// ecrm1200 QUAD 0.978928 XHEIGHT 0.43045; pdflatex reports
/// `\setlength{\parskip}{0.65em}` as 7.07704pt at 11pt. Class-load values
/// (article's `\labelsep .5em`) were evaluated in OT1 `cmr` and keep
/// [`parse_dimen`]'s class size.
fn ec_em_ex(size: u32, family: crate::fonts::Family) -> Option<(f64, f64)> {
    if family != crate::fonts::Family::ComputerModern {
        return None;
    }
    let (design, quad, xheight) = match size {
        12 => (12.0, 0.978928, 0.43045),
        11 => (10.949997, 0.994328, 0.4304495),
        _ => (10.0, 0.999756, 0.43045),
    };
    Some((design * quad, design * xheight))
}

/// [`parse_dimen`] with explicit `em`/`ex` (points) when `em_ex` is set.
fn parse_dimen_in(s: &str, size: u32, em_ex: Option<(f64, f64)>) -> Option<f64> {
    let s = s.trim();
    let split = s.find(|c: char| c.is_ascii_alphabetic())?;
    let (num, unit) = s.split_at(split);
    let v: f64 = num.trim().parse().ok()?;
    let body = match size {
        12 => 12.0,
        11 => 10.95,
        _ => 10.0,
    };
    let (em, ex) = em_ex.unwrap_or((body, body * 0.430556));
    Some(match unit.trim() {
        "pt" => v,
        "em" => v * em,
        "ex" => v * ex,
        "in" => v * 72.27,
        "cm" => v * 72.27 / 2.54,
        "mm" => v * 72.27 / 25.4,
        "bp" => v * 72.27 / 72.0,
        _ => return None,
    })
}

/// The vertical glue of one list level, in points at the class size:
/// article's `\@list<i>` values (`document-style`), with the source's
/// `\setlist[<env>]{topsep=..,itemsep=..,parsep=..,partopsep=..}`
/// overrides for `env` (enumitem evaluates `em`/`ex` in `\normalsize`).
#[derive(Debug, Clone, Copy)]
struct ListSeps {
    topsep: f64,
    partopsep: f64,
    itemsep: f64,
    parsep: f64,
    /// The same four with their stretch and shrink. LaTeX's list skips are
    /// glue, not kerns (`\topsep 8\p@ \@plus2\p@ \@minus4\p@`,
    /// `\parsep 4\p@ \@plus2\p@ \@minus\p@` at 10pt), and the page
    /// builder needs that flexibility: dropping it makes every page carry
    /// less `\pagestretch`/`\pageshrink` than pdfTeX's and the break
    /// decisions diverge.
    topsep_skip: crate::style::Skip,
    partopsep_skip: crate::style::Skip,
    itemsep_skip: crate::style::Skip,
    parsep_skip: crate::style::Skip,
}

fn list_seps(source: &str, env: &str, depth: usize, size: u32, style: &Stylesheet) -> ListSeps {
    list_seps_with(source, env, depth, size, style, "")
}

/// [`list_seps`] with the keys of the list's own `\begin{<env>}[<keys>]`
/// optional argument applied after every `\setlist` (enumitem: `nosep`
/// zeroes `topsep`/`partopsep`/`itemsep`/`parsep`, `noitemsep` zeroes
/// `itemsep`/`parsep`).
fn list_seps_with(source: &str, env: &str, depth: usize, size: u32, style: &Stylesheet, begin_keys: &str) -> ListSeps {
    let base = match size {
        12 => flashtex_document_style::BaseSize::Pt12,
        11 => flashtex_document_style::BaseSize::Pt11,
        _ => flashtex_document_style::BaseSize::Pt10,
    };
    let class = flashtex_document_style::list_level(base, depth as u8);
    let skip = |s: flashtex_document_style::Skip| crate::style::Skip::new(s.pt, s.plus, s.minus);
    let mut seps = ListSeps {
        topsep: class.topsep.pt,
        partopsep: class.partopsep.pt,
        itemsep: class.itemsep.pt,
        parsep: class.parsep.pt,
        topsep_skip: skip(class.topsep),
        partopsep_skip: skip(class.partopsep),
        itemsep_skip: skip(class.itemsep),
        parsep_skip: skip(class.parsep),
    };
    if depth == 1 {
        // The stylesheet's level-1 values are the ones the typesetter
        // reads for `\topsep`; keep both readings identical.
        seps.topsep = style.topsep.natural;
        seps.partopsep = style.partopsep.natural;
        seps.parsep = style.parsep.natural;
        seps.topsep_skip = style.topsep;
        seps.partopsep_skip = style.partopsep;
        seps.parsep_skip = style.parsep;
        seps.itemsep_skip = style.parsep;
        seps.itemsep = style.parsep.natural;
    }
    let calls = setlist_calls(source);
    let all_keys = calls.iter().filter(|(envs, _)| setlist_names(envs, env)).map(|(_, keys)| *keys).chain(std::iter::once(begin_keys));
    for keys in all_keys {
        for (key, value) in list_keys(keys) {
            let set_parsep = |seps: &mut ListSeps, pt: f64| {
                seps.parsep = pt;
                seps.parsep_skip = crate::style::Skip::fixed(pt);
            };
            let set_itemsep = |seps: &mut ListSeps, pt: f64| {
                seps.itemsep = pt;
                seps.itemsep_skip = crate::style::Skip::fixed(pt);
            };
            match key {
                "nosep" => {
                    seps.topsep = 0.0;
                    seps.topsep_skip = crate::style::Skip::default();
                    seps.partopsep = 0.0;
                    seps.partopsep_skip = crate::style::Skip::default();
                    set_itemsep(&mut seps, 0.0);
                    set_parsep(&mut seps, 0.0);
                }
                "noitemsep" => {
                    set_itemsep(&mut seps, 0.0);
                    set_parsep(&mut seps, 0.0);
                }
                _ => {
                    // `em`/`ex` are the EC body font's, as pdfTeX resolves
                    // \setlength/\setlist lengths (hw-residuals-2).
                    let Some(pt) = parse_dimen_in(value, size, ec_em_ex(size, style.family)) else { continue };
                    match key {
                        "topsep" => {
                            seps.topsep = pt;
                            seps.topsep_skip = crate::style::Skip::fixed(pt);
                        }
                        "partopsep" => {
                            seps.partopsep = pt;
                            seps.partopsep_skip = crate::style::Skip::fixed(pt);
                        }
                        "itemsep" => set_itemsep(&mut seps, pt),
                        "parsep" => set_parsep(&mut seps, pt),
                        _ => {}
                    }
                }
            }
        }
    }
    seps
}

/// The `\list`/`\trivlist` environments whose `\item`s the compiler reports
/// as `CBlock::ListItem` and whose `\@trivlist` glue this module derives.
/// `description` is one of them: article.cls builds it with `\list{}{...}`
/// exactly like `itemize`, so it carries the same `\topsep`/`\partopsep`/
/// `\itemsep`/`\parsep` and the same closing `\@endparenv` skip. Only its
/// `\labelwidth\z@`, `\itemindent-\leftmargin` and `\descriptionlabel`
/// differ, and those are the typesetter's business ([`ListGeom::description`]).
pub(crate) const LIST_ENVS: [&str; 4] = ["itemize", "enumerate", "description", "thebibliography"];

/// Whether `rest` (starting at a `\begin`) opens one of [`LIST_ENVS`].
fn list_env_after_begin(rest: &str) -> bool {
    let after = rest.strip_prefix("\\begin").unwrap_or(rest).trim_start();
    LIST_ENVS.iter().any(|env| after.starts_with(&format!("{{{env}}}")))
}

/// `\endtrivlist` for every `\end{itemize}`/`\end{enumerate}` in `gap`
/// (which starts at byte `gap_start` of `source`), innermost first: when
/// the list leaves a positive `\lastskip` it becomes `\lastskip +
/// \parskip - \@outerparskip` — the closing list's `\parsep` less the
/// `\parskip` outside it (the enclosing list's `\parsep`, or the
/// document's). The summed change, in points.
fn list_end_adjust(source: &str, gap_start: usize, gap: &str, size: u32, style: &Stylesheet) -> f64 {
    let mut adjust = 0.0;
    let mut from = 0;
    while let Some(at) = find_command(&gap[from..], "end") {
        let abs = from + at;
        from = abs + 1;
        let rest = gap[abs + "\\end".len()..].trim_start();
        if !LIST_ENVS.iter().any(|env| rest.starts_with(&format!("{{{env}}}"))) {
            continue;
        }
        let stack = list_stack_at(source, gap_start + abs);
        let Some(&(env, _)) = stack.last() else { continue };
        let depth = stack.len();
        let parsep = list_seps(source, env, depth, size, style).parsep;
        let outer = if depth > 1 { list_seps(source, stack[depth - 2].0, depth - 1, size, style).parsep } else { style.parskip.natural };
        adjust += parsep - outer;
    }
    adjust
}

/// The environment of the last `\end{itemize}`/`\end{enumerate}` in `gap`.
/// The `\endtrivlist` glue (`\addvspace\@topsepadd`) of the list that
/// `run` closes at its very end, in points; 0 when it closes none.
///
/// [`adapt`] gives this skip to the block that *follows* the list, which is
/// how LaTeX contributes it. A float body's last content run has no block
/// after it -- `\caption` is set by `\@makecaption` and the box then ends
/// -- so `crate::floats` asks for it here rather than re-deriving the list
/// parameters of a second copy.
pub(crate) fn list_end_skip(source: &str, run: &std::ops::Range<usize>, body_size_pt: f64, style: &Stylesheet) -> f64 {
    let text = &source[run.start..run.end];
    let Some(at) = rfind_command(text, "end") else { return 0.0 };
    let rest = text[at + "\\end".len()..].trim_start();
    let Some(env) = LIST_ENVS
        .into_iter()
        .find(|env| rest.strip_prefix('{').is_some_and(|r| r.starts_with(&format!("{env}}}"))))
    else {
        return 0.0;
    };
    // Only when the `\end` is the last thing in the run: material after it
    // is a block of its own, and the adapter has already given it the skip.
    if !rest["{}".len() + env.len()..].trim().is_empty() {
        return 0.0;
    }
    // `\@topsepadd` is what `\@trivlist` computed when the list opened:
    // `\topsep`, plus `\partopsep` when its own `\begin` was read in
    // vertical mode (the run's start, or after a blank line or `\par`).
    // An alignment declaration sets no material, so it does not leave
    // vertical mode.
    let opened = text[..at].rfind(&format!("\\begin{{{env}}}")).unwrap_or(0);
    let before = text[..opened].replace("\\centering", "").replace("\\raggedright", "").replace("\\raggedleft", "");
    let vmode = before.trim().is_empty() || has_blank_line(&before) || find_command(&before, "par").is_some();
    let stack = list_stack_at(source, run.start + at);
    let begin_keys = stack.last().map_or("", |(e, keys)| if *e == env && *e != "thebibliography" { keys } else { "" });
    let size = if body_size_pt >= 11.5 {
        12
    } else if body_size_pt >= 10.5 {
        11
    } else {
        10
    };
    let seps = list_seps_with(source, env, 1, size, style, begin_keys);
    seps.topsep + if vmode { seps.partopsep } else { 0.0 }
}

fn gap_has_list_end(gap: &str) -> Option<&'static str> {
    let end = rfind_command(gap, "end")?;
    let rest = gap[end + "\\end".len()..].trim_start();
    LIST_ENVS.into_iter().find(|env| rest.strip_prefix('{').is_some_and(|r| r.starts_with(&format!("{env}}}"))))
}

/// The `\setlist[<envs>]{<keys>}` calls of `source`, in order:
/// `(environment list or "" for all, keys)`.
fn setlist_calls(source: &str) -> Vec<(&str, &str)> {
    let mut calls = Vec::new();
    let mut from = 0;
    while let Some(at) = find_command(&source[from..], "setlist") {
        let abs = from + at;
        from = abs + 1;
        let rest = &source[abs + "\\setlist".len()..];
        let rest = rest.strip_prefix('*').unwrap_or(rest).trim_start();
        let (envs, rest) = match rest.strip_prefix('[') {
            Some(r) => match r.find(']') {
                Some(close) => (&r[..close], r[close + 1..].trim_start()),
                None => continue,
            },
            None => ("", rest),
        };
        if !rest.starts_with('{') {
            continue;
        }
        let Some(close) = matching_brace(rest.as_bytes(), 0) else { continue };
        calls.push((envs, &rest[1..close]));
    }
    calls
}

/// enumitem `key=value` pairs (a key without `=` gets an empty value).
fn list_keys(keys: &str) -> impl Iterator<Item = (&str, &str)> {
    keys.split(',').map(str::trim).filter(|k| !k.is_empty()).map(|k| match k.split_once('=') {
        Some((key, value)) => (key.trim(), value.trim()),
        None => (k, ""),
    })
}

/// Whether a `\setlist[<envs>]` list names `env` (enumitem also accepts
/// level numbers there, which apply to every environment).
fn setlist_names(envs: &str, env: &str) -> bool {
    envs.trim().is_empty() || envs.split(',').map(str::trim).any(|e| e == env || e.parse::<u8>().is_ok())
}

/// The `itemize`/`enumerate` environments open at byte `at` of `source`,
/// outermost first: `(environment, `\begin` optional argument)`.
fn list_stack_at(source: &str, at: usize) -> Vec<(&str, &str)> {
    let mut stack: Vec<(&str, &str)> = Vec::new();
    let mut from = 0;
    while from < at {
        let next_begin = find_command(&source[from..at], "begin").map(|i| from + i);
        let next_end = find_command(&source[from..at], "end").map(|i| from + i);
        let (pos, is_begin) = match (next_begin, next_end) {
            (Some(b), Some(e)) if b < e => (b, true),
            (Some(b), None) => (b, true),
            (_, Some(e)) => (e, false),
            (None, None) => break,
        };
        from = pos + 1;
        let rest = &source[pos + if is_begin { "\\begin".len() } else { "\\end".len() }..];
        let rest = rest.trim_start();
        let Some(inner) = rest.strip_prefix('{') else { continue };
        let Some(close) = inner.find('}') else { continue };
        let env = inner[..close].trim();
        if !LIST_ENVS.contains(&env) {
            continue;
        }
        if is_begin {
            let after = inner[close + 1..].trim_start();
            // `thebibliography`'s "options" are its widest-label argument
            // (`\begin{thebibliography}{99}` -> `99`).
            let options = match (env, after.strip_prefix('['), after.strip_prefix('{')) {
                ("thebibliography", _, Some(o)) => o.find('}').map_or("", |c| &o[..c]),
                ("thebibliography", _, None) => "",
                (_, Some(o), _) => o.find(']').map_or("", |c| &o[..c]),
                _ => "",
            };
            stack.push((env, options));
        } else if stack.last().is_some_and(|(open, _)| *open == env) {
            stack.pop();
        }
    }
    stack
}

/// Whether the text at `span` was generated by a `\cite`-family command
/// rather than copied from the source.
///
/// Every run of a citation carries the whole command's span, and the
/// generated text can coincidentally be exactly as long as the command:
/// `\citet{knuthplass1981}` is 22 bytes and sets the 22 characters of
/// "Knuth and Plass (1981)". A blank in *generated* text stands for a space
/// token — interword glue of `\fontdimen2` — while a blank in the source's
/// own bytes is kept as a character, so that coincidence would set the
/// citation's spaces as blank glyphs (LMRoman10's 0.5 em instead of cmr10's
/// 0.33333 em: 1.825 pt too wide per space at 11 pt, and cumulative).
fn generated_citation(source: &str, span: Span) -> bool {
    let Some(text) = source.get(span.start..span.end) else {
        return false;
    };
    let Some(rest) = text.strip_prefix('\\') else {
        return false;
    };
    let end = rest
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(rest.len());
    // natbib's `\Citet`/`\Citep`/... uppercase the author list, not the name
    // of the command family.
    let name = match rest[..end].strip_prefix("Cite") {
        Some(tail) => format!("cite{tail}"),
        None => rest[..end].to_string(),
    };
    matches!(
        name.as_str(),
        "cite"
            | "citet"
            | "citep"
            | "citealt"
            | "citealp"
            | "citeauthor"
            | "citefullauthor"
            | "citeyear"
            | "citeyearpar"
            | "citenum"
            | "citetext"
    )
}

/// Whether the document loads natbib in its author-year mode, which is the
/// only natbib setting that changes `thebibliography`'s own geometry: its
/// `\@biblabel` is `\hfill` (no label at all) and `\@bibsetup` is
/// `\NAT@bibsetup` (`\leftmargin\bibhang`, `\itemindent-\leftmargin`).
/// `numbers`/`super` keep the class's `[n]` label and label-width margin.
///
/// The options are read the way natbib resolves them: `\ProcessOptions`
/// executes them in *declaration* order, and `numbers`/`super` come before
/// `authoryear`, so `[authoryear,numbers]` and `[numbers,authoryear]` are
/// both author-year — `authoryear` is declared last of the three and wins.
pub(crate) fn natbib_author_year(source: &str) -> bool {
    let Some(options) = natbib_options(source) else {
        return false;
    };
    let given: Vec<&str> = options.split(',').map(str::trim).collect();
    if given.contains(&"authoryear") {
        return true;
    }
    !given.contains(&"numbers") && !given.contains(&"super")
}

/// The `[...]` of the `\usepackage` that loads natbib, or `None` when the
/// document does not load it.
fn natbib_options(source: &str) -> Option<String> {
    let mut from = 0;
    while let Some(at) = find_command(&source[from..], "usepackage").map(|i| from + i) {
        let rest = &source[at + "\\usepackage".len()..];
        let rest = rest.trim_start();
        let (options, rest) = match rest.strip_prefix('[') {
            Some(inner) => match inner.find(']') {
                Some(close) => (inner[..close].to_string(), inner[close + 1..].trim_start()),
                None => (String::new(), rest),
            },
            None => (String::new(), rest),
        };
        if let Some(inner) = rest.strip_prefix('{') {
            if let Some(close) = inner.find('}') {
                if inner[..close]
                    .split(',')
                    .map(str::trim)
                    .any(|package| package == "natbib")
                {
                    return Some(options);
                }
            }
        }
        from = at + "\\usepackage".len();
    }
    None
}

/// article's `\leftmargin<i>` for nesting `depth` (1-based), in em of
/// the body font (`\leftmarginv`/`vi` are 1em).
fn article_leftmargin_em(depth: usize) -> f64 {
    [2.5, 2.2, 1.87, 1.7, 1.0, 1.0][depth.clamp(1, 6) - 1]
}

/// The widest label enumitem assumes for the `leftmargin=*` computation:
/// a `label=` key (its `\alph*`-style counter replaced by `m`/`M`/
/// `viii`/`VIII`/`0`), a shortlabels template (`(a)` -> `(m)`), or the
/// class's own label for this depth.
fn widest_label(env: &str, depth: usize, label_key: Option<&str>, template: Option<&str>) -> String {
    if let Some(label) = label_key {
        return [("\\alph*", "m"), ("\\Alph*", "M"), ("\\roman*", "viii"), ("\\Roman*", "VIII"), ("\\arabic*", "0")]
            .iter()
            .fold(label.to_string(), |text, (command, widest)| text.replace(command, widest));
    }
    if env == "itemize" {
        return match depth {
            1 => "•",
            2 => "–",
            3 => "∗",
            _ => "·",
        }
        .to_string();
    }
    if let Some(template) = template {
        if let Some((index, style)) = template.char_indices().find(|(_, c)| "aAiI1".contains(*c)) {
            let widest = match style {
                'a' => "m",
                'A' => "M",
                'i' => "viii",
                'I' => "VIII",
                _ => "0",
            };
            return format!("{}{}{}", &template[..index], widest, &template[index + 1..]);
        }
        return template.to_string();
    }
    match depth {
        1 => "0.",
        2 => "(m)",
        3 => "viii.",
        _ => "M.",
    }
    .to_string()
}

/// `\leftmargin` of every list open at byte `at` (outermost first): the
/// class's `\leftmargin<i>` unless a `\setlist` naming the environment or
/// the `\begin` options set enumitem's `leftmargin` (`*` = the widest
/// label's width plus `\labelsep`; a `<dimen>` as given).
fn list_margins(source: &str, at: usize, size: u32, natbib_bib: bool) -> Vec<ListMargin> {
    let calls = setlist_calls(source);
    let class_margin = |depth: usize| ListMargin::Fixed(parse_dimen(&format!("{}em", article_leftmargin_em(depth)), size).unwrap_or(0.0));
    list_stack_at(source, at)
        .iter()
        .enumerate()
        .map(|(i, (env, options))| {
            let depth = i + 1;
            if *env == "thebibliography" {
                // natbib's author-year `\@bibsetup` (`\NAT@bibsetup`,
                // natbib.sty line 642) replaces the class's label-width
                // geometry with `\leftmargin\bibhang`; its `\@biblabel` is
                // `\hfill`, so there is no label to measure. Under `numbers`
                // natbib keeps `\NAT@bibsetnum`, which is the class rule.
                if natbib_bib {
                    return ListMargin::Em(1.0);
                }
                // latex.ltx/article.cls `\thebibliography`:
                // `\settowidth\labelwidth{\@biblabel{#1}}`,
                // `\leftmargin\labelwidth \advance\leftmargin\labelsep`.
                return ListMargin::Widest(format!("[{}]", options.trim()));
            }
            let mut leftmargin: Option<&str> = None;
            let mut label_key: Option<&str> = None;
            let begin_keys = options.contains('=');
            let all_keys = calls
                .iter()
                .filter(|(envs, _)| setlist_names(envs, env))
                .map(|(_, keys)| *keys)
                .chain(begin_keys.then_some(*options));
            for keys in all_keys {
                for (key, value) in list_keys(keys) {
                    match key {
                        "leftmargin" => leftmargin = Some(value),
                        "label" => label_key = Some(value),
                        _ => {}
                    }
                }
            }
            let template = (!begin_keys && !options.is_empty()).then_some(*options);
            match leftmargin {
                Some("*") => ListMargin::Widest(widest_label(env, depth, label_key, template)),
                Some(dimen) => parse_dimen(dimen, size).map_or_else(|| class_margin(depth), ListMargin::Fixed),
                None => class_margin(depth),
            }
        })
        .collect()
}

/// Byte offset of `\name` (as a whole control word, outside comments).
pub(crate) fn find_command(source: &str, name: &str) -> Option<usize> {
    let needle = format!("\\{name}");
    let bytes = source.as_bytes();
    let mut i = 0;
    let mut in_comment = false;
    while i < bytes.len() {
        let c = bytes[i];
        if in_comment {
            if c == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        if c == b'%' {
            in_comment = true;
            i += 1;
            continue;
        }
        if c == b'\\' {
            if source[i..].starts_with(&needle) {
                let after = i + needle.len();
                if after >= bytes.len() || !bytes[after].is_ascii_alphabetic() {
                    return Some(i);
                }
            }
            i += 2;
            continue;
        }
        i += 1;
    }
    None
}

/// Whether `\sloppy` is in force for the whole document: a `\sloppy` outside
/// every brace group of the entry source (preamble or body). One inside a
/// group (`{\sloppy ...}`) is local and not applied; `sloppypar` is not read.
pub fn document_sloppy(source: &str) -> bool {
    let mut from = 0;
    while let Some(at) = find_command(&source[from..], "sloppy") {
        let abs = from + at;
        if brace_depth(&source[..abs]) == 0 {
            return true;
        }
        from = abs + 1;
    }
    false
}

/// Unclosed `{` groups in `prefix` (escaped braces and comments skipped).
fn brace_depth(prefix: &str) -> i64 {
    let bytes = prefix.as_bytes();
    let (mut depth, mut i, mut comment) = (0i64, 0usize, false);
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => comment = false,
            _ if comment => {}
            b'%' => comment = true,
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => depth = (depth - 1).max(0),
            _ => {}
        }
        i += 1;
    }
    depth
}

/// The `\@topsep`/`\@topsepadd` of a theorem-like environment, read off
/// pdfTeX's own vertical list (TeX Live 2025; oracle only, never in the
/// product path — the quoted `\showoutput` glue is in
/// `tests/amsthm_topsep.rs`).
///
/// * a `\newtheorem` environment gets `\topsep` on both sides, because
///   `\@thm` assigns `\@topsep`/`\@topsepadd` from `\thm@preskip`/
///   `\thm@postskip` and `\thm@space@setup` sets both to `\topsep`. The
///   trace is `\glue 8.0 plus 2.0 minus 4.0` / `9.0 plus 3.0 minus 5.0` /
///   `10.0 plus 4.0 minus 6.0` at a 10/11/12pt base: `\topsep` exactly, with
///   no `\partopsep` and no `\parskip`.
/// * `proof` is not a `\@thm`. It is an ordinary `\trivlist` opened after
///   an explicit `\par` (so in vertical mode) under amsthm's own
///   `\topsep6\p@\@plus6\p@`, so its closing `\@topsepadd` is that 6pt
///   plus `\partopsep`: the trace is `8.0 plus 7.0 minus 1.0`,
///   `9.0 plus 7.0 minus 1.0`, `9.0 plus 8.0 minus 2.0` — equal to `\topsep`
///   at a 10pt and 11pt base and 1pt short of it at 12pt.
///
/// Its *opening* skip is left at `\topsep`: `\addvspace` keeps the larger of
/// the new skip and `\lastskip`, and the closing skip of whatever precedes a
/// `proof` is at least that in every arrangement measured here.
fn theorem_skips(style: &Stylesheet, proof: bool) -> EnvSkips {
    let topsep = style.topsep;
    if !proof {
        return EnvSkips { open: topsep, close: topsep };
    }
    let p = style.partopsep;
    EnvSkips {
        open: topsep,
        close: crate::style::Skip::new(6.0 + p.natural, 6.0 + p.stretch, p.shrink),
    }
}

/// The environments amsthm sets as a `\trivlist` holding a single `\item`:
/// every `\newtheorem`/`\newtheorem*` declaration in the sources plus the
/// fixed `proof`. See [`opens_theorem_item`] for what that costs the first
/// line.
fn theorem_environments(texts: &[&str]) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    out.insert("proof".to_string());
    for text in texts {
        let mut from = 0;
        while let Some(at) = find_command(&text[from..], "newtheorem") {
            let at = from + at;
            let rest = &text[at + "\\newtheorem".len()..];
            let rest = rest.strip_prefix('*').unwrap_or(rest).trim_start();
            if let Some(name) = rest.strip_prefix('{').and_then(|r| r.split_once('}')).map(|(n, _)| n.trim()) {
                if !name.is_empty() {
                    out.insert(name.to_string());
                }
            }
            from = at + 1;
        }
    }
    out
}

/// Whether the last `\begin{...}` in `gap` opens a theorem-like environment,
/// so the paragraph after it is that environment's `\item`.
///
/// amsthm sets theorem-like environments as `\trivlist` + `\item[<head>]`.
/// `\trivlist` leaves `\itemindent` at `\z@`, and `\@item`'s `\everypar`
/// takes the `\parindent` box back off the first line (`\setbox\z@\lastbox`)
/// before unboxing `\@labels` — whose leading `\hskip\itemindent \hskip
/// -\labelwidth \hskip -\labelsep` is `-\labelsep` here, exactly cancelling
/// the `\hskip\labelsep` the head box starts with. So the head sits flush on
/// the left margin and the first line is *not* indented; only the following
/// paragraphs of the same environment take the ambient `\parindent`.
fn opens_theorem_item<'t>(text: &'t str, gap_start: usize, at: usize, envs: &std::collections::HashSet<String>) -> Option<&'t str> {
    if gap_start > at || at > text.len() || !text.is_char_boundary(gap_start) || !text.is_char_boundary(at) {
        return None;
    }
    // The compiler gives the theorem head inline the `\begin` command's own
    // span, so the opener is usually *at* the paragraph's first span rather
    // than in the gap before it; accept either.
    let begin = if text[at..].starts_with("\\begin") {
        Some(at)
    } else {
        rfind_command(&text[gap_start..at], "begin").map(|r| gap_start + r)
    };
    let begin = begin?;
    text[begin..]
        .split_once('{')
        .and_then(|(_, rest)| rest.split_once('}'))
        .map(|(name, _)| name.trim())
        .filter(|name| envs.contains(*name))
}

/// Whether byte `at` lies inside a theorem-like environment: the `\begin`/
/// `\end` pairs before it are matched off and a theorem-like name is left
/// open. amsthm's head font (`\bfseries`, or `\itshape` for `remark` and
/// `proof`) and the `plain` style's italic body are declared by the package,
/// not written in the source at the head's span, so the weights of a
/// theorem's words come from the compiler's scoping instead of the source's
/// own brace groups.
fn in_theorem_environment(text: &str, at: usize, envs: &std::collections::HashSet<String>) -> bool {
    if at > text.len() || !text.is_char_boundary(at) {
        return false;
    }
    let head = &text[..at];
    let mut open: Vec<&str> = Vec::new();
    let mut from = 0usize;
    loop {
        let b = find_command(&head[from..], "begin").map(|r| (from + r, true));
        let e = find_command(&head[from..], "end").map(|r| (from + r, false));
        let (pos, is_begin) = match (b, e) {
            (Some(x), Some(y)) => {
                if x.0 <= y.0 {
                    x
                } else {
                    y
                }
            }
            (Some(x), None) => x,
            (None, Some(y)) => y,
            (None, None) => break,
        };
        let name = head[pos..].split_once('{').and_then(|(_, r)| r.split_once('}')).map(|(n, _)| n.trim());
        if let Some(name) = name {
            if is_begin {
                open.push(name);
            } else if open.last() == Some(&name) {
                open.pop();
            }
        }
        from = pos + 1;
    }
    open.iter().any(|n| envs.contains(*n))
}

/// Byte offset of the last `\<name>` in `source` outside comments.
fn rfind_command(source: &str, name: &str) -> Option<usize> {
    let mut last = None;
    let mut from = 0;
    while let Some(at) = find_command(&source[from..], name) {
        last = Some(from + at);
        from += at + 1;
    }
    last
}

/// Whether `source` holds a blank line (TeX's `\par` from an empty line):
/// two newlines with only blanks between them.
fn has_blank_line(source: &str) -> bool {
    let mut newlines = 0;
    for c in source.chars() {
        match c {
            '\n' => {
                newlines += 1;
                if newlines >= 2 {
                    return true;
                }
            }
            ' ' | '\t' | '\r' => {}
            _ => newlines = 0,
        }
    }
    false
}

/// One font command's content interval: start and end byte, the NFSS
/// command, and whether LaTeX's `\maybe@ic` italic correction can follow
/// its end (see [`Styles::closes_at`]).
type StyleInterval = (usize, usize, crate::nfss::Command, bool);

/// `\url{...}` and `\nolinkurl{...}` (`url.sty`, which `hyperref` loads):
/// their argument is read as *raw source bytes*, and the URL is set in the
/// typewriter family.
///
/// Two things follow for [`style_intervals`], and both are why these are not
/// ordinary [`text_font_command`] entries:
///
/// 1. The argument is **opaque**. url.sty makes every character of a URL
///    "other" before it is read, so `%`, `#`, `_`, `&` and `\` inside it are
///    literal (the compiler does the same in `parser::url_argument`). The
///    style scan must not treat a `%` in `\url{.../a%20b}` as a comment, or
///    everything to the end of that line — including a following `\textbf{}`
///    — silently loses its style.
/// 2. The interval covers the **whole command**, from the backslash through
///    the closing brace, not just the braced argument. The compiler gives
///    every run it splits a URL into the span of the entire `\url{...}`
///    (`parser::push_url_text`), and the style is looked up at `span.start`,
///    which is the backslash.
fn url_command(name: &str) -> bool {
    matches!(name, "url" | "nolinkurl")
}

/// A verbatim construct's extent in the source: `whole` is every byte the
/// style interval must cover and the scanner must skip, `body` is the
/// literal text inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VerbatimSpan {
    whole: (usize, usize),
    body: (usize, usize),
}

/// `\verb`/`\verb*` and `\lstinline`: a *delimited* argument, the next
/// character after the command (and after `\lstinline`'s optional
/// `[...]`) being the delimiter, which then closes the argument.
///
/// Like [`url_command`] these cannot be [`text_font_command`] entries,
/// for the same two reasons plus a third:
///
/// 1. The argument is **opaque** — more so than a URL's, because `\verb`
///    ends at a *character*, not a brace. `\verb|{|` and `\verb|%|` are
///    legal, and scanning them as LaTeX corrupts the `groups` stack and
///    starts a comment that eats the rest of the line's styles.
/// 2. The interval covers the **whole command**, because the compiler
///    gives `Inline::Verbatim` the span of the entire `\verb|...|` and the
///    style is looked up at `span.start`.
/// 3. The body is **literal** (`TextStyle::literal`), which no font
///    command implies: `\texttt` ligates `--` and `\verb` must not.
fn verb_command(name: &str) -> bool {
    matches!(name, "verb" | "lstinline")
}

/// The verbatim *environments*, whose body runs to the matching
/// `\end{<name>}`. `lstlisting` and `verbatim*` take an optional `[...]`
/// after the `\begin{...}` that is read as ordinary LaTeX, not as text.
fn verbatim_environment(name: &str) -> bool {
    matches!(name, "verbatim" | "verbatim*" | "lstlisting" | "lstlisting*" | "Verbatim" | "alltt")
}

/// The `\verb`/`\verb*`/`\lstinline` starting at the backslash `at`, whose
/// control word ends at `word_end`.
///
/// The delimiter is the first character after an optional `*` and, for
/// `\lstinline`, an optional bracketed key list. LaTeX forbids a space or
/// `*` as the delimiter (`\verb` reads `\@ifstar` then one token), and an
/// unterminated `\verb` is an error there, so `None` here leaves the bytes
/// to the ordinary scan rather than swallowing the rest of the document.
fn verb_span(source: &str, at: usize, word_end: usize) -> Option<VerbatimSpan> {
    let bytes = source.as_bytes();
    let mut i = word_end;
    if bytes.get(i) == Some(&b'*') {
        i += 1;
    }
    if source[at + 1..word_end] == *"lstinline" && bytes.get(i) == Some(&b'[') {
        // A key list, read as LaTeX; only the delimited body is literal.
        let close = source[i..].find(']')? + i;
        i = close + 1;
    }
    let delim = *bytes.get(i)?;
    if delim == b' ' || delim == b'\t' || delim == b'\n' || delim == b'*' {
        return None;
    }
    let body = i + 1;
    let end = source[body..].find(delim as char)? + body;
    Some(VerbatimSpan { whole: (at, end + 1), body: (body, end) })
}

/// The `\begin{<name>}` verbatim environment starting at the backslash
/// `at`, given the environment name's bytes.
///
/// The body starts after the `\begin{...}`'s optional `[...]` argument and
/// the newline that ends that line (LaTeX's verbatim discards it), and runs
/// to the `\end{<name>}`. Verbatim environments do not nest, so the *first*
/// `\end{<name>}` closes the body — which is exactly why the body must be
/// skipped rather than scanned: a `\begin{...}` typed inside a listing is
/// text, not a group.
fn verbatim_environment_span(source: &str, at: usize, name: &str, after_name: usize) -> Option<VerbatimSpan> {
    let bytes = source.as_bytes();
    let mut i = after_name;
    if bytes.get(i) == Some(&b'[') {
        let close = matching_bracket(bytes, i)?;
        i = close + 1;
    }
    // `\begin{verbatim}` swallows the rest of its own line.
    let body = match source[i..].find('\n') {
        Some(nl) => i + nl + 1,
        None => i,
    };
    let closing = format!("\\end{{{name}}}");
    let end = source[body..].find(&closing)? + body;
    // The newline in front of `\end{...}` belongs to the terminator, not to
    // the last line of the body.
    let body_end = if end > body && bytes[end - 1] == b'\n' { end - 1 } else { end };
    Some(VerbatimSpan { whole: (at, end + closing.len()), body: (body, body_end) })
}

/// The `]` matching the `[` at `open`, counting nested brackets. Used for
/// the optional key list of `lstlisting`/`\lstinline`.
fn matching_bracket(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                // A braced value (`caption={...}`) may hold a `]`.
                let mut d = 0usize;
                while i < bytes.len() {
                    match bytes[i] {
                        b'{' => d += 1,
                        b'}' => {
                            d -= 1;
                            if d == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
            }
            b'[' => depth += 1,
            b']' => {
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

/// The end of a `\url`/`\nolinkurl` argument that starts at the `{` at
/// `open`: the matching `}`, counting nested braces and reading `\{` / `\}`
/// as literal characters rather than grouping. This mirrors
/// `compiler::parser::url_argument` byte for byte, so the interval this
/// produces covers exactly the bytes that compiler put in the URL's span.
/// `None` when the argument is never closed.
fn url_argument_end(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut i = open + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if matches!(bytes.get(i + 1), Some(b'{' | b'}')) => i += 2,
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    None
}

/// The NFSS commands of a text font command with a braced argument
/// (latex.ltx 14213-14222 `\DeclareTextFontCommand`, and `\emph`).
fn text_font_command(name: &str) -> Option<&'static [crate::nfss::Command]> {
    use crate::nfss::{Command as C, FamilyKind as F, Series as S, ShapeRequest as R};
    Some(match name {
        "textbf" => &[C::Series(S::Bx)],
        "textmd" => &[C::Series(S::M)],
        "textit" => &[C::Shape(R::It)],
        "textsl" => &[C::Shape(R::Sl)],
        "textsc" => &[C::Shape(R::Sc)],
        "textup" => &[C::Shape(R::Up)],
        "textrm" => &[C::Family(F::Rm)],
        "textsf" => &[C::Family(F::Sf)],
        "texttt" => &[C::Family(F::Tt)],
        "textnormal" => &[C::Normal],
        "emph" => &[C::Emph],
        _ => return None,
    })
}

/// The NFSS commands of a font declaration, and whether the end of its
/// group is recorded for italic correction (the declarations the pipeline
/// read before NFSS selection existed keep that behaviour). The LaTeX
/// 2.09 forms reset first: `\bf` is `\normalfont\bfseries` (latex.ltx
/// `\DeclareOldFontCommand`).
fn font_declaration(name: &str) -> Option<(&'static [crate::nfss::Command], bool)> {
    use crate::nfss::{Command as C, FamilyKind as F, Series as S, ShapeRequest as R};
    Some(match name {
        "bfseries" => (&[C::Series(S::Bx)], true),
        "itshape" => (&[C::Shape(R::It)], true),
        "slshape" => (&[C::Shape(R::Sl)], true),
        "em" => (&[C::Emph], true),
        "mdseries" => (&[C::Series(S::M)], false),
        "scshape" => (&[C::Shape(R::Sc)], false),
        "upshape" => (&[C::Shape(R::Up)], false),
        "rmfamily" => (&[C::Family(F::Rm)], false),
        "sffamily" => (&[C::Family(F::Sf)], false),
        "ttfamily" => (&[C::Family(F::Tt)], false),
        "normalfont" => (&[C::Normal], false),
        "bf" => (&[C::Normal, C::Series(S::Bx)], false),
        "it" => (&[C::Normal, C::Shape(R::It)], false),
        "sl" => (&[C::Normal, C::Shape(R::Sl)], false),
        "sc" => (&[C::Normal, C::Shape(R::Sc)], false),
        "rm" => (&[C::Normal, C::Family(F::Rm)], false),
        "sf" => (&[C::Normal, C::Family(F::Sf)], false),
        "tt" => (&[C::Normal, C::Family(F::Tt)], false),
        _ => return None,
    })
}

/// The `\baselineskip` a [`ParLeading`] selects, in points: the *second*
/// argument of the `\@setfontsize` call the declaration makes
/// (`size1x.clo`'s table, e.g. `\small` at an 11 pt base is
/// `\@setfontsize\small\xpt{12}`). `None` for `\normalsize`, whose leading
/// is the stylesheet's own.
///
/// Only the leading is taken from here. The *glyph* size of each run already
/// travels on `TextStyle::size_cpt` ([`declared_size`]), and TeX's two are
/// independent: a paragraph can be set in `\small` type at the body's
/// leading, or in body type at `\small`'s, depending only on where the
/// `\par` fell (see [`ParLeading`]).
fn par_leading_pt(leading: ParLeading, base: flashtex_document_style::BaseSize) -> Option<f64> {
    use flashtex_compiler::parser::FontSizeLevel as L;
    use flashtex_document_style::SizeName as N;
    let name = match leading? {
        L::Tiny => N::Tiny,
        L::ScriptSize => N::ScriptSize,
        L::FootnoteSize => N::FootnoteSize,
        L::Small => N::Small,
        L::Large1 => N::Large,
        L::Large2 => N::LARGE2,
        L::Large3 => N::LARGE3,
        L::Huge1 => N::Huge,
        L::Huge2 => N::HUGE2,
    };
    Some(flashtex_document_style::font_size(base, name).baselineskip.0)
}

/// The point size a `\tiny`..`\Huge` declaration selects at a class base
/// size (size10/11/12.clo), in hundredths of a point; 0 for `\normalsize`
/// (the paragraph's own size). The declaration in force comes from the
/// compiler's `TextStyle::size` (pin `b38e1884`, declaration-scoped like
/// bold/italic); the source scan below no longer reads size declarations,
/// so a size is never applied twice. The compiler's own table is the same
/// one, but it resolves against its integer class size where the pipeline
/// sets `\normalsize` at the class's real `\normalsize` (10.95pt at 11pt).
fn declared_size(level: Option<flashtex_compiler::parser::FontSizeLevel>, base: u32) -> u16 {
    use flashtex_compiler::parser::FontSizeLevel as L;
    let Some(level) = level else { return 0 };
    // tiny, scriptsize, footnotesize, small, large, Large, LARGE, huge, Huge
    let table: [[u16; 3]; 9] = [
        [500, 600, 600],
        [700, 800, 800],
        [800, 900, 1000],
        [900, 1000, 1095],
        [1200, 1200, 1440],
        [1440, 1440, 1728],
        [1728, 1728, 2074],
        [2074, 2074, 2488],
        [2488, 2488, 2488],
    ];
    let col = match base {
        11 => 1,
        12 => 2,
        _ => 0,
    };
    let row = match level {
        L::Tiny => 0,
        L::ScriptSize => 1,
        L::FootnoteSize => 2,
        L::Small => 3,
        L::Large1 => 4,
        L::Large2 => 5,
        L::Large3 => 6,
        L::Huge1 => 7,
        L::Huge2 => 8,
    };
    table[row][col]
}

/// Content intervals of the font commands in source byte offsets, in
/// document order: text font commands (`\textsf{}`, `\emph{}`, ...) over
/// their braced argument, declarations (`\scshape`, `\ttfamily`, `\bf`,
/// ...) to the end of the innermost group. Family, series and shape only:
/// size declarations are read from the compiler's `TextStyle::size` (see
/// [`declared_size`]).
/// Every verbatim construct in `source`, in order and non-overlapping.
///
/// This is a separate scan from [`style_intervals`] because the two need it
/// for different reasons — the style scan must *skip* these bytes, while the
/// item builder must know that the text it is laying out is literal — and
/// because it has to run before the style scan can trust its own comment and
/// brace state: a `%` or a `{` inside `\verb|%|` or a `lstlisting` body is a
/// character, and reading it as LaTeX silently drops the style of everything
/// after it.
fn literal_spans(source: &str) -> Vec<VerbatimSpan> {
    let bytes = source.as_bytes();
    let mut out: Vec<VerbatimSpan> = Vec::new();
    let mut i = 0;
    let mut in_comment = false;
    while i < bytes.len() {
        let c = bytes[i];
        if in_comment {
            if c == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        if c == b'%' {
            in_comment = true;
            i += 1;
            continue;
        }
        if c != b'\\' {
            i += 1;
            continue;
        }
        let rest = &source[i..];
        let word_end = i + 1 + rest[1..].bytes().take_while(u8::is_ascii_alphabetic).count();
        let name = &source[i + 1..word_end];
        if verb_command(name) {
            if let Some(span) = verb_span(source, i, word_end) {
                i = span.whole.1;
                out.push(span);
                continue;
            }
        } else if name == "begin" {
            if let Some((env, after)) = environment_name(source, word_end) {
                if verbatim_environment(env) {
                    if let Some(span) = verbatim_environment_span(source, i, env, after) {
                        i = span.whole.1;
                        out.push(span);
                        continue;
                    }
                }
            }
        }
        i = word_end.max(i + 2);
    }
    out
}

/// The environment name of a `\begin`/`\end` whose control word ends at
/// `at`, and the byte after its closing brace.
fn environment_name(source: &str, at: usize) -> Option<(&str, usize)> {
    let bytes = source.as_bytes();
    let mut i = at;
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    if bytes.get(i) != Some(&b'{') {
        return None;
    }
    let close = source[i..].find('}')? + i;
    Some((&source[i + 1..close], close + 1))
}

/// The font intervals of `source`: its own brace groups and font commands
/// ([`source_style_intervals`]) plus the declarations user macros wrap
/// around their arguments ([`macro_argument_intervals`]).
fn style_intervals(source: &str) -> Vec<StyleInterval> {
    let mut out = source_style_intervals(source);
    out.extend(macro_argument_intervals(source));
    // Stable: at one start byte the invocation site's intervals stay before
    // the ones the definition adds, and those before an argument's own.
    out.sort_by_key(|(start, _, _, _)| *start);
    out
}

/// The font declarations a user macro's definition wraps around each of its
/// parameters, laid over that argument's bytes at every invocation.
///
/// The compiler gives an argument's tokens their own source span, so the
/// style lookup (`Styles::at`) reads the argument's bytes — which sit outside
/// every group the *definition* opened. For
/// `\newcommand{\note}[1]{{\small\bfseries #1}}`, `\note{words}` set `words`
/// medium where pdfLaTeX sets them in `SFBX0900`: the size came through (the
/// compiler scopes sizes) and the series did not. Here the definition body's
/// own intervals that contain `#k` are re-applied to argument `k`, in body
/// order, between the invocation site's style and the argument's own
/// commands — the order TeX applies them in.
///
/// Definitions with a default optional argument (`[n][default]`) are skipped,
/// because their `#1` is the bracketed argument and not a brace group; so is
/// an undelimited (unbraced) argument.
fn macro_argument_intervals(source: &str) -> Vec<StyleInterval> {
    let bytes = source.as_bytes();
    let defs = macro_definitions(source);
    let mut out = Vec::new();
    for (index, def) in defs.iter().enumerate() {
        let name = &source[def.name.clone()];
        // A later definition of the same name takes over from its position.
        let until = defs[index + 1..].iter().find(|d| source[d.name.clone()] == *name).map_or(bytes.len(), |d| d.at);
        let header = &source[def.name.end..def.body.start - 1];
        if header.matches('[').count() > 1 {
            continue;
        }
        let body = &source[def.body.clone()];
        let body_intervals = source_style_intervals(body);
        // For each parameter the body uses, the body intervals around it:
        // the command, whether its group closes right after `#k` (italic
        // correction), and whether it is outside every group of the body,
        // so that it stays in force after the invocation too.
        let mut params: Vec<(usize, Vec<(crate::nfss::Command, bool, bool)>)> = Vec::new();
        for k in 1..=9usize {
            let Some(p) = body.find(&format!("#{k}")) else { continue };
            let chain: Vec<_> = body_intervals.iter().filter(|(s, e, _, _)| *s <= p && p < *e).map(|(_, e, c, _)| (*c, *e == p + 2, *e >= body.len())).collect();
            if !chain.is_empty() {
                params.push((k, chain));
            }
        }
        let Some(arity) = params.iter().map(|(k, _)| *k).max() else { continue };
        let mut from = def.body.end;
        // (A redefinition nested inside this body ends the range before it
        // starts.)
        while from < until {
            let Some(at) = find_command(&source[from..until], name) else { break };
            let inv = from + at;
            from = inv + 1 + name.len();
            // The argument bytes of this invocation, brace groups only.
            let mut i = from;
            let mut args = Vec::new();
            while args.len() < arity {
                while i < bytes.len() && (bytes[i] as char).is_whitespace() {
                    i += 1;
                }
                if bytes.get(i) != Some(&b'{') {
                    break;
                }
                let Some(close) = matching_brace(bytes, i) else { break };
                args.push((i + 1, close));
                i = close + 1;
            }
            for (k, chain) in &params {
                if let Some(&(start, end)) = args.get(k - 1) {
                    for &(c, correction, leaks) in chain {
                        if leaks {
                            // As an ungrouped declaration written at the call
                            // site: to the end of the enclosing group, or to
                            // the next `\end` outside any.
                            let to = enclosing_group_end(source, inv).unwrap_or_else(|| find_command(&source[i..], "end").map_or(bytes.len(), |e| i + e));
                            out.push((start, to, c, false));
                        } else {
                            out.push((start, end, c, correction));
                        }
                    }
                }
            }
        }
    }
    out
}

/// The closing brace of the innermost brace group containing byte `at`
/// (escaped `\{`/`\}` are not groups), or `None` at the top level.
fn enclosing_group_end(source: &str, at: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let escaped = |j: usize| j > 0 && bytes[j - 1] == b'\\';
    let mut depth = 0usize;
    let mut j = at;
    while j > 0 {
        j -= 1;
        match bytes[j] {
            b'}' if !escaped(j) => depth += 1,
            b'{' if !escaped(j) => {
                if depth == 0 {
                    return matching_brace(bytes, j);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

/// The font intervals spelled in `source` itself (see [`style_intervals`]).
fn source_style_intervals(source: &str) -> Vec<StyleInterval> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let literal = literal_spans(source);
    let mut next_literal = 0usize;
    let mut i = 0;
    let mut in_comment = false;
    // Open brace groups (byte of `{`): a declaration (`\bfseries`,
    // `\Large`, ...) lasts to the end of the innermost one, or to the next
    // `\end{...}`/the document end outside any group.
    let mut groups: Vec<usize> = Vec::new();
    while i < bytes.len() {
        // A verbatim construct starting here: typewriter over the whole of
        // it, and its bytes are skipped rather than scanned (see
        // `literal_spans`). The interval covers the whole command because
        // the compiler spans `\verb|...|` and the `\begin{verbatim}` block
        // from the backslash, and the style is looked up at `span.start`.
        while next_literal < literal.len() && literal[next_literal].whole.1 <= i {
            next_literal += 1;
        }
        if let Some(span) = literal.get(next_literal) {
            if span.whole.0 == i && !in_comment {
                use crate::nfss::{Command as C, FamilyKind as F};
                out.push((span.whole.0, span.whole.1, C::Family(F::Tt), false));
                i = span.whole.1;
                next_literal += 1;
                continue;
            }
        }
        let c = bytes[i];
        if in_comment {
            if c == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'%' => {
                in_comment = true;
                i += 1;
            }
            b'{' => {
                groups.push(i);
                i += 1;
            }
            b'}' => {
                groups.pop();
                i += 1;
            }
            b'\\' => {
                let rest = &source[i..];
                // The control word's letters.
                let word_end = i + 1 + rest[1..].bytes().take_while(u8::is_ascii_alphabetic).count();
                let name = &source[i + 1..word_end];
                if url_command(name) {
                    // `\url{...}`: typewriter over the whole command, and the
                    // argument's bytes are skipped rather than scanned (see
                    // `url_command`).
                    let mut j = word_end;
                    while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                        j += 1;
                    }
                    if j < bytes.len() && bytes[j] == b'{' {
                        if let Some(close) = url_argument_end(bytes, j) {
                            use crate::nfss::{Command as C, FamilyKind as F};
                            out.push((i, close + 1, C::Family(F::Tt), false));
                            i = close + 1;
                            continue;
                        }
                    }
                    i = word_end;
                    continue;
                }
                if let Some(commands) = text_font_command(name) {
                    let mut j = word_end;
                    while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                        j += 1;
                    }
                    if j < bytes.len() && bytes[j] == b'{' {
                        if let Some(close) = matching_brace(bytes, j) {
                            out.extend(commands.iter().map(|c| (j + 1, close, *c, true)));
                        }
                    }
                    i = word_end;
                    continue;
                }
                if let Some((commands, correction)) = font_declaration(name) {
                    let end = match groups.last() {
                        Some(&open) => matching_brace(bytes, open).unwrap_or(bytes.len()),
                        None => find_command(&source[word_end..], "end").map_or(bytes.len(), |e| word_end + e),
                    };
                    out.extend(commands.iter().map(|c| (word_end, end, *c, correction)));
                }
                i = word_end.max(i + 2);
            }
            _ => i += 1,
        }
    }
    // Stable: the `\normalfont` of `\bf` stays before its `\bfseries`.
    out.sort_by_key(|(start, _, _, _)| *start);
    out
}

fn continues_word(bytes: &[u8], at: usize) -> bool {
    at < bytes.len() && bytes[at].is_ascii_alphabetic()
}

/// The control word (`\name`, letters only) that starts at byte `at`, if
/// the source holds one there and it ends before `end`.
fn control_word_at(source: &str, at: usize, end: usize) -> Option<&str> {
    let rest = source.get(at..end)?;
    let rest = rest.strip_prefix('\\')?;
    let len = rest.bytes().take_while(u8::is_ascii_alphabetic).count();
    if len == 0 || len != rest.len() {
        return None;
    }
    Some(&rest[..len])
}

/// Whether the bytes at `at` are exactly the control word `\<name>` (not
/// a longer word: `\hfil` is not `\hfill`).
fn is_control_word(source: &str, at: usize, name: &str) -> bool {
    let bytes = source.as_bytes();
    source.get(at..).is_some_and(|r| r.starts_with('\\') && r[1..].starts_with(name)) && !continues_word(bytes, at + 1 + name.len())
}

/// Whether `span` is a user-macro invocation (`\name` exactly, with a
/// `\newcommand`-style definition in the source): the compiler gives every
/// token of the replacement text this span.
fn is_invocation_span(source: &str, span: Span) -> bool {
    control_word_at(source, span.start, span.end).is_some_and(|name| macro_body(source, name, span.start).is_some())
}

/// The replacement text of the last `\newcommand`/`\renewcommand`/
/// `\providecommand`/`\def` for `\<name>` before byte `before` (or the first
/// one anywhere), as the bytes inside its braces.
fn macro_body<'a>(source: &'a str, name: &str, before: usize) -> Option<&'a str> {
    // Within an adapt call the definitions of each document are indexed
    // once (FT-065: rescanning the whole source per invocation token made a
    // warm 500 KB request take seconds); elsewhere the source is scanned.
    let pick = |defs: &[MacroDef]| defs.iter().rev().find(|d| d.at < before).or(defs.first()).map(|d| &source[d.body.clone()]);
    let indexed = MACRO_DEFS.with(|scope| {
        let mut scope = scope.borrow_mut();
        let entry = scope.iter_mut().find(|e| e.ptr == source.as_ptr() as usize && e.len == source.len())?;
        let index = entry.index.get_or_insert_with(|| {
            let mut by_name: HashMap<String, Vec<MacroDef>> = HashMap::new();
            for d in macro_definitions(source) {
                by_name.entry(source[d.name.clone()].to_string()).or_default().push(d);
            }
            by_name
        });
        Some(index.get(name).map(|defs| (defs.first().map(|d| d.body.clone()), defs.iter().rev().find(|d| d.at < before).map(|d| d.body.clone()))))
    });
    match indexed {
        Some(found) => found.and_then(|(first, last_before)| last_before.or(first)).map(|r| &source[r]),
        None => {
            let defs: Vec<MacroDef> = macro_definitions(source).into_iter().filter(|d| &source[d.name.clone()] == name).collect();
            pick(&defs)
        }
    }
}

/// One `\newcommand`-style definition: where its command starts, the
/// defined name's bytes and the replacement text inside its braces.
#[derive(Debug, Clone)]
struct MacroDef {
    at: usize,
    name: std::ops::Range<usize>,
    body: std::ops::Range<usize>,
}

/// Per-thread definition indexes for the documents of the adapt call in
/// progress, keyed by the text's address and length. Only texts registered
/// by a live [`MacroDefsScope`] are indexed, and those are borrowed for the
/// whole scope, so a key can never name different bytes while it is used.
struct MacroDefsEntry {
    ptr: usize,
    len: usize,
    index: Option<HashMap<String, Vec<MacroDef>>>,
}

thread_local! {
    static MACRO_DEFS: std::cell::RefCell<Vec<MacroDefsEntry>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Registers `texts` for definition indexing until dropped.
struct MacroDefsScope {
    saved: Vec<MacroDefsEntry>,
}

impl MacroDefsScope {
    fn enter(texts: &[&str]) -> MacroDefsScope {
        let entries = texts
            .iter()
            .map(|t| MacroDefsEntry {
                ptr: t.as_ptr() as usize,
                len: t.len(),
                index: None,
            })
            .collect();
        MacroDefsScope {
            saved: MACRO_DEFS.with(|scope| std::mem::replace(&mut *scope.borrow_mut(), entries)),
        }
    }
}

impl Drop for MacroDefsScope {
    fn drop(&mut self) {
        let saved = std::mem::take(&mut self.saved);
        MACRO_DEFS.with(|scope| *scope.borrow_mut() = saved);
    }
}

/// Every definition in `source`, sorted by position (the scan
/// [`macro_body`] filters by name).
fn macro_definitions(source: &str) -> Vec<MacroDef> {
    let bytes = source.as_bytes();
    let mut defs: Vec<MacroDef> = Vec::new();
    for command in ["newcommand", "renewcommand", "providecommand", "def"] {
        let mut from = 0;
        while let Some(at) = find_command(&source[from..], command) {
            let abs = from + at;
            from = abs + 1;
            let mut i = abs + 1 + command.len();
            let skip_ws = |i: &mut usize| {
                while *i < bytes.len() && (bytes[*i] as char).is_whitespace() {
                    *i += 1;
                }
            };
            skip_ws(&mut i);
            if bytes.get(i) == Some(&b'*') {
                i += 1;
                skip_ws(&mut i);
            }
            // `{\name}` or `\name`.
            let braced = bytes.get(i) == Some(&b'{');
            if braced {
                i += 1;
                skip_ws(&mut i);
            }
            let Some(rest) = source.get(i..) else { continue };
            let Some(rest) = rest.strip_prefix('\\') else { continue };
            let len = rest.bytes().take_while(u8::is_ascii_alphabetic).count();
            let name_range = i + 1..i + 1 + len;
            i += 1 + len;
            skip_ws(&mut i);
            if braced {
                if bytes.get(i) != Some(&b'}') {
                    continue;
                }
                i += 1;
            }
            // `[n]`, `[default]` (LaTeX) or `#1#2` (`\def`).
            loop {
                skip_ws(&mut i);
                match bytes.get(i) {
                    Some(b'[') => match source[i..].find(']') {
                        Some(close) => i += close + 1,
                        None => break,
                    },
                    Some(b'#') => i += 2,
                    _ => break,
                }
            }
            if bytes.get(i) != Some(&b'{') {
                continue;
            }
            let Some(close) = matching_brace(bytes, i) else { continue };
            defs.push(MacroDef {
                at: abs,
                name: name_range,
                body: i + 1..close,
            });
        }
    }
    defs.sort_by_key(|d| d.at);
    defs
}

/// For a macro invoked at `inv` (its `\name` span), the index (from 1) of
/// the brace-delimited argument whose bytes contain `at`, and the macro's
/// name.
fn macro_arg_index(source: &str, inv: Span, at: usize) -> Option<(&str, usize)> {
    let name = control_word_at(source, inv.start, inv.end)?;
    let bytes = source.as_bytes();
    let mut i = inv.end;
    let mut k = 0usize;
    loop {
        while i < bytes.len() && (bytes[i] as char).is_whitespace() {
            i += 1;
        }
        match bytes.get(i) {
            Some(b'[') => {
                let close = source[i..].find(']')?;
                i += close + 1;
            }
            Some(b'{') => {
                let close = matching_brace(bytes, i)?;
                k += 1;
                if at > i && at < close {
                    return Some((name, k));
                }
                i = close + 1;
            }
            _ => return None,
        }
    }
}

/// Where the reader stands inside a macro's replacement text: the
/// invocation (`\name` span the compiler gives every replacement token)
/// and the byte offset in its definition body after the last token read.
#[derive(Debug, Clone, Copy)]
struct BodyCursor {
    inv: Span,
    at: usize,
}

/// The bytes TeX read between the previous token and this one, in the
/// order it read them, so that [`gap_has_space`] can be
/// applied to them uniformly: the source between two exact spans; the
/// definition text between two tokens of one replacement (`\hfill
/// \normalfont[` between `#1` and `[`); and across the boundary between a
/// replacement token and an argument (the whitespace around `#k`). `text`
/// is the token's text, used to find its place in a definition. `None`
/// when nothing was read (the first token) or the place is unknown.
fn token_gap(src: &str, prev_end: Option<usize>, prev_span: Option<Span>, span: Span, text: Option<&str>, cursor: &mut Option<BodyCursor>) -> Option<String> {
    let source_gap = |pe: usize, ps: Span| -> Option<String> {
        if ps.document != span.document {
            // Crossing an \input boundary: TeX reads the newline that ends
            // the \input line as a space.
            Some(" ".to_string())
        } else {
            (pe <= span.start).then(|| src.get(pe..span.start).unwrap_or("").to_string())
        }
    };
    let digits = |k: usize| 1 + k.to_string().len();
    // A word of a replacement text.
    if let Some(name) = control_word_at(src, span.start, span.end) {
        if let Some(body) = macro_body(src, name, span.start) {
            let (start, prefix) = match *cursor {
                Some(c) if c.inv == span => (c.at, None),
                _ => match prev_span.and_then(|ps| macro_arg_index(src, span, ps.start)) {
                    // The previous token was an argument of this invocation.
                    Some((_, k)) => (body.find(&format!("#{k}")).map_or(0, |p| p + digits(k)), None),
                    None => (0, prev_end.zip(prev_span).map(|(pe, ps)| source_gap(pe, ps).unwrap_or_default())),
                },
            };
            let Some(text) = text else {
                // Glue or math of a replacement: its place is not searched;
                // separate tokens of one replacement are taken as spaced.
                *cursor = Some(BodyCursor { inv: span, at: start });
                return prev_end.map(|_| " ".to_string());
            };
            // A control word (the glue arms pass `\hfill`/`\quad`/...) is
            // matched as a whole word, so `\hfil` never stops at `\hfill`.
            let find_text = |rest: &str| match text.strip_prefix('\\') {
                Some(name) if name.chars().all(|c| c.is_ascii_alphabetic()) => find_command(rest, name),
                _ => rest.find(text),
            };
            match body.get(start..).and_then(find_text) {
                Some(p) => {
                    let pos = start + p;
                    *cursor = Some(BodyCursor { inv: span, at: pos + text.len() });
                    let gap = &body[start..pos];
                    return Some(match prefix {
                        Some(before) => format!("{before}{gap}"),
                        None => gap.to_string(),
                    });
                }
                None => {
                    // Not found verbatim (ligatures rewrote it).
                    *cursor = Some(BodyCursor { inv: span, at: start });
                    return prev_end.map(|_| " ".to_string());
                }
            }
        }
    }
    // An argument of the invocation being read.
    if let Some(c) = *cursor {
        if let Some((name, k)) = macro_arg_index(src, c.inv, span.start) {
            if let Some(body) = macro_body(src, name, c.inv.start) {
                if let Some(p) = body.get(c.at..).and_then(|rest| rest.find(&format!("#{k}"))) {
                    let pos = c.at + p;
                    let gap = body[c.at..pos].to_string();
                    *cursor = Some(BodyCursor { inv: c.inv, at: pos + digits(k) });
                    return Some(gap);
                }
                // Further tokens of the same argument: the source between
                // them (the cursor stays after `#k`).
                return prev_end.zip(prev_span).and_then(|(pe, ps)| source_gap(pe, ps));
            }
        }
    }
    *cursor = None;
    prev_end.zip(prev_span).and_then(|(pe, ps)| source_gap(pe, ps))
}

/// The control sequence a compiler `Inline::Kern` was read from: its own
/// source bytes, or, inside a user macro's replacement (whose tokens carry
/// the invocation span), the first spelling of that kern in the macro body.
fn kern_command_text(source: &str, span: Span, amount: &TextDimen) -> Option<String> {
    if !is_invocation_span(source, span) {
        return source.get(span.start..span.end).map(str::to_string);
    }
    let name = control_word_at(source, span.start, span.end)?;
    let body = macro_body(source, name, span.start)?;
    const SPELLINGS: &[&str] = &[",", "!", ":", ">", ";", "thinspace", "negthinspace", "medspace", "negmedspace", "thickspace", "negthickspace", "enspace"];
    // `\,`/`\!` have two amounts: amsmath renews `\thinspace`/`\negthinspace`
    // to `.1667em` against the kernel's `.16667em` (20sp apart at 10pt, 24sp
    // at 12pt, measured with TeX Live 2025 pdflatex), and the compiler picks
    // one from the document's packages. This is a reverse lookup of the
    // *spelling* that produced an amount already chosen, and it has no
    // document in scope, so it accepts either definition; the two amounts are
    // disjoint, so no spelling is claimed twice.
    SPELLINGS
        .iter()
        .filter(|s| kern_amount_matches(s, amount))
        .map(|s| format!("\\{s}"))
        .find(|s| body.contains(s.as_str()))
}

/// Whether a spelling produces `amount` under any package context.
#[cfg(feature = "compiler-package-gating")]
fn kern_amount_matches(spelling: &str, amount: &TextDimen) -> bool {
    [false, true].iter().any(|&amsmath| {
        flashtex_compiler::text_builtins::text_kern(spelling, amsmath).as_ref() == Some(amount)
    })
}

/// Against a `vendor/compiler` pinned before the package context reached
/// `text_kern`, there is only the kernel definition to match.
#[cfg(not(feature = "compiler-package-gating"))]
fn kern_amount_matches(spelling: &str, amount: &TextDimen) -> bool {
    flashtex_compiler::text_builtins::text_kern(spelling).as_ref() == Some(amount)
}

/// [`gap_has_space`] for the bytes after a control word: the whitespace
/// TeX eats right after the word does not count.
fn gap_has_space_after_control_word(rest: &str) -> bool {
    let rest = rest.trim_start_matches([' ', '\t']);
    // A newline right after the word is eaten too (it is the same skip).
    let rest = rest.strip_prefix('\n').unwrap_or(rest);
    gap_has_space(rest)
}

/// The sum of every `\vspace{<dimen>}`/`\vspace*{<dimen>}` in `gap`, in
/// points; `None` when there is none or one does not parse.
fn vspace_in_gap(gap: &str, size: u32) -> Option<f64> {
    let mut from = 0;
    let mut total = 0.0;
    let mut any = false;
    while let Some(at) = find_command(&gap[from..], "vspace") {
        let abs = from + at;
        from = abs + 1;
        let rest = gap[abs + "\\vspace".len()..].trim_start();
        let rest = rest.strip_prefix('*').unwrap_or(rest).trim_start();
        let inner = rest.strip_prefix('{')?;
        let close = inner.find('}')?;
        total += parse_dimen(&inner[..close], size)?;
        any = true;
    }
    any.then_some(total)
}

/// The `[<dimen>]` of `\\[<dimen>]`/`\\*[<dimen>]` whose `\\` ends at byte
/// `after`, in points.
fn line_break_skip(source: &str, after: usize, size: u32) -> Option<f64> {
    let rest = source.get(after..)?;
    let rest = rest.strip_prefix('*').unwrap_or(rest);
    let rest = rest.trim_start_matches([' ', '\t']);
    let inner = rest.strip_prefix('[')?;
    let close = inner.find(']')?;
    parse_dimen(&inner[..close], size)
}

/// An `Inline::LineBreak` the pipeline makes up itself (a `verbatim` line
/// ending), written through one constructor so the crate builds against a
/// pinned compiler with or without the `skip_pt` field.
fn line_break_inline(span: Span) -> Inline {
    #[cfg(feature = "linebreak-skip")]
    {
        Inline::LineBreak { span, skip_pt: None }
    }
    #[cfg(not(feature = "linebreak-skip"))]
    {
        Inline::LineBreak { span }
    }
}

/// The `\\[<dimen>]` skip the compiler itself parsed, when the pinned
/// compiler reports one (`parser::Inline::LineBreak::skip_pt`).
///
/// [`line_break_skip`] below re-reads the `[...]` out of the source bytes
/// after the node's span, which is right only for a `\\` written literally in
/// the document. A `\\` that came out of a macro body carries the *invocation*
/// as its span (`expansion::Converter::place`), so those bytes are the call's
/// own arguments -- `{Education}` of `\\cvsection{Education}` -- and the skip
/// is unreachable from the source. The compiler reads it off the expanded
/// token stream, like TeX, so its value is preferred and the byte scan stays
/// as the fallback for a vendor pin that predates the field.
#[allow(unused_variables)]
fn reported_line_break_skip(inline: &Inline) -> Option<f64> {
    #[cfg(feature = "linebreak-skip")]
    if let Inline::LineBreak { skip_pt, .. } = inline {
        return *skip_pt;
    }
    None
}

fn matching_brace(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => {
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

/// A body command read from the source (see [`body_commands`]); `start..end`
/// covers the command and its arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct BodyCommand {
    pub start: usize,
    pub end: usize,
    pub kind: BodyKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BodyKind {
    Event(ChromeEvent),
    /// `\chapter[*][short]{title}`: `title` is the argument's inner range.
    Chapter { starred: bool, title: (usize, usize) },
    NoIndent,
    /// `\maketitle` (laid out from the compiler's `TitleBlock`).
    MakeTitle,
    /// book.cls `\frontmatter`/`\mainmatter`/`\backmatter`.
    Matter(Matter),
    /// `\tableofcontents`, `\listoffigures`, `\listoftables`.
    ContentsList(crate::toc::ListKind),
    /// `\addcontentsline{<ext>}{<level>}{<entry>}`: `text` is the entry
    /// argument's inner range.
    AddContentsLine { list: crate::toc::ListKind, level: String, text: (usize, usize) },
    /// `\appendix`.
    Appendix,
    /// `\part[<short>]{<title>}` / `\part*{<title>}` (inner ranges).
    Part { starred: bool, short: Option<(usize, usize)>, title: (usize, usize) },
    /// `\input{<file>}` / `\include{<file>}`: where the entry document
    /// reads another document, so that commands before it precede that
    /// document's material.
    Input,
}

/// Which book.cls matter command (lines 284-298).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Matter {
    /// `\cleardoublepage \@mainmatterfalse \pagenumbering{roman}`.
    Front,
    /// `\cleardoublepage \@mainmattertrue \pagenumbering{arabic}`.
    Main,
    /// `\if@openright\cleardoublepage\else\clearpage\fi \@mainmatterfalse`.
    Back,
}

/// `\pagestyle`, `\thispagestyle`, `\markboth`, `\markright`, `\noindent`,
/// `\maketitle`, `\input`/`\include`, (when the class has chapters)
/// `\chapter` and (book) `\frontmatter`/`\mainmatter`/`\backmatter` after
/// `\begin{document}`, in source order, skipping comments.
pub fn body_commands(source: &str, chapters: bool, book: bool) -> Vec<BodyCommand> {
    let bytes = source.as_bytes();
    let begin = source.find("\\begin{document}").map_or(0, |b| b + "\\begin{document}".len());
    let group = |from: usize| -> Option<(usize, usize, usize)> {
        let rest = source.get(from..)?;
        let k = from + rest.len() - rest.trim_start().len();
        if bytes.get(k) != Some(&b'{') {
            return None;
        }
        let close = matching_brace(bytes, k)?;
        Some((k + 1, close, close + 1))
    };
    let mut out = Vec::new();
    let mut i = begin;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            b'\\' => {}
            _ => {
                i += 1;
                continue;
            }
        }
        let mut j = i + 1;
        while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
            j += 1;
        }
        if j == i + 1 {
            i += 2;
            continue;
        }
        let name = &source[i + 1..j];
        let found = match name {
            "pagestyle" | "thispagestyle" => group(j).and_then(|(s, e, after)| {
                let ps = PageStyle::parse(&source[s..e])?;
                let event = if name == "pagestyle" { ChromeEvent::PageStyle(ps) } else { ChromeEvent::ThisPageStyle(ps) };
                Some((BodyKind::Event(event), after))
            }),
            "markboth" => group(j).and_then(|(s1, e1, a1)| group(a1).map(|(s2, e2, a2)| (BodyKind::Event(ChromeEvent::MarkBoth(plain_text(&source[s1..e1]), plain_text(&source[s2..e2]))), a2))),
            "markright" => group(j).map(|(s, e, after)| (BodyKind::Event(ChromeEvent::MarkRight(plain_text(&source[s..e]))), after)),
            "chapter" if chapters => {
                let mut k = j;
                let starred = bytes.get(k) == Some(&b'*');
                if starred {
                    k += 1;
                }
                let rest = &source[k..];
                let trimmed = rest.trim_start();
                if trimmed.starts_with('[') {
                    if let Some(close) = trimmed.find(']') {
                        k += rest.len() - trimmed.len() + close + 1;
                    }
                }
                group(k).map(|(s, e, after)| (BodyKind::Chapter { starred, title: (s, e) }, after))
            }
            "part" => {
                let mut k = j;
                let starred = bytes.get(k) == Some(&b'*');
                if starred {
                    k += 1;
                }
                let rest = &source[k..];
                let trimmed = rest.trim_start();
                let mut short = None;
                if trimmed.starts_with('[') {
                    if let Some(close) = trimmed.find(']') {
                        let open = k + rest.len() - trimmed.len();
                        short = Some((open + 1, open + close));
                        k = open + close + 1;
                    }
                }
                group(k).map(|(s, e, after)| (BodyKind::Part { starred, short, title: (s, e) }, after))
            }
            "noindent" => Some((BodyKind::NoIndent, j)),
            "tableofcontents" => Some((BodyKind::ContentsList(crate::toc::ListKind::Toc), j)),
            "listoffigures" => Some((BodyKind::ContentsList(crate::toc::ListKind::Lof), j)),
            "listoftables" => Some((BodyKind::ContentsList(crate::toc::ListKind::Lot), j)),
            "appendix" => Some((BodyKind::Appendix, j)),
            "addcontentsline" => group(j).and_then(|(s1, e1, a1)| {
                let list = crate::toc::ListKind::from_ext(source[s1..e1].trim())?;
                let (s2, e2, a2) = group(a1)?;
                let (s3, e3, a3) = group(a2)?;
                Some((
                    BodyKind::AddContentsLine {
                        list,
                        level: source[s2..e2].trim().to_string(),
                        text: (s3, e3),
                    },
                    a3,
                ))
            }),
            "pagenumbering" => group(j).and_then(|(s, e, after)| {
                use flashtex_class_geometry::Numbering;
                let n = match source[s..e].trim() {
                    "arabic" => Numbering::Arabic,
                    "roman" => Numbering::Roman,
                    "Roman" => Numbering::UpperRoman,
                    "alph" => Numbering::Alph,
                    "Alph" => Numbering::UpperAlph,
                    _ => return None,
                };
                Some((BodyKind::Event(ChromeEvent::PageNumbering(n)), after))
            }),
            "setcounter" => group(j).and_then(|(s1, e1, a1)| {
                if source[s1..e1].trim() != "page" {
                    return None;
                }
                group(a1).and_then(|(s2, e2, a2)| source[s2..e2].trim().parse::<i64>().ok().map(|n| (BodyKind::Event(ChromeEvent::SetPage(n)), a2)))
            }),
            "maketitle" => Some((BodyKind::MakeTitle, j)),
            "input" | "include" => group(j).map(|(_, _, after)| (BodyKind::Input, after)),
            "frontmatter" if book => Some((BodyKind::Matter(Matter::Front), j)),
            "mainmatter" if book => Some((BodyKind::Matter(Matter::Main), j)),
            "backmatter" if book => Some((BodyKind::Matter(Matter::Back), j)),
            _ => None,
        };
        match found {
            Some((kind, end)) => {
                out.push(BodyCommand { start: i, end, kind });
                i = end;
            }
            None => i = j,
        }
    }
    out
}

/// Drops the compiler's text for the arguments of `\markboth`,
/// `\markright` and `\chapter` (it sets them as body text), and the
/// paragraphs left empty.
fn strip_command_text(blocks: &mut Vec<(CBlock, ParLeading)>, document: DocumentId, commands: &[BodyCommand]) {
    let ranges: Vec<(usize, usize)> = commands
        .iter()
        .filter(|c| matches!(c.kind, BodyKind::Chapter { .. } | BodyKind::Part { .. } | BodyKind::AddContentsLine { .. } | BodyKind::Event(ChromeEvent::MarkBoth(..) | ChromeEvent::MarkRight(_) | ChromeEvent::SetPage(_) | ChromeEvent::PageNumbering(_))))
        .map(|c| (c.start, c.end))
        .collect();
    if ranges.is_empty() {
        return;
    }
    let inside = |i: &Inline| {
        let s = inline_span(i);
        s.document == document && ranges.iter().any(|(a, b)| s.start >= *a && s.start < *b)
    };
    for (block, _) in blocks.iter_mut() {
        match block {
            CBlock::Paragraph(inlines) | CBlock::Styled { content: inlines, .. } | CBlock::ListItem { content: inlines, .. } | CBlock::FigureCaption { content: inlines } => inlines.retain(|i| !inside(i)),
            _ => {}
        }
    }
    blocks.retain(|(b, _)| !matches!(b, CBlock::Paragraph(i) | CBlock::Styled { content: i, .. } if i.is_empty()));
}

/// Body-font words of `source[start..end]` split at whitespace, every
/// character carrying its own bytes.
pub(crate) fn words_from_source(source: &str, document: DocumentId, start: usize, end: usize) -> Vec<Item> {
    words_at(&source[start..end], document, start)
}

/// [`words_from_source`] of `text`, which starts at byte `start` of its
/// document.
pub(crate) fn words_at(text: &str, document: DocumentId, start: usize) -> Vec<Item> {
    let mut items = Vec::new();
    let mut offset = 0usize;
    for word in text.split_whitespace() {
        let at = start + offset + text[offset..].find(word).unwrap_or(0);
        offset = at - start + word.len();
        if !items.is_empty() {
            items.push(Item::Space {
                style: TextStyle::default(),
                factor: 1000,
                no_break: false,
            });
        }
        let chars = word
            .char_indices()
            .map(|(k, c)| CharSrc {
                document,
                start: at + k,
                end: at + k + c.len_utf8(),
            })
            .collect();
        push_segment(&mut items, word.to_string(), chars, TextStyle::default());
    }
    items
}

/// `\bibname` (report.cls line 665, book.cls line 690). article.cls has
/// `\refname` = `References` instead, which is what the compiler puts in
/// the heading it synthesises for `thebibliography` whatever the class is.
const BIBNAME: &str = "Bibliography";

/// Whether this heading is the one the compiler synthesises for
/// `\begin{thebibliography}`: unnumbered, level 1, and its span — which the
/// compiler sets to the `\begin` merged with its widest-label argument —
/// really does start there in the source.
fn bibliography_heading(texts: &[&str], level: u8, number: &str, span: Span) -> bool {
    if level != 1 || !number.is_empty() {
        return false;
    }
    texts
        .get(span.document.0)
        .and_then(|t| t.get(span.start..span.end))
        .and_then(|t| t.strip_prefix("\\begin"))
        .is_some_and(|r| r.trim_start().starts_with("{thebibliography}"))
}

/// A run-in heading (`\@startsection` with a negative after-skip) opening a
/// paragraph: article.cls's `\paragraph` (level 4) and `\subparagraph`
/// (level 5).
///
/// ```tex
/// \newcommand\paragraph{\@startsection{paragraph}{4}{\z@}%
///   {3.25ex \@plus1ex \@minus.2ex}{-1em}{\normalfont\normalsize\bfseries}}
/// ```
///
/// `\@xsect`'s negative-`#5` branch does not set the head as a vertical
/// block at all. It arms `\everypar`, which throws away the following
/// paragraph's `\parindent` box (`{\setbox\z@\lastbox}`), sets
/// `\hskip #3 <head>` in its place and then `\hskip -#5` — so the head
/// *is* the first words of that paragraph, bold, at indent `#3`, followed
/// by 1 em rather than an interword space. The `\addvspace{#4}` above it is
/// the only vertical contribution.
///
/// The compiler does not parse these commands: it reports them and sets the
/// braced argument as ordinary body text, which lands at the front of
/// exactly the paragraph LaTeX runs the head into. So the title words are
/// already in the right place with the right spans, and all that is missing
/// is the weight, the indent, the 1 em and the skip above — no new block
/// type and no compiler change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunIn {
    /// 4 (`\paragraph`) or 5 (`\subparagraph`).
    pub level: u8,
    /// First byte of the braced title. Normally the paragraph's first
    /// character, but for the starred form the compiler sets the `*` itself
    /// as body text, so anything before this is dropped.
    pub title_start: usize,
    /// End of the braced title in the source (the byte after the last
    /// character of the argument), so the title's words can be told from
    /// the body text that follows them in the same paragraph.
    pub title_end: usize,
}

/// Turn the leading items whose bytes lie in the run-in heading's title
/// into the heading: `\bfseries` weight, and `\hskip <em>` in place of the
/// interword space that separates the title from the body text.
fn apply_run_in_heading(items: &mut [Item], run_in: &RunIn, em: f64, bold: bool) {
    // How many leading items are the title's. A word straddling the closing
    // brace cannot happen: the compiler ends the title's last inline at the
    // `}`. `Space`/`Label` inside the title carry no bytes worth testing, so
    // they only count once a later word proves they were still inside it.
    let mut title = 0usize;
    for (i, item) in items.iter().enumerate() {
        match item {
            Item::Word(word) if word.segments.iter().flat_map(|s| s.chars.iter()).all(|c| c.end <= run_in.title_end) => title = i + 1,
            Item::Space { .. } | Item::Label { .. } => {}
            _ => break,
        }
    }
    if title == 0 {
        return;
    }
    for item in &mut items[..title] {
        match item {
            Item::Word(word) => {
                // `\paragraph*`: the compiler does not consume the star, so
                // it arrives as the first character of the title's first
                // word. LaTeX sets no star, only a heading without a number.
                for seg in &mut word.segments {
                    if seg.chars.first().is_some_and(|c| c.start < run_in.title_start) {
                        let keep: Vec<bool> = seg.chars.iter().map(|c| c.start >= run_in.title_start).collect();
                        seg.text = seg.text.chars().zip(&keep).filter(|(_, k)| **k).map(|(c, _)| c).collect();
                        let mut it = keep.iter();
                        seg.chars.retain(|_| *it.next().unwrap_or(&true));
                    }
                }
                word.segments.retain(|s| !s.text.is_empty());
                for seg in &mut word.segments {
                    seg.style.bold = bold;
                    // `\normalfont`: the head's own weight, not a shape or
                    // family inherited from around the command.
                    seg.style.medium = false;
                }
            }
            // The interword glue inside the title is the *head* font's
            // `\fontdimen2`/`3`/`4` — `ecbx1000`'s, not `ecrm1000`'s, which
            // is 0.47 bp wider per space at 10 pt.
            Item::Space { style, .. } => {
                style.bold = bold;
                style.medium = false;
            }
            _ => {}
        }
    }
    // The interword space right after the title is `\@xsect`'s `\hskip -#5`.
    if let Some(Item::Space { .. }) = items.get(title) {
        items[title] = Item::Quad { em };
    }
}

/// [`RunIn`] when the bytes before `at` are `\paragraph{` / `\subparagraph{`
/// (with an optional `*`), i.e. `at` is the first byte of a run-in
/// heading's title. `text` is that document's source.
fn run_in_heading_at(text: &str, at: usize) -> Option<RunIn> {
    let head = text.get(..at)?;
    // Scan back over the title's `{`, the optional `*` and any whitespace.
    // The compiler starts the title's first inline just after the `{`, except
    // for the starred form, whose `*` it leaves for the inline to start at —
    // so the `{` can be on either side of `at`.
    let bytes = head.as_bytes();
    let (mut i, mut saw_open) = (head.len(), false);
    while i > 0 {
        match bytes[i - 1] {
            c if c.is_ascii_whitespace() => i -= 1,
            b'{' if !saw_open => {
                saw_open = true;
                i -= 1;
            }
            b'*' => i -= 1,
            _ => break,
        }
    }
    let before = &head[..i];
    // `\paragraph*` takes the same run-in shape; the star only suppresses a
    // number, and level 4/5 is past `secnumdepth` anyway.
    let level = if let Some(r) = before.strip_suffix("subparagraph") {
        r.ends_with('\\').then_some(5u8)
    } else if let Some(r) = before.strip_suffix("paragraph") {
        // Not `\subparagraph`, already handled, and not a control word this
        // is only the tail of (`\myparagraph`).
        r.ends_with('\\').then_some(4u8)
    } else {
        None
    }?;
    // The title's group must open at or before `at`; when it opens after,
    // only the star and whitespace may stand between.
    let title_start = if saw_open {
        at
    } else {
        let rest = text.get(at..)?;
        let open = rest.find('{')?;
        if !rest[..open].trim().trim_start_matches('*').is_empty() {
            return None;
        }
        at + open + 1
    };
    // The matching `}` of the title group.
    let rest = text.get(title_start..)?;
    let (mut depth, mut escaped) = (1i32, false);
    for (i, c) in rest.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match c {
            '\\' => escaped = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(RunIn {
                        level,
                        title_start,
                        title_end: title_start + i,
                    });
                }
            }
            _ => {}
        }
    }
    None
}

/// Words of generated text (`Chapter 1`) whose characters all point at
/// `span` (the command that produced them).
pub fn command_words(text: &str, span: Span) -> Vec<Item> {
    let mut items = Vec::new();
    for word in text.split_whitespace() {
        if !items.is_empty() {
            items.push(Item::Space {
                style: TextStyle::default(),
                factor: 1000,
                no_break: false,
            });
        }
        let chars = word
            .chars()
            .map(|_| CharSrc {
                document: span.document,
                start: span.start,
                end: span.end,
            })
            .collect();
        push_segment(&mut items, word.to_string(), chars, TextStyle::default());
    }
    items
}

/// Source text with runs of whitespace collapsed to one space.
fn plain_text(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The style groups of one document, indexed for point queries: the
/// intervals in source order (sorted by start, properly nested), the
/// running maximum of their ends (so a backward scan can stop as soon as no
/// earlier group can still contain the position), and the ends sorted.
#[derive(Debug, Clone, Default)]
struct Styles {
    intervals: Vec<(usize, usize, crate::nfss::Command)>,
    max_end: Vec<usize>,
    ends: Vec<usize>,
    /// Verbatim bodies, in order (`literal_spans`), for [`Styles::literal_at`].
    literal: Vec<VerbatimSpan>,
    scheme: crate::nfss::Scheme,
}

impl Styles {
    fn new(source: &str, intervals: Vec<StyleInterval>, scheme: crate::nfss::Scheme) -> Styles {
        let mut max_end = Vec::with_capacity(intervals.len());
        let mut m = 0;
        for (_, end, _, _) in &intervals {
            m = m.max(*end);
            max_end.push(m);
        }
        let mut ends: Vec<usize> = intervals.iter().filter(|i| i.3).map(|i| i.1).collect();
        ends.sort_unstable();
        let intervals = intervals.into_iter().map(|(s, e, c, _)| (s, e, c)).collect();
        Styles { intervals, max_end, ends, literal: literal_spans(source), scheme }
    }

    /// Whether the text an inline spanning from `at` typesets is verbatim.
    ///
    /// Two shapes answer yes, because the compiler spans the two verbatim
    /// constructs differently:
    ///
    /// * `at` is inside a verbatim *body* — a `verbatim`/`lstlisting` line,
    ///   which the compiler spans at its own bytes.
    /// * `at` is exactly where a `\verb`/`\lstinline` starts. That span is
    ///   the whole command (`parser::Inline::Verbatim` carries the span of
    ///   `\verb|...|` from the backslash), so no byte of it is in the body
    ///   and the containment test alone would miss every `\verb`.
    ///
    /// Constructs do not overlap and are in source order, so one binary
    /// search on each start byte finds the only candidate.
    fn literal_at(&self, at: usize) -> bool {
        let i = self.literal.partition_point(|s| s.body.0 <= at);
        if i > 0 && at < self.literal[i - 1].body.1 {
            return true;
        }
        let j = self.literal.partition_point(|s| s.whole.0 < at);
        self.literal.get(j).is_some_and(|s| s.whole.0 == at)
    }

    /// The style in force at byte `at`: the font commands of every interval
    /// containing it, applied outermost (earliest) first through NFSS
    /// selection (`crate::nfss::apply`), so order matters exactly as in
    /// LaTeX (`\textsc{\emph{x}}` is not `\emph{\textsc{x}}`).
    fn at(&self, at: usize) -> TextStyle {
        let p = self.intervals.partition_point(|(start, _, _)| *start <= at);
        let mut chain = Vec::new();
        let mut i = p;
        while i > 0 {
            i -= 1;
            if self.max_end[i] <= at {
                break;
            }
            let (_, end, command) = self.intervals[i];
            if at < end {
                chain.push(command);
            }
        }
        let mut key = crate::nfss::FontKey::default();
        let mut undefined = None;
        for command in chain.into_iter().rev() {
            let s = crate::nfss::apply(self.scheme, key, command);
            key = s.key;
            undefined = s.undefined.or(undefined);
        }
        TextStyle { undefined, ..TextStyle::default() }.with_key(key)
    }

    /// Whether the font in force at byte `at` is slanted (`\fontdimen1 >
    /// 0`): the loaded shape after `sub*`/`ssub*`.
    fn slanted_at(&self, at: usize) -> bool {
        let key = self.at(at).key();
        crate::nfss::terminal(self.scheme, crate::nfss::select(self.scheme, key).key).0.slanted()
    }

    /// Whether a style group's content ends exactly at `at`.
    fn closes_at(&self, at: usize) -> bool {
        self.ends.binary_search(&at).is_ok()
    }
}

fn style_at(styles: &Styles, at: usize) -> TextStyle {
    styles.at(at)
}

/// Whether the bytes between two consecutive inlines contain an interword
/// space under TeX's rules (braces and control words produce none; spaces
/// after a control word are eaten; comments swallow their newline).
fn gap_has_space(gap: &str) -> bool {
    let bytes = gap.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' | b'}' | b'[' | b']' => i += 1,
            b'%' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                i += 1;
            }
            b'\\' => {
                i += 1;
                if i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
                        i += 1;
                    }
                } else if i < bytes.len() && bytes[i] == b' ' {
                    return true;
                } else {
                    i += 1;
                }
            }
            c if (c as char).is_whitespace() => return true,
            _ => i += 1,
        }
    }
    false
}

/// TeX's space factor after `ch` (§1034 with plain.tex's `\sfcode`s, which
/// LaTeX keeps): `.?!` 3000, `:` 2000, `;` 1500, `,` 1250, closing
/// delimiters and quotes 0 (keep), uppercase 999. A code above 1000 does
/// not take effect while the factor is below 1000 (after an uppercase
/// letter "A." keeps 1000), which is why the update runs per character.
pub fn space_factor(ch: char, previous: u32) -> u32 {
    let code = match ch {
        // `…` is `\textellipsis`, whose last character is a period
        // (`.\kern\fontdimen3\font` three times), so it leaves the period's
        // space factor behind exactly as a typed `.` does: pdflatex sets
        // `ellipsis… here` with a 5.213 bp space at 12 pt
        // (`\fontdimen2 + \fontdimen7`), not the 3.902 bp of `\fontdimen2`
        // alone.
        '.' | '?' | '!' | '\u{2026}' => 3000,
        ':' => 2000,
        ';' => 1500,
        ',' => 1250,
        ')' | ']' | '\'' | '’' | '”' | '"' => 0,
        c if c.is_uppercase() => 999,
        _ => 1000,
    };
    if code == 1000 {
        1000
    } else if code < 1000 {
        if code > 0 {
            code
        } else {
            previous
        }
    } else if previous < 1000 {
        1000
    } else {
        code
    }
}

fn accent(mark: char, base: char) -> Option<char> {
    let table: &[(char, &str, &str)] = &[
        ('"', "aeiouyAEIOUY", "äëïöüÿÄËÏÖÜŸ"),
        ('\'', "aeiouyAEIOUYcnszCNSZ", "áéíóúýÁÉÍÓÚÝćńśźĆŃŚŹ"),
        ('`', "aeiouAEIOU", "àèìòùÀÈÌÒÙ"),
        ('^', "aeiouAEIOU", "âêîôûÂÊÎÔÛ"),
        ('~', "anoANO", "ãñõÃÑÕ"),
        ('=', "aeiouAEIOU", "āēīōūĀĒĪŌŪ"),
        ('.', "zcegZCEG", "żċėġŻĊĖĠ"),
    ];
    for (m, bases, composed) in table {
        if *m == mark {
            let idx = bases.chars().position(|b| b == base)?;
            return composed.chars().nth(idx);
        }
    }
    None
}

/// [`items_from_inlines`] through the cross-request cache. The key covers
/// the inlines (kinds, texts, relative spans, label/reference keys), the
/// source bytes they sit in (gaps decide spaces, groups decide styles and
/// italic corrections), the style in force at the start, and the label
/// table; the value is relocated by the block's byte offset.
#[allow(clippy::too_many_arguments)]
fn items_cached(
    texts: &[&str],
    inlines: &[Inline],
    styles: &[Styles],
    labels: &Labels,
    labels_fp: u64,
    size: u32,
    heading: bool,
    compiler_weight: bool,
    cache: Option<&crate::incremental::RenderCache>,
) -> Vec<Item> {
    let Some(cache) = cache else {
        return items_from_inlines_styled(texts, inlines, styles, labels, size, heading, compiler_weight);
    };
    // Table items nest item lists the relocation does not walk.
    if inlines.iter().any(|i| matches!(i, Inline::Tabular(_))) {
        return items_from_inlines_styled(texts, inlines, styles, labels, size, heading, compiler_weight);
    }
    let Some(first) = inlines.first().map(inline_span) else {
        return items_from_inlines_styled(texts, inlines, styles, labels, size, heading, compiler_weight);
    };
    let document = first.document;
    let mut start = first.start;
    let mut end = first.end;
    for i in inlines {
        let s = inline_span(i);
        if s.document != document {
            return items_from_inlines_styled(texts, inlines, styles, labels, size, heading, compiler_weight);
        }
        start = start.min(s.start);
        end = end.max(s.end);
    }
    let Some(src) = texts.get(document.0) else {
        return items_from_inlines_styled(texts, inlines, styles, labels, size, heading, compiler_weight);
    };
    // Macro replacement text carries the invocation's span: the spacing
    // and weight of its words come from the definition (`macro_body`), so
    // a block holding one cannot be keyed by its own bytes alone.
    if inlines.iter().any(|i| is_invocation_span(src, inline_span(i))) {
        return items_from_inlines_styled(texts, inlines, styles, labels, size, heading, compiler_weight);
    }
    // `\\[<dimen>]` reads past the block's last span: the key covers the
    // rest of that line.
    let slice_end = src[end.min(src.len())..].find('\n').map_or(src.len(), |n| end + n + 1).max((end + 2).min(src.len()));
    let Some(slice) = src.get(start..slice_end) else {
        return items_from_inlines_styled(texts, inlines, styles, labels, size, heading, compiler_weight);
    };
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    b'A'.hash(&mut h);
    document.0.hash(&mut h);
    slice.hash(&mut h);
    labels_fp.hash(&mut h);
    size.hash(&mut h);
    heading.hash(&mut h);
    compiler_weight.hash(&mut h);
    let no_styles = Styles::default();
    let st = styles.get(document.0).unwrap_or(&no_styles);
    let at = st.at(start);
    at.bold.hash(&mut h);
    at.italic.hash(&mut h);
    (at.slanted, at.caps, at.family, at.undefined, st.scheme).hash(&mut h);
    for i in inlines {
        let s = inline_span(i);
        (s.start.wrapping_sub(start), s.end.wrapping_sub(start)).hash(&mut h);
        match i {
            Inline::Text { text, style, .. } => {
                0u8.hash(&mut h);
                text.hash(&mut h);
                style.color.hash(&mut h);
                // A macro argument's font comes from the definition, which
                // may sit outside the hashed slice (`macro_argument_intervals`).
                let here = st.at(s.start);
                (here.bold, here.italic, here.slanted, here.caps, here.family, here.undefined).hash(&mut h);
            }
            Inline::LineBreak { .. } => 1u8.hash(&mut h),
            Inline::Math { list, display, number, color, .. } => {
                2u8.hash(&mut h);
                color.hash(&mut h);
                display.hash(&mut h);
                number.hash(&mut h);
                crate::incremental::hash_math(list, &mut h);
            }
            Inline::Label { key, value, .. } => {
                3u8.hash(&mut h);
                key.hash(&mut h);
                value.hash(&mut h);
            }
            Inline::Reference { key, page, equation, .. } => {
                4u8.hash(&mut h);
                key.hash(&mut h);
                page.hash(&mut h);
                equation.hash(&mut h);
            }
            // Lowered constructs (pin `d416472a`): their text is in the
            // source slice already hashed; the structure is hashed here.
            Inline::Footnote { number, mark, text, .. } => {
                9u8.hash(&mut h);
                number.hash(&mut h);
                mark.hash(&mut h);
                text.as_ref().map_or(0, Vec::len).hash(&mut h);
            }
            Inline::Tabular(t) => {
                10u8.hash(&mut h);
                t.entries.len().hash(&mut h);
                t.inline_lists().iter().map(|l| l.len()).sum::<usize>().hash(&mut h);
            }
            Inline::Verbatim { text, .. } => {
                11u8.hash(&mut h);
                text.hash(&mut h);
            }
            Inline::ColorBox(b) => {
                15u8.hash(&mut h);
                format!("{b:?}").hash(&mut h);
            }
            Inline::Underline(u) => {
                18u8.hash(&mut h);
                format!("{u:?}").hash(&mut h);
            }
            Inline::Logo { logo, style, .. } => {
                12u8.hash(&mut h);
                logo.hash(&mut h);
                style.hash(&mut h);
            }
            Inline::Rule { rule, style, .. } => {
                13u8.hash(&mut h);
                rule.hash(&mut h);
                style.hash(&mut h);
            }
            Inline::Kern { amount, style, .. } => {
                14u8.hash(&mut h);
                amount.hash(&mut h);
                style.hash(&mut h);
            }
            // The leader is hashed: `\hfill` and `\hrulefill` differ only in
            // it, and they carry different diagnostics, so an edit between
            // them must not reuse the cached block.
            Inline::HFill { leader, .. } => {
                6u8.hash(&mut h);
                match leader {
                    FillLeader::None => 0u8,
                    FillLeader::Rule => 1u8,
                    FillLeader::Dots => 2u8,
                }
                .hash(&mut h);
            }
            Inline::HSpace { pt, .. } => {
                7u8.hash(&mut h);
                pt.to_bits().hash(&mut h);
            }
            Inline::TextGlue { em, .. } => {
                8u8.hash(&mut h);
                em.to_bits().hash(&mut h);
            }
            Inline::MathRows { rows, aligned, .. } => {
                5u8.hash(&mut h);
                aligned.hash(&mut h);
                rows.len().hash(&mut h);
                for row in rows {
                    (row.span.start.wrapping_sub(start), row.span.end.wrapping_sub(start)).hash(&mut h);
                    row.number.hash(&mut h);
                    row.cells.len().hash(&mut h);
                    for cell in &row.cells {
                        crate::incremental::hash_math(cell, &mut h);
                    }
                }
            }
            Inline::Graphic(g) => {
                16u8.hash(&mut h);
                format!("{g:?}").hash(&mut h);
            }
            Inline::Transform(t) => {
                17u8.hash(&mut h);
                format!("{t:?}").hash(&mut h);
            }
            // Lowered by `lower_inline` like `Reference`, so every field
            // that selects its text is part of the key.
            Inline::CleverReference { keys, page, range, label_only, capitalise, linked, .. } => {
                19u8.hash(&mut h);
                keys.hash(&mut h);
                (page, range, label_only, capitalise, linked).hash(&mut h);
            }
        }
    }
    let key = h.finish();
    if let Some(a) = cache.adapted(key) {
        return crate::incremental::relocate_items(&a.items, start as isize - a.base as isize);
    }
    let items = items_from_inlines_styled(texts, inlines, styles, labels, size, heading, compiler_weight);
    cache.insert_adapted(key, crate::incremental::AdaptedBlock { items: items.clone(), base: start });
    items
}

/// Converts the compiler inlines into words, spaces, math and line breaks.
/// `texts` and `styles` are indexed by `DocumentId`; `size` is the class
/// size (for `em` in `\\[<dimen>]`); `heading` marks `\section{...}`
/// content, whose compiler styles start bold (`\normalfont` in it is read
/// from the compiler's own style, since the macro-expanded bytes are not in
/// the source at the invocation).
///
/// `compiler_weight`: bold and italic come from the compiler's own scoping
/// rather than the source's brace groups — a table entry whose array `>{}`
/// declarations were inserted from the column specification, or an amsthm
/// theorem-like environment, whose head and body fonts the package declares
/// and the source never spells at the head's span.
fn items_from_inlines_styled(texts: &[&str], inlines: &[Inline], styles: &[Styles], labels: &Labels, size: u32, heading: bool, compiler_weight: bool) -> Vec<Item> {
    // `\ref`/`\pageref` become ordinary text attributed to the command's
    // bytes; `\label` becomes a zero-width marker.
    let mut resolved: Vec<std::borrow::Cow<Inline>> = Vec::with_capacity(inlines.len());
    let mut reference_spans: Vec<Span> = Vec::new();
    for inline in inlines {
        lower_inline(inline, labels, &mut reference_spans, &mut resolved);
    }
    let mut items: Vec<Item> = Vec::new();
    let mut prev_end: Option<usize> = None;
    let mut prev_span: Option<Span> = None;
    let mut factor = 1000u32;
    let mut pending_accent: Option<(char, CharSrc)> = None;
    // The compiler's size declaration in force at the previous text
    // inline, for the interword space read after it.
    let mut prev_size_cpt = 0u16;
    let text_of = |d: DocumentId| -> &str { texts.get(d.0).copied().unwrap_or("") };
    let no_styles = Styles::default();
    let styles_of = |d: DocumentId| -> &Styles { styles.get(d.0).unwrap_or(&no_styles) };

    // Where the reader stands in a macro's replacement text (`token_gap`).
    let mut cursor: Option<BodyCursor> = None;
    // Whether the previous token was a glue control word (`\hfill`,
    // `\quad`, `\hspace`): TeX eats the whitespace right after it, and
    // the gap read next starts at that whitespace.
    let mut after_control_word = false;
    // Whether the bytes TeX read between `prev` and `span` held an
    // interword space (`token_gap`). `text` is the current token's text (a
    // word, or the glue's control word). `\hfill` in a title is the
    // compiler's own `Inline::HFill` (pin `3d3d5ae3`, also inside macro
    // bodies), so the gap is never scanned for fills here.
    let mut space_between = |prev_end: Option<usize>, prev_span: Option<Span>, span: Span, text: Option<&str>, after_control_word: bool| -> bool {
        let src = text_of(span.document);
        match token_gap(src, prev_end, prev_span, span, text, &mut cursor) {
            None => false,
            Some(gap) if after_control_word => gap_has_space_after_control_word(&gap),
            Some(gap) => gap_has_space(&gap),
        }
    };
    // An amsthm theorem-like `\item`: the gap between its head and the body
    // is `\hskip\thm@headsep` (or `proof`'s `\hskip\labelsep`), and the
    // head's `\ignorespaces` eats the source whitespace that would otherwise
    // be read as an interword space. See `crate::amsthm`.
    let mut head_sep = inlines
        .first()
        .map(inline_span)
        .and_then(|s| texts.get(s.document.0))
        .and_then(|src| crate::amsthm::head_separator(src, inlines, size));
    let pending_head_sep: std::cell::Cell<Option<(f64, f64, f64)>> = std::cell::Cell::new(None);
    // Pushes the space `space_between` found, or the theorem head's own glue
    // in its place. Every caller must reach this whenever a head separator is
    // pending, not only when the source had a space to replace: amsthm's head
    // ends with `\ignorespaces`, so `\begin{theorem}Body` has no source gap
    // at all, and a caller that skips the call on `!has_space` drops the
    // separator instead of substituting it.
    let push_gap = |items: &mut Vec<Item>, space: bool, style: TextStyle, factor: u32| {
        if let Some((pt, stretch_pt, shrink_pt)) = pending_head_sep.take() {
            items.push(Item::HSpace { pt, stretch_pt, shrink_pt });
            return;
        }
        if space {
            items.push(Item::Space { style, factor, no_break: false });
        }
    };

    for inline in resolved.iter() {
        if let Some(sep) = head_sep {
            if sep.opens_the_body(inline_span(inline)) {
                pending_head_sep.set(Some((sep.pt, sep.stretch_pt, sep.shrink_pt)));
                head_sep = None;
            }
        }
        match &**inline {
            Inline::Label { key, .. } => items.push(Item::Label { key: key.clone() }),
            Inline::Reference { .. } | Inline::CleverReference { .. } | Inline::Verbatim { .. } => unreachable!("lowered by lower_inline above"),
            Inline::Footnote { number, span, mark, text, .. } => {
                // `\@footnotemark` keeps the space factor; the space before
                // the command is an ordinary interword space. The command's
                // `[<n>]` and `{<text>}` are skipped for the gap that follows.
                let src = text_of(span.document);
                let end = footnote_command_end(src, span.end);
                let word = src.get(span.start..span.end).unwrap_or("\\footnote");
                let gap = space_between(prev_end, prev_span, *span, Some(word), after_control_word);
                let mut gap_style = space_style(texts, styles, prev_end, *span, TextStyle::default());
                gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                let note = text.as_ref().map(|t| {
                    let mut note = Vec::new();
                    for (k, part) in t.split(|i| matches!(i, Inline::LineBreak { span: at, .. } if at == span)).enumerate() {
                        if k > 0 {
                            note.push(Item::NoteParBreak);
                        }
                        note.extend(items_from_inlines_styled(texts, part, styles, labels, size, false, compiler_weight));
                    }
                    note
                });
                items.push(Item::Footnote { number: number.clone(), mark: *mark, span: *span, text: note });
                after_control_word = end == span.end;
                prev_end = Some(end);
                prev_span = Some(Span::in_document(span.document, span.start, end));
                pending_accent = None;
            }
            Inline::Tabular(t) => {
                // `\leavevmode\hbox{...}`: one box, with the space before it
                // read like a formula's.
                let span = t.span;
                let gap = space_between(prev_end, prev_span, span, None, after_control_word);
                let mut gap_style = space_style(texts, styles, prev_end, span, TextStyle::default());
                gap_style.size_cpt = space_size(texts, prev_end, span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                after_control_word = false;
                let src = text_of(span.document);
                let lengths = crate::table::TableLengths::read(|name| setlength(src, name, size));
                let mut items_of = |inlines: &[Inline], declared: bool| items_from_inlines_styled(texts, inlines, styles, labels, size, false, declared);
                let table = crate::table::from_compiler(t, lengths, declared_size(t.style.size, size), &mut items_of);
                items.push(Item::Table(Box::new(table)));
                prev_end = Some(span.end);
                prev_span = Some(span);
                factor = 1000;
            }
            Inline::ColorBox(b) => {
                // `\leavevmode\hbox{...}` like a tabular: one box.
                let span = b.span;
                let gap = space_between(prev_end, prev_span, span, None, after_control_word);
                let mut gap_style = space_style(texts, styles, prev_end, span, TextStyle::default());
                gap_style.size_cpt = space_size(texts, prev_end, span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                after_control_word = false;
                let content = items_from_inlines_styled(texts, &b.content, styles, labels, size, heading, compiler_weight);
                items.push(Item::ColorBox(Box::new(ColorBoxItem {
                    fill: b.fill,
                    frame: b.frame,
                    sep_pt: b.fboxsep_pt,
                    rule_pt: b.fboxrule_pt,
                    items: content,
                    span,
                })));
                prev_end = Some(span.end);
                prev_span = Some(span);
                factor = 1000;
            }
            Inline::Underline(u) => {
                let span = u.span;
                let gap = space_between(prev_end, prev_span, span, None, after_control_word);
                let mut gap_style = space_style(texts, styles, prev_end, span, TextStyle::default());
                gap_style.size_cpt = space_size(texts, prev_end, span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                after_control_word = false;
                let content = items_from_inlines_styled(texts, &u.content, styles, labels, size, heading, compiler_weight);
                items.push(Item::Underline(Box::new(UnderlineItem {
                    thickness_pt: u.thickness_pt,
                    geom: u.geom,
                    items: content,
                    span,
                })));
                prev_end = Some(span.end);
                prev_span = Some(span);
                factor = 1000;
            }
            Inline::LineBreak { span, .. } => {
                let skip_pt = reported_line_break_skip(inline)
                    .or_else(|| line_break_skip(text_of(span.document), span.end, size))
                    .unwrap_or(0.0);
                items.push(Item::LineBreak { skip_pt });
                prev_end = Some(span.end);
                prev_span = Some(*span);
                factor = 1000;
                after_control_word = false;
            }
            // #169's Inline::Graphic/Transform have no pipeline conversion
            // arm yet (#170 is not in this integration: its own diff
            // depends on a wire-protocol capability refactor -- a new
            // Wire { transforms } field threaded through display.rs's JSON
            // writers -- that collides with #158's already-merged
            // Wire { device_color } and needs real reconciliation, not a
            // mechanical merge). Degrade like the compiler's own Core 14
            // layout does: an image leaves no space for now, and a
            // transform box keeps its content set untransformed, so
            // nothing is silently dropped.
            Inline::Graphic(g) => {
                prev_end = Some(g.span.end);
                prev_span = Some(g.span);
                after_control_word = false;
            }
            Inline::Transform(t) => {
                let span = t.span;
                let gap = space_between(prev_end, prev_span, span, None, after_control_word);
                let mut gap_style = space_style(texts, styles, prev_end, span, TextStyle::default());
                gap_style.size_cpt = space_size(texts, prev_end, span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                after_control_word = false;
                items.extend(items_from_inlines_styled(texts, &t.content, styles, labels, size, false, compiler_weight));
                prev_end = Some(span.end);
                prev_span = Some(span);
                factor = 1000;
            }
            Inline::HFill { span, .. } | Inline::HSpace { span, .. } | Inline::TextGlue { span, .. } => {
                // Explicit horizontal glue: the interword space read before
                // it stays (TeX keeps both glue nodes). `TextGlue` is the
                // compiler's text-mode `\quad`/`\qquad` (`em` ems of the
                // current font, like the `\quad` after a section number).
                // The control word is passed as the token's text so that
                // a macro-body cursor moves past it (`Problem #1 \hfill
                // \normalfont[#2 points]`): the compiler gives the glue the
                // invocation's span, and the next token's gap must start
                // after the word, not before it.
                let (item, word) = match &**inline {
                    Inline::HSpace { pt, .. } => (Item::HSpace { pt: *pt, stretch_pt: 0.0, shrink_pt: 0.0 }, "\\hspace"),
                    Inline::TextGlue { em, .. } => (Item::Quad { em: *em }, if *em >= 2.0 { "\\qquad" } else { "\\quad" }),
                    // `\hrulefill` and `\dotfill` (compiler `FillLeader`, #320)
                    // are `\leavevmode\leaders<box>\hfill\kern\z@`: the glue is
                    // exactly `\hfill`, so it is set here like any other, and
                    // its leader box is carried through for painting after line
                    // breaking. The glue must still be emitted or the rest of
                    // the line lands in the wrong place, which is what
                    // `fixtures/divergence-probes/min-hrulefill` measured
                    // against pdflatex before the re-pin.
                    //
                    // The `\leavevmode` is theirs, not an invention here, and
                    // it is load-bearing: a `\hrulefill` alone in its paragraph
                    // (the fill-in rules of `enumitem-worksheet`) otherwise
                    // leaves a paragraph with glue and no box, which this
                    // pipeline drops together with the `\vspace` in front of
                    // it — that is what took the worksheet from 3 pages to 2.
                    // With the empty `\hbox` the paragraph is a line, as it is
                    // in pdflatex. (A bare `\hfill` alone in a paragraph still
                    // vanishes the same way; that is a separate pre-existing
                    // defect, reproducible on the previous pin, not this one.)
                    Inline::HFill { leader: FillLeader::Rule, .. } => {
                        (Item::HFill { fill: true, leader: FillLeader::Rule }, "\\hrulefill")
                    }
                    Inline::HFill { leader: FillLeader::Dots, .. } => {
                        (Item::HFill { fill: true, leader: FillLeader::Dots }, "\\dotfill")
                    }
                    Inline::HFill { leader: FillLeader::None, .. } => {
                        let fill = !is_control_word(text_of(span.document), span.start, "hfil");
                        (Item::HFill { fill, leader: FillLeader::None }, if fill { "\\hfill" } else { "\\hfil" })
                    }
                    _ => unreachable!(),
                };
                let gap = space_between(prev_end, prev_span, *span, Some(word), after_control_word);
                let mut gap_style = space_style(texts, styles, prev_end, *span, TextStyle::default());
                gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                // The `\leavevmode` that opens `\hrulefill`/`\dotfill`: an
                // empty `\hbox` in front of the glue (see the arms above).
                let leader_fill =
                    matches!(&**inline, Inline::HFill { leader, .. } if !matches!(leader, FillLeader::None));
                if leader_fill {
                    items.push(Item::LeaveVmode);
                }
                items.push(item);
                // ...and the `\kern\z@` that closes them, which is doing real
                // work: TeX ends a paragraph by deleting the final glue item
                // (tex.web §816, `hlist`'s trailing-glue loop) before adding
                // `\parfillskip`. A trailing bare `\hfill` is therefore eaten,
                // which is why `Name: \hfill Date: \hfill` sets `Date:` flush
                // right in pdflatex. The zero kern after `\hrulefill`'s fill
                // saves it, so both fills survive and share the leftover width
                // equally — pdflatex puts `Date:` at x 317.830 in
                // `fixtures/divergence-probes/min-hrulefill`, not at the
                // margin. Without this kern the pipeline set it at 511.918.
                if leader_fill {
                    items.push(Item::Kern {
                        amount: flashtex_compiler::text_builtins::TextDimen::zero(),
                        style: TextStyle::default(),
                    });
                }
                prev_end = Some(span.end);
                prev_span = Some(*span);
                factor = 1000;
                pending_accent = None;
                after_control_word = true;
            }
            Inline::MathRows { rows, span, .. } => {
                // Each row becomes its own display item (`is_display`
                // recognises the row spans); the environment's span ends
                // the preceding text like `\[`.
                let gap = space_between(prev_end, prev_span, *span, None, after_control_word);
                let mut gap_style = space_style(texts, styles, prev_end, *span, TextStyle::default());
                gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                after_control_word = false;
                for row in rows {
                    items.push(Item::Math {
                        list: math_row_list(row),
                        span: row.span,
                    });
                }
                prev_end = Some(span.end);
                prev_span = Some(*span);
                factor = 1000;
            }
            Inline::Math { list, span, .. } => {
                // The glue is the current font's where the space sits.
                let gap = space_between(prev_end, prev_span, *span, None, after_control_word);
                let mut gap_style = space_style(texts, styles, prev_end, *span, TextStyle::default());
                gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                after_control_word = false;
                items.push(Item::Math {
                    list: list.clone(),
                    span: *span,
                });
                prev_end = Some(span.end);
                prev_span = Some(*span);
                factor = 1000;
            }
            Inline::Logo { logo, span, style: compiler_style, .. } => {
                // `\LaTeX` is a control word: the blanks after it are eaten.
                let word = format!("\\{}", logo.command());
                let has_space = space_between(prev_end, prev_span, *span, Some(&word), after_control_word);
                let mut style = style_at(styles_of(span.document), span.start);
                style.size_cpt = declared_size(compiler_style.size, size);
                if heading {
                    style.medium = !compiler_style.bold;
                    style.italic |= compiler_style.italic;
                }
                if has_space || pending_head_sep.get().is_some() {
                    let mut gap_style = space_style(texts, styles, prev_end, *span, style);
                    gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, style.size_cpt);
                    push_gap(&mut items, has_space, gap_style, factor);
                }
                prev_size_cpt = style.size_cpt;
                items.push(Item::Logo { logo: *logo, style, span: *span });
                prev_end = Some(span.end);
                prev_span = Some(*span);
                // `\TeX` ends with `\@` and `\LaTeXe` with math: factor 1000.
                factor = 1000;
                pending_accent = None;
                after_control_word = true;
            }
            Inline::Rule { rule, span, style: compiler_style, .. } => {
                let has_space = space_between(prev_end, prev_span, *span, Some("\\rule"), after_control_word);
                let mut style = style_at(styles_of(span.document), span.start);
                style.size_cpt = declared_size(compiler_style.size, size);
                if has_space || pending_head_sep.get().is_some() {
                    let mut gap_style = space_style(texts, styles, prev_end, *span, style);
                    gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, style.size_cpt);
                    push_gap(&mut items, has_space, gap_style, factor);
                }
                prev_size_cpt = style.size_cpt;
                items.push(Item::Rule { rule: rule.clone(), style, span: *span });
                prev_end = Some(span.end);
                prev_span = Some(*span);
                factor = 1000;
                pending_accent = None;
                after_control_word = false;
            }
            Inline::Kern { amount, span, style: compiler_style } => {
                let source = text_of(span.document);
                let word = kern_command_text(source, *span, amount);
                let has_space = space_between(prev_end, prev_span, *span, word.as_deref(), after_control_word);
                let mut style = style_at(styles_of(span.document), span.start);
                style.size_cpt = declared_size(compiler_style.size, size);
                if has_space || pending_head_sep.get().is_some() {
                    let mut gap_style = space_style(texts, styles, prev_end, *span, style);
                    gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, style.size_cpt);
                    push_gap(&mut items, has_space, gap_style, factor);
                    pending_accent = None;
                }
                // A kern leaves the space factor alone (§1061 applies only to
                // characters and boxes); a control word eats the blanks after it.
                items.push(Item::Kern { amount: amount.clone(), style });
                prev_end = Some(span.end);
                prev_span = Some(*span);
                after_control_word = word.as_deref().is_some_and(|w| w.len() > 2);
            }
            Inline::Text { text, span, .. } if text == " " && text_of(span.document).get(span.start..span.end) == Some("\\ ") => {
                // `\ ` (control space, lexed as the word " "): interword glue at
                // space factor 1000 (§1041-1044), after which TeX skips blanks.
                let has_space = space_between(prev_end, prev_span, *span, Some("\\ "), after_control_word);
                let mut style = style_at(styles_of(span.document), span.start);
                let Inline::Text { style: compiler_style, .. } = &**inline else { unreachable!() };
                style.size_cpt = declared_size(compiler_style.size, size);
                if has_space || pending_head_sep.get().is_some() {
                    let mut gap_style = space_style(texts, styles, prev_end, *span, style);
                    gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, style.size_cpt);
                    push_gap(&mut items, has_space, gap_style, factor);
                }
                items.push(Item::Space { style, factor: 1000, no_break: false });
                prev_size_cpt = style.size_cpt;
                prev_end = Some(span.end);
                prev_span = Some(*span);
                factor = 1000;
                pending_accent = None;
                after_control_word = true;
            }
            Inline::Text { text, span, .. } => {
                let source = text_of(span.document);
                // The compiler (pin `8c0d65e7`) runs its text-ligature pass
                // over the accent command's own character too, so `\'` and
                // `\`` arrive as the curly quotes; map them back.
                let accent_mark = |t: &str| -> Option<char> {
                    let mut it = t.chars();
                    match (it.next(), it.next()) {
                        (Some('\u{2019}'), None) => Some('\''),
                        (Some('\u{2018}'), None) => Some('`'),
                        (Some(c), None) if "\"'`^~=.".contains(c) => Some(c),
                        _ => None,
                    }
                };
                let accent_char = if span.end - span.start == 2 && source.as_bytes().get(span.start) == Some(&b'\\') {
                    accent_mark(text)
                } else {
                    None
                };
                let mut style = style_at(styles_of(span.document), span.start);
                // Verbatim text: no ligatures, no kerns, rigid blanks. The
                // span of a `\verb|...|` starts at the backslash, so the
                // body byte is what decides — `span.start` is the `\`.
                style.literal = styles_of(span.document).literal_at(span.start);
                // `\tiny`..`\Huge` come from the compiler's scoping.
                let Inline::Text { style: compiler_style, .. } = &**inline else { unreachable!() };
                style.size_cpt = declared_size(compiler_style.size, size);
                style.color = compiler_style.color;
                if heading {
                    // `\@startsection` sets `\bfseries`; the compiler's
                    // heading styles start bold and `\normalfont`/
                    // `\mdseries` in the title clears it.
                    let Inline::Text { style: cs, .. } = &**inline else { unreachable!() };
                    style.medium = !cs.bold;
                    style.italic |= cs.italic;
                }
                if compiler_weight {
                    style.bold = compiler_style.bold;
                    style.italic = compiler_style.italic;
                }
                let has_space = space_between(prev_end, prev_span, *span, Some(text), after_control_word);
                after_control_word = false;
                if has_space || pending_head_sep.get().is_some() {
                    // TeX sizes an interword space with the font current
                    // where the space token is read ("Plain, \textbf{bold}"
                    // gets a regular space, "\textbf{bold words}" a bold one,
                    // "\textbf{\emph{x}} y" a regular one).
                    let mut gap_style = space_style(texts, styles, prev_end, *span, style);
                    if compiler_weight {
                        gap_style.bold = style.bold;
                        gap_style.italic = style.italic;
                    }
                    gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, style.size_cpt);
                    push_gap(&mut items, has_space, gap_style, factor);
                    pending_accent = None;
                }
                prev_size_cpt = style.size_cpt;
                if let Some(mark) = accent_char {
                    pending_accent = Some((
                        mark,
                        CharSrc {
                            document: span.document,
                            start: span.start,
                            end: span.end,
                        },
                    ));
                    prev_end = Some(span.end);
                    prev_span = Some(*span);
                    continue;
                }
                // Per-character sources. Macro replacement text shares the
                // invocation span; keep that attribution for every char.
                let citation = generated_citation(source, *span);
                let exact = span.end - span.start == text.len()
                    && !reference_spans.contains(span)
                    && !citation;
                let mut chars: Vec<(char, CharSrc)> = Vec::new();
                for (offset, ch) in text.char_indices() {
                    let src = if exact {
                        CharSrc {
                            document: span.document,
                            start: span.start + offset,
                            end: span.start + offset + ch.len_utf8(),
                        }
                    } else {
                        CharSrc {
                            document: span.document,
                            start: span.start,
                            end: span.end,
                        }
                    };
                    chars.push((ch, src));
                }
                if let Some((mark, msrc)) = pending_accent.take() {
                    if let Some((first, fsrc)) = chars.first().copied() {
                        if let Some(composed) = accent(mark, first) {
                            chars[0] = (
                                composed,
                                CharSrc {
                                    document: fsrc.document,
                                    start: msrc.start,
                                    end: fsrc.end,
                                },
                            );
                        }
                    }
                }
                // `--`, ``` `` ```, `''`, `` ?` `` are ligatures of the
                // *input*, and verbatim suppresses them (`\@noligs`): they
                // stay the characters that were typed. `\texttt` is not
                // verbatim and keeps them.
                let chars = if style.literal { chars } else { tex_ligatures(chars) };
                // `~` is an unbreakable space.
                let mut run: Vec<(char, CharSrc)> = Vec::new();
                let flush = |run: &mut Vec<(char, CharSrc)>, items: &mut Vec<Item>, factor: &mut u32| {
                    if run.is_empty() {
                        return;
                    }
                    let text: String = run.iter().map(|(c, _)| *c).collect();
                    let srcs: Vec<CharSrc> = run.iter().map(|(_, s)| *s).collect();
                    for (c, _) in run.iter() {
                        *factor = space_factor(*c, *factor);
                    }
                    push_segment(items, text, srcs, style);
                    run.clear();
                };
                for (ch, src) in chars {
                    // A blank inside replacement text (`!exact`: a theorem
                    // head, `\today`, any macro body) stands for a space
                    // *token*, so it is interword glue in the font in force,
                    // not a character. Only `exact` text -- the source's own
                    // bytes, verbatim included -- keeps a literal blank.
                    // A blank in verbatim is neither a space token nor a
                    // character: LaTeX's `\@vobeyspaces` makes it a control
                    // space (`\ `), which pdflatex's `\showbox` of
                    // `\verb|a b|` shows as `\penalty 10000` + `\glue
                    // 5.65837` at 11 pt. That glue is rigid here for free —
                    // the typewriter families set `\fontdimen3` and
                    // `\fontdimen4` to zero — and a control space ignores
                    // the space factor, so `\fontdimen7` is never added
                    // after a `.`.
                    if ch == ' ' && style.literal {
                        flush(&mut run, &mut items, &mut factor);
                        // `\leavevmode` before a blank that would open the
                        // line, so the indentation is not discarded.
                        if items.is_empty() || matches!(items.last(), Some(Item::LineBreak { .. })) {
                            items.push(Item::LeaveVmode);
                        }
                        items.push(Item::Space { style, factor: 1000, no_break: true });
                        factor = 1000;
                        continue;
                    }
                    if ch == ' ' && !exact {
                        flush(&mut run, &mut items, &mut factor);
                        // natbib writes every space of its own as
                        // `\NAT@spacechar` (`\ `, natbib.sty line 596) and
                        // the note's own gap is normally a tie (`p.~7`): both
                        // are control spaces, which ignore the space factor.
                        // So "et al. (1990)" and "p. 7" keep `\fontdimen2`
                        // where a space *token* after a `.` would also add
                        // `\fontdimen7` — 1.2167 pt at 11 pt, and abbreviated
                        // author lists and page notes are exactly where a `.`
                        // sits in front of a space.
                        let space_factor = if citation { 1000 } else { factor };
                        items.push(Item::Space { style, factor: space_factor, no_break: false });
                        factor = 1000;
                        continue;
                    }
                    // The tie: an interword space of the font in force with
                    // no legal breakpoint at it (`~` is catcode 13 and
                    // expands to `\nobreakspace` = `\leavevmode\nobreak\ `,
                    // latex.ltx 9411-9418; `inputenc` maps a typed U+00A0
                    // onto the same command).
                    //
                    // U+00A0 in the text is self-describing and needs no
                    // lookback, which is the point: the `~` arm below can
                    // only recognise a tie whose span covers its own byte,
                    // and replacement text carries the *invocation's* span,
                    // so `\newcommand{\fig}{Figure~7}` read `\fig` there and
                    // set a literal tilde. It cannot be fixed by dropping
                    // the span test either -- `\textasciitilde` produces the
                    // same character and must stay a tilde. A compiler that
                    // resolves the tie itself (`lexer::NO_BREAK_SPACE`)
                    // removes the ambiguity; until `vendor/compiler` is
                    // re-pinned past that change, the `~` arm still carries
                    // every tie written directly in a source.
                    if ch == NO_BREAK_SPACE || (ch == '~' && source.get(src.start..src.end) == Some("~")) {
                        flush(&mut run, &mut items, &mut factor);
                        items.push(Item::Space {
                            style,
                            factor: 1000,
                            no_break: true,
                        });
                        factor = 1000;
                        continue;
                    }
                    run.push((ch, src));
                }
                flush(&mut run, &mut items, &mut factor);
                // A style group closing right after this text: LaTeX's
                // \text@command appends \/ (`\maybe@ic`) unless the next
                // token is in \nocorrlist (`,` and `.`) or the enclosing
                // font is itself slanted (`\fontdimen1 > 0`).
                if styles_of(span.document).closes_at(span.end)
                    && source.as_bytes().get(span.end) == Some(&b'}')
                    && !matches!(source.as_bytes().get(span.end + 1), Some(b'.') | Some(b','))
                    && !styles_of(span.document).slanted_at(span.end + 1)
                    && matches!(items.last(), Some(Item::Word(_)))
                {
                    items.push(Item::ItalicCorrection);
                }
                // A text symbol the compiler set from a control word (`\AA`,
                // `\ss`, `\today`): TeX skips the blanks after the word. User
                // macro replacements keep their own cursor (`token_gap`).
                after_control_word = control_word_at(source, span.start, span.end).is_some() && !is_invocation_span(source, *span);
                prev_end = Some(span.end);
                prev_span = Some(*span);
            }
        }
    }
    items
}

/// The byte after `\footnote[<n>]{<text>}` whose command token ends at
/// `at`: an optional `[...]` and a brace group (`{}` after `\footnotemark`
/// too) are skipped; `at` itself when neither follows.
fn footnote_command_end(src: &str, at: usize) -> usize {
    let bytes = src.as_bytes();
    let mut i = at;
    if bytes.get(i) == Some(&b'[') {
        match src[i..].find(']') {
            Some(close) => i += close + 1,
            None => return at,
        }
    }
    if bytes.get(i) != Some(&b'{') {
        return i;
    }
    let mut depth = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    bytes.len()
}

/// The size declaration in force where TeX reads the space token between
/// the previous inline (ending at `prev_end`) and `span`, from the
/// compiler's sizes of the two neighbours: the previous inline's unless a
/// closing brace precedes the gap's first whitespace (`{\Large x} y`: the
/// group has ended, so the space is read at the next inline's size).
fn space_size(texts: &[&str], prev_end: Option<usize>, span: Span, prev_cpt: u16, next_cpt: u16) -> u16 {
    if prev_cpt == next_cpt {
        return prev_cpt;
    }
    let Some(pe) = prev_end else { return next_cpt };
    let Some(gap) = texts.get(span.document.0).and_then(|t| t.get(pe..span.start)) else { return next_cpt };
    let ws = gap.find(|c: char| c.is_whitespace()).unwrap_or(gap.len());
    if gap[..ws].contains('}') {
        next_cpt
    } else {
        prev_cpt
    }
}

/// The style in force where TeX reads the space token between the previous
/// inline (ending at `prev_end`) and `span`: the first whitespace byte of
/// the gap, which sits inside or outside the closing braces around it.
/// `fallback` when the gap cannot be located.
fn space_style(
    texts: &[&str],
    styles: &[Styles],
    prev_end: Option<usize>,
    span: Span,
    fallback: TextStyle,
) -> TextStyle {
    let Some(pe) = prev_end else { return fallback };
    let Some(src) = texts.get(span.document.0) else { return fallback };
    let no_styles = Styles::default();
    let intervals = styles.get(span.document.0).unwrap_or(&no_styles);
    let Some(gap) = src.get(pe..span.start) else { return fallback };
    // After a replacement token of a user macro (whose span is the `\name`
    // of the invocation) the bytes up to an argument are the call's earlier
    // arguments, not what TeX read: `\pair{\textit{a b}}{c}`'s body space
    // before `#2` is not in `a b`'s italic. Read the call site's font.
    if let Some(bs) = src[..pe].rfind('\\') {
        if is_invocation_span(src, Span { document: span.document, start: bs, end: pe }) {
            return style_at(intervals, bs);
        }
    }
    match gap.find(|c: char| c.is_whitespace()) {
        Some(off) => style_at(intervals, pe + off),
        None => style_at(intervals, pe),
    }
}

/// Appends a segment to the current word or starts a new word.
fn push_segment(items: &mut Vec<Item>, text: String, chars: Vec<CharSrc>, style: TextStyle) {
    let segment = Segment { text, chars, style };
    match items.last_mut() {
        Some(Item::Word(word)) => {
            if let Some(last) = word.segments.last_mut() {
                if last.style == style {
                    last.text.push_str(&segment.text);
                    last.chars.extend(segment.chars);
                    return;
                }
            }
            word.segments.push(segment);
        }
        _ => items.push(Item::Word(Word {
            segments: vec![segment],
        })),
    }
}

/// TeX input ligatures of T1-encoded text: `--` `---` ` `` `` `''` `'`.
/// Each is the T1 slot the font's ligature program would select, resolved
/// to a character through the declared encoding table (never a cast).
fn tex_ligatures(chars: Vec<(char, CharSrc)>) -> Vec<(char, CharSrc)> {
    use crate::ids::{Encoding, EncodingCode};
    let t1 = |code: EncodingCode| -> char { code.to_char(Encoding::T1).expect("declared T1 slot") };
    let mut out: Vec<(char, CharSrc)> = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let (c, s) = chars[i];
        let next = chars.get(i + 1).map(|(c, _)| *c);
        let next2 = chars.get(i + 2).map(|(c, _)| *c);
        let merged = |n: usize, ch: char| -> (char, CharSrc) {
            (
                ch,
                CharSrc {
                    document: s.document,
                    start: s.start,
                    end: chars[i + n - 1].1.end,
                },
            )
        };
        if c == '-' && next == Some('-') && next2 == Some('-') {
            out.push(merged(3, t1(EncodingCode::T1_EMDASH)));
            i += 3;
        } else if c == '-' && next == Some('-') {
            out.push(merged(2, t1(EncodingCode::T1_ENDASH)));
            i += 2;
        } else if c == '`' && next == Some('`') {
            out.push(merged(2, t1(EncodingCode::T1_QUOTEDBLLEFT)));
            i += 2;
        } else if c == '\'' && next == Some('\'') {
            out.push(merged(2, t1(EncodingCode::T1_QUOTEDBLRIGHT)));
            i += 2;
        } else if c == '`' {
            out.push((t1(EncodingCode::T1_QUOTELEFT), s));
            i += 1;
        } else if c == '\'' {
            out.push((t1(EncodingCode::T1_QUOTERIGHT), s));
            i += 1;
        } else {
            out.push((c, s));
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(src: &str) -> Vec<Item> {
        let parsed = flashtex_compiler::parser::parse(src);
        let doc = adapt(&[src], 0, &parsed, &RenderOptions::default(), &Labels::default());
        match &doc.blocks[0] {
            Block::Paragraph { parts, .. } => match &parts[0] {
                ParaPart::Lines(items) => items.clone(),
                _ => panic!(),
            },
            Block::Heading { items, .. } => items.clone(),
            _ => panic!("a rule, picture, chapter or page-style block holds no items"),
        }
    }

    /// The class's list skips are glue, not kerns: `\topsep`, `\partopsep`,
    /// `\itemsep` and `\parsep` all carry the `\@plus`/`\@minus` of
    /// `size1x.clo`'s `\@listI`, and the page builder needs them — a page
    /// whose stretch is short by the 2 pt per `\itemsep` breaks in a
    /// different place from pdfTeX's. An explicit `enumitem` value is a
    /// dimen assignment and *is* rigid.
    #[test]
    fn list_skips_keep_their_stretch_and_shrink() {
        let style = crate::style::Stylesheet::article(10, crate::fonts::Family::ComputerModern, None);
        let src = "\\documentclass{article}\\begin{document}\\begin{itemize}\\item a\\end{itemize}\\end{document}";
        let seps = list_seps(src, "itemize", 1, 10, &style);
        // article/size10.clo \@listI: \topsep 8pt plus 2 minus 4,
        // \parsep 4pt plus 2 minus 1, \itemsep \parsep, \partopsep 2pt
        // plus 1 minus 1.
        assert_eq!((seps.topsep_skip.natural, seps.topsep_skip.stretch, seps.topsep_skip.shrink), (8.0, 2.0, 4.0));
        assert_eq!((seps.partopsep_skip.natural, seps.partopsep_skip.stretch, seps.partopsep_skip.shrink), (2.0, 1.0, 1.0));
        assert_eq!((seps.parsep_skip.natural, seps.parsep_skip.stretch, seps.parsep_skip.shrink), (4.0, 2.0, 1.0));
        assert_eq!((seps.itemsep_skip.natural, seps.itemsep_skip.stretch, seps.itemsep_skip.shrink), (4.0, 2.0, 1.0));

        let rigid = "\\documentclass{article}\\usepackage{enumitem}\\setlist[itemize]{itemsep=3pt,topsep=5pt}\\begin{document}x\\end{document}";
        let seps = list_seps(rigid, "itemize", 1, 10, &style);
        assert_eq!((seps.itemsep_skip.natural, seps.itemsep_skip.stretch, seps.itemsep_skip.shrink), (3.0, 0.0, 0.0));
        assert_eq!((seps.topsep_skip.natural, seps.topsep_skip.stretch, seps.topsep_skip.shrink), (5.0, 0.0, 0.0));
        // `nosep` zeroes all four.
        let nosep = "\\documentclass{article}\\usepackage{enumitem}\\setlist{nosep}\\begin{document}x\\end{document}";
        let seps = list_seps(nosep, "itemize", 1, 10, &style);
        for s in [seps.topsep_skip, seps.partopsep_skip, seps.itemsep_skip, seps.parsep_skip] {
            assert_eq!((s.natural, s.stretch, s.shrink), (0.0, 0.0, 0.0));
        }
    }

    /// natbib's `\ProcessOptions` (not the starred form) executes options in
    /// *declaration* order, and `numbers`/`super` are declared before
    /// `authoryear`, so the last of the three to be declared decides.
    #[test]
    fn natbib_author_year_is_the_default_and_numbers_turns_it_off() {
        let load = |options: &str| {
            format!("\\documentclass{{article}}\\usepackage[{options}]{{natbib}}\\begin{{document}}x\\end{{document}}")
        };
        assert!(natbib_author_year(&load("")));
        assert!(natbib_author_year(&load("round")));
        assert!(natbib_author_year(&load("authoryear,round")));
        assert!(natbib_author_year(&load("numbers,authoryear")));
        assert!(natbib_author_year(&load("authoryear,numbers")));
        assert!(!natbib_author_year(&load("numbers")));
        assert!(!natbib_author_year(&load("super")));
        assert!(!natbib_author_year(&load("numbers,square")));
        // A document that never loads natbib keeps the class geometry.
        assert!(!natbib_author_year(
            "\\documentclass{article}\\begin{document}x\\end{document}"
        ));
        // natbib among several packages in one `\usepackage`.
        assert!(natbib_author_year(
            "\\usepackage{amsmath, natbib}\\begin{document}x\\end{document}"
        ));
        assert!(!natbib_author_year(
            "\\usepackage{amsmath}\\usepackage{nameref}\\begin{document}x"
        ));
    }

    /// A citation's runs all carry the command's own span, and the text they
    /// set can be exactly as long as it — `\citet{knuthplass1981}` is 22
    /// bytes and sets 22 characters — so the length test alone would treat
    /// generated text as the source's own bytes and keep its blanks as
    /// glyphs instead of interword glue.
    #[test]
    fn citation_text_is_never_the_source_s_own_bytes() {
        let span = |source: &str| Span::new(0, source.len());
        for source in [
            "\\citet{knuthplass1981}",
            "\\citep[see][p.~7]{k}",
            "\\cite{k}",
            "\\citealp{k}",
            "\\citeyearpar{k}",
            "\\Citet{k}",
            "\\Citeauthor{k}",
            "\\citetext{cf.}",
        ] {
            assert!(generated_citation(source, span(source)), "{source}");
        }
        for source in ["\\citation{k}", "\\emph{k}", "plain words", "\\ref{k}"] {
            assert!(!generated_citation(source, span(source)), "{source}");
        }
    }

    #[test]
    fn text_symbols_logos_rules_kerns_and_control_space_become_items() {
        let src = "\\AA ngstr \\LaTeX{} and \\TeX\\ x \\S\\,4 \\rule[-1pt]{2pt}{3pt} y";
        let it = items(src);
        assert_eq!(shape(&it), "WSLSWSLSWSWKWSRSW", "{it:?}");
        let Item::Word(first) = &it[0] else { panic!() };
        assert_eq!(first.text(), "\u{00C5}ngstr");
        // The symbol's source is its control word.
        let c = &first.segments[0].chars[0];
        assert_eq!(&src[c.start..c.end], "\\AA");
        let Item::Word(section) = &it[10] else { panic!() };
        assert_eq!(section.text(), "\u{00A7}");
    }

    #[test]
    fn accents_and_dashes_compose_with_exact_sources() {
        let src = "Na\\\"ive caf\\'e --- dash.";
        let it = items(src);
        let words: Vec<String> = it
            .iter()
            .filter_map(|i| match i {
                Item::Word(w) => Some(w.text()),
                _ => None,
            })
            .collect();
        assert_eq!(words, vec!["Naïve", "café", "—", "dash."]);
        if let Item::Word(w) = &it[0] {
            let c = &w.segments[0].chars[2];
            assert_eq!(&src[c.start..c.end], "\\\"i");
        }
        if let Item::Word(w) = &it[4] {
            let c = &w.segments[0].chars[0];
            assert_eq!(&src[c.start..c.end], "---");
        }
    }

    #[test]
    fn styles_and_gaps_are_recovered_from_source() {
        let src = "Plain, \\textbf{bold}, \\emph{em\\emph{up}} and \\textbf{\\emph{bi}}.";
        let it = items(src);
        let mut seen = Vec::new();
        for i in &it {
            match i {
                Item::Word(w) => {
                    for s in &w.segments {
                        seen.push((s.text.clone(), s.style.bold, s.style.italic));
                    }
                }
                Item::Space { .. } => seen.push((" ".into(), false, false)),
                _ => {}
            }
        }
        assert_eq!(
            seen,
            vec![
                ("Plain,".to_string(), false, false),
                (" ".into(), false, false),
                ("bold".into(), true, false),
                (",".into(), false, false),
                (" ".into(), false, false),
                ("em".into(), false, true),
                ("up".into(), false, false),
                (" ".into(), false, false),
                ("and".into(), false, false),
                (" ".into(), false, false),
                ("bi".into(), true, true),
                (".".into(), false, false),
            ]
        );
    }

    #[test]
    fn class_options_and_parindent_are_read_from_source() {
        let src = "\\documentclass[12pt]{article}\n\\setlength{\\parindent}{0pt}\n\\begin{document}x\\end{document}";
        assert_eq!(class_options(src).as_deref(), Some("12pt"));
        assert_eq!(parindent(src, 12), Some(0.0));
        let doc = adapt(&[src], 0, &flashtex_compiler::parser::parse(src), &RenderOptions::default(), &Labels::default());
        assert_eq!(doc.style.body_size_pt, 12.0);
        assert_eq!(doc.style.parindent_pt, 0.0);
        let src2 = "\\documentclass{article}\n\\begin{document}x\\end{document}";
        let doc2 = adapt(&[src2], 0, &flashtex_compiler::parser::parse(src2), &RenderOptions::default(), &Labels::default());
        assert_eq!(doc2.style.body_size_pt, 10.0);
        assert_eq!(doc2.style.parindent_pt, 15.0);
    }

    fn adapted(src: &str) -> Doc {
        adapt(&[src], 0, &flashtex_compiler::parser::parse(src), &RenderOptions::default(), &Labels::default())
    }

    #[test]
    fn addtolength_parindent_accumulates_after_setlength() {
        let src = "\\documentclass{article}\n\\setlength{\\parindent}{10pt}\n\\addtolength{\\parindent}{5pt}\n\\begin{document}x\\end{document}";
        let doc = adapted(src);
        assert!(
            (doc.style.parindent_pt - 15.0).abs() < 1e-6,
            "10pt + 5pt must be 15pt, got {}",
            doc.style.parindent_pt
        );
        let body = "\\documentclass{article}\\begin{document}\\setlength{\\parindent}{0pt}x\\end{document}";
        assert!((adapted(body).style.parindent_pt).abs() < 1e-9);
    }

    #[test]
    fn addtolength_parskip_keeps_class_stretch() {
        let src = "\\documentclass{article}\n\\addtolength{\\parskip}{6pt}\n\\begin{document}\nOne\n\nTwo\n\\end{document}";
        let skip = adapted(src).style.parskip;
        assert!(
            (skip.natural - 6.0).abs() < 1e-6,
            "natural {}, want 6pt",
            skip.natural
        );
        assert!(
            (skip.stretch - 1.0).abs() < 1e-6,
            "stretch {}, want class plus 1pt",
            skip.stretch
        );
        let with_plus = adapted(
            "\\documentclass{article}\\setlength{\\parskip}{6pt plus 2pt minus 1pt}\\begin{document}x\\end{document}",
        )
        .style
        .parskip;
        assert!((with_plus.natural - 6.0).abs() < 1e-6);
        assert!((with_plus.stretch - 2.0).abs() < 1e-6);
        assert!((with_plus.shrink - 1.0).abs() < 1e-6);
        let plain = adapted(
            "\\documentclass{article}\\setlength{\\parskip}{6pt}\\begin{document}x\\end{document}",
        )
        .style
        .parskip;
        assert!((plain.natural - 6.0).abs() < 1e-6);
        assert!(plain.stretch.abs() < 1e-9, "plain setlength is a fixed skip");
    }

    #[test]
    fn geometry_setlength_paperwidth_updates_mediabox() {
        // pdflatex (TeX Live 2026): with geometry,
        // `\pdfpagewidth=361.34999pt` (=5in) and `\paperwidth=361.34999pt`;
        // without geometry, `\pdfpagewidth=614.295pt` (US Letter) while
        // `\paperwidth=361.34999pt`.
        let with = "\\documentclass{article}\n\\usepackage[margin=1in]{geometry}\n\\setlength{\\paperwidth}{5in}\n\\begin{document}x\\end{document}";
        let without = "\\documentclass{article}\n\\setlength{\\paperwidth}{5in}\n\\begin{document}x\\end{document}";
        let want = flashtex_class_geometry::Sp::parse("5in").unwrap();
        let letter = flashtex_class_geometry::Sp::parse("8.5in").unwrap();
        let w = adapted(with)
            .style
            .class_geometry
            .as_ref()
            .unwrap()
            .frame
            .pdf_page_width;
        assert_eq!(w, want, "geometry copies paperwidth into the MediaBox");
        let wo = adapted(without)
            .style
            .class_geometry
            .as_ref()
            .unwrap()
            .frame
            .pdf_page_width;
        assert_eq!(wo, letter, "without geometry the MediaBox is unchanged");
    }

    #[test]
    fn subparagraph_indent_follows_final_parindent() {
        let src = "\\documentclass{article}\n\\setlength{\\parindent}{0pt}\n\\begin{document}\n\\subparagraph{Heading} body\n\\end{document}";
        let indent = adapted(src)
            .style
            .class_geometry
            .as_ref()
            .unwrap()
            .heading("subparagraph")
            .unwrap()
            .indent;
        assert_eq!(indent, flashtex_class_geometry::Sp::ZERO);
    }

    fn article_tw() -> f64 {
        adapted("\\documentclass{article}\\begin{document}x\\end{document}").style.text_width_pt
    }

    #[test]
    fn preamble_scan_skips_commented_begin_document() {
        let src = "\\documentclass{article}\n% \\begin{document}\n\\setlength{\\textwidth}{6in}\n\\begin{document}x\\end{document}";
        assert!(
            (adapted(src).style.text_width_pt - adapted(
                "\\documentclass{article}\\setlength{\\textwidth}{6in}\\begin{document}x\\end{document}"
            )
            .style
            .text_width_pt)
                .abs()
                < 1e-6
        );
    }

    #[test]
    fn preamble_scan_strips_comments_inside_dimension_groups() {
        let src = "\\documentclass{article}\\setlength{\\textwidth}{6in%\n}\\begin{document}x\\end{document}";
        assert!(
            (adapted(src).style.text_width_pt
                - adapted(
                    "\\documentclass{article}\\setlength{\\textwidth}{6in}\\begin{document}x\\end{document}"
                )
                .style
                .text_width_pt)
                .abs()
                < 1e-6
        );
    }

    #[test]
    fn preamble_scan_ignores_setlength_in_newcommand_body() {
        let src = "\\documentclass{article}\n\\setlength{\\textwidth}{5in}\n\\newcommand{\\unused}{\\setlength{\\textwidth}{6in}}\n\\begin{document}x\\end{document}";
        assert!(
            (adapted(src).style.text_width_pt
                - adapted(
                    "\\documentclass{article}\\setlength{\\textwidth}{5in}\\begin{document}x\\end{document}"
                )
                .style
                .text_width_pt)
                .abs()
                < 1e-6
        );
    }

    #[test]
    fn preamble_scan_does_not_leak_grouped_setlength() {
        let src = "\\documentclass{article}\n{\\setlength{\\textwidth}{6in}}\n\\begin{document}x\\end{document}";
        assert!(
            (adapted(src).style.text_width_pt - article_tw()).abs() < 1e-6,
            "grouped assignment must restore, got {}",
            adapted(src).style.text_width_pt
        );
    }

    #[test]
    fn preamble_scan_finds_begin_document_with_whitespace() {
        let pre = "\\documentclass{article}\\setlength{\\textwidth}{6in}\\begin {document}x\\end{document}";
        let body = "\\documentclass{article}\\begin {document}\\setlength{\\textwidth}{6in}x\\end{document}";
        assert!(
            (adapted(pre).style.text_width_pt
                - adapted(
                    "\\documentclass{article}\\setlength{\\textwidth}{6in}\\begin{document}x\\end{document}"
                )
                .style
                .text_width_pt)
                .abs()
                < 1e-6
        );
        assert!(
            (adapted(body).style.text_width_pt - article_tw()).abs() < 1e-6,
            "body page geometry after \\begin {{document}} must not apply, got {}",
            adapted(body).style.text_width_pt
        );
    }

    #[test]
    fn preamble_scan_skips_begin_document_in_macro_body() {
        let src = "\\documentclass{article}\\newcommand{\\fake}{\\begin{document}}\\setlength{\\textwidth}{6in}\\begin{document}x\\end{document}";
        let want = adapted(
            "\\documentclass{article}\\setlength{\\textwidth}{6in}\\begin{document}x\\end{document}",
        )
        .style
        .text_width_pt;
        assert!(
            (adapted(src).style.text_width_pt - want).abs() < 1e-6,
            "setlength after a fake \\begin{{document}} in a macro body must apply, got {}",
            adapted(src).style.text_width_pt
        );
    }

    #[test]
    fn preamble_scan_is_linear_in_source_length() {
        let mut src = String::with_capacity(1_200_000);
        src.push_str("\\documentclass{article}\n");
        while src.len() < 1_000_000 {
            src.push_str("\\setlength{\\textwidth}{6in}\n");
        }
        src.push_str("\\begin{document}x\\end{document}");
        let setup = document_setup(&src, true, "");
        let mut resolved = flashtex_class_geometry::resolve(&setup);
        let t0 = std::time::Instant::now();
        apply_preamble_lengths(
            &src,
            &mut resolved,
            10,
            crate::fonts::Family::ComputerModern,
            false,
        );
        let elapsed = t0.elapsed();
        assert_eq!(
            resolved.params.textwidth,
            flashtex_class_geometry::Sp::parse("6in").unwrap()
        );
        assert!(
            elapsed < std::time::Duration::from_secs(8),
            "preamble length scan of {} bytes took {elapsed:?} (quadratic brace_depth?)",
            src.len()
        );
    }

    #[test]
    #[ignore = "known limit: \\input'd preambles are not in the adapter source string"]
    fn preamble_scan_does_not_see_input_files() {
        let src = "\\documentclass{article}\n\\input{layout}\n\\begin{document}x\\end{document}";
        let _ = adapted(src);
        panic!("not implemented: scan \\input'd preambles");
    }

    #[test]
    #[ignore = "known limit: next_command is alphabetic, so \\@setlength is missed"]
    fn preamble_scan_does_not_see_at_setlength() {
        let src = "\\documentclass{article}\n\\makeatletter\n\\@setlength{\\textwidth}{6in}\n\\makeatother\n\\begin{document}x\\end{document}";
        assert!(
            (adapted(src).style.text_width_pt - article_tw()).abs() < 1e-6,
            "\\@setlength is not implemented"
        );
    }

    /// Shorthand for an item list: `W` word, `S` space, `F` fill, `Q` quad.
    fn shape(items: &[Item]) -> String {
        items
            .iter()
            .map(|i| match i {
                Item::Word(_) => 'W',
                Item::Space { .. } => 'S',
                Item::HFill { .. } => 'F',
                Item::Quad { .. } => 'Q',
                Item::HSpace { .. } => 'H',
                Item::Logo { .. } => 'L',
                Item::Rule { .. } => 'R',
                Item::Kern { .. } => 'K',
                _ => '?',
            })
            .collect()
    }

    #[test]
    fn compiler_glue_is_taken_once_and_eats_the_space_after_its_control_word() {
        // The compiler (pin `3d3d5ae3`) emits `Inline::HFill` inside titles,
        // through macro bodies too, and `Inline::TextGlue` for text-mode
        // `\quad`/`\qquad`; the pipeline must not add a second fill from
        // the macro body's bytes, and the whitespace after the control word
        // is TeX's to eat.
        let src = "\\documentclass[11pt]{article}\n\\newcommand{\\problem}[2]{\\subsection*{Problem #1 \\hfill \\normalfont[#2 points]}}\n\\begin{document}\n\\problem{1}{4}\n\\subsection*{Bonus \\hfill \\normalfont[1 pt]}\nA \\quad B\\qquad C.\n\\end{document}\n";
        let doc = adapt(&[src], 0, &flashtex_compiler::parser::parse(src), &RenderOptions::default(), &Labels::default());
        let shapes: Vec<String> = doc
            .blocks
            .iter()
            .map(|b| match b {
                Block::Heading { items, .. } => shape(items),
                Block::Paragraph { parts, .. } => parts
                    .iter()
                    .map(|p| match p {
                        ParaPart::Lines(items) => shape(items),
                        ParaPart::Display { .. } | ParaPart::Rows { .. } => "D".to_string(),
                    })
                    .collect(),
                Block::Rule { .. } => "R".to_string(),
                Block::Picture { .. } => "P".to_string(),
                Block::Chapter { .. } => "C".to_string(),
                Block::Part { .. } => "P".to_string(),
                Block::Chrome { .. } => "M".to_string(),
                Block::Title { .. } => "T".to_string(),
                Block::ClearPage { .. } => "N".to_string(),
                Block::TocEntry(..) => "E".to_string(),
                Block::LongTable { .. } => "L".to_string(),
            })
            .collect();
        // `Problem 1 \hfill \normalfont[4 points]`: one fill, no space after it.
        // `A \quad B\qquad C.`: the space before `\quad` stays, the one after
        // is eaten; `B\qquad` has none before.
        assert_eq!(shapes, ["WSWSFWSW", "WSFWSW", "WSQWQW"]);
    }

    #[test]
    fn space_factor_follows_sentence_punctuation() {
        let it = items("End. Next, more: A. B; (c.) d");
        let factors: Vec<u32> = it
            .iter()
            .filter_map(|i| match i {
                Item::Space { factor, .. } => Some(*factor),
                _ => None,
            })
            .collect();
        // "A." and "B;" stay 1000 (§1034: a code above 1000 after an uppercase
        // letter), ")" keeps the factor of the "." before it.
        assert_eq!(factors, vec![3000, 1250, 2000, 1000, 1000, 3000]);
    }

    /// The family every character of an item carries, as one string of
    /// `r`/`t`/`s` per word (`\rmfamily`/`\ttfamily`/`\sffamily`).
    fn families(items: &[Item]) -> String {
        use crate::nfss::FamilyKind;
        items
            .iter()
            .filter_map(|i| match i {
                Item::Word(w) => Some(w.segments.iter().map(|s| match s.style.family {
                    FamilyKind::Rm => 'r',
                    FamilyKind::Tt => 't',
                    FamilyKind::Sf => 's',
                })),
                _ => None,
            })
            .flatten()
            .collect()
    }

    /// url.sty sets a URL in `\UrlFont`, whose default is `\ttfamily`
    /// (url.sty 4.3 `\def\Url@FormatString`), and hyperref keeps that font.
    /// Measured against pdflatex (TeX Live 2025, 11pt `article`, T1): the
    /// `\hbox` of `\url{https://example.org/flashtex/glossary}` is
    /// 209.35973pt, the same as `\texttt` of the same string, where the
    /// roman setting this used to produce is 32pt narrower.
    #[test]
    fn url_and_nolinkurl_are_set_in_the_typewriter_family() {
        let it = items("A \\url{https://example.org/x} B");
        assert_eq!(families(&it), "rtr", "{it:?}");
        let it = items("A \\nolinkurl{https://example.org/x} B");
        assert_eq!(families(&it), "rtr", "{it:?}");
        // `\href` typesets only its second argument, in the ambient family.
        let it = items("A \\href{https://example.org/x}{link text} B");
        assert_eq!(families(&it), "rrrr", "{it:?}");
    }

    /// A URL's argument is read as raw source bytes (url.sty makes every
    /// character "other"; the compiler does the same in
    /// `parser::url_argument`), so the style scan must not interpret what is
    /// inside it. A `%` used to start a comment and swallow the rest of the
    /// line, losing the style of everything after the URL.
    #[test]
    fn a_percent_or_brace_inside_a_url_does_not_disturb_a_later_font_command() {
        let it = items("A \\url{https://e.org/a%20b} \\textbf{bold} C");
        assert_eq!(families(&it), "rtrr", "{it:?}");
        let Item::Word(bold) = it.iter().filter(|i| matches!(i, Item::Word(_))).nth(2).unwrap() else {
            panic!()
        };
        assert_eq!(bold.text(), "bold");
        assert!(bold.segments[0].style.bold, "the \\textbf after the URL is still bold: {bold:?}");
        // A brace pair inside the URL is balanced, not a group.
        let it = items("A \\url{https://e.org/{x}} \\textbf{bold} C");
        assert_eq!(families(&it), "rtrr", "{it:?}");
    }

    /// The typewriter family covers the whole `\url{...}`, not just its
    /// braced argument: the compiler gives every run it splits the URL into
    /// the span of the entire command (`parser::push_url_text`), and the
    /// style is read at that span's first byte, the backslash.
    #[test]
    fn the_url_style_interval_starts_at_the_backslash_and_ends_at_the_brace() {
        let src = "x \\url{ab} y";
        let intervals = style_intervals(src);
        let url = intervals
            .iter()
            .find(|(_, _, c, _)| matches!(c, crate::nfss::Command::Family(crate::nfss::FamilyKind::Tt)))
            .expect("the URL contributes a typewriter interval");
        assert_eq!(&src[url.0..url.1], "\\url{ab}", "{intervals:?}");
        // The text after the URL is outside it.
        assert_eq!(Styles::new(src, intervals, crate::nfss::Scheme::LmT1).at(src.find('y').unwrap()).family,
                   crate::nfss::FamilyKind::Rm);
    }

    /// A macro body's declarations around `#k` cover argument `k` at each
    /// call (`macro_argument_intervals`), and a redefinition nested inside
    /// the body (whose position is before the body's end) does not panic.
    #[test]
    fn a_macro_body_declaration_covers_its_argument() {
        let src = "\\newcommand{\\note}[1]{{\\bfseries #1}}\nA \\note{bold} C";
        let st = Styles::new(src, style_intervals(src), crate::nfss::Scheme::LmT1);
        assert!(st.at(src.find("bold").unwrap()).bold);
        assert!(!st.at(src.find('C').unwrap()).bold);
        assert!(!st.at(src.find('A').unwrap()).bold);
        let nested = "\\newcommand{\\a}[1]{\\def\\a{x}{\\bfseries #1}}\n\\a{y} z";
        let _ = style_intervals(nested);
    }

    /// `\verb`, the `verbatim` environment and `lstlisting` are set in the
    /// typewriter family, like `\url` and for the same reason: the compiler
    /// marks the run mono (`parser::Inline::Verbatim`, `Block::Verbatim`)
    /// but the pipeline re-derives the family from the source, and these
    /// were in none of its tables.
    #[test]
    fn verbatim_constructs_are_set_in_the_typewriter_family() {
        assert_eq!(families(&items("A \\verb|x| B")), "rtr");
        assert_eq!(families(&items("A \\verb*|x| B")), "rtr");
        assert_eq!(families(&items("A \\lstinline|x| B")), "rtr");
        assert_eq!(families(&items("A \\lstinline[language=C]|x| B")), "rtr");
        assert_eq!(families(&items("\\begin{verbatim}\nx\n\\end{verbatim}")), "t");
        assert_eq!(families(&items("\\begin{lstlisting}[language=C]\nx\n\\end{lstlisting}")), "t");
    }

    /// A verbatim body is raw source bytes, so the style scan must skip it
    /// rather than read it as LaTeX. A `%` inside `\verb` used to start a
    /// comment and swallow the rest of the line's styles, and a lone brace
    /// used to corrupt the group stack that decides where a declaration
    /// ends. Both are reachable from ordinary code listings.
    #[test]
    fn a_comment_or_brace_inside_verbatim_does_not_disturb_a_later_font_command() {
        let it = items("A \\verb|100%| \\textbf{bold} C");
        assert_eq!(families(&it), "rtrr", "{it:?}");
        assert!(
            it.iter().any(|i| matches!(i, Item::Word(w) if w.segments.iter().any(|s| s.style.bold))),
            "the \\textbf after the \\verb is still bold: {it:?}"
        );
        // An unbalanced brace in the body is a character, not a group.
        let it = items("A \\verb|{| \\textbf{bold} C");
        assert_eq!(families(&it), "rtrr", "{it:?}");
        // `\end{verbatim}` closes the body, so everything before it is
        // text: a `\begin`, a `%` and a lone `{` are characters, not a
        // nested environment, a comment and a group.
        let it = items("\\begin{verbatim}\n\\begin{x} 50% {\n\\end{verbatim}");
        let words: Vec<String> = it
            .iter()
            .filter_map(|i| match i {
                Item::Word(w) => Some(w.segments.iter().map(|s| s.text.clone()).collect()),
                _ => None,
            })
            .collect();
        assert_eq!(words, vec!["\\begin{x}", "50%", "{"], "{it:?}");
        assert!(
            it.iter().all(|i| match i {
                Item::Word(w) => w.segments.iter().all(|s| s.style.literal),
                _ => true,
            }),
            "{it:?}"
        );
    }

    /// Verbatim suppresses every ligature of the input (`\@noligs`), which
    /// `\texttt` over the same characters does not. Measured against
    /// pdflatex (TeX Live 2025, 12pt `article`, T1, `ectt1200`, every
    /// character 6.1735pt): `\verb|x--y|` is 24.69397pt = 4 characters,
    /// while `\ttfamily x--y` is 18.52048pt = 3, the `--` having ligated
    /// into an endash. So the family alone cannot carry this.
    #[test]
    fn verbatim_suppresses_input_ligatures_but_texttt_keeps_them() {
        let text = |it: &[Item]| -> String {
            it.iter()
                .filter_map(|i| match i {
                    Item::Word(w) => Some(w.text()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("|")
        };
        assert_eq!(text(&items("\\verb|x--y|")), "x--y");
        assert_eq!(text(&items("\\begin{verbatim}\nx--y\n\\end{verbatim}")), "x--y");
        // `\texttt` is not verbatim: the `--` is still an endash.
        assert_eq!(text(&items("\\texttt{x--y}")), "x\u{2013}y");
        assert_eq!(text(&items("plain x--y")), "plain|x\u{2013}y");
    }

    /// A blank in verbatim is a control space (`\@vobeyspaces`), not an
    /// interword space token: rigid, unbreakable, and preceded by
    /// `\leavevmode` so that the indentation of a code line is not
    /// discarded at the line break in front of it (TeX §879).
    #[test]
    fn a_verbatim_blank_is_rigid_and_survives_at_the_start_of_a_line() {
        let it = items("\\verb|a b|");
        let space = it.iter().find(|i| matches!(i, Item::Space { .. })).expect("{it:?}");
        let Item::Space { factor, no_break, style } = space else { panic!() };
        assert_eq!(*factor, 1000, "a control space ignores the space factor");
        assert!(*no_break, "`\\penalty\\@M` in front of the blank");
        assert!(style.literal);
        // An indented listing line opens with the `\leavevmode` box.
        let it = items("\\begin{verbatim}\na\n    b\n\\end{verbatim}");
        let lead = it.iter().position(|i| matches!(i, Item::LeaveVmode));
        let brk = it.iter().position(|i| matches!(i, Item::LineBreak { .. }));
        assert!(lead.is_some() && brk.is_some() && lead > brk, "{it:?}");
    }
}
