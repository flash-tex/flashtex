//! runtime-v1 fallback: every text item's source span is byte-exact into the
//! named document (multi-byte input, accents, TeX ligatures, `\input`).

mod common;

use common::*;
use flashtex_render_pipeline::v1::{Capabilities, V1Item};

fn text_items(payload: &flashtex_render_pipeline::v1::V1Payload) -> Vec<(String, String, usize, usize, f64, f64)> {
    let docs = payload.documents.clone();
    payload
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .filter_map(|it| match it {
            V1Item::Text {
                text,
                source,
                x_pt,
                baseline_y_pt,
                ..
            } => Some((text.clone(), docs.path(source.document).to_string(), source.start(), source.end(), *x_pt, *baseline_y_pt)),
            V1Item::Rule { .. } => None,
        })
        .collect()
}

#[test]
fn spans_are_byte_exact_including_multibyte_and_ligatures() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let src = "\\begin{document}Na\\\"ive caf\\'e --- office ``quoted'' Ünïcödé.\\end{document}";
    let r = render_one(src);
    let v1 = v1_of(&r, Capabilities::default());
    assert_eq!(v1.status, "ok", "{:?}", v1.diagnostics);
    let items = text_items(&v1);
    let words: Vec<&str> = items.iter().map(|i| i.0.as_str()).collect();
    assert_eq!(words, vec!["Naïve", "café", "\u{2014}", "office", "\u{201C}quoted\u{201D}", "Ünïcödé."]);
    for (text, path, start, end, _, _) in &items {
        assert_eq!(path, "main.tex");
        let slice = &src[*start..*end];
        // Every span lands on char boundaries and covers exactly the input
        // that produced the item.
        assert!(src.is_char_boundary(*start) && src.is_char_boundary(*end));
        match text.as_str() {
            "Naïve" => assert_eq!(slice, "Na\\\"ive"),
            "café" => assert_eq!(slice, "caf\\'e"),
            "\u{2014}" => assert_eq!(slice, "---"),
            "office" => assert_eq!(slice, "office"),
            "\u{201C}quoted\u{201D}" => assert_eq!(slice, "``quoted''"),
            "Ünïcödé." => assert_eq!(slice, "Ünïcödé."),
            other => panic!("unexpected item {other:?}"),
        }
    }
    // Items on one line share a baseline and advance left to right.
    let ys: Vec<f64> = items.iter().map(|i| i.5).collect();
    assert!(ys.iter().all(|y| (y - ys[0]).abs() < 1e-9));
    assert!(items.windows(2).all(|w| w[1].4 > w[0].4));
}

#[test]
fn included_documents_carry_their_own_path_and_offsets() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let main = "\\begin{document}Root text. \\input{chapter}\n\nBack in root.\\end{document}";
    let chapter = "Chapter wörds here.";
    let r = render_docs(&[("main.tex", main), ("chapter.tex", chapter)], "main.tex");
    let v1 = v1_of(&r, Capabilities::default());
    assert_eq!(v1.status, "ok", "{:?}", v1.diagnostics);
    let items = text_items(&v1);
    let by_path = |p: &str| -> Vec<(String, usize, usize)> {
        items.iter().filter(|i| i.1 == p).map(|i| (i.0.clone(), i.2, i.3)).collect()
    };
    let ch = by_path("chapter.tex");
    assert_eq!(ch.iter().map(|c| c.0.as_str()).collect::<Vec<_>>(), vec!["Chapter", "wörds", "here."]);
    for (text, s, e) in &ch {
        assert_eq!(&chapter[*s..*e], text);
    }
    let root = by_path("main.tex");
    assert_eq!(root.iter().map(|c| c.0.as_str()).collect::<Vec<_>>(), vec!["Root", "text.", "Back", "in", "root."]);
    for (text, s, e) in &root {
        assert_eq!(&main[*s..*e], text);
    }
    assert_eq!(r.v2.documents.len(), 2);
    assert_eq!(r.v2.documents[1].path, "chapter.tex");
    assert_eq!(r.v2.documents[1].byte_length, chapter.len() as u64);
}

#[test]
fn body_only_input_uses_the_compiler_geometry() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one("\\begin{document}Hello world.\\end{document}");
    let v1 = v1_of(&r, Capabilities::default());
    let page = &v1.pages[0];
    assert!((page.width_pt - 612.0).abs() < 1e-6);
    assert!((page.height_pt - 792.0).abs() < 1e-6);
    let items = text_items(&v1);
    // First baseline: 1in + \topskip (12 TeX pt) = 72 + 11.955 bp.
    assert!((items[0].4 - 72.0).abs() < 1e-6, "x {}", items[0].4);
    assert!((items[0].5 - 83.955).abs() < 1e-3, "baseline {}", items[0].5);
}
