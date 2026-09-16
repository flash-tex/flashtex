//! Margin notes: `\marginpar[<left>]{<right>}` (latex.ltx `\@marginpar`).
//!
//! The compiler's `Inline::Marginpar` carries the one-sided `<right>` note;
//! the running text carries no mark. The pipeline sets the note at
//! `\footnotesize` in the right margin, `marginparwidth` wide and
//! `marginparsep` from the text block. The top of the note aligns with the
//! top of the calling line, clipped to the text area; close-note collision
//! avoidance is out of scope (overlapping notes are left overlapping).
//! Two-column documents keep the one-sided placement (the inter-column gap),
//! and notes met inside floats or `multicols` are reported, not placed.

use flashtex_compiler::Span;
use flashtex_paragraph_layout as pl;

use crate::adapter::{Item as AItem, ParaStyle, TextStyle};
use crate::display::Diagnostic;
use crate::pagebuild::{self, VBlock, VItem};

use super::footnotes::FootnoteParams;
use super::{broken_of, drop_trailing_break, line_extents, vskips_of, BuiltBlock, Context, CLUB_PENALTY, WIDOW_PENALTY};

/// A margin note's text waiting for placement.
#[derive(Debug, Clone)]
pub struct MarginparSrc {
    pub span: Span,
    pub items: Vec<AItem>,
}

impl<'a> Context<'a> {
    /// The note of `self.marginpars[m]` set as running text at
    /// `\footnotesize`, `width` wide. Like [`footnotes`](super::footnotes)
    /// minus the `\@makefntext` mark box: a plain paragraph.
    pub(super) fn marginpar_block(&mut self, m: usize, width: f64) -> Option<BuiltBlock> {
        let fp = FootnoteParams::of(self.style);
        let note = self.marginpars.get(m)?.clone();
        let (mut list, mut recs, labels, mut skips) = self.hlist(&note.items, fp.size, TextStyle::default(), ParaStyle::Plain);
        if !list.iter().any(|i| matches!(i, pl::Item::Box(_))) {
            return None;
        }
        let _ = drop_trailing_break(&mut list, &mut recs, &mut skips, ParaStyle::Plain);
        let mut params = self.line_params(false, fp.baselineskip, ParaStyle::Plain, 0.0);
        params.line_width = width;
        let lines = self.break_paragraph(&list, &params, &note.items, Some(&recs))?;
        self.report_overfull(&lines, &list, &recs);
        let mut extents = line_extents(&lines);
        if let Some(last) = extents.last_mut() {
            last.1 = last.1.max(fp.strut_depth());
        }
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
            baselineskip: Some(fp.baselineskip),
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

/// Places every anchored margin note: appends its block to `blocks` and its
/// lines to the calling line's page, shifted into the right margin by the
/// per-line offset in `line_dx` (which [`assemble`](super::assemble) applies).
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
    let fp = FootnoteParams::of(ctx.style);
    let text_top = ctx.style.text_y_pt;
    let text_bottom = text_top + ctx.style.text_height_pt;
    for (rec, m) in anchors {
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
        // Stack the note's lines from a candidate top, measure, then clip
        // to the text area and stack for real.
        let page_params = pagebuild::PageParams {
            vsize: ctx.style.text_height_pt,
            topskip: ctx.style.topskip_pt,
            maxdepth: ctx.style.maxdepth_pt,
            baselineskip: fp.baselineskip,
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
        let call = &pages.pages[pi].lines[pli];
        let (_, height) = stack(0.0);
        let mut top = call.baseline_y - call.height;
        if top < text_top {
            top = text_top;
        }
        if top + height > text_bottom {
            top = (text_bottom - height).max(text_top);
        }
        let (stacked, _) = stack(top);
        // The note sits `marginparsep` past the text block's right edge,
        // riding the calling line's own column offset.
        let dx = line_dx.get(pi).and_then(|d| d.get(pli)).copied().unwrap_or(0.0);
        let note_dx = dx + ctx.style.text_width_pt + sep;
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
