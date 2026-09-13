//! LaTeX float placement over the TeX page builder (FT-063).
//!
//! A float is set in its own box (`\@xfloat`: `\hsize\columnwidth`,
//! `\@parboxrestore`, `\normalsize`): graphics lines and `\@makecaption`
//! (`\vskip\abovecaptionskip`, a centred `\hbox to\hsize` when the caption
//! fits on one line, else a paragraph). Its position in the text flow is a
//! marker — after the preceding paragraph (vertical mode) or after the line
//! holding it (horizontal mode, `\vadjust`) — where LaTeX's output routine
//! runs `\@addtocurcol` with the page built so far; each new page runs
//! `\@startcolumn` (`\@tryfcolumn` float pages, then `\@addtonextcol`), and
//! `\end{document}`'s `\clearpage` flushes what is left as float pages
//! (`\@makefcolumn`). The rules below are transcribed from `latex.ltx`
//! (TeX Live 2026) with article's parameters (`topnumber` 2, `\topfraction`
//! .7, `bottomnumber` 1, `\bottomfraction` .3, `totalnumber` 3,
//! `\textfraction` .2, `\floatpagefraction` .5) and `size1x.clo`'s skips.
//! Page positions follow `\@makecol`: top floats, `\textfloatsep`, the text
//! box of height `\@colroom`, bottom floats ending at the text area bottom;
//! float pages centre their floats (`\@fptop`/`\@fpsep`/`\@fpbot` fil glue).
//!
//! The box holds what the main flow would set: text paragraphs and
//! `tabular`s, and lists, displays, headings, pictures and rules through the
//! main flow's block layout (`\@setminipage`: the first `\addvspace` does
//! nothing; no `\parskip` on the empty list). Graphics and `minipage` boxes
//! (`\@iiiminipage`: `\vtop`, `$\vcenter$` or `\vbox` of their own material)
//! share lines with `\hfill`/`\hfil`/`\quad`/`\hspace` glue and spaces.
//!
//! Two-column documents: `figure*`/`table*` (`\end@dblfloat`, 1sp deep) join
//! the same float lists, where `\@testwrongwidth` keeps them out of columns.
//! Every new page runs `\@dblfloatplacement` and `\@startdblcolumn`:
//! `\@tryfcolumn` pages of wide floats (`\dblfloatpagefraction` .5,
//! `\@dblfpsep`), then `\@addtodblcol` (`dbltopnumber` 2, `\dbltopfraction`
//! .7). Placed floats sit above both columns (`\@combinedblfloats`),
//! `\dblfloatsep` apart and `\dbltextfloatsep` above the columns, whose
//! `\@colht` shrinks by the same amount. `\@doclearpage` ends a page whose
//! first column is set (an empty second column) and sets the remaining wide
//! floats on pages of their own.

use std::rc::Rc;

use flashtex_compiler::Span;

use crate::adapter::{Block as ABlock, Item as AItem, ParaStyle, TextStyle};
use crate::display::{self, Diagnostic, ImageResource, Paint, Provenance, Tick};
use crate::floats::FloatKind;
use crate::graphics::{GraphicBox, BP_PER_PT};
use crate::pagebuild::{self, badness, BuiltPage, InsertArea, InsertState, Insertions, PageIns, PageParams, Placed, VItem, AWFUL_BAD, DEPLORABLE, EJECT_PENALTY, INF_BAD, INF_PENALTY};

use super::{BoxRec, BuiltBlock, Context};

/// One float ready for layout (prepared by `crate::floats::prepare`).
#[derive(Debug, Clone)]
pub struct FloatSpec {
    pub kind: FloatKind,
    pub number: u32,
    /// `\@xfloat` placement bits (1 h, 2 t, 4 b, 8 p, 16 = not `!`).
    pub bits: u32,
    /// The environment's source span (`\begin` .. `\end`).
    pub span: Span,
    pub hmode: bool,
    /// `figure*`/`table*`: `\hsize` is `\textwidth` (`\@dblfloat`).
    pub wide: bool,
    pub parts: Vec<FloatPart>,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum FloatPart {
    Centering,
    ParBreak,
    Graphic(PreparedGraphic),
    /// The caption paragraph's items, `Figure~N: ` prefix included.
    Caption { items: Vec<AItem> },
    /// A paragraph of the float body (text, a `tabular`, ...) as the
    /// adapter sets it in the main flow, with its environment skips.
    Text {
        items: Vec<AItem>,
        style: ParaStyle,
        /// A `center`-like environment opens here: `Some(vmode)`.
        env_open: Option<bool>,
        env_close: bool,
        /// `\vspace` before the paragraph, in points.
        vspace_before: f64,
        /// `\addvspace` before the paragraph, in points.
        addvspace_before: f64,
    },
    /// A size declaration at the float's top level (`\small`), hundredths
    /// of a point (0 = `\normalsize`): the `\baselineskip` from here on.
    Size(u16),
    /// Lists, displays, headings, pictures and rules, set by the main flow's
    /// block layout inside the box.
    Flow(Vec<ABlock>),
    /// A `minipage` of `width`: its own box, placed on the line like a
    /// graphic (`pos` `t`, `c` or `b`).
    Minipage { pos: u8, width: f64, parts: Vec<FloatPart>, span: Span },
    /// Glue on a line of boxes: `order` 0 finite, 1 `fil`, 2 `fill`.
    HSkip { width: f64, order: u8 },
    /// An interword space on a line of boxes.
    Space,
    /// `\hrule height<h>` across the box (float.sty's `ruled`/`boxed`
    /// styles, `crate::algorithms`): a rule node, so no interline glue
    /// follows it and `\prevdepth` is left at `ignore_depth` (TeX 1056).
    Rule { height: f64, span: Span },
    /// `\kern` of `pt` plus `em` of the body font. A kern, not glue:
    /// `\lastskip` stays 0 and `\unskip` does not remove it.
    Kern { pt: f64, em: f64 },
    /// `\addvspace` of `em` of the body font (`\@endparenv`'s
    /// `\addvspace\@topsepadd` after a pseudocode list).
    AddVSpace { em: f64 },
    /// A float.sty caption (`\floatc@ruled`, `\floatc@plain`): a paragraph
    /// with no `\abovecaptionskip` of its own, appended with
    /// `\unvbox\@floatcapt` so it takes no interline glue either; centred
    /// when `center_if_fits` and it fits on one line.
    StyleCaption { items: Vec<AItem>, center_if_fits: bool },
    /// One pseudocode statement line.
    AlgLine(Box<super::algorithms::AlgLine>),
}

#[derive(Debug, Clone)]
pub struct PreparedGraphic {
    pub gbox: GraphicBox,
    /// `None` when the file could not be read but its size was known from
    /// `width` and `height` (space is kept, nothing is painted).
    pub resource: Option<Rc<ImageResource>>,
    /// graphicx `clip` with a viewport/trim: the unit square's visible part.
    pub clip: Option<[f64; 4]>,
    pub span: Span,
    /// graphicx `demo`: no file; painted as a black rule of the box's size.
    pub demo: bool,
}

#[derive(Clone, Copy)]
struct Skip {
    n: f64,
    st: f64,
    sh: f64,
}

#[derive(Clone, Copy)]
struct FloatParams {
    floatsep: Skip,
    textfloatsep: Skip,
    intextsep: Skip,
    fpsep: f64,
    abovecaptionskip: f64,
    /// `\dblfloatsep`, `\dbltextfloatsep` and `\@dblfpsep` (natural widths).
    dblfloatsep: f64,
    dbltextfloatsep: f64,
    dblfpsep: f64,
}

impl FloatParams {
    fn for_size(body_pt: f64) -> FloatParams {
        let s = |n, st, sh| Skip { n, st, sh };
        if body_pt >= 11.5 {
            FloatParams { floatsep: s(12.0, 2.0, 4.0), textfloatsep: s(20.0, 2.0, 4.0), intextsep: s(14.0, 4.0, 4.0), fpsep: 10.0, abovecaptionskip: 10.0, dblfloatsep: 14.0, dbltextfloatsep: 20.0, dblfpsep: 10.0 }
        } else {
            FloatParams { floatsep: s(12.0, 2.0, 2.0), textfloatsep: s(20.0, 2.0, 4.0), intextsep: s(12.0, 2.0, 2.0), fpsep: 8.0, abovecaptionskip: 10.0, dblfloatsep: 12.0, dbltextfloatsep: 20.0, dblfpsep: 8.0 }
        }
    }
}

enum Elem {
    /// A text line: block/line in `blocks`, baseline from the box top.
    Line { block: usize, line: usize, baseline: f64, height: f64, depth: f64 },
    Image { x: f64, baseline: f64, gbox: GraphicBox, resource: Option<Rc<ImageResource>>, clip: Option<[f64; 4]>, provenance: Provenance, demo: bool },
    /// An `\hrule` of the box's width: its top measured from the box top.
    Rule { x: f64, top: f64, width: f64, height: f64, provenance: Provenance },
}

struct FloatBox {
    height: f64,
    elems: Vec<Elem>,
    type_bit: u32,
    labels: Vec<String>,
    /// `figure*`/`table*` in a two-column document (`\dp` 1sp).
    dbl: bool,
}

/// The last node of a box's vertical list.
#[derive(Clone, Copy, PartialEq)]
enum Last {
    Nothing,
    Box(f64),
    /// Glue, and the depth of the box before it when there is one.
    Glue { width: f64, before: Option<f64> },
}

/// A box's vertical list while it is set (natural height).
struct VState {
    y: f64,
    prev_depth: Option<f64>,
    /// `\if@minipage`: `\@setminipage` at the box top, cleared by the first
    /// paragraph (`\everypar`) or a one-line caption; `\addvspace` does
    /// nothing while it is set.
    minipage: bool,
    last: Last,
    /// The height of the first node when it is a box (0 for glue): `\vtop`.
    first_height: Option<f64>,
}

impl VState {
    fn new() -> VState {
        VState { y: 0.0, prev_depth: None, minipage: true, last: Last::Nothing, first_height: None }
    }

