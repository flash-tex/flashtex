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

use std::rc::Rc;

use flashtex_compiler::Span;

use crate::adapter::{self, Item as AItem, ParaStyle};
use crate::display::{self, Diagnostic, ImageResource, Provenance, Tick};
use crate::floats::{Align, FloatKind};
use crate::graphics::{GraphicBox, BP_PER_PT};
use crate::pagebuild::{self, badness, BuiltPage, InsertArea, InsertState, Insertions, PageIns, PageParams, Placed, VBlock, VItem, AWFUL_BAD, DEPLORABLE, EJECT_PENALTY, INF_BAD, INF_PENALTY};

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
    pub parts: Vec<FloatPart>,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum FloatPart {
    /// `\centering`/`\raggedright`/`\raggedleft`: the line a run of
    /// graphics is set on follows it (the body's own paragraphs carry the
    /// declaration through the compiler instead).
    Align(Align),
    ParBreak,
    Graphic(PreparedGraphic),
    /// Body material as ordinary blocks, set by `Context::box_blocks`;
    /// `end_skip` is the `\endtrivlist` glue of a list the run closes
    /// (`adapter::list_end_skip`), which LaTeX contributes after it.
    Content { blocks: Vec<adapter::Block>, end_skip: f64 },
    /// The caption paragraph's items, `Figure~N: ` prefix included.
    Caption { items: Vec<AItem> },
}

#[derive(Debug, Clone)]
pub struct PreparedGraphic {
    pub gbox: GraphicBox,
    /// `None` when the file could not be read but its size was known from
    /// `width` and `height` (space is kept, nothing is painted), and
    /// whenever `placeholder` is set.
    pub resource: Option<Rc<ImageResource>>,
    /// What graphicx draws in place of the file under `draft`/`demo`.
    pub placeholder: Option<Placeholder>,
    pub span: Span,
}

/// The ink a graphic that was never read still puts on the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placeholder {
    /// `demo`: `\rule{..}{..}` fills the whole box.
    DemoRule,
    /// `draft`: `\Gin@setfile` sets `\hb@xt@\Gin@req@width{\vrule\hss\vbox
    /// to \Gin@req@height{\hrule..\vss\rlap{ \ttfamily <file>}\vss\hrule}
    /// \hss\vrule}` -- a frame of default-thickness rules around the
    /// reserved space. The `\rlap`ped file name inside it is not set yet;
    /// it is centred in the box and contributes no dimension.
    DraftFrame,
}

/// TeX's default `\vrule`/`\hrule` thickness (`\p@` / 2.5), in TeX points.
const RULE_PT: f64 = 0.4;

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
}

impl FloatParams {
    fn for_size(body_pt: f64) -> FloatParams {
        let s = |n, st, sh| Skip { n, st, sh };
        if body_pt >= 11.5 {
            FloatParams { floatsep: s(12.0, 2.0, 4.0), textfloatsep: s(20.0, 2.0, 4.0), intextsep: s(14.0, 4.0, 4.0), fpsep: 10.0, abovecaptionskip: 10.0 }
        } else {
            FloatParams { floatsep: s(12.0, 2.0, 2.0), textfloatsep: s(20.0, 2.0, 4.0), intextsep: s(12.0, 2.0, 2.0), fpsep: 8.0, abovecaptionskip: 10.0 }
        }
    }
}

enum Elem {
    /// A caption line: block/line in `blocks`, baseline from the box top.
    Line { block: usize, line: usize, baseline: f64, height: f64, depth: f64 },
    Image { x: f64, baseline: f64, gbox: GraphicBox, resource: Option<Rc<ImageResource>>, placeholder: Option<Placeholder>, provenance: Provenance },
}

struct FloatBox {
    height: f64,
    elems: Vec<Elem>,
    type_bit: u32,
    labels: Vec<String>,
}

/// One entry of the float box's vertical list: either a block set through
/// the ordinary block path (indexed into the page's `blocks`), or a line of
/// graphics set side by side.
enum BoxEntry {
    Block(usize),
    Images { graphics: Vec<PreparedGraphic>, centered: bool },
}

