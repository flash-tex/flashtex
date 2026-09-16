//! Page-bottom footnotes (`\footnote`, `\footnotemark`, `\footnotetext`).
//!
//! Geometry follows article.cls and its `size1x.clo` files:
//!
//! * text is `\footnotesize` (8/9.5pt, 9/11pt, 10/12pt for the 10/11/12pt
//!   classes), starting after `\@makefntext`'s 1.8em mark box;
//! * each footnote begins with a `\footnotesep` strut (6.65/7.7/8.4pt), which
//!   for these classes makes the baseline distance between footnotes exactly
//!   the footnote `\baselineskip`;
//! * `\skip\footins` (9/10/10.8pt, natural width) separates body and notes;
//! * `\footnoterule` is `\kern-3pt \hrule width .4\columnwidth \kern2.6pt`
//!   (0.4pt thick, zero net height);
//! * marks are `\textsuperscript`: the `\sf@size` of the current size
//!   (`\DeclareMathSizes`) raised by cmsy's `sup1` (0.412892em).
//!
//! Placement mirrors TeX's insertion builder on this compiler's line model.
//! The line carrying a mark stays on its page; the note's lines follow it
//! onto that page while the body's last baseline plus the notes still fit
//! the text area (and `\dimen\footins`, 8in). Lines that do not fit are held
//! over, split between lines, and every later note on the same page waits
//! behind them, preserving order. Held-over lines open the next page's
//! footnote area; the document end adds footnote-only pages until none are
//! left.
//!
//! Approximations: notes are always set flush with the bottom of the text
//! area. That is exact for pages ended by `\newpage` or the end of the
//! document; for a naturally broken `\raggedbottom` page LaTeX sets them
//! directly below the (nearly full) body instead. The glue stretch and shrink
//! of `\skip\footins`, the depth of the last body line, and TeX's
//! insertion-split penalties are not modelled. Nested `\footnote` text is
//! diagnosed and dropped (its mark stays), and footnotes in headings or
//! captions are not recognised yet.

use super::{
    emit, glyph_width, round2, Font, LayoutCursor, Page, RuleGeometry, TextItem, LINE_SPACING,
    MARGIN_PT, PAGE_HEIGHT_PT, PAGE_WIDTH_PT,
};
use crate::diagnostics::Diagnostic;
use crate::math::FRACTION_RULE_CHAR;
use crate::parser::Inline;
use crate::Span;
use std::collections::VecDeque;

/// Last body baseline of an unfootnoted page (the text area bottom).
const TEXT_BOTTOM_PT: f64 = PAGE_HEIGHT_PT - MARGIN_PT;
/// `\dimen\footins` (latex.ltx): at most 8in of footnotes per page.
const FOOTINS_MAX_PT: f64 = 8.0 * 72.0;
/// `\@makefntext`'s `\hb@xt@1.8em` mark box.
const MARK_BOX_EM: f64 = 1.8;
/// cmsy10 `sup1` (fontdimen 13): the raise of a text-style superscript.
const MARK_RAISE_EM: f64 = 0.412892;
/// `\footnoterule`: `\kern-3pt` above the notes, then a 0.4pt `\hrule`.
const RULE_KERN_PT: f64 = 3.0;
const RULE_THICKNESS_PT: f64 = 0.4;
const RULE_WIDTH_FRACTION: f64 = 0.4;

/// Class-dependent footnote dimensions, selected like `size_declaration_pt`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Metrics {
    /// `\footnotesize`.
    size: f64,
    /// `\footnotesize`'s `\baselineskip`.
    baselineskip: f64,
    footnotesep: f64,
    skip_footins: f64,
    /// `\sf@size` of the body size: the running-text mark.
    text_mark_size: f64,
    /// `\sf@size` of `\footnotesize`: the mark in the footnote itself.
    note_mark_size: f64,
}

impl Metrics {
    fn for_body(body_size_pt: f64) -> Self {
        let (size, baselineskip, footnotesep, skip_footins, text_mark_size, note_mark_size) =
            if body_size_pt <= 10.5 {
                (8.0, 9.5, 6.65, 9.0, 7.0, 6.0)
            } else if body_size_pt <= 11.5 {
                (9.0, 11.0, 7.7, 10.0, 8.0, 6.0)
            } else {
                (10.0, 12.0, 8.4, 10.8, 8.0, 7.0)
            };
        Metrics {
            size,
            baselineskip,
            footnotesep,
            skip_footins,
            text_mark_size,
            note_mark_size,
        }
    }

