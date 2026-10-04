//! A CI-sized subset of the positions gate over Typst's test suite
//! (`examples/positions_suite.rs` runs all of it; it needs the Typst and
//! typst-dev-assets checkouts): snippets of the kinds that once broke the
//! alignment, written here without external assets, through the host's own
//! conversion and compared with the independent checker.

mod checker;
mod common;

use common::*;
use flashtex_display_list::page::{Item, Page, StreamKind};
use flashtex_typst_host::convert::{self, ClientCaps, Positions, Tables};
use flashtex_typst_host::pdfpos;
use flashtex_typst_host::world::{FontOptions, Fonts, HostWorld};
use typst_layout::PagedDocument;

const SNIPPETS: &[(&str, &str)] = &[
    (
        "gradient-text",
        "#set text(font: \"Libertinus Serif\")\nSolid, #text(fill: gradient.linear(red, blue))[gradient], solid again.",
    ),
    (
        "svg-text",
        "#set text(font: \"Libertinus Serif\")\nBefore #image(bytes(\"<svg xmlns='http://www.w3.org/2000/svg' width='80' height='20'><text x='2' y='15' font-family='Libertinus Serif' font-size='12'>In the SVG</text></svg>\"), width: 3cm) after.",
    ),
    (
        "negative-rect",
        "#rect(width: -20pt, height: -10pt, fill: red)\n#rect(width: 20pt, height: 10pt, stroke: 0.4pt)\nText.",
    ),
    (
        "clip-and-transform",
        "#set text(font: \"Libertinus Serif\")\n#box(clip: true, width: 2cm, height: 1cm, stroke: 0.3pt)[#lorem(30)]\n#rotate(23deg)[Rotated] #scale(x: 140%)[Scaled]",
    ),
    (
        "stroke-and-tiling",
        "#set text(font: \"Libertinus Serif\")\n#text(stroke: 0.3pt + blue)[Stroked] #rect(width: 2cm, height: 1cm, fill: tiling(size: (5pt, 5pt))[#circle(radius: 1pt)])\n#line(length: 3cm, stroke: (dash: \"dashed\"))",
    ),
];

#[test]
fn suite_subset_positions_and_paths_equal_the_pdf() {
    let fonts = Fonts::load(&FontOptions {
        paths: vec![font_dir().to_path_buf()],
        system: false,
    });
    let caps = ClientCaps {
        minor: 3,
        opentype_programs: true,
        program_refs: true,
        program_budget: None,
    };
    let sp = |v: f64| (v * 65_781.76).round() as i32;
    let (mut glyphs, mut paths) = (0, 0);
    for (name, src) in SNIPPETS {
        let root = project(&format!("subset-{name}"), src);
        let world = HostWorld::new(&root, "main.typ", &fonts).unwrap();
        let doc = typst::compile::<PagedDocument>(&world)
            .output
            .unwrap_or_else(|e| panic!("{name}: {:?}", e.first().map(|d| &d.message)));
        let pdf = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()).unwrap();
        let reference = checker::reference(&pdf);
        let mut tables = Tables::new();
        for (i, rp) in reference.iter().enumerate() {
            let pp = pdfpos::derive(&doc, &[i]).unwrap();
            let out = convert::page(
                &world,
                &doc,
                i,
                &mut tables,
                caps,
                &[],
                Positions::Pdf(&pp[0]),
            )
            .unwrap();
            let page = Page::decode(StreamKind::Page, &out.body).unwrap();
            assert!(
                !page
                    .unsupported
                    .iter()
                    .any(|u| u.starts_with("glyph positions") || u.starts_with("paths:")),
                "{name} page {i}: {:?}",
                page.unsupported
            );
            assert_eq!(
                page.pdf_box.map(f64::to_bits),
                rp.media_box.map(f64::to_bits)
            );
            let mut evs = Vec::new();
            let tp = &doc.pages()[i];
            let ts = typst::layout::Transform::translate(tp.bleed.left, tp.bleed.top);
            checker::events(&tp.frame, ts, rp.media_box[3], &mut evs);
            let expected = checker::expected(&evs, &rp.glyphs)
                .unwrap_or_else(|e| panic!("{name} page {i}: {e}"));
            let mut lin = [0.0; 4];
            let mut drawn = vec![];
            for it in &page.items {
                match it {
                    Item::Matrix(m) => {
                        let m = page.matrix(*m);
                        lin = [m[0], m[1], m[2], m[3]];
                    }
                    Item::Glyph { x, y, .. } => drawn.push((*x, *y, lin)),
                    _ => {}
                }
            }
            assert_eq!(drawn.len(), expected.len(), "{name} page {i}: glyphs drawn");
            let h = page.pdf_box[3];
            for ((r, o), (x, y, l)) in expected.iter().zip(&page.origins).zip(&drawn) {
                assert_eq!(
                    r.origin.map(f64::to_bits),
                    o.map(f64::to_bits),
                    "{name}: origin"
                );
                assert_eq!(
                    r.matrix.map(f64::to_bits),
                    l.map(f64::to_bits),
                    "{name}: matrix"
                );
                assert_eq!(
                    (*x, *y),
                    (sp(r.origin[0]), sp(h - r.origin[1])),
                    "{name}: sp"
                );
            }
            let hp = checker::host_paths(&page);
            assert_eq!(
                checker::unmatched_paths(&hp, &rp.paths),
                0,
                "{name} page {i}: paths"
            );
            glyphs += drawn.len();
            paths += hp.len();
        }
    }
    eprintln!(
        "suite subset: {} snippets, {glyphs} glyphs, {paths} paths, 0 mismatches",
        SNIPPETS.len()
    );
    assert!(glyphs > 100 && paths >= 5, "{glyphs} glyphs, {paths} paths");
}