/// Sets the float's box (see the module docs): `\@xfloat` opens a `\vbox` of
/// `\hsize\columnwidth` under `\@parboxrestore`, the body contributes to its
/// vertical list, and `\@makecaption` adds the caption. The list is the
/// page builder's own (`pagebuild::vlist`) laid out at natural size, so
/// interline glue, `\parskip`, `\topsep` and a display's skips inside a
/// float follow the same rules as on the page.
fn build_box(ctx: &mut Context, blocks: &mut Vec<BuiltBlock>, spec: &FloatSpec, fp: &FloatParams, p: &PageParams) -> FloatBox {
    let tw = ctx.style.text_width_pt;
    let mut entries: Vec<BoxEntry> = Vec::new();
    let mut vblocks: Vec<VBlock> = Vec::new();
    let mut centered = false;
    let mut pending: Vec<PreparedGraphic> = Vec::new();
    // `\@setminipage`: `\addvspace` is suppressed until the box's first
    // paragraph has begun (a graphics line and a caption are paragraphs too).
    let mut minipage = true;
    // A run of graphics with no paragraph break between them is one line.
    macro_rules! flush {
        () => {
            if !pending.is_empty() {
                minipage = false;
                let h = pending.iter().map(|g| g.gbox.height).fold(0.0, f64::max);
                let d = pending.iter().map(|g| g.gbox.depth).fold(0.0, f64::max);
                vblocks.push(super::plain_vblock(vec![(h, d)]));
                entries.push(BoxEntry::Images { graphics: std::mem::take(&mut pending), centered });
            }
        };
    }
    for part in &spec.parts {
        match part {
            FloatPart::Align(a) => centered = matches!(a, Align::Center),
            FloatPart::Graphic(g) => pending.push(g.clone()),
            FloatPart::ParBreak => flush!(),
            FloatPart::Content { blocks: body, end_skip } => {
                flush!();
                let range = ctx.box_blocks(body, blocks, spec.span, minipage);
                minipage = minipage && range.is_empty();
                if *end_skip != 0.0 {
                    if let Some(last) = range.clone().last().and_then(|i| blocks.get_mut(i)) {
                        let s = last.vertical.space_after.unwrap_or((0.0, 0.0, 0.0));
                        last.vertical.space_after = Some((s.0 + end_skip, s.1, s.2));
                    }
                }
                for i in range {
                    vblocks.push(blocks[i].vertical.clone());
                    entries.push(BoxEntry::Block(i));
                }
            }
            FloatPart::Caption { items } => {
                flush!();
                minipage = false;
                let Some(mut block) = ctx.paragraph_block(items, false, true, false, ParaStyle::Plain, None, None) else { continue };
                let lines = &block.block.lines.lines;
                // `\@caption` runs `\@parboxrestore` before `\@makecaption`, so
                // `\centering` does not reach a caption set as a paragraph:
                // only the one-line `\hbox to\hsize{\hfil...\hfil}` is centred.
                let fits = lines.len() == 1 && lines[0].natural_width <= tw + 1e-6;
                if fits {
                    if let Some(b) = ctx.paragraph_block(items, false, true, false, ParaStyle::Center, None, None) {
                        block = b;
                    }
                }
                // `\@makecaption`: `\vskip\abovecaptionskip`, the caption,
                // `\vskip\belowcaptionskip` (0pt in the standard classes).
                // `\parskip` is 0 inside the float's `\@parboxrestore`.
                block.vertical.space_before = Some((fp.abovecaptionskip, 0.0, 0.0));
                block.vertical.parskip = None;
                vblocks.push(block.vertical.clone());
                entries.push(BoxEntry::Block(blocks.len()));
                blocks.push(block);
            }
        }
    }
    flush!();
    let list = pagebuild::vlist(p, &vblocks);
    let (placed, mut height) = pagebuild::natural_layout(p, &list, false);
    // `\vbox`: the depth of the last box is the box's depth unless glue
    // follows it, and the float is placed by its height *and* depth.
    if matches!(list.iter().rev().find(|i| !matches!(i, VItem::Penalty(_))), Some(VItem::Box { .. })) {
        height += placed.last().map_or(0.0, |l| l.depth);
    }
    let mut elems = Vec::new();
    for line in &placed {
        let (entry, li) = line.payload;
        match &entries[entry] {
            BoxEntry::Block(bi) => elems.push(Elem::Line { block: *bi, line: li, baseline: line.baseline, height: line.height, depth: line.depth }),
            BoxEntry::Images { graphics, centered } => {
                let w: f64 = graphics.iter().map(|g| g.gbox.width).sum();
                let mut x = if *centered { ((tw - w) / 2.0).max(0.0) } else { 0.0 };
                for g in graphics {
                    elems.push(Elem::Image { x, baseline: line.baseline, gbox: g.gbox, resource: g.resource.clone(), placeholder: g.placeholder, provenance: Provenance::Source(ctx.source(g.span)) });
                    x += g.gbox.width;
                }
            }
        }
    }
    if height > ctx.style.text_height_pt {
        ctx.diagnostics.push(Diagnostic::warning(
            "float_too_large",
            format!("{} {} is {:.2}pt taller than the text area", spec.kind.name(), spec.number, height - ctx.style.text_height_pt),
            vec![ctx.source(spec.span)],
        ));
        height = ctx.style.text_height_pt;
    }
    FloatBox { height, elems, type_bit: spec.kind.type_bit(), labels: spec.labels.clone() }
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
    parskip: Skip,
    text_x: f64,
    text_y: f64,
    pages: Vec<BuiltPage>,
    images: Vec<(u32, display::Item)>,
    labels: Vec<(String, u32)>,
    col: Col,
    deferred: Vec<usize>,
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
                if colnum > 0 && !has_type(self.boxes, &self.deferred, ty) {
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
                    if colnum > 0 && !has_type(self.boxes, &self.deferred, self.boxes[f].type_bit) {
                        inserted = self.add_to_top_or_bot(f, req, colnum);
                    }
                }
            }
            if !inserted {
                self.deferred.push(f);
            }
        }
    }

    /// `\@tryfcolumn` over the deferred list: `(floats on the page, rest)`.
    fn try_fcolumn(&self, fpmin: f64, test_p: bool) -> Option<(Vec<usize>, Vec<usize>)> {
        let list = &self.deferred;
        let mut failed: Vec<usize> = Vec::new();
        for (idx, &f) in list.iter().enumerate() {
            let b = &self.boxes[f];
            if has_type(self.boxes, &failed, b.type_bit) || (test_p && self.fps(f) & 8 == 0) || b.height > self.colht {
                failed.push(f);
                continue;
            }
            let mut succeed = vec![f];
            let mut flfail: Vec<usize> = Vec::new();
            let mut h = b.height;
            for &g in &list[idx + 1..] {
                let gb = &self.boxes[g];
                let blocked = has_type(self.boxes, &failed, gb.type_bit) || has_type(self.boxes, &flfail, gb.type_bit);
                if blocked || (test_p && self.fps(g) & 8 == 0) || h + gb.height + self.fp.fpsep > self.colht {
                    flfail.push(g);
                } else {
                    h += gb.height + self.fp.fpsep;
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
                Elem::Image { x, baseline, gbox, resource, placeholder, provenance } => {
                    let left = self.text_x + x;
                    let base = self.text_y + top + baseline;
                    // `draft`/`demo` never read a file; what they paint is
                    // rules. Both are laid out in the box's own axes, so a
                    // rotated box is left as reserved space only.
                    if let Some(kind) = placeholder {
                        let m = gbox.matrix;
                        if m[1] == 0.0 && m[2] == 0.0 && m[0] > 0.0 && m[3] > 0.0 {
                            let (t, w, h) = (base - gbox.height, gbox.width, gbox.height + gbox.depth);
                            let bars: &[(f64, f64, f64, f64)] = match kind {
                                Placeholder::DemoRule => &[(left, t, w, h)],
                                // `\hrule`s across the top and bottom,
                                // `\vrule`s up the sides, which the two
                                // `\hss`es pull inside the requested width.
                                Placeholder::DraftFrame => &[
                                    (left, t, w, RULE_PT),
                                    (left, t + h - RULE_PT, w, RULE_PT),
                                    (left, t, RULE_PT, h),
                                    (left + w - RULE_PT, t, RULE_PT, h),
                                ],
                            };
                            for (x, y, w, h) in bars.iter().copied().filter(|(_, _, w, h)| *w > 0.0 && *h > 0.0) {
                                self.images.push((
                                    page,
                                    display::Item::Rule(display::Rule {
                                        x: Tick::from_tex_pt(x),
                                        top: Tick::from_tex_pt(y),
                                        width: Tick::from_tex_pt(w),
                                        height: Tick::from_tex_pt(h),
                                        paint: display::Paint::BLACK,
                                        provenance: provenance.clone(),
                                    }),
                                ));
                            }
                        }
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
                        }),
                    ));
                }
            }
        }
        for key in &b.labels {
            self.labels.push((key.clone(), page));
        }
    }

    fn float_page(&mut self, floats: &[usize]) {
        let page = self.pages.len() as u32 + 1;
        let n = floats.len() as f64;
        let natural: f64 = floats.iter().map(|f| self.boxes[*f].height).sum::<f64>() + (n - 1.0) * self.fp.fpsep;
        let left = self.colht - natural;
        let (first, between) = if left > 0.0 { (left / (2.0 * n), self.fp.fpsep + left / n) } else { (0.0, self.fp.fpsep) };
        let mut lines = Vec::new();
        let mut y = first;
        for &f in floats {
            self.emit(f, y, page, &mut lines);
            y += self.boxes[f].height + between;
        }
        self.pages.push(BuiltPage { lines, overfull_by: 0.0 });
    }

    /// A column opened by `\LT@output` (longtable.sty 513-516): it runs
    /// `\@makecol\@outputpage \global\vsize\@colroom \copy\LT@head`, never
    /// `\@opcol`/`\@startcolumn`, so no float page is tried and no deferred
    /// float is added — the whole column is the table's, and what is
    /// deferred stays deferred until the table ends.
    fn start_longtable_column(&mut self) {
        self.col = Col::new(self.colht);
    }

    /// `\@opcol` + `\@startcolumn`.
    fn start_column(&mut self) {
        loop {
            self.col = Col::new(self.colht);
            match self.try_fcolumn(0.5 * self.colht, true) {
                Some((on_page, rest)) => {
                    self.deferred = rest;
                    self.float_page(&on_page);
                }
                None => break,
            }
        }
        self.add_to_next_col();
    }
}

