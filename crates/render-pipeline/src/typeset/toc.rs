//! One contents-list entry (`crate::toc::TocEntry`) as a paragraph, set
//! the way latex.ltx `\@dottedtocline` and article.cls `\l@section` /
//! report.cls `\l@chapter` set it:
//!
//! ```text
//! \leftskip <indent + numwidth>  \rightskip <\@tocrmarg | \@pnumwidth>
//! \parfillskip -\rightskip       \parindent <indent | 0pt>
//! \leavevmode \hskip -\leftskip \hb@xt@<numwidth>{<number>\hfil} <title>
//! \nobreak <\leaders\hbox{$\mkern4.5mu\hbox{.}\mkern4.5mu$}\hfill | \hfil>
//! \nobreak \hb@xt@\@pnumwidth{\hfil <page>} \par
//! ```
//!
//! so a long title wraps under the number's hanging indent, justified to
//! `\hsize - \rightskip`, and the last line's page number ends at `\hsize`.
//! Leader boxes are aligned (`\leaders`, tex.web §626): each starts at a
//! multiple of the box width from the line's left edge, so the dots of
//! different entries line up.

use super::*;
use crate::toc::TocEntry;

/// `\@pnumwidth` (article/report/book.cls).
const PNUMWIDTH_EM: f64 = 1.55;
/// `\@tocrmarg`.
const TOCRMARG_EM: f64 = 2.55;
/// `\@dotsep`, in mu, on either side of the leader dot.
const DOTSEP_MU: f64 = 4.5;

fn sp(pt: f64) -> i64 {
    (pt * 65536.0).round() as i64
}

