//! Minimal pgfplots `axis`: one framed box plus a single
//! `\addplot{<expression in x>}` curve in the first cycle-list style
//! (blue line, `mark=*` filled with `blue!80!black`).
//!
//! The defaults are pgfplots' own (pgfplots.code.tex,
//! pgfplots.scaling.code.tex): `width=240pt`, `height=207pt`, of which
//! 45pt per direction are set aside for tick labels unless `scale only
//! axis`; `domain=-5:5`, `samples=25`; limits not given explicitly come
//! from the data and are enlarged by 10% of the range (`enlargelimits=auto`).
//! Tick marks, tick labels and axis labels are not drawn yet, and say so;
//! anything beyond one braced expression plot is reported, never silently
//! dropped.

use super::expr;
use super::text::{matching, split_top};
use crate::color::Paint;
use crate::geom::{Point, Transform};
use crate::path::{Path, StrokeStyle};

/// pgfplots' default `samples`.
pub(crate) const DEFAULT_SAMPLES: usize = 25;
/// pgfplots' default 2D domain.
pub(crate) const DEFAULT_DOMAIN: (f64, f64) = (-5.0, 5.0);
/// pgfplots' default `width` and `height`, in pt.
pub(crate) const DEFAULT_WIDTH_PT: f64 = 240.0;
pub(crate) const DEFAULT_HEIGHT_PT: f64 = 207.0;
/// What pgfplots subtracts from `width`/`height` for the tick labels
/// (`\pgfplots@initsizes@handle@label@const`, "FIXME determine 'c'
/// correctly"), unless `scale only axis`.
const LABEL_ALLOWANCE_PT: f64 = 45.0;
/// `enlargelimits=auto`: the relative enlargement of computed limits.
const ENLARGE: f64 = 0.1;
/// `mark size` default: the radius of `mark=*`, in pt.
const MARK_SIZE_PT: f64 = 2.0;
/// PGF's circle constant (`\pgfpathellipse`).
const KAPPA: f64 = 0.55228475;

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
    /// `blue!80!black`, the first cycle-list entry's mark fill.
    pub mark_fill: Paint,
}

pub(crate) struct AxisOutput {
    /// The axis frame (`axis lines=box`, pgfplots' default).
    pub frame: (Path, StrokeStyle, Paint),
    /// The frame again, for clipping the plot (pgfplots clips by default).
    pub clip: Path,
    /// The sampled curve, if its expression evaluated.
    pub plot: Option<(Path, StrokeStyle, Paint)>,
    /// `mark=*` circles, filled then stroked, drawn after the clip ends
    /// (pgfplots draws markers outside the clip path).
    pub marks: Vec<Path>,
    /// Stroke paint and fill paint of the marks.
    pub mark_paints: (Paint, Paint),
    /// Grown frame corners (half the line width, PGF's bbox rule), and the
    /// marks' extents.
    pub bbox: Vec<Point>,
}

struct Spec {
    domain: (f64, f64),
    xmin: Option<f64>,
    xmax: Option<f64>,
    ymin: Option<f64>,
    ymax: Option<f64>,
    samples: usize,
    width_pt: f64,
    height_pt: f64,
    scale_only_axis: bool,
    marks: bool,
}

fn number(text: &str, em: f64) -> Option<f64> {
    expr::eval(text, em).ok().map(|v| v.v).filter(|v| v.is_finite())
}

