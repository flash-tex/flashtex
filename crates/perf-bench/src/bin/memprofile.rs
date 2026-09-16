//! FT-070 memory profile: where the resident bytes are, per structure.
//!
//! Renders one corpus case through the same product path `measure.rs` uses,
//! then reports three numbers that have to be read together:
//!
//! - **VmRSS** — what the kernel says the process holds.
//! - **live** — bytes the program asked the allocator for and has not freed
//!   (a counting global allocator wraps the system one).
//! - **walked** — the deep size of the structures still reachable, split by
//!   structure. `live - walked` is what the walk does not reach (font caches,
//!   the expander's thread-local caches, the compiler's own arenas);
//!   `RSS - live` is allocator retention and fragmentation.
//!
//! Usage: `memprofile <case-id> [--no-warm] [--window FIRST:COUNT] [--render-only]`.
//!
//! `--window` materialises only that page range
//! (`protocol/proposals/display-list-v2-window.md`). The capability is
//! negotiated, so a windowed run drives the protocol path like any other;
//! `--render-only` skips the protocol on both sides. Compare like with like:
//! the protocol path also builds the v1 payload and the JSON line, and those
//! bytes are not the window's to remove, so a windowed `--render-only` run is
//! compared against an unwindowed `--render-only` run.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

use flashtex_perf_bench::{corpus, fontgate, sys};
use flashtex_render_pipeline::memsize;
use flashtex_render_pipeline::{protocol, PageWindow, RenderCache, RenderOptions};

struct Counting;

static ALLOCATED: AtomicU64 = AtomicU64::new(0);
static FREED: AtomicU64 = AtomicU64::new(0);
static PEAK_LIVE: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = System.alloc(l);
        if !p.is_null() {
            let a = ALLOCATED.fetch_add(l.size() as u64, Ordering::Relaxed) + l.size() as u64;
            let live = a.saturating_sub(FREED.load(Ordering::Relaxed));
            PEAK_LIVE.fetch_max(live, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        FREED.fetch_add(l.size() as u64, Ordering::Relaxed);
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        let q = System.realloc(p, l, new);
        if !q.is_null() {
            FREED.fetch_add(l.size() as u64, Ordering::Relaxed);
            ALLOCATED.fetch_add(new as u64, Ordering::Relaxed);
        }
        q
    }
}

#[global_allocator]
static A: Counting = Counting;

fn live() -> u64 {
    ALLOCATED.load(Ordering::Relaxed).saturating_sub(FREED.load(Ordering::Relaxed))
}