impl Context<'_> {
    pub(super) fn toc_entry_block(&mut self, e: &TocEntry) -> Option<BuiltBlock> {
        let s = e.style;
        let body = self.style.body_size_pt;
        let geo = self.style.class_geometry.as_deref();
        // `\l@part`: `\large \bfseries` for the number, title and page.
        let size = match (s.part, geo) {
            (true, Some(g)) => crate::style::frame_pt(flashtex_class_geometry::FontSize::Large.metrics(g.options.size).0),
            (true, None) => body * 1.2,
            (false, _) => body,
        };
        let regular = TextStyle::default();
        let style = TextStyle { bold: s.bold, ..TextStyle::default() };
        // `em` in `\@dottedtocline` and `\l@section`'s lengths is the body
        // font's: they are set before `\bfseries`. The page box of the bold
        // forms is `\@pnumwidth` in the bold font.
        let em = self.text_params(regular, body).quad;
        let indent = s.indent_em * em;
        let numwidth = s.numwidth_em * em;
        let leftskip = indent + numwidth;
        let rightskip = if s.dotted { TOCRMARG_EM } else { PNUMWIDTH_EM } * em;
        let pnumwidth = PNUMWIDTH_EM * self.text_params(style, size).quad;

        let mut list: Vec<pl::Item> = Vec::new();
        let mut recs: Vec<Option<usize>> = Vec::new();
        list.push(pl::Item::kern(-leftskip));
        recs.push(None);
        if let Some((number, span)) = &e.number {
            // `\numberline`: `\hb@xt@\@tempdima{#1\hfil}`.
            let width = match self.toc_text(number, *span, style, size) {
                Some((run, rec)) => {
                    let w = run.width;
                    list.push(pl::Item::Box(run));
                    recs.push(Some(rec));
                    w
                }
                None => 0.0,
            };
            // `\l@part`'s `\thepart\hspace{1em}`: 1em of `\large\bfseries`.
            list.push(pl::Item::kern(if s.part { self.text_params(style, size).quad } else { numwidth - width }));
            recs.push(None);
        }
        let (title, title_recs, _labels, _skips) = self.hlist(&e.title, size, style, ParaStyle::Plain);
        // Drop the paragraph end `hlist` appends; this line has its own.
        let keep = title.len().saturating_sub(3);
        list.extend(title.into_iter().take(keep));
        recs.extend(title_recs.into_iter().take(keep));
        list.push(pl::Item::penalty(pl::INFINITE_PENALTY));
        recs.push(None);
        let mut fill = pl::Glue::fil();
        if s.dotted {
            fill.stretch_order = pl::GlueOrder::Fill;
        }
        list.push(pl::Item::Glue(fill));
        recs.push(None);
        list.push(pl::Item::penalty(pl::INFINITE_PENALTY));
        recs.push(None);
        // `\hb@xt@\@pnumwidth{\hfil <page>}` (`\hss` for the bold forms;
        // the `\kern-\p@\kern\p@` after it nets to zero).
        let page = self.toc_text(&e.page, e.list_span, style, size);
        list.push(pl::Item::kern(pnumwidth - page.as_ref().map_or(0.0, |(run, _)| run.width)));
        recs.push(None);
        if let Some((run, rec)) = page {
            list.push(pl::Item::Box(run));
            recs.push(Some(rec));
        }
        list.push(pl::Item::penalty(pl::INFINITE_PENALTY));
        recs.push(None);
        list.push(pl::Item::Glue(pl::Glue::fixed(-rightskip)));
        recs.push(None);
        list.push(pl::Item::penalty(pl::FORCED_BREAK));
        recs.push(None);

        let mut params = self.line_params(false, self.style.baselineskip_pt, ParaStyle::Plain, 0.0);
        params.left_skip = pl::Glue::fixed(leftskip);
        params.right_skip = pl::Glue::fixed(rightskip);
        params.parindent = if s.dotted { indent } else { 0.0 };
        // `\onecolumn` around a report/book list: `\hsize\textwidth`.
        if let (true, Some(g)) = (e.wide, geo) {
            params.line_width = crate::style::frame_pt(g.frame.text_width);
        }
        let line_width = params.line_width;
        let mut lines = self.break_paragraph(&list, &params, &e.title, Some(&recs))?;
        self.report_overfull(&lines, &list, &recs);

        if s.dotted {
            self.leader_dots(&mut lines, &mut list, &mut recs, e, line_width, pnumwidth, size);
        }
        let vertical = VBlock {
            lines: line_extents(&lines),
            penalty_before: s.penalty_before,
            space_before: Some((s.skip_before.0 * em, s.skip_before.1, 0.0)).filter(|k| k.0 != 0.0 || k.1 != 0.0),
            parskip: Some(skip_tuple(self.style.parskip)),
            interline_penalty: if s.dotted { pagebuild::INF_PENALTY } else { 0 },
            club_penalty: CLUB_PENALTY,
            widow_penalty: WIDOW_PENALTY,
            penalty_after: s.penalty_after,
            space_after: None,
            no_interline_first: false,
            no_interline_after: false,
            baselineskip: Some(self.style.baselineskip_pt),
            vskip_after: Vec::new(),
            pre_space_after: None,
        };
        Some(BuiltBlock {
            block: pl::ParagraphBlock::body(lines),
            items: std::rc::Rc::new(list),
            recs,
            vertical,
            labels: Vec::new(),
            cache_key: None,
        })
    }

    /// The `\leaders` dots of the entry's last line: the `\hfill` between
    /// the title and the page box, filled with `\hbox{$\mkern\@dotsep
    /// mu\hbox{.}\mkern\@dotsep mu$}` boxes aligned to the line's left edge.
    /// Each dot is appended as a box after the line's own material (its run
    /// last in the line), carrying the list command's bytes.
    #[allow(clippy::too_many_arguments)]
    fn leader_dots(&mut self, lines: &mut pl::Lines, list: &mut Vec<pl::Item>, recs: &mut Vec<Option<usize>>, e: &TocEntry, line_width: f64, pnumwidth: f64, size: f64) {
        let Some(last) = lines.lines.last_mut() else { return };
        // The fill glue is the only `fill` stretch on the line: its set
        // width is the line's glue ratio (none when the line is overfull).
        if !(last.ratio.is_finite() && last.ratio > 0.0) {
            return;
        }
        let Some((dot, rec)) = self.toc_text(".", e.list_span, TextStyle::default(), size) else { return };
        // `mu` is 1/18 of `\textfont2`'s quad: 1em of the body size.
        let mu = size / 18.0;
        let leader_wd = sp(2.0 * DOTSEP_MU * mu + dot.width);
        let rule_wd = sp(last.ratio);
        if leader_wd <= 0 {
            return;
        }
        // tex.web §626: `edge := cur_h + rule_wd + 10`; aligned leaders start
        // at the first multiple of the box width at or after `cur_h`.
        let start = sp(line_width - pnumwidth - last.ratio);
        let edge = start + rule_wd + 10;
        let mut cur = leader_wd * start.div_euclid(leader_wd);
        if cur < start {
            cur += leader_wd;
        }
        let baseline = last.runs.first().map_or(0.0, |r| r.baseline_y);
        while cur + leader_wd <= edge {
            let x = cur as f64 / 65536.0 + DOTSEP_MU * mu;
            list.push(pl::Item::Box(dot.clone()));
            recs.push(Some(rec));
            last.runs.push(position_run(&dot, x, baseline));
            cur += leader_wd;
        }
        last.items.end = list.len();
    }

    /// Generated text (a number, a page number, a leader dot) as one box
    /// whose characters all carry `span`.
    fn toc_text(&mut self, text: &str, span: Span, style: TextStyle, size: f64) -> Option<(pl::GlyphRun, usize)> {
        if text.is_empty() {
            return None;
        }
        let chars = text
            .chars()
            .map(|_| adapter::CharSrc {
                document: span.document,
                start: span.start,
                end: span.end,
            })
            .collect();
        let seg = adapter::Segment {
            text: text.to_string(),
            chars,
            style,
        };
        self.text_box(&seg, size)
    }
}
