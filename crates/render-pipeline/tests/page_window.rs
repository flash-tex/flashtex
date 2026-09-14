//! FT-070: the page window's acceptance gate
//! (`protocol/proposals/display-list-v2-window.md` §9).
//!
//! The gate is **equality, not a digest**: for every page of a document, the
//! page a windowed render produces must be byte-identical to that page in the
//! unwindowed render, and the windowed reply's document-wide parts — documents,
//! fonts, diagnostics, required features — must be byte-identical too.
//!
//! Equality rather than a digest because the risk this design carries is
//! precisely a producer that emits a correct-looking *smaller* font closure or
//! diagnostic list: a digest over a windowed list would only prove a consumer
//! rebuilt what it was sent.

mod common;

use common::lm_available;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{self, PageContent, PageWindow, Wire};
use flashtex_render_pipeline::{render_windowed, FontSet, RenderCache, RenderOptions};

const WIRE: Wire = Wire { images: true, device_color: true };

fn render_win(text: &str, window: Option<PageWindow>, cache: Option<&RenderCache>) -> display::DisplayList {
    let fonts = FontSet::with_default_dirs(&[]);
    let sources = [SourceDocument { path: "main.tex", text }];
    render_windowed(&sources, "main.tex", 7, "test-project", &fonts, &RenderOptions::default(), cache, window).v2
}

/// A body long enough to break over many pages, with maths and rules on it so
/// the font closure and the math diagnostics are not trivially empty.
fn long_body(paragraphs: usize) -> String {
    let mut s = String::from("\\documentclass[12pt]{article}\n\\begin{document}\n");
    for i in 0..paragraphs {
        s.push_str(&format!(
            "\\section{{Section {i}}}\nText for paragraph {i} with an inline formula $a_{{{i}}} + \\sum_{{k=1}}^{{n}} x_k^2$ and \
             enough words after it that the paragraph breaks over more than one line of the measure. \
             Some more words, and then some more, so the page filling is not degenerate.\n\n\
             \\[ \\int_0^1 f_{{{i}}}(x)\\,dx = \\frac{{{i}}}{{2}} \\]\n\n"
        ));
    }
    s.push_str("\\end{document}\n");
    s
}

fn page_bytes(p: &display::Page) -> String {
    let mut o = String::new();
    display::write_page(&mut o, p, WIRE);
    o
}

/// Every page, materialised through a window, is the page the unwindowed
/// render produced — byte for byte.
#[test]
fn every_windowed_page_equals_the_unwindowed_page() {
    if !lm_available() {
        return;
    }
    let text = long_body(24);
    let full = render_win(&text, None, None);
    assert!(full.window.is_none(), "an unwindowed render reports no window");
    assert!(full.pages.len() >= 6, "need a multi-page document; got {}", full.pages.len());
    let n = full.pages.len() as u32;

    for first in 1..=n {
        for count in [1u32, 3] {
            let windowed = render_win(&text, Some(PageWindow { first_page: first, page_count: count }), None);
            let w = windowed.window.expect("a windowed render reports its effective window");

            // The document is still whole: same page count, same frames.
            assert_eq!(windowed.pages.len(), full.pages.len(), "window {first}+{count} changed the page count");

            // The document-wide parts are the complete document's (§3).
            assert_eq!(windowed.fonts, full.fonts, "window {first}+{count} changed the font closure");
            assert_eq!(windowed.documents, full.documents, "window {first}+{count} changed the documents");
            assert_eq!(windowed.diagnostics, full.diagnostics, "window {first}+{count} changed the diagnostics");
            assert_eq!(
                windowed.required_features_wire(WIRE),
                full.required_features_wire(WIRE),
                "window {first}+{count} changed the required features"
            );

            for (wp, fp) in windowed.pages.iter().zip(&full.pages) {
                assert_eq!(wp.number, fp.number);
                assert_eq!(wp.width, fp.width);
                assert_eq!(wp.height, fp.height);
                if w.contains(fp.number) {
                    assert!(wp.is_resident(), "page {} is inside window {first}+{count} but was elided", fp.number);
                    assert_eq!(page_bytes(wp), page_bytes(fp), "page {} differs under window {first}+{count}", fp.number);
                } else {
                    assert!(
                        matches!(wp.content, PageContent::Elided),
                        "page {} is outside window {first}+{count} but was materialised",
                        fp.number
                    );
                }
            }
        }
    }
}