    /// Height from the top of the footnote area to the last line's baseline.
    fn stack_height(&self, lines: &[FootLine]) -> f64 {
        lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                line.extra_pt
                    + if index == 0 {
                        // `\footnotesep` strut, or `\splittopskip` (also
                        // `\footnotesep`) for a continued note.
                        self.footnotesep
                    } else {
                        self.baselineskip
                    }
            })
            .sum()
    }

    /// Vertical space the lines take from the body, including `\skip\footins`.
    fn reserve(&self, lines: &[FootLine]) -> f64 {
        if lines.is_empty() {
            0.0
        } else {
            self.skip_footins + self.stack_height(lines)
        }
    }
}

/// One typeset footnote line. Item baselines (and rule tops) are relative to
/// the line's baseline; `x` is final.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct FootLine {
    items: Vec<TextItem>,
    /// Extra depth beyond the regular line pitch (tall inline math).
    extra_pt: f64,
    /// The footnote command, attributed to the rule of a page it opens.
    span: Span,
}

#[derive(Default)]
pub(super) struct FootnoteState {
    /// Lines committed to the current (last) page, set when it closes.
    page: Vec<FootLine>,
    /// Lines waiting for a later page.
    held: VecDeque<FootLine>,
    /// Set only in the scratch cursor that typesets one footnote's text:
    /// `(page index, first item index, final baseline)` per closed line.
    line_log: Option<Vec<(usize, usize, f64)>>,
}

impl FootnoteState {
    pub(super) fn record_closed_line(&mut self, page_index: usize, line_start: usize, y: f64) {
        if let Some(log) = self.line_log.as_mut() {
            log.push((page_index, line_start, y));
        }
    }

    /// Queue lengths for a `tabbing` `\kill` rewind: footnote text queued
    /// while the killed row lays out (its mark is an ordinary item and
    /// rewinds with the rest) must not reach the page bottom.
    pub(super) fn undo_point(&self) -> (usize, usize) {
        (self.page.len(), self.held.len())
    }

    pub(super) fn rollback(&mut self, point: (usize, usize)) {
        self.page.truncate(point.0);
        self.held.truncate(point.1);
    }
}

impl LayoutCursor {
    fn footnote_metrics(&self) -> Metrics {
        Metrics::for_body(self.constraints.font_size_pt)
    }

    /// The lowest baseline body text may use on the current page.
    pub(super) fn body_bottom(&self) -> f64 {
        TEXT_BOTTOM_PT - self.footnote_metrics().reserve(&self.footnotes.page)
    }

    /// Close the current page (setting its footnotes) and open the next one,
    /// whose first body line has size `first_line_size`.
    pub(super) fn open_next_page(&mut self, first_line_size: f64) {
        self.set_page_footnotes();
        let number = self.pages.len() as u32 + 1;
        self.pages.push(Page {
            number,
            width_pt: PAGE_WIDTH_PT,
            height_pt: PAGE_HEIGHT_PT,
            items: Vec::new(),
        });
        // Every shipped page steps the displayed page counter too
        // (`\thepage`'s `\c@page`), whatever opened it.
        self.page_value += 1;
        self.take_held_footnotes(first_line_size);
    }

    /// End of document: set the last page's notes, then flush held lines.
    pub(super) fn finish_footnotes(&mut self) {
        self.set_page_footnotes();
        while !self.footnotes.held.is_empty() {
            let number = self.pages.len() as u32 + 1;
            self.pages.push(Page {
                number,
                width_pt: PAGE_WIDTH_PT,
                height_pt: PAGE_HEIGHT_PT,
                items: Vec::new(),
            });
            self.page_value += 1;
            self.take_held_footnotes(0.0);
            self.set_page_footnotes();
        }
    }

