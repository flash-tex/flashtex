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
    pub images: usize,
    /// The IMAGE resources sent.
    pub image_info: Vec<flashtex_display_list::json::Json>,
    pub mismatches: usize,
}

pub fn check_doc(name: &str, source: &str) -> Report {
    check_doc_with(name, source, &[], &[])
}

/// [`check_doc`] for a client that accepts `accept` (spec §11.7), with
/// `files` written into the project beside `main.typ`.
pub fn check_doc_with(
    name: &str,
    source: &str,
    accept: &[&str],
    files: &[(&str, Vec<u8>)],
) -> Report {
    let host = HostProc::start(&format!("oracle-{name}"));
    let root = project(&format!("oracle-{name}"), source);
    for (f, data) in files {
        std::fs::write(root.join(f), data).unwrap();
    }
    let out = scratch(&format!("out-{name}"));

    // Through the host.
    let mut c = host.connect();
    let (k, _) = c.hello_caps(3, 3, accept);
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
    let reference = checker::reference(&oracle_pdf);

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
        images: 0,
        image_info: vec![],
        mismatches: 0,
    };
    // IMAGE and IMAGE_DATA by id, each IMAGE_DATA right after its IMAGE.
    let mut image_info = std::collections::HashMap::new();
    for (n, (k, b)) in frames.iter().enumerate() {
        if *k == kind::IMAGE {
            let j = json_of(b);
            let (k2, d) = &frames[n + 1];
            assert_eq!(
                *k2,
                kind::IMAGE_DATA,
                "{name}: IMAGE_DATA right after IMAGE"
            );
            let d = flashtex_display_list::resource::ImageData::decode(d).unwrap();
            assert_eq!(
                Some(d.id as i64),
                j.get("id").and_then(|v| match v {
                    flashtex_display_list::json::Json::Int(i) => Some(*i),
                    _ => None,
                })
            );
            assert!(
                image_info.insert(d.id, (j, d.parts)).is_none(),
                "{name}: image {} twice",
                d.id
            );
        }
    }
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
        let colours = checker::host_colors(p).0;
        assert_eq!(colours.len(), g.len());
        for (j, (rg, (fill, alpha))) in r.glyphs.iter().zip(&colours).enumerate() {
            let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
            if bits(fill) != bits(&rg.fill) || alpha.to_bits() != rg.fill_alpha.to_bits() {
                if rep.mismatches < 5 {
                    eprintln!(
                        "{name} page {i} glyph {j}: fill {fill:?} {alpha}, PDF {:?} {}",
                        rg.fill, rg.fill_alpha
                    );
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
        // Images: every raster `Do` of the PDF, its CTM and pixels bit for
        // bit (spec §5.2, §11.5).
        let hi = checker::host_images(p);
        let missing = checker::unmatched_images(&hi, &image_info, &r.images);
        let raster = r.images.iter().filter(|x| !x.form).count();
        if missing > 0 || hi.len() != raster {
            eprintln!(
                "{name} page {i}: {missing} of {} images are not the PDF's; the PDF draws {raster}",
                hi.len()
            );
            rep.mismatches += missing.max(1);
        }
        rep.images += hi.len();
    }
    eprintln!(
        "{name}: {} pages, {} glyphs, {} paths, {} images, {} position mismatches; DONE.pdf == typst-pdf ({} bytes)",
        pages.len(),
        rep.glyphs,
        rep.paths,
        rep.images,
        rep.mismatches,
        host_pdf.len()
    );
    assert_eq!(rep.mismatches, 0, "{name}: positions differ from the PDF's");
    rep.image_info = image_info.into_values().map(|(j, _)| j).collect();
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

/// A PNG of `w`×`h` pixels: colour type `ctype` (0 grey, 2 RGB, 6 RGBA),
/// `depth` bits, each row's bytes from `row(y)` (filter 0).
fn png(w: u32, h: u32, ctype: u8, depth: u8, row: impl Fn(u32) -> Vec<u8>) -> Vec<u8> {
    fn crc(data: &[u8]) -> u32 {
        let mut c = !0u32;
        for &b in data {
            c ^= b as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xedb8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
        }
        !c
    }
    let chunk = |out: &mut Vec<u8>, t: &[u8], d: &[u8]| {
        out.extend((d.len() as u32).to_be_bytes());
        let mut td = t.to_vec();
        td.extend(d);
        out.extend(&td);
        out.extend(crc(&td).to_be_bytes());
    };
    let mut raw = vec![];
    for y in 0..h {
        raw.push(0);
        raw.extend(row(y));
    }
    let mut ihdr = vec![];
    ihdr.extend(w.to_be_bytes());
    ihdr.extend(h.to_be_bytes());
    ihdr.extend([depth, ctype, 0, 0, 0]);
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(
        &mut out,
        b"IDAT",
        &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6),
    );
    chunk(&mut out, b"IEND", &[]);
    out
}

/// A 16×8 baseline JPEG (made with macOS `sips` from a gradient).
const PHOTO_JPG: &[u8] = &[
    0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x48,
    0x00, 0x48, 0x00, 0x00, 0xff, 0xe1, 0x00, 0x4c, 0x45, 0x78, 0x69, 0x66, 0x00, 0x00, 0x4d, 0x4d,
    0x00, 0x2a, 0x00, 0x00, 0x00, 0x08, 0x00, 0x01, 0x87, 0x69, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01,
    0x00, 0x00, 0x00, 0x1a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xa0, 0x01, 0x00, 0x03, 0x00, 0x00,
    0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xa0, 0x02, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
    0x00, 0x10, 0xa0, 0x03, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00,
    0x00, 0x00, 0xff, 0xed, 0x00, 0x38, 0x50, 0x68, 0x6f, 0x74, 0x6f, 0x73, 0x68, 0x6f, 0x70, 0x20,
    0x33, 0x2e, 0x30, 0x00, 0x38, 0x42, 0x49, 0x4d, 0x04, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x38, 0x42, 0x49, 0x4d, 0x04, 0x25, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0xd4, 0x1d, 0x8c, 0xd9,
    0x8f, 0x00, 0xb2, 0x04, 0xe9, 0x80, 0x09, 0x98, 0xec, 0xf8, 0x42, 0x7e, 0xff, 0xc0, 0x00, 0x11,
    0x08, 0x00, 0x08, 0x00, 0x10, 0x03, 0x01, 0x22, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01, 0xff,
    0xc4, 0x00, 0x1f, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b,
    0xff, 0xc4, 0x00, 0xb5, 0x10, 0x00, 0x02, 0x01, 0x03, 0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04,
    0x04, 0x00, 0x00, 0x01, 0x7d, 0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41,
    0x06, 0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1, 0x08, 0x23, 0x42, 0xb1,
    0xc1, 0x15, 0x52, 0xd1, 0xf0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16, 0x17, 0x18, 0x19,
    0x1a, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44,
    0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64,
    0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x83, 0x84,
    0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2,
    0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9,
    0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7,
    0xd8, 0xd9, 0xda, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf1, 0xf2, 0xf3,
    0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa, 0xff, 0xc4, 0x00, 0x1f, 0x01, 0x00, 0x03, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03,
    0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0xff, 0xc4, 0x00, 0xb5, 0x11, 0x00, 0x02, 0x01,
    0x02, 0x04, 0x04, 0x03, 0x04, 0x07, 0x05, 0x04, 0x04, 0x00, 0x01, 0x02, 0x77, 0x00, 0x01, 0x02,
    0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71, 0x13, 0x22, 0x32,
    0x81, 0x08, 0x14, 0x42, 0x91, 0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33, 0x52, 0xf0, 0x15, 0x62, 0x72,
    0xd1, 0x0a, 0x16, 0x24, 0x34, 0xe1, 0x25, 0xf1, 0x17, 0x18, 0x19, 0x1a, 0x26, 0x27, 0x28, 0x29,
    0x2a, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x53,
    0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73,
    0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a,
    0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8,
    0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6,
    0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe2, 0xe3, 0xe4,
    0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa, 0xff,
    0xdb, 0x00, 0x43, 0x00, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x06, 0x04, 0x04, 0x06, 0x09, 0x06,
    0x06, 0x06, 0x09, 0x0c, 0x09, 0x09, 0x09, 0x09, 0x0c, 0x0f, 0x0c, 0x0c, 0x0c, 0x0c, 0x0c, 0x0f,
    0x12, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x15,
    0x15, 0x15, 0x15, 0x15, 0x15, 0x19, 0x19, 0x19, 0x19, 0x19, 0x1c, 0x1c, 0x1c, 0x1c, 0x1c, 0x1c,
    0x1c, 0x1c, 0x1c, 0x1c, 0xff, 0xdb, 0x00, 0x43, 0x01, 0x04, 0x05, 0x05, 0x07, 0x07, 0x07, 0x0c,
    0x07, 0x07, 0x0c, 0x1d, 0x14, 0x10, 0x14, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d,
    0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d,
    0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d,
    0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0x1d, 0xff, 0xdd, 0x00, 0x04, 0x00, 0x01, 0xff,
    0xda, 0x00, 0x0c, 0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3f, 0x00, 0xe4, 0x6c, 0xbe,
    0x14, 0xf4, 0xfd, 0xcf, 0xe9, 0x5d, 0x85, 0x97, 0xc2, 0xae, 0x9f, 0xb9, 0xfd, 0x2b, 0xd5, 0x2c,
    0xbb, 0x57, 0x5f, 0x63, 0xda, 0xb9, 0x3c, 0x4e, 0xcf, 0xf1, 0xbe, 0xff, 0x00, 0xbe, 0x7c, 0xdf,
    0x87, 0x7c, 0x4f, 0x98, 0xfb, 0x9f, 0xbc, 0x3f, 0xff, 0xd9,
];

/// Raster images (E6) for a client that accepts `image-data`: every image
/// the PDF draws is an IMAGE item with the PDF's CTM, and the IMAGE_DATA
/// parts are the PDF's samples (or its JPEG), soft mask and ICC profile.
#[test]
fn raster_images_match_typst() {
    let rgba = png(7, 5, 6, 8, |y| {
        (0..7u32)
            .flat_map(|x| {
                [
                    (x * 36) as u8,
                    (y * 60) as u8,
                    200,
                    (40 + x * 30 + y * 5) as u8,
                ]
            })
            .collect()
    });
    let grey = png(8, 3, 0, 8, |y| {
        (0..8u32).map(|x| (x * 32 + y * 7) as u8).collect()
    });
    let deep = png(3, 2, 2, 16, |y| {
        (0..3u32)
            .flat_map(|x| {
                let v = (x * 20_000 + y * 9_000) as u16;
                [
                    v.to_be_bytes(),
                    (65_535 - v).to_be_bytes(),
                    0x1234u16.to_be_bytes(),
                ]
                .concat()
            })
            .collect()
    });
    let r = check_doc_with(
        "images.typ",
        &fixture("images.typ"),
        &["image-data"],
        &[
            ("rgba.png", rgba),
            ("grey.png", grey),
            ("deep.png", deep),
            ("photo.jpg", PHOTO_JPG.to_vec()),
        ],
    );
    assert_eq!(r.images, 10);
    // Five resources: an image drawn twice is sent once; the 16-bit PNG
    // twice, smooth and not (typst-pdf writes it as 8-bit samples), with
    // a soft mask, a grey image and a JPEG.
    use flashtex_display_list::json::Json;
    let has = |k: &str, v: Json| r.image_info.iter().filter(|j| j.get(k) == Some(&v)).count();
    assert_eq!(r.image_info.len(), 5, "{:?}", r.image_info);
    assert_eq!(has("smask", Json::Bool(true)), 1);
    assert_eq!(has("components", Json::Int(1)), 1);
    assert_eq!(has("type", Json::Str("jpeg".into())), 1);
    assert_eq!(has("interpolate", Json::Bool(true)), 1);
}

/// Without `image-data` a page with an image is INCOMPLETE and no IMAGE is
/// sent; HELLO offers `image-data`.
#[test]
fn images_need_image_data() {
    let host = HostProc::start("no-image-data");
    let root = project(
        "no-image-data",
        "#set page(width: 100pt, height: 100pt)\nA #image(\"grey.png\", width: 40pt)\n",
    );
    let grey = png(8, 3, 0, 8, |y| {
        (0..8u32).map(|x| (x * 32 + y * 7) as u8).collect()
    });
    std::fs::write(root.join("grey.png"), grey).unwrap();
    for accept in [&[][..], &["image-data"][..]] {
        let mut c = host.connect();
        let (_, hello) = c.hello_caps(3, 3, accept);
        assert!(hello.to_string().contains("\"image-data\""), "{hello}");
        c.send(
            kind::COMPILE,
            &compile_json(1, &root, "main.typ", r#""font_formats":["opentype"]"#),
        );
        let frames = c.until_done();
        let images = frames.iter().filter(|(k, _)| *k == kind::IMAGE).count();
        let page = frames
            .iter()
            .find(|(k, _)| *k == kind::PAGE)
            .map(|(_, b)| {
                flashtex_display_list::page::Page::decode(
                    flashtex_display_list::page::StreamKind::Page,
                    b,
                )
                .unwrap()
            })
            .unwrap();
        if accept.is_empty() {
            assert_eq!(images, 0);
            assert_eq!(page.flags & 1, 1);
            assert!(page
                .unsupported
                .iter()
                .any(|u| u.contains("accept image-data")));
        } else {
            assert_eq!(images, 1);
            assert_eq!(page.flags & 1, 0, "{:?}", page.unsupported);
        }
    }
}

/// The image check discriminates: one sample changed, or a matrix nudged
/// by an ulp, is not the PDF's image.
#[test]
fn the_checker_rejects_wrong_pixels_and_matrices() {
    use flashtex_display_list::json::Json;
    use flashtex_display_list::resource::ImageData;
    use flashtex_typst_host::convert::{self, ClientCaps, Positions, Tables};
    use flashtex_typst_host::world::{FontOptions, Fonts, HostWorld};
    let root = project(
        "wrong-pixels",
        "#set page(width: 100pt, height: 100pt)\n#image(\"grey.png\", width: 40pt)\n",
    );
    let grey = png(8, 3, 0, 8, |y| {
        (0..8u32).map(|x| (x * 32 + y * 7) as u8).collect()
    });
    std::fs::write(root.join("grey.png"), grey).unwrap();
    let fonts = Fonts::load(&FontOptions {
        paths: vec![font_dir().to_path_buf()],
        system: false,
    });
    let world = HostWorld::new(&root, "main.typ", &fonts).unwrap();
    let doc = typst::compile::<typst_layout::PagedDocument>(&world)
        .output
        .unwrap();
    let pdf = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()).unwrap();
    let reference = checker::reference(&pdf);
    let caps = ClientCaps {
        minor: 3,
        opentype_programs: true,
        program_refs: true,
        image_data: true,
        ..Default::default()
    };
    let pp = flashtex_typst_host::pdfpos::derive(&doc, &[0]).unwrap();
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
    let page = flashtex_display_list::page::Page::decode(
        flashtex_display_list::page::StreamKind::Page,
        &out.body,
    )
    .unwrap();
    let mut info = std::collections::HashMap::new();
    for (j, d) in &out.images {
        let d = ImageData::decode(d).unwrap();
        info.insert(
            d.id,
            (
                Json::parse(std::str::from_utf8(j).unwrap()).unwrap(),
                d.parts,
            ),
        );
    }
    let hi = checker::host_images(&page);
    assert_eq!(hi.len(), 1);
    assert_eq!(
        checker::unmatched_images(&hi, &info, &reference[0].images),
        0
    );
    let mut wrong = info.clone();
    wrong.get_mut(&hi[0].1).unwrap().1[0][5] ^= 1;
    assert_eq!(
        checker::unmatched_images(&hi, &wrong, &reference[0].images),
        1
    );
    let mut nudged = hi.clone();
    nudged[0].0[4] = f64::from_bits(nudged[0].0[4].to_bits() + 1);
    assert_eq!(
        checker::unmatched_images(&nudged, &info, &reference[0].images),
        1
    );
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
    let reference = checker::reference(&pdf);
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
    let reference = checker::reference(&pdf);
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
