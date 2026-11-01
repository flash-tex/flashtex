//! Large-document compile budget (issue #65, `GH-65-COMPILER-PERF`).
//!
//! Synthesises a deterministic ~300-page `article` with sections, labels and
//! cross-references, citations and a `thebibliography`, inline and display
//! maths (`equation`, `align*`), `itemize`/`enumerate` lists and `tabular`
//! tables, then times, median of N runs (default 5):
//!
//! - `cold session`: `incremental::compile_full_project` from source text.
//! - `cold protocol`: `protocol::handle_line` for a fresh project id (request
//!   JSON parse, compile, export checks, reply JSON).
//! - `edit session`: insert one character mid-document, then
//!   `Session::compile_project` against a warm session.
//! - `edit protocol`: the same edit through `handle_line` on a warm project.
//!
//! Every reply is hashed, so a changed digest between two builds means the
//! output changed. `--release` is required for meaningful numbers.
//!
//! Usage: large_doc_bench [runs] [sections]   (defaults 5 and 450)
//!        large_doc_bench --emit [sections]    (print the document and exit)

use flashtex_compiler::incremental::{compile_full_project, Session};
use flashtex_compiler::json::{self, Value};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::SourceDocument;
use flashtex_compiler::protocol::handle_line;
use flashtex_font_engine::sha256;
use std::hint::black_box;
use std::time::{Duration, Instant};

const BIB_ENTRIES: usize = 60;

/// Deterministic large article: identical bytes on every machine and run.
pub fn large_document(sections: usize) -> String {
    let mut out = String::with_capacity(sections * 2_600 + 8_000);
    out.push_str(
        "\\documentclass[11pt]{article}\n\\usepackage{amsmath,amssymb}\n\
         \\newcommand{\\proj}{FlashTeX}\n\\newcommand{\\R}{\\mathbb{R}}\n\
         \\title{A Long Synthetic Article}\n\\author{Bench Author}\n\\date{}\n\
         \\begin{document}\n\\maketitle\n\n",
    );
    for s in 0..sections {
        let prev = s.saturating_sub(1);
        let cite = s % BIB_ENTRIES;
        let cite2 = (s * 7 + 3) % BIB_ENTRIES;
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
             \\item a second, longer observation that refers back to Section~\\ref{{sec:{s}}};\n\
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

fn request_for(project: &str, revision: usize, text: &str) -> String {
    let mut doc = Value::obj();
    doc.set("path", json::str_("main.tex"));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_(project));
    payload.set("revision", Value::Num(revision as f64));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", Value::Arr(vec![doc]));
    payload.set(
        "layout_capabilities",
        Value::Arr(vec![json::str_("font-hints-v1"), json::str_("rules-v1")]),
    );
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_(format!("r{revision}")));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    json::write(&env)
}

fn documents(text: &str) -> [SourceDocument<'_>; 1] {
    [SourceDocument {
        path: "main.tex",
        text,
    }]
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn median(mut samples: Vec<Duration>) -> f64 {
    samples.sort_unstable();
    ms(samples[samples.len() / 2])
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--emit") {
        let sections = args.get(1).map_or(450, |a| a.parse().expect("sections"));
        print!("{}", large_document(sections));
        return;
    }
    let runs: usize = args.first().map_or(5, |a| a.parse().expect("runs"));
    let sections: usize = args.get(1).map_or(450, |a| a.parse().expect("sections"));
    let constraints = LayoutConstraints::default();
    let base = large_document(sections);
    // Edit: one character typed mid-document, inside a paragraph.
    let mid = base.len() / 2;
    let offset = mid + base[mid..].find("ordinary words").expect("anchor") + "ordinary".len();
    let mut edited = base.clone();
    edited.insert(offset, 'x');

    // The first compile in a process also pays for one-time initialisation
    // (faces, lookup indexes, the per-thread shaping memo).
    let started = Instant::now();
    let probe = compile_full_project(&documents(&base), "main.tex", constraints);
    let first_ms = ms(started.elapsed());
    println!(
        "large document: {} bytes, {} sections, {} blocks, {} pages, {} diagnostics, profile={}",
        base.len(),
        sections,
        probe.blocks.len(),
        probe.pages.len(),
        probe.diagnostics.len(),
        if cfg!(debug_assertions) {
            "DEBUG (numbers not meaningful)"
        } else {
            "release"
        }
    );
    drop(probe);

    let mut cold_session = Vec::new();
    let mut cold_protocol = Vec::new();
    let mut edit_session = Vec::new();
    let mut edit_protocol = Vec::new();
    let mut digest = Vec::new();
    for run in 0..runs {
        let started = Instant::now();
        let output = black_box(compile_full_project(
            black_box(&documents(&base)),
            "main.tex",
            constraints,
        ));
        cold_session.push(started.elapsed());
        drop(output);

        let mut session = Session::new();
        session.compile_project(&documents(&base), "main.tex", constraints);
        let started = Instant::now();
        let result = black_box(session.compile_project(
            black_box(&documents(&edited)),
            "main.tex",
            constraints,
        ));
        edit_session.push(started.elapsed());
        if run == 0 {
            println!("edit reuse: {:?}", result.stats);
        }
        drop(result);
        drop(session);

        let project = format!("large-doc-{run}");
        let first = request_for(&project, 0, &base);
        let second = request_for(&project, 1, &edited);
        let started = Instant::now();
        let reply = black_box(handle_line(black_box(&first)));
        cold_protocol.push(started.elapsed());
        if run == 0 {
            digest.extend_from_slice(reply.as_bytes());
        }
        let started = Instant::now();
        let reply = black_box(handle_line(black_box(&second)));
        edit_protocol.push(started.elapsed());
        if run == 0 {
            digest.extend_from_slice(reply.as_bytes());
        }
    }
    println!("first compile in process: {first_ms:.1} ms");
    println!("median of {runs} runs, ms:");
    println!("  cold session   {:>10.1}", median(cold_session));
    println!("  cold protocol  {:>10.1}", median(cold_protocol));
    println!("  edit session   {:>10.1}", median(edit_session));
    println!("  edit protocol  {:>10.1}", median(edit_protocol));
    println!(
        "  reply sha256 (cold+edit) {}",
        hex(&sha256::digest(&digest))
    );
}
