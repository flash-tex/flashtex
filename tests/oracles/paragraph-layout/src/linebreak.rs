//! Line breaking and horizontal positioning.
//!
//! Two algorithms share one measuring core:
//!
//! * [`Algorithm::TotalFit`] — an original implementation of the Knuth–Plass
//!   total-fit algorithm following TeX's parameterisation: up to three passes
//!   (`pretolerance` without automatic hyphenation, `tolerance` with it, and a
//!   final pass with `emergency_stretch`), integer badness `100·r³`,
//!   badness-derived fitness classes, TeX's demerit formula, and TeX's
//!   tie-breaking rule (a later active node wins an equal-demerits tie, which is
//!   what makes `\raggedright` output coincide with first-fit when every line
//!   has zero badness).
//! * [`Algorithm::FirstFit`] — greedy: fill each line until the natural width
//!   overflows, break at the last legal break. Kept for comparison and because
//!   it is what the current compiler placeholder does.
//!
//! Legal break points (TeX rules): glue preceded by a box, a penalty below
//! `INFINITE_PENALTY`, and a kern immediately followed by glue. Discardable
//! items after a break are dropped from the start of the next line.
//!
//! Coordinates: the paragraph's own frame — `x` from the left edge of the
//! measure (before `left_skip`), `baseline_y` downward from the top of the
//! first line, both in the caller's unit. [`crate::pages::layout_pages`]
//! translates these onto pages.

use std::ops::Range;

use crate::adapter::LayoutError;
use crate::items::{FORCED_BREAK, Glue, GlueOrder, GlyphRun, INFINITE_PENALTY, Item};
use crate::metrics::FontId;
use crate::microtype::{MicroLine, Microtype, MtCtx, pt as mtpt};

/// TeX's `inf_bad`: badness of an unstretchable line that must stretch.
pub const INF_BAD: f64 = 10_000.0;
/// TeX's `awful_bad`: a line that is overfull even when fully shrunk.
pub const AWFUL_BAD: f64 = 1_073_741_823.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakMode {
    /// Interword glue is set so every line (but the last) fills the measure.
    Justified,
    /// LaTeX `\raggedright`: `\rightskip = 0pt plus 1fil`; interword glue keeps
    /// its finite stretch/shrink (LaTeX does not zero it), so a line that is
    /// too long may still shrink its spaces slightly, exactly as pdflatex does.
    RaggedRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    TotalFit,
    FirstFit,
}

/// Line-breaking parameters. Defaults are TeX's/LaTeX article's.
#[derive(Debug, Clone, PartialEq)]
pub struct LineBreakParams {
    /// `\hsize`. LaTeX 12pt article with `geometry margin=1in` on Letter:
    /// 469.75499pt (6.5in in TeX points).
    pub line_width: f64,
    pub mode: BreakMode,
    pub algorithm: Algorithm,
    /// `\pretolerance` (100). Negative disables the first pass.
    pub pretolerance: f64,
    /// `\tolerance` (200).
    pub tolerance: f64,
    /// `\emergencystretch` (0). When positive, a third pass adds it to every
    /// line's stretchability for badness and feasibility only; the chosen
    /// lines are set with their real glue (tex.web §§851, 863, 889).
    pub emergency_stretch: f64,
    /// `\linepenalty` (10).
    pub line_penalty: f64,
    /// `\adjdemerits` (10000): charged when adjacent lines' fitness classes
    /// differ by more than one.
    pub adj_demerits: f64,
    /// `\doublehyphendemerits` (10000).
    pub double_hyphen_demerits: f64,
    /// `\finalhyphendemerits` (5000).
    pub final_hyphen_demerits: f64,
    /// `\parindent`. LaTeX article: 15pt (10pt), 17.62482pt (12pt); the
    /// oracle variants and the FlashTeX compiler use 0pt.
    pub parindent: f64,
    /// `\leftskip` (0pt).
    pub left_skip: Glue,
    /// `\rightskip` (0pt). Ignored in [`BreakMode::RaggedRight`], which forces
    /// `0pt plus 1fil`.
    pub right_skip: Glue,
    /// `\baselineskip`: 12pt article 14.5pt; 10pt article 12pt; 11pt 13.6pt.
    pub baselineskip: f64,
    /// `\lineskip` (1pt).
    pub lineskip: f64,
    /// `\lineskiplimit` (0pt).
    pub lineskiplimit: f64,
    /// `\hfuzz` (0.1pt): an overfull line is diagnosed only when it exceeds
    /// the measure by more than this.
    pub hfuzz: f64,
    /// `\hbadness` (1000): an underfull line is diagnosed only when its
    /// badness exceeds this.
    pub hbadness: f64,
}

