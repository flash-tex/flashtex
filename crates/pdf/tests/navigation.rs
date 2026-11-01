//! Links, destinations and outlines against pdfTeX's own objects.
//!
//! `tests/fixtures/hyperref/*.json` are extracted from MacTeX pdflatex
//! (pdfTeX 1.40.29, hyperref 7.01p) by
//! `crates/compiler/scripts/hyperref_oracle.py` (oracle only; no TeX runs
//! here). Each test builds a [`Navigation`] from exactly what pdflatex wrote,
//! renders it through the exact route, parses the result back with this
//! crate's reader and requires the same annotations (rectangle decimals,
//! `/Border`, `/C`, action), destinations (page and view), outline (titles,
//! tree depth, destinations, `/Count`), Info strings, `/PageMode` and
//! `/OpenAction`.

use flashtex_pdf::exact::{
    Content, Decimal, ExactDocument, ExactPage, render_exact, render_exact_with,
};
use flashtex_pdf::json::{self, Value};
use flashtex_pdf::navigation::{
    Destination, DocumentInfo, LinkAction, LinkAnnotation, Navigation, OutlineItem, PageMode, View,
};
use flashtex_pdf::reader::{Obj, PdfFile};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn d(s: &str) -> Decimal {
    Decimal::new(s).unwrap_or_else(|e| panic!("decimal {s:?}: {e}"))
}

