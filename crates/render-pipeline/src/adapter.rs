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
use flashtex_compiler::parser::{Block as CBlock, BreakParameter, FillLeader, GlueKind, Inline, InterwordGlue, ItemLabel, ListFrame, ListLength, ListOption, ParameterAssignment, Parsed, TextStyle as CTextStyle, UnderlineGeom};
use flashtex_compiler::text_builtins::{TextDimen, TextLogo, TextRule};
use flashtex_compiler::{DocumentId, Span};

use flashtex_class_geometry::{
    ClassKind, DocumentSetup, GeometryInput, Glue, PageFrame, PageParams, PageStyle, ResolvedDocument,
    Sp,
};

use crate::display::Diagnostic;
use flashtex_compiler::color::DeviceColor;
use flashtex_paragraph_layout as pl;
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
    /// beamer covered text (`\uncover`, `\pause`, `\item<2->` on a slide
    /// before its own; `crate::overlay`): shaped, measured and broken like
    /// visible text -- the space is kept -- but its glyph runs are not
    /// emitted to the display list (`typeset::assemble_block`).
    /// `\setbeamercovered{invisible}`, the default; pdflatex moves the
    /// covered text 2000 bp off the page (`\pgfsys@begininvisible`).
    pub hidden: bool,
    /// beamer `\visible`/`\invisible`-covered text: unlike `hidden`
    /// (`\uncover`), it is never painted -- not even under
    /// `\setbeamercovered{transparent}` (beamer covers those with
    /// `\beamer@reallymakeinvisible` unconditionally;
    /// `beamerbaseoverlay.sty` 587-593). The space is still kept.
    pub unpainted: bool,
    /// A named font family in force locally (`\fontspec{..}`, a
    /// `\newfontfamily` switch, a body `\setmainfont`): an index into
    /// `Stylesheet::fontspec.families`, set by `crate::fontspec::apply`.
    /// `None` leaves the family slot's default (a preamble `\setmainfont`,
    /// the manifest, or the class font) to decide.
    pub named: Option<u16>,
    /// CJK.sty's `CJK` environment in force (the compiler's
    /// `TextStyle::cjk`): the characters inputenc does not declare are set
    /// from the family's subfont metrics with `\CJKglue` between them
    /// (`typeset::Context::cjk_items`, `crate::cjk`). The face, size and
    /// interword glue of the run are untouched: CJK.sty selects its `C70`
    /// font per character inside a group of its own.
    pub cjk: Option<flashtex_compiler::parser::CjkRun>,
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
    /// `hidden`: beamer covered material (`crate::overlay`): the formula
    /// is set and measured but not painted.
    /// `unpainted`: covered by `\visible`/`\invisible`: never painted, not
    /// even under `\setbeamercovered{transparent}`.
    /// `size_cpt`: the text size where the formula starts, in centipoints
    /// (`\small` is 1095 in a 12 pt document), 0 for the block's own size;
    /// the math fonts follow it (`\check@mathfonts`).
    Math { list: MathList, span: Span, hidden: bool, unpainted: bool, size_cpt: u16 },
    /// `\\`; `skip_pt` is the optional `[<dimen>]` (LaTeX `\@xnewline`:
    /// `\vadjust{\vskip <dimen>}` after the line, or `\vskip` after the
    /// paragraph under `\@centercr`).
    LineBreak { skip_pt: f64 },
    /// Horizontal glue of `em` ems of the current font, stretching
    /// `plus_em` and shrinking `minus_em` ems (`\quad` after a section
    /// number: rigid; `\newblock`, `\hskip .11em \@plus.33em \@minus.07em`,
    /// article.cls 584). `style` is the font in force where the glue is
    /// read (its size and series pick the quad, `\fontdimen6`); the
    /// default keeps the block's.
    Quad { em: f64, plus_em: f64, minus_em: f64, style: TextStyle },
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
    /// the source bytes (`\hfill` when they are not `\hfil`). `style` is
    /// the font in force at the fill, which `\dotfill` sets its dots in
    /// (like `Kern`'s); a rule leader paints a fixed 0.4pt rule and plain
    /// `\hfill` paints nothing, so neither reads it.
    HFill { fill: bool, leader: FillLeader, style: TextStyle },
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
    /// amsthm's `\qedsymbol`, i.e. `\openbox`: the proof-end marker
    /// `\end{proof}` appends after an `\hfill`.
    ///
    /// It is not a character. amsthm.sty defines it as four rules in an
    /// `\hbox`,
    ///
    /// ```text
    /// \hbox to.77778em{\hfil\vrule\vbox to.675em{\hrule width.6em\vfil\hrule}\vrule\hfil}
    /// ```
    ///
    /// — an *open* square 0.6 em wide and 0.675 em tall drawn with 0.4 pt
    /// rules. The compiler has no inline for it yet and emits the code point
    /// U+220E (END OF PROOF) instead, which is a *filled* square and which
    /// Latin Modern has no glyph for at all, so the marker came out blank
    /// (`missing_glyph`, GH#443). `typeset::Context::qed_items` sets the real
    /// box; `style` is the font in force at the marker (its quad is the `em`)
    /// and `span` is `\end{proof}`.
    QedBox { style: TextStyle, span: Span },
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
    /// `\marginpar` (compiler `Inline::Marginpar`). `text` is the note's
    /// items, set in `\normalsize` in the outer margin by
    /// `typeset::marginpar`; the running text carries no mark. `span` is
    /// the command token.
    Marginpar { text: Vec<Item>, span: Span },
    /// `\colorbox`/`\fcolorbox` (compiler `Inline::ColorBox`).
    ColorBox(Box<ColorBoxItem>),
    /// `\includegraphics[options]{path}` in running text (compiler
    /// `Inline::Graphic`): one box on the line, sized by `typeset::Context::
    /// graphic_box` from the file and the graphicx keys (`width=.6\textwidth`
    /// against the enclosing box's `\textwidth`). The keys are kept as
    /// written because their lengths resolve where the box is set (a beamer
    /// column's `\textwidth` is the column's).
    /// `hidden`: beamer covered material (`crate::overlay`).
    /// `unpainted`: covered by `\visible`/`\invisible`: never painted, not
    /// even under `\setbeamercovered{transparent}`.
    Graphic { options: String, path: String, span: Span, hidden: bool, unpainted: bool },
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
    /// `\textsuperscript`/`\textsubscript` (compiler `Inline::TextScript`).
    TextScript(Box<TextScriptItem>),
    /// A plain `\hbox` at its natural width (compiler `Inline::HBox`:
    /// `\mbox`, text-mode `\text`, a kernel `\cite` label).
    HBox(Box<HBoxItem>),
    /// A beamer overlay marker (compiler `Inline::OverlayBegin`/
    /// `OverlayEnd`/`Onslide`): no material. `crate::overlay::expand_frames`
    /// reads and removes them when it sets a frame once per slide; the
    /// typesetter ignores any that remain.
    Overlay(OverlayMark),
    /// A paragraph (or other horizontal list) whose assembly already passed
    /// `pl::MAX_ITEMS`: the breaker would reject it with
    /// `LayoutError::TooManyItems`, so assembly stops here instead of doing
    /// any more per-word work. `span` is the list's first inline (the error's
    /// source position); `count` is the assembly count that already exceeded
    /// the limit. `typeset` expands this back into an over-limit
    /// paragraph-layout list, so the failure surfaces through the exact same
    /// `paragraph_layout_error` path as a fully assembled list.
    Overlong { span: Span, count: usize },
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
    /// An explicit break point: `\penalty<value>`, or, `flagged`, an empty
    /// `\discretionary{}{}{}` — charged `\exhyphenpenalty` (50) and
    /// counted as a hyphenated line for `\doublehyphendemerits`. listings'
    /// `breaklines` puts one after every token of a `\lstinline`
    /// (lstmisc.sty `\lst@discretionary`, see `listings::set_inline`).
    Penalty { value: i32, flagged: bool },
    /// `\hbox{\ }`: a blank of the font in force set as a box, so it is
    /// neither stretchable nor discarded at a line break. listings sets
    /// every blank of a `\lstinline` this way (`\lst@outputspace`), which
    /// is why pdflatex's next line can open with one.
    SpaceBox { style: TextStyle },
    /// listings' column bookkeeping around the boxes of a `\lstinline`
    /// (see [`ListingMark`] and `listings::set_inline`): no material of its
    /// own, but the kern boxes `\lst@lostspace` turns into.
    Listing(ListingMark),
}

/// The steps of listings' `\lst@lostspace` bookkeeping inside a
/// `\lstinline` (listings.sty 555-570, 572-586, 830-865), which
/// `typeset::hlist` replays with the glyph widths it has and
/// `listings::set_inline` explains. Under `flexiblecolumns` a token keeps
/// its natural width, but every character is booked at `\lst@width` and
/// the running difference — negative under a typewriter face — comes out
/// as kern boxes wherever it is positive.
#[derive(Debug, Clone, PartialEq)]
pub enum ListingMark {
    /// `\lst@Init`: the lost space is 0 and `\lst@width` is `width_em`
    /// quads (`basewidth`'s flexible value) of the `basicstyle` face,
    /// `style`.
    Begin { style: TextStyle, width_em: f64 },
    /// `\lst@UseLostSpace` before a token's or blank's box: a kern box of
    /// the lost space when it is positive, which is then 0.
    LostSpace,
    /// `\lst@CalcLostSpaceAndOutput` after a box of `columns` characters:
    /// the lost space grows by `columns` times `\lst@width` less the box's
    /// width; positive, it pads the box by that, half on each side
    /// (`[c]` of `columns=[c]fixed`, `\lst@InsertHalfLostSpace`,
    /// `\lst@InsertLostSpace`), and is 0 again.
    Columns { columns: u32 },
    /// A blank gobbled after another or at the start of the argument
    /// (`\lst@AppendSpecialSpace`): no box, one `\lst@width` more of lost
    /// space.
    GobbledBlank,
}

/// beamer overlay markers (see [`Item::Overlay`]).
#[derive(Debug, Clone, PartialEq)]
pub enum OverlayMark {
    /// The opening of an overlay-aware argument or an `\item<spec>`.
    Begin { spec: flashtex_compiler::overlay::OverlaySpec, kind: flashtex_compiler::overlay::OverlayKind },
    /// Its close.
    End,
    /// `\onslide<spec>` without braces, `\pause`.
    Onslide { spec: flashtex_compiler::overlay::OverlaySpec },
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

/// A plain `\hbox{...}` (see [`Item::HBox`]): `items` set as one line at
/// their natural width (`typeset::Context::plain_hbox`).
#[derive(Debug, Clone, PartialEq)]
pub struct HBoxItem {
    pub items: Vec<Item>,
    pub span: Span,
}

/// latex.ltx `\@textsuperscript`/`\@textsubscript`:
/// `{\m@th\ensuremath{^{\mbox{\fontsize\sf@size\z@\selectfont #1}}}}` (or
/// `_{...}`). `items` are set at the `\sf@size` of the text size in effect
/// at the command (`size_cpt`, 0 for the paragraph's), then shifted as a
/// text-style script of an empty nucleus (`typeset::text_script_box`).
#[derive(Debug, Clone, PartialEq)]
pub struct TextScriptItem {
    pub superscript: bool,
    pub size_cpt: u16,
    pub items: Vec<Item>,
    pub span: Span,
}

/// Which amsmath display alignment a [`ParaPart::Rows`] is (read from the
/// environment name at the display's first byte; the compiler keeps only
/// whether cells share tab stops). amsmath's `align`/`gather`/`multline`
/// family plus LaTeX's own `eqnarray`, which the compiler lowers through
/// the same multi-row path.
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
    /// `eqnarray`/`eqnarray*` (latex.ltx, not amsmath): three columns —
    /// right, centred, left — separated by a fixed `\tw@\arraycolsep`, the
    /// block centred by the `\@centering` tabskips at its two ends.
    EqnArray,
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
            "eqnarray" => RowsEnv::EqnArray,
            _ => RowsEnv::Align,
        }
    }
}

/// One row of a [`ParaPart::Rows`] display: its `&`-separated cells, its
/// equation number (or `\tag` text) and the row's source span.
#[derive(Debug, Clone, PartialEq)]
pub struct RowPart {
    pub cells: Vec<MathList>,
    /// A `multline` row-alignment override: the compiler's
    /// [`parser::MathRow::shove`](flashtex_compiler::parser::MathRow::shove)
    /// (`\shoveleft` sets the row flush left, `\shoveright` flush right).
    /// Only the `multline` family sets this; every other display leaves it
    /// `None` and keeps its own placement. `None` here too when amsmath is
    /// not loaded: the command requires it (see the `adapt` gate, matching
    /// pdflatex's `Undefined control sequence`), so the row keeps the
    /// display's default placement.
    pub shove: Option<flashtex_compiler::parser::ShoveDirection>,
    pub number: Option<(String, Span)>,
    /// A rich `\tag` label (see [`TagLabel`]) set in place of `number`'s text.
    pub number_math: Option<MathList>,
    pub span: Span,
    /// amsthm `\qedhere` stripped from this row's cells (`strip_qedhere`):
    /// the box is set on this row's own line, flush right. `None` without
    /// one; the span is the command, for the rules' provenance.
    pub qed_here: Option<Span>,
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
        /// `\multlinegap` (amsmath.sty: a skip, default 10pt) as the
        /// document's last `\setlength` left it, in points: the first and
        /// last rows' indent and the `\shoveleft`/`\shoveright` targets.
        /// Read for `multline` displays only; other environments ignore it.
        multline_gap: f64,
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
        /// A rich `\tag` label (see [`TagLabel`]) set in place of `number`'s
        /// text.
        number_math: Option<MathList>,
        bracket: bool,
        /// amsthm `\qedhere` stripped from `list` (`strip_qedhere`): the box
        /// is set on the display's own line, flush right. `None` without
        /// one; the span is the command, for the rules' provenance.
        qed_here: Option<Span>,
    },
}

/// Which letter.cls block a [`Block::Letter`] is (letter.cls, TeX Live
/// 2026; the compiler's `LetterPart` plus the `\cc`/`\encl` paragraph).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LetterKind {
    /// `\opening`'s `{\raggedleft\begin{tabular}{l@{}}\ignorespaces
    /// \fromaddress \\*[2\parskip] \@date \end{tabular}\par}` (lines
    /// 270-273): one `tabular` -- a box `\vcenter`ed on the math axis
    /// whose rows share one left edge -- pushed to the right margin.
    ReturnAddress,
    /// `\opening`'s `{\raggedright \toname \\ \toaddress \par}` (line
    /// 276): `\raggedright` makes `\\` `\@centercr`, so every line is a
    /// paragraph of its own at natural width, `\parskip` cancelled.
    Recipient,
    /// `\closing`'s `\noindent\hspace*{\longindentation}\parbox
    /// {\indentedwidth}{\raggedright \ignorespaces #1\\[6\medskipamount]
    /// \fromsig\strut}\par` (lines 286-296): a `\parbox` (position `c`:
    /// `$\vcenter{...}$`) on a line of its own, `\longindentation` in.
    Closing,
    /// `\cc`/`\encl` (lines 298-311): `\par\noindent\parbox[t]
    /// {\textwidth}{\@hangfrom{\normalfont\ccname: }\ignorespaces
    /// #1\strut}\par` -- the label box (its trailing space inside it,
    /// `\sfcode` 2000 after the colon) hangs, and `\strut` gives the
    /// line `\strutbox`'s height and depth.
    Annotation,
}

/// A [`Block::Letter`] by reference, as the typesetter reads it.
#[derive(Debug, Clone, Copy)]
pub struct LetterBlockRef<'a> {
    pub kind: LetterKind,
    pub lines: &'a [Vec<Item>],
    pub extra_gap_after_pt: &'a [f64],
    pub gap_before_pt: f64,
    pub gap_after_pt: f64,
    pub indent_pt: f64,
    pub span: Span,
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
        /// `\addpenalty` before the block's skips: a `\list`'s
        /// `\@beginparpenalty` (first `\item`, unless `\@nobreak`),
        /// `\@itempenalty` (every later `\item`) and `\@endparpenalty`
        /// (the block after `\end{<list>}`), all `-\@lowpenalty` = -51 in
        /// the standard classes. `None` for no penalty node.
        penalty_before: Option<i32>,
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
        /// A `\hangfrom{label}` opening the paragraph: the label's own
        /// items, whose shaped natural width hangs every continuation
        /// line (`typeset` reuses the list `hang_pt` mechanism). `None`
        /// for every other paragraph.
        hang: Option<Vec<Item>>,
    },
    Heading {
        level: u8,
        items: Vec<Item>,
        eject_before: bool,
        vspace_before: f64,
        /// `\baselineskip` of the heading's lines and of the glue above its
        /// first one when its title selected a size (`\@sect`'s `#8\@@par`
        /// runs under it; compiler [`ParLeading`]). `None` is the level's own.
        leading_pt: Option<f64>,
        /// `items` open with the `\@svsec` box (the number and its
        /// `\quad`), which `\@hangfrom` hangs every later line by.
        numbered: bool,
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
        /// `\baselineskip` of the title's lines when the title selected a
        /// size (`{\LARGE \@title \par}` reads it); `None` is `\LARGE`'s.
        title_leading_pt: Option<f64>,
        authors: Vec<Vec<Vec<Item>>>,
        date: Option<Vec<Item>>,
        span: Span,
    },
    /// One of letter.cls's positioned blocks (compiler `LetterBlock`, or
    /// the `\cc`/`\encl` paragraph the compiler labels): the typesetter
    /// sets it as the class does -- a `tabular` in `\raggedleft`, a
    /// `\parbox` at `\longindentation`, a `\parbox[t]` with a hanging
    /// label -- see [`LetterKind`] and `typeset::Context::letter_block`.
    Letter {
        kind: LetterKind,
        /// The block's lines, one per `\\` of the class's own text (a
        /// `tabular` row, a `\raggedright` line); for
        /// [`LetterKind::Annotation`] the label (`encl:`) and the text.
        lines: Vec<Vec<Item>>,
        /// The class's own extra leading after line `i` (`\\*[2\parskip]`
        /// between address and date, `\\[6\medskipamount]` between
        /// closing and signature), parallel to `lines`; in points.
        extra_gap_after_pt: Vec<f64>,
        /// The class's own `\vspace` before/after the block, beyond the
        /// `\parskip` every paragraph takes (compiler `LetterBlock`).
        gap_before_pt: f64,
        gap_after_pt: f64,
        /// `\hspace*{\longindentation}` before the closing's `\parbox`.
        indent_pt: f64,
        span: Span,
        eject_before: bool,
        vspace_before: f64,
    },
    /// A class command's `\clearpage` (`double`: `\cleardoublepage`), from
    /// book.cls `\frontmatter`/`\mainmatter`/`\backmatter` (lines 284-298):
    /// the next material starts a new page (an odd one when two-sided).
    ClearPage { double: bool, span: Span },
    /// `\@starttoc`'s `\@nobreakfalse` (latex.ltx), at the end of every
    /// `\tableofcontents`/`\listoffigures`/`\listoftables`: the heading
    /// the list opened with no longer governs what follows. A sectioning
    /// command right after an empty list therefore takes `\@startsection`'s
    /// `\addpenalty`/`\addvspace` branch, not the `\if@nobreak` one that
    /// drops its before-skip under another heading. It sets nothing.
    NoBreakFalse { span: Span },
    /// A page-style or mark command in the body, attached to the material
    /// that follows it.
    Chrome { event: ChromeEvent, span: Span },
    /// One `\tableofcontents`/`\listoffigures`/`\listoftables` entry line
    /// (`crate::toc`).
    TocEntry(Box<crate::toc::TocEntry>),
    /// A `tikzpicture`, found from the source bytes (the compiler reports the
    /// environment as unknown and sets its body as text, which is dropped
    /// here): its bounding box as one box on a line of its own, indented
    /// like the paragraph it starts (flush left inside a list item, centred
    /// inside `center`).
    Picture {
        document: flashtex_compiler::DocumentId,
        picture: flashtex_vector_graphics::tikz::PictureSource,
        centered: bool,
        /// The paragraph's `\parindent` decision for a picture that opens
        /// its paragraph (see [`UnitKind::Picture`]): the box starts
        /// `\parindent` in, exactly where an ordinary paragraph's first
        /// line would. `false` after `\noindent`, a heading, `\end{...}`,
        /// in a caption, a styled environment or a list item.
        indent: bool,
        /// A compiler `ListItem` picture's `\list` geometry: the box starts
        /// at the hanging indent, where the item's text starts.
        list: Option<ListGeom>,
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
    /// beamer `\begin{frame}` (compiler `BeamerFrameBegin`, #944): the slide
    /// head. The blocks up to the matching [`Block::FrameEnd`] are the
    /// frame's body; the typesetter sets the run as one `\vbox
    /// to\textheight` page (`beamerbaseframe.sty`).
    FrameBegin {
        title: Vec<Item>,
        subtitle: Vec<Item>,
        align: flashtex_class_geometry::beamer::FrameAlign,
        /// `[plain]`: no headline/footline on the page and the frame's exit
        /// code `\vspace*{-\footheight}` (`class_geometry::beamer::plain_frame`).
        plain: bool,
        /// `[allowframebreaks]`: the body is `\vsplit` over as many pages as
        /// it needs (`class_geometry::beamer::autobreak`).
        allowframebreaks: bool,
        /// The largest slide the body names (compiler
        /// `BeamerFrameBegin::slides`); after `crate::overlay::expand_frames`
        /// the frame appears once per slide it is set on and `slide` says
        /// which.
        slides: u32,
        slide: u32,
        /// `\begin{frame}<spec>` (compiler `BeamerFrameBegin::spec`): the
        /// slides the frame is set on; `expand_frames` runs beamer's frame
        /// loop over it.
        spec: flashtex_compiler::overlay::OverlaySpec,
        /// This copy is the frame's first slide set: it steps
        /// `framenumber` (`\beamer@@@@frame`'s `\stepcounter`); the
        /// frame's later slides share the number.
        first_slide: bool,
        span: Span,
    },
    /// beamer `\end{frame}`: `addvspace_before` is the `\@endparenv` skip
    /// of a list the frame closes (natural; `addvspace_flex` its stretch
    /// and shrink), `vspace_before` any `\vspace` before it: both are glue
    /// after the body's last block.
    FrameEnd { span: Span, addvspace_before: f64, addvspace_flex: (f64, f64), vspace_before: f64 },
    /// beamer `\titlepage` (compiler `BeamerTitlePage`): the default inner
    /// theme's `title page` template, set by the typesetter with
    /// `flashtex_class_geometry::beamer::title_page`'s skips.
    BeamerTitle {
        title: Vec<Item>,
        subtitle: Vec<Item>,
        authors: Vec<Item>,
        institute: Vec<Item>,
        date: Vec<Item>,
        span: Span,
    },
    /// beamer `\tableofcontents` inside a frame (`beamerbasetoc.sty`
    /// 73-93, 113-150): `\vspace*{-.5em}`, then for every section a
    /// `\vfill` (beamer's, `plus 1fill`) and the `section in toc` line
    /// (the title in the structure colour), then a closing `\vfill` --
    /// the fills sharing the frame's free height with the frame's own
    /// `[c]` skips. `entries` are the deck's `\section`s (compiler
    /// `BeamerSection`) and subsections in document order (the
    /// `subsection in toc` template: `\leftskip 1.5em`, no `\vfill` of
    /// their own); `current` is `(\c@section, \c@subsection)` where the
    /// list stands and `options` its `[..]` key list, which shade or hide
    /// entries relative to it (`typeset::beamer_toc`). Measured (probe
    /// deck `beamer-polish` p1, five sections): entries at baselines
    /// 73.956, 109.042, 144.128, 179.213, 214.299bp (35.086 apart) at x
    /// 28.346, CMSS10.
    BeamerToc { entries: Vec<BeamerTocEntry>, current: (usize, usize), options: String, span: Span },
    /// beamer `\begin{block}{title}` and friends (compiler
    /// `BeamerBlockBegin`, #944 Tier 3): the blocks up to the matching
    /// [`Block::BeamerBlockEnd`] are the body. `addvspace_before` /
    /// `vspace_before` are the glue a list closed before it (or a
    /// `\vspace`) leaves, like [`Block::FrameEnd`]'s.
    BeamerBlockBegin {
        kind: flashtex_compiler::parser::BeamerBlockKind,
        title: Vec<Item>,
        span: Span,
        addvspace_before: f64,
        addvspace_flex: (f64, f64),
        vspace_before: f64,
    },
    BeamerBlockEnd { span: Span, addvspace_before: f64, addvspace_flex: (f64, f64), vspace_before: f64 },
    /// beamer `\begin{columns}[options]`: the blocks up to the matching
    /// [`Block::ColumnsEnd`], split at each [`Block::Column`] marker, are
    /// the columns' bodies.
    ColumnsBegin {
        options: flashtex_compiler::parser::BeamerColumnsOptions,
        span: Span,
        addvspace_before: f64,
        addvspace_flex: (f64, f64),
        vspace_before: f64,
    },
    /// `\column[align]{width}` / `\begin{column}`: `width` as written.
    Column { width: String, align: Option<flashtex_compiler::parser::BeamerColumnAlign>, span: Span },
    ColumnsEnd { span: Span, addvspace_before: f64, addvspace_flex: (f64, f64), vspace_before: f64 },
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
    /// An explicit `\item[<label>]`'s content as items (its math, styles and spaces); `None` for a counter, symbol or template label.
    pub label_items: Option<Vec<Item>>,
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
    /// The innermost list's effective enumitem `style` is `nextline`
    /// (`enumitem.sty` `\enit@style@nextline` sets `\enit@nextline` and
    /// `\labelwidth` from the narrowest fit, so `\enit@postlabel@i` breaks
    /// after every label in practice): the typesetter forces a break after
    /// the label, like `\\`, so the body starts on its own line at the
    /// hanging indent. `sameline`/`standard`/`normal` are the default and
    /// need nothing; `multiline`/`unboxed` are a follow-up (label alignment
    /// and `\itemindent` nuances, not a plain break).
    pub nextline: bool,
    /// Whether the compiler produced a default itemize symbol.
    pub label_symbol: bool,
    /// Whether the label's declaration applies bold text.
    pub label_bold: bool,
    /// Whether the list uses the kernel's left-extending label box.
    pub llap: bool,
    /// The innermost itemize/enumerate's enumitem `labelsep=`, in points;
    /// `None` keeps the class's `\labelsep`. `\@item` ends the label box
    /// `\labelsep` before the first line's text.
    pub labelsep_pt: Option<f64>,
    /// beamer: the item is covered on this slide (`\item<2->`, a `\pause`
    /// before it), so its label is not painted either (`TextStyle::hidden`).
    pub hidden: bool,
    /// beamer: the item is covered by `\visible`/`\invisible` on this
    /// slide, so its label is never painted, not even under
    /// `\setbeamercovered{transparent}`.
    pub unpainted: bool,
    /// beamer: the item is alerted on this slide (`\item<1-| alert@2>`),
    /// so its label takes the alert colour with the text.
    pub alerted: bool,
    /// The innermost itemize/enumerate's enumitem `itemindent=`, in points
    /// (added to [`Self::itemindent_em`]): the item's first line, and its
    /// label, start this much further in.
    pub itemindent_pt: f64,
    /// The list is `thebibliography` (article.cls, natbib alike), whose
    /// `\list` is followed by `\sloppy` and `\sfcode`\.\@m`: the entries
    /// are broken at `\tolerance 9999` with `\emergencystretch 3em`
    /// (`Context::paragraph_block`), and a `.` leaves the space factor at
    /// 1000, so `Knuth. The` gets an ordinary interword space
    /// ([`bibliography_space_factors`]).
    pub bibliography: bool,
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
    /// enumitem `leftmargin=*` together with a `labelsep=` or `itemindent=`
    /// key on the same level: `\enit@calcleft` gives `\leftmargin =
    /// \labelwidth + \labelsep - \itemindent` (`\labelindent` 0), with
    /// `\labelwidth` the width of `label` and `\labelsep` the class's when
    /// `labelsep_pt` is `None`. Points.
    WidestSep { label: String, labelsep_pt: Option<f64>, itemindent_pt: f64 },
    /// A `leftmargin=\len` whose register was set by `\settowidth{\len}
    /// {<text>}`: the width of `<text>` in the body font.
    TextWidth(String),
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
    /// Every line is at least `\strutbox`-tall (`.7`/`.3\baselineskip` of
    /// the size): listings' code lines (measured `\hbox(8.39996+3.60004)`
    /// for `\small` code whose glyphs reach 7.5/2.5pt). Only the last
    /// line's depth shows in an ordinary flow (interline glue absorbs the
    /// rest), but inside beamer's `\vbox to\textheight` it is part of the
    /// frame's natural height.
    pub strut: bool,
}

/// The size declaration in force when a paragraph's `\par` ran, which is the
/// `\baselineskip` TeX reads in `append_to_vlist` (§679) for every one of its
/// lines — the compiler's `parser::ParLeading`, mirrored here so the pipeline
/// builds against a `vendor/compiler` that predates the name.
pub type ParLeading = Option<flashtex_compiler::parser::FontSizeLevel>;

/// How each compiler paragraph starts (`Parsed::block_par_starts`, PLAN1
/// slice 2): `\noindent`, `\@endpe`, and whether a `\par` came before it,
/// keyed by the spans of the block's first few inlines (a unit is found by
/// its own first inline, which a lowering pass may have dropped). Empty
/// when the compiler's list is not one entry per block ([`block_leadings`]).
#[derive(Debug, Default)]
struct ParStarts(std::collections::HashMap<(usize, usize, usize), flashtex_compiler::parser::ParStart>);

impl ParStarts {
    fn new(parsed: &Parsed) -> ParStarts {
        let mut map = std::collections::HashMap::new();
        if parsed.block_par_starts.len() == parsed.blocks.len() {
            for (block, start) in parsed.blocks.iter().zip(&parsed.block_par_starts) {
                let mut key = |s: Span| {
                    map.entry((s.document.0, s.start, s.end)).or_insert(*start);
                };
                match block {
                    // The first few inlines: a lowering pass may drop the
                    // block's first (a `\markboth` argument's run).
                    CBlock::Paragraph(content) | CBlock::Styled { content, .. } | CBlock::ListItem { content, .. } => {
                        content.iter().take(4).for_each(|i| key(inline_span(i)))
                    }
                    // Lowered to a `Styled` paragraph whose first run is the
                    // first line ([`lower_blocks`]).
                    CBlock::Verbatim { lines, .. } => lines.iter().take(1).for_each(|l| key(l.span)),
                    CBlock::Alltt { lines, .. } => lines.iter().flatten().take(4).for_each(|i| key(inline_span(i))),
                    _ => {}
                }
            }
        }
        ParStarts(map)
    }

    /// The start of the paragraph whose first inline is `inlines[0]`.
    fn of(&self, inlines: &[Inline]) -> Option<flashtex_compiler::parser::ParStart> {
        let s = inline_span(inlines.first()?);
        self.0.get(&(s.document.0, s.start, s.end)).copied()
    }
}

/// One [`ParLeading`] per block, from the compiler's `block_par_leading`.
///
/// The compiler's contract is one entry per pushed block, in `blocks` order,
/// and while it holds the two lists are paired positionally. A pinned
/// `vendor/compiler` can break it: before the compiler's `P::box_inlines`
/// learned to truncate `block_par_leading` the way `P::argument_inlines`
/// always did (crates/compiler, #517), every `\colorbox`/`\fcolorbox` box
/// argument left one stray entry behind — the leading of a paragraph that
/// never reached `blocks`.
///
/// A stray entry cannot be located after the fact, and it is pushed *before*
/// the block whose paragraph contains the box, so from the first box onwards
/// entry *i* no longer names block *i*. Pairing them anyway hands a paragraph
/// the `\baselineskip` of some box's interior: `\colorbox{white}{\small x}`
/// in a body paragraph shrinks the whole paragraph's line pitch.
///
/// So a length disagreement discards the list: every block falls back to the
/// body leading, exactly as a `--no-default-features` build does, and the
/// feature resumes by itself once `vendor/` is re-pinned past #517.
///
/// This decision must not depend on the build profile. It used to be a
/// `debug_assert_eq!`, which made a debug build panic inside `\fcolorbox`
/// rendering while a release build silently mis-paired — the same input
/// producing two different outcomes depending on the optimisation level
/// (#667).
#[cfg(feature = "par-leading")]
fn block_leadings(from_compiler: &[ParLeading], blocks: usize) -> Vec<ParLeading> {
    if from_compiler.len() == blocks {
        from_compiler.to_vec()
    } else {
        vec![None; blocks]
    }
}

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
    /// Inclusive block index ranges of every `titlepage` `abstract`
    /// (`abstractenv::page_ranges`): a page of its own, `\vfil`-centred,
    /// with a page break on each side. Empty for every other document.
    pub abstract_pages: Vec<(usize, usize)>,
    /// `\begin` commands the compiler reported as unimplemented that the
    /// pipeline sets itself (`abstract`): its diagnostic is dropped, the
    /// way `toc::superseded_commands` drops the contents-list ones.
    pub superseded: Vec<Span>,
    /// `\twocolumn[<material>]`'s optional argument and the span of the
    /// whole `[..]`: the blocks `\@topnewpage` sets in a `\textwidth` box
    /// above both columns of the page the command starts. They are not in
    /// `blocks`; `typeset::build_with_floats` sets them itself. `Some` with
    /// an empty vector is `\twocolumn[]`, which is a box of no height.
    pub top_material: Option<(Vec<Block>, Span)>,
    /// A single bare `\twocolumn`/`\onecolumn` after the first material
    /// that the page builder lays out: every page from
    /// [`crate::columns::ColumnSwitch::block`] on uses [`Doc::post_style`]'s
    /// frame. `None` is today plus a `twocolumn_mid_document` limitation
    /// for every unmodelled switch.
    pub column_switch: Option<crate::columns::ColumnSwitch>,
    /// The stylesheet past the recorded [`Doc::column_switch`]: `style`
    /// cloned with the post-switch frame (the command changes only the
    /// column split, never `\parindent`/`\textwidth`/margins) and its
    /// column width. `None` when there is no recorded switch.
    pub post_style: Option<Box<Stylesheet>>,
    /// beamer (compiler `Parsed::beamer`): the theme's footline fields.
    pub beamer: Option<BeamerDeck>,
}

/// One line of a beamer contents list: a `\section` (`level` 1) or
/// `\subsection` (2) with the `\c@section`/`\c@subsection` values it was
/// written at (`\beamer@sectionintoc{\the\c@section}..`).
#[derive(Debug, Clone, PartialEq)]
pub struct BeamerTocEntry {
    pub level: u8,
    pub section: usize,
    pub subsection: usize,
    pub items: Vec<Item>,
}

/// The short title-block forms a beamer theme's footline sets (compiler
/// `parser::BeamerDeck`, as items); the theme itself is read from the
/// preamble by `flashtex_class_geometry` (`Stylesheet::class_geometry`).
#[derive(Debug, Clone, Default)]
pub struct BeamerDeck {
    pub short_title: Vec<Item>,
    pub short_author: Vec<Item>,
    pub short_institute: Vec<Item>,
    pub short_date: Vec<Item>,
    /// `\logo{..}` (empty when the deck sets none).
    pub logo: Vec<Item>,
}

/// The content pieces of a rich `\tag` (see `tag_content_pieces`), as a
/// label table value. The compiler's `TextPiece` is `PartialEq` only (a
/// `Space` atom's `em` is an `f64`); the pieces come from the parse, which
/// never makes a NaN, so equality is an equivalence here.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TagContent(pub Vec<flashtex_compiler::math::TextPiece>);

impl Eq for TagContent {}

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
    /// The body in reading order ([`reading_order`]), which the lists
    /// merge their float and `\addcontentsline` entries in.
    pub reading_order: Vec<Span>,
    /// Entry titles taken from source bytes, set as body text.
    pub entry_items: crate::toc::EntryItems,
    /// cleveref's label type per key (`section`, `equation`, `figure`, ...),
    /// from the compiler's `Inline::Label::kind`. Only `\cref` and friends
    /// read it; `\ref` needs the value alone.
    pub kinds: BTreeMap<String, String>,
    /// The content of a rich `\tag` (`\tag{hi $x^2$}`, #441) per label in
    /// its display ([`Labels::collect_rich_tags`]): what amsmath's `\eqref`
    /// sets again, as `\textup{\tagform@{..}}`, where `values` has only the
    /// compiler's flattened text of it.
    pub rich_tags: BTreeMap<String, TagContent>,
    /// The document's cleveref naming options and `\crefname` overrides
    /// (`Parsed::cleveref`), so `\cref` can name the type it refers to.
    pub cleveref: flashtex_compiler::xref::CleverefConfig,
    /// Macro invocations whose definition is in another document -- a
    /// project `.sty`/`.cls` the compiler read (`Parsed::expansions`): the
    /// invocation's `(document, start)` to the definition's `(document,
    /// end)`. `token_gap` reads the replacement text's own blanks from the
    /// defining document; every in-document definition is found by the
    /// source scan and needs no entry.
    pub foreign_definitions: BTreeMap<(usize, usize), (usize, usize)>,
}

/// The `\cc`/`\encl` paragraph as the compiler emits it (`letter_annotation`
/// in its parser): a first `Inline::Text` holding exactly `cc:` or `encl:`
/// whose span is the whole command, followed by the argument's inlines,
/// every one of them inside that span. Returns the label, the text and the
/// command's span. The label's trailing space is not in the inlines --
/// the compiler marks the text's first inline `space_before` instead, and
/// the pipeline's gap scan sees no bytes between a span and one it
/// contains -- which is why the typesetter sets the label as the class's
/// `\hbox{{\normalfont\enclname: }}`, space included.
fn letter_annotation(inlines: &[Inline]) -> Option<(&[Inline], &[Inline], Span)> {
    let Some(Inline::Text { text, span, space_before: false, .. }) = inlines.first() else { return None };
    if text != "cc:" && text != "encl:" {
        return None;
    }
    let rest = &inlines[1..];
    let inside = |i: &Inline| {
        let s = inline_span(i);
        s.document == span.document && s.start >= span.start && s.end <= span.end
    };
    if rest.is_empty() || !rest.iter().all(inside) {
        return None;
    }
    Some((&inlines[..1], rest, *span))
}

/// `\hangfrom{label}` (ltsect.dtx) opening a paragraph: the command sets
/// `\hangindent` to the label's own width, so the paragraph's first line
/// starts at the margin with the label inline while every continuation
/// line hangs the label's width in.
///
/// The compiler (any pin) reports such a paragraph as an ordinary
/// `Block::Paragraph` whose first inlines are the label, so the hang is
/// recovered from the source here: the paragraph's first inline must sit
/// inside the braced group of a `\hangfrom` found by scanning back to
/// `lo`. Callers pass the paragraph's first segment only, so a
/// mid-paragraph `\hangfrom` (TeX: the last assignment wins) keeps the
/// old shape and its diagnostic, as does a macro-supplied label
/// (`\hangfrom\foo`), which has no group to measure.
///
/// Returns the label's leading inline run and the command's span (whose
/// diagnostic [`Doc::superseded`] drops once the hang is set).
///
/// `lo` bounds the backward scan -- the previous block's end in the same
/// document -- so detection stays linear in the document (see
/// `adapt_linear_time`).
fn hangfrom_label<'p>(
    texts: &[&str],
    inlines: &'p [Inline],
    lo: usize,
) -> Option<(&'p [Inline], Span)> {
    // The label is the paragraph's first material: skip the zero-width
    // whatsits the compiler attaches ahead of it (a preamble
    // `\pagestyle` rides the first paragraph, a `\label` its own) and
    // any blank run (source indentation), which carry earlier spans.
    let head = inlines
        .iter()
        .position(|i| match i {
            Inline::PageStyle { .. } | Inline::Mark { .. } | Inline::Label { .. } => false,
            Inline::Text { text, .. } => !text.trim().is_empty(),
            _ => true,
        })?;
    let anchor = inline_span(&inlines[head]);
    let source = texts.get(anchor.document.0)?;
    let prefix = source.get(lo.min(anchor.start)..anchor.start)?;
    let cmd = rfind_command(prefix, "hangfrom").map(|at| lo + at)?;
    // TeX skips spaces (and `%`-to-newline comments) after a control word.
    let bytes = source.as_bytes();
    let mut open = cmd + "\\hangfrom".len();
    loop {
        match bytes.get(open) {
            Some(b' ' | b'\t' | b'\r' | b'\n') => open += 1,
            Some(b'%') => {
                open = source[open..].find('\n').map_or(source.len(), |at| open + at + 1);
            }
            _ => break,
        }
    }
    if bytes.get(open) != Some(&b'{') {
        return None;
    }
    let close = matching_brace(bytes, open)?;
    if !(anchor.start > open && anchor.start < close) {
        return None;
    }
    let doc = anchor.document;
    let mut n = head;
    while n < inlines.len() {
        let s = inline_span(&inlines[n]);
        if s.document != doc || s.start < open || s.start >= close {
            break;
        }
        n += 1;
    }
    // The parser re-emits a trailing space swallowed inside the group as a
    // blank run carrying the command's own span (`\hangfrom{1. }`: that
    // space is the label/body gap and part of `\hangindent`).
    while n < inlines.len() {
        let s = inline_span(&inlines[n]);
        if s.document != doc || s.start >= close {
            break;
        }
        if !matches!(&inlines[n], Inline::Text { text, .. } if text.trim().is_empty()) {
            break;
        }
        n += 1;
    }
    if n == head {
        return None;
    }
    Some((&inlines[head..n], Span::in_document(doc, cmd, cmd)))
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
        // Lowered to a flush-left paragraph by `lower_blocks`, like `Verbatim`.
        CBlock::Alltt { .. } => &[],
        // beamer's frame edges and title page are units of their own
        // (`split_at_page_breaks`); the title is what anchors the head.
        CBlock::BeamerFrameBegin { title, .. } | CBlock::BeamerTitlePage { title, .. } => title,
        CBlock::BeamerFrameEnd { .. } => &[],
        // A beamer section has no material of its own; its title anchors
        // nothing (the block is read for `Block::BeamerToc` only).
        CBlock::BeamerSection { .. } => &[],
        // Tier 3: the block title and the caption text anchor their units;
        // the column markers are units of their own bytes.
        CBlock::BeamerBlockBegin { title, .. } => title,
        CBlock::BeamerCaption { content, .. } => content,
        CBlock::BeamerBlockEnd { .. } | CBlock::BeamerColumnsBegin { .. } | CBlock::BeamerColumn { .. } | CBlock::BeamerColumnsEnd { .. } => &[],
        // Nodes a re-pinned compiler can produce that this crate has no
        // layout for yet. `Penalty` carries no content at all; `Tabbing`'s
        // rows are reached through `lower_blocks`, not this slice, exactly
        // as `LetterBlock`'s lines are. PR #569 (penalties) and the pipeline
        // half of GH-TABBING (compiler #551) replace these with real arms.
        #[cfg(feature = "compiler-node-surface")]
        CBlock::Penalty { .. } | CBlock::Tabbing { .. } => &[],
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
fn lower_blocks(texts: &[&str], blocks: &[(CBlock, ParLeading)], stash_titles: bool) -> (Vec<(CBlock, ParLeading)>, Vec<(&'static str, Span, String)>, Vec<StashedTitle>, Vec<Span>) {
    use flashtex_compiler::parser::{FontSizeLevel, ParagraphStyle, TextFamily, TextStyle as CStyle};
    let mut out: Vec<(CBlock, ParLeading)> = Vec::with_capacity(blocks.len());
    let mut limitations: Vec<(&'static str, Span, String)> = Vec::new();
    let mut titles: Vec<StashedTitle> = Vec::new();
    // Formerly the source spans of the `\opening`/`\closing` paragraphs
    // this pass lowered; `LetterBlock`s now pass through whole (see the
    // arm below), so nothing is recorded here.
    let letter_spans: Vec<Span> = Vec::new();
    let mut pending_vfill = 0usize;
    let sized = |inlines: &[Inline], size: FontSizeLevel| -> Vec<Inline> {
        inlines
            .iter()
            .map(|i| match i {
                Inline::Text { text, span, style, space_before, glue_before, boundary_before } => Inline::Text {
                    boundary_before: *boundary_before,
                    text: text.clone(),
                    span: *span,
                    style: CStyle {
                        size: Some(style.size.unwrap_or(size)),
                        ..*style
                    },
                    space_before: *space_before,
                    glue_before: glue_before.map(|g| flashtex_compiler::parser::InterwordGlue {
                        style: CStyle { size: Some(g.style.size.unwrap_or(size)), ..g.style },
                        ..g
                    }),
                },
                other => other.clone(),
            })
            .collect()
    };
    for (block, par_leading) in blocks {
        let par_leading = *par_leading;
        let first = match block {
            CBlock::Heading { number_span, .. } => Some(*number_span),
            CBlock::Verbatim { span, .. } | CBlock::Alltt { span, .. } | CBlock::TableOfContents { span, .. } | CBlock::Rule { span } => Some(*span),
            _ => anchor_span(inlines_of(block)),
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
            // `tabbing` (GH-TABBING, compiler #551), lowered here the way
            // `LetterBlock` is: every row becomes one flush-left paragraph
            // broken exactly where the source's `\\` put it, so no line of a
            // `tabbing` body is dropped by the re-pin. What is *not* applied
            // is the horizontal part -- `\=` stops, `\>` jumps and `\kill`
            // rows -- which the pipeline half of GH-TABBING adds; the
            // limitation says so at the block's own span.
            #[cfg(feature = "compiler-node-surface")]
            CBlock::Tabbing { lines, span } => {
                if lines.iter().any(|l| !l.content.is_empty()) {
                    limitations.push((
                        "unsupported_block",
                        *span,
                        "tabbing rows set as plain flush-left lines: \\= tab stops and \\> jumps are not applied".to_string(),
                    ));
                }
                let mut content: Vec<Inline> = Vec::new();
                for line in lines.iter().filter(|l| !l.killed) {
                    let Some(at) = line.content.iter().map(inline_span).next() else { continue };
                    if !content.is_empty() {
                        content.push(line_break_inline(Span { document: at.document, start: at.start, end: at.start }));
                    }
                    content.extend(line.content.iter().cloned());
                }
                if !content.is_empty() {
                    out.push((
                        CBlock::Styled {
                            style: ParagraphStyle::FlushLeft,
                            content,
                            lists: Vec::new(),
                            line_break_before: None,
                        },
                        par_leading,
                    ));
                }
            }
            // `alltt` (compiler `Block::Alltt`): typewriter lines whose
            // commands stayed active, one flush-left paragraph with a
            // forced break between lines, as `Verbatim` below. The list
            // level and margin the compiler recorded are not applied yet.
            CBlock::Alltt { lines, .. } => {
                let mut content: Vec<Inline> = Vec::new();
                let mut prev_end: Option<Span> = None;
                for line in lines {
                    let first = line.iter().find(|i| !is_marker(i)).map(inline_span);
                    if let (Some(prev), Some(first)) = (prev_end, first) {
                        content.push(line_break_inline(Span {
                            document: first.document,
                            start: prev.end.min(first.start),
                            end: first.start,
                        }));
                    }
                    if let Some(last) = line.iter().filter(|i| !is_marker(i)).map(inline_span).last() {
                        prev_end = Some(last);
                    }
                    content.extend(line.iter().cloned());
                }
                out.push((
                    CBlock::Styled {
                        style: ParagraphStyle::FlushLeft,
                        content,
                        lists: Vec::new(),
                        line_break_before: None,
                    },
                    None,
                ));
            }
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
                        // `verbatim`/`lstlisting` lines: `\verbatim@font`
                        // (`\normalfont\ttfamily`) and `\@noligs`.
                        style: CStyle {
                            family: TextFamily::Mono,
                            font: crate::nfss::Selected { key: crate::nfss::FontKey::new(crate::nfss::FamilyKind::Tt, crate::nfss::Series::M, crate::nfss::Shape::N), undefined: None },
                            literal: true,
                            ..CStyle::default()
                        },
                        space_before: true,
                        boundary_before: false,
                        glue_before: None,
                    });
                }
                // The text itself is now typewriter and literal (see
                // `TextStyle::literal`), so the old
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
            CBlock::TitleBlock { title, authors, date } if stash_titles => titles.push((title.clone(), authors.clone(), date.clone(), par_leading)),
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
                // `\@maketitle`'s author box is one `tabular` column per
                // `\and` group. This fallback has no columns, so the groups
                // are stacked one line each -- which is exactly what the
                // compiler's flat author run, joined by a `LineBreak`,
                // already produced here before the groups became
                // `Vec<Vec<Inline>>` (PLAN1 site 38). The break's span is
                // the end of the group it follows, so the `\\[<dimen>]`
                // fallback scan finds no bracket, as it found none after
                // the `\author{..}` span this used to carry.
                let mut author_run: Vec<Inline> = Vec::new();
                for group in authors {
                    if let (false, Some(last)) = (author_run.is_empty(), author_run.last()) {
                        author_run.push(line_break_inline(inline_span(last)));
                    }
                    author_run.extend(group.iter().cloned());
                }
                for (part, size, leading) in [
                    (Some(title), FontSizeLevel::Large3, par_leading.or(Some(FontSizeLevel::Large3))),
                    (Some(&author_run), FontSizeLevel::Large1, Some(FontSizeLevel::Large1)),
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
            // letter.cls's positioned blocks (`\opening`'s return address
            // and recipient, `\closing`): the typesetter sets them as the
            // class does (`Block::Letter`, `typeset::letter_blocks`), so
            // they pass through the way `Paragraph`s do.
            CBlock::LetterBlock { .. } => out.push((block.clone(), par_leading)),
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
type StashedTitle = (Vec<Inline>, Vec<Vec<Inline>>, Option<Vec<Inline>>, ParLeading);

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
        let foreign_definitions = parsed
            .expansions
            .iter()
            .filter(|site| site.definition.document != site.invocation.document)
            .map(|site| ((site.invocation.document.0, site.invocation.start), (site.definition.document.0, site.definition.end)))
            .collect();
        Labels {
            values,
            kinds,
            cleveref: parsed.cleveref.clone(),
            foreign_definitions,
            ..Labels::default()
        }
    }

    /// Records the rich `\tag` content behind every equation `\label` set
    /// in a tagged display (`rich_tags`). The compiler lifts `\label`s out
    /// of a display's tokens and pushes them right after its
    /// `Inline::Math`/`MathRows`, each spanning the `\label` where it was
    /// written, so a label belongs to the display just before it, and to
    /// the last of its rows that starts before the label (a row's span is
    /// its cells', which the lifted `\label` may follow).
    pub fn collect_rich_tags(&mut self, texts: &[&str], parsed: &Parsed) {
        #[cfg(feature = "compiler-node-surface")]
        for inlines in parsed.blocks.iter().map(inlines_of) {
            // (span, rich tag content) of the display last seen, or of each
            // of its rows.
            let mut tags: Vec<(Span, Option<Vec<flashtex_compiler::math::TextPiece>>)> = Vec::new();
            for inline in inlines {
                match inline {
                    Inline::Math { list, display: true, span, .. } => {
                        tags.clear();
                        tags.push((*span, rich_tag_of(texts, list)));
                    }
                    Inline::MathRows { rows, .. } => {
                        tags.clear();
                        for row in rows {
                            tags.push((row.span, row.cells.iter().find_map(|c| rich_tag_of(texts, c))));
                        }
                    }
                    Inline::Label { key, kind, span, .. } if kind == "equation" => {
                        let before = |s: &Span| s.document == span.document && s.start <= span.start;
                        if let Some((_, Some(pieces))) = tags.iter().rev().find(|(s, _)| before(s)) {
                            self.rich_tags.insert(key.clone(), TagContent(pieces.clone()));
                        }
                    }
                    _ => {}
                }
            }
        }
        #[cfg(not(feature = "compiler-node-surface"))]
        let _ = (texts, parsed);
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
    // A `\documentclass` naming a project `.cls` file: the compiler read
    // the file and reports the standard class it `\LoadClass`es (or
    // `article`) with the options passed on, which stand in for the class
    // line here (`flashtex_class_geometry::DocumentSetup::from_preamble_with_class`).
    let project_class = parsed
        .class_file
        .and_then(|_| parsed.document_class.as_deref())
        .and_then(flashtex_class_geometry::ClassKind::parse)
        .map(|kind| (kind, parsed.class_options.clone().unwrap_or_default()));
    let explicit_class = match &project_class {
        Some((_, class_options)) => Some(class_options.clone()),
        None => class_options(source),
    };
    let class_options = explicit_class.clone().unwrap_or_else(|| options.default_class_options.clone());
    let size = class_size(&class_options);
    // LaTeX's own \parindent (size1x.clo) applies when the document declares a
    // class; body-only input keeps the compiler's implicit 0pt.
    let family = Stylesheet::family_for(&parsed.packages, t1_encoding(source));
    let setup = match &project_class {
        Some((kind, class_options)) => {
            let mut setup = flashtex_class_geometry::DocumentSetup::from_preamble_with_class(source, *kind, class_options);
            // The class file's own `\usepackage{geometry}`/`\pagestyle` come
            // before the preamble's; a preamble setting then overrides.
            if let Some(class_text) = parsed.class_file.and_then(|id| texts.get(id.0).copied()) {
                let from_class = flashtex_class_geometry::DocumentSetup::from_preamble_with_class(class_text, *kind, class_options);
                if setup.geometry.is_none() {
                    setup.geometry = from_class.geometry;
                }
                if setup.pagestyle.is_none() {
                    setup.pagestyle = from_class.pagestyle;
                }
            }
            setup
        }
        None => document_setup(source, explicit_class.is_some(), &class_options),
    };
    let mut resolved = flashtex_class_geometry::resolve(&setup);
    // The class size as the class resolved it, not as the option list spells
    // it: beamer (and the KOMA classes) default to 11pt with no `11pt` option
    // given, and `class_size`'s 10pt fallback put beamer's 2em list margins
    // at 20pt instead of 21.9pt.
    let size = match resolved.options.size {
        flashtex_class_geometry::BaseSize::Pt10 => 10,
        flashtex_class_geometry::BaseSize::Pt11 => 11,
        flashtex_class_geometry::BaseSize::Pt12 => 12,
    };
    // `\twocolumn`/`\onecolumn` are commands, not class options: two-column
    // mode is state the document sets, and the class option is only its
    // starting value ([`crate::columns`]). The starting value itself is
    // `resolved.flags.twocolumn`, not `resolved.options.twocolumn`: the
    // latter is `\documentclass`'s own option only, while `flags` is what
    // `resolve` already folded the `geometry` package's own `twocolumn` key
    // into (`apply_geometry`, [`flashtex_class_geometry::resolve`]). Seeding
    // from `options` instead left `\documentclass{article}
    // \usepackage[twocolumn]{geometry}` starting one-column, since
    // `options.twocolumn` never saw geometry's override.
    // `set_twocolumn` runs before `apply_preamble_lengths`, which rebuilds
    // the frame from `doc.flags`.
    let columns = crate::columns::ColumnMode::from_switches(texts, &parsed.column_switches, entry, resolved.flags.twocolumn);
    resolved.set_twocolumn(columns.start());
    // A project class file's `\setlength`s ran before the preamble's, as
    // the class is read first: the compiler lists them in that order.
    let assigned = apply_preamble_lengths(&parsed.length_assignments, source, entry, &mut resolved, size, family, setup.geometry.is_some());
    let mut style = Stylesheet::from_resolved(&resolved, family);
    style.columns = columns;
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
    // `\sloppy`: `\tolerance 9999 \emergencystretch 3em \hfuzz .5pt
    // \vfuzz\hfuzz` (latex.ltx), as the class does for two columns; the
    // `em` is the compiler's, read in the font in force at the command.
    // `\fussy` and a bare `\tolerance=<n>` come through the same list, so a
    // document that turns two-column sloppiness back off is followed now.
    let (tolerance, emergency_stretch_pt) = document_break_parameters(&parsed.parameters);
    if let Some(value) = tolerance {
        style.tolerance = value;
    }
    if let Some(pt) = emergency_stretch_pt {
        style.emergency_stretch_pt = pt;
    }
    // beamer loads `amsmath` and `amsthm` itself (`beamerbasetheorems.sty`
    // 15-18, unless the `noamsthm` class option) and `amssymb`
    // (`beamerbasefont.sty` 20-21, unless `noamssymb`): the packages'
    // font declarations are in force whether or not the document names
    // them. Measured (probe deck `beamer-polish` p3): the display `\int`
    // is `CMEX10` at 10.91bp, amsfonts' scaled `cmex10 at 10.95pt`, not the
    // kernel's `sfixed*cmex10`.
    let mut packages = parsed.packages.clone();
    if style.is_beamer() {
        let opt = |name: &str| class_options.split(',').any(|o| o.trim() == name);
        if !opt("noamsthm") {
            style.class_loads_amsmath = true;
            for p in ["amsmath", "amsthm"] {
                if !packages.iter().any(|q| q == p) {
                    packages.push(p.to_string());
                }
            }
        }
        if !opt("noamssymb") {
            style.class_loads_amssymb = true;
            if !packages.iter().any(|q| q == "amssymb") {
                packages.push("amssymb".to_string());
            }
        }
    }
    // amsmath makes `\[` a plain `$$` (see [`ParaPart::Display::bracket`]).
    let amsmath = packages.iter().any(|p| p == "amsmath");
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
    style.cmex_designs = crate::style::cmex_designs(&packages, amsmath_cmex10);
    style.math_roman_lm = crate::style::math_roman_lm(&parsed.packages);
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
    // The value is the compiler's (`Parsed::secnumdepth`): the last
    // `\setcounter`/`\addtocounter` the document ran, not one inside a
    // definition that never runs (PLAN1 site 34).
    let secnumdepth = parsed.secnumdepth.map(|n| n.clamp(0, i64::from(u8::MAX)) as u8).unwrap_or_else(|| {
        match (&style.class_geometry, explicit_class.is_some()) {
            (Some(d), true) => d.secnumdepth.clamp(0, i32::from(u8::MAX)) as u8,
            _ => options.default_secnumdepth,
        }
    });
    style.nfss = crate::nfss::Scheme::for_document(&parsed.packages, t1_encoding(source));
    style.input = crate::inputenc::InputSetup::for_project(texts, entry);
    // Fonts come from the compiler's nodes (`parser::TextStyle::font`);
    // what is still read per document is the size environments.
    let styles: Vec<Styles> = texts.iter().map(|t| Styles::new(t)).collect();
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
        // A rich tag's structure, not only its flattened text in `values`:
        // `\tag{$x$}` and `\tag{\textit{x}}` read alike there and set
        // differently. Through `Debug`, as the non-`Hash` compiler nodes
        // elsewhere in this crate.
        for (k, v) in &labels.rich_tags {
            k.hash(&mut h);
            format!("{:?}", v.0).hash(&mut h);
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
    // `\pagestyle`/`\thispagestyle` come from the compiler's own markers
    // and are merged into the byte-scanned list in document order (PLAN1
    // site 32).
    let body_start = |text: &str| text.find("\\begin{document}").map_or(0, |b| b + "\\begin{document}".len());
    let commands = {
        let mut commands = body_commands(source, has_chapters, book);
        commands.extend(page_style_commands(&parsed.blocks, entry_doc, body_start(source)));
        // The contents lists likewise (PLAN1 site 39): entry-document only,
        // exactly as the byte scan had them.
        commands.extend(contents_list_commands(&parsed.blocks, entry_doc, body_start(source)));
        // The running-head marks likewise (PLAN1 site 17).
        commands.extend(mark_commands(texts, &parsed.blocks, entry_doc, body_start(source)));
        commands.sort_by_key(|c| c.start);
        commands
    };
    // Every compiler `TitleBlock` is laid out at its `\maketitle` command
    // (the entry document's, in order) when the two correspond one to one;
    // otherwise (a `\maketitle` the compiler rejected, or one in an
    // `\input` file) they stay centred paragraphs.
    let maketitles = commands.iter().filter(|c| matches!(c.kind, BodyKind::MakeTitle)).count();
    let title_blocks = parsed.blocks.iter().filter(|b| matches!(b, CBlock::TitleBlock { .. })).count();
    let stash_titles = style.class_geometry.is_some() && maketitles == title_blocks && maketitles > 0;
    // `Parsed::block_par_leading` is one entry per block, in `blocks` order
    // (the compiler pushes both from the same place) whenever the pinned
    // `vendor/compiler` keeps that contract; `block_leadings` is what decides
    // whether it did, in both build profiles alike. Without the
    // `par-leading` feature the pinned `vendor/compiler` has no such field
    // and every paragraph keeps the body's `\baselineskip`, which is what
    // the pipeline did before this existed.
    #[cfg(feature = "par-leading")]
    let leadings: Vec<ParLeading> = block_leadings(&parsed.block_par_leading, parsed.blocks.len());
    #[cfg(not(feature = "par-leading"))]
    let leadings: Vec<ParLeading> = vec![None; parsed.blocks.len()];
    let paired: Vec<(CBlock, ParLeading)> = parsed.blocks.iter().cloned().zip(leadings).collect();
    let par_starts = ParStarts::new(parsed);
    let (mut lowered, mut limitations, stashed, letter_spans) = lower_blocks(texts, &paired, stash_titles);
    let mut stashed = stashed.into_iter();
    let title_of = |(title, authors, date, leading): StashedTitle, span: Span| Block::Title {
        title: items_for(&title, false),
        title_leading_pt: par_leading_pt(leading, style.base),
        // One `tabular` column per `\and` group, each split into rows at
        // its own `\\` (PLAN1 site 38). The groups are the compiler's:
        // before, they were recovered by testing whether the source at a
        // `LineBreak`'s span began with `\author`, which no `\author` a
        // macro produced ever does.
        authors: authors.iter().map(|g| tabular_rows(items_for(g, false))).collect(),
        date: date.map(|d| items_for(&d, false)),
        span,
    };
    // book.cls `\if@mainmatter` (true until `\frontmatter`).
    let mut mainmatter = true;
    strip_command_text(&mut lowered, entry_doc, &commands);
    // The same structural commands in `\input`/`\include`d documents (one
    // list per document, empty for the entry): a `\chapter` in
    // `chapters/one.tex` is as much a chapter as one in the entry file.
    // `\maketitle`, `\noindent`, contents lists and nested `\input`s stay
    // entry-only, as before.
    let included_commands: Vec<Vec<BodyCommand>> = texts
        .iter()
        .enumerate()
        .map(|(d, text)| {
            if d == entry {
                return Vec::new();
            }
            let mut cmds: Vec<BodyCommand> = body_commands(text, has_chapters, book)
                .into_iter()
                .filter(|c| matches!(c.kind, BodyKind::Event(_) | BodyKind::Chapter { .. } | BodyKind::Part { .. } | BodyKind::Appendix | BodyKind::Matter(_) | BodyKind::AddContentsLine { .. }))
                .collect();
            cmds.extend(page_style_commands(&parsed.blocks, DocumentId(d), body_start(text)));
            cmds.extend(mark_commands(texts, &parsed.blocks, DocumentId(d), body_start(text)));
            cmds.sort_by_key(|c| c.start);
            cmds
        })
        .collect();
    for (d, cmds) in included_commands.iter().enumerate() {
        strip_command_text(&mut lowered, DocumentId(d), cmds);
    }
    let mut next_included = vec![0usize; texts.len()];
    let mut seen_included = vec![false; texts.len()];
    let mut next_command = 0usize;
    // The `\input`/`\include`d document whose units are being laid out.
    let mut input_doc: Option<DocumentId> = None;
    // The `\include` whose file is being read: its closing `\clearpage`
    // comes when the entry document resumes.
    let mut open_include: Option<&BodyCommand> = None;
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
    // beamer: the contents list is the frame's `Block::BeamerToc`, its
    // entries the deck's `\section`s (compiler `BeamerSection`).
    let toc_active = !style.is_beamer() && commands.iter().any(|c| matches!(c.kind, BodyKind::ContentsList(_)));
    // Every heading steps its counter (`\refstepcounter`, starred ones
    // too) but only an unstarred one writes an entry; each contents list
    // records the counters where it stands, in order.
    let (beamer_sections, beamer_toc_currents): (Vec<BeamerTocEntry>, Vec<(usize, usize)>) = if style.is_beamer() {
        let (mut section, mut subsection) = (0usize, 0usize);
        let mut entries = Vec::new();
        let mut currents = Vec::new();
        for b in &parsed.blocks {
            match b {
                CBlock::BeamerSection { level: 1, title, number, .. } => {
                    section += 1;
                    subsection = 0;
                    if !number.is_empty() {
                        entries.push(BeamerTocEntry { level: 1, section, subsection: 0, items: items_for(title, true) });
                    }
                }
                CBlock::BeamerSection { level: 2, title, number, .. } => {
                    subsection += 1;
                    if !number.is_empty() {
                        entries.push(BeamerTocEntry { level: 2, section, subsection, items: items_for(title, true) });
                    }
                }
                CBlock::TableOfContents { .. } => currents.push((section, subsection)),
                _ => {}
            }
        }
        (entries, currents)
    } else {
        (Vec::new(), Vec::new())
    };
    let mut beamer_toc_index = 0usize;
    let toc_settings = crate::toc::Settings::read(source, has_chapters);
    // `\@sect` writes `\numberline` up to the same `\c@secnumdepth` that
    // decides the printed number; the two are one counter, resolved above.
    let toc_secnumdepth = secnumdepth;
    let mut toc_records: Vec<crate::toc::Record> = Vec::new();
    let mut toc_lists: Vec<(usize, crate::toc::ListKind, Span, bool)> = Vec::new();
    let mut toc_pending: Vec<String> = Vec::new();
    let mut chapter_starts: Vec<(usize, String)> = Vec::new();
    let mut chapter_gaps: Vec<usize> = Vec::new();
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
    // amsthm `\qedhere` in a display or alignment row claims the proof's
    // box even when the automatic pair lands in a later paragraph (the
    // `equation`/align environments flush before `\end{proof}`). A new
    // proof opens a new claim window; consuming the pair clears it.
    let mut pending_qed_claim = false;
    // The documents an entry `\input`/`\include` command reads (nested reads
    // included), from the reading order: a file whose body is only floats
    // has no unit to lay its `\chapter` out before.
    let read_by = |cmd: &BodyCommand| -> Vec<usize> {
        let order = &labels.reading_order;
        let Some(i) = order.iter().position(|s| s.document == entry_doc && s.end == cmd.start) else {
            return Vec::new();
        };
        let mut docs: Vec<usize> = Vec::new();
        for s in order[i + 1..].iter().take_while(|s| s.document != entry_doc) {
            if !docs.contains(&s.document.0) {
                docs.push(s.document.0);
            }
        }
        docs
    };
    // One pass per unit, then one (`None`) for the commands after the last.
    // `\hangfrom` commands whose hang the pipeline sets (see
    // `hangfrom_label`): the compiler's missing-hang warning for them is
    // superseded, like `abstract`'s unimplemented-environment one below.
    let mut hangfrom_spans: Vec<Span> = Vec::new();
    for mut next in split_at_page_breaks(&par_starts, texts, &lowered, size, &style).into_iter().map(Some).chain([None]) {
        // Does this unit continue the theorem-like environment that the
        // previous block left open? Only a paragraph inside it that is not
        // itself a fresh `\item` does.
        let continues_theorem = next.as_ref().is_some_and(|unit| matches!(
            unit.kind,
            UnitKind::Paragraph { in_theorem: true, theorem_item: false, .. }
        ));
        if !continues_theorem {
            if let Some(at) = open_theorem.take() {
                if let Some(Block::Paragraph { env_close, .. }) = blocks.get_mut(at) {
                    *env_close = true;
                }
            }
        }
        let mut eject_before = next.as_ref().is_some_and(|unit| unit.eject_before);
        let vspace_before = next.as_ref().map_or(0.0, |unit| unit.vspace_before);
        if let Some(unit) = &mut next {
            limitations.append(&mut unit.limitations);
        }
        let unit_start = next.as_ref().and_then(|unit| match &unit.kind {
            UnitKind::Heading { number_span, .. } => Some(*number_span),
            UnitKind::Paragraph { inlines, .. } => anchor_span(inlines.iter()),
            UnitKind::Rule { span } | UnitKind::Letter { span, .. } => Some(*span),
            UnitKind::FrameBegin { span, .. } | UnitKind::FrameEnd { span } | UnitKind::BeamerTitle { span, .. } | UnitKind::BeamerToc { span, .. } => Some(*span),
            UnitKind::BeamerBlockBegin { span, .. }
            | UnitKind::BeamerBlockEnd { span }
            | UnitKind::ColumnsBegin { span, .. }
            | UnitKind::Column { span, .. }
            | UnitKind::ColumnsEnd { span }
            | UnitKind::BeamerCaption { span, .. } => Some(*span),
            UnitKind::Picture { document, picture, .. } => Some(Span::in_document(*document, picture.start, picture.end)),
        });
        let at_end = next.is_none();
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
                // The command that reads this document, not merely the next
                // one: an earlier `\include` may read a file with no units. A
                // file read by a command already spent (nested) flushes none.
                let mut inputs = commands[next_command..].iter().filter(|c| matches!(c.kind, BodyKind::Input));
                if labels.reading_order.iter().any(|s| s.document == at.document) {
                    inputs.find(|c| read_by(c).contains(&at.document.0)).map(|c| c.start)
                } else {
                    inputs.next().map(|c| c.start)
                }
            }
            None if at_end => Some(usize::MAX),
            _ => None,
        };
        // `(document, command)` to lay out before this unit, in order.
        let mut pending: Vec<(DocumentId, &BodyCommand)> = Vec::new();
        // The entry document resumes, or the next file is read: the included
        // documents read so far are finished, so their commands after their
        // last unit come first.
        if let Some(at) = flush_before {
            for (d, cmds) in included_commands.iter().enumerate() {
                if seen_included[d] {
                    pending.extend(cmds[next_included[d]..].iter().map(|c| (DocumentId(d), c)));
                    next_included[d] = cmds.len();
                }
            }
            pending.extend(open_include.take().map(|cmd| (entry_doc, cmd)));
            while let Some(cmd) = commands.get(next_command).filter(|c| c.start < at) {
                next_command += 1;
                pending.push((entry_doc, cmd));
                if !matches!(cmd.kind, BodyKind::Input) {
                    continue;
                }
                // A file read here without a unit of its own (its body is
                // floats only): its commands, then `\include`'s closing
                // `\clearpage`, in place.
                for d in read_by(cmd) {
                    if let Some(cmds) = included_commands.get(d).filter(|_| !seen_included[d]) {
                        seen_included[d] = true;
                        pending.extend(cmds[next_included[d]..].iter().map(|c| (DocumentId(d), c)));
                        next_included[d] = cmds.len();
                    }
                }
                if is_include(source, cmd) {
                    pending.push((entry_doc, cmd));
                }
            }
            // The `\input` command that read this unit's document is spent.
            if let Some(cmd) = commands.get(next_command).filter(|c| input_doc.is_some() && c.start == at && matches!(c.kind, BodyKind::Input)) {
                next_command += 1;
                // `\include`'s opening `\clearpage`; the closing one waits.
                if is_include(texts.get(entry).copied().unwrap_or(""), cmd) {
                    pending.push((entry_doc, cmd));
                    open_include = Some(cmd);
                }
            }
        }
        // An included document's own commands precede its next unit.
        if let Some(at) = unit_start.filter(|at| at.document != entry_doc) {
            if let Some(cmds) = included_commands.get(at.document.0) {
                seen_included[at.document.0] = true;
                while let Some(cmd) = cmds.get(next_included[at.document.0]).filter(|c| c.start < at.start) {
                    next_included[at.document.0] += 1;
                    pending.push((at.document, cmd));
                }
            }
        }
        if !pending.is_empty() {
            for (cmd_doc, cmd) in pending {
                let source = texts.get(cmd_doc.0).copied().unwrap_or("");
                match &cmd.kind {
                    // latex.ltx `\@include`: `\clearpage` before the file is
                    // read (or skipped by `\includeonly`) and after it.
                    BodyKind::Input => {
                        if cmd_doc == entry_doc && is_include(source, cmd) {
                            blocks.push(Block::ClearPage { double: false, span: Span::in_document(cmd_doc, cmd.start, cmd.end) });
                            prev_para_end = None;
                        }
                    }
                    BodyKind::Event(event) => blocks.push(Block::Chrome {
                        event: event.clone(),
                        span: Span::in_document(cmd_doc, cmd.start, cmd.end),
                    }),
                    BodyKind::Chapter { starred, title } => {
                        if !*starred {
                            // In the reading-order space `list_blocks` sorts
                            // entries by, so an `\include`d chapter's gap
                            // lands between the right floats.
                            chapter_gaps.push(reading_position(&labels.reading_order, cmd_doc, cmd.start).unwrap_or(cmd.start));
                        }
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
                            // `\thelstlisting` (#543): listings are still
                            // numbered from the entry document's chapter
                            // starts (`toc::chapter_numbers`); figures and
                            // tables carry `floats::number`'s reading-order
                            // numbers instead.
                            if cmd_doc == entry_doc {
                                chapter_starts.push((cmd.start, chapter_label.clone()));
                            }
                            chapter_label.clone()
                        });
                        let span = Span::in_document(cmd_doc, cmd.start, cmd.end);
                        let mut items = words_from_source(source, cmd_doc, title.0, title.1);
                        if toc_active {
                            if let Some(n) = &number {
                                // report.cls `\@chapter`: `\addcontentsline{toc}{chapter}{\protect\numberline{\thechapter}#1}`.
                                let key = crate::toc::key(toc_records.len());
                                toc_records.push(crate::toc::Record {
                                    list: crate::toc::ListKind::Toc,
                                    level: 0,
                                    number: Some((n.clone(), span)),
                                    title: labels.entry_items.get(cmd_doc, title.0, title.1).unwrap_or_else(|| items.clone()),
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
                            let span = Span::in_document(cmd_doc, cmd.start, cmd.end);
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
                                span: Span::in_document(cmd_doc, cmd.start, cmd.end),
                            });
                        }
                    }
                    BodyKind::Matter(matter) => {
                        let span = Span::in_document(cmd_doc, cmd.start, cmd.end);
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
                    // beamer's `\tableofcontents` is the frame's own
                    // `Block::BeamerToc` (`UnitKind::BeamerToc`), not a
                    // spliced list under a `Contents` heading.
                    BodyKind::ContentsList(_) if style.is_beamer() => {}
                    BodyKind::ContentsList(kind) => {
                        // `\newpage` (etc.) right before the command breaks
                        // before the list's heading.
                        let before = source[..cmd.start].trim_end();
                        let eject = ["\\newpage", "\\clearpage", "\\cleardoublepage", "\\pagebreak"].iter().any(|c| before.ends_with(c));
                        toc_lists.push((blocks.len(), *kind, Span::in_document(cmd_doc, cmd.start, cmd.end), eject));
                    }
                    BodyKind::AddContentsLine { list, level, text } => {
                        if let (true, Some(level)) = (toc_active, crate::toc::level_of(level)) {
                            let (number, title) = crate::toc::contentsline_text(source, cmd_doc, text.0, text.1, &labels.entry_items);
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
                        let span = Span::in_document(cmd_doc, cmd.start, cmd.end);
                        let mut items = words_from_source(source, cmd_doc, title.0, title.1);
                        if toc_active {
                            if let Some(n) = &number {
                                let (s, e) = short.unwrap_or(*title);
                                let key = crate::toc::key(toc_records.len());
                                toc_records.push(crate::toc::Record {
                                    list: crate::toc::ListKind::Toc,
                                    level: -1,
                                    number: Some((n.clone(), span)),
                                    title: labels.entry_items.get(cmd_doc, s, e).unwrap_or_else(|| words_from_source(source, cmd_doc, s, e)),
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
        }
        let Some(unit) = next else { break };
        match unit.kind {
            UnitKind::Heading {
                level,
                number,
                number_span,
                content,
                leading,
                head_style,
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
                let title = mark_title(texts, content);
                // LaTeX `\@seccntformat`: the counter, then `\quad`, then the
                // title; the number's bytes are the `\section` command's.
                let mut items = Vec::new();
                let numbered = !number.is_empty() && level <= secnumdepth;
                if numbered {
                    let chars = number
                        .chars()
                        .map(|_| CharSrc {
                            document: number_span.document,
                            start: number_span.start,
                            end: number_span.end,
                        })
                        .collect();
                    // `\@sect` sets `\@svsec` inside `#6{...}`, so the
                    // number and its `\quad` are at `#6`'s size, not the
                    // one the pipeline gives this heading level: a
                    // `{\large\bf}` `\section` numbers at 12pt where the
                    // level would use `\Large`'s 14.4pt. `size_cpt` 0 --
                    // the standard classes' own sectioning, whose `#6`
                    // declares no size -- is still the level's size.
                    let head = TextStyle { size_cpt: declared_size(head_style.size, size), ..TextStyle::default() };
                    push_segment(&mut items, number.to_string(), chars, head);
                    items.push(Item::Quad { em: 1.0, plus_em: 0.0, minus_em: 0.0, style: head });
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
                        leading_pt: par_leading_pt(leading, style.base),
                        numbered,
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
            UnitKind::FrameBegin { block, span } => {
                if let CBlock::BeamerFrameBegin { options, spec, title, subtitle, slides, .. } = block {
                    use flashtex_class_geometry::beamer::FrameAlign;
                    use flashtex_compiler::parser::BeamerFrameAlign;
                    blocks.push(Block::FrameBegin {
                        title: items_for(title, true),
                        subtitle: items_for(subtitle, true),
                        align: match options.align {
                            BeamerFrameAlign::Top => FrameAlign::Top,
                            BeamerFrameAlign::Center => FrameAlign::Center,
                            BeamerFrameAlign::Bottom => FrameAlign::Bottom,
                        },
                        plain: options.plain,
                        allowframebreaks: options.allowframebreaks,
                        slides: *slides,
                        slide: 1,
                        spec: spec.clone(),
                        first_slide: true,
                        span,
                    });
                }
                // The frame's body starts in vertical mode: no `\@nobreak`
                // from a heading, no paragraph to rejoin.
                after_heading = false;
                prev_para_end = None;
            }
            UnitKind::FrameEnd { span } => {
                blocks.push(Block::FrameEnd { span, addvspace_before: unit.addvspace_before, addvspace_flex: unit.addvspace_flex, vspace_before });
                after_heading = false;
                prev_para_end = None;
            }
            UnitKind::BeamerTitle { block, span } => {
                if let CBlock::BeamerTitlePage { title, subtitle, authors, institute, date, .. } = block {
                    blocks.push(Block::BeamerTitle {
                        title: items_for(title, true),
                        subtitle: items_for(subtitle, true),
                        authors: items_for(authors, true),
                        institute: items_for(institute, true),
                        date: items_for(date, true),
                        span,
                    });
                }
                after_heading = false;
                prev_para_end = None;
            }
            UnitKind::BeamerToc { span, options } => {
                let current = beamer_toc_currents.get(beamer_toc_index).copied().unwrap_or((0, 0));
                beamer_toc_index += 1;
                blocks.push(Block::BeamerToc { entries: beamer_sections.clone(), current, options: options.to_string(), span });
                after_heading = false;
                prev_para_end = None;
            }
            UnitKind::Letter { kind, lines, extra_gap_after_pt, gap_before_pt, gap_after_pt, indent_pt, span } => {
                blocks.push(Block::Letter {
                    kind,
                    lines: lines.iter().map(|l| items_for(l, false)).collect(),
                    extra_gap_after_pt,
                    gap_before_pt,
                    gap_after_pt,
                    indent_pt,
                    span,
                    eject_before,
                    vspace_before,
                });
                after_heading = false;
                prev_para_end = None;
            }
            // beamer Tier 3 (#944): blocks, columns and captions. Each edge
            // carries the list-closing `\addvspace` and any `\vspace`
            // before it, like `FrameEnd`; the body between the edges starts
            // in vertical mode.
            UnitKind::BeamerBlockBegin { block, span } => {
                if let CBlock::BeamerBlockBegin { kind, title, .. } = block {
                    blocks.push(Block::BeamerBlockBegin {
                        kind: *kind,
                        title: items_for(title, true),
                        span,
                        addvspace_before: unit.addvspace_before,
                        addvspace_flex: unit.addvspace_flex,
                        vspace_before,
                    });
                }
                after_heading = false;
                prev_para_end = None;
            }
            UnitKind::BeamerBlockEnd { span } => {
                blocks.push(Block::BeamerBlockEnd { span, addvspace_before: unit.addvspace_before, addvspace_flex: unit.addvspace_flex, vspace_before });
                after_heading = false;
                prev_para_end = None;
            }
            UnitKind::ColumnsBegin { block, span } => {
                if let CBlock::BeamerColumnsBegin { options, .. } = block {
                    blocks.push(Block::ColumnsBegin {
                        options: options.clone(),
                        span,
                        addvspace_before: unit.addvspace_before,
                        addvspace_flex: unit.addvspace_flex,
                        vspace_before,
                    });
                }
                after_heading = false;
                prev_para_end = None;
            }
            UnitKind::Column { block, span } => {
                if let CBlock::BeamerColumn { width, align, .. } = block {
                    blocks.push(Block::Column { width: width.clone(), align: *align, span });
                }
                after_heading = false;
                prev_para_end = None;
            }
            UnitKind::ColumnsEnd { span } => {
                blocks.push(Block::ColumnsEnd { span, addvspace_before: unit.addvspace_before, addvspace_flex: unit.addvspace_flex, vspace_before });
                after_heading = false;
                prev_para_end = None;
            }
            UnitKind::BeamerCaption { block, span } => {
                // `beamer@makecaption` (beamerbaselocalstructure.sty 589-601)
                // with the default `caption` template: `\insertcaptionname`
                // + `:\ ` in the `caption name` colour (structure), the text,
                // all `\small`; set as `\hb@xt@\hsize{\hfil ... \hfil}` when
                // it fits one line (a centred paragraph; a longer one is
                // `\raggedright`, which is not told apart here). The 7pt
                // `\abovecaptionskip`/`\belowcaptionskip` are the compiler's
                // `VSpace` blocks around it.
                if let CBlock::BeamerCaption { kind, content, .. } = block {
                    use flashtex_compiler::parser::FontSizeLevel;
                    let (r, g, b) = flashtex_class_geometry::beamer::STRUCTURE_RGB;
                    let bn = |v: f64| (v * 1e9).round() as u32;
                    let structure = DeviceColor::from_billionths(flashtex_compiler::color::ColorSpace::Rgb, &[bn(r), bn(g), bn(b)]);
                    let small = TextStyle { size_cpt: declared_size(Some(FontSizeLevel::Small), size), ..TextStyle::default() };
                    let mut items = command_words(&format!("{}:", kind.name()), span);
                    for it in &mut items {
                        if let Item::Word(w) = it {
                            for seg in &mut w.segments {
                                seg.style = TextStyle { color: structure, ..small };
                            }
                        }
                    }
                    // `\ ` (control space): factor 1000 whatever the colon.
                    items.push(Item::Space { style: small, factor: 1000, no_break: false });
                    items.extend(items_for(content, false));
                    blocks.push(Block::Paragraph {
                        parts: vec![ParaPart::Lines(items)],
                        indent: false,
                        style: ParaStyle::Center,
                        env_open: None,
                        env_close: false,
                        eject_before,
                        vspace_before,
                        addvspace_before: unit.addvspace_before,
                        addvspace_flex: unit.addvspace_flex,
                        vspace_flex: unit.vspace_flex,
                        endlist_adjust: unit.endlist_adjust,
                        penalty_before: unit.penalty_before,
                        list: None,
                        sized: None,
                        // The one-line caption is an `\hbox` appended to the
                        // outer list: its interline glue is the body's
                        // `\baselineskip`, not `\small`'s (measured: 6.656pt
                        // = 13.6 - 6.944 under a depthless image line).
                        leading_pt: None,
                        hang: None,
                    });
                }
                after_heading = false;
                prev_para_end = None;
            }
            UnitKind::Picture {
                document,
                picture,
                centered,
                initial,
                after_env,
                noindent,
                theorem_item,
                list,
                caption,
                styled,
            } => {
                // The paragraph path's indent decision verbatim (a picture
                // carries no run-in head, so that arm is empty).
                let indent = initial && !after_heading && !caption && styled.is_none() && !after_env && !theorem_item && list.is_none() && !noindent;
                blocks.push(Block::Picture {
                    document,
                    picture,
                    centered,
                    indent,
                    list,
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
                label_inlines,
                run_in,
                par_leading,
                hang_label,
            } => {
                let list = list.map(|mut geom| {
                    geom.label_items = label_inlines.map(|content| items_for(content, false));
                    geom
                });
                // `\hangfrom{label}`: the label's own items, whose shaped
                // natural width is the hang (see `hangfrom_label`). The
                // command's span joins the superseded diagnostics below:
                // the compiler's missing-hang warning no longer applies.
                let hang = hang_label.map(|(label, cmd)| {
                    hangfrom_spans.push(cmd);
                    let mut label_items = items_for_weighted(label, in_theorem);
                    // The label's trailing gap must survive the measure:
                    // `hlist` drops trailing glue (tex.web §816), so save
                    // it with a zero kern -- the same trick `\\hrulefill`
                    // uses for its fill above. Zero-width, so labels
                    // without a trailing gap measure unchanged.
                    label_items.push(Item::Kern {
                        amount: flashtex_compiler::text_builtins::TextDimen::zero(),
                        style: TextStyle::default(),
                    });
                    label_items
                });
                for inline in inlines {
                    unsupported_inlines(inline, &mut limitations);
                    if let Inline::Tabular(t) = inline {
                        table_length_limitations(texts, t.span, size, &mut limitations);
                    }
                }
                // amsthm sets the head bold (italic for `remark`/`proof`)
                // and, for the `plain` style, the body italic. None of that
                // is in the source bytes at the head's span (which is the
                // `\begin` command), so the weights come from the compiler's
                // own scoping inside a theorem-like environment.
                //
                // A paragraph opening a proof starts a new `\qedhere` claim
                // window (a pending display claim belongs to the proof that
                // held the display); a display or alignment-row marker in
                // this paragraph claims the box for the proof's end,
                // wherever the automatic pair lands.
                if inlines.first().map(inline_span).is_some_and(|s| {
                    texts.get(s.document.0).and_then(|t| t.get(s.start..)).is_some_and(|r| r.starts_with("\\begin{proof}"))
                }) {
                    pending_qed_claim = false;
                }
                pending_qed_claim |= paragraph_claims_qed(inlines, texts);
                let mut items = items_for_weighted(inlines, in_theorem);
                if list.as_ref().is_some_and(|l| l.bibliography) {
                    bibliography_space_factors(&mut items);
                }
                if pending_qed_claim && truncate_auto_pair(&mut items, texts) {
                    pending_qed_claim = false;
                }
                // `\paragraph`/`\subparagraph`: the head's weight, its
                // `\normalsize` and `\@xsect`'s `\hskip -#5` are the
                // compiler's own inlines at the front of this paragraph
                // (`ParStart::run_in`), so nothing is applied over the
                // items here. All that is left of the head for this layer
                // is `\addvspace{#4}`, below.
                items.splice(0..0, toc_pending.drain(..).map(|key| Item::Label { key }));
                let mut parts = Vec::new();
                let mut current = Vec::new();
                for item in items {
                    match item {
                        Item::Math { list, span, .. } if is_display(inlines, span) => {
                            if !current.is_empty() {
                                parts.push(ParaPart::Lines(std::mem::take(&mut current)));
                            }
                            // amsmath rows: the environment's rows become one
                            // alignment; `\tag{..}` replaces a row's number.
                            if let Some((rows_span, row)) = math_row_of(inlines, span) {
                                let rows_rest = texts.get(rows_span.document.0).and_then(|t| t.get(rows_span.start..)).unwrap_or("");
                                let mut tag = None;
                                let mut qed_here = None;
                                let cells: Vec<MathList> = row
                                    .cells
                                    .iter()
                                    .map(|c| {
                                        let tagged = strip_tag(texts, c, &mut tag, &mut limitations);
                                        let (stripped, qed) = strip_qedhere(texts, tagged);
                                        qed_here = qed_here.or(qed);
                                        stripped
                                    })
                                    .collect();
                                let (number, number_math) = match tag {
                                    Some((t, math)) => (Some((t, row.span)), math),
                                    None => (row.number.clone().map(|n| (format!("({n})"), row.span)), None),
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
                                let mut shove = row.shove;
                                if shove.is_some() && !amsmath {
                                    // `\shoveleft`/`\shoveright` are amsmath
                                    // commands (pdflatex: `Undefined control
                                    // sequence` without it); the row keeps
                                    // the display's default placement.
                                    let name = if shove.is_some_and(|s| {
                                        s == flashtex_compiler::parser::ShoveDirection::Left
                                    }) {
                                        "shoveleft"
                                    } else {
                                        "shoveright"
                                    };
                                    limitations.push((
                                        "math_limitation",
                                        row.span,
                                        format!(
                                            "\\{name} requires \\usepackage{{amsmath}}; the row keeps the display's default placement"
                                        ),
                                    ));
                                    shove = None;
                                }
                                let part = RowPart {
                                    cells,
                                    shove,
                                    number,
                                    number_math,
                                    span: row.span,
                                    intertext,
                                    qed_here,
                                };
                                match parts.last_mut() {
                                    Some(ParaPart::Rows { span: s, rows, .. }) if *s == rows_span => rows.push(part),
                                    _ => {
                                        let env = RowsEnv::at(rows_rest);
                                        let multline_gap = if matches!(env, RowsEnv::Multline) {
                                            multline_gap_of(texts.get(rows_span.document.0).copied().unwrap_or(""), size)
                                        } else {
                                            MULTLINE_GAP_DEFAULT
                                        };
                                        parts.push(ParaPart::Rows {
                                            env,
                                            rows: vec![part],
                                            span: rows_span,
                                            bracket: false,
                                            multline_gap,
                                        });
                                    }
                                }
                                continue;
                            }
                            // The compiler numbers the `equation` environment
                            // (a macro-opened one too, PLAN1 site 24); a `\tag`
                            // numbers any display.
                            let rest = texts.get(span.document.0).and_then(|t| t.get(span.start..)).unwrap_or("");
                            let mut tag = None;
                            let list = strip_tag(texts, &list, &mut tag, &mut limitations);
                            let (list, eqno) = strip_eqno(texts, list, span);
                            let number_math = tag.as_ref().and_then(|(_, math)| math.clone());
                            let number = match (tag, eqno) {
                                (Some((t, _)), _) => Some((t, span)),
                                (None, Some(n)) => Some(n),
                                (None, None) => display_number(inlines, span).map(|(n, s)| (format!("({n})"), s)),
                            };
                            let bracket = !amsmath && (rest.starts_with("\\[") || rest.starts_with("\\begin{displaymath}"));
                            let (list, qed_here) = strip_qedhere(texts, list);
                            parts.push(ParaPart::Display {
                                list,
                                span,
                                number,
                                number_math,
                                bracket,
                                qed_here,
                            });
                        }
                        other => current.push(other),
                    }
                }
                if !current.is_empty() {
                    parts.push(ParaPart::Lines(current));
                }
                // amsmath's `multline` carries a single tag/number on its
                // LAST row wherever `\tag` was typed (TeX Live 2026
                // pdflatex sets `b+b \tag{B}` over `c+c   (B)`, the tag
                // sharing the last row's baseline). Each row above
                // extracts its own number from its own cells, so a tag on
                // any row but the last lands on that row's line instead —
                // and a compiler predating the per-environment tag
                // suppression additionally numbers the last row, printing
                // both. The tag's row is the one whose source carries
                // `\tag`; its number moves to the last row and every other
                // row's number is dropped. With no `\tag` anywhere (the
                // untagged and tagged-last-row shapes) nothing moves.
                for part in &mut parts {
                    if let ParaPart::Rows { env: RowsEnv::Multline, rows, .. } = part {
                        if rows.len() < 2 {
                            continue;
                        }
                        let tagged = rows.iter().position(|row| {
                            texts
                                .get(row.span.document.0)
                                .and_then(|text| text.get(row.span.start..row.span.end))
                                .is_some_and(|source| find_command(source, "tag").is_some())
                        });
                        if let Some(found) = tagged {
                            if rows[found].number.is_none() {
                                continue;
                            }
                            let number = rows[found].number.take();
                            // A rich tag's run (`number_math`) travels with
                            // its text, or the last row would set the
                            // flattened text instead (#441).
                            let number_math = rows[found].number_math.take();
                            for row in rows.iter_mut() {
                                row.number = None;
                                row.number_math = None;
                            }
                            if let Some(last) = rows.last_mut() {
                                last.number = number;
                                last.number_math = number_math;
                            }
                        }
                    }
                }
                let only_labels = parts
                    .iter()
                    .all(|p| matches!(p, ParaPart::Lines(items) if items.iter().all(|i| matches!(i, Item::Label { .. }))));
                if parts.is_empty() {
                    // An empty-body list item (`\item` with no text) still
                    // carries its bullet/label -- pdflatex typesets the
                    // marker on a line of its own.  Give it an empty Lines
                    // part so the typesetter can prepend the label box.
                    if list.is_some() {
                        parts.push(ParaPart::Lines(Vec::new()));
                    } else {
                        continue;
                    }
                }
                // `\longtable` begins with `\par` and `\endlongtable`
                // ends with one, so the compiler always gives it a
                // paragraph of its own: it becomes a block the page
                // builder can break inside rather than a box on a line.
                if let Some(table) = lone_longtable(&mut parts) {
                    let src = texts.get(table.span.document.0).copied().unwrap_or("");
                    blocks.push(Block::LongTable {
                        lengths: LongtableLengths::read(|name| length_at(src, name, size, table.span.start, 0.0)),
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
                let first_span = anchor_span(inlines.iter());
                let starts_display = matches!(parts.first(), Some(ParaPart::Display { .. } | ParaPart::Rows { .. }));
                if let (Some(Block::Paragraph { parts: prev_parts, style: prev_style, list: prev_list, .. }), Some(_), Some(_)) = (blocks.last_mut(), first_span, prev_para_end) {
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
                    if same_flow && (starts_display || prev_ends_display) && par_starts.of(inlines).is_some_and(|s| !s.par_before) {
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
                if only_labels && list.is_none() {
                    continue;
                }
                prev_para_end = inlines.iter().map(inline_span).last();
                // `\noindent` before the paragraph's first material (the
                // compiler's `ParStart::indent`, from the source or a macro).
                let noindent = par_starts.of(inlines).is_some_and(|s| !s.indent);
                // `\centering` sets `\parindent 0pt`; a list item's first
                // paragraph carries no indent and `\list` sets
                // `\parindent\listparindent` (0pt in article) for the
                // ones after it, `quote` likewise.
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
                let run_in_skip = run_in.map(|level| style.heading(level).before);
                let (addvspace_before, addvspace_flex) = match run_in_skip {
                    Some(s) if s.natural > unit.addvspace_before => (s.natural, (s.stretch, s.shrink)),
                    _ => (unit.addvspace_before, unit.addvspace_flex),
                };
                blocks.push(Block::Paragraph {
                    parts,
                    // `\@xsect` throws this paragraph's `\parindent` box away
                    // and sets `\hskip #3` in its place, unconditionally, so
                    // for a run-in head the compiler's own answer stands
                    // whatever else here would have suppressed the indent.
                    indent: match run_in {
                        Some(_) => !noindent,
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
                    penalty_before: unit.penalty_before,
                    list,
                    sized: None,
                    leading_pt: par_leading_pt(par_leading.or_else(|| size_env_par_leading(texts, &styles, inlines)), style.base),
                    hang,
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
    // A list after the last material (a document that is nothing but its
    // lists, or `\listoffigures` at the very end) is set there too.
    for cmd in &commands[next_command..] {
        if style.is_beamer() {
            break;
        }
        if let BodyKind::ContentsList(kind) = cmd.kind {
            let before = source[..cmd.start].trim_end();
            let eject = ["\\newpage", "\\clearpage", "\\cleardoublepage", "\\pagebreak"].iter().any(|c| before.ends_with(c));
            toc_lists.push((blocks.len(), kind, Span::in_document(entry_doc, cmd.start, cmd.end), eject));
        }
    }
    // The contents lists, now that every record is known.
    for (at, kind, span, eject) in toc_lists.into_iter().rev() {
        let list = crate::toc::list_blocks(kind, span, eject, &toc_settings, &toc_records, labels, &chapter_starts, &chapter_gaps);
        blocks.splice(at..at, list);
    }
    // `abstract`: the compiler sets its body as plain text, so the class's
    // own shape (the centred `\small\bfseries` head and the `\small`
    // `quotation`) is read from the source bytes here, before the
    // `env_close` pass below derives the closing skips from the styles.
    let mut superseded = crate::abstractenv::apply(texts, &mut blocks, &style);
    superseded.extend(hangfrom_spans);
    // fontspec's `\setmainfont`/`\fontspec`/`\newfontfamily` (and the
    // manifest's `[fonts]`): the named families, read from the source the
    // same way, marked on the runs they cover. A document naming no font
    // returns at once with the blocks untouched.
    let fontspec = crate::fontspec::apply(texts, entry, &mut blocks, &mut style, options);
    superseded.extend(fontspec.superseded);
    limitations.extend(fontspec.limitations);
    // `\twocolumn`/`\onecolumn` are set here, from the source, the same way:
    // the pinned `vendor/compiler` reports them as unknown commands.
    superseded.extend(
        style
            .columns
            .spans()
            .iter()
            .map(|&(s, e)| Span::in_document(flashtex_compiler::DocumentId(entry), s, e)),
    );
    // `\twocolumn[<material>]` sets its argument at the full `\textwidth`
    // above both columns (`\@topnewpage`, latex.ltx 20466-20505). The
    // material is cut out of the block stream here, brackets and all, and
    // carried on `Doc::top_material` for `typeset::build_with_floats` to
    // set in the box; what it cannot cut exactly stays where it is and is
    // reported, as before.
    // Size environments are set here (`apply_size_environments`); the
    // compiler's "environment is not implemented" for them is superseded.
    for (d, st) in styles.iter().enumerate() {
        superseded.extend(st.size_envs.iter().map(|e| Span::in_document(flashtex_compiler::DocumentId(d), e.begin, e.begin)));
    }
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
    let (listing_superseded, listing_limitations) = crate::listings::apply(texts, &mut blocks, &style, labels, &chapter_starts);
    superseded.extend(listing_superseded);
    superseded.extend(crate::listings::lstset_spans(texts));
    limitations.extend(listing_limitations);
    let mut top_material = None;
    // `\twocolumn[<material>]` sets its argument in a `\textwidth` box
    // above both columns (`\@topnewpage`, latex.ltx 20466-20505). The
    // material is cut out of the block stream here, brackets and all, and
    // carried on `Doc::top_material` for `typeset::build_with_floats`;
    // `\@topnewpage`'s own geometry is that page builder's business.
    //
    // Only when the frame really has two columns: the box belongs to the
    // two-column output routine, and a one-column page has nowhere for it.
    let two_column = style.class_geometry.as_deref().is_some_and(|g| g.frame.columns.len() > 1);
    let mut boxed = None;
    if let Some((open, close)) = style.columns.top_material().filter(|_| two_column) {
        let document = flashtex_compiler::DocumentId(entry);
        if let Some(m) = split_top_material(&mut blocks, document, open, close) {
            boxed = Some(open);
            top_material = Some((m, Span::in_document(document, open, close + 1)));
            // The compiler warns on the `[` that its own IR has no
            // `\@topnewpage` model ("the material is typeset as ordinary
            // text instead, brackets included") and deliberately leaves the
            // tokens where they stand so a renderer that *does* have the box
            // can cut them back out. This is that renderer, and it just did:
            // the warning describes an output this pipeline does not
            // produce, so it is superseded the way `abstract`'s is. The
            // unboxed case below supersedes it too, replacing it with the
            // typed `twocolumn_top_material` limitation.
            superseded.push(Span::in_document(document, open, open));
        }
    }
    // Every optional argument that did *not* become a box: one on a
    // `\twocolumn` that is not the document's first material (a preamble
    // one is `\@nodocument`'s error, a later one would have to change the
    // column count of the pages), and one whose material cannot be cut out
    // of the column text exactly. The compiler leaves those where they
    // stand, brackets and all, which is what #746 reported.
    for &(_, end) in style.columns.spans() {
        // The same rule `columns::optional_bracket` uses: `\@ifnextchar [`
        // skips space tokens, and a blank line is a `\par`, not a space —
        // so a `[` after a blank line is ordinary text, not the argument.
        let Some(open) = crate::columns::optional_bracket(source, end) else {
            continue;
        };
        if source[..end].ends_with("\\twocolumn") && boxed != Some(open) {
            limitations.push((
                "twocolumn_top_material",
                Span::in_document(flashtex_compiler::DocumentId(entry), end, end),
                "the optional argument of \\twocolumn sets material at the full \\textwidth \
                 above both columns (\\@topnewpage); that is not done here, so the material \
                 is set in the first column instead, brackets included"
                    .to_string(),
            ));
            // The compiler's own warning on the same `[` says the same fact
            // less precisely (it cannot know whether this pipeline boxed the
            // material); this typed limitation replaces it, so the reader
            // sees one diagnostic per `\twocolumn[`, not two.
            superseded.push(Span::in_document(flashtex_compiler::DocumentId(entry), open, open));
        }
    }
    // beamer: nested itemize/enumerate bodies take `\small`/`\footnotesize`
    // and the leading of the size in force at their `\par`.
    if style.is_beamer() {
        beamer_nested_list_sizes(&mut blocks, style.base);
    }
    // beamer: a frame with overlays is set once per slide
    // (`crate::overlay`), before the block-index tables below are taken.
    crate::overlay::expand_frames(&mut blocks);
    let beamer = parsed.beamer.as_ref().map(|d| BeamerDeck {
        short_title: items_for(&d.short_title, false),
        short_author: items_for(&d.short_author, false),
        short_institute: items_for(&d.short_institute, false),
        short_date: items_for(&d.short_date, false),
        logo: items_for(&d.logo, false),
    });
    // One post-material switch the page builder lays out (slice 1): a
    // single bare `\twocolumn`/`\onecolumn` at a clean block boundary,
    // with the post-switch stylesheet beside it. Everything else keeps
    // today's limitation. Computed here, on the final block list (the
    // `listings` inserts and the `top_material` cut above both move
    // indices), so the recorded block index is what `typeset` will read.
    let column_switch = column_switch_block(&blocks, source, entry, &style.columns);
    let post_style = column_switch.and_then(|sw| {
        let mut frame = style.class_geometry.as_deref()?.clone();
        frame.set_twocolumn(sw.on);
        let mut post = style.clone();
        post.text_width_pt = crate::style::frame_pt(frame.frame.columns.first()?.width);
        post.class_geometry = Some(Box::new(frame));
        Some(Box::new(post))
    });
    let column_switch = column_switch.filter(|_| post_style.is_some());
    // Every switch the page frame cannot follow: each one sets every
    // `\if@twocolumn` test (and its own page break), but the pages it
    // opens keep whatever column count the frame was built with. The
    // recorded switch is laid out instead, so it says nothing here; the
    // page builder reports it back itself on the paths it has to decline.
    let recorded = column_switch.map(|sw| sw.at);
    for &(at, on) in &style.columns.unmodelled() {
        if recorded == Some(at) {
            continue;
        }
        limitations.push((
            "twocolumn_mid_document",
            Span::in_document(flashtex_compiler::DocumentId(entry), at, at + if on { "\\twocolumn".len() } else { "\\onecolumn".len() }),
            crate::columns::mid_document_message(on, style.columns.start()),
        ));
    }
    let page_starts = clear_page_blocks(texts, &blocks);
    // After `listings::apply`, which can insert blocks: the ranges are
    // block indices, so they are taken once the block list is final.
    let abstract_pages = crate::abstractenv::page_ranges(texts, &blocks, &style);
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
        abstract_pages,
        superseded,
        top_material,
        beamer,
        column_switch,
        post_style,
    }
}

/// The characters of `w` whose source lies in `lo..hi` of `document`, as a
/// word of their own; `None` when none do. A word's `chars` are one per
/// `char` of its `text`, in order, so the two are cut together.
fn trim_word(w: &Word, document: flashtex_compiler::DocumentId, lo: usize, hi: usize) -> Option<Word> {
    let mut segments: Vec<Segment> = Vec::new();
    for s in &w.segments {
        let mut text = String::new();
        let mut chars: Vec<CharSrc> = Vec::new();
        for (c, ch) in s.chars.iter().zip(s.text.chars()) {
            if c.document == document && c.start >= lo && c.start < hi {
                text.push(ch);
                chars.push(c.clone());
            }
        }
        if !text.is_empty() {
            segments.push(Segment {
                text,
                chars,
                style: s.style,
            });
        }
    }
    (!segments.is_empty()).then_some(Word { segments })
}

/// The source range an item covers in `document`, for the items that carry
/// one. `None` is "no position of its own" (interword glue, `\hfill`, a
/// `\label`), which belongs with whatever stands before it.
fn item_range(it: &Item, document: flashtex_compiler::DocumentId) -> Option<(usize, usize)> {
    let of = |s: Span| (s.document == document).then_some((s.start, s.end));
    match it {
        Item::Word(w) => of(w.span()),
        Item::Math { span, .. } | Item::Logo { span, .. } | Item::Rule { span, .. } | Item::Footnote { span, .. } | Item::Overlong { span, .. } => of(*span),
        _ => None,
    }
}

/// The source range a block's material covers in `document`. `None` means
/// the block carries no position there, which `split_top_material` reads as
/// "cannot be placed relative to the box" and refuses.
fn block_range(b: &Block, document: flashtex_compiler::DocumentId) -> Option<(usize, usize)> {
    let of = |s: &Span| (s.document == document).then_some((s.start, s.end));
    match b {
        Block::Paragraph { parts, .. } => {
            let mut range: Option<(usize, usize)> = None;
            for part in parts {
                let r = match part {
                    ParaPart::Lines(items) => items.iter().filter_map(|i| item_range(i, document)).fold(None, |a: Option<(usize, usize)>, r| {
                        Some(a.map_or(r, |a| (a.0.min(r.0), a.1.max(r.1))))
                    }),
                    ParaPart::Display { span, .. } | ParaPart::Rows { span, .. } => of(span),
                };
                if let Some(r) = r {
                    range = Some(range.map_or(r, |a| (a.0.min(r.0), a.1.max(r.1))));
                }
            }
            range
        }
        Block::Heading { span, .. }
        | Block::Chapter { span, .. }
        | Block::Part { span, .. }
        | Block::Title { span, .. }
        | Block::Letter { span, .. }
        | Block::ClearPage { span, .. }
        | Block::NoBreakFalse { span }
        | Block::Chrome { span, .. }
        | Block::Rule { span, .. } => of(span),
        _ => None,
    }
}

/// Whether a block contributes to the page's vertical list. The ones that
/// do not (a page-style command, `\clearpage`) may stand on either side of
/// `\twocolumn`'s box without saying anything about where the box is.
fn is_material(b: &Block) -> bool {
    !matches!(b, Block::Chrome { .. } | Block::ClearPage { .. } | Block::NoBreakFalse { .. })
}

/// Splits `\twocolumn`'s optional argument out of `blocks`. `open` and
/// `close` are the byte offsets of its `[` and `]`, which are dropped.
///
/// `\@topnewpage` sets the argument in a box of its own, so the text after
/// the `]` starts a fresh paragraph in vertical mode — hence the `indent`
/// on the remainder of a paragraph the `]` fell inside.
///
/// Returns `None`, leaving `blocks` untouched, when the material cannot be
/// cut exactly: anything the box's page would already have set before it,
/// a block other than a paragraph straddling a bracket, or material from
/// another document (`\input` inside the argument).
fn split_top_material(blocks: &mut Vec<Block>, document: flashtex_compiler::DocumentId, open: usize, close: usize) -> Option<Vec<Block>> {
    let mut top: Vec<Block> = Vec::new();
    let mut keep: Vec<Block> = Vec::new();
    // Blocks are in document order: once a material block starts past the
    // `]`, every block after it does too, so they join `keep` without
    // needing a source range of their own. Only blocks up to and including
    // the one containing the `]` must place relative to the box — a later
    // `tikzpicture`, `longtable`, `\tableofcontents` or `\input` block
    // carries no entry-document range, and requiring one of it refused the
    // whole split and lost the banner.
    let mut past_close = false;
    for block in blocks.iter() {
        if !is_material(block) {
            keep.push(block.clone());
            continue;
        }
        if past_close {
            keep.push(block.clone());
            continue;
        }
        let (lo, hi) = block_range(block, document)?;
        if hi <= open {
            // Material before the `[`: the command is not this page's
            // first material after all, and `\@topnewpage` never ran here.
            return None;
        }
        if lo > close {
            past_close = true;
            keep.push(block.clone());
            continue;
        }
        if lo > open && hi <= close {
            top.push(block.clone());
            continue;
        }
        // The block straddles a bracket. Only a paragraph can be cut.
        let Block::Paragraph { parts, indent, .. } = block else {
            return None;
        };
        if hi > close {
            // This block already reaches past the `]`: everything after it
            // in document order does too.
            past_close = true;
        }
        let (inside, after) = split_parts(parts, document, open, close)?;
        if !inside.is_empty() {
            let mut b = block.clone();
            if let Block::Paragraph { parts, indent, .. } = &mut b {
                *parts = inside;
                // `\@parboxrestore` zeroes `\parindent` in the box.
                *indent = false;
            }
            top.push(b);
        }
        if !after.is_empty() {
            let mut b = block.clone();
            if let Block::Paragraph {
                parts,
                indent: ind,
                env_open,
                eject_before,
                vspace_before,
                addvspace_before,
                addvspace_flex,
                vspace_flex,
                endlist_adjust,
                ..
            } = &mut b
            {
                *parts = after;
                if !top.is_empty() {
                    // A fresh paragraph in vertical mode after the box.
                    *ind = true;
                    *env_open = None;
                    *eject_before = false;
                    *vspace_before = 0.0;
                    *addvspace_before = 0.0;
                    *addvspace_flex = (0.0, 0.0);
                    *vspace_flex = (0.0, 0.0);
                    *endlist_adjust = 0.0;
                } else {
                    *ind = *indent;
                }
            }
            keep.push(b);
        }
    }
    // `Context::box_blocks` sets a box's body, and drops page-level
    // material (a sectioning command, `longtable`) with a warning of its
    // own. Rather than lose it, refuse the whole split and leave the
    // argument where it was.
    if top.iter().any(|b| !matches!(b, Block::Paragraph { .. } | Block::Rule { .. } | Block::Picture { .. })) {
        return None;
    }
    // The box's vertical list starts in vertical mode, so an environment
    // that opens the material is a `\begin` read in vertical mode and
    // `\@topsepadd` keeps `\partopsep` -- at both ends, since the closing
    // `\@endparenv` reads the same flag. The source scan cannot see this:
    // what precedes the `\begin` there is `\twocolumn[`.
    if let Some(Block::Paragraph { env_open: Some(e), .. }) = top.first_mut() {
        e.vmode = true;
    }
    if top.is_empty() {
        // `\twocolumn[]`: the box is empty and `\@colht` loses nothing
        // (its height is `-\dbltextfloatsep`, which the `\vskip
        // \dbltextfloatsep` below it gives straight back). Measured:
        // identical to `\twocolumn` with no argument.
        *blocks = keep;
        return Some(Vec::new());
    }
    *blocks = keep;
    Some(top)
}

/// [`split_top_material`] for one paragraph's parts: `(what is inside the
/// brackets, what follows the `]`)`.
fn split_parts(parts: &[ParaPart], document: flashtex_compiler::DocumentId, open: usize, close: usize) -> Option<(Vec<ParaPart>, Vec<ParaPart>)> {
    let mut inside: Vec<ParaPart> = Vec::new();
    let mut after: Vec<ParaPart> = Vec::new();
    for part in parts {
        match part {
            ParaPart::Display { span, .. } | ParaPart::Rows { span, .. } => {
                if span.document != document {
                    return None;
                }
                if span.start > close {
                    after.push(part.clone());
                } else if span.start > open && span.end <= close {
                    inside.push(part.clone());
                } else {
                    return None;
                }
            }
            ParaPart::Lines(items) => {
                let (a, b) = split_items(items, document, open, close)?;
                if !a.is_empty() {
                    inside.push(ParaPart::Lines(a));
                }
                if !b.is_empty() {
                    after.push(ParaPart::Lines(b));
                }
            }
        }
    }
    Some((inside, after))
}

/// [`split_parts`] for one run of items. Words are cut character by
/// character, which is how the `[` and the `]` are dropped: the compiler
/// glues them to the words they touch (`[Short` .. `Line]`).
fn split_items(items: &[Item], document: flashtex_compiler::DocumentId, open: usize, close: usize) -> Option<(Vec<Item>, Vec<Item>)> {
    let mut inside: Vec<Item> = Vec::new();
    let mut after: Vec<Item> = Vec::new();
    for it in items {
        match it {
            Item::Word(w) => {
                let chars = || w.segments.iter().flat_map(|s| s.chars.iter());
                if w.segments.iter().any(|s| s.chars.len() != s.text.chars().count()) || chars().any(|c| c.document != document) {
                    return None;
                }
                if chars().any(|c| c.start < open) {
                    return None;
                }
                if let Some(a) = trim_word(w, document, open + 1, close) {
                    inside.push(Item::Word(a));
                }
                if let Some(b) = trim_word(w, document, close + 1, usize::MAX) {
                    after.push(Item::Word(b));
                }
            }
            other => match item_range(other, document) {
                Some((lo, _)) if lo > close => after.push(other.clone()),
                Some((_, hi)) if hi <= open => return None,
                Some(_) => inside.push(other.clone()),
                // No position of its own: interword glue, `\hfill`, a
                // `\label`. It belongs with what stands before it.
                None if after.is_empty() => inside.push(other.clone()),
                None => after.push(other.clone()),
            },
        }
    }
    // A paragraph neither ends nor starts with interword glue: the `\par`
    // that closes the box discards the one, `\@parboxrestore`'s new
    // paragraph the other.
    while matches!(inside.last(), Some(Item::Space { .. })) {
        inside.pop();
    }
    while matches!(after.first(), Some(Item::Space { .. })) {
        after.remove(0);
    }
    Some((inside, after))
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
                Inline::Marginpar { text, .. } => walk(text, out),
                Inline::Tabular(t) => {
                    for list in t.inline_lists() {
                        walk(list, out);
                    }
                }
                Inline::ColorBox(b) => walk(&b.content, out),
                Inline::Underline(u) => walk(&u.content, out),
                Inline::TextScript(t) => walk(&t.content, out),
                Inline::Phantom(p) => walk(&p.content, out),
                Inline::HBox(b) => walk(&b.content, out),
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
                for group in authors {
                    walk(group, &mut out);
                }
                walk(date.as_deref().unwrap_or(&[]), &mut out);
            }
            CBlock::BeamerFrameBegin { title, subtitle, .. } => {
                walk(title, &mut out);
                walk(subtitle, &mut out);
            }
            CBlock::BeamerBlockBegin { title: inlines, .. } | CBlock::BeamerCaption { content: inlines, .. } => walk(inlines, &mut out),
            CBlock::BeamerTitlePage { title, subtitle, authors, institute, date, .. } => {
                for part in [title, subtitle, authors, institute, date] {
                    walk(part, &mut out);
                }
            }
            _ => {}
        }
    }
    out
}

/// The source span of one adapter `Item`, for the items that carry one.
/// `None` is glue, penalties and whatsits with no position of their own
/// (`\label` included: it records a page, it sets nothing).
fn item_source_span(item: &Item) -> Option<Span> {
    match item {
        Item::Word(w) => Some(w.span()),
        Item::Math { span, .. }
        | Item::Logo { span, .. }
        | Item::Rule { span, .. }
        | Item::Footnote { span, .. }
        | Item::Marginpar { span, .. }
        | Item::QedBox { span, .. } => Some(*span),
        Item::Table(t) => Some(t.span),
        Item::ColorBox(b) => Some(b.span),
        Item::Underline(u) => Some(u.span),
        Item::TextScript(t) => Some(t.span),
        Item::HBox(b) => Some(b.span),
        Item::Lap { items } => {
            let mut spans = items.iter().filter_map(item_source_span);
            let first = spans.next()?;
            let last = spans.last().unwrap_or(first);
            (first.start <= last.end).then(|| Span::in_document(first.document, first.start, last.end))
        }
        _ => None,
    }
}

/// `(start, end)` of the entry-document source `block` sets, for the
/// blocks that set material: the range the single-switch scan compares
/// against the switch offset. `None` for page furniture (`Chrome`,
/// `NoBreakFalse`, `ClearPage`), contents entries (whose spans point at
/// the list sources rather than the laid-out position) and other
/// documents' material, all of which the scan passes over transparently.
fn block_source_range(block: &Block, entry: usize) -> Option<(usize, usize)> {
    let document = flashtex_compiler::DocumentId(entry);
    let in_entry = |s: Span| (s.document == document).then_some((s.start, s.end));
    match block {
        Block::Paragraph { parts, .. } => {
            let mut first: Option<Span> = None;
            let mut last: Option<Span> = None;
            for part in parts {
                match part {
                    ParaPart::Lines(items) => {
                        for s in items.iter().filter_map(item_source_span) {
                            if s.document == document {
                                first.get_or_insert(s);
                                last = Some(s);
                            }
                        }
                    }
                    ParaPart::Display { span, .. } | ParaPart::Rows { span, .. } => {
                        if span.document == document {
                            first.get_or_insert(*span);
                            last = Some(*span);
                        }
                    }
                }
            }
            Some((first?.start, last?.end))
        }
        Block::Heading { span, .. }
        | Block::Chapter { span, .. }
        | Block::Part { span, .. }
        | Block::Title { span, .. }
        | Block::Letter { span, .. }
        | Block::Rule { span, .. } => in_entry(*span),
        Block::Picture { document: d, picture, .. } => (*d == document).then_some((picture.start, picture.end)),
        Block::LongTable { table, .. } => in_entry(table.span),
        Block::TocEntry(_) | Block::ClearPage { .. } | Block::NoBreakFalse { .. } | Block::Chrome { .. } => None,
        // beamer's frame, block and columns furniture (no `\twocolumn` in a
        // deck): passed over like the page furniture above.
        Block::FrameBegin { .. }
        | Block::FrameEnd { .. }
        | Block::BeamerTitle { .. }
        | Block::BeamerToc { .. }
        | Block::BeamerBlockBegin { .. }
        | Block::BeamerBlockEnd { .. }
        | Block::ColumnsBegin { .. }
        | Block::Column { .. }
        | Block::ColumnsEnd { .. } => None,
    }
}

/// Whether `items` hold a `\marginpar` the page builder would place from
/// entry-document position `at` on, or from a position the scan cannot
/// compare (another document): either rules out laying out a mid-document
/// column switch, whose margin notes the single frame would misplace. A
/// note strictly before the switch rides the old frame either way, so it
/// constrains nothing.
fn marginpar_from(items: &[Item], entry: usize, at: usize) -> bool {
    items.iter().any(|item| match item {
        Item::Marginpar { span, .. } => span.document.0 != entry || span.start >= at,
        Item::Lap { items } => marginpar_from(items, entry, at),
        Item::ColorBox(b) => marginpar_from(&b.items, entry, at),
        Item::Underline(u) => marginpar_from(&u.items, entry, at),
        Item::TextScript(t) => marginpar_from(&t.items, entry, at),
        Item::HBox(b) => marginpar_from(&b.items, entry, at),
        Item::Footnote { text, .. } => text.as_ref().is_some_and(|t| marginpar_from(t, entry, at)),
        Item::Table(t) => t.entries.iter().any(|e| match e {
            crate::table::TableEntry::Row { cells, .. } => cells.iter().any(|c| marginpar_from(&c.items, entry, at)),
            _ => false,
        }),
        _ => false,
    })
}

/// Whether `block` holds such a `\marginpar`: in paragraph and heading
/// text, titles, contents lines and longtable cells. Displays are math;
/// pictures are graphics; neither can carry one.
fn block_marginpar_from(block: &Block, entry: usize, at: usize) -> bool {
    match block {
        Block::Paragraph { parts, .. } => parts.iter().any(|part| match part {
            ParaPart::Lines(items) => marginpar_from(items, entry, at),
            ParaPart::Display { .. } | ParaPart::Rows { .. } => false,
        }),
        Block::Heading { items, .. } | Block::Chapter { items, .. } | Block::Part { items, .. } => marginpar_from(items, entry, at),
        Block::Title { title, authors, date, .. } => {
            marginpar_from(title, entry, at)
                || authors.iter().flatten().flatten().any(|i| marginpar_from(std::slice::from_ref(i), entry, at))
                || date.as_ref().is_some_and(|d| marginpar_from(d, entry, at))
        }
        Block::TocEntry(e) => marginpar_from(&e.title, entry, at),
        Block::LongTable { table, .. } => table.entries.iter().any(|e| match e {
            crate::table::TableEntry::Row { cells, .. } => cells.iter().any(|c| marginpar_from(&c.items, entry, at)),
            _ => false,
        }),
        _ => false,
    }
}

/// The single-switch case slice 1 lays out: exactly one unmodelled switch
/// (so it actually changes the column count), bare (a `[...]` argument is
/// `twocolumn_top_material`'s case, explicitly out of scope), at a clean
/// block boundary (a block spanning the offset is a mid-paragraph switch,
/// which stays reported), opening a block the page builder already breaks
/// before (the switch's own `\clearpage`), in a document with no margin
/// note at or after it (those ride the frame being left).
///
/// Returns the first block after the switch. Anything else — a second
/// switch, a switch that changes nothing, an argument, a mid-block
/// offset, a missing page break or a later margin note — is `None`, and
/// the switch keeps its `twocolumn_mid_document` limitation.
fn column_switch_block(blocks: &[Block], source: &str, entry: usize, columns: &crate::columns::ColumnMode) -> Option<crate::columns::ColumnSwitch> {
    let unmodelled = columns.unmodelled();
    if unmodelled.len() != 1 {
        return None;
    }
    let (at, on) = unmodelled[0];
    let end = columns.spans().iter().find(|(s, _)| *s == at)?.1;
    if crate::columns::optional_bracket(source, end).is_some() {
        return None;
    }
    if blocks.iter().any(|b| block_marginpar_from(b, entry, at)) {
        return None;
    }
    let mut found = None;
    for (i, block) in blocks.iter().enumerate() {
        let Some((s, e)) = block_source_range(block, entry) else { continue };
        if s < at && at < e {
            return None;
        }
        if s >= at && found.is_none() {
            // `Chapter`/`Title` eject themselves (`\clearpage` is what the
            // commands are); every other kind carries the switch's own
            // `\clearpage` in `eject_before`.
            let ejects = match block {
                Block::Paragraph { eject_before, .. }
                | Block::Heading { eject_before, .. }
                | Block::Part { eject_before, .. }
                | Block::Rule { eject_before, .. }
                | Block::Letter { eject_before, .. }
                | Block::Picture { eject_before, .. }
                | Block::LongTable { eject_before, .. } => *eject_before,
                Block::Chapter { .. } | Block::Title { .. } => true,
                Block::TocEntry(_) | Block::ClearPage { .. } | Block::NoBreakFalse { .. } | Block::Chrome { .. } => false,
                Block::FrameBegin { .. }
                | Block::FrameEnd { .. }
                | Block::BeamerTitle { .. }
                | Block::BeamerToc { .. }
                | Block::BeamerBlockBegin { .. }
                | Block::BeamerBlockEnd { .. }
                | Block::ColumnsBegin { .. }
                | Block::Column { .. }
                | Block::ColumnsEnd { .. } => false,
            };
            if !ejects {
                return None;
            }
            found = Some(i);
        }
    }
    found.map(|block| crate::columns::ColumnSwitch { block, at, on })
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
            // `\twocolumn`/`\onecolumn` open with `\clearpage`, not
            // `\newpage`: in a two-column document they end the *page*, not
            // the column (measured — pdflatex puts the material after an
            // `\onecolumn` in a `[twocolumn]` article on a new page).
            let clear = text
                .rfind("\\clearpage")
                .max(text.rfind("\\cleardoublepage"))
                .max(text.rfind("\\twocolumn"))
                .max(text.rfind("\\onecolumn"));
            let column = text.rfind("\\newpage").max(text.rfind("\\pagebreak"));
            (clear.is_some() && clear > column).then_some(i)
        })
        .collect()
}

/// The first inline that sits at a real source position: a `\pagestyle` /
/// `\thispagestyle` marker rides in the paragraph with the command's own
/// span (for `\maketitle`, before the title's text), so it must not anchor
/// the paragraph for gap scans and page-break detection.
fn anchor_span<'a>(inlines: impl IntoIterator<Item = &'a Inline>) -> Option<Span> {
    inlines.into_iter().find(|i| !is_marker(i)).map(inline_span)
}

/// A zero-width marker riding in the paragraph with its command's own
/// span (`\pagestyle`, `\markboth`/`\markright`, beamer's overlay
/// markers): never the paragraph's first or last source position for gap
/// scans, and never the position the chrome fold lays the paragraph out
/// at -- a command's own event must reach the page the paragraph ships on.
fn is_marker(i: &Inline) -> bool {
    matches!(i, Inline::PageStyle { .. } | Inline::Mark { .. } | Inline::OverlayBegin { .. } | Inline::OverlayEnd { .. } | Inline::Onslide { .. })
}

fn inline_span(i: &Inline) -> Span {
    match i {
        Inline::Text { span, .. }
        | Inline::LineBreak { span, .. }
        | Inline::Math { span, .. }
        | Inline::MathRows { span, .. }
        | Inline::Label { span, .. }
        | Inline::PageStyle { span, .. }
        | Inline::Mark { span, .. }
        | Inline::Reference { span, .. }
        | Inline::CleverReference { span, .. }
        | Inline::HFill { span, .. }
        | Inline::HSpace { span, .. }
        | Inline::Footnote { span, .. }
        | Inline::Marginpar { span, .. }
        | Inline::Verbatim { span, .. }
        | Inline::TextGlue { span, .. }
        | Inline::Logo { span, .. }
        | Inline::Rule { span, .. }
        | Inline::Kern { span, .. } => *span,
        Inline::Tabular(t) => t.span,
        Inline::ColorBox(b) => b.span,
        Inline::Underline(u) => u.span,
        Inline::TextScript(t) => t.span,
        Inline::Phantom(p) => p.span,
        Inline::HBox(b) => b.span,
        Inline::Graphic(g) => g.span,
        Inline::Transform(t) => t.span,
        // Nodes only a re-pinned compiler emits; all of them carry the
        // command's own span, so the generic answer is already right and
        // the stacked PRs need not revisit this function.
        #[cfg(feature = "compiler-node-surface")]
        Inline::ThePage { span, .. }
        | Inline::PageNumbering { span, .. }
        | Inline::TabStop { span, .. }
        | Inline::TabJump { span, .. }
        | Inline::Marginpar { span, .. }
        | Inline::Penalty { span, .. }
        | Inline::PagePenalty { span, .. }
        | Inline::Discretionary { span, .. } => *span,
        Inline::OverlayBegin { span, .. } | Inline::OverlayEnd { span } | Inline::Onslide { span, .. } => *span,
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
        Inline::Marginpar { text, .. } => {
            // Set by `typeset::marginpar`; contexts it does not reach
            // are diagnosed there.
            for i in text {
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
            // suppressed (the compiler's `TextStyle::literal`), so
            // there is nothing to report. Measured against pdflatex at
            // 12 pt T1: `\verb"ftxc --version"` 86.4289 pt, the oracle's
            // 86.4289 pt. (`\lstinline` is then re-set the way listings
            // does it — the `basicstyle` face, one box per token, the
            // column bookkeeping — by `listings::apply`.)
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
    match inline {
        // amsmath `\eqref` to a rich `\tag` (#441): `\textup{\tagform@{..}}`
        // sets the tag's content again -- its text pieces upright, its
        // formulas at `\textstyle` -- so it is an inline formula holding
        // the content's `TextRun` between `\tagform@`'s parentheses (a
        // `\tag*` label gets them here too), every atom attributed to the
        // command's bytes.
        #[cfg(feature = "compiler-node-surface")]
        Inline::Reference { key, page: false, equation: true, span, space_before, glue_before, .. } if labels.rich_tags.contains_key(key) => {
            use flashtex_compiler::math::{MathAtom, Nucleus};
            let content = labels.rich_tags[key].0.clone();
            let mut list = MathList {
                atoms: vec![MathAtom {
                    nucleus: Nucleus::TextRun(tagform_pieces(content)),
                    span: *span,
                    superscript: None,
                    subscript: None,
                    class_override: None,
                    width_em: None,
                    ams_symbol: None,
                    limits: None,
                }],
            };
            crate::incremental::respan_math(&mut list, *span);
            reference_spans.push(*span);
            out.push(std::borrow::Cow::Owned(Inline::Math {
                list,
                display: false,
                number: None,
                number_span: None,
                span: *span,
                space_before: *space_before,
                color: None,
                size: None,
                color_ranges: Vec::new(),
                glue_before: *glue_before,
            }));
        }
        Inline::Reference { key, page, equation, span, style, glue_before, .. } => {
            let text = if *page {
                labels.pages.get(key).map(|p| p.to_string())
            } else {
                labels.values.get(key).cloned()
            }
            .unwrap_or_else(|| "??".to_string());
            // amsmath `\eqref`: the value in parentheses (compiler's flag).
            let text = if *equation { format!("({text})") } else { text };
            reference_spans.push(*span);
            // The value is set in the font in force at the command. amsmath's
            // `\eqref` is `\textup{\tagform@{..}}`: `\textup`'s `\check@icl`
            // runs in an upright font before a macro, so it always puts the
            // italic correction of the character in front (`see \eqref`).
            let mut style = *style;
            style.italic_correction.before = *equation;
            out.push(std::borrow::Cow::Owned(Inline::Text {
                text,
                span: *span,
                style,
                // Interword gaps are read from the source bytes between
                // spans here, never from the compiler's flag.
                space_before: true,
                boundary_before: false,
                glue_before: *glue_before,
            }));
        }
        Inline::CleverReference { keys, page, range, label_only, capitalise, span, style, glue_before, .. } => {
            let text = clever_reference_text(keys, labels, *page, *range, *label_only, *capitalise);
            reference_spans.push(*span);
            out.push(std::borrow::Cow::Owned(Inline::Text {
                text,
                span: *span,
                style: *style,
                // As for `Reference`: the gap comes from the source bytes
                // between spans, not the compiler's flag.
                space_before: true,
                boundary_before: false,
                glue_before: *glue_before,
            }));
        }
        Inline::Verbatim { text, span, space_before, style, glue_before } => {
            reference_spans.push(*span);
            out.push(std::borrow::Cow::Owned(Inline::Text {
                text: text.clone(),
                span: *span,
                style: *style,
                space_before: *space_before,
                boundary_before: false,
                glue_before: *glue_before,
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

/// `\@beginparpenalty`, `\@itempenalty` and `\@endparpenalty`: each is
/// `-\@lowpenalty` (latex.ltx), and `\@lowpenalty` is 51 in article,
/// report and book (`\@lowpenalty 51`). A list offers the page builder a
/// slightly favoured break before its first item, between its items and
/// after it: pdflatex ends a page between two `\item`s where the break
/// inside the next item costs no more than 51 less.
const LIST_PENALTY: i32 = -51;

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
    /// See [`Block::Paragraph::penalty_before`].
    penalty_before: Option<i32>,
    /// Constructs before this unit the pipeline set approximately.
    limitations: Vec<(&'static str, Span, String)>,
}

enum UnitKind<'p> {
    Heading {
        level: u8,
        number: &'p str,
        number_span: Span,
        content: &'p [Inline],
        /// The size the title selected ([`ParLeading`]).
        leading: ParLeading,
        /// `\@startsection`'s `#6` around the whole head
        /// (compiler `Block::Heading::style`): the number is inside it.
        head_style: &'p CTextStyle,
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
        /// The explicit `\item[<label>]` content, converted into
        /// [`ListGeom::label_items`] once the styles are at hand.
        label_inlines: Option<&'p [Inline]>,
        /// The paragraph opens with a run-in heading (`\paragraph`,
        /// `\subparagraph`, or a class's own `\@startsection` with a
        /// non-positive `#5`), at this level: the compiler's
        /// [`ParStart::run_in`](flashtex_compiler::parser::ParStart::run_in).
        /// The head itself is already the paragraph's first inlines; the
        /// level is here only for `\@startsection`'s `\addvspace{#4}`.
        run_in: Option<u8>,
        /// The leading this paragraph's `\par` selected ([`ParLeading`]).
        par_leading: ParLeading,
        /// A `\hangfrom{label}` opening the paragraph ([`hangfrom_label`]):
        /// the label's leading inline run and the command's span, whose
        /// diagnostic is superseded once the hang is set. `None` for every
        /// other paragraph, including later segments of a hangfrom one.
        hang_label: Option<(&'p [Inline], Span)>,
    },
    Rule {
        span: Span,
    },
    /// A letter.cls block ([`Block::Letter`]): a compiler `LetterBlock`, or
    /// the `\cc`/`\encl` paragraph (see [`letter_annotation`]).
    Letter {
        kind: LetterKind,
        lines: Vec<&'p [Inline]>,
        extra_gap_after_pt: Vec<f64>,
        gap_before_pt: f64,
        gap_after_pt: f64,
        indent_pt: f64,
        span: Span,
    },
    /// beamer `\begin{frame}` / `\end{frame}` (#944).
    FrameBegin {
        block: &'p CBlock,
        span: Span,
    },
    FrameEnd {
        span: Span,
    },
    /// beamer `\titlepage`.
    BeamerTitle {
        block: &'p CBlock,
        span: Span,
    },
    /// beamer `\tableofcontents[options]` in a frame.
    BeamerToc {
        span: Span,
        options: &'p str,
    },
    /// beamer Tier 3 (#944): block edges, column markers, captions.
    BeamerBlockBegin {
        block: &'p CBlock,
        span: Span,
    },
    BeamerBlockEnd {
        span: Span,
    },
    ColumnsBegin {
        block: &'p CBlock,
        span: Span,
    },
    Column {
        block: &'p CBlock,
        span: Span,
    },
    ColumnsEnd {
        span: Span,
    },
    BeamerCaption {
        block: &'p CBlock,
        span: Span,
    },
    Picture {
        document: flashtex_compiler::DocumentId,
        picture: flashtex_vector_graphics::tikz::PictureSource,
        centered: bool,
        /// The picture opens its paragraph (every inline before its
        /// segment is paragraph-leading whitespace): only then does the
        /// paragraph's `\parindent` apply. A picture after text in the
        /// same paragraph is set on a line of its own at the margin, as
        /// before.
        initial: bool,
        /// LaTeX's `\@endpe`: a picture that follows `\end{center}`/...
        /// without a blank line continues in the same paragraph,
        /// unindented -- the same signal paragraphs carry.
        after_env: bool,
        /// `\noindent` before the paragraph the picture opens (the
        /// compiler's `ParStart::indent`).
        noindent: bool,
        /// The picture is the `\item` of an amsthm theorem-like
        /// environment: not indented, as a paragraph would not be.
        theorem_item: bool,
        /// A compiler `ListItem` picture: its `\list` geometry, for the
        /// hanging indent instead of `\parindent`.
        list: Option<ListGeom>,
        /// The picture is a figure caption: never indented.
        caption: bool,
        /// A compiler `Styled` picture (`center`, `quote`, ...): never
        /// `\parindent`-indented (`center` is centred instead).
        styled: Option<ParaStyle>,
    },
}

/// `\twocolumn` and `\onecolumn` both open with `\clearpage` (latex.ltx
/// 20256-20275), so both end the page: measured against pdflatex, a
/// `\twocolumn` after a paragraph puts the following text on a new page,
/// and so does an `\onecolumn` in a `[twocolumn]` document.
const PAGE_BREAKS: [&str; 5] = ["newpage", "clearpage", "pagebreak", "twocolumn", "onecolumn"];

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

fn split_at_page_breaks<'p>(
    par_starts: &ParStarts,
    texts: &[&str],
    blocks: &'p [(CBlock, ParLeading)],
    size: u32,
    style: &Stylesheet,
) -> Vec<Unit<'p>> {
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
    // The compiler's list frames of the previous `\item` unit.
    let mut prev_frames: Vec<ListFrame> = Vec::new();
    // The compiler's list frames of the last `\item` unit, whatever came
    // after it: a labelled item whose innermost list is not among them
    // opens that list.
    let mut item_frames: Vec<ListFrame> = Vec::new();
    // Whether each open list's `\begin` was read in vertical mode, by its
    // `begin_span` (`\@topsepadd` keeps `\partopsep` for the closing skip
    // too): the compiler's [`ListFrame::vmode`], or a previous unit that
    // left TeX in vertical mode.
    let mut list_vmode: Vec<(Span, bool)> = Vec::new();
    // `tikzpicture` environments per document, and those already emitted.
    let pictures: Vec<Vec<flashtex_vector_graphics::tikz::PictureSource>> = texts.iter().map(|t| flashtex_vector_graphics::tikz::find_pictures(t)).collect();
    let mut emitted_pictures: std::collections::BTreeSet<(usize, usize)> = std::collections::BTreeSet::new();
    // List and theorem nesting per document, read at each block's offset.
    let indexes = SourceIndexes::new(texts, &theorem_envs);
    for (block, par_leading) in blocks {
        let par_leading = *par_leading;
        match block {
            CBlock::PageBreak => {
                pending_eject = true;
                continue;
            }
            CBlock::VSpace { pt, .. } => {
                pending_vspace += pt;
                continue;
            }
            CBlock::TableOfContents { span, options, .. } => {
                // beamer: the contents are a frame's material of their
                // own (`Block::BeamerToc`), not a spliced contents list.
                if style.is_beamer() {
                    units.push(Unit {
                        kind: UnitKind::BeamerToc { span: *span, options: options.as_str() },
                        eject_before: false,
                        vspace_before: std::mem::take(&mut pending_vspace),
                        addvspace_before: 0.0,
                        addvspace_flex: (0.0, 0.0),
                        vspace_flex: (0.0, 0.0),
                        endlist_adjust: 0.0,
                        penalty_before: None,
                        limitations: std::mem::take(&mut pending_limitations),
                    });
                }
                prev_end = Some(*span);
                prev_vmode = true;
                continue;
            }
            // A paragraph holding only `\pagestyle`/`\thispagestyle`
            // markers (compiler pin `75a2a03a`, fancyhdr #849; the old pin
            // read the command's argument and emitted nothing). They are
            // whatsits, no material -- TeX stays in vertical mode -- so it
            // is no block here: the page style is read from the source by
            // [`body_commands`], and `prev_end`/`prev_vmode` keep telling
            // the next block what really precedes it. A preamble
            // `\pagestyle{empty}` ahead of the document's first list made
            // that list's `\begin` look like it was read in horizontal
            // mode, which dropped `\partopsep` from its closing
            // `\@topsepadd` (`nested_list_end_skips`).
            CBlock::Paragraph(inlines) if !inlines.is_empty() && inlines.iter().all(|i| matches!(i, Inline::PageStyle { .. } | Inline::Mark { .. })) => continue,
            // letter.cls's positioned blocks: vertical-mode material of
            // their own. The class's `\vspace`s around them are already in
            // the block (`gap_before_pt`/`gap_after_pt`), so the gap scan
            // below is not run for them; a `\vspace` block the compiler
            // emitted before one still arrives through `pending_vspace`.
            CBlock::LetterBlock { part, lines, extra_gap_after_pt, gap_before_pt, gap_after_pt, indent_pt, span } => {
                units.push(Unit {
                    kind: UnitKind::Letter {
                        kind: match part {
                            flashtex_compiler::parser::LetterPart::ReturnAddress => LetterKind::ReturnAddress,
                            flashtex_compiler::parser::LetterPart::Recipient => LetterKind::Recipient,
                            flashtex_compiler::parser::LetterPart::Closing => LetterKind::Closing,
                        },
                        lines: lines.iter().map(Vec::as_slice).collect(),
                        extra_gap_after_pt: extra_gap_after_pt.clone(),
                        gap_before_pt: *gap_before_pt,
                        gap_after_pt: *gap_after_pt,
                        indent_pt: *indent_pt,
                        span: *span,
                    },
                    eject_before: std::mem::take(&mut pending_eject),
                    vspace_before: std::mem::take(&mut pending_vspace),
                    addvspace_before: 0.0,
                    addvspace_flex: (0.0, 0.0),
                    vspace_flex: (0.0, 0.0),
                    endlist_adjust: 0.0,
                    penalty_before: None,
                    limitations: std::mem::take(&mut pending_limitations),
                });
                prev_end = Some(*span);
                prev_vmode = true;
                continue;
            }
            CBlock::Paragraph(inlines) if style.is_letter() && letter_annotation(inlines).is_some() => {
                let (label, text, span) = letter_annotation(inlines).expect("checked above");
                units.push(Unit {
                    kind: UnitKind::Letter {
                        kind: LetterKind::Annotation,
                        lines: vec![label, text],
                        extra_gap_after_pt: Vec::new(),
                        gap_before_pt: 0.0,
                        gap_after_pt: 0.0,
                        indent_pt: 0.0,
                        span,
                    },
                    eject_before: std::mem::take(&mut pending_eject),
                    vspace_before: std::mem::take(&mut pending_vspace),
                    addvspace_before: 0.0,
                    addvspace_flex: (0.0, 0.0),
                    vspace_flex: (0.0, 0.0),
                    endlist_adjust: 0.0,
                    penalty_before: None,
                    limitations: std::mem::take(&mut pending_limitations),
                });
                prev_end = Some(span);
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
                    penalty_before: None,
                    limitations: std::mem::take(&mut pending_limitations),
                });
                prev_end = Some(*span);
                prev_vmode = true;
                continue;
            }
            // beamer frame edges and the title page: vertical-mode material
            // of their own (the page break is the frame's, not a
            // `\newpage` in the gap). The `\end{frame}` closes any list
            // still open, so the next block starts fresh.
            // beamer's frame head and title page: vertical-mode material of
            // their own (the page break is the frame's, not a `\newpage` in
            // the gap). `\end{frame}` takes the ordinary path below so the
            // list it closes gets its `\@endparenv` skip.
            CBlock::BeamerFrameBegin { span, .. } | CBlock::BeamerTitlePage { span, .. } => {
                let kind = match block {
                    CBlock::BeamerFrameBegin { .. } => UnitKind::FrameBegin { block, span: *span },
                    _ => UnitKind::BeamerTitle { block, span: *span },
                };
                pending_eject = false;
                units.push(Unit {
                    kind,
                    eject_before: false,
                    vspace_before: std::mem::take(&mut pending_vspace),
                    addvspace_before: 0.0,
                    addvspace_flex: (0.0, 0.0),
                    vspace_flex: (0.0, 0.0),
                    endlist_adjust: 0.0,
                    penalty_before: None,
                    limitations: std::mem::take(&mut pending_limitations),
                });
                prev_end = Some(*span);
                // Not a heading: the first list of the body takes its
                // `\@topsep` (no `\@nbitem` absorption).
                prev_vmode = false;
                prev_styled = false;
                prev_list = false;
                prev_frames = Vec::new();
                item_frames = Vec::new();
                list_vmode.clear();
                continue;
            }
            _ => {}
        }
        // An empty-body `\item` has no inlines: its `\item` command's own
        // span is the block's material, so the gap bookkeeping below (and
        // `prev_end` for the block after it) is measured from there rather
        // than from before the list, which would make the *next* item read
        // the list's `\begin` and open it a second time.
        let item_label_span = match block {
            CBlock::ListItem { label: Some((_, span)), .. } => Some(*span),
            _ => None,
        };
        let first = match block {
            CBlock::Heading { number_span, .. } => Some(*number_span),
            CBlock::BeamerFrameEnd { span } => Some(*span),
            CBlock::BeamerBlockBegin { span, .. }
            | CBlock::BeamerBlockEnd { span }
            | CBlock::BeamerColumnsBegin { span, .. }
            | CBlock::BeamerColumn { span, .. }
            | CBlock::BeamerColumnsEnd { span }
            | CBlock::BeamerCaption { span, .. } => Some(*span),
            _ => anchor_span(inlines_of(block)).or(item_label_span),
        };
        let mut eject = std::mem::take(&mut pending_eject) || matches!((prev_end, first), (Some(p), Some(f)) if gap_has_page_break(texts, p, f));
        // `\vspace`'s `em`/`ex` are the compiler's, in the font where the
        // command stands (PLAN1 site 30).
        let mut vspace_before = std::mem::take(&mut pending_vspace);
        // The compiler's list model (pin `42557b09`): every `\item`
        // paragraph is a `ListItem` with its nesting level and, for the
        // item's first paragraph, the marker text. Its `\setlist`
        // itemsep/topsep gaps are attached to whichever paragraph the
        // *next* `\item`/`\end` flushes, so an item holding a display
        // (which ends the paragraph early) carries them on the wrong
        // block; the pipeline sets the list's vertical glue from each
        // item's `ListItem.lists` frames instead (the enumitem keys in
        // force, parsed, and whether the `\begin` was read in vertical
        // mode):
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
        // (`list_margins`, from the same frames).
        let is_heading = matches!(block, CBlock::Heading { .. });
        let mut addvspace_before = 0.0;
        let mut addvspace_flex = (0.0f64, 0.0f64);
        let mut vspace_flex = (0.0f64, 0.0f64);
        let mut endlist_adjust = 0.0;
        let mut penalty_before: Option<i32> = None;
        // `\endtrivlist` of the lists closed in the gap: each is
        // `\addvspace\@topsepadd` with its own level's `\topsep` (plus
        // `\partopsep` when that list opened in vertical mode), and
        // successive `\addvspace`s keep the larger natural skip — so does a
        // following `\item`'s `\addvspace\itemsep`. (natural, stretch, shrink)
        let mut list_end_skip: Option<(f64, f64, f64)> = None;
        // The lists closed since the previous unit, innermost first: the
        // compiler's frames the previous `\item` sat in and this block does
        // not (PLAN1 slice 2; an `\end` from a macro body counts too).
        let frames: &[ListFrame] = match block {
            CBlock::ListItem { lists, .. } => lists,
            _ => &[],
        };
        let common = prev_frames.iter().zip(frames).take_while(|(a, b)| a.begin_span == b.begin_span).count();
        // Innermost first.
        let closed: Vec<&ListFrame> = prev_frames[common.min(prev_frames.len())..].iter().rev().filter(|f| modelled_list(f)).collect();
        if prev_list && !is_heading && !closed.is_empty() {
            let open = modelled_lists(&prev_frames);
            let topsepadd = |seps: &ListSeps, vmode: bool| {
                let p = if vmode { seps.partopsep_skip } else { crate::style::Skip::default() };
                (seps.topsep + p.natural, seps.topsep_skip.stretch + p.stretch, seps.topsep_skip.shrink + p.shrink)
            };
            for (k, frame) in closed.iter().enumerate() {
                let depth = open.len() - k;
                let seps = list_seps_of(&frame.options, depth, size, style);
                let vmode = list_vmode.iter().rev().find(|(at, _)| *at == frame.begin_span).map_or(frame.vmode, |(_, v)| *v);
                let skip = topsepadd(&seps, vmode);
                list_end_skip = Some(match list_end_skip {
                    Some(kept) if kept.0 >= skip.0 => kept,
                    _ => skip,
                });
            }
            endlist_adjust = list_end_adjust(&open, closed.len(), size, style);
            // `\@endparenv`: `\addpenalty\@endparpenalty` before
            // its `\addvspace\@topsepadd`.
            if !style.is_beamer() {
                penalty_before = Some(LIST_PENALTY);
            }
        }
        let mut list = None;
        let mut label_inlines: Option<&'p [Inline]> = None;
        if let CBlock::ListItem { level, label, item, lists, .. } = block {
            let anchor = label.as_ref().map(|(_, span)| *span).or(first);
            if let Some(at) = anchor {
                let index = indexes.get(at.document.0);
                // The compiler's frames of the lists this item sits in
                // (PLAN1 slice 3: a `\begin` from a macro body counts too).
                let stack = modelled_lists(lists);
                let innermost = stack.last().copied();
                let env = innermost.map_or("enumerate", |f| f.environment.name());
                let seps = list_seps_of(innermost.map_or(&[][..], |f| &f.options), stack.len().max(1), size, style);
                // `\@outerparskip`: the `\parskip` in force when `\begin`
                // was read — the enclosing list's `\parsep` when nested.
                let outer_parskip_skip = match stack.len() {
                    n if n > 1 => list_seps_of(&stack[n - 2].options, n - 1, size, style).parsep_skip,
                    _ => style.parskip,
                };
                let outer_parskip = outer_parskip_skip.natural;
                if label.is_some() {
                    // The item opens its list when the last `\item` was not
                    // in it.
                    let opens = innermost.filter(|f| !item_frames.iter().any(|g| g.begin_span == f.begin_span));
                    match opens {
                        Some(frame) => {
                            // `\@trivlist`'s `\ifvmode` at the `\begin`: after
                            // a `\par`, a blank line, a heading, or the
                            // `\par` of another `\trivlist`'s `\end`
                            // (`\@endparenv`) -- that is every list, but also
                            // `center`, `quote`, `quotation`, `verse` and a
                            // theorem -- as the compiler read it.
                            let vmode = frame.vmode || prev_vmode || prev_end.is_none();
                            list_vmode.retain(|(at, _)| lists.iter().any(|f| f.begin_span == *at));
                            list_vmode.push((frame.begin_span, vmode));
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
                                // `\addpenalty\@beginparpenalty` (the
                                // `\@nbitem` branch above has none).
                                if !style.is_beamer() {
                                    penalty_before = Some(LIST_PENALTY);
                                }
                                let p = if vmode { seps.partopsep_skip } else { crate::style::Skip::default() };
                                let open = (
                                    seps.topsep + outer_parskip + p.natural,
                                    seps.topsep_skip.stretch + outer_parskip_skip.stretch + p.stretch,
                                    seps.topsep_skip.shrink + outer_parskip_skip.shrink + p.shrink,
                                );
                                // Both are `\addvspace`: the `\@topsepadd` the
                                // closing list left behind and this `\list`'s
                                // own `\addvspace\@topsep` keep the larger
                                // natural skip, they are not summed -- except
                                // under beamer, whose `\beamer@enum@`/`itemize`
                                // put `\usebeamercolor[fg]{...}`'s colour
                                // whatsit between the two, so `\lastskip` is
                                // 0 at the second `\addvspace` and both
                                // skips land (measured: adjacent lists
                                // 19.53bp apart = 13.6pt + 3pt + 3pt).
                                let skip = match list_end_skip.take() {
                                    Some(end) if style.is_beamer() => (end.0 + open.0, end.1 + open.1, end.2 + open.2),
                                    Some(end) if end.0 >= open.0 => end,
                                    _ => open,
                                };
                                addvspace_before += skip.0;
                                addvspace_flex.0 += skip.1;
                                addvspace_flex.1 += skip.2;
                                vspace_before -= seps.parsep;
                                vspace_flex.0 -= seps.parsep_skip.stretch;
                                vspace_flex.1 -= seps.parsep_skip.shrink;
                            }
                        }
                        None => {
                            // `\addpenalty\@itempenalty`, then `\addvspace\itemsep`.
                            if !style.is_beamer() {
                                penalty_before = Some(LIST_PENALTY);
                            }
                            let itemsep = (seps.itemsep, seps.itemsep_skip.stretch, seps.itemsep_skip.shrink);
                            let skip = match list_end_skip.take() {
                                Some(end) if end.0 >= itemsep.0 => end,
                                _ => itemsep,
                            };
                            addvspace_before += skip.0;
                            addvspace_flex.0 += skip.1;
                            addvspace_flex.1 += skip.2;
                        }
                    }
                }
                // natbib's author-year `thebibliography`, and only when the
                // compiler really did drop the entry's marker (`\@biblabel`
                // is `\hfill`): a build whose compiler still numbers the
                // entries keeps the class's label-width geometry, so this
                // never draws a `[1]` on top of the hanging indent.
                let natbib_bib = env == "thebibliography"
                    && index.natbib_author_year
                    && label.as_ref().is_none_or(|(text, _)| text.is_empty());
                let (margins, labelsep_pt, itemindent_pt) = list_margins(index, &stack, texts.get(at.document.0).copied().unwrap_or(""), at.start, size, natbib_bib, style);
                // The explicit label's inlines; `adapt_cached` converts
                // them to items (the styles and label table live there).
                label_inlines = match item {
                    Some(ItemLabel::Explicit { content, .. }) if label.is_some() && !content.is_empty() => Some(content.as_slice()),
                    _ => None,
                };
                // GH-924: an item whose text holds a box argument (`\uline`,
                // `\sout`, `\underline`, `\colorbox`) reaches here with its
                // label text but no `item`: the compiler's `box_inlines`
                // restores `pending_item_label` around the nested parse but
                // not `pending_item`, which the box's own paragraph flush
                // consumes. A labelled itemize item never has any other
                // `item` than article's `\labelitem<i>` symbol, so read it
                // back from the text: otherwise the bullet is set as a
                // Latin Modern Roman word (0.7778 em) instead of `tcrm`'s
                // 0.5 em symbol, 2.77 bp too far left.
                let (label_symbol, label_bold) = match item {
                    Some(ItemLabel::Symbol { bold, .. }) => (true, *bold),
                    None if env == "itemize" => match label.as_ref().map(|(text, _)| text.as_str()) {
                        Some("•" | "∗" | "⋅") => (true, false),
                        Some("–") => (true, true),
                        _ => (false, false),
                    },
                    _ => (false, false),
                };
                list = Some(ListGeom {
                    level: *level,
                    margins,
                    label: label.clone(),
                    label_items: None,
                    description: env == "description",
                    // enumitem's `style=nextline`: the label takes a line of
                    // its own.
                    nextline: innermost.and_then(ListFrame::style).is_some_and(|v| v.trim() == "nextline"),
                    label_symbol,
                    label_bold,
                    llap: matches!(env, "itemize" | "enumerate"),
                    parsep: seps.parsep_skip,
                    // `\NAT@bibsetup`: `\itemindent-\leftmargin`, so the
                    // entry's first line is flush at the margin and the rest
                    // of the entry hangs `\bibhang` in.
                    itemindent_em: if natbib_bib { -1.0 } else { 0.0 },
                    labelsep_pt,
                    itemindent_pt,
                    hidden: false,
                    unpainted: false,
                    alerted: false,
                    bibliography: env == "thebibliography",
                });
            }
        }
        if let Some(skip) = list_end_skip {
            addvspace_before += skip.0;
            addvspace_flex.0 += skip.1;
            addvspace_flex.1 += skip.2;
        }
        let closed_list = prev_list;
        prev_list = list.is_some();
        prev_frames = frames.to_vec();
        if list.is_some() {
            item_frames = frames.to_vec();
        }
        let limitations = std::mem::take(&mut pending_limitations);
        let styled = match block {
            CBlock::Styled { style, .. } => Some(ParaStyle::of(*style)),
            _ => None,
        };
        // The environment opens here when the compiler saw its `\begin`
        // (from the source or a macro body) since the previous block;
        // `\partopsep` applies when that `\begin` was read in vertical mode
        // (`TrivlistStart::vmode`: nothing before it, a blank line / `\par`,
        // a heading, or the `\par` of an `\endtrivlist` or a theorem's end).
        let env_open = styled
            .and_then(|_| par_starts.of(inlines_of(block))?.trivlist)
            .map(|t| EnvOpen { vmode: t.vmode, skips: None });
        // `\@endpe`: a plain paragraph right after `\end{...}` (no blank line
        // or `\par` between them) is not indented. A list's `\endtrivlist`
        // is `\@endparenv` too. The compiler reads it, macro-expanded
        // `\end`s and `\par`s included (`ParStart::indent`).
        let after_env = styled.is_none() && (prev_styled || closed_list) && par_starts.of(inlines_of(block)).is_some_and(|s| !s.indent);
        // The `\item` of an amsthm theorem-like environment: the gap before
        // this block holds its `\begin{...}` (only the environment's first
        // paragraph, so later ones keep the ambient `\parindent`).
        // `Some(is_proof)` when this block is the `\item` that opens a
        // theorem-like environment; `proof` is told apart because its closing
        // `\@topsepadd` is not `\topsep` (see [`theorem_skips`]).
        //
        // A theorem-like environment nested in a list item is its own
        // `\trivlist`, so its first paragraph takes this path too even
        // though the compiler reports it as a `ListItem`: a nested proof's
        // opening `\@topsep` and its head's compiler-scoped italic both
        // ride on `theorem_item`/`in_theorem`, and without them the head
        // sets upright and the boundary loses the skip (GH-897). Only the
        // label-less continuation paragraphs qualify: a labelled `\item`
        // paragraph opens the enclosing list, whose own `\@topsep`/
        // `\itemsep` path above already accounts for the boundary.
        let theorem_open: Option<bool> = (styled.is_none() && list.as_ref().is_none_or(|l| l.label.is_none()))
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
            || first.is_some_and(|f| texts.get(f.document.0).is_some_and(|t| indexes.get(f.document.0).in_theorem(t.is_char_boundary(f.start), f.start)));
        // `\paragraph{...}`/`\subparagraph{...}`: the compiler emits the
        // head as this paragraph's first inlines and names its level on the
        // paragraph's `ParStart`, so nothing is read back from the source.
        let mut run_in = par_starts.of(inlines_of(block)).and_then(|start| start.run_in);
        prev_styled = styled.is_some();
        // A paragraph of nothing but `\label`s is a `\write` whatsit on the
        // vertical list (`\label` in vertical mode is `\@bsphack` +
        // `\protected@write`, no `\leavevmode`): TeX stays in vertical mode
        // and `\@afterheading`'s `\@nobreaktrue` survives it, so a list
        // right after `\section{..}\label{..}` still opens with `\@nbitem`,
        // not `\addvspace\@topsep` -- pdflatex's `\showlists` of both
        // (11pt article, `\topsep` 9pt + `\partopsep` 3pt): the heading's
        // `\glue 10.84085 plus 0.94266`, then the `\write`, then `\glue
        // -4.5 plus -1.0 minus -1.0` and the item's `\parskip` 4.5pt, the
        // same 10.84085pt as without the label. Treating the label as a
        // paragraph took `\topsep + \partopsep` = 12pt instead, 1.159pt
        // too low (hyperref-toc page 4, every word).
        let label_only = matches!(block, CBlock::Paragraph(inlines)
            if !inlines.is_empty() && inlines.iter().all(|i| matches!(i, Inline::Label { .. })));
        prev_vmode = matches!(block, CBlock::Heading { .. }) || (label_only && prev_vmode);
        match block {
            CBlock::Heading {
                level,
                number,
                number_span,
                content,
                style,
            } => {
                units.push(Unit {
                    kind: UnitKind::Heading {
                        level: *level,
                        number,
                        number_span: *number_span,
                        content,
                        leading: par_leading,
                        head_style: style,
                    },
                    eject_before: eject,
                    vspace_before,
                    addvspace_before,
                    addvspace_flex,
                    vspace_flex,
                    endlist_adjust: 0.0,
                    penalty_before: None,
                    limitations,
                });
            }
            CBlock::Paragraph(inlines) | CBlock::ListItem { content: inlines, .. } | CBlock::FigureCaption { content: inlines } | CBlock::Styled { content: inlines, .. } => {
                let caption = matches!(block, CBlock::FigureCaption { .. });
                // `\hangfrom{label}` opens this paragraph: recover the
                // label's leading inline run for the hang (see
                // `hangfrom_label`). Only a plain paragraph: inside
                // `\item` (or `quote`, which is a `\list`) TeX's
                // `\parshape` wins over `\hangindent`, and a caption is
                // its own box. The first pushed unit takes it; later
                // segments get none.
                let mut hang_label: Option<(&[Inline], Span)> =
                    if matches!(block, CBlock::Paragraph(_)) {
                        let lo = prev_end
                            .filter(|s| {
                                inlines.first().is_some_and(|i| inline_span(i).document == s.document)
                            })
                            .map_or(0, |s| s.end);
                        hangfrom_label(texts, inlines, lo)
                    } else {
                        None
                    };
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
                            // Paragraph-initial means no material before the
                            // picture in this block: TeX skips whitespace at
                            // a paragraph's start, so whitespace-only runs
                            // (source indentation of `\begin{tikzpicture}`)
                            // do not count. Anything else before it -- text,
                            // a `\label`, glue -- means the picture continues
                            // the paragraph and takes no `\parindent`.
                            let initial = inlines[..seg_start]
                                .iter()
                                .all(|i| matches!(i, Inline::Text { text, .. } if text.trim().is_empty()));
                            // `run_in` stays with the paragraph unit: a
                            // `\paragraph` head is body text ahead of the
                            // picture, which carries it.
                            units.push(Unit {
                                kind: UnitKind::Picture {
                                    document,
                                    picture: pictures[document.0][k].clone(),
                                    centered,
                                    initial,
                                    after_env,
                                    noindent: initial && par_starts.of(inlines).is_some_and(|s| !s.indent),
                                    // Only the environment's first unit
                                    // carries the `\item`, as for paragraphs.
                                    theorem_item: std::mem::take(&mut theorem_item),
                                    list: list.clone(),
                                    caption,
                                    styled,
                                },
                                eject_before: eject,
                                vspace_before: std::mem::take(&mut vspace_before),
                                addvspace_before: std::mem::take(&mut addvspace_before),
                                addvspace_flex: std::mem::take(&mut addvspace_flex),
                                vspace_flex: std::mem::take(&mut vspace_flex),
                                endlist_adjust: std::mem::take(&mut endlist_adjust),
                                penalty_before: penalty_before.take(),
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
                                    label_inlines,
                                    run_in: std::mem::take(&mut run_in),
                                    par_leading,
                                    hang_label: hang_label.take(),
                                },
                                eject_before: eject,
                                vspace_before: std::mem::take(&mut vspace_before),
                                addvspace_before: std::mem::take(&mut addvspace_before),
                                addvspace_flex: std::mem::take(&mut addvspace_flex),
                                vspace_flex: std::mem::take(&mut vspace_flex),
                                endlist_adjust: std::mem::take(&mut endlist_adjust),
                                penalty_before: penalty_before.take(),
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
                            label_inlines,
                            run_in: std::mem::take(&mut run_in),
                            par_leading,
                            hang_label: hang_label.take(),
                        },
                        eject_before: eject,
                        vspace_before: std::mem::take(&mut vspace_before),
                        addvspace_before: std::mem::take(&mut addvspace_before),
                        addvspace_flex: std::mem::take(&mut addvspace_flex),
                        vspace_flex: std::mem::take(&mut vspace_flex),
                        endlist_adjust: std::mem::take(&mut endlist_adjust),
                        penalty_before: penalty_before.take(),
                        limitations: std::mem::take(&mut limitations),
                    });
                    eject = false;
                }
            }
            CBlock::BeamerFrameEnd { span } => {
                // The `\@endparenv` skip of a list the frame closes
                // (`addvspace_before`) becomes glue after the body's last
                // block (`typeset::beamer`); a `\newpage` in the gap is
                // the frame's own break.
                units.push(Unit {
                    kind: UnitKind::FrameEnd { span: *span },
                    eject_before: false,
                    vspace_before,
                    addvspace_before,
                    addvspace_flex,
                    vspace_flex,
                    endlist_adjust,
                    penalty_before: None,
                    limitations,
                });
                eject = false;
                prev_vmode = true;
                item_frames = Vec::new();
                list_vmode.clear();
            }
            // beamer Tier 3: every edge is vertical-mode material that ends
            // a paragraph (`\par` in the templates) and, like `\end{frame}`,
            // takes the list-closing skip computed above.
            CBlock::BeamerBlockBegin { span, .. }
            | CBlock::BeamerBlockEnd { span }
            | CBlock::BeamerColumnsBegin { span, .. }
            | CBlock::BeamerColumn { span, .. }
            | CBlock::BeamerColumnsEnd { span }
            | CBlock::BeamerCaption { span, .. } => {
                let kind = match block {
                    CBlock::BeamerBlockBegin { .. } => UnitKind::BeamerBlockBegin { block, span: *span },
                    CBlock::BeamerBlockEnd { .. } => UnitKind::BeamerBlockEnd { span: *span },
                    CBlock::BeamerColumnsBegin { .. } => UnitKind::ColumnsBegin { block, span: *span },
                    CBlock::BeamerColumn { .. } => UnitKind::Column { block, span: *span },
                    CBlock::BeamerColumnsEnd { .. } => UnitKind::ColumnsEnd { span: *span },
                    _ => UnitKind::BeamerCaption { block, span: *span },
                };
                units.push(Unit {
                    kind,
                    eject_before: false,
                    vspace_before,
                    addvspace_before,
                    addvspace_flex,
                    vspace_flex,
                    endlist_adjust,
                    penalty_before: None,
                    limitations,
                });
                eject = false;
                // Not a heading: a list that opens right after the edge takes
                // its `\@topsep` (no `\@nbitem` absorption), like the first
                // list of a frame.
                prev_vmode = false;
                prev_styled = false;
                prev_list = false;
                prev_frames = Vec::new();
                item_frames = Vec::new();
                list_vmode.clear();
            }
            CBlock::VSpace { .. } | CBlock::Rule { .. } | CBlock::PageBreak | CBlock::BeamerFrameBegin { .. } | CBlock::BeamerTitlePage { .. } => unreachable!("handled above"),
            CBlock::Verbatim { .. } | CBlock::Alltt { .. } | CBlock::TableOfContents { .. } | CBlock::TitleBlock { .. } | CBlock::VFill | CBlock::LetterBlock { .. } => unreachable!("lowered by lower_blocks"),
            // Blocks only a re-pinned compiler emits. Skipping a `Penalty`
            // is exactly what the old pin did (it had no such node), so page
            // breaking is unchanged until PR #569's pipeline half reads it;
            // `Tabbing` is lowered to flush-left paragraphs by `lower_blocks`
            // above, as `LetterBlock` is, so it never reaches this walk.
            #[cfg(feature = "compiler-node-surface")]
            CBlock::Penalty { .. } => continue,
            #[cfg(feature = "compiler-node-surface")]
            CBlock::Tabbing { .. } => unreachable!("lowered by lower_blocks"),
            // A beamer `\section` sets nothing: it is a `\tableofcontents`
            // entry (`Block::BeamerToc`) and the gap bookkeeping's marker
            // for the material that follows.
            CBlock::BeamerSection { span, .. } => {
                prev_end = Some(*span);
                prev_vmode = true;
                continue;
            }
        }
        let block_end = match block {
            CBlock::BeamerFrameEnd { span }
            | CBlock::BeamerBlockEnd { span }
            | CBlock::BeamerColumnsBegin { span, .. }
            | CBlock::BeamerColumn { span, .. }
            | CBlock::BeamerColumnsEnd { span } => Some(*span),
            _ => None,
        };
        if let Some(last) = inlines_of(block).iter().filter(|i| !is_marker(i)).map(inline_span).last().or(item_label_span).or(block_end) {
            prev_end = Some(last);
        }
        let _ = eject;
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

/// A `\tag{..}` label as the compiler made it: its text (`\tagform@`'s
/// parentheses for `\tag`, none for `\tag*`) and, for a label that is not
/// plain upright text -- `\tag{hi $x^2$}`, `\tag{\textbf{A}}` (#441) -- the
/// label as a one-atom math list holding its `TextRun`, which the typesetter
/// sets as amsmath's `\maketag@@@` `\hbox` instead of the text.
pub type TagLabel = (String, Option<MathList>);

/// `list` without the atoms the compiler makes of `\tag{..}`/`\tag*{..}` (the
/// label text and the interim glue it inserts, both spanning the command);
/// the label as set goes to `tag`: `\tagform@`'s parentheses for `\tag`,
/// none for `\tag*`. amsmath places the label itself (`\tagform@` flush to
/// the margin, `\eqnshift`/`\calc@shift@*`), so the compiler's interim gap
/// (`math::INTERIM_TAG_GAP_EM`) never reaches the layout.
///
/// A label the compiler could not flatten to one upright string --
/// `\tag{hi $x^2$}`, `\tag{\textbf{A}}` (#441) -- arrives as
/// `Nucleus::TextRun`, a run of text and nested math pieces. It becomes a
/// [`TagLabel`] carrying the run, which the typesetter sets as amsmath's
/// `\maketag@@@` `\hbox` (text pieces in the body face, formulas at
/// `\textstyle`). **It must never be left as `None`:** the caller's `None`
/// arm is the *automatic* equation number, so a dropped rich tag does not
/// look dropped -- `\tag{hi $x^2$}` silently becomes a plausible `(1)`, and
/// a reader cross-referencing the source cannot tell.
fn strip_tag(texts: &[&str], list: &MathList, tag: &mut Option<TagLabel>, notes: &mut Vec<(&'static str, Span, String)>) -> MathList {
    use flashtex_compiler::math::Nucleus;
    let is_tag = |span: Span| texts.get(span.document.0).and_then(|t| t.get(span.start..)).is_some_and(|r| r.starts_with("\\tag"));
    let mut atoms = Vec::with_capacity(list.atoms.len());
    for a in &list.atoms {
        if is_tag(a.span) {
            match &a.nucleus {
                Nucleus::Text(s) | Nucleus::Symbol(s) => *tag = Some((s.clone(), None)),
                // The compiler's interim `2\quad` gap (`INTERIM_TAG_GAP_EM`),
                // which spans the command too. It is not a label; the
                // pipeline places the tag itself.
                Nucleus::Space { .. } => {}
                // A rich label: the run itself, trimmed like `\tagform@`'s
                // `\ignorespaces#1\unskip`, is what gets set; its text is
                // `text_run_reference_text_with_source`, the flattening the
                // compiler itself uses for `\eqref` to this tag, so the
                // set label and the reference to it read alike (composite
                // atoms with no single glyph, e.g. `\frac`, fall back to
                // their source text there).
                #[cfg(feature = "compiler-node-surface")]
                Nucleus::TextRun(pieces) => {
                    let source = texts.get(a.span.document.0).copied().unwrap_or("");
                    let starred = source.get(a.span.start..).is_some_and(|r| r.starts_with("\\tag*"));
                    let content = tag_content_pieces(pieces, starred);
                    let pieces = if starred { content } else { tagform_pieces(content) };
                    let text = flashtex_compiler::math::text_run_reference_text_with_source(&pieces, source);
                    let mut atom = a.clone();
                    atom.nucleus = Nucleus::TextRun(pieces);
                    *tag = Some((text, Some(MathList { atoms: vec![atom] })));
                }
                // Anything else: the label cannot be set, so the display is
                // left unnumbered and the reason is reported. Taking the
                // automatic number here would print a plausible `(1)` for a
                // source that says `\tag{..}`, which no reader could catch.
                _ => {
                    notes.push((
                        "math_limitation",
                        a.span,
                        "\\tag label could not be set; the display is left unnumbered rather than taking the automatic equation number (#441)".to_string(),
                    ));
                    *tag = Some((String::new(), None));
                }
            }
            continue;
        }
        atoms.push(a.clone());
    }
    MathList { atoms }
}

/// The content of a rich `\tag{..}`/`\tag*{..}` label as amsmath stores it
/// in `\@currentlabel` (`\make@df@tag@@`/`\make@df@tag@@@`, amsmath.sty
/// 1224-1227): the compiler's run without the parentheses it wraps an
/// unstarred `\tag` in, and without the spaces at either end --
/// `\tagform@` is `(\ignorespaces#1\unskip\@@italiccorr)` (1211-1212), so
/// those are never set, in the tag or in an `\eqref` to it.
#[cfg(feature = "compiler-node-surface")]
fn tag_content_pieces(pieces: &[flashtex_compiler::math::TextPiece], starred: bool) -> Vec<flashtex_compiler::math::TextPiece> {
    use flashtex_compiler::math::TextPiece;
    let mut out = pieces.to_vec();
    if let Some(TextPiece::Text { text, .. }) = out.first_mut() {
        if !starred {
            *text = text.strip_prefix('(').unwrap_or(text).to_string();
        }
        *text = text.trim_start().to_string();
    }
    if let Some(TextPiece::Text { text, .. }) = out.last_mut() {
        if !starred {
            *text = text.strip_suffix(')').unwrap_or(text).to_string();
        }
        *text = text.trim_end().to_string();
    }
    out.retain(|p| !matches!(p, TextPiece::Text { text, .. } if text.is_empty()));
    out
}

/// `\tagform@`'s parentheses around a label's content, in the upright body
/// face whatever face the content opens or closes in (`\tag{\textbf{B}}`
/// sets `(` upright, `B` bold, `)` upright).
#[cfg(feature = "compiler-node-surface")]
fn tagform_pieces(content: Vec<flashtex_compiler::math::TextPiece>) -> Vec<flashtex_compiler::math::TextPiece> {
    use flashtex_compiler::math::{TextPiece, TextStyle};
    let mut out = Vec::with_capacity(content.len() + 2);
    out.push(TextPiece::text("(", TextStyle::NORMAL));
    for piece in content {
        match (out.last_mut(), piece) {
            (Some(TextPiece::Text { text, style }), TextPiece::Text { text: t, style: st }) if *style == st => text.push_str(&t),
            (_, piece) => out.push(piece),
        }
    }
    match out.last_mut() {
        Some(TextPiece::Text { text, style: TextStyle::Normal }) => text.push(')'),
        _ => out.push(TextPiece::text(")", TextStyle::NORMAL)),
    }
    out
}

/// The rich `\tag` of a display's `list`, if it has one: its content pieces
/// (see [`tag_content_pieces`]), for the `\label`s in the same display.
#[cfg(feature = "compiler-node-surface")]
fn rich_tag_of(texts: &[&str], list: &MathList) -> Option<Vec<flashtex_compiler::math::TextPiece>> {
    use flashtex_compiler::math::Nucleus;
    list.atoms.iter().find_map(|a| {
        let rest = texts.get(a.span.document.0).and_then(|t| t.get(a.span.start..))?;
        match &a.nucleus {
            Nucleus::TextRun(pieces) if rest.starts_with("\\tag") => Some(tag_content_pieces(pieces, rest.starts_with("\\tag*"))),
            _ => None,
        }
    })
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

/// Whether `atom` is amsthm `\qedhere`'s marker: a literal `\qedhere`
/// symbol whose span opens the command in source (both must hold, so a
/// coincidental span never eats formula content).
fn is_qedhere_marker(texts: &[&str], atom: &flashtex_compiler::math::MathAtom) -> bool {
    use flashtex_compiler::math::Nucleus;
    matches!(&atom.nucleus, Nucleus::Symbol(s) if s == "\\qedhere")
        && texts
            .get(atom.span.document.0)
            .and_then(|t| t.get(atom.span.start..))
            .is_some_and(|r| r.starts_with("\\qedhere"))
}

/// Whether `list` holds such a marker atom: the command's span when so.
fn qedhere_at(texts: &[&str], list: &MathList) -> Option<Span> {
    list.atoms.iter().find_map(|a| is_qedhere_marker(texts, a).then_some(a.span))
}

/// `list` without amsthm `\qedhere`'s atoms, and the command's span when
/// one was stripped, so the caller sets the end-of-proof box on the
/// display's own line, flush right, instead of the automatic box after it
/// (which is suppressed once the box is claimed).
///
/// The compiler leaves `\qedhere` as a literal `\qedhere` symbol atom (as
/// it does `\tag`'s atoms for `strip_tag` above).
fn strip_qedhere(texts: &[&str], list: MathList) -> (MathList, Option<Span>) {
    if qedhere_at(texts, &list).is_none() {
        return (list, None);
    }
    let mut found = None;
    let atoms: Vec<_> = list
        .atoms
        .into_iter()
        .filter(|a| {
            if is_qedhere_marker(texts, &a) {
                found = found.or(Some(a.span));
                false
            } else {
                true
            }
        })
        .collect();
    (MathList { atoms }, found)
}

/// Whether any display or alignment row in these inlines carries a
/// `\qedhere` marker: amsthm's claim on the proof's box, which suppresses
/// the automatic pair even when it lands in a later paragraph (the
/// `equation`/align environments flush before `\end{proof}`). Text
/// `\qedhere` needs no such cross-paragraph claim: it shares its
/// paragraph with the pair it suppresses.
fn paragraph_claims_qed(inlines: &[Inline], texts: &[&str]) -> bool {
    inlines.iter().any(|inline| match inline {
        Inline::Math { list, display: true, .. } => qedhere_at(texts, list).is_some(),
        Inline::MathRows { rows, .. } => rows
            .iter()
            .any(|row| row.cells.iter().any(|cell| qedhere_at(texts, cell).is_some())),
        _ => false,
    })
}

/// Retracts a trailing automatic end-of-proof pair — an `HFill` and a
/// `QedBox` whose span opens `\end{proof}` in source, with a possible
/// interword gap before the fill (TeX deletes trailing glue at `\par`
/// anyway) — once a `\qedhere` claim is pending: true when one was
/// removed. Only the automatic pair matches: a box the claim placed
/// itself spans `\qedhere`, never `\end{proof}`.
fn truncate_auto_pair(items: &mut Vec<Item>, texts: &[&str]) -> bool {
    let is_auto = |item: &Item| {
        matches!(item, Item::QedBox { span, .. }
            if texts
                .get(span.document.0)
                .and_then(|t| t.get(span.start..))
                .is_some_and(|r| r.starts_with("\\end{proof}")))
    };
    if !items.last().is_some_and(is_auto) {
        return false;
    }
    items.pop();
    if matches!(items.last(), Some(Item::HFill { .. })) {
        items.pop();
    }
    if matches!(items.last(), Some(Item::Space { .. })) {
        items.pop();
    }
    true
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

struct LengthAssigns {
    parindent: bool,
    parskip: bool,
}

/// Apply the document's `\setlength` / `\addtolength` / `\len=<dimen>`
/// assignments to the page and paragraph lengths after the class defaults
/// and the geometry package, in the order they ran: the compiler's
/// [`Parsed::length_assignments`] (PLAN1 site 33), each already resolved
/// by the expansion engine. A macro that sets a length counts, a
/// definition that is never called does not, and one inside a brace group
/// is local and does not either.
///
/// A page length (`GEOMETRY_LENGTHS`) counts only in the preamble. In the
/// root document it is ignored when `geometry` is loaded after it, matching
/// LaTeX. `source` is the root document, read for that position only (PLAN1
/// site 35).
///
/// [`Parsed::length_assignments`]: flashtex_compiler::parser::Parsed::length_assignments
fn apply_preamble_lengths(
    assignments: &[flashtex_compiler::parser::LengthAssignment],
    source: &str,
    entry: usize,
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
    for assignment in assignments {
        let name = assignment.name.as_str();
        let page = GEOMETRY_LENGTHS.contains(&name);
        if page && !assignment.preamble {
            continue;
        }
        if page && assignment.span.document.0 == entry && last_geometry.is_some_and(|g| assignment.span.start < g) {
            continue;
        }
        if let Some(v) = parse_assignment_glue(&assignment.value, &params, size, em_ex) {
            assign_param(&mut params, name, v, false);
            assigned.parindent |= name == "parindent";
            assigned.parskip |= name == "parskip";
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

/// A `\setlength{<target>}{<value>}`'s target and value, and the byte
/// after the value's closing brace.
fn setlength_args_end(source: &str, mut i: usize) -> Option<(String, String, usize)> {
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
    Some((target, value, i))
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

/// A TeX assignment's dimension (`\len=<dimen>`) and the byte after it.
fn read_assignment_dimen_end(source: &str, mut i: usize) -> Option<(String, usize)> {
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
    Some((source[start..i].to_string(), i))
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

/// Whether the source is a REVTeX document of the `rmp` journal (or the
/// `apsrmp` society), whose natbib is author-year (compiler
/// `natbib::Options::revtex`, `rmp.rtx`/`apsrmp4-*.rtx`
/// `\bibpunct{(}{)}{;}{a}{,}{,}`).
fn revtex_author_year(source: &str) -> bool {
    let Some(at) = find_command(source, "documentclass") else { return false };
    let rest = source[at + "\\documentclass".len()..].trim_start();
    let (options, rest) = match rest.strip_prefix('[') {
        Some(inner) => match inner.find(']') {
            Some(end) => (&inner[..end], inner[end + 1..].trim_start()),
            None => return false,
        },
        None => ("", rest),
    };
    let class = rest.strip_prefix('{').and_then(|r| r.find('}').map(|end| r[..end].trim()));
    matches!(class, Some("revtex4" | "revtex4-1" | "revtex4-2")) && options.split(',').map(str::trim).any(|o| o == "rmp" || o == "apsrmp")
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

/// amsmath.sty's default `\multlinegap` (`\multlinegap10pt`).
const MULTLINE_GAP_DEFAULT: f64 = 10.0;

/// The last `\setlength{\multlinegap}{<dimen>}` of the source, in points
/// ([`MULTLINE_GAP_DEFAULT`] without one): the first and last `multline`
/// rows' indent and the `\shoveleft`/`\shoveright` targets.
fn multline_gap_of(source: &str, size: u32) -> f64 {
    setlength(source, "multlinegap", size).unwrap_or(MULTLINE_GAP_DEFAULT)
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
    // Within an adapt call each answer is read once per document: every
    // `tabular` asks for four lengths, and a scan of the whole source per
    // table made a warm 450-section request spend 0.8 s here (#613).
    let key = (source.as_ptr() as usize, source.len());
    let memo = |scope: &mut Vec<MacroDefsEntry>| scope.iter_mut().find(|e| (e.ptr, e.len) == key).map(|e| e.setlengths.get(&(name.to_string(), size)).copied());
    match MACRO_DEFS.with(|scope| memo(&mut scope.borrow_mut())) {
        Some(Some(found)) => found,
        Some(None) => {
            let found = setlength_in(source, name, size, None);
            MACRO_DEFS.with(|scope| {
                if let Some(entry) = scope.borrow_mut().iter_mut().find(|e| (e.ptr, e.len) == key) {
                    entry.setlengths.insert((name.to_string(), size), found);
                }
            });
            found
        }
        None => setlength_in(source, name, size, None),
    }
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

/// The length register `\<name>` as seen at byte `at`, when the source
/// assigns it before then: `\setlength{\<name>}{v}`, `\setlength\<name>{v}`,
/// `\addtolength` (added to `base`, the value before any assignment, or to
/// the assignment before it) and TeX's `\<name>=v` / `\<name> v`. Every
/// assignment is local, so one made inside `{...}`,
/// `\begingroup...\endgroup` or an environment that has ended by `at` is
/// undone -- a `\tabcolsep` set for one table does not reach the next.
/// The definitions of macros are skipped, and an invocation of one makes
/// the assignments its replacement text makes outside its own groups
/// ([`macro_length_assignments`]), at the invocation.
fn length_at(source: &str, name: &str, size: u32, at: usize, base: f64) -> Option<f64> {
    length_at_checked(source, name, size, at, base).0
}

/// A `table_limitation` for each table length a macro invoked before the
/// table at `span` assigns in a way [`length_at_checked`] cannot read: the
/// table is set with the value before that assignment instead.
fn table_length_limitations(texts: &[&str], span: Span, size: u32, out: &mut Vec<(&'static str, Span, String)>) {
    let Some(src) = texts.get(span.document.0) else { return };
    let mut unread = Vec::new();
    crate::table::TableLengths::read(|name, base| {
        let (value, unresolved) = length_at_checked(src, name, size, span.start, base);
        if unresolved {
            unread.push(name.to_string());
        }
        value
    });
    for name in unread {
        {
            out.push((
                "table_limitation",
                span,
                format!("a macro assigns \\{name} before this table with an argument FlashTeX cannot read; the table uses \\{name} without that assignment"),
            ));
        }
    }
}

/// [`length_at`], and whether a macro invoked before `at` assigns `\<name>`
/// in a way that could not be read (an argument that is not a braced
/// group, an optional argument, or a value that does not parse): the value
/// then ignores that assignment, and the caller reports it.
fn length_at_checked(source: &str, name: &str, size: u32, at: usize, base: f64) -> (Option<f64>, bool) {
    let at = at.min(source.len());
    // Within an adapt call each length is indexed once per document and
    // every table looks its value up (#623): a scan of the source before
    // each table made a document of 800 tables spend 1.6 s here.
    if !source.is_char_boundary(at) || splits_a_control_word(source, at) {
        return length_at_scan(source, name, size, at, base);
    }
    let key = (source.as_ptr() as usize, source.len());
    let index_key = (name.to_string(), size, base.to_bits());
    let lookup = |index_key: &(String, u32, u64)| {
        MACRO_DEFS.with(|scope| {
            let scope = scope.borrow();
            let entry = scope.iter().find(|e| (e.ptr, e.len) == key)?;
            Some(entry.lengths.by_name.get(index_key).map(|index| index.as_ref().map(|index| index.at(at))))
        })
    };
    let found = match lookup(&index_key) {
        // Not in an adapt call: nothing is indexed.
        None => return length_at_scan(source, name, size, at, base),
        Some(Some(found)) => found,
        Some(None) => {
            // `macro_length_assignments` reads the definition index, so the
            // assignments are collected before the scope is borrowed.
            let assignments = length_assignments(source, name, source.len());
            MACRO_DEFS.with(|scope| {
                let mut scope = scope.borrow_mut();
                let entry = scope.iter_mut().find(|e| (e.ptr, e.len) == key)?;
                let lengths = &mut entry.lengths;
                let groups = lengths.groups.get_or_insert_with(|| GroupTokens::new(source));
                let index = LengthIndex::new(assignments, groups, size, base);
                let found = index.as_ref().map(|index| index.at(at));
                lengths.by_name.insert(index_key, index);
                Some(found)
            })
            .flatten()
        }
    };
    found.unwrap_or_else(|| length_at_scan(source, name, size, at, base))
}

/// [`length_at_checked`] by scanning the source before `at`: the reference
/// [`LengthIndex`] reproduces, and what is used outside an adapt call.
fn length_at_scan(source: &str, name: &str, size: u32, at: usize, base: f64) -> (Option<f64>, bool) {
    let mut value = None;
    let mut unresolved = false;
    for (assignments, end) in length_assignments(source, name, at) {
        if end > at || !group_open_between(source, end, at) {
            continue;
        }
        apply_length_assignments(assignments, size, base, &mut value, &mut unresolved);
    }
    (value, unresolved)
}

/// The assignments to `\<name>` made by the control words of
/// `source[..until]`, in source order, each with the byte after its
/// arguments: a `\setlength`/`\addtolength` of it, TeX's `\<name>=v`, or an
/// invocation of a macro that assigns it ([`macro_length_assignments`]).
/// The definitions of macros are skipped.
#[allow(clippy::type_complexity)]
fn length_assignments(source: &str, name: &str, until: usize) -> Vec<(Vec<Option<(String, bool)>>, usize)> {
    let mut out = Vec::new();
    let mut scan = CmdScan::new(&source[..until]);
    while let Some((cmd_at, cmd, _)) = scan.next() {
        let after_name = cmd_at + 1 + cmd.len();
        if matches!(cmd, "newcommand" | "renewcommand" | "providecommand" | "def" | "gdef" | "edef" | "xdef") {
            scan.skip_to(skip_macro_definition(source, cmd, after_name));
            continue;
        }
        if cmd == "setlength" || cmd == "addtolength" {
            let Some((target, raw, end)) = setlength_args_end(source, after_name) else { continue };
            if target == name {
                out.push((vec![Some((raw, cmd == "addtolength"))], end));
            }
        } else if cmd == name {
            if let Some((raw, end)) = read_assignment_dimen_end(source, after_name) {
                out.push((vec![Some((raw, false))], end));
            }
        } else if let Some(found) = macro_length_assignments(source, cmd, cmd_at, after_name, name, 0) {
            out.push(found);
        }
    }
    out
}

/// Applies one command's assignments ([`length_assignments`]) to the value
/// before it; one that does not parse leaves the value and is reported.
fn apply_length_assignments(assignments: Vec<Option<(String, bool)>>, size: u32, base: f64, value: &mut Option<f64>, unresolved: &mut bool) {
    for assignment in assignments {
        match assignment.and_then(|(raw, add)| Some((parse_dimen_in(raw.trim().trim_start_matches('='), size, None)?, add))) {
            Some((v, add)) => *value = Some(if add { value.unwrap_or(base) + v } else { v }),
            None => *unresolved = true,
        }
    }
}

/// Whether byte `at` falls inside a control word or right after a `\`,
/// where [`length_at_scan`] lexes the command cut short at `at` and the
/// index, which lexes whole commands, could differ. A table's offset is the
/// `\` of its `\begin`, never one of these.
fn splits_a_control_word(source: &str, at: usize) -> bool {
    let b = source.as_bytes();
    if at == 0 || at >= b.len() {
        return false;
    }
    if b[at - 1] == b'\\' {
        return true;
    }
    b[at].is_ascii_alphabetic() && b[..at].iter().rposition(|c| !c.is_ascii_alphabetic()).is_some_and(|s| b[s] == b'\\')
}

/// The tokens [`group_open_between`] counts, lexed once from byte 0: where
/// each `{`, `}`, `\begin`, `\begingroup`, `\end` and `\endgroup` is and the
/// depth around it. Lexing from any byte the lexer from 0 also reaches
/// yields the same tokens after it, so `group_open_between(from, to)` is
/// whether no token from `from` on and before `to` takes the depth below
/// the depth at `from`.
struct GroupTokens {
    /// The byte of each token, in order.
    at: Vec<usize>,
    /// `closes[k]`: the byte of the first token from the `k`-th on that
    /// leaves the depth below the depth before the `k`-th (`usize::MAX` for
    /// none); `k` runs to `at.len()` inclusive.
    closes: Vec<usize>,
    /// Bytes the lexer from 0 steps over inside a token and a lexer started
    /// there would read differently, as sorted ranges: comment bodies and
    /// the byte an escaping `\` takes.
    unsynced: Vec<(usize, usize)>,
}

impl GroupTokens {
    fn new(source: &str) -> GroupTokens {
        // `group_open_between(source, 0, source.len())`'s lexing.
        let b = source.as_bytes();
        let (mut at, mut deltas, mut unsynced) = (Vec::new(), Vec::new(), Vec::new());
        let mut i = 0;
        while i < b.len() {
            match b[i] {
                b'%' => {
                    let start = i;
                    while i < b.len() && b[i] != b'\n' {
                        i += 1;
                    }
                    if i > start + 1 {
                        unsynced.push((start + 1, i));
                    }
                }
                b'{' => {
                    at.push(i);
                    deltas.push(1);
                }
                b'}' => {
                    at.push(i);
                    deltas.push(-1);
                }
                b'\\' => {
                    let name_end = source[i + 1..].find(|c: char| !c.is_ascii_alphabetic()).map_or(b.len(), |n| i + 1 + n);
                    match &source[i + 1..name_end] {
                        "begin" | "begingroup" => {
                            at.push(i);
                            deltas.push(1);
                        }
                        "end" | "endgroup" => {
                            at.push(i);
                            deltas.push(-1);
                        }
                        "" => {
                            unsynced.push((i + 1, i + 2));
                            i += 1;
                        }
                        _ => {}
                    }
                    i = name_end.max(i + 1);
                    continue;
                }
                _ => {}
            }
            i += 1;
        }
        // `depth[k]`: the depth before the `k`-th token. The first later
        // depth below it, by a stack of indexes of non-decreasing depth.
        let mut depth = Vec::with_capacity(deltas.len() + 1);
        depth.push(0i64);
        for d in &deltas {
            depth.push(depth[depth.len() - 1] + d);
        }
        let mut closes = vec![usize::MAX; depth.len()];
        let mut stack: Vec<usize> = Vec::new();
        for (k, &d) in depth.iter().enumerate() {
            while stack.last().is_some_and(|&top| depth[top] > d) {
                // `depth[k]` is the depth after token `k - 1`.
                closes[stack.pop().unwrap_or_default()] = at[k - 1];
            }
            stack.push(k);
        }
        GroupTokens { at, closes, unsynced }
    }

    /// Whether a lexer started at `byte` reads what the lexer from 0 reads.
    fn synced(&self, byte: usize) -> bool {
        let k = self.unsynced.partition_point(|r| r.0 <= byte);
        k == 0 || byte >= self.unsynced[k - 1].1
    }

    /// The byte of the first token from `from` on that closes the group
    /// open at `from` (`usize::MAX` for none): `group_open_between(from, to)`
    /// is `to <= group_close(from)` for a synced `from`.
    fn group_close(&self, from: usize) -> usize {
        self.closes[self.at.partition_point(|&p| p < from)]
    }
}

/// The value [`length_at_scan`] gives one length at every byte, from its
/// assignments read once ([`length_assignments`] of the whole source).
///
/// The assignments still in force at a byte are a chain: an assignment is
/// in force where no group closes between its end and the byte, so when one
/// is, every earlier one in force at the byte is also in force at its end.
/// Each assignment keeps the latest earlier one in force at its end
/// (`parent`) and the value and `unresolved` flag of the chain up to it; a
/// lookup takes the last assignment ending by the byte and walks the chain
/// back past those whose group has closed.
struct LengthIndex {
    /// The byte after each assignment's arguments, never decreasing.
    ends: Vec<usize>,
    /// Where each assignment's group closes ([`GroupTokens::group_close`]).
    closes: Vec<usize>,
    parent: Vec<Option<usize>>,
    value: Vec<Option<f64>>,
    unresolved: Vec<bool>,
}

impl LengthIndex {
    /// `None` when the assignments cannot be read as a chain: one ends before
    /// an earlier one (an assigning macro in another's argument) or ends where
    /// lexing from its end differs from lexing from 0. The table then scans.
    #[allow(clippy::type_complexity)]
    fn new(assignments: Vec<(Vec<Option<(String, bool)>>, usize)>, groups: &GroupTokens, size: u32, base: f64) -> Option<LengthIndex> {
        let n = assignments.len();
        let mut index = LengthIndex {
            ends: Vec::with_capacity(n),
            closes: Vec::with_capacity(n),
            parent: Vec::with_capacity(n),
            value: Vec::with_capacity(n),
            unresolved: Vec::with_capacity(n),
        };
        for (assigned, end) in assignments {
            if index.ends.last().is_some_and(|&last| end < last) || !groups.synced(end) {
                return None;
            }
            let parent = index.in_force(index.ends.len().checked_sub(1), end);
            let (mut value, mut unresolved) = parent.map_or((None, false), |p| (index.value[p], index.unresolved[p]));
            apply_length_assignments(assigned, size, base, &mut value, &mut unresolved);
            index.ends.push(end);
            index.closes.push(groups.group_close(end));
            index.parent.push(parent);
            index.value.push(value);
            index.unresolved.push(unresolved);
        }
        Some(index)
    }

    /// The last assignment from `from` back along the chain still in force
    /// at byte `at`.
    fn in_force(&self, mut from: Option<usize>, at: usize) -> Option<usize> {
        while let Some(k) = from {
            if self.closes[k] >= at {
                break;
            }
            from = self.parent[k];
        }
        from
    }

    /// [`length_at_scan`] at byte `at`.
    fn at(&self, at: usize) -> (Option<f64>, bool) {
        let last = self.ends.partition_point(|&end| end <= at).checked_sub(1);
        self.in_force(last, at).map_or((None, false), |k| (self.value[k], self.unresolved[k]))
    }
}

/// [`length_at_checked`]'s indexes of one document within an adapt call.
#[derive(Default)]
struct LengthIndexes {
    groups: Option<GroupTokens>,
    /// By length name, class size and the bits of the value before any
    /// assignment; `None` for a document the index cannot read.
    by_name: HashMap<(String, u32, u64), Option<LengthIndex>>,
}

/// The assignments to `\<name>` the user macro `\<cmd>` makes when invoked
/// at `cmd_at` (its name ends at `after_name`): each `\setlength`/
/// `\addtolength`/`\<name>=` at the top level of its replacement text, and
/// those of the macros it invokes there, in order, as `(value, add)` with
/// `#k` replaced by the invocation's braced arguments; `None` for one that
/// cannot be read that way. Also the byte where the invocation's arguments
/// end. `None` when `\<cmd>` is not a macro or assigns nothing to `\<name>`.
/// Assignments inside the replacement text's own groups are undone by
/// them, as in TeX.
fn macro_length_assignments(source: &str, cmd: &str, cmd_at: usize, after_name: usize, name: &str, depth: u8) -> Option<(Vec<Option<(String, bool)>>, usize)> {
    if depth > 4 {
        return None;
    }
    let body = macro_body(source, cmd, cmd_at)?;
    if !body.contains('\\') {
        return None;
    }
    let body_start = body.as_ptr() as usize - source.as_ptr() as usize;
    // `#k` of the body; `[n][default]` makes `#1` optional.
    let params = body.as_bytes().windows(2).filter(|w| w[0] == b'#' && w[1].is_ascii_digit()).map(|w| usize::from(w[1] - b'0')).max().unwrap_or(0);
    let optional = {
        let head = source[..body_start].trim_end();
        head.strip_suffix('{').map(str::trim_end).is_some_and(|h| h.ends_with(']') && h[..h.len() - 1].rfind('[').is_some_and(|o| h[..o].trim_end().ends_with(']')))
    };
    let mut end = after_name;
    let mut args: Vec<String> = Vec::new();
    let mut args_ok = !optional;
    if args_ok {
        for _ in 0..params {
            match read_group(source, &mut end) {
                Some(arg) => args.push(arg),
                None => {
                    args_ok = false;
                    break;
                }
            }
        }
    }
    let substitute = |raw: &str| -> Option<String> {
        if !raw.contains('#') {
            return Some(raw.to_string());
        }
        if !args_ok {
            return None;
        }
        let mut out = String::new();
        let mut chars = raw.chars().peekable();
        while let Some(c) = chars.next() {
            match (c, chars.peek().and_then(|d| d.to_digit(10))) {
                ('#', Some(k)) => {
                    chars.next();
                    out.push_str(args.get(k as usize - 1)?);
                }
                _ => out.push(c),
            }
        }
        Some(out)
    };
    let mut found = Vec::new();
    let mut scan = CmdScan::new(body);
    while let Some((off, inner, level)) = scan.next() {
        let after = off + 1 + inner.len();
        if matches!(inner, "newcommand" | "renewcommand" | "providecommand" | "def" | "gdef" | "edef" | "xdef") {
            scan.skip_to(skip_macro_definition(body, inner, after));
            continue;
        }
        if level != 0 {
            continue;
        }
        if inner == "setlength" || inner == "addtolength" {
            let Some((target, raw, _)) = setlength_args_end(body, after) else { continue };
            // `\setlength{#1}{..}`: the target is an argument.
            let target = if target.starts_with('#') { substitute(&target).map(|t| t.trim().trim_start_matches('\\').to_string()) } else { Some(target) };
            match target {
                Some(t) if t == name => found.push(substitute(&raw).map(|r| (r, inner == "addtolength"))),
                Some(_) => {}
                None => found.push(None),
            }
        } else if inner == name {
            if let Some((raw, _)) = read_assignment_dimen_end(body, after) {
                found.push(substitute(&raw).map(|r| (r, false)));
            } else if body[after..].trim_start().starts_with(['=', '#']) {
                found.push(None);
            }
        } else if inner != cmd {
            let nested_at = body_start + off;
            if let Some((nested, _)) = macro_length_assignments(source, inner, nested_at, body_start + after, name, depth + 1) {
                found.extend(nested);
            }
        }
    }
    (!found.is_empty()).then_some((found, end))
}

/// Whether the group open at byte `from` is still open at `to`: no `}`,
/// `\endgroup` or `\end` in between closes more than was opened after
/// `from`. Comments and escaped braces are skipped.
fn group_open_between(source: &str, from: usize, to: usize) -> bool {
    let b = source.as_bytes();
    let mut depth = 0i64;
    let mut i = from;
    while i < to {
        match b[i] {
            b'%' => {
                while i < to && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b'\\' => {
                let name_end = source[i + 1..to].find(|c: char| !c.is_ascii_alphabetic()).map_or(to, |n| i + 1 + n);
                match &source[i + 1..name_end] {
                    "begin" | "begingroup" => depth += 1,
                    "end" | "endgroup" => depth -= 1,
                    // `\{`, `\}`, `\%`, `\\`: one escaped character.
                    "" => i += 1,
                    _ => {}
                }
                i = name_end.max(i + 1);
                if depth < 0 {
                    return false;
                }
                continue;
            }
            _ => {}
        }
        if depth < 0 {
            return false;
        }
        i += 1;
    }
    true
}

/// `<n>` when the bytes of `span` are exactly `\hspace{<n>em}` or
/// `\hspace*{<n>em}` (a rigid length in ems, no `plus`/`minus`).
fn hspace_ems(source: &str, span: Span) -> Option<f64> {
    let text = source.get(span.start..span.end)?;
    let rest = text.strip_prefix("\\hspace")?.trim_start();
    let rest = rest.strip_prefix('*').unwrap_or(rest).trim_start();
    let arg = rest.strip_prefix('{')?.strip_suffix('}')?.trim();
    let number = arg.strip_suffix("em")?.trim();
    if number.is_empty() || !number.bytes().all(|c| c.is_ascii_digit() || matches!(c, b'.' | b'-' | b'+')) {
        return None;
    }
    number.parse().ok()
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

#[cfg(test)]
fn list_seps(source: &str, env: &str, depth: usize, size: u32, style: &Stylesheet) -> ListSeps {
    list_seps_with(source, env, depth, size, style, "")
}

/// [`list_seps`] with the keys of the list's own `\begin{<env>}[<keys>]`
/// optional argument applied after every `\setlist` (enumitem: `nosep`
/// zeroes `topsep`/`partopsep`/`itemsep`/`parsep`, `noitemsep` zeroes
/// `itemsep`/`parsep`).
fn list_seps_with(source: &str, env: &str, depth: usize, size: u32, style: &Stylesheet, begin_keys: &str) -> ListSeps {
    list_seps_from(&setlist_calls(source), env, depth, size, style, begin_keys)
}

/// The class's (or beamer's) glue of list level `depth`, before any
/// enumitem key.
fn class_list_seps(depth: usize, size: u32, style: &Stylesheet) -> ListSeps {
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
        seps.itemsep_skip = style.itemsep;
        seps.itemsep = style.itemsep.natural;
    } else if style.is_beamer() {
        // `beamerbaselocalstructure.sty` `\@listii`/`\@listiii`.
        if let Some(g) = style.class_geometry.as_deref() {
            let l = flashtex_class_geometry::beamer::list_level(g.font, depth as u8);
            let glue = |g: flashtex_class_geometry::Glue| crate::style::Skip::new(crate::style::frame_pt(g.natural), crate::style::frame_pt(g.stretch), crate::style::frame_pt(g.shrink));
            seps.topsep_skip = glue(l.topsep);
            seps.topsep = seps.topsep_skip.natural;
            seps.partopsep_skip = glue(l.partopsep);
            seps.partopsep = seps.partopsep_skip.natural;
            seps.parsep_skip = glue(l.parsep);
            seps.parsep = seps.parsep_skip.natural;
            seps.itemsep_skip = glue(l.itemsep);
            seps.itemsep = seps.itemsep_skip.natural;
        }
    }
    seps
}

impl ListSeps {
    fn set_topsep(&mut self, skip: crate::style::Skip) {
        self.topsep = skip.natural;
        self.topsep_skip = skip;
    }
    fn set_partopsep(&mut self, skip: crate::style::Skip) {
        self.partopsep = skip.natural;
        self.partopsep_skip = skip;
    }
    fn set_itemsep(&mut self, skip: crate::style::Skip) {
        self.itemsep = skip.natural;
        self.itemsep_skip = skip;
    }
    fn set_parsep(&mut self, skip: crate::style::Skip) {
        self.parsep = skip.natural;
        self.parsep_skip = skip;
    }
}

/// [`class_list_seps`] with the enumitem keys of the compiler's
/// [`ListFrame::options`] (every matching `\setlist`, then the `\begin`
/// keys, their `em`/`ex` evaluated where the list starts) applied in order:
/// `nosep` zeroes `topsep`/`partopsep`/`itemsep`/`parsep`, `noitemsep`
/// zeroes `itemsep`/`parsep`.
fn list_seps_of(options: &[ListOption], depth: usize, size: u32, style: &Stylesheet) -> ListSeps {
    let mut seps = class_list_seps(depth, size, style);
    let skip = |s: &flashtex_compiler::parser::ListSkip| crate::style::Skip::new(s.pt, s.plus, s.minus);
    for option in options {
        match option {
            ListOption::NoSep => {
                seps.set_topsep(crate::style::Skip::default());
                seps.set_partopsep(crate::style::Skip::default());
                seps.set_itemsep(crate::style::Skip::fixed(0.0));
                seps.set_parsep(crate::style::Skip::fixed(0.0));
            }
            ListOption::NoItemSep => {
                seps.set_itemsep(crate::style::Skip::fixed(0.0));
                seps.set_parsep(crate::style::Skip::fixed(0.0));
            }
            ListOption::TopSep(s) => seps.set_topsep(skip(s)),
            ListOption::PartopSep(s) => seps.set_partopsep(skip(s)),
            ListOption::ItemSep(s) => seps.set_itemsep(skip(s)),
            ListOption::ParSep(s) => seps.set_parsep(skip(s)),
            _ => {}
        }
    }
    seps
}

/// The compiler's frames of the `\list` environments this module models
/// ([`LIST_ENVS`]), outermost first: the stack the list glue and margins
/// are read from.
fn modelled_lists(frames: &[ListFrame]) -> Vec<&ListFrame> {
    frames.iter().filter(|f| modelled_list(f)).collect()
}

fn modelled_list(frame: &ListFrame) -> bool {
    LIST_ENVS.contains(&frame.environment.name())
}

/// [`list_seps_with`] given the source's [`setlist_calls`].
fn list_seps_from(calls: &[(&str, &str)], env: &str, depth: usize, size: u32, style: &Stylesheet, begin_keys: &str) -> ListSeps {
    let mut seps = class_list_seps(depth, size, style);
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
///
/// The kernel `list` itself is one of them: `\begin{list}{<label>}{<decl>}`
/// *is* `\list`, so it advances `\@listdepth`, takes `\@list<i>`'s glue and
/// margin and ends with the same `\endtrivlist`. Leaving it out gave every
/// `\begin{list}` a zero `\leftmargin` and the document's paragraph glue --
/// visible on the algorithm-pseudocode packages, whose `algorithmic`
/// environment is a `list` and nothing else.
pub(crate) const LIST_ENVS: [&str; 5] = ["itemize", "enumerate", "description", "thebibliography", "list"];

/// `\endtrivlist` for each of the `closed` innermost lists of `stack` (the
/// lists open where the previous unit ended), innermost first: when the
/// list leaves a positive `\lastskip` it becomes `\lastskip + \parskip -
/// \@outerparskip` — the closing list's `\parsep` less the `\parskip`
/// outside it (the enclosing list's `\parsep`, or the document's). The
/// summed change, in points.
fn list_end_adjust(stack: &[&ListFrame], closed: usize, size: u32, style: &Stylesheet) -> f64 {
    let mut adjust = 0.0;
    for k in 0..closed {
        let Some(depth) = stack.len().checked_sub(k).filter(|d| *d > 0) else { break };
        let parsep = list_seps_of(&stack[depth - 1].options, depth, size, style).parsep;
        let outer = if depth > 1 { list_seps_of(&stack[depth - 2].options, depth - 1, size, style).parsep } else { style.parskip.natural };
        adjust += parsep - outer;
    }
    adjust
}

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

/// The paragraph-shape environments that are a `\trivlist` or a `\list` but
/// whose `\item`s the compiler does *not* report as `CBlock::ListItem`:
/// article.cls builds `center`/`flushleft`/`flushright` with `\trivlist
/// \centering \item\relax` and `quote`/`quotation`/`verse` with
/// `\list{}{...}\item\relax`.
///
/// They matter here only for what their `\end` leaves behind, which is the
/// same `\endtrivlist` -> `\@endparenv` every list ends with.
///
/// `verbatim`/`verbatim*` are here for the same reason: `\@verbatim` is
/// `\trivlist \item\relax ...` and `\endverbatim` is `\endtrivlist`
/// (latex.ltx). `abstract` is here for its `\end`: article.cls sets its
/// one-column form as `\small`, a centred head and a `\quotation`, so
/// `\end{abstract}` is `\endquotation` -> `\endlist` -> `\endtrivlist`.
const TRIVLIST_ENVS: [&str; 9] =
    ["center", "flushleft", "flushright", "quote", "quotation", "verse", "verbatim", "verbatim*", "abstract"];

/// Whether the material immediately before `at` ends with the `\end` of a
/// `\trivlist`-derived environment, so `\@endparenv` has just put that
/// environment's `\addvspace\@topsepadd` on the vertical list.
///
/// [`crate::listings`] asks this because a `lstlisting` opens with a
/// `\vspace`, not an `\addvspace`, so the skip the previous `\end` left is
/// *added to* rather than shared with it and has to survive the pass that
/// undoes the `flushleft` lowering.
///
/// Theorem-like environments are deliberately not counted: `\@thm` assigns
/// `\@topsepadd` outright from `\thm@postskip`, which is not the
/// `\@trivlist` value this answer stands for, and
/// `\end{thm}\begin{lstlisting}` measures correct without it. `abstract` is
/// not counted either, for a sharper reason: its `\end` is an
/// `\endtrivlist` only in one column, and its `\@topsepadd` is then
/// `\small`'s (6/9/12 pt, not the body's 10/12/13) — both branches are
/// already exact without this, one because the body block closes the
/// environment itself and one because there is no list to close.
pub(crate) fn ends_trivlist_env_before(text: &str, at: usize) -> bool {
    let before = text.get(..at).unwrap_or("").trim_end();
    let Some(end) = before.rfind("\\end") else { return false };
    let rest = before[end + "\\end".len()..].trim_start();
    let Some(rest) = rest.strip_prefix('{') else { return false };
    let Some((name, after)) = rest.split_once('}') else { return false };
    let name = name.trim();
    after.trim().is_empty() && name != "abstract" && (LIST_ENVS.contains(&name) || TRIVLIST_ENVS.contains(&name))
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
///
/// Only [`list_end_skip`] reads this, for a float body: floats are masked
/// out of the source before the compiler runs (PLAN1 site 41), so their
/// lists have no `ListItem` frames. Every compiled list's stack is its
/// frames.
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

/// Every `\begin` and `\end` control word of `source` outside comments, in
/// order: `(byte offset, is_begin)`. The lexing is [`find_command`]'s, so
/// this is exactly what repeated `find_command` calls restarted one byte
/// past each match report.
fn begin_end_commands(source: &str) -> Vec<(usize, bool)> {
    let bytes = source.as_bytes();
    let word = |i: usize, needle: &str| bytes[i..].starts_with(needle.as_bytes()) && bytes.get(i + needle.len()).is_none_or(|b| !b.is_ascii_alphabetic());
    let mut out = Vec::new();
    let (mut i, mut in_comment) = (0, false);
    while i < bytes.len() {
        match bytes[i] {
            b'\n' if in_comment => in_comment = false,
            _ if in_comment => {}
            b'%' => in_comment = true,
            b'\\' => {
                if word(i, "\\begin") {
                    out.push((i, true));
                } else if word(i, "\\end") {
                    out.push((i, false));
                }
                i += 2;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    out
}

/// What [`split_at_page_breaks`] reads from one source document for every
/// block, computed in one forward pass per call.
///
/// `in_theorem_environment` rescans the source from byte 0 each time, and
/// [`natbib_author_year`] scans all of it, so asking them once per block made
/// the split quadratic in the document's length (#613: 16 s of adapt time at
/// 450 sections). The index answers the same questions from prefix snapshots
/// with a binary search, and returns what those functions return at every
/// byte offset (`source_index_matches_prefix_scans` checks this). The list
/// stack and the enumitem keys are the compiler's `ListItem.lists` frames
/// (PLAN1 slice 3), not a scan.
struct SourceIndex {
    /// [`natbib_author_year`] of the source.
    natbib_author_year: bool,
    /// The document loads `enumerate` (tools) and not enumitem, so an
    /// `enumerate` environment's `[<template>]` is enumerate.sty's
    /// ([`enumerate_sty_widest`]).
    enumerate_package: bool,
    /// For each named `\begin{..}`/`\end{..}`, in source order, the byte of
    /// the `}` closing its name: `in_theorem_environment` matches the
    /// command at offset `at` only when the name is complete before `at`.
    /// The marks never decrease.
    theorem_marks: Vec<usize>,
    /// `in_theorem[k]`: a theorem-like environment is open after the first
    /// `k` named commands.
    in_theorem: Vec<bool>,
}

impl SourceIndex {
    fn new(source: &str, theorem_envs: &std::collections::HashSet<String>) -> Self {
        let commands = begin_end_commands(source);
        // `in_theorem_environment`: the name is between the first `{` after
        // the command and the first `}` after that, wherever they are.
        let mut open: Vec<&str> = Vec::new();
        let mut theorems_open = 0usize;
        let (mut theorem_marks, mut in_theorem) = (Vec::new(), vec![false]);
        for &(pos, is_begin) in &commands {
            let Some(brace) = source[pos..].find('{').map(|b| pos + b) else { break };
            let Some(close) = source[brace + 1..].find('}').map(|c| brace + 1 + c) else { break };
            let name = source[brace + 1..close].trim();
            if is_begin {
                open.push(name);
                theorems_open += usize::from(theorem_envs.contains(name));
            } else if open.last() == Some(&name) {
                open.pop();
                theorems_open -= usize::from(theorem_envs.contains(name));
            }
            theorem_marks.push(close);
            in_theorem.push(theorems_open > 0);
        }
        SourceIndex {
            natbib_author_year: natbib_author_year(source),
            enumerate_package: package_options(source, "enumerate").is_some() && package_options(source, "enumitem").is_none(),
            theorem_marks,
            in_theorem,
        }
    }

    /// `in_theorem_environment(source, at, theorem_envs)`, given
    /// whether `at` is a char boundary within the source.
    fn in_theorem(&self, at_in_bounds: bool, at: usize) -> bool {
        at_in_bounds && self.in_theorem[self.theorem_marks.partition_point(|&mark| mark < at)]
    }
}

/// One lazily built [`SourceIndex`] per document of a `split_at_page_breaks`
/// call; a document index past `texts` reads as the empty source, as the
/// per-block code did.
struct SourceIndexes<'a, 't> {
    texts: &'a [&'t str],
    theorem_envs: &'a std::collections::HashSet<String>,
    /// [`natbib_author_year`] of the document, not of one file: a package
    /// is loaded once, in the preamble, and holds for every file the
    /// document reads -- the `.bbl` that `\bibliography` inputs never
    /// loads natbib itself, so its `thebibliography` was set with the
    /// class's `[n]` label-width geometry (15.5 bp too far right).
    natbib_author_year: bool,
    cells: Vec<std::cell::OnceCell<SourceIndex>>,
    empty: std::cell::OnceCell<SourceIndex>,
}

impl<'a, 't> SourceIndexes<'a, 't> {
    fn new(texts: &'a [&'t str], theorem_envs: &'a std::collections::HashSet<String>) -> Self {
        // A REVTeX class loads natbib itself; its `rmp` journal is
        // author-year (compiler `natbib::Options::revtex`).
        let revtex_rmp = texts.iter().any(|text| revtex_author_year(text));
        let natbib_author_year = revtex_rmp || texts.iter().find(|text| natbib_options(text).is_some()).is_some_and(|text| natbib_author_year(text));
        SourceIndexes {
            texts,
            theorem_envs,
            natbib_author_year,
            cells: texts.iter().map(|_| std::cell::OnceCell::new()).collect(),
            empty: std::cell::OnceCell::new(),
        }
    }

    fn get(&self, document: usize) -> &SourceIndex {
        match self.texts.get(document) {
            Some(text) => self.cells[document].get_or_init(|| SourceIndex { natbib_author_year: self.natbib_author_year, ..SourceIndex::new(text, self.theorem_envs) }),
            None => self.empty.get_or_init(|| SourceIndex::new("", self.theorem_envs)),
        }
    }
}

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
///
/// A `widest=<text>` key replaces that assumption (`\enit@calcwidth`): an
/// enumerate's counter is set as `<text>` itself inside the label
/// (`widest=iii` with `label=\roman*.` measures `iii.`), and an itemize
/// measures `<text>` alone.
fn widest_label(env: &str, depth: usize, label_key: Option<&str>, template: Option<&str>, widest: Option<&str>) -> String {
    if let (Some(text), "itemize") = (widest, env) {
        return text.to_string();
    }
    let counter = |default: &'static str| widest.unwrap_or(default).to_string();
    if let Some(label) = label_key {
        return [("\\alph*", "m"), ("\\Alph*", "M"), ("\\roman*", "viii"), ("\\Roman*", "VIII"), ("\\arabic*", "0")]
            .iter()
            .fold(label.to_string(), |text, (command, default)| text.replace(command, &counter(default)));
    }
    if env == "itemize" {
        // Named, not spelled: the typesetter measures `\labelitemii`'s en
        // dash bold and, without `lmodern`, the TS1 symbols in `tcrm`.
        return match depth {
            1 => "\\labelitemi",
            2 => "\\labelitemii",
            3 => "\\labelitemiii",
            _ => "\\labelitemiv",
        }
        .to_string();
    }
    if let Some(template) = template {
        if let Some((index, style)) = template.char_indices().find(|(_, c)| "aAiI1".contains(*c)) {
            let widest = counter(match style {
                'a' => "m",
                'A' => "M",
                'i' => "viii",
                'I' => "VIII",
                _ => "0",
            });
            return format!("{}{}{}", &template[..index], widest, &template[index + 1..]);
        }
        return template.to_string();
    }
    match depth {
        1 => format!("{}.", counter("0")),
        2 => format!("({})", counter("m")),
        3 => format!("{}.", counter("viii")),
        _ => format!("{}.", counter("M")),
    }
}

/// The label enumerate.sty (tools, v3.00) measures for `\leftmargin<depth>`
/// of `\begin{enumerate}[<template>]`: `\@enloop` (lines 52-65) makes every
/// unbraced `A`, `a`, `i`, `I` or `1` token of the template the counter
/// (`\Alph`, `\alph`, `\roman`, `\Roman`, `\arabic`), keeps a braced
/// group as literal text and any other token as itself; `\@@enum@` (lines
/// 72-83) then sets the counter to 7 and `\settowidth`s the margin to
/// `\the\@enLab\hspace{\labelsep}` -- so `[(i)]` measures `(vii)`, `[a)]`
/// measures `g)` and `[1.]` measures `7.`, each plus `\labelsep`
/// ([`ListMargin::Widest`]). The counter is measured in the current text
/// font: `\settowidth` runs before `\list`, in the enclosing font.
fn enumerate_sty_widest(template: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    for c in template.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            'A' if depth == 0 => out.push('G'),
            'a' if depth == 0 => out.push('g'),
            'i' if depth == 0 => out.push_str("vii"),
            'I' if depth == 0 => out.push_str("VII"),
            '1' if depth == 0 => out.push('7'),
            _ => out.push(c),
        }
    }
    out
}

/// The value of a length register the preamble set before byte `at`, for
/// an enumitem key given as `\name`: the last `\setlength{\name}{<dimen>}`
/// (as a [`ListMargin::Fixed`]) or `\settowidth{\name}{<text>}` (as a
/// [`ListMargin::TextWidth`], measured by the typesetter; plain text only).
fn length_register(source: &str, at: usize, name: &str, size: u32, em_ex: Option<(f64, f64)>) -> Option<ListMargin> {
    let mut found: Option<(usize, ListMargin)> = None;
    let head = source.get(..at).unwrap_or(source);
    for (command, width) in [("setlength", false), ("settowidth", true)] {
        let mut from = 0;
        while let Some(pos) = find_command(&head[from..], command).map(|i| from + i) {
            from = pos + command.len() + 1;
            let rest = head[from..].trim_start();
            let (register, rest) = match rest.strip_prefix('{') {
                Some(inner) => match inner.find('}') {
                    Some(close) => (inner[..close].trim(), &inner[close + 1..]),
                    None => continue,
                },
                None if rest.starts_with('\\') => {
                    let end = rest[1..].find(|c: char| !c.is_ascii_alphabetic()).map_or(rest.len(), |e| e + 1);
                    (&rest[..end], &rest[end..])
                }
                None => continue,
            };
            if register != name {
                continue;
            }
            let rest = rest.trim_start();
            if !rest.starts_with('{') {
                continue;
            }
            let Some(close) = matching_brace(rest.as_bytes(), 0) else { continue };
            let arg = rest[1..close].trim();
            let value = if width {
                (!arg.contains('\\')).then(|| ListMargin::TextWidth(arg.to_string()))
            } else {
                parse_dimen_in(arg, size, em_ex).map(ListMargin::Fixed)
            };
            if let Some(value) = value.filter(|_| found.as_ref().is_none_or(|(at, _)| *at < pos)) {
                found = Some((pos, value));
            }
        }
    }
    found.map(|(_, value)| value)
}

/// `\leftmargin` of every list in `stack` (the compiler's frames, outermost
/// first): the class's `\leftmargin<i>` unless the frame's enumitem keys
/// (every matching `\setlist`, then the `\begin` options) set `leftmargin`
/// (`*` = the widest label's width plus `\labelsep`; a `<dimen>` or a
/// `\settowidth`/`\setlength` register as given).
///
/// Also returns the innermost itemize/enumerate's enumitem `labelsep=` (if
/// set) and `itemindent=` (points, zero if unset). With `leftmargin=*`
/// enumitem's `\enit@calcleft` solves `\leftmargin + \itemindent =
/// \labelindent + \labelwidth + \labelsep` for `\leftmargin`, so both keys
/// move the item text of that level ([`ListMargin::WidestSep`]); with any
/// other `leftmargin` it solves for `\labelindent`, and `labelsep` only moves
/// the label while `itemindent` moves the first line and its label.
///
/// `source` is read only by the `leftmargin=\<register>` arm
/// ([`length_register`], a prefix scan guarded by that rare key: the
/// compiler keeps a register value as `ListOption::Other`).
fn list_margins(index: &SourceIndex, stack: &[&ListFrame], source: &str, at: usize, size: u32, natbib_bib: bool, style: &Stylesheet) -> (Vec<ListMargin>, Option<f64>, f64) {
    let family = style.family;
    let em_ex = list_em_ex(size, family);
    // The class sets `\leftmargin<i>` while it loads, before `fontenc`, so
    // its `em` is OT1 `cmr`'s quad (Latin Modern's), not the EC font's
    // (`ecrm1095`'s quad is 0.06 pt smaller at 11 pt).
    let class_em_ex = if family == crate::fonts::Family::ComputerModern { list_em_ex(size, crate::fonts::Family::ComputerModernOt1) } else { em_ex };
    // beamer: `\leftmargin<i>` is 2em at every level
    // (`beamerbaselocalstructure.sty` 144-146).
    let beamer = style.is_beamer();
    let class_margin = |depth: usize| {
        let em = if beamer { 2.0 } else { article_leftmargin_em(depth) };
        ListMargin::Fixed(parse_dimen_in(&format!("{em}em"), size, class_em_ex).unwrap_or(0.0))
    };
    enum LeftMargin<'a> {
        Star,
        Pt(f64),
        Register(&'a str),
        Class,
    }
    let (mut labelsep_pt, mut itemindent_pt) = (None, 0.0);
    let margins = stack
        .iter()
        .enumerate()
        .map(|(i, frame)| {
            let depth = i + 1;
            let env = frame.environment.name();
            // `\list` resets `\itemindent` but not `\labelsep`, so a
            // `labelsep=` stays in force in the lists nested inside.
            itemindent_pt = 0.0;
            if env == "thebibliography" {
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
                return ListMargin::Widest(format!("[{}]", frame.widest_label.as_deref().unwrap_or("").trim()));
            }
            let mut leftmargin: Option<LeftMargin> = None;
            let mut label_key: Option<&str> = None;
            let mut widest: Option<&str> = None;
            let mut template: Option<&str> = None;
            let item_list = matches!(env, "itemize" | "enumerate");
            for option in &frame.options {
                match option {
                    ListOption::LeftMargin(ListLength::Star) => leftmargin = Some(LeftMargin::Star),
                    ListOption::LeftMargin(ListLength::Pt(pt)) => leftmargin = Some(LeftMargin::Pt(*pt)),
                    ListOption::LeftMargin(ListLength::Bang) => leftmargin = Some(LeftMargin::Class),
                    ListOption::Other { key, value: Some(value) } if key == "leftmargin" => {
                        leftmargin = Some(if value.starts_with('\\') { LeftMargin::Register(value) } else { LeftMargin::Class })
                    }
                    ListOption::Label(label) => label_key = Some(label),
                    ListOption::Widest(value) => widest = value.as_deref().filter(|v| !v.is_empty()),
                    ListOption::ShortLabel(label) => template = Some(label),
                    ListOption::LabelSep(ListLength::Pt(pt)) if item_list => labelsep_pt = Some(*pt),
                    ListOption::ItemIndent(ListLength::Pt(pt)) if item_list => itemindent_pt = *pt,
                    _ => {}
                }
            }
            // enumerate.sty (not enumitem): the template sets this depth's
            // `\leftmargin` to the width of the label at counter value 7
            // plus `\labelsep` (`\@@enum@`, enumerate.sty lines 79-82);
            // measured: `[(i)]` in an 11pt article gives `\leftmargin`
            // 25.74338pt = `\wd\hbox{(vii)}` 20.26837pt + 5.475pt where the
            // class's `\leftmargini` is 27.37506pt. Deeper than
            // `\@enumdepth` 4 is `\@toodeep`, an error, not a list.
            if let (Some(template), "enumerate", true) = (template, env, index.enumerate_package) {
                if depth <= 4 {
                    return ListMargin::Widest(enumerate_sty_widest(template));
                }
            }
            match leftmargin {
                Some(LeftMargin::Star) => {
                    let label = widest_label(env, depth, label_key, template, widest);
                    if labelsep_pt.is_none() && itemindent_pt == 0.0 {
                        ListMargin::Widest(label)
                    } else {
                        ListMargin::WidestSep { label, labelsep_pt, itemindent_pt }
                    }
                }
                Some(LeftMargin::Register(register)) => length_register(source, at, register, size, em_ex).unwrap_or_else(|| class_margin(depth)),
                Some(LeftMargin::Pt(pt)) => ListMargin::Fixed(pt),
                Some(LeftMargin::Class) | None => class_margin(depth),
            }
        })
        .collect();
    (margins, labelsep_pt, itemindent_pt)
}

fn list_em_ex(size: u32, family: crate::fonts::Family) -> Option<(f64, f64)> {
    match family {
        // document-style's size table is `cmr`'s (`\fontdimen6` 10.00002pt
        // at 10pt), which Latin Modern's `rm-lmr`/`ec-lm` reproduce.
        crate::fonts::Family::LatinModern | crate::fonts::Family::ComputerModernOt1 => {
            let base = match size {
                12 => flashtex_document_style::BaseSize::Pt12,
                11 => flashtex_document_style::BaseSize::Pt11,
                _ => flashtex_document_style::BaseSize::Pt10,
            };
            let font = flashtex_document_style::size_params(base).normal;
            Some((font.quad.0, font.x_height.0))
        }
        crate::fonts::Family::ComputerModern => ec_em_ex(size, family),
        // The stylesheet's family is the class family the preamble's
        // lengths were evaluated in; a named family is layered over it
        // (`Stylesheet::fontspec`) and never reaches here.
        crate::fonts::Family::Times | crate::fonts::Family::Named(_) => None,
    }
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

/// The line-breaking parameters a document sets for its whole length
/// (PLAN1 site 36): the last value the compiler recorded for each, among
/// the assignments made at the outermost level.
///
/// The compiler already runs `\sloppy`/`\fussy` and every explicit
/// `\tolerance`/`\emergencystretch` assignment, from the source, a macro
/// body, a class file or a package, and reports each as a
/// [`ParameterAssignment`] in document order. `until` is where TeX restores
/// the previous value: `None` exactly for an assignment made outside every
/// brace group, which therefore stays in force to the end of the document.
/// One inside a group (`{\sloppy ...}`, the `sloppypar` environment) has an
/// `until` and is skipped here, because the pipeline has no per-paragraph
/// break parameters to apply it to -- the same limitation the byte scan
/// this replaced had, now stated by the node stream rather than by a brace
/// counter over the entry source.
fn document_break_parameters(parameters: &[ParameterAssignment]) -> (Option<f64>, Option<f64>) {
    let (mut tolerance, mut emergency_stretch_pt) = (None, None);
    for assignment in parameters.iter().filter(|a| a.until.is_none()) {
        match assignment.parameter {
            BreakParameter::Tolerance(value) => tolerance = Some(f64::from(value)),
            BreakParameter::EmergencyStretch(pt) => emergency_stretch_pt = Some(pt),
            _ => {}
        }
    }
    (tolerance, emergency_stretch_pt)
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
// The per-offset reference that [`SourceIndex::in_theorem`] reproduces.
#[cfg_attr(not(test), allow(dead_code))]
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

/// `\url{...}` and `\nolinkurl{...}` (`url.sty`, which `hyperref` loads):
/// their argument is read as *raw source bytes* (url.sty makes every
/// character of a URL "other", so `%`, `#`, `_`, `&` and `\` inside it are
/// literal; the compiler does the same in `parser::url_argument`), and the
/// URL is set in the typewriter family. The compiler gives every run it
/// splits a URL into the span of the entire `\url{...}`
/// (`parser::push_url_text`), from the backslash through the closing brace.
fn url_command(name: &str) -> bool {
    matches!(name, "url" | "nolinkurl")
}

/// A verbatim construct's extent in the source: `whole` is every byte a
/// scan for markup must skip, `body` is the literal text inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VerbatimSpan {
    whole: (usize, usize),
    body: (usize, usize),
}

/// `\verb`/`\verb*` and `\lstinline`: a *delimited* argument, the next
/// character after the command (and after `\lstinline`'s optional
/// `[...]`) being the delimiter, which then closes the argument.
///
/// The argument is **opaque** — more so than a URL's, because `\verb` ends
/// at a *character*, not a brace. `\verb|{|` and `\verb|%|` are legal, and
/// a scan reading them as LaTeX corrupts its brace stack and starts a
/// comment that eats the rest of the line. The compiler gives
/// `Inline::Verbatim` the span of the entire `\verb|...|`, and its text is
/// **literal** (`TextStyle::literal`), which no font command implies:
/// `\texttt` ligates `--` and `\verb` must not.
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

/// Whether the inline whose span is `span` is one of the runs the compiler
/// splits a `\url{...}`/`\nolinkurl{...}` into (`parser::push_url_text`:
/// every run carries the span of the whole command, backslash through
/// closing brace).
fn url_run_at(source: &str, span: Span) -> bool {
    let Some(rest) = source.get(span.start..span.end) else { return false };
    let Some(rest) = rest.strip_prefix('\\') else { return false };
    let len = rest.bytes().take_while(u8::is_ascii_alphabetic).count();
    url_command(&rest[..len]) && rest[len..].trim_start().starts_with('{')
}

/// url.sty's `\UrlBreakPenalty` (`\binoppenalty`, 700).
const URL_BREAK_PENALTY: i32 = 700;
/// url.sty's `\UrlBigBreakPenalty` (`\relpenalty`, 500).
const URL_BIG_BREAK_PENALTY: i32 = 500;

/// The math atom class url.sty gives a URL character: `\UrlBreaks` are
/// binary operators (`\mathcode "2...`), `\UrlBigBreaks` (`:`) relations,
/// everything else ordinary. The set is the compiler's measured
/// `parser::URL_BREAK_AFTER` less `:`; `-` is ordinary (url.sty breaks at
/// a hyphen only under its `hyphens` option, and the compiler puts the
/// 0.5pt kern there instead).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UrlAtom {
    Ord,
    Bin,
    Rel,
}

fn url_atom(c: char) -> UrlAtom {
    match c {
        ':' => UrlAtom::Rel,
        '/' | '.' | '?' | '&' | '#' | '=' | '+' | '_' | ',' | ';' | '!' | '|' | '>' | ')' | ']' | '\'' | '@' => UrlAtom::Bin,
        _ => UrlAtom::Ord,
    }
}

/// The penalty TeX puts between the last character of `before` and the
/// character `next` of the same URL, or `None` when there is no legal
/// break there.
///
/// url.sty typesets a URL as a math list (`\Url@do`: `\mathsurround\z@`,
/// `\medmuskip\Urlmuskip`, `\thickmuskip\Urlmuskip`, both 0mu), so its
/// break points are TeX's own math-list penalties: `\binoppenalty` after a
/// Bin atom and `\relpenalty` after a Rel atom (TeX §761), subject to the
/// Bin→Ord demotions of §728-729 — a Bin preceded by Bin, Rel or nothing
/// is Ord, and a Bin followed by a Rel (or the end of the list) is Ord.
/// No penalty follows a Rel that another Rel follows.
///
/// pdflatex (TeX Live 2026, 11pt `article`, T1, hyperref) shows both the
/// list (`\showbox`) and the break (`\tracingparagraphs`):
///
/// ```text
/// .\T1/cmtt/m/n/10.95 s
/// .\glue(\thickmuskip) 0.0
/// .\T1/cmtt/m/n/10.95 :
/// .\glue(\thickmuskip) 0.0
/// .\T1/cmtt/m/n/10.95 /
/// .\glue(\medmuskip) 0.0
/// .\T1/cmtt/m/n/10.95 /
/// .\glue(\medmuskip) 0.0
/// .\T1/cmtt/m/n/10.95 e
/// ...
/// \T1/cmtt/m/n/10.95 https : / / example . org / ftxc /
/// @\penalty via @@1 b=7 p=700 d=490289
/// @@2: line 2.2 t=490458 -> @@1
///  issues$[][] \T1/cmr/m/n/10.95 rather than emailed directly.
/// ```
///
/// — the first `/` of `://` follows the relation and is Ord (no medmuskip
/// on its left), the second is Bin, and the paragraph breaks after the
/// last `/` at 700 (`fixtures/real-world/listings-manual`, page 1; the
/// whole URL is one box without the penalties, and `issues` cannot fit).
fn url_break_penalty(before: &str, next: char) -> Option<i32> {
    let mut r_type: Option<UrlAtom> = None;
    for c in before.chars() {
        let mut t = url_atom(c);
        if t == UrlAtom::Bin && r_type != Some(UrlAtom::Ord) {
            t = UrlAtom::Ord;
        }
        r_type = Some(t);
    }
    match (r_type?, url_atom(next)) {
        (UrlAtom::Rel, UrlAtom::Rel) => None,
        (UrlAtom::Rel, _) => Some(URL_BIG_BREAK_PENALTY),
        (UrlAtom::Bin, UrlAtom::Rel) => None,
        (UrlAtom::Bin, _) => Some(URL_BREAK_PENALTY),
        (UrlAtom::Ord, _) => None,
    }
}

/// Feeds a compiler text style to the paragraph cache key: every field
/// the items read, packed into one word (a derived `Hash` is a hasher call
/// per field, per word of the block, on every keystroke), the colour and
/// CJK run only when set.
fn hash_style(s: &flashtex_compiler::parser::TextStyle, h: &mut impl std::hash::Hasher) {
    use std::hash::Hash;
    let key = |k: crate::nfss::FontKey| (k.family as u64) | (k.series as u64) << 2 | (k.shape as u64) << 4;
    let flags = [s.bold, s.italic, s.slanted, s.small_caps, s.ams_tiny, s.medium, s.literal, s.italic_correction.before, s.italic_correction.after];
    use flashtex_compiler::parser::FontSizeLevel as L;
    // Four bits: `None` 0, the nine named levels 1..=9, an explicit
    // `\fontsize` 15 (its value follows).
    let size = match s.size {
        None => 0,
        Some(L::Tiny) => 1,
        Some(L::ScriptSize) => 2,
        Some(L::FootnoteSize) => 3,
        Some(L::Small) => 4,
        Some(L::Large1) => 5,
        Some(L::Large2) => 6,
        Some(L::Large3) => 7,
        Some(L::Huge1) => 8,
        Some(L::Huge2) => 9,
        Some(L::Explicit(_)) => 15,
    };
    let mut word = key(s.font.key) | s.font.undefined.map_or(0, |u| 0x80 | key(u)) << 7 | size << 15 | (s.family as u64) << 19;
    for (i, flag) in flags.into_iter().enumerate() {
        word |= u64::from(flag) << (21 + i);
    }
    h.write_u64(word);
    if let Some(L::Explicit(explicit)) = s.size {
        explicit.hash(h);
    }
    if s.color.is_some() || s.cjk.is_some() {
        (s.color, s.cjk).hash(h);
    }
}

/// [`hash_style`] for the glue in front of a run.
fn hash_glue(g: &Option<flashtex_compiler::parser::InterwordGlue>, h: &mut impl std::hash::Hasher) {
    match g {
        None => h.write_u8(0),
        Some(g) => {
            h.write_u8(1 + g.kind as u8);
            hash_style(&g.style, h);
        }
    }
}

/// The style of a run from the compiler's node (PLAN1 slice 2): the NFSS
/// font its font commands selected (`parser::TextStyle::font`, macro
/// expansion included), and the size, colour, CJK run, verbatim and
/// `\mdseries` marks it carries. A block's base font (a heading's
/// `\bfseries`) is not in it; `typeset::merge_style` adds that.
fn node_style(cs: &flashtex_compiler::parser::TextStyle, size: u32) -> TextStyle {
    let mut style = TextStyle { undefined: cs.font.undefined, ..TextStyle::default() }.with_key(cs.font.key);
    style.size_cpt = declared_size(cs.size, size);
    style.color = cs.color;
    style.cjk = cs.cjk;
    style.literal = cs.literal;
    style.medium = cs.medium;
    style
}

/// beamer's nested list bodies (`beamerfontthemedefault.sty` 105-106:
/// `itemize/enumerate subbody` is `size=\small`, `subsubbody`
/// `\footnotesize`; `beamerbaselocalstructure.sty` 197 and 256 apply the
/// font before `\list`): every paragraph of a level-2 item is set in
/// `\small`, of a level-3 item in `\footnotesize` (their labels with it:
/// `\usebeamerfont*{itemize subitem}` keeps the size). The leading is the
/// `\baselineskip` at the paragraph's `\par` (TeX §679): a nested
/// `\begin{itemize}` applies the inner size *before* `\@trivlist`'s
/// `\par`, so a paragraph followed by a deeper item takes the deeper
/// level's leading. Measured (probe deck `beamer-polish` p2, `\showoutput`
/// transcript of the same frame): the level-1 item's line ends under
/// `\glue(\baselineskip) 4.39584` + `\hbox(7.60416+0.0)` = 12pt (`\small`'s)
/// because level 2 opens next; the level-2 item under 4.0014 + 6.9986 = 11pt
/// (`\footnotesize`'s, level 3 opens next); the level-3 item under 3.30127
/// + 6.44873 + 1.25 = 11pt (its own `\end{itemize}`); "Another second level
/// item." under 3.86252 + 6.9986 + 1.13889 = 12pt.
fn beamer_nested_list_sizes(blocks: &mut [Block], base: flashtex_document_style::BaseSize) {
    use flashtex_document_style::{font_size, SizeName};
    let level_of = |b: &Block| match b {
        Block::Paragraph { list: Some(g), .. } if g.llap => Some(g.level),
        _ => None,
    };
    let size_for = |level: u8| match level {
        0 | 1 => None,
        2 => Some(font_size(base, SizeName::Small)),
        _ => Some(font_size(base, SizeName::FootnoteSize)),
    };
    for i in 0..blocks.len() {
        let Some(level) = level_of(&blocks[i]) else { continue };
        let next = blocks.get(i + 1).and_then(level_of).unwrap_or(0);
        let leading = size_for(level.max(next)).map(|f| f.baselineskip.0);
        let Block::Paragraph { sized, leading_pt, .. } = &mut blocks[i] else { continue };
        if let Some(f) = size_for(level) {
            if sized.is_none() {
                *sized = Some(SizedPara { size_pt: f.size.0, baselineskip_pt: f.baselineskip.0, parindent_em: None, vspace_after_em: 0.0, close_skip: None, strut: false });
            }
        }
        if leading_pt.is_none() {
            *leading_pt = leading;
        }
    }
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
        // `\fontsize{..}{<skip>}\selectfont`: its own `\f@baselineskip`.
        L::Explicit(size) => return Some(size.baselineskip_pt()),
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
    // `\fontsize{<size>}{..}\selectfont`: the engine's exact `\f@size`, as
    // the family's `.fd` loads it (`ExplicitSize::font_sp`).
    if let L::Explicit(size) = level {
        return (size.font_pt() * 100.0).round().clamp(1.0, f64::from(u16::MAX)) as u16;
    }
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
        L::Explicit(_) => unreachable!("returned above"),
    };
    table[row][col]
}

/// The byte ranges a scan for LaTeX markup must not look inside: every
/// verbatim construct (`verb_command`, `verbatim_environment`) plus the environments whose body pdflatex
/// never reads as markup though the compiler does not set them literally —
/// `minted` (a listing) and `comment` (verbatim.sty's discarded body).
/// `%` comments are not included; callers skip those line by line.
pub(crate) fn opaque_regions(source: &str) -> Vec<(usize, usize)> {
    literal_spans_of(source, |name| verbatim_environment(name) || matches!(name, "minted" | "comment"))
        .into_iter()
        .map(|s| s.whole)
        .collect()
}

fn literal_spans_of(source: &str, environment: fn(&str) -> bool) -> Vec<VerbatimSpan> {
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
                if environment(env) {
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
    macro_def(source, name, before).map(|d| &source[d.body])
}

/// The definition [`macro_body`] reads: the last one for `\<name>` before
/// byte `before`, or the first one anywhere.
fn macro_def(source: &str, name: &str, before: usize) -> Option<MacroDef> {
    // Within an adapt call the definitions of each document are indexed
    // once (FT-065: rescanning the whole source per invocation token made a
    // warm 500 KB request take seconds); elsewhere the source is scanned.
    let pick = |defs: &[MacroDef]| defs.iter().rev().find(|d| d.at < before).or(defs.first()).cloned();
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
        Some(index.get(name).and_then(|defs| pick(defs)))
    });
    match indexed {
        Some(found) => found,
        None => {
            let defs: Vec<MacroDef> = macro_definitions(source).into_iter().filter(|d| &source[d.name.clone()] == name).collect();
            pick(&defs)
        }
    }
}

/// Where TeX resumed reading the source after the user-macro invocation
/// whose `\name` is `inv`: the byte after its last argument (`[opt]` when
/// the definition gives a default, then the `{...}` groups, a control
/// sequence or a single character per undelimited argument, blanks between
/// them skipped), or after the name itself for a macro without arguments.
/// `None` when `inv` is not a user-macro invocation.
///
/// The compiler gives every token of a replacement text the `\name` span
/// alone, so the gap after such a token must not start at `inv.end`: that
/// would read the invocation's own argument bytes (`{a b}`) as if TeX had
/// set them between the replacement and the next token.
fn invocation_end(source: &str, inv: Span) -> Option<usize> {
    let name = control_word_at(source, inv.start, inv.end)?;
    let def = macro_def(source, name, inv.start)?;
    let bytes = source.as_bytes();
    let mut end = inv.end;
    let mut optional = def.optional;
    for _ in 0..def.args {
        let mut j = end;
        while j < bytes.len() && (bytes[j] as char).is_whitespace() {
            j += 1;
        }
        let absent_optional = optional && bytes.get(j) != Some(&b'[');
        optional = false;
        if absent_optional {
            continue;
        }
        let next = match bytes.get(j) {
            Some(b'[') => source[j..].find(']').map(|close| j + close + 1),
            Some(b'{') => matching_brace(bytes, j).map(|close| close + 1),
            Some(b'\\') => {
                let letters = bytes[j + 1..].iter().take_while(|b| b.is_ascii_alphabetic()).count();
                let symbol = source[j + 1..].chars().next().map_or(0, char::len_utf8);
                Some(j + 1 + if letters > 0 { letters } else { symbol })
            }
            Some(_) => source[j..].chars().next().map(|c| j + c.len_utf8()),
            None => None,
        };
        let Some(next) = next else { break };
        end = next;
    }
    Some(end)
}

/// One `\newcommand`-style definition: where its command starts, the
/// defined name's bytes, the replacement text inside its braces, and how
/// many arguments an invocation takes (`[n]`, or `\def`'s `#1#2`), the
/// first of which is optional when the definition gives it a default
/// (`[n][default]`; `default` is that text's bytes).
#[derive(Debug, Clone)]
struct MacroDef {
    at: usize,
    name: std::ops::Range<usize>,
    body: std::ops::Range<usize>,
    args: usize,
    optional: bool,
    default: Option<std::ops::Range<usize>>,
}

/// Per-thread definition indexes for the documents of the adapt call in
/// progress, keyed by the text's address and length. Only texts registered
/// by a live [`MacroDefsScope`] are indexed, and those are borrowed for the
/// whole scope, so a key can never name different bytes while it is used.
struct MacroDefsEntry {
    ptr: usize,
    len: usize,
    /// [`length_at_checked`]'s table-length indexes.
    lengths: LengthIndexes,
    index: Option<HashMap<String, Vec<MacroDef>>>,
    /// [`setlength`] answers already read, by length name and class size.
    setlengths: HashMap<(String, u32), Option<f64>>,
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
                lengths: LengthIndexes::default(),
                index: None,
                setlengths: HashMap::new(),
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
            let mut args = 0usize;
            let mut brackets = 0usize;
            let mut default = None;
            loop {
                skip_ws(&mut i);
                match bytes.get(i) {
                    Some(b'[') => match source[i..].find(']') {
                        Some(close) => {
                            if brackets == 0 {
                                args = source[i + 1..i + close].trim().parse().unwrap_or(0);
                            } else if brackets == 1 {
                                default = Some(i + 1..i + close);
                            }
                            brackets += 1;
                            i += close + 1;
                        }
                        None => break,
                    },
                    Some(b'#') => {
                        args += 1;
                        i += 2;
                    }
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
                args,
                optional: brackets > 1 && args > 0,
                default: default.filter(|_| brackets > 1 && args > 0),
            });
        }
    }
    defs.sort_by_key(|d| d.at);
    defs
}

/// For a macro invoked at `inv` (its `\name` span), the index (from 1) of
/// the argument whose bytes contain `at`, and the macro's name. When the
/// definition gives `#1` a default, a `[..]` right after the name is that
/// argument and the brace groups count from 2 (present or not: `\lb{a}`'s
/// group is `#2`); otherwise brackets are skipped.
fn macro_arg_index(source: &str, inv: Span, at: usize) -> Option<(&str, usize)> {
    let name = control_word_at(source, inv.start, inv.end)?;
    let optional = macro_def(source, name, inv.start).is_some_and(|d| d.optional);
    let bytes = source.as_bytes();
    let mut i = inv.end;
    let mut k = usize::from(optional);
    let mut first = true;
    loop {
        while i < bytes.len() && (bytes[i] as char).is_whitespace() {
            i += 1;
        }
        match bytes.get(i) {
            Some(b'[') => {
                let close = source[i..].find(']')?;
                if optional && first && at > i && at < i + close {
                    return Some((name, 1));
                }
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
        first = false;
    }
}

/// Whether the invocation at `inv` supplies its optional argument (a `[`
/// after the name, blanks skipped): if not, the tokens of `#1` come from
/// the definition's default text.
fn optional_given(source: &str, inv: Span) -> bool {
    let bytes = source.as_bytes();
    let mut i = inv.end;
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    bytes.get(i) == Some(&b'[')
}

/// Where the reader stands inside a macro's replacement text: the
/// invocation (`\name` span the compiler gives every replacement token)
/// and the byte offset in its definition body after the last token read.
#[derive(Debug, Clone, Copy)]
struct BodyCursor {
    inv: Span,
    at: usize,
    /// The reader stands inside the optional argument's default text
    /// (`[n][default]`, invoked without `[..]`), at this byte offset of it;
    /// `at` is then the body offset after the `#1` being read.
    default_at: Option<usize>,
}

impl BodyCursor {
    fn new(inv: Span, at: usize) -> BodyCursor {
        BodyCursor { inv, at, default_at: None }
    }
}

/// The bytes TeX read between the previous token and this one, in the
/// order it read them, so that [`gap_has_space`] can be
/// applied to them uniformly: the source between two exact spans; the
/// definition text between two tokens of one replacement (`\hfill
/// \normalfont[` between `#1` and `[`); and across the boundary between a
/// replacement token and an argument (the whitespace around `#k`). `text`
/// is the token's text, used to find its place in a definition. `None`
/// when nothing was read (the first token) or the place is unknown.
/// `foreign` gives, for an invocation whose macro is defined in another
/// document (a project package file), that document's text and a byte
/// position inside the definition (`Labels::foreign_definitions`); the
/// body is then read there instead of in `src`.
fn token_gap<'a>(
    src: &'a str,
    prev_end: Option<usize>,
    prev_span: Option<Span>,
    span: Span,
    text: Option<&str>,
    cursor: &mut Option<BodyCursor>,
    foreign: &dyn Fn(Span) -> Option<(&'a str, usize)>,
) -> Option<String> {
    let source_gap = |pe: usize, ps: Span| -> Option<String> {
        if ps.document != span.document {
            // Crossing an \input boundary: TeX reads the newline that ends
            // the \input line as a space.
            Some(" ".to_string())
        } else {
            // After a replacement token (the invocation's `\name` span)
            // the reader stands past the invocation's arguments, not past
            // the name: `"\ul{a b}"` has no blank between the underline
            // and the closing quote, whatever `{a b}` holds.
            let from = invocation_end(src, ps).unwrap_or(pe);
            (from <= span.start).then(|| src.get(from..span.start).unwrap_or("").to_string())
        }
    };
    let digits = |k: usize| 1 + k.to_string().len();
    // A word of a replacement text.
    if let Some(name) = control_word_at(src, span.start, span.end) {
        let (def_src, before) = foreign(span).unwrap_or((src, span.start));
        if let Some(def) = macro_def(def_src, name, before) {
            let body = &def_src[def.body.clone()];
            let (start, prefix, default_at) = match *cursor {
                Some(c) if c.inv == span => (c.at, None, c.default_at),
                _ => match prev_span.and_then(|ps| macro_arg_index(src, span, ps.start)) {
                    // The previous token was an argument of this invocation.
                    Some((_, k)) => (body.find(&format!("#{k}")).map_or(0, |p| p + digits(k)), None, None),
                    None => (0, prev_end.zip(prev_span).map(|(pe, ps)| source_gap(pe, ps).unwrap_or_default()), None),
                },
            };
            let Some(text) = text else {
                // Glue, math or a box (`\uline{#1}`, `\colorbox`, `$..$`)
                // of a replacement: it has no text to search for, so it is
                // taken to be the first thing that can open one after the
                // cursor -- a control word, `\(`, `\[` or `$` -- and the
                // bytes read are the source before the invocation plus the
                // body up to there (`"\ul{a b}"` reads `""`, no blank). The
                // cursor moves past the construct when its end is clear, so
                // a word after it is not searched inside it. When nothing in
                // the body can open one, separate tokens of one replacement
                // are still taken as spaced.
                let Some(pos) = box_start(body, start) else {
                    *cursor = Some(BodyCursor::new(span, start));
                    return prev_end.map(|_| " ".to_string());
                };
                let gap = &body[start..pos];
                *cursor = Some(BodyCursor { inv: span, at: box_end(body, pos).unwrap_or(pos), default_at: None });
                return Some(match prefix {
                    Some(before) => format!("{before}{gap}"),
                    None => gap.to_string(),
                });
            };
            // A control word (the glue arms pass `\hfill`/`\quad`/...) is
            // matched as a whole word, so `\hfil` never stops at `\hfill`.
            let find_text = |rest: &str| match text.strip_prefix('\\') {
                Some(name) if name.chars().all(|c| c.is_ascii_alphabetic()) => find_command(rest, name),
                _ => rest.find(text),
            };
            // The default of the optional argument (`[n][default]`, invoked
            // without `[..]`): the compiler spans its tokens at the
            // invocation like the body's, but they are not in the body --
            // TeX reads them where the body reaches `#1`.
            let default = def.default.clone().filter(|_| !optional_given(src, span)).map(|r| &def_src[r]);
            if let Some(default) = default {
                // Standing inside the default: the next word of it, if any,
                // is found there before the body is searched.
                if let Some(d) = default_at {
                    if let Some(p) = default.get(d..).and_then(find_text) {
                        let gap = &default[d..d + p];
                        *cursor = Some(BodyCursor { inv: span, at: start, default_at: Some(d + p + text.len()) });
                        return Some(gap.to_string());
                    }
                }
                // Reaching `#1` before the word's own bytes in the body: the
                // word is the default's, and the gap is the body up to `#1`
                // plus the default up to the word.
                let param = body.get(start..).and_then(|rest| rest.find("#1")).map(|p| start + p);
                let in_body = body.get(start..).and_then(find_text).map(|p| start + p);
                if let (Some(param), Some(p)) = (param, find_text(default)) {
                    if in_body.is_none_or(|w| param < w) {
                        let body_gap = &body[start..param];
                        let gap = &default[..p];
                        *cursor = Some(BodyCursor { inv: span, at: param + digits(1), default_at: Some(p + text.len()) });
                        return Some(match prefix {
                            Some(before) => format!("{before}{body_gap}{gap}"),
                            None => format!("{body_gap}{gap}"),
                        });
                    }
                }
            }
            match body.get(start..).and_then(find_text) {
                Some(p) => {
                    let pos = start + p;
                    let gap = &body[start..pos];
                    *cursor = Some(BodyCursor { inv: span, at: pos + text.len(), default_at: None });
                    return Some(match prefix {
                        Some(before) => format!("{before}{gap}"),
                        None => gap.to_string(),
                    });
                }
                None => {
                    // Not found verbatim (ligatures rewrote it).
                    *cursor = Some(BodyCursor::new(span, start));
                    return prev_end.map(|_| " ".to_string());
                }
            }
        }
    }
    // An argument of the invocation being read.
    if let Some(c) = *cursor {
        if let Some((name, k)) = macro_arg_index(src, c.inv, span.start) {
            let (def_src, before) = foreign(c.inv).unwrap_or((src, c.inv.start));
            if let Some(body) = macro_body(def_src, name, before) {
                if let Some(p) = body.get(c.at..).and_then(|rest| rest.find(&format!("#{k}"))) {
                    let pos = c.at + p;
                    let gap = body[c.at..pos].to_string();
                    *cursor = Some(BodyCursor::new(c.inv, pos + digits(k)));
                    return Some(gap);
                }
                // Further tokens of the same argument: the source between
                // them (the cursor stays after `#k`).
                *cursor = Some(BodyCursor::new(c.inv, c.at));
                return prev_end.zip(prev_span).and_then(|(pe, ps)| source_gap(pe, ps));
            }
        }
    }
    *cursor = None;
    prev_end.zip(prev_span).and_then(|(pe, ps)| source_gap(pe, ps))
}

/// The first byte at or after `from` in a macro body that can open a
/// textless construct of the replacement -- a control word (`\uline{#1}`,
/// `\begin{tabular}`), `\(`, `\[` or `$` -- or `None` when nothing can.
/// Control symbols (`\ `, `\,`, `\\`) are part of the gap before it, so a
/// control space still counts as a blank.
fn box_start(body: &str, from: usize) -> Option<usize> {
    let bytes = body.as_bytes();
    let mut i = from;
    while i < bytes.len() {
        match bytes[i] {
            b'$' => return Some(i),
            b'\\' => match bytes.get(i + 1) {
                Some(b'(') | Some(b'[') => return Some(i),
                Some(c) if c.is_ascii_alphabetic() => return Some(i),
                _ => i += 2,
            },
            _ => i += 1,
        }
    }
    None
}

/// The byte after the construct [`box_start`] found at `start`: past the
/// matching `$`/`$$`, `\)`, `\]` or `\end{<env>}`, or past a control word
/// and the `[..]`/`{..}` arguments that follow it. `None` when the end is
/// not clear (an unbalanced body).
fn box_end(body: &str, start: usize) -> Option<usize> {
    let bytes = body.as_bytes();
    let rest = body.get(start..)?;
    if let Some(after) = rest.strip_prefix("$$") {
        return after.find("$$").map(|p| start + 2 + p + 2);
    }
    if let Some(after) = rest.strip_prefix('$') {
        return after.find('$').map(|p| start + 1 + p + 1);
    }
    if let Some(after) = rest.strip_prefix("\\(") {
        return after.find("\\)").map(|p| start + 2 + p + 2);
    }
    if let Some(after) = rest.strip_prefix("\\[") {
        return after.find("\\]").map(|p| start + 2 + p + 2);
    }
    let name = rest.strip_prefix('\\')?;
    let len = name.bytes().take_while(u8::is_ascii_alphabetic).count();
    let mut i = start + 1 + len;
    if &name[..len] == "begin" {
        let open = body[i..].find('{')? + i;
        let close = matching_brace(bytes, open)?;
        let env = &body[open + 1..close];
        let end = format!("\\end{{{env}}}");
        return body[close..].find(&end).map(|p| close + p + end.len());
    }
    let mut consumed = false;
    loop {
        let mut j = i;
        while j < bytes.len() && (bytes[j] as char).is_whitespace() {
            j += 1;
        }
        match bytes.get(j) {
            Some(b'[') => i = j + body[j..].find(']')? + 1,
            Some(b'{') => i = matching_brace(bytes, j)? + 1,
            // The blanks after a bare control word are skipped with it; the
            // ones after an argument's `}` are read.
            _ => return Some(if consumed { i } else { j }),
        }
        consumed = true;
    }
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

/// Without package gating, match only the kernel definition.
#[cfg(not(feature = "compiler-package-gating"))]
fn kern_amount_matches(spelling: &str, amount: &TextDimen) -> bool {
    flashtex_compiler::text_builtins::text_kern(spelling, false).as_ref() == Some(amount)
}

/// [`gap_has_space`] for the bytes after a control word: the whitespace
/// TeX eats right after the word does not count.
fn gap_has_space_after_control_word(rest: &str) -> bool {
    let rest = rest.trim_start_matches([' ', '\t']);
    // A newline right after the word is eaten too (state S ignores the end
    // of the line, TeX §347), and the next line then begins in state N,
    // where its indentation is skipped as well (§344): `\quad\n  \href`
    // holds no space token. Reading that indentation as one put an extra
    // interword space after each line-ending `\quad` of
    // `fixtures/real-world/cv`'s contact line (both ends 3.62 bp out).
    let rest = match rest.strip_prefix('\n') {
        Some(next_line) => next_line.trim_start_matches([' ', '\t']),
        None => rest,
    };
    gap_has_space(rest)
}

/// The sum of every `\vspace{<dimen>}`/`\vspace*{<dimen>}` in `gap`, in
/// points; `None` when there is none or one does not parse.
///
/// `pub(crate)` so [`crate::abstractenv`] can re-derive how much of a
/// paragraph's leading skip came from inside a `\begin{abstract}` rather
/// than before it.
pub(crate) fn vspace_in_gap(gap: &str, size: u32) -> Option<f64> {
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

/// A rigid `\vspace` block, written through one constructor so the crate
/// builds against a pinned compiler with or without `VSpace`'s glue
/// components (`stretch_pt`/`shrink_pt`, compiler PR #606). Every caller
/// here lowers a gap the pipeline computed itself (`\opening`'s skips), so
/// the glue is zero either way and the two arms are the same block.
fn vspace_block(pt: f64) -> CBlock {
    #[cfg(feature = "compiler-node-surface")]
    {
        CBlock::VSpace { pt, stretch_pt: 0.0, shrink_pt: 0.0 }
    }
    #[cfg(not(feature = "compiler-node-surface"))]
    {
        CBlock::VSpace { pt }
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
    /// `\maketitle` (laid out from the compiler's `TitleBlock`).
    MakeTitle,
    /// book.cls `\frontmatter`/`\mainmatter`/`\backmatter`.
    Matter(Matter),
    /// `\tableofcontents`, `\listoffigures`, `\listoftables`,
    /// `\lstlistoflistings`.
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

/// The `\pagestyle`/`\thispagestyle` switches of one document, as
/// [`BodyCommand`]s at their own byte positions (PLAN1 site 32).
///
/// The compiler runs the command and emits a zero-width
/// `Inline::PageStyle` marker wherever it ran -- from the source, a macro
/// body or a project `.sty` -- so this reads the switch off the node
/// stream instead of finding `\pagestyle{` in the bytes, which for a
/// macro-produced command is not what stands at the span.
///
/// `from` is where the document's body starts, so a preamble
/// `\pagestyle` is left to `DocumentSetup::from_preamble` exactly as the
/// byte scan left it (the scan began at `\begin{document}`). A style
/// name this pipeline does not model (`fancy`, or an unknown one) yields
/// no command, as `PageStyle::parse` returning `None` did.
fn page_style_commands(blocks: &[CBlock], document: DocumentId, from: usize) -> Vec<BodyCommand> {
    let mut out = Vec::new();
    for block in blocks {
        for inline in inlines_of(block) {
            let Inline::PageStyle { style, this_page, span } = inline else { continue };
            if span.document != document || span.start < from {
                continue;
            }
            use flashtex_compiler::parser::PageStyleName;
            let Some(ps) = (match style {
                PageStyleName::Empty => Some(PageStyle::Empty),
                PageStyleName::Plain => Some(PageStyle::Plain),
                PageStyleName::Headings => Some(PageStyle::Headings),
                PageStyleName::MyHeadings => Some(PageStyle::MyHeadings),
                PageStyleName::Fancy | PageStyleName::Unknown => None,
            }) else {
                continue;
            };
            let event = if *this_page { ChromeEvent::ThisPageStyle(ps) } else { ChromeEvent::PageStyle(ps) };
            out.push(BodyCommand {
                start: span.start,
                end: span.end,
                kind: BodyKind::Event(event),
            });
        }
    }
    out.sort_by_key(|c| c.start);
    out
}

/// The `\markboth`/`\markright` of one document, as [`BodyCommand`]s at
/// their own byte positions (PLAN1 site 17).
///
/// The compiler runs the command and leaves an `Inline::Mark` marker
/// wherever it ran -- from the source, a macro body or a project `.sty` --
/// carrying the marks' own already-expanded content, so this reads them
/// off the node stream instead of finding `\markboth{` in the bytes,
/// which for a macro-produced command is not what stands at the span.
///
/// The mark's text is [`mark_title`], the same flattening a heading's own
/// `\sectionmark` goes through (PLAN1 site 19), so the two agree.
fn mark_commands(texts: &[&str], blocks: &[CBlock], document: DocumentId, from: usize) -> Vec<BodyCommand> {
    let mut out = Vec::new();
    for block in blocks {
        for inline in inlines_of(block) {
            let Inline::Mark { left, right, span } = inline else { continue };
            if span.document != document || span.start < from {
                continue;
            }
            let event = match left {
                Some(left) => ChromeEvent::MarkBoth(mark_title(texts, left), mark_title(texts, right)),
                None => ChromeEvent::MarkRight(mark_title(texts, right)),
            };
            out.push(BodyCommand { start: span.start, end: span.end, kind: BodyKind::Event(event) });
        }
    }
    out.sort_by_key(|c| c.start);
    out
}

/// The `\tableofcontents`/`\listoffigures`/`\listoftables`/
/// `\lstlistoflistings` of one document, as [`BodyCommand`]s at their own
/// byte positions (PLAN1 site 39).
///
/// The compiler runs the command and pushes a `Block::TableOfContents`
/// carrying which list it is, wherever it ran -- from the source, a macro
/// body or a project `.sty` -- so this reads the request off the node
/// stream instead of finding `\tableofcontents` in the bytes, which for a
/// macro-produced command is not what stands at the span.
///
/// `from` is where the document's body starts, matching the byte scan this
/// replaces (it began at `\begin{document}`); a contents list in the
/// preamble is not a thing LaTeX sets either.
fn contents_list_commands(blocks: &[CBlock], document: DocumentId, from: usize) -> Vec<BodyCommand> {
    let mut out = Vec::new();
    for block in blocks {
        let CBlock::TableOfContents { span, list, .. } = block else { continue };
        if span.document != document || span.start < from {
            continue;
        }
        use flashtex_compiler::parser::ContentsList;
        let kind = match list {
            ContentsList::Toc => crate::toc::ListKind::Toc,
            ContentsList::Lof => crate::toc::ListKind::Lof,
            ContentsList::Lot => crate::toc::ListKind::Lot,
            ContentsList::Lol => crate::toc::ListKind::Lol,
        };
        out.push(BodyCommand { start: span.start, end: span.end, kind: BodyKind::ContentsList(kind) });
    }
    out.sort_by_key(|c| c.start);
    out
}

/// `\maketitle`, `\input`/`\include`, (when
/// the class has chapters) `\chapter` and (book)
/// `\frontmatter`/`\mainmatter`/`\backmatter` after `\begin{document}`, in
/// source order, skipping comments.
///
/// `\pagestyle`/`\thispagestyle` used to be here too; they are
/// [`page_style_commands`] now (PLAN1 site 32), the four contents-list
/// commands are [`contents_list_commands`] (PLAN1 site 39), and
/// `\markboth`/`\markright` are [`mark_commands`] (PLAN1 site 17).
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
            // `\pagestyle`/`\thispagestyle` are not here: they come from
            // the compiler's `Inline::PageStyle` (`page_style_commands`,
            // PLAN1 site 32), which the parser emits wherever the command
            // ran, including from a macro body.
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

/// Whether an [`BodyKind::Input`] command is `\include` (not `\input`).
fn is_include(source: &str, cmd: &BodyCommand) -> bool {
    source.get(cmd.start..cmd.end).is_some_and(|c| c.starts_with("\\include"))
}

/// `\include`/`\input` nesting the reading order follows (the compiler's
/// own limit is far above anything a real project needs).
const READING_DEPTH_LIMIT: usize = 32;

/// The document body in the order pdfLaTeX reads it: byte ranges of the
/// documents (`paths`, parallel to `texts`), the entry one split at every
/// `\input{..}`/`\include{..}` of [`body_commands`] with the read file's
/// own ranges in between, recursively. A file is found as the compiler's
/// `include` finds it (the path as written, then with `.tex`); an unknown
/// file, a cycle, and an `\include` the preamble's last `\includeonly`
/// does not name read nothing. A document never read has no range: a
/// fresh run has no `.aux` of an excluded file to restore its counters
/// from, so it steps none.
pub fn reading_order(texts: &[&str], paths: &[&str], entry: usize) -> Vec<Span> {
    let only = texts.get(entry).and_then(|t| includeonly(t));
    let mut out = Vec::new();
    let mut stack = Vec::new();
    read_document(entry, texts, paths, only.as_deref(), &mut stack, &mut out);
    out
}

fn read_document(d: usize, texts: &[&str], paths: &[&str], only: Option<&[String]>, stack: &mut Vec<usize>, out: &mut Vec<Span>) {
    let Some(text) = texts.get(d).copied() else { return };
    stack.push(d);
    let mut from = 0;
    for cmd in body_commands(text, false, false).iter().filter(|c| matches!(c.kind, BodyKind::Input)) {
        let written = &text[cmd.start..cmd.end];
        let include = is_include(text, cmd);
        let requested = written.find('{').map_or("", |open| written[open + 1..written.len() - 1].trim());
        let listed = |name: &str| name == requested || name.strip_suffix(".tex") == Some(requested) || requested.strip_suffix(".tex") == Some(name);
        if include && only.is_some_and(|names| !names.iter().any(|n| listed(n))) {
            continue;
        }
        let with_tex = format!("{requested}.tex");
        let Some(target) = paths.iter().position(|p| *p == requested).or_else(|| paths.iter().position(|p| *p == with_tex)) else {
            continue;
        };
        if stack.contains(&target) || stack.len() > READING_DEPTH_LIMIT {
            continue;
        }
        out.push(Span::in_document(DocumentId(d), from, cmd.start));
        read_document(target, texts, paths, only, stack, out);
        from = cmd.end;
    }
    out.push(Span::in_document(DocumentId(d), from, text.len()));
    stack.pop();
}

/// The names of the last preamble `\includeonly{a,b}` (`None` without
/// one; an empty list reads no `\include` at all).
fn includeonly(entry: &str) -> Option<Vec<String>> {
    let preamble = &entry[..entry.find("\\begin{document}").unwrap_or(entry.len())];
    let mut found = None;
    for line in preamble.lines() {
        let code = line.char_indices().find(|&(i, c)| c == '%' && !line[..i].ends_with('\\')).map_or(line, |(i, _)| &line[..i]);
        let mut rest = code;
        while let Some(at) = rest.find("\\includeonly") {
            rest = &rest[at + "\\includeonly".len()..];
            let arg = rest.trim_start();
            if let Some(close) = arg.strip_prefix('{').and_then(|a| a.find('}')) {
                found = Some(arg[1..close + 1].split(',').map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect());
            }
        }
    }
    found
}

/// Where `(document, offset)` falls in `order` ([`reading_order`]): the
/// bytes read before it. `None` for bytes never read.
pub fn reading_position(order: &[Span], document: DocumentId, offset: usize) -> Option<usize> {
    let mut before = 0;
    for s in order {
        if s.document == document && (s.start..s.end).contains(&offset) {
            return Some(before + offset - s.start);
        }
        before += s.end - s.start;
    }
    None
}

/// Drops the compiler's text for the arguments of `\chapter`, `\part`,
/// `\addcontentsline`, `\setcounter{page}` and `\pagenumbering` (it sets
/// them as body text), and the paragraphs left empty.
///
/// `\markboth`/`\markright` are no longer among them: the compiler
/// consumes their arguments itself now (PLAN1 site 17).
fn strip_command_text(blocks: &mut Vec<(CBlock, ParLeading)>, document: DocumentId, commands: &[BodyCommand]) {
    let ranges: Vec<(usize, usize)> = commands
        .iter()
        .filter(|c| matches!(c.kind, BodyKind::Chapter { .. } | BodyKind::Part { .. } | BodyKind::AddContentsLine { .. } | BodyKind::Event(ChromeEvent::SetPage(_) | ChromeEvent::PageNumbering(_))))
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
/// A heading's title as the plain text of its `\sectionmark`: the
/// compiler's text runs, a space wherever TeX appends interword glue
/// (PLAN1 site 19: a title from a macro body reads as what the macro set).
/// An inline that is not text (a formula, a box) keeps the source text of
/// its span, as the whole title did before.
fn mark_title(texts: &[&str], content: &[Inline]) -> String {
    let mut out = String::new();
    let mut prev_end: Option<Span> = None;
    for inline in content {
        match inline {
            Inline::Text { text, glue_before, space_before, .. } => {
                if glue_before.is_some() || *space_before {
                    out.push(' ');
                }
                out.push_str(text);
            }
            Inline::Label { .. } => {}
            other => {
                let span = inline_span(other);
                let gap = prev_end.filter(|p| p.document == span.document && p.end <= span.start).and_then(|p| texts.get(span.document.0)?.get(p.end..span.start));
                if gap.is_some_and(|g| g.chars().any(char::is_whitespace)) {
                    out.push(' ');
                }
                out.push_str(texts.get(span.document.0).and_then(|t| t.get(span.start..span.end)).unwrap_or(""));
            }
        }
        prev_end = Some(inline_span(inline));
    }
    plain_text(&out)
}

fn plain_text(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// What the pipeline still reads from one document's source around the
/// compiler's styles: its size environments. Fonts, verbatim and the
/// italic corrections come from the nodes (`parser::TextStyle`).
#[derive(Debug, Clone, Default)]
struct Styles {
    /// Size environments (`\begin{small}...\end{small}`), in order of their
    /// `\begin`: the byte of the `\begin`, the body's bytes and the size.
    size_envs: Vec<SizeEnv>,
}

/// One `\begin{<size>}...\end{<size>}` (`size_environments`).
#[derive(Debug, Clone, Copy)]
struct SizeEnv {
    begin: usize,
    body: (usize, usize),
    level: Option<flashtex_compiler::parser::FontSizeLevel>,
}

/// The size a size-declaration control word selects: `Some(None)` for
/// `\normalsize`, `None` for any other word.
fn size_command_level(name: &str) -> Option<Option<flashtex_compiler::parser::FontSizeLevel>> {
    use flashtex_compiler::parser::FontSizeLevel as L;
    Some(match name {
        "tiny" => Some(L::Tiny),
        "scriptsize" => Some(L::ScriptSize),
        "footnotesize" => Some(L::FootnoteSize),
        "small" => Some(L::Small),
        "normalsize" => None,
        "large" => Some(L::Large1),
        "Large" => Some(L::Large2),
        "LARGE" => Some(L::Large3),
        "huge" => Some(L::Huge1),
        "Huge" => Some(L::Huge2),
        _ => return None,
    })
}

/// Every `\begin{<size>}...\end{<size>}` in `source`. LaTeX lets any
/// declaration be used as an environment (`\begin{small}` runs `\small`
/// in the environment's group); the compiler reports these as unknown
/// environments and sets their bodies at the surrounding size.
fn size_environments(source: &str) -> Vec<SizeEnv> {
    let mut out = Vec::new();
    if !source.contains("\\begin{") {
        return out;
    }
    let mut open: Vec<(usize, usize, &str)> = Vec::new();
    let mut scan = CmdScan::new(source);
    while let Some((at, cmd, _)) = scan.next() {
        if cmd != "begin" && cmd != "end" {
            continue;
        }
        let after = at + 1 + cmd.len();
        let rest = &source[after..];
        let trimmed = rest.trim_start();
        let Some(inner) = trimmed.strip_prefix('{') else { continue };
        let Some(close) = inner.find('}') else { continue };
        let name = inner[..close].trim();
        let Some(level) = size_command_level(name) else { continue };
        let end = after + (rest.len() - trimmed.len()) + 1 + close + 1;
        if cmd == "begin" {
            open.push((at, end, name));
        } else if let Some(k) = open.iter().rposition(|(_, _, n)| *n == name) {
            let (begin, body_start, _) = open.remove(k);
            out.push(SizeEnv { begin, body: (body_start, at), level });
        }
    }
    out.sort_by_key(|e| e.begin);
    out
}

impl Styles {
    fn new(source: &str) -> Styles {
        Styles { size_envs: size_environments(source) }
    }

    /// The size a size environment gives the text at byte `at` when the
    /// compiler has no size declaration there (it does not know the
    /// environments): the innermost one whose body holds `at`, unless a size
    /// declaration made inside that body is still in force at `at` -- then
    /// the compiler's own scoping already has the size.
    fn size_env_at(&self, source: &str, at: usize) -> Option<flashtex_compiler::parser::FontSizeLevel> {
        let env = self.size_envs.iter().rev().find(|e| e.body.0 <= at && at < e.body.1)?;
        let mut scan = CmdScan::new(source.get(env.body.0..at)?);
        while let Some((off, cmd, _)) = scan.next() {
            if size_command_level(cmd).is_some() && group_open_between(source, env.body.0 + off + 1 + cmd.len(), at) {
                return None;
            }
        }
        env.level
    }
}

/// Whether the bytes between two consecutive inlines contain an interword
/// space under TeX's rules (braces and control words produce none; spaces
/// after a control word are eaten; comments swallow their newline).
/// Whether `ch` is one CJK.sty reads in a `CJK` environment (CJKutf8.sty
/// 30-60: every non-ASCII character inputenc's `utf8.def`/`*.dfu` tables do
/// not declare), so that `\CJK@ignorespaces` follows it. The project's own
/// `\DeclareUnicodeCharacter`s are not consulted here (they would make the
/// character inputenc's); `typeset` classifies with them.
fn cjk_read_char(ch: char) -> bool {
    !ch.is_ascii() && flashtex_tex_text_encoding::unicode::lookup_declared(ch).is_none()
}

/// The interword spaces of a gap that crosses a `CJK` environment boundary,
/// or `None` for a gap without one (the ordinary [`gap_has_space`] rule).
///
/// `\begin{CJK}[..]{..}{..}` and `\end{CJK}` (CJK.sty 1084-1094) expand
/// to assignments and typeset nothing, and `\end` does not `\ignorespaces`
/// (latex.ltx's `\@ignore` is only set by lists), so a blank on each side
/// of the command is a space token of its own: pdflatex's `\showoutput`
/// of `です。⏎\end{CJK}⏎(` has two `\glue 3.63054` in front of the `(`.
/// `after_nospace_cjk` is `CJK*`/`\CJKnospace` after a CJK character: its
/// `\ignorespaces` expands `\end{CJK*}` on its way to the next non-blank
/// token and eats only the blank before the command (CJK.sty 879-882).
fn cjk_gap_spaces(gap: &str, after_nospace_cjk: bool) -> Option<u8> {
    let at = [("\\end{CJK}", false), ("\\end{CJK*}", false), ("\\begin{CJK}", true), ("\\begin{CJK*}", true)]
        .iter()
        .filter_map(|(needle, begin)| gap.find(needle).map(|i| (i, needle.len(), *begin)))
        .min()?;
    let (before, rest) = gap.split_at(at.0);
    let mut rest = &rest[at.1..];
    if before.contains("\\begin{") || before.contains("\\end{") {
        // More than one environment command in one gap: not modelled.
        return None;
    }
    if at.2 {
        // `\begin{CJK}`'s arguments: an optional `[..]` and two `{..}`.
        if let Some(close) = rest.strip_prefix('[').and_then(|t| t.find(']')) {
            rest = &rest[close + 2..];
        }
        for _ in 0..2 {
            let close = rest.strip_prefix('{').and_then(|t| t.find('}'))?;
            rest = &rest[close + 2..];
        }
    }
    let first = gap_has_space(before) && !after_nospace_cjk;
    let second = gap_has_space(rest);
    Some(u8::from(first) + u8::from(second))
}

/// Whether the source gap between two inlines holds an interword space.
/// Whitespace inside a brace group the gap itself opens (the `\hangfrom`
/// label's own trailing gap, read back as `{Label. }`) is the group's
/// own material -- TeX tokenizes it inside the group, so it never
/// separates the surrounding tokens -- and does not count. Depth goes
/// negative through a gap that only closes groups (`} {`); only depth
/// zero and below is interword. `\{`/`\}` never reach the brace arm (the
/// `\\` arm consumes the escape), so they change nothing.
fn gap_has_space(gap: &str) -> bool {
    let bytes = gap.as_bytes();
    let mut depth = 0i32;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
                depth -= 1;
                i += 1;
            }
            b'[' | b']' => i += 1,
            b'%' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                i += 1;
                // The comment ate the line end, and the next line starts
                // in state N: its leading blanks are skipped too
                // (`page.}%⏎  \only<2>{On` has no space before `On`).
                while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
                    i += 1;
                }
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
            // Interword only at depth zero and below (see above); a
            // group the gap opens owns its blanks.
            c if (c as char).is_whitespace() => {
                if depth <= 0 {
                    return true;
                }
                i += 1;
            }
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
    space_factor_with(ch, previous, 3000)
}

/// article.cls's `thebibliography` (natbib's alike) runs `\sfcode`\.\@m`
/// after its `\list`: a period leaves the space factor at 1000, so the
/// space after `Knuth.` or `TeXbook.` in an entry is `\fontdimen2` with
/// the ordinary stretch and shrink, no `\fontdimen7` (1.3 pt at 12 pt;
/// two of them pushed `Practice` to the next line of input-bibliography's
/// entry [2], `\tracingparagraphs` @@1 b=99 in pdfTeX's first pass).
/// `?`, `!` and the rest keep plain.tex's codes. Only a factor the
/// ordinary table raised above 1000 is revisited, so a control space
/// (`\ `, factor 1000 by construction) is untouched.
pub fn bibliography_space_factors(items: &mut [Item]) {
    let mut word_factor: Option<u32> = None;
    for item in items.iter_mut() {
        match item {
            Item::Word(w) => {
                let mut factor = 1000u32;
                for ch in w.segments.iter().flat_map(|s| s.text.chars()) {
                    factor = space_factor_with(ch, factor, 1000);
                }
                word_factor = Some(factor);
            }
            Item::Space { factor, .. } => {
                if let Some(f) = word_factor.take() {
                    if *factor > 1000 {
                        *factor = f.min(*factor);
                    }
                }
            }
            _ => word_factor = None,
        }
    }
}

/// [`space_factor`] with the `\sfcode` of `.` (and of `…`, which ends in
/// one) as `period_code`: plain.tex's 3000, or 1000 under `\sfcode`\.\@m`.
fn space_factor_with(ch: char, previous: u32, period_code: u32) -> u32 {
    let code = match ch {
        // `…` is `\textellipsis`, whose last character is a period
        // (`.\kern\fontdimen3\font` three times), so it leaves the period's
        // space factor behind exactly as a typed `.` does: pdflatex sets
        // `ellipsis… here` with a 5.213 bp space at 12 pt
        // (`\fontdimen2 + \fontdimen7`), not the 3.902 bp of `\fontdimen2`
        // alone.
        '.' | '\u{2026}' => period_code,
        '?' | '!' => 3000,
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
        return items_from_inlines_styled(texts, inlines, styles, labels, size, heading, compiler_weight, true);
    };
    // Table items nest item lists the relocation does not walk.
    if inlines.iter().any(|i| matches!(i, Inline::Tabular(_))) {
        return items_from_inlines_styled(texts, inlines, styles, labels, size, heading, compiler_weight, true);
    }
    let Some(first) = inlines.first().map(inline_span) else {
        return items_from_inlines_styled(texts, inlines, styles, labels, size, heading, compiler_weight, true);
    };
    let document = first.document;
    let mut start = first.start;
    let mut end = first.end;
    for i in inlines {
        let s = inline_span(i);
        if s.document != document {
            return items_from_inlines_styled(texts, inlines, styles, labels, size, heading, compiler_weight, true);
        }
        start = start.min(s.start);
        end = end.max(s.end);
    }
    let Some(src) = texts.get(document.0) else {
        return items_from_inlines_styled(texts, inlines, styles, labels, size, heading, compiler_weight, true);
    };
    // Macro replacement text carries the invocation's span: the spacing
    // and weight of its words come from the definition (`macro_body`), so
    // a block holding one cannot be keyed by its own bytes alone.
    if inlines.iter().any(|i| is_invocation_span(src, inline_span(i))) {
        return items_from_inlines_styled(texts, inlines, styles, labels, size, heading, compiler_weight, true);
    }
    // `\\[<dimen>]` reads past the block's last span: the key covers the
    // rest of that line.
    let slice_end = src[end.min(src.len())..].find('\n').map_or(src.len(), |n| end + n + 1).max((end + 2).min(src.len()));
    let Some(slice) = src.get(start..slice_end) else {
        return items_from_inlines_styled(texts, inlines, styles, labels, size, heading, compiler_weight, true);
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
    inlines.len().hash(&mut h);
    for i in inlines {
        let s = inline_span(i);
        (s.start.wrapping_sub(start), s.end.wrapping_sub(start)).hash(&mut h);
        // The kind tag comes from the variant itself, never a hand-written
        // number: see `incremental::tag`.
        crate::incremental::tag(i, &mut h);
        match i {
            // The font is the compiler's, whatever defined it (a macro
            // body, a package), so the node's style keys it, and the space
            // in front's.
            Inline::Text { text, style, glue_before, .. } => {
                text.hash(&mut h);
                hash_style(style, &mut h);
                hash_glue(glue_before, &mut h);
            }
            // The tag is the whole payload.
            Inline::LineBreak { .. } => {}
            Inline::Math { list, display, number, color, size, glue_before, .. } => {
                hash_glue(glue_before, &mut h);
                color.hash(&mut h);
                size.hash(&mut h);
                display.hash(&mut h);
                number.hash(&mut h);
                crate::incremental::hash_math(list, &mut h);
            }
            Inline::Label { key, value, .. } => {
                key.hash(&mut h);
                value.hash(&mut h);
            }
            Inline::PageStyle { style, this_page, .. } => {
                (*style as u8).hash(&mut h);
                this_page.hash(&mut h);
            }
            // `\markboth`/`\markright` set nothing on the page, but the
            // running head they arm is part of this block's answer, so the
            // marks' own content keys the cache (PLAN1 site 17).
            Inline::Mark { left, right, .. } => {
                left.is_some().hash(&mut h);
                for inline in left.iter().flatten().chain(right) {
                    let s = inline_span(inline);
                    (s.start.wrapping_sub(start), s.end.wrapping_sub(start)).hash(&mut h);
                    crate::incremental::tag(inline, &mut h);
                    if let Inline::Text { text, .. } = inline {
                        text.hash(&mut h);
                    }
                }
            }
            Inline::Reference { key, page, equation, style, .. } => {
                hash_style(style, &mut h);
                key.hash(&mut h);
                page.hash(&mut h);
                equation.hash(&mut h);
            }
            // Lowered constructs (pin `d416472a`): their text is in the
            // source slice already hashed; the structure is hashed here.
            Inline::Footnote { number, mark, text, .. } => {
                number.hash(&mut h);
                mark.hash(&mut h);
                text.as_ref().map_or(0, Vec::len).hash(&mut h);
            }
            Inline::Marginpar { text, .. } => {
                text.len().hash(&mut h);
            }
            Inline::Tabular(t) => {
                t.entries.len().hash(&mut h);
                t.inline_lists().iter().map(|l| l.len()).sum::<usize>().hash(&mut h);
            }
            Inline::Verbatim { text, style, glue_before, .. } => {
                hash_style(style, &mut h);
                hash_glue(glue_before, &mut h);
                text.hash(&mut h);
            }
            Inline::ColorBox(b) => {
                format!("{b:?}").hash(&mut h);
            }
            Inline::Underline(u) => {
                format!("{u:?}").hash(&mut h);
            }
            Inline::TextScript(t) => {
                format!("{t:?}").hash(&mut h);
            }
            Inline::Phantom(p) => {
                format!("{p:?}").hash(&mut h);
            }
            Inline::HBox(b) => {
                format!("{b:?}").hash(&mut h);
            }
            Inline::Logo { logo, style, .. } => {
                logo.hash(&mut h);
                hash_style(style, &mut h);
            }
            Inline::Rule { rule, style, .. } => {
                rule.hash(&mut h);
                hash_style(style, &mut h);
            }
            Inline::Kern { amount, style, .. } => {
                amount.hash(&mut h);
                hash_style(style, &mut h);
            }
            // The leader is hashed: `\hfill` and `\hrulefill` differ only in
            // it, and they carry different diagnostics, so an edit between
            // them must not reuse the cached block.
            Inline::HFill { leader, style, .. } => {
                hash_style(style, &mut h);
                match leader {
                    FillLeader::None => 0u8,
                    FillLeader::Rule => 1u8,
                    FillLeader::Dots => 2u8,
                }
                .hash(&mut h);
            }
            Inline::HSpace { pt, style, .. } => {
                hash_style(style, &mut h);
                pt.to_bits().hash(&mut h);
            }
            Inline::TextGlue { em, style, .. } => {
                hash_style(style, &mut h);
                em.to_bits().hash(&mut h);
            }
            Inline::MathRows { rows, aligned, .. } => {
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
                format!("{g:?}").hash(&mut h);
            }
            Inline::Transform(t) => {
                format!("{t:?}").hash(&mut h);
            }
            // Lowered by `lower_inline` like `Reference`, so every field
            // that selects its text is part of the key.
            Inline::CleverReference { keys, page, range, label_only, capitalise, linked, style, .. } => {
                hash_style(style, &mut h);
                keys.hash(&mut h);
                (page, range, label_only, capitalise, linked).hash(&mut h);
            }
            // Nodes only a re-pinned compiler emits. The cache key must
            // still change when any of their fields does, so -- exactly as
            // the `Graphic`/`Transform` arms above do -- the whole node is
            // hashed through its `Debug` form rather than field by field.
            // That is conservative (it can only over-invalidate) and cannot
            // return a stale adaptation once the stacked PRs give these
            // nodes real layout.
            #[cfg(feature = "compiler-node-surface")]
            other @ (Inline::ThePage { .. }
            | Inline::PageNumbering { .. }
            | Inline::TabStop { .. }
            | Inline::TabJump { .. }
            | Inline::Marginpar { .. }
            | Inline::Penalty { .. }
            | Inline::PagePenalty { .. }
            | Inline::Discretionary { .. }) => {
                format!("{other:?}").hash(&mut h);
            }
            Inline::OverlayBegin { spec, kind, .. } => {
                spec.hash(&mut h);
                kind.hash(&mut h);
            }
            Inline::OverlayEnd { .. } => {}
            Inline::Onslide { spec, .. } => spec.hash(&mut h),
        }
    }
    let key = h.finish();
    if let Some(a) = cache.adapted(key) {
        return crate::incremental::relocate_items(&a.items, start as isize - a.base as isize);
    }
    let items = items_from_inlines_styled(texts, inlines, styles, labels, size, heading, compiler_weight, true);
    cache.insert_adapted(key, crate::incremental::AdaptedBlock { items: items.clone(), base: start });
    items
}

/// The blanks just inside an `\\hbox`'s braces (`\\mbox{ lead}`,
/// `\\mbox{trail }`). TeX keeps both: restricted horizontal mode appends a
/// space token as interword glue wherever it stands, so pdfTeX's box is
/// `glue, lead` and `trail, glue`. The compiler's content starts at its
/// first word and ends at its last, so the list built from it has neither;
/// they are read from the source between the braces and the content. The
/// glue is the adjacent word's font's, the trailing one at the space factor
/// that word leaves (`\\mbox{end. }` is a sentence space).
fn hbox_edge_spaces(src: &str, span: Span, content: &[Inline], items: &mut Vec<Item>) {
    let (Some(first), Some(last)) = (content.first().map(inline_span), content.last().map(inline_span)) else {
        return;
    };
    let inside = |s: Span| s.document == span.document && s.start >= span.start && s.end <= span.end;
    if !inside(first) || !inside(last) || span.end == 0 || src.as_bytes().get(span.end - 1) != Some(&b'}') {
        return;
    }
    let blank = |g: Option<&str>| g.is_some_and(|g| !g.is_empty() && g.chars().all(|c| c == ' ' || c == '\t' || c == '\n' || c == '\r'));
    let open = src.get(span.start..first.start).and_then(|s| s.rfind('{')).map(|i| span.start + i + 1);
    if let (Some(open), Some(Item::Word(w))) = (open, items.first()) {
        if blank(src.get(open..first.start)) {
            if let Some(seg) = w.segments.first() {
                let style = seg.style;
                items.insert(0, Item::Space { style, factor: 1000, no_break: false });
            }
        }
    }
    if let Some(Item::Word(w)) = items.last() {
        if blank(src.get(last.end..span.end - 1)) {
            if let Some(seg) = w.segments.last() {
                let style = seg.style;
                let factor = w.segments.iter().flat_map(|s| s.text.chars()).fold(1000, |f, ch| space_factor(ch, f));
                items.push(Item::Space { style, factor, no_break: false });
            }
        }
    }
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
/// `bound` is whether the list being built is laid out as a paragraph (or
/// paragraph-like block) subject to the breaker's item limit. Pure-`\hbox`
/// content (`\colorbox`, `\underline`, `\textsuperscript`: set at natural
/// width by `hbox_runs`, never line-broken) passes `false`, so an enormous
/// box keeps today's slow success instead of a spurious paragraph error;
/// everything laid out through `break_paragraph` passes `true`.
#[allow(clippy::too_many_arguments)]
fn items_from_inlines_styled<'a>(texts: &[&'a str], inlines: &[Inline], styles: &[Styles], labels: &Labels, size: u32, heading: bool, compiler_weight: bool, bound: bool) -> Vec<Item> {
    // `\ref`/`\pageref` become ordinary text attributed to the command's
    // bytes; `\label` becomes a zero-width marker.
    let mut resolved: Vec<std::borrow::Cow<Inline>> = Vec::with_capacity(inlines.len());
    let mut reference_spans: Vec<Span> = Vec::new();
    for inline in inlines {
        lower_inline(inline, labels, &mut reference_spans, &mut resolved);
    }
    apply_size_environments(texts, styles, &mut resolved);
    let mut items: Vec<Item> = Vec::new();
    let mut prev_end: Option<usize> = None;
    let mut prev_span: Option<Span> = None;
    // amsthm `\qedhere` under the pinned compiler leaves no inline (it is
    // reported as unknown), so the gap scan below synthesises the box and
    // this records that the automatic end-of-proof box is suppressed.
    let mut qedhere_used = false;
    // `items.len()` before the current `Inline::HFill`'s additions, so the
    // suppressed automatic pair below can retract them exactly.
    let mut hfill_start = None;
    let mut factor = 1000u32;
    // The `\url{...}` whose runs are being assembled (its span) and the
    // characters of it seen so far, for `url_break_penalty` between runs.
    let mut url_run: Option<(Span, String)> = None;
    // The compiler's size declaration in force at the previous text
    // inline, for the interword space read after it.
    let mut prev_size_cpt = 0u16;
    let mut ambient = TextStyle::default();
    let text_of = |d: DocumentId| -> &str { texts.get(d.0).copied().unwrap_or("") };

    // Where the reader stands in a macro's replacement text (`token_gap`).
    let cursor: std::cell::Cell<Option<BodyCursor>> = std::cell::Cell::new(None);
    // Whether the previous token was a glue control word (`\hfill`,
    // `\quad`, `\hspace`): TeX eats the whitespace right after it, and
    // the gap read next starts at that whitespace.
    let mut after_control_word = false;
    // Whether the bytes TeX read between `prev` and `span` held an
    // interword space (`token_gap`). `text` is the current token's text (a
    // word, or the glue's control word). `\hfill` in a title is the
    // compiler's own `Inline::HFill` (pin `3d3d5ae3`, also inside macro
    // bodies), so the gap is never scanned for fills here.
    // A macro defined in a project package or class file: its body is in
    // that document (`Labels::foreign_definitions`).
    let foreign = |inv: Span| -> Option<(&'a str, usize)> {
        let (document, end) = labels.foreign_definitions.get(&(inv.document.0, inv.start))?;
        Some((texts.get(*document).copied()?, *end))
    };
    // The replacement text of the macro `inv` invokes, wherever it is defined.
    let body_of = |source: &'a str, inv: Span| -> Option<&'a str> {
        let name = control_word_at(source, inv.start, inv.end)?;
        let (def_src, before) = foreign(inv).unwrap_or((source, inv.start));
        macro_body(def_src, name, before)
    };
    // `\newblock` read in the gap (article.cls 584: `\hskip .11em
    // \@plus.33em \@minus.07em`; the compiler lowers the control word to a
    // space token, so only the source bytes still show it). The glue
    // follows the interword space the gap's whitespace gives, as in
    // pdfTeX's list (`Liang.  \OT1/cmr/m/it/10.95 Word`: two glues).
    let pending_newblock = std::cell::Cell::new(false);
    let space_between = |prev_end: Option<usize>, prev_span: Option<Span>, span: Span, text: Option<&str>, after_control_word: bool| -> bool {
        let src = text_of(span.document);
        let mut c = cursor.get();
        let gap = token_gap(src, prev_end, prev_span, span, text, &mut c, &foreign);
        cursor.set(c);
        if gap.as_deref().is_some_and(|g| find_command(g, "newblock").is_some()) {
            pending_newblock.set(true);
        }
        match gap {
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
        // Glue never opens a list: TeX drops a space read in vertical mode
        // (a mark or a `\label` before the first word sets nothing).
        if space && !items.is_empty() {
            items.push(Item::Space { style, factor, no_break: false });
        }
        if pending_newblock.take() {
            items.push(Item::Quad { em: 0.11, plus_em: 0.33, minus_em: 0.07, style });
        }
    };

    // The first inline's span, for the over-long marker below.
    let first_span = resolved.first().map(|i| inline_span(i));
    for inline in resolved.iter() {
        // Fail fast instead of failing slow: the breaker rejects any list
        // past `pl::MAX_ITEMS`, and assembling further only burns
        // superlinear work (`token_gap`'s source rescan per word, shaping
        // and the breaker itself) on a paragraph that cannot be set.
        // `typeset` expands the marker back into an over-limit list, so this
        // surfaces as the same `paragraph_layout_error` a full assembly
        // would have produced — same code, same limit, same position.
        // Lists at or under the limit never take this branch, so every
        // paragraph that succeeds (or fails quickly) today is unaffected.
        if bound && items.len() > pl::MAX_ITEMS {
            let span = first_span.unwrap_or_else(|| inline_span(inline));
            return vec![Item::Overlong { span, count: items.len() }];
        }
        if let Some(sep) = head_sep {
            if sep.opens_the_body(inline_span(inline)) {
                pending_head_sep.set(Some((sep.pt, sep.stretch_pt, sep.shrink_pt)));
                head_sep = None;
            }
        }
        // amsthm `\qedhere` under the pinned compiler: it reports the
        // command as unknown and emits no inline, so without this the box
        // would never be placed and the automatic end-of-proof box would
        // still follow. The command's own bytes sit in the gap between the
        // surrounding inlines; when one is found, the same fill + box the
        // automatic mark uses are emitted at exactly this position (the
        // current end of `items`), and the automatic pair is suppressed
        // when it arrives below. A compiler that emits the pair itself
        // (with the command's bytes as the pair's own span) leaves no such
        // gap, so this never double-fires after a re-pin.
        let here = inline_span(inline);
        if let (Some(pe), Some(ps)) = (prev_end, prev_span) {
            if ps.document == here.document && pe <= here.start {
                let gap_found = text_of(here.document)
                    .get(pe..here.start)
                    .and_then(|gap| find_command(gap, "qedhere"))
                    .map(|at| (pe + at, pe + at + "\\qedhere".len()));
                if let Some((qs, qe)) = gap_found {
                    let qspan = Span::in_document(here.document, qs, qe);
                    let gap = space_between(prev_end, prev_span, qspan, Some("\\qedhere"), after_control_word);
                    let mut gap_style = ambient;
                    gap_style.size_cpt = space_size(texts, prev_end, qspan, prev_size_cpt, 0);
                    push_gap(&mut items, gap, gap_style, factor);
                    let mut fill_style = ambient;
                    fill_style.size_cpt = space_size(texts, prev_end, qspan, prev_size_cpt, 0);
                    items.push(Item::HFill { fill: true, leader: FillLeader::None, style: fill_style });
                    let mut qed_style = ambient;
                    qed_style.size_cpt = fill_style.size_cpt;
                    prev_size_cpt = qed_style.size_cpt;
                    items.push(Item::QedBox { style: qed_style, span: qspan });
                    prev_end = Some(qe);
                    prev_span = Some(qspan);
                    factor = 1000;
                    after_control_word = true;
                    qedhere_used = true;
                }
            }
        }
        match &**inline {
            Inline::Label { key, .. } => items.push(Item::Label { key: key.clone() }),
            // `\pagestyle`/`\thispagestyle` set no horizontal material;
            // the page chrome is the compiler layout's (fancyhdr, #849).
            // `\markboth`/`\markright` likewise: a `\mark` whatsit, read
            // as a running-head event by `mark_commands` (PLAN1 site 17).
            Inline::PageStyle { .. } | Inline::Mark { .. } => {}
            Inline::Reference { .. } | Inline::CleverReference { .. } | Inline::Verbatim { .. } => unreachable!("lowered by lower_inline above"),
            Inline::Footnote { number, span, mark, text, space_before, .. } => {
                // `\@footnotemark` keeps the space factor; the space before
                // the command is an ordinary interword space, when the
                // compiler read one there (PLAN1 site 13: the gap bytes are a
                // macro's own arguments when a macro sets the note). The
                // command's `[<n>]` and `{<text>}` are skipped for the gap
                // that follows. `space_between` still runs for the
                // macro-body cursor.
                let src = text_of(span.document);
                let end = footnote_command_end(src, span.end);
                let word = src.get(span.start..span.end).unwrap_or("\\footnote");
                let _ = space_between(prev_end, prev_span, *span, Some(word), after_control_word);
                let gap = *space_before;
                let mut gap_style = ambient;
                gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                let note = text.as_ref().map(|t| {
                    let mut note = Vec::new();
                    for (k, part) in t.split(|i| matches!(i, Inline::LineBreak { span: at, .. } if at == span)).enumerate() {
                        if k > 0 {
                            note.push(Item::NoteParBreak);
                        }
                        note.extend(items_from_inlines_styled(texts, part, styles, labels, size, false, compiler_weight, bound));
                    }
                    note
                });
                items.push(Item::Footnote { number: number.clone(), mark: *mark, span: *span, text: note });
                after_control_word = end == span.end;
                prev_end = Some(end);
                prev_span = Some(Span::in_document(span.document, span.start, end));
            }
            Inline::Marginpar { text, span, .. } => {
                // `\marginpar` sets no mark: the note is placed in the
                // margin by `typeset::marginpar`. Gap handling matches
                // `Footnote` (`footnote_command_end` reads the same
                // `[<left>]{<right>}` bracket-plus-group shape).
                let src = text_of(span.document);
                let end = footnote_command_end(src, span.end);
                let word = src.get(span.start..span.end).unwrap_or("\\marginpar");
                let gap = space_between(prev_end, prev_span, *span, Some(word), after_control_word);
                let mut gap_style = ambient;
                gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                let mut note = Vec::new();
                for (k, part) in text.split(|i| matches!(i, Inline::LineBreak { span: at, .. } if at == span)).enumerate() {
                    if k > 0 {
                        note.push(Item::NoteParBreak);
                    }
                    note.extend(items_from_inlines_styled(texts, part, styles, labels, size, false, compiler_weight, bound));
                }
                items.push(Item::Marginpar { text: note, span: *span });
                after_control_word = end == span.end;
                prev_end = Some(end);
                prev_span = Some(Span::in_document(span.document, span.start, end));
            }
            Inline::Tabular(t) => {
                // `\leavevmode\hbox{...}`: one box, with the space before it
                // read like a formula's.
                let span = t.span;
                let gap = space_between(prev_end, prev_span, span, None, after_control_word);
                let mut gap_style = node_style(&t.style, size);
                gap_style.size_cpt = space_size(texts, prev_end, span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                after_control_word = false;
                let src = text_of(span.document);
                let lengths = crate::table::TableLengths::read(|name, base| length_at(src, name, size, span.start, base));
                let mut items_of = |inlines: &[Inline], declared: bool| items_from_inlines_styled(texts, inlines, styles, labels, size, false, declared, bound);
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
                let mut gap_style = ambient;
                gap_style.size_cpt = space_size(texts, prev_end, span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                after_control_word = false;
                let content = items_from_inlines_styled(texts, &b.content, styles, labels, size, heading, compiler_weight, false);
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
                let mut gap_style = ambient;
                gap_style.size_cpt = space_size(texts, prev_end, span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                after_control_word = false;
                let content = items_from_inlines_styled(texts, &u.content, styles, labels, size, heading, compiler_weight, false);
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
            // A plain `\hbox` (compiler `Inline::HBox`): `\leavevmode\hbox`,
            // one box like a `\colorbox` without the colour.
            Inline::HBox(b) => {
                let span = b.span;
                let gap = space_between(prev_end, prev_span, span, None, after_control_word);
                let mut gap_style = ambient;
                gap_style.size_cpt = space_size(texts, prev_end, span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                after_control_word = false;
                let mut content = items_from_inlines_styled(texts, &b.content, styles, labels, size, heading, compiler_weight, false);
                hbox_edge_spaces(text_of(span.document), span, &b.content, &mut content);
                items.push(Item::HBox(Box::new(HBoxItem { items: content, span })));
                prev_end = Some(span.end);
                prev_span = Some(span);
                factor = 1000;
            }
            // Text-mode `\phantom{...}` (compiler `Inline::Phantom`): the
            // argument is set and measured but not painted, as beamer's
            // covered text is (`overlay::hide_items`). `\hphantom` keeps
            // the height it should drop and `\vphantom` the width it
            // should drop: an approximation, noted rather than modelled,
            // until the pipeline has a zero-height/zero-width box.
            Inline::Phantom(p) => {
                let span = p.span;
                let gap = space_between(prev_end, prev_span, span, None, after_control_word);
                let mut gap_style = ambient;
                gap_style.size_cpt = space_size(texts, prev_end, span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                after_control_word = false;
                let mut content = items_from_inlines_styled(texts, &p.content, styles, labels, size, heading, compiler_weight, false);
                crate::overlay::hide_items(&mut content);
                items.extend(content);
                prev_end = Some(span.end);
                prev_span = Some(span);
                factor = 1000;
            }
            Inline::TextScript(t) => {
                // A formula (`\ensuremath`): the space before it is read
                // like one, and the space factor after it is 1000.
                let span = t.span;
                let gap = space_between(prev_end, prev_span, span, None, after_control_word);
                let mut gap_style = node_style(&t.style, size);
                gap_style.size_cpt = space_size(texts, prev_end, span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                after_control_word = false;
                let size_cpt = declared_size(t.style.size, size);
                let mut content = items_from_inlines_styled(texts, &t.content, styles, labels, size, heading, compiler_weight, false);
                // `\fontsize\sf@size` replaces the declared size the
                // argument inherited from the command's context.
                if size_cpt != 0 {
                    for item in &mut content {
                        match item {
                            Item::Word(w) => {
                                for seg in &mut w.segments {
                                    if seg.style.size_cpt == size_cpt {
                                        seg.style.size_cpt = 0;
                                    }
                                }
                            }
                            Item::Space { style, .. } if style.size_cpt == size_cpt => style.size_cpt = 0,
                            _ => {}
                        }
                    }
                }
                items.push(Item::TextScript(Box::new(TextScriptItem { superscript: t.superscript, size_cpt, items: content, span })));
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
                // `\includegraphics`: `\leavevmode` then one `\hbox`, like a
                // tabular or a `\colorbox`.
                let span = g.span;
                let gap = space_between(prev_end, prev_span, span, None, after_control_word);
                let mut gap_style = ambient;
                gap_style.size_cpt = space_size(texts, prev_end, span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                after_control_word = false;
                items.push(Item::Graphic { options: g.options.clone(), path: g.path.clone(), span, hidden: false, unpainted: false });
                prev_end = Some(span.end);
                prev_span = Some(span);
                factor = 1000;
            }
            Inline::Transform(t) => {
                let span = t.span;
                let gap = space_between(prev_end, prev_span, span, None, after_control_word);
                let mut gap_style = ambient;
                gap_style.size_cpt = space_size(texts, prev_end, span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                after_control_word = false;
                items.extend(items_from_inlines_styled(texts, &t.content, styles, labels, size, false, compiler_weight, bound));
                prev_end = Some(span.end);
                prev_span = Some(span);
                factor = 1000;
            }
            Inline::HFill { span, style: glue_font, .. } | Inline::HSpace { span, style: glue_font, .. } | Inline::TextGlue { span, style: glue_font, .. } => {
                hfill_start = Some(items.len());
                // Explicit horizontal glue: the interword space read before
                // it stays (TeX keeps both glue nodes). `TextGlue` is the
                // compiler's text-mode `\quad`/`\qquad` (`em` ems of the
                // current font, like the `\quad` after a section number).
                // The control word is passed as the token's text so that
                // a macro-body cursor moves past it (`Problem #1 \hfill
                // \normalfont[#2 points]`): the compiler gives the glue the
                // invocation's span, and the next token's gap must start
                // after the word, not before it.
                // The font the fill's leader is set in: the family, series
                // and shape in force at the command (the compiler's
                // `style`), with the size declaration read at the fill's own
                // span. Like every other gap-style site here, that size goes
                // through `space_size`, not the raw previous-text size: a
                // size group that already closed before the fill (`{\Large
                // A}\dotfill`) leaves the fill at the ambient size; the
                // `next_cpt` is 0 (no declared size: ambient), and the size
                // is only the previous text's when no group closed in
                // between -- never a size established after the fill.
                let mut fill_style = node_style(glue_font, size);
                fill_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, 0);
                // The font the glue's `em` is read in: the compiler's font
                // at the command, size included (PLAN1 site 5: a size
                // declaration from a macro body counts too).
                let quad_style = || node_style(glue_font, size);
                let (item, word) = match &**inline {
                    // `em` is the current font's quad (`\fontdimen6`), which
                    // the compiler's `pt` cannot know: it converts at a fixed
                    // size. An `\hspace{<n>em}` read from the source is set
                    // as `<n>` quads of the font in force, like `\quad`.
                    Inline::HSpace { pt, span, .. } => match hspace_ems(text_of(span.document), *span) {
                        Some(em) => (Item::Quad { em, plus_em: 0.0, minus_em: 0.0, style: quad_style() }, "\\hspace"),
                        None => (Item::HSpace { pt: *pt, stretch_pt: 0.0, shrink_pt: 0.0 }, "\\hspace"),
                    },
                    Inline::TextGlue { em, plus_em, minus_em, .. } => (Item::Quad { em: *em, plus_em: *plus_em, minus_em: *minus_em, style: quad_style() }, if *em >= 2.0 { "\\qquad" } else { "\\quad" }),
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
                        (Item::HFill { fill: true, leader: FillLeader::Rule, style: fill_style }, "\\hrulefill")
                    }
                    Inline::HFill { leader: FillLeader::Dots, .. } => {
                        (Item::HFill { fill: true, leader: FillLeader::Dots, style: fill_style }, "\\dotfill")
                    }
                    Inline::HFill { leader: FillLeader::None, order, .. } => {
                        // `\hfil`/`\hss` are `fil`, `\hfill` is `fill`: the
                        // compiler's order, not the span's bytes (PLAN1 site 14).
                        let fill = *order >= 2;
                        (Item::HFill { fill, leader: FillLeader::None, style: fill_style }, if fill { "\\hfill" } else { "\\hfil" })
                    }
                    _ => unreachable!(),
                };
                let gap = space_between(prev_end, prev_span, *span, Some(word), after_control_word);
                let mut gap_style = ambient;
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
                // The space factor survives: glue (`\hskip`, `\hfill`, the
                // leaders), `\kern` and the `\leavevmode`'s `\unhbox` of a
                // void box leave it alone (tex.web §1041 sets it only for
                // characters, boxes appended in horizontal mode, rules and
                // math). So `Name: \hrulefill{} Date:` keeps the colon's 2000
                // and the blank after `{}` gets `\fontdimen7` too.
                // `\hspace{..}` ends with its argument's `}`: the blank after
                // it is an ordinary space token (`a\hspace{1em} b`), not one
                // skipped after a control word. From a macro the span is the
                // invocation's `\name`, whose blanks are skipped.
                let src = text_of(span.document);
                after_control_word = !(matches!(&**inline, Inline::HSpace { .. }) && span.end > span.start && src.as_bytes().get(span.end - 1) == Some(&b'}'));
            }
            Inline::MathRows { rows, span, .. } => {
                // Each row becomes its own display item (`is_display`
                // recognises the row spans); the environment's span ends
                // the preceding text like `\[`.
                let gap = space_between(prev_end, prev_span, *span, None, after_control_word);
                let mut gap_style = ambient;
                gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, 0);
                push_gap(&mut items, gap, gap_style, factor);
                after_control_word = false;
                for row in rows {
                    items.push(Item::Math {
                        list: math_row_list(row),
                        span: row.span,
                        hidden: false,
                        unpainted: false,
                        size_cpt: 0,
                    });
                }
                prev_end = Some(span.end);
                prev_span = Some(*span);
                factor = 1000;
            }
            Inline::Math { list, span, size: math_size, glue_before, .. } => {
                // `{\small $x$}`: the formula is set with the math fonts of
                // the text size where it starts (`\check@mathfonts`).
                let size_cpt = declared_size(*math_size, size);
                // The glue is the current font's where the space sits.
                // The space in front is the compiler's `glue_before`; the gap is
                // still read for the cursor and `\newblock` (`space_between`).
                let _ = space_between(prev_end, prev_span, *span, None, after_control_word);
                let gap = glue_before.is_some();
                let mut gap_style = glue_before.map_or(ambient, |g| node_style(&g.style, size));
                // As for the text arm below: a theorem body's `\itshape`
                // rides on the compiler's `bold`/`italic`, not on `font`
                // (`parser::TextStyle::font`), so without this the space in
                // `a $x$` is the upright face's, not the italic body's.
                if compiler_weight {
                    if let Some(g) = glue_before {
                        gap_style.bold = g.style.bold;
                        gap_style.italic = g.style.italic;
                    }
                }
                gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, size_cpt);
                push_gap(&mut items, gap, gap_style, factor);
                after_control_word = false;
                // A rich `\eqref` (`lower_inline`): `\textup`'s `\check@icl`
                // puts the italic correction of the word before it under
                // the interword space, as for the text `\eqref` below.
                if reference_spans.contains(span) {
                    let at = items.len() - usize::from(matches!(items.last(), Some(Item::Space { .. })));
                    if at > 0 && matches!(items[at - 1], Item::Word(_)) {
                        items.insert(at, Item::ItalicCorrection);
                    }
                }
                items.push(Item::Math {
                    list: list.clone(),
                    span: *span,
                    hidden: false,
                    unpainted: false,
                    size_cpt,
                });
                prev_end = Some(span.end);
                prev_span = Some(*span);
                prev_size_cpt = size_cpt;
                factor = 1000;
            }
            Inline::Logo { logo, span, style: compiler_style, .. } => {
                // `\LaTeX` is a control word: the blanks after it are eaten.
                let word = format!("\\{}", logo.command());
                let has_space = space_between(prev_end, prev_span, *span, Some(&word), after_control_word);
                let mut style = node_style(compiler_style, size);
                style.size_cpt = declared_size(compiler_style.size, size);
                if heading {
                    style.medium = !compiler_style.bold;
                    style.italic |= compiler_style.italic;
                }
                if has_space || pending_head_sep.get().is_some() {
                    let mut gap_style = style;
                    gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, style.size_cpt);
                    push_gap(&mut items, has_space, gap_style, factor);
                }
                prev_size_cpt = style.size_cpt;
                items.push(Item::Logo { logo: *logo, style, span: *span });
                prev_end = Some(span.end);
                prev_span = Some(*span);
                // `\TeX` ends with `\@` and `\LaTeXe` with math: factor 1000.
                factor = 1000;
                after_control_word = true;
            }
            Inline::Rule { rule, span, style: compiler_style, .. } => {
                let has_space = space_between(prev_end, prev_span, *span, Some("\\rule"), after_control_word);
                let mut style = node_style(compiler_style, size);
                style.size_cpt = declared_size(compiler_style.size, size);
                if has_space || pending_head_sep.get().is_some() {
                    let mut gap_style = style;
                    gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, style.size_cpt);
                    push_gap(&mut items, has_space, gap_style, factor);
                }
                prev_size_cpt = style.size_cpt;
                items.push(Item::Rule { rule: rule.clone(), style, span: *span });
                prev_end = Some(span.end);
                prev_span = Some(*span);
                factor = 1000;
                after_control_word = false;
            }
            Inline::Kern { amount, span, style: compiler_style } => {
                let source = text_of(span.document);
                let word = kern_command_text(source, *span, amount);
                let has_space = space_between(prev_end, prev_span, *span, word.as_deref(), after_control_word);
                let mut style = node_style(compiler_style, size);
                style.size_cpt = declared_size(compiler_style.size, size);
                if has_space || pending_head_sep.get().is_some() {
                    let mut gap_style = style;
                    gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, style.size_cpt);
                    push_gap(&mut items, has_space, gap_style, factor);
                }
                // A kern leaves the space factor alone (§1061 applies only to
                // characters and boxes); a control word eats the blanks after it.
                items.push(Item::Kern { amount: amount.clone(), style });
                prev_end = Some(span.end);
                prev_span = Some(*span);
                after_control_word = word.as_deref().is_some_and(|w| w.len() > 2);
            }
            // amsthm's automatic `\qedsymbol` (GH#443). `\end{proof}` appends
            // exactly two inlines, an `Inline::HFill` with no leader and an
            // `Inline::Text` holding U+220E. The fill carries the
            // `\end{proof}` bytes; the text carries either the same span or,
            // since #862, an empty span at that `\end`'s start (so the mark
            // no longer covers `\end{proof}` in the source map). That pair is
            // the compiler's marker (it emits U+220E nowhere else); a U+220E
            // typed in the source is a lone text inline with its own span and
            // still sets whatever the font has. Latin Modern has no U+220E
            // glyph, so the text arm below would warn `missing_glyph` and
            // draw nothing; amsthm never wanted a character here anyway.
            Inline::Text { text, span, .. }
                if text == "\u{220E}"
                    && prev_span.is_some_and(|fill| {
                        fill.document == span.document
                            && fill.start == span.start
                            && (fill.end == span.end || span.end == span.start)
                    })
                    && matches!(items.last(), Some(Item::HFill { leader: FillLeader::None, .. })) =>
            {
                if qedhere_used {
                    // amsthm's `\popQED`: the gap scan above already placed
                    // the box at `\qedhere`, so this automatic end-of-proof
                    // pair is suppressed — retracted exactly, gap glue and
                    // fill with it.
                    if let Some(start) = hfill_start {
                        items.truncate(start);
                    }
                    prev_end = Some(span.end);
                    prev_span = Some(*span);
                    factor = 1000;
                    after_control_word = false;
                    hfill_start = None;
                    continue;
                }
                let Inline::Text { style: compiler_style, .. } = &**inline else { unreachable!() };
                let mut style = node_style(compiler_style, size);
                style.size_cpt = declared_size(compiler_style.size, size);
                prev_size_cpt = style.size_cpt;
                // amsthm.sty 273-279, `\qed` in text:
                //
                // ```text
                // \leavevmode\unskip\penalty9999 \hbox{}\nobreak\hfill
                // \quad\hbox{\qedsymbol}
                // ```
                //
                // `\unskip` takes the interword glue the source's blank
                // before `\end{proof}` gave, then a `\penalty9999` (the box
                // may go to a line of its own, at a price), an empty box,
                // `\nobreak`, the fill, and a `\quad` in front of the symbol.
                // The fill's arm above pushed that glue and the fill; both
                // are replaced. With the glue kept and the quad missing the
                // last line was 1em + a space too short in the breaker's
                // eyes: `fixtures/divergence-probes/min5-proof-close-shrink`
                // kept `hence by zero.` on one shrunk line where pdflatex
                // breaks after `by` (`zero.` 445.7 bp off).
                let fill_style = match hfill_start.and_then(|s| items.get(s..)).and_then(|tail| tail.iter().find_map(|i| match i {
                    Item::HFill { style, .. } => Some(*style),
                    _ => None,
                })) {
                    Some(fill_style) => {
                        items.truncate(hfill_start.unwrap_or(items.len()));
                        if matches!(items.last(), Some(Item::Space { .. })) {
                            items.pop();
                        }
                        Some(fill_style)
                    }
                    None => None,
                };
                if let Some(fill_style) = fill_style {
                    items.push(Item::Penalty { value: 9999, flagged: false });
                    items.push(Item::LeaveVmode);
                    items.push(Item::Penalty { value: 10000, flagged: false });
                    items.push(Item::HFill { fill: true, leader: FillLeader::None, style: fill_style });
                    items.push(Item::Quad { em: 1.0, plus_em: 0.0, minus_em: 0.0, style });
                }
                items.push(Item::QedBox { style, span: *span });
                prev_end = Some(span.end);
                prev_span = Some(*span);
                factor = 1000;
                after_control_word = false;
            }
            Inline::Text { span, glue_before: Some(InterwordGlue { kind: GlueKind::ControlSpace, .. }), .. } => {
                // `\ ` (the compiler's control-space node, `GlueKind::
                // ControlSpace`, from the source or a macro body): interword
                // glue at space factor 1000 (§1041-1044), after which TeX
                // skips blanks. A space read before it is not glue of its own
                // (the compiler drops it), unless a theorem head's separator
                // is pending.
                let _ = space_between(prev_end, prev_span, *span, Some("\\ "), after_control_word);
                let Inline::Text { style: compiler_style, .. } = &**inline else { unreachable!() };
                let style = node_style(compiler_style, size);
                if pending_head_sep.get().is_some() {
                    push_gap(&mut items, false, style, factor);
                }
                items.push(Item::Space { style, factor: 1000, no_break: false });
                prev_size_cpt = style.size_cpt;
                ambient = style;
                prev_end = Some(span.end);
                prev_span = Some(*span);
                factor = 1000;
                after_control_word = true;
            }
            Inline::Text { text, span, .. } => {
                let source = text_of(span.document);
                // Whether TeX appended interword glue in front of the run is the
                // compiler's `glue_before` (PLAN1 slice 2): a space token read in
                // horizontal mode, not after a control word or a line break.
                // The gap is still read for the macro-body cursor, `\newblock`,
                // the CJK boundaries below, and one boundary the token stream
                // does not show: the end of an `\input` file whose last line
                // has no newline still ends with `\endlinechar`, a space.
                let gap = space_between(prev_end, prev_span, *span, Some(text), after_control_word);
                let Inline::Text { glue_before: text_glue, .. } = &**inline else { unreachable!() };
                let crossed_input = prev_span.is_some_and(|p| p.document != span.document);
                let mut has_space = text_glue.is_some() || crossed_input && gap;
                after_control_word = false;
                // A `CJK` environment boundary in the gap, and `CJK*`'s
                // `\ignorespaces` after the previous CJK character
                // (`cjk_gap_spaces`): the gap may then hold two interword
                // spaces, or none where the source shows one.
                let mut second_space = false;
                if let Some(pe) = prev_end.filter(|_| prev_span.is_some_and(|ps| ps.document == span.document)) {
                    let after_nospace_cjk = matches!(items.last(), Some(Item::Word(w)) if w.segments.last().is_some_and(|s| s.style.cjk.is_some_and(|r| r.nospace) && s.text.chars().last().is_some_and(cjk_read_char)));
                    match cjk_gap_spaces(source.get(pe..span.start).unwrap_or(""), after_nospace_cjk) {
                        Some(0) => has_space = false,
                        Some(2) => second_space = true,
                        Some(_) => {}
                        None if after_nospace_cjk => has_space = false,
                        None => {}
                    }
                }
                // The run's font is the compiler's (PLAN1 slice 2): every font
                // command it read, macro-expanded or not, is in
                // `TextStyle::font`, together with the size, colour, CJK run
                // and verbatim marks.
                let Inline::Text { style: compiler_style, glue_before, .. } = &**inline else { unreachable!() };
                let mut style = node_style(compiler_style, size);
                if heading {
                    // `\@startsection` sets `\bfseries`; the compiler's
                    // heading styles start bold and `\normalfont`/
                    // `\mdseries` in the title clears it.
                    style.medium = !compiler_style.bold;
                    style.italic |= compiler_style.italic;
                }
                // A citation's runs, its label markup (`\emph{et~al.}`, an
                // undefined key's `\reset@font\bfseries ?`) included, are
                // set by the compiler in the font around the `\cite`
                // (`parser::citation_style`).
                if compiler_weight {
                    style.bold = compiler_style.bold;
                    style.italic = compiler_style.italic;
                }
                if has_space || pending_head_sep.get().is_some() {
                    // TeX sizes an interword space with the font current
                    // where the space token is read ("Plain, \textbf{bold}"
                    // gets a regular space, "\textbf{bold words}" a bold one,
                    // "\textbf{\emph{x}} y" a regular one): the compiler's
                    // `glue_before`, else the font the last run was set in.
                    let mut gap_style = match glue_before {
                        Some(glue) => {
                            let mut gap_style = node_style(&glue.style, size);
                            if heading {
                                // `\subsection*{Bonus \hfill \normalfont[1
                                // bonus point]}`: a space read after the
                                // declaration is `ecrm1200`'s 3.90bp, not the
                                // head's `ecbx1200` 4.48bp.
                                gap_style.medium = !glue.style.bold;
                                gap_style.italic |= glue.style.italic;
                            }
                            gap_style
                        }
                        None => ambient,
                    };
                    if compiler_weight {
                        gap_style.bold = style.bold;
                        gap_style.italic = style.italic;
                    }
                    gap_style.size_cpt = space_size(texts, prev_end, *span, prev_size_cpt, style.size_cpt);
                    push_gap(&mut items, has_space, gap_style, factor);
                    if second_space && has_space {
                        // The second space token is read with the same
                        // space factor: glue never changes `\spacefactor`.
                        items.push(Item::Space { style: gap_style, factor, no_break: false });
                    }
                }
                // LaTeX's `\check@icl`: a text font command whose font is
                // upright (`\fontdimen1 = 0`) runs `\maybe@ic` before its
                // argument, and `\sw@slant` puts the italic correction of
                // the character before it *under* the interword space
                // (`of \textbf{x}`: `f`, kern 0.7922 pt, space). The
                // compiler decides it (`TextStyle::italic_correction`;
                // amsmath's `\eqref`, `\textup{\tagform@{..}}`, always does,
                // and so does a citation label's `\emph` turning upright).
                let check_icl = compiler_style.italic_correction.before;
                if check_icl && !style.literal {
                    let at = items.len() - usize::from(matches!(items.last(), Some(Item::Space { .. })));
                    if at > 0 && matches!(items[at - 1], Item::Word(_)) {
                        items.insert(at, Item::ItalicCorrection);
                    }
                }
                prev_size_cpt = style.size_cpt;
                ambient = node_style(compiler_style, size);
                // A run of a `\url{...}` (the compiler splits the argument
                // after every url.sty break character, `parser::url_pieces`,
                // each run with the whole command's span): the break between
                // it and the previous run of the same URL is TeX's math-list
                // penalty (`url_break_penalty`), a bare `\penalty` since the
                // muskips around it are 0mu. Without it the runs merged into
                // one unbreakable word.
                let is_url_run = url_run_at(source, *span);
                if is_url_run {
                    match url_run.as_mut().filter(|(s, _)| s == span) {
                        Some((_, before)) => {
                            if let Some(value) = text.chars().next().and_then(|next| url_break_penalty(before, next)) {
                                items.push(Item::Penalty { value, flagged: false });
                            }
                            before.push_str(text);
                        }
                        None => url_run = Some((*span, text.clone())),
                    }
                } else {
                    url_run = None;
                }
                // Per-character sources. Macro replacement text shares the
                // invocation span; keep that attribution for every char.
                let citation = generated_citation(source, *span);
                // A citation's text arrives as several runs, all with the
                // command's span, and the compiler splits them exactly where
                // the package's macros put a non-character token between two
                // characters -- natbib's `\NAT@nmfmt{\NAT@nm}` is the group
                // `{\NAT@up Hobby}`, the kernel's labels are `\hbox`es. TeX's
                // lig/kern program stops at such a token (§1034-1040), so
                // `Hobby}` then `,` is set without cmr's `y`-`,` kern of
                // -0.0833 em (-0.91 pt at 10.95 pt, which moved every later
                // word of natbib-review's citation lines 0.91 bp left). Such
                // a run starts a segment of its own.
                // An empty group or `\relax` before the run (the compiler's
                // `boundary_before`, `Shelf{}ful`, also from a macro body)
                // stops the program the same way.
                let Inline::Text { boundary_before, .. } = &**inline else { unreachable!() };
                let kern_break = std::cell::Cell::new((citation && prev_span == Some(*span) || *boundary_before) && matches!(items.last(), Some(Item::Word(_))));
                // The compiler's input-ligature pass (`lexer::
                // apply_text_ligatures`) makes the text of a word shorter
                // than its bytes (`--` is one U+2013), so the sources are
                // aligned ligature by ligature, not by equal length: with the
                // length test, `pp.~1119--1184,` was "not the source's own
                // bytes" and its `~` was set as a tilde glyph (6.09 pt in
                // `ecrm1000`) instead of a tie — `1981.` 2.76 bp right on
                // `fixtures/real-world/article-twocolumn` page 2.
                let exact_sources = if reference_spans.contains(span) || citation {
                    None
                } else {
                    ligature_char_sources(source, *span, text)
                };
                let exact = exact_sources.is_some();
                let chars: Vec<(char, CharSrc)> = match exact_sources {
                    Some(sources) => text.chars().zip(sources).collect(),
                    None => text
                        .chars()
                        .map(|ch| {
                            (
                                ch,
                                CharSrc {
                                    document: span.document,
                                    start: span.start,
                                    end: span.end,
                                },
                            )
                        })
                        .collect(),
                };
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
                    if kern_break.replace(false) {
                        push_segment_apart(items, text, srcs, style);
                    } else {
                        push_segment(items, text, srcs, style);
                    }
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
                    // onto the same command). The compiler hands every tie it
                    // read over as U+00A0 (`parser::word_node`), from the
                    // source or a macro body alike (`\newcommand{\fig}
                    // {Figure~7}`), while `\textasciitilde` stays a `~`.
                    if ch == NO_BREAK_SPACE {
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
                // A URL is a formula, and leaving math mode sets the space
                // factor to 1000 (TeX §1196): the space after `\url{x.}` is
                // an ordinary one, not the `\sfcode` 3000 of the `.`.
                if is_url_run {
                    factor = 1000;
                }
                // A text font command's group closing right after this run:
                // LaTeX's `\check@icr` appends `\/` (`\maybe@ic`) unless the
                // next token is in `\nocorrlist` (`,` and `.`) or the font
                // after the group is slanted (`\fontdimen1 > 0`). The
                // compiler decides it (`TextStyle::italic_correction`).
                if compiler_style.italic_correction.after && matches!(items.last(), Some(Item::Word(_))) {
                    items.push(Item::ItalicCorrection);
                }
                // A text symbol the compiler set from a control word (`\AA`,
                // `\ss`, `\today`): TeX skips the blanks after the word. User
                // macro replacements keep their own cursor (`token_gap`),
                // wherever the macro is defined (`body_of`).
                after_control_word = control_word_at(source, span.start, span.end).is_some() && body_of(source, *span).is_none();
                prev_end = Some(span.end);
                prev_span = Some(*span);
                // A lowered `\verb`/`\lstinline` (`lower_inline`): the
                // compiler's span is the control word alone, so the gap
                // after it would be read from the delimited argument's own
                // bytes -- `\verb|a b|.` got a rigid typewriter blank before
                // the `.`, where TeX has none (the `|` ends the group and
                // the `.` follows it directly), and `\verb|a b| y` a blank
                // of the typewriter font where TeX reads the space token in
                // the outer font. What was read is the whole command.
                // (`\lstinline` is spelled over as `\verb` for the engine,
                // and its span is those five bytes: the word is re-read
                // from the source at the span's start.)
                if reference_spans.contains(span) {
                    let word_end = source.get(span.start + 1..).map_or(span.start, |r| span.start + 1 + r.bytes().take_while(u8::is_ascii_alphabetic).count());
                    if let Some(name) = control_word_at(source, span.start, word_end).filter(|n| verb_command(n)) {
                        if let Some(v) = verb_span(source, span.start, span.start + 1 + name.len()) {
                            prev_end = Some(v.whole.1);
                            prev_span = Some(Span::in_document(span.document, span.start, v.whole.1));
                            after_control_word = false;
                        }
                    }
                }
            }
            // A `\penalty` in the horizontal list (`\linebreak`/
            // `\nolinebreak`'s `\@no@lnbk`, amsmath's `\nobreakdash`,
            // cite.sty's `\penalty\@m` before its thin glue). `unskip` is
            // `\@no@lnbk`'s: the interword space in front of the command is
            // removed here and re-read from the source for the next word,
            // so it lands after the penalty as TeX's `\unskip ... \ ` puts
            // it -- and glue after a penalty is not a break point (tex.web
            // §866: only glue after a non-discardable node is), which is
            // what makes `word \nolinebreak word` unbreakable there. The
            // source gap is read from the previous text's end, so the
            // marker itself advances nothing.
            #[cfg(feature = "compiler-node-surface")]
            Inline::Penalty { value, unskip, span } => {
                if *unskip && matches!(items.last(), Some(Item::Space { .. })) {
                    items.pop();
                }
                // The primitive `\penalty<number>` (its span covers the
                // number) and `\nobreak`/`\allowbreak` written in the
                // source: the blank after the number or control word is
                // TeX's optional space (§443) or the space after a control
                // word, never glue, so the gap is read from the command's
                // end (`60:\penalty0 3461` is one word). The blank before it
                // is an interword space like any other.
                let source = text_of(span.document);
                let command = !*unskip
                    && source.get(span.start..span.end).is_some_and(|s| {
                        let name = s.strip_prefix('\\').map(|r| &r[..r.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(r.len())]);
                        matches!(name, Some("penalty" | "nobreak" | "allowbreak"))
                    });
                if command {
                    let has_space = space_between(prev_end, prev_span, *span, None, after_control_word);
                    if has_space {
                        let gap_style = ambient;
                        push_gap(&mut items, true, gap_style, factor);
                    }
                }
                items.push(Item::Penalty { value: *value, flagged: false });
                if command {
                    prev_end = Some(span.end);
                    prev_span = Some(*span);
                    after_control_word = true;
                }
            }
            // Inlines only a re-pinned compiler emits. Every one of them is
            // a zero-width marker in the horizontal list -- a discretionary,
            // a tab stop or jump, a page-number marker -- so producing no
            // item is what the old pin already did for the same source, and
            // the line breaker sees exactly the same sequence. `Marginpar`
            // is the one that carries text; the pipeline has no margin
            // column yet (GH-505), so its note is not set here either way.
            // PR #569 (discretionaries), GH-TABBING and GH-505 (marginpar)
            // replace this arm.
            #[cfg(feature = "compiler-node-surface")]
            Inline::ThePage { .. }
            | Inline::PageNumbering { .. }
            | Inline::TabStop { .. }
            | Inline::TabJump { .. }
            | Inline::Marginpar { .. }
            | Inline::PagePenalty { .. }
            | Inline::Discretionary { .. } => {}
            // beamer overlay markers: no material, no gap of their own (the
            // interword space around `\only<2>{...}` is read from the
            // source on either side, as TeX's two glues are).
            Inline::OverlayBegin { spec, kind, .. } => items.push(Item::Overlay(OverlayMark::Begin { spec: spec.clone(), kind: *kind })),
            Inline::OverlayEnd { .. } => items.push(Item::Overlay(OverlayMark::End)),
            Inline::Onslide { spec, .. } => items.push(Item::Overlay(OverlayMark::Onslide { spec: spec.clone() })),
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

/// The size environment in force where a paragraph's `\par` is read (the
/// blank line or `\par` after its last inline), for its `\baselineskip`:
/// the compiler's `block_par_leading` does not know size environments.
fn size_env_par_leading(texts: &[&str], styles: &[Styles], inlines: &[Inline]) -> ParLeading {
    let span = inline_span(inlines.last()?);
    let st = styles.get(span.document.0).filter(|s| !s.size_envs.is_empty())?;
    let src = texts.get(span.document.0)?;
    let rest = src.get(span.end..)?;
    let mut par = rest.len();
    let mut blank_from = None;
    for (off, c) in rest.char_indices() {
        match c {
            '\n' if blank_from.is_some() => {
                par = off;
                break;
            }
            '\n' => blank_from = Some(off),
            ' ' | '\t' | '\r' => {}
            _ => blank_from = None,
        }
    }
    if let Some(p) = find_command(&rest[..par], "par") {
        par = par.min(p);
    }
    st.size_env_at(src, span.end + par)
}

/// Gives the text, rules, kerns, tables and explicit glue inside a size environment the
/// environment's size where the compiler left them at the surrounding size
/// ([`Styles::size_env_at`]).
fn apply_size_environments(texts: &[&str], styles: &[Styles], resolved: &mut [std::borrow::Cow<Inline>]) {
    if styles.iter().all(|s| s.size_envs.is_empty()) {
        return;
    }
    for inline in resolved.iter_mut() {
        let span = inline_span(inline);
        let (Some(st), Some(src)) = (styles.get(span.document.0), texts.get(span.document.0)) else { continue };
        let no_size = match &**inline {
            Inline::Text { style, .. } | Inline::Logo { style, .. } | Inline::Rule { style, .. } | Inline::Kern { style, .. } => style.size.is_none(),
            Inline::Tabular(t) => t.style.size.is_none(),
            Inline::TextGlue { style, .. } | Inline::HSpace { style, .. } | Inline::HFill { style, .. } => style.size.is_none(),
            _ => false,
        };
        if !no_size {
            continue;
        }
        let Some(level) = st.size_env_at(src, span.start) else { continue };
        match inline.to_mut() {
            Inline::Text { style, .. } | Inline::Logo { style, .. } | Inline::Rule { style, .. } | Inline::Kern { style, .. } => style.size = Some(level),
            Inline::Tabular(t) => t.style.size = Some(level),
            Inline::TextGlue { style, .. } | Inline::HSpace { style, .. } | Inline::HFill { style, .. } => style.size = Some(level),
            _ => {}
        }
    }
}

/// The size declaration (`declared_size`) of an inline that carries the
/// compiler's text style; `None` for one that does not.
fn inline_declared_size(inline: &Inline, base: u32) -> Option<u16> {
    match inline {
        Inline::Text { style, .. } | Inline::Logo { style, .. } | Inline::Rule { style, .. } | Inline::Kern { style, .. } => {
            Some(declared_size(style.size, base))
        }
        _ => None,
    }
}

/// Appends a segment to the current word or starts a new word.
///
/// One segment is shaped as one string, with the face's ligature/kern
/// program running across it, so a run in the same style joins the segment
/// before it. Where TeX's lig/kern lookahead stops at a non-character token
/// between two runs (§1034-1040) -- an empty group or `\relax`
/// (`Inline::Text::boundary_before`): `-{}-` is two hyphens in `T1/cmtt`
/// where `--` is the en dash of `ectt1095`'s `LIG O 55 O 25`, `f{}i` two
/// letters -- the caller uses [`push_segment_apart`] instead. (A closing
/// brace alone, `Schr\"{o}dinger`, stops the program too but leaves TeX's
/// hyphenation pass one word, and a segment is the unit `typeset`
/// hyphenates, so the compiler does not mark it.)
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

/// Appends a segment to the current word without joining the segment
/// before it: the two are shaped apart, so no ligature or kern of the face
/// runs across the boundary (a non-character token between them in TeX).
fn push_segment_apart(items: &mut Vec<Item>, text: String, chars: Vec<CharSrc>, style: TextStyle) {
    let segment = Segment { text, chars, style };
    match items.last_mut() {
        Some(Item::Word(word)) => word.segments.push(segment),
        _ => items.push(Item::Word(Word { segments: vec![segment] })),
    }
}

/// The source bytes of each character of `text`, when `text` is exactly the
/// bytes of `span` read through the compiler's input-ligature pass
/// (`lexer::apply_text_ligatures`: `---` `--` ``` `` ``` `''` ``!` `` ``?` ``
/// `` ` `` `'`), and `None` when it is anything else — a macro's
/// replacement text, a control word's symbol, a theorem head. A ligature's
/// character gets the bytes of its whole input sequence.
fn ligature_char_sources(source: &str, span: Span, text: &str) -> Option<Vec<CharSrc>> {
    let bytes = source.get(span.start..span.end)?;
    let mut sources = Vec::with_capacity(text.len());
    let mut at = 0usize;
    for ch in text.chars() {
        let rest = &bytes[at..];
        let len = if rest.starts_with(ch) {
            ch.len_utf8()
        } else {
            let input = match ch {
                '\u{2014}' => "---",
                '\u{2013}' => "--",
                '\u{201C}' => "``",
                '\u{201D}' => "''",
                '\u{00A1}' => "!`",
                '\u{00BF}' => "?`",
                // A tie (`parser::word_node`).
                '\u{00A0}' => "~",
                '\u{2018}' => "`",
                '\u{2019}' => "'",
                _ => return None,
            };
            if !rest.starts_with(input) {
                return None;
            }
            input.len()
        };
        sources.push(CharSrc { document: span.document, start: span.start + at, end: span.start + at + len });
        at += len;
    }
    (at == bytes.len()).then_some(sources)
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

    /// `\hangfrom{label}` detection (see `hangfrom_label`): the label run
    /// is the paragraph's leading inlines through the group's trailing
    /// gap, and the command span is the backslash.
    #[test]
    fn hangfrom_detection_takes_the_label_run() {
        let src = "\\documentclass[11pt]{article}\n\\begin{document}\n\\hangfrom{Label. }body text here\n\\end{document}\n";
        let docs = [flashtex_compiler::parser::SourceDocument { path: "main.tex", text: src }];
        let parsed = flashtex_compiler::parser::parse_project(&docs, "main.tex");
        let paragraph = parsed.blocks.iter().find_map(|b| match b {
            CBlock::Paragraph(inlines) => Some(inlines.as_slice()),
            _ => None,
        });
        let inlines = paragraph.expect("a paragraph");
        let (label, cmd) = hangfrom_label(&[src], inlines, 0).expect("hangfrom detected");
        assert_eq!(label.len(), 2, "label words plus the trailing gap");
        let prose: String = label
            .iter()
            .filter_map(|i| match i {
                Inline::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(prose, "Label. ");
        assert_eq!(&src[cmd.start..cmd.start + 9], "\\hangfrom");
    }

    /// No group to measure, no hang: a macro-supplied label and a
    /// paragraph that merely follows an earlier `\hangfrom` both keep
    /// the old shape (and their diagnostic).
    #[test]
    fn hangfrom_detection_bails_without_a_leading_group() {
        let paragraphs = |src: &str| {
            let docs = [flashtex_compiler::parser::SourceDocument { path: "main.tex", text: src }];
            let parsed = flashtex_compiler::parser::parse_project(&docs, "main.tex");
            let mut out = Vec::new();
            for b in &parsed.blocks {
                if let CBlock::Paragraph(inlines) = b {
                    out.push(inlines.clone());
                }
            }
            out
        };
        let src = "\\newcommand{\\lab}{Label. }\\begin{document}\n\\hangfrom\\lab body\n\\end{document}\n";
        let paras = paragraphs(src);
        let inlines = paras.first().expect("a paragraph");
        assert!(hangfrom_label(&[src], inlines, 0).is_none(), "macro label has no group");
        let src = "\\begin{document}\n\\hangfrom{A}foo\n\nbar baz\n\\end{document}\n";
        let paras = paragraphs(src);
        assert_eq!(paras.len(), 2, "two paragraphs");
        assert!(
            hangfrom_label(&[src], &paras[1], 0).is_none(),
            "a paragraph after the group is ordinary text"
        );
    }

    /// CJK.sty's environment boundary in a gap (`cjk_gap_spaces`): pdflatex
    /// reads a space token on each side of `\end{CJK}`, `CJK*`'s
    /// `\ignorespaces` eats the one before it, and `\begin{CJK}`'s three
    /// arguments are skipped.
    #[test]
    fn cjk_environment_boundary_spaces() {
        assert_eq!(cjk_gap_spaces("\n\\end{CJK}\n", false), Some(2), "two glues before the `(` of the fixture");
        assert_eq!(cjk_gap_spaces("\n\\end{CJK*}\n", true), Some(1), "CJK*: \\ignorespaces after the character eats the first");
        assert_eq!(cjk_gap_spaces("\\end{CJK} ", false), Some(1));
        assert_eq!(cjk_gap_spaces("\\end{CJK}", false), Some(0));
        assert_eq!(cjk_gap_spaces("\n\\begin{CJK}{UTF8}{min}", false), Some(1), "a line end before the environment: one");
        assert_eq!(cjk_gap_spaces(" \\begin{CJK*}[T1]{UTF8}{min} ", false), Some(2));
        assert_eq!(cjk_gap_spaces(" \\begin{CJK}{UTF8}{min}", false), Some(1));
        assert_eq!(cjk_gap_spaces(" ", false), None, "no environment command: the ordinary rule");
        assert_eq!(cjk_gap_spaces("\\end{CJK}\\begin{CJK}{UTF8}{min}", false), Some(0));
    }

    /// The characters CJK.sty reads in a `CJK` environment: everything
    /// inputenc does not declare.
    #[test]
    fn cjk_read_characters() {
        assert!(cjk_read_char('東'));
        assert!(cjk_read_char('。'));
        assert!(cjk_read_char('α'));
        assert!(!cjk_read_char('ü'), "utf8.def declares it");
        assert!(!cjk_read_char('a'));
        assert!(!cjk_read_char('—'), "U+2014 is \\textemdash");
    }

    /// Issue #520: the environment name at the display's first byte picks
    /// the alignment, so `eqnarray` reaches the kernel `\halign` arm rather
    /// than falling through to `align`, starred or not.
    #[test]
    fn rows_env_reads_eqnarray_from_source() {
        assert_eq!(RowsEnv::at("\\begin{eqnarray} a &=& b \\end{eqnarray}"), RowsEnv::EqnArray);
        assert_eq!(RowsEnv::at("\\begin{eqnarray*} a &=& b \\end{eqnarray*}"), RowsEnv::EqnArray);
        assert_eq!(RowsEnv::at("\\begin{align} a &= b \\end{align}"), RowsEnv::Align);
        assert_eq!(RowsEnv::at("\\begin{gather} a \\\\ b \\end{gather}"), RowsEnv::Gather);
        assert_eq!(RowsEnv::at("\\begin{multline} a \\\\ b \\end{multline}"), RowsEnv::Multline);
    }

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

    /// [`block_leadings`] pairs positionally only when the compiler's list is
    /// exactly one entry per block, and otherwise discards it — with no
    /// dependence on `debug_assertions`, so a debug and a release build lay
    /// the same document out the same way (#667).
    ///
    /// The predecessor of this function was a `debug_assert_eq!` followed by a
    /// `zip`: a debug build panicked and a release build paired block *i* with
    /// a stray entry, which is how `\colorbox{white}{\small x}` came to set a
    /// whole body paragraph's `\baselineskip` in release only.
    #[cfg(feature = "par-leading")]
    #[test]
    fn block_leadings_are_profile_independent() {
        use flashtex_compiler::parser::FontSizeLevel;
        let small = Some(FontSizeLevel::Small);
        let large = Some(FontSizeLevel::Large1);

        // One entry per block: used as it stands, in order.
        assert_eq!(block_leadings(&[small, None, large], 3), vec![small, None, large]);
        assert_eq!(block_leadings(&[], 0), Vec::<ParLeading>::new());

        // Any other length is unpairable: every block takes the body leading.
        // A stray entry is pushed before the block it belongs to, so a longer
        // list must not simply be truncated to `blocks` (that is the release
        // mis-pairing this replaced) and a shorter one must not be padded.
        assert_eq!(block_leadings(&[small, None], 1), vec![None]);
        assert_eq!(block_leadings(&[small, small, None], 2), vec![None, None]);
        assert_eq!(block_leadings(&[small], 3), vec![None, None, None]);
        assert_eq!(block_leadings(&[small], 0), Vec::<ParLeading>::new());

        // The property the old `debug_assert!` broke: the result depends on
        // the inputs alone, never on how the crate was compiled.
        for compiler_len in 0..6usize {
            for blocks in 0..6usize {
                let list = vec![small; compiler_len];
                let got = block_leadings(&list, blocks);
                assert_eq!(got.len(), blocks, "{compiler_len} entries, {blocks} blocks");
                assert!(
                    got.iter().all(|l| *l == small) || got.iter().all(|l| l.is_none()),
                    "{compiler_len} entries, {blocks} blocks: {got:?}"
                );
                assert_eq!(got, block_leadings(&list, blocks), "not deterministic");
            }
        }
    }

    /// [`SourceIndex`] answers exactly what the prefix scans it replaces
    /// (`list_stack_at`, `in_theorem_environment`) answer, at every byte
    /// offset, including offsets inside control words and names, comments,
    /// escaped `\%`, unclosed braces and mismatched `\end`s.
    #[test]
    fn source_index_matches_prefix_scans() {
        let envs: std::collections::HashSet<String> = ["proof", "theorem", "lemma"].iter().map(|s| s.to_string()).collect();
        let sources = [
            "",
            "\\begin{itemize}\\item a\\end{itemize}",
            "\\newtheorem{theorem}{Theorem}\n\\begin{document}\n\\begin{theorem}[Name] text \\begin{itemize}[nosep, leftmargin=*]\n\\item x\n\\begin{enumerate}[(a)]\\item y\\end{enumerate}\\end{itemize}\n\\end{theorem}\n\\begin{proof}p\\end{proof}\\end{document}",
            "% \\begin{theorem}\n\\begin {lemma} a \\% \\begin{proof} b\\end{lemma}\\beginning{x}\\endgroup \\begin{description}\\item[k] v\\end{itemize}\\end{description}",
            "\\begin{thebibliography}{99}\\bibitem{a} A\\end{thebibliography}\\begin{theorem} \\end{lemma} \\end{theorem}",
            "\\begin{theorem} unclosed \\begin{itemize \\item \\end{itemize",
            "\\begin[x]{theorem} é \\\\begin{proof} \\begin{enumerate}\\item ü\\end{enumerate} \\begin",
        ];
        for source in sources {
            let index = SourceIndex::new(source, &envs);
            for at in 0..=source.len() {
                let boundary = source.is_char_boundary(at);
                assert_eq!(index.in_theorem(boundary, at), in_theorem_environment(source, at, &envs), "in_theorem at {at} of {source:?}");
            }
            assert_eq!(index.natbib_author_year, natbib_author_year(source));
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

    /// PLAN1 site 36: `\sloppy` reaches the stylesheet through the
    /// compiler's `ParameterAssignment`s, not a byte scan. The three cases
    /// the retired `document_sloppy` scanner was tested on (a document-wide
    /// `\sloppy`, one inside a group, and one inside a comment) keep the
    /// same answers, and a `\sloppy` a macro produced now counts too.
    #[test]
    fn document_sloppy_comes_from_the_compilers_parameters() {
        let doc = |body: &str| adapted(&format!("\\documentclass{{article}}\n\\begin{{document}}{body}\\end{{document}}"));
        let loose = |s: &Stylesheet| (s.tolerance, s.emergency_stretch_pt);
        assert_eq!(loose(&doc("\\sloppy text").style), (9999.0, 30.0));
        assert_eq!(loose(&doc("{\\sloppy text} more").style), (200.0, 0.0));
        assert_eq!(loose(&doc("% \\sloppy\ntext").style), (200.0, 0.0));
        let macro_form = adapted("\\documentclass{article}\n\\newcommand\\slp{\\sloppy}\n\\begin{document}\\slp text\\end{document}");
        assert_eq!(loose(&macro_form.style), (9999.0, 30.0));
        // `\fussy` after the class's own two-column `\sloppy` turns it back
        // off; the byte scan could only ever raise the tolerance.
        let fussy = adapted("\\documentclass[twocolumn]{article}\n\\begin{document}\\fussy text\\end{document}");
        assert_eq!(loose(&fussy.style), (200.0, 0.0));
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
    fn table_lengths_are_read_with_their_group_scope() {
        let src = "\\documentclass{article}\\setlength{\\tabcolsep}{3pt}\\begin{document}\
                   {\\setlength{\\tabcolsep}{4pt}A}B\\begin{center}\\setlength{\\tabcolsep}{5pt}C\\end{center}D\
                   \\begingroup\\setlength{\\tabcolsep}{7pt}\\{E\\endgroup F % \\setlength{\\tabcolsep}{9pt}\nG\\end{document}";
        let at = |marker: &str| src.find(marker).unwrap();
        let sep = |marker: &str| length_at(src, "tabcolsep", 10, at(marker), 6.0);
        assert_eq!(sep("\\begin{document}"), Some(3.0));
        assert_eq!(sep("A}"), Some(4.0));
        assert_eq!(sep("B\\begin"), Some(3.0));
        assert_eq!(sep("C\\end"), Some(5.0));
        assert_eq!(sep("D\\begingroup"), Some(3.0));
        // `\{` is an escaped brace, not a group.
        assert_eq!(sep("E\\endgroup"), Some(7.0));
        assert_eq!(sep("F %"), Some(3.0));
        // A commented-out assignment is not one.
        assert_eq!(sep("G\\end"), Some(3.0));
        assert_eq!(length_at(src, "arrayrulewidth", 10, src.len(), 0.4), None);
    }

    #[test]
    fn table_lengths_are_read_in_every_assignment_form() {
        let src = "\\begin{document}\\setlength\\tabcolsep{2pt}A\\addtolength{\\tabcolsep}{3pt}B{\\tabcolsep=1pt C}{\\tabcolsep 1.5pt D}\\newcommand{\\tight}{\\setlength{\\tabcolsep}{0pt}}E\\end{document}";
        let at = |marker: &str| src.find(marker).unwrap();
        let sep = |marker: &str| length_at(src, "tabcolsep", 10, at(marker), 6.0);
        assert_eq!(sep("A"), Some(2.0));
        assert_eq!(sep("B"), Some(5.0));
        assert_eq!(sep("C"), Some(1.0));
        assert_eq!(sep("D"), Some(1.5));
        // A definition's body is not an assignment until the macro is used.
        assert_eq!(sep("E"), Some(5.0));
        // `\addtolength` with nothing before it adds to the default.
        assert_eq!(length_at("\\addtolength{\\tabcolsep}{3pt}X", "tabcolsep", 10, 30, 6.0), Some(9.0));
    }

    #[test]
    fn table_lengths_follow_macro_invocations() {
        // pdflatex (fixture 128): `\tight` before a table narrows it exactly
        // as the `\setlength` in its body would.
        let src = concat!(
            "\\newcommand{\\tight}{\\setlength{\\tabcolsep}{0pt}}",
            "\\newcommand{\\widen}[1]{\\addtolength{\\tabcolsep}{#1}}",
            "\\newcommand{\\nested}{\\tight\\widen{2pt}}",
            "\\newcommand{\\scoped}{{\\setlength{\\tabcolsep}{20pt}}}",
            "\\newcommand{\\opt}[1][3pt]{\\setlength{\\tabcolsep}{#1}}",
            "\\begin{document}",
            "{\\tight @1}",
            "{\\widen{4pt}@2}",
            "{\\nested @3}",
            "{\\scoped @4}",
            "{\\tight\\opt[1pt]@5}",
            "\\end{document}"
        );
        let at = |marker: &str| src.find(marker).unwrap();
        let sep = |marker: &str| length_at_checked(src, "tabcolsep", 10, at(marker), 6.0);
        assert_eq!(sep("@1"), (Some(0.0), false));
        assert_eq!(sep("@2"), (Some(10.0), false));
        assert_eq!(sep("@3"), (Some(2.0), false));
        // The body's own group undoes its assignment.
        assert_eq!(sep("@4"), (None, false));
        // An optional argument is not read: the value is the one before the
        // invocation, and the caller is told so (`table_length_limitations`).
        assert_eq!(sep("@5"), (Some(0.0), true));
        let mut out = Vec::new();
        table_length_limitations(&[src], Span::new(at("@5"), at("@5") + 2), 10, &mut out);
        assert_eq!(out.len(), 1);
        assert!(out[0].2.contains("\\tabcolsep"), "{}", out[0].2);
    }

    /// The indexed lookup of an adapt call answers exactly what the scan of
    /// the source before the byte answers, at every byte: grouped, escaped
    /// and commented braces, environments, macros with and without
    /// arguments, an assigning macro inside another's argument, unclosed
    /// groups and non-ASCII text.
    #[test]
    fn table_length_index_matches_the_prefix_scan() {
        let sources = [
            "",
            "\\tabcolsep",
            "\\documentclass{article}\\setlength{\\tabcolsep}{3pt}\\begin{document}{\\setlength{\\tabcolsep}{4pt}A}B\\begin{center}\\setlength{\\tabcolsep}{5pt}C\\end{center}D\\begingroup\\setlength{\\tabcolsep}{7pt}\\{E\\endgroup F % \\setlength{\\tabcolsep}{9pt}\nG\\end{document}",
            "\\begin{document}\\setlength\\tabcolsep{2pt}A\\addtolength{\\tabcolsep}{3pt}B{\\tabcolsep=1pt C}{\\tabcolsep 1.5pt D}\\newcommand{\\tight}{\\setlength{\\tabcolsep}{0pt}}E\\addtolength\\tabcolsep{1pt}\\end{document}",
            concat!(
                "\\newcommand{\\tight}{\\setlength{\\tabcolsep}{0pt}}",
                "\\newcommand{\\widen}[1]{\\addtolength{\\tabcolsep}{#1}}",
                "\\newcommand{\\nested}{\\tight\\widen{2pt}}",
                "\\newcommand{\\scoped}{{\\setlength{\\tabcolsep}{20pt}}}",
                "\\newcommand{\\opt}[1][3pt]{\\setlength{\\tabcolsep}{#1}}",
                "\\begin{document}{\\tight @1}{\\widen{4pt}@2}{\\nested @3}{\\scoped @4}{\\tight\\opt[1pt]@5}",
                "\\widen{bad}z\\end{document}"
            ),
            "\\newcommand{\\tight}{\\setlength{\\tabcolsep}{0pt}}\\newcommand{\\widen}[1]{\\addtolength{\\tabcolsep}{#1}}\\widen{\\tight}x\\setlength{\\tabcolsep}{\\tight}y{\\widen{1pt}}z",
            "\\addtolength{\\tabcolsep}{3pt}X{\\addtolength{\\tabcolsep}{1pt}{\\addtolength{\\tabcolsep}{1pt}}\\addtolength{\\tabcolsep}{-1pt}}}}\\tabcolsep=2pt{{{\\tabcolsep 1pt",
            "é\\setlength{\\tabcolsep}{2pt}% } ü {\n\\{\\setlength{\\tabcolsep}{3pt}\\}}\\\\{\\setlength{\\arrayrulewidth}{1pt}\\end{x}\\setlength{\\tabcolsep}{4pt}\\beginx{\\endgroupx}\\%{\\tabcolsep=5pt%\n}",
        ];
        // And documents strung together from the same pieces at random.
        let pieces = [
            "{", "}", "\\{", "\\}", "\\\\", "%", "\n", " ", "x", "é", "\\begin{center}", "\\end{center}", "\\begingroup", "\\endgroup",
            "\\setlength{\\tabcolsep}{2pt}", "\\addtolength\\tabcolsep{1pt}", "\\tabcolsep=3pt", "\\tabcolsep", "\\tight", "\\widen{1pt}", "\\widen{",
            "\\newcommand{\\tight}{\\setlength{\\tabcolsep}{0pt}}", "\\newcommand{\\widen}[1]{{\\addtolength{\\tabcolsep}{#1}}\\addtolength{\\tabcolsep}{#1}}",
        ];
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let generated: Vec<String> = (0..300)
            .map(|_| {
                (0..24)
                    .map(|_| {
                        state ^= state << 13;
                        state ^= state >> 7;
                        state ^= state << 17;
                        pieces[(state % pieces.len() as u64) as usize]
                    })
                    .collect()
            })
            .collect();
        for source in sources.iter().copied().chain(generated.iter().map(String::as_str)) {
            let _scope = MacroDefsScope::enter(&[source]);
            for (name, base) in [("tabcolsep", 6.0), ("arrayrulewidth", 0.4), ("LTpre", 0.0)] {
                for at in (0..=source.len()).filter(|&at| source.is_char_boundary(at)) {
                    assert_eq!(length_at_checked(source, name, 10, at, base), length_at_scan(source, name, 10, at, base), "{name} at {at} of {source:?}");
                }
            }
            if sources.contains(&source) {
                // Only an assigning macro in another's argument is left to the scan.
                let scanned = MACRO_DEFS.with(|scope| scope.borrow()[0].lengths.by_name.values().filter(|i| i.is_none()).count());
                assert_eq!(scanned, usize::from(source.contains("\\widen{\\tight}")), "{source:?}");
            }
        }
    }

    /// A document of `tables` tables in the forms `length_at` reads: a
    /// preamble assignment, a macro, top-level `\addtolength`s, grouped and
    /// environment-scoped `\setlength`s, and `\tabcolsep=` inside a group.
    fn many_tables_source(tables: usize) -> (String, Vec<usize>) {
        let mut src = String::from("\\documentclass{article}\\setlength{\\tabcolsep}{3pt}\n\\newcommand{\\tight}{\\setlength{\\tabcolsep}{1pt}}\n\\begin{document}\n");
        let mut at = Vec::new();
        for k in 0..tables {
            src.push_str(&format!("\\section{{S{k}}} Some text % a comment {{\n\n"));
            let table = "\\begin{tabular}{|l|l|}\\hline a & b \\\\ \\hline\\end{tabular}\n\n";
            match k % 5 {
                0 => src.push_str(&format!("{{\\setlength{{\\tabcolsep}}{{{}pt}}", k % 7)),
                1 => src.push_str("\\begin{center}\\tight "),
                2 => src.push_str("\\addtolength{\\arrayrulewidth}{0.01pt}{"),
                3 => src.push_str("\\begingroup\\tabcolsep=2pt "),
                _ => src.push_str("{"),
            }
            at.push(src.len());
            src.push_str(table);
            src.push_str(match k % 5 {
                1 => "\\end{center}\n",
                3 => "\\endgroup\n",
                _ => "}\n",
            });
        }
        src.push_str("\\end{document}\n");
        (src, at)
    }

    /// Every table reads its lengths in one adapt call without rescanning the
    /// source before it: doubling the number of tables about doubles the
    /// time (a prefix scan per table grew it fourfold, #525/#623).
    ///
    /// This is a complexity guard, not a benchmark, so it uses an absolute
    /// ceiling rather than a `t800 < t200 * N` ratio. A ratio over a
    /// sub-millisecond baseline is fragile: on PR #765 CI this failed as
    /// "800 tables took 5.044542ms, 200 took 587.834us: not linear", and the
    /// identical commit passed on a bare re-run with no change -- a few
    /// hundred microseconds of scheduler noise is a large fraction of a
    /// ~600us baseline, and the error amplifies because the ratio's margin
    /// scales with the *smaller* operand. Measured on dev hardware, normal
    /// 800-table runs (best of 5) take ~1-1.3ms in `--release` and
    /// ~10-11ms unoptimized. Forcing `length_at_checked` to always fall
    /// through to `length_at_scan` (i.e. reverting #623 so every table
    /// rescans the source instead of using the per-document index) makes
    /// 800 tables take ~1.4s in `--release` -- about a thousandfold jump.
    /// 300ms sits roughly 250-300x above the normal case and ~5x below the
    /// reintroduced-quadratic case, so it stays quiet on a loaded machine
    /// and still fires if the per-table rescan comes back.
    #[test]
    fn table_lengths_scale_linearly_with_the_number_of_tables() {
        let resolve = |tables: usize| {
            let (src, at) = many_tables_source(tables);
            let mut best = std::time::Duration::MAX;
            let mut lengths = Vec::new();
            for _ in 0..5 {
                let t0 = std::time::Instant::now();
                let _scope = MacroDefsScope::enter(&[&src]);
                lengths = at.iter().map(|&a| crate::table::TableLengths::read(|name, base| length_at(&src, name, 10, a, base)).tabcolsep).collect::<Vec<_>>();
                best = best.min(t0.elapsed());
            }
            (best, lengths)
        };
        let (t200, l200) = resolve(200);
        let (t400, _) = resolve(400);
        let (t800, l800) = resolve(800);
        eprintln!("table lengths: 200 tables {t200:?}, 400 tables {t400:?}, 800 tables {t800:?}");
        assert_eq!(&l800[..200], &l200[..]);
        assert_eq!(&l200[..5], &[0.0, 1.0, 3.0, 2.0, 3.0]);
        assert!(
            t800 < std::time::Duration::from_millis(300),
            "800 tables took {t800:?} (200 took {t200:?}, 400 took {t400:?}): quadratic regression suspected, see #623"
        );
    }

    #[test]
    fn size_environments_give_their_size_until_a_declaration() {
        use flashtex_compiler::parser::FontSizeLevel as L;
        let src = "\\begin{document}\\begin{small}@a{\\Large @b}@c\\normalsize @d\\end{small}@e\\begin{Large}@f\\end{Large}\\end{document}";
        let styles = Styles::new(src);
        let at = |marker: &str| src.find(marker).unwrap();
        assert_eq!(styles.size_env_at(src, at("@a")), Some(L::Small));
        // A closed group's declaration is undone; one still open wins.
        assert_eq!(styles.size_env_at(src, at("@c")), Some(L::Small));
        assert_eq!(styles.size_env_at(src, at("@b")), None);
        assert_eq!(styles.size_env_at(src, at("@d")), None);
        assert_eq!(styles.size_env_at(src, at("@e")), None);
        assert_eq!(styles.size_env_at(src, at("@f")), Some(L::Large2));
    }

    #[test]
    fn glue_takes_the_size_in_force_at_the_command() {
        // The size of a `\quad`'s `em` is the compiler's font at the
        // command (PLAN1 site 5), which scopes groups and declarations the
        // way the removed gap scan approximated.
        use flashtex_compiler::parser::FontSizeLevel as L;
        let src = "\\documentclass{article}\\begin{document}{\\Large a}\\quad b a\\quad{\\Large b} a \\Large\\quad b\\end{document}";
        let parsed = flashtex_compiler::parser::parse(src);
        let sizes: Vec<Option<L>> = parsed
            .blocks
            .iter()
            .flat_map(|b| inlines_of(b).iter())
            .filter_map(|i| match i {
                Inline::TextGlue { style, .. } => Some(style.size),
                _ => None,
            })
            .collect();
        assert_eq!(sizes, [None, None, Some(L::Large2)]);
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
        let parsed = flashtex_compiler::parser::parse(&src);
        apply_preamble_lengths(
            &parsed.length_assignments,
            &src,
            0,
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
    #[ignore = "fails: panics at src/adapter.rs:7602: not implemented: scan \\input'd preambles at ae62d63d"]
    fn preamble_scan_does_not_see_input_files() {
        let src = "\\documentclass{article}\n\\input{layout}\n\\begin{document}x\\end{document}";
        let _ = adapted(src);
        panic!("not implemented: scan \\input'd preambles");
    }

    #[test]
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
                Item::Underline(_) => 'U',
                Item::Math { .. } => 'M',
                _ => '?',
            })
            .collect()
    }

    /// GH-919: a replacement text that is a box (`\uline{#1}`, `$x$`) has
    /// no word to search for in the body; the gap before it is the source
    /// before the invocation plus the body up to the box, and the gap after
    /// it starts past the invocation's arguments -- never inside `{a b}`.
    /// The unquoted `\ul{c} y` keeps its blanks, and only as many groups as
    /// the definition's arity are arguments: `\ul{a}{x}` sets `x` glued.
    ///
    /// (A `\newcommand` with a default argument is not covered: the
    /// tex-expansion engine expands it through `\@protected@testopt`, whose
    /// `\futurelet` drops the invocation origin, so its replacement tokens
    /// reach the parser with the definition's own spans and no
    /// `maps_to_invocation` — a separate defect of that seam.)
    #[test]
    fn macro_box_gaps_follow_the_source_around_the_invocation() {
        let src = "\\documentclass{article}\n\\usepackage[normalem]{ulem}\n\\newcommand{\\ul}[1]{\\uline{#1}}\n\\newcommand{\\R}{$x$}\n\\begin{document}\n\"\\ul{a b}\" x \\ul{c} y (\\R) z\n\n(\\ul{a}){x} y (\\ul{b}) z\n\\end{document}\n";
        let parsed = flashtex_compiler::parser::parse(src);
        let doc = adapt(&[src], 0, &parsed, &RenderOptions::default(), &Labels::default());
        let shapes: Vec<String> = doc
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::Paragraph { parts, .. } => Some(parts.iter().map(|p| if let ParaPart::Lines(items) = p { shape(items) } else { "D".to_string() }).collect()),
                _ => None,
            })
            .collect();
        // `(\ul{a}){x} y`: `)` and `x` glue into one word.
        assert_eq!(shapes, ["WUWSWSUSWSWMWSW", "WUWSWSWUWSW"]);
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
                Block::Letter { lines, .. } => lines.iter().map(|l| shape(l)).collect::<Vec<_>>().join("/"),
                Block::Picture { .. } => "P".to_string(),
                Block::Chapter { .. } => "C".to_string(),
                Block::Part { .. } => "P".to_string(),
                Block::Chrome { .. } => "M".to_string(),
                Block::Title { .. } => "T".to_string(),
                Block::ClearPage { .. } => "N".to_string(),
                Block::NoBreakFalse { .. } => "B".to_string(),
                Block::TocEntry(..) => "E".to_string(),
                Block::LongTable { .. } => "L".to_string(),
                Block::FrameBegin { .. } | Block::FrameEnd { .. } | Block::BeamerTitle { .. } | Block::BeamerToc { .. } => "F".to_string(),
                Block::BeamerBlockBegin { .. } | Block::BeamerBlockEnd { .. } | Block::ColumnsBegin { .. } | Block::Column { .. } | Block::ColumnsEnd { .. } => "F".to_string(),
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
        // One word per url.sty break run (`https:` `//` `example.` `org/` `x`).
        let it = items("A \\url{https://example.org/x} B");
        assert_eq!(families(&it), "rtttttr", "{it:?}");
        let it = items("A \\nolinkurl{https://example.org/x} B");
        assert_eq!(families(&it), "rtttttr", "{it:?}");
        // `\href` typesets only its second argument, in the ambient family.
        let it = items("A \\href{https://example.org/x}{link text} B");
        assert_eq!(families(&it), "rrrr", "{it:?}");
    }

    /// `-{}-`: the empty group is a token TeX's lig/kern lookahead stops at,
    /// so the two hyphens are two segments (shaped apart, no `--` ligature);
    /// a closing brace alone (`{Experi}ence`) keeps one segment, so the word
    /// is still hyphenated whole (`push_segment_in`).
    #[test]
    fn an_empty_group_splits_a_word_into_two_segments() {
        let segs = |src: &str| -> Vec<Vec<String>> {
            items(src)
                .iter()
                .filter_map(|i| match i {
                    Item::Word(w) => Some(w.segments.iter().map(|s| s.text.clone()).collect()),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(segs("\\texttt{-{}-set} f{}ine"), [vec!["-", "-set"], vec!["f", "ine"]]);
        assert_eq!(segs("{Experi}ence"), [vec!["Experience"]]);
    }

    /// A URL's argument is read as raw source bytes (url.sty makes every
    /// character "other"; the compiler does the same in
    /// `parser::url_argument`), so the style scan must not interpret what is
    /// inside it. A `%` used to start a comment and swallow the rest of the
    /// line, losing the style of everything after the URL.
    #[test]
    fn a_percent_or_brace_inside_a_url_does_not_disturb_a_later_font_command() {
        let it = items("A \\url{https://e.org/a%20b} \\textbf{bold} C");
        assert_eq!(families(&it), "rtttttrr", "{it:?}");
        let Item::Word(bold) = it.iter().filter(|i| matches!(i, Item::Word(_))).nth(6).unwrap() else {
            panic!()
        };
        assert_eq!(bold.text(), "bold");
        assert!(bold.segments[0].style.bold, "the \\textbf after the URL is still bold: {bold:?}");
        // A brace pair inside the URL is balanced, not a group.
        let it = items("A \\url{https://e.org/{x}} \\textbf{bold} C");
        assert_eq!(families(&it), "rtttttrr", "{it:?}");
    }

    /// A tie inside a word the compiler's ligature pass shortened
    /// (`pp.~1119--1184,`: `--` is one U+2013, so the text is a byte shorter
    /// than its span) is still the source's own `~`, i.e. an unbreakable
    /// interword space and not a tilde glyph (`ligature_char_sources`).
    #[test]
    fn a_tie_next_to_an_input_ligature_is_still_a_tie() {
        let it = items("pp.~1119--1184, 1981.");
        let shape: Vec<String> = it
            .iter()
            .map(|i| match i {
                Item::Word(w) => w.text(),
                Item::Space { no_break: true, .. } => "~".to_string(),
                Item::Space { .. } => " ".to_string(),
                other => format!("{other:?}"),
            })
            .collect();
        assert_eq!(shape, ["pp.", "~", "1119\u{2013}1184,", " ", "1981."], "{it:?}");
        // The en dash's source is both hyphens.
        let Item::Word(w) = &it[2] else { panic!() };
        let dash = w.segments[0].chars[4];
        assert_eq!((dash.start, dash.end), (8, 10));
        assert_eq!(
            ligature_char_sources("a``b''", Span::new(0, 6), "a\u{201C}b\u{201D}").map(|s| s.iter().map(|c| (c.start, c.end)).collect::<Vec<_>>()),
            Some(vec![(0, 1), (1, 3), (3, 4), (4, 6)])
        );
        assert_eq!(ligature_char_sources("\\today", Span::new(0, 6), "September 19, 2026"), None);
    }

    /// The whitespace after a control word is eaten whether it is blanks,
    /// the end of the line, or the end of the line plus the next line's
    /// indentation (`gap_has_space_after_control_word`): `15213 \quad
    /// $\cdot$ \quad\n  \href{..}{..}` is glue, quad, math, glue, quad,
    /// text — pdflatex's contact line of `fixtures/real-world/cv`.
    #[test]
    fn the_indentation_of_the_line_after_a_control_word_is_no_space() {
        let shape = |src: &str| -> String {
            items(src)
                .iter()
                .map(|i| match i {
                    Item::Word(w) => w.text(),
                    Item::Space { .. } => " ".to_string(),
                    Item::Quad { .. } => "<quad>".to_string(),
                    Item::Math { .. } => "<math>".to_string(),
                    other => format!("{other:?}"),
                })
                .collect()
        };
        assert_eq!(shape("15213 \\quad $\\cdot$ \\quad\n  (412)"), "15213 <quad><math> <quad>(412)");
        assert_eq!(shape("A \\quad\n  \\href{mailto:x@y.z}{x@y.z} B"), "A <quad>x@y.z B");
        assert_eq!(shape("A \\quad B"), "A <quad>B");
    }

    /// `\end{proof}` appends amsthm.sty 273-279's `\qed`: the blank before
    /// it is `\unskip`ped, then `\penalty9999 \hbox{}\nobreak\hfill\quad`
    /// and the symbol box — no interword glue, and a `\quad` in front.
    #[test]
    fn the_end_of_a_proof_is_amsthm_s_qed_list() {
        let src = "\\begin{proof}\nHence by zero.\n\\end{proof}";
        let parsed = flashtex_compiler::parser::parse(src);
        let doc = adapt(&[src], 0, &parsed, &RenderOptions::default(), &Labels::default());
        let Block::Paragraph { parts, .. } = &doc.blocks[0] else { panic!("{:?}", doc.blocks) };
        let ParaPart::Lines(it) = &parts[0] else { panic!() };
        let tail: Vec<String> = it
            .iter()
            .skip_while(|i| !matches!(i, Item::Word(w) if w.text() == "zero."))
            .map(|i| match i {
                Item::Word(w) => w.text(),
                Item::Penalty { value, .. } => format!("<{value}>"),
                Item::LeaveVmode => "<hbox>".to_string(),
                Item::HFill { fill: true, leader: FillLeader::None, .. } => "<hfill>".to_string(),
                Item::Quad { em, .. } => format!("<quad {em}>"),
                Item::QedBox { .. } => "<qed>".to_string(),
                Item::Space { .. } => "<space>".to_string(),
                other => format!("{other:?}"),
            })
            .collect();
        assert_eq!(tail, ["zero.", "<9999>", "<hbox>", "<10000>", "<hfill>", "<quad 1>", "<qed>"], "{it:?}");
    }

    /// A URL breaks where TeX's math-list penalties fall
    /// (`url_break_penalty`): `\relpenalty` 500 after the `:` (a Rel),
    /// `\binoppenalty` 700 after a Bin that an Ord precedes, and nothing
    /// after the first `/` of `://` (a Bin after a Rel is an Ord) or after
    /// the hyphen (an Ord, with url.sty's 0.5pt kern). pdflatex (TeX Live
    /// 2026, 11pt `article`, T1, hyperref) breaks
    /// `\url{https://example.org/ftxc/issues}` after the last `/` at
    /// `p=700` (`\tracingparagraphs`: `@\penalty via @@1 b=7 p=700
    /// d=490289`), and the space after a URL ending in `.` is an ordinary
    /// one (math mode leaves the space factor at 1000).
    #[test]
    fn a_url_carries_tex_s_math_list_penalties_between_its_runs() {
        fn shape(items: &[Item]) -> String {
            items
                .iter()
                .map(|i| match i {
                    Item::Word(w) => w.text(),
                    Item::Penalty { value, flagged: false } => format!("<{value}>"),
                    Item::Kern { .. } => "<kern>".to_string(),
                    Item::Space { factor, .. } => format!(" ({factor}) "),
                    other => format!("{other:?}"),
                })
                .collect()
        }
        let it = items("at \\url{https://example.org/ftxc/issues} rather");
        assert_eq!(shape(&it), "at (1000) https:<500>//<700>example.<700>org/<700>ftxc/<700>issues (1000) rather", "{it:?}");
        // A hyphen is no break; a run ending the URL gets no penalty; the
        // space factor after the URL's `.` is 1000, not 3000.
        let it = items("x \\url{a-b.} y");
        assert_eq!(shape(&it), "x (1000) a-<kern>b. (1000) y", "{it:?}");
        assert_eq!(url_break_penalty("https:", '/'), Some(URL_BIG_BREAK_PENALTY));
        assert_eq!(url_break_penalty("https:/", '/'), None);
        assert_eq!(url_break_penalty("https://", 'e'), Some(URL_BREAK_PENALTY));
        assert_eq!(url_break_penalty("a/", ':'), None, "a Bin followed by a Rel is an Ord");
        assert_eq!(url_break_penalty("a:", ':'), None, "no penalty between two Rels");
        assert_eq!(url_break_penalty("a?", '&'), Some(URL_BREAK_PENALTY));
        assert_eq!(url_break_penalty("a?&", 'b'), None, "the `&` after a Bin is an Ord");
    }

    /// The typewriter family covers the whole `\url{...}` and nothing after
    /// it: the compiler sets every run it splits the URL into in `\ttfamily`
    /// over the font around it (`parser::P::url_text`), and the text after
    /// the command is back in the outer font.
    #[test]
    fn the_url_runs_are_typewriter_and_the_text_after_them_is_not() {
        let it = items("x \\url{ab} y");
        assert_eq!(families(&it), "rtr", "{it:?}");
    }

    /// A macro body's declarations around `#k` cover argument `k` at each
    /// call, and nothing around the call; a redefinition nested inside the
    /// body does not panic. The compiler expands the macro, so the font
    /// comes with the tokens (`parser::TextStyle::font`).
    #[test]
    fn a_macro_body_declaration_covers_its_argument() {
        let it = items("\\newcommand{\\note}[1]{{\\bfseries #1}}\nA \\note{bold} C");
        let bold: Vec<(String, bool)> = it
            .iter()
            .filter_map(|i| match i {
                Item::Word(w) => Some(w.segments.iter().map(|s| (s.text.clone(), s.style.bold))),
                _ => None,
            })
            .flatten()
            .collect();
        assert_eq!(bold, vec![("A".to_string(), false), ("bold".to_string(), true), ("C".to_string(), false)], "{it:?}");
        let _ = items("\\newcommand{\\a}[1]{\\def\\a{x}{\\bfseries #1}}\n\\a{y} z");
    }

    /// `\verb`, the `verbatim` environment and `lstlisting` are set in the
    /// typewriter family, like `\url`: the compiler marks the run
    /// (`parser::Inline::Verbatim`'s `\verbatim@font` style,
    /// `Block::Verbatim`).
    ///
    /// `\lstinline` is not `\verb`: listings sets it in the `basicstyle`
    /// face, and the default `basicstyle={}` changes nothing, so the face
    /// around the command stays (`listings::apply`). pdflatex (11 pt
    /// article, `\usepackage{listings}`, no `\lstset`): `A \lstinline|xy|
    /// B` sets `xy` in `SFRM1095`, `\textsf{sans \lstinline|s| here}` sets
    /// `s` in `SFSS1095`; with `\lstset{basicstyle=\ttfamily\small}` the
    /// `listings-manual` reference sets its inlines in `SFTT1000`.
    #[test]
    fn verbatim_constructs_are_set_in_the_typewriter_family() {
        assert_eq!(families(&items("A \\verb|x| B")), "rtr");
        assert_eq!(families(&items("A \\verb*|x| B")), "rtr");
        assert_eq!(families(&items("A \\lstinline|x| B")), "rrr");
        assert_eq!(families(&items("A \\lstinline[language=C]|x| B")), "rrr");
        assert_eq!(families(&items("A \\lstinline[basicstyle=\\ttfamily]|x| B")), "rtr");
        assert_eq!(families(&items("\\lstset{basicstyle=\\ttfamily\\small}\nA \\lstinline|x| B")), "rtr");
        assert_eq!(families(&items("\\textsf{A \\lstinline|x| B}")), "sss");
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

    /// `reading_order`: nested reads, `.tex` lookup, `\includeonly` (as
    /// written or with `.tex`), cycles and unknown files.
    #[test]
    fn reading_order_follows_the_include_tree() {
        let main = "\\documentclass{report}\n% \\includeonly{nothing}\n\\includeonly{a.tex, c}\n\\begin{document}\nX\\include{a}Y\\include{b}Z\\input{missing}\\include{c}W\\end{document}";
        let a = "A1\\input{sub/n}A2";
        let n = "N\\input{a}";
        let b = "B";
        let c = "C";
        let texts = [c, n, main, b, a];
        let paths = ["c.tex", "sub/n.tex", "main.tex", "b.tex", "a.tex"];
        let got: Vec<(usize, &str)> = reading_order(&texts, &paths, 2).iter().map(|s| (s.document.0, &texts[s.document.0][s.start..s.end])).collect();
        let at = main.find("\\begin").unwrap();
        let entry_head = &main[..main.find("\\include{a}").unwrap()];
        assert!(entry_head.len() > at);
        assert_eq!(
            got,
            [
                (2, entry_head),
                (4, "A1"),
                (1, "N\\input{a}"),
                (4, "A2"),
                (2, "Y\\include{b}Z\\input{missing}"),
                (0, "C"),
                (2, "W\\end{document}"),
            ]
        );
        assert_eq!(reading_position(&reading_order(&texts, &paths, 2), DocumentId(0), 0), Some(got[..5].iter().map(|(_, t)| t.len()).sum()));
        assert_eq!(reading_position(&reading_order(&texts, &paths, 2), DocumentId(3), 0), None);
    }
}
