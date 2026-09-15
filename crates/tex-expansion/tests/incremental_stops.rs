//! Incremental re-expansion across stops: edits that make a run hit (or no
//! longer hit) the step limit, the output token limit, TeX's capacity limits
//! or a nesting limit must still give exactly a from-scratch run's tokens,
//! invocation origins, diagnostics and labels.
//!
//! The step and output limits count from the document start, so they are the
//! one part of a run that an equivalent state at a checkpoint does not decide.
//! Before this was handled, a reused suffix could end at the old run's stop
//! (or run past the new one), and a checkpoint carried past a converged edit
//! kept its old step count.

use std::path::{Path, PathBuf};

use flashtex_tex_expansion::{Diagnostic, Edit, Engine, IncrementalExpander, LabelRecord, Limits, Span, Token};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

struct Run {
    tokens: Vec<Token>,
    origins: Vec<Option<Span>>,
    diagnostics: Vec<Diagnostic>,
    labels: Vec<LabelRecord>,
    steps: u64,
}

/// A from-scratch run, as `Engine::run` does it, keeping origins too.
fn full(src: &str, limits: Limits) -> Run {
    let mut e = Engine::with_limits(src, limits);
    let mut tokens = Vec::new();
    let mut origins = Vec::new();
    while let Some((tok, origin)) = e.next_content_token_with_origin() {
        tokens.push(tok);
        origins.push(origin);
        if tokens.len() as u64 > limits.max_output_tokens {
            e.push_diagnostic(Diagnostic::error("output token limit exceeded", Span::synthetic()));
            break;
        }
    }
    Run { tokens, origins, diagnostics: e.take_diagnostics(), labels: e.take_labels(), steps: e.steps() }
}

fn assert_same(inc: &IncrementalExpander, limits: Limits, ctx: &dyn Fn() -> String) {
    let f = full(inc.source(), limits);
    if inc.tokens() != &f.tokens[..] {
        let at = inc.tokens().iter().zip(&f.tokens).position(|(a, b)| a != b).unwrap_or(inc.tokens().len().min(f.tokens.len()));
        panic!(
            "tokens differ at {at} (incremental {} tokens, full {}): {:?} vs {:?} -- {}",
            inc.tokens().len(),
            f.tokens.len(),
            inc.tokens().get(at),
            f.tokens.get(at),
            ctx()
        );
    }
    assert!(inc.origins() == &f.origins[..], "origins differ -- {}", ctx());
    assert_eq!(inc.diagnostics(), &f.diagnostics[..], "diagnostics differ -- {}", ctx());
    assert_eq!(inc.labels(), &f.labels[..], "labels differ -- {}", ctx());
}

fn lines(n: usize) -> String {
    (0..n).map(|i| format!("line {i} \\a.\n")).collect()
}

/// An edit that adds steps near the start and converges a few lines later:
/// the old suffix fit the step limit, the new run does not.
#[test]
fn converged_edit_that_pushes_the_run_over_the_step_limit() {
    let doc = format!("\\def\\a{{}}\n{}", lines(40));
    let limits = Limits { max_expansion_steps: full(&doc, Limits::default()).steps, ..Limits::default() };
    let mut inc = IncrementalExpander::with_options(&doc, limits, 16);
    assert!(!inc.diagnostics().iter().any(|d| d.message.contains("step limit")));
    let at = doc.find("line 1 ").unwrap();
    inc.edit(&Edit { start: at, end: at, replacement: "\\a\\a ".into() });
    assert_same(&inc, limits, &|| "insert \\a\\a on line 1".into());
    assert!(inc.diagnostics().iter().any(|d| d.message.contains("step limit")));
}

/// The reverse: the old run stopped, the edit removes the steps that took it
/// over, and the new run must not reuse the stopped suffix.
#[test]
fn converged_edit_that_takes_the_run_back_under_the_step_limit() {
    let doc = format!("\\def\\a{{}}\n\\a\\a\\a\\a\n{}", lines(40));
    let steps = full(&doc, Limits::default()).steps;
    let limits = Limits { max_expansion_steps: steps - 3, ..Limits::default() };
    let mut inc = IncrementalExpander::with_options(&doc, limits, 16);
    assert!(inc.diagnostics().iter().any(|d| d.message.contains("step limit")));
    let at = doc.find("\\a\\a\\a\\a").unwrap();
    inc.edit(&Edit { start: at, end: at + 8, replacement: String::new() });
    assert_same(&inc, limits, &|| "delete \\a\\a\\a\\a".into());
    assert!(!inc.diagnostics().iter().any(|d| d.message.contains("step limit")));
}