    /// Appends a box with interline glue (§679).
    fn add_box(&mut self, h: f64, d: f64, baselineskip: f64, lineskip: f64, lineskiplimit: f64) -> f64 {
        if let Some(pd) = self.prev_depth {
            let mut g = baselineskip - pd - h;
            if g < lineskiplimit {
                g = lineskip;
            }
            self.y += g;
        }
        self.place_box(h, d)
    }

    /// Appends a box whose interline glue the list already holds.
    fn place_box(&mut self, h: f64, d: f64) -> f64 {
        let b = self.y + h;
        self.y = b + d;
        self.prev_depth = Some(d);
        self.first_height.get_or_insert(h);
        self.last = Last::Box(d);
        b
    }

    /// A rule node (`\hrule height<h>`): no interline glue, and
    /// `\prevdepth` is left at `ignore_depth` (TeX 1056). `\lastskip` is 0
    /// after it, and `\unskip` cannot remove it.
    fn rule(&mut self, height: f64) -> f64 {
        let top = self.y;
        self.y += height;
        self.prev_depth = None;
        self.first_height.get_or_insert(height);
        self.last = Last::Box(0.0);
        top
    }

    /// A kern node: like `vskip` for the box's height, but `\lastskip` is 0
    /// after it and `\unskip` does not remove it.
    fn kern(&mut self, pt: f64) {
        self.y += pt;
        self.first_height.get_or_insert(0.0);
        self.last = Last::Box(0.0);
    }

    fn vskip(&mut self, pt: f64) {
        self.y += pt;
        self.first_height.get_or_insert(0.0);
        let before = if let Last::Box(d) = self.last { Some(d) } else { None };
        self.last = Last::Glue { width: pt, before };
    }

    /// `\lastskip`: the glue the list ends with (0 after a box).
    fn last_skip(&self) -> f64 {
        if let Last::Glue { width, .. } = self.last {
            width
        } else {
            0.0
        }
    }

    /// `\addvspace` (latex.ltx `\@xaddvskip`, natural widths).
    fn addvspace(&mut self, pt: f64) {
        if self.minipage {
            return;
        }
        let last = self.last_skip();
        if last == 0.0 {
            self.vskip(pt);
        } else if last < pt {
            self.y += pt - last;
            if let Last::Glue { width, .. } = &mut self.last {
                *width = pt;
            }
        }
    }

    /// `\unskip`: removes the glue the list ends with.
    fn unskip(&mut self) {
        if let Last::Glue { width, before } = self.last {
            self.y -= width;
            self.last = before.map_or(Last::Glue { width: 0.0, before: None }, Last::Box);
        }
    }
}

/// `\baselineskip` of a size declaration (0 = `\normalsize`).
fn size_baselineskip(ctx: &Context, size_cpt: u16) -> f64 {
    let body = ctx.style.body_size_pt;
    if size_cpt == 0 || (f64::from(size_cpt) / 100.0 - body).abs() < 1e-9 {
        ctx.style.baselineskip_pt
    } else {
        crate::table::baselineskip_pt(crate::adapter::class_size_of(body), size_cpt)
    }
}

/// The size a paragraph is set at: its first sized word or table.
fn items_size(items: &[AItem]) -> u16 {
    items
        .iter()
        .find_map(|i| match i {
            AItem::Word(w) => w.segments.iter().map(|s| s.style.size_cpt).find(|c| *c != 0),
            AItem::Table(t) => Some(t.size_cpt).filter(|c| *c != 0),
            _ => None,
        })
        .unwrap_or(0)
}

/// A box being set.
struct VBox {
    v: VState,
    elems: Vec<Elem>,
}

impl VBox {
    /// Height and depth as `\vtop` (`t`: the first box's height), `\vbox`
    /// (`b`: the last box's depth) or `$\vcenter$` (`c`: centred on the math
    /// axis `axis`).
    fn extents(&self, pos: u8, axis: f64) -> (f64, f64) {
        let total = self.v.y;
        match pos {
            b't' => {
                let h = self.v.first_height.unwrap_or(0.0);
                (h, total - h)
            }
            b'b' => {
                let d = if let Last::Box(d) = self.v.last { d } else { 0.0 };
                (total - d, d)
            }
            _ => (total / 2.0 + axis, total / 2.0 - axis),
        }
    }
}

/// Material of the current line of boxes.
enum HItem<'p> {
    Graphic(&'p PreparedGraphic),
    Mini { vbox: VBox, width: f64, pos: u8 },
    Glue { width: f64, order: u8 },
}

impl HItem<'_> {
    fn width(&self) -> f64 {
        match self {
            HItem::Graphic(g) => g.gbox.width,
            HItem::Mini { width, .. } | HItem::Glue { width, .. } => *width,
        }
    }

    fn is_glue(&self) -> bool {
        matches!(self, HItem::Glue { .. })
    }
}

