//! `flashtex-host-unicode`: the engine host of Unicode mode
//! (docs/design/xetex/PLAN.md §3.3, phase S3; modes PROPOSAL.md §5.1). It
//! speaks `display-list-v3` over a Unix socket (docs/protocol/
//! display-list-v3.md §6) as `flashtex-host` does for Classic, so the app
//! and `flashtex-v3` start whichever host the document's mode names, and
//! a mode switch starts the other program (owner, Q10).
//!
//! **This first stage compiles cold.** Each `COMPILE` runs the engine in
//! full, as a child process of this program invoked as `xelatex` (as the
//! pdfTeX host's export runs): its pages stream to the client as they are
//! shipped, through a socket of its own (`FLASHTEX_DISPLAY_LIST`); a run
//! that changed the `.aux` (or another file LaTeX reads back) is followed
//! by another, up to five passes, each pass's pages replacing the
//! previous (spec §6.4); then the external tools (`tools`) and their
//! follow-up compiles. `DONE.pdf` is FlashTeX's own PDF, written from the
//! display list (PLAN.md §3.2). The resident, incremental engine of
//! Classic (checkpoints, convergence, S₀) is the next stage; its pieces
//! move into the shared runtime crate then (PLAN.md §3.3). Classic's
//! `flashtex-host` is not changed by this program.
//!
//! The formats are FlashTeX's own (`format`), built from the user's TeX
//! Live as fmtutil builds them and cached.
//!
//! ```text
//! flashtex-host-unicode --socket PATH [--format NAME]... [--once [--accept-timeout SECONDS]]
//!     [--external-tools off|auto] [--tool-timeout SECONDS] [--no-warm]
//! ```
//!
//! Invoked as `xetex` or `xelatex` (`argv[0]`), it is the engine itself.

pub mod compile;
pub mod fonts;
mod format;
pub mod tools;

use compile::{send_json, Accept, Job, Out, Running};
use flashtex_display_list::frame::read_frame;
use flashtex_display_list::json::{obj, s as js, Json};
use flashtex_display_list::{kind, PROTOCOL, VERSION_MAJOR};
use std::collections::HashMap;
use std::io::{BufReader, BufWriter};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The host's options.
#[derive(Clone, Debug)]
pub struct Opts {
    pub socket: PathBuf,
    pub formats: Vec<String>,
    pub once: bool,
    pub accept_timeout: Option<Duration>,
    pub external_tools: bool,
    pub tool_timeout: Duration,
}

const USAGE: &str = "usage: flashtex-host-unicode --socket PATH [--format NAME]... [--once [--accept-timeout SECONDS]] [--external-tools off|auto] [--tool-timeout SECONDS] [--no-warm]";

impl Opts {
    pub fn parse(args: &[String]) -> Result<Opts, String> {
        let mut o = Opts {
            socket: PathBuf::new(),
            formats: vec![],
            once: false,
            accept_timeout: None,
            external_tools: false,
            tool_timeout: Duration::from_secs(120),
        };
        let mut i = 0;
        while i < args.len() {
            let a = args[i].as_str();
            let mut val = || {
                i += 1;
                args.get(i)
                    .cloned()
                    .ok_or_else(|| format!("{a} needs a value"))
            };
            match a {
                "--socket" => o.socket = PathBuf::from(val()?),
                "--format" => o.formats.push(val()?),
                "--once" => o.once = true,
                "--accept-timeout" => {
                    let s: f64 = val()?.parse().map_err(|_| "--accept-timeout: seconds")?;
                    o.accept_timeout = Some(Duration::from_secs_f64(s));
                }
                "--external-tools" => o.external_tools = val()? == "auto",
                "--tool-timeout" => {
                    let s: f64 = val()?.parse().map_err(|_| "--tool-timeout: seconds")?;
                    o.tool_timeout = Duration::from_secs_f64(s);
                }
                // the pdfTeX host's warm-up has nothing to warm here
                "--no-warm" => {}
                _ => return Err(format!("unknown option {a}")),
            }
            i += 1;
        }
        if o.socket.as_os_str().is_empty() {
            return Err("--socket PATH is required".into());
        }
        if o.formats.is_empty() {
            o.formats.push("xelatex".into());
        }
        Ok(o)
    }
}