/// The same two directions for the output token limit.
#[test]
fn converged_edits_across_the_output_token_limit() {
    let doc = format!("\\def\\a{{}}\n{}", lines(40));
    let limits = Limits { max_output_tokens: full(&doc, Limits::default()).tokens.len() as u64, ..Limits::default() };
    let mut inc = IncrementalExpander::with_options(&doc, limits, 16);
    let at = doc.find("line 1 ").unwrap();
    inc.edit(&Edit { start: at, end: at, replacement: "xy".into() });
    assert_same(&inc, limits, &|| "insert xy".into());
    assert!(inc.diagnostics().iter().any(|d| d.message.contains("output token limit")));
    inc.edit(&Edit { start: at, end: at + 2, replacement: String::new() });
    assert_same(&inc, limits, &|| "delete xy".into());
    assert!(!inc.diagnostics().iter().any(|d| d.message.contains("output token limit")));
}

/// The minimised form of `incremental_matches_full_with_repeated_diagnostics_
/// and_nesting_limits` seed 3 edit 54: an edit converges with a different step
/// count, and a later edit restarts from a checkpoint carried past it and runs
/// into the step limit. The carried checkpoint kept the old run's step count,
/// so the incremental run stopped later than a full run.
#[test]
fn restart_from_a_checkpoint_carried_past_a_converged_edit_hits_the_step_limit() {
    let doc = format!("\\def\\a{{}}\\def\\r{{x\\r}}\n{}", lines(40));
    let limits = Limits { max_expansion_steps: 5_000, ..Limits::default() };
    let mut inc = IncrementalExpander::with_options(&doc, limits, 16);
    let at = doc.find("line 1 ").unwrap();
    let stats = inc.edit(&Edit { start: at, end: at, replacement: "\\a\\a\\a\\a\\a\\a ".into() });
    assert!(stats.converged_at.is_some());
    assert_same(&inc, limits, &|| "insert \\a x6".into());
    let at = inc.source().find("line 30 ").unwrap();
    inc.edit(&Edit { start: at, end: at, replacement: "\\r ".into() });
    assert_same(&inc, limits, &|| "insert \\r on line 30".into());
}

/// A macro whose argument is delimited by the end of the line leaves its
/// invocation as the engine's last origin at the next safe point, which is
/// where a step limit hit on the next step is reported. Every cut of the step
/// limit stops the run somewhere in the document, and the edit after the stop
/// restarts from the last checkpoint before it, so some cut restarts exactly
/// one step before the limit.
#[test]
fn step_limit_right_after_a_checkpoint_reports_the_last_origin() {
    let doc = format!("\\def\\d#1 {{[#1]}}\n{}end\n", (0..40).map(|i| format!("\\d w{i}\n")).collect::<String>());
    let base = full(&doc, Limits::default()).steps;
    for cut in 1..base - 20 {
        let limits = Limits { max_expansion_steps: base - cut, ..Limits::default() };
        let mut inc = IncrementalExpander::with_options(&doc, limits, 1);
        let at = doc.len() - 2;
        inc.edit(&Edit { start: at, end: at, replacement: "x".into() });
        assert_same(&inc, limits, &|| format!("cut {cut}"));
    }
}

// ------------------------------------------------------------ property test

/// Every `.tex` under `fixtures/`, `crates/*/tests/` and `crates/*/oracle/`
/// (the compiler fuzzer's seeds), sorted; never `vendor/` or `target*`.
fn seeds() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if name != "vendor" && !name.starts_with("target") && !name.starts_with('.') {
                    walk(&path, out);
                }
            } else if name.ends_with(".tex") {
                out.push(path);
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    walk(&root.join("fixtures"), &mut files);
    if let Ok(entries) = std::fs::read_dir(root.join("crates")) {
        for entry in entries.flatten() {
            walk(&entry.path().join("tests"), &mut files);
            walk(&entry.path().join("oracle"), &mut files);
        }
    }
    files.sort();
    files
        .into_iter()
        .filter_map(|p| Some((p.strip_prefix(&root).unwrap_or(&p).display().to_string(), std::fs::read_to_string(&p).ok()?)))
        .collect()
}

