//! Minimal pgfplots `axis`: one framed box plus a single
//! `\addplot{<expression in x>}` curve in the default first-cycle colour.
//!
//! Defaults (an implementation default where pgfplots would read its own
//! keys): the box is 6cm square, the domain is `-5:5` with 101 samples,
//! the y limits come from the sampled data. Tick marks, tick labels and
//! grid lines are not drawn yet; anything beyond one braced expression
//! plot is reported, never silently dropped.

use super::expr;
use super::text::{matching, split_top};
use crate::color::Paint;
use crate::geom::{Point, Transform};
use crate::path::{Path, StrokeStyle};

/// Samples across the default domain (pgfplots samples its own default;
/// this count is ours).
pub(crate) const DEFAULT_SAMPLES: usize = 101;
/// pgfplots' default 2D domain.
pub(crate) const DEFAULT_DOMAIN: (f64, f64) = (-5.0, 5.0);
/// Default axis size, centimetres (an implementation default).
pub(crate) const DEFAULT_WIDTH_CM: f64 = 6.0;
pub(crate) const DEFAULT_HEIGHT_CM: f64 = 6.0;

/// What the interpreter hands over: the axis `[options]`, the body up to
/// `\end{axis}`, and the ambient graphics state the axis inherits.
pub(crate) struct AxisInput<'a> {
    pub opts: &'a str,
    pub body: &'a str,
    pub em: f64,
    pub line_width: f64,
    pub transform: Transform,
    pub frame_style: StrokeStyle,
    pub frame_paint: Paint,
    pub plot_paint: Paint,
}

pub(crate) struct AxisOutput {
    /// The axis frame (`axis lines=box`, pgfplots' default).
    pub frame: (Path, StrokeStyle, Paint),
    /// The frame again, for clipping the plot (pgfplots clips by default).
    pub clip: Path,
    /// The sampled curve, if its expression evaluated.
    pub plot: Option<(Path, StrokeStyle, Paint)>,
    /// Grown frame corners (half the line width, PGF's bbox rule).
    pub bbox: [Point; 2],
}

struct Spec {
    xmin: f64,
    xmax: f64,
    ymin: Option<f64>,
    ymax: Option<f64>,
    samples: usize,
    width_pt: f64,
    height_pt: f64,
}

fn number(text: &str, em: f64) -> Option<f64> {
    expr::eval(text, em).ok().map(|v| v.v).filter(|v| v.is_finite())
}

fn parse_opts(opts: &str, em: f64, warnings: &mut Vec<String>) -> Spec {
    let mut spec = Spec {
        xmin: DEFAULT_DOMAIN.0,
        xmax: DEFAULT_DOMAIN.1,
        ymin: None,
        ymax: None,
        samples: DEFAULT_SAMPLES,
        width_pt: DEFAULT_WIDTH_CM * expr::PT_PER_CM,
        height_pt: DEFAULT_HEIGHT_CM * expr::PT_PER_CM,
    };
    for entry in split_top(opts, b',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (key, val) = match entry.split_once('=') {
            Some((k, v)) => (k.trim(), Some(v.trim())),
            None => (entry, None),
        };
        match (key, val) {
            ("xmin" | "xmax" | "ymin" | "ymax", Some(v)) => match number(v, em) {
                Some(x) => match key {
                    "xmin" => spec.xmin = x,
                    "xmax" => spec.xmax = x,
                    "ymin" => spec.ymin = Some(x),
                    _ => spec.ymax = Some(x),
                },
                None => warnings.push(format!("axis option `{key}={v}` is not a number; ignored")),
            },
            ("domain", Some(v)) => match v.split_once(':') {
                Some((a, b)) => match (number(a, em), number(b, em)) {
                    (Some(lo), Some(hi)) => {
                        spec.xmin = lo;
                        spec.xmax = hi;
                    }
                    _ => warnings.push(format!("axis option `domain={v}` is not `a:b`; ignored")),
                },
                None => warnings.push(format!("axis option `domain={v}` is not `a:b`; ignored")),
            },
            ("samples", Some(v)) => match v.parse::<usize>() {
                Ok(n) if n >= 2 => spec.samples = n.min(10_000),
                _ => warnings.push(format!("axis option `samples={v}` needs an integer >= 2; ignored")),
            },
            ("width" | "height", Some(v)) => match expr::length_pt(v, em) {
                Ok(w) if w.is_finite() && w > 0.0 => {
                    if key == "width" {
                        spec.width_pt = w;
                    } else {
                        spec.height_pt = w;
                    }
                }
                _ => warnings.push(format!("axis option `{key}={v}` is not a positive length; ignored")),
            },
            _ => warnings.push(format!("axis option `{key}` is not supported; ignored")),
        }
    }
    spec
}

