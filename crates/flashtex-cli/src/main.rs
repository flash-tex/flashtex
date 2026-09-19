//! `flashtex`: the FlashTeX engine on the command line.
//!
//! ```text
//! flashtex build [<main.tex>|<dir>] [-o out.pdf] [--project-root DIR] [--font-dir DIR]...
//!                [--v2 out.json] [--timing] [--strict] [--json] [-j N]
//! flashtex check [<main.tex>|<dir>] [--json] [--strict] [--fix] [--dry-run] [--project-root DIR] [--font-dir DIR]...
//! flashtex watch [<main.tex>|<dir>] [-o out.pdf] [--project-root DIR] [--font-dir DIR]... [--interval MS]
//! flashtex manifest init [<main.tex>|<dir>] [--force] | show [<main.tex>|<dir>] [--json]
//! flashtex supported [--json|--md|--coverage]
//! flashtex worker [--font-dir DIR]... [--project-root DIR] [--v2 out.json] [--pdf out.pdf] [--timing]
//! flashtex fonts [--font-dir DIR]... [--json]
//! flashtex install-cli [DIR]
//! flashtex --version | --help
//! ```
//!
//! The engine is linked in (`flashtex-render-pipeline`), never spawned;
//! the PDF is the exact route (`flashtex_pdf::v2` + `exact`, real embedded
//! font subsets, images, links), the same bytes the Mac app's Export PDF
//! produces. Exit status: 0 when the document rendered (`ok`, or
//! `recovered` unless `--strict`), 1 when it failed, 2 for a usage error.
//! Diagnostics go to stderr, rustc-style with a source excerpt when stderr
//! is a terminal and as `file:line:col: severity[code] message` otherwise
//! (`--diagnostics`), then one summary line. See docs/user/compiler.md.
//!
//! The entry may be a directory (or omitted for the current one): then a
//! `flashtex.toml` names it (`[project] entry`), or the directory's only
//! `.tex` file is it. A manifest also adds `texinputs` directories to the
//! closure and may set the output directory (docs/user/project-manifest.md);
//! without one, nothing differs from naming the file.

mod compile;
mod fix;
mod project;
mod report;
mod requestdate;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use flashtex_render_pipeline::fonts::{self, Discovery};
use flashtex_render_pipeline::{FontSet, RenderOptions};

const EXIT_OK: i32 = 0;
const EXIT_FAILED: i32 = 1;
const EXIT_USAGE: i32 = 2;

const USAGE: &str = "\
flashtex — the FlashTeX LaTeX engine

usage:
  flashtex build [<main.tex>|<dir>] [-o out.pdf] [--project-root DIR] [--font-dir DIR]...
                 [--font ROLE=NAME]... [--v2 out.json] [--timing] [--verbose]
                 [--strict] [--json] [-j N]
  flashtex check [<main.tex>|<dir>] [--json] [--strict] [--fix] [--dry-run]
                 [--project-root DIR] [--font-dir DIR]...
  flashtex watch [<main.tex>|<dir>] [-o out.pdf] [--project-root DIR] [--font-dir DIR]...
                 [--interval MS] [--timing]
  flashtex manifest init [<main.tex>|<dir>] [--force]
  flashtex manifest show [<main.tex>|<dir>] [--json]
  flashtex supported [--json|--md|--coverage]
  flashtex worker [--font-dir DIR]... [--project-root DIR] [--v2 out.json]
                  [--pdf out.pdf] [--timing]
  flashtex fonts [--font-dir DIR]... [--json]
  flashtex install-cli [DIR]
  flashtex --version | --help

commands:
  build      typeset a project (entry file + its \\input/\\include closure) to a
             PDF through the exact route: embedded font subsets, images, links
  check      diagnostics only, no output files (`--json`: flashtex-check/1;
             `--fix` applies suggestions in place, `--dry-run` prints the diff)
  watch      rebuild whenever a file of the project closure changes; Ctrl-C stops
  manifest   `init` writes a commented flashtex.toml next to the entry; `show`
             prints the manifest that governs the entry (defaults when absent)
  supported  the implemented-LaTeX inventory and coverage of the linked compiler
  worker     the runtime-v1 JSON Lines worker the IDE speaks (stdin/stdout)
  fonts      the font and TFM directories this binary resolves, in search order
  install-cli symlink this binary into DIR (default /usr/local/bin)

