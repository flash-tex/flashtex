//! Log printing and box display (tex.web part 5 §§57–71 and part 12
//! §§173–198): `print_ln`/`print_nl`, `short_display`, `show_box`.

use crate::node::{
    BoxNode, CharMetrics, GlueKind, GlueOrder, GlueSign, KernKind, LeaderKind, ListKind, MarginSide, Node, is_running,
};
use crate::scaled::{Scaled, print_scaled};

/// A transcript printer that tracks `file_offset` so that `print_ln`,
/// `print_nl` and line wrapping behave exactly like TeX's log output (§§57–62).
#[derive(Debug, Clone)]
pub struct Printer {
    pub out: String,
    pub file_offset: usize,
    /// `max_print_line` (texmf.cnf, default 79). `usize::MAX` disables wrapping.
    pub max_print_line: usize,
}

impl Default for Printer {
    fn default() -> Self {
        Printer { out: String::new(), file_offset: 0, max_print_line: 79 }
    }
}

impl Printer {
    pub fn new(max_print_line: usize) -> Printer {
        Printer { out: String::new(), file_offset: 0, max_print_line }
    }
    /// `print_ln` (§57).
    pub fn print_ln(&mut self) {
        self.out.push('\n');
        self.file_offset = 0;
    }
    /// `print_char` (§58).
    pub fn print_char(&mut self, c: char) {
        self.out.push(c);
        self.file_offset += 1;
        if self.file_offset == self.max_print_line {
            self.print_ln();
        }
    }
    /// `print` (§59) for ASCII strings.
    pub fn print(&mut self, s: &str) {
        for c in s.chars() {
            self.print_char(c);
        }
    }
    /// `print_nl` (§62).
    pub fn print_nl(&mut self, s: &str) {
        if self.file_offset > 0 {
            self.print_ln();
        }
        self.print(s);
    }
    /// `print_esc` (§63) with `\escapechar` = `\`.
    pub fn print_esc(&mut self, s: &str) {
        self.print_char('\\');
        self.print(s);
    }
    pub fn print_int(&mut self, n: i64) {
        self.print(&n.to_string());
    }
    pub fn print_scaled(&mut self, s: Scaled) {
        self.print(&print_scaled(s));
    }
}

/// Settings for `show_box` (`\showboxdepth`, `\showboxbreadth`).
#[derive(Debug, Clone, Copy)]
pub struct ShowLimits {
    pub depth: i32,
    pub breadth: i32,
}

impl ShowLimits {
    /// LaTeX's defaults (latex.ltx lines 524–525): both `-1`.
    pub const LATEX: ShowLimits = ShowLimits { depth: -1, breadth: -1 };
    pub const ALL: ShowLimits = ShowLimits { depth: i32::MAX / 2, breadth: i32::MAX / 2 };
}

/// `print_glue` (§177).
pub fn print_glue(p: &mut Printer, d: Scaled, order: GlueOrder, unit: Option<&str>) {
    p.print_scaled(d);
    match order {
        GlueOrder::Normal => {
            if let Some(u) = unit {
                p.print(u);
            }
        }
        o => {
            p.print("fil");
            for _ in 1..o.index() {
                p.print_char('l');
            }
        }
    }
}

/// `print_spec` (§178).
pub fn print_spec(p: &mut Printer, spec: &crate::node::GlueSpec, unit: Option<&str>) {
    p.print_scaled(spec.width);
    if let Some(u) = unit {
        p.print(u);
    }
    if spec.stretch != 0 {
        p.print(" plus ");
        print_glue(p, spec.stretch, spec.stretch_order, unit);
    }
    if spec.shrink != 0 {
        p.print(" minus ");
        print_glue(p, spec.shrink, spec.shrink_order, unit);
    }
}

/// `print_rule_dimen` (§176).
fn print_rule_dimen(p: &mut Printer, d: Scaled) {
    if is_running(d) { p.print_char('*') } else { p.print_scaled(d) }
}

