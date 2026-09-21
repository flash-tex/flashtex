//! beamer frames and the title page (issue #944, Tier 0, 1 and 4), for the
//! default theme and Madrid. The geometry is `flashtex_class_geometry::beamer`'s;
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
//! Tier 4 (`[plain]`, `[allowframebreaks]`, the Madrid theme): a plain
//! frame's exit code `\vspace*{-\footheight}` widens the free height and
//! its body's first line has no interline glue; an `allowframebreaks`
//! frame is `\vsplit` at `0.95\textheight` ([`Context::autobreak_split`])
//! and its glue is finite (`plus .4\paperheight` / `.6\paperheight`,
//! [`resolve_autobreak`]); a theme paints the frametitle bar, the footline
//! boxes and the rounded title box as page chrome ([`page_chrome`]) from
//! the numbers in `flashtex_class_geometry::beamer::theme`.
//!
//! Measured (pdflatex TeX Live 2026, `tools/visual-oracle/pdftext.py`, bp
//! from the paper top): frametitle baseline 21.057 at x 8.504; first body
//! baseline 129.06 / 123.64 / 118.22 / 96.54 for a 1 / 2 / 3 / 7-line body
//! under a title, 42.01 with `[t]`, 109.97 without a title; the corpus
//! title page (`fixtures/real-world/beamer-default`) at 85.41 / 102.47 /
//! 140.02 / 162.49 / 188.55 (title, subtitle, author, institute, date);
//! `[plain]` two lines at 120.80; Madrid `Outline` at 20.061 over a
//! 27.569bp bar, footline text at 269.469.

use flashtex_class_geometry::beamer::{self as spec, FrameAlign};
use flashtex_compiler::color::{ColorSpace, DeviceColor};
use flashtex_compiler::Span;
use flashtex_paragraph_layout as pl;

use crate::adapter::{self, Item as AItem, ParaStyle, TextStyle};
use crate::pagebuild::{self, VBlock, VItem};
use crate::style::frame_pt;

use super::{empty_block, line_extents, page_params, plain_vblock, vskips_of, BuiltBlock, Context};

/// beamer's `structure` colour as the compiler's exact paint.
pub fn structure_color() -> DeviceColor {
    rgb_color(spec::STRUCTURE_RGB)
}

/// A theme's rgb triple as the compiler's exact paint.
pub fn rgb_color((r, g, b): spec::Rgb) -> DeviceColor {
    let bn = |v: f64| (v * 1e9).round() as u32;
    DeviceColor::from_billionths(ColorSpace::Rgb, &[bn(r), bn(g), bn(b)]).expect("rgb in range")
}

/// The head of a frame as the typesetter opens it (`Block::FrameBegin`).
pub struct FrameHead<'a> {
    pub title: &'a [AItem],
    pub subtitle: &'a [AItem],
    pub align: FrameAlign,
    pub plain: bool,
    pub allowframebreaks: bool,
    /// The frame's first slide steps `framenumber`; later slides of the
    /// same frame share its number.
    pub first_slide: bool,
    pub span: Span,
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
    /// Built-block index of the body's own `\vbox{}`.
    pub body_at: usize,
    pub align: FrameAlign,
    /// `[plain]`: the exit code's `-\footheight` (points), which the fills
    /// gain, and `\thispagestyle{empty}` (no footline on the page).
    pub plain: Option<f64>,
    /// `[allowframebreaks]`: the split rule and the finite autobreak skips.
    pub autobreak: Option<spec::AutoBreak>,
    /// Finite stretch after the last block (`\beamer@framebottomskipautobreak`).
    pub trailing_stretch: f64,
    /// The frame's title and subtitle, for a continuation's own head.
    pub title: Vec<AItem>,
    pub subtitle: Vec<AItem>,
    pub span: Span,
    /// Whether this page steps `framenumber` (a frame's first slide, or a
    /// continuation: `\beamer@continueautobreak` does `\refstepcounter`).
    pub first_slide: bool,
    /// The theme's painted frametitle bar: its height from the page top.
    pub bar: Option<f64>,
    /// The Madrid title page's rounded box: the built-block indices of the
    /// title line and of the last title-box line (the subtitle's).
    pub rounded_title: Option<(usize, usize)>,
    /// How many autobreak pages of this frame precede this one.
    pub continuation: usize,
}

impl OpenFrame {
    fn end_index(&self) -> usize {
        self.end.unwrap_or(self.start)
    }
}

/// An `AItem::Word` of `text` in `style`, every character pointing at
/// `span` (a theme's generated text: the `~~(` `)` of the footline, the
/// ` I` continuation suffix).
fn word_item(text: &str, span: Span, style: TextStyle) -> AItem {
    AItem::Word(adapter::Word {
        segments: vec![adapter::Segment {
            chars: text.chars().map(|_| adapter::CharSrc { document: span.document, start: span.start, end: span.end }).collect(),
            text: text.to_string(),
            style,
        }],
    })
}

/// `(items)`: the parentheses glued to the first and last word (one box,
/// as TeX sets `(FlashTeX)`), or words of their own around non-word items.
fn parenthesized(mut items: Vec<AItem>, span: Span, style: TextStyle) -> Vec<AItem> {
    let src = adapter::CharSrc { document: span.document, start: span.start, end: span.end };
    match items.first_mut() {
        // Into the word's own first segment, so the run stays one box.
        Some(AItem::Word(w)) if !w.segments.is_empty() => {
            let s = &mut w.segments[0];
            s.text.insert(0, '(');
            s.chars.insert(0, src);
        }
        _ => items.insert(0, word_item("(", span, style)),
    }
    match items.last_mut() {
        Some(AItem::Word(w)) if !w.segments.is_empty() => {
            let s = w.segments.last_mut().expect("non-empty");
            s.text.push(')');
            s.chars.push(src);
        }
        _ => items.push(word_item(")", span, style)),
    }
    items
}

/// `items` with every word painted in `color` (the theme's fg for a text
/// set on a coloured box).
fn recolored(items: &[AItem], color: DeviceColor) -> Vec<AItem> {
    items
        .iter()
        .cloned()
        .map(|item| match item {
            AItem::Word(mut w) => {
                for s in &mut w.segments {
                    s.style.color = Some(color);
                }
                AItem::Word(w)
            }
            other => other,
        })
        .collect()
}

/// How a contents entry is set (`beamerbasetoc.sty`
/// `\beamer@tocaction@show/shaded/hide`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TocShow {
    Show,
    Shaded,
    Hide,
}

impl TocShow {
    fn parse(word: &str) -> Option<TocShow> {
        match word.trim() {
            "show" => Some(TocShow::Show),
            "shaded" => Some(TocShow::Shaded),
            "hide" => Some(TocShow::Hide),
            _ => None,
        }
    }
}

/// `\tableofcontents[<options>]`'s styles (`beamerbasetoc.sty` 29-62):
/// `\beamer@toc@cs`/`os` for the current and the other sections, and
/// `css`/`oss`/`ooss` for the current subsection, the other subsections
/// of the current section and the subsections of other sections. Keys
/// this does not know (`pausesections`, `sections=..`, `firstsection`,
/// `part`, ...) are read past.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TocStyles {
    pub current_section: TocShow,
    pub other_section: TocShow,
    pub current_subsection: TocShow,
    pub other_subsection: TocShow,
    pub other_section_subsection: TocShow,
}

impl TocStyles {
    pub(super) fn parse(options: &str) -> TocStyles {
        let mut st = TocStyles {
            current_section: TocShow::Show,
            other_section: TocShow::Show,
            current_subsection: TocShow::Show,
            other_subsection: TocShow::Show,
            other_section_subsection: TocShow::Show,
        };
        for key in options.split(',') {
            let (name, value) = match key.split_once('=') {
                Some((n, v)) => (n.trim(), v.trim().trim_matches(|c| c == '{' || c == '}')),
                None => (key.trim(), ""),
            };
            match name {
                "currentsection" => {
                    st.sections("show/shaded");
                    st.subsections("show/show/shaded");
                }
                "currentsubsection" => st.subsections("show/shaded"),
                "hideallsubsections" => st.subsections("hide"),
                "hideothersubsections" => st.subsections("show/show/hide"),
                "sectionstyle" => st.sections(value),
                "subsectionstyle" => st.subsections(value),
                _ => {}
            }
        }
        st
    }

    /// `\beamer@toc@process`: `a` sets both, `a/b` each.
    fn sections(&mut self, spec: &str) {
        let parts: Vec<TocShow> = spec.split('/').filter_map(TocShow::parse).collect();
        if let Some(&a) = parts.first() {
            self.current_section = a;
            self.other_section = parts.get(1).copied().unwrap_or(a);
        }
    }