options:
  -o, --output FILE    the PDF to write (default: <main>.pdf next to the entry, or
                       in the manifest's `[project] output` directory when set)
  --project-root DIR   the directory includes and images resolve under (default:
                       the manifest's directory, else the entry file's); nothing
                       outside it is read except the manifest's `texinputs`
  --font-dir DIR       an extra font directory, probed first (repeatable)
  --font ROLE=NAME     the installed family for a role -- text, sans, mono or
                       math (repeatable); outranks the manifest's `[fonts]`,
                       the document's own \\setmainfont still wins
  --v2 FILE            also write the rendering-v2 display list envelope
  --timing             print render/PDF/total wall time to stderr
  -v, --verbose        also print the PDF route's notes (embedded fonts, widths)
  --strict             exit 1 when any error diagnostic was reported, even if
                       the document rendered (`recovered`)
  --json               (check/build) print the flashtex-check/1 report on stdout
  --fix                (check) apply each diagnostic suggestion to its source
                       span; overlapping edits and files that changed since
                       compile are skipped; then the check is re-run
  --dry-run            (check, with --fix) print a unified diff and write nothing
  --diagnostics STYLE  full (source excerpt and carets), short (one line each)
                       or json (= --json); default full on a terminal, else short
  --color WHEN         auto (default; off when NO_COLOR is set), always, never
  -j, --jobs N         accepted for compatibility; the engine is single-threaded
  --class-options OPTS class options assumed when the source has no \\documentclass
                       (default `12pt`)
  --secnumdepth N      section numbering depth when the source does not set it

exit status: 0 rendered (ok/recovered), 1 failed (or recovered with --strict),
             2 usage error / unreadable input
environment: FLASHTEX_FONT_DIRS, FLASHTEX_TFM_DIRS, FLASHTEX_LM_DIR (colon separated)
entry:       a .tex file, or a directory whose flashtex.toml names `[project] entry`
             (or that holds exactly one .tex file); omitted means the current directory
";

/// `flashtex --version`: the release version and the Git revision it was
/// built from (both from `build.rs`). `FLASHTEX_VERSION` in the build
/// environment is the release tag; a plain checkout falls back to the crate
/// version.
pub fn version_string() -> String {
    format!("flashtex {} ({})", env!("FLASHTEX_VERSION"), env!("FLASHTEX_GIT_SHA"))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        None | Some("-h") | Some("--help") | Some("help") => {
            print!("{USAGE}");
            if args.is_empty() {
                EXIT_USAGE
            } else {
                EXIT_OK
            }
        }
        Some("-V") | Some("--version") | Some("version") => {
            println!("{}", version_string());
            EXIT_OK
        }
        Some("build") => run(&args[1..], Mode::Build),
        Some("check") => run(&args[1..], Mode::Check),
        Some("watch") => run(&args[1..], Mode::Watch),
        Some("manifest") => manifest(&args[1..]),
        Some("supported") => supported(&args[1..]),
        Some("worker") => worker(&args[1..]),
        Some("fonts") => fonts_cmd(&args[1..]),
        Some("install-cli") => install_cli(&args[1..]),
        Some(other) => usage_error(&format!("unknown command {other:?}")),
    };
    std::process::exit(code);
}

fn usage_error(msg: &str) -> i32 {
    eprintln!("flashtex: {msg}\n\nrun `flashtex --help` for usage");
    EXIT_USAGE
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Build,
    Check,
    Watch,
}

/// Options shared by build/check/watch.
struct Common {
    /// The entry file or directory as typed; `None` is the current directory.
    main: Option<PathBuf>,
    output: Option<PathBuf>,
    project_root: Option<PathBuf>,
    font_dirs: Vec<PathBuf>,
    v2: Option<PathBuf>,
    timing: bool,
    verbose: bool,
    strict: bool,
    json: bool,
    /// `None`: full when stderr is a terminal, short otherwise.
    diagnostics: Option<report::Style>,
    color: Option<bool>,
    /// `check --fix`: apply suggestions, then re-run the check.
    fix: bool,
    dry_run: bool,
    interval_ms: u64,
    render: RenderOptions,
}