/// Sets `parts` in a `\vbox` of width `hsize` (see the module docs):
/// `\@parboxrestore` (no `\parindent`, `\parskip` 0, `\sloppy`) and
/// `\@setminipage`. `minipage`: `\endminipage`'s `\par\unskip` at the end.
fn set_box(ctx: &mut Context, blocks: &mut Vec<BuiltBlock>, parts: &[FloatPart], hsize: f64, fp: &FloatParams, minipage: bool) -> VBox {
    let s = ctx.style;
    let (normal_bs, ls, lsl) = (s.baselineskip_pt, s.lineskip_pt, s.lineskiplimit_pt);
    let (topsep, partopsep) = (s.topsep.natural, s.partopsep.natural);
    let saved = (ctx.hsize_override, ctx.sloppy, ctx.baselineskip_override, ctx.parbox);
    ctx.hsize_override = Some(hsize);
    ctx.sloppy = true;
    ctx.parbox = true;
    let text_params = ctx.text_params(TextStyle::default(), s.body_size_pt);
    let space = text_params.space;
    // `\kern2pt` and `\topsep` of the pseudocode lists are in ems.
    let quad = text_params.quad;
    // `\fontdimen22` of the math symbol font (Latin Modern: .25em).
    let axis = 0.25 * s.body_size_pt;
    let mut bx = VBox { v: VState::new(), elems: Vec::new() };
    if !minipage {
        // The float's `\vbox` is not empty when its material starts (pdfTeX
        // adds `\parsep` above a list at a float top, fixtures/float-bodies
        // 01, 02), unlike a minipage's.
        bx.v.last = Last::Glue { width: 0.0, before: None };
    }
    let mut centered = false;
    let mut bs = normal_bs;
    let mut env_vmode = false;
    let mut line: Vec<HItem> = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        match &parts[i] {
            FloatPart::Centering => centered = true,
            FloatPart::Size(cpt) => bs = size_baselineskip(ctx, *cpt),
            FloatPart::Graphic(g) => line.push(HItem::Graphic(g)),
            FloatPart::HSkip { width, order } => line.push(HItem::Glue { width: *width, order: *order }),
            FloatPart::Space => {
                if !line.is_empty() {
                    line.push(HItem::Glue { width: space, order: 0 });
                }
            }
            FloatPart::Minipage { pos, width, parts: inner, .. } => {
                let vbox = set_box(ctx, blocks, inner, *width, fp, true);
                line.push(HItem::Mini { vbox, width: *width, pos: *pos });
            }
            FloatPart::ParBreak => flush_line(ctx, blocks, &mut bx, &mut line, centered, bs, hsize, axis),
            FloatPart::Text { items, style, env_open, env_close, vspace_before, addvspace_before } => {
                flush_line(ctx, blocks, &mut bx, &mut line, centered, bs, hsize, axis);
                if *vspace_before != 0.0 {
                    bx.v.vskip(*vspace_before);
                }
                if *addvspace_before != 0.0 {
                    bx.v.addvspace(*addvspace_before);
                }
                if let Some(vmode) = env_open {
                    // `\@trivlist`: `\@topsep` = `\topsep` (+ `\partopsep`
                    // from vertical mode) + `\parskip` (0), by `\addvspace`.
                    env_vmode = *vmode;
                    bx.v.addvspace(topsep + if *vmode { partopsep } else { 0.0 });
                }
                let para_bs = match items_size(items) {
                    0 => bs,
                    c => size_baselineskip(ctx, c),
                };
                ctx.baselineskip_override = Some(para_bs);
                if let Some(block) = ctx.paragraph_block(items, false, false, false, *style, None) {
                    let bi = blocks.len();
                    for (li, ln) in block.block.lines.lines.iter().enumerate() {
                        let b = bx.v.add_box(ln.height, ln.depth, para_bs, ls, lsl);
                        bx.elems.push(Elem::Line { block: bi, line: li, baseline: b, height: ln.height, depth: ln.depth });
                        if let Some(&sk) = block.vertical.vskip_after.get(li).filter(|sk| **sk != 0.0) {
                            bx.v.vskip(sk);
                        }
                    }
                    bx.v.minipage = false;
                    blocks.push(block);
                }
                ctx.baselineskip_override = saved.2;
                if *env_close {
                    // `\@endparenv`: `\addvspace\@topsepadd`.
                    bx.v.addvspace(topsep + if env_vmode { partopsep } else { 0.0 });
                }
            }
            FloatPart::Rule { height, span } => {
                flush_line(ctx, blocks, &mut bx, &mut line, centered, bs, hsize, axis);
                let top = bx.v.rule(*height);
                bx.elems.push(Elem::Rule { x: 0.0, top, width: hsize, height: *height, provenance: Provenance::Source(ctx.source(*span)) });
                bx.v.minipage = false;
            }
            FloatPart::Kern { pt, em } => {
                flush_line(ctx, blocks, &mut bx, &mut line, centered, bs, hsize, axis);
                bx.v.kern(pt + em * quad);
            }
            FloatPart::AddVSpace { em } => {
                flush_line(ctx, blocks, &mut bx, &mut line, centered, bs, hsize, axis);
                bx.v.addvspace(em * quad);
            }
            FloatPart::StyleCaption { items, center_if_fits } => {
                flush_line(ctx, blocks, &mut bx, &mut line, centered, bs, hsize, axis);
                // float.sty's `\@fs@capt` runs inside the float box with no
                // `\abovecaptionskip`; `\float@caption` boxes it and
                // `\unvbox`es it into place, so its first line takes no
                // interline glue either.
                let Some(mut block) = ctx.paragraph_block(items, false, true, false, ParaStyle::Plain, None) else {
                    i += 1;
                    continue;
                };
                let lines = &block.block.lines.lines;
                if *center_if_fits && lines.len() == 1 && lines[0].natural_width <= hsize + 1e-6 {
                    if let Some(b) = ctx.paragraph_block(items, false, true, false, ParaStyle::Center, None) {
                        block = b;
                    }
                }
                bx.v.prev_depth = None;
                let bi = blocks.len();
                for (li, ln) in block.block.lines.lines.iter().enumerate() {
                    let b = bx.v.add_box(ln.height, ln.depth, normal_bs, ls, lsl);
                    bx.elems.push(Elem::Line { block: bi, line: li, baseline: b, height: ln.height, depth: ln.depth });
                }
                bx.v.minipage = false;
                blocks.push(block);
            }
            FloatPart::AlgLine(alg) => {
                flush_line(ctx, blocks, &mut bx, &mut line, centered, bs, hsize, axis);
                if alg.no_text {
                    // algorithmicx's `\item[]\nointerlineskip` and an empty
                    // line (algorithmicx.sty 194).
                    bx.v.prev_depth = None;
                    bx.v.place_box(0.0, 0.0);
                    bx.v.minipage = false;
                    i += 1;
                    continue;
                }
                match ctx.algorithm_line_block(alg) {
                    Some(block) => {
                        let bi = blocks.len();
                        for (li, ln) in block.block.lines.lines.iter().enumerate() {
                            let b = bx.v.add_box(ln.height, ln.depth, bs, ls, lsl);
                            bx.elems.push(Elem::Line { block: bi, line: li, baseline: b, height: ln.height, depth: ln.depth });
                        }
                        blocks.push(block);
                    }
                    // An `\item` without material still sets an empty line.
                    None => {
                        bx.v.add_box(0.0, 0.0, bs, ls, lsl);
                    }
                }
                bx.v.minipage = false;
            }
            FloatPart::Caption { items } => {
                flush_line(ctx, blocks, &mut bx, &mut line, centered, bs, hsize, axis);
                // `\@caption`: `\par`, `\@parboxrestore`, `\normalsize`, then
                // `\@makecaption`'s `\vskip\abovecaptionskip`.
                bx.v.vskip(fp.abovecaptionskip);
                if let Some(mut block) = ctx.paragraph_block(items, false, true, false, ParaStyle::Plain, None) {
                    let lines = &block.block.lines.lines;
                    // `\@caption` runs `\@parboxrestore` before `\@makecaption`,
                    // so `\centering` does not reach a caption set as a
                    // paragraph: only the one-line `\hbox to\hsize{\hfil...\hfil}`
                    // is centred.
                    let fits = lines.len() == 1 && lines[0].natural_width <= hsize + 1e-6;
                    if fits {
                        if let Some(b) = ctx.paragraph_block(items, false, true, false, ParaStyle::Center, None) {
                            block = b;
                        }
                    }
                    let bi = blocks.len();
                    for (li, ln) in block.block.lines.lines.iter().enumerate() {
                        let b = bx.v.add_box(ln.height, ln.depth, normal_bs, ls, lsl);
                        bx.elems.push(Elem::Line { block: bi, line: li, baseline: b, height: ln.height, depth: ln.depth });
                    }
                    bx.v.minipage = false;
                    // `\vskip\belowcaptionskip` (article: 0pt).
                    bx.v.vskip(0.0);
                    blocks.push(block);
                }
            }
            FloatPart::Flow(_) => {
                flush_line(ctx, blocks, &mut bx, &mut line, centered, bs, hsize, axis);
                // A run of flow blocks is laid out together (a list's items,
                // the skips between them); blank lines between do not matter.
                let mut group = Vec::new();
                while i < parts.len() {
                    match &parts[i] {
                        FloatPart::Flow(b) => group.extend(b.iter().cloned()),
                        FloatPart::ParBreak | FloatPart::Space => {}
                        _ => break,
                    }
                    i += 1;
                }
                let closing = list_closing_skip(&group, ctx.style.parskip.natural);
                set_flow(ctx, blocks, &mut bx, group, centered);
                // `\@endparenv` of a list that ends the group: `\addvspace
                // \@topsepadd` (a paragraph of the same body that follows
                // already carries it from the adapter).
                if let Some(skip) = closing.filter(|_| !matches!(parts.get(i), Some(FloatPart::Text { .. }))) {
                    bx.v.addvspace(skip);
                }
                continue;
            }
        }
        i += 1;
    }
    flush_line(ctx, blocks, &mut bx, &mut line, centered, bs, hsize, axis);
    if minipage {
        bx.v.unskip();
    }
    (ctx.hsize_override, ctx.sloppy, ctx.baselineskip_override, ctx.parbox) = saved;
    bx
}

/// Ends a paragraph of boxes (graphics, minipages) and glue: TeX's lines of
/// `\hsize` (`\leftskip`/`\rightskip` `fil` under `\centering`, else
/// `\parfillskip`; `\hfill` beats both), broken at glue when too wide.
#[allow(clippy::too_many_arguments)]
fn flush_line(ctx: &Context, blocks: &mut [BuiltBlock], bx: &mut VBox, line: &mut Vec<HItem>, centered: bool, bs: f64, hsize: f64, axis: f64) {
    // `\par` removes the glue a paragraph ends with.
    while line.last().is_some_and(HItem::is_glue) {
        line.pop();
    }
    if line.is_empty() {
        return;
    }
    let s = ctx.style;
    let (ls, lsl) = (s.lineskip_pt, s.lineskiplimit_pt);
    let mut rows: Vec<Vec<HItem>> = vec![Vec::new()];
    let mut w = 0.0;
    for item in line.drain(..) {
        let row = rows.last_mut().expect("a row");
        if !item.is_glue() && w + item.width() > hsize + 1e-6 && row.iter().any(|x| !x.is_glue()) {
            while row.last().is_some_and(HItem::is_glue) {
                row.pop();
            }
            rows.push(Vec::new());
            w = 0.0;
        } else if item.is_glue() && row.is_empty() && rows.len() > 1 {
            // Glue after a break is discarded.
            continue;
        }
        w += item.width();
        rows.last_mut().expect("a row").push(item);
    }
    for row in rows {
        let natural: f64 = row.iter().map(HItem::width).sum();
        let extra = hsize - natural;
        let count = |o: u8| row.iter().filter(|x| matches!(x, HItem::Glue { order, .. } if *order == o)).count() as f64;
        let (fill, fil) = (count(2), count(1));
        let (lead, per_fill, per_fil) = if extra <= 0.0 {
            (0.0, 0.0, 0.0)
        } else if fill > 0.0 {
            (0.0, extra / fill, 0.0)
        } else {
            let n = fil + if centered { 2.0 } else { 1.0 };
            (if centered { extra / n } else { 0.0 }, 0.0, extra / n)
        };
        let (mut h, mut d) = (0.0f64, 0.0f64);
        for it in &row {
            let (ih, id) = match it {
                HItem::Graphic(g) => (g.gbox.height, g.gbox.depth),
                HItem::Mini { vbox, pos, .. } => vbox.extents(*pos, axis),
                HItem::Glue { .. } => continue,
            };
            h = h.max(ih);
            d = d.max(id);
        }
        let baseline = bx.v.add_box(h, d, bs, ls, lsl);
        bx.v.minipage = false;
        let mut x = lead;
        for it in row {
            match it {
                HItem::Glue { width, order } => {
                    x += width
                        + match order {
                            2 => per_fill,
                            1 => per_fil,
                            _ => 0.0,
                        }
                }
                HItem::Graphic(g) => {
                    bx.elems.push(Elem::Image { x, baseline, gbox: g.gbox, resource: g.resource.clone(), clip: g.clip, provenance: Provenance::Source(ctx.source(g.span)), demo: g.demo });
                    x += g.gbox.width;
                }
                HItem::Mini { vbox, width, pos } => {
                    let top = baseline - vbox.extents(pos, axis).0;
                    for e in vbox.elems {
                        bx.elems.push(match e {
                            Elem::Line { block, line, baseline: lb, height, depth } => {
                                for r in &mut blocks[block].block.lines.lines[line].runs {
                                    r.x += x;
                                }
                                Elem::Line { block, line, baseline: top + lb, height, depth }
                            }
                            Elem::Image { x: ix, baseline: ib, gbox, resource, clip, provenance, demo } => Elem::Image { x: x + ix, baseline: top + ib, gbox, resource, clip, provenance, demo },
                            Elem::Rule { x: rx, top: rt, width, height, provenance } => Elem::Rule { x: x + rx, top: top + rt, width, height, provenance },
                        });
                    }
                    x += width;
                }
            }
        }
    }
}

