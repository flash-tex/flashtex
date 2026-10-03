//! `flashtex-v3`: the command line on the new engine (DESIGN.md §10, row D5).
//!
//! ```text
//! flashtex-v3 build [<main.tex>|<dir>] [-o out.pdf] [--no-tools] [--host PATH]
//! flashtex-v3 check [<main.tex>|<dir>] [--json] [--no-tools] [--host PATH]
//! flashtex-v3 watch [<main.tex>|<dir>] [-o out.pdf] [--interval MS] [--no-tools] [--host PATH]
//! ```
//!
//! It is the successor of `flashtex build/check/watch` (crates/flashtex-cli,
//! which links the old engine and is deleted with it, retirement plan S7,
//! RQ6). It never links the engine: it starts `flashtex-host` as a separate
//! process and speaks display-list-v3 over its socket, as the Mac app does
//! (DESIGN.md §3).
//!
//! - `build` compiles as latexmk would: a resident compile that runs
//!   bibtex, biber and makeindex when the document needs them (`--no-tools`:
//!   never), then an `export` compile, whose PDF is the one pdflatex writes
//!   (P-T2). The PDF goes to `-o`, else next to the main file; the `.aux`,
//!   `.log` and the rest stay in a per-project directory under the system's
//!   temporary directory, so the project gets only the PDF (as with the old
//!   `flashtex build`).
//! - `check` compiles and prints TeX's diagnostics (`diag-v1`), as
//!   `file:line:col: severity: message`, or one JSON object a line with
//!   `--json`. The exit status is 1 when there is an error.
//! - `watch` builds, then builds again whenever a file of the project
//!   changes (checked every `--interval` ms, default 500); Ctrl-C stops.
//!
//! The host: `--host`, else `$FLASHTEX_HOST`, else `flashtex-host` beside
//! this program, else on `PATH`. It needs a TeX Live (D12); the string pool
//! is found as the app finds it (`$FLASHTEX_POOL`, beside the host).

use flashtex_display_list::client::{Client, CompileRequest, Event};
use flashtex_display_list::diag::Diag;
use flashtex_display_list::json::Json;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::time::{Duration, Instant, SystemTime};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first().cloned() else {
        return usage("a command: build, check or watch");
    };
    let opts = match Opts::parse(&args[1..]) {
        Ok(o) => o,
        Err(e) => return usage(&e),
    };
    let result = match cmd.as_str() {
        "build" => build(&opts).map(|r| r.exit),
        "check" => check(&opts),
        "watch" => watch(&opts),
        "-h" | "--help" | "help" => {
            println!("{}", USAGE);
            return ExitCode::SUCCESS;
        }
        other => {
            return usage(&format!(
                "unknown command {other:?} (use build, check or watch)"
            ))
        }
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("flashtex-v3: {e}");
            ExitCode::from(1)
        }
    }
}

const USAGE: &str = "usage: flashtex-v3 build|check|watch [<main.tex>|<dir>] [-o out.pdf] [--json] [--no-tools] [--interval MS] [--host PATH]";

fn usage(why: &str) -> ExitCode {
    eprintln!("flashtex-v3: {why}\n{USAGE}");
    ExitCode::from(2)
}

/// The parsed options.
#[derive(Debug, Default)]
struct Opts {
    target: Option<PathBuf>,
    out: Option<PathBuf>,
    json: bool,
    tools: bool,
    interval_ms: u64,
    host: Option<PathBuf>,
}

impl Opts {
    fn parse(a: &[String]) -> Result<Opts, String> {
        let mut o = Opts {
            tools: true,
            interval_ms: 500,
            ..Opts::default()
        };
        let mut i = 0;
        while i < a.len() {
            let v = a.get(i + 1);
            match a[i].as_str() {
                "-o" | "--output" => o.out = Some(v.ok_or("-o needs a file")?.into()),
                "--host" => o.host = Some(v.ok_or("--host needs a path")?.into()),
                "--interval" => {
                    o.interval_ms = v
                        .and_then(|s| s.parse().ok())
                        .ok_or("--interval needs milliseconds")?
                }
                "--json" => {
                    o.json = true;
                    i += 1;
                    continue;
                }
                "--no-tools" => {
                    o.tools = false;
                    i += 1;
                    continue;
                }
                s if s.starts_with('-') => return Err(format!("unknown option {s}")),
                s => {
                    if o.target.is_some() {
                        return Err(format!("one main file or directory, not also {s}"));
                    }
                    o.target = Some(s.into());
                    i += 1;
                    continue;
                }
            }
            i += 2;
        }
        Ok(o)
    }
}

