//! TeX's page builder (TeX: The Program §§980–1028) over a vertical list of
//! line boxes, glue and penalties.
//!
//! Why not `paragraph-layout::layout_pages`: it enforces club/widow lines as
//! hard minimums, while LaTeX's `\clubpenalty`/`\widowpenalty` (150) are
//! costs weighed against page badness. pdflatex ends a page with the first
//! line of a paragraph when the alternative page is a whole line short (cost
//! 150 versus badness 417 on the 08-two-page fixture); this builder makes
//! the same choice. The interline-glue rule (`\baselineskip`,
//! `\lineskip`, `\lineskiplimit`), `\topskip`, `\maxdepth`, discarding of
//! glue and penalties at a page top and `\raggedbottom` (natural glue) are
//! implemented as in TeX; `\vsize` is the text height. Requested sibling
//! API: a penalty-based page cost model in paragraph-layout, after which
//! this file can go.

/// One node of the vertical list.
#[derive(Debug, Clone, PartialEq)]
pub enum VItem {
    /// A line box; `payload` identifies it to the caller.
    Box { height: f64, depth: f64, payload: (usize, usize) },
    Glue { width: f64, stretch: f64, shrink: f64, fil: bool },
    Penalty(i32),
}

pub const INF_PENALTY: i32 = 10_000;
pub const EJECT_PENALTY: i32 = -10_000;
pub const INF_BAD: i64 = 10_000;
pub const AWFUL_BAD: i64 = 0x3FFF_FFFF;
pub const DEPLORABLE: i64 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageParams {
    pub vsize: f64,
    pub topskip: f64,
    pub maxdepth: f64,
    pub baselineskip: f64,
    pub lineskip: f64,
    pub lineskiplimit: f64,
    /// `\flushbottom` (the LaTeX kernel default; standard classes keep it
    /// for two-sided and two-column documents): a page that ends at an
    /// ordinary break is `\vbox to\vsize`, its glue stretched or shrunk
    /// (TeX §676). Pages ended by `\newpage`/`\clearpage` (whose `\vfil`
    /// absorbs the difference) and the last page keep natural glue.
    pub flushbottom: bool,
}

/// A line placed on a page: baseline measured downward from the text
/// area's top.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    pub payload: (usize, usize),
    pub baseline: f64,
    pub height: f64,
    pub depth: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BuiltPage {
    pub lines: Vec<Placed>,
    /// Lines whose bottom (baseline + depth beyond `\maxdepth`) passes
    /// `\vsize`: TeX places them anyway (an overfull page) rather than
    /// losing content.
    pub overfull_by: f64,
}

fn sp(pt: f64) -> i64 {
    (pt * 65536.0).round() as i64
}

/// TeX's `badness(t, s)` (§108), on scaled points.
pub fn badness(t_pt: f64, s_pt: f64) -> i64 {
    let t = sp(t_pt);
    let s = sp(s_pt);
    if t == 0 {
        return 0;
    }
    if s <= 0 {
        return INF_BAD;
    }
    let r = if t <= 7_230_584 {
        (t * 297) / s
    } else if s >= 1_663_497 {
        t / (s / 297)
    } else {
        t
    };
    if r > 1290 {
        INF_BAD
    } else {
        (r * r * r + 0x20000) / 0x40000
    }
}

/// Appends interline glue for a box of `height` after a box of
/// `prev_depth` (§679 `append_to_vlist`). `None` for the first box of the
/// list (TeX's `ignore_depth`).
fn interline_glue(p: &PageParams, baselineskip: f64, prev_depth: Option<f64>, height: f64) -> Option<VItem> {
    let prev = prev_depth?;
    let mut g = baselineskip - prev - height;
    if g < p.lineskiplimit {
        g = p.lineskip;
    }
    Some(VItem::Glue {
        width: g,
        stretch: 0.0,
        shrink: 0.0,
        fil: false,
    })
}

