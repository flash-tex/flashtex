//! The TikZ statement interpreter. Geometry is kept in TeX points on PGF's
//! y-up canvas until [`Interp::finish`] converts it to picture space.

use std::collections::HashMap;

use super::expr::{self, BP_PER_PT, PT_PER_CM, Value};
use super::text::{self as tx, matching, split_top, strip_braces};
use super::xcolor::Palette;
use super::{Diagnostic, Picture, PictureText, Severity, TextMeasurer, TextStyle, Tikz};
use crate::clip::Clip;
use crate::color::{Color, Paint};
use crate::geom::{Point, Transform};
use crate::item::{Group, Item, ItemId, PathFill, PathStroke, Pattern};
use crate::path::{Dash, FillRule, LineCap, LineJoin, Path, StrokeStyle};

type V = Point;

fn v(x: f64, y: f64) -> V {
    Point::new(x, y)
}
fn add(a: V, b: V) -> V {
    v(a.x + b.x, a.y + b.y)
}
fn sub(a: V, b: V) -> V {
    v(a.x - b.x, a.y - b.y)
}
fn mul(a: V, s: f64) -> V {
    v(a.x * s, a.y * s)
}
fn len(a: V) -> f64 {
    a.x.hypot(a.y)
}
fn unit(a: V) -> Option<V> {
    let l = len(a);
    if l < 1e-9 { None } else { Some(mul(a, 1.0 / l)) }
}
fn rad(deg: f64) -> f64 {
    deg.to_radians()
}
fn linear(t: &Transform) -> Transform {
    Transform::new(t.a, t.b, t.c, t.d, 0.0, 0.0)
}

/// Scoped definitions saved around a group: styles, colours, macros.
type Defs = (HashMap<String, (String, Option<String>)>, Palette, HashMap<String, String>);

