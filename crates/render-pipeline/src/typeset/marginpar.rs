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

use std::rc::Rc;

use flashtex_compiler::Span;
use flashtex_paragraph_layout as pl;

use crate::adapter::{Item as AItem, ParaStyle, TextStyle};
use crate::display::Diagnostic;
use crate::pagebuild::{self, VBlock, VItem};

use super::{broken_of, drop_trailing_break, line_extents, vskips_of, BoxRec, BuiltBlock, Context, CLUB_PENALTY, WIDOW_PENALTY};

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
        Some(BuiltBlock { block: pl::ParagraphBlock::body(lines), items: Rc::new(list), recs, vertical, labels, cache_key: None })
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

/// Whether the `%` at byte `i` is escaped (`\%` typesets a percent and
/// starts no comment): an odd run of backslashes directly before it.
fn escaped_percent(bytes: &[u8], i: usize) -> bool {
    let mut backslashes = 0;
    let mut j = i;
    while j > 0 && bytes[j - 1] == b'\\' {
        backslashes += 1;
        j -= 1;
    }
    backslashes % 2 == 1
}

/// `(byte offset, `\if@reversemargin` after it)` of every literal
/// `\reversemarginpar` / `\normalmarginpar` in `source`, in order, outside
/// `%` comments (the byte-scan shape [`crate::adapter::body_commands`]
/// still has: a switch a macro runs is missed, and one inside a definition
/// that never runs is counted).
fn margin_switches(source: &str) -> Vec<(usize, bool)> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if !escaped_percent(bytes, i) => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'\\' => {
                let mut j = i + 1;
                while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                    j += 1;
                }
                if j > i + 1 {
                    match &source[i + 1..j] {
                        "reversemarginpar" => out.push((i, true)),
                        "normalmarginpar" => out.push((i, false)),
                        _ => {}
                    }
                    i = j;
                } else {
                    i += 1;
                }
            }
            _ => {
                i += 1;
            }
        }
    }
    out
}

