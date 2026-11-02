//! Error paths of [`flashtex_pdf::verify`].
//!
//! Happy paths are covered by `render.rs` (structure fields, placements,
//! rules); nothing covered the `Err` branches. Structural tests mutate a real
//! writer-produced PDF so the base input is known-good; the helpers panic on
//! a missing needle so a writer format change fails loudly, not silently.

use flashtex_pdf::{CompileResult, Item, Page, TextItem, render_pdf, verify};

fn sample_pdf() -> Vec<u8> {
    let result = CompileResult {
        pages: vec![Page {
            number: 1,
            width_pt: 612.0,
            height_pt: 792.0,
            items: vec![Item::Text(TextItem {
                text: "Hello".into(),
                x_pt: 72.0,
                baseline_y_pt: 100.0,
                font_size_pt: 12.0,
                font: None,
            })],
        }],
        capabilities: None,
    };
    let out = render_pdf(&result).unwrap();
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    out.bytes
}

fn find(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len())
        .position(|w| w == needle)
        .unwrap_or_else(|| panic!("{needle:?} missing"))
}

fn replace_first(hay: &[u8], needle: &[u8], replacement: &[u8]) -> Vec<u8> {
    let at = find(hay, needle);
    let mut out = hay.to_vec();
    out.splice(at..at + needle.len(), replacement.iter().copied());
    out
}

/// `endobj` appears once per object, so the last one ends the last object.
fn replace_last_endobj(pdf: &[u8]) -> Vec<u8> {
    let at = pdf
        .windows(6)
        .rposition(|w| w == b"endobj")
        .expect("endobj missing");
    let mut out = pdf.to_vec();
    out[at..at + 6].copy_from_slice(b"endobX");
    out
}

#[track_caller]
fn assert_err<T: std::fmt::Debug>(result: Result<T, String>, want: &str) {
    let err = result.unwrap_err();
    assert!(err.contains(want), "want {want:?} in {err:?}");
}

#[test]
fn structure_rejects_bad_header_and_trailer() {
    let pdf = sample_pdf();
    let bad = replace_first(&pdf, b"%PDF-1.4\n", b"%PDF-9.9\n");
    assert_err(verify::check_structure(&bad), "missing %PDF-1.4 header");

    let mut truncated = pdf.clone();
    truncated.truncate(pdf.len() - 6); // drops "%%EOF\n"
    assert_err(
        verify::check_structure(&truncated),
        "missing %%EOF trailer marker",
    );

    let mut trailing = pdf.clone();
    trailing.extend_from_slice(b"%garbage");
    assert_err(verify::check_structure(&trailing), "bytes after %%EOF");
}

#[test]
fn structure_rejects_bad_startxref() {
    let pdf = sample_pdf();
    let bad = replace_first(&pdf, b"startxref\n", b"startXref\n");
    assert_err(verify::check_structure(&bad), "missing startxref");

    let sx = pdf.windows(10).rposition(|w| w == b"startxref\n").unwrap() + 10;
    let end = sx + pdf[sx..].iter().position(|&b| b == b'\n').unwrap();
    let mut bad = pdf.clone();
    bad.splice(sx..end, b"not-a-number".iter().copied());
    assert_err(verify::check_structure(&bad), "startxref is not a number");

    // Offset 0 points at the header, not the xref table.
    let mut bad = pdf.clone();
    bad.splice(sx..end, b"0".iter().copied());
    assert_err(verify::check_structure(&bad), "does not point at 'xref'");
}

