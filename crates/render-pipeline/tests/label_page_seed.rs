//! Label-page seeding (`RenderCache::label_seed`): the previous request's
//! converged `\pageref`/contents-list page tables seed the next request's
//! first pass, the role LaTeX's `.aux` file plays between runs. An edit
//! that moves no page number then converges on pass 1; every warm output
//! must stay byte-identical to a fresh-cache compile of the same text.

mod common;

use common::lm_available;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Wire;
use flashtex_render_pipeline::{render_cached, FontSet, RenderCache, RenderOptions};

/// A contents list plus a `\pageref` per section, across several pages.
/// `lead` is inserted at the head of section 1's body: empty for the base
/// document, paragraphs for an edit that pushes later sections down a page.
fn doc(lead: &str) -> String {
    let mut t = String::from("\\documentclass{article}\n\\begin{document}\n\\tableofcontents\n\n");
    for s in 0..8 {
        t.push_str(&format!("\\section{{Topic {s}}}\\label{{sec:{s}}}\n"));
        if s == 1 {
            t.push_str(lead);
        }
        t.push_str(&format!(
            "Discussed on page~\\pageref{{sec:{s}}}. Enough ordinary filler words follow to wrap \
             this paragraph across several lines so the page builder has real work in section {s}.\n\n"
        ));
    }
    t.push_str("\\end{document}\n");
    t
}

fn render(text: &str, project: &str, fonts: &FontSet, cache: &RenderCache) -> (u32, String) {
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render_cached(&docs, "main.tex", 1, project, fonts, &RenderOptions::default(), Some(cache));
    (r.passes, r.v2.write_json_wire("t", Wire { images: false, device_color: false, diagnostics: false }))
}

#[test]
fn unchanged_text_converges_on_the_seeded_first_pass() {
    if !lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let cache = RenderCache::new();
    let text = doc("");
    let (cold_passes, cold) = render(&text, "seed", &fonts, &cache);
    assert!(cold_passes >= 2, "a cold \\pageref document needs a page-number pass: {cold_passes}");
    let (warm_passes, warm) = render(&text, "seed", &fonts, &cache);
    assert_eq!(warm_passes, 1, "the seeded tables are already the fixed point");
    assert_eq!(cold, warm, "seeding must not change a byte of the output");
}

#[test]
fn seed_is_per_project() {
    if !lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let cache = RenderCache::new();
    let text = doc("");
    let (_, _first) = render(&text, "seed-a", &fonts, &cache);
    let (other_passes, other) = render(&text, "seed-b", &fonts, &cache);
    assert!(other_passes >= 2, "another project's tables must not seed this one: {other_passes}");
    // Same bytes as project b compiled fresh (the id itself is in the JSON).
    let (_, fresh) = render(&text, "seed-b", &fonts, &RenderCache::new());
    assert_eq!(other, fresh);
}

#[test]
fn typed_character_that_moves_no_page_stays_one_pass_and_byte_identical() {
    if !lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let cache = RenderCache::new();
    let base = doc("");
    render(&base, "seed", &fonts, &cache);
    let edited = base.replace("real work in section 5", "realx work in section 5");
    assert_ne!(edited, base);
    let (warm_passes, warm) = render(&edited, "seed", &fonts, &cache);
    let (fresh_passes, fresh) = render(&edited, "seed", &fonts, &RenderCache::new());
    assert_eq!(warm, fresh, "a warm edit must render the same bytes as a fresh compile");
    assert_eq!(warm_passes, 1, "no page moved, so the seed converges at once");
    assert!(fresh_passes >= 2);
}

#[test]
fn edit_that_moves_pages_relays_out_and_matches_a_fresh_compile() {
    if !lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let cache = RenderCache::new();
    let base = doc("");
    render(&base, "seed", &fonts, &cache);
    let lead = "An inserted opening paragraph with plenty of ordinary words to occupy several \
                lines of the page before the original text resumes below it.\n\n"
        .repeat(12);
    let edited = doc(&lead);
    let (warm_passes, warm) = render(&edited, "seed", &fonts, &cache);
    let (_, fresh) = render(&edited, "seed", &fonts, &RenderCache::new());
    assert_eq!(warm, fresh, "a page-moving edit must render the same bytes as a fresh compile");
    assert!(warm_passes >= 2, "stale tables cannot converge on pass 1: {warm_passes}");
    // The document really does span more pages now, and \pageref values moved.
    assert_ne!(warm, {
        let (_, b) = render(&base, "seed", &fonts, &RenderCache::new());
        b
    });
}
