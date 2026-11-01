//! Packaging: `badness` (§108), `hpack` (§§649–667) and `vpackage`
//! (§§668–678), including over/underfull box reporting with TeX's exact
//! message text.

use crate::display::{Printer, ShowLimits, short_display, show_box};
use crate::node::{BoxNode, CharMetrics, GlueKind, GlueOrder, GlueSign, ListKind, Node, Rule};
use crate::scaled::{MAX_DIMEN, Scaled};

/// `inf_bad` (§108).
pub const INF_BAD: i32 = 10000;

/// `badness(t, s)` (§108), bit-exact.
pub fn badness(t: Scaled, s: Scaled) -> i32 {
    if t == 0 {
        return 0;
    }
    if s <= 0 {
        return INF_BAD;
    }
    let (t, s) = (i64::from(t), i64::from(s));
    let r = if t <= 7230584 {
        (t * 297) / s
    } else if s >= 1663497 {
        t / (s / 297)
    } else {
        t
    };
    if r > 1290 { INF_BAD } else { ((r * r * r + 0o400000) / 0o1000000) as i32 }
}

/// Box specification: `to <dimen>` (`exactly`) or `spread <dimen>`
/// (`additional`); a bare `\hbox` is `spread 0pt` (§645).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackSpec {
    Exactly(Scaled),
    Additional(Scaled),
}

impl PackSpec {
    pub const NATURAL: PackSpec = PackSpec::Additional(0);
}

/// Parameters consulted by the packagers.
#[derive(Debug, Clone, Copy)]
pub struct PackParams {
    pub hbadness: i32,
    pub vbadness: i32,
    pub hfuzz: Scaled,
    pub vfuzz: Scaled,
    pub overfull_rule: Scaled,
    pub show_limits: ShowLimits,
    /// `max_print_line` used when rendering the diagnostic.
    pub max_print_line: usize,
}

impl Default for PackParams {
    /// LaTeX kernel values (latex.ltx lines 493–529) with `\overfullrule=0pt`
    /// as set by the standard classes' `final` option.
    fn default() -> Self {
        PackParams {
            hbadness: 1000,
            vbadness: 1000,
            hfuzz: 6554,
            vfuzz: 6554,
            overfull_rule: 0,
            show_limits: ShowLimits::LATEX,
            max_print_line: 79,
        }
    }
}

/// Where the packaging request originates, for the diagnostic tail (§663).
#[derive(Debug, Clone, Copy, Default)]
pub struct PackOrigin {
    /// Current input line (`line`).
    pub line: i32,
    /// `pack_begin_line`: >0 paragraph start line, <0 alignment, 0 none.
    pub pack_begin_line: i32,
    pub output_active: bool,
}

/// Kind of box diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    Underfull,
    Loose,
    Tight,
    Overfull,
}

/// A box-quality report as TeX prints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackReport {
    pub kind: ReportKind,
    pub vertical: bool,
    /// Badness for under/loose/tight; excess in sp for overfull.
    pub amount: i32,
    /// Exact transcript text, starting with TeX's leading `print_ln`.
    pub text: String,
}

/// Result of a packaging call.
#[derive(Debug, Clone)]
pub struct PackOutcome {
    pub node: BoxNode,
    /// `last_badness` (`\badness`).
    pub last_badness: i32,
    pub report: Option<PackReport>,
    /// Material migrated out by `hpack` when `adjust_tail` was set (§655).
    pub adjust: Vec<Node>,
}

/// Font-expansion hook for pdfTeX's `\pdfadjustspacing` (KC-102). The default
/// implementation performs no expansion.
pub trait ExpansionHook {
    fn enabled(&self) -> bool {
        false
    }
}

