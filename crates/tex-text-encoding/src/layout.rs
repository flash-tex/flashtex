//! A minimal horizontal/vertical list model sufficient for the LaTeX text
//! constructs in this crate: `\hbox` packing with TeX's glue setting
//! (§649–§667) and ship-out rounding (§625), `\oalign`/`\o@lign`/`\ooalign`
//! (latex.ltx:632–635, i.e. `\vtop{\baselineskip0pt\lineskip.25ex\ialign{…}}`),
//! and `\vbox to <h>{\hbox{…}\vss}`.

use crate::tfm::{Scaled, ScaledFont};

pub const MAX_DIMEN: Scaled = 0o7777777777;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernKind {
    /// From a font's lig/kern program.
    Font,
    /// `\accent` kern (displayed `(for accent)`).
    Accent,
    /// Explicit `\kern`.
    Explicit,
}

/// A glue specification with orders (0 = finite, 1 = fil, 2 = fill, 3 = filll).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Glue {
    pub width: Scaled,
    pub stretch: Scaled,
    pub stretch_order: u8,
    pub shrink: Scaled,
    pub shrink_order: u8,
}

impl Glue {
    pub const ZERO: Glue = Glue {
        width: 0,
        stretch: 0,
        stretch_order: 0,
        shrink: 0,
        shrink_order: 0,
    };
    /// `\hideskip` = `-1000pt plus 1fill` (latex.ltx `\hidewidth`).
    pub const HIDEWIDTH: Glue = Glue {
        width: -1000 * 65536,
        stretch: 65536,
        stretch_order: 2,
        shrink: 0,
        shrink_order: 0,
    };
    /// `\hss` / `\vss` = `0pt plus 1fil minus 1fil`.
    pub const SS: Glue = Glue {
        width: 0,
        stretch: 65536,
        stretch_order: 1,
        shrink: 65536,
        shrink_order: 1,
    };