/// The column's float material for `\@makecol` (`\@cflt`/`\@cflb`).
fn column_floats(boxes: &[FloatBox], tops: &[usize], bots: &[usize], fp: &FloatParams) -> pagebuild::ColumnFloats {
    let sk = |s: Skip| (s.n, s.st, s.sh);
    pagebuild::ColumnFloats {
        tops: tops.iter().map(|f| boxes[*f].height).collect(),
        bots: bots.iter().map(|f| boxes[*f].height).collect(),
        floatsep: sk(fp.floatsep),
        textfloatsep: sk(fp.textfloatsep),
    }
}

fn block_source(ctx: &Context, b: &BuiltBlock, items: impl Iterator<Item = usize>) -> Vec<Span> {
    items
        .filter_map(|i| b.recs.get(i).copied().flatten())
        .filter_map(|r| match &ctx.recs[r] {
            BoxRec::Text { clusters, .. } => clusters.first().map(|c| c.span),
            BoxRec::Math(m) => Some(ctx.maths[*m].span),
            BoxRec::Rule { span, .. } => Some(*span),
            BoxRec::Picture(p) => Some(p.span),
            BoxRec::Table(t) => Some(t.span),
            BoxRec::ColorBox(c) => Some(c.span),
            BoxRec::Underline(u) => Some(u.span),
        })
        .collect()
}