#[test]
fn structure_rejects_bad_xref_table() {
    let pdf = sample_pdf();
    let bad = replace_first(&pdf, b"xref\n0 ", b"xref\nX ");
    assert_err(
        verify::check_structure(&bad),
        "malformed xref subsection header",
    );
    let bad = replace_first(&pdf, b"xref\n0 ", b"xref\n1 ");
    assert_err(verify::check_structure(&bad), "xref must start at object 0");

    // Object 0 must stay free; in-use entries must be marked `n`.
    let bad = replace_first(&pdf, b"0000000000 65535 f \n", b"0000000000 65535 n \n");
    assert_err(verify::check_structure(&bad), "object 0 must be free");
    let bad = replace_first(&pdf, b" 00000 n \n", b" 00000 f \n");
    assert_err(verify::check_structure(&bad), "unexpected type");

    // Entries span exactly 20 bytes with a numeric ASCII offset.
    let bad = replace_first(&pdf, b" 00000 n \n", b" 00000 n X");
    assert_err(verify::check_structure(&bad), "is not 20 bytes");
    let at = find(&pdf, b" 00000 n \n") - 10; // start of the first in-use entry
    let mut bad = pdf.clone();
    bad[at..at + 10].copy_from_slice(b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff");
    assert_err(verify::check_structure(&bad), "not ASCII");
    let mut bad = pdf.clone();
    bad[at..at + 10].copy_from_slice(b"notanumber");
    assert_err(verify::check_structure(&bad), "offset");

    // An offset pointing at the wrong bytes.
    let mut bad = pdf.clone();
    bad[at] = if bad[at] == b'0' { b'1' } else { b'0' };
    assert_err(verify::check_structure(&bad), "does not point at");

    // No object body may lose its end marker.
    assert_err(
        verify::check_structure(&replace_last_endobj(&pdf)),
        "has no endobj",
    );
}

#[test]
fn structure_rejects_bad_trailer() {
    let pdf = sample_pdf();
    let bad = replace_first(&pdf, b"trailer\n", b"TRAILER\n");
    assert_err(verify::check_structure(&bad), "not followed by trailer");

    let at = find(&pdf, b"/Size ") + b"/Size ".len();
    let mut bad = pdf.clone();
    assert!(bad[at].is_ascii_digit());
    bad[at] = if bad[at] == b'9' { b'8' } else { bad[at] + 1 };
    assert_err(verify::check_structure(&bad), "/Size disagrees");

    let bad = replace_first(&pdf, b"/Root 1 0 R", b"/Root 2 0 R");
    assert_err(verify::check_structure(&bad), "no /Root 1 0 R");
}

#[test]
fn stream_data_rejects_bad_dictionaries() {
    let pdf = sample_pdf();
    assert_err(verify::stream_data(&pdf, 9999), "not found");
    // Object 1 is the catalog dictionary, not a stream.
    assert_err(verify::stream_data(&pdf, 1), "has no /Length");

    // Corrupt the first stream's /Length in its own dictionary.
    let s = verify::check_structure(&pdf).unwrap();
    let obj = s.stream_objects[0];
    let head = format!("\n{obj} 0 obj\n");
    let obj_at = find(&pdf, head.as_bytes());
    let len_at = find(&pdf[obj_at..], b"/Length ") + obj_at + b"/Length ".len();
    let digits = pdf[len_at..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    assert!(digits > 0, "no /Length digits");
    let length: usize = std::str::from_utf8(&pdf[len_at..len_at + digits])
        .unwrap()
        .parse()
        .unwrap();

    let mut bad = pdf.clone();
    bad[len_at..len_at + digits].fill(b'x');
    assert_err(verify::stream_data(&bad, obj), "non-numeric /Length");

    let mut bad = pdf.clone();
    bad.splice(
        len_at..len_at + digits,
        format!("{}", length + 1_000_000).bytes(),
    );
    assert_err(verify::stream_data(&bad, obj), "stream truncated");

    let off_by_one = if length % 10 == 9 {
        length - 1
    } else {
        length + 1
    };
    let mut bad = pdf.clone();
    bad.splice(len_at..len_at + digits, format!("{off_by_one}").bytes());
    assert_err(verify::stream_data(&bad, obj), "does not end at endstream");
}

#[test]
fn placements_recovers_escaped_and_hex_bytes() {
    // The writer's escaping round-trips through the verifier's unescaping.
    let raw = b"a(b)c\\d";
    let mut content = b"BT\n/F1 12 Tf\n72 692 Td\n(".to_vec();
    content.extend_from_slice(&flashtex_pdf::writer::escape_string(raw));
    content.extend_from_slice(b") Tj\n/F2 10 Tf\n<4142> Tj\nET\n");
    let p = verify::placements(&content).unwrap();
    assert_eq!(p.len(), 2);
    assert_eq!(p[0].bytes.as_slice(), raw);
    assert_eq!(p[1].bytes.as_slice(), b"AB");
    // Control escapes map; a trailing backslash escapes nothing and is kept.
    let content = b"BT\n/F1 12 Tf\n0 0 Td\n(a\\nb\\rc\\\\d\\)e) Tj\n(ab\\) Tj\nET\n";
    let p = verify::placements(content).unwrap();
    assert_eq!(p[0].bytes.as_slice(), b"a\nb\rc\\d)e");
    assert_eq!(p[1].bytes.as_slice(), b"ab\\");
}

#[test]
fn placements_rejects_dangling_operators() {
    assert_err(
        verify::placements(b"BT\n/F1 12 Tf\n(x) Tj\nET\n"),
        "without preceding Td",
    );
    assert_err(
        verify::placements(b"BT\n72 692 Td\n(x) Tj\nET\n"),
        "without Tf",
    );
    // ET clears the position; a later Tj dangles again.
    let after_et = b"BT\n/F1 12 Tf\n72 692 Td\n(x) Tj\nET\n(y) Tj\n";
    assert_err(verify::placements(after_et), "without preceding Td");
    // A new BT block clears the font.
    let after_bt = b"BT\n/F1 12 Tf\n72 692 Td\n(x) Tj\nET\nBT\n0 0 Td\n(y) Tj\nET\n";
    assert_err(verify::placements(after_bt), "without Tf");
}

#[test]
fn placements_rejects_bad_font_and_position_lines() {
    for line in ["/Foo 12 Tf", "/F 12 Tf", "/Fx 12 Tf", "/F1x 12 Tf"] {
        let content = format!("BT\n{line}\n72 692 Td\n(x) Tj\nET\n");
        assert_err(
            verify::placements(content.as_bytes()),
            "unexpected font resource",
        );
    }
    assert_err(
        verify::placements(b"BT\n/F1 big Tf\n72 692 Td\n(x) Tj\nET\n"),
        "Tf size",
    );
    assert_err(
        verify::placements(b"BT\n/F1 12 Tf\nx 692 Td\n(x) Tj\nET\n"),
        "Td x",
    );
    assert_err(
        verify::placements(b"BT\n/F1 12 Tf\n72 y Td\n(x) Tj\nET\n"),
        "Td y",
    );
    assert_err(
        verify::placements(b"BT\n\xff 12 Tf\n72 692 Td\n(x) Tj\nET\n"),
        "Tf line not ASCII",
    );
    assert_err(
        verify::placements(b"BT\n/F1 12 Tf\n\xff 692 Td\n(x) Tj\nET\n"),
        "Td line not ASCII",
    );
}

#[test]
fn placements_rejects_bad_string_operands() {
    // Operand starts with neither `(` nor `<`.
    assert_err(
        verify::placements(b"BT\n/F1 12 Tf\n72 692 Td\nXabc) Tj\nET\n"),
        "unexpected Tj operand",
    );
    assert_err(
        verify::placements(b"BT\n/F1 12 Tf\n72 692 Td\n<ABC> Tj\nET\n"),
        "odd-length",
    );
    assert_err(
        verify::placements(b"BT\n/F1 12 Tf\n72 692 Td\n<ZZ> Tj\nET\n"),
        "bad hex",
    );
    assert_err(
        verify::placements(b"BT\n/F1 12 Tf\n72 692 Td\n<\xff\xff> Tj\nET\n"),
        "not ASCII",
    );
}

#[test]
fn rules_skips_unfilled_rects_and_rejects_malformed_lines() {
    assert!(verify::rules(b"1 2 3 4 re\n").unwrap().is_empty());
    assert_err(
        verify::rules(b"10 20 wide 40 re f\n"),
        "malformed rectangle",
    );
    assert_err(verify::rules(b"\xff 20 30 40 re f\n"), "not ASCII");
}