/// The project: its root directory and the main file relative to it.
#[derive(Debug, Clone, PartialEq)]
struct Project {
    root: PathBuf,
    main: String,
}

impl Project {
    /// `main.tex` → its folder and name; a directory → its only `.tex`
    /// with `\documentclass`, or `main.tex`.
    fn resolve(target: Option<&Path>) -> Result<Project, String> {
        let t = target
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let t = std::fs::canonicalize(&t).map_err(|e| format!("{}: {e}", t.display()))?;
        if t.is_file() {
            let root = t.parent().ok_or("no parent directory")?.to_path_buf();
            let main = t
                .file_name()
                .ok_or("no file name")?
                .to_string_lossy()
                .into_owned();
            return Ok(Project { root, main });
        }
        let mut mains: Vec<String> = std::fs::read_dir(&t)
            .map_err(|e| format!("{}: {e}", t.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".tex"))
            .filter(|n| std::fs::read_to_string(t.join(n)).is_ok_and(|s| declares_class(&s)))
            .collect();
        mains.sort();
        let main = match mains.as_slice() {
            [one] => one.clone(),
            _ if t.join("main.tex").is_file() => "main.tex".into(),
            [] => {
                return Err(format!(
                    "{}: no .tex file with \\documentclass",
                    t.display()
                ))
            }
            many => {
                return Err(format!(
                    "{}: several main files ({}); name one",
                    t.display(),
                    many.join(", ")
                ))
            }
        };
        Ok(Project { root: t, main })
    }

    fn jobname(&self) -> String {
        self.main
            .strip_suffix(".tex")
            .unwrap_or(&self.main)
            .to_string()
    }

    /// Where the `.aux`, `.log` and the rest go: a directory per project
    /// under the system's temporary directory.
    fn work_dir(&self) -> PathBuf {
        let mut h: u64 = 0xcbf29ce484222325; // FNV-1a of the root's path
        for b in self.root.to_string_lossy().bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        std::env::temp_dir().join(format!("flashtex-v3-{h:016x}"))
    }
}

/// A `\documentclass` outside a comment.
fn declares_class(s: &str) -> bool {
    s.lines().any(|l| {
        l.split('%')
            .next()
            .unwrap_or("")
            .contains("\\documentclass")
    })
}

/// The engine host, running for this command.
struct Host {
    child: Child,
    socket: PathBuf,
}