/// The same, with the block cache live across the requests: a cached
/// assembled block must not leak a stale placement into a later window.
#[test]
fn a_shared_cache_does_not_change_a_windowed_page() {
    if !lm_available() {
        return;
    }
    let text = long_body(16);
    let full = render_win(&text, None, None);
    let n = full.pages.len() as u32;
    assert!(n >= 4);

    let cache = RenderCache::new();
    // Walk the document forwards then backwards through one cache, the way a
    // viewer scrolling would.
    let order: Vec<u32> = (1..=n).chain((1..=n).rev()).collect();
    for first in order {
        let windowed = render_win(&text, Some(PageWindow { first_page: first, page_count: 2 }), Some(&cache));
        let w = windowed.window.expect("effective window");
        assert_eq!(windowed.fonts, full.fonts, "font closure drifted at window {first}");
        assert_eq!(windowed.diagnostics, full.diagnostics, "diagnostics drifted at window {first}");
        for (wp, fp) in windowed.pages.iter().zip(&full.pages) {
            if w.contains(fp.number) {
                assert_eq!(page_bytes(wp), page_bytes(fp), "page {} differs at window {first} through a shared cache", fp.number);
            }
        }
    }
}

/// An unwindowed render through the windowed entry point is the old render,
/// byte for byte — the property every corpus digest depends on.
#[test]
fn no_window_is_the_unwindowed_line() {
    if !lm_available() {
        return;
    }
    let text = long_body(20);
    let fonts = FontSet::with_default_dirs(&[]);
    let sources = [SourceDocument { path: "main.tex", text: &text }];
    let a = flashtex_render_pipeline::render(&sources, "main.tex", 7, "test-project", &fonts, &RenderOptions::default());
    let b = render_win(&text, None, None);
    assert_eq!(a.v2.write_json_wire("id", WIRE), b.write_json_wire("id", WIRE));
    assert!(a.window.is_none());
}

/// A window is clamped, never refused, and the clamp is what the reply says.
#[test]
fn windows_are_clamped_and_reported() {
    if !lm_available() {
        return;
    }
    let text = long_body(20);
    let full = render_win(&text, None, None);
    let n = full.pages.len() as u32;
    assert!(n >= 3);

    // Past the end: serve the last pages rather than nothing.
    let past = render_win(&text, Some(PageWindow { first_page: n + 50, page_count: 2 }), None);
    let w = past.window.expect("clamped window");
    assert!(w.first_page + w.page_count - 1 <= n, "clamped window {w:?} runs past page {n}");
    assert!(past.pages.iter().any(display::Page::is_resident), "a clamped window still materialises pages");

    // Wider than the document: every page is resident, and that is a complete
    // set of pages — but still reported as a window.
    let wide = render_win(&text, Some(PageWindow { first_page: 1, page_count: n + 100 }), None);
    // Clamped to the document, and never above MAX_WINDOW_PAGES -- which is a
    // reply-limit bound, not a memory one (see display.rs).
    assert_eq!(wide.window.expect("window").page_count, n.min(display::MAX_WINDOW_PAGES));
    if n <= display::MAX_WINDOW_PAGES {
        assert!(wide.pages.iter().all(display::Page::is_resident));
    }
    for (a, b) in wide.pages.iter().zip(&full.pages) {
        if a.is_resident() {
            assert_eq!(page_bytes(a), page_bytes(b));
        }
    }

    // Degenerate requests are not windows at all.
    for bad in [PageWindow { first_page: 0, page_count: 4 }, PageWindow { first_page: 1, page_count: 0 }] {
        assert!(render_win(&text, Some(bad), None).window.is_none(), "{bad:?} should not be a window");
    }
}

