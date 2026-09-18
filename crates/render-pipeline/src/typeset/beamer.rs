//! beamer frames and the title page (issue #944, Tier 0 and Tier 1), for
//! the **default** theme. The geometry is `flashtex_class_geometry::beamer`'s;
//! this module turns it into vertical-list blocks.
//!
//! A frame (`beamerbaseframe.sty` 118-162, `beamerbaseframesize.sty`
//! 242-256) is one `\vbox to\textheight`:
//!
//! ```text
//! \vbox{}                          the frame's top: a zero box
//! <frametitle box>                 frametitle_blocks (Tier 1)
//! \vskip 0pt plus 1fill            [c]: the body sits at 40% of the free height
//! \vbox{}  <body>                  the frame's material
//! \vskip 0pt plus 1.5fill
//! ```
//!
//! and the page's only material, so `\topskip` never acts (the stylesheet
//! sets it to 0 for beamer). The page builder has no stretchable vertical
//! glue: [`resolve_fills`] measures the run's natural height once every
//! block of the frame is built and turns each `fill` into a rigid skip, as
//! `Context::part_page_blocks` and the `titlepage` `abstract` do.
//!
//! Measured (pdflatex TeX Live 2026, `tools/visual-oracle/pdftext.py`, bp
//! from the paper top): frametitle baseline 21.057 at x 8.504; first body
//! baseline 129.06 / 123.64 / 118.22 / 96.54 for a 1 / 2 / 3 / 7-line body
//! under a title, 42.01 with `[t]`, 109.97 without a title; the corpus
//! title page (`fixtures/real-world/beamer-default`) at 85.41 / 102.47 /
//! 140.02 / 162.49 / 188.55 (title, subtitle, author, institute, date).

use flashtex_class_geometry::beamer::{self as spec, FrameAlign};
use flashtex_compiler::color::{ColorSpace, DeviceColor};
use flashtex_compiler::Span;
use flashtex_paragraph_layout as pl;

use crate::adapter::{Item as AItem, ParaStyle, TextStyle};
use crate::pagebuild::{self, VBlock};
use crate::style::frame_pt;

use super::{empty_block, line_extents, page_params, plain_vblock, vskips_of, BuiltBlock, Context};

/// beamer's `structure` colour as the compiler's exact paint.
pub fn structure_color() -> DeviceColor {
    let (r, g, b) = spec::STRUCTURE_RGB;
    let bn = |v: f64| (v * 1e9).round() as u32;
    DeviceColor::from_billionths(ColorSpace::Rgb, &[bn(r), bn(g), bn(b)]).expect("rgb in range")
}

/// A frame whose blocks are being collected (`Context::build`).
#[derive(Debug, Clone)]
pub struct OpenFrame {
    /// Built-block index of the frame's first block (the top `\vbox{}`).
    pub start: usize,
    /// `(block index, fill weight)`: `plus <weight>fill` glue standing right
    /// before that block, resolved into its `space_before`.
    pub fills: Vec<(usize, f64)>,
    /// Fill weight after the last block (`\beamer@framebottomskip`, the
    /// title page's closing `\vfill`).
    pub trailing_fill: f64,
    /// Built-block index of the frame's last block, once `\end{frame}` came.
    pub end: Option<usize>,
}

impl<'a> Context<'a> {
    /// The frame's opening blocks: the top `\vbox{}` (with the `\newpage`
    /// that starts the slide), the frametitle box when there is a title,
    /// and the body's own `\vbox{}` carrying the glue above the body.
    /// Returns the [`OpenFrame`] to collect the body into.
    pub(super) fn beamer_frame_begin(&mut self, blocks: &mut Vec<BuiltBlock>, title: &[AItem], subtitle: &[AItem], align: FrameAlign, span: Span) -> OpenFrame {
        let start = blocks.len();
        let mut top = plain_vblock(vec![(0.0, 0.0)]);
        top.penalty_before = Some(pagebuild::EJECT_PENALTY);
        blocks.push(empty_block(top));
        if !title.is_empty() {
            blocks.extend(self.frametitle_blocks(title, subtitle, span));
        }
        let glue = spec::body_glue(align);
        let mut body = plain_vblock(vec![(0.0, 0.0)]);
        // `\nointerlineskip\box\beamer@zoombox\nointerlineskip`, then the
        // body's `\vskip-\parskip\vbox{}` (`\parskip` is 0).
        body.no_interline_first = true;
        body.space_before = Some((frame_pt(glue.above), 0.0, 0.0));
        let body_at = blocks.len();
        blocks.push(empty_block(body));
        OpenFrame { start, fills: vec![(body_at, glue.above_fill)], trailing_fill: glue.below_fill, end: None }
    }