/// A block of the vertical list as the caller describes it.
#[derive(Debug, Clone, PartialEq)]
pub struct VBlock {
    /// (height, depth) of each line, in order.
    pub lines: Vec<(f64, f64)>,
    /// Penalty appended before the block's skips (`\addpenalty`; LaTeX's
    /// `\@secpenalty` for sections). `None` for no penalty node.
    pub penalty_before: Option<i32>,
    /// `\addvspace` before the block (a section's before-skip, a display's
    /// `\abovedisplayskip`).
    pub space_before: Option<(f64, f64, f64)>,
    /// `\parskip` glue is appended before the first line when set.
    pub parskip: Option<(f64, f64, f64)>,
    /// Penalty between line i and i+1 (§890): `\interlinepenalty` plus
    /// `\clubpenalty` after the first line, plus `\widowpenalty` before the
    /// last line.
    pub interline_penalty: i32,
    pub club_penalty: i32,
    pub widow_penalty: i32,
    /// Penalty after the last line (`\nobreak` after a heading:
    /// `INF_PENALTY`; `\predisplaypenalty` handled by the display's own
    /// `penalty_before`).
    pub penalty_after: Option<i32>,
    pub space_after: Option<(f64, f64, f64)>,
    /// `\nointerlineskip` before the first line (TeX's `ignore_depth`).
    pub no_interline_first: bool,
    /// The block leaves `prev_depth` at `ignore_depth` (an `\hrule`, §1056):
    /// no interline glue before whatever follows.
    pub no_interline_after: bool,
    /// `\baselineskip` in force while this block's lines are appended (a
    /// heading's `\Large` value); `None` uses the page's.
    pub baselineskip: Option<f64>,
    /// `\vskip` glue appended right after line `i` (`\\[<dimen>]`:
    /// LaTeX's `\vadjust{\vskip <dimen>}`, §888 adjust material, before
    /// the interline penalty); entries past the end are 0.
    pub vskip_after: Vec<f64>,
    /// Glue appended after `penalty_after` and before `space_after`, so
    /// `space_after` stays the list's `\lastskip` (`\@maketitle`'s
    /// `\@endparenv` `\topsep` before its closing `\vskip 1.5em`).
    pub pre_space_after: Option<(f64, f64, f64)>,
}

/// Builds the vertical list with interline glue and penalties.
pub fn vlist(p: &PageParams, blocks: &[VBlock]) -> Vec<VItem> {
    let mut out: Vec<VItem> = Vec::new();
    let mut prev_depth: Option<f64> = None;
    let glue = |(w, st, sh): (f64, f64, f64)| VItem::Glue {
        width: w,
        stretch: st,
        shrink: sh,
        fil: false,
    };
    for (bi, b) in blocks.iter().enumerate() {
        if b.lines.is_empty() {
            continue;
        }
        if let Some(pen) = b.penalty_before {
            // \addpenalty: skipped at the very top of the list (\if@nobreak).
            if !out.is_empty() {
                out.push(VItem::Penalty(pen));
            }
        }
        if let Some(s) = b.space_before {
            out.push(glue(s));
        }
        if let Some(s) = b.parskip {
            out.push(glue(s));
        }
        let n = b.lines.len();
        for (li, (h, d)) in b.lines.iter().enumerate() {
            let prev = if li == 0 && b.no_interline_first { None } else { prev_depth };
            if let Some(g) = interline_glue(p, b.baselineskip.unwrap_or(p.baselineskip), prev, *h) {
                out.push(g);
            }
            out.push(VItem::Box {
                height: *h,
                depth: *d,
                payload: (bi, li),
            });
            prev_depth = if li + 1 == n && b.no_interline_after { None } else { Some(*d) };
            if let Some(&v) = b.vskip_after.get(li) {
                if v != 0.0 {
                    out.push(glue((v, 0.0, 0.0)));
                }
            }
            if li + 1 < n {
                let mut pen = b.interline_penalty;
                if li == 0 {
                    pen += b.club_penalty;
                }
                if li + 2 == n {
                    pen += b.widow_penalty;
                }
                if pen != 0 {
                    out.push(VItem::Penalty(pen.min(INF_PENALTY)));
                }
            }
        }
        if let Some(pen) = b.penalty_after {
            out.push(VItem::Penalty(pen));
        }
        if let Some(s) = b.pre_space_after {
            out.push(glue(s));
        }
        if let Some(s) = b.space_after {
            out.push(glue(s));
        }
    }
    out
}