/// `hpack(p, w, m)` (§649). When `collect_adjust` is true the list is packed
/// with `adjust_tail<>null`, moving ins/mark/adjust nodes to `adjust`.
pub fn hpack(
    mut list: Vec<Node>,
    spec: PackSpec,
    collect_adjust: bool,
    params: &PackParams,
    origin: PackOrigin,
    metrics: &dyn CharMetrics,
) -> PackOutcome {
    let mut r = BoxNode::null(ListKind::H);
    let (mut h, mut d, mut x): (Scaled, Scaled, Scaled) = (0, 0, 0);
    let mut total_stretch = [0 as Scaled; 4];
    let mut total_shrink = [0 as Scaled; 4];
    let mut adjust = Vec::new();

    let mut kept = Vec::with_capacity(list.len());
    for node in list.drain(..) {
        match &node {
            Node::Char { font, ch } => {
                let (w, ht, dp) = metrics.char_dims(*font, *ch);
                x = x.wrapping_add(w);
                h = h.max(ht);
                d = d.max(dp);
            }
            Node::Ligature { font, ch, .. } => {
                let (w, ht, dp) = metrics.char_dims(*font, *ch);
                x = x.wrapping_add(w);
                h = h.max(ht);
                d = d.max(dp);
            }
            Node::Box(b) => {
                x = x.wrapping_add(b.width);
                let s = b.shift;
                h = h.max(b.height - s);
                d = d.max(b.depth + s);
            }
            Node::Rule(rule) => {
                // §653: running dimensions are null_flag and thus never win.
                x = x.wrapping_add(rule.width);
                h = h.max(rule.height);
                d = d.max(rule.depth);
            }
            Node::Insert(_) | Node::Mark(_) | Node::Adjust(_) if collect_adjust => {
                // §655
                match node {
                    Node::Adjust(inner) => adjust.extend(inner),
                    other => adjust.push(other),
                }
                continue;
            }
            Node::Glue(g) => {
                // §656
                x = x.wrapping_add(g.spec.width);
                total_stretch[g.spec.stretch_order.index()] += g.spec.stretch;
                total_shrink[g.spec.shrink_order.index()] += g.spec.shrink;
                if let GlueKind::Leaders(_, leader) = &g.kind {
                    let (lh, ld) = leader_hd(leader);
                    h = h.max(lh);
                    d = d.max(ld);
                }
            }
            Node::Kern { width, .. } | Node::MarginKern { width, .. } | Node::Math { width, .. } => {
                x = x.wrapping_add(*width);
            }
            _ => {}
        }
        kept.push(node);
    }
    r.list = kept;
    r.height = h;
    r.depth = d;

    // §657
    let w = match spec {
        PackSpec::Additional(w) => x + w,
        PackSpec::Exactly(w) => w,
    };
    r.width = w;
    let x = w - x;
    let mut last_badness = 0;
    let mut report = None;
    if x == 0 {
        r.glue_sign = GlueSign::Normal;
        r.glue_order = GlueOrder::Normal;
        r.glue_set = 0.0;
    } else if x > 0 {
        // §658
        let o = top_order(&total_stretch);
        r.glue_order = o;
        r.glue_sign = GlueSign::Stretching;
        if total_stretch[o.index()] != 0 {
            r.glue_set = f64::from(x) / f64::from(total_stretch[o.index()]);
        } else {
            r.glue_sign = GlueSign::Normal;
            r.glue_set = 0.0;
        }
        if o == GlueOrder::Normal && !r.list.is_empty() {
            // §660
            last_badness = badness(x, total_stretch[0]);
            if last_badness > params.hbadness {
                let kind = if last_badness > 100 { ReportKind::Underfull } else { ReportKind::Loose };
                report = Some((kind, last_badness));
            }
        }
    } else {
        // §664
        let o = top_order(&total_shrink);
        r.glue_order = o;
        r.glue_sign = GlueSign::Shrinking;
        if total_shrink[o.index()] != 0 {
            r.glue_set = f64::from(-x) / f64::from(total_shrink[o.index()]);
        } else {
            r.glue_sign = GlueSign::Normal;
            r.glue_set = 0.0;
        }
        if total_shrink[o.index()] < -x && o == GlueOrder::Normal && !r.list.is_empty() {
            last_badness = 1000000;
            r.glue_set = 1.0;
            // §666
            let excess = -x - total_shrink[0];
            if excess > params.hfuzz || params.hbadness < 100 {
                if params.overfull_rule > 0 && excess > params.hfuzz {
                    r.list.push(Node::Rule(Rule { width: params.overfull_rule, ..Rule::running() }));
                }
                report = Some((ReportKind::Overfull, excess));
            }
        } else if o == GlueOrder::Normal && !r.list.is_empty() {
            // §667
            last_badness = badness(-x, total_shrink[0]);
            if last_badness > params.hbadness {
                report = Some((ReportKind::Tight, last_badness));
            }
        }
    }
    let report = report.map(|(kind, amount)| render_report(&r, kind, false, amount, params, origin, metrics));
    PackOutcome { node: r, last_badness, report, adjust }
}

