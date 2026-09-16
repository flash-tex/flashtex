//! Image XObjects on the exact route (`display-list-v2-images`, proposal
//! `protocol/proposals/display-list-v2-image.md` §5.4).
//!
//! `tests/fixtures/images/oracle/expected.json` and the `*.v2.json`
//! envelopes are written by `tests/fixtures/images/make_oracle.py` from
//! pdfLaTeX (pdfTeX 1.40.29) placing each fixture image; cargo never runs
//! TeX. Each envelope is exported here and checked against pdfTeX's CTM at
//! `Do` (0.01 bp), its XObject dictionary summary and the SHA-256 of the
//! decoded samples (image) or content stream (form).

use flashtex_pdf::exact::{self, Content, Decimal, ExactDocument, ExactPage, Op};
use flashtex_pdf::json::{self, Value};
use flashtex_pdf::reader::{Obj, PdfFile};
use flashtex_pdf::sha256;
use flashtex_pdf::v2::{self, V2Options};
use flashtex_pdf::verify;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const IMAGES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/images");

fn root() -> Option<&'static Path> {
    Some(Path::new(IMAGES))
}

fn mul(m: [f64; 6], n: [f64; 6]) -> [f64; 6] {
    let [a, b, c, d, e, f] = m;
    let [aa, bb, cc, dd, ee, ff] = n;
    [
        a * aa + b * cc,
        a * bb + b * dd,
        c * aa + d * cc,
        c * bb + d * dd,
        e * aa + f * cc + ee,
        e * bb + f * dd + ff,
    ]
}

/// (name, CTM) at every `Do`.
fn placements(ops: &[Op]) -> Vec<(String, [f64; 6])> {
    let mut ctm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut stack = Vec::new();
    let mut out = Vec::new();
    for op in ops {
        match op {
            Op::Save => stack.push(ctm),
            Op::Restore => ctm = stack.pop().expect("balanced"),
            Op::Concat(m) => {
                let m: Vec<f64> = m.iter().map(Decimal::approx).collect();
                ctm = mul(m.try_into().unwrap(), ctm);
            }
            Op::Do(name) => out.push((name.clone(), ctm)),
            _ => {}
        }
    }
    out
}

fn num(o: &Obj) -> f64 {
    o.as_number().and_then(|n| n.parse().ok()).expect("number")
}

fn value_num(v: &Value) -> f64 {
    v.as_f64().expect("number in expected.json")
}

/// Checks our XObject stream against pdfTeX's summary, key by key.
fn check_xobject(file: &PdfFile, obj: &Obj, expected: &Value, what: &str) {
    let resolved = file.resolve(obj);
    let Obj::Stream { dict, raw } = resolved else {
        panic!("{what}: not a stream");
    };
    let get = |k: &str| file.get(dict, k);
    let name = |k: &str| get(k).and_then(Obj::as_name).map(str::to_string);
    let exp_str = |k: &str| expected.get(k).and_then(Value::as_str).map(str::to_string);
    assert_eq!(name("Subtype"), exp_str("Subtype"), "{what}: Subtype");
    assert_eq!(name("Filter"), exp_str("Filter"), "{what}: Filter");
    for key in ["Width", "Height", "BitsPerComponent"] {
        match expected.get(key) {
            Some(v) => assert_eq!(get(key).map(num), Some(value_num(v)), "{what}: {key}"),
            None => assert!(get(key).is_none(), "{what}: unexpected /{key}"),
        }
    }
    for key in ["BBox", "Matrix", "Decode"] {
        match expected.get(key).and_then(Value::as_array) {
            Some(v) => {
                let ours: Vec<f64> = get(key)
                    .and_then(Obj::as_array)
                    .unwrap_or_else(|| panic!("{what}: missing /{key}"))
                    .iter()
                    .map(num)
                    .collect();
                let theirs: Vec<f64> = v.iter().map(value_num).collect();
                assert_eq!(ours, theirs, "{what}: {key}");
            }
            None => assert!(get(key).is_none(), "{what}: unexpected /{key}"),
        }
    }
    match expected.get("ColorSpace") {
        None => assert!(
            get("ColorSpace").is_none(),
            "{what}: unexpected /ColorSpace"
        ),
        Some(Value::String(s)) => assert_eq!(
            name("ColorSpace").as_deref(),
            Some(s.as_str()),
            "{what}: ColorSpace"
        ),
        Some(v) => {
            let e = v.as_array().unwrap();
            let ours = get("ColorSpace")
                .and_then(Obj::as_array)
                .expect("indexed array");
            assert_eq!(ours[0].as_name(), e[0].as_str(), "{what}: Indexed");
            assert_eq!(ours[1].as_name(), e[1].as_str(), "{what}: base");
            assert_eq!(num(&ours[2]), value_num(&e[2]), "{what}: hival");
            let lookup = file.decode_stream(&ours[3]).expect("lookup stream");
            assert_eq!(
                Some(sha256::hex(&lookup).as_str()),
                expected.get("Lookup_sha256").and_then(Value::as_str),
                "{what}: palette"
            );
        }
    }
    let samples = if name("Filter").as_deref() == Some("DCTDecode") {
        raw.clone()
    } else {
        file.decode_stream(resolved).expect("decodable")
    };
    assert_eq!(
        Some(sha256::hex(&samples).as_str()),
        expected.get("samples_sha256").and_then(Value::as_str),
        "{what}: decoded samples differ from pdfTeX's"
    );
    match expected.get("SMask") {
        Some(m) => check_xobject(
            file,
            dict.get("SMask").expect("SMask"),
            m,
            &format!("{what} SMask"),
        ),
        None => assert!(dict.get("SMask").is_none(), "{what}: unexpected /SMask"),
    }
}