fn mib(b: u64) -> f64 {
    b as f64 / (1024.0 * 1024.0)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let want = args.first().cloned().unwrap_or_else(|| "synthetic-2mb".into());
    let warm = !args.iter().any(|a| a == "--no-warm");
    let window = args.iter().position(|a| a == "--window").and_then(|i| args.get(i + 1)).map(|spec| {
        let (f, c) = spec.split_once(':').expect("--window FIRST:COUNT");
        PageWindow { first_page: f.parse().expect("FIRST"), page_count: c.parse().expect("COUNT") }
    });
    // `--window` used to imply `--render-only` because the capability was not
    // negotiated; it is now, so a windowed protocol run is a real measurement
    // of the product path. Comparing a window against an unwindowed control
    // still has to be done on the SAME path: the protocol path also builds the
    // v1 payload and the JSON line, and those bytes are not the window's to
    // remove.
    let render_only = args.iter().any(|a| a == "--render-only");
    // `--scroll N`: after the warm keystrokes, move the window across the
    // document in N steps through the SAME cache -- a viewer scrolling. This
    // is what proves the window is bounded over a session rather than only at
    // the first render.
    let scroll: u32 = args
        .iter()
        .position(|a| a == "--scroll")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    // crates/perf-bench -> crates -> repo root, as `main.rs` resolves it.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root");
    let cases = corpus::all_cases(&root);
    let case = cases.iter().find(|c| c.id == want).unwrap_or_else(|| {
        eprintln!("no case {want}; have: {}", cases.iter().map(|c| c.id.as_str()).collect::<Vec<_>>().join(", "));
        std::process::exit(2)
    });

    let font_config = fontgate::resolve(&root, None, None);
    let fonts = font_config.build();
    fontgate::preflight(&fonts).expect("font preflight");
    let options = RenderOptions::default();

    let owned: Vec<(String, String)> = case.docs.iter().map(|d| (d.path.clone(), d.text.clone())).collect();
    let docs: Vec<flashtex_compiler::parser::SourceDocument<'_>> = owned
        .iter()
        .map(|(p, t)| flashtex_compiler::parser::SourceDocument { path: p, text: t })
        .collect();

    let before = live();
    let cache = RenderCache::new();
    let rendered =
        flashtex_render_pipeline::render_windowed(&docs, &case.entry, 1, "memprofile", &fonts, &options, Some(&cache), window);
    fontgate::check_diagnostics(&rendered.v2.diagnostics).expect("zero font diagnostics");
    let after_render = live();

    // The harness samples steady RSS after the warm keystroke scenarios, so
    // reproduce them: they are what populates the cache to its real size.
    if warm {
        let mut docs_now = owned.clone();
        let entry_index = owned.iter().position(|(p, _)| *p == case.entry).unwrap_or(0);
        let base = docs_now[entry_index].1.clone();
        for step in 0..4 {
            docs_now[entry_index].1 = format!("{base}\n% keystroke {step}\n");
            if render_only {
                let srcs: Vec<flashtex_compiler::parser::SourceDocument<'_>> =
                    docs_now.iter().map(|(p, t)| flashtex_compiler::parser::SourceDocument { path: p, text: t }).collect();
                let r = flashtex_render_pipeline::render_windowed(
                    &srcs, &case.entry, step + 2, "memprofile", &fonts, &options, Some(&cache), window,
                );
                std::hint::black_box(&r.v2.pages.len());
            } else {
                let line = request(step + 2, &docs_now, &case.entry, window);
                let _ = protocol::handle_line(&line, &fonts, &options, Some(&cache));
            }
        }
    }

    if scroll > 0 {
        let pages = rendered.v2.pages.len() as u32;
        let w = window.expect("--scroll needs --window");
        let srcs: Vec<flashtex_compiler::parser::SourceDocument<'_>> =
            owned.iter().map(|(p, t)| flashtex_compiler::parser::SourceDocument { path: p, text: t }).collect();
        for step in 1..=scroll {
            let first = 1 + (pages.saturating_sub(w.page_count) * step) / scroll.max(1);
            let r = flashtex_render_pipeline::render_windowed(
                &srcs,
                &case.entry,
                1,
                "memprofile",
                &fonts,
                &options,
                Some(&cache),
                Some(PageWindow { first_page: first, page_count: w.page_count }),
            );
            std::hint::black_box(&r.v2.pages.len());
        }
        println!("(scrolled the window across the document in {scroll} steps through one cache)");
    }

    let mut audit = memsize::CaretAudit::default();
    memsize::audit_carets(&rendered.v2, &mut audit);

    let rss = sys::current_rss_kb().unwrap_or(0) * 1024;
    let peak = sys::peak_rss_kb().unwrap_or(0) * 1024;
    let live_now = live();

    let mut r = memsize::Report::new();
    memsize::display_list(&rendered.v2, &mut r);
    let dl_total = r.total();
    memsize::render_cache(&cache, &mut r);
    let walked = r.total();

    println!("case            {} ({} bytes, {} pages)", case.id, case.bytes(), rendered.v2.pages.len());
    match rendered.v2.window {
        Some(w) => println!(
            "window          pages {}-{} of {} resident ({} elided); warm path: {}",
            w.first_page,
            w.first_page + w.page_count - 1,
            rendered.v2.pages.len(),
            rendered.v2.pages.len() as u32 - w.page_count,
            if render_only { "render-only" } else { "protocol" }
        ),
        None => println!("window          none (complete compile); warm path: {}", if render_only { "render-only" } else { "protocol" }),
    }
    println!("VmRSS           {:>10.1} MiB", mib(rss));
    println!("VmHWM (peak)    {:>10.1} MiB", mib(peak));
    println!("live (alloc)    {:>10.1} MiB", mib(live_now));
    println!("peak live       {:>10.1} MiB", mib(PEAK_LIVE.load(Ordering::Relaxed)));
    println!("walked          {:>10.1} MiB   ({:.1}% of live)", mib(walked), 100.0 * walked as f64 / live_now.max(1) as f64);
    println!("  display list  {:>10.1} MiB", mib(dl_total));
    println!("  render cache  {:>10.1} MiB", mib(walked - dl_total));
    println!("unreached       {:>10.1} MiB   (live - walked: font/expander/compiler caches)", mib(live_now.saturating_sub(walked)));
    println!("allocator held  {:>10.1} MiB   (RSS - live: retention + fragmentation)", mib(rss.saturating_sub(live_now)));
    println!("render delta    {:>10.1} MiB   (live after render - before)", mib(after_render.saturating_sub(before)));
    println!();
    println!("{}", r.table());
    println!("struct sizes (bytes):");
    for (n, s) in memsize::struct_sizes() {
        println!("  {n:<14} {s}");
    }
    let glyphs = r.count_of("GlyphRun.glyphs");
    let clusters = r.count_of("Cluster.carets (per glyph)");
    println!();
    println!("display list: {glyphs} glyphs, {clusters} clusters, {} runs", r.count_of("GlyphRun count (header in Item slot)"));
    if clusters > 0 {
        println!("per cluster in the display list: {:.1} bytes", dl_total as f64 / clusters as f64);
    }
    println!("per source byte (RSS): {:.0} bytes", rss as f64 / case.bytes().max(1) as f64);
    println!(
        "carets: {} clusters in {} runs; {} runs carry an end caret (one caret per cluster plus one per such run)",
        audit.clusters, audit.runs, audit.runs_with_end_caret
    );

    // Is the gap between RSS and live bytes the allocator holding freed
    // memory, or is it something the walk cannot see? `malloc_trim` asks
    // glibc to return free arena pages to the kernel; if RSS falls by the
    // gap, the gap was retention and nothing in the program owns it.
    extern "C" {
        fn malloc_trim(pad: usize) -> i32;
    }
    let rc = unsafe { malloc_trim(0) };
    let after = sys::current_rss_kb().unwrap_or(0) * 1024;
    println!();
    println!("malloc_trim(0) -> {rc}");
    println!("VmRSS after trim {:>9.1} MiB   (released {:.1} MiB)", mib(after), mib(rss.saturating_sub(after)));
    println!("live after trim  {:>9.1} MiB", mib(live()));
}

