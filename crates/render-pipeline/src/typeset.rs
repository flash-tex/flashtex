//! Paragraph assembly, math boxes, pages, and the v2 display list.
//!
//! Words are shaped through font-engine and become `paragraph-layout` boxes
//! (`GlyphRun::from_shaped`, one per styled segment: advances in font units
//! with kerning folded in, original glyph ids, source-byte clusters).
//! Interword glue follows TeX's space factor and the face's `\fontdimen`s.
//! Inline math is laid out by `math-layout` (Appendix G) and enters the
//! horizontal list as one unbreakable box; a display equation is a
//! one-line block between `\abovedisplayskip`/`\belowdisplayskip` (the
//! short variants when the preceding line leaves room, TeX §1199). Line
//! breaking is `paragraph-layout`'s total-fit Knuth–Plass; page breaking is
//! `pagebuild` (TeX's page builder: interline glue, `\topskip`, penalty
//! costs for club/widow lines and `\nobreak` after headings,
//! `\raggedbottom`). This module keeps a record per box so every placed run
//! maps back to its document, bytes, glyph extents and math box.

use flashtex_compiler::text_builtins::TextLogo;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::rc::Rc;
use std::sync::OnceLock;

use flashtex_compiler::parser::{FillLeader, SourceDocument};
use flashtex_compiler::{DocumentId, Span};
use flashtex_font_engine::sha256;
use flashtex_math_layout as ml;
use flashtex_paragraph_layout as pl;
use flashtex_paragraph_layout::Hyphenator as _;

use crate::adapter::{self, Block, Doc, Item as AItem, ListGeom, ListMargin, ParaPart, ParaStyle, TextStyle};
use crate::display::{
    self, Caret, Cluster, Diagnostic, DisplayList, DocumentResource, FontResource, Glyph, GlyphRun, Paint, Provenance,
    Rect, Rule, SourceRange, Tick,
};
use crate::fonts::{Family, FontSet, LoadedFace, Role};
use crate::incremental::{self, CachedBlock, RenderCache};
use crate::mathfont::{MathFonts, MathSizes};
use crate::mathtex::TexMathMetrics;
use crate::pagebuild::{self, VBlock};
use crate::params;
use crate::shape::Shaper;
use crate::style::Stylesheet;

/// paragraph-layout identity of a math box (never a real font hash: real
/// ids are SHA-256 digests, this is a labelled sentinel).
const MATH_SENTINEL: pl::FontId = pl::FontId::from_label("flashtex:math-box");

/// `\hyphenpenalty` and `\exhyphenpenalty` (plain TeX and article: 50).
const HYPHEN_PENALTY: i32 = 50;
const EX_HYPHEN_PENALTY: i32 = 50;

/// pdflatex's default `english` hyphenation (Knuth's `hyphen.tex`,
/// `\lefthyphenmin 2`, `\righthyphenmin 3`, `\uchyph 1`), compiled once
/// per process.
fn english_hyphenator() -> &'static pl::LiangHyphenator {
    static ENGLISH: OnceLock<pl::LiangHyphenator> = OnceLock::new();
    ENGLISH.get_or_init(pl::LiangHyphenator::english)
}

#[derive(Debug, Clone)]
pub struct GlyphRec {
    pub gid: u16,
    /// TFM italic correction in fixwords (0 without a TFM).
    pub italic_fix: i32,
    pub x_offset_units: i32,
    pub y_offset_units: i32,
    pub y_max_units: i32,
    pub y_min_units: i32,
    pub empty: bool,
    /// TFM shaping: the character code, the advance and the font kern after
    /// the glyph (fixwords; `advance_fix - kern_fix` is the char width).
    pub tfm_code: Option<u8>,
    pub advance_fix: i32,
    pub kern_fix: i32,
}

#[derive(Debug, Clone)]
pub struct ClusterRec {
    /// Byte range into the run text.
    pub text_range: Range<usize>,
    pub span: Span,
    /// Glyph indices (into the run) belonging to this cluster.
    pub glyphs: Range<usize>,
}

#[derive(Clone)]
pub enum BoxRec {
    Text {
        face: Rc<LoadedFace>,
        size: f64,
        text: String,
        style: TextStyle,
        clusters: Vec<ClusterRec>,
        glyphs: Vec<GlyphRec>,
        /// Box height/depth in points (max glyph extents).
        height: f64,
        depth: f64,
        /// A hyphenation fragment (or the discretionary hyphen) that
        /// continues the word box before it on the same line: painted into
        /// that run, so an unbroken word is one glyph run as before.
        continues: bool,
        /// Baseline shift upward in points (`\LaTeX`'s raised `A`, `\TeX`'s
        /// lowered `E`); 0 for ordinary text.
        raise: f64,
    },
    Math(usize),
    /// `\hrule`: a filled rectangle `width` x `height` sitting on the line's
    /// baseline (depth 0), painted as a display-list rule.
    /// `\rule` boxes set `bottom` (the painted part's bottom above the
    /// baseline); nothing is painted when `width` or `height` is not positive.
    Rule { width: f64, height: f64, bottom: f64, span: Span },
    /// A leader attached to horizontal fill glue. It is painted after line
    /// breaking, when the glue's final width is known.
    Leader {
        leader: FillLeader,
        box_width: f64,
        dot: Option<(Rc<LoadedFace>, pl::GlyphRun)>,
    },
    /// A `tikzpicture`: its bounding box sits on the baseline (depth 0).
    Picture(Rc<PictureRec>),
    /// A `tabular` (`table.rs`): its cell lines and rules, set as one box
    /// whose origin is the table's reference baseline.
    Table(Rc<TableRec>),
    /// `\colorbox`/`\fcolorbox` (`Context::color_box`).
    ColorBox(Rc<ColorBoxRec>),
    /// ulem `\uline` (`Context::underline_box`).
    Underline(Rc<UnderlineRec>),
}

/// A laid-out `\colorbox`/`\fcolorbox`: the content as one line whose
/// runs are placed from the box's left edge, and the box's `width`,
/// `height` and `depth` including `\fboxsep` and the `rule` frame.
#[derive(Clone)]
pub struct ColorBoxRec {
    pub block: BuiltBlock,
    pub width: f64,
    pub height: f64,
    pub depth: f64,
    pub rule: f64,
    pub fill: flashtex_compiler::color::DeviceColor,
    pub frame: Option<flashtex_compiler::color::DeviceColor>,
    pub span: Span,
}

/// A laid-out `\uline`/`\sout`/`\underline`: the content as one line, plus
/// a `thickness` rule whose top is `ul_depth` from the baseline (positive
/// down). ulem `\uline`: `\dp` of `\hbox{{(j}}`; kernel `\underline`:
/// box depth + 3θ; `\sout`: −(0.55ex + thickness).
#[derive(Clone)]
pub struct UnderlineRec {
    pub block: BuiltBlock,
    pub width: f64,
    pub height: f64,
    pub depth: f64,
    pub thickness: f64,
    pub ul_depth: f64,
    pub span: Span,
}

/// A laid-out table (`Context::table_box`): every entry or `@{}` box as
/// its broken lines at their position, and the rules. y is measured down
/// from the table's baseline.
#[derive(Clone)]
pub struct TableRec {
    pub pieces: Vec<TablePiece>,
    pub rules: Vec<crate::table::PlacedRule>,
    /// colortbl fills, painted under the pieces and rules.
    pub fills: Vec<crate::table::PlacedRule>,
    pub span: Span,
}

#[derive(Clone)]
pub struct TablePiece {
    pub x: f64,
    /// The first line's baseline below the table's baseline.
    pub baseline: f64,
    pub block: BuiltBlock,
}

/// A compiled `tikzpicture` and its node text shaped for painting
/// (`texts[i]` belongs to `picture.texts[i]`).
#[derive(Clone)]
pub struct PictureRec {
    pub picture: flashtex_vector_graphics::tikz::Picture,
    pub texts: Vec<crate::tikz::ShapedText>,
    pub span: Span,
}

impl std::fmt::Debug for PictureRec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PictureRec")
            .field("span", &self.span)
            .field("items", &self.picture.items.len())
            .field("texts", &self.picture.texts.len())
            .finish()
    }
}

#[derive(Clone)]
pub struct MathRec {
    pub root: ml::MathBox,
    pub span: Span,
    pub face: Rc<LoadedFace>,
    /// The metrics the box was laid out with; maps placed glyphs to the
    /// face's glyph ids.
    pub metrics: MathProvider,
    /// `\text{...}` runs of this formula (`mathtext`), addressed by the
    /// placed glyphs' `font_id` above `RUN_FONT_BASE`.
    pub text_runs: Vec<crate::mathtext::TextRun>,
    /// The formula's colour (`adapter::Doc::math_colors`).
    pub color: Option<flashtex_compiler::color::DeviceColor>,
    /// Baseline shift upward in points (`\LaTeXe`'s subscript `ε`).
    pub raise: f64,
    /// Where a paragraph may break inside this text-style formula: TeX
    /// §760/§767 inserts `\binoppenalty` (700) after a Bin atom and
    /// `\relpenalty` (500) after a Rel atom (never when the next noad is a
    /// Rel, never after the last one), and only at the top level: a
    /// `\left...\right` body, a fraction or a script is packed by
    /// `clean_box` without penalties. Each entry is the index of the
    /// atom's box among `root`'s children (the root is flattened to one
    /// hbox when this is non-empty) and the penalty; `Context::math_pieces`
    /// cuts the formula there into consecutive boxes. Empty for display
    /// math and once the pieces are cut.
    pub inline_breaks: Vec<(usize, i32)>,
    /// A piece cut from the formula before it (`math_pieces`): its glyph
    /// runs continue the previous piece's items when they land on the
    /// same line, so an unbroken formula assembles exactly as one box.
    pub continues: bool,
    /// Paint for glyphs and rules whose source span lies inside a byte
    /// range of this formula's document (xcolor `\textcolor`/`\color` in
    /// math); the innermost range wins, unpainted leaves stay black.
    /// Empty until the compiler reports colour ranges (#150/#158).
    #[cfg(feature = "math-glyph-spans")]
    pub span_paints: Vec<(std::ops::Range<usize>, Paint)>,
}

impl MathRec {
    /// The paint of a leaf with math-layout provenance `tag`.
    #[cfg(feature = "math-glyph-spans")]
    pub fn paint_of(&self, tag: ml::SourceTag) -> Paint {
        tag.span
            .filter(|s| s.document as usize == self.span.document.0)
            .and_then(|s| innermost_paint(&self.span_paints, s.start, s.end))
            .unwrap_or(Paint::BLACK)
    }

    /// The `\text` run glyph a placed glyph stands for, if it is one.
    pub fn run_glyph(&self, g: &ml::PositionedGlyph) -> Option<&crate::mathtext::RunGlyph> {
        crate::mathtext::run_of(&self.text_runs, g.font_id)?.glyph_at(g.font_id, g.gid)
    }

    /// The face and original glyph id that draw a placed glyph: a `\text`
    /// run's own shaped glyph (the text face's cmap id, 0 for its interword
    /// space) or the math provider's mapping.
    pub fn otf_glyph(&self, g: &ml::PositionedGlyph) -> Option<(Rc<LoadedFace>, u16)> {
        // The OTF fallback ids (`u32::MAX`, `u32::MAX - 1`) lie above the
        // `\text` run ids: they must be answered by the provider, not
        // looked up as a run.
        if !crate::mathtex::is_provider_font(g.font_id) && g.font_id.0 >= crate::mathtext::RUN_FONT_BASE {
            let run = crate::mathtext::run_of(&self.text_runs, g.font_id)?;
            let glyph = run.glyph_at(g.font_id, g.gid)?;
            return Some((run.face.clone(), glyph.gid.0));
        }
        self.metrics.otf_glyph(g)
    }

    /// Whether a placed glyph is one of the cmex extensible pieces tex.web
    /// §713 stacks for a delimiter taller than every fixed size.
    pub fn extensible_piece(&self, g: &ml::PositionedGlyph) -> bool {
        if g.font_id.0 >= crate::mathtext::RUN_FONT_BASE {
            return false;
        }
        match &self.metrics {
            MathProvider::Tex(t) => t.extensible_piece(g.font_id, g.gid as u8, g.ch).is_some(),
            MathProvider::Otf(_) => false,
        }
    }

    /// The Latin Modern Math vertical assembly that paints such a run; see
    /// [`TexMathMetrics::vertical_assembly`].
    pub fn vertical_assembly(&self, ch: char, span: f64, size: f64) -> Option<Vec<(u16, f64)>> {
        match &self.metrics {
            MathProvider::Tex(t) => t.vertical_assembly(ch, span, size),
            MathProvider::Otf(_) => None,
        }
    }

    /// The TFM box a placed cmex glyph was laid out with; see
    /// [`TexMathMetrics::extension_box`].
    pub fn extension_box(&self, g: &ml::PositionedGlyph) -> Option<(f64, f64)> {
        if g.font_id.0 >= crate::mathtext::RUN_FONT_BASE {
            // Includes `OTF_FALLBACK_FONT`: no TFM box for those glyphs.
            return None;
        }
        match &self.metrics {
            MathProvider::Tex(t) => t.extension_box(g.font_id, g.gid as u8, g.size),
            MathProvider::Otf(_) => None,
        }
    }
}

/// Which metrics lay math out: TeX's TFMs (pdfLaTeX's geometry) when the
/// `lm` TFMs are installed, else the OpenType `MATH` table.
#[derive(Clone)]
pub enum MathProvider {
    Tex(Rc<TexMathMetrics>),
    Otf(Rc<MathFonts>),
}

impl MathProvider {
    fn metrics(&self) -> &dyn ml::MathFontMetrics {
        match self {
            MathProvider::Tex(t) => &**t,
            MathProvider::Otf(o) => &**o,
        }
    }
    fn otf(&self) -> &Rc<MathFonts> {
        match self {
            MathProvider::Tex(t) => t.otf_fonts(),
            MathProvider::Otf(o) => o,
        }
    }
    /// The face and original glyph id that draw a placed glyph.
    pub fn otf_glyph(&self, g: &ml::PositionedGlyph) -> Option<(Rc<LoadedFace>, u16)> {
        match self {
            MathProvider::Tex(_) if g.font_id == crate::mathtex::OTF_FALLBACK_FONT => Some((self.otf().face().clone(), g.gid)),
            MathProvider::Tex(_) if g.font_id == crate::mathtex::OTF_FALLBACK_BB_FONT => self.otf().bb_face().map(|f| (f.clone(), g.gid)),
            MathProvider::Tex(t) => t.otf_glyph(g.font_id, g.gid as u8, g.ch),
            MathProvider::Otf(o) if g.font_id == crate::mathfont::BB_FONT => o.bb_face().map(|f| (f.clone(), g.gid)),
            MathProvider::Otf(o) => Some((o.face().clone(), g.gid)),
        }
    }
}

/// One vertical-list block: its broken lines, the horizontal list they
/// index into, the map from item indices to box records, and how it enters
/// the page builder's vertical list.
#[derive(Clone)]
pub struct BuiltBlock {
    pub block: pl::ParagraphBlock,
    /// The horizontal list the block's lines index into.
    pub items: Vec<pl::Item>,
    pub recs: Vec<Option<usize>>,
    /// Penalties and skips around and inside the block (lines filled).
    pub vertical: VBlock,
    /// `\label` keys and the item index they precede.
    pub labels: Vec<(String, usize)>,
    /// `(cache key, first source byte)` when the block came through the
    /// cache, so its assembled items can be cached too.
    pub cache_key: Option<(u64, DocumentId, usize)>,
}

/// Resolved microtype font parameters (and the warning an unmodelled font
/// raised) by (metrics identity, size bits, options and family), per thread.
type MicrotypeFontEntry = (Option<Rc<flashtex_microtype::FontParams>>, Option<(String, String)>);
thread_local! {
    static MICROTYPE_FONTS: std::cell::RefCell<std::collections::HashMap<(String, u64, String), MicrotypeFontEntry>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// What `Block::Paragraph` carries from one block to the next.
struct ParaState {
    /// LaTeX's `\@afterheading` is still in force (`\clubpenalty 10000`).
    after_heading: bool,
    /// The open paragraph-shape environment began in vertical mode
    /// (`\@topsepadd` keeps `\partopsep` for the closing skip too).
    env_vmode: bool,
    /// The open environment's own `\@topsep`/`\@topsepadd`, when it set
    /// them itself (`adapter::EnvSkips`: every amsthm theorem-like
    /// environment does). Carried from the block that opened the
    /// environment to the one that closes it, like `env_vmode`.
    env_skips: Option<crate::adapter::EnvSkips>,
}

/// LaTeX/plain penalties (article defaults).
const CLUB_PENALTY: i32 = 150;
const WIDOW_PENALTY: i32 = 150;
const SEC_PENALTY: i32 = -300;
/// `\brokenpenalty` (latex.ltx: `\brokenpenalty 100`), added to the penalty
/// after a line the paragraph broke at a discretionary (TeX §890).
const BROKEN_PENALTY: i32 = 100;
const PREDISPLAY_PENALTY: i32 = pagebuild::INF_PENALTY;

/// Sets `run`'s glyphs at `x` on a line (what `layout_paragraph` does for
/// broken lines; used for the single-line display block).
fn position_run(run: &pl::GlyphRun, x: f64, baseline_y: f64) -> pl::PositionedRun {
    let mut off = 0.0;
    let glyphs = run
        .glyphs
        .iter()
        .map(|g| {
            let pg = pl::PositionedGlyph {
                gid: g.gid,
                x_offset: off,
                advance: g.advance + g.kern,
                cluster: g.cluster.clone(),
            };
            off += g.advance + g.kern;
            pg
        })
        .collect();
    pl::PositionedRun {
        x,
        baseline_y,
        width: run.width,
        font: run.font,
        size: run.size,
        glyphs,
        source: run.source.clone(),
        is_hyphen: false,
    }
}

pub mod floatpage;
pub mod footnotes;
mod toc;
pub mod multicol;

pub struct Laid {
    pub blocks: Vec<BuiltBlock>,
    pub pages: pl::Pages,
    pub recs: Vec<BoxRec>,
    pub maths: Vec<MathRec>,
    /// Image items per page number (FT-063 floats).
    pub images: Vec<(u32, display::Item)>,
    /// The page of every float `\label`.
    pub float_labels: Vec<(String, u32)>,
    /// Per page, per placed line: the x offset (TeX points) from the
    /// one-sided first-column left edge the blocks were assembled at (an even
    /// page's `\evensidemargin`, the second column's offset).
    pub line_dx: Vec<Vec<f64>>,
    /// `\thepage` of every page (empty without a resolved class frame).
    pub page_text: Vec<String>,
}

pub struct Context<'a> {
    fonts: &'a FontSet,
    style: &'a Stylesheet,
    paths: &'a [&'a str],
    /// Document sources (indexed like `paths`), read only to re-derive what
    /// the compiler's math list flattens (`\left`/`\right` fences).
    texts: &'a [&'a str],
    shaper: &'a Shaper,
    diagnostics: Vec<Diagnostic>,
    recs: Vec<BoxRec>,
    maths: Vec<MathRec>,
    math_fonts: Option<MathProvider>,
    math_unavailable: bool,
    /// Whether the project loads `amssymb`/`amsfonts`, which decides where
    /// `\mathbb` and the AMS symbol repertoire come from
    /// (`mathtext::TextSink::amsfonts`).
    ///
    /// This is a fact about the preamble: it is the same for every formula
    /// in the project, and `texts` cannot change for the life of a
    /// `Context`. It is therefore answered once, here, rather than by
    /// re-scanning every document for `\usepackage` per formula -- which
    /// made a whole-document render quadratic in (source size x formula
    /// count). A field rather than a memo so that `math_box` has no
    /// re-derivation to reach for.
    ams_symbol_fonts: bool,
    /// Whether `amsmath` itself is loaded. Distinct from
    /// [`Self::ams_symbol_fonts`]: `amssymb`/`amsfonts` bring the msam/msbm
    /// symbol fonts, they do **not** bring `amsmath`, and a document may
    /// load either without the other. `\big`..`\Bigg` key off this one,
    /// because `\bBigg@` is amsmath's (amsmath.sty 721-738) and the kernel
    /// keeps its own fixed lengths (fontmath.ltx 513-520) without it.
    ///
    /// Answered once, here, for the same reason `ams_symbol_fonts` is: a
    /// per-formula `\usepackage` re-scan made a whole-document render
    /// quadratic in (source size x formula count).
    amsmath_loaded: bool,
    reported: BTreeSet<String>,
    /// Diagnostics emitted while a cacheable block is being built (with
    /// their once-only keys, suppressed ones included).
    capture: Option<Vec<(Option<String>, Diagnostic)>>,
    path_rcs: std::cell::RefCell<BTreeMap<usize, Rc<str>>>,
    /// microtype's per-font pdfTeX parameters by (metrics identity, size).
    microtype_fonts: BTreeMap<(Rc<str>, u64), Option<Rc<flashtex_microtype::FontParams>>>,
    math_colors: std::collections::HashMap<(usize, usize, usize), flashtex_compiler::color::DeviceColor>,
    /// Box records of `\item` labels. LaTeX sets a label inside the
    /// `\@labels` hbox, so pdfTeX's line packer never expands its
    /// characters (`hpack` adds `char_stretch` only for character nodes
    /// of the line itself) and they take no part in the line's font
    /// stretch/shrink.
    label_recs: BTreeSet<usize>,
    /// Footnote texts met while building horizontal lists, and for each
    /// the box record its `\insert` follows (the mark, or the box before
    /// `\footnotetext`): see [`footnotes`].
    notes: Vec<footnotes::NoteSrc>,
    note_anchors: Vec<(usize, usize)>,
    /// The body in reading order (`adapter::reading_order`): where a float
    /// of an `\include`d file stands among the other documents' blocks.
    reading_order: Vec<Span>,
    /// `multicols` environments of the project (`multicol::attach`).
    multicol: multicol::State,
    /// Footnote marks are `\rlap`ped (article/report/book `\maketitle`).
    rlap_marks: bool,
    /// `\vadjust{\penalty}` nodes met by [`Context::hlist`] (adapter
    /// `Item::PagePenalty`): the item index of the undiscardable anchor and
    /// the penalty. The builder of the paragraph takes them after breaking.
    vadjusts: Vec<(usize, i32)>,
    /// The document's breaking parameters for the paragraph being built
    /// ([`adapter::BreakOverrides`]), set by [`Context::build_paragraph`].
    breaking: adapter::BreakOverrides,
    /// The English patterns with the document's `\hyphenation` exceptions,
    /// when it has any (`english_hyphenator` otherwise).
    hyphenator: Option<pl::LiangHyphenator>,
    /// Math providers for text sizes other than the body's (footnotes), by
    /// size in centipoints; `None` when that size's metrics are missing.
    math_fonts_sized: BTreeMap<u32, Option<MathProvider>>,
    /// LaTeX's `\@parboxrestore` is in force: the material is being set in
    /// a box of its own (a float body), not in the page's text. It sets
    /// `\parindent` and `\parskip` to zero and ends with `\sloppy`
    /// (`\tolerance 9999`, `\emergencystretch 3em`), so a paragraph inside
    /// the box breaks the way LaTeX breaks it there.
    parbox: bool,
}

impl<'a> Context<'a> {
    /// Formula colours (`adapter::Doc::math_colors`).
    /// `adapter::Labels::reading_order`, for placing floats.
    pub fn set_reading_order(&mut self, order: Vec<Span>) {
        self.reading_order = order;
    }

    pub fn set_math_colors(&mut self, colors: std::collections::HashMap<(usize, usize, usize), flashtex_compiler::color::DeviceColor>) {
        self.math_colors = colors;
    }

    pub fn new(fonts: &'a FontSet, style: &'a Stylesheet, paths: &'a [&'a str]) -> Context<'a> {
        Self::with_texts(fonts, style, paths, &[])
    }

    /// [`Context::new`] with the document sources, which lets `\left`/`\right`
    /// fences be recovered from the bytes before each delimiter.
    pub fn with_texts(fonts: &'a FontSet, style: &'a Stylesheet, paths: &'a [&'a str], texts: &'a [&'a str]) -> Context<'a> {
        Context {
            fonts,
            style,
            paths,
            texts,
            shaper: fonts.shaper(),
            diagnostics: Vec::new(),
            recs: Vec::new(),
            maths: Vec::new(),
            math_fonts: None,
            math_unavailable: false,
            ams_symbol_fonts: texts
                .iter()
                .any(|t| crate::adapter::package_options(t, "amssymb").is_some() || crate::adapter::package_options(t, "amsfonts").is_some()),
            amsmath_loaded: texts
                .iter()
                .any(|t| crate::adapter::package_options(t, "amsmath").is_some()),
            reported: BTreeSet::new(),
            capture: None,
            path_rcs: std::cell::RefCell::new(BTreeMap::new()),
            microtype_fonts: BTreeMap::new(),
            math_colors: Default::default(),
            label_recs: BTreeSet::new(),
            notes: Vec::new(),
            note_anchors: Vec::new(),
            reading_order: Vec::new(),
            parbox: false,
            multicol: multicol::State::default(),
            rlap_marks: false,
            vadjusts: Vec::new(),
            breaking: adapter::BreakOverrides::default(),
            hyphenator: (!style.hyphenation.is_empty()).then(|| {
                let mut h = pl::LiangHyphenator::english();
                // An entry TeX would reject (`\hyphenation{x1}`) is skipped.
                for word in &style.hyphenation {
                    let _ = h.add_exceptions(word);
                }
                h
            }),
            math_fonts_sized: BTreeMap::new(),
        }
    }

    /// Emits a diagnostic; with a key, only the first one per key is kept.
    fn emit(&mut self, key: Option<String>, d: Diagnostic) {
        let keep = match &key {
            Some(k) => self.reported.insert(k.clone()),
            None => true,
        };
        if let Some(c) = &mut self.capture {
            c.push((key, d.clone()));
        }
        if keep {
            self.diagnostics.push(d);
        }
    }

    /// Builds a block through the cache: a hit clones the cached records
    /// back (offsets relocated to the block's new position) and replays
    /// its diagnostics; a miss builds and stores. `origin` is the block's
    /// document and first source byte.
    fn cached<F>(&mut self, cache: Option<&RenderCache>, key: Option<u64>, origin: Option<(DocumentId, usize)>, build: F) -> Option<BuiltBlock>
    where
        F: FnOnce(&mut Self) -> Option<BuiltBlock>,
    {
        let (Some(cache), Some(key), Some((document, base))) = (cache, key, origin) else {
            return build(self);
        };
        let path = self.paths.get(document.0).copied().unwrap_or("").to_string();
        if let Some(c) = cache.get(key) {
            if c.document == document && *c.path == *path {
                let _ = &c.block.cache_key;
                let rec_delta = self.recs.len() as isize - c.rec_base as isize;
                let math_delta = self.maths.len() as isize - c.math_base as isize;
                let mut block = c.block.clone();
                for r in &mut block.recs {
                    if let Some(i) = r {
                        *i = (*i as isize + rec_delta) as usize;
                    }
                }
                let mut recs = c.recs.clone();
                for r in &mut recs {
                    if let BoxRec::Math(mi) = r {
                        *mi = (*mi as isize + math_delta) as usize;
                    }
                }
                let mut maths = c.maths.clone();
                let mut diags = c.diagnostics.clone();
                let delta = base as isize - c.base as isize;
                incremental::relocate_block(&mut block, &mut recs, &mut maths, &mut diags, &path, delta);
                block.cache_key = Some((key, document, base));
                self.recs.extend(recs);
                self.maths.extend(maths);
                for (k, d) in diags {
                    self.emit(k, d);
                }
                return Some(block);
            }
        }
        let rec_base = self.recs.len();
        let math_base = self.maths.len();
        let outer = self.capture.replace(Vec::new());
        let mut built = build(self);
        let captured = self.capture.take().unwrap_or_default();
        self.capture = outer;
        if let Some(b) = &mut built {
            b.cache_key = Some((key, document, base));
        }
        if let Some(b) = &built {
            cache.insert(
                key,
                CachedBlock {
                    block: b.clone(),
                    rec_base,
                    math_base,
                    recs: self.recs[rec_base..].to_vec(),
                    maths: self.maths[math_base..].to_vec(),
                    diagnostics: captured,
                    document,
                    base,
                    path,
                },
            );
        }
        built
    }

    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    fn source(&self, span: Span) -> SourceRange {
        SourceRange {
            path: self.path_rc(span.document),
            start_byte: span.start,
            end_byte: span.end,
        }
    }

    /// The shared path string of a document (one allocation per document).
    fn path_rc(&self, document: DocumentId) -> Rc<str> {
        let mut cache = self.path_rcs.borrow_mut();
        if let Some(p) = cache.get(&document.0) {
            return p.clone();
        }
        let p: Rc<str> = Rc::from(self.paths.get(document.0).copied().unwrap_or(""));
        cache.insert(document.0, p.clone());
        p
    }

    fn report_once(&mut self, key: String, d: Diagnostic) {
        self.emit(Some(key), d);
    }

    fn face(&mut self, style: TextStyle, size: f64, span: Span) -> Rc<LoadedFace> {
        let (role, notes) = self.text_role(style, size);
        let r = self.fonts.resolve(self.style.family, role, size);
        for (key, message) in notes {
            let src = self.source(span);
            self.report_once(key, Diagnostic::warning("font_shape_substituted", message, vec![src]));
        }
        if let Some(note) = &r.note {
            let src = self.source(span);
            self.report_once(
                format!("outline:{note}"),
                Diagnostic::warning("font_outline_substituted", note.clone(), vec![src]),
            );
        }
        if let Some(reason) = r.substituted {
            let src = self.source(span);
            self.report_once(
                format!("subst:{reason}"),
                Diagnostic::error(
                    "font_unavailable",
                    format!("Latin Modern face unavailable ({reason}); Times metrics substituted, output is not the requested document"),
                    vec![src],
                ),
            );
        }
        if let Some(note) = &r.face.metrics_fallback {
            let src = self.source(span);
            self.report_once(
                format!("ecmetrics:{}", r.face.name),
                Diagnostic::warning("ec_metrics_unavailable", format!("{}: {note}", r.face.name), vec![src]),
            );
        }
        match &r.face.tfm_status {
            crate::fonts::TfmStatus::Loaded => {}
            crate::fonts::TfmStatus::RequiredUnavailable(reason) => {
                let src = self.source(span);
                self.report_once(
                    format!("tfm:{}", r.face.name),
                    Diagnostic::error(
                        "required_metrics_unavailable",
                        format!("{}: {reason}; the pinned Latin Modern 2.004 metrics are required for this size, OpenType advances were used and the layout is not the reference geometry", r.face.name),
                        vec![src],
                    ),
                );
            }
            crate::fonts::TfmStatus::Missing(_) => {
                if let Some(reason) = &r.face.tfm_missing {
                    let src = self.source(span);
                    self.report_once(
                        format!("tfm:{}", r.face.name),
                        Diagnostic::warning(
                            "tfm_missing",
                            format!("{}: {reason}; OpenType advances are used instead of TeX's metrics", r.face.name),
                            vec![src],
                        ),
                    );
                }
            }
        }
        r.face
    }

    /// The font role of a text style: its NFSS shape selected in the
    /// document's scheme (`\wrong@fontshape` substitutions) and followed
    /// through `sub*`/`ssub*` to the font LaTeX loads, with LaTeX's font
    /// warnings keyed for [`Self::report_once`].
    fn text_role(&self, style: TextStyle, size: f64) -> (Role, Vec<(String, String)>) {
        let scheme = self.style.nfss;
        let selected = crate::nfss::select(scheme, style.key());
        let (terminal, sub) = crate::nfss::terminal(scheme, selected.key);
        let mut notes = Vec::new();
        for undefined in [style.undefined, selected.undefined].into_iter().flatten() {
            let (from, to) = (scheme.describe(undefined), scheme.describe(selected.key));
            notes.push((format!("nfss:{from}"), format!("Font shape `{from}' undefined, using `{to}' instead")));
        }
        if let Some((from, to)) = sub {
            let (from, to) = (scheme.describe(from), scheme.describe(to));
            notes.push((
                format!("nfss:{from}:{size}"),
                format!("Font shape `{from}' in size <{size}> not available, Font shape `{to}' tried instead"),
            ));
        }
        (Role::Font(terminal), notes)
    }

    fn math_fonts(&mut self, span: Span) -> Option<MathProvider> {
        if let Some(m) = &self.math_fonts {
            return Some(m.clone());
        }
        if self.math_unavailable {
            return None;
        }
        let r = self.fonts.resolve(self.style.family, Role::Math, self.style.body_size_pt);
        let sizes = MathSizes {
            text: self.style.body_size_pt,
            script: self.style.script_size_pt,
            script_script: self.style.scriptscript_size_pt,
        };
        match (r.substituted, MathFonts::new(r.face, sizes)) {
            (None, Some(m)) => {
                // The double-struck secondary face (msbm's design) is
                // optional: absent, Latin Modern Math draws `\mathbb` and
                // the profile note below says so once.
                let m = Rc::new(m.with_double_struck(self.fonts.otf(crate::mathfont::BB_FONT_FILE)));
                // pdfLaTeX's geometry needs the lm math TFMs' parameters
                // (metric-identical to CM, embedded in math-layout) and
                // `rm-lmr` for the roman family; without the TFM directory
                // the OpenType MATH table is used and reported.
                let base = match self.style.base {
                    flashtex_document_style::BaseSize::Pt10 => 10,
                    flashtex_document_style::BaseSize::Pt11 => 11,
                    flashtex_document_style::BaseSize::Pt12 => 12,
                };
                // The math alphabets the sources name get their text fonts.
                let used: Vec<crate::mathalpha::MathAlphabet> = crate::mathalpha::TEXT_ALPHABETS
                    .iter()
                    .copied()
                    .filter(|a| self.texts.iter().any(|t| t.contains(a.command())))
                    .collect();
                let tex = TexMathMetrics::new(base, self.style.cmex_designs, m.clone(), self.fonts)
                    .with_alphabets(self.fonts, &used);
                let provider = if tex.roman_available() {
                    MathProvider::Tex(Rc::new(tex))
                } else {
                    let src = self.source(span);
                    let diag = match tex.roman_status() {
                        Some(crate::fonts::TfmStatus::RequiredUnavailable(e)) => Diagnostic::error(
                            "required_metrics_unavailable",
                            format!("math roman metrics: {e}; math is laid out with the OpenType MATH table and is not the reference geometry"),
                            vec![src],
                        ),
                        other => Diagnostic::warning(
                            "math_metrics_opentype",
                            format!(
                                "rm-lmr*.tfm unavailable ({}); math is laid out with the OpenType MATH table instead of TeX's metrics",
                                match other {
                                    Some(crate::fonts::TfmStatus::Missing(m)) => m.clone(),
                                    _ => "not found".into(),
                                }
                            ),
                            vec![src],
                        ),
                    };
                    self.report_once("math:no-tfm".into(), diag);
                    MathProvider::Otf(m)
                };
                self.math_fonts = Some(provider.clone());
                Some(provider)
            }
            (subst, _) => {
                let src = self.source(span);
                let reason = subst.unwrap_or_else(|| "face has no MATH table".into());
                self.emit(None, Diagnostic::error(
                    "math_font_unavailable",
                    format!("Latin Modern Math unavailable ({reason}); math is not typeset"),
                    vec![src],
                ));
                self.math_unavailable = true;
                None
            }
        }
    }

    /// The math provider for text at `size`: the body's, except at the
    /// class's `\footnotesize` below it, where LaTeX selects the math fonts
    /// of that size (`\DeclareMathSizes`: 8/6/5 pt in a 10pt class, 10/7/5 in
    /// a 12pt class). A size whose TeX metrics are not available (9 pt, the
    /// 11pt class's notes: cmmi9/cmsy9 are not embedded) keeps the body's
    /// metrics and is reported once.
    fn math_fonts_at(&mut self, span: Span, size: f64) -> Option<MathProvider> {
        let body = self.math_fonts(span)?;
        let note_size = footnotes::FootnoteParams::of(self.style).size;
        if (size - self.style.body_size_pt).abs() < 0.01 || (size - note_size).abs() > 0.01 || !matches!(body, MathProvider::Tex(_)) {
            return Some(body);
        }
        let key = (size * 100.0).round() as u32;
        if let Some(sized) = self.math_fonts_sized.get(&key) {
            return Some(sized.clone().unwrap_or(body));
        }
        let (script, script_script) = match (size * 100.0).round() as u32 {
            800 | 900 => (6.0, 5.0),
            1000 => (7.0, 5.0),
            _ => (8.0, 6.0),
        };
        let r = self.fonts.resolve(self.style.family, Role::Math, size);
        let sized = MathFonts::new(r.face, MathSizes { text: size, script, script_script })
            .map(|m| Rc::new(m.with_double_struck(self.fonts.otf(crate::mathfont::BB_FONT_FILE))))
            .and_then(|m| TexMathMetrics::at_text_size(size, self.style.cmex_designs, m, self.fonts))
            .filter(TexMathMetrics::roman_available)
            .map(|t| MathProvider::Tex(Rc::new(t)));
        if sized.is_none() {
            let src = self.source(span);
            let msg = format!("math at {size}pt (\\footnotesize) is laid out with the {}pt math metrics: TeX's math fonts for that size are not available", self.style.body_size_pt);
            self.report_once(format!("mathlim:{msg}"), Diagnostic::warning("math_limitation", msg, vec![src]));
        }
        self.math_fonts_sized.insert(key, sized.clone());
        Some(sized.unwrap_or(body))
    }

    /// `\fontdimen`s of the face for `style` at `size`: the face's TFM
    /// when it has one (exact fixwords), else the transcribed table.
    fn text_params(&self, style: TextStyle, size: f64) -> params::TextParamsPt {
        let r = self.fonts.resolve(self.style.family, self.text_role(style, size).0, size);
        if let (None, Some(tfm)) = (&r.substituted, &r.face.tfm) {
            let dim = |n: usize| tfm.param(n).map_or(0.0, |v| crate::tfm::Tfm::pt(v, size));
            return params::TextParamsPt {
                space: dim(2),
                stretch: dim(3),
                shrink: dim(4),
                x_height: dim(5),
                quad: dim(6),
                extra_space: dim(7),
            };
        }
        let design = design_size(self.style.family, size);
        params::text_params(self.style.family, style.bold, style.italic, design).at(size)
    }

    /// Interword glue for the face/style at `size` with TeX's space factor.
    fn space_glue(&self, style: TextStyle, size: f64, factor: u32) -> pl::Glue {
        let p = self.text_params(style, size);
        let f = f64::from(factor.max(1));
        let mut width = p.space;
        if factor >= 2000 {
            width += p.extra_space;
        }
        pl::Glue::finite(width, p.stretch * f / 1000.0, p.shrink * 1000.0 / f)
    }

    /// `text_builtins::DimenContext` for `style` at `size`: the face's
    /// `\fontdimen6`/`\fontdimen5`, and the text width for `\textwidth`,
    /// `\linewidth` and `\columnwidth` (a list's narrower `\linewidth` is
    /// not tracked at this level).
    fn dimen_context(&self, style: TextStyle, size: f64) -> flashtex_compiler::text_builtins::DimenContext {
        use flashtex_compiler::text_builtins::{pt_to_sp, DimenContext};
        let p = self.text_params(style, size);
        let width = pt_to_sp(self.style.text_width_pt);
        DimenContext {
            quad: pt_to_sp(p.quad),
            x_height: pt_to_sp(p.x_height),
            text_width: width,
            line_width: width,
            column_width: width,
        }
    }

    /// `\rule` (compiler `Inline::Rule`, latex.ltx 16359-16367): an hbox
    /// `RuleBox::width` wide whose painted part spans `rule_bottom..rule_top`
    /// above the baseline; zero-width or empty rules are struts.
    fn rule_box(&mut self, rule: &flashtex_compiler::text_builtins::TextRule, cx: &flashtex_compiler::text_builtins::DimenContext, size: f64, span: Span) -> (pl::GlyphRun, usize) {
        use flashtex_compiler::text_builtins::sp_to_pt;
        let b = rule.resolve(cx);
        let width = sp_to_pt(b.width);
        let (paint_width, paint_height) = if b.painted() { (width, sp_to_pt(b.rule_top - b.rule_bottom)) } else { (0.0, 0.0) };
        self.recs.push(BoxRec::Rule {
            width: paint_width,
            height: paint_height,
            bottom: sp_to_pt(b.rule_bottom),
            span,
        });
        let run = pl::GlyphRun {
            font: MATH_SENTINEL,
            size,
            glyphs: Vec::new(),
            width,
            height: sp_to_pt(b.height),
            depth: sp_to_pt(b.depth),
            source: span.start..span.end,
        };
        (run, self.recs.len() - 1)
    }

    /// `\TeX`/`\LaTeX`/`\LaTeXe` (compiler `Inline::Logo`): one box per glyph
    /// at the x `text_builtins::layout_logo` computes from this face's TFM
    /// metrics, joined by kerns (not break points: no glue follows them),
    /// with `E` lowered and `A` raised through the box records. `\LaTeXe`'s
    /// `ε` is a math box from the formula fonts, shifted like the subscript.
    fn logo_items(&mut self, logo: TextLogo, style: TextStyle, size: f64, span: Span) -> Vec<(pl::Item, Option<usize>)> {
        use flashtex_compiler::text_builtins::{self as tb, LogoFont};
        let sf = tb::sp_to_pt(tb::sf_size(tb::pt_to_sp(size)));
        let sf_style = TextStyle { size_cpt: (sf * 100.0).round() as u16, ..style };
        let current = self.face(style, size, span);
        let small = self.face(sf_style, sf, span);
        let params = self.text_params(style, size);
        let mut epsilon: Option<(usize, f64, f64)> = None;
        if logo == TextLogo::LaTeXe {
            if let Some(rec) = self.math_box(&flashtex_compiler::math::varepsilon_list(span), span, false, size) {
                if let BoxRec::Math(mi) = &self.recs[rec] {
                    let root = &self.maths[*mi].root;
                    epsilon = Some((rec, root.width, root.height));
                }
            }
        }
        let metrics = TfmLogoMetrics {
            current: current.tfm.clone(),
            small: small.tfm.clone(),
            size,
            sf,
            quad: params.quad,
            x_height: params.x_height,
            epsilon: epsilon.map_or((0.0, 0.0), |(_, w, h)| (w, h)),
        };
        let built = tb::layout_logo(logo, &metrics);
        let mut out = Vec::new();
        let mut x = 0.0f64;
        for g in &built.glyphs {
            let target = tb::sp_to_pt(g.x);
            if (target - x).abs() > 1e-9 {
                out.push((pl::Item::kern(target - x), None));
                x = target;
            }
            let raise = tb::sp_to_pt(g.raise);
            let placed = match g.font {
                LogoFont::MathItalic => epsilon.and_then(|(rec, ..)| {
                    let BoxRec::Math(mi) = &self.recs[rec] else { return None };
                    let mi = *mi;
                    self.maths[mi].raise = raise;
                    Some((math_run(&self.maths[mi].root, size, span), rec))
                }),
                font => {
                    let (seg_style, seg_size) = if font == LogoFont::ScriptSize { (sf_style, sf) } else { (style, size) };
                    let seg = adapter::Segment {
                        text: g.ch.to_string(),
                        chars: vec![adapter::CharSrc { document: span.document, start: span.start, end: span.end }],
                        style: seg_style,
                    };
                    let placed = self.text_box(&seg, seg_size);
                    if let Some((_, rec)) = placed {
                        if let BoxRec::Text { raise: r, .. } = &mut self.recs[rec] {
                            *r = raise;
                        }
                    }
                    placed
                }
            };
            if let Some((mut run, rec)) = placed {
                run.height += raise;
                run.depth -= raise;
                x += run.width;
                out.push((pl::Item::Box(run), Some(rec)));
            }
        }
        let total = tb::sp_to_pt(built.width);
        if (total - x).abs() > 1e-9 {
            out.push((pl::Item::kern(total - x), None));
        }
        out
    }

    /// Shapes one styled segment into a box record and a paragraph-layout box.
    fn text_box(&mut self, seg: &adapter::Segment, size: f64) -> Option<(pl::GlyphRun, usize)> {
        let filtered = self.input_filtered(seg);
        let seg = filtered.as_ref().unwrap_or(seg);
        let span = seg_span(seg)?;
        let face = self.face(seg.style, size, span);
        // Where the input encoding puts a character outside the font's
        // ligature/kern program (an OT1 `\accent`, a TS1 symbol), the text
        // is shaped in pieces split around it.
        let cuts: Vec<usize> = match &self.style.input {
            Some(input) if !seg.text.is_ascii() => seg
                .text
                .char_indices()
                .filter(|(_, c)| input.cuts_ligkern(*c))
                .flat_map(|(i, c)| [i, i + c.len_utf8()])
                .collect(),
            _ => Vec::new(),
        };
        // Verbatim runs the font's ligature/kern program not at all
        // (`\@noligs`); every other run runs it as TeX does.
        let shaped = if seg.style.literal {
            self.shaper.shape_literal(&face, &seg.text)
        } else if !cuts.is_empty() {
            self.shaper.shape_cut(&face, &seg.text, &cuts)
        } else {
            self.shaper.shape(&face, &seg.text)
        };
        if let Some(e) = &shaped.tfm_error {
            let src = self.source(span);
            self.report_once(
                format!("tfmrun:{}:{e}", face.name),
                Diagnostic::warning(
                    "tfm_run_error",
                    format!("{}: TFM ligature/kern program failed for {:?} ({e}); OpenType metrics used for this word", face.name, seg.text),
                    vec![src],
                ),
            );
        }
        if let Some(reason) = &shaped.refused {
            let src = self.source(span);
            self.emit(None, Diagnostic::error("unsupported_script", format!("cannot shape {:?}: {reason}", seg.text), vec![src]));
            return None;
        }
        for (ch, off) in &shaped.missing {
            let ch_src = seg
                .chars
                .get(seg.text[..*off].chars().count())
                .map(|c| c.span())
                .unwrap_or(span);
            let src = self.source(ch_src);
            self.report_once(
                format!("missing:{}:{}", face.font_id, ch),
                Diagnostic::warning(
                    "missing_glyph",
                    format!("U+{:04X} '{}' has no glyph in {}; nothing drawn for it", *ch as u32, ch, face.name),
                    vec![src],
                ),
            );
        }
        let mut glyphs = Vec::new();
        let mut recs = Vec::new();
        let mut clusters = Vec::new();
        let byte_to_char: Vec<usize> = {
            let mut v = vec![0usize; seg.text.len() + 1];
            for (ci, (bi, _)) in seg.text.char_indices().enumerate() {
                v[bi] = ci;
            }
            v[seg.text.len()] = seg.text.chars().count();
            v
        };
        for c in &shaped.clusters {
            let first = seg.chars.get(byte_to_char[c.text_range.start]).copied()?;
            let last_char_index = byte_to_char[c.text_range.end].saturating_sub(1);
            let last = seg.chars.get(last_char_index).copied().unwrap_or(first);
            let cspan = Span::in_document(first.document, first.start.min(last.start), first.end.max(last.end));
            let g0 = glyphs.len();
            for g in &c.glyphs {
                glyphs.push(pl::ShapedGlyph {
                    gid: u32::from(g.gid.0),
                    advance_units: i64::from(g.advance),
                    cluster: cspan.start..cspan.end,
                });
                recs.push(GlyphRec {
                    gid: g.gid.0,
                    italic_fix: g.italic,
                    x_offset_units: g.x_offset,
                    y_offset_units: g.y_offset,
                    y_max_units: g.y_max,
                    y_min_units: g.y_min,
                    empty: g.empty,
                    tfm_code: if shaped.tfm_metrics { g.tfm_code } else { None },
                    advance_fix: g.advance,
                    kern_fix: g.tfm_kern,
                });
            }
            clusters.push(ClusterRec {
                text_range: c.text_range.clone(),
                span: cspan,
                glyphs: g0..glyphs.len(),
            });
        }
        if glyphs.is_empty() {
            return None;
        }
        let run = pl::GlyphRun::from_shaped(
            face.layout_id(),
            size,
            shaped.units_per_em as f64,
            f64::from(shaped.height_units),
            -f64::from(shaped.depth_units),
            &glyphs,
            span.start..span.end,
        );
        let height = run.height;
        let depth = run.depth;
        self.recs.push(BoxRec::Text {
            face,
            size,
            text: seg.text.clone(),
            style: seg.style,
            clusters,
            glyphs: recs,
            height,
            depth,
            continues: false,
            raise: 0.0,
        });
        Some((run, self.recs.len() - 1))
    }

    /// pdfLaTeX's input errors for the literal UTF-8 characters of `seg`
    /// (`crate::inputenc`): each rejected character is reported at its
    /// source as the LaTeX error pdfLaTeX logs, and removed from the text —
    /// or replaced by the letter pdfLaTeX still sets (`\k a` in OT1 sets
    /// `a`). `None` when the segment keeps every character.
    fn input_filtered(&mut self, seg: &adapter::Segment) -> Option<adapter::Segment> {
        let input = self.style.input.as_ref()?;
        if seg.text.is_ascii() {
            return None;
        }
        // Only a character typed in the source is input: one a macro
        // generates (`\fnsymbol`'s U+2217 for `\thanks`) has its invocation's
        // span, not its own bytes.
        let texts = self.texts;
        let typed = |i: usize, c: char| {
            seg.chars.get(i).is_some_and(|s| {
                texts.get(s.document.0).and_then(|t| t.get(s.start..s.end)).is_some_and(|t| t.len() == c.len_utf8() && t.starts_with(c))
            })
        };
        let rejected: Vec<(usize, char, crate::inputenc::Rejected)> = seg
            .text
            .chars()
            .enumerate()
            .filter_map(|(i, c)| input.rejected(c).filter(|_| typed(i, c)).map(|r| (i, c, r)))
            .collect();
        if rejected.is_empty() {
            return None;
        }
        let mut text = String::with_capacity(seg.text.len());
        let mut chars = Vec::with_capacity(seg.chars.len());
        let mut next = rejected.iter().peekable();
        for (i, c) in seg.text.chars().enumerate() {
            let src = seg.chars.get(i).copied();
            let keep = match next.peek() {
                Some((at, _, r)) if *at == i => {
                    let keep = r.keep;
                    next.next();
                    keep
                }
                _ => Some(c),
            };
            if let (Some(k), Some(src)) = (keep, src) {
                text.push(k);
                chars.push(src);
            }
        }
        for (i, c, r) in rejected {
            let Some(csrc) = seg.chars.get(i).copied() else { continue };
            let src = self.source(csrc.span());
            let code = if r.message.contains("not set up for use with LaTeX") { "unicode_not_set_up" } else { "command_unavailable_in_encoding" };
            let mut d = Diagnostic::error(code, r.message, vec![src]);
            d.recovery = Some(match r.keep {
                Some(k) => format!("typeset `{k}` without the accent, as pdfLaTeX does after this error"),
                None => format!("typeset nothing for U+{:04X}, as pdfLaTeX does after this error", c as u32),
            });
            self.report_once(format!("input:{}:{}:{}", csrc.document.0, csrc.start, c), d);
        }
        Some(adapter::Segment { text, chars, style: seg.style })
    }

    /// Marks a text box as the continuation of the word box before it.
    fn mark_continues(&mut self, rec: usize) {
        if let BoxRec::Text { continues, .. } = &mut self.recs[rec] {
            *continues = true;
        }
    }

    fn math_box(&mut self, list: &flashtex_compiler::math::MathList, span: Span, display: bool, size: f64) -> Option<usize> {
        let fonts = self.math_fonts_at(span, size)?;
        let mut sink = crate::mathtext::TextSink::default();
        // `\quad` in math is `\hskip1em` of the text font (`\fontdimen6`),
        // not 18 mu of the math symbol font.
        let fam2_quad = ml::MathFontMetrics::params(fonts.metrics(), ml::Style::TEXT.size_class()).quad;
        let text_quad = self.text_params(TextStyle::default(), size).quad;
        if fam2_quad > 0.0 && text_quad > 0.0 {
            sink.text_quad = Some((text_quad, text_quad / fam2_quad));
        }
        sink.body_size_pt = self.style.body_size_pt;
        sink.amsfonts = self.ams_symbol_fonts;
        sink.amsmath = self.amsmath_loaded;
        let texts = self.texts;
        let fence = |sp: &Span| fence_of(texts.get(sp.document.0).copied().unwrap_or(""), sp.start);
        // The atom's own class when the pinned compiler exposes it, and the
        // source-text re-derivation otherwise. `\colon` is why the field is
        // needed: the kernel's is Punct and amsmath's is Ord, produced from
        // the same five characters of source, so no amount of reading the
        // control word back can tell them apart.
        let class = |a: &flashtex_compiler::math::MathAtom| {
            #[cfg(feature = "math-class-override")]
            if let Some(forced) = a.class_override {
                return Some(ml_class(forced));
            }
            let sp = &a.span;
            class_override_of(texts.get(sp.document.0).copied().unwrap_or(""), sp.start)
        };
        // `\lim`/`\sin`/`\max`: the `\mathop` class and the limit placement
        // the kernel declares each with, plus any `\limits`/`\nolimits`
        // switch after it -- none of which the compiler's `Nucleus::Text`
        // carries, so all of it is re-read from the source at the span.
        let op_limits = |sp: &Span| operator_limits_of(texts.get(sp.document.0).copied().unwrap_or(""), sp.start);
        // `\lim`/`\mathrm{...}`/`\bmod` against `\text{...}`/`\tag{...}` and
        // against a one-character siunitx unit run: the compiler spells all
        // of them `Nucleus::Text`, but only a *whole* run of math characters
        // keeps the italic correction of its last character (§752).
        let text_italic = |sp: &Span| math_text_keeps_italic(texts.get(sp.document.0).copied().unwrap_or(""), sp.start);
        // `\limsup`/`\liminf`: `lim`, a thin space, then `sup`/`inf`.
        let text_split = |sp: &Span| operator_thin_space_split(texts.get(sp.document.0).copied().unwrap_or(""), sp.start);
        // `\ldots`/`\cdots`: `\mathinner` of three Punct dots.
        let ellipsis = |sp: &Span| math_ellipsis_of(texts.get(sp.document.0).copied().unwrap_or(""), sp.start);
        // `\quad`/`\qquad`/`\,`/`\:`/`\;`/`\!` (compiler `Space { em }`) at
        // the top level of the formula (outside `\left...\right`): math-layout
        // has no kern atom, so the formula is split there into runs laid out
        // separately and joined by kerns of the requested width plus the
        // inter-atom spacing TeX still inserts across glue (glue does not
        // reset `r_type`, §760). Glue inside a fence pair or a sub-formula
        // cannot be split out and stays reported.
        // Likewise a top-level `array`/`cases`/matrix grid (compiler
        // `Matrix`) is laid out on this side (`mathgrid`) from its cells,
        // each a formula of its own; a grid nested in a sub-formula (a
        // fraction, a script, inside `\left...\right`, another grid's cell)
        // enters math-layout as a box handle (`mathtext::GridCells`).
        let segments = split_at_spaces(list, &fence, sink.font_em_ratio());
        let ml_lists: Vec<ml::MathList> = segments.iter().map(|(atoms, _)| convert_math_classed(&flashtex_compiler::math::MathList { atoms: atoms.clone() }, &mut sink, &fence, &class, &op_limits, &text_italic, &text_split, &ellipsis)).collect();
        // Glue at a top-level grid cell's own top level is split out like
        // the formula's; only deeper glue is dropped.
        let nested_glue_em: f64 = segments
            .iter()
            .flat_map(|(atoms, _)| atoms.iter())
            .map(|a| match &a.nucleus {
                flashtex_compiler::math::Nucleus::Matrix { rows, .. } if a.superscript.is_none() && a.subscript.is_none() => rows
                    .iter()
                    .flatten()
                    .flat_map(|cell| cell.atoms.iter())
                    .filter(|c| !matches!(c.nucleus, flashtex_compiler::math::Nucleus::Space { .. }))
                    .map(|c| math_glue_em(&flashtex_compiler::math::MathList { atoms: vec![c.clone()] }))
                    .sum(),
                _ => math_glue_em(&flashtex_compiler::math::MathList { atoms: vec![a.clone()] }),
            })
            .sum();
        // With math-layout's `Glue` atom (feature `amsmath-inline`) nested
        // glue is set, not dropped.
        if nested_glue_em.abs() > 0.0 && cfg!(not(feature = "amsmath-inline")) {
            let src = self.source(span);
            let msg = format!("\\quad/\\qquad glue ({nested_glue_em} em in this formula) inside \\left...\\right or a sub-formula dropped: math-layout has no kern atom and only top-level glue can be split into separate runs");
            self.report_once(format!("mathlim:{msg}"), Diagnostic::warning("math_limitation", msg, vec![src]));
        }
        let mut approximations = Vec::new();
        math_approximations(list, &mut approximations);
        for msg in approximations {
            let src = self.source(span);
            self.report_once(format!("mathlim:{msg}"), Diagnostic::warning("math_limitation", msg, vec![src]));
        }
        let default_style = if display { ml::Style::DISPLAY } else { ml::Style::TEXT };
        let style = leading_style_switch(list, texts).unwrap_or(default_style);
        let has_grid = segments.iter().any(|(atoms, _)| top_level_grids(atoms, &fence).into_iter().any(|top| top));
        // Every `\text` must be collected before the metrics borrow the sink.
        let grid_pieces = if has_grid { grid_pieces(&segments, &mut sink, &fence, &class, &op_limits, texts) } else { Vec::new() };
        let pitch = crate::mathgrid::Pitch {
            baselineskip: self.style.baselineskip_pt,
            lineskip: self.style.lineskip_pt,
            lineskiplimit: self.style.lineskiplimit_pt,
        };
        let class_size = crate::adapter::class_size_of(self.style.body_size_pt);
        let nested: Vec<crate::mathtext::NestedGrid> = sink
            .grids
            .iter()
            .map(|g| crate::mathtext::NestedGrid {
                spec: crate::mathgrid::GridSpec::from_source(texts.get(g.span.document.0).copied().unwrap_or(""), g.span, class_size),
                pitch,
                grid: g.clone(),
            })
            .collect();
        let frames = sink.frames.clone();
        let text_metrics = crate::mathtext::TextRunMetrics::new(fonts.metrics(), self.fonts, self.shaper, self.style.family, &sink.texts, &sink.keys, &sink.italics)
            .with_grids(&nested)
            .with_frames(&frames);
        let mut laid = if has_grid {
            self.grid_formula(&grid_pieces, style, &text_metrics, span)
        } else {
            let glue: Vec<Option<f64>> = segments.iter().map(|(_, em)| *em).collect();
            layout_kerned(&ml_lists, &glue, style, &text_metrics)
        };
        let (grid_boxes, grid_limitations) = text_metrics.take_grids();
        let (frame_boxes, frame_limitations) = text_metrics.take_frames();
        laid.limitations.extend(grid_limitations);
        laid.limitations.extend(frame_limitations);
        let (text_runs, notices) = text_metrics.finish();
        crate::mathtext::substitute_math_boxes(&mut laid.root, &grid_boxes, &frame_boxes);
        crate::mathtext::substitute(&mut laid.root, &text_runs);
        // Text-style formulas in a paragraph break after top-level Bin/Rel
        // atoms; a formula holding a grid stays one box.
        let kerned = !(ml_lists.len() == 1 && segments[0].1.is_none());
        let inline_breaks = if display || has_grid || !inline_math_breaks_enabled() { Vec::new() } else { inline_break_points(&mut laid.root, &ml_lists, kerned) };
        for text in &sink.refused {
            let src = self.source(span);
            self.emit(
                None,
                Diagnostic::error(
                    "math_text_overflow",
                    format!(
                        "more than {} \\text arguments in one formula; {:?} is not typeset",
                        crate::mathtext::MAX_TEXT_ATOMS,
                        crate::mathtext::abbreviate(text)
                    ),
                    vec![src],
                ),
            );
        }
        for n in notices {
            use crate::mathtext::Notice;
            match n {
                // The same availability/TFM diagnostics the paragraph path
                // reports for this face (once per face).
                Notice::FaceUsed { size } => {
                    let _ = self.face(TextStyle::default(), size, span);
                }
                Notice::Refused { word, reason } => {
                    let src = self.source(span);
                    self.emit(None, Diagnostic::error("unsupported_script", format!("cannot shape {word:?} in \\text: {reason}"), vec![src]));
                }
                Notice::MissingGlyph { ch, face } => {
                    let src = self.source(span);
                    self.report_once(
                        format!("missing:{face}:{ch}"),
                        Diagnostic::warning("missing_glyph", format!("U+{:04X} '{}' has no glyph in {}; nothing drawn for it", ch as u32, ch, face), vec![src]),
                    );
                }
                Notice::TooLarge { text_chars, glyphs } => {
                    let src = self.source(span);
                    self.emit(
                        None,
                        Diagnostic::error(
                            "math_text_overflow",
                            format!("a \\text argument of {text_chars} characters ({glyphs} entries) exceeds what one formula can address; it is not typeset"),
                            vec![src],
                        ),
                    );
                }
                Notice::TfmRunError { word, face, error } => {
                    let src = self.source(span);
                    self.report_once(
                        format!("tfmrun:{face}:{error}"),
                        Diagnostic::warning("tfm_run_error", format!("{face}: TFM ligature/kern program failed for {word:?} ({error}); OpenType metrics used for this word"), vec![src]),
                    );
                }
            }
        }
        for ch in fonts.otf().take_missing() {
            let src = self.source(span);
            self.report_once(
                format!("mathmissing:{ch}"),
                Diagnostic::warning("missing_glyph", format!("U+{:04X} '{}' has no glyph in {}", ch as u32, ch, fonts.otf().face().name), vec![src]),
            );
        }
        if fonts.otf().take_bb_fallback() {
            let src = self.source(span);
            let reason = fonts.otf().bb_status().unwrap_or("not loaded");
            self.report_once(
                "math:bb-fallback".into(),
                Diagnostic::warning(
                    "math_resource_profile",
                    format!(
                        "msbm10/cmsy10: double-struck (\\mathbb) and calligraphic (\\mathcal) glyphs drawn from {} (open-face and script designs); {} unavailable ({reason}), so the outlines and advances are not the reference's msbm/cmsy design",
                        fonts.otf().face().name,
                        crate::mathfont::BB_FONT_FILE
                    ),
                    vec![src],
                ),
            );
        }
        for l in laid.limitations {
            let src = self.source(span);
            let msg = match l {
                // A refused `\text` run's placeholder: already reported as
                // math_text_overflow above.
                ml::Limitation::MissingGlyph(c) if crate::mathtext::is_handle(c) => continue,
                ml::Limitation::MissingGlyph(c) => format!("no math glyph for '{c}'; empty box used"),
                ml::Limitation::DelimiterTooSmall { ch, wanted, used } => {
                    format!("delimiter '{ch}' wanted {wanted:.2}pt, largest variant {used:.2}pt used")
                }
                ml::Limitation::RadicalTooSmall { wanted, used } => format!("radical wanted {wanted:.2}pt, largest {used:.2}pt used"),
                ml::Limitation::MissingAccent(c) => format!("unknown accent '{c}'"),
            };
            self.report_once(format!("mathlim:{msg}"), Diagnostic::warning("math_limitation", msg, vec![src]));
        }
        self.maths.push(MathRec {
            root: laid.root,
            span,
            face: fonts.otf().face().clone(),
            metrics: fonts.clone(),
            text_runs,
            color: self.math_colors.get(&(span.document.0, span.start, span.end)).copied(),
            raise: 0.0,
            inline_breaks,
            continues: false,
            #[cfg(feature = "math-glyph-spans")]
            span_paints: Vec::new(),
        });
        let idx = self.maths.len() - 1;
        self.recs.push(BoxRec::Math(idx));
        Some(self.recs.len() - 1)
    }

    /// The paragraph items of an inline formula: one box, or, at each of
    /// its `inline_breaks`, the box up to the Bin/Rel atom, the penalty
    /// TeX puts after that atom, then the glue after it (the inter-atom
    /// spacing, plus any explicit kern) and the next box. A break taken at
    /// the penalty discards the glue, so the line ends after the operator;
    /// with no break, the pieces sit exactly where the one box's children
    /// did. The first piece keeps `rec`; the others are new records that
    /// `continues` the piece before them.
    fn math_pieces(&mut self, rec: usize, size: f64, span: Span) -> Vec<(pl::Item, Option<usize>)> {
        let BoxRec::Math(mi) = self.recs[rec] else { unreachable!() };
        let breaks = std::mem::take(&mut self.maths[mi].inline_breaks);
        if breaks.is_empty() {
            return vec![(pl::Item::Box(math_run(&self.maths[mi].root, size, span)), Some(rec))];
        }
        let ml::BoxKind::HBox(children) = &self.maths[mi].root.kind else {
            return vec![(pl::Item::Box(math_run(&self.maths[mi].root, size, span)), Some(rec))];
        };
        let units: Vec<ml::MathBox> = children.iter().map(|c| c.content.clone()).collect();
        let discardable = |b: &ml::MathBox| matches!(b.kind, ml::BoxKind::Glue { .. } | ml::BoxKind::Kern);
        // (piece, penalty and glue after it)
        let mut pieces: Vec<(ml::MathBox, Option<(i32, pl::Glue)>)> = Vec::new();
        let mut start = 0usize;
        for (last, penalty) in breaks {
            if last < start || last >= units.len() {
                continue;
            }
            let piece = ml::MathBox::hlist(units[start..=last].to_vec());
            let mut next = last + 1;
            let mut glue = pl::Glue::fixed(0.0);
            while next < units.len() && discardable(&units[next]) {
                let u = &units[next];
                glue.width += u.width;
                // plain.tex: `\thickmuskip=5mu plus 5mu`, `\medmuskip=4mu
                // plus 2mu minus 4mu`, `\thinmuskip=3mu`; a kern is fixed.
                if let ml::BoxKind::Glue { mu, .. } = u.kind {
                    if mu >= 5.0 {
                        glue.stretch += u.width;
                    } else if mu >= 4.0 {
                        glue.stretch += u.width / 2.0;
                        glue.shrink += u.width;
                    }
                }
                next += 1;
            }
            pieces.push((piece, Some((penalty, glue))));
            start = next;
        }
        if start < units.len() {
            pieces.push((ml::MathBox::hlist(units[start..].to_vec()), None));
        }
        let mut out = Vec::with_capacity(pieces.len() * 3);
        for (i, (piece, after)) in pieces.into_iter().enumerate() {
            let piece_rec = if i == 0 {
                self.maths[mi].root = piece;
                rec
            } else {
                let mut m = self.maths[mi].clone();
                m.root = piece;
                m.continues = true;
                self.maths.push(m);
                self.recs.push(BoxRec::Math(self.maths.len() - 1));
                self.recs.len() - 1
            };
            let BoxRec::Math(pm) = self.recs[piece_rec] else { unreachable!() };
            out.push((pl::Item::Box(math_run(&self.maths[pm].root, size, span)), Some(piece_rec)));
            if let Some((penalty, glue)) = after {
                out.push((pl::Item::penalty(penalty), None));
                out.push((pl::Item::Glue(glue), None));
            }
        }
        out
    }

    /// A formula holding a top-level grid: the top-level atoms are split
    /// into runs (laid out by math-layout), kerns (`\quad`...) and grids
    /// (`mathgrid`), joined in one hbox with the inter-atom spacing TeX
    /// would insert between the neighbouring classes (a fenced grid is an
    /// Inner atom, a bare `array` an Ord `\vcenter`).
    fn grid_formula(&mut self, pieces: &[GridPiece], style: ml::Style, text_metrics: &crate::mathtext::TextRunMetrics<'_>, span: Span) -> ml::Layout {
        // Classes of the whole formula for Rules 5/6 and the spacing at
        // each join: a grid stands as one placeholder atom.
        let mut all_atoms: Vec<ml::Atom> = Vec::new();
        for piece in pieces {
            match piece {
                GridPiece::Run(l) => all_atoms.extend(l.atoms.iter().cloned()),
                GridPiece::Grid { left, right, .. } => {
                    let fenced = !(left.is_empty() && right.is_empty());
                    all_atoms.push(ml::Atom::new(if fenced { ml::AtomClass::Inner } else { ml::AtomClass::Ord }, ml::Nucleus::Empty));
                }
                GridPiece::Kern(_) => {}
            }
        }
        let classes = ml::layout::effective_classes(&all_atoms);
        let params = ml::MathFontMetrics::params(text_metrics, style.size_class());
        let mu = params.mu();
        let pitch = crate::mathgrid::Pitch {
            baselineskip: self.style.baselineskip_pt,
            lineskip: self.style.lineskip_pt,
            lineskiplimit: self.style.lineskiplimit_pt,
        };
        let src_text = self.texts.get(span.document.0).copied().unwrap_or("");
        let size = crate::adapter::class_size_of(self.style.body_size_pt);
        let mut boxes: Vec<(f64, ml::MathBox)> = Vec::new();
        let mut limitations = Vec::new();
        let mut at = 0usize;
        // Class index of the last atom placed, for the spacing at a join.
        let mut prev_class: Option<ml::AtomClass> = None;
        let mut pending_kern = 0.0;
        for piece in pieces {
            let (b, n) = match piece {
                GridPiece::Kern(em) => {
                    pending_kern += em * params.quad;
                    continue;
                }
                GridPiece::Run(l) => {
                    let part = ml::layout_with_report(l, style, text_metrics);
                    limitations.extend(part.limitations);
                    (part.root, l.atoms.len())
                }
                GridPiece::Grid {
                    rows,
                    columns,
                    left,
                    right,
                    span: grid_span,
                } => {
                    let spec = crate::mathgrid::GridSpec::from_source(src_text, *grid_span, size);
                    let cells: Vec<Vec<ml::MathBox>> = rows
                        .iter()
                        .map(|row| {
                            row.iter()
                                .map(|(runs, glue)| {
                                    let part = layout_kerned(runs, glue, spec.style, text_metrics);
                                    limitations.extend(part.limitations);
                                    part.root
                                })
                                .collect()
                        })
                        .collect();
                    let grid = crate::mathgrid::layout_grid(cells, columns, &spec, pitch, &params, params.quad);
                    let fence_char = |s: &str| {
                        let mut it = s.chars();
                        match (it.next(), it.next()) {
                            (Some(c), None) => Some(c),
                            _ => None,
                        }
                    };
                    let b = if left.is_empty() && right.is_empty() {
                        grid
                    } else {
                        let (h, d) = (grid.height, grid.depth);
                        let mut fenced = Vec::new();
                        for ch in [fence_char(left), fence_char(right)] {
                            let (b, short) = crate::mathgrid::delimiter(text_metrics, ch, h, d, style, &params);
                            if let (Some(ch), Some((wanted, used))) = (ch, short) {
                                limitations.push(ml::Limitation::DelimiterTooSmall { ch, wanted, used });
                            }
                            fenced.push(b);
                        }
                        let close = fenced.pop().expect("two fences");
                        let open = fenced.pop().expect("two fences");
                        ml::MathBox::hlist(vec![open, grid, close])
                    };
                    // Fences and rules of a top-level grid map to it.
                    #[cfg(feature = "math-glyph-spans")]
                    let b = {
                        let mut b = b;
                        b.inherit_tag(math_tag(*grid_span));
                        b
                    };
                    (b, 1)
                }
            };
            // Spacing at the join: TeX's inter-atom space between the
            // classes on either side (glue does not reset r_type).
            if let (Some(left), Some(&right)) = (prev_class, classes.get(at)) {
                let spacing = ml::between(left, right, style).mu() * mu;
                if spacing + pending_kern != 0.0 {
                    boxes.push((0.0, ml::MathBox::kern(spacing + pending_kern)));
                }
            } else if pending_kern != 0.0 {
                boxes.push((0.0, ml::MathBox::kern(pending_kern)));
            }
            pending_kern = 0.0;
            at += n;
            prev_class = at.checked_sub(1).and_then(|j| classes.get(j)).copied();
            boxes.push((0.0, b));
        }
        if pending_kern != 0.0 {
            boxes.push((0.0, ml::MathBox::kern(pending_kern)));
        }
        ml::Layout {
            root: ml::MathBox::hbox(boxes),
            limitations,
        }
    }

    /// The items of one word segment: its box, or — when TeX would
    /// hyphenate it — the fragments between its legal breaks with a flagged
    /// discretionary at each: `\hyphenpenalty` with the face's hyphen
    /// (`\hyphenchar`, `-`) as pre-break text at a Liang point, or TeX's
    /// empty discretionary at `\exhyphenpenalty` after an explicit hyphen
    /// (§1039; a run of hyphens gets one after the run). A word with an
    /// explicit hyphen gets no automatic points (§896, `hyphen.tex`
    /// semantics in the crate's `LiangHyphenator`); capitalised words are
    /// hyphenated (`\uchyph=1`). Fragments are shaped separately: the font
    /// kern the whole-word shaping had across a point is kept as a kern
    /// after the penalty, so an unbroken word keeps exactly its width and
    /// the kern is discarded when the line breaks there, as TeX's
    /// reconstitution does (§903–918); the hyphen's advance is measured
    /// after the fragment it ends so a letter–hyphen kern is included. A
    /// point inside a ligature (`of-fice`) needs the pre/post/no-break
    /// reconstitution and is skipped.
    fn word_items(&mut self, seg: &adapter::Segment, size: f64, hyphenate: bool) -> Vec<(pl::Item, Option<usize>)> {
        if !hyphenate {
            return self.whole_word(seg, size);
        }
        let text = &seg.text;
        let mut points: Vec<(usize, bool)> = Vec::new();
        // The adapter has already turned `--`/`---` into U+2013/U+2014: those
        // T1 ligatures end in the hyphen char, so TeX appends the empty
        // discretionary after them too (a break after an em dash).
        let is_dash = |c: char| matches!(c, '-' | '\u{2013}' | '\u{2014}');
        if text.chars().any(is_dash) {
            let mut prev_dash = false;
            for (i, c) in text.char_indices() {
                if prev_dash && !is_dash(c) {
                    points.push((i, false));
                }
                prev_dash = is_dash(c);
            }
        } else {
            let hyphenator = self.hyphenator.as_ref().unwrap_or_else(|| english_hyphenator());
            points.extend(hyphenator.hyphenate(text).into_iter().map(|p| (p.offset, true)));
        }
        if points.is_empty() {
            return self.whole_word(seg, size);
        }
        let Some(span) = seg_span(seg) else { return Vec::new() };
        let face = self.face(seg.style, size, span);
        let shaper = self.shaper;
        let shaped = shaper.shape(&face, text);
        if shaped.refused.is_some() {
            return self.whole_word(seg, size);
        }
        let boundaries: BTreeSet<usize> = shaped.clusters.iter().map(|c| c.text_range.start).collect();
        points.retain(|(o, _)| *o > 0 && *o < text.len() && boundaries.contains(o));
        if points.is_empty() {
            return self.whole_word(seg, size);
        }
        // Each measurement carries the units of the shaping that produced
        // it: a substring whose characters all have T1 slots is measured by
        // the TFM ligature/kern program (2^20 per em), one that carries a
        // character with no T1 slot falls back to the font program (the
        // face's own units per em). `shape` decides that per string, so two
        // substrings of the same word can come back in different units;
        // measuring in points is the only scale they share. Dividing a TFM
        // width by the face's 1000 units instead put the tail of a word like
        // `ellipsis\dots` (U+2026 has no T1 slot, `ellip` does) 12676 pt to
        // the left of the page, and every later word on its line with it.
        let width_pt = |t: &str| shaper.shape(&face, t).width_pt(size);
        // Whole minus parts, with the exact integer subtraction kept for the
        // usual case where all three came back in the same units.
        let residual_pt = |whole: &str, head: &str, tail: &str| {
            let (w, h, t) = (shaper.shape(&face, whole), shaper.shape(&face, head), shaper.shape(&face, tail));
            if w.units_per_em == h.units_per_em && h.units_per_em == t.units_per_em {
                size * (w.width_units - h.width_units - t.width_units) as f64 / w.units_per_em as f64
            } else {
                w.width_pt(size) - h.width_pt(size) - t.width_pt(size)
            }
        };
        // Byte offset -> char index, for the fragments' `chars`.
        let mut char_index = vec![0usize; text.len() + 1];
        for (ci, (bi, _)) in text.char_indices().enumerate() {
            char_index[bi] = ci;
        }
        char_index[text.len()] = seg.chars.len();
        let mut cuts: Vec<usize> = Vec::with_capacity(points.len() + 2);
        cuts.push(0);
        cuts.extend(points.iter().map(|p| p.0));
        cuts.push(text.len());
        let mut out = Vec::new();
        for k in 1..cuts.len() {
            let (a, b) = (cuts[k - 1], cuts[k]);
            if k > 1 {
                let (at, automatic) = points[k - 2];
                let prev = cuts[k - 2];
                let pre_break = if automatic {
                    // The hyphen glyph of the run's face, provenance the
                    // letter it follows, cluster the empty range at the
                    // break (like the crate's own automatic points).
                    let hyphen = adapter::Segment {
                        text: "-".to_string(),
                        chars: vec![seg.chars[char_index[at] - 1]],
                        style: seg.style,
                    };
                    self.text_box(&hyphen, size).map(|(mut run, rec)| {
                        self.mark_continues(rec);
                        let adv = width_pt(&format!("{}-", &text[prev..at])) - width_pt(&text[prev..at]);
                        let doc_at = seg.chars[char_index[at]].start;
                        for g in &mut run.glyphs {
                            g.cluster = doc_at..doc_at;
                        }
                        if let Some(last) = run.glyphs.last_mut() {
                            last.advance += adv - run.width;
                        }
                        run.width = adv;
                        run.source = doc_at..doc_at;
                        (run, rec)
                    })
                } else {
                    None
                };
                let (pre_break, rec) = match pre_break {
                    Some((run, rec)) => (Some(run), Some(rec)),
                    None => (None, None),
                };
                out.push((
                    pl::Item::Penalty(pl::Penalty {
                        value: if automatic { HYPHEN_PENALTY } else { EX_HYPHEN_PENALTY },
                        flagged: true,
                        pre_break,
                        automatic,
                        post_break: None,
                        replace_count: 0,
                    }),
                    rec,
                ));
                let kern = residual_pt(&text[prev..b], &text[prev..at], &text[at..b]);
                if kern != 0.0 {
                    out.push((pl::Item::kern(kern), None));
                }
            }
            let frag = adapter::Segment {
                text: text[a..b].to_string(),
                chars: seg.chars[char_index[a]..char_index[b]].to_vec(),
                style: seg.style,
            };
            if let Some((run, rec)) = self.text_box(&frag, size) {
                if k > 1 {
                    self.mark_continues(rec);
                }
                out.push((pl::Item::Box(run), Some(rec)));
            }
        }
        out
    }

    fn whole_word(&mut self, seg: &adapter::Segment, size: f64) -> Vec<(pl::Item, Option<usize>)> {
        self.text_box(seg, size).map(|(run, rec)| vec![(pl::Item::Box(run), Some(rec))]).unwrap_or_default()
    }

    /// Runs the breaker. A list it rejects (non-finite or overlong, which
    /// the adapter never produces) is reported as a typed error and the
    /// block skipped rather than panicking the worker.
    fn break_paragraph(&mut self, list: &[pl::Item], params: &pl::LineBreakParams, items: &[AItem], recs: Option<&[Option<usize>]>) -> Option<pl::Lines> {
        // `recs` is `None` for material pdfTeX never line-breaks (a natural
        // width `\hbox`): no protrusion or expansion there.
        let result = match (self.style.microtype.clone().filter(|m| m.active()), recs) {
            (None, _) | (_, None) => pl::layout_paragraph(list, params),
            (Some(setup), Some(recs)) => {
                let mt = self.microtype_items(list, recs, &setup);
                let mut params = params.clone();
                // pdfTeX measures `\hsize` in sp: use the class frame's exact
                // width when the f64 measure is its 0.001pt snap.
                if let Some(doc) = &self.style.class_geometry {
                    let exact = doc.frame.columns[0].width.0 as f64 / 65536.0;
                    if (exact - params.line_width).abs() < 4.0 / 65536.0 {
                        params.line_width = exact;
                    }
                }
                pl::layout_paragraph_microtype(list, &params, &mt).map(|(lines, _)| lines)
            }
        };
        match result {
            Ok(lines) => Some(lines),
            Err(e) => {
                let span = items.iter().find_map(|i| match i {
                    AItem::Word(w) => w.segments.iter().find_map(seg_span),
                    AItem::Math { span, .. } => Some(*span),
                    _ => None,
                });
                let sources = span.map(|s| vec![self.source(s)]).unwrap_or_default();
                self.emit(None, Diagnostic::error("paragraph_layout_error", format!("paragraph not set: {e}"), sources));
                None
            }
        }
    }

    /// The microtype side of a horizontal list: each text box's (and
    /// discretionary hyphen's) TFM codes, sp widths and in-run kerns with its
    /// font's resolved pdfTeX parameters, and which kerns are font kerns (the
    /// kern `word_items` keeps after a hyphenation point).
    fn microtype_items(&mut self, list: &[pl::Item], recs: &[Option<usize>], setup: &crate::style::MicrotypeSetup) -> pl::Microtype {
        let mut out = Vec::with_capacity(list.len());
        for (i, item) in list.iter().enumerate() {
            let rec = recs.get(i).copied().flatten();
            let mut mi = pl::MicroItem::default();
            match item {
                pl::Item::Box(run) => mi.run = rec.filter(|r| !self.label_recs.contains(r)).and_then(|r| self.micro_run(r, run)),
                pl::Item::Penalty(p) => {
                    if let Some(pre) = &p.pre_break {
                        mi.pre_break = rec.and_then(|r| self.micro_run(r, pre));
                    }
                }
                pl::Item::Kern(_) => {
                    mi.font_kern = i > 0 && matches!(&list[i - 1], pl::Item::Penalty(p) if p.flagged) && matches!(list.get(i + 1), Some(pl::Item::Box(_)));
                }
                pl::Item::Glue(_) => {}
            }
            out.push(mi);
        }
        pl::Microtype { protrude_chars: setup.protrude_chars, adjust_spacing: setup.adjust_spacing, items: out }
    }

    /// A text box record as pdfTeX characters, or `None` for a box that is
    /// not TFM-shaped text in a font microtype configures.
    fn micro_run(&mut self, rec: usize, run: &pl::GlyphRun) -> Option<pl::MicroRun> {
        let BoxRec::Text { face, size, style, glyphs, .. } = &self.recs[rec] else { return None };
        if glyphs.len() != run.glyphs.len() || !glyphs.iter().any(|g| g.tfm_code.is_some()) {
            return None;
        }
        let (face, size, style) = (face.clone(), *size, *style);
        let z = (size * 65536.0).round() as i32;
        let scaled = |fix: i32| flashtex_microtype::arith::tfm_scaled(fix, z);
        let micro: Vec<pl::MicroGlyph> = glyphs
            .iter()
            .zip(&run.glyphs)
            .map(|(g, pg)| {
                let Some(code) = g.tfm_code else {
                    // A left-boundary kern carried as an empty glyph.
                    return pl::MicroGlyph { code: None, width: 0, kern: scaled(g.advance_fix) };
                };
                let width_fix = g.advance_fix - g.kern_fix;
                // The discretionary hyphen's advance also carries the kern
                // between the letter before it and the hyphen (`word_items`).
                let placed = pg.advance + pg.kern;
                let kern_fix = if (placed - crate::tfm::Tfm::pt(g.advance_fix, size)).abs() > 1e-9 {
                    ((placed - crate::tfm::Tfm::pt(width_fix, size)) * crate::tfm::FIX as f64 / size).round() as i32
                } else {
                    g.kern_fix
                };
                pl::MicroGlyph { code: Some(code), width: scaled(width_fix), kern: scaled(kern_fix) }
            })
            .collect();
        let params = self.microtype_params(&face, style, size)?;
        Some(pl::MicroRun { params, glyphs: micro })
    }

    /// microtype's `\lpcode`/`\rpcode`/`\efcode` and expansion limits for the
    /// NFSS font LaTeX selects for this face (T1, `cmr` for EC metrics or
    /// `lmr` with `lmodern`), resolved once per face and size from the
    /// bundled `microtype.cfg` + `mt-cmr.cfg`.
    fn microtype_params(&mut self, face: &Rc<LoadedFace>, style: TextStyle, size: f64) -> Option<Rc<flashtex_microtype::FontParams>> {
        let key = (face.shape_key.clone(), size.to_bits());
        if let Some(hit) = self.microtype_fonts.get(&key) {
            return hit.clone();
        }
        let setup = self.style.microtype.clone()?;
        // Across requests (a fresh `Context` per render): resolving parses
        // nothing but still walks the config per font, so the result and any
        // warning are kept per thread for the face, size and options.
        let global_key = (face.shape_key.to_string(), size.to_bits(), format!("{:?}|{:?}|{:?}", setup.options, self.style.family, self.style.nfss));
        if let Some((hit, warning)) = MICROTYPE_FONTS.with(|c| c.borrow().get(&global_key).cloned()) {
            if let Some((k, message)) = warning {
                self.emit(Some(k), Diagnostic::warning("microtype_unsupported", message, Vec::new()));
            }
            self.microtype_fonts.insert(key, hit.clone());
            return hit;
        }
        let mut warning: Option<(String, String)> = None;
        // The NFSS font LaTeX loads for this style (family slot, series and
        // shape after substitutions), named in the document's scheme: `cmr`/
        // `cmss`/`cmtt` or `lmr`/`lmss`/`lmtt`, OT1 or T1 (mt-cmr.cfg and
        // mt-lmr.cfg list both encodings).
        let scheme = self.style.nfss;
        let loaded = match self.text_role(style, size).0 {
            Role::Font(key) => key,
            _ => style.key(),
        };
        let families = match self.style.family {
            Family::ComputerModern | Family::LatinModern => Some((
                scheme.family_name(crate::nfss::FamilyKind::Rm),
                scheme.family_name(crate::nfss::FamilyKind::Sf),
                scheme.family_name(crate::nfss::FamilyKind::Tt),
            )),
            Family::Times => None,
        };
        let resolved = match (families, face.tfm.clone()) {
            (Some((rm, sf, tt)), Some(tfm)) => {
                struct Metrics<'t> {
                    tfm: &'t crate::tfm::Tfm,
                    z: i32,
                }
                impl flashtex_microtype::FontMetrics for Metrics<'_> {
                    fn char_width(&self, slot: u8) -> flashtex_microtype::Scaled {
                        self.tfm.metrics(slot).map_or(0, |m| flashtex_microtype::arith::tfm_scaled(m.width, self.z))
                    }
                    fn quad(&self) -> flashtex_microtype::Scaled {
                        self.tfm.param(6).map_or(0, |q| flashtex_microtype::arith::tfm_scaled(q, self.z))
                    }
                }
                static CONFIG: OnceLock<flashtex_microtype::MicrotypeConfig> = OnceLock::new();
                // `ENC/family/series/shape` as `nfss::Scheme::describe` spells it.
                let described = scheme.describe(loaded);
                let mut parts = described.split('/').skip(2);
                let (series, shape) = (parts.next().unwrap_or("m"), parts.next().unwrap_or("n"));
                let font = flashtex_microtype::NfssFont {
                    encoding: scheme.encoding().to_string(),
                    family: match loaded.family {
                        crate::nfss::FamilyKind::Rm => rm,
                        crate::nfss::FamilyKind::Sf => sf,
                        crate::nfss::FamilyKind::Tt => tt,
                    }
                    .to_string(),
                    series: if style.medium && !loaded.bold() { "m" } else { series }.to_string(),
                    shape: shape.to_string(),
                    size: format!("{size}"),
                };
                let metrics = Metrics { tfm: &tfm, z: (size * 65536.0).round() as i32 };
                let defaults = flashtex_microtype::NfssDefaults::latex(scheme.encoding(), rm, sf, tt);
                match CONFIG.get_or_init(flashtex_microtype::MicrotypeConfig::bundled).resolve(&setup.options, &defaults, &font, &metrics) {
                    Ok(r) => Some(Rc::new(r.params)),
                    Err(e) => {
                        let message = format!(
                            "microtype setup for {}/{}/{}/{}/{} is not modelled ({e:?}); that font gets no protrusion or expansion",
                            font.encoding, font.family, font.series, font.shape, font.size
                        );
                        warning = Some((format!("microtype:{}:{}", face.name, font.size), message));
                        None
                    }
                }
            }
            _ => None,
        };
        if let Some((k, message)) = &warning {
            self.emit(Some(k.clone()), Diagnostic::warning("microtype_unsupported", message.clone(), Vec::new()));
        }
        MICROTYPE_FONTS.with(|c| {
            let mut c = c.borrow_mut();
            if c.len() >= 4096 {
                c.clear();
            }
            c.insert(global_key, (resolved.clone(), warning));
        });
        self.microtype_fonts.insert(key, resolved.clone());
        resolved
    }

    /// Builds a horizontal list. Returns paragraph-layout items, the
    /// per-item box record and the `\label` keys with the item they precede.
    /// `\\[<dimen>]` skips are returned as `(forced-break item, points)`;
    /// [`vskips_of`] maps them onto the lines after breaking.
    #[allow(clippy::type_complexity)]
    fn hlist(&mut self, items: &[AItem], size: f64, base: TextStyle, style: ParaStyle) -> (Vec<pl::Item>, Vec<Option<usize>>, Vec<(String, usize)>, Vec<(usize, f64)>) {
        // `\centering`/`\raggedleft` set `\parfillskip 0pt` and make `\\`
        // end the paragraph (`\@centercr`); the fil glue of the skips
        // fills the line. Elsewhere `\\` is `\hfil\break` and the paragraph
        // ends with `\parfillskip 0pt plus 1fil`.
        let fills = !matches!(style, ParaStyle::Center | ParaStyle::FlushRight);
        let mut out: Vec<pl::Item> = Vec::new();
        let mut recs: Vec<Option<usize>> = Vec::new();
        let mut labels: Vec<(String, usize)> = Vec::new();
        let mut skips: Vec<(usize, f64)> = Vec::new();
        let push = |out: &mut Vec<pl::Item>, recs: &mut Vec<Option<usize>>, item: pl::Item, rec: Option<usize>| {
            out.push(item);
            recs.push(rec);
        };
        // Notes of this list: (item index, mark record, note).
        let mut notes: Vec<(usize, Option<usize>, usize)> = Vec::new();
        for (idx, item) in items.iter().enumerate() {
            match item {
                AItem::Footnote { number, mark, span, text } => {
                    let note = text.as_ref().map(|t| {
                        self.notes.push(footnotes::NoteSrc { number: number.clone(), span: *span, items: t.clone() });
                        self.notes.len() - 1
                    });
                    let mut anchor = None;
                    if *mark {
                        // `\@footnotemark`: `\nobreak\@makefnmark`.
                        push(&mut out, &mut recs, pl::Item::penalty(pl::INFINITE_PENALTY), None);
                        if let Some((mut run, rec)) = self.footnote_mark(number, *span, size) {
                            if self.rlap_marks {
                                // `\rlap{\@textsuperscript{...}}`.
                                run.width = 0.0;
                            }
                            push(&mut out, &mut recs, pl::Item::Box(run), Some(rec));
                            anchor = Some(rec);
                        }
                    }
                    if let Some(n) = note {
                        notes.push((out.len(), anchor, n));
                    }
                }
                AItem::Word(w) => {
                    // TeX hyphenates a word only when it directly follows
                    // glue (§894: never the first word of a paragraph, which
                    // follows the `\parindent` box, nor an `\item`'s, which
                    // follows the label box and `\penalty0`), when its
                    // letters are in one font (§896), and when nothing but
                    // non-letters follows them up to the next glue, penalty
                    // or kern (§899: a math or word box glued straight on
                    // ends the search with no hyphens).
                    let after_glue = matches!(out.last(), Some(pl::Item::Glue(_)));
                    // §899 also stops at a discretionary: `manu\-scripts`
                    // keeps its one explicit break point.
                    let joined = matches!(items.get(idx + 1), Some(AItem::Word(_) | AItem::Math { .. } | AItem::Discretionary { .. }));
                    // The typewriter families declare `\hyphenchar\font=-1`
                    // (`ot1cmtt.fd`, `t1cmtt.fd`, `t1lmtt.fd`): no hyphens.
                    let hyphenate = after_glue
                        && !joined
                        && w.segments.len() == 1
                        && merge_style(base, w.segments[0].style).family != crate::nfss::FamilyKind::Tt;
                    for seg in &w.segments {
                        let seg = adapter::Segment {
                            text: seg.text.clone(),
                            chars: seg.chars.clone(),
                            style: merge_style(base, seg.style),
                        };
                        // A size declaration in force (`{\Large ...}`) sets
                        // this segment at its own size.
                        let seg_size = seg.style.size_or(size);
                        for (item, rec) in self.word_items(&seg, seg_size, hyphenate) {
                            push(&mut out, &mut recs, item, rec);
                        }
                    }
                }
                AItem::Space { style, factor, no_break } => {
                    let style = merge_style(base, *style);
                    if *no_break {
                        push(&mut out, &mut recs, pl::Item::penalty(pl::INFINITE_PENALTY), None);
                    }
                    let glue = self.space_glue(style, style.size_or(size), *factor);
                    push(&mut out, &mut recs, pl::Item::Glue(glue), None);
                }
                AItem::Math { list, span } => {
                    // `size`, not the body size: math inside a footnote is set
                    // with that size's math fonts (`math_fonts_at`). The split
                    // into `math_pieces` is main's inline-math line breaking and
                    // is orthogonal.
                    if let Some(rec) = self.math_box(list, *span, false, size) {
                        for (item, rec) in self.math_pieces(rec, size, *span) {
                            push(&mut out, &mut recs, item, rec);
                        }
                    }
                }
                AItem::Lap { items: lapped } => {
                    // `\llap{#1}` is `\hb@xt@\z@{\hss #1}`: the material at
                    // its natural width, ending at the reference point.
                    let (mut list, mut lrecs, _, _) = self.hlist(lapped, size, base, style);
                    // `hlist` ends every list it builds with TeX's paragraph
                    // end (`\penalty10000 \parfillskip \penalty-10000`).
                    // That belongs to a paragraph, not to the `\hbox` this
                    // is: left in place it breaks the line after the lapped
                    // material, which is how every numbered listing line
                    // came out one line below its own number.
                    if matches!(
                        list.last_chunk::<3>(),
                        Some([pl::Item::Penalty(_), pl::Item::Glue(_), pl::Item::Penalty(_)])
                    ) {
                        list.truncate(list.len() - 3);
                        lrecs.truncate(lrecs.len().saturating_sub(3));
                    }
                    let width: f64 = list
                        .iter()
                        .map(|i| match i {
                            pl::Item::Box(run) => run.width,
                            pl::Item::Glue(glue) => glue.width,
                            pl::Item::Kern(kern) => kern.width,
                            pl::Item::Penalty(_) => 0.0,
                        })
                        .sum();
                    // The anchor: a box of no size, so the pull-back kern
                    // behind it survives a line break (see `Item::Lap`).
                    self.recs.push(BoxRec::Rule { width: 0.0, height: 0.0, bottom: 0.0, span: Span::new(0, 0) });
                    let anchor = pl::GlyphRun {
                        font: MATH_SENTINEL,
                        size,
                        glyphs: Vec::new(),
                        width: 0.0,
                        height: 0.0,
                        depth: 0.0,
                        source: 0..0,
                    };
                    push(&mut out, &mut recs, pl::Item::Box(anchor), Some(self.recs.len() - 1));
                    push(&mut out, &mut recs, pl::Item::kern(-width), None);
                    for (item, rec) in list.into_iter().zip(lrecs) {
                        push(&mut out, &mut recs, item, rec);
                    }
                }
                AItem::LeaveVmode => {
                    // The empty `\hbox` `\leavevmode` starts a paragraph
                    // with; its only job is to be undiscardable so the
                    // verbatim blank behind it survives the line break.
                    // Every box needs a record (see `NoteParBreak`), so it
                    // is a rule of no width, height or depth: nothing is
                    // shipped for it.
                    self.recs.push(BoxRec::Rule { width: 0.0, height: 0.0, bottom: 0.0, span: Span::new(0, 0) });
                    let run = pl::GlyphRun {
                        font: MATH_SENTINEL,
                        size,
                        glyphs: Vec::new(),
                        width: 0.0,
                        height: 0.0,
                        depth: 0.0,
                        source: 0..0,
                    };
                    push(&mut out, &mut recs, pl::Item::Box(run), Some(self.recs.len() - 1));
                }
                AItem::Penalty { value } => {
                    push(&mut out, &mut recs, pl::Item::penalty(*value), None);
                }
                AItem::PagePenalty { value } => {
                    // A `\vadjust` node is not discardable (TeX §148, §866):
                    // like `LeaveVmode`'s empty box it keeps a following
                    // glue a legal breakpoint and takes no width.
                    self.recs.push(BoxRec::Rule { width: 0.0, height: 0.0, bottom: 0.0, span: Span::new(0, 0) });
                    let run = pl::GlyphRun {
                        font: MATH_SENTINEL,
                        size,
                        glyphs: Vec::new(),
                        width: 0.0,
                        height: 0.0,
                        depth: 0.0,
                        source: 0..0,
                    };
                    self.vadjusts.push((out.len(), *value));
                    push(&mut out, &mut recs, pl::Item::Box(run), Some(self.recs.len() - 1));
                }
                AItem::Discretionary { pre } => {
                    // TeX §1117 `append_discretionary`: `\-` is a flagged
                    // penalty at `\hyphenpenalty` with the face's hyphen as
                    // its pre-break text; an empty pre-break text is
                    // `\exhyphenpenalty`'s.
                    let merged = pre.as_ref().map(|seg| adapter::Segment { text: seg.text.clone(), chars: seg.chars.clone(), style: merge_style(base, seg.style) });
                    let (pre_break, rec) = match merged.as_ref().and_then(|seg| self.text_box(seg, seg.style.size_or(size))) {
                        Some((run, rec)) => (Some(run), Some(rec)),
                        None => (None, None),
                    };
                    let value = if pre_break.is_some() { HYPHEN_PENALTY } else { EX_HYPHEN_PENALTY };
                    push(
                        &mut out,
                        &mut recs,
                        pl::Item::Penalty(pl::Penalty { value, flagged: true, pre_break, automatic: false, post_break: None, replace_count: 0 }),
                        rec,
                    );
                }
                AItem::LineBreak { skip_pt } => {
                    if fills {
                        push(&mut out, &mut recs, pl::Item::Glue(pl::Glue::fil()), None);
                    }
                    if *skip_pt != 0.0 {
                        skips.push((out.len(), *skip_pt));
                    }
                    push(&mut out, &mut recs, pl::Item::penalty(pl::FORCED_BREAK), None);
                }
                AItem::Quad { em, style } => {
                    // `em` is `\fontdimen6` of the font current where the
                    // glue is read: `{\Large a\hspace{2em}b}` is two quads
                    // of the `\Large` face, `{\bfseries a\quad b}` of the bold.
                    let style = merge_base(*style, base);
                    let quad = self.text_params(style, style.size_or(size)).quad;
                    push(&mut out, &mut recs, pl::Item::Glue(pl::Glue::fixed(em * quad)), None);
                }
                AItem::HFill { fill, leader } => {
                    // `\hfill` is second-order glue: it beats the line's
                    // `\parfillskip` (`\hfil`), as in a `\section` title
                    // set as `Problem 1 \hfill [4 points]`.
                    let mut glue = pl::Glue::fil();
                    if *fill {
                        glue.stretch_order = pl::GlueOrder::Fill;
                    }
                    let (box_width, dot) = match leader {
                        FillLeader::Dots => {
                            let face = self.face(base, size, Span::new(0, 0));
                            let shaped = self.shaper.shape(&face, ".");
                            let glyphs = shaped
                                .clusters
                                .iter()
                                .flat_map(|c| c.glyphs.iter().map(|g| pl::ShapedGlyph {
                                    gid: u32::from(g.gid.0),
                                    advance_units: i64::from(g.advance),
                                    cluster: c.text_range.clone(),
                                }))
                                .collect::<Vec<_>>();
                            let run = pl::GlyphRun::from_shaped(
                                face.layout_id(),
                                size,
                                shaped.units_per_em as f64,
                                f64::from(shaped.height_units),
                                -f64::from(shaped.depth_units),
                                &glyphs,
                                0..1,
                            );
                            (0.44 * self.text_params(base, size).quad, Some((face, run)))
                        }
                        _ => (0.0, None),
                    };
                    let rec = if *leader == FillLeader::None {
                        None
                    } else {
                        self.recs.push(BoxRec::Leader { leader: *leader, box_width, dot });
                        Some(self.recs.len() - 1)
                    };
                    push(&mut out, &mut recs, pl::Item::Glue(glue), rec)
                }
                AItem::HSpace { pt, stretch_pt, shrink_pt } => push(
                    &mut out,
                    &mut recs,
                    pl::Item::Glue(pl::Glue::finite(*pt, *stretch_pt, *shrink_pt)),
                    None,
                ),
                AItem::NoteParBreak => {
                    // `\par` (`\parfillskip`), then `\indent`: an empty box
                    // `\parindent` (1em of the note's font) wide.
                    push(&mut out, &mut recs, pl::Item::Glue(pl::Glue::fil()), None);
                    push(&mut out, &mut recs, pl::Item::penalty(pl::FORCED_BREAK), None);
                    // A rule of no height: every box needs a record, and
                    // pdfTeX ships no rule whose height plus depth is 0.
                    let quad = self.text_params(base, size).quad;
                    self.recs.push(BoxRec::Rule { width: quad, height: 0.0, bottom: 0.0, span: Span::new(0, 0) });
                    let indent = pl::GlyphRun { font: MATH_SENTINEL, size, glyphs: Vec::new(), width: quad, height: 0.0, depth: 0.0, source: 0..0 };
                    push(&mut out, &mut recs, pl::Item::Box(indent), Some(self.recs.len() - 1));
                }
                AItem::Table(table) => {
                    if let Some((run, rec)) = self.table_box(table, size) {
                        push(&mut out, &mut recs, pl::Item::Box(run), Some(rec));
                    }
                }
                AItem::ColorBox(cb) => {
                    let (run, rec) = self.color_box(cb, size);
                    push(&mut out, &mut recs, pl::Item::Box(run), Some(rec));
                }
                AItem::Underline(ul) => {
                    let (run, rec) = self.underline_box(ul, size);
                    push(&mut out, &mut recs, pl::Item::Box(run), Some(rec));
                }
                AItem::Kern { amount, style } => {
                    let style = merge_base(*style, base);
                    let cx = self.dimen_context(style, style.size_or(size));
                    let pt = flashtex_compiler::text_builtins::sp_to_pt(amount.resolve(&cx));
                    push(&mut out, &mut recs, pl::Item::kern(pt), None);
                }
                AItem::Rule { rule, style, span } => {
                    let style = merge_base(*style, base);
                    let rule_size = style.size_or(size);
                    let cx = self.dimen_context(style, rule_size);
                    let (run, rec) = self.rule_box(rule, &cx, rule_size, *span);
                    push(&mut out, &mut recs, pl::Item::Box(run), Some(rec));
                }
                AItem::Logo { logo, style, span } => {
                    let style = merge_base(*style, base);
                    for (item, rec) in self.logo_items(*logo, style, style.size_or(size), *span) {
                        push(&mut out, &mut recs, item, rec);
                    }
                }
                AItem::Label { key } => labels.push((key.clone(), out.len())),
                AItem::ItalicCorrection => {
                    // `\/`: a kern of the last character's TFM italic
                    // correction (§1113); nothing when the last node is not
                    // a character or the metrics carry no correction.
                    let last = recs.iter().rev().find_map(|r| *r).and_then(|r| match &self.recs[r] {
                        BoxRec::Text { glyphs, size, .. } if matches!(out.last(), Some(pl::Item::Box(_))) => {
                            glyphs.last().map(|g| crate::tfm::Tfm::pt(g.italic_fix, *size))
                        }
                        _ => None,
                    });
                    if let Some(ic) = last {
                        if ic > 0.0 {
                            push(&mut out, &mut recs, pl::Item::Glue(pl::Glue::fixed(ic)), None);
                        }
                    }
                }
            }
        }
        // TeX's paragraph end: drop trailing glue, then
        // \penalty10000 \parfillskip \penalty-10000.
        while matches!(out.last(), Some(pl::Item::Glue(_))) {
            out.pop();
            recs.pop();
        }
        push(&mut out, &mut recs, pl::Item::penalty(pl::INFINITE_PENALTY), None);
        push(&mut out, &mut recs, pl::Item::Glue(if fills { pl::Glue::fil() } else { pl::Glue::fixed(0.0) }), None);
        push(&mut out, &mut recs, pl::Item::penalty(pl::FORCED_BREAK), None);
        // A `\footnotetext` insert follows the line of the box before it
        // (the first box of the list when there is none).
        for (at, anchor, n) in notes {
            let rec = anchor.or_else(|| recs[..at.min(recs.len())].iter().rev().find_map(|r| *r)).or_else(|| recs.iter().find_map(|r| *r));
            if let Some(rec) = rec {
                self.note_anchors.push((rec, n));
            }
        }
        (out, recs, labels, skips)
    }

    /// A `tabular` as one box (`table.rs`): every entry and `@{}` text is
    /// set as an hbox (a `p{}` entry as its `\vtop`), then placed by the
    /// kernel's alignment rules. The record keeps the pieces to paint.
    fn table_box(&mut self, t: &crate::table::TableItem, outer_size: f64) -> Option<(pl::GlyphRun, usize)> {
        let (metrics, rows, mut blocks) = self.table_measure(t, outer_size);
        let size = if t.size_cpt == 0 { outer_size } else { f64::from(t.size_cpt) / 100.0 };
        let mut geometry = crate::table::layout(t, &rows, &metrics);
        self.resolve_table_colors(t.span, &mut geometry);
        let mut pieces = Vec::new();
        for p in &geometry.placed {
            if let Some(block) = blocks.remove(&(p.row, p.cell, p.slot)) {
                pieces.push(TablePiece { x: p.x, baseline: p.baseline, block });
            }
        }
        self.recs.push(BoxRec::Table(Rc::new(TableRec { pieces, rules: geometry.rules, fills: geometry.fills, span: t.span })));
        let run = pl::GlyphRun {
            font: MATH_SENTINEL,
            size,
            glyphs: Vec::new(),
            width: geometry.width,
            height: geometry.height,
            depth: geometry.depth,
            source: t.span.start..t.span.end,
        };
        Some((run, self.recs.len() - 1))
    }

    /// Sets every entry of a table and measures its cells: the `\@arstrut`
    /// metrics, the measured cells in `row_slots` order, and the built
    /// block of each piece by `(row, cell, slot)`. `table::layout` turns
    /// these into a geometry; a longtable measures once and sets each
    /// chunk against the result (`\LT@get@widths`).
    fn table_measure(
        &mut self,
        t: &crate::table::TableItem,
        outer_size: f64,
    ) -> (crate::table::Metrics, Vec<Vec<crate::table::MCell>>, std::collections::HashMap<(usize, usize, crate::table::Slot), BuiltBlock>) {
        use crate::table::{self as tb, Dims, MCell, Slot, TableMaterial};
        use flashtex_compiler::parser::ParagraphStyle;
        use flashtex_compiler::tabular::Align;
        let size = if t.size_cpt == 0 { outer_size } else { f64::from(t.size_cpt) / 100.0 };
        let body_size = self.style.body_size_pt;
        // `\strutbox` of the size in force (`\@setfontsize`).
        let bskip = if (size - body_size).abs() < 1e-9 {
            self.style.baselineskip_pt
        } else {
            tb::baselineskip_pt(adapter::class_size_of(body_size), (size * 100.0).round() as u16)
        };
        // `\@arstrutbox` (array.sty 207 adds `\extrarowheight` to the height).
        let plain_strut_height = tb::sp(0.7 * bskip);
        let strut_height = tb::sp(t.arraystretch * (plain_strut_height + tb::sp(t.lengths.extrarowheight)));
        let strut_depth = tb::sp(t.arraystretch * tb::sp(0.3 * bskip));
        let body = self.text_params(TextStyle::default(), body_size);
        let quad = self.text_params(TextStyle::default(), size).quad;
        let measure = self.style.text_width_pt;
        let metrics = tb::Metrics { strut_height, strut_depth, plain_strut_height, em: body.quad, ex: body.x_height, axis: tb::AXIS_EM * size, measure };
        let mut blocks: std::collections::HashMap<(usize, usize, Slot), BuiltBlock> = std::collections::HashMap::new();
        let mut rows: Vec<Vec<MCell>> = Vec::new();
        for entry in &t.entries {
            let tb::TableEntry::Row { cells, .. } = entry else { continue };
            let ri = rows.len();
            let mut out = Vec::new();
            for (ci, (cell, (column, columns, template))) in cells.iter().zip(tb::row_slots(t, cells)).enumerate() {
                let (before_m, align, after_m): (&[TableMaterial], Align, &[TableMaterial]) = match template {
                    Some(c) => (&c.before, c.align, &c.after),
                    None => (&[], Align::Left, &[]),
                };
                let before = self.table_pieces(before_m, size, (ri, ci), true, &mut blocks);
                let mut content_offset = (0.0, 0.0);
                if let Some(mr) = &cell.multirow {
                    let content = self.multirow_box(mr, &cell.items, size, bskip, quad, align, &metrics, (ri, ci), &mut blocks, &mut content_offset);
                    let after = self.table_pieces(after_m, size, (ri, ci), false, &mut blocks);
                    out.push(MCell { column, columns, align, before, content, after, content_offset });
                    continue;
                }
                let content = match align {
                    Align::Paragraph(len) | Align::Middle(len) | Align::Bottom(len) => {
                        let width = tb::resolve(len, measure).max(0.0);
                        let style = match cell.alignment {
                            Some(ParagraphStyle::Center) => ParaStyle::Center,
                            Some(ParagraphStyle::FlushLeft) => ParaStyle::FlushLeft,
                            Some(ParagraphStyle::FlushRight) => ParaStyle::FlushRight,
                            Some(ParagraphStyle::Quote) | None => ParaStyle::Plain,
                        };
                        let lines = match self.table_pbox(&cell.items, size, width, bskip, quad, style) {
                            Some((block, lines)) => {
                                blocks.insert((ri, ci, Slot::Content), block);
                                lines
                            }
                            None => tb::ParLines::default(),
                        };
                        let (dims, shift) = tb::parbox(align, lines, t.array_package, &metrics);
                        content_offset.1 = shift;
                        dims
                    }
                    _ => match self.table_hbox(&cell.items, size) {
                        Some((block, mut dims)) => {
                            blocks.insert((ri, ci, Slot::Content), block);
                            if let Align::Fixed(len, pos) = align {
                                let (width, dx) = tb::fixed_box(pos, tb::resolve(len, measure).max(0.0), dims.width);
                                dims.width = width;
                                content_offset.0 = dx;
                            }
                            dims
                        }
                        None => Dims { width: if let Align::Fixed(len, _) = align { tb::resolve(len, measure).max(0.0) } else { 0.0 }, ..Dims::default() },
                    },
                };
                let after = self.table_pieces(after_m, size, (ri, ci), false, &mut blocks);
                out.push(MCell { column, columns, align, before, content, after, content_offset });
            }
            rows.push(out);
        }
        (metrics, rows, blocks)
    }

    /// A `longtable` as a block of the page's vertical list
    /// (`crate::longtable`). Every row, rule row and head or foot box is a
    /// "line" of the block set at `\LTleft`; the repeating head and foot
    /// are built too but kept out of the contributed list, for the page
    /// builder to insert at a break (`\LT@output`).
    fn longtable_block(&mut self, t: &crate::table::TableItem, lengths: &adapter::LongtableLengths) -> Option<(BuiltBlock, pagebuild::Region)> {
        use crate::longtable::{self as lt, UnitKind};
        let outer_size = self.style.body_size_pt;
        let size = if t.size_cpt == 0 { outer_size } else { f64::from(t.size_cpt) / 100.0 };
        let mut blocks_for_captions: std::collections::HashMap<(usize, usize, crate::table::Slot), BuiltBlock> = std::collections::HashMap::new();
        // `\LT@makecaption` sets each `\caption` in its own parbox before
        // the alignment is measured, so the table below works on a copy
        // whose caption entries carry their boxes.
        let mut owned;
        let t = if t.entries.iter().any(|e| matches!(e, crate::table::TableEntry::Caption { .. })) {
            owned = t.clone();
            self.longtable_captions(&mut owned, lengths, outer_size, &mut blocks_for_captions);
            &owned
        } else {
            t
        };
        let (metrics, rows, mut blocks) = self.table_measure(t, outer_size);
        blocks.extend(std::mem::take(&mut blocks_for_captions));
        // `\LT@get@widths` measures every chunk, `\kill` rows included.
        let cols = crate::table::widths(t, &rows, &metrics);
        let parts = lt::parts(t);
        // `\tabskip\LTleft`/`\LTright` carry the difference between the
        // table's natural width and `\hsize` (longtable.sty 167-171).
        let measure = self.style.text_width_pt;
        let x = match (lengths.left, lengths.right) {
            (Some(l), _) => l,
            (None, Some(r)) => (measure - cols.box_width - r).max(0.0),
            (None, None) => lt::indent(t.longtable.as_ref().and_then(|l| l.align), cols.box_width, measure),
        };
        let span = t.span;
        let width = cols.box_width;
        let mut lines: Vec<pl::Line> = Vec::new();
        let mut vlines: Vec<(f64, f64)> = Vec::new();
        let mut items: Vec<pl::Item> = Vec::new();
        let mut recs: Vec<Option<usize>> = Vec::new();
        let mut line_penalty: Vec<(usize, i32)> = Vec::new();
        // One "line" holding the slice `top..bottom` of a chunk's geometry,
        // its own baseline at y = 0 like any other box on a line.
        let push = |ctx: &mut Context,
                        chunk: &lt::Chunk,
                        top: f64,
                        bottom: f64,
                        baseline: Option<f64>,
                        blocks: &mut std::collections::HashMap<(usize, usize, crate::table::Slot), BuiltBlock>,
                        lines: &mut Vec<pl::Line>,
                        vlines: &mut Vec<(f64, f64)>,
                        items: &mut Vec<pl::Item>,
                        recs: &mut Vec<Option<usize>>| {
            let base = baseline.unwrap_or(bottom);
            let inside = |v: f64| v > top - 1e-9 && v < bottom - 1e-9 || (v - top).abs() < 1e-9;
            let mut pieces = Vec::new();
            for p in &chunk.geometry.placed {
                if !inside(p.baseline) {
                    continue;
                }
                // Cloned, not removed: without `\endfirsthead` the opening
                // head *is* `\LT@head` (longtable.sty 239), and without
                // `\endlastfoot` the closing foot *is* `\LT@foot` (506), so
                // the same cells are set once in the contributed list and
                // again in the box the output routine repeats — `\copy`,
                // not `\box`. Removing left the repeated head and foot with
                // their rules but no text.
                if let Some(block) = blocks.get(&(p.row, p.cell, p.slot)) {
                    pieces.push(TablePiece { x: p.x, baseline: p.baseline - base, block: block.clone() });
                }
            }
            let cut = |rs: &[crate::table::PlacedRule]| -> Vec<crate::table::PlacedRule> {
                rs.iter()
                    .filter(|r| inside(r.top))
                    .map(|r| crate::table::PlacedRule { top: r.top - base, ..r.clone() })
                    .collect()
            };
            let rec = TableRec { pieces, rules: cut(&chunk.geometry.rules), fills: cut(&chunk.geometry.fills), span };
            let (height, depth) = (base - top, bottom - base);
            ctx.recs.push(BoxRec::Table(Rc::new(rec)));
            let run = pl::GlyphRun {
                font: MATH_SENTINEL,
                size,
                glyphs: Vec::new(),
                width,
                height,
                depth,
                source: span.start..span.end,
            };
            let index = lines.len();
            lines.push(pl::Line {
                index,
                runs: vec![position_run(&run, x, 0.0)],
                baseline_y: height,
                height,
                depth,
                natural_width: width,
                set_width: width,
                ratio: 0.0,
                badness: 0.0,
                items: items.len()..items.len() + 1,
                hyphenated: false,
            });
            items.push(pl::Item::Box(run));
            recs.push(Some(ctx.recs.len() - 1));
            vlines.push((height, depth));
        };
        // `\ifvoid\LT@firsthead\copy\LT@head\else\box\LT@firsthead\fi`
        // followed by `\nobreak` (longtable.sty 239).
        let opening = parts.opening_head().to_vec();
        let mut opening_depth = None;
        if !opening.is_empty() {
            let mut chunk = lt::chunk_geometry(t, &rows, &metrics, &cols, &opening);
            self.resolve_table_colors(span, &mut chunk.geometry);
            let (h, d) = (chunk.height, chunk.depth);
            opening_depth = Some(d);
            push(self, &chunk, 0.0, h + d, Some(h), &mut blocks, &mut lines, &mut vlines, &mut items, &mut recs);
            line_penalty.push((vlines.len(), pagebuild::INF_PENALTY));
        }
        // The body, row by row, so the page builder can break inside it.
        let body = parts.body.clone();
        if !body.is_empty() {
            let mut chunk = lt::chunk_geometry(t, &rows, &metrics, &cols, &body);
            self.resolve_table_colors(span, &mut chunk.geometry);
            let mut pending: Option<i32> = None;
            for unit in lt::units(t, &chunk) {
                if let Some(p) = unit.penalty_before {
                    pending = Some(pending.map_or(p, |q: i32| q.min(p)));
                }
                if unit.kind == UnitKind::Box {
                    if let Some(p) = pending.take() {
                        line_penalty.push((vlines.len(), p));
                    }
                    push(self, &chunk, unit.top, unit.bottom, unit.baseline, &mut blocks, &mut lines, &mut vlines, &mut items, &mut recs);
                }
            }
        }
        // `\box\ifvoid\LT@lastfoot\LT@foot\else\LT@lastfoot\fi` (506).
        let closing = parts.closing_foot().to_vec();
        let tail_from = vlines.len();
        let mut tail_foot_height = 0.0;
        let mut closing_depth = None;
        if !closing.is_empty() {
            let mut chunk = lt::chunk_geometry(t, &rows, &metrics, &cols, &closing);
            self.resolve_table_colors(span, &mut chunk.geometry);
            let (h, d) = (chunk.height, chunk.depth);
            tail_foot_height = h;
            closing_depth = Some(d);
            push(self, &chunk, 0.0, h + d, Some(h), &mut blocks, &mut lines, &mut vlines, &mut items, &mut recs);
        }
        if vlines.is_empty() {
            return None;
        }
        let contributed = vlines.len();
        // `\LT@head` and `\LT@foot` are built but not contributed: the page
        // builder inserts them at a break inside the table (`\LT@output`).
        let reserve = |ctx: &mut Context,
                           indices: &[usize],
                           blocks: &mut std::collections::HashMap<(usize, usize, crate::table::Slot), BuiltBlock>,
                           lines: &mut Vec<pl::Line>,
                           vlines: &mut Vec<(f64, f64)>,
                           items: &mut Vec<pl::Item>,
                           recs: &mut Vec<Option<usize>>|
         -> Option<(usize, f64, f64)> {
            if indices.is_empty() {
                return None;
            }
            let mut chunk = lt::chunk_geometry(t, &rows, &metrics, &cols, indices);
            ctx.resolve_table_colors(span, &mut chunk.geometry);
            let at = vlines.len();
            let (h, d) = (chunk.height, chunk.depth);
            push(ctx, &chunk, 0.0, h + d, Some(h), blocks, lines, vlines, items, recs);
            Some((at, h, d))
        };
        let head = reserve(self, parts.head.as_slice(), &mut blocks, &mut lines, &mut vlines, &mut items, &mut recs);
        let foot = reserve(self, parts.foot.as_slice(), &mut blocks, &mut lines, &mut vlines, &mut items, &mut recs);
        // `\LTpre`/`\LTpost` default to `\bigskipamount` (size1X.clo:
        // 12pt plus 4pt minus 4pt in every standard size).
        let bigskip = crate::style::Skip { natural: 12.0, stretch: 4.0, shrink: 4.0 };
        let pre = lengths.pre.map_or(bigskip, crate::style::Skip::fixed);
        let post = lengths.post.map_or(bigskip, crate::style::Skip::fixed);
        let vertical = VBlock {
            lines: vlines,
            penalty_before: Some(0),
            space_before: Some(skip_tuple(pre)),
            parskip: None,
            interline_penalty: 0,
            club_penalty: 0,
            widow_penalty: 0,
            penalty_after: Some(0),
            space_after: Some(skip_tuple(post)),
            no_interline_first: true,
            no_interline_after: false,
            // longtable.sty 191: `\lineskip\z@\baselineskip\z@`, so the
            // rows abut and every gap between them is a legal breakpoint.
            baselineskip: Some(0.0),
            lineskip: Some(0.0),
            vskip_after: Vec::new(),
            broken_penalty: Vec::new(),
            vadjust_penalty: Vec::new(),
            fil_break: false,
            pre_space_after: None,
            contributed: Some(contributed),
            line_penalty,
            // The chunks are `\unvbox`ed, which leaves `\prevdepth` alone,
            // so only a `\box` sets it. `\LT@start` runs `\box\LT@firsthead`
            // (or `\copy\LT@head`) on the outer vertical list, so the
            // opening head's depth is what the table leaves behind.
            //
            // The closing foot does *not*: `\box\ifvoid\LT@lastfoot\LT@foot
            // \else\LT@lastfoot\fi` is inside `\LT@output` (longtable.sty
            // 506), and the output routine builds its own vertical list
            // (§1025 `push_nest`), whose `prev_depth` is discarded when
            // §1026 hands the material back to the contribution list.
            // Measured: a table whose `\endfoot` ends in a text row and
            // whose opening head ends in `\hline` leaves `\prevdepth` 0,
            // not 4.35pt — and the paragraph after it is the same distance
            // below whether the paragraph *before* the table had a
            // descender or not, so the value is fixed, not inherited
            // (106-longtable-head-foot-only).
            depth_after: match opening_depth {
                Some(d) => pagebuild::DepthAfter::Fixed(d),
                None => pagebuild::DepthAfter::Unchanged,
            },
        };
        let region = pagebuild::Region {
            lines: 0..contributed,
            head: head.map(|(i, h, d)| (i, h, d)),
            foot: foot.map(|(i, h, d)| (i, h, d)),
            tail_from,
            tail_foot_height,
        };
        let height = lines.first().map_or(0.0, |l| l.height);
        let block = pl::ParagraphBlock::body(pl::Lines {
            lines,
            breaks: Vec::new(),
            stats: one_line_stats(),
            diagnostics: Vec::new(),
            height,
        });
        Some((BuiltBlock { block, items, recs, vertical, labels: Vec::new(), cache_key: None }, region))
    }

    /// `\LT@makecaption` (longtable.sty 475-485): every `\caption` of a
    /// longtable is set in a `\parbox[t]\LTcapwidth` that the row centres
    /// on the table. The whole caption goes on one centred line when it
    /// fits in `\LTcapwidth` (`\hbox to\hsize{\hfil\box\@tempboxa\hfil}`)
    /// and is set as a paragraph otherwise, followed by
    /// `\endgraf\vskip\baselineskip`. The parbox is a `\vtop`, so its
    /// height is the first line's and its depth everything below.
    fn longtable_captions(
        &mut self,
        t: &mut crate::table::TableItem,
        lengths: &adapter::LongtableLengths,
        outer_size: f64,
        blocks: &mut std::collections::HashMap<(usize, usize, crate::table::Slot), BuiltBlock>,
    ) {
        let size = if t.size_cpt == 0 { outer_size } else { f64::from(t.size_cpt) / 100.0 };
        let body_size = self.style.body_size_pt;
        let bskip = if (size - body_size).abs() < 1e-9 {
            self.style.baselineskip_pt
        } else {
            crate::table::baselineskip_pt(adapter::class_size_of(body_size), (size * 100.0).round() as u16)
        };
        let quad = self.text_params(TextStyle::default(), size).quad;
        let width = crate::longtable::caption_width(lengths.capwidth);
        let mut set: Vec<(usize, crate::table::CaptionBox, BuiltBlock)> = Vec::new();
        for (index, entry) in t.entries.iter().enumerate() {
            let crate::table::TableEntry::Caption { items, .. } = entry else { continue };
            // `\sbox\@tempboxa{...}`: does the whole caption fit on a line?
            let natural = self.table_hbox(items, size).map_or(0.0, |(_, d)| d.width);
            let style = if natural > width { ParaStyle::Plain } else { ParaStyle::Center };
            let Some((block, lines)) = self.table_pbox(items, size, width, bskip, quad, style) else { continue };
            set.push((
                index,
                crate::table::CaptionBox {
                    width,
                    height: lines.first_height,
                    depth: lines.inner + lines.last_depth + bskip,
                },
                block,
            ));
        }
        for (index, box_, block) in set {
            if let crate::table::TableEntry::Caption { box_: slot, .. } = &mut t.entries[index] {
                *slot = Some(box_);
            }
            // `table::layout_with` places a caption as `(usize::MAX, index)`.
            blocks.insert((usize::MAX, index, crate::table::Slot::Content), block);
        }
    }

    /// Resolves colortbl colours to sRGB; an unresolvable colour paints
    /// black and is reported once per specification.
    fn resolve_table_colors(&mut self, span: Span, geometry: &mut crate::table::Geometry) {
        let defined = crate::tablecolor::definitions(self.texts.get(span.document.0).copied().unwrap_or(""));
        let mut unresolved = Vec::new();
        for r in geometry.rules.iter_mut().chain(geometry.fills.iter_mut()) {
            let Some(c) = &r.color else { continue };
            r.rgb = crate::tablecolor::resolve(c.model.as_deref(), &c.spec, &defined);
            if r.rgb.is_none() && !unresolved.iter().any(|(s, _): &(String, Span)| *s == c.spec) {
                unresolved.push((c.spec.clone(), c.span));
            }
        }
        for (spec, at) in unresolved {
            let src = vec![self.source(at)];
            self.emit(
                None,
                Diagnostic::warning(
                    "table_limitation",
                    format!("table colour '{spec}' is not a colour this pipeline resolves yet (base xcolor names, \\definecolor rgb/RGB/HTML/gray/cmyk, a!p!b mixes); painted black"),
                    src,
                ),
            );
        }
    }

    /// multirow.sty v2.9 `\@xmultirow` (168-201): the text is set in box 0,
    /// `\vtop to\multirow@dima` of `nrows` strut rows plus the bigstruts
    /// (with `\vfill` above and/or below by `vpos`), raised by the final
    /// `\multirow@dima` plus `vmove`, and entered as a box of no height and
    /// no depth. Returns that box; `offset.1` is the first baseline's y
    /// below the row's baseline.
    #[allow(clippy::too_many_arguments)]
    fn multirow_box(
        &mut self,
        mr: &flashtex_compiler::tabular::Multirow,
        items: &[AItem],
        size: f64,
        bskip: f64,
        quad: f64,
        align: flashtex_compiler::tabular::Align,
        m: &crate::table::Metrics,
        key: (usize, usize),
        blocks: &mut std::collections::HashMap<(usize, usize, crate::table::Slot), BuiltBlock>,
        offset: &mut (f64, f64),
    ) -> crate::table::Dims {
        use crate::table::{self as tb, Dims, Slot};
        use flashtex_compiler::tabular::{MultirowPos, MultirowWidth};
        const BIGSTRUTJOT: f64 = 3.0; // `\bigstrutjot=\jot`
        // `\strut` inside the box is `\strutbox` of the size in force.
        let plain_height = tb::sp(0.7 * bskip);
        let plain_depth = tb::sp(0.3 * bskip);
        let (ht, dp) = (m.strut_height, m.strut_depth);
        let jot = |on: bool| if on { BIGSTRUTJOT } else { 0.0 };
        let d0 = mr.rows.abs() * (ht + dp) + f64::from(mr.bigstrut_count) * BIGSTRUTJOT;
        // (width, first line height, first-to-last baseline, last depth).
        let (width, first, inner, last) = match mr.width {
            MultirowWidth::Natural => match self.table_hbox(items, size) {
                Some((block, dims)) => {
                    blocks.insert((key.0, key.1, Slot::Content), block);
                    (dims.width, dims.height.max(plain_height), 0.0, dims.depth.max(plain_depth))
                }
                None => (0.0, plain_height, 0.0, plain_depth),
            },
            MultirowWidth::Column | MultirowWidth::Fixed(_) => {
                let width = match mr.width {
                    MultirowWidth::Fixed(len) => tb::resolve(len, m.measure),
                    _ => align.paragraph_width().map_or(m.measure, |len| tb::resolve(len, m.measure)),
                }
                .max(0.0);
                // `\multirowsetup` is `\raggedright`.
                match self.table_pbox(items, size, width, bskip, quad, ParaStyle::FlushLeft) {
                    Some((block, lines)) => {
                        blocks.insert((key.0, key.1, Slot::Content), block);
                        (width, lines.first_height.max(plain_height), lines.inner, lines.last_depth.max(plain_depth))
                    }
                    None => (width, plain_height, 0.0, plain_depth),
                }
            }
        };
        // The first baseline below box 0's top: `\vfill` glue does not
        // shrink, so an overfull box keeps its natural positions.
        let within = match mr.vpos {
            MultirowPos::Top => first,
            MultirowPos::Center => ((d0 - first - inner - last) / 2.0).max(0.0) + first,
            MultirowPos::Bottom => (d0 - first - inner).max(0.0) + first,
        };
        // `\ht0`: box 0 is a `\vtop` whose first item is the text for `t`.
        let ht0 = if mr.vpos == MultirowPos::Top { first } else { 0.0 };
        let mut raise = if mr.rows > 0.0 {
            match mr.vpos {
                MultirowPos::Top => ht0,
                MultirowPos::Center => ht + jot(mr.bigstrut_top),
                MultirowPos::Bottom => ht + jot(mr.bigstrut_top) + dp + jot(mr.bigstrut_bottom),
            }
        } else {
            match mr.vpos {
                MultirowPos::Bottom => d0,
                MultirowPos::Center => d0 - dp - jot(mr.bigstrut_bottom),
                MultirowPos::Top => d0 - dp - jot(mr.bigstrut_bottom) - ht - jot(mr.bigstrut_top) + ht0,
            }
        };
        raise += mr.vmove_pt;
        offset.1 = within - raise;
        Dims { width, height: 0.0, depth: 0.0 }
    }

    /// Measures a template's `u`/`v` material, shaping `@{}` text.
    fn table_pieces(
        &mut self,
        material: &[crate::table::TableMaterial],
        size: f64,
        key: (usize, usize),
        before: bool,
        blocks: &mut std::collections::HashMap<(usize, usize, crate::table::Slot), BuiltBlock>,
    ) -> Vec<crate::table::MPiece> {
        use crate::table::{MPiece, Slot, TableMaterial};
        let mut out = Vec::with_capacity(material.len());
        for (i, m) in material.iter().enumerate() {
            out.push(match m {
                TableMaterial::Space(pt) => MPiece::Space(*pt),
                TableMaterial::Rule(span) => MPiece::Rule(*span),
                TableMaterial::VLine(span, width) => MPiece::VLine(*span, *width),
                TableMaterial::DoubleRuleGap(width) => MPiece::DoubleRuleGap(*width),
                // `@{...}` material is set in the template as it stands, with
                // no `\ignorespaces`/`\unskip` around it, so glue at either
                // end is kept (`@{\hspace{1em}}`, `@{\quad--\quad}`). The
                // empty boxes `\leavevmode` would put there keep `hlist`'s
                // paragraph end from dropping it.
                TableMaterial::Text(items) => match self.table_hbox(&anchored(items), size) {
                    Some((block, dims)) => {
                        blocks.insert((key.0, key.1, if before { Slot::Before(i) } else { Slot::After(i) }), block);
                        MPiece::Text(dims)
                    }
                    None => MPiece::Text(Default::default()),
                },
            });
        }
        out
    }

    /// An `l`/`c`/`r` entry: its material at natural width (`\hbox`), set
    /// as one unbroken line in a `\maxdimen` measure with `\parfillskip`.
    fn table_hbox(&mut self, items: &[AItem], size: f64) -> Option<(BuiltBlock, crate::table::Dims)> {
        let (list, recs, labels, _) = self.hlist(items, size, TextStyle::default(), ParaStyle::Plain);
        if !list.iter().any(|i| matches!(i, pl::Item::Box(_))) {
            return None;
        }
        let mut params = self.line_params(false, self.style.baselineskip_pt, ParaStyle::Plain, 0.0);
        params.line_width = crate::table::MAX_DIMEN_PT;
        let lines = self.break_paragraph(&list, &params, items, None)?;
        let width = lines.lines.iter().map(|l| l.natural_width).fold(0.0, f64::max);
        let (first, last) = (lines.lines.first()?, lines.lines.last()?);
        let dims = crate::table::Dims { width, height: first.height, depth: last.baseline_y - first.baseline_y + last.depth };
        Some((table_cell_block(lines, list, recs, labels), dims))
    }

    /// A `p{width}`/`m{}`/`b{}` entry's paragraph: `\@startpbox` (`\hsize`
    /// width, `\@arrayparboxrestore`: no indent, `\normalbaselineskip`,
    /// `\sloppy`), then any `\centering`/`\raggedright` from the entry.
    /// `table::parbox` makes the box from these lines.
    fn table_pbox(&mut self, items: &[AItem], size: f64, width: f64, baselineskip: f64, em: f64, style: ParaStyle) -> Option<(BuiltBlock, crate::table::ParLines)> {
        let (list, recs, labels, _) = self.hlist(items, size, TextStyle::default(), style);
        if !list.iter().any(|i| matches!(i, pl::Item::Box(_))) {
            return None;
        }
        let mut params = self.line_params(false, baselineskip, style, 0.0);
        params.line_width = width;
        // `\sloppy`: `\tolerance 9999 \emergencystretch 3em \hfuzz .5pt`.
        // paragraph-layout's `badness` is TeX's integer one (tex.web 108,
        // infinite only above a stretch ratio of 1290/297 = 4.34), so loose
        // lines TeX accepts at this tolerance are feasible here too.
        params.tolerance = 9999.0;
        params.emergency_stretch = 3.0 * em;
        params.hfuzz = 0.5;
        let lines = self.break_paragraph(&list, &params, items, Some(&recs))?;
        self.report_overfull(&lines, &list, &recs);
        let (first, last) = (lines.lines.first()?, lines.lines.last()?);
        let par = crate::table::ParLines { first_height: first.height, inner: last.baseline_y - first.baseline_y, last_depth: last.depth };
        Some((table_cell_block(lines, list, recs, labels), par))
    }

    fn line_params(&self, indent: bool, baselineskip: f64, style: ParaStyle, hang_pt: f64) -> pl::LineBreakParams {
        let s = self.style;
        // `\centering`: `\leftskip`/`\rightskip` `0pt plus 1fil`; `\raggedleft`:
        // `\leftskip` alone; `\raggedright`: `\rightskip` (the crate's ragged
        // mode); `quote`: `\list` with `\leftmargin=\rightmargin=\leftmargini`
        // (`\parshape` in LaTeX; the same lines as fixed skips here).
        let fil = pl::Glue::fil();
        let margin = pl::Glue::fixed(s.leftmargini_pt);
        let (mode, left_skip, right_skip) = match style {
            ParaStyle::Plain => (pl::BreakMode::Justified, pl::Glue::fixed(0.0), pl::Glue::fixed(0.0)),
            ParaStyle::Center => (pl::BreakMode::Justified, fil.clone(), fil),
            ParaStyle::FlushRight => (pl::BreakMode::Justified, fil, pl::Glue::fixed(0.0)),
            ParaStyle::FlushLeft => (pl::BreakMode::RaggedRight, pl::Glue::fixed(0.0), pl::Glue::fixed(0.0)),
            ParaStyle::Quote => (pl::BreakMode::Justified, margin.clone(), margin),
        };
        // `\list`: `\parshape` every line `\@totalleftmargin` in (`\rightmargin`
        // is 0pt), on top of any `quote` margin.
        let left_skip = if hang_pt != 0.0 { pl::Glue::fixed(left_skip.width + hang_pt) } else { left_skip };
        let mut params = pl::LineBreakParams {
            line_width: s.text_width_pt,
            mode,
            algorithm: pl::Algorithm::TotalFit,
            pretolerance: s.pretolerance,
            tolerance: s.tolerance,
            emergency_stretch: s.emergency_stretch_pt,
            line_penalty: s.linepenalty,
            adj_demerits: s.adjdemerits,
            double_hyphen_demerits: 10_000.0,
            final_hyphen_demerits: 5_000.0,
            parindent: if indent { s.parindent_pt } else { 0.0 },
            left_skip,
            right_skip,
            baselineskip,
            lineskip: s.lineskip_pt,
            lineskiplimit: s.lineskiplimit_pt,
            hfuzz: 0.1,
            hbadness: 1000.0,
        };
        // The document's own `\tolerance`, `\sloppy`, ... where the paragraph
        // ends. A box's `\@parboxrestore` below starts from its own values.
        let o = self.breaking;
        if !self.parbox {
            if let Some(v) = o.tolerance {
                params.tolerance = f64::from(v);
            }
            if let Some(v) = o.pretolerance {
                params.pretolerance = f64::from(v);
            }
            if let Some(pt) = o.emergency_stretch_pt {
                params.emergency_stretch = pt;
            }
            if let Some(pt) = o.hfuzz_pt {
                params.hfuzz = pt;
            }
        }
        // `\@parboxrestore`: `\parindent\z@ ... \sloppy`, the same values
        // `\@startpbox` gives a `p{}` cell.
        if self.parbox {
            params.parindent = 0.0;
            params.tolerance = 9999.0;
            params.emergency_stretch = 3.0 * self.text_params(TextStyle::default(), s.body_size_pt).quad;
            params.hfuzz = 0.5;
            params.hbadness = 10_000.0;
        }
        params
    }

    /// A body paragraph (or the part of one before/after a display).
    /// `starts_paragraph` adds `\parskip`; `after_heading` is LaTeX's
    /// `\@afterheading` (`\clubpenalty 10000`). `sized` sets the paragraph
    /// at a size other than `\normalsize` with that size's own leading and
    /// `em` (`abstract`; see [`adapter::SizedPara`]).
    ///
    /// `leading` is the narrower thing: `\baselineskip` alone, for a
    /// paragraph whose `\par` ran under a size declaration
    /// (`adapter::ParLeading`). The runs keep the sizes they were typed at —
    /// TeX reads `\baselineskip` in `append_to_vlist` (§679) without ever
    /// looking at the boxes it is stacking, so the two are independent.
    fn paragraph_block(
        &mut self,
        items: &[AItem],
        indent: bool,
        starts_paragraph: bool,
        after_heading: bool,
        style: ParaStyle,
        list_geom: Option<&ListGeom>,
        sized: Option<adapter::SizedPara>,
        leading: Option<f64>,
    ) -> Option<BuiltBlock> {
        let size = sized.map_or(self.style.body_size_pt, |s| s.size_pt);
        let baselineskip = leading
            .or(sized.map(|s| s.baselineskip_pt))
            .unwrap_or(self.style.baselineskip_pt);
        let vadjusts_from = self.vadjusts.len();
        let (mut list, mut recs, labels, mut skips) = self.hlist(items, size, TextStyle::default(), style);
        let mut vadjusts: Vec<(usize, i32)> = self.vadjusts.drain(vadjusts_from..).collect();
        if !list.iter().any(|i| matches!(i, pl::Item::Box(_))) {
            return None;
        }
        let trailing_skip = drop_trailing_break(&mut list, &mut recs, &mut skips, style);
        // `\item`: the label box `\hskip-\labelwidth \hskip-\labelsep
        // \hbox to\labelwidth{\hfil <label>} \hskip\labelsep` opens the
        // first line (`\@item`'s `\everypar`); a label wider than
        // `\labelwidth` keeps its own width and pushes the text right.
        let mut hang_pt = 0.0;
        let mut inner_margin_pt = 0.0;
        if let Some(geom) = list_geom {
            let (hang, labelwidth, inner) = self.list_geometry(geom, size);
            hang_pt = hang;
            inner_margin_pt = inner;
            if let Some((text, span)) = geom.label.as_ref().filter(|_| starts_paragraph) {
                if let Some(nb) = self.label_box(text, *span, size, geom.description) {
                    let labelsep = self.style.labelsep_pt;
                    let protrude = self.item_left_protrusion(&list, &recs);
                    let mut lead = vec![(pl::Item::kern(-(labelsep + nb.width.min(labelwidth))), None)];
                    // `\descriptionlabel`: `\hspace\labelsep \normalfont
                    // \bfseries #1` — the label box itself opens with
                    // `\labelsep`, so the bold text starts at the margin the
                    // `\itemindent` below put the line on.
                    if geom.description {
                        lead.push((pl::Item::kern(labelsep), None));
                    }
                    let mut at = 0.0;
                    for (run, rec, x) in nb.pieces {
                        if x > at {
                            lead.push((pl::Item::kern(x - at), None));
                        }
                        at = x + run.width;
                        lead.push((pl::Item::Box(run), Some(rec)));
                    }
                    lead.push((pl::Item::kern(labelsep), None));
                    if protrude != 0.0 {
                        lead.push((pl::Item::kern(-protrude), None));
                    }
                    let n = lead.len();
                    for (i, (item, rec)) in lead.into_iter().enumerate() {
                        list.insert(i, item);
                        recs.insert(i, rec);
                    }
                    for (at, _) in &mut skips {
                        *at += n;
                    }
                    for (at, _) in &mut vadjusts {
                        *at += n;
                    }
                }
            }
        }
        let mut params = self.line_params(indent, baselineskip, style, hang_pt);
        // `\list` sets `\parindent\listparindent`, evaluated in the `em`
        // (`\fontdimen6`) of the font the list's own text is set in.
        if let (true, Some(em)) = (indent, sized.and_then(|s| s.parindent_em)) {
            params.parindent = em * self.text_params(TextStyle::default(), size).quad;
        }
        // `\itemindent`: `\@labels` opens the item's first line with
        // `\hskip\itemindent`, so that line alone starts `\leftmargin +
        // \itemindent` in. Only natbib's author-year bibliography sets it
        // (to `-\bibhang`), and only the line the `\item` starts.
        if let Some(geom) = list_geom.filter(|g| g.itemindent_em != 0.0 && starts_paragraph) {
            params.parindent += geom.itemindent_em * self.text_params(TextStyle::default(), size).quad;
        }
        // `description`: `\itemindent-\leftmargin`, so the item's first line
        // is flush at the text margin and only its continuation lines hang
        // `\leftmargin` in (article.cls `\description`).
        if list_geom.is_some_and(|g| g.description) && starts_paragraph {
            params.parindent -= inner_margin_pt;
        }
        let lines = self.break_paragraph(&list, &params, items, Some(&recs))?;
        self.report_overfull(&lines, &list, &recs);
        // `\vspace{<n>em}` inside the paragraph's last group (the abstract
        // head's `\vspace{-.5em}`): `\@vspace` puts it after the line
        // through `\vadjust`, so it is evaluated in the `em` of the font in
        // force there — the style of the paragraph's last word.
        let vspace_after = match sized.map(|s| s.vspace_after_em).filter(|em| *em != 0.0) {
            Some(em) => {
                let last = items.iter().rev().find_map(|i| match i {
                    AItem::Word(w) => w.segments.last().map(|s| s.style.clone()),
                    _ => None,
                });
                em * self.text_params(last.unwrap_or_default(), size).quad
            }
            None => 0.0,
        };
        // `\list` sets `\parskip\parsep`: an item paragraph adds `\parsep`.
        let parskip = self.parskip_of(list_geom);
        let vertical = VBlock {
            lines: line_extents(&lines),
            penalty_before: None,
            space_before: None,
            parskip: starts_paragraph.then(|| skip_tuple(parskip)),
            interline_penalty: self.breaking.interline_penalty.unwrap_or(0),
            club_penalty: if after_heading { pagebuild::INF_PENALTY } else { self.breaking.club_penalty.unwrap_or(CLUB_PENALTY) },
            widow_penalty: self.breaking.widow_penalty.unwrap_or(WIDOW_PENALTY),
            penalty_after: None,
            space_after: match (trailing_skip, vspace_after) {
                (None, 0.0) => None,
                (None, pt) => Some((pt, 0.0, 0.0)),
                (Some(pt), after) => {
                    // `\@xcentercr`: `\par \addvspace{-\parskip} \vskip <dimen>`;
                    // the paragraph that follows adds `\parskip` back, so under
                    // `\centering` only the `[<dimen>]` separates the lines.
                    let p = self.style.parskip;
                    Some(if matches!(style, ParaStyle::Center | ParaStyle::FlushRight) {
                        (pt + after - p.natural, -p.stretch, -p.shrink)
                    } else {
                        (pt + after, 0.0, 0.0)
                    })
                }
            },
            no_interline_first: false,
            no_interline_after: false,
            // Also the glue *above* the first line: `post_line_break`
            // appends every line of the paragraph, the first included, under
            // the same `\baselineskip`.
            baselineskip: leading.or(sized.map(|s| s.baselineskip_pt)),
            vskip_after: vskips_of(&lines, &skips),
            broken_penalty: broken_of(&lines),
            vadjust_penalty: vadjusts
                .iter()
                .filter_map(|(at, pen)| lines.lines.iter().position(|l| l.items.contains(at)).map(|li| (li, *pen)))
                .collect(),
            fil_break: false,
            pre_space_after: None,
            lineskip: None,
            contributed: None,
            line_penalty: Vec::new(),
            depth_after: pagebuild::DepthAfter::default(),
        };
        Some(BuiltBlock {
            block: pl::ParagraphBlock::body(lines),
            items: list,
            recs,
            vertical,
            labels,
            cache_key: None,
        })
    }

    /// The `Block::Paragraph` arm of [`build_with_floats`]: one paragraph,
    /// `\item`, display or `tabular`-carrying block of a vertical list,
    /// with the `\addvspace`/`\topsep` bookkeeping that joins it to the
    /// block before it. A float body sets its own blocks through this same
    /// method ([`Context::box_blocks`]), so there is one code path for
    /// ordinary body content wherever it stands.
    fn build_paragraph(&mut self, blocks: &mut Vec<BuiltBlock>, block: &Block, st: &mut ParaState, cache: Option<&RenderCache>, style_fp: u64, quad: f64) {
        use std::hash::{Hash, Hasher};
        let Block::Paragraph {
            parts,
            indent,
            style,
            env_open,
            env_close,
            eject_before,
            penalty_before,
            breaking,
            vspace_before,
            addvspace_before,
            addvspace_flex,
            vspace_flex,
            endlist_adjust,
            list,
            sized,
            leading_pt,
        } = block
        else {
            return;
        };
        let first_built = blocks.len();
        let outer_breaking = std::mem::replace(&mut self.breaking, *breaking);
        let ctx = self;
        let key_for = |tag: u8, items: &[AItem], flags: &[u64]| block_key(cache, style_fp, tag, items, flags);
            let mut first = true;
            let mut eject = *eject_before;
            let mut vspace = *vspace_before;
            let mut flex = *vspace_flex;
            // `\endtrivlist`: a positive trailing skip of the previous
            // block is changed in place before `\@endparenv`'s
            // `\addvspace` compares against it.
            if *endlist_adjust != 0.0 {
                if let Some(prev) = blocks.last_mut() {
                    if let Some(s) = prev.vertical.space_after.filter(|s| s.0 > 0.0) {
                        prev.vertical.space_after = Some((s.0 + endlist_adjust, s.1, s.2));
                    }
                }
            }
            // `\addvspace`: only the excess over the skip the previous
            // block already left (`\@xaddvskip`).
            if *addvspace_before != 0.0 {
                let prev_after = blocks.last().and_then(|b| b.vertical.space_after).map_or(0.0, |s| s.0);
                vspace += (addvspace_before - prev_after).max(0.0);
                // `\@xaddvskip` keeps whichever skip is larger *whole*: when
                // the previous block's trailing skip wins, its own stretch
                // and shrink are what survive, so the list skip's are not
                // added on top of them.
                if prev_after <= 0.0 {
                    flex.0 += addvspace_flex.0;
                    flex.1 += addvspace_flex.1;
                }
            }
            // `\begin{center}`/`\begin{quote}`: `\addvspace{\topsep}` (plus
            // `\partopsep` from vertical mode) before the first paragraph;
            // `\end{...}` adds the same after the last (`\@endparenv`).
            let env_skip = |vmode: bool| {
                let t = ctx.style.topsep;
                let p = if vmode { ctx.style.partopsep } else { crate::style::Skip::default() };
                (t.natural + p.natural, t.stretch + p.stretch, t.shrink + p.shrink)
            };
            if let Some(e) = env_open {
                st.env_vmode = e.vmode;
                st.env_skips = e.skips;
            }
            // `\@item` opens the environment with `\addvspace{\@topsep}`,
            // not `\vskip`, and only when `\if@nobreak` is false:
            //
            // * `\@xaddvskip` keeps whichever of `\@topsep` and `\lastskip`
            //   is larger, so a skip the previous block already left
            //   absorbs it. `\@maketitle`'s trailing `\vskip 1.5em` (15pt
            //   at a 10pt base, 16.425pt at 11pt) beats `\topsep +
            //   \partopsep` (10pt / 12pt) at every class size, so an
            //   `abstract` or a `quote` right after `\maketitle` opens with
            //   no skip of its own at all.
            // * right after a heading `\@afterheading` has set
            //   `\@nobreaktrue`, so `\@nbitem` runs instead:
            //   `\addvspace{\@outerparskip - \parskip}` cancels `\lastskip`
            //   and the list's own `\parskip` restores it, leaving exactly
            //   the heading's after-skip and no `\@topsep` at all.
            //
            // Both are read off pdfTeX's vertical list; the probes and the
            // quoted `\showoutput` glue are in `tests/abstract_env.rs`.
            let mut env_before = env_open.map(|e| {
                let (n, stretch, shrink) = match e.skips {
                    Some(s) => (s.open.natural, s.open.stretch, s.open.shrink),
                    None => env_skip(e.vmode),
                };
                let last = blocks.last().and_then(|b| b.vertical.space_after).map_or(0.0, |s| s.0);
                if st.after_heading || last >= n {
                    (0.0, 0.0, 0.0)
                } else {
                    (n - last, stretch, shrink)
                }
            });
            // `\endlist` of a list opened at another size takes *that*
            // size's `\@listi` (`abstract`'s `quotation` under `\small`),
            // not the class's `\normalsize` one.
            let env_after = env_close.then(|| match (sized.and_then(|s| s.close_skip), st.env_skips) {
                (Some(s), _) => (s.natural, s.stretch, s.shrink),
                (None, Some(e)) => (e.close.natural, e.close.stretch, e.close.shrink),
                (None, None) => env_skip(st.env_vmode),
            });
            let first_block = blocks.len();
            // TeX's pre_display_size: the width of the line before a
            // display plus 2em; -infinity when nothing precedes it.
            let mut pre_display: Option<f64> = None;
            let geom = list.as_ref();
            let list_fp = geom.map_or(0, |g| {
                let mut h = std::collections::hash_map::DefaultHasher::new();
                g.level.hash(&mut h);
                for m in &g.margins {
                    match m {
                        ListMargin::Fixed(pt) => pt.to_bits().hash(&mut h),
                        ListMargin::Widest(text) => text.hash(&mut h),
                        ListMargin::Em(em) => em.to_bits().hash(&mut h),
                    }
                }
                if let Some((text, span)) = &g.label {
                    text.hash(&mut h);
                    (span.end - span.start).hash(&mut h);
                }
                g.parsep.natural.to_bits().hash(&mut h);
                h.finish()
            });
            for part in parts {
                match part {
                    ParaPart::Lines(items) => {
                        // TeX discards the space token right after a
                        // display's closing `$$` (§1200 resume_after_display).
                        let items = if !first && matches!(items.first(), Some(AItem::Space { .. })) { &items[1..] } else { &items[..] };
                        let (ind, starts, ah) = (*indent && first, first, st.after_heading && first);
                        let sized_fp = sized.map_or(0, |s| {
                            let mut h = std::collections::hash_map::DefaultHasher::new();
                            s.size_pt.to_bits().hash(&mut h);
                            s.baselineskip_pt.to_bits().hash(&mut h);
                            s.parindent_em.map(f64::to_bits).hash(&mut h);
                            s.vspace_after_em.to_bits().hash(&mut h);
                            h.finish()
                        });
                        let (key, origin) = key_for(
                            b'P',
                            items,
                            &[u64::from(ind), u64::from(starts), u64::from(ah), *style as u64, list_fp, sized_fp, leading_pt.map_or(0, f64::to_bits)],
                        );
                        let st = *style;
                        let sz = *sized;
                        let lead = *leading_pt;
                        // `\label` whatsits and a space left in horizontal
                        // mode after a display (the adapter's
                        // `label_line` part): TeX's line_break still sets
                        // them as an empty line, and `\predisplaysize` of a
                        // display after it is -\maxdimen (§1145-§1146).
                        let label_line = !items.is_empty()
                            && matches!(items.last(), Some(AItem::Space { .. }))
                            && items.iter().all(|i| matches!(i, AItem::Label { .. } | AItem::Space { .. }))
                            && items.iter().any(|i| matches!(i, AItem::Label { .. }));
                        if label_line && !first {
                            blocks.push(ctx.empty_line_block());
                            pre_display = None;
                        } else if let Some(mut b) = ctx.cached(cache, key, origin, |c| c.paragraph_block(items, ind, starts, ah, st, geom, sz, lead)) {
                            pre_display = b.block.lines.lines.last().map(|l| l.natural_width + 2.0 * quad);
                            if std::mem::take(&mut eject) {
                                b.vertical.penalty_before = Some(pagebuild::EJECT_PENALTY);
                            }
                            add_skip(&mut b.vertical, std::mem::take(&mut vspace), std::mem::take(&mut flex));
                            add_skip_before(&mut b.vertical, env_before.take());
                            blocks.push(b);
                        }
                    }
                    ParaPart::Rows { env, rows, span, bracket } => {
                        // TeX §1145, exactly as the `Display` arm below: a
                        // display that opens a paragraph whose horizontal
                        // list is still empty sets no line at all. After a
                        // heading `\@afterheading`'s `\everypar` has taken
                        // the `\parindent` box straight back off
                        // (`\setbox\z@\lastbox`), so there is nothing left
                        // to break into one and only `\parskip` precedes the
                        // alignment. Without this an `align` right after a
                        // `\section` carried a phantom empty line worth
                        // `\baselineskip` less the heading's depth -- 13.6 pt
                        // under a heading with no descender, 10.8007 pt under
                        // one with (`tests/align_after_heading.rs`).
                        let mut empty_start = None;
                        if first && st.after_heading && geom.is_none_or(|g| g.label.is_none()) {
                            empty_start = Some((std::mem::take(&mut eject), std::mem::take(&mut vspace), env_before.take()));
                        } else if first {
                            let (mut opener, _) = ctx.display_opener_block(*bracket, geom);
                            if std::mem::take(&mut eject) {
                                opener.vertical.penalty_before = Some(pagebuild::EJECT_PENALTY);
                            }
                            add_vspace(&mut opener.vertical, std::mem::take(&mut vspace));
                            add_skip_before(&mut opener.vertical, env_before.take());
                            blocks.push(opener);
                        }
                        let (key, origin) = if cache.is_some() {
                            let mut h = std::collections::hash_map::DefaultHasher::new();
                            b'A'.hash(&mut h);
                            style_fp.hash(&mut h);
                            span.document.0.hash(&mut h);
                            env.hash(&mut h);
                            (span.end - span.start).hash(&mut h);
                            for row in rows {
                                (row.span.start.wrapping_sub(span.start), row.span.end.wrapping_sub(span.start)).hash(&mut h);
                                row.number.as_ref().map(|(n, _)| n).hash(&mut h);
                                row.cells.len().hash(&mut h);
                                for cell in &row.cells {
                                    incremental::hash_math(cell, &mut h);
                                }
                                row.intertext.len().hash(&mut h);
                                for text in &row.intertext {
                                    (text.short, text.mathtools).hash(&mut h);
                                    incremental::hash_items(&text.items, span.start, &mut h);
                                }
                            }
                            bracket.hash(&mut h);
                            (Some(h.finish()), Some((span.document, span.start)))
                        } else {
                            (None, None)
                        };
                        if let Some(mut b) = ctx.cached(cache, key, origin, |c| c.rows_block(*env, rows, *span)) {
                            if let Some((ej, vs, env_skip)) = empty_start {
                                let parskip = geom.map_or(ctx.style.parskip, |g| g.parsep);
                                if ej {
                                    b.vertical.penalty_before = Some(pagebuild::EJECT_PENALTY);
                                }
                                add_skip_before(&mut b.vertical, Some(skip_tuple(parskip)));
                                add_vspace(&mut b.vertical, vs);
                                add_skip_before(&mut b.vertical, env_skip);
                            }
                            blocks.push(b);
                        }
                        pre_display = None;
                    }
                    ParaPart::Display {
                        list,
                        span,
                        number,
                        bracket,
                    } => {
                        // TeX §1145: a display that opens a paragraph whose
                        // list is still empty sets no line — after a
                        // heading, `\@afterheading`'s `\everypar` has
                        // removed the indent box — so only `\parskip`
                        // precedes it and `\predisplaysize` is -\maxdimen.
                        let mut empty_start = None;
                        if first && st.after_heading && geom.is_none_or(|g| g.label.is_none()) {
                            empty_start = Some((std::mem::take(&mut eject), std::mem::take(&mut vspace), env_before.take()));
                            pre_display = None;
                        } else if first {
                            let (mut opener, size) = ctx.display_opener_block(*bracket, geom);
                            if std::mem::take(&mut eject) {
                                opener.vertical.penalty_before = Some(pagebuild::EJECT_PENALTY);
                            }
                            add_vspace(&mut opener.vertical, std::mem::take(&mut vspace));
                            add_skip_before(&mut opener.vertical, env_before.take());
                            blocks.push(opener);
                            pre_display = Some(size);
                        }
                        let (key, origin) = if cache.is_some() {
                            let mut h = std::collections::hash_map::DefaultHasher::new();
                            b'D'.hash(&mut h);
                            style_fp.hash(&mut h);
                            span.document.0.hash(&mut h);
                            incremental::hash_math(list, &mut h);
                            (span.end - span.start).hash(&mut h);
                            pre_display.map(f64::to_bits).hash(&mut h);
                            if let Some((n, ns)) = number {
                                n.hash(&mut h);
                                (ns.start.wrapping_sub(span.start), ns.end.wrapping_sub(span.start)).hash(&mut h);
                            }
                            bracket.hash(&mut h);
                            (*style as u64).hash(&mut h);
                            list_fp.hash(&mut h);
                            (Some(h.finish()), Some((span.document, span.start)))
                        } else {
                            (None, None)
                        };
                        let pd = pre_display;
                        let st = *style;
                        if let Some(mut b) = ctx.cached(cache, key, origin, |c| c.display_block(list, *span, pd, number.as_ref(), st, geom)) {
                            if let Some((ej, vs, env)) = empty_start {
                                let parskip = geom.map_or(ctx.style.parskip, |g| g.parsep);
                                if ej {
                                    b.vertical.penalty_before = Some(pagebuild::EJECT_PENALTY);
                                }
                                add_skip_before(&mut b.vertical, Some(skip_tuple(parskip)));
                                add_vspace(&mut b.vertical, vs);
                                add_skip_before(&mut b.vertical, env);
                            }
                            blocks.push(b);
                        }
                        pre_display = None;
                    }
                }
                first = false;
            }
            if let Some(skip) = env_after {
                if blocks.len() > first_block {
                    if let Some(last) = blocks.last_mut() {
                        // `\@endparenv` is `\addvspace\@topsepadd`, and
                        // `\addvspace` keeps whichever of the new skip and
                        // `\lastskip` is the larger, *whole* -- it does not
                        // add them (`\@xaddvskip`: `\vskip-\lastskip
                        // \vskip\@tempskipb`, or nothing at all). That is
                        // visible the moment an environment ends in a display:
                        // a theorem whose last thing is `\[...\]` leaves
                        // `\belowdisplayskip` (11pt at an 11pt base), which
                        // beats `\topsep` (9pt) and absorbs it. pdfTeX's own
                        // vertical list for `fixtures/real-world/lecture-notes`
                        // shows `\glue(\belowdisplayskip) 11.0 plus 3.0
                        // minus 6.0`, then `\glue -11.0 ...` and
                        // `\glue 11.0 ...` again: net 11.0, not 20.0.
                        //
                        // Only an environment that declares its own skips
                        // (`adapter::EnvSkips`, i.e. amsthm's) takes that
                        // path. For the rest, `space_after` is not
                        // necessarily `\lastskip` at all: the abstract head's
                        // `\vspace{-.5em}` reaches the page through
                        // `\vadjust`, *before* the penalty and the closing
                        // skip, so `\addvspace` cannot see it and the two do
                        // add up (`adapter::SizedPara::vspace_after_em`).
                        // Telling those two apart for every environment needs
                        // `VBlock` to carry them separately, which is a
                        // change of its own; a theorem block never has a
                        // `sized` vspace, so this one is exact as it stands.
                        let absorbs = st.env_skips.is_some();
                        last.vertical.space_after = Some(match last.vertical.space_after {
                            Some(prev) if absorbs && prev.0 >= skip.0 => prev,
                            Some(_) if absorbs => skip,
                            Some((n, s, k)) => (n + skip.0, s + skip.1, k + skip.2),
                            None => skip,
                        });
                    }
                }
            }
            st.after_heading = false;
        // A vertical-mode penalty before the paragraph (compiler
        // `Block::Penalty`: `\goodbreak`, `\pagebreak[n]`, `\nobreak`, ...)
        // sits after the previous block and before this one's `\parskip`;
        // a page-break command before it already put a forced one there.
        if let (Some((value, fil)), Some(b)) = (penalty_before, blocks.get_mut(first_built)) {
            if b.vertical.penalty_before.is_none() {
                b.vertical.penalty_before = Some(*value);
                b.vertical.fil_break = *fil;
            }
        }
        ctx.breaking = outer_breaking;
    }

    /// The vertical material of a float body: `\@xfloat` opens a `\vbox`
    /// of `\hsize\columnwidth`, runs `\@parboxrestore` and
    /// `\@floatboxreset` (`\reset@font\normalsize`), and the body is
    /// contributed to that box's vertical list like any other body text.
    /// Every block goes through the same builder the page uses, so a
    /// `tabular`, an `itemize`, a display or a paragraph of prose is set
    /// inside a float exactly as it is outside one. Returns the range of
    /// `blocks` the body added.
    pub(crate) fn box_blocks(&mut self, body: &[Block], out: &mut Vec<BuiltBlock>, float: Span, minipage: bool) -> std::ops::Range<usize> {
        // The box has a vertical list of its own: `\addvspace` compares
        // against what *it* left last (`\lastskip`), not against the page's
        // last block, so the body builds into its own vector.
        let mut owned: Vec<BuiltBlock> = Vec::new();
        let blocks = &mut owned;
        let quad = self.text_params(TextStyle::default(), self.style.body_size_pt).quad;
        // `\footnote` inside a float box: `footnotes::prepare` has already
        // run, so a note raised here would set its mark and never be placed.
        let (notes, anchors) = (self.notes.len(), self.note_anchors.len());
        let mut st = ParaState { after_heading: false, env_vmode: false, env_skips: None };
        let outer = std::mem::replace(&mut self.parbox, true);
        // `\@floatboxreset` runs `\@setminipage`, and `\addvspace` does
        // nothing while `\if@minipage` holds (latex.ltx: it is cleared by
        // the first paragraph's `\everypar`). So the `\topsep` of a list
        // that opens the float box, and an environment's `\@topsepadd`, add
        // no glue above the box's first paragraph.
        let mut minipage = minipage;
        for block in body {
            match block {
                Block::Paragraph { .. } if minipage => {
                    let mut opened = block.clone();
                    if let Block::Paragraph { addvspace_before, addvspace_flex, vspace_flex, env_open, vspace_before, list, .. } = &mut opened {
                        *addvspace_before = 0.0;
                        *addvspace_flex = (0.0, 0.0);
                        *vspace_flex = (0.0, 0.0);
                        *env_open = None;
                        // `\@item`'s `\addvspace\@topsep` and its paired
                        // `\addvspace{-\parskip}` are both suppressed.
                        if list.is_some() {
                            *vspace_before = 0.0;
                        }
                    }
                    let at = blocks.len();
                    self.build_paragraph(blocks, &opened, &mut st, None, 0, quad);
                    // TeX 1091: a paragraph that starts an empty internal
                    // vertical list adds no `\parskip` glue either.
                    if let Some(b) = blocks.get_mut(at) {
                        b.vertical.parskip = None;
                        b.vertical.space_before = None;
                    }
                    // A block that set nothing (a paragraph with no boxes)
                    // started no paragraph, so `\if@minipage` still holds.
                    minipage = blocks.len() == at;
                }
                // The cache is keyed on a block's own bytes and the page's
                // parameters, which `\@parboxrestore` has changed: a float
                // body builds uncached.
                Block::Paragraph { .. } => self.build_paragraph(blocks, block, &mut st, None, 0, quad),
                Block::Rule { span, vspace_before, .. } => {
                    let mut b = self.rule_block(*span);
                    add_vspace(&mut b.vertical, *vspace_before);
                    blocks.push(b);
                }
                Block::Picture { document, picture, centered, vspace_before, .. } => {
                    let mut b = self.picture_block(*document, picture, *centered);
                    add_vspace(&mut b.vertical, *vspace_before);
                    blocks.push(b);
                    // A picture is set in a paragraph: `\everypar` has run.
                    minipage = false;
                }
                // Page-level material: LaTeX forbids it in a float box (a
                // `\section` there would number and mark out of order, and
                // `longtable` breaks pages, which a float cannot).
                other => {
                    let what = match other {
                        Block::Heading { .. } => "a sectioning command",
                        Block::Chapter { .. } => "\\chapter",
                        Block::Part { .. } => "\\part",
                        Block::Title { .. } => "\\maketitle",
                        Block::ClearPage { .. } => "\\clearpage",
                        Block::NoBreakFalse { .. } => "a contents list",
                        Block::Chrome { .. } => "a page-style command",
                        Block::TocEntry(_) => "a contents list",
                        Block::LongTable { .. } => "longtable",
                        Block::Paragraph { .. } | Block::Rule { .. } | Block::Picture { .. } => unreachable!(),
                    };
                    let source = vec![self.source(float)];
                    self.diagnostics.push(Diagnostic::warning(
                        "float_content_unsupported",
                        format!("{what} cannot be set inside a float box; it is omitted"),
                        source,
                    ));
                }
            }
        }
        self.parbox = outer;
        if self.notes.len() > notes {
            let source = vec![self.source(float)];
            self.notes.truncate(notes);
            self.note_anchors.truncate(anchors);
            self.diagnostics.push(Diagnostic::warning(
                "float_footnote_unplaced",
                "a \\footnote inside a float body is not placed yet (LaTeX needs \\footnotemark here and \\footnotetext outside the float); the mark is set and the note text is omitted",
                source,
            ));
        }
        let first = out.len();
        out.append(blocks);
        first..out.len()
    }

    /// `\parskip` as the box being set has it: `\@parboxrestore` zeroed it
    /// for a float body, and `\list` sets it to `\parsep` inside a list.
    fn parskip_of(&self, list_geom: Option<&ListGeom>) -> crate::style::Skip {
        match list_geom {
            Some(g) => g.parsep,
            None if self.parbox => crate::style::Skip::default(),
            None => self.style.parskip,
        }
    }

    /// `(\@totalleftmargin, \labelwidth, innermost \leftmargin)` of an item
    /// paragraph, in points: the sum of the enclosing lists' `\leftmargin`s,
    /// the innermost list's label width (`\leftmargin - \labelsep` for a
    /// class margin; the widest label's own width under enumitem's
    /// `leftmargin=*`; zero for a `description`, whose `\list` sets
    /// `\labelwidth\z@`), and the innermost `\leftmargin` on its own, which
    /// is what `description`'s `\itemindent-\leftmargin` cancels.
    fn list_geometry(&mut self, geom: &ListGeom, size: f64) -> (f64, f64, f64) {
        let labelsep = self.style.labelsep_pt;
        let mut hang = 0.0;
        let mut labelwidth = 0.0;
        let mut inner = 0.0;
        let quad = self.text_params(TextStyle::default(), size).quad;
        for margin in &geom.margins {
            let (m, w) = match margin {
                ListMargin::Fixed(pt) => (*pt, (pt - labelsep).max(0.0)),
                // natbib's `\bibhang`: `1em` of the body font, and no label
                // to measure (`\@biblabel` is `\hfill`).
                ListMargin::Em(em) => (em * quad, 0.0),
                ListMargin::Widest(text) => {
                    let w = self.text_width(text, size, geom.label.as_ref().map_or(Span::new(0, 0), |(_, span)| *span));
                    (w + labelsep, w)
                }
            };
            hang += m;
            inner = m;
            labelwidth = w;
        }
        if geom.description {
            labelwidth = 0.0;
        }
        (hang, labelwidth, inner)
    }

    /// Width of `text` shaped in the body font at `size`, in points.
    fn text_width(&mut self, text: &str, size: f64, span: Span) -> f64 {
        let face = self.face(TextStyle::default(), size, span);
        let shaped = self.shaper.shape(&face, text);
        shaped.width_units as f64 * size / shaped.units_per_em as f64
    }

    /// microtype's `\leftprotrusion`, which it appends to `\@item`'s
    /// `\everypar` (microtype.sty, `\MT@patch@patch\@item{\everypar{}}
    /// {\everypar{\leftprotrusion}}`): `\MT@get@prot` sets the item text's
    /// first group alone and adds `\kern\leftmarginkern` of that line, the
    /// negated `char_pw` of its first character, after the label. pdfTeX's
    /// own margin kern cannot reach that character (`find_protchar_left`
    /// stops at the `\@labels` box's glue), so this explicit kern is the
    /// item text's only protrusion. In points; 0 without protrusion or when
    /// the text does not open with a character of a configured font.
    fn item_left_protrusion(&mut self, list: &[pl::Item], recs: &[Option<usize>]) -> f64 {
        if !self.style.microtype.as_ref().is_some_and(|m| m.protrude_chars > 0) {
            return 0.0;
        }
        let (Some(pl::Item::Box(run)), Some(Some(rec))) = (list.first(), recs.first()) else { return 0.0 };
        let Some(micro) = self.micro_run(*rec, run) else { return 0.0 };
        match micro.glyphs.first().and_then(|g| g.code) {
            Some(c) => f64::from(micro.params.left_protrusion(c)) / 65536.0,
            None => 0.0,
        }
    }

    /// The `\item` label as `\@item` boxes it, every character pointing at
    /// the `\item` command's bytes: the words of `text` in the
    /// list's label style (`\descriptionlabel`'s `\bfseries` for a
    /// `description`), separated by interword glue at natural width.
    fn label_box(&mut self, text: &str, span: Span, size: f64, bold: bool) -> Option<NumberBox> {
        let boxed = self.word_box(text, span, size, TextStyle { bold, ..TextStyle::default() });
        if let Some(nb) = &boxed {
            for (_, rec, _) in &nb.pieces {
                self.label_recs.insert(*rec);
            }
        }
        boxed
    }

    fn heading_block(&mut self, level: u8, items: &[AItem]) -> Option<BuiltBlock> {
        let h = self.style.heading(level);
        let (list, recs, labels, skips) = self.hlist(
            items,
            h.size_pt,
            TextStyle {
                bold: h.bold,
                ..TextStyle::default()
            },
            ParaStyle::Plain,
        );
        if !list.iter().any(|i| matches!(i, pl::Item::Box(_))) {
            return None;
        }
        let params = self.line_params(false, h.baselineskip_pt, ParaStyle::Plain, 0.0);
        let lines = self.break_paragraph(&list, &params, items, Some(&recs))?;
        self.report_overfull(&lines, &list, &recs);
        // The heading's lines are appended under its own \baselineskip
        // (`\Large` is in force inside \@sect's group); the before/after
        // skips are body-font `ex`.
        let before = h.before;
        // \@startsection: \addpenalty\@secpenalty, \addvspace{before},
        // the title with \interlinepenalty\@M, \nobreak, \vskip{after}.
        let vertical = VBlock {
            lines: line_extents(&lines),
            penalty_before: Some(SEC_PENALTY),
            space_before: Some(skip_tuple(before)),
            parskip: Some(skip_tuple(self.style.parskip)),
            interline_penalty: pagebuild::INF_PENALTY,
            club_penalty: 0,
            widow_penalty: 0,
            penalty_after: Some(pagebuild::INF_PENALTY),
            space_after: Some(skip_tuple(h.after)),
            no_interline_first: false,
            no_interline_after: false,
            baselineskip: Some(h.baselineskip_pt),
            vskip_after: vskips_of(&lines, &skips),
            broken_penalty: Vec::new(),
            vadjust_penalty: Vec::new(),
            fil_break: false,
            pre_space_after: None,
            lineskip: None,
            contributed: None,
            line_penalty: Vec::new(),
            depth_after: pagebuild::DepthAfter::default(),
        };
        Some(BuiltBlock {
            block: pl::ParagraphBlock {
                lines,
                space_before: before.glue(),
                space_after: h.after.glue(),
                keep_with_next: true,
            },
            items: list,
            recs,
            vertical,
            labels,
            cache_key: None,
        })
    }

    /// The line TeX sets when a display opens a paragraph: the
    /// `\parindent` box alone (`$$`, `equation`), or LaTeX's
    /// `\nointerlineskip\makebox[.6\linewidth]{}` for `\[`. It carries
    /// `\parskip` and decides the display's `pre_display_size` (its
    /// width plus 2em, `\parshape` indent included).
    ///
    /// Inside a list (`list_geom`) the line starts `\@totalleftmargin` in
    /// and `\parskip` is `\parsep`. When it opens an `\item` (`\item \[`),
    /// `\@item`'s `\everypar` removes the indent box (`\lastbox`) and sets
    /// the label instead, so the display follows a line holding only the
    /// label — the label on a line of its own, with the label's height and
    /// depth.
    fn display_opener_block(&mut self, bracket: bool, list_geom: Option<&ListGeom>) -> (BuiltBlock, f64) {
        let s = self.style;
        let size = s.body_size_pt;
        let (hang, labelwidth, inner) = list_geom.map_or((0.0, 0.0, 0.0), |g| self.list_geometry(g, size));
        let description = list_geom.is_some_and(|g| g.description);
        let linewidth = s.text_width_pt - hang;
        let label = list_geom
            .and_then(|g| g.label.as_ref().map(|l| (l, g.description)))
            .and_then(|((text, span), bold)| self.label_box(text, *span, size, bold));
        let mut width = hang + if bracket { 0.6 * linewidth } else { 0.0 };
        let (mut runs, mut items, mut recs) = (Vec::new(), Vec::new(), Vec::new());
        let (mut height, mut depth) = (0.0, 0.0);
        match label {
            Some(nb) => {
                // `\hskip-\labelwidth \hskip-\labelsep \hbox to\labelwidth
                // {\hss <label>} \hskip\labelsep`: the label's right edge
                // ends `\labelsep` before the text edge.
                //
                // `description` instead has `\labelwidth\z@
                // \itemindent-\leftmargin`, and `\descriptionlabel`'s own
                // leading `\hspace\labelsep` cancels the `\hskip-\labelsep`,
                // so the term sets flush at `\leftmargin + \itemindent` —
                // the same rule [`Self::paragraph_block`] applies when the
                // item opens with text rather than a display.
                let x0 = if description {
                    // `\itemindent-\leftmargin`: `inner` is the innermost
                    // list's `\leftmargin`, so this is `hang + \itemindent`.
                    hang - inner
                } else {
                    hang - s.labelsep_pt - nb.width.min(labelwidth)
                };
                height = nb.height;
                depth = nb.depth;
                for (run, rec, dx) in nb.pieces {
                    runs.push(position_run(&run, x0 + dx, nb.height));
                    items.push(pl::Item::Box(run));
                    recs.push(Some(rec));
                }
            }
            // `\@parboxrestore` has zeroed `\parindent`, so the box a
            // display opens a paragraph with is empty in a float body.
            None => width += if self.parbox { 0.0 } else { s.parindent_pt },
        }
        let n = items.len();
        let line = pl::Line {
            index: 0,
            runs,
            baseline_y: height,
            height,
            depth,
            natural_width: width,
            set_width: s.text_width_pt,
            ratio: 0.0,
            badness: 0.0,
            items: 0..n,
            hyphenated: false,
        };
        let lines = pl::Lines {
            lines: vec![line],
            breaks: Vec::new(),
            stats: pl::Stats {
                algorithm: pl::Algorithm::TotalFit,
                lines: 1,
                pass: 1,
                total_demerits: 0.0,
                overfull: Vec::new(),
                underfull: Vec::new(),
                hyphenated_lines: 0,
                emergency_pass_used: false,
            },
            diagnostics: Vec::new(),
            height: height + depth,
        };
        let quad = self.text_params(TextStyle::default(), size).quad;
        let parskip = self.parskip_of(list_geom);
        let vertical = VBlock {
            lines: vec![(height, depth)],
            penalty_before: None,
            space_before: None,
            parskip: Some(skip_tuple(parskip)),
            interline_penalty: 0,
            club_penalty: 0,
            widow_penalty: 0,
            penalty_after: None,
            space_after: None,
            no_interline_first: bracket,
            no_interline_after: false,
            baselineskip: None,
            vskip_after: Vec::new(),
            broken_penalty: Vec::new(),
            vadjust_penalty: Vec::new(),
            fil_break: false,
            pre_space_after: None,
            lineskip: None,
            contributed: None,
            line_penalty: Vec::new(),
            depth_after: pagebuild::DepthAfter::default(),
        };
        (
            BuiltBlock {
                block: pl::ParagraphBlock::body(lines),
                items,
                recs,
                vertical,
                labels: Vec::new(),
                cache_key: None,
            },
            width + 2.0 * quad,
        )
    }

    /// `\hrule` in vertical mode (TeX §1056): a rule node of the full
    /// measure, `0.4pt` high and `0pt` deep, appended with no interline glue
    /// before it and `prev_depth` left at `ignore_depth` after it. Set as a
    /// one-line block whose single box is a [`BoxRec::Rule`].
    fn rule_block(&mut self, span: Span) -> BuiltBlock {
        const HRULE_HEIGHT: f64 = 0.4;
        self.rule_block_sized(span, self.style.text_width_pt, HRULE_HEIGHT, 0.0)
    }

    /// A rule `width` x `height` whose bottom sits on the line's baseline,
    /// `x` from the line's left edge.
    fn rule_block_sized(&mut self, span: Span, width: f64, height: f64, x: f64) -> BuiltBlock {
        self.recs.push(BoxRec::Rule { width, height, bottom: 0.0, span });
        let rec = self.recs.len() - 1;
        let run = pl::GlyphRun {
            font: MATH_SENTINEL,
            size: self.style.body_size_pt,
            glyphs: Vec::new(),
            width,
            height,
            depth: 0.0,
            source: span.start..span.end,
        };
        let line = pl::Line {
            index: 0,
            runs: vec![position_run(&run, x, height)],
            baseline_y: height,
            height,
            depth: 0.0,
            natural_width: width,
            set_width: width,
            ratio: 0.0,
            badness: 0.0,
            items: 0..1,
            hyphenated: false,
        };
        let lines = pl::Lines {
            lines: vec![line],
            breaks: Vec::new(),
            stats: one_line_stats(),
            diagnostics: Vec::new(),
            height,
        };
        let vertical = VBlock {
            lines: vec![(height, 0.0)],
            penalty_before: None,
            space_before: None,
            parskip: None,
            interline_penalty: 0,
            club_penalty: 0,
            widow_penalty: 0,
            penalty_after: None,
            space_after: None,
            no_interline_first: true,
            no_interline_after: true,
            baselineskip: None,
            vskip_after: Vec::new(),
            broken_penalty: Vec::new(),
            vadjust_penalty: Vec::new(),
            fil_break: false,
            pre_space_after: None,
            lineskip: None,
            contributed: None,
            line_penalty: Vec::new(),
            depth_after: pagebuild::DepthAfter::default(),
        };
        BuiltBlock {
            block: pl::ParagraphBlock::body(lines),
            items: vec![pl::Item::Box(run)],
            recs: vec![Some(rec)],
            vertical,
            labels: Vec::new(),
            cache_key: None,
        }
    }

    /// `\@makechapterhead` / `\@makeschapterhead` (report.cls, book.cls)
    /// after `\clearpage`: `\vspace*{50\p@}` (a zero-height rule kept at the
    /// page top, its baseline at `\topskip`, then `\nobreak` and the skip),
    /// `\huge\bfseries Chapter <n>\par\nobreak\vskip 20\p@` (numbered only),
    /// `\Huge\bfseries <title>\par\nobreak\vskip 40\p@`, both `\raggedright`
    /// at their own `\baselineskip`.
    fn chapter_blocks(&mut self, number: Option<&str>, title: &[AItem], span: Span, spec: &flashtex_class_geometry::ChapterSpec, base: flashtex_class_geometry::BaseSize) -> Vec<BuiltBlock> {
        use crate::style::frame_pt;
        let empty = pl::Lines {
            lines: vec![pl::Line {
                index: 0,
                runs: Vec::new(),
                baseline_y: 0.0,
                height: 0.0,
                depth: 0.0,
                natural_width: 0.0,
                set_width: 0.0,
                ratio: 0.0,
                badness: 0.0,
                items: 0..0,
                hyphenated: false,
            }],
            breaks: Vec::new(),
            stats: one_line_stats(),
            diagnostics: Vec::new(),
            height: 0.0,
        };
        let mut out = vec![BuiltBlock {
            block: pl::ParagraphBlock::body(empty),
            items: Vec::new(),
            recs: Vec::new(),
            vertical: VBlock {
                lines: vec![(0.0, 0.0)],
                penalty_before: Some(pagebuild::EJECT_PENALTY),
                space_before: None,
                parskip: None,
                interline_penalty: 0,
                club_penalty: 0,
                widow_penalty: 0,
                penalty_after: Some(pagebuild::INF_PENALTY),
                space_after: Some((frame_pt(spec.top_space), 0.0, 0.0)),
                no_interline_first: true,
                no_interline_after: false,
                baselineskip: None,
                vskip_after: Vec::new(),
                broken_penalty: Vec::new(),
                vadjust_penalty: Vec::new(),
                fil_break: false,
                pre_space_after: None,
                lineskip: None,
                contributed: None,
                line_penalty: Vec::new(),
                depth_after: pagebuild::DepthAfter::default(),
            },
            labels: Vec::new(),
            cache_key: None,
        }];
        let metrics = |size: flashtex_class_geometry::FontSize| {
            let (s, b) = size.metrics(base);
            (frame_pt(s), frame_pt(b))
        };
        if let Some(n) = number {
            let (size, bs) = metrics(spec.number_size);
            let words = adapter::command_words(&format!("{} {n}", spec.prefix), span);
            out.extend(self.chapter_line(&words, size, bs, frame_pt(spec.number_title_skip)));
        }
        let (size, bs) = metrics(spec.title_size);
        out.extend(self.chapter_line(title, size, bs, frame_pt(spec.after_title)));
        out
    }

    /// One bold `\raggedright` chapter-head paragraph at `size_pt` with its
    /// `\baselineskip`, `\nobreak` and `\vskip after_pt` after it.
    fn chapter_line(&mut self, items: &[AItem], size_pt: f64, baselineskip_pt: f64, after_pt: f64) -> Option<BuiltBlock> {
        self.part_line(items, size_pt, baselineskip_pt, after_pt, ParaStyle::FlushLeft, None)
    }

    /// article.cls `\@part`/`\@spart` (lines 275-301): `\addvspace{4ex}`,
    /// `{\parindent\z@ \raggedright \interlinepenalty\@M \Large\bfseries
    /// \partname\nobreakspace\thepart \par\nobreak \huge\bfseries #2\par}`,
    /// `\nobreak \vskip 3ex`.
    fn part_flow_blocks(&mut self, number: Option<&str>, title: &[AItem], span: Span, spec: &flashtex_class_geometry::PartSpec, base: flashtex_class_geometry::BaseSize) -> Vec<BuiltBlock> {
        use crate::style::frame_pt;
        let metrics = |size: flashtex_class_geometry::FontSize| {
            let (s, b) = size.metrics(base);
            (frame_pt(s), frame_pt(b))
        };
        let mut out = Vec::new();
        if let Some(n) = number {
            let (size, bs) = metrics(spec.number_size);
            let words = adapter::command_words(&format!("Part {n}"), span);
            out.extend(self.part_line(&words, size, bs, frame_pt(spec.number_title_skip), ParaStyle::FlushLeft, None));
        }
        let (size, bs) = metrics(spec.title_size);
        out.extend(self.part_line(title, size, bs, frame_pt(spec.space_after), ParaStyle::FlushLeft, None));
        if let Some(first) = out.first_mut() {
            first.vertical.space_before = Some((frame_pt(spec.space_before), 0.0, 0.0));
        }
        out
    }

    /// report.cls `\part` (lines 278-328; book.cls 299-349): the page break,
    /// `\null\vfil`, `{\centering \huge\bfseries \partname\nobreakspace
    /// \thepart \par \vskip 20\p@ \Huge\bfseries #2\par}`, `\@endpart`'s
    /// `\vfil\newpage` (and `\newpage`'s own `\vfil`): the three `\vfil`s
    /// share what the page leaves, one before the title and two after.
    fn part_page_blocks(&mut self, number: Option<&str>, title: &[AItem], span: Span, spec: &flashtex_class_geometry::PartSpec, base: flashtex_class_geometry::BaseSize, width: f64) -> Vec<BuiltBlock> {
        use crate::style::frame_pt;
        let metrics = |size: flashtex_class_geometry::FontSize| {
            let (s, b) = size.metrics(base);
            (frame_pt(s), frame_pt(b))
        };
        let mut null = plain_vblock(vec![(0.0, 0.0)]);
        null.penalty_before = Some(pagebuild::EJECT_PENALTY);
        null.space_after = Some((0.0, 0.0, 0.0));
        let mut out = vec![empty_block(null)];
        if let Some(n) = number {
            let (size, bs) = metrics(spec.number_size);
            let words = adapter::command_words(&format!("Part {n}"), span);
            out.extend(self.part_line(&words, size, bs, frame_pt(spec.number_title_skip), ParaStyle::Center, Some(width)));
        }
        let (size, bs) = metrics(spec.title_size);
        out.extend(self.part_line(title, size, bs, 0.0, ParaStyle::Center, Some(width)));
        let last = out.len() - 1;
        out[last].vertical.penalty_after = Some(pagebuild::EJECT_PENALTY);
        let s = self.style;
        let p = page_params(s);
        let vb: Vec<VBlock> = out.iter().map(|b| b.vertical.clone()).collect();
        let (_, natural) = pagebuild::natural_layout(&p, &pagebuild::vlist(&p, &vb), true);
        let fils = 3.0 + if s.raggedbottom { 1e-4 } else { 0.0 };
        let fil = ((s.text_height_pt - natural) / fils).max(0.0);
        if let Some(sa) = out[0].vertical.space_after.as_mut() {
            sa.0 += fil;
        }
        out
    }

    /// One bold heading paragraph (`\raggedright` or `\centering`) at
    /// `size_pt` with its `\baselineskip`, `\nobreak` and `\vskip after_pt`
    /// after it, `width` wide (the column by default).
    fn part_line(&mut self, items: &[AItem], size_pt: f64, baselineskip_pt: f64, after_pt: f64, para: ParaStyle, width: Option<f64>) -> Option<BuiltBlock> {
        let (list, recs, labels, skips) = self.hlist(
            items,
            size_pt,
            TextStyle {
                bold: true,
                ..TextStyle::default()
            },
            para,
        );
        if !list.iter().any(|i| matches!(i, pl::Item::Box(_))) {
            return None;
        }
        let mut params = self.line_params(false, baselineskip_pt, para, 0.0);
        if let Some(w) = width {
            params.line_width = w;
        }
        let lines = self.break_paragraph(&list, &params, items, Some(&recs))?;
        let vertical = VBlock {
            lines: line_extents(&lines),
            penalty_before: None,
            space_before: None,
            parskip: Some(skip_tuple(self.style.parskip)),
            interline_penalty: pagebuild::INF_PENALTY,
            club_penalty: 0,
            widow_penalty: 0,
            penalty_after: Some(pagebuild::INF_PENALTY),
            space_after: Some((after_pt, 0.0, 0.0)),
            no_interline_first: false,
            no_interline_after: false,
            baselineskip: Some(baselineskip_pt),
            vskip_after: vskips_of(&lines, &skips),
            broken_penalty: Vec::new(),
            vadjust_penalty: Vec::new(),
            fil_break: false,
            pre_space_after: None,
            lineskip: None,
            contributed: None,
            line_penalty: Vec::new(),
            depth_after: pagebuild::DepthAfter::default(),
        };
        Some(BuiltBlock {
            block: pl::ParagraphBlock::body(lines),
            items: list,
            recs,
            vertical,
            labels,
            cache_key: None,
        })
    }

    /// `\maketitle`'s vertical material, transcribed from article.cls
    /// (report.cls and book.cls are identical here):
    ///
    /// - `\@maketitle` (lines 236-251): `\newpage \null \vskip 2em`, then
    ///   `center` (`\trivlist`: its `\@item` adds `\addvspace{\@topsep}`,
    ///   nothing after the larger `2em`), `{\LARGE \@title \par}`,
    ///   `\vskip 1.5em`, `{\large \lineskip .5em \begin{tabular}[t]{c}
    ///   \@author \end{tabular}\par}` (`\and` is `\end{tabular}\hskip 1em
    ///   \@plus.17fil\begin{tabular}[t]{c}`, latex.ltx), `\vskip 1em`,
    ///   `{\large \@date}` (the paragraph ends at `\end{center}`, so at the
    ///   `\normalsize` `\baselineskip`), `\@endparenv`'s
    ///   `\addvspace{\@topsepadd}` and `\vskip 1.5em`.
    /// - `titlepage` (lines 170-189): `\null\vfil\vskip 60\p@`, the title,
    ///   `\vskip 3em`, authors with `\lineskip .75em`, `\vskip 1.5em`,
    ///   `{\large \@date \par}`, `\vfil\null` and `\newpage`'s `\vfil`: the
    ///   three `\vfil`s share what the page leaves (a fourth of `.0001fil`
    ///   under `\raggedbottom`'s `\@textbottom`).
    /// - `Float`: the `\@maketitle` box of `\twocolumn[...]`, its first
    ///   box at the box top (no `\topskip`), `\textwidth` wide.
    ///
    /// Each `tabular` row is `\@arstrut` (`.7`/`.3\baselineskip` of
    /// `\large`) plus the row's natural width, centred in the column,
    /// `\tabcolsep` on both sides; rows abut (`\baselineskip\z@
    /// \lineskip\z@`). The author line breaks only between `tabular`s.
    fn title_blocks(&mut self, title: &[AItem], authors: &[Vec<Vec<AItem>>], date: Option<&[AItem]>, g: &flashtex_class_geometry::ResolvedDocument, form: TitleForm, columns: usize) -> Vec<BuiltBlock> {
        // `\maketitle` sets `\@makefnmark` to `\rlap{\@textsuperscript
        // {\normalfont\@thefnmark}}` and issues `\@thanks` (the
        // `\footnotetext`s of `\thanks`) after `\@maketitle` in vertical
        // mode: the notes' inserts follow the title's last line.
        let before = self.note_anchors.len();
        self.rlap_marks = true;
        let out = self.title_blocks_set(title, authors, date, g, form, columns);
        self.rlap_marks = false;
        let last = out.iter().rev().find_map(|b| b.block.lines.lines.last().and_then(|l| b.recs.get(l.items.clone()).and_then(|r| r.iter().rev().find_map(|r| *r))));
        if let Some(rec) = last {
            for anchor in &mut self.note_anchors[before..] {
                anchor.0 = rec;
            }
        }
        out
    }

    fn title_blocks_set(&mut self, title: &[AItem], authors: &[Vec<Vec<AItem>>], date: Option<&[AItem]>, g: &flashtex_class_geometry::ResolvedDocument, form: TitleForm, columns: usize) -> Vec<BuiltBlock> {
        use crate::style::frame_pt;
        use flashtex_class_geometry::FontSize;
        let s = self.style;
        let metrics = |size: FontSize| {
            let (a, b) = size.metrics(g.options.size);
            (frame_pt(a), frame_pt(b))
        };
        let (title_size, title_bs) = metrics(FontSize::LARGE);
        let (large_size, large_bs) = metrics(FontSize::Large);
        let width = frame_pt(g.frame.text_width);
        let normal_quad = self.text_params(TextStyle::default(), s.body_size_pt).quad;
        let large_quad = self.text_params(TextStyle::default(), large_size).quad;
        let parskip = skip_tuple(s.parskip);
        let topsepadd = (s.topsep.natural + s.partopsep.natural, s.topsep.stretch + s.partopsep.stretch, s.topsep.shrink + s.partopsep.shrink);
        let page = form == TitleForm::Page;
        let (before, after_title, author_lineskip, after_authors, date_bs, date_lineskip) = if page {
            (60.0, 3.0 * normal_quad, 0.75 * large_quad, 1.5 * normal_quad, large_bs, 0.75 * large_quad)
        } else {
            (2.0 * normal_quad, 1.5 * normal_quad, 0.5 * large_quad, normal_quad, s.baselineskip_pt, s.lineskip_pt)
        };
        // TeX §679 with the `\lineskip` in force.
        let interline = |bs: f64, prev: f64, h: f64, lineskip: f64| {
            let glue = bs - prev - h;
            if glue < s.lineskiplimit_pt {
                lineskip
            } else {
                glue
            }
        };
        let mut out: Vec<BuiltBlock> = Vec::new();
        // `\newpage \null`, then `\vskip 2em` (`\vfil\vskip 60\p@`) and
        // `center`'s `\addvspace{\@topsep}` (`\@topsepadd` + `\parskip`).
        let center_top = (topsepadd.0 + parskip.0, topsepadd.1 + parskip.1, topsepadd.2 + parskip.2);
        let mut null = plain_vblock(vec![(0.0, 0.0)]);
        if form != TitleForm::Float {
            null.penalty_before = Some(pagebuild::EJECT_PENALTY);
        }
        null.space_after = Some(if before < center_top.0 { center_top } else { (before, 0.0, 0.0) });
        out.push(empty_block(null));
        // `{\LARGE \@title \par}`.
        let mut prev_depth = 0.0;
        match self.title_par(title, title_size, title_bs, width) {
            Some(mut b) => {
                b.vertical.space_after = Some((after_title, 0.0, 0.0));
                prev_depth = b.block.lines.lines.last().map_or(0.0, |l| l.depth);
                out.push(b);
            }
            None => {
                if let Some(sa) = out[0].vertical.space_after.as_mut() {
                    sa.0 += after_title;
                }
            }
        }
        // The authors: one `tabular` each.
        struct Tab {
            cells: Vec<(Vec<(pl::GlyphRun, usize, f64)>, f64)>,
            row_h: Vec<f64>,
            row_d: Vec<f64>,
            offsets: Vec<f64>,
            column: f64,
        }
        let strut = (0.7 * large_bs, 0.3 * large_bs);
        let mut tabs: Vec<Tab> = Vec::with_capacity(authors.len());
        for rows in authors {
            let mut tab = Tab {
                cells: Vec::new(),
                row_h: Vec::new(),
                row_d: Vec::new(),
                offsets: Vec::new(),
                column: 0.0,
            };
            let mut off = 0.0;
            for (k, row) in rows.iter().enumerate() {
                let (runs, w) = self.hbox_runs(row, large_size);
                let h = runs.iter().map(|r| r.0.height).fold(strut.0, f64::max);
                let d = runs.iter().map(|r| r.0.depth).fold(strut.1, f64::max);
                if k > 0 {
                    off += tab.row_d[k - 1] + h;
                }
                tab.offsets.push(off);
                tab.row_h.push(h);
                tab.row_d.push(d);
                tab.column = tab.column.max(w);
                tab.cells.push((runs, w));
            }
            if !tab.cells.is_empty() {
                tabs.push(tab);
            }
        }
        // `\@maketitle` sets the author `tabular` unconditionally
        // (`{\large \lineskip .5em \begin{tabular}[t]{c}\@author
        // \end{tabular}\par}`), so `\author{}` -- and no `\author` at all,
        // which only adds a warning -- still contributes a line to the
        // centred paragraph: a `tabular` with no rows, `\hbox(0.0+0.0)`.
        // It carries no ink but it does carry its own interline glue, and
        // dropping it took `\baselineskip` less the title's depth out of the
        // title block -- 10.63972 pt at an 11pt base, which is what
        // `fixtures/real-world/math-sheet` (`\author{}`) was missing
        // (`tests/maketitle_empty_author.rs`).
        if tabs.is_empty() {
            tabs.push(Tab {
                cells: vec![(Vec::new(), 0.0)],
                row_h: vec![0.0],
                row_d: vec![0.0],
                offsets: vec![0.0],
                column: 0.0,
            });
        }
        let tab_width = |t: &Tab| t.column + 2.0 * TABCOLSEP_PT;
        let tab_height = |t: &Tab| t.row_h.first().copied().unwrap_or(0.0);
        let tab_depth = |t: &Tab| t.offsets.last().copied().unwrap_or(0.0) + t.row_d.last().copied().unwrap_or(0.0);
        // Line breaks between `tabular`s: every line with natural width
        // within `\textwidth` has badness 0 (fil glue), so the fewest lines.
        let mut para_lines: Vec<Vec<usize>> = Vec::new();
        let mut line_w = 0.0;
        for (i, t) in tabs.iter().enumerate() {
            match para_lines.last_mut() {
                Some(line) if line_w + large_quad + tab_width(t) <= width + 1e-9 => {
                    line.push(i);
                    line_w += large_quad + tab_width(t);
                }
                _ => {
                    para_lines.push(vec![i]);
                    line_w = tab_width(t);
                }
            }
        }
        // Rows by their offset below the first line's baseline.
        let mut rows: Vec<(f64, f64, f64, Vec<(pl::GlyphRun, usize, f64)>)> = Vec::new();
        let (mut line_off, mut line_depth, mut first_height) = (0.0, 0.0, 0.0);
        for (li, line) in para_lines.iter().enumerate() {
            let h = line.iter().map(|&i| tab_height(&tabs[i])).fold(0.0, f64::max);
            let d = line.iter().map(|&i| tab_depth(&tabs[i])).fold(0.0, f64::max);
            if li == 0 {
                first_height = h;
            } else {
                line_off += line_depth + interline(large_bs, line_depth, h, author_lineskip) + h;
            }
            // `\centering`'s `\leftskip`/`\rightskip` (1fil each) and the
            // `\and` glue (`.17fil` each) share the line's shortfall.
            let natural: f64 = line.iter().map(|&i| tab_width(&tabs[i])).sum::<f64>() + large_quad * (line.len() - 1) as f64;
            let per_fil = ((width - natural) / (2.0 + 0.17 * (line.len() - 1) as f64)).max(0.0);
            let mut x = per_fil;
            for &i in line {
                let column = tabs[i].column;
                let total = tab_width(&tabs[i]);
                let cells = std::mem::take(&mut tabs[i].cells);
                for (k, (runs, w)) in cells.into_iter().enumerate() {
                    let off = line_off + tabs[i].offsets[k];
                    let dx = x + TABCOLSEP_PT + (column - w) / 2.0;
                    let at = match rows.iter().position(|r| (r.0 - off).abs() < 1e-6) {
                        Some(at) => at,
                        None => {
                            rows.push((off, 0.0, 0.0, Vec::new()));
                            rows.len() - 1
                        }
                    };
                    rows[at].1 = rows[at].1.max(tabs[i].row_h[k]);
                    rows[at].2 = rows[at].2.max(tabs[i].row_d[k]);
                    rows[at].3.extend(runs.into_iter().map(|(r, rec, rx)| (r, rec, dx + rx)));
                }
                x += total + large_quad + 0.17 * per_fil;
            }
            line_depth = d;
        }
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        let bottom = line_off + line_depth;
        let n_rows = rows.len();
        let mut last_depth = prev_depth;
        let mut prev_off = 0.0;
        for (k, (off, h, d, runs)) in rows.into_iter().enumerate() {
            // The `tabular`s' rows below the first are boxes of no height
            // at their own offsets (`\nobreak` between them); the last one
            // carries the depth left below it.
            let depth_left = if k + 1 == n_rows { bottom - off } else { 0.0 };
            let mut v = plain_vblock(vec![(if k == 0 { first_height } else { 0.0 }, depth_left)]);
            if k == 0 {
                v.no_interline_first = true;
                v.space_before = Some((interline(large_bs, prev_depth, first_height, author_lineskip), 0.0, 0.0));
                v.parskip = Some(parskip);
            } else {
                v.penalty_before = Some(pagebuild::INF_PENALTY);
                v.baselineskip = Some(off - prev_off);
            }
            prev_off = off;
            last_depth = depth_left;
            out.push(positioned_block(runs, h, d, width, v));
        }
        // The date, then `\@endparenv`'s `\addvspace{\@topsepadd}`: after a
        // paragraph a plain skip; after `\vskip` (no date) it replaces the
        // skip only when larger.
        match date.and_then(|d| self.title_par(d, large_size, date_bs, width)) {
            Some(mut b) => {
                let h = b.block.lines.lines.first().map_or(0.0, |l| l.height);
                b.vertical.no_interline_first = true;
                b.vertical.space_before = Some((after_authors + interline(date_bs, last_depth, h, date_lineskip), 0.0, 0.0));
                b.vertical.pre_space_after = Some(topsepadd);
                out.push(b);
            }
            None => {
                let last = out.last_mut().expect("the \\null block");
                last.vertical.pre_space_after = Some(if after_authors < topsepadd.0 { topsepadd } else { (after_authors, 0.0, 0.0) });
            }
        }
        let last = out.last_mut().expect("the \\null block");
        if !page {
            last.vertical.space_after = Some((1.5 * normal_quad, 0.0, 0.0));
            return out;
        }
        // `\vfil\null` and `\newpage`; in two-column mode `\onecolumn` made
        // the page one column, so the second column is empty too.
        last.vertical.space_after = Some((0.0, 0.0, 0.0));
        let last_material = out.len() - 1;
        let mut null2 = plain_vblock(vec![(0.0, 0.0)]);
        null2.penalty_after = Some(pagebuild::EJECT_PENALTY);
        out.push(empty_block(null2.clone()));
        let p = page_params(s);
        let vb: Vec<VBlock> = out.iter().map(|b| b.vertical.clone()).collect();
        let (_, natural) = pagebuild::natural_layout(&p, &pagebuild::vlist(&p, &vb), true);
        let fils = 3.0 + if s.raggedbottom { 1e-4 } else { 0.0 };
        let fil = ((s.text_height_pt - natural) / fils).max(0.0);
        for at in [0, last_material] {
            if let Some(sa) = out[at].vertical.space_after.as_mut() {
                sa.0 += fil;
            }
        }
        if columns > 1 {
            out.push(empty_block(null2));
        }
        out
    }

    /// A centred `\maketitle` paragraph (`center`: `\centering`) at
    /// `size_pt` with its `\baselineskip`, `width` wide.
    fn title_par(&mut self, items: &[AItem], size_pt: f64, baselineskip_pt: f64, width: f64) -> Option<BuiltBlock> {
        let (mut list, mut recs, labels, mut skips) = self.hlist(items, size_pt, TextStyle::default(), ParaStyle::Center);
        if !list.iter().any(|i| matches!(i, pl::Item::Box(_))) {
            return None;
        }
        let _ = drop_trailing_break(&mut list, &mut recs, &mut skips, ParaStyle::Center);
        let mut params = self.line_params(false, baselineskip_pt, ParaStyle::Center, 0.0);
        params.line_width = width;
        let lines = self.break_paragraph(&list, &params, items, Some(&recs))?;
        self.report_overfull(&lines, &list, &recs);
        let mut vertical = plain_vblock(line_extents(&lines));
        vertical.parskip = Some(skip_tuple(self.style.parskip));
        vertical.club_penalty = CLUB_PENALTY;
        vertical.widow_penalty = WIDOW_PENALTY;
        vertical.baselineskip = Some(baselineskip_pt);
        vertical.vskip_after = vskips_of(&lines, &skips);
        Some(BuiltBlock {
            block: pl::ParagraphBlock::body(lines),
            items: list,
            recs,
            vertical,
            labels,
            cache_key: None,
        })
    }

    /// `items` as an `\hbox` at natural width: the runs that carry a
    /// record, each with its x, and the box width.
    fn hbox_runs(&mut self, items: &[AItem], size_pt: f64) -> (Vec<(pl::GlyphRun, usize, f64)>, f64) {
        let (list, recs, _, _) = self.hlist(items, size_pt, TextStyle::default(), ParaStyle::FlushLeft);
        let mut x = 0.0;
        let mut out = Vec::new();
        for (item, rec) in list.into_iter().zip(recs) {
            match item {
                pl::Item::Box(run) => {
                    let advance = run.width;
                    if let Some(rec) = rec {
                        out.push((run, rec, x));
                    }
                    x += advance;
                }
                pl::Item::Glue(glue) => x += glue.width,
                pl::Item::Kern(kern) => x += kern.width,
                pl::Item::Penalty(_) => {}
            }
        }
        (out, x)
    }

    /// `\colorbox`/`\fcolorbox` (xcolor.sty 3.02 `\color@b@x`): the content
    /// as an `\hbox` at natural width with `\fboxsep` on both sides and its
    /// height and depth grown by `\fboxsep`; `\fcolorbox` adds a `\fboxrule`
    /// frame around that (`\XC@frameb@x`).
    fn color_box(&mut self, cb: &adapter::ColorBoxItem, size: f64) -> (pl::GlyphRun, usize) {
        let (placed, content_width) = self.hbox_runs(&cb.items, size);
        let rule = if cb.frame.is_some() { cb.rule_pt } else { 0.0 };
        let inset = rule + cb.sep_pt;
        let (mut ht, mut dp) = (0.0f64, 0.0f64);
        let mut runs = Vec::with_capacity(placed.len());
        let mut items = Vec::with_capacity(placed.len());
        let mut recs = Vec::with_capacity(placed.len());
        for (run, rec, x) in placed {
            ht = ht.max(run.height);
            dp = dp.max(run.depth);
            runs.push(position_run(&run, inset + x, 0.0));
            items.push(pl::Item::Box(run));
            recs.push(Some(rec));
        }
        let width = content_width + 2.0 * inset;
        let n = items.len();
        let lines = pl::Lines {
            lines: vec![pl::Line {
                index: 0,
                runs,
                baseline_y: ht,
                height: ht,
                depth: dp,
                natural_width: width,
                set_width: width,
                ratio: 0.0,
                badness: 0.0,
                items: 0..n,
                hyphenated: false,
            }],
            breaks: Vec::new(),
            stats: one_line_stats(),
            diagnostics: Vec::new(),
            height: ht + dp,
        };
        let block = BuiltBlock {
            block: pl::ParagraphBlock::body(lines),
            items,
            recs,
            vertical: VBlock {
                lines: vec![(ht, dp)],
                penalty_before: None,
                space_before: None,
                parskip: None,
                interline_penalty: 0,
                club_penalty: 0,
                widow_penalty: 0,
                penalty_after: None,
                space_after: None,
                no_interline_first: true,
                no_interline_after: true,
                baselineskip: None,
                vskip_after: Vec::new(),
                broken_penalty: Vec::new(),
                vadjust_penalty: Vec::new(),
                fil_break: false,
                pre_space_after: None,
                lineskip: None,
                contributed: None,
                line_penalty: Vec::new(),
                depth_after: pagebuild::DepthAfter::default(),
            },
            labels: Vec::new(),
            cache_key: None,
        };
        let (height, depth) = (ht + inset, dp + inset);
        self.recs.push(BoxRec::ColorBox(Rc::new(ColorBoxRec { block, width, height, depth, rule, fill: cb.fill, frame: cb.frame, span: cb.span })));
        let run = pl::GlyphRun { font: MATH_SENTINEL, size, glyphs: Vec::new(), width, height, depth, source: cb.span.start..cb.span.end };
        (run, self.recs.len() - 1)
    }

    /// ulem `\uline`/`\sout` or kernel `\underline`: content as an `\hbox`,
    /// rule placed by `ul.geom`. `\uline` keeps the 0.25em-top / 0.4pt path.
    fn underline_box(&mut self, ul: &adapter::UnderlineItem, size: f64) -> (pl::GlyphRun, usize) {
        let (placed, width) = self.hbox_runs(&ul.items, size);
        let (mut ht, mut dp) = (0.0f64, 0.0f64);
        let mut runs = Vec::with_capacity(placed.len());
        let mut items = Vec::with_capacity(placed.len());
        let mut recs = Vec::with_capacity(placed.len());
        for (run, rec, x) in placed {
            ht = ht.max(run.height);
            dp = dp.max(run.depth);
            runs.push(position_run(&run, x, 0.0));
            items.push(pl::Item::Box(run));
            recs.push(Some(rec));
        }
        let n = items.len();
        let lines = pl::Lines {
            lines: vec![pl::Line {
                index: 0,
                runs,
                baseline_y: ht,
                height: ht,
                depth: dp,
                natural_width: width,
                set_width: width,
                ratio: 0.0,
                badness: 0.0,
                items: 0..n,
                hyphenated: false,
            }],
            breaks: Vec::new(),
            stats: one_line_stats(),
            diagnostics: Vec::new(),
            height: ht + dp,
        };
        let block = BuiltBlock {
            block: pl::ParagraphBlock::body(lines),
            items,
            recs,
            vertical: VBlock {
                lines: vec![(ht, dp)],
                penalty_before: None,
                space_before: None,
                parskip: None,
                interline_penalty: 0,
                club_penalty: 0,
                widow_penalty: 0,
                penalty_after: None,
                space_after: None,
                no_interline_first: true,
                no_interline_after: true,
                baselineskip: None,
                vskip_after: Vec::new(),
                pre_space_after: None,
                lineskip: None,
                contributed: None,
                line_penalty: Vec::new(),
                // TeX §890 `\brokenpenalty` follows a discretionary-broken
                // line; the underlined fragment is one unbreakable line
                // (`hyphenated: false`), so there is never one to follow.
                broken_penalty: Vec::new(),
                vadjust_penalty: Vec::new(),
                fil_break: false,
                depth_after: pagebuild::DepthAfter::default(),
            },
            labels: Vec::new(),
            cache_key: None,
        };
        let descender = self.uline_depth(size);
        let ex = self.text_params(TextStyle::default(), size).x_height;
        let (top, extra_depth) =
            ul.geom
                .rule_top_and_depth(ul.thickness_pt, dp, descender, ex);
        let depth = dp.max(extra_depth);
        self.recs.push(BoxRec::Underline(Rc::new(UnderlineRec {
            block,
            width,
            height: ht,
            depth,
            thickness: ul.thickness_pt,
            ul_depth: top,
            span: ul.span,
        })));
        let run = pl::GlyphRun { font: MATH_SENTINEL, size, glyphs: Vec::new(), width, height: ht, depth, source: ul.span.start..ul.span.end };
        (run, self.recs.len() - 1)
    }

    /// ulem `\UL@setULdepth`: `\dp` of `\hbox{{(j}}` — max depth of `(`
    /// and `j` in the current text font. For cmr/lmr that is `(` at
    /// 0.25em (pdflatex 10pt 2.5pt, 12pt 3.0pt). Fallback 0.25em when
    /// the face has no TFM.
    fn uline_depth(&self, size: f64) -> f64 {
        use crate::ids::{Encoding, EncodingCode};
        let r = self.fonts.resolve(
            self.style.family,
            crate::fonts::Role::Text { bold: false, italic: false },
            size,
        );
        if let (None, Some(tfm)) = (&r.substituted, &r.face.tfm) {
            let depth = |ch: char| {
                EncodingCode::for_char(ch, Encoding::T1)
                    .and_then(|c| tfm.metrics(c.0))
                    .map(|m| crate::tfm::Tfm::pt(m.depth, size))
            };
            match (depth('('), depth('j')) {
                (Some(a), Some(b)) => return a.max(b),
                (Some(a), None) => return a,
                (None, Some(b)) => return b,
                (None, None) => {}
            }
        }
        0.25 * size
    }

    /// A header or footer line,`\hb@xt@\textwidth{<left>\hfil <center>\hfil
    /// <right>}` in the `\normalsize` body font: each slot is `(text,
    /// \slshape)`; `\thepage` is upright, marks slanted. Words are separated
    /// by interword glue (space factor 1000, `\ `/`\space` in the class
    /// macros) or a `\quad`.
    fn chrome_line(&mut self, slots: [Option<(&str, bool)>; 3], width: f64, span: Span) -> Option<BuiltBlock> {
        let size = self.style.body_size_pt;
        let mut placed: Vec<(pl::GlyphRun, usize, f64, usize)> = Vec::new();
        let mut widths = [0.0f64; 3];
        for (k, slot) in slots.iter().enumerate() {
            let Some((text, slanted)) = slot else { continue };
            let style = TextStyle {
                slanted: *slanted,
                ..TextStyle::default()
            };
            let mut x = 0.0;
            for tok in chrome_tokens(text) {
                match tok {
                    ChromeTok::Space(factor) => x += self.space_glue(style, size, factor).width,
                    ChromeTok::Quad => x += self.text_params(style, size).quad,
                    ChromeTok::Word(w) => {
                        let seg = adapter::Segment {
                            chars: w
                                .chars()
                                .map(|_| adapter::CharSrc {
                                    document: span.document,
                                    start: span.start,
                                    end: span.end,
                                })
                                .collect(),
                            text: w,
                            style,
                        };
                        for (item, rec) in self.word_items(&seg, size, false) {
                            match (item, rec) {
                                (pl::Item::Box(run), Some(rec)) => {
                                    let advance = run.width;
                                    placed.push((run, rec, x, k));
                                    x += advance;
                                }
                                (pl::Item::Box(run), None) => x += run.width,
                                (pl::Item::Kern(kern), _) => x += kern.width,
                                (pl::Item::Glue(glue), _) => x += glue.width,
                                (pl::Item::Penalty(_), _) => {}
                            }
                        }
                    }
                }
            }
            widths[k] = x;
        }
        if placed.is_empty() {
            return None;
        }
        let origin = [0.0, widths[0] + (width - widths[0] - widths[1] - widths[2]) / 2.0, width - widths[2]];
        let (mut height, mut depth) = (0.0f64, 0.0f64);
        let mut runs = Vec::with_capacity(placed.len());
        let mut items = Vec::with_capacity(placed.len());
        let mut recs = Vec::with_capacity(placed.len());
        for (run, rec, x, k) in placed {
            height = height.max(run.height);
            depth = depth.max(run.depth);
            runs.push(position_run(&run, origin[k] + x, 0.0));
            items.push(pl::Item::Box(run));
            recs.push(Some(rec));
        }
        let n = items.len();
        let lines = pl::Lines {
            lines: vec![pl::Line {
                index: 0,
                runs,
                baseline_y: height,
                height,
                depth,
                natural_width: width,
                set_width: width,
                ratio: 0.0,
                badness: 0.0,
                items: 0..n,
                hyphenated: false,
            }],
            breaks: Vec::new(),
            stats: one_line_stats(),
            diagnostics: Vec::new(),
            height: height + depth,
        };
        Some(BuiltBlock {
            block: pl::ParagraphBlock::body(lines),
            items,
            recs,
            vertical: VBlock {
                lines: vec![(height, depth)],
                penalty_before: None,
                space_before: None,
                parskip: None,
                interline_penalty: 0,
                club_penalty: 0,
                widow_penalty: 0,
                penalty_after: None,
                space_after: None,
                no_interline_first: true,
                no_interline_after: true,
                baselineskip: None,
                vskip_after: Vec::new(),
                broken_penalty: Vec::new(),
                vadjust_penalty: Vec::new(),
                fil_break: false,
                pre_space_after: None,
                lineskip: None,
                contributed: None,
                line_penalty: Vec::new(),
                depth_after: pagebuild::DepthAfter::default(),
            },
            labels: Vec::new(),
            cache_key: None,
        })
    }

    /// `(\displayindent, \displaywidth)` of a display in a paragraph set
    /// under `style` inside the lists `list_geom` (TeX §1149: taken from
    /// the `\parshape`; `quote` indents both sides by `\leftmargini`, a
    /// list its `\@totalleftmargin`).
    fn display_shape(&mut self, style: ParaStyle, list_geom: Option<&ListGeom>) -> (f64, f64) {
        let size = self.style.body_size_pt;
        let quote = if matches!(style, ParaStyle::Quote) { self.style.leftmargini_pt } else { 0.0 };
        let hang = list_geom.map_or(0.0, |g| self.list_geometry(g, size).0);
        let s = quote + hang;
        (s, (self.style.text_width_pt - s - quote).max(0.0))
    }

    /// A `tikzpicture` (the TikZ subset of `flashtex-vector-graphics`),
    /// compiled at the body size with node text shaped in Latin Modern and
    /// set as one box of the picture's bounding box whose bottom edge is the
    /// baseline (TikZ's default `baseline`), flush left (centred inside
    /// `center`), with the paragraph's `\parskip` and interline glue.
    fn picture_block(&mut self, document: DocumentId, source: &flashtex_vector_graphics::tikz::PictureSource, centered: bool) -> BuiltBlock {
        use flashtex_vector_graphics::tikz::{Severity, Tikz};
        const PT_PER_BP: f64 = 72.27 / 72.0;
        let text = self.texts.get(document.0).copied().unwrap_or("");
        let mut tikz = Tikz::new(self.style.body_size_pt);
        let preamble_end = text.find("\\begin{document}").filter(|e| *e <= source.start).unwrap_or(0);
        let mut diags = tikz.read_preamble(&text[..preamble_end]);
        let measurer = crate::tikz::FontMeasurer { fonts: self.fonts };
        let picture = tikz.render(text, source, &measurer);
        diags.extend(picture.diagnostics.iter().cloned());
        let span = Span::in_document(document, source.start, source.end);
        for d in diags {
            let src = vec![self.source(Span::in_document(document, d.start.min(text.len()), d.end.min(text.len())))];
            let diag = match d.severity {
                Severity::Error => Diagnostic::error("tikz_error", d.message, src),
                Severity::Warning => Diagnostic::warning("tikz_unsupported", d.message, src),
            };
            self.emit(None, diag);
        }
        let mut shaped = Vec::with_capacity(picture.texts.len());
        for t in &picture.texts {
            let tr = t.transform;
            if tr.b.abs() > 1e-9 || tr.c.abs() > 1e-9 || (tr.a - 1.0).abs() > 1e-9 || (tr.d - 1.0).abs() > 1e-9 {
                let src = vec![self.source(Span::in_document(document, t.source.0, t.source.1))];
                self.emit(
                    None,
                    Diagnostic::warning(
                        "tikz_text_transform",
                        format!("node text `{}` is rotated or scaled; glyph runs carry no transform, so it is set upright at its origin", t.text),
                        src,
                    ),
                );
            }
            shaped.push(crate::tikz::shape_text(self.fonts, &t.text, &t.style));
        }
        if !picture.items.is_empty() {
            let src = vec![self.source(span)];
            self.emit(
                Some("tikz_display_list_only".into()),
                Diagnostic::warning(
                    "tikz_display_list_only",
                    "TikZ paths are emitted only as display-list-v2 path items (proposal path-v0); runtime-v1 items and --pdf omit them",
                    src,
                ),
            );
        }
        let width = picture.width_bp * PT_PER_BP;
        let height = picture.height_bp * PT_PER_BP;
        self.recs.push(BoxRec::Picture(Rc::new(PictureRec {
            picture,
            texts: shaped,
            span,
        })));
        let rec = self.recs.len() - 1;
        let x = if centered { ((self.style.text_width_pt - width) / 2.0).max(0.0) } else { 0.0 };
        let run = pl::GlyphRun {
            font: MATH_SENTINEL,
            size: self.style.body_size_pt,
            glyphs: Vec::new(),
            width,
            height,
            depth: 0.0,
            source: span.start..span.end,
        };
        let line = pl::Line {
            index: 0,
            runs: vec![position_run(&run, x, height)],
            baseline_y: height,
            height,
            depth: 0.0,
            natural_width: width,
            set_width: width,
            ratio: 0.0,
            badness: 0.0,
            items: 0..1,
            hyphenated: false,
        };
        let lines = pl::Lines {
            lines: vec![line],
            breaks: Vec::new(),
            stats: pl::Stats {
                algorithm: pl::Algorithm::TotalFit,
                lines: 1,
                pass: 1,
                total_demerits: 0.0,
                overfull: Vec::new(),
                underfull: Vec::new(),
                hyphenated_lines: 0,
                emergency_pass_used: false,
            },
            diagnostics: Vec::new(),
            height,
        };
        let vertical = VBlock {
            lines: vec![(height, 0.0)],
            penalty_before: None,
            space_before: None,
            parskip: Some(skip_tuple(self.parskip_of(None))),
            interline_penalty: 0,
            club_penalty: 0,
            widow_penalty: 0,
            penalty_after: None,
            space_after: None,
            no_interline_first: false,
            no_interline_after: false,
            baselineskip: None,
            vskip_after: Vec::new(),
            broken_penalty: Vec::new(),
            vadjust_penalty: Vec::new(),
            fil_break: false,
            pre_space_after: None,
            lineskip: None,
            contributed: None,
            line_penalty: Vec::new(),
            depth_after: pagebuild::DepthAfter::default(),
        };
        BuiltBlock {
            block: pl::ParagraphBlock::body(lines),
            items: vec![pl::Item::Box(run)],
            recs: vec![Some(rec)],
            vertical,
            labels: Vec::new(),
            cache_key: None,
        }
    }

    /// A paragraph line with no boxes (only whatsits such as `\label`, its
    /// trailing space dropped by line_break): height and depth 0, set with
    /// the usual interline glue, no `\parskip` (the paragraph continues).
    fn empty_line_block(&mut self) -> BuiltBlock {
        let line = pl::Line {
            index: 0,
            runs: Vec::new(),
            baseline_y: 0.0,
            height: 0.0,
            depth: 0.0,
            natural_width: 0.0,
            set_width: self.style.text_width_pt,
            ratio: 0.0,
            badness: 0.0,
            items: 0..0,
            hyphenated: false,
        };
        let vertical = VBlock {
            lines: vec![(0.0, 0.0)],
            penalty_before: None,
            space_before: None,
            parskip: None,
            interline_penalty: 0,
            club_penalty: 0,
            widow_penalty: 0,
            penalty_after: None,
            space_after: None,
            no_interline_first: false,
            no_interline_after: false,
            baselineskip: None,
            vskip_after: Vec::new(),
            broken_penalty: Vec::new(),
            vadjust_penalty: Vec::new(),
            fil_break: false,
            pre_space_after: None,
            lineskip: None,
            contributed: None,
            line_penalty: Vec::new(),
            depth_after: pagebuild::DepthAfter::default(),
        };
        BuiltBlock {
            block: pl::ParagraphBlock {
                lines: pl::Lines {
                    lines: vec![line],
                    breaks: Vec::new(),
                    stats: pl::Stats {
                        algorithm: pl::Algorithm::TotalFit,
                        lines: 1,
                        pass: 1,
                        total_demerits: 0.0,
                        overfull: Vec::new(),
                        underfull: Vec::new(),
                        hyphenated_lines: 0,
                        emergency_pass_used: false,
                    },
                    diagnostics: Vec::new(),
                    height: 0.0,
                },
                space_before: pl::Glue::fixed(0.0),
                space_after: pl::Glue::fixed(0.0),
                keep_with_next: false,
            },
            items: Vec::new(),
            recs: Vec::new(),
            vertical,
            labels: Vec::new(),
            cache_key: None,
        }
    }

    /// An equation number or `\tag` label as amsmath's `\maketag@@@` sets
    /// it: an `\hbox` of the text in the body face whose interword spaces
    /// are TeX's glue at natural width (`\ignorespaces`/`\unskip` drop the
    /// outer ones), not the T1 visible-space glyph one shaped run would use.
    /// Each word is its own text box at its offset.
    fn number_box(&mut self, text: &str, nspan: Span, size: f64) -> Option<NumberBox> {
        self.word_box(text, nspan, size, TextStyle::default())
    }

    /// `text` as one `\hbox`, each word its own shaped run at its offset and
    /// the gaps TeX's interword glue at natural width. One shaped run over
    /// the whole string would paint the T1 *visible space* glyph in the gap,
    /// whose advance is not `\fontdimen2` — 6.27 bp instead of 4.18 bp for
    /// `\bfseries` Latin Modern at 11 pt, which pushed everything after a
    /// two-word `\item[...]` label 2.1 bp right of pdflatex.
    fn word_box(&mut self, text: &str, nspan: Span, size: f64, style: TextStyle) -> Option<NumberBox> {
        let space = self.space_glue(style, size, 1000).width;
        let mut pieces = Vec::new();
        let (mut x, mut height, mut depth) = (0.0f64, 0.0f64, 0.0f64);
        for (i, word) in text.split_whitespace().enumerate() {
            if i > 0 {
                x += space;
            }
            let seg = adapter::Segment {
                text: word.to_string(),
                chars: word
                    .chars()
                    .map(|_| adapter::CharSrc {
                        document: nspan.document,
                        start: nspan.start,
                        end: nspan.end,
                    })
                    .collect(),
                style,
            };
            let (run, rec) = self.text_box(&seg, size)?;
            height = height.max(run.height);
            depth = depth.max(run.depth);
            let w = run.width;
            pieces.push((run, rec, x));
            x += w;
        }
        (!pieces.is_empty()).then_some(NumberBox { pieces, width: x, height, depth })
    }

    /// A display equation. `pre_display_size` is TeX's measure of the line
    /// before it (its material width plus 2em, or `None` when the display
    /// starts the paragraph); `number` is the `equation` counter set flush
    /// right (`\eqno`). `style` and `list_geom` give the paragraph's
    /// `\parshape` (§1149): the display is centred in `\displaywidth`
    /// (`\linewidth`) starting `\displayindent` (`\@totalleftmargin`) in.
    fn display_block(
        &mut self,
        list: &flashtex_compiler::math::MathList,
        span: Span,
        pre_display_size: Option<f64>,
        number: Option<&(String, Span)>,
        style: ParaStyle,
        list_geom: Option<&ListGeom>,
    ) -> Option<BuiltBlock> {
        let rec = self.math_box(list, span, true, self.style.body_size_pt)?;
        let BoxRec::Math(mi) = &self.recs[rec] else { unreachable!() };
        let mi = *mi;
        let size = self.style.body_size_pt;
        let natural_width = self.maths[mi].root.width;
        let (s, z) = self.display_shape(style, list_geom);
        let source: &str = self.texts.get(span.document.0).copied().unwrap_or("");
        let quad = self.text_params(TextStyle::default(), size).quad;
        // The number's box `a` (§1199): its width reduces the room for the
        // formula. `\leqno` (amsmath `leqno`, or the primitive) puts it left.
        let mut eqno: Option<NumberBox> = None;
        let mut left = false;
        let mut e = 0.0;
        let mut q = 0.0;
        if let Some((text, nspan)) = number {
            let at = source.get(nspan.start..).unwrap_or("");
            left = if at.starts_with("\\leqno") {
                true
            } else if at.starts_with("\\eqno") {
                false
            } else {
                self.style.leqno
            };
            if let Some(nb) = self.number_box(text, *nspan, size) {
                e = nb.width;
                q = e + quad;
                eqno = Some(nb);
            }
        }
        let rest = source.get(span.start..).unwrap_or("");
        // amsmath's `\mathdisplay` honours `fleqn`; a primitive `$$` does not.
        let fleqn = self.style.fleqn && !rest.starts_with("$$");
        // §1201: a number that cannot sit beside the squeezed formula goes on
        // a line of its own (TeX sets e := 0).
        let mut separate = false;
        let mut overfull = (natural_width - z).max(0.0);
        // (formula x, number x, d of §1202)
        let (x, number_x, d) = if fleqn {
            // amsmath `fleqn` (`\endmathdisplay@fleqn`): the formula is set
            // in an `\hbox to\displaywidth` after `\@mathmargin`
            // (`\leftmargini`); the tag follows `\hfil` flush right
            // (`\emdf@R`) or comes first with at least `\mintagsep` before
            // the formula (`\emdf@L`). TeX sees one display-wide box and no
            // `\eqno`: d = 0.
            let mintagsep = 0.5 * size;
            let margin = self.style.leftmargini_pt;
            if left && e > 0.0 {
                (s + margin.max(e + mintagsep), s, 0.0)
            } else {
                (s + margin, s + z - e, 0.0)
            }
        } else {
            // §1199-§1201: a formula too wide beside its number is squeezed.
            // With a number, `hpack(p, z - q, exactly)` when its finite
            // shrink reaches (or any infinite shrink exists); otherwise the
            // number goes on a line of its own and the formula alone is
            // packed to `z` if it is still wider. The glue set comes from
            // math-layout's `pack_to` (tex.web §649-§667).
            let mut w = natural_width;
            if w + q > z {
                let root = &self.maths[mi].root;
                let totals = root.glue_totals();
                let infinite = totals.shrink[1..].iter().any(|t| *t != 0.0);
                let packed = if e != 0.0 && (w - totals.shrink[0] + q <= z || infinite) {
                    Some(root.pack_to(z - q))
                } else {
                    separate = e != 0.0;
                    (w > z).then(|| root.pack_to(z))
                };
                if let Some(p) = packed {
                    w = p.root.width;
                    overfull = p.overfull;
                    self.maths[mi].root = p.root;
                }
            }
            let e = if separate { 0.0 } else { e };
            // §1202: centred, or moved off a number closer than 2e (to 0
            // when the formula starts with glue).
            let mut d = (z - w) / 2.0;
            if e > 0.0 && d < 2.0 * e {
                d = (z - w - e) / 2.0;
                if let ml::BoxKind::HBox(children) = &self.maths[mi].root.kind {
                    if children.first().is_some_and(|c| matches!(c.content.kind, ml::BoxKind::Glue { .. })) {
                        d = 0.0;
                    }
                }
            }
            // §1204: `\leqno` packs [a, kern z-w-e-d, b] at s; `\eqno`
            // [b, kern z-w-e-d, a] at s + d. §1203/§1205: a number on its own
            // line sits at s (`\leqno`, above) or s + z - width(a) (below).
            if left && e > 0.0 {
                (s + z - w - d, s, d)
            } else if left && separate {
                (s + d, s, d)
            } else {
                (s + d, s + z - eqno.as_ref().map_or(0.0, |n| n.width), d)
            }
        };
        let run = math_run(&self.maths[mi].root, size, span);
        let width = run.width;
        let (formula_height, formula_depth) = (run.height, run.depth);
        let (mut height, mut depth) = (formula_height, formula_depth);
        if let (Some(nb), false) = (&eqno, separate) {
            height = height.max(nb.height);
            depth = depth.max(nb.depth);
        }
        // amsmath sets `equation{split}` through `\gather@`'s `\halign`, a
        // display alignment: always the non-short skips (§1206).
        let split = rest
            .strip_prefix("\\begin{equation*}")
            .or_else(|| rest.strip_prefix("\\begin{equation}"))
            .is_some_and(|r| r.trim_start().starts_with("\\begin{split}"));
        // §1203: "not enough clearance" (`d + s <= \predisplaysize`) or a
        // `\leqno` number takes the normal skips.
        let long = pre_display_size.is_some_and(|p| s + d <= p) || (left && eqno.is_some() && !fleqn) || split;
        let (above, below) = if long {
            (self.style.abovedisplayskip, self.style.belowdisplayskip)
        } else {
            (self.style.abovedisplayshortskip, self.style.belowdisplayshortskip)
        };
        let formula_run = pl::PositionedRun {
            x,
            baseline_y: height,
            width,
            font: run.font,
            size: run.size,
            glyphs: Vec::new(),
            source: run.source.clone(),
            is_hyphen: false,
        };
        // Each output line: (runs, items, recs, height, depth, natural width).
        type OutLine = (Vec<pl::PositionedRun>, Vec<pl::Item>, Vec<Option<usize>>, f64, f64, f64);
        let mut out_lines: Vec<OutLine> = Vec::new();
        // The number's word boxes at `number_x`, on a baseline of `baseline`.
        let number_parts = |nb: NumberBox, baseline: f64| {
            let mut runs = Vec::new();
            let mut items = Vec::new();
            let mut recs = Vec::new();
            for (nrun, nrec, dx) in nb.pieces {
                runs.push(position_run(&nrun, number_x + dx, baseline));
                items.push(pl::Item::Box(nrun));
                recs.push(Some(nrec));
            }
            (runs, items, recs)
        };
        match eqno {
            Some(nb) if separate => {
                let (nh, nd, nw) = (nb.height, nb.depth, nb.width);
                let (runs, line_items, line_recs) = number_parts(nb, nh);
                let number_line: OutLine = (runs, line_items, line_recs, nh, nd, number_x + nw);
                let formula_line: OutLine = (vec![formula_run], vec![pl::Item::Box(run)], vec![Some(rec)], formula_height, formula_depth, x + width);
                if left {
                    out_lines.push(number_line);
                    out_lines.push(formula_line);
                } else {
                    out_lines.push(formula_line);
                    out_lines.push(number_line);
                }
            }
            Some(nb) => {
                let (number_runs, number_items, number_recs) = number_parts(nb, height);
                let mut runs = vec![formula_run];
                runs.extend(number_runs);
                let mut line_items = vec![pl::Item::Box(run)];
                line_items.extend(number_items);
                let mut line_recs = vec![Some(rec)];
                line_recs.extend(number_recs);
                out_lines.push((runs, line_items, line_recs, height, depth, s + width));
            }
            None => out_lines.push((vec![formula_run], vec![pl::Item::Box(run)], vec![Some(rec)], height, depth, s + width)),
        }
        let mut items = Vec::new();
        let mut recs = Vec::new();
        let mut line_list = Vec::new();
        let mut extents = Vec::new();
        for (index, (runs, line_items, line_recs, h, dp, natural)) in out_lines.into_iter().enumerate() {
            let start = items.len();
            items.extend(line_items);
            recs.extend(line_recs);
            line_list.push(pl::Line {
                index,
                runs,
                baseline_y: h,
                height: h,
                depth: dp,
                natural_width: natural,
                set_width: s + z,
                ratio: 0.0,
                badness: 0.0,
                items: start..items.len(),
                hyphenated: false,
            });
            extents.push((h, dp));
        }
        let n_lines = line_list.len();
        let lines = pl::Lines {
            lines: line_list,
            breaks: Vec::new(),
            stats: pl::Stats {
                algorithm: pl::Algorithm::TotalFit,
                lines: n_lines,
                pass: 1,
                total_demerits: 0.0,
                overfull: Vec::new(),
                underfull: Vec::new(),
                hyphenated_lines: 0,
                emergency_pass_used: false,
            },
            diagnostics: Vec::new(),
            height: extents.iter().map(|(h, dp)| h + dp).sum(),
        };
        if overfull > 1e-6 {
            let src = self.source(span);
            self.emit(None, Diagnostic::warning(
                "overfull_display",
                format!("display is {:.2}pt wider than the {}", overfull, if s > 0.0 { "line width" } else { "text width" }),
                vec![src],
            ));
        }
        // $$: \penalty\predisplaypenalty, \abovedisplayskip, the display,
        // \penalty\postdisplaypenalty (0), \belowdisplayskip. A number on its
        // own line is kept with the formula (\penalty10000, interline glue
        // between them); above the formula (`\leqno`) it replaces the
        // above-display skip, below it (`\eqno`) the below-display skip
        // (§1203, §1205).
        let (space_before, space_after) = match (separate, left) {
            (true, true) => (None, Some(skip_tuple(below))),
            (true, false) => (Some(skip_tuple(above)), None),
            _ => (Some(skip_tuple(above)), Some(skip_tuple(below))),
        };
        let vertical = VBlock {
            lines: extents,
            penalty_before: Some(PREDISPLAY_PENALTY),
            space_before,
            parskip: None,
            interline_penalty: if separate { pagebuild::INF_PENALTY } else { 0 },
            club_penalty: 0,
            widow_penalty: 0,
            penalty_after: None,
            space_after,
            no_interline_first: false,
            no_interline_after: false,
            baselineskip: None,
            vskip_after: Vec::new(),
            broken_penalty: Vec::new(),
            vadjust_penalty: Vec::new(),
            fil_break: false,
            pre_space_after: None,
            lineskip: None,
            contributed: None,
            line_penalty: Vec::new(),
            depth_after: pagebuild::DepthAfter::default(),
        };
        Some(BuiltBlock {
            block: pl::ParagraphBlock {
                lines,
                space_before: if space_before.is_some() { above.glue() } else { pl::Glue::fixed(0.0) },
                space_after: if space_after.is_some() { below.glue() } else { pl::Glue::fixed(0.0) },
                keep_with_next: false,
            },
            items,
            recs,
            vertical,
            labels: Vec::new(),
            cache_key: None,
        })
    }

    /// An amsmath display alignment (`align`, `alignat`, `flalign`,
    /// `gather`, `multline`) as one block of rows: amsmath's own measuring
    /// (`\measure@`/`\calc@shift@align`, `\calc@shift@gather`,
    /// `\multline@`) decides every cell's x; rows are `\halign` lines with
    /// `\strut@` minima and `\openup\jot` pitch; numbers (`\tagform@`) sit
    /// flush right. Display alignments always take `\abovedisplayskip`/
    /// `\belowdisplayskip` (§1206); `\@display@init` removes one `\jot`
    /// before the first row of `align`/`gather`.
    fn rows_block(&mut self, env: adapter::RowsEnv, rows: &[adapter::RowPart], span: Span) -> Option<BuiltBlock> {
        use adapter::RowsEnv;
        const MINALIGNSEP: f64 = 10.0;
        const MULTLINEGAP: f64 = 10.0;
        const MULTLINETAGGAP: f64 = 10.0;
        const JOT: f64 = 3.0;
        let size = self.style.body_size_pt;
        let dw = self.style.text_width_pt;
        // `\mintagsep`: half of cmsy's quad at the text size.
        let mintagsep = 0.5 * size;
        // amsmath `fleqn` (`\@mathmargin` = `\leftmargini`) and `leqno`.
        let (fleqn, leqno) = (self.style.fleqn, self.style.leqno);
        let margin = self.style.leftmargini_pt;
        let aligned = matches!(env, RowsEnv::Align | RowsEnv::AlignAt | RowsEnv::FlAlign);
        // Cell boxes: (run, rec) per row per cell; right-hand (even-index
        // from 1) align cells and every multline row start with `{}`.
        struct Cell {
            run: Option<(pl::GlyphRun, usize)>,
        }
        let mut cells: Vec<Vec<Cell>> = Vec::with_capacity(rows.len());
        for row in rows {
            let mut out = Vec::with_capacity(row.cells.len());
            for (ci, list) in row.cells.iter().enumerate() {
                let Some(first) = list.atoms.first() else {
                    out.push(Cell { run: None });
                    continue;
                };
                let prefix = (aligned && ci % 2 == 1) || matches!(env, RowsEnv::Multline);
                let mut list = list.clone();
                if prefix {
                    let mut empty = first.clone();
                    empty.nucleus = flashtex_compiler::math::Nucleus::Symbol(String::new());
                    empty.superscript = None;
                    empty.subscript = None;
                    list.atoms.insert(0, empty);
                }
                let cspan = list.atoms.iter().map(|a| a.span).reduce(Span::merge).unwrap_or(row.span);
                let run = self.math_box(&list, cspan, true, self.style.body_size_pt).map(|rec| {
                    let BoxRec::Math(mi) = &self.recs[rec] else { unreachable!() };
                    (math_run(&self.maths[*mi].root, size, cspan), rec)
                });
                out.push(Cell { run });
            }
            cells.push(out);
        }
        let width = |c: &Cell| c.run.as_ref().map_or(0.0, |(r, _)| r.width);
        // Number boxes.
        let mut tags: Vec<Option<(pl::GlyphRun, usize)>> = Vec::with_capacity(rows.len());
        for row in rows {
            tags.push(row.number.as_ref().and_then(|(text, nspan)| {
                let label = text.clone();
                let seg = adapter::Segment {
                    text: label.clone(),
                    chars: label
                        .chars()
                        .map(|_| adapter::CharSrc {
                            document: nspan.document,
                            start: nspan.start,
                            end: nspan.end,
                        })
                        .collect(),
                    style: TextStyle::default(),
                };
                self.text_box(&seg, size)
            }));
        }
        let tagw = |i: usize| tags[i].as_ref().map_or(0.0, |(r, _)| r.width);
        // x of every cell, per row.
        let mut xs: Vec<Vec<f64>> = cells.iter().map(|r| vec![0.0; r.len()]).collect();
        let mut shifted_tags = 0usize;
        match env {
            RowsEnv::Align | RowsEnv::AlignAt | RowsEnv::FlAlign => {
                let mut maxfields = cells.iter().map(Vec::len).max().unwrap_or(0);
                if maxfields % 2 == 1 {
                    maxfields += 1;
                }
                let mut colw = vec![0.0f64; maxfields];
                for row in &cells {
                    for (ci, c) in row.iter().enumerate() {
                        colw[ci] = colw[ci].max(width(c));
                    }
                }
                // `\measure@`: under `fleqn` `\totwidth@` includes `\@mathmargin`.
                let totwidth: f64 = colw.iter().sum::<f64>() + if fleqn { margin } else { 0.0 };
                let d = dw - totwidth;
                let p = (maxfields / 2) as i64;
                let (mut eqnshift, mut alignsep, minalignsep, tempcntb, tempcnta);
                match env {
                    RowsEnv::AlignAt => {
                        alignsep = 0.0;
                        minalignsep = 0.0;
                        tempcntb = 0i64;
                        tempcnta = if fleqn { 1i64 } else { 2i64 };
                        eqnshift = if fleqn { margin } else { d / 2.0 };
                    }
                    RowsEnv::Align if fleqn => {
                        tempcntb = p - 1;
                        tempcnta = p;
                        eqnshift = margin;
                        alignsep = if tempcnta != 0 { d / tempcnta as f64 } else { d };
                        minalignsep = MINALIGNSEP;
                    }
                    RowsEnv::Align => {
                        tempcntb = p - 1;
                        tempcnta = p + 1;
                        eqnshift = d / tempcnta as f64;
                        alignsep = eqnshift;
                        minalignsep = MINALIGNSEP;
                    }
                    _ => {
                        tempcntb = p - 1;
                        tempcnta = p - 1;
                        eqnshift = 0.0;
                        let d = if fleqn { d + margin } else { d };
                        // TeX's \divide by zero leaves the dimension unchanged.
                        alignsep = if tempcntb > 0 { d / tempcntb as f64 } else { d };
                        minalignsep = MINALIGNSEP;
                    }
                }
                if alignsep < minalignsep {
                    alignsep = minalignsep;
                    if eqnshift > 0.0 && !fleqn {
                        eqnshift = (dw - totwidth - tempcntb as f64 * alignsep) / 2.0;
                    }
                }
                eqnshift = eqnshift.max(0.0);
                // `\calc@shift@align` (tags right, not fleqn): last row first.
                // The `fleqn`/`leqno` variants only move tags that do not
                // fit onto their own line, which is not modelled.
                for ri in (0..cells.len()).rev().filter(|_| !(fleqn || leqno)) {
                    let t = tagw(ri);
                    if t <= 0.0 {
                        continue;
                    }
                    // `\x@rcalc@width`: right columns count fully, left
                    // columns by their own width, trailing empty space dropped.
                    let (mut dimb, mut dimc) = (0.0f64, 0.0f64);
                    for (ci, c) in cells[ri].iter().enumerate() {
                        let a = width(c);
                        if a > 0.0 {
                            dimc += dimb;
                            if ci % 2 == 0 {
                                dimc += colw[ci];
                                dimb = 0.0;
                            } else {
                                dimc += a;
                                dimb = colw[ci] - a;
                            }
                        } else {
                            dimb += colw[ci];
                        }
                    }
                    let k = (cells[ri].len() as i64 - 1).max(0) / 2;
                    let (mut cntb, mut cnta) = (tempcntb, tempcnta);
                    if cntb > k {
                        cnta = cnta - cntb + k;
                        cntb = k;
                    }
                    let dima = dimc + t;
                    let mut dimen = minalignsep * cntb as f64 + mintagsep + dima;
                    if env != RowsEnv::FlAlign {
                        dimen += mintagsep;
                    }
                    if dimen > dw {
                        shifted_tags += 1;
                        continue;
                    }
                    let dimen = eqnshift + dima + cntb as f64 * alignsep + t;
                    if dimen > dw {
                        let mut dimen = dw - dima;
                        if env == RowsEnv::FlAlign {
                            dimen -= mintagsep;
                        }
                        if cnta != 0 {
                            dimen /= cnta as f64;
                        }
                        if dimen < minalignsep {
                            alignsep = minalignsep;
                            eqnshift = (dw - dima - cntb as f64 * alignsep) / 2.0;
                        } else {
                            if dimen < eqnshift {
                                eqnshift = dimen.max(0.0);
                            }
                            if dimen < alignsep {
                                alignsep = dimen;
                            }
                        }
                    }
                }
                for (ri, row) in cells.iter().enumerate() {
                    let mut x = eqnshift;
                    for (ci, c) in row.iter().enumerate() {
                        xs[ri][ci] = if ci % 2 == 0 { x + colw[ci] - width(c) } else { x };
                        x += colw[ci];
                        if ci % 2 == 1 {
                            x += alignsep;
                        }
                    }
                }
            }
            RowsEnv::Gather => {
                for (ri, row) in cells.iter().enumerate() {
                    let w: f64 = row.iter().map(width).sum();
                    let t = tagw(ri);
                    let mut shift = dw - w;
                    if t > 0.0 {
                        if 2.0 * mintagsep + w + t > dw {
                            shifted_tags += 1;
                        } else if shift < 4.0 * t {
                            shift -= t;
                        }
                    }
                    // `\calc@shift@gather`: `\@mathmargin` under `fleqn`;
                    // with `leqno` the shift is mirrored.
                    let mut x = if fleqn {
                        margin
                    } else if leqno {
                        (dw - w - shift / 2.0).max(0.0)
                    } else {
                        (shift / 2.0).max(0.0)
                    };
                    for (ci, c) in row.iter().enumerate() {
                        xs[ri][ci] = x;
                        x += width(c);
                    }
                }
            }
            RowsEnv::Multline => {
                let n = cells.len();
                for (ri, row) in cells.iter().enumerate() {
                    let w: f64 = row.iter().map(width).sum();
                    let t = tagw(ri);
                    let x0 = if n > 1 && ri == 0 {
                        MULTLINEGAP
                    } else if n > 1 && ri + 1 == n {
                        dw - w - if t > 0.0 { MULTLINETAGGAP + t } else { MULTLINEGAP }
                    } else {
                        (dw - w) / 2.0
                    };
                    let mut x = x0;
                    for (ci, c) in row.iter().enumerate() {
                        xs[ri][ci] = x;
                        x += width(c);
                    }
                }
            }
        }
        if shifted_tags > 0 {
            let src = self.source(span);
            self.emit(None, Diagnostic::warning(
                "math_limitation",
                format!("{shifted_tags} equation number(s) too wide for their row set on the row's baseline; amsmath moves them to a line of their own"),
                vec![src],
            ));
        }
        // Rows: `\strut@` (.7/.3 `\normalbaselineskip`) minima.
        let normal = self.style.baselineskip_pt;
        let (strut_h, strut_d) = (0.7 * normal, 0.3 * normal);
        let mut items = Vec::new();
        let mut recs = Vec::new();
        let mut lines: Vec<pl::Line> = Vec::with_capacity(rows.len());
        let mut extents = Vec::with_capacity(rows.len());
        let mut vskips: Vec<f64> = Vec::with_capacity(rows.len());
        // Skip ahead of the first line when an `\intertext` opens the display.
        let mut lead = 0.0;
        let mut total = 0.0;
        for (ri, row) in cells.iter().enumerate() {
            // `\intertext`: `\noalign{\penalty\postdisplaypenalty \vskip<before>
            // \vbox{\normalbaselines \noindent#1\par} \penalty\predisplaypenalty
            // \vskip<after>}` between the previous row and this one; the
            // interline glue around the `\vbox` is the alignment's
            // (`\baselineskip+\jot`), inside it `\normalbaselines`.
            for text in &rows[ri].intertext {
                let (before, after) = self.intertext_skips(text);
                match vskips.last_mut() {
                    Some(v) => *v += before,
                    None => lead += before,
                }
                if let Some(b) = self.paragraph_block(&text.items, false, true, false, ParaStyle::Plain, None, None, None) {
                    let offset = items.len();
                    let n = b.block.lines.lines.len();
                    for (k, mut line) in b.block.lines.lines.into_iter().enumerate() {
                        // Hyphen runs find their record through `breaks`,
                        // which this block does not carry: they stay drawn
                        // but unmapped.
                        line.index = lines.len();
                        line.items = line.items.start + offset..line.items.end + offset;
                        extents.push((line.height, line.depth));
                        total += line.height + line.depth;
                        let mut v = b.vertical.vskip_after.get(k).copied().unwrap_or(0.0);
                        if k + 1 < n {
                            v -= JOT;
                        }
                        vskips.push(v);
                        lines.push(line);
                    }
                    items.extend(b.items);
                    recs.extend(b.recs);
                }
                match vskips.last_mut() {
                    Some(v) => *v += after,
                    None => lead += after,
                }
            }
            let start = items.len();
            let (mut h, mut d) = (strut_h, strut_d);
            let mut runs = Vec::new();
            let mut natural = 0.0f64;
            for (ci, c) in row.iter().enumerate() {
                let Some((run, rec)) = &c.run else { continue };
                h = h.max(run.height);
                d = d.max(run.depth);
                natural = natural.max(xs[ri][ci] + run.width);
                runs.push(pl::PositionedRun {
                    x: xs[ri][ci],
                    baseline_y: 0.0,
                    width: run.width,
                    font: run.font,
                    size: run.size,
                    glyphs: Vec::new(),
                    source: run.source.clone(),
                    is_hyphen: false,
                });
                items.push(pl::Item::Box(run.clone()));
                recs.push(Some(*rec));
            }
            if let Some((nrun, nrec)) = &tags[ri] {
                h = h.max(nrun.height);
                d = d.max(nrun.depth);
                runs.push(position_run(nrun, if leqno { 0.0 } else { dw - nrun.width }, 0.0));
                items.push(pl::Item::Box(nrun.clone()));
                recs.push(Some(*nrec));
            }
            if natural > dw + 1e-6 {
                let src = self.source(rows[ri].span);
                self.emit(None, Diagnostic::warning("overfull_display", format!("display row is {:.2}pt wider than the text width", natural - dw), vec![src]));
            }
            lines.push(pl::Line {
                index: lines.len(),
                runs,
                baseline_y: h,
                height: h,
                depth: d,
                natural_width: natural,
                set_width: dw,
                ratio: 0.0,
                badness: 0.0,
                items: start..items.len(),
                hyphenated: false,
            });
            extents.push((h, d));
            vskips.push(0.0);
            total += h + d;
        }
        if lines.is_empty() {
            return None;
        }
        let above = self.style.abovedisplayskip;
        let below = self.style.belowdisplayskip;
        let first_adjust = if matches!(env, RowsEnv::Multline) { 0.0 } else { -JOT };
        let (an, ast, ash) = skip_tuple(above);
        let n = lines.len();
        let lines = pl::Lines {
            lines,
            breaks: Vec::new(),
            stats: pl::Stats {
                algorithm: pl::Algorithm::TotalFit,
                lines: n,
                pass: 1,
                total_demerits: 0.0,
                overfull: Vec::new(),
                underfull: Vec::new(),
                hyphenated_lines: 0,
                emergency_pass_used: false,
            },
            diagnostics: Vec::new(),
            height: total,
        };
        let vertical = VBlock {
            lines: extents,
            penalty_before: Some(PREDISPLAY_PENALTY),
            space_before: Some((an + first_adjust + lead, ast, ash)),
            parskip: None,
            // `\interdisplaylinepenalty` is 10000 in LaTeX.
            interline_penalty: pagebuild::INF_PENALTY,
            club_penalty: 0,
            widow_penalty: 0,
            penalty_after: None,
            space_after: Some(skip_tuple(below)),
            no_interline_first: false,
            no_interline_after: false,
            baselineskip: Some(normal + JOT),
            // `\openup\jot` (amsmath `\displ@y@`) advances `\lineskip` as
            // well as `\baselineskip` — `\openup` is `\advance` on all three
            // of `\lineskip`, `\baselineskip` and `\lineskiplimit`. Leaving
            // this `None` used the page's 1pt `\lineskip`, so every row gap
            // that fell into lineskip mode was one `\jot` = 3pt short, and
            // it only falls into lineskip mode when a row is tall enough
            // that `\baselineskip - prevdepth - height < \lineskiplimit`.
            // Short-row alignments (`a &= b \\ c &= d`) stay in baselineskip
            // mode and were always right, which is why every pinned
            // display-placement align fixture passed while the tall
            // integral/fraction rows of a real problem set drifted 3pt per
            // row. pdfLaTeX's own `\showoutput` for
            // `fixtures/real-world/ps-calculus` prints `\glue(\lineskip) 4.0`
            // between the rows of both of its alignments.
            lineskip: Some(self.style.lineskip_pt + JOT),
            vskip_after: vskips,
            broken_penalty: Vec::new(),
            vadjust_penalty: Vec::new(),
            fil_break: false,
            pre_space_after: None,
            contributed: None,
            line_penalty: Vec::new(),
            depth_after: pagebuild::DepthAfter::default(),
        };
        Some(BuiltBlock {
            block: pl::ParagraphBlock {
                lines,
                space_before: above.glue(),
                space_after: below.glue(),
                keep_with_next: false,
            },
            items,
            recs,
            vertical,
            labels: Vec::new(),
            cache_key: None,
        })
    }

    /// The `\noalign` skips before and after an `\intertext` paragraph:
    /// amsmath `\belowdisplayskip`/`\abovedisplayskip` (`amsmath.sty`
    /// 1190-1197). mathtools (`\MT_intertext:`, `mathtools.sty` 1424-1448;
    /// `\MT_shortintertext:n`, 1502-1528, with `\abovedisplayshortskip` on
    /// both sides) adds `-\lineskiplimit+\normallineskiplimit` to each, which
    /// is `-\jot` under the alignment's `\openup\jot`, and the
    /// `(above|below)[short]intertext` dimensions: 0pt, 3pt for the short form.
    fn intertext_skips(&self, text: &adapter::IntertextPart) -> (f64, f64) {
        const JOT: f64 = 3.0;
        let s = self.style;
        let (before, after, sep) = if text.short {
            (s.abovedisplayshortskip.natural, s.abovedisplayshortskip.natural, 3.0)
        } else {
            (s.belowdisplayskip.natural, s.abovedisplayskip.natural, 0.0)
        };
        if text.mathtools {
            (before - JOT + sep, after - JOT + sep)
        } else {
            (before, after)
        }
    }

    fn report_overfull(&mut self, lines: &pl::Lines, list: &[pl::Item], recs: &[Option<usize>]) {
        for o in &lines.stats.overfull {
            let line = &lines.lines[o.line];
            let span = line
                .items
                .clone()
                .filter_map(|i| recs.get(i).copied().flatten())
                .filter_map(|r| match &self.recs[r] {
                    BoxRec::Text { clusters, .. } => clusters.first().map(|c| c.span),
                    BoxRec::Math(m) => Some(self.maths[*m].span),
                    BoxRec::Rule { span, .. } => Some(*span),
                    BoxRec::Picture(p) => Some(p.span),
                    BoxRec::Table(t) => Some(t.span),
                    BoxRec::ColorBox(c) => Some(c.span),
                    BoxRec::Leader { .. } => None,
                    BoxRec::Underline(u) => Some(u.span),
                })
                .next();
            let _ = list;
            let src = span.map(|s| vec![self.source(s)]).unwrap_or_default();
            self.emit(None, Diagnostic::warning(
                "overfull_hbox",
                format!("overfull line: {:.2}pt too wide", o.excess),
                src,
            ));
        }
    }
}

/// The `\\[<dimen>]` skip after each line: a skip recorded at a forced
/// break lands after the line that break ends.
/// `\brokenpenalty` per line: TeX's `disc_break` is the flagged break at the
/// end of the line (a hyphenation point, `\-`, or the empty discretionary
/// pdfTeX inserts after an explicit hyphen), which `paragraph-layout` reports
/// as `Line::hyphenated`. The last line's entry is never read (§890 appends
/// no penalty after it), and an all-zero vector is dropped so blocks without
/// a hyphenated line keep the empty vector.
fn broken_of(lines: &pl::Lines) -> Vec<i32> {
    if !lines.lines.iter().any(|l| l.hyphenated) {
        return Vec::new();
    }
    lines.lines.iter().map(|l| if l.hyphenated { BROKEN_PENALTY } else { 0 }).collect()
}

fn vskips_of(lines: &pl::Lines, skips: &[(usize, f64)]) -> Vec<f64> {
    if skips.is_empty() {
        return Vec::new();
    }
    lines
        .breaks
        .iter()
        .map(|b| skips.iter().filter(|(item, _)| *item == b.item).map(|(_, pt)| *pt).sum())
        .collect()
}

fn skip_tuple(s: crate::style::Skip) -> (f64, f64, f64) {
    (s.natural, s.stretch, s.shrink)
}

/// `items` between two empty `\hbox`es ([`AItem::LeaveVmode`]), so glue at
/// either end of an `\hbox`'s material is not taken for a paragraph's.
fn anchored(items: &[AItem]) -> Vec<AItem> {
    let mut out = Vec::with_capacity(items.len() + 2);
    out.push(AItem::LeaveVmode);
    out.extend(items.iter().cloned());
    out.push(AItem::LeaveVmode);
    out
}

/// A table entry's lines as a block assembled like a paragraph's.
fn table_cell_block(lines: pl::Lines, items: Vec<pl::Item>, recs: Vec<Option<usize>>, labels: Vec<(String, usize)>) -> BuiltBlock {
    let vertical = VBlock {
        lines: line_extents(&lines),
        penalty_before: None,
        space_before: None,
        parskip: None,
        interline_penalty: 0,
        club_penalty: 0,
        widow_penalty: 0,
        penalty_after: None,
        space_after: None,
        no_interline_first: false,
        no_interline_after: false,
        baselineskip: None,
        vskip_after: Vec::new(),
        broken_penalty: Vec::new(),
        vadjust_penalty: Vec::new(),
        fil_break: false,
        pre_space_after: None,
        lineskip: None,
        contributed: None,
        line_penalty: Vec::new(),
        depth_after: pagebuild::DepthAfter::default(),
    };
    BuiltBlock { block: pl::ParagraphBlock::body(lines), items, recs, vertical, labels, cache_key: None }
}

fn line_extents(lines: &pl::Lines) -> Vec<(f64, f64)> {
    lines.lines.iter().map(|l| (l.height, l.depth)).collect()
}

/// A trailing `\\` under `\centering`/`\raggedleft` (`\@centercr`) is
/// exactly `\par`: the forced break and the glue before it are dropped from
/// the list, its `[<dimen>]` is `\vskip`ped after the paragraph and returned
/// (`Some(0.0)` for a bare `\\`, so the caller still cancels the `\parskip`).
/// Elsewhere `\\` is `\hfil\break` and the list is left alone: the breaker
/// sets the empty last line TeX sets there (the familiar "Underfull \hbox
/// (badness 10000)"), one line pitch tall, and the skip lands after the line
/// the break ends (`vskips_of`). `list`/`recs`/`skips` are the outputs of
/// [`Context::hlist`].
fn drop_trailing_break(list: &mut Vec<pl::Item>, recs: &mut Vec<Option<usize>>, skips: &mut Vec<(usize, f64)>, style: ParaStyle) -> Option<f64> {
    if !matches!(style, ParaStyle::Center | ParaStyle::FlushRight) {
        return None;
    }
    // `hlist` appends `\penalty10000 \parfillskip \penalty-10000`; the
    // item before that triple is the last one of the paragraph proper.
    let trailing_break = |list: &[pl::Item]| {
        let n = list.len();
        n >= 4 && matches!(&list[n - 4], pl::Item::Penalty(p) if p.value <= pl::FORCED_BREAK)
    };
    if !trailing_break(list) {
        return None;
    }
    let mut trailing_skip = 0.0;
    while trailing_break(list) {
        let at = list.len() - 4;
        if let Some(i) = skips.iter().position(|(item, _)| *item == at) {
            trailing_skip += skips.remove(i).1;
        }
        list.remove(at);
        recs.remove(at);
        // The `\hfil` glue `\\` carries plus any glue read before it
        // (discardable after a break, TeX §879); stop at the next `\\`
        // so the outer loop drops it the same way.
        loop {
            let last = list.len() - 3; // the paragraph-end triple starts here
            if last == 0 || !matches!(list[last - 1], pl::Item::Glue(_)) || trailing_break(list) {
                break;
            }
            list.remove(last - 1);
            recs.remove(last - 1);
        }
    }
    Some(trailing_skip)
}

/// A segment's style inside a block whose own style is `base` (a heading's
/// `\bfseries`, a running head's `\slshape`): the block's weight unless the
/// segment is `\normalfont`/`\mdseries`, its shape added, and the segment's
/// family when it selects one.
fn merge_style(base: TextStyle, s: TextStyle) -> TextStyle {
    TextStyle {
        bold: s.bold || (base.bold && !s.medium),
        italic: s.italic || base.italic,
        size_cpt: s.size_cpt,
        medium: s.medium,
        slanted: s.slanted || base.slanted,
        caps: s.caps || base.caps,
        family: if s.family != crate::nfss::FamilyKind::Rm { s.family } else { base.family },
        // Verbatim is a property of the text, so it never comes from the
        // block's base style; it is carried, not merged away.
        literal: s.literal,
        undefined: s.undefined.or(base.undefined),
        color: s.color.or(base.color),
    }
}

/// A word style under the block's base style (as `AItem::Space` merges it).
fn merge_base(style: TextStyle, base: TextStyle) -> TextStyle {
    TextStyle {
        bold: style.bold || (base.bold && !style.medium),
        italic: style.italic || base.italic,
        size_cpt: style.size_cpt,
        medium: style.medium,
        slanted: style.slanted || base.slanted,
        caps: style.caps || base.caps,
        family: if style.family != crate::nfss::FamilyKind::Rm { style.family } else { base.family },
        literal: style.literal,
        undefined: style.undefined.or(base.undefined),
        color: style.color.or(base.color),
    }
}

/// `text_builtins::LogoMetrics` from the `ec-lm*` TFMs pdfTeX sets the
/// logo with (T1 codes), the current face's quad/x-height, and the formula
/// fonts' `ε` box for `\LaTeXe`.
struct TfmLogoMetrics {
    current: Option<Rc<crate::tfm::Tfm>>,
    small: Option<Rc<crate::tfm::Tfm>>,
    size: f64,
    sf: f64,
    quad: f64,
    x_height: f64,
    /// Width and height of `\varepsilon` at text style, in points.
    epsilon: (f64, f64),
}

impl flashtex_compiler::text_builtins::LogoMetrics for TfmLogoMetrics {
    fn char_box(&self, font: flashtex_compiler::text_builtins::LogoFont, ch: char) -> flashtex_compiler::text_builtins::CharBox {
        use crate::ids::{Encoding, EncodingCode};
        use flashtex_compiler::text_builtins::{pt_to_sp, CharBox, LogoFont};
        let (tfm, size) = match font {
            LogoFont::MathItalic => {
                return CharBox { width: pt_to_sp(self.epsilon.0), height: pt_to_sp(self.epsilon.1), depth: 0, italic: 0 };
            }
            LogoFont::ScriptSize => (&self.small, self.sf),
            LogoFont::Current => (&self.current, self.size),
        };
        let m = tfm.as_ref().and_then(|t| EncodingCode::for_char(ch, Encoding::T1).and_then(|code| t.metrics(code.0)));
        match m {
            Some(m) => CharBox {
                width: pt_to_sp(crate::tfm::Tfm::pt(m.width, size)),
                height: pt_to_sp(crate::tfm::Tfm::pt(m.height, size)),
                depth: pt_to_sp(crate::tfm::Tfm::pt(m.depth, size)),
                italic: pt_to_sp(crate::tfm::Tfm::pt(m.italic, size)),
            },
            // No TFM (reported by `face`): Latin Modern's cap height stands
            // in; the boxes still land at their widths' positions.
            None => CharBox { width: 0, height: pt_to_sp(0.683 * size), depth: 0, italic: 0 },
        }
    }

    fn quad(&self) -> i32 {
        flashtex_compiler::text_builtins::pt_to_sp(self.quad)
    }

    fn x_height(&self) -> i32 {
        flashtex_compiler::text_builtins::pt_to_sp(self.x_height)
    }

    /// lmsy10's `\fontdimen16` (sub1, .15em) and `\fontdimen5` (.430555em)
    /// at the text size and the script symbol font's `\fontdimen19`
    /// (sub_drop, .05em at `\sf@size`): the formula `max` is decided by sub1
    /// at every class size.
    fn math_sub_params(&self) -> flashtex_compiler::text_builtins::MathSubParams {
        use flashtex_compiler::text_builtins::{pt_to_sp, MathSubParams};
        MathSubParams {
            sub1: pt_to_sp(0.15 * self.size),
            math_x_height: pt_to_sp(0.430555 * self.size),
            script_sub_drop: pt_to_sp(0.05 * self.sf),
        }
    }
}

pub(crate) fn design_size(family: Family, size: f64) -> u32 {
    match family {
        Family::Times => 10,
        Family::LatinModern | Family::ComputerModern => {
            if size < 8.5 {
                8
            } else if size < 11.0 {
                10
            } else if size < 15.0 {
                12
            } else {
                17
            }
        }
    }
}

fn seg_span(seg: &adapter::Segment) -> Option<Span> {
    let first = seg.chars.first()?;
    let last = seg.chars.last()?;
    Some(Span::in_document(first.document, first.start.min(last.start), first.end.max(last.end)))
}

/// A display's equation number (see `Context::number_box`): word boxes
/// with their x offsets inside the number, and the number's dimensions.
struct NumberBox {
    pieces: Vec<(pl::GlyphRun, usize, f64)>,
    width: f64,
    height: f64,
    depth: f64,
}

fn math_run(root: &ml::MathBox, size: f64, span: Span) -> pl::GlyphRun {
    pl::GlyphRun {
        font: MATH_SENTINEL,
        size,
        glyphs: Vec::new(),
        width: root.width,
        height: root.height,
        depth: root.depth,
        source: span.start..span.end,
    }
}

/// Compiler math list -> math-layout list. Symbols are single characters
/// with plain.tex's default classification; an unsupported `\command` the
/// compiler kept literally is spelled out as ordinary atoms. `\text`
/// arguments are dropped here (see [`convert_math_with`]).
pub fn convert_math(list: &flashtex_compiler::math::MathList) -> ml::MathList {
    convert_math_with(list, &mut crate::mathtext::TextSink::default())
}

/// [`convert_math`] collecting `\text{...}` arguments into `sink`, which
/// hands back the ordinary atom standing for each run (compiler pin
/// `887bf21` carries `Nucleus::Text` on main; the `compiler-text-nucleus`
/// feature is kept as a no-op for existing build invocations).
pub fn convert_math_with(list: &flashtex_compiler::math::MathList, sink: &mut crate::mathtext::TextSink) -> ml::MathList {
    convert_math_fenced(list, sink, &|_| None)
}

/// Which fence, if any, a delimiter atom was introduced by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fence {
    Left,
    Right,
}

/// The fence a delimiter atom whose span starts at `at` was introduced by:
/// the compiler pairs `\left`/`\right` but emits each delimiter as a plain
/// symbol, so the fence is re-derived from the source. Since pin `87df3e4a`
/// the delimiter's span starts at the control word itself (older pins
/// started it at the delimiter character, with the control word before).
pub fn fence_of(text: &str, at: usize) -> Option<Fence> {
    let rest = text.get(at..)?;
    for (word, fence) in [("\\left", Fence::Left), ("\\right", Fence::Right)] {
        if let Some(after) = rest.strip_prefix(word) {
            // `\leftarrow` is not a fence: the control word must end here.
            if !after.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
                return Some(fence);
            }
        }
    }
    fence_before(text, at)
}

/// Whether the bytes of `text` before offset `at` end in `\left` or
/// `\right` (spaces between the control word and the delimiter allowed).
pub fn fence_before(text: &str, at: usize) -> Option<Fence> {
    let before = text.get(..at)?.trim_end_matches([' ', '\t', '\n', '\r']);
    for (word, fence) in [("\\left", Fence::Left), ("\\right", Fence::Right)] {
        if let Some(stem) = before.strip_suffix(word) {
            // `\\left` itself (an escaped backslash) is not a control word.
            let escaped = stem.chars().rev().take_while(|c| *c == '\\').count() % 2 == 1;
            if !escaped {
                return Some(fence);
            }
        }
    }
    None
}

/// The math style a formula opening with `\displaystyle`, `\textstyle`,
/// `\scriptstyle` or `\scriptscriptstyle` is set in, re-read from the control
/// word at the atom's span like [`class_override_of`].
///
/// A style switch is not a command with an argument: it changes the style for
/// the rest of the enclosing group (TeX §1171, `\mathchoice`-free). The pinned
/// compiler does not model that at all — all four switches, and `\nonumber`,
/// `\notag` and `\middle` with them, are one zero-width `Nucleus::Space` atom
/// (`crates/compiler/src/math.rs`), which the pipeline then skips. So
/// `$\displaystyle\sum_{n=1}^{\infty}x_n$` was laid out in *text* style: the
/// limits sat beside the operator as scripts instead of above and below it,
/// and the box was 8 pt shorter than pdfTeX's. That is a vertical defect as
/// much as a horizontal one, because a short box never trips TeX's interline
/// rule (`\baselineskip - \prevdepth - height < \lineskiplimit` -> `\lineskip`,
/// §679), so the following baseline stayed a plain `\baselineskip` away and
/// every later line in the document was that much too high.
///
/// Only a switch that is the formula's *first* atom is honoured, which is the
/// case where it governs the whole formula and nothing else — the idiom in
/// every corpus use (`$\displaystyle\int_0^{\pi/2}\dots$`). A switch in the
/// middle of a list, or inside a grid cell or sub-formula, still needs the
/// compiler to emit an atom for it (math-layout is ready: it already has
/// `Nucleus::Styled`, laid out at `layout.rs:345`).
pub fn leading_style_switch(list: &flashtex_compiler::math::MathList, texts: &[&str]) -> Option<ml::Style> {
    let a = list.atoms.first()?;
    if a.superscript.is_some() || a.subscript.is_some() {
        return None;
    }
    let flashtex_compiler::math::Nucleus::Space { em, .. } = a.nucleus else { return None };
    if em != 0.0 {
        return None;
    }
    style_switch_of(texts.get(a.span.document.0).copied().unwrap_or(""), a.span.start)
}

/// The style a style-switch control word at byte `at` of `text` selects.
pub fn style_switch_of(text: &str, at: usize) -> Option<ml::Style> {
    let rest = text.get(at..)?.strip_prefix('\\')?;
    let word_len = rest.chars().take_while(|c| c.is_ascii_alphabetic()).map(char::len_utf8).sum::<usize>();
    Some(match &rest[..word_len] {
        "displaystyle" => ml::Style::DISPLAY,
        "textstyle" => ml::Style::TEXT,
        "scriptstyle" => ml::Style::SCRIPT,
        "scriptscriptstyle" => ml::Style::SCRIPT_SCRIPT,
        _ => return None,
    })
}

/// [`convert_math_with`] with `fence` telling which delimiter atoms follow a
/// `\left`/`\right`; matched pairs become math-layout `Delimited` atoms
/// (Appendix G Rule 19: sized to the body, `Inner` class). An unmatched
/// fence stays a plain symbol, as the compiler already reports it.
pub fn convert_math_fenced(list: &flashtex_compiler::math::MathList, sink: &mut crate::mathtext::TextSink, fence: &dyn Fn(&Span) -> Option<Fence>) -> ml::MathList {
    // No source to read, so no source-derived fact: no fence, no forced
    // class, no operator limits, and no run shown to be a whole run of math
    // characters (so no italic correction).
    convert_math_classed(list, sink, fence, &|_| None, &|_| None, &|_| false, &|_| None, &|_| None)
}

/// Maps the compiler's own atom class onto math-layout's.
#[cfg(feature = "math-class-override")]
pub fn ml_class(class: flashtex_compiler::math::AtomClass) -> ml::AtomClass {
    use flashtex_compiler::math::AtomClass as C;
    match class {
        C::Ord => ml::AtomClass::Ord,
        C::Op => ml::AtomClass::Op,
        C::Bin => ml::AtomClass::Bin,
        C::Rel => ml::AtomClass::Rel,
        C::Open => ml::AtomClass::Open,
        C::Close => ml::AtomClass::Close,
        C::Punct => ml::AtomClass::Punct,
        C::Inner => ml::AtomClass::Inner,
    }
}

/// The atom class a `\mathbin`/`\mathrel`/`\mathord`/`\mathop`/`\mathopen`/
/// `\mathclose`/`\mathpunct` command, or `\bot`/`\bigtriangleup`, forces on
/// the atom whose span starts at `at`, re-read from the control word at the
/// span like [`fence_of`].
///
/// This is the fallback for a pinned compiler that keeps
/// `MathAtom::class_override` crate-private (pin `d416472a` does). It is
/// lossy by construction -- one control word can carry two classes, as
/// `\colon` does between its kernel and amsmath definitions -- so with the
/// `math-class-override` feature the atom's own field is used instead.
pub fn class_override_of(text: &str, at: usize) -> Option<ml::AtomClass> {
    let rest = text.get(at..)?.strip_prefix('\\')?;
    let word_len = rest.chars().take_while(|c| c.is_ascii_alphabetic()).map(char::len_utf8).sum::<usize>();
    Some(match &rest[..word_len] {
        "mathbin" => ml::AtomClass::Bin,
        "mathrel" => ml::AtomClass::Rel,
        "mathord" => ml::AtomClass::Ord,
        "mathop" => ml::AtomClass::Op,
        "mathopen" => ml::AtomClass::Open,
        "mathclose" => ml::AtomClass::Close,
        "mathpunct" => ml::AtomClass::Punct,
        "bot" => ml::AtomClass::Ord,
        "bigtriangleup" => ml::AtomClass::Bin,
        // The *kernel's* `\colon` (`fontmath.ltx` 400,
        // `\DeclareMathSymbol{\colon}{\mathpunct}{operators}{"3A}`), which
        // sets a bare `:` -- a relation -- as punctuation. amsmath's `\colon`
        // is Ord and reaches this same control word, so this row is right
        // only for a document that has not loaded amsmath; the
        // `math-class-override` feature reads the atom instead and is right
        // for both.
        "colon" => ml::AtomClass::Punct,
        // amsfonts' dashed arrows: a `\mathrel` group of msam pieces.
        "dashrightarrow" | "dasharrow" | "dashleftarrow" => ml::AtomClass::Rel,
        _ => return None,
    })
}

/// The named operators of the LaTeX kernel's "Log-like functions"
/// (`latex.ltx` 15523-15556), and the limit placement each is declared with.
///
/// The ten defined as a bare `\mathop{\operator@font ...}` -- no `\nolimits`
/// after it -- keep TeX's default `\displaylimits`: limits over and under in
/// display style, scripts beside in text style. The rest are declared
/// `\mathop{...}\nolimits` and keep their scripts beside them at every style.
/// `\sgn` is not a kernel command at all; the compiler accepts it anyway, and
/// the amsmath spelling everyone writes for it (`\DeclareMathOperator{\sgn}`,
/// no star) is `\nolimits`, so that is the row it gets here.
const NAMED_OPERATORS: &[(&str, ml::Limits)] = &[
    ("lim", ml::Limits::DisplayLimits),
    ("liminf", ml::Limits::DisplayLimits),
    ("limsup", ml::Limits::DisplayLimits),
    ("max", ml::Limits::DisplayLimits),
    ("min", ml::Limits::DisplayLimits),
    ("sup", ml::Limits::DisplayLimits),
    ("inf", ml::Limits::DisplayLimits),
    ("det", ml::Limits::DisplayLimits),
    ("gcd", ml::Limits::DisplayLimits),
    ("Pr", ml::Limits::DisplayLimits),
    ("sin", ml::Limits::NoLimits),
    ("cos", ml::Limits::NoLimits),
    ("tan", ml::Limits::NoLimits),
    ("cot", ml::Limits::NoLimits),
    ("sec", ml::Limits::NoLimits),
    ("csc", ml::Limits::NoLimits),
    ("arcsin", ml::Limits::NoLimits),
    ("arccos", ml::Limits::NoLimits),
    ("arctan", ml::Limits::NoLimits),
    ("sinh", ml::Limits::NoLimits),
    ("cosh", ml::Limits::NoLimits),
    ("tanh", ml::Limits::NoLimits),
    ("coth", ml::Limits::NoLimits),
    ("log", ml::Limits::NoLimits),
    ("ln", ml::Limits::NoLimits),
    ("lg", ml::Limits::NoLimits),
    ("exp", ml::Limits::NoLimits),
    ("deg", ml::Limits::NoLimits),
    ("dim", ml::Limits::NoLimits),
    ("ker", ml::Limits::NoLimits),
    ("arg", ml::Limits::NoLimits),
    ("hom", ml::Limits::NoLimits),
    ("sgn", ml::Limits::NoLimits),
];

/// The limit placement of the named operator (`\lim`, `\sin`, `\max`, ...)
/// whose control word starts at `at`, or `None` when the atom at that span
/// did not come from one.
///
/// The compiler turns every one of them into an upright [`Nucleus::Text`] run
/// (`text_atom(operator, span)`) and keeps neither TeX's `\mathop` class nor
/// the `\limits`/`\nolimits`/`\displaylimits` switch that may follow: its
/// parser drops those switches without producing an atom, so that a following
/// script still attaches to the operator. Both facts are therefore re-read
/// from the source at the atom's span, exactly as [`fence_of`] and
/// [`class_override_of`] do for the other things a pinned compiler does not
/// carry.
///
/// Reading the *control word* rather than matching the letters is what keeps
/// `\mathrm{lim}` out: it also arrives as `Nucleus::Text("lim")`, but it is an
/// ordinary atom in TeX and its span starts at `\mathrm`.
pub fn operator_limits_of(text: &str, at: usize) -> Option<ml::Limits> {
    let rest = text.get(at..)?.strip_prefix('\\')?;
    let word_len = rest.chars().take_while(|c| c.is_ascii_alphabetic()).map(char::len_utf8).sum::<usize>();
    let declared = NAMED_OPERATORS.iter().find(|(name, _)| *name == &rest[..word_len]).map(|(_, limits)| *limits)?;
    // `\lim\limits_{n}`, `\max\nolimits_{k}`: the switch overrides the
    // declaration (TeXbook p. 144). Only an immediately following switch
    // counts, as in TeX, where it is read by `\mathop`'s scanner.
    let after = rest[word_len..].trim_start_matches([' ', '\t', '\r', '\n']);
    for (switch, limits) in [
        ("nolimits", ml::Limits::NoLimits),
        ("limits", ml::Limits::Limits),
        ("displaylimits", ml::Limits::DisplayLimits),
    ] {
        match after.strip_prefix('\\').and_then(|a| a.strip_prefix(switch)) {
            Some(tail) if !tail.starts_with(|c: char| c.is_ascii_alphabetic()) => return Some(limits),
            _ => {}
        }
    }
    Some(declared)
}

/// Whether the `Nucleus::Text` atom whose span starts at `at` is a *complete*
/// run of math characters, and so keeps the italic correction of its last
/// character (tex.web §752), re-read from the control word at the span like
/// [`fence_of`].
///
/// The compiler spells three different things `Nucleus::Text`, and only one
/// of them takes the correction:
///
///  1. **A whole run of math characters.** `\lim` and the rest of the
///     log-like functions, `\mathrm{...}`, `\bmod`/`\mod`, and `\pmod`'s
///     `(mod` and `)`. These are `\operator@font` characters of the
///     `operators` family and the run ends where the atom ends, so §752
///     leaves the last character its `delta`: `$\lim$` and `$\mathrm{lim}$`
///     are 16.3773 pt where `$\text{lim}$` is 16.31999 pt.
///
///  2. **An `\hbox`.** amsmath's `\text`, and its `\tag`, whose label is
///     `\maketag@@@#1 -> \hbox{\m@th\normalfont#1}` (amsmath.sty 1211). An
///     hbox is not a run of math characters and never had a correction.
///
///  3. **A *fragment* of a longer run**, which is the case worth stating
///     because it looks exactly like (1) and must behave like (2). The
///     pinned compiler emits a multi-character siunitx unit as one
///     `Nucleus::Text` per character (`siunitx.rs` `upright`: "One character
///     is one upright run; several are a boxed group of one-character
///     runs"), so `\unit{\katal}` arrives as three runs `k`, `a`, `t` where
///     TeX has a single `\mathrm{kat}`. Correcting each of them would add an
///     italic correction *inside* a word, which is precisely what §752's
///     `math_text_char` rule exists to prevent -- and it is measurable:
///     `12-unit-derived-c.tex` moved 1.255 bp when this function was written
///     the other way round, as an exclusion list that let every unit through.
///
/// So this is a whitelist of the control words that produce a whole run, not
/// a blacklist of the ones that do not. A construct that reaches
/// `Nucleus::Text` by some other route keeps today's uncorrected geometry
/// rather than silently acquiring a correction that may be wrong; the known
/// under-application is a `\operatorname{...}` body, whose runs are built
/// from the individual characters and so carry no control word at their span.
pub fn math_text_keeps_italic(text: &str, at: usize) -> bool {
    let Some(rest) = text.get(at..).and_then(|r| r.strip_prefix('\\')) else {
        return false;
    };
    let word_len = rest.chars().take_while(|c| c.is_ascii_alphabetic()).map(char::len_utf8).sum::<usize>();
    let word = &rest[..word_len];
    // `\mathrm{lim}` is one `text_atom` of all the letters, so it is whole;
    // `\bmod`/`\mod`/`\pmod` likewise put `mod` (and `(mod`, `)`) in runs of
    // their own. Every log-like function is a whole word by construction.
    matches!(word, "mathrm" | "bmod" | "mod" | "pmod") || NAMED_OPERATORS.iter().any(|(name, _)| *name == word)
}

/// The two log-like functions the kernel defines with a thin space inside
/// them, split at it, re-read from the control word at the span like
/// [`fence_of`].
///
/// ```text
/// \DeclareRobustCommand\limsup{\mathop{\operator@font lim\,sup}}   % latex.ltx 15528
/// \DeclareRobustCommand\liminf{\mathop{\operator@font lim\,inf}}   % latex.ltx 15529
/// ```
///
/// Every other one of the thirty-odd log-like functions is a single word, so
/// this is the whole list rather than a sample of it. The compiler has no
/// glue inside a named operator and emits one `Nucleus::Text("limsup")`, so
/// the space is re-derived here and the operator becomes a three-atom list.
///
/// Two consequences follow from the split, both of them TeX's:
///
///  - `lim` ends a run of math characters (the next node is glue, not a math
///    char of the same family, so §753's `make_ord` never demotes its `m` to
///    a `math_text_char`), which means it keeps its own italic correction as
///    well. pdfTeX's `\limsup` box is
///    `l i m \kern0.05731 \glue 1.99997 s u p`, so the gap between `m` and
///    `s` is 2.05728 pt and not the 1.99997 pt of the glue alone.
///  - `sup` and `inf` likewise end runs, so `\liminf` takes `f`'s 0.84708 pt
///    correction at the end -- pdfTeX's box is 32.60657 pt and closes with
///    `\kern0.84708`.
pub fn operator_thin_space_split(text: &str, at: usize) -> Option<(&'static str, &'static str)> {
    let rest = text.get(at..)?.strip_prefix('\\')?;
    let word_len = rest.chars().take_while(|c| c.is_ascii_alphabetic()).map(char::len_utf8).sum::<usize>();
    match &rest[..word_len] {
        "limsup" => Some(("lim", "sup")),
        "liminf" => Some(("lim", "inf")),
        _ => None,
    }
}

/// The dot that a `\ldots`/`\cdots`-family control word at `at` sets three of
/// inside a `\mathinner`, re-read from the control word at the span like
/// [`fence_of`].
///
/// Both families are `\mathinner{\dotp\dotp\dotp}` of a **punctuation** atom
/// (plain.tex 360-361, `\mathellipsis` in `latex.ltx`), which is two separate
/// facts about spacing that an ordinary atom does not have:
///
///  - Punct against Punct is a thin space, so the three dots sit 3mu apart;
///  - `\mathinner` is the Inner class, so the group takes a thin space
///    against the Ord on each side of it.
///
/// The dots differ only in which family the glyph comes from, and
/// `flashtex_math_layout::cm::symbol_slot` already maps both the way plain
/// TeX's `\mathcode`s do -- `.` to `letters` (math italic) slot `"3A`, the
/// `\ldotp` glyph, and `U+22C5` to `symbols` (cmsy) slot `"01`, `\cdotp`.
/// So this is about the atoms, not the fonts: the two periods are within
/// 0.0001 bp of each other in width, and the whole of the difference from
/// what the pinned compiler produces is four thin spaces.
///
/// Measured with TeX Live 2025 pdflatex under the harness preamble
/// (`\showbox`):
///
/// ```text
/// \setbox0=\hbox{$a\ldots b$}          \setbox0=\hbox{$a\cdots b$}
/// .\OML/lmm/m/it/12 a                  .\OML/lmm/m/it/12 a
/// .\glue(\thinmuskip) 1.99997          .\glue(\thinmuskip) 1.99997
/// .\hbox(1.16666+0.0)x13.7915          .\hbox(5.33334+0.0)x13.99997
/// ..\OML/lmm/m/it/12 :                 ..\OMS/lmsy/m/n/12 ^^A
/// ..\glue(\thinmuskip) 1.99997         ..\glue(\thinmuskip) 1.99997
/// ..\OML/lmm/m/it/12 :                 ..\OMS/lmsy/m/n/12 ^^A
/// ..\glue(\thinmuskip) 1.99997         ..\glue(\thinmuskip) 1.99997
/// ..\OML/lmm/m/it/12 :                 ..\OMS/lmsy/m/n/12 ^^A
/// .\glue(\thinmuskip) 1.99997          .\glue(\thinmuskip) 1.99997
/// .\OML/lmm/m/it/12 b                  .\OML/lmm/m/it/12 b
/// ```
///
/// (`:` and `^^A` are how `\showbox` names slots `"3A` and `"01`.) The
/// grouping of the eight commands is the compiler's own
/// (`crates/compiler/src/math.rs`): amsmath's `\dotsc`/`\dotso` are low dots
/// and `\dotsb`/`\dotsm`/`\dotsi` are centred ones. Bare `\dots` follows the
/// kernel's `\mathellipsis` and is low, which is what the compiler already
/// assumes; amsmath makes `\dots` guess from what follows it, and neither
/// side models that.
///
/// `\vdots` and `\ddots` are deliberately **not** here. They are not runs of
/// dots at all but vertical box constructions over *text*-font periods --
/// `\vdots` is a `\vbox` of three `\hbox{.}` at `\baselineskip` 2.83334 over
/// a `\kern 6.0`, and `\ddots` an Inner hbox of three `\hbox{.}` shifted
/// -7.0/-4.0/-1.0 between kerns of 0.66666 and 1.33331 -- and math-layout has
/// no atom that builds a vbox, so they need their own change.
pub fn math_ellipsis_of(text: &str, at: usize) -> Option<char> {
    let rest = text.get(at..)?.strip_prefix('\\')?;
    let word_len = rest.chars().take_while(|c| c.is_ascii_alphabetic()).map(char::len_utf8).sum::<usize>();
    Some(match &rest[..word_len] {
        // `\ldotp`, `\mathcode`"013A: the math italic period.
        "ldots" | "dots" | "dotsc" | "dotso" => '.',
        // `\cdotp`, cmsy `"01`: the centred dot.
        "cdots" | "dotsb" | "dotsm" | "dotsi" => '\u{22C5}',
        _ => return None,
    })
}

/// [`convert_math_fenced`] with `class` giving the forced class of a
/// `Group` (`\mathbin{...}`) or class-overridden symbol atom at a span, and
/// `op_limits` the limit placement of a named operator at a span
/// ([`operator_limits_of`]).
pub fn convert_math_classed(
    list: &flashtex_compiler::math::MathList,
    sink: &mut crate::mathtext::TextSink,
    fence: &dyn Fn(&Span) -> Option<Fence>,
    class: &dyn Fn(&flashtex_compiler::math::MathAtom) -> Option<ml::AtomClass>,
    op_limits: &dyn Fn(&Span) -> Option<ml::Limits>,
    text_italic: &dyn Fn(&Span) -> bool,
    text_split: &dyn Fn(&Span) -> Option<(&'static str, &'static str)>,
    ellipsis: &dyn Fn(&Span) -> Option<char>,
) -> ml::MathList {
    use flashtex_compiler::math::{DelimiterRole, Nucleus as N};
    // Open fences: (left delimiter, atoms converted since it, its span).
    let mut stack: Vec<(Option<char>, Vec<ml::Atom>, Span)> = Vec::new();
    let mut atoms = Vec::new();
    for a in &list.atoms {
        let sub = |l: &flashtex_compiler::math::MathList, sink: &mut crate::mathtext::TextSink| convert_math_classed(l, sink, fence, class, op_limits, text_italic, text_split, ellipsis);
        let mut out: Vec<ml::Atom> = match &a.nucleus {
            // `\ldots`/`\cdots` and the amsmath spellings: TeX's
            // `\mathinner{\ldotp\ldotp\ldotp}` (`math_ellipsis_of`). The
            // compiler flattens both to three characters with no class and no
            // spacing -- `Nucleus::Text("...")` for the low dots and
            // `Nucleus::Symbol("⋅⋅⋅")` for the centred ones -- so the atoms
            // are rebuilt here: three Punct dots (3mu apart) inside one Inner
            // atom (a thin space against each neighbour).
            N::Text(_) | N::Symbol(_) if ellipsis(&a.span).is_some() => {
                let dot = ellipsis(&a.span).expect("checked by the guard");
                let dots = (0..3).map(|_| ml::Atom::new(ml::AtomClass::Punct, ml::Nucleus::Symbol(dot))).collect();
                vec![ml::Atom::new(ml::AtomClass::Inner, ml::Nucleus::List(ml::MathList::new(dots)))]
            }
            // `\lim`, `\sin`, `\max`, ...: TeX's `\mathop` of upright roman
            // text (`latex.ltx` 15523-15556), so an `Op` atom -- which is both
            // the thin space the Op class contributes on each side and, for
            // the ten declared without `\nolimits`, Rule 13a limits over and
            // under the word in display style instead of scripts beside it.
            // Every other `Nucleus::Text` (`\text{...}`, `\mathrm{K}`, a
            // grid or `\boxed` handle) is an hbox in math, which TeX §1076
            // makes an ordinary atom.
            //
            // Whether the run is a whole run of math characters is a second,
            // independent fact the compiler's `Nucleus::Text` does not carry
            // (`math_text_keeps_italic`), and it decides the italic
            // correction of the run's last character: `$\lim$` and
            // `$\mathrm{lim}$` are 16.3773 pt, `$\text{lim}$` 16.31999 pt.
            // `\mathrm{K}`: a group holding one ordinary character is that
            // math character of family 0 (TeX §1186), so `make_ord` joins it
            // to a neighbouring one (`\mathrm{f}\mathrm{i}` is the fi
            // ligature, `\mathrm{A}\mathrm{V}` kerned) and its scripts sit
            // as on a character; the provider boxes it from the roman TFM.
            // Only letters and digits: they are the variable-family math
            // codes `\mathrm` moves to family 0.
            #[cfg(feature = "math-font-kerns")]
            N::Text(text)
                if text_italic(&a.span)
                    && op_limits(&a.span).is_none()
                    && text_split(&a.span).is_none()
                    && matches!(text.as_bytes(), [c] if c.is_ascii_alphanumeric()) =>
            {
                vec![ml::Atom::new(ml::AtomClass::Ord, ml::Nucleus::TextChar(char::from(text.as_bytes()[0])))]
            }
            N::Text(text) => {
                // `\limsup`/`\liminf` are `lim\,sup` and `lim\,inf`: one
                // operator whose nucleus is a list of two math-character runs
                // with 3mu between them (`operator_thin_space_split`).
                let mut atom = match text_split(&a.span) {
                    Some((head, tail)) => {
                        let parts = vec![sink.atom_corrected(head), ml::Atom::glue(3.0, 0.0), sink.atom_corrected(tail)];
                        ml::Atom::new(ml::AtomClass::Ord, ml::Nucleus::List(ml::MathList::new(parts)))
                    }
                    None if text_italic(&a.span) => sink.atom_corrected(text),
                    None => sink.atom(text),
                };
                if let Some(limits) = op_limits(&a.span) {
                    atom.class = ml::AtomClass::Op;
                    atom.limits = limits;
                }
                vec![atom]
            }
            // Glue inside a sub-formula (`\operatorname*{arg\,max}`, `\;`
            // inside `\left...\right`): math-layout's `Glue` atom, which
            // takes no part in atom spacing, like TeX's glue node. Top-level
            // glue is still split out by `split_at_spaces`.
            #[cfg(feature = "amsmath-inline")]
            N::Space { em, font_em } if a.superscript.is_none() && a.subscript.is_none() => match (*font_em, sink.text_quad) {
                // `\quad`: text-font ems, the same at every math style.
                (true, Some((quad, _))) => vec![ml::Atom::glue(0.0, em * quad)],
                _ => vec![ml::Atom::glue(em * 18.0, 0.0)],
            },
            // amsmath `\genfrac` family (`\dfrac`, `\tfrac`, `\binom`, ...):
            // a Rule 15e fraction with its delimiters, in a group (Ord),
            // under the explicit style when one is given.
            #[cfg(feature = "amsmath-inline")]
            N::GenFraction { numerator, denominator, thickness_pt, left, right, style } => {
                let one = |s: &str| {
                    let mut it = s.chars();
                    match (it.next(), it.next()) {
                        (Some(c), None) => Some(c),
                        _ => None,
                    }
                };
                let frac = ml::Atom::genfrac(sub(numerator, sink), sub(denominator, sink), *thickness_pt, one(left), one(right));
                match style {
                    Some(s) => vec![ml::Atom::styled(ml_style(*s), ml::MathList::new(vec![frac]))],
                    None => vec![frac],
                }
            }
            #[cfg(feature = "amsmath-inline")]
            N::Phantom { body, horizontal, vertical } => vec![ml::Atom::phantom(sub(body, sink), *horizontal, *vertical)],
            // amsopn `\qopname`: `\mathop{\operator@font ...}\limits` or `\nolimits`.
            #[cfg(feature = "amsmath-inline")]
            N::Operator { body, limits } => vec![ml::Atom::new(ml::AtomClass::Op, ml::Nucleus::List(sub(body, sink))).with_limits(if *limits { ml::Limits::Limits } else { ml::Limits::NoLimits })],
            #[cfg(feature = "amsmath-inline")]
            N::SubArray { rows, align } => vec![ml::Atom::subarray(rows.iter().map(|r| sub(r, sink)).collect(), *align)],
            // amsmath `\ext@arrow#1#2#3#4` kerns and `\arrowfill@` pieces:
            // `\xrightarrow` 0359 `\relbar\relbar\rightarrow`, `\xleftarrow`
            // 3095 `\leftarrow\relbar\relbar` (amsmath.sty 977-978,
            // 1027-1028), mathtools `\xleftrightarrow` 3399
            // `\leftarrow\relbar\rightarrow` (mathtools.sty 323-326).
            #[cfg(feature = "amsmath-inline")]
            N::ExtArrow { arrow, above, below } => {
                use flashtex_compiler::math::ExtArrow as X;
                let (pieces, kerns) = match arrow {
                    X::Right => (['-', '-', '\u{2192}'], [0.0, 3.0, 5.0, 9.0]),
                    X::Left => (['\u{2190}', '-', '-'], [3.0, 0.0, 9.0, 5.0]),
                    X::LeftRight => (['\u{2190}', '-', '\u{2192}'], [3.0, 3.0, 9.0, 9.0]),
                };
                vec![ml::Atom::ext_arrow(pieces, kerns, sub(above, sink), sub(below, sink))]
            }
            // `\quad`/`\qquad` (compiler `Space { em }`): TeX glue in the
            // math list. math-layout has no kern/glue atom, so the glue is
            // dropped (inter-atom spacing across it is what TeX's mlist_to_hlist
            // does too, since glue does not reset r_type) and reported once per
            // formula by `math_box` as a typed math_limitation.
            N::Space { .. } => continue,
            // `\left`/`\right` (pin `d416472a`: the compiler now pairs them
            // itself and emits each as a `SizedDelimiter` with the `Left`/
            // `Right` role; an empty glyph is the null delimiter `.`), matched
            // here exactly like the source-derived fences below.
            N::SizedDelimiter { glyph, role: DelimiterRole::Left, .. } => {
                stack.push((glyph.chars().next(), Vec::new(), a.span));
                continue;
            }
            N::SizedDelimiter { glyph, role: DelimiterRole::Right, .. } if !stack.is_empty() => {
                let (left, body, left_span) = stack.pop().expect("checked non-empty");
                vec![fenced(left, glyph.chars().next(), body, left_span, a.span)]
            }
            // amsmath `\big(`..`\Bigg]` (`\bBigg@`): math-layout's
            // `BigDelimiter` at 1/1.5/2/2.5 `\big@size` (compiler scale
            // 1.2/1.8/2.4/3.0), in the command's class.
            #[cfg(feature = "amsmath-inline")]
            N::SizedDelimiter { glyph, role, scale } if !matches!(role, DelimiterRole::Left | DelimiterRole::Right) => {
                let class = match role {
                    DelimiterRole::Open => ml::AtomClass::Open,
                    DelimiterRole::Close => ml::AtomClass::Close,
                    DelimiterRole::Rel => ml::AtomClass::Rel,
                    _ => ml::AtomClass::Ord,
                };
                // Which of the two `\big` rules applies is a fact about the
                // package list, not about the formula: amsmath's `\bBigg@`
                // when amsmath is loaded, the LaTeX kernel's fixed `\vbox`
                // lengths when it is not. The compiler's `scale` is
                // amsmath's 1.2/1.8/2.4/3.0, so `scale / 1.2` is its
                // 1/1.5/2/2.5 factor; `BigSizing::kernel_for_factor` maps
                // that same factor onto the kernel's 8.5/11.5/14.5/17.5pt,
                // so the two rules stay in one place rather than the
                // constants being copied into this crate.
                let factor = scale / 1.2;
                let delim = glyph.chars().next();
                vec![if sink.amsmath {
                    ml::Atom::big_delimiter(class, delim, factor)
                } else {
                    let ml::BigSizing::Kernel { pt } = ml::BigSizing::kernel_for_factor(factor) else {
                        unreachable!("kernel_for_factor always returns BigSizing::Kernel")
                    };
                    ml::Atom::big_delimiter_kernel(class, delim, pt)
                }]
            }
            // `\big(`..`\Bigg]` (and an unmatched `\right`): math-layout has
            // no fixed-step delimiter atom, so the glyph is set at text size
            // with plain TeX's class for the command (`\bigl` Open, `\bigr`
            // Close, `\bigm` Rel, bare `\big` Ord); `math_approximations`
            // reports the dropped scale once per formula.
            N::SizedDelimiter { glyph, role, .. } => match glyph.chars().next() {
                Some(c) => {
                    let class = match role {
                        DelimiterRole::Open | DelimiterRole::Left => ml::AtomClass::Open,
                        DelimiterRole::Close | DelimiterRole::Right => ml::AtomClass::Close,
                        DelimiterRole::Rel => ml::AtomClass::Rel,
                        DelimiterRole::Ord => ml::AtomClass::Ord,
                    };
                    vec![ml::Atom::new(class, ml::Nucleus::Symbol(c))]
                }
                None => vec![ml::Atom::new(ml::AtomClass::Ord, ml::Nucleus::Empty)],
            },
            // `\mathbin{...}`/`\mathrel{...}`/...: one atom of the forced
            // class (TeXbook Chapter 17). The compiler keeps the class in a
            // crate-private field, so it is re-read from the command at the
            // atom's span (`class_override_of`); without a source the group
            // is ordinary.
            N::Group(body) => {
                let class = class(a).unwrap_or(ml::AtomClass::Ord);
                vec![ml::Atom::new(class, ml::Nucleus::List(sub(body, sink)))]
            }
            // amssymb/amsfonts symbols (compiler `MathAtom.ams_symbol`): one
            // atom of the declared class whose sentinel carries the msam/msbm
            // slot to the metrics providers.
            N::Symbol(_) if a.ams_symbol.is_some() => {
                use flashtex_compiler::amssymb::SymbolClass as C;
                let ams = a.ams_symbol.expect("checked");
                let class = match ams.class {
                    C::Ord => ml::AtomClass::Ord,
                    C::Bin => ml::AtomClass::Bin,
                    C::Rel => ml::AtomClass::Rel,
                    C::Open => ml::AtomClass::Open,
                    C::Close => ml::AtomClass::Close,
                };
                vec![ml::Atom::new(class, ml::Nucleus::Symbol(crate::mathfont::ams_sentinel(ams)))]
            }
            // Math-mode `\rule` (compiler `Nucleus::Rule`): math-layout has no
            // rule atom, so the box's width is kept as glue and nothing is
            // painted (reported by `math_approximations`). Font-relative
            // widths resolve against no font here (0).
            N::Rule(rule) => {
                let cx = flashtex_compiler::text_builtins::DimenContext::default();
                let width = flashtex_compiler::text_builtins::sp_to_pt(rule.width.resolve(&cx));
                vec![ml::Atom::glue(0.0, width)]
            }
            // `\mathsf{AB}`, `\mathtt`, `\mathit` (compiler: Unicode
            // mathematical alphanumerics): runs of one text-font alphabet go
            // to the text sink in that font (kerns, ligatures, last italic
            // correction); fraktur and any other character stay symbols.
            // A single character stays a math character (TeX §1186 unpacks
            // the one-Ord group): `TexMathMetrics` boxes it from the TFM.
            N::Symbol(s) if s.chars().count() > 1 && s.chars().any(|c| crate::mathalpha::classify(c).is_some_and(|(al, _)| al.text_key().is_some())) => {
                let mut parts: Vec<ml::Atom> = Vec::new();
                let mut run = String::new();
                let mut run_key = None;
                let flush = |run: &mut String, run_key: &mut Option<crate::nfss::FontKey>, parts: &mut Vec<ml::Atom>, sink: &mut crate::mathtext::TextSink| {
                    if let Some(key) = run_key.take() {
                        parts.push(sink.atom_in(run, key));
                    }
                    run.clear();
                };
                for c in s.chars() {
                    match crate::mathalpha::classify(c).and_then(|(al, letter)| al.text_key().map(|k| (k, letter))) {
                        Some((key, letter)) => {
                            if run_key != Some(key) {
                                flush(&mut run, &mut run_key, &mut parts, sink);
                                run_key = Some(key);
                            }
                            run.push(letter);
                        }
                        None => {
                            flush(&mut run, &mut run_key, &mut parts, sink);
                            parts.extend(symbol_atoms(c, None));
                        }
                    }
                }
                flush(&mut run, &mut run_key, &mut parts, sink);
                if parts.len() == 1 {
                    parts
                } else {
                    vec![ml::Atom::group(ml::MathList::new(parts))]
                }
            }
            N::Symbol(s) => {
                let mut chars = s.chars();
                let single = match (chars.next(), chars.next()) {
                    (Some(c), None) => Some(Some(c)),
                    (None, _) => Some(None),
                    _ => None,
                };
                match (single, fence(&a.span)) {
                    (Some(delim), Some(Fence::Left)) => {
                        stack.push((delim, Vec::new(), a.span));
                        continue;
                    }
                    (Some(delim), Some(Fence::Right)) if !stack.is_empty() => {
                        let (left, body, left_span) = stack.pop().expect("checked non-empty");
                        vec![fenced(left, delim, body, left_span, a.span)]
                    }
                    _ => match single {
                        Some(Some(c)) => match class(a) {
                            // `\bot` (Ord, same glyph as `\perp`) and
                            // `\bigtriangleup` (Bin, same glyph as `\triangle`).
                            Some(forced) => {
                                // mathtools defines `\vcentcolon` as
                                // `\mathrel{\mathop\ordinarycolon}`. The
                                // compiler marks exactly that atom as a
                                // `Symbol(":")` with a forced `Rel` class;
                                // kernel/amsmath `\colon` and a literal `:`
                                // do not carry this override. Keep the outer
                                // atom Rel for its spacing, and use the
                                // existing math-layout Op path for the
                                // glyph's real-bounds axis centring.
                                #[cfg(feature = "math-class-override")]
                                let vcentcolon = c == ':'
                                    && a.class_override
                                        == Some(flashtex_compiler::math::AtomClass::Rel);
                                #[cfg(not(feature = "math-class-override"))]
                                let vcentcolon = false;
                                let nucleus = if vcentcolon {
                                    ml::Nucleus::List(ml::MathList::new(vec![ml::Atom::op(c)]))
                                } else {
                                    ml::Nucleus::Symbol(c)
                                };
                                vec![ml::Atom::new(forced, nucleus)]
                            }
                            None => symbol_atoms(c, a.width_em),
                        },
                        Some(None) => vec![ml::Atom::new(ml::AtomClass::Ord, ml::Nucleus::Empty)],
                        None => {
                            // Multi-character symbol (e.g. a literal "\foo"): the
                            // characters as upright ordinary atoms.
                            vec![ml::Atom::group(ml::MathList::new(s.chars().map(ml::Atom::ord).collect()))]
                        }
                    },
                }
            }
            N::Fraction { numerator, denominator } => vec![ml::Atom::frac(sub(numerator, sink), sub(denominator, sink))],
            N::Radical(r) => vec![ml::Atom::sqrt(sub(r, sink))],
            // `\mathbf{...}` (fontmath.ltx OT1/cmr/bx/n): a run in the bold
            // roman text font; spaces in math take no part.
            N::Bold(text) => {
                use crate::mathalpha::{alphanumeric, MathAlphabet};
                let letters: String = text.chars().filter(|c| !c.is_whitespace()).collect();
                let mut chars = letters.chars();
                match (chars.next(), chars.next()) {
                    // One letter or digit: a math character (TeX §1186).
                    (Some(c), None) if alphanumeric(MathAlphabet::Bold, c).is_some() => {
                        symbol_atoms(alphanumeric(MathAlphabet::Bold, c).expect("checked"), None)
                    }
                    _ => vec![sink.atom_in(&letters, MathAlphabet::Bold.text_key().expect("a text alphabet"))],
                }
            }
            // `\overline`/`\underline` are Appendix G Rules 9/10 atoms;
            // `\boxed` uses the pipeline's existing framed-box placeholder
            // and rule substitution path.
            N::Framed { body, frame } => {
                use flashtex_compiler::math::Frame;
                let body = sub(body, sink);
                vec![match frame {
                    Frame::Over => ml::Atom::overline(body),
                    Frame::Under => ml::Atom::underline(body),
                    Frame::Box => sink.frame_atom(body, {
                        #[cfg(feature = "math-glyph-spans")]
                        {
                            math_tag(a.span)
                        }
                        #[cfg(not(feature = "math-glyph-spans"))]
                        {
                            ml::SourceTag::NONE
                        }
                    }),
                    // `\mathop{..}\limits` (`fontmath.ltx` 430-437): the
                    // compiler's scripts attach below as limits.
                    Frame::OverBrace => ml::Atom::brace(body, false),
                    Frame::UnderBrace => ml::Atom::brace(body, true),
                    // amsmath `\overarrow@`/`\underarrow@` over the
                    // `\arrowfill@` pieces (`amsmath.sty` 977-979).
                    arrow_frame => {
                        use flashtex_compiler::math::ExtArrow as X;
                        let pieces = match arrow_frame.arrow() {
                            Some(X::Left) => ['\u{2190}', '-', '-'],
                            Some(X::LeftRight) => ['\u{2190}', '-', '\u{2192}'],
                            _ => ['-', '-', '\u{2192}'],
                        };
                        ml::Atom::over_arrow(pieces, body, arrow_frame.is_under(), 1.3 * ams_ex(sink.body_size_pt))
                    }
                }]
            }
            // amsmath's `\overset{a}{b}` is `\mathop{b}\limits^{a}` wrapped
            // in the base's own class (`\binrel@`), which math-layout sets
            // exactly (Rule 13a limits).
            N::Stacked { base, over, under } => {
                let class = match base.atoms.as_slice() {
                    [only] if only.superscript.is_none() && only.subscript.is_none() => match &only.nucleus {
                        N::Symbol(s) if s.chars().count() == 1 => ml::mathlist::default_class(s.chars().next().expect("one char")).0,
                        _ => ml::AtomClass::Ord,
                    },
                    _ => ml::AtomClass::Ord,
                };
                let mut op = ml::Atom::new(ml::AtomClass::Op, ml::Nucleus::List(sub(base, sink))).with_limits(ml::Limits::Limits);
                op.superscript = over.as_ref().map(|l| sub(l, sink));
                op.subscript = under.as_ref().map(|l| sub(l, sink));
                vec![ml::Atom::new(class, ml::Nucleus::List(ml::MathList::new(vec![op])))]
            }
            // `\hat`/`\bar`/...: Rule 12 accents with the unicode-math
            // combining mark Latin Modern Math carries for each command
            // (`\widehat`/`\widetilde` use the same mark; the horizontal
            // variants are not read, so a wide base gets the plain one).
            // amsfonts' `\widehat`/`\widetilde` (`amsfonts.sty` 78-86): the cmex
            // chain up to 2em of the text font, msbm's extra-wide form past it.
            N::Accent { accent: accent @ (flashtex_compiler::math::Accent::WideHat | flashtex_compiler::math::Accent::WideTilde), body }
                if sink.amsfonts && sink.text_quad.is_some() =>
            {
                let wide = if *accent == flashtex_compiler::math::Accent::WideHat { "widehat@" } else { "widetilde@" };
                let threshold = 2.0 * sink.text_quad.map_or(0.0, |(q, _)| q);
                let base = sub(body, sink);
                match flashtex_compiler::amssymb::piece(wide) {
                    Some(ams) => vec![ml::Atom::measured_accent(accent_char(*accent), crate::mathfont::ams_sentinel(ams), threshold, base)],
                    None => vec![ml::Atom::accent(accent_char(*accent), base)],
                }
            }
            N::Accent { accent, body } => vec![ml::Atom::accent(accent_char(*accent), sub(body, sink))],
            // `array`/`cases`/matrix/`aligned` grids below the top level (a
            // top-level grid without scripts is split out by `grid_pieces`):
            // math-layout has no array atom, so the grid is a handle whose
            // box `mathtext::TextRunMetrics` lays out with `mathgrid` at the
            // size it is met — a `\vcenter` (Ord), or `\left...\right` around
            // it (Inner) for the fenced environments — so atom spacing,
            // Rule 19 delimiters, Rule 18 scripts, fractions and radicals
            // treat it as the box TeX builds.
            N::Matrix { rows, columns, left, right } => {
                let cells = rows.iter().map(|row| row.iter().map(|cell| sub(cell, sink)).collect()).collect();
                let atom_class = if left.is_empty() && right.is_empty() { ml::AtomClass::Ord } else { ml::AtomClass::Inner };
                vec![sink.grid_atom(atom_class, cells, columns, left, right, a.span)]
            }
        };
        // Every atom this compiler atom produced maps to its bytes unless a
        // more precise span was already given (a `\left...\right` pair).
        #[cfg(feature = "math-glyph-spans")]
        for atom in &mut out {
            if atom.tag.span.is_none() {
                atom.tag.span = math_tag(a.span).span;
            }
        }
        if let Some(last) = out.last_mut() {
            if let Some(sup) = &a.superscript {
                last.superscript = Some(sub(sup, sink));
            }
            if let Some(sb) = &a.subscript {
                last.subscript = Some(sub(sb, sink));
            }
        }
        match stack.last_mut() {
            Some((_, body, _)) => body.extend(out),
            None => atoms.extend(out),
        }
    }
    // Unclosed \left: the compiler reports it; the delimiter is set as the
    // plain symbol it would have been without the fence.
    for (left, body, left_span) in stack {
        if let Some(c) = left {
            let delimiter = symbol_atoms(c, None);
            #[cfg(feature = "math-glyph-spans")]
            let delimiter: Vec<ml::Atom> = delimiter.into_iter().map(|d| d.with_tag(math_tag(left_span))).collect();
            atoms.extend(delimiter);
        }
        let _ = left_span;
        atoms.extend(body);
    }
    ml::MathList::new(atoms)
}

/// A matched `\left...\right` pair as math-layout's `Delimited` atom; with
/// `math-glyph-spans` each delimiter maps to its own command and the atom
/// to the whole pair.
fn fenced(left: Option<char>, right: Option<char>, body: Vec<ml::Atom>, left_span: Span, right_span: Span) -> ml::Atom {
    let atom = ml::Atom::left_right(left, right, ml::MathList::new(body));
    #[cfg(feature = "math-glyph-spans")]
    let atom = atom
        .with_tag(math_tag(left_span.merge(right_span)))
        .with_delimiter_tags(math_tag(left_span), math_tag(right_span));
    #[cfg(not(feature = "math-glyph-spans"))]
    let _ = (left_span, right_span);
    atom
}

/// math-layout provenance for a compiler span.
#[cfg(feature = "math-glyph-spans")]
fn math_tag(span: Span) -> ml::SourceTag {
    ml::SourceTag::span(ml::SourceSpan::new(span.document.0 as u32, span.start, span.end))
}

/// The paint of the innermost (shortest) range containing `start..end`.
#[cfg(feature = "math-glyph-spans")]
fn innermost_paint(ranges: &[(std::ops::Range<usize>, Paint)], start: usize, end: usize) -> Option<Paint> {
    ranges
        .iter()
        .filter(|(r, _)| r.start <= start && end <= r.end)
        .min_by_key(|(r, _)| r.end - r.start)
        .map(|(_, p)| *p)
}

/// math-layout's style for a compiler `\genfrac` style argument.
#[cfg(feature = "amsmath-inline")]
fn ml_style(s: flashtex_compiler::math::MathStyle) -> ml::Style {
    use flashtex_compiler::math::MathStyle as S;
    match s {
        S::Display => ml::Style::DISPLAY,
        S::Text => ml::Style::TEXT,
        S::Script => ml::Style::SCRIPT,
        S::ScriptScript => ml::Style::SCRIPT_SCRIPT,
    }
}

/// Lays out the kern-split runs of one formula (`runs[i]` is followed by
/// `glue[i]` ems of explicit glue, `None` for the last) and joins them
/// with kerns of the glue plus the inter-atom spacing TeX still inserts
/// across glue (glue does not reset `r_type`, §760). Rules 5/6 (Bin ->
/// Ord) run over the whole formula, so the classes at each split are the
/// ones TeX would space by.
fn layout_kerned(runs: &[ml::MathList], glue: &[Option<f64>], style: ml::Style, metrics: &dyn ml::MathFontMetrics) -> ml::Layout {
    if runs.len() == 1 && glue.first().is_none_or(|g| g.is_none()) {
        return ml::layout_with_report(&runs[0], style, metrics);
    }
    let all_atoms: Vec<ml::Atom> = runs.iter().flat_map(|l| l.atoms.iter().cloned()).collect();
    let classes = ml::layout::effective_classes(&all_atoms);
    let params = metrics.params(style.size_class());
    let (quad, mu) = (params.quad, params.mu());
    // One flat hlist, as TeX's mlist_to_hlist makes: each run's own list
    // (atoms and inter-atom glue) is spliced in at its offset rather than
    // nested as a rigid box, and the spacing across the split is real muskip
    // glue, so `MathBox::glue_totals`/`pack_to` see every glue of the formula
    // (a too-wide display is squeezed by it, §1201). Positions are unchanged.
    let mut children: Vec<ml::Child> = Vec::new();
    let (mut x, mut height, mut depth) = (0.0f64, 0.0f64, 0.0f64);
    let mut push = |children: &mut Vec<ml::Child>, x: &mut f64, b: ml::MathBox| {
        let w = b.width;
        match b.kind {
            ml::BoxKind::HBox(kids) => {
                for c in kids {
                    height = height.max(c.content.height - c.dy);
                    depth = depth.max(c.content.depth + c.dy);
                    children.push(ml::Child { dx: *x + c.dx, dy: c.dy, content: c.content });
                }
            }
            _ => {
                height = height.max(b.height);
                depth = depth.max(b.depth);
                children.push(ml::Child { dx: *x, dy: 0.0, content: b });
            }
        }
        *x += w;
    };
    let mut limitations = Vec::new();
    let mut at = 0usize;
    for (i, l) in runs.iter().enumerate() {
        let part = ml::layout_with_report(l, style, metrics);
        limitations.extend(part.limitations);
        push(&mut children, &mut x, part.root);
        at += l.atoms.len();
        if let Some(em) = glue.get(i).copied().flatten() {
            push(&mut children, &mut x, ml::MathBox::kern(em * quad));
            if let (Some(&left), Some(&right)) = (at.checked_sub(1).and_then(|j| classes.get(j)), classes.get(at)) {
                let space = ml::between(left, right, style);
                if space != ml::Space::None {
                    let flex = |amount: f64| ml::Flex::pt(amount * mu);
                    push(&mut children, &mut x, ml::MathBox::glue_flex(space.mu() * mu, space.mu(), flex(space.stretch_mu()), flex(space.shrink_mu())));
                }
            }
        }
    }
    ml::Layout {
        root: ml::MathBox {
            tag: ml::SourceTag::NONE,
            kind: ml::BoxKind::HBox(children),
            width: x,
            height,
            depth,
        },
        limitations,
    }
}

/// `FLASHTEX_INLINE_MATH_BREAKS=0` keeps every inline formula one
/// unbreakable box (the behaviour before break points existed), for the
/// tests that compare the two and for bisecting a layout difference.
fn inline_math_breaks_enabled() -> bool {
    !std::env::var_os("FLASHTEX_INLINE_MATH_BREAKS").is_some_and(|v| v == "0")
}

/// The paragraph break points of a text-style formula (tex.web §760,
/// §767: `\binoppenalty` after a Bin atom, `\relpenalty` after a Rel
/// atom, unless the atom is the formula's last noad or the next noad is a
/// Rel; glue counts as a next noad). `runs` are the kern-split lists the
/// formula was laid out from and `root` their layout: one hlist when
/// `!kerned` (`layout_with_report`), else `layout_kerned`'s hbox of run
/// hlists joined by kerns. Rules 5/6 run per run, as the layout did.
///
/// With break points, `root` is flattened to one hbox of the runs'
/// children and the kerns (all on the baseline, so the pieces re-hbox
/// without a shift) and each entry is the index of the Bin/Rel atom's box
/// among those children. Math-layout's `list` emits, per non-glue atom,
/// the inter-atom glue (when Rule 20 gives any) then the atom's box, and
/// one glue box per glue atom, so the boxes are paired with the atoms by
/// walking both.
fn inline_break_points(root: &mut ml::MathBox, runs: &[ml::MathList], kerned: bool) -> Vec<(usize, i32)> {
    use ml::AtomClass::{Bin, Rel};
    let is_glue_atom = |a: &ml::Atom| matches!(a.nucleus, ml::Nucleus::Glue { .. }) && a.superscript.is_none() && a.subscript.is_none();
    let ml::BoxKind::HBox(children) = &root.kind else { return Vec::new() };
    // The runs' children in order, and each run's range in them.
    let mut units: Vec<ml::MathBox> = Vec::new();
    let mut run_ranges: Vec<(usize, usize)> = Vec::new();
    if kerned {
        let mut ci = 0usize;
        for _ in runs {
            let start = units.len();
            let Some(ml::BoxKind::HBox(run_children)) = children.get(ci).map(|c| &c.content.kind) else { return Vec::new() };
            units.extend(run_children.iter().map(|c| c.content.clone()));
            run_ranges.push((start, units.len()));
            ci += 1;
            if let Some(kern) = children.get(ci) {
                units.push(kern.content.clone());
                ci += 1;
            }
        }
    } else {
        units.extend(children.iter().map(|c| c.content.clone()));
        run_ranges.push((0, units.len()));
    }
    let mut breaks = Vec::new();
    for (r, l) in runs.iter().enumerate() {
        let classes = ml::layout::effective_classes(&l.atoms);
        let (start, end) = run_ranges[r];
        let mut ci = start;
        for (i, (atom, class)) in l.atoms.iter().zip(&classes).enumerate() {
            if is_glue_atom(atom) {
                ci += 1;
                continue;
            }
            if matches!(units.get(ci).map(|u| &u.kind), Some(ml::BoxKind::Glue { .. })) {
                ci += 1;
            }
            let at = ci;
            ci += 1;
            let next = l.atoms.get(i + 1);
            let has_next = next.is_some() || r + 1 < runs.len();
            let next_rel = next.is_some_and(|n| n.class == Rel && !is_glue_atom(n));
            let penalty = match class {
                Bin if has_next && !next_rel => Some(700),
                Rel if has_next && !next_rel => Some(500),
                _ => None,
            };
            // `\medmuskip`/`\thickmuskip` after this atom stretch and shrink
            // with the line (`4mu plus 2mu minus 4mu`, `5mu plus 5mu`): the
            // formula is cut there too, with no break allowed unless the
            // atom carries a penalty.
            let stretchy_glue_next = matches!(units.get(ci).map(|u| &u.kind), Some(ml::BoxKind::Glue { mu, .. }) if *mu >= 4.0);
            if let Some(penalty) = penalty {
                breaks.push((at, penalty));
            } else if stretchy_glue_next {
                breaks.push((at, pl::INFINITE_PENALTY));
            }
        }
        // The walk must land on the run's end, else the pairing is off and
        // the formula stays one box.
        if ci != end {
            return Vec::new();
        }
    }
    if !breaks.is_empty() {
        *root = ml::MathBox::hlist(units);
    }
    breaks
}

/// One top-level part of a formula holding a grid (see
/// `Context::grid_formula`): a run for math-layout, a kern in ems, or an
/// `array`/`cases`/matrix grid with its cells already converted.
pub enum GridPiece {
    Run(ml::MathList),
    Kern(f64),
    Grid {
        /// Each cell as its kern-split runs and the glue after each.
        rows: Vec<Vec<(Vec<ml::MathList>, Vec<Option<f64>>)>>,
        columns: String,
        left: String,
        right: String,
        span: Span,
    },
}

/// Converts the kern-split segments of a formula into [`GridPiece`]s,
/// collecting every `\text` into `sink` (runs and cells alike).
fn grid_pieces(
    segments: &[(Vec<flashtex_compiler::math::MathAtom>, Option<f64>)],
    sink: &mut crate::mathtext::TextSink,
    fence: &dyn Fn(&Span) -> Option<Fence>,
    class: &dyn Fn(&flashtex_compiler::math::MathAtom) -> Option<ml::AtomClass>,
    op_limits: &dyn Fn(&Span) -> Option<ml::Limits>,
    texts: &[&str],
) -> Vec<GridPiece> {
    use flashtex_compiler::math::{MathAtom, MathList as CList, Nucleus as N};
    // As in `math_box`: which `Nucleus::Text` atoms are whole runs of math
    // characters, re-read from the control word at the span.
    let text_italic = |sp: &Span| math_text_keeps_italic(texts.get(sp.document.0).copied().unwrap_or(""), sp.start);
    let text_italic = &text_italic;
    let text_split = |sp: &Span| operator_thin_space_split(texts.get(sp.document.0).copied().unwrap_or(""), sp.start);
    let text_split = &text_split;
    let ellipsis = |sp: &Span| math_ellipsis_of(texts.get(sp.document.0).copied().unwrap_or(""), sp.start);
    let ellipsis = &ellipsis;
    let mut pieces = Vec::new();
    for (atoms, em) in segments {
        let mut run: Vec<MathAtom> = Vec::new();
        let flush = |run: &mut Vec<MathAtom>, pieces: &mut Vec<GridPiece>, sink: &mut crate::mathtext::TextSink| {
            if !run.is_empty() {
                pieces.push(GridPiece::Run(convert_math_classed(&CList { atoms: std::mem::take(run) }, sink, fence, class, op_limits, text_italic, text_split, ellipsis)));
            }
        };
        for (a, top) in atoms.iter().zip(top_level_grids(atoms, fence)) {
            match &a.nucleus {
                N::Matrix { rows, columns, left, right } if top => {
                    flush(&mut run, &mut pieces, sink);
                    // amsmath `aligned`/`alignedat`/`split`: a right-hand
                    // cell is `{}##`, so a leading relation or operator is
                    // spaced against an empty Ord. The compiler trims the
                    // column letters to the grid's width, so the environment
                    // is read at the grid's `\begin`.
                    let pairs = texts
                        .get(a.span.document.0)
                        .and_then(|t| t.get(a.span.start..))
                        .is_some_and(|r| ["\\begin{aligned}", "\\begin{alignedat}", "\\begin{split}"].iter().any(|p| r.starts_with(p)));
                    let cell_runs = |ci: usize, cell: &CList, sink: &mut crate::mathtext::TextSink| {
                        let mut prefixed;
                        let cell = match cell.atoms.first() {
                            Some(first) if pairs && ci % 2 == 1 => {
                                let mut empty = first.clone();
                                empty.nucleus = N::Symbol(String::new());
                                empty.superscript = None;
                                empty.subscript = None;
                                prefixed = cell.clone();
                                prefixed.atoms.insert(0, empty);
                                &prefixed
                            }
                            _ => cell,
                        };
                        let parts = split_at_spaces(cell, fence, sink.font_em_ratio());
                        let runs = parts.iter().map(|(atoms, _)| convert_math_classed(&CList { atoms: atoms.clone() }, sink, fence, class, op_limits, text_italic, text_split, ellipsis)).collect();
                        let glue = parts.iter().map(|(_, em)| *em).collect();
                        (runs, glue)
                    };
                    pieces.push(GridPiece::Grid {
                        rows: rows.iter().map(|row| row.iter().enumerate().map(|(ci, cell)| cell_runs(ci, cell, sink)).collect()).collect(),
                        columns: columns.clone(),
                        left: left.clone(),
                        right: right.clone(),
                        span: a.span,
                    });
                }
                _ => run.push(a.clone()),
            }
        }
        flush(&mut run, &mut pieces, sink);
        if let Some(em) = em {
            pieces.push(GridPiece::Kern(*em));
        }
    }
    pieces
}

/// Which of `atoms` are grids at the formula's top level, laid out by
/// `grid_formula`: without scripts and outside every `\left...\right` pair
/// (the compiler lists `\left`/`\right` as sibling atoms; a grid between
/// them belongs to that Inner atom's body and is set as a nested box).
fn top_level_grids(atoms: &[flashtex_compiler::math::MathAtom], fence: &dyn Fn(&Span) -> Option<Fence>) -> Vec<bool> {
    use flashtex_compiler::math::{DelimiterRole, Nucleus as N};
    let mut depth = 0usize;
    atoms
        .iter()
        .map(|a| match &a.nucleus {
            N::Symbol(sym) if sym.chars().count() <= 1 => {
                match fence(&a.span) {
                    Some(Fence::Left) => depth += 1,
                    Some(Fence::Right) => depth = depth.saturating_sub(1),
                    None => {}
                }
                false
            }
            N::SizedDelimiter { role, .. } => {
                match role {
                    DelimiterRole::Left => depth += 1,
                    DelimiterRole::Right => depth = depth.saturating_sub(1),
                    _ => {}
                }
                false
            }
            N::Matrix { .. } => depth == 0 && a.superscript.is_none() && a.subscript.is_none(),
            _ => false,
        })
        .collect()
}

/// Splits `list` at its top-level `Space` atoms (outside `\left...\right`
/// pairs): each entry is a run of atoms and the glue after it in ems
/// (`None` for the last run). Consecutive spaces sum; a formula without
/// top-level glue is one run.
///
/// Glue in text-font ems (`\quad`, compiler `font_em`) is converted to math
/// symbol font quads with `font_em_ratio` (text quad / family-2 quad).
fn split_at_spaces(list: &flashtex_compiler::math::MathList, fence: &dyn Fn(&Span) -> Option<Fence>, font_em_ratio: f64) -> Vec<(Vec<flashtex_compiler::math::MathAtom>, Option<f64>)> {
    use flashtex_compiler::math::Nucleus as N;
    let mut out: Vec<(Vec<flashtex_compiler::math::MathAtom>, Option<f64>)> = Vec::new();
    let mut current = Vec::new();
    let mut depth = 0usize;
    for a in &list.atoms {
        match &a.nucleus {
            N::Space { em, .. } if depth == 0 && a.superscript.is_none() && a.subscript.is_none() => {
                let em = &space_em(a, *em, font_em_ratio);
                if current.is_empty() {
                    if let Some((_, Some(prev))) = out.last_mut() {
                        *prev += em;
                        continue;
                    }
                }
                out.push((std::mem::take(&mut current), Some(*em)));
            }
            N::Symbol(sym) if sym.chars().count() <= 1 => {
                match fence(&a.span) {
                    Some(Fence::Left) => depth += 1,
                    Some(Fence::Right) => depth = depth.saturating_sub(1),
                    None => {}
                }
                current.push(a.clone());
            }
            N::SizedDelimiter { role, .. } => {
                match role {
                    flashtex_compiler::math::DelimiterRole::Left => depth += 1,
                    flashtex_compiler::math::DelimiterRole::Right => depth = depth.saturating_sub(1),
                    _ => {}
                }
                current.push(a.clone());
            }
            _ => current.push(a.clone()),
        }
    }
    // A trailing space keeps its kern: TeX includes it in the formula's
    // box (an empty run follows it).
    out.push((current, None));
    out
}

/// Every `array`/`cases`/matrix grid in `list` and its sub-formulas as
/// `(rows, columns)`; see the `Matrix` arm of [`convert_math_fenced`].
fn math_grids(list: &flashtex_compiler::math::MathList, out: &mut Vec<(usize, usize)>) {
    use flashtex_compiler::math::Nucleus as N;
    for a in &list.atoms {
        match &a.nucleus {
            N::Matrix { rows, .. } => {
                out.push((rows.len(), rows.iter().map(Vec::len).max().unwrap_or(0)));
                for cell in rows.iter().flatten() {
                    math_grids(cell, out);
                }
            }
            N::Fraction { numerator, denominator } => {
                math_grids(numerator, out);
                math_grids(denominator, out);
            }
            N::Radical(r) | N::Framed { body: r, .. } | N::Accent { body: r, .. } | N::Group(r) => math_grids(r, out),
            N::Rule(_) => {}
            N::Stacked { base, over, under } => {
                math_grids(base, out);
                for part in [over, under].into_iter().flatten() {
                    math_grids(part, out);
                }
            }
            N::Symbol(_) | N::Text(_) | N::Space { .. } | N::Bold(_) | N::SizedDelimiter { .. } => {}
            #[cfg(feature = "amsmath-inline")]
            N::GenFraction { numerator, denominator, .. } => {
                math_grids(numerator, out);
                math_grids(denominator, out);
            }
            #[cfg(feature = "amsmath-inline")]
            N::Phantom { body: r, .. } | N::Operator { body: r, .. } => math_grids(r, out),
            #[cfg(feature = "amsmath-inline")]
            N::SubArray { rows, .. } => rows.iter().for_each(|r| math_grids(r, out)),
            #[cfg(feature = "amsmath-inline")]
            N::ExtArrow { above, below, .. } => {
                math_grids(above, out);
                math_grids(below, out);
            }
        }
        if let Some(s) = &a.superscript {
            math_grids(s, out);
        }
        if let Some(s) = &a.subscript {
            math_grids(s, out);
        }
    }
}

/// `em` of a compiler `Space` atom in family-2 quads: text-font ems
/// (`\quad`) are scaled by `ratio`.
fn space_em(a: &flashtex_compiler::math::MathAtom, em: f64, ratio: f64) -> f64 {
    #[cfg(feature = "amsmath-inline")]
    if matches!(a.nucleus, flashtex_compiler::math::Nucleus::Space { font_em: true, .. }) {
        return em * ratio;
    }
    let _ = (a, ratio);
    em
}

/// Total explicit math glue (`\quad`/`\qquad`, in ems) in `list` and its
/// sub-formulas; see the `Space` arm of [`convert_math_fenced`].
fn math_glue_em(list: &flashtex_compiler::math::MathList) -> f64 {
    use flashtex_compiler::math::Nucleus as N;
    list.atoms
        .iter()
        .map(|a| {
            let own = match &a.nucleus {
                N::Space { em, .. } => *em,
                N::Fraction { numerator, denominator } => math_glue_em(numerator) + math_glue_em(denominator),
                N::Radical(r) | N::Framed { body: r, .. } | N::Accent { body: r, .. } | N::Group(r) => math_glue_em(r),
                N::Stacked { base, over, under } => {
                    math_glue_em(base) + [over, under].into_iter().flatten().map(math_glue_em).sum::<f64>()
                }
                N::Matrix { rows, .. } => rows.iter().flatten().map(math_glue_em).sum(),
                N::Symbol(_) | N::Text(_) | N::Bold(_) | N::SizedDelimiter { .. } | N::Rule(_) => 0.0,
                #[cfg(feature = "amsmath-inline")]
                N::GenFraction { numerator, denominator, .. } => math_glue_em(numerator) + math_glue_em(denominator),
                #[cfg(feature = "amsmath-inline")]
                N::Phantom { body: r, .. } | N::Operator { body: r, .. } => math_glue_em(r),
                #[cfg(feature = "amsmath-inline")]
                N::SubArray { rows, .. } => rows.iter().map(math_glue_em).sum(),
                #[cfg(feature = "amsmath-inline")]
                N::ExtArrow { above, below, .. } => math_glue_em(above) + math_glue_em(below),
            };
            own + a.superscript.as_ref().map_or(0.0, math_glue_em) + a.subscript.as_ref().map_or(0.0, math_glue_em)
        })
        .sum()
}

/// The unicode-math combining mark for a compiler accent command, which is
/// what Latin Modern Math's `MATH` table carries accent attachment for.
fn accent_char(a: flashtex_compiler::math::Accent) -> char {
    use flashtex_compiler::math::Accent as A;
    // The character math-layout's `cm` table slots each `\mathaccent` at
    // (`fontmath.ltx` 410-421): the `operators` (roman) spacing accents
    // "5E `\hat`, "7E `\tilde`, "16 `\bar`, "5F `\dot`, "7F `\ddot`, "14
    // `\check`, "15 `\breve`, "13 `\acute`, "12 `\grave`; `\vec` letters
    // "7E; `\widehat`/`\widetilde` the `largesymbols` chains from "62/"65,
    // which math-layout keys by the combining marks.
    match a {
        A::Hat => '\u{02C6}',
        A::WideHat => '\u{0302}',
        A::Bar => '\u{00AF}',
        A::Vec => '\u{20D7}',
        A::Tilde => '\u{02DC}',
        A::WideTilde => '\u{0303}',
        A::Dot => '\u{02D9}',
        A::Ddot => '\u{00A8}',
        A::Check => '\u{02C7}',
        A::Breve => '\u{02D8}',
        A::Acute => '\u{00B4}',
        A::Grave => '`',
    }
}

/// amsmath's `\ex@` at a font size (`amsgen.sty` 104-125, `\compute@ex@`):
/// 1pt at 10pt, growing by 3% compounding per 0.5pt of size over 10pt (and
/// shrinking below), 1.5pt past 20pt. Used for `\underarrow@`'s
/// `\kern1.3\ex@`.
fn ams_ex(size_pt: f64) -> f64 {
    if size_pt <= 0.0 {
        return 1.0;
    }
    if -size_pt < -20.0 {
        return 1.5;
    }
    // In scaled points, as TeX computes it.
    let mut d: i64 = ((10.0 - size_pt) * 2.0 * 65536.0).round() as i64;
    let negative = d > 0;
    d = d.abs() - 1000;
    let mut vfuzz: i64 = 65536;
    while d > 0 {
        // `\vfuzz=.97\vfuzz`: `.97` scans as 63570/65536.
        vfuzz = vfuzz * 63570 / 65536;
        d -= 65536;
    }
    let delta = 65536 - vfuzz;
    let ex = if negative { 65536 - delta } else { 65536 + delta };
    ex as f64 / 65536.0
}

/// Constructs in `list` and its sub-formulas the pipeline sets only
/// approximately, as `math_limitation` messages (one entry per occurrence;
/// `math_box` deduplicates by message): `\mathbf` in the roman face.
fn math_approximations(list: &flashtex_compiler::math::MathList, out: &mut Vec<String>) {
    use flashtex_compiler::math::Nucleus as N;
    for a in &list.atoms {
        match &a.nucleus {
            // `\mathbf` is now set through the text sink's bold alphabet role
            // (or a Unicode bold math alphanumeric for one letter/digit), not
            // the regular roman face, so it is no longer a math_limitation.
            N::Bold(_) => {}
            N::Rule(_) => out.push("math-mode \\rule set as horizontal space of its width: math-layout has no rule atom, nothing painted".to_string()),
            N::Framed { body, .. } => math_approximations(body, out),
            N::Fraction { numerator, denominator } => {
                math_approximations(numerator, out);
                math_approximations(denominator, out);
            }
            N::Radical(r) | N::Accent { body: r, .. } | N::Group(r) => math_approximations(r, out),
            N::Stacked { base, over, under } => {
                math_approximations(base, out);
                for part in [over, under].into_iter().flatten() {
                    math_approximations(part, out);
                }
            }
            N::Matrix { rows, .. } => {
                for cell in rows.iter().flatten() {
                    math_approximations(cell, out);
                }
            }
            // `\big`..`\Bigg` (scale > 1 with a fixed role): the glyph is set
            // at text size. `\left`/`\right` arrive at scale 1 and are
            // stretched by math-layout's own Rule 19, so they are exact.
            N::SizedDelimiter { glyph, scale, role } => {
                use flashtex_compiler::math::DelimiterRole;
                if !matches!(role, DelimiterRole::Left | DelimiterRole::Right) && *scale != 1.0 && cfg!(not(feature = "amsmath-inline")) {
                    out.push(format!("\\big-family delimiter {glyph:?} (scale {scale}) set at text size: math-layout has no fixed-step delimiter atom"));
                }
            }
            N::Symbol(_) | N::Text(_) | N::Space { .. } => {}
            #[cfg(feature = "amsmath-inline")]
            N::GenFraction { numerator, denominator, .. } => {
                math_approximations(numerator, out);
                math_approximations(denominator, out);
            }
            #[cfg(feature = "amsmath-inline")]
            N::Phantom { body: r, .. } | N::Operator { body: r, .. } => math_approximations(r, out),
            #[cfg(feature = "amsmath-inline")]
            N::SubArray { rows, .. } => rows.iter().for_each(|r| math_approximations(r, out)),
            #[cfg(feature = "amsmath-inline")]
            N::ExtArrow { above, below, .. } => {
                math_approximations(above, out);
                math_approximations(below, out);
            }
        }
        for part in [&a.superscript, &a.subscript].into_iter().flatten() {
            math_approximations(part, out);
        }
    }
}

/// The math-layout atoms for one compiler symbol character: plain.tex's
/// default classification, with the compiler's spellings that TeX sets as
/// composites expanded (`fontmath.ltx`: `\neq` is `\not=`, `\notin` is
/// `\not\in`, the zero-width relation slash before the relation).
///
/// `width_em` is the originating `MathAtom.width_em` (compiler pin
/// `c583d6d4`: `pub` now, `Some(0.777781)` for `\varnothing`, `None`
/// otherwise, including for plain `\emptyset`'s identical U+2205); read here
/// instead of re-scanning the source at the atom's span for the control
/// word, like [`class_override_of`].
fn symbol_atoms(c: char, width_em: Option<f64>) -> Vec<ml::Atom> {
    match c {
        // The compiler spells \cdot as U+00B7; the Bin class and cmsy slot
        // are those of U+22C5.
        '\u{00B7}' => vec![ml::Atom::symbol('\u{22C5}')],
        '\u{2260}' => vec![ml::Atom::rel(crate::mathtex::NOT_SLASH), ml::Atom::symbol('=')],
        '\u{2209}' => vec![ml::Atom::rel(crate::mathtex::NOT_SLASH), ml::Atom::symbol('\u{2208}')],
        // `\varnothing`: same U+2205 as `\emptyset`, but forced to msbm10's
        // width. A sentinel keeps the two apart for the metrics providers
        // (`TexMathMetrics`/`MathFonts`), which paint both from cmsy10's
        // `\emptyset` slot but only force this one's advance and outline.
        '\u{2205}' if width_em.is_some() => vec![ml::Atom::symbol(crate::mathfont::VARNOTHING_SENTINEL)],
        _ => match long_arrow_pieces(c) {
            Some((left, right)) => vec![long_arrow(left, right)],
            None => vec![ml::Atom::symbol(c)],
        },
    }
}

/// The two relations a LaTeX long arrow joins (`latex.ltx`:
/// `\longrightarrow` = `\relbar\joinrel\rightarrow`, `\Longrightarrow` =
/// `\Relbar\joinrel\Rightarrow`, ...). `\relbar` is cmsy's minus and
/// `\Relbar` cmr's `=`; the arrows are cmsy "20/"21/"24/"28/"29/"2C.
fn long_arrow_pieces(c: char) -> Option<(char, char)> {
    Some(match c {
        '\u{27F5}' => ('\u{2190}', '\u{2212}'), // \longleftarrow
        '\u{27F6}' => ('\u{2212}', '\u{2192}'), // \longrightarrow
        '\u{27F7}' => ('\u{2190}', '\u{2192}'), // \longleftrightarrow
        '\u{27F8}' => ('\u{21D0}', '='),        // \Longleftarrow
        '\u{27F9}' => ('=', '\u{21D2}'),        // \Longrightarrow
        '\u{27FA}' => ('\u{21D0}', '\u{21D2}'), // \Longleftrightarrow
        _ => return None,
    })
}

/// A long arrow as TeX builds it: the two relations with `\joinrel`
/// (`\mathrel{\mkern-3mu}`) between them. Adjacent relations get no
/// inter-atom space and no break between them, so the three are one
/// relation whose nucleus is `left`, a -3mu kern and `right`. Latin Modern
/// Math's single U+27F9 glyph is 1.457em wide where pdfTeX's `=`+`⇒` join
/// is 0.777781 + 1.000003 - 3/18 = 1.611em, which moved every glyph after
/// `\Longrightarrow` in a centred display by half the 1.69bp difference at
/// 11pt (HW1 Problem 4(b)).
///
/// Not modelled: `\relbar` is `\smash`ed (amsmath `\mathsm@sh`), so pdfTeX's
/// `\longrightarrow` box is only as tall as the arrow; here the minus keeps
/// its 0.583em height and 0.083em depth.
fn long_arrow(left: char, right: char) -> ml::Atom {
    let piece = |ch| ml::Atom::new(ml::AtomClass::Ord, ml::Nucleus::Symbol(ch));
    ml::Atom::new(ml::AtomClass::Rel, ml::Nucleus::List(ml::MathList::new(vec![piece(left), ml::Atom::glue(-3.0, 0.0), piece(right)])))
}

/// Adds `\addvspace` glue (a list environment's `\topsep`) to the block's
/// before-skip; `None` is a no-op.
/// The page builder's parameters for the stylesheet's text area.
/// A longtable on a page-building path that does not carry its region yet:
/// the table is still set, but `\LT@head`/`\LT@foot` are not repeated and
/// `\pagegoal` is not reduced, so a break inside it loses them.
fn longtable_limitation(ctx: &mut Context, longtables: &[(usize, pagebuild::Region)], blocks: &[BuiltBlock], what: &str) {
    for (bi, region) in longtables {
        if region.head.is_none() && region.foot.is_none() {
            continue;
        }
        let span = blocks.get(*bi).and_then(|b| b.recs.iter().flatten().next().copied()).and_then(|r| match &ctx.recs[r] {
            BoxRec::Table(t) => Some(t.span),
            _ => None,
        });
        let src = span.map(|sp| vec![ctx.source(sp)]).unwrap_or_default();
        ctx.diagnostics.push(Diagnostic::warning(
            "table_limitation",
            format!("a longtable {what} does not repeat \\endhead/\\endfoot across a page break yet; the rows are set without them"),
            src,
        ));
    }
}

/// The incremental cache key of one block: `None` whenever the block
/// cannot be keyed on its own bytes (no cache, a footnote's per-build record
/// indices, or no source origin at all).
fn block_key(cache: Option<&RenderCache>, style_fp: u64, tag: u8, items: &[AItem], flags: &[u64]) -> (Option<u64>, Option<(DocumentId, usize)>) {
    use std::hash::{Hash, Hasher};
    if cache.is_none() {
        return (None, None);
    }
    // A footnote's record indices and note table are per build.
    if items.iter().any(|i| matches!(i, AItem::Footnote { .. })) {
        return (None, None);
    }
    let Some((document, base)) = incremental::block_origin(items) else {
        return (None, None);
    };
    let mut h = std::collections::hash_map::DefaultHasher::new();
    tag.hash(&mut h);
    style_fp.hash(&mut h);
    document.0.hash(&mut h);
    flags.hash(&mut h);
    incremental::hash_items(items, base, &mut h);
    (Some(h.finish()), Some((document, base)))
}

fn page_params(s: &Stylesheet) -> pagebuild::PageParams {
    pagebuild::PageParams {
        vsize: s.text_height_pt,
        topskip: s.topskip_pt,
        maxdepth: s.maxdepth_pt,
        baselineskip: s.baselineskip_pt,
        lineskip: s.lineskip_pt,
        lineskiplimit: s.lineskiplimit_pt,
        flushbottom: !s.raggedbottom,
    }
}

/// A vertical block of `lines` with no penalties, skips or `\parskip`.
fn plain_vblock(lines: Vec<(f64, f64)>) -> VBlock {
    VBlock {
        lines,
        penalty_before: None,
        space_before: None,
        parskip: None,
        interline_penalty: 0,
        club_penalty: 0,
        widow_penalty: 0,
        penalty_after: None,
        space_after: None,
        no_interline_first: false,
        no_interline_after: false,
        baselineskip: None,
        vskip_after: Vec::new(),
        broken_penalty: Vec::new(),
        vadjust_penalty: Vec::new(),
        fil_break: false,
        pre_space_after: None,
        lineskip: None,
        contributed: None,
        line_penalty: Vec::new(),
        depth_after: pagebuild::DepthAfter::default(),
    }
}

/// `\null` (an empty `\hbox`) as a block with `vertical`'s skips.
fn empty_block(vertical: VBlock) -> BuiltBlock {
    positioned_block(Vec::new(), 0.0, 0.0, 0.0, vertical)
}

/// One line of already positioned runs (`(run, record, x)`, `x` from the
/// start of the text area) as a block.
fn positioned_block(runs: Vec<(pl::GlyphRun, usize, f64)>, height: f64, depth: f64, width: f64, vertical: VBlock) -> BuiltBlock {
    let mut items = Vec::with_capacity(runs.len());
    let mut recs = Vec::with_capacity(runs.len());
    let mut placed = Vec::with_capacity(runs.len());
    for (run, rec, x) in runs {
        placed.push(position_run(&run, x, 0.0));
        items.push(pl::Item::Box(run));
        recs.push(Some(rec));
    }
    let n = items.len();
    BuiltBlock {
        block: pl::ParagraphBlock::body(pl::Lines {
            lines: vec![pl::Line {
                index: 0,
                runs: placed,
                baseline_y: height,
                height,
                depth,
                natural_width: width,
                set_width: width,
                ratio: 0.0,
                badness: 0.0,
                items: 0..n,
                hyphenated: false,
            }],
            breaks: Vec::new(),
            stats: one_line_stats(),
            diagnostics: Vec::new(),
            height: height + depth,
        }),
        items,
        recs,
        vertical,
        labels: Vec::new(),
        cache_key: None,
    }
}

/// Which form of `\maketitle` [`Context::title_blocks`] sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleForm {
    /// `\@maketitle` in the page flow (one column).
    Flow,
    /// `\twocolumn[\@maketitle]`: an internal `\vbox` at the page top.
    Float,
    /// The `titlepage` form: a page of its own.
    Page,
}

/// `\tabcolsep` (article.cls line 444, report.cls/book.cls the same).
const TABCOLSEP_PT: f64 = 6.0;

/// `\dbltextfloatsep` (latex.ltx, and `size10/11/12.clo` all keep 20pt): the
/// gap `\@combinedblfloats` leaves between the `\@dbltoplist` material and
/// the two-column box, and the amount `\@topnewpage`'s `\vskip
/// -\dbltextfloatsep` takes back off its own box.
const DBLTEXTFLOATSEP_PT: f64 = 20.0;

/// Reports a `\twocolumn[<material>]` case `\@topnewpage` handles and this
/// page builder does not, against the title block's first source span.
fn top_title_warning(ctx: &mut Context, blocks: &[BuiltBlock], first: usize, message: &str) {
    let span = blocks.get(first).and_then(|b| b.recs.iter().flatten().next().copied()).and_then(|r| match &ctx.recs[r] {
        BoxRec::Text { clusters, .. } => clusters.first().map(|c| c.span),
        _ => None,
    });
    let src = span.map(|sp| vec![ctx.source(sp)]).unwrap_or_default();
    ctx.diagnostics.push(Diagnostic::warning("unsupported_block", message.to_string(), src));
}

fn add_skip_before(v: &mut pagebuild::VBlock, skip: Option<(f64, f64, f64)>) {
    let Some((n, s, k)) = skip else { return };
    v.space_before = Some(match v.space_before {
        Some((n0, s0, k0)) => (n0 + n, s0 + s, k0 + k),
        None => (n, s, k),
    });
}

/// Adds `pt` points of `\vspace` glue (compiler `Block::VSpace`) to the
/// block's before-skip. Zero is a no-op so cached blocks stay identical.
fn add_vspace(v: &mut pagebuild::VBlock, pt: f64) {
    add_skip(v, pt, (0.0, 0.0));
}

/// [`add_vspace`] with the skip's stretch and shrink (a list's `\topsep` /
/// `\itemsep` / `\parsep` glue, which is not rigid).
fn add_skip(v: &mut pagebuild::VBlock, pt: f64, flex: (f64, f64)) {
    if pt == 0.0 && flex == (0.0, 0.0) {
        return;
    }
    v.space_before = Some(match v.space_before {
        Some((n, s, k)) => (n + pt, s + flex.0, k + flex.1),
        None => (pt, flex.0, flex.1),
    });
}

/// Lays out every block of `doc` onto pages. With `cache`, blocks whose
/// items, flags and style match an earlier build are reused (see
/// `incremental`); the result is identical either way.
pub fn build(ctx: &mut Context, doc: &Doc, cache: Option<&RenderCache>) -> Laid {
    build_with_floats(ctx, doc, cache, &[])
}

/// [`build`] with `figure`/`table` floats placed by LaTeX's algorithm
/// ([`floatpage`]); without floats the page builder is unchanged.
pub fn build_with_floats(ctx: &mut Context, doc: &Doc, cache: Option<&RenderCache>, floats: &[floatpage::FloatSpec]) -> Laid {
    if let Some(outer) = multicol::outer_doc(ctx, doc, floats) {
        return build_with_floats(ctx, &outer, cache, floats);
    }
    let mut blocks: Vec<BuiltBlock> = Vec::new();
    let style: &Stylesheet = ctx.style;
    let geo = style.class_geometry.as_deref();
    // `(index of the block that follows, event, source)`: page-style and
    // mark commands; a heading's mark sits on the heading's own block.
    let mut events: Vec<(usize, adapter::ChromeEvent, Span)> = Vec::new();
    // First block of every `\chapter` (its `\cleardoublepage`).
    // Each is paired with the number of events issued before that
    // `\cleardoublepage` (book matter commands issue their own).
    let mut chapter_starts: Vec<(usize, usize)> = Vec::new();
    // Blocks after a class command's `\clearpage` (book matter commands).
    let mut clears: Vec<usize> = Vec::new();
    let n_columns = geo.map_or(1, |g| g.frame.columns.len().max(1));
    // `\twocolumn[\@maketitle]` (`\@topnewpage`): its first block, its lines
    // placed in the box, and the box height plus `\dbltextfloatsep` that
    // both columns of the first page lose.
    let mut top_title: Option<(usize, Vec<pagebuild::Placed>, f64)> = None;
    // `\sectionmark`/`\chaptermark` as defined by the last `\ps@headings` or
    // `\ps@myheadings` (`\ps@plain`/`\ps@empty` leave them alone).
    let mut mark_rules: Vec<flashtex_class_geometry::MarkRule> = geo.map(|g| g.mark_rules.clone()).unwrap_or_default();
    let mut after_heading = false;
    // Whether the open paragraph-shape environment began in vertical mode
    // (`\@topsepadd` keeps `\partopsep` for the closing skip too).
    let mut env_vmode = false;
    let quad = ctx.text_params(TextStyle::default(), ctx.style.body_size_pt).quad;
    let style_fp = if cache.is_some() { incremental::style_fingerprint(ctx.style) } else { 0 };
    let key_for = |tag: u8, items: &[AItem], flags: &[u64]| block_key(cache, style_fp, tag, items, flags);
    // Two-column documents: blocks that start a page (`\clearpage`,
    // `\chapter`, `\part`) rather than a column, and blocks set across the
    // text width (`\onecolumn` material), by built-block index.
    let mut page_start_blocks: Vec<usize> = Vec::new();
    let mut wide_blocks: Vec<usize> = Vec::new();
    // longtable page-breaking regions, by built-block index.
    let mut longtables: Vec<(usize, pagebuild::Region)> = Vec::new();
    for (doc_index, block) in doc.blocks.iter().enumerate() {
        if doc.page_starts.binary_search(&doc_index).is_ok() {
            page_start_blocks.push(blocks.len());
        }
        // A body `\cleardoublepage`: the odd-page test `open_right` applies
        // to a chapter's.
        if doc.double_page_starts.binary_search(&doc_index).is_ok() {
            chapter_starts.push((blocks.len(), events.len()));
        }
        match block {
            Block::Heading {
                level,
                items,
                eject_before,
                vspace_before,
                number,
                title,
                span,
            } => {
                let (key, origin) = key_for(b'H', items, &[u64::from(*level)]);
                if let Some(mut b) = ctx.cached(cache, key, origin, |c| c.heading_block(*level, items)) {
                    if *eject_before {
                        b.vertical.penalty_before = Some(pagebuild::EJECT_PENALTY);
                    }
                    // `\@startsection`: `\addvspace{<before>}` — right after
                    // another heading (`\@nobreak`) no skip at all; otherwise
                    // only the excess over the skip the previous block left
                    // (`\lastskip`: a display's `\belowdisplayskip`, an
                    // environment's closing `\topsep`), that skip removed.
                    // `\addpenalty\@secpenalty` belongs to the same branch:
                    // under `\@nobreak` there is no breakpoint between the
                    // two heads at all.
                    if after_heading {
                        b.vertical.space_before = None;
                        if !*eject_before {
                            b.vertical.penalty_before = None;
                        }
                    } else if let (Some(before), Some(prev)) = (b.vertical.space_before, blocks.last_mut()) {
                        if let Some(last) = prev.vertical.space_after {
                            if last.0 < before.0 {
                                prev.vertical.space_after = None;
                            } else {
                                b.vertical.space_before = None;
                            }
                        }
                    }
                    add_vspace(&mut b.vertical, *vspace_before);
                    // `\@sect` issues `\sectionmark` after the heading box
                    // (starred headings issue none).
                    let command = match level {
                        1 => "section",
                        2 => "subsection",
                        _ => "subsubsection",
                    };
                    if let Some(rule) = mark_rules.iter().find(|r| r.command == command).filter(|_| !number.is_empty()) {
                        events.push((blocks.len(), mark_event(rule, Some(number), title, doc.secnumdepth), *span));
                    }
                    blocks.push(b);
                    after_heading = true;
                }
            }
            Block::TocEntry(entry) => {
                if let Some(mut b) = ctx.toc_entry_block(entry) {
                    // `\addpenalty` does nothing under `\@nobreak` (right
                    // after the list's heading).
                    if after_heading {
                        b.vertical.penalty_before = None;
                    }
                    // report/book `\@chapter`'s `\addvspace{10\p@}` ahead of
                    // the entry's own `\vskip`: `\vskip-\lastskip \vskip
                    // 10pt` unless the previous skip is already as large.
                    if entry.addvspace_pt > 0.0 {
                        let prev_after = blocks.last().and_then(|p| p.vertical.space_after).map_or(0.0, |k| k.0);
                        if prev_after < entry.addvspace_pt {
                            if let Some(prev) = blocks.last_mut() {
                                prev.vertical.space_after = None;
                            }
                            let own = b.vertical.space_before.unwrap_or((0.0, 0.0, 0.0));
                            b.vertical.space_before = Some((own.0 + entry.addvspace_pt, own.1, own.2));
                        }
                    }
                    // `\addvspace`: only the excess over the previous skip.
                    if entry.style.addvspace {
                        if let (Some(before), Some(prev)) = (b.vertical.space_before, blocks.last_mut()) {
                            if let Some(last) = prev.vertical.space_after {
                                if last.0 < before.0 {
                                    prev.vertical.space_after = None;
                                } else {
                                    b.vertical.space_before = None;
                                }
                            }
                        }
                    }
                    if entry.wide {
                        wide_blocks.push(blocks.len());
                    }
                    blocks.push(b);
                    // report/book `\l@part`: `\global\@nobreaktrue`.
                    after_heading = entry.style.nobreak_after;
                }
            }
            Block::Part { number, items, span, eject_before, clear_before } => {
                let Some(g) = geo else { continue };
                if *clear_before {
                    page_start_blocks.push(blocks.len());
                }
                let spec = &g.part;
                let first = blocks.len();
                // `\markboth{}{}` (and report/book `\thispagestyle{plain}`).
                let empty_marks = adapter::ChromeEvent::MarkBoth(String::new(), String::new());
                if spec.own_page {
                    events.push((first, adapter::ChromeEvent::ThisPageStyle(flashtex_class_geometry::PageStyle::Plain), *span));
                    events.push((first, empty_marks, *span));
                    // report/book `\part`: `\if@twocolumn \onecolumn`.
                    let width = if n_columns > 1 { crate::style::frame_pt(g.frame.text_width) } else { ctx.style.text_width_pt };
                    let built = ctx.part_page_blocks(number.as_deref(), items, *span, spec, g.options.size, width);
                    if spec.page_break == flashtex_class_geometry::PageBreak::ClearDoublePage {
                        chapter_starts.push((first, events.len()));
                    }
                    blocks.extend(built);
                    if spec.blank_page_after {
                        // `\@endpart`: `\null \thispagestyle{empty} \newpage`.
                        let mut blank = plain_vblock(vec![(0.0, 0.0)]);
                        blank.penalty_after = Some(pagebuild::EJECT_PENALTY);
                        events.push((blocks.len(), adapter::ChromeEvent::ThisPageStyle(flashtex_class_geometry::PageStyle::Empty), *span));
                        blocks.push(empty_block(blank));
                    }
                    page_start_blocks.push(first);
                    if n_columns > 1 {
                        wide_blocks.extend(first..blocks.len());
                    }
                    after_heading = false;
                } else {
                    let mut built = ctx.part_flow_blocks(number.as_deref(), items, *span, spec, g.options.size);
                    if let (true, Some(b)) = (*eject_before, built.first_mut()) {
                        b.vertical.penalty_before = Some(pagebuild::EJECT_PENALTY);
                    }
                    // `\addvspace{4ex}`: only the excess over the previous skip.
                    if let (Some(b), Some(prev)) = (built.first_mut(), blocks.last_mut()) {
                        if let (Some(before), Some(last)) = (b.vertical.space_before, prev.vertical.space_after) {
                            if last.0 < before.0 {
                                prev.vertical.space_after = None;
                            } else {
                                b.vertical.space_before = None;
                            }
                        }
                    }
                    events.push((first, empty_marks, *span));
                    blocks.extend(built);
                    // `\@afterheading`.
                    after_heading = true;
                }
            }
            Block::Chapter { number, appendix, items, title, span, mark } => {
                page_start_blocks.push(blocks.len());
                let Some((g, spec)) = geo.and_then(|g| g.chapter.as_ref().map(|c| (g, c))) else { continue };
                // `\chapter`: `\clearpage`, `\thispagestyle{plain}`, then
                // `\@chapter`'s `\chaptermark` before `\@makechapterhead`.
                events.push((blocks.len(), adapter::ChromeEvent::ThisPageStyle(spec.page_style), *span));
                if let (true, Some(rule)) = (*mark, mark_rules.iter().find(|r| r.command == "chapter")) {
                    events.push((blocks.len(), mark_event(rule, number.as_deref(), title, doc.secnumdepth), *span));
                }
                // `\appendix` makes `\@chapapp` `\appendixname`.
                let appendix_spec;
                let spec = if *appendix {
                    appendix_spec = flashtex_class_geometry::ChapterSpec { prefix: "Appendix", ..spec.clone() };
                    &appendix_spec
                } else {
                    spec
                };
                let built = ctx.chapter_blocks(number.as_deref(), items, *span, spec, g.options.size);
                if spec.page_break == flashtex_class_geometry::PageBreak::ClearDoublePage {
                    chapter_starts.push((blocks.len(), events.len()));
                }
                blocks.extend(built);
                after_heading = true;
            }
            // `\@starttoc`'s `\@nobreakfalse`: a heading next takes its
            // `\addvspace` again (only the excess over the list heading's
            // after-skip), and a paragraph next its normal `\clubpenalty`.
            Block::NoBreakFalse { .. } => after_heading = false,
            Block::ClearPage { double, .. } => {
                clears.push(blocks.len());
                if *double {
                    // Tested before the `\pagenumbering` that follows it.
                    chapter_starts.push((blocks.len(), events.len()));
                }
            }
            Block::Title { title, authors, date, span } => {
                let Some(g) = geo else { continue };
                if g.options.titlepage {
                    // `titlepage`: `\newpage`, `\thispagestyle{empty}`,
                    // `\setcounter{page}\@ne`; at its end, one-sided,
                    // `\setcounter{page}\@ne` again.
                    events.push((blocks.len(), adapter::ChromeEvent::ThisPageStyle(flashtex_class_geometry::PageStyle::Empty), *span));
                    events.push((blocks.len(), adapter::ChromeEvent::SetPage(1), *span));
                    let built = ctx.title_blocks(title, authors, date.as_deref(), g, TitleForm::Page, n_columns);
                    blocks.extend(built);
                    if !g.flags.twoside {
                        events.push((blocks.len(), adapter::ChromeEvent::SetPage(1), *span));
                    }
                } else if n_columns > 1 && top_title.is_none() && blocks.iter().all(|b| b.vertical.lines.is_empty()) {
                    // `\twocolumn[\@maketitle]`: a `\textwidth` box above both
                    // columns of the first page.
                    let first = blocks.len();
                    let mut built = ctx.title_blocks(title, authors, date.as_deref(), g, TitleForm::Float, n_columns);
                    let p = page_params(ctx.style);
                    let vb: Vec<VBlock> = built.iter().map(|b| b.vertical.clone()).collect();
                    let (placed, height) = pagebuild::natural_layout(&p, &pagebuild::vlist(&p, &vb), false);
                    for b in &mut built {
                        b.vertical.lines.clear();
                    }
                    top_title = Some((first, placed, height));
                    blocks.extend(built);
                } else {
                    if n_columns > 1 {
                        ctx.diagnostics.push(Diagnostic::warning(
                            "unsupported_block",
                            "\\maketitle after other material in a two-column document: \\twocolumn[\\@maketitle] would start a new page; the title block is set in the column instead".to_string(),
                            vec![ctx.source(*span)],
                        ));
                    }
                    let built = ctx.title_blocks(title, authors, date.as_deref(), g, TitleForm::Flow, n_columns);
                    blocks.extend(built);
                }
                after_heading = false;
            }
            Block::Chrome { event, span } => {
                if let (adapter::ChromeEvent::PageStyle(ps), Some(g)) = (event, geo) {
                    match ps {
                        flashtex_class_geometry::PageStyle::Headings => mark_rules = flashtex_class_geometry::pagestyle::mark_rules(g.options.kind, *ps, g.options.twoside),
                        flashtex_class_geometry::PageStyle::MyHeadings => mark_rules.clear(),
                        _ => {}
                    }
                }
                events.push((blocks.len(), event.clone(), *span));
            }
            Block::Paragraph { .. } => {
                let mut st = ParaState { after_heading, env_vmode, env_skips: None };
                ctx.build_paragraph(&mut blocks, block, &mut st, cache, style_fp, quad);
                (after_heading, env_vmode) = (st.after_heading, st.env_vmode);
            }
            Block::Rule {
                span,
                eject_before,
                vspace_before,
            } => {
                let mut b = ctx.rule_block(*span);
                if *eject_before {
                    b.vertical.penalty_before = Some(pagebuild::EJECT_PENALTY);
                }
                add_vspace(&mut b.vertical, *vspace_before);
                blocks.push(b);
                after_heading = false;
            }
            Block::LongTable {
                table,
                eject_before,
                vspace_before,
                lengths,
                labels,
            } => {
                let _ = labels;
                if let Some((mut b, region)) = ctx.longtable_block(table, lengths) {
                    if *eject_before {
                        b.vertical.penalty_before = Some(pagebuild::EJECT_PENALTY);
                    }
                    add_vspace(&mut b.vertical, *vspace_before);
                    longtables.push((blocks.len(), region));
                    blocks.push(b);
                    after_heading = false;
                }
            }
            Block::Picture {
                document,
                picture,
                centered,
                eject_before,
                vspace_before,
            } => {
                let mut b = ctx.picture_block(*document, picture, *centered);
                if *eject_before {
                    b.vertical.penalty_before = Some(pagebuild::EJECT_PENALTY);
                }
                add_vspace(&mut b.vertical, *vspace_before);
                blocks.push(b);
                after_heading = false;
            }
        }
    }
    // `\twocolumn` begins with `\clearpage`: column material after
    // full-width material starts a page (`\onecolumn`'s own `\clearpage`
    // comes with the `\chapter*` heading of the list).
    if n_columns > 1 && !wide_blocks.is_empty() {
        for i in 1..blocks.len() {
            let wide = |b: usize| wide_blocks.binary_search(&b).is_ok();
            if wide(i - 1) && !wide(i) {
                blocks[i].vertical.penalty_before = Some(pagebuild::EJECT_PENALTY);
                page_start_blocks.push(i);
            }
        }
    }
    for &at in &clears {
        if let Some(b) = blocks.get_mut(at) {
            b.vertical.penalty_before = Some(pagebuild::EJECT_PENALTY);
        }
    }
    let s = style;
    let params = page_params(s);
    let vblocks: Vec<VBlock> = blocks.iter().map(|b| b.vertical.clone()).collect();
    let list = pagebuild::vlist(&params, &vblocks);
    // Footnote blocks are appended after the body's (not in `vblocks`).
    let body_blocks = blocks.len();
    let insertions = footnotes::prepare(ctx, &mut blocks, &params);
    // Two-column documents: the page builder fills columns of `\textheight`
    // (`\@colht`); `\@outputdblcol` ships the first column and the second
    // side by side, the second `\columnwidth + \columnsep` to the right.
    let columns = n_columns;
    // `\@topnewpage` (latex.ltx 20466-20505). `\twocolumn[<material>]` sets
    // the material in `\vbox{\hsize\textwidth \@parboxrestore \col@number\@ne
    // <material> \vskip -\dbltextfloatsep}`: the trailing negative skip makes
    // `\ht\@currbox` the material's natural height *less* `\dbltextfloatsep`
    // (and zero depth, the last list item being glue). `\@colht` then loses
    // `\ht\@currbox + \dbltextfloatsep` -- exactly the natural height -- and
    // `\vsize`/`\@colroom` follow it, so **both** columns of this page are
    // that much shorter; `\@outputpage` restores `\global\@colht\textheight`
    // when the page ships, so only this page is affected.
    //
    // The box is `\@cons`ed to `\@dbltoplist`, and `\@combinedblfloats`
    // (21019) stacks `\ht\@currbox`, `\vskip\dbltextfloatsep` and the
    // two-column box inside a `\vbox to\textheight` -- so the columns begin
    // the same natural height below the top of the text area that `\@colht`
    // lost. That is why one number, `top_title`'s height, is both the page's
    // shortening and the column material's shift.
    //
    // `\@topnewpage` also sets `\global\@dbltopnum\m@ne`, which suppresses
    // `\dblfigrule` and makes `\@addtodblcol` (21280) defer every later
    // full-width float off this page. Full-width floats are not modelled as
    // `\@dbltoplist` entries here at all (`figure*` is placed as `figure`,
    // floats.rs), so that part is not reproduced; see the module note.
    let (short_cols, short) = match top_title.as_ref() {
        None => (0, 0.0),
        Some(&(first, _, height)) => {
            // `\ifdim \ht\@currbox>\textheight \ht\@currbox\textheight \fi`
            // caps the box, so the shortening is capped the same way.
            let capped = height.min(params.vsize + DBLTEXTFLOATSEP_PT);
            if capped < params.vsize {
                if params.vsize - capped < 2.5 * s.baselineskip_pt {
                    top_title_warning(
                        ctx,
                        &blocks,
                        first,
                        "the optional argument of \\twocolumn is too tall for page 1 (\\@topnewpage leaves \\@colht under 2.5\\baselineskip); LaTeX would ship empty columns here, which is not implemented",
                    );
                }
                (columns, capped)
            } else {
                top_title_warning(
                    ctx,
                    &blocks,
                    first,
                    "the optional argument of \\twocolumn is taller than \\textheight; the first page's columns are not shortened by it",
                );
                (0, 0.0)
            }
        }
    };
    let (mut built, mut images, float_labels) = if let Some(b) = multicol::paginate(ctx, doc, &mut blocks, &params) {
        (b, Vec::new(), Vec::new())
    } else if floats.is_empty() {
        let (short_pages, short) = (short_cols, short);
        match &insertions {
            Some(ins) => {
                let regions = pagebuild::resolve_regions(&list, &longtables);
                let (mut pages, areas) = pagebuild::break_pages_inserts_regions(&params, &list, short_pages, short, ins, &regions);
                footnotes::place(ctx, &mut blocks, &mut pages, areas);
                (pages, Vec::new(), Vec::new())
            }
            None => {
                let regions = pagebuild::resolve_regions(&list, &longtables);
                (pagebuild::break_pages_regions(&params, &list, short_pages, short, &regions), Vec::new(), Vec::new())
            }
        }
    } else {
        let regions = pagebuild::resolve_regions(&list, &longtables);
        let (mut pages, images, labels, areas) =
            floatpage::paginate(ctx, &mut blocks, &params, &list, floats, &regions, body_blocks, insertions.as_ref(), short_cols, short, columns);
        if insertions.is_some() {
            footnotes::place(ctx, &mut blocks, &mut pages, areas);
        }
        (pages, images, labels)
    };
    // `letter.cls` line 405:
    //
    //     \def\@texttop{\ifnum\c@page=1\vskip \z@ plus.00006fil\relax\fi}
    //
    // On **page 1 only** a fil glue sits at the top of the text block. Line
    // 404's unguarded `\raggedbottom` puts `\@textbottom`'s
    // `\vskip \z@ \@plus.0001fil` at the bottom, so the page's leftover space
    // is shared between the two in the ratio of their stretch: the top takes
    // .00006/(.00006+.0001) = 3/8 of it and the first baseline moves down by
    // that much. This is page building, not a frame length, which is why
    // `crates/class-geometry`'s `letter_oracle` checks its model on page 2
    // and only bounds page 1 -- the shift belongs here.
    //
    // Without it every letter's page 1 rode 3/8 of its slack too high:
    // 41.95 bp on `fixtures/real-world/letter`, a rigid offset that put 0%
    // of the page's words within 0.5 bp on the vertical axis however exactly
    // the spacing between them was set.
    if ctx
        .style
        .class_geometry
        .as_ref()
        .is_some_and(|d| d.options.kind == flashtex_class_geometry::ClassKind::Letter)
        && ctx.style.raggedbottom
    {
        if let Some(page1) = built.first_mut() {
            let used = page1
                .lines
                .iter()
                .map(|l| l.baseline + l.depth.min(params.maxdepth))
                .fold(0.0_f64, f64::max);
            let leftover = params.vsize - used;
            if leftover > 0.0 {
                let shift = leftover * (6e-5 / (6e-5 + 1e-4));
                for l in &mut page1.lines {
                    l.baseline += shift;
                }
            }
        }
    }
    // The `\twocolumn[...]` box sits at the top of the first page
    // (`\@combinedblfloats`), both columns `\dbltextfloatsep` below it --
    // that is, the material's natural height below the top of the text area,
    // which is what `\@colht` lost above. `short` is that number after
    // `\@topnewpage`'s cap, so the shift and the shortening cannot disagree.
    if let Some((first, placed, _)) = top_title.take() {
        if built.is_empty() {
            built.push(pagebuild::BuiltPage::default());
        }
        let shift = short;
        for bp in built.iter_mut().take(columns) {
            for l in &mut bp.lines {
                l.baseline += shift;
            }
        }
        // A float's caption lines are in `built` and move with them, but its
        // graphics are display-list items the float placer already put in
        // page coordinates (`floatpage::Placer::emit`). They belong to the
        // same column box, so they take the same shift; `n` numbers the
        // page-builder column, and the first `columns` of them are this page.
        for (n, it) in &mut images {
            if (*n as usize) <= columns {
                if let display::Item::Image(img) = it {
                    let dy = Tick::from_tex_pt(shift);
                    img.top = Tick(img.top.0 + dy.0);
                    img.transform[5] += dy.to_bp();
                }
            }
        }
        let mut lines: Vec<pagebuild::Placed> = placed
            .into_iter()
            .map(|p| pagebuild::Placed {
                payload: (p.payload.0 + first, p.payload.1),
                ..p
            })
            .collect();
        lines.append(&mut built[0].lines);
        built[0].lines = lines;
    }
    // `\c@page` and `\thepage` of every page; `\cleardoublepage`'s empty
    // page (`\hbox{}\newpage`) before an `openright` chapter that would
    // start on an even page of a two-sided document.
    // Two-column documents: a page start or `\onecolumn` material that the
    // page builder put in a later column moves to the next page.
    let mut aligned: Vec<usize> = Vec::new();
    page_start_blocks.extend(clears.iter().copied());
    if columns > 1 && !(page_start_blocks.is_empty() && wide_blocks.is_empty()) {
        let flags = |list: &[usize]| {
            let mut v = vec![false; blocks.len()];
            for &b in list {
                if let Some(f) = v.get_mut(b) {
                    *f = true;
                }
            }
            v
        };
        aligned = align_columns(&mut built, columns, &flags(&page_start_blocks), &flags(&wide_blocks));
    }
    let mut blank_pages: Vec<usize> = Vec::new();
    let counters = geo.map_or_else(Vec::new, |g| {
        if g.flags.twoside && !chapter_starts.is_empty() {
            blank_pages = open_right(&mut built, &chapter_starts, blocks.len(), &events, g.numbering, columns);
        }
        page_counters(&built, columns, blocks.len(), &events, g.numbering)
    });
    let mut pages = pl::Pages {
        pages: Vec::with_capacity(built.len().div_ceil(columns)),
        overflow: Vec::new(),
        text_height: s.text_height_pt,
    };
    let mut line_dx: Vec<Vec<f64>> = Vec::with_capacity(pages.pages.capacity());
    for (ci, bp) in built.iter().enumerate() {
        let (pi, col) = (ci / columns, ci % columns);
        let number = pi as u32 + 1;
        if col == 0 {
            pages.pages.push(pl::Page {
                number,
                width: s.page_width_pt,
                height: s.page_height_pt,
                lines: Vec::with_capacity(bp.lines.len()),
                runs: Vec::new(),
            });
            line_dx.push(Vec::with_capacity(bp.lines.len()));
        }
        // `\@themargin` of this page (0 on odd and one-sided pages: the
        // blocks are assembled at `\oddsidemargin`) plus the column offset.
        let counter = counters.get(pi).map_or(i64::from(number), |c| c.0);
        let dx = geo.map_or(0.0, |g| crate::style::frame_pt(g.frame.text_left(counter)) - s.text_x_pt + crate::style::frame_pt(g.frame.columns[col].offset));
        let page = pages.pages.last_mut().expect("pushed above");
        let dxs = line_dx.last_mut().expect("pushed above");
        for placed in &bp.lines {
            let (bi, li) = placed.payload;
            let line = &blocks[bi].block.lines.lines[li];
            let y = s.text_y_pt + placed.baseline;
            page.lines.push(pl::PlacedLine {
                paragraph: bi,
                line: li,
                baseline_y: y,
                height: line.height,
                depth: line.depth,
            });
            dxs.push(dx);
            for r in &line.runs {
                let mut r = r.clone();
                r.x += s.text_x_pt + dx;
                r.baseline_y = y;
                page.runs.push(r);
            }
        }
        if bp.overfull_by > 0.0 {
            if let Some(last) = bp.lines.last() {
                let (bi, li) = last.payload;
                pages.overflow.push(pl::PageOverflow {
                    page: number,
                    paragraph: bi,
                    line: li,
                    bottom: s.text_y_pt + last.baseline + last.depth,
                    limit: s.text_y_pt + s.text_height_pt,
                });
            }
        }
    }
    multicol::shift(ctx, &mut pages, &mut line_dx, &blocks);
    if let Some(g) = geo {
        page_chrome(ctx, g, &mut blocks, &mut pages, &mut line_dx, &events, &counters);
    }
    // Float image items and labels are numbered by page-builder column
    // (before `\cleardoublepage`'s empty pages): map them to their page and
    // give them the page's margin and column offset (an even twoside page's
    // `\evensidemargin`, the second column).
    let column_page = |n: u32| -> (u32, usize) {
        let mut ci = n.saturating_sub(1) as usize;
        // Both insertion lists, in the order they were made.
        for &b in aligned.iter().chain(&blank_pages) {
            if b <= ci {
                ci += 1;
            }
        }
        ((ci / columns) as u32 + 1, ci % columns)
    };
    let images: Vec<(u32, display::Item)> = images
        .into_iter()
        .map(|(n, mut it)| {
            let (number, col) = column_page(n);
            if let Some(g) = geo {
                let counter = counters.get(number as usize - 1).map_or(i64::from(number), |c| c.0);
                let dx = crate::style::frame_pt(g.frame.text_left(counter)) - s.text_x_pt + crate::style::frame_pt(g.frame.columns[col].offset);
                if dx != 0.0 {
                    display::shift_x(&mut it, Tick::from_tex_pt(dx));
                }
            }
            (number, it)
        })
        .collect();
    let float_labels: Vec<(String, u32)> = float_labels.into_iter().map(|(k, n)| (k, column_page(n).0)).collect();
    for o in &pages.overflow {
        let span = blocks
            .get(o.paragraph)
            .and_then(|b| b.recs.iter().flatten().next().copied())
            .and_then(|r| match &ctx.recs[r] {
                BoxRec::Text { clusters, .. } => clusters.first().map(|c| c.span),
                BoxRec::Math(m) => Some(ctx.maths[*m].span),
                BoxRec::Rule { span, .. } => Some(*span),
                BoxRec::Picture(p) => Some(p.span),
                    BoxRec::Table(t) => Some(t.span),
                    BoxRec::ColorBox(c) => Some(c.span),
                    BoxRec::Leader { .. } => None,
                    BoxRec::Underline(u) => Some(u.span),
            });
        let src = span.map(|sp| vec![ctx.source(sp)]).unwrap_or_default();
        ctx.diagnostics.push(Diagnostic::warning(
            "overfull_vbox",
            format!("page {}: a line extends {:.2}pt past the text area", o.page, o.bottom - o.limit),
            src,
        ));
    }
    let page_text = counters.iter().map(|(n, numbering)| numbering.format(*n)).collect();
    Laid {
        blocks,
        pages,
        recs: std::mem::take(&mut ctx.recs),
        maths: std::mem::take(&mut ctx.maths),
        images,
        float_labels,
        line_dx,
        page_text,
    }
}

/// The text of a `\sectionmark`/`\chaptermark` under `rule` (article.cls,
/// report.cls, book.cls `\ps@headings`).
fn mark_event(rule: &flashtex_class_geometry::MarkRule, number: Option<&str>, title: &str, secnumdepth: u8) -> adapter::ChromeEvent {
    use flashtex_class_geometry::pagestyle::{MarkNumber, MarkTarget};
    let mut text = String::new();
    // book.cls `\chaptermark` outside `\if@mainmatter`: the title alone.
    if let Some(number) = number.filter(|_| i32::from(secnumdepth) > rule.number_if_depth_above) {
        match rule.number {
            // `\thesection\quad`.
            MarkNumber::Quad => {
                text.push_str(number);
                text.push(QUAD_MARK);
            }
            // `\@chapapp\ \thechapter. \ `: after the period a space token
            // (space factor 3000) and then a control space.
            MarkNumber::ChapterDot => text.push_str(&format!("Chapter {number}.{SENTENCE_SPACE_MARK} ")),
            // `\thesection. \ `.
            MarkNumber::Dot => text.push_str(&format!("{number}.{SENTENCE_SPACE_MARK} ")),
        }
    }
    text.push_str(title);
    if rule.uppercase {
        text = text.to_uppercase();
    }
    match rule.target {
        MarkTarget::Both => adapter::ChromeEvent::MarkBoth(text, String::new()),
        MarkTarget::Right => adapter::ChromeEvent::MarkRight(text),
    }
}

/// `\quad` inside mark text.
const QUAD_MARK: char = '\u{2003}';
/// An interword space at space factor 3000 (after a period) inside mark text.
const SENTENCE_SPACE_MARK: char = '\u{2002}';

/// `(\c@page, \thepage style)` of every shipped page: 1 and the class
/// numbering at the start, `\pagenumbering` resets to 1, `\setcounter{page}`
/// sets, each shipout steps. Events belong to the page holding the next
/// block (as in [`page_chrome`]).
fn page_counters(built: &[pagebuild::BuiltPage], columns: usize, n_blocks: usize, events: &[(usize, adapter::ChromeEvent, Span)], numbering: flashtex_class_geometry::Numbering) -> Vec<(i64, flashtex_class_geometry::Numbering)> {
    let n_pages = built.len().div_ceil(columns.max(1));
    if n_pages == 0 {
        return Vec::new();
    }
    let mut first_page: Vec<Option<usize>> = vec![None; n_blocks];
    for (ci, bp) in built.iter().enumerate() {
        for l in &bp.lines {
            if let Some(slot) = first_page.get_mut(l.payload.0) {
                slot.get_or_insert(ci / columns.max(1));
            }
        }
    }
    let mut per: Vec<Vec<&adapter::ChromeEvent>> = vec![Vec::new(); n_pages];
    for (b, e, _) in events {
        let page = (*b..n_blocks).find_map(|i| first_page[i]).unwrap_or(n_pages - 1);
        per[page].push(e);
    }
    let (mut counter, mut style) = (1i64, numbering);
    let mut out = Vec::with_capacity(n_pages);
    for events in per {
        for e in events {
            match e {
                adapter::ChromeEvent::PageNumbering(n) => {
                    style = *n;
                    counter = 1;
                }
                adapter::ChromeEvent::SetPage(n) => counter = *n,
                _ => {}
            }
        }
        out.push((counter, style));
        counter += 1;
    }
    out
}

/// `\cleardoublepage` before an `openright` chapter (one-column,
/// two-sided): an empty page (`\hbox{}\newpage`, the page style in force)
/// when the chapter's page would be even.
fn open_right(built: &mut Vec<pagebuild::BuiltPage>, chapter_starts: &[(usize, usize)], n_blocks: usize, events: &[(usize, adapter::ChromeEvent, Span)], numbering: flashtex_class_geometry::Numbering, columns: usize) -> Vec<usize> {
    // Indices (in the final list, ascending) of the inserted empty columns:
    // a whole page of them (`\cleardoublepage`'s `\if@twocolumn\hbox{}
    // \newpage\fi`).
    let columns = columns.max(1);
    let mut inserted = Vec::new();
    let mut k = 0;
    while k < built.len() {
        // The first `\cleardoublepage` recorded for the page's first block
        // (a page start is always a page's first column).
        let start = built[k].lines.first().filter(|_| k % columns == 0).and_then(|l| chapter_starts.iter().find(|(b, _)| *b == l.payload.0).copied());
        if let Some((block, cut)) = start {
            // `\ifodd\c@page` is tested before the counter commands issued
            // after the clear (book `\mainmatter`'s `\pagenumbering{arabic}`).
            let before: Vec<(usize, adapter::ChromeEvent, Span)> = events.iter().enumerate().filter(|(i, (b, _, _))| !(*b == block && *i >= cut)).map(|(_, e)| e.clone()).collect();
            if page_counters(built, columns, n_blocks, &before, numbering)[k / columns].0 % 2 == 0 {
                for _ in 0..columns {
                    built.insert(k, pagebuild::BuiltPage::default());
                    inserted.push(k);
                    k += 1;
                }
            }
        }
        k += 1;
    }
    inserted
}

/// Two-column documents (`\@outputdblcol`): the page builder fills
/// columns, but material after `\clearpage` (and `\chapter`/`\part`) starts
/// a new page, and `\onecolumn` material (report/book contents lists, the
/// report/book `\part` page, set at `\textwidth`) takes the whole page.
/// Inserts empty columns so such a column is a page's first and the column
/// after `\onecolumn` material starts the next page; returns the inserted
/// indices (in the final list, ascending).
fn align_columns(built: &mut Vec<pagebuild::BuiltPage>, columns: usize, page_start: &[bool], wide: &[bool]) -> Vec<usize> {
    let flag = |v: &[bool], b: usize| v.get(b).copied().unwrap_or(false);
    let mut inserted = Vec::new();
    let mut prev_wide = false;
    let mut k = 0;
    while k < built.len() {
        let starts = built[k].lines.first().is_some_and(|l| flag(page_start, l.payload.0));
        let has_wide = built[k].lines.iter().any(|l| flag(wide, l.payload.0));
        if (starts || has_wide || prev_wide) && k % columns != 0 {
            while k % columns != 0 {
                built.insert(k, pagebuild::BuiltPage::default());
                inserted.push(k);
                k += 1;
            }
        }
        prev_wide = has_wide;
        k += 1;
    }
    inserted
}

/// Header, footer and `\columnseprule` of every page (`\@outputpage`,
/// `\@outputdblcol`): the page style in force when the page ships
/// (`\pagestyle` changes before its last material count; `\thispagestyle`
/// only for its page), `\leftmark` from the page's last mark and
/// `\rightmark` from its first (`\botmark`/`\firstmark`, the previous
/// page's last mark when the page has none).
fn page_chrome(ctx: &mut Context, g: &flashtex_class_geometry::ResolvedDocument, blocks: &mut Vec<BuiltBlock>, pages: &mut pl::Pages, line_dx: &mut [Vec<f64>], events: &[(usize, adapter::ChromeEvent, Span)], counters: &[(i64, flashtex_class_geometry::Numbering)]) {
    use crate::style::frame_pt;
    use adapter::ChromeEvent;
    use flashtex_class_geometry::Field;
    let n_pages = pages.pages.len();
    if n_pages == 0 {
        return;
    }
    let mut first_page: Vec<Option<usize>> = vec![None; blocks.len()];
    for (pi, page) in pages.pages.iter().enumerate() {
        for l in &page.lines {
            if let Some(slot) = first_page.get_mut(l.paragraph) {
                slot.get_or_insert(pi);
            }
        }
    }
    let page_of = |b: usize| (b..first_page.len()).find_map(|i| first_page[i]).unwrap_or(n_pages - 1);
    let mut by_page: Vec<Vec<&ChromeEvent>> = vec![Vec::new(); n_pages];
    for (b, e, _) in events {
        by_page[page_of(*b)].push(e);
    }
    // Page numbers and rules have no source of their own.
    let span = Span::in_document(DocumentId(0), 0, 0);
    let frame = &g.frame;
    let width = frame_pt(frame.text_width);
    let text_x = ctx.style.text_x_pt;
    let mut macros = g.style_macros;
    let mut top = (String::new(), String::new());
    let mut current = top.clone();
    for pi in 0..n_pages {
        let (number, numbering) = counters.get(pi).copied().unwrap_or((pi as i64 + 1, g.numbering));
        let mut this = None;
        let mut first: Option<(String, String)> = None;
        for e in &by_page[pi] {
            match e {
                ChromeEvent::PageStyle(ps) => macros = macros.apply(*ps, g.options.twoside),
                ChromeEvent::ThisPageStyle(ps) => this = Some(*ps),
                ChromeEvent::MarkBoth(l, r) => {
                    current = (l.clone(), r.clone());
                    first.get_or_insert_with(|| current.clone());
                }
                ChromeEvent::MarkRight(r) => {
                    current.1 = r.clone();
                    first.get_or_insert_with(|| current.clone());
                }
                ChromeEvent::PageNumbering(_) | ChromeEvent::SetPage(_) => {}
            }
        }
        let first = first.unwrap_or_else(|| top.clone());
        let bot = current.clone();
        let m = this.map_or(macros, |ps| macros.apply(ps, g.options.twoside));
        let (head, foot) = m.for_page(g.flags.twoside, number);
        let page_no = numbering.format(number);
        let dx = frame_pt(frame.text_left(number)) - text_x;
        if frame.twocolumn && frame.columns.len() > 1 && ctx.style.columnseprule_pt > 0.0 {
            // `\hb@xt@\columnwidth{..}\hfil\vrule\@width\columnseprule\hfil`:
            // centred in `\columnsep`, as tall as the column boxes.
            let cw = frame_pt(frame.columns[0].width);
            let sep = frame_pt(frame.columns[1].offset) - cw;
            let rw = ctx.style.columnseprule_pt;
            let h = frame_pt(frame.text_height);
            let rule = ctx.rule_block_sized(span, rw, h, cw + (sep - rw) / 2.0);
            pages.pages[pi].lines.push(pl::PlacedLine {
                paragraph: blocks.len(),
                line: 0,
                baseline_y: frame_pt(frame.text_top) + h,
                height: h,
                depth: 0.0,
            });
            line_dx[pi].push(dx);
            blocks.push(rule);
        }
        let slot = |f: Field| -> Option<(&str, bool)> {
            match f {
                Field::Empty => None,
                Field::PageNumber => Some((page_no.as_str(), false)),
                Field::LeftMark => Some((bot.0.as_str(), true)),
                Field::RightMark => Some((first.1.as_str(), true)),
            }
        };
        for (line, baseline, at_top) in [(head, frame.head_baseline, true), (foot, frame.foot_baseline, false)] {
            if line.is_empty() {
                continue;
            }
            let Some(b) = ctx.chrome_line([slot(line.left), slot(line.center), slot(line.right)], width, span) else { continue };
            let l = &b.block.lines.lines[0];
            let placed = pl::PlacedLine {
                paragraph: blocks.len(),
                line: 0,
                baseline_y: frame_pt(baseline),
                height: l.height,
                depth: l.depth,
            };
            blocks.push(b);
            if at_top {
                pages.pages[pi].lines.insert(0, placed);
                line_dx[pi].insert(0, dx);
            } else {
                pages.pages[pi].lines.push(placed);
                line_dx[pi].push(dx);
            }
        }
        top = bot;
    }
}

/// A word of a header/footer line, or the glue between words (`Space`
/// carries TeX's space factor).
enum ChromeTok {
    Word(String),
    Space(u32),
    Quad,
}

fn chrome_tokens(text: &str) -> Vec<ChromeTok> {
    let mut out = Vec::new();
    let mut word = String::new();
    for c in text.trim().chars() {
        match c {
            ' ' | QUAD_MARK | SENTENCE_SPACE_MARK => {
                if !word.is_empty() {
                    out.push(ChromeTok::Word(std::mem::take(&mut word)));
                }
                out.push(match c {
                    ' ' => ChromeTok::Space(1000),
                    SENTENCE_SPACE_MARK => ChromeTok::Space(3000),
                    _ => ChromeTok::Quad,
                });
            }
            _ => word.push(c),
        }
    }
    if !word.is_empty() {
        out.push(ChromeTok::Word(word));
    }
    out
}

fn one_line_stats() -> pl::Stats {
    pl::Stats {
        algorithm: pl::Algorithm::TotalFit,
        lines: 1,
        pass: 1,
        total_demerits: 0.0,
        overfull: Vec::new(),
        underfull: Vec::new(),
        hyphenated_lines: 0,
        emergency_pass_used: false,
    }
}

/// The page each `\label` landed on (the page of the line holding the item
/// it precedes, or the block's last line when it ends the block).
pub fn label_pages(laid: &Laid) -> BTreeMap<String, u32> {
    let mut out = BTreeMap::new();
    for (bi, block) in laid.blocks.iter().enumerate() {
        for (key, item) in &block.labels {
            let lines = &block.block.lines.lines;
            let li = lines
                .iter()
                .position(|l| l.items.contains(item))
                .unwrap_or(lines.len().saturating_sub(1));
            let page = laid
                .pages
                .pages
                .iter()
                .find(|p| p.lines.iter().any(|pl| pl.paragraph == bi && pl.line == li))
                .map(|p| p.number)
                .unwrap_or(1);
            out.insert(key.clone(), page);
        }
    }
    for (key, page) in &laid.float_labels {
        out.insert(key.clone(), *page);
    }
    out
}

/// `\thepage` of every contents-list entry, from the pages of its synthetic
/// label (`crate::toc::key`) in `pages` ([`label_pages`] of `laid`).
pub fn toc_pages(laid: &Laid, pages: &BTreeMap<String, u32>) -> BTreeMap<String, String> {
    pages
        .iter()
        .filter(|(key, _)| crate::toc::is_key(key))
        .map(|(key, page)| {
            let text = laid.page_text.get((*page as usize).wrapping_sub(1)).cloned().unwrap_or_else(|| page.to_string());
            (key.clone(), text)
        })
        .collect()
}

/// Converts the placed pages into the display list.
pub fn assemble(
    project_id: &str,
    revision: u64,
    documents: &[SourceDocument<'_>],
    style: &Stylesheet,
    _fonts: &FontSet,
    laid: Laid,
    mut diagnostics: Vec<Diagnostic>,
    cache: Option<&RenderCache>,
    page_color: Option<flashtex_compiler::color::DeviceColor>,
    default_color: Option<flashtex_compiler::color::DeviceColor>,
) -> DisplayList {
    let paths: Vec<Rc<str>> = documents.iter().map(|d| Rc::from(d.path)).collect();
    let empty: Rc<str> = Rc::from("");
    let source_of = |span: Span| SourceRange {
        path: paths.get(span.document.0).cloned().unwrap_or_else(|| empty.clone()),
        start_byte: span.start,
        end_byte: span.end,
    };
    let mut used: BTreeMap<Rc<str>, Rc<LoadedFace>> = BTreeMap::new();
    // Every block's lines are assembled once in line-local coordinates
    // (cached across requests by the block's key), then placed per page by
    // integer tick/byte moves.
    let text_x = style.text_x_pt;
    let mut assembled: Vec<Option<Rc<incremental::AssembledBlock>>> = Vec::with_capacity(laid.blocks.len());
    for block in &laid.blocks {
        let hit = block
            .cache_key
            .and_then(|(k, _, _)| cache.and_then(|c| c.assembled(k)))
            .filter(|a| block.cache_key.is_some_and(|(_, d, _)| *a.path == *paths.get(d.0).map_or("", |p| &**p)));
        let a = match hit {
            Some(a) => a,
            None => {
                let built = assemble_block(block, &laid.recs, &laid.maths, text_x, &source_of, &paths, &empty);
                match (cache, block.cache_key) {
                    (Some(c), Some((k, _, _))) => c.insert_assembled(k, built),
                    _ => Rc::new(built),
                }
            }
        };
        for f in &a.faces {
            used.entry(f.font_id.clone()).or_insert_with(|| f.clone());
        }
        assembled.push(Some(a));
    }
    let mut pages = Vec::new();
    for (pi, page) in laid.pages.pages.iter().enumerate() {
        let mut items: Vec<display::Item> = Vec::new();
        for (li, placed) in page.lines.iter().enumerate() {
            let block = &laid.blocks[placed.paragraph];
            let Some(a) = assembled[placed.paragraph].as_ref() else { continue };
            let Some(line_items) = a.lines.get(placed.line) else { continue };
            let dy = Tick::from_tex_pt(placed.baseline_y);
            let dx = laid.line_dx.get(pi).and_then(|d| d.get(li)).map_or(Tick(0), |d| Tick::from_tex_pt(*d));
            let delta = block.cache_key.map_or(0, |(_, _, b)| b as isize - a.base as isize);
            for it in line_items {
                let mut item = incremental::place_item(it, dy, &a.path, delta);
                if dx.0 != 0 {
                    display::shift_x(&mut item, dx);
                }
                items.push(item);
            }
        }
        items.extend(laid.images.iter().filter(|(n, _)| *n == page.number).map(|(_, it)| it.clone()));
        if let Some(color) = default_color {
            // Under a target model the default colour is written too.
            for item in &mut items {
                let paint = match item {
                    display::Item::GlyphRun(r) => &mut r.paint,
                    display::Item::Rule(r) => &mut r.paint,
                    _ => continue,
                };
                if paint.device.is_none() {
                    *paint = Paint::of(Some(color));
                }
            }
        }
        if let Some(color) = page_color {
            // pdfTeX paints `\pagecolor` before the page: `q 0 0 W H re f Q`.
            items.insert(
                0,
                display::Item::Rule(Rule {
                    x: Tick(0),
                    top: Tick(0),
                    width: Tick::from_tex_pt(page.width),
                    height: Tick::from_tex_pt(page.height),
                    paint: Paint::of(Some(color)),
                    provenance: Provenance::Synthetic("\\pagecolor".into()),
                }),
            );
        }
        pages.push(display::Page {
            number: page.number,
            width: Tick::from_tex_pt(page.width),
            height: Tick::from_tex_pt(page.height),
            items,
        });
    }
    // Resource selection provenance: which outline resource drew each TFM
    // font's glyphs. The roman family has exact optical siblings
    // (lmroman12/8/6 for lmr12/8/6); the italic, symbol and extension
    // families only exist as the single-design Latin Modern Math, which is
    // reported rather than passed off as the reference's lmmi/lmsy/lmex.
    // Collected over every provider (cached blocks keep the provider that
    // built them), then emitted in TFM-name order without a source so the
    // report does not depend on which block was built first.
    let mut profiles: BTreeMap<String, String> = BTreeMap::new();
    for a in assembled.iter().flatten() {
        for (tfm, face, exact) in &a.resources {
            if !exact {
                profiles.entry(tfm.clone()).or_insert(face.clone());
            }
        }
    }
    for (tfm, face) in profiles {
        diagnostics.push(Diagnostic::warning(
            "math_resource_profile",
            format!("{tfm}: glyphs drawn from {face} (one 10pt design); no optical-size OpenType outline resource exists for this family, so the outlines are not the reference's {tfm} design"),
            Vec::new(),
        ));
    }
    // Glyphs TeX's metrics placed that the OpenType face cannot draw
    // (extensible assemblies, unknown chains): reported, not faked.
    let mut reported = BTreeSet::new();
    for (block, a) in laid.blocks.iter().zip(assembled.iter()) {
        let Some(a) = a else { continue };
        for (font, code, ch) in &a.unmapped {
            if reported.insert((font.clone(), *code)) {
                let span = block.recs.iter().flatten().find_map(|r| match &laid.recs[*r] {
                    BoxRec::Math(mi) => Some(laid.maths[*mi].span),
                    BoxRec::Rule { span, .. } => Some(*span),
                    BoxRec::Picture(p) => Some(p.span),
                    BoxRec::Table(t) => Some(t.span),
                    BoxRec::ColorBox(c) => Some(c.span),
                    BoxRec::Leader { .. } => None,
                    BoxRec::Underline(u) => Some(u.span),
                    BoxRec::Text { .. } => None,
                });
                diagnostics.push(Diagnostic::warning(
                    "math_glyph_unmapped",
                    format!("{font} code {code:#04x} ('{ch}') has no Latin Modern Math glyph mapping; nothing drawn for it"),
                    span.map(|s| vec![source_of(s)]).unwrap_or_default(),
                ));
            }
        }
    }
    let fonts = used
        .values()
        .map(|f| FontResource {
            font_id: f.font_id.clone(),
            sha256: f.font_id.to_string(),
            byte_length: f.byte_length,
            format: f.format.to_string(),
            face_index: 0,
            units_per_em: f.units_per_em,
            glyph_count: f.glyph_count,
            postscript_name: f.postscript_name.clone(),
            path: f.path.as_ref().map(|p| p.display().to_string()),
        })
        .collect();
    let docs = documents
        .iter()
        .map(|d| DocumentResource {
            path: d.path.to_string(),
            revision,
            sha256: sha256::hex(&sha256::digest(d.text.as_bytes())),
            byte_length: d.text.len() as u64,
        })
        .collect();
    let _ = style;
    DisplayList {
        project_id: project_id.to_string(),
        revision,
        documents: docs,
        fonts,
        pages,
        diagnostics,
    }
}

/// Assembles one block's lines in line-local coordinates.
#[allow(clippy::too_many_arguments)]
fn assemble_block(
    block: &BuiltBlock,
    recs: &[BoxRec],
    maths: &[MathRec],
    text_x: f64,
    source_of: &dyn Fn(Span) -> SourceRange,
    paths: &[Rc<str>],
    empty: &Rc<str>,
) -> incremental::AssembledBlock {
    let mut used: BTreeMap<Rc<str>, Rc<LoadedFace>> = BTreeMap::new();
    let mut lines = Vec::with_capacity(block.block.lines.lines.len());
    let mut resources = Vec::new();
    let mut unmapped = Vec::new();
    for line in &block.block.lines.lines {
        let mut items: Vec<display::Item> = Vec::new();
        // Boxes of this line in item order pair with its runs in order.
        let boxes: Vec<usize> = line
            .items
            .clone()
            .filter(|i| matches!(block.items.get(*i), Some(pl::Item::Box(_))))
            .filter_map(|i| block.recs.get(i).copied().flatten())
            .collect();
        let mut bi = 0usize;
        for run in &line.runs {
            // A discretionary hyphen is the pre-break text of the penalty
            // the line breaks at; its box record sits at that item.
            let rec = if run.is_hyphen {
                block.block.lines.breaks.get(line.index).and_then(|b| block.recs.get(b.item).copied().flatten())
            } else {
                let r = boxes.get(bi).copied();
                bi += 1;
                r
            };
            let Some(rec) = rec else {
                if run.is_hyphen {
                    continue;
                }
                break;
            };
            let mut local = run.clone();
            local.x += text_x;
            local.baseline_y = 0.0;
            match &recs[rec] {
                BoxRec::Text {
                    face,
                    size,
                    text,
                    clusters,
                    glyphs,
                    height,
                    depth,
                    continues,
                    style,
                    raise,
                    ..
                } => {
                    used.entry(face.font_id.clone()).or_insert_with(|| face.clone());
                    if let Some(item) = text_item(&local, face, *size, text, clusters, glyphs, *height, *depth, *raise, source_of, Paint::of(style.color)) {
                        match (items.last_mut(), item) {
                            (Some(display::Item::GlyphRun(prev)), display::Item::GlyphRun(next)) if *continues && prev.font_id == next.font_id && prev.font_size == next.font_size && prev.paint == next.paint => {
                                join_runs(prev, next);
                            }
                            (_, item) => items.push(item),
                        }
                    }
                }
                BoxRec::Math(mi) => {
                    let m = &maths[*mi];
                    math_items(&local, m, source_of, &mut items, &mut used);
                    if let MathProvider::Tex(t) = &m.metrics {
                        resources.extend(t.take_resources());
                        unmapped.extend(t.take_unmapped());
                    }
                }
                BoxRec::Picture(p) => picture_items(&local, p, source_of, &mut items, &mut used),
                BoxRec::Table(t) => {
                    // `device: None`: `crate::tablecolor` has already flattened the
                    // colortbl colour to sRGB, so the operands pdfTeX would write
                    // (`k`/`rg`/`g`) are gone by here. Filling this in needs the
                    // colour resolved by `crate::color` in the compiler instead --
                    // see `tablecolor`'s module comment.
                    let paint_of = |r: &crate::table::PlacedRule| {
                        r.rgb.map_or(Paint::BLACK, |[r, g, b]| Paint { r, g, b, a: 1.0, device: None })
                    };
                    // colortbl's leaders come before the entry in each cell.
                    for r in &t.fills {
                        items.push(display::Item::Rule(Rule {
                            x: Tick::from_tex_pt(local.x + r.x),
                            top: Tick::from_tex_pt(r.top),
                            width: Tick::from_tex_pt(r.width).max(Tick(1)),
                            height: Tick::from_tex_pt(r.height).max(Tick(1)),
                            paint: paint_of(r),
                            provenance: Provenance::Source(source_of(r.span)),
                        }));
                    }
                    // Each piece is assembled like a block of its own, then
                    // moved to its place; the rules follow the text.
                    for piece in &t.pieces {
                        let a = assemble_block(&piece.block, recs, maths, 0.0, source_of, paths, empty);
                        let piece_lines = &piece.block.block.lines.lines;
                        let first = piece_lines.first().map_or(0.0, |l| l.baseline_y);
                        let dx = Tick::from_tex_pt(local.x + piece.x);
                        for (li, line_items) in a.lines.iter().enumerate() {
                            let dy = piece.baseline + piece_lines.get(li).map_or(0.0, |l| l.baseline_y - first);
                            let dy = Tick::from_tex_pt(dy);
                            for it in line_items {
                                let mut item = incremental::place_item(it, dy, "", 0);
                                display::shift_x(&mut item, dx);
                                items.push(item);
                            }
                        }
                        for f in a.faces {
                            used.entry(f.font_id.clone()).or_insert(f);
                        }
                        resources.extend(a.resources);
                        unmapped.extend(a.unmapped);
                    }
                    for r in &t.rules {
                        items.push(display::Item::Rule(Rule {
                            x: Tick::from_tex_pt(local.x + r.x),
                            top: Tick::from_tex_pt(r.top),
                            width: Tick::from_tex_pt(r.width).max(Tick(1)),
                            height: Tick::from_tex_pt(r.height).max(Tick(1)),
                            paint: paint_of(r),
                            provenance: Provenance::Source(source_of(r.span)),
                        }));
                    }
                }
                BoxRec::ColorBox(cb) => {
                    // xcolor draws the fill (`\color@block`: a `\vrule`), the
                    // content, then the frame (`\boxframe`: top and bottom
                    // `\hrule`s, side `\vrule`s `\fboxrule` shorter, half a
                    // rule inside each end).
                    let x0 = local.x;
                    let provenance = Provenance::Source(source_of(cb.span));
                    let block_rule = |x: f64, top: f64, w: f64, h: f64, color| {
                        display::Item::Rule(Rule {
                            x: Tick::from_tex_pt(x),
                            top: Tick::from_tex_pt(top),
                            width: Tick::from_tex_pt(w).max(Tick(1)),
                            height: Tick::from_tex_pt(h).max(Tick(1)),
                            paint: Paint::of(Some(color)),
                            provenance: provenance.clone(),
                        })
                    };
                    let r = cb.rule;
                    items.push(block_rule(x0 + r, r - cb.height, cb.width - 2.0 * r, cb.height + cb.depth - 2.0 * r, cb.fill));
                    let a = assemble_block(&cb.block, recs, maths, 0.0, source_of, paths, empty);
                    let dx = Tick::from_tex_pt(x0);
                    for line_items in &a.lines {
                        for it in line_items {
                            let mut item = incremental::place_item(it, Tick(0), "", 0);
                            display::shift_x(&mut item, dx);
                            items.push(item);
                        }
                    }
                    for f in a.faces {
                        used.entry(f.font_id.clone()).or_insert(f);
                    }
                    resources.extend(a.resources);
                    unmapped.extend(a.unmapped);
                    if let Some(frame) = cb.frame {
                        let total = cb.height + cb.depth;
                        items.push(block_rule(x0, -cb.height, cb.width, r, frame));
                        items.push(block_rule(x0, r / 2.0 - cb.height, r, total - r, frame));
                        items.push(block_rule(x0 + cb.width - r, r / 2.0 - cb.height, r, total - r, frame));
                        items.push(block_rule(x0, cb.depth - r, cb.width, r, frame));
                    }
                }
                BoxRec::Underline(ul) => {
                    let x0 = local.x;
                    let a = assemble_block(&ul.block, recs, maths, 0.0, source_of, paths, empty);
                    let dx = Tick::from_tex_pt(x0);
                    for line_items in &a.lines {
                        for it in line_items {
                            let mut item = incremental::place_item(it, Tick(0), "", 0);
                            display::shift_x(&mut item, dx);
                            items.push(item);
                        }
                    }
                    for f in a.faces {
                        used.entry(f.font_id.clone()).or_insert(f);
                    }
                    resources.extend(a.resources);
                    unmapped.extend(a.unmapped);
                    if ul.width > 0.0 && ul.thickness > 0.0 {
                        items.push(display::Item::Rule(Rule {
                            x: Tick::from_tex_pt(x0),
                            top: Tick::from_tex_pt(ul.ul_depth),
                            width: Tick::from_tex_pt(ul.width).max(Tick(1)),
                            height: Tick::from_tex_pt(ul.thickness).max(Tick(1)),
                            paint: Paint::BLACK,
                            provenance: Provenance::Source(source_of(ul.span)),
                        }));
                    }
                }
                BoxRec::Leader { .. } => {}
                BoxRec::Rule { width, height, bottom, span } => {
                    // Line-local like text: the rule's bottom is `bottom`
                    // above the baseline (0 for `\hrule`); a strut paints
                    // nothing.
                    if *width <= 0.0 || *height <= 0.0 {
                        continue;
                    }
                    items.push(display::Item::Rule(Rule {
                        x: Tick::from_tex_pt(local.x),
                        top: Tick::from_tex_pt(-(bottom + height)),
                        width: Tick::from_tex_pt(*width).max(Tick(1)),
                        height: Tick::from_tex_pt(*height).max(Tick(1)),
                        paint: Paint::BLACK,
                        provenance: Provenance::Source(source_of(*span)),
                    }));
                }
            }
        }
        append_leaders(block, line, recs, text_x, &mut items, &mut used);
        lines.push(items);
    }
    let (document, base) = block.cache_key.map_or((DocumentId(0), 0), |(_, d, b)| (d, b));
    incremental::AssembledBlock {
        lines,
        faces: used.into_values().collect(),
        base,
        path: paths.get(document.0).cloned().unwrap_or_else(|| empty.clone()),
        resources,
        unmapped,
    }
}

fn line_stretch_order(items: &[pl::Item], range: Range<usize>) -> Option<pl::GlueOrder> {
    let mut order = None;
    for i in range {
        let Some(pl::Item::Glue(g)) = items.get(i) else { continue };
        if g.stretch > 0.0 {
            order = Some(order.map_or(g.stretch_order, |o: pl::GlueOrder| o.max(g.stretch_order)));
        }
    }
    order
}

fn line_glue_width(g: &pl::Glue, line: &pl::Line, order: Option<pl::GlueOrder>) -> f64 {
    if line.ratio >= 0.0 {
        if order == Some(g.stretch_order) && line.ratio.is_finite() {
            g.width + line.ratio * g.stretch
        } else {
            g.width
        }
    } else {
        g.width + line.ratio * g.shrink
    }
}

fn append_leaders(
    block: &BuiltBlock,
    line: &pl::Line,
    recs: &[BoxRec],
    text_x: f64,
    items: &mut Vec<display::Item>,
    used: &mut BTreeMap<Rc<str>, Rc<LoadedFace>>,
) {
    let Some(first_box) = line.items.clone().find(|i| matches!(block.items.get(*i), Some(pl::Item::Box(_)))) else { return };
    let Some(first_run) = line.runs.iter().position(|r| !r.is_hyphen) else { return };
    let order = line_stretch_order(&block.items, line.items.clone());
    let prefix = line.items.start..first_box;
    let prefix_width: f64 = prefix
        .filter_map(|i| block.items.get(i))
        .map(|item| match item {
            pl::Item::Glue(g) => line_glue_width(g, line, order),
            pl::Item::Kern(k) => k.width,
            _ => 0.0,
        })
        .sum();
    let pre_break_width: f64 = line.runs[..first_run].iter().filter(|r| r.is_hyphen).map(|r| r.width).sum();
    let Some(run) = line.runs.get(first_run) else { return };
    let mut x = run.x - prefix_width - pre_break_width;
    let mut run_i = first_run;
    for i in line.items.clone() {
        let Some(item) = block.items.get(i) else { continue };
        match item {
            pl::Item::Box(_) => {
                while line.runs.get(run_i).is_some_and(|r| r.is_hyphen) {
                    run_i += 1;
                }
                if let Some(run) = line.runs.get(run_i) {
                    x = run.x + run.width;
                    run_i += 1;
                }
            }
            pl::Item::Glue(g) => {
                let width = line_glue_width(g, line, order);
                if let Some(BoxRec::Leader { leader, box_width, dot }) = block
                    .recs
                    .get(i)
                    .copied()
                    .flatten()
                    .and_then(|r| recs.get(r))
                {
                    match leader {
                        FillLeader::Rule if width > 0.0 => items.push(display::Item::Rule(Rule {
                            x: Tick::from_tex_pt(text_x + x),
                            top: Tick::from_tex_pt(-0.4),
                            width: Tick::from_tex_pt(width).max(Tick(1)),
                            height: Tick::from_tex_pt(0.4).max(Tick(1)),
                            paint: Paint::BLACK,
                            provenance: Provenance::Synthetic("\\hrulefill".into()),
                        })),
                        FillLeader::Dots if width >= *box_width && *box_width > 0.0 => {
                            if let Some((face, dot_run)) = dot {
                                if let Some(item) = dots_item(text_x + x, width, *box_width, face, dot_run) {
                                    used.entry(face.font_id.clone()).or_insert_with(|| face.clone());
                                    items.push(item);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                x += width;
            }
            pl::Item::Kern(k) => x += k.width,
            pl::Item::Penalty(_) => {}
        }
    }
}

fn dots_item(x: f64, glue_width: f64, box_width: f64, face: &Rc<LoadedFace>, dot: &pl::GlyphRun) -> Option<display::Item> {
    let count = (glue_width / box_width).floor() as usize;
    if count == 0 || dot.glyphs.is_empty() {
        return None;
    }
    let leftover = glue_width - count as f64 * box_width;
    let top = Tick::from_tex_pt(-dot.height);
    let height = Tick::from_tex_pt(dot.height + dot.depth);
    let mut glyphs = Vec::with_capacity(count * dot.glyphs.len());
    let mut clusters = Vec::with_capacity(count);
    let mut text = String::with_capacity(count);
    for i in 0..count {
        let dot_x = x + leftover / 2.0 + i as f64 * box_width + (box_width - dot.width) / 2.0;
        let cluster = i as u32;
        text.push('.');
        for g in &dot.glyphs {
            if g.gid != 0 {
                glyphs.push(Glyph {
                    gid: g.gid as u16,
                    origin_x: Tick::from_tex_pt(dot_x),
                    baseline_y: Tick(0),
                    advance_x: Tick::from_tex_pt(g.advance),
                    advance_y: Tick(0),
                    cluster,
                });
            }
        }
        let x0 = Tick::from_tex_pt(dot_x);
        let x1 = Tick::from_tex_pt(dot_x + dot.width);
        clusters.push(Cluster {
            text_start_byte: i,
            text_end_byte: i + 1,
            hit_rect: Rect { x: x0, top, width: Tick(x1.0 - x0.0), height },
            carets: display::Carets {
                first: Caret { text_byte: i, x: x0, top, height },
                last: (i + 1 == count).then(|| Caret { text_byte: i + 1, x: x1, top, height }),
            },
            provenance: Provenance::Synthetic("\\dotfill".into()),
        });
    }
    (!glyphs.is_empty()).then_some(display::Item::GlyphRun(GlyphRun {
        font_id: face.font_id.clone(),
        font_size: Tick::from_tex_pt(dot.size),
        text,
        glyphs,
        clusters,
        paint: Paint::BLACK,
        role: display::RunRole::Text,
    }))
}

/// A picture's paths and node text in line-local coordinates: the picture's
/// bottom-left corner is the run origin on the baseline. Paths keep the
/// picture's paint order; clip groups become per-item `clips`; each node
/// text is one glyph run (one cluster per glyph, the source range of the
/// statement that made the node).
fn picture_items(
    run: &pl::PositionedRun,
    p: &PictureRec,
    source_of: &dyn Fn(Span) -> SourceRange,
    items: &mut Vec<display::Item>,
    used: &mut BTreeMap<Rc<str>, Rc<LoadedFace>>,
) {
    use flashtex_vector_graphics as vg;
    const PT_PER_BP: f64 = 72.27 / 72.0;
    let height_pt = p.picture.height_bp * PT_PER_BP;
    let x0 = run.x;
    let tx = |x_bp: f64| Tick::from_tex_pt(x0 + x_bp * PT_PER_BP);
    let ty = |y_bp: f64| Tick::from_tex_pt(-height_pt + y_bp * PT_PER_BP);
    let len = |bp: f64| Tick::from_tex_pt(bp * PT_PER_BP);
    let source = source_of(p.span);
    let paint = |pt: &vg::Paint| {
        let (r, g, b) = pt.color.to_rgb();
        Paint {
            r,
            g,
            b,
            a: pt.alpha.clamp(0.0, 1.0),
            device: None,
        }
    };
    let conv = |path: &vg::Path| -> Vec<display::PathCmd> {
        let mut cur = vg::Point::ZERO;
        path.commands()
            .iter()
            .map(|c| match *c {
                vg::PathCommand::MoveTo(q) => {
                    cur = q;
                    display::PathCmd::Move(tx(q.x), ty(q.y))
                }
                vg::PathCommand::LineTo(q) => {
                    cur = q;
                    display::PathCmd::Line(tx(q.x), ty(q.y))
                }
                vg::PathCommand::QuadTo(c1, q) => {
                    let (a, b) = (cur.lerp(c1, 2.0 / 3.0), q.lerp(c1, 2.0 / 3.0));
                    cur = q;
                    display::PathCmd::Cubic(tx(a.x), ty(a.y), tx(b.x), ty(b.y), tx(q.x), ty(q.y))
                }
                vg::PathCommand::CubicTo(a, b, q) => {
                    cur = q;
                    display::PathCmd::Cubic(tx(a.x), ty(a.y), tx(b.x), ty(b.y), tx(q.x), ty(q.y))
                }
                vg::PathCommand::Close => display::PathCmd::Close,
            })
            .collect()
    };
    let mk = |item: &vg::Item, clips: &[display::ClipPath]| -> Option<display::Item> {
        let (op, path, pnt) = match item {
            vg::Item::PathFill(f) => (display::PathPaintOp::Fill { even_odd: f.rule == vg::FillRule::EvenOdd }, &f.path, &f.paint),
            vg::Item::PathStroke(s) => (
                display::PathPaintOp::Stroke(display::Stroke {
                    width: len(s.style.width).max(Tick(1)),
                    cap: match s.style.cap {
                        vg::LineCap::Butt => display::LineCap::Butt,
                        vg::LineCap::Round => display::LineCap::Round,
                        vg::LineCap::Square => display::LineCap::Square,
                    },
                    join: match s.style.join {
                        vg::LineJoin::Miter => display::LineJoin::Miter,
                        vg::LineJoin::Round => display::LineJoin::Round,
                        vg::LineJoin::Bevel => display::LineJoin::Bevel,
                    },
                    miter_limit: s.style.miter_limit,
                    dash: s.style.dash.as_ref().map(|d| d.array.iter().map(|v| len(*v)).collect()).unwrap_or_default(),
                    dash_phase: s.style.dash.as_ref().map_or(Tick(0), |d| len(d.phase)),
                }),
                &s.path,
                &s.paint,
            ),
            _ => return None,
        };
        Some(display::Item::Path(display::PathItem {
            op,
            commands: conv(path),
            clips: clips.to_vec(),
            paint: paint(pnt),
            provenance: Provenance::Source(source.clone()),
        }))
    };
    let clip_of = |c: &vg::Clip| match c {
        vg::Clip::Path { path, rule } => display::ClipPath {
            commands: conv(path),
            even_odd: *rule == vg::FillRule::EvenOdd,
        },
        vg::Clip::Rect(r) => display::ClipPath {
            commands: conv(&vg::Path::rect(*r)),
            even_odd: false,
        },
    };
    fn walk(
        item: &vg::Item,
        clips: &mut Vec<display::ClipPath>,
        out: &mut Vec<display::Item>,
        mk: &dyn Fn(&vg::Item, &[display::ClipPath]) -> Option<display::Item>,
        clip_of: &dyn Fn(&vg::Clip) -> display::ClipPath,
    ) {
        if let vg::Item::Group(g) = item {
            let pushed = match &g.clip {
                Some(c) => {
                    clips.push(clip_of(c));
                    true
                }
                None => false,
            };
            for child in &g.items {
                walk(child, clips, out, mk, clip_of);
            }
            if pushed {
                clips.pop();
            }
        } else if let Some(i) = mk(item, clips) {
            out.push(i);
        }
    }
    let emit_text = |ti: usize, items: &mut Vec<display::Item>, used: &mut BTreeMap<Rc<str>, Rc<LoadedFace>>| {
        let t = &p.picture.texts[ti];
        let Some(g) = p.texts.get(ti) else { return };
        if g.glyphs.is_empty() {
            return;
        }
        used.entry(g.face.font_id.clone()).or_insert_with(|| g.face.clone());
        let (bx, by) = (t.transform.e, t.transform.f);
        let baseline = ty(by);
        let top = Tick::from_tex_pt(-height_pt + by * PT_PER_BP - g.metrics.height_pt);
        let box_h = Tick::from_tex_pt(g.metrics.height_pt + g.metrics.depth_pt).max(Tick(1));
        let node_source = source_of(Span::in_document(p.span.document, t.source.0, t.source.1));
        let n = g.glyphs.len();
        let mut glyphs = Vec::with_capacity(n);
        let mut clusters = Vec::with_capacity(n);
        for (ci, sg) in g.glyphs.iter().enumerate() {
            let ox = tx(bx + sg.x_pt / PT_PER_BP);
            let adv = Tick::from_tex_pt(sg.advance_pt);
            glyphs.push(display::Glyph {
                gid: sg.gid,
                origin_x: ox,
                baseline_y: baseline,
                advance_x: adv,
                advance_y: Tick(0),
                cluster: ci as u32,
            });
            // Clusters partition the text: spaces belong to the glyph before.
            let start = if ci == 0 { 0 } else { sg.text_range.start };
            let end = g.glyphs.get(ci + 1).map_or(t.text.len(), |next| next.text_range.start).max(start);
            let last = (ci + 1 == n).then(|| display::Caret {
                text_byte: t.text.len(),
                x: Tick(ox.0 + adv.0),
                top,
                height: box_h,
            });
            clusters.push(display::Cluster {
                text_start_byte: start,
                text_end_byte: end,
                hit_rect: display::Rect {
                    x: ox,
                    top,
                    width: adv.max(Tick(1)),
                    height: box_h,
                },
                carets: display::Carets {
                    first: display::Caret {
                        text_byte: start,
                        x: ox,
                        top,
                        height: box_h,
                    },
                    last,
                },
                provenance: Provenance::Source(node_source.clone()),
            });
        }
        items.push(display::Item::GlyphRun(display::GlyphRun {
            font_id: g.face.font_id.clone(),
            font_size: Tick::from_tex_pt(t.style.size_pt),
            text: t.text.clone(),
            glyphs,
            clusters,
            paint: paint(&t.paint),
            role: display::RunRole::Text,
        }));
    };
    let mut ti = 0;
    let mut clips = Vec::new();
    for (idx, it) in p.picture.items.iter().enumerate() {
        while ti < p.picture.texts.len() && p.picture.texts[ti].after_item <= idx {
            emit_text(ti, items, used);
            ti += 1;
        }
        walk(it, &mut clips, items, &mk, &clip_of);
    }
    while ti < p.picture.texts.len() {
        emit_text(ti, items, used);
        ti += 1;
    }
}

#[allow(clippy::too_many_arguments)]
/// Appends a word fragment's run to the run it continues: one text, the
/// clusters re-based onto it, the caret at the join dropped (only the last
/// cluster of a run carries the trailing caret).
fn join_runs(prev: &mut GlyphRun, next: GlyphRun) {
    let offset = prev.text.len();
    let base = prev.clusters.len() as u32;
    if let Some(last) = prev.clusters.last_mut() {
        last.carets.last = None;
    }
    prev.text.push_str(&next.text);
    prev.glyphs.extend(next.glyphs.into_iter().map(|mut g| {
        g.cluster += base;
        g
    }));
    prev.clusters.extend(next.clusters.into_iter().map(|mut c| {
        c.text_start_byte += offset;
        c.text_end_byte += offset;
        // Carets are byte offsets into the same run text: re-base them too
        // (a caret outside its cluster is refused by rendering-core and the
        // Mac consumer, which then shows no frame at all).
        c.carets.first.text_byte += offset;
        if let Some(last) = c.carets.last.as_mut() {
            last.text_byte += offset;
        }
        c
    }));
}

fn text_item(
    run: &pl::PositionedRun,
    face: &Rc<LoadedFace>,
    size: f64,
    text: &str,
    clusters: &[ClusterRec],
    recs: &[GlyphRec],
    height: f64,
    depth: f64,
    raise: f64,
    source_of: &dyn Fn(Span) -> SourceRange,
    paint: Paint,
) -> Option<display::Item> {
    // Line-local: the baseline is 0 and every y is an offset from it; the
    // page position is added as an integer tick move when the line is
    // placed, so a block placed anywhere yields identical ticks.
    let baseline = -raise;
    let top = Tick::from_tex_pt(baseline - height);
    let box_height = Tick::from_tex_pt(height + depth);
    let mut glyphs = Vec::with_capacity(run.glyphs.len());
    let mut origins = Vec::with_capacity(run.glyphs.len());
    // Cluster glyph ranges are contiguous and ordered: walk them alongside
    // the glyphs instead of searching per glyph.
    let mut ci = 0usize;
    for (i, g) in run.glyphs.iter().enumerate() {
        let rec = recs.get(i)?;
        while ci + 1 < clusters.len() && i >= clusters[ci].glyphs.end {
            ci += 1;
        }
        let x = run.x + g.x_offset + face.pt(i64::from(rec.x_offset_units), size);
        let y = baseline - face.pt(i64::from(rec.y_offset_units), size);
        origins.push((run.x + g.x_offset, g.advance));
        if rec.gid == 0 {
            continue;
        }
        let cluster = if clusters.get(ci).is_some_and(|c| c.glyphs.contains(&i)) { ci as u32 } else { clusters.iter().position(|c| c.glyphs.contains(&i)).unwrap_or(0) as u32 };
        glyphs.push(Glyph {
            gid: rec.gid,
            origin_x: Tick::from_tex_pt(x),
            baseline_y: Tick::from_tex_pt(y),
            advance_x: Tick::from_tex_pt(g.advance),
            advance_y: Tick(0),
            cluster,
        });
    }
    if glyphs.is_empty() {
        return None;
    }
    let last_index = clusters.len().saturating_sub(1);
    let out_clusters = clusters
        .iter()
        .enumerate()
        .map(|(ci, c)| {
            let (x0, x1) = match (origins.get(c.glyphs.start), origins.get(c.glyphs.end.saturating_sub(1))) {
                (Some(a), Some(b)) => (a.0, b.0 + b.1),
                _ => (run.x, run.x),
            };
            let carets = display::Carets {
                first: Caret {
                    text_byte: c.text_range.start,
                    x: Tick::from_tex_pt(x0),
                    top,
                    height: box_height,
                },
                last: (ci == last_index).then(|| Caret {
                    text_byte: c.text_range.end,
                    x: Tick::from_tex_pt(x1),
                    top,
                    height: box_height,
                }),
            };
            Cluster {
                text_start_byte: c.text_range.start,
                text_end_byte: c.text_range.end,
                hit_rect: Rect {
                    x: Tick::from_tex_pt(x0),
                    top,
                    width: Tick::from_tex_pt(x1 - x0),
                    height: box_height,
                },
                carets,
                provenance: Provenance::Source(source_of(c.span)),
            }
        })
        .collect();
    Some(display::Item::GlyphRun(GlyphRun {
        font_id: face.font_id.clone(),
        font_size: Tick::from_tex_pt(size),
        text: text.to_string(),
        glyphs,
        clusters: out_clusters,
        paint,
        role: display::RunRole::Text,
    }))
}

/// One stacked extensible delimiter in a flattened math box.
struct VerticalAssembly {
    /// The Latin Modern Math parts that paint it, bottom to top: the part's
    /// glyph id and the rise of its ink bottom above `bottom`.
    parts: Vec<(u16, f64)>,
    /// Top and bottom edges of the cmex boxes TeX stacked, in the flattened
    /// box's coordinates (y downward).
    top: f64,
    bottom: f64,
}

/// Finds the runs of cmex extensible pieces in `glyphs` (tex.web §713
/// stacks them for one delimiter, so they are consecutive, share the
/// character and the horizontal position, and their boxes tile) and builds
/// the Latin Modern Math assembly that paints each run as a whole.
///
/// Painting piece by piece cannot work: the font's parts are a different
/// design from cmex's, so the extender's own height (0.498 em for `(`,
/// 1.202 em for `|`) is not cmex's repeat (0.6 em), and the assembly is
/// held together by overlapping connectors rather than by abutting boxes.
/// Assembling once per run instead makes the painted ink span exactly the
/// TeX box, which is what pdfTeX's own cmex pieces do.
///
/// Returns the assembly for the run's first glyph and the flags of the
/// pieces it swallowed (nothing is painted for those).
fn vertical_assemblies(glyphs: &[ml::PositionedGlyph], m: &MathRec) -> (BTreeMap<usize, VerticalAssembly>, Vec<bool>) {
    let mut runs = BTreeMap::new();
    let mut swallowed = vec![false; glyphs.len()];
    let mut i = 0;
    while i < glyphs.len() {
        let g = &glyphs[i];
        if !m.extensible_piece(g) {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < glyphs.len() {
            let n = &glyphs[j];
            if n.ch != g.ch || n.font_id != g.font_id || (n.x - g.x).abs() > 1e-6 || n.size != g.size || !m.extensible_piece(n) {
                break;
            }
            j += 1;
        }
        // The stack runs top to bottom, so the run's box is the first
        // piece's top edge down to the last piece's bottom edge.
        let top = g.baseline_y - m.extension_box(g).map_or(0.0, |(h, _)| h);
        let last = &glyphs[j - 1];
        let bottom = last.baseline_y + m.extension_box(last).map_or(0.0, |(_, d)| d);
        if let Some(parts) = m.vertical_assembly(g.ch, bottom - top, g.size) {
            runs.insert(i, VerticalAssembly { parts, top, bottom });
            swallowed[i + 1..j].fill(true);
        }
        i = j;
    }
    (runs, swallowed)
}

fn math_items(
    run: &pl::PositionedRun,
    m: &MathRec,
    source_of: &dyn Fn(Span) -> SourceRange,
    items: &mut Vec<display::Item>,
    used: &mut BTreeMap<Rc<str>, Rc<LoadedFace>>,
) {
    let flat = ml::positioned_runs(&m.root, (run.x, -m.root.height - m.raise));
    let src = source_of(m.span);
    // With `math-glyph-spans` every glyph and rule maps to the atom that
    // produced it (math-layout's `SourceTag`); a leaf without one (never
    // expected: every converted atom is tagged) falls back to the formula.
    #[cfg(feature = "math-glyph-spans")]
    let leaf_src = |tag: ml::SourceTag| match tag.span {
        Some(s) => source_of(Span::in_document(DocumentId(s.document as usize), s.start, s.end)),
        None => src.clone(),
    };
    // Group consecutive glyphs of one face, size and paint into a run; each
    // glyph is a cluster.
    let mut current: Option<GlyphRun> = None;
    // A piece cut from the formula before it (`math_pieces`), on the same
    // line: its glyphs go on before the rules that piece appended (one
    // box's rules follow all its glyph runs) and continue its last run
    // when the face and size match, as one box would have grouped them.
    let mut tail: Vec<display::Item> = Vec::new();
    if m.continues {
        let own = Provenance::Source(src.clone());
        while matches!(items.last(), Some(display::Item::Rule(r)) if r.provenance == own) {
            tail.push(items.pop().expect("checked"));
        }
        tail.reverse();
        if matches!(items.last(), Some(display::Item::GlyphRun(r)) if r.role == display::RunRole::Math) {
            if let Some(display::Item::GlyphRun(r)) = items.pop() {
                current = Some(r);
            }
        }
    }
    let flush = |current: &mut Option<GlyphRun>, items: &mut Vec<display::Item>| {
        if let Some(r) = current.take() {
            if !r.glyphs.is_empty() {
                items.push(display::Item::GlyphRun(r));
            }
        }
    };
    let (assemblies, swallowed) = vertical_assemblies(&flat.glyphs, m);
    for (gi, g) in flat.glyphs.iter().enumerate() {
        // Painted by the run's first piece, as one assembly.
        if swallowed[gi] {
            continue;
        }
        let assembly = assemblies.get(&gi);
        let Some((face, gid)) = m.otf_glyph(g) else { continue };
        if gid == 0 {
            if g.ch == ' ' {
                // A `\text` interword space: glue, no glyph; the run is
                // split so the words stay separate items, as in paragraphs.
                flush(&mut current, items);
            }
            continue;
        }
        used.entry(face.font_id.clone()).or_insert_with(|| face.clone());
        let size_tick = Tick::from_tex_pt(g.size);
        #[cfg(feature = "math-glyph-spans")]
        let (paint, glyph_src) = (m.paint_of(g.tag), leaf_src(g.tag));
        // No per-glyph span_paints yet without the feature (#150/#158 fill
        // them as a follow-up): fall back to the formula's own colour.
        #[cfg(not(feature = "math-glyph-spans"))]
        let (paint, glyph_src) = (Paint::of(m.color), src.clone());
        if current.as_ref().is_some_and(|r| r.font_size != size_tick || r.font_id != face.font_id || r.paint != paint) {
            flush(&mut current, items);
        }
        let r = current.get_or_insert_with(|| GlyphRun {
            font_id: face.font_id.clone(),
            font_size: size_tick,
            text: String::new(),
            glyphs: Vec::new(),
            clusters: Vec::new(),
            paint,
            role: display::RunRole::Math,
        });
        // A stacked extensible delimiter: one cluster holding the whole
        // Latin Modern Math assembly. Each part draws from its own origin up
        // to its `fullAdvance`, so a part whose ink bottom rises `r` above
        // the bottom of the run sits on the baseline `bottom - r`. The parts
        // keep the TFM advance the pieces were laid out with (pdfTeX's
        // `/Widths`, which Latin Modern Math's parts match to 0.0001 pt).
        if let Some(a) = assembly {
            let start = r.text.len();
            r.text.push(g.ch);
            let ci = r.clusters.len() as u32;
            for (part, rise) in &a.parts {
                r.glyphs.push(Glyph {
                    gid: *part,
                    origin_x: Tick::from_tex_pt(g.x),
                    baseline_y: Tick::from_tex_pt(a.bottom - rise),
                    advance_x: Tick::from_tex_pt(g.width),
                    advance_y: Tick(0),
                    cluster: ci,
                });
            }
            let top = Tick::from_tex_pt(a.top);
            let hh = Tick::from_tex_pt((a.bottom - a.top).max(0.01));
            r.clusters.push(Cluster {
                text_start_byte: start,
                text_end_byte: r.text.len(),
                hit_rect: Rect {
                    x: Tick::from_tex_pt(g.x),
                    top,
                    width: Tick::from_tex_pt(g.width),
                    height: hh,
                },
                carets: display::Carets {
                    first: Caret {
                        text_byte: start,
                        x: Tick::from_tex_pt(g.x),
                        top,
                        height: hh,
                    },
                    last: None,
                },
                provenance: Provenance::Source(glyph_src),
            });
            continue;
        }
        let b = face.bounds(crate::ids::GlyphId(gid), Some(g.ch));
        // The advance TeX used: the laid-out glyph box's width (the TFM
        // width, pdfTeX's `/Widths`), not the painted OpenType glyph's own.
        #[cfg(feature = "amsmath-inline")]
        let adv = g.width;
        #[cfg(not(feature = "amsmath-inline"))]
        let adv = face.pt(i64::from(face.face().advance(crate::ids::GlyphId(gid)).unwrap_or(0)), g.size);
        let (h, d) = if b.empty {
            (0.0, 0.0)
        } else {
            (face.pt(i64::from(b.y_max), g.size), face.pt(-i64::from(b.y_min), g.size))
        };
        // A cmex glyph was laid out as its TFM box, which is the Type 1
        // outline hanging from the origin; the OpenType variant painted for
        // it is centred on the axis relative to its own origin, so its
        // baseline moves to put the drawn ink's centre on the TFM box's.
        let baseline_y = match m.extension_box(g) {
            Some((th, td)) if !b.empty => g.baseline_y + ((td - th) - (d - h)) / 2.0,
            _ => g.baseline_y,
        };
        // Where the painted outline starts. `\overbrace`/`\underbrace`: Latin
        // Modern Math's assembly parts stand for cmex's four pieces
        // (`TexMathMetrics::otf_gid`), the left end at the first piece, the
        // middle centred on the cusp between the two middle pieces, the right
        // end flush with the last piece, each advancing by its own width.
        // amsfonts' dashed-arrow head ("4B): the 1em arrow is right-aligned
        // in the msam box. Everything else starts at its TeX box.
        let face_adv = face.pt(i64::from(face.face().advance(crate::ids::GlyphId(gid)).unwrap_or(0)), g.size);
        let ams = crate::mathfont::ams_of(g.ch);
        let (paint_x, adv) = match (g.ch, g.gid) {
            ('\u{23DE}', 0x7D) | ('\u{23DF}', 0x7B) => (g.x + g.width - face_adv / 2.0, face_adv),
            ('\u{23DE}', 0x7B) | ('\u{23DF}', 0x7D) => (g.x + g.width - face_adv, face_adv),
            ('\u{23DE}' | '\u{23DF}', _) => (g.x, face_adv),
            _ if ams.is_some_and(|a| a.name == "dashrightarrow@") => (g.x + g.width - face_adv, adv),
            _ => (g.x, adv),
        };
        let start = r.text.len();
        match m.run_glyph(g) {
            // A `\text` cluster keeps its whole source text (`ffi`).
            Some(rg) => r.text.push_str(&rg.text),
            // `VARNOTHING_SENTINEL` (`\varnothing`) is never real text: it
            // stands for U+2205 everywhere outside the metrics/painting
            // lookup that needs to tell it apart from plain `\emptyset`.
            None if g.ch == crate::mathfont::VARNOTHING_SENTINEL => r.text.push('\u{2205}'),
            // An amssymb sentinel stands for its table text.
            None if ams.is_some() => r.text.push_str(ams.expect("checked").text),
            None => match crate::mathfont::MathFonts::extraction_text(g.ch) {
                Some(text) => r.text.push_str(text),
                None => r.text.push(g.ch),
            },
        }
        let ci = r.clusters.len() as u32;
        let top = Tick::from_tex_pt(baseline_y - h);
        let hh = Tick::from_tex_pt((h + d).max(0.01));
        r.glyphs.push(Glyph {
            gid,
            origin_x: Tick::from_tex_pt(paint_x),
            baseline_y: Tick::from_tex_pt(baseline_y),
            advance_x: Tick::from_tex_pt(adv),
            advance_y: Tick(0),
            cluster: ci,
        });
        // A negated amssymb relation Unicode spells as base + U+0338: the
        // painting face's combining long solidus, its ink centred on the
        // base's ink, in the same cluster.
        if let Some(slash) = ams
            .filter(|a| a.text.chars().nth(1) == Some('\u{0338}'))
            .and_then(|_| face.face().glyph_id('\u{0338}'))
        {
            let sb = face.bounds(slash, Some('\u{0338}'));
            if !b.empty && !sb.empty {
                let centre = |x0: i32, x1: i32| face.pt(i64::from(x0) + i64::from(x1), g.size) / 2.0;
                r.glyphs.push(Glyph {
                    gid: slash.0,
                    origin_x: Tick::from_tex_pt(paint_x + centre(b.x_min, b.x_max) - centre(sb.x_min, sb.x_max)),
                    baseline_y: Tick::from_tex_pt(baseline_y),
                    advance_x: Tick(0),
                    advance_y: Tick(0),
                    cluster: ci,
                });
            }
        }
        r.clusters.push(Cluster {
            text_start_byte: start,
            text_end_byte: r.text.len(),
            hit_rect: Rect {
                x: Tick::from_tex_pt(g.x),
                top,
                width: Tick::from_tex_pt(adv),
                height: hh,
            },
            carets: display::Carets {
                first: Caret {
                    text_byte: start,
                    x: Tick::from_tex_pt(g.x),
                    top,
                    height: hh,
                },
                last: None,
            },
            provenance: Provenance::Source(glyph_src),
        });
    }
    flush(&mut current, items);
    items.extend(tail);
    for rule in &flat.rules {
        if rule.w <= 0.0 || rule.h <= 0.0 {
            continue;
        }
        #[cfg(feature = "math-glyph-spans")]
        let (paint, rule_src) = (m.paint_of(rule.tag), leaf_src(rule.tag));
        // See the matching glyph-run fallback above.
        #[cfg(not(feature = "math-glyph-spans"))]
        let (paint, rule_src) = (Paint::of(m.color), src.clone());
        items.push(display::Item::Rule(Rule {
            x: Tick::from_tex_pt(rule.x),
            top: Tick::from_tex_pt(rule.y),
            width: Tick::from_tex_pt(rule.w).max(Tick(1)),
            height: Tick::from_tex_pt(rule.h).max(Tick(1)),
            paint,
            provenance: Provenance::Source(rule_src),
        }));
    }
}

impl Tick {
    fn max(self, other: Tick) -> Tick {
        if self.0 >= other.0 {
            self
        } else {
            other
        }
    }
}

/// Which documents a laid-out page references (for tests).
pub fn documents_referenced(list: &DisplayList) -> BTreeSet<DocumentId> {
    let mut out = BTreeSet::new();
    for p in &list.pages {
        for it in &p.items {
            if let display::Item::GlyphRun(r) = it {
                for c in &r.clusters {
                    for s in c.provenance.sources() {
                        if let Some(i) = list.documents.iter().position(|d| *d.path == *s.path) {
                            out.insert(DocumentId(i));
                        }
                    }
                }
            }
        }
    }
    out
}

#[cfg(all(test, feature = "math-glyph-spans"))]
mod math_paint_tests {
    use super::*;

    #[test]
    fn innermost_source_range_paints_a_math_leaf() {
        let red = Paint { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
        let blue = Paint { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
        // `\textcolor{red}{a {\color{blue} b} c}`: red over 10..40, blue 20..30.
        let ranges = vec![(10..40, red), (20..30, blue)];
        assert_eq!(innermost_paint(&ranges, 12, 13), Some(red));
        assert_eq!(innermost_paint(&ranges, 25, 26), Some(blue));
        assert_eq!(innermost_paint(&ranges, 35, 36), Some(red));
        // Partly outside every range, or before it: unpainted.
        assert_eq!(innermost_paint(&ranges, 5, 12), None);
        assert_eq!(innermost_paint(&ranges, 0, 1), None);
    }
}