/// Maximum nesting of styles, scopes and `\foreach` bodies.
const MAX_DEPTH: usize = 48;
/// Maximum `\foreach` iterations per loop.
const MAX_ITERATIONS: usize = 10_000;
/// `to` paths: control distance factor at looseness 1 (PGF's value).
const TO_CONTROL: f64 = 0.3915;
/// Rounded-corner curves: control points at this fraction of the radius from
/// the tangent points toward the corner (a quarter circle for 90°).
const KAPPA: f64 = 0.5523;
const PGF_PATTERNS: [&str; 8] = [
    "north east lines",
    "north west lines",
    "horizontal lines",
    "vertical lines",
    "grid",
    "crosshatch",
    "dots",
    "crosshatch dots",
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Tip {
    To,
    Stealth,
    Latex,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Shape {
    Rectangle,
    Circle,
    Ellipse,
    Coordinate,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DashLen {
    Pt(f64),
    LineWidth,
}

/// The graphics state: what TeX groups save and restore.
#[derive(Clone, Debug)]
struct St {
    tf: Transform,
    xv: V,
    yv: V,
    lw: f64,
    color: Color,
    draw_color: Option<Color>,
    fill_color: Option<Color>,
    pattern: Option<String>,
    pattern_color: Option<Color>,
    text_color: Option<Color>,
    do_draw: bool,
    do_fill: bool,
    do_clip: bool,
    bbox_only: bool,
    draw_opacity: f64,
    fill_opacity: f64,
    text_opacity: Option<f64>,
    dash: Option<Vec<DashLen>>,
    dash_phase: f64,
    cap: LineCap,
    join: LineJoin,
    miter: f64,
    start_tip: Option<Tip>,
    end_tip: Option<Tip>,
    gt_tip: Tip,
    rounded: Option<f64>,
    even_odd: bool,
    shape: Shape,
    inner_xsep: Option<f64>,
    inner_ysep: Option<f64>,
    outer_sep: Option<f64>,
    min_w: f64,
    min_h: f64,
    anchor: Option<String>,
    place_shift: V,
    pos: Option<f64>,
    sloped: bool,
    transform_shape: bool,
    font_size: f64,
    base_font: f64,
    bold: bool,
    italic: bool,
    name: Option<String>,
    radius: Option<Value>,
    x_radius: Option<Value>,
    y_radius: Option<Value>,
    start_angle: Option<f64>,
    end_angle: Option<f64>,
    delta_angle: Option<f64>,
    xstep: Value,
    ystep: Value,
    bend: Option<f64>,
    out_angle: Option<f64>,
    in_angle: Option<f64>,
    looseness: f64,
}

impl St {
    fn new(font: f64) -> St {
        St {
            tf: Transform::IDENTITY,
            xv: v(PT_PER_CM, 0.0),
            yv: v(0.0, PT_PER_CM),
            lw: 0.4,
            color: Color::BLACK,
            draw_color: None,
            fill_color: None,
            pattern: None,
            pattern_color: None,
            text_color: None,
            do_draw: false,
            do_fill: false,
            do_clip: false,
            bbox_only: false,
            draw_opacity: 1.0,
            fill_opacity: 1.0,
            text_opacity: None,
            dash: None,
            dash_phase: 0.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter: 10.0,
            start_tip: None,
            end_tip: None,
            gt_tip: Tip::To,
            rounded: None,
            even_odd: false,
            shape: Shape::Rectangle,
            inner_xsep: None,
            inner_ysep: None,
            outer_sep: None,
            min_w: 0.0,
            min_h: 0.0,
            anchor: None,
            place_shift: v(0.0, 0.0),
            pos: None,
            sloped: false,
            transform_shape: false,
            font_size: font,
            base_font: font,
            bold: false,
            italic: false,
            name: None,
            radius: None,
            x_radius: None,
            y_radius: None,
            start_angle: None,
            end_angle: None,
            delta_angle: None,
            xstep: Value { v: 1.0, dim: false },
            ystep: Value { v: 1.0, dim: false },
            bend: None,
            out_angle: None,
            in_angle: None,
            looseness: 1.0,
        }
    }

    /// A value along the x axis: lengths are points, plain numbers use the
    /// x vector (centimetres by default).
    fn xlen(&self, val: Value) -> f64 {
        if val.dim { val.v } else { val.v * len(self.xv) }
    }
    fn ylen(&self, val: Value) -> f64 {
        if val.dim { val.v } else { val.v * len(self.yv) }
    }
    fn xvec(&self, val: Value) -> V {
        if val.dim { v(val.v, 0.0) } else { mul(self.xv, val.v) }
    }
    fn yvec(&self, val: Value) -> V {
        if val.dim { v(0.0, val.v) } else { mul(self.yv, val.v) }
    }
    fn stroke_paint(&self) -> Paint {
        Paint::new(self.draw_color.unwrap_or(self.color), self.draw_opacity)
    }
    fn fill_paint(&self) -> Paint {
        Paint::new(self.fill_color.unwrap_or(self.color), self.fill_opacity)
    }
    fn fill_pattern(&self) -> Option<Pattern> {
        self.pattern.as_ref().map(|name| Pattern {
            name: name.clone(),
            color: self.pattern_color.unwrap_or(self.fill_paint().color),
        })
    }
    fn stroke_style(&self) -> StrokeStyle {
        let dash = self.dash.as_ref().filter(|d| !d.is_empty()).map(|d| Dash {
            array: d
                .iter()
                .map(|l| match l {
                    DashLen::Pt(p) => *p,
                    DashLen::LineWidth => self.lw,
                })
                .collect(),
            phase: self.dash_phase,
        });
        StrokeStyle {
            width: self.lw,
            cap: self.cap,
            join: self.join,
            miter_limit: self.miter,
            dash,
        }
    }
    fn apply_tf(&mut self, t: Transform) {
        // TikZ transformations act on coordinates before the ones already in
        // force.
        self.tf = t.then(&self.tf);
    }
}

#[derive(Clone, Debug)]
enum Raw {
    Fill { path: Path, even_odd: bool, paint: Paint, pattern: Option<Pattern> },
    Stroke { path: Path, style: StrokeStyle, paint: Paint },
    ClipBegin { path: Path, even_odd: bool },
    ClipEnd,
    Text { text: String, style: TextStyle, m: Transform, paint: Paint, span: (usize, usize) },
}

#[derive(Clone, Copy, Debug)]
enum Seg {
    M(V),
    L(V),
    C(V, V, V),
    Z,
}

#[derive(Clone, Copy, Debug)]
enum Last {
    None,
    Line(V, V),
    Curve(V, V, V, V),
    Corner(V, V, V),
}

/// Geometry of a named node, local coordinates in points (origin at the
/// text baseline start) and its local-to-canvas transform.
#[derive(Clone, Debug)]
struct NodeGeom {
    shape: Shape,
    m: Transform,
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
    c: V,
    rx: f64,
    ry: f64,
    outer: f64,
    em: f64,
}

impl NodeGeom {
    fn border_local(&self, dir: V) -> V {
        let Some(u) = unit(dir) else {
            return self.c;
        };
        let o = self.outer;
        match self.shape {
            Shape::Coordinate => self.c,
            Shape::Circle => add(self.c, mul(u, self.rx + o)),
            Shape::Ellipse => {
                let (a, b) = (self.rx + o, self.ry + o);
                let t = 1.0 / ((u.x / a).powi(2) + (u.y / b).powi(2)).sqrt();
                add(self.c, mul(u, t))
            }
            Shape::Rectangle => {
                let hw = (self.xmax - self.xmin) / 2.0 + o;
                let hh = (self.ymax - self.ymin) / 2.0 + o;
                let tx = if u.x.abs() > 1e-12 { hw / u.x.abs() } else { f64::INFINITY };
                let ty = if u.y.abs() > 1e-12 { hh / u.y.abs() } else { f64::INFINITY };
                add(self.c, mul(u, tx.min(ty)))
            }
        }
    }

    fn local_anchor(&self, name: &str) -> Option<V> {
        let name = name.trim();
        if let Ok(a) = expr::eval(name, self.em)
            && !a.dim {
                return Some(self.border_local(v(rad(a.v).cos(), rad(a.v).sin())));
            }
        let o = self.outer;
        let c = self.c;
        let mid_y = 0.5 * 0.430_555 * self.em;
        if self.shape == Shape::Rectangle {
            let (l, r, b, t) = (self.xmin - o, self.xmax + o, self.ymin - o, self.ymax + o);
            return Some(match name {
                "center" => c,
                "north" => v(c.x, t),
                "south" => v(c.x, b),
                "east" => v(r, c.y),
                "west" => v(l, c.y),
                "north east" => v(r, t),
                "north west" => v(l, t),
                "south east" => v(r, b),
                "south west" => v(l, b),
                "base" => v(c.x, 0.0),
                "base west" => v(l, 0.0),
                "base east" => v(r, 0.0),
                "mid" => v(c.x, mid_y),
                "mid west" => v(l, mid_y),
                "mid east" => v(r, mid_y),
                "text" => v(0.0, 0.0),
                _ => return None,
            });
        }
        let dir = |deg: f64| self.border_local(v(rad(deg).cos(), rad(deg).sin()));
        Some(match name {
            "center" => c,
            "east" => dir(0.0),
            "north east" => dir(45.0),
            "north" => dir(90.0),
            "north west" => dir(135.0),
            "west" => dir(180.0),
            "south west" => dir(225.0),
            "south" => dir(270.0),
            "south east" => dir(315.0),
            "base" => v(c.x, 0.0),
            "mid" => v(c.x, mid_y),
            "text" => v(0.0, 0.0),
            "base west" => v(dir(180.0).x, 0.0),
            "base east" => v(dir(0.0).x, 0.0),
            _ => return None,
        })
    }

    fn center(&self) -> V {
        self.m.apply(self.c)
    }

    fn border_toward(&self, target: V) -> V {
        match self.m.invert() {
            Some(inv) => {
                let lt = inv.apply(target);
                self.m.apply(self.border_local(sub(lt, self.c)))
            }
            None => self.center(),
        }
    }

    fn angle_anchor(&self, deg: f64) -> V {
        // The angle is a canvas direction.
        let d = v(rad(deg).cos(), rad(deg).sin());
        match self.m.invert() {
            Some(inv) => self.m.apply(self.border_local(inv.apply_vector(d))),
            None => self.center(),
        }
    }
}

struct NodeSpec {
    opts: String,
    name: Option<String>,
    at: Option<V>,
    text: Option<String>,
    coordinate: bool,
}

/// Path under construction.
struct Pb {
    segs: Vec<(Seg, Option<f64>)>,
    cur: V,
    rel: V,
    have_cur: bool,
    cur_node: Option<String>,
    last: Last,
    node_raws: Vec<Raw>,
}

pub(crate) struct Interp<'a> {
    styles: HashMap<String, (String, Option<String>)>,
    palette: Palette,
    measurer: &'a dyn TextMeasurer,
    macros: HashMap<String, String>,
    nodes: HashMap<String, NodeGeom>,
    raws: Vec<Raw>,
    bbox: Option<[f64; 4]>,
    bbox_locked: bool,
    pub diags: Vec<Diagnostic>,
    span: (usize, usize),
    depth: usize,
    base_font: f64,
}

fn is_word_at(s: &str, i: usize, w: &str) -> bool {
    s[i..].starts_with(w) && !s[i + w.len()..].starts_with(|c: char| c.is_ascii_alphabetic())
}

fn skip_ws(s: &str, mut i: usize) -> usize {
    let b = s.as_bytes();
    while i < b.len() && (b[i] as char).is_whitespace() {
        i += 1;
    }
    i
}

fn fmt_num(x: f64) -> String {
    if (x - x.round()).abs() < 1e-9 {
        format!("{}", x.round() as i64)
    } else {
        let s = format!("{x:.5}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn font_size_for(cmd: &str, base: f64) -> Option<f64> {
    // LaTeX size tables (size10.clo / size11.clo / size12.clo).
    let table: [f64; 10] = if base >= 11.5 {
        [6.0, 8.0, 10.0, 10.95, 12.0, 14.4, 17.28, 20.74, 24.88, 24.88]
    } else if base >= 10.5 {
        [6.0, 8.0, 9.0, 10.0, 10.95, 12.0, 14.4, 17.28, 20.74, 24.88]
    } else {
        [5.0, 7.0, 8.0, 9.0, 10.0, 12.0, 14.4, 17.28, 20.74, 24.88]
    };
    let idx = match cmd {
        "tiny" => 0,
        "scriptsize" => 1,
        "footnotesize" => 2,
        "small" => 3,
        "normalsize" => 4,
        "large" => 5,
        "Large" => 6,
        "LARGE" => 7,
        "huge" => 8,
        "Huge" => 9,
        _ => return None,
    };
    Some(table[idx])
}

impl<'a> Interp<'a> {
    pub(crate) fn new(ctx: &Tikz, measurer: &'a dyn TextMeasurer, offset: usize) -> Interp<'a> {
        Interp {
            styles: ctx.styles.clone(),
            palette: ctx.palette.clone(),
            measurer,
            macros: HashMap::new(),
            nodes: HashMap::new(),
            raws: Vec::new(),
            bbox: None,
            bbox_locked: false,
            diags: Vec::new(),
            span: (offset, offset),
            depth: 0,
            base_font: ctx.font_size_pt,
        }
    }

    pub(crate) fn into_definitions(self) -> (HashMap<String, (String, Option<String>)>, Palette) {
        (self.styles, self.palette)
    }

    fn warn(&mut self, message: impl Into<String>) {
        self.diags.push(Diagnostic {
            severity: Severity::Warning,
            message: message.into(),
            start: self.span.0,
            end: self.span.1,
        });
    }

    fn error(&mut self, message: impl Into<String>) {
        self.diags.push(Diagnostic {
            severity: Severity::Error,
            message: message.into(),
            start: self.span.0,
            end: self.span.1,
        });
    }

    fn bbox_add(&mut self, p: V) {
        if self.bbox_locked || !p.is_finite() {
            return;
        }
        self.bbox = Some(match self.bbox {
            None => [p.x, p.y, p.x, p.y],
            Some([a, b, c, d]) => [a.min(p.x), b.min(p.y), c.max(p.x), d.max(p.y)],
        });
    }

    // ----------------------------------------------------------------- blocks

    pub(crate) fn preamble(&mut self, text: &str) {
        let clean = tx::blank_comments(text);
        let mut st = St::new(self.base_font);
        let mut i = 0;
        while let Some(rel) = clean[i..].find('\\') {
            let at = i + rel;
            match tx::control_word(&clean, at) {
                Some((name, j)) if matches!(name, "tikzset" | "tikzstyle" | "definecolor" | "colorlet") => {
                    let end = self.statement_end(&clean, at);
                    self.span = (at, end);
                    let rest = clean[j..end].to_string();
                    if name == "tikzset" {
                        // Plain keys in a preamble \tikzset (e.g. `>=stealth`)
                        // become part of every picture.
                        let body = self.group_arg(&rest, 0).map(|(g, _)| g).unwrap_or_default();
                        for entry in split_top(&body, b',') {
                            let entry = entry.trim();
                            if entry.is_empty() {
                                continue;
                            }
                            let key = entry.split('=').next().unwrap_or("").trim();
                            if key.contains("/.") {
                                self.apply_opts(&mut st, entry);
                            } else {
                                let e = self.styles.entry("every picture".into()).or_insert((String::new(), None));
                                if !e.0.is_empty() {
                                    e.0.push(',');
                                }
                                e.0.push_str(entry);
                            }
                        }
                    } else {
                        let mut clips = 0;
                        self.command(name, &rest, &mut st, &mut clips, None);
                    }
                    i = end.max(j);
                }
                Some((_, j)) => i = j,
                None => i = at + 1,
            }
        }
    }

    pub(crate) fn picture(&mut self, options: &str, body: &str, offset: usize) {
        let mut st = St::new(self.base_font);
        self.span = (offset, offset + body.len());
        if self.styles.contains_key("every picture") {
            self.apply_opts(&mut st, "every picture");
        }
        self.apply_opts(&mut st, options);
        self.block(body, &mut st, Some(offset));
    }

    /// Index after one statement starting at `i`.
    fn statement_end(&self, s: &str, i: usize) -> usize {
        let b = s.as_bytes();
        if b.get(i) == Some(&b'{') {
            return matching(s, i).unwrap_or(s.len());
        }
        let Some((name, j)) = tx::control_word(s, i) else {
            return (i + 1).min(s.len());
        };
        let groups = |n: usize| {
            let mut k = j;
            for _ in 0..n {
                k = skip_ws(s, k);
                if s[k..].starts_with('{') {
                    k = matching(s, k).unwrap_or(s.len());
                } else if let Some((_, e)) = tx::control_word(s, k) {
                    k = e;
                } else {
                    break;
                }
            }
            k
        };
        match name {
            "begin" => {
                let k = skip_ws(s, j);
                let Some(e) = matching(s, k) else { return s.len() };
                let env = s[k + 1..e - 1].trim();
                let open = format!("\\begin{{{env}}}");
                let close = format!("\\end{{{env}}}");
                let mut depth = 1;
                let mut p = e;
                while p < s.len() {
                    let no = s[p..].find(&open).map(|x| x + p);
                    let nc = s[p..].find(&close).map(|x| x + p);
                    match (no, nc) {
                        (Some(o), Some(c)) if o < c => {
                            depth += 1;
                            p = o + open.len();
                        }
                        (_, Some(c)) => {
                            depth -= 1;
                            p = c + close.len();
                            if depth == 0 {
                                return p;
                            }
                        }
                        _ => return s.len(),
                    }
                }
                s.len()
            }
            "foreach" => {
                // Variables and options up to `in`, the list, then the body.
                let mut k = j;
                loop {
                    k = skip_ws(s, k);
                    if k >= s.len() {
                        return s.len();
                    }
                    if is_word_at(s, k, "in") {
                        k += 2;
                        break;
                    }
                    if s[k..].starts_with('[') || s[k..].starts_with('{') {
                        k = matching(s, k).unwrap_or(s.len());
                    } else if let Some((_, e)) = tx::control_word(s, k) {
                        k = e;
                    } else {
                        k += 1;
                    }
                }
                k = skip_ws(s, k);
                if s[k..].starts_with('{') {
                    k = matching(s, k).unwrap_or(s.len());
                } else if let Some((_, e)) = tx::control_word(s, k) {
                    k = e;
                }
                k = skip_ws(s, k);
                if k >= s.len() {
                    return s.len();
                }
                self.statement_end(s, k)
            }
            "tikzset" | "usetikzlibrary" | "pgfkeys" => groups(1),
            "definecolor" => groups(3),
            "colorlet" | "pgfmathsetmacro" | "newcommand" | "renewcommand" => groups(2),
            "def" => groups(2),
            "tikzstyle" => {
                let k = groups(1);
                let k = skip_ws(s, k);
                let k = if s[k..].starts_with("+=") {
                    k + 2
                } else if s[k..].starts_with('=') {
                    k + 1
                } else {
                    return k;
                };
                let k = skip_ws(s, k);
                if s[k..].starts_with('[') { matching(s, k).unwrap_or(s.len()) } else { k }
            }
            _ => {
                // Up to the top-level `;`.
                let mut depth = 0i32;
                let mut k = j;
                while k < b.len() {
                    match b[k] {
                        b'\\' => {
                            k += 2;
                            continue;
                        }
                        b'{' | b'[' | b'(' => depth += 1,
                        b'}' | b']' | b')' => depth -= 1,
                        b';' if depth <= 0 => return k + 1,
                        _ => {}
                    }
                    k += 1;
                }
                s.len()
            }
        }
    }

    fn block(&mut self, text: &str, st: &mut St, offset: Option<usize>) {
        if self.depth > MAX_DEPTH {
            self.error("TikZ nesting is too deep; the rest of this picture is skipped");
            return;
        }
        self.depth += 1;
        let mut clips = 0usize;
        let b = text.as_bytes();
        let mut i = 0;
        while i < b.len() {
            i = skip_ws(text, i);
            if i >= b.len() {
                break;
            }
            let end = self.statement_end(text, i).max(i + 1);
            if let Some(off) = offset {
                self.span = (off + i, off + end);
            }
            match b[i] {
                b'\\' => {
                    if let Some((name, j)) = tx::control_word(text, i) {
                        let rest = &text[j..end];
                        self.command(name, rest, st, &mut clips, offset.map(|o| o + j));
                    } else {
                        self.warn("unsupported control symbol in tikzpicture; skipped");
                    }
                }
                b'{' => {
                    let inner = &text[i + 1..end.saturating_sub(1).max(i + 1)];
                    let saved = self.save_defs();
                    let mut st2 = st.clone();
                    let lead = inner.len() - inner.trim_start().len();
                    let mut body_at = 0;
                    if inner.trim_start().starts_with('[')
                        && let Some(close) = matching(inner, lead) {
                            let opts = inner[lead + 1..close - 1].to_string();
                            self.apply_opts(&mut st2, &opts);
                            body_at = close;
                        }
                    self.block(&inner[body_at..], &mut st2, offset.map(|o| o + i + 1 + body_at));
                    self.restore_defs(saved);
                }
                b';' => {}
                _ => {
                    let snippet: String = text[i..end].chars().take(24).collect();
                    self.warn(format!("unexpected text `{}` in tikzpicture; skipped", snippet.trim()));
                }
            }
            i = end;
        }
        for _ in 0..clips {
            self.raws.push(Raw::ClipEnd);
        }
        self.depth -= 1;
    }

    fn save_defs(&self) -> Defs {
        (self.styles.clone(), self.palette.clone(), self.macros.clone())
    }

    fn restore_defs(&mut self, saved: Defs) {
        self.styles = saved.0;
        self.palette = saved.1;
        self.macros = saved.2;
    }

    /// Reads `{group}` at or after `i`; returns its content and the index after.
    fn group_arg(&self, s: &str, i: usize) -> Option<(String, usize)> {
        let k = skip_ws(s, i);
        if s[k..].starts_with('{') {
            let e = matching(s, k)?;
            Some((s[k + 1..e - 1].to_string(), e))
        } else if let Some((name, e)) = tx::control_word(s, k) {
            Some((format!("\\{name}"), e))
        } else {
            None
        }
    }

    fn command(&mut self, name: &str, rest: &str, st: &mut St, clips: &mut usize, rest_offset: Option<usize>) {
        match name {
            "draw" | "fill" | "filldraw" | "path" | "clip" => {
                let r = self.path_statement(name, rest, st);
                if r {
                    *clips += 1;
                }
            }
            "node" | "coordinate" => {
                let body = format!("{name} {rest}");
                self.path_statement("path", &body, st);
            }
            "shade" | "shadedraw" | "pic" | "matrix" | "graph" | "datavisualization" | "calendar" | "spy" => {
                self.warn(format!("\\{name} is not supported by the FlashTeX TikZ subset; skipped"));
            }
            "foreach" => self.foreach(rest, st),
            "begin" => {
                let Some((env, after)) = self.group_arg(rest, 0) else {
                    self.warn("malformed \\begin in tikzpicture; skipped");
                    return;
                };
                let env = env.trim().to_string();
                if env != "scope" {
                    self.warn(format!("environment `{env}` inside tikzpicture is not supported; skipped"));
                    return;
                }
                let close = "\\end{scope}";
                let body_end = rest.rfind(close).unwrap_or(rest.len());
                let mut k = skip_ws(rest, after);
                let mut opts = String::new();
                if rest[k..].starts_with('[')
                    && let Some(e) = matching(rest, k) {
                        opts = rest[k + 1..e - 1].to_string();
                        k = e;
                    }
                let saved = self.save_defs();
                let mut st2 = st.clone();
                if self.styles.contains_key("every scope") {
                    self.apply_opts(&mut st2, "every scope");
                }
                self.apply_opts(&mut st2, &opts);
                let body = &rest[k..body_end.max(k)];
                self.block(body, &mut st2, rest_offset.map(|o| o + k));
                self.restore_defs(saved);
            }
            "end" => self.warn("\\end without a matching \\begin in tikzpicture; skipped"),
            "tikzset" => {
                if let Some((g, _)) = self.group_arg(rest, 0) {
                    self.apply_opts(st, &g);
                }
            }
            "tikzstyle" => {
                let Some((nm, k)) = self.group_arg(rest, 0) else { return };
                let k = skip_ws(rest, k);
                let append = rest[k..].starts_with("+=");
                let k = skip_ws(rest, k + if append { 2 } else { 1 });
                let body = if rest[k..].starts_with('[') {
                    matching(rest, k).map(|e| rest[k + 1..e - 1].to_string()).unwrap_or_default()
                } else {
                    String::new()
                };
                let nm = nm.trim().to_string();
                if append {
                    let e = self.styles.entry(nm).or_insert((String::new(), None));
                    e.0 = format!("{},{}", e.0, body);
                } else {
                    self.styles.insert(nm, (body, None));
                }
            }
            "definecolor" => {
                let a = self.group_arg(rest, 0);
                let b = a.as_ref().and_then(|(_, k)| self.group_arg(rest, *k));
                let c = b.as_ref().and_then(|(_, k)| self.group_arg(rest, *k));
                match (a, b, c) {
                    (Some((n, _)), Some((m, _)), Some((vals, _))) => {
                        if let Err(e) = self.palette.define_from_spec(&n, &m, &vals) {
                            self.warn(e);
                        }
                    }
                    _ => self.warn("malformed \\definecolor; skipped"),
                }
            }
            "colorlet" => {
                let a = self.group_arg(rest, 0);
                let b = a.as_ref().and_then(|(_, k)| self.group_arg(rest, *k));
                if let (Some((n, _)), Some((e, _))) = (a, b) {
                    match self.palette.parse(&e) {
                        Some(c) => self.palette.define(&n, c),
                        None => self.warn(format!("unknown colour `{e}` in \\colorlet")),
                    }
                }
            }
            "def" | "newcommand" | "renewcommand" => {
                let Some((nm, k)) = self.group_arg(rest, 0) else { return };
                let k2 = skip_ws(rest, k);
                if rest[k2..].starts_with('[') || rest[k2..].starts_with('#') {
                    self.warn(format!("\\{name} with parameters is not supported in tikzpicture; skipped"));
                    return;
                }
                let Some((body, _)) = self.group_arg(rest, k) else { return };
                let nm = nm.trim().trim_start_matches('\\').to_string();
                self.macros.insert(nm, body);
            }
            "pgfmathsetmacro" | "pgfmathtruncatemacro" => {
                let a = self.group_arg(rest, 0);
                let b = a.as_ref().and_then(|(_, k)| self.group_arg(rest, *k));
                if let (Some((n, _)), Some((e, _))) = (a, b) {
                    let e = tx::substitute(&e, &self.macros);
                    match expr::eval(&e, st.font_size) {
                        Ok(val) => {
                            let x = if name == "pgfmathtruncatemacro" { val.v.trunc() } else { val.v };
                            self.macros.insert(n.trim().trim_start_matches('\\').to_string(), fmt_num(x));
                        }
                        Err(err) => self.warn(err),
                    }
                }
            }
            "usetikzlibrary" => {}
            _ => self.warn(format!("\\{name} is not supported inside tikzpicture; skipped")),
        }
    }

    // ---------------------------------------------------------------- foreach

    fn foreach(&mut self, rest: &str, st: &mut St) {
        let mut vars: Vec<String> = Vec::new();
        let mut count_var: Option<String> = None;
        let mut k = 0;
        loop {
            k = skip_ws(rest, k);
            if k >= rest.len() {
                self.warn("malformed \\foreach (no `in`); skipped");
                return;
            }
            if is_word_at(rest, k, "in") {
                k += 2;
                break;
            }
            if rest[k..].starts_with('[') {
                let e = matching(rest, k).unwrap_or(rest.len());
                for opt in split_top(&rest[k + 1..e.saturating_sub(1)], b',') {
                    let opt = opt.trim();
                    if let Some(c) = opt.strip_prefix("count") {
                        let c = c.trim_start().trim_start_matches('=').trim();
                        let c = c.split_whitespace().next().unwrap_or("");
                        count_var = Some(c.trim_start_matches('\\').to_string());
                    } else if !opt.is_empty() {
                        self.warn(format!("\\foreach option `{opt}` is not supported; ignored"));
                    }
                }
                k = e;
            } else if let Some((nm, e)) = tx::control_word(rest, k) {
                vars.push(nm.to_string());
                k = e;
            } else {
                k += 1;
            }
        }
        k = skip_ws(rest, k);
        let list = if rest[k..].starts_with('{') {
            let e = matching(rest, k).unwrap_or(rest.len());
            let l = rest[k + 1..e.saturating_sub(1)].to_string();
            k = e;
            l
        } else if let Some((nm, e)) = tx::control_word(rest, k) {
            k = e;
            self.macros.get(nm).cloned().unwrap_or_default()
        } else {
            self.warn("malformed \\foreach list; skipped");
            return;
        };
        let list = tx::substitute(&list, &self.macros);
        let items = match self.expand_list(&list, st.font_size) {
            Ok(items) => items,
            Err(e) => {
                self.warn(e);
                return;
            }
        };
        k = skip_ws(rest, k);
        let body = rest[k..].trim();
        let body = if body.starts_with('{') && matching(body, 0) == Some(body.len()) {
            &body[1..body.len() - 1]
        } else {
            body
        };
        for (idx, item) in items.iter().enumerate() {
            let saved = self.macros.clone();
            let parts: Vec<&str> = split_top(item, b'/').into_iter().map(str::trim).collect();
            for (vi, var) in vars.iter().enumerate() {
                let val = parts.get(vi).or(parts.last()).copied().unwrap_or("");
                self.macros.insert(var.clone(), strip_braces(val).to_string());
            }
            if let Some(c) = &count_var {
                self.macros.insert(c.clone(), (idx + 1).to_string());
            }
            let saved_styles = (self.styles.clone(), self.palette.clone());
            let mut st2 = st.clone();
            let span = self.span;
            self.block(body, &mut st2, None);
            self.span = span;
            self.styles = saved_styles.0;
            self.palette = saved_styles.1;
            self.macros = saved;
        }
    }

    fn expand_list(&self, list: &str, em: f64) -> Result<Vec<String>, String> {
        let elems: Vec<String> = split_top(list, b',').into_iter().map(|e| e.trim().to_string()).filter(|e| !e.is_empty()).collect();
        let mut out: Vec<String> = Vec::new();
        let num = |s: &str| expr::eval(s, em).ok().filter(|v| !v.dim).map(|v| v.v);
        let mut i = 0;
        while i < elems.len() {
            if elems[i] == "..." {
                let prev = out.last().and_then(|p| num(p));
                let prev2 = if out.len() >= 2 { num(&out[out.len() - 2]) } else { None };
                let next = elems.get(i + 1).and_then(|n| num(n));
                match (prev, next) {
                    (Some(p), Some(n)) => {
                        let step = match prev2 {
                            Some(p2) => p - p2,
                            None => {
                                if n >= p {
                                    1.0
                                } else {
                                    -1.0
                                }
                            }
                        };
                        if step.abs() < 1e-12 {
                            return Err("\\foreach `...` with a zero step".into());
                        }
                        let mut x = p + step;
                        while (step > 0.0 && x < n - 1e-9) || (step < 0.0 && x > n + 1e-9) {
                            out.push(fmt_num(x));
                            if out.len() > MAX_ITERATIONS {
                                return Err("\\foreach list is too long".into());
                            }
                            x += step;
                        }
                    }
                    _ => {
                        let pc = out.last().and_then(|p| single_char(p));
                        let nc = elems.get(i + 1).and_then(|n| single_char(n));
                        match (pc, nc) {
                            (Some(a), Some(b)) if a.is_ascii_alphabetic() && b.is_ascii_alphabetic() && a < b => {
                                for c in (a as u8 + 1)..(b as u8) {
                                    out.push((c as char).to_string());
                                }
                            }
                            _ => return Err("\\foreach `...` needs numeric or single-letter bounds".into()),
                        }
                    }
                }
            } else {
                out.push(elems[i].clone());
            }
            i += 1;
        }
        if out.len() > MAX_ITERATIONS {
            return Err("\\foreach list is too long".into());
        }
        Ok(out)
    }

    // ---------------------------------------------------------------- options

    fn apply_opts(&mut self, st: &mut St, opts: &str) {
        if self.depth > MAX_DEPTH {
            self.error("TikZ style nesting is too deep (recursive style?)");
            return;
        }
        let opts = tx::substitute(opts, &self.macros);
        self.depth += 1;
        for entry in split_top(&opts, b',') {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            let (key, val) = match tx::find_top(entry, b'=') {
                Some(p) => (entry[..p].trim(), Some(strip_braces(&entry[p + 1..]))),
                None => (entry, None),
            };
            let key = key.split_whitespace().collect::<Vec<_>>().join(" ");
            self.apply_key(st, &key, val);
        }
        self.depth -= 1;
    }

    fn eval(&mut self, text: &str, em: f64) -> Option<Value> {
        match expr::eval(text, em) {
            Ok(v) => Some(v),
            Err(e) => {
                self.warn(e);
                None
            }
        }
    }

    fn apply_key(&mut self, st: &mut St, key: &str, val: Option<&str>) {
        let em = st.font_size;
        if let Some(base) = key.strip_suffix("/.style") {
            self.styles.insert(base.trim().to_string(), (val.unwrap_or("").to_string(), None));
            return;
        }
        if let Some(base) = key.strip_suffix("/.append style") {
            let e = self.styles.entry(base.trim().to_string()).or_insert((String::new(), None));
            e.0 = format!("{},{}", e.0, val.unwrap_or(""));
            return;
        }
        if let Some(base) = key.strip_suffix("/.default") {
            let e = self.styles.entry(base.trim().to_string()).or_insert((String::new(), None));
            e.1 = val.map(str::to_string);
            return;
        }
        if key.contains("/.") {
            self.warn(format!("TikZ key handler `{key}` is not supported; ignored"));
            return;
        }
        if let Some((body, default)) = self.styles.get(key).cloned() {
            let arg = val.map(str::to_string).or(default).unwrap_or_default();
            let body = body.replace("#1", &arg);
            self.apply_opts(st, &body);
            return;
        }
        let width = |w: f64| Some(w);
        let lw = match key {
            "ultra thin" => width(0.1),
            "very thin" => width(0.2),
            "thin" => width(0.4),
            "semithick" => width(0.6),
            "thick" => width(0.8),
            "very thick" => width(1.2),
            "ultra thick" => width(1.6),
            _ => None,
        };
        if let Some(w) = lw {
            st.lw = w;
            return;
        }
        let dash = |on_off: &[DashLen]| Some(on_off.to_vec());
        use DashLen::{LineWidth as LW, Pt};
        let pattern = match key {
            "solid" => Some(Vec::new()),
            "dotted" => dash(&[LW, Pt(2.0)]),
            "densely dotted" => dash(&[LW, Pt(1.0)]),
            "loosely dotted" => dash(&[LW, Pt(4.0)]),
            "dashed" => dash(&[Pt(3.0), Pt(3.0)]),
            "densely dashed" => dash(&[Pt(3.0), Pt(2.0)]),
            "loosely dashed" => dash(&[Pt(3.0), Pt(6.0)]),
            "dash dot" | "dashdotted" => dash(&[Pt(3.0), Pt(2.0), LW, Pt(2.0)]),
            "densely dash dot" | "densely dashdotted" => dash(&[Pt(3.0), Pt(1.0), LW, Pt(1.0)]),
            "loosely dash dot" | "loosely dashdotted" => dash(&[Pt(3.0), Pt(4.0), LW, Pt(4.0)]),
            _ => None,
        };
        if let Some(p) = pattern {
            // `\the\pgflinewidth` patterns freeze the width when set; plain
            // `dotted` reads it when the path is used. Both are the width at
            // use in practice for this subset.
            st.dash = Some(p);
            st.dash_phase = 0.0;
            return;
        }
        let val_s = val.unwrap_or("");
        match key {
            "line width" => {
                if let Some(x) = self.eval(val_s, em) {
                    st.lw = x.v;
                }
            }
            "help lines" => {
                st.color = Color::Gray(0.5);
                st.lw = 0.2;
            }
            "color" => match self.palette.parse(val_s) {
                Some(c) => {
                    st.color = c;
                    st.draw_color = None;
                    st.fill_color = None;
                    st.text_color = None;
                }
                None => self.warn(format!("unknown colour `{val_s}`")),
            },
            "pattern" => {
                let name = val_s.trim();
                if PGF_PATTERNS.contains(&name) {
                    st.pattern = Some(name.to_string());
                    st.do_fill = true;
                } else {
                    self.warn(format!("unknown pattern `{name}`; ignored"));
                }
            }
            "pattern color" => match self.palette.parse(val_s) {
                Some(c) => st.pattern_color = Some(c),
                None => self.warn(format!("unknown colour `{val_s}` for pattern color")),
            },
            "draw" => self.mode(st, val, true),
            "fill" => self.mode(st, val, false),
            "text" => match self.palette.parse(val_s) {
                Some(c) => st.text_color = Some(c),
                None => self.warn(format!("unknown colour `{val_s}`")),
            },
            "opacity" | "draw opacity" | "fill opacity" | "text opacity" => {
                if let Some(x) = self.eval(val_s, em) {
                    let a = x.v.clamp(0.0, 1.0);
                    match key {
                        "opacity" => {
                            st.draw_opacity = a;
                            st.fill_opacity = a;
                        }
                        "draw opacity" => st.draw_opacity = a,
                        "fill opacity" => st.fill_opacity = a,
                        _ => st.text_opacity = Some(a),
                    }
                }
            }
            "transparent" => {
                st.draw_opacity = 0.0;
                st.fill_opacity = 0.0;
            }
            "dash pattern" => {
                let words: Vec<&str> = val_s.split_whitespace().collect();
                let mut out = Vec::new();
                let mut i = 0;
                while i + 1 < words.len() {
                    if words[i] != "on" && words[i] != "off" {
                        break;
                    }
                    let w = words[i + 1];
                    if w.contains("pgflinewidth") {
                        out.push(DashLen::LineWidth);
                    } else if let Some(x) = self.eval(w, em) {
                        out.push(DashLen::Pt(x.v));
                    }
                    i += 2;
                }
                st.dash = Some(out);
            }
            "dash phase" => {
                if let Some(x) = self.eval(val_s, em) {
                    st.dash_phase = x.v;
                }
            }
            "line cap" => {
                st.cap = match val_s {
                    "round" => LineCap::Round,
                    "rect" => LineCap::Square,
                    _ => LineCap::Butt,
                }
            }
            "line join" => {
                st.join = match val_s {
                    "round" => LineJoin::Round,
                    "bevel" => LineJoin::Bevel,
                    _ => LineJoin::Miter,
                }
            }
            "miter limit" => {
                if let Some(x) = self.eval(val_s, em) {
                    st.miter = x.v;
                }
            }
            "rounded corners" => {
                let r = if val_s.is_empty() { Some(4.0) } else { self.eval(val_s, em).map(|x| x.v) };
                st.rounded = r;
            }
            "sharp corners" => st.rounded = None,
            "even odd rule" => st.even_odd = true,
            "nonzero rule" => st.even_odd = false,
            "scale" | "xscale" | "yscale" => {
                if let Some(x) = self.eval(val_s, em) {
                    let (sx, sy) = match key {
                        "scale" => (x.v, x.v),
                        "xscale" => (x.v, 1.0),
                        _ => (1.0, x.v),
                    };
                    st.apply_tf(Transform::scale(sx, sy));
                }
            }
            "xshift" | "yshift" => {
                if let Some(x) = self.eval(val_s, em) {
                    let d = if key == "xshift" { st.xvec(x) } else { st.yvec(x) };
                    st.apply_tf(Transform::translate(d.x, d.y));
                }
            }
            "shift" => {
                if let Some(u) = self.user_vector(st, val_s) {
                    st.apply_tf(Transform::translate(u.x, u.y));
                }
            }
            "rotate" => {
                if let Some(x) = self.eval(val_s, em) {
                    st.apply_tf(Transform::rotate(rad(x.v)));
                }
            }
            "rotate around" => {
                let parts: Vec<&str> = val_s.splitn(2, ':').collect();
                if parts.len() == 2 {
                    if let (Some(a), Some(p)) = (self.eval(parts[0], em), self.user_vector(st, parts[1])) {
                        st.apply_tf(Transform::rotate_about(rad(a.v), p));
                    }
                } else {
                    self.warn("malformed `rotate around`");
                }
            }
            "reset cm" => st.tf = Transform::IDENTITY,
            "x" | "y" => {
                let vec = if val_s.starts_with('(') {
                    self.user_vector(st, val_s)
                } else {
                    self.eval(val_s, em).map(|x| if key == "x" { v(x.v, 0.0) } else { v(0.0, x.v) })
                };
                if let Some(u) = vec {
                    if key == "x" {
                        st.xv = u;
                    } else {
                        st.yv = u;
                    }
                }
            }
            ">" => match parse_tip(val_s) {
                Some(t) => st.gt_tip = t,
                None => self.warn(format!("arrow tip `{val_s}` is not supported; using the default")),
            },
            "arrows" => {
                self.arrows(st, val_s);
            }
            "inner sep" | "inner xsep" | "inner ysep" | "outer sep" | "minimum size" | "minimum width" | "minimum height" => {
                if let Some(x) = self.eval(val_s, em) {
                    match key {
                        "inner sep" => {
                            st.inner_xsep = Some(x.v);
                            st.inner_ysep = Some(x.v);
                        }
                        "inner xsep" => st.inner_xsep = Some(x.v),
                        "inner ysep" => st.inner_ysep = Some(x.v),
                        "outer sep" => st.outer_sep = Some(x.v),
                        "minimum size" => {
                            st.min_w = x.v;
                            st.min_h = x.v;
                        }
                        "minimum width" => st.min_w = x.v,
                        _ => st.min_h = x.v,
                    }
                }
            }
            "circle" => st.shape = Shape::Circle,
            "ellipse" => st.shape = Shape::Ellipse,
            "rectangle" => st.shape = Shape::Rectangle,
            "shape" => match val_s {
                "circle" => st.shape = Shape::Circle,
                "rectangle" => st.shape = Shape::Rectangle,
                "ellipse" => st.shape = Shape::Ellipse,
                "coordinate" => st.shape = Shape::Coordinate,
                _ => self.warn(format!("node shape `{val_s}` is not supported; using rectangle")),
            },
            "anchor" => st.anchor = Some(val_s.to_string()),
            "above" | "below" | "left" | "right" | "above left" | "above right" | "below left" | "below right" => {
                let anchor = match key {
                    "above" => "south",
                    "below" => "north",
                    "left" => "east",
                    "right" => "west",
                    "above left" => "south east",
                    "above right" => "south west",
                    "below left" => "north east",
                    _ => "north west",
                };
                st.anchor = Some(anchor.to_string());
                let off = if val_s.is_empty() {
                    0.0
                } else if val_s.contains(" of ") || val_s.starts_with("of ") {
                    self.warn(format!("`{key}={val_s}` (positioning library) is not supported"));
                    0.0
                } else {
                    self.eval(val_s, em).map(|x| st.ylen(x)).unwrap_or(0.0)
                };
                let (dx, dy) = match key {
                    "above" => (0.0, off),
                    "below" => (0.0, -off),
                    "left" => (-off, 0.0),
                    "right" => (off, 0.0),
                    "above left" => (-off, off),
                    "above right" => (off, off),
                    "below left" => (-off, -off),
                    _ => (off, -off),
                };
                st.place_shift = v(dx, dy);
            }
            "midway" => st.pos = Some(0.5),
            "near start" => st.pos = Some(0.25),
            "near end" => st.pos = Some(0.75),
            "very near start" => st.pos = Some(0.125),
            "very near end" => st.pos = Some(0.875),
            "at start" => st.pos = Some(0.0),
            "at end" => st.pos = Some(1.0),
            "pos" => {
                if let Some(x) = self.eval(val_s, em) {
                    st.pos = Some(x.v);
                }
            }
            "sloped" => st.sloped = true,
            "transform shape" => st.transform_shape = true,
            "font" => self.font(st, val_s),
            "name" => st.name = Some(val_s.to_string()),
            "radius" | "x radius" | "y radius" => {
                if let Some(x) = self.eval(val_s, em) {
                    match key {
                        "radius" => st.radius = Some(x),
                        "x radius" => st.x_radius = Some(x),
                        _ => st.y_radius = Some(x),
                    }
                }
            }
            "start angle" | "end angle" | "delta angle" => {
                if let Some(x) = self.eval(val_s, em) {
                    match key {
                        "start angle" => st.start_angle = Some(x.v),
                        "end angle" => st.end_angle = Some(x.v),
                        _ => st.delta_angle = Some(x.v),
                    }
                }
            }
            "step" | "xstep" | "ystep" => {
                if val_s.starts_with('(') {
                    self.warn("vector grid steps are not supported");
                } else if let Some(x) = self.eval(val_s, em) {
                    match key {
                        "step" => {
                            st.xstep = x;
                            st.ystep = x;
                        }
                        "xstep" => st.xstep = x,
                        _ => st.ystep = x,
                    }
                }
            }
            "bend left" | "bend right" => {
                let a = if val_s.is_empty() { Some(30.0) } else { self.eval(val_s, em).map(|x| x.v) };
                if let Some(a) = a {
                    st.bend = Some(if key == "bend left" { a } else { -a });
                }
            }
            "out" | "in" => {
                if let Some(x) = self.eval(val_s, em) {
                    if key == "out" {
                        st.out_angle = Some(x.v);
                    } else {
                        st.in_angle = Some(x.v);
                    }
                }
            }
            "looseness" => {
                if let Some(x) = self.eval(val_s, em) {
                    st.looseness = x.v;
                }
            }
            "use as bounding box" => st.bbox_only = true,
            "clip" => st.do_clip = true,
            "align" | "baseline" | "every node" | "every path" => {
                if key == "baseline" || key == "align" {
                    // Baseline changes only the vertical placement in the
                    // surrounding line; align affects multi-line text only.
                }
            }
            _ => {
                if val.is_none() {
                    if let Some(c) = self.palette.parse(key) {
                        st.color = c;
                        st.draw_color = None;
                        st.fill_color = None;
                        st.text_color = None;
                        return;
                    }
                    if key.contains('-') && self.arrows(st, key) {
                        return;
                    }
                }
                self.warn(format!("TikZ option `{key}` is not supported; ignored"));
            }
        }
    }

    fn mode(&mut self, st: &mut St, val: Option<&str>, draw: bool) {
        let (flag, color) = match val {
            None | Some("") => (true, None),
            Some("none") => (false, None),
            Some(c) => match self.palette.parse(c) {
                Some(col) => (true, Some(col)),
                None => {
                    self.warn(format!("unknown colour `{c}`"));
                    (true, None)
                }
            },
        };
        if draw {
            st.do_draw = flag;
            if color.is_some() {
                st.draw_color = color;
            }
        } else {
            st.do_fill = flag;
            if color.is_some() {
                st.fill_color = color;
            }
        }
    }

    /// Parses an arrow specification like `->`, `<->`, `-stealth`, `latex-`.
    fn arrows(&mut self, st: &mut St, spec: &str) -> bool {
        let spec = spec.trim();
        let Some(dash) = spec.find('-') else { return false };
        let (l, r) = (spec[..dash].trim(), spec[dash + 1..].trim());
        let side = |s: &str, gt: Tip| -> Result<Option<Tip>, ()> {
            match s {
                "" => Ok(None),
                ">" | "<" => Ok(Some(gt)),
                _ => parse_tip(s).map(Some).ok_or(()),
            }
        };
        match (side(l, st.gt_tip), side(r, st.gt_tip)) {
            (Ok(a), Ok(b)) => {
                if l == ">" || r == "<" {
                    self.warn(format!("reversed arrow tips in `{spec}` are drawn as forward tips"));
                }
                st.start_tip = a;
                st.end_tip = b;
                true
            }
            _ => {
                if spec.chars().all(|c| c.is_ascii_alphabetic() || "<>|-. ".contains(c)) && spec.len() <= 24 {
                    self.warn(format!("arrow specification `{spec}` is not supported; no tips drawn"));
                    true
                } else {
                    false
                }
            }
        }
    }

    fn font(&mut self, st: &mut St, val: &str) {
        let mut i = 0;
        while let Some(rel) = val[i..].find('\\') {
            let at = i + rel;
            let Some((nm, e)) = tx::control_word(val, at) else { break };
            if let Some(size) = font_size_for(nm, st.base_font) {
                st.font_size = size;
            } else {
                match nm {
                    "bfseries" | "bf" => st.bold = true,
                    "itshape" | "it" | "em" | "slshape" => st.italic = true,
                    "mdseries" => st.bold = false,
                    "upshape" | "rmfamily" | "normalfont" => {
                        st.italic = false;
                        if nm == "normalfont" {
                            st.bold = false;
                        }
                    }
                    _ => self.warn(format!("font command `\\{nm}` is not supported in TikZ fonts; ignored")),
                }
            }
            i = e;
        }
    }

    // ------------------------------------------------------------ coordinates

    /// A `(x,y)`/`(a:r)` vector in user space without the transformation.
    fn user_vector(&mut self, st: &St, text: &str) -> Option<V> {
        let t = text.trim();
        let t = t.strip_prefix('(').and_then(|x| x.strip_suffix(')')).unwrap_or(t);
        match self.coord_kind(st, t) {
            Some(CoordKind::User(u)) => Some(u),
            Some(CoordKind::Canvas(p, _)) => Some(p),
            None => None,
        }
    }

    fn coord_kind(&mut self, st: &St, content: &str) -> Option<CoordKind> {
        let content = content.trim();
        let em = st.font_size;
        if content.starts_with('$') {
            return self.calc(st, content);
        }
        if content.starts_with('[') {
            let e = matching(content, 0)?;
            let mut local = st.clone();
            let opts = content[1..e - 1].to_string();
            self.apply_opts(&mut local, &opts);
            return match self.coord_kind(&local, &content[e..])? {
                CoordKind::User(u) => Some(CoordKind::Canvas(local.tf.apply(u), None)),
                CoordKind::Canvas(p, n) => {
                    let shift = sub(local.tf.apply(v(0.0, 0.0)), st.tf.apply(v(0.0, 0.0)));
                    Some(CoordKind::Canvas(add(p, shift), n))
                }
            };
        }
        let comma = split_top(content, b',');
        if comma.len() == 2 {
            let x = self.eval(comma[0], em)?;
            let y = self.eval(comma[1], em)?;
            return Some(CoordKind::User(add(st.xvec(x), st.yvec(y))));
        }
        let colon = split_top(content, b':');
        if colon.len() == 2 {
            let a = self.eval(colon[0], em)?.v;
            let (rx, ry) = match colon[1].split_once(" and ") {
                Some((x, y)) => (self.eval(x, em)?, self.eval(y, em)?),
                None => {
                    let r = self.eval(colon[1], em)?;
                    (r, r)
                }
            };
            let p = add(mul(st.xvec(rx), rad(a).cos()), mul(st.yvec(ry), rad(a).sin()));
            return Some(CoordKind::User(p));
        }
        // A node, optionally with an anchor.
        if let Some(n) = self.nodes.get(content) {
            let geom = n.clone();
            return Some(CoordKind::Canvas(geom.center(), if geom.shape == Shape::Coordinate { None } else { Some(content.to_string()) }));
        }
        if let Some((name, anchor)) = content.rsplit_once('.')
            && let Some(n) = self.nodes.get(name.trim()).cloned() {
                return match n.local_anchor(anchor) {
                    Some(p) => Some(CoordKind::Canvas(n.m.apply(p), None)),
                    None => {
                        self.warn(format!("anchor `{anchor}` of node `{name}` is not supported"));
                        None
                    }
                };
            }
        self.warn(format!("unknown coordinate `({content})`"));
        None
    }

    /// `($(a)!t!(b)$)` and `($(a) + (b)$)` / `($(a) - (b)$)` / `($k*(a)$)`.
    fn calc(&mut self, st: &St, content: &str) -> Option<CoordKind> {
        let inner = content.trim().trim_start_matches('$').trim_end_matches('$').trim();
        let point = |this: &mut Self, s: &str| -> Option<V> {
            let s = s.trim();
            let s2 = s.strip_prefix('(').and_then(|x| x.strip_suffix(')'))?;
            match this.coord_kind(st, s2)? {
                CoordKind::User(u) => Some(st.tf.apply(u)),
                CoordKind::Canvas(p, _) => Some(p),
            }
        };
        let bangs = split_top(inner, b'!');
        if bangs.len() == 3 {
            let a = point(self, bangs[0])?;
            let b = point(self, bangs[2])?;
            let t = self.eval(bangs[1], st.font_size)?.v;
            return Some(CoordKind::Canvas(add(a, mul(sub(b, a), t)), None));
        }
        // Sum of optionally scaled terms.
        let mut total = v(0.0, 0.0);
        let mut i = 0;
        let bytes = inner.as_bytes();
        let mut sign = 1.0;
        let mut any = false;
        while i < bytes.len() {
            i = skip_ws(inner, i);
            if i >= bytes.len() {
                break;
            }
            match bytes[i] {
                b'+' => {
                    sign = 1.0;
                    i += 1;
                }
                b'-' => {
                    sign = -1.0;
                    i += 1;
                }
                _ => {
                    let open = inner[i..].find('(')? + i;
                    let factor = inner[i..open].trim().trim_end_matches('*').trim();
                    let k = if factor.is_empty() { 1.0 } else { self.eval(factor, st.font_size)?.v };
                    let close = matching(inner, open)?;
                    let p = point(self, &inner[open..close])?;
                    total = add(total, mul(p, sign * k));
                    any = true;
                    sign = 1.0;
                    i = close;
                }
            }
        }
        if !any {
            self.warn(format!("calc expression `{content}` is not supported"));
            return None;
        }
        // Sums of canvas points: only the first term keeps the translation.
        Some(CoordKind::Canvas(total, None))
    }

    /// Parses a coordinate at `i` (`(..)`, `+(..)`, `++(..)`). Returns the
    /// canvas point, the node name when it is a bare node, and the index
    /// after it.
    fn coordinate(&mut self, pb: &mut Pb, st: &St, s: &str, i: usize) -> Option<(V, Option<String>, usize)> {
        let mut k = skip_ws(s, i);
        let rel = if s[k..].starts_with("++") {
            k += 2;
            2
        } else if s[k..].starts_with('+') {
            k += 1;
            1
        } else {
            0
        };
        k = skip_ws(s, k);
        if !s[k..].starts_with('(') {
            return None;
        }
        let e = matching(s, k)?;
        let kind = self.coord_kind(st, &s[k + 1..e - 1])?;
        let (p, node) = match kind {
            CoordKind::User(u) => {
                if rel > 0 {
                    (add(pb.rel, st.tf.apply_vector(u)), None)
                } else {
                    (st.tf.apply(u), None)
                }
            }
            CoordKind::Canvas(p, n) => {
                if rel > 0 {
                    self.warn("relative node coordinates are not supported; used as absolute");
                }
                (p, n)
            }
        };
        if rel != 1 {
            pb.rel = p;
        }
        Some((p, node, e))
    }

    // ------------------------------------------------------------------ paths

    /// Returns `true` when the path installed a clip.
    fn path_statement(&mut self, kind: &str, rest: &str, st: &St) -> bool {
        let mut ps = st.clone();
        ps.do_draw = matches!(kind, "draw" | "filldraw");
        ps.do_fill = matches!(kind, "fill" | "filldraw");
        ps.do_clip = kind == "clip";
        ps.bbox_only = false;
        if self.styles.contains_key("every path") {
            self.apply_opts(&mut ps, "every path");
        }
        let s = tx::substitute(rest, &self.macros);
        let s = s.trim_end().trim_end_matches(';');
        let mut pb = Pb {
            segs: Vec::new(),
            cur: v(0.0, 0.0),
            rel: v(0.0, 0.0),
            have_cur: false,
            cur_node: None,
            last: Last::None,
            node_raws: Vec::new(),
        };
        let b = s.as_bytes();
        let mut i = 0;
        let mut ok = true;
        while i < b.len() {
            i = skip_ws(s, i);
            if i >= b.len() {
                break;
            }
            let r = self.path_op(&mut pb, &mut ps, s, i);
            match r {
                Some(n) if n > i => i = n,
                _ => {
                    ok = false;
                    break;
                }
            }
        }
        let _ = ok;
        self.finish_path(pb, &ps)
    }

    fn path_op(&mut self, pb: &mut Pb, ps: &mut St, s: &str, i: usize) -> Option<usize> {
        let c = s.as_bytes()[i];
        if c == b'[' {
            let e = matching(s, i)?;
            let opts = s[i + 1..e - 1].to_string();
            self.apply_opts(ps, &opts);
            return Some(e);
        }
        if c == b'(' || c == b'+' {
            let (p, node, e) = self.coordinate(pb, ps, s, i)?;
            self.move_to(pb, p, node);
            return Some(e);
        }
        if s[i..].starts_with("--") || s[i..].starts_with("-|") || s[i..].starts_with("|-") {
            let op = &s[i..i + 2];
            let (deferred, k) = self.deferred_nodes(s, i + 2)?;
            let k = skip_ws(s, k);
            if op == "--" && is_word_at(s, k, "cycle") {
                self.close(pb, ps);
                self.place_deferred(pb, ps, deferred);
                return Some(k + 5);
            }
            let (p, node, e) = self.coordinate(pb, ps, s, k).or_else(|| {
                self.warn(format!("expected a coordinate after `{op}`"));
                None
            })?;
            if op == "--" {
                self.line_to(pb, ps, p, node);
            } else {
                let corner = if op == "-|" { v(p.x, pb.cur.y) } else { v(pb.cur.x, p.y) };
                self.corner_to(pb, ps, corner, p, node);
            }
            self.place_deferred(pb, ps, deferred);
            return Some(e);
        }
        if s[i..].starts_with("..") {
            let mut k = skip_ws(s, i + 2);
            if !is_word_at(s, k, "controls") {
                self.warn("expected `controls` after `..`");
                return None;
            }
            k += "controls".len();
            let start_rel = pb.rel;
            let (c1, _, e1) = self.coordinate(pb, ps, s, k)?;
            let mut k = skip_ws(s, e1);
            let mut c2_at = None;
            if is_word_at(s, k, "and") {
                c2_at = Some(k + 3);
                // Skip the second control for now; it may be relative to the end.
                let k2 = skip_ws(s, k + 3);
                let k2 = if s[k2..].starts_with("++") { k2 + 2 } else if s[k2..].starts_with('+') { k2 + 1 } else { k2 };
                let k2 = skip_ws(s, k2);
                k = matching(s, k2)?;
                k = skip_ws(s, k);
            }
            if !s[k..].starts_with("..") {
                self.warn("expected `..` after the control points");
                return None;
            }
            pb.rel = start_rel;
            let (deferred, k3) = self.deferred_nodes(s, k + 2)?;
            // Target relative to the start point.
            let (p, node, e) = self.coordinate(pb, ps, s, k3)?;
            let c2 = match c2_at {
                Some(at) => {
                    let saved = pb.rel;
                    pb.rel = p;
                    let r = self.coordinate(pb, ps, s, at).map(|x| x.0);
                    pb.rel = saved;
                    r?
                }
                None => c1,
            };
            pb.rel = p;
            self.curve_to(pb, ps, c1, c2, p, node);
            self.place_deferred(pb, ps, deferred);
            return Some(e);
        }
        if !c.is_ascii_alphabetic() {
            let snippet: String = s[i..].chars().take(16).collect();
            self.warn(format!("unsupported path syntax near `{snippet}`; rest of path skipped"));
            return None;
        }
        let mut e = i;
        while e < s.len() && s.as_bytes()[e].is_ascii_alphabetic() {
            e += 1;
        }
        let word = &s[i..e];
        match word {
            "cycle" => {
                self.close(pb, ps);
                Some(e)
            }
            "node" | "coordinate" => {
                let (spec, k) = self.node_spec(s, i, ps, pb)?;
                let (pos, ang) = match (spec.at, spec_pos(&spec)) {
                    (Some(at), _) => (at, None),
                    (None, Some(t)) => self.path_position(pb, Some(t), 1.0),
                    (None, None) => (pb.cur, self.path_position(pb, Some(1.0), 1.0).1),
                };
                let raws = self.place_node(&spec, ps, pos, ang);
                pb.node_raws.extend(raws);
                Some(k)
            }
            "rectangle" => {
                let (deferred, k) = self.deferred_nodes(s, e)?;
                let from = pb.cur;
                let (p, _, k2) = self.coordinate(pb, ps, s, k)?;
                if !pb.have_cur {
                    self.move_to(pb, from, None);
                }
                self.rectangle(pb, ps, from, p);
                self.place_deferred(pb, ps, deferred);
                Some(k2)
            }
            "circle" | "ellipse" => {
                let mut local = ps.clone();
                let mut k = skip_ws(s, e);
                if s[k..].starts_with('[') {
                    let close = matching(s, k)?;
                    let opts = s[k + 1..close - 1].to_string();
                    self.apply_opts(&mut local, &opts);
                    k = skip_ws(s, close);
                }
                let em = local.font_size;
                if s[k..].starts_with('(') {
                    let close = matching(s, k)?;
                    let inner = &s[k + 1..close - 1];
                    match inner.split_once(" and ") {
                        Some((a, b)) => {
                            local.x_radius = Some(self.eval(a, em)?);
                            local.y_radius = Some(self.eval(b, em)?);
                        }
                        None => local.radius = Some(self.eval(inner, em)?),
                    }
                    k = close;
                }
                let rx = local.x_radius.or(local.radius);
                let ry = local.y_radius.or(local.radius);
                let (Some(rx), Some(ry)) = (rx, ry) else {
                    self.warn(format!("{word} without a radius; skipped"));
                    return Some(k);
                };
                let u = ps.tf.apply_vector(local.xvec(rx));
                let w = ps.tf.apply_vector(local.yvec(ry));
                let center = pb.cur;
                if !pb.have_cur {
                    self.move_to(pb, center, None);
                }
                let cn = pb.cur_node.take();
                let _ = cn;
                self.ellipse(pb, center, u, w);
                Some(k)
            }
            "arc" => {
                let mut local = ps.clone();
                let mut k = skip_ws(s, e);
                if s[k..].starts_with('[') {
                    let close = matching(s, k)?;
                    let opts = s[k + 1..close - 1].to_string();
                    self.apply_opts(&mut local, &opts);
                    k = skip_ws(s, close);
                }
                let em = local.font_size;
                if s[k..].starts_with('(') {
                    let close = matching(s, k)?;
                    let parts = split_top(&s[k + 1..close - 1], b':');
                    if parts.len() != 3 {
                        self.warn("arc needs (start:end:radius)");
                        return None;
                    }
                    local.start_angle = Some(self.eval(parts[0], em)?.v);
                    local.end_angle = Some(self.eval(parts[1], em)?.v);
                    match parts[2].split_once(" and ") {
                        Some((a, b)) => {
                            local.x_radius = Some(self.eval(a, em)?);
                            local.y_radius = Some(self.eval(b, em)?);
                        }
                        None => {
                            let r = self.eval(parts[2], em)?;
                            local.radius = Some(r);
                            local.x_radius = None;
                            local.y_radius = None;
                        }
                    }
                    k = close;
                }
                let rx = local.x_radius.or(local.radius);
                let ry = local.y_radius.or(local.radius);
                let sa = local.start_angle;
                let ea = local.end_angle.or_else(|| Some(sa? + local.delta_angle?));
                let (Some(rx), Some(ry), Some(sa), Some(ea)) = (rx, ry, sa, ea) else {
                    self.warn("arc needs a start angle, an end angle (or delta) and a radius; skipped");
                    return Some(k);
                };
                let u = ps.tf.apply_vector(local.xvec(rx));
                let w = ps.tf.apply_vector(local.yvec(ry));
                if !pb.have_cur {
                    let c = pb.cur;
                    self.move_to(pb, c, None);
                }
                self.arc(pb, u, w, sa, ea);
                Some(k)
            }
            "grid" => {
                let mut local = ps.clone();
                let mut k = skip_ws(s, e);
                if s[k..].starts_with('[') {
                    let close = matching(s, k)?;
                    let opts = s[k + 1..close - 1].to_string();
                    self.apply_opts(&mut local, &opts);
                    k = close;
                }
                let from = pb.cur;
                let (p, _, k2) = self.coordinate(pb, ps, s, k)?;
                self.grid(pb, ps, &local, from, p);
                Some(k2)
            }
            "to" => {
                let mut local = ps.clone();
                local.bend = None;
                local.out_angle = None;
                local.in_angle = None;
                let mut k = skip_ws(s, e);
                if s[k..].starts_with('[') {
                    let close = matching(s, k)?;
                    let opts = s[k + 1..close - 1].to_string();
                    self.apply_opts(&mut local, &opts);
                    k = close;
                }
                let (deferred, k) = self.deferred_nodes(s, k)?;
                let (p, node, k2) = self.coordinate(pb, ps, s, k)?;
                self.bend_or_line(pb, ps, &local, p, node);
                self.place_deferred(pb, ps, deferred);
                Some(k2)
            }
            _ => {
                self.warn(format!("path operation `{word}` is not supported; rest of path skipped"));
                None
            }
        }
    }

    /// Nodes written between an operation and its target.
    fn deferred_nodes(&mut self, s: &str, mut k: usize) -> Option<(Vec<NodeSpec>, usize)> {
        let mut out = Vec::new();
        loop {
            k = skip_ws(s, k);
            if is_word_at(s, k, "node") || is_word_at(s, k, "coordinate") {
                let dummy_st = St::new(self.base_font);
                let mut dummy_pb = Pb {
                    segs: Vec::new(),
                    cur: v(0.0, 0.0),
                    rel: v(0.0, 0.0),
                    have_cur: false,
                    cur_node: None,
                    last: Last::None,
                    node_raws: Vec::new(),
                };
                let (spec, e) = self.node_spec(s, k, &dummy_st, &mut dummy_pb)?;
                out.push(spec);
                k = e;
            } else {
                return Some((out, k));
            }
        }
    }

    fn place_deferred(&mut self, pb: &mut Pb, ps: &St, deferred: Vec<NodeSpec>) {
        for spec in deferred {
            let (pos, ang) = self.path_position(pb, ps_pos(&spec, ps, &self.styles), 0.5);
            let raws = self.place_node(&spec, ps, pos, ang);
            pb.node_raws.extend(raws);
        }
    }

    /// Parses `node[opts] (name) at (p) {text}` or `coordinate (name) ...`.
    fn node_spec(&mut self, s: &str, i: usize, ps: &St, pb: &mut Pb) -> Option<(NodeSpec, usize)> {
        let coordinate = is_word_at(s, i, "coordinate");
        let mut k = i + if coordinate { "coordinate".len() } else { "node".len() };
        let mut spec = NodeSpec {
            opts: String::new(),
            name: None,
            at: None,
            text: None,
            coordinate,
        };
        loop {
            k = skip_ws(s, k);
            if k >= s.len() {
                break;
            }
            let c = s.as_bytes()[k];
            if c == b'[' {
                let e = matching(s, k)?;
                if !spec.opts.is_empty() {
                    spec.opts.push(',');
                }
                spec.opts.push_str(&s[k + 1..e - 1]);
                k = e;
            } else if c == b'(' {
                let e = matching(s, k)?;
                spec.name = Some(s[k + 1..e - 1].trim().to_string());
                k = e;
            } else if is_word_at(s, k, "at") {
                let saved_rel = pb.rel;
                let (p, _, e) = self.coordinate(pb, ps, s, k + 2)?;
                pb.rel = saved_rel;
                spec.at = Some(p);
                k = e;
            } else if c == b'{' && !coordinate {
                let e = matching(s, k)?;
                spec.text = Some(s[k + 1..e - 1].to_string());
                k = e;
                break;
            } else {
                break;
            }
        }
        if !coordinate && spec.text.is_none() {
            self.warn("node without a `{text}` argument; skipped");
            return None;
        }
        Some((spec, k))
    }

    fn path_position(&self, pb: &Pb, pos: Option<f64>, default: f64) -> (V, Option<f64>) {
        let t = pos.unwrap_or(default);
        match pb.last {
            Last::None => (pb.cur, None),
            Last::Line(a, b) => (add(a, mul(sub(b, a), t)), Some(sub(b, a).y.atan2(sub(b, a).x))),
            Last::Corner(a, c, b) => {
                if t <= 0.5 {
                    (add(a, mul(sub(c, a), 2.0 * t)), Some(sub(c, a).y.atan2(sub(c, a).x)))
                } else {
                    (add(c, mul(sub(b, c), 2.0 * t - 1.0)), Some(sub(b, c).y.atan2(sub(b, c).x)))
                }
            }
            Last::Curve(a, c1, c2, b) => {
                let mt = 1.0 - t;
                let p = add(add(mul(a, mt * mt * mt), mul(c1, 3.0 * mt * mt * t)), add(mul(c2, 3.0 * mt * t * t), mul(b, t * t * t)));
                let d = add(add(mul(sub(c1, a), 3.0 * mt * mt), mul(sub(c2, c1), 6.0 * mt * t)), mul(sub(b, c2), 3.0 * t * t));
                (p, Some(d.y.atan2(d.x)))
            }
        }
    }

    fn move_to(&mut self, pb: &mut Pb, p: V, node: Option<String>) {
        pb.segs.push((Seg::M(p), None));
        pb.cur = p;
        pb.have_cur = true;
        pb.cur_node = node;
        pb.last = Last::None;
    }

    /// Replaces a trailing move-to or starts a new subpath at `a`.
    fn restart_at(&mut self, pb: &mut Pb, a: V) {
        if let Some((Seg::M(_), _)) = pb.segs.last() {
            pb.segs.pop();
        }
        pb.segs.push((Seg::M(a), None));
    }

    fn segment_start(&mut self, pb: &mut Pb, toward: V) -> V {
        if !pb.have_cur {
            let c = pb.cur;
            self.move_to(pb, c, None);
        }
        match pb.cur_node.take() {
            Some(n) => {
                let a = self.nodes.get(&n).map(|g| g.border_toward(toward)).unwrap_or(pb.cur);
                self.restart_at(pb, a);
                a
            }
            None => pb.cur,
        }
    }

    fn line_to(&mut self, pb: &mut Pb, ps: &St, p: V, node: Option<String>) {
        let a = self.segment_start(pb, p);
        let from_center = pb.cur;
        let b = match &node {
            Some(n) => self.nodes.get(n).map(|g| g.border_toward(from_center)).unwrap_or(p),
            None => p,
        };
        pb.segs.push((Seg::L(b), ps.rounded));
        pb.last = Last::Line(a, b);
        pb.cur = p;
        pb.cur_node = node;
    }

    fn corner_to(&mut self, pb: &mut Pb, ps: &St, corner: V, p: V, node: Option<String>) {
        let a = self.segment_start(pb, corner);
        let b = match &node {
            Some(n) => self.nodes.get(n).map(|g| g.border_toward(corner)).unwrap_or(p),
            None => p,
        };
        pb.segs.push((Seg::L(corner), ps.rounded));
        pb.segs.push((Seg::L(b), ps.rounded));
        pb.last = Last::Corner(a, corner, b);
        pb.cur = p;
        pb.cur_node = node;
    }

    fn curve_to(&mut self, pb: &mut Pb, ps: &St, c1: V, c2: V, p: V, node: Option<String>) {
        let a = self.segment_start(pb, c1);
        let b = match &node {
            Some(n) => self.nodes.get(n).map(|g| g.border_toward(c2)).unwrap_or(p),
            None => p,
        };
        pb.segs.push((Seg::C(c1, c2, b), ps.rounded));
        pb.last = Last::Curve(a, c1, c2, b);
        pb.cur = p;
        pb.cur_node = node;
    }

    fn bend_or_line(&mut self, pb: &mut Pb, ps: &St, local: &St, p: V, node: Option<String>) {
        if local.bend.is_none() && local.out_angle.is_none() && local.in_angle.is_none() {
            self.line_to(pb, ps, p, node);
            return;
        }
        let a0 = pb.cur;
        let d = sub(p, a0);
        let ang = d.y.atan2(d.x).to_degrees();
        let bend = local.bend.unwrap_or(0.0);
        let out = local.out_angle.map(|o| if local.bend.is_some() { ang + o } else { o }).unwrap_or(ang + bend);
        let inn = local.in_angle.map(|i| if local.bend.is_some() { ang + 180.0 + i } else { i }).unwrap_or(ang + 180.0 - bend);
        if !pb.have_cur {
            self.move_to(pb, a0, None);
        }
        let a = match pb.cur_node.take() {
            Some(n) => {
                let q = self.nodes.get(&n).map(|g| g.angle_anchor(out)).unwrap_or(a0);
                self.restart_at(pb, q);
                q
            }
            None => a0,
        };
        let b = match &node {
            Some(n) => self.nodes.get(n).map(|g| g.angle_anchor(inn)).unwrap_or(p),
            None => p,
        };
        let dist = pgf_veclen(sub(b, a)) * TO_CONTROL * local.looseness;
        let c1 = add(a, mul(v(rad(out).cos(), rad(out).sin()), dist));
        let c2 = add(b, mul(v(rad(inn).cos(), rad(inn).sin()), dist));
        pb.segs.push((Seg::C(c1, c2, b), ps.rounded));
        pb.last = Last::Curve(a, c1, c2, b);
        pb.cur = p;
        pb.cur_node = node;
    }

    fn close(&mut self, pb: &mut Pb, ps: &St) {
        pb.segs.push((Seg::Z, ps.rounded));
        // The current point returns to the subpath start.
        if let Some(start) = pb.segs.iter().rev().find_map(|(s, _)| if let Seg::M(p) = s { Some(*p) } else { None }) {
            pb.last = Last::Line(pb.cur, start);
            pb.cur = start;
        }
        pb.cur_node = None;
    }

    fn rectangle(&mut self, pb: &mut Pb, ps: &St, from: V, to: V) {
        pb.cur_node = None;
        let inv = ps.tf.invert().unwrap_or(Transform::IDENTITY);
        let (ua, ub) = (inv.apply(from), inv.apply(to));
        let pts = [v(ua.x, ua.y), v(ub.x, ua.y), v(ub.x, ub.y), v(ua.x, ub.y)].map(|q| ps.tf.apply(q));
        self.restart_at(pb, pts[0]);
        for q in &pts[1..] {
            pb.segs.push((Seg::L(*q), ps.rounded));
        }
        pb.segs.push((Seg::Z, ps.rounded));
        pb.segs.push((Seg::M(to), None));
        pb.cur = to;
        pb.last = Last::None;
    }

    fn ellipse(&mut self, pb: &mut Pb, c: V, u: V, w: V) {
        pb.segs.push((Seg::M(add(c, u)), None));
        let pt = |t: f64| add(c, add(mul(u, t.cos()), mul(w, t.sin())));
        let dp = |t: f64| add(mul(u, -t.sin()), mul(w, t.cos()));
        let k = 4.0 / 3.0 * (std::f64::consts::FRAC_PI_2 / 4.0).tan();
        for q in 0..4 {
            let t0 = q as f64 * std::f64::consts::FRAC_PI_2;
            let t1 = t0 + std::f64::consts::FRAC_PI_2;
            pb.segs.push((Seg::C(add(pt(t0), mul(dp(t0), k)), sub(pt(t1), mul(dp(t1), k)), pt(t1)), None));
        }
        pb.segs.push((Seg::Z, None));
        pb.segs.push((Seg::M(c), None));
        pb.cur = c;
        pb.last = Last::None;
    }

    fn arc(&mut self, pb: &mut Pb, u: V, w: V, sa: f64, ea: f64) {
        let p0 = pb.cur;
        let (s, e) = (rad(sa), rad(ea));
        let c = sub(sub(p0, mul(u, s.cos())), mul(w, s.sin()));
        let pt = |t: f64| add(c, add(mul(u, t.cos()), mul(w, t.sin())));
        let dp = |t: f64| add(mul(u, -t.sin()), mul(w, t.cos()));
        // PGF splits arcs into quarter turns from the start angle plus a
        // remainder; the control points (which count for the bounding box)
        // then match its output.
        let total = e - s;
        let quarter = std::f64::consts::FRAC_PI_2.copysign(total);
        let mut last = Last::None;
        let mut t0 = s;
        let mut guard = 0;
        while (e - t0).abs() > 1e-9 && guard < 64 {
            guard += 1;
            let t1 = if (e - t0).abs() > quarter.abs() + 1e-9 { t0 + quarter } else { e };
            let k = 4.0 / 3.0 * ((t1 - t0) / 4.0).tan();
            let (a, c1, c2, b) = (pt(t0), add(pt(t0), mul(dp(t0), k)), sub(pt(t1), mul(dp(t1), k)), pt(t1));
            pb.segs.push((Seg::C(c1, c2, b), None));
            last = Last::Curve(a, c1, c2, b);
            t0 = t1;
        }
        pb.cur = pt(e);
        pb.rel = pb.cur;
        pb.cur_node = None;
        pb.last = last;
    }

    fn grid(&mut self, pb: &mut Pb, ps: &St, local: &St, from: V, to: V) {
        let inv = ps.tf.invert().unwrap_or(Transform::IDENTITY);
        let (ua, ub) = (inv.apply(from), inv.apply(to));
        let (x0, x1) = (ua.x.min(ub.x), ua.x.max(ub.x));
        let (y0, y1) = (ua.y.min(ub.y), ua.y.max(ub.y));
        let sx = local.xlen(local.xstep);
        let sy = local.ylen(local.ystep);
        if sx <= 1e-6 || sy <= 1e-6 || (x1 - x0) / sx > 10_000.0 || (y1 - y0) / sy > 10_000.0 {
            self.warn("grid step is too small; grid skipped");
            return;
        }
        // PGF computes grid lines in scaled points with TeX's integer
        // division; emulate it so lines at (or rounding-close to) the ends
        // appear or vanish exactly as in its output.
        let (xa, xb, xs) = (tex_sp(x0), tex_sp(x1), tex_sp(sx));
        let (ya, yb, ys) = (tex_sp(y0), tex_sp(y1), tex_sp(sy));
        let sp = |s: i64| s as f64 / 65536.0;
        for y in grid_lines(ya, yb, ys) {
            pb.segs.push((Seg::M(ps.tf.apply(v(sp(xa), sp(y)))), None));
            pb.segs.push((Seg::L(ps.tf.apply(v(sp(xb), sp(y)))), None));
        }
        for x in grid_lines(xa, xb, xs) {
            pb.segs.push((Seg::M(ps.tf.apply(v(sp(x), sp(ya)))), None));
            pb.segs.push((Seg::L(ps.tf.apply(v(sp(x), sp(yb)))), None));
        }
        pb.segs.push((Seg::M(to), None));
        pb.cur = to;
        pb.cur_node = None;
        pb.last = Last::None;
    }

    // ------------------------------------------------------------------ nodes

    fn place_node(&mut self, spec: &NodeSpec, base: &St, pos: V, slope: Option<f64>) -> Vec<Raw> {
        let mut ns = base.clone();
        ns.do_draw = false;
        ns.do_fill = false;
        ns.do_clip = false;
        ns.start_tip = None;
        ns.end_tip = None;
        ns.anchor = None;
        ns.place_shift = v(0.0, 0.0);
        ns.pos = None;
        ns.shape = Shape::Rectangle;
        ns.name = None;
        ns.min_w = 0.0;
        ns.min_h = 0.0;
        ns.inner_xsep = None;
        ns.inner_ysep = None;
        ns.outer_sep = None;
        ns.sloped = false;
        ns.transform_shape = false;
        let outer_tf = ns.tf;
        ns.tf = Transform::IDENTITY;
        if self.styles.contains_key("every node") {
            self.apply_opts(&mut ns, "every node");
        }
        let opts = spec.opts.clone();
        self.apply_opts(&mut ns, &opts);
        let shape_style = match ns.shape {
            Shape::Circle => "every circle node",
            Shape::Rectangle => "every rectangle node",
            _ => "",
        };
        if self.styles.contains_key(shape_style) {
            self.apply_opts(&mut ns, shape_style);
        }
        if spec.coordinate {
            ns.shape = Shape::Coordinate;
        }
        let node_tf = ns.tf;
        ns.tf = outer_tf;

        let raw_text = spec.text.clone().unwrap_or_default();
        let (text, bold, italic, size) = self.node_text(&raw_text, &ns);
        let style = TextStyle {
            size_pt: size,
            bold,
            italic,
        };
        let metrics = if text.is_empty() { Default::default() } else { self.measurer.measure(&text, &style) };
        let (w, h, d) = (metrics.width_pt, metrics.height_pt, metrics.depth_pt);
        let em = ns.font_size;
        let isx = ns.inner_xsep.unwrap_or(0.3333 * em);
        let isy = ns.inner_ysep.unwrap_or(0.3333 * em);
        let outer = ns.outer_sep.unwrap_or(0.5 * ns.lw);
        let mut g = NodeGeom {
            shape: ns.shape,
            m: Transform::IDENTITY,
            xmin: -isx,
            xmax: w + isx,
            ymin: -d - isy,
            ymax: h + isy,
            c: v(w / 2.0, (h - d) / 2.0),
            rx: 0.0,
            ry: 0.0,
            outer,
            em,
        };
        match ns.shape {
            Shape::Rectangle => {
                if g.xmax - g.xmin < ns.min_w {
                    let grow = (ns.min_w - (g.xmax - g.xmin)) / 2.0;
                    g.xmin -= grow;
                    g.xmax += grow;
                }
                if g.ymax - g.ymin < ns.min_h {
                    let grow = (ns.min_h - (g.ymax - g.ymin)) / 2.0;
                    g.ymin -= grow;
                    g.ymax += grow;
                }
                g.c = v((g.xmin + g.xmax) / 2.0, (g.ymin + g.ymax) / 2.0);
            }
            Shape::Circle => {
                let r = (w / 2.0 + isx).hypot((h + d) / 2.0 + isy).max(ns.min_w / 2.0).max(ns.min_h / 2.0);
                g.rx = r;
                g.ry = r;
                g.xmin = g.c.x - r;
                g.xmax = g.c.x + r;
                g.ymin = g.c.y - r;
                g.ymax = g.c.y + r;
            }
            Shape::Ellipse => {
                let s2 = std::f64::consts::SQRT_2;
                g.rx = ((w / 2.0 + isx) * s2).max(ns.min_w / 2.0);
                g.ry = (((h + d) / 2.0 + isy) * s2).max(ns.min_h / 2.0);
                g.xmin = g.c.x - g.rx;
                g.xmax = g.c.x + g.rx;
                g.ymin = g.c.y - g.ry;
                g.ymax = g.c.y + g.ry;
            }
            Shape::Coordinate => {
                g.c = v(0.0, 0.0);
                g.xmin = 0.0;
                g.xmax = 0.0;
                g.ymin = 0.0;
                g.ymax = 0.0;
                g.outer = 0.0;
            }
        }
        let anchor_name = ns.anchor.clone().unwrap_or_else(|| "center".into());
        let a = match g.local_anchor(&anchor_name) {
            Some(a) => a,
            None => {
                self.warn(format!("node anchor `{anchor_name}` is not supported; using center"));
                g.c
            }
        };
        let mut m = Transform::translate(-a.x, -a.y);
        if ns.sloped
            && let Some(mut ang) = slope {
                // Keep text upright, as TikZ does.
                if ang.to_degrees() > 90.0 + 1e-9 || ang.to_degrees() < -90.0 - 1e-9 {
                    ang += std::f64::consts::PI;
                }
                m = m.then(&Transform::rotate(ang));
            }
        m = m.then(&node_tf);
        if ns.transform_shape {
            m = m.then(&linear(&outer_tf));
        }
        m = m.then(&Transform::translate(pos.x + ns.place_shift.x, pos.y + ns.place_shift.y));
        g.m = m;

        let mut raws = Vec::new();
        if ns.shape != Shape::Coordinate {
            let segs: Vec<(Seg, Option<f64>)> = match ns.shape {
                Shape::Rectangle => {
                    let r = ns.rounded;
                    vec![
                        (Seg::M(v(g.xmin, g.ymin)), None),
                        (Seg::L(v(g.xmax, g.ymin)), r),
                        (Seg::L(v(g.xmax, g.ymax)), r),
                        (Seg::L(v(g.xmin, g.ymax)), r),
                        (Seg::Z, r),
                    ]
                }
                _ => {
                    let mut pb = Pb {
                        segs: Vec::new(),
                        cur: g.c,
                        rel: g.c,
                        have_cur: true,
                        cur_node: None,
                        last: Last::None,
                        node_raws: Vec::new(),
                    };
                    self.ellipse(&mut pb, g.c, v(g.rx, 0.0), v(0.0, g.ry));
                    pb.segs.pop();
                    pb.segs
                }
            };
            let segs: Vec<(Seg, Option<f64>)> = segs.into_iter().map(|(sg, r)| (map_seg(sg, &m), r)).collect();
            let segs = round_corners(&segs);
            // PGF's picture size grows by the node's shape (without outer
            // sep), plus half the line width when the shape is drawn.
            let h = if ns.do_draw { ns.lw / 2.0 } else { 0.0 };
            for q in [v(g.xmin - h, g.ymin - h), v(g.xmax + h, g.ymin - h), v(g.xmax + h, g.ymax + h), v(g.xmin - h, g.ymax + h)] {
                self.bbox_add(m.apply(q));
            }
            let path = to_path(&segs);
            if ns.do_fill {
                raws.push(Raw::Fill {
                    path: path.clone(),
                    even_odd: ns.even_odd,
                    paint: ns.fill_paint(),
                    pattern: ns.fill_pattern(),
                });
            }
            if ns.do_draw {
                raws.push(Raw::Stroke {
                    path,
                    style: ns.stroke_style(),
                    paint: ns.stroke_paint(),
                });
            }
            if !text.is_empty() {
                let color = ns.text_color.unwrap_or(ns.color);
                let alpha = ns.text_opacity.unwrap_or(ns.fill_opacity);
                raws.push(Raw::Text {
                    text,
                    style,
                    m,
                    paint: Paint::new(color, alpha),
                    span: self.span,
                });
            }
        } else {
            self.bbox_add(m.apply(v(0.0, 0.0)));
        }
        let name = spec.name.clone().or(ns.name.clone());
        if let Some(n) = name {
            self.nodes.insert(n, g);
        }
        raws
    }

    /// Plain text for a node plus its font. Simple markup is understood;
    /// anything else is reported.
    fn node_text(&mut self, raw: &str, ns: &St) -> (String, bool, bool, f64) {
        let mut bold = ns.bold;
        let mut italic = ns.italic;
        let mut size = ns.font_size;
        let mut out = String::new();
        let s = raw.trim();
        let b = s.as_bytes();
        let mut i = 0;
        let mut math_warned = false;
        while i < b.len() {
            match b[i] {
                b'\\' => {
                    if s[i..].starts_with("\\\\") {
                        self.warn("line breaks in node text need `align`, which is not supported; joined with a space");
                        out.push(' ');
                        i += 2;
                        continue;
                    }
                    match tx::control_word(s, i) {
                        Some((nm, e)) => {
                            match nm {
                                "textbf" | "bfseries" => bold = true,
                                "textit" | "emph" | "itshape" => italic = true,
                                "textrm" | "textnormal" | "rmfamily" => {}
                                _ => {
                                    if let Some(sz) = font_size_for(nm, ns.base_font) {
                                        size = sz;
                                    } else {
                                        self.warn(format!("`\\{nm}` in node text is not supported; omitted"));
                                    }
                                }
                            }
                            i = e;
                        }
                        None => {
                            // Control symbols like \% \& \$ print the character.
                            if let Some(ch) = s[i + 1..].chars().next() {
                                out.push(ch);
                                i += 1 + ch.len_utf8();
                            } else {
                                i += 1;
                            }
                        }
                    }
                }
                b'$' => {
                    if !math_warned {
                        self.warn("math in TikZ node text is set as italic text (math layout is not wired for nodes)");
                        math_warned = true;
                    }
                    italic = true;
                    i += 1;
                }
                b'{' | b'}' => i += 1,
                b'~' => {
                    out.push('\u{a0}');
                    i += 1;
                }
                _ => {
                    let ch = s[i..].chars().next().unwrap_or(' ');
                    if ch.is_whitespace() {
                        if !out.ends_with(' ') && !out.is_empty() {
                            out.push(' ');
                        }
                    } else {
                        out.push(ch);
                    }
                    i += ch.len_utf8();
                }
            }
        }
        (out.trim_end().to_string(), bold, italic, size)
    }

    // ----------------------------------------------------------------- output

    fn finish_path(&mut self, pb: Pb, ps: &St) -> bool {
        let segs = round_corners(&pb.segs);
        // PGF protocols the corner points before rounding replaces them.
        for (sg, _) in &pb.segs {
            match *sg {
                Seg::M(p) | Seg::L(p) => self.bbox_add(p),
                Seg::C(a, b, c) => {
                    self.bbox_add(a);
                    self.bbox_add(b);
                    self.bbox_add(c);
                }
                Seg::Z => {}
            }
        }
        let drawable = segs.iter().any(|(s, _)| !matches!(s, Seg::M(_)));
        let mut clip = false;
        if drawable {
            // Stroked paths grow the picture by half the line width.
            if ps.do_draw
                && let Some([x0, y0, x1, y1]) = path_extent(&pb.segs) {
                    let h = ps.lw / 2.0;
                    self.bbox_add(v(x0 - h, y0 - h));
                    self.bbox_add(v(x1 + h, y1 + h));
                }
            if ps.bbox_only
                && let Some([x0, y0, x1, y1]) = path_extent(&segs) {
                    self.bbox = Some([x0, y0, x1, y1]);
                    self.bbox_locked = true;
                }
            if ps.do_clip {
                self.raws.push(Raw::ClipBegin {
                    path: to_path(&segs),
                    even_odd: ps.even_odd,
                });
                clip = true;
            }
            if ps.do_fill {
                self.raws.push(Raw::Fill {
                    path: to_path(&segs),
                    even_odd: ps.even_odd,
                    paint: ps.fill_paint(),
                    pattern: ps.fill_pattern(),
                });
            }
            if ps.do_draw {
                let mut segs = segs.clone();
                let open = !matches!(segs.iter().rev().find(|(s, _)| !matches!(s, Seg::M(_))), Some((Seg::Z, _)));
                let mut tips = Vec::new();
                if open {
                    if let Some(t) = ps.end_tip
                        && let Some((o, d)) = shorten_end(&mut segs, tip_extend(t, ps.lw)) {
                            tips.extend(self.tip(t, o, d, ps));
                        }
                    if let Some(t) = ps.start_tip
                        && let Some((o, d)) = shorten_start(&mut segs, tip_extend(t, ps.lw)) {
                            tips.extend(self.tip(t, o, d, ps));
                        }
                }
                self.raws.push(Raw::Stroke {
                    path: to_path(&segs),
                    style: ps.stroke_style(),
                    paint: ps.stroke_paint(),
                });
                self.raws.extend(tips);
            }
        }
        self.raws.extend(pb.node_raws);
        clip
    }

    fn tip(&mut self, tip: Tip, o: V, d: V, ps: &St) -> Vec<Raw> {
        let lw = ps.lw;
        let a = 0.28 + 0.3 * lw;
        let perp = v(-d.y, d.x);
        let at = |x: f64, y: f64| add(o, add(mul(d, x * a), mul(perp, y * a)));
        let mut p = Path::new();
        let raw = match tip {
            Tip::To => {
                p.move_to(at(-3.0, 4.0))
                    .cubic_to(at(-2.75, 2.5), at(0.0, 0.25), at(0.75, 0.0))
                    .cubic_to(at(0.0, -0.25), at(-2.75, -2.5), at(-3.0, -4.0));
                Raw::Stroke {
                    path: p,
                    style: StrokeStyle {
                        width: 0.8 * lw,
                        cap: LineCap::Round,
                        join: LineJoin::Round,
                        miter_limit: ps.miter,
                        dash: None,
                    },
                    paint: ps.stroke_paint(),
                }
            }
            Tip::Stealth => {
                p.move_to(at(5.0, 0.0)).line_to(at(-3.0, 4.0)).line_to(at(0.0, 0.0)).line_to(at(-3.0, -4.0)).close();
                Raw::Fill {
                    path: p,
                    even_odd: false,
                    paint: ps.stroke_paint(),
                    pattern: None,
                }
            }
            Tip::Latex => {
                p.move_to(at(9.0, 0.0))
                    .cubic_to(at(6.3333, 0.5), at(2.0, 2.0), at(-1.0, 3.75))
                    .line_to(at(-1.0, -3.75))
                    .cubic_to(at(2.0, -2.0), at(6.3333, -0.5), at(9.0, 0.0))
                    .close();
                Raw::Fill {
                    path: p,
                    even_odd: false,
                    paint: ps.stroke_paint(),
                    pattern: None,
                }
            }
        };
        vec![raw]
    }

    pub(crate) fn finish(mut self) -> Picture {
        let [x0, _y0, x1, y1] = self.bbox.unwrap_or([0.0; 4]);
        let y0 = self.bbox.map(|b| b[1]).unwrap_or(0.0);
        let k = BP_PER_PT;
        let f = Transform::new(k, 0.0, 0.0, -k, -x0 * k, y1 * k);
        let text_space = Transform::new(1.0 / k, 0.0, 0.0, -1.0 / k, 0.0, 0.0);
        let mut stack: Vec<(Vec<Item>, Option<Clip>)> = vec![(Vec::new(), None)];
        let mut texts = Vec::new();
        let mut id = 0u64;
        let raws = std::mem::take(&mut self.raws);
        for raw in raws {
            id += 1;
            match raw {
                Raw::Fill { path, even_odd, paint, pattern } => stack.last_mut().expect("stack").0.push(Item::PathFill(PathFill {
                    id: ItemId(id),
                    path: path.transformed(&f),
                    rule: if even_odd { FillRule::EvenOdd } else { FillRule::NonZero },
                    paint,
                    pattern,
                    source: None,
                })),
                Raw::Stroke { path, mut style, paint } => {
                    style.width *= k;
                    if let Some(dash) = &mut style.dash {
                        for x in &mut dash.array {
                            *x *= k;
                        }
                        dash.phase *= k;
                    }
                    stack.last_mut().expect("stack").0.push(Item::PathStroke(PathStroke {
                        id: ItemId(id),
                        path: path.transformed(&f),
                        style,
                        paint,
                        source: None,
                    }))
                }
                Raw::ClipBegin { path, even_odd } => stack.push((
                    Vec::new(),
                    Some(Clip::Path {
                        path: path.transformed(&f),
                        rule: if even_odd { FillRule::EvenOdd } else { FillRule::NonZero },
                    }),
                )),
                Raw::ClipEnd => {
                    if stack.len() > 1 {
                        let (items, clip) = stack.pop().expect("stack");
                        let mut g = Group::new(ItemId(id));
                        g.clip = clip;
                        g.items = items;
                        stack.last_mut().expect("stack").0.push(Item::Group(g));
                    }
                }
                Raw::Text { text, style, m, paint, span } => texts.push(PictureText {
                    text,
                    style,
                    transform: text_space.then(&m).then(&f),
                    paint,
                    after_item: stack[0].0.len(),
                    source: span,
                }),
            }
        }
        while stack.len() > 1 {
            let (items, clip) = stack.pop().expect("stack");
            id += 1;
            let mut g = Group::new(ItemId(id));
            g.clip = clip;
            g.items = items;
            stack.last_mut().expect("stack").0.push(Item::Group(g));
        }
        Picture {
            width_bp: (x1 - x0).max(0.0) * k,
            height_bp: (y1 - y0).max(0.0) * k,
            items: stack.pop().map(|s| s.0).unwrap_or_default(),
            texts,
            diagnostics: self.diags,
        }
    }
}

enum CoordKind {
    User(V),
    Canvas(V, Option<String>),
}

/// The `to` path's distance estimate (`\tikz@to@compute@distance@main`):
/// the larger normalised component, truncated to 1/255, divides the
/// matching absolute component in scaled points. It is up to ~0.4 % short
/// of the true length, and PGF's control points inherit that.
fn pgf_veclen(d: V) -> f64 {
    let (xa, ya) = (d.x.abs(), d.y.abs());
    let l = xa.hypot(ya);
    if l < 1e-9 {
        return 0.0;
    }
    let (nx, ny) = ((xa / l * 65536.0).round() as i64, (ya / l * 65536.0).round() as i64);
    let (comp, n) = if nx > ny { (xa, nx) } else { (ya, ny) };
    let count = n / 255;
    if count == 0 {
        return xa;
    }
    let comp_sp = (comp * 65536.0).round() as i64;
    (16 * ((16 * comp_sp) / count)) as f64 / 65536.0
}

/// TeX scaled points for a length. Whole or decimal multiples of a
/// centimetre are converted the way TeX does (`1cm` = 1864679sp, a decimal
/// factor truncates), since TikZ coordinates are multiples of the x/y
/// vectors; other lengths are rounded.
fn tex_sp(pt: f64) -> i64 {
    const CM_SP: f64 = 1_864_679.0;
    let n = pt / PT_PER_CM;
    let scaled = n * 100_000.0;
    if (scaled - scaled.round()).abs() < 1e-3 {
        let n = scaled.round() / 100_000.0;
        let v = (n.abs() * CM_SP + 1e-6).trunc();
        (v as i64) * if n < 0.0 { -1 } else { 1 }
    } else {
        (pt * 65536.0).round() as i64
    }
}

/// PGF's grid loop (`\pgfpathgrid`): the first multiple of `step` at or
/// after `a` (integer division truncates), lines while below `b`, and one
/// last line when within 0.01pt of `b`.
fn grid_lines(a: i64, b: i64, step: i64) -> Vec<i64> {
    let mut out = Vec::new();
    if step <= 655 {
        return out;
    }
    let mut t = (a / step) * step;
    if t < a {
        t += step;
    }
    loop {
        out.push(t);
        t += step;
        if t >= b || out.len() > 10_000 {
            break;
        }
    }
    t -= 655;
    if t < b {
        out.push(t);
    }
    out
}

fn single_char(s: &str) -> Option<char> {
    let mut it = s.chars();
    let c = it.next()?;
    if it.next().is_none() { Some(c) } else { None }
}

fn spec_pos(spec: &NodeSpec) -> Option<f64> {
    // Positions given in the node's own options.
    for entry in split_top(&spec.opts, b',') {
        let e = entry.trim();
        match e {
            "midway" => return Some(0.5),
            "near start" => return Some(0.25),
            "near end" => return Some(0.75),
            "very near start" => return Some(0.125),
            "very near end" => return Some(0.875),
            "at start" => return Some(0.0),
            "at end" => return Some(1.0),
            _ => {
                if let Some(v) = e.strip_prefix("pos")
                    && let Some(v) = v.trim_start().strip_prefix('=') {
                        return expr::eval(v, 10.0).ok().map(|x| x.v);
                    }
            }
        }
    }
    None
}

fn ps_pos(spec: &NodeSpec, _ps: &St, _styles: &HashMap<String, (String, Option<String>)>) -> Option<f64> {
    spec_pos(spec)
}

fn parse_tip(s: &str) -> Option<Tip> {
    match s.trim() {
        "to" | "To" | ">" => Some(Tip::To),
        "stealth" | "Stealth" => Some(Tip::Stealth),
        "latex" | "Latex" => Some(Tip::Latex),
        _ => None,
    }
}

fn tip_extend(tip: Tip, lw: f64) -> f64 {
    let a = 0.28 + 0.3 * lw;
    match tip {
        Tip::To => 0.21 + 0.625 * lw,
        Tip::Stealth => 5.0 * a,
        Tip::Latex => 9.0 * a,
    }
}

fn map_seg(s: Seg, m: &Transform) -> Seg {
    match s {
        Seg::M(p) => Seg::M(m.apply(p)),
        Seg::L(p) => Seg::L(m.apply(p)),
        Seg::C(a, b, c) => Seg::C(m.apply(a), m.apply(b), m.apply(c)),
        Seg::Z => Seg::Z,
    }
}

fn to_path(segs: &[(Seg, Option<f64>)]) -> Path {
    let mut p = Path::new();
    for (s, _) in segs {
        match *s {
            Seg::M(a) => {
                p.move_to(a);
            }
            Seg::L(a) => {
                p.line_to(a);
            }
            Seg::C(a, b, c) => {
                p.cubic_to(a, b, c);
            }
            Seg::Z => {
                p.close();
            }
        }
    }
    // Drop move-tos that start nothing (PDF ignores them, but keep the item tidy).
    let cmds = p.commands().to_vec();
    let mut out = Vec::with_capacity(cmds.len());
    for (i, c) in cmds.iter().enumerate() {
        if matches!(c, crate::path::PathCommand::MoveTo(_))
            && cmds.get(i + 1).is_none_or(|n| matches!(n, crate::path::PathCommand::MoveTo(_)))
        {
            continue;
        }
        out.push(*c);
    }
    Path::from_commands(out)
}

fn path_extent(segs: &[(Seg, Option<f64>)]) -> Option<[f64; 4]> {
    let mut bb: Option<[f64; 4]> = None;
    let mut addp = |p: V| {
        bb = Some(match bb {
            None => [p.x, p.y, p.x, p.y],
            Some([a, b, c, d]) => [a.min(p.x), b.min(p.y), c.max(p.x), d.max(p.y)],
        })
    };
    for (s, _) in segs {
        match *s {
            Seg::M(p) | Seg::L(p) => addp(p),
            Seg::C(a, b, c) => {
                addp(a);
                addp(b);
                addp(c);
            }
            Seg::Z => {}
        }
    }
    bb
}

/// Shortens the last drawn segment by `ext`; returns the tip origin and the
/// unit direction the tip points.
fn shorten_end(segs: &mut [(Seg, Option<f64>)], ext: f64) -> Option<(V, V)> {
    let idx = segs.iter().rposition(|(s, _)| !matches!(s, Seg::M(_)))?;
    let prev = point_before(segs, idx)?;
    match segs[idx].0 {
        Seg::L(b) => {
            let d = unit(sub(b, prev))?;
            let nb = sub(b, mul(d, ext));
            segs[idx].0 = Seg::L(nb);
            Some((nb, d))
        }
        Seg::C(c1, c2, b) => {
            let d = unit(sub(b, c2)).or_else(|| unit(sub(b, c1))).or_else(|| unit(sub(b, prev)))?;
            let shift = mul(d, ext);
            let nb = sub(b, shift);
            segs[idx].0 = Seg::C(c1, sub(c2, shift), nb);
            Some((nb, d))
        }
        _ => None,
    }
}

fn shorten_start(segs: &mut [(Seg, Option<f64>)], ext: f64) -> Option<(V, V)> {
    let first_draw = segs.iter().position(|(s, _)| !matches!(s, Seg::M(_)))?;
    let m_idx = first_draw.checked_sub(1)?;
    let Seg::M(a) = segs[m_idx].0 else { return None };
    match segs[first_draw].0 {
        Seg::L(b) => {
            let d = unit(sub(a, b))?;
            let na = sub(a, mul(d, ext));
            segs[m_idx].0 = Seg::M(na);
            Some((na, d))
        }
        Seg::C(c1, c2, b) => {
            let d = unit(sub(a, c1)).or_else(|| unit(sub(a, c2))).or_else(|| unit(sub(a, b)))?;
            let shift = mul(d, ext);
            let na = sub(a, shift);
            segs[m_idx].0 = Seg::M(na);
            segs[first_draw].0 = Seg::C(sub(c1, shift), c2, b);
            Some((na, d))
        }
        _ => None,
    }
}

fn point_before(segs: &[(Seg, Option<f64>)], idx: usize) -> Option<V> {
    let mut start = None;
    let mut cur = None;
    for (s, _) in &segs[..idx] {
        match *s {
            Seg::M(p) => {
                start = Some(p);
                cur = Some(p);
            }
            Seg::L(p) | Seg::C(_, _, p) => cur = Some(p),
            Seg::Z => cur = start,
        }
    }
    cur
}

/// Replaces corners between straight segments by curves when the outgoing
/// segment was added under `rounded corners`.
fn round_corners(segs: &[(Seg, Option<f64>)]) -> Vec<(Seg, Option<f64>)> {
    if segs.iter().all(|(_, r)| r.is_none()) {
        return segs.to_vec();
    }
    let mut out: Vec<(Seg, Option<f64>)> = Vec::with_capacity(segs.len() * 2);
    let mut i = 0;
    while i < segs.len() {
        let Seg::M(start) = segs[i].0 else {
            out.push(segs[i]);
            i += 1;
            continue;
        };
        // One subpath: M followed by non-M segments.
        let mut j = i + 1;
        while j < segs.len() && !matches!(segs[j].0, Seg::M(_)) {
            j += 1;
        }
        let body = &segs[i + 1..j];
        let closed = matches!(body.last(), Some((Seg::Z, _)));
        // Edges as (from, seg, radius); the close becomes an explicit line.
        let mut edges: Vec<(V, Seg, Option<f64>)> = Vec::new();
        let mut cur = start;
        for (s, r) in body {
            match *s {
                Seg::L(p) => {
                    edges.push((cur, Seg::L(p), *r));
                    cur = p;
                }
                Seg::C(a, b, p) => {
                    edges.push((cur, Seg::C(a, b, p), *r));
                    cur = p;
                }
                Seg::Z => {
                    if len(sub(cur, start)) > 1e-9 {
                        edges.push((cur, Seg::L(start), *r));
                    }
                    cur = start;
                }
                Seg::M(_) => {}
            }
        }
        let n = edges.len();
        if n == 0 {
            out.push(segs[i]);
            out.extend_from_slice(body);
            i = j;
            continue;
        }
        let end_of = |e: &(V, Seg, Option<f64>)| match e.1 {
            Seg::L(p) | Seg::C(_, _, p) => p,
            _ => e.0,
        };
        // Corner k is between edge k and edge k+1 (wrapping when closed).
        let corner_radius = |k: usize| -> Option<f64> {
            let next = if k + 1 < n { k + 1 } else if closed { 0 } else { return None };
            let (a, b) = (&edges[k], &edges[next]);
            if !matches!(a.1, Seg::L(_)) || !matches!(b.1, Seg::L(_)) {
                return None;
            }
            let r = b.2?;
            let l1 = len(sub(end_of(a), a.0));
            let l2 = len(sub(end_of(b), b.0));
            Some(r.min(l1 / 2.0).min(l2 / 2.0))
        };
        let trims: Vec<Option<f64>> = (0..n).map(corner_radius).collect();
        let trim_start = |k: usize| -> f64 {
            let prev = if k > 0 { Some(k - 1) } else if closed { Some(n - 1) } else { None };
            prev.and_then(|p| trims[p]).unwrap_or(0.0)
        };
        let edge_start = |k: usize| -> V {
            let e = &edges[k];
            let t = trim_start(k);
            if t > 0.0 { add(e.0, mul(unit(sub(end_of(e), e.0)).unwrap_or(v(0.0, 0.0)), t)) } else { e.0 }
        };
        out.push((Seg::M(edge_start(0)), None));
        for k in 0..n {
            let e = &edges[k];
            let end = end_of(e);
            match (e.1, trims[k]) {
                (Seg::L(_), Some(t)) => {
                    let d_in = unit(sub(end, e.0)).unwrap_or(v(0.0, 0.0));
                    let next = if k + 1 < n { k + 1 } else { 0 };
                    let ne = &edges[next];
                    let d_out = unit(sub(end_of(ne), ne.0)).unwrap_or(v(0.0, 0.0));
                    let p1 = sub(end, mul(d_in, t));
                    let p2 = add(end, mul(d_out, t));
                    out.push((Seg::L(p1), None));
                    out.push((Seg::C(add(p1, mul(d_in, t * KAPPA)), sub(p2, mul(d_out, t * KAPPA)), p2), None));
                }
                (seg, _) => out.push((seg, None)),
            }
        }
        if closed {
            out.push((Seg::Z, None));
        }
        i = j;
    }
    out
}
