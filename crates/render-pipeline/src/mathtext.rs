//! `\text{...}` inside math: TeX's `\hbox` of *text-font* characters.
//!
//! amsmath's `\text` sets its argument in the current text font at the
//! size of the surrounding math style (12/8/6 pt in a 12 pt article), as an
//! unbreakable hbox at natural width. That is exactly what the paragraph
//! path already does for a word: T1 `ec-lm*` TFM ligature/kern program
//! (`f f i` → slot 0x1E, braces at T1 123/125), glyph ids from the face's
//! own `cmap`, and interword glue from `\fontdimen2` (`+\fontdimen7` at
//! space factor ≥ 2000, TeX §1041–1044). This module reuses that shaper and
//! glue so `\text` gets the same geometry the paragraphs have — never the
//! math roman family's OT1 slots, which is what the HW1 text-producer
//! evidence (main f261b36c) found: `a b` advanced by rm-lmr12 slot 32,
//! `\{x\}` by OT1 slots 123/125, `ffi` as three glyphs.
//!
//! math-layout has no nucleus for a pre-typeset box, so text, grids and boxed
//! math enter the layout as `Nucleus::Text(handle)` whose single placeholder
//! character reports the exact width/height/depth through
//! [`MathFontMetrics::text_glyph`]; after layout the placeholder glyph box
//! is replaced by the shaped or framed hbox (identical metrics ⇒ identical
//! Appendix G spacing and script placement). Handles are Supplementary
//! Private Use Area-A characters and never reach the display list.

use std::cell::RefCell;
use std::rc::Rc;

use flashtex_math_layout as ml;
use flashtex_math_layout::metrics::Extensible;
use flashtex_math_layout::{FontId as MathFontId, Glyph, MathFontMetrics, MathParams, SizeClass};

use crate::adapter::space_factor;
use crate::fonts::{Family, FontSet, LoadedFace, Role};
use crate::ids::GlyphId;
use crate::shape::Shaper;

/// First `FontId` value of a text-run *slot*. A placed glyph box addresses
/// a run glyph by `(font_id, gid)`: `font_id - RUN_FONT_BASE` is the slot,
/// `gid` (math-layout's `u16`) the index within that slot's chunk of
/// [`SLOT_GLYPHS`] shaped entries. A run longer than one chunk occupies
/// consecutive slots (`TextRun::first_slot .. first_slot + slots`), so no
/// index is ever truncated (issue #43). The CM/OpenType providers use small
/// font ids, far below this base.
pub const RUN_FONT_BASE: u32 = 0x4000_0000;

/// Shaped entries (ink glyphs and spaces) addressable through one slot.
pub const SLOT_GLYPHS: usize = 1 << 16;

/// Most slots one formula may use (keeps `RUN_FONT_BASE + slot` in `u32`).
const MAX_SLOTS: usize = 0x1000_0000;

/// Placeholder characters: Supplementary Private Use Area-A and -B,
/// U+F0000..=U+10FFFF, one per `\text` argument of a formula.
const HANDLE_BASE: u32 = 0xF_0000;
/// Number of handles (131072); a formula with more `\text` arguments is
/// refused with a diagnostic, never given an unrecognised or invalid handle.
pub const MAX_TEXT_ATOMS: usize = (0x10_FFFF - 0xF_0000 + 1) as usize;

fn handle_char(index: usize) -> Option<char> {
    if index >= MAX_TEXT_ATOMS {
        return None;
    }
    char::from_u32(HANDLE_BASE + index as u32)
}

/// Whether `ch` is a run placeholder (never drawn, never extracted).
pub fn is_handle(ch: char) -> bool {
    handle_index(ch).is_some()
}

fn handle_index(ch: char) -> Option<usize> {
    let c = ch as u32;
    (c >= HANDLE_BASE).then(|| (c - HANDLE_BASE) as usize)
}