fn parse_common(args: &[String], mode: Mode) -> Result<Common, String> {
    let mut c = Common {
        main: None,
        output: None,
        project_root: None,
        font_dirs: Vec::new(),
        v2: None,
        timing: false,
        verbose: false,
        strict: false,
        json: false,
        diagnostics: None,
        color: None,
        fix: false,
        dry_run: false,
        interval_ms: 250,
        render: RenderOptions::default(),
    };
    let mut main: Option<PathBuf> = None;
    let mut explicit_date: Option<String> = None;
    let mut i = 0;
    let value = |i: &mut usize, flag: &str| -> Result<String, String> {
        *i += 1;
        args.get(*i).cloned().ok_or_else(|| format!("{flag} needs a value"))
    };
    while i < args.len() {
        // `--diagnostics=full` is `--diagnostics full`.
        let (a, inline) = match args[i].split_once('=') {
            Some((flag, v)) if flag == "--diagnostics" || flag == "--color" => (flag, Some(v.to_string())),
            _ => (args[i].as_str(), None),
        };
        match a {
            "-o" | "--output" => c.output = Some(PathBuf::from(value(&mut i, a)?)),
            "--project-root" => c.project_root = Some(PathBuf::from(value(&mut i, a)?)),
            "--font-dir" => c.font_dirs.push(PathBuf::from(value(&mut i, a)?)),
            "--v2" => c.v2 = Some(PathBuf::from(value(&mut i, a)?)),
            // What `\today` renders. The CLI is the caller, so it is the one
            // component allowed to read the clock; the engine never does.
            // protocol/proposals/runtime-v1-request-date.md
            "--date" => {
                let raw = value(&mut i, a)?;
                explicit_date = Some(raw);
            }
            "--timing" => c.timing = true,
            "-v" | "--verbose" => c.verbose = true,
            "--strict" => c.strict = true,
            "--json" => c.json = true,
            "--fix" => c.fix = true,
            "--dry-run" => c.dry_run = true,
            "--diagnostics" => match inline.map_or_else(|| value(&mut i, a), Ok)?.as_str() {
                "full" => c.diagnostics = Some(report::Style::Full),
                "short" => c.diagnostics = Some(report::Style::Short),
                "json" => c.json = true,
                v => return Err(format!("--diagnostics is full, short or json, got {v:?}")),
            },
            "--color" => match inline.map_or_else(|| value(&mut i, a), Ok)?.as_str() {
                "auto" => c.color = None,
                "always" => c.color = Some(true),
                "never" => c.color = Some(false),
                v => return Err(format!("--color is auto, always or never, got {v:?}")),
            },
            "-j" | "--jobs" => {
                let n = value(&mut i, a)?;
                n.parse::<usize>().map_err(|_| format!("{a} needs a number, got {n:?}"))?;
            }
            "--interval" => {
                let n = value(&mut i, a)?;
                c.interval_ms = n.parse::<u64>().map_err(|_| format!("{a} needs milliseconds, got {n:?}"))?.max(20);
            }
            "--class-options" => c.render.default_class_options = value(&mut i, a)?,
            // `--font text=NAME` (repeatable; roles text, sans, mono, math):
            // the same slots as the manifest's `[fonts]`, outranking it.
            "--font" => {
                let raw = value(&mut i, a)?;
                let (role, name) = raw.split_once('=').ok_or_else(|| format!("{a} needs ROLE=NAME (roles: text, sans, mono, math), got {raw:?}"))?;
                let name = name.trim();
                if name.is_empty() {
                    return Err(format!("{a} {role}= needs a family name"));
                }
                let fonts = c.render.fonts.get_or_insert_with(Default::default);
                match role.trim() {
                    "text" => fonts.text = Some(name.to_string()),
                    "sans" => fonts.sans = Some(name.to_string()),
                    "mono" => fonts.mono = Some(name.to_string()),
                    "math" => fonts.math = Some(name.to_string()),
                    r => return Err(format!("{a} role must be text, sans, mono or math, got {r:?}")),
                }
            }
            "--secnumdepth" => {
                let n = value(&mut i, a)?;
                c.render.default_secnumdepth = n.parse::<u8>().map_err(|_| format!("{a} needs a small number, got {n:?}"))?;
            }
            "-h" | "--help" => return Err("help".into()),
            s if s.starts_with('-') && s.len() > 1 => return Err(format!("unknown option {s:?}")),
            s => {
                if main.replace(PathBuf::from(s)).is_some() {
                    return Err("only one entry file is accepted; includes are resolved from it".into());
                }
            }
        }
        i += 1;
    }
    c.main = main;
    if mode == Mode::Check && (c.output.is_some() || c.v2.is_some()) {
        return Err("`check` writes no output files; use `build` for -o/--v2".into());
    }
    if c.dry_run && !c.fix {
        return Err("`--dry-run` needs `--fix`".into());
    }
    if mode != Mode::Check && (c.fix || c.dry_run) {
        return Err("`--fix` is only valid with `check`".into());
    }
    // `--date`, else SOURCE_DATE_EPOCH, else the clock. Resolved here, once per
    // invocation, so the engine receives a date and never reads a clock itself.
    c.render.today = requestdate::resolve(
        explicit_date.as_deref(),
        requestdate::source_date_epoch_from_env().as_deref(),
        requestdate::now_unix_seconds(),
    )
    .map_err(|e| e.0)?;
    Ok(c)
}

fn font_set(extra: &[PathBuf]) -> FontSet {
    FontSet::with_default_dirs(extra)
}

fn run(args: &[String], mode: Mode) -> i32 {
    let c = match parse_common(args, mode) {
        Ok(c) => c,
        Err(e) if e == "help" => {
            print!("{USAGE}");
            return EXIT_OK;
        }
        Err(e) => return usage_error(&e),
    };
    let fonts = font_set(&c.font_dirs);
    match mode {
        Mode::Build | Mode::Check => match build_once(&c, &fonts, mode, 1) {
            Ok(code) => code,
            Err(e) => usage_error(&e),
        },
        Mode::Watch => watch(&c, &fonts),
    }
}