    /// The default outer theme's frametitle box as blocks
    /// (`flashtex_class_geometry::beamer::frametitle_box` for the numbers):
    /// the title at `\Large` in the structure colour, `\raggedright`, set
    /// `\paperwidth - 0.6cm` wide starting 0.3cm from the paper edge; the
    /// subtitle at `\footnotesize` under it. The lines carry the box's
    /// geometry: the first title line's height is its baseline's distance
    /// from the frame top (the `\lineskip` after the frame's `\vbox{}` and
    /// the colour box's own `sep - 1ex + (18 - 12.6)` are folded in), and
    /// the last line's depth closes the box (`\vskip-1ex`, `\vskip0.25em`).
    fn frametitle_blocks(&mut self, title: &[AItem], subtitle: &[AItem], span: Span) -> Vec<BuiltBlock> {
        let s = self.style;
        let paperwidth = s.page_width_pt;
        let color = Some(structure_color());
        let title_style = TextStyle { color, ..TextStyle::default() };
        let (large, footnote) = (spec::LARGE, spec::FOOTNOTE);
        let (large_size, large_bs) = (frame_pt(large.size), frame_pt(large.baselineskip));
        let (foot_size, foot_bs) = (frame_pt(footnote.size), frame_pt(footnote.baselineskip));
        // Provisional geometry to learn the measure; the line counts fix
        // the real box.
        let Some(probe) = spec::frametitle_box(flashtex_class_geometry::Sp((paperwidth * 65536.0).round() as i64), 1, 0) else { return Vec::new() };
        let width = frame_pt(probe.text_width);
        let hang = frame_pt(probe.text_left) - s.text_x_pt;
        let Some(mut title_block) = self.beamer_line(title, large_size, title_style, ParaStyle::FlushLeft, width, hang, large_bs, span) else { return Vec::new() };
        let sub_style = TextStyle { color, ..TextStyle::default() };
        let mut sub_block = if subtitle.is_empty() { None } else { self.beamer_line(subtitle, foot_size, sub_style, ParaStyle::FlushLeft, width, hang, foot_bs, span) };
        let title_lines = title_block.vertical.lines.len();
        let sub_lines = sub_block.as_ref().map_or(0, |b| b.vertical.lines.len());
        let Some(geometry) = spec::frametitle_box(flashtex_class_geometry::Sp((paperwidth * 65536.0).round() as i64), title_lines, sub_lines) else { return Vec::new() };
        let box_height = frame_pt(geometry.height);
        let title_strut = (0.7 * large_bs, 0.3 * large_bs);
        let sub_strut = (0.7 * foot_bs, 0.3 * foot_bs);
        // Every title line is `\strut`-tall; the first one's height reaches
        // up to the frame top.
        let mut consumed = 0.0;
        for (i, line) in title_block.vertical.lines.iter_mut().enumerate() {
            *line = if i == 0 { (frame_pt(geometry.title_baseline), title_strut.1) } else { title_strut };
            consumed += line.0 + line.1;
        }
        title_block.vertical.no_interline_first = true;
        title_block.vertical.baselineskip = Some(large_bs);
        title_block.vertical.penalty_before = None;
        title_block.vertical.space_before = None;
        title_block.vertical.parskip = None;
        title_block.vertical.penalty_after = Some(pagebuild::INF_PENALTY);
        title_block.vertical.space_after = None;
        if let Some(sub) = sub_block.as_mut() {
            for (i, line) in sub.vertical.lines.iter_mut().enumerate() {
                // `\lineskip` (1pt) before the subtitle's first line.
                *line = if i == 0 { (sub_strut.0 + 1.0, sub_strut.1) } else { sub_strut };
                consumed += line.0 + line.1;
            }
            sub.vertical.no_interline_first = true;
            sub.vertical.baselineskip = Some(foot_bs);
            sub.vertical.penalty_before = Some(pagebuild::INF_PENALTY);
            sub.vertical.space_before = None;
            sub.vertical.parskip = None;
            sub.vertical.penalty_after = Some(pagebuild::INF_PENALTY);
            sub.vertical.space_after = None;
        }
        // The box closes with `\vskip-1ex`, `\vskip-.3cm` + `sep` and the
        // frame's `\vskip0.25em`: the last line's depth absorbs the rest.
        let last = sub_block.as_mut().unwrap_or(&mut title_block);
        if let Some(line) = last.vertical.lines.last_mut() {
            line.1 += box_height - consumed;
        }
        let mut out = vec![title_block];
        out.extend(sub_block);
        out
    }

