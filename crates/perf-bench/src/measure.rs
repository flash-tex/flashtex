//! One repetition of one case, measured inside a process of its own.
//!
//! Why a child process per repetition: "cold" has to mean cold. The pipeline
//! keeps a `RenderCache` the caller controls, but the expander also keeps a
//! thread-local stream cache, the font set memoises faces and metrics, and the
//! allocator keeps whatever the last document taught it. Only a fresh process
//! clears all of those at once, and only a fresh process can report a peak RSS
//! that belongs to one document instead of to the whole suite.
//!
//! The parent spawns N children per case and takes order statistics over them,
//! so the spawn is not on any timed path — each child times from inside its
//! own `main`.

use std::time::Instant;

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::parser::SourceDocument;
use flashtex_font_engine::sha256;
use flashtex_render_pipeline::v1::Capabilities;
use flashtex_render_pipeline::{adapter, pdf, protocol, typeset, v1, FontSet, RenderCache, RenderOptions};

use crate::corpus::Case;
use crate::fontgate::{self, FontConfig};
use crate::sys;

/// The keystroke scenarios. Each types one more character per step at a fixed
/// place against a warm cache — the single-edit path the editor drives.
pub const SCENARIOS: &[&str] = &["type-paragraph", "type-inline-math", "type-display-eq", "delete-restore-line"];

/// What gets typed, cycled. Contains spaces on purpose: a run of one letter
/// would grow an unbreakable word and start measuring overfull-box recovery
/// instead of ordinary line breaking. Same bytes as the FT-065 harness.
const TYPED: &[u8] = b"abcde fghij ";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Caps {
    /// `rules-v1` + `font-hints-v1`: what the Mac app negotiates by default.
    V1,
    /// ... plus `display-list-v2`: the V2 pane, and the heavier path.
    V2,
}

impl Caps {
    pub fn as_str(self) -> &'static str {
        match self {
            Caps::V1 => "v1",
            Caps::V2 => "v2",
        }
    }
}

#[derive(Clone, Copy)]
enum Edit {
    Type { offset: usize },
    DeleteLine { start: usize, end: usize },
}

/// Per-phase split of one cold render, taken at the public seams the pipeline
/// already exposes (the same ones `examples/stages.rs` uses).
///
/// This is a *decomposition*, not instrumentation inside `render_cached`:
/// `crates/render-pipeline` belongs to another lane, so the harness does not
/// add hooks to it. The consequence is recorded honestly as `coverage` —
/// documents with floats, `\pageref` or contents lists run extra passes in the
/// product path that the decomposition does not replay, and their coverage is
/// below 1.0. Use the phases to attribute a regression, and
/// `cold.render_ms` as the number that is actually true.
#[derive(Clone, Default)]
pub struct Phases {
    /// Macro expansion alone (`expansion::expand_project`, uncached),
    /// measured *after* `parse` on the same input.
    ///
    /// Informational, and excluded from the coverage sum: `parse_project`
    /// expands internally, so this overlaps `parse_ms` rather than adding to
    /// it. The first attempt at this reported `parse - expand`, which came out
    /// at 0.00 ms on HW1 — not because parsing is free, but because whichever
    /// of the two runs second pays less for cold code and a cold allocator.
    /// A derived difference between two measurements of overlapping work is
    /// not a phase, so it is no longer presented as one.
    pub expand_ms: f64,
    /// `parser::parse_project`: lexing, macro expansion and parsing. One
    /// seam, one number.
    pub parse_ms: f64,
    /// Parse tree to styled blocks.
    pub adapt_ms: f64,
    /// Shaping, Knuth-Plass line breaking, Appendix G math boxes and page
    /// building. These four cannot be separated from outside the crate; the
    /// `--perf-attribution` mode splits them by symbol instead.
    pub typeset_ms: f64,
    /// The same typesetting without the document sources (`Context::new`):
    /// a control, not a stage. Excluded from the coverage sum.
    pub typeset_notexts_ms: f64,
    /// Display-list v2 emission.
    pub assemble_ms: f64,
    /// runtime-v1 fallback payload.
    pub v1_ms: f64,
    /// Serialising the runtime-v1 envelope.
    pub json_v1_ms: f64,
    /// Serialising the display-list-v2 envelope.
    pub json_v2_ms: f64,
}

