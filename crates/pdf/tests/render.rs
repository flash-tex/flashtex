//! Acceptance tests for FT-009: page dimensions, text placement, multiline
//! output, Unicode/font behaviour, structural validity, and (on macOS) that the
//! system's own PDF reader opens the fixture output.

use flashtex_pdf::verify::{check_structure, placements, rules, stream_data};
use flashtex_pdf::{CompileResult, Item, Page, PdfError, TextItem, render_envelope, render_pdf};

const FIXTURE: &str = include_str!("../../../protocol/fixtures/compile-result.json");
/// Exact output of `flashtex-compiler` at de1020c for
/// `tests/fixtures/math-compile-request.json` (`$\\frac{a}{b}+\\alpha+\\sqrt{x}$`),
/// the reproduction from GitHub issue #9.
const MATH_RESULT: &str = include_str!("fixtures/math-compile-result.json");

fn text_item(text: &str, x: f64, baseline: f64, size: f64) -> TextItem {
    TextItem {
        text: text.into(),
        x_pt: x,
        baseline_y_pt: baseline,
        font_size_pt: size,
        font: None,
    }
}

fn item(text: &str, x: f64, baseline: f64, size: f64) -> Item {
    Item::Text(text_item(text, x, baseline, size))
}

fn page(number: u32, w: f64, h: f64, items: Vec<Item>) -> Page {
    Page {
        number,
        width_pt: w,
        height_pt: h,
        items,
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn count(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len()).filter(|w| *w == needle).count()
}

#[test]
fn fixture_renders_one_letter_page_with_valid_structure() {
    let out = render_envelope(FIXTURE).expect("fixture renders");
    assert!(
        out.warnings.is_empty(),
        "fixture is plain ASCII: {:?}",
        out.warnings
    );
    assert!(out.bytes.starts_with(b"%PDF-1.4\n"));
    assert!(out.bytes.ends_with(b"%%EOF\n"));

    let s = check_structure(&out.bytes).expect("xref offsets point at their objects");
    // catalog, pages, two fonts, info, one page, one content stream
    assert_eq!(s.object_count, 7);
    assert_eq!(s.stream_objects, vec![7]);

    assert_eq!(
        count(&out.bytes, b"/Type /Page\n"),
        0,
        "no stray page markers"
    );
    assert_eq!(count(&out.bytes, b"/Type /Page "), 1);
    assert!(find(&out.bytes, b"/MediaBox [ 0 0 612 792 ]").is_some());
    assert!(find(&out.bytes, b"/Count 1 ").is_some());
    assert!(
        find(
            &out.bytes,
            b"/BaseFont /Times-Roman /Encoding /WinAnsiEncoding"
        )
        .is_some()
    );
    assert!(find(&out.bytes, b"/BaseFont /Symbol >>").is_some());
    assert!(find(&out.bytes, b"/Font << /F1 3 0 R /F2 4 0 R >>").is_some());

    let content = stream_data(&out.bytes, 7).unwrap();
    let placed = placements(&content).unwrap();
    assert_eq!(placed.len(), 1);
    assert_eq!(placed[0].font, "F1");
    assert_eq!(placed[0].font_size, 12.0);
    assert_eq!((placed[0].x, placed[0].y), (72.0, 792.0 - 84.0));
    assert_eq!(placed[0].bytes, b"Hello FlashTeX.");
}

#[test]
fn two_synthetic_pages_get_their_own_mediabox_and_content() {
    let result = CompileResult {
        pages: vec![
            page(1, 612.0, 792.0, vec![item("first", 72.0, 84.0, 12.0)]),
            page(2, 595.28, 841.89, vec![item("second", 56.7, 70.9, 10.5)]),
        ],
        capabilities: None,
    };
    let out = render_pdf(&result).unwrap();
    let s = check_structure(&out.bytes).unwrap();
    assert_eq!(s.object_count, 9);
    assert_eq!(s.stream_objects, vec![7, 9]);
    assert!(find(&out.bytes, b"/Count 2 ").is_some());
    assert!(find(&out.bytes, b"/Kids [ 6 0 R 8 0 R ]").is_some());

    let p6 = find(&out.bytes, b"\n6 0 obj\n").unwrap();
    let p8 = find(&out.bytes, b"\n8 0 obj\n").unwrap();
    let obj6 = &out.bytes[p6..p8];
    assert!(find(obj6, b"/MediaBox [ 0 0 612 792 ]").is_some());
    assert!(find(obj6, b"/Contents 7 0 R").is_some());
    let obj8 = &out.bytes[p8..];
    assert!(find(obj8, b"/MediaBox [ 0 0 595.28 841.89 ]").is_some());
    assert!(find(obj8, b"/Contents 9 0 R").is_some());

    let second = placements(&stream_data(&out.bytes, 9).unwrap()).unwrap();
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].font_size, 10.5);
    assert_eq!(second[0].x, 56.7);
    assert!((second[0].y - (841.89 - 70.9)).abs() < 0.0005);
    assert_eq!(second[0].bytes, b"second");
}

#[test]
fn multiline_placement_maps_every_baseline_to_pdf_space() {
    // Three lines as the FT-002 layout would emit them: one item per word, a
    // 14.4pt leading, and a heading at a larger size.
    let items = vec![
        text_item("Title", 72.0, 89.0, 17.0),
        text_item("Hello", 72.0, 115.4, 12.0),
        text_item("world", 105.36, 115.4, 12.0),
        text_item("Second", 72.0, 129.8, 12.0),
        text_item("line", 114.36, 129.8, 12.0),
        text_item("Third", 72.0, 144.2, 12.0),
    ];
    let result = CompileResult {
        pages: vec![page(
            1,
            612.0,
            792.0,
            items.iter().cloned().map(Item::Text).collect(),
        )],
        capabilities: None,
    };
    let out = render_pdf(&result).unwrap();
    check_structure(&out.bytes).unwrap();
    let placed = placements(&stream_data(&out.bytes, 7).unwrap()).unwrap();
    assert_eq!(placed.len(), items.len());
    for (p, i) in placed.iter().zip(&items) {
        assert_eq!(p.font_size, i.font_size_pt, "{}", i.text);
        assert_eq!(p.x, i.x_pt, "{}", i.text);
        assert!(
            (p.y - (792.0 - i.baseline_y_pt)).abs() < 0.0005,
            "{}",
            i.text
        );
        assert_eq!(p.bytes, i.text.as_bytes());
    }
    let baselines: std::collections::BTreeSet<String> =
        placed.iter().map(|p| format!("{:.3}", p.y)).collect();
    assert_eq!(baselines.len(), 4, "four distinct baselines survive");
}

#[test]
fn winansi_characters_are_encoded_and_others_warn() {
    let result = CompileResult {
        pages: vec![page(
            1,
            612.0,
            792.0,
            vec![
                item("café — naïve", 72.0, 84.0, 12.0),
                item("中 x dx 😀", 72.0, 100.0, 12.0),
                item("(paren) back\\slash", 72.0, 120.0, 12.0),
            ],
        )],
        capabilities: None,
    };
    let out = render_pdf(&result).unwrap();
    check_structure(&out.bytes).unwrap();
    let placed = placements(&stream_data(&out.bytes, 7).unwrap()).unwrap();
    assert_eq!(placed.len(), 3);

    // é -> 0xE9, — -> 0x97, ï -> 0xEF: WinAnsi bytes, not UTF-8 sequences.
    assert_eq!(placed[0].bytes, b"caf\xE9 \x97 na\xEFve");
    assert!(
        find(&out.bytes, b"caf\xE9").is_some(),
        "byte 0xE9 appears literally"
    );
    assert!(
        find(&out.bytes, "café".as_bytes()).is_none(),
        "no UTF-8 leak"
    );

    // Unrepresentable characters become '?' and are reported, never dropped.
    assert_eq!(placed[1].bytes, b"? x dx ?");
    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
    let w = &out.warnings[0];
    assert!(w.contains("page 1"), "{w}");
    assert!(w.contains("item 1"), "{w}");
    assert!(w.contains("U+4E2D"), "{w}");
    assert!(w.contains("U+1F600"), "{w}");
    assert!(w.contains("WinAnsi"), "{w}");
    assert!(w.contains("Symbol"), "{w}");

    // Delimiters are escaped in the file and round-trip through the reader.
    assert_eq!(placed[2].bytes, b"(paren) back\\slash");
    assert!(find(&out.bytes, b"(\\(paren\\) back\\\\slash) Tj").is_some());
}

