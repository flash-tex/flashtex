//! beamer blocks and columns (issue #944, Tier 3), for the **default**
//! theme (and, Tier 4, the rounded blocks of the Madrid theme:
//! [`Context::rounded_block_begin`]); the numbers are
//! `flashtex_class_geometry::beamer`'s. The
//! in-flow `figure`/`table` and their captions need nothing here: the
//! compiler makes them `center` paragraphs (with beamer's 9pt `\topsep`,
//! `Stylesheet::trivlist_topsep`) and the adapter sets the caption line.
//!
//! # Blocks
//!
//! The default inner theme's `block begin`/`block end`
//! (`beamerinnerthemedefault.sty` 390-405) with no background colour is,
//! read off `\showlists` (`crates/class-geometry/src/beamer.rs`,
//! [`spec::block`]):
//!
//! ```text
//! \vskip\medskipamount                     6pt plus 2pt minus 2pt
//! <interline glue>                         \baselineskip - prev depth - h
//! \hbox(h+d){ \large title }               no strut: glyph extents
//! <interline glue>                         (\lineskip 1pt in practice)
//! \hbox{\vbox{ \vskip-.25ex \vbox{} <body ...> }}
//! \vskip\smallskipamount                   3pt plus 1pt minus 1pt
//! ```
//!
//! The body's blocks stay **flat** in the frame's vertical list (the
//! theme paints no box around them, and flat blocks keep their footnotes,
//! labels and page glue): the body box's own top -- `\vskip-.25ex` then a
//! `\vbox{}` that resets `\prevdepth` -- is an empty block, and the glue
//! that TeX computes between the title box and the body *box* (against
//! the whole body's height) is computed here once the body is built and
//! put before that empty block. What flat cannot express is a body box
//! whose depth differs from its last line's depth (a body ending in glue),
//! which shifts what follows by that depth; no corpus deck does that.
//!
//! # Columns
//!
//! `\begin{columns}` (`beamerbaseframecomponents.sty` 212-291) is one
//! `\hbox to\textwidth` on the vertical list, holding a paper-wide box with
//! `\hfill` before, between and after the column `minipage`s
//! ([`spec::column_origins`]); each column is a `\vtop`/`\vcenter`/`\vbox`
//! of its body ([`spec::column_box`]). Here every column body is built in a
//! sub-[`Context`] whose measure is the column's width (`\@iiiminipage`
//! sets `\hsize`, `\textwidth` and `\columnwidth` to it, under
//! `\@parboxrestore`), laid out at natural size, and the row becomes one
//! line whose only box is a [`BoxRec::Table`] of pieces -- the same record
//! a `tabular` paints its cells with -- placed at their column's x and
//! their line's y on the row's baseline.
//!
//! Measured (pdflatex TeX Live 2026, `fixtures/real-world/beamer-blocks-
//! columns`): p3 `[T]` columns at x = 18.90 / 190.87bp, first baselines
//! 100.357 (both); p6 `[c]` columns inside a block: "narrow" at (18.90,
//! 151.23), "wide," at (160.25, 144.76).

use flashtex_class_geometry::beamer::{self as spec, ColumnAlign, ColumnsBox};
use flashtex_class_geometry::Sp;
use flashtex_compiler::color::{ColorSpace, DeviceColor};
use flashtex_compiler::parser::{BeamerBlockKind, BeamerColumnAlign, BeamerColumnsOptions};
use flashtex_compiler::Span;
use flashtex_paragraph_layout as pl;

use crate::adapter::{Block, Item as AItem, ParaStyle, TextStyle};
use crate::pagebuild::{self, DepthAfter, VBlock};
use crate::style::frame_pt;

use super::{absorb, empty_block, page_params, plain_vblock, positioned_block, BoxRec, BuiltBlock, Context, Diagnostic, ParaState, TablePiece, TableRec, MATH_SENTINEL};

/// The colour a block title is set in (`block title` = `structure`,
/// `block title alerted` = `alerted text`, `block title example` =
/// `example text`; `beamercolorthemedefault.sty` 19-20, 123-125).
pub fn block_title_color(kind: BeamerBlockKind) -> DeviceColor {
    let (r, g, b) = match kind {
        BeamerBlockKind::Plain => spec::STRUCTURE_RGB,
        BeamerBlockKind::Alert => spec::ALERT_RGB,
        BeamerBlockKind::Example => spec::EXAMPLE_RGB,
    };
    let bn = |v: f64| (v * 1e9).round() as u32;
    DeviceColor::from_billionths(ColorSpace::Rgb, &[bn(r), bn(g), bn(b)]).expect("rgb in range")
}