struct PageState {
    /// `page_total`: heights and depths so far, excluding the last depth.
    total: f64,
    stretch: f64,
    fil: bool,
    shrink: f64,
    /// `page_depth`: depth of the last box (bounded by `\maxdepth`).
    depth: f64,
    /// Whether a box has been placed (`page_contents = box_there`).
    has_box: bool,
    /// y of the current bottom edge below the last box's baseline.
    lines: Vec<Placed>,
}

impl PageState {
    fn new() -> PageState {
        PageState {
            total: 0.0,
            stretch: 0.0,
            fil: false,
            shrink: 0.0,
            depth: 0.0,
            has_box: false,
            lines: Vec::new(),
        }
    }
}

/// Positions of the boxes of `list` at natural size, measured down from the
/// top of the box they are set in: with `topskip` the first box sits below
/// `\topskip` glue (a page), otherwise at its own height (an internal
/// `\vbox`, whose leading glue also counts). Returns the placed boxes and
/// the natural height through the last item (glue after the last box
/// included, the last box's depth not).
pub fn natural_layout(p: &PageParams, list: &[VItem], topskip: bool) -> (Vec<Placed>, f64) {
    let mut placed = Vec::new();
    let (mut total, mut depth, mut has_box) = (0.0, 0.0, false);
    for item in list {
        match item {
            VItem::Box { height, depth: d, payload } => {
                let baseline = match (has_box, topskip) {
                    (true, _) => total + depth + height,
                    (false, true) => (p.topskip - height).max(0.0) + height,
                    (false, false) => total + height,
                };
                total = baseline;
                depth = *d;
                has_box = true;
                placed.push(Placed {
                    payload: *payload,
                    baseline,
                    height: *height,
                    depth: *d,
                });
            }
            VItem::Glue { width, .. } => {
                if has_box || !topskip {
                    total += depth + width;
                    depth = 0.0;
                }
            }
            VItem::Penalty(_) => {}
        }
    }
    (placed, total)
}

/// Breaks `list` into pages. Returns the pages with lines at their natural
/// (`\raggedbottom`) positions.
pub fn break_pages(p: &PageParams, list: &[VItem]) -> Vec<BuiltPage> {
    break_pages_shortened(p, list, 0, 0.0)
}

/// [`break_pages`] with the first `short_pages` pages (page-builder
/// columns) `short` points shorter: `\@topnewpage` (latex.ltx) lowers
/// `\@colht` by the height of `\twocolumn[<material>]`'s box plus
/// `\dbltextfloatsep` for both columns of that page.
pub fn break_pages_shortened(base: &PageParams, list: &[VItem], short_pages: usize, short: f64) -> Vec<BuiltPage> {
    break_pages_tops(base, list, short_pages, short, &[], 1)
}

