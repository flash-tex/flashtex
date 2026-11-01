//! Vertical layout: stacking paragraphs with TeX interline glue, paragraph and
//! heading skips, an optional baseline grid, widow/orphan control, and
//! deterministic page breaking with explicit overflow reporting.
//!
//! Model (raggedbottom, like LaTeX `article`): the vertical list is
//! `[space_before] [parskip] line… [space_after]` per block. Glue at the top
//! of a page is discarded (TeX rule), the first box on a page sits at
//! `topskip` below the text area top (or at its own height if taller), and
//! consecutive lines — across paragraphs too — are separated by TeX's
//! interline glue `baselineskip - prev_depth - height` (or `lineskip` when
//! that would be below `lineskiplimit`); intervening vertical skips add to it
//! without changing `prev_depth`, exactly as in TeX.
//!
//! Page coordinates: origin at the page's top-left corner, `x` rightward,
//! `y` downward, same unit as the input. `PositionedRun::x` = left margin +
//! run x; `baseline_y` absolute from the page top.

use crate::items::Glue;
use crate::linebreak::{Lines, PositionedRun};

/// One paragraph-shaped block of already broken lines plus its vertical context.
#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphBlock {
    pub lines: Lines,
    /// Glue before the block (`\section`: `3.5ex plus 1ex minus .2ex` of the
    /// body font). Natural width only; discarded at a page top.
    pub space_before: Glue,
    /// Glue after the block (`\section`: `2.3ex plus .2ex`). Natural width only.
    pub space_after: Glue,
    /// Do not end a page right after this block (headings). The following
    /// block's first `club_lines` lines must fit on the same page.
    pub keep_with_next: bool,
}

impl ParagraphBlock {
    pub fn body(lines: Lines) -> Self {
        ParagraphBlock {
            lines,
            space_before: Glue::fixed(0.0),
            space_after: Glue::fixed(0.0),
            keep_with_next: false,
        }
    }

    /// LaTeX 12pt article `\section` spacing evaluated in the 12pt Times body
    /// font (1ex = 5.4pt): before 3.5ex = 18.9pt, after 2.3ex = 12.42pt.
    pub fn section_heading_12pt(lines: Lines) -> Self {
        ParagraphBlock {
            lines,
            space_before: Glue::finite(18.9, 5.4, 1.08),
            space_after: Glue::finite(12.42, 1.08, 0.0),
            keep_with_next: true,
        }
    }
}

/// Page geometry and vertical parameters. Defaults: LaTeX 12pt article on US
/// Letter with `geometry{margin=1in}`.
#[derive(Debug, Clone, PartialEq)]
pub struct PageParams {
    pub page_width: f64,
    pub page_height: f64,
    pub margin_top: f64,
    pub margin_bottom: f64,
    pub margin_left: f64,
    pub margin_right: f64,
    /// `\topskip`: 12pt article 12pt (10pt article 10pt).
    pub topskip: f64,
    /// `\maxdepth`: LaTeX sets `.5\topskip` (6pt). A line's depth beyond this
    /// pushes its baseline up when checking the page bottom.
    pub max_depth: f64,
    /// `\parskip`: article `0pt plus 1pt`. Inserted before every paragraph and
    /// discarded at a page top.
    pub parskip: Glue,
    /// `\baselineskip` used between lines of consecutive blocks (14.5pt).
    pub baselineskip: f64,
    pub lineskip: f64,
    pub lineskiplimit: f64,
    /// When set, every baseline is snapped down to the next multiple of this
    /// pitch measured from the first baseline position (`topskip`) of the page.
    pub baseline_grid: Option<f64>,
    /// Minimum lines of a paragraph kept at the bottom of a page (`\clubpenalty`
    /// style). 2.
    pub club_lines: usize,
    /// Minimum lines of a paragraph carried to the top of the next page
    /// (`\widowpenalty` style). 2.
    pub widow_lines: usize,
}

impl PageParams {
    /// US Letter (612x792 bp = 614.295x794.97 TeX pt), margins 1in = 72.27pt,
    /// 12pt article vertical defaults. Units are TeX points.
    pub fn article_12pt_letter_1in_tex_pt() -> Self {
        PageParams {
            page_width: 614.295,
            page_height: 794.97,
            margin_top: 72.27,
            margin_bottom: 72.27,
            margin_left: 72.27,
            margin_right: 72.27,
            topskip: 12.0,
            max_depth: 6.0,
            parskip: Glue::finite(0.0, 1.0, 0.0),
            baselineskip: 14.5,
            lineskip: 1.0,
            lineskiplimit: 0.0,
            baseline_grid: None,
            club_lines: 2,
            widow_lines: 2,
        }
    }