/// `\@topsepadd` of the outermost list when `group` ends inside a list: the
/// first outer `\item`'s `\addvspace` (`\topsep` + `\partopsep` from
/// vertical mode + the outer `\parskip`, adapter) less that `\parskip`.
fn list_closing_skip(group: &[ABlock], outer_parskip: f64) -> Option<f64> {
    if !matches!(group.last(), Some(ABlock::Paragraph { list: Some(_), .. })) {
        return None;
    }
    group.iter().find_map(|b| match b {
        ABlock::Paragraph { list: Some(g), addvspace_before, .. } if g.level == 1 && g.label.is_some() => Some(addvspace_before - outer_parskip),
        _ => None,
    })
}

/// Lists, displays, headings, pictures and rules: the main flow's block
/// layout, appended to the box's vertical list.
fn set_flow(ctx: &mut Context, blocks: &mut Vec<BuiltBlock>, bx: &mut VBox, mut group: Vec<ABlock>, centered: bool) {
    for b in &mut group {
        match b {
            ABlock::Paragraph { indent, eject_before, .. } => {
                *indent = false;
                *eject_before = false;
            }
            ABlock::Picture { centered: c, .. } => *c |= centered,
            _ => {}
        }
    }
    let keep = match group.first() {
        Some(ABlock::Paragraph { vspace_before, .. } | ABlock::Heading { vspace_before, .. } | ABlock::Picture { vspace_before, .. } | ABlock::Rule { vspace_before, .. }) => *vspace_before,
        _ => 0.0,
    };
    let mut built = super::layout_blocks(ctx, &group, &[], 0, false, None).blocks;
    let Some(first) = built.first_mut() else { return };
    if bx.v.last == Last::Nothing {
        // TeX adds no `\parskip` to an empty internal vertical list.
        first.vertical.parskip = None;
    }
    first.vertical.penalty_before = None;
    if let Some((n, st, sh)) = first.vertical.space_before {
        // `\addvspace` does nothing at the box top (`\@setminipage`) and
        // otherwise adds its excess over the glue the list ends with;
        // `\vspace` stays.
        let added = n - keep;
        let kept = if bx.v.minipage { 0.0 } else { (added - bx.v.last_skip()).max(0.0) };
        first.vertical.space_before = (keep + kept != 0.0).then_some((keep + kept, st, sh));
    }
    let s = ctx.style;
    let p = PageParams { vsize: s.text_height_pt, topskip: 0.0, maxdepth: s.maxdepth_pt, baselineskip: s.baselineskip_pt, lineskip: s.lineskip_pt, lineskiplimit: s.lineskiplimit_pt, flushbottom: false };
    let vbs: Vec<pagebuild::VBlock> = built.iter().map(|b| b.vertical.clone()).collect();
    let list = pagebuild::vlist(&p, &vbs);
    let base = blocks.len();
    let mut seen_box = false;
    for item in &list {
        match *item {
            VItem::Glue { width, .. } => bx.v.vskip(width),
            VItem::Penalty(_) => {}
            VItem::Box { height, depth, payload: (bi, li) } => {
                let baseline = if seen_box {
                    bx.v.place_box(height, depth)
                } else {
                    if vbs[bi].no_interline_first {
                        bx.v.prev_depth = None;
                    }
                    bx.v.add_box(height, depth, vbs[bi].baselineskip.unwrap_or(p.baselineskip), p.lineskip, p.lineskiplimit)
                };
                seen_box = true;
                bx.v.minipage = false;
                bx.elems.push(Elem::Line { block: base + bi, line: li, baseline, height, depth });
            }
        }
    }
    if seen_box && vbs.iter().rev().find(|b| !b.lines.is_empty()).is_some_and(|b| b.no_interline_after) {
        bx.v.prev_depth = None;
    }
    blocks.extend(built);
}

/// Sets the float's box: `\@xfloat`'s `\vbox{\hsize\columnwidth
/// \@parboxrestore \@floatboxreset ...}` (`\textwidth` for
/// `figure*`/`table*`), whose natural height is the float's height.
fn build_box(ctx: &mut Context, blocks: &mut Vec<BuiltBlock>, spec: &FloatSpec, fp: &FloatParams, twocolumn: bool) -> FloatBox {
    let s = ctx.style;
    let tw = if spec.wide { s.class_geometry.as_deref().map_or(s.text_width_pt, |g| crate::style::frame_pt(g.frame.text_width)) } else { s.text_width_pt };
    let bx = set_box(ctx, blocks, &spec.parts, tw, fp, false);
    let mut height = bx.v.y;
    if height > s.text_height_pt {
        ctx.diagnostics.push(Diagnostic::warning(
            "float_too_large",
            format!("{} {} is {:.2}pt taller than the text area", spec.kind.name(), spec.number, height - s.text_height_pt),
            vec![ctx.source(spec.span)],
        ));
        height = s.text_height_pt;
    }
    FloatBox { height, elems: bx.elems, type_bit: spec.kind.type_bit(), labels: spec.labels.clone(), dbl: spec.wide && twocolumn }
}

#[derive(Clone, Copy)]
enum N {
    V(usize),
    Penalty(i32),
    Glue(f64, f64, f64),
    Marker(usize),
    FBox(usize),
}

struct Col {
    colroom: f64,
    toproom: f64,
    botroom: f64,
    topnum: i32,
    botnum: i32,
    colnum: i32,
    textfloatsheight: f64,
    top: Vec<usize>,
    bot: Vec<usize>,
    mid: Vec<usize>,
}

impl Col {
    fn new(colht: f64) -> Col {
        Col { colroom: colht, toproom: 0.7 * colht, botroom: 0.3 * colht, topnum: 2, botnum: 1, colnum: 3, textfloatsheight: 0.0, top: Vec::new(), bot: Vec::new(), mid: Vec::new() }
    }
}

struct Placer<'b> {
    boxes: &'b [FloatBox],
    bits: Vec<u32>,
    fp: FloatParams,
    colht: f64,
    /// `\textheight`, and the first page's column count with its reduced
    /// `\@colht` below a `\twocolumn[...]` box (`\@topnewpage`).
    full_colht: f64,
    first_colht: Option<(usize, f64)>,
    parskip: Skip,
    text_x: f64,
    text_y: f64,
    pages: Vec<BuiltPage>,
    images: Vec<(u32, display::Item)>,
    labels: Vec<(String, u32)>,
    col: Col,
    deferred: Vec<usize>,
    twocolumn: bool,
    /// `\f@depth` is 1sp (`\@dblfloatplacement`): only wide floats fit.
    dbl_phase: bool,
    /// The current page's `\@colht` after `\@addtodblcol`, and the wide
    /// floats above its columns (not yet emitted).
    page_colht: f64,
    dbl_top: Vec<usize>,
}

fn has_type(boxes: &[FloatBox], list: &[usize], ty: u32) -> bool {
    list.iter().any(|g| boxes[*g].type_bit == ty)
}

/// `\@flsetnum`: a `!` float treats an exhausted count as 1.
fn flsetnum(n: i32, fps: u32) -> i32 {
    if fps < 16 && n == 0 {
        1
    } else {
        n
    }
}