/// A block whose body is being collected: where its body box starts.
#[derive(Debug, Clone, Copy)]
pub struct OpenBlock {
    /// Built-block index of the body box's `\vbox{}` (its `space_before`
    /// is patched with the interline glue once the body is known).
    pub body_at: usize,
    /// Index into `Context::rounded_blocks` under the rounded inner theme.
    pub rounded: Option<usize>,
}

/// A block set as a `beamerboxesrounded` (`beamerbaseboxes.sty` 36-252
/// through `blocks` `[rounded]`, `beamerbaseauxtemplates.sty` 765-796):
/// what the page chrome needs to paint its two rectangles (the rounded
/// corners and the shadow are not modelled). Measured (pdflatex,
/// `\showoutput`, Madrid, a two-line block): `\vbox(48.19664)` =
/// `4bp + head(8.33331 + 1.5) − 1 + 6 − 0.5 + body(2 + lines + dp + 0.5)
/// + 4bp`; the head fill runs from 3bp above the head box's top to 2pt
/// below its baseline, the lower fill from the body box's top to 3bp
/// below its baseline, a 2pt gradient between them (cut at the head's
/// `+2pt` here).
#[derive(Debug, Clone)]
pub struct RoundedBlockRec {
    /// Built-block index of the title line (the head box).
    pub title_at: usize,
    /// `\bmb@temp` of the head: `max(depth, 1.5pt)` — how far the head
    /// box's baseline sits under the title baseline.
    pub head_raise: f64,
    /// Built-block index of the body's last block, once `\end{block}` came.
    pub last_at: Option<usize>,
    pub title_bg: spec::Rgb,
    pub body_bg: spec::Rgb,
}

/// TeX's interline glue before a box of `height` (§679): `\baselineskip -
/// prev_depth - height`, or `\lineskip` when that is under
/// `\lineskiplimit`; nothing after `\nointerlineskip`/a rule
/// (`prev_depth` `None`).
fn interline(p: &pagebuild::PageParams, prev_depth: Option<f64>, height: f64) -> f64 {
    match prev_depth {
        None => 0.0,
        Some(d) => {
            let g = p.baselineskip - d - height;
            if g < p.lineskiplimit {
                p.lineskip
            } else {
                g
            }
        }
    }
}

/// `\prevdepth` after the blocks so far: the last line's depth, what a
/// package appended, or `None` for `\ignoredepth` (a rule, a box start).
fn prev_depth(blocks: &[BuiltBlock]) -> Option<f64> {
    for b in blocks.iter().rev() {
        let v = &b.vertical;
        if v.no_interline_after {
            return None;
        }
        match v.depth_after {
            DepthAfter::Fixed(d) => return Some(d),
            DepthAfter::Unchanged => continue,
            DepthAfter::LastLine => return v.lines.last().map(|l| l.1),
        }
    }
    None
}

/// The `(height, depth)` TeX gives a `\vbox` of these blocks at natural
/// size: the last box's depth is the box's depth unless glue follows it,
/// in which case it counts toward the height. Also the first box's height
/// (a `\vtop`'s height).
fn vbox_extent(p: &pagebuild::PageParams, blocks: &[BuiltBlock]) -> (f64, f64, f64) {
    let vb: Vec<VBlock> = blocks.iter().map(|b| b.vertical.clone()).collect();
    let list = pagebuild::vlist(p, &vb);
    let (placed, total) = pagebuild::natural_layout(p, &list, false);
    let ends_in_box = matches!(list.iter().rev().find(|i| !matches!(i, pagebuild::VItem::Penalty(_))), Some(pagebuild::VItem::Box { .. }));
    let depth = if ends_in_box { placed.last().map_or(0.0, |l| l.depth) } else { 0.0 };
    let first_height = placed.first().map_or(0.0, |l| l.height);
    (total, depth, first_height)
}

/// Adds `pt` (natural, stretch, shrink) to a block's `space_before`.
fn add_before(v: &mut VBlock, skip: (f64, f64, f64)) {
    if skip == (0.0, 0.0, 0.0) {
        return;
    }
    v.space_before = Some(match v.space_before {
        Some((n, s, k)) => (n + skip.0, s + skip.1, k + skip.2),
        None => skip,
    });
}

