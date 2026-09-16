//! Per-keystroke latency through the warm incremental path (issue #65).
//!
//! Replays deterministic editor edits on HW1 and the 500 KB scaling document:
//! typing in a paragraph, in inline math and in a display equation, and deleting
//! (then restoring) a whole line. Every step is one compile against a warm
//! session. Two paths are timed separately:
//!
//! - `session`: `incremental::Session::compile_project` only.
//! - `protocol`: `protocol::handle_line`, the IDE's real request path
//!   (request JSON parse, warm session compile, export checks, reply JSON).
//!
//! `--release` is required for meaningful numbers. The `reply sha256` column
//! hashes every protocol reply of the scenario in order, so any output change
//! between two builds is visible as a different digest.
//!
//! `HW1 runaway` is HW1 with a `\def\r{x\r}\r` loop in its body: every
//! typing revision runs into the expansion step limit, and the line edit
//! deletes and restores the loop.
//!
//! Usage: edit_latency_bench [iterations] (default 60) [--verify] [--only SUBSTR].

use flashtex_compiler::incremental::{compile_full_project, Session};
use flashtex_compiler::json::{self, Value};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::SourceDocument;
use flashtex_compiler::protocol::handle_line;
use flashtex_font_engine::sha256;
use std::hint::black_box;
use std::time::{Duration, Instant};

const HW1: &str = include_str!("../../../../fixtures/real-world/hw1/HW1.tex");
const TYPED: &[u8] = b"abcde fghij ";

/// Same generator as `scaling_bench`: identical bytes on every run.
fn scaling_document(target_bytes: usize) -> String {
    let mut out = String::with_capacity(target_bytes + 512);
    out.push_str("\\documentclass{article}\n\\newcommand{\\proj}{FlashTeX}\n\\begin{document}\n");
    let mut n = 0usize;
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    while out.len() < target_bytes {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        match (seed >> 33) % 4 {
            0 => out.push_str(&format!("\\section{{Section {n}}}\n")),
            1 => out.push_str(&format!(
                "Paragraph {n} of \\proj{{}} with inline $x^{{{n}}} + \\alpha$ maths.\n\n"
            )),
            2 => out.push_str(&format!(
                "Paragraph {n} discusses results in some detail, with enough words to wrap \
                 across a line and exercise the paragraph breaker properly.\n\n"
            )),
            _ => out.push_str(&format!("Displayed: $$\\frac{{a_{{{n}}}}}{{b}}$$\n\n")),
        }
        n += 1;
    }
    out.push_str("\\end{document}\n");
    out
}

#[derive(Clone, Copy)]
enum Edit {
    /// Insert one more character at `offset` per step (cumulative typing).
    Type { offset: usize },
    /// Alternately delete and restore the line `start..end`.
    DeleteLine { start: usize, end: usize },
}

struct Scenario {
    name: String,
    base: String,
    edit: Edit,
}

impl Scenario {
    /// Source text after `step` (1-based) edits have been applied.
    fn text_at(&self, step: usize) -> String {
        match self.edit {
            Edit::Type { offset } => {
                let typed: String = (0..step).map(|i| TYPED[i % TYPED.len()] as char).collect();
                let mut text = self.base.clone();
                text.insert_str(offset, &typed);
                text
            }
            Edit::DeleteLine { start, end } => {
                if step % 2 == 1 {
                    let mut text = self.base.clone();
                    text.replace_range(start..end, "");
                    text
                } else {
                    self.base.clone()
                }
            }
        }
    }
}

/// Offset just after the first occurrence of `needle` at or after `from`.
fn after(text: &str, from: usize, needle: &str) -> usize {
    from + text[from..]
        .find(needle)
        .unwrap_or_else(|| panic!("anchor {needle:?} not found"))
        + needle.len()
}

fn line_containing(text: &str, from: usize, needle: &str) -> (usize, usize) {
    let hit = from
        + text[from..]
            .find(needle)
            .unwrap_or_else(|| panic!("line anchor {needle:?} not found"));
    let start = text[..hit].rfind('\n').map_or(0, |i| i + 1);
    let end = text[hit..].find('\n').map_or(text.len(), |i| hit + i + 1);
    (start, end)
}

fn scenarios_for(label: &str, base: &str, anchors: [(usize, &str); 4]) -> Vec<Scenario> {
    let [paragraph, inline, display, line] = anchors;
    let (start, end) = line_containing(base, line.0, line.1);
    [
        (
            "type in paragraph",
            Edit::Type {
                offset: after(base, paragraph.0, paragraph.1),
            },
        ),
        (
            "type in inline math",
            Edit::Type {
                offset: after(base, inline.0, inline.1),
            },
        ),
        (
            "type in display eq",
            Edit::Type {
                offset: after(base, display.0, display.1),
            },
        ),
        ("delete/restore line", Edit::DeleteLine { start, end }),
    ]
    .into_iter()
    .map(|(name, edit)| Scenario {
        name: format!("{label}: {name}"),
        base: base.to_string(),
        edit,
    })
    .collect()
}

