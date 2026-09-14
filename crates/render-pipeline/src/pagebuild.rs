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
fn interline_glue(p: &PageParams, baselineskip: f64, lineskip: f64, prev_depth: Option<f64>, height: f64) -> Option<VItem> {
    let prev = prev_depth?;
    let mut g = baselineskip - prev - height;
    if g < p.lineskiplimit {
        g = lineskip;
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
    /// last line, plus `\brokenpenalty` (see `broken_penalty`).
    pub interline_penalty: i32,
    pub club_penalty: i32,
    pub widow_penalty: i32,
    /// `\brokenpenalty` (LaTeX: 100) for line `i`, charged when that line's
    /// own break was a discretionary (§890's `disc_break`). It goes into the
    /// same penalty node as the three above, so a hyphenated line is a
    /// *worse* place to end a page than the line after it — which is how
    /// TeX fits one more line onto a page by shrinking its glue instead of
    /// breaking at the hyphen. Entries past the end are 0.
    pub broken_penalty: Vec<i32>,
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
    /// `\lineskip` in force while this block's lines are appended
    /// (longtable sets it to 0 so its rows abut); `None` uses the page's.
    pub lineskip: Option<f64>,
    /// How many of `lines` go into the vertical list. The rest are built
    /// but held back for a [`Region`] to insert (longtable's `\LT@head`
    /// and `\LT@foot`); `None` contributes all of them.
    pub contributed: Option<usize>,
    /// `(line index, penalty)`: a `\penalty` node right before that line,
    /// instead of the club/widow/interline penalties. Sorted by index; an
    /// index equal to the line count puts the penalty after the last line.
    pub line_penalty: Vec<(usize, i32)>,
    /// What the block leaves in `\prevdepth`.
    pub depth_after: DepthAfter,
}

/// `\prevdepth` after a block, for the interline glue of whatever follows.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DepthAfter {
    /// The last line's depth: an ordinary `\box` appended in vertical mode
    /// (TeX §679 `append_to_vlist`).
    #[default]
    LastLine,
    /// Unchanged. longtable `\unvbox`es each chunk into the page's vertical
    /// list (longtable.sty 252, 322, 334), and `\unvbox` sets no
    /// `\prevdepth`, so a table with no head or foot box leaves the value
    /// the material before it had.
    Unchanged,
    /// The depth of a box the package appended itself: longtable's
    /// `\box\LT@firsthead` (239) or `\box\LT@lastfoot` (506).
    Fixed(f64),
}

/// A stretch of the vertical list that carries its own page-breaking rules:
/// longtable's region between `\LT@start` and `\endlongtable`
/// (longtable.sty 196-241, 487-517). Inside it `\pagegoal` is reduced by
/// `\ht\LT@foot`; a break appends `\LT@foot` to the page that ends and
/// starts the next one with `\LT@head`.
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    /// The block's contributed line indices the region covers.
    pub lines: std::ops::Range<usize>,
    /// `\LT@head`: `(line index, height, depth)`, contributed at the top
    /// of every page after a break inside the region.
    pub head: Option<(usize, f64, f64)>,
    /// `\LT@foot`: appended to a page broken inside the region.
    pub foot: Option<(usize, f64, f64)>,
    /// The first line of the closing foot (`\LT@lastfoot`), from which
    /// `tail_foot_height` rather than the foot's height is reserved.
    pub tail_from: usize,
    pub tail_foot_height: f64,
}