    /// `\beamer@toc@sprocess`: `a` sets all three, `a/b` the current one
    /// and both others, `a/b/c` each.
    fn subsections(&mut self, spec: &str) {
        let parts: Vec<TocShow> = spec.split('/').filter_map(TocShow::parse).collect();
        if let Some(&a) = parts.first() {
            let b = parts.get(1).copied().unwrap_or(a);
            self.current_subsection = a;
            self.other_subsection = b;
            self.other_section_subsection = parts.get(2).copied().unwrap_or(b);
        }
    }
}

/// `pct` of `c` over the white page (`colormixin{<pct>!parent.bg}`), in
/// `c`'s own space: `0.2 0.2 0.7 rg` becomes `0.84 0.84 0.94 rg`, black
/// `0.8 g`.
fn mix_into_white(c: DeviceColor, pct: f64) -> DeviceColor {
    const BILLION: f64 = 1e9;
    let white_is_zero = matches!(c.space, ColorSpace::Cmyk);
    let values: Vec<u32> = c
        .billionths()
        .iter()
        .map(|&v| {
            let white = if white_is_zero { 0.0 } else { BILLION };
            (f64::from(v) * pct + white * (1.0 - pct)).round() as u32
        })
        .collect();
    DeviceColor::from_billionths(c.space, &values).unwrap_or(c)
}

/// `items` in `color` where they set no colour of their own
/// (`\usebeamercolor[fg]{logo}` around `\insertlogo`: an explicit
/// `\color` inside the logo wins).
fn default_colored(items: &[AItem], color: DeviceColor) -> Vec<AItem> {
    items
        .iter()
        .cloned()
        .map(|item| match item {
            AItem::Word(mut w) => {
                for s in &mut w.segments {
                    s.style.color.get_or_insert(color);
                }
                AItem::Word(w)
            }
            other => other,
        })
        .collect()
}

impl<'a> Context<'a> {
    /// The document's beamer theme (the default one outside beamer).
    pub(super) fn beamer_theme(&self) -> spec::Theme {
        self.style.class_geometry.as_ref().map_or_else(|| spec::theme(spec::ThemeKind::Default), |g| g.beamer_theme)
    }

    /// The frame's opening blocks: the top `\vbox{}` (with the `\newpage`
    /// that starts the slide), the frametitle box when there is a title,
    /// and the body's own `\vbox{}` carrying the glue above the body.
    /// Returns the [`OpenFrame`] to collect the body into.
    pub(super) fn beamer_frame_begin(&mut self, blocks: &mut Vec<BuiltBlock>, head: &FrameHead<'_>) -> OpenFrame {
        self.beamer_frame_begin_at(blocks, head, 0)
    }

    /// [`Self::beamer_frame_begin`] for the `continuation`th autobreak page
    /// of the frame (0: the first, or a frame that never breaks).
    fn beamer_frame_begin_at(&mut self, blocks: &mut Vec<BuiltBlock>, head: &FrameHead<'_>, continuation: usize) -> OpenFrame {
        let theme = self.beamer_theme();
        let start = blocks.len();
        let mut top = plain_vblock(vec![(0.0, 0.0)]);
        top.penalty_before = Some(pagebuild::EJECT_PENALTY);
        blocks.push(empty_block(top));
        let mut bar = None;
        if !head.title.is_empty() {
            // `\insertframetitle` appends `\space` and the `frametitle
            // continuation` template (` I`, ` II`, ...) whenever
            // `\beamer@autobreakcount` is positive: on every page of an
            // `[allowframebreaks]` frame, the unbroken ones included.
            let suffixed;
            let title: &[AItem] = if head.allowframebreaks {
                let style = head.title.iter().find_map(|i| match i {
                    AItem::Word(w) => w.segments.first().map(|s| s.style),
                    _ => None,
                }).unwrap_or_default();
                let mut t = head.title.to_vec();
                t.push(AItem::Space { style, factor: 1000, no_break: false });
                t.push(word_item(&spec::continuation_suffix(continuation + 1), head.span, style));
                suffixed = t;
                &suffixed
            } else {
                head.title
            };
            let (built, painted) = self.frametitle_blocks(title, head.subtitle, head.span, &theme);
            blocks.extend(built);
            bar = painted;
        }
        let glue = spec::body_glue(head.align);
        let mut body = plain_vblock(vec![(0.0, 0.0)]);
        // `\nointerlineskip\box\beamer@zoombox\nointerlineskip`, then the
        // body's `\vskip-\parskip\vbox{}` (`\parskip` is 0).
        body.no_interline_first = true;
        let autobreak = if head.allowframebreaks {
            Some(spec::autobreak(head.align, flashtex_class_geometry::Sp((self.style.page_height_pt * 65536.0).round() as i64)))
        } else {
            None
        };
        let (fills, trailing_fill, trailing_stretch) = match autobreak {
            // Finite `plus` glue above the body (its stretch shares the
            // free height with the body's own and the bottom skip's).
            Some(a) => {
                body.space_before = Some((frame_pt(a.top), frame_pt(a.top_stretch), 0.0));
                (Vec::new(), 0.0, frame_pt(a.bottom_stretch))
            }
            None => {
                body.space_before = Some((frame_pt(glue.above), 0.0, 0.0));
                (vec![(blocks.len(), glue.above_fill)], glue.below_fill, 0.0)
            }
        };
        let body_at = blocks.len();
        blocks.push(empty_block(body));
        OpenFrame {
            start,
            fills,
            trailing_fill,
            end: None,
            body_at,
            align: head.align,
            plain: head.plain.then(|| frame_pt(spec::plain_frame(&theme).foot)),
            autobreak,
            trailing_stretch,
            title: head.title.to_vec(),
            subtitle: head.subtitle.to_vec(),
            span: head.span,
            first_slide: head.first_slide,
            bar,
            rounded_title: None,
            continuation,
        }
    }

