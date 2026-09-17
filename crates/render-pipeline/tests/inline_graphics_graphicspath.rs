//! GH-762 follow-up: `\graphicspath` for inline (non-float)
//! `\includegraphics`, and incremental staleness of measured inline boxes.
//!
//! (1) `\graphicspath{{dir/}...}` is graphics.sty's `\Ginput@path`: an
//! inline graphic tries the file as written first, then under each
//! directory, with the float path's extension search applied to every
//! candidate (every candidate goes through `floats::ImageCache::load`).
//!
//! (2) A paragraph holding an inline graphic is never served from the
//! incremental block cache: its measured box depends on the file's bytes,
//! the graphicx draft/demo mode and `\graphicspath`, none of which are in
//! the items the key hashes. The float path re-measures every render
//! through its per-request image cache; running text does the same by not
//! caching the block. Without that, adding `\graphicspath` (an edit that
//! leaves the paragraph's own bytes untouched) still shows the paragraph
//! as the previous render built it -- here, with the image missing.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render_cached, FontSet, RenderCache, RenderOptions};

/// `fixtures/float-graphics/images/objstm.pdf`, natural 150 x 60 bp: the
/// same file the float and inline oracles measure.
const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/float-graphics");

fn options() -> RenderOptions {
    RenderOptions { project_root: Some(ROOT.into()), ..RenderOptions::default() }
}

fn source(graphicspath: Option<&str>, body: &str) -> String {
    let graphicspath = graphicspath.unwrap_or_default();
    format!(
        "\\documentclass[10pt]{{article}}\n\
         \\usepackage[margin=1in]{{geometry}}\n\
         \\usepackage{{graphicx}}\n\
         {graphicspath}\
         \\pagestyle{{empty}}\n\
         \\setlength{{\\parindent}}{{0pt}}\n\
         \\begin{{document}}\n\
         Alpha alpha alpha alpha.\n\n\
         {body}\n\n\
         zzz zzz zzz zzz.\n\
         \\end{{document}}\n"
    )
}

fn render_one_lm(text: &str, cache: Option<&RenderCache>) -> flashtex_render_pipeline::Rendered {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    render_cached(&docs, "main.tex", 1, "inline-graphics-graphicspath", &fonts, &options(), cache)
}

/// `(x, top, width, height)` of every painted image, in bp.
fn image_geoms(r: &flashtex_render_pipeline::Rendered) -> Vec<(f64, f64, f64, f64)> {
    let mut out: Vec<_> = r
        .v2
        .pages
        .iter()
        .flat_map(|p| {
            p.resident_items().iter().filter_map(|it| match it {
                Item::Image(i) => Some((i.x.to_bp(), i.top.to_bp(), i.width.to_bp(), i.height.to_bp())),
                _ => None,
            })
        })
        .collect();
    out.sort_by(|a, b| (a.1, a.0).partial_cmp(&(b.1, b.0)).unwrap());
    out
}

fn image_diagnostics(r: &flashtex_render_pipeline::Rendered) -> Vec<String> {
    r.v2
        .diagnostics
        .iter()
        .filter(|d| d.code.starts_with("image") || d.code.starts_with("graphics"))
        .map(|d| format!("{}: {}", d.code, d.message))
        .collect()
}

fn assert_same_box(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) {
    for (i, (x, y)) in [a.0, a.1, a.2, a.3].into_iter().zip([b.0, b.1, b.2, b.3]).enumerate() {
        assert!((x - y).abs() < 0.01, "box component {i}: {a:?} vs {b:?}");
    }
}

#[test]
fn graphicspath_lists_each_group_last_command_wins() {
    use flashtex_render_pipeline::adapter::graphicspath;
    assert!(graphicspath("\\documentclass{article}\n\\begin{document}\nHi.\n\\end{document}\n").is_empty());
    assert_eq!(graphicspath("\\graphicspath{{images/}{figs/}}"), vec!["images/", "figs/"]);
    // Bare names are not groups (`\input@path` only ever holds brace
    // groups), and a later command overwrites the earlier list (`\def`).
    assert!(graphicspath("\\graphicspath{figs/}").is_empty());
    assert_eq!(
        graphicspath("\\graphicspath{{a/}}\n\\graphicspath{{b/}{c/}}"),
        vec!["b/", "c/"]
    );
    // A commented-out command is not one.
    assert!(graphicspath("%\\graphicspath{{images/}}\n").is_empty());
}