impl Host {
    fn locate(explicit: Option<&Path>) -> Result<PathBuf, String> {
        if let Some(p) = explicit {
            return Ok(p.into());
        }
        if let Ok(p) = std::env::var("FLASHTEX_HOST") {
            if !p.is_empty() {
                return Ok(p.into());
            }
        }
        if let Some(beside) = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|d| d.join("flashtex-host")))
        {
            if beside.is_file() {
                return Ok(beside);
            }
        }
        for dir in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
            let p = dir.join("flashtex-host");
            if p.is_file() {
                return Ok(p);
            }
        }
        Err(
            "flashtex-host not found (--host PATH, $FLASHTEX_HOST, beside flashtex-v3, or on PATH)"
                .into(),
        )
    }

    /// The engine's string pool, as the app finds it: beside the host (a
    /// bundle), beside this program, else a checkout's
    /// `crates/flashtex-engine/pdftex.pool` above the host (development).
    fn pool(exe: &Path) -> Option<PathBuf> {
        let mut c: Vec<PathBuf> = vec![exe.with_file_name("pdftex.pool")];
        if let Ok(me) = std::env::current_exe() {
            c.push(me.with_file_name("pdftex.pool"));
        }
        let abs = std::fs::canonicalize(exe).unwrap_or_else(|_| exe.to_path_buf());
        c.extend(
            abs.ancestors()
                .map(|d| d.join("crates/flashtex-engine/pdftex.pool")),
        );
        c.into_iter().find(|p| p.is_file())
    }

    fn start(exe: &Path) -> Result<Host, String> {
        let socket =
            std::env::temp_dir().join(format!("ftx-v3-{}-{}.sock", std::process::id(), nanos()));
        let mut cmd = Command::new(exe);
        cmd.args(["--socket", &socket.to_string_lossy(), "--once", "--no-warm"]);
        if std::env::var_os("FLASHTEX_POOL").is_none() {
            if let Some(pool) = Self::pool(exe) {
                cmd.env("FLASHTEX_POOL", pool);
            }
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        let child = cmd.spawn().map_err(|e| format!("{}: {e}", exe.display()))?;
        Ok(Host { child, socket })
    }

    /// Connects once the host listens (it prepares the format first).
    fn connect(&mut self) -> Result<Client, String> {
        let t0 = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().map_err(|e| e.to_string())? {
                return Err(format!(
                    "flashtex-host exited before it listened ({status})"
                ));
            }
            if self.socket.exists() {
                if let Ok(c) = Client::connect_accepting(
                    &self.socket,
                    &[flashtex_display_list::diag::CAPABILITY],
                ) {
                    return Ok(c);
                }
            }
            if t0.elapsed() > Duration::from_secs(600) {
                return Err("flashtex-host did not listen within 10 minutes".into());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.socket);
    }
}

fn nanos() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// What one compile reported.
#[derive(Default)]
struct Outcome {
    status: String,
    pdf: Option<PathBuf>,
    diags: Vec<Diag>,
    /// Plain `DIAGNOSTIC`s (a host without `diag-v1`): file, line, severity, message.
    plain: Vec<Json>,
    tool_notes: Vec<String>,
}

/// Sends `req` and reads to its `DONE`; with external tools, on to the
/// cycle's `settled` (the follow-up compiles report under the same id).
fn compile(c: &mut Client, req: &CompileRequest) -> Result<Outcome, String> {
    c.compile(req).map_err(|e| e.to_string())?;
    let mut out = Outcome::default();
    let tools = req.external_tools.as_deref() == Some("auto");
    let (mut done, mut settled, mut tools_ran) = (false, !tools, false);
    while !(done && settled) {
        let Some(ev) = c.next_event().map_err(|e| e.to_string())? else {
            return Err("the host closed the connection".into());
        };
        match ev {
            Event::Started(j) if j.int_field("id") == Some(req.id) => {
                // A follow-up compile with what the tools made: its rows replace the last ones.
                if j.str_field("cause") == Some("tools") {
                    out.diags.clear();
                    out.plain.clear();
                    done = false;
                }
            }
            Event::Diag(d) if d.id == req.id => out.diags.push(d),
            Event::Diagnostic(j) if j.int_field("id") == Some(req.id) => out.plain.push(j),
            Event::Done(j) if j.int_field("id") == Some(req.id) => {
                done = true;
                out.status = j.str_field("status").unwrap_or("?").into();
                out.pdf = j.str_field("pdf").map(PathBuf::from);
            }
            Event::Tool(j) if j.int_field("id") == Some(req.id) => match j.str_field("event") {
                Some("run") => tools_ran = true,
                Some("done") => {
                    let st = j.str_field("status").unwrap_or("?");
                    if st != "ok" && st != "warnings" {
                        out.tool_notes
                            .push(format!("{}: {st}", j.str_field("tool").unwrap_or("tool")));
                    }
                }
                Some("settled") => settled = true,
                _ => {}
            },
            Event::Error(j) if j.int_field("id").is_none_or(|i| i == req.id) => {
                return Err(format!(
                    "host error: {}",
                    j.str_field("message").unwrap_or("?")
                ));
            }
            _ => {}
        }
    }
    let _ = tools_ran;
    Ok(out)
}

