//! Deterministic mutation fuzzer for the compiler (stable Rust, no deps).
//!
//! Shared by the `#[ignore]` integration test `tests/fuzz_compile.rs` and
//! `cargo run --release --example fuzz_compile`. Every case is a pure
//! function of `(rng seed, case index)`, so any finding replays exactly.
//!
//! Isolation: a stack overflow aborts the whole process and a hung thread
//! cannot be killed, so cases run in worker *processes* (the same binary,
//! re-executed). A worker announces each case on stdout before running it;
//! when it dies (overflow) or reports a hang and exits, the supervisor knows
//! exactly which case did it and restarts a worker after that case.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use flashtex_compiler::incremental::compile_full_project;
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::SourceDocument;

/// The entry document's path in every case; `\input{main}` names itself.
pub const ENTRY: &str = "main.tex";
/// Stack for a case thread: the size of a macOS/Linux main thread, where the
/// CLI (`flashtex check`) runs the compile. `FLASHTEX_FUZZ_STACK_KB` overrides.
pub const CASE_STACK: usize = 8 * 1024 * 1024;

// ---------------------------------------------------------------- RNG

pub struct Rng(u64);

impl Rng {
    /// splitmix64 over `seed` and `index`: independent streams per case.
    pub fn for_case(seed: u64, index: u64) -> Rng {
        let mut z = seed ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Rng((z ^ (z >> 31)) | 1)
    }
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
    pub fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

// ---------------------------------------------------------------- seeds

pub struct Seed {
    pub name: String,
    pub text: String,
}

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every UTF-8 `.tex` under `fixtures/`, `crates/*/tests/` and
/// `crates/*/oracle/`, in sorted
/// order (never `vendor/` or a `target*` directory).
pub fn load_seeds(root: &Path) -> Vec<Seed> {
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
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path).ok()?;
            let name = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            Some(Seed { name, text })
        })
        .collect()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
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

// ---------------------------------------------------------------- cases

#[derive(Clone, Debug)]
pub struct Case {
    /// `(path, text)`; the first is [`ENTRY`].
    pub documents: Vec<(String, String)>,
    pub seed_name: String,
    pub mutations: Vec<&'static str>,
}

impl Case {
    pub fn from_text(text: String) -> Case {
        Case {
            documents: vec![(ENTRY.into(), text)],
            seed_name: "file".into(),
            mutations: vec![],
        }
    }
    pub fn main(&self) -> &str {
        &self.documents[0].1
    }
}

