//! The block style tree: `document -> section -> paragraph -> inline`, with
//! measured inheritance and article's sectioning and list spacing.

use std::fmt;

use crate::fonts::{
    BaseSize, FontParams, PARSKIP, SectionAfter, SizeName, font_size, list_level, section_spec,
    size_params,
};
use crate::geometry::{ClassOptions, Geometry, PageLayout, apply_geometry, article_page_params};
use crate::length::{Pt, Skip};

/// Maximum list/alignment nesting depth accepted by [`Stylesheet::try_resolve`].
///
/// Six mirrors real LaTeX, which errors with "Too deeply nested" past six
/// levels. It also covers every distinctly styled row of [`list_level`]:
/// margins are distinct for depths 1–4 and uniform `1em` from depth 5 on,
/// while vertical skips stop changing past depth 3 — so clamping deeper
/// nesting to this level reuses the table's last row with no styling loss.
pub const MAX_LIST_NESTING_DEPTH: usize = 6;

/// Error returned by [`Stylesheet::try_resolve`] when a block path nests
/// more [`Block::List`]/[`Block::Align`] environments than
/// [`MAX_LIST_NESTING_DEPTH`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListNestingTooDeep {
    /// The requested nesting depth (number of list/alignment nodes in the path).
    pub depth: usize,
}

impl fmt::Display for ListNestingTooDeep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "list nesting too deep: depth {} exceeds maximum {}",
            self.depth, MAX_LIST_NESTING_DEPTH
        )
    }
}

impl std::error::Error for ListNestingTooDeep {}

/// Horizontal alignment of lines in a block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Alignment {
    Justified,
    Left,
    Center,
    Right,
}

impl Alignment {
    pub fn name(self) -> &'static str {
        match self {
            Alignment::Justified => "justified",
            Alignment::Left => "left",
            Alignment::Center => "center",
            Alignment::Right => "right",
        }
    }
    pub fn parse(s: &str) -> Option<Alignment> {
        Some(match s {
            "justified" => Alignment::Justified,
            "left" => Alignment::Left,
            "center" => Alignment::Center,
            "right" => Alignment::Right,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListKind {
    Itemize,
    Enumerate,
    Description,
}

impl ListKind {
    pub fn name(self) -> &'static str {
        match self {
            ListKind::Itemize => "itemize",
            ListKind::Enumerate => "enumerate",
            ListKind::Description => "description",
        }
    }
}

/// Inline style switches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InlineStyle {
    /// `\emph`: toggles italic.
    Emph,
    /// `\textbf` / `\bfseries`.
    Bold,
    /// `\textit` / `\itshape`.
    Italic,
    /// A size command such as `\small` or `\Large`.
    Size(SizeName),
}

/// One node of a block path. A path is the chain of enclosing blocks from the
/// document root down to the block being resolved, for example
/// `[Document, Section(1), Paragraph]` or `[Document, List(Itemize), Item, Paragraph]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Block {
    Document,
    /// A sectioning container at the given level (1 = `\section`). It inherits
    /// body style; the heading text itself is `Heading(level)`.
    Section(u8),
    /// The heading block of `\section` (1) .. `\subparagraph` (5).
    Heading(u8),
    /// A body paragraph.
    Paragraph,
    /// The first paragraph after a `\section`..`\subsubsection` heading
    /// (`\@afterheading`: no first-line indent).
    ParagraphAfterHeading,
    /// A list environment; nesting depth is the number of `List` nodes in the path.
    List(ListKind),
    /// One `\item`.
    Item,
    /// `center`, `flushleft`, `flushright` (`\trivlist` based).
    Align(Alignment),
    Inline(InlineStyle),
}

impl Block {
    /// A stable textual name for JSON and diagnostics.
    pub fn name(self) -> String {
        match self {
            Block::Document => "document".into(),
            Block::Section(l) => format!("section{l}"),
            Block::Heading(l) => format!("heading{l}"),
            Block::Paragraph => "paragraph".into(),
            Block::ParagraphAfterHeading => "paragraph-after-heading".into(),
            Block::List(k) => k.name().into(),
            Block::Item => "item".into(),
            Block::Align(a) => format!("align-{}", a.name()),
            Block::Inline(InlineStyle::Emph) => "emph".into(),
            Block::Inline(InlineStyle::Bold) => "bold".into(),
            Block::Inline(InlineStyle::Italic) => "italic".into(),
            Block::Inline(InlineStyle::Size(s)) => format!("size-{}", s.command()),
        }
    }