impl Placer<'_> {
    fn fps(&self, f: usize) -> u32 {
        self.bits[f] & 31
    }

    /// `\@testwrongwidth`.
    fn wrong_width(&self, f: usize) -> bool {
        self.boxes[f].dbl != self.dbl_phase
    }

    /// How far the current page's columns start below the text top (the
    /// wide floats above them and `\dbltextfloatsep`).
    fn dbl_off(&self) -> f64 {
        self.full_colht - self.page_colht
    }

    fn textmin(&self, f: usize) -> f64 {
        if self.fps(f) < 16 {
            0.0
        } else {
            0.2 * self.colht
        }
    }

    fn add_to_bot(&mut self, f: usize, saved: f64, colnum: i32) -> bool {
        let fps = self.fps(f);
        let ht = self.boxes[f].height;
        if fps & 4 == 0 {
            return false;
        }
        let botnum = flsetnum(self.col.botnum, fps);
        if botnum <= 0 {
            return false;
        }
        let sep = if self.col.bot.is_empty() { self.fp.textfloatsep.n } else { self.fp.floatsep.n };
        if self.col.colroom > saved + sep && (self.col.botroom > ht || fps < 16) {
            self.col.botnum = botnum - 1;
            self.col.colnum = colnum - 1;
            self.col.botroom -= ht + sep;
            self.col.colroom -= ht + sep;
            self.col.bot.push(f);
            return true;
        }
        false
    }

    fn add_to_top_or_bot(&mut self, f: usize, saved: f64, colnum: i32) -> bool {
        let fps = self.fps(f);
        let ht = self.boxes[f].height;
        let ty = self.boxes[f].type_bit;
        if fps & 2 != 0 {
            let topnum = flsetnum(self.col.topnum, fps);
            if topnum > 0 {
                let sep = if self.col.top.is_empty() { self.fp.textfloatsep.n } else { self.fp.floatsep.n };
                if self.col.colroom > saved + sep && (self.col.toproom > ht || fps < 16) && !has_type(self.boxes, &self.col.mid, ty) && !has_type(self.boxes, &self.col.bot, ty) {
                    self.col.topnum = topnum - 1;
                    self.col.colnum = colnum - 1;
                    self.col.toproom -= ht + sep;
                    self.col.colroom -= ht + sep;
                    self.col.top.push(f);
                    return true;
                }
            }
        }
        self.add_to_bot(f, saved, colnum)
    }

    /// `\@addtocurcol`. Returns the nodes to insert for a `here` float.
    fn add_to_cur_col(&mut self, f: usize, pageht: f64, vmode: bool) -> Option<Vec<N>> {
        let fps = self.fps(f);
        let ht = self.boxes[f].height;
        let ty = self.boxes[f].type_bit;
        let mut inserted = false;
        let mut here = None;
        if fps != 8 && fps != 24 {
            let textmin = self.textmin(f) + self.col.textfloatsheight;
            let req = pageht.max(textmin) + ht;
            if self.col.colroom > req {
                let colnum = flsetnum(self.col.colnum, fps);
                if colnum > 0 && !has_type(self.boxes, &self.deferred, ty) && !self.wrong_width(f) {
                    if has_type(self.boxes, &self.col.bot, ty) {
                        inserted = self.add_to_bot(f, req, colnum);
                    } else {
                        if fps & 1 != 0 && self.col.colroom > req + self.fp.intextsep.n {
                            let i = self.fp.intextsep;
                            self.col.colnum = colnum - 1;
                            self.col.textfloatsheight += ht + 2.0 * i.n;
                            self.col.mid.push(f);
                            let mut nodes = vec![N::Penalty(0), N::Glue(i.n, i.st, i.sh), N::FBox(f), N::Penalty(0), N::Glue(i.n, i.st, i.sh)];
                            if vmode {
                                nodes.push(N::Glue(-self.parskip.n, -self.parskip.st, -self.parskip.sh));
                            }
                            here = Some(nodes);
                            inserted = true;
                        }
                        if !inserted {
                            inserted = self.add_to_top_or_bot(f, req, colnum);
                        }
                    }
                }
            }
        }
        if !inserted {
            // `\@resethfps`: a lone `h` becomes `ht`.
            if fps == 1 || fps == 17 {
                self.bits[f] |= 2;
            }
            self.deferred.push(f);
        }
        here
    }

    /// `\@addtonextcol` for every deferred float, in order.
    fn add_to_next_col(&mut self) {
        let old = std::mem::take(&mut self.deferred);
        for f in old {
            let fps = self.fps(f);
            let mut inserted = false;
            if fps != 8 && fps != 24 {
                let req = self.boxes[f].height + self.textmin(f);
                if self.col.colroom > req {
                    let colnum = flsetnum(self.col.colnum, fps);
                    if colnum > 0 && !has_type(self.boxes, &self.deferred, self.boxes[f].type_bit) && !self.wrong_width(f) {
                        inserted = self.add_to_top_or_bot(f, req, colnum);
                    }
                }
            }
            if !inserted {
                self.deferred.push(f);
            }
        }
    }

    /// `\@tryfcolumn` over the deferred list with `\@colht` `colht` and
    /// `\@fpsep` `fpsep`: `(floats on the page, rest)`.
    fn try_fcolumn(&self, fpmin: f64, test_p: bool, colht: f64, fpsep: f64) -> Option<(Vec<usize>, Vec<usize>)> {
        let list = &self.deferred;
        let mut failed: Vec<usize> = Vec::new();
        for (idx, &f) in list.iter().enumerate() {
            let b = &self.boxes[f];
            if has_type(self.boxes, &failed, b.type_bit) || (test_p && self.fps(f) & 8 == 0) || self.wrong_width(f) || b.height > colht {
                failed.push(f);
                continue;
            }
            let mut succeed = vec![f];
            let mut flfail: Vec<usize> = Vec::new();
            let mut h = b.height;
            for &g in &list[idx + 1..] {
                let gb = &self.boxes[g];
                let blocked = has_type(self.boxes, &failed, gb.type_bit) || has_type(self.boxes, &flfail, gb.type_bit);
                if blocked || (test_p && self.fps(g) & 8 == 0) || self.wrong_width(g) || h + gb.height + fpsep > colht {
                    flfail.push(g);
                } else {
                    h += gb.height + fpsep;
                    succeed.push(g);
                }
            }
            if h > fpmin {
                failed.extend(flfail);
                return Some((succeed, failed));
            }
            failed.push(f);
        }
        None
    }

    fn emit(&mut self, f: usize, top: f64, page: u32, lines: &mut Vec<Placed>) {
        let b = &self.boxes[f];
        for e in &b.elems {
            match e {
                Elem::Line { block, line, baseline, height, depth } => lines.push(Placed { payload: (*block, *line), baseline: top + baseline, height: *height, depth: *depth }),
                Elem::Rule { x, top: rule_top, width, height, provenance } => {
                    self.images.push((
                        page,
                        display::Item::Rule(display::Rule {
                            x: Tick::from_tex_pt(self.text_x + x),
                            top: Tick::from_tex_pt(self.text_y + top + rule_top),
                            width: Tick::from_tex_pt(*width),
                            height: Tick::from_tex_pt(*height),
                            paint: Paint::BLACK,
                            provenance: provenance.clone(),
                        }),
                    ));
                }
                Elem::Image { x, baseline, gbox, resource, clip, provenance, demo } => {
                    let left = self.text_x + x;
                    let base = self.text_y + top + baseline;
                    if *demo {
                        // graphicx `demo`: `\rule{<width>}{<height>}`.
                        self.images.push((
                            page,
                            display::Item::Rule(display::Rule {
                                x: Tick::from_tex_pt(left),
                                top: Tick::from_tex_pt(base - gbox.height),
                                width: Tick::from_tex_pt(gbox.width).max(Tick(1)),
                                height: Tick::from_tex_pt(gbox.height + gbox.depth).max(Tick(1)),
                                paint: display::Paint::BLACK,
                                provenance: provenance.clone(),
                            }),
                        ));
                        continue;
                    }
                    let Some(resource) = resource else { continue };
                    let m = gbox.matrix;
                    let k = BP_PER_PT;
                    self.images.push((
                        page,
                        display::Item::Image(display::Image {
                            x: Tick::from_tex_pt(left),
                            top: Tick::from_tex_pt(base - gbox.height),
                            width: Tick::from_tex_pt(gbox.width),
                            height: Tick::from_tex_pt(gbox.height + gbox.depth),
                            transform: [m[0] * k, -m[1] * k, m[2] * k, -m[3] * k, (left + m[4]) * k, (base - m[5]) * k],
                            resource: resource.clone(),
                            provenance: provenance.clone(),
                            clip: *clip,
                        }),
                    ));
                }
            }
        }
        for key in &b.labels {
            self.labels.push((key.clone(), page));
        }
    }

    /// Ships a column; a page's first column also carries the wide floats
    /// above both columns (`\@combinedblfloats`).
    fn push_column(&mut self, mut page: BuiltPage) {
        if self.twocolumn && self.pages.len() % 2 == 0 && !self.dbl_top.is_empty() {
            let page_no = self.pages.len() as u32 + 1;
            let mut lines = Vec::new();
            let mut y = 0.0;
            for f in std::mem::take(&mut self.dbl_top) {
                self.emit(f, y, page_no, &mut lines);
                y += self.boxes[f].height + self.fp.dblfloatsep;
            }
            lines.append(&mut page.lines);
            page.lines = lines;
        }
        self.pages.push(page);
    }

    /// A float column (`\@vtryfc`): `\vbox to colht` with `fil` glue, `off`
    /// below the text top.
    fn float_page(&mut self, floats: &[usize], colht: f64, fpsep: f64, off: f64) {
        let page = self.pages.len() as u32 + 1;
        let n = floats.len() as f64;
        let natural: f64 = floats.iter().map(|f| self.boxes[*f].height).sum::<f64>() + (n - 1.0) * fpsep;
        let left = colht - natural;
        let (first, between) = if left > 0.0 { (left / (2.0 * n), fpsep + left / n) } else { (0.0, fpsep) };
        let mut lines = Vec::new();
        let mut y = off + first;
        for &f in floats {
            self.emit(f, y, page, &mut lines);
            y += self.boxes[f].height + between;
        }
        self.push_column(BuiltPage { lines, overfull_by: 0.0 });
    }

    /// `\@colht` of the column about to start: the page's (`\textheight`
    /// less its wide floats), or the first page's below a `\twocolumn[...]`
    /// box.
    fn set_colht(&mut self) {
        self.colht = match self.first_colht {
            Some((n, h)) if self.pages.len() < n => h,
            _ => self.page_colht,
        };
    }

    /// `\@outputdblcol` after a page: `\@dblfloatplacement` and
    /// `\@startdblcolumn` (pages of wide floats while `\@tryfcolumn` makes
    /// them, then `\@addtodblcol` for every deferred float).
    fn start_dbl_page(&mut self) {
        let full = self.full_colht;
        self.page_colht = full;
        self.dbl_phase = true;
        while let Some((on_page, rest)) = self.try_fcolumn(0.5 * full, true, full, self.fp.dblfpsep) {
            self.deferred = rest;
            self.float_page(&on_page, full, self.fp.dblfpsep, 0.0);
            self.push_column(BuiltPage::default());
        }
        let mut toproom = 0.7 * full;
        let textmin = full - toproom;
        let mut topnum = 2;
        let mut colht = full;
        for f in std::mem::take(&mut self.deferred) {
            let fps = self.fps(f);
            let ht = self.boxes[f].height;
            let mut inserted = false;
            if fps & 2 != 0 {
                let n = flsetnum(topnum, fps);
                if n > 0 && (toproom > ht || (fps < 16 && toproom + textmin > ht)) && !has_type(self.boxes, &self.deferred, self.boxes[f].type_bit) && !self.wrong_width(f) {
                    let used = ht + if self.dbl_top.is_empty() { self.fp.dbltextfloatsep } else { self.fp.dblfloatsep };
                    toproom -= used;
                    colht -= used;
                    topnum = n - 1;
                    self.dbl_top.push(f);
                    inserted = true;
                }
            }
            if !inserted {
                self.deferred.push(f);
            }
        }
        self.page_colht = colht;
        self.dbl_phase = false;
    }

    /// `\@opcol` + `\@startcolumn`.
    fn start_column(&mut self) {
        loop {
            if self.twocolumn && self.pages.len() % 2 == 0 {
                self.start_dbl_page();
            }
            self.set_colht();
            self.col = Col::new(self.colht);
            match self.try_fcolumn(0.5 * self.colht, true, self.colht, self.fp.fpsep) {
                Some((on_page, rest)) => {
                    self.deferred = rest;
                    let (colht, sep, off) = (self.colht, self.fp.fpsep, self.dbl_off());
                    self.float_page(&on_page, colht, sep, off);
                }
                None => break,
            }
        }
        self.add_to_next_col();
    }
}

