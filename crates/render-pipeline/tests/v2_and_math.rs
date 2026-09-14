//! v2 <-> v1 consistency, explicit math rules, capability negotiation and
//! page-break determinism.

mod common;

use common::*;
use flashtex_compiler::json;
use flashtex_render_pipeline::display::{Item, Provenance, Tick};
use flashtex_render_pipeline::v1::{Capabilities, V1Item};

const MATH_DOC: &str = "\\begin{document}Inline $x^2 + \\frac{a}{b}$ here.\n\n\\[ \\sum_{i=1}^{n} i = \\frac{n(n+1)}{2} \\]\nAfter.\\end{document}";

#[test]
fn v1_items_are_positioned_exactly_where_v2_glyph_runs_start() {
    if !lm_available() {
        return;
    }
    let r = render_one(MATH_DOC);
    let v1 = v1_of(&r, Capabilities { rules: true, font_hints: true, display_list: false, images: false, device_color: false, ..Capabilities::default() });
    // Only the outline-resource profile notes for lmmi/lmex (drawn from
    // Latin Modern Math) are expected; they make the status `recovered`.
    assert!(v1.diagnostics.iter().all(|d| d.code == "math_resource_profile"), "{:?}", v1.diagnostics);
    assert_eq!(v1.status, "recovered");
    let mut v2_origins: Vec<(f64, f64, String)> = Vec::new();
    let mut v2_rules = 0;
    for page in &r.v2.pages {
        for item in page.resident_items() {
            match item {
                Item::GlyphRun(run) => {
                    assert!(!run.glyphs.is_empty() && !run.clusters.is_empty());
                    assert!(run.glyphs.iter().all(|g| g.gid != 0), "gid 0 must never be emitted");
                    // Clusters partition the ActualText.
                    let mut pos = 0;
                    for c in &run.clusters {
                        assert_eq!(c.text_start_byte, pos);
                        assert!(c.text_end_byte > c.text_start_byte);
                        assert!(run.text.is_char_boundary(c.text_end_byte));
                        assert!(!c.hit_rects().is_empty());
                        assert!(!c.provenance.sources().is_empty());
                        pos = c.text_end_byte;
                    }
                    assert_eq!(pos, run.text.len());
                    assert!(run.glyphs.iter().all(|g| (g.cluster as usize) < run.clusters.len()));
                    let g = &run.glyphs[0];
                    v2_origins.push((g.origin_x.to_bp(), g.baseline_y.to_bp(), run.text.clone()));
                    for g in &run.glyphs[1..] {
                        v2_origins.push((g.origin_x.to_bp(), g.baseline_y.to_bp(), String::new()));
                    }
                }
                Item::Image(_) => {}
                Item::Rule(rule) => {
                    v2_rules += 1;
                    assert!(rule.width.0 > 0 && rule.height.0 > 0);
                    assert!(matches!(rule.provenance.sources(), [s] if &*s.path == "main.tex"));
                }
                Item::Path(_) => panic!("this document has no pictures"),
            }
        }
    }
    assert_eq!(v2_rules, 2, "two fraction bars");
    let mut v1_rules = 0;
    for page in &v1.pages {
        for it in &page.items {
            match it {
                V1Item::Text {
                    x_pt, baseline_y_pt, font, ..
                } => {
                    assert!(font.is_some(), "font hints were accepted");
                    assert!(
                        v2_origins.iter().any(|(x, y, _)| (x - x_pt).abs() < 1e-6 && (y - baseline_y_pt).abs() < 1e-6),
                        "v1 item at ({x_pt},{baseline_y_pt}) has no v2 glyph origin"
                    );
                }
                V1Item::Rule { .. } => v1_rules += 1,
            }
        }
    }
    assert_eq!(v1_rules, v2_rules);
    let fam: Vec<String> = v1
        .pages[0]
        .items
        .iter()
        .filter_map(|i| match i {
            V1Item::Text { font: Some(f), .. } => Some(f.family.to_string()),
            _ => None,
        })
        .collect();
    assert!(fam.contains(&"Latin Modern Roman".to_string()) && fam.contains(&"Latin Modern Math".to_string()));
}