/// Breaks the text into pages with the floats placed. Returns the pages,
/// the image items per page number, the page of every float `\label` and
/// each page's footnote area.
///
/// `regions` are the document's longtable regions (`pagebuild::Region`
/// resolved against `list`): inside one the column goal loses `\ht\LT@foot`
/// and `\pageshrink` is zero (longtable.sty 223, 229), and a break inside
/// one ends the page with `\LT@foot` and opens the next with `\LT@head`
/// (`\LT@output`, 487-517).
///
/// `text_blocks` is the number of body blocks (the blocks `list` was made
/// from; footnote blocks follow them) and `ins` the footnote insertions,
/// charged against the page goal (`\@colroom`) as TeX's page builder does
/// and set below the text and above bottom floats (`\@makecol`:
/// `\@outputbox@appendfootnotes` before `\@outputbox@attachfloats`).
///
/// A page can carry both at once. `\pagegoal` takes the two charges
/// together -- `\LT@start` subtracts `\ht\LT@foot` (longtable.sty 229) and
/// the insertion machinery subtracts `\skip\footins` and the notes (section
/// 1009) -- so a continuation foot and a note compete for the same page.
/// And because `\LT@output` puts `\copy\LT@head`/`\copy\LT@foot` inside
/// `\box\@cclv` *before* calling `\@makecol` (487-517), the head and foot
/// are part of the column body that the notes are set under and that the
/// floats are wrapped around.
#[allow(clippy::type_complexity)]
pub fn paginate(
    ctx: &mut Context,
    blocks: &mut Vec<BuiltBlock>,
    p: &PageParams,
    list: &[VItem],
    specs: &[FloatSpec],
    regions: &[pagebuild::ResolvedRegion],
    text_blocks: usize,
    ins: Option<&Insertions>,
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
    // longtable.sty 489-491: `\LT@output` answers a float or marginpar
    // inside the table with `floats~ and~ marginpars~ not~ allowed~ in~ a~
    // longtable`. The float still has to go somewhere, so it is placed as
    // if the table were not there and the error is reported.
    {
        let inside = pagebuild::Regions::new(regions);
        for &(index, f) in &markers {
            if inside.contains(index) {
                let src = vec![ctx.source(specs[f].span)];
                ctx.diagnostics.push(Diagnostic::warning(
                    "table_limitation",
                    "floats and marginpars are not allowed in a longtable (longtable.sty \\LT@output); this one is placed as if the table were not there".to_string(),
                    src,
                ));
            }
        }
    }
    let fp = FloatParams::for_size(ctx.style.body_size_pt);
    let boxes: Vec<FloatBox> = specs.iter().map(|s| build_box(ctx, blocks, s, &fp, p)).collect();
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
        colht: p.vsize,
        parskip: Skip { n: s.parskip.natural, st: s.parskip.stretch, sh: s.parskip.shrink },
        text_x: s.text_x_pt,
        text_y: s.text_y_pt,
        pages: Vec::new(),
        images: Vec::new(),
        labels: Vec::new(),
        col: Col::new(p.vsize),
        deferred: Vec::new(),
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
    let regions = pagebuild::Regions::new(regions);
    // The list index a node reports to the regions: a `V` node is at its
    // own, anything the float machinery spliced in belongs to the last one
    // seen (`\@specialoutput` runs between two items of the main list).
    let list_index = |n: &N, cur: usize| -> usize {
        match n {
            N::V(j) => *j,
            _ => cur,
        }
    };
    // `\copy\LT@head\nobreak` at the top of a continuation page.
    let mut pending_head: Option<pagebuild::Placed3> = None;
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
        // The continuation head is on the page before any of its material.
        let head = pending_head;
        if let Some((h, d, _)) = head {
            total = (p.topskip - h).max(0.0) + h;
            depth = d;
            has_box = true;
        }
        let mut cur_li = list_index(&nodes[start], 0);
        // §987 `freeze_page_specs` with longtable.sty 230: `\maxdepth` is
        // zero for a page a longtable opens (see `pagebuild::page_max_depth`).
        let maxdepth = if head.is_none() && regions.starts_at(cur_li) { 0.0 } else { p.maxdepth };
        // Held-over notes are contributed ahead of the page's material, and
        // above whatever `\ht\LT@foot` the region already reserves: the head
        // is already in `total`, so they see it as `\pagetotal`. `held` is
        // only cleared once the page is settled, because the float machinery
        // can `restart` this page and the notes must survive that.
        let mut is = InsertState::new(insr, vsize);
        let head_reserve = regions.reserved(cur_li);
        for h in &held {
            is.append(h.list.clone(), h.height_plus_depth, None, total, depth, head_reserve, &mut stretch, &mut shrink);
        }
        let mut best_ins = is.last_ins;
        let mut lines_seen = 0usize;
        let mut best: Option<(usize, i64)> = None;
        let mut fired = None;
        let mut restart = false;
        let mut prev_box = false;
        let mut i = start;
        // `\pagegoal` carries both charges at once: the insertions have
        // already taken `\skip\footins` and the notes off `is.goal`
        // (section 1009), and `reserve` is the `\ht\LT@foot` a longtable
        // region holds back on top of that (longtable.sty 229).
        #[allow(clippy::too_many_arguments)]
        let cost = |total: f64, stretch: f64, shrink: f64, fil: bool, is: &InsertState, pi: i32, reserve: f64| -> i64 {
            let goal = is.goal - reserve;
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
            cur_li = list_index(&nodes[i], cur_li);
            let (legal, pi) = match nodes[i] {
                N::Penalty(pen) => (pen < INF_PENALTY, pen),
                N::Glue(..) => (prev_box, 0),
                N::V(j) => (matches!(list[j], VItem::Glue { .. }) && prev_box, 0),
                _ => (false, 0),
            };
            // What `\ht\LT@foot` the region holds back at this index.
            let reserve = regions.reserved(cur_li);
            if legal && has_box {
                let c = cost(total, stretch, shrink, fil, &is, pi, reserve);
                if best.is_none_or(|(_, lc)| c <= lc) {
                    best = Some((i, c));
                    best_ins = is.last_ins;
                }
                if c == AWFUL_BAD || pi <= EJECT_PENALTY {
                    fired = Some(best.map_or(i, |(bi, _)| bi));
                    break;
                }
            }
            // longtable.sty 223: `\LT@start` ends with
            // `\ifdim\pageshrink>\z@\pageshrink\z@\fi`.
            if regions.starts_at(cur_li) {
                shrink = 0.0;
            }
            let bx = match nodes[i] {
                N::FBox(f) => Some((boxes[f].height, 0.0)),
                ref n => box_of(n, list),
            };
            if let Some((h, d)) = bx {
                total += if has_box { depth + h } else { (p.topskip - h).max(0.0) + h };
                has_box = true;
                depth = d;
                if depth > maxdepth {
                    total += depth - maxdepth;
                    depth = maxdepth;
                }
                lines_seen += 1;
                prev_box = true;
                // §1005: the page is hopeless only once the excess passes
                // what the glue can shrink. In a body of `\parskip 0pt
                // plus 1pt` that shrink is zero, but `\skip\footins`'s
                // `minus 2pt` (and an `h` float's `\intextsep`) buys the
                // column a line TeX keeps. Inside a longtable `\pageshrink`
                // is zeroed above, so there the allowance is nil again.
                if total > is.goal - reserve + shrink + 1e-9 && lines_seen > 1 {
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
                            is.append(note, hd, Some(i), total, depth, reserve, &mut stretch, &mut shrink);
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
            if cost(total, stretch, shrink, fil, &is, EJECT_PENALTY, regions.reserved(cur_li)) == AWFUL_BAD {
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
        let text_off = if tops.is_empty() {
            0.0
        } else {
            tops.iter().map(|f| boxes[*f].height).sum::<f64>() + (tops.len() as f64 - 1.0) * fp.floatsep.n + fp.textfloatsep.n
        };
        let mut lines: Vec<Placed> = Vec::new();
        // `\LT@output`: a page broken inside the table ends with `\LT@foot`
        // and the next one opens with `\LT@head`. Both boxes go *inside*
        // `\box\@cclv` (longtable.sty 487-517 builds it before calling
        // `\@makecol`), so the break has to be settled here, before the
        // column is assembled and the notes are set under it.
        pending_head = None;
        let mut region_break = false;
        let mut region_foot: Option<pagebuild::Placed3> = None;
        if fired.is_some() {
            let next_li = nodes[end..].iter().find_map(|n| match n {
                N::V(j) if matches!(list[*j], VItem::Box { .. }) => Some(*j),
                _ => None,
            });
            if let Some((foot, next_head)) = next_li.and_then(|li| regions.at_break(li)) {
                region_break = true;
                region_foot = foot;
                pending_head = next_head;
            }
        }
        let mut overfull_by = 0.0;
        if notes.iter().any(|l| !l.is_empty()) {
            // `\@makecol` with `\footins` and floats, over a `\box\@cclv`
            // the longtable output routine may already have built. The notes
            // attach to `\box\@cclv` first (the default
            // `build/column/outputbox` plug is `footnotes-floats-legacy`)
            // and `\@combinefloats` wraps the floats around that, so the
            // column reads: top floats, `\textfloatsep`, [`\LT@head`], the
            // body, [`\LT@foot`], `\skip\footins`, the `\footnoterule`, the
            // notes, `\textfloatsep`, the bottom floats. `\@cflt`/`\@cflb`
            // are plain `\vbox`es and only `\@make@normalcolbox` packs
            // `\vbox to\@colht`, so one glue set ratio covers the body's
            // glue, `\skip\footins` and both float separations together:
            // `\@colht`, not `\@colroom`, is the size to reach.
            let mut body: Vec<VItem> = Vec::with_capacity(end - start + 2);
            if let Some((h, d, payload)) = head {
                body.push(VItem::Box { height: h, depth: d, payload });
            }
            for n in &nodes[start..end] {
                match n {
                    N::FBox(f) => body.push(VItem::Box { height: boxes[*f].height, depth: 0.0, payload: (usize::MAX, *f) }),
                    N::V(j) => body.push(list[*j].clone()),
                    N::Glue(w, st, sh) => body.push(VItem::Glue { width: *w, stretch: *st, shrink: *sh, fil: false }),
                    N::Penalty(pen) => body.push(VItem::Penalty(*pen)),
                    N::Marker(_) => {}
                }
            }
            if let Some((h, d, payload)) = region_foot {
                body.push(VItem::Box { height: h, depth: d, payload });
            }
            // `\LT@output` builds `\box\@cclv` as `\vbox{\unvbox\@cclv
            // \copy\LT@foot \vss}` and only then calls `\@makecol`: that
            // `\vss` is inside the body, ahead of `\skip\footins`, so it
            // takes the column's slack and pushes the note block to the foot
            // of the page, leaving every finite glue at its natural size.
            let vfil = region_break || ejected || fired.is_none();
            let colp = PageParams { vsize: pl.colht, maxdepth, ..*p };
            let cf = column_floats(&boxes, &tops, &bots, &fp);
            let (page, area, placed) = pagebuild::make_column(&colp, &body, false, vfil, &notes, insr, &cf);
            overfull_by = page.overfull_by;
            for (&f, &y) in tops.iter().zip(&placed.tops) {
                pl.emit(f, y, page_no, &mut lines);
            }
            for l in page.lines {
                if l.payload.0 == usize::MAX {
                    pl.emit(l.payload.1, l.baseline - l.height, page_no, &mut lines);
                } else {
                    lines.push(l);
                }
            }
            for (&f, &y) in bots.iter().zip(&placed.bots) {
                pl.emit(f, y, page_no, &mut lines);
            }
            areas.resize(pl.pages.len(), None);
            areas.push(Some(area));
        } else {
            let mut y = 0.0;
            for &f in &tops {
                pl.emit(f, y, page_no, &mut lines);
                y += boxes[f].height + fp.floatsep.n;
            }
            let (mut total, mut depth, mut has_box) = (0.0f64, 0.0f64, false);
            let mut last_text: Option<Placed> = None;
            // `\copy\LT@head\nobreak` opens a page continuing a longtable.
            if let Some((h, d, payload)) = head {
                let baseline = (p.topskip - h).max(0.0) + h;
                total = baseline;
                depth = d;
                has_box = true;
                let placed = Placed { payload, baseline: baseline + text_off, height: h, depth: d };
                last_text = Some(placed);
                lines.push(placed);
            }
            for n in &nodes[start..end] {
                let bx = match n {
                    N::FBox(f) => Some((boxes[*f].height, 0.0, None, Some(*f))),
                    N::V(j) => match list[*j] {
                        VItem::Box { height, depth, payload } => Some((height, depth, Some(payload), None)),
                        VItem::Glue { width, .. } => {
                            if has_box {
                                total += depth + width;
                                depth = 0.0;
                            }
                            None
                        }
                        VItem::Penalty(_) => None,
                    },
                    N::Glue(w, ..) => {
                        if has_box {
                            total += depth + w;
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
            // `\copy\LT@foot` closes a page broken inside the table.
            if let Some((h, d, payload)) = region_foot {
                let baseline = total + depth + h;
                total = baseline;
                depth = d;
                let placed = Placed { payload, baseline: baseline + text_off, height: h, depth: d };
                last_text = Some(placed);
                lines.push(placed);
            }
            let _ = (total, depth);
            if let Some(last) = last_text {
                let bottom = last.baseline - text_off + (last.depth - maxdepth).max(0.0);
                if bottom > vsize + 1e-6 {
                    overfull_by = bottom - vsize;
                }
            }
            if !bots.is_empty() {
                let span: f64 = bots.iter().map(|f| boxes[*f].height).sum::<f64>() + (bots.len() as f64 - 1.0) * fp.floatsep.n;
                let mut y = pl.colht - span;
                for &f in &bots {
                    pl.emit(f, y, page_no, &mut lines);
                    y += boxes[f].height + fp.floatsep.n;
                }
            }
        }
        pl.pages.push(BuiltPage { lines, overfull_by });
        pl.col.mid.clear();
        start = end;
        if region_break {
            pl.start_longtable_column();
        } else {
            pl.start_column();
        }
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
            is.append(h.list, h.height_plus_depth, None, p.topskip, 0.0, 0.0, &mut stretch, &mut shrink);
        }
        let best_ins = is.last_ins;
        let notes = pagebuild::settle_inserts(is, 0, best_ins, insr.split_top_skip, &mut held);
        if !notes.iter().any(|l| !l.is_empty()) {
            break;
        }
        let page_no = pl.pages.len() as u32 + 1;
        let tops = std::mem::take(&mut pl.col.top);
        let bots = std::mem::take(&mut pl.col.bot);
        let mut lines: Vec<Placed> = Vec::new();
        let colp = PageParams { vsize: pl.colht, ..*p };
        let cf = column_floats(&boxes, &tops, &bots, &fp);
        let (_, area, placed) = pagebuild::make_column(&colp, &[], true, true, &notes, insr, &cf);
        for (&f, &y) in tops.iter().zip(&placed.tops) {
            pl.emit(f, y, page_no, &mut lines);
        }
        for (&f, &y) in bots.iter().zip(&placed.bots) {
            pl.emit(f, y, page_no, &mut lines);
        }
        areas.resize(pl.pages.len(), None);
        areas.push(Some(area));
        pl.pages.push(BuiltPage { lines, overfull_by: 0.0 });
        pl.col.mid.clear();
        pl.start_column();
    }
    // `\end{document}` -> `\clearpage` -> `\@doclearpage`: floats already
    // queued for the unstarted column go back to the deferred list, then
    // `\@makefcolumn` sets every remaining float on float pages.
    let mut rest = std::mem::take(&mut pl.col.top);
    rest.append(&mut pl.col.bot);
    rest.append(&mut pl.deferred);
    pl.deferred = rest;
    while !pl.deferred.is_empty() {
        match pl.try_fcolumn(f64::NEG_INFINITY, false) {
            Some((on_page, rest)) => {
                pl.deferred = rest;
                pl.float_page(&on_page);
            }
            None => {
                let f = pl.deferred.remove(0);
                pl.float_page(&[f]);
            }
        }
    }
    let _ = FloatKind::Figure;
    areas.resize(pl.pages.len(), None);
    (pl.pages, pl.images, pl.labels, areas)
}
