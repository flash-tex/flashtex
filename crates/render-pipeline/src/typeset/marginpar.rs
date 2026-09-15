//! Margin notes: `\marginpar[<left>]{<right>}` (latex.ltx `\@marginpar`).
//!
//! The compiler always keeps the one-sided `<right>` note (its optional
//! `[<left>]` is consumed and reported there). The note is set here in
//! `\footnotesize` — the same size as footnotes, an honest stand-in for
//! the kernel leaving the size to the class — in a box `\marginparwidth`
//! wide, and placed in the right margin with its first baseline on the
//! anchor line's baseline, starting `\marginparsep` past the text block's
//! right edge. Width and separation come from the resolved class frame,
//! so the `geometry` package and preamble `\setlength`s apply.
//!
//! Stated simplifications (see the inventory description): always the
//! right margin (no two-sided alternation, `\reversemarginpar` or
//! `mparswitch`), and no collision avoidance between close notes —
//! following the `BoxMeasurer`/`\addvspace` precedent, these are
//! documented rather than implemented.

use flashtex_compiler::Span;
use flashtex_paragraph_layout as pl;

use crate::adapter::{Item as AItem, ParaStyle, TextStyle};
use crate::display::Diagnostic;
use crate::pagebuild::VBlock;
use crate::style::{frame_pt, Stylesheet};

use super::footnotes::FootnoteParams;
use super::{broken_of, drop_trailing_break, line_extents, vskips_of, BuiltBlock, Context, CLUB_PENALTY, WIDOW_PENALTY};

/// A margin note's text waiting for page placement, with the box record
/// of the line it sits on.
#[derive(Debug, Clone)]
pub struct NoteSrc {
    pub span: Span,
    pub items: Vec<AItem>,
}

impl<'a> Context<'a> {
    /// The note of `self.margin_notes[n]` set `\marginparwidth` wide at
    /// `\footnotesize`, like `footnote_block` but with no mark lead.
    fn margin_block(&mut self, n: usize, width: f64) -> Option<BuiltBlock> {
        let fp = FootnoteParams::of(self.style);
        let note = self.margin_notes.get(n)?.clone();
        let (mut list, mut recs, labels, mut skips) = self.hlist(&note.items, fp.size, TextStyle::default(), ParaStyle::Plain);
        let _ = drop_trailing_break(&mut list, &mut recs, &mut skips, ParaStyle::Plain);
        let mut params = self.line_params(false, fp.baselineskip, ParaStyle::Plain, 0.0);
        params.line_width = width;
        let lines = self.break_paragraph(&list, &params, &note.items, Some(&recs))?;
        self.report_overfull(&lines, &list, &recs);
        // The block never goes through the page breaker (it is hung off
        // its anchor line below), so these penalties never fire; plain
        // zero like body text rather than the footnote insert's.
        let vertical = VBlock {
            lines: line_extents(&lines),
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
            depth_after: crate::pagebuild::DepthAfter::default(),
        };
        Some(BuiltBlock { block: pl::ParagraphBlock::body(lines), items: list, recs, vertical, labels, cache_key: None })
    }
}

/// Hang every anchored margin note off its anchor line: the note's first
/// baseline sits on the anchor's, further lines step by `\footnotesize`
/// `\baselineskip`, and every line is shifted into the right margin.
/// `counters` is `\c@page` per page, as in the main conversion loop.
pub(super) fn place(
    ctx: &mut Context,
    blocks: &mut Vec<BuiltBlock>,
    pages: &mut pl::Pages,
    line_dx: &mut [Vec<f64>],
    counters: &[(i64, flashtex_class_geometry::Numbering)],
) {
    let anchors = std::mem::take(&mut ctx.margin_anchors);
    if anchors.is_empty() {
        return;
    }
    let fp = FootnoteParams::of(ctx.style);
    let style: &Stylesheet = ctx.style;
    let frame = style.class_geometry.as_deref().map(|g| {
        (frame_pt(g.frame.marginpar_width), frame_pt(g.frame.marginpar_sep), frame_pt(g.frame.text_width), style.text_x_pt)
    });
    let Some((width, sep, text_width, text_x)) = frame else {
        // No class frame to measure a margin from: diagnose, never drop
        // silently.
        for (_, n) in anchors {
            let span = ctx.margin_notes[n].span;
            let src = vec![ctx.source(span)];
            ctx.diagnostics.push(Diagnostic::warning(
                "marginpar_unplaced",
                "a \\marginpar is not placed: the document has no resolved class frame to measure the margin from",
                src,
            ));
        }
        return;
    };
    // (page, line-in-page) of every placed body line, by box record.
    let mut line_of: std::collections::HashMap<usize, (usize, usize)> = std::collections::HashMap::new();
    for (pi, page) in pages.pages.iter().enumerate() {
        for (li, placed) in page.lines.iter().enumerate() {
            let Some(block) = blocks.get(placed.paragraph) else { continue };
            let Some(line) = block.block.lines.lines.get(placed.line) else { continue };
            for it in line.items.clone() {
                if let Some(Some(r)) = block.recs.get(it) {
                    line_of.entry(*r).or_insert((pi, li));
                }
            }
        }
    }
    // The right-margin x of every page, like the conversion loop's `dx`.
    let margin_dx: Vec<f64> = pages
        .pages
        .iter()
        .enumerate()
        .map(|(pi, page)| {
            let counter = counters.get(pi).map_or(i64::from(page.number), |c| c.0);
            let left = ctx.style.class_geometry.as_deref().map_or(text_x, |g| frame_pt(g.frame.text_left(counter)));
            left + text_width + sep - text_x
        })
        .collect();
    let mut seen = std::collections::HashSet::new();
    for (rec, n) in anchors {
        if !seen.insert(n) {
            continue;
        }
        let Some(&(pi, li)) = line_of.get(&rec) else {
            let span = ctx.margin_notes[n].span;
            let src = vec![ctx.source(span)];
            ctx.diagnostics.push(Diagnostic::warning(
                "marginpar_unplaced",
                "a \\marginpar is not placed: its anchor line is not on any page",
                src,
            ));
            continue;
        };
        let Some(b) = ctx.margin_block(n, width) else { continue };
        let extents = b.vertical.lines.clone();
        let nb = blocks.len();
        blocks.push(b);
        let mut y = pages.pages[pi].lines[li].baseline_y;
        let dx = margin_dx[pi];
        for (k, (h, d)) in extents.iter().enumerate() {
            if k > 0 {
                y += fp.baselineskip;
            }
            pages.pages[pi].lines.push(pl::PlacedLine { paragraph: nb, line: k, baseline_y: y, height: *h, depth: *d });
            line_dx[pi].push(dx);
            let Some(line) = blocks[nb].block.lines.lines.get(k) else { continue };
            for r in &line.runs {
                let mut r = r.clone();
                r.x += text_x + dx;
                r.baseline_y = y;
                pages.pages[pi].runs.push(r);
            }
        }
    }
}
