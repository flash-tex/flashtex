//! Basic `\foreach` coverage: item counts and centres for plain lists,
//! slash-separated pairs, `...` ranges and `[count=\i]`.
//!
//! The render-pipeline fixtures `16-foreach-dots.tex` / `17-foreach-pairs.tex`
//! exercise these shapes end to end but assert nothing; these tests pin the
//! geometry directly (1 TikZ unit = 1cm, radii in TeX pt, converted with
//! K = 72/72.27 to the crate's PDF-point picture space).

use flashtex_vector_graphics::item::Item;
use flashtex_vector_graphics::tikz::{ApproxMeasurer, Picture, Tikz};

const K: f64 = 72.0 / 72.27;
const CM: f64 = 72.27 / 2.54;
/// One cm in picture points.
const STEP: f64 = CM * K;
/// Radius of `circle (2pt)` in picture points.
const R: f64 = 2.0 * K;

fn render(body: &str) -> Picture {
    Tikz::new(10.0).render_body("", body, &ApproxMeasurer)
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// Centres of the fill items, in paint order, from exact path bounds.
fn fill_centres(p: &Picture) -> Vec<(f64, f64)> {
    p.items
        .iter()
        .filter_map(|i| {
            if let Item::PathFill(f) = i {
                let b = f.path.bounds().unwrap();
                Some((b.x + b.width / 2.0, b.y + b.height / 2.0))
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn foreach_plain_list_item_count_and_centres() {
    let p = render(r"\foreach \x in {0,1,2} \fill (\x,0) circle (2pt);");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    assert_eq!(p.items.len(), 3);
    let c = fill_centres(&p);
    assert_eq!(c.len(), 3);
    for (i, (x, y)) in c.iter().enumerate() {
        assert!(
            close(*x, R + i as f64 * STEP, 1e-6),
            "item {i}: x={x}, expected {}",
            R + i as f64 * STEP
        );
        assert!(close(*y, R, 1e-6), "item {i}: y={y}, expected {R}");
    }
    // Span is 2cm plus one radius on each side.
    assert!(close(p.width_bp, 2.0 * STEP + 2.0 * R, 1e-6), "{}", p.width_bp);
}

#[test]
fn foreach_slash_pairs_item_count_and_centres() {
    let p = render(r"\foreach \x/\y in {0/1,1/2} \fill (\x,\y) circle (2pt);");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    assert_eq!(p.items.len(), 2);
    let c = fill_centres(&p);
    assert_eq!(c.len(), 2);
    // y grows down in picture space: (\x=0,\y=1) sits below (\x=1,\y=2).
    assert!(close(c[0].0, R, 1e-6) && close(c[0].1, R + STEP, 1e-6), "{:?}", c[0]);
    assert!(close(c[1].0, R + STEP, 1e-6) && close(c[1].1, R, 1e-6), "{:?}", c[1]);
}

#[test]
fn foreach_dots_range_item_count_and_spacing() {
    let p = render(r"\foreach \x in {0,...,4} \fill (\x,0) circle (2pt);");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    assert_eq!(p.items.len(), 5);
    let c = fill_centres(&p);
    assert_eq!(c.len(), 5);
    for (i, (x, y)) in c.iter().enumerate() {
        assert!(
            close(*x, R + i as f64 * STEP, 1e-6),
            "item {i}: x={x}, expected {}",
            R + i as f64 * STEP
        );
        assert!(close(*y, R, 1e-6), "item {i}: y={y}, expected {R}");
    }
    assert!(close(p.width_bp, 4.0 * STEP + 2.0 * R, 1e-6), "{}", p.width_bp);
}

#[test]
fn foreach_count_option_is_one_based() {
    // List values (5,6,7) are far from the count (1,2,3); driving the radius
    // from `\i` pins the count absolutely instead of bbox-relatively.
    let p = render(r"\foreach \x [count=\i] in {5,6,7} \fill (\x,0) circle (\i pt);");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    let fills: Vec<_> = p
        .items
        .iter()
        .filter_map(|i| if let Item::PathFill(f) = i { Some(f) } else { None })
        .collect();
    assert_eq!(fills.len(), 3);
    for (k, f) in fills.iter().enumerate() {
        let b = f.path.bounds().unwrap();
        let expected = 2.0 * (k as f64 + 1.0) * K;
        assert!(close(b.width, expected, 1e-9), "item {k}: w={}, expected {expected}", b.width);
        assert!(close(b.height, expected, 1e-9), "item {k}: h={}, expected {expected}", b.height);
    }
}