/// [`break_pages_shortened`] with `\@topnewpage` boxes on later pages too
/// (report/book `\chapter` in two-column mode: `\@topnewpage[\@makechapterhead]`):
/// `tops` holds, in order, the first block after each box and the height
/// the box takes from the columns; the page-builder column whose first line
/// belongs to that block or a later one and the `columns - 1` after it are
/// that much shorter (column alignment later puts the first on a page's
/// first column).
pub fn break_pages_tops(base: &PageParams, list: &[VItem], short_pages: usize, short: f64, tops: &[(usize, f64)], columns: usize) -> Vec<BuiltPage> {
    let mut pages: Vec<BuiltPage> = Vec::new();
    let mut start = 0usize;
    let (mut next_top, mut top_left, mut top_short) = (0usize, 0usize, 0.0);
    while start < list.len() {
        // Discard glue/penalties at the top of the page.
        while start < list.len() && !matches!(list[start], VItem::Box { .. }) {
            start += 1;
        }
        if start >= list.len() {
            break;
        }
        if let (Some(&(block, height)), VItem::Box { payload, .. }) = (tops.get(next_top), &list[start]) {
            if payload.0 >= block {
                next_top += 1;
                top_left = columns.max(1);
                top_short = height;
            }
        }
        let page_params = PageParams {
            vsize: base.vsize - if pages.len() < short_pages { short } else { 0.0 } - if top_left > 0 { top_short } else { 0.0 },
            ..*base
        };
        top_left = top_left.saturating_sub(1);
        let p = &page_params;
        let mut st = PageState::new();
        let mut best: Option<(usize, i64)> = None; // (break index, cost)
        let mut fired: Option<usize> = None;
        let mut i = start;
        while i < list.len() {
            let legal = match &list[i] {
                VItem::Penalty(pen) => *pen < INF_PENALTY,
                VItem::Glue { .. } => i > 0 && matches!(list[i - 1], VItem::Box { .. }),
                VItem::Box { .. } => false,
            };
            let pi = match &list[i] {
                VItem::Penalty(pen) => *pen,
                _ => 0,
            };
            if legal && st.has_box {
                // §1005: page badness and cost at this breakpoint.
                let b = if st.total < p.vsize {
                    if st.fil {
                        0
                    } else {
                        badness(p.vsize - st.total, st.stretch)
                    }
                } else if st.total - p.vsize > st.shrink {
                    AWFUL_BAD
                } else {
                    badness(st.total - p.vsize, st.shrink)
                };
                let c = if b < AWFUL_BAD {
                    if pi <= EJECT_PENALTY {
                        i64::from(pi)
                    } else if b < INF_BAD {
                        b + i64::from(pi)
                    } else {
                        DEPLORABLE
                    }
                } else {
                    b
                };
                if best.map_or(true, |(_, lc)| c <= lc) {
                    best = Some((i, c));
                }
                if c == AWFUL_BAD || pi <= EJECT_PENALTY {
                    fired = Some(best.map_or(i, |(bi, _)| bi));
                    break;
                }
            }
            match &list[i] {
                VItem::Box { height, depth, payload } => {
                    // §1002: contribute the box.
                    let (baseline, top_glue) = if !st.has_box {
                        let g = (p.topskip - height).max(0.0);
                        (g + height, g)
                    } else {
                        (st.total + st.depth + height, 0.0)
                    };
                    let _ = top_glue;
                    st.total += if st.has_box { st.depth + height } else { baseline };
                    st.has_box = true;
                    st.depth = *depth;
                    if st.depth > p.maxdepth {
                        st.total += st.depth - p.maxdepth;
                        st.depth = p.maxdepth;
                    }
                    st.lines.push(Placed {
                        payload: *payload,
                        baseline,
                        height: *height,
                        depth: *depth,
                    });
                    // A box that overfills the page beyond what its glue
                    // can shrink (§1005: `page_total - page_goal >
                    // page_shrink` is `awful_bad` at the next breakpoint):
                    // TeX fires at the best break so far. Material that the
                    // shrink absorbs stays a candidate.
                    if st.total > p.vsize + st.shrink + 1e-9 && st.lines.len() > 1 {
                        if let Some((bi, _)) = best {
                            fired = Some(bi);
                            break;
                        }
                    }
                }
                VItem::Glue { width, stretch, shrink, fil } => {
                    if st.has_box {
                        st.total += st.depth + width;
                        st.depth = 0.0;
                        st.stretch += stretch;
                        st.fil |= *fil;
                        st.shrink += shrink;
                    }
                }
                VItem::Penalty(_) => {}
            }
            i += 1;
        }
        let end = match fired {
            Some(bi) => bi,
            None => list.len(),
        };
        let ejected = matches!(list.get(end), Some(VItem::Penalty(pen)) if *pen <= EJECT_PENALTY);
        // `\@makecol` sets every column `\vbox to\@colht`: material taller
        // than the column shrinks whatever the page style (`\raggedbottom`'s
        // `\@textbottom` and `\newpage`'s `\vfil` only stretch); a short
        // page stretches only under `\flushbottom` at an ordinary break.
        let set = match glue_set(p, &list[start..end]) {
            g if g < 0.0 => g,
            g if p.flushbottom && fired.is_some() && !ejected => g,
            _ => 0.0,
        };
        // Materialise the page: lines whose box index is before `end`.
        let mut page = BuiltPage::default();
        let mut cursor = start;
        let mut total = 0.0;
        let mut depth = 0.0;
        let mut has_box = false;
        while cursor < end {
            match &list[cursor] {
                VItem::Box { height, depth: d, payload } => {
                    let baseline = if !has_box {
                        (p.topskip - height).max(0.0) + height
                    } else {
                        total + depth + height
                    };
                    total = baseline;
                    depth = *d;
                    has_box = true;
                    page.lines.push(Placed {
                        payload: *payload,
                        baseline,
                        height: *height,
                        depth: *d,
                    });
                }
                VItem::Glue { width, stretch, shrink, .. } => {
                    if has_box {
                        total += depth + width + if set > 0.0 { set * stretch } else { set * shrink };
                        depth = 0.0;
                    }
                }
                VItem::Penalty(_) => {}
            }
            cursor += 1;
        }
        if let Some(last) = page.lines.last() {
            let bottom = last.baseline + (last.depth - p.maxdepth).max(0.0);
            if bottom > p.vsize + 1e-6 {
                page.overfull_by = bottom - p.vsize;
            }
        }
        if page.lines.is_empty() {
            // No progress possible (should not happen: a box always lands).
            break;
        }
        pages.push(page);
        start = end;
        if fired.is_none() {
            break;
        }
    }
    pages
}