    /// The outer theme's frametitle box as blocks
    /// (`flashtex_class_geometry::beamer::frametitle_box_themed` for the
    /// numbers): the title at `\Large` in the theme's frametitle colour,
    /// `\raggedright`, set `\paperwidth - 0.6cm` wide starting 0.3cm from
    /// the paper edge; the subtitle at `\footnotesize` under it. The lines
    /// carry the box's geometry: the first title line's height is its
    /// baseline's distance from the frame top (the `\lineskip` after the
    /// frame's `\vbox{}` and the colour box's own `sep - 1ex + (18 - 12.6)`
    /// are folded in), and the last line's depth closes the box
    /// (`\vskip-1ex`, `\vskip0.25em`). Returns the blocks and the painted
    /// bar's height when the theme's frametitle colour has a background.
    fn frametitle_blocks(&mut self, title: &[AItem], subtitle: &[AItem], span: Span, theme: &spec::Theme) -> (Vec<BuiltBlock>, Option<f64>) {
        let s = self.style;
        let paperwidth = flashtex_class_geometry::Sp((s.page_width_pt * 65536.0).round() as i64);
        let color = Some(rgb_color(theme.frametitle_fg));
        let title_style = TextStyle { color, ..TextStyle::default() };
        let (large, footnote) = (spec::LARGE, spec::FOOTNOTE);
        let (large_size, large_bs) = (frame_pt(large.size), frame_pt(large.baselineskip));
        let (foot_size, foot_bs) = (frame_pt(footnote.size), frame_pt(footnote.baselineskip));
        // Provisional geometry to learn the measure; the line counts fix
        // the real box.
        let Some(probe) = spec::frametitle_box(paperwidth, 1, 0) else { return (Vec::new(), None) };
        let width = frame_pt(probe.text_width);
        let hang = frame_pt(probe.text_left) - s.text_x_pt;
        let title = recolored(title, rgb_color(theme.frametitle_fg));
        let subtitle = recolored(subtitle, rgb_color(theme.frametitle_fg));
        let Some(mut title_block) = self.beamer_line(&title, large_size, title_style, ParaStyle::FlushLeft, width, hang, large_bs, span) else { return (Vec::new(), None) };
        let sub_style = TextStyle { color, ..TextStyle::default() };
        let mut sub_block = if subtitle.is_empty() { None } else { self.beamer_line(&subtitle, foot_size, sub_style, ParaStyle::FlushLeft, width, hang, foot_bs, span) };
        let title_lines = title_block.vertical.lines.len();
        let sub_lines = sub_block.as_ref().map_or(0, |b| b.vertical.lines.len());
        let Some(themed) = spec::frametitle_box_themed(paperwidth, title_lines, sub_lines, theme) else { return (Vec::new(), None) };
        let geometry = themed.geometry;
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
        (out, themed.bar_height.map(frame_pt))
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
    /// Under the rounded inner theme (Madrid) the title box is a
    /// `beamerboxesrounded` in the `title` colours: `above`/`below` more
    /// glue around it, white text, and the box painted as page chrome
    /// ([`OpenFrame::rounded_title`]).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn beamer_title_page(&mut self, blocks: &mut Vec<BuiltBlock>, frame: Option<&mut OpenFrame>, title: &[AItem], subtitle: &[AItem], authors: &[AItem], institute: &[AItem], date: &[AItem], span: Span) {
        let tp = spec::title_page();
        let theme = self.beamer_theme();
        let rounded = theme.title_page;
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
        let title_color = rounded.map_or(structure, |r| Some(rgb_color(r.fg)));
        let title_items = rounded.map_or_else(|| title.to_vec(), |r| recolored(title, rgb_color(r.fg)));
        let subtitle_items = rounded.map_or_else(|| subtitle.to_vec(), |r| recolored(subtitle, rgb_color(r.fg)));
        let boxes: [(&[AItem], f64, f64, TextStyle); 4] = [
            (&title_items, frame_pt(spec::LARGE.size), frame_pt(spec::LARGE.baselineskip), TextStyle { color: title_color, ..plain }),
            (authors, frame_pt(spec::NORMAL.size), frame_pt(spec::NORMAL.baselineskip), plain),
            (institute, frame_pt(spec::SCRIPT.size), frame_pt(spec::SCRIPT.baselineskip), plain),
            (date, frame_pt(spec::NORMAL.size), frame_pt(spec::NORMAL.baselineskip), plain),
        ];
        let mut rounded_title = None;
        for (k, (items, size, bs, style)) in boxes.iter().enumerate() {
            // Every `beamercolorbox[sep=8pt]` of the template is set whether
            // or not its content is empty: an unset `\institute` is still
            // an `\hbox(16.0+0.0)` (the `\leavevmode` line of no height
            // between the two `sep`s) under its `\lineskip` glue. Measured:
            // the corpus deck `beamer-blocks-columns` (no institute) puts
            // the title 7.53bp higher and the date 9.41bp lower than a
            // layout that drops the box.
            let built = if items.is_empty() { None } else { self.beamer_line(items, *size, *style, ParaStyle::Center, width, 0.0, *bs, span) };
            let mut b = built.unwrap_or_else(|| empty_block(plain_vblock(vec![(0.0, 0.0)])));
            b.vertical.no_interline_first = true;
            let mut before = lineskip + sep;
            if k == 0 {
                if let Some(r) = rounded {
                    before += frame_pt(r.above);
                }
            }
            b.vertical.space_before = Some((before, 0.0, 0.0));
            b.vertical.penalty_before = Some(pagebuild::INF_PENALTY);
            b.vertical.penalty_after = Some(pagebuild::INF_PENALTY);
            let mut after = sep;
            let mut sub = None;
            if k == 0 {
                if !subtitle_items.is_empty() {
                    let sub_style = TextStyle { color: rounded.map(|r| rgb_color(r.fg)), ..plain };
                    if let Some(mut sb) = self.beamer_line(&subtitle_items, frame_pt(spec::NORMAL.size), sub_style, ParaStyle::Center, width, 0.0, frame_pt(spec::NORMAL.baselineskip), span) {
                        // `\vskip0.25em` (title font), then ordinary
                        // interline glue at the body's `\baselineskip`.
                        b.vertical.space_after = Some((frame_pt(tp.subtitle_skip), 0.0, 0.0));
                        sb.vertical.penalty_before = Some(pagebuild::INF_PENALTY);
                        sb.vertical.penalty_after = Some(pagebuild::INF_PENALTY);
                        sub = Some(sb);
                    }
                }
                if let Some(r) = rounded {
                    after += frame_pt(r.below);
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
                    let first = blocks.len();
                    blocks.push(b);
                    blocks.push(sb);
                    if k == 0 && rounded.is_some() {
                        rounded_title = Some((first, first + 1));
                    }
                }
                None => {
                    b.vertical.space_after = Some((after, 0.0, 0.0));
                    let first = blocks.len();
                    blocks.push(b);
                    if k == 0 && rounded.is_some() {
                        rounded_title = Some((first, first));
                    }
                }
            }
        }
        if let Some(f) = frame {
            f.fills.extend(fills);
            // The closing `\vfill`.
            f.trailing_fill += 1.0;
            f.rounded_title = rounded_title;
        }
    }

    /// beamer's `\tableofcontents` in a frame (`beamerbasetoc.sty` 73-93
    /// and 113-150; `\showoutput` of a three-section probe): `\vspace*
    /// {-.5em}` (a zero `\hrule` that keeps `\prevdepth`, then the skip),
    /// and for every section `\vfill` -- beamer's, `0pt plus 1fill`, the
    /// same order as the frame's own `[c]` skips; `\vskip1.5em` instead
    /// when the subsections of other sections are hidden -- followed by
    /// the `section in toc` line (`\hbox{\vbox{<title>\par}}`, structure
    /// colour, `\normalsize`) under ordinary `\baselineskip` glue; each
    /// subsection is a `subsection in toc` paragraph (`\leavevmode
    /// \leftskip=1.5em <title>\par`, the default foreground) under
    /// ordinary interline glue; then a closing `\vfill`. The fills go into
    /// `frame`, whose resolver shares the free height between them and the
    /// `1fill`/`1.5fill` frame skips: five sections give 8.5 units.
    ///
    /// `options` shade or hide entries relative to `current` (`(\c@section,
    /// \c@subsection)` where the list stands; [`TocStyles`]). A shaded
    /// entry is its colour mixed 20% into the white page (`colormixin
    /// {20!parent.bg}`: `0.84 0.84 0.94 rg` for a section, `0.8 g` for a
    /// subsection); a hidden one sets nothing.
    ///
    /// Measured (probe deck `beamer-polish` p1): entries 35.086bp apart
    /// from 73.956bp. With subsections (`\AtBeginSection` probe,
    /// `\tableofcontents[currentsection]`, `\showoutput`): `Intro` at
    /// 96.679bp, its subsections at 110.228 and 123.777 and x 44.710
    /// (`\leftskip` 16.42503pt), `Results` 170.225, `Speed` 183.774.
    pub(super) fn beamer_toc(&mut self, blocks: &mut Vec<BuiltBlock>, frame: Option<&mut OpenFrame>, entries: &[adapter::BeamerTocEntry], current: (usize, usize), options: &str, span: Span) {
        let s = self.style;
        let width = s.text_width_pt;
        let size = frame_pt(spec::NORMAL.size);
        let bs = frame_pt(spec::NORMAL.baselineskip);
        let em = self.text_params(TextStyle::default(), size).quad;
        let styles = TocStyles::parse(options);
        let mut fills: Vec<(usize, f64)> = Vec::new();
        let mut first = true;
        for entry in entries {
            let in_current = entry.section == current.0;
            let (show, color, indent) = if entry.level == 1 {
                let st = if in_current { styles.current_section } else { styles.other_section };
                (st, structure_color(), 0.0)
            } else {
                // `\beamer@tocifnothide` tests `css`/`oss`, the action
                // then takes `css`, `oss` or `ooss`.
                let check = if in_current { styles.current_subsection } else { styles.other_subsection };
                let action = if !in_current {
                    styles.other_section_subsection
                } else if entry.subsection == current.1 {
                    styles.current_subsection
                } else {
                    styles.other_subsection
                };
                let st = if check == TocShow::Hide { TocShow::Hide } else { action };
                (st, DeviceColor::BLACK, 1.5 * em)
            };
            let color = match show {
                TocShow::Hide => continue,
                TocShow::Show => color,
                TocShow::Shaded => mix_into_white(color, 0.2),
            };
            let style = TextStyle { color: Some(color), ..TextStyle::default() };
            let items = recolored(&entry.items, color);
            let Some(mut b) = self.beamer_line(&items, size, style, ParaStyle::FlushLeft, width - indent, indent, bs, span) else { continue };
            // `\vspace*{-.5em}` before the first entry.
            let lead = if first { -0.5 * em } else { 0.0 };
            if entry.level == 1 && styles.other_section_subsection == TocShow::Hide {
                // `\vskip1.5em` in place of the section's `\vfill`.
                b.vertical.space_before = Some((lead + 1.5 * em, 0.0, 0.0));
            } else {
                b.vertical.space_before = Some((lead, 0.0, 0.0));
                if entry.level == 1 {
                    fills.push((blocks.len(), 1.0));
                }
            }
            b.vertical.penalty_before = Some(pagebuild::INF_PENALTY);
            b.vertical.penalty_after = Some(pagebuild::INF_PENALTY);
            blocks.push(b);
            first = false;
        }
        if let Some(f) = frame {
            f.fills.extend(fills);
            // The closing `\vfill`.
            f.trailing_fill += 1.0;
        }
    }

    /// `\end{frame}` of a `[plain]` frame (`beamerbaseframe.sty` 116): the
    /// body's `\vbox{}` is followed by `\nointerlineskip`, so the first
    /// body line sits without interline glue.
    pub(super) fn beamer_plain_body(blocks: &mut [BuiltBlock], frame: &OpenFrame) {
        if frame.plain.is_none() {
            return;
        }
        if let Some(first) = blocks.iter_mut().skip(frame.body_at + 1).take(frame.end_index().saturating_sub(frame.body_at)).find(|b| !b.vertical.lines.is_empty()) {
            first.vertical.no_interline_first = true;
        }
    }