impl Phases {
    pub const NAMES: &'static [&'static str] =
        &["expand", "parse", "adapt", "typeset", "typeset_notexts", "assemble", "v1", "json_v1", "json_v2"];

    pub fn get(&self, name: &str) -> f64 {
        match name {
            "expand" => self.expand_ms,
            "parse" => self.parse_ms,
            "adapt" => self.adapt_ms,
            "typeset" => self.typeset_ms,
            "typeset_notexts" => self.typeset_notexts_ms,
            "assemble" => self.assemble_ms,
            "v1" => self.v1_ms,
            "json_v1" => self.json_v1_ms,
            "json_v2" => self.json_v2_ms,
            _ => f64::NAN,
        }
    }
}

#[derive(Clone)]
pub struct ScenarioSample {
    pub name: String,
    pub caps: String,
    /// One entry per typed character. Empty when the scenario was skipped.
    pub steps_ms: Vec<f64>,
    pub digest: String,
    pub skipped: Option<String>,
}

/// Everything one child process produces.
#[derive(Clone)]
pub struct Sample {
    pub case_id: String,
    pub bytes: usize,
    pub documents: usize,
    pub pages: usize,
    pub passes: u32,
    /// Building the `FontSet` (directory discovery; faces load lazily).
    pub fontset_ms: f64,
    /// The first render in the process: cold caches *and* cold fonts. What
    /// opening a file in a freshly launched engine costs.
    pub first_render_ms: f64,
    /// A render with an empty `RenderCache` but faces already loaded. What a
    /// full recompile costs in a running engine.
    pub cold_render_ms: f64,
    /// The same render with no cache attached at all.
    pub cold_nocache_ms: f64,
    /// `write_pdf_exact` on the cold display list: the app's Export PDF,
    /// including the one-time font-program reads and subsetting.
    pub export_pdf_ms: f64,
    /// A second export of the same display list, with the font programs
    /// already read: the marginal cost of an export.
    pub export_pdf_warm_ms: f64,
    pub export_pdf_bytes: usize,
    pub phases: Phases,
    pub warm: Vec<ScenarioSample>,
    pub peak_rss_kb: Option<u64>,
    pub steady_rss_kb: Option<u64>,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub digest_reply: String,
    pub digest_pdf: String,
    pub load_before: Option<(f64, f64, f64)>,
    pub load_after: Option<(f64, f64, f64)>,
    pub notes: Vec<String>,
}

pub struct RunConfig {
    pub fonts: FontConfig,
    pub steps: usize,
    /// Above this page count the PDF export is not attempted.
    pub export_max_pages: usize,
    pub caps: Vec<Caps>,
    pub scenarios: Vec<String>,
    pub export: bool,
}

fn hex16(bytes: &[u8]) -> String {
    sha256::digest(bytes).iter().take(8).map(|b| format!("{b:02x}")).collect()
}

fn request(revision: usize, docs: &[(String, String)], entry: &str, caps: Caps) -> String {
    let mut arr = Vec::with_capacity(docs.len());
    for (path, text) in docs {
        let mut d = Value::obj();
        d.set("path", json::str_(path.as_str()));
        d.set("text", json::str_(text.as_str()));
        arr.push(d);
    }
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("perfbench"));
    payload.set("revision", json::num(revision as f64));
    payload.set("entry_path", json::str_(entry));
    payload.set("documents", Value::Arr(arr));
    let mut c = vec![json::str_("rules-v1"), json::str_("font-hints-v1")];
    if caps == Caps::V2 {
        c.push(json::str_("display-list-v2"));
    }
    payload.set("layout_capabilities", Value::Arr(c));
    let mut v = Value::obj();
    v.set("protocol_version", json::num(1.0));
    v.set("id", json::str_(&format!("r{revision}")));
    v.set("type", json::str_("compile"));
    v.set("payload", payload);
    json::write(&v)
}