/// A region resolved against vertical-list indices.
#[derive(Debug, Clone, PartialEq)]
struct VRegion {
    /// Vertical-list index range.
    range: std::ops::Range<usize>,
    /// Index from which `tail_foot` applies instead of `foot`.
    tail: usize,
    foot_height: f64,
    tail_foot_height: f64,
    head: Option<(f64, f64, (usize, usize))>,
    foot: Option<(f64, f64, (usize, usize))>,
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
        let depth_before = prev_depth;
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
        let n = b.contributed.unwrap_or(b.lines.len()).min(b.lines.len());
        for (li, (h, d)) in b.lines.iter().take(n).enumerate() {
            let prev = if li == 0 && b.no_interline_first { None } else { prev_depth };
            if let Some(g) = interline_glue(p, b.baselineskip.unwrap_or(p.baselineskip), b.lineskip.unwrap_or(p.lineskip), prev, *h) {
                out.push(g);
            }
            if let Some((_, pen)) = b.line_penalty.iter().find(|(i, _)| *i == li) {
                out.push(VItem::Penalty(*pen));
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
            if li + 1 < n && b.line_penalty.is_empty() {
                let mut pen = b.interline_penalty;
                if li == 0 {
                    pen += b.club_penalty;
                }
                if li + 2 == n {
                    pen += b.widow_penalty;
                }
                pen += b.broken_penalty.get(li).copied().unwrap_or(0);
                if pen != 0 {
                    out.push(VItem::Penalty(pen.min(INF_PENALTY)));
                }
            }
        }
        if let Some((_, pen)) = b.line_penalty.iter().find(|(i, _)| *i == n) {
            out.push(VItem::Penalty(*pen));
        }
        match b.depth_after {
            DepthAfter::LastLine => {}
            DepthAfter::Unchanged => prev_depth = depth_before,
            DepthAfter::Fixed(d) => prev_depth = Some(d),
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

impl Region {
    /// Where the region's lines ended up in the vertical list, and the
    /// head and foot boxes as `(height, depth, payload)`.
    fn resolve(&self, list: &[VItem], block: usize) -> Option<VRegion> {
        let at = |line: usize| {
            list.iter().position(|v| matches!(v, VItem::Box { payload, .. } if *payload == (block, line)))
        };
        let first = (self.lines.start..self.lines.end).find_map(at)?;
        let last = (self.lines.start..self.lines.end).rev().find_map(at)?;
        let tail = (self.tail_from..self.lines.end).find_map(at).unwrap_or(last + 1);
        Some(VRegion {
            range: first..last + 1,
            tail,
            foot_height: self.foot.map_or(0.0, |(_, h, _)| h),
            tail_foot_height: self.tail_foot_height,
            head: self.head.map(|(l, h, d)| (h, d, (block, l))),
            foot: self.foot.map(|(l, h, d)| (h, d, (block, l))),
        })
    }
}

/// Resolves each `(block index, region)` against the vertical list.
pub fn resolve_regions(list: &[VItem], regions: &[(usize, Region)]) -> Vec<ResolvedRegion> {
    regions.iter().filter_map(|(b, r)| r.resolve(list, *b).map(ResolvedRegion)).collect()
}

/// A [`Region`] located in the vertical list, ready for [`break_pages_regions`].
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedRegion(VRegion);

impl ResolvedRegion {
    /// The vertical-list indices the region covers.
    pub fn range(&self) -> std::ops::Range<usize> {
        self.0.range.clone()
    }
}

/// The regions' view of the vertical list: what `\pagegoal` loses at each
/// index (longtable.sty 229: `\LT@start` subtracts `\ht\LT@foot` from
/// `\pagegoal` itself, and 274-275 gives it back at `\endlongtable`), and
/// the `\LT@foot`/`\LT@head` boxes a break inside one needs.
///
/// Every page-building path shares it: [`break_pages_regions`],
/// [`break_pages_inserts_regions`] and `typeset::floatpage::paginate`.
#[derive(Clone, Copy)]
pub(crate) struct Regions<'a>(&'a [ResolvedRegion]);

impl<'a> Regions<'a> {
    pub(crate) fn new(regions: &'a [ResolvedRegion]) -> Regions<'a> {
        Regions(regions)
    }

    fn at(&self, i: usize) -> Option<&'a VRegion> {
        self.0.iter().map(|r| &r.0).find(|r| r.range.contains(&i))
    }

    /// `\ht\LT@foot` reserved from `\pagegoal` at vertical-list index `i`,
    /// or `\ht\LT@lastfoot` once the closing foot's rows are reached.
    pub(crate) fn reserved(&self, i: usize) -> f64 {
        self.at(i).map_or(0.0, |r| if i >= r.tail { r.tail_foot_height } else { r.foot_height })
    }

    /// Whether vertical-list index `i` is inside some region.
    pub(crate) fn contains(&self, i: usize) -> bool {
        self.at(i).is_some()
    }

    /// longtable.sty 223: unless the table cannot start on this page at all,
    /// `\LT@start` ends with `\ifdim\pageshrink>\z@\pageshrink\z@\fi` — the
    /// glue already on the page (the paragraph's `\parskip`, `\LTpre`'s own
    /// `minus 4pt`, `\skip\footins`) may no longer be shrunk to buy the
    /// table another row. Without it the builder squeezes one row too many
    /// onto the page the table starts on.
    pub(crate) fn starts_at(&self, i: usize) -> bool {
        self.0.iter().any(|r| r.0.range.start == i)
    }

    /// `\LT@output`'s ordinary branch (longtable.sty 513-516) for a page
    /// that ended at `end`: the foot to append to it and the head to open
    /// the next one with, when the break fell inside a region and the
    /// region continues past it.
    pub(crate) fn at_break(&self, end: usize) -> Option<(Option<Placed3>, Option<Placed3>)> {
        let r = self.at(end.saturating_sub(1)).filter(|r| end < r.range.end)?;
        Some((r.foot, r.head))
    }
}

/// `page_max_depth` for a page whose first box is `first` (§987
/// `freeze_page_specs`: `\maxdepth` is copied into `page_max_depth` when
/// the page's first box is contributed, and never re-read for that page).
///
/// `\LT@start` sets `\maxdepth\z@` (longtable.sty 230) at the outer
/// vertical list, after `\LT@echunk` has closed the first chunk's box but
/// before `\unvbox\z@` contributes any of its rows — so the zero reaches
/// the page exactly when the table's first row is also the page's first
/// box. A table starting under existing material finds `page_max_depth`
/// already frozen at `\@maxdepth`, and every later page of the table gets
/// `\@maxdepth` back from `\@makecol`'s `\global\maxdepth\@maxdepth`, so
/// only the page a table opens is affected. Measured against pdflatex:
/// 104-longtable-chunk-boundary, whose document begins with the table,
/// loses one row on its first page and none afterwards.
fn page_max_depth(base: &PageParams, regions: Regions, first: usize) -> f64 {
    if regions.starts_at(first) {
        0.0
    } else {
        base.maxdepth
    }
}

/// `(height, depth, payload)` of a head or foot box.
pub(crate) type Placed3 = (f64, f64, (usize, usize));

/// The head or foot box as a vertical-list item, so a page's material can
/// carry it through the same natural-size and glue-setting passes as the
/// rest (`\LT@output` puts both inside `\box\@cclv`, which `\@makecol`
/// then packs).
fn region_box((height, depth, payload): Placed3) -> VItem {
    VItem::Box { height, depth, payload }
}

/// [`break_pages`] with the first `short_pages` pages (page-builder
/// columns) `short` points shorter: `\@topnewpage` (latex.ltx) lowers
/// `\@colht` by the height of `\twocolumn[<material>]`'s box plus
/// `\dbltextfloatsep` for both columns of that page.
pub fn break_pages_shortened(base: &PageParams, list: &[VItem], short_pages: usize, short: f64) -> Vec<BuiltPage> {
    break_pages_regions(base, list, short_pages, short, &[])
}

/// [`break_pages_shortened`] with longtable regions: inside one, the page
/// goal is reduced by `\ht\LT@foot` (longtable.sty 226-229), a page broken
/// inside it ends with `\LT@foot` and the next begins with `\LT@head`
/// (`\LT@output`, 487-517).
pub fn break_pages_regions(base: &PageParams, list: &[VItem], short_pages: usize, short: f64, regions: &[ResolvedRegion]) -> Vec<BuiltPage> {
    let mut pages: Vec<BuiltPage> = Vec::new();
    let mut start = 0usize;
    // `\copy\LT@head\nobreak` at the top of a continuation page.
    let mut pending_head: Option<Placed3> = None;
    let regions = Regions(regions);
    while start < list.len() {
        // Discard glue/penalties at the top of the page.
        while start < list.len() && !matches!(list[start], VItem::Box { .. }) {
            start += 1;
        }
        if start >= list.len() {
            break;
        }
        let page_params = PageParams {
            vsize: if pages.len() < short_pages { base.vsize - short } else { base.vsize },
            maxdepth: page_max_depth(base, regions, start),
            ..*base
        };
        let p = &page_params;
        // `\pagegoal` loses `\ht\LT@foot` for as long as the page's
        // material is inside a longtable, and `\maxdepth` is zero there.
        let reserved = |i: usize| regions.reserved(i);
        let head = pending_head.take();
        let mut st = PageState::new();
        let mut best: Option<(usize, i64)> = None; // (break index, cost)
        let mut fired: Option<usize> = None;
        let mut i = start;
        if let Some((h, d, _)) = head {
            st.total = (p.topskip - h).max(0.0) + h;
            st.depth = d;
            st.has_box = true;
        }
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
                let goal = p.vsize - reserved(i);
                let b = if st.total < goal {
                    if st.fil {
                        0
                    } else {
                        badness(goal - st.total, st.stretch)
                    }
                } else if st.total - goal > st.shrink {
                    AWFUL_BAD
                } else {
                    badness(st.total - goal, st.shrink)
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
            if regions.starts_at(i) {
                st.shrink = 0.0;
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
                    // `\LT@start`'s `\maxdepth\z@` (longtable.sty 230) does
                    // not reach the page: TeX froze `page_max_depth` when
                    // the page's first box landed (§987), which for a table
                    // starting mid-page is before `\LT@start` runs, and
                    // `\@makecol` ends every page with
                    // `\global\maxdepth\@maxdepth`, so each continuation
                    // page freezes the class value again. Measured against
                    // pdflatex on a three-page longtable: a zeroed maxdepth
                    // loses one row per continuation page.
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
                    if st.total > p.vsize - reserved(i) + st.shrink + 1e-9 && st.lines.len() > 1 {
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
        // A page the longtable output routine ends carries `\vss` after
        // `\copy\LT@foot` (longtable.sty 513), which takes the column's
        // whole slack and leaves the finite glue at its natural size.
        let region_break = fired.is_some() && end < list.len() && regions.at_break(end).is_some();
        // `\@makecol` sets every column `\vbox to\@colht`: material taller
        // than the column shrinks whatever the page style (`\raggedbottom`'s
        // `\@textbottom` and `\newpage`'s `\vfil` only stretch); a short
        // page stretches only under `\flushbottom` at an ordinary break.
        let set = match glue_set(p, &list[start..end]) {
            _ if region_break => 0.0,
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
        if let Some((h, d, payload)) = head {
            let baseline = (p.topskip - h).max(0.0) + h;
            total = baseline;
            depth = d;
            has_box = true;
            page.lines.push(Placed { payload, baseline, height: h, depth: d });
        }
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
        // `\LT@output`: a page broken inside the table ends with
        // `\LT@foot` and the next one opens with `\LT@head`.
        if fired.is_some() && end < list.len() {
            if let Some((foot, head)) = regions.at_break(end) {
                if let Some((h, d, payload)) = foot {
                    let baseline = total + depth + h;
                    total = baseline;
                    depth = d;
                    page.lines.push(Placed { payload, baseline, height: h, depth: d });
                }
                pending_head = head;
            }
        }
        let _ = (total, depth);
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

/// One insertion class (LaTeX's `\footins`) for [`break_pages_inserts`]:
/// the notes' vertical lists and the lines their `\insert`s follow.
#[derive(Debug, Clone, Default)]
pub struct Insertions {
    /// For a line box payload, the notes (indices into `notes`) whose
    /// `\insert` migrated out of that line (TeX §655: after the line box,
    /// before the interline penalty and glue), in order.
    pub after: std::collections::BTreeMap<(usize, usize), Vec<usize>>,
    /// Each note's vertical list (`\vbox` contents of its `\insert`).
    pub notes: Vec<Vec<VItem>>,
    /// `\skip\footins` (natural, stretch, shrink).
    pub skip: (f64, f64, f64),
    /// `\dimen\footins`.
    pub max: f64,
    /// `\splittopskip` and `\splitmaxdepth` in the notes (`\footnotesep`,
    /// `\dp\strutbox`).
    pub split_top_skip: f64,
    pub split_max_depth: f64,
    /// `\floatingpenalty` (`\@MM`).
    pub floating_penalty: i32,
    /// `\footnoterule`: `\kern<above> \hrule height<rule> \kern<below>`
    /// (article: -3pt, 0.4pt, 2.6pt).
    pub rule: (f64, f64, f64),
}

/// The footnote material of one column (`\@makecol`'s `\vskip\skip\footins
/// \footnoterule \unvbox\footins`), in the column's coordinates.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InsertArea {
    /// Top edge of the `\footnoterule`.
    pub rule_top: f64,
    /// The note lines (payloads from [`Insertions::notes`]).
    pub lines: Vec<Placed>,
}

/// An `\insert` on the current page: what is left of the note's list (all
/// of it, or the remainder after a split) and its height plus depth.
#[derive(Debug, Clone)]
pub(crate) struct PageIns {
    pub(crate) list: Vec<VItem>,
    pub(crate) height_plus_depth: f64,
    /// Index in the main list of the line box it follows (`None`: held
    /// over from the previous page, ahead of the page's material).
    pub(crate) at: Option<usize>,
}

/// `vert_break(p, h, d)` (§970–§976) on a note's list: the index of the best
/// break (`list.len()` for the end), `best_height_plus_depth`, and the
/// penalty there (`EJECT_PENALTY` at the end, 0 at glue).
fn vert_break(list: &[VItem], h: f64, d: f64) -> (usize, f64, i32) {
    let mut least = AWFUL_BAD;
    let mut best = (list.len(), 0.0, EJECT_PENALTY);
    let (mut cur, mut stretch, mut fil, mut shrink, mut prev_dp) = (0.0f64, 0.0f64, false, 0.0f64, 0.0f64);
    let mut i = 0usize;
    loop {
        let pi = match list.get(i) {
            None => Some(EJECT_PENALTY),
            Some(VItem::Box { height, depth, .. }) => {
                cur += prev_dp + height;
                prev_dp = *depth;
                None
            }
            Some(VItem::Glue { .. }) => (i > 0 && matches!(list[i - 1], VItem::Box { .. })).then_some(0),
            Some(VItem::Penalty(v)) => Some(*v),
        };
        if let Some(pi) = pi.filter(|p| *p < INF_PENALTY) {
            let mut b = if cur < h {
                if fil {
                    0
                } else {
                    badness(h - cur, stretch)
                }
            } else if cur - h > shrink {
                AWFUL_BAD
            } else {
                badness(cur - h, shrink)
            };
            if b < AWFUL_BAD {
                b = if pi <= EJECT_PENALTY {
                    i64::from(pi)
                } else if b < INF_BAD {
                    b + i64::from(pi)
                } else {
                    DEPLORABLE
                };
            }
            if b <= least {
                least = b;
                best = (i, cur + prev_dp, pi);
            }
            if b == AWFUL_BAD || pi <= EJECT_PENALTY {
                return best;
            }
        }
        if let Some(VItem::Glue { width, stretch: st, shrink: sh, fil: f }) = list.get(i) {
            stretch += st;
            fil |= *f;
            shrink += sh;
            cur += prev_dp + width;
            prev_dp = 0.0;
        }
        if prev_dp > d {
            cur += prev_dp - d;
            prev_dp = d;
        }
        i += 1;
    }
}

/// `prune_page_top` (§968): glue and penalties before the first box go,
/// `\splittopskip` glue (less the box height, at least 0) comes before it.
fn prune_page_top(list: &[VItem], split_top_skip: f64) -> Vec<VItem> {
    let Some(first) = list.iter().position(|v| matches!(v, VItem::Box { .. })) else { return Vec::new() };
    let VItem::Box { height, .. } = list[first] else { unreachable!() };
    let mut out = Vec::with_capacity(list.len() - first + 1);
    out.push(VItem::Glue {
        width: (split_top_skip - height).max(0.0),
        stretch: 0.0,
        shrink: 0.0,
        fil: false,
    });
    out.extend_from_slice(&list[first..]);
    out
}

/// Height plus depth of a list packed at its natural size (`vpack`).
pub(crate) fn natural_height_plus_depth(list: &[VItem]) -> f64 {
    let (mut x, mut d) = (0.0, 0.0);
    for v in list {
        match v {
            VItem::Box { height, depth, .. } => {
                x += d + height;
                d = *depth;
            }
            VItem::Glue { width, .. } => {
                x += d + width;
                d = 0.0;
            }
            VItem::Penalty(_) => {}
        }
    }
    x + d
}

/// The insertion part of TeX's page builder for one class (§1008–§1010)
/// and the state `fire_up` reads (§1018–§1021).
pub(crate) struct InsertState<'a> {
    ins: &'a Insertions,
    /// `page_goal`.
    pub(crate) goal: f64,
    /// The class has a page-insertion record (its `\skip` is charged).
    pub(crate) started: bool,
    /// The record's `height`, `split_up`, `broken_ins`/`broken_ptr`.
    pub(crate) height: f64,
    split_up: bool,
    broken: Option<(usize, Option<usize>)>,
    pub(crate) last_ins: Option<usize>,
    pub(crate) penalties: i64,
    pub(crate) page: Vec<PageIns>,
}

impl<'a> InsertState<'a> {
    pub(crate) fn new(ins: &'a Insertions, vsize: f64) -> InsertState<'a> {
        InsertState { ins, goal: vsize, started: false, height: 0.0, split_up: false, broken: None, last_ins: None, penalties: 0, page: Vec::new() }
    }

    /// §1008–§1010 for an insertion arriving with the page at `total`,
    /// `depth` and `shrink` (TeX's `page_so_far`); `stretch`/`shrink` get
    /// `\skip\footins` when the class first appears on the page.
    /// `reserve` is what a longtable region has already taken off
    /// `\pagegoal` (`\ht\LT@foot`), which the note must fit above.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append(&mut self, list: Vec<VItem>, height_plus_depth: f64, at: Option<usize>, total: f64, depth: f64, reserve: f64, stretch: &mut f64, shrink: &mut f64) {
        let index = self.page.len();
        if !self.started {
            // §1009: `\box\footins` is void at the start of a page.
            self.started = true;
            self.goal -= self.ins.skip.0;
            *stretch += self.ins.skip.1;
            *shrink += self.ins.skip.2;
        }
        if self.split_up {
            self.penalties += i64::from(self.ins.floating_penalty);
            self.page.push(PageIns { list, height_plus_depth, at });
            return;
        }
        self.last_ins = Some(index);
        let delta = self.goal - reserve - total - depth + *shrink;
        if (height_plus_depth <= 0.0 || height_plus_depth <= delta + 1e-9) && height_plus_depth + self.height <= self.ins.max + 1e-9 {
            self.goal -= height_plus_depth;
            self.height += height_plus_depth;
            self.page.push(PageIns { list, height_plus_depth, at });
            return;
        }
        // §1010: split the insertion.
        let w = (self.goal - reserve - total - depth).min(self.ins.max - self.height);
        let (at_break, best_hd, pi) = vert_break(&list, w, self.ins.split_max_depth);
        self.height += best_hd;
        self.goal -= best_hd;
        self.split_up = true;
        self.broken = Some((index, (at_break < list.len()).then_some(at_break)));
        self.penalties += i64::from(pi);
        self.page.push(PageIns { list, height_plus_depth, at });
    }
}

/// §1018–§1021 at a page break at `end` (an index in the list the
/// insertions' `at` refer to): the note lists that go on the page, in
/// order; a split note's remainder (`prune_page_top` with `\splittopskip`)
/// and the notes after `best_ins` are pushed to `held`; notes contributed
/// after the break are dropped (their lines are not on the page).
pub(crate) fn settle_inserts(is: InsertState, end: usize, best_ins: Option<usize>, split_top_skip: f64, held: &mut Vec<PageIns>) -> Vec<Vec<VItem>> {
    let mut placed_notes: Vec<Vec<VItem>> = Vec::new();
    for (k, pi) in is.page.into_iter().enumerate() {
        if pi.at.is_some_and(|a| a >= end) {
            continue;
        }
        match best_ins {
            Some(b) if k < b => placed_notes.push(pi.list),
            Some(b) if k == b => match is.broken {
                Some((bk, Some(bp))) if is.split_up && bk == k => {
                    placed_notes.push(pi.list[..bp].to_vec());
                    let rest = prune_page_top(&pi.list[bp..], split_top_skip);
                    if !rest.is_empty() {
                        let hd = natural_height_plus_depth(&rest);
                        held.push(PageIns { list: rest, height_plus_depth: hd, at: None });
                    }
                }
                _ => placed_notes.push(pi.list),
            },
            _ => held.push(PageIns { at: None, ..pi }),
        }
    }
    placed_notes
}

/// [`break_pages_shortened`] with footnote insertions: TeX's page builder
/// charges `\skip\footins` and each note against `\pagegoal`, splits a
/// note that does not fit (`\vsplit` at `vert_break`), holds over what
/// does not go on the page, and `\@makecol` sets the notes below the text
/// (`\skip\footins`, `\footnoterule`) in the column box `\vbox
/// to\@colht`. Returns the pages and each page's note area.
pub fn break_pages_inserts(base: &PageParams, list: &[VItem], short_pages: usize, short: f64, ins: &Insertions) -> (Vec<BuiltPage>, Vec<Option<InsertArea>>) {
    break_pages_inserts_regions(base, list, short_pages, short, ins, &[])
}

/// [`break_pages_inserts`] with longtable regions. `\pagegoal` carries both
/// charges at once: `\LT@start` subtracts `\ht\LT@foot` from it
/// (longtable.sty 229) and TeX's insertion machinery subtracts
/// `\skip\footins` and the notes (§1009), so a note and a continuation foot
/// compete for the same page. `\LT@output` ends such a page by putting
/// `\copy\LT@foot` inside `\box\@cclv` *before* calling `\@makecol` (487-517),
/// so the foot is part of the column body the notes are set under, and the
/// next page opens with `\copy\LT@head\nobreak`.
pub fn break_pages_inserts_regions(
    base: &PageParams,
    list: &[VItem],
    short_pages: usize,
    short: f64,
    ins: &Insertions,
    regions: &[ResolvedRegion],
) -> (Vec<BuiltPage>, Vec<Option<InsertArea>>) {
    let mut pages: Vec<BuiltPage> = Vec::new();
    let mut areas: Vec<Option<InsertArea>> = Vec::new();
    let mut start = 0usize;
    let mut held: Vec<PageIns> = Vec::new();
    let regions = Regions(regions);
    // `\copy\LT@head\nobreak` at the top of a continuation page.
    let mut pending_head: Option<Placed3> = None;
    loop {
        while start < list.len() && !matches!(list[start], VItem::Box { .. }) {
            start += 1;
        }
        if start >= list.len() && held.is_empty() {
            break;
        }
        let page_params = PageParams {
            vsize: if pages.len() < short_pages { base.vsize - short } else { base.vsize },
            maxdepth: page_max_depth(base, regions, start),
            ..*base
        };
        let p = &page_params;
        let mut st = PageState::new();
        let mut is = InsertState::new(ins, p.vsize);
        // Held-over insertions are contributed ahead of the page's material.
        let body_less = start >= list.len();
        if body_less {
            // `\clearpage` with `\footins` not void (`\@doclearpage`): the
            // notes go on a page holding `\vbox{}` at `\topskip`.
            st.total = p.topskip;
            st.has_box = true;
        }
        // `\copy\LT@head\nobreak`: the continuation head is on the page
        // before any of its material, so it is what `\topskip` measures to
        // and what the first note sees as `\pagetotal`.
        let head = pending_head.take();
        if let Some((h, d, _)) = head {
            st.total = (p.topskip - h).max(0.0) + h;
            st.depth = d;
            st.has_box = true;
        }
        let head_reserve = regions.reserved(start);
        for h in std::mem::take(&mut held) {
            is.append(h.list, h.height_plus_depth, None, st.total, st.depth, head_reserve, &mut st.stretch, &mut st.shrink);
        }
        let mut best: Option<(usize, i64)> = None;
        let mut best_ins: Option<usize> = is.last_ins;
        let mut fired: Option<usize> = None;
        let mut i = start;
        let cost = |st: &PageState, is: &InsertState, pi: i32, reserve: f64| -> i64 {
            let goal = is.goal - reserve;
            let b = if st.total < goal {
                if st.fil {
                    0
                } else {
                    badness(goal - st.total, st.stretch)
                }
            } else if st.total - goal > st.shrink {
                AWFUL_BAD
            } else {
                badness(st.total - goal, st.shrink)
            };
            let c = if b < AWFUL_BAD {
                if pi <= EJECT_PENALTY {
                    i64::from(pi)
                } else if b < INF_BAD {
                    b + i64::from(pi) + is.penalties
                } else {
                    DEPLORABLE
                }
            } else {
                b
            };
            if is.penalties >= i64::from(INF_PENALTY) {
                AWFUL_BAD
            } else {
                c
            }
        };
        while i < list.len() && !body_less {
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
                let c = cost(&st, &is, pi, regions.reserved(i));
                if best.map_or(true, |(_, lc)| c <= lc) {
                    best = Some((i, c));
                    best_ins = is.last_ins;
                }
                if c == AWFUL_BAD || pi <= EJECT_PENALTY {
                    fired = Some(best.map_or(i, |(bi, _)| bi));
                    break;
                }
            }
            if regions.starts_at(i) {
                st.shrink = 0.0;
            }
            match &list[i] {
                VItem::Box { height, depth, payload } => {
                    let baseline = if !st.has_box { (p.topskip - height).max(0.0) + height } else { st.total + st.depth + height };
                    st.total += if st.has_box { st.depth + height } else { baseline };
                    st.has_box = true;
                    st.depth = *depth;
                    if st.depth > p.maxdepth {
                        st.total += st.depth - p.maxdepth;
                        st.depth = p.maxdepth;
                    }
                    st.lines.push(Placed { payload: *payload, baseline, height: *height, depth: *depth });
                    if st.total > is.goal - regions.reserved(i) + st.shrink + 1e-9 && st.lines.len() > 1 {
                        if let Some((bi, _)) = best {
                            fired = Some(bi);
                            break;
                        }
                    }
                    for &n in ins.after.get(payload).map(Vec::as_slice).unwrap_or(&[]) {
                        let note = ins.notes[n].clone();
                        let hd = natural_height_plus_depth(&note);
                        is.append(note, hd, Some(i), st.total, st.depth, regions.reserved(i), &mut st.stretch, &mut st.shrink);
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
        // The document's end (`\clearpage`: `\vfil\penalty-\@M`) is a
        // breakpoint like any other once insertions are on the page.
        if fired.is_none() && !is.page.is_empty() {
            let c = cost(&st, &is, EJECT_PENALTY, regions.reserved(list.len().saturating_sub(1)));
            if c == AWFUL_BAD {
                if let Some((bi, _)) = best {
                    fired = Some(bi);
                }
            } else {
                best_ins = is.last_ins;
            }
        }
        let end = match fired {
            Some(bi) => bi,
            None => list.len(),
        };
        let ejected = matches!(list.get(end), Some(VItem::Penalty(pen)) if *pen <= EJECT_PENALTY);
        let placed_notes = settle_inserts(is, end, best_ins, ins.split_top_skip, &mut held);
        let has_notes = placed_notes.iter().any(|l| !l.is_empty());
        // `\LT@output`: the continuation head opens the page and the foot
        // closes it, both inside `\box\@cclv`, so `\@makecol` packs them
        // with the rest of the body and the notes go under the foot.
        let mut body_owned: Vec<VItem> = Vec::new();
        if let Some(h) = head {
            body_owned.push(region_box(h));
        }
        if !body_less {
            body_owned.extend_from_slice(&list[start..end]);
        }
        // `\LT@output` builds `\box\@cclv` as `\vbox{\unvbox\@cclv
        // \copy\LT@foot \vss}` and only then calls `\@makecol`, so the
        // `\vss` is inside the body, ahead of `\skip\footins`: it takes the
        // column's whole slack and pushes the note block to the foot of the
        // page, leaving every finite glue at its natural size.
        let mut region_break = false;
        if fired.is_some() && end < list.len() {
            if let Some((foot, next_head)) = regions.at_break(end) {
                if let Some(f) = foot {
                    body_owned.push(region_box(f));
                }
                pending_head = next_head;
                region_break = true;
            }
        }
        let body: &[VItem] = &body_owned;
        if !has_notes {
            // The page as `break_pages_shortened` sets it; a page the
            // longtable output routine ended carries `\vss`, which absorbs
            // the difference and leaves the finite glue alone.
            let set = match glue_set(p, body) {
                _ if region_break => 0.0,
                g if g < 0.0 => g,
                g if p.flushbottom && fired.is_some() && !ejected => g,
                _ => 0.0,
            };
            let mut page = BuiltPage::default();
            let (mut total, mut depth, mut has_box) = (0.0, 0.0, false);
            for v in body {
                match v {
                    VItem::Box { height, depth: d, payload } => {
                        let baseline = if !has_box { (p.topskip - height).max(0.0) + height } else { total + depth + height };
                        total = baseline;
                        depth = *d;
                        has_box = true;
                        page.lines.push(Placed { payload: *payload, baseline, height: *height, depth: *d });
                    }
                    VItem::Glue { width, stretch, shrink, .. } => {
                        if has_box {
                            total += depth + width + if set > 0.0 { set * stretch } else { set * shrink };
                            depth = 0.0;
                        }
                    }
                    VItem::Penalty(_) => {}
                }
            }
            if let Some(last) = page.lines.last() {
                let bottom = last.baseline + (last.depth - p.maxdepth).max(0.0);
                if bottom > p.vsize + 1e-6 {
                    page.overfull_by = bottom - p.vsize;
                }
            }
            if !page.lines.is_empty() {
                pages.push(page);
                areas.push(None);
            }
        } else {
            let (page, area, _) = make_column(p, body, body_less, region_break || ejected || fired.is_none(), &placed_notes, ins, &ColumnFloats::default());
            pages.push(page);
            areas.push(Some(area));
        }
        if !body_less {
            start = end;
        }
        if fired.is_none() && held.is_empty() {
            break;
        }
        if body_less && !has_notes {
            // Nothing could be placed (a note taller than any page).
            break;
        }
    }
    (pages, areas)
}

/// The floats `\@combinefloats` wraps around a column, in `\@makecol`'s
/// order (latex.ltx, TeX Live 2025).
///
/// `\@cflt` puts the top floats above the body with `\floatsep` between
/// them (`\@comflelt` appends one after each and `\vskip-\floatsep`
/// cancels the last) and `\textfloatsep` before the body; `\@cflb` puts
/// `\textfloatsep` after the column's footnotes and then the bottom floats
/// the same way. Both are plain `\vbox`es: only `\@make@normalcolbox`
/// packs `\vbox to\@colht`, so these skips are set by the same glue set
/// ratio as the body's own glue and `\skip\footins`.
#[derive(Debug, Clone, Default)]
pub(crate) struct ColumnFloats {
    /// Height of each top and each bottom float box, in order.
    pub(crate) tops: Vec<f64>,
    pub(crate) bots: Vec<f64>,
    /// `\floatsep` and `\textfloatsep` (natural, stretch, shrink).
    pub(crate) floatsep: (f64, f64, f64),
    pub(crate) textfloatsep: (f64, f64, f64),
}

/// Where [`make_column`] put a column's float boxes: the top edge of each,
/// in the column's coordinates.
#[derive(Debug, Clone, Default)]
pub(crate) struct FloatPlacement {
    pub(crate) tops: Vec<f64>,
    pub(crate) bots: Vec<f64>,
    /// Distance from the column's top to the body's own origin (the top
    /// floats and `\textfloatsep` above it), which is what a caller adds to
    /// positions it computed in the body's coordinates.
    pub(crate) text_off: f64,
}

/// `\@makecol` for a column with footnotes: the body list, `\vfil` when
/// the page was ended by `\newpage`/`\clearpage`, `\skip\footins`, the
/// `\footnoterule` and the notes, then `\vskip-\dp` and `\@textbottom`
/// (`\vskip 0pt plus.0001fil` under `\raggedbottom`), packed `\vbox
/// to\@colht`. With `keep_depth` the last note's depth stays inside
/// `p.vsize` (bottom floats follow the notes, `\@cflb`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn make_column(p: &PageParams, body: &[VItem], body_less: bool, vfil: bool, notes: &[Vec<VItem>], ins: &Insertions, floats: &ColumnFloats) -> (BuiltPage, InsertArea, FloatPlacement) {
    // Pass 1: natural size and glue totals. `p.vsize` is `\@colht`: the
    // whole column, floats included, is one `\vbox to\@colht`.
    let (mut x, mut d, mut has_box) = (0.0f64, 0.0f64, false);
    let (mut stretch, mut shrink) = (0.0f64, 0.0f64);
    let mut fil_in_body = false;
    // `\@cflt`.
    for (k, h) in floats.tops.iter().enumerate() {
        if k > 0 {
            x += floats.floatsep.0;
            stretch += floats.floatsep.1;
            shrink += floats.floatsep.2;
        }
        x += h;
    }
    if !floats.tops.is_empty() {
        x += floats.textfloatsep.0;
        stretch += floats.textfloatsep.1;
        shrink += floats.textfloatsep.2;
    }
    // `\box\@cclv` starts here; `\topskip` is inside it.
    let text_off = x;
    if body_less {
        x += p.topskip;
        has_box = true;
    }
    for v in body {
        match v {
            VItem::Box { height, depth, .. } => {
                x = if has_box { x + d + height } else { text_off + (p.topskip - height).max(0.0) + height };
                d = *depth;
                has_box = true;
            }
            VItem::Glue { width, stretch: st, shrink: sh, fil } => {
                if has_box {
                    x += d + width;
                    d = 0.0;
                    if *fil {
                        fil_in_body = true;
                    } else {
                        stretch += st;
                    }
                    shrink += sh;
                }
            }
            VItem::Penalty(_) => {}
        }
    }
    x += d + ins.skip.0;
    d = 0.0;
    stretch += ins.skip.1;
    shrink += ins.skip.2;
    x += ins.rule.0 + ins.rule.1 + ins.rule.2;
    for v in notes.iter().flatten() {
        match v {
            VItem::Box { height, depth, .. } => {
                x += d + height;
                d = *depth;
            }
            VItem::Glue { width, stretch: st, shrink: sh, .. } => {
                x += d + width;
                d = 0.0;
                stretch += st;
                shrink += sh;
            }
            VItem::Penalty(_) => {}
        }
    }
    // `\@cflb`: `\textfloatsep` after the notes (so the notes' last depth
    // counts), the bottom floats, `\floatsep` between them.
    if !floats.bots.is_empty() {
        x += d + floats.textfloatsep.0;
        stretch += floats.textfloatsep.1;
        shrink += floats.textfloatsep.2;
        for (k, h) in floats.bots.iter().enumerate() {
            if k > 0 {
                x += floats.floatsep.0;
                stretch += floats.floatsep.1;
                shrink += floats.floatsep.2;
            }
            x += h;
        }
    }
    // `\vskip-\@outputbox@depth`: the box's height ends at the last
    // baseline.
    let natural = x;
    let excess = p.vsize - natural;
    let fil_total = f64::from(u8::from(vfil)) + if p.flushbottom { 0.0 } else { 0.0001 } + f64::from(u8::from(fil_in_body));
    // (stretch ratio for finite glue, shift given to the `\vfil`)
    let (ratio, vfil_shift) = if excess > 0.0 {
        if fil_total > 0.0 {
            (0.0, if vfil { excess / fil_total } else { 0.0 })
        } else if stretch > 0.0 {
            (excess / stretch, 0.0)
        } else {
            (0.0, 0.0)
        }
    } else if excess < 0.0 && shrink > 0.0 {
        (-(-excess / shrink).min(1.0), 0.0)
    } else {
        (0.0, 0.0)
    };
    let set_glue = |w: f64, st: f64, sh: f64| w + if ratio > 0.0 { ratio * st } else { ratio * sh };
    // Pass 2: positions.
    let mut page = BuiltPage::default();
    let mut placement = FloatPlacement::default();
    let (mut y, mut d, mut has_box) = (0.0f64, 0.0f64, false);
    for (k, h) in floats.tops.iter().enumerate() {
        if k > 0 {
            y += set_glue(floats.floatsep.0, floats.floatsep.1, floats.floatsep.2);
        }
        placement.tops.push(y);
        y += h;
    }
    if !floats.tops.is_empty() {
        y += set_glue(floats.textfloatsep.0, floats.textfloatsep.1, floats.textfloatsep.2);
    }
    placement.text_off = y;
    let text_off = y;
    if body_less {
        y += p.topskip;
        has_box = true;
    }
    for v in body {
        match v {
            VItem::Box { height, depth, payload } => {
                y = if has_box { y + d + height } else { text_off + (p.topskip - height).max(0.0) + height };
                d = *depth;
                has_box = true;
                page.lines.push(Placed { payload: *payload, baseline: y, height: *height, depth: *depth });
            }
            VItem::Glue { width, stretch: st, shrink: sh, fil } => {
                if has_box {
                    y += d + if *fil { *width } else { set_glue(*width, *st, *sh) };
                    d = 0.0;
                }
            }
            VItem::Penalty(_) => {}
        }
    }
    y += d + vfil_shift + set_glue(ins.skip.0, ins.skip.1, ins.skip.2);
    d = 0.0;
    let rule_top = y + ins.rule.0;
    y += ins.rule.0 + ins.rule.1 + ins.rule.2;
    let mut area = InsertArea { rule_top, lines: Vec::new() };
    for v in notes.iter().flatten() {
        match v {
            VItem::Box { height, depth, payload } => {
                y += d + height;
                d = *depth;
                area.lines.push(Placed { payload: *payload, baseline: y, height: *height, depth: *depth });
            }
            VItem::Glue { width, stretch: st, shrink: sh, .. } => {
                y += d + set_glue(*width, *st, *sh);
                d = 0.0;
            }
            VItem::Penalty(_) => {}
        }
    }
    if !floats.bots.is_empty() {
        y += d + set_glue(floats.textfloatsep.0, floats.textfloatsep.1, floats.textfloatsep.2);
        for (k, h) in floats.bots.iter().enumerate() {
            if k > 0 {
                y += set_glue(floats.floatsep.0, floats.floatsep.1, floats.floatsep.2);
            }
            placement.bots.push(y);
            y += h;
        }
    }
    if let Some(last) = page.lines.last() {
        let bottom = last.baseline + (last.depth - p.maxdepth).max(0.0);
        if natural > p.vsize + 1e-6 && shrink <= 0.0 {
            page.overfull_by = (natural - p.vsize).max(bottom - p.vsize).max(0.0);
        }
    }
    (page, area, placement)
}

/// Glue set ratio of a page box `\vbox to\vsize` holding `items` (from the
/// first box to the break): positive stretches by `ratio * stretch`,
/// negative shrinks by `-ratio * shrink` (capped at the available shrink,
/// TeX §676/§677). 0 when fil glue absorbs the excess or nothing stretches.
pub(crate) fn glue_set(p: &PageParams, items: &[VItem]) -> f64 {
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
            broken_penalty: Vec::new(),
            penalty_after: None,
            space_after: None,
            no_interline_first: false,
            no_interline_after: false,
            baselineskip: None,
            vskip_after: Vec::new(),
            pre_space_after: None,
            lineskip: None,
            contributed: None,
            line_penalty: Vec::new(),
            depth_after: DepthAfter::default(),
        }
    }

    /// A note of `n` 6.65pt/2.85pt lines at a 9.5pt baselineskip with the
    /// `\@footnotetext` penalties (100 + club/widow 150).
    fn note(n: usize, block: usize) -> Vec<VItem> {
        let mut v = Vec::new();
        for li in 0..n {
            if li > 0 {
                let mut pen = 100;
                if li == 1 {
                    pen += 150;
                }
                if li + 1 == n {
                    pen += 150;
                }
                v.push(VItem::Penalty(pen));
                v.push(VItem::Glue { width: 9.5 - 2.85 - 6.65, stretch: 0.0, shrink: 0.0, fil: false });
            }
            v.push(VItem::Box { height: 6.65, depth: 2.85, payload: (block, li) });
        }
        v
    }

    fn insertions(notes: Vec<Vec<VItem>>, after: &[((usize, usize), usize)]) -> Insertions {
        let mut ins = Insertions {
            notes,
            skip: (9.0, 4.0, 2.0),
            max: 8.0 * 72.27,
            split_top_skip: 6.65,
            split_max_depth: 2.85,
            floating_penalty: 20_000,
            rule: (-3.0, 0.4, 2.6),
            ..Insertions::default()
        };
        for (line, n) in after {
            ins.after.entry(*line).or_default().push(*n);
        }
        ins
    }

    #[test]
    fn a_note_shortens_the_page_by_its_height_and_the_skip() {
        // 45 lines fit alone; a 3-line note (height+depth 6.65 + 2*9.5 +
        // 2.85 = 28.5) plus \skip\footins (9) takes 37.5pt: 42 lines fit.
        let blocks = vec![para(60)];
        let list = vlist(&params(), &blocks);
        let ins = insertions(vec![note(3, 1)], &[((0, 2), 0)]);
        let (pages, areas) = break_pages_inserts(&params(), &list, 0, 0.0, &ins);
        assert_eq!(pages[0].lines.len(), 42);
        let area = areas[0].as_ref().expect("the note is on page 1");
        assert_eq!(area.lines.len(), 3);
        // \raggedbottom natural break: the skip, the rule's net 0 and the
        // first note line below the last body line's depth.
        let last = pages[0].lines.last().unwrap();
        assert!((area.lines[0].baseline - (last.baseline + last.depth + 9.0 + 6.65)).abs() < 1e-9);
        assert!((area.rule_top - (last.baseline + last.depth + 9.0 - 3.0)).abs() < 1e-9);
        assert!(areas[1].is_none());
    }

    #[test]
    fn a_long_note_is_split_and_its_remainder_opens_the_next_page() {
        // A 70-line note anchored on line 30 cannot fit: it is split at the
        // room left, the remainder is held over ahead of page 2's text.
        let blocks = vec![para(80)];
        let list = vlist(&params(), &blocks);
        let ins = insertions(vec![note(70, 1)], &[((0, 29), 0)]);
        let (pages, areas) = break_pages_inserts(&params(), &list, 0, 0.0, &ins);
        let first = areas[0].as_ref().expect("part of the note on page 1");
        let second = areas[1].as_ref().expect("the rest on page 2");
        assert_eq!(first.lines.len() + second.lines.len(), 70);
        assert!(first.lines.len() > 10);
        // The remainder starts with \splittopskip: its first baseline is
        // 6.65pt below the rule's net position.
        assert!((second.lines[0].baseline - (second.rule_top + 3.0 + 6.65)).abs() < 1e-9);
        // Page 1's text and notes fill the goal: the last note line ends
        // within \vsize.
        let bottom = first.lines.last().unwrap();
        assert!(bottom.baseline <= params().vsize + 1e-6);
    }

    #[test]
    fn a_note_after_a_split_on_the_same_page_waits() {
        // The second note's \floatingpenalty (20000) makes every later
        // breakpoint awful: the page ends before the line that holds it.
        let blocks = vec![para(80)];
        let list = vlist(&params(), &blocks);
        let ins = insertions(vec![note(70, 1), note(2, 2)], &[((0, 29), 0), ((0, 31), 1)]);
        let (pages, _) = break_pages_inserts(&params(), &list, 0, 0.0, &ins);
        assert!(pages[0].lines.len() <= 31, "line 31 moves to page 2");
        assert!(pages[1].lines.iter().any(|l| l.payload == (0, 31)));
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
            broken_penalty: Vec::new(),
            penalty_after: Some(INF_PENALTY),
            space_after: Some((12.4, 1.0, 0.0)),
            no_interline_first: false,
            no_interline_after: false,
            vskip_after: Vec::new(),
            pre_space_after: None,
            lineskip: None,
            contributed: None,
            line_penalty: Vec::new(),
            depth_after: DepthAfter::default(),
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