/// Collects the `\text` arguments of one formula while the compiler list is
/// converted; each becomes an ordinary atom carrying a handle.
#[derive(Default, Debug)]
pub struct TextSink {
    pub texts: Vec<String>,
    /// Per text: `None` for `\text` (the document's text font), or the
    /// NFSS shape of a math alphabet run (`\mathbf`, `\mathsf`, ...; see
    /// `crate::mathalpha`).
    pub keys: Vec<Option<crate::nfss::FontKey>>,
    /// Arguments beyond [`MAX_TEXT_ATOMS`], in order: refused before any
    /// state changed, reported by the caller as `math_text_overflow`.
    pub refused: Vec<String>,
    /// The text font's quad in pt and its ratio to the math symbol font's
    /// quad at the text size, for `\quad` glue in math (`None`: unknown,
    /// the glue is measured in math quads).
    pub text_quad: Option<(f64, f64)>,
    /// Grid environments set as boxes inside the formula (see
    /// [`TextSink::grid_atom`]); each reserves a handle (an empty entry of
    /// `texts`).
    pub grids: Vec<GridCells>,
    /// `\boxed` bodies set as framed boxes inside the formula.
    pub(crate) frames: Vec<FrameBoxSpec>,
    /// The document's body font size in pt (`\f@size`), for size-dependent
    /// kerns such as amsmath's `\ex@`; 0 when unknown.
    pub body_size_pt: f64,
    /// Whether `amsfonts` (or `amssymb`, which loads it) is loaded: its
    /// `\widehat`/`\widetilde` switch to msbm's extra-wide accents past 2em.
    pub amsfonts: bool,
    /// Whether `amsmath` is loaded. **Not** implied by [`Self::amsfonts`]:
    /// `amssymb`/`amsfonts` bring the msam/msbm symbol fonts and nothing
    /// else, so a document may load either package without the other.
    ///
    /// `\big`..`\Bigg` are sized by whichever definition is in force:
    /// amsmath's `\bBigg@` (amsmath.sty 721-738), a `\vcenter` scaled by
    /// the body size, or -- without amsmath -- the kernel's own absolute
    /// `\vbox` lengths (fontmath.ltx 513-520), which do not move with the
    /// body size at all.
    pub amsmath: bool,
}

/// An `array`/`cases`/matrix/`aligned` grid met inside a sub-formula (a
/// fraction, a script, a radicand, a `\left...\right` body, another grid's
/// cell, or a top-level grid carrying scripts), its cells already converted.
/// TeX sets it as a `\vcenter`/`\vtop`/`\vbox` in the math list — an Ord
/// atom, or, for the fenced environments, the Inner atom of the
/// `\left...\right` around it — so it enters math-layout as a handle whose
/// metrics are the laid-out box at the size the layout asks for.
#[derive(Debug, Clone)]
pub struct GridCells {
    /// Index of the handle character (as for `\text`).
    pub handle: usize,
    pub cells: Vec<Vec<ml::MathList>>,
    pub columns: String,
    pub left: String,
    pub right: String,
    /// The environment's `\begin`, where its spec is read.
    pub span: flashtex_compiler::Span,
}

/// A `\boxed` body converted to a math-layout list. It is always laid out in
/// display style, as amsmath defines `\boxed{#1}` through `\fbox{...$\displaystyle#1$}`.
#[derive(Debug, Clone)]
pub(crate) struct FrameBoxSpec {
    /// Index of the handle character (as for [`GridCells`]).
    handle: usize,
    body: ml::MathList,
    tag: ml::SourceTag,
}

/// A [`GridCells`] with its environment spec resolved from the source.
#[derive(Debug, Clone)]
pub struct NestedGrid {
    pub grid: GridCells,
    pub spec: crate::mathgrid::GridSpec,
    pub pitch: crate::mathgrid::Pitch,
}

/// A nested grid laid out at one size: substituted for the placeholder
/// glyph (`ch` at `size`) after layout.
#[derive(Debug, Clone)]
pub struct GridBox {
    pub ch: char,
    pub size: f64,
    pub hbox: ml::MathBox,
}

/// A framed body laid out at one parent size: substituted for its placeholder
/// after the parent formula has been laid out.
#[derive(Debug, Clone)]
pub(crate) struct FrameBox {
    ch: char,
    size: f64,
    hbox: ml::MathBox,
}

/// `font_id` of a nested grid's placeholder glyph (never drawn: every one
/// is replaced by [`substitute_grids`]).
pub const GRID_FONT_ID: u32 = RUN_FONT_BASE - 1;

/// `font_id` of a `\boxed` placeholder glyph (never emitted).
const FRAME_FONT_ID: u32 = RUN_FONT_BASE - 2;

impl TextSink {
    /// Text-font quad / math quad, 1 when unknown.
    pub fn font_em_ratio(&self) -> f64 {
        self.text_quad.map_or(1.0, |(_, r)| r)
    }

    /// An atom of `class` standing for a grid (see [`GridCells`]); an empty
    /// atom of that class once the handle space is exhausted.
    pub fn grid_atom(&mut self, class: ml::AtomClass, cells: Vec<Vec<ml::MathList>>, columns: &str, left: &str, right: &str, span: flashtex_compiler::Span) -> ml::Atom {
        let index = self.texts.len();
        match handle_char(index) {
            Some(handle) => {
                self.grids.push(GridCells {
                    handle: index,
                    cells,
                    columns: columns.to_string(),
                    left: left.to_string(),
                    right: right.to_string(),
                    span,
                });
                self.texts.push(String::new());
                self.keys.push(None);
                ml::Atom::new(class, ml::Nucleus::Text(handle.to_string()))
            }
            None => {
                self.refused.push("\\begin{...} grid".to_string());
                ml::Atom::new(class, ml::Nucleus::Empty)
            }
        }
    }