#[test]
fn pdftex_oracle_placement_objects_and_samples_match() {
    let expected = std::fs::read_to_string(format!("{IMAGES}/oracle/expected.json")).unwrap();
    let expected = json::parse(&expected).unwrap();
    let tolerance = value_num(expected.get("tolerance_bp").unwrap());
    let cases = expected.get("cases").and_then(Value::as_array).unwrap();
    assert!(cases.len() >= 10, "at least ten oracle fixtures");
    for case in cases {
        let name = case.get("name").and_then(Value::as_str).unwrap();
        let envelope = PathBuf::from(format!("{IMAGES}/oracle/{name}.v2.json"));
        let (doc, report) = v2::from_v2_file_rooted(&envelope, &V2Options::default(), root())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((report.images, report.image_resources), (1, 1), "{name}");
        let pdf = exact::render_exact(&doc).unwrap_or_else(|e| panic!("{name}: {e}"));
        verify::check_structure(&pdf.bytes).unwrap();
        assert_eq!(
            exact::render_exact(&doc).unwrap().bytes,
            pdf.bytes,
            "{name}: deterministic"
        );
        let file = PdfFile::parse(&pdf.bytes).unwrap();
        let pages = file.pages().unwrap();
        let page = pages[0];
        let ops = exact::parse(&file.page_content(page).unwrap()).unwrap();
        let placed = placements(&ops);
        assert_eq!(placed.len(), 1, "{name}");
        let (res, ctm) = &placed[0];
        let want: Vec<f64> = case
            .get("ctm")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .map(value_num)
            .collect();
        for k in 0..6 {
            assert!(
                (ctm[k] - want[k]).abs() <= tolerance,
                "{name}: CTM[{k}] {} vs pdfTeX {} (ours {ctm:?}, pdfTeX {want:?})",
                ctm[k],
                want[k]
            );
        }
        let xobjects = file
            .page_attr(page, "Resources")
            .and_then(Obj::as_dict)
            .and_then(|r| file.get(r, "XObject"))
            .and_then(Obj::as_dict)
            .unwrap();
        assert_eq!(xobjects.len(), 1, "{name}");
        check_xobject(
            &file,
            &xobjects[res.as_str()],
            case.get("xobject").unwrap(),
            name,
        );
        let group = matches!(case.get("page_group"), Some(Value::Bool(true)));
        assert_eq!(
            page.contains_key("Group"),
            group,
            "{name}: page transparency group"
        );
    }
}

fn read_case(name: &str) -> String {
    std::fs::read_to_string(format!("{IMAGES}/oracle/{name}.v2.json")).unwrap()
}

