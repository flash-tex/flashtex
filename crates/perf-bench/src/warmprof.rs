//! Per-phase profile of ONE warm keystroke (FT-070).
//!
//! `measure::decompose` splits a **cold** render. The number FT-070 is about is
//! the warm one, and the two do not have the same shape: the block caches turn
//! `adapt`/`typeset`/`assemble` into near-nothing while every whole-document
//! scan and every serialisation stays exactly as expensive as it was. Guessing
//! which of those dominates is how a day gets spent on the wrong phase, so this
//! measures it.

use std::time::Instant;

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::v1::Capabilities;
use flashtex_render_pipeline::{adapter, display, floats, protocol, toc, typeset, v1, FontSet, RenderCache, RenderOptions};

use crate::corpus::Case;
use crate::fontgate;
use crate::measure::{self, Caps, RunConfig};

#[derive(Default, Clone)]
pub struct Seams {
    pub req_json_ms: f64,
    pub scan_ms: f64,
    pub parse_ms: f64,
    pub labels_ms: f64,
    pub adapt_ms: f64,
    pub typeset_ms: f64,
    pub assemble_ms: f64,
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// One seam-by-seam replay of `render_cached`'s body, mirroring `lib.rs`.
/// Serialisation is not replayed here: it is timed in pass B on the real list.
fn replay(docs: &[(String, String)], entry: &str, revision: u64, fonts: &FontSet, options: &RenderOptions, cache: &RenderCache, s: &mut Seams) {
    let documents: Vec<SourceDocument<'_>> = docs.iter().map(|(p, t)| SourceDocument { path: p, text: t }).collect();

    // --- whole-document scans that run before the parser ---
    let t = Instant::now();
    let float_envs: Vec<Vec<floats::FloatEnv>> = documents
        .iter()
        .enumerate()
        .map(|(i, d)| floats::scan(d.text, flashtex_compiler::DocumentId(i)))
        .collect();
    let any_floats = float_envs.iter().any(|e| !e.is_empty());
    let masked: Vec<String> = documents
        .iter()
        .zip(&float_envs)
        .map(|(d, e)| if e.is_empty() { String::new() } else { floats::mask(d.text, e) })
        .collect();
    let texts: Vec<&str> = documents
        .iter()
        .zip(&float_envs)
        .zip(&masked)
        .map(|((d, e), m)| if e.is_empty() { d.text } else { m.as_str() })
        .collect();
    let multicol_scans: Vec<typeset::multicol::Scan> = texts.iter().map(|t| typeset::multicol::scan(t)).collect();
    let multicol_masked: Vec<Option<String>> = texts.iter().zip(&multicol_scans).map(|(t, sc)| sc.masked(t)).collect();
    let texts: Vec<&str> = texts.iter().zip(&multicol_masked).map(|(t, m)| m.as_deref().unwrap_or(t)).collect();
    let picture_ranges: Vec<Vec<(usize, usize)>> = texts
        .iter()
        .map(|t| flashtex_vector_graphics::tikz::find_pictures(t).into_iter().map(|p| (p.start, p.end)).collect())
        .collect();
    s.scan_ms += ms(t);

    // --- parse ---
    let parse_docs: Vec<SourceDocument<'_>> = documents.iter().zip(&texts).map(|(d, t)| SourceDocument { path: d.path, text: t }).collect();
    let t = Instant::now();
    let parsed = flashtex_compiler::parser::parse_project(&parse_docs, entry);
    s.parse_ms += ms(t);

    // --- labels / contents ---
    let t = Instant::now();
    let (float_numbers, float_label_values) = floats::number(&float_envs);
    let mut image_cache = floats::ImageCache::default();
    let paths: Vec<&str> = documents.iter().map(|d| d.path).collect();
    let entry_index = documents.iter().position(|d| d.path == entry).unwrap_or(0);
    let in_picture = |sp: &flashtex_compiler::Span| picture_ranges.get(sp.document.0).is_some_and(|r| r.iter().any(|(a, b)| sp.start >= *a && sp.start < *b));
    let mut labels = adapter::Labels::from_parsed(&parsed);
    labels.values.extend(float_label_values);
    let entry_text = texts.get(entry_index).copied().unwrap_or("");
    let has_lists = toc::has_lists(entry_text);
    labels.floats = toc::float_entries(&float_envs, &documents.iter().map(|d| d.text).collect::<Vec<_>>());
    if has_lists {
        let spans = toc::entry_spans(entry_text, flashtex_compiler::DocumentId(entry_index), &labels.floats);
        labels.entry_items = toc::entry_items(&documents, entry_index, &texts, options, &labels, &spans);
    }
    let superseded = toc::superseded_commands(entry_text);
    let is_superseded = |sp: &flashtex_compiler::Span| sp.document.0 == entry_index && superseded.binary_search(&sp.start).is_ok();
    s.labels_ms += ms(t);

    // --- adapt ---
    let t = Instant::now();
    let doc = adapter::adapt_cached(&texts, entry_index, &parsed, options, &labels, Some(cache));
    let mut diagnostics: Vec<display::Diagnostic> = parsed
        .diagnostics
        .iter()
        .filter(|d| !d.span.as_ref().is_some_and(&in_picture))
        .filter(|d| !d.span.as_ref().is_some_and(&is_superseded))
        .map(|d| display::Diagnostic::from_compiler(d, &paths))
        .collect();
    diagnostics.extend(doc.diagnostics.iter().cloned());
    let (float_specs, float_diagnostics) = if any_floats {
        floats::prepare(&float_envs, &float_numbers, &documents, entry_index, &texts, &doc.style, options, &labels, &mut image_cache)
    } else {
        (Vec::new(), Vec::new())
    };
    diagnostics.extend(float_diagnostics);
    s.adapt_ms += ms(t);

    // --- typeset ---
    let t = Instant::now();
    let mut ctx = typeset::Context::with_texts(fonts, &doc.style, &paths, &texts);
    ctx.set_math_colors(doc.math_colors.clone());
    typeset::multicol::attach(&mut ctx, &multicol_scans);
    let laid = typeset::build_with_floats(&mut ctx, &doc, Some(cache), &float_specs);
    diagnostics.extend(ctx.take_diagnostics());
    s.typeset_ms += ms(t);

    // --- assemble ---
    let t = Instant::now();
    let v2 = typeset::assemble("perfbench", revision, &documents, &doc.style, fonts, laid, diagnostics, Some(cache), doc.page_color, doc.default_color);
    s.assemble_ms += ms(t);
    std::hint::black_box(&v2);
}

/// Profiles one warm keystroke scenario and prints the split.
///
/// Three passes, each with its own `RenderCache` primed by the identical
/// unedited request and driven by the identical sequence of edited documents,
/// so all three caches evolve through the same states and the timings compare:
///
/// * **A** `protocol::handle_line` -- the authoritative total, and the record of
///   what the product path actually did (whether a `display_list` sibling line
///   was emitted at all, and how big each reply was).
/// * **B** `render_cached` alone. `A - B` is the reply cost, measured rather
///   than assumed.
/// * **C** a seam-by-seam replay of `render_cached`'s body, to split B.
///
/// The serialisation seams are timed in pass B on the real display list, and
/// only the ones `handle_line` would actually have run: over the line limit it
/// declines the display list without serialising it, and billing a keystroke
/// for a serialisation that never happened is how a phantom bottleneck is born.
pub fn run(case: &Case, cfg: &RunConfig, scenario: &str) -> Result<(), String> {
    let fonts = cfg.fonts.build();
    fontgate::preflight(&fonts)?;
    let options = measure::options_for(case);
    let owned: Vec<(String, String)> = case.docs.iter().map(|d| (d.path.clone(), d.text.clone())).collect();
    let entry_index = owned.iter().position(|(p, _)| *p == case.entry).unwrap_or(0);
    let base = case.entry_text().to_string();
    // Exactly what `Caps::V2` negotiates: rules-v1 + font-hints-v1 + display-list-v2.
    let caps_v2 = Capabilities {
        rules: true,
        font_hints: true,
        display_list: true,
        images: false,
        device_color: false,
        delta: false,
        v2_only: false,
    };
    let wire = display::Wire { images: false, device_color: false };

    let edit = measure::edit_named(case, scenario).ok_or_else(|| format!("{}: no scenario '{scenario}' in this document", case.id))?;
    let steps = cfg.steps.max(4);
    let text_for = |step: usize| measure::text_at_step(&base, edit, step);

    // ---- Pass A: the authoritative total, through the real product entry point.
    let cache_a = RenderCache::new();
    let mut docs_now = owned.clone();
    let _ = protocol::handle_line(&measure::request_line(0, &docs_now, &case.entry, Caps::V2), &fonts, &options, Some(&cache_a));
    let mut totals = Vec::new();
    let mut emitted_dl = 0usize;
    let mut reply_bytes = 0usize;
    let mut extra_bytes = 0usize;
    for step in 1..=steps {
        docs_now[entry_index].1 = text_for(step);
        let line = measure::request_line(step + 2, &docs_now, &case.entry, Caps::V2);
        let t = Instant::now();
        let reply = protocol::handle_line(&line, &fonts, &options, Some(&cache_a));
        totals.push(ms(t));
        if !reply.extra_lines.is_empty() {
            emitted_dl += 1;
            extra_bytes = reply.extra_lines.iter().map(String::len).sum();
        }
        reply_bytes = reply.line.len();
        if step == steps {
            let head: String = reply.line.chars().take(360).collect();
            println!("  [A] last reply head: {head}");
        }
    }

    // ---- Pass B: `render_cached` alone, plus the reply seams on the real list.
    let cache_b = RenderCache::new();
    let mut docs_now = owned.clone();
    let _ = protocol::handle_line(&measure::request_line(0, &docs_now, &case.entry, Caps::V2), &fonts, &options, Some(&cache_b));
    let mut renders = Vec::new();
    let mut v1_ms = Vec::new();
    let mut est_ms = Vec::new();
    let mut jsonv2_ms = Vec::new();
    let mut jsonv1_ms = Vec::new();
    let mut est_bytes = 0usize;
    for step in 1..=steps {
        docs_now[entry_index].1 = text_for(step);
        let sources: Vec<SourceDocument<'_>> = docs_now.iter().map(|(p, t)| SourceDocument { path: p, text: t }).collect();
        let id = format!("r{}", step + 2);
        let t = Instant::now();
        let rendered = flashtex_render_pipeline::render_cached(&sources, &case.entry, (step + 2) as u64, "perfbench", &fonts, &options, Some(&cache_b));
        renders.push(ms(t));

        let t = Instant::now();
        let payload = v1::fallback(&rendered.v2, caps_v2, Some(vec![]));
        v1_ms.push(ms(t));
        let t = Instant::now();
        let estimate = rendered.v2.estimated_json_bytes();
        est_ms.push(ms(t));
        est_bytes = estimate;
        // Only bill the display-list serialisation the product path would have
        // run: over the line limit `handle_line` declines it without serialising.
        if estimate <= protocol::max_reply_bytes() {
            let t = Instant::now();
            let dl = rendered.v2.write_json_wire(&id, wire);
            jsonv2_ms.push(ms(t));
            std::hint::black_box(&dl);
        } else {
            jsonv2_ms.push(0.0);
        }
        let t = Instant::now();
        let line = payload.write_envelope(&id);
        jsonv1_ms.push(ms(t));
        std::hint::black_box(&line);
    }

    // ---- Pass C: seam-by-seam replay, to split pass B.
    let cache_c = RenderCache::new();
    let mut docs_now = owned.clone();
    let _ = protocol::handle_line(&measure::request_line(0, &docs_now, &case.entry, Caps::V2), &fonts, &options, Some(&cache_c));
    let mut per_step: Vec<Seams> = Vec::new();
    for step in 1..=steps {
        docs_now[entry_index].1 = text_for(step);
        let line = measure::request_line(step + 2, &docs_now, &case.entry, Caps::V2);
        let mut s = Seams::default();
        // Request decode, as `handle_line_inner` does it: parse the envelope
        // (which carries the whole document as an escaped JSON string) and copy
        // each document out of it into an owned `String`.
        let t = Instant::now();
        let value = json::parse(&line).map_err(|e| e.0)?;
        let payload = value.get("payload").ok_or("no payload")?;
        let empty = Vec::new();
        let arr = payload.get("documents").and_then(Value::as_arr).unwrap_or(&empty);
        let project: Vec<(String, String)> = arr
            .iter()
            .map(|d| {
                (
                    d.get("path").and_then(Value::as_str).unwrap_or("").to_string(),
                    d.get("text").and_then(Value::as_str).unwrap_or("").to_string(),
                )
            })
            .collect();
        s.req_json_ms += ms(t);
        replay(&project, &case.entry, (step + 2) as u64, &fonts, &options, &cache_c, &mut s);
        per_step.push(s);
    }

    let pick = |f: fn(&Seams) -> f64| {
        let mut v: Vec<f64> = per_step.iter().map(f).collect();
        median(&mut v)
    };
    let med = |v: &[f64]| {
        let mut c = v.to_vec();
        median(&mut c)
    };
    let total = med(&totals);
    let render = med(&renders);

    println!("\n== warm keystroke profile: {} / {scenario} / v2 ==", case.id);
    println!("  steps {steps}, medians over steps, load1 {:.2}", crate::sys::load_avg().map_or(f64::NAN, |l| l.0));
    println!("  display_list sibling emitted on {emitted_dl}/{steps} steps");
    println!("  compile_result {reply_bytes} B, display_list {extra_bytes} B, estimate {est_bytes} B, limit {} B", protocol::max_reply_bytes());
    let (hb, mb) = cache_b.stats();
    println!("  pass-B block cache: {hb} hits / {mb} misses");
    println!();
    println!("  {:<44} {:>10} {:>8}", "phase", "ms", "% total");
    println!("  {:<44} {:>10.3} {:>7.1}%", "[A] handle_line TOTAL (authoritative)", total, 100.0);
    println!("  {:<44} {:>10.3} {:>7.1}%", "[B] render_cached", render, 100.0 * render / total);
    println!("  {:<44} {:>10.3} {:>7.1}%", "[A-B] reply build + serialise", total - render, 100.0 * (total - render) / total);
    println!("  -- reply seams, timed on the real display list --");
    for (name, v) in [
        ("v1::fallback", med(&v1_ms)),
        ("estimated_json_bytes", med(&est_ms)),
        ("write_json_wire (display-list-v2)", med(&jsonv2_ms)),
        ("write_envelope (compile_result)", med(&jsonv1_ms)),
    ] {
        println!("  {:<44} {:>10.3} {:>7.1}%", name, v, 100.0 * v / total);
    }
    println!("  -- render seams (replay of render_cached body) --");
    let rows: Vec<(&str, f64)> = vec![
        ("req_json (decode request + copy docs)", pick(|s| s.req_json_ms)),
        ("scan (floats + multicol + tikz)", pick(|s| s.scan_ms)),
        ("parse (parse_project: lex+expand+parse)", pick(|s| s.parse_ms)),
        ("labels (labels + contents)", pick(|s| s.labels_ms)),
        ("adapt (adapt_cached)", pick(|s| s.adapt_ms)),
        ("typeset (build_with_floats)", pick(|s| s.typeset_ms)),
        ("assemble (display list v2)", pick(|s| s.assemble_ms)),
    ];
    for (name, v) in &rows {
        println!("  {:<44} {:>10.3} {:>7.1}%", name, v, 100.0 * v / total);
    }
    let rsum: f64 = rows.iter().map(|(_, v)| v).sum();
    println!("  {:<44} {:>10.3} {:>7.1}%", "-- sum of render seams (compare to [B])", rsum, 100.0 * rsum / total);
    println!("  render-seam coverage {:.2}  (sum of render seams / [B])", rsum / render);
    Ok(())
}
