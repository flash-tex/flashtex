//! Integration tests for the display-primitive model.

use flashtex_vector_graphics::diagram::{self, ArrowHead, KAPPA};
use flashtex_vector_graphics::display_list::ValidationError;
use flashtex_vector_graphics::json;
use flashtex_vector_graphics::pdf;
use flashtex_vector_graphics::*;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

fn approx(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

fn approx_point(a: Point, b: Point, eps: f64) -> bool {
    approx(a.x, b.x, eps) && approx(a.y, b.y, eps)
}

// ---------------------------------------------------------------------------
// Transform algebra
// ---------------------------------------------------------------------------

#[test]
fn transform_compose_invert_round_trip() {
    let samples = [
        Transform::IDENTITY,
        Transform::translate(12.5, -7.25),
        Transform::scale(2.0, 0.5),
        Transform::rotate(0.7),
        Transform::skew(0.3, -0.2),
        Transform::rotate_about(FRAC_PI_4, Point::new(100.0, 50.0)),
        Transform::new(0.6, 0.8, -0.8, 0.6, 30.0, 40.0),
    ];
    let points = [
        Point::ZERO,
        Point::new(1.0, 0.0),
        Point::new(-3.5, 7.0),
        Point::new(612.0, 792.0),
    ];
    for a in &samples {
        for b in &samples {
            let ab = a.then(b);
            let inv = ab.invert().expect("samples are invertible");
            for p in points {
                // Composition applies a then b.
                assert!(
                    approx_point(ab.apply(p), b.apply(a.apply(p)), 1e-9),
                    "{a} then {b} at {p:?}"
                );
                // Inverse undoes the composition in both orders.
                assert!(approx_point(inv.apply(ab.apply(p)), p, 1e-9));
                assert!(approx_point(ab.apply(inv.apply(p)), p, 1e-9));
            }
            // (ab)^-1 == b^-1 a^-1
            let expected = b.invert().unwrap().then(&a.invert().unwrap());
            assert!(inv.approx_eq(&expected, 1e-9));
            // then / pre agree.
            assert_eq!(a.then(b), b.pre(a));
        }
    }
}

#[test]
fn singular_transform_has_no_inverse() {
    assert_eq!(Transform::scale(0.0, 1.0).invert(), None);
    assert_eq!(Transform::new(1.0, 2.0, 2.0, 4.0, 0.0, 0.0).invert(), None);
}

#[test]
fn rotation_direction_is_clockwise_on_screen() {
    // y-down: rotating +x by 90° gives +y (down on screen).
    let p = Transform::rotate(FRAC_PI_2).apply(Point::new(1.0, 0.0));
    assert!(approx_point(p, Point::new(0.0, 1.0), 1e-12));
    assert!(!Transform::rotate(0.1).is_axis_aligned());
    assert!(Transform::scale(-1.0, 3.0).is_axis_aligned());
    assert!(approx(Transform::scale(2.0, 8.0).mean_scale(), 4.0, 1e-12));
}

// ---------------------------------------------------------------------------
// Paths: bounds and flattening accuracy
// ---------------------------------------------------------------------------

#[test]
fn circle_bounds_are_exact() {
    let c = diagram::circle(Point::new(100.0, 100.0), 40.0);
    let b = c.bounds().unwrap();
    assert!(approx(b.x, 60.0, 1e-9));
    assert!(approx(b.y, 60.0, 1e-9));
    assert!(approx(b.width, 80.0, 1e-9));
    assert!(approx(b.height, 80.0, 1e-9));
}

#[test]
fn circle_flattening_stays_within_tolerance() {
    // The 4-cubic circle itself deviates from a true circle by at most
    // ~2.7e-4 r; the flattening must add no more than `tol` on top.
    let bezier_error = 2.8e-4;
    for &(r, tol) in &[(100.0, 0.5), (100.0, 0.05), (10.0, 0.01), (500.0, 1.0)] {
        let center = Point::new(300.0, 300.0);
        let polylines = diagram::circle(center, r).flatten(tol);
        assert_eq!(polylines.len(), 1);
        let pl = &polylines[0];
        assert!(pl.closed);
        assert!(
            pl.points.len() >= 8,
            "r={r} tol={tol} points={}",
            pl.points.len()
        );
        // Every vertex lies on the Bézier (so within the Bézier's own error of
        // the circle) and every segment midpoint stays within `tol` of it.
        let n = pl.points.len();
        for i in 0..n {
            let a = pl.points[i];
            let b = pl.points[(i + 1) % n];
            let mid = a.lerp(b, 0.5);
            let d_vertex = (a.distance(center) - r).abs();
            let d_mid = (mid.distance(center) - r).abs();
            assert!(
                d_vertex <= bezier_error * r + 1e-9,
                "vertex off circle by {d_vertex} (r={r})"
            );
            assert!(
                d_mid <= tol + bezier_error * r,
                "midpoint off by {d_mid} > tol {tol} (r={r})"
            );
        }
    }
}

#[test]
fn flattening_is_deterministic_and_finer_for_smaller_tolerance() {
    let c = diagram::circle(Point::ZERO, 50.0);
    let coarse = c.flatten(1.0);
    let fine = c.flatten(0.01);
    assert_eq!(c.flatten(1.0), coarse);
    assert!(fine[0].points.len() > coarse[0].points.len());
}

#[test]
fn quadratic_flattening_accuracy() {
    // A quadratic arc with a known analytic form.
    let p0 = Point::new(0.0, 0.0);
    let c = Point::new(50.0, 100.0);
    let p2 = Point::new(100.0, 0.0);
    let mut path = Path::new();
    path.move_to(p0).quad_to(c, p2);
    let tol = 0.1;
    let pl = &path.flatten(tol)[0];
    // Sample the analytic curve densely and check it is within tol of the polyline.
    for k in 0..=1000 {
        let t = k as f64 / 1000.0;
        let mt = 1.0 - t;
        let q = Point::new(
            mt * mt * p0.x + 2.0 * mt * t * c.x + t * t * p2.x,
            mt * mt * p0.y + 2.0 * mt * t * c.y + t * t * p2.y,
        );
        let d = (1..pl.points.len())
            .map(|i| q.distance_to_segment(pl.points[i - 1], pl.points[i]))
            .fold(f64::INFINITY, f64::min);
        assert!(d <= tol + 1e-9, "t={t} d={d}");
    }
}

#[test]
fn kappa_matches_quarter_circle() {
    let expected = 4.0 / 3.0 * (FRAC_PI_4 / 2.0).tan();
    assert!(approx(KAPPA, expected, 1e-15));
}

// ---------------------------------------------------------------------------
// Clips
// ---------------------------------------------------------------------------

#[test]
fn nested_group_clips_intersect_in_device_space() {
    let mut inner = Group::new(ItemId(2));
    inner.transform = Transform::translate(10.0, 10.0);
    inner.clip = Some(Clip::Rect(Rect::new(0.0, 0.0, 50.0, 50.0)));
    inner
        .items
        .push(Item::rule(3, Rect::new(-100.0, -100.0, 300.0, 300.0)));

    let mut outer = Group::new(ItemId(1));
    outer.transform = Transform::translate(20.0, 0.0);
    outer.clip = Some(Clip::Rect(Rect::new(0.0, 0.0, 30.0, 100.0)));
    outer.items.push(Item::Group(inner));

    let mut list = DisplayList::new(Size::new(200.0, 200.0));
    list.items.push(Item::Group(outer));

    let device = list.flatten();
    assert_eq!(device.items.len(), 1);
    let item = &device.items[0];
    assert_eq!(item.clip.clips().len(), 2);
    // outer clip: x 20..50, y 0..100; inner clip: x 30..80, y 10..60.
    assert_eq!(
        item.clip.clip_rect(list.page_rect()),
        Some(Rect::new(30.0, 10.0, 20.0, 50.0))
    );
    assert_eq!(item.bounds(), Some(Rect::new(30.0, 10.0, 20.0, 50.0)));
    assert_eq!(list.bounds(), Some(Rect::new(30.0, 10.0, 20.0, 50.0)));
    assert_eq!(item.ancestors, vec![ItemId(1), ItemId(2)]);
}

#[test]
fn rotated_group_clip_becomes_path_clip_and_hits_correctly() {
    let mut g = Group::new(ItemId(1));
    g.transform = Transform::rotate_about(FRAC_PI_4, Point::new(100.0, 100.0));
    g.clip = Some(Clip::Rect(Rect::new(50.0, 50.0, 100.0, 100.0)));
    g.items
        .push(Item::rule(2, Rect::new(0.0, 0.0, 200.0, 200.0)));
    let mut list = DisplayList::new(Size::new(200.0, 200.0));
    list.items.push(Item::Group(g));
    let device = list.flatten();
    assert!(matches!(device.items[0].clip.clips()[0], Clip::Path { .. }));
    // A rotated square (diamond) centred at (100,100) with half-diagonal 50√2 ≈ 70.7.
    assert_eq!(device.hit(Point::new(100.0, 100.0)), vec![ItemId(2)]);
    assert_eq!(device.hit(Point::new(100.0, 165.0)), vec![ItemId(2)]);
    assert!(device.hit(Point::new(145.0, 145.0)).is_empty()); // corner of the unrotated square is outside the diamond
    assert!(device.hit(Point::new(100.0, 175.0)).is_empty());
}

// ---------------------------------------------------------------------------
// Hit-testing
// ---------------------------------------------------------------------------

fn sample_list() -> DisplayList {
    let mut list = DisplayList::new(Size::new(612.0, 792.0));
    // Bottom: a big rule.
    list.items.push(Item::Rule(Rule {
        id: ItemId(1),
        rect: Rect::new(0.0, 0.0, 300.0, 300.0),
        paint: Paint::opaque(Color::Rgb(0.9, 0.9, 0.9)),
        source: Some(SourceRange::new("main.tex", 0, 10)),
    }));
    // A group translated by (100, 100) with a clip and a stroke + circle fill.
    let mut g = Group::new(ItemId(2));
    g.transform = Transform::translate(100.0, 100.0);
    g.clip = Some(Clip::Rect(Rect::new(0.0, 0.0, 100.0, 100.0)));
    g.source = Some(SourceRange::new("main.tex", 20, 40));
    let mut line = Path::new();
    line.move_to(Point::new(0.0, 50.0))
        .line_to(Point::new(200.0, 50.0));
    g.items.push(Item::stroke(
        3,
        line,
        StrokeStyle::with_width(4.0),
        Paint::BLACK,
    ));
    g.items.push(Item::fill(
        4,
        diagram::circle(Point::new(50.0, 50.0), 20.0),
        Paint::opaque(Color::Cmyk(0.0, 1.0, 1.0, 0.0)),
    ));
    list.items.push(Item::Group(g));
    // An image rotated 90° about its own origin then moved.
    list.items.push(Item::Image(Image {
        id: ItemId(5),
        content_hash: "sha256:abc".into(),
        width_pt: 40.0,
        height_pt: 20.0,
        transform: Transform::rotate(FRAC_PI_2).then(&Transform::translate(400.0, 400.0)),
        alpha: 1.0,
        source: None,
    }));
    list
}

#[test]
fn hit_testing_respects_transforms_clips_and_order() {
    let list = sample_list();
    assert!(list.validate().is_empty());
    let device = list.flatten();

    // Inside the circle (device (150,150)): circle, stroke (y=150 is the centre line), rule.
    assert_eq!(
        device.hit(Point::new(150.0, 150.0)),
        vec![ItemId(4), ItemId(3), ItemId(1)]
    );
    // On the stroke but outside the circle: stroke half-width is 2.
    assert_eq!(
        device.hit(Point::new(120.0, 151.9)),
        vec![ItemId(3), ItemId(1)]
    );
    assert_eq!(device.hit(Point::new(120.0, 152.5)), vec![ItemId(1)]);
    // Tolerance widens the stroke hit.
    assert_eq!(
        device.hit_with_tolerance(Point::new(120.0, 152.5), 1.0),
        vec![ItemId(3), ItemId(1)]
    );
    // The stroke extends to device x=300 but the group clip stops it at x=200.
    assert_eq!(device.hit(Point::new(250.0, 150.0)), vec![ItemId(1)]);
    // Outside everything.
    assert!(device.hit(Point::new(350.0, 350.0)).is_empty());
    // Image: 40x20 box rotated 90° cw then translated: occupies x 380..400, y 400..440.
    assert_eq!(device.hit(Point::new(390.0, 420.0)), vec![ItemId(5)]);
    assert!(device.hit(Point::new(410.0, 420.0)).is_empty());
    assert!(device.hit(Point::new(390.0, 445.0)).is_empty());
}

#[test]
fn source_ranges_inherit_from_groups() {
    let device = sample_list().flatten();
    // The stroke has no source; the group's applies.
    let hit = device.source_at(Point::new(120.0, 151.0), 0.0).unwrap();
    assert_eq!(hit, &SourceRange::new("main.tex", 20, 40));
    // The rule has its own.
    let hit = device.source_at(Point::new(10.0, 10.0), 0.0).unwrap();
    assert_eq!(hit, &SourceRange::new("main.tex", 0, 10));
    assert_eq!(device.source_at(Point::new(390.0, 420.0), 0.0), None);
}

#[test]
fn even_odd_fill_hit_has_a_hole() {
    let mut p = Path::rect(Rect::new(0.0, 0.0, 100.0, 100.0));
    p.append(&Path::rect(Rect::new(25.0, 25.0, 50.0, 50.0)));
    let mut list = DisplayList::new(Size::new(100.0, 100.0));
    list.items.push(Item::PathFill(PathFill {
        id: ItemId(1),
        path: p,
        rule: FillRule::EvenOdd,
        paint: Paint::BLACK,
        pattern: None,
        source: None,
    }));
    assert!(list.hit(Point::new(50.0, 50.0)).is_empty());
    assert_eq!(list.hit(Point::new(10.0, 50.0)), vec![ItemId(1)]);
}

#[test]
fn rule_under_rotation_becomes_a_fill() {
    let mut g = Group::new(ItemId(1));
    g.transform = Transform::rotate(0.5);
    g.items.push(Item::rule(2, Rect::new(0.0, 0.0, 10.0, 10.0)));
    let mut list = DisplayList::new(Size::new(100.0, 100.0));
    list.items.push(Item::Group(g));
    let d = list.flatten();
    assert!(matches!(d.items[0].shape, DeviceShape::Fill { .. }));
    let mut g2 = Group::new(ItemId(1));
    g2.transform = Transform::scale(2.0, 2.0);
    g2.items
        .push(Item::rule(2, Rect::new(1.0, 1.0, 10.0, 10.0)));
    list.items[0] = Item::Group(g2);
    let d = list.flatten();
    assert_eq!(
        d.items[0].shape,
        DeviceShape::Rule {
            rect: Rect::new(2.0, 2.0, 20.0, 20.0),
            paint: Paint::BLACK
        }
    );
}

#[test]
fn stroke_width_and_alpha_scale_through_groups() {
    let mut inner = Group::new(ItemId(2));
    inner.opacity = 0.5;
    let mut line = Path::new();
    line.move_to(Point::ZERO).line_to(Point::new(10.0, 0.0));
    let mut style = StrokeStyle::with_width(2.0);
    style.dash = Some(Dash {
        array: vec![3.0, 1.0],
        phase: 0.5,
    });
    inner
        .items
        .push(Item::stroke(3, line, style, Paint::new(Color::BLACK, 0.5)));
    let mut outer = Group::new(ItemId(1));
    outer.transform = Transform::scale(3.0, 3.0);
    outer.items.push(Item::Group(inner));
    let mut list = DisplayList::new(Size::new(100.0, 100.0));
    list.items.push(Item::Group(outer));
    let d = list.flatten();
    match &d.items[0].shape {
        DeviceShape::Stroke { style, paint, path } => {
            assert!(approx(style.width, 6.0, 1e-12));
            assert_eq!(style.dash.as_ref().unwrap().array, vec![9.0, 3.0]);
            assert!(approx(style.dash.as_ref().unwrap().phase, 1.5, 1e-12));
            assert!(approx(paint.alpha, 0.25, 1e-12));
            assert_eq!(path.bounds().unwrap().width, 30.0);
        }
        other => panic!("unexpected {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[test]
fn validation_reports_problems() {
    let mut list = DisplayList::new(Size::new(0.0, 10.0));
    list.items
        .push(Item::rule(1, Rect::new(0.0, 0.0, 1.0, 1.0)));
    list.items
        .push(Item::rule(1, Rect::new(f64::NAN, 0.0, 1.0, 1.0)));
    let mut g = Group::new(ItemId(2));
    g.transform = Transform::scale(0.0, 0.0);
    g.source = Some(SourceRange::new("a.tex", 5, 2));
    list.items.push(Item::Group(g));
    let errors = list.validate();
    assert!(errors.contains(&ValidationError::NonPositivePageSize));
    assert!(errors.contains(&ValidationError::DuplicateId(ItemId(1))));
    assert!(errors.contains(&ValidationError::NonFiniteGeometry(ItemId(1))));
    assert!(errors.contains(&ValidationError::SingularTransform(ItemId(2))));
    assert!(errors.contains(&ValidationError::InvalidSourceRange(ItemId(2))));
    assert!(pdf::content_stream(&list).is_err());
}

// ---------------------------------------------------------------------------
// PDF fragments (golden strings)
// ---------------------------------------------------------------------------

#[test]
fn pdf_rule_golden_matches_pdf_crate_convention() {
    // Same rule crates/pdf would write for a fraction bar at baseline 100.5
    // with thickness 0.5: bottom edge at 792 - 100.5 = 691.5.
    let mut list = DisplayList::new(Size::new(612.0, 792.0));
    list.items
        .push(Item::rule(1, Rect::new(72.0, 100.0, 100.0, 0.5)));
    let frag = pdf::content_stream(&list).unwrap();
    assert_eq!(frag.content, "0 g\n72 691.5 100 0.5 re f\n");
    assert!(frag.ext_g_states.is_empty());
    assert!(frag.images.is_empty());
}

#[test]
fn pdf_stroked_path_golden() {
    let mut list = DisplayList::new(Size::new(200.0, 200.0));
    let mut p = Path::new();
    p.move_to(Point::new(10.0, 10.0))
        .line_to(Point::new(100.0, 50.0));
    list.items.push(Item::stroke(
        1,
        p,
        StrokeStyle::with_width(2.0),
        Paint::BLACK,
    ));
    let frag = pdf::content_stream(&list).unwrap();
    assert_eq!(frag.content, "0 G\n2 w\n10 190 m\n100 150 l\nS\n");
}

#[test]
fn pdf_group_transform_clip_alpha_and_image() {
    let mut list = DisplayList::new(Size::new(200.0, 300.0));
    let mut g = Group::new(ItemId(1));
    g.transform = Transform::translate(10.0, 20.0);
    g.clip = Some(Clip::Rect(Rect::new(0.0, 0.0, 50.0, 40.0)));
    g.opacity = 0.5;
    g.items.push(Item::Rule(Rule {
        id: ItemId(2),
        rect: Rect::new(5.0, 5.0, 10.0, 2.0),
        paint: Paint::opaque(Color::Rgb(1.0, 0.0, 0.0)),
        source: None,
    }));
    g.items.push(Item::Image(Image {
        id: ItemId(3),
        content_hash: "sha256:deadbeef".into(),
        width_pt: 30.0,
        height_pt: 15.0,
        transform: Transform::translate(1.0, 2.0),
        alpha: 1.0,
        source: None,
    }));
    list.items.push(Item::Group(g));
    // A second, top-level rule with the same alpha reuses the ExtGState; a
    // second use of the same image hash reuses the XObject name.
    list.items.push(Item::Rule(Rule {
        id: ItemId(4),
        rect: Rect::new(0.0, 0.0, 1.0, 1.0),
        paint: Paint::new(Color::Gray(0.25), 0.5),
        source: None,
    }));
    list.items.push(Item::Image(Image {
        id: ItemId(5),
        content_hash: "sha256:deadbeef".into(),
        width_pt: 30.0,
        height_pt: 15.0,
        transform: Transform::IDENTITY,
        alpha: 1.0,
        source: None,
    }));
    let frag = pdf::content_stream(&list).unwrap();
    let expected = concat!(
        "q\n",
        "1 0 0 1 10 -20 cm\n",
        "0 260 50 40 re W n\n",
        "/GS0 gs\n",
        "1 0 0 rg\n",
        "5 293 10 2 re f\n",
        "q\n",
        // unit square -> [0,30]x[0,15] (row 0 at top) -> +(1,2) -> flipped: y = 300 - 2 - 15 = 283
        "30 0 0 15 1 283 cm\n",
        "/Im0 Do\n",
        "Q\n",
        "Q\n",
        "/GS0 gs\n",
        "0.25 g\n",
        "0 299 1 1 re f\n",
        "q\n",
        "/GS1 gs\n",
        "30 0 0 15 0 285 cm\n",
        "/Im0 Do\n",
        "Q\n",
    );
    assert_eq!(frag.content, expected);
    assert_eq!(
        frag.ext_g_states,
        vec![
            pdf::ExtGState {
                name: "GS0".into(),
                alpha: 0.5
            },
            pdf::ExtGState {
                name: "GS1".into(),
                alpha: 1.0
            }
        ]
    );
    assert_eq!(
        frag.ext_g_states[0].dictionary(),
        "<< /Type /ExtGState /ca 0.5 /CA 0.5 >>"
    );
    assert_eq!(
        frag.images,
        vec![pdf::ImageResource {
            name: "Im0".into(),
            content_hash: "sha256:deadbeef".into()
        }]
    );
}

#[test]
fn pdf_rotated_group_conjugates_the_flip() {
    // A 90° rotation in y-down space must appear as a -90° rotation in PDF's
    // y-up space: F·T·F⁻¹ where F = [1 0 0 -1 0 H].
    let mut list = DisplayList::new(Size::new(100.0, 100.0));
    let mut g = Group::new(ItemId(1));
    g.transform = Transform::rotate(FRAC_PI_2);
    g.items.push(Item::rule(2, Rect::new(10.0, 0.0, 20.0, 5.0)));
    list.items.push(Item::Group(g));
    let frag = pdf::content_stream(&list).unwrap();
    // T = [0 1 -1 0 0 0]; F T F^-1 = [0 -1 1 0 -100 100].
    assert_eq!(
        frag.content,
        "q\n0 -1 1 0 -100 100 cm\n0 g\n10 95 20 5 re f\nQ\n"
    );
    // Verify numerically: the rule's top-left (10,0) rotates to (0,10) in
    // page space, i.e. PDF (0, 90). Applying the emitted cm to the emitted
    // flipped-local top-left (10, 100) must give the same point.
    let cm = Transform::new(0.0, -1.0, 1.0, 0.0, -100.0, 100.0);
    let p = cm.apply(Point::new(10.0, 100.0));
    assert!(approx_point(p, Point::new(0.0, 90.0), 1e-12));
}

#[test]
fn pdf_path_operators_and_stroke_state() {
    let mut list = DisplayList::new(Size::new(100.0, 100.0));
    let mut p = Path::new();
    p.move_to(Point::new(0.0, 0.0))
        .quad_to(Point::new(30.0, 0.0), Point::new(30.0, 30.0))
        .cubic_to(
            Point::new(30.0, 60.0),
            Point::new(0.0, 60.0),
            Point::new(0.0, 30.0),
        )
        .close();
    let style = StrokeStyle {
        width: 1.5,
        cap: LineCap::Round,
        join: LineJoin::Bevel,
        miter_limit: 4.0,
        dash: Some(Dash {
            array: vec![3.0, 2.0],
            phase: 1.0,
        }),
    };
    list.items.push(Item::stroke(
        1,
        p.clone(),
        style,
        Paint::opaque(Color::Cmyk(0.0, 0.0, 0.0, 1.0)),
    ));
    list.items.push(Item::PathFill(PathFill {
        id: ItemId(2),
        path: p,
        rule: FillRule::EvenOdd,
        paint: Paint::BLACK,
        pattern: None,
        source: None,
    }));
    let frag = pdf::content_stream(&list).unwrap();
    let expected = concat!(
        "0 0 0 1 K\n",
        "1.5 w\n",
        "1 J\n",
        "2 j\n",
        "4 M\n",
        "[3 2] 1 d\n",
        "0 100 m\n",
        "20 100 30 90 30 70 c\n",
        "30 40 0 40 0 70 c\n",
        "h\n",
        "S\n",
        "0 g\n",
        "0 100 m\n",
        "20 100 30 90 30 70 c\n",
        "30 40 0 40 0 70 c\n",
        "h\n",
        "f*\n",
    );
    assert_eq!(frag.content, expected);
}

// ---------------------------------------------------------------------------
// JSON
// ---------------------------------------------------------------------------

fn round_trip_list() -> DisplayList {
    let mut list = sample_list();
    let mut g = Group::new(ItemId(10));
    g.transform = Transform::rotate(0.1);
    g.clip = Some(Clip::path(
        diagram::circle(Point::new(1.0, 2.0), 3.0),
        FillRule::EvenOdd,
    ));
    g.opacity = 0.75;
    let mut style = StrokeStyle::with_width(0.3);
    style.cap = LineCap::Square;
    style.join = LineJoin::Round;
    style.dash = Some(Dash {
        array: vec![1.0, 0.5],
        phase: 0.25,
    });
    let mut p = Path::new();
    p.move_to(Point::new(1e-7, -0.0))
        .quad_to(Point::new(1.5, 2.5), Point::new(3.0, 4.0))
        .close();
    g.items.push(Item::PathStroke(PathStroke {
        id: ItemId(11),
        path: p,
        style,
        paint: Paint::new(Color::Gray(0.5), 0.5),
        source: Some(SourceRange::new("dir/with \"quotes\"\n.tex", 3, 3)),
    }));
    list.items.push(Item::Group(g));
    list
}

#[test]
fn json_round_trip_is_exact() {
    let list = round_trip_list();
    let text = json::write_display_list(&list);
    assert!(text.starts_with("{\"format\":\"flashtex-display-list\",\"version\":0,\"page_size\":{\"width_pt\":612,\"height_pt\":792},\"items\":["));
    let back = json::read_display_list(&text).expect("parses");
    assert_eq!(back, list);
    // Writing again yields the identical document.
    assert_eq!(json::write_display_list(&back), text);
}

#[test]
fn json_reader_rejects_bad_input() {
    assert!(json::read_display_list("{}").is_err());
    assert!(json::read_display_list("{\"format\":\"other\",\"version\":0,\"page_size\":{\"width_pt\":1,\"height_pt\":1},\"items\":[]}").is_err());
    assert!(json::read_display_list("{\"format\":\"flashtex-display-list\",\"version\":1,\"page_size\":{\"width_pt\":1,\"height_pt\":1},\"items\":[]}").is_err());
    let bad_kind = "{\"format\":\"flashtex-display-list\",\"version\":0,\"page_size\":{\"width_pt\":1,\"height_pt\":1},\"items\":[{\"kind\":\"text\",\"id\":1}]}";
    assert!(json::read_display_list(bad_kind).is_err());
    let ok = "{\"format\":\"flashtex-display-list\",\"version\":0,\"page_size\":{\"width_pt\":1,\"height_pt\":1},\"items\":[]}";
    assert_eq!(
        json::read_display_list(ok).unwrap(),
        DisplayList::new(Size::new(1.0, 1.0))
    );
}

#[test]
fn device_list_json_is_valid_and_flattened() {
    let list = sample_list();
    let text = json::write_device_list(&list.flatten());
    let v = json::parse(&text).expect("valid JSON");
    assert_eq!(
        v.get("format"),
        Some(&json::Value::String("flashtex-device-list".into()))
    );
    let items = match v.get("items") {
        Some(json::Value::Array(a)) => a,
        other => panic!("{other:?}"),
    };
    assert_eq!(items.len(), 4);
    // The stroke inside the translated group carries its ancestors, clip, and inherited source.
    let stroke = &items[1];
    assert_eq!(
        stroke.get("kind"),
        Some(&json::Value::String("path_stroke".into()))
    );
    assert_eq!(
        stroke.get("ancestors"),
        Some(&json::Value::Array(vec![json::Value::Number(2.0)]))
    );
    assert!(matches!(stroke.get("clips"), Some(json::Value::Array(c)) if c.len() == 1));
    assert_eq!(
        stroke.get("source").and_then(|s| s.get("start_byte")),
        Some(&json::Value::Number(20.0))
    );
}

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

#[test]
fn everything_is_deterministic() {
    let list = round_trip_list();
    let a = list.flatten();
    let b = list.clone().flatten();
    assert_eq!(a, b);
    assert_eq!(
        json::write_display_list(&list),
        json::write_display_list(&list)
    );
    assert_eq!(json::write_device_list(&a), json::write_device_list(&b));
    assert_eq!(
        pdf::content_stream(&list).unwrap(),
        pdf::content_stream(&list).unwrap()
    );
    assert_eq!(a.bounds(), b.bounds());
    for p in [
        Point::new(150.0, 150.0),
        Point::new(120.0, 151.0),
        Point::new(390.0, 420.0),
    ] {
        assert_eq!(a.hit(p), b.hit(p));
    }
}

// ---------------------------------------------------------------------------
// Diagram helpers
// ---------------------------------------------------------------------------

#[test]
fn diagram_helpers_produce_usable_items() {
    let arrow = diagram::arrow(
        Point::new(10.0, 10.0),
        Point::new(10.0, 110.0),
        ArrowHead::default(),
    );
    let ell = diagram::ellipse(Point::new(50.0, 50.0), 30.0, 10.0);
    let poly = diagram::polyline(
        &[Point::ZERO, Point::new(10.0, 0.0), Point::new(10.0, 10.0)],
        true,
    );
    let grid = diagram::grid(Rect::new(0.0, 0.0, 100.0, 100.0), 4, 4);
    let label = diagram::text_anchor_box(
        Point::new(50.0, 50.0),
        Size::new(30.0, 12.0),
        diagram::Anchor::Bottom,
    );
    let mut list = DisplayList::new(Size::new(200.0, 200.0));
    list.items.push(Item::stroke(
        1,
        arrow.shaft,
        StrokeStyle::default(),
        Paint::BLACK,
    ));
    list.items.push(Item::fill(2, arrow.head, Paint::BLACK));
    list.items
        .push(Item::fill(3, ell, Paint::opaque(Color::Rgb(0.0, 0.0, 1.0))));
    list.items
        .push(Item::stroke(4, poly, StrokeStyle::default(), Paint::BLACK));
    list.items.push(Item::stroke(
        5,
        grid,
        StrokeStyle::with_width(0.25),
        Paint::opaque(Color::Gray(0.8)),
    ));
    list.items.push(Item::rule(6, label));
    assert!(list.validate().is_empty());
    let d = list.flatten();
    assert_eq!(d.items.len(), 6);
    // Ellipse hit at its centre, arrow tip in the head, label box anchored above the point.
    assert!(d.hit(Point::new(50.0, 50.0)).contains(&ItemId(3)));
    assert!(d.hit(Point::new(10.0, 108.0)).contains(&ItemId(2)));
    assert!(d.hit(Point::new(50.0, 45.0)).contains(&ItemId(6)));
    assert!(!d.hit(Point::new(50.0, 55.0)).contains(&ItemId(6)));
    // Ellipse extremes are exact.
    let eb = d.items[2].geometry_bounds().unwrap();
    assert!(
        approx(eb.x, 20.0, 1e-9) && approx(eb.width, 60.0, 1e-9) && approx(eb.height, 20.0, 1e-9)
    );
    let _ = PI;
    assert!(!pdf::content_stream(&list).unwrap().content.is_empty());
}