fn fixture(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/hyperref")
        .join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    json::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn arr(v: Option<&Value>) -> &[Value] {
    v.and_then(Value::as_array).unwrap_or(&[])
}

fn text(v: Option<&Value>) -> String {
    v.and_then(Value::as_str).unwrap_or_default().to_string()
}

/// (page, rect, border, color, action, target)
type AnnotationRow = (usize, Vec<String>, Vec<String>, Vec<String>, String, String);

/// What a reader sees: the comparison unit.
#[derive(Debug, PartialEq)]
struct Summary {
    annotations: Vec<AnnotationRow>,
    /// name -> (page, view)
    destinations: BTreeMap<String, (usize, Vec<Option<String>>)>,
    /// (title, depth, destination, count)
    outlines: Vec<(String, usize, String, Option<i64>)>,
    info: BTreeMap<String, String>,
    page_mode: Option<String>,
    open_action: Option<(usize, String)>,
}

fn oracle_summary(expected: &Value) -> Summary {
    let mut annotations = Vec::new();
    for (index, page) in arr(expected.get("pages")).iter().enumerate() {
        for a in arr(page.get("annotations")) {
            let strings = |key: &str| {
                arr(a.get(key))
                    .iter()
                    .map(|v| text(Some(v)))
                    .collect::<Vec<_>>()
            };
            let action = text(a.get("action"));
            let target = if action == "GoTo" {
                text(a.get("destination"))
            } else {
                text(a.get("uri"))
            };
            annotations.push((
                index,
                strings("rect"),
                strings("border"),
                strings("color"),
                action,
                target,
            ));
        }
    }
    let mut destinations = BTreeMap::new();
    if let Some(Value::Object(map)) = expected.get("destinations") {
        for (name, dest) in map {
            let page = dest.get("page").and_then(Value::as_f64).unwrap() as usize - 1;
            let view = arr(dest.get("view"))
                .iter()
                .map(|v| v.as_str().map(str::to_string))
                .collect();
            destinations.insert(name.clone(), (page, view));
        }
    }
    let outlines = arr(expected.get("outlines"))
        .iter()
        .map(|o| {
            (
                text(o.get("title")),
                o.get("level").and_then(Value::as_f64).unwrap() as usize,
                text(o.get("destination")),
                o.get("count").and_then(Value::as_f64).map(|c| c as i64),
            )
        })
        .collect();
    let mut info = BTreeMap::new();
    if let Some(Value::Object(map)) = expected.get("info") {
        for (key, value) in map {
            info.insert(key.clone(), text(Some(value)));
        }
    }
    let catalog = expected.get("catalog");
    let open = arr(catalog.and_then(|c| c.get("open_action")));
    Summary {
        annotations,
        destinations,
        outlines,
        info,
        page_mode: catalog
            .and_then(|c| c.get("page_mode"))
            .and_then(Value::as_str)
            .map(str::to_string),
        open_action: (open.len() == 2)
            .then(|| (open[0].as_f64().unwrap() as usize - 1, text(open.get(1)))),
    }
}

/// The document pdflatex produced, as blank pages, and a [`Navigation`]
/// holding exactly its annotations, destinations, outline and Info.
fn oracle_navigation(expected: &Value) -> (ExactDocument, Navigation) {
    let pages = arr(expected.get("pages"));
    let doc = ExactDocument {
        pages: pages
            .iter()
            .map(|page| {
                let size = arr(page.get("size"));
                ExactPage {
                    width: d(&text(size.first())),
                    height: d(&text(size.get(1))),
                    content: Content::Ops(Vec::new()),
                    fonts: Some(Vec::new()),
                }
            })
            .collect(),
        ..ExactDocument::default()
    };
    let summary = oracle_summary(expected);
    let mut navigation = Navigation {
        links: vec![Vec::new(); pages.len()],
        ..Navigation::default()
    };
    for (page, rect, border, color, action, target) in &summary.annotations {
        let decimals = |values: &[String]| values.iter().map(|v| d(v)).collect::<Vec<_>>();
        let rect = decimals(rect);
        navigation.links[*page].push(LinkAnnotation {
            rect: [
                rect[0].clone(),
                rect[1].clone(),
                rect[2].clone(),
                rect[3].clone(),
            ],
            border: decimals(border),
            color: decimals(color),
            action: if action == "GoTo" {
                LinkAction::GoTo(target.clone())
            } else {
                LinkAction::Uri(target.clone())
            },
        });
    }
    for (name, (page, view)) in &summary.destinations {
        let view = match view.first().and_then(|v| v.as_deref()) {
            Some("XYZ") => View::Xyz {
                left: d(view[1].as_deref().unwrap()),
                top: d(view[2].as_deref().unwrap()),
            },
            Some("Fit") => View::Fit,
            other => panic!("{name}: view {other:?}"),
        };
        navigation
            .destinations
            .insert(name.clone(), Destination { page: *page, view });
    }
    navigation.outlines = summary
        .outlines
        .iter()
        .map(|(title, depth, destination, _)| OutlineItem {
            title: title.clone(),
            destination: destination.clone(),
            level: *depth as i32,
        })
        .collect();
    navigation.outlines_open = summary
        .outlines
        .iter()
        .any(|(.., count)| count.is_some_and(|c| c > 0));
    let field = |key: &str| summary.info.get(key).cloned();
    navigation.info = DocumentInfo {
        title: field("Title"),
        author: field("Author"),
        subject: field("Subject"),
        keywords: field("Keywords"),
        creator: field("Creator"),
    };
    navigation.page_mode = match summary.page_mode.as_deref() {
        Some("UseOutlines") => Some(PageMode::UseOutlines),
        Some("UseNone") => Some(PageMode::UseNone),
        _ => None,
    };
    navigation.open_fit_first_page = summary.open_action == Some((0, "Fit".to_string()));
    (doc, navigation)
}

fn decode_text(obj: &Obj) -> String {
    let Obj::String(bytes) = obj else {
        panic!("expected a string, got {obj:?}");
    };
    if let Some(utf16) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let units: Vec<u16> = utf16
            .chunks(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16(&units).unwrap();
    }
    bytes.iter().map(|&b| b as char).collect()
}

fn number_strings(pdf: &PdfFile, obj: Option<&Obj>) -> Vec<String> {
    obj.map(|o| pdf.resolve(o))
        .and_then(Obj::as_array)
        .unwrap_or(&[])
        .iter()
        .map(|v| v.as_number().unwrap_or_default().to_string())
        .collect()
}

fn name_tree(pdf: &PdfFile, node: &Obj, out: &mut BTreeMap<String, Obj>) {
    let node = pdf.resolve(node).as_dict().expect("name tree node");
    if let Some(names) = node.get("Names").and_then(Obj::as_array) {
        for pair in names.chunks(2) {
            out.insert(decode_text(&pair[0]), pair[1].clone());
        }
    }
    if let Some(kids) = node.get("Kids").and_then(Obj::as_array) {
        for kid in kids {
            name_tree(pdf, kid, out);
        }
    }
}

fn outline_items(
    pdf: &PdfFile,
    first: Option<&Obj>,
    depth: usize,
    out: &mut Vec<(String, usize, String, Option<i64>)>,
) {
    let mut current = first.cloned();
    while let Some(item_ref) = current {
        let item = pdf
            .resolve(&item_ref)
            .as_dict()
            .expect("outline item")
            .clone();
        let action = pdf
            .get(&item, "A")
            .and_then(Obj::as_dict)
            .expect("outline action");
        out.push((
            decode_text(pdf.get(&item, "Title").unwrap()),
            depth,
            decode_text(pdf.get(action, "D").unwrap()),
            pdf.get(&item, "Count")
                .and_then(Obj::as_number)
                .map(|c| c.parse().unwrap()),
        ));
        outline_items(pdf, item.get("First"), depth + 1, out);
        current = item.get("Next").cloned();
    }
}

fn our_summary(bytes: &[u8]) -> Summary {
    let pdf = PdfFile::parse(bytes).expect("our PDF parses");
    let catalog = pdf.catalog().unwrap().clone();
    let pages_root = pdf.get(&catalog, "Pages").and_then(Obj::as_dict).unwrap();
    let kids: Vec<u32> = pages_root
        .get("Kids")
        .and_then(Obj::as_array)
        .unwrap()
        .iter()
        .map(|k| match k {
            Obj::Ref(n, _) => *n,
            other => panic!("kid {other:?}"),
        })
        .collect();
    let page_index = |obj: &Obj| match obj {
        Obj::Ref(n, _) => kids.iter().position(|k| k == n).expect("page reference"),
        other => panic!("page {other:?}"),
    };
    let mut annotations = Vec::new();
    for (index, page) in pdf.pages().unwrap().iter().enumerate() {
        for annot in pdf
            .get(page, "Annots")
            .and_then(Obj::as_array)
            .unwrap_or(&[])
        {
            let annot = pdf.resolve(annot).as_dict().unwrap();
            assert_eq!(annot.get("Subtype").and_then(Obj::as_name), Some("Link"));
            assert_eq!(annot.get("H").and_then(Obj::as_name), Some("I"));
            let action = pdf.get(annot, "A").and_then(Obj::as_dict).unwrap();
            let kind = action.get("S").and_then(Obj::as_name).unwrap().to_string();
            let target = if kind == "GoTo" {
                decode_text(action.get("D").unwrap())
            } else {
                decode_text(action.get("URI").unwrap())
            };
            annotations.push((
                index,
                number_strings(&pdf, annot.get("Rect")),
                number_strings(&pdf, annot.get("Border")),
                number_strings(&pdf, annot.get("C")),
                kind,
                target,
            ));
        }
    }
    let mut destinations = BTreeMap::new();
    if let Some(names) = pdf.get(&catalog, "Names").and_then(Obj::as_dict) {
        let mut tree = BTreeMap::new();
        name_tree(&pdf, names.get("Dests").unwrap(), &mut tree);
        for (name, value) in tree {
            let dest = pdf.resolve(&value).as_dict().unwrap();
            let array = pdf.get(dest, "D").and_then(Obj::as_array).unwrap();
            let view = array[1..]
                .iter()
                .map(|v| match v {
                    Obj::Name(n) => Some(n.clone()),
                    Obj::Number(n) => Some(n.clone()),
                    Obj::Null => None,
                    other => panic!("view {other:?}"),
                })
                .collect();
            destinations.insert(name, (page_index(&array[0]), view));
        }
    }
    let mut outlines = Vec::new();
    if let Some(root) = pdf.get(&catalog, "Outlines").and_then(Obj::as_dict) {
        outline_items(&pdf, root.get("First"), 1, &mut outlines);
    }
    let mut info = BTreeMap::new();
    for (key, value) in pdf.info().unwrap() {
        if key != "Producer" {
            info.insert(key.clone(), decode_text(pdf.resolve(value)));
        }
    }
    let open_action = pdf
        .get(&catalog, "OpenAction")
        .and_then(Obj::as_array)
        .map(|a| (page_index(&a[0]), a[1].as_name().unwrap().to_string()));
    Summary {
        annotations,
        destinations,
        outlines,
        info,
        page_mode: pdf
            .get(&catalog, "PageMode")
            .and_then(Obj::as_name)
            .map(str::to_string),
        open_action,
    }
}

fn round_trip(case: &str) {
    let expected = fixture(case);
    let (doc, navigation) = oracle_navigation(&expected);
    let out = render_exact_with(&doc, &navigation).unwrap_or_else(|e| panic!("{case}: {e}"));
    let again = render_exact_with(&doc, &navigation).unwrap();
    assert_eq!(out.bytes, again.bytes, "{case}: deterministic");
    assert_eq!(our_summary(&out.bytes), oracle_summary(&expected), "{case}");
}

#[test]
fn autoref_links_destinations_and_closed_outline_match_pdflatex() {
    round_trip("01-autoref-sections");
}

#[test]
fn links_across_two_pages_match_pdflatex() {
    round_trip("13-pageref");
}

#[test]
fn colorlinks_borders_and_cite_destinations_match_pdflatex() {
    round_trip("17-colorlinks");
}

#[test]
fn nested_bookmark_tree_counts_match_pdflatex() {
    round_trip("22-bookmarks-tree");
}

#[test]
fn open_numbered_bookmarks_match_pdflatex() {
    round_trip("23-bookmarks-numbered-open");
}

#[test]
fn pdfbookmark_levels_match_pdflatex() {
    round_trip("25-pdfbookmark");
}

#[test]
fn a_url_broken_across_lines_is_two_annotations_like_pdflatex() {
    round_trip("27-url-breaking");
}

fn one_page() -> ExactDocument {
    ExactDocument {
        pages: vec![ExactPage {
            width: d("612"),
            height: d("792"),
            content: Content::Ops(Vec::new()),
            fonts: Some(Vec::new()),
        }],
        ..ExactDocument::default()
    }
}

fn link(action: LinkAction) -> LinkAnnotation {
    LinkAnnotation {
        rect: [d("72"), d("700"), d("100"), d("712")],
        border: vec![d("0"), d("0"), d("1")],
        color: vec![d("1"), d("0"), d("0")],
        action,
    }
}

fn xyz() -> Destination {
    Destination {
        page: 0,
        view: View::Xyz {
            left: d("72"),
            top: d("720"),
        },
    }
}

#[test]
fn an_empty_navigation_writes_the_same_bytes_as_render_exact() {
    let doc = one_page();
    assert_eq!(
        render_exact(&doc).unwrap().bytes,
        render_exact_with(&doc, &Navigation::default())
            .unwrap()
            .bytes
    );
}

#[test]
fn many_destinations_build_a_multi_level_name_tree_with_every_name() {
    let mut navigation = Navigation::default();
    for i in 0..100 {
        navigation
            .destinations
            .insert(format!("equation.{i}"), xyz());
    }
    let out = render_exact_with(&one_page(), &navigation).unwrap();
    let summary = our_summary(&out.bytes);
    assert_eq!(summary.destinations.len(), 100);
    let text = String::from_utf8_lossy(&out.bytes);
    // Byte order: equation.0, equation.1, equation.10 .. equation.13 first.
    assert!(
        text.contains("/Limits [(equation.0) (equation.13)]"),
        "sorted leaves of six"
    );
    assert!(text.contains("/Kids ["), "a parent level above the leaves");
}

#[test]
fn dangling_or_malformed_navigation_is_refused_before_writing() {
    let doc = one_page();
    let dangling = Navigation {
        links: vec![vec![link(LinkAction::GoTo("section.9".into()))]],
        ..Navigation::default()
    };
    assert!(render_exact_with(&doc, &dangling).is_err());

    let mut off_page = Navigation::default();
    off_page.destinations.insert(
        "x".into(),
        Destination {
            page: 3,
            view: View::Fit,
        },
    );
    assert!(render_exact_with(&doc, &off_page).is_err());

    let mut inverted = Navigation::default();
    let mut bad = link(LinkAction::Uri("https://example.com".into()));
    bad.rect = [d("100"), d("700"), d("72"), d("712")];
    inverted.links = vec![vec![bad]];
    assert!(render_exact_with(&doc, &inverted).is_err());

    let control = Navigation {
        links: vec![vec![link(LinkAction::Uri("https://a\nb".into()))]],
        ..Navigation::default()
    };
    assert!(render_exact_with(&doc, &control).is_err());

    let mut outline = Navigation::default();
    outline.outlines.push(OutlineItem {
        title: "T".into(),
        destination: "nowhere".into(),
        level: 1,
    });
    assert!(render_exact_with(&doc, &outline).is_err());

    let too_many_pages = Navigation {
        links: vec![Vec::new(), Vec::new()],
        ..Navigation::default()
    };
    assert!(render_exact_with(&doc, &too_many_pages).is_err());
}

#[test]
fn uri_strings_are_escaped_and_non_ascii_titles_are_utf16() {
    let mut navigation = Navigation::default();
    navigation.destinations.insert("s".into(), xyz());
    navigation.links = vec![vec![link(LinkAction::Uri(
        "https://example.com/a(b)".into(),
    ))]];
    navigation.outlines.push(OutlineItem {
        title: "Résumé".into(),
        destination: "s".into(),
        level: 1,
    });
    let out = render_exact_with(&one_page(), &navigation).unwrap();
    let summary = our_summary(&out.bytes);
    assert_eq!(summary.annotations[0].5, "https://example.com/a(b)");
    assert_eq!(summary.outlines[0].0, "Résumé");
}
