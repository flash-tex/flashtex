//! Sample `.typ` documents compiled end to end through the host process,
//! checked against the pinned `typst` itself (DESIGN.md §15: Typst's own
//! output is the oracle) -- and the **positions checker**, T1's
//! correctness gate (§15.10):
//!
//! 1. `DONE.pdf` is byte-identical to `typst_pdf::pdf` of the same document
//!    compiled in this process by an independent `World`.
//! 2. The display list has the oracle's pages and the oracle's glyphs (font,
//!    glyph id, in painting order).
//! 3. Every glyph's origin (ORIGINS) and glyph matrix (MATRIX), and every
//!    page's box, are **bit-identical** to what the independent checker
//!    (`tests/checker`) computes from that PDF as the reference viewer does
//!    (spec §4.2, §11.2), and each GLYPH's x, y is that origin rounded to sp.

mod checker;
mod common;
mod oracle_world;

use common::*;
use flashtex_display_list::client::Event;
use flashtex_display_list::kind;
use flashtex_display_list::page::{Item, Page};

/// Glyph ids of a frame in painting order, the typst-pdf walk.
fn oracle_glyphs(frame: &typst::layout::Frame, out: &mut Vec<u16>) {
    use typst::layout::FrameItem;
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(g) => oracle_glyphs(&g.frame, out),
            FrameItem::Text(t) => out.extend(t.glyphs.iter().map(|g| g.id)),
            _ => {}
        }
    }
}

/// (x, y, glyph matrix) of every GLYPH item, with the MATRIX in effect.
pub fn drawn(p: &Page) -> Vec<(u16, i32, i32, [f64; 4])> {
    let mut lin = [0.0; 4];
    let mut out = vec![];
    for it in &p.items {
        match it {
            Item::Matrix(m) => {
                let m = p.matrix(*m);
                lin = [m[0], m[1], m[2], m[3]];
            }
            Item::Glyph { code, x, y, .. } => out.push((*code, *x, *y, lin)),
            _ => {}
        }
    }
    out
}

pub struct Report {
    pub glyphs: usize,
    pub paths: usize,
    pub mismatches: usize,
}

