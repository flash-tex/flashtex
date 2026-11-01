//! Final positioning: `hlist_out` (§§619–628) and `vlist_out` (§§629–637),
//! as mirrored by pdfTeX's `pdf_hlist_out`/`pdf_vlist_out`. Glue is rounded
//! with the accumulated `cur_glue`/`cur_g` scheme so positions match pdfTeX
//! to the scaled point.

use crate::display::tex_round;
use crate::node::{BoxNode, CharMetrics, GlueKind, GlueSign, LeaderKind, ListKind, Node, Rule, is_running};
use crate::scaled::Scaled;

/// A positioned output item. Coordinates are TeX's `cur_h`/`cur_v`: `h`
/// grows rightward, `v` grows downward, both relative to the origin passed
/// to [`ship_out`].
#[derive(Debug, Clone, PartialEq)]
pub enum ShipEvent {
    /// A character with its reference point on the baseline.
    Char { font: u32, ch: u32, h: Scaled, v: Scaled },
    /// A filled rule; `(h, v)` is the bottom-left corner.
    Rule { h: Scaled, v: Scaled, width: Scaled, height: Scaled },
    /// A whatsit reached at `(h, v)`; `in_leaders` is TeX's `doing_leaders`.
    Whatsit { tag: u32, h: Scaled, v: Scaled, in_leaders: bool },
    /// A box is being output with its reference point at `(h, v)`.
    BoxStart { kind: ListKind, h: Scaled, v: Scaled, width: Scaled, height: Scaled, depth: Scaled },
    BoxEnd,
}

/// `vet_glue` (§625).
fn vet_glue(g: f64) -> f64 {
    if g > 1_000_000_000.0 {
        1_000_000_000.0
    } else if g < -1_000_000_000.0 {
        -1_000_000_000.0
    } else {
        g
    }
}

/// Ships out `node` with its reference point at `(h, v)`.
pub fn ship_out(node: &BoxNode, h: Scaled, v: Scaled, metrics: &dyn CharMetrics) -> Vec<ShipEvent> {
    let mut out = Out { events: Vec::new(), doing_leaders: false, metrics };
    match node.kind {
        ListKind::H => out.hlist_out(node, h, v),
        ListKind::V => out.vlist_out(node, h, v),
    }
    out.events
}

struct Out<'a> {
    events: Vec<ShipEvent>,
    doing_leaders: bool,
    metrics: &'a dyn CharMetrics,
}

/// Accumulated glue rounding state for one box (§619 `cur_g`, `cur_glue`).
struct GlueRounder {
    cur_g: Scaled,
    cur_glue: f64,
}

impl GlueRounder {
    /// §625 / §634: returns the rounded glue size.
    fn size(&mut self, this_box: &BoxNode, spec: &crate::node::GlueSpec) -> Scaled {
        let mut rule = spec.width - self.cur_g;
        match this_box.glue_sign {
            GlueSign::Stretching if spec.stretch_order == this_box.glue_order => {
                self.cur_glue += f64::from(spec.stretch);
                self.cur_g = tex_round(vet_glue(this_box.glue_set * self.cur_glue));
            }
            GlueSign::Shrinking if spec.shrink_order == this_box.glue_order => {
                self.cur_glue -= f64::from(spec.shrink);
                self.cur_g = tex_round(vet_glue(this_box.glue_set * self.cur_glue));
            }
            _ => {}
        }
        rule += self.cur_g;
        rule
    }
}

