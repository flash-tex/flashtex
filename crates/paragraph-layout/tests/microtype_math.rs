//! Inline formulas under pdfTeX font expansion and protrusion
//! ([`MicroItem::math`]): the formula's character nodes expand with the
//! line, its math glue stretches with the line's glue, and its first/last
//! character protrudes. Synthetic fonts; the pdflatex oracle for real
//! documents is `crates/render-pipeline/tests/microtype_math.rs`.

use std::rc::Rc;

use flashtex_microtype::pdftex::{ExpansionLimits, FontParams, expanded_width};
use flashtex_paragraph_layout::{
    FORCED_BREAK, FontId, Glue, GlyphRun, Item, LineBreakParams, MathNode, MicroItem, MicroMath, Microtype,
    layout_paragraph_microtype,
};

const SP: f64 = 65536.0;

fn expandable(quad: i32) -> Rc<FontParams> {
    let mut p = FontParams::plain(quad);
    p.expansion = ExpansionLimits::from_primitive(20, 20, 1);
    p.set_lpcode(b'(', 500);
    p.set_rpcode(b')', 500);
    Rc::new(p)
}

fn opaque_box(width_sp: i32) -> GlyphRun {
    GlyphRun {
        font: FontId::from_label("math"),
        size: 10.0,
        glyphs: Vec::new(),
        width: f64::from(width_sp) / SP,
        height: 7.0,
        depth: 2.0,
        source: 0..0,
    }
}

/// `( 1 <medmuskip> + <medmuskip> 2 )` with cmr-like widths.
fn formula(p: &Rc<FontParams>) -> MicroMath {
    let ch = |code: u8, w: f64| MathNode::Char { params: Some(p.clone()), code, width: (w * SP) as i32 };
    let med = MathNode::Glue { width: (2.22 * SP) as i32, stretch: (1.11 * SP) as i32, shrink: (2.22 * SP) as i32 };
    MicroMath {
        nodes: vec![
            ch(b'(', 3.89),
            ch(b'1', 5.0),
            med.clone(),
            ch(b'+', 7.78),
            med,
            ch(b'2', 5.0),
            ch(b')', 3.89),
        ],
    }
}

fn params(width: f64) -> LineBreakParams {
    LineBreakParams::article_12pt_letter_1in().with_width(width)
}

/// Words `w` wide separated by interword glue, the formula in the middle.
fn paragraph(p: &Rc<FontParams>, with_math: bool) -> (Vec<Item>, Microtype) {
    let f = formula(p);
    let fw: i32 = f
        .nodes
        .iter()
        .map(|n| match n {
            MathNode::Char { width, .. } | MathNode::Glue { width, .. } => *width,
            _ => 0,
        })
        .sum();
    let space = Glue { width: 3.33, stretch: 1.67, shrink: 1.11, ..Glue::fixed(0.0) };
    let mut items = Vec::new();
    let mut micro = Vec::new();
    for k in 0..3 {
        if k > 0 {
            items.push(Item::Glue(space.clone()));
            micro.push(MicroItem::default());
        }
        items.push(Item::Box(opaque_box((20.0 * SP) as i32)));
        micro.push(MicroItem::default());
    }
    items.push(Item::Glue(space.clone()));
    micro.push(MicroItem::default());
    items.push(Item::Box(opaque_box(fw)));
    micro.push(MicroItem { math: with_math.then_some(f), ..MicroItem::default() });
    items.push(Item::Glue(Glue::fil()));
    micro.push(MicroItem::default());
    items.push(Item::penalty(FORCED_BREAK));
    micro.push(MicroItem::default());
    (items, Microtype { protrude_chars: 2, adjust_spacing: 2, items: micro })
}

#[test]
fn formula_characters_expand_and_math_glue_stretches_with_the_line() {
    let p = expandable((10.0 * SP) as i32);
    let (mut items, mut mt) = paragraph(&p, true);
    // A justified single line: replace \parfillskip by nothing so the line
    // must stretch to the measure.
    items.remove(items.len() - 2);
    mt.items.remove(mt.items.len() - 2);
    let (lines, micro) = layout_paragraph_microtype(&items, &params(110.0), &mt).expect("layout");
    assert_eq!(lines.lines.len(), 1);
    let line = &lines.lines[0];
    let m = &micro[0];
    assert!(m.expand_ratio > 0, "the line stretches its fonts: {}", m.expand_ratio);
    let math_run = line.runs.last().expect("formula run");
    let exp = m.expansion.last().expect("formula expansion");
    assert_eq!(math_run.glyphs.len(), 7);
    assert_eq!(exp.len(), 7);
    // Every character node expanded by the same amount, the glue not at all.
    let e = exp[0];
    assert!(e > 0);
    for k in [0, 1, 3, 5, 6] {
        assert_eq!(exp[k], e, "node {k}");
    }
    assert_eq!((exp[2], exp[4]), (0, 0));
    // Node positions tile the formula's set width.
    let mut x = 0.0;
    for g in &math_run.glyphs {
        assert!((g.x_offset - x).abs() < 1e-9);
        x += g.advance;
    }
    assert!((x - math_run.width).abs() < 1e-9);
    // Characters advance by their expanded width; the math glue is set wider
    // than its natural 2.22pt as the line stretches.
    let one = expanded_width((5.0 * SP) as i32, e);
    assert_eq!((math_run.glyphs[1].advance * SP).round() as i32, one);
    assert!(math_run.glyphs[2].advance > 2.22 + 1e-6, "{}", math_run.glyphs[2].advance);
    // The right parenthesis ends the line: it protrudes half a quad.
    assert_eq!(m.right_margin_kern, -((5.0 * SP) as i32));
    assert!((line.runs.last().unwrap().x + math_run.width - (110.0 + 5.0)).abs() < 2.0 / SP);
}

#[test]
fn an_opaque_formula_box_neither_expands_nor_protrudes() {
    let p = expandable((10.0 * SP) as i32);
    let (mut items, mut mt) = paragraph(&p, false);
    items.remove(items.len() - 2);
    mt.items.remove(mt.items.len() - 2);
    let (lines, micro) = layout_paragraph_microtype(&items, &params(110.0), &mt).expect("layout");
    assert_eq!(micro[0].right_margin_kern, 0);
    assert!(lines.lines[0].runs.last().unwrap().glyphs.is_empty());
}

#[test]
fn formula_nodes_that_disagree_with_the_box_width_are_ignored() {
    let p = expandable((10.0 * SP) as i32);
    let (mut items, mut mt) = paragraph(&p, true);
    let i = items.iter().rposition(|it| matches!(it, Item::Box(_))).unwrap();
    if let Item::Box(b) = &mut items[i] {
        b.width += 1.0;
    }
    items.remove(items.len() - 2);
    mt.items.remove(mt.items.len() - 2);
    let (lines, micro) = layout_paragraph_microtype(&items, &params(111.0), &mt).expect("layout");
    assert_eq!(micro[0].right_margin_kern, 0);
    assert!(lines.lines[0].runs.last().unwrap().glyphs.is_empty());
}