/// The program's entry point: the engine when invoked as `xetex` or
/// `xelatex`, else the host.
pub fn main(argv: Vec<String>) -> i32 {
    let name = argv
        .first()
        .map(|a| flashtex_engine::system::program_name_from_argv0(a))
        .unwrap_or_default();
    if matches!(name.as_str(), "xetex" | "xelatex") {
        crate::driver::run(argv);
    }
    let opts = match Opts::parse(&argv[1..]) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("flashtex-host-unicode: {e}\n{USAGE}");
            return 2;
        }
    };
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("flashtex-host-unicode: {e}");
            return 1;
        }
    };
    let (texmf, formats) = prepare(&exe, &opts);
    println!("flashtex-host-unicode: {texmf}");
    serve(&exe, &opts, texmf, formats)
}

/// The TeX Live the engine reads and each format made ready: `HELLO.texmf`
/// and the directory of each ready format.
fn prepare(exe: &Path, opts: &Opts) -> (Json, HashMap<String, PathBuf>) {
    let texlive = flashtex_engine::resolver::discover_texlive().map(|t| t.describe());
    let resolver = flashtex_engine::system::with_resolver_for("xelatex", |r| r.describe());
    let mut ready = HashMap::new();
    let mut fj = vec![];
    for f in &opts.formats {
        let t = Instant::now();
        match format::ensure(exe, f) {
            Ok(dir) => {
                fj.push(obj([
                    ("name", js(f.clone())),
                    ("status", js("ready")),
                    (
                        "ms",
                        Json::Num((t.elapsed().as_secs_f64() * 1e4).round() / 10.0),
                    ),
                ]));
                ready.insert(f.clone(), dir);
            }
            Err(e) => fj.push(obj([
                ("name", js(f.clone())),
                ("status", js("failed")),
                ("error", js(e)),
            ])),
        }
    }
    let programs = flashtex_engine::host::external::Programs::discover();
    let texmf = obj([
        ("texlive", texlive.map_or(Json::Null, js)),
        ("resolver", js(resolver)),
        ("bundle", Json::Null),
        ("formats", Json::Arr(fj)),
        ("tools", programs.json()),
        (
            "external_tools",
            js(if opts.external_tools { "auto" } else { "off" }),
        ),
    ]);
    (texmf, ready)
}

fn serve(exe: &Path, opts: &Opts, texmf: Json, formats: HashMap<String, PathBuf>) -> i32 {
    let _ = std::fs::remove_file(&opts.socket);
    let listener = match UnixListener::bind(&opts.socket) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("flashtex-host-unicode: {}: {e}", opts.socket.display());
            return 1;
        }
    };
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&opts.socket, std::fs::Permissions::from_mode(0o600));
    }
    println!(
        "flashtex-host-unicode: listening on {}",
        opts.socket.display()
    );
    let parent = std::os::unix::process::parent_id();
    let started = Instant::now();
    let host = Arc::new(Host {
        exe: exe.to_path_buf(),
        texmf,
        formats: Mutex::new(formats),
        programs: flashtex_engine::host::external::Programs::discover(),
        external_tools: opts.external_tools,
        tool_timeout: opts.tool_timeout,
    });
    let _ = listener.set_nonblocking(opts.once);
    let mut n = 0u64;
    loop {
        let conn = match listener.accept() {
            Ok((s, _)) => s,
            Err(_) if opts.once => {
                // until the client connects: gone with the process that
                // started us, or after --accept-timeout
                let orphaned = std::os::unix::process::parent_id() != parent;
                let late = opts.accept_timeout.is_some_and(|t| started.elapsed() > t);
                if orphaned || late {
                    let _ = std::fs::remove_file(&opts.socket);
                    return 1;
                }
                std::thread::sleep(Duration::from_millis(20));
                continue;
            }
            Err(e) => {
                eprintln!("flashtex-host-unicode: accept: {e}");
                continue;
            }
        };
        let _ = conn.set_nonblocking(false);
        let h = host.clone();
        n += 1;
        if opts.once {
            connection(&h, conn, n);
            let _ = std::fs::remove_file(&opts.socket);
            return 0;
        }
        std::thread::spawn(move || connection(&h, conn, n));
    }
}

/// What every connection shares.
struct Host {
    exe: PathBuf,
    texmf: Json,
    formats: Mutex<HashMap<String, PathBuf>>,
    programs: flashtex_engine::host::external::Programs,
    external_tools: bool,
    tool_timeout: Duration,
}