/// Glue set ratio of the column box `\vbox to vsize` holding `nodes` and
/// float material `extra` (natural height, stretch, shrink), as
/// `pagebuild`'s page builder: positive stretches, negative shrinks, 0
/// under `fil` glue.
fn column_glue_set(p: &PageParams, vsize: f64, nodes: &[N], list: &[VItem], boxes: &[FloatBox], extra: (f64, f64, f64)) -> f64 {
    let (mut total, mut depth, mut has_box, mut last_box) = (0.0f64, 0.0f64, false, false);
    let (mut stretch, mut shrink, mut fil) = (0.0f64, 0.0f64, false);
    for n in nodes {
        let (bx, glue) = match *n {
            N::FBox(f) => (Some((boxes[f].height, 0.0)), None),
            N::V(j) => match list[j] {
                VItem::Box { height, depth, .. } => (Some((height, depth)), None),
                VItem::Glue { width, stretch, shrink, fil } => (None, Some((width, stretch, shrink, fil))),
                VItem::Penalty(_) => (None, None),
            },
            N::Glue(w, st, sh) => (None, Some((w, st, sh, false))),
            _ => (None, None),
        };
        if let Some((h, d)) = bx {
            total = if has_box { total + depth + h } else { (p.topskip - h).max(0.0) + h };
            depth = d;
            has_box = true;
            last_box = true;
        }
        if let Some((w, st, sh, fl)) = glue {
            if has_box {
                total += depth + w;
                depth = 0.0;
                stretch += st;
                shrink += sh;
                fil |= fl;
                last_box = false;
            }
        }
    }
    let natural = total + if last_box { (depth - p.maxdepth).max(0.0) } else { 0.0 } + extra.0;
    let (stretch, shrink) = (stretch + extra.1, shrink + extra.2);
    let excess = vsize - natural;
    if excess > 0.0 {
        if fil || stretch <= 0.0 {
            0.0
        } else {
            excess / stretch
        }
    } else if excess < 0.0 && shrink > 0.0 {
        -(-excess / shrink).min(1.0)
    } else {
        0.0
    }
}

fn block_source(ctx: &Context, b: &BuiltBlock, items: impl Iterator<Item = usize>) -> Vec<Span> {
    items
        .filter_map(|i| b.recs.get(i).copied().flatten())
        .filter_map(|r| match &ctx.recs[r] {
            BoxRec::Text { clusters, .. } => clusters.first().map(|c| c.span),
            BoxRec::Math(m) => Some(ctx.maths[*m].span),
            BoxRec::Rule { span, .. } | BoxRec::Rules { span, .. } => Some(*span),
            BoxRec::Picture(p) => Some(p.span),
            BoxRec::Table(t) => Some(t.span),
            BoxRec::ColorBox(c) => Some(c.span),
            BoxRec::Graphic(g) => Some(g.span),
            BoxRec::Transform(t) => Some(t.span),
        })
        .collect()
}