#[test]
fn fraction_bars_are_explicit_rules_in_v2_and_negotiated_in_v1() {
    if !lm_available() {
        return;
    }
    let src = "\\begin{document}$\\frac{1}{2}$\\end{document}";
    let r = render_one(src);
    let math_span = src.find("\\frac").unwrap();
    let rules: Vec<_> = r
        .v2
        .pages
        .iter()
        .flat_map(|p| p.resident_items().iter())
        .filter_map(|i| if let Item::Rule(r) = i { Some(r) } else { None })
        .collect();
    assert_eq!(rules.len(), 1);
    let rule = rules[0];
    // TeX: \fontdimen8 of lmex10 (= cmex10) is 0.4pt at its fixed 10pt
    // design size, the \frac rule thickness pdflatex draws (0.398bp).
    let want = Tick::from_tex_pt(flashtex_math_layout::tfm::scale(41943, 10.0));
    assert!((rule.height.0 - want.0).abs() <= 32, "fixword 41943 at 10pt: {:?} vs {:?} (tolerance: one scaled point)", rule.height, want);
    let s = rule.provenance.sources();
    assert_eq!(s.len(), 1);
    assert!(s[0].start_byte <= math_span && s[0].end_byte >= math_span + "\\frac{1}{2}".len());

    // Legacy route: U+2500 approximation, no typed rule.
    let legacy = v1_of(&r, Capabilities::default());
    assert!(legacy.pages[0].items.iter().all(|i| matches!(i, V1Item::Text { .. })));
    let dash = legacy
        .pages[0]
        .items
        .iter()
        .find_map(|i| match i {
            V1Item::Text { text, font_size_pt, baseline_y_pt, .. } if text.starts_with('\u{2500}') => Some((*font_size_pt, *baseline_y_pt)),
            _ => None,
        })
        .expect("legacy rule text");
    assert!((dash.0 * 0.0857 - rule.height.to_bp()).abs() < 1e-6);
    assert!((dash.1 - (rule.top.to_bp() + rule.height.to_bp())).abs() < 1e-6);
    // Negotiated route: typed rule with top-left corner and source.
    let typed = v1_of(&r, Capabilities { rules: true, font_hints: false, display_list: false, images: false, device_color: false, ..Capabilities::default() });
    let typed_rule = typed
        .pages[0]
        .items
        .iter()
        .find_map(|i| match i {
            V1Item::Rule { x_pt, y_pt, width_pt, height_pt, source } => Some((*x_pt, *y_pt, *width_pt, *height_pt, source.clone())),
            _ => None,
        })
        .expect("typed rule");
    assert!((typed_rule.1 - rule.top.to_bp()).abs() < 1e-9);
    assert!((typed_rule.3 - rule.height.to_bp()).abs() < 1e-9);
    assert_eq!(&*typed_rule.4.path, "main.tex");
    assert!(typed.pages[0].items.iter().all(|i| !matches!(i, V1Item::Text { text, .. } if text.contains('\u{2500}'))));
    // No font hints unless accepted.
    assert!(typed.pages[0].items.iter().all(|i| !matches!(i, V1Item::Text { font: Some(_), .. })));
    let json = json::write(&typed.to_json());
    assert!(json.contains("\"layout_capabilities\":[\"rules-v1\"]"));
    assert!(json.contains("\"kind\":\"rule\""));
}

#[test]
fn page_breaks_are_deterministic_and_reach_a_second_page() {
    if !lm_available() {
        return;
    }
    let para = "The quick brown fox jumps over the lazy dog while the patient owl watches from an old oak branch and counts every leaf that falls into the quiet river below. ";
    let mut body = String::from("\\begin{document}");
    for _ in 0..30 {
        body.push_str(&para.repeat(3));
        body.push_str("\n\n");
    }
    body.push_str("\\end{document}");
    let a = render_one(&body);
    let b = render_one(&body);
    assert!(a.v2.pages.len() >= 2, "{} pages", a.v2.pages.len());
    assert_eq!(a.v2, b.v2);
    assert_eq!(json::write(&a.v2.to_json("x")), json::write(&b.v2.to_json("x")));
    // Every page's content stays inside the page and pages are numbered 1..n.
    for (i, p) in a.v2.pages.iter().enumerate() {
        assert_eq!(p.number as usize, i + 1);
        for item in p.resident_items() {
            if let Item::GlyphRun(r) = item {
                for g in &r.glyphs {
                    assert!(g.baseline_y.0 > 0 && g.baseline_y < p.height);
                    assert!(g.origin_x.0 >= 0 && g.origin_x < p.width);
                }
            }
        }
    }
    // Widow/club control: the last page carries at least two lines of its paragraph.
    let last = a.v2.pages.last().unwrap();
    let baselines: std::collections::BTreeSet<i64> = last
        .resident_items()
        .iter()
        .filter_map(|i| if let Item::GlyphRun(r) = i { Some(r.glyphs[0].baseline_y.0) } else { None })
        .collect();
    assert!(baselines.len() >= 2);
}