/// One build (or check). `Err` is a usage-level failure (exit 2).
fn build_once(c: &Common, fonts: &FontSet, mode: Mode, revision: u64) -> Result<i32, String> {
    let started = Instant::now();
    let input = project::resolve(c.main.as_deref())?;
    let mut project = project::load(&input, c.project_root.as_deref())?;
    let mut outcome = compile::compile(&project, fonts, &c.render, revision);
    let mut outputs: Vec<(&str, PathBuf)> = Vec::new();
    let mut pdf_ms = 0.0;
    let mut pdf_notes: Vec<String> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    if mode != Mode::Check {
        let pdf_path = c.output.clone().unwrap_or_else(|| default_pdf_path(&project));
        if outcome.status != "failed" {
            // The manifest's output directory is created on demand; an
            // explicit `-o` is the user's own path and is left alone, as before.
            if c.output.is_none() {
                if let Some(dir) = project.output_dir.as_deref() {
                    if let Err(e) = std::fs::create_dir_all(dir) {
                        failures.push(format!("cannot create output directory {}: {e}", dir.display()));
                    }
                }
            }
            let t = Instant::now();
            match compile::exact_pdf(&outcome, fonts, &project.root) {
                Ok(pdf) => {
                    pdf_ms = t.elapsed().as_secs_f64() * 1000.0;
                    pdf_notes = pdf.notes;
                    match compile::write_atomic(&pdf_path, &pdf.bytes) {
                        Ok(()) => outputs.push(("pdf", pdf_path.clone())),
                        Err(e) => failures.push(e),
                    }
                }
                Err(e) => failures.push(format!("pdf: {e}")),
            }
        }
        if let Some(v2) = &c.v2 {
            let text = compile::v2_json(&outcome, &project.entry);
            match compile::write_atomic(v2, text.as_bytes()) {
                Ok(()) => outputs.push(("v2", v2.clone())),
                Err(e) => failures.push(e),
            }
        }
        // A render that reported `ok`/`recovered` but then failed to write an
        // output file (the PDF above, or `--v2`) is not truthful as `ok`: no
        // usable output exists. Downgrade the status so the summary line,
        // `--json` and the exit code (below) all agree with what's on disk.
        if !failures.is_empty() && outcome.status != "failed" {
            outcome.status = "failed";
        }
    }
    let mut total_ms = started.elapsed().as_secs_f64() * 1000.0;

    // Diagnostics, then the summary, on stderr; the JSON report on stdout.
    let mut err = std::io::stderr().lock();
    let terminal = std::io::IsTerminal::is_terminal(&err);
    let style = c.diagnostics.unwrap_or(if terminal { report::Style::Full } else { report::Style::Short });
    let color = c.color.unwrap_or(terminal && std::env::var_os("NO_COLOR").is_none());
    let shown = match style {
        report::Style::Full => report::collapse_repeats(&outcome.diagnostics),
        report::Style::Short => outcome.diagnostics.clone(),
    };
    for d in &shown {
        match style {
            report::Style::Short => {
                let _ = writeln!(err, "{}", d.line_text());
            }
            report::Style::Full => {
                let text = project.documents.iter().find(|doc| doc.path == d.path).map(|doc| doc.text.as_str());
                let _ = writeln!(err, "{}", report::render_full(d, text, color));
            }
        }
    }
    if c.verbose {
        for n in &pdf_notes {
            let _ = writeln!(err, "flashtex: pdf: {n}");
        }
    }
    for f in &failures {
        let _ = writeln!(err, "flashtex: error: {f}");
    }
    // A build that failed only at the output stage (the PDF or `--v2`
    // write above) has no render diagnostic, so the summary counters would
    // read "0 errors" on a failed build. Count each failure as an error
    // diagnostic so the counters, `--json` and the exit code agree. Pushed
    // after the per-diagnostic printing above on purpose: the `flashtex:
    // error: ...` line just printed is already this failure's report, and
    // printing it again in diagnostic form would duplicate it.
    for f in &failures {
        outcome.diagnostics.push(compile::Diagnostic {
            path: project.entry.clone(),
            line: None,
            column: None,
            start_byte: None,
            end_byte: None,
            error: true,
            code: "output".to_string(),
            message: f.clone(),
            recovery: None,
            suggestion: None,
        });
    }
    let wrote = outputs
        .iter()
        .map(|(k, p)| format!("{k} {}", p.display()))
        .collect::<Vec<_>>()
        .join(", ");
    let arrow = if wrote.is_empty() { String::new() } else { format!(" -> {wrote}") };
    let _ = writeln!(
        err,
        "flashtex: {}: {}, {} page{}, {} error{}, {} warning{}{arrow}",
        project.entry,
        outcome.status,
        outcome.pages,
        plural(outcome.pages),
        outcome.errors(),
        plural(outcome.errors()),
        outcome.warnings(),
        plural(outcome.warnings()),
    );
    if c.timing {
        let _ = writeln!(
            err,
            "flashtex: timing: render {:.2} ms ({} pass{}), pdf {:.2} ms, total {:.2} ms",
            outcome.render_ms,
            outcome.passes,
            if outcome.passes == 1 { "" } else { "es" },
            pdf_ms,
            total_ms
        );
    }
    if c.fix {
        let planned = fix::plan(fix::collect_edits(&outcome.diagnostics, &project), &project);
        let applied = fix::apply(&project, planned, c.dry_run);
        for s in &applied.skipped {
            let _ = writeln!(err, "{}", s.line_text());
        }
        if c.dry_run {
            for d in &applied.diffs {
                let _ = write!(err, "{d}");
            }
        }
        let _ = writeln!(err, "{}", fix::summary_line(applied.issues, applied.files, applied.skipped.len()));
        if !c.dry_run {
            let re_started = Instant::now();
            project = project::load(&input, c.project_root.as_deref())?;
            outcome = compile::compile(&project, fonts, &c.render, revision);
            total_ms = re_started.elapsed().as_secs_f64() * 1000.0;
            let shown = match style {
                report::Style::Full => report::collapse_repeats(&outcome.diagnostics),
                report::Style::Short => outcome.diagnostics.clone(),
            };
            for d in &shown {
                match style {
                    report::Style::Short => {
                        let _ = writeln!(err, "{}", d.line_text());
                    }
                    report::Style::Full => {
                        let text = project.documents.iter().find(|doc| doc.path == d.path).map(|doc| doc.text.as_str());
                        let _ = writeln!(err, "{}", report::render_full(d, text, color));
                    }
                }
            }
            let _ = writeln!(
                err,
                "flashtex: {}: {}, {} page{}, {} error{}, {} warning{}",
                project.entry,
                outcome.status,
                outcome.pages,
                plural(outcome.pages),
                outcome.errors(),
                plural(outcome.errors()),
                outcome.warnings(),
                plural(outcome.warnings()),
            );
            if c.timing {
                let _ = writeln!(
                    err,
                    "flashtex: timing: render {:.2} ms ({} pass{}), pdf {:.2} ms, total {:.2} ms",
                    outcome.render_ms,
                    outcome.passes,
                    if outcome.passes == 1 { "" } else { "es" },
                    0.0,
                    total_ms
                );
            }
        }
    }
    if c.json {
        let refs: Vec<(&str, &Path)> = outputs.iter().map(|(k, p)| (*k, p.as_path())).collect();
        println!("{}", compile::report_json(&project, &outcome, &refs, total_ms));
    }
    Ok(if outcome.status == "failed" || !failures.is_empty() || (c.strict && outcome.errors() > 0) {
        EXIT_FAILED
    } else {
        EXIT_OK
    })
}

