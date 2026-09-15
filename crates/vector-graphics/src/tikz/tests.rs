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
