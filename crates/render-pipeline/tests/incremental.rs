//! Incremental layout reuse must be invisible: for every edit of a script,
//! a worker that keeps its block cache produces the same `compile_result`
//! and `display_list` bytes as a fresh compile of the same text.

mod common;

use common::*;
use flashtex_compiler::parser::SourceDocument;
use flashtex_compiler::json;
use flashtex_render_pipeline::v1::{self, Capabilities};
use flashtex_render_pipeline::{render_cached, FontSet, RenderCache, RenderOptions};

const PARA: &str = "The quick brown fox jumps over the lazy dog while the patient owl watches from an old oak \
branch and counts every leaf that falls into the quiet river below. A \\textbf{bold} word, an \\emph{emphasised} \
one, the UTF-8 word caf\\'e, ``quotes'' --- and inline math $x_{i}^{2} + \\frac{a}{b} = \\sqrt{z}$ inside the \
sentence; then more text so that the paragraph wraps onto several lines of the page.";

fn document(sections: usize) -> String {
    let mut s = String::from("\\begin{document}\n");
    for i in 0..sections {
        s.push_str(&format!("\\section{{Part {i}}}\\label{{s{i}}}\n"));
        for j in 0..4 {
            s.push_str(&format!("Paragraph {i}.{j} of part \\ref{{s{i}}} on page \\pageref{{s{i}}}. {PARA}\n\n"));
        }
        s.push_str("\\[\n\\sum_{k=0}^{n} k^{2} = \\frac{n(n+1)(2n+1)}{6}\n\\]\nAfter the display the paragraph goes on.\n\n");
        if i % 3 == 2 {
            s.push_str("\\newpage\n");
        }
    }
    s.push_str("\\end{document}\n");
    s
}

/// A deterministic edit script: insert or delete a word at a position that
/// moves through the document, edit a display, and remove/reinsert a
/// paragraph so later blocks shift.
fn edited(base: &str, i: usize) -> String {
    let mut s = base.to_string();
    let body_start = s.find("\\begin{document}").unwrap() + 16;
    let span = s.len() - body_start - 20;
    let k = body_start + (i * 7919) % span;
    let k = s[..k].rfind(' ').unwrap_or(body_start);
    match i % 5 {
        0 => s.insert_str(k, &format!(" inserted{i}")),
        1 => {
            // delete the word after k
            let end = s[k + 1..].find(' ').map(|e| k + 1 + e).unwrap_or(k + 1);
            if !s[k..end].contains('\\') && !s[k..end].contains('$') && !s[k..end].contains('\n') {
                s.replace_range(k..end, "");
            } else {
                s.insert_str(k, " x");
            }
        }
        2 => {
            // change every display's numerator
            s = s.replace("k^{2} = ", &format!("k^{{{}}} = ", 2 + i % 3));
        }
        3 => {
            // drop one paragraph entirely
            if let Some(p) = s.find("Paragraph 1.2") {
                if let Some(e) = s[p..].find("\n\n") {
                    s.replace_range(p..p + e + 2, "");
                }
            }
        }
        _ => s.insert_str(k, &format!(" {}", "word ".repeat(1 + i % 4))),
    }
    s
}

fn lines(text: &str, fonts: &FontSet, cache: Option<&RenderCache>) -> (String, String) {
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render_cached(&docs, "main.tex", 1, "inc", fonts, &RenderOptions::default(), cache);
    let caps = Capabilities {
        images: false,
        rules: true,
        font_hints: true,
        display_list: true,
        device_color: false,
        ..Capabilities::default()
    };
    let payload = v1::fallback(&r.v2, caps, Some(vec!["rules-v1".into(), "font-hints-v1".into(), "display-list-v2".into()]));
    (payload.write_envelope("e"), json::write(&r.v2.to_json("e")))
}

fn check(sections: usize, edits: usize, min_pages: usize) {
    let fonts = FontSet::with_default_dirs(&[]);
    let cache = RenderCache::new();
    let base = document(sections);
    let mut pages = 0;
    for i in 0..edits {
        let text = edited(&base, i);
        let (fresh_v1, fresh_v2) = lines(&text, &fonts, None);
        let (inc_v1, inc_v2) = lines(&text, &fonts, Some(&cache));
        assert!(fresh_v1 == inc_v1, "compile_result differs at edit {i} ({} vs {} bytes)", fresh_v1.len(), inc_v1.len());
        assert!(fresh_v2 == inc_v2, "display_list differs at edit {i}");
        if i == 0 {
            pages = json::parse(&fresh_v2).unwrap().get("payload").unwrap().get("pages").unwrap().as_arr().unwrap().len();
        }
    }
    let (hits, misses) = cache.stats();
    eprintln!("{sections} sections, {pages} pages, {edits} edits: cache hits {hits}, misses {misses}");
    assert!(pages >= min_pages, "{pages} pages");
    assert!(hits > misses, "the cache should serve most blocks: {hits} hits, {misses} misses");
}

#[test]
fn incremental_output_is_byte_identical_over_200_edits_of_a_27_page_document() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    check(40, 200, 27);
}

/// ~2 minutes in release on a loaded machine (three `\pageref` passes per
/// fresh compile of 107 pages); run explicitly with `--ignored`.
#[test]
#[ignore = "slow: cargo test --release --test incremental -- --ignored"]
fn incremental_output_is_byte_identical_over_30_edits_of_a_107_page_document() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    check(160, 30, 107);
}
