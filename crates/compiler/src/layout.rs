//! Layout: turns parsed blocks into positioned items on pages.
//!
//! Line placement uses the shared font engine's Adobe Core 14 shaping:
//! Times-Roman for body text, Times-Bold for headings, and Symbol for supported
//! math glyphs. Kerning and available ligatures are applied. Hyphenation and
//! TeX's optimal paragraph breaking remain missing; breaking here is greedy.
//!
//! One item is emitted per word rather than per line. That keeps each item's
//! source span exact, which is what click-to-source navigation (FT-003) needs.

use crate::bib;
use crate::diagnostics::Diagnostic;
use crate::export::{self, ExportFont};
use crate::math::{self, MathBox};
use crate::parser::{FillLeader, 
    Block, FontSizeLevel, Inline, ListLeftMargin, MathRow, ParagraphStyle, TextFamily, TextStyle,
};
use crate::Span;
use flashtex_font_engine::core14::Core14;
use flashtex_font_engine::shape::{shape, ShapeOptions, Shaped};
use flashtex_font_engine::Core14Face;
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub use flashtex_font_engine::core14::Core14 as Font;

mod footnotes;

pub const PAGE_WIDTH_PT: f64 = 612.0;
pub const PAGE_HEIGHT_PT: f64 = 792.0;
pub const MARGIN_PT: f64 = 72.0;
pub const BODY_SIZE_PT: f64 = 12.0;
pub const LINE_SPACING: f64 = 1.2;
pub const PARAGRAPH_GAP_PT: f64 = 6.0;
/// Justification leaves a wrapped line ragged rather than stretch its
/// inter-word spaces by more than this multiple of their natural width
/// (so no gap exceeds 3x its natural size), standing in for TeX's
/// tolerance/badness limit on an underfull line.
const JUSTIFY_MAX_STRETCH: f64 = 2.0;
/// `\topsep`'s flat point value, as `\@verbatim`'s underlying `\trivlist`
/// would apply it around a `verbatim`/`lstlisting` block (article's 10pt
/// class default is close to this; this layout has no per-class variation or
/// rubber lengths, so — like `SMALL_SKIP_PT` et al. — one representative flat
/// amount stands in for the real `plus`/`minus` stretch).
pub const VERBATIM_TOPSEP_PT: f64 = 8.0;
/// `quote` margins: LaTeX's `\leftmargini` (2.5em at 10pt).
pub const QUOTE_INDENT_PT: f64 = 25.0;
/// `\leftmargini`..`\leftmarginiv` (standard classes' 10pt-class defaults):
/// each `itemize`/`enumerate` nesting level's own hanging-indent increment,
/// in em of the document body size. LaTeX's `\list` macro advances
/// `\@totalleftmargin` by the new level's `\leftmargin` on top of whatever
/// the enclosing list already established, so nested levels are cumulative
/// (see `list_margin_pt`). Nesting past level 4 — real LaTeX's `\@toodeep`
/// limit for these environments — reuses the deepest defined increment.
const LIST_LEFTMARGIN_EM: [f64; 4] = [2.5, 2.2, 1.87, 1.7];
/// `\labelsep`: gap between a list label's right edge and the item text,
/// constant across nesting levels.
const LIST_LABELSEP_EM: f64 = 0.5;
/// amsmath `\jot`: extra gap between rows of a multi-row display. Measured
/// against pdflatex (12pt article + amsmath): row gap = `\baselineskip` + 3pt.
pub const JOT_PT: f64 = 3.0;
/// Horizontal gap between right/left column pairs of an `align` row.
pub const ALIGN_PAIR_GAP_EM: f64 = 2.0;
/// References normally settle in two passes; the cap also covers page-number
/// changes caused by a resolved reference changing line or page breaks.
pub const REFERENCE_ITERATION_LIMIT: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReferenceValue {
    number: String,
    page: u32,
}

/// article.cls `\l@section`/`\l@subsection`/`\l@subsubsection` geometry:
/// (entry indent, `\numberline` box width) in em. Sections use `1.5em`
/// numbers at the margin; `\@dottedtocline{2}{1.5em}{2.3em}` and
/// `\@dottedtocline{3}{3.8em}{3.2em}` for the deeper levels.
const TOC_INDENT_NUMWIDTH_EM: [(f64, f64); 3] = [(0.0, 1.5), (1.5, 2.3), (3.8, 3.2)];
/// `\@pnumwidth`: the right-aligned page-number box.
const TOC_PNUMWIDTH_EM: f64 = 1.55;
/// `\@tocrmarg`: right margin that dotted-line titles wrap before.
const TOC_RMARG_EM: f64 = 2.55;
/// `\@dotsep`: each leader box is `\mkern4.5mu . \mkern4.5mu`.
const TOC_DOTSEP_MU: f64 = 4.5;
/// `\l@section`'s `\addvspace{1.0em \@plus\p@}` before each section entry.
const TOC_SECTION_SKIP_EM: f64 = 1.0;

/// One `\addcontentsline` record: a numbered heading and its page.
#[derive(Debug, Clone, PartialEq)]
struct TocEntry {
    level: u8,
    number: String,
    number_span: Span,
    content: Vec<Inline>,
    page: u32,
}

/// Layout inputs that participate in incremental cache validation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutConstraints {
    pub font_size_pt: f64,
    pub measure_pt: f64,
    /// `\setlength{\parskip}{..}`; `None` keeps `PARAGRAPH_GAP_PT`.
    pub parskip_pt: Option<f64>,
}

impl Default for LayoutConstraints {
    fn default() -> Self {
        Self {
            font_size_pt: BODY_SIZE_PT,
            measure_pt: PAGE_WIDTH_PT - 2.0 * MARGIN_PT,
            parskip_pt: None,
        }
    }
}

/// Retained only so math's script-size boxes can be measured consistently with
/// body text while math moves onto real metrics too.
pub fn text_width(text: &str, size: f64, font: Font) -> f64 {
    shape_text(font, text).map_or(0.0, |shaped| shaped.width_pt(size))
}

pub fn word_space(size: f64, font: Font) -> f64 {
    text_width(" ", size, font)
}

fn face(font: Font) -> &'static Core14Face {
    static TIMES_ROMAN: OnceLock<Core14Face> = OnceLock::new();
    static TIMES_BOLD: OnceLock<Core14Face> = OnceLock::new();
    static TIMES_ITALIC: OnceLock<Core14Face> = OnceLock::new();
    static TIMES_BOLD_ITALIC: OnceLock<Core14Face> = OnceLock::new();
    static HELVETICA: OnceLock<Core14Face> = OnceLock::new();
    static COURIER: OnceLock<Core14Face> = OnceLock::new();
    static SYMBOL: OnceLock<Core14Face> = OnceLock::new();
    match font {
        Core14::TimesRoman => TIMES_ROMAN.get_or_init(|| Core14Face::new(font)),
        Core14::TimesBold => TIMES_BOLD.get_or_init(|| Core14Face::new(font)),
        Core14::TimesItalic => TIMES_ITALIC.get_or_init(|| Core14Face::new(font)),
        Core14::TimesBoldItalic => TIMES_BOLD_ITALIC.get_or_init(|| Core14Face::new(font)),
        Core14::Helvetica => HELVETICA.get_or_init(|| Core14Face::new(font)),
        Core14::Courier => COURIER.get_or_init(|| Core14Face::new(font)),
        Core14::Symbol => SYMBOL.get_or_init(|| Core14Face::new(font)),
    }
}

fn shape_text(font: Font, text: &str) -> Result<Shaped, flashtex_font_engine::Error> {
    shape(face(font), text, &ShapeOptions::default())
}

/// Select the same Core 14 face that the export mapping assigns to a math glyph.
/// A single Latin letter is a math variable and uses the italic face, as TeX's
/// math italic does; digits, operators and multi-letter names stay upright.
pub(crate) fn math_font(text: &str) -> Font {
    let mut chars = text.chars();
    if let (Some(ch), None) = (chars.next(), chars.next()) {
        if ch.is_ascii_alphabetic() {
            return Font::TimesItalic;
        }
    }
    if !text.is_empty()
        && text.chars().all(|ch| {
            matches!(
                export::map_char(ch),
                export::Glyph::Encodable {
                    font: ExportFont::Symbol,
                    ..
                }
            )
        })
    {
        Font::Symbol
    } else {
        Font::TimesRoman
    }
}

/// The face's real x-height at `size`, in points, for accent vertical
/// placement (see `math::Nucleus::Accent`). `Symbol` declares no XHeight in
/// its AFM (Adobe's Symbol font has no case distinction to measure), so this
/// falls back to a conservative fraction of the em rather than claiming a
/// number the font never declared.
pub(crate) fn x_height_pt(font: Font, size: f64) -> f64 {
    use flashtex_font_engine::Face as _;
    let metrics = face(font).vertical_metrics();
    let ratio = if metrics.x_height_declared {
        f64::from(metrics.x_height) / 1000.0
    } else {
        0.45
    };
    ratio * size
}

/// Horizontal displacement, in points, that `font`'s italic slant introduces
/// at `height_pt` above the baseline (see `math::layout_accent`).
///
/// TeX's real accent-skew rule (TeXbook Appendix G, rule 12) shifts an accent
/// right by the base character's TFM skewchar kern, a per-glyph value that
/// Adobe Core 14 AFM metrics do not carry (there is no skewchar concept in an
/// AFM at all). What the AFM does carry is `ItalicAngle`, the whole-font
/// slant Times-Italic's outlines lean by (-15.5 degrees). Shearing a vertical
/// stroke by that angle is exactly what produces a skewchar-shaped effect: a
/// point `height_pt` above the baseline sits `height_pt * tan(|angle|)` to
/// the right of where the same point would fall in an upright face. Using
/// one whole-font angle rather than a per-glyph kern cannot reproduce TeX's
/// letter-by-letter skewchar table exactly, but it is the real metric this
/// font actually declares, rather than a borrowed constant from a different
/// font (e.g. cmmi10) applied to Times-Italic's differently-shaped glyphs.
/// Upright faces declare `ItalicAngle == 0`, so this is a no-op for them.
pub(crate) fn italic_skew_pt(font: Font, height_pt: f64) -> f64 {
    use flashtex_font_engine::Face as _;
    let angle_deg = face(font).italic_angle();
    if angle_deg == 0.0 {
        return 0.0;
    }
    height_pt * angle_deg.to_radians().tan().abs()
}

fn source_span(text: &str, span: Span, shaped: &Shaped) -> Span {
    let Some(first) = shaped.clusters.first() else {
        return span;
    };
    let last = shaped.clusters.last().expect("first cluster exists");
    for cluster in &shaped.clusters {
        debug_assert_eq!(
            text.get(cluster.source_range.clone()),
            Some(cluster.text.as_str())
        );
    }
    if span.end - span.start == text.len() {
        Span::in_document(
            span.document,
            span.start + first.source_range.start,
            span.start + last.source_range.end,
        )
    } else {
        // Macro replacement bytes do not exist in the document. Preserve the
        // invocation attribution instead of leaking shaping-relative offsets.
        span
    }
}

fn relative_span(text: &str, span: Span, start: usize, end: usize) -> Span {
    if span.end - span.start == text.len() && text.get(start..end).is_some() {
        Span::in_document(span.document, span.start + start, span.start + end)
    } else {
        span
    }
}