/// A runtime-v1 `compile` line.
///
/// This used to build a JSON-RPC 2.0 envelope (`jsonrpc`/`method`/`params`),
/// which `protocol::handle_line` answers with `missing_protocol_version` —
/// so every warm keystroke on the protocol path was an error envelope and
/// rendered nothing. The `--render-only` runs the window was measured with
/// were unaffected (they never build a request), but a default run's warm
/// phase was measuring an empty cache.
fn request(rev: u64, docs: &[(String, String)], entry: &str, window: Option<PageWindow>) -> String {
    let mut o = String::from("{\"protocol_version\":1,\"id\":\"memprofile\",\"type\":\"compile\",\"payload\":{\"project_id\":\"memprofile\",\"revision\":");
    o.push_str(&rev.to_string());
    o.push_str(",\"entry_path\":");
    push_json_str(&mut o, entry);
    o.push_str(",\"documents\":[");
    for (i, (p, t)) in docs.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        o.push_str("{\"path\":");
        push_json_str(&mut o, p);
        o.push_str(",\"text\":");
        push_json_str(&mut o, t);
        o.push('}');
    }
    o.push_str("],\"layout_capabilities\":[\"display-list-v2\"");
    if let Some(w) = window {
        // The window is on the wire now (`display-list-v2-window`), so the
        // protocol path can be measured windowed — which is what the product
        // actually does for a long document. `-only` goes with it: at scale
        // neither lever is sufficient alone (proposal §7).
        o.push_str(",\"display-list-v2-only\",\"display-list-v2-window\"],\"display_list_window\":{\"first_page\":");
        o.push_str(&w.first_page.to_string());
        o.push_str(",\"page_count\":");
        o.push_str(&w.page_count.to_string());
        o.push_str("}}}");
    } else {
        o.push_str("]}}");
    }
    o
}

fn push_json_str(o: &mut String, s: &str) {
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
}