impl LineBreakParams {
    /// LaTeX `\documentclass[12pt]{article}` + `geometry{margin=1in}` on Letter,
    /// `\parindent 0pt` (the FlashTeX/oracle configuration), justified, total-fit.
    pub fn article_12pt_letter_1in() -> Self {
        LineBreakParams {
            line_width: 469.75499,
            mode: BreakMode::Justified,
            algorithm: Algorithm::TotalFit,
            pretolerance: 100.0,
            tolerance: 200.0,
            emergency_stretch: 0.0,
            line_penalty: 10.0,
            adj_demerits: 10_000.0,
            double_hyphen_demerits: 10_000.0,
            final_hyphen_demerits: 5_000.0,
            parindent: 0.0,
            left_skip: Glue::fixed(0.0),
            right_skip: Glue::fixed(0.0),
            baselineskip: 14.5,
            lineskip: 1.0,
            lineskiplimit: 0.0,
            hfuzz: 0.1,
            hbadness: 1000.0,
        }
    }

    pub fn with_width(mut self, w: f64) -> Self {
        self.line_width = w;
        self
    }

    pub fn ragged(mut self) -> Self {
        self.mode = BreakMode::RaggedRight;
        self
    }

    pub fn first_fit(mut self) -> Self {
        self.algorithm = Algorithm::FirstFit;
        self
    }

    fn effective_right_skip(&self) -> Glue {
        match self.mode {
            BreakMode::Justified => self.right_skip.clone(),
            BreakMode::RaggedRight => Glue::fil(),
        }
    }
}

/// A glyph placed on a line.
#[derive(Debug, Clone, PartialEq)]
pub struct PositionedGlyph {
    pub gid: u32,
    /// Offset from the run's `x`.
    pub x_offset: f64,
    /// Pen advance to the next glyph (advance + kern).
    pub advance: f64,
    pub cluster: Range<usize>,
}

/// A glyph run placed on a line. Font identity, glyph ids and clusters are the
/// input's, unchanged.
#[derive(Debug, Clone, PartialEq)]
pub struct PositionedRun {
    pub x: f64,
    pub baseline_y: f64,
    pub width: f64,
    pub font: FontId,
    pub size: f64,
    pub glyphs: Vec<PositionedGlyph>,
    pub source: Range<usize>,
    /// True for the hyphen materialised from a discretionary break.
    pub is_hyphen: bool,
}

/// Fitness class in TeX's numbering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fitness {
    VeryLoose = 0,
    Loose = 1,
    Decent = 2,
    Tight = 3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BreakPoint {
    /// Index of the item the line breaks at (glue/penalty/kern), or the final
    /// forced penalty.
    pub item: usize,
    pub ratio: f64,
    pub badness: f64,
    pub fitness: Fitness,
    /// Cumulative demerits through this break (total-fit); 0 for first-fit.
    pub demerits: f64,
    pub hyphenated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub index: usize,
    pub runs: Vec<PositionedRun>,
    /// Baseline, downward from the top of the paragraph's first line.
    pub baseline_y: f64,
    pub height: f64,
    pub depth: f64,
    /// Natural width of the material on the line (incl. skips, indent, hyphen).
    pub natural_width: f64,
    /// Set width after glue adjustment (equals the measure when justified and
    /// feasible; more when overfull).
    pub set_width: f64,
    pub ratio: f64,
    pub badness: f64,
    /// First and one-past-last item index of the material on the line.
    pub items: Range<usize>,
    pub hyphenated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Overfull {
    pub line: usize,
    pub excess: f64,
}

/// runtime-v1 diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticKind {
    /// The line is `excess` wider than the measure even at full shrink.
    Overfull { excess: f64 },
    /// The line's badness exceeds `\hbadness`.
    Underfull { badness: f64 },
}

/// A layout diagnostic in the shape of runtime-v1 (`severity`, `message`,
/// `source`, `recovery`) plus exact box provenance: the line number and the
/// source byte ranges of every box on the offending line.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    /// Span from the first to the last box on the line (`None` for an empty line).
    pub source: Option<Range<usize>>,
    /// What was rendered instead.
    pub recovery: Option<String>,
    pub kind: DiagnosticKind,
    /// Zero-based line index within the paragraph.
    pub line: usize,
    /// Source byte ranges of the boxes (runs) set on the line, in order; a
    /// discretionary hyphen contributes its marker range.
    pub boxes: Vec<Range<usize>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    pub algorithm: Algorithm,
    pub lines: usize,
    /// Which total-fit pass produced the result (1..=3); 0 for first-fit.
    pub pass: u8,
    pub total_demerits: f64,
    pub overfull: Vec<Overfull>,
    /// Lines whose badness exceeds the tolerance in force.
    pub underfull: Vec<(usize, f64)>,
    pub hyphenated_lines: usize,
    /// True when the paragraph needed the `emergency_stretch` pass.
    pub emergency_pass_used: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Lines {
    pub lines: Vec<Line>,
    pub breaks: Vec<BreakPoint>,
    pub stats: Stats,
    /// `\hfuzz`/`\hbadness` diagnostics with box provenance.
    pub diagnostics: Vec<Diagnostic>,
    /// Total height: last baseline + last depth.
    pub height: f64,
}

// ---------------------------------------------------------------------------
// Measuring core
// ---------------------------------------------------------------------------

struct Prefix {
    width: Vec<f64>,
    stretch: [Vec<f64>; 4],
    shrink: Vec<f64>,
}

