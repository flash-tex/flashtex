//! Test support: parse pdfTeX `\showbox`/`\showlists` output and the font
//! dump written by `tests/oracle/generate.py`, and a reference
//! implementation of pdfTeX's `line_break` + `post_line_break` + `hpack`
//! (pdftex.web, TeX Live trunk) built on the crate's public primitives.
//!
//! Scope of the reference implementation: no `\parshape`/`\hangindent`,
//! `\looseness` 0, `\lastlinefit` 0, no TeXXeT, no inserts/marks/adjusts,
//! autoexpand fonts only. Everything else follows the web code, including
//! TeX's active-list order and tie rules.
#![allow(dead_code)]

use std::collections::BTreeMap;

use flashtex_microtype::arith::{INF_BAD, Scaled, badness, parse_scaled};
use flashtex_microtype::pdftex::{ExpansionLimits, FontParams, ParagraphExpansion, expanded_width};
use flashtex_microtype::{adjust_shortfall, line_expand_ratio};

// ---------------------------------------------------------------------------
// Node lists as shown by \showbox

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Order {
    Normal = 0,
    Fil = 1,
    Fill = 2,
    Filll = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernKind {
    Normal,
    Explicit,
    Accent,
    LeftMargin,
    RightMargin,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlueSpec {
    pub name: Option<String>,
    pub width: Scaled,
    pub stretch: Scaled,
    pub stretch_order: Order,
    pub shrink: Scaled,
    pub shrink_order: Order,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GlueSign {
    Normal,
    Stretching,
    Shrinking,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Char { font: String, expansion: i32, code: u8, lig: bool },
    Glue(GlueSpec),
    Kern { width: Scaled, kind: KernKind },
    Penalty(i32),
    Disc { pre: Vec<Node>, post: Vec<Node>, replace: usize },
    Math { on: bool, width: Scaled },
    HBox { width: Scaled, height: Scaled, depth: Scaled, sign: GlueSign, set: f64, order: Order, children: Vec<Node> },
    VBox { width: Scaled, children: Vec<Node> },
    Rule { width: Scaled },
    Other(String),
}

impl Node {
    pub fn is_char(&self) -> bool {
        matches!(self, Node::Char { lig: false, .. })
    }
    pub fn is_char_or_lig(&self) -> bool {
        matches!(self, Node::Char { .. })
    }
    /// `non_discardable(p)`: `type(p) < math_node`.
    fn non_discardable(&self) -> bool {
        matches!(
            self,
            Node::HBox { .. } | Node::VBox { .. } | Node::Rule { .. } | Node::Disc { .. } | Node::Other(_)
        ) || matches!(self, Node::Char { lig: true, .. })
    }
    /// `precedes_break(p)` (same set as non_discardable for our node kinds).
    fn precedes_break(&self) -> bool {
        self.non_discardable()
    }
    /// `cp_skipable(p)`.
    fn cp_skipable(&self) -> bool {
        match self {
            Node::Char { .. } => false,
            Node::Penalty(_) => true,
            Node::Disc { pre, post, replace } => pre.is_empty() && post.is_empty() && *replace == 0,
            Node::Math { width, .. } => *width == 0,
            Node::Kern { width, kind } => *width == 0 || *kind == KernKind::Normal,
            Node::Glue(g) => is_zero_glue(g),
            Node::HBox { width, height, depth, children, .. } => {
                *width == 0 && *height == 0 && *depth == 0 && children.is_empty()
            }
            _ => false,
        }
    }
}

/// pdfTeX compares `glue_ptr(p) = zero_glue`; the parameter glues that are
/// zero share that spec, anonymous `\hskip0pt` glue does not.
fn is_zero_glue(g: &GlueSpec) -> bool {
    g.name.is_some() && g.width == 0 && g.stretch == 0 && g.shrink == 0
}

fn parse_order(s: &str) -> (Scaled, Order) {
    for (suffix, o) in [("filll", Order::Filll), ("fill", Order::Fill), ("fil", Order::Fil)] {
        if let Some(num) = s.strip_suffix(suffix) {
            return (parse_scaled(num).unwrap(), o);
        }
    }
    (parse_scaled(s.trim_end_matches("pt")).unwrap(), Order::Normal)
}

/// `0.0pt plus 1.0fil minus 2.0pt` or `3.33252 plus 1.66626 minus 1.11084`.
pub fn parse_glue(name: Option<String>, s: &str) -> GlueSpec {
    let toks: Vec<&str> = s.split_whitespace().collect();
    let (width, _) = parse_order(toks[0]);
    let mut g = GlueSpec { name, width, stretch: 0, stretch_order: Order::Normal, shrink: 0, shrink_order: Order::Normal };
    let mut i = 1;
    while i + 1 < toks.len() {
        let (v, o) = parse_order(toks[i + 1]);
        match toks[i] {
            "plus" => {
                g.stretch = v;
                g.stretch_order = o;
            }
            "minus" => {
                g.shrink = v;
                g.shrink_order = o;
            }
            other => panic!("bad glue token {other} in {s}"),
        }
        i += 2;
    }
    g
}

fn unescape_char(s: &str) -> u8 {
    if let Some(rest) = s.strip_prefix("^^") {
        if rest.len() == 2 && rest.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
            return u8::from_str_radix(rest, 16).unwrap();
        }
        let c = rest.as_bytes()[0];
        return if c == b'?' { 127 } else { c - 64 };
    }
    assert_eq!(s.len(), 1, "unexpected char display {s:?}");
    s.as_bytes()[0]
}

fn parse_node_line(line: &str) -> Node {
    if let Some(rest) = line.strip_prefix("\\glue") {
        let (name, spec) = if let Some(r) = rest.strip_prefix("(\\") {
            let end = r.find(')').unwrap();
            (Some(r[..end].to_string()), r[end + 1..].trim())
        } else {
            (None, rest.trim())
        };
        return Node::Glue(parse_glue(name, spec));
    }
    if let Some(rest) = line.strip_prefix("\\kern") {
        let explicit = rest.starts_with(' ');
        let rest = rest.trim();
        let (num, tail) = rest.split_once(' ').map(|(a, b)| (a, b.trim())).unwrap_or((rest, ""));
        let kind = match tail {
            "(left margin)" => KernKind::LeftMargin,
            "(right margin)" => KernKind::RightMargin,
            "(for accent)" => KernKind::Accent,
            "" if explicit => KernKind::Explicit,
            "" => KernKind::Normal,
            t => panic!("kern tail {t}"),
        };
        return Node::Kern { width: parse_scaled(num).unwrap(), kind };
    }
    if let Some(rest) = line.strip_prefix("\\penalty ") {
        return Node::Penalty(rest.trim().parse().unwrap());
    }
    if let Some(rest) = line.strip_prefix("\\discretionary") {
        let replace = rest.trim().strip_prefix("replacing ").map(|n| n.parse().unwrap()).unwrap_or(0);
        return Node::Disc { pre: vec![], post: vec![], replace };
    }
    if let Some(rest) = line.strip_prefix("\\mathon") {
        let w = rest.trim().strip_prefix(", surrounded ").map(|n| parse_scaled(n).unwrap()).unwrap_or(0);
        return Node::Math { on: true, width: w };
    }
    if let Some(rest) = line.strip_prefix("\\mathoff") {
        let w = rest.trim().strip_prefix(", surrounded ").map(|n| parse_scaled(n).unwrap()).unwrap_or(0);
        return Node::Math { on: false, width: w };
    }
    if let Some(rest) = line.strip_prefix("\\hbox(").or_else(|| line.strip_prefix("\\vbox(")) {
        let is_h = line.starts_with("\\hbox");
        let close = rest.find(')').unwrap();
        let (h, d) = rest[..close].split_once('+').unwrap();
        let after = &rest[close + 1..];
        let after = after.strip_prefix('x').unwrap();
        let mut parts = after.split(", ");
        let width = parse_scaled(parts.next().unwrap()).unwrap();
        let (mut sign, mut set, mut order) = (GlueSign::Normal, 0.0, Order::Normal);
        for p in parts {
            if let Some(g) = p.strip_prefix("glue set ") {
                let (neg, g) = match g.strip_prefix("- ") {
                    Some(r) => (true, r),
                    None => (false, g),
                };
                let (num, o) = [("filll", Order::Filll), ("fill", Order::Fill), ("fil", Order::Fil)]
                    .iter()
                    .find_map(|(s, o)| g.strip_suffix(s).map(|n| (n, *o)))
                    .unwrap_or((g, Order::Normal));
                set = num.parse().unwrap();
                order = o;
                sign = if neg { GlueSign::Shrinking } else { GlueSign::Stretching };
            }
        }
        return if is_h {
            Node::HBox {
                width,
                height: parse_scaled(h).unwrap(),
                depth: parse_scaled(d).unwrap(),
                sign,
                set,
                order,
                children: vec![],
            }
        } else {
            Node::VBox { width, children: vec![] }
        };
    }
    if let Some(rest) = line.strip_prefix("\\rule(") {
        let x = rest.rfind('x').unwrap();
        return Node::Rule { width: parse_scaled(&rest[x + 1..]).unwrap_or(0) };
    }
    // character: \<font> [(+e)] <char> [(ligature ...)]
    if line.starts_with('\\') && line.contains('/') {
        let (font, mut rest) = line[1..].split_once(' ').unwrap();
        let mut expansion = 0;
        if rest.len() > 1 && (rest.starts_with("(+") || rest.starts_with("(-")) {
            if let Some(end) = rest.find(") ") {
                expansion = rest[1..end].parse().unwrap();
                rest = &rest[end + 2..];
            }
        }
        let (ch, lig) = match rest.find(" (ligature ") {
            Some(p) if p > 0 => (&rest[..p], true),
            _ => (rest, false),
        };
        return Node::Char { font: font.to_string(), expansion, code: unescape_char(ch), lig };
    }
    Node::Other(line.to_string())
}

/// Parse the node list whose lines carry `prefix`.
pub fn parse_list(lines: &[&str], idx: &mut usize, prefix: &str) -> Vec<Node> {
    let mut out = Vec::new();
    while *idx < lines.len() {
        let line = lines[*idx];
        let Some(rest) = line.strip_prefix(prefix) else { break };
        if !rest.starts_with('\\') {
            break;
        }
        *idx += 1;
        let mut node = parse_node_line(rest);
        match &mut node {
            Node::HBox { children, .. } | Node::VBox { children, .. } => {
                *children = parse_list(lines, idx, &format!("{prefix}."));
            }
            Node::Disc { pre, post, .. } => {
                *pre = parse_list(lines, idx, &format!("{prefix}."));
                *post = parse_list(lines, idx, &format!("{prefix}|"));
            }
            _ => {}
        }
        out.push(node);
    }
    out
}

// ---------------------------------------------------------------------------
// Fixture files

#[derive(Debug, Clone)]
pub struct FontEntry {
    pub params: FontParams,
    pub widths: [Scaled; 256],
    pub size: String,
}

#[derive(Debug, Clone)]
pub struct Fixture {
    pub name: String,
    pub params: BTreeMap<String, String>,
    pub list1: Vec<Node>,
    pub list2: Vec<Node>,
    pub result: Vec<Node>,
    pub fonts: BTreeMap<String, FontEntry>,
    /// Raw dumped `\lpcode`/`\rpcode`/`\efcode` straight from pdfTeX.
    pub probes: Vec<Node>,
}

impl Fixture {
    pub fn int(&self, k: &str) -> i32 {
        self.params[k].parse().unwrap_or_else(|_| panic!("param {k}"))
    }
    pub fn glue(&self, k: &str) -> GlueSpec {
        parse_glue(Some(k.to_string()), &self.params[k])
    }
}

pub fn load_fixture(name: &str, text: &str) -> Fixture {
    let lines: Vec<&str> = text.lines().collect();
    let mut params = BTreeMap::new();
    let mut list1 = vec![];
    let mut list2 = vec![];
    let mut result = vec![];
    let mut probes = vec![];
    let mut fonts: BTreeMap<String, FontEntry> = BTreeMap::new();
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        if let Some(rest) = l.strip_prefix("@@params ") {
            let mut key = String::new();
            for tok in rest.split_whitespace() {
                if let Some((k, v)) = tok.split_once('=') {
                    key = k.to_string();
                    params.insert(key.clone(), v.to_string());
                } else {
                    let e = params.get_mut(&key).unwrap();
                    e.push(' ');
                    e.push_str(tok);
                }
            }
            i += 1;
        } else if l == "@@list1" {
            i += 1;
            list1 = parse_list(&lines, &mut i, "");
        } else if l == "@@list2" {
            i += 1;
            let boxes = parse_list(&lines, &mut i, "");
            let Node::VBox { children, .. } = &boxes[0] else { panic!() };
            let Node::HBox { children, .. } = &children[0] else { panic!() };
            list2 = children.clone();
        } else if l == "@@result" {
            i += 1;
            let boxes = parse_list(&lines, &mut i, "");
            let Node::VBox { children, .. } = &boxes[0] else { panic!() };
            result = children.clone();
        } else if l == "@@probe" {
            i += 1;
            probes.extend(parse_list(&lines, &mut i, ""));
        } else if let Some(rest) = l.strip_prefix("@@font \\") {
            let mut it = rest.split_whitespace();
            let fname = it.next().unwrap().to_string();
            let quad: Scaled = it.next().unwrap().strip_prefix("quad=").unwrap().parse().unwrap();
            let size = it.next().unwrap().strip_prefix("size=").unwrap().to_string();
            let mut e = FontEntry { params: FontParams::plain(quad), widths: [0; 256], size };
            i += 1;
            while i < lines.len() && !lines[i].starts_with("@@") {
                let v: Vec<i32> = lines[i].split_whitespace().map(|x| x.parse().unwrap()).collect();
                let c = v[0] as usize;
                e.widths[c] = v[1];
                e.params.lpcode[c] = v[2] as i16;
                e.params.rpcode[c] = v[3] as i16;
                e.params.efcode[c] = v[4] as i16;
                i += 1;
            }
            fonts.insert(fname, e);
        } else {
            i += 1;
        }
    }
    // expansion limits from the probes: one-glyph lines stretched/shrunk to the limit
    for b in &probes {
        let Node::VBox { children, .. } = b else { continue };
        for line in children {
            let Node::HBox { children, .. } = line else { continue };
            for n in children {
                if let Node::Char { font, expansion, .. } = n {
                    if *expansion != 0 {
                        let e = fonts.get_mut(font).unwrap();
                        let mut l = e.params.expansion.unwrap_or(ExpansionLimits { stretch: 0, shrink: 0, step: 1 });
                        if *expansion > 0 {
                            l.stretch = *expansion;
                        } else {
                            l.shrink = -*expansion;
                        }
                        e.params.expansion = Some(l);
                    }
                }
            }
        }
    }
    Fixture { name: name.to_string(), params, list1, list2, result, fonts, probes }
}

// ---------------------------------------------------------------------------
// Reference line breaker

const AWFUL_BAD: i64 = 0o7777777777;
const EJECT: i32 = -10_000;

/// Sensitivity check: `MT_MUTATE=<name> cargo test --test oracle` disables one
/// pdfTeX rule so the oracle can be seen to fail without it. Names:
/// `no-total-pw`, `no-shortfall-rule`, `no-kern-stretch`, `no-margin-kerns`,
/// `half-ratio`.
pub fn mutated(name: &str) -> bool {
    std::env::var("MT_MUTATE").is_ok_and(|v| v == name)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct W([i64; 8]); // index 0..8 = TeX's active_width[1..8]

impl std::ops::Add for W {
    type Output = W;
    fn add(mut self, o: W) -> W {
        for i in 0..8 {
            self.0[i] += o.0[i];
        }
        self
    }
}
impl std::ops::Sub for W {
    type Output = W;
    fn sub(mut self, o: W) -> W {
        for i in 0..8 {
            self.0[i] -= o.0[i];
        }
        self
    }
}

pub struct Env<'a> {
    pub fonts: &'a BTreeMap<String, FontEntry>,
    pub hsize: Scaled,
    pub pretolerance: i32,
    pub tolerance: i32,
    pub emergency_stretch: Scaled,
    pub line_penalty: i32,
    pub hyphen_penalty: i32,
    pub ex_hyphen_penalty: i32,
    pub adj_demerits: i32,
    pub double_hyphen_demerits: i32,
    pub final_hyphen_demerits: i32,
    pub left_skip: GlueSpec,
    pub right_skip: GlueSpec,
    pub par_fill_skip: GlueSpec,
    pub adjust_spacing: i32,
    pub protrude_chars: i32,
}

impl<'a> Env<'a> {
    pub fn from_fixture(f: &'a Fixture) -> Env<'a> {
        assert_eq!(f.int("looseness"), 0);
        assert_eq!(f.int("lastlinefit"), 0);
        Env {
            fonts: &f.fonts,
            hsize: f.int("hsize"),
            pretolerance: f.int("pretolerance"),
            tolerance: f.int("tolerance"),
            emergency_stretch: f.int("emergencystretch"),
            line_penalty: f.int("linepenalty"),
            hyphen_penalty: f.int("hyphenpenalty"),
            ex_hyphen_penalty: f.int("exhyphenpenalty"),
            adj_demerits: f.int("adjdemerits"),
            double_hyphen_demerits: f.int("doublehyphendemerits"),
            final_hyphen_demerits: f.int("finalhyphendemerits"),
            left_skip: f.glue("leftskip"),
            right_skip: f.glue("rightskip"),
            par_fill_skip: f.glue("parfillskip"),
            adjust_spacing: f.int("pdfadjustspacing"),
            protrude_chars: f.int("pdfprotrudechars"),
        }
    }

    fn font(&self, name: &str) -> &FontEntry {
        self.fonts.get(name).unwrap_or_else(|| panic!("font {name} not dumped"))
    }

    /// Width of a character node in its (possibly expanded) font.
    pub fn char_width(&self, n: &Node) -> Scaled {
        let Node::Char { font, expansion, code, .. } = n else { unreachable!() };
        expanded_width(self.font(font).widths[*code as usize], *expansion)
    }

    fn left_pw(&self, n: Option<&Node>) -> Scaled {
        if mutated("no-margin-kerns") {
            return 0;
        }
        match n {
            Some(Node::Char { font, code, .. }) => self.font(font).params.left_protrusion(*code),
            _ => 0,
        }
    }
    fn right_pw(&self, n: Option<&Node>) -> Scaled {
        match n {
            Some(Node::Char { font, code, .. }) => self.font(font).params.right_protrusion(*code),
            _ => 0,
        }
    }

    /// `kern_stretch`/`kern_shrink` of the normal kern at `list[i]`.
    fn kern_var(&self, list: &[Node], i: usize) -> (Scaled, Scaled) {
        let Node::Kern { width, kind: KernKind::Normal } = &list[i] else { return (0, 0) };
        if i == 0 || i + 1 >= list.len() || mutated("no-kern-stretch") {
            return (0, 0);
        }
        match (&list[i - 1], &list[i + 1]) {
            (Node::Char { font: fl, code: cl, .. }, Node::Char { font: fr, .. }) if fl == fr => {
                let p = &self.font(fl).params;
                if !p.is_expandable() {
                    return (0, 0);
                }
                (p.kern_stretch(*cl, *width), p.kern_shrink(*cl, *width))
            }
            _ => (0, 0),
        }
    }

    /// The width vector a node contributes in `line_break` (slots 7/8 only
    /// when `\pdfadjustspacing > 1`).
    fn contribution(&self, list: &[Node], i: usize, par: &mut ParagraphExpansion) -> W {
        let mut w = W::default();
        match &list[i] {
            n @ Node::Char { font, code, .. } => {
                w.0[0] = self.char_width(n) as i64;
                let e = self.font(font);
                if self.adjust_spacing > 1 && par.note_font(&e.params).unwrap() {
                    let base = e.widths[*code as usize];
                    w.0[6] = e.params.char_stretch(*code, base) as i64;
                    w.0[7] = e.params.char_shrink(*code, base) as i64;
                }
            }
            Node::Kern { width, kind } => {
                w.0[0] = *width as i64;
                if self.adjust_spacing > 1 && *kind == KernKind::Normal {
                    let (s, k) = self.kern_var(list, i);
                    w.0[6] = s as i64;
                    w.0[7] = k as i64;
                }
            }
            Node::Glue(g) => {
                w.0[0] = g.width as i64;
                w.0[1 + g.stretch_order as usize] = g.stretch as i64;
                w.0[5] = g.shrink as i64;
            }
            Node::Math { width, .. } => w.0[0] = *width as i64,
            Node::HBox { width, .. } | Node::VBox { width, .. } | Node::Rule { width } => w.0[0] = *width as i64,
            _ => {}
        }
        w
    }

    fn list_width(&self, list: &[Node], par: &mut ParagraphExpansion) -> W {
        (0..list.len()).fold(W::default(), |acc, i| acc + self.contribution(list, i, par))
    }
}

#[derive(Clone, Debug)]
struct Active {
    fitness: usize,
    hyphenated: bool,
    line: i32,
    total: i64,
    passive: Option<usize>,
    alpha: W,
}

#[derive(Clone, Debug)]
struct Passive {
    cur_break: Option<usize>,
    prev: Option<usize>,
}

pub struct Breaks {
    /// Break node indices into `list` in order; `None` = end of paragraph.
    pub breaks: Vec<Option<usize>>,
    pub list: Vec<Node>,
    pub pass: u8,
}

/// `find_protchar_left(l, d)` restricted to one list level plus hbox descent.
fn find_protchar_left<'n>(list: &'n [Node], start: usize, d: bool) -> Option<&'n Node> {
    let mut stack: Vec<(&'n [Node], usize)> = vec![];
    let mut cur: &'n [Node] = list;
    let mut l = start;
    if l >= cur.len() {
        return None;
    }
    let has_link = |cur: &[Node], l: usize| l + 1 < cur.len();
    if has_link(cur, l) && matches!(&cur[l], Node::HBox { width: 0, height: 0, depth: 0, children, .. } if children.is_empty()) {
        l += 1;
    } else if d {
        while has_link(cur, l) && !(cur[l].is_char_or_lig() || cur[l].non_discardable()) {
            l += 1;
        }
    }
    let mut run = true;
    loop {
        let t = (cur.as_ptr(), l);
        while run {
            if let Node::HBox { children, .. } = &cur[l] {
                if !children.is_empty() {
                    stack.push((cur, l));
                    cur = children;
                    l = 0;
                    continue;
                }
            }
            break;
        }
        while run && cur[l].cp_skipable() {
            while !has_link(cur, l) && !stack.is_empty() {
                let (c, i) = stack.pop().unwrap();
                cur = c;
                l = i;
            }
            if has_link(cur, l) {
                l += 1;
            } else if stack.is_empty() {
                run = false;
            }
        }
        if t == (cur.as_ptr(), l) {
            break;
        }
    }
    Some(&cur[l])
}

/// `find_protchar_right(l, r)` over `list[l..=r]` plus hbox descent.
fn find_protchar_right<'n>(list: &'n [Node], l0: usize, r0: usize) -> Option<&'n Node> {
    let mut stack: Vec<(&'n [Node], usize, usize)> = vec![];
    let mut cur: &'n [Node] = list;
    let (mut l, mut r) = (l0, r0);
    let mut run = true;
    loop {
        let t = (cur.as_ptr(), r);
        while run {
            if let Node::HBox { children, .. } = &cur[r] {
                if !children.is_empty() {
                    stack.push((cur, l, r));
                    cur = children;
                    l = 0;
                    r = children.len() - 1;
                    continue;
                }
            }
            break;
        }
        while run && cur[r].cp_skipable() {
            while r == l && !stack.is_empty() {
                let (c, pl, pr) = stack.pop().unwrap();
                cur = c;
                l = pl;
                r = pr;
            }
            if r != l {
                r -= 1;
            } else if stack.is_empty() {
                run = false;
            }
        }
        if t == (cur.as_ptr(), r) {
            break;
        }
    }
    Some(&cur[r])
}