impl Out<'_> {
    fn hlist_out(&mut self, this_box: &BoxNode, h: Scaled, v: Scaled) {
        self.events.push(ShipEvent::BoxStart {
            kind: ListKind::H,
            h,
            v,
            width: this_box.width,
            height: this_box.height,
            depth: this_box.depth,
        });
        let mut g = GlueRounder { cur_g: 0, cur_glue: 0.0 };
        let base_line = v;
        let left_edge = h;
        let mut cur_h = h;
        for p in &this_box.list {
            match p {
                Node::Char { font, ch } | Node::Ligature { font, ch, .. } => {
                    self.events.push(ShipEvent::Char { font: *font, ch: *ch, h: cur_h, v: base_line });
                    cur_h += self.metrics.char_dims(*font, *ch).0;
                }
                Node::Box(b) => {
                    // §623
                    if b.list.is_empty() {
                        cur_h += b.width;
                    } else {
                        let edge = cur_h;
                        let cv = base_line + b.shift;
                        match b.kind {
                            ListKind::V => self.vlist_out(b, cur_h, cv),
                            ListKind::H => self.hlist_out(b, cur_h, cv),
                        }
                        cur_h = edge + b.width;
                    }
                }
                Node::Rule(r) => {
                    self.hrule_out(this_box, r, cur_h, base_line);
                    cur_h += r.width;
                }
                Node::Whatsit(w) => {
                    self.events.push(ShipEvent::Whatsit { tag: w.tag, h: cur_h, v: base_line, in_leaders: self.doing_leaders });
                }
                Node::Glue(gn) => {
                    // §625
                    let mut rule_wd = g.size(this_box, &gn.spec);
                    if let GlueKind::Leaders(kind, leader) = &gn.kind {
                        match leader.as_ref() {
                            Node::Rule(lr) => {
                                let r = Rule { width: rule_wd, height: lr.height, depth: lr.depth };
                                self.hrule_out(this_box, &r, cur_h, base_line);
                                cur_h += rule_wd;
                                continue;
                            }
                            Node::Box(lb) => {
                                // §626
                                let leader_wd = lb.width;
                                if leader_wd > 0 && rule_wd > 0 {
                                    rule_wd += 10;
                                    let edge = cur_h + rule_wd;
                                    let mut lx = 0;
                                    // §627
                                    if *kind == LeaderKind::Aligned {
                                        let save_h = cur_h;
                                        cur_h = left_edge + leader_wd * ((cur_h - left_edge) / leader_wd);
                                        if cur_h < save_h {
                                            cur_h += leader_wd;
                                        }
                                    } else {
                                        let lq = rule_wd / leader_wd;
                                        let lr = rule_wd % leader_wd;
                                        if *kind == LeaderKind::Centered {
                                            cur_h += lr / 2;
                                        } else {
                                            lx = lr / (lq + 1);
                                            cur_h += (lr - (lq - 1) * lx) / 2;
                                        }
                                    }
                                    while cur_h + leader_wd <= edge {
                                        // §628
                                        let cv = base_line + lb.shift;
                                        let save_h = cur_h;
                                        let outer = self.doing_leaders;
                                        self.doing_leaders = true;
                                        match lb.kind {
                                            ListKind::V => self.vlist_out(lb, cur_h, cv),
                                            ListKind::H => self.hlist_out(lb, cur_h, cv),
                                        }
                                        self.doing_leaders = outer;
                                        cur_h = save_h + leader_wd + lx;
                                    }
                                    cur_h = edge - 10;
                                    continue;
                                }
                            }
                            _ => {}
                        }
                    }
                    cur_h += rule_wd;
                }
                Node::Kern { width, .. } | Node::MarginKern { width, .. } | Node::Math { width, .. } => cur_h += *width,
                _ => {}
            }
        }
        self.events.push(ShipEvent::BoxEnd);
    }

    /// §624.
    fn hrule_out(&mut self, this_box: &BoxNode, r: &Rule, h: Scaled, base_line: Scaled) {
        let mut rule_ht = r.height;
        let mut rule_dp = r.depth;
        if is_running(rule_ht) {
            rule_ht = this_box.height;
        }
        if is_running(rule_dp) {
            rule_dp = this_box.depth;
        }
        rule_ht += rule_dp;
        if rule_ht > 0 && r.width > 0 {
            self.events.push(ShipEvent::Rule { h, v: base_line + rule_dp, width: r.width, height: rule_ht });
        }
    }

    fn vlist_out(&mut self, this_box: &BoxNode, h: Scaled, v: Scaled) {
        self.events.push(ShipEvent::BoxStart {
            kind: ListKind::V,
            h,
            v,
            width: this_box.width,
            height: this_box.height,
            depth: this_box.depth,
        });
        let mut g = GlueRounder { cur_g: 0, cur_glue: 0.0 };
        let left_edge = h;
        let mut cur_v = v - this_box.height;
        let top_edge = cur_v;
        for p in &this_box.list {
            match p {
                Node::Box(b) => {
                    // §632
                    if b.list.is_empty() {
                        cur_v += b.height + b.depth;
                    } else {
                        cur_v += b.height;
                        let save_v = cur_v;
                        let ch = left_edge + b.shift;
                        match b.kind {
                            ListKind::V => self.vlist_out(b, ch, cur_v),
                            ListKind::H => self.hlist_out(b, ch, cur_v),
                        }
                        cur_v = save_v + b.depth;
                    }
                }
                Node::Rule(r) => {
                    cur_v = self.vrule_out(this_box, r.width, r.height + r.depth, left_edge, cur_v);
                }
                Node::Whatsit(w) => {
                    self.events.push(ShipEvent::Whatsit { tag: w.tag, h: left_edge, v: cur_v, in_leaders: self.doing_leaders });
                }
                Node::Glue(gn) => {
                    // §634
                    let mut rule_ht = g.size(this_box, &gn.spec);
                    if let GlueKind::Leaders(kind, leader) = &gn.kind {
                        match leader.as_ref() {
                            Node::Rule(lr) => {
                                cur_v = self.vrule_out(this_box, lr.width, rule_ht, left_edge, cur_v);
                                continue;
                            }
                            Node::Box(lb) => {
                                // §635
                                let leader_ht = lb.height + lb.depth;
                                if leader_ht > 0 && rule_ht > 0 {
                                    rule_ht += 10;
                                    let edge = cur_v + rule_ht;
                                    let mut lx = 0;
                                    // §636
                                    if *kind == LeaderKind::Aligned {
                                        let save_v = cur_v;
                                        cur_v = top_edge + leader_ht * ((cur_v - top_edge) / leader_ht);
                                        if cur_v < save_v {
                                            cur_v += leader_ht;
                                        }
                                    } else {
                                        let lq = rule_ht / leader_ht;
                                        let lr = rule_ht % leader_ht;
                                        if *kind == LeaderKind::Centered {
                                            cur_v += lr / 2;
                                        } else {
                                            lx = lr / (lq + 1);
                                            cur_v += (lr - (lq - 1) * lx) / 2;
                                        }
                                    }
                                    while cur_v + leader_ht <= edge {
                                        // §637
                                        let ch = left_edge + lb.shift;
                                        cur_v += lb.height;
                                        let save_v = cur_v;
                                        let outer = self.doing_leaders;
                                        self.doing_leaders = true;
                                        match lb.kind {
                                            ListKind::V => self.vlist_out(lb, ch, cur_v),
                                            ListKind::H => self.hlist_out(lb, ch, cur_v),
                                        }
                                        self.doing_leaders = outer;
                                        cur_v = save_v - lb.height + leader_ht + lx;
                                    }
                                    cur_v = edge - 10;
                                    continue;
                                }
                            }
                            _ => {}
                        }
                    }
                    cur_v += rule_ht;
                }
                Node::Kern { width, .. } | Node::MarginKern { width, .. } => cur_v += *width,
                Node::Char { .. } | Node::Ligature { .. } => panic!("This can't happen (vlistout)"),
                _ => {}
            }
        }
        self.events.push(ShipEvent::BoxEnd);
    }

    /// §633: returns the new `cur_v`.
    fn vrule_out(&mut self, this_box: &BoxNode, width: Scaled, thickness: Scaled, h: Scaled, cur_v: Scaled) -> Scaled {
        let rule_wd = if is_running(width) { this_box.width } else { width };
        let cur_v = cur_v + thickness;
        if thickness > 0 && rule_wd > 0 {
            self.events.push(ShipEvent::Rule { h, v: cur_v, width: rule_wd, height: thickness });
        }
        cur_v
    }
}

/// Convenience: positions of all whatsits (outside leaders) in shipout order.
pub fn whatsit_positions(events: &[ShipEvent]) -> Vec<(u32, Scaled, Scaled)> {
    events
        .iter()
        .filter_map(|e| match e {
            ShipEvent::Whatsit { tag, h, v, in_leaders: false } => Some((*tag, *h, *v)),
            _ => None,
        })
        .collect()
}
