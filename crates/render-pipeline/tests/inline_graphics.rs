//! GH-762: `\includegraphics` in running text sets a real box.
//!
//! A graphic in a paragraph used to contribute no width, height or depth at
//! all (the adapter's `Inline::Graphic` arm pushed no `Item`), so the text
//! after it sat one plain `\baselineskip` below the text before it, and a
//! body holding only a graphic shipped no page. Every size below was
//! measured with `pdflatex` (pdfTeX 3.141592653-2.6-1.40.29, TeX Live 2026)
//! against the same documents; pdflatex is an oracle only, nothing here runs
//! TeX. Tolerances are the project's 1 bp image gate, except the image box
//! itself, which is pure key arithmetic (`width=200pt` of a 72x36 bp file)
//! and pinned tight.
//!
//! The image files are written by the tests themselves (a minimal one-page
//! PDF with the required `MediaBox`), so no binary fixture is committed.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, Rule, Severity};
use flashtex_render_pipeline::{render_cached, FontSet, RenderCache, RenderOptions};

/// `width`/`height` of an image item in bp, and of a rule.
fn rect(it: &Item) -> (f64, f64, f64, f64) {
    match it {
        Item::Image(i) => (i.x.to_bp(), i.top.to_bp(), i.width.to_bp(), i.height.to_bp()),
        Item::Rule(Rule { x, top, width, height, .. }) => (x.to_bp(), top.to_bp(), width.to_bp(), height.to_bp()),
        other => panic!("not a rect item: {other:?}"),
    }
}

fn images_of(r: &flashtex_render_pipeline::Rendered) -> Vec<(f64, f64, f64, f64)> {
    r.v2.pages
        .iter()
        .flat_map(|p| p.resident_items())
        .filter_map(|it| match it {
            Item::Image(_) => Some(rect(it)),
            _ => None,
        })
        .collect()
}

fn rules_of(r: &flashtex_render_pipeline::Rendered) -> Vec<(f64, f64, f64, f64)> {
    let mut out: Vec<(f64, f64, f64, f64)> = r
        .v2
        .pages
        .iter()
        .flat_map(|p| p.resident_items())
        .filter_map(|it| match it {
            Item::Rule(_) => Some(rect(it)),
            _ => None,
        })
        .collect();
    out.sort_by(|a, b| a.partial_cmp(b).unwrap());
    out
}

/// Baseline of the word reading `text` (first match), in bp.
fn baseline(r: &flashtex_render_pipeline::Rendered, text: &str) -> f64 {
    common::words_of(r).iter().find(|w| w.text == text).unwrap_or_else(|| panic!("no word {text:?}")).baseline
}

/// A minimal one-page PDF whose page box is `w` x `h` bp.
fn pdf_bytes(w: u32, h: u32) -> Vec<u8> {
    let objs: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] >>").into_bytes(),
    ];
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offs = Vec::new();
    for (i, o) in objs.iter().enumerate() {
        offs.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offs {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF", objs.len() + 1).as_bytes());
    out
}

/// A scratch project root holding `pic.pdf` (`w` x `h` bp), removed first so
/// a previous run's file cannot linger.
fn project_dir(tag: &str, w: u32, h: u32) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("flashtex-inline-graphics-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch project root");
    std::fs::write(dir.join("pic.pdf"), pdf_bytes(w, h)).expect("scratch image");
    dir
}

fn render_in(dir: &std::path::Path, tex: &str) -> flashtex_render_pipeline::Rendered {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text: tex }];
    let options = RenderOptions { project_root: Some(dir.into()), ..RenderOptions::default() };
    flashtex_render_pipeline::render(&docs, "main.tex", 1, "inline-graphics", &fonts, &options)
}

const PREAMBLE: &str = "\\documentclass{article}\n\\usepackage[margin=1in]{geometry}\n\\usepackage{graphicx}\n";