#[test]
fn unsupported_item_kinds_and_bad_envelopes_are_explicit() {
    // Legacy route (no layout_capabilities): unknown kinds are skipped with
    // a warning; a rule can only come from an unrequested capability, so it
    // is an error even here.
    let json = r#"{"protocol_version":1,"id":"x","type":"compile_result","payload":{"pages":[
        {"number":1,"width_pt":612,"height_pt":792,"items":[
            {"kind":"image","x_pt":1,"y_pt":2},
            {"kind":"text","text":"ok","x_pt":72,"baseline_y_pt":84,"font_size_pt":12}
        ]}]}}"#;
    let out = render_envelope(json).unwrap();
    assert_eq!(out.warnings.len(), 1);
    assert!(out.warnings[0].contains("\"image\""));
    assert!(out.warnings[0].contains("skipped"));
    let legacy_rule = json.replace("\"kind\":\"image\"", "\"kind\":\"rule\"");
    assert!(
        matches!(render_envelope(&legacy_rule), Err(PdfError::Protocol(m)) if m.contains("rules-v1"))
    );

    let v9 = json.replace("\"protocol_version\":1", "\"protocol_version\":9");
    assert!(matches!(render_envelope(&v9), Err(PdfError::Protocol(_))));
    let wrong = json.replace(
        "\"type\":\"compile_result\"",
        "\"type\":\"capture_proposal\"",
    );
    assert!(matches!(
        render_envelope(&wrong),
        Err(PdfError::Protocol(_))
    ));
    assert!(matches!(
        render_envelope("not json"),
        Err(PdfError::Json(_))
    ));

    assert!(matches!(
        render_pdf(&CompileResult::default()),
        Err(PdfError::Invalid(_))
    ));
    let zero = CompileResult {
        pages: vec![page(1, 0.0, 792.0, vec![])],
        capabilities: None,
    };
    assert!(matches!(render_pdf(&zero), Err(PdfError::Invalid(_))));
    let nan = CompileResult {
        pages: vec![page(1, 612.0, 792.0, vec![item("x", f64::NAN, 1.0, 12.0)])],
        capabilities: None,
    };
    assert!(matches!(render_pdf(&nan), Err(PdfError::Invalid(_))));
}

#[test]
fn export_is_white_and_theme_independent() {
    // The writer has no theme input at all; the only colour operator it emits
    // is black text, and it never paints a background rectangle.
    let out = render_envelope(FIXTURE).unwrap();
    let content = stream_data(&out.bytes, 7).unwrap();
    assert!(
        content.starts_with(b"0 g\n"),
        "black non-stroking colour set explicitly"
    );
    assert_eq!(count(&content, b" g\n"), 1, "no other gray colour changes");
    for op in [b" rg\n".as_slice(), b" k\n", b" re\n", b" f\n", b" G\n"] {
        assert!(
            find(&content, op).is_none(),
            "no fill/rect/stroke operator {:?}",
            op
        );
    }
    assert!(find(&out.bytes, b"/Group").is_none());
}

#[test]
fn issue_9_math_repro_renders_rule_greek_and_radical_without_warnings() {
    let out = render_envelope(MATH_RESULT).expect("math result renders");
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    check_structure(&out.bytes).unwrap();
    let content = stream_data(&out.bytes, 7).unwrap();

    // The fraction bar "──" (2 × U+2500 at 8.4pt, x=72, baseline 84.72) is a
    // filled rectangle, not glyphs: width 2 × 0.5 × 8.4 = 8.4pt, thickness
    // 0.06/0.7 × 8.4 = 0.72pt, bottom edge on the baseline.
    let bars = rules(&content).unwrap();
    assert_eq!(bars.len(), 1, "{bars:?}");
    let bar = &bars[0];
    assert_eq!(bar.x, 72.0);
    assert!((bar.width - 8.4).abs() < 0.0005, "{bar:?}");
    assert!((bar.height - 0.72).abs() < 0.0005, "{bar:?}");
    assert!((bar.y - (792.0 - 84.72)).abs() < 0.0005, "{bar:?}");
    assert!(find(&content, "─".as_bytes()).is_none(), "no U+2500 glyphs");
    assert!(find(&content, b"?").is_none(), "nothing substituted");

    // Text items, in compiler order, minus the rule: a b + α + √ x.
    let placed = placements(&content).unwrap();
    let summary: Vec<(&str, Vec<u8>)> = placed
        .iter()
        .map(|p| (p.font.as_str(), p.bytes.clone()))
        .collect();
    assert_eq!(
        summary,
        vec![
            ("F1", b"a".to_vec()),
            ("F1", b"b".to_vec()),
            ("F1", b"+".to_vec()),
            ("F2", vec![0x61]), // alpha in Symbol
            ("F1", b"+".to_vec()),
            ("F2", vec![0xD6]), // radical in Symbol
            ("F1", b"x".to_vec()),
        ]
    );
    let alpha = &placed[3];
    assert_eq!(alpha.font_size, 12.0);
    assert_eq!(alpha.x, 87.17);
    assert!((alpha.y - (792.0 - 87.36)).abs() < 0.0005);
    let radical = &placed[5];
    assert_eq!(radical.x, 99.94);
    assert!(placed.iter().all(|p| !p.continues));
    assert!(
        placed
            .iter()
            .filter(|p| p.font == "F1")
            .all(|p| p.font_size == 12.0 || (p.font_size - 8.4).abs() < 1e-9)
    );
}

#[test]
fn mixed_item_switches_fonts_inside_one_text_object() {
    let result = CompileResult {
        pages: vec![page(
            1,
            612.0,
            792.0,
            vec![
                item("x∈ℝ→∞", 72.0, 84.0, 12.0),
                item("─", 72.0, 100.0, 10.0),
            ],
        )],
        capabilities: None,
    };
    let out = render_pdf(&result).unwrap();
    check_structure(&out.bytes).unwrap();
    assert!(find(&out.bytes, b"/Font << /F1 3 0 R /F2 4 0 R >>").is_some());
    let content = stream_data(&out.bytes, 7).unwrap();

    // ℝ (U+211D) is in neither font: substituted and reported; the rest of
    // the item still switches between Times and Symbol within one BT block.
    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
    assert!(out.warnings[0].contains("U+211D"));
    let placed = placements(&content).unwrap();
    let runs: Vec<(&str, bool, Vec<u8>)> = placed
        .iter()
        .map(|p| (p.font.as_str(), p.continues, p.bytes.clone()))
        .collect();
    assert_eq!(
        runs,
        vec![
            ("F1", false, b"x".to_vec()),
            ("F2", true, vec![0xCE]),
            ("F1", true, b"?".to_vec()),
            ("F2", true, vec![0xAE, 0xA5]),
        ]
    );
    assert!(placed.iter().all(|p| p.x == 72.0 && p.y == 708.0));
    assert_eq!(count(&content, b"BT\n"), 1);
    assert_eq!(count(&content, b" Tf\n"), 4);

    // A single dash is still a rule: 0.5 em wide, 0.06/0.7 em thick.
    let bars = rules(&content).unwrap();
    assert_eq!(bars.len(), 1);
    assert!((bars[0].width - 5.0).abs() < 0.0005);
    assert!((bars[0].height - 10.0 * 0.06 / 0.7).abs() < 0.0005);
    assert_eq!(bars[0].y, 692.0);
}

#[test]
fn output_is_deterministic() {
    let a = render_envelope(FIXTURE).unwrap().bytes;
    let b = render_envelope(FIXTURE).unwrap().bytes;
    assert_eq!(a, b);
}