#[test]
fn graphicspath_resolves_inline_includegraphics() {
    if !common::lm_available() {
        return;
    }
    // Through `\graphicspath`, the bare file name paints the same box the
    // explicit path paints: 200 TeX pt = 199.272 bp wide, aspect-kept
    // 79.709 bp tall, at the 1 in margin.
    let via_path = render_one_lm(&source(Some("\\graphicspath{{images/}}\n"), "\\includegraphics[width=200pt]{objstm.pdf}"), None);
    assert!(image_diagnostics(&via_path).is_empty(), "{:?}", image_diagnostics(&via_path));
    let geoms = image_geoms(&via_path);
    assert_eq!(geoms.len(), 1, "one image expected, got {geoms:?}");
    let (x, _top, w, h) = geoms[0];
    assert!((x - 72.0).abs() < 1.0, "x {x}");
    assert!((w - 199.272).abs() < 1.0, "width {w}");
    assert!((h - 79.709).abs() < 1.0, "height {h}");

    // The explicit path, without any `\graphicspath`, paints the identical
    // box: the search only adds candidates, it never moves a file that is
    // already found as written.
    let explicit = render_one_lm(&source(None, "\\includegraphics[width=200pt]{images/objstm.pdf}"), None);
    assert!(image_diagnostics(&explicit).is_empty(), "{:?}", image_diagnostics(&explicit));
    let control = image_geoms(&explicit);
    assert_eq!(control.len(), 1, "one image expected, got {control:?}");
    assert_same_box(geoms[0], control[0]);

    // The float path's extension search applies inside each search
    // directory too: no extension, still found, same box.
    let no_ext = render_one_lm(&source(Some("\\graphicspath{{images/}}\n"), "\\includegraphics[width=200pt]{objstm}"), None);
    assert!(image_diagnostics(&no_ext).is_empty(), "{:?}", image_diagnostics(&no_ext));
    let geoms_no_ext = image_geoms(&no_ext);
    assert_eq!(geoms_no_ext.len(), 1, "one image expected, got {geoms_no_ext:?}");
    assert_same_box(geoms[0], geoms_no_ext[0]);

    // The control that proves the search matters: the bare name with no
    // `\graphicspath` is missing (an error) and paints nothing.
    let missing = render_one_lm(&source(None, "\\includegraphics[width=200pt]{objstm.pdf}"), None);
    assert!(image_geoms(&missing).is_empty(), "no image expected, got {:?}", image_geoms(&missing));
    assert!(
        image_diagnostics(&missing).iter().any(|m| m.starts_with("image_unavailable")),
        "an image_unavailable diagnostic is expected, got {:?}",
        image_diagnostics(&missing)
    );
}

/// Adding `\graphicspath` leaves the graphic's own paragraph bytes
/// untouched, so a block cache keyed on those bytes alone would keep
/// serving the paragraph as the previous render built it (image missing).
/// The incremental recompile must instead match a fresh compile exactly.
#[test]
fn inline_graphic_remeasures_after_incremental_edit() {
    if !common::lm_available() {
        return;
    }
    // The graphic shares its line with words, so the paragraph lays out
    // (and caches, before the fix) even while the file is missing.
    let body = "Charlie \\includegraphics[width=40pt]{objstm.pdf} delta.";
    let before = source(None, body);
    let after = source(Some("\\graphicspath{{images/}}\n"), body);

    let cache = RenderCache::new();
    let first = render_one_lm(&before, Some(&cache));
    assert!(image_geoms(&first).is_empty(), "the file is not at the root: {:?}", image_geoms(&first));

    let incremental = render_one_lm(&after, Some(&cache));
    let fresh = render_one_lm(&after, None);
    let json = |r: &flashtex_render_pipeline::Rendered| flashtex_compiler::json::write(&r.v2.to_json("e"));
    assert_eq!(json(&incremental), json(&fresh), "incremental recompile must match a fresh compile");

    // And the image really is there now: 40 TeX pt = 39.854 bp wide,
    // aspect-kept 15.942 bp tall.
    let geoms = image_geoms(&incremental);
    assert_eq!(geoms.len(), 1, "one image expected, got {geoms:?}");
    assert!((geoms[0].2 - 39.854).abs() < 1.0, "width {:?}", geoms[0]);
    assert!((geoms[0].3 - 15.942).abs() < 1.0, "height {:?}", geoms[0]);
    assert!(image_diagnostics(&incremental).is_empty(), "{:?}", image_diagnostics(&incremental));
}