    /// Typeset one `Inline::Footnote`: the running-text mark, then the note.
    pub(super) fn footnote(
        &mut self,
        number: &str,
        span: Span,
        mark: bool,
        text: Option<&[Inline]>,
        space_before: bool,
    ) {
        let metrics = self.footnote_metrics();
        if mark {
            self.place(
                number.to_string(),
                metrics.text_mark_size,
                span,
                Font::TimesRoman,
                space_before,
            );
            let raise = MARK_RAISE_EM * self.constraints.font_size_pt;
            if let Some(item) = self.pages.last_mut().and_then(|page| page.items.last_mut()) {
                item.baseline_y_pt = round2(item.baseline_y_pt - raise);
            }
        }
        let Some(text) = text else {
            return;
        };
        if self.footnotes.line_log.is_some() {
            self.diagnostics.push(Diagnostic::warning(
                "nested \\footnote text is not supported",
                Some(span),
                Some("kept the nested mark and dropped its footnote text".into()),
            ));
            return;
        }
        let lines = self.footnote_lines(number, span, text, metrics);
        for line in lines {
            if !self.footnotes.held.is_empty() {
                self.footnotes.held.push_back(line);
                continue;
            }
            self.footnotes.page.push(line);
            let stack = metrics.stack_height(&self.footnotes.page);
            let fits = self.y <= TEXT_BOTTOM_PT - metrics.skip_footins - stack + 1e-9
                && stack <= FOOTINS_MAX_PT;
            if !fits {
                let line = self.footnotes.page.pop().expect("line was just pushed");
                self.footnotes.held.push_back(line);
            }
        }
    }

    /// Move held-over lines onto the freshly opened page while they leave
    /// room for a first body line of `first_line_size` (0 on a notes-only
    /// page). An empty footnote area always takes one line, so held notes
    /// can never stall.
    fn take_held_footnotes(&mut self, first_line_size: f64) {
        let metrics = self.footnote_metrics();
        let limit = (TEXT_BOTTOM_PT - MARGIN_PT - first_line_size).min(FOOTINS_MAX_PT);
        while let Some(line) = self.footnotes.held.pop_front() {
            self.footnotes.page.push(line);
            if self.footnotes.page.len() > 1 && metrics.reserve(&self.footnotes.page) > limit {
                let line = self.footnotes.page.pop().expect("line was just pushed");
                self.footnotes.held.push_front(line);
                break;
            }
        }
    }

    /// Emit the current page's committed footnote lines above its bottom.
    fn set_page_footnotes(&mut self) {
        let lines = std::mem::take(&mut self.footnotes.page);
        let Some(first) = lines.first() else {
            return;
        };
        let metrics = self.footnote_metrics();
        let top = TEXT_BOTTOM_PT - metrics.stack_height(&lines);
        let rule_top = round2(top - RULE_KERN_PT);
        let rule = TextItem {
            text: FRACTION_RULE_CHAR.to_string(),
            x_pt: MARGIN_PT,
            baseline_y_pt: rule_top,
            font_size_pt: metrics.size,
            span: first.span,
            font: Font::TimesRoman,
            rule: Some(RuleGeometry {
                y_pt: rule_top,
                width_pt: round2(RULE_WIDTH_FRACTION * self.constraints.measure_pt),
                height_pt: RULE_THICKNESS_PT,
            }),
        };
        let page = self.pages.last_mut().expect("at least one page");
        page.items.push(rule);
        let mut baseline = top;
        for (index, line) in lines.into_iter().enumerate() {
            baseline += line.extra_pt
                + if index == 0 {
                    metrics.footnotesep
                } else {
                    metrics.baselineskip
                };
            for mut item in line.items {
                item.baseline_y_pt = round2(baseline + item.baseline_y_pt);
                if let Some(rule) = item.rule.as_mut() {
                    rule.y_pt = round2(baseline + rule.y_pt);
                }
                page.items.push(item);
            }
        }
    }