/// A windowed list is an incomplete view: it is never exported, and never
/// becomes a delta base (§4.1, §7).
#[test]
fn a_windowed_list_is_refused_where_completeness_is_required() {
    if !lm_available() {
        return;
    }
    let text = long_body(20);
    let windowed = render_win(&text, Some(PageWindow { first_page: 1, page_count: 2 }), None);
    let err = match flashtex_render_pipeline::pdf::write_pdf(&windowed) {
        Err(e) => e,
        Ok(_) => panic!("a windowed render must not export"),
    };
    assert!(err.contains("windowed"), "unhelpful refusal: {err}");

    let state = flashtex_render_pipeline::delta::DeltaState::new();
    let docs = vec![("main.tex".to_string(), text.clone())];
    let bytes = vec![0usize; windowed.pages.len()];
    flashtex_render_pipeline::delta::note_full(&state, "id", &windowed, WIRE, bytes, 1024, &docs);
    assert!(!state.has_snapshot(), "a windowed list must not be retained as a delta base");
}

/// An elided page serialises with its frame, an explicit `resident: false` and
/// no `items` key — so a consumer that ignored the flag fails to decode rather
/// than painting a blank page.
#[test]
fn an_elided_page_carries_no_items_key() {
    if !lm_available() {
        return;
    }
    let text = long_body(20);
    let windowed = render_win(&text, Some(PageWindow { first_page: 1, page_count: 1 }), None);
    let elided = windowed.pages.iter().find(|p| !p.is_resident()).expect("some page is elided");
    let json = page_bytes(elided);
    assert!(json.contains("\"resident\":false"), "{json}");
    assert!(!json.contains("\"items\""), "{json}");
    assert!(json.contains("\"number\""), "{json}");

    let resident = windowed.pages.iter().find(|p| p.is_resident()).expect("some page is resident");
    // A resident page gains no marker: an unwindowed line is unchanged.
    assert!(!page_bytes(resident).contains("\"resident\""));
}

/// `PageWindow::fitting` is the arithmetic that turns this from a memory
/// optimisation into the fix for a document that has no reply at all
/// (`display-list-v2-window.md` §1.0): at the 500 KB corpus case's measured
/// ~426 KB a page, a 16 MiB line carries tens of pages, not 385.
#[test]
fn a_window_is_sized_by_the_reply_limit() {
    const MIB16: u64 = 16 * 1024 * 1024;
    const MIB8: u64 = 8 * 1024 * 1024;
    // PR #273: 164 MB of display list over 385 pages.
    let bytes_per_page = 164_000_000u64 / 385;

    let w = PageWindow::fitting(200, 385, bytes_per_page, MIB16).expect("a window fits");
    assert!(u64::from(w.page_count) * bytes_per_page <= MIB16, "{w:?} does not fit 16 MiB");
    assert!(w.page_count >= 8, "a 16 MiB line should carry more than a handful: {w:?}");

    // The runtime's framed default is half that, so the window is smaller.
    let w8 = PageWindow::fitting(200, 385, bytes_per_page, MIB8).expect("a window fits");
    assert!(u64::from(w8.page_count) * bytes_per_page <= MIB8, "{w8:?} does not fit 8 MiB");
    assert!(w8.page_count <= w.page_count);

    // The whole document never fits, which is exactly today's failure.
    assert!(385 * bytes_per_page > MIB16);
    assert!(w.page_count < 385);

    // A page so large that not even one fits still yields a one-page window
    // rather than nothing: refusing is the protocol's job, not the window's.
    let huge = PageWindow::fitting(3, 10, MIB16 * 4, MIB16).expect("still a window");
    assert_eq!(huge.page_count, 1);

    // Never wider than the cap, however generous the limit.
    let generous = PageWindow::fitting(500, 1507, 1, MIB16).expect("a window");
    assert_eq!(generous.page_count, display::MAX_WINDOW_PAGES);
}
