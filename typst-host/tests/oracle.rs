//! Sample `.typ` documents compiled end to end through the host process,
//! checked against the pinned `typst` itself (DESIGN.md §15: Typst's own
//! output is the oracle):
//!
//! 1. `DONE.pdf` is byte-identical to `typst_pdf::pdf` of the same document
//!    compiled in this process by an independent `World`.
//! 2. The display list has the oracle's pages (count, size, box) and the
//!    oracle's glyphs (font, glyph id, in painting order).
//! 3. Each glyph's f64 origin (ORIGINS_F64) is where the PDF puts it, as a
//!    viewer computes it from typst-pdf's content stream (`tests/pdfpos`):
//!    the maximum difference is printed and bounded. typst-pdf writes
//!    positions in f32 (krilla), so equality is not expected in T0; the
//!    PDF-derived origins that make it exact are T1 (§15.5).

mod common;
mod oracle_world;
mod pdfpos;

use common::*;
use flashtex_display_list::client::Event;
use flashtex_display_list::kind;
use flashtex_display_list::page::Item;
use flashtex_typst_host::v33;
use typst::layout::{Abs, Frame, FrameItem, Point, Transform};

/// Glyphs of a frame in painting order: (glyph id, x, y) in pt, y down,
/// the typst-pdf walk (convert.rs `handle_frame`).
fn oracle_glyphs(frame: &Frame, ts: Transform, out: &mut Vec<(u16, f64, f64)>) {
    for (pos, item) in frame.items() {
        let ts = ts.pre_concat(Transform::translate(pos.x, pos.y));
        match item {
            FrameItem::Group(g) => oracle_glyphs(&g.frame, ts.pre_concat(g.transform), out),
            FrameItem::Text(t) => {
                let (mut x, mut y) = (Abs::zero(), Abs::zero());
                for g in &t.glyphs {
                    let p = Point::new(x + g.x_offset.at(t.size), y - g.y_offset.at(t.size))
                        .transform(ts);
                    out.push((g.id, p.x.to_pt(), p.y.to_pt()));
                    x += g.x_advance.at(t.size);
                    y -= g.y_advance.at(t.size);
                }
            }
            _ => {}
        }
    }
}

struct Report {
    glyphs: usize,
    max_bp: f64,
    sp_off: usize,
}

fn check(name: &str) -> Report {
    let host = HostProc::start(&format!("oracle-{name}"));
    let root = project(&format!("oracle-{name}"), &fixture(name));
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
    let pdf_path = done.str_field("pdf").unwrap().to_string();
    let host_pdf = std::fs::read(&pdf_path).unwrap();

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
    let pretty = typst_pdf::pdf(
        &doc,
        &typst_pdf::PdfOptions {
            pretty: true,
            ..Default::default()
        },
    )
    .unwrap();
    let pdf = pdfpos::Pdf::parse(&pretty);
    let pdf_pages = pdf.pages();

    // Pages, in order, with the oracle's sizes.
    let pages: Vec<(Vec<u8>, flashtex_display_list::page::Page)> = frames
        .iter()
        .filter(|(k, _)| *k == kind::PAGE)
        .map(|(_, b)| {
            (
                b.clone(),
                match flashtex_display_list::client::decode_event(kind::PAGE, b.clone()).unwrap() {
                    Event::Page(p) => p,
                    _ => unreachable!(),
                },
            )
        })
        .collect();
    assert_eq!(pages.len(), doc.pages().len(), "{name}: page count");
    assert_eq!(done.int_field("pages"), Some(doc.pages().len() as i64));
    let mut rep = Report {
        glyphs: 0,
        max_bp: 0.0,
        sp_off: 0,
    };
    for (i, (body, p)) in pages.iter().enumerate() {
        assert_eq!(p.index as usize, i);
        assert_eq!(
            p.flags & 1,
            0,
            "{name} page {i} INCOMPLETE: {:?}",
            p.unsupported
        );
        let tp = &doc.pages()[i];
        let size = tp.frame.size() + tp.bleed.sum_by_axis();
        let mb = pdf.media_box(pdf_pages[i]);
        assert_eq!(p.pdf_box, [0.0, 0.0, size.x.to_pt(), size.y.to_pt()]);
        assert!(
            (p.pdf_box[2] - mb[2]).abs() < 1e-4 && (p.pdf_box[3] - mb[3]).abs() < 1e-4,
            "{name} page {i} box {:?} vs PDF {mb:?}",
            p.pdf_box
        );

        // Glyph ids and positions against the oracle's frames.
        let mut og = vec![];
        let ts = Transform::translate(tp.bleed.left, tp.bleed.top);
        oracle_glyphs(&tp.frame, ts, &mut og);
        let glyphs: Vec<(u16, i32, i32)> = p
            .items
            .iter()
            .filter_map(|it| {
                if let Item::Glyph { code, x, y, .. } = it {
                    Some((*code, *x, *y))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(glyphs.len(), og.len(), "{name} page {i}: glyph count");
        let k = 65_781.76;
        for (g, o) in glyphs.iter().zip(&og) {
            assert_eq!(g.0, o.0, "{name} page {i}: glyph id");
            assert_eq!(
                (g.1, g.2),
                ((o.1 * k).round() as i32, (o.2 * k).round() as i32),
                "{name} page {i}: sp origin"
            );
        }

        // f64 origins against the PDF, as a viewer computes them.
        let secs = v33::sections(body).unwrap();
        let origins = v33::decode_origins(
            secs.iter()
                .find(|(t, _)| *t == v33::tag::ORIGINS_F64)
                .expect("ORIGINS_F64")
                .1,
        )
        .unwrap();
        let from_pdf = pdf.glyph_origins(pdf_pages[i]);
        assert_eq!(origins.len(), glyphs.len());
        assert_eq!(
            from_pdf.len(),
            origins.len(),
            "{name} page {i}: glyphs in the PDF"
        );
        let h = p.pdf_box[3];
        for ((o, q), g) in origins.iter().zip(&from_pdf).zip(&glyphs) {
            rep.max_bp = rep.max_bp.max((o.0 - q.0).abs()).max((o.1 - q.1).abs());
            if (g.1, g.2) != ((q.0 * k).round() as i32, ((h - q.1) * k).round() as i32) {
                rep.sp_off += 1;
            }
        }
        rep.glyphs += glyphs.len();
    }
    eprintln!(
        "{name}: {} pages, {} glyphs; DONE.pdf == typst-pdf ({} bytes); max |origin - PDF| = {:.2e} bp; {} glyph(s) whose sp rounding differs from the PDF-derived one",
        pages.len(),
        rep.glyphs,
        host_pdf.len(),
        rep.max_bp,
        rep.sp_off
    );
    // Track A §5.2 measured Typst frame positions within 5.8e-5 bp of the PDF.
    assert!(
        rep.max_bp < 2e-4,
        "{name}: frame origins {:.2e} bp from the PDF",
        rep.max_bp
    );
    rep
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