/// The `\addvspace` a list closed before an edge leaves (`addvspace`,
/// only its excess over the previous block's trailing skip, as
/// `\@xaddvskip` keeps the larger) plus any `\vspace`: the glue that
/// precedes the edge's own `\vskip`.
fn edge_glue(blocks: &[BuiltBlock], addvspace: f64, flex: (f64, f64), vspace: f64) -> (f64, f64, f64) {
    let prev_after = blocks.last().and_then(|b| b.vertical.space_after).map_or(0.0, |s| s.0);
    let mut out = (vspace, 0.0, 0.0);
    if addvspace > prev_after {
        out.0 += addvspace - prev_after;
        if prev_after <= 0.0 {
            out.1 += flex.0;
            out.2 += flex.1;
        }
    }
    out
}

impl<'a> Context<'a> {
    /// `\begin{block}{title}`: the `\medskipamount`, the title line(s) at
    /// `\large` in the block's colour, `\raggedright`, then the body box's
    /// top (`\vskip-.25ex` and an empty `\vbox{}`). `addvspace`/`vspace`
    /// are the glue of a list the block follows (see [`edge_glue`]).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn beamer_block_begin(&mut self, blocks: &mut Vec<BuiltBlock>, kind: BeamerBlockKind, title: &[AItem], span: Span, addvspace: f64, flex: (f64, f64), vspace: f64) -> OpenBlock {
        if let Some(rounded) = self.beamer_theme().blocks {
            return self.rounded_block_begin(blocks, kind, title, span, addvspace, flex, vspace, &rounded);
        }
        let p = page_params(self.style);
        let b = spec::block();
        let mut before = edge_glue(blocks, addvspace, flex, vspace);
        before.0 += frame_pt(b.before.natural);
        before.1 += frame_pt(b.before.stretch);
        before.2 += frame_pt(b.before.shrink);
        let (size, bs) = (frame_pt(spec::BLOCK_TITLE.size), frame_pt(spec::BLOCK_TITLE.baselineskip));
        let style = TextStyle { color: Some(block_title_color(kind)), ..TextStyle::default() };
        let width = self.style.text_width_pt;
        if let Some(mut t) = self.beamer_line(title, size, style, ParaStyle::FlushLeft, width, 0.0, bs, span) {
            // The title box's height is its first line's (all but the last
            // depth for a longer title); the glue before it is computed
            // here against that, the lines inside keep `\large`'s leading.
            let (h, _, _) = vbox_extent(&p, std::slice::from_ref(&t));
            let d_prev = prev_depth(blocks);
            let glue = interline(&p, d_prev, h);
            t.vertical.no_interline_first = true;
            t.vertical.baselineskip = Some(bs);
            t.vertical.penalty_before = None;
            t.vertical.parskip = None;
            add_before(&mut t.vertical, (before.0 + glue, before.1, before.2));
            t.vertical.penalty_after = Some(pagebuild::INF_PENALTY);
            blocks.push(t);
        } else {
            // An empty title: the box is `\hbox(0+0)` (the `\leavevmode`
            // line), still under the skip and its interline glue.
            let mut e = plain_vblock(vec![(0.0, 0.0)]);
            e.no_interline_first = true;
            let glue = interline(&p, prev_depth(blocks), 0.0);
            add_before(&mut e, (before.0 + glue, before.1, before.2));
            e.penalty_after = Some(pagebuild::INF_PENALTY);
            blocks.push(empty_block(e));
        }
        // The body box: `\vskip-.25ex` then `\vbox{}`, whose `\prevdepth`
        // (0) the first paragraph's interline glue reads. The glue between
        // the title box and this box is added at `\end{block}`.
        let mut top = plain_vblock(vec![(0.0, 0.0)]);
        top.no_interline_first = true;
        top.space_before = Some((frame_pt(b.body_top), 0.0, 0.0));
        top.penalty_before = Some(pagebuild::INF_PENALTY);
        let body_at = blocks.len();
        blocks.push(empty_block(top));
        OpenBlock { body_at, rounded: None }
    }

    /// `block begin` `[rounded]` (`beamerbaseauxtemplates.sty` 765-771):
    /// `\par\vskip\medskipamount` then a `beamerboxesrounded` whose head
    /// is the `\large` title (`\raggedright`, `block title` colours: white
    /// on the theme's title bg) and whose lower minipage holds the body.
    /// The box (`beamerbaseboxes.sty`): `\vskip4bp`, the head `\hbox`
    /// (the title box raised `max(dp, 1.5pt)`, depth 0), `\vskip-1pt`, the
    /// 6pt transition strip, `\vskip-0.5pt`, the body minipage (`\vskip2pt`
    /// then the body, its first line without interline glue, raised `dp +
    /// 0.5pt`), `\vskip4bp minus 2bp` (`shadow=true`); as one box it takes
    /// `\lineskip` glue before it. Measured (Madrid, `\showoutput`): title
    /// head `\hbox(9.83331+0)` for an 8.33331pt title, body `\hbox(25.83333)`
    /// for two 13.6pt lines ending in a 2.12917pt depth.
    #[allow(clippy::too_many_arguments)]
    fn rounded_block_begin(&mut self, blocks: &mut Vec<BuiltBlock>, kind: BeamerBlockKind, title: &[AItem], span: Span, addvspace: f64, flex: (f64, f64), vspace: f64, rounded: &spec::RoundedBlocks) -> OpenBlock {
        let p = page_params(self.style);
        let b = spec::block();
        let bp = |v: f64| v * 72.27 / 72.0;
        let (title_bg, body_bg) = match kind {
            BeamerBlockKind::Plain => (rounded.title_bg, rounded.body_bg),
            BeamerBlockKind::Alert => (rounded.alert_title_bg, rounded.alert_body_bg),
            BeamerBlockKind::Example => (rounded.example_title_bg, rounded.example_body_bg),
        };
        let mut before = edge_glue(blocks, addvspace, flex, vspace);
        before.0 += frame_pt(b.before.natural);
        before.1 += frame_pt(b.before.stretch);
        before.2 += frame_pt(b.before.shrink);
        // The rounded box is taller than `\baselineskip`: `\lineskip`
        // before it (nothing after a rule or a box start).
        before.0 += interline(&p, prev_depth(blocks), 1000.0);
        let (size, bs) = (frame_pt(spec::BLOCK_TITLE.size), frame_pt(spec::BLOCK_TITLE.baselineskip));
        let fg = super::beamer::rgb_color(rounded.title_fg);
        let style = TextStyle { color: Some(fg), ..TextStyle::default() };
        let width = self.style.text_width_pt;
        let title: Vec<AItem> = title
            .iter()
            .cloned()
            .map(|item| match item {
                AItem::Word(mut w) => {
                    for seg in &mut w.segments {
                        seg.style.color = Some(fg);
                    }
                    AItem::Word(w)
                }
                other => other,
            })
            .collect();
        let mut head_raise = 1.5;
        let title_at = blocks.len();
        match self.beamer_line(&title, size, style, ParaStyle::FlushLeft, width, 0.0, bs, span) {
            Some(mut t) => {
                // `\vskip4bp` is folded into the first line's height; the
                // last line's depth becomes the head box's raise.
                if let Some(first) = t.vertical.lines.first_mut() {
                    first.0 += bp(4.0);
                }
                if let Some(last) = t.vertical.lines.last_mut() {
                    head_raise = last.1.max(1.5);
                    last.1 = head_raise;
                }
                t.vertical.no_interline_first = true;
                t.vertical.baselineskip = Some(bs);
                t.vertical.penalty_before = None;
                t.vertical.parskip = None;
                add_before(&mut t.vertical, before);
                t.vertical.penalty_after = Some(pagebuild::INF_PENALTY);
                blocks.push(t);
            }
            None => {
                // An empty head: `\hbox{}` of height 1.5pt, no transition.
                let mut e = plain_vblock(vec![(bp(4.0) + 1.5, 0.0)]);
                e.no_interline_first = true;
                add_before(&mut e, before);
                e.penalty_after = Some(pagebuild::INF_PENALTY);
                blocks.push(empty_block(e));
                head_raise = 0.0;
            }
        }
        // `\vskip-1pt`, the 6pt transition, `\vskip-0.5pt`, the minipage's
        // `\vskip2pt`: 6.5pt from the head box's bottom to the body's first
        // line, which carries no interline glue (`rounded_block_end`).
        let mut top = plain_vblock(vec![(0.0, 0.0)]);
        top.no_interline_first = true;
        top.space_before = Some((6.5, 0.0, 0.0));
        top.penalty_before = Some(pagebuild::INF_PENALTY);
        let body_at = blocks.len();
        blocks.push(empty_block(top));
        self.rounded_blocks.push(RoundedBlockRec { title_at, head_raise, last_at: None, title_bg, body_bg });
        OpenBlock { body_at, rounded: Some(self.rounded_blocks.len() - 1) }
    }

    /// `\end{block}`: the interline glue between the title box and the
    /// body box (TeX sees the body as one box of its natural height), then
    /// the `\smallskipamount` after it, on top of the `\@endparenv` skip of
    /// a list the body ends with.
    pub(super) fn beamer_block_end(&mut self, blocks: &mut Vec<BuiltBlock>, open: OpenBlock, addvspace: f64, flex: (f64, f64), vspace: f64) {
        if let Some(rec) = open.rounded {
            // The body's first line has no interline glue (a fresh
            // `\vbox`); after its last line: the raise `0.5pt`, `\vskip4bp
            // minus 2bp`, then `\vskip\smallskipamount`.
            if let Some(first) = blocks.iter_mut().skip(open.body_at + 1).find(|b| !b.vertical.lines.is_empty()) {
                first.vertical.no_interline_first = true;
                first.vertical.parskip = None;
            }
            let b = spec::block();
            let mut after = edge_glue(blocks, addvspace, flex, vspace);
            after.0 += 0.5 + 4.0 * 72.27 / 72.0 + frame_pt(b.after.natural);
            after.1 += frame_pt(b.after.stretch);
            after.2 += 2.0 * 72.27 / 72.0 + frame_pt(b.after.shrink);
            if let Some(last) = blocks.last_mut() {
                let v = &mut last.vertical;
                v.space_after = Some(match v.space_after {
                    Some((n, s, k)) => (n + after.0, s + after.1, k + after.2),
                    None => after,
                });
            }
            let last_at = (open.body_at..blocks.len()).rev().find(|&i| !blocks[i].vertical.lines.is_empty()).unwrap_or(open.body_at);
            if let Some(r) = self.rounded_blocks.get_mut(rec) {
                r.last_at = Some(last_at);
            }
            return;
        }
        let p = page_params(self.style);
        let b = spec::block();
        if open.body_at < blocks.len() {
            let (h, _, _) = vbox_extent(&p, &blocks[open.body_at..]);
            let d_title = prev_depth(&blocks[..open.body_at]);
            let glue = interline(&p, d_title, h);
            add_before(&mut blocks[open.body_at].vertical, (glue, 0.0, 0.0));
        }
        let mut after = edge_glue(blocks, addvspace, flex, vspace);
        after.0 += frame_pt(b.after.natural);
        after.1 += frame_pt(b.after.stretch);
        after.2 += frame_pt(b.after.shrink);
        if let Some(last) = blocks.last_mut() {
            let v = &mut last.vertical;
            v.space_after = Some(match v.space_after {
                Some((n, s, k)) => (n + after.0, s + after.1, k + after.2),
                None => after,
            });
        }
    }

    /// A `columns` row: `columns` are `(width as written, alignment
    /// override, body blocks)`. Returns nothing when no column resolved
    /// (every width unreadable), after a diagnostic.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn beamer_columns(&mut self, blocks: &mut Vec<BuiltBlock>, options: &BeamerColumnsOptions, columns: &[(String, Option<BeamerColumnAlign>, &[Block])], span: Span, addvspace: f64, flex: (f64, f64), vspace: f64) {
        let p = page_params(self.style);
        let s = self.style;
        let text_width = s.text_width_pt;
        let tp = self.text_params(TextStyle::default(), s.body_size_pt);
        let env = crate::graphics::LengthEnv { text_width, line_width: text_width, text_height: s.text_height_pt, paper_width: s.page_width_pt, paper_height: s.page_height_pt, em: tp.quad, ex: tp.x_height };
        let sp = |pt: f64| Sp((pt * 65536.0).round() as i64);
        let mut widths: Vec<Sp> = Vec::new();
        let mut kept: Vec<(ColumnAlign, &[Block])> = Vec::new();
        for (width, align, body) in columns {
            match crate::graphics::parse_dimen(width, &env) {
                Some(w) => {
                    widths.push(sp(w));
                    let align = match align.unwrap_or(options.align) {
                        BeamerColumnAlign::Center => ColumnAlign::Center,
                        BeamerColumnAlign::Top => ColumnAlign::Top,
                        BeamerColumnAlign::TopBaseline => ColumnAlign::TopBaseline,
                        BeamerColumnAlign::Bottom => ColumnAlign::Bottom,
                    };
                    kept.push((align, body));
                }
                None => {
                    let src = vec![self.source(span)];
                    self.emit(None, Diagnostic::error("beamer_column_width", format!("\\column width '{width}' is not a length this renderer reads; the column is omitted"), src));
                }
            }
        }
        if kept.is_empty() {
            return;
        }
        // Where each column starts, from the text's left edge.
        let margins = beamer_margins(s);
        let kind = match (&options.total_width, options.only_text_width) {
            (Some(w), _) => ColumnsBox::Fixed(sp(crate::graphics::parse_dimen(w, &env).unwrap_or(text_width))),
            (None, true) => ColumnsBox::Fixed(sp(text_width)),
            (None, false) => ColumnsBox::PaperWide,
        };
        let origins = spec::column_origins(kind, &widths, sp(text_width), sp(margins.0), sp(margins.1));
        // Every column body at its own measure, in a sub-context.
        let mut pieces: Vec<TablePiece> = Vec::new();
        let (mut row_h, mut row_d) = (0.0f64, 0.0f64);
        for (i, (align, body)) in kept.iter().enumerate() {
            let mut col_style = s.clone();
            col_style.text_width_pt = frame_pt(widths[i]);
            let mut sub = Context::with_texts(self.fonts, &col_style, self.paths, self.texts);
            sub.set_sources(self.sources);
            if let Some((o, c)) = self.images {
                sub.set_images(o, c);
            }
            sub.set_math_colors(self.math_colors.clone());
            // The column is a `minipage`: its `\footnote`s are
            // `\@mpfootnotetext`'s, set at the column's foot.
            sub.minipage_notes = true;
            let mut sub_blocks: Vec<BuiltBlock> = Vec::new();
            sub.beamer_body(body, &mut sub_blocks, span, *align == ColumnAlign::TopBaseline);
            // `\endminipage`: `\par\unskip` drops the glue the body ends
            // with (a block's `\smallskipamount`); the box then ends in
            // its last line and that line's depth is the box's.
            if let Some(last) = sub_blocks.last_mut() {
                last.vertical.space_after = None;
            }
            // `\ifvoid\@mpfootins\else \vskip\skip\@mpfootins \footnoterule
            // \unvbox\@mpfootins \fi` (latex.ltx `\endminipage`): the
            // notes close the box, so its depth is the last note's.
            if !sub.notes.is_empty() {
                let notes: Vec<usize> = (0..sub.notes.len()).collect();
                sub.minipage_foot(&mut sub_blocks, &notes, frame_pt(widths[i]), span);
                sub.notes.clear();
                sub.note_anchors.clear();
            }
            let (height, last_depth, first_height) = vbox_extent(&p, &sub_blocks);
            let (h, d) = spec::column_box(*align, sp(height + last_depth), sp(first_height), sp(last_depth));
            let (h, d) = (frame_pt(h), frame_pt(d));
            row_h = row_h.max(h);
            row_d = row_d.max(d);
            // The column's lines, placed from its box top; a piece per
            // block, at the block's first baseline relative to the row's.
            let vb: Vec<VBlock> = sub_blocks.iter().map(|b| b.vertical.clone()).collect();
            let list = pagebuild::vlist(&p, &vb);
            let (placed, _) = pagebuild::natural_layout(&p, &list, false);
            let x = frame_pt(origins[i]);
            let start = blocks.len();
            let mut scratch: Vec<BuiltBlock> = Vec::new();
            absorb(self, sub, sub_blocks, &mut scratch);
            let _ = start;
            for (bi, block) in scratch.into_iter().enumerate() {
                let Some(first) = placed.iter().find(|l| l.payload.0 == bi) else { continue };
                pieces.push(TablePiece { x, baseline: first.baseline - h, block });
            }
        }
        // The row: one line holding the pieces' box, `\hbox to\textwidth`
        // starting at the text edge (`\hskip-\beamer@leftmargin` and the
        // paper-wide box inside it are the pieces' own x).
        let rec = TableRec { pieces, rules: Vec::new(), fills: Vec::new(), span, hidden: false, unpainted: false };
        self.recs.push(BoxRec::Table(std::rc::Rc::new(rec)));
        let rec_at = self.recs.len() - 1;
        let run = pl::GlyphRun { font: MATH_SENTINEL, size: s.body_size_pt, glyphs: Vec::new(), width: text_width, height: row_h, depth: row_d, source: span.start..span.end };
        let mut vertical = plain_vblock(vec![(row_h, row_d)]);
        // `\par` before the box: the glue a list left, any `\vspace`; the
        // interline glue is the page builder's (`\baselineskip` against the
        // row's height, `\lineskip` when it does not fit).
        add_before(&mut vertical, edge_glue(blocks, addvspace, flex, vspace));
        blocks.push(positioned_block(vec![(run, rec_at, 0.0)], row_h, row_d, text_width, vertical));
    }

    /// The vertical material of a column's `minipage`: the body blocks set
    /// through the ordinary builders at this context's measure, blocks and
    /// nested `columns` included. `\@parboxrestore` is in force
    /// (`Context::parbox`). Unlike a float body, `\@setminipage` suppresses
    /// nothing here: `\beamer@columncom` opens every column with
    /// `\leavevmode`, whose `\everypar` clears `\if@minipage` at once. That
    /// `\leavevmode` also means a column whose body starts in vertical mode
    /// (a block, a list, a rule) begins with an empty line -- the paragraph
    /// `\leavevmode` started and the body's `\par` ended. `top_baseline` is
    /// a `[T]` column: the empty line is always there (`\vskip-1ex` ends
    /// the paragraph), the `-1ex` follows and `\nointerlineskip` leaves
    /// the content's first box with no interline glue.
    pub(super) fn beamer_body(&mut self, body: &[Block], out: &mut Vec<BuiltBlock>, span: Span, top_baseline: bool) {
        let quad = self.text_params(TextStyle::default(), self.style.body_size_pt).quad;
        let mut st = ParaState { after_heading: false, env_vmode: false, env_skips: None, closed_env: None, outer_env_skips: Vec::new() };
        let outer = std::mem::replace(&mut self.parbox, true);
        let starts_in_vmode = !matches!(body.first(), Some(Block::Paragraph { .. } | Block::Picture { .. }));
        if top_baseline || starts_in_vmode {
            let mut line = plain_vblock(vec![(0.0, 0.0)]);
            // `\nointerlineskip`: `\prevdepth` is ignored for the next box.
            line.no_interline_after = top_baseline;
            out.push(empty_block(line));
        }
        let first_content = out.len();
        let mut minipage = false;
        let mut open_blocks: Vec<OpenBlock> = Vec::new();
        let mut i = 0;
        while i < body.len() {
            let block = &body[i];
            i += 1;
            match block {
                Block::Paragraph { .. } if minipage => {
                    let mut opened = block.clone();
                    if let Block::Paragraph { addvspace_before, addvspace_flex, vspace_flex, env_open, vspace_before, list, .. } = &mut opened {
                        *addvspace_before = 0.0;
                        *addvspace_flex = (0.0, 0.0);
                        *vspace_flex = (0.0, 0.0);
                        *env_open = None;
                        if list.is_some() {
                            *vspace_before = 0.0;
                        }
                    }
                    let at = out.len();
                    self.build_paragraph(out, &opened, &mut st, None, 0, quad);
                    if let Some(b) = out.get_mut(at) {
                        b.vertical.parskip = None;
                        b.vertical.space_before = None;
                    }
                    minipage = out.len() == at;
                }
                Block::Paragraph { .. } => self.build_paragraph(out, block, &mut st, None, 0, quad),
                Block::Rule { span, vspace_before, .. } => {
                    let mut b = self.rule_block(*span);
                    add_before(&mut b.vertical, (*vspace_before, 0.0, 0.0));
                    out.push(b);
                    minipage = false;
                }
                Block::Picture { document, picture, centered, indent, list, vspace_before, .. } => {
                    let mut b = self.picture_block(*document, picture, *centered, *indent, list.as_ref());
                    add_before(&mut b.vertical, (*vspace_before, 0.0, 0.0));
                    out.push(b);
                    minipage = false;
                }
                Block::BeamerBlockBegin { kind, title, span, addvspace_before, addvspace_flex, vspace_before } => {
                    let (add, flex) = if minipage { (0.0, (0.0, 0.0)) } else { (*addvspace_before, *addvspace_flex) };
                    let open = self.beamer_block_begin(out, *kind, title, *span, add, flex, *vspace_before);
                    open_blocks.push(open);
                    minipage = false;
                }
                Block::BeamerBlockEnd { addvspace_before, addvspace_flex, vspace_before, .. } => {
                    if let Some(open) = open_blocks.pop() {
                        self.beamer_block_end(out, open, *addvspace_before, *addvspace_flex, *vspace_before);
                    }
                }
                Block::ColumnsBegin { options, span, addvspace_before, addvspace_flex, vspace_before } => {
                    let end = columns_end(body, i);
                    let columns = split_columns(&body[i..end]);
                    let (add, flex) = if minipage { (0.0, (0.0, 0.0)) } else { (*addvspace_before, *addvspace_flex) };
                    self.beamer_columns(out, options, &columns, *span, add, flex, *vspace_before);
                    i = (end + 1).min(body.len());
                    minipage = false;
                }
                Block::Column { .. } | Block::ColumnsEnd { .. } => {}
                other => {
                    let what = match other {
                        Block::Heading { .. } => "a sectioning command",
                        Block::LongTable { .. } => "longtable",
                        Block::FrameBegin { .. } | Block::FrameEnd { .. } => "a beamer frame",
                        Block::BeamerTitle { .. } => "\\titlepage",
                        _ => "page-level material",
                    };
                    let source = vec![self.source(span)];
                    self.emit(None, Diagnostic::warning("beamer_column_unsupported", format!("{what} cannot be set inside a beamer column; it is omitted"), source));
                }
            }
        }
        for open in open_blocks.into_iter().rev() {
            self.beamer_block_end(out, open, 0.0, (0.0, 0.0), 0.0);
        }
        if top_baseline {
            if let Some(b) = out.get_mut(first_content) {
                // `\vskip-1ex` before whatever the content starts with (a
                // paragraph's `\parskip` is 0 under `\@parboxrestore`).
                add_before(&mut b.vertical, (-frame_pt(spec::SANS_BODY_EX), 0.0, 0.0));
            }
        }
        self.parbox = outer;
    }
}