fn order_idx(o: GlueOrder) -> usize {
    match o {
        GlueOrder::Finite => 0,
        GlueOrder::Fil => 1,
        GlueOrder::Fill => 2,
        GlueOrder::Filll => 3,
    }
}

fn prefix_sums(items: &[Item]) -> Prefix {
    let n = items.len();
    let mut p = Prefix {
        width: vec![0.0; n + 1],
        stretch: [
            vec![0.0; n + 1],
            vec![0.0; n + 1],
            vec![0.0; n + 1],
            vec![0.0; n + 1],
        ],
        shrink: vec![0.0; n + 1],
    };
    for (i, it) in items.iter().enumerate() {
        let mut w = 0.0;
        let mut st = [0.0; 4];
        let mut sh = 0.0;
        match it {
            Item::Box(b) => w = b.width,
            Item::Glue(g) => {
                w = g.width;
                st[order_idx(g.stretch_order)] = g.stretch;
                sh = g.shrink;
            }
            Item::Kern(k) => w = k.width,
            Item::Penalty(_) => {}
        }
        p.width[i + 1] = p.width[i] + w;
        for (o, s) in st.iter().enumerate() {
            p.stretch[o][i + 1] = p.stretch[o][i] + s;
        }
        p.shrink[i + 1] = p.shrink[i] + sh;
    }
    p
}

fn is_legal_break(items: &[Item], i: usize, use_automatic: bool) -> bool {
    match &items[i] {
        Item::Glue(_) => i > 0 && matches!(items[i - 1], Item::Box(_)),
        Item::Penalty(p) => p.value < INFINITE_PENALTY && (use_automatic || !p.automatic),
        Item::Kern(_) => matches!(items.get(i + 1), Some(Item::Glue(_))),
        Item::Box(_) => false,
    }
}

fn penalty_value(items: &[Item], i: usize) -> i32 {
    match &items[i] {
        Item::Penalty(p) => p.value,
        _ => 0,
    }
}

fn is_flagged(items: &[Item], i: usize) -> bool {
    matches!(&items[i], Item::Penalty(p) if p.flagged)
}

fn post_break(items: &[Item], i: usize) -> Option<&GlyphRun> {
    match &items[i] {
        Item::Penalty(p) => p.post_break.as_ref(),
        _ => None,
    }
}

fn pre_break(items: &[Item], i: usize) -> Option<&GlyphRun> {
    match &items[i] {
        Item::Penalty(p) => p.pre_break.as_ref(),
        _ => None,
    }
}

