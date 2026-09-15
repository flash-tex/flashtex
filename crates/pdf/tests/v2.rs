//! rendering-v2 display list → exact export (`flashtex_pdf::v2`).
//!
//! `tests/fixtures/v2-plain-paragraph.json` is the unmodified `display_list`
//! envelope `flashtex-render --v2 --secnumdepth 0` (branch
//! `agent/mac-render-pipeline/unified` at `d556519`) wrote for the visual-corpus
//! fixture `01-plain-paragraph` (body only). It references Latin Modern
//! `lmroman12-regular.otf` by content hash; tests that need the bytes skip
//! loudly when the font is not installed.

use flashtex_pdf::exact::{self, Op, SubsetOutcome};
use flashtex_pdf::reader::PdfFile;
use flashtex_pdf::sha256;
use flashtex_pdf::truetype::TrueTypeFont;
use flashtex_pdf::v2::{self, HashForm, V2Options};
use flashtex_pdf::verify;
use std::path::{Path, PathBuf};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/v2-plain-paragraph.json"
);

fn lm12() -> Option<PathBuf> {
    v2::font_dirs(&V2Options::default())
        .into_iter()
        .map(|d| d.join("lmroman12-regular.otf"))
        .find(|p| p.is_file())
}

fn ops_text(pdf: &[u8], object: usize) -> String {
    String::from_utf8_lossy(&verify::stream_data(pdf, object).unwrap()).into_owned()
}

#[test]
fn real_pipeline_envelope_exports_glyphs_by_original_gid_at_exact_positions() {
    let Some(font_path) = lm12() else {
        eprintln!("skipped: Latin Modern 12 not installed");
        return;
    };
    let (doc, report) = v2::from_v2_file(Path::new(FIXTURE), &V2Options::default()).unwrap();
    assert_eq!(report.pages, 1);
    assert_eq!(report.runs, 13);
    assert_eq!(report.glyphs, 55);
    assert_eq!(report.rules, 0);
    assert_eq!(report.fonts.len(), 1);
    let f = &report.fonts[0];
    assert_eq!(f.resource, "F1");
    assert_eq!(f.postscript_name, "LMRoman12-Regular");
    assert_eq!(f.path, font_path);
    assert_eq!(f.outcome, SubsetOutcome::CffSubset);
    assert_eq!(f.glyphs, 18);
    assert_eq!(
        f.hash_form,
        HashForm::Bytes,
        "flashtex-render d556519 emits SHA-256 of the raw font bytes"
    );
    // The producer's advances are TFM widths; where they differ from hmtx
    // the /W entry follows the producer (word boundaries in text extraction).
    assert!(report.display_widths > 0, "{report:?}");
    assert_eq!(report.notes.len(), 1, "{:?}", report.notes);
    assert!(
        report.notes[0].contains("/W width(s) taken from the display list's advances"),
        "{:?}",
        report.notes
    );
    assert!(report.diagnostics.is_empty());

    let out = exact::render_exact(&doc).unwrap();
    verify::check_structure(&out.bytes).unwrap();
    assert_eq!(
        sha256::hex(&out.bytes),
        "bedc30983b6ccd486e861b64bf9562d6d5066bb72692cf88603934f1ceefc1db",
        "the ungrouped v2 fixture must stay byte-identical"
    );
    let content = ops_text(&out.bytes, 5);
    // 12 TeX pt = 12535902 ticks: the exact decimal, not pdfTeX's 11.9552.
    assert!(
        content.contains("/F1 11.9551677703857421875 Tf\n"),
        "{content}"
    );
    // First glyph 'H' (LM GID 62) at origin 75497472 ticks = 72 bp exactly,
    // baseline 88033374 ticks below the top of a 830472192-tick page.
    assert!(
        content.contains("1 0 0 1 72 708.0448322296142578125 Tm\n(\\000>) Tj\n"),
        "{content}"
    );
    // Every glyph carries its own Tm: 12 TeX pt is 12535902 ticks, which has
    // a factor of 3, so no gap between consecutive origins is an exactly
    // representable TJ adjustment and nothing joins or kerns.
    assert_eq!((report.joined_glyphs, report.kerned_glyphs), (0, 0));
    assert_eq!(content.matches(" Tm\n").count(), 55);
    let ops = exact::parse(content.as_bytes()).unwrap();
    assert_positions_round_trip(&ops, &doc, FIXTURE);
    assert_eq!(
        ops.iter().filter(|o| matches!(o, Op::BeginText)).count(),
        13
    );
    // The embedded program is a CID-keyed subset of the real file, GIDs kept.
    let file = PdfFile::parse(&out.bytes).unwrap();
    let page = file.pages().unwrap()[0];
    let fonts = file.page_fonts(page);
    assert_eq!(fonts.len(), 1);
    let re = flashtex_pdf::compare::font_from_dict(&file, fonts["F1"]).unwrap();
    let exact::ExactFont::CidCff(cid) = re else {
        panic!("expected CIDFontType0C")
    };
    let sub = flashtex_pdf::cff::CffFont::parse(cid.program.bytes()).unwrap();
    let src = TrueTypeFont::load(&font_path).unwrap();
    let src_cff = flashtex_pdf::cff::CffFont::parse(src.cff_table().unwrap()).unwrap();
    assert!(sub.is_cid_keyed());
    // Subset glyph i carries CID = original GID and the original charstring.
    let mut used: Vec<u16> = cid.widths.keys().copied().collect();
    used.sort_unstable();
    assert_eq!(used.len(), 18);
    for (i, &gid) in std::iter::once(&0u16).chain(used.iter()).enumerate() {
        assert_eq!(sub.charset_entry(i as u16), Some(gid));
        assert_eq!(
            sub.expanded_charstring(i as u16).unwrap(),
            src_cff.expanded_charstring(gid).unwrap()
        );
    }
    let tu = exact::parse_to_unicode(cid.to_unicode_verbatim.as_deref().unwrap()).unwrap();
    assert_eq!(tu.get(&62).map(String::as_str), Some("H"));
    assert!(
        tu.values().any(|t| t == "fi"),
        "the fi ligature's cluster text reaches ToUnicode: {tu:?}"
    );
    // Deterministic.
    let (doc2, _) = v2::from_v2_file(Path::new(FIXTURE), &V2Options::default()).unwrap();
    assert_eq!(exact::render_exact(&doc2).unwrap().bytes, out.bytes);
}

