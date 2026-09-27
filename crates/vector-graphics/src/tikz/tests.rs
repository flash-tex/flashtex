use super::*;
use crate::color::Color;
use crate::item::Item;
use crate::path::PathCommand;

const K: f64 = 72.0 / 72.27;
const CM: f64 = 72.27 / 2.54;

fn render(body: &str) -> Picture {
    Tikz::new(10.0).render_body("", body, &ApproxMeasurer)
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn strokes(p: &Picture) -> Vec<&crate::item::PathStroke> {
    p.items
        .iter()
        .filter_map(|i| if let Item::PathStroke(s) = i { Some(s) } else { None })
        .collect()
}

fn fills(p: &Picture) -> Vec<&crate::item::PathFill> {
    p.items
        .iter()
        .filter_map(|i| if let Item::PathFill(s) = i { Some(s) } else { None })
        .collect()
}

#[test]
fn line_geometry_and_bbox_match_pgf() {
    let p = render(r"\draw (0,0) -- (2,1);");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    // bbox: 2cm x 1cm plus the line width (half on each side).
    assert!(close(p.width_bp, (2.0 * CM + 0.4) * K, 1e-6), "{}", p.width_bp);
    assert!(close(p.height_bp, (CM + 0.4) * K, 1e-6));
    let s = strokes(&p);
    assert_eq!(s.len(), 1);
    assert!(close(s[0].style.width, 0.4 * K, 1e-9));
    let cmds = s[0].path.commands();
    // Top-left origin, y down: (0,0) is at the bottom-left of the bbox.
    match (cmds[0], cmds[1]) {
        (PathCommand::MoveTo(a), PathCommand::LineTo(b)) => {
            assert!(close(a.x, 0.2 * K, 1e-6) && close(a.y, (CM + 0.2) * K, 1e-6), "{a:?}");
            assert!(close(b.x, (2.0 * CM + 0.2) * K, 1e-6) && close(b.y, 0.2 * K, 1e-6), "{b:?}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn arrow_to_tip_shortens_the_line_like_pgf() {
    let p = render(r"\draw[->] (0,0) -- (2,0);");
    let s = strokes(&p);
    assert_eq!(s.len(), 2, "shaft + tip");
    let end = match s[0].path.commands()[1] {
        PathCommand::LineTo(b) => b,
        c => panic!("{c:?}"),
    };
    // Shortened by 0.21pt + 0.625 * 0.4pt = 0.46pt (pdflatex: 56.6929 - 0.4583 bp).
    let x0 = match s[0].path.commands()[0] {
        PathCommand::MoveTo(a) => a.x,
        c => panic!("{c:?}"),
    };
    assert!(close(end.x - x0, (2.0 * CM - 0.46) * K, 1e-6), "{}", end.x - x0);
    assert!(close(s[1].style.width, 0.32 * K, 1e-9));
}

#[test]
fn stealth_and_latex_tips_are_filled() {
    let p = render(r"\draw[-stealth] (0,0) -- (1,0); \draw[latex-] (0,1) -- (1,1);");
    assert_eq!(fills(&p).len(), 2);
    assert_eq!(strokes(&p).len(), 2);
}

#[test]
fn circle_rectangle_and_fill_colors() {
    let p = render(r"\fill[red!50] (0,0) rectangle (1,1); \draw[blue, thick] (2,0) circle (0.5);");
    let f = fills(&p);
    assert_eq!(f[0].paint.color, Color::Rgb(1.0, 0.5, 0.5));
    let s = strokes(&p);
    assert_eq!(s[0].paint.color, Color::Rgb(0.0, 0.0, 1.0));
    assert!(close(s[0].style.width, 0.8 * K, 1e-9));
    let b = s[0].path.bounds().unwrap();
    assert!(close(b.width, CM * K, 1e-3), "{b:?}");
}

#[test]
fn foreach_and_scopes() {
    let p = render(r"\foreach \x in {0,1,...,4} { \draw (\x,0) -- (\x,1); }
        \begin{scope}[xshift=5cm, dashed] \draw (0,0) -- (1,0); \end{scope}");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    let s = strokes(&p);
    assert_eq!(s.len(), 6);
    assert!(s[5].style.dash.is_some());
    assert!(s[4].style.dash.is_none());
}

#[test]
fn nodes_anchor_and_clip_lines() {
    let p = render(r"\node[draw] (a) at (0,0) {ab}; \node[draw,circle] (b) at (3,0) {c}; \draw[->] (a) -- (b);");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    assert_eq!(p.texts.len(), 2);
    let s = strokes(&p);
    assert_eq!(s.len(), 4, "two borders, a shaft and a tip");
    // The shaft starts at a's east border: half text width 5pt + inner sep + outer sep.
    let a_center_x = p.texts[0].transform.e + 5.0 * K;
    let start = match s[2].path.commands()[0] {
        PathCommand::MoveTo(q) => q,
        c => panic!("{c:?}"),
    };
    assert!(close(start.x - a_center_x, (5.0 + 3.333 + 0.2) * K, 1e-3), "{}", start.x - a_center_x);
}

#[test]
fn styles_and_midway_labels() {
    let mut t = Tikz::new(10.0);
    let d = t.read_preamble(r"\tikzset{box/.style={draw=#1, thick}, box/.default=red}");
    assert!(d.is_empty(), "{d:?}");
    let p = t.render_body("", r"\node[box] at (0,0) {x}; \draw (0,0) -- (2,0) node[midway,above] {m};", &ApproxMeasurer);
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    let s = strokes(&p);
    assert_eq!(s[0].paint.color, Color::Rgb(1.0, 0.0, 0.0));
    // The label's baseline start sits above the line's midpoint.
    let label = &p.texts[1];
    let line_y = match s[1].path.commands()[0] {
        PathCommand::MoveTo(q) => q.y,
        c => panic!("{c:?}"),
    };
    assert!(label.transform.f < line_y);
}

#[test]
fn node_align_centers_stacked_lines_like_pdflatex() {
    let p = render(r"\node[align=center] at (0,0) {Line one\\Line two};");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    assert_eq!(p.texts.len(), 2, "two stacked runs, not one joined line");
    assert_eq!(p.texts[0].text, "Line one");
    assert_eq!(p.texts[1].text, "Line two");
    // pdflatex sets the two baselines one \baselineskip apart (12pt at 10pt).
    assert!(
        close(p.texts[1].transform.f - p.texts[0].transform.f, 12.0 * K, 1e-6),
        "{:?} {:?}",
        p.texts[0].transform,
        p.texts[1].transform
    );
    // Equal widths stay centred: no x shift between the runs.
    assert!(close(p.texts[1].transform.e - p.texts[0].transform.e, 0.0, 1e-6));
}

#[test]
fn node_align_left_center_right_offsets_and_border() {
    // ApproxMeasurer: half an em a char at 10pt, so "AAAA" is 20pt wide and
    // "BB" is 10pt wide; neither has ascenders beyond 6.83pt nor depth.
    for (align, dx) in [("left", 0.0), ("center", 5.0), ("right", 10.0)] {
        let p = render(&format!(r"\node[align={align}] at (0,0) {{AAAA\\BB}};"));
        assert!(p.diagnostics.is_empty(), "{align}: {:?}", p.diagnostics);
        assert_eq!(p.texts.len(), 2, "{align}");
        assert!(
            close(p.texts[1].transform.f - p.texts[0].transform.f, 12.0 * K, 1e-6),
            "{align}: {:?} {:?}",
            p.texts[0].transform,
            p.texts[1].transform
        );
        assert!(
            close(p.texts[1].transform.e - p.texts[0].transform.e, dx * K, 1e-6),
            "{align}: {:?} {:?}",
            p.texts[0].transform,
            p.texts[1].transform
        );
    }
    // The drawn border grows to fit both lines.
    let p = render(r"\node[align=center,draw] at (0,0) {AAAA\\BB};");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    let s = strokes(&p);
    assert_eq!(s.len(), 1);
    let b = s[0].path.bounds().unwrap();
    // Width: 20pt text + 2 x 3.333pt inner sep; height: 6.83pt first-line
    // height + 12pt baselineskip + 2 x 3.333pt inner sep ("BB" has no depth).
    assert!(close(b.width, (20.0 + 2.0 * 3.3333) * K, 1e-2), "{b:?}");
    assert!(close(b.height, (6.83 + 12.0 + 2.0 * 3.3333) * K, 1e-2), "{b:?}");
}

#[test]
fn node_line_break_without_align_still_joins_with_space() {
    let p = render(r"\node at (0,0) {AAAA\\BB};");
    assert_eq!(p.texts.len(), 1);
    assert_eq!(p.texts[0].text, "AAAA BB");
    assert!(
        p.diagnostics.iter().any(|d| d.message.contains("need `align`")),
        "{:?}",
        p.diagnostics
    );
}

#[test]
fn unsupported_input_is_reported_not_dropped() {
    let p = render(r"\shade (0,0) rectangle (1,1); \draw[decorate] (0,0) -- (1,0); \draw (0,0) plot (1,1);");
    assert!(p.diagnostics.len() >= 3, "{:?}", p.diagnostics);
}

#[test]
fn all_pgf_patterns_and_pattern_color_attach_to_fills() {
    let names = [
        "north east lines",
        "north west lines",
        "horizontal lines",
        "vertical lines",
        "grid",
        "crosshatch",
        "dots",
        "crosshatch dots",
    ];
    let mut body = String::new();
    for (i, name) in names.iter().enumerate() {
        body.push_str(&format!(r"\fill[pattern={name},pattern color=red!20] ({i},0) rectangle ({},1);", i + 1));
    }
    body.push_str(r"\path[pattern=dots,pattern color=blue] (9,0) rectangle (10,1);");
    let p = render(&body);
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    let f = fills(&p);
    assert_eq!(f.len(), 9);
    for (f, name) in f.iter().zip(names.iter()) {
        let pattern = f.pattern.as_ref().expect("pattern");
        assert_eq!(pattern.name, *name);
        assert_eq!(pattern.color, Color::Rgb(1.0, 0.8, 0.8));
    }
    assert_eq!(f[8].pattern.as_ref().unwrap().name, "dots");
    assert_eq!(f[8].pattern.as_ref().unwrap().color, Color::Rgb(0.0, 0.0, 1.0));
}

#[test]
fn pattern_styles_and_scope_inheritance_are_preserved() {
    let mut t = Tikz::new(10.0);
    let d = t.read_preamble(r"\tikzset{gridfill/.style={pattern=grid,pattern color=blue}}");
    assert!(d.is_empty(), "{d:?}");
    let p = t.render_body(
        "",
        r"\fill[gridfill] (0,0) rectangle (1,1);
            \begin{scope}[pattern=north west lines,pattern color=green]
              \fill (1,0) rectangle (2,1);
              \begin{scope}[pattern color=red]
                \fill (2,0) rectangle (3,1);
              \end{scope}
            \end{scope}",
        &ApproxMeasurer,
    );
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    let f = fills(&p);
    assert_eq!(f.len(), 3);
    assert_eq!(f[0].pattern.as_ref().unwrap().name, "grid");
    assert_eq!(f[0].pattern.as_ref().unwrap().color, Color::Rgb(0.0, 0.0, 1.0));
    assert_eq!(f[1].pattern.as_ref().unwrap().name, "north west lines");
    assert_eq!(f[1].pattern.as_ref().unwrap().color, Color::Rgb(0.0, 1.0, 0.0));
    assert_eq!(f[2].pattern.as_ref().unwrap().name, "north west lines");
    assert_eq!(f[2].pattern.as_ref().unwrap().color, Color::Rgb(1.0, 0.0, 0.0));
}

#[test]
fn unknown_pattern_warns_and_keeps_the_plain_fill() {
    let p = render(r"\fill[pattern=not-a-pgf-pattern] (0,0) rectangle (1,1);");
    assert_eq!(fills(&p).len(), 1);
    assert!(fills(&p)[0].pattern.is_none());
    assert!(p.diagnostics.iter().any(|d| d.message.contains("unknown pattern")), "{:?}", p.diagnostics);
}

#[test]
fn find_pictures_locates_bodies_and_options() {
    let doc = "a % \\begin{tikzpicture}\n\\begin{tikzpicture}[scale=2]\\draw (0,0)--(1,1);\\end{tikzpicture} b";
    let pics = find_pictures(doc);
    assert_eq!(pics.len(), 1);
    let pic = &pics[0];
    assert_eq!(&doc[pic.options.unwrap().0..pic.options.unwrap().1], "scale=2");
    assert_eq!(&doc[pic.body_start..pic.body_end], "\\draw (0,0)--(1,1);");
    let out = Tikz::default().render(doc, pic, &ApproxMeasurer);
    assert!(close(out.width_bp, (2.0 * CM + 0.4) * K, 1e-6));
}

#[test]
fn braced_arithmetic_in_coordinate_defines_the_node() {
    // Issue #898: `({\a+\c},2)` is the idiomatic way to write macro
    // arithmetic in a coordinate; the braces must parse as a group and
    // later paths referencing `(P)` must still draw.
    let p = render(
        r"\def\a{3}
        \def\c{1}
        \coordinate (O) at (0,0);
        \coordinate (P) at ({\a+\c},2);
        \draw[thick] (O) -- (P);
        \draw[dotted, thick] (O) -- (4,0) -- (P) -- cycle;",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    let s = strokes(&p);
    assert_eq!(s.len(), 2, "both draws reference (P) and must draw");
    // (P) is (4,2): the first stroke spans 4cm horizontally.
    let (a, b) = match (s[0].path.commands()[0], s[0].path.commands()[1]) {
        (PathCommand::MoveTo(a), PathCommand::LineTo(b)) => (a, b),
        other => panic!("{other:?}"),
    };
    assert!(close(b.x - a.x, 4.0 * CM * K, 1e-6), "{a:?} {b:?}");
    assert!(close(a.y - b.y, 2.0 * CM * K, 1e-6), "{a:?} {b:?}");
}

#[test]
fn rounded_corners_arcs_grids_and_curves() {
    let p = render(r"\draw[rounded corners] (0,0) rectangle (2,1);
        \draw (3,0) arc (0:90:1);
        \draw[step=0.5] (0,2) grid (1,3);
        \draw (0,4) .. controls (1,5) and (2,5) .. (3,4);
        \draw (0,6) to[bend left] (2,6);
        \draw (0,7) -| (1,8) |- (2,9);");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    let s = strokes(&p);
    assert_eq!(s.len(), 6);
    let curves = |i: usize| s[i].path.commands().iter().filter(|c| matches!(c, PathCommand::CubicTo(..))).count();
    assert_eq!(curves(0), 4, "four rounded corners");
    assert_eq!(curves(1), 1, "a quarter arc");
    // PGF's sp arithmetic: 0.5cm truncates to 932339sp, so 2cm is not a
    // multiple and its line is skipped; 3cm and 1cm come from the final
    // 0.01pt-early line. 2 horizontal + 3 vertical.
    assert_eq!(s[2].path.commands().iter().filter(|c| matches!(c, PathCommand::MoveTo(..))).count(), 5);
    assert_eq!(curves(3), 1);
    assert_eq!(curves(4), 1);
}

#[test]
fn sin_and_cos_use_pgf_control_points() {
    let p = render(r"\draw (0,0) sin (1,1) cos (2,0);");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    let s = strokes(&p);
    assert_eq!(s.len(), 1);
    let cmds = s[0].path.commands();
    assert_eq!(cmds.len(), 3, "{cmds:?}");
    let (a, c1, c2, b, d1, d2, c) = match (cmds[0], cmds[1], cmds[2]) {
        (
            PathCommand::MoveTo(a),
            PathCommand::CubicTo(c1, c2, b),
            PathCommand::CubicTo(d1, d2, c),
        ) => (a, c1, c2, b, d1, d2, c),
        other => panic!("{other:?}"),
    };
    // PGF's quarter-period Bézier approximations from
    // pgfcorepathconstruct.code.tex: \pgfpathsine uses
    // (0.3260, 0.5120) and (0.6380, 1.0); \pgfpathcosine uses
    // (0.3620, 0.0) and (0.6740, 0.4880), relative to the start point.
    let tol = 1e-6;
    assert!(close(c1.x - a.x, 0.3260 * (b.x - a.x), tol), "{a:?} {c1:?} {b:?}");
    assert!(close(c1.y - a.y, 0.5120 * (b.y - a.y), tol), "{a:?} {c1:?} {b:?}");
    assert!(close(c2.x - a.x, 0.6380 * (b.x - a.x), tol), "{a:?} {c2:?} {b:?}");
    assert!(close(c2.y - a.y, b.y - a.y, tol), "{a:?} {c2:?} {b:?}");
    assert!(close(d1.x - b.x, 0.3620 * (c.x - b.x), tol), "{b:?} {d1:?} {c:?}");
    assert!(close(d1.y - b.y, 0.0, tol), "{b:?} {d1:?} {c:?}");
    assert!(close(d2.x - b.x, 0.6740 * (c.x - b.x), tol), "{b:?} {d2:?} {c:?}");
    assert!(close(d2.y - b.y, 0.4880 * (c.y - b.y), tol), "{b:?} {d2:?} {c:?}");
    // Both segments are monotone, so the curve bbox is (0,0)-(2,1)cm
    // plus the line width (half on each side).
    assert!(close(p.width_bp, (2.0 * CM + 0.4) * K, 1e-6), "{}", p.width_bp);
    assert!(close(p.height_bp, (CM + 0.4) * K, 1e-6), "{}", p.height_bp);
}

#[test]
fn sin_control_points_rotate_with_the_scope() {
    // The PGF fractions are only valid in the *local* (pre-transform) frame:
    // a control point at local (0.3260, 0.5120) relative to the segment's
    // local delta must come out, after a 90 degree scope rotation about the
    // origin, as that same local vector rotated as a whole -- length
    // sqrt(0.3260^2 + 0.5120^2) * CM preserved, at (-0.5120, -0.3260) * CM
    // in this render pipeline's output axes -- NOT at (-0.3260, 0.5120) * CM,
    // which is what you get if the fractions are wrongly applied to the
    // already-rotated device-space delta's x/y components independently
    // (the bug this test regresses against).
    let p = render(r"\begin{scope}[rotate=90] \draw (0,0) sin (1,1); \end{scope}");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    let s = strokes(&p);
    assert_eq!(s.len(), 1);
    let cmds = s[0].path.commands();
    let (a, c1) = match (cmds[0], cmds[1]) {
        (PathCommand::MoveTo(a), PathCommand::CubicTo(c1, _, _)) => (a, c1),
        other => panic!("{other:?}"),
    };
    let tol = 1e-6;
    assert!(close(c1.x - a.x, -0.5120 * CM * K, tol), "{a:?} {c1:?}");
    assert!(close(c1.y - a.y, -0.3260 * CM * K, tol), "{a:?} {c1:?}");
}

/// Points of a path's move/line commands.
fn pts_of(path: &crate::path::Path) -> Vec<crate::Point> {
    path.commands()
        .iter()
        .filter_map(|c| match *c {
            PathCommand::MoveTo(a) | PathCommand::LineTo(a) => Some(a),
            _ => None,
        })
        .collect()
}

/// Only the always-on "ticks are not drawn" warning.
fn only_tick_warning(p: &Picture) -> bool {
    p.diagnostics.len() == 1 && p.diagnostics[0].message.contains("tick labels")
}

// pdflatex 2026, `\begin{axis} \addplot {x^2}; \end{axis}` (pgfplots,
// no compat): box -16.19081 -13.45036 to 178.0893 147.94608, i.e.
// 194.27011 x 161.39644bp = 195 x 162pt (240pt, 207pt minus 45pt each);
// polyline 0 134.4957 m ... 80.94922 0 l ... 161.89847 134.48708 l with
// the data origin (-5,0) at (0,0): x=-5 and 5 sit 1/12 in from the box
// sides (limits -6:6), y=0 at 1/12 and y=25 at 11/12 of the height
// (limits -2.5:27.5); then 25 `B` circles of radius 1.99255bp, fill
// 0 0 0.8 rg, stroke 0 0 1 RG, after the clip ends. PGF writes 1pt as
// 0.99627bp and pgfplots' unit vectors round, so compare within 0.01bp
// in PGF's scale.
#[test]
fn axis_defaults_match_pgfplots() {
    let p = render("\\begin{axis}\n\\addplot{x^2};\n\\end{axis}");
    assert!(only_tick_warning(&p), "{:?}", p.diagnostics);
    let k = 28.3468 / (CM * K);
    let frame = match &p.items[0] {
        Item::PathStroke(s) => s,
        other => panic!("frame: {other:?}"),
    };
    assert_eq!(frame.paint.color, Color::BLACK);
    let f = pts_of(&frame.path);
    // Lower left, then up: m l l l l h.
    assert_eq!(f.len(), 5, "{f:?}");
    let (x0, y0) = (f[0].x, f[0].y);
    let (w, h) = ((f[2].x - x0) * k, (y0 - f[2].y) * k);
    assert!(close(w, 194.27011, 0.01), "{w}");
    assert!(close(h, 161.39644, 0.01), "{h}");
    assert!(close(f[1].x, x0, 1e-9) && f[1].y < y0, "goes up first: {f:?}");
    let rel = |q: crate::Point| ((q.x - x0) * k, (y0 - q.y) * k);

    let plot = match &p.items[1] {
        Item::Group(g) => {
            assert!(g.clip.is_some(), "plot is clipped to the frame");
            match &g.items[0] {
                Item::PathStroke(s) => s,
                other => panic!("plot: {other:?}"),
            }
        }
        other => panic!("plot group: {other:?}"),
    };
    assert_eq!(plot.paint.color, Color::Rgb(0.0, 0.0, 1.0));
    let pts = pts_of(&plot.path);
    assert_eq!(pts.len(), 25, "samples=25");
    for (i, (ex, ey)) in [(0, (16.19081, 147.94606)), (12, (97.14003, 13.45036)), (24, (178.08928, 147.93744))] {
        let (x, y) = rel(pts[i]);
        assert!(close(x, ex, 0.01) && close(y, ey, 0.01), "sample {i}: ({x}, {y})");
    }

    // 25 marks, each a filled then stroked circle of radius 2pt.
    let marks = &p.items[2..];
    assert_eq!(marks.len(), 50, "{marks:?}");
    for (n, pair) in marks.chunks(2).enumerate() {
        let (fill, stroke) = match pair {
            [Item::PathFill(f), Item::PathStroke(s)] => (f, s),
            other => panic!("mark {n}: {other:?}"),
        };
        assert_eq!(fill.paint.color, Color::Rgb(0.0, 0.0, 0.8));
        assert_eq!(stroke.paint.color, Color::Rgb(0.0, 0.0, 1.0));
        let first = match fill.path.commands()[0] {
            PathCommand::MoveTo(q) => q,
            c => panic!("{c:?}"),
        };
        // The circle starts at its rightmost point, 2pt right of the sample.
        assert!(close((first.x - pts[n].x) * k, 1.99255, 1e-3), "mark {n}");
        assert!(close(first.y, pts[n].y, 1e-9), "mark {n}");
    }
}

#[test]
fn axis_width_height_and_explicit_limits() {
    // pdflatex, [width=8cm,height=6cm,ymin=-10,domain=0:2,samples=3]
    // \addplot {x^3}: box 181.94031 x 125.24944bp (8cm-45pt x 6cm-45pt);
    // points 0 63.25694, 75.8077 69.58264, 151.6154 113.8625 with the data
    // origin 15.16245 right of the box's left side: x limits -0.2:2.2,
    // y limits -10 (explicit, not enlarged) to 9.8 (8 + 0.1 * 18).
    let p = render("\\begin{axis}[width=8cm,height=6cm,ymin=-10,domain=0:2,samples=3]\n\\addplot {x^3};\n\\end{axis}");
    assert!(only_tick_warning(&p), "{:?}", p.diagnostics);
    let k = 28.3468 / (CM * K);
    let f = match &p.items[0] {
        Item::PathStroke(s) => pts_of(&s.path),
        other => panic!("frame: {other:?}"),
    };
    let (x0, y0) = (f[0].x, f[0].y);
    assert!(close((f[2].x - x0) * k, 181.94031, 0.01), "{f:?}");
    assert!(close((y0 - f[2].y) * k, 125.24944, 0.01), "{f:?}");
    let pts = match &p.items[1] {
        Item::Group(g) => match &g.items[0] {
            Item::PathStroke(s) => pts_of(&s.path),
            other => panic!("plot: {other:?}"),
        },
        other => panic!("plot group: {other:?}"),
    };
    let want = [(15.16245, 63.25694), (90.97015, 69.58264), (166.77785, 113.8625)];
    assert_eq!(pts.len(), 3);
    for (q, (ex, ey)) in pts.iter().zip(want) {
        let (x, y) = ((q.x - x0) * k, (y0 - q.y) * k);
        assert!(close(x, ex, 0.01) && close(y, ey, 0.01), "({x}, {y}) vs ({ex}, {ey})");
    }
    // `no markers` drops the circles; `scale only axis` keeps the full size.
    let p = render("\\begin{axis}[no markers,scale only axis,width=4cm,height=3cm]\n\\addplot {x};\n\\end{axis}");
    assert_eq!(p.items.len(), 2, "frame + clipped plot: {:?}", p.items);
    let f = match &p.items[0] {
        Item::PathStroke(s) => pts_of(&s.path),
        other => panic!("frame: {other:?}"),
    };
    assert!(close(f[2].x - f[0].x, 4.0 * CM * K, 1e-6), "{f:?}");
    assert!(close(f[0].y - f[2].y, 3.0 * CM * K, 1e-6), "{f:?}");
}

#[test]
fn axis_without_expression_warns_and_keeps_the_frame() {
    let p = render("\\begin{axis}\n\\addplot coordinates {(0,0) (1,1)};\n\\end{axis}");
    assert_eq!(p.items.len(), 1, "{:?}", p.items);
    assert!(matches!(p.items[0], Item::PathStroke(_)), "{:?}", p.items);
    assert!(p.diagnostics.iter().any(|d| d.message.contains("coordinates")), "{:?}", p.diagnostics);
}

#[test]
fn axis_with_extra_content_warns_instead_of_silently_dropping_it() {
    // A \draw statement alongside a real \addplot must not vanish
    // silently -- the axis still draws the plot, but says so.
    let p = render("\\begin{axis}\n\\draw (0,0) -- (1,1);\n\\addplot{x^2};\n\\end{axis}");
    assert_eq!(p.items.len(), 2 + 50, "{:?} plot and its 25 marks still drawn", p.items);
    assert!(
        p.diagnostics.iter().any(|d| d.message.contains("besides \\addplot")),
        "{:?}",
        p.diagnostics
    );
}

#[test]
fn axis_with_extra_content_but_no_addplot_still_warns() {
    let p = render("\\begin{axis}\n\\node at (0,0) {hi};\n\\end{axis}");
    assert_eq!(p.items.len(), 1, "{:?} frame only", p.items);
    assert!(
        p.diagnostics.iter().any(|d| d.message.contains("besides \\addplot")),
        "{:?}",
        p.diagnostics
    );
}

#[test]
fn axis_with_a_singular_sample_gaps_instead_of_dropping_the_whole_curve() {
    // 1/x is undefined at x=0, which the default -5:5 domain samples
    // exactly (index 12 of 25): eval_inner rejects the resulting
    // infinity as Err("... is not a finite number"). The curve on
    // either side must still draw, split into two branches by a gap,
    // with a warning -- not vanish entirely.
    let p = render("\\begin{axis}\n\\addplot{1/x};\n\\end{axis}");
    assert!(
        p.diagnostics.iter().any(|d| d.message.contains("gapped")),
        "{:?}",
        p.diagnostics
    );
    assert_eq!(p.items.len(), 2 + 2 * 24, "frame, plot, 24 marks: {:?}", p.items);
    let plot = match &p.items[1] {
        Item::Group(g) => match &g.items[0] {
            Item::PathStroke(s) => s,
            other => panic!("plot: {other:?}"),
        },
        other => panic!("plot group: {other:?}"),
    };
    let moves = plot.path.commands().iter().filter(|c| matches!(c, PathCommand::MoveTo(_))).count();
    assert_eq!(moves, 2, "two branches either side of the x=0 gap: {:?}", plot.path.commands());
}
