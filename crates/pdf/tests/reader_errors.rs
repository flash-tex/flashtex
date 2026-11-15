//! Error paths of [`flashtex_pdf::reader::PdfFile`].
//!
//! The only unit tests parse one well-formed file and exercise reference
//! lookahead; every integration test unwraps the happy path. These tests
//! feed malformed bytes to each fallible entry point so every `Err` branch
//! has a witness. Inputs are minimal hand-built PDFs in the style of the
//! `reader` unit tests, with the `assert_err` helper from `verify_errors.rs`.

use flashtex_pdf::reader::{Obj, PdfFile};

#[track_caller]
fn assert_err<T: std::fmt::Debug>(result: Result<T, String>, want: &str) {
    let err = result.unwrap_err();
    assert!(err.contains(want), "want {want:?} in {err:?}");
}

/// Wraps object bodies in the smallest file the scanner accepts. The trailer
/// carries no `/Root` on purpose: [`PdfFile::catalog`] falls back to finding
/// the catalog object, and tests that need no catalog get none.
fn pdf_with(objects: &[u8]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    pdf.extend_from_slice(objects);
    pdf.extend_from_slice(b"trailer\n<< /Size 1 >>\n%%EOF\n");
    pdf
}

#[test]
fn parse_reports_malformed_object_bodies() {
    // Each body fails the lexer; the object number is attached by the caller.
    for (body, want) in [
        ("(abc", "unterminated literal string"), // no `)` anywhere later
        ("<ZZ>", "bad hex digit"),
        ("<< 123 >>", "dictionary key expected"),
        (")", "unexpected delimiter"),
        ("foobar", "unexpected keyword"),
        ("99999999999 0 R", "object number"), // u32 overflow in `n g R`
        ("1 99999 R", "generation"),          // u16 overflow in `n g R`
    ] {
        let pdf = pdf_with(format!("1 0 obj\n{body}\nendobj\n").as_bytes());
        assert_err(PdfFile::parse(&pdf), want);
    }
    // A non-ASCII word is not a keyword, number, or name.
    let pdf = pdf_with(b"1 0 obj\n\xff\nendobj\n");
    assert_err(PdfFile::parse(&pdf), "non-ASCII token");
    // Truncated inputs, which need no trailer: nothing follows the object.
    assert_err(
        PdfFile::parse(b"%PDF-1.4\n1 0 obj"),
        "unexpected end of file",
    );
    assert_err(
        PdfFile::parse(b"%PDF-1.4\n1 0 obj\n(abc\\"),
        "unterminated escape",
    );
    assert_err(
        PdfFile::parse(b"%PDF-1.4\n1 0 obj\n<4142\nendobj\n"),
        "unterminated hex string",
    );
}

#[test]
fn parse_rejects_deeply_nested_objects() {
    let mut pdf = b"%PDF-1.4\n1 0 obj\n".to_vec();
    pdf.extend_from_slice(&[b'['; 70]);
    assert_err(PdfFile::parse(&pdf), "object nesting deeper than 64");
}

#[test]
fn parse_rejects_bad_streams() {
    let pdf = pdf_with(b"1 0 obj\n42\nstream\nhello\nendstream\nendobj\n");
    assert_err(PdfFile::parse(&pdf), "stream without dictionary");

    let pdf = b"%PDF-1.4\n1 0 obj\n<< /Length 5 >>\nstream\nhello";
    assert_err(PdfFile::parse(pdf), "no endstream");
}

#[test]
fn stream_decoding_failures_surface_to_callers() {
    // Not a stream: a plain dictionary, and a dangling reference (Null).
    let pdf = pdf_with(b"1 0 obj\n<< /Type /Catalog >>\nendobj\n");
    let file = PdfFile::parse(&pdf).unwrap();
    assert_err(file.decode_stream(&Obj::Ref(1, 0)), "not a stream");
    assert_err(file.decode_stream(&Obj::Ref(99, 0)), "not a stream");

    // A /Filter that is neither a name nor an array of names.
    let pdf = pdf_with(b"2 0 obj\n<< /Length 5 /Filter 42 >>\nstream\nhello\nendstream\nendobj\n");
    let file = PdfFile::parse(&pdf).unwrap();
    assert_err(
        file.decode_stream(&Obj::Ref(2, 0)),
        "unsupported /Filter value",
    );

    // Unknown filters, as a single name and as an array.
    for filter in ["/DCTDecode", "[/DCTDecode]"] {
        let body = format!(
            "2 0 obj\n<< /Length 5 /Filter {filter} >>\nstream\nhello\nendstream\nendobj\n"
        );
        let file = PdfFile::parse(&pdf_with(body.as_bytes())).unwrap();
        assert_err(
            file.decode_stream(&Obj::Ref(2, 0)),
            "filter DCTDecode is not supported",
        );
    }

    // A predictor needs PNG unfiltering, which the reader does not do.
    let pdf = pdf_with(
        b"2 0 obj\n<< /Length 5 /Filter /FlateDecode /DecodeParms << /Predictor 2 >> >>\nstream\nhello\nendstream\nendobj\n",
    );
    let file = PdfFile::parse(&pdf).unwrap();
    assert_err(
        file.decode_stream(&Obj::Ref(2, 0)),
        "FlateDecode with a predictor is not supported",
    );

    // A single zero byte is not even a complete zlib header.
    let pdf = pdf_with(
        b"2 0 obj\n<< /Length 1 /Filter /FlateDecode >>\nstream\n\x00\nendstream\nendobj\n",
    );
    let file = PdfFile::parse(&pdf).unwrap();
    assert_err(
        file.decode_stream(&Obj::Ref(2, 0)),
        "deflate stream truncated",
    );

    // Page contents propagate the decode failure, single and array form.
    for contents in ["4 0 R", "[4 0 R]"] {
        let body = format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /Contents {contents} >>\nendobj\n\
             4 0 obj\n<< /Length 1 /Filter /FlateDecode >>\nstream\n\x00\nendstream\nendobj\n"
        );
        let file = PdfFile::parse(&pdf_with(body.as_bytes())).unwrap();
        let pages = file.pages().unwrap();
        assert_eq!(pages.len(), 1);
        assert_err(file.page_content(pages[0]), "deflate stream truncated");
    }
}

