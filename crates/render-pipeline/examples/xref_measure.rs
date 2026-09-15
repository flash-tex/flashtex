//! Cross-reference invalidation through `flashtex-render` (step 0 of the
//! incremental cross-references design, PR #575). Measurement only.
//!
//! Renders #535's deterministic ~300-page article through the worker's real
//! request path (`protocol::handle_line` with a `RenderCache` that outlives
//! requests), then applies one warm edit and reports, for that edit only:
//! `adapted` misses, `blocks` misses, `assembled` misses, layout passes
//! (1 = no page-number relayout) and latency, median of N runs. Every run
//! starts from a fresh cache warmed with the unedited document, so each run
//! measures the same transition.
//!
//! Documents:
//! - `535`: the bytes of #535's `large_document(450)` (labels, `\ref`,
//!   `\eqref`, `\cite`, `thebibliography`; no `\pageref`, no contents list).
//! - `535+toc+pageref`: the same, plus `\tableofcontents` after `\maketitle`
//!   and one `\pageref{sec:N}` in every section, so page numbers are needed.
//!
//! Edits:
//! - `no-label`: one character typed mid-document (#535's edit).
//! - `renumber`: a new `\section` with a `\label` inserted before section 1,
//!   so every later section number (and every `\ref{sec:N}`) changes.
//! - `move-label`: one paragraph with no label is moved across section K's
//!   heading (section K-1's closing paragraph to after it, or section K's
//!   first paragraph to before it), for the first mid-document K where that
//!   moves `sec:K` to another page. No label value changes. The run reports
//!   which move it used and how many section headings changed page.
//!
//! Usage (release):
//!   CARGO_BUILD_JOBS=4 cargo run --release -p flashtex-render-pipeline \
//!       --example xref_measure -- [runs] [sections]      (defaults 5, 450)
//!   ... --example xref_measure -- --emit [sections] [toc]  (print a document)

use std::collections::BTreeMap;
use std::time::Instant;

use flashtex_compiler::json::{self, Value};
use flashtex_render_pipeline::incremental::CacheCounters;
use flashtex_render_pipeline::{protocol, FontSet, RenderCache, RenderOptions};

const BIB_ENTRIES: usize = 60;

