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
        ..Default::default()
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
        let mut reference = checker::reference(&pdf);
        checker::device_only(&mut reference);
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
            let colours = checker::host_colors(&page).0;
            for (r, (fill, _)) in expected.iter().zip(&colours) {
                let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
                assert_eq!(bits(fill), bits(&r.fill), "{name}: fill colour");
            }
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

/// With `color-spaces` and `line-state` accepted (spec §11.3, §11.4):
/// ICCBased colours go as FILL_COLOR_CS with the PDF's profile, constant
/// alpha as FILL_ALPHA, a spot colour as a Separation, stroked glyphs with
/// their line state, all in the PDF's numbers, and the page is complete.
const COLOURS: &str = "#set text(font: \"Libertinus Serif\")\n\
        #text(fill: rgb(\"#3366cc\"))[Blue] #text(fill: luma(40%))[grey] \
        #text(fill: cmyk(10%, 20%, 30%, 40%))[cmyk] \
        #text(fill: rgb(255, 0, 0, 128))[half red] \
        #rect(width: 1cm, height: 5mm, fill: rgb(0, 128, 0, 64), stroke: 0.4pt + rgb(\"#aa0000\")) \
        #text(stroke: 0.3pt + blue)[Stroked] \
        #box(rotate(20deg, text(stroke: 0.4pt + red)[Turned])) \
        #box(scale(150%, text(stroke: (paint: green, thickness: 0.3pt, dash: \"dashed\"))[Big])) \
        #text(fill: color.spot(\"PANTONE 300 C\", cmyk(100%, 44%, 0%, 0%)).tint(80%))[spot]";

/// Page 0 of `src` converted for a client with `caps`, and the checker's
/// reference for it.
fn convert_page(name: &str, src: &str, caps: ClientCaps) -> (Page, Vec<checker::RefPage>) {
    let fonts = Fonts::load(&FontOptions {
        paths: vec![font_dir().to_path_buf()],
        system: false,
    });
    let root = project(name, src);
    let world = HostWorld::new(&root, "main.typ", &fonts).unwrap();
    let doc = typst::compile::<PagedDocument>(&world)
        .output
        .unwrap_or_else(|e| panic!("{:?}", e.first().map(|d| &d.message)));
    let pdf = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()).unwrap();
    let reference = checker::reference(&pdf);
    let pp = pdfpos::derive(&doc, &[0]).unwrap();
    let out = convert::page(
        &world,
        &doc,
        0,
        &mut Tables::new(),
        caps,
        &[],
        Positions::Pdf(&pp[0]),
    )
    .unwrap();
    (
        Page::decode(StreamKind::Page, &out.body).unwrap(),
        reference,
    )
}

fn caps(colour: bool, ungated: bool) -> ClientCaps {
    ClientCaps {
        minor: 3,
        opentype_programs: true,
        program_refs: true,
        program_budget: None,
        color_spaces: colour,
        line_state: colour,
        image_data: false,
        image_budget: None,
        ungated,
    }
}

/// Without `color-spaces` and `line-state`: no 3.3 colour item, no
/// COLORSPACES; alpha, the spot colour and stroked text make the page
/// INCOMPLETE.
#[test]
fn colours_without_the_capabilities_are_incomplete() {
    let (page, _) = convert_page("subset-colour-none", COLOURS, caps(false, false));
    assert_eq!(page.flags & 1, 1);
    assert!(page.colorspaces.is_empty());
    assert!(!page.items.iter().any(|i| matches!(
        i,
        Item::FillColorCs { .. }
            | Item::StrokeColorCs { .. }
            | Item::FillAlpha(_)
            | Item::StrokeAlpha(_)
            | Item::LineState(_)
    )));
    for what in ["alpha", "separation colour", "stroked text"] {
        assert!(
            page.unsupported.iter().any(|u| u.starts_with(what)),
            "{what}: {:?}",
            page.unsupported
        );
    }
}