/// Prepare a pass list: TeX's `line_break` start (trailing glue becomes
/// `\penalty10000`, then `\parfillskip`).
pub fn finish_list1(mut list: Vec<Node>, env: &Env) -> Vec<Node> {
    if matches!(list.last(), Some(Node::Glue(_))) {
        list.pop();
    }
    list.push(Node::Penalty(10_000));
    list.push(Node::Glue(env.par_fill_skip.clone()));
    list
}

/// The one-line pass-2 dump without what `post_line_break` added.
pub fn strip_list2(list: Vec<Node>) -> Vec<Node> {
    let mut out: Vec<Node> = list
        .into_iter()
        .filter(|n| !matches!(n, Node::Kern { kind: KernKind::LeftMargin | KernKind::RightMargin, .. }))
        .collect();
    if matches!(out.last(), Some(Node::Glue(g)) if g.name.as_deref() == Some("rightskip")) {
        out.pop();
    }
    if matches!(out.first(), Some(Node::Glue(g)) if g.name.as_deref() == Some("leftskip")) {
        out.remove(0);
    }
    out
}

pub fn line_break(env: &Env, list1: &[Node], list2: &[Node]) -> Breaks {
    let mut threshold;
    let mut second_pass;
    let mut final_pass;
    let mut background = W::default();
    background.0[0] = (env.left_skip.width + env.right_skip.width) as i64;
    background.0[1 + env.left_skip.stretch_order as usize] += env.left_skip.stretch as i64;
    background.0[1 + env.right_skip.stretch_order as usize] += env.right_skip.stretch as i64;
    background.0[5] = (env.left_skip.shrink + env.right_skip.shrink) as i64;
    if env.pretolerance >= 0 {
        threshold = env.pretolerance;
        second_pass = false;
        final_pass = false;
    } else {
        threshold = env.tolerance;
        second_pass = true;
        final_pass = env.emergency_stretch <= 0;
    }
    let mut pass = 1;
    loop {
        if threshold > INF_BAD {
            threshold = INF_BAD;
        }
        let list = if second_pass { list2 } else { list1 };
        if let Some(b) = try_pass(env, list, background, threshold, final_pass) {
            return Breaks { breaks: b, list: list.to_vec(), pass };
        }
        pass += 1;
        if !second_pass {
            threshold = env.tolerance;
            second_pass = true;
            final_pass = env.emergency_stretch <= 0;
        } else {
            background.0[1] += env.emergency_stretch as i64;
            final_pass = true;
        }
    }
}