    /// `[allowframebreaks]` (`beamerbaseframesize.sty` 212-243): the closed
    /// frame's vertical list is `\vsplit` to `factor × \textheight` at the
    /// last feasible break (TeX §970-977 `vert_break`: the least-cost
    /// break, later ones winning ties; `\@itempenalty` −51 before every
    /// item), the remainder is pruned to its first box under a
    /// `\splittopskip` of `\baselineskip` and set as a new frame after
    /// `\frametitle{<title> II}`, and so on until a remainder fits. Returns
    /// every page's [`OpenFrame`]; the blocks of the continuation pages are
    /// appended to `blocks` (the frame is `blocks`' tail when `\end{frame}`
    /// comes, so the order holds). Measured (corpus p4/p5): the 20-item list
    /// breaks after item 14.
    pub(super) fn autobreak_split(&mut self, blocks: &mut Vec<BuiltBlock>, frame: OpenFrame) -> Vec<OpenFrame> {
        let mut out = Vec::new();
        let mut cur = frame;
        let Some(auto) = cur.autobreak else {
            out.push(cur);
            return out;
        };
        let p = page_params(self.style);
        let limit = auto.factor * self.style.text_height_pt;
        let splittopskip = frame_pt(auto.splittopskip);
        // At most as many pages as blocks: every split moves at least one
        // box to the next page.
        for _ in 0..blocks.len().max(1) {
            let end = cur.end_index();
            let vb: Vec<VBlock> = blocks[cur.start..=end].iter().map(|b| b.vertical.clone()).collect();
            let list = pagebuild::vlist(&p, &vb);
            let items: Vec<bool> = blocks[cur.start..=end].iter().map(|b| b.recs.iter().flatten().any(|r| self.label_recs.contains(r))).collect();
            let Some(brk) = vert_break(&list, limit, &items) else {
                out.push(cur);
                return out;
            };
            // The last box before the break and the first after it.
            let prev = list[..brk].iter().rev().find_map(|v| match v {
                VItem::Box { payload, .. } => Some(*payload),
                _ => None,
            });
            let next = list[brk..].iter().find_map(|v| match v {
                VItem::Box { payload, height, .. } => Some((*payload, *height)),
                _ => None,
            });
            let (Some((pb, _)), Some(((nb, nl), first_height))) = (prev, next) else {
                // Nothing after the break: everything fits.
                out.push(cur);
                return out;
            };
            let (pb, nb) = (cur.start + pb, cur.start + nb);
            let mut moved: Vec<BuiltBlock> = Vec::new();
            if nb == pb {
                // A break inside a paragraph: its lines split in two.
                let (keep, rest) = split_block(&blocks[pb], nl);
                blocks[pb] = keep;
                moved.push(rest);
                moved.extend(blocks.drain(pb + 1..=end));
            } else {
                // The break sits in the glue after block `pb`: what the
                // break discards (`prune_page_top`) is that glue and the
                // next block's leading glue and penalties.
                let tail = &mut blocks[pb].vertical;
                tail.space_after = None;
                tail.pre_space_after = None;
                tail.penalty_after = None;
                moved.extend(blocks.drain(nb..=end));
                blocks.truncate(pb + 1);
            }
            // The remainder's first box sits under `\splittopskip` glue
            // (`prune_page_top`: `\splittopskip` less its height, at least 0).
            if let Some(first) = moved.first_mut() {
                let v = &mut first.vertical;
                v.penalty_before = None;
                v.parskip = None;
                v.no_interline_first = true;
                v.space_before = Some(((splittopskip - first_height).max(0.0), 0.0, 0.0));
            }
            let last = blocks.len() - 1;
            blocks[last].vertical.penalty_after = Some(pagebuild::EJECT_PENALTY);
            cur.end = Some(last);
            let (title, subtitle) = (cur.title.clone(), cur.subtitle.clone());
            let head = FrameHead {
                title: &title,
                subtitle: &subtitle,
                align: cur.align,
                plain: cur.plain.is_some(),
                allowframebreaks: true,
                first_slide: true,
                span: cur.span,
            };
            let continuation = cur.continuation + 1;
            out.push(cur);
            let mut next_frame = self.beamer_frame_begin_at(blocks, &head, continuation);
            blocks.extend(moved);
            let last = blocks.len() - 1;
            blocks[last].vertical.penalty_after = Some(pagebuild::EJECT_PENALTY);
            next_frame.end = Some(last);
            cur = next_frame;
        }
        out.push(cur);
        out
    }

    /// A filled disc of `radius` centred `(cx, cy)` above the box's
    /// reference point, as pgf writes a circle (four cubics, tangent
    /// factor 0.5523).
    fn disc(cx: f64, cy: f64, radius: f64, color: DeviceColor) -> super::Shape {
        use super::ShapeCmd::{Close, Cubic, Move};
        let (r, k) = (radius, 0.5522847 * radius);
        super::Shape {
            op: super::ShapeOp::Fill,
            commands: vec![
                Move(cx + r, cy),
                Cubic(cx + r, cy + k, cx + k, cy + r, cx, cy + r),
                Cubic(cx - k, cy + r, cx - r, cy + k, cx - r, cy),
                Cubic(cx - r, cy - k, cx - k, cy - r, cx, cy - r),
                Cubic(cx + k, cy - r, cx + r, cy - k, cx + r, cy),
                Close,
            ],
            color,
        }
    }

    /// The label of a Madrid (`items[ball]`) itemize item: `itemize item`
    /// `[ball]` (`beamerbaseauxtemplates.sty` 366), `\raise0.2pt` of an
    /// `\hbox{\pgfuseshading{bigsphere}}` in `item projected`'s colours:
    /// a `2 x 0.53ex` square (`\normalsize` ex) holding the radial
    /// shading, painted here as the flat disc `spec::ball` describes.
    /// Measured (Madrid p3): the shading XObject (`/BBox [0 0 5.139
    /// 5.139]`) at `1 0 0 1 22.133 162.312 cm`, its bottom 0.2pt above the
    /// item baseline (110.013bp) and its right edge `\labelsep` before the
    /// text (32.727bp).
    pub(super) fn beamer_ball_item(&mut self, span: Span, hidden: bool) -> super::NumberBox {
        const RAISE_PT: f64 = 0.2;
        let ball = spec::ball(spec::STRUCTURE_RGB);
        let (side, radius) = (frame_pt(ball.side), frame_pt(ball.radius));
        let shape = Self::disc(side / 2.0, RAISE_PT + side / 2.0, radius, rgb_color(ball.color));
        let (run, rec) = self.paths_box(span, side, side + RAISE_PT, 0.0, vec![shape], hidden);
        super::NumberBox { width: side, height: side + RAISE_PT, depth: 0.0, pieces: vec![(run, rec, 0.0)] }
    }

    /// The label of a Madrid (`items[ball]`) enumerate item: `enumerate
    /// item` `[ball]` (`beamerbaseauxtemplates.sty` 374-384), a
    /// `pgfpicture{-1ex}{-0.65ex}{1ex}{1ex}` (in the body font) holding
    /// the `bigsphere` shading and `\insertenumlabel` in `\tiny`, centred
    /// on the picture's origin and raised 0.5pt: the picture's baseline is
    /// its bounding box's bottom, so the number's baseline sits `0.65ex +
    /// 0.5pt - ht/2` above the item's. The ball is the `bigsphere` shading
    /// scaled 1.75 and centred on the origin (`\pgftext` centres its box):
    /// painted as the flat disc `spec::ball` describes, 1.75 x its radius,
    /// under the white number. Measured (corpus p4): `1` at x = 20.837bp,
    /// 1.688bp above the item baseline; the shading XObject drawn under
    /// `1 0 0 1 22.424 164.069 cm`, `-4.497 -4.497 cm`, `1.75 0 0 1.75 0 0
    /// cm` (a 8.993bp disc centred 3.152bp above the baseline).
    pub(super) fn beamer_ball_number(&mut self, text: &str, span: Span, hidden: bool) -> Option<super::NumberBox> {
        let ex = frame_pt(spec::SANS_BODY_EX);
        let tiny = frame_pt(spec::TINY.size);
        let ball = spec::ball(spec::STRUCTURE_RGB);
        let disc = Self::disc(ex, 0.65 * ex, 1.75 * frame_pt(ball.radius), rgb_color(ball.color));
        let (disc_run, disc_rec) = self.paths_box(span, 0.0, 1.65 * ex, 0.0, vec![disc], hidden);
        let digits = text.trim_end_matches('.');
        let seg = adapter::Segment {
            text: digits.to_string(),
            chars: digits.chars().map(|_| adapter::CharSrc { document: span.document, start: span.start, end: span.end }).collect(),
            style: TextStyle { color: Some(rgb_color(spec::WHITE)), hidden, ..TextStyle::default() },
        };
        let (mut run, rec) = self.text_box(&seg, tiny)?;
        let raise_pt = 0.65 * ex + 0.5 - (run.height - run.depth) / 2.0;
        if let super::BoxRec::Text { raise, height, depth, .. } = &mut self.recs[rec] {
            *raise = raise_pt;
            *height = run.height + raise_pt;
            *depth = (run.depth - raise_pt).max(0.0);
        }
        run.height += raise_pt;
        run.depth = (run.depth - raise_pt).max(0.0);
        let width = 2.0 * ex;
        let x = (width - run.width) / 2.0;
        Some(super::NumberBox { width, height: 1.65 * ex, depth: 0.0, pieces: vec![(disc_run, disc_rec, 0.0), (run, rec, x)] })
    }
}