    /// One paragraph of `items` at `size` in `style`, `width` wide, its
    /// lines starting `hang` (may be negative) from the text edge, broken
    /// as `para` says. The vertical block is plain (no skips or penalties);
    /// callers set the geometry.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn beamer_line(&mut self, items: &[AItem], size: f64, style: TextStyle, para: ParaStyle, width: f64, hang: f64, baselineskip: f64, _span: Span) -> Option<BuiltBlock> {
        let (list, recs, labels, skips) = self.hlist(items, size, style, para);
        if !list.iter().any(|i| matches!(i, pl::Item::Box(_))) {
            return None;
        }
        let mut params = self.line_params(false, baselineskip, para, hang);
        params.line_width = width;
        let lines = self.break_paragraph(&list, &params, items, Some(&recs))?;
        let mut vertical = plain_vblock(line_extents(&lines));
        vertical.baselineskip = Some(baselineskip);
        vertical.vskip_after = vskips_of(&lines, &skips);
        vertical.interline_penalty = pagebuild::INF_PENALTY;
        Some(BuiltBlock { block: pl::ParagraphBlock::body(lines), items: list, recs, vertical, labels, cache_key: None })
    }

    /// beamer's `\titlepage` (`beamerinnerthemedefault.sty` `title page`,
    /// `flashtex_class_geometry::beamer::title_page` for the skips): the
    /// template's `\vbox{}`, `\vfill`, the centred `sep=8pt` boxes for the
    /// title (+ subtitle), author, institute and date, `\vskip0.5em`,
    /// `\vfill`. The fills go into `frame` when the page is inside one.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn beamer_title_page(&mut self, blocks: &mut Vec<BuiltBlock>, frame: Option<&mut OpenFrame>, title: &[AItem], subtitle: &[AItem], authors: &[AItem], institute: &[AItem], date: &[AItem], span: Span) {
        let tp = spec::title_page();
        let (sep, lineskip) = (frame_pt(tp.sep), frame_pt(tp.lineskip));
        let s = self.style;
        let width = s.text_width_pt;
        let mut fills: Vec<(usize, f64)> = Vec::new();
        // `\vbox{}`: `\baselineskip` glue from the body's own `\vbox{}`.
        blocks.push(empty_block(plain_vblock(vec![(0.0, 0.0)])));
        // `\vfill` before the title box.
        fills.push((blocks.len(), 1.0));
        let structure = Some(structure_color());
        let plain = TextStyle::default();
        let boxes: [(&[AItem], f64, f64, TextStyle); 4] = [
            (title, frame_pt(spec::LARGE.size), frame_pt(spec::LARGE.baselineskip), TextStyle { color: structure, ..plain }),
            (authors, frame_pt(spec::NORMAL.size), frame_pt(spec::NORMAL.baselineskip), plain),
            (institute, frame_pt(spec::SCRIPT.size), frame_pt(spec::SCRIPT.baselineskip), plain),
            (date, frame_pt(spec::NORMAL.size), frame_pt(spec::NORMAL.baselineskip), plain),
        ];
        for (k, (items, size, bs, style)) in boxes.iter().enumerate() {
            if items.is_empty() {
                continue;
            }
            let Some(mut b) = self.beamer_line(items, *size, *style, ParaStyle::Center, width, 0.0, *bs, span) else { continue };
            b.vertical.no_interline_first = true;
            b.vertical.space_before = Some((lineskip + sep, 0.0, 0.0));
            b.vertical.penalty_before = Some(pagebuild::INF_PENALTY);
            b.vertical.penalty_after = Some(pagebuild::INF_PENALTY);
            let mut after = sep;
            let mut sub = None;
            if k == 0 {
                if !subtitle.is_empty() {
                    if let Some(mut sb) = self.beamer_line(subtitle, frame_pt(spec::NORMAL.size), plain, ParaStyle::Center, width, 0.0, frame_pt(spec::NORMAL.baselineskip), span) {
                        // `\vskip0.25em` (title font), then ordinary
                        // interline glue at the body's `\baselineskip`.
                        b.vertical.space_after = Some((frame_pt(tp.subtitle_skip), 0.0, 0.0));
                        sb.vertical.penalty_before = Some(pagebuild::INF_PENALTY);
                        sb.vertical.penalty_after = Some(pagebuild::INF_PENALTY);
                        sub = Some(sb);
                    }
                }
                // `\vskip1em\par` after the title box.
                after += frame_pt(tp.after_title);
            }
            if k == 3 {
                // `\vskip0.5em` after the date box.
                after += frame_pt(tp.after_date);
            }
            match sub {
                Some(mut sb) => {
                    sb.vertical.space_after = Some((after, 0.0, 0.0));
                    blocks.push(b);
                    blocks.push(sb);
                }
                None => {
                    b.vertical.space_after = Some((after, 0.0, 0.0));
                    blocks.push(b);
                }
            }
        }
        match frame {
            Some(f) => {
                f.fills.extend(fills);
                // The closing `\vfill`.
                f.trailing_fill += 1.0;
            }
            None => {}
        }
    }
}