fn floor_boundary(text: &str, mut at: usize) -> usize {
    at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

fn random_offset(rng: &mut Rng, text: &str) -> usize {
    floor_boundary(text, rng.below(text.len() + 1))
}

/// Byte ranges of the structural tokens the mutations delete or duplicate.
fn structural_tokens(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' | b'}' | b'[' | b']' | b'$' => out.push((i, i + 1)),
            b'\\' => {
                let rest = &text[i..];
                if rest.starts_with("\\begin{") || rest.starts_with("\\end{") {
                    if let Some(close) = rest.find('}') {
                        out.push((i, i + close + 1));
                        i += close + 1;
                        continue;
                    }
                } else if rest.starts_with("\\begin") || rest.starts_with("\\end") {
                    let len = if rest.starts_with("\\begin") { 6 } else { 4 };
                    out.push((i, i + len));
                }
                // Skip the escaped character so `\{` is not a brace.
                i += 1;
                if i < bytes.len() && !bytes[i].is_ascii_alphabetic() {
                    i += text[i..].chars().next().map_or(1, char::len_utf8);
                    continue;
                }
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    out
}

const CONDITIONALS: &[&str] = &[
    "\\iftrue ",
    "\\iffalse ",
    "\\fi ",
    "\\else ",
    "\\ifx\\a\\b ",
    "\\ifnum1<2 ",
    "\\ifdim1pt<2pt ",
    "\\ifcase3 ",
    "\\or ",
    "\\ifcat a b",
    "\\if aa",
    "\\unless\\ifx ",
    "\\ifdefined ",
    "\\newif\\ifq \\ifq ",
    "\\ifmmode ",
    "\\ifhmode ",
    "\\csname iftrue\\endcsname ",
];

const RECURSIONS: &[&str] = &[
    "\\def\\a{\\a}\\a ",
    "\\def\\a{x\\a}\\a ",
    "\\def\\a{\\a x}\\a ",
    "\\def\\a#1{\\a{#1#1}}\\a x",
    "\\def\\a{\\b}\\def\\b{\\a}\\a ",
    "\\newcommand{\\a}{\\a}\\a ",
    "\\newcommand\\a[1]{\\a{#1}}\\a{x}",
    "\\renewcommand\\a{{\\a}}\\a ",
    "\\let\\a\\relax\\def\\a{\\a\\a}\\a ",
    "\\edef\\a{\\a}\\a ",
    "\\def\\a{\\expandafter\\a}\\a ",
    "\\def\\a{\\csname a\\endcsname}\\a ",
    "\\def\\a{\\begin{a}}\\newenvironment{a}{\\a}{}\\a ",
    "\\newenvironment{rec}{\\begin{rec}}{\\end{rec}}\\begin{rec}\\end{rec}",
    "\\def\\a{\\section{\\a}}\\a ",
    "\\def\\a{$\\a$}\\a ",
    "\\def\\a{\\iftrue\\a\\fi}\\a ",
    "\\def\\a{\\def\\a{\\a}\\a}\\a ",
    "\\def\\a{\\input{main}}\\a ",
    "\\DeclareRobustCommand\\a{\\a}\\a ",
    "\\providecommand\\a{\\a}\\a ",
    "\\def\\a{\\a#}\\a ",
];

const CHARS: &[&str] = &[
    "\\char\"D800 ",
    "\\char\"DFFF ",
    "\\char55296 ",
    "\\char\"110000 ",
    "\\char-1 ",
    "\\char\"FFFFFFFF ",
    "\\char99999999999999999999 ",
    "\\symbol{\"D800}",
    "\\symbol{-5}",
    "^^^^d800",
    "^^^^^^10ffff",
    "^^^^^^110000",
    "\\char`\\",
    "\\char'777777777 ",
    "\\char\"",
    "\\Uchar\"D800 ",
    "\\mathchar\"FFFFFF ",
    "\\delimiter\"FFFFFFFFF ",
    "\\catcode`\\{=12 ",
    "\\catcode 300=1 ",
    "\\lccode\"D800=1 ",
    "$\\char\"D800$",
    "\\textsuperscript{\\char\"DC00}",
];

const INPUTS: &[&str] = &[
    "\\input{main}",
    "\\input{main.tex}",
    "\\input main ",
    "\\include{main}",
    "\\input{sub}",
    "\\subfile{main}",
    "\\include{sub}",
    "\\InputIfFileExists{main}{}{}",
    "\\input{./main}",
];

const NESTERS: &[(&str, &str)] = &[
    ("{", "}"),
    ("\\begin{itemize}\\item ", "\\end{itemize}"),
    ("\\begin{quote}", "\\end{quote}"),
    ("$\\left(", "\\right)$"),
    ("\\left(", "\\right)"),
    ("\\textbf{", "}"),
    ("[", "]"),
    ("\\begingroup ", "\\endgroup "),
    ("\\sqrt{", "}"),
    ("\\footnote{", "}"),
    ("\\begin{minipage}{1cm}", "\\end{minipage}"),
    ("\\fbox{", "}"),
    ("\\begin{tabular}{c}", "\\end{tabular}"),
];

pub fn generate(seeds: &[Seed], rng_seed: u64, index: u64) -> Case {
    let mut rng = Rng::for_case(rng_seed, index);
    let seed = rng.pick(seeds);
    let mut text = seed.text.clone();
    let mut documents_extra: Vec<(String, String)> = Vec::new();
    let mut mutations = Vec::new();
    let count = 1 + rng.below(4);
    for _ in 0..count {
        let kind = rng.below(12);
        match kind {
            0 => {
                mutations.push("truncate");
                let at = random_offset(&mut rng, &text);
                text.truncate(at);
            }
            1 | 2 => {
                let tokens = structural_tokens(&text);
                if tokens.is_empty() {
                    continue;
                }
                let deletions = 1 + rng.below(8);
                if kind == 1 {
                    mutations.push("delete-token");
                } else {
                    mutations.push("duplicate-token");
                }
                // Apply right-to-left so earlier ranges stay valid.
                let mut chosen: Vec<(usize, usize)> =
                    (0..deletions).map(|_| *rng.pick(&tokens)).collect();
                chosen.sort();
                chosen.dedup();
                for (a, b) in chosen.into_iter().rev() {
                    if kind == 1 {
                        text.replace_range(a..b, "");
                    } else {
                        let piece = text[a..b].to_string();
                        let copies = if rng.chance(10) { 1 + rng.below(50) } else { 1 };
                        text.insert_str(b, &piece.repeat(copies));
                    }
                }
            }
            3 => {
                mutations.push("splice-lines");
                let other = rng.pick(seeds);
                let lines: Vec<&str> = other.text.lines().collect();
                if lines.is_empty() {
                    continue;
                }
                let start = rng.below(lines.len());
                let end = (start + 1 + rng.below(20)).min(lines.len());
                let chunk = lines[start..end].join("\n") + "\n";
                let at = random_offset(&mut rng, &text);
                text.insert_str(at, &chunk);
            }
            4 => {
                mutations.push("deep-nesting");
                let (open, close) = *rng.pick(NESTERS);
                let depth = if rng.chance(50) {
                    10_000
                } else {
                    50 + rng.below(3000)
                };
                let balanced = rng.chance(60);
                let mut piece = open.repeat(depth);
                piece.push('x');
                if balanced {
                    piece.push_str(&close.repeat(depth));
                }
                let at = random_offset(&mut rng, &text);
                text.insert_str(at, &piece);
            }
            5 => {
                mutations.push("long-control-sequence");
                let len = if rng.chance(50) {
                    100_000
                } else {
                    1 + rng.below(5000)
                };
                let letter = (b'a' + rng.below(26) as u8) as char;
                let mut piece = String::from("\\");
                piece.extend(std::iter::repeat(letter).take(len));
                if rng.chance(50) {
                    // Define it too, so the long name is looked up and stored.
                    piece = format!("\\def{piece}{{y}}{piece} ");
                }
                let at = random_offset(&mut rng, &text);
                text.insert_str(at, &piece);
            }
            6 => {
                mutations.push("conditional");
                for _ in 0..1 + rng.below(6) {
                    let piece = if rng.chance(5) {
                        rng.pick(CONDITIONALS).repeat(1 + rng.below(5000))
                    } else {
                        rng.pick(CONDITIONALS).to_string()
                    };
                    let at = random_offset(&mut rng, &text);
                    text.insert_str(at, &piece);
                }
            }
            7 => {
                mutations.push("recursion");
                let piece = *rng.pick(RECURSIONS);
                let at = random_offset(&mut rng, &text);
                text.insert_str(at, piece);
            }
            8 => {
                mutations.push("invalid-char");
                for _ in 0..1 + rng.below(3) {
                    let piece = *rng.pick(CHARS);
                    let at = random_offset(&mut rng, &text);
                    text.insert_str(at, piece);
                }
            }
            9 => {
                mutations.push("self-input");
                let piece = *rng.pick(INPUTS);
                let at = random_offset(&mut rng, &text);
                text.insert_str(at, piece);
                if documents_extra.is_empty() {
                    let sub = if rng.chance(50) {
                        "\\input{main}\n"
                    } else {
                        "x\\input{sub}\n"
                    };
                    documents_extra.push(("sub.tex".into(), sub.into()));
                }
            }
            10 => {
                mutations.push("delete-range");
                let a = random_offset(&mut rng, &text);
                let b = floor_boundary(&text, a + rng.below(200));
                text.replace_range(a..b, "");
            }
            _ => {
                mutations.push("duplicate-range");
                let a = random_offset(&mut rng, &text);
                let b = floor_boundary(&text, a + rng.below(400));
                let piece = text[a..b].to_string();
                let limit = if rng.chance(10) { 200 } else { 3 };
                let copies = 1 + rng.below(limit);
                text.insert_str(b, &piece.repeat(copies));
            }
        }
    }
    let mut documents = vec![(ENTRY.to_string(), text)];
    documents.extend(documents_extra);
    Case {
        documents,
        seed_name: seed.name.clone(),
        mutations,
    }
}

// ---------------------------------------------------------------- running

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    Ok,
    /// `location`, `message`.
    Panic(String, String),
    Hang,
    /// The process died: stack overflow or abort.
    Crash(String),
}