/// Byte offset of the literal `needle` outside `%` comments, if any.
fn find_uncommented(text: &str, needle: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && !escaped_percent(bytes, i) {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if text.get(i..).is_some_and(|rest| rest.starts_with(needle)) {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// The literal margin-side switches per document, scanned once per
/// [`place`] call.
///
/// The compiler logs the switches the document actually ran (its
/// `marginpar_switches` field), whatever produced them -- but this build
/// renders through the pinned vendor compiler, which predates that log, so
/// the scan above stands in for it until the re-pin (the same interim the
/// old entry-source column scan was before `Parsed::column_switches`).
struct SwitchLog {
    /// `margin_switches` per document, indexed like the pipeline's `texts`.
    per_doc: Vec<Vec<(usize, bool)>>,
    /// The entry preamble's last switch (before `\begin{document}`): it ran
    /// before every body note, so a note whose own document runs no earlier
    /// switch inherits it.
    preamble: bool,
}

impl SwitchLog {
    fn of(texts: &[&str]) -> Self {
        let per_doc: Vec<Vec<(usize, bool)>> = texts.iter().map(|t| margin_switches(t)).collect();
        let mut preamble = false;
        for (text, switches) in texts.iter().zip(&per_doc) {
            if let Some(begin) = find_uncommented(text, "\\begin{document}") {
                if let Some((_, reversed)) = switches.iter().filter(|(off, _)| *off < begin).last() {
                    preamble = *reversed;
                }
                break;
            }
        }
        SwitchLog { per_doc, preamble }
    }

    /// `\if@reversemargin` in force at byte `at` of document `doc`: the last
    /// switch at or before it in its own document, else the entry
    /// preamble's.
    fn reversed_at(&self, doc: usize, at: usize) -> bool {
        if let Some(switches) = self.per_doc.get(doc) {
            if let Some((_, reversed)) = switches.iter().filter(|(off, _)| *off <= at).last() {
                return *reversed;
            }
        }
        self.preamble
    }
}

/// Minimum `span.start` per document of the boxes starting on each page:
/// the page break in source terms falls between one page's last content
/// and the next page's first, so a note's page "ends" where the next page's
/// content begins (see [`page_end`]). Built before [`place`] sets any note,
/// so the notes' own lines never move a break.
fn page_first_starts(
    ctx: &Context,
    blocks: &[BuiltBlock],
    pages: &pl::Pages,
) -> Vec<std::collections::HashMap<usize, usize>> {
    let mut out: Vec<std::collections::HashMap<usize, usize>> = vec![std::collections::HashMap::new(); pages.pages.len()];
    for (pi, page) in pages.pages.iter().enumerate() {
        for line in &page.lines {
            let Some(block) = blocks.get(line.paragraph) else { continue };
            let Some(bline) = block.block.lines.lines.get(line.line) else { continue };
            for idx in bline.items.clone() {
                let Some(&Some(rec)) = block.recs.get(idx) else { continue };
                let mut push = |span: Span| {
                    out[pi].entry(span.document.0).and_modify(|at| *at = (*at).min(span.start)).or_insert(span.start);
                };
                match ctx.recs.get(rec) {
                    Some(BoxRec::Text { clusters, .. }) => {
                        for c in clusters {
                            push(c.span);
                        }
                    }
                    Some(BoxRec::Rule { span, .. }) => push(*span),
                    Some(BoxRec::Math(mi)) => {
                        if let Some(m) = ctx.maths.get(*mi) {
                            push(m.span);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    out
}

/// The source offset at which the note's page ships: everything the output
/// routine has processed by then, which is everything up to the page break.
///
/// latex.ltx `\@addmarginpar` (21324-21336) runs inside the output routine
/// and reads `\if@reversemargin` there, so a `\reversemarginpar` after the
/// note but before the break (even on the next page's side of `\newpage`'s
/// line, while still before the next page's first box) already flips that
/// page's notes -- and a `\normalmarginpar` after a later page's note flips
/// it back. The break in source terms is the next page's first content in
/// the note's own document; past the last page, the document's end. A page
/// whose later pages hold no content of the document (an empty trailing
/// page) falls through to the end the same way.
fn page_end(
    ctx: &Context,
    first_starts: &[std::collections::HashMap<usize, usize>],
    doc: usize,
    page: usize,
) -> usize {
    first_starts
        .iter()
        .skip(page + 1)
        .filter_map(|starts| starts.get(&doc).copied())
        .min()
        .unwrap_or_else(|| ctx.texts.get(doc).map(|t| t.len()).unwrap_or(usize::MAX))
}

/// Which margin a note whose page ships at `page_end` (see [`page_end`])
/// set from the calling line's `dx` goes in: latex.ltx `\@addmarginpar`
/// (lines 21324-21336) takes the calling column's outer side under
/// `\if@twocolumn` -- where `\if@reversemargin` is never read (a
/// `[twocolumn]` left-column note stays left with `\reversemarginpar` in
/// force, per the pdflatex oracle) -- and negates the one-column right
/// side for it otherwise. `page_end` is `None` when the note has no span.
fn margin_side(ctx: &Context, log: &SwitchLog, page_end: Option<(usize, usize)>, dx: f64) -> bool {
    let default_left = is_left_column(ctx, dx);
    let twocolumn = ctx
        .style
        .class_geometry
        .as_deref()
        .is_some_and(|g| g.frame.twocolumn && g.frame.columns.len() > 1);
    if twocolumn {
        return default_left;
    }
    let Some((doc, at)) = page_end else { return default_left };
    log.reversed_at(doc, at)
}

/// Places every anchored margin note: appends its block to `blocks` and its
/// lines to the calling line's page, shifted by the per-line offset in
/// `line_dx` (which [`assemble`](super::assemble) applies) into the outer
/// margin of the calling line's column -- or the opposite margin while
/// `\reversemarginpar` is in force at the page's shipout (see
/// [`margin_side`]).
pub(super) fn place(ctx: &mut Context, blocks: &mut Vec<BuiltBlock>, pages: &mut pl::Pages, line_dx: &mut [Vec<f64>]) {
    let anchors = std::mem::take(&mut ctx.marginpar_anchors);
    if anchors.is_empty() {
        return;
    }
    let switch_log = SwitchLog::of(ctx.texts);
    // Where each page breaks in source terms, before any note line joins
    // the pages (see [`page_first_starts`]).
    let first_starts = page_first_starts(ctx, blocks, pages);
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
        // `\@addmarginpar` reads the flag when the page ships, not where
        // the note stands: a switch after the note but before the break
        // still flips this page's notes.
        let shipout = ctx.marginpars.get(m).map(|n| n.span).map(|span| {
            let doc = span.document.0;
            (doc, page_end(ctx, &first_starts, doc, pi))
        });
        let left = margin_side(ctx, &switch_log, shipout, dx);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn reversed(src: &str, at: usize) -> bool {
        SwitchLog::of(&[src]).reversed_at(0, at)
    }

    #[test]
    fn comments_and_longer_names_are_not_switches() {
        let src = "\\documentclass{article}\n% \\reversemarginpar\n\\reversemarginparfoo\n\\begin{document}x\\end{document}\n";
        assert!(!reversed(src, src.len()));
    }

    #[test]
    fn an_escaped_percent_starts_no_comment() {
        let src = "100\\% \\reversemarginpar\n\\begin{document}x\\end{document}\n";
        assert!(reversed(src, src.len()));
    }

    #[test]
    fn the_last_switch_before_the_note_wins() {
        let src = "\\reversemarginpar\nAAAA\n\\normalmarginpar\nBBBB\n";
        assert!(reversed(src, src.find("AAAA").unwrap()));
        assert!(!reversed(src, src.find("BBBB").unwrap()));
    }

    #[test]
    fn a_note_without_its_own_switch_inherits_the_entry_preamble() {
        let entry = "\\documentclass{article}\n\\reversemarginpar\n\\begin{document}\n";
        let frag = "text\\marginpar{M}\n";
        let log = SwitchLog::of(&[entry, frag]);
        assert!(log.reversed_at(1, frag.find("marginpar").unwrap()));
    }
}