/// TeX's `vert_break` (§970-977) over `list` for `\vsplit ... to h`:
/// the index of the break item (a glue after a non-discardable item, or a
/// penalty below 10000) of least cost, later ones winning ties. `items`
/// says which blocks are list items (`\@itempenalty` −51 at the glue
/// before them). `None` when the whole list fits at natural size (no
/// break needed) or has no legal break.
fn vert_break(list: &[VItem], h: f64, items: &[bool]) -> Option<usize> {
    let mut cur_height = 0.0f64;
    let mut prev_dp = 0.0f64;
    let mut stretch = 0.0f64;
    let mut shrink = 0.0f64;
    let mut fil = false;
    let mut best: Option<(usize, i64)> = None;
    let mut prev_box = false;
    let mut total_natural = 0.0f64;
    for (i, item) in list.iter().enumerate() {
        let candidate = match item {
            VItem::Glue { .. } if prev_box => Some(0i32),
            VItem::Penalty(p) if *p < pagebuild::INF_PENALTY => Some(*p),
            _ => None,
        };
        if let Some(pen) = candidate {
            let item_bonus = if list[i..].iter().find_map(|v| match v {
                VItem::Box { payload, .. } => Some(*payload),
                _ => None,
            })
            .is_some_and(|(b, l)| l == 0 && items.get(b).copied().unwrap_or(false))
            {
                -51
            } else {
                0
            };
            // §974: a list taller than `h` by no more than its shrink is
            // still a feasible break, at the badness of shrinking it.
            // Measured (probe deck `beamer-polish`, `[t,allowframebreaks]`):
            // the 14th item's bottom sits 1.47bp past `0.95\textheight`,
            // within the 13 `\itemsep`s' `minus 3pt`, and pdflatex still
            // breaks after it, as in the `[c]` corpus frame.
            let b = if cur_height < h {
                if fil {
                    0
                } else {
                    pagebuild::badness(h - cur_height, stretch)
                }
            } else if cur_height - h > shrink {
                pagebuild::AWFUL_BAD
            } else {
                pagebuild::badness(cur_height - h, shrink)
            };
            let cost = if b == pagebuild::AWFUL_BAD {
                b
            } else {
                let pen = i64::from(pen) + i64::from(item_bonus);
                if b < pagebuild::INF_BAD {
                    b + pen
                } else {
                    pagebuild::DEPLORABLE
                }
            };
            if cost == pagebuild::AWFUL_BAD {
                break;
            }
            if best.map_or(true, |(_, c)| cost <= c) {
                best = Some((i, cost));
            }
        }
        match item {
            VItem::Box { height, depth, .. } => {
                cur_height += prev_dp + height;
                prev_dp = *depth;
                prev_box = true;
                total_natural = cur_height + prev_dp;
            }
            VItem::Glue { width, stretch: st, shrink: sh, fil: f } => {
                cur_height += prev_dp + width;
                prev_dp = 0.0;
                stretch += st;
                shrink += sh;
                fil |= *f;
                prev_box = false;
                total_natural = cur_height;
            }
            VItem::Penalty(_) => prev_box = false,
        }
    }
    // The whole list fits (its last depth excluded, as at every break),
    // its shrink allowed for: the end of the list is a forced break
    // (§972 `pi := eject_penalty`) of cost `eject_penalty` whenever it is
    // not awful, so it wins over every earlier break -- no split.
    let _ = total_natural;
    if cur_height <= h + shrink + 1e-6 {
        return None;
    }
    // A break after the last box is no break at all.
    let last_box = list.iter().rposition(|v| matches!(v, VItem::Box { .. }))?;
    best.map(|(i, _)| i).filter(|&i| i < last_box)
}

/// `block` cut before line `at`: the lines before it keep the block's
/// leading skips and penalties, the rest its trailing ones. Both halves
/// keep the whole horizontal list (lines index into it by range).
fn split_block(block: &BuiltBlock, at: usize) -> (BuiltBlock, BuiltBlock) {
    let mut keep = block.clone();
    let mut rest = block.clone();
    let n = block.vertical.lines.len();
    let at = at.min(n);
    keep.block.lines.lines.truncate(at);
    rest.block.lines.lines.drain(..at);
    keep.vertical.lines.truncate(at);
    rest.vertical.lines.drain(..at);
    keep.vertical.vskip_after.truncate(at);
    if rest.vertical.vskip_after.len() > at {
        rest.vertical.vskip_after.drain(..at);
    } else {
        rest.vertical.vskip_after.clear();
    }
    keep.vertical.broken_penalty.truncate(at);
    if rest.vertical.broken_penalty.len() > at {
        rest.vertical.broken_penalty.drain(..at);
    } else {
        rest.vertical.broken_penalty.clear();
    }
    keep.vertical.line_penalty.retain(|(i, _)| *i < at);
    rest.vertical.line_penalty = rest.vertical.line_penalty.iter().filter(|(i, _)| *i >= at).map(|(i, p)| (i - at, *p)).collect();
    keep.vertical.space_after = None;
    keep.vertical.pre_space_after = None;
    keep.vertical.penalty_after = None;
    rest.vertical.space_before = None;
    rest.vertical.parskip = None;
    rest.vertical.penalty_before = None;
    rest.vertical.contributed = None;
    keep.vertical.contributed = None;
    rest.labels.clear();
    keep.cache_key = None;
    rest.cache_key = None;
    (keep, rest)
}