fn request(p: &Project, id: i64, tools: bool, export: bool) -> CompileRequest {
    let mut r = CompileRequest::new(id, &p.root.to_string_lossy(), &p.main);
    r.output_dir = Some(p.work_dir().to_string_lossy().into_owned());
    r.jobname = Some(p.jobname());
    r.export = export;
    if !export {
        r.external_tools = Some(if tools { "auto" } else { "off" }.into());
    }
    r
}

/// One diagnostic as a line: `file:line:col: severity: message`, the file
/// relative to the project, the column (1-based) where the reported token
/// starts (`range`), else TeX's split.
fn line_of(d: &Diag, root: &Path) -> String {
    let file = d.file.as_ref().map(|f| {
        let f = f.strip_prefix("./").unwrap_or(f);
        let r = root.to_string_lossy().into_owned();
        let real = std::fs::canonicalize(root)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let short = r.replace("/private/var/", "/var/");
        [r, real, short]
            .iter()
            .find_map(|r| f.strip_prefix(&format!("{r}/")).map(String::from))
            .unwrap_or_else(|| f.to_string())
    });
    let col = d.range.map(|r| r.0).or(d.col);
    let at = match (&file, d.line, col) {
        (Some(f), Some(l), Some(c)) => format!("{f}:{l}:{}: ", c + 1),
        (Some(f), Some(l), None) => format!("{f}:{l}: "),
        (Some(f), None, _) => format!("{f}: "),
        _ => String::new(),
    };
    let sev = d.severity.map(|s| s.as_str()).unwrap_or("error");
    format!("{at}{sev}: {}", d.message)
}

fn is_error(d: &Diag) -> bool {
    d.severity.map(|s| s.as_str() == "error").unwrap_or(true)
}

struct Built {
    exit: ExitCode,
}

/// `build`: the resident compile (with the tools), then the export.
fn build(o: &Opts) -> Result<Built, String> {
    let p = Project::resolve(o.target.as_deref())?;
    std::fs::create_dir_all(p.work_dir()).map_err(|e| e.to_string())?;
    let exe = Host::locate(o.host.as_deref())?;
    let mut host = Host::start(&exe)?;
    let mut c = host.connect()?;
    let t0 = Instant::now();
    let first = compile(&mut c, &request(&p, 1, o.tools, false))?;
    for n in &first.tool_notes {
        eprintln!("flashtex-v3: {n}");
    }
    let errors: Vec<&Diag> = first.diags.iter().filter(|d| is_error(d)).collect();
    for d in &errors {
        eprintln!("{}", line_of(d, &p.root));
    }
    let export = compile(&mut c, &request(&p, 2, false, true))?;
    let _ = c.bye();
    let target = o
        .out
        .clone()
        .unwrap_or_else(|| p.root.join(format!("{}.pdf", p.jobname())));
    match export.pdf.filter(|f| f.is_file()) {
        Some(pdf) if export.status != "failed" => {
            std::fs::copy(&pdf, &target).map_err(|e| format!("{}: {e}", target.display()))?;
            eprintln!(
                "flashtex-v3: wrote {} ({} ms){}",
                target.display(),
                t0.elapsed().as_millis(),
                if errors.is_empty() {
                    String::new()
                } else {
                    format!(", {} error(s)", errors.len())
                }
            );
            Ok(Built {
                exit: if errors.is_empty() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                },
            })
        }
        _ => {
            eprintln!(
                "flashtex-v3: no PDF ({}){}",
                export.status,
                if errors.is_empty() {
                    ""
                } else {
                    "; see the errors above"
                }
            );
            Ok(Built {
                exit: ExitCode::from(1),
            })
        }
    }
}