    /// An `Ord` atom for a `\boxed` body; the frame is built after its body is
    /// laid out in display style through the existing placeholder seam.
    pub(crate) fn frame_atom(&mut self, body: ml::MathList, tag: ml::SourceTag) -> ml::Atom {
        let index = self.texts.len();
        match handle_char(index) {
            Some(handle) => {
                self.frames.push(FrameBoxSpec { handle: index, body, tag });
                self.texts.push(String::new());
                self.keys.push(None);
                ml::Atom::new(ml::AtomClass::Ord, ml::Nucleus::Text(handle.to_string()))
            }
            None => {
                self.refused.push("\\boxed{...}".to_string());
                ml::Atom::new(ml::AtomClass::Ord, ml::Nucleus::Empty)
            }
        }
    }

    /// An `Ord` atom for `text` (TeX §1076: an hbox in math is an Ord); an
    /// empty Ord (scripts still attach) once the handle space is exhausted.
    pub fn atom(&mut self, text: &str) -> ml::Atom {
        self.atom_keyed(text, None)
    }

    /// An `Ord` atom for a run of math-alphabet characters set in the text
    /// font `key` (TeX §752: consecutive characters of one text font are
    /// kerned and ligatured, with the last one's italic correction).
    pub fn atom_in(&mut self, text: &str, key: crate::nfss::FontKey) -> ml::Atom {
        self.atom_keyed(text, Some(key))
    }

    fn atom_keyed(&mut self, text: &str, key: Option<crate::nfss::FontKey>) -> ml::Atom {
        match handle_char(self.texts.len()) {
            Some(handle) => {
                self.texts.push(text.to_string());
                self.keys.push(key);
                ml::Atom::new(ml::AtomClass::Ord, ml::Nucleus::Text(handle.to_string()))
            }
            None => {
                self.refused.push(text.to_string());
                ml::Atom::new(ml::AtomClass::Ord, ml::Nucleus::Empty)
            }
        }
    }
}

/// One glyph of a shaped run, addressed by its index (`gid` of the
/// placeholder-free glyph boxes in [`TextRun::hbox`]).
#[derive(Debug, Clone, PartialEq)]
pub struct RunGlyph {
    /// Original glyph id in [`TextRun::face`]; 0 for the interword space
    /// (no ink; the display list splits the run there).
    pub gid: GlyphId,
    /// First character of the cluster (what the placed glyph carries).
    pub ch: char,
    /// The cluster's source text: `"ffi"` for the ligature glyph, so text
    /// extraction keeps every character.
    pub text: String,
}

/// A `\text` argument shaped at one size.
#[derive(Clone)]
pub struct TextRun {
    pub text: String,
    pub size: f64,
    pub face: Rc<LoadedFace>,
    /// First slot (`font_id - RUN_FONT_BASE`) of this run; it uses
    /// `glyphs.len().div_ceil(SLOT_GLYPHS).max(1)` consecutive slots.
    pub first_slot: usize,
    pub glyphs: Vec<RunGlyph>,
    /// The hbox: glyph boxes (`font_id` = this run, `gid` = index into
    /// `glyphs`, widths = TFM advances with kerns folded in) and the space
    /// glue at natural width, on the baseline.
    pub hbox: ml::MathBox,
    /// True when the TFM produced the advances (TeX's geometry).
    pub tfm_metrics: bool,
    /// The math-alphabet shape of the run, `None` for `\text`.
    pub key: Option<crate::nfss::FontKey>,
    /// The italic correction of the run's last character (pt) for a math
    /// alphabet run, which math-layout applies as the nucleus' δ; 0 for
    /// `\text` (an hbox has none).
    pub italic: f64,
}

impl TextRun {
    /// Slots this run occupies.
    pub fn slots(&self) -> usize {
        self.glyphs.len().div_ceil(SLOT_GLYPHS).max(1)
    }

    /// The run glyph a placed `(font_id, gid)` addresses, if it is in this run.
    pub fn glyph_at(&self, font_id: MathFontId, gid: u16) -> Option<&RunGlyph> {
        let slot = font_id.0.checked_sub(RUN_FONT_BASE)? as usize;
        let chunk = slot.checked_sub(self.first_slot)?;
        if chunk >= self.slots() {
            return None;
        }
        self.glyphs.get(chunk * SLOT_GLYPHS + usize::from(gid))
    }

    /// Whether `font_id` is one of this run's slots.
    pub fn owns(&self, font_id: MathFontId) -> bool {
        font_id.0.checked_sub(RUN_FONT_BASE).is_some_and(|s| {
            let s = s as usize;
            s >= self.first_slot && s < self.first_slot + self.slots()
        })
    }
}

/// The run among `runs` that a placed glyph's `font_id` belongs to.
pub fn run_of(runs: &[TextRun], font_id: MathFontId) -> Option<&TextRun> {
    runs.iter().find(|r| r.owns(font_id))
}

