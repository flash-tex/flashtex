//! `vpackage` (§668–§678) and the vertical positions `vlist_out` assigns
//! (§629–§637) to the nodes of a packaged vlist.

use crate::node::{Node, Order};
use crate::scaled::{Scaled, MAX_DIMEN};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GlueSign {
    #[default]
    Normal,
    Stretching,
    Shrinking,
}

/// `exactly`/`additional` (§644).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackSpec {
    Exactly(Scaled),
    Additional(Scaled),
}

impl PackSpec {
    pub const NATURAL: PackSpec = PackSpec::Additional(0);
}

/// A packaged vertical box.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VBox {
    pub list: Vec<Node>,
    pub width: Scaled,
    pub height: Scaled,
    pub depth: Scaled,
    pub glue_sign: GlueSign,
    pub glue_order: Order,
    /// `glue_set` as a real number (web2c `glueratio`).
    pub glue_set: f64,
}

impl VBox {
    pub fn empty() -> VBox {
        VBox::default()
    }

    /// The box as a node of an enclosing vertical list.
    pub fn as_node(&self, id: u32) -> Node {
        Node::Box(crate::node::BoxNode {
            vertical: true,
            width: self.width,
            height: self.height,
            depth: self.depth,
            shift: 0,
            id,
        })
    }
}

/// `vpackage(p, h, m, l)` (§668): packs `list` to height `spec` with the
/// box depth limited to `max_depth`.
pub fn vpackage(list: Vec<Node>, spec: PackSpec, max_depth: Scaled) -> VBox {
    let mut w: Scaled = 0;
    let mut d: Scaled = 0;
    let mut x: Scaled = 0;
    let mut total_stretch = [0 as Scaled; 4];
    let mut total_shrink = [0 as Scaled; 4];
    for p in &list {
        match p {
            Node::Box(b) => {
                x = x.wrapping_add(d).wrapping_add(b.height);
                d = b.depth;
                let s = b.shift;
                if b.width + s > w {
                    w = b.width + s;
                }
            }
            Node::Rule { width, height, depth } => {
                x = x.wrapping_add(d).wrapping_add(*height);
                d = *depth;
                if let Some(rw) = width {
                    if *rw > w {
                        w = *rw;
                    }
                }
            }
            Node::Whatsit(_) | Node::Ins(_) | Node::Mark(_) | Node::Penalty(_) => {}
            Node::Glue { spec, .. } => {
                x = x.wrapping_add(d);
                d = 0;
                x = x.wrapping_add(spec.width);
                total_stretch[spec.stretch_order.index()] += spec.stretch;
                total_shrink[spec.shrink_order.index()] += spec.shrink;
            }
            Node::Kern { width, .. } => {
                x = x.wrapping_add(d).wrapping_add(*width);
                d = 0;
            }
        }
    }
    let depth;
    if d > max_depth {
        x = x.wrapping_add(d - max_depth);
        depth = if max_depth >= 0 { max_depth } else { 0 };
    } else {
        depth = d;
    }
    let h = match spec {
        PackSpec::Additional(extra) => x.wrapping_add(extra),
        PackSpec::Exactly(h) => h,
    };
    let excess = h.wrapping_sub(x);
    let mut vbox = VBox { list, width: w, height: h, depth, ..VBox::default() };
    if excess == 0 {
        return vbox;
    }
    if excess > 0 {
        let o = highest_order(&total_stretch);
        vbox.glue_order = o;
        vbox.glue_sign = GlueSign::Stretching;
        if total_stretch[o.index()] != 0 {
            vbox.glue_set = f64::from(excess) / f64::from(total_stretch[o.index()]);
        } else {
            vbox.glue_sign = GlueSign::Normal;
        }
    } else {
        let o = highest_order(&total_shrink);
        vbox.glue_order = o;
        vbox.glue_sign = GlueSign::Shrinking;
        if total_shrink[o.index()] != 0 {
            vbox.glue_set = f64::from(-excess) / f64::from(total_shrink[o.index()]);
        } else {
            vbox.glue_sign = GlueSign::Normal;
        }
        // §677: overfull with finite shrink.
        if total_shrink[o.index()] < -excess && o == Order::Normal && !vbox.list.is_empty() {
            vbox.glue_set = 1.0;
        }
    }
    vbox
}