fn try_pass(env: &Env, list: &[Node], background: W, threshold: i32, final_pass: bool) -> Option<Vec<Option<usize>>> {
    let n = list.len();
    let mut par = ParagraphExpansion::default();
    // prefix sums W[i] = widths of list[0..i]
    let mut pre = vec![W::default(); n + 1];
    for i in 0..n {
        pre[i + 1] = pre[i] + env.contribution(list, i, &mut par);
    }
    let disc_width = |i: usize, par: &mut ParagraphExpansion| -> (W, W) {
        let Node::Disc { pre: p, post: q, .. } = &list[i] else { unreachable!() };
        (env.list_width(p, par), env.list_width(q, par))
    };
    let mut active = vec![Active { fitness: 2, hyphenated: false, line: 1, total: 0, passive: None, alpha: W::default() }];
    let mut passive: Vec<Passive> = vec![];

    // try_break
    let try_break = |cur: Option<usize>,
                         mut pi: i32,
                         hyphenated: bool,
                         active: &mut Vec<Active>,
                         passive: &mut Vec<Passive>,
                         par: &mut ParagraphExpansion| {
        if pi.abs() >= 10_000 {
            if pi > 0 {
                return;
            }
            pi = EJECT;
        }
        let beta = match cur {
            Some(c) => {
                let mut b = pre[c] + background;
                if hyphenated && matches!(list[c], Node::Disc { .. }) {
                    b = b + disc_width(c, par).0;
                }
                b
            }
            None => pre[n] + background,
        };
        let mut minimal = [AWFUL_BAD; 4];
        let mut best_place = [None; 4];
        let mut best_line = [0i32; 4];
        let mut minimum = AWFUL_BAD;
        let line_width = env.hsize as i64;
        let mut j = 0;
        while j < active.len() {
            let r = active[j].clone();
            let caw = beta - r.alpha;
            let mut shortfall = line_width - caw.0[0];
            if env.protrude_chars > 1 && !mutated("no-total-pw") {
                shortfall += total_pw(env, list, &r, passive, cur) as i64;
            }
            if env.adjust_spacing > 1 && shortfall != 0 && !mutated("no-shortfall-rule") {
                // margin kern variations are zero for autoexpand fonts
                shortfall = adjust_shortfall(shortfall as Scaled, caw.0[6] as Scaled, caw.0[7] as Scaled, 0, 0, *par) as i64;
            }
            let (b, fit): (i32, usize);
            if shortfall > 0 {
                if caw.0[2] != 0 || caw.0[3] != 0 || caw.0[4] != 0 {
                    b = 0;
                    fit = 2;
                } else if shortfall > 7_230_584 && caw.0[1] < 1_663_497 {
                    b = INF_BAD;
                    fit = 0;
                } else {
                    b = badness(shortfall as Scaled, caw.0[1] as Scaled);
                    fit = if b > 99 { 0 } else if b > 12 { 1 } else { 2 };
                }
            } else {
                if -shortfall > caw.0[5] {
                    b = INF_BAD + 1;
                } else {
                    b = badness((-shortfall) as Scaled, caw.0[5] as Scaled);
                }
                fit = if b > 12 { 3 } else { 2 };
            }
            let mut artificial = false;
            let stays;
            if b > INF_BAD || pi == EJECT {
                if final_pass && minimum == AWFUL_BAD && active.len() == 1 {
                    artificial = true;
                } else if b > threshold {
                    active.remove(j);
                    continue;
                }
                stays = false;
            } else {
                if b > threshold {
                    j += 1;
                    continue;
                }
                stays = true;
            }
            // record a feasible break
            let mut d: i64;
            if artificial {
                d = 0;
            } else {
                d = (env.line_penalty + b) as i64;
                d = if d.abs() >= 10_000 { 100_000_000 } else { d * d };
                if pi != 0 {
                    if pi > 0 {
                        d += (pi as i64) * (pi as i64);
                    } else if pi > EJECT {
                        d -= (pi as i64) * (pi as i64);
                    }
                }
                if hyphenated && r.hyphenated {
                    d += if cur.is_some() { env.double_hyphen_demerits } else { env.final_hyphen_demerits } as i64;
                }
                if (fit as i32 - r.fitness as i32).abs() > 1 {
                    d += env.adj_demerits as i64;
                }
            }
            d += r.total;
            if d <= minimal[fit] {
                minimal[fit] = d;
                best_place[fit] = r.passive;
                best_line[fit] = r.line;
                if d < minimum {
                    minimum = d;
                }
            }
            if stays {
                j += 1;
            } else {
                active.remove(j);
            }
        }
        if minimum < AWFUL_BAD {
            // break_width → alpha of the new nodes
            let alpha = match cur {
                None => pre[n],
                Some(c) => {
                    let mut post = W::default();
                    let mut s: Option<usize> = Some(c);
                    let is_disc = hyphenated && matches!(list[c], Node::Disc { .. });
                    if is_disc {
                        let Node::Disc { post: pb, replace, .. } = &list[c] else { unreachable!() };
                        post = disc_width(c, par).1;
                        let v = c + replace;
                        s = if pb.is_empty() { Some(v + 1) } else { None };
                    }
                    let mut start = match s {
                        Some(x) => x,
                        None => c + 1 + if let Node::Disc { replace, .. } = &list[c] { *replace } else { 0 },
                    };
                    if s.is_some() {
                        while start < n {
                            match &list[start] {
                                Node::Glue(_) | Node::Penalty(_) | Node::Math { .. } => start += 1,
                                Node::Kern { kind: KernKind::Explicit, .. } => start += 1,
                                _ => break,
                            }
                        }
                    }
                    pre[start] - post
                }
            };
            let adj = env.adj_demerits.abs() as i64;
            minimum = if adj >= AWFUL_BAD - minimum { AWFUL_BAD - 1 } else { minimum + adj };
            for fit in 0..4 {
                if minimal[fit] <= minimum {
                    passive.push(Passive { cur_break: cur, prev: best_place[fit] });
                    active.push(Active {
                        fitness: fit,
                        hyphenated,
                        line: best_line[fit] + 1,
                        total: minimal[fit],
                        passive: Some(passive.len() - 1),
                        alpha,
                    });
                }
            }
        }
    };

    let mut i = 0;
    let mut prev: Option<usize> = None; // prev_p starts at the first node: glue there is illegal
    let mut auto_breaking = true;
    while i < n && !active.is_empty() {
        let legal_after_prev = |p: Option<usize>| match p {
            None => false,
            Some(p) => {
                list[p].is_char_or_lig() || list[p].precedes_break() || matches!(list[p], Node::Kern { kind, .. } if kind != KernKind::Explicit)
            }
        };
        match &list[i] {
            Node::Glue(_) => {
                if auto_breaking && legal_after_prev(prev) {
                    try_break(Some(i), 0, false, &mut active, &mut passive, &mut par);
                }
            }
            Node::Kern { kind: KernKind::Explicit, .. } => {
                if auto_breaking && i + 1 < n && matches!(list[i + 1], Node::Glue(_)) {
                    try_break(Some(i), 0, false, &mut active, &mut passive, &mut par);
                }
            }
            Node::Math { on, .. } => {
                auto_breaking = !*on;
                if auto_breaking && i + 1 < n && matches!(list[i + 1], Node::Glue(_)) {
                    try_break(Some(i), 0, false, &mut active, &mut passive, &mut par);
                }
            }
            Node::Penalty(p) => try_break(Some(i), *p, false, &mut active, &mut passive, &mut par),
            Node::Disc { pre: p, replace, .. } => {
                let pen = if p.is_empty() { env.ex_hyphen_penalty } else { env.hyphen_penalty };
                try_break(Some(i), pen, true, &mut active, &mut passive, &mut par);
                prev = Some(i);
                i += 1 + replace;
                continue;
            }
            _ => {}
        }
        prev = Some(i);
        i += 1;
    }
    if i >= n && !active.is_empty() {
        try_break(None, EJECT, true, &mut active, &mut passive, &mut par);
        if !active.is_empty() {
            let mut best = 0;
            for (k, a) in active.iter().enumerate() {
                if a.total < active[best].total {
                    best = k;
                }
            }
            let mut out = vec![];
            let mut p = active[best].passive;
            while let Some(k) = p {
                out.push(passive[k].cur_break);
                p = passive[k].prev;
            }
            out.reverse();
            return Some(out);
        }
    }
    None
}