/// `<entry stem>.pdf` in the manifest's output directory when it sets one,
/// else next to the entry as the user named it (byte-identical to the
/// pre-manifest CLI when there is no manifest).
fn default_pdf_path(project: &project::Project) -> PathBuf {
    match &project.output_dir {
        Some(dir) => {
            let stem = Path::new(&project.entry).file_stem().and_then(|s| s.to_str()).unwrap_or("main");
            dir.join(format!("{stem}.pdf"))
        }
        None => compile::default_pdf_path(&project.entry_file),
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// `watch`: a polling loop over the project closure (path, length,
/// modification time of every file the graph reaches, re-discovered after
/// each build so new includes are picked up). Portable and dependency-free;
/// the interval is `--interval` (default 250 ms). Output files are written
/// atomically, so Ctrl-C (the default SIGINT disposition) never leaves a
/// torn PDF behind.
fn watch(c: &Common, fonts: &FontSet) -> i32 {
    let mut revision = 1u64;
    let (mut snapshot, root) = match project_snapshot(c) {
        Ok(s) => s,
        Err(e) => return usage_error(&e),
    };
    eprintln!("flashtex: watching {} file{} under {} (every {} ms; Ctrl-C stops)", snapshot.len(), plural(snapshot.len()), root_label(c, &root), c.interval_ms);
    let timed = Common { timing: true, ..clone_common(c) };
    if let Err(e) = build_once(&timed, fonts, Mode::Build, revision) {
        eprintln!("flashtex: {e}");
    }
    loop {
        std::thread::sleep(std::time::Duration::from_millis(c.interval_ms));
        let now = match project_snapshot(c) {
            Ok((s, _)) => s,
            Err(e) => {
                eprintln!("flashtex: {e}");
                continue;
            }
        };
        if now == snapshot {
            continue;
        }
        let changed: Vec<String> = now
            .iter()
            .filter(|(p, st)| snapshot.iter().find(|(q, _)| q == p).map_or(true, |(_, old)| old != st))
            .map(|(p, _)| p.clone())
            .chain(snapshot.iter().filter(|(p, _)| !now.iter().any(|(q, _)| q == p)).map(|(p, _)| p.clone()))
            .collect();
        snapshot = now;
        revision += 1;
        if std::io::IsTerminal::is_terminal(&std::io::stderr()) {
            // Clear the screen so the latest rebuild is the only one on it.
            eprint!("\x1b[2J\x1b[H");
        }
        eprintln!("flashtex: change in {} -> rebuild #{revision}", changed.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
        if let Err(e) = build_once(&timed, fonts, Mode::Build, revision) {
            eprintln!("flashtex: {e}");
        }
    }
}

fn clone_common(c: &Common) -> Common {
    Common {
        main: c.main.clone(),
        output: c.output.clone(),
        project_root: c.project_root.clone(),
        font_dirs: c.font_dirs.clone(),
        v2: c.v2.clone(),
        timing: c.timing,
        verbose: c.verbose,
        strict: c.strict,
        json: c.json,
        diagnostics: c.diagnostics,
        color: c.color,
        fix: c.fix,
        dry_run: c.dry_run,
        interval_ms: c.interval_ms,
        render: c.render.clone(),
    }
}

/// The root as the user would recognise it: what they typed when they
/// typed a path, else the resolved one (a manifest's directory).
fn root_label(c: &Common, resolved: &Path) -> String {
    c.project_root
        .clone()
        .or_else(|| c.main.as_deref().filter(|m| m.is_file()).and_then(|m| m.parent().map(Path::to_path_buf)))
        .map_or_else(|| resolved.display().to_string(), |p| p.display().to_string())
}

/// `(path, (length, mtime))` for every file in the closure — project-relative
/// for files under the root, absolute for the manifest and its outside
/// `texinputs` files — and the root they were resolved under.
fn project_snapshot(c: &Common) -> Result<(Vec<(String, (u64, Option<std::time::SystemTime>))>, PathBuf), String> {
    let input = project::resolve(c.main.as_deref())?;
    let project = project::load(&input, c.project_root.as_deref())?;
    let stat = |p: &Path| {
        let meta = std::fs::metadata(p).ok();
        (meta.as_ref().map_or(0, |m| m.len()), meta.and_then(|m| m.modified().ok()))
    };
    let mut files: Vec<(String, (u64, Option<std::time::SystemTime>))> =
        project.files.iter().map(|p| (p.clone(), stat(&project.root.join(p)))).collect();
    files.extend(project.outside_files.iter().map(|p| (p.display().to_string(), stat(p))));
    Ok((files, project.root))
}

/// `manifest init [<entry>|<dir>] [--force]` writes the commented template
/// next to the entry (refusing to overwrite unless `--force`); `manifest
/// show [<entry>|<dir>] [--json]` prints the manifest that governs the
/// entry — the defaults when there is none — and its warnings.
fn manifest(args: &[String]) -> i32 {
    use flashtex_project_manifest::{Manifest, TexInputLocation, FILE_NAME};
    let (sub, rest) = match args.first().map(String::as_str) {
        Some(s @ ("init" | "show")) => (s, &args[1..]),
        Some(other) => return usage_error(&format!("manifest: unknown subcommand {other:?} (use init or show)")),
        None => return usage_error("manifest needs a subcommand: init or show"),
    };
    let mut target: Option<PathBuf> = None;
    let mut force = false;
    let mut json_out = false;
    for a in rest {
        match a.as_str() {
            "--force" if sub == "init" => force = true,
            "--json" if sub == "show" => json_out = true,
            s if s.starts_with('-') => return usage_error(&format!("manifest {sub}: unknown option {s:?}")),
            s => {
                if target.replace(PathBuf::from(s)).is_some() {
                    return usage_error("manifest takes at most one entry file or directory");
                }
            }
        }
    }
    if sub == "init" {
        // The template names the actual entry: the file given, or the
        // directory's only .tex (the resolver's rule), else `main.tex`.
        let given = target.clone().unwrap_or_else(|| PathBuf::from("."));
        let (dir, entry) = if given.is_file() {
            let dir = given.parent().map(Path::to_path_buf).filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| PathBuf::from("."));
            (dir, given.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "main.tex".into()))
        } else if given.is_dir() {
            let entry = project::resolve(Some(&given))
                .ok()
                .and_then(|i| i.entry.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "main.tex".into());
            (given, entry)
        } else {
            return usage_error(&format!("{} is neither a file nor a directory", given.display()));
        };
        let path = dir.join(FILE_NAME);
        if force {
            if let Err(e) = std::fs::remove_file(&path) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    eprintln!("flashtex: cannot replace {}: {e}", path.display());
                    return EXIT_FAILED;
                }
            }
        }
        return match Manifest::write_template(&path, &entry) {
            Ok(()) => {
                println!("wrote {} (entry {entry:?})", path.display());
                EXIT_OK
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                eprintln!("flashtex: {} exists; `--force` replaces it", path.display());
                EXIT_FAILED
            }
            Err(e) => {
                eprintln!("flashtex: cannot write {}: {e}", path.display());
                EXIT_FAILED
            }
        };
    }
    let input = match project::resolve(target.as_deref()) {
        Ok(i) => i,
        Err(e) => return usage_error(&e),
    };
    let m = &input.manifest.manifest;
    let mdir = input.manifest_dir.clone().or_else(|| input.entry.parent().map(Path::to_path_buf)).unwrap_or_default();
    let texinputs = m.texinputs(&mdir);
    if json_out {
        use flashtex_compiler::json::{self, Value};
        let opt = |v: &Option<String>| v.clone().map_or(Value::Null, json::str_);
        let map = |m: &std::collections::BTreeMap<String, String>| {
            let mut o = Value::obj();
            for (k, v) in m {
                o.set(k, json::str_(v.clone()));
            }
            o
        };
        let mut o = Value::obj();
        o.set("schema", json::str_("flashtex-manifest/1"));
        o.set("entry", json::str_(input.entry.display().to_string()));
        o.set("path", input.manifest.found.as_ref().map_or(Value::Null, |p| json::str_(p.display().to_string())));
        o.set("exists", Value::Bool(input.manifest.found.is_some()));
        let mut project = Value::obj();
        project.set("entry", opt(&m.project.entry));
        project.set("texinputs", Value::Arr(m.project.texinputs.iter().map(|t| json::str_(t.clone())).collect()));
        project.set("output", opt(&m.project.output));
        let mut fonts = Value::obj();
        for (k, v) in [("text", &m.fonts.text), ("math", &m.fonts.math), ("mono", &m.fonts.mono), ("sans", &m.fonts.sans)] {
            fonts.set(k, opt(v));
        }
        let mut packages = Value::obj();
        packages.set("source", json::str_(m.packages.source.as_str().to_string()));
        packages.set("fetch", json::str_(m.packages.fetch.as_str().to_string()));
        packages.set("pin", map(&m.packages.pin));
        packages.set("path", map(&m.packages.path));
        let library = m.library.as_ref().map_or(Value::Null, |l| {
            let mut o = Value::obj();
            o.set("name", json::str_(l.name.clone()));
            o
        });
        let mut manifest = Value::obj();
        manifest.set("project", project);
        manifest.set("fonts", fonts);
        manifest.set("packages", packages);
        manifest.set("library", library);
        o.set("manifest", manifest);
        o.set(
            "texinputs",
            Value::Arr(
                texinputs
                    .iter()
                    .map(|t| {
                        let mut e = Value::obj();
                        e.set("index", json::num(t.index as f64));
                        e.set("raw", json::str_(t.raw.clone()));
                        match &t.location {
                            TexInputLocation::Inside(d) => {
                                e.set("location", json::str_("inside"));
                                e.set("dir", json::str_(d.clone()));
                            }
                            TexInputLocation::Outside(p) => {
                                e.set("location", json::str_("outside"));
                                e.set("dir", json::str_(flashtex_project_manifest::outside_virtual_dir(t.index)));
                                e.set("path", json::str_(p.display().to_string()));
                            }
                            TexInputLocation::Invalid(why) => {
                                e.set("location", json::str_("invalid"));
                                e.set("reason", json::str_(why.clone()));
                            }
                        }
                        e
                    })
                    .collect(),
            ),
        );
        o.set(
            "warnings",
            Value::Arr(
                input
                    .manifest
                    .warnings
                    .iter()
                    .map(|w| {
                        let mut e = Value::obj();
                        e.set("key", json::str_(w.key.clone()));
                        e.set("message", json::str_(w.message.clone()));
                        e
                    })
                    .collect(),
            ),
        );
        println!("{}", json::write(&o));
    } else {
        match &input.manifest.found {
            Some(p) => println!("# {}", p.display()),
            None => println!("# no {FILE_NAME} governs {} (defaults shown)", input.entry.display()),
        }
        println!("# entry: {}", input.entry.display());
        for t in &texinputs {
            match &t.location {
                TexInputLocation::Inside(d) => println!("# texinputs[{}] = {:?}: {d}/ under the project root", t.index, t.raw),
                TexInputLocation::Outside(p) => println!("# texinputs[{}] = {:?}: {} (outside the root; mounted at texinputs/{}/)", t.index, t.raw, p.display(), t.index),
                TexInputLocation::Invalid(why) => println!("# texinputs[{}] = {:?}: ignored, {why}", t.index, t.raw),
            }
        }
        print!("{}", m.to_toml());
        for w in &input.manifest.warnings {
            eprintln!("flashtex: warning: {FILE_NAME}: {w}");
        }
    }
    EXIT_OK
}