fn reply_bytes(reply: protocol::Reply) -> Vec<u8> {
    let mut bytes = reply.line.into_bytes();
    for extra in reply.extra_lines {
        bytes.push(b'\n');
        bytes.extend_from_slice(extra.as_bytes());
    }
    bytes
}

/// Where the body starts, so an anchor is never placed in the preamble.
fn body_start(text: &str) -> usize {
    text.find("\\begin{document}").map_or(0, |i| i + "\\begin{document}".len())
}

/// A line of ordinary prose: long enough to reflow, with no command in it, so
/// typing into it cannot change what the document means.
fn prose_line(text: &str) -> Option<(usize, usize)> {
    let from = body_start(text);
    let mut at = from;
    for line in text[from..].split_inclusive('\n') {
        let start = at;
        at += line.len();
        let t = line.trim_end();
        if t.len() >= 60 && !t.contains('\\') && !t.contains('$') && !t.contains('%') && t.starts_with(|c: char| c.is_ascii_alphabetic()) {
            return Some((start, at));
        }
    }
    None
}

/// Just inside the first `$...$`. Typing letters there stays valid maths.
fn inline_math_offset(text: &str) -> Option<usize> {
    let from = body_start(text);
    let bytes = text.as_bytes();
    let mut i = from;
    while i < bytes.len() {
        if bytes[i] == b'$' && (i == 0 || bytes[i - 1] != b'\\') {
            if bytes.get(i + 1) == Some(&b'$') {
                // Display maths; skip past the whole `$$...$$`.
                i = text[i + 2..].find("$$").map_or(bytes.len(), |j| i + 2 + j + 2);
                continue;
            }
            let close = text[i + 1..].find('$')?;
            if close > 1 {
                return Some(i + 1);
            }
            i += 1;
        }
        i += 1;
    }
    None
}

/// Just inside the first display equation, whichever spelling opens it.
fn display_math_offset(text: &str) -> Option<usize> {
    let from = body_start(text);
    let hay = &text[from..];
    let mut best: Option<usize> = None;
    for open in ["$$", "\\begin{equation}", "\\begin{align}", "\\begin{gather}", "\\begin{equation*}", "\\begin{align*}", "\\["] {
        if let Some(i) = hay.find(open) {
            let at = from + i + open.len();
            best = Some(best.map_or(at, |b: usize| b.min(at)));
        }
    }
    best
}