/// What happened while shaping, reported by the caller with the formula's
/// source range once layout is done.
#[derive(Debug, Clone, PartialEq)]
pub enum Notice {
    /// The face at this size was resolved (the caller replays the font
    /// availability/TFM diagnostics through its usual `face` path).
    FaceUsed { size: f64 },
    Refused { word: String, reason: String },
    MissingGlyph { ch: char, face: String },
    TfmRunError { word: String, face: String, error: String },
    /// The run would need more slots than a formula may address; nothing
    /// was shaped for it (the placeholder stays an empty box).
    TooLarge { text_chars: usize, glyphs: usize },
}

/// Lazily shapes the registered texts at the sizes the layout asks for and
/// answers the math-layout metrics interface for them, delegating everything
/// else to the real provider.
pub struct TextRunMetrics<'a> {
    inner: &'a dyn MathFontMetrics,
    fonts: &'a FontSet,
    shaper: &'a Shaper,
    family: Family,
    texts: &'a [String],
    keys: &'a [Option<crate::nfss::FontKey>],
    runs: RefCell<Vec<TextRun>>,
    notices: RefCell<Vec<Notice>>,
    grids: &'a [NestedGrid],
    grid_boxes: RefCell<Vec<GridBox>>,
    grid_limitations: RefCell<Vec<ml::Limitation>>,
    frames: &'a [FrameBoxSpec],
    frame_boxes: RefCell<Vec<FrameBox>>,
    frame_limitations: RefCell<Vec<ml::Limitation>>,
}

impl<'a> TextRunMetrics<'a> {
    /// `keys` parallels `texts` ([`TextSink::keys`]); a missing entry is a
    /// `\text` run.
    pub fn new(
        inner: &'a dyn MathFontMetrics,
        fonts: &'a FontSet,
        shaper: &'a Shaper,
        family: Family,
        texts: &'a [String],
        keys: &'a [Option<crate::nfss::FontKey>],
    ) -> TextRunMetrics<'a> {
        TextRunMetrics {
            inner,
            fonts,
            shaper,
            family,
            texts,
            keys,
            runs: RefCell::new(Vec::new()),
            notices: RefCell::new(Vec::new()),
            grids: &[],
            grid_boxes: RefCell::new(Vec::new()),
            grid_limitations: RefCell::new(Vec::new()),
            frames: &[],
            frame_boxes: RefCell::new(Vec::new()),
            frame_limitations: RefCell::new(Vec::new()),
        }
    }

    /// Answers the handles of `grids` with their laid-out boxes.
    pub fn with_grids(mut self, grids: &'a [NestedGrid]) -> TextRunMetrics<'a> {
        self.grids = grids;
        self
    }

    pub(crate) fn with_frames(mut self, frames: &'a [FrameBoxSpec]) -> TextRunMetrics<'a> {
        self.frames = frames;
        self
    }

    /// The nested grid boxes laid out so far (for [`substitute_grids`]) and
    /// the limitations met inside their cells and fences.
    pub fn take_grids(&self) -> (Vec<GridBox>, Vec<ml::Limitation>) {
        (self.grid_boxes.take(), self.grid_limitations.take())
    }

    /// The framed boxes laid out so far and limitations met inside their
    /// display-style bodies.
    pub(crate) fn take_frames(&self) -> (Vec<FrameBox>, Vec<ml::Limitation>) {
        (self.frame_boxes.take(), self.frame_limitations.take())
    }

    /// Lays out `grid` at `size` (cached per handle and size): each cell a
    /// formula in the environment's own style — the `$##$` of `\halign`
    /// starts a new list, so the cells do not shrink in a script (only
    /// `smallmatrix` is `\scriptstyle`) — placed by `mathgrid` on the axis of
    /// `size`, and for the fenced environments `\left`/`\right` delimiters
    /// of `size` (Rule 19). Returns width, height and depth.
    fn grid_box(&self, grid: &NestedGrid, ch: char, size: SizeClass) -> (f64, f64, f64) {
        use crate::mathgrid as mg;
        let p = self.inner.params(size);
        if let Some(b) = self.grid_boxes.borrow().iter().find(|b| b.ch == ch && b.size == p.size) {
            return (b.hbox.width, b.hbox.height, b.hbox.depth);
        }
        let quad = self.inner.params(SizeClass::Text).quad;
        let mut limitations = Vec::new();
        let cells: Vec<Vec<ml::MathBox>> = grid
            .grid
            .cells
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(ci, cell)| {
                        // amsmath `aligned`: a right-hand cell is `{}##`.
                        let laid = if grid.spec.gaps == mg::Gaps::Pairs && ci % 2 == 1 {
                            let mut prefixed = cell.clone();
                            prefixed.atoms.insert(0, ml::Atom::new(ml::AtomClass::Ord, ml::Nucleus::Empty));
                            ml::layout_with_report(&prefixed, grid.spec.style, self)
                        } else {
                            ml::layout_with_report(cell, grid.spec.style, self)
                        };
                        limitations.extend(laid.limitations);
                        laid.root
                    })
                    .collect()
            })
            .collect();
        let body = mg::layout_grid(cells, &grid.grid.columns, &grid.spec, grid.pitch, &p, quad);
        let hbox = if grid.grid.left.is_empty() && grid.grid.right.is_empty() {
            body
        } else {
            let style = match size {
                SizeClass::Text => ml::Style::TEXT,
                SizeClass::Script => ml::Style::SCRIPT,
                SizeClass::ScriptScript => ml::Style::SCRIPT_SCRIPT,
            };
            let one = |s: &str| {
                let mut it = s.chars();
                match (it.next(), it.next()) {
                    (Some(c), None) => Some(c),
                    _ => None,
                }
            };
            let (h, d) = (body.height, body.depth);
            let mut fence = |s: &str| {
                let ch = one(s);
                let (b, short) = mg::delimiter(self, ch, h, d, style, &p);
                if let (Some(ch), Some((wanted, used))) = (ch, short) {
                    limitations.push(ml::Limitation::DelimiterTooSmall { ch, wanted, used });
                }
                b
            };
            let open = fence(&grid.grid.left);
            let close = fence(&grid.grid.right);
            ml::MathBox::hlist(vec![open, body, close])
        };
        let dims = (hbox.width, hbox.height, hbox.depth);
        self.grid_limitations.borrow_mut().extend(limitations);
        self.grid_boxes.borrow_mut().push(GridBox { ch, size: p.size, hbox });
        dims
    }

    /// Lays out a `\boxed` body in display style and wraps it in the standard
    /// `\fbox` frame. The result is cached per placeholder and parent size.
    fn frame_box(&self, frame: &FrameBoxSpec, ch: char, size: SizeClass) -> (f64, f64, f64) {
        let p = self.inner.params(size);
        if let Some(b) = self.frame_boxes.borrow().iter().find(|b| b.ch == ch && b.size == p.size) {
            return (b.hbox.width, b.hbox.height, b.hbox.depth);
        }
        let laid = ml::layout_with_report(&frame.body, ml::Style::DISPLAY, self);
        self.frame_limitations.borrow_mut().extend(laid.limitations);
        let hbox = framed_math_box(laid.root, frame.tag);
        let dims = (hbox.width, hbox.height, hbox.depth);
        self.frame_boxes.borrow_mut().push(FrameBox { ch, size: p.size, hbox });
        dims
    }

    /// The runs shaped so far and the notices, in order.
    pub fn finish(self) -> (Vec<TextRun>, Vec<Notice>) {
        (self.runs.into_inner(), self.notices.into_inner())
    }

    fn run_for(&self, text_index: usize, size: f64) -> Option<usize> {
        let text = self.texts.get(text_index)?;
        let key = self.keys.get(text_index).copied().flatten();
        if let Some(i) = self.runs.borrow().iter().position(|r| r.text == *text && r.size == size && r.key == key) {
            return Some(i);
        }
        let (index, first_slot) = {
            let runs = self.runs.borrow();
            (runs.len(), runs.last().map_or(0, |r| r.first_slot + r.slots()))
        };
        let run = shape_run(self.fonts, self.shaper, self.family, key, text, size, first_slot, &mut self.notices.borrow_mut())?;
        self.runs.borrow_mut().push(run);
        Some(index)
    }
}