const PIECES: &[&str] = &[
    "a",
    " ",
    "\n",
    "\n\n",
    "%",
    "{",
    "}",
    "{{{{{{",
    "}}}}}}",
    "\\iftrue\\iftrue\\iftrue\\iftrue\\iftrue\\iftrue",
    "\\iffalse",
    "\\fi\\fi\\fi",
    "\\else",
    "\\relax ",
    "\\def\\x{Q}",
    "\\x",
    "\\newcommand{\\y}[1]{<#1>}",
    "\\y{v}",
    "\\begingroup",
    "\\endgroup",
    "\\global",
    "\\edef\\z{",
    "\\csname",
    "\\endcsname",
    "#",
    "\\par",
    "\\label{k}",
    "\\stepcounter{section}",
    "$x^2$",
    "\\section{S}",
    "\\begin{itemize}\\item ",
    "\\end{itemize}",
    "\\end{document}",
    "\\endinput",
    // Stops: the step limit, the output limit, main memory, input stack.
    "\\def\\r{\\r}\\r",
    "\\def\\o{xy\\o}\\o",
    "\\def\\w#1{\\w{#1#1}}\\w x",
    "\\def\\s{\\s x}\\s",
    // Leaves the invocation as the last origin at the next safe point.
    "\\def\\d#1 {[#1]}\\d ab\n",
];

fn floor_boundary(s: &str, mut i: usize) -> usize {
    i = i.min(s.len());
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn random_edit(rng: &mut Rng, src: &str) -> Edit {
    let start = floor_boundary(src, rng.below(src.len() + 1));
    match rng.below(6) {
        0 => Edit { start, end: floor_boundary(src, start + rng.below(40)), replacement: String::new() },
        1 => {
            // Duplicate a range after itself.
            let end = floor_boundary(src, start + rng.below(120));
            Edit { start: end, end, replacement: src[start..end].repeat(1 + rng.below(3)) }
        }
        2 => Edit { start, end: floor_boundary(src, start + rng.below(4)), replacement: PIECES[rng.below(PIECES.len())].into() },
        _ => Edit { start, end: start, replacement: PIECES[rng.below(PIECES.len())].into() },
    }
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// Random edit sequences over the fuzz seeds with small limits. Each seed gets
/// limits drawn around its own step and output totals, so edits keep crossing
/// them, and nesting limits of a few levels.
///
/// `FLASHTEX_INC_STOP_EDITS` (default 6) sets the edits per seed and
/// `FLASHTEX_INC_STOP_SEED` the PRNG seed.
#[test]
fn incremental_matches_full_on_fuzz_seeds_with_small_limits() {
    let seeds = seeds();
    assert!(seeds.len() > 100, "found only {} seeds", seeds.len());
    let edits = env_u64("FLASHTEX_INC_STOP_EDITS", 6) as usize;
    let prng = env_u64("FLASHTEX_INC_STOP_SEED", 0x5EED_1A57);
    let (mut stops, mut nesting, mut converged, mut checked) = (0usize, 0usize, 0usize, 0usize);
    for (n, (name, text)) in seeds.iter().enumerate() {
        let mut rng = Rng((prng ^ (n as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)) | 1);
        let natural = full(text, Limits { max_expansion_steps: 60_000, max_output_tokens: 60_000, ..Limits::default() });
        let steps = natural.steps.max(20);
        let out = natural.tokens.len().max(20) as u64;
        let limits = Limits {
            max_expansion_steps: match rng.below(3) {
                0 => 200 + rng.below(5_000) as u64,
                _ => steps * (70 + rng.below(40) as u64) / 100,
            },
            max_output_tokens: match rng.below(3) {
                0 => out * (70 + rng.below(40) as u64) / 100,
                _ => 60_000,
            },
            max_group_depth: 2 + rng.below(5) as u32,
            max_conditional_depth: 2 + rng.below(5) as u32,
        };
        let interval = [8, 32, 128, 512][rng.below(4)];
        let mut inc = IncrementalExpander::with_options(text, limits, interval);
        assert_same(&inc, limits, &|| format!("{name}: initial run, {limits:?} interval {interval}"));
        for i in 0..edits {
            let edit = random_edit(&mut rng, inc.source());
            let stats = inc.edit(&edit);
            converged += stats.converged_at.is_some() as usize;
            checked += 1;
            let messages = || inc.diagnostics().iter().map(|d| d.message.as_str());
            if messages().any(|m| {
                m.starts_with("expansion step limit exceeded")
                    || m == "output token limit exceeded"
                    || m.starts_with("TeX capacity exceeded")
            }) {
                stops += 1;
            }
            if messages().any(|m| m.ends_with("nesting limit exceeded")) {
                nesting += 1;
            }
            assert_same(&inc, limits, &|| format!("{name}: edit #{i} {edit:?}, {limits:?} interval {interval}"));
        }
    }
    eprintln!(
        "{} seeds, {checked} edits: {stops} ended at a stop, {nesting} went past a nesting limit, {converged} converged",
        seeds.len()
    );
    assert!(stops > checked / 10, "too few edits reached a stop ({stops}/{checked})");
    assert!(converged > 0);
}