#[test]
fn image_bytes_are_read_rooted_without_links_and_hash_checked() {
    let envelope = read_case("01-rgb8");
    let dir = std::env::temp_dir().join(format!("flashtex-pdf-images-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("figs")).unwrap();
    let png = std::fs::read(format!("{IMAGES}/rgb8.png")).unwrap();
    std::fs::write(dir.join("figs/rgb8.png"), &png).unwrap();
    let rooted = |root: &Path, path: &str| {
        let text = envelope.replace("\"path\": \"rgb8.png\"", &format!("\"path\": {path:?}"));
        v2::from_v2_rooted(&text, &V2Options::default(), Some(root)).map(|_| ())
    };
    rooted(&dir, "figs/rgb8.png").unwrap();

    let e = v2::from_v2(&envelope, &V2Options::default()).unwrap_err();
    assert!(e.contains("needs a project root"), "{e}");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(dir.join("figs/rgb8.png"), dir.join("link.png")).unwrap();
        let e = rooted(&dir, "link.png").unwrap_err();
        assert!(e.contains("symbolic link"), "{e}");
        std::os::unix::fs::symlink(dir.join("figs"), dir.join("dirlink")).unwrap();
        let e = rooted(&dir, "dirlink/rgb8.png").unwrap_err();
        assert!(e.contains("symbolic link"), "{e}");
    }
    for bad in [
        "../rgb8.png",
        "figs/../figs/rgb8.png",
        "/etc/passwd",
        "./figs/rgb8.png",
    ] {
        let e = rooted(&dir, bad).unwrap_err();
        assert!(e.contains("must stay under the project root"), "{bad}: {e}");
    }
    let e = rooted(&dir, "figs").unwrap_err();
    assert!(e.contains("not a regular file"), "{e}");

    // Same length, different bytes: the hash refuses it.
    let mut changed = png.clone();
    let last = changed.len() - 20;
    changed[last] ^= 1;
    std::fs::write(dir.join("figs/rgb8.png"), &changed).unwrap();
    let e = rooted(&dir, "figs/rgb8.png").unwrap_err();
    assert!(e.contains("is stale") && e.contains("hash"), "{e}");
    std::fs::write(dir.join("figs/rgb8.png"), &png[..png.len() - 1]).unwrap();
    let e = rooted(&dir, "figs/rgb8.png").unwrap_err();
    assert!(e.contains("is stale") && e.contains("bytes on disk"), "{e}");
    let e = v2::from_v2_rooted(
        &envelope,
        &V2Options::default(),
        Some(Path::new("relative/dir")),
    )
    .unwrap_err();
    assert!(e.contains("not absolute"), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn stale_geometry_and_unknown_formats_are_refused() {
    let e = v2::from_v2_rooted(
        &read_case("01-rgb8").replace("\"pixel_width\": 48", "\"pixel_width\": 49"),
        &V2Options::default(),
        root(),
    )
    .unwrap_err();
    assert!(e.contains("decoded 48x32 px"), "{e}");
    let e = v2::from_v2_rooted(
        &read_case("16-pdf-rotate90").replace("\"pdf_rotate\": 90", "\"pdf_rotate\": 0"),
        &V2Options::default(),
        root(),
    )
    .unwrap_err();
    assert!(e.contains("/Rotate is 90"), "{e}");
    let e = v2::from_v2_rooted(
        &read_case("11-jpeg-rgb-96dpi").replace("\"format\": \"jpeg\"", "\"format\": \"gif\""),
        &V2Options::default(),
        root(),
    )
    .unwrap_err();
    assert!(e.contains("not png, jpeg or pdf"), "{e}");
    // A PNG declared as JPEG fails in the JPEG parser, not later.
    let e = v2::from_v2_rooted(
        &read_case("01-rgb8").replace("\"format\": \"png\"", "\"format\": \"jpeg\""),
        &V2Options::default(),
        root(),
    )
    .unwrap_err();
    assert!(e.contains("JPEG: no SOI"), "{e}");
}

#[test]
fn one_resource_per_distinct_image_and_page_group_only_for_alpha_channels() {
    // Two placements of the same bytes share /Im1; an RGBA image adds /Im2
    // and the page transparency group.
    // The envelopes are single-item pages: splice the item texts together.
    let split = |text: &str| -> (String, String, String) {
        // make_oracle.py writes indent=1: the items array closes at four spaces.
        let start = text.find("\"items\": [").unwrap() + "\"items\": [".len();
        let end = start + text[start..].find("\n    ]").unwrap();
        (
            text[..start].to_string(),
            text[start..end].to_string(),
            text[end..].to_string(),
        )
    };
    let (prefix, item_a, suffix) = split(&read_case("01-rgb8"));
    let (_, item_rgba, _) = split(&read_case("07-rgba8"));
    let combined = format!("{prefix}{item_a},{item_a},{item_rgba}{suffix}");
    json::parse(&combined).expect("spliced envelope is JSON");
    let (doc, report) = v2::from_v2_rooted(&combined, &V2Options::default(), root()).unwrap();
    assert_eq!((report.images, report.image_resources), (3, 2));
    assert_eq!(doc.images.keys().collect::<Vec<_>>(), ["Im1", "Im2"]);
    let pdf = exact::render_exact(&doc).unwrap();
    let file = PdfFile::parse(&pdf.bytes).unwrap();
    let page = file.pages().unwrap()[0];
    assert!(page.contains_key("Group"));
    let ops = exact::parse(&file.page_content(page).unwrap()).unwrap();
    let names: Vec<String> = placements(&ops).into_iter().map(|(n, _)| n).collect();
    assert_eq!(names, ["Im1", "Im1", "Im2"]);
}

#[test]
fn do_needs_a_declared_xobject_outside_text() {
    let doc = ExactDocument {
        pages: vec![ExactPage {
            width: Decimal::from_i64(100),
            height: Decimal::from_i64(100),
            content: Content::Verbatim(b"q /Im1 Do Q".to_vec()),
            fonts: None,
        }],
        fonts: BTreeMap::new(),
        images: BTreeMap::new(),
        patterns: BTreeMap::new(),
    };
    let e = exact::render_exact(&doc).unwrap_err().to_string();
    assert!(e.contains("XObject resource /Im1 is not declared"), "{e}");
    let png = std::fs::read(format!("{IMAGES}/gray2.png")).unwrap();
    let mut doc = doc;
    doc.images
        .insert("Im1".into(), flashtex_pdf::images::from_png(&png).unwrap());
    let ok = exact::render_exact(&doc).unwrap();
    verify::check_structure(&ok.bytes).unwrap();
    doc.pages[0].content = Content::Verbatim(b"BT /Im1 Do ET".to_vec());
    let e = exact::render_exact(&doc).unwrap_err().to_string();
    assert!(e.contains("Do inside a text object"), "{e}");
}
