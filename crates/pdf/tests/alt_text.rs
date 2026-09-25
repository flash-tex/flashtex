//! `/Alt` on image XObjects: `\includegraphics[alt=...]` text reaches the
//! exported PDF as an `/Alt` entry on the image XObject dictionary,
//! encoded as a PDF text string (ASCII verbatim, anything else UTF-16BE
//! with a BOM). Without alt text no `/Alt` entry is written.

use flashtex_pdf::exact::{self, Content, Decimal, ExactDocument, ExactPage, Op};
use flashtex_pdf::images::{Geometry, ImageXObject, LocalObject, Piece};
use flashtex_pdf::reader::{Obj, PdfFile};
use flashtex_pdf::verify;
use std::collections::BTreeMap;

/// A hand-built 1x1 gray image XObject: no fixture file is needed, so
/// these tests pin only the `/Alt` behaviour, never any decoder.
fn one_px_image() -> ImageXObject {
    ImageXObject {
        objects: vec![LocalObject {
            dict: vec![Piece::Text(
                "/Type /XObject /Subtype /Image /Width 1 /Height 1 /BitsPerComponent 8 /ColorSpace /DeviceGray".into(),
            )],
            stream: Some(vec![0x80]),
        }],
        geometry: Geometry::Raster { width: 1, height: 1 },
        needs_page_group: false,
        summary: "test 1x1 gray".into(),
    }
}

/// One 100x100 page painting `image` as `/Im0`.
fn doc_with(image: ImageXObject) -> ExactDocument {
    let one = Decimal::from_i64(1);
    let zero = Decimal::from_i64(0);
    ExactDocument {
        pages: vec![ExactPage {
            width: Decimal::new("100").unwrap(),
            height: Decimal::new("100").unwrap(),
            content: Content::Ops(vec![
                Op::Save,
                Op::Concat([
                    one.clone(),
                    zero.clone(),
                    zero.clone(),
                    one.clone(),
                    zero.clone(),
                    zero.clone(),
                ]),
                Op::Do("Im0".into()),
                Op::Restore,
            ]),
            fonts: Some(Vec::new()),
        }],
        fonts: BTreeMap::new(),
        images: BTreeMap::from([("Im0".to_string(), image)]),
        patterns: BTreeMap::new(),
    }
}

/// The `/Alt` entry of the page's only image XObject.
fn alt_of(bytes: &[u8]) -> Obj {
    let file = PdfFile::parse(bytes).unwrap();
    let pages = file.pages().unwrap();
    let xobjects = file
        .page_attr(pages[0], "Resources")
        .and_then(Obj::as_dict)
        .and_then(|r| file.get(r, "XObject"))
        .and_then(Obj::as_dict)
        .unwrap();
    let resolved = file.resolve(&xobjects["Im0"]);
    let Obj::Stream { dict, .. } = resolved else {
        panic!("Im0: not a stream");
    };
    file.get(dict, "Alt").cloned().unwrap()
}

#[test]
fn alt_text_reaches_the_written_pdf_bytes() {
    let mut image = one_px_image();
    image.set_alt("A red square (1x1)");
    let pdf = exact::render_exact(&doc_with(image)).unwrap();
    verify::check_structure(&pdf.bytes).unwrap();
    // Delimiters are escaped in the file, exactly as for other strings.
    let text = String::from_utf8_lossy(&pdf.bytes);
    assert!(
        text.contains("/Alt (A red square \\(1x1\\))"),
        "no escaped /Alt entry in {text}"
    );
    // And it round-trips through the reader to the original string.
    assert_eq!(
        alt_of(&pdf.bytes),
        Obj::String(b"A red square (1x1)".to_vec())
    );
}

#[test]
fn non_ascii_alt_is_utf16_with_a_bom() {
    let mut image = one_px_image();
    image.set_alt("café");
    let pdf = exact::render_exact(&doc_with(image)).unwrap();
    verify::check_structure(&pdf.bytes).unwrap();
    let text = String::from_utf8_lossy(&pdf.bytes);
    assert!(
        text.contains("/Alt (\\376\\377\\000c\\000a\\000f\\000\\351)"),
        "no UTF-16 /Alt entry in {text}"
    );
    assert_eq!(
        alt_of(&pdf.bytes),
        Obj::String(vec![0xFE, 0xFF, 0, 0x63, 0, 0x61, 0, 0x66, 0, 0xE9])
    );
}

#[test]
fn no_alt_entry_without_alt_text() {
    let pdf = exact::render_exact(&doc_with(one_px_image())).unwrap();
    verify::check_structure(&pdf.bytes).unwrap();
    assert!(
        !String::from_utf8_lossy(&pdf.bytes).contains("/Alt"),
        "unexpected /Alt entry"
    );
}