/// `supported`: the inventory of the compiler linked into this binary
/// (`flashtex_compiler::supported`), so it never drifts from what `build`
/// accepts. `--json` is the `flashtex-supported-latex/1` document with the
/// coverage section; `--md` the reference page; `--coverage` the coverage
/// table; the default is a one-screen summary with the percentage.
fn supported(args: &[String]) -> i32 {
    use flashtex_compiler::supported;
    let inventory = supported::inventory();
    match args.first().map(String::as_str) {
        None => {
            let rows = supported::coverage(&inventory);
            let total = rows.iter().map(|r| r.supported.len()).sum::<usize>();
            let canonical = rows.iter().map(|r| r.canonical.len()).sum::<usize>();
            let pct = if canonical == 0 { 0.0 } else { total as f64 * 100.0 / canonical as f64 };
            println!("FlashTeX supported LaTeX ({})", version_string());
            println!("commands: {}, environments: {}", inventory.commands.len(), inventory.environments.len());
            println!("coverage of the canonical inventory: {total}/{canonical} ({pct:.1}%)");
            for r in &rows {
                println!("  {:<12} {:<12} {:>4}/{:<4} {:>5.1}%", r.set, r.kind, r.supported.len(), r.canonical.len(), r.percent());
            }
            println!("any other command produces an explicit 'not supported' diagnostic naming it");
            println!("`flashtex supported --json|--md|--coverage` for the full inventory");
            EXIT_OK
        }
        Some("--json") | Some("json") => {
            print!("{}", supported::render_json(&inventory));
            EXIT_OK
        }
        Some("--md") | Some("--markdown") | Some("markdown") => {
            print!("{}", supported::render_markdown(&inventory));
            EXIT_OK
        }
        Some("--coverage") | Some("coverage") => {
            print!("{}", supported::render_coverage_markdown(&inventory));
            EXIT_OK
        }
        Some(other) => usage_error(&format!("supported: unknown option {other:?} (use --json, --md or --coverage)")),
    }
}