impl Outcome {
    /// The dedup key: panics by location and message with digits removed
    /// (indices and lengths vary between inputs hitting the same bug).
    pub fn signature(&self) -> String {
        match self {
            Outcome::Ok => "ok".into(),
            Outcome::Panic(loc, msg) => {
                let msg: String = msg
                    .chars()
                    .filter(|c| !c.is_ascii_digit())
                    .take(120)
                    .collect();
                format!("panic {loc} {msg}")
            }
            Outcome::Hang => "hang".into(),
            Outcome::Crash(what) => format!("crash {what}"),
        }
    }
}

static PANIC_SLOT: OnceLock<Mutex<Option<(String, String)>>> = OnceLock::new();

/// Records the panic location and message instead of printing them.
pub fn install_quiet_panic_hook() {
    let slot = PANIC_SLOT.get_or_init(|| Mutex::new(None));
    panic::set_hook(Box::new(move |info| {
        let loc = info.location().map_or("?".into(), |l| {
            format!("{}:{}:{}", l.file(), l.line(), l.column())
        });
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "<non-string panic>".into()
        };
        if let Ok(mut guard) = slot.lock() {
            guard.get_or_insert((loc, msg.replace(['\n', '\t'], " ")));
        }
    }));
}

pub fn compile_case(case: &Case) {
    let documents: Vec<SourceDocument<'_>> = case
        .documents
        .iter()
        .map(|(path, text)| SourceDocument { path, text })
        .collect();
    let out = compile_full_project(&documents, ENTRY, LayoutConstraints::default());
    // Touch every span, as a consumer would.
    for page in &out.pages {
        for item in &page.items {
            let text = &case.documents[item.span.document.0.min(case.documents.len() - 1)].1;
            let _ = text.get(item.span.start..item.span.end);
        }
    }
    std::hint::black_box(out);
}