pub fn check_doc(name: &str, source: &str) -> Report {
    let host = HostProc::start(&format!("oracle-{name}"));
    let root = project(&format!("oracle-{name}"), source);
    let out = scratch(&format!("out-{name}"));

    // Through the host.
    let mut c = host.connect();
    let (k, _) = c.hello(3, 3);
    assert_eq!(k, kind::HELLO);
    let extra = format!(
        r#""output_dir":{:?},"font_formats":["opentype"],"export":true"#,
        out.to_string_lossy()
    );
    c.send(kind::COMPILE, &compile_json(1, &root, "main.typ", &extra));
    let frames = c.until_done();
    let done = json_of(&frames.last().unwrap().1);
    assert_eq!(done.str_field("status"), Some("ok"), "{name}: {done}");
    let host_pdf = std::fs::read(done.str_field("pdf").unwrap()).unwrap();

    // The oracle.
    let ow = oracle_world::OracleWorld::new(&root, "main.typ", font_dir());
    let doc = ow.compile();
    let oracle_pdf = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()).unwrap();
    assert!(
        host_pdf == oracle_pdf,
        "{name}: DONE.pdf differs from typst-pdf's export ({} vs {} bytes)",
        host_pdf.len(),
        oracle_pdf.len()
    );
    let mut reference = checker::reference(&oracle_pdf);
    checker::device_only(&mut reference);

    let pages: Vec<Page> = frames
        .iter()
        .filter(|(k, _)| *k == kind::PAGE)
        .map(|(_, b)| {
            match flashtex_display_list::client::decode_event(kind::PAGE, b.clone()).unwrap() {
                Event::Page(p) => p,
                _ => unreachable!(),
            }
        })
        .collect();
    assert_eq!(pages.len(), doc.pages().len(), "{name}: page count");
    assert_eq!(reference.len(), pages.len(), "{name}: pages in the PDF");
    let mut rep = Report {
        glyphs: 0,
        paths: 0,
        mismatches: 0,
    };
    let sp = |v: f64| (v * 65_781.76).round() as i32;
    for (i, (p, r)) in pages.iter().zip(&reference).enumerate() {
        assert_eq!(p.index as usize, i);
        assert_eq!(
            p.flags & 1,
            0,
            "{name} page {i} INCOMPLETE: {:?}",
            p.unsupported
        );
        assert_eq!(
            p.pdf_box.map(f64::to_bits),
            r.media_box.map(f64::to_bits),
            "{name} page {i}: box {:?} vs the PDF's {:?}",
            p.pdf_box,
            r.media_box
        );
        assert_eq!(p.width, sp(r.media_box[2]));
        assert_eq!(p.height, sp(r.media_box[3]));
        let mut ids = vec![];
        oracle_glyphs(&doc.pages()[i].frame, &mut ids);
        let g = drawn(p);
        assert_eq!(g.len(), ids.len(), "{name} page {i}: glyph count");
        assert_eq!(
            g.len(),
            r.glyphs.len(),
            "{name} page {i}: glyphs in the PDF"
        );
        assert_eq!(p.origins.len(), g.len());
        let h = p.pdf_box[3];
        // The whole paint state: colours, spaces, alphas, render mode,
        // line state.
        let colours = checker::host_glyph_paints(p);
        assert_eq!(colours.len(), g.len());
        for (j, (rg, h)) in r.glyphs.iter().zip(&colours).enumerate() {
            if let Some(d) = checker::glyph_paint_mismatch(h, rg) {
                if rep.mismatches < 5 {
                    eprintln!("{name} page {i} glyph {j}: {d}");
                }
                rep.mismatches += 1;
            }
        }
        for (j, ((d, o), (rg, id))) in g
            .iter()
            .zip(&p.origins)
            .zip(r.glyphs.iter().zip(&ids))
            .enumerate()
        {
            assert_eq!(d.0, *id, "{name} page {i} glyph {j}: glyph id");
            let ok = o.map(f64::to_bits) == rg.origin.map(f64::to_bits)
                && d.3.map(f64::to_bits) == rg.matrix.map(f64::to_bits)
                && d.1 == sp(rg.origin[0])
                && d.2 == sp(h - rg.origin[1]);
            if !ok {
                if rep.mismatches < 5 {
                    eprintln!(
                        "{name} page {i} glyph {j}: host {o:?} {:?} ({}, {}), PDF {:?} {:?}",
                        d.3, d.1, d.2, rg.origin, rg.matrix
                    );
                }
                rep.mismatches += 1;
            }
        }
        rep.glyphs += g.len();
        // Paths and clips: the PDF's own numbers (spec §4.4).
        let hp = checker::host_paths(p);
        let missing = checker::unmatched_paths(&hp, &r.paths);
        if missing > 0 {
            eprintln!(
                "{name} page {i}: {missing} of {} paths are not the PDF's",
                hp.len()
            );
        }
        rep.paths += hp.len();
        rep.mismatches += missing;
    }
    eprintln!(
        "{name}: {} pages, {} glyphs, {} paths, {} position mismatches; DONE.pdf == typst-pdf ({} bytes)",
        pages.len(),
        rep.glyphs,
        rep.paths,
        rep.mismatches,
        host_pdf.len()
    );
    assert_eq!(rep.mismatches, 0, "{name}: positions differ from the PDF's");
    rep
}

fn check(name: &str) -> Report {
    check_doc(name, &fixture(name))
}

#[test]
fn text_matches_typst() {
    let r = check("text.typ");
    assert!(r.glyphs > 500);
}

#[test]
fn math_matches_typst() {
    check("math.typ");
}

#[test]
fn shapes_match_typst() {
    check("shapes.typ");
}

#[test]
fn links_match_typst() {
    check("links.typ");
}

/// Transformed text: rotation, scaling, skew, sub- and superscripts, sizes,
/// tracking, a page with bleed. The glyph matrices and origins come from
/// the PDF, not from the frame.
#[test]
fn transformed_text_matches_typst() {
    check("xform.typ");
}