/// `total_pw(r, cur_p)`.
fn total_pw(env: &Env, list: &[Node], r: &Active, passive: &[Passive], cur: Option<usize>) -> Scaled {
    let n = list.len();
    let l = match r.passive {
        None => Some(0),
        Some(k) => passive[k].cur_break,
    };
    let l = l.unwrap_or(0);
    // right side
    let right = match cur {
        Some(c) if matches!(&list[c], Node::Disc { pre, .. } if !pre.is_empty()) => {
            let Node::Disc { pre, .. } = &list[c] else { unreachable!() };
            pre.last()
        }
        _ => {
            let rr = match cur {
                Some(c) => c.checked_sub(1),
                None => Some(n - 1),
            };
            rr.and_then(|rr| if rr >= l { find_protchar_right(list, l, rr) } else { None })
        }
    };
    // left side
    let left = if r.passive.is_some() && matches!(list[l], Node::Disc { .. }) {
        let Node::Disc { post, replace, .. } = &list[l] else { unreachable!() };
        if !post.is_empty() {
            post.first()
        } else {
            let mut ll = l + 1;
            for _ in 0..*replace {
                if ll + 1 < n {
                    ll += 1;
                }
            }
            find_protchar_left(list, ll.min(n - 1), true)
        }
    } else {
        find_protchar_left(list, l, true)
    };
    env.left_pw(left) + env.right_pw(right)
}