/// Turns every `fill` of a closed frame into rigid glue: the free height is
/// `\textheight` less the run's natural height (the last line's depth
/// included, as `\vbox to` counts it), shared by weight. A body taller than
/// the frame gets no glue (beamer overfills the box the same way).
/// `inserts` is the height footnotes take from the frame (`\skip\footins`,
/// the rule and the notes), which the fills must leave free.
pub(super) fn resolve_fills(style: &crate::style::Stylesheet, blocks: &mut [BuiltBlock], frame: &OpenFrame, inserts: f64) {
    let Some(end) = frame.end else { return };
    let p = page_params(style);
    let vb: Vec<VBlock> = blocks[frame.start..=end].iter().map(|b| b.vertical.clone()).collect();
    let list = pagebuild::vlist(&p, &vb);
    let (_, mut natural) = pagebuild::natural_layout(&p, &list, true);
    // `natural_layout` leaves the last box's depth out; `\vbox to` counts it
    // (the frame's bottom glue follows it). Penalties after the box (the
    // frame's `\newpage`) are not glue, so the depth is still pending.
    for item in list.iter().rev() {
        match item {
            pagebuild::VItem::Penalty(_) => continue,
            pagebuild::VItem::Box { depth, .. } => natural += depth,
            pagebuild::VItem::Glue { .. } => {}
        }
        break;
    }
    let total: f64 = frame.fills.iter().map(|(_, w)| w).sum::<f64>() + frame.trailing_fill;
    // A hair under the exact fit keeps floating-point rounding from making
    // the page builder see an overfull page at the frame's `\newpage`.
    let free = (style.text_height_pt - natural - inserts - 1e-6).max(0.0);
    if total <= 0.0 {
        return;
    }
    let unit = free / total;
    for &(at, weight) in &frame.fills {
        if weight <= 0.0 {
            continue;
        }
        if let Some(b) = blocks.get_mut(at) {
            let v = &mut b.vertical;
            v.space_before = Some(match v.space_before {
                Some((n, st, sh)) => (n + unit * weight, st, sh),
                None => (unit * weight, 0.0, 0.0),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structure_colour_is_the_measured_rgb() {
        assert_eq!(structure_color().components(), vec![0.2, 0.2, 0.7]);
    }
}