/// The checker discriminates: Typst's frame positions (what T0 sent) are
/// not the PDF's, and it says so.
#[test]
fn the_checker_rejects_frame_positions() {
    use flashtex_typst_host::convert::{self, ClientCaps, Positions, Tables};
    use flashtex_typst_host::world::{FontOptions, Fonts, HostWorld};
    let root = project("frame-pos", &fixture("text.typ"));
    let fonts = Fonts::load(&FontOptions {
        paths: vec![font_dir().to_path_buf()],
        system: false,
    });
    let world = HostWorld::new(&root, "main.typ", &fonts).unwrap();
    let doc = typst::compile::<typst_layout::PagedDocument>(&world)
        .output
        .unwrap();
    let pdf = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()).unwrap();
    let mut reference = checker::reference(&pdf);
    checker::device_only(&mut reference);
    let caps = ClientCaps {
        minor: 3,
        opentype_programs: true,
        program_refs: true,
        program_budget: None,
        ..Default::default()
    };
    let mut t = Tables::new();
    let out = convert::page(&world, &doc, 0, &mut t, caps, &[], Positions::Frame).unwrap();
    let p = flashtex_display_list::page::Page::decode(
        flashtex_display_list::page::StreamKind::Page,
        &out.body,
    )
    .unwrap();
    let differ = p
        .origins
        .iter()
        .zip(&reference[0].glyphs)
        .filter(|(o, r)| o.map(f64::to_bits) != r.origin.map(f64::to_bits))
        .count();
    assert_eq!(p.origins.len(), reference[0].glyphs.len());
    assert!(
        differ > p.origins.len() / 2,
        "{differ} of {} differ",
        p.origins.len()
    );
    // And the host's own derivation, one page exported alone and untagged,
    // agrees with the checker on every glyph.
    let pp = flashtex_typst_host::pdfpos::derive(&doc, &[0]).unwrap();
    let mut t = Tables::new();
    let out = convert::page(&world, &doc, 0, &mut t, caps, &[], Positions::Pdf(&pp[0])).unwrap();
    let p = flashtex_display_list::page::Page::decode(
        flashtex_display_list::page::StreamKind::Page,
        &out.body,
    )
    .unwrap();
    assert!(p
        .origins
        .iter()
        .zip(&reference[0].glyphs)
        .all(|(o, r)| o.map(f64::to_bits) == r.origin.map(f64::to_bits)));
}

/// The same for paths: Typst's frame numbers are not the PDF's (krilla
/// writes f32), and the checker says so; the PDF-derived ones are.
#[test]
fn the_checker_rejects_frame_paths() {
    use flashtex_typst_host::convert::{self, ClientCaps, Positions, Tables};
    use flashtex_typst_host::world::{FontOptions, Fonts, HostWorld};
    let root = project("frame-paths", &fixture("shapes.typ"));
    let fonts = Fonts::load(&FontOptions {
        paths: vec![font_dir().to_path_buf()],
        system: false,
    });
    let world = HostWorld::new(&root, "main.typ", &fonts).unwrap();
    let doc = typst::compile::<typst_layout::PagedDocument>(&world)
        .output
        .unwrap();
    let pdf = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()).unwrap();
    let mut reference = checker::reference(&pdf);
    checker::device_only(&mut reference);
    let caps = ClientCaps {
        minor: 3,
        opentype_programs: true,
        program_refs: true,
        program_budget: None,
        ..Default::default()
    };
    let decode = |body: &[u8]| {
        flashtex_display_list::page::Page::decode(
            flashtex_display_list::page::StreamKind::Page,
            body,
        )
        .unwrap()
    };
    let mut t = Tables::new();
    let frame = decode(
        &convert::page(&world, &doc, 0, &mut t, caps, &[], Positions::Frame)
            .unwrap()
            .body,
    );
    let hp = checker::host_paths(&frame);
    let missing = checker::unmatched_paths(&hp, &reference[0].paths);
    assert!(!hp.is_empty());
    assert!(missing > 0, "frame paths all equal the PDF's?");
    let pp = flashtex_typst_host::pdfpos::derive(&doc, &[0]).unwrap();
    let mut t = Tables::new();
    let pdf_page = decode(
        &convert::page(&world, &doc, 0, &mut t, caps, &[], Positions::Pdf(&pp[0]))
            .unwrap()
            .body,
    );
    let hp = checker::host_paths(&pdf_page);
    assert_eq!(checker::unmatched_paths(&hp, &reference[0].paths), 0);
    eprintln!(
        "shapes.typ page 1: {missing} of {} frame paths differ from the PDF's; 0 of {} PDF-derived",
        hp.len(),
        hp.len()
    );
}