    pub fn parse(s: &str) -> Option<Block> {
        Some(match s {
            "document" => Block::Document,
            "paragraph" => Block::Paragraph,
            "paragraph-after-heading" => Block::ParagraphAfterHeading,
            "itemize" => Block::List(ListKind::Itemize),
            "enumerate" => Block::List(ListKind::Enumerate),
            "description" => Block::List(ListKind::Description),
            "item" => Block::Item,
            "emph" => Block::Inline(InlineStyle::Emph),
            "bold" => Block::Inline(InlineStyle::Bold),
            "italic" => Block::Inline(InlineStyle::Italic),
            _ => {
                if let Some(l) = s.strip_prefix("section") {
                    return l.parse().ok().map(Block::Section);
                }
                if let Some(l) = s.strip_prefix("heading") {
                    return l.parse().ok().map(Block::Heading);
                }
                if let Some(a) = s.strip_prefix("align-") {
                    return Alignment::parse(a).map(Block::Align);
                }
                if let Some(n) = s.strip_prefix("size-") {
                    return SizeName::parse(n).map(|n| Block::Inline(InlineStyle::Size(n)));
                }
                return None;
            }
        })
    }
}

/// List-specific measurements attached to a resolved list, item, or their
/// descendants.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListStyle {
    pub kind: ListKind,
    pub depth: u8,
    /// This level's own `\leftmargin` (the resolved `left_margin` accumulates
    /// all levels).
    pub leftmargin: Pt,
    pub labelwidth: Pt,
    pub labelsep: Pt,
    pub topsep: Skip,
    /// Added to `topsep` when the list starts a new paragraph.
    pub partopsep: Skip,
    pub parsep: Skip,
    pub itemsep: Skip,
}

/// The fully inherited style of one block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedStyle {
    pub font_size: Pt,
    pub baselineskip: Pt,
    pub bold: bool,
    pub italic: bool,
    /// `\parindent` in force for paragraphs in this block.
    pub parindent: Pt,
    /// Whether this block's first line is indented by `parindent`.
    pub first_line_indent: bool,
    /// Vertical glue before the block. Not inherited.
    pub space_before: Skip,
    /// Vertical glue after the block. Not inherited.
    pub space_after: Skip,
    pub alignment: Alignment,
    /// Left indent of the block relative to the text area.
    pub left_margin: Pt,
    /// Right indent of the block relative to the text area.
    pub right_margin: Pt,
    /// For run-in headings (`\paragraph`, `\subparagraph`): the horizontal
    /// space between the heading and the following text.
    pub run_in_after: Option<Pt>,
    pub list: Option<ListStyle>,
}

/// One override rule of a [`StyleDelta`]: applies to blocks whose innermost
/// path node equals `block`, or to every block when `block` is `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DeltaRule {
    pub block: Option<Block>,
    pub font_size: Option<Pt>,
    pub baselineskip: Option<Pt>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub parindent: Option<Pt>,
    pub first_line_indent: Option<bool>,
    pub space_before: Option<Skip>,
    pub space_after: Option<Skip>,
    pub alignment: Option<Alignment>,
}

/// User settings layered over the class defaults. Rules apply in order after
/// the class resolution; later rules win. Rules on inherited properties
/// (size, leading, indent, alignment, weight, shape) propagate to descendants
/// because the overlay is applied at every ancestor during resolution.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleDelta {
    pub rules: Vec<DeltaRule>,
    /// Replace `\parskip` for every paragraph boundary.
    pub parskip: Option<Skip>,
}

impl StyleDelta {
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty() && self.parskip.is_none()
    }

    fn apply(&self, block: Block, style: &mut ResolvedStyle) {
        for r in &self.rules {
            if r.block.is_some() && r.block != Some(block) {
                continue;
            }
            if let Some(v) = r.font_size {
                style.font_size = v;
            }
            if let Some(v) = r.baselineskip {
                style.baselineskip = v;
            }
            if let Some(v) = r.bold {
                style.bold = v;
            }
            if let Some(v) = r.italic {
                style.italic = v;
            }
            if let Some(v) = r.parindent {
                style.parindent = v;
            }
            if let Some(v) = r.first_line_indent {
                style.first_line_indent = v;
            }
            if let Some(v) = r.space_before {
                style.space_before = v;
            }
            if let Some(v) = r.space_after {
                style.space_after = v;
            }
            if let Some(v) = r.alignment {
                style.alignment = v;
            }
        }
    }
}

/// A complete style model: class options, optional geometry, and a delta.
#[derive(Clone, Debug, PartialEq)]
pub struct Stylesheet {
    pub options: ClassOptions,
    pub geometry: Option<Geometry>,
    pub delta: StyleDelta,
}