/// One case on a fresh thread with [`CASE_STACK`], under `catch_unwind`, with
/// a watchdog. On `Hang` the thread is still running: the caller must exit.
pub fn run_case(case: Case, timeout: Duration) -> Outcome {
    let slot = PANIC_SLOT.get_or_init(|| Mutex::new(None));
    *slot.lock().unwrap() = None;
    let (tx, rx) = mpsc::channel();
    let stack = std::env::var("FLASHTEX_FUZZ_STACK_KB")
        .ok()
        .and_then(|kb| kb.parse::<usize>().ok())
        .map_or(CASE_STACK, |kb| kb * 1024);
    let spawned = std::thread::Builder::new()
        .stack_size(stack)
        .spawn(move || {
            let result = panic::catch_unwind(AssertUnwindSafe(|| compile_case(&case)));
            let _ = tx.send(result.is_ok());
        });
    if spawned.is_err() {
        return Outcome::Crash("thread spawn failed".into());
    }
    match rx.recv_timeout(timeout) {
        Ok(true) => Outcome::Ok,
        Ok(false) | Err(mpsc::RecvTimeoutError::Disconnected) => {
            let (loc, msg) = slot
                .lock()
                .unwrap()
                .take()
                .unwrap_or(("?".into(), "?".into()));
            Outcome::Panic(loc, msg)
        }
        Err(mpsc::RecvTimeoutError::Timeout) => Outcome::Hang,
    }
}

// ---------------------------------------------------------------- worker / supervisor

