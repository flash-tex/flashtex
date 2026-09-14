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
use flashtex_compiler::parser::{Block as CBlock, Inline, Parsed};
use flashtex_compiler::text_builtins::{TextDimen, TextLogo, TextRule};
use flashtex_compiler::{DocumentId, Span};

use flashtex_class_geometry::{ClassKind, DocumentSetup, GeometryInput, PageStyle};

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
    /// `\hfill`/`\hfil` (compiler `Inline::HFill`): infinitely stretchable
    /// glue; a legal break point that is discarded at a line break. `fill`
    /// is the `\hfill` order (it beats `\parfillskip`'s `fil`); the
    /// compiler does not distinguish the two, so the order is re-read from
    /// the source bytes (`\hfill` when they are not `\hfil`).
    HFill { fill: bool },
    /// `\hspace{<dimen>}` (compiler `Inline::HSpace`): fixed glue in points.
    HSpace { pt: f64 },
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

/// How a paragraph-shape environment began (see [`Block::Paragraph`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvOpen {
    /// `\begin{...}` was read in vertical mode (after a blank line, a
    /// heading, a rule or at the document start): `\partopsep` is added.
    pub vmode: bool,
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
}

fn inlines_of(block: &CBlock) -> &[Inline] {
    match block {
        CBlock::Paragraph(i) => i,
        CBlock::ListItem { content, .. } | CBlock::Heading { content, .. } | CBlock::FigureCaption { content } | CBlock::Styled { content, .. } => content,
        // `\maketitle`'s parts are lowered to `Styled` paragraphs before
        // the block walk (`lower_blocks`); only the title is visible here.
        CBlock::TitleBlock { title, .. } => title,
        CBlock::VSpace { .. } | CBlock::Rule { .. } | CBlock::PageBreak | CBlock::Verbatim { .. } | CBlock::TableOfContents { .. } | CBlock::VFill => &[],
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
fn lower_blocks(texts: &[&str], blocks: &[CBlock], stash_titles: bool) -> (Vec<CBlock>, Vec<(&'static str, Span, String)>, Vec<StashedTitle>) {
    use flashtex_compiler::parser::{FontSizeLevel, ParagraphStyle, TextFamily, TextStyle as CStyle};
    let mut out: Vec<CBlock> = Vec::with_capacity(blocks.len());
    let mut limitations: Vec<(&'static str, Span, String)> = Vec::new();
    let mut titles: Vec<StashedTitle> = Vec::new();
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
    for block in blocks {
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
            CBlock::Verbatim { lines, span } => {
                let mut content: Vec<Inline> = Vec::with_capacity(lines.len() * 2);
                for (i, line) in lines.iter().enumerate() {
                    if i > 0 {
                        // The break owns the bytes between the lines so no
                        // interword space is read across it.
                        let prev = lines[i - 1].span;
                        content.push(Inline::LineBreak {
                            span: Span {
                                document: line.span.document,
                                start: prev.end.min(line.span.start),
                                end: line.span.start,
                            },
                        });
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
                limitations.push((
                    "unsupported_block",
                    *span,
                    format!("verbatim ({} line(s)) set as a flush-left paragraph in the body face with forced line breaks: the pipeline has no monospaced face or literal-text block", lines.len()),
                ));
                out.push(CBlock::Styled {
                    style: ParagraphStyle::FlushLeft,
                    content,
                    lists: Vec::new(),
                    line_break_before: None,
                });
            }
            // Set by `crate::toc` from the source command; the block stays
            // as the position a following `\clearpage` is measured from.
            CBlock::TableOfContents { .. } => out.push(block.clone()),
            CBlock::TitleBlock { title, authors, date } if stash_titles => titles.push((title.clone(), authors.clone(), date.clone())),
            CBlock::TitleBlock { title, authors, date } => {
                if let Some(at) = first {
                    limitations.push((
                        "unsupported_block",
                        at,
                        "\\maketitle set as centred paragraphs (title \\LARGE, authors/date \\large): article's exact \\@maketitle skips and \\thanks are not applied".to_string(),
                    ));
                }
                for (part, size) in [(Some(title), FontSizeLevel::Large3), (Some(authors), FontSizeLevel::Large1), (date.as_ref(), FontSizeLevel::Large1)] {
                    let Some(part) = part else { continue };
                    if part.is_empty() {
                        continue;
                    }
                    out.push(CBlock::Styled {
                        style: ParagraphStyle::Center,
                        content: sized(part, size),
                        lists: Vec::new(),
                        line_break_before: None,
                    });
                }
            }
            CBlock::VFill => pending_vfill += 1,
            other => out.push(other.clone()),
        }
    }
    if pending_vfill > 0 {
        let at = out.iter().rev().flat_map(|b| inlines_of(b).iter().map(inline_span).last()).next().unwrap_or(Span {
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
    (out, limitations, titles)
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
        if let Inline::LineBreak { span } = inline {
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
        for inline in parsed.blocks.iter().flat_map(inlines_of) {
            if let Inline::Label { key, value, .. } = inline {
                values.insert(key.clone(), value.clone());
            }
        }
        Labels {
            values,
            ..Labels::default()
        }
    }

    /// Whether any `\pageref` in the parse needs a page number.
    pub fn needs_pages(parsed: &Parsed) -> bool {
        parsed
            .blocks
            .iter()
            .flat_map(inlines_of)
            .any(|i| matches!(i, Inline::Reference { page: true, .. }))
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
    let mut style = Stylesheet::from_resolved(
        &flashtex_class_geometry::resolve(&document_setup(source, explicit_class.is_some(), &class_options)),
        Stylesheet::family_for(&parsed.packages, t1_encoding(source)),
    );
    // The class's `\parindent` (`size1x.clo`: 15pt / 17pt / 1.5em; `1em` in
    // two-column mode) comes with the resolved frame.
    let em_ex = ec_em_ex(size, style.family);
    style.parindent_pt = setlength_in(source, "parindent", size, em_ex).unwrap_or(if explicit_class.is_some() {
        style.parindent_pt
    } else {
        options.default_parindent_pt
    });
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
    // `\setlength{\parskip}{...}`: a fixed skip (no stretch) replaces
    // article's `0pt plus 1pt`.
    if let Some(pt) = setlength_in(source, "parskip", size, em_ex) {
        style.parskip = crate::style::Skip::fixed(pt);
    }
    let secnumdepth = counter(source, "secnumdepth").unwrap_or(options.default_secnumdepth);
    style.nfss = crate::nfss::Scheme::for_document(&parsed.packages, t1_encoding(source));
    let styles: Vec<Styles> = texts.iter().map(|t| Styles::new(style_intervals(t), style.nfss)).collect();
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
    let (mut lowered, mut limitations, stashed) = lower_blocks(texts, &parsed.blocks, stash_titles);
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
    // `\@sect` writes `\numberline` up to the class's `secnumdepth`
    // (article.cls 3, report/book.cls 2) when the document declares one.
    let toc_secnumdepth = counter(source, "secnumdepth").unwrap_or(match (explicit_class.is_some(), has_chapters) {
        (true, true) => 2,
        (true, false) => 3,
        (false, _) => options.default_secnumdepth,
    });
    let mut toc_records: Vec<crate::toc::Record> = Vec::new();
    let mut toc_lists: Vec<(usize, crate::toc::ListKind, Span, bool)> = Vec::new();
    let mut toc_pending: Vec<String> = Vec::new();
    let mut chapter_starts: Vec<(usize, String)> = Vec::new();
    let mut after_heading = false;
    // Last source span of the previous paragraph block (None after a
    // heading or rule), for rejoining a display with its paragraph.
    let mut prev_para_end: Option<Span> = None;
    for unit in split_at_page_breaks(texts, &lowered, size, &style) {
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
                blocks.push(Block::Heading {
                    level,
                    items,
                    eject_before,
                    vspace_before,
                    number,
                    title,
                    span: number_span,
                });
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
                blocks.push(Block::Paragraph {
                    parts,
                    indent: !after_heading && !caption && styled.is_none() && !after_env && !theorem_item && list.is_none() && !noindent,
                    style: styled.unwrap_or_default(),
                    env_open,
                    env_close: false,
                    eject_before,
                    vspace_before,
                    addvspace_before: unit.addvspace_before,
                    endlist_adjust: unit.endlist_adjust,
                    list,
                    sized: None,
                });
                after_heading = false;
            }
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
    let (abstract_limits, superseded) = crate::abstractenv::apply(texts, &mut blocks, &style);
    limitations.extend(abstract_limits);
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
    for (i, block) in blocks.iter_mut().enumerate() {
        if let Block::Paragraph { style, env_close, .. } = block {
            if *style != ParaStyle::Plain {
                *env_close = styles.get(i + 1).is_none_or(|next| *next != *style);
            }
        }
    }
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
        | Inline::LineBreak { span }
        | Inline::Math { span, .. }
        | Inline::MathRows { span, .. }
        | Inline::Label { span, .. }
        | Inline::Reference { span, .. }
        | Inline::HFill { span }
        | Inline::HSpace { span, .. }
        | Inline::Footnote { span, .. }
        | Inline::Verbatim { span, .. }
        | Inline::TextGlue { span, .. }
        | Inline::Logo { span, .. }
        | Inline::Rule { span, .. }
        | Inline::Kern { span, .. } => *span,
        Inline::Tabular(t) => t.span,
        Inline::ColorBox(b) => b.span,
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
        Inline::Verbatim { text, span, .. } => {
            out.push(("unsupported_block", *span, format!("\\verb {text:?} set in the body face: the pipeline has no monospaced face")));
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

/// Whether the source between `prev` and `next` (same document, in order)
/// holds a page-break command.
fn gap_has_page_break(texts: &[&str], prev: Span, next: Span) -> bool {
    if prev.document != next.document || prev.end > next.start {
        return false;
    }
    let gap = texts.get(next.document.0).and_then(|t| t.get(prev.end..next.start)).unwrap_or("");
    PAGE_BREAKS.iter().any(|c| find_command(gap, c).is_some())
}

fn split_at_page_breaks<'p>(texts: &[&str], blocks: &'p [CBlock], size: u32, style: &Stylesheet) -> Vec<Unit<'p>> {
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
    for block in blocks {
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
        let mut endlist_adjust = 0.0;
        if prev_list && !is_heading {
            if let Some(gap) = first.and_then(gap_before) {
                if let Some(env) = gap_has_list_end(gap) {
                    let src = texts.get(prev_end.map_or(0, |p| p.document.0)).copied().unwrap_or("");
                    let stack = prev_end.map(|p| list_stack_at(src, p.end)).unwrap_or_default();
                    let begin_keys = stack.last().map_or("", |(e, keys)| if *e == env && *e != "thebibliography" { keys } else { "" });
                    let seps = list_seps_with(src, env, 1, size, style, begin_keys);
                    addvspace_before += seps.topsep + if list_vmode { seps.partopsep } else { 0.0 };
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
                let outer_parskip = match stack.len() {
                    n if n > 1 => list_seps(src, stack[n - 2].0, n - 1, size, style).parsep,
                    _ => style.parskip.natural,
                };
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
                                if nb < 0.0 {
                                    vspace_before += nb;
                                } else {
                                    addvspace_before += nb;
                                }
                            } else {
                                addvspace_before += seps.topsep + outer_parskip + if list_vmode { seps.partopsep } else { 0.0 };
                                vspace_before -= seps.parsep;
                            }
                        }
                        _ => addvspace_before += seps.itemsep,
                    }
                }
                list = Some(ListGeom {
                    level: *level,
                    margins: list_margins(src, at.start, size),
                    label: label.clone(),
                    parsep: seps.parsep_skip,
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
            Some(EnvOpen { vmode })
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
        let theorem_item = list.is_none()
            && styled.is_none()
            && first.is_some_and(|f| {
                let gap_start = match prev_end {
                    Some(p) if p.document == f.document && p.end <= f.start => Some(p.end),
                    Some(_) => None,
                    None => Some(0),
                };
                match (texts.get(f.document.0), gap_start) {
                    (Some(t), Some(g)) => opens_theorem_item(t, g, f.start, &theorem_envs),
                    _ => false,
                }
            });
        let in_theorem = theorem_item
            || first.is_some_and(|f| texts.get(f.document.0).is_some_and(|t| in_theorem_environment(t, f.start, &theorem_envs)));
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
                                },
                                eject_before: eject,
                                vspace_before: std::mem::take(&mut vspace_before),
                                addvspace_before: std::mem::take(&mut addvspace_before),
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
                        },
                        eject_before: eject,
                        vspace_before: std::mem::take(&mut vspace_before),
                        addvspace_before: std::mem::take(&mut addvspace_before),
                        endlist_adjust: std::mem::take(&mut endlist_adjust),
                        limitations: std::mem::take(&mut limitations),
                    });
                    eject = false;
                }
            }
            CBlock::VSpace { .. } | CBlock::Rule { .. } | CBlock::PageBreak => unreachable!("handled above"),
            CBlock::Verbatim { .. } | CBlock::TableOfContents { .. } | CBlock::TitleBlock { .. } | CBlock::VFill => unreachable!("lowered by lower_blocks"),
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
    /// `\parsep` with its stretch and shrink.
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
    let mut seps = ListSeps {
        topsep: class.topsep.pt,
        partopsep: class.partopsep.pt,
        itemsep: class.itemsep.pt,
        parsep: class.parsep.pt,
        parsep_skip: crate::style::Skip::new(class.parsep.pt, class.parsep.plus, class.parsep.minus),
    };
    if depth == 1 {
        // The stylesheet's level-1 values are the ones the typesetter
        // reads for `\topsep`; keep both readings identical.
        seps.topsep = style.topsep.natural;
        seps.partopsep = style.partopsep.natural;
        seps.parsep = style.parsep.natural;
        seps.parsep_skip = style.parsep;
    }
    let calls = setlist_calls(source);
    let all_keys = calls.iter().filter(|(envs, _)| setlist_names(envs, env)).map(|(_, keys)| *keys).chain(std::iter::once(begin_keys));
    for keys in all_keys {
        for (key, value) in list_keys(keys) {
            let set_parsep = |seps: &mut ListSeps, pt: f64| {
                seps.parsep = pt;
                seps.parsep_skip = crate::style::Skip::fixed(pt);
            };
            match key {
                "nosep" => {
                    seps.topsep = 0.0;
                    seps.partopsep = 0.0;
                    seps.itemsep = 0.0;
                    set_parsep(&mut seps, 0.0);
                }
                "noitemsep" => {
                    seps.itemsep = 0.0;
                    set_parsep(&mut seps, 0.0);
                }
                _ => {
                    // `em`/`ex` are the EC body font's, as pdfTeX resolves
                    // \setlength/\setlist lengths (hw-residuals-2).
                    let Some(pt) = parse_dimen_in(value, size, ec_em_ex(size, style.family)) else { continue };
                    match key {
                        "topsep" => seps.topsep = pt,
                        "partopsep" => seps.partopsep = pt,
                        "itemsep" => seps.itemsep = pt,
                        "parsep" => set_parsep(&mut seps, pt),
                        _ => {}
                    }
                }
            }
        }
    }
    seps
}

/// Whether `rest` (starting at a `\begin`) opens `itemize`/`enumerate`.
fn list_env_after_begin(rest: &str) -> bool {
    let after = rest.strip_prefix("\\begin").unwrap_or(rest).trim_start();
    after.starts_with("{itemize}") || after.starts_with("{enumerate}") || after.starts_with("{thebibliography}")
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
        if !rest.starts_with("{itemize}") && !rest.starts_with("{enumerate}") && !rest.starts_with("{thebibliography}") {
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
fn gap_has_list_end(gap: &str) -> Option<&'static str> {
    let end = rfind_command(gap, "end")?;
    let rest = gap[end + "\\end".len()..].trim_start();
    ["itemize", "enumerate", "thebibliography"].into_iter().find(|env| rest.strip_prefix('{').is_some_and(|r| r.starts_with(&format!("{env}}}"))))
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
        if !matches!(env, "itemize" | "enumerate" | "thebibliography") {
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
fn list_margins(source: &str, at: usize, size: u32) -> Vec<ListMargin> {
    let calls = setlist_calls(source);
    let class_margin = |depth: usize| ListMargin::Fixed(parse_dimen(&format!("{}em", article_leftmargin_em(depth)), size).unwrap_or(0.0));
    list_stack_at(source, at)
        .iter()
        .enumerate()
        .map(|(i, (env, options))| {
            let depth = i + 1;
            if *env == "thebibliography" {
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
fn opens_theorem_item(text: &str, gap_start: usize, at: usize, envs: &std::collections::HashSet<String>) -> bool {
    if gap_start > at || at > text.len() || !text.is_char_boundary(gap_start) || !text.is_char_boundary(at) {
        return false;
    }
    // The compiler gives the theorem head inline the `\begin` command's own
    // span, so the opener is usually *at* the paragraph's first span rather
    // than in the gap before it; accept either.
    let begin = if text[at..].starts_with("\\begin") {
        Some(at)
    } else {
        rfind_command(&text[gap_start..at], "begin").map(|r| gap_start + r)
    };
    let Some(begin) = begin else {
        return false;
    };
    text[begin..]
        .split_once('{')
        .and_then(|(_, rest)| rest.split_once('}'))
        .is_some_and(|(name, _)| envs.contains(name.trim()))
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
fn style_intervals(source: &str) -> Vec<StyleInterval> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    let mut in_comment = false;
    // Open brace groups (byte of `{`): a declaration (`\bfseries`,
    // `\Large`, ...) lasts to the end of the innermost one, or to the next
    // `\end{...}`/the document end outside any group.
    let mut groups: Vec<usize> = Vec::new();
    while i < bytes.len() {
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
fn strip_command_text(blocks: &mut Vec<CBlock>, document: DocumentId, commands: &[BodyCommand]) {
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
    for block in blocks.iter_mut() {
        match block {
            CBlock::Paragraph(inlines) | CBlock::Styled { content: inlines, .. } | CBlock::ListItem { content: inlines, .. } | CBlock::FigureCaption { content: inlines } => inlines.retain(|i| !inside(i)),
            _ => {}
        }
    }
    blocks.retain(|b| !matches!(b, CBlock::Paragraph(i) | CBlock::Styled { content: i, .. } if i.is_empty()));
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
    scheme: crate::nfss::Scheme,
}

impl Styles {
    fn new(intervals: Vec<StyleInterval>, scheme: crate::nfss::Scheme) -> Styles {
        let mut max_end = Vec::with_capacity(intervals.len());
        let mut m = 0;
        for (_, end, _, _) in &intervals {
            m = m.max(*end);
            max_end.push(m);
        }
        let mut ends: Vec<usize> = intervals.iter().filter(|i| i.3).map(|i| i.1).collect();
        ends.sort_unstable();
        let intervals = intervals.into_iter().map(|(s, e, c, _)| (s, e, c)).collect();
        Styles { intervals, max_end, ends, scheme }
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
pub(crate) fn space_factor(ch: char, previous: u32) -> u32 {
    let code = match ch {
        '.' | '?' | '!' => 3000,
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
            Inline::HFill { .. } => 6u8.hash(&mut h),
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
    // Pushes the space `space_between` found.
    let push_gap = |items: &mut Vec<Item>, space: bool, style: TextStyle, factor: u32| {
        if space {
            items.push(Item::Space { style, factor, no_break: false });
        }
    };

    for inline in resolved.iter() {
        match &**inline {
            Inline::Label { key, .. } => items.push(Item::Label { key: key.clone() }),
            Inline::Reference { .. } | Inline::Verbatim { .. } => unreachable!("lowered by lower_inline above"),
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
                let note = text.as_ref().map(|t| items_from_inlines_styled(texts, t, styles, labels, size, false, compiler_weight));
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
            Inline::LineBreak { span } => {
                let skip_pt = line_break_skip(text_of(span.document), span.end, size).unwrap_or(0.0);
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
            Inline::HFill { span } | Inline::HSpace { span, .. } | Inline::TextGlue { span, .. } => {
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
                    Inline::HSpace { pt, .. } => (Item::HSpace { pt: *pt }, "\\hspace"),
                    Inline::TextGlue { em, .. } => (Item::Quad { em: *em }, if *em >= 2.0 { "\\qquad" } else { "\\quad" }),
                    _ => {
                        let fill = !is_control_word(text_of(span.document), span.start, "hfil");
                        (Item::HFill { fill }, if fill { "\\hfill" } else { "\\hfil" })
                    }
                };
                let gap = space_between(prev_end, prev_span, *span, Some(word), after_control_word);
                let mut gap_style = space_style(texts, styles, prev_end, *span, TextStyle::default());
                gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                items.push(item);
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
                if has_space {
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
                if has_space {
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
                if has_space {
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
                if has_space {
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
                if has_space {
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
                let exact = span.end - span.start == text.len() && !reference_spans.contains(span);
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
                let chars = tex_ligatures(chars);
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
                    if ch == ' ' && !exact {
                        flush(&mut run, &mut items, &mut factor);
                        items.push(Item::Space { style, factor, no_break: false });
                        factor = 1000;
                        continue;
                    }
                    // Only a typed `~` is the active tie; `\textasciitilde`
                    // (the compiler's symbol text) is the character itself.
                    if ch == '~' && source.get(src.start..src.end) == Some("~") {
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
}