/// #535's `large_document` (crates/compiler/src/bin/large_doc_bench.rs on
/// `agent/daniel-parent/compiler-perf`), byte for byte when `xref` is
/// false. `xref` adds `\tableofcontents` and a `\pageref` per section.
fn large_document(sections: usize, xref: bool) -> String {
    let mut out = String::with_capacity(sections * 2_700 + 8_000);
    out.push_str(
        "\\documentclass[11pt]{article}\n\\usepackage{amsmath,amssymb}\n\
         \\newcommand{\\proj}{FlashTeX}\n\\newcommand{\\R}{\\mathbb{R}}\n\
         \\title{A Long Synthetic Article}\n\\author{Bench Author}\n\\date{}\n\
         \\begin{document}\n\\maketitle\n\n",
    );
    if xref {
        out.push_str("\\tableofcontents\n\n");
    }
    for s in 0..sections {
        let prev = s.saturating_sub(1);
        let cite = s % BIB_ENTRIES;
        let cite2 = (s * 7 + 3) % BIB_ENTRIES;
        let back = if xref { format!(" on page~\\pageref{{sec:{s}}}") } else { String::new() };
        out.push_str(&format!(
            "\\section{{Topic number {s}}}\\label{{sec:{s}}}\n\
             This section continues the discussion of Section~\\ref{{sec:{prev}}} and builds on \
             the results of~\\cite{{ref{cite}}}. The \\proj{{}} compiler lays out paragraph {s} \
             with enough ordinary words to wrap across several lines, so that the line breaker \
             and the page builder both have real work to do on every block of this section.\n\n\
             Let $f_{{{s}}}\\colon \\R \\to \\R$ be given by $f_{{{s}}}(x) = x^{{2}} + {s}x + 1$. \
             Then by Equation~\\eqref{{eq:{s}}} and the bound in~\\cite{{ref{cite2}}} we obtain \
             $\\alpha_{{{s}}} \\le \\beta + \\sum_{{i=1}}^{{n}} \\gamma_i$ for every admissible \
             choice of the parameters, which is what we wanted to show here.\n\
             \\begin{{equation}}\\label{{eq:{s}}}\n\
             \\int_0^1 f_{{{s}}}(x)\\,dx = \\frac{{1}}{{3}} + \\frac{{{s}}}{{2}} + 1\n\
             \\end{{equation}}\n\
             The following observations hold:\n\
             \\begin{{itemize}}\n\
             \\item the first observation about topic {s};\n\
             \\item a second, longer observation that refers back to Section~\\ref{{sec:{s}}}{back};\n\
             \\item a third observation.\n\
             \\end{{itemize}}\n\
             \\begin{{enumerate}}\n\
             \\item Compute $a_{{{s}}}$.\n\
             \\item Bound $b_{{{s}}}$ from above.\n\
             \\end{{enumerate}}\n\
             \\begin{{align*}}\n\
             x + y &= {s} \\\\\n\
             x - y &= 1\n\
             \\end{{align*}}\n\
             \\begin{{center}}\n\
             \\begin{{tabular}}{{lrr}}\n\
             \\hline\n\
             Case & Value & Error \\\\\n\
             \\hline\n\
             A{s} & {s} & 0.{cite} \\\\\n\
             B{s} & {cite2} & 1.{s} \\\\\n\
             \\hline\n\
             \\end{{tabular}}\n\
             \\end{{center}}\n\
             A closing paragraph for section {s} discusses results in some detail, with enough \
             words to wrap across a line and exercise the paragraph breaker properly.\n\n"
        ));
    }
    out.push_str("\\begin{thebibliography}{99}\n");
    for i in 0..BIB_ENTRIES {
        out.push_str(&format!(
            "\\bibitem{{ref{i}}} A.~Author{i} and B.~Writer. \\emph{{A study of topic {i}}}. \
             Journal of Examples, {i}(2):1--20, 2020.\n"
        ));
    }
    out.push_str("\\end{thebibliography}\n\\end{document}\n");
    out
}

fn request(revision: usize, text: &str) -> String {
    let mut doc = Value::obj();
    doc.set("path", json::str_("main.tex"));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("xref-measure"));
    payload.set("revision", json::num(revision as f64));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", Value::Arr(vec![doc]));
    payload.set("layout_capabilities", Value::Arr(vec![json::str_("rules-v1"), json::str_("font-hints-v1")]));
    let mut v = Value::obj();
    v.set("protocol_version", json::num(1.0));
    v.set("id", json::str_(&format!("r{revision}")));
    v.set("type", json::str_("compile"));
    v.set("payload", payload);
    json::write(&v)
}

fn heading(s: usize) -> String {
    format!("\\section{{Topic number {s}}}")
}

/// The page number of every `\section{Topic number N}` heading, from the
/// source provenance of the glyph clusters on each page (the last page
/// they appear on, so the body heading and not its contents entry).
fn heading_pages(text: &str, v2: &flashtex_render_pipeline::DisplayList, sections: usize) -> BTreeMap<usize, u32> {
    let spans: Vec<(usize, usize, usize)> = (0..sections)
        .filter_map(|s| {
            let h = heading(s);
            text.find(&h).map(|at| (at, at + h.len(), s))
        })
        .collect();
    let mut out = BTreeMap::new();
    for page in &v2.pages {
        for item in &page.items {
            if let flashtex_render_pipeline::display::Item::GlyphRun(run) = item {
                for c in &run.clusters {
                    for src in c.provenance.sources() {
                        let b = src.start_byte;
                        let i = spans.partition_point(|(start, _, _)| *start <= b);
                        if i > 0 && b < spans[i - 1].1 {
                            // The last page wins: a contents-list entry
                            // carries its heading's provenance too.
                            out.insert(spans[i - 1].2, page.number);
                        }
                    }
                }
            }
        }
    }
    out
}

