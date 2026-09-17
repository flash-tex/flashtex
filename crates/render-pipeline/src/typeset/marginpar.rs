//! Margin notes: `\marginpar[<left>]{<right>}` (latex.ltx `\@marginpar`).
//!
//! The compiler's `Inline::Marginpar` carries the one-sided `<right>` note;
//! the running text carries no mark. The pipeline sets the note at
//! `\normalsize`, `marginparwidth` wide and `marginparsep` from the text
//! block. The note's first line's baseline aligns with the calling line's
//! baseline (`\@addmarginpar`'s `\vtop`), with no vertical clipping to the
//! text area; consecutive notes on the same page and side are kept at least
//! `\marginparpush` apart. In `[twocolumn]`, left-column notes go in the
//! left margin and right-column notes in the right margin (the outer side
//! of each column); notes met inside floats or `multicols` are reported,
//! not placed.

use flashtex_compiler::Span;
use flashtex_paragraph_layout as pl;

use crate::adapter::{Item as AItem, ParaStyle, TextStyle};
use crate::display::Diagnostic;
use crate::pagebuild::{self, VBlock, VItem};

use super::{broken_of, drop_trailing_break, line_extents, vskips_of, BuiltBlock, Context, CLUB_PENALTY, WIDOW_PENALTY};

/// A margin note's text waiting for placement.
#[derive(Debug, Clone)]
pub struct MarginparSrc {
    pub span: Span,
    pub items: Vec<AItem>,
}

impl<'a> Context<'a> {
    /// The note of `self.marginpars[m]` set as running text at
    /// `\normalsize` (`\@marginparreset`: `\reset@font\normalsize...`, not
    /// `\footnotesize` -- GH-505's original guess), `width` wide. Like
    /// [`footnotes`](super::footnotes) minus the `\@makefntext` mark box: a
    /// plain paragraph.
    pub(super) fn marginpar_block(&mut self, m: usize, width: f64) -> Option<BuiltBlock> {
        let size = self.style.body_size_pt;
        let baselineskip = self.style.baselineskip_pt;
        let note = self.marginpars.get(m)?.clone();
        let (mut list, mut recs, labels, mut skips) = self.hlist(&note.items, size, TextStyle::default(), ParaStyle::Plain);
        if !list.iter().any(|i| matches!(i, pl::Item::Box(_))) {
            return None;
        }
        let _ = drop_trailing_break(&mut list, &mut recs, &mut skips, ParaStyle::Plain);
        let mut params = self.line_params(false, baselineskip, ParaStyle::Plain, 0.0);
        params.line_width = width;
        let lines = self.break_paragraph(&list, &params, &note.items, Some(&recs))?;
        self.report_overfull(&lines, &list, &recs);
        // `\@savemarbox`/`\@marginparreset` add no strut: `\@addmarginpar`
        // stacks on the box's real `\dp` (round-2 review finding 1 -- a
        // phantom `0.3\baselineskip` strut here, copied from the footnote
        // builder where `\@makefntext` really does add one, drifted every
        // pushed note low, cumulatively).
        let extents = line_extents(&lines);
        let vertical = VBlock {
            lines: extents,
            penalty_before: None,
            space_before: None,
            parskip: None,
            interline_penalty: 0,
            club_penalty: CLUB_PENALTY,
            widow_penalty: WIDOW_PENALTY,
            penalty_after: None,
            space_after: None,
            no_interline_first: false,
            no_interline_after: false,
            baselineskip: Some(baselineskip),
            vskip_after: vskips_of(&lines, &skips),
            broken_penalty: broken_of(&lines),
            pre_space_after: None,
            lineskip: None,
            contributed: None,
            line_penalty: Vec::new(),
            depth_after: pagebuild::DepthAfter::default(),
        };
        Some(BuiltBlock { block: pl::ParagraphBlock::body(lines), items: list, recs, vertical, labels, cache_key: None })
    }
}

/// Whether a line's `line_dx` offset (from [`assemble`](super::assemble))
/// places it in the left (outer) column of a `[twocolumn]` document.
///
/// `assemble` computes `dx = text_left(counter) - text_x_pt +
/// columns[col].offset`; `columns[0].offset` is always zero and
/// `columns[1].offset` is a page-independent constant, so trying both
/// possible `text_left` values (odd/even) against both columns and taking
/// the closest match recovers `col` without needing the page's real
/// (possibly `\twoside`-shifted) counter here.
fn is_left_column(ctx: &Context, dx: f64) -> bool {
    let Some(g) = ctx.style.class_geometry.as_deref() else { return false };
    if !g.frame.twocolumn || g.frame.columns.len() < 2 {
        return false;
    }
    let col1_offset = crate::style::frame_pt(g.frame.columns[1].offset);
    let bases = [
        crate::style::frame_pt(g.frame.odd_text_left) - ctx.style.text_x_pt,
        crate::style::frame_pt(g.frame.even_text_left) - ctx.style.text_x_pt,
    ];
    bases
        .into_iter()
        .flat_map(|base| [(base, true), (base + col1_offset, false)])
        .min_by(|(a, _), (b, _)| (a - dx).abs().total_cmp(&(b - dx).abs()))
        .is_some_and(|(_, left)| left)
}