/// Turns every `fill` of a closed frame into rigid glue: the free height is
/// `\textheight` less the run's natural height (the last line's depth
/// included, as `\vbox to` counts it), shared by weight. A body taller than
/// the frame gets no glue (beamer overfills the box the same way).
/// `inserts` is the height footnotes take from the frame (`\skip\footins`,
/// the rule and the notes), which the fills must leave free. A `[plain]`
/// frame's `-\footheight` exit glue adds to the free height.
pub(super) fn resolve_fills(style: &crate::style::Stylesheet, blocks: &mut [BuiltBlock], frame: &OpenFrame, inserts: f64) {
    if frame.autobreak.is_some() {
        resolve_autobreak(style, blocks, frame, inserts);
        return;
    }
    let Some(end) = frame.end else { return };
    let (natural, _) = natural_and_stretch(style, blocks, frame.start, end);
    let total: f64 = frame.fills.iter().map(|(_, w)| w).sum::<f64>() + frame.trailing_fill;
    // A hair under the exact fit keeps floating-point rounding from making
    // the page builder see an overfull page at the frame's `\newpage`.
    let free = (style.text_height_pt - natural - inserts + frame.plain.unwrap_or(0.0) - 1e-6).max(0.0);
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

/// The frame's natural height (its last depth included) and the finite
/// stretch of its glue.
fn natural_and_stretch(style: &crate::style::Stylesheet, blocks: &[BuiltBlock], start: usize, end: usize) -> (f64, f64) {
    let p = page_params(style);
    let vb: Vec<VBlock> = blocks[start..=end].iter().map(|b| b.vertical.clone()).collect();
    let list = pagebuild::vlist(&p, &vb);
    let (_, mut natural) = pagebuild::natural_layout(&p, &list, true);
    // `natural_layout` leaves the last box's depth out; `\vbox to` counts it
    // (the frame's bottom glue follows it). Penalties after the box (the
    // frame's `\newpage`) are not glue, so the depth is still pending.
    for item in list.iter().rev() {
        match item {
            VItem::Penalty(_) => continue,
            VItem::Box { depth, .. } => natural += depth,
            VItem::Glue { .. } => {}
        }
        break;
    }
    let stretch = list.iter().map(|v| match v {
        VItem::Glue { stretch, fil: false, .. } => *stretch,
        _ => 0.0,
    }).sum();
    (natural, stretch)
}

/// An `[allowframebreaks]` page: `\vbox to\textheight` over finite glue
/// (`plus .4\paperheight` above the body, `plus .6\paperheight` below it,
/// the body's own `plus 2pt`s): every glue stretches by the same ratio
/// `free / total stretch` (TeX §676; the ratio may exceed 1). Measured
/// (corpus p4/p5): `glue set 0.04607` and `0.50691`.
fn resolve_autobreak(style: &crate::style::Stylesheet, blocks: &mut [BuiltBlock], frame: &OpenFrame, inserts: f64) {
    let Some(end) = frame.end else { return };
    let Some(auto) = frame.autobreak else { return };
    let (natural, stretch) = natural_and_stretch(style, blocks, frame.start, end);
    let free = style.text_height_pt - natural - inserts + frame.plain.unwrap_or(0.0) - 1e-6;
    if free <= 0.0 {
        return;
    }
    // `[t]`/`[b]`: a `1fill` skip takes the whole free height and no
    // finite glue stretches (TeX §659). A `[t]` page's fill is the
    // trailing skip, which nothing follows; a `[b]` page's is the glue
    // above the body. Measured (probe deck `beamer-polish` p10, `[t]`):
    // sharing the free height by weight put the continuation's first
    // baseline 0.02bp low (the `.5\paperheight` stretch's share).
    if auto.top_fill || auto.bottom_fill {
        if auto.top_fill {
            if let Some(v) = blocks.get_mut(frame.body_at).map(|b| &mut b.vertical) {
                let share = if auto.bottom_fill { free / 2.0 } else { free };
                v.space_before = Some(match v.space_before {
                    Some((n, _, _)) => (n + share, 0.0, 0.0),
                    None => (share, 0.0, 0.0),
                });
            }
        }
        return;
    }
    let total = stretch + frame.trailing_stretch;
    if total <= 0.0 {
        return;
    }
    let ratio = free / total;
    let set = |g: &mut Option<(f64, f64, f64)>| {
        if let Some((n, st, _)) = g {
            *n += ratio * *st;
            *g = Some((*n, 0.0, 0.0));
        }
    };
    for b in &mut blocks[frame.start..=end] {
        let v = &mut b.vertical;
        set(&mut v.space_before);
        set(&mut v.parskip);
        set(&mut v.pre_space_after);
        set(&mut v.space_after);
    }
}

/// A theme's page chrome on every frame page (`page_chrome` for the
/// standard classes): the frametitle bar under the title, the infolines
/// footline (three colour boxes with the short author `(institute)`, the
/// short title, and the short date + `n / N` frame number at their `\tiny`
/// baseline `1ex` above the paper bottom), the Madrid title page's
/// rounded title box and the rounded blocks (`beamerboxesrounded`: 4bp
/// corner arcs, the 2pt head/body transition fade as sixteen strips and the
/// `shadow=true` fade as 32 grey strokes -- [`rounded_head_shape`],
/// [`rounded_body_shape`], [`rounded_rect_shape`], [`transition_shapes`],
/// [`shadow_shapes`]). Bars and boxes go first on the page so the text
/// paints over them. `[plain]` pages get no footline
/// (`\thispagestyle{empty}`).
pub(super) fn page_chrome(ctx: &mut Context, blocks: &mut Vec<BuiltBlock>, pages: &mut pl::Pages, line_dx: &mut [Vec<f64>], frames: &[OpenFrame], deck: Option<&adapter::BeamerDeck>) {
    let theme = ctx.beamer_theme();
    let n_pages = pages.pages.len();
    if n_pages == 0 || frames.is_empty() {
        return;
    }
    let s = ctx.style;
    let text_x = s.text_x_pt;
    let mut first_page: Vec<Option<usize>> = vec![None; blocks.len()];
    for (pi, page) in pages.pages.iter().enumerate() {
        for l in &page.lines {
            if let Some(slot) = first_page.get_mut(l.paragraph) {
                slot.get_or_insert(pi);
            }
        }
    }
    let page_of = |b: usize| (b..first_page.len()).find_map(|i| first_page[i]);
    let total_frames = frames.iter().filter(|f| f.first_slide).count().max(1);
    let mut number = 0usize;
    let span = super::NO_SOURCE_SPAN;
    for frame in frames {
        if frame.first_slide {
            number += 1;
        }
        let Some(pi) = page_of(frame.start) else { continue };
        let mut front: Vec<(BuiltBlock, f64, f64)> = Vec::new();
        let mut back: Vec<(BuiltBlock, f64, f64)> = Vec::new();
        if let (Some(bar), Some(bg)) = (frame.bar, theme.frametitle_bg) {
            front.push((ctx.rule_block_colored(span, s.page_width_pt, bar, -text_x, Some(rgb_color(bg))), bar, 0.0));
        }
        if let Some((first, last)) = frame.rounded_title {
            if let Some(r) = theme.title_page {
                let sep = frame_pt(spec::title_page().sep);
                let top_line = pages.pages[pi].lines.iter().find(|l| l.paragraph == first && l.line == 0);
                let bottom_line = pages.pages[pi].lines.iter().filter(|l| l.paragraph == last).last();
                if let (Some(t), Some(b)) = (top_line, bottom_line) {
                    let top = t.baseline_y - t.height - sep - frame_pt(r.above) + frame_pt(r.inset);
                    let bottom = b.baseline_y + b.depth + sep + frame_pt(r.below) - frame_pt(r.inset);
                    // An empty head: `\bmb@prevheight` is `-4.5pt` and the
                    // body box starts `4bp + 1.5pt - 1pt` under the vbox
                    // top, i.e. 0.5pt under the head picture's origin,
                    // which is 3bp under the fill's top edge (no
                    // transition: `\vskip-0.5pt` is inside the `\else`).
                    let bp = |v: f64| v * 72.27 / 72.0;
                    let body_origin = bottom - bp(3.0);
                    let body_top = top + bp(3.0) + 0.5;
                    let box_height = frame_pt(spec::rounded_box_height(pt_sp(body_origin - body_top), spec::rounded_empty_head_height()));
                    let mut shapes = shadow_shapes(s.text_width_pt, bottom, body_origin, box_height);
                    shapes.push(rounded_rect_shape(s.text_width_pt, bottom - top, rgb_color(r.bg)));
                    front.push((ctx.paths_block(span, s.text_width_pt, bottom - top, bp(4.0) + 1.0, 0.0, shapes), bottom, 0.0));
                }
            }
        }
        // Rounded blocks on this page (`beamerbaseboxes.sty` 77-107,
        // 210-242): the shadow, the head fill (rounded top corners) from
        // 3bp above the head picture's origin to 2pt under the head box's
        // baseline, the body fill (rounded bottom corners) from there to
        // 3bp under the body box's baseline, both overhanging `\textwidth`
        // by 4bp, and the 2pt transition fade over the seam.
        let recs: Vec<super::beamer_blocks::RoundedBlockRec> = ctx.rounded_blocks.iter().filter(|r| r.title_at >= frame.start && r.title_at <= frame.end_index()).cloned().collect();
        for r in recs {
            let Some(last_at) = r.last_at else { continue };
            let title = pages.pages[pi].lines.iter().find(|l| l.paragraph == r.title_at && l.line == 0);
            let last = pages.pages[pi].lines.iter().filter(|l| l.paragraph == last_at).last();
            let (Some(t), Some(b)) = (title, last) else { continue };
            let bp = |v: f64| v * 72.27 / 72.0;
            let tr = spec::transition();
            // The placed line's height is the glyphs'; the head picture's
            // origin (`{0bp}{-\ht\bmb@box}`) is the head box's top, the
            // `\vskip4bp` above it.
            let head_origin = t.baseline_y - t.height;
            let head_baseline = t.baseline_y + r.head_raise;
            let head_bottom = head_baseline + 2.0;
            let trans_origin = head_baseline + frame_pt(tr.baseline_below_head);
            let body_top = trans_origin - frame_pt(tr.body_top_above);
            let body_origin = b.baseline_y + b.depth + 0.5;
            let body_bottom = body_origin + bp(3.0);
            let box_height = frame_pt(spec::rounded_box_height(pt_sp(body_origin - body_top), pt_sp(t.height + r.head_raise)));
            let w = s.text_width_pt;
            let mut shapes = shadow_shapes(w, body_bottom, body_origin, box_height);
            shapes.push(rounded_head_shape(w, body_bottom - head_origin, body_bottom - head_bottom, rgb_color(r.title_bg)));
            shapes.push(rounded_body_shape(w, body_bottom - head_bottom, rgb_color(r.body_bg)));
            shapes.extend(transition_shapes(w, body_bottom - trans_origin, &tr, r.title_bg, r.body_bg));
            let height = body_bottom - (head_origin - bp(3.0));
            front.push((ctx.paths_block(span, w, height, bp(4.0) + 1.0, 0.0, shapes), body_bottom, 0.0));
        }
        // The navigation symbol strip (`sidebar right` `[default]`), on
        // every page with a head/foot (`[plain]` has `\thispagestyle
        // {empty}`), unless the template was emptied.
        if frame.plain.is_none() && ctx.style.class_geometry.as_ref().is_some_and(|g| g.beamer_navigation_symbols) {
            let strip = spec::navigation_strip(&theme);
            let (width, height, depth) = (frame_pt(strip.width()), frame_pt(strip.height), frame_pt(strip.depth));
            let x = s.page_width_pt - frame_pt(strip.right_inset) - width;
            let baseline = s.page_height_pt - frame_pt(strip.baseline_above_bottom);
            let block = ctx.paths_block(span, width, height, depth, x - text_x, navigation_symbol_shapes(&strip));
            front.push((block, baseline, 0.0));
        }
        // `\logo{..}` (`sidebar right`), right-aligned above the
        // navigation symbols in the logo colour, on the same pages.
        if let (Some(deck), None) = (deck, frame.plain) {
            if !deck.logo.is_empty() {
                let nav = ctx.style.class_geometry.as_ref().is_some_and(|g| g.beamer_navigation_symbols);
                let fg = rgb_color(theme.logo_fg);
                let size = frame_pt(spec::logo_placement(&theme, nav, flashtex_class_geometry::Sp::ZERO).size);
                let (block, width, _, depth) = ctx.hbox_block(&default_colored(&deck.logo, fg), size);
                let at = spec::logo_placement(&theme, nav, pt_sp(depth));
                let x = s.page_width_pt - frame_pt(at.right_inset) - width;
                back.push((block, s.page_height_pt - frame_pt(at.baseline_above_bottom), x - text_x));
            }
        }
        if let (Some(foot), None) = (theme.footline, frame.plain) {
            let (ht, dp) = (frame_pt(foot.height), frame_pt(foot.depth));
            let size = frame_pt(foot.font.size);
            let baseline = s.page_height_pt - dp;
            let empty = adapter::BeamerDeck::default();
            let deck = deck.unwrap_or(&empty);
            let mut x0 = 0.0;
            for fb in &foot.boxes {
                let w = frame_pt(fb.width);
                let fg = rgb_color(fb.fg);
                front.push((ctx.rule_block_colored(span, w, ht + dp, x0 - text_x, Some(rgb_color(fb.bg))), s.page_height_pt, 0.0));
                let style = TextStyle::default();
                let line = match fb.content {
                    spec::FootContent::AuthorInstitute => {
                        let mut items = recolored(&deck.short_author, fg);
                        if !deck.short_institute.is_empty() {
                            // `~~(\insertshortinstitute)`: the parentheses
                            // join the institute's first and last words.
                            items.push(AItem::Space { style, factor: 1000, no_break: true });
                            items.push(AItem::Space { style, factor: 1000, no_break: true });
                            items.extend(parenthesized(recolored(&deck.short_institute, fg), span, TextStyle { color: Some(fg), ..style }));
                        }
                        let (block, width, _, _) = ctx.hbox_block(&items, size);
                        // `center`: `leftskip`/`rightskip` `0pt plus1fill`.
                        vec![(block, x0 + (w - width) / 2.0)]
                    }
                    spec::FootContent::Title => {
                        let items = recolored(&deck.short_title, fg);
                        let (block, width, _, _) = ctx.hbox_block(&items, size);
                        vec![(block, x0 + (w - width) / 2.0)]
                    }
                    spec::FootContent::DateFrameNumber => {
                        // `\hfill\insertshortdate{}\hfill<n \,/\, N>`, the
                        // number right-aligned in a `\makebox` as wide as
                        // `N\,/\,N`, between `leftskip` and `rightskip`.
                        let thin = 0.16667 * frame_pt(spec::TINY.size) * 1.062515;
                        let num_style = TextStyle { color: Some(fg), ..style };
                        let counter = |n: usize| -> Vec<AItem> {
                            vec![
                                word_item(&n.to_string(), span, num_style),
                                AItem::HSpace { pt: thin, stretch_pt: 0.0, shrink_pt: 0.0 },
                                word_item("/", span, num_style),
                                AItem::HSpace { pt: thin, stretch_pt: 0.0, shrink_pt: 0.0 },
                                word_item(&total_frames.to_string(), span, num_style),
                            ]
                        };
                        let (_, box_width, _, _) = ctx.hbox_block(&counter(total_frames), size);
                        let (num_block, num_width, _, _) = ctx.hbox_block(&counter(number), size);
                        let date_items = recolored(&deck.short_date, fg);
                        let (date_block, date_width, _, _) = ctx.hbox_block(&date_items, size);
                        let (left, right) = (x0 + frame_pt(fb.leftskip), x0 + w - frame_pt(fb.rightskip));
                        let fill = ((right - left) - date_width - box_width) / 2.0;
                        vec![(date_block, left + fill), (num_block, right - num_width)]
                    }
                };
                for (block, x) in line {
                    back.push((block, baseline, x - text_x));
                }
                x0 += w;
            }
        }
        // Bars first (behind the text), text after.
        for (block, baseline, dx) in front.into_iter().rev() {
            let l = &block.block.lines.lines[0];
            let placed = pl::PlacedLine { paragraph: blocks.len(), line: 0, baseline_y: baseline, height: l.height, depth: l.depth };
            pages.pages[pi].lines.insert(0, placed);
            line_dx[pi].insert(0, dx);
            blocks.push(block);
        }
        for (block, baseline, dx) in back {
            let l = &block.block.lines.lines[0];
            let placed = pl::PlacedLine { paragraph: blocks.len(), line: 0, baseline_y: baseline, height: l.height, depth: l.depth };
            pages.pages[pi].lines.push(placed);
            line_dx[pi].push(dx);
            blocks.push(block);
        }
    }
}

/// Points to `Sp` (the class-geometry unit).
fn pt_sp(pt: f64) -> flashtex_class_geometry::Sp {
    flashtex_class_geometry::Sp((pt * 65536.0).round() as i64)
}

/// A filled `beamerboxesrounded` head (`beamerbaseboxes.sty` 76-83): the
/// rectangle from `-4bp` to `\bmb@width + 4bp` whose top corners are 4bp
/// arcs, in a paths box whose origin is the text's left edge on the
/// fill's bottom edge, y up. `origin_y` is the head picture's origin
/// (the head box's top: the arcs run from 1bp under it, up to 3bp over
/// it), `bottom_y` the fill's lower edge (2pt under the head box's
/// baseline). Both in points above the box's baseline. Measured (probe,
/// `Plain block`): the path from `y = -1` to `3` and down to `-11.78925`
/// under `1 0 0 1 10.909 196.543 cm`, i.e. 72.583 to 87.372bp from the
/// page top for a title baseline of 83.885.
fn rounded_head_shape(width: f64, origin_y: f64, bottom_y: f64, color: DeviceColor) -> super::Shape {
    use super::ShapeCmd::{Close, Cubic, Line, Move};
    let r = frame_pt(spec::rounded_corner_radius());
    let k = frame_pt(spec::rounded_corner_tangent());
    let bp = |v: f64| v * 72.27 / 72.0;
    let (w, y) = (width, origin_y);
    super::Shape {
        op: super::ShapeOp::Fill,
        commands: vec![
            Move(-r, y - bp(1.0)),
            Cubic(-r, y + bp(1.2), -k, y + bp(3.0), 0.0, y + bp(3.0)),
            Line(w, y + bp(3.0)),
            Cubic(w + k, y + bp(3.0), w + r, y + bp(1.2), w + r, y - bp(1.0)),
            Line(w + r, bottom_y),
            Line(-r, bottom_y),
            Close,
        ],
        color,
    }
}

/// The body fill (`beamerbaseboxes.sty` 227-240): from `top_y` (where
/// the head fill ends; the seam is under the transition fade) down to
/// the box's baseline, 3bp under the body picture's origin, the bottom
/// corners 4bp arcs. Measured: `-4.00005 1.0 m -4.00005 -1.2 -2.20001
/// -3.00003 0.0 -3.00003 c 341.02087 -3.00003 l ... 345.02094 1.0 c
/// 345.02094 25.73714 l` under `cm 10.909 156.526`: 118.600bp from the
/// page top for a last body baseline of 112.980 (depth 2.129pt).
fn rounded_body_shape(width: f64, top_y: f64, color: DeviceColor) -> super::Shape {
    use super::ShapeCmd::{Close, Cubic, Line, Move};
    let r = frame_pt(spec::rounded_corner_radius());
    let k = frame_pt(spec::rounded_corner_tangent());
    let bp = |v: f64| v * 72.27 / 72.0;
    let w = width;
    super::Shape {
        op: super::ShapeOp::Fill,
        commands: vec![
            Move(-r, bp(4.0)),
            Cubic(-r, bp(1.8), -k, 0.0, 0.0, 0.0),
            Line(w, 0.0),
            Cubic(w + k, 0.0, w + r, bp(1.8), w + r, bp(4.0)),
            Line(w + r, top_y),
            Line(-r, top_y),
            Close,
        ],
        color,
    }
}

/// The Madrid title page's box: an empty-headed `beamerboxesrounded`
/// whose head path (`lower.bg`, `\ifdim\wd\bmb@box=0pt`) and body path
/// share the colour and overlap (the head fill ends 3.5pt under its
/// origin, the body fill starts 0.5pt under it), so the union is one
/// rectangle with four 4bp corner arcs, `height` tall above the box's
/// baseline. Measured (corpus `beamer-madrid` p1): head path `-4.00005
/// -1.0 m ... 345.02094 -3.48697 l` under `cm 10.909 209.368`, body path
/// `... 345.02094 47.5763 l` under `cm 10.909 161.294`: 59.758 to
/// 113.832bp from the page top.
fn rounded_rect_shape(width: f64, height: f64, color: DeviceColor) -> super::Shape {
    use super::ShapeCmd::{Close, Cubic, Line, Move};
    let r = frame_pt(spec::rounded_corner_radius());
    let k = frame_pt(spec::rounded_corner_tangent());
    let (w, h) = (width, height);
    super::Shape {
        op: super::ShapeOp::Fill,
        commands: vec![
            Move(-r, h - r),
            Cubic(-r, h - r + k, -k, h, 0.0, h),
            Line(w, h),
            Cubic(w + k, h, w + r, h - r + k, w + r, h - r),
            Line(w + r, r),
            Cubic(w + r, r - k, w + k, 0.0, w, 0.0),
            Line(0.0, 0.0),
            Cubic(-k, 0.0, -r, r - k, -r, r),
            Close,
        ],
        color,
    }
}

/// The `bmb@transition` fade (`spec::transition`) as flat strips over the
/// head/body seam: `origin_y` is the transition picture's baseline above
/// the box's baseline; strip `i` of `steps` spans its share of the 2pt
/// between `fade_bottom` and `fade_top` above that, in the colour mixed
/// at its midpoint (`upper` at the top, `lower` at the bottom), the
/// picture's `-4bp .. w+4bp` clip wide.
fn transition_shapes(width: f64, origin_y: f64, tr: &spec::Transition, upper: spec::Rgb, lower: spec::Rgb) -> Vec<super::Shape> {
    use super::ShapeCmd::{Close, Line, Move};
    let r = frame_pt(spec::rounded_corner_radius());
    let (bottom, top) = (origin_y + frame_pt(tr.fade_bottom), origin_y + frame_pt(tr.fade_top));
    let steps = tr.steps.max(1);
    let step = (top - bottom) / steps as f64;
    (0..steps)
        .map(|i| {
            let (y0, y1) = (bottom + step * i as f64, bottom + step * (i + 1) as f64);
            // 0 at the bottom (`lower.bg`), 1 at the top (`upper.bg`).
            let t = (i as f64 + 0.5) / steps as f64;
            let mix = |a: f64, b: f64| a + (b - a) * t;
            let color = rgb_color((mix(lower.0, upper.0), mix(lower.1, upper.1), mix(lower.2, upper.2)));
            super::Shape { op: super::ShapeOp::Fill, commands: vec![Move(-r, y0), Line(width + r, y0), Line(width + r, y1), Line(-r, y1), Close], color }
        })
        .collect()
}

/// The `shadow=true` drop shadow (`spec::shadow`): the box's right and
/// bottom edges (with the 4bp bottom-right arc) as one path from the
/// right band's cap centre, `cap_below_box_top` under the
/// `\bmb@boxheight` top, down to the bottom band's cap centre 4bp from
/// the left edge, stroked [`spec::shadow_rings`] times with round caps,
/// widest and lightest first: each ring's outer half is the fade at its
/// distance, the inner half is under the fills painted after it.
/// `bottom_y` is the box's baseline (the fill's bottom edge) and
/// `origin_y` the body picture's origin (3bp above it), both in points
/// above the paths box's baseline; `box_height` is `\bmb@boxheight`.
fn shadow_shapes(width: f64, bottom_y: f64, origin_y: f64, box_height: f64) -> Vec<super::Shape> {
    use super::ShapeCmd::{Cubic, Line, Move};
    let sh = spec::shadow();
    let r = frame_pt(spec::rounded_corner_radius());
    let k = frame_pt(spec::rounded_corner_tangent());
    // Local y up from the fill's bottom edge.
    let y0 = origin_y - bottom_y;
    let cap_y = y0 + box_height - frame_pt(sh.cap_below_box_top);
    let cap_x = frame_pt(sh.cap_from_left);
    let w = width;
    spec::shadow_rings(&sh)
        .into_iter()
        .map(|(stroke, grey)| super::Shape {
            op: super::ShapeOp::Stroke { width: frame_pt(stroke), round_cap: true },
            commands: vec![Move(w + r, cap_y), Line(w + r, r), Cubic(w + r, r - k, w + k, 0.0, w, 0.0), Line(cap_x, 0.0)],
            color: rgb_color((grey, grey, grey)),
        })
        .collect()
}

/// The six navigation symbols as [`super::Shape`]s in the strip's own
/// points (x from the strip's left edge, y up from the pictures' baseline):
/// picture `i` at `i x (20pt + gap)`, its pgf coordinates (bp) scaled to
/// points, the strong paths in `navigation symbols` and the light ones in
/// `navigation symbols dimmed`.
fn navigation_symbol_shapes(strip: &spec::NavigationStrip) -> Vec<super::Shape> {
    use super::{Shape, ShapeCmd, ShapeOp};
    const PT_PER_BP: f64 = 72.27 / 72.0;
    let pitch = frame_pt(strip.picture_width + strip.gap);
    let strong = rgb_color(spec::NAVIGATION_RGB);
    let dimmed = rgb_color(spec::NAVIGATION_DIMMED_RGB);
    let mut shapes = Vec::new();
    for (i, symbol) in spec::NAVIGATION_SYMBOLS.iter().enumerate() {
        let x0 = i as f64 * pitch;
        let px = |x: f64| x0 + x * PT_PER_BP;
        let py = |y: f64| y * PT_PER_BP;
        for path in symbol.iter() {
            let commands = path
                .commands
                .iter()
                .map(|c| match *c {
                    spec::NavCmd::Move(x, y) => ShapeCmd::Move(px(x), py(y)),
                    spec::NavCmd::Line(x, y) => ShapeCmd::Line(px(x), py(y)),
                    spec::NavCmd::Cubic(a, b, c, d, e, f) => ShapeCmd::Cubic(px(a), py(b), px(c), py(d), px(e), py(f)),
                    spec::NavCmd::Close => ShapeCmd::Close,
                })
                .collect();
            shapes.push(Shape {
                op: if path.fill { ShapeOp::Fill } else { ShapeOp::Stroke { width: path.line_width_pt, round_cap: path.round_cap } },
                commands,
                color: if path.dimmed { dimmed } else { strong },
            });
        }
    }
    shapes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structure_colour_is_the_measured_rgb() {
        assert_eq!(structure_color().components(), vec![0.2, 0.2, 0.7]);
        assert_eq!(rgb_color(spec::PALETTE_TERTIARY_BG).components(), vec![0.1, 0.1, 0.35]);
    }

    fn glue(w: f64, st: f64) -> VItem {
        VItem::Glue { width: w, stretch: st, shrink: 0.0, fil: false }
    }

    fn boxed(b: usize, l: usize, h: f64, d: f64) -> VItem {
        VItem::Box { height: h, depth: d, payload: (b, l) }
    }

    /// Items 16.6pt apart (7.6 high, 3pt `\itemsep plus 2pt`, baselineskip
    /// glue) under a 0pt plus 109pt top skip: the break falls at the glue
    /// after the last item whose top fits the limit, later item boundaries
    /// winning ties.
    #[test]
    fn vert_break_takes_the_last_fitting_item_boundary() {
        let mut list = vec![glue(0.0, 109.0)];
        let items = vec![true; 20];
        for i in 0..20 {
            list.push(glue(3.0, 2.0));
            list.push(glue(6.0, 0.0));
            list.push(boxed(i, 0, 7.6, 0.0));
        }
        // 14 items: 14 x 16.6 = 232.4; the 15th's box top would be 249.
        let brk = vert_break(&list, 240.0, &items).unwrap();
        // The break is the `\itemsep` glue right after item 13's box.
        assert!(matches!(list[brk], VItem::Glue { width, .. } if width == 3.0), "{:?}", list[brk]);
        let prev = list[..brk].iter().rev().find_map(|v| match v {
            VItem::Box { payload, .. } => Some(payload.0),
            _ => None,
        });
        assert_eq!(prev, Some(13));
        assert_eq!(vert_break(&list, 1000.0, &items), None, "everything fits");
    }

    #[test]
    fn continuation_suffix_is_appended() {
        assert_eq!(spec::continuation_suffix(3), "III");
    }
}