fn leader_hd(leader: &Node) -> (Scaled, Scaled) {
    match leader {
        Node::Box(b) => (b.height, b.depth),
        Node::Rule(r) => (r.height, r.depth),
        _ => (0, 0),
    }
}

fn leader_width(leader: &Node) -> Scaled {
    match leader {
        Node::Box(b) => b.width,
        Node::Rule(r) => r.width,
        _ => 0,
    }
}

/// §659 / §665.
fn top_order(totals: &[Scaled; 4]) -> GlueOrder {
    if totals[3] != 0 {
        GlueOrder::Filll
    } else if totals[2] != 0 {
        GlueOrder::Fill
    } else if totals[1] != 0 {
        GlueOrder::Fil
    } else {
        GlueOrder::Normal
    }
}

/// `vpackage(p, h, m, l)` (§668). `vpack` is `vpackage(.., MAX_DIMEN)`.
pub fn vpackage(
    list: Vec<Node>,
    spec: PackSpec,
    max_depth: Scaled,
    params: &PackParams,
    origin: PackOrigin,
    metrics: &dyn CharMetrics,
) -> PackOutcome {
    let mut r = BoxNode::null(ListKind::V);
    let (mut w, mut d, mut x): (Scaled, Scaled, Scaled) = (0, 0, 0);
    let mut total_stretch = [0 as Scaled; 4];
    let mut total_shrink = [0 as Scaled; 4];
    for node in &list {
        match node {
            Node::Box(b) => {
                // §670
                x = x + d + b.height;
                d = b.depth;
                w = w.max(b.width + b.shift);
            }
            Node::Rule(rule) => {
                x = x + d + rule.height;
                d = rule.depth;
                w = w.max(rule.width);
            }
            Node::Glue(g) => {
                // §671
                x += d;
                d = 0;
                x += g.spec.width;
                total_stretch[g.spec.stretch_order.index()] += g.spec.stretch;
                total_shrink[g.spec.shrink_order.index()] += g.spec.shrink;
                if let GlueKind::Leaders(_, leader) = &g.kind {
                    w = w.max(leader_width(leader));
                }
            }
            Node::Kern { width, .. } | Node::MarginKern { width, .. } => {
                x = x + d + width;
                d = 0;
            }
            Node::Char { .. } | Node::Ligature { .. } => panic!("This can't happen (vpack)"),
            _ => {}
        }
    }
    r.list = list;
    r.width = w;
    if d > max_depth {
        x = x + d - max_depth;
        r.depth = max_depth;
    } else {
        r.depth = d;
    }
    // §672
    let h = match spec {
        PackSpec::Additional(h) => x + h,
        PackSpec::Exactly(h) => h,
    };
    r.height = h;
    let x = h - x;
    let mut last_badness = 0;
    let mut report = None;
    if x == 0 {
        r.glue_sign = GlueSign::Normal;
        r.glue_order = GlueOrder::Normal;
        r.glue_set = 0.0;
    } else if x > 0 {
        let o = top_order(&total_stretch);
        r.glue_order = o;
        r.glue_sign = GlueSign::Stretching;
        if total_stretch[o.index()] != 0 {
            r.glue_set = f64::from(x) / f64::from(total_stretch[o.index()]);
        } else {
            r.glue_sign = GlueSign::Normal;
            r.glue_set = 0.0;
        }
        if o == GlueOrder::Normal && !r.list.is_empty() {
            last_badness = badness(x, total_stretch[0]);
            if last_badness > params.vbadness {
                let kind = if last_badness > 100 { ReportKind::Underfull } else { ReportKind::Loose };
                report = Some((kind, last_badness));
            }
        }
    } else {
        let o = top_order(&total_shrink);
        r.glue_order = o;
        r.glue_sign = GlueSign::Shrinking;
        if total_shrink[o.index()] != 0 {
            r.glue_set = f64::from(-x) / f64::from(total_shrink[o.index()]);
        } else {
            r.glue_sign = GlueSign::Normal;
            r.glue_set = 0.0;
        }
        if total_shrink[o.index()] < -x && o == GlueOrder::Normal && !r.list.is_empty() {
            last_badness = 1000000;
            r.glue_set = 1.0;
            let excess = -x - total_shrink[0];
            if excess > params.vfuzz || params.vbadness < 100 {
                report = Some((ReportKind::Overfull, excess));
            }
        } else if o == GlueOrder::Normal && !r.list.is_empty() {
            last_badness = badness(-x, total_shrink[0]);
            if last_badness > params.vbadness {
                report = Some((ReportKind::Tight, last_badness));
            }
        }
    }
    let report = report.map(|(kind, amount)| render_report(&r, kind, true, amount, params, origin, metrics));
    PackOutcome { node: r, last_badness, report, adjust: Vec::new() }
}

