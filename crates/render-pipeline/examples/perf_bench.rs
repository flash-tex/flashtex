//! FT-065 render-pipeline benchmark: the `flashtex-render` worker's real
//! request path (`protocol::handle_line`, JSON in -> render -> JSON out)
//! on HW1, HW2 and the 500 KB scaling document.
//!
//! Scenarios per document:
//! - `full`: a fresh `RenderCache` per request (fonts already loaded), i.e.
//!   a whole-document render as after opening the file.
//! - `type-paragraph`, `type-inline-math`, `type-display-eq`: one more
//!   character per step at a fixed place, against a warm cache (the
//!   single-keystroke path the Mac app drives).
//! - `delete-restore-line`: alternately delete and restore one line.
//!
//! Each scenario runs with the Mac's default capabilities (`rules-v1`,
//! `font-hints-v1`) and, as `+v2`, with `display-list-v2` (the V2 pane).
//! As `+links+delta` it additionally negotiates `display-list-v2-links` and
//! `display-list-v2-delta` over the stateful worker entry (`handle_line_with`
//! with the acknowledged installed base, as the Mac app drives it): the warm
//! keystroke latency with links + delta together (issue #1003).
//! Every reply line (compile_result + display_list) is hashed in order; the
//! scenario digest changes if any output byte changes, so two builds are
//! byte-identical exactly when all digests match (`--digests` writes them,
//! `--check` compares). `--verify-fresh` additionally compares every warm
//! reply with a cacheless render of the same text (skipped for `+links+delta`,
//! whose delta lines have no cacheless counterpart).
//!
//! Usage (release):
//!   cargo run --release --example perf_bench -- [--steps N] [--only SUBSTR]
//!       [--hw2 PATH] [--digests OUT] [--check FILE] [--verify-fresh]

use std::collections::BTreeMap;
use std::time::Instant;

use flashtex_compiler::json::{self, Value};
use flashtex_font_engine::sha256;
use flashtex_render_pipeline::delta::{Base, DeltaState};
use flashtex_render_pipeline::{protocol, FontSet, RenderCache, RenderOptions};

/// The capability sets each scenario runs under.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Caps {
    /// The Mac's default: `rules-v1` + `font-hints-v1`.
    V1,
    /// Plus `display-list-v2` (the V2 pane).
    V2,
    /// Plus `display-list-v2-links` and `display-list-v2-delta`: the Mac
    /// app's keystroke path, which always requests links (issue #1003).
    /// Driven over the stateful worker entry with the acknowledged installed
    /// base, so warm steps can take the incremental delta path.
    V2LinksDelta,
}

impl Caps {
    fn label(self) -> &'static str {
        match self {
            Caps::V1 => "",
            Caps::V2 => " +v2",
            Caps::V2LinksDelta => " +links+delta",
        }
    }
}

const HW1: &str = include_str!("../../../fixtures/real-world/hw1/HW1.tex");
const TYPED: &[u8] = b"abcde fghij ";