/// A hand-built envelope over Latin Modern 12: the plain SHA-256(bytes) hash
/// form, a glyph that continues by hmtx advance, one that does not, a rule
/// and a coloured run.
#[test]
fn hand_built_envelope_joins_by_hmtx_advance_and_converts_rules_and_colour() {
    let Some(font_path) = lm12() else {
        eprintln!("skipped: Latin Modern 12 not installed");
        return;
    };
    let bytes = std::fs::read(&font_path).unwrap();
    let font = TrueTypeFont::load(&font_path).unwrap();
    let sha = sha256::hex(&bytes);
    let gid_h = font.glyph_id('H').unwrap();
    let gid_e = font.glyph_id('e').unwrap();
    // A size whose tick count is a multiple of 1000 so that hmtx advances
    // (1000/em) scale to whole ticks: 12,500,000 ticks = 11.920928955078125 bp.
    let size: i64 = 12_500_000;
    let adv_h = font.advance(gid_h) as i64;
    assert_eq!(font.units_per_em, 1000);
    let adv_ticks = adv_h * size / 1000;
    assert_eq!(adv_ticks * 1000, adv_h * size, "exact");
    let adv_e = font.advance(gid_e) as i64;
    let adv_e_ticks = adv_e * size / 1000;
    assert_eq!(adv_e_ticks * 1000, adv_e * size, "exact");
    let x0: i64 = 72 << 20;
    let y: i64 = 100 << 20;
    let envelope = format!(
        r#"{{"protocol_version":2,"id":"t","type":"display_list","payload":{{
        "render_format":"display-list-v2","coordinate_unit":"bp_2pow20","color_space":"srgb","text_extraction":"cluster-actualtext",
        "project_id":"t","revision":1,"required_features":["glyph_run","rule"],"documents":[],
        "fonts":[{{"font_id":"{sha}","sha256":"{sha}","byte_length":{len},"format":"opentype-cff","face_index":0,"units_per_em":1000,"glyph_count":{gc},"postscript_name":"LMRoman12-Regular"}}],
        "pages":[{{"number":1,"width":{pw},"height":{ph},"items":[
          {{"kind":"glyph_run","font_id":"{sha}","font_size":{size},"text":"HeH","paint":{{"r":0,"g":0,"b":0,"a":1}},
           "glyphs":[
             {{"gid":{gh},"origin_x":{x0},"baseline_y":{y},"advance_x":{adv},"advance_y":0,"cluster":0}},
             {{"gid":{ge},"origin_x":{x1},"baseline_y":{y},"advance_x":{adv_e},"advance_y":0,"cluster":1}},
             {{"gid":{gh},"origin_x":{x2},"baseline_y":{y},"advance_x":{adv},"advance_y":0,"cluster":2}}],
           "clusters":[{{"text_start_byte":0,"text_end_byte":1}},{{"text_start_byte":1,"text_end_byte":2}},{{"text_start_byte":2,"text_end_byte":3}}]}},
          {{"kind":"rule","x":{x0},"top":{rt},"width":{rw},"height":{rh},"paint":{{"r":0,"g":0,"b":0,"a":1}}}},
          {{"kind":"glyph_run","font_id":"{sha}","font_size":{size},"text":"e","paint":{{"r":0.5,"g":0,"b":1,"a":1}},
           "glyphs":[{{"gid":{ge},"origin_x":{x0},"baseline_y":{y2},"advance_x":{adv_e},"advance_y":0,"cluster":0}}],
           "clusters":[{{"text_start_byte":0,"text_end_byte":1}}]}}
        ]}}],"diagnostics":[]}}}}"#,
        len = bytes.len(),
        gc = font.num_glyphs(),
        pw = 612i64 << 20,
        ph = 792i64 << 20,
        gh = gid_h,
        ge = gid_e,
        adv = adv_ticks,
        adv_e = adv_e_ticks,
        x1 = x0 + adv_ticks,
        x2 = x0 + adv_ticks + 1000 + 5, // not hmtx-continued
        rt = 110i64 << 20,
        rw = 3 << 19, // 1.5 bp
        rh = 1 << 18, // 0.25 bp
        y2 = 200i64 << 20,
    );
    let dir = std::env::temp_dir().join(format!("flashtex-pdf-v2-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(&font_path, dir.join("lm.otf")).unwrap();
    let options = V2Options {
        font_dirs: vec![dir.clone()],
    };
    let (doc, report) = v2::from_v2(&envelope, &options).unwrap();
    assert_eq!(report.fonts[0].hash_form, HashForm::Bytes);
    assert_eq!(report.fonts[0].path, dir.join("lm.otf"));
    assert!(report.notes.is_empty(), "{:?}", report.notes);
    assert_eq!(
        report.display_widths, 0,
        "advances equal hmtx: /W untouched"
    );
    assert_eq!(
        (report.joined_glyphs, report.kerned_glyphs),
        (1, 1),
        "e continues at the natural advance; the second H needs an exact kern"
    );
    let out = exact::render_exact(&doc).unwrap();
    verify::check_structure(&out.bytes).unwrap();
    let content = ops_text(&out.bytes, 5);
    // The second H sits 1005 ticks past e's natural advance (e is 435/1000
    // wide): n = 435 - 1000 * 1005 / 12,500,000 = 434.9196, exactly.
    let expect = format!(
        "BT\n/F1 11.920928955078125 Tf\n1 0 0 1 72 692 Tm\n[(\\000{h}\\000{e})434.9196(\\000{h})] TJ\nET\n72 {ry} 1.5 0.25 re\nf\nq\n0.5 0 1 rg\nBT\n/F1 11.920928955078125 Tf\n1 0 0 1 72 592 Tm\n(\\000{e}) Tj\nET\nQ\n",
        h = gid_h as u8 as char,
        e = gid_e as u8 as char,
        ry = v2::bp(((792i64 << 20) - (110i64 << 20) - (1 << 18)) as i128).unwrap(),
    );
    assert_eq!(
        font.advance(gid_e),
        435,
        "the expectation above assumes e's advance"
    );
    // Codes below 32 or above 126 are octal-escaped by the writer; compare
    // through the parser instead of raw text where GIDs are small.
    let got = exact::parse(content.as_bytes()).unwrap();
    let want = exact::parse(expect.as_bytes()).unwrap();
    assert_eq!(got, want, "content:\n{content}\nexpected:\n{expect}");
    std::fs::write(dir.join("list.json"), &envelope).unwrap();
    assert_positions_round_trip(&got, &doc, dir.join("list.json").to_str().unwrap());
    // Deterministic bytes.
    assert_eq!(exact::render_exact(&doc).unwrap().bytes, out.bytes);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The producer lays out with TFM metrics, and for a handful of Latin Modern
/// glyphs (bold `W`: 1093/1000 em in the TFM, 1189 in hmtx) the next glyph
/// starts well before the hmtx pen. With hmtx `/W` widths a viewer's text
/// extraction reads that as a word break ("W ednesday ,"), so `/W` follows
/// the producer's most frequent kern-free advance; painting is unchanged
/// (every glyph keeps its envelope origin) and joins use the written width.
#[test]
fn display_list_advances_become_the_w_widths_where_hmtx_differs() {
    let Some(font_path) = lm12() else {
        eprintln!("skipped: Latin Modern 12 not installed");
        return;
    };
    let bytes = std::fs::read(&font_path).unwrap();
    let font = TrueTypeFont::load(&font_path).unwrap();
    let sha = sha256::hex(&bytes);
    let gid_w = font.glyph_id('W').unwrap();
    let gid_e = font.glyph_id('e').unwrap();
    let hmtx_w = font.advance(gid_w) as i64;
    let hmtx_e = font.advance(gid_e) as i64;
    let size: i64 = 12_500_000; // 2,5-smooth: 1000/em widths are whole ticks
    // The producer's W is 100/1000 em narrower than hmtx; e follows at exactly
    // that advance. A second W carries a kern to a comma (advance 20 less),
    // the minority value, so the mode is the kern-free width.
    let tfm_w = hmtx_w - 100;
    let adv_w = tfm_w * size / 1000;
    let adv_w_kerned = (tfm_w - 20) * size / 1000;
    let adv_e = hmtx_e * size / 1000;
    let x0: i64 = 72 << 20;
    let y: i64 = 100 << 20;
    let envelope = format!(
        r#"{{"protocol_version":2,"id":"t","type":"display_list","payload":{{
        "render_format":"display-list-v2","coordinate_unit":"bp_2pow20","color_space":"srgb","text_extraction":"cluster-actualtext",
        "project_id":"t","revision":1,"required_features":["glyph_run"],"documents":[],
        "fonts":[{{"font_id":"{sha}","sha256":"{sha}","byte_length":{len},"format":"opentype-cff","face_index":0,"units_per_em":1000,"glyph_count":{gc},"postscript_name":"LMRoman12-Regular"}}],
        "pages":[{{"number":1,"width":{pw},"height":{ph},"items":[
          {{"kind":"glyph_run","font_id":"{sha}","font_size":{size},"text":"WeW","paint":{{"r":0,"g":0,"b":0,"a":1}},
           "glyphs":[
             {{"gid":{gw},"origin_x":{x0},"baseline_y":{y},"advance_x":{adv_w},"advance_y":0,"cluster":0}},
             {{"gid":{ge},"origin_x":{x1},"baseline_y":{y},"advance_x":{adv_e},"advance_y":0,"cluster":1}},
             {{"gid":{gw},"origin_x":{x2},"baseline_y":{y},"advance_x":{adv_w},"advance_y":0,"cluster":2}}],
           "clusters":[{{"text_start_byte":0,"text_end_byte":1}},{{"text_start_byte":1,"text_end_byte":2}},{{"text_start_byte":2,"text_end_byte":3}}]}},
          {{"kind":"glyph_run","font_id":"{sha}","font_size":{size},"text":"W","paint":{{"r":0,"g":0,"b":0,"a":1}},
           "glyphs":[{{"gid":{gw},"origin_x":{x0},"baseline_y":{y2},"advance_x":{adv_w_kerned},"advance_y":0,"cluster":0}}],
           "clusters":[{{"text_start_byte":0,"text_end_byte":1}}]}}
        ]}}],"diagnostics":[]}}}}"#,
        len = bytes.len(),
        gc = font.num_glyphs(),
        pw = 612i64 << 20,
        ph = 792i64 << 20,
        gw = gid_w,
        ge = gid_e,
        x1 = x0 + adv_w,
        x2 = x0 + adv_w + adv_e,
        y2 = 200i64 << 20,
    );
    let dir = std::env::temp_dir().join(format!("flashtex-pdf-v2w-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(&font_path, dir.join("lm.otf")).unwrap();
    let options = V2Options {
        font_dirs: vec![dir.clone()],
    };
    let (doc, report) = v2::from_v2(&envelope, &options).unwrap();
    assert_eq!(report.display_widths, 1, "{report:?}");
    assert_eq!(report.notes.len(), 1, "{:?}", report.notes);
    assert!(
        report.notes[0].contains("1 /W width(s)"),
        "{:?}",
        report.notes
    );
    let exact::ExactFont::CidCff(cid) = &doc.fonts["F1"] else {
        panic!("expected CIDFontType0C")
    };
    assert_eq!(cid.widths[&gid_w], exact::Decimal::from_i64(tfm_w));
    assert_eq!(cid.widths[&gid_e], exact::Decimal::from_i64(hmtx_e));
    // Both continuations are exact at the written widths: one string.
    assert_eq!((report.joined_glyphs, report.kerned_glyphs), (2, 0));
    let out = exact::render_exact(&doc).unwrap();
    verify::check_structure(&out.bytes).unwrap();
    let content = ops_text(&out.bytes, 5);
    let ops = exact::parse(content.as_bytes()).unwrap();
    assert_eq!(
        ops.iter()
            .filter(|o| matches!(o, Op::ShowText(t) if t.len() == 6))
            .count(),
        1,
        "WeW is one three-glyph string: {content}"
    );
    std::fs::write(dir.join("list.json"), &envelope).unwrap();
    assert_positions_round_trip(&ops, &doc, dir.join("list.json").to_str().unwrap());
    // The written /W is what the file carries.
    let file = PdfFile::parse(&out.bytes).unwrap();
    let page = file.pages().unwrap()[0];
    let fonts = file.page_fonts(page);
    let re = flashtex_pdf::compare::font_from_dict(&file, fonts["F1"]).unwrap();
    let exact::ExactFont::CidCff(read_back) = re else {
        panic!("expected CIDFontType0C")
    };
    assert_eq!(read_back.widths[&gid_w], exact::Decimal::from_i64(tfm_w));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Replays the written text operators exactly and checks every glyph lands
/// on the envelope's origin (`origin_x / 2^20`, `(height - baseline_y) / 2^20`).
fn assert_positions_round_trip(ops: &[Op], doc: &exact::ExactDocument, envelope_path: &str) {
    use exact::{ExactFont, Ratio};
    let width = |font: &str, code: u16| -> Option<Ratio> {
        match doc.fonts.get(font)? {
            ExactFont::CidCff(c) | ExactFont::CidTrueType(c) => Some(Ratio::from_decimal(
                c.widths.get(&code).unwrap_or(&c.default_width),
            )),
            ExactFont::Simple(_) => None,
        }
    };
    let positions = exact::glyph_positions(ops, &|_| true, &width).unwrap();
    // Expected origins straight from the envelope, in page order.
    let text = std::fs::read_to_string(envelope_path).unwrap();
    let json = flashtex_pdf::json::parse(&text).unwrap();
    let mut expected: Vec<(u16, Ratio, Ratio)> = Vec::new();
    let page = &json
        .get("payload")
        .unwrap()
        .get("pages")
        .unwrap()
        .as_array()
        .unwrap()[0];
    let height = page.get("height").unwrap().as_f64().unwrap() as i128;
    for item in page.get("items").unwrap().as_array().unwrap() {
        if item.get("kind").unwrap().as_str() != Some("glyph_run") {
            continue;
        }
        for g in item.get("glyphs").unwrap().as_array().unwrap() {
            let gid = g.get("gid").unwrap().as_f64().unwrap() as u16;
            let ox = g.get("origin_x").unwrap().as_f64().unwrap() as i128;
            let by = g.get("baseline_y").unwrap().as_f64().unwrap() as i128;
            expected.push((
                gid,
                Ratio::new(ox, 1 << 20),
                Ratio::new(height - by, 1 << 20),
            ));
        }
    }
    assert_eq!(positions.len(), expected.len());
    for (p, (gid, x, y)) in positions.iter().zip(&expected) {
        assert_eq!(p.code, *gid);
        assert_eq!(
            (p.x, p.y),
            (*x, *y),
            "glyph {gid} replays to its envelope origin"
        );
    }
}

#[test]
fn unsupported_envelope_content_is_refused_not_approximated() {
    let base = r#"{"protocol_version":2,"id":"t","type":"display_list","payload":{"render_format":"display-list-v2","coordinate_unit":"bp_2pow20","color_space":"srgb","fonts":[],"pages":[{"number":1,"width":1048576,"height":1048576,"items":[ITEM]}],"diagnostics":[]}}"#;
    let cases = [
        (
            // Image items are supported (tests/images.rs); a malformed one is
            // a named error.
            r#"{"kind":"image","paint":{"r":0,"g":0,"b":0,"a":1}}"#,
            "items[0].width: expected a number",
        ),
        (
            r#"{"kind":"rule","x":0,"top":0,"width":5,"height":5,"paint":{"r":0,"g":0,"b":0,"a":0.5}}"#,
            "alpha 0.5",
        ),
        (
            r#"{"kind":"rule","x":0,"top":0,"width":0,"height":5,"paint":{"r":0,"g":0,"b":0,"a":1}}"#,
            "not positive",
        ),
        (
            r#"{"kind":"rule","x":0.5,"top":0,"width":5,"height":5,"paint":{"r":0,"g":0,"b":0,"a":1}}"#,
            "not an integer tick",
        ),
        (
            r#"{"kind":"glyph_run","font_id":"nope","font_size":100,"text":"","paint":{"r":0,"g":0,"b":0,"a":1},"glyphs":[],"clusters":[]}"#,
            "not in payload.fonts",
        ),
    ];
    for (item, expect) in cases {
        let e = v2::from_v2(&base.replace("ITEM", item), &V2Options::default()).unwrap_err();
        assert!(e.contains(expect), "{item}: {e}");
    }
    let e = v2::from_v2(&base.replace("ITEM", r#"{"kind":"rule","x":0,"top":0,"width":5,"height":5,"paint":{"r":0.1,"g":0,"b":0,"a":1}}"#), &V2Options::default())
        .unwrap_err();
    assert!(e.contains("0.1"), "non-terminating colour is refused: {e}");
    // A missing font is a named error, not a fallback.
    let with_font = r#"{"protocol_version":2,"id":"t","type":"display_list","payload":{"render_format":"display-list-v2","coordinate_unit":"bp_2pow20","color_space":"srgb","fonts":[{"font_id":"0000000000000000000000000000000000000000000000000000000000000000","sha256":"0000000000000000000000000000000000000000000000000000000000000000","byte_length":7,"format":"opentype-cff","face_index":0,"units_per_em":1000,"glyph_count":10,"postscript_name":"Nope"}],"pages":[{"number":1,"width":1048576,"height":1048576,"items":[{"kind":"glyph_run","font_id":"0000000000000000000000000000000000000000000000000000000000000000","font_size":100,"text":"a","paint":{"r":0,"g":0,"b":0,"a":1},"glyphs":[{"gid":3,"origin_x":0,"baseline_y":0,"advance_x":0,"advance_y":0,"cluster":0}],"clusters":[{"text_start_byte":0,"text_end_byte":1}]}]}],"diagnostics":[]}}"#;
    let e = v2::from_v2(
        with_font,
        &V2Options {
            font_dirs: vec![std::env::temp_dir()],
        },
    )
    .unwrap_err();
    assert!(e.contains("Nope") && e.contains("was not found"), "{e}");
}

/// The older producer form, SHA-256(bytes || face_index), is still resolved
/// and reported as a deviation.
#[test]
fn content_hash_with_face_index_is_still_accepted_and_reported() {
    let Some(font_path) = lm12() else {
        eprintln!("skipped: Latin Modern 12 not installed");
        return;
    };
    let mut bytes = std::fs::read(&font_path).unwrap();
    let font = TrueTypeFont::load(&font_path).unwrap();
    let len = bytes.len();
    bytes.extend_from_slice(&0u32.to_be_bytes());
    let sha = sha256::hex(&bytes);
    let gid = font.glyph_id('A').unwrap();
    let envelope = format!(
        r#"{{"protocol_version":2,"id":"t","type":"display_list","payload":{{"render_format":"display-list-v2","coordinate_unit":"bp_2pow20","color_space":"srgb","fonts":[{{"font_id":"{sha}","sha256":"{sha}","byte_length":{len},"format":"opentype-cff","face_index":0,"units_per_em":1000,"glyph_count":{gc},"postscript_name":"LMRoman12-Regular"}}],"pages":[{{"number":1,"width":1048576,"height":1048576,"items":[{{"kind":"glyph_run","font_id":"{sha}","font_size":1048576,"text":"A","paint":{{"r":0,"g":0,"b":0,"a":1}},"glyphs":[{{"gid":{gid},"origin_x":0,"baseline_y":0,"advance_x":0,"advance_y":0,"cluster":0}}],"clusters":[{{"text_start_byte":0,"text_end_byte":1}}]}}]}}],"diagnostics":[]}}}}"#,
        gc = font.num_glyphs()
    );
    let (_, report) = v2::from_v2(&envelope, &V2Options::default()).unwrap();
    assert_eq!(report.fonts[0].hash_form, HashForm::BytesAndFaceIndex);
    assert!(
        report
            .notes
            .iter()
            .any(|n| n.contains("render-pipeline deviation")),
        "{:?}",
        report.notes
    );
}

/// The searchable-text contract (`docs/proposals/pdf-searchable-text.md`):
/// word boundaries are the producer's own inter-glyph gaps, replayed exactly
/// (pen after the written `/W` width → next origin), and the report counts
/// them. `v2-text-a-b.json` is the published `\text{a b}` display list from
/// GH48 (rendering-core handoff 647c50c5: two one-glyph runs with glue
/// between them and no space cluster); `v2-searchable-mixed.json` is the
/// bundled producer's list for `The AV office fixed a b: $\forall x\, f(x)$
/// and ffi.` (kerned `AV`, `ffi`/`fi` ligatures, math italic corrections).
/// Fonts come from `apps/mac/Fonts` or an installed Latin Modern; skipped
/// otherwise.
#[test]
fn searchable_text_word_gaps_are_the_producers_and_are_counted() {
    use exact::{ExactFont, Ratio};
    let options = V2Options {
        font_dirs: vec![PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../apps/mac/Fonts"
        ))],
    };
    let ab = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/v2-text-a-b.json"
    );
    let (doc, report) = match v2::from_v2_file(Path::new(ab), &options) {
        Ok(x) => x,
        Err(e) if e.contains("was not found") => {
            eprintln!("skipped: {e}");
            return;
        }
        Err(e) => panic!("{e}"),
    };
    assert_eq!((report.runs, report.glyphs), (2, 2));
    // One word gap (cmr12's word space, 0.326 em), nothing ambiguous.
    assert_eq!(
        (report.word_gaps, report.ambiguous_gaps),
        (1, 0),
        "{report:?}"
    );
    let out = exact::render_exact(&doc).unwrap();
    let content = ops_text(&out.bytes, 5);
    let ops = exact::parse(content.as_bytes()).unwrap();
    // Both glyphs replay to their envelope origins.
    assert_positions_round_trip(&ops, &doc, ab);
    // The pen after `a` (its written /W width) lands 0.326 em before `b`:
    // the boundary is geometry only — no space glyph, ActualText or Tw.
    let ExactFont::CidCff(cid) = &doc.fonts["F1"] else {
        panic!("expected CIDFontType0C")
    };
    let (a_origin, b_origin, size) = (134_651_073i128, 144_879_963i128, 12_535_902i128);
    let w_a = Ratio::from_decimal(&cid.widths[&28]);
    let gap = b_origin - (a_origin + w_a.num * size / (1000 * w_a.den));
    assert!(
        gap * 1000 >= v2::WORD_GAP_EM * size && gap * 1000 < 330 * size,
        "gap {gap} ticks is {}/1000 em",
        gap * 1000 / size
    );
    assert_eq!(
        cid.widths.len(),
        2,
        "only a and b are embedded: {:?}",
        cid.widths
    );
    assert!(!content.contains("Tw"), "{content}");
    assert!(!content.contains("ActualText"), "{content}");
    // ToUnicode as written (parsed back from the PDF bytes).
    let file = PdfFile::parse(&out.bytes).unwrap();
    let page = file.pages().unwrap()[0];
    let fonts = file.page_fonts(page);
    let ExactFont::CidCff(re) = flashtex_pdf::compare::font_from_dict(&file, fonts["F1"]).unwrap()
    else {
        panic!("expected CIDFontType0C")
    };
    let tu = exact::parse_to_unicode(re.to_unicode_verbatim.as_deref().unwrap()).unwrap();
    assert_eq!(tu.get(&28).map(String::as_str), Some("a"));
    assert_eq!(tu.get(&35).map(String::as_str), Some("b"));
    assert_eq!(tu.len(), 2);

    let mixed = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/v2-searchable-mixed.json"
    );
    let (doc, report) = match v2::from_v2_file(Path::new(mixed), &options) {
        Ok(x) => x,
        Err(e) if e.contains("was not found") => {
            eprintln!("skipped (mixed): {e}");
            return;
        }
        Err(e) => panic!("{e}"),
    };
    // Nine word gaps (The|AV|office|fixed|a|b:|∀…, `\,` at 0.161 em, )|and,
    // and|ffi.) and two ambiguous ones: the 0.099 em italic correction after
    // math `f` before `o` and before `(`. Kerns inside `AVAV` are below 30.
    assert_eq!(
        (report.word_gaps, report.ambiguous_gaps),
        (9, 2),
        "{report:?}"
    );
    let named: Vec<&String> = report
        .notes
        .iter()
        .filter(|n| n.contains("gap of 99/1000 em"))
        .collect();
    assert_eq!(named.len(), 2, "{:?}", report.notes);
    assert!(named[0].contains("between \"f\" and \"o\""), "{}", named[0]);
    assert!(named[1].contains("between \"f\" and \"(\""), "{}", named[1]);
    // Ligatures and every other cluster reach ToUnicode as their text.
    let out = exact::render_exact(&doc).unwrap();
    let file = PdfFile::parse(&out.bytes).unwrap();
    let page = file.pages().unwrap()[0];
    let fonts = file.page_fonts(page);
    let mut texts = Vec::new();
    for obj in fonts.values() {
        let ExactFont::CidCff(cid) = flashtex_pdf::compare::font_from_dict(&file, obj).unwrap()
        else {
            panic!("expected CIDFontType0C")
        };
        let tu = exact::parse_to_unicode(cid.to_unicode_verbatim.as_deref().unwrap()).unwrap();
        texts.extend(tu.into_values());
    }
    for t in ["ffi", "fi", "A", "V", "\\", "(", "x"] {
        assert!(
            texts.iter().any(|x| x == t),
            "{t:?} missing from ToUnicode: {texts:?}"
        );
    }
    let content = ops_text(&out.bytes, 5);
    let ops = exact::parse(content.as_bytes()).unwrap();
    assert_positions_round_trip(&ops, &doc, mixed);
}

