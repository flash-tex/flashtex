//! One-node-per-line text format shared with `oracle/gen.py`.
//!
//! ```text
//! H <h> <d> <w>                 hbox          V <h> <d> <w>   vbox
//! R <h> <d>                     rule          M | W           mark | whatsit
//! G <w> <st> <sto> <sh> <sho> [T]   glue (T = \topskip glue), orders 0..3
//! K <w>                         kern          P <v>           penalty
//! I <n> <h> <d> <w> <st> <sto> <sh> <sho> <cost> <k>   insert + k node lines
//! ```
//! All dimensions in scaled points.

use crate::node::{BoxNode, GlueKind, GlueSpec, InsNode, Node, Order};

fn order(o: Order) -> u8 {
    o as u8
}

fn order_from(v: &str) -> Result<Order, String> {
    Ok(match v {
        "0" => Order::Normal,
        "1" => Order::Fil,
        "2" => Order::Fill,
        "3" => Order::Filll,
        _ => return Err(format!("bad order {v}")),
    })
}

pub fn glue_to_string(g: &GlueSpec) -> String {
    format!("{} {} {} {} {}", g.width, g.stretch, order(g.stretch_order), g.shrink, order(g.shrink_order))
}

pub fn parse_glue(f: &[&str]) -> Result<GlueSpec, String> {
    if f.len() < 5 {
        return Err("short glue".into());
    }
    let n = |s: &str| s.parse::<i32>().map_err(|e| format!("{s}: {e}"));
    Ok(GlueSpec {
        width: n(f[0])?,
        stretch: n(f[1])?,
        stretch_order: order_from(f[2])?,
        shrink: n(f[3])?,
        shrink_order: order_from(f[4])?,
    })
}

/// The signature line of a node (insert contents are not included).
pub fn node_line(n: &Node) -> String {
    match n {
        Node::Box(b) => format!("{} {} {} {}", if b.vertical { "V" } else { "H" }, b.height, b.depth, b.width),
        Node::Rule { height, depth, .. } => format!("R {height} {depth}"),
        Node::Ins(i) => format!("I {} {}", i.number, i.height),
        Node::Mark(_) => "M".into(),
        Node::Whatsit(_) => "W".into(),
        Node::Glue { spec, kind } => {
            format!("G {}{}", glue_to_string(spec), if *kind == GlueKind::TopSkip { " T" } else { "" })
        }
        Node::Kern { width, .. } => format!("K {width}"),
        Node::Penalty(v) => format!("P {v}"),
    }
}

/// Parses node lines starting at `lines[*pos]`, `count` top-level nodes.
/// Box ids are assigned from `next_id`.
pub fn parse_nodes(lines: &[&str], pos: &mut usize, count: usize, next_id: &mut u32) -> Result<Vec<Node>, String> {
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let line = lines.get(*pos).ok_or("unexpected end of node list")?;
        *pos += 1;
        let f: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| -> Result<i32, String> {
            f.get(i).ok_or(format!("missing field in {line}"))?.parse::<i32>().map_err(|e| format!("{line}: {e}"))
        };
        let node = match f.first().copied() {
            Some("H") | Some("V") => {
                let id = *next_id;
                *next_id += 1;
                Node::Box(BoxNode { vertical: f[0] == "V", height: n(1)?, depth: n(2)?, width: n(3)?, shift: 0, id })
            }
            Some("R") => Node::Rule { width: None, height: n(1)?, depth: n(2)? },
            Some("M") => Node::Mark(0),
            Some("W") => Node::Whatsit(0),
            Some("G") => Node::Glue {
                spec: parse_glue(&f[1..])?,
                kind: if f.get(6) == Some(&"T") { GlueKind::TopSkip } else { GlueKind::Normal },
            },
            Some("K") => Node::Kern { width: n(1)?, explicit: true },
            Some("P") => Node::Penalty(n(1)?),
            Some("I") => {
                let number = n(1)? as u8;
                let height = n(2)?;
                let split_max_depth = n(3)?;
                let split_top_skip = parse_glue(&f[4..9])?;
                let float_cost = n(9)?;
                let k = n(10)? as usize;
                let list = parse_nodes(lines, pos, k, next_id)?;
                Node::Ins(InsNode { number, height, split_max_depth, split_top_skip, float_cost, list })
            }
            _ => return Err(format!("bad node line {line:?}")),
        };
        out.push(node);
    }
    Ok(out)
}