/// `\beamer@leftmargin`/`\beamer@rightmargin` (1cm each in the default
/// theme): the text block's distance from the paper edges.
fn beamer_margins(s: &crate::style::Stylesheet) -> (f64, f64) {
    let left = s.text_x_pt;
    let right = s.page_width_pt - s.text_x_pt - s.text_width_pt;
    (left, right)
}

/// The index of the [`Block::ColumnsEnd`] matching a [`Block::ColumnsBegin`]
/// whose body starts at `from` (nesting-aware); `body.len()` when unclosed.
pub(super) fn columns_end(body: &[Block], from: usize) -> usize {
    let mut depth = 0usize;
    for (k, b) in body.iter().enumerate().skip(from) {
        match b {
            Block::ColumnsBegin { .. } => depth += 1,
            Block::ColumnsEnd { .. } if depth == 0 => return k,
            Block::ColumnsEnd { .. } => depth -= 1,
            _ => {}
        }
    }
    body.len()
}

/// The columns of a row's body (the blocks between `\begin{columns}` and
/// its `\end`): `(width, alignment override, blocks)` per `\column`
/// marker at the top level. Material before the first marker is outside
/// any column (beamer sets it in the row's `\hbox`); it is dropped with
/// no glue.
pub(super) fn split_columns(body: &[Block]) -> Vec<(String, Option<BeamerColumnAlign>, &[Block])> {
    let mut out: Vec<(String, Option<BeamerColumnAlign>, usize)> = Vec::new();
    let mut depth = 0usize;
    for (k, b) in body.iter().enumerate() {
        match b {
            Block::ColumnsBegin { .. } => depth += 1,
            Block::ColumnsEnd { .. } => depth = depth.saturating_sub(1),
            Block::Column { width, align, .. } if depth == 0 => out.push((width.clone(), *align, k + 1)),
            _ => {}
        }
    }
    let mut columns = Vec::with_capacity(out.len());
    for (n, (width, align, start)) in out.iter().enumerate() {
        let end = out.get(n + 1).map_or(body.len(), |(_, _, s)| s - 1);
        columns.push((width.clone(), *align, &body[*start..end]));
    }
    columns
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_colours_are_the_default_colour_theme() {
        assert_eq!(block_title_color(BeamerBlockKind::Plain).components(), vec![0.2, 0.2, 0.7]);
        assert_eq!(block_title_color(BeamerBlockKind::Alert).components(), vec![1.0, 0.0, 0.0]);
        assert_eq!(block_title_color(BeamerBlockKind::Example).components(), vec![0.0, 0.5, 0.0]);
    }

    #[test]
    fn interline_glue_follows_tex() {
        let p = pagebuild::PageParams { vsize: 100.0, topskip: 0.0, maxdepth: 0.0, baselineskip: 13.6, lineskip: 1.0, lineskiplimit: 0.0, flushbottom: false };
        assert_eq!(interline(&p, Some(0.0), 8.33331), 13.6 - 8.33331);
        assert_eq!(interline(&p, Some(0.0), 25.98335), 1.0);
        assert_eq!(interline(&p, None, 25.98335), 0.0);
    }
}