/// `check`: one resident compile, its diagnostics printed.
fn check(o: &Opts) -> Result<ExitCode, String> {
    let p = Project::resolve(o.target.as_deref())?;
    std::fs::create_dir_all(p.work_dir()).map_err(|e| e.to_string())?;
    let exe = Host::locate(o.host.as_deref())?;
    let mut host = Host::start(&exe)?;
    let mut c = host.connect()?;
    let out = compile(&mut c, &request(&p, 1, o.tools, false))?;
    let _ = c.bye();
    let mut errors = 0;
    for d in out
        .diags
        .iter()
        .filter(|d| d.severity.map(|s| s.as_str() != "info").unwrap_or(true))
    {
        if is_error(d) {
            errors += 1;
        }
        if o.json {
            println!("{}", d.to_json());
        } else {
            println!("{}", line_of(d, &p.root));
        }
    }
    for j in &out.plain {
        if j.str_field("severity") == Some("error") {
            errors += 1;
        }
        println!(
            "{}",
            if o.json {
                j.to_string()
            } else {
                j.str_field("message").unwrap_or("?").to_string()
            }
        );
    }
    if !o.json {
        eprintln!("flashtex-v3: {} · {errors} error(s)", out.status);
    }
    Ok(if errors > 0 || out.status == "failed" {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

/// `watch`: build, then build again when a project file changes.
fn watch(o: &Opts) -> Result<ExitCode, String> {
    let p = Project::resolve(o.target.as_deref())?;
    let mut last = stamp(&p.root);
    loop {
        if let Err(e) = build(o) {
            eprintln!("flashtex-v3: {e}");
        }
        loop {
            std::thread::sleep(Duration::from_millis(o.interval_ms.max(50)));
            let now = stamp(&p.root);
            if now != last {
                last = now;
                break;
            }
        }
    }
}

/// The newest modification time and the count of the project's source
/// files (`.tex`, `.sty`, `.cls`, `.bib`, `.bst` and images), hidden
/// directories skipped.
fn stamp(root: &Path) -> (u128, usize) {
    fn walk(d: &Path, depth: usize, acc: &mut (u128, usize)) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let path = e.path();
            if path.is_dir() {
                if depth < 8 {
                    walk(&path, depth + 1, acc);
                }
            } else if [
                ".tex", ".sty", ".cls", ".bib", ".bst", ".png", ".jpg", ".pdf", ".eps",
            ]
            .iter()
            .any(|x| name.ends_with(x))
            {
                let m = e.metadata().and_then(|m| m.modified()).ok();
                let t = m
                    .and_then(|m| m.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|d| d.as_nanos())
                    .unwrap_or(0);
                acc.0 = acc.0.max(t);
                acc.1 += 1;
            }
        }
    }
    let mut acc = (0, 0);
    walk(root, 0, &mut acc);
    acc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn options_parse_as_the_old_cli_spelled_them() {
        let o = Opts::parse(&args(
            "paper/main.tex -o out.pdf --json --no-tools --interval 250 --host /x/flashtex-host",
        ))
        .unwrap();
        assert_eq!(o.target, Some("paper/main.tex".into()));
        assert_eq!(o.out, Some("out.pdf".into()));
        assert!(o.json && !o.tools);
        assert_eq!(o.interval_ms, 250);
        assert_eq!(o.host, Some("/x/flashtex-host".into()));
        assert!(Opts::parse(&args("a.tex b.tex")).is_err());
        assert!(Opts::parse(&args("--bogus")).is_err());
        assert!(Opts::parse(&args("-o")).is_err());
        let d = Opts::parse(&[]).unwrap();
        assert!(d.tools && d.target.is_none() && d.interval_ms == 500);
    }

    #[test]
    fn a_directory_resolves_to_its_main_file() {
        let dir = std::env::temp_dir().join(format!("flashtex-v3-test-{}", nanos()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("chapter.tex"),
            "Text.\n% \\documentclass in a comment\n",
        )
        .unwrap();
        std::fs::write(dir.join("paper.tex"), "\\documentclass{article}\n").unwrap();
        let p = Project::resolve(Some(&dir)).unwrap();
        assert_eq!(p.main, "paper.tex");
        assert_eq!(p.jobname(), "paper");
        assert!(p.work_dir().starts_with(std::env::temp_dir()));
        let f = Project::resolve(Some(&dir.join("chapter.tex"))).unwrap();
        assert_eq!(f.main, "chapter.tex");
        std::fs::write(dir.join("other.tex"), "\\documentclass{book}\n").unwrap();
        assert!(Project::resolve(Some(&dir))
            .unwrap_err()
            .contains("several main files"));
        std::fs::write(dir.join("main.tex"), "\\documentclass{book}\n").unwrap();
        assert_eq!(Project::resolve(Some(&dir)).unwrap().main, "main.tex");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