impl Stylesheet {
    /// `\documentclass[options]{article}` with no packages.
    pub fn article(options: ClassOptions) -> Stylesheet {
        Stylesheet {
            options,
            geometry: None,
            delta: StyleDelta::default(),
        }
    }

    /// `\usepackage[...]{geometry}`.
    pub fn with_geometry(mut self, geometry: Geometry) -> Stylesheet {
        self.geometry = Some(geometry);
        self
    }

    pub fn with_delta(mut self, delta: StyleDelta) -> Stylesheet {
        self.delta = delta;
        self
    }

    pub fn base_size(&self) -> BaseSize {
        self.options.size
    }

    /// `ex`/`em` of the body font, used for section and list measurements.
    pub fn body_font(&self) -> FontParams {
        size_params(self.options.size).normal
    }

    pub fn parskip(&self) -> Skip {
        self.delta.parskip.unwrap_or(PARSKIP)
    }

    pub fn page_layout(&self) -> PageLayout {
        let base = article_page_params(self.options);
        let params = match &self.geometry {
            Some(g) => apply_geometry(base, g),
            None => base,
        };
        PageLayout::from_params(self.options.paper, params)
    }

    /// Style of the document root.
    fn root_style(&self) -> ResolvedStyle {
        let p = size_params(self.options.size);
        let normal = font_size(self.options.size, SizeName::NormalSize);
        ResolvedStyle {
            font_size: normal.size,
            baselineskip: normal.baselineskip,
            bold: false,
            italic: false,
            parindent: p.parindent,
            first_line_indent: false,
            space_before: Skip::ZERO,
            space_after: Skip::ZERO,
            alignment: Alignment::Justified,
            left_margin: Pt::ZERO,
            right_margin: Pt::ZERO,
            run_in_after: None,
            list: None,
        }
    }

    /// Resolve the style of the innermost block of `path`, inheriting from
    /// every ancestor. An empty path resolves the document root.
    ///
    /// Nesting deeper than [`MAX_LIST_NESTING_DEPTH`] list/alignment levels
    /// does not wrap or panic: like real LaTeX past its own list-nesting
    /// limit, resolution keeps using the deepest defined level (the reported
    /// [`ListStyle::depth`] saturates at the maximum and per-level parameters
    /// match [`list_level`] at that depth). Use [`Stylesheet::try_resolve`]
    /// to get an error instead of clamping.
    pub fn resolve(&self, path: &[Block]) -> ResolvedStyle {
        let mut style = self.root_style();
        self.delta.apply(Block::Document, &mut style);
        // `usize` so arbitrarily deep paths can never wrap back to a low
        // depth the way the old `u8` counter did past 255 levels.
        let mut list_depth: usize = 0;
        let mut inside_item = false;
        for (i, &block) in path.iter().enumerate() {
            // Spacing and per-block flags never inherit.
            style.space_before = Skip::ZERO;
            style.space_after = Skip::ZERO;
            style.first_line_indent = false;
            style.run_in_after = None;
            if block == Block::Document && i == 0 {
                self.delta.apply(block, &mut style);
                continue;
            }
            self.apply_block(block, &mut style, &mut list_depth, &mut inside_item);
            self.delta.apply(block, &mut style);
        }
        style
    }

    /// Like [`Stylesheet::resolve`], but returns [`ListNestingTooDeep`] when
    /// the path nests more than [`MAX_LIST_NESTING_DEPTH`] list/alignment
    /// environments instead of clamping to the deepest defined level.
    /// Depths at or under the maximum resolve exactly as `resolve` does.
    pub fn try_resolve(&self, path: &[Block]) -> Result<ResolvedStyle, ListNestingTooDeep> {
        // `apply_block` increments the depth exactly once per `List`/`Align`
        // node and never decrements it, so this pre-count is the tracked depth.
        let depth = path
            .iter()
            .filter(|b| matches!(b, Block::List(_) | Block::Align(_)))
            .count();
        if depth > MAX_LIST_NESTING_DEPTH {
            return Err(ListNestingTooDeep { depth });
        }
        Ok(self.resolve(path))
    }