// ---------------------------------------------------------------------------
// post_line_break + hpack

#[derive(Debug, Clone)]
pub struct PackedLine {
    pub nodes: Vec<Node>,
    pub sign: GlueSign,
    pub order: Order,
    pub set: f64,
    pub ratio: i32,
}

pub fn post_line_break(env: &Env, br: &Breaks) -> Vec<PackedLine> {
    let mut rest: Vec<(Option<usize>, Node)> = br.list.iter().cloned().enumerate().map(|(i, n)| (Some(i), n)).collect();
    let mut lines = vec![];
    for (k, b) in br.breaks.iter().enumerate() {
        let mut disc_break = false;
        let mut glue_break = false;
        let mut post_disc_break = false;
        let mut q: usize;
        match b {
            None => q = rest.len() - 1,
            Some(id) => {
                q = rest.iter().position(|(i, _)| *i == Some(*id)).unwrap();
                match &mut rest[q].1 {
                    Node::Glue(g) => {
                        *g = GlueSpec { name: Some("rightskip".into()), ..env.right_skip.clone() };
                        glue_break = true;
                    }
                    Node::Disc { pre, post, replace } => {
                        let t = *replace;
                        let pre_nodes = std::mem::take(pre);
                        let post_nodes = std::mem::take(post);
                        *replace = 0;
                        rest.drain(q + 1..q + 1 + t);
                        if !post_nodes.is_empty() {
                            post_disc_break = true;
                        }
                        let npre = pre_nodes.len();
                        let ins: Vec<(Option<usize>, Node)> = pre_nodes
                            .into_iter()
                            .map(|n| (None, n))
                            .chain(post_nodes.into_iter().map(|n| (None, n)))
                            .collect();
                        rest.splice(q + 1..q + 1, ins);
                        q += npre;
                        disc_break = true;
                    }
                    Node::Kern { width, .. } => *width = 0,
                    Node::Math { width, .. } => *width = 0,
                    _ => {}
                }
            }
        }
        let mut line_end = q;
        if env.protrude_chars > 0 {
            let is_pre_char = disc_break && !matches!(rest[q].1, Node::Disc { .. });
            let (p, ptmp) = if is_pre_char {
                (Some(q), q)
            } else {
                let ptmp = q - 1;
                let slice: Vec<Node> = rest[..=ptmp].iter().map(|(_, n)| n.clone()).collect();
                let found = find_protchar_right(&slice, 0, ptmp).cloned();
                let w = env.right_pw(found.as_ref());
                if w != 0 {
                    rest.insert(ptmp + 1, (None, Node::Kern { width: -w, kind: KernKind::RightMargin }));
                    line_end += 1;
                }
                (None, ptmp)
            };
            if let Some(p) = p {
                let w = env.right_pw(Some(&rest[p].1));
                if w != 0 {
                    rest.insert(ptmp + 1, (None, Node::Kern { width: -w, kind: KernKind::RightMargin }));
                    if ptmp == q {
                        line_end += 1;
                    }
                }
            }
        }
        if !glue_break {
            rest.insert(line_end + 1, (None, Node::Glue(GlueSpec { name: Some("rightskip".into()), ..env.right_skip.clone() })));
            line_end += 1;
        }
        let mut line: Vec<Node> = rest.drain(..=line_end).map(|(_, n)| n).collect();
        if env.protrude_chars > 0 {
            let w = env.left_pw(find_protchar_left(&line, 0, false));
            if w != 0 {
                line.insert(0, Node::Kern { width: -w, kind: KernKind::LeftMargin });
            }
        }
        if !is_zero_left(&env.left_skip) {
            line.insert(0, Node::Glue(GlueSpec { name: Some("leftskip".into()), ..env.left_skip.clone() }));
        }
        lines.push(hpack(env, line, env.hsize));
        // prune
        if k + 1 < br.breaks.len() && !post_disc_break {
            let next = br.breaks[k + 1];
            while let Some((id, node)) = rest.first() {
                if next.is_some() && *id == next {
                    break;
                }
                let discard = matches!(node, Node::Glue(_) | Node::Penalty(_) | Node::Math { .. } | Node::Kern { kind: KernKind::Explicit, .. });
                if !discard {
                    break;
                }
                rest.remove(0);
            }
        }
    }
    lines
}