/// First item of the line that starts after a break at `after` (skips
/// discardables). `None` = paragraph start.
///
/// The result may lie *past* the next legal break when everything after
/// `after` up to that break is discardable (a forced break followed only by
/// penalties and glue: `\\` at the end of a paragraph, where
/// [`crate::items::ParagraphBuilder::finish`] appends `\penalty10000
/// \parfillskip \penalty-10000`; or two consecutive `\\`). Such a line is
/// empty, as in TeX (§837 `break_width` drops the discardables' widths and
/// §879 prunes them, but the break at the penalty is still taken, giving the
/// familiar "Underfull \hbox (badness 10000)" empty line). Every consumer
/// clamps the start to the break: [`measure`], [`set_line`] and first-fit's
/// [`resume_after_cut`].
fn line_start(items: &[Item], after: Option<usize>) -> usize {
    match after {
        None => 0,
        Some(b) => {
            let mut s = b + 1;
            // `\discretionary` no-break text is not typeset after a break there.
            if let Item::Penalty(p) = &items[b] {
                s = (s + p.replace_count).min(items.len());
            }
            // Post-break text starts the line; TeX prunes discardables only
            // when there is none (§879, §882).
            if post_break(items, b).is_none() {
                while s < items.len() && items[s].is_discardable() {
                    s += 1;
                }
            }
            s
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Measure {
    natural: f64,
    /// Target width for this line.
    target: f64,
    ratio: f64,
    badness: f64,
    /// Highest glue order present among stretch (for setting).
    stretch_order: usize,
    stretch: f64,
}

/// TeX's `badness(t, s)` (tex.web §108) on `t` and `s` rounded to scaled
/// points: `r = t·297/s` (or TeX's large-`t` approximations), `inf_bad` when
/// `r > 1290` — a stretch ratio of about 4.34, not 1.29 — otherwise
/// `(r³ + 2^17) / 2^18`, which is within a unit or two of `100·(t/s)³` but
/// is the integer pdfTeX compares with `\tolerance` and puts in demerits.
pub(crate) fn badness(t: f64, s: f64) -> f64 {
    if t <= 0.0 {
        0.0
    } else if s <= 0.0 {
        INF_BAD
    } else {
        let sp = |v: f64| (v * 65536.0).round().clamp(0.0, f64::from(i32::MAX)) as i32;
        f64::from(flashtex_microtype::arith::badness(sp(t), sp(s)))
    }
}

fn measure(
    items: &[Item],
    p: &Prefix,
    params: &LineBreakParams,
    after: Option<usize>,
    brk: usize,
    line_no: usize,
    extra_stretch: f64,
) -> Measure {
    let start = line_start(items, after);
    // A break inside the discardable run after the previous break sets an
    // empty line (see `line_start`); `start > brk` must not read backwards.
    let start = start.min(brk);
    let right = params.effective_right_skip();
    let left = &params.left_skip;
    let mut natural = p.width[brk] - p.width[start] + left.width + right.width;
    if line_no == 0 {
        natural += params.parindent;
    }
    if let Some(h) = pre_break(items, brk) {
        natural += h.width;
    }
    if let Some(h) = after.and_then(|a| post_break(items, a)) {
        natural += h.width;
    }
    let mut stretch = [0.0; 4];
    for (o, s) in stretch.iter_mut().enumerate() {
        *s = p.stretch[o][brk] - p.stretch[o][start];
    }
    stretch[order_idx(left.stretch_order)] += left.stretch;
    stretch[order_idx(right.stretch_order)] += right.stretch;
    stretch[0] += extra_stretch;
    let shrink = p.shrink[brk] - p.shrink[start] + left.shrink + right.shrink;
    let target = params.line_width;
    let order = (0..4).rev().find(|&o| stretch[o] > 0.0).unwrap_or(0);
    let (ratio, bad) = if natural < target {
        let short = target - natural;
        if order > 0 {
            (short / stretch[order], 0.0)
        } else if stretch[0] > 0.0 {
            (short / stretch[0], badness(short, stretch[0]))
        } else {
            (f64::INFINITY, INF_BAD)
        }
    } else if natural > target {
        let excess = natural - target;
        if shrink > 0.0 {
            let r = -excess / shrink;
            if r < -1.0 {
                (r, AWFUL_BAD)
            } else {
                (r, badness(excess, shrink))
            }
        } else {
            (f64::NEG_INFINITY, AWFUL_BAD)
        }
    } else {
        (0.0, 0.0)
    };
    Measure {
        natural,
        target,
        ratio,
        badness: bad,
        stretch_order: order,
        stretch: stretch[order],
    }
}

/// [`measure`], or its integer pdfTeX counterpart when microtype is active.
#[allow(clippy::too_many_arguments)]
fn measure_any(
    items: &[Item],
    p: &Prefix,
    params: &LineBreakParams,
    ctx: Option<&MtCtx<'_>>,
    after: Option<usize>,
    brk: usize,
    line_no: usize,
    extra_stretch: f64,
) -> Measure {
    let Some(c) = ctx else {
        return measure(items, p, params, after, brk, line_no, extra_stretch);
    };
    let start = line_start(items, after).min(brk);
    let right = params.effective_right_skip();
    let m = c.measure(
        after,
        start,
        brk,
        line_no,
        extra_stretch,
        &params.left_skip,
        &right,
        params.parindent,
        params.line_width,
    );
    let order = (0..4).rev().find(|&o| m.stretch[o] > 0).unwrap_or(0);
    let ratio = if m.shortfall > 0 {
        if m.stretch[order] > 0 {
            m.shortfall as f64 / m.stretch[order] as f64
        } else {
            f64::INFINITY
        }
    } else if m.shortfall < 0 {
        if m.shrink > 0 {
            m.shortfall as f64 / m.shrink as f64
        } else {
            f64::NEG_INFINITY
        }
    } else {
        0.0
    };
    // `natural < target` exactly when the (adjusted) shortfall is positive,
    // which is what TeX's fitness classes and first-fit's test read.
    Measure {
        natural: params.line_width - mtpt(m.shortfall),
        target: params.line_width,
        ratio,
        badness: m.badness,
        stretch_order: order,
        stretch: mtpt(m.stretch[order]),
    }
}

fn fitness_of(m: &Measure) -> Fitness {
    if m.natural < m.target {
        if m.badness > 99.0 {
            Fitness::VeryLoose
        } else if m.badness > 12.0 {
            Fitness::Loose
        } else {
            Fitness::Decent
        }
    } else if m.badness > 12.0 {
        Fitness::Tight
    } else {
        Fitness::Decent
    }
}

// ---------------------------------------------------------------------------
// Total-fit (Knuth–Plass)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Node {
    /// Break item index; `None` for the paragraph start.
    pos: Option<usize>,
    line: usize,
    fitness: Fitness,
    demerits: f64,
    prev: Option<usize>,
    ratio: f64,
    badness: f64,
    hyphenated: bool,
}

struct Chosen {
    breaks: Vec<BreakPoint>,
    total: f64,
}

fn total_fit_pass(
    items: &[Item],
    p: &Prefix,
    params: &LineBreakParams,
    threshold: f64,
    use_automatic: bool,
    extra_stretch: f64,
    final_pass: bool,
    ctx: Option<&MtCtx<'_>>,
) -> Option<Chosen> {
    let mut arena: Vec<Node> = vec![Node {
        pos: None,
        line: 0,
        fitness: Fitness::Decent,
        demerits: 0.0,
        prev: None,
        ratio: 0.0,
        badness: 0.0,
        hyphenated: false,
    }];
    let mut active: Vec<usize> = vec![0];
    let last = items.len() - 1;

    for b in 0..items.len() {
        if !is_legal_break(items, b, use_automatic) {
            continue;
        }
        let pen = penalty_value(items, b);
        let forced = pen <= FORCED_BREAK;
        let flagged = is_flagged(items, b);
        // Best candidate per fitness class: (demerits, arena index, ratio, badness).
        let mut best: [Option<(f64, usize, f64, f64)>; 4] = [None; 4];
        let mut minimum = f64::INFINITY;
        let mut survivors: Vec<usize> = Vec::with_capacity(active.len());
        let n_active = active.len();
        for (k, &a) in active.iter().enumerate() {
            let node = &arena[a];
            let m = measure_any(items, p, params, ctx, node.pos, b, node.line, extra_stretch);
            let overfull = m.badness >= AWFUL_BAD;
            // TeX deactivates a node once the line from it is overfull (later
            // breaks only make it longer) or when the break is forced.
            let deactivate = overfull || forced;
            let mut artificial = false;
            if m.badness > threshold {
                // Infeasible. TeX: on the final pass, if deactivating this node
                // would empty the active list with nothing feasible found for
                // this break, accept it anyway with zero ("artificial") demerits
                // and let the line be overfull/underfull rather than fail.
                if final_pass
                    && deactivate
                    && minimum == f64::INFINITY
                    && k == n_active - 1
                    && survivors.is_empty()
                {
                    artificial = true;
                } else {
                    if !deactivate {
                        survivors.push(a);
                    }
                    continue;
                }
            }
            let fit = fitness_of(&m);
            let d = if artificial {
                0.0
            } else {
                let mut d = params.line_penalty + m.badness;
                d = if d.abs() >= 10_000.0 {
                    100_000_000.0
                } else {
                    d * d
                };
                if pen > 0 {
                    d += f64::from(pen) * f64::from(pen);
                } else if pen > FORCED_BREAK {
                    d -= f64::from(pen) * f64::from(pen);
                }
                // The final forced break counts as "hyphenated" in TeX so that
                // a hyphen on the penultimate line is charged.
                let cur_hyph = flagged || b == last;
                if cur_hyph && node.hyphenated {
                    d += if b == last {
                        params.final_hyphen_demerits
                    } else {
                        params.double_hyphen_demerits
                    };
                }
                if (fit as i32 - node.fitness as i32).abs() > 1 {
                    d += params.adj_demerits;
                }
                d
            };
            let total = d + node.demerits;
            let c = fit as usize;
            // TeX tie rule: `<=`, so a later active node wins.
            if best[c].is_none_or(|(bd, _, _, _)| total <= bd) {
                best[c] = Some((total, a, m.ratio, m.badness));
            }
            if total <= minimum {
                minimum = total;
            }
            if !deactivate {
                survivors.push(a);
            }
        }
        active = survivors;
        if minimum < f64::INFINITY {
            for (c, cand) in best.iter().enumerate() {
                if let Some((total, a, ratio, bad)) = *cand
                    && total <= minimum + params.adj_demerits
                {
                    let fitness = match c {
                        0 => Fitness::VeryLoose,
                        1 => Fitness::Loose,
                        2 => Fitness::Decent,
                        _ => Fitness::Tight,
                    };
                    arena.push(Node {
                        pos: Some(b),
                        line: arena[a].line + 1,
                        fitness,
                        demerits: total,
                        prev: Some(a),
                        ratio,
                        badness: bad,
                        hyphenated: flagged,
                    });
                    active.push(arena.len() - 1);
                }
            }
        }
        if active.is_empty() {
            return None;
        }
    }

    // The last item is the forced break; pick the best node ending there.
    let mut best_end: Option<usize> = None;
    for &a in &active {
        if arena[a].pos == Some(last) {
            let better = match best_end {
                None => true,
                Some(e) => arena[a].demerits <= arena[e].demerits,
            };
            if better {
                best_end = Some(a);
            }
        }
    }
    let end = best_end?;
    let total = arena[end].demerits;
    let mut breaks = Vec::new();
    let mut cur = Some(end);
    while let Some(i) = cur {
        let n = &arena[i];
        if let Some(pos) = n.pos {
            breaks.push(BreakPoint {
                item: pos,
                ratio: n.ratio,
                badness: n.badness,
                fitness: n.fitness,
                demerits: n.demerits,
                hyphenated: n.hyphenated,
            });
        }
        cur = n.prev;
    }
    breaks.reverse();
    Some(Chosen { breaks, total })
}

// ---------------------------------------------------------------------------
// First-fit
// ---------------------------------------------------------------------------

fn first_fit(items: &[Item], p: &Prefix, params: &LineBreakParams, ctx: Option<&MtCtx<'_>>) -> Chosen {
    let mut breaks = Vec::new();
    let mut line_no = 0;
    let mut start = 0;
    let mut after: Option<usize> = None;
    let mut last_legal: Option<usize> = None;
    let mut b = 0;
    // Emits a break at `at` and advances to the next line.
    let cut = |at: usize,
               line_no: &mut usize,
               breaks: &mut Vec<BreakPoint>,
               start: &mut usize,
               after: &mut Option<usize>| {
        let m = measure_any(items, p, params, ctx, *after, at, *line_no, 0.0);
        push_break(breaks, items, at, &m);
        *after = Some(at);
        *start = line_start(items, Some(at));
        *line_no += 1;
    };
    while b < items.len() {
        if is_legal_break(items, b, true) {
            let forced = penalty_value(items, b) <= FORCED_BREAK;
            let m = measure_any(items, p, params, ctx, after, b, line_no, 0.0);
            let fits = m.natural <= m.target + 1e-9;
            if forced {
                if !fits && let Some(lb) = last_legal {
                    // Overflow before a forced break: cut at the last legal
                    // point first, then honour the forced break.
                    cut(lb, &mut line_no, &mut breaks, &mut start, &mut after);
                }
                cut(b, &mut line_no, &mut breaks, &mut start, &mut after);
                last_legal = None;
                b = resume_after_cut(items, b, start);
                continue;
            }
            if fits {
                last_legal = Some(b);
            } else {
                // Overflow: break at the last legal point, or here if none
                // (an overfull line; content is never dropped).
                let at = last_legal.unwrap_or(b);
                cut(at, &mut line_no, &mut breaks, &mut start, &mut after);
                last_legal = None;
                // Re-examine from the new line start; `b` may still lie ahead.
                b = resume_after_cut(items, at, start);
                continue;
            }
        }
        b += 1;
    }
    let total = breaks.iter().map(|bp| bp.demerits).sum();
    Chosen { breaks, total }
}

/// Where first-fit resumes scanning after a cut at `at` whose next line
/// starts at `start`: the discardables in between are never legal first-fit
/// breaks (an empty line always "fits"), except a forced penalty, which must
/// still be honoured with an empty line exactly as total-fit and TeX do.
fn resume_after_cut(items: &[Item], at: usize, start: usize) -> usize {
    (at + 1..start.min(items.len()))
        .find(|&i| penalty_value(items, i) <= FORCED_BREAK)
        .unwrap_or_else(|| start.max(at + 1))
}

fn push_break(breaks: &mut Vec<BreakPoint>, items: &[Item], at: usize, m: &Measure) {
    breaks.push(BreakPoint {
        item: at,
        ratio: m.ratio,
        badness: m.badness,
        fitness: fitness_of(m),
        demerits: 0.0,
        hyphenated: is_flagged(items, at),
    });
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Breaks `items` into lines and positions every run.
///
/// `items` must end with a forced break (see
/// [`crate::items::ParagraphBuilder::finish`]); if it does not, one is
/// appended internally so no content is ever dropped.
///
/// ## Authoritative validation
///
/// This function — not [`crate::adapter::try_layout_paragraph`] — is where
/// "is this input valid" is decided: it rejects a non-finite or
/// [`MAX_DIMEN_PT`](crate::adapter::MAX_DIMEN_PT)-overflowing dimension, and
/// an oversized item list, exactly the way [`try_layout_paragraph`] already
/// documented doing. Before this fix, that validation lived only in
/// `try_layout_paragraph`, so calling this raw entry point directly (as the
/// crate's own doc comments always showed as a legitimate, supported way to
/// use the crate — see [`crate::items::ParagraphBuilder`]'s docs) silently
/// fed NaN or overflowing dimensions straight into the breaker's `f64`
/// arithmetic, producing garbage geometry (or, depending on the exact
/// values, an internal panic) instead of a typed rejection. Silently
/// computing layout from NaN/overflowing input is never useful output, so
/// this raw entry point is made to reject it the same way the checked one
/// always did, and `try_layout_paragraph` now simply delegates to this
/// validation instead of duplicating it — one validation path, so the two
/// entry points cannot disagree about what counts as valid input.
pub fn layout_paragraph(items: &[Item], params: &LineBreakParams) -> Result<Lines, LayoutError> {
    layout_impl(items, params, None).map(|(lines, _)| lines)
}

/// [`layout_paragraph`] with pdfTeX character protrusion and font expansion
/// (`crate::microtype`): breaks are chosen and lines packed in integer
/// scaled points by pdfTeX's rules, and each line's margin kerns and
/// per-glyph expansion are returned alongside (one [`MicroLine`] per line;
/// run and glyph positions already include both). With
/// `protrude_chars <= 0 && adjust_spacing <= 0` the breaks are TeX's
/// without microtype, still measured in sp.
pub fn layout_paragraph_microtype(
    items: &[Item],
    params: &LineBreakParams,
    microtype: &Microtype,
) -> Result<(Lines, Vec<MicroLine>), LayoutError> {
    layout_impl(items, params, Some(microtype))
}

fn layout_impl(
    items: &[Item],
    params: &LineBreakParams,
    microtype: Option<&Microtype>,
) -> Result<(Lines, Vec<MicroLine>), LayoutError> {
    crate::adapter::validate_params(params)?;
    crate::adapter::validate_items(items)?;
    let owned;
    let items: &[Item] = if matches!(items.last(), Some(Item::Penalty(p)) if p.value <= FORCED_BREAK)
    {
        items
    } else {
        let mut v = items.to_vec();
        v.push(Item::Glue(Glue::fil()));
        v.push(Item::penalty(FORCED_BREAK));
        owned = v;
        &owned
    };
    let p = prefix_sums(items);
    let ctx = microtype.map(|m| MtCtx::new(items, m));
    let ctx = ctx.as_ref();

    let (chosen, pass) = match params.algorithm {
        Algorithm::FirstFit => (first_fit(items, &p, params, ctx), 0u8),
        Algorithm::TotalFit => {
            let has_emergency = params.emergency_stretch > 0.0;
            let mut result = None;
            if params.pretolerance >= 0.0
                && let Some(c) =
                    total_fit_pass(items, &p, params, params.pretolerance, false, 0.0, false, ctx)
            {
                result = Some((c, 1u8));
            }
            if result.is_none()
                && let Some(c) = total_fit_pass(
                    items,
                    &p,
                    params,
                    params.tolerance,
                    true,
                    0.0,
                    !has_emergency,
                    ctx,
                )
            {
                result = Some((c, 2u8));
            }
            if result.is_none()
                && has_emergency
                && let Some(c) = total_fit_pass(
                    items,
                    &p,
                    params,
                    params.tolerance,
                    true,
                    params.emergency_stretch,
                    true,
                    ctx,
                )
            {
                result = Some((c, 3u8));
            }
            result.expect("final pass always yields a break sequence")
        }
    };

    let tolerance_in_force = match pass {
        1 => params.pretolerance,
        _ => params.tolerance,
    };

    // Position lines.
    let mut lines = Vec::with_capacity(chosen.breaks.len());
    let mut prev: Option<usize> = None;
    let mut stats = Stats {
        algorithm: params.algorithm,
        lines: chosen.breaks.len(),
        pass,
        total_demerits: chosen.total,
        overfull: Vec::new(),
        underfull: Vec::new(),
        hyphenated_lines: 0,
        emergency_pass_used: pass == 3,
    };
    let mut diagnostics = Vec::new();
    let mut micro_lines = Vec::new();
    let mut y = 0.0;
    let mut prev_depth = 0.0;
    for (li, bp) in chosen.breaks.iter().enumerate() {
        // tex.web §§851, 863: `\emergencystretch` enters `background[2]` only
        // while the final pass computes badness and feasibility. The chosen
        // lines are then packed by `post_line_break` with `hpack(…, hsize,
        // exactly)` over the glue actually on the line (§§889, 657–667), so
        // the line is always set to the measure with its real stretch ratio,
        // and hpack's badness drives the underfull/overfull reports.
        let m = measure_any(items, &p, params, ctx, prev, bp.item, li, 0.0);
        let mut line = match ctx {
            None => set_line(items, params, prev, bp.item, li, &m),
            Some(c) => {
                let (line, micro) = set_line_mt(c, params, prev, bp.item, li, &m);
                micro_lines.push(micro);
                line
            }
        };
        if line.badness >= AWFUL_BAD || line.set_width > params.line_width + 1e-9 {
            let excess = line.set_width - params.line_width;
            stats.overfull.push(Overfull { line: li, excess });
            // §664: reported when the excess beyond total shrink exceeds
            // `\hfuzz`, or whenever `\hbadness < 100`.
            if excess > params.hfuzz || params.hbadness < 100.0 {
                diagnostics.push(diagnose(
                    &line,
                    li,
                    DiagnosticKind::Overfull { excess },
                    format!(
                        "Overfull \\hbox ({excess:.3}pt too wide) in paragraph, line {}",
                        li + 1
                    ),
                    "line set at maximum shrink; content extends past the measure",
                ));
            }
        } else if line.badness > tolerance_in_force {
            stats.underfull.push((li, line.badness));
        }
        // §660: a stretched line with finite stretch whose badness exceeds
        // `\hbadness` is reported underfull (TeX says "Loose" at ≤ 100, which
        // only shows with `\hbadness < 100`).
        if line.badness < AWFUL_BAD
            && line.badness > params.hbadness
            && line.natural_width < params.line_width
        {
            diagnostics.push(diagnose(
                &line,
                li,
                DiagnosticKind::Underfull { badness: line.badness },
                format!(
                    "Underfull \\hbox (badness {}) in paragraph, line {}",
                    line.badness,
                    li + 1
                ),
                "interword glue stretched beyond \\hbadness",
            ));
        }
        if line.hyphenated {
            stats.hyphenated_lines += 1;
        }
        // Baseline placement: TeX interline glue.
        if li == 0 {
            y = line.height;
        } else {
            let mut g = params.baselineskip - prev_depth - line.height;
            if g < params.lineskiplimit {
                g = params.lineskip;
            }
            y += prev_depth + g + line.height;
        }
        line.baseline_y = y;
        for r in &mut line.runs {
            r.baseline_y = y;
        }
        prev_depth = line.depth;
        prev = Some(bp.item);
        lines.push(line);
    }
    let height = lines.last().map_or(0.0, |l| l.baseline_y + l.depth);
    Ok((
        Lines {
            lines,
            breaks: chosen.breaks,
            stats,
            diagnostics,
            height,
        },
        micro_lines,
    ))
}

/// [`set_line`] through pdfTeX's `post_line_break` + `hpack` + `hlist_out`.
fn set_line_mt(
    c: &MtCtx<'_>,
    params: &LineBreakParams,
    after: Option<usize>,
    brk: usize,
    index: usize,
    _m: &Measure,
) -> (Line, MicroLine) {
    let items = c.items;
    let start = line_start(items, after).min(brk);
    let right = params.effective_right_skip();
    let pk = c.pack(
        after,
        start,
        brk,
        index,
        &params.left_skip,
        &right,
        params.parindent,
        params.line_width,
    );
    let line = Line {
        index,
        runs: pk.runs,
        baseline_y: 0.0,
        height: pk.height,
        depth: pk.depth,
        natural_width: pk.natural,
        set_width: pk.set_width,
        ratio: pk.ratio,
        badness: pk.badness,
        items: start..brk,
        hyphenated: is_flagged(items, brk),
    };
    (line, pk.micro)
}

fn diagnose(
    line: &Line,
    li: usize,
    kind: DiagnosticKind,
    message: String,
    recovery: &str,
) -> Diagnostic {
    let boxes: Vec<Range<usize>> = line.runs.iter().map(|r| r.source.clone()).collect();
    let source = match (boxes.first(), boxes.last()) {
        (Some(a), Some(b)) => Some(a.start.min(b.start)..a.end.max(b.end)),
        _ => None,
    };
    Diagnostic {
        severity: Severity::Warning,
        message,
        source,
        recovery: Some(recovery.to_string()),
        kind,
        line: li,
        boxes,
    }
}

fn set_line(
    items: &[Item],
    params: &LineBreakParams,
    after: Option<usize>,
    brk: usize,
    index: usize,
    m: &Measure,
) -> Line {
    let start = line_start(items, after);
    // Same clamp as `measure`: an empty line has `items == brk..brk`.
    let start = start.min(brk);
    let right = params.effective_right_skip();
    let r = if m.ratio < -1.0 { -1.0 } else { m.ratio };
    let set_glue = |g: &Glue| -> f64 {
        if r >= 0.0 {
            if order_idx(g.stretch_order) == m.stretch_order && m.stretch > 0.0 && r.is_finite() {
                g.width + r * g.stretch
            } else {
                g.width
            }
        } else {
            g.width + r * g.shrink
        }
    };
    let mut x = set_glue(&params.left_skip);
    if index == 0 {
        x += params.parindent;
    }
    let mut runs = Vec::new();
    let mut height: f64 = 0.0;
    let mut depth: f64 = 0.0;
    let mut place = |run: &GlyphRun, x: &mut f64, is_hyphen: bool| {
        let mut off = 0.0;
        let glyphs = run
            .glyphs
            .iter()
            .map(|g| {
                let pg = PositionedGlyph {
                    gid: g.gid,
                    x_offset: off,
                    advance: g.advance + g.kern,
                    cluster: g.cluster.clone(),
                };
                off += g.advance + g.kern;
                pg
            })
            .collect();
        runs.push(PositionedRun {
            x: *x,
            baseline_y: 0.0,
            width: run.width,
            font: run.font,
            size: run.size,
            glyphs,
            source: run.source.clone(),
            is_hyphen,
        });
        *x += run.width;
        height = height.max(run.height);
        depth = depth.max(run.depth);
    };
    if let Some(h) = after.and_then(|a| post_break(items, a)) {
        place(h, &mut x, false);
    }
    for it in &items[start..brk] {
        match it {
            Item::Box(b) => place(b, &mut x, false),
            Item::Glue(g) => x += set_glue(g),
            Item::Kern(k) => x += k.width,
            Item::Penalty(_) => {}
        }
    }
    let hyphenated = is_flagged(items, brk);
    if let Some(h) = pre_break(items, brk) {
        place(h, &mut x, true);
    }
    x += set_glue(&right);
    Line {
        index,
        runs,
        baseline_y: 0.0,
        height,
        depth,
        natural_width: m.natural,
        set_width: x,
        // hpack leaves `glue_set` 0 when the line has no glue of the needed
        // kind (§§658, 664).
        ratio: if m.ratio.is_finite() { m.ratio } else { 0.0 },
        badness: m.badness,
        items: start..brk,
        hyphenated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// tex.web §108 values for a 3pt stretchability at several ratios; the
    /// old `100·r³` with a 1.29 cutoff gave 10000 from ratio 1.3 on.
    #[test]
    fn badness_is_tex_integer_badness() {
        let s: f64 = 3.0;
        for (ratio, want) in [
            (0.0, 0.0),
            (0.5, 12.0),
            (0.9, 73.0),
            (1.0, 100.0),
            (1.3, 219.0),
            (1.5, 336.0),
            (2.0, 800.0),
            (4.0, 6396.0),
            (4.3, 7944.0),
            (4.4, INF_BAD),
        ] {
            let t = (s * 65536.0 * ratio).floor() / 65536.0;
            assert_eq!(badness(t, s), want, "ratio {ratio}");
        }
        assert_eq!(badness(1.0, 0.0), INF_BAD);
    }
}