/// With the capabilities but no pixel gate rows (the default): the items
/// are sent and the page is INCOMPLETE, naming each pending class.
#[test]
fn colours_are_gated_until_their_pixel_rows_pass() {
    let (page, _) = convert_page("subset-colour-gated", COLOURS, caps(true, false));
    assert_eq!(page.flags & 1, 1);
    assert!(page
        .items
        .iter()
        .any(|i| matches!(i, Item::FillColorCs { .. })));
    for what in ["ICC colour", "alpha", "separation colour", "stroked text"] {
        assert!(
            page.unsupported
                .iter()
                .any(|u| u.starts_with(what) && u.contains("pixel gate row pending")),
            "{what}: {:?}",
            page.unsupported
        );
    }
}

/// Stroked text under a non-uniform scale has no one line width in
/// stream space (spec §11.4): INCOMPLETE.
#[test]
fn stroked_text_under_a_non_uniform_scale_is_incomplete() {
    let src = "#set text(font: \"Libertinus Serif\")\n\
        #scale(x: 150%, y: 100%, text(stroke: 0.3pt + blue)[Wide])";
    let (page, _) = convert_page("subset-stroke-skew", src, caps(true, true));
    assert_eq!(page.flags & 1, 1);
    assert!(
        page.unsupported.iter().any(|u| u.contains("non-uniform")),
        "{:?}",
        page.unsupported
    );
}

#[test]
fn colour_spaces_alpha_spot_and_stroked_text_from_the_pdf() {
    let src = COLOURS;
    let fonts = Fonts::load(&FontOptions {
        paths: vec![font_dir().to_path_buf()],
        system: false,
    });
    let root = project("subset-colour", src);
    let world = HostWorld::new(&root, "main.typ", &fonts).unwrap();
    let doc = typst::compile::<PagedDocument>(&world)
        .output
        .unwrap_or_else(|e| panic!("{:?}", e.first().map(|d| &d.message)));
    let pdf = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()).unwrap();
    let reference = checker::reference(&pdf);
    let caps = caps(true, true);
    let pp = pdfpos::derive(&doc, &[0]).unwrap();
    let mut tables = Tables::new();
    let out = convert::page(
        &world,
        &doc,
        0,
        &mut tables,
        caps,
        &[],
        Positions::Pdf(&pp[0]),
    )
    .unwrap();
    let page = Page::decode(StreamKind::Page, &out.body).unwrap();
    assert_eq!(page.flags & 1, 0, "INCOMPLETE: {:?}", page.unsupported);
    let has = |f: &dyn Fn(&Item) -> bool| page.items.iter().any(f);
    assert!(
        has(&|i| matches!(i, Item::FillColorCs { .. })),
        "FILL_COLOR_CS"
    );
    assert!(
        has(&|i| matches!(i, Item::FillAlpha(a) if *a < 1.0)),
        "FILL_ALPHA"
    );
    assert!(has(&|i| matches!(i, Item::LineState(_))), "LINE_STATE");
    assert!(
        has(&|i| matches!(i, Item::TextRender(r) if *r != 0)),
        "a stroking render mode"
    );
    use flashtex_display_list::page::ColorSpace;
    assert!(page
        .colorspaces
        .iter()
        .any(|c| matches!(c, ColorSpace::Icc { profile, .. } if !profile.is_empty())));
    assert!(page
        .colorspaces
        .iter()
        .any(|c| matches!(c, ColorSpace::Separation { name, .. } if name == "PANTONE 300 C")));
    // Every glyph's paint state (colours, spaces and profiles, alphas,
    // render mode, line state in stream space) and every path are the
    // PDF's.
    let rp = &reference[0];
    let colours = checker::host_glyph_paints(&page);
    assert_eq!(colours.len(), rp.glyphs.len());
    for (i, (r, h)) in rp.glyphs.iter().zip(&colours).enumerate() {
        assert_eq!(checker::glyph_paint_mismatch(h, r), None, "glyph {i}");
    }
    // The rotated and the scaled stroked runs: line widths in stream space.
    let widths: std::collections::BTreeSet<u64> = page
        .items
        .iter()
        .filter_map(|i| match i {
            Item::LineState(l) => Some(l.width.to_bits()),
            _ => None,
        })
        .collect();
    assert!(widths.len() >= 3, "{widths:?}");
    let hp = checker::host_paths(&page);
    assert_eq!(checker::unmatched_paths(&hp, &rp.paths), 0, "paths");
}