impl Host {
    /// The directory of format `f`, made ready now if it was not at start.
    fn format_dir(&self, f: &str) -> Result<PathBuf, String> {
        if let Some(d) = self.formats.lock().unwrap().get(f) {
            return Ok(d.clone());
        }
        let d = format::ensure(&self.exe, f)?;
        self.formats
            .lock()
            .unwrap()
            .insert(f.to_string(), d.clone());
        Ok(d)
    }
}

enum Msg {
    Compile(Job, Instant),
}

fn connection(host: &Arc<Host>, conn: UnixStream, n: u64) {
    let Ok(wconn) = conn.try_clone() else { return };
    let out: Out = Arc::new(Mutex::new(Box::new(BufWriter::new(wconn))));
    let mut reader = BufReader::new(conn);
    // HELLO first
    let Ok(Some((k, body))) = read_frame(&mut reader) else {
        return;
    };
    let hello = Json::parse(&String::from_utf8_lossy(&body)).unwrap_or(Json::Null);
    if k != kind::C_HELLO {
        send_json(
            &out,
            kind::ERROR,
            &obj([("code", js("protocol")), ("message", js("HELLO first"))]),
        );
        return;
    }
    let version = hello
        .get("version")
        .and_then(Json::as_array)
        .map(|v| v.iter().filter_map(Json::as_i64).collect::<Vec<_>>());
    let (major, minor) = match version.as_deref() {
        Some([a, b, ..]) => (*a, *b),
        _ => (VERSION_MAJOR as i64, 2),
    };
    if major != VERSION_MAJOR as i64 {
        send_json(
            &out,
            kind::ERROR,
            &obj([
                ("code", js("version")),
                ("message", js("display-list-v3 only")),
            ]),
        );
        return;
    }
    let accepted: Vec<String> = hello
        .get("accept")
        .and_then(Json::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let accept = Accept {
        color_spaces: accepted
            .iter()
            .any(|a| a == flashtex_display_list::accept::COLOR_SPACES),
        line_state: accepted
            .iter()
            .any(|a| a == flashtex_display_list::accept::LINE_STATE),
    };
    let caps = [
        "compile",
        "cancel",
        "diagnostics",
        "font-programs",
        "font-formats",
        "have-fonts",
        "export",
        "external-tools",
        "halt-on-error",
        "buffers",
        "edits",
    ];
    send_json(
        &out,
        kind::HELLO,
        &obj([
            ("protocol", js(PROTOCOL)),
            (
                "version",
                Json::Arr(vec![
                    Json::Int(VERSION_MAJOR as i64),
                    Json::Int(minor.min(3)),
                ]),
            ),
            (
                "server",
                js(format!(
                    "flashtex-host-unicode {}",
                    env!("CARGO_PKG_VERSION")
                )),
            ),
            (
                "engine",
                js("XeTeX 3.141592653-2.6-0.999998 (FlashTeX Unicode engine)"),
            ),
            (
                "capabilities",
                Json::Arr(caps.iter().map(|c| js(*c)).collect()),
            ),
            ("texmf", host.texmf.clone()),
        ]),
    );
    let scratch =
        std::env::temp_dir().join(format!("flashtex-unicode-{}-{}", std::process::id(), n));
    let running = Arc::new(Mutex::new(Running::default()));
    let current: Arc<Mutex<Option<i64>>> = Arc::new(Mutex::new(None));
    let (tx, rx) = mpsc::channel::<Msg>();
    let worker = {
        let (host, out, running, current, scratch) = (
            host.clone(),
            out.clone(),
            running.clone(),
            current.clone(),
            scratch.clone(),
        );
        std::thread::spawn(move || {
            compile_thread(&host, &out, &running, &current, &scratch, &accept, rx)
        })
    };
    while let Ok(Some((k, body))) = read_frame(&mut reader) {
        let j = Json::parse(&String::from_utf8_lossy(&body)).unwrap_or(Json::Null);
        match k {
            kind::COMPILE => match Job::parse(&j, &scratch) {
                Ok(job) => {
                    // a newer compile supersedes the running one (spec §6.3)
                    cancel(&running);
                    let _ = tx.send(Msg::Compile(job, Instant::now()));
                }
                Err(e) => {
                    let mut f = vec![
                        ("code".to_string(), js("bad-request")),
                        ("message".to_string(), js(e)),
                    ];
                    if let Some(id) = j.int_field("id") {
                        f.push(("id".into(), Json::Int(id)));
                    }
                    send_json(&out, kind::ERROR, &Json::Obj(f));
                }
            },
            kind::CANCEL => {
                if j.int_field("id").is_some() && j.int_field("id") == *current.lock().unwrap() {
                    cancel(&running);
                }
            }
            kind::BYE => break,
            _ => {}
        }
    }
    cancel(&running);
    drop(tx);
    let _ = worker.join();
    let _ = std::fs::remove_dir_all(&scratch);
}

/// Stop the running engine, if any (its compile's `DONE` says `cancelled`).
fn cancel(running: &Arc<Mutex<Running>>) {
    let mut r = running.lock().unwrap();
    r.cancelled = true;
    if let Some(c) = r.child.as_mut() {
        let _ = c.kill();
    }
}

fn compile_thread(
    host: &Arc<Host>,
    out: &Out,
    running: &Arc<Mutex<Running>>,
    current: &Arc<Mutex<Option<i64>>>,
    scratch: &Path,
    accept: &Accept,
    rx: mpsc::Receiver<Msg>,
) {
    let mut memory = tools::Memory::default();
    while let Ok(Msg::Compile(mut job, mut t0)) = rx.recv() {
        // compiles superseded before they started only apply their edits
        while let Ok(Msg::Compile(next, t)) = rx.try_recv() {
            let _ = job.apply_files();
            done(out, &job, "cancelled", None, 0, 0, 0, None, t0, None);
            job = next;
            t0 = t;
        }
        *current.lock().unwrap() = Some(job.id);
        *running.lock().unwrap() = Running::default();
        run_job(host, out, running, scratch, accept, &job, t0, &mut memory);
        *current.lock().unwrap() = None;
    }
}

#[allow(clippy::too_many_arguments)]
fn run_job(
    host: &Host,
    out: &Out,
    running: &Arc<Mutex<Running>>,
    scratch: &Path,
    accept: &Accept,
    job: &Job,
    t0: Instant,
    memory: &mut tools::Memory,
) {
    if let Err(e) = job.apply_files() {
        send_json(
            out,
            kind::ERROR,
            &obj([
                ("id", Json::Int(job.id)),
                ("code", js("files")),
                ("message", js(e)),
            ]),
        );
        done(out, job, "failed", None, 0, 0, 0, None, t0, None);
        return;
    }
    let fmt_dir = match host.format_dir(&job.format) {
        Ok(d) => d,
        Err(e) => {
            send_json(
                out,
                kind::ERROR,
                &obj([
                    ("id", Json::Int(job.id)),
                    ("code", js("format")),
                    ("message", js(e)),
                ]),
            );
            done(out, job, "failed", None, 0, 0, 0, None, t0, None);
            return;
        }
    };
    let _ = std::fs::create_dir_all(scratch);
    // `[fonts]` (PROPOSAL.md §4.5), read after the client's buffers are
    // written, so an unsaved manifest counts
    let fonts = fonts::setup(&job.root, &job.format);
    let mut notes = fonts.notes;
    let with_fonts;
    let job = if fonts.code.is_empty() {
        job
    } else if job.main.contains(['{', '}', '%', '\\', '#']) {
        notes.push((
            "warning",
            format!("[fonts] not applied: the main file's name {:?} cannot follow them on TeX's first line", job.main),
        ));
        job
    } else {
        with_fonts = Job {
            preamble: fonts.code,
            ..job.clone()
        };
        &with_fonts
    };
    let auto = job
        .external_tools
        .as_deref()
        .map_or(host.external_tools, |p| p == "auto");
    let mut cause = None;
    let mut rounds = 0;
    let mut ran_any = false;
    loop {
        started(out, job, &host.exe);
        for (sev, m) in notes.drain(..) {
            send_json(
                out,
                kind::DIAGNOSTIC,
                &obj([
                    ("id", Json::Int(job.id)),
                    ("severity", js(sev)),
                    ("message", js(m)),
                    ("file", js("flashtex.toml")),
                ]),
            );
        }
        let o = compile::compile(&host.exe, &fmt_dir, job, accept, out, running, t0, scratch);
        done(
            out,
            job,
            o.status,
            o.exit_code,
            o.pages,
            o.diagnostics,
            o.passes,
            o.first_page_ms,
            t0,
            cause,
        );
        if job.export || o.status == "cancelled" || o.status == "failed" {
            break;
        }
        let (ran, changed) =
            tools::after_compile(job, auto, &host.programs, memory, out, host.tool_timeout);
        ran_any |= ran;
        if !changed || rounds >= compile::MAX_ROUNDS || running.lock().unwrap().cancelled {
            if job.external_tools.is_some() {
                let mut f = vec![
                    ("id".to_string(), Json::Int(job.id)),
                    ("event".to_string(), js("settled")),
                    ("ran".to_string(), Json::Bool(ran_any)),
                    ("rounds".to_string(), Json::Int(rounds as i64)),
                ];
                if changed && rounds >= compile::MAX_ROUNDS {
                    f.push(("limit".into(), Json::Bool(true)));
                }
                send_json(out, kind::TOOL, &Json::Obj(f));
            }
            break;
        }
        rounds += 1;
        cause = Some("tools");
    }
}

fn started(out: &Out, job: &Job, exe: &Path) {
    let mut argv = job.argv();
    argv[0] = exe.to_string_lossy().into_owned();
    send_json(
        out,
        kind::STARTED,
        &obj([
            ("id", Json::Int(job.id)),
            ("pid", Json::Int(std::process::id() as i64)),
            ("argv", Json::Arr(argv.into_iter().map(js).collect())),
            (
                "output_dir",
                js(job.output_dir.to_string_lossy().into_owned()),
            ),
            ("mode", js(if job.export { "export" } else { "resident" })),
            ("keep", Json::Bool(false)),
            ("incremental", Json::Bool(false)),
        ]),
    );
}

#[allow(clippy::too_many_arguments)]
fn done(
    out: &Out,
    job: &Job,
    status: &str,
    exit_code: Option<i32>,
    pages: u32,
    diagnostics: usize,
    passes: usize,
    first_page_ms: Option<f64>,
    t0: Instant,
    cause: Option<&str>,
) {
    let pdf = job.output_dir.join(format!("{}.pdf", job.jobname));
    let log = job.output_dir.join(format!("{}.log", job.jobname));
    let ms = |v: f64| Json::Num((v * 10.0).round() / 10.0);
    let mut f = vec![
        ("id".to_string(), Json::Int(job.id)),
        ("status".to_string(), js(status)),
        (
            "exit_code".to_string(),
            exit_code.map_or(Json::Null, |c| Json::Int(c as i64)),
        ),
        ("pages".to_string(), Json::Int(pages as i64)),
        (
            "bytes".to_string(),
            Json::Int(std::fs::metadata(&pdf).map_or(0, |m| m.len() as i64)),
        ),
        ("diagnostics".to_string(), Json::Int(diagnostics as i64)),
        (
            "elapsed_ms".to_string(),
            ms(t0.elapsed().as_secs_f64() * 1e3),
        ),
        (
            "first_page_ms".to_string(),
            first_page_ms.map_or(Json::Null, ms),
        ),
        ("pdf".to_string(), js(pdf.to_string_lossy().into_owned())),
        ("log".to_string(), js(log.to_string_lossy().into_owned())),
        (
            "mode".to_string(),
            js(if job.export { "export" } else { "cold" }),
        ),
        ("passes".to_string(), Json::Int(passes as i64)),
        ("typeset_pages".to_string(), Json::Int(pages as i64)),
        ("keep".to_string(), Json::Bool(false)),
    ];
    if let Some(c) = cause {
        f.push(("cause".into(), js(c)));
    }
    send_json(out, kind::DONE, &Json::Obj(f));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_parse() {
        let a: Vec<String> = [
            "--socket",
            "/tmp/s",
            "--once",
            "--accept-timeout",
            "2",
            "--external-tools",
            "auto",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let o = Opts::parse(&a).unwrap();
        assert_eq!(o.socket, PathBuf::from("/tmp/s"));
        assert!(o.once && o.external_tools);
        assert_eq!(o.formats, ["xelatex"]);
        assert!(Opts::parse(&[]).is_err());
    }
}