/// `short_display` (§§174–175). `font_in_short_display` is threaded through.
pub fn short_display(p: &mut Printer, list: &[Node], font_in_short_display: &mut Option<u32>, metrics: &dyn CharMetrics) {
    let mut i = 0;
    while i < list.len() {
        match &list[i] {
            Node::Char { font, ch } => {
                if *font_in_short_display != Some(*font) {
                    p.print(&metrics.font_identifier(*font));
                    p.print_char(' ');
                    *font_in_short_display = Some(*font);
                }
                print_ascii(p, *ch);
            }
            Node::Box(_) | Node::Insert(_) | Node::Whatsit(_) | Node::Mark(_) | Node::Adjust(_) => p.print("[]"),
            Node::Rule(_) => p.print_char('|'),
            Node::Glue(g) => {
                if !g.zero_glue_ref {
                    p.print_char(' ');
                }
            }
            Node::Math { .. } => p.print_char('$'),
            Node::Ligature { original, .. } => short_display(p, original, font_in_short_display, metrics),
            Node::Disc { pre, post, replace_count } => {
                short_display(p, pre, font_in_short_display, metrics);
                short_display(p, post, font_in_short_display, metrics);
                i += usize::from(*replace_count);
            }
            Node::Kern { .. } | Node::MarginKern { .. } | Node::Penalty(_) => {}
        }
        i += 1;
    }
}

/// `print_ASCII` (§68) for printable characters; others use `^^` notation.
fn print_ascii(p: &mut Printer, ch: u32) {
    match char::from_u32(ch) {
        Some(c) if (32..127).contains(&ch) => p.print_char(c),
        _ if ch < 64 => {
            p.print("^^");
            p.print_char(char::from_u32(ch + 64).unwrap_or('?'));
        }
        _ if ch == 127 => p.print("^^?"),
        _ if ch < 256 => p.print(&format!("^^{:02x}", ch)),
        Some(c) => p.print_char(c),
        None => p.print_char('?'),
    }
}

/// `show_box` (§198) applied to a single box node.
pub fn show_box(p: &mut Printer, node: &Node, limits: ShowLimits, metrics: &dyn CharMetrics) {
    let breadth = if limits.breadth <= 0 { 5 } else { limits.breadth };
    let mut prefix = String::new();
    show_node_list(p, std::slice::from_ref(node), &mut prefix, limits.depth, breadth, metrics);
    p.print_ln();
}

/// `show_node_list` (§182). `prefix` is TeX's "current string".
pub fn show_node_list(
    p: &mut Printer,
    list: &[Node],
    prefix: &mut String,
    depth_threshold: i32,
    breadth_max: i32,
    metrics: &dyn CharMetrics,
) {
    if prefix.len() as i64 > i64::from(depth_threshold) {
        if !list.is_empty() {
            p.print(" []");
        }
        return;
    }
    let mut n = 0;
    for node in list {
        p.print_ln();
        let pre = prefix.clone();
        p.print(&pre);
        n += 1;
        if n > breadth_max {
            p.print("etc.");
            return;
        }
        display_node(p, node, prefix, depth_threshold, breadth_max, metrics);
    }
}

fn sublist(p: &mut Printer, list: &[Node], prefix: &mut String, dt: i32, bm: i32, m: &dyn CharMetrics, c: char) {
    prefix.push(c);
    show_node_list(p, list, prefix, dt, bm, m);
    prefix.pop();
}

fn display_box(p: &mut Printer, b: &BoxNode) {
    p.print_esc(if b.kind == ListKind::H { "h" } else { "v" });
    p.print("box(");
    p.print_scaled(b.height);
    p.print_char('+');
    p.print_scaled(b.depth);
    p.print(")x");
    p.print_scaled(b.width);
    // §186
    let g = b.glue_set;
    if g != 0.0 && b.glue_sign != GlueSign::Normal {
        p.print(", glue set ");
        if b.glue_sign == GlueSign::Shrinking {
            p.print("- ");
        }
        if g.abs() > 20000.0 {
            if g > 0.0 { p.print_char('>') } else { p.print("< -") }
            print_glue(p, 20000 * 65536, b.glue_order, None);
        } else {
            print_glue(p, tex_round(65536.0 * g), b.glue_order, None);
        }
    }
    if b.shift != 0 {
        p.print(", shifted ");
        p.print_scaled(b.shift);
    }
}

/// web2c `zround`: round half away from zero, clamped to 32-bit range.
pub fn tex_round(r: f64) -> i32 {
    if r > 2147483647.0 {
        2147483647
    } else if r < -2147483647.0 {
        -2147483647
    } else if r >= 0.0 {
        (r + 0.5) as i32
    } else {
        (r - 0.5) as i32
    }
}