    /// Break one footnote's text into lines with the shared layout engine,
    /// in a scratch cursor at `\footnotesize` over the full column width.
    fn footnote_lines(
        &mut self,
        number: &str,
        span: Span,
        text: &[Inline],
        metrics: Metrics,
    ) -> Vec<FootLine> {
        let mut scratch =
            LayoutCursor::with_labels(self.constraints, self.resolved_labels.clone(), true);
        // A `\thepage` (or `\label`) inside the note resolves on the page
        // the footnote mark sits on, in the style in force there.
        scratch.page_style = self.page_style;
        scratch.page_value = self.page_value;
        scratch.footnotes.line_log = Some(Vec::new());
        // A footnote is an ordinary justified paragraph in LaTeX: wrapped
        // lines stretch to the right edge, the last line stays ragged.
        scratch.justify = true;
        let indent = MARGIN_PT + MARK_BOX_EM * metrics.size;
        scratch.x = indent;
        scratch.content_end = indent;
        scratch.y = MARGIN_PT + metrics.size;
        scratch.line_ascent = metrics.size;
        scratch.line_descent = metrics.size * (LINE_SPACING - 1.0);
        emit(&mut scratch, text, metrics.size, Font::TimesRoman);
        scratch.resolve_hfill();

        self.diagnostics.append(&mut scratch.diagnostics);
        let page_number = self.pages.len() as u32;
        for (key, mut value) in std::mem::take(&mut scratch.collected_labels) {
            value.page = page_number;
            value.page_text = self.page_style.format(self.page_value);
            self.collected_labels.insert(key, value);
        }

        let mut log = scratch.footnotes.line_log.take().unwrap_or_default();
        log.push((scratch.pages.len() - 1, scratch.line_start, scratch.y));
        let mut offsets = Vec::with_capacity(scratch.pages.len());
        let mut total = 0;
        for page in &scratch.pages {
            offsets.push(total);
            total += page.items.len();
        }
        let flat: Vec<TextItem> = scratch.pages.into_iter().flat_map(|p| p.items).collect();
        let starts: Vec<usize> = log
            .iter()
            .map(|(page, start, _)| offsets[*page] + start)
            .collect();

        let mut lines = Vec::with_capacity(log.len());
        for (index, &(page, _, y)) in log.iter().enumerate() {
            let start = starts[index];
            let end = starts
                .get(index + 1)
                .copied()
                .unwrap_or(flat.len())
                .max(start);
            if index > 0 && index + 1 == log.len() && start == end {
                // A trailing `\\` leaves an empty last line; TeX drops it too.
                break;
            }
            let pitch = match index.checked_sub(1).map(|previous| log[previous]) {
                Some((previous_page, _, previous_y)) if previous_page == page => {
                    y - previous_y - LINE_SPACING * metrics.size
                }
                _ => y - (MARGIN_PT + metrics.size),
            };
            let extra_pt = if pitch > 0.005 { round2(pitch) } else { 0.0 };
            let mut items: Vec<TextItem> = flat[start..end]
                .iter()
                .cloned()
                .map(|mut item| {
                    item.baseline_y_pt -= y;
                    if let Some(rule) = item.rule.as_mut() {
                        rule.y_pt -= y;
                    }
                    item
                })
                .collect();
            if index == 0 {
                let width = glyph_width(number, metrics.note_mark_size, Font::TimesRoman);
                items.insert(
                    0,
                    TextItem {
                        text: number.to_string(),
                        x_pt: round2(indent - width),
                        baseline_y_pt: -MARK_RAISE_EM * metrics.size,
                        font_size_pt: metrics.note_mark_size,
                        span,
                        font: Font::TimesRoman,
                        rule: None,
                    },
                );
            }
            lines.push(FootLine {
                items,
                extra_pt,
                span,
            });
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::incremental::{compile_full, LayoutConstraints, Session};
    use crate::parser::{self, Block};

    fn compile(source: &str) -> crate::incremental::CompileOutput {
        compile_full(source, LayoutConstraints::default())
    }

    fn items(pages: &[Page]) -> Vec<(usize, &TextItem)> {
        pages
            .iter()
            .enumerate()
            .flat_map(|(index, page)| page.items.iter().map(move |item| (index, item)))
            .collect()
    }

    fn find<'a>(pages: &'a [Page], source: &str, needle: &str) -> (usize, &'a TextItem) {
        let start = source.find(needle).expect("needle in source");
        pages
            .iter()
            .enumerate()
            .flat_map(|(index, page)| page.items.iter().map(move |item| (index, item)))
            .find(|(_, item)| item.span.start == start && item.rule.is_none())
            .expect("item for needle")
    }

    fn footnote_inlines(source: &str) -> Vec<Inline> {
        let parsed = parser::parse(source);
        parsed
            .blocks
            .iter()
            .flat_map(|block| match block {
                Block::Paragraph(inlines) => inlines.clone(),
                _ => Vec::new(),
            })
            .filter(|inline| matches!(inline, Inline::Footnote { .. }))
            .collect()
    }

    #[test]
    fn counter_follows_latex_for_marks_texts_and_optional_numbers() {
        let source =
            "a\\footnote{one} b\\footnotemark{} c\\footnotetext{two} d\\footnote[7]{seven} e\\footnote{three}";
        let parsed = parser::parse(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert!(parsed.document_global_state);
        let summary: Vec<(String, bool, bool)> = footnote_inlines(source)
            .into_iter()
            .map(|inline| match inline {
                Inline::Footnote {
                    number, mark, text, ..
                } => (number, mark, text.is_some()),
                _ => unreachable!(),
            })
            .collect();
        let expected = [
            ("1", true, true),
            ("2", true, false),
            ("2", false, true),
            ("7", true, true),
            ("3", true, true),
        ];
        assert_eq!(
            summary,
            expected
                .iter()
                .map(|(n, m, t)| (n.to_string(), *m, *t))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn mark_is_a_raised_script_glued_to_the_word_and_note_sits_at_the_bottom() {
        let source = "Body word\\footnote{Note with $x$ math.} after.";
        let output = compile(source);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let pages = &output.pages;
        assert_eq!(pages.len(), 1);
        let metrics = Metrics::for_body(12.0);
        let word = find(pages, source, "word").1;
        let mark_start = source.find("\\footnote").unwrap();
        let marks: Vec<&TextItem> = items(pages)
            .into_iter()
            .map(|(_, item)| item)
            .filter(|item| item.span.start == mark_start && item.rule.is_none())
            .collect();
        assert_eq!(marks.len(), 2, "running-text mark and footnote mark");
        let text_mark = marks[0];
        assert_eq!(text_mark.text, "1");
        assert_eq!(text_mark.font_size_pt, 8.0);
        assert_eq!(
            &source[text_mark.span.start..text_mark.span.end],
            "\\footnote"
        );
        let word_end = word.x_pt + glyph_width("word", 12.0, Font::TimesRoman);
        assert!((text_mark.x_pt - word_end).abs() < 0.02);
        assert_eq!(
            text_mark.baseline_y_pt,
            round2(word.baseline_y_pt - MARK_RAISE_EM * 12.0)
        );
        let after = find(pages, source, "after.").1;
        assert!(after.x_pt > text_mark.x_pt);

        let note = find(pages, source, "Note").1;
        assert_eq!(note.font_size_pt, 10.0);
        assert_eq!(note.baseline_y_pt, TEXT_BOTTOM_PT);
        assert_eq!(note.x_pt, round2(MARGIN_PT + MARK_BOX_EM * 10.0));
        let note_mark = marks[1];
        assert_eq!(note_mark.font_size_pt, metrics.note_mark_size);
        let mark_end = note_mark.x_pt + glyph_width("1", 7.0, Font::TimesRoman);
        assert!((mark_end - note.x_pt).abs() < 0.02);
        assert_eq!(
            note_mark.baseline_y_pt,
            round2(TEXT_BOTTOM_PT - MARK_RAISE_EM * 10.0)
        );

        let rule = items(pages)
            .into_iter()
            .find_map(|(_, item)| item.rule.filter(|_| item.span.start == mark_start))
            .expect("footnote rule");
        assert_eq!(rule.y_pt, round2(TEXT_BOTTOM_PT - 8.4 - 3.0));
        assert_eq!(rule.width_pt, round2(0.4 * 468.0));
        assert_eq!(rule.height_pt, 0.4);
        assert_eq!(find(pages, source, "x$").1.baseline_y_pt, TEXT_BOTTOM_PT);
        for (_, item) in items(pages) {
            let slice = &source[item.span.start..item.span.end];
            assert!(!slice.is_empty());
        }
    }

    #[test]
    fn ten_point_class_uses_footnotesize_eight_and_its_skips() {
        let source = "\\documentclass[10pt]{article}\n\\begin{document}\nA\\footnote{First.} B\\footnote{Second.}\n\\end{document}";
        let output = compile(source);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let pages = &output.pages;
        let first = find(pages, source, "First.").1;
        let second = find(pages, source, "Second.").1;
        assert_eq!(first.font_size_pt, 8.0);
        assert_eq!(second.baseline_y_pt, TEXT_BOTTOM_PT);
        assert_eq!(first.baseline_y_pt, TEXT_BOTTOM_PT - 9.5);
        assert_eq!(first.x_pt, round2(MARGIN_PT + 1.8 * 8.0));
        let mark = find(pages, source, "\\footnote{First").1;
        assert_eq!(mark.font_size_pt, 7.0);
        let rule = items(pages)
            .into_iter()
            .find_map(|(_, item)| item.rule)
            .expect("rule");
        assert_eq!(rule.y_pt, round2(TEXT_BOTTOM_PT - 9.5 - 6.65 - 3.0));
    }

    fn filler(lines: usize) -> String {
        // Each `\\`-separated word is its own 14.4pt body line.
        (0..lines)
            .map(|index| format!("line{index}"))
            .collect::<Vec<_>>()
            .join("\\\\ ")
    }

    #[test]
    fn body_text_leaves_room_for_the_page_footnotes() {
        let source = format!("Start\\footnote{{Short note.}}\\\\ {}", filler(60));
        let output = compile(&source);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let pages = &output.pages;
        assert_eq!(pages.len(), 2);
        let body_bottom = TEXT_BOTTOM_PT - 10.8 - 8.4;
        let note = find(pages, &source, "Short").1;
        assert_eq!(find(pages, &source, "Short").0, 0);
        let lowest_body = pages[0]
            .items
            .iter()
            .filter(|item| item.font_size_pt == 12.0)
            .map(|item| item.baseline_y_pt)
            .fold(0.0, f64::max);
        assert!(lowest_body <= body_bottom + 1e-9, "{lowest_body}");
        assert!(lowest_body > body_bottom - 14.4);
        assert_eq!(note.baseline_y_pt, TEXT_BOTTOM_PT);
        // The second page has no footnotes: its body may reach the bottom.
        assert!(pages[1].items.iter().all(|item| item.rule.is_none()));
    }

    #[test]
    fn wrapped_note_lines_are_justified_except_the_last() {
        let source = format!("Marked\\footnote{{{}end.}} tail", "word ".repeat(40));
        let output = compile(&source);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let metrics = Metrics::for_body(LayoutConstraints::default().font_size_pt);
        let mut lines: Vec<(f64, f64)> = Vec::new();
        for item in output.pages[0]
            .items
            .iter()
            .filter(|item| item.rule.is_none() && item.font_size_pt == metrics.size)
        {
            let end =
                item.x_pt + crate::layout::text_width(&item.text, item.font_size_pt, item.font);
            match lines.iter_mut().find(|(y, _)| *y == item.baseline_y_pt) {
                Some(line) => line.1 = line.1.max(end),
                None => lines.push((item.baseline_y_pt, end)),
            }
        }
        lines.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert!(lines.len() >= 3, "{lines:?}");
        let right = PAGE_WIDTH_PT - MARGIN_PT;
        let (last, wrapped) = lines.split_last().unwrap();
        for (y, end) in wrapped {
            assert!((end - right).abs() < 0.02, "line at {y} ends at {end}");
        }
        assert!(last.1 < right - 1.0, "last line {last:?}");
    }

    #[test]
    fn a_note_that_does_not_fit_splits_and_continues_on_the_next_page() {
        let long_note = "word ".repeat(120);
        let source = format!(
            "{}\\\\ Marked\\footnote{{{}end}} tail\\\\ {}",
            filler(40),
            long_note.trim_end(),
            filler(30)
        );
        let output = compile(&source);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let pages = &output.pages;
        let marked = find(pages, &source, "Marked");
        assert_eq!(marked.0, 0);
        let note_start = source.find("word").unwrap();
        let note_end = source.find("end}").unwrap();
        let note_pages: Vec<usize> = items(pages)
            .into_iter()
            .filter(|(_, item)| {
                item.span.start >= note_start
                    && item.span.end <= note_end + 3
                    && item.rule.is_none()
            })
            .map(|(page, _)| page)
            .collect();
        assert!(note_pages.contains(&0), "first part stays with its mark");
        assert!(note_pages.contains(&1), "remainder continues on page 2");
        // Words keep document order across the split.
        let starts: Vec<usize> = items(pages)
            .into_iter()
            .filter(|(_, item)| item.text == "word" && item.font_size_pt == 10.0)
            .map(|(_, item)| item.span.start)
            .collect();
        assert!(starts.windows(2).all(|pair| pair[0] < pair[1]));
        // 119 separate "word" items; the 120th is glued to "end".
        assert_eq!(starts.len(), 119);
        // Each page's notes end on the text-area bottom, below its body.
        for page in &pages[..2] {
            let note_bottom = page
                .items
                .iter()
                .filter(|item| item.font_size_pt == 10.0)
                .map(|item| item.baseline_y_pt)
                .fold(0.0, f64::max);
            assert_eq!(note_bottom, TEXT_BOTTOM_PT);
            let rule = page.items.iter().find_map(|item| item.rule).expect("rule");
            let body = page
                .items
                .iter()
                .filter(|item| item.font_size_pt == 12.0)
                .map(|item| item.baseline_y_pt)
                .fold(0.0, f64::max);
            assert!(
                body + 10.8 - 3.0 <= rule.y_pt + 1e-9,
                "{body} vs {}",
                rule.y_pt
            );
        }
        for (_, item) in items(pages) {
            if item.rule.is_none() && item.text != "1" {
                assert_eq!(&source[item.span.start..item.span.end], item.text);
            }
        }
    }

    #[test]
    fn a_note_marked_on_the_last_line_moves_whole_to_the_next_page() {
        let source = format!("{}\\\\ Marked\\footnote{{Moved note.}}", filler(44));
        let output = compile(&source);
        let pages = &output.pages;
        assert_eq!(find(pages, &source, "Marked").0, 0);
        let (page, note) = find(pages, &source, "Moved");
        assert_eq!(page, 1, "a notes-only page follows at the document end");
        assert_eq!(note.baseline_y_pt, TEXT_BOTTOM_PT);
        assert!(pages[0].items.iter().all(|item| item.rule.is_none()));
    }

    #[test]
    fn footnotes_keep_order_and_held_notes_precede_later_ones() {
        let source = format!(
            "{}\\\\ A\\footnote{{Held.}}\\\\ {}\\\\ B\\footnote{{Next.}}",
            filler(44),
            filler(3)
        );
        let output = compile(&source);
        let pages = &output.pages;
        let held = find(pages, &source, "Held.");
        let next = find(pages, &source, "Next.");
        assert_eq!((held.0, next.0), (1, 1));
        assert!(held.1.baseline_y_pt < next.1.baseline_y_pt);
        assert_eq!(next.1.baseline_y_pt, TEXT_BOTTOM_PT);
    }

    #[test]
    fn nested_footnote_text_is_diagnosed() {
        let output = compile("A\\footnote{outer\\footnote{inner}}");
        assert!(output
            .diagnostics
            .iter()
            .any(|d| d.message.contains("nested \\footnote")));
    }

    #[test]
    fn incremental_edits_match_a_fresh_layout() {
        let constraints = LayoutConstraints::default();
        let old = format!(
            "Intro\\footnote{{First note.}} text.\n\n{}\n\nLater\\footnote{{Second note.}} words.",
            filler(40)
        );
        let edits = [
            old.replace("First note.", "First note, now longer."),
            old.replace("Intro", "Intro edited"),
            old.replace("Later\\footnote{Second note.}", "Later"),
        ];
        let mut session = Session::new();
        let cold = session.compile(&old, constraints);
        assert_eq!(cold.output, compile_full(&old, constraints));
        for edit in &edits {
            let result = session.compile(edit, constraints);
            assert_eq!(result.output, compile_full(edit, constraints));
        }
    }
}
