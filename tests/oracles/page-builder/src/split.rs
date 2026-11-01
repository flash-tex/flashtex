//! Breaking vertical lists: `prune_page_top` (§968), `vert_break`
//! (§970–§976) and `\vsplit` (§977–§979).

use crate::node::{GlueKind, GlueSpec, Node};
use crate::pack::{vpack_natural, vpackage, PackSpec, VBox};
use crate::scaled::{badness, Scaled, AWFUL_BAD, DEPLORABLE, EJECT_PENALTY, INF_BAD, INF_PENALTY};

/// `prune_page_top` (§968): deletes glue, kerns and penalties before the
/// first box or rule and puts `\splittopskip` glue ahead of it.
pub fn prune_page_top(list: Vec<Node>, split_top_skip: GlueSpec) -> Vec<Node> {
    let mut out = Vec::with_capacity(list.len() + 1);
    let mut iter = list.into_iter();
    for p in iter.by_ref() {
        match &p {
            Node::Box(_) | Node::Rule { .. } => {
                let (h, _) = p.height_depth();
                let mut g = split_top_skip;
                g.width = if g.width > h { g.width - h } else { 0 };
                out.push(Node::Glue { spec: g, kind: GlueKind::SplitTopSkip });
                out.push(p);
                break;
            }
            Node::Whatsit(_) | Node::Mark(_) | Node::Ins(_) => out.push(p),
            Node::Glue { .. } | Node::Kern { .. } | Node::Penalty(_) => {}
        }
    }
    out.extend(iter);
    out
}

/// Result of `vert_break`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertBreak {
    /// Index of the best breakpoint; `list.len()` is the implicit final
    /// forced break (TeX's `null`).
    pub index: usize,
    /// `best_height_plus_depth` (§971).
    pub height_plus_depth: Scaled,
    /// The cost of the chosen break (`least_cost`).
    pub cost: i32,
}

/// `vert_break(p, h, d)` (§970): the best place to break `list` for a box
/// of height `h` with maximum depth `d`.
pub fn vert_break(list: &[Node], h: Scaled, d: Scaled) -> VertBreak {
    let mut least_cost = AWFUL_BAD;
    let mut best = VertBreak { index: list.len(), height_plus_depth: 0, cost: AWFUL_BAD };
    // active_height[1..6]: natural, stretch by order (normal..filll), shrink.
    let mut cur_height: Scaled = 0;
    let mut stretch = [0 as Scaled; 4];
    let mut shrink: Scaled = 0;
    let mut prev_dp: Scaled = 0;
    let mut i = 0usize;
    loop {
        enum Step {
            Break(i32),
            UpdateHeights,
            NotFound,
        }
        let step = if i >= list.len() {
            Step::Break(EJECT_PENALTY)
        } else {
            match &list[i] {
                Node::Box(_) | Node::Rule { .. } => {
                    let (ht, dp) = list[i].height_depth();
                    cur_height = cur_height.wrapping_add(prev_dp).wrapping_add(ht);
                    prev_dp = dp;
                    Step::NotFound
                }
                Node::Whatsit(_) | Node::Mark(_) | Node::Ins(_) => Step::NotFound,
                Node::Glue { .. } => {
                    // `prev_p` starts as `p` itself: an initial glue is not legal.
                    let prev = if i == 0 { &list[0] } else { &list[i - 1] };
                    if prev.precedes_break() {
                        Step::Break(0)
                    } else {
                        Step::UpdateHeights
                    }
                }
                Node::Kern { .. } => match list.get(i + 1) {
                    Some(Node::Glue { .. }) => Step::Break(0),
                    _ => Step::UpdateHeights,
                },
                Node::Penalty(v) => Step::Break(*v),
            }
        };
        let mut do_update = matches!(step, Step::UpdateHeights);
        if let Step::Break(pi) = step {
            if pi < INF_PENALTY {
                // §975
                let mut b = if cur_height < h {
                    if stretch[1] != 0 || stretch[2] != 0 || stretch[3] != 0 {
                        0
                    } else {
                        badness(h - cur_height, stretch[0])
                    }
                } else if cur_height - h > shrink {
                    AWFUL_BAD
                } else {
                    badness(cur_height - h, shrink)
                };
                if b < AWFUL_BAD {
                    b = if pi <= EJECT_PENALTY {
                        pi
                    } else if b < INF_BAD {
                        b + pi
                    } else {
                        DEPLORABLE
                    };
                }
                if b <= least_cost {
                    least_cost = b;
                    best = VertBreak { index: i, height_plus_depth: cur_height.wrapping_add(prev_dp), cost: b };
                }
                if b == AWFUL_BAD || pi <= EJECT_PENALTY {
                    return best;
                }
            }
            if matches!(list.get(i), Some(Node::Glue { .. } | Node::Kern { .. })) {
                do_update = true;
            }
        }
        if do_update {
            // §976
            match &list[i] {
                Node::Kern { width, .. } => {
                    cur_height = cur_height.wrapping_add(prev_dp).wrapping_add(*width);
                }
                Node::Glue { spec, .. } => {
                    stretch[spec.stretch_order.index()] += spec.stretch;
                    shrink += spec.shrink;
                    // Infinite shrinkage is an error; TeX makes it finite.
                    cur_height = cur_height.wrapping_add(prev_dp).wrapping_add(spec.width);
                }
                _ => unreachable!(),
            }
            prev_dp = 0;
        }
        // not_found:
        if prev_dp > d {
            cur_height = cur_height.wrapping_add(prev_dp - d);
            prev_dp = d;
        }
        i += 1;
    }
}