/// The compiler's `scaling_bench` generator: identical bytes everywhere.
fn scaling_document(target_bytes: usize) -> String {
    let mut out = String::with_capacity(target_bytes + 512);
    out.push_str("\\documentclass{article}\n\\newcommand{\\proj}{FlashTeX}\n\\begin{document}\n");
    let mut n = 0usize;
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    while out.len() < target_bytes {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        match (seed >> 33) % 4 {
            0 => out.push_str(&format!("\\section{{Section {n}}}\n")),
            1 => out.push_str(&format!("Paragraph {n} of \\proj{{}} with inline $x^{{{n}}} + \\alpha$ maths.\n\n")),
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
    Full,
    Type { offset: usize },
    DeleteLine { start: usize, end: usize },
}

struct Scenario {
    name: String,
    base: std::rc::Rc<String>,
    edit: Edit,
}

impl Scenario {
    fn text_at(&self, step: usize) -> String {
        match self.edit {
            Edit::Full => (*self.base).clone(),
            Edit::Type { offset } => {
                let typed: String = (0..step).map(|i| TYPED[i % TYPED.len()] as char).collect();
                let mut text = (*self.base).clone();
                text.insert_str(offset, &typed);
                text
            }
            Edit::DeleteLine { start, end } => {
                let mut text = (*self.base).clone();
                if step % 2 == 1 {
                    text.replace_range(start..end, "");
                }
                text
            }
        }
    }
}

fn after(text: &str, from: usize, needle: &str) -> usize {
    from + text[from..].find(needle).unwrap_or_else(|| panic!("anchor {needle:?} not found")) + needle.len()
}

fn line_containing(text: &str, from: usize, needle: &str) -> (usize, usize) {
    let hit = from + text[from..].find(needle).unwrap_or_else(|| panic!("line anchor {needle:?} not found"));
    let start = text[..hit].rfind('\n').map_or(0, |i| i + 1);
    let end = text[hit..].find('\n').map_or(text.len(), |i| hit + i + 1);
    (start, end)
}

fn scenarios_for(label: &str, base: String, anchors: [(usize, &str); 4]) -> Vec<Scenario> {
    let base = std::rc::Rc::new(base);
    let [paragraph, inline, display, line] = anchors;
    let (start, end) = line_containing(&base, line.0, line.1);
    [
        ("full", Edit::Full),
        ("type-paragraph", Edit::Type { offset: after(&base, paragraph.0, paragraph.1) }),
        ("type-inline-math", Edit::Type { offset: after(&base, inline.0, inline.1) }),
        ("type-display-eq", Edit::Type { offset: after(&base, display.0, display.1) }),
        ("delete-restore-line", Edit::DeleteLine { start, end }),
    ]
    .into_iter()
    .map(|(name, edit)| Scenario { name: format!("{label} {name}"), base: base.clone(), edit })
    .collect()
}

fn request(revision: usize, text: &str, caps: Caps, base: Option<&Base>) -> String {
    let mut doc = Value::obj();
    doc.set("path", json::str_("main.tex"));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("perf"));
    payload.set("revision", json::num(revision as f64));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", Value::Arr(vec![doc]));
    let mut c = vec![json::str_("rules-v1"), json::str_("font-hints-v1")];
    match caps {
        Caps::V1 => {}
        Caps::V2 => c.push(json::str_("display-list-v2")),
        Caps::V2LinksDelta => {
            c.push(json::str_("display-list-v2"));
            c.push(json::str_("display-list-v2-links"));
            c.push(json::str_("display-list-v2-delta"));
        }
    }
    payload.set("layout_capabilities", Value::Arr(c));
    // The acknowledged installed base, as the consumer echoes it back on the
    // stateful path (`display-list-v2-delta` proposal r5 §3).
    if let Some(b) = base {
        let mut o = Value::obj();
        o.set("request_id", json::str_(&b.request_id));
        o.set("project_id", json::str_(&b.project_id));
        o.set("revision", json::num(b.revision as f64));
        o.set("page_count", json::num(b.page_count as f64));
        o.set("list_digest", json::str_(&b.list_digest));
        payload.set("display_list_base", o);
    }
    let mut v = Value::obj();
    v.set("protocol_version", json::num(1.0));
    v.set("id", json::str_(&format!("r{revision}")));
    v.set("type", json::str_("compile"));
    v.set("payload", payload);
    json::write(&v)
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn pct(sorted: &[f64], q: f64) -> f64 {
    sorted[(((sorted.len() - 1) as f64) * q).round() as usize]
}

fn loadavg() -> String {
    std::process::Command::new("sysctl")
        .args(["-n", "vm.loadavg"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().trim_matches(['{', '}', ' ']).to_string())
        .unwrap_or_default()
}

fn main() {
    let mut steps = 40usize;
    let mut only: Option<String> = None;
    let mut hw2_path: Option<String> = None;
    let mut digests_out: Option<String> = None;
    let mut check: Option<String> = None;
    let mut verify_fresh = false;
    let mut hash = true;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--steps" => steps = args.next().and_then(|s| s.parse().ok()).expect("--steps N"),
            "--only" => only = args.next(),
            "--hw2" => hw2_path = args.next(),
            "--digests" => digests_out = args.next(),
            "--check" => check = args.next(),
            "--verify-fresh" => verify_fresh = true,
            // Profiling: skip hashing replies so samples show the product path only.
            "--no-hash" => hash = false,
            other => panic!("unknown argument {other}"),
        }
    }
    let hw2_path = hw2_path.unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/real-world/hw2/HW2.tex").to_string());

    let mut all = scenarios_for("HW1", HW1.to_string(), [(0, "Your solution"), (0, "Let $a"), (0, "5=2"), (0, "For every true proposition")]);
    match std::fs::read_to_string(&hw2_path) {
        Ok(hw2) => all.extend(scenarios_for(
            "HW2",
            hw2,
            [(0, "Your proof must proceed"), (0, "nonempty sets $A,B"), (0, "M_n=\\{x"), (0, "Your proof should explicitly")],
        )),
        Err(e) => eprintln!("perf_bench: HW2 skipped ({hw2_path}: {e})"),
    }
    let big = scaling_document(500_000);
    let mid = big.len() / 2;
    all.extend(scenarios_for("500KB", big, [(mid, "discusses"), (mid, "inline $x"), (mid, "$$\\frac{a"), (mid, "discusses results")]));

    let t_fonts = Instant::now();
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    // Load every face once so no scenario pays first-use font loading.
    let warm = RenderCache::new();
    let _ = protocol::handle_line(&request(0, HW1, Caps::V2, None), &fonts, &options, Some(&warm));
    println!("# machine: {} | load(1,5,15): {} | font set + first HW1 request: {:.1} ms", machine(), loadavg(), t_fonts.elapsed().as_secs_f64() * 1e3);
    println!("{:<34} {:>5} {:>9} {:>9} {:>9} {:>9}  {:<16} load", "scenario", "steps", "p50 ms", "p95 ms", "max ms", "first ms", "digest");

    let mut digests: BTreeMap<String, String> = BTreeMap::new();
    let mut failures = 0usize;
    for caps in [Caps::V1, Caps::V2, Caps::V2LinksDelta] {
        for s in &all {
            let name = format!("{}{}", s.name, caps.label());
            if only.as_deref().is_some_and(|o| !name.contains(o)) {
                continue;
            }
            let load_before = loadavg();
            let cache = RenderCache::new();
            // The worker's delta state, held across the warm steps exactly as
            // the serving worker holds it. Only the +links+delta pass sends a
            // base; the other passes run the stateless entry as before.
            let delta_state = DeltaState::new();
            let delta = (caps == Caps::V2LinksDelta).then_some(&delta_state);
            let is_full = matches!(s.edit, Edit::Full);
            let n = if is_full { steps.min(if s.base.len() > 100_000 { 5 } else { steps }) } else { steps };
            // Warm: the unedited document once (not timed, not hashed).
            if !is_full {
                let _ = protocol::handle_line_with(&request(0, &s.base, caps, None), &fonts, &options, Some(&cache), delta);
            }
            let mut times = Vec::with_capacity(n);
            let mut all_bytes = Vec::new();
            let mut deltas = 0usize;
            for step in 1..=n {
                let text = s.text_at(step);
                // The in-sync consumer echoes the last installed base.
                let ack = delta.and_then(|st| st.acknowledgement());
                let line = request(step, &text, caps, ack.as_ref());
                let fresh_cache;
                let c = if is_full {
                    fresh_cache = RenderCache::new();
                    &fresh_cache
                } else {
                    &cache
                };
                let t = Instant::now();
                let reply = protocol::handle_line_with(&line, &fonts, &options, Some(c), delta);
                times.push(t.elapsed().as_secs_f64() * 1e3);
                if delta.is_some() && reply.extra_lines.iter().any(|l| l.contains("\"type\":\"display_list_delta\"")) {
                    deltas += 1;
                }
                let mut bytes = reply.line.into_bytes();
                for extra in reply.extra_lines {
                    bytes.push(b'\n');
                    bytes.extend_from_slice(extra.as_bytes());
                }
                // A delta line has no cacheless counterpart, so there is
                // nothing fresh to compare it against.
                if verify_fresh && !is_full && delta.is_none() {
                    let fresh = protocol::handle_line(&line, &fonts, &options, None);
                    let mut fb = fresh.line.into_bytes();
                    for extra in fresh.extra_lines {
                        fb.push(b'\n');
                        fb.extend_from_slice(extra.as_bytes());
                    }
                    if fb != bytes {
                        failures += 1;
                        eprintln!("MISMATCH {name} step {step}: warm reply differs from a fresh render");
                    }
                }
                if hash {
                    all_bytes.extend_from_slice(&sha256::digest(&bytes));
                }
            }
            let digest = hex(&sha256::digest(&all_bytes))[..16].to_string();
            let first = times[0];
            let mut sorted = times.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            println!(
                "{:<34} {:>5} {:>9.2} {:>9.2} {:>9.2} {:>9.2}  {:<16} {} -> {}",
                name,
                n,
                pct(&sorted, 0.5),
                pct(&sorted, 0.95),
                sorted[sorted.len() - 1],
                first,
                digest,
                load_before,
                loadavg()
            );
            if delta.is_some() {
                println!("# {name}: {deltas}/{n} warm replies took the delta path");
            }
            digests.insert(name, digest);
        }
    }
    let text: String = digests.iter().map(|(k, v)| format!("{v} {k}\n")).collect();
    if let Some(p) = digests_out {
        std::fs::write(&p, &text).expect("write digests");
    }
    if let Some(p) = check {
        let want = std::fs::read_to_string(&p).expect("read digests");
        for l in want.lines() {
            let Some((d, name)) = l.split_once(' ') else { continue };
            match digests.get(name) {
                Some(got) if got == d => {}
                Some(got) => {
                    failures += 1;
                    eprintln!("DIGEST MISMATCH {name}: expected {d}, got {got}");
                }
                None => {}
            }
        }
        println!("# digest check against {p}: {}", if failures == 0 { "identical" } else { "DIFFERENT" });
    }
    if failures > 0 {
        std::process::exit(1);
    }
}

fn machine() -> String {
    let cpu = std::process::Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    format!("{cpu}, {} logical CPUs", std::thread::available_parallelism().map_or(0, |n| n.get()))
}