#[test]
fn v2_envelope_has_the_contract_shape_and_content_addressed_fonts() {
    if !lm_available() {
        return;
    }
    let r = render_one(MATH_DOC);
    let v = r.v2.to_json("req-1");
    let s = json::write(&v);
    let back = json::parse(&s).unwrap();
    assert_eq!(back.get("protocol_version").and_then(|v| v.as_i64()), Some(2));
    assert_eq!(back.get("type").and_then(|v| v.as_str()), Some("display_list"));
    let p = back.get("payload").unwrap();
    assert_eq!(p.get("coordinate_unit").and_then(|v| v.as_str()), Some("bp_2pow20"));
    assert_eq!(p.get("text_extraction").and_then(|v| v.as_str()), Some("cluster-actualtext"));
    let fonts = p.get("fonts").and_then(|v| v.as_arr()).unwrap();
    assert!(fonts.len() >= 2, "text + math faces");
    for f in fonts {
        let id = f.get("font_id").and_then(|v| v.as_str()).unwrap();
        assert_eq!(id.len(), 64);
        assert_eq!(f.get("sha256").and_then(|v| v.as_str()), Some(id));
        assert_eq!(f.get("format").and_then(|v| v.as_str()), Some("opentype-cff"));
    }
    // `sha256` is the digest of the RAW file bytes (what rendering-core and
    // font-resources verify), not font-engine's bytes||face_index id.
    for f in &r.v2.fonts {
        let path = f.path.as_ref().expect("loaded from a file");
        let bytes = std::fs::read(path).unwrap();
        let raw = flashtex_font_engine::sha256::hex(&flashtex_font_engine::sha256::digest(&bytes));
        assert_eq!(f.sha256, raw, "{}", path);
        assert_eq!(f.byte_length, bytes.len() as u64);
        let mut with_index = bytes.clone();
        with_index.extend_from_slice(&0u32.to_be_bytes());
        assert_ne!(f.sha256, flashtex_font_engine::sha256::hex(&flashtex_font_engine::sha256::digest(&with_index)), "engine id is not the resource digest");
    }
    let feats = p.get("required_features").and_then(|v| v.as_arr()).unwrap();
    assert!(feats.iter().any(|f| f.as_str() == Some("rule")));
    let docs = p.get("documents").and_then(|v| v.as_arr()).unwrap();
    assert_eq!(docs[0].get("sha256").and_then(|v| v.as_str()).map(str::len), Some(64));
}

#[test]
fn times_is_used_only_when_the_document_selects_it() {
    if !lm_available() {
        return;
    }
    let lm = render_one("\\documentclass{article}\\begin{document}Hello\\end{document}");
    assert!(lm.v2.fonts.iter().all(|f| f.format == "opentype-cff"));
    let times = render_one("\\documentclass{article}\\usepackage{times}\\begin{document}Hello\\end{document}");
    assert!(times.v2.fonts.iter().any(|f| f.format == "core14-afm" && f.postscript_name == "Times-Roman"));
}

/// A word set as several shaped fragments (kern/ligature boundaries) is
/// joined into one run; every caret's `text_byte` must be re-based onto the
/// joined text like the cluster ranges are (regression: `join_runs` shifted
/// the ranges only, so "office"/"before" carried carets outside their cluster
/// and the Mac consumer refused the whole frame).
#[test]
fn joined_word_fragments_keep_carets_inside_their_clusters() {
    if !lm_available() {
        return;
    }
    let r = render_one("\\begin{document}The AV office fixed the fi ligature: before the figure.\\end{document}");
    let mut runs = 0;
    for page in &r.v2.pages {
        for item in page.resident_items() {
            if let Item::GlyphRun(run) = item {
                runs += 1;
                for (ci, c) in run.clusters.iter().enumerate() {
                    let range = c.text_start_byte..=c.text_end_byte;
                    for caret in run.carets_of(ci).iter() {
                        assert!(range.contains(&caret.text_byte), "run {:?} cluster {ci}: caret {} outside {:?}", run.text, caret.text_byte, range);
                    }
                }
            }
        }
    }
    assert!(runs > 0);
}