pub struct Config {
    pub rng_seed: u64,
    pub cases: u64,
    pub jobs: usize,
    pub timeout: Duration,
    pub out_dir: PathBuf,
}

/// Worker loop: runs `[start, end)`, one protocol line per event. Exits the
/// process after a hang (the stuck thread cannot be stopped).
pub fn worker(seeds: &[Seed], rng_seed: u64, start: u64, end: u64, timeout: Duration) {
    install_quiet_panic_hook();
    let stdout = std::io::stdout();
    for index in start..end {
        let case = generate(seeds, rng_seed, index);
        {
            let mut out = stdout.lock();
            let _ = writeln!(out, "START\t{index}");
            let _ = out.flush();
        }
        let outcome = run_case(case, timeout);
        let mut out = stdout.lock();
        match &outcome {
            Outcome::Ok => {
                let _ = writeln!(out, "OK\t{index}");
            }
            Outcome::Panic(loc, msg) => {
                let _ = writeln!(out, "PANIC\t{index}\t{loc}\t{msg}");
            }
            Outcome::Hang => {
                let _ = writeln!(out, "HANG\t{index}");
                let _ = out.flush();
                std::process::exit(3);
            }
            Outcome::Crash(what) => {
                let _ = writeln!(out, "CRASH\t{index}\t{what}");
            }
        }
        let _ = out.flush();
    }
}

pub struct Finding {
    pub signature: String,
    pub first_case: u64,
    pub count: u64,
}

pub struct Report {
    pub cases_run: u64,
    pub findings: BTreeMap<String, Finding>,
}

/// Runs `config.cases` cases across `config.jobs` worker processes built by
/// `worker_command(start, end)`, restarting after crashes and hangs.
pub fn supervise(
    seeds: &[Seed],
    config: &Config,
    worker_command: &(dyn Fn(u64, u64) -> Command + Sync),
) -> Report {
    let chunk = 500u64;
    let next = Arc::new(Mutex::new(0u64));
    let report = Arc::new(Mutex::new(Report {
        cases_run: 0,
        findings: BTreeMap::new(),
    }));
    let _ = std::fs::create_dir_all(&config.out_dir);
    std::thread::scope(|scope| {
        for _ in 0..config.jobs {
            let next = Arc::clone(&next);
            let report = Arc::clone(&report);
            scope.spawn(move || loop {
                let (mut start, end) = {
                    let mut n = next.lock().unwrap();
                    if *n >= config.cases {
                        return;
                    }
                    let s = *n;
                    *n = (s + chunk).min(config.cases);
                    (s, *n)
                };
                while start < end {
                    let mut child = worker_command(start, end)
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn()
                        .expect("spawn worker");
                    let stderr = child.stderr.take().unwrap();
                    let stderr_tail = std::thread::spawn(move || {
                        let mut tail = String::new();
                        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                            if line.contains("overflow")
                                || line.contains("abort")
                                || line.contains("fatal")
                            {
                                tail = line;
                            }
                        }
                        tail
                    });
                    let mut open: Option<u64> = None;
                    let mut resume = end;
                    for line in BufReader::new(child.stdout.take().unwrap())
                        .lines()
                        .map_while(Result::ok)
                    {
                        let fields: Vec<&str> = line.splitn(4, '\t').collect();
                        let Some(index) = fields.get(1).and_then(|s| s.parse::<u64>().ok()) else {
                            continue;
                        };
                        let outcome = match fields[0] {
                            "START" => {
                                open = Some(index);
                                continue;
                            }
                            "OK" => Outcome::Ok,
                            "PANIC" => Outcome::Panic(
                                fields.get(2).unwrap_or(&"?").to_string(),
                                fields.get(3).unwrap_or(&"?").to_string(),
                            ),
                            "HANG" => Outcome::Hang,
                            _ => Outcome::Crash(fields.get(2).unwrap_or(&"?").to_string()),
                        };
                        open = None;
                        record(&report, seeds, config, index, &outcome);
                        if outcome == Outcome::Hang {
                            resume = index + 1;
                        }
                    }
                    let status = child.wait().expect("wait worker");
                    let tail = stderr_tail.join().unwrap_or_default();
                    if let Some(index) = open {
                        let what = if tail.contains("stack overflow")
                            || tail.contains("overflowed its stack")
                        {
                            "stack overflow".to_string()
                        } else {
                            format!("{status} {tail}")
                        };
                        record(&report, seeds, config, index, &Outcome::Crash(what));
                        resume = index + 1;
                    }
                    start = if status.success() && open.is_none() && resume == end {
                        end
                    } else {
                        resume
                    };
                }
            });
        }
    });
    Arc::try_unwrap(report).ok().unwrap().into_inner().unwrap()
}