/// A paragraph moved across section `k`'s heading, changing no label value.
/// `push == false`: section `k-1`'s closing paragraph goes to just after
/// the `\label{sec:k}` line (the heading moves up). `push == true`: section
/// `k`'s first paragraph goes to just before the heading (it moves down).
fn move_paragraph(base: &str, k: usize, push: bool) -> String {
    let anchor = format!("\\label{{sec:{k}}}\n");
    let (start, end) = if push {
        let start = base.find(&anchor).expect("label line") + anchor.len();
        (start, start + base[start..].find("\n\n").expect("paragraph end") + 2)
    } else {
        let start = base.find(&format!("A closing paragraph for section {} ", k - 1)).expect("closing paragraph");
        (start, start + base[start..].find("\n\n").expect("paragraph end") + 2)
    };
    let para = base[start..end].to_string();
    let mut text = base.to_string();
    text.replace_range(start..end, "");
    let at = if push { text.find(&heading(k)).expect("heading") } else { text.find(&anchor).expect("label line") + anchor.len() };
    text.insert_str(at, &para);
    text
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn loadavg() -> String {
    std::process::Command::new("sysctl")
        .args(["-n", "vm.loadavg"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().trim_matches(['{', '}', ' ']).to_string())
        .unwrap_or_default()
}

struct Sample {
    counters: CacheCounters,
    handle_ms: f64,
    render_ms: f64,
    passes: u32,
}

/// Fresh cache, warm it with `base`, then time `edited` against it.
fn measure_once(fonts: &FontSet, options: &RenderOptions, base: &str, edited: &str) -> (Sample, Sample) {
    let cache = RenderCache::new();
    let t = Instant::now();
    let cold = protocol::handle_line(&request(1, base), fonts, options, Some(&cache));
    let cold_ms = t.elapsed().as_secs_f64() * 1e3;
    let cold_r = cold.rendered.expect("cold render");
    let cold_sample = Sample { counters: cache.counters(), handle_ms: cold_ms, render_ms: cold_r.elapsed_ms, passes: cold_r.passes };
    let before = cache.counters();
    let t = Instant::now();
    let warm = protocol::handle_line(&request(2, edited), fonts, options, Some(&cache));
    let warm_ms = t.elapsed().as_secs_f64() * 1e3;
    let warm_r = warm.rendered.expect("warm render");
    let warm_sample = Sample { counters: cache.counters().since(before), handle_ms: warm_ms, render_ms: warm_r.elapsed_ms, passes: warm_r.passes };
    (cold_sample, warm_sample)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--emit") {
        let sections = args.get(1).map_or(450, |a| a.parse().expect("sections"));
        print!("{}", large_document(sections, args.get(2).map(String::as_str) == Some("toc")));
        return;
    }
    let runs: usize = args.first().map_or(5, |a| a.parse().expect("runs"));
    let sections: usize = args.get(1).map_or(450, |a| a.parse().expect("sections"));
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    println!(
        "# xref_measure: runs={runs} sections={sections} profile={} load(1,5,15)={}",
        if cfg!(debug_assertions) { "DEBUG (numbers not meaningful)" } else { "release" },
        loadavg()
    );
    // First request in the process pays one-time font loading.
    let _ = protocol::handle_line(&request(0, &large_document(4, true)), &fonts, &options, Some(&RenderCache::new()));

    println!("| document | edit | adapted miss / lookups | blocks miss / lookups | assembled miss / lookups | layout passes | handle_line ms (median) | render ms (median) |");
    println!("|---|---|---:|---:|---:|---:|---:|---:|");
    for xref in [false, true] {
        let name = if xref { "535+toc+pageref" } else { "535" };
        let base = large_document(sections, xref);
        let probe = protocol::handle_line(&request(0, &base), &fonts, &options, None).rendered.expect("render");
        let pages = heading_pages(&base, &probe.v2, sections);
        if std::env::var_os("XREF_MEASURE_DEBUG").is_some() {
            eprintln!("{name}: heading pages {pages:?}");
        }
        eprintln!(
            "{name}: {} bytes, {} pages, {} passes, {} diagnostics",
            base.len(),
            probe.v2.pages.len(),
            probe.passes,
            probe.v2.diagnostics.len()
        );

        let mid = base.len() / 2;
        let offset = mid + base[mid..].find("ordinary words").expect("anchor") + "ordinary".len();
        let mut no_label = base.clone();
        no_label.insert(offset, 'x');

        let at = base.find(&heading(1)).expect("section 1");
        let mut renumber = base.clone();
        renumber.insert_str(at, "\\section{An inserted section}\\label{sec:inserted}\nA short inserted paragraph.\n\n");

        // A mid-document section whose heading starts a page, and whose
        // move-label edit really moves it.
        let mut move_label = None;
        for (k, push) in (sections / 2..sections - 1).flat_map(|k| [(k, false), (k, true)]) {
            let text = move_paragraph(&base, k, push);
            let r = protocol::handle_line(&request(0, &text), &fonts, &options, None).rendered.expect("render");
            let moved = heading_pages(&text, &r.v2, sections);
            if moved.get(&k) != pages.get(&k) {
                let changed = (0..sections).filter(|s| moved.get(s) != pages.get(s)).count();
                eprintln!(
                    "{name}: move-label moves a paragraph {} section {k}'s heading: sec:{k} page {:?} -> {:?}; {changed} of {} section headings changed page; {} -> {} pages",
                    if push { "from after to before" } else { "from before to after" },
                    pages.get(&k),
                    moved.get(&k),
                    pages.len(),
                    probe.v2.pages.len(),
                    r.v2.pages.len()
                );
                move_label = Some(text);
                break;
            }
            if std::env::var_os("XREF_MEASURE_DEBUG").is_some() {
                eprintln!("{name}: section {k} (push={push}) did not move page, trying the next");
            }
        }
        let move_label = move_label.expect("no section heading moved page");

        for (edit, text) in [("no-label", &no_label), ("renumber", &renumber), ("move-label", &move_label)] {
            let mut handle = Vec::new();
            let mut render = Vec::new();
            let mut first: Option<(Sample, Sample)> = None;
            for _ in 0..runs {
                let (cold, warm) = measure_once(&fonts, &options, &base, text);
                handle.push(warm.handle_ms);
                render.push(warm.render_ms);
                if let Some((_, w0)) = &first {
                    assert_eq!(w0.counters, warm.counters, "counters differ between runs");
                } else {
                    let c = cold.counters;
                    eprintln!(
                        "{name} cold: adapted {}/{} blocks {}/{} assembled {}/{} passes {} handle {:.1} ms render {:.1} ms",
                        c.adapted_misses,
                        c.adapted_hits + c.adapted_misses,
                        c.block_misses,
                        c.block_hits + c.block_misses,
                        c.assembled_misses,
                        c.assembled_hits + c.assembled_misses,
                        cold.passes,
                        cold.handle_ms,
                        cold.render_ms
                    );
                    first = Some((cold, warm));
                }
            }
            let (_, w) = first.expect("runs >= 1");
            let c = w.counters;
            assert_eq!(c.label_passes, u64::from(w.passes), "RenderCache pass counter disagrees with Rendered::passes");
            println!(
                "| {name} | {edit} | {} / {} | {} / {} | {} / {} | {} | {:.1} | {:.1} |",
                c.adapted_misses,
                c.adapted_hits + c.adapted_misses,
                c.block_misses,
                c.block_hits + c.block_misses,
                c.assembled_misses,
                c.assembled_hits + c.assembled_misses,
                w.passes,
                median(handle),
                median(render)
            );
        }
    }
}
