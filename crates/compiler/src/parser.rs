//! Parser for the documented LaTeX subset.
//!
//! Honest boundary: this is a finite grammar, not TeX. It recognises the common
//! LaTeX preamble and implements bounded `\newcommand`/`\renewcommand`
//! expansion, but there is no category-code mutation, register, conditional,
//! package loading, or general environment implementation.

use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use crate::bib;
use crate::date::TodayDate;
use crate::color::{Colors, DeviceColor};
use crate::diagnostics::Diagnostic;
use crate::expansion::{self, ExpansionSite};
use crate::lexer::{apply_text_ligatures, tokenize_document, Token, TokenKind};
#[cfg(test)]
use crate::lexer::tokenize;
use crate::math::{self, MathList, MathPackages};
use crate::natbib;
use crate::siunitx;
use crate::text_builtins::{self, AccentOutcome, SymbolOutcome, TextDimen, TextLogo, TextRule};
use crate::theorems::{self, TheoremDef, TheoremStyle};
use crate::vocabulary;
use crate::{DocumentId, Span};
use flashtex_tex_text_encoding::encoding::Encoding;

mod colors;
mod lists;
mod tabular;

pub use lists::{
    CounterStyle, ItemLabel, ListEnvironment, ListFrame, ListLength, ListOption, ListSkip,
};

/// Maximum number of active nested `\input`/`\include` calls.
pub const INCLUDE_DEPTH_LIMIT: usize = 64;

/// Per-request inputs that are neither document text nor the entry path.
///
/// Today this is only the date `\today` renders. It is an input rather than
/// something this crate reads from the clock, because `docs/contracts/runtime-v1.md`
/// requires byte-identical output for byte-identical input and a compiler that
/// reads the clock is not a function of its inputs at all. The caller reads the
/// clock and sends the answer; see `crate::date` and
/// `protocol/proposals/runtime-v1-request-date.md`.
///
/// [`ParseOptions::default`] is the Unix epoch, so [`parse_project`] and
/// [`parse`] behave exactly as they always have.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ParseOptions {
    /// What `\today` expands to, and what `\maketitle` uses when the document
    /// has no `\date` of its own.
    pub today: TodayDate,
}

/// One project document supplied by the runtime compile payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceDocument<'a> {
    pub path: &'a str,
    pub text: &'a str,
}

/// The material a fill's glue is filled with. latex.ltx:
/// `\def\hrulefill{\leavevmode\leaders\hrule\hfill\kern\z@}` and
/// `\def\dotfill{\leavevmode\cleaders\hb@xt@.44em{\hss.\hss}\hfill\kern\z@}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FillLeader {
    #[default]
    None,
    /// A 0.4pt rule on the baseline (`\hrule` in horizontal leaders).
    Rule,
    /// Periods centred in 0.44em boxes, the boxes centred in the glue (`\cleaders`).
    Dots,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text {
        text: String,
        span: Span,
        style: TextStyle,
        /// Whether the source had real whitespace (or nothing — start of a
        /// paragraph/group) immediately before this run, as opposed to
        /// sitting directly against whatever came before it (the far side
        /// of `$...$`, or a macro-argument splice such as `\normalfont[#2
        /// points]` gluing the literal `[` to the substituted digits).
        /// Real TeX never inserts an inter-word gap that is not present in
        /// the source; see `layout::LayoutCursor::place`.
        space_before: bool,
    },
    LineBreak {
        span: Span,
        /// `\\[<dimen>]`'s optional argument in TeX points, when the source
        /// carried one: latex.ltx's `\\@normalcr` ends the line and then
        /// `\\@xnewline` issues `\\vspace{<dimen>}`, so this is a real vertical
        /// skip after the broken line, and a negative one is as real as a
        /// positive one (`\\\\[-6pt]` is how a heading macro pulls a rule up
        /// under its title).
        ///
        /// The parser has always *consumed* this argument -- it must not
        /// reach the page as text -- but used to discard the value, leaving
        /// each consumer to re-read the `[...]` out of the source bytes that
        /// follow `span`. That works only for a `\\\\` written literally in the
        /// document: expanded from a macro body, `span` is the *invocation*
        /// (`crate::expansion::Converter::place`), so the bytes after it are
        /// the call's own arguments and the skip is invisible. Reporting the
        /// parsed value is the only way a consumer can see it at all.
        skip_pt: Option<f64>,
    },
    /// Explicit text-mode horizontal glue (`\quad` is 1em, `\qquad` is 2em),
    /// measured in ems of the surrounding body text size. Named distinctly
    /// from `HSpace` below (a fixed-point `\hspace{<dimen>}` glue) since the
    /// two behave differently at a line break: this discardable glue mirrors
    /// TeX by breaking the line rather than overflowing it (see
    /// `layout::LayoutCursor::text_glue`).
    TextGlue {
        em: f64,
        span: Span,
    },
    Math {
        list: MathList,
        display: bool,
        number: Option<String>,
        number_span: Option<Span>,
        span: Span,
        /// See `Inline::Text::space_before`.
        space_before: bool,
        /// The text colour where the formula starts (`TextStyle::color`).
        color: Option<DeviceColor>,
        /// Source ranges inside the formula recoloured by `\color` or
        /// `\textcolor`, merged per colour in source order; an atom takes
        /// the colour of the range containing its span, else `color`.
        color_ranges: Vec<(Span, DeviceColor)>,
    },
    /// A multi-row amsmath display (`gather`, `align` and their starred forms).
    /// `aligned` cells alternate right/left alignment around shared tab stops.
    MathRows {
        rows: Vec<MathRow>,
        aligned: bool,
        span: Span,
    },
    Label {
        key: String,
        value: String,
        /// cleveref's label type (`section`, `equation`, `figure`, ...).
        kind: String,
        span: Span,
    },
    Reference {
        key: String,
        page: bool,
        /// amsmath `\eqref`: the value is typeset in parentheses.
        equation: bool,
        span: Span,
        /// See `Inline::Text::space_before`.
        space_before: bool,
    },
    /// A `cleveref`/`hyperref` reference whose label names are resolved after
    /// the document has been laid out. The compiler has no link backend yet;
    /// `linked` preserves whether the source used the starred no-link form
    /// for the future pipeline consumer.
    CleverReference {
        keys: Vec<String>,
        page: bool,
        range: bool,
        label_only: bool,
        capitalise: bool,
        linked: bool,
        span: Span,
        /// See `Inline::Text::space_before`.
        space_before: bool,
    },
    /// `\hfill`/`\hfil`: infinite horizontal stretch. Multiple fills on one
    /// line share the line's leftover width equally, as real TeX glue does;
    /// unlike TeX, `\hfil` and `\hfill` are not distinguished by stretch
    /// order (this layout has only one order of infinite glue), an accepted
    /// simplification. See `layout::LayoutCursor::resolve_hfill`.
    HFill {
        span: Span,
        /// What fills the glue: nothing (`\hfill`), a rule (`\hrulefill`)
        /// or dots (`\dotfill`).
        leader: FillLeader,
    },
    /// `\hspace{<dimen>}`/`\hspace*{<dimen>}`: a fixed, non-stretching space.
    /// `pt` is already converted (see `parse_dimen_pt`). Real TeX also lets
    /// plain `\hspace` glue (unlike the starred form) be discarded when it
    /// falls at a line break; this layout never discards glue at a line
    /// start, so both forms behave identically here.
    HSpace {
        pt: f64,
        span: Span,
    },
    /// `\footnote[<n>]{..}`, `\footnotemark[<n>]` or `\footnotetext[<n>]{..}`.
    /// `number` is the resolved `\thefootnote` (arabic). `span` is the
    /// command token, attributed to both superscript marks. `mark` is false
    /// only for `\footnotetext`; `text` is `None` only for `\footnotemark`.
    /// Page-bottom placement lives in `layout::footnotes`.
    Footnote {
        number: String,
        span: Span,
        mark: bool,
        text: Option<Vec<Inline>>,
        /// See `Inline::Text::space_before`.
        space_before: bool,
    },
    /// `\TeX`, `\LaTeX`, `\LaTeXe`: the kernel logo construction (kerns,
    /// a lowered `E`, a raised script-size `A`; see
    /// `text_builtins::layout_logo`), laid out against each layout's own
    /// font metrics. `span` is the command.
    Logo {
        logo: TextLogo,
        span: Span,
        style: TextStyle,
        /// See `Inline::Text::space_before`.
        space_before: bool,
    },
    /// `\rule[<raise>]{<width>}{<height>}` in text: an unbreakable box
    /// holding a filled rectangle (see `text_builtins::TextRule::resolve`).
    /// `span` covers the command and its arguments.
    Rule {
        rule: TextRule,
        span: Span,
        style: TextStyle,
        /// See `Inline::Text::space_before`.
        space_before: bool,
    },
    /// A text-mode kern (`\,`, `\thinspace`, `\enspace`, ...; see
    /// `text_builtins::text_kern`): fixed, not a break point unless glue
    /// follows it, resolved against the current font's quad at layout time.
    Kern {
        amount: TextDimen,
        span: Span,
        style: TextStyle,
    },
    /// `tabular`/`tabular*`: an inline box (see `crate::tabular`).
    Tabular(Box<crate::tabular::Tabular>),
    /// `\verb`/`\verb*` sitting inline in running text: an unbreakable run of
    /// literal Courier text (already tab-expanded, and with visible dots for
    /// starred spaces — see `verbatim_display`). Ligatures are never applied.
    /// Wraps to a new line like an ordinary long word rather than breaking
    /// internally, since it is placed as a single `Inline::Text`-shaped item.
    Verbatim {
        text: String,
        span: Span,
        /// See `Inline::Text::space_before`.
        space_before: bool,
    },
    /// xcolor `\colorbox`/`\fcolorbox` (see [`ColorBox`]).
    ColorBox(Box<ColorBox>),
    /// ulem `\uline`/`\sout` or kernel text-mode `\underline`: the argument
    /// as one fragment with a rule. First step: the fragment does not
    /// break across lines (ulem's leaders can). Geometry is [`Underline::geom`].
    Underline(Box<Underline>),
    /// `\includegraphics` in running text: an image box (see
    /// `crate::graphics`). Figures and tables re-derive their graphics from
    /// the source instead.
    Graphic(Box<crate::graphics::Graphic>),
    /// `\scalebox`, `\resizebox`, `\rotatebox`, `\reflectbox` around
    /// horizontal material (see `crate::graphics`).
    Transform(Box<crate::graphics::TransformBox>),
}

/// ulem.sty `\def\ULthickness{.4pt}`.
pub const UL_THICKNESS_PT: f64 = 0.4;

/// cmex10 `\fontdimen8` (TeX `default_rule_thickness`). pdflatex shows
/// `0.39998pt`; article 12pt still uses unscaled cmex10, so \theta is
/// the same at 10pt and 12pt.
pub const MATH_RULE_THETA_PT: f64 = 0.39998;

/// cmr x-height / design size. pdflatex: 4.30554pt at 10pt, 5.16667pt at
/// 12pt. Used for ulem `\sout`'s `-.55ex` (not Core 14 Times x-height).
pub const CMR_EX_PER_EM: f64 = 0.430554;

/// ulem.sty `\def\sout{\bgroup \ULdepth=-.55ex \ULset}`.
pub const SOUT_RAISE_EX: f64 = 0.55;

/// How [`Underline`] places its rule. Thickness is [`Underline::thickness_pt`].
///
/// Offsets are positive downward from the content baseline. Core 14 has no
/// per-glyph TFM: `\uline` uses the cmr/lmr 0.25em `(` depth and kernel
/// `\underline` uses hbox depth 0 (true for the no-descender test words).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnderlineGeom {
    /// ulem `\uline`: rule top at `\dp` of `\hbox{{(j}}` (0.25em for cmr/lmr).
    /// pdflatex 10pt `rule(-2.5+2.9)`; 12pt `rule(-3.0+3.4)`.
    UlemDescender,
    /// latex.ltx text `\underline` = `$\@@underline{\hbox{#1}}$`. TeXbook
    /// Rule 10 / tex.web §735: kern 3\theta, rule \theta, extra depth \theta
    /// (total depth = box depth + 5\theta). Rule top is 3\theta below the
    /// hbox depth. \theta = [`MATH_RULE_THETA_PT`].
    MathUnderline,
    /// ulem `\sout`: `\UL@setULdepth` is a no-op when `\ULdepth` is not
    /// `\maxdimen`, so `-.55ex` is kept. Leaders are
    /// `\hrule height (0.55ex+0.4pt) depth -0.55ex`: rule bottom 0.55ex
    /// above the baseline, thickness `\ULthickness`. pdflatex 10pt
    /// `rule(2.76805+-2.36806)`; 12pt `rule(3.24167+-2.84167)`.
    Strike,
}

impl UnderlineGeom {
    /// Rule top relative to the baseline (positive down) and the extra
    /// depth the construction adds below the baseline.
    ///
    /// `box_depth` is the hbox depth of the content; `descender` is `\dp`
    /// of `\hbox{{(j}}`; `ex` is the current x-height.
    pub fn rule_top_and_depth(
        self,
        thickness: f64,
        box_depth: f64,
        descender: f64,
        ex: f64,
    ) -> (f64, f64) {
        match self {
            Self::UlemDescender => (descender, descender + thickness),
            Self::MathUnderline => (
                box_depth + 3.0 * thickness,
                box_depth + 5.0 * thickness,
            ),
            Self::Strike => {
                let bottom_above = SOUT_RAISE_EX * ex;
                (-(bottom_above + thickness), 0.0)
            }
        }
    }
}

/// An underline / strike wrapper (`Inline::Underline`).
///
/// [`UnderlineGeom::UlemDescender`] is ulem `\uline` (`\ULthickness` 0.4pt,
/// top at 0.25em). [`UnderlineGeom::MathUnderline`] is kernel text
/// `\underline`. [`UnderlineGeom::Strike`] is ulem `\sout`. The fragment
/// does not break across lines.
#[derive(Debug, Clone, PartialEq)]
pub struct Underline {
    pub content: Vec<Inline>,
    pub thickness_pt: f64,
    pub geom: UnderlineGeom,
    /// From the command through the argument's closing brace.
    pub span: Span,
    /// See `Inline::Text::space_before`.
    pub space_before: bool,
}

/// `\colorbox[model]{fill}{text}` or `\fcolorbox[model]{frame}{fill}{text}`
/// (xcolor.sty 3.02 `\color@b@x`, `\XC@frameb@x`): `content` in an
/// unbreakable box, behind it a `fill` rectangle `\fboxsep` larger on
/// every side, and for `\fcolorbox` a `frame` of `\fboxrule` around that.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorBox {
    pub fill: DeviceColor,
    pub frame: Option<DeviceColor>,
    pub content: Vec<Inline>,
    /// `\fboxsep` and `\fboxrule` when the box was made, in TeX points.
    pub fboxsep_pt: f64,
    pub fboxrule_pt: f64,
    /// From the command through its last argument's closing brace.
    pub span: Span,
    /// See `Inline::Text::space_before`.
    pub space_before: bool,
}

/// One `\\`-separated row of a multi-row display; cells are split on `&`.
#[derive(Debug, Clone, PartialEq)]
pub struct MathRow {
    pub cells: Vec<MathList>,
    pub number: Option<String>,
    pub span: Span,
    /// `\intertext`/`\shortintertext` paragraphs set between the previous
    /// row and this one, in order.
    pub intertext: Vec<Intertext>,
}

/// amsmath `\intertext{..}` (`amsmath.sty` 1186-1199 `\intertext@`) or
/// mathtools `\shortintertext{..}` (`mathtools.sty` 1464-1529): a
/// `\noindent` paragraph in a `\noalign` between two alignment rows.
#[derive(Debug, Clone, PartialEq)]
pub struct Intertext {
    pub content: Vec<Inline>,
    /// `\shortintertext`: the short display skips.
    pub short: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Paragraph(Vec<Inline>),
    Heading {
        level: u8,
        number: String,
        number_span: Span,
        content: Vec<Inline>,
    },
    FigureCaption {
        content: Vec<Inline>,
    },
    /// A paragraph inside `center`, `flushleft`, `flushright`, `quote`,
    /// `quotation` or `verse` (the last three report `ParagraphStyle::Quote`;
    /// `lists` tells them apart).
    Styled {
        style: ParagraphStyle,
        content: Vec<Inline>,
        /// Every enclosing `\list`-based environment, outermost first
        /// (a `quote` inside an `itemize` item has both).
        lists: Vec<ListFrame>,
        /// `verse` only: this paragraph was started by the previous line's
        /// `\\` (article.cls `\let\\\@centercr`: `\par`,
        /// `\addvspace{-\parskip}`, then the optional `\vskip`), not by a
        /// blank line.
        line_break_before: Option<LineBreakBefore>,
    },
    /// One paragraph of an `itemize`/`enumerate` `\item`. `level` (1 =
    /// outermost) drives the hanging-indent margin; `label` carries the
    /// marker text and the `\item` span, and is `None` for a continuation
    /// paragraph of the same item (a blank line inside `\item`'s text) so the
    /// marker is not repeated while the hanging indent still applies.
    /// `extra_gap_before_pt`/`extra_gap_after_pt` are `\setlist`
    /// itemsep/topsep overrides (`0.0` without `\setlist`).
    ListItem {
        level: u8,
        label: Option<(String, Span)>,
        content: Vec<Inline>,
        /// Extra gap before this item, beyond the ordinary paragraph gap:
        /// `topsep` before the list's first item, `itemsep` before the rest.
        extra_gap_before_pt: f64,
        /// Extra gap after this item: `topsep`, set only on the list's last
        /// item.
        extra_gap_after_pt: f64,
        /// `\setlist{leftmargin=...}`'s effect on this level's own share of
        /// the cumulative hanging-indent margin (`Default` outside
        /// `\setlist`, or when the level's default `LIST_LEFTMARGIN_EM`
        /// share applies unchanged).
        leftmargin: ListLeftMargin,
        /// `thebibliography`'s widest-label argument (`\begin{thebibliography}{99}`'s
        /// `"99"`), overriding `level`'s hanging indent with `\labelwidth` +
        /// `\labelsep` measured from `[<text>]`, exactly like real LaTeX's
        /// `\settowidth\labelwidth{\@biblabel{#1}}`. `None` for an ordinary
        /// `itemize`/`enumerate` item, which keeps using `level`'s indent.
        widest_label: Option<String>,
        /// Every enclosing `\list`-based environment, outermost first; the
        /// last is the list this item belongs to.
        lists: Vec<ListFrame>,
        /// How the label was produced (`None` exactly when `label` is: a
        /// later paragraph of the same item).
        item: Option<ItemLabel>,
    },
    /// `\vspace{<dimen>}`: additional vertical glue, in points.
    VSpace {
        pt: f64,
    },
    /// `\hrule`: a full-measure-width rule at the current line.
    Rule {
        span: Span,
    },
    /// `\newpage`: force the next block onto a fresh page.
    PageBreak,
    /// `verbatim`/`verbatim*` and basic `lstlisting`: literal, unreflowed
    /// Courier text at body size, one output line per source line, set off
    /// from surrounding paragraphs the way `\trivlist`'s `\topsep` does (see
    /// `layout::VERBATIM_TOPSEP_PT`). `span` covers the whole environment,
    /// from `\begin` through `\end`, so an edit anywhere inside it correctly
    /// invalidates the cached block (see `incremental::shift_block`).
    Verbatim {
        lines: Vec<VerbatimLine>,
        span: Span,
    },
    /// `\tableofcontents`: the article.cls contents list, built from the
    /// numbered headings of the previous layout pass (see
    /// `layout::layout_converged`). `span` is the command.
    TableOfContents {
        span: Span,
    },
    /// `\maketitle`: `article.cls`'s `\@maketitle` (title/author/date block).
    /// `title`/`authors` are already-resolved inline content (never empty —
    /// `\maketitle` fails with a diagnostic instead, see `P::maketitle`);
    /// `date` is `None` exactly when `\date{}` suppressed the date line
    /// (`\@date` empty), matching `flashtex_title_layout::DateField`. Layout
    /// (exact `\vskip` amounts, `\LARGE`/`\large` sizes, and the leading
    /// `\newpage`) lives in `layout::LayoutCursor`'s own `TitleBlock` arms;
    /// see those for the `\@maketitle` provenance this transcribes.
    TitleBlock {
        title: Vec<Inline>,
        authors: Vec<Inline>,
        date: Option<Vec<Inline>>,
    },
    /// `\vfill`: vertical glue that stretches to fill whatever room is left
    /// on the current page, computed at layout time from the cursor's
    /// actual position (unlike `VSpace`'s flat, parse-time amount).
    VFill,
    /// A `letter.cls` block whose horizontal placement no [`ParagraphStyle`]
    /// expresses: the return address, which is a *left-aligned box pushed to
    /// the right margin* (not a ragged-left column — `\opening` sets it in a
    /// `tabular{l@{}}` inside `\raggedleft`, so every line shares one left
    /// edge), and the closing/signature, which sits at `\longindentation`
    /// inside a `\parbox{\indentedwidth}`.
    ///
    /// `lines` are broken exactly where the source's `\\` put them and are
    /// not re-wrapped, matching the `tabular` and `\parbox` they come from.
    /// `extra_gap_after_pt` is the class's own extra leading after line `i`
    /// (`\\*[2\parskip]` between address and date, `\\[6\medskipamount]`
    /// between closing and signature); it is parallel to `lines`.
    LetterBlock {
        part: LetterPart,
        lines: Vec<Vec<Inline>>,
        extra_gap_after_pt: Vec<f64>,
        /// The class's own `\vspace` before the block, beyond the ordinary
        /// `\parskip` every paragraph takes.
        gap_before_pt: f64,
        /// The class's own `\vspace` after the block. Carried here rather
        /// than as a separate `Block::VSpace` so that the next block sees a
        /// line this one already closed (`layout`'s `closed_line_skip`) and
        /// does not open a second one.
        gap_after_pt: f64,
        /// Fixed left offset from the text margin, in points:
        /// `\longindentation` for [`LetterPart::Closing`], zero otherwise.
        /// A *class* length (see `letter_longindentation_pt`), so it is
        /// resolved here rather than from whatever measure the layout has.
        indent_pt: f64,
        span: Span,
    },
}

/// Which `letter.cls` block a [`Block::LetterBlock`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LetterPart {
    /// `\opening`'s `{\raggedleft ... \par}`: `\fromaddress`'s lines and
    /// then `\@date`, as one box whose *right* edge is the right margin and
    /// whose lines all start at the box's own left edge. With no
    /// `\address` the box holds the date alone.
    ReturnAddress,
    /// `\opening`'s `{\raggedright \toname \\ \toaddress \par}`: the
    /// recipient at the left margin, `2\parskip` clear of the date above and
    /// of the salutation below.
    Recipient,
    /// `\closing`'s `\hspace*{\longindentation}\parbox{\indentedwidth}{...}`:
    /// the closing line, `6\medskipamount`, then `\fromsig` (or `\fromname`).
    Closing,
}

/// verse's `\\` (`\@centercr`, latex.ltx `\@xcentercr`/`\@icentercr`).
#[derive(Debug, Clone, PartialEq)]
pub struct LineBreakBefore {
    /// The `\\` (with its `*` and `[<dimen>]`).
    pub span: Span,
    /// `\\[<dimen>]`, in TeX points.
    pub skip_pt: Option<f64>,
    /// `\\*` (`\nobreak`).
    pub star: bool,
}

/// One physical source line of a `Block::Verbatim`. `text` is already
/// tab-expanded (and dot-marked for a starred environment); `span` is the
/// exact original source bytes for that line, excluding its trailing `\n`.
#[derive(Debug, Clone, PartialEq)]
pub struct VerbatimLine {
    pub text: String,
    pub span: Span,
}

/// `\setlist{leftmargin=...}`'s effect on a `Block::ListItem`'s own
/// contribution to the cumulative hanging-indent margin (see
/// `layout::list_margin_pt`); enclosing levels' shares are unaffected.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ListLeftMargin {
    /// No override: the level's default `LIST_LEFTMARGIN_EM` share applies.
    #[default]
    Default,
    /// `leftmargin=<dimen>`, already resolved to points.
    Explicit(f64),
    /// `leftmargin=*`: every distinct label text that can appear in this
    /// list, resolved once every `\item` in it has been seen (enumitem picks
    /// the widest of these once the labels are known — see `set_list`);
    /// `layout` measures each at the body size and adds `\labelsep`.
    Widest(Vec<String>),
}

/// Font selection for one text item, as set by `\textbf`, `\itshape`, etc.
/// Slanted shapes (`\textsl`, `\slshape`) are recorded as italic: the Core 14
/// faces have no slanted Times.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct TextStyle {
    pub bold: bool,
    pub italic: bool,
    pub family: TextFamily,
    /// The active `\tiny`..`\Huge` declaration, if any (`None` is
    /// `\normalsize`, the body size). Resolved to an actual point size in
    /// `layout::size_declaration_pt`, against the layout's own body size
    /// rather than here, since that is the one authoritative value.
    pub size: Option<FontSizeLevel>,
    /// The text colour (`\color`, `\textcolor`), scoped like the face.
    /// `None` is the page's default colour: pdfTeX writes no operator.
    /// `Some` carries the exact operator values (`crate::color`).
    pub color: Option<DeviceColor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum TextFamily {
    #[default]
    Roman,
    Sans,
    Mono,
}

/// One `\tiny`..`\Huge` declaration level. Scoped on `TextStyle` exactly like
/// bold/italic/family, via the same group/environment style stack, rather
/// than a parallel size stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontSizeLevel {
    Tiny,
    ScriptSize,
    FootnoteSize,
    Small,
    /// `\large`.
    Large1,
    /// `\Large`.
    Large2,
    /// `\LARGE`.
    Large3,
    /// `\huge`.
    Huge1,
    /// `\Huge`.
    Huge2,
}

impl TextStyle {
    pub const BOLD: TextStyle = TextStyle {
        bold: true,
        italic: false,
        family: TextFamily::Roman,
        size: None,
        color: None,
    };
}

/// Argument-taking style commands (`\textbf{...}`).
pub(crate) fn style_command(name: &str) -> bool {
    matches!(
        name,
        "textbf"
            | "textmd"
            | "textit"
            | "textsl"
            | "textsc"
            | "textup"
            | "emph"
            | "texttt"
            | "textrm"
            | "textsf"
            | "textnormal"
    )
}

/// Group-scoped style declarations (`\bfseries`, `{\bf ...}`). `\tiny`..
/// `\Huge` are declarations too, not argument-taking commands: `\Large{...}`
/// (a common `\textbf{...}`-style misuse) is deliberately handled the same
/// way as `{\Large ...}` — its size stays active past the immediate group,
/// matching real LaTeX (the group only undoes assignments made *inside* it).
pub(crate) fn style_declaration(name: &str) -> bool {
    matches!(
        name,
        "bfseries"
            | "mdseries"
            | "itshape"
            | "slshape"
            | "scshape"
            | "upshape"
            | "ttfamily"
            | "rmfamily"
            | "sffamily"
            | "normalfont"
            | "em"
            | "bf"
            | "it"
            | "sl"
            | "sc"
            | "tt"
            | "rm"
            | "sf"
            | "tiny"
            | "scriptsize"
            | "footnotesize"
            | "small"
            | "normalsize"
            | "large"
            | "Large"
            | "LARGE"
            | "huge"
            | "Huge"
    )
}

/// The style after applying one style command or declaration to `style`.
fn apply_style(style: TextStyle, name: &str) -> TextStyle {
    let mut next = style;
    match name {
        "textbf" | "bfseries" => next.bold = true,
        "textmd" | "mdseries" => next.bold = false,
        "textit" | "textsl" | "itshape" | "slshape" => next.italic = true,
        // Small capitals (latex.ltx `\textsc`/`\scshape`): the Core 14
        // layout has no small-caps faces and keeps the current style; the
        // render pipeline selects the NFSS `sc` shape from the source.
        "textsc" | "scshape" => {}
        "textup" | "upshape" => next.italic = false,
        "emph" | "em" => next.italic = !style.italic,
        "texttt" | "ttfamily" => next.family = TextFamily::Mono,
        "textrm" | "rmfamily" => next.family = TextFamily::Roman,
        "textsf" | "sffamily" => next.family = TextFamily::Sans,
        "textnormal" | "normalfont" => next = TextStyle::default(),
        // LaTeX 2.09 forms reset the other attributes: `\bf` is
        // `\normalfont\bfseries`.
        "bf" => next = TextStyle::BOLD,
        "it" | "sl" => {
            next = TextStyle {
                italic: true,
                ..TextStyle::default()
            }
        }
        // `\sc` is `\normalfont\scshape`: upright roman here.
        "sc" => next = TextStyle::default(),
        "tt" | "rm" | "sf" => next = apply_style(TextStyle::default(), &format!("{name}family")),
        "tiny" => next.size = Some(FontSizeLevel::Tiny),
        "scriptsize" => next.size = Some(FontSizeLevel::ScriptSize),
        "footnotesize" => next.size = Some(FontSizeLevel::FootnoteSize),
        "small" => next.size = Some(FontSizeLevel::Small),
        "normalsize" => next.size = None,
        "large" => next.size = Some(FontSizeLevel::Large1),
        "Large" => next.size = Some(FontSizeLevel::Large2),
        "LARGE" => next.size = Some(FontSizeLevel::Large3),
        "huge" => next.size = Some(FontSizeLevel::Huge1),
        "Huge" => next.size = Some(FontSizeLevel::Huge2),
        _ => {}
    }
    // Font commands (`\normalfont`, `\bf`) never change the colour.
    next.color = style.color;
    next
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParagraphStyle {
    Center,
    FlushRight,
    FlushLeft,
    /// `quote`/`quotation`: both margins indented.
    Quote,
}

/// The size declaration in force when a paragraph's `\par` ran — the
/// `\baselineskip` every one of its lines is set under.
///
/// TeX reads `\baselineskip` in `append_to_vlist` (§679), which
/// `post_line_break` (§877) calls once per line *at `\par` time*. One value
/// therefore governs the whole paragraph, and it is the register's value when
/// the paragraph **ended**, not the one where the words were typed. Hence
///
/// - `{\small ... }` followed by a blank line keeps the body's leading: the
///   `}` restores `\baselineskip` before the blank line's `\par`;
/// - `{\small ... \par}` takes `\small`'s 12 pt (11 pt class), because the
///   `\par` is inside the group;
/// - `\begin{quote}\small ...\end{quote}` and `\begin{itemize}\small ...`
///   likewise, because `\endtrivlist` runs `\ifhmode\unskip\par\fi` *before*
///   `\end` closes the group;
/// - a mid-paragraph switch (`words {\small more} words`) never changes the
///   leading at all.
///
/// `None` is `\normalsize`'s. The class's own table
/// (`flashtex_document_style::font_size`, from `size1x.clo`) turns the level
/// into points; this crate deliberately carries the level, not the length, so
/// the 10/11/12 pt tables stay in one place.
pub type ParLeading = Option<FontSizeLevel>;

/// A macro definition actually consulted while producing one block.
#[derive(Debug, Clone, PartialEq)]
pub struct MacroDependency {
    pub name: String,
    pub argument_count: usize,
    pub replacement: Vec<TokenKind>,
}

#[derive(Debug)]
pub struct Parsed {
    pub blocks: Vec<Block>,
    pub diagnostics: Vec<Diagnostic>,
    /// The argument of the first valid `\documentclass`, if present.
    pub document_class: Option<String>,
    /// Body size from a `10pt`/`11pt`/`12pt` `\documentclass` option.
    pub class_size_pt: Option<f64>,
    /// `\setlength{\parskip}{..}` from the preamble, in points.
    pub parskip_pt: Option<f64>,
    /// Package names mentioned by valid `\usepackage` commands.
    pub packages: Vec<String>,
    /// One dependency list per block, in `blocks` order.
    pub block_dependencies: Vec<Vec<MacroDependency>>,
    /// One [`ParLeading`] per block, in `blocks` order: the leading the
    /// block's `\par` selected. `None` for every block that is not a
    /// paragraph (a heading sets its own leading) and for paragraphs whose
    /// `\par` ran at `\normalsize`.
    pub block_par_leading: Vec<ParLeading>,
    /// Exact preamble bytes. A change invalidates every cached block.
    pub preamble_source: String,
    /// False for recovery/unsupported cases whose state effects are not proven.
    pub incremental_safe: bool,
    /// True when counters or the label table make layout document-global.
    pub document_global_state: bool,
    /// `cleveref` naming options and `\crefname` overrides.
    pub cleveref: crate::xref::CleverefConfig,
    /// `\pagecolor`: the page background, document-wide (`None`: none).
    pub page_color: Option<DeviceColor>,
    /// The default text colour when xcolor converts to a target model
    /// (`0 0 0 rg` under `[rgb]`); `None` is pdfTeX's `0 g`.
    pub default_color: Option<DeviceColor>,
    /// Every run of macro replacement text in the parser's input, in input
    /// order: the invocation span its tokens carry, and the exact bytes of
    /// the definition they were copied from (see `crate::expansion`).
    pub expansions: Vec<ExpansionSite>,
}

impl Parsed {
    /// Layout constraints with the preamble's body size and `\parskip` applied.
    pub fn preamble_constraints(
        &self,
        constraints: crate::layout::LayoutConstraints,
    ) -> crate::layout::LayoutConstraints {
        crate::layout::LayoutConstraints {
            font_size_pt: self.class_size_pt.unwrap_or(constraints.font_size_pt),
            parskip_pt: self.parskip_pt.or(constraints.parskip_pt),
            ..constraints
        }
    }
}

pub(crate) const BUILT_INS: &[&str] = &[
    "num",
    "qty",
    "unit",
    "si",
    "SI",
    "numlist",
    "numrange",
    "qtylist",
    "qtyrange",
    "SIlist",
    "SIrange",
    "ang",
    "sisetup",
    "DeclareSIUnit",
    "section",
    "subsection",
    "subsubsection",
    "paragraph",
    "subparagraph",
    "tableofcontents",
    "index",
    "glossary",
    "textbf",
    "textmd",
    "emph",
    "textit",
    "textsl",
    "textsc",
    "textup",
    "texttt",
    "textrm",
    "textsf",
    "textnormal",
    "begin",
    "end",
    "par",
    "documentclass",
    "setlength",
    "addtolength",
    "usepackage",
    "definecolor",
    "providecolor",
    "xdefinecolor",
    "colorlet",
    "definecolorset",
    "DefineNamedColor",
    "selectcolormodel",
    "color",
    "textcolor",
    "pagecolor",
    "nopagecolor",
    "normalcolor",
    "colorbox",
    "fcolorbox",
    "newcolumntype",
    "arraybackslash",
    "arrayrulecolor",
    "doublerulesepcolor",
    "setlist",
    "newcommand",
    "renewcommand",
    "DeclareMathOperator",
    "input",
    "include",
    "label",
    "ref",
    "pageref",
    "eqref",
    "cref",
    "Cref",
    "crefrange",
    "Crefrange",
    "cpageref",
    "Cpageref",
    "labelcref",
    "crefname",
    "Crefname",
    "numberwithin",
    "counterwithin",
    "counterwithout",
    "caption",
    "item",
    "includegraphics",
    "scalebox",
    "resizebox",
    "rotatebox",
    "reflectbox",
    "graphicspath",
    "hypersetup",
    "lstset",
    "allowdisplaybreaks",
    "url",
    "href",
    "nolinkurl",
    "hfill",
    "hrulefill",
    "dotfill",
    "hfil",
    "hspace",
    "footnote",
    "footnotemark",
    "footnotetext",
    "normalfont",
    "bfseries",
    "mdseries",
    "itshape",
    "slshape",
    "scshape",
    "upshape",
    "ttfamily",
    "rmfamily",
    "sffamily",
    "em",
    "bf",
    "it",
    "sl",
    "sc",
    "tt",
    "rm",
    "sf",
    "quad",
    "qquad",
    "bigskip",
    "medskip",
    "smallskip",
    "vspace",
    "hrule",
    "newpage",
    "clearpage",
    "cleardoublepage",
    "pagebreak",
    "nopagebreak",
    "linebreak",
    "nolinebreak",
    "vfill",
    "columnbreak",
    "newcolumn",
    "raggedcolumns",
    "flushcolumns",
    "pagestyle",
    "thispagestyle",
    "pagenumbering",
    "listfiles",
    "centering",
    "Centering",
    "raggedright",
    "RaggedRight",
    "raggedleft",
    "RaggedLeft",
    "noindent",
    "indent",
    "tiny",
    "scriptsize",
    "footnotesize",
    "small",
    "normalsize",
    "large",
    "Large",
    "LARGE",
    "huge",
    "Huge",
    "cite",
    "citet",
    "citep",
    "citealt",
    "citealp",
    "citeauthor",
    "citefullauthor",
    "citeyear",
    "citeyearpar",
    "citenum",
    "citetext",
    "Citet",
    "Citep",
    "Citealt",
    "Citealp",
    "Citeauthor",
    "nocite",
    "bibitem",
    "bibliography",
    "bibliographystyle",
    "title",
    "author",
    "date",
    "maketitle",
    // letter.cls.
    "address",
    "signature",
    "name",
    "location",
    "telephone",
    "opening",
    "closing",
    "cc",
    "encl",
    "ps",
    "startbreaks",
    "stopbreaks",
    "stopletter",
    "makelabels",
    "thanks",
    "and",
    "today",
    "TeX",
    "LaTeX",
    "LaTeXe",
    "rule",
    "thinspace",
    "negthinspace",
    "medspace",
    "negmedspace",
    "thickspace",
    "negthickspace",
    "enspace",
    "enskip",
    "AA",
    "aa",
    "AE",
    "ae",
    "OE",
    "oe",
    "O",
    "o",
    "L",
    "l",
    "ss",
    "SS",
    "TH",
    "th",
    "DH",
    "dh",
    "DJ",
    "dj",
    "NG",
    "ng",
    "IJ",
    "ij",
    "i",
    "j",
    "S",
    "P",
    "dag",
    "ddag",
    "copyright",
    "pounds",
    "dots",
    "ldots",
    "textsection",
    "textparagraph",
    "textdagger",
    "textdaggerdbl",
    "textcopyright",
    "textsterling",
    "textellipsis",
    "textbackslash",
    "textasciitilde",
    "textasciicircum",
    "textunderscore",
    "textbar",
    "textless",
    "textgreater",
    "textbraceleft",
    "textbraceright",
    // `text_builtins::TEXT_ACCENTS` and the
    // `text_builtins::CAPITAL_ACCENT_ALIASES` alias names.
    "c",
    "v",
    "u",
    "H",
    "r",
    "k",
    "d",
    "b",
    "capitalcaron",
    "capitalbreve",
    "capitalring",
    "capitalogonek",
    "capitalhungarumlaut",
    "capitalcedilla",
    "uline",
    "underline",
    "sout",
];

/// Parses a LaTeX dimension (`12pt`, `1.5em`, `0.5in`, `2cm`, `10mm`, `2ex`,
/// `12bp`) to points. `em`/`ex` are relative to the compiler's fixed body size
/// because the layout does not yet carry a current font size into dimension
/// parsing. Physical units follow TeX `scan_dimen` §458 (`1bp` = 72.27/72 pt,
/// not 1pt). `ex` uses [`CMR_EX_PER_EM`] (cmr x-height/em, the same constant
/// as ulem `\sout`); this crate has no TFM, unlike the pipeline's `ec_em_ex`.
pub(crate) fn parse_dimen_pt(text: &str) -> Option<f64> {
    parse_dimen_pt_at(text, crate::layout::BODY_SIZE_PT)
}

/// Page and paragraph lengths a preamble may assign (`\setlength`,
/// `\addtolength`, or a TeX `\len=<dimen>` / `\len <dimen>` assignment).
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
];

fn is_preamble_length(name: &str) -> bool {
    PREAMBLE_LENGTHS.contains(&name)
}

fn is_length_reference(raw: &str) -> bool {
    let s = raw.trim().trim_start_matches('=').trim();
    s.contains('\\') || is_preamble_length(s.trim_start_matches('\\'))
}

fn length_reference_parts(raw: &str) -> Option<(f64, &str)> {
    let s = raw.trim().trim_start_matches('=').trim();
    if let Some(bs) = s.find('\\') {
        let (factor, rest) = s.split_at(bs);
        let name = rest[1..].trim();
        if !is_preamble_length(name) && !matches!(name, "linewidth" | "columnwidth" | "hsize") {
            return None;
        }
        let f = factor.trim();
        let scale = if f.is_empty() || f == "+" {
            1.0
        } else if f == "-" {
            -1.0
        } else {
            f.parse().ok()?
        };
        return Some((scale, name));
    }
    let stripped = s.trim_start_matches('\\');
    if is_preamble_length(stripped) {
        return Some((1.0, stripped));
    }
    None
}

/// `parse_dimen_pt` with `em`/`ex` relative to `body_pt`.
///
/// Also accepts an optional leading `=`, a leading sign, and a factor times
/// a known length (`\textwidth`, `-.5\textwidth`). A bare length name (with
/// or without the backslash) is a factor of 1. The referenced length is not
/// looked up here: the render pipeline applies real page geometry from the
/// source; a zero is enough for the compiler to accept the assignment.
pub(crate) fn parse_dimen_pt_at(text: &str, body_pt: f64) -> Option<f64> {
    let text = text.trim().trim_start_matches('=').trim();
    if text.is_empty() {
        return None;
    }
    if let Some(bs) = text.find('\\') {
        let (factor, rest) = text.split_at(bs);
        let name = rest[1..].trim();
        if !is_preamble_length(name) && !matches!(name, "linewidth" | "columnwidth" | "hsize") {
            return None;
        }
        let factor = factor.trim();
        if !factor.is_empty() {
            let _: f64 = factor.parse().ok()?;
        }
        return Some(0.0);
    }
    let stripped = text.trim_start_matches('\\');
    if is_preamble_length(stripped) {
        return Some(0.0);
    }
    let unit_len = text
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_alphabetic())
        .count();
    if unit_len == 0 || unit_len > text.len() {
        return None;
    }
    let split = text.len() - unit_len;
    let (number, unit) = text.split_at(split);
    let value: f64 = number.trim().parse().ok()?;
    // TeX: The Program §458. `in`/`cm`/`mm`/`bp` share the 7227 numerator
    // (72.27 pt per inch); `dd`/`cc` are Didot; `sp` is 2^-16 pt.
    let per_pt = match unit {
        "pt" => 1.0,
        "bp" => 72.27 / 72.0,
        "in" => 72.27,
        "cm" => 72.27 / 2.54,
        "mm" => 72.27 / 25.4,
        "dd" => 1238.0 / 1157.0,
        "cc" => 14856.0 / 1157.0,
        "sp" => 1.0 / 65536.0,
        "em" => body_pt,
        "ex" => body_pt * CMR_EX_PER_EM,
        _ => return None,
    };
    Some(value * per_pt)
}

/// True when `content` (already trimmed) is safe for the unsupported-command
/// recovery policy to assume is a parameter rather than prose — see the
/// policy comment on `unsupported` below for the full rationale. A dimension
/// (reusing `parse_dimen_pt`, so `\vspace{0.6em}`-style values match) or a
/// single lowercase keyword (`empty`, `arabic`, ...) both qualify. A
/// single-*character* word is deliberately excluded: real LaTeX keyword
/// parameters are essentially always two or more letters (`empty`, `plain`,
/// `arabic`, ...), while a single lowercase letter is far more likely to be
/// genuine one-letter prose or a macro body (`\def\x{y}`) that must not be
/// silently dropped.
fn looks_like_recoverable_argument(content: &str) -> bool {
    parse_dimen_pt(content).is_some()
        || (content.chars().count() > 1 && content.chars().all(|ch| ch.is_ascii_lowercase()))
}

/// Plain TeX's conventional `\smallskipamount`/`\medskipamount`/
/// `\bigskipamount`, in points. Real TeX also gives each a `plus`/`minus`
/// stretch component; this layout model has no rubber lengths (see
/// `Block::VSpace`, which `\vspace` already feeds a flat point value), so
/// these are the flat amounts with the stretch/shrink honestly dropped.
const SMALL_SKIP_PT: f64 = 3.0;
const MEDIUM_SKIP_PT: f64 = 6.0;
const BIG_SKIP_PT: f64 = 12.0;

/// Project-relative paths only: no absolute paths or parent traversal.
pub(crate) fn path_is_safe(path: &str) -> bool {
    if path.is_empty() || path.starts_with('/') || path.starts_with('\\') {
        return false;
    }
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        return false;
    }
    !path.split(['/', '\\']).any(|component| component == "..")
}

/// One parser input token (see `crate::expansion::ExpandedToken`).
type InputToken = expansion::ExpandedToken;

/// `letter.cls` line 91's `\setlength\parskip{0.7em}`, evaluated in the class
/// body font, in points. The three values are what pdflatex prints for
/// `\the\parskip` (TeX Live 2025) rather than `0.7 * size`, because TeX
/// scales `0.7em` in fixed point against `\fontdimen6` — 10pt gives
/// 6.99997pt, not 7pt. An unrecognised size keeps the class's own 10pt
/// default (`\ExecuteOptions{letterpaper,10pt,...}`).
fn letter_parskip_pt(class_size_pt: Option<f64>) -> f64 {
    match class_size_pt {
        Some(size) if size == 11.0 => 7.66498,
        Some(size) if size == 12.0 => 8.22487,
        _ => 6.99997,
    }
}

/// `letter.cls` line 236: `\medskipamount=\parskip`, which `\closing` uses
/// six of between the closing line and the signature.
fn letter_signature_gap_pt(class_size_pt: Option<f64>) -> f64 {
    6.0 * letter_parskip_pt(class_size_pt)
}

/// `\longindentation` (letter.cls 219: `.5\textwidth`), in points.
///
/// It is a *class* length, assigned once when letter.cls loads, from the
/// class's own `\textwidth` — `size1x.clo`'s 345/360/390pt — and nothing
/// updates it afterwards. Under `\usepackage[margin=1in]{geometry}` at 11pt
/// the measure is 469.75502pt but `\longindentation` is still 180pt
/// (pdflatex, TeX Live 2025), which is why this is keyed on the class size
/// rather than taken from the layout's measure. The committed
/// `fixtures/real-world/letter/reference.pdf` confirms it: "Sincerely,"
/// starts at 251.328bp, and 251.328bp − 72bp is exactly 180pt.
pub(crate) fn letter_longindentation_pt(class_size_pt: Option<f64>) -> f64 {
    match class_size_pt {
        Some(size) if size == 11.0 => 180.0,
        Some(size) if size == 12.0 => 195.0,
        _ => 172.5,
    }
}

/// Splits inline content at its `Inline::LineBreak`s (the source's `\\`),
/// dropping the breaks. An empty run between two breaks is kept, because
/// `\address{A\\\\B}` really does leave a blank line in the box.
fn split_at_line_breaks(content: Vec<Inline>) -> Vec<Vec<Inline>> {
    let mut lines = vec![Vec::new()];
    for inline in content {
        match inline {
            Inline::LineBreak { .. } => lines.push(Vec::new()),
            other => lines.last_mut().expect("one line").push(other),
        }
    }
    lines
}

/// Whether the token at `index` in `tokens` sits directly against real
/// source whitespace — a preceding `TokenKind::Space`/`ParBreak` — or is the
/// first token, in which case there is nothing before it to glue against.
/// Any other neighbour (a word, a control word, `$`, a brace, ...) means the
/// source had no space there, so layout must not invent one. Shared by the
/// main token cursor (`P::space_precedes`) and `P::inlines_from_tokens`,
/// which walks its own flattened, macro-expanded token list.
fn preceded_by_space(tokens: &[InputToken], index: usize) -> bool {
    index == 0
        || matches!(
            tokens.get(index - 1).map(|input| &input.token.kind),
            Some(TokenKind::Space) | Some(TokenKind::ParBreak)
        )
}

/// Splits a captured `\author{...}` argument on top-level `\and`: real
/// `article.cls` gives each `\and`-separated name its own
/// `tabular[t]{c}` column. An `\and` nested inside a brace group does not
/// split, matching how this parser only ever splits at brace depth zero
/// (e.g. `&`/`\\` in `multirow_environment`).
fn split_on_and(tokens: Vec<InputToken>) -> Vec<Vec<InputToken>> {
    let mut groups = vec![Vec::new()];
    let mut depth = 0usize;
    for input in tokens {
        match &input.token.kind {
            TokenKind::LBrace => {
                depth += 1;
                groups.last_mut().expect("at least one group").push(input);
            }
            TokenKind::RBrace => {
                depth = depth.saturating_sub(1);
                groups.last_mut().expect("at least one group").push(input);
            }
            TokenKind::Command(name) if depth == 0 && name == "and" => {
                groups.push(Vec::new());
            }
            _ => groups.last_mut().expect("at least one group").push(input),
        }
    }
    groups
}

pub fn parse(text: &str) -> Parsed {
    parse_project(&[SourceDocument { path: "", text }], "")
}

/// Parse an entry document and every project document it includes, with the
/// epoch date. Kept for callers that have no date to supply; see
/// [`parse_project_with`].
pub fn parse_project(documents: &[SourceDocument<'_>], entry_path: &str) -> Parsed {
    parse_project_with(documents, entry_path, &ParseOptions::default())
}

/// Parse an entry document and every project document it includes.
pub fn parse_project_with(
    documents: &[SourceDocument<'_>],
    entry_path: &str,
    options: &ParseOptions,
) -> Parsed {
    let entry = documents
        .iter()
        .position(|document| document.path == entry_path)
        .unwrap_or(0);
    let entry_document = documents.get(entry).copied().unwrap_or(SourceDocument {
        path: entry_path,
        text: "",
    });
    let mut expanded = if documents.is_empty() {
        expansion::Expansion {
            tokens: Rc::new(Vec::new()),
            diagnostics: Vec::new(),
            arraystretch: HashMap::new(),
        }
    } else {
        expansion::expand_project_cached(documents, entry)
    };
    // Structure queries read the expanded stream the parser walks: one
    // tokenizer pass per revision instead of three.
    let has_document = has_document_environment(&expanded.tokens);
    let mut bibliography_diags = Vec::new();
    let bibliography = bib::prescan(&expanded.tokens[..], &mut bibliography_diags);
    let mut expansions: Vec<ExpansionSite> = Vec::new();
    for token in expanded.tokens.iter() {
        if let (true, Some(definition)) = (token.maps_to_invocation, token.definition) {
            match expansions.last_mut() {
                Some(last)
                    if last.invocation == token.token.span
                        && last.definition.document == definition.document
                        && last.definition.end <= definition.start
                        && gap_is_blank(documents, last.definition, definition) =>
                {
                    last.definition = last.definition.merge(definition);
                }
                _ => expansions.push(ExpansionSite {
                    invocation: token.token.span,
                    definition,
                }),
            }
        }
    }
    siunitx::reset();
    let mut p = P {
        t: std::mem::replace(&mut expanded.tokens, Rc::new(Vec::new())),
        entry_path: entry_document.path,
        lent_from_cache: false,
        undo: Vec::new(),
        i: 0,
        diags: Vec::new(),
        brace_stack: Vec::new(),
        env_stack: Vec::new(),
        arraystretch: expanded.arraystretch,
        has_document,
        in_body: !has_document,
        document_ended: false,
        document_class: None,
        class_size_pt: None,
        parskip_pt: None,
        packages: Vec::new(),
        math_packages: MathPackages::KERNEL,
        font_encoding: Encoding::OT1,
        block_dependencies: Vec::new(),
        block_par_leading: Vec::new(),
        next_block_par_leading: None,
        current_dependencies: BTreeMap::new(),
        documents,
        document_by_path: documents
            .iter()
            .enumerate()
            .map(|(index, document)| (document.path, index))
            .collect(),
        include_stack: vec![entry],
        counters: crate::xref::Counters::article(),
        subequations: Vec::new(),
        table_rule_color: None,
        table_double_rule_sep_color: None,
        footnote_counter: 0,
        mpfootnote_counter: 0,
        chapter_class: false,
        current_counter: None,
        current_counter_kind: None,
        seen_labels: HashMap::new(),
        list_stack: Vec::new(),
        list_frames: Vec::new(),
        setlists: Vec::new(),
        resume_counters: HashMap::new(),
        resume_keys: HashMap::new(),
        pending_item_label: None,
        pending_item: None,
        pending_line_break: None,
        paragraph_styles: Vec::new(),
        document_global_state: false,
        cleveref: crate::xref::CleverefConfig::default(),
        style: TextStyle::default(),
        style_stack: Vec::new(),
        env_styles: Vec::new(),
        declared_alignment: None,
        alignment_stack: Vec::new(),
        env_alignments: Vec::new(),
        list_spacing: HashMap::new(),
        theorems: HashMap::new(),
        theorem_style: TheoremStyle::default(),
        theorem_counters: HashMap::new(),
        noted_unclickable_link: false,
        noted_hypersetup_keys: false,
        noted_lstset_keys: false,
        bibliography,
        bib_cursor: 0,
        natbib_limitations: std::collections::BTreeSet::new(),
        natbib_forced_numbers_reported: false,
        title: None,
        author: None,
        date: None,
        today: options.today,
        titlepage_option: false,
        twocolumn_option: false,
        letter: LetterDeclarations::default(),
        column_types: HashMap::new(),
        colors: None,
        page_color: None,
        fboxsep_pt: 3.0,
        fboxrule_pt: 0.4,
    };
    p.diags.extend(bibliography_diags);
    p.diags.extend(expanded.diagnostics);
    let blocks = p.document();

    while let Some(open) = p.brace_stack.pop() {
        p.diags.push(Diagnostic::error(
            "unmatched '{' — group never closed",
            Some(open),
            Some("treated the rest of the document as part of the group".into()),
        )
        .with_help("add a closing '}'")
        .with_label(open, "this group opens here", true));
    }
    while let Some((name, span)) = p.env_stack.pop() {
        p.diags.push(Diagnostic::error(
            format!("unterminated environment '{}' — no matching \\end", name),
            Some(span),
            Some("closed the environment at end of input".into()),
        )
        .with_help(format!("add \\end{{{name}}}")));
    }

    let incremental_safe = p.diags.is_empty();
    let tokens = p.restore_tokens();
    let preamble_source = preamble_source(entry_document.text, has_document, &tokens);
    if p.lent_from_cache {
        expansion::return_cached_tokens(entry_document.path, tokens);
    }
    Parsed {
        blocks,
        diagnostics: p.diags,
        document_class: p.document_class,
        class_size_pt: p.class_size_pt,
        parskip_pt: p.parskip_pt,
        packages: p.packages,
        block_dependencies: p.block_dependencies,
        block_par_leading: p.block_par_leading,
        preamble_source,
        incremental_safe,
        document_global_state: p.document_global_state,
        cleveref: p.cleveref,
        page_color: p.page_color,
        default_color: p.colors.as_ref().and_then(|c| c.default_color()),
        expansions,
    }
}

/// Whether only whitespace separates two definition spans (so they belong
/// to one run of replacement text).
fn gap_is_blank(documents: &[SourceDocument<'_>], a: Span, b: Span) -> bool {
    documents
        .get(a.document.0)
        .and_then(|document| document.text.get(a.end..b.start))
        .is_some_and(|gap| gap.chars().all(char::is_whitespace))
}

struct P<'a> {
    /// The expanded stream. Edits go through [`P::token_mut`]: when the
    /// expansion cache holds the stream, it is lent to the parser (no copy)
    /// and every edit is undone before it goes back.
    t: Rc<Vec<InputToken>>,
    entry_path: &'a str,
    /// `t` was lent by the expansion cache and must be restored and returned.
    lent_from_cache: bool,
    /// Original tokens of in-place edits, in edit order.
    undo: Vec<(usize, InputToken)>,
    i: usize,
    diags: Vec<Diagnostic>,
    brace_stack: Vec<Span>,
    env_stack: Vec<(String, Span)>,
    /// `\arraystretch` at each `\begin{tabular}`, from the expansion pass.
    arraystretch: HashMap<(usize, usize), String>,
    has_document: bool,
    in_body: bool,
    document_ended: bool,
    document_class: Option<String>,
    class_size_pt: Option<f64>,
    parskip_pt: Option<f64>,
    packages: Vec<String>,
    /// The loaded packages that redefine math commands (`math::MathPackages`),
    /// folded in as `\documentclass` and `\usepackage` are read. Math parsed
    /// before the class line is parsed with the kernel's definitions, which is
    /// what pdfLaTeX does too: a redefinition applies only after it runs.
    math_packages: MathPackages,
    /// array's `\newcolumntype{X}[n]{spec}` definitions (`parser/tabular.rs`).
    column_types: HashMap<char, (usize, Vec<InputToken>)>,
    /// The loaded colour package (`crate::color`), `None` before one.
    colors: Option<Colors>,
    /// `\pagecolor`.
    page_color: Option<DeviceColor>,
    /// `\fboxsep`/`\fboxrule` in TeX points (latex.ltx: 3pt, 0.4pt).
    fboxsep_pt: f64,
    fboxrule_pt: f64,
    /// The current text font encoding: OT1 unless `fontenc` selected another
    /// (`text_builtins::fontenc_encoding`).
    font_encoding: Encoding,
    block_dependencies: Vec<Vec<MacroDependency>>,
    /// One [`ParLeading`] per pushed block, kept in step with
    /// `block_dependencies` by [`P::finish_block_dependencies`].
    block_par_leading: Vec<ParLeading>,
    /// The [`ParLeading`] of the block about to be pushed, set by
    /// [`P::flush_list_item`] and consumed by the same
    /// `finish_block_dependencies` call that closes the block.
    next_block_par_leading: ParLeading,
    current_dependencies: BTreeMap<String, (usize, Vec<TokenKind>)>,
    documents: &'a [SourceDocument<'a>],
    document_by_path: HashMap<&'a str, usize>,
    include_stack: Vec<usize>,
    /// Sectioning, `equation`, `figure` and `table` counters (see
    /// `crate::xref`).
    counters: crate::xref::Counters,
    /// The `\theequation` in force outside each open `subequations`
    /// environment, restored at its `\end` (amsmath's group).
    subequations: Vec<Vec<crate::xref::Piece>>,
    /// colortbl `\arrayrulecolor`/`\doublerulesepcolor` (global assignments).
    table_rule_color: Option<crate::tabular::ColorSpec>,
    table_double_rule_sep_color: Option<crate::tabular::ColorSpec>,
    /// LaTeX's `footnote` counter; article never resets it, report and
    /// book reset it at every numbered `\chapter` (`\@addtoreset`).
    footnote_counter: u32,
    /// `mpfootnote`: `\footnote` inside a `minipage` (zeroed by every
    /// `\begin{minipage}`, printed `\alph`).
    mpfootnote_counter: u32,
    /// The class is report or book: `\chapter` exists and numbers
    /// sections, figures and equations within it.
    chapter_class: bool,
    current_counter: Option<String>,
    current_counter_kind: Option<String>,
    seen_labels: HashMap<String, Span>,
    /// Environment name, item count, an enumitem label template if given,
    /// the `\setlist` spacing resolved when this list's `\begin` ran, and the
    /// `blocks` length at that point (where this list's own items start, for
    /// the `leftmargin=*` backpatch once every item is known — see
    /// `environment`).
    list_stack: Vec<OpenList>,
    /// Every open `\list`-based environment (lists and `quote`/`quotation`/
    /// `verse`), outermost first; see `Block::ListItem::lists`.
    list_frames: Vec<ListFrame>,
    /// `\setlist[<target>]{<keys>}` calls so far, in order.
    setlists: Vec<(lists::SetlistTarget, Vec<ListOption>)>,
    /// enumitem `resume` state: the last counter value and the `\begin`
    /// keys of each environment name / `series@<name>`.
    resume_counters: HashMap<String, i64>,
    resume_keys: HashMap<String, Vec<ListOption>>,
    /// The marker text and span set by the most recent `\item`, consumed by
    /// the next `flush_paragraph`/`flush_list_item` call (its own paragraph,
    /// or a later one if the item's text is empty). `None` once consumed, so
    /// later paragraphs of the same item render with the hanging indent but
    /// no repeated label.
    pending_item_label: Option<(String, Span)>,
    /// The structured form of `pending_item_label`, taken with it.
    pending_item: Option<ItemLabel>,
    /// verse's `\\` waiting for the next paragraph.
    pending_line_break: Option<LineBreakBefore>,
    paragraph_styles: Vec<ParagraphStyle>,
    document_global_state: bool,
    cleveref: crate::xref::CleverefConfig,
    /// Every `\bibitem`'s resolved citation label, built once by
    /// `bib::prescan` before this parse starts — see that module's doc
    /// comment for why `\cite` does not need a page-aware two-pass pass.
    bibliography: bib::Bibliography,
    /// How many of `bibliography`'s document-order `\bibitem`s this parse has
    /// reached so far; advanced by each real `\bibitem`, whose own displayed
    /// label is looked up at this index.
    bib_cursor: usize,
    /// natbib options that are parsed but not applied, reported once each.
    natbib_limitations: std::collections::BTreeSet<String>,
    /// Whether natbib's `\NAT@force@numbers` fallback has been reported.
    natbib_forced_numbers_reported: bool,
    /// Current text style; saved on `{` and environment entry, restored on
    /// the matching `}` or `\end`.
    style: TextStyle,
    style_stack: Vec<TextStyle>,
    env_styles: Vec<TextStyle>,
    /// `\centering`/`\raggedright`/`\raggedleft` in force. Like TeX's
    /// paragraph parameters it is read when a paragraph ends, and it is
    /// saved on `{`/`\begin` and restored on the matching `}`/`\end`.
    declared_alignment: Option<ParagraphStyle>,
    alignment_stack: Vec<Option<ParagraphStyle>>,
    env_alignments: Vec<Option<ParagraphStyle>>,
    /// `\setlist` overrides, keyed by environment name ("itemize" /
    /// "enumerate"). A list resolves its spacing from here when `\begin`
    /// runs, so a later `\setlist` does not retroactively change an
    /// already-open list.
    list_spacing: HashMap<String, ListSpacing>,
    /// `\newtheorem` registrations, keyed by environment name.
    theorems: HashMap<String, TheoremDef>,
    /// The style set by the most recent `\theoremstyle`, applied to
    /// `\newtheorem` declarations from that point on (`plain` until then,
    /// matching amsthm's own default).
    theorem_style: TheoremStyle,
    /// Theorem counters, keyed by `TheoremDef::counter` (an environment's
    /// own name, or the name of the environment whose counter it shares).
    theorem_counters: HashMap<String, u32>,
    /// Set once `\url`/`\href` has already produced the one honest
    /// "links are not clickable yet" diagnostic (see `note_links_unclickable`),
    /// so a document with many links gets a single notice, not one per use.
    noted_unclickable_link: bool,
    /// Set once `\hypersetup` has reported a key outside the measured
    /// layout-neutral set (see `hypersetup`), so a document that calls it
    /// several times gets a single notice.
    noted_hypersetup_keys: bool,
    /// Set once `\lstset` has reported a name that is not a listings key,
    /// so a document that calls it in a loop reports it once.
    noted_lstset_keys: bool,
    /// Raw (unexpanded) tokens most recently given to `\title`/`\author`,
    /// with the command's own span for diagnostics. `\maketitle` reads
    /// whichever is active at its call site, mirroring how real
    /// `article.cls` reads `\@title`/`\@author`.
    title: Option<(Vec<InputToken>, Span)>,
    author: Option<(Vec<InputToken>, Span)>,
    /// `None` until `\date` is called at all — `\maketitle` then defaults to
    /// `\today`, matching `article.cls`'s own `\date{\today}` preamble
    /// default. `Some(tokens)` after an explicit `\date{...}`; an empty
    /// argument (`\date{}`) suppresses the date line entirely once
    /// `\maketitle` expands it.
    date: Option<(Vec<InputToken>, Span)>,
    /// The date `\today` expands to, supplied by the caller in the compile
    /// request rather than read from the clock here (`ParseOptions::today`).
    today: TodayDate,
    /// Set by `\documentclass[titlepage]{...}`. Real `article.cls` then
    /// gives `\maketitle` an entirely different definition: a dedicated
    /// `titlepage` page, `\vfil`-centred vertically, with wider vskips (60pt,
    /// 3em, 1.5em) and no trailing skip. This compiler has no vertical-fill
    /// primitive, so `P::maketitle` renders the ordinary compact block and
    /// says so once, rather than silently ignoring the option.
    titlepage_option: bool,
    /// The `twocolumn` class option: multicol.sty's `twocolumn` option
    /// handler (lines 111-113) warns when the package is loaded with it.
    twocolumn_option: bool,
    /// `letter.cls`'s preamble declarations, each `\def`ined to empty by the
    /// class itself (lines 154-163) and read by `\opening`/`\closing`:
    /// `\address` (`\fromaddress`), `\signature` (`\fromsig`), `\name`
    /// (`\fromname`), `\location` (`\fromlocation`) and `\telephone`
    /// (`\telephonenum`). The last two feed only the `firstpage` page style's
    /// footer, which this compiler does not render; they are still captured
    /// so that writing them is not reported as an unknown command.
    letter: LetterDeclarations,
}

/// `letter.cls`'s document-level declarations and the current
/// `\begin{letter}{...}` recipient. Only meaningful under
/// `\documentclass{letter}`; every field is empty until the document sets it,
/// exactly as the class's own `\name{}`/`\signature{}`/`\address{}`/
/// `\location{}`/`\telephone{}` calls leave them.
#[derive(Debug, Clone, Default)]
struct LetterDeclarations {
    /// `\address{...}` -> `\fromaddress`.
    address: Option<(Vec<InputToken>, Span)>,
    /// `\signature{...}` -> `\fromsig`.
    signature: Option<(Vec<InputToken>, Span)>,
    /// `\name{...}` -> `\fromname`, the fallback when `\fromsig` is empty.
    name: Option<(Vec<InputToken>, Span)>,
    /// `\location{...}` and `\telephone{...}`: captured, never typeset (see
    /// `Parser::letter`).
    location: Option<(Vec<InputToken>, Span)>,
    telephone: Option<(Vec<InputToken>, Span)>,
    /// `\begin{letter}{<to name>\\<to address>}`, split at the first `\\`
    /// exactly as `\@processto` does (`\toname`, `\toaddress`).
    recipient: Option<(Vec<InputToken>, Span)>,
    /// Whether `\opening` has run in the current `letter` environment, so
    /// `\closing` outside one can say so.
    opened: bool,
}

/// One open `itemize`/`enumerate`/`description`/`thebibliography`.
#[derive(Debug, Clone)]
struct OpenList {
    kind: String,
    /// `\item`s seen so far.
    count: u32,
    /// An enumitem label as `enumitem_label` reads it (`label=<t>` or a
    /// shortlabels template); `thebibliography`'s widest-label argument.
    template: Option<String>,
    spacing: ListSpacing,
    /// `blocks.len()` at `\begin`.
    start: usize,
    /// The enumerate counter (`\c@enum<i>`), after `start=`/`resume`.
    counter: i64,
    /// `label*=<t>`: appended to the enclosing enumerate's current label.
    label_star: Option<String>,
    /// The label text of the latest counted `\item` (for `label*` below).
    current_label: String,
    /// The latest enumerate counter value, without its display punctuation.
    current_reference: String,
    /// `series=<name>`: the counter is also saved under `series@<name>`.
    series: Option<String>,
    /// The `\begin` keys (saved for `resume*`).
    begin_options: Vec<ListOption>,
}

/// Extra vertical space `\setlist{itemsep=...,topsep=...}` adds on top of
/// the compiler's ordinary paragraph gap. Both default to zero, matching
/// today's spacing exactly when `\setlist` is never called.
#[derive(Debug, Clone, Copy, Default)]
struct ListSpacing {
    itemsep_pt: f64,
    topsep_pt: f64,
    leftmargin: LeftMarginSetting,
}

/// `\setlist{leftmargin=...}`'s value, resolved into a `Block::ListItem`'s
/// `ListLeftMargin` once the list's items are known (`Widest` needs every
/// label; `Explicit` is applied to each item as it is created).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum LeftMarginSetting {
    #[default]
    Unset,
    /// `leftmargin=<dimen>`, already resolved to points.
    Explicit(f64),
    /// `leftmargin=*`.
    Widest,
}

impl P<'_> {
    fn peek(&self) -> Option<&Token> {
        self.t.get(self.i).map(|t| &t.token)
    }

    /// See the free function `preceded_by_space`, applied to the main token
    /// cursor.
    fn space_precedes(&self, index: usize) -> bool {
        preceded_by_space(&self.t, index)
    }

    fn document(&mut self) -> Vec<Block> {
        let mut blocks = Vec::new();
        let mut para = Vec::new();

        self.parse_stream(&mut blocks, &mut para);
        self.flush_paragraph(&mut blocks, &mut para);
        blocks
    }

    fn parse_stream(&mut self, blocks: &mut Vec<Block>, para: &mut Vec<Inline>) {
        while self.i < self.t.len() {
            let input = self.t[self.i].clone();
            let tok = input.token;
            let render = self.in_body && !self.document_ended;
            match tok.kind {
                TokenKind::ParBreak => {
                    self.i += 1;
                    if render {
                        self.flush_paragraph(blocks, para);
                    }
                }
                TokenKind::Space | TokenKind::Comment => self.i += 1,
                TokenKind::Word(word) if control_symbol_kern(&word, tok.span, self.math_packages.amsmath).is_some() => {
                    self.i += 1;
                    if render {
                        if let Some(amount) = control_symbol_kern(&word, tok.span, self.math_packages.amsmath) {
                            para.push(Inline::Kern {
                                amount,
                                span: tok.span,
                                style: self.style,
                            });
                        }
                    }
                }
                TokenKind::Word(word) => {
                    let space_before = self.space_precedes(self.i);
                    self.i += 1;
                    if render {
                        para.push(Inline::Text {
                            text: apply_text_ligatures(&word),
                            span: tok.span,
                            style: self.style,
                            space_before,
                        });
                    }
                }
                TokenKind::LineBreak => {
                    self.i += 1;
                    // article.cls 390 `verse`: `\let\\\@centercr`, which ends
                    // the paragraph (latex.ltx `\@centercr`: `\par`, then
                    // `\@xcentercr` `\addvspace{-\parskip}` and `\@icentercr`
                    // `\vskip #1`); `\\*` adds `\nobreak`.
                    if render
                        && self.in_body
                        && self
                            .list_frames
                            .last()
                            .is_some_and(|frame| frame.environment == ListEnvironment::Verse)
                    {
                        let star = self.take_optional_star();
                        let end_before = self.i;
                        let skip_pt = self.skip_line_break_length();
                        let end = self
                            .t
                            .get(end_before.max(1) - 1)
                            .map_or(tok.span.end, |t| t.token.span.end.max(tok.span.end));
                        self.flush_paragraph(blocks, para);
                        self.pending_line_break = Some(LineBreakBefore {
                            span: Span::in_document(tok.span.document, tok.span.start, end),
                            skip_pt,
                            star,
                        });
                        continue;
                    }
                    // `\\[<length>]`: the length is reported on the node rather
                    // than dropped, so a consumer sees it even when the `\\\\`
                    // came from a macro body and the bytes after `span` are
                    // the invocation's arguments. Consuming it here (so it is
                    // never typeset as text) is unchanged.
                    let skip_pt = self.skip_line_break_length();
                    if render {
                        para.push(Inline::LineBreak {
                            span: tok.span,
                            skip_pt,
                        });
                    }
                }
                TokenKind::LBrace => {
                    self.i += 1;
                    self.open_group(tok.span);
                }
                TokenKind::RBrace => {
                    self.i += 1;
                    if self.brace_stack.pop().is_none() {
                        if render {
                            self.diags.push(Diagnostic::error(
                                "unmatched '}' — no group is open here",
                                Some(tok.span),
                                Some("ignored the stray brace and continued".into()),
                            )
                            .with_help("remove this '}' or add a matching '{'"));
                        }
                    } else {
                        if let Some(style) = self.style_stack.pop() {
                            self.style = style;
                        }
                        if let Some(alignment) = self.alignment_stack.pop() {
                            self.declared_alignment = alignment;
                        }
                    }
                }
                TokenKind::MathShift if render => self.dollar_math(tok.span, para),
                TokenKind::DisplayMathOpen if render => self.bracket_math(tok.span, para),
                TokenKind::InlineMathOpen if render => self.paren_math(tok.span, para),
                TokenKind::DisplayMathClose if render => {
                    self.i += 1;
                    self.diags.push(Diagnostic::error(
                        "stray \\] has no matching \\[",
                        Some(tok.span),
                        Some("ignored the stray display-math delimiter".into()),
                    ));
                }
                TokenKind::InlineMathClose if render => {
                    self.i += 1;
                    self.diags.push(Diagnostic::error(
                        "stray \\) has no matching \\(",
                        Some(tok.span),
                        Some("ignored the stray inline-math delimiter".into()),
                    ));
                }
                TokenKind::Superscript | TokenKind::Subscript if render => {
                    self.i += 1;
                    self.diags.push(Diagnostic::error(
                        "math script marker used outside math mode",
                        Some(tok.span),
                        Some("ignored the script marker and continued".into()),
                    )
                    .with_help("wrap the marked atom in math mode: \\(x^{...}\\)"));
                }
                TokenKind::MathShift
                | TokenKind::DisplayMathOpen
                | TokenKind::DisplayMathClose
                | TokenKind::InlineMathOpen
                | TokenKind::InlineMathClose
                | TokenKind::Superscript
                | TokenKind::Subscript => self.i += 1,
                TokenKind::Command(name) => {
                    self.i += 1;
                    self.command(&name, tok.span, blocks, para);
                }
                TokenKind::Verb {
                    text,
                    starred,
                    terminated,
                    listing,
                } => {
                    let space_before = self.space_precedes(self.i);
                    self.i += 1;
                    if !terminated && render {
                        self.diags.push(Diagnostic::error(
                            if listing {
                                "\\lstinline has no closing delimiter on this line"
                            } else {
                                "\\verb has no closing delimiter on this line"
                            },
                            Some(tok.span),
                            Some("used the text through end of line and continued".into()),
                        ));
                    }
                    if render {
                        para.push(Inline::Verbatim {
                            text: verbatim_display(&text, starred),
                            span: tok.span,
                            space_before,
                        });
                    }
                }
            }
        }
    }

    fn command(&mut self, name: &str, span: Span, blocks: &mut Vec<Block>, para: &mut Vec<Inline>) {
        if self.document_ended {
            return;
        }

        match name {
            "documentclass" => self.document_class(span),
            "setlength" => self.set_length(span),
            "addtolength" => self.add_to_length(span),
            "usepackage" => self.use_package(span),
            "definecolor" | "providecolor" | "xdefinecolor" | "colorlet" | "definecolorset"
            | "DefineNamedColor" => self.define_color(name, span),
            "selectcolormodel" => self.select_color_model(span),
            "color" => self.color_declaration(span),
            "textcolor" => self.text_color(span, para),
            "pagecolor" | "nopagecolor" => self.page_color_command(name, span),
            "normalcolor" => self.style.color = None,
            "colorbox" | "fcolorbox" => self.color_box(name, span, para),
            "newcolumntype" => self.new_column_type(span),
            // array.sty 247: `\let\\\tabularnewline`; this parser already
            // ends table rows at `\\` inside `p`-column entries.
            "arraybackslash" => {}
            // colortbl.sty 156-165: global colour of later rules and
            // `\doublerulesep` gaps (inside a table the row scanner takes them).
            "arrayrulecolor" | "doublerulesepcolor" => {
                let color = self.table_color_argument(name, span);
                if !self.colortbl() {
                    self.diags.push(Diagnostic::error(
                        format!("\\{name} needs the colortbl package (or xcolor with the table option)"),
                        Some(span),
                        Some("ignored the colour".into()),
                    ));
                } else if name == "arrayrulecolor" {
                    self.table_rule_color = Some(color);
                } else {
                    self.table_double_rule_sep_color = Some(color);
                }
            }
            "setlist" => self.set_list(span),
            // Definitions run in the expansion pass (`crate::expansion`); the
            // parser only sees their expansions, never these names.
            "newcommand" | "renewcommand" | "DeclareMathOperator" => {}
            "newtheorem" => self.new_theorem(span),
            "theoremstyle" => self.set_theorem_style(span),
            "begin" | "end" => self.environment(name, span, blocks, para),
            "input" | "include" => self.include(name, span, blocks, para),
            // MacTeX writes package-version banners to the log for `\listfiles`;
            // this compiler has no log stream to write them to, so the honest
            // behaviour is a documented no-op rather than an "unsupported"
            // diagnostic for a command every corpus fixture's preamble carries.
            "listfiles" => {}
            // Alignment declarations (ragged2e's capitalised forms differ only
            // in hyphenation, which this compiler does not do). Handled before
            // the preamble guard because a global `\raggedright` there is
            // ordinary LaTeX.
            "centering" | "Centering" => self.declared_alignment = Some(ParagraphStyle::Center),
            "raggedright" | "RaggedRight" => {
                self.declared_alignment = Some(ParagraphStyle::FlushLeft)
            }
            "raggedleft" | "RaggedLeft" => {
                self.declared_alignment = Some(ParagraphStyle::FlushRight)
            }
            // `\title`/`\author`/`\date` are ordinarily preamble commands but
            // real LaTeX also accepts them in the body before `\maketitle`;
            // this arm runs in either place, unlike the preamble catch-all
            // just below.
            "title" => {
                let (tokens, argument_span) = self.required_group(name, span);
                self.title = Some((tokens, span.merge(argument_span)));
            }
            "author" => {
                let (tokens, argument_span) = self.required_group(name, span);
                self.author = Some((tokens, span.merge(argument_span)));
            }
            "date" => {
                let (tokens, argument_span) = self.required_group(name, span);
                self.date = Some((tokens, span.merge(argument_span)));
            }
            "maketitle" => self.maketitle(span, blocks, para),
            // letter.cls's preamble declarations (lines 154-163). Each is
            // `\def`ined to empty by the class, so writing one simply
            // records its replacement text; nothing is typeset here. They
            // exist only under `\documentclass{letter}` — see
            // `P::letter_declaration`, which diagnoses them in any other
            // class exactly as pdflatex's "Undefined control sequence" does.
            "address" | "signature" | "name" | "location" | "telephone" => {
                self.letter_declaration(name, span)
            }
            "opening" => self.letter_opening(span, blocks, para),
            "closing" => self.letter_closing(span, blocks, para),
            "cc" | "encl" => self.letter_annotation(name, span, blocks, para),
            // `\ps` takes NO argument: letter.cls line 245 is
            // `\newcommand*\ps{\par\startbreaks}`. A document writing
            // `\ps{P.S. ...}` — as the corpus fixture does — gets the
            // paragraph break and then typesets the brace group as ordinary
            // text, which is exactly what pdflatex produces (the committed
            // `fixtures/real-world/letter/reference.pdf` sets "P.S. My
            // application number is 2027-0412." as its own paragraph at the
            // left margin). Consuming the argument here would silently
            // delete the author's sentence.
            "ps" | "startbreaks" | "stopbreaks" | "stopletter" => {
                if self.letter_command_available(name, span) && name == "ps" {
                    self.flush_paragraph(blocks, para);
                    self.finish_block_dependencies();
                }
            }
            // `\makelabels` (letter.cls 165-173) writes an address-label
            // page from the `.aux` at the end of the document. There is no
            // `.aux` round trip here, so it is a documented no-op rather
            // than an unknown command.
            "makelabels" => {
                let _ = self.letter_command_available(name, span);
            }
            // `\index{entry}` (makeidx) and `\glossary{entry}` write an
            // `.idx`/`.glo` file for an external program to process. There
            // is no indexing or glossary backend here, and neither command
            // typesets anything in real LaTeX either, so the argument is
            // read and discarded: a silent no-op with no diagnostic, like
            // `\graphicspath` and `\pagestyle` above. The whole entry --
            // `|`-modifiers (`\index{term|textbf}`), `@`-sort keys and
            // `!`-subentries -- lives inside the one braced group, so
            // consuming it consumes the variants too.
            "index" | "glossary" => {
                let _ = self.required_group(name, span);
            }
            // `\today` in ordinary body text. It had no arm here, so it fell
            // through to `unsupported`, whose `debug_assert!(!BUILT_INS
            // .contains(&name))` fires because `today` *is* a built-in: a
            // debug-build panic on any document that simply writes the date in
            // its prose (`tests/supported_latex.rs`'s
            // `canonical_names_outside_the_inventory_are_diagnosed` hit exactly
            // this). Real LaTeX expands `\today` the same way everywhere.
            // `\thanks` and `\and` are meaningful only inside a `\title`/
            // `\author`/`\date` argument, where `strip_thanks` and the `\and`
            // author split consume them. `TEXT_CONTEXT_ONLY` has always claimed
            // that "on their own they are diagnosed" -- but there was no arm,
            // so they reached `unsupported`, whose `debug_assert` on
            // `BUILT_INS` panicked the debug build instead. These arms make the
            // documented behaviour real. Same latent bug as `\today` below,
            // different command.
            "thanks" => {
                // `\thanks{...}` is `\footnotemark\footnotetext` (latex.ltx);
                // this compiler has no footnote implementation, so the note
                // text must not leak into the running prose either.
                let (_, argument_span) = self.required_group(name, span);
                self.diags.push(Diagnostic::command_error(
                    name,
                    "\\thanks outside \\title/\\author/\\date makes a footnote, which this compiler does not implement",
                    Some(span.merge(argument_span)),
                    Some("dropped the command and its note text rather than typesetting the note inline".into()),
                ));
            }
            "and" => {
                // latex.ltx defines `\and` only for the `\author` block's
                // tabular; elsewhere real LaTeX produces spurious column
                // material rather than anything meaningful.
                self.diags.push(Diagnostic::command_error(
                    name,
                    "\\and separates authors inside \\author; outside it there is no author block to split",
                    Some(span),
                    Some("ignored the command".into()),
                ));
            }
            "today" => {
                let space_before = self.space_precedes(self.i - 1);
                para.push(Inline::Text {
                    text: self.today.latex_today(),
                    span,
                    style: self.style,
                    space_before,
                });
            }
            // Preamble or body: amsmath's `\numberwithin` and the kernel's
            // `\counterwithin`/`\counterwithout` (handed to the parser by
            // `expansion::HOST_PRELUDE`).
            "numberwithin" => self.counter_numbering(name, span),
            "counterwithin" | "counterwithout" => self.counter_numbering(name, span),
            // siunitx settings are ordinary preamble material (`crate::siunitx`).
            "sisetup" => {
                let (tokens, argument_span) = self.required_group(name, span);
                let keys = siunitx::raw_text(tokens.iter().map(|t| &t.token));
                siunitx::sisetup(&keys, span.merge(argument_span), &mut self.diags);
            }
            "DeclareSIUnit" => {
                let _ = self.siunitx_bracket();
                let unit = self.command_or_group(name, span);
                let (tokens, _) = self.required_group(name, span);
                siunitx::declare_unit(&unit, &siunitx::raw_text(tokens.iter().map(|t| &t.token)));
            }
            // `\def\graphicspath#1{\def\Ginput@path{#1}}` (graphics.sty): no
            // material. The search list is re-read from the source by the
            // consumer that loads image files (see `crate::graphics`).
            "graphicspath" => {
                let _ = self.required_group(name, span);
            }
            // amsmath's `\allowdisplaybreaks[<0-4>]` only changes where a
            // page may break inside a display: nothing typeset, no material.
            "allowdisplaybreaks" => {
                let _ = self.optional_bracket_argument();
            }
            // Preamble or body (GH#321: the preamble is where documents usually
            // declare them).
            "pagestyle" => {
                // No header/footer rendering exists yet, so every style is
                // accepted with the same (honest) effect: none. `empty` and
                // `plain` both describe "no footer content beyond a page
                // number", which is already what happens.
                let _ = self.required_group(name, span);
            }
            // `\thispagestyle` differs from `\pagestyle` only in scope
            // (current page vs. every later one); since no style ever
            // renders anything either way, the same honest no-op covers it.
            "thispagestyle" => {
                let _ = self.required_group(name, span);
            }
            // `\pagenumbering{arabic|roman}` resets the page counter and its
            // display style. With no footer rendering to show a number in
            // (see `\pagestyle` above) and no separate "displayed page
            // number" distinct from `Page::number` for `\pageref` to read,
            // there is nothing observable left for it to change; accepted
            // with the same honest no-op rather than faking a counter reset
            // whose only visible effect would be through those two missing
            // features.
            "pagenumbering" => {
                let _ = self.required_group(name, span);
            }
            // `\hypersetup{key=value,...}` (hyperref): the same keys the
            // package options take, settable anywhere. Every key this
            // compiler recognises is a PDF annotation, outline or metadata
            // setting that moves no glyph -- see
            // `hyperref_option_is_layout_neutral` for the pdflatex
            // measurement -- so the argument is read and the keys checked,
            // and nothing is typeset. It is accepted in the preamble, where
            // real documents put it, and in the body, where LaTeX also
            // allows it.
            "hypersetup" => self.hypersetup(span),
            // `\lstset{key=value,...}` (listings): the package's own
            // defaults, settable anywhere and global from that point on.
            // The command typesets nothing itself -- `\lst@Init` reads the
            // values when a listing is set -- so the argument is read, the
            // key names are checked, and no material is contributed. It is
            // accepted in the preamble, where every real document puts it,
            // and in the body, where listings also allows it.
            //
            // Before this, `\lstset` was an unknown preamble command *and*
            // its argument was then read as preamble material, so
            // `fixtures/real-world/listings-manual`'s one `\lstset` produced
            // six errors: the command, then `\ttfamily`, `\small`,
            // `\bfseries`, `\itshape` and `\tiny` out of `basicstyle=`,
            // `keywordstyle=`, `commentstyle=` and `numberstyle=`.
            "lstset" => self.lstset(span),
            "crefname" | "Crefname" => self.cleveref_name(name, span),
            _ if self.has_document && !self.in_body && is_preamble_length(name) => {
                self.length_assignment(name, span)
            }
            _ if self.has_document && !self.in_body => self.unsupported_preamble(name, span),
            "num" | "qty" | "unit" | "si" | "SI" | "numlist" | "numrange" | "qtylist"
            | "qtyrange" | "SIlist" | "SIrange" | "ang" => self.siunitx(name, span, para),
            "chapter" if self.chapter_class => self.chapter(span, blocks, para),
            // `\paragraph`/`\subparagraph` are `\@startsection` with a
            // *negative* after-skip (article.cls 406-414), and `\@xsect`'s
            // negative branch never sets the head as a block of its own: it
            // arms `\everypar`, throws away the following paragraph's
            // `\parindent` box and sets the head into that paragraph's first
            // line instead. So the right thing for this layer is to take the
            // star and the optional short title and then get out of the way:
            // the braced title falls through to the main token loop as
            // ordinary body text, which is exactly the material LaTeX runs
            // into that paragraph, in the right place with the right spans.
            //
            // `flush_paragraph` is deliberately NOT called for the same
            // reason — a run-in head does not start a new paragraph.
            //
            // The head's weight, indent, `\hskip 1em` and `\addvspace` come
            // from the render pipeline, which reads the command back from the
            // source at that position (`adapter::run_in_heading_at`). This
            // arm only retires the `\paragraph is not supported by this
            // compiler version` error, which has been stale since the
            // pipeline started laying these heads out correctly.
            "paragraph" | "subparagraph" => {
                let _ = self.take_optional_star();
                let _ = self.optional_bracket_argument();
            }
            "section" | "subsection" | "subsubsection" => {
                let level = match name {
                    "section" => 1,
                    "subsection" => 2,
                    _ => 3,
                };
                let starred = self.take_optional_star();
                let (tokens, _) = self.required_group(name, span);
                self.flush_paragraph(blocks, para);
                let number = if starred {
                    String::new()
                } else {
                    if level == 1 {
                        theorems::reset_within_section(&self.theorems, &mut self.theorem_counters);
                    }
                    self.counters.step(name).unwrap_or_default()
                };
                if !starred {
                    self.set_current_counter(name, Some(number.clone()));
                }
                let content = self.inlines_from_tokens(tokens, TextStyle::BOLD);
                if content.is_empty() {
                    // A missing/empty heading is already diagnosed where
                    // applicable and has nothing to position. Do not create an
                    // empty block: incremental block spans require real source.
                    self.current_dependencies.clear();
                } else {
                    blocks.push(Block::Heading {
                        level,
                        number,
                        number_span: span,
                        content,
                    });
                    self.finish_block_dependencies();
                }
            }
            "label" => {
                let (tokens, argument_span) = self.required_group(name, span);
                let key = token_text(&tokens).trim().to_string();
                self.document_global_state = true;
                if key.is_empty() {
                    self.diags.push(Diagnostic::warning(
                        "\\label was given an empty key",
                        Some(span.merge(argument_span)),
                        Some("ignored the empty label".into()),
                    ));
                } else {
                    if self.seen_labels.insert(key.clone(), span).is_some() {
                        self.diags.push(Diagnostic::warning(
                            format!("duplicate \\label{{{key}}}; the second definition wins"),
                            Some(span.merge(argument_span)),
                            Some("replaced the earlier label definition".into()),
                        ));
                    }
                    para.push(Inline::Label {
                        key,
                        value: self.current_counter.clone().unwrap_or_default(),
                        kind: self.current_counter_kind.clone().unwrap_or_default(),
                        span,
                    });
                }
            }
            "ref" | "pageref" | "eqref" => {
                let space_before = self.space_precedes(self.i - 1);
                let (tokens, argument_span) = self.required_group(name, span);
                let key = token_text(&tokens).trim().to_string();
                self.document_global_state = true;
                para.push(Inline::Reference {
                    key,
                    page: name == "pageref",
                    equation: name == "eqref",
                    span: span.merge(argument_span),
                    space_before,
                });
            }
            "cref" | "Cref" | "crefrange" | "Crefrange" | "cpageref" | "Cpageref"
            | "labelcref" => self.clever_reference(name, span, para),
            "tableofcontents" => {
                self.flush_paragraph(blocks, para);
                self.document_global_state = true;
                blocks.push(Block::TableOfContents { span });
                self.finish_block_dependencies();
            }
            "cite" => {
                // natbib redefines `\cite` (natbib.sty line 693): with an
                // optional argument it is `\citep`, without one `\citet`.
                // That asymmetry is natbib's, not a simplification here.
                let natbib = self.bibliography.natbib().cloned();
                let star = natbib.is_some() && self.take_cite_star();
                let (pre, note) = match &natbib {
                    Some(_) => self.cite_notes(),
                    // The kernel's `\cite` takes one optional argument only.
                    None => (None, self.optional_bracket_argument().map(|(text, _)| text)),
                };
                let (tokens, argument_span) = self.required_group(name, span);
                let full_span = span.merge(argument_span);
                let keys = cite_keys(&tokens);
                self.document_global_state = true;
                if keys.is_empty() {
                    self.diags.push(Diagnostic::warning(
                        "\\cite was given an empty key list",
                        Some(full_span),
                        Some("rendered nothing for the empty citation".into()),
                    ));
                } else if let Some(options) = natbib {
                    let mut kind = if note.is_some() || options.numbers {
                        natbib::CITE_WITH_NOTE
                    } else {
                        natbib::CITE_PLAIN
                    };
                    kind.full = star;
                    self.push_natbib_cite(&options, kind, pre, note, &keys, full_span, para);
                } else {
                    para.extend(bib::cite_inlines(
                        &keys,
                        note,
                        &self.bibliography,
                        full_span,
                        &mut self.diags,
                    ));
                }
            }
            "citet" | "citep" | "citealt" | "citealp" | "citeauthor" | "citefullauthor"
            | "citeyear" | "citeyearpar" | "citenum" | "Citet" | "Citep" | "Citealt"
            | "Citealp" | "Citeauthor" => self.natbib_cite(name, span, para),
            // `\citetext{...}`: natbib's delimiters around arbitrary text
            // (natbib.sty line 741).
            "citetext" => {
                let options = self.natbib_options(name, span);
                let (tokens, argument_span) = self.required_group(name, span);
                let full_span = span.merge(argument_span);
                para.extend(natbib::citetext_inlines(
                    &options,
                    token_text(&tokens).trim(),
                    full_span,
                ));
            }
            // Real LaTeX's `\nocite` only writes a BibTeX aux-file entry (to
            // pull an uncited reference into the printed bibliography); it
            // has no visible output of its own either way, and this compiler
            // has no `.bib`/aux-file pipeline to feed (see `bibliography`
            // below), so consuming the argument is the whole honest behaviour.
            "nocite" => {
                let _ = self.required_group(name, span);
            }
            "bibliography" => {
                let _ = self.required_group(name, span);
                self.diags.push(Diagnostic::warning(
                    "\\bibliography requires BibTeX/biblatex .bib input, which this compiler does not read",
                    Some(span),
                    Some("write the bibliography by hand with thebibliography and \\bibitem".into()),
                ));
            }
            "bibliographystyle" => {
                let _ = self.required_group(name, span);
                self.diags.push(Diagnostic::warning(
                    "\\bibliographystyle has no effect without BibTeX/biblatex .bib support",
                    Some(span),
                    Some("ignored the style and continued".into()),
                ));
            }
            "caption" => {
                let (tokens, _) = self.required_group(name, span);
                if self.env_stack.last().map(|(name, _)| name.as_str()) != Some("figure") {
                    self.diags.push(Diagnostic::error(
                        "\\caption is only supported inside a figure environment",
                        Some(span),
                        Some("typeset the caption text as an ordinary paragraph".into()),
                    ));
                    let style = self.style;
                    para.extend(self.inlines_from_tokens(tokens, style));
                } else {
                    self.flush_paragraph(blocks, para);
                    let number = self.counters.step("figure").unwrap_or_default();
                    self.set_current_counter("figure", Some(number.clone()));
                    let mut content = vec![Inline::Text {
                        text: format!("Figure {number}:"),
                        span,
                        style: TextStyle::default(),
                        space_before: true,
                    }];
                    content.extend(self.inlines_from_tokens(tokens, TextStyle::default()));
                    blocks.push(Block::FigureCaption { content });
                    self.finish_block_dependencies();
                }
            }
            "item" => {
                let gap_before = self
                    .list_stack
                    .last()
                    .map(|list| {
                        if list.count <= 1 {
                            list.spacing.topsep_pt
                        } else {
                            list.spacing.itemsep_pt
                        }
                    })
                    .unwrap_or(0.0);
                self.flush_list_item(blocks, para, gap_before, 0.0);
                match self.list_stack.last() {
                    Some(_) => {
                        let explicit = self.item_label_argument();
                        self.begin_item(span, explicit);
                    }
                    None => self.diags.push(Diagnostic::error(
                        "\\item is only supported inside itemize or enumerate",
                        Some(span),
                        Some("ignored the item marker and continued".into()),
                    )),
                }
            }
            "bibitem" => {
                let in_bibliography =
                    matches!(self.list_stack.last(), Some(list) if list.kind == "thebibliography");
                if !in_bibliography {
                    self.diags.push(Diagnostic::error(
                        "\\bibitem is only supported inside thebibliography",
                        Some(span),
                        Some("ignored the entry and continued".into()),
                    ));
                    let _ = self.optional_bracket_argument();
                    let _ = self.required_group(name, span);
                } else {
                    let gap_before = self
                        .list_stack
                        .last()
                        .map(|list| {
                            if list.count <= 1 {
                                list.spacing.topsep_pt
                            } else {
                                list.spacing.itemsep_pt
                            }
                        })
                        .unwrap_or(0.0);
                    self.flush_list_item(blocks, para, gap_before, 0.0);
                    // The optional `[label]`/required `{key}` were already
                    // read by `bib::prescan`, which resolved this occurrence
                    // (by document order, via `bib_cursor`) to its label
                    // before this parse began; only the token positions need
                    // consuming here.
                    let _ = self.optional_bracket_argument();
                    let _ = self.required_group(name, span);
                    self.document_global_state = true;
                    // The printed marker, already bracketed — and empty under
                    // natbib's author-year mode, whose `\@biblabel` is
                    // `\hfill` (natbib.sty line 622), so the entry starts
                    // flush at the margin with no `[1]` in front of it.
                    let text = self
                        .bibliography
                        .marker_at(self.bib_cursor)
                        .map(str::to_string)
                        .unwrap_or_else(|| {
                            // Should not happen: the pre-scan and this real
                            // parse walk the same literal `\bibitem`s in
                            // lockstep (see `bib::prescan`). Recover with a
                            // plain sequential number rather than losing the
                            // entry.
                            bib::label_bracket(&(self.bib_cursor + 1).to_string())
                        });
                    self.bib_cursor += 1;
                    if let Some(list) = self.list_stack.last_mut() {
                        list.count += 1;
                    }
                    self.pending_item = Some(ItemLabel::Template { text: text.clone() });
                    self.pending_item_label = Some((text, span));
                }
            }
            "includegraphics" => self.include_graphics(span, para),
            "scalebox" | "resizebox" | "rotatebox" | "reflectbox" => {
                self.transform_box(name, span, para)
            }
            // See `url_argument` for why the URL is read from raw source
            // bytes rather than the ordinary token stream, and
            // `note_links_unclickable` for the once-per-document diagnostic.
            "url" => {
                let space_before = self.space_precedes(self.i - 1);
                let (text, arg_span) = self.url_argument(name, span);
                let full_span = span.merge(arg_span);
                self.note_links_unclickable(full_span);
                self.push_url_text(&text, full_span, space_before, para);
            }
            // `\nolinkurl`: url.sty-style literal, monospaced text with no
            // hyperlink at all, so it never needs the "not clickable" notice
            // — nothing here was ever meant to be clickable.
            "nolinkurl" => {
                let space_before = self.space_precedes(self.i - 1);
                let (text, arg_span) = self.url_argument(name, span);
                self.push_url_text(&text, span.merge(arg_span), space_before, para);
            }
            "href" => {
                let (_url, url_span) = self.url_argument(name, span);
                let (text_tokens, text_span) = self.required_group(name, span.merge(url_span));
                self.note_links_unclickable(span.merge(text_span));
                let style = self.style;
                para.extend(self.inlines_from_tokens(text_tokens, style));
            }
            _ if style_command(name) => {
                self.skip_spaces();
                let next = apply_style(self.style, name);
                if let Some(open) = self.closed_group_start() {
                    // Re-enter the argument as an ordinary group so math and
                    // other commands inside it are parsed normally.
                    self.i += 1;
                    self.open_group(open);
                    self.style = next;
                } else {
                    let (tokens, _) = self.required_group(name, span);
                    para.extend(self.inlines_from_tokens(tokens, next));
                }
            }
            _ if style_declaration(name) => self.style = apply_style(self.style, name),
            "hfill" | "hfil" => para.push(Inline::HFill { span, leader: FillLeader::None }),
            "hrulefill" => para.push(Inline::HFill { span, leader: FillLeader::Rule }),
            "dotfill" => para.push(Inline::HFill { span, leader: FillLeader::Dots }),
            "footnote" | "footnotemark" | "footnotetext" => self.footnote(name, span, para),
            // `\linebreak[n]`/`\nolinebreak[n]`: real TeX's 0-4 priority only
            // ever hints a badness-based line-breaking algorithm this greedy
            // layout does not have. An absent bracket or an explicit `4` is
            // TeX's own "you must break here", which is exactly what `\\`
            // already forces (see `Inline::LineBreak`), so that priority
            // alone gets a real break; anything lower is honestly left alone
            // rather than guessing whether a real engine would have broken
            // there. `\nolinebreak` can only ever discourage a break this
            // layout was never going to insert on its own initiative, so
            // honouring it exactly means doing nothing beyond consuming its
            // bracket.
            "linebreak" => {
                if self.mandatory_break_requested() {
                    para.push(Inline::LineBreak { span, skip_pt: None });
                }
            }
            "nolinebreak" => {
                let _ = self.optional_bracket_argument();
            }
            "hspace" => {
                // The star only affects whether the glue survives being
                // discarded at a line break in real TeX, which this layout
                // never does anyway (see the `Inline::HSpace` comment), so
                // both forms are parsed identically.
                let _starred = self.take_optional_star();
                let (tokens, argument_span) = self.required_group(name, span);
                let raw = token_text(&tokens);
                match parse_dimen_pt(&raw) {
                    Some(pt) => para.push(Inline::HSpace {
                        pt,
                        span: span.merge(argument_span),
                    }),
                    None => self.diags.push(Diagnostic::error(
                        format!(
                            "\\hspace requires a recognised dimension, got '{}'",
                            raw.trim()
                        ),
                        Some(span.merge(argument_span)),
                        Some("ignored the malformed \\hspace argument".into()),
                    )),
                }
            }
            // No paragraph is ever given a first-line indent in this layout
            // model, so there is nothing for \noindent to suppress: an honest
            // no-op rather than a fabricated indent to cancel.
            "noindent" => {}
            // The opposite request: unlike \noindent above, this one is not a
            // coincidental match with real LaTeX's output — \indent asks for
            // a first-line indent that this layout has no way to draw (see
            // `set_length`'s `\parindent` handling), so it is named honestly
            // via a diagnostic rather than silently accepted.
            "indent" => self.diags.push(Diagnostic::warning(
                "\\indent is recognised but paragraph indentation is not implemented",
                Some(span),
                Some("the paragraph was not given a first-line indent".into()),
            )),
            // Text-mode horizontal glue. `\quad`/`\qquad` are also implemented
            // in math mode (`src/math.rs`); this arm covers the same commands
            // used directly in running text, 1em/2em of the body text size.
            "quad" => para.push(Inline::TextGlue {
                em: math::QUAD_EM,
                span,
            }),
            "qquad" => para.push(Inline::TextGlue {
                em: 2.0 * math::QUAD_EM,
                span,
            }),
            "par" => self.flush_paragraph(blocks, para),
            "bigskip" | "medskip" | "smallskip" => {
                let pt = match name {
                    "bigskip" => BIG_SKIP_PT,
                    "medskip" => MEDIUM_SKIP_PT,
                    _ => SMALL_SKIP_PT,
                };
                self.flush_paragraph(blocks, para);
                blocks.push(Block::VSpace { pt });
                self.finish_block_dependencies();
            }
            "vspace" => {
                // The star only affects whether the glue survives being
                // discarded at a page break in real TeX, which this layout
                // never does anyway (see `hspace`'s identical star), so both
                // forms are parsed identically. Consuming it here (as
                // `hspace` already does for itself) is the fix: left alone,
                // `required_group` sees `*` where it expects `{` and reports
                // a missing argument instead of reading the dimension after it.
                let _starred = self.take_optional_star();
                let (tokens, argument_span) = self.required_group(name, span);
                let raw = token_text(&tokens);
                let body = self.class_size_pt.unwrap_or(crate::layout::BODY_SIZE_PT);
                match parse_dimen_pt_at(&raw, body) {
                    Some(pt) => {
                        self.flush_paragraph(blocks, para);
                        blocks.push(Block::VSpace { pt });
                        self.finish_block_dependencies();
                    }
                    None => self.diags.push(Diagnostic::error(
                        format!(
                            "\\vspace requires a recognised dimension, got '{}'",
                            raw.trim()
                        ),
                        Some(span.merge(argument_span)),
                        Some("ignored the vertical space and continued".into()),
                    )),
                }
            }
            "hrule" => {
                self.flush_paragraph(blocks, para);
                blocks.push(Block::Rule { span });
                self.finish_block_dependencies();
            }
            "newpage" => {
                self.flush_paragraph(blocks, para);
                blocks.push(Block::PageBreak);
                self.finish_block_dependencies();
            }
            // `\clearpage`/`\cleardoublepage` also flush any queued floats
            // and, for `\cleardoublepage` in a `twoside` class, insert a
            // blank page to land back on an odd one. Neither float queuing
            // nor the oneside/twoside distinction exists in this compiler
            // (article defaults to oneside, where the two commands are
            // already identical in real LaTeX), so both reduce honestly to
            // the same unconditional break as `\newpage`.
            "clearpage" | "cleardoublepage" => {
                self.flush_paragraph(blocks, para);
                blocks.push(Block::PageBreak);
                self.finish_block_dependencies();
            }
            // `\pagebreak[n]`: see the `\linebreak[n]` comment above for why
            // only the mandatory priority (absent or `4`) forces a break.
            "pagebreak" => {
                if self.mandatory_break_requested() {
                    self.flush_paragraph(blocks, para);
                    blocks.push(Block::PageBreak);
                    self.finish_block_dependencies();
                }
            }
            "nopagebreak" => {
                let _ = self.optional_bracket_argument();
            }
            "vfill" => {
                self.flush_paragraph(blocks, para);
                blocks.push(Block::VFill);
                self.finish_block_dependencies();
            }
            // multicol.sty 919-950: `\columnbreak[n]` and `\newcolumn` end a
            // column of `multicols` (set by the render pipeline); outside the
            // environment multicol raises an error.
            "columnbreak" | "newcolumn" => {
                if name == "columnbreak" {
                    let _ = self.optional_bracket_argument();
                }
                if !self.env_stack.iter().any(|(environment, _)| environment == "multicols" || environment == "multicols*") {
                    self.diags.push(Diagnostic::error(
                        format!("Package multicol Error: \\{name} outside multicols; this command can only be used within a multicols or multicols* environment"),
                        Some(span),
                        Some("ignored the command".into()),
                    ));
                }
            }
            // multicol.sty 564-567: column heights at output time.
            "raggedcolumns" | "flushcolumns" => {}
            // Kernel text symbols (`text_builtins::TEXT_SYMBOLS`; the
            // `text_symbol_arms_match_the_builtin_table` test keeps them equal).
            "AA" | "aa" | "AE" | "ae" | "OE" | "oe" | "O" | "o" | "L" | "l" | "ss" | "SS"
            | "TH" | "th" | "DH" | "dh" | "DJ" | "dj" | "NG" | "ng" | "IJ" | "ij" | "i" | "j"
            | "S" | "P" | "dag" | "ddag" | "copyright" | "pounds" | "dots" | "ldots"
            | "textsection" | "textparagraph" | "textdagger" | "textdaggerdbl"
            | "textcopyright" | "textsterling" | "textellipsis" | "textbackslash"
            | "textasciitilde" | "textasciicircum" | "textunderscore" | "textbar" | "textless"
            | "textgreater" | "textbraceleft" | "textbraceright" => {
                self.text_symbol(name, span, para)
            }
            // `text_builtins::TEXT_ACCENTS` and the
            // `text_builtins::CAPITAL_ACCENT_ALIASES` names that resolve to
            // one of them; the alias reaches the same implementation under
            // its canonical name.
            "c" | "v" | "u" | "H" | "r" | "k" | "d" | "b" | "capitalcaron" | "capitalbreve"
            | "capitalring" | "capitalogonek" | "capitalhungarumlaut" | "capitalcedilla" => {
                self.text_accent(text_builtins::canonical_accent_name(name), span, para)
            }
            "TeX" | "LaTeX" | "LaTeXe" => self.text_logo(name, span, para),
            // ulem `\uline`/`\sout` (need the package). Kernel text-mode
            // `\underline` is latex.ltx `$\@@underline{\hbox{#1}}$` (TeXbook
            // Rule 10); math-mode `\underline` is in `math.rs`.
            "uline" | "underline" | "sout" => {
                let geom = match name {
                    "underline" => UnderlineGeom::MathUnderline,
                    "sout" => UnderlineGeom::Strike,
                    _ => UnderlineGeom::UlemDescender,
                };
                self.text_underline_cmd(name, span, para, geom);
            }
            "thinspace" | "negthinspace" | "medspace" | "negmedspace" | "thickspace"
            | "negthickspace" | "enspace" => {
                if let Some(amount) = text_builtins::text_kern(name, self.math_packages.amsmath) {
                    para.push(Inline::Kern {
                        amount,
                        span,
                        style: self.style,
                    });
                }
            }
            // `\def\enskip{\hskip.5em\relax}` (latex.ltx 9434): glue, like `\quad`.
            "enskip" => para.push(Inline::TextGlue { em: 0.5, span }),
            "rule" => self.text_rule(span, para),
            "frac" | "sqrt" => self.diags.push(Diagnostic::error(
                format!("\\{} requires math mode", name),
                Some(span),
                Some("skipped the command and typeset its braced arguments as plain text".into()),
            )
            .with_help(format!("wrap it in math mode: \\(\\{name}{{...}}\\)"))
            .with_label(span, "this command", true)),
            other => self.unsupported(other, span),
        }
    }

    fn include(
        &mut self,
        command: &str,
        span: Span,
        blocks: &mut Vec<Block>,
        para: &mut Vec<Inline>,
    ) {
        let (tokens, _) = self.required_group(command, span);
        let requested = token_text(&tokens).trim().to_string();
        if requested.is_empty() {
            self.diags.push(Diagnostic::error(
                format!("\\{command} requires a non-empty project-relative path"),
                Some(span),
                Some("skipped the empty include and continued".into()),
            ));
            return;
        }
        if !path_is_safe(&requested) {
            self.diags.push(Diagnostic::error(
                format!(
                    "rejected include path '{requested}': paths must be project-relative with no parent traversal"
                ),
                Some(span),
                Some("skipped the unsafe include and continued".into()),
            ));
            return;
        }

        let appended = format!("{requested}.tex");
        let resolved = self
            .document_by_path
            .get(requested.as_str())
            .copied()
            .or_else(|| self.document_by_path.get(appended.as_str()).copied());
        let Some(document_index) = resolved else {
            self.diags.push(Diagnostic::error(
                format!("included file not found: looked for '{requested}' and '{appended}'"),
                Some(span),
                Some("skipped the missing include and continued".into()),
            )
            .with_help(format!(
                "add '{requested}' or '{appended}' to the project documents, or fix the \\input path"
            )));
            return;
        };

        if let Some(cycle_start) = self
            .include_stack
            .iter()
            .position(|active| *active == document_index)
        {
            let mut cycle: Vec<&str> = self.include_stack[cycle_start..]
                .iter()
                .map(|index| self.documents[*index].path)
                .collect();
            cycle.push(self.documents[document_index].path);
            self.diags.push(Diagnostic::error(
                format!("include cycle detected: {}", cycle.join(" -> ")),
                Some(span),
                Some("skipped the cyclic include and continued".into()),
            ));
            return;
        }
        if self.include_stack.len() > INCLUDE_DEPTH_LIMIT {
            self.diags.push(Diagnostic::error(
                format!(
                    "include depth exceeds the limit of {INCLUDE_DEPTH_LIMIT} while loading '{}'",
                    self.documents[document_index].path
                ),
                Some(span),
                Some("skipped the too-deep include and continued".into()),
            ));
            return;
        }

        let saved_tokens = std::mem::replace(
            &mut self.t,
            tokenize_document(
                self.documents[document_index].text,
                DocumentId(document_index),
            )
            .into_iter()
            .map(|token| InputToken {
                token,
                definition: None,
                maps_to_invocation: false,
            })
            .collect::<Vec<_>>().into(),
        );
        let saved_index = std::mem::replace(&mut self.i, 0);
        self.include_stack.push(document_index);
        self.parse_stream(blocks, para);
        self.include_stack.pop();
        self.t = saved_tokens;
        self.i = saved_index;
    }

    fn document_class(&mut self, span: Span) {
        let options = self.optional_bracket_argument();
        let option_list: Vec<&str> = options
            .as_ref()
            .map(|(options, _)| options.split(',').map(str::trim).collect())
            .unwrap_or_default();
        if self.class_size_pt.is_none() {
            self.class_size_pt = option_list.iter().find_map(|option| match *option {
                "10pt" => Some(10.0),
                "11pt" => Some(11.0),
                "12pt" => Some(12.0),
                _ => None,
            });
        }
        // `\maketitle` reads this: the `titlepage` option asks for a
        // dedicated, vertically centred title page (see `P::maketitle`).
        if option_list.contains(&"titlepage") {
            self.titlepage_option = true;
        }
        if option_list.contains(&"twocolumn") {
            self.twocolumn_option = true;
        }
        let (tokens, _) = self.required_group("documentclass", span);
        let class = token_text(&tokens).trim().to_string();
        if class.is_empty() {
            self.diags.push(Diagnostic::warning(
                "\\documentclass was given an empty argument",
                Some(span),
                Some("no document class was recorded".into()),
            ));
        } else if self.document_class.is_none() {
            self.math_packages.load_class(&class);
            if matches!(class.as_str(), "report" | "book") {
                self.chapter_class = true;
                self.counters = crate::xref::Counters::report();
            }
            self.document_class = Some(class);
        }
        // letter.cls lines 91-92 replace the standard classes' paragraph
        // shape outright: `\parskip 0.7em` (rigid, in the class body font)
        // and `\parindent 0pt`. This engine never indents paragraphs, so
        // only the skip has to be carried; a later `\setlength{\parskip}`
        // still wins, exactly as it would in real LaTeX.
        if self.is_letter_class() && self.parskip_pt.is_none() {
            self.parskip_pt = Some(letter_parskip_pt(self.class_size_pt));
        }
    }

    /// Whether `\documentclass{letter}` is in force. `letter.cls` is the only
    /// class that defines `\opening`, `\closing`, `\address`, `\signature`,
    /// `\cc`, `\encl` and the `letter` environment; in an `article` every one
    /// of them is an undefined control sequence, and this compiler must say
    /// so rather than quietly accepting them.
    fn is_letter_class(&self) -> bool {
        self.document_class.as_deref() == Some("letter")
    }

    /// `\setlength{\parskip}{..}` and `\setlength{\parindent}{..}` in the
    /// preamble. `em`/`ex` resolve against the class body size. This engine
    /// never indents paragraphs, so only a zero `\parindent` is exact.
    /// Page-geometry lengths (`\textwidth`, `\oddsidemargin`, ...) are
    /// accepted in the preamble without a diagnostic; the render pipeline
    /// applies them from the source.
    fn set_length(&mut self, span: Span) {
        self.length_command("setlength", span, false);
    }

    fn add_to_length(&mut self, span: Span) {
        self.length_command("addtolength", span, true);
    }

    fn length_command(&mut self, command: &str, span: Span, add: bool) {
        let (target_tokens, _) = self.required_group(command, span);
        let (value_tokens, value_span) = self.required_group(command, span);
        let span = span.merge(value_span);
        let target = token_text(&target_tokens)
            .trim()
            .trim_start_matches('\\')
            .to_string();
        let raw = dimen_source(&value_tokens);
        self.apply_length_value(command, &target, &raw, span, add);
    }

    /// A TeX assignment `\textwidth=6in` / `\textwidth 6in` in the preamble.
    fn length_assignment(&mut self, name: &str, span: Span) {
        let body = self.class_size_pt.unwrap_or(crate::layout::BODY_SIZE_PT);
        let mut raw = String::new();
        let mut end = span;
        loop {
            self.skip_spaces();
            let Some(input) = self.t.get(self.i).cloned() else {
                break;
            };
            let tok = &input.token;
            match &tok.kind {
                TokenKind::Word(word) => {
                    raw.push_str(word);
                    end = end.merge(tok.span);
                    self.i += 1;
                    if parse_dimen_pt_at(&raw, body).is_some() {
                        break;
                    }
                }
                TokenKind::Command(cmd)
                    if is_preamble_length(cmd)
                        || matches!(cmd.as_str(), "linewidth" | "columnwidth" | "hsize") =>
                {
                    raw.push('\\');
                    raw.push_str(cmd);
                    end = end.merge(tok.span);
                    self.i += 1;
                    if parse_dimen_pt_at(&raw, body).is_some() {
                        break;
                    }
                }
                _ => break,
            }
            if raw.len() > 64 {
                break;
            }
        }
        self.apply_length_value("", name, &raw, end, false);
    }

    fn resolve_known_length_ref(&self, raw: &str) -> Option<f64> {
        let (scale, name) = length_reference_parts(raw)?;
        let base = match name {
            "parskip" => self.parskip_pt?,
            "fboxsep" => self.fboxsep_pt,
            "fboxrule" => self.fboxrule_pt,
            _ => return None,
        };
        Some(scale * base)
    }

    fn apply_length_value(&mut self, command: &str, target: &str, raw: &str, span: Span, add: bool) {
        let body = self.class_size_pt.unwrap_or(crate::layout::BODY_SIZE_PT);
        let Some(pt) = parse_dimen_pt_at(raw, body) else {
            let who = if command.is_empty() {
                format!("\\{target}")
            } else {
                format!("\\{command}")
            };
            self.diags.push(Diagnostic::error(
                format!("{who} requires a recognised dimension, got '{}'", raw.trim()),
                Some(span),
                Some("ignored the length assignment".into()),
            ));
            return;
        };
        let in_preamble = self.has_document && !self.in_body;
        let pt = if is_length_reference(raw) {
            match self.resolve_known_length_ref(raw) {
                Some(v) => v,
                None => {
                    // Page geometry (`\textwidth`, `\paperwidth`, ...) is
                    // applied by the pipeline; the compiler must not pretend
                    // the value is 0pt or drop the assignment with no diagnostic.
                    self.diags.push(Diagnostic::warning(
                        "unsupported length expression",
                        Some(span),
                        Some("ignored the length assignment".into()),
                    ));
                    return;
                }
            }
        } else {
            pt
        };
        match target {
            // Read by `\colorbox`/`\fcolorbox` (not group-scoped here).
            "fboxsep" => {
                self.fboxsep_pt = if add { self.fboxsep_pt + pt } else { pt };
            }
            "fboxrule" => {
                self.fboxrule_pt = if add { self.fboxrule_pt + pt } else { pt };
            }
            // longtable's lengths are read from the source by the render
            // pipeline's longtable layout.
            "LTleft" | "LTright" | "LTpre" | "LTpost" | "LTcapwidth" => {}
            "parskip" if in_preamble => {
                self.parskip_pt = Some(if add {
                    self.parskip_pt.unwrap_or(0.0) + pt
                } else {
                    pt
                });
            }
            "parindent" if in_preamble && pt == 0.0 => {}
            // A TeX assignment or `\addtolength` is accepted without noise
            // (the layout still does not indent). `\setlength{\parindent}{nonzero}`
            // keeps the existing "not implemented" warning.
            "parindent" if in_preamble && (add || command.is_empty()) => {}
            "parindent" if in_preamble => self.diags.push(Diagnostic::warning(
                "\\parindent is recognised but paragraph indentation is not implemented",
                Some(span),
                Some("paragraphs are not indented".into()),
            )),
            name if in_preamble && is_preamble_length(name) => {}
            _ => self.diags.push(Diagnostic::warning(
                format!(
                    "\\{command}{{\\{target}}} is recognised but not implemented here"
                ),
                Some(span),
                Some("ignored the length assignment".into()),
            )),
        }
    }

    /// `\setlist[<env list>]{key=value,...}`: enumitem's list-spacing
    /// override. The optional argument names which environments the given
    /// keys apply to (a comma list; omitted means every list). `itemsep`,
    /// `topsep` and `leftmargin` (an explicit dimension, or `*`) change
    /// layout; every other recognised enumitem key (`label`, `parsep`,
    /// `partopsep`, ...) has no equivalent in this layout engine and is
    /// reported once, by name.
    fn set_list(&mut self, span: Span) {
        // `em` is the document's body size here, as in `\setlength`.
        let body = self.class_size_pt.unwrap_or(crate::layout::BODY_SIZE_PT);
        let parse_dimen_pt = |value: &str| parse_dimen_pt_at(value, body);
        let environments = self
            .optional_bracket_argument()
            .map(|(options, _)| options)
            .unwrap_or_default();
        let (tokens, argument_span) = self.required_group("setlist", span);
        let full_span = span.merge(argument_span);
        self.setlists.push((
            lists::SetlistTarget::parse(&environments),
            lists::parse_options(&token_source(&tokens), body, false),
        ));
        let envs: Vec<String> = if environments.trim().is_empty() {
            vec![
                "itemize".to_string(),
                "enumerate".to_string(),
                "description".to_string(),
            ]
        } else {
            environments
                .split(',')
                .map(str::trim)
                .filter(|env| !env.is_empty())
                .map(str::to_string)
                .collect()
        };

        let mut itemsep_pt = None;
        let mut topsep_pt = None;
        let mut leftmargin = None;
        let mut ignored_keys: Vec<String> = Vec::new();
        for pair in token_text(&tokens).split(',') {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }
            let (key, value) = match pair.split_once('=') {
                Some((key, value)) => (key.trim(), Some(value.trim())),
                None => (pair, None),
            };
            match key {
                "itemsep" if value.and_then(parse_dimen_pt).is_some() => {
                    itemsep_pt = value.and_then(parse_dimen_pt);
                }
                "topsep" if value.and_then(parse_dimen_pt).is_some() => {
                    topsep_pt = value.and_then(parse_dimen_pt);
                }
                "leftmargin" if value == Some("*") => {
                    leftmargin = Some(LeftMarginSetting::Widest);
                }
                "leftmargin" if value.and_then(parse_dimen_pt).is_some() => {
                    leftmargin = value
                        .and_then(parse_dimen_pt)
                        .map(LeftMarginSetting::Explicit);
                }
                _ if !ignored_keys.iter().any(|seen| seen == key) => {
                    ignored_keys.push(key.to_string());
                }
                _ => {}
            }
        }

        for env in &envs {
            let spacing = self.list_spacing.entry(env.clone()).or_default();
            if let Some(pt) = itemsep_pt {
                spacing.itemsep_pt = pt;
            }
            if let Some(pt) = topsep_pt {
                spacing.topsep_pt = pt;
            }
            if let Some(lm) = leftmargin {
                spacing.leftmargin = lm;
            }
        }

        if !ignored_keys.is_empty() {
            ignored_keys.sort();
            self.diags.push(Diagnostic::warning(
                format!(
                    "\\setlist keys {} are recognised but not implemented",
                    ignored_keys.join(", ")
                ),
                Some(full_span),
                Some("lists use the compiler's default spacing for these keys".into()),
            ));
        }
    }

    /// natbib's `[pre][post]` optional arguments (`\NAT@citetp`/`\NAT@@citetp`,
    /// natbib.sty lines 688-690). **One bracket is the post-note**: natbib
    /// reads `[#1]` and, only if a second bracket follows, treats the first
    /// as the pre-note; otherwise it calls `\@citex[][#1]`.
    fn cite_notes(&mut self) -> (Option<String>, Option<String>) {
        let Some(first) = self.cite_note_argument() else {
            return (None, None);
        };
        match self.cite_note_argument() {
            Some(second) => (Some(first), Some(second)),
            None => (None, Some(first)),
        }
    }

    /// One `[...]`, leaving whatever is glued to it after the `]` in the
    /// stream. `[`, `]` and `*` are ordinary word characters to the lexer, so
    /// `\citep[see][p.~7]` is a **single** `Word` token: the shared
    /// [`P::optional_bracket_argument`], which consumes whole tokens, would
    /// take "see" and swallow `[p.~7]` with it — the pre-note would silently
    /// become the post-note and the post-note would vanish.
    fn cite_note_argument(&mut self) -> Option<String> {
        self.skip_spaces();
        let word = match &self.t.get(self.i)?.token.kind {
            TokenKind::Word(word) if word.starts_with('[') => word.clone(),
            _ => return None,
        };
        match word.find(']') {
            Some(close) => {
                let note = word[1..close].to_string();
                self.trim_word_prefix(close + 1);
                Some(note)
            }
            // The note runs past this word (`[see this]`): the shared reader
            // already accumulates tokens until the `]`, and nothing can be
            // glued to that `]` inside the same word.
            None => self.optional_bracket_argument().map(|(text, _)| text),
        }
    }

    /// natbib's `\@ifstar` on a citation command, with the same word-splitting
    /// as [`P::cite_note_argument`]: `\citet*[p.~7]` is one lexer word.
    fn take_cite_star(&mut self) -> bool {
        self.skip_spaces();
        let starred = matches!(
            self.t.get(self.i).map(|input| &input.token.kind),
            Some(TokenKind::Word(word)) if word.starts_with('*')
        );
        if starred {
            self.trim_word_prefix(1);
        }
        starred
    }

    /// Drops the first `len` bytes of the `Word` token at the cursor, moving
    /// past the token when nothing is left (`skip_line_break_length` does the
    /// same for `\\[3pt]Next`).
    fn trim_word_prefix(&mut self, len: usize) {
        let Some(input) = self.token_mut(self.i) else {
            return;
        };
        let TokenKind::Word(word) = &input.token.kind else {
            return;
        };
        let full = word.len();
        let rest = word[len.min(full)..].to_string();
        if rest.is_empty() {
            self.i += 1;
            return;
        }
        let span = input.token.span;
        // Only a token whose span matches its text can be re-spanned; one a
        // macro produced keeps the call site's span.
        if span.end - span.start == full {
            input.token.span = Span::in_document(span.document, span.start + len, span.end);
        }
        input.token.kind = TokenKind::Word(rest);
    }

    /// The natbib options in force, or natbib's own defaults plus one error
    /// when the document never loaded the package — which is what pdfLaTeX
    /// reports too, as an undefined control sequence.
    fn natbib_options(&mut self, name: &str, span: Span) -> natbib::Options {
        match self.bibliography.natbib() {
            Some(options) => options.clone(),
            None => {
                self.diags.push(Diagnostic::error(
                    format!("\\{name} is a natbib command, but this document does not \\usepackage{{natbib}}"),
                    Some(span),
                    Some("set the citation with natbib's default author-year style".into()),
                ));
                natbib::Options::default()
            }
        }
    }

    /// `\citet`, `\citep`, `\citealt`, `\citealp`, `\citeauthor`,
    /// `\citefullauthor`, `\citeyear`, `\citeyearpar`, `\citenum` and their
    /// starred and `\Cite`-capitalised forms.
    fn natbib_cite(&mut self, name: &str, span: Span, para: &mut Vec<Inline>) {
        let star = self.take_cite_star();
        let command = if star { format!("{name}*") } else { name.to_string() };
        let options = self.natbib_options(name, span);
        let (pre, post) = self.cite_notes();
        let (tokens, argument_span) = self.required_group(name, span);
        let full_span = span.merge(argument_span);
        let keys = cite_keys(&tokens);
        self.document_global_state = true;
        if keys.is_empty() {
            self.diags.push(Diagnostic::warning(
                format!("\\{name} was given an empty key list"),
                Some(full_span),
                Some("rendered nothing for the empty citation".into()),
            ));
            return;
        }
        let Some(kind) = natbib::kind(&command) else {
            // `\citeyear*` and friends: natbib's `\citeyear` takes no star,
            // so the `*` is ordinary text after the citation.
            self.diags.push(Diagnostic::warning(
                format!("\\{command} is not a natbib command"),
                Some(full_span),
                Some(format!("set it as \\{name}")),
            ));
            let kind = natbib::kind(name).expect("dispatch arm is a natbib command");
            self.push_natbib_cite(&options, kind, pre, post, &keys, full_span, para);
            return;
        };
        self.push_natbib_cite(&options, kind, pre, post, &keys, full_span, para);
    }

    #[allow(clippy::too_many_arguments)]
    fn push_natbib_cite(
        &mut self,
        options: &natbib::Options,
        kind: natbib::Kind,
        pre: Option<String>,
        post: Option<String>,
        keys: &[String],
        span: Span,
        para: &mut Vec<Inline>,
    ) {
        if options.sort || options.compress {
            self.note_natbib_limitation(
                "natbib's sort/compress options are parsed but not applied; citations keep the order the document wrote them",
                span,
            );
        }
        if options.superscript {
            self.note_natbib_limitation(
                "natbib's super option is parsed but the numbers are set on the baseline, not raised",
                span,
            );
        }
        if options.longnamesfirst {
            self.note_natbib_limitation(
                "natbib's longnamesfirst option is parsed but every citation uses the short author list",
                span,
            );
        }
        if self.bibliography.natbib_forced_numbers() && !self.natbib_forced_numbers_reported {
            self.natbib_forced_numbers_reported = true;
            self.diags.push(Diagnostic::warning(
                "Package natbib Error: Bibliography not compatible with author-year citations",
                Some(span),
                Some(
                    "a \\bibitem has no [Author(Year)] label; continued in numerical citation style, as natbib's second pass does"
                        .into(),
                ),
            ));
        }
        let bibliography = &self.bibliography;
        let inlines = natbib::cite_inlines(
            options,
            kind,
            pre.as_deref(),
            post.as_deref(),
            keys,
            &|key: &str| bibliography.entry(key),
            span,
            &mut self.diags,
        );
        para.extend(inlines);
    }

    /// One diagnostic per natbib option this implementation does not apply,
    /// however many citations the document has.
    fn note_natbib_limitation(&mut self, message: &str, span: Span) {
        if !self.natbib_limitations.insert(message.to_string()) {
            return;
        }
        self.diags.push(Diagnostic::warning(
            message,
            Some(span),
            Some("rendered the citation without it".into()),
        ));
    }

    fn set_current_counter(&mut self, kind: &str, value: Option<String>) {
        self.current_counter_kind = value.as_ref().map(|_| kind.to_string());
        self.current_counter = value;
    }

    fn use_package(&mut self, span: Span) {
        // siunitx keys keep their braces (`output-decimal-marker={,}`).
        let raw_options = {
            let start = self.i;
            let raw = self.siunitx_bracket().map(|(raw, _)| raw);
            self.i = start;
            raw
        };
        let options = self
            .optional_bracket_argument()
            .map(|(options, _)| options)
            .unwrap_or_default();
        let (tokens, argument_span) = self.required_group("usepackage", span);
        let packages: Vec<String> = token_text(&tokens)
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect();
        if packages.is_empty() {
            self.diags.push(Diagnostic::warning(
                "\\usepackage was given an empty package list",
                Some(span.merge(argument_span)),
                Some("no packages were loaded".into()),
            ));
            return;
        }
        self.packages.extend(packages.iter().cloned());
        for package in &packages {
            self.math_packages.load_package(package);
            self.load_color_package(package, &options);
            if package == "cleveref" {
                self.cleveref.set_options(&options);
            }
        }
        // xcolor.sty's `table` option loads colortbl (and so array).
        if packages.iter().any(|package| package == "xcolor")
            && options.split(',').any(|option| option.trim() == "table")
        {
            self.packages.push("colortbl".into());
        }
        // multicol.sty lines 111-113: the global `twocolumn` class option
        // reaches the package's option handler.
        if self.twocolumn_option && packages.iter().any(|package| package == "multicol") {
            self.diags.push(Diagnostic::warning(
                "Package multicol Warning: May not work with the twocolumn option",
                Some(span.merge(argument_span)),
                Some("multicols is set inside the page column".into()),
            ));
        }
        if packages.iter().any(|package| package == "fontenc") {
            if let Some(encoding) = text_builtins::fontenc_encoding(&options) {
                self.font_encoding = encoding;
            }
        }
        if let Some(raw) = raw_options.filter(|_| packages.iter().any(|p| p == "siunitx")) {
            siunitx::load_package(&raw, span.merge(argument_span), &mut self.diags);
        }
        let packages: Vec<String> = packages
            .into_iter()
            .filter(|package| !package_matches_layout(package, &options))
            .collect();
        if packages.is_empty() {
            return;
        }
        self.diags.push(Diagnostic::warning(
            format!(
                "packages {} are recognised but not implemented",
                packages.join(", ")
            ),
            Some(span.merge(argument_span)),
            Some("continued without package-specific commands or formatting".into()),
        )
        .with_help(
            "remove that \\usepackage if you do not need it; its commands are still diagnosed when used",
        ));
    }

    fn clever_reference(&mut self, name: &str, span: Span, para: &mut Vec<Inline>) {
        let linked = !self.take_optional_star();
        let space_before = self.space_precedes(self.i - 1);
        let range = matches!(name, "crefrange" | "Crefrange");
        let page = matches!(name, "cpageref" | "Cpageref");
        let label_only = name == "labelcref";
        let capitalise = matches!(name, "Cref" | "Crefrange" | "Cpageref");
        let full_span;
        let keys = if range {
            let (first, first_span) = self.required_group(name, span);
            let (second, second_span) = self.required_group(name, span);
            full_span = span.merge(first_span).merge(second_span);
            vec![token_text(&first).trim().to_string(), token_text(&second).trim().to_string()]
        } else {
            let (tokens, argument_span) = self.required_group(name, span);
            full_span = span.merge(argument_span);
            token_text(&tokens)
                .split(',')
                .map(str::trim)
                .map(str::to_string)
                .collect()
        };
        self.document_global_state = true;
        para.push(Inline::CleverReference {
            keys,
            page,
            range,
            label_only,
            capitalise,
            linked,
            span: full_span,
            space_before,
        });
    }

    fn cleveref_name(&mut self, name: &str, span: Span) {
        let (kind, kind_span) = self.required_group(name, span);
        let (singular, singular_span) = self.required_group(name, span);
        let (plural, plural_span) = self.required_group(name, span);
        self.cleveref.set_name(
            token_text(&kind).trim().to_string(),
            token_text(&singular).to_string(),
            token_text(&plural).to_string(),
            name == "Crefname",
        );
        self.document_global_state = true;
        self.current_dependencies.clear();
        let _ = kind_span.merge(singular_span).merge(plural_span);
    }

    /// `\begin{multicols}{<n>}[<preface>][<premulticols>]` and `multicols*`
    /// (multicol.sty 2025/10/21 v2.0b, lines 145-205 and 894-905). The
    /// column count is checked as `\multicols` checks it: fewer than two
    /// columns become two with multicol's warning, more than twenty become
    /// twenty with its error. The preface is `#1\par` in `\mult@@cols`: it
    /// stays in the token stream as ordinary body material, its `[` blanked
    /// and its `]` turned into the paragraph break. `[<premulticols>]` only
    /// decides a page break and is dropped here. The columns themselves are
    /// laid out by the render pipeline (`typeset::multicol`).
    fn multicols_arguments(&mut self, span: Span, environment: &str) {
        let (tokens, count_span) = self.required_group(environment, span);
        let count = token_text(&tokens).trim().to_string();
        let at = Some(span.merge(count_span));
        match count.parse::<i64>() {
            Ok(n) if n < 2 => self.diags.push(Diagnostic::warning(
                format!("Package multicol Warning: Using `{n}' columns doesn't seem a good idea. I therefore use two columns instead"),
                at,
                Some("set two columns".into()),
            )),
            Ok(n) if n > 20 => self.diags.push(Diagnostic::error(
                "Package multicol Error: Too many columns; the current implementation doesn't support more than 20 columns",
                at,
                Some("set 20 columns".into()),
            )),
            Ok(_) => {}
            Err(_) => self.diags.push(Diagnostic::warning(
                format!("{environment} expects a number of columns, got '{count}'"),
                at,
                Some("set two columns".into()),
            )),
        }
        self.skip_spaces();
        let open = self.i;
        let Some(close) = self.bracket_close(open) else { return };
        // `[<premulticols>]` right after the preface (`\@ifnextchar[` skips
        // spaces): in the same word as the preface's `]` (`][80pt]`), or in
        // the words that follow.
        let (close_token, at) = close;
        let rest = match &self.t[close_token].token.kind {
            TokenKind::Word(word) => word[at + 1..].to_string(),
            _ => String::new(),
        };
        if rest.starts_with('[') {
            if let Some(end) = rest.find(']') {
                if let Some(t) = self.token_mut(close_token) {
                    if let TokenKind::Word(word) = &mut t.token.kind {
                        word.replace_range(at + 1..at + 2 + end, "");
                    }
                }
            }
        } else if rest.is_empty() {
            let mut next = close_token + 1;
            while next < self.t.len() && matches!(self.t[next].token.kind, TokenKind::Space | TokenKind::Comment) {
                next += 1;
            }
            if let Some((close2, _)) = self.bracket_close(next) {
                for k in next..=close2 {
                    if let Some(t) = self.token_mut(k) {
                        t.token.kind = TokenKind::Comment;
                    }
                }
            }
        }
        self.blank_preface_brackets(open, close);
    }

    /// The token and byte offset of the `]` closing the `[` that starts the
    /// word at `open` (brackets inside braces do not count; a blank line
    /// ends the search).
    fn bracket_close(&self, open: usize) -> Option<(usize, usize)> {
        let TokenKind::Word(first) = &self.t.get(open)?.token.kind else {
            return None;
        };
        if !first.starts_with('[') {
            return None;
        }
        let (mut depth, mut braces) = (0i32, 0i32);
        for k in open..self.t.len() {
            match &self.t[k].token.kind {
                TokenKind::LBrace => braces += 1,
                TokenKind::RBrace => braces -= 1,
                TokenKind::ParBreak => return None,
                TokenKind::Word(word) if braces == 0 => {
                    for (at, c) in word.char_indices() {
                        match c {
                            '[' => depth += 1,
                            ']' => {
                                depth -= 1;
                                if depth == 0 {
                                    return Some((k, at));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// The preface's `[` disappears and its `]` becomes `\par`.
    fn blank_preface_brackets(&mut self, open: usize, (close, at): (usize, usize)) {
        let mut rest_of_word = false;
        if let Some(t) = self.token_mut(close) {
            if let TokenKind::Word(word) = &mut t.token.kind {
                word.remove(at);
                if word.is_empty() {
                    t.token.kind = TokenKind::ParBreak;
                } else {
                    rest_of_word = true;
                }
            }
        }
        if rest_of_word && matches!(self.t.get(close + 1).map(|t| &t.token.kind), Some(TokenKind::Space)) {
            if let Some(t) = self.token_mut(close + 1) {
                t.token.kind = TokenKind::ParBreak;
            }
        }
        if let Some(t) = self.token_mut(open) {
            if let TokenKind::Word(word) = &mut t.token.kind {
                if word.starts_with('[') {
                    word.remove(0);
                }
                if word.is_empty() {
                    t.token.kind = TokenKind::Comment;
                }
            }
        }
    }

    /// `\maketitle`: builds `Block::TitleBlock` from whatever `\title`/
    /// `\author`/`\date` are currently set to, mirroring how real
    /// `article.cls` reads `\@title`/`\@author`/`\@date`. Requires `\title`
    /// (LaTeX's `No \title given` is an error) and otherwise produces no
    /// block. A missing `\author` is LaTeX's `No \author given` warning and an
    /// empty `\author{}` is silent; both set the block with no author line,
    /// which is LaTeX's empty author box, not a fabricated placeholder.
    fn maketitle(&mut self, span: Span, blocks: &mut Vec<Block>, para: &mut Vec<Inline>) {
        self.flush_paragraph(blocks, para);

        let Some((title_tokens, title_span)) = self.title.clone() else {
            self.diags.push(Diagnostic::error(
                "\\maketitle requires \\title to be set first",
                Some(span),
                Some("no title block was produced".into()),
            )
            .with_help("add \\title{...} before \\maketitle"));
            return;
        };
        // latex.ltx: `\def\@author{\@latex@warning@no@line{No \noexpand\author
        // given}}`. The title block is set regardless, with an empty author box.
        let (author_tokens, author_span) = self.author.clone().unwrap_or_else(|| {
            self.diags.push(Diagnostic::warning(
                "No \\author given",
                Some(span),
                Some("set the title block without an author line, as LaTeX does".into()),
            )
            .with_help("add \\author{...} before \\maketitle; an empty \\author{} is silent like LaTeX"));
            (Vec::new(), span)
        });

        // `\@maketitle` sets `\@title`, `\@author`, `\@date` in that
        // order; each `\thanks` steps `footnote` there.
        let title_content = self.thanks_inlines(title_tokens, TextStyle::default());
        if title_content.is_empty() {
            self.diags.push(Diagnostic::error(
                "\\title was given an empty title",
                Some(title_span),
                Some("no title block was produced".into()),
            ));
            return;
        }

        let author_groups = split_on_and(author_tokens);
        let and_count = author_groups.len().saturating_sub(1);
        let mut author_content: Vec<Inline> = Vec::new();
        let mut wrote_author = false;
        for group in author_groups {
            let inlines = self.thanks_inlines(group, TextStyle::default());
            if inlines.is_empty() {
                // A blank `\and`-separated slot (`\author{A \and }`)
                // contributes nothing, like an empty tabular column.
                continue;
            }
            if wrote_author {
                author_content.push(Inline::LineBreak {
                    span: author_span,
                    skip_pt: None,
                });
            }
            author_content.extend(inlines);
            wrote_author = true;
        }
        // `\author{}` (or only blank `\and` slots) is an author that is given
        // but empty: pdfLaTeX sets an empty author box without a warning.
        if and_count > 0 && wrote_author {
            self.diags.push(Diagnostic::warning(
                "multiple \\and-separated authors are typeset one per line; this compiler does not yet place them side by side in columns",
                Some(author_span),
                Some("stacked the authors vertically instead of in columns".into()),
            ));
        }

        let date_content = match self.date.clone() {
            None => {
                // `\date` was never called: `article.cls`'s own preamble
                // default is `\date{\today}` (latex.ltx `\gdef\@date{\today}`),
                // so this is the same date `\date{\today}` would print.
                Some(vec![Inline::Text {
                    text: self.today.latex_today(),
                    span,
                    style: TextStyle::default(),
                    space_before: true,
                }])
            }
            Some((date_tokens, _)) => {
                let inlines = self.thanks_inlines(date_tokens, TextStyle::default());
                if inlines.is_empty() {
                    None // `\date{}`: suppressed, matching `DateField::Suppressed`.
                } else {
                    Some(inlines)
                }
            }
        };

        if self.titlepage_option {
            self.diags.push(Diagnostic::warning(
                "the 'titlepage' document class option asks for a dedicated, vertically centred title page; this compiler has no vertical-fill layout primitive yet",
                Some(span),
                Some("rendered the ordinary compact \\@maketitle block instead of a separate title page".into()),
            ));
        }

        blocks.push(Block::TitleBlock {
            title: title_content,
            authors: author_content,
            date: date_content,
        });
        // `\maketitle` ends with `\setcounter{footnote}{0}`.
        self.footnote_counter = 0;
        self.finish_block_dependencies();
    }

    /// A captured `\title`/`\author`/`\date` argument as inline content
    /// with every `\thanks{...}` turned into a footnote. article/report/
    /// book's `\maketitle` sets `\thefootnote` to `\@fnsymbol\c@footnote`
    /// and `\thanks` is `\footnotemark` plus a `\footnotetext[n]{...}`
    /// queued in `\@thanks` (set after `\@maketitle`, in vertical mode):
    /// the inline carries the symbol mark and the note text at the mark's
    /// position; the layout decides where the text goes. The span is the
    /// `\thanks` token.
    fn thanks_inlines(&mut self, tokens: Vec<InputToken>, style: TextStyle) -> Vec<Inline> {
        let mut out: Vec<Inline> = Vec::new();
        let mut segment: Vec<InputToken> = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            let is_thanks = matches!(
                &tokens[i].token.kind,
                TokenKind::Command(name) if name == "thanks"
            );
            if !is_thanks {
                segment.push(tokens[i].clone());
                i += 1;
                continue;
            }
            let thanks_span = tokens[i].token.span;
            let mut j = i + 1;
            while j < tokens.len()
                && matches!(tokens[j].token.kind, TokenKind::Space | TokenKind::Comment)
            {
                j += 1;
            }
            if j >= tokens.len() || tokens[j].token.kind != TokenKind::LBrace {
                self.diags.push(Diagnostic::warning(
                    "\\thanks without a braced argument",
                    Some(thanks_span),
                    Some("omitted the footnote mark".into()),
                ));
                i += 1;
                continue;
            }
            let open = j;
            let mut depth = 0usize;
            while j < tokens.len() {
                match tokens[j].token.kind {
                    TokenKind::LBrace => depth += 1,
                    TokenKind::RBrace => depth -= 1,
                    _ => {}
                }
                j += 1;
                if depth == 0 {
                    break;
                }
            }
            let close = if depth == 0 { j - 1 } else { j };
            let argument = tokens[open + 1..close].to_vec();
            let before = std::mem::take(&mut segment);
            out.extend(self.inlines_from_tokens(before, style));
            self.document_global_state = true;
            self.footnote_counter += 1;
            let number = match fnsymbol(self.footnote_counter) {
                Some(symbol) => symbol.to_string(),
                None => {
                    self.diags.push(Diagnostic::error(
                        format!(
                            "\\thanks number {} is outside \\@fnsymbol's nine symbols",
                            self.footnote_counter
                        ),
                        Some(thanks_span),
                        Some("printed the number in arabic instead".into()),
                    ));
                    self.footnote_counter.to_string()
                }
            };
            let text = self.footnote_inlines(argument, thanks_span);
            out.push(Inline::Footnote {
                number,
                span: thanks_span,
                mark: true,
                text: Some(text),
                space_before: false,
            });
            i = j;
        }
        out.extend(self.inlines_from_tokens(segment, style));
        out
    }

    /// `\chapter[*][<short>]{<title>}` in report/book:
    /// `\refstepcounter{chapter}` for the numbered form, which resets
    /// `section` (and below), `figure`, `table` and `equation` through the
    /// counter table (report.cls/book.cls `\@addtoreset`), and `footnote`,
    /// which this parser still counts in a field of its own. The head itself
    /// (`\@makechapterhead`, the page break, the running marks) is layout:
    /// the title is kept as a bold paragraph.
    fn chapter(&mut self, span: Span, blocks: &mut Vec<Block>, para: &mut Vec<Inline>) {
        let starred = self.take_optional_star();
        if !starred {
            let _short = self.optional_bracket_argument();
        }
        let (tokens, _) = self.required_group("chapter", span);
        self.flush_paragraph(blocks, para);
        self.document_global_state = true;
        if !starred {
            // Stepping `chapter` resets every counter registered within it
            // (`Counters::report`), so `figure`/`table`/`equation` need no
            // zeroing here; `footnote` is not in the counter table yet.
            let number = self.counters.step("chapter").unwrap_or_default();
            self.footnote_counter = 0;
            self.set_current_counter("chapter", Some(number));
        }
        let content = self.inlines_from_tokens(tokens, TextStyle::BOLD);
        if content.is_empty() {
            self.current_dependencies.clear();
        } else {
            blocks.push(Block::Paragraph(content));
            self.finish_block_dependencies();
        }
    }

    // ---- letter.cls -----------------------------------------------------

    /// Whether a `letter.cls` command may run here. Every one of them is
    /// defined by that class alone: in an `article` pdflatex answers
    /// "Undefined control sequence" and typesets the argument as ordinary
    /// text, so that is what happens here too — the diagnostic names the
    /// command and the brace group is left for the main token loop, which
    /// keeps the author's prose on the page.
    fn letter_command_available(&mut self, name: &str, span: Span) -> bool {
        if self.is_letter_class() {
            return true;
        }
        let class = self
            .document_class
            .clone()
            .unwrap_or_else(|| "no \\documentclass".to_string());
        self.diags.push(
            Diagnostic::error(
                format!(
                    "\\{name} is defined by the letter document class; this document is {class}"
                ),
                Some(span),
                Some("skipped the command; any braced argument was typeset as plain text".into()),
            )
            .with_code(crate::diagnostics::DiagnosticCode::UnknownCommand),
        );
        false
    }

    /// `\address`, `\signature`, `\name`, `\location`, `\telephone`
    /// (letter.cls 154-158): `\newcommand*\x[1]{\def\fromx{#1}}`. Nothing is
    /// typeset; the replacement text is stored for `\opening`/`\closing`.
    fn letter_declaration(&mut self, name: &str, span: Span) {
        if !self.letter_command_available(name, span) {
            return;
        }
        let (tokens, argument_span) = self.required_group(name, span);
        let value = Some((tokens, span.merge(argument_span)));
        match name {
            "address" => self.letter.address = value,
            "signature" => self.letter.signature = value,
            "name" => self.letter.name = value,
            "location" => self.letter.location = value,
            _ => self.letter.telephone = value,
        }
    }

    /// The date `\opening` sets: `\@date`, which latex.ltx initialises to
    /// `\today` and `\date{...}` overrides. `\date{}` really does leave it
    /// empty, and the box then holds nothing for that line.
    fn letter_date_inlines(&mut self, span: Span) -> Vec<Inline> {
        match self.date.clone() {
            Some((tokens, _)) => self.inlines_from_tokens(tokens, TextStyle::default()),
            None => vec![Inline::Text {
                text: self.today.latex_today(),
                span,
                style: TextStyle::default(),
                space_before: false,
            }],
        }
    }

    /// `\opening{...}` (letter.cls 223-234), in source order:
    ///
    /// 1. `{\raggedleft <\fromaddress lines> \\*[2\parskip] \@date \par}`,
    ///    the address lines and the date in one `tabular{l@{}}` box pushed to
    ///    the right margin — [`LetterPart::ReturnAddress`]. With no
    ///    `\address` the class sets only `{\raggedleft\@date\par}`, which is
    ///    the same box with one line.
    /// 2. `\vspace{2\parskip}`.
    /// 3. `{\raggedright \toname \\ \toaddress \par}` at the left margin.
    /// 4. `\vspace{2\parskip}`.
    /// 5. the salutation, `#1\par\nobreak`.
    fn letter_opening(&mut self, span: Span, blocks: &mut Vec<Block>, para: &mut Vec<Inline>) {
        if !self.letter_command_available("opening", span) {
            return;
        }
        self.flush_paragraph(blocks, para);
        let (tokens, argument_span) = self.required_group("opening", span);
        let full = span.merge(argument_span);
        if self.letter.recipient.is_none() {
            self.diags.push(Diagnostic::warning(
                "\\opening is outside \\begin{letter}{...}, so there is no recipient address to set",
                Some(span),
                Some("set the return address, the date and the salutation without a recipient block".into()),
            ));
        }
        self.letter.opened = true;
        let parskip = letter_parskip_pt(self.class_size_pt);

        // 1. return address and date.
        let mut lines: Vec<Vec<Inline>> = Vec::new();
        let mut gaps: Vec<f64> = Vec::new();
        if let Some((address, _)) = self.letter.address.clone() {
            let address = self.inlines_from_tokens(address, TextStyle::default());
            let address = split_at_line_breaks(address);
            let last = address.len().saturating_sub(1);
            for (index, line) in address.into_iter().enumerate() {
                lines.push(line);
                // `\\*[2\parskip]` sits between the address and the date.
                gaps.push(if index == last { 2.0 * parskip } else { 0.0 });
            }
        }
        lines.push(self.letter_date_inlines(span));
        gaps.push(0.0);
        blocks.push(Block::LetterBlock {
            part: LetterPart::ReturnAddress,
            lines,
            extra_gap_after_pt: gaps,
            gap_before_pt: 0.0,
            gap_after_pt: 0.0,
            indent_pt: 0.0,
            span: full,
        });
        self.finish_block_dependencies();

        // 2-4. the recipient, `\raggedright` at the left margin, with
        // `\vspace{2\parskip}` on each side of it.
        let recipient = self
            .letter
            .recipient
            .clone()
            .map(|(tokens, _)| self.inlines_from_tokens(tokens, TextStyle::default()))
            .unwrap_or_default();
        let recipient_span = self
            .letter
            .recipient
            .as_ref()
            .map_or(full, |(_, span)| *span);
        let lines = split_at_line_breaks(recipient);
        let gaps = vec![0.0; lines.len()];
        blocks.push(Block::LetterBlock {
            part: LetterPart::Recipient,
            lines,
            extra_gap_after_pt: gaps,
            gap_before_pt: 2.0 * parskip,
            gap_after_pt: 2.0 * parskip,
            indent_pt: 0.0,
            span: recipient_span,
        });
        self.finish_block_dependencies();

        // 5. the salutation: an ordinary paragraph, so it justifies and
        // wraps like the body that follows it.
        let content = self.inlines_from_tokens(tokens, TextStyle::default());
        if !content.is_empty() {
            blocks.push(Block::Paragraph(content));
            self.finish_block_dependencies();
        }
    }

    /// `\closing{...}` (letter.cls 235-247):
    /// `\par\nobreak\vspace{\parskip}\noindent\hspace*{\longindentation}`
    /// `\parbox{\indentedwidth}{\raggedright #1 \\[6\medskipamount]`
    /// `\fromsig-or-\fromname\strut}`. `\medskipamount` is `\parskip` here
    /// (line 236), so the gap is exactly six paragraph skips.
    ///
    /// The `\hspace*{\longindentation}` is omitted when `\fromaddress` is
    /// empty, so a letter with no `\address` closes at the left margin.
    fn letter_closing(&mut self, span: Span, blocks: &mut Vec<Block>, para: &mut Vec<Inline>) {
        if !self.letter_command_available("closing", span) {
            return;
        }
        self.flush_paragraph(blocks, para);
        let (tokens, argument_span) = self.required_group("closing", span);
        let full = span.merge(argument_span);
        let parskip = letter_parskip_pt(self.class_size_pt);
        let closing = self.inlines_from_tokens(tokens, TextStyle::default());
        let mut lines = split_at_line_breaks(closing);
        let mut gaps = vec![0.0; lines.len()];
        // `\ifx\@empty\fromsig \fromname \else \fromsig \fi`.
        let signature = self
            .letter
            .signature
            .clone()
            .or_else(|| self.letter.name.clone());
        if let Some((signature, _)) = signature {
            let signature = self.inlines_from_tokens(signature, TextStyle::default());
            let signature = split_at_line_breaks(signature);
            if let Some(last) = gaps.last_mut() {
                *last = letter_signature_gap_pt(self.class_size_pt);
            }
            lines.extend(signature);
            gaps.resize(lines.len(), 0.0);
        }
        blocks.push(Block::LetterBlock {
            part: LetterPart::Closing,
            lines,
            extra_gap_after_pt: gaps,
            // `\par\nobreak\vspace{\parskip}` opens `\closing` (letter.cls
            // 235), on top of the paragraph's own `\parskip`.
            gap_before_pt: parskip,
            gap_after_pt: 0.0,
            // `\hspace*{\longindentation}` — but only when there is a
            // return address: letter.cls 239 makes the indent conditional on
            // `\fromaddress` being non-empty, so a letter without one closes
            // at the left margin.
            indent_pt: if self.letter.address.is_some() {
                letter_longindentation_pt(self.class_size_pt)
            } else {
                0.0
            },
            span: full,
        });
        self.finish_block_dependencies();
    }

    /// `\cc{...}` and `\encl{...}` (letter.cls 237-244):
    /// `\par\noindent\parbox[t]{\textwidth}{\@hangfrom{\ccname: }#1\strut}\par`.
    ///
    /// The label is `\ccname`/`\enclname` — literally `cc` and `encl`
    /// (letter.cls 392-393), lowercase, with a colon and a space. The
    /// `\@hangfrom` hangs continuation lines under the text after the label;
    /// this compiler has no hanging indent outside `\item`, so a short
    /// annotation (the common case, and the corpus fixture's) is exact and a
    /// wrapped one loses the hang. That is a placement difference within the
    /// same block, not dropped content, so it is not worth a diagnostic on
    /// every `\cc`.
    fn letter_annotation(
        &mut self,
        name: &str,
        span: Span,
        blocks: &mut Vec<Block>,
        para: &mut Vec<Inline>,
    ) {
        if !self.letter_command_available(name, span) {
            return;
        }
        self.flush_paragraph(blocks, para);
        let (tokens, argument_span) = self.required_group(name, span);
        // No trailing space in the label: the annotation's own first run
        // starts a group, so `inlines_from_tokens` already marks it
        // `space_before`, and baking one in here would set two.
        let mut content = vec![Inline::Text {
            text: format!("{name}:"),
            span: span.merge(argument_span),
            style: TextStyle::default(),
            space_before: false,
        }];
        content.extend(self.inlines_from_tokens(tokens, TextStyle::default()));
        blocks.push(Block::Paragraph(content));
        self.finish_block_dependencies();
    }

    fn environment(
        &mut self,
        kind: &str,
        span: Span,
        blocks: &mut Vec<Block>,
        para: &mut Vec<Inline>,
    ) {
        let (tokens, argument_span) = self.required_group(kind, span);
        let environment = token_text(&tokens).trim().to_string();
        if kind == "begin" {
            if matches!(
                environment.as_str(),
                "equation" | "equation*" | "displaymath"
            ) && self.in_body
            {
                self.equation_environment(span, &environment, blocks, para);
                return;
            }
            if matches!(
                environment.as_str(),
                "gather"
                    | "gather*"
                    | "align"
                    | "align*"
                    | "alignat"
                    | "alignat*"
                    | "flalign"
                    | "flalign*"
                    | "multline"
                    | "multline*"
            ) && self.in_body
            {
                self.multirow_environment(span, &environment, blocks, para);
                return;
            }
            if matches!(environment.as_str(), "tabular" | "tabular*") && self.in_body {
                self.tabular_environment(span, &environment, para);
                return;
            }
            // Package environments (inventoried with their package).
            if self.in_body && self.package_table_environment(span, &environment, blocks, para) {
                return;
            }
            if matches!(
                environment.as_str(),
                "verbatim" | "verbatim*" | "lstlisting"
            ) && self.in_body
            {
                self.verbatim_environment(span, argument_span, &environment, blocks, para);
                return;
            }
            self.env_alignments.push(self.declared_alignment);
            if environment == "document" && self.has_document {
                self.in_body = true;
            } else if environment == "figure" && self.in_body {
                self.flush_paragraph(blocks, para);
            } else if let (Some(style), true) = (paragraph_style(&environment), self.in_body) {
                self.flush_paragraph(blocks, para);
                self.paragraph_styles.push(style);
                // An inner alignment environment overrides an outer declaration.
                if style != ParagraphStyle::Quote {
                    self.declared_alignment = None;
                }
                if let Some(kind) = ListEnvironment::from_name(&environment) {
                    self.push_list_frame(kind, Vec::new(), span.merge(argument_span));
                }
            } else if matches!(
                environment.as_str(),
                "itemize" | "enumerate" | "description"
            ) && self.in_body
            {
                self.flush_paragraph(blocks, para);
                let options = self.optional_bracket_argument();
                let begin_span = options
                    .as_ref()
                    .map_or(span.merge(argument_span), |(_, o)| span.merge(*o));
                self.open_list(&environment, options.map(|(text, _)| text), begin_span, blocks.len());
            } else if self.in_body
                && (self.theorems.contains_key(&environment) || environment == "proof")
            {
                self.flush_paragraph(blocks, para);
            } else if environment == "thebibliography" && self.in_body {
                self.flush_paragraph(blocks, para);
                // article.cls: `\begin{thebibliography}{#1}` is
                // `\section*{\refname}` followed by a `\list` whose
                // `\labelwidth` is set from `#1` (the widest label the
                // author expects, e.g. `{99}` for up to 99 entries).
                let (widest_tokens, widest_span) = self.required_group(&environment, span);
                let widest_label = token_text(&widest_tokens).trim().to_string();
                self.document_global_state = true;
                let heading_span = span.merge(argument_span).merge(widest_span);
                blocks.push(Block::Heading {
                    level: 1,
                    number: String::new(),
                    number_span: heading_span,
                    content: vec![Inline::Text {
                        text: "References".to_string(),
                        span: heading_span,
                        style: TextStyle::BOLD,
                        space_before: false,
                    }],
                });
                self.finish_block_dependencies();
                let spacing = self
                    .list_spacing
                    .get(&environment)
                    .copied()
                    .unwrap_or_default();
                self.list_stack.push(OpenList {
                    kind: environment.clone(),
                    count: 0,
                    template: Some(widest_label),
                    spacing,
                    start: blocks.len(),
                    counter: 0,
                    label_star: None,
                    current_label: String::new(),
                    current_reference: String::new(),
                    series: None,
                    begin_options: Vec::new(),
                });
                self.push_list_frame(ListEnvironment::Bibliography, Vec::new(), heading_span);
            } else if environment == "subequations" && self.in_body {
                self.begin_subequations();
            } else if matches!(environment.as_str(), "multicols" | "multicols*") && self.in_body {
                // `\mult@@cols` starts with `\par`.
                self.flush_paragraph(blocks, para);
                self.multicols_arguments(span.merge(argument_span), &environment);
            } else if environment == "letter" && self.in_body && self.is_letter_class() {
                // letter.cls 174-186: `\newenvironment{letter}[1]{\newpage
                // ... \c@page\@ne ... \@processto{...#1}}`. The mandatory
                // argument is the recipient; `\@processto` splits it at the
                // first `\\` into `\toname` and `\toaddress`, which
                // `\opening` then sets one per line — so it is stored whole
                // and the `\\`s are kept, exactly as written.
                self.flush_paragraph(blocks, para);
                let (recipient, recipient_span) = self.required_group(&environment, span);
                self.letter.recipient = Some((recipient, span.merge(recipient_span)));
                self.letter.opened = false;
                // Each letter starts a fresh page; the first one in a
                // document does not, because `\newpage` with nothing queued
                // ships no page (see `Block::PageBreak` in `layout`).
                blocks.push(Block::PageBreak);
                self.finish_block_dependencies();
            } else if self.in_body {
                self.diags.push(Diagnostic::environment_warning(
                    &environment,
                    format!(
                        "environment '{}' is not implemented; its body is typeset as plain text",
                        environment
                    ),
                    Some(span),
                    Some("typeset the body without the environment's formatting".into()),
                )
                .with_optional_help(vocabulary::environment_help(&environment)));
            }
            if is_minipage(&environment) {
                // `\@iiiminipage`: `\c@mpfootnote\z@`.
                self.mpfootnote_counter = 0;
            }
            self.env_stack
                .push((environment.clone(), span.merge(argument_span)));
            self.env_styles.push(self.style);
            if self.in_body {
                if let Some(theorem) = self.theorems.get(&environment).cloned() {
                    self.begin_theorem(&theorem, &environment, span, para);
                } else if environment == "proof" {
                    self.begin_proof(span, para);
                }
            }
            return;
        }

        let popped = self.env_stack.pop();
        let had_open_environment = popped.is_some();
        match popped {
            Some((open, _)) if open == environment => {}
            Some((open, _)) => self.diags.push(Diagnostic::error(
                format!(
                    "\\end{{{}}} does not match \\begin{{{}}}",
                    environment, open
                ),
                Some(span),
                Some("closed the innermost open environment".into()),
            )),
            None => self.diags.push(Diagnostic::error(
                format!("\\end{{{}}} with no matching \\begin", environment),
                Some(span),
                Some("ignored the stray \\end".into()),
            )),
        }
        if environment == "subequations" && self.in_body {
            self.end_subequations();
        }
        if environment == "letter" && self.in_body && self.is_letter_class() {
            // letter.cls 179-186 ends with `\stopletter\@@par\pagebreak`.
            // The `\pagebreak` is not emitted: `\end{document}`'s own
            // `\clearpage` absorbs the last one in real LaTeX, and a
            // `Block::PageBreak` here would ship a blank trailing page. The
            // next `\begin{letter}` starts its own page anyway.
            self.flush_paragraph(blocks, para);
            self.letter.recipient = None;
            self.letter.opened = false;
        }
        if paragraph_style(&environment).is_some() && self.in_body {
            self.flush_paragraph(blocks, para);
            self.paragraph_styles.pop();
        } else if matches!(
            environment.as_str(),
            "itemize" | "enumerate" | "description" | "thebibliography"
        ) {
            let (gap_before, gap_after) = match self.list_stack.last() {
                Some(list) => (
                    if list.count <= 1 {
                        list.spacing.topsep_pt
                    } else {
                        list.spacing.itemsep_pt
                    },
                    list.spacing.topsep_pt,
                ),
                None => (0.0, 0.0),
            };
            self.flush_list_item(blocks, para, gap_before, gap_after);
            let level = self.list_stack.len() as u8;
            if let Some(open) = self.list_stack.pop() {
                // `\enit@endlist` (enumitem.sty 1127-1146): the counter and
                // the `\begin` keys are kept for `resume`/`resume*`.
                if open.kind == "enumerate" {
                    self.resume_counters.insert(open.kind.clone(), open.counter);
                    self.resume_keys
                        .insert(open.kind.clone(), open.begin_options.clone());
                    if let Some(series) = &open.series {
                        let key = format!("series@{series}");
                        self.resume_counters.insert(key.clone(), open.counter);
                        self.resume_keys.insert(key, open.begin_options.clone());
                    }
                }
                let OpenList {
                    kind,
                    count,
                    template,
                    spacing,
                    start,
                    ..
                } = open;
                if spacing.leftmargin == LeftMarginSetting::Widest && count > 0 {
                    let labels: Vec<String> = if kind == "enumerate" {
                        // An alphabetic counter has only 26 possible single-
                        // letter values, so enumitem checks every one of them
                        // regardless of how many items this particular list
                        // has; other styles use this list's own item count
                        // (its labels only grow wider as the count does).
                        let widest_count = match &template {
                            Some(t) if matches!(enumitem_label_style(t), 'a' | 'A') => 26,
                            _ => count,
                        };
                        (1..=widest_count)
                            .map(|n| match &template {
                                Some(template) => enumitem_label(template, n),
                                None => format!("{n}."),
                            })
                            .collect()
                    } else {
                        vec!["•".to_string()]
                    };
                    for block in &mut blocks[start..] {
                        if let Block::ListItem {
                            level: item_level,
                            leftmargin,
                            ..
                        } = block
                        {
                            if *item_level == level && matches!(leftmargin, ListLeftMargin::Default)
                            {
                                *leftmargin = ListLeftMargin::Widest(labels.clone());
                            }
                        }
                    }
                }
            }
        } else if matches!(environment.as_str(), "multicols" | "multicols*") && self.in_body {
            // `\endmulticols` starts with `\par`.
            self.flush_paragraph(blocks, para);
        } else if environment == "figure" || self.theorems.contains_key(&environment) {
            self.flush_paragraph(blocks, para);
        } else if environment == "proof" {
            para.push(Inline::HFill { span, leader: FillLeader::None });
            para.push(Inline::Text {
                text: "∎".to_string(),
                span,
                style: TextStyle::default(),
                space_before: false,
            });
            self.flush_paragraph(blocks, para);
        }
        if environment == "document" && self.has_document {
            self.flush_paragraph(blocks, para);
            self.in_body = false;
            self.document_ended = true;
        }
        if let Some(kind) = ListEnvironment::from_name(&environment) {
            if self
                .list_frames
                .last()
                .is_some_and(|frame| frame.environment == kind)
            {
                self.list_frames.pop();
            }
            if kind == ListEnvironment::Verse {
                self.pending_line_break = None;
            }
        }
        // Restored only after the flushes above: environments that end their
        // paragraph do so while their own declarations are still in force.
        // The text style goes back with it, for the same reason and with the
        // same consequence: `\endtrivlist`'s `\ifhmode\unskip\par\fi` runs
        // before `\end`'s `\endgroup`, so the `\par` that closes
        // `\begin{quote}\small ...\end{quote}` reads `\small`'s
        // `\baselineskip`, not the body's (see [`ParLeading`]).
        if had_open_environment {
            if let Some(alignment) = self.env_alignments.pop() {
                self.declared_alignment = alignment;
            }
            if let Some(style) = self.env_styles.pop() {
                self.style = style;
            }
        }
    }

    /// `\newtheorem{name}{Title}`, its starred (unnumbered) form, the
    /// shared-counter form `\newtheorem{name}[shared]{Title}`, and the
    /// reset-on-section form `\newtheorem{name}{Title}[section]`. See
    /// `theorems::TheoremDef`.
    fn new_theorem(&mut self, span: Span) {
        let starred = self.take_optional_star();
        let (name_tokens, name_span) = self.required_group("newtheorem", span);
        let name = token_text(&name_tokens).trim().to_string();
        let shared = self.optional_bracket_argument();
        let (title_tokens, _) = self.required_group("newtheorem", span);
        let title = token_text(&title_tokens).trim().to_string();
        let within = if shared.is_none() {
            self.optional_bracket_argument()
        } else {
            None
        };
        if name.is_empty() {
            self.diags.push(Diagnostic::error(
                "\\newtheorem was given an empty environment name",
                Some(span.merge(name_span)),
                Some("ignored the declaration".into()),
            ));
            return;
        }
        let (counter, within_section) = match shared {
            Some((shared_name, shared_span)) => {
                let shared_name = shared_name.trim().to_string();
                match self.theorems.get(&shared_name) {
                    Some(existing) => (existing.counter.clone(), existing.within_section),
                    None => {
                        self.diags.push(Diagnostic::error(
                            format!(
                                "\\newtheorem{{{name}}}[{shared_name}] shares the counter of \
                                 undefined theorem environment '{shared_name}'"
                            ),
                            Some(shared_span),
                            Some("ignored the declaration".into()),
                        ));
                        return;
                    }
                }
            }
            None => {
                let within_section = match within {
                    None => false,
                    Some((counter_name, _)) if counter_name.trim() == "section" => true,
                    Some((counter_name, counter_span)) => {
                        let counter_name = counter_name.trim().to_string();
                        self.diags.push(Diagnostic::warning(
                            format!(
                                "\\newtheorem counter '[{counter_name}]' is recognised but not implemented"
                            ),
                            Some(counter_span),
                            Some(format!(
                                "'{name}' is numbered without resetting on '{counter_name}'"
                            )),
                        ));
                        false
                    }
                };
                (name.clone(), within_section)
            }
        };
        self.theorems.insert(
            name,
            TheoremDef {
                title,
                style: self.theorem_style,
                numbered: !starred,
                counter,
                within_section,
            },
        );
    }

    fn set_theorem_style(&mut self, span: Span) {
        let (tokens, argument_span) = self.required_group("theoremstyle", span);
        let name = token_text(&tokens).trim().to_string();
        match TheoremStyle::from_name(&name) {
            Some(style) => self.theorem_style = style,
            None => self.diags.push(Diagnostic::error(
                format!("\\theoremstyle{{{name}}} is not a recognised amsthm style"),
                Some(span.merge(argument_span)),
                Some("kept the previous \\theoremstyle in effect".into()),
            )),
        }
    }

    /// The head run and, for numbered environments, the counter for
    /// entering a `\newtheorem`-registered environment. Called after
    /// `self.style` has already been saved onto `env_styles` by the caller
    /// (see `environment`), so mutating it here to the body's default style
    /// is correctly restored at the matching `\end`.
    ///
    /// The head is `\the\thm@headfont \thm@indent <name> <number> <note>
    /// \the\thm@headpunct` (amsthm.sty `\@begintheorem`/`\thmhead@plain`):
    /// everything but the note is set in the head font — including the
    /// space tokens between the pieces and the trailing punctuation — while
    /// the note itself is `\thm@notefont{\fontseries\mddefault\upshape}` and
    /// the number is `\@upn` (upright). Measured against pdflatex (TeX Live
    /// 2025, `\documentclass[11pt]{article}`): `Definition 1.1 (Divides).`
    /// traces as `\T1/cmr/bx/n/10.95 D…n`, `\glue 4.17043 plus 2.08443
    /// minus 1.3896` (the *bold* interword space), `1.1`, the same bold
    /// glue, `\T1/cmr/m/n/10.95 (Divides)`, then `\T1/cmr/bx/n/10.95 .`;
    /// a numbered `remark` traces as italic `Remark`, italic glue,
    /// `\OT1/cmr/m/n/10.95 1`, italic `.`.
    fn begin_theorem(
        &mut self,
        def: &TheoremDef,
        kind: &str,
        span: Span,
        para: &mut Vec<Inline>,
    ) {
        let note = self.optional_bracket_argument();
        let head_style = def.style.head_style();
        // `\thmnumber{...\@upn{#2}}`: the number is `\textup`, so a
        // `remark`-style head (`\thm@headfont{\itshape}`) numbers upright
        // inside its italic name. For the bold heads `\@upn` is a no-op.
        let number_style = TextStyle {
            italic: false,
            ..head_style
        };
        let mut head = def.title.clone();
        let mut number = None;
        if def.numbered {
            let counter = self
                .theorem_counters
                .entry(def.counter.clone())
                .or_insert(0);
            *counter += 1;
            let n = *counter;
            let value = if def.within_section {
                format!("{}.{}", self.counters.value("section").unwrap_or(0), n)
            } else {
                n.to_string()
            };
            self.set_current_counter(kind, Some(value.clone()));
            // `\@ifnotempty{#1}{ }` sits outside `\@upn`, so the space token
            // between the name and the number is read in the head font
            // either way; the number only needs a run of its own where
            // `\@upn` actually changes the shape (a `remark` head).
            if number_style == head_style {
                head.push(' ');
                head.push_str(&value);
            } else {
                number = Some(value);
            }
        }
        para.push(Inline::Text {
            text: head,
            span,
            style: head_style,
            space_before: true,
        });
        if let Some(number) = number {
            para.push(Inline::Text {
                text: " ".to_string(),
                span,
                style: head_style,
                space_before: false,
            });
            para.push(Inline::Text {
                text: number,
                span,
                style: number_style,
                space_before: false,
            });
        }
        if let Some((note_text, note_span)) = note {
            let note_text = note_text.trim();
            if !note_text.is_empty() {
                // `\thmnote{ {\the\thm@notefont(#3)}}`: the space is outside
                // the `\thm@notefont` group, so it too is a head-font space;
                // only the parenthesised note itself is `\fontseries
                // \mddefault\upshape`.
                para.push(Inline::Text {
                    text: " ".to_string(),
                    span,
                    style: head_style,
                    space_before: false,
                });
                para.push(Inline::Text {
                    text: format!("({note_text})"),
                    span: note_span,
                    style: TextStyle::default(),
                    space_before: false,
                });
            }
        }
        // `\the\thm@headpunct` is typeset inside `\the\thm@headfont`'s
        // group: pdflatex sets a `plain`/`definition` head's period from the
        // bold face (`\T1/cmr/bx/n/10.95 .`) and a `remark`'s from the
        // italic one, never from the body font.
        para.push(Inline::Text {
            text: ".".to_string(),
            span,
            style: head_style,
            space_before: false,
        });
        self.style = def.style.body_style();
    }

    /// `proof`'s italic "Proof." head (or a custom `[...]` heading, still
    /// period-terminated) and upright body. The closing "∎" is appended by
    /// `environment`'s `\end` handling, once the body's last paragraph is
    /// known.
    fn begin_proof(&mut self, span: Span, para: &mut Vec<Inline>) {
        let heading = self
            .optional_bracket_argument()
            .map(|(text, _)| text.trim().to_string())
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "Proof".to_string());
        para.push(Inline::Text {
            text: format!("{heading}."),
            span,
            style: TextStyle {
                italic: true,
                ..TextStyle::default()
            },
            space_before: true,
        });
        self.style = TextStyle::default();
    }

    /// `verbatim`, `verbatim*`, and basic `lstlisting`. The body is not read
    /// from `self.t` at all: those tokens were produced by the ordinary
    /// tokenizer, which has already (mis)interpreted anything special inside
    /// (a `%` there would otherwise swallow the rest of its "line" as a
    /// `Comment`, hiding a real `\end{verbatim}` after it). Instead this
    /// finds the raw source bytes directly, with a plain literal search for
    /// `\end{name}` — the same finicky, whitespace-intolerant match real
    /// LaTeX's own verbatim scanner performs — then fast-forwards `self.i`
    /// past every token whose span the raw region swallowed.
    fn verbatim_environment(
        &mut self,
        open: Span,
        argument_span: Span,
        name: &str,
        blocks: &mut Vec<Block>,
        para: &mut Vec<Inline>,
    ) {
        self.flush_paragraph(blocks, para);
        let starred = name.ends_with('*');
        let mut content_start = argument_span.end;
        if name == "lstlisting" {
            if let Some((options, options_span)) = self.optional_bracket_argument() {
                content_start = options_span.end;
                if !options.trim().is_empty() {
                    self.diags.push(Diagnostic::warning(
                        "lstlisting options are not implemented; typeset as plain verbatim",
                        Some(options_span),
                        Some("ignored the options and typeset the body literally".into()),
                    ));
                }
            }
        }
        let document = open.document;
        let source = self.documents[document.0].text;
        // The newline right after `\begin{...}` is not part of the body.
        if source.as_bytes().get(content_start) == Some(&b'\n') {
            content_start += 1;
        }
        let end_tag = format!("\\end{{{name}}}");
        let (content_end, tag_end, found) = match source[content_start..].find(end_tag.as_str()) {
            Some(offset) => {
                let tag_start = content_start + offset;
                (tag_start, tag_start + end_tag.len(), true)
            }
            None => (source.len(), source.len(), false),
        };
        // The newline right before `\end{...}` is not part of the body either.
        let mut trimmed_end = content_end;
        if trimmed_end > content_start && source.as_bytes()[trimmed_end - 1] == b'\n' {
            trimmed_end -= 1;
        }
        let body = &source[content_start..trimmed_end];
        let mut lines = Vec::new();
        let mut line_start = content_start;
        for raw_line in body.split('\n') {
            lines.push(VerbatimLine {
                text: verbatim_display(raw_line, starred),
                span: Span::in_document(document, line_start, line_start + raw_line.len()),
            });
            line_start += raw_line.len() + 1;
        }
        if !found {
            self.diags.push(Diagnostic::error(
                format!("unterminated environment '{name}' — no matching \\end"),
                Some(open),
                Some("closed the verbatim block at end of input".into()),
            ));
        }
        while self.i < self.t.len()
            && self.t[self.i].token.span.document == document
            && self.t[self.i].token.span.start < tag_end
        {
            self.i += 1;
        }
        blocks.push(Block::Verbatim {
            lines,
            span: Span::in_document(document, open.start, tag_end),
        });
        self.finish_block_dependencies();
    }

    fn equation_environment(
        &mut self,
        open: Span,
        name: &str,
        blocks: &mut Vec<Block>,
        para: &mut Vec<Inline>,
    ) {
        self.flush_paragraph(blocks, para);
        let numbered = name == "equation";
        let number = if numbered {
            let number = self.counters.step("equation").unwrap_or_default();
            self.set_current_counter("equation", Some(number.clone()));
            number
        } else {
            self.counters.the("equation").unwrap_or_default()
        };
        let mut raw = Vec::new();
        let mut labels = Vec::new();
        let mut end = open.end;
        let mut found_end = false;

        while self.i < self.t.len() {
            if let Some((after, end_span)) = environment_end_at(&self.t, self.i, name) {
                self.i = after;
                end = end_span.end;
                found_end = true;
                break;
            }
            if paragraph_boundary_at(&self.t, self.i) {
                break;
            }
            if matches!(&self.t[self.i].token.kind, TokenKind::Command(name) if name == "label") {
                let label_span = self.t[self.i].token.span;
                self.i += 1;
                let (tokens, argument_span) = self.required_group("label", label_span);
                let key = token_text(&tokens).trim().to_string();
                self.document_global_state = true;
                if !key.is_empty() {
                    if self.seen_labels.insert(key.clone(), label_span).is_some() {
                        self.diags.push(Diagnostic::warning(
                            format!("duplicate \\label{{{key}}}; the second definition wins"),
                            Some(label_span.merge(argument_span)),
                            Some("replaced the earlier label definition".into()),
                        ));
                    }
                    labels.push(Inline::Label {
                        key,
                        value: number.clone(),
                        kind: "equation".into(),
                        span: label_span,
                    });
                }
                continue;
            }
            end = self.t[self.i].token.span.end;
            raw.push(self.t[self.i].token.clone());
            self.i += 1;
        }
        if !found_end {
            self.diags.push(Diagnostic::error(
                format!("unterminated environment '{name}' — no matching \\end"),
                Some(open),
                Some(
                    if self.i < self.t.len() {
                        "closed the equation at the end of the paragraph"
                    } else {
                        "closed the equation at end of input"
                    }
                    .into(),
                ),
            ));
        }
        let list = math::parse_tokens(&raw, self.math_packages, &mut self.diags);
        let color_ranges = self.math_color_ranges(&raw);
        para.push(Inline::Math {
            color: self.style.color,
            color_ranges,
            list,
            display: true,
            number: numbered.then_some(number),
            number_span: numbered.then_some(open),
            span: Span::in_document(open.document, open.start, end),
            // Always its own line (see `layout::LayoutCursor::display_math`),
            // so whether real source whitespace preceded it is moot.
            space_before: true,
        });
        para.extend(labels);
        self.flush_paragraph(blocks, para);
    }

    /// amsmath `gather`/`align` (and starred forms): rows split on top-level
    /// `\\`, `align` cells split on top-level `&`. Numbered forms number every
    /// row except those carrying `\nonumber`/`\notag`.
    fn multirow_environment(
        &mut self,
        open: Span,
        name: &str,
        blocks: &mut Vec<Block>,
        para: &mut Vec<Inline>,
    ) {
        self.flush_paragraph(blocks, para);
        let numbered = !name.ends_with('*');
        let aligned = name.starts_with("align") || name.starts_with("flalign");
        if name.starts_with("alignat") {
            // The column-pair count; cells are split on `&` regardless.
            let _ = self.required_group("alignat", open);
        }
        // Per row: (cells of raw tokens, unnumbered flag, labels, intertext
        // set before the row).
        type RawRow = (Vec<Vec<Token>>, bool, Vec<(String, Span)>, Vec<Intertext>);
        let mut rows: Vec<RawRow> = vec![(vec![Vec::new()], false, Vec::new(), Vec::new())];
        let mut depth = 0usize;
        let mut end = open.end;
        let mut found_end = false;

        while self.i < self.t.len() {
            if depth == 0 {
                if let Some((after, end_span)) = environment_end_at(&self.t, self.i, name) {
                    self.i = after;
                    end = end_span.end;
                    found_end = true;
                    break;
                }
            }
            if paragraph_boundary_at(&self.t, self.i) {
                break;
            }
            let token = self.t[self.i].token.clone();
            let row = rows.last_mut().expect("at least one row");
            match &token.kind {
                TokenKind::Command(command) if command == "label" => {
                    self.i += 1;
                    let (tokens, argument_span) = self.required_group("label", token.span);
                    let key = token_text(&tokens).trim().to_string();
                    if !key.is_empty() {
                        let row = rows.last_mut().expect("at least one row");
                        row.2.push((key, token.span.merge(argument_span)));
                    }
                    continue;
                }
                TokenKind::Command(command)
                    if depth == 0 && (command == "intertext" || command == "shortintertext") =>
                {
                    self.i += 1;
                    let (tokens, argument_span) = self.required_group(command, token.span);
                    let content = self.inlines_from_tokens(tokens, TextStyle::default());
                    // `\ifvmode\else\\\@empty\fi`: a row holding material is
                    // ended first; right after `\\` the text joins the next row.
                    let blank = |t: &Token| {
                        matches!(
                            t.kind,
                            TokenKind::Space | TokenKind::Comment | TokenKind::ParBreak
                        )
                    };
                    let started = {
                        let row = rows.last().expect("at least one row");
                        row.0.len() > 1 || row.0.iter().flatten().any(|t| !blank(t))
                    };
                    if started {
                        rows.push((vec![Vec::new()], false, Vec::new(), Vec::new()));
                    }
                    rows.last_mut()
                        .expect("at least one row")
                        .3
                        .push(Intertext {
                            content,
                            short: command == "shortintertext",
                            span: token.span.merge(argument_span),
                        });
                    continue;
                }
                TokenKind::Command(command) if command == "nonumber" || command == "notag" => {
                    row.1 = true;
                }
                TokenKind::LineBreak if depth == 0 => {
                    rows.push((vec![Vec::new()], false, Vec::new(), Vec::new()));
                }
                TokenKind::Word(word) if depth == 0 && word.contains('&') => {
                    let exact = token.span.end - token.span.start == word.len();
                    for (index, piece) in word.split('&').enumerate() {
                        if index > 0 {
                            row.0.push(Vec::new());
                        }
                        if piece.is_empty() {
                            continue;
                        }
                        let offset = piece.as_ptr() as usize - word.as_ptr() as usize;
                        let span = if exact {
                            Span::in_document(
                                token.span.document,
                                token.span.start + offset,
                                token.span.start + offset + piece.len(),
                            )
                        } else {
                            token.span
                        };
                        row.0.last_mut().expect("at least one cell").push(Token {
                            kind: TokenKind::Word(piece.to_string()),
                            span,
                        });
                    }
                }
                _ => {
                    // Nested groups and environments (`cases`, `pmatrix`)
                    // own their `\\` and `&`.
                    match &token.kind {
                        TokenKind::LBrace => depth += 1,
                        TokenKind::Command(command) if command == "begin" => depth += 1,
                        TokenKind::RBrace => depth = depth.saturating_sub(1),
                        TokenKind::Command(command) if command == "end" => {
                            depth = depth.saturating_sub(1)
                        }
                        _ => {}
                    }
                    row.0
                        .last_mut()
                        .expect("at least one cell")
                        .push(token.clone());
                }
            }
            end = token.span.end;
            self.i += 1;
        }
        if !found_end {
            self.diags.push(Diagnostic::error(
                format!("unterminated environment '{name}' — no matching \\end"),
                Some(open),
                Some(
                    if self.i < self.t.len() {
                        "closed the display at the end of the paragraph"
                    } else {
                        "closed the display at end of input"
                    }
                    .into(),
                ),
            ));
        }
        // A trailing `\\` before `\end` does not start a real row.
        if rows.len() > 1
            && rows.last().is_some_and(|(cells, _, labels, intertext)| {
                labels.is_empty()
                    && intertext.is_empty()
                    && cells.iter().flatten().all(|t| {
                        matches!(
                            t.kind,
                            TokenKind::Space | TokenKind::Comment | TokenKind::ParBreak
                        )
                    })
            })
        {
            rows.pop();
        }
        if name == "multline" {
            // One multline display carries a single number, on its last line.
            let last = rows.len().saturating_sub(1);
            for (index, row) in rows.iter_mut().enumerate() {
                row.1 |= index != last;
            }
        }

        let mut math_rows = Vec::new();
        let mut labels = Vec::new();
        for (cells, unnumbered, row_labels, intertext) in rows {
            let span = cells
                .iter()
                .flatten()
                .map(|t| t.span)
                .reduce(Span::merge)
                .unwrap_or(open);
            let number = (numbered && !unnumbered).then(|| {
                let number = self.counters.step("equation").unwrap_or_default();
                self.set_current_counter("equation", Some(number.clone()));
                number
            });
            for (key, label_span) in row_labels {
                self.document_global_state = true;
                if self.seen_labels.insert(key.clone(), label_span).is_some() {
                    self.diags.push(Diagnostic::warning(
                        format!("duplicate \\label{{{key}}}; the second definition wins"),
                        Some(label_span),
                        Some("replaced the earlier label definition".into()),
                    ));
                }
                labels.push(Inline::Label {
                    key,
                    value: number
                        .clone()
                        .unwrap_or_else(|| self.counters.the("equation").unwrap_or_default()),
                    kind: "equation".into(),
                    span: label_span,
                });
            }
            let packages = self.math_packages;
            let cells = cells
                .iter()
                .map(|cell| math::parse_tokens(cell, packages, &mut self.diags))
                .collect();
            math_rows.push(MathRow {
                cells,
                number,
                span,
                intertext,
            });
        }
        para.push(Inline::MathRows {
            rows: math_rows,
            aligned,
            span: Span::in_document(open.document, open.start, end),
        });
        para.extend(labels);
        self.flush_paragraph(blocks, para);
    }

    fn dollar_math(&mut self, open: Span, para: &mut Vec<Inline>) {
        let space_before = self.space_precedes(self.i);
        self.i += 1;
        let display = matches!(self.peek().map(|t| &t.kind), Some(TokenKind::MathShift));
        if display {
            self.i += 1;
        }
        let content_start = self.i;
        let mut content_end = self.t.len();
        let mut close_end = open.end;
        let mut found = false;
        while self.i < self.t.len() {
            // Unterminated math ends with its paragraph (TeX: "Missing $
            // inserted"), never at a `$` pages later.
            if paragraph_boundary_at(&self.t, self.i) {
                content_end = self.i;
                break;
            }
            if self.t[self.i].token.kind == TokenKind::MathShift {
                let closes = !display
                    || self.t.get(self.i + 1).map(|t| &t.token.kind) == Some(&TokenKind::MathShift);
                if closes {
                    content_end = self.i;
                    close_end = if display {
                        self.t[self.i + 1].token.span.end
                    } else {
                        self.t[self.i].token.span.end
                    };
                    self.i += if display { 2 } else { 1 };
                    found = true;
                    break;
                }
            }
            self.i += 1;
        }
        self.finish_math(
            open,
            content_start,
            content_end,
            close_end,
            found,
            display,
            space_before,
            para,
        );
    }

    /// `\(...\)`: LaTeX's inline math, the `$...$` rules with the
    /// robust delimiters (an unterminated one ends with its paragraph too).
    fn paren_math(&mut self, open: Span, para: &mut Vec<Inline>) {
        let space_before = self.space_precedes(self.i);
        self.i += 1;
        let content_start = self.i;
        while self.i < self.t.len() {
            if self.t[self.i].token.kind == TokenKind::InlineMathClose
                || paragraph_boundary_at(&self.t, self.i)
            {
                break;
            }
            self.i += 1;
        }
        let content_end = self.i;
        let found = matches!(
            self.t.get(self.i).map(|input| &input.token.kind),
            Some(TokenKind::InlineMathClose)
        );
        let close_end = if found {
            let end = self.t[self.i].token.span.end;
            self.i += 1;
            end
        } else {
            open.end
        };
        self.finish_math(
            open,
            content_start,
            content_end,
            close_end,
            found,
            false,
            space_before,
            para,
        );
    }

    fn bracket_math(&mut self, open: Span, para: &mut Vec<Inline>) {
        let space_before = self.space_precedes(self.i);
        self.i += 1;
        let content_start = self.i;
        while self.i < self.t.len() {
            if self.t[self.i].token.kind == TokenKind::DisplayMathClose
                || paragraph_boundary_at(&self.t, self.i)
            {
                break;
            }
            self.i += 1;
        }
        let content_end = self.i;
        let found = matches!(
            self.t.get(self.i).map(|input| &input.token.kind),
            Some(TokenKind::DisplayMathClose)
        );
        let close_end = if found {
            let end = self.t[self.i].token.span.end;
            self.i += 1;
            end
        } else {
            open.end
        };
        self.finish_math(
            open,
            content_start,
            content_end,
            close_end,
            found,
            true,
            space_before,
            para,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_math(
        &mut self,
        open: Span,
        content_start: usize,
        content_end: usize,
        close_end: usize,
        found: bool,
        display: bool,
        space_before: bool,
        para: &mut Vec<Inline>,
    ) {
        // Unterminated math inside an expansion can report a content end past the
        // token stream: the closing token the caller expected was never produced.
        // Clamp rather than slice out of range — the diagnostic for the unclosed
        // construct is emitted by the caller either way.
        let content_start = content_start.min(self.t.len());
        let content_end = content_end.clamp(content_start, self.t.len());
        let mut raw = Vec::new();
        for input in &self.t[content_start..content_end] {
            if input.maps_to_invocation {
                if let TokenKind::Word(word) = &input.token.kind {
                    for ch in word.chars() {
                        raw.push(Token {
                            kind: TokenKind::Word(ch.to_string()),
                            span: input.token.span,
                        });
                    }
                    continue;
                }
            }
            raw.push(input.token.clone());
        }
        let (list, unclosed) = math::parse_tokens_reporting_unclosed(
            &raw,
            self.math_packages,
            &mut self.diags,
            !found,
        );
        let end = if found {
            close_end
        } else {
            raw.last().map_or(open.end, |t| t.span.end)
        };
        match (found, unclosed) {
            (true, Some(group)) => self.diags.push(Diagnostic::error(
                "math group is missing its closing brace",
                Some(group),
                Some("closed the group at the math delimiter".into()),
            )
            .with_help("add a closing '}'")),
            // One primary diagnostic at the innermost opener: closing it is
            // the next thing the author has to type.
            (false, Some(group)) => self.diags.push(Diagnostic::error(
                "'{' opened here is not closed before the end of the paragraph",
                Some(group),
                Some(
                    if display {
                        "closed the group and the display math at the end of the paragraph"
                    } else {
                        "closed the group and the inline math at the end of the paragraph"
                    }
                    .into(),
                ),
            )),
            (false, None) => self.diags.push(Diagnostic::error(
                if display {
                    "display math is missing its closing delimiter"
                } else {
                    "inline math is missing its closing '$'"
                },
                Some(open),
                Some(
                    "closed math mode at the end of the paragraph and typeset its contents".into(),
                ),
            )
            .with_help(if display {
                "add a closing \\] or $$ to end the display"
            } else {
                "add a closing '$' to end the formula"
            })
            .with_label(open, "math starts here", true)),
            (true, None) => {}
        }
        // `\[...\]` and `$$...$$` are unnumbered displays in LaTeX: they never
        // print a number or advance the equation counter.
        let color_ranges = self.math_color_ranges(&raw);
        para.push(Inline::Math {
            color: self.style.color,
            color_ranges,
            list,
            display,
            number: None,
            number_span: None,
            span: Span::in_document(open.document, open.start, end),
            space_before,
        });
    }

    /// A non-`\long` argument: like TeX, it cannot run past the end of the
    /// paragraph, so an unclosed one is closed there.
    fn required_group(&mut self, command: &str, command_span: Span) -> (Vec<InputToken>, Span) {
        self.required_group_bounded(command, command_span, false)
    }

    /// `long`: a `\long` argument (`\@footnotetext`), where a blank line is
    /// an ordinary paragraph break inside the argument rather than its end.
    /// An argument that is never closed at all is still closed at the end of
    /// its first paragraph, so a missing brace cannot swallow the document.

    fn required_group_bounded(
        &mut self,
        command: &str,
        command_span: Span,
        long: bool,
    ) -> (Vec<InputToken>, Span) {
        self.skip_spaces();
        let open = match self.peek() {
            Some(token) if token.kind == TokenKind::LBrace => token.span,
            _ => {
                self.diags.push(Diagnostic::error(
                    format!("\\{} requires a braced argument", command),
                    Some(command_span),
                    Some("used an empty argument and continued".into()),
                ));
                return (Vec::new(), command_span);
            }
        };
        self.i += 1;
        let start = self.i;
        let mut depth = 1usize;
        let mut end = open.end;
        let mut boundary = None;
        while self.i < self.t.len() {
            if boundary.is_none() && paragraph_boundary_at(&self.t, self.i) {
                boundary = Some((self.i, end));
                if !long {
                    break;
                }
            }
            let token = &self.t[self.i].token;
            match token.kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        end = token.span.end;
                        let content = self.t[start..self.i].to_vec();
                        self.i += 1;
                        return (content, Span::in_document(open.document, open.start, end));
                    }
                }
                _ => {}
            }
            end = token.span.end;
            self.i += 1;
        }
        let (stop, recovery) = match boundary {
            Some((index, before)) => {
                end = before;
                (index, "closed the argument at the end of the paragraph")
            }
            None => (self.t.len(), "closed the argument at end of input"),
        };
        self.i = stop;
        self.diags.push(Diagnostic::error(
            format!("argument to \\{} is missing its closing brace", command),
            Some(open),
            Some(recovery.into()),
        )
        .with_help("add a closing '}'"));
        (
            self.t[start..stop].to_vec(),
            Span::in_document(open.document, open.start, end),
        )
    }

    /// Reads a `\url`/`\nolinkurl`/`\href` URL argument as literal source
    /// text, bypassing the ordinary token stream.
    ///
    /// Real `url.sty` works by temporarily changing category codes so `%
    /// # _ ~ &` — otherwise a comment, a macro-parameter marker, a math
    /// subscript, and (not modelled here) an active tie and alignment tab —
    /// read as plain "other" characters for the duration of the argument.
    /// This compiler tokenizes the whole document once up front (see
    /// `lexer.rs`), with no notion of a mid-document catcode change, so the
    /// same effect is reached by re-reading the exact source bytes between
    /// the braces directly instead of trusting the tokens already produced
    /// for that range. The expansion pass (`crate::expansion`) blanks those
    /// bytes before any tokenizing, so a literal `%` inside the URL can no
    /// longer start a comment that swallows the closing brace.
    ///
    /// Only `\{` and `\}` are recognised as escapes, for a literal brace
    /// inside the URL; an unescaped `{`/`}` still opens/closes a nested
    /// group that does not end the argument, matching `required_group`'s
    /// own depth balancing so a URL is never truncated by a brace it
    /// happens to contain.
    fn url_argument(&mut self, command: &str, command_span: Span) -> (String, Span) {
        self.skip_spaces();
        // A URL group produced by macro expansion has no source bytes of its
        // own to re-read; take its (already expanded) tokens.
        if self
            .t
            .get(self.i)
            .is_some_and(|input| input.maps_to_invocation && input.token.kind == TokenKind::LBrace)
        {
            let (tokens, span) = self.required_group(command, command_span);
            return (token_text(&tokens), span);
        }
        let open = match self.peek() {
            Some(token) if token.kind == TokenKind::LBrace => token.span,
            _ => {
                self.diags.push(Diagnostic::error(
                    format!("\\{command} requires a braced argument"),
                    Some(command_span),
                    Some("used an empty argument and continued".into()),
                ));
                return (String::new(), command_span);
            }
        };
        let document = open.document;
        let source = self.documents[document.0].text;
        let mut depth = 1usize;
        let mut content = String::new();
        let mut pos = open.end;
        let close_end = loop {
            let Some(ch) = source[pos..].chars().next() else {
                self.diags.push(Diagnostic::error(
                    format!("argument to \\{command} is missing its closing brace"),
                    Some(open),
                    Some("closed the argument at end of input".into()),
                )
                .with_help("add a closing '}'"));
                break pos;
            };
            let ch_len = ch.len_utf8();
            match ch {
                '\\' if matches!(source[pos + ch_len..].chars().next(), Some('{' | '}')) => {
                    let escaped = source[pos + ch_len..].chars().next().unwrap();
                    content.push(escaped);
                    pos += ch_len + escaped.len_utf8();
                }
                '{' => {
                    depth += 1;
                    content.push(ch);
                    pos += ch_len;
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        break pos + ch_len;
                    }
                    content.push(ch);
                    pos += ch_len;
                }
                _ => {
                    content.push(ch);
                    pos += ch_len;
                }
            }
        };
        let span = Span::in_document(document, open.start, close_end);
        self.resync_after_raw_group(document, close_end);
        (content, span)
    }

    /// Moves past every token a raw-scanned group (see `url_argument`)
    /// covered. The expansion pass blanked that group's bytes before
    /// expansion, so nothing inside it was interpreted and the tokens after
    /// its closing brace are already correct; only the group's own tokens are
    /// skipped (re-tokenizing the source here would discard every later macro
    /// expansion).
    fn resync_after_raw_group(&mut self, document: DocumentId, close_end: usize) {
        while let Some(input) = self.t.get(self.i) {
            if input.maps_to_invocation
                || input.token.span.document != document
                || input.token.span.start >= close_end
            {
                break;
            }
            self.i += 1;
        }
    }

    /// Pushes literal `\url`/`\nolinkurl` text as one or more `Inline::Text`
    /// runs (see `url_pieces`), so the layout can wrap a long URL at a
    /// `URL_BREAK_AFTER` character without ever inserting a hyphen, with the
    /// 0.5pt `URL_HYPHEN_KERN_PT` url.sty puts after each hyphen.
    fn push_url_text(
        &mut self,
        text: &str,
        span: Span,
        space_before: bool,
        para: &mut Vec<Inline>,
    ) {
        if text.is_empty() {
            return;
        }
        let style = apply_style(self.style, "ttfamily");
        for (index, piece) in url_pieces(text).into_iter().enumerate() {
            match piece {
                UrlPiece::Run(run) => para.push(Inline::Text {
                    text: run.to_string(),
                    span,
                    style,
                    space_before: index == 0 && space_before,
                }),
                UrlPiece::HyphenKern => para.push(Inline::Kern {
                    amount: crate::text_builtins::TextDimen {
                        negative: false,
                        integer: 0,
                        frac: vec![URL_HYPHEN_KERN_PT],
                        unit: crate::text_builtins::DimenUnit::Physical(
                            crate::text_builtins::PhysicalUnit::Pt,
                        ),
                    },
                    span,
                    style,
                }),
            }
        }
    }

    /// `\hypersetup{key=value,...}`: reads the key list and typesets
    /// nothing. A key outside `hyperref_option_is_layout_neutral` is
    /// reported once, because that is the set whose neutrality was actually
    /// measured against pdflatex; an unlisted key may well be neutral too,
    /// but this compiler has not checked it and will not say that it has.
    fn hypersetup(&mut self, span: Span) {
        let (tokens, argument_span) = self.required_group("hypersetup", span);
        let keys = token_text(&tokens);
        let unchecked: Vec<&str> = keys
            .split(',')
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .filter(|key| !hyperref_option_is_layout_neutral(key))
            .map(|key| key.split_once('=').map_or(key, |(name, _)| name).trim())
            .collect();
        if unchecked.is_empty() || self.noted_hypersetup_keys {
            return;
        }
        self.noted_hypersetup_keys = true;
        self.diags.push(Diagnostic::warning(
            format!(
                "\\hypersetup keys {} are not modelled by this compiler",
                unchecked.join(", ")
            ),
            Some(span.merge(argument_span)),
            Some("read the key list and typeset nothing for it".into()),
        ));
    }

    /// `\lstset{key=value,...}` (listings v1.10c): reads the key list and
    /// typesets nothing. A name outside [`listings_key_is_known`] is
    /// reported once — that list is listings' own documented keys, and a
    /// name this compiler has never heard of is far more likely a typo than
    /// a key it silently honours.
    ///
    /// The values are deliberately not interpreted here. This compiler's own
    /// layout sets an `lstlisting` as plain verbatim lines and applies none
    /// of them, and saying otherwise in a diagnostic would be a claim it has
    /// not earned; `crates/render-pipeline`'s `listings` module reads the
    /// same keys from the source bytes and applies the geometric ones.
    fn lstset(&mut self, span: Span) {
        let (tokens, argument_span) = self.required_group("lstset", span);
        let keys = token_text(&tokens);
        let unknown: Vec<String> = listings_key_names(&keys)
            .into_iter()
            .filter(|key| !listings_key_is_known(key))
            .collect();
        if unknown.is_empty() || self.noted_lstset_keys {
            return;
        }
        self.noted_lstset_keys = true;
        self.diags.push(Diagnostic::warning(
            format!("\\lstset keys {} are not listings keys", unknown.join(", ")),
            Some(span.merge(argument_span)),
            Some("read the key list and typeset nothing for it".into()),
        ));
    }

    /// Emits the one honest "links are not clickable yet" diagnostic the
    /// first time `\url`/`\href` is used in this document (see
    /// `noted_unclickable_link`): `docs/contracts/runtime-v1.md` has no link
    /// or annotation item, so a real hyperlink cannot be produced yet, but
    /// the URL/text itself is still typeset faithfully.
    fn note_links_unclickable(&mut self, span: Span) {
        if self.noted_unclickable_link {
            return;
        }
        self.noted_unclickable_link = true;
        self.diags.push(Diagnostic::warning(
            "links are not clickable in the preview/PDF yet",
            Some(span),
            Some("typeset the link text without an active hyperlink annotation".into()),
        ));
    }

    /// Brackets stay ordinary lexer word characters, preserving normal text.
    fn optional_bracket_argument(&mut self) -> Option<(String, Span)> {
        self.skip_spaces();
        let first = self.peek()?;
        let TokenKind::Word(first_word) = &first.kind else {
            return None;
        };
        if !first_word.starts_with('[') {
            return None;
        }
        let start = first.span.start;
        let document = first.span.document;
        let mut end = first.span.end;
        let mut found = first_word.contains(']');
        let mut raw = first_word.clone();
        self.i += 1;
        while !found && self.i < self.t.len() {
            let token = &self.t[self.i].token;
            end = token.span.end;
            match &token.kind {
                TokenKind::Word(word) => {
                    raw.push_str(word);
                    found = word.contains(']');
                }
                TokenKind::Space | TokenKind::ParBreak => raw.push(' '),
                TokenKind::Command(name) => {
                    raw.push('\\');
                    raw.push_str(name);
                }
                _ => {}
            }
            self.i += 1;
        }
        let span = Span::in_document(document, start, end);
        let content = raw
            .strip_prefix('[')
            .unwrap_or(&raw)
            .split_once(']')
            .map_or(raw.as_str(), |(inside, _)| inside)
            .to_string();
        if !found {
            self.diags.push(Diagnostic::error(
                "optional argument is missing its closing ']'",
                Some(span),
                Some("used the text through end of input as the option".into()),
            )
            .with_help("add a closing ']'"));
        }
        Some((content, span))
    }

    /// `\pagebreak[n]`/`\linebreak[n]`'s priority argument: real TeX's `n`
    /// (0-4) only ever hints a badness-based breaking algorithm this greedy
    /// layout does not implement. An absent bracket defaults, as in real
    /// TeX, to `4` — "you must break here" — which this layout can honour
    /// exactly as a forced break; any other value is honestly left alone
    /// rather than guessing whether a real engine would have broken there.
    /// The bracket, present or not, is always consumed.
    fn mandatory_break_requested(&mut self) -> bool {
        match self.optional_bracket_argument() {
            None => true,
            Some((content, _)) => content.trim() == "4",
        }
    }

    /// `\includegraphics*[<keys>]{<file>}`, or graphics.sty's
    /// `[<llx>,<lly>][<urx>,<ury>]{<file>}` bounding-box form (graphicx.sty
    /// `\Gin@ii` hands two brackets to `\Gin@iii`, which pdftex.def replaces
    /// by `\Gin@iii@vp`: a viewport).
    fn include_graphics(&mut self, span: Span, para: &mut Vec<Inline>) {
        let space_before = self.space_precedes(self.i - 1);
        let starred = self.take_star_prefix();
        let mut options = self.bracket_argument().unwrap_or_default();
        if !options.is_empty() || self.bracket_follows() {
            if let Some(upper) = self.bracket_argument() {
                let corner = |s: &str| s.replace(',', " ").split_whitespace().collect::<Vec<_>>().join(" ");
                options = format!("viewport={} {}", corner(&options), corner(&upper));
            }
        }
        let (tokens, argument) = self.required_group("includegraphics", span);
        let path = self.argument_text(&tokens, argument);
        para.push(Inline::Graphic(Box::new(crate::graphics::Graphic {
            starred,
            options,
            path,
            span: span.merge(argument),
            space_before,
        })));
    }

    /// `\scalebox{x}[y]{..}`, `\resizebox*{w}{h}{..}`,
    /// `\rotatebox[keys]{angle}{..}`, `\reflectbox{..}`: the parameters as
    /// written, then the content parsed as horizontal material in the
    /// current style.
    fn transform_box(&mut self, name: &str, span: Span, para: &mut Vec<Inline>) {
        use crate::graphics::TransformKind;
        let space_before = self.space_precedes(self.i - 1);
        let kind = match name {
            "scalebox" => {
                let (tokens, argument) = self.required_group(name, span);
                let x = self.argument_text(&tokens, argument);
                let y = self.bracket_argument();
                TransformKind::Scale { x, y }
            }
            "resizebox" => {
                let starred = self.take_optional_star();
                let (tokens, argument) = self.required_group(name, span);
                let width = self.argument_text(&tokens, argument);
                let (tokens, argument) = self.required_group(name, span);
                let height = self.argument_text(&tokens, argument);
                TransformKind::Resize { starred, width, height }
            }
            "rotatebox" => {
                let options = self.bracket_argument();
                let (tokens, argument) = self.required_group(name, span);
                let angle = self.argument_text(&tokens, argument);
                TransformKind::Rotate { options, angle }
            }
            _ => TransformKind::Reflect,
        };
        let (tokens, argument) = self.required_group(name, span);
        let style = self.style;
        let content = self.argument_inlines(tokens, span, style);
        para.push(Inline::Transform(Box::new(crate::graphics::TransformBox {
            kind,
            content,
            span: span.merge(argument),
            space_before,
        })));
    }

    /// A `*` after a command, alone or glued to the word that follows it
    /// (`\includegraphics*[..]` lexes as one word `*[..]`).
    fn take_star_prefix(&mut self) -> bool {
        if self.take_optional_star() {
            return true;
        }
        let Some(input) = self.token_mut(self.i) else {
            return false;
        };
        let TokenKind::Word(word) = &input.token.kind else {
            return false;
        };
        let Some(rest) = word.strip_prefix('*') else {
            return false;
        };
        let rest = rest.to_string();
        let span = input.token.span;
        if span.end - span.start == word.len() {
            input.token.span = Span::in_document(span.document, span.start + 1, span.end);
        }
        input.token.kind = TokenKind::Word(rest);
        true
    }

    /// Whether the next non-space token starts a `[..]` argument.
    fn bracket_follows(&self) -> bool {
        self.t[self.i..]
            .iter()
            .find(|input| !matches!(input.token.kind, TokenKind::Space | TokenKind::Comment))
            .is_some_and(|input| matches!(&input.token.kind, TokenKind::Word(w) if w.starts_with('[')))
    }

    /// An optional `[..]` argument's exact source text (braces kept), or
    /// its token reconstruction when it came from a macro expansion.
    fn bracket_argument(&mut self) -> Option<String> {
        if !self.bracket_follows() {
            return None;
        }
        self.skip_spaces();
        // `[a][b]` or `[a]text` lexes as one word: take the first bracket and
        // leave the rest of the word in place.
        if let Some(input) = self.token_mut(self.i) {
            if let TokenKind::Word(word) = &input.token.kind {
                if let Some(close) = word.find(']').filter(|&c| c + 1 < word.len() && !word[..c].contains('{')) {
                    let content = word[1..close].trim().to_string();
                    let rest = word[close + 1..].to_string();
                    let span = input.token.span;
                    if span.end - span.start == word.len() {
                        input.token.span = Span::in_document(span.document, span.start + close + 1, span.end);
                    }
                    input.token.kind = TokenKind::Word(rest);
                    return Some(content);
                }
            }
        }
        let from_source = !self.t[self.i..]
            .iter()
            .find(|input| !matches!(input.token.kind, TokenKind::Space | TokenKind::Comment))
            .is_some_and(|input| input.maps_to_invocation);
        let (content, span) = self.optional_bracket_argument()?;
        if from_source {
            if let Some(inner) = self
                .documents
                .get(span.document.0)
                .and_then(|document| bracket_inner(document.text, span.start))
            {
                return Some(inner.trim().to_string());
            }
        }
        Some(content.trim().to_string())
    }

    /// A braced argument's exact source text (without the outer braces) when
    /// its tokens come from the source, else a reconstruction of the tokens.
    fn argument_text(&self, tokens: &[InputToken], outer: Span) -> String {
        if tokens.iter().all(|input| !input.maps_to_invocation) {
            if let Some(text) = self.documents.get(outer.document.0).map(|document| document.text) {
                let end = if text.as_bytes().get(outer.end.wrapping_sub(1)) == Some(&b'}') {
                    outer.end - 1
                } else {
                    outer.end
                };
                if let Some(inner) = text.get(outer.start + 1..end) {
                    return inner.trim().to_string();
                }
            }
        }
        let mut out = String::new();
        for input in tokens {
            match &input.token.kind {
                TokenKind::Word(word) => out.push_str(word),
                TokenKind::Command(name) => {
                    out.push('\\');
                    out.push_str(name);
                }
                TokenKind::Space | TokenKind::ParBreak => out.push(' '),
                TokenKind::LBrace => out.push('{'),
                TokenKind::RBrace => out.push('}'),
                _ => {}
            }
        }
        out.trim().to_string()
    }

    fn take_optional_star(&mut self) -> bool {
        self.skip_spaces();
        if matches!(
            self.peek().map(|token| &token.kind),
            Some(TokenKind::Word(word)) if word == "*"
        ) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    /// The `{` span when the next token opens a group that closes in this
    /// token stream. Unclosed arguments keep `required_group`'s diagnostics.
    fn closed_group_start(&self) -> Option<Span> {
        let open = self
            .peek()
            .filter(|token| token.kind == TokenKind::LBrace)?;
        let mut depth = 0usize;
        for input in &self.t[self.i..] {
            match input.token.kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(open.span);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn open_group(&mut self, span: Span) {
        self.brace_stack.push(span);
        self.style_stack.push(self.style);
        self.alignment_stack.push(self.declared_alignment);
    }

    fn inlines_from_tokens(&mut self, tokens: Vec<InputToken>, base: TextStyle) -> Vec<Inline> {
        let outer_tokens = std::mem::replace(&mut self.t, std::rc::Rc::new(tokens));
        let outer_index = std::mem::replace(&mut self.i, 0);
        let mut expanded = Vec::new();
        while self.i < self.t.len() {
            expanded.push(self.t[self.i].clone());
            self.i += 1;
        }
        self.t = outer_tokens;
        self.i = outer_index;

        let mut content = Vec::new();
        let mut style = base;
        let mut saved = Vec::new();
        let mut pending = None;
        // Tokens already read as a siunitx command's arguments.
        let mut skip_until = 0usize;
        for (index, input) in expanded.iter().enumerate() {
            if index < skip_until {
                continue;
            }
            let space_before = preceded_by_space(&expanded, index);
            match &input.token.kind {
                TokenKind::Command(name) if name == "color" || name == "textcolor" => {
                    let (next, color) = self.flat_color(&expanded, index, style.color);
                    skip_until = next;
                    match color {
                        Some(color) if name == "color" => style.color = Some(color),
                        Some(color) => pending = Some(TextStyle { color: Some(color), ..style }),
                        None => {}
                    }
                }
                // siunitx in a heading, caption or style argument: the same
                // formula as in running text (`P::siunitx`).
                TokenKind::Command(name) if siunitx::arity(name).is_some() => {
                    let (required, pre_unit_bracket) = siunitx::arity(name).unwrap_or((0, false));
                    let mut next = index + 1;
                    let mut span = input.token.span;
                    let widen = |span: &mut Span, other: Span| {
                        if other.document == span.document {
                            *span = span.merge(other);
                        }
                    };
                    let options = siunitx_bracket_at(&expanded, next).map(|(raw, s, after)| {
                        next = after;
                        widen(&mut span, s);
                        raw
                    });
                    let mut pre_unit = None;
                    let mut args = Vec::with_capacity(required);
                    for argument in 0..required {
                        if pre_unit_bracket && argument == 1 {
                            if let Some((raw, s, after)) = siunitx_bracket_at(&expanded, next) {
                                next = after;
                                widen(&mut span, s);
                                pre_unit = Some(raw);
                            }
                        }
                        match siunitx_group_at(&expanded, next) {
                            Some((raw, s, after)) => {
                                next = after;
                                widen(&mut span, s);
                                args.push(raw);
                            }
                            None => {
                                self.diags.push(Diagnostic::error(
                                    format!("\\{name} requires an argument"),
                                    Some(input.token.span),
                                    Some("used an empty argument and continued".into()),
                                ));
                                args.push(String::new());
                            }
                        }
                    }
                    skip_until = next;
                    let atoms = siunitx::typeset(
                        name,
                        options.as_deref(),
                        pre_unit.as_deref(),
                        &args,
                        false,
                        self.math_packages,
                        span,
                        &mut self.diags,
                    );
                    if !atoms.is_empty() {
                        content.push(Inline::Math {
                            color: style.color,
                            color_ranges: Vec::new(),
                            list: MathList { atoms },
                            display: false,
                            number: None,
                            number_span: None,
                            span,
                            space_before,
                        });
                    }
                }
                TokenKind::Command(name) if style_command(name) => {
                    pending = Some(apply_style(style, name));
                }
                TokenKind::Command(name) if style_declaration(name) => {
                    style = apply_style(style, name);
                }
                TokenKind::LBrace => {
                    saved.push(style);
                    if let Some(next) = pending.take() {
                        style = next;
                    }
                }
                TokenKind::RBrace => {
                    if let Some(previous) = saved.pop() {
                        style = previous;
                    }
                }
                TokenKind::Word(text) if control_symbol_kern(text, input.token.span, self.math_packages.amsmath).is_some() => {
                    if let Some(amount) = control_symbol_kern(text, input.token.span, self.math_packages.amsmath) {
                        content.push(Inline::Kern {
                            amount,
                            span: input.token.span,
                            style,
                        });
                    }
                }
                TokenKind::Command(name)
                    if text_builtins::text_kern(name, self.math_packages.amsmath).is_some() =>
                {
                    if let Some(amount) =
                        text_builtins::text_kern(name, self.math_packages.amsmath)
                    {
                        content.push(Inline::Kern {
                            amount,
                            span: input.token.span,
                            style,
                        });
                    }
                }
                TokenKind::Word(text) => content.push(Inline::Text {
                    text: apply_text_ligatures(text),
                    span: input.token.span,
                    style,
                    space_before,
                }),
                TokenKind::LineBreak => content.push(Inline::LineBreak {
                    span: input.token.span,
                    skip_pt: None,
                }),
                // `\hfill`/`\hfil` take no argument, so — unlike `\hspace`,
                // which needs a following brace group this flat,
                // one-token-at-a-time pass has no way to consume — they fit
                // here directly. This is what makes `\problem`-style macro
                // bodies like `\subsection*{Problem #1 \hfill [#2 points]}`
                // (see the `problem_style_macro...` test below) right-flush:
                // heading/caption/`\textbf`-style content all reach the page
                // through this function rather than through `command`'s
                // ordinary dispatch. General nested-command dispatch inside
                // that content remains out of scope, per the module doc
                // comment.
                TokenKind::Command(name) if name == "hfill" || name == "hfil" => {
                    content.push(Inline::HFill {
                        span: input.token.span,
                        leader: FillLeader::None,
                    })
                }
                TokenKind::Command(name) if name == "hrulefill" || name == "dotfill" => {
                    content.push(Inline::HFill {
                        span: input.token.span,
                        leader: if name == "hrulefill" { FillLeader::Rule } else { FillLeader::Dots },
                    })
                }
                TokenKind::Verb { text, starred, .. } => content.push(Inline::Verbatim {
                    text: verbatim_display(text, *starred),
                    span: input.token.span,
                    space_before,
                }),
                TokenKind::Command(name)
                    if text_builtins::TEXT_SYMBOLS.iter().any(|(n, _)| n == name) =>
                {
                    if let Some(inline) =
                        self.symbol_inline(name, input.token.span, style, space_before)
                    {
                        content.push(inline);
                    }
                }
                TokenKind::Command(name) if TextLogo::from_command(name).is_some() => {
                    if let Some(logo) = TextLogo::from_command(name) {
                        content.push(Inline::Logo {
                            logo,
                            span: input.token.span,
                            style,
                            space_before,
                        });
                    }
                }
                // The request's date (`ParseOptions::today`), not the wall
                // clock: see `crate::date`.
                TokenKind::Command(name) if name == "today" => content.push(Inline::Text {
                    text: self.today.latex_today(),
                    span: input.token.span,
                    style,
                    space_before,
                }),
                _ => {}
            }
        }
        content
    }

    /// A kernel text symbol (`\AA`, `\ss`, `\S`, ...) under the current font
    /// encoding; see `text_builtins::text_symbol`.
    fn symbol_inline(
        &mut self,
        name: &str,
        span: Span,
        style: TextStyle,
        space_before: bool,
    ) -> Option<Inline> {
        let text = match text_builtins::text_symbol(name, self.font_encoding)? {
            SymbolOutcome::Char(ch) => ch.to_string(),
            SymbolOutcome::Text(text) => text,
            SymbolOutcome::Unavailable(message) => {
                self.diags.push(Diagnostic::error(
                    message,
                    Some(span),
                    Some(
                        "typeset nothing for the command, as pdfLaTeX does after this error".into(),
                    ),
                ));
                return None;
            }
        };
        Some(Inline::Text {
            text,
            span,
            style,
            space_before,
        })
    }

    /// A kernel text accent (`text_builtins::TEXT_ACCENTS`): `\c{c}`,
    /// `\v{\i}`, `\k{}`, or unbraced `\v s`, where TeX reads one token so
    /// `\v sice` accents only the `s`. One text inline spans the command and
    /// its argument and holds the character the dfu tables declare for it.
    fn text_accent(&mut self, name: &str, span: Span, para: &mut Vec<Inline>) {
        let space_before = self.space_precedes(self.i - 1);
        let style = self.style;
        self.skip_spaces();
        let dotless = |kind: Option<&TokenKind>| match kind {
            Some(TokenKind::Command(c)) if c == "i" || c == "j" => Some(format!("\\{c}")),
            _ => None,
        };
        fn kind_at<'a>(p: &'a P<'_>, j: usize) -> Option<&'a TokenKind> {
            p.t.get(j).map(|t| &t.token.kind)
        }
        // A macro's argument can come from another document than its body.
        let join = |a: Span, b: Span| if a.document == b.document { a.merge(b) } else { a };
        let (base, full) = match kind_at(self, self.i) {
            Some(TokenKind::LBrace) => {
                let j = self.i + 1;
                let (base, mut close) = match kind_at(self, j) {
                    Some(TokenKind::RBrace) => (String::new(), j),
                    Some(TokenKind::Word(w)) if w.chars().count() == 1 => (w.clone(), j + 1),
                    kind => match dotless(kind) {
                        Some(base) => (base, j + 1),
                        None => (String::new(), usize::MAX),
                    },
                };
                if close != usize::MAX
                    && close != j
                    && matches!(kind_at(self, close), Some(TokenKind::Space))
                {
                    close += 1;
                }
                if close == usize::MAX || !matches!(kind_at(self, close), Some(TokenKind::RBrace)) {
                    // `\v{\textbf{s}}`, `\c{cc}`: typeset the group as text.
                    self.diags.push(Diagnostic::warning(
                        format!("the argument to \\{name} is not a single letter, \\i or \\j; the accent is not drawn"),
                        Some(span),
                        Some("typeset the argument without the accent".into()),
                    ));
                    return;
                }
                let end = self.t[close].token.span;
                self.i = close + 1;
                (base, join(span, end))
            }
            Some(TokenKind::Word(w)) => {
                let w = w.clone();
                let first = w.chars().next().expect("words are non-empty");
                let word_span = self.t[self.i].token.span;
                let exact = word_span.end - word_span.start == w.len();
                let base_end =
                    if exact { word_span.start + first.len_utf8() } else { word_span.end };
                if w.len() == first.len_utf8() {
                    self.i += 1;
                } else if let Some(input) = self.token_mut(self.i) {
                    if exact {
                        input.token.span =
                            Span::in_document(word_span.document, base_end, word_span.end);
                    }
                    input.token.kind = TokenKind::Word(w[first.len_utf8()..].to_string());
                }
                let base_span = Span::in_document(word_span.document, word_span.start, base_end);
                (first.to_string(), join(span, base_span))
            }
            kind => match dotless(kind) {
                Some(base) => {
                    let end = self.t[self.i].token.span;
                    self.i += 1;
                    (base, join(span, end))
                }
                None => {
                    self.diags.push(Diagnostic::warning(
                        format!("\\{name} has no letter to accent"),
                        Some(span),
                        Some("typeset nothing for the accent".into()),
                    ));
                    return;
                }
            },
        };
        let enc = self.font_encoding;
        let bare = || match base.strip_prefix('\\') {
            Some(dotless) => match text_builtins::text_symbol(dotless, enc) {
                Some(SymbolOutcome::Char(ch)) => ch.to_string(),
                _ => String::new(),
            },
            None => base.clone(),
        };
        let text = match text_builtins::text_accent(name, &base, enc) {
            Some(AccentOutcome::Char(ch)) => ch.to_string(),
            Some(AccentOutcome::NoComposite) => {
                self.diags.push(Diagnostic::warning(
                    format!("\\{name}{{{base}}} has no precomposed character and \\accent is not implemented; the accent is not drawn"),
                    Some(full),
                    Some("typeset the letter without the accent".into()),
                ));
                bare()
            }
            Some(AccentOutcome::Unavailable(message)) => {
                self.diags.push(Diagnostic::error(
                    message,
                    Some(span),
                    Some("typeset the letter without the accent".into()),
                ));
                bare()
            }
            None => return,
        };
        if text.is_empty() {
            return;
        }
        para.push(Inline::Text {
            text,
            span: full,
            style,
            space_before,
        });
    }

    fn text_symbol(&mut self, name: &str, span: Span, para: &mut Vec<Inline>) {
        let space_before = self.space_precedes(self.i - 1);
        let style = self.style;
        if let Some(inline) = self.symbol_inline(name, span, style, space_before) {
            para.push(inline);
        }
    }

    /// A siunitx typesetting command (`crate::siunitx`): its arguments are
    /// read as raw source and the result is one inline formula spanning the
    /// command and its arguments.
    fn siunitx(&mut self, name: &str, span: Span, para: &mut Vec<Inline>) {
        let space_before = self.space_precedes(self.i - 1);
        let Some((required, pre_unit_bracket)) = siunitx::arity(name) else {
            return;
        };
        let options = self.siunitx_bracket();
        let mut full = options.as_ref().map_or(span, |(_, s)| span.merge(*s));
        let mut pre_unit = None;
        let mut args = Vec::with_capacity(required);
        for index in 0..required {
            if pre_unit_bracket && index == 1 {
                if let Some((raw, s)) = self.siunitx_bracket() {
                    full = full.merge(s);
                    pre_unit = Some(raw);
                }
            }
            let (tokens, argument_span) = self.required_group(name, span);
            if argument_span.document == span.document {
                full = full.merge(argument_span);
            }
            args.push(siunitx::raw_text(tokens.iter().map(|t| &t.token)));
        }
        let atoms = siunitx::typeset(
            name,
            options.as_ref().map(|(o, _)| o.as_str()),
            pre_unit.as_deref(),
            &args,
            false,
            self.math_packages,
            full,
            &mut self.diags,
        );
        if atoms.is_empty() {
            return;
        }
        para.push(Inline::Math {
            color: self.style.color,
            color_ranges: Vec::new(),
            list: crate::math::MathList { atoms },
            display: false,
            number: None,
            number_span: None,
            span: full,
            space_before,
        });
    }

    /// A `[key=value, ...]` argument read as raw source with its braces kept
    /// (`optional_bracket_argument` drops them, which would split
    /// `output-decimal-marker={,}` at the comma). Nothing is consumed when
    /// no bracket follows.
    fn siunitx_bracket(&mut self) -> Option<(String, Span)> {
        let (raw, span, next) = siunitx_bracket_at(&self.t, self.i)?;
        self.i = next;
        Some((raw, span))
    }

    /// `\DeclareSIUnit\name` or `\DeclareSIUnit{\name}`: the unit's name.
    fn command_or_group(&mut self, name: &str, span: Span) -> String {
        self.skip_spaces();
        if let Some(TokenKind::Command(command)) = self.peek().map(|t| t.kind.clone()) {
            self.i += 1;
            return command;
        }
        let (tokens, _) = self.required_group(name, span);
        siunitx::raw_text(tokens.iter().map(|t| &t.token))
    }

    /// `\uline`/`\sout` (ulem) or kernel text-mode `\underline`. Without
    /// ulem, the package commands diagnose and typeset the argument as
    /// plain text. Kernel `\underline` needs no package.
    fn text_underline_cmd(
        &mut self,
        name: &str,
        span: Span,
        para: &mut Vec<Inline>,
        geom: UnderlineGeom,
    ) {
        let space_before = self.space_precedes(self.i - 1);
        let (tokens, argument_span) = self.required_group(name, span);
        let full = span.merge(argument_span);
        let needs_ulem = !matches!(geom, UnderlineGeom::MathUnderline);
        if needs_ulem && !self.packages.iter().any(|package| package == "ulem") {
            self.diags.push(Diagnostic::command_error(
                name,
                format!("\\{name} needs \\usepackage{{ulem}}"),
                Some(full),
                Some("typeset the argument as plain text".into()),
            ));
            para.extend(self.box_inlines(tokens));
            return;
        }
        let content = self.box_inlines(tokens);
        let thickness_pt = match geom {
            UnderlineGeom::MathUnderline => MATH_RULE_THETA_PT,
            _ => UL_THICKNESS_PT,
        };
        para.push(Inline::Underline(Box::new(Underline {
            content,
            thickness_pt,
            geom,
            span: full,
            space_before,
        })));
    }

    fn text_logo(&mut self, name: &str, span: Span, para: &mut Vec<Inline>) {
        let space_before = self.space_precedes(self.i - 1);
        if let Some(logo) = TextLogo::from_command(name) {
            para.push(Inline::Logo {
                logo,
                span,
                style: self.style,
                space_before,
            });
        }
    }

    /// `\rule[<raise>]{<width>}{<height>}` (latex.ltx 16359-16367).
    fn text_rule(&mut self, span: Span, para: &mut Vec<Inline>) {
        let space_before = self.space_precedes(self.i - 1);
        let raise = self.optional_bracket_argument();
        let (width_tokens, width_span) = self.required_group("rule", span);
        let (height_tokens, height_span) = self.required_group("rule", span.merge(width_span));
        let full = span.merge(height_span);
        let parse = |text: String, what: &str, diags: &mut Vec<Diagnostic>| {
            let parsed = TextDimen::parse(&text);
            if parsed.is_none() {
                diags.push(Diagnostic::error(
                    format!(
                        "\\rule requires a recognised {what} dimension, got '{}'",
                        text.trim()
                    ),
                    Some(full),
                    Some("omitted the rule and continued".into()),
                ));
            }
            parsed
        };
        let raise = match raise {
            Some((text, _)) => parse(text, "raise", &mut self.diags),
            None => Some(TextDimen::zero()),
        };
        let width = parse(dimen_source(&width_tokens), "width", &mut self.diags);
        let height = parse(dimen_source(&height_tokens), "height", &mut self.diags);
        if let (Some(raise), Some(width), Some(height)) = (raise, width, height) {
            para.push(Inline::Rule {
                rule: TextRule {
                    raise,
                    width,
                    height,
                },
                span: full,
                style: self.style,
                space_before,
            });
        }
    }

    /// `\footnote`, `\footnotemark` and `\footnotetext`, following latex.ltx:
    /// without `[<n>]`, `\footnote`/`\footnotemark` step the counter and
    /// `\footnotetext` reuses its current value; with `[<n>]` none of them
    /// step it. The footnote counter and page-bottom placement are
    /// document-global, so incremental block reuse is disabled (the same
    /// conservative rule `\label`/`\ref` use).
    fn footnote(&mut self, name: &str, span: Span, para: &mut Vec<Inline>) {
        let space_before = self.space_precedes(self.i - 1);
        self.document_global_state = true;
        let explicit = self
            .optional_bracket_argument()
            .and_then(|(raw, raw_span)| {
                let parsed = raw.trim().parse::<u32>().ok();
                if parsed.is_none() {
                    self.diags.push(Diagnostic::warning(
                        format!(
                            "\\{name} optional argument '{}' is not a number",
                            raw.trim()
                        ),
                        Some(raw_span),
                        Some("numbered the footnote from the footnote counter instead".into()),
                    ));
                }
                parsed
            });
        // Inside a `minipage`, `\footnote` and `\footnotetext` use
        // `\@mpfn` = `mpfootnote` (`\thempfootnote`: `\alph`);
        // `\footnotemark` always uses `footnote`.
        let minipage =
            name != "footnotemark" && self.env_stack.iter().any(|(env, _)| is_minipage(env));
        let counter = if minipage {
            &mut self.mpfootnote_counter
        } else {
            &mut self.footnote_counter
        };
        let value = match explicit {
            Some(number) => number,
            None if name == "footnotetext" => *counter,
            None => {
                *counter += 1;
                *counter
            }
        };
        let number = if minipage {
            match alph(value) {
                Some(letter) => letter,
                None => {
                    self.diags.push(Diagnostic::error(
                        format!("minipage footnote number {value} is outside \\alph's a-z"),
                        Some(span),
                        Some("printed the number in arabic instead".into()),
                    ));
                    value.to_string()
                }
            }
        } else {
            value.to_string()
        };
        self.set_current_counter("footnote", Some(number.clone()));
        let text = if name == "footnotemark" {
            None
        } else {
            // `\@footnotetext` is `\long` (latex.ltx): a blank line inside
            // the argument is a paragraph break in the note, not its end.
            let (tokens, _) = self.required_group_bounded(name, span, true);
            Some(self.footnote_inlines(tokens, span))
        };
        para.push(Inline::Footnote {
            number,
            span,
            mark: name != "footnotetext",
            text,
            space_before,
        });
    }

    /// Parses a footnote argument with the ordinary dispatch, so math, style
    /// commands and macros work inside it. The text starts from
    /// `\normalfont` (`\@footnotetext` resets the font). Paragraph breaks
    /// inside the argument become line breaks: the footnote is one inline
    /// sequence, not separate blocks; each break is attributed to `span`.
    fn footnote_inlines(&mut self, tokens: Vec<InputToken>, span: Span) -> Vec<Inline> {
        self.argument_inlines(tokens, span, TextStyle::default())
    }

    /// Parses an argument with the ordinary dispatch starting in `style`
    /// (see [`P::footnote_inlines`]); paragraph breaks become line breaks
    /// attributed to `span`.
    fn argument_inlines(&mut self, tokens: Vec<InputToken>, span: Span, style: TextStyle) -> Vec<Inline> {
        let outer_tokens = std::mem::replace(&mut self.t, std::rc::Rc::new(tokens));
        let outer_index = std::mem::replace(&mut self.i, 0);
        let outer_style = std::mem::replace(&mut self.style, style);
        let outer_label = self.pending_item_label.take();
        let outer_item = self.pending_item.take();
        let outer_dependency_blocks = self.block_dependencies.len();
        let outer_par_leading_blocks = self.block_par_leading.len();
        let mut blocks = Vec::new();
        let mut para = Vec::new();
        self.parse_stream(&mut blocks, &mut para);
        self.flush_paragraph(&mut blocks, &mut para);
        self.block_dependencies.truncate(outer_dependency_blocks);
        self.block_par_leading.truncate(outer_par_leading_blocks);
        self.t = outer_tokens;
        self.i = outer_index;
        self.style = outer_style;
        self.pending_item_label = outer_label;
        self.pending_item = outer_item;

        let mut content: Vec<Inline> = Vec::new();
        for block in blocks {
            let inlines = match block {
                Block::Paragraph(inlines)
                | Block::Styled {
                    content: inlines, ..
                }
                | Block::ListItem {
                    content: inlines, ..
                } => inlines,
                _ => continue,
            };
            if !content.is_empty() && !inlines.is_empty() {
                content.push(Inline::LineBreak { span, skip_pt: None });
            }
            content.extend(inlines);
        }
        content
    }

    fn finish_block_dependencies(&mut self) {
        // Exactly one entry per pushed block, like `block_dependencies`:
        // every block push is followed by this call, and only
        // `flush_list_item` leaves a non-`None` value here.
        self.block_par_leading
            .push(std::mem::take(&mut self.next_block_par_leading));
        self.block_dependencies.push(
            std::mem::take(&mut self.current_dependencies)
                .into_iter()
                .map(|(name, (argument_count, replacement))| MacroDependency {
                    name,
                    argument_count,
                    replacement,
                })
                .collect::<Vec<_>>().into(),
        );
    }

    fn flush_paragraph(&mut self, blocks: &mut Vec<Block>, paragraph: &mut Vec<Inline>) {
        self.flush_list_item(blocks, paragraph, 0.0, 0.0);
    }

    /// The [`ParLeading`] of the paragraph being flushed: the size declaration
    /// in force *now*, which is what TeX's `\par` reads.
    ///
    /// Nothing looks at the sizes of the runs inside the paragraph:
    /// `\baselineskip` is a vertical parameter, and TeX never consults the
    /// boxes it stacks, only the register's value when it stacks them. `}`
    /// has already restored a group that closed before the paragraph did, and
    /// `\end` restores only after this flush, so `self.style` is exactly the
    /// state `\par` would see.
    fn par_leading(&self) -> ParLeading {
        self.style.size
    }

    /// Flushes the accumulated paragraph. Inside a list, this attaches the
    /// pending `\item` marker (for the first paragraph of an item; later
    /// paragraphs of the same item get the hanging indent without repeating
    /// it), the item's nesting level, and any `\setlist` itemsep/topsep gap
    /// due before or after it (`0.0`/`0.0` from `flush_paragraph`, meaning no
    /// override — mid-item paragraph breaks never get itemsep/topsep, which
    /// are gaps between items, not between paragraphs within one). Falls
    /// back to an ordinary `Block::Paragraph`/`Block::Styled` outside a list.
    fn flush_list_item(
        &mut self,
        blocks: &mut Vec<Block>,
        paragraph: &mut Vec<Inline>,
        extra_gap_before_pt: f64,
        extra_gap_after_pt: f64,
    ) {
        let label = self.pending_item_label.take();
        if paragraph.is_empty() && label.is_none() {
            return;
        }
        let item = self.pending_item.take();
        // The item's topsep/itemsep belongs to its labelled first paragraph,
        // even when a blank line inside the item flushes that paragraph
        // through `flush_paragraph` (which passes `0.0`); later paragraphs
        // of the same item never get it.
        let extra_gap_before_pt = match (&label, self.list_stack.last()) {
            (Some(_), Some(list)) if list.count > 0 => {
                if list.count <= 1 {
                    list.spacing.topsep_pt
                } else {
                    list.spacing.itemsep_pt
                }
            }
            (Some(_), _) => extra_gap_before_pt,
            (None, _) => 0.0,
        };
        let content = std::mem::take(paragraph);
        // A list level is "current" only once its first `\item` has been
        // seen (`count > 0`); text typed directly inside `itemize`/
        // `enumerate` before any `\item` falls back to an ordinary
        // paragraph, same as before this paragraph became list-aware.
        // A `quote`/`quotation`/`verse` inside an item is its own `\list`:
        // its paragraphs are `Styled`, with both frames in `lists`.
        let in_quote = label.is_none()
            && self
                .list_frames
                .last()
                .is_some_and(|frame| frame.environment.is_quote_like());
        let list_level = self
            .list_stack
            .last()
            .filter(|list| list.count > 0 && !in_quote)
            .map(|_| self.list_stack.len() as u8);
        // `leftmargin=*` needs every item's label, so it is resolved later
        // (backpatched once the list's `\end` is reached — see
        // `environment`); an explicit dimension is already known.
        let leftmargin = match self.list_stack.last() {
            Some(OpenList { spacing, .. }) => match spacing.leftmargin {
                LeftMarginSetting::Explicit(pt) => ListLeftMargin::Explicit(pt),
                LeftMarginSetting::Unset | LeftMarginSetting::Widest => ListLeftMargin::Default,
            },
            None => ListLeftMargin::Default,
        };
        // `template` doubles as `thebibliography`'s widest-label argument
        // (see the `\begin` handling in `environment`); `itemize`/`enumerate`
        // use it for their own unrelated `enumitem` template instead, so it
        // only carries a `widest_label` for a `thebibliography` list.
        let widest_label = self.list_stack.last().and_then(|list| {
            (list.kind == "thebibliography")
                .then(|| list.template.clone())
                .flatten()
        });
        let lists = self.list_frames.clone();
        self.next_block_par_leading = self.par_leading();
        blocks.push(match list_level {
            Some(level) => Block::ListItem {
                level,
                label,
                content,
                extra_gap_before_pt,
                extra_gap_after_pt,
                leftmargin,
                widest_label,
                lists,
                item,
            },
            None => match (self.paragraph_styles.last(), self.declared_alignment) {
                // A declaration inside `quote` would otherwise drop its indent.
                (Some(&ParagraphStyle::Quote), _) => Block::Styled {
                    style: ParagraphStyle::Quote,
                    content,
                    lists,
                    line_break_before: self.pending_line_break.take(),
                },
                (_, Some(style)) | (Some(&style), None) => Block::Styled {
                    style,
                    content,
                    lists,
                    line_break_before: None,
                },
                (None, None) => Block::Paragraph(content),
            },
        });
        self.finish_block_dependencies();
    }

    /// Mutable access to token `index` of the stream, without copying the
    /// stream: a stream still held by the expansion cache is borrowed from it
    /// (see [`expansion::lend_cached_tokens`]) and the edit is logged so
    /// [`P::restore_tokens`] can undo it.
    pub(crate) fn token_mut(&mut self, index: usize) -> Option<&mut InputToken> {
        if index >= self.t.len() {
            return None;
        }
        if !self.lent_from_cache && Rc::get_mut(&mut self.t).is_none() {
            self.lent_from_cache = expansion::lend_cached_tokens(self.entry_path, &self.t);
        }
        if self.lent_from_cache {
            self.undo.push((index, self.t[index].clone()));
        }
        Rc::make_mut(&mut self.t).get_mut(index)
    }

    /// The stream with every logged edit undone (the stream as expanded).
    fn restore_tokens(&mut self) -> Rc<Vec<InputToken>> {
        let mut tokens = std::mem::replace(&mut self.t, Rc::new(Vec::new()));
        if !self.undo.is_empty() {
            let stream = Rc::make_mut(&mut tokens);
            for (index, original) in self.undo.drain(..).rev() {
                stream[index] = original;
            }
        }
        tokens
    }

    /// Drops a `[<length>]` that directly follows `\\`, keeping any text glued
    /// to it (`\\[3pt]Next`) as the remainder of the word.
    /// Returns the length in TeX points when it reads as one.
    fn skip_line_break_length(&mut self) -> Option<f64> {
        let body = self.class_size_pt.unwrap_or(crate::layout::BODY_SIZE_PT);
        let input = self.token_mut(self.i)?;
        let TokenKind::Word(word) = &input.token.kind else {
            return None;
        };
        if !word.starts_with('[') {
            return None;
        }
        let close = word.find(']')?;
        let length = parse_dimen_pt_at(&word[1..close], body);
        let rest = word[close + 1..].to_string();
        if rest.is_empty() {
            self.i += 1;
            return length;
        }
        let span = input.token.span;
        if span.end - span.start == word.len() {
            input.token.span = Span::in_document(span.document, span.start + close + 1, span.end);
        }
        input.token.kind = TokenKind::Word(rest);
        length
    }

    /// `\item[<label>]` (latex.ltx 15964-15966: `\@ifnextchar[`, which
    /// skips spaces): the tokens between the brackets at brace depth 0,
    /// with the words holding `[`/`]` trimmed, and the span of the whole
    /// bracketed argument. `None` (nothing consumed but spaces) without a
    /// `[` or without its closing `]` before a paragraph break.
    fn item_label_argument(&mut self) -> Option<(Vec<InputToken>, Span)> {
        self.skip_spaces();
        let first = self.t.get(self.i)?;
        let TokenKind::Word(word) = &first.token.kind else {
            return None;
        };
        if !word.starts_with('[') {
            return None;
        }
        let open = first.token.span;
        let piece = |input: &InputToken, from: usize, to: usize| -> InputToken {
            let TokenKind::Word(word) = &input.token.kind else {
                return input.clone();
            };
            let span = input.token.span;
            let mut out = input.clone();
            out.token.kind = TokenKind::Word(word[from..to].to_string());
            if span.end - span.start == word.len() {
                out.token.span =
                    Span::in_document(span.document, span.start + from, span.start + to);
            }
            out
        };
        let mut tokens = Vec::new();
        let mut depth = 0usize;
        let mut index = self.i;
        while index < self.t.len() {
            let input = &self.t[index];
            let from = usize::from(index == self.i);
            match &input.token.kind {
                TokenKind::ParBreak => return None,
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => depth = depth.checked_sub(1)?,
                TokenKind::Word(word) if depth == 0 && word[from..].contains(']') => {
                    let close = from + word[from..].find(']').unwrap_or(0);
                    if close > from {
                        tokens.push(piece(input, from, close));
                    }
                    let span = input.token.span;
                    let literal = span.end - span.start == word.len();
                    let end = if literal { span.start + close + 1 } else { span.end };
                    if close + 1 == word.len() {
                        self.i = index + 1;
                    } else {
                        let rest = piece(input, close + 1, word.len());
                        if let Some(slot) = self.token_mut(index) {
                            *slot = rest;
                        }
                        self.i = index;
                    }
                    return Some((tokens, Span::in_document(open.document, open.start, end)));
                }
                _ => {}
            }
            let input = &self.t[index];
            if from == 1 {
                if let TokenKind::Word(word) = &input.token.kind {
                    if word.len() > 1 {
                        tokens.push(piece(input, 1, word.len()));
                    }
                }
            } else {
                tokens.push(input.clone());
            }
            index += 1;
        }
        None
    }

    /// The label of the `\item` just read (the innermost open list is
    /// `self.list_stack.last()`); sets `pending_item_label`/`pending_item`.
    fn begin_item(&mut self, span: Span, explicit: Option<(Vec<InputToken>, Span)>) {
        let frame = self
            .list_frames
            .iter()
            .rev()
            .find(|frame| !frame.environment.is_quote_like())
            .cloned();
        let environment = frame
            .as_ref()
            .map_or(ListEnvironment::Itemize, |frame| frame.environment);
        let kind_depth = frame.as_ref().map_or(1, |frame| frame.kind_depth);
        let explicit = explicit.map(|(tokens, arg_span)| {
            // `\descriptionlabel`: `\normalfont\bfseries #1`.
            let base = if environment == ListEnvironment::Description {
                TextStyle::BOLD
            } else {
                TextStyle::default()
            };
            let content = self.inlines_from_tokens(tokens, base);
            let mut text = String::new();
            for inline in &content {
                if let Inline::Text {
                    text: word,
                    space_before,
                    ..
                } = inline
                {
                    if *space_before && !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(word);
                }
            }
            ItemLabel::Explicit {
                content,
                text,
                span: arg_span,
            }
        });
        // `label*`: the enclosing enumerate's current label comes first.
        let enclosing_label = self
            .list_stack
            .iter()
            .rev()
            .skip(1)
            .find(|list| list.kind == "enumerate")
            .map(|list| list.current_label.clone())
            .unwrap_or_default();
        let enclosing_references = self
            .list_stack
            .iter()
            .take(self.list_stack.len().saturating_sub(1))
            .filter(|list| list.kind == "enumerate")
            .map(|list| list.current_reference.clone())
            .collect::<Vec<_>>();
        let Some(list) = self.list_stack.last_mut() else {
            return;
        };
        list.count += 1;
        let item = match explicit {
            Some(item) => item,
            None if environment == ListEnvironment::Enumerate => {
                list.counter += 1;
                let value = list.counter;
                let item = match (&list.label_star, &list.template) {
                    (Some(star), _) => ItemLabel::Template {
                        text: format!(
                            "{enclosing_label}{}",
                            lists::template_label(star, value).text()
                        ),
                    },
                    (None, Some(template)) => match template.strip_prefix("label=") {
                        Some(label) => lists::template_label(label, value),
                        None => lists::short_label(template, value),
                    },
                    (None, None) => lists::default_label(environment, kind_depth, value),
                };
                list.current_label = item.text().to_string();
                item
            }
            None => match (&list.template, environment) {
                (Some(template), ListEnvironment::Itemize) => ItemLabel::Template {
                    text: apply_text_ligatures(
                        template.strip_prefix("label=").unwrap_or(template),
                    ),
                },
                _ => lists::default_label(environment, kind_depth, 0),
            },
        };
        let item_text = item.text().to_string();
        let item_reference = match &item {
            ItemLabel::Counter { value, style, .. } => style.format(*value),
            _ => item_text.clone(),
        };
        list.current_reference = item_reference.clone();
        let reference_value = if environment == ListEnvironment::Enumerate {
            Self::enumerate_reference_value(&enclosing_references, item_reference)
        } else {
            item_reference
        };
        self.set_current_counter("item", Some(reference_value));
        self.pending_item_label = Some((item_text, span));
        self.pending_item = Some(item);
    }

    fn enumerate_reference_value(prefixes: &[String], current: String) -> String {
        let mut values = prefixes.to_vec();
        values.push(current);
        match values.as_slice() {
            [] => String::new(),
            [value] => value.clone(),
            [outer, inner] => format!("{outer}{inner}"),
            [outer, inner, rest @ ..] => {
                let mut value = format!("{outer}({inner})");
                for part in rest {
                    value.push_str(part);
                }
                value
            }
        }
    }

    fn push_list_frame(&mut self, environment: ListEnvironment, options: Vec<ListOption>, begin_span: Span) {
        let kind_depth = self
            .list_frames
            .iter()
            .filter(|frame| frame.environment == environment)
            .count() as u8
            + 1;
        self.list_frames.push(ListFrame {
            environment,
            kind_depth,
            options,
            begin_span,
        });
    }

    /// `\begin{itemize|enumerate|description}[<options>]`: resolves the
    /// enumitem keys in force (every matching `\setlist`, then `resume*`'s
    /// saved keys, then the `\begin` keys) and the counter's start value.
    fn open_list(&mut self, environment: &str, options: Option<String>, begin_span: Span, start: usize) {
        let Some(kind) = ListEnvironment::from_name(environment) else {
            return;
        };
        let body = self.class_size_pt.unwrap_or(crate::layout::BODY_SIZE_PT);
        let kind_depth = self
            .list_frames
            .iter()
            .filter(|frame| frame.environment == kind)
            .count() as u8
            + 1;
        let list_depth = self.list_frames.len() as u8 + 1;
        let mut effective: Vec<ListOption> = self
            .setlists
            .iter()
            .filter(|(target, _)| target.applies(kind, kind_depth, list_depth))
            .flat_map(|(_, options)| options.iter().cloned())
            .collect();
        let begin_options = options
            .as_deref()
            .map(|text| lists::parse_options(text, body, true))
            .unwrap_or_default();
        let start_of = |options: &[ListOption]| {
            options.iter().rev().find_map(|option| match option {
                ListOption::Start(n) => Some(n - 1),
                _ => None,
            })
        };
        let mut counter = start_of(&effective).unwrap_or(0);
        let mut series = None;
        for option in &begin_options {
            match option {
                ListOption::Resume(name) | ListOption::ResumeStar(name) => {
                    let key = name
                        .as_ref()
                        .map_or_else(|| environment.to_string(), |n| format!("series@{n}"));
                    counter = self.resume_counters.get(&key).copied().unwrap_or(0);
                    if matches!(option, ListOption::ResumeStar(_)) {
                        effective.extend(self.resume_keys.get(&key).cloned().unwrap_or_default());
                    }
                    self.document_global_state = true;
                }
                ListOption::Series(name) => {
                    series = Some(name.clone());
                    self.document_global_state = true;
                }
                _ => {}
            }
        }
        if let Some(value) = start_of(&begin_options) {
            counter = value;
        }
        effective.extend(begin_options.iter().cloned());
        let (template, label_star) = effective
            .iter()
            .rev()
            .find_map(|option| match option {
                ListOption::Label(label) => Some((Some(format!("label={label}")), None)),
                ListOption::ShortLabel(label) => Some((Some(label.clone()), None)),
                ListOption::LabelStar(label) => Some((None, Some(label.clone()))),
                _ => None,
            })
            .unwrap_or((None, None));
        let spacing = self
            .list_spacing
            .get(environment)
            .copied()
            .unwrap_or_default();
        self.list_stack.push(OpenList {
            kind: environment.to_string(),
            count: 0,
            template,
            spacing,
            start,
            counter,
            label_star,
            current_label: String::new(),
            current_reference: String::new(),
            series,
            begin_options,
        });
        self.push_list_frame(kind, effective, begin_span);
    }

    fn skip_spaces(&mut self) {
        while matches!(
            self.peek().map(|token| &token.kind),
            Some(TokenKind::Space | TokenKind::Comment)
        ) {
            self.i += 1;
        }
    }

    /// amsmath `\numberwithin[\style]{counter}{parent}` and the LaTeX
    /// kernel's `\counterwithin(*)`/`\counterwithout(*){counter}{parent}`:
    /// the counter is reset (or no longer reset) by `parent` and printed as
    /// `\the<parent>.\<style>{counter}` (see `xref::Counters`). A
    /// `\newtheorem` counter only follows `section` (the theorem numbering in
    /// `theorems`); an unknown counter is LaTeX's "No counter defined" error.
    fn counter_numbering(&mut self, name: &str, span: Span) {
        use crate::xref::{CounterError, NumberStyle};
        let starred = name != "numberwithin" && self.take_optional_star();
        let mut style = NumberStyle::Arabic;
        if name == "numberwithin" {
            if let Some((text, style_span)) = self.optional_bracket_argument() {
                match NumberStyle::from_command(&text) {
                    Some(parsed) => style = parsed,
                    None => self.diags.push(Diagnostic::warning(
                        format!(
                            "\\numberwithin format '{}' is not \\arabic, \\alph, \\Alph, \\roman or \\Roman",
                            text.trim()
                        ),
                        Some(style_span),
                        Some("numbered the counter in arabic".into()),
                    )),
                }
            }
        }
        let (child_tokens, child_span) = self.required_group(name, span);
        let (parent_tokens, parent_span) = self.required_group(name, span);
        let child = token_text(&child_tokens).trim().to_string();
        let parent = token_text(&parent_tokens).trim().to_string();
        let whole = span.merge(child_span).merge(parent_span);
        self.document_global_state = true;
        if !self.counters.exists(&child) && self.theorems.values().any(|def| def.counter == child) {
            if parent == "section" {
                let within = name != "counterwithout";
                for def in self.theorems.values_mut() {
                    if def.counter == child {
                        def.within_section = within;
                    }
                }
            } else {
                self.diags.push(Diagnostic::warning(
                    format!("\\{name} for theorem counter '{child}' within '{parent}' is recognised but not implemented"),
                    Some(whole),
                    Some(format!("'{child}' keeps its numbering")),
                ));
            }
            return;
        }
        let result = match name {
            "numberwithin" => self.counters.numberwithin(&child, &parent, style),
            "counterwithin" => self.counters.counter_within(&child, &parent, starred),
            _ => self.counters.counter_without(&child, &parent, starred),
        };
        if let Err(CounterError::NoCounter(missing)) = result {
            self.diags.push(Diagnostic::error(
                format!("No counter '{missing}' defined"),
                Some(whole),
                Some(format!("ignored the \\{name}")),
            ));
        }
    }

    /// amsmath.sty `\subequations`: `\refstepcounter{equation}` (a `\label`
    /// right after `\begin{subequations}` gets the parent number),
    /// `\protected@edef\theparentequation{\theequation}`,
    /// `\setcounter{parentequation}{\value{equation}}`,
    /// `\setcounter{equation}{0}` and
    /// `\def\theequation{\theparentequation\alph{equation}}`.
    fn begin_subequations(&mut self) {
        use crate::xref::{NumberStyle, Piece};
        let parent = self.counters.step("equation").unwrap_or_default();
        self.set_current_counter("equation", Some(parent.clone()));
        let value = self.counters.value("equation").unwrap_or(0);
        self.counters.set_value("parentequation", value);
        self.counters.set_value("equation", 0);
        let saved = self.counters.representation("equation").unwrap_or_default();
        self.counters.set_representation(
            "equation",
            vec![
                Piece::Text(parent),
                Piece::Value("equation".into(), NumberStyle::AlphLower),
            ],
        );
        self.subequations.push(saved);
        self.document_global_state = true;
    }

    /// `\endsubequations`: `\setcounter{equation}{\value{parentequation}}`;
    /// the group end restores `\theequation`.
    fn end_subequations(&mut self) {
        if let Some(saved) = self.subequations.pop() {
            let parent = self.counters.value("parentequation").unwrap_or(0);
            self.counters.set_value("equation", parent);
            self.counters.set_representation("equation", saved);
        }
    }

    fn unsupported_preamble(&mut self, name: &str, span: Span) {
        self.diags.push(Diagnostic::command_error(
            name,
            format!("\\{} is not supported in the document preamble", name),
            Some(span),
            Some("skipped the command and did not typeset preamble content".into()),
        )
        // `with_optional_help` keeps help `command_error` already attached: an
        // unknown command here is usually a typo, and its did-you-mean (with
        // the replacement the editor can apply) is worth more than advice to
        // move a command that does not exist. Plain `with_help` would drop it.
        .with_optional_help(Some(format!(
            "move \\{name} after \\begin{{document}}, or remove it from the preamble"
        ))));
    }

    /// Recovery policy for a command this compiler does not implement.
    ///
    /// The diagnostic naming the command must always survive — that is the
    /// contract that lets an author discover the gap; it is never hidden or
    /// weakened by what follows. What varies is only whether the following
    /// brace/bracket argument is also consumed. Left alone, the main token
    /// loop just keeps walking: a `{` opens an anonymous group and its
    /// contents fall through to ordinary paragraph text, so the argument
    /// itself becomes visible body text (e.g. `\vspace{0.6em}` used to leak
    /// the word "0.6em" onto the page, before `\vspace` gained its own
    /// implementation). That is fine — even correct — for a command whose
    /// argument IS meant to be read as prose: an unknown macro someone typoed,
    /// `\mycommand{Some real sentence}`, must keep that sentence visible, or
    /// the recovery would silently eat the author's content.
    ///
    /// So the argument is only skipped when it is conservatively safe to
    /// assume it is a parameter, not prose:
    ///   1. `name` is in `KNOWN_ARITY_UNIMPLEMENTED`: a command this compiler
    ///      recognises by name as taking a fixed count of non-prose
    ///      arguments it does not yet implement. Its whole arity is consumed
    ///      unconditionally — the command name alone is enough context.
    ///   2. Otherwise, for a genuinely unrecognised command, only the ONE
    ///      immediately following `{...}` group is inspected, and only
    ///      consumed if its full (trimmed) contents look like a dimension or
    ///      a keyword — see `looks_like_recoverable_argument`. Anything else
    ///      (multiple words, punctuation, a capitalized word, a lone letter)
    ///      is left in place and typeset as text, exactly as before.
    ///
    /// Either way, the diagnostic's recovery note records whether an argument
    /// was skipped, so the choice itself stays auditable from the output.
    fn unsupported(&mut self, name: &str, span: Span) {
        debug_assert!(!BUILT_INS.contains(&name));
        let skipped = self.skip_recoverable_argument(name);
        self.diags.push(Diagnostic::command_error(
            name,
            // A text-mode command: math has its own reader and diagnostics,
            // so this message says nothing about math mode.
            format!("\\{} is not supported by this compiler version", name),
            Some(span),
            Some(if skipped {
                "skipped the command and its argument, which looked like a parameter rather than text".into()
            } else {
                "skipped the command; any braced argument was typeset as plain text".into()
            }),
        )
        .with_optional_help(vocabulary::command_help(name))
        .with_label(span, "this command", true));
    }

    /// Commands this compiler recognises by name as taking a fixed count of
    /// non-prose arguments it does not implement. Each listed argument is
    /// always skipped, regardless of content — the command name alone gives
    /// enough context to know the text was never meant to reach the page.
    /// Deliberately excludes `\vspace`/`\hrule`/`\newpage`/`\pagestyle` (and
    /// `\Large`/`\setlength`): those already have, or are gaining, their own
    /// real implementations elsewhere, so hardcoding them here would fight
    /// that work instead of falling out of it automatically.
    fn skip_recoverable_argument(&mut self, name: &str) -> bool {
        const KNOWN_ARITY_UNIMPLEMENTED: &[(&str, usize)] = &[
            // `\linespread{1.5}`: a bare scale factor with no unit suffix, so
            // the dimension heuristic below would never catch it on its own.
            ("linespread", 1),
        ];
        if let Some(&(_, arity)) = KNOWN_ARITY_UNIMPLEMENTED
            .iter()
            .find(|(known, _)| *known == name)
        {
            let mut skipped_any = false;
            for _ in 0..arity {
                if self.try_skip_braced_group(None) {
                    skipped_any = true;
                } else {
                    break;
                }
            }
            return skipped_any;
        }
        self.try_skip_braced_group(Some(looks_like_recoverable_argument))
    }

    /// Skips one `{...}` group immediately ahead (after whitespace), if one
    /// is there — and, when `predicate` is given, only when the group's
    /// trimmed text content satisfies it. Never emits a diagnostic of its
    /// own and never advances past anything on a rejected attempt: the
    /// missing- or non-matching-argument case is silent by design, since the
    /// ordinary token loop is what typesets it as text afterwards.
    fn try_skip_braced_group(&mut self, predicate: Option<fn(&str) -> bool>) -> bool {
        let mut cursor = self.i;
        while matches!(
            self.t.get(cursor).map(|input| &input.token.kind),
            Some(TokenKind::Space | TokenKind::Comment)
        ) {
            cursor += 1;
        }
        if !matches!(
            self.t.get(cursor).map(|input| &input.token.kind),
            Some(TokenKind::LBrace)
        ) {
            return false;
        }
        let mut depth = 1usize;
        let mut scan = cursor + 1;
        let close = loop {
            match self.t.get(scan).map(|input| &input.token.kind) {
                Some(TokenKind::LBrace) => depth += 1,
                Some(TokenKind::RBrace) => {
                    depth -= 1;
                    if depth == 0 {
                        break scan;
                    }
                }
                Some(_) => {}
                // Unterminated group: leave it for ordinary recovery rather
                // than guessing where it would have closed.
                None => return false,
            }
            scan += 1;
        };
        if let Some(predicate) = predicate {
            let content = token_text(&self.t[cursor + 1..close]);
            if !predicate(content.trim()) {
                return false;
            }
        }
        self.i = close + 1;
        true
    }
}

/// True when loading `package` with `options` changes nothing about the output,
/// because the fixed layout already behaves that way.
fn package_matches_layout(package: &str, options: &str) -> bool {
    let options: Vec<&str> = options
        .split(',')
        .map(str::trim)
        .filter(|option| !option.is_empty())
        .collect();
    match package {
        // Source text is decoded as UTF-8 already.
        "inputenc" => options.iter().all(|option| *option == "utf8"),
        // Text glyphs are mapped from Unicode, which is what T1 approximates.
        "fontenc" => options.iter().all(|option| *option == "T1"),
        // Enumerate label templates are implemented; \setlist reports its own gap.
        "enumitem" => options.iter().all(|option| *option == "shortlabels"),
        "geometry" => {
            !options.is_empty()
                && options.iter().all(|option| match option.split_once('=') {
                    Some(("margin", value)) => length_pt(value)
                        .is_some_and(|pt| (pt - crate::layout::MARGIN_PT).abs() < 0.01),
                    None => *option == "letterpaper",
                    _ => false,
                })
        }
        // \newtheorem/\theoremstyle/proof are implemented (see theorems.rs);
        // amsthm takes no package options of its own.
        "amsthm" => options.is_empty(),
        // array.sty's preamble builder, column types and row strut are
        // implemented (parser/tabular.rs, crate::tabular); no options.
        "array" => options.is_empty(),
        // natbib citation commands (crate::natbib) with the delimiter,
        // separator and citation-style options that decide the characters
        // they set. `sort`/`compress`/`super`/`longnamesfirst` are parsed but
        // change the output, so they keep the warning.
        "natbib" => options
            .iter()
            .all(|option| crate::natbib::IMPLEMENTED_OPTIONS.contains(option)),
        // siunitx v3 numbers, units, quantities, lists, ranges and angles
        // (crate::siunitx); its options are \sisetup keys, and a key that
        // is not modelled gets its own diagnostic there.
        "siunitx" => true,
        // multicols/multicols*, \columnbreak and \raggedcolumns are parsed
        // (the render pipeline sets the columns); the tracing options change
        // nothing typeset.
        "multicol" => options
            .iter()
            .all(|option| matches!(*option, "errorshow" | "infoshow" | "balancingshow" | "markshow" | "debugshow")),
        // Table packages (parser/tabular.rs, crate::tabular): booktabs rules
        // and spacing, longtable page-breaking tables, multirow entries and
        // colortbl row/column/cell colours and rule colours.
        "booktabs" | "longtable" | "multirow" | "colortbl" => options.is_empty(),
        // amsmath/amssymb/amsfonts: implemented here, not merely recognised.
        // `crate::math` parses the constructs into their own nuclei and
        // `math::layout_nucleus` sets them — `GenFraction`
        // (\dfrac/\tfrac/\binom/\genfrac), `Phantom`, `Operator`
        // (\operatorname, \DeclareMathOperator), `SubArray` (\substack),
        // `ExtArrow` and `Framed` (\boxed) — while the align/gather/multline
        // /cases/matrix families reach `layout::display_rows`, and
        // `takes_display_limits` gives the \lim family, \sum and \prod their
        // display limits. The symbol inventory is gated on which file
        // declared each name (`MathPackages::provides`, `crate::amssymb`:
        // amsfonts' 22-name subset with \mathbb/\mathfrak, amssymb's full
        // 203), so loading the package is what makes those names exist at
        // all, and \colon takes amsmath's wider definition
        // (`MathPackages::amsmath`).
        //
        // Like `siunitx` and `enumitem` above, the gaps that remain report
        // themselves where they are used rather than at \usepackage:
        // \sideset, \shoveleft, \smash, \mspace, \hdotsfor and the
        // \varinjlim family each raise "\X is not supported in math mode" at
        // their own span. A blanket package warning on top of that is false
        // for every document that stays inside the implemented set --
        // `fixtures/real-world/hw1` and `hw2` are exactly that -- and adds
        // nothing to a document that does not, which already has a precise
        // error pointing at the construct.
        //
        // Only the options that are amsmath's own defaults are accepted: they
        // select behaviour this crate already produces. `leqno`, `fleqn`,
        // `tbtags`, `nosumlimits`, `intlimits` and `nonamelimits` each move
        // real output and are not read here, so they keep the warning (the
        // same rule `natbib` and `geometry` follow). amssymb and amsfonts
        // take no options of their own.
        "amsmath" => options.iter().all(|option| {
            matches!(
                *option,
                "centertags" | "sumlimits" | "nointlimits" | "namelimits" | "reqno"
            )
        }),
        "amssymb" | "amsfonts" => options.is_empty(),
        // microtype (character protrusion and font expansion) is genuinely
        // absent from this crate: it has no dependency on
        // `flashtex-microtype`, and nothing here protrudes a character or
        // expands a font, so the warning is true of the output this crate
        // lays out and must stay. The render pipeline, which does set it
        // (`typeset.rs` calls `paragraph_layout::layout_paragraph_microtype`),
        // drops this line for its own consumers in
        // `render_pipeline::packages::supersede_message` -- which is where
        // route-specific knowledge belongs, because this crate's display list
        // is consumed by both `flashtex-render` and the plain
        // `flashtex-compiler` worker and it cannot tell which is asking.
        // hyperref: its options are PDF annotation, outline and metadata
        // settings, and none of them moves a glyph (see
        // `hyperref_option_is_layout_neutral`). `\url`/`\href`/`\nolinkurl`
        // are typeset; the annotations themselves are reported once by
        // `note_links_unclickable`, so a second "not implemented" line here
        // would only suggest the *text* is wrong, which it is not.
        "hyperref" => options.iter().all(|option| hyperref_option_is_layout_neutral(option)),
        // cleveref's unknown package options are intentionally ignored by
        // the package, so loading it is silent for every option here.
        "cleveref" => true,
        // Colour packages (crate::color) with every option replayed.
        "xcolor" => crate::color::Colors::xcolor(&options.join(","), None).1.is_empty(),
        "color" => crate::color::Colors::color_sty(&options.join(",")).1.is_empty(),
        // `\uline` and `\sout` are implemented; `\emph` is not redefined
        // (ulem's default `ULforem`) and `\uuline` stays unsupported if used.
        "ulem" => options.iter().all(|option| *option == "normalem"),
        _ => false,
    }
}

/// The key *names* of a `listings` key list: entries split at top-level
/// commas, each truncated at its first top-level `=`. Braces and brackets
/// nest, so `caption={a, b}` and `basewidth={0.6em,0.45em}` are one key
/// each, and a backslash skips the character after it so `\\{` inside a
/// style value does not open a group.
fn listings_key_names(list: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = list.as_bytes();
    let (mut start, mut depth, mut i) = (0usize, 0i32, 0usize);
    while i <= bytes.len() {
        let end = i == bytes.len();
        match if end { b',' } else { bytes[i] } {
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth -= 1,
            b'\\' if !end => i += 1,
            b',' if depth <= 0 => {
                let entry = &list[start..i];
                let name = entry.split_once('=').map_or(entry, |(name, _)| name).trim();
                if !name.is_empty() {
                    out.push(name.to_string());
                }
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    out
}

/// Whether `key` is a `listings` key at all (listings.sty / lstmisc.sty
/// v1.10c, TeX Live 2025: every `\\lst@Key` the package defines, plus the
/// `\\lst@Key`-less switches `\\lstset` accepts). Being on this list is not a
/// claim that anything is done with it — `\\lstset` typesets nothing either
/// way — only that the name is spelled like a key of the package.
fn listings_key_is_known(key: &str) -> bool {
    const KEYS: &[&str] = &[
        // Styles and the character grid.
        "basicstyle", "identifierstyle", "commentstyle", "stringstyle", "keywordstyle",
        "ndkeywordstyle", "classoffset", "texcsstyle", "directivestyle", "emph", "moreemph",
        "deleteemph", "emphstyle", "delim", "moredelim", "deletedelim", "columns", "flexiblecolumns",
        "basewidth", "fontadjust", "keepspaces", "showspaces", "showtabs", "showstringspaces",
        "formatstyle", "literate", "alsoletter", "alsodigit", "alsoother", "sensitive",
        // Line numbers and labels.
        "numbers", "numberstyle", "numbersep", "stepnumber", "numberfirstline", "firstnumber",
        "numberblanklines", "name", "numberbychapter",
        // Frames, margins and background.
        "frame", "frameshape", "frameround", "framerule", "framesep", "framexleftmargin",
        "framexrightmargin", "framextopmargin", "framexbottommargin", "backgroundcolor",
        "fillcolor", "rulecolor", "rulesepcolor", "rulesep", "xleftmargin", "xrightmargin",
        "resetmargins", "linewidth", "lineskip", "boxpos",
        // Captions, floats and the list of listings.
        "caption", "title", "label", "captionpos", "abovecaptionskip", "belowcaptionskip",
        "aboveskip", "belowskip", "float", "floatplacement", "nolol", "multicols",
        // Language and what is typeset.
        "language", "alsolanguage", "defaultdialect", "print", "firstline", "lastline",
        "linerange", "consecutivenumbers", "showlines", "extendedchars", "inputencoding",
        "escapechar", "escapeinside", "escapebegin", "escapeend", "mathescape", "texcl",
        "gobble", "tabsize", "index", "moreindex", "deleteindex", "indexstyle",
        // Line breaking.
        "breaklines", "breakatwhitespace", "breakindent", "breakautoindent", "prebreak",
        "postbreak", "breakbefore", "breakafter", "style", "morecomment", "morestring",
        "morekeywords", "deletekeywords", "morendkeywords", "keywordsprefix", "procnamekeys",
        "procnamestyle", "indexprocnames", "tag",
    ];
    KEYS.contains(&key)
}

/// Whether one `hyperref` package option or `\\hypersetup` key leaves the
/// typeset material alone.
///
/// Measured, not assumed: the same document (`\maketitle`, `abstract`,
/// `\tableofcontents`, `\ref`/`\pageref`, `\url`, `\href`, a `\footnote` and
/// a `\cite`d `thebibliography`) was set by pdflatex (TeX Live 2025) once per
/// option set and every word's origin compared against a control that loads
/// `url.sty` with a text-only `\href`. `default`, `colorlinks`, `hidelinks`,
/// `pdfborder`, `bookmarks=false`, `breaklinks`, `linktoc=all`,
/// `pdfstartview`, the `pdf*` metadata keys, `unicode` and `pdfpagemode` all
/// give **identical text at identical positions**: 137 words, 1 page, 0
/// moved. The same holds on `fixtures/real-world/hyperref-toc` itself: 1062
/// words and 4 pages with and without hyperref, 0 moved.
///
/// `backref` and `pagebackref` are **not** on the list. They add
/// back-reference text to every bibliography entry, which is new material,
/// and they were not measurable in that harness (they need `\newblock`
/// structure this document did not have), so they keep warning rather than
/// being claimed neutral on an unmeasured guess. Anything unrecognised warns
/// for the same reason.
fn hyperref_option_is_layout_neutral(option: &str) -> bool {
    let key = option.split_once('=').map_or(option, |(key, _)| key).trim();
    matches!(
        key,
        // Link appearance: colour and border only.
        "colorlinks" | "hidelinks" | "linkcolor" | "urlcolor" | "citecolor" | "filecolor"
            | "menucolor" | "runcolor" | "anchorcolor" | "allcolors" | "pdfborder"
            | "linkbordercolor" | "urlbordercolor" | "citebordercolor" | "allbordercolors"
            | "pdfborderstyle" | "borderwidth"
            // Outline (bookmark) settings: no page material.
            | "bookmarks" | "bookmarksopen" | "bookmarksnumbered" | "bookmarksdepth"
            | "bookmarksopenlevel" | "bookmarkstype"
            // Viewer preferences and document metadata.
            | "pdfstartview" | "pdfstartpage" | "pdfpagemode" | "pdfpagelayout" | "pdfview"
            | "pdftitle" | "pdfauthor" | "pdfsubject" | "pdfkeywords" | "pdfcreator"
            | "pdfproducer" | "pdflang" | "pdfdisplaydoctitle" | "pdfnewwindow"
            // Which constructs become links, and how names are made.
            | "linktoc" | "linktocpage" | "hyperindex" | "hyperfootnotes" | "pageanchor"
            | "plainpages" | "hypertexnames" | "naturalnames" | "destlabel" | "breaklinks"
            // Encoding of the PDF strings, and the driver.
            | "unicode" | "psdextra" | "pdfencoding" | "driverfallback" | "pdftex" | "dvipdfm"
            | "dvips" | "xetex" | "luatex" | "final" | "draft"
    )
}

fn length_pt(value: &str) -> Option<f64> {
    let value = value.trim();
    let split = value
        .find(|c: char| c.is_ascii_alphabetic())
        .unwrap_or(value.len());
    let number: f64 = value[..split].trim().parse().ok()?;
    let per_unit = match value[split..].trim() {
        "in" => 72.0,
        "pt" => 72.0 / 72.27,
        "bp" => 1.0,
        "cm" => 72.0 / 2.54,
        "mm" => 72.0 / 25.4,
        _ => return None,
    };
    Some(number * per_unit)
}

/// Formats an enumitem label: a `label=` key using `\alph*`-style counters,
/// or a shortlabels template whose first `a A i I 1` is the counter.
fn enumitem_label(template: &str, count: u32) -> String {
    let counter = |style: char| match style {
        'a' => alphabetic(count, b'a'),
        'A' => alphabetic(count, b'A'),
        'i' => roman(count),
        'I' => roman(count).to_uppercase(),
        _ => count.to_string(),
    };
    if template.contains('=') {
        let Some(label) = template
            .split(',')
            .find_map(|key| key.trim().strip_prefix("label="))
        else {
            return format!("{}.", count);
        };
        return [
            ("\\alph*", 'a'),
            ("\\Alph*", 'A'),
            ("\\roman*", 'i'),
            ("\\Roman*", 'I'),
            ("\\arabic*", '1'),
        ]
        .iter()
        .fold(label.trim().to_string(), |text, (command, style)| {
            text.replace(command, &counter(*style))
        });
    }
    match template.char_indices().find(|(_, c)| "aAiI1".contains(*c)) {
        Some((index, style)) => format!(
            "{}{}{}",
            &template[..index],
            counter(style),
            &template[index + style.len_utf8()..]
        ),
        None => template.to_string(),
    }
}

/// The counter style (`a A i I 1`) an enumitem label template selects,
/// mirroring `enumitem_label`'s own template parsing (defaulting to `1`,
/// arabic, exactly like it does). Used by `\setlist{leftmargin=*}` to decide
/// how far its widest-label search needs to look — see `environment`.
fn enumitem_label_style(template: &str) -> char {
    if template.contains('=') {
        let Some(label) = template
            .split(',')
            .find_map(|key| key.trim().strip_prefix("label="))
        else {
            return '1';
        };
        return [
            ("\\alph*", 'a'),
            ("\\Alph*", 'A'),
            ("\\roman*", 'i'),
            ("\\Roman*", 'I'),
        ]
        .iter()
        .find(|(command, _)| label.contains(command))
        .map_or('1', |(_, style)| *style);
    }
    template
        .char_indices()
        .find(|(_, c)| "aAiI1".contains(*c))
        .map_or('1', |(_, style)| style)
}

fn alphabetic(count: u32, base: u8) -> String {
    match count {
        1..=26 => char::from(base + (count - 1) as u8).to_string(),
        _ => count.to_string(),
    }
}

fn roman(mut count: u32) -> String {
    const NUMERALS: &[(u32, &str)] = &[
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut text = String::new();
    for (value, numeral) in NUMERALS {
        while count >= *value {
            text.push_str(numeral);
            count -= value;
        }
    }
    text
}

fn preamble_source(text: &str, has_document: bool, tokens: &[InputToken]) -> String {
    let end = if has_document {
        document_begin_end(tokens)
    } else if tokens.iter().any(|input| {
        matches!(
            &input.token.kind,
            TokenKind::Command(name) if name == "documentclass" || name == "usepackage"
        )
    }) {
        Some(text.len())
    } else {
        None
    };
    end.filter(|end| *end <= text.len() && text.is_char_boundary(*end))
        .map_or("", |end| &text[..end])
        .to_string()
}

/// The kern a control-symbol token (`\,` lexed as the word `,` with a
/// two-byte span, the same test `math.rs` uses) stands for in text mode.
fn control_symbol_kern(word: &str, span: Span, amsmath: bool) -> Option<TextDimen> {
    let mut chars = word.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None)
            if span.end - span.start == 2 && text_builtins::KERN_CONTROL_SYMBOLS.contains(&c) =>
        {
            text_builtins::text_kern(word, amsmath)
        }
        _ => None,
    }
}

/// A dimension argument's source text with control words kept
/// (`\textwidth`), unlike `token_text`.
fn dimen_source(tokens: &[InputToken]) -> String {
    let mut result = String::new();
    for input in tokens {
        match &input.token.kind {
            TokenKind::Word(text) => result.push_str(text),
            TokenKind::Command(name) => {
                result.push('\\');
                result.push_str(name);
            }
            TokenKind::Space | TokenKind::ParBreak => result.push(' '),
            _ => {}
        }
    }
    result
}

/// Like `token_text`, but control words keep their backslash and braces
/// are kept (enumitem values such as `label={(\alph*)}`).
fn token_source(tokens: &[InputToken]) -> String {
    let mut result = String::new();
    for input in tokens {
        match &input.token.kind {
            TokenKind::Word(text) => result.push_str(text),
            TokenKind::Command(text) => {
                result.push('\\');
                result.push_str(text);
            }
            TokenKind::LBrace => result.push('{'),
            TokenKind::RBrace => result.push('}'),
            TokenKind::Space | TokenKind::ParBreak => result.push(' '),
            _ => {}
        }
    }
    result
}

/// A siunitx `[key=value, ...]` argument at `index` (after spaces), read as
/// raw source with its braces kept: (options, span, index after `]`).
fn siunitx_bracket_at(tokens: &[InputToken], index: usize) -> Option<(String, Span, usize)> {
    let mut index = index;
    while matches!(tokens.get(index).map(|t| &t.token.kind), Some(TokenKind::Space)) {
        index += 1;
    }
    let first = tokens.get(index)?;
    if !matches!(&first.token.kind, TokenKind::Word(w) if w.starts_with('[')) {
        return None;
    }
    let start = first.token.span;
    let mut raw = String::new();
    let mut depth = 0usize;
    let mut cursor = index;
    while let Some(input) = tokens.get(cursor) {
        cursor += 1;
        match &input.token.kind {
            TokenKind::LBrace => {
                depth += 1;
                raw.push('{');
            }
            TokenKind::RBrace => {
                depth = depth.saturating_sub(1);
                raw.push('}');
            }
            TokenKind::ParBreak => return None,
            _ => {
                let mut piece = siunitx::raw_text(std::iter::once(&input.token));
                if cursor == index + 1 {
                    piece.remove(0);
                }
                if depth == 0 {
                    if let Some(close) = piece.find(']') {
                        raw.push_str(&piece[..close]);
                        return Some((raw, start.merge(input.token.span), cursor));
                    }
                }
                raw.push_str(&piece);
            }
        }
    }
    None
}

/// A braced siunitx argument at `index` (after spaces) as raw source without
/// its outer braces: (argument, span, index after `}`).
fn siunitx_group_at(tokens: &[InputToken], index: usize) -> Option<(String, Span, usize)> {
    let mut index = index;
    while matches!(tokens.get(index).map(|t| &t.token.kind), Some(TokenKind::Space)) {
        index += 1;
    }
    let open = tokens.get(index)?;
    if open.token.kind != TokenKind::LBrace {
        return None;
    }
    let mut depth = 0usize;
    for (offset, input) in tokens[index..].iter().enumerate() {
        match input.token.kind {
            TokenKind::LBrace => depth += 1,
            TokenKind::RBrace => {
                depth -= 1;
                if depth == 0 {
                    let inner = tokens[index + 1..index + offset].iter().map(|t| &t.token);
                    let span = if input.token.span.document == open.token.span.document {
                        open.token.span.merge(input.token.span)
                    } else {
                        open.token.span
                    };
                    return Some((siunitx::raw_text(inner), span, index + offset + 1));
                }
            }
            _ => {}
        }
    }
    None
}

/// The text between the `[` at `open` and its `]` (brace groups may hold
/// `]`), or `None` when `open` is not a `[` or the bracket is unclosed.
fn bracket_inner(text: &str, open: usize) -> Option<&str> {
    if text.as_bytes().get(open) != Some(&b'[') {
        return None;
    }
    let mut depth = 0usize;
    for (i, c) in text[open + 1..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ']' if depth == 0 => return Some(&text[open + 1..open + 1 + i]),
            '\n' if text[open + 1..open + 1 + i].ends_with('\n') => return None,
            _ => {}
        }
    }
    None
}

/// The keys of a `\cite`-family argument: comma-separated, each trimmed
/// (`\@for` over `\NAT@cite@list` does the same, which is why
/// `\citep{a, b}` and `\citep{a,b}` set identically).
fn cite_keys(tokens: &[InputToken]) -> Vec<String> {
    token_text(tokens)
        .split(',')
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_string)
        .collect()
}

fn token_text(tokens: &[InputToken]) -> String {
    let mut result = String::new();
    for input in tokens {
        match &input.token.kind {
            TokenKind::Word(text) | TokenKind::Command(text) => result.push_str(text),
            TokenKind::Space | TokenKind::ParBreak => result.push(' '),
            _ => {}
        }
    }
    result
}

/// Characters after which `url.sty` allows a URL to break onto a new line,
/// with no hyphen ever inserted at the break — see `url_pieces` and
/// `P::push_url_text`.
///
/// **Measured, not copied from the package source.** Each candidate was set
/// as `\url{xxxxxxxx<c>xxxxxxxx}` in a 56pt `minipage` by pdflatex (TeX Live
/// 2025, 11pt `article`, T1), a width where the only possible break is right
/// after `<c>`; the box has two lines exactly when url.sty permits that
/// break. Breaking: `/ . ? & # = + : _ , ; ! | > ) ] ' @`. Not breaking:
/// `- ~ * $` (each stayed on one overfull line).
///
/// Two of those corrections matter in real documents:
///
/// - **`-` is not a break.** url.sty deliberately refuses to break at a
///   hyphen, so a reader cannot mistake a URL's own hyphen for hyphenation.
///   It puts a 0.5pt kern there instead (`URL_HYPHEN_KERN_PT`).
/// - **`~` is not a break** either, though it reads like a path separator.
const URL_BREAK_AFTER: &[char] = &[
    '/', '.', '?', '&', '#', '=', '+', ':', '_', ',', ';', '!', '|', '>', ')', ']', '\'', '@',
];

/// The kern `url.sty` puts after every hyphen of a URL, in points.
///
/// Measured with pdflatex (TeX Live 2025, 11pt `article`, T1):
/// `\setbox0=\hbox{\url{a-b}}` is 17.47511pt where `\texttt{a-b}` is
/// 16.97511pt, and `\url{a-b-c}` is 29.29185pt against `\texttt`'s
/// 28.29185pt — exactly 0.5pt per hyphen, and nothing for `/`, `.` or any
/// other character (`\url{x.y}` and `\texttt{x.y}` are both 16.97511pt).
/// Reading the glyph origins back out of the PDF puts the gap immediately
/// *after* the hyphen, in the same `ectt` run.
const URL_HYPHEN_KERN_PT: u8 = 5; // tenths of a point

/// One piece of a literal `\url`/`\nolinkurl` argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UrlPiece<'a> {
    /// A run of URL text, ending at a break character or a hyphen.
    Run(&'a str),
    /// The 0.5pt kern `url.sty` puts after a hyphen (`URL_HYPHEN_KERN_PT`).
    HyphenKern,
}

/// Splits literal `\url`/`\nolinkurl` text into the pieces
/// `P::push_url_text` emits.
///
/// A run ends after a `URL_BREAK_AFTER` character — keeping that character,
/// because url.sty breaks *after* `/`, `.`, ... rather than before — so a
/// long URL can wrap there without any hyphen being inserted. A run also
/// ends after a hyphen, but only so the `HyphenKern` that follows can be
/// placed: a hyphen is deliberately *not* a break in url.sty.
///
/// Concatenating the `Run` pieces reproduces the argument exactly; the kerns
/// carry no text.
fn url_pieces(text: &str) -> Vec<UrlPiece<'_>> {
    let mut pieces = Vec::new();
    let mut start = 0;
    for (index, ch) in text.char_indices() {
        let hyphen = ch == '-';
        if !hyphen && !URL_BREAK_AFTER.contains(&ch) {
            continue;
        }
        let end = index + ch.len_utf8();
        pieces.push(UrlPiece::Run(&text[start..end]));
        if hyphen {
            pieces.push(UrlPiece::HyphenKern);
        }
        start = end;
    }
    if start < text.len() {
        pieces.push(UrlPiece::Run(&text[start..]));
    }
    pieces
}

/// Renders one raw verbatim source line for layout: a tab becomes exactly one
/// space, and for a starred `\verb*`/`verbatim*` every resulting space becomes
/// a middle dot. CRLF line endings are not specially handled; a trailing `\r`
/// is kept as a literal character.
///
/// **Tabs are one space, not a column stop.** `\@vobeytabs` in `latex.ltx`
/// makes TAB active and `\let`s it to `\@xobeytab`, which is `\let` to
/// `\@xobeysp` (`= \nobreakspace = \leavevmode\nobreak\ `); `verbatim*`
/// re-points both through `\@setupverbvisibletab`. Either way a tab is
/// whatever *one* space is — TeX has no tab stops, and nothing in the
/// expansion depends on the current column. pdfTeX 3.141592653-2.6-1.40.27
/// (TeX Live 2025) confirms it: in `\showbox`, `<TAB>one` in `verbatim`
/// yields a single `\glue 5.24995` (cmtt10's `\fontdimen2`, stretch and
/// shrink both 0) — byte-identical to the one-literal-space baseline — and
/// glyph origins in the generated PDF put the first letter at these offsets
/// from the left text edge:
///
/// | source line          | measured | one-space rule | old 8-column rule |
/// |----------------------|----------|----------------|-------------------|
/// | `<TAB>Q`             |  5.23 bp | 5.23 bp (1)    | 41.84 bp (8)      |
/// | `ab<TAB>W`           | 15.69 bp | 15.69 bp (3)   | 41.84 bp (8)      |
/// | `abcdefgh<TAB>R`     | 47.07 bp | 47.07 bp (9)   | 83.99 bp (16)     |
/// | `abc<TAB><TAB>T`     | 26.15 bp | 26.15 bp (5)   | 83.99 bp (16)     |
/// | `<TAB><TAB><TAB>Y`   | 15.69 bp | 15.69 bp (3)   | 125.99 bp (24)    |
///
/// (`abcdefg<TAB>E` at 41.84 bp is the one column where the two rules happen
/// to agree, which is why it alone cannot settle the question.) `verbatim*`
/// measures identically. This function used to expand to the next multiple of
/// 8 columns, which encoded a *text-editor* convention rather than anything
/// pdflatex does.
///
/// **The middle dot is a width-equivalent stand-in.** On pdfTeX
/// `\verbvisiblespace` is `\asciispace` = `\char32`, and cmtt10's slot 32 is
/// `/visiblespace` — U+2423 OPEN BOX. The Core 14 Courier face this crate
/// lays out on has no U+2423 at all, so some substitute is unavoidable. A
/// middle dot is the honest choice because it costs nothing geometrically:
/// Courier's AFM width table is uniform, and `periodcentered` (U+00B7) and
/// `space` (U+0020) both advance 600/1000 em — 6.0 pt at the 10 pt body size
/// — so the substitution moves no glyph. It is also WinAnsi-safe (see
/// `export.rs`) and a widely recognised "visible space" mark on its own.
fn verbatim_display(line: &str, starred: bool) -> String {
    const VISIBLE_SPACE: char = '\u{B7}';
    let mut out = String::with_capacity(line.len());
    for ch in line.chars() {
        match ch {
            '\t' | ' ' => out.push(if starred { VISIBLE_SPACE } else { ' ' }),
            _ => out.push(ch),
        }
    }
    out
}

fn paragraph_style(environment: &str) -> Option<ParagraphStyle> {
    match environment {
        "center" => Some(ParagraphStyle::Center),
        "flushright" => Some(ParagraphStyle::FlushRight),
        "flushleft" => Some(ParagraphStyle::FlushLeft),
        "quote" | "quotation" | "verse" => Some(ParagraphStyle::Quote),
        _ => None,
    }
}

fn environment_end_at(
    tokens: &[InputToken],
    index: usize,
    expected: &str,
) -> Option<(usize, Span)> {
    let command = tokens.get(index)?;
    if !matches!(&command.token.kind, TokenKind::Command(name) if name == "end") {
        return None;
    }
    let mut cursor = index + 1;
    while matches!(
        tokens.get(cursor).map(|input| &input.token.kind),
        Some(TokenKind::Space | TokenKind::Comment)
    ) {
        cursor += 1;
    }
    if !matches!(
        tokens.get(cursor).map(|input| &input.token.kind),
        Some(TokenKind::LBrace)
    ) {
        return None;
    }
    cursor += 1;
    let name = tokens.get(cursor)?;
    if !matches!(&name.token.kind, TokenKind::Word(name) if name == expected) {
        return None;
    }
    cursor += 1;
    let close = tokens.get(cursor)?;
    if close.token.kind != TokenKind::RBrace {
        return None;
    }
    Some((cursor + 1, command.token.span.merge(close.token.span)))
}

/// The environment name of a complete `\begin{name}` / `\end{name}` at
/// `index`, if the token there is one.
fn environment_name_at(tokens: &[InputToken], index: usize) -> Option<&str> {
    let command = tokens.get(index)?;
    if !matches!(&command.token.kind, TokenKind::Command(name) if name == "begin" || name == "end")
    {
        return None;
    }
    let mut cursor = index + 1;
    while matches!(
        tokens.get(cursor).map(|input| &input.token.kind),
        Some(TokenKind::Space | TokenKind::Comment)
    ) {
        cursor += 1;
    }
    let [open, name, close] = tokens.get(cursor..cursor + 3)? else {
        return None;
    };
    match (&open.token.kind, &name.token.kind, &close.token.kind) {
        (TokenKind::LBrace, TokenKind::Word(name), TokenKind::RBrace) => Some(name),
        _ => None,
    }
}

/// Whether the token at `index` ends the paragraph for error recovery, the
/// way TeX's `\par` stops a runaway argument or unterminated math: a blank
/// line, `\par`, or a command that starts a new vertical-mode block (`\item`,
/// a sectioning command, or `\begin`/`\end` of an environment that is not
/// typeset inside math). An unclosed argument, group or math span is closed
/// at this token so the rest of the document is laid out as if it were
/// balanced — one keystroke of half-typed input never reflows later pages.
fn paragraph_boundary_at(tokens: &[InputToken], index: usize) -> bool {
    match tokens.get(index).map(|input| &input.token.kind) {
        Some(TokenKind::ParBreak) => true,
        Some(TokenKind::Command(name)) => match name.as_str() {
            "par" | "item" | "section" | "subsection" => true,
            "begin" | "end" => environment_name_at(tokens, index)
                .is_some_and(|environment| !math::is_math_environment(environment)),
            _ => false,
        },
        _ => false,
    }
}

fn has_document_environment(tokens: &[InputToken]) -> bool {
    document_begin_end(tokens).is_some()
}

/// The end offset of the first `\begin{document}` (past its closing brace).
fn document_begin_end(tokens: &[InputToken]) -> Option<usize> {
    tokens.iter().enumerate().find_map(|(index, input)| {
        if !matches!(&input.token.kind, TokenKind::Command(name) if name == "begin") {
            return None;
        }
        let significant: Vec<&Token> = tokens[index + 1..]
            .iter()
            .map(|input| &input.token)
            .filter(|token| !matches!(token.kind, TokenKind::Space | TokenKind::Comment))
            .take(3)
            .collect();
        match significant.as_slice() {
            [Token {
                kind: TokenKind::LBrace,
                ..
            }, Token {
                kind: TokenKind::Word(name),
                ..
            }, Token {
                kind: TokenKind::RBrace,
                span,
            }] if name == "document" => Some(span.end),
            _ => None,
        }
    })
}

/// `\@fnsymbol` (latex.ltx): `\textasteriskcentered`, `\textdagger`,
/// `\textdaggerdbl`, `\textsection`, `\textparagraph`, `\textbardbl` and
/// the doubled first three; `None` past nine (`\@ctrerr`).
pub(crate) fn fnsymbol(n: u32) -> Option<&'static str> {
    const SYMBOLS: [&str; 9] = [
        "\u{2217}",
        "\u{2020}",
        "\u{2021}",
        "\u{a7}",
        "\u{b6}",
        "\u{2016}",
        "\u{2217}\u{2217}",
        "\u{2020}\u{2020}",
        "\u{2021}\u{2021}",
    ];
    SYMBOLS.get(n.checked_sub(1)? as usize).copied()
}

/// `minipage`, whose footnotes number `mpfootnote` (the environment itself
/// is not implemented: its body is set as running text).
fn is_minipage(environment: &str) -> bool {
    environment == "minipage"
}

/// `\@alph`: 1-26 as a-z; `None` otherwise (`\@ctrerr`).
pub(crate) fn alph(n: u32) -> Option<String> {
    (1..=26)
        .contains(&n)
        .then(|| char::from(b'a' + (n - 1) as u8).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout;

    fn items(source: &str) -> (Parsed, Vec<crate::layout::TextItem>) {
        let parsed = parse(source);
        let items = layout::layout(&parsed.blocks)
            .into_iter()
            .flat_map(|page| page.items)
            .collect();
        (parsed, items)
    }

    fn pages(source: &str) -> (Parsed, Vec<crate::layout::Page>) {
        let parsed = parse(source);
        let pages = layout::layout(&parsed.blocks);
        (parsed, pages)
    }

    #[test]
    fn siunitx_commands_are_inline_formulas_in_text_and_headings() {
        let parsed = parse(
            "\\usepackage[output-decimal-marker={,}]{siunitx}\n\\begin{document}\n\\section{Speed \\qty{3.5}{\\metre\\per\\second}}\nA \\num[group-digits=none]{12345} b.\n\\end{document}\n",
        );
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let math_in = |inlines: &[Inline]| {
            inlines
                .iter()
                .filter_map(|i| match i {
                    Inline::Math { list, display: false, .. } => Some(list.atoms.len()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let mut heading = None;
        let mut paragraph = None;
        for block in &parsed.blocks {
            match block {
                Block::Heading { content, .. } => heading = Some(math_in(content)),
                Block::Paragraph(content) => paragraph = Some(math_in(content)),
                _ => {}
            }
        }
        // 3 , 5 (the braced comma is one Ord group), thin space, m, s^-1
        // with its inter-unit thin space.
        assert_eq!(heading, Some(vec![7]));
        // 1 2 3 4 5: ungrouped digits.
        assert_eq!(paragraph, Some(vec![5]));
    }

    /// The document's packages reach math parsing, so the same `$f\colon A$`
    /// is the kernel's single punctuation atom in an `article` and amsmath's
    /// glue-`:`-glue trio once amsmath is loaded — directly, through a
    /// package that loads it, or through the document class.
    ///
    /// Verified against TeX Live 2025 pdflatex at 10pt: `\hbox{$a\colon b$}`
    /// is 14.02196pt without amsmath and 16.79967pt with it, a 5mu
    /// difference, and `\usepackage{mathtools}` and `\documentclass{amsart}`
    /// both measure the same as `\usepackage{amsmath}`.
    #[test]
    fn math_parsing_takes_the_documents_package_definitions() {
        fn colon_atoms(preamble: &str) -> usize {
            let source =
                format!("{preamble}\\begin{{document}}\n$f\\colon A$\n\\end{{document}}\n");
            let parsed = parse(&source);
            // amsmath itself still reports that it is not implemented as a
            // whole (`package_matches_layout`); what must not appear is any
            // complaint about the formula.
            assert!(
                !parsed
                    .diagnostics
                    .iter()
                    .any(|d| d.message.contains("colon")),
                "{preamble}: {:?}",
                parsed.diagnostics
            );
            let mut found = None;
            for block in &parsed.blocks {
                if let Block::Paragraph(content) = block {
                    for inline in content {
                        if let Inline::Math { list, .. } = inline {
                            found = Some(list.atoms.len());
                        }
                    }
                }
            }
            found.unwrap_or_else(|| panic!("{preamble}: no formula"))
        }

        // `f`, the colon, `A`.
        assert_eq!(colon_atoms("\\documentclass{article}\n"), 3);
        // `f`, 2mu, the colon, 6mu, `A`.
        for preamble in [
            "\\documentclass{article}\n\\usepackage{amsmath}\n",
            "\\documentclass{article}\n\\usepackage{mathtools}\n",
            "\\documentclass{article}\n\\usepackage{amssymb,amsmath}\n",
            "\\documentclass{amsart}\n",
        ] {
            assert_eq!(colon_atoms(preamble), 5, "{preamble}");
        }
        // A package that does not load amsmath leaves the kernel's definition.
        assert_eq!(
            colon_atoms("\\documentclass{article}\n\\usepackage{amsthm}\n"),
            3
        );
    }

    #[test]
    fn dimen_parsing_supports_the_common_units() {
        assert_eq!(parse_dimen_pt("12pt"), Some(12.0));
        assert_eq!(parse_dimen_pt(" 1em "), Some(crate::layout::BODY_SIZE_PT));
        assert_eq!(parse_dimen_pt("1in"), Some(72.27));
        assert_eq!(parse_dimen_pt("-.5in"), Some(-0.5 * 72.27));
        assert_eq!(parse_dimen_pt("=6in"), Some(6.0 * 72.27));
        assert_eq!(parse_dimen_pt("\\textwidth"), Some(0.0));
        assert_eq!(parse_dimen_pt("0.5\\textwidth"), Some(0.0));
        assert!(parse_dimen_pt("banana").is_none());
        assert!(parse_dimen_pt("").is_none());
        // TeXbook Appendix B / scan_dimen §458 ratios, in TeX points.
        assert_eq!(parse_dimen_pt("1bp"), Some(72.27 / 72.0));
        assert_eq!(parse_dimen_pt("1dd"), Some(1238.0 / 1157.0));
        assert_eq!(parse_dimen_pt("1cc"), Some(14856.0 / 1157.0));
        assert_eq!(parse_dimen_pt("1sp"), Some(1.0 / 65536.0));
        assert_eq!(parse_dimen_pt("1mm"), Some(72.27 / 25.4));
        assert_eq!(parse_dimen_pt("1cm"), Some(72.27 / 2.54));
        // `ex` uses cmr's x-height/em (same constant as ulem); the compiler
        // has no TFM metrics, unlike the pipeline's `ec_em_ex`.
        assert_eq!(
            parse_dimen_pt("1ex"),
            Some(crate::layout::BODY_SIZE_PT * CMR_EX_PER_EM)
        );
    }

    #[test]
    fn newpage_forces_a_fresh_page_even_with_room_left() {
        let (parsed, pages) = pages(r"First page\newpage Second page");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert_eq!(pages.len(), 2, "expected exactly one forced page break");
        assert!(pages[0].items.iter().any(|item| item.text == "First"));
        assert!(pages[1].items.iter().any(|item| item.text == "Second"));
    }

    #[test]
    fn hrule_emits_a_full_measure_rule_with_a_real_span() {
        let source = r"Above\hrule Below";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let rule_item = items
            .iter()
            .find(|item| item.rule.is_some())
            .expect("hrule must emit an item carrying rule geometry");
        let rule = rule_item.rule.unwrap();
        assert!(rule.width_pt > 0.0);
        assert!(rule.height_pt > 0.0);
        assert_eq!(
            rule_item.span,
            Span::new(
                source.find("\\hrule").unwrap(),
                source.find("\\hrule").unwrap() + "\\hrule".len()
            )
        );
    }

    #[test]
    fn vspace_adds_extra_gap_beyond_the_ordinary_paragraph_gap() {
        let baseline = items("One\n\nTwo").1;
        let spaced = items(r"One\vspace{50pt}Two").1;
        let one = baseline.iter().find(|i| i.text == "One").unwrap();
        let two_baseline = baseline.iter().find(|i| i.text == "Two").unwrap();
        let two_spaced = spaced.iter().find(|i| i.text == "Two").unwrap();
        assert!(
            two_spaced.baseline_y_pt - one.baseline_y_pt
                > two_baseline.baseline_y_pt - one.baseline_y_pt,
            "\\vspace{{50pt}} should push the following text further down than an ordinary paragraph break"
        );
    }

    #[test]
    fn run_in_headings_are_accepted_and_keep_their_title_as_body_text() {
        // `\@xsect`'s negative-after-skip branch sets the head into the
        // following paragraph's first line, so the title belongs in the body
        // text stream exactly where it stands. The render pipeline reads the
        // command back from the source there and gives it its weight, indent
        // and `\hskip 1em`; this layer must only stop erroring, take the star
        // and the optional short title, and leave the title alone.
        // `\paragraph*[short]{...}` is not in the list: `\@startsection`'s
        // starred form takes no optional argument, and pdflatex itself
        // typesets the brackets there (`[Short]Solution.`), so there is no
        // oracle behaviour to match.
        for source in [
            r"\paragraph{Solution.} Body text.",
            r"\subparagraph{Solution.} Body text.",
            r"\paragraph*{Solution.} Body text.",
            r"\paragraph[Short]{Solution.} Body text.",
            r"\subparagraph*{Solution.} Body text.",
        ] {
            let parsed = parse(source);
            assert!(parsed.diagnostics.is_empty(), "{source}: {:?}", parsed.diagnostics);
            let text: String = items(source).1.iter().map(|i| i.text.as_str()).collect::<Vec<_>>().join(" ");
            assert!(text.contains("Solution."), "{source}: title kept as body text, got {text:?}");
            assert!(text.contains("Body"), "{source}: body text kept, got {text:?}");
            // The star and the short title are the command's parameters, not
            // prose: neither may reach the page.
            assert!(!text.contains('*'), "{source}: the star is not set, got {text:?}");
            assert!(!text.contains("Short"), "{source}: the short title is not set, got {text:?}");
        }
    }

    #[test]
    fn pagestyle_is_accepted_without_a_diagnostic() {
        for style in ["empty", "plain", "headings"] {
            let parsed = parse(&format!(r"\pagestyle{{{style}}}Body text"));
            assert!(
                parsed.diagnostics.is_empty(),
                "\\pagestyle{{{style}}}: {:?}",
                parsed.diagnostics
            );
        }
    }

    #[test]
    fn thispagestyle_is_accepted_without_a_diagnostic() {
        for style in ["empty", "plain"] {
            let parsed = parse(&format!(r"\thispagestyle{{{style}}}Body text"));
            assert!(
                parsed.diagnostics.is_empty(),
                "\\thispagestyle{{{style}}}: {:?}",
                parsed.diagnostics
            );
        }
    }

    #[test]
    fn pagenumbering_is_accepted_without_a_diagnostic() {
        for style in ["arabic", "roman"] {
            let parsed = parse(&format!(r"\pagenumbering{{{style}}}Body text"));
            assert!(
                parsed.diagnostics.is_empty(),
                "\\pagenumbering{{{style}}}: {:?}",
                parsed.diagnostics
            );
        }
    }

    #[test]
    fn clearpage_and_cleardoublepage_force_a_fresh_page() {
        for command in [r"\clearpage", r"\cleardoublepage"] {
            let (parsed, pages) = pages(&format!("First page{command} Second page"));
            assert!(
                parsed.diagnostics.is_empty(),
                "{command}: {:?}",
                parsed.diagnostics
            );
            assert_eq!(
                pages.len(),
                2,
                "{command} must force exactly one page break"
            );
            assert!(pages[0].items.iter().any(|item| item.text == "First"));
            assert!(pages[1].items.iter().any(|item| item.text == "Second"));
        }
    }

    #[test]
    fn pagebreak_at_default_or_explicit_priority_four_forces_a_fresh_page() {
        for command in [r"\pagebreak", r"\pagebreak[4]"] {
            let (parsed, pages) = pages(&format!("First page{command} Second page"));
            assert!(
                parsed.diagnostics.is_empty(),
                "{command}: {:?}",
                parsed.diagnostics
            );
            assert_eq!(
                pages.len(),
                2,
                "{command} must force exactly one page break"
            );
            assert!(pages[0].items.iter().any(|item| item.text == "First"));
            assert!(pages[1].items.iter().any(|item| item.text == "Second"));
        }
    }

    #[test]
    fn pagebreak_below_priority_four_is_a_no_op_hint() {
        let (parsed, pages) = pages(r"First page\pagebreak[1] Second page");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert_eq!(
            pages.len(),
            1,
            "a priority below 4 is only a hint this compiler has no badness model to honour"
        );
        assert!(
            pages[0].items.iter().all(|item| item.text != "[1]"),
            "the priority argument must not leak onto the page as text"
        );
    }

    #[test]
    fn nopagebreak_is_accepted_without_a_diagnostic_and_has_no_visible_effect() {
        for command in [r"\nopagebreak", r"\nopagebreak[3]"] {
            let with = pages(&format!("First page{command} Second page"));
            assert!(
                with.0.diagnostics.is_empty(),
                "{command}: {:?}",
                with.0.diagnostics
            );
            assert_eq!(with.1.len(), 1);
            assert!(with.1[0].items.iter().all(|item| item.text != "[3]"));
        }
    }

    #[test]
    fn linebreak_at_default_or_explicit_priority_four_starts_a_new_line() {
        for command in [r"\linebreak", r"\linebreak[4]"] {
            let (parsed, items) = items(&format!("AAA{command} BBB"));
            assert!(
                parsed.diagnostics.is_empty(),
                "{command}: {:?}",
                parsed.diagnostics
            );
            let aaa = items.iter().find(|i| i.text == "AAA").unwrap();
            let bbb = items.iter().find(|i| i.text == "BBB").unwrap();
            assert!(
                bbb.baseline_y_pt > aaa.baseline_y_pt,
                "{command} must move to a new line, not just insert a space"
            );
            assert_eq!(
                bbb.x_pt,
                layout::MARGIN_PT,
                "{command} must return to the left margin on its new line"
            );
        }
    }

    #[test]
    fn linebreak_below_priority_four_is_a_no_op_hint() {
        let (parsed, items) = items(r"AAA\linebreak[1] BBB");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let aaa = items.iter().find(|i| i.text == "AAA").unwrap();
        let bbb = items.iter().find(|i| i.text == "BBB").unwrap();
        assert_eq!(
            bbb.baseline_y_pt, aaa.baseline_y_pt,
            "a priority below 4 must not force a line break"
        );
        assert!(items.iter().all(|item| item.text != "[1]"));
    }

    #[test]
    fn nolinebreak_is_accepted_without_a_diagnostic_and_has_no_visible_effect() {
        for command in [r"\nolinebreak", r"\nolinebreak[2]"] {
            let (parsed, items) = items(&format!("AAA{command} BBB"));
            assert!(
                parsed.diagnostics.is_empty(),
                "{command}: {:?}",
                parsed.diagnostics
            );
            let aaa = items.iter().find(|i| i.text == "AAA").unwrap();
            let bbb = items.iter().find(|i| i.text == "BBB").unwrap();
            assert_eq!(bbb.baseline_y_pt, aaa.baseline_y_pt);
            assert!(items.iter().all(|item| item.text != "[2]"));
        }
    }

    #[test]
    fn vspace_star_behaves_exactly_like_unstarred_vspace() {
        let unstarred = items(r"One\vspace{50pt}Two").1;
        let (parsed, starred) = items(r"One\vspace*{50pt}Two");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let positions = |items: &[crate::layout::TextItem]| -> Vec<(String, f64, f64)> {
            items
                .iter()
                .map(|item| (item.text.clone(), item.x_pt, item.baseline_y_pt))
                .collect()
        };
        assert_eq!(
            positions(&unstarred),
            positions(&starred),
            "\\vspace* must lay out identically to \\vspace on this non-breaking layout"
        );
    }

    #[test]
    fn vfill_consumes_the_rest_of_the_page_pushing_what_follows_to_a_new_page() {
        let baseline = pages("Top.\n\nBottom.").1;
        assert_eq!(
            baseline.len(),
            1,
            "two short paragraphs alone must fit on one page"
        );
        let (parsed, filled) = pages(r"Top.\vfill Bottom.");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert_eq!(
            filled.len(),
            2,
            "\\vfill should consume the remaining room on the page, pushing what follows onto a new one"
        );
        assert!(filled[0].items.iter().any(|item| item.text == "Top."));
        assert!(filled[1].items.iter().any(|item| item.text == "Bottom."));
    }

    #[test]
    fn indent_is_named_honestly_since_first_line_indentation_is_not_implemented() {
        let (parsed, items) = items(r"\indent Indented paragraph");
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|d| d.message.contains("\\indent") && d.message.contains("not implemented")),
            "{:?}",
            parsed.diagnostics
        );
        assert!(
            items.iter().any(|item| item.text == "Indented"),
            "the paragraph text must still be typeset even though the indent itself is not"
        );
    }

    #[test]
    fn preamble_is_recorded_and_only_document_body_is_typeset() {
        let source = "\\documentclass[draft]{article}\n\\usepackage[demo]{amsmath}\n\\begin{document}Body only\\end{document}trailer";
        let (parsed, items) = items(source);
        assert_eq!(parsed.document_class.as_deref(), Some("article"));
        assert_eq!(parsed.packages, ["amsmath"]);
        assert_eq!(
            items
                .iter()
                .map(|item| item.text.as_str())
                .collect::<Vec<_>>(),
            ["Body", "only"]
        );
        assert_eq!(parsed.diagnostics.len(), 1);
        assert!(parsed.diagnostics[0].message.contains("amsmath"));
    }

    #[test]
    fn starred_subsection_consumes_its_star_and_does_not_advance_numbering() {
        let parsed = parse(r"\section{One}\subsection*{Aside}\subsection{Two}");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let numbers: Vec<&str> = parsed
            .blocks
            .iter()
            .filter_map(|block| match block {
                Block::Heading { number, .. } => Some(number.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(numbers, ["1", "", "1.1"]);
    }

    #[test]
    fn problem_style_macro_and_font_declarations_preserve_content_without_errors() {
        let source = r"\newcommand{\problem}[2]{\subsection*{Problem #1 \hfill \normalfont[#2 points]}}\problem{1}{4}{\bfseries Body}";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert!(items.iter().any(|item| item.text == "Problem"));
        assert!(items.iter().any(|item| item.text == "Body"));
        assert!(!items.iter().any(|item| item.text == "*"));
    }

    /// audit A8: inline math must not gain an inter-word gap the source
    /// never had, on either side of `$...$`.
    #[test]
    fn math_glued_to_following_punctuation_has_no_gap() {
        let (parsed, glued) = items("$x$.");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let (parsed, spaced) = items("$x$ .");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let glued_period = glued.iter().find(|i| i.text == ".").unwrap();
        let spaced_period = spaced.iter().find(|i| i.text == ".").unwrap();
        let space = layout::word_space(layout::BODY_SIZE_PT, layout::Font::TimesRoman);
        assert!(
            (spaced_period.x_pt - glued_period.x_pt - space).abs() < 0.01,
            "expected `$x$ .` to sit exactly one word space right of `$x$.`: {} vs {}",
            spaced_period.x_pt,
            glued_period.x_pt
        );
    }

    #[test]
    fn math_followed_by_a_real_space_keeps_exactly_one_word_space() {
        let (parsed, spaced) = items("$x$ y");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let (parsed, glued) = items("$x$y");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let spaced_y = spaced.iter().find(|i| i.text == "y").unwrap();
        let glued_y = glued.iter().find(|i| i.text == "y").unwrap();
        let space = layout::word_space(layout::BODY_SIZE_PT, layout::Font::TimesRoman);
        assert!(
            (spaced_y.x_pt - glued_y.x_pt - space).abs() < 0.01,
            "expected `$x$ y` to sit exactly one word space right of `$x$y`: {} vs {}",
            spaced_y.x_pt,
            glued_y.x_pt
        );
    }

    #[test]
    fn text_followed_by_a_real_space_before_math_keeps_exactly_one_word_space() {
        let (parsed, spaced) = items("a $x$");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let (parsed, glued) = items("a$x$");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let spaced_x = spaced.iter().find(|i| i.text == "x").unwrap();
        let glued_x = glued.iter().find(|i| i.text == "x").unwrap();
        let space = layout::word_space(layout::BODY_SIZE_PT, layout::Font::TimesRoman);
        assert!(
            (spaced_x.x_pt - glued_x.x_pt - space).abs() < 0.01,
            "expected `a $x$` to sit exactly one word space right of `a$x$`: {} vs {}",
            spaced_x.x_pt,
            glued_x.x_pt
        );
    }

    /// audit A9: a control *word* swallows the whitespace that follows it
    /// (real TeX's "skip blanks" state), so `\normalfont 4` and
    /// `\normalfont4` must typeset identically. Exercised through
    /// `\section{...}` content, the same `inlines_from_tokens` path used by
    /// `\problem`-style macro bodies like `\normalfont[#2 points]`.
    #[test]
    fn normalfont_swallows_its_following_space() {
        let (parsed, spaced) = items("\\section{X\\normalfont 4}");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let (parsed, glued) = items("\\section{X\\normalfont4}");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let spaced_four = spaced.iter().find(|i| i.text == "4").unwrap();
        let glued_four = glued.iter().find(|i| i.text == "4").unwrap();
        assert_eq!(
            spaced_four.x_pt, glued_four.x_pt,
            "the space after \\normalfont must not shift what follows it"
        );
    }

    /// `\ ` is a control *symbol* (an escaped literal space), not a control
    /// word, so it is never swallowed; `\\` is the unrelated line-break
    /// token. Neither is affected by the control-word space-swallow rule.
    #[test]
    fn control_space_and_linebreak_are_not_swallowed() {
        let toks = tokenize("x\\ y");
        assert_eq!(toks[0].kind, TokenKind::Word("x".into()));
        assert_eq!(toks[1].kind, TokenKind::Word(" ".into()));
        assert_eq!(toks[2].kind, TokenKind::Word("y".into()));

        let toks = tokenize("x\\\\ y");
        assert_eq!(toks[0].kind, TokenKind::Word("x".into()));
        assert_eq!(toks[1].kind, TokenKind::LineBreak);
        assert_eq!(toks[2].kind, TokenKind::Space);
        assert_eq!(toks[3].kind, TokenKind::Word("y".into()));
    }

    #[test]
    fn tex_input_ligatures_convert_in_ordinary_text() {
        // The exact shape found in fixtures/real-world/hw1/HW1.tex: a ligature
        // pair straddling a word boundary and one embedded inside a single
        // compound word with no surrounding whitespace.
        let source = "``Quoted'' and a turn---after dash, don't stop.\n";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let texts: Vec<&str> = items.iter().map(|item| item.text.as_str()).collect();
        assert!(texts.contains(&"\u{201C}Quoted\u{201D}"), "{texts:?}");
        assert!(texts.contains(&"turn\u{2014}after"), "{texts:?}");
        assert!(texts.iter().any(|t| t.contains('\u{2019}')), "{texts:?}");
    }

    #[test]
    fn tex_input_ligatures_convert_in_headings_and_text_style_arguments() {
        let source = "\\section{Notes---Continued}\n\\textbf{can't---won't}\n";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let texts: Vec<&str> = items.iter().map(|item| item.text.as_str()).collect();
        assert!(texts.iter().any(|t| t.contains('\u{2014}')), "{texts:?}");
        assert!(
            texts.iter().any(|t| t.contains('\u{2019}')),
            "expected a converted apostrophe in {texts:?}"
        );
    }

    #[test]
    fn tex_input_ligatures_never_apply_inside_math() {
        // Math is parsed through an entirely separate path (`math::parse_tokens`)
        // that this function is never wired into; a literal double-hyphen inside
        // `$...$` must stay two separate math minus signs, never an en dash.
        let source = "Text. $a--b$ more text.\n";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert!(!items.iter().any(|item| item.text.contains('\u{2013}')));
        assert!(!items.iter().any(|item| item.text.contains('\u{2014}')));
        assert_eq!(
            items
                .iter()
                .filter(|item| item.text == crate::math::MINUS_SIGN)
                .count(),
            2
        );
    }

    #[test]
    fn zero_argument_macro_maps_literal_output_to_invocation() {
        let source = "\\newcommand{\\hi}{Hello} \\hi";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty());
        let hello = items.iter().find(|item| item.text == "Hello").unwrap();
        let start = source.rfind("\\hi").unwrap();
        assert_eq!(hello.span, Span::new(start, start + "\\hi".len()));
    }

    #[test]
    fn macro_argument_keeps_argument_source_span() {
        let source = "\\newcommand{\\greet}[1]{Hello #1} \\greet{world}";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty());
        assert!(items.iter().any(|item| item.text == "Hello"));
        let world = items.iter().find(|item| item.text == "world").unwrap();
        let start = source.rfind("world").unwrap();
        assert_eq!(world.span, Span::new(start, start + "world".len()));
    }

    #[test]
    fn nested_macros_expand_and_renewcommand_replaces_an_existing_macro() {
        // (`\outer` would be a TeX primitive, which `\newcommand` refuses.)
        let source = r"\newcommand{\inner}{first} \newcommand{\wrapper}{\inner} \wrapper \renewcommand{\inner}{second} \wrapper";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty());
        assert_eq!(
            items
                .iter()
                .map(|item| item.text.as_str())
                .collect::<Vec<_>>(),
            ["first", "second"]
        );
    }

    #[test]
    fn invalid_newcommand_and_renewcommand_relationships_are_diagnostic() {
        let source =
            r"\newcommand{\same}{old}\newcommand{\same}{new}\renewcommand{\missing}{body}\same";
        let (parsed, items) = items(source);
        assert_eq!(
            items
                .iter()
                .map(|item| item.text.as_str())
                .collect::<Vec<_>>(),
            ["old"]
        );
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains(r"LaTeX Error: Command \same already defined.")));
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains(r"LaTeX Error: Command \missing undefined.")));
    }

    #[test]
    fn macros_expand_inside_supported_command_arguments() {
        let source = r"\newcommand{\titleword}{Title}\section{\titleword}";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].text, "Title");
        let start = source.rfind(r"\titleword").unwrap();
        assert_eq!(items[0].span, Span::new(start, start + r"\titleword".len()));
    }

    #[test]
    fn macro_expansion_in_math_does_not_fabricate_per_glyph_spans() {
        let source = r"\newcommand{\pair}{abcde} $\pair$";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty());
        let invocation_start = source.rfind(r"\pair").unwrap();
        let invocation_span = Span::new(invocation_start, invocation_start + r"\pair".len());
        let math_items: Vec<_> = items
            .iter()
            .filter(|item| "abcde".contains(item.text.as_str()))
            .collect();
        assert_eq!(math_items.len(), 5);
        assert!(
            math_items.iter().all(|item| item.span == invocation_span),
            "expected {invocation_span:?}, got {:?}",
            math_items.iter().map(|item| item.span).collect::<Vec<_>>()
        );
    }

    #[test]
    fn group_local_macro_is_restored_when_the_group_closes() {
        let source = "{\\newcommand{\\local}{inside} \\local} \\local";
        let (parsed, items) = items(source);
        assert_eq!(items.iter().filter(|item| item.text == "inside").count(), 1);
        assert!(parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("\\local is not supported")));
    }

    #[test]
    fn self_referential_macro_hits_explicit_recursion_limit() {
        let source = "\\newcommand{\\loop}{\\loop} \\loop";
        let (parsed, _) = items(source);
        // The expansion pass bounds runaway expansion by its step limit.
        assert!(parsed.diagnostics.iter().any(|diagnostic| {
            diagnostic.message.contains("expansion step limit exceeded")
        }));
    }

    #[test]
    fn gather_star_rows_are_math_with_no_diagnostics() {
        let source = "\\documentclass{article}\\begin{document}\n\\begin{gather*}\\int_{0}^{\\infty} e^{-x^{2}}\\,dx = \\frac{\\sqrt{\\pi}}{2} \\\\ \\sum_{n=1}^{\\infty}\\frac{1}{n^{2}} = \\frac{\\pi^{2}}{6}\\end{gather*}\n\\end{document}";
        let (parsed, items) = items(source);
        let errors: Vec<_> = parsed
            .diagnostics
            .iter()
            .filter(|d| d.severity == crate::diagnostics::Severity::Error)
            .collect();
        assert!(errors.is_empty(), "{errors:?}");
        let Block::Paragraph(inlines) = &parsed.blocks[0] else {
            panic!("expected a paragraph");
        };
        let Inline::MathRows { rows, aligned, .. } = &inlines[0] else {
            panic!("expected multi-row math, got {inlines:?}");
        };
        assert!(!aligned);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.number.is_none()));
        let texts: Vec<_> = items.iter().map(|i| i.text.as_str()).collect();
        for glyph in ["∫", "∑", "π", "∞"] {
            assert!(texts.contains(&glyph), "{glyph} missing from {texts:?}");
        }
        assert!(!texts.contains(&","), "\\, must be spacing, not a comma");
        let int_y = items.iter().find(|i| i.text == "∫").unwrap().baseline_y_pt;
        let sum_y = items.iter().find(|i| i.text == "∑").unwrap().baseline_y_pt;
        assert!(sum_y > int_y, "second row must sit below the first");
    }

    #[test]
    fn align_shares_tab_stop_and_numbers_rows() {
        let source = "\\begin{align}x^{2} &= y \\label{a}\\\\ 2xyz &= 1 \\nonumber\\\\ w &= 3\\\\\\end{align}\\ref{a}";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let equals: Vec<_> = items.iter().filter(|i| i.text == "=").collect();
        assert_eq!(equals.len(), 3);
        assert!(equals
            .iter()
            .all(|i| (i.x_pt - equals[0].x_pt).abs() < 0.01));
        let numbers: Vec<_> = items
            .iter()
            .filter(|i| i.text.starts_with('('))
            .map(|i| i.text.as_str())
            .collect();
        assert_eq!(numbers, ["(1)", "(2)"]);
        let Block::Paragraph(inlines) = &parsed.blocks[0] else {
            panic!("expected a paragraph");
        };
        assert!(inlines.iter().any(
            |inline| matches!(inline, Inline::Label { key, value, .. } if key == "a" && value == "1")
        ));
    }

    #[test]
    fn intertext_is_set_between_align_rows() {
        let source = "\\begin{align} a &= b \\\\ \\intertext{so that} c &= d \\shortintertext{and} e &= f \\end{align}";
        let parsed = parse(source);
        let rows = parsed
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::Paragraph(inlines) => Some(inlines),
                _ => None,
            })
            .flatten()
            .find_map(|i| match i {
                Inline::MathRows { rows, .. } => Some(rows),
                _ => None,
            })
            .expect("align rows");
        // `\\ \intertext` does not start an empty row; `\shortintertext`
        // after material ends the row first.
        assert_eq!(rows.len(), 3);
        assert!(rows[0].intertext.is_empty());
        assert_eq!(
            rows.iter().map(|r| r.number.as_deref()).collect::<Vec<_>>(),
            [Some("1"), Some("2"), Some("3")]
        );
        let texts: Vec<(bool, String)> = rows[1..]
            .iter()
            .flat_map(|r| &r.intertext)
            .map(|t| {
                let words: Vec<&str> = t
                    .content
                    .iter()
                    .filter_map(|i| match i {
                        Inline::Text { text, .. } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect();
                (t.short, words.join(" "))
            })
            .collect();
        assert_eq!(
            texts,
            [(false, "so that".to_string()), (true, "and".to_string())]
        );
    }

    #[test]
    fn equation_star_is_unnumbered_display_math() {
        let (parsed, items) = items("\\begin{equation*}a=b\\end{equation*}");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert_eq!(
            items.iter().map(|i| i.text.as_str()).collect::<Vec<_>>(),
            ["a", "=", "b"]
        );
    }

    #[test]
    fn math_grid_environments_lay_out_cells_in_rows_and_columns() {
        let source = "\\[ f = \\begin{cases} x & x \\geq 0 \\\\ -y & y < 0 \\end{cases} \\]\n\\begin{gather*}\\begin{pmatrix} 1 & 2 \\\\ 3 & 4 \\end{pmatrix}\\end{gather*}\n\\[\\begin{array}{rl} a & b,\\\\[2pt] cc & d \\end{array}\\]";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let at = |text: &str| items.iter().find(|i| i.text == text).unwrap();
        // cases: left brace only, two rows, second column shared.
        assert!(items.iter().any(|i| i.text == "{"));
        assert!(at("≥").baseline_y_pt < at("<").baseline_y_pt);
        // pmatrix: fences and a 2x2 grid.
        assert!(items.iter().any(|i| i.text == "(") && items.iter().any(|i| i.text == ")"));
        assert_eq!(at("1").baseline_y_pt, at("2").baseline_y_pt);
        assert_eq!(at("1").x_pt, at("3").x_pt);
        assert!(at("3").baseline_y_pt > at("1").baseline_y_pt);
        // array {rl}: right-aligned first column, `[2pt]` consumed.
        assert!(!items.iter().any(|i| i.text == "p" || i.text == "t"));
        let a = at("a");
        let cs: Vec<_> = items.iter().filter(|i| i.text == "c").collect();
        assert!(a.x_pt > cs[0].x_pt, "right-aligned column");
        assert_eq!(at("b").x_pt, at("d").x_pt);
    }

    #[test]
    fn alignment_declarations_are_group_scoped_and_read_at_paragraph_end() {
        let styles = |source: &str| {
            let parsed = parse(source);
            assert!(
                !parsed
                    .diagnostics
                    .iter()
                    .any(|d| d.message.contains("not supported")
                        || d.message.contains("ragged")
                        || d.message.contains("centering")),
                "{:?}",
                parsed.diagnostics
            );
            parsed
                .blocks
                .iter()
                .map(|block| match block {
                    Block::Styled { style, .. } => Some(*style),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        use ParagraphStyle::{Center, FlushLeft, FlushRight, Quote};
        assert_eq!(
            styles("{\\centering Title\\par} After."),
            [Some(Center), None]
        );
        assert_eq!(
            styles("{\\raggedright Ragged\\par}\n\n{\\raggedleft Left\\par}\n\nPlain."),
            [Some(FlushLeft), Some(FlushRight), None]
        );
        // The group closed before the paragraph ended, so (as in TeX) the
        // declaration no longer applies to it.
        assert_eq!(styles("{\\centering Early} close.\n\nNext."), [None, None]);
        // Scope ends at `\end`; environments that end their paragraph do so
        // with their own declaration still in force.
        assert_eq!(
            styles("\\begin{figure}\\centering Body\\end{figure}\nAfter."),
            [Some(Center), None]
        );
        assert_eq!(
            styles("\\raggedleft\\begin{center}\\RaggedRight Inner\\end{center}\nOuter."),
            [Some(FlushLeft), Some(FlushRight)]
        );
        assert_eq!(
            styles("\\centering\\begin{center}Env\\end{center}\n\\begin{quote}Q\\end{quote}"),
            [Some(Center), Some(Quote)]
        );
        assert_eq!(
            styles("\\documentclass{article}\n\\raggedright\n\\begin{document}\nText.\n\\end{document}"),
            [Some(FlushLeft)]
        );

        // A declared paragraph lays out exactly like its environment form,
        // i.e. it is not justified.
        let words = "Ragged text keeps its natural spaces here. ".repeat(6);
        let positions = |source: String| {
            items(&source)
                .1
                .iter()
                .map(|item| item.x_pt)
                .collect::<Vec<_>>()
        };
        let declared = positions(format!("{{\\raggedright {words}\\par}}"));
        assert_eq!(
            declared,
            positions(format!("\\begin{{flushleft}}{words}\\end{{flushleft}}"))
        );
        assert_ne!(declared, positions(words.clone()));
    }

    #[test]
    fn center_and_quote_align_their_paragraphs() {
        let source = "Plain.\n\\begin{center}Title\\\\[3pt]Subtitle words\\end{center}\n\\begin{quote}Quoted.\\end{quote}\nAfter.";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert!(matches!(
            parsed.blocks[1],
            Block::Styled {
                style: ParagraphStyle::Center,
                ..
            }
        ));
        let at = |text: &str| items.iter().find(|i| i.text == text).unwrap();
        assert!(!items.iter().any(|i| i.text.contains("3pt")));
        let page_centre = crate::layout::PAGE_WIDTH_PT / 2.0;
        assert!((at("Title").x_pt - page_centre).abs() < 40.0);
        assert!(at("Subtitle").x_pt > crate::layout::MARGIN_PT + 100.0);
        assert_eq!(
            at("Quoted.").x_pt,
            crate::layout::MARGIN_PT + crate::layout::QUOTE_INDENT_PT
        );
        assert_eq!(at("After.").x_pt, crate::layout::MARGIN_PT);
        assert_eq!(at("Plain.").x_pt, crate::layout::MARGIN_PT);
    }

    #[test]
    fn numberwithin_and_subequations_number_like_amsmath() {
        // pdflatex (display-placement fixtures 17 and 18): (1.1), (2.1), a
        // subequations block (2.2a)-(2.2c) whose leading \label is 2.2, then
        // (2.3).
        let source = r"\numberwithin{equation}{section}\section{A}\begin{equation}a\label{a}\end{equation}\section{B}\begin{equation}b\end{equation}\begin{subequations}\label{sub}\begin{align}c\label{c}\\d\end{align}\begin{equation}e\label{e}\end{equation}\end{subequations}\begin{equation}f\label{f}\end{equation}";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let numbers: Vec<_> = items
            .iter()
            .filter(|i| i.text.starts_with('('))
            .map(|i| i.text.as_str())
            .collect();
        assert_eq!(numbers, ["(1.1)", "(2.1)", "(2.2a)", "(2.2b)", "(2.2c)", "(2.3)"]);
        let labels: Vec<_> = parsed
            .blocks
            .iter()
            .flat_map(|block| match block {
                Block::Paragraph(inlines) => inlines.as_slice(),
                _ => &[],
            })
            .filter_map(|inline| match inline {
                Inline::Label { key, value, .. } => Some((key.as_str(), value.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(
            labels,
            [("a", "1.1"), ("sub", "2.2"), ("c", "2.2a"), ("e", "2.2c"), ("f", "2.3")]
        );
    }

    #[test]
    fn numberwithin_reports_unknown_counters_and_formats() {
        // `[\roman]` reaches the parser as a format name (the expansion pass
        // renames it so the engine's `\roman` does not read `]`).
        let (parsed, items) = items(r"\numberwithin{equation}{chapter}\numberwithin[\textbf]{figure}{section}\numberwithin[\roman]{equation}{section}\section{S}\begin{equation}x\end{equation}");
        let messages: Vec<_> = parsed.diagnostics.iter().map(|d| d.message.as_str()).collect();
        assert!(messages.iter().any(|m| m.contains("No counter 'chapter' defined")), "{messages:?}");
        assert!(messages.iter().any(|m| m.contains("'\\textbf' is not \\arabic")), "{messages:?}");
        assert_eq!(messages.len(), 2, "{messages:?}");
        let texts: Vec<_> = items.iter().map(|i| i.text.as_str()).collect();
        assert!(texts.contains(&"(1.i)"), "{texts:?}");
    }

    #[test]
    fn only_numbered_displays_print_numbers_and_advance_the_counter() {
        let source = "\\[a\\] $$b$$ \\begin{displaymath}c\\end{displaymath}\\begin{equation*}d\\end{equation*}\\begin{gather*}e\\end{gather*}\\begin{align*}f&=g\\end{align*}\\begin{equation}h\\label{h}\\end{equation}\\begin{align}i\\nonumber\\\\j\\label{j}\\end{align}";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let numbers: Vec<_> = items
            .iter()
            .filter(|i| i.text.starts_with('('))
            .map(|i| i.text.as_str())
            .collect();
        assert_eq!(numbers, ["(1)", "(2)"]);
        let labels: Vec<_> = parsed
            .blocks
            .iter()
            .flat_map(|block| match block {
                Block::Paragraph(inlines) => inlines.as_slice(),
                _ => &[],
            })
            .filter_map(|inline| match inline {
                Inline::Label { key, value, .. } => Some((key.as_str(), value.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(labels, [("h", "1"), ("j", "2")]);
    }

    #[test]
    fn hfill_right_flushes_a_problem_style_subsection_header() {
        // The exact HW1 shape: `\hfill` inside a starred subsection built by
        // a user macro, which routes through `inlines_from_tokens` rather
        // than `command`'s ordinary dispatch.
        let source = r"\newcommand{\problem}[2]{\subsection*{Problem #1 \hfill \normalfont[#2 points]}}\problem{1}{4}";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let problem = items.iter().find(|i| i.text == "Problem").unwrap();
        let points = items.iter().find(|i| i.text == "points]").unwrap();
        assert_eq!(problem.x_pt, layout::MARGIN_PT);
        let points_width = layout::text_width("points]", points.font_size_pt, points.font);
        assert!(
            (points.x_pt + points_width - (layout::PAGE_WIDTH_PT - layout::MARGIN_PT)).abs() < 0.5,
            "expected 'points]' flushed to the right margin, got x_pt={} width={}",
            points.x_pt,
            points_width
        );
    }

    #[test]
    fn multiple_hfills_on_one_line_share_the_leftover_space_equally() {
        let (parsed, items) = items(r"A \hfill B \hfill C");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let a = items.iter().find(|i| i.text == "A").unwrap();
        let b = items.iter().find(|i| i.text == "B").unwrap();
        let c = items.iter().find(|i| i.text == "C").unwrap();
        assert_eq!(a.x_pt, layout::MARGIN_PT);
        let c_width = layout::text_width("C", c.font_size_pt, c.font);
        assert!(
            (c.x_pt + c_width - (layout::PAGE_WIDTH_PT - layout::MARGIN_PT)).abs() < 0.5,
            "expected the last item flushed to the right margin, got {}",
            c.x_pt
        );
        // Two equal-sized fill gaps: B sits roughly a third of the way across
        // the leftover space, not at the midpoint (one fill) or the margin
        // (no fill).
        let leftover = c.x_pt - a.x_pt;
        assert!(
            (b.x_pt - a.x_pt - leftover / 2.0).abs() < 0.5,
            "expected B roughly midway between A and C, got a={} b={} c={}",
            a.x_pt,
            b.x_pt,
            c.x_pt
        );
    }

    #[test]
    fn hfil_behaves_like_hfill() {
        let (parsed, items) = items(r"A \hfil B");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let a = items.iter().find(|i| i.text == "A").unwrap();
        let b = items.iter().find(|i| i.text == "B").unwrap();
        let b_width = layout::text_width("B", b.font_size_pt, b.font);
        assert_eq!(a.x_pt, layout::MARGIN_PT);
        assert!((b.x_pt + b_width - (layout::PAGE_WIDTH_PT - layout::MARGIN_PT)).abs() < 0.5);
    }

    #[test]
    fn hspace_inserts_a_fixed_non_stretching_gap() {
        let (parsed, items) = items(r"A\hspace{36pt}B");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let a = items.iter().find(|i| i.text == "A").unwrap();
        let b = items.iter().find(|i| i.text == "B").unwrap();
        let a_width = layout::text_width("A", a.font_size_pt, a.font);
        // `hspace` (layout.rs) starts from the preceding item's true end
        // (`content_end`), not from the cursor's eagerly reserved trailing
        // inter-word space, so it adds exactly the requested 36pt on top of
        // "A"'s real width — no separate word gap is also added. See the
        // doc comment on `LayoutCursor::hspace`.
        assert!(
            (b.x_pt - (a.x_pt + a_width) - 36.0).abs() < 0.02,
            "a={} a_width={} b={}",
            a.x_pt,
            a_width,
            b.x_pt
        );
    }

    #[test]
    fn hspace_star_and_malformed_dimension_are_handled() {
        let (starred_parsed, starred_items) = items(r"A\hspace*{1em}B");
        assert!(
            starred_parsed.diagnostics.is_empty(),
            "{:?}",
            starred_parsed.diagnostics
        );
        assert!(
            starred_items.iter().any(|i| i.text == "A")
                && starred_items.iter().any(|i| i.text == "B")
        );

        let (malformed_parsed, malformed_items) = items(r"A\hspace{oops}B");
        assert!(malformed_parsed.diagnostics.iter().any(|d| d
            .message
            .contains(r"\hspace requires a recognised dimension")));
        assert!(!malformed_items.iter().any(|i| i.text == "oops"));
    }

    #[test]
    fn unsupported_command_dimension_or_keyword_argument_is_silently_skipped() {
        let (parsed, items) = items(r"Visible \foocmd{0.6em} \barcmd{empty} Tail.");
        assert!(!items.iter().any(|i| i.text == "0.6em"));
        assert!(!items.iter().any(|i| i.text == "empty"));
        assert!(items.iter().any(|i| i.text == "Visible"));
        assert!(items.iter().any(|i| i.text == "Tail."));
        let messages: Vec<&str> = parsed
            .diagnostics
            .iter()
            .map(|d| d.message.as_str())
            .collect();
        assert!(messages.iter().any(|m| m.contains(r"\foocmd")));
        assert!(messages.iter().any(|m| m.contains(r"\barcmd")));
        assert!(parsed.diagnostics.iter().any(|d| d
            .recovery
            .as_deref()
            .is_some_and(|r| r.contains("looked like a parameter"))));
    }

    #[test]
    fn unsupported_command_prose_argument_is_never_swallowed() {
        // Multiple words, and a single capitalized word, both fail the
        // dimension/keyword heuristic and must survive as visible text.
        let (parsed, items) = items(r"\foocmd{Hello world} \barcmd{Capitalized}");
        let _ = parsed;
        assert!(items.iter().any(|i| i.text == "Hello"));
        assert!(items.iter().any(|i| i.text == "world"));
        assert!(items.iter().any(|i| i.text == "Capitalized"));
    }

    #[test]
    fn unsupported_command_single_letter_argument_is_never_swallowed() {
        // A lone lowercase letter is excluded from the keyword heuristic:
        // it is far more likely to be real one-letter content (as in
        // `\def\x{y}`, from crates/compiler/tests/unsupported_inventory.rs)
        // than a parameter like `empty` or `arabic`.
        let (parsed, items) = items(r"\foocmd{y}");
        let _ = parsed;
        assert!(items.iter().any(|i| i.text == "y"));
    }

    #[test]
    fn known_arity_unimplemented_command_always_skips_its_argument() {
        // `1.5` has no unit suffix, so the dimension heuristic alone would
        // never match it: this exercises the explicit
        // `KNOWN_ARITY_UNIMPLEMENTED` list instead.
        let (parsed, items) = items(r"\linespread{1.5} Visible.");
        assert!(!items.iter().any(|i| i.text == "1.5"));
        assert!(items.iter().any(|i| i.text == "Visible."));
        assert!(parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains(r"\linespread")));
    }

    fn font_of(items: &[crate::layout::TextItem], text: &str) -> layout::Font {
        items
            .iter()
            .find(|item| item.text == text)
            .unwrap_or_else(|| panic!("no item {text:?}"))
            .font
    }

    #[test]
    fn text_style_commands_select_real_core14_variants() {
        use layout::Font;
        let source = r"a \textbf{b $x$ c} \textit{d \textbf{e}} \emph{f \emph{g}} \textsl{h} \texttt{i} \textsf{j} \textbf{\textrm{k}} l";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        for (text, font) in [
            ("a", Font::TimesRoman),
            ("b", Font::TimesBold),
            // Math variables are math italic even inside \textbf.
            ("x", Font::TimesItalic),
            ("c", Font::TimesBold),
            ("d", Font::TimesItalic),
            ("e", Font::TimesBoldItalic),
            ("f", Font::TimesItalic),
            ("g", Font::TimesRoman),
            ("h", Font::TimesItalic),
            ("i", Font::Courier),
            ("j", Font::Helvetica),
            ("k", Font::TimesBold),
            ("l", Font::TimesRoman),
        ] {
            assert_eq!(font_of(&items, text), font, "{text}");
        }
    }

    #[test]
    fn small_caps_commands_are_supported_and_scoped() {
        use layout::Font;
        let source = r"a \textsc{Bb \textbf{c}} {\scshape d \itshape e} f {\bf g \sc h} i";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        for (text, font) in [
            ("a", Font::TimesRoman),
            ("Bb", Font::TimesRoman),
            ("c", Font::TimesBold),
            ("d", Font::TimesRoman),
            ("e", Font::TimesItalic),
            ("f", Font::TimesRoman),
            ("g", Font::TimesBold),
            // `\sc` resets like the other LaTeX 2.09 forms.
            ("h", Font::TimesRoman),
            ("i", Font::TimesRoman),
        ] {
            assert_eq!(font_of(&items, text), font, "{text}");
        }
    }

    #[test]
    fn style_declarations_are_scoped_to_groups_and_environments() {
        use layout::Font;
        let source = "\\begin{document}{\\bf a} b {\\it c \\bfseries d} e \\begin{center}\\itshape f\n\ng\\end{center} h {\\ttfamily i \\normalfont j} \\bfseries k\\end{document}";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        for (text, font) in [
            ("a", Font::TimesBold),
            ("b", Font::TimesRoman),
            ("c", Font::TimesItalic),
            ("d", Font::TimesBoldItalic),
            ("e", Font::TimesRoman),
            ("f", Font::TimesItalic),
            ("g", Font::TimesItalic),
            ("h", Font::TimesRoman),
            ("i", Font::Courier),
            ("j", Font::TimesRoman),
            ("k", Font::TimesBold),
        ] {
            assert_eq!(font_of(&items, text), font, "{text}");
        }
    }

    fn size_of(items: &[crate::layout::TextItem], text: &str) -> f64 {
        items
            .iter()
            .find(|item| item.text == text)
            .unwrap_or_else(|| panic!("no item {text:?}"))
            .font_size_pt
    }

    #[test]
    fn size_declarations_scale_relative_to_normalsize() {
        // No `\documentclass`, so the body size defaults to the 12pt class's
        // own table.
        let source = r"\tiny a \scriptsize b \footnotesize c \small d \normalsize e \large f \Large g \LARGE h \huge i \Huge j";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        for (text, size) in [
            ("a", 6.0),
            ("b", 8.0),
            ("c", 10.0),
            ("d", 10.95),
            ("e", 12.0),
            ("f", 14.4),
            ("g", 17.28),
            ("h", 20.74),
            ("i", 24.88),
            ("j", 24.88),
        ] {
            assert_eq!(size_of(&items, text), size, "{text}");
        }
    }

    #[test]
    fn large_scales_text_until_its_group_closes() {
        let (parsed, items) = items(r"Normal {\Large Big text} After");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert_eq!(size_of(&items, "Normal"), crate::layout::BODY_SIZE_PT);
        assert_eq!(size_of(&items, "Big"), 17.28);
        assert_eq!(size_of(&items, "text"), 17.28);
        assert_eq!(size_of(&items, "After"), crate::layout::BODY_SIZE_PT);
    }

    #[test]
    fn bare_size_declaration_followed_by_a_group_is_not_scoped_to_it() {
        // `\Large{...}` is a common `\textbf{...}`-style misuse: unlike an
        // argument-taking command, `\Large` is a declaration, so it takes
        // effect in whatever scope it appears and stays active past the
        // following group — exactly like real LaTeX, where a group only
        // undoes assignments made *inside* it.
        let (parsed, items) = items(r"\Large{Big} still big");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert_eq!(size_of(&items, "Big"), 17.28);
        assert_eq!(size_of(&items, "still"), 17.28);
        assert_eq!(size_of(&items, "big"), 17.28);
    }

    #[test]
    fn size_declarations_use_the_active_documentclass_option_table() {
        // The three real LaTeX class tables are not a uniform scale of one
        // another: e.g. `\large` is the same absolute size as `\Large` in
        // the 10pt/11pt classes, but the 12pt class's own
        // `\normalsize`-plus-one-step.
        const LEVELS: [&str; 9] = [
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
        for (class_option, expected_pt) in [
            (
                "10pt",
                [5.0, 7.0, 8.0, 9.0, 12.0, 14.4, 17.28, 20.74, 24.88],
            ),
            (
                "11pt",
                [6.0, 8.0, 9.0, 10.0, 12.0, 14.4, 17.28, 20.74, 24.88],
            ),
            (
                "12pt",
                [6.0, 8.0, 10.0, 10.95, 14.4, 17.28, 20.74, 24.88, 24.88],
            ),
        ] {
            let body: String = LEVELS
                .iter()
                .enumerate()
                .map(|(i, level)| format!("\\{level} w{i} "))
                .collect();
            let source = format!(
                "\\documentclass[{class_option}]{{article}}\\begin{{document}}{body}\\end{{document}}"
            );
            let parsed = parse(&source);
            assert!(
                parsed.diagnostics.is_empty(),
                "{class_option}: {:?}",
                parsed.diagnostics
            );
            let output =
                crate::incremental::compile_full(&source, layout::LayoutConstraints::default());
            for (i, level) in LEVELS.iter().enumerate() {
                let word = format!("w{i}");
                let size = output
                    .pages
                    .iter()
                    .flat_map(|page| &page.items)
                    .find(|item| item.text == word)
                    .unwrap_or_else(|| panic!("{class_option} \\{level}: no item {word:?}"))
                    .font_size_pt;
                assert_eq!(size, expected_pt[i], "{class_option} \\{level}");
            }
        }
    }

    #[test]
    fn normalsize_is_exactly_the_documentclass_body_size_even_at_11pt() {
        // This compiler's `\documentclass[11pt]` body size is a literal
        // 11pt, not real LaTeX's 10.95pt `\normalsize` (see `class_size_pt`).
        // `\normalsize` must match that approximation exactly, not the real
        // class table value, so text with no size declaration in effect
        // renders identically to before this feature existed.
        let source = r"\documentclass[11pt]{article}\begin{document}Body {\small Small} \normalsize Reset\end{document}";
        let parsed = parse(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let output = crate::incremental::compile_full(source, layout::LayoutConstraints::default());
        let size_of = |text: &str| {
            output
                .pages
                .iter()
                .flat_map(|page| &page.items)
                .find(|item| item.text == text)
                .unwrap_or_else(|| panic!("no item {text:?}"))
                .font_size_pt
        };
        assert_eq!(size_of("Body"), 11.0);
        assert_eq!(size_of("Small"), 10.0);
        assert_eq!(size_of("Reset"), 11.0);
    }

    #[test]
    fn size_declarations_grow_the_line_height_so_larger_lines_do_not_overlap() {
        let (parsed, items) = items(r"{\Large Big}\\Small line");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let big = items.iter().find(|i| i.text == "Big").unwrap();
        let small = items.iter().find(|i| i.text == "Small").unwrap();
        assert!(small.baseline_y_pt > big.baseline_y_pt);
        let gap = small.baseline_y_pt - big.baseline_y_pt;
        // Without this feature `\Large` renders at the plain body size, so
        // the two lines would sit exactly `BODY_SIZE_PT * LINE_SPACING` apart
        // (14.4pt) regardless of the declared size. The real `\Large` line is
        // taller, so the gap to the next line must be strictly larger than
        // that, or the two lines would overlap.
        let old_buggy_gap = crate::layout::BODY_SIZE_PT * crate::layout::LINE_SPACING;
        assert!(
            gap > old_buggy_gap,
            "gap = {gap}, old_buggy_gap = {old_buggy_gap}"
        );
    }

    #[test]
    fn heading_styles_start_bold_and_honour_normalfont() {
        use layout::Font;
        let source = r"\newcommand{\problem}[2]{\subsection*{Problem #1 \normalfont[#2 \textit{pts}]}}\problem{1}{4}";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert_eq!(font_of(&items, "Problem"), Font::TimesBold);
        assert_eq!(font_of(&items, "["), Font::TimesRoman);
        assert_eq!(font_of(&items, "4"), Font::TimesRoman);
        assert_eq!(font_of(&items, "pts"), Font::TimesItalic);
    }

    #[test]
    fn unsupported_parameter_like_argument_is_skipped_but_prose_is_preserved() {
        let source = r"A \unknown{0.6em} B \typo{Readable prose} C";
        let (parsed, items) = items(source);
        let text: Vec<_> = items.iter().map(|item| item.text.as_str()).collect();
        assert!(!text.contains(&"0.6em"), "{text:?}");
        assert!(
            text.contains(&"Readable") && text.contains(&"prose"),
            "{text:?}"
        );
        assert_eq!(
            parsed
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.message.contains("not supported"))
                .count(),
            2
        );
        assert!(parsed.diagnostics.iter().any(|diagnostic| {
            diagnostic.message.contains("\\unknown")
                && diagnostic
                    .recovery
                    .as_deref()
                    .is_some_and(|note| note.contains("parameter"))
        }));
        assert!(parsed.diagnostics.iter().any(|diagnostic| {
            diagnostic.message.contains("\\typo")
                && diagnostic
                    .recovery
                    .as_deref()
                    .is_some_and(|note| note.contains("plain text"))
        }));
    }

    #[test]
    fn malformed_hspace_is_diagnosed_without_leaking_its_argument() {
        let source = r"A\hspace{wide}B";
        let (parsed, items) = items(source);
        assert!(parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("\\hspace requires")));
        assert_eq!(
            items
                .iter()
                .map(|item| item.text.as_str())
                .collect::<Vec<_>>(),
            ["A", "B"]
        );
    }

    #[test]
    fn url_is_typeset_literally_in_courier_with_one_link_diagnostic() {
        use layout::Font;
        // `%`, `#`, `_`, `~`, and `&` all have some other special meaning to
        // the ordinary tokenizer (comment, none, math subscript, none, none)
        // — `\url` must still take every one of them literally. The URL is
        // laid out as several break-opportunity runs (see `url_pieces`),
        // which is only visible in wrapping; concatenated, it must still
        // read back exactly as written, with "now." never swallowed by the
        // literal '%' as a stray comment.
        let url = "http://ex.com/a_b~c#d&e%f";
        let source = format!(r"See \url{{{url}}} now.");
        let (parsed, items) = items(&source);
        assert_eq!(items.first().unwrap().text, "See");
        assert_eq!(items.last().unwrap().text, "now.");
        let url_items = &items[1..items.len() - 1];
        assert_eq!(
            url_items
                .iter()
                .map(|i| i.text.as_str())
                .collect::<String>(),
            url,
            "the literal URL must survive intact across its break-opportunity runs"
        );
        assert!(url_items.iter().all(|i| i.font == Font::Courier));
        assert_eq!(
            parsed
                .diagnostics
                .iter()
                .filter(|d| d.message.contains("not clickable"))
                .count(),
            1
        );
    }

    #[test]
    fn url_diagnostic_is_emitted_once_per_document_not_once_per_use() {
        let source = r"\url{http://a.com} and \url{http://b.com} and \href{http://c.com}{link}";
        let (parsed, _) = items(source);
        assert_eq!(
            parsed
                .diagnostics
                .iter()
                .filter(|d| d.message.contains("not clickable"))
                .count(),
            1
        );
    }

    #[test]
    fn href_renders_only_its_text_argument_in_the_current_font() {
        use layout::Font;
        let source = r"Plain \textbf{\href{http://example.com/secret}{Click here}} end.";
        let (parsed, items) = items(source);
        let texts: Vec<&str> = items.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(texts, ["Plain", "Click", "here", "end."]);
        assert_eq!(font_of(&items, "Click"), Font::TimesBold);
        assert!(
            !items.iter().any(|i| i.text.contains("secret")),
            "the URL itself must never be typeset, only the link text"
        );
        assert!(parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("not clickable")));
    }

    #[test]
    fn nolinkurl_is_literal_courier_text_with_no_link_diagnostic() {
        use layout::Font;
        let source = r"\nolinkurl{http://example.com/x_y}";
        let (parsed, items) = items(source);
        assert_eq!(
            items.iter().map(|i| i.text.as_str()).collect::<String>(),
            "http://example.com/x_y"
        );
        assert!(items.iter().all(|i| i.font == Font::Courier));
        assert!(
            parsed.diagnostics.is_empty(),
            "\\nolinkurl was never a link, so it needs no 'not clickable' notice: {:?}",
            parsed.diagnostics
        );
    }

    /// `\lstset` is where every listings document puts its defaults, and it
    /// used to be a hard *error* — and a compounding one. The command's
    /// argument was then read as preamble material, so
    /// `fixtures/real-world/listings-manual`'s single `\lstset` produced six
    /// errors: `\lstset` itself, then `\ttfamily`, `\small`, `\bfseries`,
    /// `\itshape` and `\tiny` out of the style values inside it. Built from
    /// a `git archive` of `origin/main:crates` at 9d50d312 with this test
    /// dropped in, main reports
    ///
    /// ```text
    /// only the package notice may remain:
    /// ["\\lstset is not supported in the document preamble",
    ///  "\\ttfamily is not supported in the document preamble",
    ///  "\\small is not supported in the document preamble",
    ///  "\\bfseries is not supported in the document preamble",
    ///  "\\itshape is not supported in the document preamble",
    ///  "\\tiny is not supported in the document preamble"]
    /// ```
    #[test]
    fn lstset_is_accepted_in_the_preamble_and_typesets_nothing() {
        let source = concat!(
            r"\documentclass[11pt]{article}",
            "\n",
            r"\usepackage{listings}",
            "\n",
            "\\lstset{\n  basicstyle=\\ttfamily\\small,\n  keywordstyle=\\bfseries,\n",
            "  commentstyle=\\itshape,\n  numbers=left,\n  numberstyle=\\tiny,\n",
            "  frame=single,\n  breaklines=true,\n  showstringspaces=false,\n  tabsize=2\n}",
            "\n",
            r"\begin{document}",
            "\nBody text.\n",
            r"\end{document}",
            "\n",
        );
        let (parsed, items) = items(source);
        // `\usepackage{listings}` still says honestly that this compiler
        // does not implement the package; nothing else may be reported.
        let other: Vec<&str> = parsed
            .diagnostics
            .iter()
            .map(|d| d.message.as_str())
            .filter(|m| !m.starts_with("packages listings"))
            .collect();
        assert!(other.is_empty(), "only the package notice may remain: {other:?}");
        assert_eq!(
            items.iter().map(|i| i.text.as_str()).collect::<Vec<_>>(),
            ["Body", "text."],
            "\\lstset contributes no material"
        );
    }

    /// `\lstset` is global from its point of use, so listings allows it in
    /// the body too; a name that is not a listings key at all is reported
    /// once rather than silently swallowed.
    #[test]
    fn a_name_that_is_not_a_listings_key_is_reported_once() {
        let source = concat!(
            r"\documentclass{article}\usepackage{listings}",
            "\n",
            r"\begin{document}",
            "\n",
            r"\lstset{bacicstyle=\ttfamily}A",
            "\n\n",
            r"\lstset{bacicstyle=\ttfamily}B",
            "\n",
            r"\end{document}",
            "\n",
        );
        let (parsed, items) = items(source);
        let notes: Vec<&str> = parsed
            .diagnostics
            .iter()
            .filter(|d| d.message.contains("\\lstset keys"))
            .map(|d| d.message.as_str())
            .collect();
        assert_eq!(notes.len(), 1, "reported once, not per call: {notes:?}");
        assert!(notes[0].contains("bacicstyle"), "{notes:?}");
        assert_eq!(items.iter().map(|i| i.text.as_str()).collect::<Vec<_>>(), ["A", "B"]);
    }

    /// The key list nests: a braced value may hold commas and `=`, and a
    /// style value's backslashes never open a group.
    #[test]
    fn listings_key_names_split_at_top_level_commas_only() {
        assert_eq!(
            listings_key_names(r"basicstyle=\ttfamily\small,caption={A caption, part 2},breaklines"),
            vec!["basicstyle".to_string(), "caption".to_string(), "breaklines".to_string()]
        );
        assert_eq!(
            listings_key_names("basewidth={0.6em,0.45em}"),
            vec!["basewidth".to_string()]
        );
        assert!(listings_key_names("  ,  ,  ").is_empty());
        // Every key of the corpus fixture is a listings key.
        for key in listings_key_names(
            r"basicstyle=\ttfamily\small,backgroundcolor=\color{codebg},keywordstyle=\color{codekw}\bfseries,commentstyle=\color{codecomment}\itshape,numbers=left,numberstyle=\tiny,frame=single,breaklines=true,showstringspaces=false,tabsize=2",
        ) {
            assert!(listings_key_is_known(&key), "{key} is a listings key");
        }
    }

    /// `\hypersetup` is where real documents put hyperref's options, and it
    /// used to be a hard *error* ("not supported in the document preamble")
    /// on a perfectly valid document — `fixtures/real-world/hyperref-toc`
    /// line 7 is exactly this call. Every key it carries is a PDF
    /// annotation, outline or metadata setting: pdflatex (TeX Live 2025)
    /// sets the same 1062 words on the same 4 pages with and without it.
    #[test]
    fn hypersetup_is_accepted_in_the_preamble_and_typesets_nothing() {
        let source = concat!(
            r"\documentclass{article}",
            "\n",
            r"\usepackage{hyperref}",
            "\n",
            r"\hypersetup{colorlinks=true,linkcolor=blue,urlcolor=blue,citecolor=blue}",
            "\n",
            r"\begin{document}",
            "\nBody text.\n",
            r"\end{document}",
            "\n",
        );
        let (parsed, items) = items(source);
        assert!(
            parsed.diagnostics.is_empty(),
            "a valid hyperref preamble must produce no diagnostic at all: {:?}",
            parsed.diagnostics
        );
        assert_eq!(
            items.iter().map(|i| i.text.as_str()).collect::<Vec<_>>(),
            ["Body", "text."],
            "\\hypersetup contributes no material"
        );
    }

    /// `\hypersetup` is legal in the body too, and a key whose neutrality
    /// this compiler has not measured says so once rather than being
    /// silently swallowed.
    #[test]
    fn an_unmeasured_hypersetup_key_is_reported_once() {
        let source = concat!(
            r"\documentclass{article}\usepackage{hyperref}",
            "\n",
            r"\begin{document}",
            "\n",
            r"\hypersetup{pagebackref=true}A",
            "\n\n",
            r"\hypersetup{pagebackref=true}B",
            "\n",
            r"\end{document}",
            "\n",
        );
        let (parsed, items) = items(source);
        let notes: Vec<&Diagnostic> = parsed
            .diagnostics
            .iter()
            .filter(|d| d.message.contains("hypersetup keys"))
            .collect();
        assert_eq!(notes.len(), 1, "one notice per document: {:?}", parsed.diagnostics);
        assert!(notes[0].message.contains("pagebackref"), "{:?}", notes[0]);
        assert_eq!(items.iter().map(|i| i.text.as_str()).collect::<Vec<_>>(), ["A", "B"]);
    }

    /// `\usepackage{hyperref}` no longer warns "recognised but not
    /// implemented": that wording says the *text* may be wrong, and it is
    /// not — pdflatex sets `fixtures/real-world/hyperref-toc` identically
    /// with and without hyperref (1062 words, 4 pages, 0 moved). What is
    /// genuinely missing (the link annotations) keeps its own diagnostic.
    /// `backref`/`pagebackref` add bibliography text and still warn.
    #[test]
    fn hyperref_is_accepted_with_its_annotation_options_but_not_with_backref() {
        let doc = |options: &str| {
            format!(
                "\\documentclass{{article}}\\usepackage{options}{{hyperref}}\
                 \\begin{{document}}x\\end{{document}}"
            )
        };
        for options in ["", "[colorlinks]", "[hidelinks,breaklinks,unicode]", "[bookmarks=false]"] {
            let parsed = parse(&doc(options));
            assert!(
                !parsed.diagnostics.iter().any(|d| d.message.contains("hyperref are recognised")),
                "hyperref{options} must not warn: {:?}",
                parsed.diagnostics
            );
        }
        for options in ["[backref]", "[pagebackref]"] {
            let parsed = parse(&doc(options));
            assert!(
                parsed.diagnostics.iter().any(|d| d.message.contains("hyperref are recognised")),
                "hyperref{options} adds bibliography text and must keep warning: {:?}",
                parsed.diagnostics
            );
        }
    }

    #[test]
    fn verbatim_preserves_specials_and_splits_lines() {
        let source = "\\begin{verbatim}\n100% \\foo ${x}\nline two\n\\end{verbatim}";
        let parsed = parse(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let Block::Verbatim { lines, .. } = &parsed.blocks[0] else {
            panic!("expected a Block::Verbatim, got {:?}", parsed.blocks[0]);
        };
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "100% \\foo ${x}");
        assert_eq!(lines[1].text, "line two");
    }

    #[test]
    fn verbatim_star_marks_spaces_with_a_visible_dot() {
        let source = "\\begin{verbatim*}\na b\n\\end{verbatim*}";
        let parsed = parse(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let Block::Verbatim { lines, .. } = &parsed.blocks[0] else {
            panic!("expected a Block::Verbatim, got {:?}", parsed.blocks[0]);
        };
        assert_eq!(lines[0].text, "a\u{B7}b");
    }

    /// A tab in `verbatim` is one space, never a jump to a column stop: LaTeX
    /// `\let`s the active tab to `\@xobeysp`, and pdflatex measurably puts the
    /// following glyph exactly one cmtt10 space (5.24995 pt) further along, at
    /// every starting column. See `verbatim_display` for the measurements.
    #[test]
    fn verbatim_sets_a_tab_as_a_single_space_not_a_column_stop() {
        // Column 0, column 2, column 8 and two consecutive tabs. Under the old
        // 8-column rule these would have been 8, 8, 16 and 16 cells wide.
        let source = "\\begin{verbatim}\n\ta\nab\tcd\nabcdefgh\tX\nabc\t\tX\n\\end{verbatim}";
        let parsed = parse(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let Block::Verbatim { lines, .. } = &parsed.blocks[0] else {
            panic!("expected a Block::Verbatim, got {:?}", parsed.blocks[0]);
        };
        let texts: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, [" a", "ab cd", "abcdefgh X", "abc  X"]);
    }

    /// `\@setupverbvisibletab` points the active tab at the same visible-space
    /// box as an ordinary space, so a starred tab is one dot, not eight.
    #[test]
    fn verbatim_star_sets_a_tab_as_a_single_visible_space() {
        let source = "\\begin{verbatim*}\n\ta\nabc\t\tX\n\\end{verbatim*}";
        let parsed = parse(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let Block::Verbatim { lines, .. } = &parsed.blocks[0] else {
            panic!("expected a Block::Verbatim, got {:?}", parsed.blocks[0]);
        };
        let texts: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["\u{B7}a", "abc\u{B7}\u{B7}X"]);
    }

    #[test]
    fn lstlisting_options_are_parsed_and_diagnosed_then_typeset_literally() {
        let source = "\\begin{lstlisting}[language=Python]\nprint(1)\n\\end{lstlisting}";
        let parsed = parse(source);
        assert!(parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("lstlisting options")));
        let Block::Verbatim { lines, .. } = &parsed.blocks[0] else {
            panic!("expected a Block::Verbatim, got {:?}", parsed.blocks[0]);
        };
        assert_eq!(lines[0].text, "print(1)");
    }

    #[test]
    fn lstlisting_without_options_has_no_diagnostic() {
        let source = "\\begin{lstlisting}\nplain\n\\end{lstlisting}";
        let parsed = parse(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    }

    /// listings' `\lstinline` reads a raw delimited argument like `\verb`,
    /// after an optional `[<keys>]`. The keys set no text (listings.sty's
    /// `\lstinline` does `\lstset{flexiblecolumns,#1}` and typesets
    /// nothing), and `\lstinline{...}` closes on the brace.
    #[test]
    fn lstinline_reads_a_raw_delimited_argument_like_verb() {
        for (source, want) in [
            (r"A \lstinline|x y| B", "x y"),
            (r"A \lstinline!int z! B", "int z"),
            (r"A \lstinline[language=C]!int z! B", "int z"),
            (r"A \lstinline{p q} B", "p q"),
            // Blanks after the command are skipped (`\@ifnextchar`), unlike
            // `\verb`, whose very next character is the delimiter.
            (r"A \lstinline  |x| B", "x"),
            // The body is raw: %, \, $, { and } are not reinterpreted.
            (r"A \lstinline|a%b\c${}| B", r"a%b\c${}"),
        ] {
            let parsed = parse(source);
            assert!(parsed.diagnostics.is_empty(), "{source:?}: {:?}", parsed.diagnostics);
            let Block::Paragraph(inlines) = &parsed.blocks[0] else {
                panic!("{source:?}: expected a paragraph, got {:?}", parsed.blocks[0]);
            };
            let verbatim: Vec<&str> = inlines
                .iter()
                .filter_map(|i| match i {
                    Inline::Verbatim { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(verbatim, vec![want], "{source:?}");
        }
    }

    /// An unclosed `\lstinline` ends at the end of the line and says so
    /// under its own name, not `\verb`'s.
    #[test]
    fn unterminated_lstinline_ends_at_end_of_line() {
        let parsed = parse("A \\lstinline|x y\nB");
        assert!(
            parsed.diagnostics.iter().any(|d| d.message.contains("\\lstinline has no closing delimiter")),
            "{:?}",
            parsed.diagnostics
        );
    }

    /// A `[` that does not close on the same line is not an option list:
    /// it is the delimiter.
    #[test]
    fn lstinline_bracket_that_does_not_close_on_the_line_is_the_delimiter() {
        let parsed = parse("A \\lstinline[x[ B");
        let Block::Paragraph(inlines) = &parsed.blocks[0] else {
            panic!("expected a paragraph, got {:?}", parsed.blocks[0]);
        };
        assert!(
            inlines.iter().any(|i| matches!(i, Inline::Verbatim { text, .. } if text == "x")),
            "{inlines:?}"
        );
    }

    #[test]
    fn unterminated_verbatim_environment_recovers_at_end_of_input() {
        let source = "\\begin{verbatim}\nabc";
        let parsed = parse(source);
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("unterminated environment 'verbatim'")));
        let Block::Verbatim { lines, .. } = &parsed.blocks[0] else {
            panic!("expected a Block::Verbatim, got {:?}", parsed.blocks[0]);
        };
        assert_eq!(lines[0].text, "abc");
    }

    #[test]
    fn inline_verb_wraps_like_a_word_in_running_text() {
        let source = r"Use \verb|foo(x)| here.";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let verb_item = items
            .iter()
            .find(|item| item.text == "foo(x)")
            .expect("verb content placed as its own item");
        assert_eq!(verb_item.font, layout::Font::Courier);
        assert_eq!(
            items
                .iter()
                .map(|item| item.text.as_str())
                .collect::<Vec<_>>(),
            ["Use", "foo(x)", "here."]
        );
    }

    #[test]
    fn long_url_wraps_at_break_characters_without_a_hyphen() {
        // A URL far wider than the text measure, built almost entirely from
        // `URL_BREAK_AFTER` characters, must still wrap across at least two
        // lines (rather than overflow the page unbroken) and never gain a
        // hyphen at the break.
        let long_url = "http://example.com/".to_string() + &"segment/".repeat(40);
        let source = format!(r"\url{{{long_url}}}");
        let (parsed, pages) = pages(&source);
        assert!(parsed
            .diagnostics
            .iter()
            .all(|d| d.severity != crate::diagnostics::Severity::Error));
        let lines: std::collections::BTreeSet<_> = pages[0]
            .items
            .iter()
            .map(|item| item.baseline_y_pt.to_bits())
            .collect();
        assert!(
            lines.len() > 1,
            "a URL wider than the measure must wrap onto more than one line"
        );
        assert!(
            pages[0].items.iter().all(|item| !item.text.contains('-')),
            "url.sty never inserts a hyphen at a URL line break: {:?}",
            pages[0].items.iter().map(|i| &i.text).collect::<Vec<_>>()
        );
    }

    /// Runs of a URL, ignoring the kerns (see `url_runs_and_kerns` for those).
    fn url_runs(text: &str) -> Vec<&str> {
        url_pieces(text)
            .into_iter()
            .filter_map(|p| match p {
                UrlPiece::Run(run) => Some(run),
                UrlPiece::HyphenKern => None,
            })
            .collect()
    }

    #[test]
    fn url_segments_break_after_but_not_before_the_delimiter() {
        // Adjacent break characters (the `//` in `http://`) each end their
        // own run rather than being merged, which is harmless: `place`
        // glues consecutive zero-`space_before` runs back together whenever
        // they fit on the line.
        assert_eq!(
            url_runs("http://ex.com/a/b.c?d&e"),
            ["http:", "/", "/", "ex.", "com/", "a/", "b.", "c?", "d&", "e"]
        );
        assert_eq!(url_runs("plain"), ["plain"]);
        assert!(url_pieces("").is_empty());
    }

    /// url.sty's break set, measured with pdflatex (TeX Live 2025, 11pt
    /// `article`, T1): `\url{xxxxxxxx<c>xxxxxxxx}` in a 56pt `minipage`,
    /// where the only possible break is right after `<c>`, gives two lines
    /// for `/ . ? & # = + : _ , ; ! | > ) ] ' @` and one overfull line for
    /// `- ~ * $`. The hyphen is the one that matters in practice: url.sty
    /// refuses to break there so a URL's own hyphen cannot be read as
    /// hyphenation, and puts a 0.5pt kern there instead.
    #[test]
    fn url_runs_and_kerns() {
        for c in "/.?&#=+:_,;!|>)]'@".chars() {
            let text = format!("aa{c}bb");
            assert_eq!(
                url_runs(&text),
                [format!("aa{c}"), "bb".to_string()],
                "url.sty breaks after {c:?}"
            );
            assert!(
                !url_pieces(&text).contains(&UrlPiece::HyphenKern),
                "only a hyphen takes a kern, not {c:?}"
            );
        }
        for c in "~*$".chars() {
            let text = format!("aa{c}bb");
            assert_eq!(url_runs(&text), [text.as_str()], "url.sty does not break after {c:?}");
        }
        // A hyphen ends a run only so the kern has a place; it is not a break.
        assert_eq!(
            url_pieces("a-b-c"),
            [
                UrlPiece::Run("a-"),
                UrlPiece::HyphenKern,
                UrlPiece::Run("b-"),
                UrlPiece::HyphenKern,
                UrlPiece::Run("c"),
            ]
        );
        // The runs always reproduce the argument exactly.
        for text in ["a-b", "http://e.org/a-b/c.d", "-", "--", "a-"] {
            assert_eq!(url_runs(text).concat(), text, "runs must reconstruct {text:?}");
        }
    }

    /// `\url{a-b}` is 17.47511pt where `\texttt{a-b}` is 16.97511pt
    /// (pdflatex, TeX Live 2025, 11pt `article`, T1): url.sty puts a 0.5pt
    /// kern after every hyphen, and `\url{a-b-c}` is a full 1.0pt wider
    /// than its `\texttt` for the same reason.
    #[test]
    fn a_url_hyphen_carries_the_half_point_kern_url_sty_puts_there() {
        let (parsed, _items) = items(r"\url{a-b}");
        let kerns: Vec<&crate::text_builtins::TextDimen> = parsed
            .blocks
            .iter()
            .flat_map(|block| match block {
                Block::Paragraph(content) => content.iter().collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .filter_map(|inline| match inline {
                Inline::Kern { amount, .. } => Some(amount),
                _ => None,
            })
            .collect();
        assert_eq!(kerns.len(), 1, "one kern per hyphen: {:?}", parsed.blocks);
        assert_eq!(kerns[0].integer, 0);
        assert_eq!(kerns[0].frac, vec![5]);
        assert_eq!(
            kerns[0].unit,
            crate::text_builtins::DimenUnit::Physical(crate::text_builtins::PhysicalUnit::Pt)
        );
    }

    #[test]
    fn unterminated_verb_diagnoses_and_recovers_at_end_of_line() {
        let source = "\\verb|open\nmore text";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("\\verb has no closing delimiter")));
        assert!(items.iter().any(|item| item.text == "open"));
        assert!(items.iter().any(|item| item.text == "more"));
    }

    #[test]
    fn cite_resolves_a_forward_reference_before_the_bibliography_appears() {
        let source =
            r"See \cite{a}.\begin{thebibliography}{9}\bibitem{a}First.\end{thebibliography}";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let texts: Vec<&str> = items.iter().map(|i| i.text.as_str()).collect();
        assert!(texts.windows(3).any(|w| w == ["[", "1", "]"]));
    }

    #[test]
    fn cite_with_multiple_keys_joins_labels_with_a_comma() {
        let source =
            r"\cite{a,b}\begin{thebibliography}{9}\bibitem{a}A.\bibitem{b}B.\end{thebibliography}";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let texts: Vec<&str> = items.iter().map(|i| i.text.as_str()).collect();
        assert!(texts.windows(4).any(|w| w == ["[", "1", ", ", "2"]));
    }

    #[test]
    fn cite_note_is_appended_after_the_labels_and_a_tie_becomes_a_space() {
        let source = r"\cite[p.~2]{a}\begin{thebibliography}{9}\bibitem{a}A.\end{thebibliography}";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert!(items.iter().any(|i| i.text == ", p. 2"));
    }

    #[test]
    fn undefined_citation_renders_a_bold_question_mark_and_warns() {
        let (parsed, items) = items(r"\cite{missing}");
        assert_eq!(parsed.diagnostics.len(), 1, "{:?}", parsed.diagnostics);
        assert!(parsed.diagnostics[0]
            .message
            .contains("'missing' is undefined"));
        let mark = items.iter().find(|i| i.text == "?").expect("question mark");
        assert_eq!(mark.font, layout::Font::TimesBold);
    }

    #[test]
    fn bibitem_optional_label_overrides_the_number_and_does_not_consume_one() {
        let source = r"\begin{thebibliography}{9}\bibitem[Knuth 1984]{tex}A.\bibitem{b}B.\end{thebibliography}\cite{tex,b}";
        let (parsed, items) = items(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let texts: Vec<&str> = items.iter().map(|i| i.text.as_str()).collect();
        assert!(texts.contains(&"[Knuth 1984]"));
        assert!(texts
            .windows(4)
            .any(|w| w == ["[", "Knuth 1984", ", ", "1"]));
    }

    #[test]
    fn nocite_produces_no_visible_output() {
        let with_nocite =
            items(r"\nocite{a}\begin{thebibliography}{9}\bibitem{a}A.\end{thebibliography}").1;
        let without = items(r"\begin{thebibliography}{9}\bibitem{a}A.\end{thebibliography}").1;
        let texts =
            |items: &[layout::TextItem]| items.iter().map(|i| i.text.clone()).collect::<Vec<_>>();
        assert_eq!(texts(&with_nocite), texts(&without));
    }

    #[test]
    fn bibliography_command_reports_bibtex_is_out_of_scope() {
        let (parsed, _items) = items(r"\bibliography{refs}");
        assert_eq!(parsed.diagnostics.len(), 1, "{:?}", parsed.diagnostics);
        assert!(parsed.diagnostics[0].message.contains("BibTeX"));
    }

    #[test]
    fn bibitem_outside_thebibliography_is_an_error() {
        let (parsed, _items) = items(r"\bibitem{a}Stray.");
        assert_eq!(parsed.diagnostics.len(), 1, "{:?}", parsed.diagnostics);
        assert!(parsed.diagnostics[0]
            .message
            .contains("\\bibitem is only supported"));
    }
}