pub(crate) fn shaped_width(
    text: &str,
    size: f64,
    font: Font,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> (f64, Span) {
    // Symbol has no lunate epsilon (`\epsilon`, U+03F5) and names its angle
    // brackets by the deprecated U+2329/U+232A: they are drawn with the open
    // form and `angleleft`/`angleright`, as `export::map_char` encodes them.
    let substituted;
    let text = if font == Font::Symbol && text.contains(['\u{03F5}', '\u{27E8}', '\u{27E9}']) {
        substituted = text
            .replace('\u{03F5}', "\u{03B5}")
            .replace('\u{27E8}', "\u{2329}")
            .replace('\u{27E9}', "\u{232A}");
        substituted.as_str()
    } else {
        text
    };
    match shape_text(font, text) {
        Ok(shaped) => {
            for missing in &shaped.missing {
                let missing_span = relative_span(
                    text,
                    span,
                    missing.byte_offset,
                    missing.byte_offset + missing.ch.len_utf8(),
                );
                diagnostics.push(Diagnostic::warning(
                    format!(
                        "{} has no glyph for {:?} (U+{:04X})",
                        font.header().font_name,
                        missing.ch,
                        missing.ch as u32
                    ),
                    Some(missing_span),
                    Some("emitted the face's explicit .notdef glyph and continued".into()),
                ));
            }
            (shaped.width_pt(size), source_span(text, span, &shaped))
        }
        Err(error) => {
            let error_span = match error {
                flashtex_font_engine::Error::UnsupportedScript {
                    ch, byte_offset, ..
                } => relative_span(text, span, byte_offset, byte_offset + ch.len_utf8()),
                _ => span,
            };
            diagnostics.push(Diagnostic::error(
                format!(
                    "could not shape text with {}: {error}",
                    font.header().font_name
                ),
                Some(error_span),
                Some("kept the source item with zero advance and continued".into()),
            ));
            (0.0, span)
        }
    }
}

/// A pending `\hfill` on the current line (see `LayoutCursor::resolve_hfill`).
#[derive(Debug, Clone, Copy)]
struct LineFill {
    /// Index of the first item after the fill.
    boundary: usize,
    /// Where the content before the fill ended.
    x: f64,
    leader: FillLeader,
    size: f64,
    font: Font,
    span: Span,
}

/// `\hrule` thickness in horizontal leaders (TeX's default rule height).
const LEADER_RULE_PT: f64 = 0.4;

/// The items that draw `fill`'s leader across `width` points from `start`.
fn leader_items(fill: &LineFill, start: f64, width: f64, baseline: f64) -> Vec<TextItem> {
    match fill.leader {
        FillLeader::None => Vec::new(),
        FillLeader::Rule => vec![TextItem {
            text: math::FRACTION_RULE_CHAR.to_string(),
            x_pt: round2(start),
            baseline_y_pt: round2(baseline),
            font_size_pt: fill.size,
            span: fill.span,
            font: Font::TimesRoman,
            rule: Some(RuleGeometry {
                y_pt: round2(baseline - LEADER_RULE_PT),
                width_pt: round2(width),
                height_pt: LEADER_RULE_PT,
            }),
        }],
        FillLeader::Dots => {
            // `\cleaders\hb@xt@.44em{\hss.\hss}`: whole boxes only, the
            // leftover split evenly before the first and after the last.
            let box_width = 0.44 * fill.size;
            let count = (width / box_width).floor().max(0.0) as usize;
            let offset = (width - count as f64 * box_width) / 2.0;
            let dot = text_width(".", fill.size, fill.font);
            (0..count)
                .map(|i| TextItem {
                    text: ".".to_string(),
                    x_pt: round2(start + offset + i as f64 * box_width + (box_width - dot) / 2.0),
                    baseline_y_pt: round2(baseline),
                    font_size_pt: fill.size,
                    span: fill.span,
                    font: fill.font,
                    rule: None,
                })
                .collect()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextItem {
    pub text: String,
    pub x_pt: f64,
    pub baseline_y_pt: f64,
    pub font_size_pt: f64,
    pub span: Span,
    /// The face actually used to measure and lay out this item.
    pub font: Font,
    /// Typed geometry for an item that has a legacy text fallback.
    pub rule: Option<RuleGeometry>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuleGeometry {
    pub y_pt: f64,
    pub width_pt: f64,
    pub height_pt: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub number: u32,
    pub width_pt: f64,
    pub height_pt: f64,
    pub items: Vec<TextItem>,
}

/// `text_builtins::LogoMetrics` over a Core 14 face at `size` points.
struct Core14LogoMetrics {
    font: Font,
    size: f64,
}

impl Core14LogoMetrics {
    fn size_of(&self, font: crate::text_builtins::LogoFont) -> f64 {
        use crate::text_builtins::{self as tb, LogoFont};
        match font {
            LogoFont::ScriptSize => tb::sp_to_pt(tb::sf_size(tb::pt_to_sp(self.size))),
            LogoFont::Current | LogoFont::MathItalic => self.size,
        }
    }

    fn cap_pt(&self, size: f64) -> f64 {
        use flashtex_font_engine::Face as _;
        f64::from(face(self.font).vertical_metrics().cap_height) / 1000.0 * size
    }
}

impl crate::text_builtins::LogoMetrics for Core14LogoMetrics {
    fn char_box(
        &self,
        font: crate::text_builtins::LogoFont,
        ch: char,
    ) -> crate::text_builtins::CharBox {
        use crate::text_builtins::{self as tb, LogoFont};
        let size = self.size_of(font);
        let (face_font, height) = match font {
            LogoFont::MathItalic => (Font::Symbol, x_height_pt(self.font, size)),
            _ if ch.is_ascii_digit() || ch.is_ascii_uppercase() => (self.font, self.cap_pt(size)),
            _ => (self.font, x_height_pt(self.font, size)),
        };
        tb::CharBox {
            width: tb::pt_to_sp(text_width(&ch.to_string(), size, face_font)),
            height: tb::pt_to_sp(height),
            depth: 0,
            italic: 0,
        }
    }

    fn quad(&self) -> i32 {
        crate::text_builtins::pt_to_sp(self.size)
    }

    fn x_height(&self) -> i32 {
        crate::text_builtins::pt_to_sp(x_height_pt(self.font, self.size))
    }

    /// Core 14 has no math font parameters; cmsy10's ratios (sub1 .15em,
    /// sub_drop .05em at 7/10 size, x-height .430555em) stand in.
    fn math_sub_params(&self) -> crate::text_builtins::MathSubParams {
        use crate::text_builtins::{self as tb, MathSubParams};
        MathSubParams {
            sub1: tb::pt_to_sp(0.15 * self.size),
            math_x_height: tb::pt_to_sp(0.430555 * self.size),
            script_sub_drop: tb::pt_to_sp(0.05 * 0.7 * self.size),
        }
    }
}

fn glyph_width(text: &str, size: f64, font: Font) -> f64 {
    text_width(text, size, font)
}

/// Geometry needed to resume the one authoritative layout engine.
#[derive(Debug, Clone, Copy)]
pub struct FlowState {
    page_index: usize,
    x: f64,
    y: f64,
    line_ascent: f64,
    line_descent: f64,
    trailing_line_items: usize,
    /// See `LayoutCursor::content_end`. Without this, resuming from a cached
    /// fragment would leave `content_end` stale, so a reused block's first
    /// glued item (`space_before: false`) could rewind to the wrong `x`.
    content_end: f64,
    /// See `LayoutCursor::closed_line_skip`.
    closed_line_skip: Option<f64>,
}

impl FlowState {
    pub fn same_geometry(self, other: Self) -> bool {
        self.page_index == other.page_index
            && self.x.to_bits() == other.x.to_bits()
            && self.y.to_bits() == other.y.to_bits()
            && self.line_ascent.to_bits() == other.line_ascent.to_bits()
            && self.line_descent.to_bits() == other.line_descent.to_bits()
            && self.trailing_line_items == other.trailing_line_items
            && self.content_end.to_bits() == other.content_end.to_bits()
            && self.closed_line_skip.map(f64::to_bits) == other.closed_line_skip.map(f64::to_bits)
    }
}

/// One positioned item together with the zero-based page it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedItem {
    pub page_index: usize,
    pub item: TextItem,
}

/// Resumable layout cursor shared by clean and incremental compilation.
pub struct LayoutCursor {
    pages: Vec<Page>,
    x: f64,
    y: f64,
    line_ascent: f64,
    line_descent: f64,
    line_start: usize,
    /// The pen position right after the most recently placed glyph/box or
    /// `\hspace`, deliberately excluding any trailing inter-word space. Kept
    /// in sync only by `place`, `place_math`, `hspace`, and reset by
    /// `newline`; that is every way `x` can advance while an `\hfill` mark
    /// might be pending; see `resolve_hfill`.
    content_end: f64,
    /// Page-item-index boundaries recorded by `mark_hfill` for the current
    /// line, resolved (and cleared) by `resolve_hfill` when the line closes.
    line_fills: Vec<LineFill>,
    /// Inter-word spaces on the current line as (page-item index of the item
    /// after the space, natural width), consumed by `justify_line` when the
    /// line wraps and cleared whenever a line or block ends.
    line_spaces: Vec<(usize, f64)>,
    /// Whether the block being rendered is set justified (body paragraphs,
    /// list items, `quote`); off for headings, captions, `center`/`flush*`
    /// and displays.
    justify: bool,
    first_block: bool,
    constraints: LayoutConstraints,
    resolved_labels: BTreeMap<String, ReferenceValue>,
    collected_labels: BTreeMap<String, ReferenceValue>,
    /// Contents entries from the previous pass, typeset by
    /// `Block::TableOfContents`.
    resolved_toc: Vec<TocEntry>,
    /// Numbered headings of this pass; collected only when the document has
    /// a `\tableofcontents`, so other documents keep converging in one pass.
    collected_toc: Vec<TocEntry>,
    collect_toc: bool,
    emit_heading_numbers: bool,
    diagnostics: Vec<Diagnostic>,
    /// Active only while rendering a `Block::Styled` paragraph.
    style: Option<ParagraphStyle>,
    /// The current `Block::ListItem`'s left margin in points, or `0.0`
    /// outside one. Unlike `style`, this only ever affects `left_edge` —
    /// lists don't pull in the right margin the way `quote` does.
    list_margin_pt: f64,
    /// Page-bottom footnote state; see `layout::footnotes`.
    footnotes: footnotes::FootnoteState,
    /// Set when a display or heading has already ended its line and advanced
    /// past its own trailing skip (`\belowdisplayskip` or the sectioning
    /// after-skip), so `y` sits on a fresh, still-empty baseline. The next
    /// block starts on that baseline instead of ending another (empty) line,
    /// and its own `\addvspace`-style gap only adds what exceeds this skip.
    /// Cleared by `newline`.
    closed_line_skip: Option<f64>,
}

impl LayoutCursor {
    pub fn new(constraints: LayoutConstraints) -> Self {
        Self::with_labels(constraints, BTreeMap::new(), true)
    }

    fn with_labels(
        constraints: LayoutConstraints,
        resolved_labels: BTreeMap<String, ReferenceValue>,
        emit_heading_numbers: bool,
    ) -> Self {
        LayoutCursor {
            pages: vec![Page {
                number: 1,
                width_pt: PAGE_WIDTH_PT,
                height_pt: PAGE_HEIGHT_PT,
                items: Vec::new(),
            }],
            x: MARGIN_PT,
            y: MARGIN_PT + constraints.font_size_pt,
            line_ascent: constraints.font_size_pt,
            line_descent: constraints.font_size_pt * (LINE_SPACING - 1.0),
            line_start: 0,
            content_end: MARGIN_PT,
            line_fills: Vec::new(),
            line_spaces: Vec::new(),
            justify: false,
            first_block: true,
            constraints,
            resolved_labels,
            collected_labels: BTreeMap::new(),
            resolved_toc: Vec::new(),
            collected_toc: Vec::new(),
            collect_toc: false,
            emit_heading_numbers,
            diagnostics: Vec::new(),
            style: None,
            list_margin_pt: 0.0,
            footnotes: footnotes::FootnoteState::default(),
            closed_line_skip: None,
        }
    }

    fn right_edge(&self) -> f64 {
        MARGIN_PT + self.constraints.measure_pt - self.right_indent()
    }

    fn left_edge(&self) -> f64 {
        MARGIN_PT + self.left_indent()
    }

    /// `quote`/`quotation` indent both margins by the same amount; a list's
    /// `\rightmargin` defaults to `0`, so only `left_indent` looks at it.
    fn right_indent(&self) -> f64 {
        if self.style == Some(ParagraphStyle::Quote) {
            QUOTE_INDENT_PT
        } else {
            0.0
        }
    }

    fn left_indent(&self) -> f64 {
        if self.list_margin_pt > 0.0 {
            self.list_margin_pt
        } else if self.style == Some(ParagraphStyle::Quote) {
            QUOTE_INDENT_PT
        } else {
            0.0
        }
    }

    /// Shift the current line for `center`/`flushright` before it is closed.
    fn align_current_line(&mut self) {
        let factor = match self.style {
            Some(ParagraphStyle::Center) => 0.5,
            Some(ParagraphStyle::FlushRight) => 1.0,
            _ => return,
        };
        let line_end = self.x - word_space(self.constraints.font_size_pt, Font::TimesRoman);
        let shift = ((self.right_edge() - line_end) * factor).max(0.0);
        if let Some(page) = self.pages.last_mut() {
            for item in page.items.iter_mut().skip(self.line_start) {
                item.x_pt = round2(item.x_pt + shift);
            }
        }
    }

    /// Ends a line because the next item does not fit: unlike an explicit
    /// `\\` or a paragraph's last line, such a line is justified.
    fn wrap_line(&mut self, size: f64) {
        self.justify_line();
        self.newline(size);
    }

    /// Stretches the current line's inter-word spaces, in proportion to their
    /// natural widths, so its last item ends at the right edge. Lines with
    /// infinite glue (`\hfill`), no spaces, overfull content, or needing more
    /// than `JUSTIFY_MAX_STRETCH` stay as they are.
    fn justify_line(&mut self) {
        let spaces = std::mem::take(&mut self.line_spaces);
        let natural: f64 = spaces.iter().map(|&(_, width)| width).sum();
        let slack = self.right_edge() - self.content_end;
        if !self.justify
            || !self.line_fills.is_empty()
            || natural <= 0.0
            || slack <= 0.0
            || slack > JUSTIFY_MAX_STRETCH * natural
        {
            return;
        }
        let Some(page) = self.pages.last_mut() else {
            return;
        };
        let mut shift = 0.0;
        let mut spaces = spaces.into_iter().peekable();
        for (index, item) in page.items.iter_mut().enumerate().skip(self.line_start) {
            while let Some((_, width)) = spaces.next_if(|&(boundary, _)| boundary <= index) {
                shift += slack * width / natural;
            }
            item.x_pt = round2(item.x_pt + shift);
        }
    }

    /// Records the reserved gap before an item about to be pushed as a
    /// stretchable inter-word space, unless the item starts its line.
    fn note_space(&mut self) {
        let gap = self.x - self.content_end;
        let len = self.pages.last().expect("at least one page").items.len();
        if gap > 0.0 && len > self.line_start {
            self.line_spaces.push((len, gap));
        }
    }

    fn newline(&mut self, size: f64) {
        self.line_spaces.clear();
        self.closed_line_skip = None;
        self.resolve_hfill();
        self.align_current_line();
        self.footnotes
            .record_closed_line(self.pages.len() - 1, self.line_start, self.y);
        self.x = self.left_edge();
        self.content_end = self.x;
        self.y += self.line_descent + size;
        self.line_ascent = size;
        self.line_descent = size * (LINE_SPACING - 1.0);
        if self.y > self.body_bottom() {
            self.open_next_page(size);
            self.y = MARGIN_PT + size;
        }
        self.line_start = self.pages.last().expect("at least one page").items.len();
    }

    fn vertical_gap(&mut self, gap: f64) {
        self.x = self.left_edge();
        self.y += gap;
    }

    /// Records an `\hfill`/`\hfil` mark at the current position on the line
    /// being built. `resolve_hfill` turns this into an actual shift once the
    /// line's full width is known.
    fn mark_hfill(&mut self, leader: FillLeader, size: f64, font: Font, span: Span) {
        let boundary = self.pages.last().expect("at least one page").items.len();
        self.line_fills.push(LineFill { boundary, x: self.content_end, leader, size, font, span });
    }

    /// `\hspace{<dimen>}`/`\hspace*`: a fixed space with no visible glyph.
    /// Real TeX lets this glue break a line; this layout does not check for
    /// overflow here, matching how the rest of this line model only checks
    /// for a break before placing the next word (see `place`). Start at the
    /// preceding item's true end so the layout engine's eagerly reserved
    /// inter-word space is not accidentally added to the requested dimension.
    fn hspace(&mut self, pt: f64) {
        self.x = self.content_end + pt;
        self.content_end = self.x;
    }

    /// Distributes the current line's leftover width across every
    /// `\hfill`/`\hfil` mark collected since the line began, then clears
    /// them. Called from `newline`, right before the line closes, so both an
    /// explicit `\\` and an ordinary word-wrap resolve any pending fills —
    /// this is the one place a line is known to be complete. Multiple fills
    /// on one line share the leftover space equally, as real TeX glue does.
    fn resolve_hfill(&mut self) {
        if self.line_fills.is_empty() {
            return;
        }
        let fills = std::mem::take(&mut self.line_fills);
        let slack = (self.right_edge() - self.content_end).max(0.0);
        if slack <= 0.0 {
            return;
        }
        let per_fill = slack / fills.len() as f64;
        let Some(page) = self.pages.last_mut() else {
            return;
        };
        let mut shift = 0.0;
        let mut boundaries = fills.iter().map(|fill| fill.boundary).peekable();
        for (index, item) in page.items.iter_mut().enumerate().skip(self.line_start) {
            while boundaries.peek().is_some_and(|&boundary| boundary <= index) {
                boundaries.next();
                shift += per_fill;
            }
            item.x_pt = round2(item.x_pt + shift);
        }
        // Leaders fill each fill's share of the slack. The k-th fill starts
        // where its content ended plus the k fills before it; insert from the
        // last so earlier boundaries stay valid.
        let baseline = self.y;
        for (k, fill) in fills.iter().enumerate().rev() {
            let start = fill.x + k as f64 * per_fill;
            let drawn = leader_items(fill, start, per_fill, baseline);
            let at = fill.boundary.min(page.items.len());
            page.items.splice(at..at, drawn);
        }
    }

    /// `\newpage`: start a fresh page unconditionally, even if the current
    /// one still has room. Unlike `newline`'s overflow break, this always
    /// creates a new page rather than only doing so past the bottom margin.
    /// LaTeX's `\predisplaypenalty` is 10000: a page never breaks between a
    /// display and the line just before it. When the display's own line
    /// would start a new page, carry that preceding line over with it.
    fn keep_line_with_display(&mut self, size: f64) {
        let overflows = self.y + self.line_descent + size > PAGE_HEIGHT_PT - MARGIN_PT;
        let page = self.pages.last().expect("at least one page");
        if !overflows || self.line_start == 0 || page.items.len() == self.line_start {
            return;
        }
        self.resolve_hfill();
        let items = self
            .pages
            .last_mut()
            .expect("at least one page")
            .items
            .split_off(self.line_start);
        let shift = MARGIN_PT + self.line_ascent - self.y;
        self.force_page_break();
        self.y = MARGIN_PT + self.line_ascent;
        let page = self.pages.last_mut().expect("at least one page");
        page.items = items
            .into_iter()
            .map(|mut item| {
                item.baseline_y_pt = round2(item.baseline_y_pt + shift);
                if let Some(rule) = item.rule.as_mut() {
                    rule.y_pt = round2(rule.y_pt + shift);
                }
                item
            })
            .collect();
    }

    fn force_page_break(&mut self) {
        self.line_spaces.clear();
        self.x = MARGIN_PT;
        self.content_end = self.x;
        self.open_next_page(self.constraints.font_size_pt);
        self.y = MARGIN_PT + self.constraints.font_size_pt;
        self.line_start = 0;
    }

    /// `space_before` is false when the source glued this run directly
    /// against whatever came before it (no space token, no paragraph
    /// start) — a math boundary or a macro-argument splice such as
    /// `\normalfont[#2 points]`. Placing then rewinds to `content_end`,
    /// discarding the previous item's eagerly reserved trailing space,
    /// exactly as `hspace` already does for `\hspace{<dimen>}`.
    fn place(&mut self, text: String, size: f64, span: Span, font: Font, space_before: bool) {
        if !space_before {
            self.x = self.content_end;
        }
        let (w, span) = shaped_width(&text, size, font, span, &mut self.diagnostics);
        if self.x > self.left_edge() && self.x + w > self.right_edge() {
            self.wrap_line(size);
        }
        self.note_space();
        self.ensure_extents(size, size * (LINE_SPACING - 1.0));
        let item = TextItem {
            text,
            x_pt: round2(self.x),
            baseline_y_pt: round2(self.y),
            font_size_pt: size,
            span,
            font,
            rule: None,
        };
        self.pages
            .last_mut()
            .expect("at least one page")
            .items
            .push(item);
        self.content_end = self.x + w;
        self.x += w + word_space(size, font);
    }

    /// `\TeX`/`\LaTeX`/`\LaTeXe` (`text_builtins::layout_logo`) against this
    /// layout's Core 14 metrics: the construction's kerns, raise and lower
    /// are latex.ltx's, while glyph heights are the face's declared cap
    /// height (AFM carries no per-glyph TFM heights), so only the render
    /// pipeline's TFM-backed layout reproduces pdfLaTeX's positions.
    fn place_logo(
        &mut self,
        logo: crate::text_builtins::TextLogo,
        size: f64,
        span: Span,
        font: Font,
        space_before: bool,
    ) {
        use crate::text_builtins::{self as tb, LogoFont};
        if !space_before {
            self.x = self.content_end;
        }
        let metrics = Core14LogoMetrics { font, size };
        let built = tb::layout_logo(logo, &metrics);
        let width = tb::sp_to_pt(built.width);
        if self.x > self.left_edge() && self.x + width > self.right_edge() {
            self.wrap_line(size);
        }
        self.note_space();
        let ascent = built
            .glyphs
            .iter()
            .map(|g| tb::sp_to_pt(g.raise) + metrics.cap_pt(metrics.size_of(g.font)))
            .fold(size, f64::max);
        let descent = built
            .glyphs
            .iter()
            .map(|g| -tb::sp_to_pt(g.raise))
            .fold(size * (LINE_SPACING - 1.0), f64::max);
        self.ensure_extents(ascent, descent);
        let (base_x, base_y) = (self.x, self.y);
        for glyph in &built.glyphs {
            let glyph_font = if glyph.font == LogoFont::MathItalic {
                Font::Symbol
            } else {
                font
            };
            let glyph_size = metrics.size_of(glyph.font);
            let text = glyph.ch.to_string();
            let (_, glyph_span) =
                shaped_width(&text, glyph_size, glyph_font, span, &mut self.diagnostics);
            self.pages
                .last_mut()
                .expect("at least one page")
                .items
                .push(TextItem {
                    text,
                    x_pt: round2(base_x + tb::sp_to_pt(glyph.x)),
                    baseline_y_pt: round2(base_y - tb::sp_to_pt(glyph.raise)),
                    font_size_pt: glyph_size,
                    span: glyph_span,
                    font: glyph_font,
                    rule: None,
                });
        }
        self.content_end = self.x + width;
        self.x += width + word_space(size, font);
    }

    /// `\rule` in running text: a box `RuleBox::width` wide whose painted
    /// part spans `rule_bottom..rule_top` above the baseline
    /// (`text_builtins::TextRule::resolve`, latex.ltx 16359-16367).
    fn place_rule(
        &mut self,
        rule: &crate::text_builtins::TextRule,
        size: f64,
        span: Span,
        font: Font,
        space_before: bool,
    ) {
        use crate::text_builtins::{self as tb, DimenContext};
        if !space_before {
            self.x = self.content_end;
        }
        let cx = DimenContext {
            quad: tb::pt_to_sp(size),
            x_height: tb::pt_to_sp(x_height_pt(font, size)),
            text_width: tb::pt_to_sp(self.constraints.measure_pt),
            line_width: tb::pt_to_sp(self.right_edge() - self.left_edge()),
            column_width: tb::pt_to_sp(self.constraints.measure_pt),
        };
        let b = rule.resolve(&cx);
        let width = tb::sp_to_pt(b.width);
        if self.x > self.left_edge() && self.x + width > self.right_edge() {
            self.wrap_line(size);
        }
        self.note_space();
        self.ensure_extents(tb::sp_to_pt(b.height), tb::sp_to_pt(b.depth));
        if b.painted() {
            let top = self.y - tb::sp_to_pt(b.rule_top);
            let item = TextItem {
                text: math::FRACTION_RULE_CHAR.to_string(),
                x_pt: round2(self.x),
                baseline_y_pt: round2(self.y),
                font_size_pt: size,
                span,
                font: Font::TimesRoman,
                rule: Some(RuleGeometry {
                    y_pt: round2(top),
                    width_pt: round2(width),
                    height_pt: round2(tb::sp_to_pt(b.rule_top - b.rule_bottom)),
                }),
            };
            self.pages
                .last_mut()
                .expect("at least one page")
                .items
                .push(item);
        }
        self.content_end = self.x + width;
        self.x += width + word_space(size, font);
    }

    /// Explicit horizontal glue (`\quad`/`\qquad` in text mode): no glyph is
    /// placed, so there is nothing to draw, only `x` to advance. Mirrors TeX's
    /// discardable glue at a line break: if the glue would overflow the
    /// measure, the line breaks instead and the glue is dropped rather than
    /// carried onto the new line.
    fn text_glue(&mut self, em: f64, size: f64) {
        let width = em * size;
        if self.x > self.left_edge() && self.x + width > self.right_edge() {
            self.wrap_line(size);
            return;
        }
        self.x += width;
    }

    fn ensure_extents(&mut self, ascent: f64, descent: f64) {
        if ascent > self.line_ascent {
            let shift = ascent - self.line_ascent;
            self.y += shift;
            if let Some(page) = self.pages.last_mut() {
                for item in &mut page.items[self.line_start..] {
                    item.baseline_y_pt = round2(item.baseline_y_pt + shift);
                }
            }
            self.line_ascent = ascent;
        }
        self.line_descent = self.line_descent.max(descent);
    }

    /// See `place` for what `space_before` means and why rewinding to
    /// `content_end` is the correct way to honour it.
    fn place_math(&mut self, b: MathBox, size: f64, space_before: bool) {
        if !space_before {
            self.x = self.content_end;
        }
        if self.x > self.left_edge() && self.x + b.width > self.right_edge() {
            self.wrap_line(size);
        }
        self.note_space();
        self.ensure_extents(b.ascent, b.descent);
        let base_x = self.x;
        let base_y = self.y;
        let page = self.pages.last_mut().expect("at least one page");
        for item in b.items {
            let font = item.font.unwrap_or_else(|| math_font(&item.text));
            let rule = item.rule.map(|rule| RuleGeometry {
                y_pt: round2(base_y + rule.y),
                width_pt: round2(rule.width),
                height_pt: round2(rule.height),
            });
            page.items.push(TextItem {
                text: item.text,
                x_pt: round2(base_x + item.x),
                baseline_y_pt: round2(base_y + item.baseline),
                font_size_pt: item.size,
                span: item.span,
                font,
                rule,
            });
        }
        self.content_end = self.x + b.width;
        self.x += b.width + word_space(size, Font::TimesRoman);
    }

    fn display_math(&mut self, b: MathBox, size: f64, number: Option<(&str, Span)>) {
        // Displays centre themselves; line alignment must not move them again.
        let style = self.style.take();
        let justify = std::mem::replace(&mut self.justify, false);
        let left = self.left_edge();
        let display_x = left + (self.right_edge() - left - b.width).max(0.0) / 2.0;
        // TeX's short-skip test: the text before the display (an `\item`
        // label hangs outside it, so it counts as empty) plus 2em ends
        // before the display starts.
        let short = self.content_end + 2.0 * size < display_x;
        if self.x > left
            || self
                .pages
                .last()
                .is_some_and(|p| p.items.len() > self.line_start)
        {
            self.keep_line_with_display(size);
            self.newline(size);
        }
        let (above, below) = self.display_skips(short);
        self.vertical_gap(above);
        self.x = display_x;
        self.place_math(b, size, true);
        if let Some((number, span)) = number {
            self.place_equation_number(number, span, size);
        }
        self.newline(self.constraints.font_size_pt);
        self.vertical_gap(below);
        self.closed_line_skip = Some(below);
        self.style = style;
        self.justify = justify;
    }

    /// `\abovedisplayskip`/`\belowdisplayskip` (both the class size: 10, 11
    /// or 12pt) or, for a short pre-display line, `\abovedisplayshortskip`
    /// (0pt) and `\belowdisplayshortskip` (6pt, or 6.5pt in the 11pt and
    /// 12pt classes), from `size10.clo`..`size12.clo`.
    fn display_skips(&self, short: bool) -> (f64, f64) {
        let body = self.constraints.font_size_pt;
        if short {
            (0.0, if body <= 10.5 { 6.0 } else { 6.5 })
        } else {
            (body, body)
        }
    }

    /// Draws an `\item` label (bullet/number) right-aligned so it ends
    /// `\labelsep` before the item's hanging-indent margin, on the item's
    /// first baseline — mirroring `\makelabel`'s right-justified label box.
    /// Deliberately unclamped: a label wider than the available `labelwidth`
    /// is not wrapped or pushed into the item text, it just extends further
    /// left, exactly like real LaTeX's overfull label box.
    fn place_list_label(&mut self, text: &str, span: Span, margin_pt: f64, size: f64) {
        let width = glyph_width(text, size, Font::TimesRoman);
        let label_sep = LIST_LABELSEP_EM * size;
        let x_pt = round2(MARGIN_PT + margin_pt - label_sep - width);
        let item = TextItem {
            text: text.to_string(),
            x_pt,
            baseline_y_pt: round2(self.y),
            font_size_pt: size,
            span,
            font: Font::TimesRoman,
            rule: None,
        };
        self.pages
            .last_mut()
            .expect("at least one page")
            .items
            .push(item);
    }

    /// One article.cls contents line: the number in its `\numberline` box,
    /// the title wrapping at `\@pnumwidth` (sections) or `\@tocrmarg`,
    /// aligned `\leaders` dots for levels below section, and the page number
    /// flush right. Section entries are bold, with `1em` before each but the
    /// first. Dots and the page number carry the `\tableofcontents` span;
    /// number and title keep the heading's own spans.
    fn toc_entry(&mut self, entry: &TocEntry, toc_span: Span, first: bool) {
        let size = self.constraints.font_size_pt;
        let section = entry.level <= 1;
        let (indent_em, numwidth_em) =
            TOC_INDENT_NUMWIDTH_EM[usize::from(entry.level.clamp(1, 3)) - 1];
        let font = if section {
            Font::TimesBold
        } else {
            Font::TimesRoman
        };
        self.newline(size);
        if section && !first {
            self.vertical_gap(TOC_SECTION_SKIP_EM * size);
        }
        self.push_item(
            entry.number.clone(),
            MARGIN_PT + indent_em * size,
            size,
            entry.number_span,
            font,
        );

        // Headings are parsed bold; a contents line below section level is
        // upright medium, so only the heading's base weight is dropped.
        let content: Vec<Inline> = entry
            .content
            .iter()
            .filter(|inline| !matches!(inline, Inline::Label { .. }))
            .cloned()
            .map(|inline| match inline {
                Inline::Text {
                    text,
                    span,
                    style,
                    space_before,
                } if !section => Inline::Text {
                    text,
                    span,
                    style: TextStyle {
                        bold: false,
                        ..style
                    },
                    space_before,
                },
                other => other,
            })
            .collect();
        let saved = self.constraints;
        self.constraints.measure_pt -= if section {
            TOC_PNUMWIDTH_EM
        } else {
            TOC_RMARG_EM
        } * size;
        self.list_margin_pt = (indent_em + numwidth_em) * size;
        self.x = self.left_edge();
        self.content_end = self.x;
        emit(self, &content, size, font);
        self.resolve_hfill();
        self.constraints = saved;
        self.list_margin_pt = 0.0;

        let right = MARGIN_PT + self.constraints.measure_pt;
        let page_box = right - TOC_PNUMWIDTH_EM * size;
        if !section {
            // `\leaders` align their boxes to multiples of the box width from
            // the line's left edge, so dots line up across entries.
            let mu = size / 18.0;
            let box_width = 2.0 * TOC_DOTSEP_MU * mu + glyph_width(".", size, Font::TimesRoman);
            let mut slot = ((self.content_end - MARGIN_PT) / box_width).ceil();
            while MARGIN_PT + (slot + 1.0) * box_width <= page_box {
                let x = MARGIN_PT + slot * box_width + TOC_DOTSEP_MU * mu;
                self.push_item(".".to_string(), x, size, toc_span, Font::TimesRoman);
                slot += 1.0;
            }
        }
        let page = entry.page.to_string();
        let x = right - glyph_width(&page, size, font);
        self.push_item(page, x, size, toc_span, font);
        self.x = right;
        self.content_end = right;
    }

    /// Place `text` at an absolute `x` on the current baseline.
    fn push_item(&mut self, text: String, x: f64, size: f64, span: Span, font: Font) {
        if text.is_empty() {
            return;
        }
        let y = self.y;
        self.pages
            .last_mut()
            .expect("at least one page")
            .items
            .push(TextItem {
                text,
                x_pt: round2(x),
                baseline_y_pt: round2(y),
                font_size_pt: size,
                span,
                font,
                rule: None,
            });
    }

    fn place_equation_number(&mut self, number: &str, span: Span, size: f64) {
        let text = format!("({number})");
        let width = glyph_width(&text, size, Font::TimesRoman);
        let x_pt = round2(self.right_edge() - width);
        let page = self.pages.last_mut().expect("at least one page");
        page.items.push(TextItem {
            text,
            x_pt,
            baseline_y_pt: round2(self.y),
            font_size_pt: size,
            span,
            font: Font::TimesRoman,
            rule: None,
        });
    }

    /// Multi-row display (`gather`/`align`). `gather` rows are centred one by
    /// one; `align` cells alternate right/left alignment against column widths
    /// shared by every row, and the whole block is centred.
    fn display_rows(&mut self, rows: &[MathRow], aligned: bool, size: f64) {
        // Displays centre themselves; line alignment must not move them again.
        let style = self.style.take();
        let justify = std::mem::replace(&mut self.justify, false);
        if self.x > self.left_edge()
            || self
                .pages
                .last()
                .is_some_and(|p| p.items.len() > self.line_start)
        {
            self.keep_line_with_display(size);
            self.newline(size);
        }
        // amsmath's multi-row displays always use the full skips.
        let (above, below) = self.display_skips(false);
        self.vertical_gap(above);
        let boxes: Vec<Vec<MathBox>> = rows
            .iter()
            .map(|row| {
                row.cells
                    .iter()
                    .map(|cell| math::layout_display(cell, size, &mut self.diagnostics))
                    .collect()
            })
            .collect();
        let columns = boxes.iter().map(Vec::len).max().unwrap_or(0);
        let mut widths = vec![0.0f64; columns];
        for row in &boxes {
            for (column, b) in row.iter().enumerate() {
                widths[column] = widths[column].max(b.width);
            }
        }
        let gap = ALIGN_PAIR_GAP_EM * size;
        let total: f64 = widths.iter().sum::<f64>() + gap * (columns / 2) as f64;
        let measure = self.right_edge() - MARGIN_PT;
        for (index, (row, cells)) in rows.iter().zip(boxes).enumerate() {
            if index > 0 {
                self.newline(size);
                self.vertical_gap(JOT_PT);
            }
            if aligned {
                let mut column_x = MARGIN_PT + (measure - total).max(0.0) / 2.0;
                for (column, b) in cells.into_iter().enumerate() {
                    let width = widths[column];
                    // Even columns are right-aligned, odd columns left-aligned.
                    self.x = if column % 2 == 0 {
                        column_x + width - b.width
                    } else {
                        column_x
                    };
                    self.place_math(b, size, true);
                    column_x += width + if column % 2 == 1 { gap } else { 0.0 };
                }
            } else {
                let width: f64 = cells.iter().map(|b| b.width).sum();
                self.x = MARGIN_PT + (measure - width).max(0.0) / 2.0;
                for b in cells {
                    let next = self.x + b.width;
                    self.place_math(b, size, true);
                    self.x = next;
                }
            }
            if let Some(number) = &row.number {
                self.place_equation_number(number, row.span, size);
            }
        }
        self.newline(self.constraints.font_size_pt);
        self.vertical_gap(below);
        self.closed_line_skip = Some(below);
        self.style = style;
        self.justify = justify;
    }

    /// Apply the inter-block spacing and return the state used as a cache key.
    pub fn prepare_block(&mut self, block: &Block) -> FlowState {
        let body_size = self.constraints.font_size_pt;
        if matches!(block, Block::Paragraph(inlines) if inlines.iter().all(|inline| matches!(inline, Inline::Label { .. })))
        {
            return self.state();
        }
        // A display or heading already closed its line (see
        // `closed_line_skip`): start this block on that fresh baseline.
        let closed = self
            .closed_line_skip
            .take()
            .filter(|_| self.state().trailing_line_items == 0);
        let parskip = self.constraints.parskip_pt.unwrap_or(PARAGRAPH_GAP_PT);
        // Lists reset `\parskip` to `\parsep`, so a document's custom
        // `\parskip` never reaches its items. Without one, the fixed
        // `PARAGRAPH_GAP_PT` stand-in is kept for both.
        let item_parskip = match self.constraints.parskip_pt {
            Some(_) => list_parsep_pt(body_size),
            None => PARAGRAPH_GAP_PT,
        };
        match block {
            Block::Paragraph(_)
            | Block::FigureCaption { .. }
            | Block::Styled { .. }
            | Block::Rule { .. }
                if closed.is_some() =>
            {
                let gap = match block {
                    Block::Paragraph(_) => parskip,
                    // A `\\` that ended a centred paragraph is `\@centercr`,
                    // which cancels the next paragraph's `\parskip`.
                    Block::Styled { .. } if closed == Some(0.0) => 0.0,
                    _ => PARAGRAPH_GAP_PT,
                };
                self.vertical_gap(gap);
            }
            Block::ListItem {
                extra_gap_before_pt,
                ..
            } if closed.is_some() => {
                // `\item`'s `\addvspace{\itemsep}` merges with the skip
                // already there instead of adding to it.
                let skip = closed.unwrap_or(0.0);
                self.vertical_gap(item_parskip + (extra_gap_before_pt - skip).max(0.0));
            }
            Block::Heading { level, .. } if closed.is_some() => {
                let skip = closed.unwrap_or(0.0);
                let size = heading_size(*level, body_size);
                self.vertical_gap(
                    (size - body_size).max(0.0)
                        + (heading_before_skip(*level, body_size) - skip).max(0.0)
                        + parskip,
                );
            }
            Block::Paragraph(_) => {
                if !self.first_block {
                    self.newline(body_size);
                    self.vertical_gap(parskip);
                }
            }
            // The list's paragraph gap (see `item_parskip`), plus any
            // `\setlist` itemsep/topsep override before this item.
            Block::ListItem {
                extra_gap_before_pt,
                ..
            } => {
                if !self.first_block {
                    self.newline(body_size);
                    self.vertical_gap(item_parskip + extra_gap_before_pt);
                }
            }
            Block::Heading { level, .. } => {
                let size = heading_size(*level, body_size);
                if !self.first_block {
                    self.newline(size);
                    self.vertical_gap(heading_before_skip(*level, body_size) + parskip);
                }
            }
            // Opens with `\section*{\contentsname}`.
            Block::TableOfContents { .. } => {
                if !self.first_block {
                    self.newline(heading_size(1, body_size));
                    self.vertical_gap(PARAGRAPH_GAP_PT * 2.0);
                }
            }
            Block::FigureCaption { .. } | Block::Styled { .. } => {
                if !self.first_block {
                    self.newline(body_size);
                    self.vertical_gap(PARAGRAPH_GAP_PT);
                }
            }
            Block::VSpace { pt } => {
                // Only end a line that has content: after a rule or another
                // vertical block there is no text line to finish, and TeX adds
                // no interline glue there either.
                if !self.first_block && self.state().trailing_line_items > 0 {
                    self.newline(body_size);
                }
                self.vertical_gap(*pt);
            }
            Block::Rule { .. } => {
                if !self.first_block {
                    self.newline(body_size);
                    self.vertical_gap(PARAGRAPH_GAP_PT);
                }
            }
            Block::PageBreak => {
                if !self.first_block {
                    self.force_page_break();
                }
            }
            Block::Verbatim { .. } => {
                if !self.first_block {
                    self.newline(body_size);
                    self.vertical_gap(
                        self.constraints.parskip_pt.unwrap_or(PARAGRAPH_GAP_PT)
                            + VERBATIM_TOPSEP_PT,
                    );
                }
            }
            Block::TitleBlock { .. } => {
                // `\@maketitle` opens with `\newpage \null \vskip 2em`. Like
                // `Block::PageBreak`, the page break is unconditional once
                // something precedes it, but is skipped when `\maketitle` is
                // the very first thing in the document — matching real TeX,
                // whose page builder never ships an empty first page for a
                // `\newpage` that has nothing queued yet.
                if !self.first_block {
                    self.force_page_break();
                }
                self.vertical_gap(2.0 * body_size);
            }
            // `\vfill`: stretch to fill whatever room is left below the
            // current line on this page. Real TeX distributes stretch across
            // every `\vfill` sharing a page equally; this layout instead
            // gives the first one all the remaining room, which matches the
            // common single-`\vfill`-per-page idiom (pushing a signature or
            // footer to the bottom) exactly, and degrades to a hard bottom
            // clamp — rather than an overlap or a fabricated split — for the
            // rarer multi-`\vfill` case.
            Block::VFill => {
                if !self.first_block && self.state().trailing_line_items > 0 {
                    self.newline(body_size);
                }
                let remaining = (PAGE_HEIGHT_PT - MARGIN_PT - self.y).max(0.0);
                self.vertical_gap(remaining);
            }
        }
        self.first_block = false;
        self.state()
    }

    /// Lay out a block after `prepare_block`, returning its reusable fragment.
    pub fn render_prepared_block(&mut self, block: &Block) -> Vec<PlacedItem> {
        let starts: Vec<usize> = self.pages.iter().map(|page| page.items.len()).collect();
        let body_size = self.constraints.font_size_pt;
        match block {
            Block::Paragraph(inlines) => {
                self.justify = true;
                emit(self, inlines, body_size, Font::TimesRoman);
            }
            Block::Styled { style, content, .. } => {
                self.style = Some(*style);
                self.justify = *style == ParagraphStyle::Quote;
                // `left_edge()` depends on `self.style` (the `quote` indent),
                // which just changed, so `content_end` — synced to the old
                // margin by `prepare_block`'s `newline` — must move with it.
                // Otherwise this block's first item, if glued to what
                // precedes it in the source (`space_before: false`), would
                // rewind past the indent to the stale, unindented position.
                self.x = self.left_edge();
                self.content_end = self.x;
                emit(self, content, body_size, Font::TimesRoman);
                self.resolve_hfill();
                self.align_current_line();
                if *style != ParagraphStyle::Quote
                    && matches!(content.last(), Some(Inline::LineBreak { .. }))
                    && self.state().trailing_line_items == 0
                {
                    // In `center`/`flushleft`/`flushright` a trailing `\\`
                    // ends the paragraph rather than adding an empty line.
                    self.closed_line_skip = Some(0.0);
                }
                self.style = None;
            }
            Block::ListItem {
                level,
                label,
                content,
                extra_gap_after_pt,
                leftmargin,
                widest_label,
                ..
            } => {
                self.list_margin_pt = match widest_label {
                    // `thebibliography`'s `\labelwidth` + `\labelsep`: the
                    // width of its widest label's own bracket text, not the
                    // fixed `itemize`/`enumerate` leftmargin table.
                    Some(text) => {
                        glyph_width(&bib::label_bracket(text), body_size, Font::TimesRoman)
                            + LIST_LABELSEP_EM * body_size
                    }
                    None => {
                        let override_pt = list_leftmargin_override_pt(leftmargin, body_size);
                        list_margin_pt(*level, body_size, override_pt)
                    }
                };
                self.justify = true;
                if let Some((text, span)) = label.as_ref().filter(|(text, _)| !text.is_empty()) {
                    self.place_list_label(text, *span, self.list_margin_pt, body_size);
                }
                // Same reasoning as `Block::Styled`: `left_edge()` now
                // reflects the new hanging indent, so `content_end` must
                // move with `x` or this item's first glued inline
                // (`space_before: false`) would rewind past it.
                self.x = self.left_edge();
                self.content_end = self.x;
                emit(self, content, body_size, Font::TimesRoman);
                self.list_margin_pt = 0.0;
                if *extra_gap_after_pt != 0.0 {
                    // The list's closing `\addvspace{\topsep}` merges with a
                    // display's below-skip instead of adding to it.
                    match self.closed_line_skip {
                        Some(skip) => {
                            self.vertical_gap((extra_gap_after_pt - skip).max(0.0));
                            self.closed_line_skip = Some(skip.max(*extra_gap_after_pt));
                        }
                        None => self.vertical_gap(*extra_gap_after_pt),
                    }
                }
            }
            Block::Heading {
                level,
                number,
                number_span,
                content,
            } => {
                if self.collect_toc && !number.is_empty() {
                    self.collected_toc.push(TocEntry {
                        level: *level,
                        number: number.clone(),
                        number_span: *number_span,
                        content: content.clone(),
                        page: self.pages.len() as u32,
                    });
                }
                if self.emit_heading_numbers && !number.is_empty() {
                    let size = heading_size(*level, body_size);
                    self.place(number.clone(), size, *number_span, Font::TimesBold, true);
                    // `\@seccntformat`: `\csname the#1\endcsname\quad`.
                    self.x = self.content_end + size;
                }
                emit(
                    self,
                    content,
                    heading_size(*level, body_size),
                    Font::TimesBold,
                );
                self.newline(body_size);
                let after = heading_after_skip(*level, body_size);
                self.vertical_gap(after);
                self.closed_line_skip = Some(after);
                self.x = MARGIN_PT;
            }
            Block::FigureCaption { content } => {
                let width: f64 = content
                    .iter()
                    .map(|inline| match inline {
                        Inline::Text { text, .. } => {
                            glyph_width(text, body_size, Font::TimesRoman)
                                + word_space(body_size, Font::TimesRoman)
                        }
                        _ => 0.0,
                    })
                    .sum();
                self.x = MARGIN_PT + (self.constraints.measure_pt - width).max(0.0) / 2.0;
                emit(self, content, body_size, Font::TimesRoman);
                self.newline(body_size);
            }
            Block::VSpace { .. } | Block::PageBreak | Block::VFill => {}
            Block::TableOfContents { span } => {
                self.render_prepared_block(&Block::Heading {
                    level: 1,
                    number: String::new(),
                    number_span: *span,
                    content: vec![Inline::Text {
                        text: "Contents".to_string(),
                        span: *span,
                        style: TextStyle::BOLD,
                        space_before: true,
                    }],
                });
                let entries = std::mem::take(&mut self.resolved_toc);
                for (index, entry) in entries.iter().enumerate() {
                    self.toc_entry(entry, *span, index == 0);
                }
                self.resolved_toc = entries;
            }
            Block::TitleBlock {
                title,
                authors,
                date,
            } => {
                // `\@maketitle`, transcribed from `article.cls` (see
                // `crates/title-layout/src/title.rs`, this compiler's own
                // measured-layout oracle for these same numbers): `\LARGE`
                // title, `\vskip 1.5em`, `\large` author block, `\vskip
                // 1em`, `\large` date, trailing `\vskip 1.5em`. All four
                // `\vskip` amounts are plain, unconditionally additive glue
                // in the body (`\normalsize`) font, per that crate's own
                // documented boundary — never `\parskip`, which is why this
                // uses `vertical_gap`/`newline` directly rather than
                // `Block::Styled`'s ordinary per-paragraph gap.
                let title_size = size_declaration_pt(FontSizeLevel::Large3, body_size);
                let author_size = size_declaration_pt(FontSizeLevel::Large1, body_size);
                self.style = Some(ParagraphStyle::Center);

                self.x = self.left_edge();
                self.content_end = self.x;
                // `prepare_block` positioned `y` with `vertical_gap`, not
                // `newline`, so the line metrics still reflect whatever
                // preceded this block (or the cursor's own initial body-size
                // line, when `\maketitle` is the very first thing in the
                // document). `place`'s own `ensure_extents` then grows `y` by
                // the difference to the title's actual (larger) size, the
                // same self-correction an ordinary first-block `\section`
                // already relies on to sit below the margin correctly.
                emit(self, title, title_size, Font::TimesRoman);
                self.newline(author_size);
                self.vertical_gap(1.5 * body_size);

                self.x = self.left_edge();
                self.content_end = self.x;
                emit(self, authors, author_size, Font::TimesRoman);
                self.newline(if date.is_some() {
                    author_size
                } else {
                    body_size
                });
                self.vertical_gap(body_size);

                if let Some(date) = date {
                    self.x = self.left_edge();
                    self.content_end = self.x;
                    emit(self, date, author_size, Font::TimesRoman);
                    self.newline(body_size);
                }
                self.vertical_gap(1.5 * body_size);

                self.style = None;
            }
            Block::Rule { span } => {
                let width = self.constraints.measure_pt;
                let item = TextItem {
                    text: math::FRACTION_RULE_CHAR.to_string(),
                    x_pt: round2(self.x),
                    baseline_y_pt: round2(self.y),
                    font_size_pt: body_size,
                    span: *span,
                    font: Font::TimesRoman,
                    rule: Some(RuleGeometry {
                        y_pt: round2(self.y),
                        width_pt: round2(width),
                        height_pt: 0.5,
                    }),
                };
                self.pages
                    .last_mut()
                    .expect("at least one page")
                    .items
                    .push(item);
                // A rule has no depth: end its line without adding a text line.
                self.newline(0.0);
            }
            Block::Verbatim { lines, .. } => {
                self.x = self.left_edge();
                self.content_end = self.x;
                for (index, line) in lines.iter().enumerate() {
                    // One `TextItem` per source line, never split across a
                    // `place` call, so the measure-overflow check in `place`
                    // (only triggered once something already sits on the
                    // line) never wraps it — long lines simply overflow the
                    // margin, exactly like real LaTeX's own verbatim.
                    self.place(line.text.clone(), body_size, line.span, Font::Courier, true);
                    if index + 1 < lines.len() {
                        self.newline(body_size);
                    }
                }
            }
        }
        // A block is the incremental cache unit. Resolve its final line before
        // collecting the placed fragment so a reused block never depends on
        // ephemeral `line_fills` state that is intentionally absent from
        // `FlowState`. Explicit line breaks already resolve earlier lines.
        self.resolve_hfill();
        self.line_spaces.clear();
        self.justify = false;
        let mut placed = Vec::new();
        for (page_index, page) in self.pages.iter().enumerate() {
            let start = starts.get(page_index).copied().unwrap_or(0);
            placed.extend(
                page.items[start..]
                    .iter()
                    .cloned()
                    .map(|item| PlacedItem { page_index, item }),
            );
        }
        placed
    }

    pub(crate) fn measure_pt(&self) -> f64 {
        self.constraints.measure_pt
    }

    pub(crate) fn body_size_pt(&self) -> f64 {
        self.constraints.font_size_pt
    }

    /// Lays `inlines` out in a detached cursor as one box whose origin is its
    /// first baseline (a `tabular` entry or `@{}` text): a single unbroken
    /// line when `measure` is `None`, else a paragraph of that width. Returns
    /// the box and the offset of its last baseline. Diagnostics and labels
    /// flow back into this cursor.
    pub(crate) fn inline_box(
        &mut self,
        inlines: &[Inline],
        size: f64,
        measure: Option<f64>,
    ) -> (MathBox, f64) {
        let constraints = LayoutConstraints {
            measure_pt: measure.unwrap_or(crate::tabular::MAX_DIMEN_PT),
            ..self.constraints
        };
        let mut inner = LayoutCursor::with_labels(
            constraints,
            std::mem::take(&mut self.resolved_labels),
            self.emit_heading_numbers,
        );
        let left = inner.left_edge();
        let mut first_y = inner.y;
        let mut natural: f64 = 0.0;
        for inline in inlines {
            emit(
                &mut inner,
                std::slice::from_ref(inline),
                size,
                Font::TimesRoman,
            );
            if inner.pages.len() == 1 && inner.line_start == 0 {
                first_y = inner.y;
            }
            natural = natural.max(inner.content_end - left);
        }
        if measure.is_some() {
            inner.resolve_hfill();
        } else {
            // An unbreakable entry has no measure to fill.
            inner.line_fills.clear();
        }
        self.resolved_labels = std::mem::take(&mut inner.resolved_labels);
        self.diagnostics.append(&mut inner.diagnostics);
        let page = self.pages.len() as u32;
        for (key, mut value) in std::mem::take(&mut inner.collected_labels) {
            value.page = page;
            self.collected_labels.insert(key, value);
        }
        let mut items = Vec::new();
        let (mut ascent, mut descent) = (0.0f64, 0.0f64);
        let mut offset = 0.0;
        let page_count = inner.pages.len();
        for (index, page) in inner.pages.into_iter().enumerate() {
            let mut last_baseline = first_y;
            for item in page.items {
                let baseline = item.baseline_y_pt + offset - first_y;
                last_baseline = last_baseline.max(item.baseline_y_pt);
                let (top, bottom) = match item.rule {
                    Some(rule) => (
                        rule.y_pt + offset - first_y,
                        rule.y_pt + rule.height_pt + offset - first_y,
                    ),
                    None => {
                        let (up, down) = font_extents(item.font, item.font_size_pt);
                        (baseline - up, baseline + down)
                    }
                };
                ascent = ascent.max(-top);
                descent = descent.max(bottom);
                items.push(math::MathItem {
                    font: Some(item.font),
                    text: item.text,
                    x: item.x_pt - left,
                    baseline,
                    size: item.font_size_pt,
                    span: item.span,
                    rule: item.rule.map(|rule| math::MathRule {
                        y: rule.y_pt + offset - first_y,
                        width: rule.width_pt,
                        height: rule.height_pt,
                    }),
                });
            }
            // A detached paragraph never really breaks pages; continue the
            // next page's lines below this page's last line.
            if index + 1 < page_count {
                offset += last_baseline + LINE_SPACING * size - (MARGIN_PT + size);
            }
        }
        let width = measure.unwrap_or(natural);
        let last = inner.y + offset - first_y;
        (
            MathBox {
                items,
                width,
                ascent,
                descent,
            },
            last,
        )
    }

    pub fn state(&self) -> FlowState {
        let page_index = self.pages.len() - 1;
        FlowState {
            page_index,
            x: self.x,
            y: self.y,
            line_ascent: self.line_ascent,
            line_descent: self.line_descent,
            trailing_line_items: self.pages[page_index].items.len() - self.line_start,
            content_end: self.content_end,
            closed_line_skip: self.closed_line_skip,
        }
    }

    /// Restore a cached block whose prepared state matched the current state.
    pub fn append_reused(
        &mut self,
        placed: &[PlacedItem],
        diagnostics: &[Diagnostic],
        end: FlowState,
    ) {
        while self.pages.len() <= end.page_index {
            let number = self.pages.len() as u32 + 1;
            self.pages.push(Page {
                number,
                width_pt: PAGE_WIDTH_PT,
                height_pt: PAGE_HEIGHT_PT,
                items: Vec::new(),
            });
        }
        for placed_item in placed {
            self.pages[placed_item.page_index]
                .items
                .push(placed_item.item.clone());
        }
        self.x = end.x;
        self.content_end = end.content_end;
        self.closed_line_skip = end.closed_line_skip;
        self.y = end.y;
        self.line_ascent = end.line_ascent;
        self.line_descent = end.line_descent;
        self.line_start = self.pages[end.page_index]
            .items
            .len()
            .saturating_sub(end.trailing_line_items);
        self.diagnostics.extend_from_slice(diagnostics);
    }

    pub fn diagnostics_len(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn diagnostics_since(&self, start: usize) -> &[Diagnostic] {
        &self.diagnostics[start..]
    }

    pub fn into_pages(mut self) -> Vec<Page> {
        // A trailing `\hfill` on the document's very last line has no
        // following block to trigger `newline`'s resolution, so give it one
        // last chance here. Idempotent when nothing is pending.
        self.resolve_hfill();
        self.finish_footnotes();
        self.pages
    }

    pub fn into_pages_and_diagnostics(mut self) -> (Vec<Page>, Vec<Diagnostic>) {
        self.resolve_hfill();
        self.finish_footnotes();
        (self.pages, self.diagnostics)
    }

    fn into_result(mut self) -> (Vec<Page>, CrossReferences, Vec<Diagnostic>) {
        self.resolve_hfill();
        self.finish_footnotes();
        (
            self.pages,
            (self.collected_labels, self.collected_toc),
            self.diagnostics,
        )
    }
}

/// `\parsep` for a first-level list: 4pt, 4.5pt or 5pt in the 10pt, 11pt
/// and 12pt classes (`size1x.clo`'s `\@listi`).
fn list_parsep_pt(body_size: f64) -> f64 {
    if body_size <= 10.5 {
        4.0
    } else if body_size <= 11.5 {
        4.5
    } else {
        5.0
    }
}

/// One `ex` of the body font (cmr10's x-height is 0.4306em), the unit
/// `article.cls`'s `\@startsection` skips are written in.
fn body_ex(body_size: f64) -> f64 {
    0.4306 * body_size
}

/// `\@startsection` before-skip: 3.5ex for `\section`, 3.25ex below it.
fn heading_before_skip(level: u8, body_size: f64) -> f64 {
    body_ex(body_size) * if level == 1 { 3.5 } else { 3.25 }
}

/// `\@startsection` after-skip: 2.3ex for `\section`, 1.5ex below it.
fn heading_after_skip(level: u8, body_size: f64) -> f64 {
    body_ex(body_size) * if level == 1 { 2.3 } else { 1.5 }
}

fn heading_size(level: u8, body_size: f64) -> f64 {
    body_size
        * match level {
            1 => 17.0 / BODY_SIZE_PT,
            2 => 14.0 / BODY_SIZE_PT,
            // article.cls `\subsubsection`: `\normalsize\bfseries`.
            _ => 1.0,
        }
}

/// Absolute point size for one `\tiny`..`\Huge` declaration, from the real
/// LaTeX class files' own tables (`size10.clo`/`size11.clo`/`size12.clo`),
/// selected by the document's body size (`\documentclass[10pt|11pt|12pt]`;
/// see `parser::Parsed::class_size_pt`). The three tables are not a uniform
/// scale of each other — e.g. `\large` is 12pt in the 10pt and 11pt classes
/// but 14.4pt in the 12pt class — so the whole table is picked by class
/// rather than computed from one ratio.
///
/// `\normalsize` is deliberately not looked up here: it is always exactly
/// `body_size_pt` itself, so text with no size declaration in effect renders
/// identically to before this existed, even for the 11pt class, where this
/// compiler's own body size is a literal 11pt rather than real LaTeX's
/// 10.95pt normalsize (`class_size_pt`'s documented approximation).
fn size_declaration_pt(level: FontSizeLevel, body_size_pt: f64) -> f64 {
    // tiny, scriptsize, footnotesize, small, large, Large, LARGE, huge, Huge
    // (normalsize is handled by the caller before reaching here).
    const SIZE_10PT: [f64; 9] = [5.0, 7.0, 8.0, 9.0, 12.0, 14.4, 17.28, 20.74, 24.88];
    const SIZE_11PT: [f64; 9] = [6.0, 8.0, 9.0, 10.0, 12.0, 14.4, 17.28, 20.74, 24.88];
    const SIZE_12PT: [f64; 9] = [6.0, 8.0, 10.0, 10.95, 14.4, 17.28, 20.74, 24.88, 24.88];
    let table = if body_size_pt <= 10.5 {
        SIZE_10PT
    } else if body_size_pt <= 11.5 {
        SIZE_11PT
    } else {
        SIZE_12PT
    };
    let index = match level {
        FontSizeLevel::Tiny => 0,
        FontSizeLevel::ScriptSize => 1,
        FontSizeLevel::FootnoteSize => 2,
        FontSizeLevel::Small => 3,
        FontSizeLevel::Large1 => 4,
        FontSizeLevel::Large2 => 5,
        FontSizeLevel::Large3 => 6,
        FontSizeLevel::Huge1 => 7,
        FontSizeLevel::Huge2 => 8,
    };
    table[index]
}

/// Height above and depth below the baseline of `font` at `size`, from the
/// face's declared ascender/descender. Symbol declares none (its AFM bounding
/// box is far taller than its glyphs), so it uses Times-Roman's.
fn font_extents(font: Font, size: f64) -> (f64, f64) {
    use flashtex_font_engine::Face as _;
    let font = if font == Font::Symbol {
        Font::TimesRoman
    } else {
        font
    };
    let metrics = face(font).vertical_metrics();
    (
        f64::from(metrics.ascender) * size / 1000.0,
        -f64::from(metrics.descender) * size / 1000.0,
    )
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Cumulative left margin, in points, for an `itemize`/`enumerate` item at
/// `level` (1 = outermost), scaled by the document body size. Enclosing
/// levels always use their `LIST_LEFTMARGIN_EM` default; `own_override_pt`,
/// when given, replaces only `level`'s own share (a `\setlist{leftmargin=...}`
/// override on the item's own list — see `list_leftmargin_override_pt`).
fn list_margin_pt(level: u8, body_size_pt: f64, own_override_pt: Option<f64>) -> f64 {
    let depth = level.max(1) as usize;
    let outer_em: f64 = (1..depth)
        .map(|l| LIST_LEFTMARGIN_EM[(l - 1).min(LIST_LEFTMARGIN_EM.len() - 1)])
        .sum();
    let own_pt = own_override_pt.unwrap_or_else(|| {
        LIST_LEFTMARGIN_EM[(depth - 1).min(LIST_LEFTMARGIN_EM.len() - 1)] * body_size_pt
    });
    outer_em * body_size_pt + own_pt
}

/// Resolves a `Block::ListItem`'s `leftmargin` into the absolute point value
/// `list_margin_pt` should use for that item's own level, or `None` to keep
/// the level's `LIST_LEFTMARGIN_EM` default (no `\setlist{leftmargin=...}`
/// override, or a `leftmargin=*` list with no items to measure).
fn list_leftmargin_override_pt(leftmargin: &ListLeftMargin, body_size_pt: f64) -> Option<f64> {
    match leftmargin {
        ListLeftMargin::Default => None,
        ListLeftMargin::Explicit(pt) => Some(*pt),
        ListLeftMargin::Widest(labels) => labels
            .iter()
            .map(|label| glyph_width(label, body_size_pt, Font::TimesRoman))
            .reduce(f64::max)
            .map(|widest_pt| widest_pt + LIST_LABELSEP_EM * body_size_pt),
    }
}

pub fn layout(blocks: &[Block]) -> Vec<Page> {
    let mut c = LayoutCursor::with_labels(LayoutConstraints::default(), BTreeMap::new(), false);
    for block in blocks {
        c.prepare_block(block);
        c.render_prepared_block(block);
    }
    c.into_pages()
}

pub fn layout_with_constraints(blocks: &[Block], constraints: LayoutConstraints) -> Vec<Page> {
    let mut c = LayoutCursor::new(constraints);
    for block in blocks {
        c.prepare_block(block);
        c.render_prepared_block(block);
    }
    c.into_pages()
}

/// Everything one layout pass feeds the next: label values with their pages,
/// and the contents entries.
type CrossReferences = (BTreeMap<String, ReferenceValue>, Vec<TocEntry>);

/// Lay out repeatedly until label values, their page numbers and the contents
/// entries stabilize — LaTeX's rerun cycle, bounded by
/// `REFERENCE_ITERATION_LIMIT`. Layout is a pure function of its input, so the
/// result is deterministic; a pass that reproduces an earlier non-adjacent
/// state is a page-number oscillation and stops early with a warning.
pub fn layout_converged(
    blocks: &[Block],
    constraints: LayoutConstraints,
) -> (Vec<Page>, Vec<Diagnostic>) {
    let collect_toc = blocks
        .iter()
        .any(|block| matches!(block, Block::TableOfContents { .. }));
    let mut state: CrossReferences = (BTreeMap::new(), Vec::new());
    let mut history: Vec<CrossReferences> = Vec::new();
    let mut last_pages = Vec::new();
    let mut diagnostics = Vec::new();
    let mut converged = false;
    let mut oscillating = false;
    for _ in 0..REFERENCE_ITERATION_LIMIT {
        let mut cursor = LayoutCursor::with_labels(constraints, state.0.clone(), true);
        cursor.resolved_toc = state.1.clone();
        cursor.collect_toc = collect_toc;
        for block in blocks {
            cursor.prepare_block(block);
            cursor.render_prepared_block(block);
        }
        let (pages, next, shape_diagnostics) = cursor.into_result();
        last_pages = pages;
        diagnostics = shape_diagnostics;
        if next == state {
            converged = true;
            break;
        }
        if history.contains(&next) {
            oscillating = true;
            break;
        }
        history.push(std::mem::replace(&mut state, next));
    }

    visit_references(blocks, &mut |key, span| {
        if !state.0.contains_key(key) {
            let page = last_pages
                .iter()
                .find(|page| page.items.iter().any(|item| item.span == span))
                .map_or_else(String::new, |page| format!(" on page {}", page.number));
            diagnostics.push(Diagnostic::warning(
                format!("Reference `{key}'{page} undefined"),
                Some(span),
                Some("rendered ?? for the unresolved reference".into()),
            ));
        }
    });
    if oscillating {
        diagnostics.push(Diagnostic::warning(
            "cross-reference page numbers oscillate between layout passes",
            None,
            Some(
                "returned the last layout pass; its page references may be off by the oscillation"
                    .into(),
            ),
        ));
    } else if !converged {
        diagnostics.push(Diagnostic::warning(
            format!(
                "cross-reference values did not converge after {REFERENCE_ITERATION_LIMIT} layout passes"
            ),
            None,
            Some("returned the final bounded layout pass".into()),
        ));
    }
    (last_pages, diagnostics)
}

fn visit_references(blocks: &[Block], visitor: &mut impl FnMut(&str, Span)) {
    for block in blocks {
        match block {
            Block::Paragraph(inlines) => visit_inline_references(inlines, visitor),
            Block::ListItem { content, .. }
            | Block::Heading { content, .. }
            | Block::FigureCaption { content }
            | Block::Styled { content, .. } => visit_inline_references(content, visitor),
            Block::TitleBlock {
                title,
                authors,
                date,
            } => {
                visit_inline_references(title, visitor);
                visit_inline_references(authors, visitor);
                if let Some(date) = date {
                    visit_inline_references(date, visitor);
                }
            }
            Block::VSpace { .. }
            | Block::Rule { .. }
            | Block::PageBreak
            | Block::Verbatim { .. }
            | Block::TableOfContents { .. }
            | Block::VFill => {}
        }
    }
}

fn visit_inline_references(inlines: &[Inline], visitor: &mut impl FnMut(&str, Span)) {
    for inline in inlines {
        match inline {
            Inline::Reference { key, span, .. } => visitor(key, *span),
            Inline::Footnote {
                text: Some(text), ..
            } => visit_inline_references(text, visitor),
            Inline::Tabular(table) => {
                for list in table.inline_lists() {
                    visit_inline_references(list, visitor);
                }
            }
            Inline::Transform(b) => visit_inline_references(&b.content, visitor),
            _ => {}
        }
    }
}

/// The Core 14 face for a text style. Times has all four weight/shape
/// variants; the engine carries only upright medium Helvetica and Courier, so
/// bold or italic sans/typewriter text uses those faces unchanged.
pub(crate) fn style_font(style: TextStyle) -> Font {
    match (style.family, style.bold, style.italic) {
        (TextFamily::Sans, ..) => Font::Helvetica,
        (TextFamily::Mono, ..) => Font::Courier,
        (TextFamily::Roman, false, false) => Font::TimesRoman,
        (TextFamily::Roman, true, false) => Font::TimesBold,
        (TextFamily::Roman, false, true) => Font::TimesItalic,
        (TextFamily::Roman, true, true) => Font::TimesBoldItalic,
    }
}

fn emit(c: &mut LayoutCursor, inlines: &[Inline], size: f64, font: Font) {
    for inline in inlines {
        match inline {
            Inline::Text {
                text,
                span,
                style,
                space_before,
            } => {
                // A `\tiny`..`\Huge` declaration is always relative to the
                // document's own body size, not to `size` (which can already
                // be a heading's or a math script's own scaled context).
                let text_size = style.size.map_or(size, |level| {
                    size_declaration_pt(level, c.constraints.font_size_pt)
                });
                c.place(
                    text.clone(),
                    text_size,
                    *span,
                    style_font(*style),
                    *space_before,
                )
            }
            Inline::LineBreak { .. } => c.newline(size),
            Inline::TextGlue { em, .. } => c.text_glue(*em, size),
            Inline::Math {
                list,
                display,
                number,
                number_span,
                span,
                space_before,
                ..
            } => {
                let b = if *display {
                    math::layout_display(list, size, &mut c.diagnostics)
                } else {
                    math::layout(list, size, &mut c.diagnostics)
                };
                if *display {
                    c.display_math(
                        b,
                        size,
                        number
                            .as_deref()
                            .zip(*number_span)
                            .or_else(|| number.as_deref().map(|number| (number, *span))),
                    );
                } else {
                    c.place_math(b, size, *space_before);
                }
            }
            Inline::MathRows { rows, aligned, .. } => c.display_rows(rows, *aligned, size),
            Inline::Label { key, value, .. } => {
                c.collected_labels.insert(
                    key.clone(),
                    ReferenceValue {
                        number: value.clone(),
                        page: c.pages.len() as u32,
                    },
                );
            }
            Inline::Reference {
                key,
                page,
                equation,
                span,
                space_before,
            } => match c.resolved_labels.get(key) {
                Some(value) => {
                    let text = if *page {
                        value.page.to_string()
                    } else {
                        value.number.clone()
                    };
                    let text = if *equation { format!("({text})") } else { text };
                    c.place(text, size, *span, font, *space_before);
                }
                // `\@setref`: an undefined key typesets a bold `??`.
                None if *equation => {
                    c.place("(".to_string(), size, *span, font, *space_before);
                    c.place("??".to_string(), size, *span, Font::TimesBold, false);
                    c.place(")".to_string(), size, *span, font, false);
                }
                None => c.place(
                    "??".to_string(),
                    size,
                    *span,
                    Font::TimesBold,
                    *space_before,
                ),
            },
            Inline::HFill { leader, span } => c.mark_hfill(*leader, size, font, *span),
            Inline::HSpace { pt, .. } => c.hspace(*pt),
            Inline::Footnote {
                number,
                span,
                mark,
                text,
                space_before,
            } => c.footnote(number, *span, *mark, text.as_deref(), *space_before),
            Inline::Tabular(table) => {
                let table_size = table.style.size.map_or(size, |level| {
                    size_declaration_pt(level, c.constraints.font_size_pt)
                });
                let b = crate::tabular::layout(c, table, table_size);
                c.place_math(b, size, table.space_before);
            }
            Inline::Verbatim {
                text,
                span,
                space_before,
            } => c.place(text.clone(), size, *span, Font::Courier, *space_before),
            // The Core 14 layout has no box model: the content is set inline.
            Inline::ColorBox(b) => emit(c, &b.content, size, font),
            // This Core 14 layout reads no image files and has no transformed
            // boxes; the rendering pipeline sets both (`crate::graphics`).
            Inline::Graphic(g) => c.diagnostics.push(
                Diagnostic::warning(
                    "\\includegraphics: this layout does not load or draw images",
                    Some(g.span),
                    Some("left no space for the image".into()),
                )
                .with_code(crate::diagnostics::DiagnosticCode::UnsupportedFeature),
            ),
            Inline::Transform(b) => {
                c.diagnostics.push(
                    Diagnostic::warning(
                        "graphics transforms are not applied by this layout",
                        Some(b.span),
                        Some("set the content untransformed".into()),
                    )
                    .with_code(crate::diagnostics::DiagnosticCode::UnsupportedFeature),
                );
                emit(c, &b.content, size, font);
            }
            Inline::Logo {
                logo,
                span,
                style,
                space_before,
            } => {
                let text_size = style.size.map_or(size, |level| {
                    size_declaration_pt(level, c.constraints.font_size_pt)
                });
                c.place_logo(*logo, text_size, *span, style_font(*style), *space_before)
            }
            Inline::Kern { amount, style, .. } => {
                let text_size = style.size.map_or(size, |level| {
                    size_declaration_pt(level, c.constraints.font_size_pt)
                });
                let cx = crate::text_builtins::DimenContext {
                    quad: crate::text_builtins::pt_to_sp(text_size),
                    ..Default::default()
                };
                c.hspace(crate::text_builtins::sp_to_pt(amount.resolve(&cx)))
            }
            Inline::Rule {
                rule,
                span,
                style,
                space_before,
            } => {
                let text_size = style.size.map_or(size, |level| {
                    size_declaration_pt(level, c.constraints.font_size_pt)
                });
                c.place_rule(rule, text_size, *span, style_font(*style), *space_before)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn laid_out(source: &str) -> (parser::Parsed, Vec<Page>) {
        let parsed = parser::parse(source);
        let pages = layout(&parsed.blocks);
        (parsed, pages)
    }

    fn item_at(pages: &[Page], start: usize) -> &TextItem {
        pages
            .iter()
            .flat_map(|p| &p.items)
            .find(|item| item.span.start == start)
            .expect("expected item at source offset")
    }

    #[test]
    fn inline_and_display_math_both_produce_positioned_items() {
        let source = "before $x$ after $$y$$ end";
        let (parsed, pages) = laid_out(source);
        assert!(parsed.diagnostics.is_empty());
        let x = item_at(&pages, source.find('x').unwrap());
        let y = item_at(&pages, source.find('y').unwrap());
        assert_eq!(x.font_size_pt, BODY_SIZE_PT);
        assert_eq!(x.baseline_y_pt, item_at(&pages, 0).baseline_y_pt);
        assert!(
            (y.x_pt - (PAGE_WIDTH_PT - glyph_width("y", BODY_SIZE_PT, Font::TimesItalic)) / 2.0)
                .abs()
                < 0.02
        );
        assert!(y.baseline_y_pt > x.baseline_y_pt);
    }

    #[test]
    fn fraction_stacks_smaller_children_and_advances_as_one_unit() {
        let source = "$\\frac{1}{2}z$";
        let (_, pages) = laid_out(source);
        let numerator = item_at(&pages, source.find('1').unwrap());
        let denominator = item_at(&pages, source.find('2').unwrap());
        let following = item_at(&pages, source.find('z').unwrap());
        assert!(numerator.baseline_y_pt < denominator.baseline_y_pt);
        assert!(numerator.font_size_pt < BODY_SIZE_PT);
        assert!(denominator.font_size_pt < BODY_SIZE_PT);
        assert!(following.x_pt > numerator.x_pt && following.x_pt > denominator.x_pt);
    }

    #[test]
    fn scripts_attach_to_atoms_and_nested_scripts_decrease_in_size() {
        let source = "$x^2 x_i x^{a+b} x^{y^z}$";
        let (_, pages) = laid_out(source);
        let two = item_at(&pages, source.find('2').unwrap());
        let i = item_at(&pages, source.find('i').unwrap());
        let a = item_at(&pages, source.find('a').unwrap());
        let plus = item_at(&pages, source.find('+').unwrap());
        let y = item_at(&pages, source.find('y').unwrap());
        let z = item_at(&pages, source.find('z').unwrap());
        assert_eq!(two.font_size_pt, BODY_SIZE_PT * math::SCRIPT_SCALE);
        assert_eq!(i.font_size_pt, BODY_SIZE_PT * math::SCRIPT_SCALE);
        assert_eq!(a.font_size_pt, two.font_size_pt);
        assert_eq!(plus.font_size_pt, two.font_size_pt);
        assert!(z.font_size_pt < y.font_size_pt);
        assert_eq!(
            z.font_size_pt,
            BODY_SIZE_PT * math::SECOND_ORDER_SCRIPT_SCALE
        );
        assert!(two.baseline_y_pt < item_at(&pages, source.find('x').unwrap()).baseline_y_pt);
        assert!(
            i.baseline_y_pt > item_at(&pages, source[5..].find('x').unwrap() + 5).baseline_y_pt
        );
    }

    #[test]
    fn unknown_command_and_unclosed_shift_recover_without_losing_content() {
        let source = "$x+\\unknown";
        let (parsed, pages) = laid_out(source);
        let messages: Vec<_> = parsed
            .diagnostics
            .iter()
            .map(|d| d.message.as_str())
            .collect();
        assert!(messages.iter().any(|m| m.contains("\\unknown")));
        assert!(messages
            .iter()
            .any(|m| m.contains("missing its closing '$'")));
        assert!(pages
            .iter()
            .any(|p| p.items.iter().any(|i| i.text == "\\unknown")));
    }

    #[test]
    fn every_math_item_span_is_a_valid_source_slice() {
        let source = "$é^2+\\alpha+\\frac{1}{β_3}+\\sqrt{x}$";
        let (parsed, pages) = laid_out(source);
        assert!(parsed.diagnostics.is_empty());
        for item in pages.iter().flat_map(|p| &p.items) {
            let slice = &source[item.span.start..item.span.end];
            assert!(!slice.is_empty());
        }
    }

    #[test]
    fn tall_math_increases_the_distance_to_the_next_line() {
        let plain = laid_out("top\\\\ bottom").1;
        let tall = laid_out("top $\\frac{1}{2}$\\\\ bottom").1;
        let plain_gap = item_at(&plain, 6).baseline_y_pt - item_at(&plain, 0).baseline_y_pt;
        let tall_gap = item_at(&tall, 20).baseline_y_pt - item_at(&tall, 0).baseline_y_pt;
        assert!(tall_gap > plain_gap);
    }

    #[test]
    fn core14_shaping_applies_kerning_and_keeps_document_byte_identity() {
        let unkerned = text_width("A", BODY_SIZE_PT, Font::TimesRoman)
            + text_width("V", BODY_SIZE_PT, Font::TimesRoman);
        let kerned = text_width("AV", BODY_SIZE_PT, Font::TimesRoman);
        assert!(kerned < unkerned, "the Times-Roman AV pair must kern");

        let source = "é AV";
        let (_, pages) = laid_out(source);
        let item = item_at(&pages, source.find("AV").unwrap());
        assert_eq!(&source[item.span.start..item.span.end], "AV");
    }

    #[test]
    fn shaping_missing_glyph_diagnostic_uses_document_not_relative_offset() {
        let source = "prefix 東京";
        let output = crate::incremental::compile_full(source, LayoutConstraints::default());
        let tokyo = source.find('東').unwrap();
        assert!(output.diagnostics.iter().any(|diagnostic| {
            diagnostic.span == Some(Span::new(tokyo, tokyo + '東'.len_utf8()))
                && diagnostic.message.contains("U+6771")
        }));

        let origin = Span::in_document(crate::DocumentId(4), 100, 104);
        let mut diagnostics = Vec::new();
        let (_, shaped_span) = shaped_width(
            "A日",
            BODY_SIZE_PT,
            Font::TimesRoman,
            origin,
            &mut diagnostics,
        );
        assert_eq!(shaped_span, origin);
        assert_eq!(
            diagnostics[0].span,
            Some(Span::in_document(crate::DocumentId(4), 101, 104))
        );
    }

    #[test]
    fn body_and_heading_faces_do_not_depend_on_numeric_font_size() {
        let parsed = parser::parse("body\n\n\\section{heading}\n");
        let pages = layout_with_constraints(
            &parsed.blocks,
            LayoutConstraints {
                font_size_pt: 11.0,
                measure_pt: LayoutConstraints::default().measure_pt,
                parskip_pt: None,
            },
        );
        let body = pages
            .iter()
            .flat_map(|page| &page.items)
            .find(|item| item.text == "body")
            .unwrap();
        let heading = pages
            .iter()
            .flat_map(|page| &page.items)
            .find(|item| item.text == "heading")
            .unwrap();
        assert_eq!(body.font, Font::TimesRoman);
        assert_eq!(heading.font, Font::TimesBold);
    }

    #[test]
    fn unsupported_shaping_is_an_explicit_source_mapped_error() {
        let source = "before אב";
        let output = crate::incremental::compile_full(source, LayoutConstraints::default());
        let start = source.find('א').unwrap();
        assert!(output.diagnostics.iter().any(|diagnostic| {
            diagnostic.span == Some(Span::new(start, start + 'א'.len_utf8()))
                && diagnostic.message.contains("could not shape text")
                && diagnostic.message.contains("Hebrew needs bidi reordering")
        }));
    }

    #[test]
    fn hfill_pushes_following_text_to_the_right_edge() {
        let source = "left\\hfill right";
        let (parsed, pages) = laid_out(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let right = item_at(&pages, source.find("right").unwrap());
        let width = glyph_width("right", BODY_SIZE_PT, Font::TimesRoman);
        assert!(
            (right.x_pt + width - (PAGE_WIDTH_PT - MARGIN_PT)).abs() < 0.02,
            "right edge was {}, expected {}",
            right.x_pt + width,
            PAGE_WIDTH_PT - MARGIN_PT
        );
    }

    #[test]
    fn hspace_advances_by_the_requested_dimension() {
        let source = "left\\hspace{12pt}right";
        let (parsed, pages) = laid_out(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let left = item_at(&pages, 0);
        let right = item_at(&pages, source.find("right").unwrap());
        let left_width = glyph_width("left", BODY_SIZE_PT, Font::TimesRoman);
        assert!((right.x_pt - left.x_pt - left_width - 12.0).abs() < 0.02);
    }

    #[test]
    fn list_item_label_ends_labelsep_before_the_hanging_indent() {
        let source = "\\begin{itemize}\\item Text\\end{itemize}";
        let (parsed, pages) = laid_out(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let items: Vec<&TextItem> = pages.iter().flat_map(|p| &p.items).collect();
        let label = items
            .iter()
            .find(|item| item.text == "•")
            .expect("bullet label item");
        let text = items
            .iter()
            .find(|item| item.text == "Text")
            .expect("item text");
        let text_x = MARGIN_PT + list_margin_pt(1, BODY_SIZE_PT, None);
        let label_sep = LIST_LABELSEP_EM * BODY_SIZE_PT;
        let label_width = glyph_width("•", BODY_SIZE_PT, Font::TimesRoman);
        assert_eq!(text.x_pt, round2(text_x));
        assert_eq!(label.x_pt, round2(text_x - label_sep - label_width));
        assert_eq!(label.baseline_y_pt, text.baseline_y_pt);
    }

    #[test]
    fn wrapped_continuation_lines_keep_the_same_hanging_indent() {
        let words = "sample ".repeat(20);
        let source = format!("\\begin{{itemize}}\\item {}\\end{{itemize}}", words.trim());
        let (parsed, pages) = laid_out(&source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let mut baselines: Vec<f64> = pages[0]
            .items
            .iter()
            .map(|item| item.baseline_y_pt)
            .collect();
        baselines.sort_by(|a, b| a.partial_cmp(b).unwrap());
        baselines.dedup();
        assert!(
            baselines.len() >= 2,
            "expected the item text to wrap onto a second line"
        );
        let first_on_wrapped_line = pages[0]
            .items
            .iter()
            .filter(|item| item.baseline_y_pt == baselines[1])
            .min_by(|a, b| a.x_pt.partial_cmp(&b.x_pt).unwrap())
            .expect("an item on the wrapped line");
        assert_eq!(
            first_on_wrapped_line.x_pt,
            round2(MARGIN_PT + list_margin_pt(1, BODY_SIZE_PT, None))
        );
    }

    #[test]
    fn nested_list_levels_add_their_own_margin_cumulatively() {
        let source =
            "\\begin{itemize}\\item Outer\\begin{itemize}\\item Inner\\end{itemize}\\end{itemize}";
        let (parsed, pages) = laid_out(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let items: Vec<&TextItem> = pages.iter().flat_map(|p| &p.items).collect();
        let outer = items.iter().find(|item| item.text == "Outer").unwrap();
        let inner = items.iter().find(|item| item.text == "Inner").unwrap();
        assert_eq!(
            outer.x_pt,
            round2(MARGIN_PT + list_margin_pt(1, BODY_SIZE_PT, None))
        );
        assert_eq!(
            inner.x_pt,
            round2(MARGIN_PT + list_margin_pt(2, BODY_SIZE_PT, None))
        );
        assert!(
            inner.x_pt > outer.x_pt,
            "a nested item indents further than its enclosing item"
        );
    }

    #[test]
    fn bibliography_label_width_comes_from_the_widest_label_argument() {
        let source = "\\begin{thebibliography}{9}\\bibitem{a}Text\\end{thebibliography}";
        let (parsed, pages) = laid_out(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let items: Vec<&TextItem> = pages.iter().flat_map(|p| &p.items).collect();
        let label = items
            .iter()
            .find(|item| item.text == "[1]")
            .expect("bibitem label");
        let text = items
            .iter()
            .find(|item| item.text == "Text")
            .expect("item text");
        let label_sep = LIST_LABELSEP_EM * BODY_SIZE_PT;
        let widest_width = glyph_width("[9]", BODY_SIZE_PT, Font::TimesRoman);
        let text_x = MARGIN_PT + widest_width + label_sep;
        assert_eq!(text.x_pt, round2(text_x));
        assert_eq!(
            label.x_pt,
            round2(text_x - label_sep - glyph_width("[1]", BODY_SIZE_PT, Font::TimesRoman))
        );
        // Not the `itemize`/`enumerate` leftmargin table's fixed 2.5em: a
        // `thebibliography`'s indent tracks its own widest-label argument.
        assert_ne!(
            round2(text_x),
            round2(MARGIN_PT + list_margin_pt(1, BODY_SIZE_PT, None))
        );
    }

    #[test]
    fn thebibliography_emits_an_unnumbered_bold_references_heading() {
        let source = "\\begin{thebibliography}{9}\\bibitem{a}Text\\end{thebibliography}";
        let (parsed, pages) = laid_out(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let items: Vec<&TextItem> = pages.iter().flat_map(|p| &p.items).collect();
        let heading = items
            .iter()
            .find(|item| item.text == "References")
            .expect("References heading");
        assert_eq!(heading.font, Font::TimesBold);
        assert!(
            matches!(&parsed.blocks[0], parser::Block::Heading { number, .. } if number.is_empty()),
            "the References heading must be unnumbered, like \\section*"
        );
    }

    #[test]
    fn a_blank_line_inside_an_item_keeps_the_indent_without_repeating_the_label() {
        let source = "\\begin{itemize}\\item First\n\nSecond\\end{itemize}";
        let (parsed, pages) = laid_out(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let list_items: Vec<(u8, bool)> = parsed
            .blocks
            .iter()
            .filter_map(|block| match block {
                parser::Block::ListItem { level, label, .. } => Some((*level, label.is_some())),
                _ => None,
            })
            .collect();
        assert_eq!(
            list_items,
            [(1, true), (1, false)],
            "the blank line must start a second paragraph of the same item, not a new item"
        );
        let second = pages
            .iter()
            .flat_map(|p| &p.items)
            .find(|item| item.text == "Second")
            .expect("continuation paragraph text");
        assert_eq!(
            second.x_pt,
            round2(MARGIN_PT + list_margin_pt(1, BODY_SIZE_PT, None))
        );
    }

    #[test]
    fn a_label_wider_than_its_margin_overflows_left_without_pushing_the_item_text() {
        let source =
            "\\begin{enumerate}[label=Preposterously-Long-Label-\\arabic*]\\item Text\\end{enumerate}";
        let (parsed, pages) = laid_out(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let items: Vec<&TextItem> = pages.iter().flat_map(|p| &p.items).collect();
        let label = items
            .iter()
            .find(|item| item.text.starts_with("Preposterously"))
            .expect("overlong label item");
        let text = items.iter().find(|item| item.text == "Text").unwrap();
        assert_eq!(
            text.x_pt,
            round2(MARGIN_PT + list_margin_pt(1, BODY_SIZE_PT, None)),
            "the item text must stay at the normal hanging-indent margin"
        );
        assert!(
            label.x_pt < MARGIN_PT,
            "an overlong label should overflow past the page margin, like LaTeX's overfull label box (got {})",
            label.x_pt
        );
    }

    /// Right edge of each line (by baseline) on the first page, in order.
    fn line_ends(pages: &[Page]) -> Vec<f64> {
        let mut ends: Vec<(f64, f64)> = Vec::new();
        for item in &pages[0].items {
            let end = item.x_pt + glyph_width(&item.text, item.font_size_pt, item.font);
            match ends.last_mut() {
                Some((y, e)) if *y == item.baseline_y_pt => *e = e.max(end),
                _ => ends.push((item.baseline_y_pt, end)),
            }
        }
        ends.into_iter().map(|(_, end)| end).collect()
    }

    #[test]
    fn wrapped_lines_are_justified_but_last_forced_and_centered_lines_are_not() {
        let words = "Justified text stretches its spaces evenly. ".repeat(8);
        let right = PAGE_WIDTH_PT - MARGIN_PT;
        let ends = line_ends(&laid_out(&format!("{words}\\\\ {words}")).1);
        assert!(ends.len() >= 5, "{ends:?}");
        // Each paragraph piece: wrapped lines justified, the line closed by
        // `\\` and the paragraph's final line left ragged.
        let ragged: Vec<bool> = ends.iter().map(|end| (end - right).abs() > 0.05).collect();
        assert_eq!(ragged.iter().filter(|r| **r).count(), 2, "{ends:?}");
        assert!(ragged[ragged.len() - 1]);

        let centered = line_ends(&laid_out(&format!("\\begin{{center}}{words}\\end{{center}}")).1);
        assert!(centered.len() >= 3);
        assert!(
            centered.iter().all(|end| (end - right).abs() > 0.05),
            "{centered:?}"
        );

        // A line with infinite glue keeps the `\hfill` distribution only.
        let filled = laid_out(&format!("a\\hfill b {words}")).1;
        let items = &filled[0].items;
        let space = word_space(BODY_SIZE_PT, Font::TimesRoman);
        let natural_after_b = items[1].x_pt + glyph_width("b", BODY_SIZE_PT, Font::TimesRoman);
        assert!((items[2].x_pt - natural_after_b - space).abs() < 0.05);
        assert!((line_ends(&filled)[0] - right).abs() < 0.05);
    }
}