impl MathFontMetrics for TextRunMetrics<'_> {
    fn params(&self, size: SizeClass) -> MathParams {
        self.inner.params(size)
    }

    fn font_name(&self, font: MathFontId) -> String {
        match font.0.checked_sub(RUN_FONT_BASE) {
            Some(slot) => run_of(&self.runs.borrow(), font).map_or_else(|| format!("text-run-slot-{slot}"), |r| r.face.name.clone()),
            None => self.inner.font_name(font),
        }
    }

    fn glyph(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        self.inner.glyph(ch, size)
    }

    fn large_operator(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        self.inner.large_operator(ch, size)
    }

    fn delimiter_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        self.inner.delimiter_sizes(ch, size)
    }

    fn radical_sizes(&self, size: SizeClass) -> Vec<Glyph> {
        self.inner.radical_sizes(size)
    }

    fn accent_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        self.inner.accent_sizes(ch, size)
    }

    fn delimiter_extensible(&self, ch: char, size: SizeClass) -> Option<Extensible> {
        self.inner.delimiter_extensible(ch, size)
    }

    fn radical_extensible(&self, size: SizeClass) -> Option<Extensible> {
        self.inner.radical_extensible(size)
    }

    fn extension_glyph(&self, code: u8, ch: char, size: SizeClass) -> Option<Glyph> {
        self.inner.extension_glyph(code, ch, size)
    }

    fn text_glyph(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        let Some(text_index) = handle_index(ch) else {
            return self.inner.text_glyph(ch, size);
        };
        let at = self.inner.params(size).size;
        if let Some(grid) = self.grids.iter().find(|g| g.grid.handle == text_index) {
            let (width, height, depth) = self.grid_box(grid, ch, size);
            return Some(Glyph {
                font_id: MathFontId(GRID_FONT_ID),
                gid: 0,
                ch,
                size: at,
                width,
                height,
                depth,
                italic: 0.0,
                skew: 0.0,
            });
        }
        if let Some(frame) = self.frames.iter().find(|f| f.handle == text_index) {
            let (width, height, depth) = self.frame_box(frame, ch, size);
            return Some(Glyph {
                font_id: MathFontId(FRAME_FONT_ID),
                gid: 0,
                ch,
                size: at,
                width,
                height,
                depth,
                italic: 0.0,
                skew: 0.0,
            });
        }
        let i = self.run_for(text_index, at)?;
        let runs = self.runs.borrow();
        let run = &runs[i];
        Some(Glyph {
            font_id: MathFontId(RUN_FONT_BASE + run.first_slot as u32),
            gid: 0,
            ch,
            size: at,
            width: run.hbox.width,
            height: run.hbox.height,
            depth: run.hbox.depth,
            italic: run.italic,
            skew: 0.0,
        })
    }
}