/// `vpack` (§668): `vpackage` with unconstrained depth.
pub fn vpack(list: Vec<Node>, spec: PackSpec, params: &PackParams, origin: PackOrigin, metrics: &dyn CharMetrics) -> PackOutcome {
    vpackage(list, spec, MAX_DIMEN, params, origin, metrics)
}

/// Renders the report exactly as §§660–667 and §§674–678 print it.
fn render_report(
    r: &BoxNode,
    kind: ReportKind,
    vertical: bool,
    amount: i32,
    params: &PackParams,
    origin: PackOrigin,
    metrics: &dyn CharMetrics,
) -> PackReport {
    let mut p = Printer::new(params.max_print_line);
    let what = if vertical { "\\vbox" } else { "\\hbox" };
    p.print_ln();
    match kind {
        ReportKind::Underfull | ReportKind::Loose => {
            p.print_nl(if kind == ReportKind::Underfull { "Underfull" } else { "Loose" });
            p.print(&format!(" {what} (badness "));
            p.print_int(i64::from(amount));
        }
        ReportKind::Tight => {
            p.print_nl(&format!("Tight {what} (badness "));
            p.print_int(i64::from(amount));
        }
        ReportKind::Overfull => {
            p.print_nl(&format!("Overfull {what} ("));
            p.print_scaled(amount);
            p.print(if vertical { "pt too high" } else { "pt too wide" });
        }
    }
    // §663 / §675 common ending
    if origin.output_active {
        p.print(") has occurred while \\output is active");
    } else {
        if origin.pack_begin_line != 0 {
            if vertical || origin.pack_begin_line < 0 {
                p.print(") in alignment at lines ");
            } else {
                p.print(") in paragraph at lines ");
            }
            p.print_int(i64::from(origin.pack_begin_line.abs()));
            p.print("--");
        } else {
            p.print(") detected at line ");
        }
        p.print_int(i64::from(origin.line));
        if vertical {
            p.print_ln();
        }
    }
    if !vertical {
        p.print_ln();
        let mut f = None;
        short_display(&mut p, &r.list, &mut f, metrics);
        p.print_ln();
    }
    // begin_diagnostic; show_box(r); end_diagnostic(true)
    show_box(&mut p, &Node::Box(r.clone()), params.show_limits, metrics);
    p.print_nl("");
    p.print_ln();
    PackReport { kind, vertical, amount, text: p.out }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badness_table() {
        assert_eq!(badness(0, 0), 0);
        assert_eq!(badness(1, 0), INF_BAD);
        assert_eq!(badness(65536, 65536), 100);
        assert_eq!(badness(32768, 65536), 12);
        assert_eq!(badness(2 * 65536, 65536), 800);
        assert_eq!(badness(5 * 65536, 65536), INF_BAD);
    }
}