fn all_scenarios() -> Vec<Scenario> {
    let mut scenarios = scenarios_for(
        "HW1",
        HW1,
        [
            (0, "Your solution"),
            (0, "Let $a"),
            (0, "5=2"),
            (0, "For every true proposition"),
        ],
    );
    let runaway = HW1.replacen("Your solution", "\\def\\r{x\\r}\\r Your solution", 1);
    scenarios.extend(scenarios_for(
        "HW1 runaway",
        &runaway,
        [
            (0, "Your solution"),
            (0, "Let $a"),
            (0, "5=2"),
            // Deleting the loop's line ends the runaway; restoring starts it.
            (0, "\\def\\r{"),
        ],
    ));
    let big = scaling_document(500_000);
    let mid = big.len() / 2;
    scenarios.extend(scenarios_for(
        "500KB",
        &big,
        [
            (mid, "discusses"),
            (mid, "inline $x"),
            (mid, "$$\\frac{a"),
            (mid, "discusses results"),
        ],
    ));
    scenarios
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

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn percentile(sorted: &[Duration], q: f64) -> f64 {
    ms(sorted[((sorted.len() - 1) as f64 * q).round() as usize])
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn documents(text: &str) -> [SourceDocument<'_>; 1] {
    [SourceDocument {
        path: "main.tex",
        text,
    }]
}

fn main() {
    let iterations: usize = std::env::args()
        .nth(1)
        .map(|arg| arg.parse().expect("iterations must be a number"))
        .unwrap_or(60);
    let verify = std::env::args().any(|arg| arg == "--verify");
    let args: Vec<String> = std::env::args().collect();
    let only = args.iter().position(|arg| arg == "--only").and_then(|at| args.get(at + 1).cloned());
    let constraints = LayoutConstraints::default();
    println!(
        "edit latency, {iterations} iterations per scenario, profile={}",
        if cfg!(debug_assertions) {
            "DEBUG (numbers not meaningful)"
        } else {
            "release"
        }
    );
    println!(
        "{:<30} {:>10} {:>10} {:>10} {:>10}  {:<16} reuse(first step)",
        "scenario", "sess p50", "sess p95", "proto p50", "proto p95", "reply sha256"
    );

    for (scenario_index, scenario) in all_scenarios().iter().enumerate() {
        if only.as_ref().is_some_and(|only| !scenario.name.contains(only.as_str())) {
            continue;
        }
        let texts: Vec<String> = (0..=iterations).map(|i| scenario.text_at(i)).collect();

        let mut session = Session::new();
        session.compile_project(&documents(&texts[0]), "main.tex", constraints);
        let mut session_times = Vec::with_capacity(iterations);
        let mut first_stats = None;
        for text in &texts[1..] {
            let started = Instant::now();
            let result = black_box(session.compile_project(
                black_box(&documents(text)),
                "main.tex",
                constraints,
            ));
            session_times.push(started.elapsed());
            first_stats.get_or_insert(result.stats);
            if verify {
                let clean = compile_full_project(&documents(text), "main.tex", constraints);
                assert_eq!(
                    format!("{:#?}", result.output),
                    format!("{clean:#?}"),
                    "{}: incremental diverged from clean build",
                    scenario.name
                );
            }
        }

        let project = format!("edit-latency-{scenario_index}");
        let requests: Vec<String> = texts
            .iter()
            .enumerate()
            .map(|(revision, text)| request_for(&project, revision, text))
            .collect();
        let mut digest_input = Vec::new();
        digest_input.extend_from_slice(handle_line(&requests[0]).as_bytes());
        let mut protocol_times = Vec::with_capacity(iterations);
        for request in &requests[1..] {
            let started = Instant::now();
            let reply = black_box(handle_line(black_box(request)));
            protocol_times.push(started.elapsed());
            digest_input.extend_from_slice(reply.as_bytes());
        }

        session_times.sort_unstable();
        protocol_times.sort_unstable();
        let stats = first_stats.expect("at least one iteration");
        println!(
            "{:<30} {:>10.3} {:>10.3} {:>10.3} {:>10.3}  {:<16} {}/{}{}",
            scenario.name,
            percentile(&session_times, 0.50),
            percentile(&session_times, 0.95),
            percentile(&protocol_times, 0.50),
            percentile(&protocol_times, 0.95),
            &hex(&sha256::digest(&digest_input))[..16],
            stats.blocks_reused,
            stats.blocks_total,
            if stats.full_recompile { " full" } else { "" },
        );
    }
    if verify {
        println!("incremental == clean full build at every step: PASS");
    }
}