fn is_zero_left(g: &GlueSpec) -> bool {
    g.width == 0 && g.stretch == 0 && g.shrink == 0
}

fn natural(env: &Env, line: &[Node]) -> (i64, [i64; 4], [i64; 4]) {
    let mut x = 0i64;
    let mut st = [0i64; 4];
    let mut sh = [0i64; 4];
    for n in line {
        match n {
            Node::Char { .. } => x += env.char_width(n) as i64,
            Node::Glue(g) => {
                x += g.width as i64;
                st[g.stretch_order as usize] += g.stretch as i64;
                sh[g.shrink_order as usize] += g.shrink as i64;
            }
            Node::Kern { width, .. } | Node::Math { width, .. } => x += *width as i64,
            Node::HBox { width, .. } | Node::VBox { width, .. } | Node::Rule { width } => x += *width as i64,
            _ => {}
        }
    }
    (x, st, sh)
}

fn top_order(t: &[i64; 4]) -> Order {
    if t[3] != 0 {
        Order::Filll
    } else if t[2] != 0 {
        Order::Fill
    } else if t[1] != 0 {
        Order::Fil
    } else {
        Order::Normal
    }
}

pub fn hpack(env: &Env, mut line: Vec<Node>, w: Scaled) -> PackedLine {
    let mut ratio = 0;
    if env.adjust_spacing > 0 {
        let (x, st, sh) = natural(env, &line);
        let (mut fs, mut fk) = (0i64, 0i64);
        for i in 0..line.len() {
            match &line[i] {
                Node::Char { font, code, .. } => {
                    let e = env.font(font);
                    let base = e.widths[*code as usize];
                    fs += e.params.char_stretch(*code, base) as i64;
                    fk += e.params.char_shrink(*code, base) as i64;
                }
                Node::Kern { kind: KernKind::Normal, .. } => {
                    let (s, k) = env.kern_var(&line, i);
                    fs += s as i64;
                    fk += k as i64;
                }
                _ => {}
            }
        }
        let excess = w as i64 - x;
        let finite = if excess > 0 { top_order(&st) == Order::Normal } else { top_order(&sh) == Order::Normal };
        ratio = line_expand_ratio(excess as Scaled, fs as Scaled, fk as Scaled, finite);
        if mutated("half-ratio") {
            ratio /= 2;
        }
        if ratio != 0 {
            let subst = |n: &mut Node| {
                if let Node::Char { font, code, expansion, .. } = n {
                    *expansion = env.font(font).params.char_expansion(*code, ratio);
                }
            };
            for n in line.iter_mut() {
                match n {
                    Node::Disc { pre, post, .. } => {
                        pre.iter_mut().for_each(subst);
                        post.iter_mut().for_each(subst);
                    }
                    _ => subst(n),
                }
            }
        }
    }
    let (x, st, sh) = natural(env, &line);
    let excess = w as i64 - x;
    let (mut sign, mut order, mut set) = (GlueSign::Normal, Order::Normal, 0.0);
    if excess > 0 {
        order = top_order(&st);
        if st[order as usize] != 0 {
            sign = GlueSign::Stretching;
            set = excess as f64 / st[order as usize] as f64;
        } else {
            order = Order::Normal;
        }
    } else if excess < 0 {
        order = top_order(&sh);
        if sh[order as usize] != 0 {
            sign = GlueSign::Shrinking;
            set = (-excess) as f64 / sh[order as usize] as f64;
        } else {
            order = Order::Normal;
        }
        if sh[order as usize] < -excess && order == Order::Normal && !line.is_empty() {
            set = 1.0;
        }
    }
    PackedLine { nodes: line, sign, order, set, ratio }
}