/// A short, single-line rendering of a refused argument for diagnostics.
pub fn abbreviate(text: &str) -> String {
    let mut s: String = text.chars().take(32).collect();
    if text.chars().count() > 32 {
        s.push('…');
    }
    s
}

/// `\fbox` geometry used by amsmath's `\boxed`: 3pt separation and a 0.4pt
/// rule on every side. Side rules overlap the horizontal rules by half their
/// thickness, matching the existing color-box display-list geometry.
fn framed_math_box(body: ml::MathBox, tag: ml::SourceTag) -> ml::MathBox {
    const SEP: f64 = 3.0;
    const RULE: f64 = 0.4;
    let inset = SEP + RULE;
    let width = body.width + 2.0 * inset;
    let height = body.height + inset;
    let depth = body.depth + inset;
    let side_height = height + depth - RULE;
    let side_dy = depth - RULE / 2.0;
    let rule = |width, height| ml::MathBox::rule(width, height, 0.0).with_tag(tag);
    ml::MathBox {
        kind: ml::BoxKind::HBox(vec![
            ml::Child {
                dx: inset,
                dy: 0.0,
                content: body,
            },
            ml::Child {
                dx: 0.0,
                dy: -height + RULE,
                content: rule(width, RULE),
            },
            ml::Child {
                dx: 0.0,
                dy: side_dy,
                content: rule(RULE, side_height),
            },
            ml::Child {
                dx: width - RULE,
                dy: side_dy,
                content: rule(RULE, side_height),
            },
            ml::Child {
                dx: 0.0,
                dy: depth,
                content: rule(width, RULE),
            },
        ]),
        width,
        height,
        depth,
        tag: ml::SourceTag::NONE,
    }
}

/// Replaces nested grid and framed-box placeholders in `root` by their boxes,
/// including handles nested in a substituted box. Run [`substitute`]
/// afterwards for the `\text` runs inside them.
pub(crate) fn substitute_math_boxes(root: &mut ml::MathBox, grids: &[GridBox], frames: &[FrameBox]) {
    let found = match &root.kind {
        ml::BoxKind::Glyph { ch, size, .. } => grids
            .iter()
            .find(|b| b.ch == *ch && b.size == *size)
            .map(|b| &b.hbox)
            .or_else(|| frames.iter().find(|b| b.ch == *ch && b.size == *size).map(|b| &b.hbox)),
        _ => None,
    };
    if let Some(hbox) = found {
        #[cfg(feature = "math-glyph-spans")]
        let tag = root.tag;
        *root = hbox.clone();
        #[cfg(feature = "math-glyph-spans")]
        root.inherit_tag(tag);
    }
    if let ml::BoxKind::HBox(children) | ml::BoxKind::VBox(children) = &mut root.kind {
        for c in children {
            substitute_math_boxes(&mut c.content, grids, frames);
        }
    }
}

pub fn substitute_grids(root: &mut ml::MathBox, grids: &[GridBox]) {
    substitute_math_boxes(root, grids, &[]);
}

/// Replaces every placeholder glyph box in `root` by its shaped hbox.
pub fn substitute(root: &mut ml::MathBox, runs: &[TextRun]) {
    match &mut root.kind {
        ml::BoxKind::Glyph { font_id, ch, size, .. } => {
            if handle_index(*ch).is_some() {
                if let Some(run) = run_of(runs, *font_id) {
                    debug_assert_eq!(run.size, *size);
                    // The run's glyphs map to the `\text` command's span.
                    #[cfg(feature = "math-glyph-spans")]
                    let tag = root.tag;
                    *root = run.hbox.clone();
                    #[cfg(feature = "math-glyph-spans")]
                    root.inherit_tag(tag);
                }
            }
        }
        ml::BoxKind::HBox(children) | ml::BoxKind::VBox(children) => {
            for c in children {
                substitute(&mut c.content, runs);
            }
        }
        ml::BoxKind::Rule | ml::BoxKind::Kern | ml::BoxKind::Glue { .. } => {}
    }
}