/// Places every anchored margin note: appends its block to `blocks` and its
/// lines to the calling line's page, shifted by the per-line offset in
/// `line_dx` (which [`assemble`](super::assemble) applies) into the outer
/// margin of the calling line's column.
pub(super) fn place(ctx: &mut Context, blocks: &mut Vec<BuiltBlock>, pages: &mut pl::Pages, line_dx: &mut [Vec<f64>]) {
    let anchors = std::mem::take(&mut ctx.marginpar_anchors);
    if anchors.is_empty() {
        return;
    }
    // Notes met while setting a note are not placed (the compiler
    // diagnoses them), like nested footnotes.
    let mut line_of: std::collections::HashMap<usize, (usize, usize)> = std::collections::HashMap::new();
    for (bi, b) in blocks.iter().enumerate() {
        for (li, l) in b.block.lines.lines.iter().enumerate() {
            for it in l.items.clone() {
                if let Some(Some(r)) = b.recs.get(it) {
                    line_of.entry(*r).or_insert((bi, li));
                }
            }
        }
    }
    let mut page_of: std::collections::HashMap<(usize, usize), (usize, usize)> = std::collections::HashMap::new();
    for (pi, page) in pages.pages.iter().enumerate() {
        for (pli, line) in page.lines.iter().enumerate() {
            page_of.entry((line.paragraph, line.line)).or_insert((pi, pli));
        }
    }
    let width = ctx.style.marginparwidth_pt;
    let sep = ctx.style.marginparsep_pt;
    if width <= 0.0 {
        for (_, m) in &anchors {
            let span = ctx.marginpars.get(*m).map_or(Span::new(0, 0), |n| n.span);
            ctx.diagnostics.push(Diagnostic::warning(
                "marginpar_zero_width",
                format!("\\marginparwidth is {width:.2}pt, so the margin note is not set"),
                vec![ctx.source(span)],
            ));
        }
        return;
    }
    // Tracks each note's bottom (in page-top-relative pt) so the next note
    // on the same page and side leaves at least `\marginparpush` below it
    // (`\@addmarginpar`'s own stacking rule; the text area itself is never
    // clamped against).
    let mut bottom_of: std::collections::HashMap<(usize, bool), f64> = std::collections::HashMap::new();
    for (rec, m) in anchors {
        // multicol.sty's own real warning: floats and marginpars are not
        // allowed inside `multicols`. The diagnostic already fired from the
        // source scan (`multicol::scan`); this is the actual skip.
        let in_multicols = ctx.multicol.in_region_body
            || ctx
                .marginpars
                .get(m)
                .map(|n| n.span)
                .is_some_and(|span| ctx.multicol.contains(span.document.0, span.start));
        if in_multicols {
            continue;
        }
        let Some(&(bi, li)) = line_of.get(&rec) else {
            let src = ctx.marginpars.get(m).map(|n| vec![ctx.source(n.span)]).unwrap_or_default();
            ctx.diagnostics.push(Diagnostic::warning(
                "unsupported_block",
                "a \\marginpar is not in a body paragraph (heading, caption or table cell): its text is not set",
                src,
            ));
            continue;
        };
        let Some(&(pi, pli)) = page_of.get(&(bi, li)) else {
            let src = ctx.marginpars.get(m).map(|n| vec![ctx.source(n.span)]).unwrap_or_default();
            ctx.diagnostics.push(Diagnostic::warning(
                "unsupported_block",
                "a \\marginpar's calling line is not on any page: its text is not set",
                src,
            ));
            continue;
        };
        let Some(b) = ctx.marginpar_block(m, width) else { continue };
        let page_params = pagebuild::PageParams {
            vsize: ctx.style.text_height_pt,
            topskip: ctx.style.topskip_pt,
            maxdepth: ctx.style.maxdepth_pt,
            baselineskip: ctx.style.baselineskip_pt,
            lineskip: ctx.style.lineskip_pt,
            lineskiplimit: ctx.style.lineskiplimit_pt,
            flushbottom: !ctx.style.raggedbottom,
        };
        let vlist = pagebuild::vlist(&page_params, std::slice::from_ref(&b.vertical));
        let stack = |top: f64| -> (Vec<(usize, f64, f64, f64)>, f64) {
            let mut y = top;
            let mut d = 0.0;
            let mut out = Vec::new();
            for v in &vlist {
                match v {
                    VItem::Box { height, depth, payload } => {
                        y += d + height;
                        d = *depth;
                        out.push((payload.1, y, *height, *depth));
                    }
                    VItem::Glue { width, .. } => {
                        y += d + width;
                        d = 0.0;
                    }
                    VItem::Penalty(_) => {}
                }
            }
            (out, y + d - top)
        };
        // The note is a `\vtop`: its first line's baseline is the point
        // `\@addmarginpar` places, i.e. the calling line's own baseline.
        let (trial, height) = stack(0.0);
        let Some(&(_, first_baseline, ..)) = trial.first() else { continue };
        let call = &pages.pages[pi].lines[pli];
        let dx = line_dx.get(pi).and_then(|d| d.get(pli)).copied().unwrap_or(0.0);
        let left = is_left_column(ctx, dx);
        let mut top = call.baseline_y - first_baseline;
        if let Some(&prev_bottom) = bottom_of.get(&(pi, left)) {
            top = top.max(prev_bottom + ctx.style.marginparpush_pt);
        }
        let (stacked, _) = stack(top);
        bottom_of.insert((pi, left), top + height);
        // The note sits `marginparsep` past the outer edge of the calling
        // line's own column (the right edge normally, the left edge for a
        // `[twocolumn]` left-column note).
        let note_dx = if left { dx - sep - width } else { dx + ctx.style.text_width_pt + sep };
        let nb = blocks.len();
        blocks.push(b);
        for (li2, baseline, h, dp) in stacked {
            pages.pages[pi].lines.push(pl::PlacedLine { paragraph: nb, line: li2, baseline_y: baseline, height: h, depth: dp });
            line_dx[pi].push(note_dx);
        }
    }
    // Notes met while setting a note are not placed.
    ctx.marginpar_anchors.clear();
}