/// Byte index just past `\addplot[+][opts]` at `i`, plus the options text.
fn addplot_head(s: &str, i: usize) -> (usize, String) {
    let mut k = i + "\\addplot".len();
    let b = s.as_bytes();
    if b.get(k) == Some(&b'+') {
        k += 1;
    }
    while k < b.len() && (b[k] as char).is_whitespace() {
        k += 1;
    }
    let mut opts = String::new();
    if s[k..].starts_with('[')
        && let Some(e) = matching(s, k) {
            opts = s[k + 1..e - 1].to_string();
            k = e;
        }
    (k, opts)
}

/// The braced plot expressions in the axis body, in order; `\addplot`
/// forms without one (coordinates, tables) come back as warnings.
/// Every `\addplot` occurrence's extracted expression (if it had a braced
/// one) together with the exact byte span of `body` it consumed, so the
/// caller can tell what's left over and outside every `\addplot` this
/// function handled -- content the axis silently doesn't draw otherwise.
fn find_plots(body: &str, warnings: &mut Vec<String>) -> (Vec<String>, Vec<(usize, usize)>) {
    let mut out = Vec::new();
    let mut spans = Vec::new();
    let mut from = 0;
    while let Some(rel) = body[from..].find("\\addplot") {
        let at = from + rel;
        let after = body[at + "\\addplot".len()..].chars().next();
        if after.is_some_and(|c| c.is_ascii_alphanumeric()) {
            from = at + 1;
            continue;
        }
        let (mut k, opts) = addplot_head(body, at);
        if !opts.trim().is_empty() {
            warnings.push(format!("\\addplot options `{}` are ignored; the default blue line is drawn", opts.trim()));
        }
        while k < body.len() && (body.as_bytes()[k] as char).is_whitespace() {
            k += 1;
        }
        // `\addplot expression {x^2}` spells the keyword out.
        if body[k..].starts_with("expression") && !body[k + 10..].starts_with(|c: char| c.is_ascii_alphanumeric()) {
            k += "expression".len();
            while k < body.len() && (body.as_bytes()[k] as char).is_whitespace() {
                k += 1;
            }
        }
        if body[k..].starts_with('{')
            && let Some(e) = matching(body, k) {
                out.push(body[k + 1..e - 1].to_string());
                spans.push((at, e));
                from = e;
                continue;
            }
        warnings.push("\\addplot without a braced expression (coordinates, tables) is not supported; skipped".into());
        let end = k.max(at + 1);
        spans.push((at, end));
        from = end;
    }
    (out, spans)
}