#[test]
fn cli_writes_fixture_pdf_and_rejects_unknown_type() {
    let exe = env!("CARGO_BIN_EXE_flashtex-pdf");
    let dir = std::env::temp_dir().join(format!("flashtex-pdf-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("fixture.pdf");

    let status = std::process::Command::new(exe)
        .arg("--out")
        .arg(&out)
        .arg("--verify")
        .stdin(std::fs::File::open(fixture_path()).unwrap())
        .status()
        .unwrap();
    assert!(status.success());
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(bytes, render_envelope(FIXTURE).unwrap().bytes);

    let bad = dir.join("bad.json");
    std::fs::write(&bad, FIXTURE.replace("compile_result", "compile")).unwrap();
    let output = std::process::Command::new(exe)
        .arg(&bad)
        .arg("--out")
        .arg(dir.join("bad.pdf"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("\"compile\""), "{stderr}");
    assert!(!dir.join("bad.pdf").exists());

    let _ = std::fs::remove_dir_all(&dir);
}

fn fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../protocol/fixtures/compile-result.json")
}

/// macOS ships a PDF reader in `sips`; if it can report pixel dimensions the
/// file parsed as a document, independent of this crate's own checks.
#[cfg(target_os = "macos")]
#[test]
fn macos_sips_opens_the_fixture_pdf() {
    let out = render_envelope(FIXTURE).unwrap();
    let dir = std::env::temp_dir().join(format!("flashtex-pdf-sips-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("fixture.pdf");
    std::fs::write(&path, &out.bytes).unwrap();

    let output = std::process::Command::new("/usr/bin/sips")
        .args(["-g", "pixelWidth", "-g", "pixelHeight"])
        .arg(&path)
        .output()
        .expect("sips exists on macOS");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "sips failed: {stdout}{stderr}");
    assert!(stdout.contains("pixelWidth: 612"), "{stdout}");
    assert!(stdout.contains("pixelHeight: 792"), "{stdout}");
    assert!(!stderr.contains("Error"), "{stderr}");

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Opt-in font embedding. These tests need a real TrueType font; they use the
// same discovery as the CLI (`FLASHTEX_UNICODE_FONT`, then macOS system fonts)
// and skip with a message when none exists.

/// First candidate on this machine with the requested outline format, using
/// the same search list as `--embed-font auto` (Latin Modern first, then the
/// macOS system TrueType fonts); `FLASHTEX_UNICODE_FONT` is tried first.
fn font_with_outlines_or_skip(
    test: &str,
    outlines: flashtex_pdf::truetype::Outlines,
) -> Option<flashtex_pdf::embed::EmbedFont> {
    let mut candidates = Vec::new();
    if let Some(p) = std::env::var_os(flashtex_pdf::embed::ENV_VAR) {
        candidates.push(std::path::PathBuf::from(p));
    }
    candidates.extend(flashtex_pdf::embed::candidate_paths());
    for path in candidates {
        if !path.is_file() {
            continue;
        }
        let font = flashtex_pdf::embed::EmbedFont::load(&path)
            .unwrap_or_else(|e| panic!("candidate font failed to parse: {e}"));
        if font.font.outlines == outlines {
            return Some(font);
        }
    }
    eprintln!(
        "SKIPPED {test}: no {outlines:?} font found (set {} or install one of {:?})",
        flashtex_pdf::embed::ENV_VAR,
        flashtex_pdf::embed::candidate_paths()
    );
    None
}

fn embed_font_or_skip(test: &str) -> Option<flashtex_pdf::embed::EmbedFont> {
    font_with_outlines_or_skip(test, flashtex_pdf::truetype::Outlines::TrueType)
}

/// Characters outside WinAnsi and Symbol: Cyrillic (in every candidate
/// font), CJK and double-struck R (coverage varies), and an emoji (in none).
const EMBED_TEXT: &str = "ж中ℝ😀";

#[test]
fn embedded_subset_font_covers_unicode_and_reports_the_rest() {
    use flashtex_pdf::embed::parse_to_unicode;
    use flashtex_pdf::truetype::{TrueTypeFont, verify_checksums};
    let Some(font) = embed_font_or_skip("embedded_subset_font_covers_unicode_and_reports_the_rest")
    else {
        return;
    };
    let result = CompileResult {
        pages: vec![page(
            1,
            612.0,
            792.0,
            vec![
                item("plain", 72.0, 84.0, 12.0),
                item(EMBED_TEXT, 72.0, 100.0, 12.0),
            ],
        )],
        capabilities: None,
    };
    let options = flashtex_pdf::RenderOptions {
        embed_font: Some(font.clone()),
        face: flashtex_pdf::encoding::Face::Times,
    };
    let out = flashtex_pdf::render_pdf_with(&result, &options).unwrap();
    let s = check_structure(&out.bytes).unwrap();
    // 7 base objects + 5 font objects; FontFile2 and ToUnicode are streams.
    assert_eq!(s.object_count, 12);
    assert_eq!(s.stream_objects, vec![7, 11, 12]);
    assert!(find(&out.bytes, b"/Subtype /Type0 /BaseFont /").is_some());
    assert!(find(&out.bytes, b"/Encoding /Identity-H").is_some());
    assert!(find(&out.bytes, b"/Subtype /CIDFontType2").is_some());
    assert!(find(&out.bytes, b"/CIDToGIDMap /Identity").is_some());
    assert!(find(&out.bytes, b"/FontFile2 11 0 R").is_some());
    assert!(find(&out.bytes, b"/ToUnicode 12 0 R").is_some());
    assert!(find(&out.bytes, b"/Font << /F1 3 0 R /F2 4 0 R /F3 8 0 R >>").is_some());

    // Every character either went through /F3 with a ToUnicode entry mapping
    // its glyph id back to it, or is named in a warning. Nothing vanishes.
    let to_unicode = parse_to_unicode(&stream_data(&out.bytes, 12).unwrap()).unwrap();
    let placed = placements(&stream_data(&out.bytes, 7).unwrap()).unwrap();
    let mut embedded_chars = Vec::new();
    for p in placed.iter().filter(|p| p.font == "F3") {
        for gid in p.bytes.chunks(2) {
            let gid = u16::from_be_bytes([gid[0], gid[1]]);
            assert_ne!(gid, 0, "never write .notdef");
            embedded_chars.push(
                *to_unicode
                    .get(&gid)
                    .unwrap_or_else(|| panic!("gid {gid} has no ToUnicode entry")),
            );
        }
    }
    let warned = out.warnings.join("\n");
    for c in EMBED_TEXT.chars() {
        let code = format!("U+{:04X}", c as u32);
        assert!(
            embedded_chars.contains(&c) || warned.contains(&code),
            "{c:?} neither embedded nor warned about; warnings: {warned}"
        );
    }
    assert!(
        embedded_chars.contains(&'ж'),
        "Cyrillic must be in any candidate font"
    );
    assert!(
        !embedded_chars.contains(&'😀'),
        "no candidate font has colour emoji outlines"
    );
    assert!(warned.contains("U+1F600"), "{warned}");
    assert!(warned.contains(&font.font.postscript_name), "{warned}");
    // Substituted characters are still written as '?' in a Times run.
    let substituted: Vec<u8> = placed
        .iter()
        .filter(|p| p.font == "F1" && p.y == 692.0)
        .flat_map(|p| p.bytes.clone())
        .collect();
    assert!(substituted.iter().all(|&b| b == b'?'), "{substituted:?}");
    assert_eq!(
        substituted.len(),
        EMBED_TEXT.chars().count() - embedded_chars.len()
    );
    assert_eq!(placed[0].font, "F1");
    assert_eq!(placed[0].bytes, b"plain");

    // The embedded program parses back as a consistent TrueType font whose
    // glyph count is exactly the used glyphs plus .notdef plus the components
    // composites pulled in, with valid table checksums.
    let program = stream_data(&out.bytes, 11).unwrap();
    verify_checksums(&program).unwrap();
    let parsed = TrueTypeFont::parse(program.clone()).unwrap();
    let subset = font.subset_for(EMBED_TEXT.chars()).unwrap();
    let flashtex_pdf::embed::Program::TrueType(tt) = &subset.program else {
        panic!("expected a TrueType subset");
    };
    assert_eq!(program, tt.bytes, "deterministic subset");
    assert_eq!(parsed.num_glyphs(), tt.num_glyphs());
    assert_eq!(subset.chars.len(), embedded_chars.len());
    assert!(parsed.num_glyphs() as usize > embedded_chars.len());
    assert_eq!(parsed.units_per_em, font.font.units_per_em);
    let composites = parsed.num_glyphs() as usize - embedded_chars.len() - 1;
    eprintln!(
        "embedded {} from {}: {} glyphs ({} used + .notdef + {composites} composite parts), {} bytes",
        font.font.postscript_name,
        font.source.display(),
        parsed.num_glyphs(),
        embedded_chars.len(),
        program.len()
    );
    for (&c, &gid) in &subset.chars {
        assert_eq!(to_unicode.get(&gid), Some(&c));
        let original = font.font.glyph_id(c).unwrap();
        assert_eq!(parsed.advance(gid), font.font.advance(original));
    }
    assert!(
        program.len() < 60_000,
        "subset should be small, got {}",
        program.len()
    );
}

#[test]
fn embedding_is_off_by_default_and_a_bad_font_path_is_an_error() {
    let result = CompileResult {
        pages: vec![page(1, 612.0, 792.0, vec![item("ж", 72.0, 84.0, 12.0)])],
        capabilities: None,
    };
    let out = render_pdf(&result).unwrap();
    assert!(find(&out.bytes, b"/Type0").is_none());
    assert!(find(&out.bytes, b"/F3").is_none());
    assert_eq!(out.warnings.len(), 1);
    assert!(out.warnings[0].contains("U+0436"));

    let missing =
        flashtex_pdf::embed::EmbedFont::load(std::path::Path::new("/nonexistent/font.ttf"));
    assert!(missing.is_err());
    let not_a_font =
        std::env::temp_dir().join(format!("flashtex-pdf-notafont-{}.ttf", std::process::id()));
    std::fs::write(&not_a_font, b"ttcf this is not really a font").unwrap();
    let err = flashtex_pdf::embed::EmbedFont::load(&not_a_font).unwrap_err();
    assert!(err.contains("collection"), "{err}");
    std::fs::write(&not_a_font, b"OTTO\0\0\0\0\0\0\0\0").unwrap();
    let err = flashtex_pdf::embed::EmbedFont::load(&not_a_font).unwrap_err();
    assert!(err.contains("missing required table"), "{err}");
    let _ = std::fs::remove_file(&not_a_font);
}

#[cfg(target_os = "macos")]
#[test]
fn cli_embed_font_writes_a_pdf_that_sips_opens() {
    let Some(font) = embed_font_or_skip("cli_embed_font_writes_a_pdf_that_sips_opens") else {
        return;
    };
    let exe = env!("CARGO_BIN_EXE_flashtex-pdf");
    let dir = std::env::temp_dir().join(format!("flashtex-pdf-embed-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("unicode.json");
    let json = format!(
        r#"{{"protocol_version":1,"id":"u","type":"compile_result","payload":{{"pages":[{{"number":1,"width_pt":612,"height_pt":792,"items":[{{"kind":"text","text":"{EMBED_TEXT}","x_pt":72,"baseline_y_pt":84,"font_size_pt":12}}]}}]}}}}"#
    );
    std::fs::write(&input, json).unwrap();
    let out = dir.join("unicode.pdf");
    let output = std::process::Command::new(exe)
        .arg(&input)
        .arg("--out")
        .arg(&out)
        .arg("--verify")
        .arg("--embed-font")
        .arg(&font.source)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(
        stderr.contains("note: embedding ") && stderr.contains("(Times remains the face)"),
        "{stderr}"
    );
    assert!(
        stderr.contains("warning:") && stderr.contains("U+1F600"),
        "{stderr}"
    );
    assert!(
        stderr.contains("warning(s); the PDF was written"),
        "{stderr}"
    );

    let sips = std::process::Command::new("/usr/bin/sips")
        .args(["-g", "pixelWidth", "-g", "pixelHeight"])
        .arg(&out)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&sips.stdout);
    assert!(
        sips.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&sips.stderr)
    );
    assert!(stdout.contains("pixelWidth: 612"), "{stdout}");
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// CFF OpenType (Latin Modern) embedding: the whole `CFF ` table, verbatim.

/// Characters Latin Modern Roman has beyond WinAnsi/Symbol (Latin Extended),
/// plus one it lacks (CJK) that must be warned about.
const CFF_TEXT: &str = "ŵŷ ő 中";

#[test]
fn latin_modern_cff_is_embedded_whole_and_verbatim() {
    use flashtex_pdf::embed::{Program, parse_to_unicode};
    use flashtex_pdf::truetype::Outlines;
    let Some(font) = font_with_outlines_or_skip(
        "latin_modern_cff_is_embedded_whole_and_verbatim",
        Outlines::Cff,
    ) else {
        return;
    };
    let cff = font
        .font
        .cff_table()
        .expect("OTTO has a CFF table")
        .to_vec();
    assert_eq!(cff[0], 1, "CFF major version");
    let result = CompileResult {
        pages: vec![page(
            1,
            612.0,
            792.0,
            vec![item(CFF_TEXT, 72.0, 84.0, 14.0)],
        )],
        capabilities: None,
    };
    let options = flashtex_pdf::RenderOptions {
        embed_font: Some(font.clone()),
        face: flashtex_pdf::encoding::Face::Times,
    };
    let out = flashtex_pdf::render_pdf_with(&result, &options).unwrap();
    let s = check_structure(&out.bytes).unwrap();
    assert_eq!(s.object_count, 12);
    assert_eq!(s.stream_objects, vec![7, 11, 12]);

    // Font object chain for a whole CFF: CIDFontType0, FontFile3/CIDFontType0C,
    // no CIDToGIDMap, no subset tag on the name.
    let name = format!("/BaseFont /{}", font.font.postscript_name);
    assert!(find(&out.bytes, name.as_bytes()).is_some());
    assert!(find(&out.bytes, b"/Subtype /Type0").is_some());
    assert!(find(&out.bytes, b"/Encoding /Identity-H").is_some());
    assert!(find(&out.bytes, b"/Subtype /CIDFontType0 ").is_some());
    assert!(find(&out.bytes, b"/CIDToGIDMap").is_none());
    assert!(find(&out.bytes, b"/FontFile3 11 0 R").is_some());
    assert!(find(&out.bytes, b"/Subtype /CIDFontType0C").is_some());
    assert!(find(&out.bytes, b"/FontFile2").is_none());
    let tagged = format!("+{}", font.font.postscript_name);
    assert!(
        find(&out.bytes, tagged.as_bytes()).is_none(),
        "whole font carries no subset tag"
    );

    // The embedded program is the CFF table, byte for byte.
    let program = stream_data(&out.bytes, 11).unwrap();
    assert_eq!(program.len(), cff.len());
    assert_eq!(program, cff, "CFF table must be embedded verbatim");
    assert!(
        out.bytes.len() > cff.len() && out.bytes.len() < cff.len() + 4096,
        "one-line PDF is the CFF plus a little structure, got {} bytes",
        out.bytes.len()
    );

    // Glyph ids are the original font's; ToUnicode maps them back.
    let subset = font.subset_for(CFF_TEXT.chars()).unwrap();
    let Program::Cff {
        bytes,
        used_advances,
    } = &subset.program
    else {
        panic!("expected a whole-CFF program");
    };
    assert_eq!(bytes, &cff);
    let to_unicode = parse_to_unicode(&stream_data(&out.bytes, 12).unwrap()).unwrap();
    let placed = placements(&stream_data(&out.bytes, 7).unwrap()).unwrap();
    let mut embedded_chars = Vec::new();
    for p in placed.iter().filter(|p| p.font == "F3") {
        for gid in p.bytes.chunks(2) {
            let gid = u16::from_be_bytes([gid[0], gid[1]]);
            let c = *to_unicode.get(&gid).expect("ToUnicode entry");
            assert_eq!(
                font.font.glyph_id(c),
                Some(gid),
                "CID must be the font's own GID"
            );
            assert!(used_advances.contains_key(&gid));
            embedded_chars.push(c);
        }
    }
    assert_eq!(embedded_chars, vec!['ŵ', 'ŷ', 'ő']);
    let warned = out.warnings.join("\n");
    assert!(warned.contains("U+4E2D"), "{warned}");
    assert!(warned.contains(&font.font.postscript_name), "{warned}");
    // Sparse /W with one entry per used glyph.
    for gid in embedded_chars
        .iter()
        .map(|&c| font.font.glyph_id(c).unwrap())
    {
        let w = (font.font.advance(gid) as f64 * 1000.0 / font.font.units_per_em as f64).round();
        assert!(
            find(&out.bytes, format!("{gid} [ {w} ]").as_bytes()).is_some(),
            "/W entry for {gid}"
        );
    }
}

#[test]
fn auto_discovery_prefers_latin_modern_when_present() {
    let paths = flashtex_pdf::embed::candidate_paths();
    let first_existing = paths.iter().find(|p| p.is_file());
    let Some(first) = first_existing else {
        eprintln!(
            "SKIPPED auto_discovery_prefers_latin_modern_when_present: no candidate font exists"
        );
        return;
    };
    let lm_exists = paths
        .iter()
        .any(|p| p.is_file() && p.ends_with(flashtex_pdf::embed::LATIN_MODERN_FILE));
    if lm_exists {
        assert!(
            first.ends_with(flashtex_pdf::embed::LATIN_MODERN_FILE),
            "{first:?}"
        );
    }
    let lm_index = paths
        .iter()
        .position(|p| p.ends_with(flashtex_pdf::embed::LATIN_MODERN_FILE));
    let tnr_index = paths
        .iter()
        .position(|p| p.ends_with("Times New Roman.ttf"));
    if let (Some(l), Some(t)) = (lm_index, tnr_index) {
        assert!(
            l < t,
            "Latin Modern must be searched before Times New Roman"
        );
    }
    if std::env::var_os(flashtex_pdf::embed::ENV_VAR).is_none() {
        let discovered = flashtex_pdf::embed::EmbedFont::discover().unwrap().unwrap();
        assert_eq!(&discovered.source, first);
    }
}

#[cfg(target_os = "macos")]
#[test]
fn latin_modern_pdf_renders_in_sips_and_round_trips_through_pdfkit() {
    let Some(font) = font_with_outlines_or_skip(
        "latin_modern_pdf_renders_in_sips_and_round_trips_through_pdfkit",
        flashtex_pdf::truetype::Outlines::Cff,
    ) else {
        return;
    };
    let exe = env!("CARGO_BIN_EXE_flashtex-pdf");
    let dir = std::env::temp_dir().join(format!("flashtex-pdf-lm-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("lm.json");
    let text = "Latin Modern ŵŷ ő";
    let json = format!(
        r#"{{"protocol_version":1,"id":"lm","type":"compile_result","payload":{{"pages":[{{"number":1,"width_pt":612,"height_pt":792,"items":[{{"kind":"text","text":"{text}","x_pt":72,"baseline_y_pt":84,"font_size_pt":14}}]}}]}}}}"#
    );
    std::fs::write(&input, json).unwrap();
    let out = dir.join("lm.pdf");
    let output = std::process::Command::new(exe)
        .arg(&input)
        .arg("--out")
        .arg(&out)
        .arg("--verify")
        .arg("--embed-font")
        .arg(&font.source)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(
        !stderr.contains("warning:"),
        "all characters are in Latin Modern: {stderr}"
    );
    let size = std::fs::metadata(&out).unwrap().len();
    eprintln!(
        "one-line PDF with {} embedded whole: {size} bytes",
        font.font.postscript_name
    );

    // CoreGraphics must accept the font program: with CG_PDF_VERBOSE it
    // reports unsupported font programs on stderr while still rasterising.
    let sips = std::process::Command::new("/usr/bin/sips")
        .env("CG_PDF_VERBOSE", "1")
        .args(["-s", "format", "png"])
        .arg(&out)
        .arg("--out")
        .arg(dir.join("lm.png"))
        .output()
        .unwrap();
    let sips_err = String::from_utf8_lossy(&sips.stderr);
    assert!(sips.status.success(), "{sips_err}");
    assert!(
        !sips_err.contains("unsupported") && !sips_err.contains("Replacing"),
        "CoreGraphics did not accept the embedded program: {sips_err}"
    );
    assert!(dir.join("lm.png").is_file());

    // PDFKit text extraction round-trips the characters through ToUnicode.
    let script = "import sys\nfrom Quartz import PDFDocument\nfrom Foundation import NSURL\n\
                  d = PDFDocument.alloc().initWithURL_(NSURL.fileURLWithPath_(sys.argv[1]))\n\
                  print(d.pageAtIndex_(0).string())\n";
    let py = std::process::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&out)
        .output()
        .unwrap();
    let py_err = String::from_utf8_lossy(&py.stderr);
    if !py.status.success() && py_err.contains("No module named") {
        eprintln!(
            "SKIPPED PDFKit round-trip: PyObjC Quartz not available ({})",
            py_err.trim()
        );
    } else {
        assert!(py.status.success(), "{py_err}");
        let extracted = String::from_utf8_lossy(&py.stdout);
        assert_eq!(extracted.trim(), text, "PDFKit page.string round-trip");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// The embedded font as the document face (Latin Modern primary).

/// Width of `text` at `size` from the font's own advances, in points.
#[cfg(target_os = "macos")]
fn advance_width(font: &flashtex_pdf::embed::EmbedFont, text: &str, size: f64) -> f64 {
    text.chars()
        .map(|c| font.font.advance(font.font.glyph_id(c).expect("glyph")) as f64)
        .sum::<f64>()
        * size
        / font.font.units_per_em as f64
}

/// Times-Roman AFM widths for the letters in "Latin Modern", 1/1000 em.
#[cfg(target_os = "macos")]
fn times_width(text: &str, size: f64) -> f64 {
    text.chars()
        .map(|c| match c {
            'L' => 611.0,
            'a' => 444.0,
            't' => 278.0,
            'i' => 278.0,
            'n' => 500.0,
            ' ' => 250.0,
            'M' => 889.0,
            'o' => 500.0,
            'd' => 500.0,
            'e' => 444.0,
            'r' => 333.0,
            other => panic!("no Times width for {other:?}"),
        })
        .sum::<f64>()
        * size
        / 1000.0
}

#[test]
fn latin_modern_as_document_face_sets_all_latin_text_in_it() {
    use flashtex_pdf::embed::{Program, parse_to_unicode};
    let Some(font) = font_with_outlines_or_skip(
        "latin_modern_as_document_face_sets_all_latin_text_in_it",
        flashtex_pdf::truetype::Outlines::Cff,
    ) else {
        return;
    };
    let text = "Latin Modern naïve — café";
    let result = CompileResult {
        pages: vec![page(1, 612.0, 792.0, vec![item(text, 72.0, 84.0, 12.0)])],
        capabilities: None,
    };
    assert_eq!(
        flashtex_pdf::RenderOptions::default_face_for(&font),
        flashtex_pdf::encoding::Face::Embedded,
        "Latin Modern implies the embedded face"
    );
    let options = flashtex_pdf::RenderOptions::with_document_face(font.clone());
    let out = flashtex_pdf::render_pdf_with(&result, &options).unwrap();
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    check_structure(&out.bytes).unwrap();

    // Every character went through /F3; Times and Symbol are unused.
    let content = stream_data(&out.bytes, 7).unwrap();
    assert!(
        find(&content, b"/F1 ").is_none(),
        "no Times run: {}",
        String::from_utf8_lossy(&content)
    );
    assert!(find(&content, b"/F2 ").is_none());
    assert_eq!(count(&content, b"/F3 12 Tf\n"), 1);
    let placed = placements(&content).unwrap();
    assert_eq!(placed.len(), 1);
    assert_eq!(placed[0].font, "F3");
    assert_eq!(placed[0].bytes.len(), 2 * text.chars().count());

    // /W carries the hmtx advance of every used glyph, and ToUnicode maps
    // every glyph back to its character.
    let subset = font.subset_for(text.chars()).unwrap();
    let Program::Cff { used_advances, .. } = &subset.program else {
        panic!("expected a whole-CFF program");
    };
    assert_eq!(
        used_advances.len(),
        text.chars()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );
    let to_unicode = parse_to_unicode(&stream_data(&out.bytes, 12).unwrap()).unwrap();
    let mut round_trip = String::new();
    for gid in placed[0].bytes.chunks(2) {
        let gid = u16::from_be_bytes([gid[0], gid[1]]);
        let w = (font.font.advance(gid) as f64 * 1000.0 / font.font.units_per_em as f64).round();
        assert!(
            find(&out.bytes, format!(" {gid} [ {w} ]").as_bytes()).is_some(),
            "/W for {gid}"
        );
        round_trip.push(to_unicode[&gid]);
    }
    assert_eq!(round_trip, text);
    assert!(find(&out.bytes, b"/W [ ]").is_none());

    // The same sample with the Times face still uses /F1 for Latin text.
    let times_options = flashtex_pdf::RenderOptions {
        embed_font: Some(font.clone()),
        face: flashtex_pdf::encoding::Face::Times,
    };
    let times_out = flashtex_pdf::render_pdf_with(&result, &times_options).unwrap();
    let times_content = stream_data(&times_out.bytes, 7).unwrap();
    assert!(find(&times_content, b"/F1 12 Tf").is_some());
    assert!(find(&times_content, b"/F3 ").is_none());
    assert!(
        find(&times_out.bytes, b"/W [ ]").is_some(),
        "no glyph used from the embedded font"
    );

    // Asking for the embedded face without a font is warned, not silent.
    let no_font = flashtex_pdf::RenderOptions {
        embed_font: None,
        face: flashtex_pdf::encoding::Face::Embedded,
    };
    let fallback = flashtex_pdf::render_pdf_with(&result, &no_font).unwrap();
    assert!(
        fallback
            .warnings
            .iter()
            .any(|w| w.contains("no font is embedded"))
    );
}

#[test]
fn latin_modern_face_keeps_rules_and_symbol_math() {
    let Some(font) = font_with_outlines_or_skip(
        "latin_modern_face_keeps_rules_and_symbol_math",
        flashtex_pdf::truetype::Outlines::Cff,
    ) else {
        return;
    };
    let options = flashtex_pdf::RenderOptions::with_document_face(font.clone());
    let out = flashtex_pdf::render_envelope_with(MATH_RESULT, &options).unwrap();
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    check_structure(&out.bytes).unwrap();
    let content = stream_data(&out.bytes, 7).unwrap();

    // The fraction bar is still a rectangle, never a glyph.
    let bars = rules(&content).unwrap();
    assert_eq!(bars.len(), 1);
    assert!((bars[0].width - 8.4).abs() < 0.0005);

    // Latin letters and '+' come from Latin Modern; α falls back to Symbol
    // because LM Roman has no Greek; √ comes from whichever has it (LM Roman
    // does carry a radical glyph); nothing uses Times.
    let placed = placements(&content).unwrap();
    let summary: Vec<(&str, Vec<u8>)> = placed
        .iter()
        .map(|p| (p.font.as_str(), p.bytes.clone()))
        .collect();
    let gid = |c: char| font.font.glyph_id(c).unwrap().to_be_bytes().to_vec();
    let radical = match font.font.glyph_id('√') {
        Some(g) => ("F3", g.to_be_bytes().to_vec()),
        None => ("F2", vec![0xD6]),
    };
    assert_eq!(
        font.font.glyph_id('α'),
        None,
        "LM Roman has no Greek; Symbol covers it"
    );
    assert_eq!(
        summary,
        vec![
            ("F3", gid('a')),
            ("F3", gid('b')),
            ("F3", gid('+')),
            ("F2", vec![0x61]),
            ("F3", gid('+')),
            radical,
            ("F3", gid('x')),
        ]
    );
    assert!(find(&content, b"/F1 ").is_none());
}

#[cfg(target_os = "macos")]
#[test]
fn latin_modern_face_widths_match_lm_metrics_in_pdfkit() {
    let Some(font) = font_with_outlines_or_skip(
        "latin_modern_face_widths_match_lm_metrics_in_pdfkit",
        flashtex_pdf::truetype::Outlines::Cff,
    ) else {
        return;
    };
    let exe = env!("CARGO_BIN_EXE_flashtex-pdf");
    let dir = std::env::temp_dir().join(format!("flashtex-pdf-lmface-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let text = "Latin Modern naïve — café";
    let probe = "Latin Modern";
    let input = dir.join("lmface.json");
    std::fs::write(
        &input,
        format!(
            r#"{{"protocol_version":1,"id":"f","type":"compile_result","payload":{{"pages":[{{"number":1,"width_pt":612,"height_pt":792,"items":[{{"kind":"text","text":"{text}","x_pt":72,"baseline_y_pt":84,"font_size_pt":12}}]}}]}}}}"#
        ),
    )
    .unwrap();
    let out = dir.join("lmface.pdf");
    // No --default-face: Latin Modern must imply the embedded face.
    let output = std::process::Command::new(exe)
        .arg(&input)
        .arg("--out")
        .arg(&out)
        .arg("--verify")
        .arg("--embed-font")
        .arg(&font.source)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stderr.contains("as the document face"), "{stderr}");
    assert!(!stderr.contains("warning:"), "{stderr}");
    let pdf = std::fs::read(&out).unwrap();
    assert!(find(&pdf, b"/F1 12 Tf").is_none());
    assert!(find(&pdf, b"/F3 12 Tf").is_some());

    let sips = std::process::Command::new("/usr/bin/sips")
        .env("CG_PDF_VERBOSE", "1")
        .args(["-s", "format", "png"])
        .arg(&out)
        .arg("--out")
        .arg(dir.join("lmface.png"))
        .output()
        .unwrap();
    let sips_err = String::from_utf8_lossy(&sips.stderr);
    assert!(
        sips.status.success() && !sips_err.contains("unsupported"),
        "{sips_err}"
    );

    // PDFKit: text round-trips, and the selection bounds of "Latin Modern"
    // are as wide as Latin Modern's advances say, not Times-Roman's.
    let script = "import sys\nfrom Quartz import PDFDocument\nfrom Foundation import NSURL\n\
                  d = PDFDocument.alloc().initWithURL_(NSURL.fileURLWithPath_(sys.argv[1]))\n\
                  pg = d.pageAtIndex_(0)\ns = pg.string()\nprint(s)\n\
                  i = s.index(sys.argv[2])\nsel = pg.selectionForRange_((i, len(sys.argv[2])))\n\
                  print(sel.boundsForPage_(pg).size.width)\n";
    let py = std::process::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&out)
        .arg(probe)
        .output()
        .unwrap();
    let py_err = String::from_utf8_lossy(&py.stderr);
    if !py.status.success() && py_err.contains("No module named") {
        eprintln!(
            "SKIPPED PDFKit checks: PyObjC Quartz not available ({})",
            py_err.trim()
        );
        return;
    }
    assert!(py.status.success(), "{py_err}");
    let stdout = String::from_utf8_lossy(&py.stdout);
    let mut lines = stdout.lines();
    assert_eq!(
        lines.next().unwrap_or(""),
        text,
        "PDFKit page.string round-trip"
    );
    let measured: f64 = lines.next().unwrap_or("").trim().parse().unwrap();
    let expected_lm = advance_width(&font, probe, 12.0);
    let expected_times = times_width(probe, 12.0);
    eprintln!(
        "\"{probe}\" at 12pt: PDFKit selection width {measured:.3}pt, Latin Modern advances {expected_lm:.3}pt, Times-Roman advances {expected_times:.3}pt"
    );
    assert!(
        (measured - expected_lm).abs() <= 0.5,
        "PDFKit width {measured} should match Latin Modern {expected_lm} within 0.5pt"
    );
    assert!(
        (measured - expected_times).abs() > 2.0,
        "width {measured} must not look like Times {expected_times}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Negotiated layout capabilities (docs/contracts/runtime-v1-layout-capabilities.md).

/// A negotiated result: the contract's rule example plus hinted text.
fn negotiated_envelope(capabilities: &str, items: &str) -> String {
    format!(
        r#"{{"protocol_version":1,"id":"caps","type":"compile_result","payload":{{"project_id":"caps","revision":1,"status":"ok","layout_capabilities":[{capabilities}],"pages":[{{"number":1,"width_pt":612,"height_pt":792,"items":[{items}]}}],"diagnostics":[],"pdf_path":null}}}}"#
    )
}

const RULE_ITEM: &str = r#"{"kind":"rule","x_pt":72,"y_pt":84,"width_pt":24,"height_pt":0.5,"source":{"path":"main.tex","start_byte":0,"end_byte":11}}"#;

#[test]
fn accepted_rules_v1_draws_the_rectangle_in_pdf_space() {
    let json = negotiated_envelope(
        r#""rules-v1""#,
        &format!(
            r#"{RULE_ITEM},{{"kind":"text","text":"after","x_pt":72,"baseline_y_pt":100,"font_size_pt":12,"source":{{"path":"main.tex","start_byte":12,"end_byte":17}}}}"#
        ),
    );
    let out = render_envelope(&json).unwrap();
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    check_structure(&out.bytes).unwrap();
    let content = stream_data(&out.bytes, 7).unwrap();
    let bars = rules(&content).unwrap();
    // Top-left (72, 84) with height 0.5 in y-down page space is bottom-left
    // (72, 792 - 84 - 0.5) in PDF space.
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].x, 72.0);
    assert!((bars[0].y - 707.5).abs() < 0.0005, "{:?}", bars[0]);
    assert_eq!(bars[0].width, 24.0);
    assert_eq!(bars[0].height, 0.5);
    // Paint order follows item order: the rule precedes the text.
    assert!(find(&content, b" re f\n").unwrap() < find(&content, b"BT\n").unwrap());
    let placed = placements(&content).unwrap();
    assert_eq!(placed.len(), 1);
    assert_eq!(placed[0].bytes, b"after");

    // On the negotiated route U+2500 is ordinary text, not a legacy rule.
    let dashes = negotiated_envelope(
        r#""rules-v1""#,
        r#"{"kind":"text","text":"──","x_pt":72,"baseline_y_pt":84,"font_size_pt":8.4}"#,
    );
    let out = render_envelope(&dashes).unwrap();
    let content = stream_data(&out.bytes, 7).unwrap();
    assert!(rules(&content).unwrap().is_empty());
    assert_eq!(out.warnings.len(), 1);
    assert!(out.warnings[0].contains("U+2500"));
}

#[test]
fn unrequested_rules_and_unknown_kinds_are_errors_with_source() {
    // Capabilities negotiated but without rules-v1.
    let json = negotiated_envelope(r#""font-hints-v1""#, RULE_ITEM);
    let err = render_envelope(&json).unwrap_err();
    assert!(
        matches!(&err, PdfError::Protocol(m) if m.contains("rules-v1") && m.contains("main.tex:0-11")),
        "{err}"
    );
    // Empty capability set: still negotiated, still no rules.
    let json = negotiated_envelope("", RULE_ITEM);
    assert!(matches!(render_envelope(&json), Err(PdfError::Protocol(_))));
    // Unknown kind under negotiation names the kind and the source range.
    let json = negotiated_envelope(
        r#""rules-v1""#,
        r#"{"kind":"image","x_pt":1,"y_pt":2,"source":{"path":"fig.tex","start_byte":3,"end_byte":9}}"#,
    );
    let err = render_envelope(&json).unwrap_err();
    assert!(
        matches!(&err, PdfError::Protocol(m) if m.contains("\"image\"") && m.contains("fig.tex:3-9")),
        "{err}"
    );
    // Rule geometry limits.
    for bad in [
        RULE_ITEM.replace("\"height_pt\":0.5", "\"height_pt\":0"),
        RULE_ITEM.replace("\"width_pt\":24", "\"width_pt\":-24"),
        RULE_ITEM.replace("\"x_pt\":72", "\"x_pt\":1000001"),
    ] {
        let json = negotiated_envelope(r#""rules-v1""#, &bad);
        assert!(render_envelope(&json).is_err(), "{bad}");
    }
    // A rule built directly, rendered without the capability, is refused too.
    let direct = CompileResult {
        pages: vec![page(
            1,
            612.0,
            792.0,
            vec![Item::Rule(flashtex_pdf::RuleItem {
                x_pt: 1.0,
                y_pt: 1.0,
                width_pt: 1.0,
                height_pt: 1.0,
            })],
        )],
        capabilities: Some(vec![]),
    };
    assert!(matches!(render_pdf(&direct), Err(PdfError::Invalid(m)) if m.contains("rules-v1")));
}

#[test]
fn symbol_font_hints_use_base14_symbol_with_its_builtin_encoding() {
    // The compiler's font-hints-v1 names "Symbol" for math operators. Every
    // character must be drawn from /F2 in Symbol's own encoding, including
    // those WinAnsi also carries (× is 0xD7 in WinAnsi but 0xB4 in Symbol);
    // a character Symbol lacks falls back to Times. No substitution warning.
    let items = r#"{"kind":"text","text":"∈×∀a","x_pt":72,"baseline_y_pt":84,"font_size_pt":12,"font":{"family":"Symbol","weight":"normal","style":"normal"}}"#;
    let json = negotiated_envelope(r#""font-hints-v1""#, items);
    let out = render_envelope(&json).unwrap();
    check_structure(&out.bytes).unwrap();
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    assert!(find(&out.bytes, b"/F4").is_none());
    let placed = placements(&stream_data(&out.bytes, 7).unwrap()).unwrap();
    let fonts: Vec<(&str, &[u8])> = placed
        .iter()
        .map(|p| (p.font.as_str(), p.bytes.as_slice()))
        .collect();
    assert_eq!(
        fonts,
        vec![("F2", b"\xCE\xB4\x22".as_slice()), ("F1", b"a")]
    );
}

#[test]
fn font_hints_select_times_variants_and_report_substitutions() {
    let items = r#"{"kind":"text","text":"plain","x_pt":72,"baseline_y_pt":84,"font_size_pt":12},
        {"kind":"text","text":"bold","x_pt":120,"baseline_y_pt":84,"font_size_pt":12,"font":{"family":"Times New Roman","weight":"bold","style":"normal"}},
        {"kind":"text","text":"italic","x_pt":160,"baseline_y_pt":84,"font_size_pt":12,"font":{"family":"Times","weight":"normal","style":"italic"}},
        {"kind":"text","text":"both","x_pt":200,"baseline_y_pt":84,"font_size_pt":12,"font":{"family":"Times","weight":"bold","style":"italic"}},
        {"kind":"text","text":"bold2","x_pt":240,"baseline_y_pt":84,"font_size_pt":12,"font":{"family":"Times","weight":"bold","style":"normal"}},
        {"kind":"text","text":"helv","x_pt":280,"baseline_y_pt":84,"font_size_pt":12,"font":{"family":"Palatino","weight":"bold","style":"normal"}},
        {"kind":"text","text":"roman","x_pt":320,"baseline_y_pt":84,"font_size_pt":12,"font":{"family":"Times","weight":"normal","style":"normal"}}"#;
    let json = negotiated_envelope(r#""font-hints-v1""#, items);
    let out = render_envelope(&json).unwrap();
    check_structure(&out.bytes).unwrap();
    // Three extra base-14 objects after the one page: Bold, Italic, BoldItalic.
    assert!(
        find(
            &out.bytes,
            b"/Font << /F1 3 0 R /F2 4 0 R /F4 8 0 R /F5 9 0 R /F6 10 0 R >>"
        )
        .is_some()
    );
    assert!(find(&out.bytes, b"\n8 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Times-Bold /Encoding /WinAnsiEncoding >>").is_some());
    assert!(
        find(
            &out.bytes,
            b"\n9 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Times-Italic /Encoding"
        )
        .is_some()
    );
    assert!(
        find(
            &out.bytes,
            b"\n10 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Times-BoldItalic /Encoding"
        )
        .is_some()
    );
    let placed = placements(&stream_data(&out.bytes, 7).unwrap()).unwrap();
    let fonts: Vec<(&str, &[u8])> = placed
        .iter()
        .map(|p| (p.font.as_str(), p.bytes.as_slice()))
        .collect();
    assert_eq!(
        fonts,
        vec![
            ("F1", b"plain".as_slice()),
            ("F4", b"bold"),
            ("F5", b"italic"),
            ("F6", b"both"),
            ("F4", b"bold2"),
            ("F4", b"helv"),
            ("F1", b"roman"),
        ]
    );
    // Palatino is not available: substituted, said so, once.
    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
    assert!(
        out.warnings[0].contains("\"Palatino\""),
        "{}",
        out.warnings[0]
    );
    assert!(
        out.warnings[0].contains("substituted by 'Times-Bold'"),
        "{}",
        out.warnings[0]
    );

    // A hint without font-hints-v1 in the negotiated set is an error; on
    // the legacy route it is ignored with a warning.
    let no_cap = negotiated_envelope(r#""rules-v1""#, items);
    assert!(
        matches!(render_envelope(&no_cap), Err(PdfError::Protocol(m)) if m.contains("font-hints-v1"))
    );
    let legacy = no_cap.replace(r#""layout_capabilities":["rules-v1"],"#, "");
    let out = render_envelope(&legacy).unwrap();
    assert!(
        out.warnings.iter().all(|w| w.contains("legacy route")),
        "{:?}",
        out.warnings
    );
    assert!(find(&out.bytes, b"/F4").is_none());
}

#[test]
fn font_hints_select_base14_courier_and_helvetica_variants() {
    let items = r#"{"kind":"text","text":"mono","x_pt":72,"baseline_y_pt":84,"font_size_pt":12,"font":{"family":"Courier","weight":"normal","style":"normal"}},
        {"kind":"text","text":"sans","x_pt":120,"baseline_y_pt":84,"font_size_pt":12,"font":{"family":"Helvetica","weight":"bold","style":"italic"}}"#;
    let json = negotiated_envelope(r#""font-hints-v1""#, items);
    let out = render_envelope(&json).unwrap();
    check_structure(&out.bytes).unwrap();
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    assert!(find(&out.bytes, b"\n8 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Encoding /WinAnsiEncoding >>").is_some());
    assert!(find(&out.bytes, b"\n9 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-BoldOblique /Encoding /WinAnsiEncoding >>").is_some());
    let placed = placements(&stream_data(&out.bytes, 7).unwrap()).unwrap();
    let fonts: Vec<(&str, &[u8])> = placed
        .iter()
        .map(|p| (p.font.as_str(), p.bytes.as_slice()))
        .collect();
    assert_eq!(fonts, vec![("F4", b"mono".as_slice()), ("F5", b"sans")]);
}

#[test]
fn font_hints_select_latin_modern_faces_as_their_own_embedded_fonts() {
    let Some(font) = font_with_outlines_or_skip(
        "font_hints_select_latin_modern_faces_as_their_own_embedded_fonts",
        flashtex_pdf::truetype::Outlines::Cff,
    ) else {
        return;
    };
    if !font.font.postscript_name.starts_with("LMRoman") {
        eprintln!(
            "SKIPPED: CFF font found is not Latin Modern ({})",
            font.font.postscript_name
        );
        return;
    }
    let items = r#"{"kind":"text","text":"regular","x_pt":72,"baseline_y_pt":84,"font_size_pt":12,"font":{"family":"Latin Modern Roman","weight":"normal","style":"normal"}},
        {"kind":"text","text":"bold","x_pt":120,"baseline_y_pt":84,"font_size_pt":12,"font":{"family":"Latin Modern Roman","weight":"bold","style":"normal"}},
        {"kind":"text","text":"italic","x_pt":160,"baseline_y_pt":84,"font_size_pt":12,"font":{"family":"Latin Modern Roman","weight":"normal","style":"italic"}},
        {"kind":"text","text":"unhinted","x_pt":200,"baseline_y_pt":84,"font_size_pt":12},
        {"kind":"text","text":"bold again","x_pt":260,"baseline_y_pt":84,"font_size_pt":12,"font":{"family":"Latin Modern Roman","weight":"bold","style":"normal"}},
        {"kind":"text","text":"mystery","x_pt":340,"baseline_y_pt":84,"font_size_pt":12,"font":{"family":"Palatino","weight":"normal","style":"italic"}}"#;
    let json = negotiated_envelope(r#""font-hints-v1""#, items);
    let options = flashtex_pdf::RenderOptions::with_document_face(font.clone());
    let out = flashtex_pdf::render_envelope_with(&json, &options).unwrap();
    let s = check_structure(&out.bytes).unwrap();
    // 7 base objects + document LM regular (5) + LM bold (5) + LM italic (5).
    assert_eq!(s.object_count, 22);
    assert!(
        find(
            &out.bytes,
            b"/Font << /F1 3 0 R /F2 4 0 R /F3 8 0 R /F4 13 0 R /F5 18 0 R >>"
        )
        .is_some()
    );
    assert!(find(&out.bytes, b"/BaseFont /LMRoman10-Regular").is_some());
    assert!(find(&out.bytes, b"/BaseFont /LMRoman10-Bold").is_some());
    assert!(find(&out.bytes, b"/BaseFont /LMRoman10-Italic").is_some());
    assert_eq!(
        count(&out.bytes, b"/Subtype /CIDFontType0C"),
        3,
        "three whole CFF programs"
    );
    assert_eq!(s.stream_objects, vec![7, 11, 12, 16, 17, 21, 22]);

    let placed = placements(&stream_data(&out.bytes, 7).unwrap()).unwrap();
    let fonts: Vec<&str> = placed.iter().map(|p| p.font.as_str()).collect();
    assert_eq!(fonts, vec!["F3", "F4", "F5", "F3", "F4", "F5"]);
    // The italic face's ToUnicode covers both "italic" and the substituted
    // "mystery"; the bold face's covers "bold" and "bold again".
    let bold_map =
        flashtex_pdf::embed::parse_to_unicode(&stream_data(&out.bytes, 17).unwrap()).unwrap();
    let italic_map =
        flashtex_pdf::embed::parse_to_unicode(&stream_data(&out.bytes, 22).unwrap()).unwrap();
    let decode = |p: &flashtex_pdf::verify::Placement,
                  map: &std::collections::BTreeMap<u16, char>|
     -> String {
        p.bytes
            .chunks(2)
            .map(|g| map[&u16::from_be_bytes([g[0], g[1]])])
            .collect()
    };
    assert_eq!(decode(&placed[1], &bold_map), "bold");
    assert_eq!(decode(&placed[4], &bold_map), "bold again");
    assert_eq!(decode(&placed[2], &italic_map), "italic");
    assert_eq!(decode(&placed[5], &italic_map), "mystery");
    // Bold and italic programs are different fonts, not the regular one again.
    let regular = stream_data(&out.bytes, 11).unwrap();
    let bold = stream_data(&out.bytes, 16).unwrap();
    let italic = stream_data(&out.bytes, 21).unwrap();
    assert!(regular != bold && bold != italic && regular != italic);

    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
    assert!(
        out.warnings[0].contains("\"Palatino\"")
            && out.warnings[0].contains("Latin Modern Roman italic"),
        "{}",
        out.warnings[0]
    );
    assert!(out.warnings[0].contains("not preserved"));
}

#[test]
fn latin_modern_hint_without_installation_is_substituted_by_times() {
    // No embedded font and (as far as this test can tell) the hint asks for
    // LM: if no LM installation exists, Times-Bold with a warning; if one
    // exists it is embedded even without --embed-font, because the hint
    // named it explicitly.
    let items = r#"{"kind":"text","text":"x","x_pt":72,"baseline_y_pt":84,"font_size_pt":12,"font":{"family":"Latin Modern Roman","weight":"bold","style":"normal"}}"#;
    let json = negotiated_envelope(r#""font-hints-v1""#, items);
    let out = render_envelope(&json).unwrap();
    check_structure(&out.bytes).unwrap();
    let lm_installed = flashtex_pdf::embed::candidate_paths()
        .iter()
        .any(|p| p.ends_with(flashtex_pdf::embed::LATIN_MODERN_FILE) && p.is_file());
    if lm_installed {
        assert!(out.warnings.is_empty(), "{:?}", out.warnings);
        assert!(find(&out.bytes, b"/BaseFont /LMRoman10-Bold").is_some());
    } else {
        assert_eq!(out.warnings.len(), 1);
        assert!(out.warnings[0].contains("substituted by 'Times-Bold'"));
        assert!(find(&out.bytes, b"/BaseFont /Times-Bold").is_some());
    }
}

#[test]
fn latin_modern_math_hint_embeds_the_math_font_instead_of_writing_question_marks() {
    // The compiler hints blackboard bold, \setminus and \Longrightarrow as
    // "Latin Modern Math". Its roman siblings lack those glyphs, so the hint
    // must resolve to latinmodern-math.otf itself.
    let lm_dir = flashtex_pdf::embed::candidate_paths()
        .into_iter()
        .find(|p| p.ends_with(flashtex_pdf::embed::LATIN_MODERN_FILE) && p.is_file())
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let has_math = lm_dir.as_ref().is_some_and(|d| {
        d.join("latinmodern-math.otf").is_file()
            || d.join("../lm-math/latinmodern-math.otf").is_file()
    });
    if !has_math {
        eprintln!("skipped: Latin Modern Math not installed beside Latin Modern");
        return;
    }
    let items = r#"{"kind":"text","text":"ℝ∖ℚ⟹ℤ","x_pt":72,"baseline_y_pt":84,"font_size_pt":12,"font":{"family":"Latin Modern Math","weight":"normal","style":"normal"}}"#;
    let json = negotiated_envelope(r#""font-hints-v1""#, items);
    let out = render_envelope(&json).unwrap();
    check_structure(&out.bytes).unwrap();
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    assert!(find(&out.bytes, b"LatinModernMath-Regular").is_some());
    let placed = placements(&stream_data(&out.bytes, 7).unwrap()).unwrap();
    assert!(
        placed
            .iter()
            .all(|p| p.font == "F4" && !p.bytes.contains(&b'?')),
        "every glyph drawn from the embedded math font: {placed:?}"
    );
}
