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

/// [`ApproxMeasurer`] that also lays out math: every formula is a 10 x 7 +
/// 2 pt box, so node geometry around one is exact to check.
struct MathBoxMeasurer;

impl TextMeasurer for MathBoxMeasurer {
    fn measure(&self, text: &str, style: &TextStyle) -> TextMetrics {
        ApproxMeasurer.measure(text, style)
    }

    fn measure_math(&self, _math: &NodeMath, _style: &TextStyle) -> Option<TextMetrics> {
        Some(TextMetrics { width_pt: 10.0, height_pt: 7.0, depth_pt: 2.0 })
    }
}

fn render_doc(doc: &str, measurer: &dyn TextMeasurer) -> Picture {
    let pics = find_pictures(doc);
    Tikz::new(10.0).render(doc, &pics[0], measurer)
}

#[test]
fn node_math_is_its_own_piece_with_its_source() {
    let doc = "\\begin{document}\n\\begin{tikzpicture}\n\\node[draw] at (0,0) {Speed $\\vec v_0$ here};\n\\end{tikzpicture}\n";
    let p = render_doc(doc, &MathBoxMeasurer);
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    let texts: Vec<(&str, Option<&str>)> = p.texts.iter().map(|t| (t.text.as_str(), t.math.as_ref().map(|m| m.tex.as_str()))).collect();
    assert_eq!(texts, [("Speed", None), ("v0", Some("\\vec v_0")), ("here", None)]);
    // The formula's source is the `$...$` as written.
    let (a, b) = p.texts[1].math.as_ref().and_then(|m| m.source).expect("written formula has a source");
    assert_eq!(&doc[a..b], "$\\vec v_0$");
    // Pieces sit side by side on one baseline: "Speed " (6 chars at 5pt),
    // the 10pt formula, then " " before "here".
    let x = |i: usize| p.texts[i].transform.e;
    assert!(close(x(1) - x(0), 30.0 * K, 1e-6), "{} {}", x(0), x(1));
    assert!(close(x(2) - x(1), 15.0 * K, 1e-6), "{} {}", x(1), x(2));
    assert!(p.texts.iter().all(|t| t.transform.f == p.texts[0].transform.f));
    // The box holds the formula's height and depth: 7 + 2 pt plus the
    // inner sep (0.3333em) twice.
    let s = strokes(&p);
    let r = s[0].path.bounds().expect("border");
    assert!(close(r.height, (9.0 + 2.0 * 3.333) * K, 1e-3), "{r:?}");
    assert!(close(r.width, (30.0 + 10.0 + 25.0 + 2.0 * 3.333) * K, 1e-3), "{r:?}");
}

#[test]
fn node_math_built_by_foreach_has_no_source() {
    let doc = "\\begin{tikzpicture}\n\\foreach \\x in {1,2} \\node at (\\x,0) {$\\x$};\n\\end{tikzpicture}\n";
    let p = render_doc(doc, &MathBoxMeasurer);
    let maths: Vec<(&str, Option<(usize, usize)>)> = p.texts.iter().filter_map(|t| t.math.as_ref()).map(|m| (m.tex.as_str(), m.source)).collect();
    assert_eq!(maths, [("1", None), ("2", None)]);
}

#[test]
fn node_math_without_layout_is_italic_text_and_reported() {
    let p = render(r"\node at (0,0) {$\alpha_1 + x^2$};");
    assert_eq!(p.texts.len(), 1);
    assert_eq!(p.texts[0].text, "1+x2");
    assert!(p.texts[0].style.italic);
    assert_eq!(p.diagnostics.len(), 1, "{:?}", p.diagnostics);
    assert!(p.diagnostics[0].message.contains("set as italic text"), "{:?}", p.diagnostics);
    // Text after a formula keeps the node's own (upright) font.
    let p = render(r"\node at (0,0) {$x$ and $y$};");
    let styles: Vec<(&str, bool)> = p.texts.iter().map(|t| (t.text.as_str(), t.style.italic)).collect();
    assert_eq!(styles, [("x", true), ("and", false), ("y", true)]);
}

#[test]
fn node_inner_sep_em_is_the_surrounding_font() {
    // `font=` selects the font inside the text box only; the .3333em inner
    // sep is evaluated outside it (pdflatex: a `font=\footnotesize` node's
    // text sits 0.667pt further in than an 8pt em would put it).
    let small = render(r"\node[draw,font=\footnotesize] at (0,0) {AB};");
    let r = strokes(&small)[0].path.bounds().expect("border");
    // "AB" at 8pt: 2 x 4pt; inner sep 3.333pt each side.
    assert!(close(r.width, (8.0 + 2.0 * 3.333) * K, 1e-3), "{r:?}");
}