#[test]
fn catalog_and_pages_report_a_broken_tree() {
    // No catalog object and no trailer /Root.
    let pdf = pdf_with(b"1 0 obj\n42\nendobj\n");
    let file = PdfFile::parse(&pdf).unwrap();
    assert_err(file.catalog(), "no catalog");
    assert_err(file.pages(), "no catalog");

    // A catalog whose /Pages is missing or is not a dictionary.
    for pages in ["", "/Pages 42"] {
        let body = format!("1 0 obj\n<< /Type /Catalog {pages} >>\nendobj\n");
        let file = PdfFile::parse(&pdf_with(body.as_bytes())).unwrap();
        assert_err(file.pages(), "no /Pages");
    }

    // A page-tree node without /Kids.
    let pdf = pdf_with(
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
          2 0 obj\n<< /Type /Pages >>\nendobj\n",
    );
    let file = PdfFile::parse(&pdf).unwrap();
    assert_err(file.pages(), "page node without /Kids");

    // A kid that is not a dictionary.
    let pdf = pdf_with(
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
          2 0 obj\n<< /Type /Pages /Kids [42] /Count 1 >>\nendobj\n",
    );
    let file = PdfFile::parse(&pdf).unwrap();
    assert_err(file.pages(), "kid is not a dictionary");

    // A page-tree chain 34 nodes deep trips the depth guard.
    let mut objs = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec();
    for i in 2..=35u32 {
        objs.extend_from_slice(
            format!("{i} 0 obj\n<< /Kids [{} 0 R] >>\nendobj\n", i + 1).as_bytes(),
        );
    }
    let file = PdfFile::parse(&pdf_with(&objs)).unwrap();
    assert_err(file.pages(), "page tree deeper than 32");
}

#[test]
fn object_streams_reject_malformed_headers() {
    // Object 2 lives inside stream 1; the length is found by searching for
    // `endstream`, so mutations never need to renumber a /Length.
    fn objstm(dict: &str, data: &[u8]) -> Vec<u8> {
        let mut pdf = b"%PDF-1.4\n1 0 obj\n<< /Type /ObjStm ".to_vec();
        pdf.extend_from_slice(dict.as_bytes());
        pdf.extend_from_slice(b" >>\nstream\n");
        pdf.extend_from_slice(data);
        pdf.extend_from_slice(b"\nendstream\nendobj\ntrailer\n<< /Size 2 >>\n");
        pdf
    }

    // Valid base: the embedded object appears in the map.
    let file = PdfFile::parse(&objstm("/N 1 /First 4", b"2 0 (hi)")).unwrap();
    assert_eq!(file.objects.get(&2), Some(&Obj::String(b"hi".to_vec())));

    assert_err(
        PdfFile::parse(&objstm("/First 4", b"2 0 (hi)")),
        "ObjStm /N",
    );
    assert_err(
        PdfFile::parse(&objstm("/N 1", b"2 0 (hi)")),
        "ObjStm /First",
    );
    assert_err(
        PdfFile::parse(&objstm("/N 1 /First 99", b"2 0 (hi)")),
        "ObjStm header",
    );
    assert_err(
        PdfFile::parse(&objstm("/N 1 /First 4", b"oops(hi)")),
        "ObjStm header numbers",
    );
    assert_err(
        PdfFile::parse(&objstm("/N 2 /First 4", b"2 0 (hi)")),
        "ObjStm header too short",
    );
    assert_err(
        PdfFile::parse(&objstm("/N 1 /First 4", b"2 0 )oops")),
        "ObjStm object 2",
    );
    // An object stream the reader cannot decode fails the whole parse.
    assert_err(
        PdfFile::parse(&objstm("/N 1 /First 4 /Filter /FlateDecode", b"\x00")),
        "deflate stream truncated",
    );
}