fn record(report: &Mutex<Report>, seeds: &[Seed], config: &Config, index: u64, outcome: &Outcome) {
    let mut report = report.lock().unwrap();
    report.cases_run += 1;
    if *outcome == Outcome::Ok {
        return;
    }
    let signature = outcome.signature();
    let entry = report
        .findings
        .entry(signature.clone())
        .or_insert_with(|| Finding {
            signature: signature.clone(),
            first_case: index,
            count: 0,
        });
    entry.count += 1;
    let first = index < entry.first_case || entry.count == 1;
    entry.first_case = entry.first_case.min(index);
    // Hangs and crashes share one signature whatever their cause, so keep
    // a sample of inputs to triage, not only the first.
    let sample = entry.count <= 50 && matches!(outcome, Outcome::Hang | Outcome::Crash(_));
    if first || sample {
        let case = generate(seeds, config.rng_seed, index);
        let name = format!("case-{index}.tex");
        if !first {
            let _ = std::fs::write(config.out_dir.join(&name), case.main());
            if let Some((_, sub)) = case.documents.get(1) {
                let _ = std::fs::write(config.out_dir.join(format!("case-{index}.sub.tex")), sub);
            }
            return;
        }
        let _ = std::fs::write(config.out_dir.join(&name), case.main());
        if let Some((_, sub)) = case.documents.get(1) {
            let _ = std::fs::write(config.out_dir.join(format!("case-{index}.sub.tex")), sub);
        }
        eprintln!(
            "finding: {signature}\n  case {index} seed {} mutations {:?} -> {}",
            case.seed_name,
            case.mutations,
            config.out_dir.join(&name).display()
        );
    }
}

pub fn print_report(report: &Report) {
    println!(
        "fuzz: {} cases, {} unique findings",
        report.cases_run,
        report.findings.len()
    );
    for finding in report.findings.values() {
        println!(
            "  {:>6}x first case {:>6}: {}",
            finding.count, finding.first_case, finding.signature
        );
    }
}

// ---------------------------------------------------------------- minimising

/// Classic ddmin over lines, then characters, keeping `interesting` true.
pub fn minimise(text: &str, interesting: &mut dyn FnMut(&str) -> bool) -> String {
    let mut current: Vec<String> = text.split_inclusive('\n').map(str::to_string).collect();
    current = ddmin(current, interesting);
    let joined: String = current.concat();
    let chars: Vec<String> = joined.chars().map(|c| c.to_string()).collect();
    ddmin(chars, interesting).concat()
}

fn ddmin(mut items: Vec<String>, interesting: &mut dyn FnMut(&str) -> bool) -> Vec<String> {
    let mut granularity = 2usize;
    while items.len() >= 2 {
        let chunk = items.len().div_ceil(granularity);
        let mut reduced = false;
        let mut start = 0;
        while start < items.len() {
            let end = (start + chunk).min(items.len());
            let candidate: Vec<String> = items[..start]
                .iter()
                .chain(&items[end..])
                .cloned()
                .collect();
            if !candidate.is_empty() && interesting(&candidate.concat()) {
                items = candidate;
                reduced = true;
                granularity = (granularity - 1).max(2);
                break;
            }
            start = end;
        }
        if !reduced {
            if granularity >= items.len() {
                break;
            }
            granularity = (granularity * 2).min(items.len());
        }
    }
    items
}