fn display_node(p: &mut Printer, node: &Node, prefix: &mut String, dt: i32, bm: i32, m: &dyn CharMetrics) {
    match node {
        Node::Char { font, ch } => {
            p.print(&m.font_identifier(*font));
            p.print_char(' ');
            print_ascii(p, *ch);
        }
        Node::Box(b) => {
            display_box(p, b);
            sublist(p, &b.list, prefix, dt, bm, m, '.');
        }
        Node::Rule(r) => {
            p.print_esc("rule(");
            print_rule_dimen(p, r.height);
            p.print_char('+');
            print_rule_dimen(p, r.depth);
            p.print(")x");
            print_rule_dimen(p, r.width);
        }
        Node::Insert(ins) => {
            p.print_esc("insert");
            p.print_int(i64::from(ins.number));
            p.print(", natural size ");
            p.print_scaled(ins.height);
            p.print("; split(");
            print_spec(p, &ins.split_top, None);
            p.print_char(',');
            p.print_scaled(ins.depth);
            p.print("); float cost ");
            p.print_int(i64::from(ins.float_cost));
            sublist(p, &ins.list, prefix, dt, bm, m, '.');
        }
        Node::Whatsit(w) => p.print_esc(&w.display),
        Node::Glue(g) => match &g.kind {
            GlueKind::Leaders(kind, leader) => {
                p.print_esc("");
                match kind {
                    LeaderKind::Centered => p.print_char('c'),
                    LeaderKind::Expanded => p.print_char('x'),
                    LeaderKind::Aligned => {}
                }
                p.print("leaders ");
                print_spec(p, &g.spec, None);
                sublist(p, std::slice::from_ref(leader.as_ref()), prefix, dt, bm, m, '.');
            }
            kind => {
                p.print_esc("glue");
                match kind {
                    GlueKind::Param(sp) => {
                        p.print_char('(');
                        p.print_esc(sp.name());
                        p.print_char(')');
                    }
                    GlueKind::CondMath => {
                        p.print_char('(');
                        p.print_esc("nonscript");
                        p.print_char(')');
                    }
                    GlueKind::Mu => {
                        p.print_char('(');
                        p.print_esc("mskip");
                        p.print_char(')');
                    }
                    _ => {}
                }
                if *kind != GlueKind::CondMath {
                    p.print_char(' ');
                    if *kind == GlueKind::Mu { print_spec(p, &g.spec, Some("mu")) } else { print_spec(p, &g.spec, None) }
                }
            }
        },
        Node::Kern { width, kind } => {
            if *kind != KernKind::Mu {
                p.print_esc("kern");
                if *kind != KernKind::Normal {
                    p.print_char(' ');
                }
                p.print_scaled(*width);
                if *kind == KernKind::Accent {
                    p.print(" (for accent)");
                }
            } else {
                p.print_esc("mkern");
                p.print_scaled(*width);
                p.print("mu");
            }
        }
        Node::MarginKern { width, side } => {
            // pdftex.web: <Display kern |p|> for margin_kern_node
            p.print_esc("kern");
            p.print_scaled(*width);
            p.print(if *side == MarginSide::Left { " (left margin)" } else { " (right margin)" });
        }
        Node::Math { on, width } => {
            p.print_esc("math");
            p.print(if *on { "on" } else { "off" });
            if *width != 0 {
                p.print(", surrounded ");
                p.print_scaled(*width);
            }
        }
        Node::Ligature { font, ch, original, left_boundary, right_boundary } => {
            p.print(&m.font_identifier(*font));
            p.print_char(' ');
            print_ascii(p, *ch);
            p.print(" (ligature ");
            if *left_boundary {
                p.print_char('|');
            }
            let mut f = Some(*font);
            short_display(p, original, &mut f, m);
            if *right_boundary {
                p.print_char('|');
            }
            p.print_char(')');
        }
        Node::Penalty(n) => {
            p.print_esc("penalty ");
            p.print_int(i64::from(*n));
        }
        Node::Disc { pre, post, replace_count } => {
            p.print_esc("discretionary");
            if *replace_count > 0 {
                p.print(" replacing ");
                p.print_int(i64::from(*replace_count));
            }
            sublist(p, pre, prefix, dt, bm, m, '.');
            sublist(p, post, prefix, dt, bm, m, '|');
        }
        Node::Mark(text) => {
            p.print_esc("mark");
            p.print_char('{');
            p.print(text);
            p.print_char('}');
        }
        Node::Adjust(list) => {
            p.print_esc("vadjust");
            sublist(p, list, prefix, dt, bm, m, '.');
        }
    }
}

/// Renders `\showbox` body text (what follows `> \box<n>=` in the log).
pub fn show_box_string(node: Option<&Node>, limits: ShowLimits, metrics: &dyn CharMetrics) -> String {
    let mut p = Printer::new(usize::MAX);
    match node {
        None => p.print("void"),
        Some(n) => show_box(&mut p, n, limits, metrics),
    }
    p.out
}