    pub fn finite(width: Scaled, stretch: Scaled, shrink: Scaled) -> Glue {
        Glue {
            width,
            stretch,
            stretch_order: 0,
            shrink,
            shrink_order: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Char {
        tfm: String,
        code: u8,
        width: Scaled,
        height: Scaled,
        depth: Scaled,
        ligature: bool,
    },
    Kern {
        width: Scaled,
        kind: KernKind,
    },
    /// `set_width` is the width after the enclosing box's glue was set.
    Glue {
        spec: Glue,
        set_width: Scaled,
    },
    Penalty(i32),
    HBox(BoxNode),
    VBox(VBoxNode),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BoxNode {
    pub width: Scaled,
    pub height: Scaled,
    pub depth: Scaled,
    /// `shift_amount`: positive moves the box down in an hlist.
    pub shift: Scaled,
    pub contents: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VBoxNode {
    pub width: Scaled,
    pub height: Scaled,
    pub depth: Scaled,
    pub shift: Scaled,
    /// Each row: baseline offset below the vbox reference point, and the row box.
    pub rows: Vec<(Scaled, BoxNode)>,
}

impl Node {
    pub fn char(font: &ScaledFont, code: u8) -> Node {
        let d = font.char_dims(code).unwrap_or_default();
        Node::Char {
            tfm: font.name.clone(),
            code,
            width: d.width,
            height: d.height,
            depth: d.depth,
            ligature: false,
        }
    }

    pub fn glue(spec: Glue) -> Node {
        Node::Glue {
            spec,
            set_width: spec.width,
        }
    }

    pub fn width(&self) -> Scaled {
        match self {
            Node::Char { width, .. } => *width,
            Node::Kern { width, .. } => *width,
            Node::Glue { spec, .. } => spec.width,
            Node::Penalty(_) => 0,
            Node::HBox(b) => b.width,
            Node::VBox(v) => v.width,
        }
    }

    /// (height, depth) contribution inside an hlist.
    pub fn hlist_extent(&self) -> (Scaled, Scaled) {
        match self {
            Node::Char { height, depth, .. } => (*height, *depth),
            Node::HBox(b) => (b.height - b.shift, b.depth + b.shift),
            Node::VBox(v) => (v.height - v.shift, v.depth + v.shift),
            _ => (0, 0),
        }
    }
}

/// Pascal round for glue computations.
fn round(r: f64) -> Scaled {
    crate::accent::tex_round(r)
}

/// `hpack(p, w, exactly/additional)`: natural size when `target` is `None`.
pub fn hpack(mut contents: Vec<Node>, target: Option<Scaled>) -> BoxNode {
    let (mut w, mut h, mut d) = (0, 0, 0);
    let mut total_stretch = [0i64; 4];
    let mut total_shrink = [0i64; 4];
    for n in &contents {
        w += n.width();
        let (nh, nd) = n.hlist_extent();
        h = h.max(nh);
        d = d.max(nd);
        if let Node::Glue { spec, .. } = n {
            total_stretch[spec.stretch_order as usize] += spec.stretch as i64;
            total_shrink[spec.shrink_order as usize] += spec.shrink as i64;
        }
    }
    let width = target.unwrap_or(w);
    let x = width - w;
    // (sign: 1 stretching, -1 shrinking, 0 normal), order, glue_set
    let mut sign = 0;
    let mut order = 0usize;
    let mut set = 0.0f64;
    if x > 0 {
        order = (0..4).rev().find(|&o| total_stretch[o] != 0).unwrap_or(0);
        if total_stretch[order] != 0 {
            sign = 1;
            set = x as f64 / total_stretch[order] as f64;
        }
    } else if x < 0 {
        order = (0..4).rev().find(|&o| total_shrink[o] != 0).unwrap_or(0);
        if total_shrink[order] != 0 {
            sign = -1;
            set = (-x) as f64 / total_shrink[order] as f64;
            if order == 0 && total_shrink[0] < (-x) as i64 {
                set = 1.0;
            }
        }
    }
    if sign != 0 {
        // §625 ship-out rounding with accumulated glue.
        let mut cur_glue = 0.0f64;
        let mut cur_g: Scaled = 0;
        for n in contents.iter_mut() {
            if let Node::Glue { spec, set_width } = n {
                let mut rule_wd = spec.width - cur_g;
                if sign == 1 && spec.stretch_order as usize == order {
                    cur_glue += spec.stretch as f64;
                    cur_g = round((set * cur_glue).clamp(-1e9, 1e9));
                } else if sign == -1 && spec.shrink_order as usize == order {
                    cur_glue -= spec.shrink as f64;
                    cur_g = round((set * cur_glue).clamp(-1e9, 1e9));
                }
                rule_wd += cur_g;
                *set_width = rule_wd;
            }
        }
    }
    BoxNode {
        width,
        height: h,
        depth: d,
        shift: 0,
        contents,
    }
}

/// `\oalign`-family alignment of single-column rows (`\ialign{#\crcr …}` inside
/// `\vtop{\baselineskip\z@skip \lineskip<lineskip>}` with `\lineskiplimit`).
pub fn oalign(rows: Vec<Vec<Node>>, lineskiplimit: Scaled, lineskip: Scaled) -> VBoxNode {
    let naturals: Vec<Scaled> = rows.iter().map(|r| hpack(r.clone(), None).width).collect();
    let col = naturals.iter().copied().max().unwrap_or(0);
    let boxes: Vec<BoxNode> = rows.into_iter().map(|r| hpack(r, Some(col))).collect();
    let mut placed = Vec::new();
    let mut y: Scaled = 0;
    let mut prev_depth: Option<Scaled> = None;
    for b in boxes {
        if let Some(pd) = prev_depth {
            let d = -pd - b.height; // \baselineskip is 0pt
            let glue = if d >= lineskiplimit { d } else { lineskip };
            y += pd + glue + b.height;
        }
        prev_depth = Some(b.depth);
        placed.push((y, b));
    }
    let first_h = placed.first().map_or(0, |(_, b)| b.height);
    let last = placed.last().map_or((0, 0), |(y, b)| (*y, b.depth));
    VBoxNode {
        width: col,
        height: first_h,
        depth: last.0 + last.1,
        shift: 0,
        rows: placed,
    }
}

/// `\vbox to <height>{\hbox{…}\vss}`.
pub fn vbox_to_top(height: Scaled, content: BoxNode) -> VBoxNode {
    VBoxNode {
        width: content.width,
        height,
        depth: 0,
        shift: 0,
        rows: vec![(content.height - height, content)],
    }
}

/// A glyph with its position relative to the list origin (x right, y down).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedGlyph {
    pub tfm: String,
    pub code: u8,
    pub x: Scaled,
    pub y: Scaled,
}

pub fn flatten(list: &[Node]) -> Vec<PlacedGlyph> {
    let mut out = Vec::new();
    flatten_into(list, 0, 0, &mut out);
    out
}

fn flatten_into(list: &[Node], mut x: Scaled, y: Scaled, out: &mut Vec<PlacedGlyph>) {
    for n in list {
        match n {
            Node::Char {
                tfm, code, width, ..
            } => {
                out.push(PlacedGlyph {
                    tfm: tfm.clone(),
                    code: *code,
                    x,
                    y,
                });
                x += width;
            }
            Node::Kern { width, .. } => x += width,
            Node::Glue { set_width, .. } => x += set_width,
            Node::Penalty(_) => {}
            Node::HBox(b) => {
                flatten_into(&b.contents, x, y + b.shift, out);
                x += b.width;
            }
            Node::VBox(v) => {
                for (ry, row) in &v.rows {
                    flatten_into(&row.contents, x + row.shift, y + v.shift + ry, out);
                }
                x += v.width;
            }
        }
    }
}