    fn apply_block(
        &self,
        block: Block,
        style: &mut ResolvedStyle,
        list_depth: &mut usize,
        inside_item: &mut bool,
    ) {
        let base = self.options.size;
        let body = self.body_font();
        let parskip = self.parskip();
        // \list sets \parskip to the enclosing level's \parsep, so a nested
        // environment's \@topsep adds that instead of the document \parskip.
        let enclosing_parskip = if *inside_item {
            style.list.map(|l| l.parsep).unwrap_or(parskip)
        } else {
            parskip
        };
        match block {
            Block::Document | Block::Section(_) => {}
            Block::Heading(level) => {
                let spec = section_spec(level.clamp(1, 5)).expect("level clamped");
                let f = font_size(base, spec.font);
                style.font_size = f.size;
                style.baselineskip = f.baselineskip;
                style.bold = spec.bold;
                style.italic = false;
                // \@startsection: the before/after skips are evaluated in the
                // font current when \section is called, i.e. the body font.
                style.space_before = spec.before_ex.scale(body.x_height.0);
                match spec.after {
                    SectionAfter::VerticalEx(s) => {
                        style.space_after = s.scale(body.x_height.0);
                    }
                    SectionAfter::RunInEm(em) => {
                        style.run_in_after = Some(body.em(em));
                    }
                }
                if spec.indent_parindent {
                    style.left_margin += style.parindent;
                }
                style.first_line_indent = false;
            }
            Block::Paragraph => {
                style.space_before = if *inside_item {
                    style.list.map(|l| l.parsep).unwrap_or(parskip)
                } else {
                    parskip
                };
                style.first_line_indent = !*inside_item;
            }
            Block::ParagraphAfterHeading => {
                style.space_before = parskip;
                // \@afterheading after a negative before-skip (article's
                // \section..\subsubsection) sets \@afterindentfalse; use
                // Paragraph after run-in headings (see indent_after_heading).
                style.first_line_indent = false;
            }
            Block::List(kind) => {
                *list_depth += 1;
                // Clamp before narrowing to `u8`: past the deepest tabled
                // level keep using the last row instead of overflowing.
                let level = (*list_depth).min(MAX_LIST_NESTING_DEPTH) as u8;
                let lp = list_level(base, level);
                style.left_margin += lp.leftmargin;
                style.list = Some(ListStyle {
                    kind,
                    depth: level,
                    leftmargin: lp.leftmargin,
                    labelwidth: lp.labelwidth,
                    labelsep: lp.labelsep,
                    topsep: lp.topsep,
                    partopsep: lp.partopsep,
                    parsep: lp.parsep,
                    itemsep: lp.itemsep,
                });
                // \list: \topsep + \parskip before and after the environment
                // (\partopsep is added when the list starts a paragraph).
                style.space_before = lp.topsep.plus(enclosing_parskip);
                style.space_after = lp.topsep.plus(enclosing_parskip);
                // \listparindent is 0pt in article lists.
                style.parindent = Pt::ZERO;
                *inside_item = false;
            }
            Block::Item => {
                *inside_item = true;
                if let Some(l) = style.list {
                    // Between items: \itemsep + \parsep (none before the first).
                    style.space_before = l.itemsep.plus(l.parsep);
                }
            }
            Block::Align(a) => {
                // center/flushleft/flushright are \trivlist environments at the
                // next list depth: \topsep + \parskip around, no margins.
                *list_depth += 1;
                let level = (*list_depth).min(MAX_LIST_NESTING_DEPTH) as u8;
                let lp = list_level(base, level);
                style.alignment = a;
                style.space_before = lp.topsep.plus(enclosing_parskip);
                style.space_after = lp.topsep.plus(enclosing_parskip);
                style.parindent = Pt::ZERO;
            }
            Block::Inline(s) => match s {
                InlineStyle::Emph => style.italic = !style.italic,
                InlineStyle::Bold => style.bold = true,
                InlineStyle::Italic => style.italic = true,
                InlineStyle::Size(name) => {
                    let f = font_size(base, name);
                    style.font_size = f.size;
                    style.baselineskip = f.baselineskip;
                }
            },
        }
    }

    /// Baseline-to-baseline distance from the last body line before a heading
    /// to the heading's baseline: the heading paragraph is built with its own
    /// `\baselineskip` inside the font group, and `\addvspace` adds the
    /// before-skip.
    pub fn heading_gap_before(&self, level: u8) -> Skip {
        let h = self.resolve(&[Block::Document, Block::Heading(level)]);
        Skip::fixed(h.baselineskip.0).plus(h.space_before)
    }

    /// Baseline-to-baseline distance from the heading to the first body line:
    /// the after-skip plus the body `\baselineskip` (the font group has closed).
    pub fn heading_gap_after(&self, level: u8) -> Skip {
        let h = self.resolve(&[Block::Document, Block::Heading(level)]);
        let body = self.resolve(&[Block::Document, Block::Paragraph]);
        Skip::fixed(body.baselineskip.0).plus(h.space_after)
    }
}