/// `\vsplit` result.
#[derive(Debug, Clone, PartialEq)]
pub struct Split {
    /// The extracted box, packed `to h` with `\splitmaxdepth`.
    pub extracted: VBox,
    /// What stays in the register (`None` = void), pruned and packed
    /// naturally.
    pub remainder: Option<VBox>,
    /// Index into the original list where the break occurred.
    pub break_index: usize,
    /// `\splitfirstmark`/`\splitbotmark` ids.
    pub first_mark: Option<u32>,
    pub bot_mark: Option<u32>,
}

/// `\vsplit n to h` (§977) on the list of a vbox.
pub fn vsplit(list: Vec<Node>, h: Scaled, split_top_skip: GlueSpec, split_max_depth: Scaled) -> Split {
    let q = vert_break(&list, h, split_max_depth).index;
    let mut first_mark = None;
    let mut bot_mark = None;
    for n in &list[..q] {
        if let Node::Mark(m) = n {
            if first_mark.is_none() {
                first_mark = Some(*m);
            }
            bot_mark = Some(*m);
        }
    }
    let mut head = list;
    let tail = head.split_off(q);
    let tail = prune_page_top(tail, split_top_skip);
    let remainder = if tail.is_empty() { None } else { Some(vpack_natural(tail)) };
    Split {
        extracted: vpackage(head, PackSpec::Exactly(h), split_max_depth),
        remainder,
        break_index: q,
        first_mark,
        bot_mark,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::BoxNode;
    use crate::scaled::pt;

    fn line(id: u32) -> Node {
        Node::Box(BoxNode::hbox(pt(300.0), pt(7.0), pt(2.0), id))
    }

    fn lines(n: u32) -> Vec<Node> {
        let mut v = Vec::new();
        for i in 0..n {
            if i > 0 {
                v.push(Node::glue(GlueSpec::fixed(pt(3.0))));
            }
            v.push(line(i));
        }
        v
    }

    #[test]
    fn vsplit_takes_the_lines_that_fit() {
        // Line i has its baseline at 7 + 12i; 3 lines need 7+24+2 = 33pt.
        let s = vsplit(lines(5), pt(31.0), GlueSpec::fixed(pt(10.0)), pt(4.0));
        let taken = s.extracted.list.iter().filter(|n| matches!(n, Node::Box(_))).count();
        assert_eq!(taken, 3);
        let rem = s.remainder.expect("remainder");
        assert!(matches!(rem.list[0], Node::Glue { kind: GlueKind::SplitTopSkip, spec } if spec.width == pt(3.0)));
        assert_eq!(rem.list.iter().filter(|n| matches!(n, Node::Box(_))).count(), 2);
    }

    #[test]
    fn prune_keeps_marks_and_drops_discardables() {
        let l = vec![Node::Penalty(5), Node::Mark(1), Node::kern(pt(1.0)), line(0)];
        let p = prune_page_top(l, GlueSpec::fixed(pt(10.0)));
        assert_eq!(p.len(), 3);
        assert_eq!(p[0], Node::Mark(1));
    }
}