/// The nearest character boundary at or before `at`. `unicode-accents` has
/// multi-byte characters in its first prose line, and inserting into the
/// middle of one panics.
fn boundary(text: &str, at: usize) -> usize {
    let mut i = at.min(text.len());
    while i > 0 && !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

#[cfg(test)]
pub fn boundary_for_tests(text: &str, at: usize) -> usize {
    boundary(text, at)
}

fn edits_for(case: &Case, wanted: &[String]) -> Vec<(String, Edit)> {
    let text = case.entry_text();
    let line = prose_line(text);
    let mut out = Vec::new();
    for name in wanted {
        let edit = match name.as_str() {
            // Ten bytes into the prose line: past any leading capital, well
            // inside the line so the whole paragraph must reflow.
            "type-paragraph" => line.map(|(s, e)| Edit::Type { offset: boundary(text, (s + 10).min(e.saturating_sub(1))) }),
            "type-inline-math" => inline_math_offset(text).map(|offset| Edit::Type { offset }),
            "type-display-eq" => display_math_offset(text).map(|offset| Edit::Type { offset }),
            "delete-restore-line" => line.map(|(start, end)| Edit::DeleteLine { start, end }),
            _ => None,
        };
        if let Some(edit) = edit {
            out.push((name.clone(), edit));
        }
    }
    out
}

fn error_count(diagnostics: &[flashtex_render_pipeline::display::Diagnostic]) -> usize {
    diagnostics.iter().filter(|d| d.severity == flashtex_render_pipeline::display::Severity::Error).count()
}

fn text_at(base: &str, edit: Edit, step: usize) -> String {
    match edit {
        Edit::Type { offset } => {
            let typed: String = (0..step).map(|i| TYPED[i % TYPED.len()] as char).collect();
            let mut t = base.to_string();
            t.insert_str(offset, &typed);
            t
        }
        Edit::DeleteLine { start, end } => {
            let mut t = base.to_string();
            if step % 2 == 1 {
                t.replace_range(start..end, "");
            }
            t
        }
    }
}

/// Runs one repetition.
///
/// `Err` means this case produced no valid measurement — the reason travels
/// back to the parent, which records the case as not measured and carries on
/// with the rest of the corpus. What it never does is turn into a timing: a
/// measurement that could not be trusted is not downgraded to a warning.
pub fn run(case: &Case, cfg: &RunConfig) -> Result<Sample, String> {
    let load_before = sys::load_avg();
    let mut notes = Vec::new();

    let t = Instant::now();
    let fonts = cfg.fonts.build();
    let fontset_ms = ms(t);
    fontgate::preflight(&fonts)?;

    let options = RenderOptions { project_root: case.project_root.clone(), ..RenderOptions::default() };
    let owned: Vec<(String, String)> = case.docs.iter().map(|d| (d.path.clone(), d.text.clone())).collect();

    // (1) First render in the process: cold caches and cold faces.
    let line = request(1, &owned, &case.entry, Caps::V2);
    let cache = RenderCache::new();
    let t = Instant::now();
    let reply = protocol::handle_line(&line, &fonts, &options, Some(&cache));
    let first_render_ms = ms(t);
    let digest_reply = hex16(&reply_bytes(reply));

    // (2) Cold caches, warm faces: a full recompile in a running engine.
    let docs: Vec<SourceDocument<'_>> = owned.iter().map(|(p, t)| SourceDocument { path: p, text: t }).collect();
    let cache = RenderCache::new();
    let t = Instant::now();
    let rendered = flashtex_render_pipeline::render_cached(&docs, &case.entry, 2, "perfbench", &fonts, &options, Some(&cache));
    let cold_render_ms = ms(t);
    // The populated cache is the largest thing in the process (gigabytes on
    // the 2 MB document). Nothing after this point reads it, and leaving it
    // alive would make every later measurement pay for its memory pressure.
    drop(cache);

    // (2b) The same render with no cache attached. On a small document the
    // two are within noise; where they diverge, the cost of *populating* the
    // reuse cache is what separates them, and that cost is paid on the very
    // first compile, before any edit can benefit from it.
    let t = Instant::now();
    let fresh = flashtex_render_pipeline::render_cached(&docs, &case.entry, 2, "perfbench", &fonts, &options, None);
    let cold_nocache_ms = ms(t);
    if fresh.v2.pages.len() != rendered.v2.pages.len() {
        return Err(format!("{}: cached and uncached renders disagree on the page count", case.id));
    }
    drop(fresh);
    // Substituted faces or metrics: fatal, this is not the document we meant
    // to measure. Everything else the render complained about — a character
    // with no glyph, a command the compiler does not model — is deterministic,
    // identical every run, and therefore still worth timing. It is recorded on
    // the case so the number is never read as a complete document.
    fontgate::check_diagnostics(&rendered.v2.diagnostics)?;
    notes.extend(fontgate::degradations(&rendered.v2.diagnostics));
    let base_errors = error_count(&rendered.v2.diagnostics);
    let pages = rendered.v2.pages.len();
    let passes = rendered.passes;

    // (3) Export the display list we just built, through the app's own route.
    //
    // Twice: the first export reads and subsets the font programs from disk,
    // the second does not. A single number would hide which half a change
    // moved, and the first-export figure is the one the user waits for.
    let skip = |why: String| (f64::NAN, f64::NAN, 0, String::new(), Some(why));
    let (export_pdf_ms, export_pdf_warm_ms, export_pdf_bytes, digest_pdf, export_note) = if !cfg.export {
        skip("export not requested".into())
    } else if pages > cfg.export_max_pages {
        // At 1507 pages the display-list envelope passes 256 MiB and the pdf
        // crate refuses it. Attempting it anyway costs about a minute per
        // repetition to reach the same failure, so the limit is declared
        // rather than rediscovered; raise it with --export-max-pages to
        // reproduce the refusal.
        skip(format!("export skipped: {pages} pages is over --export-max-pages {}", cfg.export_max_pages))
    } else {
        let t = Instant::now();
        match pdf::write_pdf_exact(&rendered.v2, fonts.dirs(), case.project_root.as_deref()) {
            Ok(out) => {
                let cold = ms(t);
                let t = Instant::now();
                let again = pdf::write_pdf_exact(&rendered.v2, fonts.dirs(), case.project_root.as_deref())
                    .map_err(|e| format!("PDF re-export failed for {}: {e}", case.id))?;
                let warm = ms(t);
                if again.bytes != out.bytes {
                    return Err(format!("{}: two exports of one display list produced different bytes", case.id));
                }
                (cold, warm, out.bytes.len(), hex16(&out.bytes), None)
            }
            // An export the engine cannot perform is a product limitation to
            // record, not a broken measurement: the render timings this run
            // produced are still valid, so the run continues.
            Err(e) => skip(format!("export failed: {e}")),
        }
    };
    if let Some(n) = export_note {
        notes.push(n);
    }

    // (4) Phase decomposition, its own empty cache.
    let phases = decompose(&docs, case, &fonts, &options);

    // (5) Warm keystrokes.
    let base = case.entry_text().to_string();
    let entry_index = owned.iter().position(|(p, _)| *p == case.entry).unwrap_or(0);
    let mut warm = Vec::new();
    let mut cache_hits = 0;
    let mut cache_misses = 0;
    for (name, edit) in edits_for(case, &cfg.scenarios) {
        for &caps in &cfg.caps {
            let cache = RenderCache::new();
            let mut docs_now = owned.clone();
            // Prime: the unedited document, not timed and not hashed.
            docs_now[entry_index].1 = base.clone();
            let _ = protocol::handle_line(&request(0, &docs_now, &case.entry, caps), &fonts, &options, Some(&cache));
            // Validate the edit before timing anything: typing into the wrong
            // place turns the scenario into a measurement of error recovery,
            // which is fast for the wrong reason.
            //
            // The bar is "no worse than the unedited document", not "clean".
            // Several fixtures use class commands the compiler does not model
            // yet and already carry errors; demanding zero would have thrown
            // away every warm number for `cv`, `letter` and `math-sheet` for a
            // property of the corpus rather than of the edit.
            let mut skipped = None;
            {
                let edited = text_at(&base, edit, 1);
                let mut probe = owned.clone();
                probe[entry_index].1 = edited;
                let pd: Vec<SourceDocument<'_>> = probe.iter().map(|(p, t)| SourceDocument { path: p, text: t }).collect();
                let r = flashtex_render_pipeline::render_cached(&pd, &case.entry, 1, "perfbench", &fonts, &options, None);
                let after = error_count(&r.v2.diagnostics);
                if after > base_errors {
                    skipped = Some(format!("the edit adds {} error diagnostic(s) to the document's own {base_errors}", after - base_errors));
                }
                if let Err(why) = fontgate::check_diagnostics(&r.v2.diagnostics) {
                    skipped = Some(why.replace('\n', " "));
                }
            }
            let mut steps_ms = Vec::with_capacity(cfg.steps);
            let mut hashes = Vec::new();
            for step in 1..=cfg.steps {
                if skipped.is_some() {
                    break;
                }
                docs_now[entry_index].1 = text_at(&base, edit, step);
                let line = request(step + 2, &docs_now, &case.entry, caps);
                let t = Instant::now();
                let reply = protocol::handle_line(&line, &fonts, &options, Some(&cache));
                steps_ms.push(ms(t));
                hashes.extend_from_slice(&sha256::digest(&reply_bytes(reply)));
            }
            let (h, m) = cache.stats();
            cache_hits += h;
            cache_misses += m;
            warm.push(ScenarioSample {
                name: name.clone(),
                caps: caps.as_str().to_string(),
                digest: hex16(&hashes),
                steps_ms,
                skipped,
            });
        }
    }
    for name in &cfg.scenarios {
        if !warm.iter().any(|w| w.name == *name) {
            notes.push(format!("{name}: no anchor in this document"));
        }
    }

    Ok(Sample {
        case_id: case.id.clone(),
        bytes: case.bytes(),
        documents: case.docs.len(),
        pages,
        passes,
        fontset_ms,
        first_render_ms,
        cold_render_ms,
        cold_nocache_ms,
        export_pdf_ms,
        export_pdf_warm_ms,
        export_pdf_bytes,
        phases,
        warm,
        peak_rss_kb: sys::peak_rss_kb(),
        steady_rss_kb: sys::current_rss_kb(),
        cache_hits,
        cache_misses,
        digest_reply,
        digest_pdf,
        load_before,
        load_after: sys::load_avg(),
        notes,
    })
}

fn decompose(docs: &[SourceDocument<'_>], case: &Case, fonts: &FontSet, options: &RenderOptions) -> Phases {
    let entry_index = docs.iter().position(|d| d.path == case.entry).unwrap_or(0);
    let cache = RenderCache::new();
    let texts: Vec<&str> = docs.iter().map(|d| d.text).collect();
    let paths: Vec<&str> = docs.iter().map(|d| d.path).collect();

    // Parse first, so the phase that is reported is the one that ran cold.
    // The standalone expansion afterwards is a share-of-parse indicator, not
    // a separate stage of the pipeline.
    let t = Instant::now();
    let parsed = flashtex_compiler::parser::parse_project(docs, &case.entry);
    let parse_ms = ms(t);

    let t = Instant::now();
    let expanded = flashtex_compiler::expansion::expand_project(docs, entry_index);
    let expand_ms = ms(t);
    std::hint::black_box(&expanded);

    let t = Instant::now();
    let labels = adapter::Labels::from_parsed(&parsed);
    let doc = adapter::adapt_cached(&texts, entry_index, &parsed, options, &labels, Some(&cache));
    let adapt_ms = ms(t);

    let t = Instant::now();
    let mut ctx = typeset::Context::with_texts(fonts, &doc.style, &paths, &texts);
    let laid = typeset::build(&mut ctx, &doc, Some(&cache));
    let typeset_ms = ms(t);

    // Control: the same typesetting with `Context::new`, i.e. without the
    // document sources.
    //
    // `render_cached` always uses `with_texts` — the sources are what lets
    // `\left`/`\right` fences and `\mathbin`-style class overrides be
    // recovered from the bytes at each delimiter. `examples/stages.rs` uses
    // `Context::new`, so its per-stage numbers describe a cheaper pipeline
    // than the one the worker runs, and the two must not be compared.
    // Keeping the control here makes that difference a measured quantity
    // instead of a footnote, and it is the first place to look when the
    // typeset phase moves.
    //
    // Not run for documents containing a picture: with no sources, the TikZ
    // path slices the empty string by the picture's byte range and panics
    // (`vector-graphics/src/tikz/mod.rs`, `render`). That is a latent hole in
    // `Context::new` rather than a product-path fault — `render_cached` always
    // passes the sources — but the control must not be the thing that brings
    // the harness down.
    let has_picture = texts.iter().any(|t| t.contains("\\begin{tikzpicture}"));
    let typeset_notexts_ms = if has_picture {
        f64::NAN
    } else {
        let cache2 = RenderCache::new();
        let t = Instant::now();
        let mut ctx2 = typeset::Context::new(fonts, &doc.style, &paths);
        let laid2 = typeset::build(&mut ctx2, &doc, Some(&cache2));
        let elapsed = ms(t);
        std::hint::black_box(&laid2);
        elapsed
    };

    let diagnostics = ctx.take_diagnostics();
    let t = Instant::now();
    // `page_color`/`default_color` (added after the harness was written) are
    // `None` here: the harness measures the default-colour path, which is what
    // every committed baseline was recorded against.
    let v2 = typeset::assemble(
        "perfbench",
        3,
        docs,
        &doc.style,
        fonts,
        laid,
        diagnostics,
        Some(&cache),
        None,
        None,
    );
    let assemble_ms = ms(t);

    let t = Instant::now();
    let payload = v1::fallback(&v2, Capabilities {
            rules: true,
            font_hints: true,
            display_list: false,
            images: false,
            // Capabilities gained fields after the harness was written. Spreading
            // the default keeps new ones off, so the measured path stays the one
            // the committed baselines were recorded against.
            ..Default::default()
        }, Some(vec![]));
    let v1_ms = ms(t);

    let t = Instant::now();
    let envelope = payload.write_envelope("perfbench");
    let json_v1_ms = ms(t);
    std::hint::black_box(&envelope);

    let t = Instant::now();
    let v2_json = v2.write_json_with("perfbench", true);
    let json_v2_ms = ms(t);
    std::hint::black_box(&v2_json);

    Phases {
        expand_ms,
        parse_ms,
        adapt_ms,
        typeset_ms,
        typeset_notexts_ms,
        assemble_ms,
        v1_ms,
        json_v1_ms,
        json_v2_ms,
    }
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

// ---------------------------------------------------------------------------
// Child <-> parent transport. One JSON object on one line, so the parent can
// read it without a serialisation dependency and a human can read it too.
// ---------------------------------------------------------------------------

impl Sample {
    pub fn to_json(&self) -> Value {
        let mut v = Value::obj();
        v.set("case_id", json::str_(self.case_id.as_str()));
        v.set("bytes", json::num(self.bytes as f64));
        v.set("documents", json::num(self.documents as f64));
        v.set("pages", json::num(self.pages as f64));
        v.set("passes", json::num(self.passes as f64));
        v.set("fontset_ms", json::num(self.fontset_ms));
        v.set("first_render_ms", json::num(self.first_render_ms));
        v.set("cold_render_ms", json::num(self.cold_render_ms));
        v.set("cold_nocache_ms", json::num(self.cold_nocache_ms));
        v.set("export_pdf_ms", json::num(self.export_pdf_ms));
        v.set("export_pdf_warm_ms", json::num(self.export_pdf_warm_ms));
        v.set("export_pdf_bytes", json::num(self.export_pdf_bytes as f64));
        let mut p = Value::obj();
        for n in Phases::NAMES {
            p.set(n, json::num(self.phases.get(n)));
        }
        v.set("phases", p);
        let warm: Vec<Value> = self
            .warm
            .iter()
            .map(|w| {
                let mut o = Value::obj();
                o.set("name", json::str_(w.name.as_str()));
                o.set("caps", json::str_(w.caps.as_str()));
                o.set("digest", json::str_(w.digest.as_str()));
                o.set("steps_ms", Value::Arr(w.steps_ms.iter().map(|x| json::num(*x)).collect()));
                if let Some(s) = &w.skipped {
                    o.set("skipped", json::str_(s.as_str()));
                }
                o
            })
            .collect();
        v.set("warm", Value::Arr(warm));
        v.set("peak_rss_kb", self.peak_rss_kb.map_or(Value::Null, |x| json::num(x as f64)));
        v.set("steady_rss_kb", self.steady_rss_kb.map_or(Value::Null, |x| json::num(x as f64)));
        v.set("cache_hits", json::num(self.cache_hits as f64));
        v.set("cache_misses", json::num(self.cache_misses as f64));
        v.set("digest_reply", json::str_(self.digest_reply.as_str()));
        v.set("digest_pdf", json::str_(self.digest_pdf.as_str()));
        v.set("load_before", load_json(self.load_before));
        v.set("load_after", load_json(self.load_after));
        v.set("notes", Value::Arr(self.notes.iter().map(|n| json::str_(n.as_str())).collect()));
        v
    }

    pub fn from_json(v: &Value) -> Option<Sample> {
        let num = |o: &Value, k: &str| match o.get(k) {
            Some(Value::Num(n)) => Some(*n),
            _ => None,
        };
        let phases_v = v.get("phases")?;
        let ph = |k: &str| num(phases_v, k).unwrap_or(f64::NAN);
        let warm = v
            .get("warm")?
            .as_arr()?
            .iter()
            .map(|w| ScenarioSample {
                name: w.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
                caps: w.get("caps").and_then(Value::as_str).unwrap_or("").to_string(),
                digest: w.get("digest").and_then(Value::as_str).unwrap_or("").to_string(),
                steps_ms: w
                    .get("steps_ms")
                    .and_then(Value::as_arr)
                    .map(|a| a.iter().filter_map(|x| match x { Value::Num(n) => Some(*n), _ => None }).collect())
                    .unwrap_or_default(),
                skipped: w.get("skipped").and_then(Value::as_str).map(str::to_string),
            })
            .collect();
        Some(Sample {
            case_id: v.get("case_id")?.as_str()?.to_string(),
            bytes: num(v, "bytes")? as usize,
            documents: num(v, "documents")? as usize,
            pages: num(v, "pages")? as usize,
            passes: num(v, "passes")? as u32,
            fontset_ms: num(v, "fontset_ms")?,
            first_render_ms: num(v, "first_render_ms")?,
            cold_render_ms: num(v, "cold_render_ms")?,
            cold_nocache_ms: num(v, "cold_nocache_ms").unwrap_or(f64::NAN),
            export_pdf_ms: num(v, "export_pdf_ms").unwrap_or(f64::NAN),
            export_pdf_warm_ms: num(v, "export_pdf_warm_ms").unwrap_or(f64::NAN),
            export_pdf_bytes: num(v, "export_pdf_bytes").unwrap_or(0.0) as usize,
            phases: Phases {
                expand_ms: ph("expand"),
                parse_ms: ph("parse"),
                adapt_ms: ph("adapt"),
                typeset_ms: ph("typeset"),
                typeset_notexts_ms: ph("typeset_notexts"),
                assemble_ms: ph("assemble"),
                v1_ms: ph("v1"),
                json_v1_ms: ph("json_v1"),
                json_v2_ms: ph("json_v2"),
            },
            warm,
            peak_rss_kb: num(v, "peak_rss_kb").map(|x| x as u64),
            steady_rss_kb: num(v, "steady_rss_kb").map(|x| x as u64),
            cache_hits: num(v, "cache_hits").unwrap_or(0.0) as u64,
            cache_misses: num(v, "cache_misses").unwrap_or(0.0) as u64,
            digest_reply: v.get("digest_reply").and_then(Value::as_str).unwrap_or("").to_string(),
            digest_pdf: v.get("digest_pdf").and_then(Value::as_str).unwrap_or("").to_string(),
            load_before: load_from(v.get("load_before")),
            load_after: load_from(v.get("load_after")),
            notes: v
                .get("notes")
                .and_then(Value::as_arr)
                .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
                .unwrap_or_default(),
        })
    }
}

fn load_json(l: Option<(f64, f64, f64)>) -> Value {
    match l {
        Some((a, b, c)) => Value::Arr(vec![json::num(a), json::num(b), json::num(c)]),
        None => Value::Null,
    }
}

fn load_from(v: Option<&Value>) -> Option<(f64, f64, f64)> {
    let a = v?.as_arr()?;
    let n = |i: usize| match a.get(i) {
        Some(Value::Num(x)) => Some(*x),
        _ => None,
    };
    Some((n(0)?, n(1)?, n(2)?))
}