fn parse_opts(opts: &str, em: f64, warnings: &mut Vec<String>) -> Spec {
    let mut spec = Spec {
        domain: DEFAULT_DOMAIN,
        xmin: None,
        xmax: None,
        ymin: None,
        ymax: None,
        samples: DEFAULT_SAMPLES,
        width_pt: DEFAULT_WIDTH_PT,
        height_pt: DEFAULT_HEIGHT_PT,
        scale_only_axis: false,
        marks: true,
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
                    "xmin" => spec.xmin = Some(x),
                    "xmax" => spec.xmax = Some(x),
                    "ymin" => spec.ymin = Some(x),
                    _ => spec.ymax = Some(x),
                },
                None => warnings.push(format!("axis option `{key}={v}` is not a number; ignored")),
            },
            ("domain", Some(v)) => match v.split_once(':') {
                Some((a, b)) => match (number(a, em), number(b, em)) {
                    (Some(lo), Some(hi)) => spec.domain = (lo, hi),
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
            ("scale only axis", None | Some("true")) => spec.scale_only_axis = true,
            ("no markers", None) => spec.marks = false,
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
            warnings.push(format!("\\addplot options `{}` are ignored; the default blue line with marks is drawn", opts.trim()));
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


/// Final limits of one axis: explicit ones stay, the computed ones are
/// widened by 10% of the range between the (explicit or data) limits
/// (`enlargelimits=auto`, pgfplots.code.tex `/pgfplots/@enlargelimits/auto`).
fn limits(explicit: (Option<f64>, Option<f64>), data: (f64, f64)) -> (f64, f64) {
    let lo = explicit.0.unwrap_or(data.0);
    let hi = explicit.1.unwrap_or(data.1);
    let d = ENLARGE * (hi - lo);
    (if explicit.0.is_some() { lo } else { lo - d }, if explicit.1.is_some() { hi } else { hi + d })
}

/// PGF's `\pgfpathcircle` at `c` with radius `r`: four quarter arcs
/// counter-clockwise from the rightmost point, then a close.
fn circle(c: Point, r: f64, tf: &impl Fn(Point) -> Point) -> Path {
    let k = KAPPA * r;
    let p = |dx: f64, dy: f64| tf(Point::new(c.x + dx, c.y + dy));
    let mut path = Path::new();
    path.move_to(p(r, 0.0))
        .cubic_to(p(r, k), p(k, r), p(0.0, r))
        .cubic_to(p(-k, r), p(-r, k), p(-r, 0.0))
        .cubic_to(p(-r, -k), p(-k, -r), p(0.0, -r))
        .cubic_to(p(k, -r), p(r, -k), p(r, 0.0))
        .close();
    path
}

pub(crate) fn render_axis(inp: &AxisInput, warnings: &mut Vec<String>) -> Option<AxisOutput> {
    let spec = parse_opts(inp.opts, inp.em, warnings);
    if let (Some(lo), Some(hi)) = (spec.xmin, spec.xmax)
        && !(hi > lo)
    {
        warnings.push("axis has an empty x range; skipped".into());
        return None;
    }
    if let (Some(lo), Some(hi)) = (spec.ymin, spec.ymax)
        && !(hi > lo)
    {
        warnings.push("axis has an empty y range; skipped".into());
        return None;
    }
    let allowance = if spec.scale_only_axis { 0.0 } else { LABEL_ALLOWANCE_PT };
    let (w, h) = ((spec.width_pt - allowance).max(0.0), (spec.height_pt - allowance).max(0.0));
    let tf = |p: Point| inp.transform.apply(p);

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
    warnings.push("axis tick marks, tick labels and axis labels are not drawn".into());

    // One value per sample: `None` at a singular point (e.g. `1/x` at
    // x=0) breaks the polyline there instead of discarding the whole
    // curve.
    let mut samples: Vec<Option<(f64, f64)>> = Vec::new();
    if let Some(expr_text) = exprs.first() {
        let n = spec.samples;
        let (a, b) = spec.domain;
        let mut first_err: Option<String> = None;
        for i in 0..n {
            let x = a + (b - a) * i as f64 / (n - 1) as f64;
            match expr::eval_bound(expr_text, inp.em, "x", x) {
                Ok(val) if val.v.is_finite() => samples.push(Some((x, val.v))),
                Ok(_) => samples.push(None),
                Err(err) => {
                    first_err.get_or_insert(err.to_string());
                    samples.push(None);
                }
            }
        }
        if samples.iter().all(Option::is_none) {
            let reason = first_err.clone().unwrap_or_else(|| "has no finite points".to_string());
            warnings.push(format!("\\addplot{{{expr_text}}} {reason}; skipped"));
            samples.clear();
        } else if let Some(err) = first_err {
            warnings.push(format!("\\addplot{{{expr_text}}} {err} at some samples; those points are gapped"));
        }
    }
    let finite: Vec<(f64, f64)> = samples.iter().flatten().copied().collect();
    // Data limits; an empty axis falls back to pgfplots' [0,1] square.
    let span = |f: fn(&(f64, f64)) -> f64| {
        if finite.is_empty() {
            (0.0, 1.0)
        } else {
            finite.iter().map(f).fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| (lo.min(v), hi.max(v)))
        }
    };
    let widen = |(lo, hi): (f64, f64)| if hi > lo { (lo, hi) } else { (lo - 1.0, hi + 1.0) };
    let (xmin, xmax) = widen(limits((spec.xmin, spec.xmax), span(|p| p.0)));
    let (ymin, ymax) = widen(limits((spec.ymin, spec.ymax), span(|p| p.1)));
    let map = |x: f64, y: f64| Point::new((x - xmin) / (xmax - xmin) * w, (y - ymin) / (ymax - ymin) * h);

    // pgfplots strokes the box from the lower left corner upwards and
    // clips to it counter-clockwise from the same corner.
    let (c00, c10, c11, c01) =
        (tf(Point::new(0.0, 0.0)), tf(Point::new(w, 0.0)), tf(Point::new(w, h)), tf(Point::new(0.0, h)));
    let mut frame = Path::new();
    frame.move_to(c00).line_to(c01).line_to(c11).line_to(c10).line_to(c00).close();
    let mut clip = Path::new();
    clip.move_to(c00).line_to(c10).line_to(c11).line_to(c01).close();

    let hw = inp.line_width / 2.0;
    let mut bbox = vec![tf(Point::new(-hw, -hw)), tf(Point::new(w + hw, h + hw))];

    let mut plot = None;
    let mut marks = Vec::new();
    if !finite.is_empty() {
        let mut path = Path::new();
        let mut pen = false;
        for sample in &samples {
            let Some((x, y)) = sample else {
                pen = false;
                continue;
            };
            let q = map(*x, *y);
            let p = tf(q);
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
            if spec.marks {
                marks.push(circle(q, MARK_SIZE_PT, &tf));
                let e = MARK_SIZE_PT + hw;
                bbox.push(tf(Point::new(q.x - e, q.y - e)));
                bbox.push(tf(Point::new(q.x + e, q.y + e)));
            }
        }
        plot = Some((path, inp.frame_style.clone(), inp.plot_paint));
    }

    Some(AxisOutput {
        frame: (frame, inp.frame_style.clone(), inp.frame_paint),
        clip,
        plot,
        marks,
        mark_paints: (inp.plot_paint, inp.mark_fill),
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