/// `worker`: the runtime-v1 JSON Lines worker (`protocol::serve`), the
/// process the IDE launches (`FLASHTEX_COMPILER`). `flashtex-render` is the
/// same loop; the app may launch either.
fn worker(args: &[String]) -> i32 {
    let mut dirs = Vec::new();
    let mut options = RenderOptions::default();
    let mut v2: Option<PathBuf> = None;
    let mut pdf: Option<PathBuf> = None;
    let mut timing = false;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        let mut value = || {
            i += 1;
            args.get(i).cloned().ok_or_else(|| format!("{a} needs a value"))
        };
        let r: Result<(), String> = match a {
            "--font-dir" => value().map(|v| dirs.push(PathBuf::from(v))),
            "--project-root" => value().map(|v| options.project_root = Some(PathBuf::from(v))),
            "--class-options" => value().map(|v| options.default_class_options = v),
            "--secnumdepth" => value().and_then(|v| v.parse::<u8>().map_err(|_| format!("--secnumdepth needs a small number, got {v:?}"))).map(|n| options.default_secnumdepth = n),
            "--v2" => value().map(|v| v2 = Some(PathBuf::from(v))),
            "--pdf" => value().map(|v| pdf = Some(PathBuf::from(v))),
            "--timing" => {
                timing = true;
                Ok(())
            }
            other => Err(format!("worker: unknown option {other:?}")),
        };
        if let Err(e) = r {
            return usage_error(&e);
        }
        i += 1;
    }
    let fonts = font_set(&dirs);
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    let served = flashtex_render_pipeline::protocol::serve(&mut input, &mut output, &fonts, &options, |id, r| {
        if timing {
            eprintln!("flashtex: worker: {id} rendered in {:.2} ms", r.elapsed_ms);
        }
        if let Some(p) = &v2 {
            if let Err(e) = compile::write_atomic(p, r.v2.write_json_with(id, true).as_bytes()) {
                eprintln!("flashtex: worker: {e}");
            }
        }
        if let Some(p) = &pdf {
            match flashtex_render_pipeline::pdf::write_pdf_exact(&r.v2, fonts.dirs(), options.project_root.as_deref()) {
                Ok(out) => {
                    if let Err(e) = compile::write_atomic(p, &out.bytes) {
                        eprintln!("flashtex: worker: {e}");
                    }
                }
                Err(e) => eprintln!("flashtex: worker: pdf: {e}"),
            }
        }
    });
    match served {
        Ok(()) => EXIT_OK,
        Err(e) => {
            eprintln!("flashtex: worker: read error: {e}");
            EXIT_FAILED
        }
    }
}