/// Glyph x positions (sp from the box's left edge) as `hlist_out` computes them.
pub fn glyph_positions(env: &Env, nodes: &[Node], sign: GlueSign, order: Order, set: f64) -> Vec<(u8, f64)> {
    let mut out = vec![];
    let mut cur_h = 0f64;
    let mut cur_g = 0f64;
    let mut cur_glue = 0f64;
    for n in nodes {
        match n {
            Node::Char { code, .. } => {
                out.push((*code, cur_h));
                cur_h += env.char_width(n) as f64;
            }
            Node::Glue(g) => {
                let mut rule_wd = g.width as f64 - cur_g;
                match sign {
                    GlueSign::Stretching if g.stretch_order == order => {
                        cur_glue += g.stretch as f64;
                        cur_g = (set * cur_glue).round();
                    }
                    GlueSign::Shrinking if g.shrink_order == order => {
                        cur_glue -= g.shrink as f64;
                        cur_g = (set * cur_glue).round();
                    }
                    _ => {}
                }
                rule_wd += cur_g;
                cur_h += rule_wd;
            }
            Node::Kern { width, .. } | Node::Math { width, .. } => cur_h += *width as f64,
            Node::HBox { width, .. } | Node::VBox { width, .. } | Node::Rule { width } => cur_h += *width as f64,
            _ => {}
        }
    }
    out
}