/// `\fontdimen2` and `\fontdimen7` of `face` at `size`, in points.
fn space_dimens(fonts: &FontSet, family: Family, face: &LoadedFace, size: f64) -> (f64, f64) {
    if let Some(tfm) = &face.tfm {
        let dim = |n: usize| tfm.param(n).map_or(0.0, |v| crate::tfm::Tfm::pt(v, size));
        return (dim(2), dim(7));
    }
    let _ = fonts;
    let design = crate::typeset::design_size(family, size);
    let p = crate::params::text_params(family, false, false, design).at(size);
    (p.space, p.extra_space)
}

/// Shapes `text` as an hbox at `size`: words through the face's shaper
/// (TFM ligatures/kerns), one glue per space at natural width with TeX's
/// space factor (1000 at the start of the box, §1034 per character).
/// `None` when the run cannot be addressed (more than `MAX_SLOTS` chunks of
/// entries from `first_slot`): a `TooLarge` notice is recorded instead.
#[allow(clippy::too_many_arguments)]
fn shape_run(
    fonts: &FontSet,
    shaper: &Shaper,
    family: Family,
    key: Option<crate::nfss::FontKey>,
    text: &str,
    size: f64,
    first_slot: usize,
    notices: &mut Vec<Notice>,
) -> Option<TextRun> {
    // Every shaped entry (space or glyph) gets its own index; chunk `k` of
    // `SLOT_GLYPHS` entries is addressed through slot `first_slot + k`.
    let address = |entry: usize| -> (MathFontId, u16) {
        let slot = first_slot + entry / SLOT_GLYPHS;
        (MathFontId(RUN_FONT_BASE + slot as u32), (entry % SLOT_GLYPHS) as u16)
    };
    let face = match key {
        None => fonts.resolve(family, Role::Text { bold: false, italic: false }, size).face,
        // A math alphabet is an OT1 cmr/cmss/cmtt shape in pdfLaTeX
        // (fontmath.ltx), whatever the text encoding: Latin Modern's metrics
        // of the same design (`ec-lm*`, whose letters and digits equal
        // `rm-lm*`'s) at the `.fd` optical size.
        Some(k) => fonts.resolve(Family::LatinModern, Role::Font(k), size).face,
    };
    notices.push(Notice::FaceUsed { size });
    let mut last_italic = 0.0;
    let (space, extra) = space_dimens(fonts, family, &face, size);
    let mut glyphs: Vec<RunGlyph> = Vec::new();
    let mut boxes: Vec<ml::MathBox> = Vec::new();
    let mut factor = 1000u32;
    let mut tfm_metrics = true;
    let mut first = true;
    // Capacity check before any state is built: the largest possible entry
    // count is one per character plus one per space, which bounds the slots.
    let max_entries = text.chars().count();
    if first_slot + max_entries.div_ceil(SLOT_GLYPHS).max(1) > MAX_SLOTS {
        notices.push(Notice::TooLarge { text_chars: max_entries, glyphs: max_entries });
        return None;
    }
    for word in text.split(' ') {
        if !first {
            // Interword glue at natural width; an hbox is never stretched.
            let mut width = space;
            if factor >= 2000 {
                width += extra;
            }
            let (font_id, gid) = address(glyphs.len());
            glyphs.push(RunGlyph { gid: GlyphId(0), ch: ' ', text: " ".to_string() });
            boxes.push(ml::MathBox {
                kind: ml::BoxKind::Glyph { font_id, gid, ch: ' ', size },
                width,
                height: 0.0,
                depth: 0.0,
                ..ml::MathBox::empty()
            });
        }
        first = false;
        if word.is_empty() {
            continue;
        }
        let shaped = shaper.shape(&face, word);
        tfm_metrics &= shaped.tfm_metrics;
        if let Some(e) = &shaped.tfm_error {
            notices.push(Notice::TfmRunError { word: word.to_string(), face: face.name.clone(), error: e.clone() });
        }
        if let Some(reason) = &shaped.refused {
            notices.push(Notice::Refused { word: word.to_string(), reason: reason.clone() });
            continue;
        }
        for (ch, _) in &shaped.missing {
            notices.push(Notice::MissingGlyph { ch: *ch, face: face.name.clone() });
        }
        let height = shaped.height_pt(size);
        let depth = shaped.depth_pt(size);
        for c in &shaped.clusters {
            let ch = c.text.chars().next().unwrap_or('\u{FFFD}');
            for (k, g) in c.glyphs.iter().enumerate() {
                let (font_id, gid) = address(glyphs.len());
                // A cluster with several glyphs attributes its text to the first.
                let text = if k == 0 { c.text.clone() } else { String::new() };
                glyphs.push(RunGlyph { gid: g.gid, ch, text });
                let width = g.advance as f64 * size / shaped.units_per_em as f64;
                if !g.empty {
                    last_italic = if shaped.tfm_metrics { crate::tfm::Tfm::pt(g.italic, size) } else { 0.0 };
                }
                boxes.push(ml::MathBox {
                    kind: ml::BoxKind::Glyph { font_id, gid, ch, size },
                    width,
                    height: if g.empty { 0.0 } else { height },
                    depth: if g.empty { 0.0 } else { depth },
                    ..ml::MathBox::empty()
                });
            }
        }
        for ch in word.chars() {
            factor = space_factor(ch, factor);
        }
    }
    // Shaping never yields more entries than characters (ligatures merge,
    // font-engine clusters map one char to at most one glyph per char).
    debug_assert!(glyphs.len() <= max_entries.max(1));
    let hbox = ml::MathBox::hlist(boxes);
    let italic = if key.is_some() { last_italic } else { 0.0 };
    Some(TextRun { text: text.to_string(), size, face, first_slot, glyphs, hbox, tfm_metrics, key, italic })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_round_trip_and_stay_private_use() {
        for i in [0usize, 1, 255, 0xFFFF, 0x1_0000, 0x1_0001, MAX_TEXT_ATOMS - 1] {
            let h = handle_char(i).unwrap();
            assert_eq!(handle_index(h), Some(i), "{i}");
            assert!(!h.is_ascii());
            assert!(h as u32 >= 0xF_0000);
        }
        assert_eq!(handle_char(MAX_TEXT_ATOMS), None, "beyond U+10FFFF: refused, never a panic");
        assert_eq!(handle_char(usize::MAX), None);
        assert_eq!(handle_index('a'), None);
        assert_eq!(handle_index(' '), None);
        assert_eq!(handle_index('\u{E000}'), None, "BMP private use is not a handle");
        assert_eq!(handle_index('\u{FFFF}'), None);
    }

    #[test]
    fn sink_gives_ordinary_atoms_in_order_and_refuses_beyond_the_handle_space() {
        let mut sink = TextSink::default();
        let a = sink.atom("(a)");
        let b = sink.atom("and");
        assert_eq!(sink.texts, vec!["(a)", "and"]);
        assert_eq!(a.class, ml::AtomClass::Ord);
        assert_eq!(a.nucleus, ml::Nucleus::Text(handle_char(0).unwrap().to_string()));
        assert_eq!(b.nucleus, ml::Nucleus::Text(handle_char(1).unwrap().to_string()));
        for i in 2..MAX_TEXT_ATOMS {
            let _ = sink.atom(&i.to_string());
        }
        assert_eq!(sink.texts.len(), MAX_TEXT_ATOMS);
        assert!(sink.refused.is_empty());
        let over = sink.atom("one too many");
        assert_eq!(over.nucleus, ml::Nucleus::Empty);
        assert_eq!(sink.refused, vec!["one too many"]);
        assert_eq!(sink.texts.len(), MAX_TEXT_ATOMS, "no state changed for the refused atom");
    }

    #[test]
    fn slot_addressing_does_not_alias_across_chunks() {
        // A run whose glyph table spans three chunks: the boundary entries
        // 65535, 65536 and 65537 resolve to themselves, not to 65535 % 65536.
        let face = crate::fonts::FontSet::with_default_dirs(&[])
            .resolve(Family::Times, Role::Text { bold: false, italic: false }, 10.0)
            .face;
        let glyphs: Vec<RunGlyph> = (0..(2 * SLOT_GLYPHS + 2)).map(|i| RunGlyph { gid: GlyphId(i as u16), ch: 'x', text: i.to_string() }).collect();
        let run = TextRun { text: String::new(), size: 10.0, face, first_slot: 3, glyphs, hbox: ml::MathBox::empty(), tfm_metrics: false, key: None, italic: 0.0 };
        assert_eq!(run.slots(), 3);
        let at = |entry: usize| {
            let slot = 3 + entry / SLOT_GLYPHS;
            run.glyph_at(MathFontId(RUN_FONT_BASE + slot as u32), (entry % SLOT_GLYPHS) as u16).map(|g| g.text.clone())
        };
        for e in [0usize, 1, SLOT_GLYPHS - 1, SLOT_GLYPHS, SLOT_GLYPHS + 1, 2 * SLOT_GLYPHS + 1] {
            assert_eq!(at(e).as_deref(), Some(e.to_string().as_str()), "entry {e}");
        }
        assert_eq!(at(2 * SLOT_GLYPHS + 2), None, "past the end");
        assert!(run.glyph_at(MathFontId(RUN_FONT_BASE + 2), 0).is_none(), "slot before the run");
        assert!(run.glyph_at(MathFontId(RUN_FONT_BASE + 6), 0).is_none(), "slot after the run");
        assert!(run.owns(MathFontId(RUN_FONT_BASE + 5)) && !run.owns(MathFontId(RUN_FONT_BASE + 6)));
    }
}