/// The #762 repro: an inline graphic displaces the following baseline by its
/// own height (through `\lineskip`), exactly as in pdflatex.
#[test]
fn inline_graphic_displaces_the_following_baseline() {
    if !common::lm_available() {
        return;
    }
    let dir = project_dir("repro", 72, 36);
    let tex = format!("{PREAMBLE}\\begin{{document}}\nAlpha.\n\nBravo \\includegraphics[width=200pt]{{pic.pdf}} xxx.\n\nAfter.\n\\end{{document}}\n");
    let r = render_in(&dir, &tex);
    assert_eq!(r.v2.pages.len(), 1, "one page");
    assert!(r.v2.diagnostics.is_empty(), "no diagnostics: {:?}", r.v2.diagnostics);
    // `width=200pt` of a 72x36 bp file: 200pt x 100pt, i.e. this rect in bp.
    let got = images_of(&r);
    assert_eq!(got.len(), 1, "the graphic paints one image: {got:?}");
    let (_, _, w, h) = (got[0].0, got[0].1, got[0].2, got[0].3);
    assert!((w - 200.0 * 72.0 / 72.27).abs() < 0.02, "image width: {w}");
    assert!((h - 100.0 * 72.0 / 72.27).abs() < 0.02, "image height: {h}");
    // pdflatex puts `Bravo`/`xxx` at 184.528 bp and `After` at 196.483 bp;
    // before the fix both sat a plain baselineskip below `Alpha` (the
    // graphic added nothing) and no image shipped at all.
    let bravo = baseline(&r, "Bravo");
    let xxx = baseline(&r, "xxx.");
    assert!((bravo - 184.528).abs() < 1.0, "Bravo baseline: {bravo}");
    assert!((xxx - 184.528).abs() < 1.0, "xxx baseline: {xxx}");
    assert!((baseline(&r, "After.") - 196.483).abs() < 1.0, "After baseline");
    let _ = std::fs::remove_dir_all(&dir);
}