    pub fn text_height(&self) -> f64 {
        self.page_height - self.margin_top - self.margin_bottom
    }

    pub fn text_width(&self) -> f64 {
        self.page_width - self.margin_left - self.margin_right
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlacedLine {
    pub paragraph: usize,
    pub line: usize,
    /// Absolute baseline from the page top.
    pub baseline_y: f64,
    pub height: f64,
    pub depth: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// 1-based.
    pub number: u32,
    pub width: f64,
    pub height: f64,
    pub lines: Vec<PlacedLine>,
    /// Every run on the page in absolute page coordinates.
    pub runs: Vec<PositionedRun>,
}

/// A line that had to be placed although it extends past the text area
/// bottom (it did not fit even at the top of an empty page, or a keep-together
/// group was taller than a page). Content is never dropped.
#[derive(Debug, Clone, PartialEq)]
pub struct PageOverflow {
    pub page: u32,
    pub paragraph: usize,
    pub line: usize,
    /// Bottom of the placed line (baseline + depth), absolute.
    pub bottom: f64,
    /// Text area bottom.
    pub limit: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pages {
    pub pages: Vec<Page>,
    pub overflow: Vec<PageOverflow>,
    pub text_height: f64,
}

// ---------------------------------------------------------------------------

struct Cursor<'a> {
    params: &'a PageParams,
    pages: Vec<Page>,
    overflow: Vec<PageOverflow>,
    /// Baseline of the last box on the current page, or None when the page is empty.
    last_baseline: Option<f64>,
    prev_depth: f64,
    /// Pending vertical skip (natural width) since the last box.
    pending_skip: f64,
    /// Lines placed on the current page (for widow/orphan bookkeeping).
    placed_on_page: usize,
}

impl<'a> Cursor<'a> {
    fn new(params: &'a PageParams) -> Self {
        Cursor {
            params,
            pages: vec![new_page(1, params)],
            overflow: Vec::new(),
            last_baseline: None,
            prev_depth: 0.0,
            pending_skip: 0.0,
            placed_on_page: 0,
        }
    }

    fn text_top(&self) -> f64 {
        self.params.margin_top
    }

    fn text_bottom(&self) -> f64 {
        self.params.page_height - self.params.margin_bottom
    }

    fn page_empty(&self) -> bool {
        self.last_baseline.is_none()
    }

    fn skip(&mut self, glue: &Glue) {
        // Glue at the top of a page is discarded.
        if !self.page_empty() {
            self.pending_skip += glue.width;
        }
    }

    /// Where a line of `height`/`depth` would sit if placed next.
    fn next_baseline(&self, height: f64) -> f64 {
        let p = self.params;
        let y = match self.last_baseline {
            None => self.text_top() + self.params.topskip.max(height),
            Some(prev) => {
                let mut g = p.baselineskip - self.prev_depth - height;
                if g < p.lineskiplimit {
                    g = p.lineskip;
                }
                prev + self.prev_depth + self.pending_skip + g + height
            }
        };
        match p.baseline_grid {
            Some(grid) if grid > 0.0 => {
                let origin = self.text_top() + p.topskip;
                let k = ((y - origin) / grid - 1e-9).ceil().max(0.0);
                origin + k * grid
            }
            _ => y,
        }
    }

    fn fits(&self, baseline: f64, depth: f64) -> bool {
        baseline + (depth - self.params.max_depth).max(0.0) <= self.text_bottom() + 1e-9
    }

    fn new_page(&mut self) {
        let n = self.pages.len() as u32 + 1;
        self.pages.push(new_page(n, self.params));
        self.last_baseline = None;
        self.prev_depth = 0.0;
        self.pending_skip = 0.0;
        self.placed_on_page = 0;
    }

    fn place(&mut self, para: usize, lines: &Lines, li: usize) {
        let line = &lines.lines[li];
        let y = self.next_baseline(line.height);
        if !self.fits(y, line.depth) {
            let page = self.pages.len() as u32;
            self.overflow.push(PageOverflow {
                page,
                paragraph: para,
                line: li,
                bottom: y + line.depth,
                limit: self.text_bottom(),
            });
        }
        let page = self.pages.last_mut().expect("at least one page");
        page.lines.push(PlacedLine {
            paragraph: para,
            line: li,
            baseline_y: y,
            height: line.height,
            depth: line.depth,
        });
        for r in &line.runs {
            let mut r = r.clone();
            r.x += self.params.margin_left;
            r.baseline_y = y;
            page.runs.push(r);
        }
        self.last_baseline = Some(y);
        self.prev_depth = line.depth;
        self.pending_skip = 0.0;
        self.placed_on_page += 1;
    }

    /// How many leading lines of `lines` (from `from`) fit on the current page
    /// without placing them.
    fn count_fitting(&self, lines: &Lines, from: usize) -> usize {
        let mut sim = SimCursor {
            last_baseline: self.last_baseline,
            prev_depth: self.prev_depth,
            pending_skip: self.pending_skip,
        };
        let mut n = 0;
        for li in from..lines.lines.len() {
            let line = &lines.lines[li];
            let y = self.sim_next_baseline(&sim, line.height);
            if !self.fits(y, line.depth) {
                break;
            }
            sim.last_baseline = Some(y);
            sim.prev_depth = line.depth;
            sim.pending_skip = 0.0;
            n += 1;
        }
        n
    }

    fn sim_next_baseline(&self, sim: &SimCursor, height: f64) -> f64 {
        let saved = Cursor {
            params: self.params,
            pages: Vec::new(),
            overflow: Vec::new(),
            last_baseline: sim.last_baseline,
            prev_depth: sim.prev_depth,
            pending_skip: sim.pending_skip,
            placed_on_page: 0,
        };
        saved.next_baseline(height)
    }
}

struct SimCursor {
    last_baseline: Option<f64>,
    prev_depth: f64,
    pending_skip: f64,
}

fn new_page(number: u32, params: &PageParams) -> Page {
    Page {
        number,
        width: params.page_width,
        height: params.page_height,
        lines: Vec::new(),
        runs: Vec::new(),
    }
}

/// Stacks `blocks` onto pages.
pub fn layout_pages(blocks: &[ParagraphBlock], params: &PageParams) -> Pages {
    let mut c = Cursor::new(params);
    let mut i = 0;
    while i < blocks.len() {
        let block = &blocks[i];
        let n = block.lines.lines.len();
        if n == 0 {
            i += 1;
            continue;
        }
        c.skip(&block.space_before);
        c.skip(&params.parskip);

        // keep_with_next: the whole block plus the next block's first
        // club_lines must fit here, else start a new page (if it helps).
        if block.keep_with_next && !c.page_empty() {
            let mut needed = c.count_fitting(&block.lines, 0);
            let mut ok = needed == n;
            if ok && let Some(next) = blocks.get(i + 1) {
                // Simulate placing this block, its skips, and the next block.
                let mut sim = Cursor {
                    params,
                    pages: vec![new_page(0, params)],
                    overflow: Vec::new(),
                    last_baseline: c.last_baseline,
                    prev_depth: c.prev_depth,
                    pending_skip: c.pending_skip,
                    placed_on_page: 0,
                };
                for li in 0..n {
                    sim.place(i, &block.lines, li);
                }
                sim.skip(&block.space_after);
                sim.skip(&next.space_before);
                sim.skip(&params.parskip);
                let want = params.club_lines.min(next.lines.lines.len());
                needed = sim.count_fitting(&next.lines, 0);
                ok = needed >= want;
            }
            if !ok {
                c.new_page();
                c.skip(&block.space_before);
                c.skip(&params.parskip);
            }
        }

        let mut li = 0;
        while li < n {
            let fitting = c.count_fitting(&block.lines, li);
            if fitting >= n - li {
                for l in li..n {
                    c.place(i, &block.lines, l);
                }
                break;
            }
            // Not everything fits: decide how many to keep here.
            let mut keep = fitting;
            let remaining_after = n - li - keep;
            if keep > 0 && remaining_after > 0 && remaining_after < params.widow_lines {
                // Avoid a widow: push extra lines to the next page.
                let pull = params.widow_lines - remaining_after;
                keep = keep.saturating_sub(pull);
            }
            let on_page_total = c.placed_on_page_for(i, li) + keep;
            if keep > 0 && on_page_total < params.club_lines && !c.page_empty() {
                keep = 0; // avoid an orphan: move the whole start to the next page
            }
            if keep == 0 && c.page_empty() {
                // Nothing fits on an empty page: place one line anyway (overflow
                // is reported by `place`), so progress is guaranteed.
                keep = 1;
            }
            for l in li..li + keep {
                c.place(i, &block.lines, l);
            }
            li += keep;
            if li < n {
                c.new_page();
            }
        }
        c.skip(&block.space_after);
        i += 1;
    }
    Pages {
        pages: c.pages,
        overflow: c.overflow,
        text_height: params.text_height(),
    }
}

impl Cursor<'_> {
    /// Lines of paragraph `para` already placed on the current page before
    /// index `li`.
    fn placed_on_page_for(&self, para: usize, li: usize) -> usize {
        self.pages
            .last()
            .map(|p| {
                p.lines
                    .iter()
                    .filter(|l| l.paragraph == para && l.line < li)
                    .count()
            })
            .unwrap_or(0)
    }
}