/// Breaks the text into pages with the floats placed. `text_blocks` is the
/// number of body blocks (the blocks `list` was made from; footnote blocks
/// follow them) and `ins` the footnote insertions, charged against the
/// page goal (`\@colroom`) as TeX's page builder does and set below the
/// text and above bottom floats (`\@makecol`: `\@outputbox@appendfootnotes`
/// before `\@outputbox@attachfloats`). `first_colht` is the first page's
/// column count and `\@colht` below a `\twocolumn[...]` box. Returns the
/// pages, the image items per page number, the page of every float `\label`
/// and each page's footnote area.
#[allow(clippy::type_complexity)]
pub fn paginate(
    ctx: &mut Context,
    blocks: &mut Vec<BuiltBlock>,
    p: &PageParams,
    list: &[VItem],
    specs: &[FloatSpec],
    text_blocks: usize,
    ins: Option<&Insertions>,
    first_colht: Option<(usize, f64)>,
) -> (Vec<BuiltPage>, Vec<(u32, display::Item)>, Vec<(String, u32)>, Vec<Option<InsertArea>>) {
    // Marker positions, before caption blocks are appended.
    let vblocks: Vec<pagebuild::VBlock> = blocks.iter().map(|b| b.vertical.clone()).collect();
    let mut markers: Vec<(usize, usize)> = Vec::new();
    for (f, spec) in specs.iter().enumerate() {
        let at = spec.span;
        let mut after_block = None;
        for (bi, b) in blocks.iter().enumerate().take(text_blocks) {
            if let Some(first) = block_source(ctx, b, 0..b.items.len()).into_iter().find(|s| s.document == at.document) {
                if first.start < at.start {
                    after_block = Some(bi);
                }
            }
        }
        let index = match after_block {
            None => 0,
            Some(bi) if spec.hmode => {
                let b = &blocks[bi];
                let line = b.block.lines.lines.iter().position(|l| block_source(ctx, b, l.items.clone()).iter().any(|s| s.document == at.document && s.end > at.start));
                match line.and_then(|li| list.iter().position(|v| matches!(v, VItem::Box { payload, .. } if *payload == (bi, li)))) {
                    Some(i) => i + 1,
                    None => pagebuild::vlist(p, &vblocks[..=bi]).len(),
                }
            }
            Some(bi) => pagebuild::vlist(p, &vblocks[..=bi]).len(),
        };
        markers.push((index, f));
    }
    markers.sort();
    let fp = FloatParams::for_size(ctx.style.body_size_pt);
    let twocolumn = ctx.style.class_geometry.as_deref().is_some_and(|g| g.frame.columns.len() > 1);
    let boxes: Vec<FloatBox> = specs.iter().map(|s| build_box(ctx, blocks, s, &fp, twocolumn)).collect();
    let mut nodes: Vec<N> = Vec::with_capacity(list.len() + 4 * specs.len());
    let mut mi = 0;
    for i in 0..=list.len() {
        while mi < markers.len() && markers[mi].0 == i {
            nodes.push(N::Marker(markers[mi].1));
            mi += 1;
        }
        if i < list.len() {
            nodes.push(match list[i] {
                VItem::Box { .. } => N::V(i),
                VItem::Glue { width, stretch, shrink, .. } if !matches!(list[i], VItem::Glue { fil: true, .. }) => N::Glue(width, stretch, shrink),
                VItem::Glue { .. } => N::V(i),
                VItem::Penalty(pen) => N::Penalty(pen),
            });
        }
    }
    let s = ctx.style;
    let mut pl = Placer {
        boxes: &boxes,
        bits: specs.iter().map(|s| s.bits).collect(),
        fp,
        colht: first_colht.filter(|(n, _)| *n > 0).map_or(p.vsize, |(_, h)| h),
        full_colht: p.vsize,
        first_colht,
        parskip: Skip { n: s.parskip.natural, st: s.parskip.stretch, sh: s.parskip.shrink },
        text_x: s.text_x_pt,
        text_y: s.text_y_pt,
        pages: Vec::new(),
        images: Vec::new(),
        labels: Vec::new(),
        col: Col::new(first_colht.filter(|(n, _)| *n > 0).map_or(p.vsize, |(_, h)| h)),
        deferred: Vec::new(),
        twocolumn,
        dbl_phase: false,
        page_colht: p.vsize,
        dbl_top: Vec::new(),
    };
    let mut processed = vec![false; specs.len()];
    let box_of = |n: &N, nodes_list: &[VItem]| -> Option<(f64, f64)> {
        match n {
            N::V(i) => match nodes_list[*i] {
                VItem::Box { height, depth, .. } => Some((height, depth)),
                _ => None,
            },
            _ => None,
        }
    };
    let no_ins = Insertions::default();
    let insr = ins.unwrap_or(&no_ins);
    let mut held: Vec<PageIns> = Vec::new();
    let mut areas: Vec<Option<InsertArea>> = Vec::new();
    let mut start = 0usize;
    loop {
        while start < nodes.len() {
            match nodes[start] {
                N::V(i) if matches!(list[i], VItem::Box { .. }) => break,
                N::FBox(_) => break,
                N::Marker(f) if !processed[f] => break,
                _ => start += 1,
            }
        }
        if start >= nodes.len() {
            break;
        }
        let vsize = pl.col.colroom;
        let (mut total, mut stretch, mut shrink, mut fil, mut depth, mut has_box) = (0.0f64, 0.0f64, 0.0f64, false, 0.0f64, false);
        // Held-over notes are contributed ahead of the page's material.
        let mut is = InsertState::new(insr, vsize);
        for h in &held {
            is.append(h.list.clone(), h.height_plus_depth, None, total, depth, &mut stretch, &mut shrink);
        }
        let mut best_ins = is.last_ins;
        let mut lines_seen = 0usize;
        let mut best: Option<(usize, i64)> = None;
        let mut fired = None;
        let mut restart = false;
        let mut prev_box = false;
        let mut i = start;
        let cost = |total: f64, stretch: f64, shrink: f64, fil: bool, is: &InsertState, pi: i32| -> i64 {
            let goal = is.goal;
            let b = if total < goal {
                if fil {
                    0
                } else {
                    badness(goal - total, stretch)
                }
            } else if total - goal > shrink {
                AWFUL_BAD
            } else {
                badness(total - goal, shrink)
            };
            let c = if b < AWFUL_BAD {
                if pi <= EJECT_PENALTY {
                    i64::from(pi)
                } else if b < INF_BAD {
                    b + i64::from(pi) + is.penalties
                } else {
                    DEPLORABLE
                }
            } else {
                b
            };
            if is.penalties >= i64::from(INF_PENALTY) {
                AWFUL_BAD
            } else {
                c
            }
        };
        while i < nodes.len() {
            if let N::Marker(f) = nodes[i] {
                if !processed[f] {
                    processed[f] = true;
                    // `\@specialoutput`: `\@pageht` is the held page plus
                    // `\ht\footins + \skip\footins + \dp\footins`.
                    let mut pageht = if has_box { total + depth } else { 0.0 };
                    if is.started && is.height > 0.0 {
                        pageht += is.height + insr.skip.0;
                    }
                    let before = pl.col.colroom;
                    let here = pl.add_to_cur_col(f, pageht, !specs[f].hmode);
                    let mut ins = here.unwrap_or_default();
                    // `\@specialoutput` ends with `\addpenalty\interlinepenalty`.
                    let pos = if ins.len() == 6 { 5 } else { ins.len() };
                    ins.insert(pos, N::Penalty(0));
                    nodes.splice(i + 1..i + 1, ins);
                    if pl.col.colroom != before {
                        restart = true;
                        break;
                    }
                }
                i += 1;
                continue;
            }
            let (legal, pi) = match nodes[i] {
                N::Penalty(pen) => (pen < INF_PENALTY, pen),
                N::Glue(..) => (prev_box, 0),
                N::V(j) => (matches!(list[j], VItem::Glue { .. }) && prev_box, 0),
                _ => (false, 0),
            };
            if legal && has_box {
                let c = cost(total, stretch, shrink, fil, &is, pi);
                if best.is_none_or(|(_, lc)| c <= lc) {
                    best = Some((i, c));
                    best_ins = is.last_ins;
                }
                if c == AWFUL_BAD || pi <= EJECT_PENALTY {
                    fired = Some(best.map_or(i, |(bi, _)| bi));
                    break;
                }
            }
            let bx = match nodes[i] {
                N::FBox(f) => Some((boxes[f].height, 0.0)),
                ref n => box_of(n, list),
            };
            if let Some((h, d)) = bx {
                total += if has_box { depth + h } else { (p.topskip - h).max(0.0) + h };
                has_box = true;
                depth = d;
                if depth > p.maxdepth {
                    total += depth - p.maxdepth;
                    depth = p.maxdepth;
                }
                lines_seen += 1;
                prev_box = true;
                if total > is.goal + 1e-9 && lines_seen > 1 {
                    if let Some((bi, _)) = best {
                        fired = Some(bi);
                        break;
                    }
                }
                if let N::V(j) = nodes[i] {
                    if let VItem::Box { payload, .. } = list[j] {
                        for &n in insr.after.get(&payload).map(Vec::as_slice).unwrap_or(&[]) {
                            let note = insr.notes[n].clone();
                            let hd = pagebuild::natural_height_plus_depth(&note);
                            is.append(note, hd, Some(i), total, depth, &mut stretch, &mut shrink);
                        }
                    }
                }
            } else {
                match nodes[i] {
                    N::Glue(w, st, sh) => {
                        if has_box {
                            total += depth + w;
                            depth = 0.0;
                            stretch += st;
                            shrink += sh;
                        }
                    }
                    N::V(j) => {
                        if let VItem::Glue { width, stretch: st, shrink: sh, fil: fl } = list[j] {
                            if has_box {
                                total += depth + width;
                                depth = 0.0;
                                stretch += st;
                                fil |= fl;
                                shrink += sh;
                            }
                        }
                    }
                    _ => {}
                }
                prev_box = false;
            }
            i += 1;
        }
        if restart {
            continue;
        }
        // The document's end is a breakpoint like any other once notes are
        // on the page.
        if fired.is_none() && !is.page.is_empty() {
            if cost(total, stretch, shrink, fil, &is, EJECT_PENALTY) == AWFUL_BAD {
                if let Some((bi, _)) = best {
                    fired = Some(bi);
                }
            } else {
                best_ins = is.last_ins;
            }
        }
        let end = fired.unwrap_or(nodes.len());
        let ejected = matches!(nodes.get(end), Some(N::Penalty(pen)) if *pen <= EJECT_PENALTY);
        held.clear();
        let notes = pagebuild::settle_inserts(is, end, best_ins, insr.split_top_skip, &mut held);
        let page_no = pl.pages.len() as u32 + 1;
        let tops = std::mem::take(&mut pl.col.top);
        let bots = std::mem::take(&mut pl.col.bot);
        // `\@makecol`: the text (`\box255`), the top floats with `\floatsep`
        // and `\textfloatsep` (`\@cflt`) and the bottom floats (`\@cflb`) are
        // unboxed into `\@make@normalcolbox`'s `\vbox to\@colht`, whose glue
        // setting covers all of them: material taller than the column
        // shrinks; a short column stretches only under `\flushbottom` at an
        // ordinary break (as `pagebuild::break_pages`).
        let floats_glue = |fl: &[usize]| -> (f64, f64, f64) {
            if fl.is_empty() {
                return (0.0, 0.0, 0.0);
            }
            let k = fl.len() as f64 - 1.0;
            (
                fl.iter().map(|f| boxes[*f].height).sum::<f64>() + k * fp.floatsep.n + fp.textfloatsep.n,
                k * fp.floatsep.st + fp.textfloatsep.st,
                k * fp.floatsep.sh + fp.textfloatsep.sh,
            )
        };
        let (tg, bg) = (floats_glue(&tops), floats_glue(&bots));
        let ejected = matches!(nodes.get(end), Some(N::Penalty(pen)) if *pen <= EJECT_PENALTY);
        let set = match column_glue_set(p, pl.colht, &nodes[start..end], list, &boxes, (tg.0 + bg.0, tg.1 + bg.1, tg.2 + bg.2)) {
            g if g < 0.0 => g,
            g if p.flushbottom && fired.is_some() && !ejected => g,
            _ => 0.0,
        };
        let glue_adj = |st: f64, sh: f64| if set > 0.0 { set * st } else { set * sh };
        let floatsep = fp.floatsep.n + glue_adj(fp.floatsep.st, fp.floatsep.sh);
        let off = pl.dbl_off();
        let text_off = off
            + if tops.is_empty() {
                0.0
            } else {
                tops.iter().map(|f| boxes[*f].height).sum::<f64>() + (tops.len() as f64 - 1.0) * floatsep + fp.textfloatsep.n + glue_adj(fp.textfloatsep.st, fp.textfloatsep.sh)
            };
        let mut lines: Vec<Placed> = Vec::new();
        let mut y = off;
        for &f in &tops {
            pl.emit(f, y, page_no, &mut lines);
            y += boxes[f].height + floatsep;
        }
        // `floatsep`/`off` carry main's column glue setting (`glue_adj`);
        // the two branches below are #154's footnote-aware `\@makecol` and
        // the plain column it keeps for pages without notes.
        let bots_span: f64 = if bots.is_empty() { 0.0 } else { bots.iter().map(|f| boxes[*f].height).sum::<f64>() + (bots.len() as f64 - 1.0) * floatsep };
        let mut overfull_by = 0.0;
        if notes.iter().any(|l| !l.is_empty()) {
            // `\@makecol` with `\footins`: the body (here floats as boxes),
            // `\skip\footins`, `\footnoterule` and the notes, then the
            // bottom floats `\textfloatsep` below, in `\@colroom`.
            let mut body: Vec<VItem> = Vec::with_capacity(end - start);
            for n in &nodes[start..end] {
                match n {
                    N::FBox(f) => body.push(VItem::Box { height: boxes[*f].height, depth: 0.0, payload: (usize::MAX, *f) }),
                    N::V(j) => body.push(list[*j].clone()),
                    N::Glue(w, st, sh) => body.push(VItem::Glue { width: *w, stretch: *st, shrink: *sh, fil: false }),
                    N::Penalty(pen) => body.push(VItem::Penalty(*pen)),
                    N::Marker(_) => {}
                }
            }
            let vfil = ejected || fired.is_none();
            let colp = PageParams { vsize, ..*p };
            let (page, area) = pagebuild::make_column(&colp, &body, false, vfil, &notes, insr, !bots.is_empty());
            overfull_by = page.overfull_by;
            for l in page.lines {
                if l.payload.0 == usize::MAX {
                    pl.emit(l.payload.1, l.baseline - l.height + text_off, page_no, &mut lines);
                } else {
                    lines.push(Placed { baseline: l.baseline + text_off, ..l });
                }
            }
            if !bots.is_empty() {
                // Glue set to `\@colht` pushes them to the bottom; under
                // `\raggedbottom` without a `\vfil` they follow the notes.
                let natural = area.lines.last().map_or(vsize, |l| l.baseline + l.depth) + text_off + fp.textfloatsep.n;
                let mut y = if vfil || p.flushbottom { off + pl.colht - bots_span } else { natural };
                for &f in &bots {
                    pl.emit(f, y, page_no, &mut lines);
                    y += boxes[f].height + floatsep;
                }
            }
            areas.resize(pl.pages.len(), None);
            areas.push(Some(InsertArea {
                rule_top: area.rule_top + text_off,
                lines: area.lines.into_iter().map(|l| Placed { baseline: l.baseline + text_off, ..l }).collect(),
            }));
        } else {
            let (mut total, mut depth, mut has_box) = (0.0f64, 0.0f64, false);
            let mut last_text: Option<Placed> = None;
            for n in &nodes[start..end] {
                let bx = match n {
                    N::FBox(f) => Some((boxes[*f].height, 0.0, None, Some(*f))),
                    N::V(j) => match list[*j] {
                        VItem::Box { height, depth, payload } => Some((height, depth, Some(payload), None)),
                        VItem::Glue { width, stretch, shrink, .. } => {
                            if has_box {
                                total += depth + width + glue_adj(stretch, shrink);
                                depth = 0.0;
                            }
                            None
                        }
                        VItem::Penalty(_) => None,
                    },
                    N::Glue(w, st, sh) => {
                        if has_box {
                            total += depth + w + glue_adj(*st, *sh);
                            depth = 0.0;
                        }
                        None
                    }
                    _ => None,
                };
                if let Some((h, d, payload, float)) = bx {
                    let baseline = if has_box { total + depth + h } else { (p.topskip - h).max(0.0) + h };
                    total = baseline;
                    depth = d;
                    has_box = true;
                    if let Some(payload) = payload {
                        let placed = Placed { payload, baseline: baseline + text_off, height: h, depth: d };
                        last_text = Some(placed);
                        lines.push(placed);
                    }
                    if let Some(f) = float {
                        pl.emit(f, baseline - h + text_off, page_no, &mut lines);
                    }
                }
            }
            if let Some(last) = last_text {
                let bottom = last.baseline - text_off + (last.depth - p.maxdepth).max(0.0);
                if bottom > vsize + 1e-6 {
                    overfull_by = bottom - vsize;
                }
            }
            if !bots.is_empty() {
                let mut y = off + pl.colht - bots_span;
                for &f in &bots {
                    pl.emit(f, y, page_no, &mut lines);
                    y += boxes[f].height + floatsep;
                }
            }
        }
        pl.push_column(BuiltPage { lines, overfull_by });
        pl.col.mid.clear();
        start = end;
        pl.start_column();
        if fired.is_none() {
            break;
        }
    }
    // `\@doclearpage` with `\footins` not void: `\vbox{}` and the held notes
    // (with the floats already queued for the column) make a page of their
    // own before the deferred floats are flushed.
    while !held.is_empty() {
        let vsize = pl.col.colroom;
        let mut is = InsertState::new(insr, vsize);
        let (mut stretch, mut shrink) = (0.0, 0.0);
        for h in std::mem::take(&mut held) {
            is.append(h.list, h.height_plus_depth, None, p.topskip, 0.0, &mut stretch, &mut shrink);
        }
        let best_ins = is.last_ins;
        let notes = pagebuild::settle_inserts(is, 0, best_ins, insr.split_top_skip, &mut held);
        if !notes.iter().any(|l| !l.is_empty()) {
            break;
        }
        let page_no = pl.pages.len() as u32 + 1;
        let tops = std::mem::take(&mut pl.col.top);
        let bots = std::mem::take(&mut pl.col.bot);
        let text_off = if tops.is_empty() {
            0.0
        } else {
            tops.iter().map(|f| boxes[*f].height).sum::<f64>() + (tops.len() as f64 - 1.0) * fp.floatsep.n + fp.textfloatsep.n
        };
        let mut lines: Vec<Placed> = Vec::new();
        let mut y = 0.0;
        for &f in &tops {
            pl.emit(f, y, page_no, &mut lines);
            y += boxes[f].height + fp.floatsep.n;
        }
        let colp = PageParams { vsize, ..*p };
        let (_, area) = pagebuild::make_column(&colp, &[], true, true, &notes, insr, !bots.is_empty());
        if !bots.is_empty() {
            let span: f64 = bots.iter().map(|f| boxes[*f].height).sum::<f64>() + (bots.len() as f64 - 1.0) * fp.floatsep.n;
            let mut y = pl.colht - span;
            for &f in &bots {
                pl.emit(f, y, page_no, &mut lines);
                y += boxes[f].height + fp.floatsep.n;
            }
        }
        areas.resize(pl.pages.len(), None);
        areas.push(Some(InsertArea {
            rule_top: area.rule_top + text_off,
            lines: area.lines.into_iter().map(|l| Placed { baseline: l.baseline + text_off, ..l }).collect(),
        }));
        pl.pages.push(BuiltPage { lines, overfull_by: 0.0 });
        pl.col.mid.clear();
        pl.start_column();
    }
    // `\end{document}` -> `\clearpage` -> `\@doclearpage`: floats already
    // queued for the unstarted column go back to the deferred list, then
    // `\@makefcolumn` sets float columns. A two-column document then ends a
    // page whose first column is set (`\vbox{}\clearpage`: an empty second
    // column, whose `\@outputdblcol` places wide floats on the next page),
    // and on a fresh page sets the wide floats on pages of their own.
    let mut rest = std::mem::take(&mut pl.col.top);
    rest.append(&mut pl.col.bot);
    rest.append(&mut pl.deferred);
    pl.deferred = rest;
    let mut rounds = 0;
    while !pl.deferred.is_empty() || !pl.dbl_top.is_empty() {
        rounds += 1;
        let before = (pl.pages.len(), pl.deferred.len());
        loop {
            pl.set_colht();
            match pl.try_fcolumn(f64::NEG_INFINITY, false, pl.colht, pl.fp.fpsep) {
                Some((on_page, rest)) => {
                    pl.deferred = rest;
                    let (colht, sep, off) = (pl.colht, pl.fp.fpsep, pl.dbl_off());
                    pl.float_page(&on_page, colht, sep, off);
                    if pl.twocolumn && pl.pages.len() % 2 == 0 {
                        pl.start_dbl_page();
                    }
                }
                None => break,
            }
        }
        if pl.twocolumn {
            if pl.pages.len() % 2 == 0 {
                let mut wide = std::mem::take(&mut pl.dbl_top);
                wide.append(&mut pl.deferred);
                pl.deferred = wide;
                pl.page_colht = pl.full_colht;
                pl.dbl_phase = true;
                let (full, sep) = (pl.full_colht, pl.fp.dblfpsep);
                while let Some((on_page, rest)) = pl.try_fcolumn(f64::NEG_INFINITY, false, full, sep) {
                    pl.deferred = rest;
                    pl.float_page(&on_page, full, sep, 0.0);
                    pl.push_column(BuiltPage::default());
                }
                pl.dbl_phase = false;
            } else if !pl.deferred.is_empty() {
                pl.push_column(BuiltPage::default());
                pl.start_dbl_page();
                if rounds < 64 {
                    continue;
                }
            }
        }
        if pl.deferred.is_empty() || rounds >= 64 {
            if rounds >= 64 {
                break;
            }
            continue;
        }
        if (pl.pages.len(), pl.deferred.len()) == before {
            // No page takes the first float (taller than any column): set it
            // alone rather than lose it.
            let f = pl.deferred.remove(0);
            if pl.boxes[f].dbl {
                let (full, sep) = (pl.full_colht, pl.fp.dblfpsep);
                pl.float_page(&[f], full, sep, 0.0);
                pl.push_column(BuiltPage::default());
            } else {
                pl.set_colht();
                let (colht, sep, off) = (pl.colht, pl.fp.fpsep, pl.dbl_off());
                pl.float_page(&[f], colht, sep, off);
            }
        }
    }
    let _ = FloatKind::Figure;
    areas.resize(pl.pages.len(), None);
    (pl.pages, pl.images, pl.labels, areas)
}
