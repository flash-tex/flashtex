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
//!   (P-T2). The PDF goes to `-o`, else next to the main file. The `.aux`,
//!   `.log` and the rest go to a fresh private directory (mode 0700 on Unix)
//!   that is removed at the end, so the project gets only the PDF and two
//!   runs never share a directory.
//! - `check` compiles and prints TeX's diagnostics (`diag-v1`), as
//!   `file:line:col: severity: message`, or one JSON object a line with
//!   `--json`, and the external tools' failures. The exit status is 1 when
//!   there is an error.
//! - `watch` builds, then builds again when a source file of the project
//!   changes (checked every `--interval` ms, default 500, settled for 200 ms;
//!   the PDFs it writes are not sources), with one warm host for the whole
//!   session; Ctrl-C stops.
//!
//! The host: `--host`, else `$FLASHTEX_HOST`, else `flashtex-host` beside
//! this program, else on `PATH`. It needs a TeX Live (D12); the string pool
//! is found as the app finds it (`$FLASHTEX_POOL`, beside the host). It is
//! started with `--once`: if this program is killed before it connects, the
//! host notices its parent is gone, removes its socket and exits; once
//! connected, the connection's end ends it.

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
    if matches!(cmd.as_str(), "-h" | "--help" | "help") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let opts = match Opts::parse(&args[1..]) {
        Ok(o) => o,
        Err(e) => return usage(&e),
    };
    let result = match cmd.as_str() {
        "build" => Session::open(&opts, false).and_then(|mut s| s.build(&opts)),
        "check" => Session::open(&opts, false).and_then(|mut s| s.check(&opts)),
        "watch" => watch(&opts),
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

/// A path without Windows' verbatim prefix (`\\?\C:\…` → `C:\…`), which
/// `canonicalize` adds and which neither TeX nor the host expects; a
/// verbatim UNC path (`\\?\UNC\…`) is kept. Other paths pass unchanged.
fn simplified(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => p,
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
        let t = simplified(std::fs::canonicalize(&t).map_err(|e| format!("{}: {e}", t.display()))?);
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

    /// Where `build` writes the PDF.
    fn target(&self, out: Option<&Path>) -> PathBuf {
        out.map(Path::to_path_buf)
            .unwrap_or_else(|| self.root.join(format!("{}.pdf", self.jobname())))
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

/// A fresh private directory for one run's `.aux`, `.log` and the rest:
/// created new (never an existing one, so no other user's directory is
/// used), mode 0700 on Unix, removed when dropped.
struct WorkDir(PathBuf);

impl WorkDir {
    fn new() -> Result<WorkDir, String> {
        let tmp = simplified(std::env::temp_dir());
        for n in 0..100u32 {
            let d = tmp.join(format!(
                "flashtex-v3-{}-{}-{n}",
                std::process::id(),
                nanos()
            ));
            let mut b = std::fs::DirBuilder::new();
            #[cfg(unix)]
            std::os::unix::fs::DirBuilderExt::mode(&mut b, 0o700);
            match b.create(&d) {
                Ok(()) => return Ok(WorkDir(d)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(format!("{}: {e}", d.display())),
            }
        }
        Err("could not create a private work directory".into())
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The engine host, running for this command.
struct Host {
    child: Child,
    socket: PathBuf,
}

impl Host {
    fn exe_name() -> String {
        format!("flashtex-host{}", std::env::consts::EXE_SUFFIX)
    }

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
            .and_then(|e| e.parent().map(|d| d.join(Self::exe_name())))
        {
            if beside.is_file() {
                return Ok(beside);
            }
        }
        for dir in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
            let p = dir.join(Self::exe_name());
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
        let abs = simplified(std::fs::canonicalize(exe).unwrap_or_else(|_| exe.to_path_buf()));
        c.extend(
            abs.ancestors()
                .map(|d| d.join("crates/flashtex-engine/pdftex.pool")),
        );
        c.into_iter().find(|p| p.is_file())
    }

    /// `warm`: the host warms its engine up first (a watch session keeps it).
    fn start(exe: &Path, warm: bool) -> Result<Host, String> {
        let socket = simplified(std::env::temp_dir()).join(format!(
            "ftx-v3-{}-{}.sock",
            std::process::id(),
            nanos()
        ));
        let mut cmd = Command::new(exe);
        // --once: the host serves this program only, and goes when it goes
        // (also when it is killed before it connects); --accept-timeout
        // bounds the wait for the connection.
        cmd.args([
            "--socket",
            &socket.to_string_lossy(),
            "--once",
            "--accept-timeout",
            "600",
        ]);
        if !warm {
            cmd.arg("--no-warm");
        }
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
    /// Plain `DIAGNOSTIC`s (a host without `diag-v1`).
    plain: Vec<Json>,
    /// External tools that did not succeed: "bibtex: errors".
    tool_failures: Vec<String>,
    /// Tools the host did not run, and why: "bibtex not run: not found: refs".
    tool_notes: Vec<String>,
}

/// Sends `req` and reads to its `DONE`; with external tools, on to the
/// cycle's `settled` (the follow-up compiles report under the same id).
fn compile(c: &mut Client, req: &CompileRequest) -> Result<Outcome, String> {
    c.compile(req).map_err(|e| e.to_string())?;
    let mut out = Outcome::default();
    let tools = req.external_tools.as_deref() == Some("auto");
    let (mut done, mut settled) = (false, !tools);
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
                Some("skip") => out.tool_notes.push(format!(
                    "{} not run: {}",
                    j.str_field("tool").unwrap_or("tool"),
                    j.str_field("reason").unwrap_or("skipped")
                )),
                Some("done") => {
                    let st = j.str_field("status").unwrap_or("?");
                    if st != "ok" && st != "warnings" {
                        let what = j
                            .str_field("message")
                            .map(|m| format!(" ({m})"))
                            .unwrap_or_default();
                        out.tool_failures.push(format!(
                            "{}: {st}{what}",
                            j.str_field("tool").unwrap_or("tool")
                        ));
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
    Ok(out)
}

/// `file` relative to the project root when it is inside it (any of the
/// root's spellings: as given, canonical, macOS's /private/var for /var),
/// with either separator; else as given.
fn relative(file: &str, root: &Path) -> String {
    let f = file
        .strip_prefix("./")
        .or_else(|| file.strip_prefix(".\\"))
        .unwrap_or(file);
    let fp = Path::new(f);
    let real = simplified(std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf()));
    let short = PathBuf::from(root.to_string_lossy().replacen("/private/var/", "/var/", 1));
    for r in [root.to_path_buf(), real, short] {
        if let Ok(rest) = fp.strip_prefix(&r) {
            return rest.to_string_lossy().into_owned();
        }
    }
    f.to_string()
}

/// One diagnostic as a line: `file:line:col: severity: message`, the
/// column (1-based) where the reported token starts (`range`), else TeX's split.
fn line_of(d: &Diag, root: &Path) -> String {
    let file = d.file.as_deref().map(|f| relative(f, root));
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

/// A host, its connection and a private work directory: one per command,
/// one for a whole `watch` session.
struct Session {
    project: Project,
    work: WorkDir,
    client: Client,
    next_id: i64,
    // Last: the connection closes before the host is stopped.
    _host: Host,
}

impl Session {
    fn open(o: &Opts, warm: bool) -> Result<Session, String> {
        let project = Project::resolve(o.target.as_deref())?;
        let work = WorkDir::new()?;
        let exe = Host::locate(o.host.as_deref())?;
        let mut host = Host::start(&exe, warm)?;
        let client = host.connect()?;
        Ok(Session {
            project,
            work,
            client,
            next_id: 1,
            _host: host,
        })
    }

    fn request(&mut self, tools: bool, export: bool) -> CompileRequest {
        let p = &self.project;
        let mut r = CompileRequest::new(self.next_id, &p.root.to_string_lossy(), &p.main);
        self.next_id += 1;
        r.output_dir = Some(self.work.0.to_string_lossy().into_owned());
        r.jobname = Some(p.jobname());
        r.export = export;
        if !export {
            r.external_tools = Some(if tools { "auto" } else { "off" }.into());
        }
        r
    }

    /// `build`: the resident compile (with the tools), then the export.
    fn build(&mut self, o: &Opts) -> Result<ExitCode, String> {
        let t0 = Instant::now();
        let req = self.request(o.tools, false);
        let first = compile(&mut self.client, &req)?;
        for n in &first.tool_failures {
            eprintln!("flashtex-v3: {n}");
        }
        for n in &first.tool_notes {
            eprintln!("flashtex-v3: warning: {n}");
        }
        let errors: Vec<&Diag> = first.diags.iter().filter(|d| is_error(d)).collect();
        for d in &errors {
            eprintln!("{}", line_of(d, &self.project.root));
        }
        let req = self.request(false, true);
        let export = compile(&mut self.client, &req)?;
        let target = self.project.target(o.out.as_deref());
        // `ok`, or `error`: TeX reported errors and wrote its PDF anyway, as
        // pdflatex in nonstop mode does. `failed` (no output) and
        // `cancelled` write nothing.
        let wrote = matches!(export.status.as_str(), "ok" | "error");
        match export.pdf.filter(|f| wrote && f.is_file()) {
            Some(pdf) => {
                std::fs::copy(&pdf, &target).map_err(|e| format!("{}: {e}", target.display()))?;
                if self.rerun_warned() {
                    eprintln!("flashtex-v3: warning: LaTeX asks for another run (\"Rerun\" in the log); cross-references may be off");
                }
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
                Ok(if errors.is_empty() && first.tool_failures.is_empty() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                })
            }
            None => {
                eprintln!(
                    "flashtex-v3: no PDF ({}){}",
                    export.status,
                    if errors.is_empty() {
                        ""
                    } else {
                        "; see the errors above"
                    }
                );
                Ok(ExitCode::from(1))
            }
        }
    }

    /// The export's log asks for another run.
    fn rerun_warned(&self) -> bool {
        std::fs::read_to_string(self.work.0.join(format!("{}.log", self.project.jobname())))
            .is_ok_and(|l| l.contains("Rerun to get") || l.contains("Rerun LaTeX"))
    }

    /// `check`: one resident compile, its diagnostics and tool failures printed.
    fn check(&mut self, o: &Opts) -> Result<ExitCode, String> {
        let req = self.request(o.tools, false);
        let out = compile(&mut self.client, &req)?;
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
                println!("{}", line_of(d, &self.project.root));
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
        for t in &out.tool_failures {
            errors += 1;
            if o.json {
                println!(
                    "{{\"severity\":\"error\",\"code\":\"tool\",\"message\":{}}}",
                    Json::Str(t.clone())
                );
            } else {
                println!("error: {t}");
            }
        }
        for t in &out.tool_notes {
            if o.json {
                println!(
                    "{{\"severity\":\"warning\",\"code\":\"tool\",\"message\":{}}}",
                    Json::Str(t.clone())
                );
            } else {
                println!("warning: {t}");
            }
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
}

/// `watch`: one warm host for the session; build, then build again when a
/// source changes.
fn watch(o: &Opts) -> Result<ExitCode, String> {
    let mut s = Session::open(o, true)?;
    let root = s.project.root.clone();
    let ignore: Vec<PathBuf> = vec![
        s.project.target(o.out.as_deref()),
        o.out.clone().unwrap_or_default(),
    ];
    watch_loop(
        || stamp(&root, &ignore),
        || {
            if let Err(e) = s.build(o) {
                eprintln!("flashtex-v3: {e}");
            }
        },
        Duration::from_millis(o.interval_ms.max(50)),
        Duration::from_millis(200),
        None,
    );
    Ok(ExitCode::SUCCESS)
}

/// The watch loop: build, then poll every `interval`; a change of the
/// stamp, once it has stayed the same for `debounce` (an editor's save in
/// several writes), builds again. `polls`: stop after that many polls
/// (tests); `None`: for ever. Returns the number of builds.
fn watch_loop<S: PartialEq>(
    mut stamp: impl FnMut() -> S,
    mut build: impl FnMut(),
    interval: Duration,
    debounce: Duration,
    polls: Option<usize>,
) -> usize {
    let mut builds = 0;
    let mut n = 0;
    loop {
        build();
        builds += 1;
        // After the build: what it wrote itself is not a change.
        let mut last = stamp();
        loop {
            if polls.is_some_and(|p| n >= p) {
                return builds;
            }
            n += 1;
            std::thread::sleep(interval);
            let now = stamp();
            if now == last {
                continue;
            }
            // Settle: until it stops changing for `debounce`.
            last = now;
            loop {
                std::thread::sleep(debounce);
                let again = stamp();
                if again == last {
                    break;
                }
                last = again;
            }
            break;
        }
    }
}

/// The newest modification time and the number of the project's source
/// files (`.tex`, `.sty`, `.cls`, `.bib`, `.bst` and images), hidden
/// directories skipped, and `ignore` (the PDFs `build` writes) left out.
fn stamp(root: &Path, ignore: &[PathBuf]) -> (u128, usize) {
    fn walk(d: &Path, depth: usize, ignore: &[PathBuf], acc: &mut (u128, usize)) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let path = e.path();
            if ignore
                .iter()
                .any(|i| !i.as_os_str().is_empty() && same_file(i, &path))
            {
                continue;
            }
            if path.is_dir() {
                if depth < 8 {
                    walk(&path, depth + 1, ignore, acc);
                }
            } else if [
                ".tex", ".sty", ".cls", ".bib", ".bst", ".png", ".jpg", ".jpeg", ".pdf", ".eps",
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
    walk(root, 0, ignore, &mut acc);
    acc
}

/// The same path, however spelled (relative `-o`, symlinked temp dirs).
fn same_file(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "flashtex-v3-test-{name}-{}-{}",
            std::process::id(),
            nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
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
        let dir = scratch("resolve");
        std::fs::write(
            dir.join("chapter.tex"),
            "Text.\n% \\documentclass in a comment\n",
        )
        .unwrap();
        std::fs::write(dir.join("paper.tex"), "\\documentclass{article}\n").unwrap();
        let p = Project::resolve(Some(&dir)).unwrap();
        assert_eq!(p.main, "paper.tex");
        assert_eq!(p.jobname(), "paper");
        assert_eq!(p.target(None), p.root.join("paper.pdf"));
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

    #[test]
    fn work_dirs_are_fresh_private_and_removed() {
        let a = WorkDir::new().unwrap();
        let b = WorkDir::new().unwrap();
        assert_ne!(a.0, b.0, "two runs never share a directory");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&a.0).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        let path = a.0.clone();
        drop(a);
        assert!(!path.exists());
    }

    #[test]
    fn windows_verbatim_paths_are_simplified() {
        assert_eq!(
            simplified(PathBuf::from(r"\\?\C:\Users\me\paper")),
            PathBuf::from(r"C:\Users\me\paper")
        );
        assert_eq!(
            simplified(PathBuf::from(r"\\?\UNC\server\share")),
            PathBuf::from(r"\\?\UNC\server\share")
        );
        assert_eq!(simplified(PathBuf::from("/tmp/x")), PathBuf::from("/tmp/x"));
    }

    #[test]
    fn diagnostics_name_the_file_relative_to_the_project() {
        let root = scratch("relative");
        let f = root.join("chap").join("one.tex");
        assert_eq!(
            relative(&f.to_string_lossy(), &root),
            Path::new("chap").join("one.tex").to_string_lossy()
        );
        assert_eq!(relative("./main.tex", &root), "main.tex");
        assert_eq!(relative(".\\main.tex", &root), "main.tex");
        assert_eq!(relative("/elsewhere/x.sty", &root), "/elsewhere/x.sty");
        let d = Diag {
            file: Some(root.join("main.tex").to_string_lossy().into_owned()),
            line: Some(3),
            col: Some(20),
            range: Some((6, 21)),
            message: "Undefined control sequence.".into(),
            ..Diag::default()
        };
        assert_eq!(
            line_of(&d, &root),
            "main.tex:3:7: error: Undefined control sequence."
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The PDF a build writes is not a source: watch on an untouched
    /// document builds exactly once (it rebuilt for ever before).
    #[test]
    fn watch_builds_an_untouched_document_once() {
        let root = scratch("watch-once");
        std::fs::write(root.join("main.tex"), "\\documentclass{article}\n").unwrap();
        let pdf = root.join("main.pdf");
        let builds = Cell::new(0);
        let n = watch_loop(
            || stamp(&root, std::slice::from_ref(&pdf)),
            || {
                builds.set(builds.get() + 1);
                std::thread::sleep(Duration::from_millis(5));
                std::fs::write(&pdf, format!("%PDF build {}", builds.get())).unwrap();
                // as build writes it
            },
            Duration::from_millis(20),
            Duration::from_millis(20),
            Some(15),
        );
        assert_eq!((n, builds.get()), (1, 1));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// An edited source builds again, once per settled change.
    #[test]
    fn watch_builds_again_once_after_an_edit() {
        let root = scratch("watch-edit");
        let main = root.join("main.tex");
        std::fs::write(&main, "\\documentclass{article}\n").unwrap();
        let pdf = root.join("main.pdf");
        let edit = std::thread::spawn({
            let main = main.clone();
            move || {
                std::thread::sleep(Duration::from_millis(150));
                // A save in several writes, a few milliseconds apart.
                for i in 0..3 {
                    std::fs::write(&main, format!("\\documentclass{{article}}\n% edit {i}\n"))
                        .unwrap();
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        });
        let n = watch_loop(
            || stamp(&root, std::slice::from_ref(&pdf)),
            || std::fs::write(&pdf, "%PDF").unwrap(),
            Duration::from_millis(20),
            Duration::from_millis(60),
            Some(40),
        );
        edit.join().unwrap();
        assert_eq!(n, 2, "the first build and one for the edit");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn stamp_counts_sources_not_the_written_pdf_or_hidden_dirs() {
        let root = scratch("stamp");
        std::fs::write(root.join("main.tex"), "x").unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join(".git").join("x.tex"), "x").unwrap();
        let out = root.join("main.pdf");
        let before = stamp(&root, std::slice::from_ref(&out));
        std::fs::write(&out, "%PDF").unwrap();
        assert_eq!(
            stamp(&root, std::slice::from_ref(&out)),
            before,
            "the output PDF is not a source"
        );
        std::fs::write(root.join("figure.pdf"), "%PDF").unwrap();
        assert_eq!(
            stamp(&root, std::slice::from_ref(&out)).1,
            before.1 + 1,
            "a PDF figure is"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