/// `vpack(p, natural)`.
pub fn vpack_natural(list: Vec<Node>) -> VBox {
    vpackage(list, PackSpec::NATURAL, MAX_DIMEN)
}

fn highest_order(totals: &[Scaled; 4]) -> Order {
    if totals[3] != 0 {
        Order::Filll
    } else if totals[2] != 0 {
        Order::Fill
    } else if totals[1] != 0 {
        Order::Fil
    } else {
        Order::Normal
    }
}

/// Vertical offset of each node's reference point from the top of `vbox`
/// as `vlist_out` computes it (§629, §634–§637, with web2c's `vet_glue`
/// and cumulative rounding): for boxes and rules the baseline (top +
/// height), for glue and kerns their top edge. Other nodes get the
/// current position.
pub fn vlist_positions(vbox: &VBox) -> Vec<Scaled> {
    const BILLION: f64 = 1_000_000_000.0;
    let mut out = Vec::with_capacity(vbox.list.len());
    let mut cur_v: i64 = 0;
    let mut cur_g: i64 = 0;
    let mut cur_glue: f64 = 0.0;
    for p in &vbox.list {
        match p {
            Node::Box(b) => {
                cur_v += i64::from(b.height);
                out.push(cur_v as Scaled);
                cur_v += i64::from(b.depth);
            }
            Node::Rule { height, depth, .. } => {
                cur_v += i64::from(*height);
                out.push(cur_v as Scaled);
                cur_v += i64::from(*depth);
            }
            Node::Glue { spec, .. } => {
                out.push(cur_v as Scaled);
                let mut rule_ht = i64::from(spec.width) - cur_g;
                match vbox.glue_sign {
                    GlueSign::Stretching if spec.stretch_order == vbox.glue_order => {
                        cur_glue += f64::from(spec.stretch);
                        let t = (vbox.glue_set * cur_glue).clamp(-BILLION, BILLION);
                        cur_g = t.round() as i64;
                    }
                    GlueSign::Shrinking if spec.shrink_order == vbox.glue_order => {
                        cur_glue -= f64::from(spec.shrink);
                        let t = (vbox.glue_set * cur_glue).clamp(-BILLION, BILLION);
                        cur_g = t.round() as i64;
                    }
                    _ => {}
                }
                rule_ht += cur_g;
                cur_v += rule_ht;
            }
            Node::Kern { width, .. } => {
                out.push(cur_v as Scaled);
                cur_v += i64::from(*width);
            }
            _ => out.push(cur_v as Scaled),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{BoxNode, GlueSpec};
    use crate::scaled::pt;

    #[test]
    fn stretches_to_exact_height_and_positions_lines() {
        let list = vec![
            Node::Box(BoxNode::hbox(pt(100.0), pt(7.0), pt(2.0), 0)),
            Node::glue(GlueSpec::new(pt(3.0), pt(2.0), 0)),
            Node::Box(BoxNode::hbox(pt(100.0), pt(7.0), pt(2.0), 1)),
        ];
        let b = vpackage(list, PackSpec::Exactly(pt(22.0)), pt(4.0));
        assert_eq!(b.glue_sign, GlueSign::Stretching);
        assert!((b.glue_set - 1.5).abs() < 1e-12);
        let pos = vlist_positions(&b);
        assert_eq!(pos[0], pt(7.0));
        assert_eq!(pos[2], pt(22.0));
        assert_eq!(b.depth, pt(2.0));
    }

    #[test]
    fn depth_beyond_max_depth_moves_into_height() {
        let list = vec![Node::Box(BoxNode::hbox(0, pt(5.0), pt(9.0), 0))];
        let b = vpackage(list, PackSpec::NATURAL, pt(4.0));
        assert_eq!((b.height, b.depth), (pt(10.0), pt(4.0)));
    }
}