pub(crate) fn render_axis(inp: &AxisInput, warnings: &mut Vec<String>) -> Option<AxisOutput> {
    let spec = parse_opts(inp.opts, inp.em, warnings);
    if !(spec.xmax > spec.xmin) {
        warnings.push("axis has an empty x range; skipped".into());
        return None;
    }
    if let (Some(lo), Some(hi)) = (spec.ymin, spec.ymax)
        && !(hi > lo)
    {
        warnings.push("axis has an empty y range; skipped".into());
        return None;
    }
    let map = |x: f64, y: f64, ymin: f64, ymax: f64| {
        Point::new((x - spec.xmin) / (spec.xmax - spec.xmin) * spec.width_pt, (y - ymin) / (ymax - ymin) * spec.height_pt)
    };
    let tf = |p: Point| inp.transform.apply(p);

    let mut frame = Path::new();
    let c00 = tf(Point::new(0.0, 0.0));
    let c10 = tf(Point::new(spec.width_pt, 0.0));
    let c11 = tf(Point::new(spec.width_pt, spec.height_pt));
    let c01 = tf(Point::new(0.0, spec.height_pt));
    frame.move_to(c00).line_to(c10).line_to(c11).line_to(c01).close();

    let h = inp.line_width / 2.0;
    let bbox = [tf(Point::new(-h, -h)), tf(Point::new(spec.width_pt + h, spec.height_pt + h))];

    let (exprs, plot_spans) = find_plots(inp.body, warnings);
    if exprs.len() > 1 {
        warnings.push("only one \\addplot is supported; the first is drawn".into());
    }
    if has_undrawn_content(inp.body, &plot_spans) {
        warnings.push(
            "axis body has content besides \\addplot (\\draw, \\node, \\legend, coordinates, ...); \
             only the plotted expression is drawn"
                .into(),
        );
    }
    let plot = exprs.first().and_then(|expr_text| {
        let n = spec.samples;
        // One value per sample: `None` at a singular point (e.g. `1/x` at
        // x=0) breaks the polyline there instead of discarding the whole
        // curve -- only abort entirely when NOT ONE sample evaluated.
        let mut samples: Vec<Option<(f64, f64)>> = Vec::with_capacity(n);
        let mut first_err: Option<String> = None;
        for i in 0..n {
            let x = spec.xmin + (spec.xmax - spec.xmin) * i as f64 / (n - 1) as f64;
            match expr::eval_bound(expr_text, inp.em, "x", x) {
                Ok(val) if val.v.is_finite() => samples.push(Some((x, val.v))),
                Ok(_) => samples.push(None),
                Err(err) => {
                    first_err.get_or_insert(err.to_string());
                    samples.push(None);
                }
            }
        }
        let finite: Vec<(f64, f64)> = samples.iter().flatten().copied().collect();
        if finite.is_empty() {
            let reason = first_err.unwrap_or_else(|| "has no finite points".to_string());
            warnings.push(format!("\\addplot{{{expr_text}}} {reason}; skipped"));
            return None;
        }
        if let Some(err) = first_err {
            warnings.push(format!("\\addplot{{{expr_text}}} {err} at some samples; those points are gapped"));
        }
        let ymin = spec.ymin.unwrap_or_else(|| finite.iter().fold(f64::INFINITY, |a, (_, y)| a.min(*y)));
        let ymax = spec.ymax.unwrap_or_else(|| finite.iter().fold(f64::NEG_INFINITY, |a, (_, y)| a.max(*y)));
        let (ymin, ymax) = if ymax > ymin { (ymin, ymax) } else { (ymin - 1.0, ymax + 1.0) };
        let mut path = Path::new();
        let mut pen = false;
        for sample in &samples {
            let Some((x, y)) = sample else {
                pen = false;
                continue;
            };
            let p = tf(map(*x, *y, ymin, ymax));
            if !p.is_finite() {
                pen = false;
                continue;
            }
            if pen {
                path.line_to(p);
            } else {
                path.move_to(p);
                pen = true;
            }
        }
        Some((path, inp.frame_style.clone(), inp.plot_paint))
    });

    Some(AxisOutput {
        frame: (frame.clone(), inp.frame_style.clone(), inp.frame_paint),
        clip: frame,
        plot,
        bbox,
    })
}

/// Whether `body` has any non-whitespace content outside the exact byte
/// spans `find_plots` already consumed (every `\addplot` occurrence it
/// handled, successfully or not) -- `\draw`, `\node`, `\legend`, bare
/// text, anything this minimal axis doesn't model and would otherwise
/// silently not draw.
fn has_undrawn_content(body: &str, plot_spans: &[(usize, usize)]) -> bool {
    // `;` is TikZ's statement terminator, not drawable content.
    let is_gap = |c: char| c.is_whitespace() || c == ';';
    let mut spans = plot_spans.to_vec();
    spans.sort_unstable();
    let mut pos = 0;
    for (start, end) in spans {
        if body[pos..start.max(pos)].chars().any(|c| !is_gap(c)) {
            return true;
        }
        pos = end.max(pos);
    }
    body[pos..].chars().any(|c| !is_gap(c))
}