#[test]
fn actual_text_groups_adjacent_cross_font_runs_without_changing_tounicode() {
    let font_dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../apps/mac/Fonts"));
    let roman_path = font_dir.join("lmroman10-regular.otf");
    let math_path = font_dir.join("latinmodern-math.otf");
    if !roman_path.is_file() || !math_path.is_file() {
        eprintln!("skipped: bundled Latin Modern fonts are not present");
        return;
    }
    let roman_bytes = std::fs::read(&roman_path).unwrap();
    let math_bytes = std::fs::read(&math_path).unwrap();
    let roman = TrueTypeFont::load(&roman_path).unwrap();
    let math = TrueTypeFont::load(&math_path).unwrap();
    let roman_id = sha256::hex(&roman_bytes);
    let math_id = sha256::hex(&math_bytes);
    let eq = roman.glyph_id('=').unwrap();
    let implies = math.glyph_id('⇒').unwrap();
    let arrow = math.glyph_id('→').unwrap();
    let size = 12i64 << 20;
    let baseline = 100i64 << 20;
    let x0 = 72i64 << 20;
    let x1 = x0 + i64::from(roman.advance(eq)) * size / i64::from(roman.units_per_em);
    let x2 = x1 + i64::from(math.advance(implies)) * size / i64::from(math.units_per_em);
    let x3 = x2 + i64::from(math.advance(arrow)) * size / i64::from(math.units_per_em);
    let run = |font_id: &str, text: &str, gid: u16, x: i64, actual: bool| {
        let actual = if actual {
            r#","actual_text":"⟹""#
        } else {
            ""
        };
        format!(
            r#"{{"kind":"glyph_run","font_id":"{font_id}","font_size":{size},"text":"{text}"{actual},"paint":{{"r":0,"g":0,"b":0,"a":1}},"glyphs":[{{"gid":{gid},"origin_x":{x},"baseline_y":{baseline},"advance_x":0,"advance_y":0,"cluster":0}}],"clusters":[{{"text_start_byte":0,"text_end_byte":{text_len}}}]}}"#,
            text_len = text.len(),
        )
    };
    let font = |id: &str, bytes: &[u8], f: &TrueTypeFont| {
        format!(
            r#"{{"font_id":"{id}","sha256":"{id}","byte_length":{},"format":"opentype-cff","face_index":0,"units_per_em":{},"glyph_count":{},"postscript_name":"{}"}}"#,
            bytes.len(),
            f.units_per_em,
            f.num_glyphs(),
            f.postscript_name
        )
    };
    let envelope = format!(
        r#"{{"protocol_version":2,"id":"actual","type":"display_list","payload":{{"render_format":"display-list-v2","coordinate_unit":"bp_2pow20","color_space":"srgb","text_extraction":"cluster-actualtext","project_id":"actual","revision":1,"required_features":["glyph_run","cluster-actualtext"],"documents":[],"fonts":[{roman_font},{math_font}],"pages":[{{"number":1,"width":{page_width},"height":{page_height},"items":[{items}]}}],"diagnostics":[]}}}}"#,
        roman_font = font(&roman_id, &roman_bytes, &roman),
        math_font = font(&math_id, &math_bytes, &math),
        items = [
            run(&roman_id, "=", eq, x0, true),
            run(&math_id, "⇒", implies, x1, true),
            run(&roman_id, "=", eq, x2, false),
            run(&math_id, "→", arrow, x3, false),
        ]
        .join(","),
        page_width = 612i64 << 20,
        page_height = 792i64 << 20,
    );
    let (doc, _) = v2::from_v2(
        &envelope,
        &V2Options {
            font_dirs: vec![font_dir],
        },
    )
    .unwrap();
    let out = exact::render_exact(&doc).unwrap();
    let exact::ExactFont::CidCff(roman_font) = &doc.fonts["F1"] else {
        panic!("expected a CFF font for the roman run")
    };
    let exact::ExactFont::CidCff(math_font) = &doc.fonts["F2"] else {
        panic!("expected a CFF font for the math run")
    };
    assert_eq!(
        roman_font.to_unicode.get(&eq).map(String::as_str),
        Some("=")
    );
    assert_eq!(
        math_font.to_unicode.get(&implies).map(String::as_str),
        Some("⇒")
    );
    assert_eq!(
        math_font.to_unicode.get(&arrow).map(String::as_str),
        Some("→")
    );
    let content = ops_text(&out.bytes, 5);
    assert_eq!(content.matches(" BDC\n").count(), 1, "{content}");
    assert_eq!(content.matches("EMC\n").count(), 1, "{content}");
    assert!(content.contains("<FEFF27F9>"), "{content}");
    assert_eq!(
        exact::parse(content.as_bytes()).unwrap(),
        match &doc.pages[0].content {
            exact::Content::Ops(ops) => ops.clone(),
            exact::Content::Verbatim(_) => unreachable!(),
        }
    );

    let dir = std::env::temp_dir().join(format!("flashtex-pdf-actualtext-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let pdf = dir.join("actual-text.pdf");
    std::fs::write(&pdf, &out.bytes).unwrap();
    let output = std::process::Command::new("python3")
        .args([
            "-c",
            "import contextlib, io, sys\nwith contextlib.redirect_stdout(io.StringIO()):\n import fitz\nprint(''.join(page.get_text() for page in fitz.open(sys.argv[1])), end='')",
            pdf.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    if !output.status.success()
        && String::from_utf8_lossy(&output.stderr).contains("No module named")
    {
        eprintln!("skipped: PyMuPDF is not installed");
        let _ = std::fs::remove_dir_all(&dir);
        return;
    }
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let extracted = String::from_utf8_lossy(&output.stdout).replace(['\r', '\n'], "");
    assert_eq!(extracted, "⟹=→");
    let _ = std::fs::remove_dir_all(&dir);
}