/// Glue set ratio of a page box `\vbox to\vsize` holding `items` (from the
/// first box to the break): positive stretches by `ratio * stretch`,
/// negative shrinks by `-ratio * shrink` (capped at the available shrink,
/// TeX §676/§677). 0 when fil glue absorbs the excess or nothing stretches.
fn glue_set(p: &PageParams, items: &[VItem]) -> f64 {
    let (mut total, mut depth, mut has_box, mut last_box) = (0.0, 0.0, false, false);
    let (mut stretch, mut shrink, mut fil) = (0.0, 0.0, false);
    for item in items {
        match item {
            VItem::Box { height, depth: d, .. } => {
                total = if has_box { total + depth + height } else { (p.topskip - height).max(0.0) + height };
                depth = *d;
                has_box = true;
                last_box = true;
            }
            VItem::Glue { width, stretch: st, shrink: sh, fil: f } => {
                if has_box {
                    total += depth + width;
                    depth = 0.0;
                    stretch += st;
                    shrink += sh;
                    fil |= *f;
                    last_box = false;
                }
            }
            VItem::Penalty(_) => {}
        }
    }
    // `\boxmaxdepth`: depth beyond `\maxdepth` counts as height.
    let natural = total + if last_box { (depth - p.maxdepth).max(0.0) } else { 0.0 };
    let excess = p.vsize - natural;
    if excess > 0.0 {
        if fil || stretch <= 0.0 {
            0.0
        } else {
            excess / stretch
        }
    } else if excess < 0.0 && shrink > 0.0 {
        -(-excess / shrink).min(1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> PageParams {
        PageParams {
            vsize: 650.43,
            topskip: 12.0,
            maxdepth: 6.0,
            baselineskip: 14.5,
            lineskip: 1.0,
            lineskiplimit: 0.0,
            flushbottom: false,
        }
    }

    fn para(n: usize) -> VBlock {
        VBlock {
            lines: vec![(8.4, 2.3); n],
            penalty_before: None,
            space_before: None,
            parskip: Some((0.0, 1.0, 0.0)),
            interline_penalty: 0,
            club_penalty: 150,
            widow_penalty: 150,
            penalty_after: None,
            space_after: None,
            no_interline_first: false,
            no_interline_after: false,
            baselineskip: None,
            vskip_after: Vec::new(),
            pre_space_after: None,
        }
    }

    #[test]
    fn badness_matches_tex() {
        assert_eq!(badness(0.0, 1.0), 0);
        assert_eq!(badness(1.0, 0.0), INF_BAD);
        assert_eq!(badness(1.0, 1.0), 100);
        assert_eq!(badness(0.5, 1.0), 12);
        assert_eq!(badness(14.5, 9.0), 417); // 100(14.5/9)^3 = 418.1; TeX's integer arithmetic gives 417
    }

    #[test]
    fn club_line_is_kept_when_the_alternative_page_is_worse() {
        // 45 lines fit (baseline 45 = 12 + 44*14.5 = 650 <= 650.43); the
        // 45th line starts a paragraph: cost 150 beats badness 418.
        let blocks: Vec<VBlock> = vec![para(44), para(6)];
        let list = vlist(&params(), &blocks);
        let pages = break_pages(&params(), &list);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].lines.len(), 45);
        assert_eq!(pages[0].lines[44].payload, (1, 0));
        assert!((pages[0].lines[44].baseline - 650.0).abs() < 1e-9);
        assert_eq!(pages[1].lines.len(), 5);
        assert!((pages[1].lines[0].baseline - 12.0).abs() < 1e-9);
    }

    #[test]
    fn a_widow_is_avoided_when_it_costs_less_than_the_badness() {
        // Stretch is large (many parskips): breaking one line early is
        // cheap, so the widow penalty decides.
        let mut blocks: Vec<VBlock> = (0..21).map(|_| para(2)).collect();
        blocks.push(para(4));
        // 21*2 = 42 lines; lines 43..46 are the 4-line paragraph; 45 fit,
        // so the break after line 45 would leave a widow (cost 150 + b).
        let list = vlist(&params(), &blocks);
        let pages = break_pages(&params(), &list);
        assert_eq!(pages.len(), 2);
        // Either 44 (widow avoided) or 45 lines: with 22 parskips (22pt
        // stretch) the 14.5pt shortfall has badness 28 < 150, so 44.
        assert_eq!(pages[0].lines.len(), 44);
    }

    #[test]
    fn nobreak_after_a_heading_moves_it_to_the_next_page() {
        let heading = VBlock {
            lines: vec![(12.0, 3.0)],
            penalty_before: Some(-300),
            space_before: Some((18.0, 5.0, 1.0)),
            parskip: Some((0.0, 1.0, 0.0)),
            interline_penalty: INF_PENALTY,
            club_penalty: 0,
            widow_penalty: 0,
            penalty_after: Some(INF_PENALTY),
            space_after: Some((12.4, 1.0, 0.0)),
            no_interline_first: false,
            no_interline_after: false,
            vskip_after: Vec::new(),
            pre_space_after: None,
            baselineskip: Some(22.0),
        };
        let mut after = para(3);
        after.club_penalty = INF_PENALTY;
        let blocks = vec![para(44), heading, after];
        let list = vlist(&params(), &blocks);
        let pages = break_pages(&params(), &list);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].lines.len(), 44);
        assert_eq!(pages[1].lines[0].payload, (1, 0));
        assert_eq!(pages[1].lines.len(), 4);
    }
}