/// No keys: the graphic sets at its natural size.
#[test]
fn inline_graphic_at_natural_size() {
    if !common::lm_available() {
        return;
    }
    let dir = project_dir("natural", 72, 36);
    let tex = format!("{PREAMBLE}\\begin{{document}}\nBravo \\includegraphics{{pic.pdf}} xxx.\n\\end{{document}}\n");
    let r = render_in(&dir, &tex);
    assert_eq!(r.v2.pages.len(), 1);
    let got = images_of(&r);
    assert_eq!(got.len(), 1, "{got:?}");
    assert!((got[0].2 - 72.0).abs() < 0.02, "natural width: {:?}", got[0]);
    assert!((got[0].3 - 36.0).abs() < 0.02, "natural height: {:?}", got[0]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// `draft` with a missing file: pdfTeX warns and reserves one inch square
/// scaled by the keys, framing it with default-thickness rules.
#[test]
fn inline_draft_missing_file_warns_and_frames() {
    if !common::lm_available() {
        return;
    }
    let dir = project_dir("draft", 72, 36);
    let tex = format!(
        "{}\\usepackage[draft]{{graphicx}}\n\\begin{{document}}\nBravo \\includegraphics[width=100pt]{{missing.png}} xxx.\n\\end{{document}}\n",
        "\\documentclass{article}\n\\usepackage[margin=1in]{geometry}\n"
    );
    let r = render_in(&dir, &tex);
    assert_eq!(r.v2.pages.len(), 1);
    assert!(
        r.v2.diagnostics.iter().any(|d| d.code == "image_unavailable" && d.severity == Severity::Warning),
        "draft warns, not errors: {:?}",
        r.v2.diagnostics.iter().map(|d| (&d.code, &d.severity)).collect::<Vec<_>>()
    );
    assert!(images_of(&r).is_empty(), "a draft frame paints rules, not an image");
    // One inch square asked for by width alone comes out square (pdfTeX:
    // 100pt x 100pt); the frame is four 0.4pt rules around that box.
    let side = 100.0 * 72.0 / 72.27;
    let got = rules_of(&r);
    assert_eq!(got.len(), 4, "a frame is four rules: {got:?}");
    let (left, top) = (got[0].0, got[0].1);
    let (right, bottom) = (got.iter().map(|r| r.0 + r.2).fold(f64::MIN, f64::max), got.iter().map(|r| r.1 + r.3).fold(f64::MIN, f64::max));
    assert!((right - left - side).abs() < 0.02, "frame width: {got:?}");
    assert!((bottom - top - side).abs() < 0.02, "frame height: {got:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// No `draft`, missing file, explicit size: pdfTeX's package error, and the
/// requested size is kept empty (space, no ink).
#[test]
fn inline_missing_file_without_draft_is_an_error_keeping_the_size() {
    if !common::lm_available() {
        return;
    }
    let dir = project_dir("missing", 72, 36);
    let tex = format!("{PREAMBLE}\\begin{{document}}\nBravo \\includegraphics[width=100pt,height=50pt]{{missing.png}} xxx.\n\\end{{document}}\n");
    let r = render_in(&dir, &tex);
    assert_eq!(r.v2.pages.len(), 1, "the page still ships");
    assert!(
        r.v2.diagnostics.iter().any(|d| d.code == "image_unavailable" && d.severity == Severity::Error),
        "a missing file without draft is an error: {:?}",
        r.v2.diagnostics.iter().map(|d| (&d.code, &d.severity)).collect::<Vec<_>>()
    );
    assert!(images_of(&r).is_empty(), "nothing is painted");
    assert!(rules_of(&r).is_empty(), "no frame either: {:?}", rules_of(&r));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A body holding only a graphic ships one page with the image at its
/// natural size, like pdflatex (not zero pages).
#[test]
fn lone_graphic_sets_one_page_with_the_image() {
    if !common::lm_available() {
        return;
    }
    let dir = project_dir("lone", 72, 36);
    let tex = format!("{PREAMBLE}\\begin{{document}}\n\\includegraphics{{pic.pdf}}\n\\end{{document}}\n");
    let r = render_in(&dir, &tex);
    assert_eq!(r.v2.pages.len(), 1, "a lone graphic is a page, as in pdflatex");
    let got = images_of(&r);
    assert_eq!(got.len(), 1, "{got:?}");
    assert!((got[0].2 - 72.0).abs() < 0.02 && (got[0].3 - 36.0).abs() < 0.02, "natural size: {:?}", got[0]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Paragraphs holding graphics bypass the cross-request item cache: the same
/// source against a rewritten image file must pick up the new size, not a
/// stale box. (The cache key cannot see the file; without the bypass the
/// second render below would reuse the first render's 72 bp box.)
#[test]
fn graphic_paragraphs_bypass_the_cross_request_cache() {
    if !common::lm_available() {
        return;
    }
    let dir1 = project_dir("cache1", 72, 36);
    let dir2 = project_dir("cache2", 144, 72);
    let tex = format!("{PREAMBLE}\\begin{{document}}\nBravo \\includegraphics{{pic.pdf}} xxx.\n\\end{{document}}\n");
    let fonts = FontSet::with_default_dirs(&[]);
    let cache = RenderCache::new();
    let render = |dir: &std::path::Path| {
        let docs = [SourceDocument { path: "main.tex", text: tex.as_str() }];
        let options = RenderOptions { project_root: Some(dir.into()), ..RenderOptions::default() };
        render_cached(&docs, "main.tex", 1, "inline-graphics-cache", &fonts, &options, Some(&cache))
    };
    let w1 = {
        let got = images_of(&render(&dir1));
        assert_eq!(got.len(), 1);
        got[0].2
    };
    let w2 = {
        let got = images_of(&render(&dir2));
        assert_eq!(got.len(), 1);
        got[0].2
    };
    assert!((w1 - 72.0).abs() < 0.02, "first root: {w1}");
    assert!((w2 - 144.0).abs() < 0.02, "rewritten file is picked up, not cached: {w2}");
    let _ = std::fs::remove_dir_all(&dir1);
    let _ = std::fs::remove_dir_all(&dir2);
}

/// A graphic nested in a float's group (or `tabular` cell) is still the
/// float path's case, not this fix's: it takes no space and still reports
/// `float_content_unsupported`.
#[test]
fn graphic_nested_in_a_float_still_warns_and_takes_no_space() {
    if !common::lm_available() {
        return;
    }
    let dir = project_dir("nested", 72, 36);
    let tex = format!(
        "{PREAMBLE}\\begin{{document}}\nHello.\n\\begin{{figure}}[t]\n\\centering\n{{\\includegraphics[width=200pt]{{pic.pdf}}}}\n\\caption{{A.}}\n\\end{{figure}}\nWorld.\n\\end{{document}}\n"
    );
    let r = render_in(&dir, &tex);
    assert_eq!(r.v2.pages.len(), 1);
    assert!(
        r.v2.diagnostics.iter().any(|d| d.code == "float_content_unsupported"),
        "the warning still fires: {:?}",
        r.v2.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
    assert!(images_of(&r).is_empty(), "the nested graphic still takes no space");
    let _ = std::fs::remove_dir_all(&dir);
}