/// `fonts`: the search lists in order, marking which directories exist,
/// plus whether the pinned Latin Modern metrics load from them.
fn fonts_cmd(args: &[String]) -> i32 {
    let mut extra = Vec::new();
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--font-dir" => {
                i += 1;
                match args.get(i) {
                    Some(d) => extra.push(PathBuf::from(d)),
                    None => return usage_error("--font-dir needs a value"),
                }
            }
            "--json" => json_out = true,
            other => return usage_error(&format!("fonts: unknown option {other:?}")),
        }
        i += 1;
    }
    let d = Discovery::from_process();
    let set = font_set(&extra);
    let metrics = set.required_metrics().map(|_| ()).map_err(|e| e.to_string());
    let latin_modern = set.latin_modern_available();
    let exe_dir = d.exe_dir.clone();
    if json_out {
        use flashtex_compiler::json::{self, Value};
        let dirs = |list: &[PathBuf]| {
            Value::Arr(
                list.iter()
                    .map(|p| {
                        let mut o = Value::obj();
                        o.set("path", json::str_(p.display().to_string()));
                        o.set("exists", Value::Bool(p.is_dir()));
                        o
                    })
                    .collect(),
            )
        };
        let mut o = Value::obj();
        o.set("schema", json::str_("flashtex-fonts/1"));
        o.set("executable_dir", exe_dir.as_ref().map_or(Value::Null, |p| json::str_(p.display().to_string())));
        o.set("font_dirs", dirs(set.dirs()));
        o.set("tfm_dirs", dirs(set.tfm_dirs()));
        o.set("latin_modern_available", Value::Bool(latin_modern));
        o.set("required_metrics", metrics.clone().map_or_else(|e| json::str_(e), |_| json::str_("loaded")));
        let mut env = Value::obj();
        for (k, v) in [("FLASHTEX_FONT_DIRS", &d.font_dirs), ("FLASHTEX_TFM_DIRS", &d.tfm_dirs), ("FLASHTEX_LM_DIR", &d.lm_dir)] {
            env.set(k, v.clone().map_or(Value::Null, json::str_));
        }
        o.set("environment", env);
        println!("{}", json::write(&o));
    } else {
        println!("{}", version_string());
        println!("executable directory: {}", exe_dir.as_ref().map_or("unknown".to_string(), |p| p.display().to_string()));
        for (k, v) in [("FLASHTEX_FONT_DIRS", &d.font_dirs), ("FLASHTEX_TFM_DIRS", &d.tfm_dirs), ("FLASHTEX_LM_DIR", &d.lm_dir)] {
            println!("{k}: {}", v.as_deref().unwrap_or("(unset)"));
        }
        let mark = |p: &Path| if p.is_dir() { "+" } else { "-" };
        println!("font directories (search order; + exists, - absent):");
        for p in set.dirs() {
            println!("  {} {}", mark(p), p.display());
        }
        println!("TFM directories (search order):");
        for p in set.tfm_dirs() {
            println!("  {} {}", mark(p), p.display());
        }
        println!("Latin Modern text + math faces: {}", if latin_modern { "found" } else { "NOT found" });
        match &metrics {
            Ok(()) => println!("pinned Latin Modern 2.004 TFM metrics: loaded ({})", fonts::REQUIRED_TFM_DIR),
            Err(e) => println!("pinned Latin Modern 2.004 TFM metrics: unavailable: {e}"),
        }
        println!("bundled layouts probed: <exe>/../Resources/{{Fonts,texmf}}, <exe>/{{Fonts,texmf}}, <exe>/{}/{{Fonts,texmf}}", fonts::SHARE_DIR);
    }
    if latin_modern && metrics.is_ok() {
        EXIT_OK
    } else {
        EXIT_FAILED
    }
}

/// `install-cli [DIR]`: a symlink `DIR/flashtex` to this executable
/// (default `/usr/local/bin`). A symlink, not a copy, so the binary keeps
/// finding its `share/flashtex` (tarball) or `../Resources` (app bundle)
/// fonts relative to its real location.
fn install_cli(args: &[String]) -> i32 {
    let dir = match args {
        [] => PathBuf::from("/usr/local/bin"),
        [d] if !d.starts_with('-') => PathBuf::from(d),
        _ => return usage_error("install-cli takes at most one directory"),
    };
    let exe = match std::env::current_exe().and_then(std::fs::canonicalize) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("flashtex: cannot locate this executable: {e}");
            return EXIT_FAILED;
        }
    };
    let link = dir.join("flashtex");
    if let Ok(target) = std::fs::read_link(&link) {
        if target == exe {
            println!("{} already links to {}", link.display(), exe.display());
            return EXIT_OK;
        }
    }
    if link.exists() || std::fs::symlink_metadata(&link).is_ok() {
        eprintln!("flashtex: {} exists and is not a link to this binary; remove it first", link.display());
        return EXIT_FAILED;
    }
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(&exe, &link);
    #[cfg(not(unix))]
    let made: std::io::Result<()> = Err(std::io::Error::other("symlinks need a Unix host; copy the binary and its share/flashtex directory instead"));
    match made {
        Ok(()) => {
            println!("linked {} -> {}", link.display(), exe.display());
            EXIT_OK
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!(
                "flashtex: no permission to write {}: rerun with sudo, or pass a directory you own (e.g. `flashtex install-cli ~/.local/bin`)",
                link.display()
            );
            EXIT_FAILED
        }
        Err(e) => {
            eprintln!("flashtex: cannot create {}: {e}", link.display());
            EXIT_FAILED
        }
    }
}
