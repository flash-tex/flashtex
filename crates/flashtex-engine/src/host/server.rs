//! `flashtex-host --socket PATH`: the engine host's socket server
//! (docs/protocol/display-list-v3.md §6; DESIGN.md §3, §5, §6.1).
//!
//! A client connects, says `HELLO`, and asks to `COMPILE` a project. The host
//! keeps **one resident engine** per document (`crate::incr::Session`, on its
//! own thread: `super::resident`): the first compile is a full run that takes
//! S₀ and a checkpoint after every page; a later compile finds what the
//! client's edits changed, restores the newest checkpoint before them, and
//! re-typesets from there until the state converges with the previous run
//! (DESIGN.md §5.1–§5.4). The display list of every page the engine ships
//! out streams to the client as it is shipped (`crate::displaylist`), so the
//! edited page arrives first; pages the compile did not re-typeset are
//! either kept by the client (`"incremental": true`, protocol 3.1) or sent
//! again from the host's cache (the default, as in 3.0), always in page
//! order. `PAGES` messages say which of the client's pages are current and
//! which are stale while the rest is re-typeset; diagnostics come from the
//! engine's terminal; `DONE` ends each compile.
//!
//! With `"external_tools": "auto"` (a trusted project; protocol 3.2) or
//! `--external-tools auto`, the host also runs bibtex, biber and makeindex
//! from the user's TeX Live when latexmk would, after the compile's `DONE`,
//! and compiles again with what they made (`super::external`).
//!
//! `"export": true` asks instead for a one-shot run of the engine as a child
//! process with pdflatex's command line (the exported PDF, compressed, which
//! the parity gates compare with pdflatex's): the path of lane P3, kept.
//!
//! GPL-2.0-or-later like the engine. The app never links this program; it
//! runs it and talks to its socket.
//!
//! ```text
//! flashtex-host --socket /tmp/flashtex.sock [--engine PATH] [--format NAME]...
//!     [--once [--accept-timeout SECONDS]] [--no-warm] [--s0-cache DIR]
//!     [--profile low-memory|balanced|high-performance] [--budget BYTES] [--timed SECONDS]
//!     [--keep-warm MS] [--keep-warm-pause US]
//!     [--external-tools off|auto] [--tool-timeout SECONDS]
//! ```
//!
//! `--once` serves one connection, then exits. Until that connection comes,
//! the host also exits (removing its socket) when the process that started
//! it is gone (Unix: its parent changed), or after `--accept-timeout`
//! seconds, so a client killed before it connected leaves no host behind.
//!
//! `--profile NAME` (or `FLASHTEX_PROFILE`): the performance mode the host
//! starts in, `balanced` by default (`crate::profile`: the checkpoints'
//! budget and spacing, keep-warm, prepare-ahead, idle trimming); a client
//! may choose another (`HELLO.profile`, `PROFILE`; capability `profile-v1`).
//! `--budget`, `--timed` and `--keep-warm` pin their knob in every mode.
//!
//! `--keep-warm MS` (or `FLASHTEX_HOST_KEEP_WARM_MS`; default 2000, the
//! owner's decision 10A, and High Performance's 10000; 0 turns it off): after each compile the engine
//! thread polls for the next request for MS milliseconds instead of
//! sleeping, so that the next keystroke's compile starts on a core already
//! at full speed (an idle Apple Silicon core runs a burst at a half to a
//! third of its speed). It costs a busy core only while the user types and
//! MS after the last keystroke, never while idle; `--keep-warm-pause US`
//! (default 100; 0 spins throughout) alternates sleeps and spins of US
//! microseconds, which kept the latency at about half the CPU.
//! Measured in docs/evidence/p4-finish-2026-09-30/.
//!
//! At start-up it reports which TeX Live (or bundle) the engine reads and
//! makes each `--format` ready (default `pdflatex`), building it into the
//! format cache if needed (see [`prepare`]); HELLO carries both. Then, unless
//! `--no-warm`, it warms the resident engine up (kpathsea, the font map, the
//! format: a one-page document) so that the first compile, or the reopening
//! of a document from its persisted S₀ (`--s0-cache`, DESIGN.md §1.2's
//! ≤ 100 ms reopen), pays none of that.
//!
//! `--engine PATH` is the engine program the export runs and the format
//! preparation start (default: this program, which runs as the engine when
//! invoked as `pdftex`).

use flashtex_display_list::frame::{read_frame, write_frame};
use flashtex_display_list::json::{obj, s as js, Json};
use flashtex_display_list::transport::Stream;
use flashtex_display_list::{kind, PROTOCOL, VERSION_MAJOR, VERSION_MINOR};
use std::collections::HashSet;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub(crate) type Out = Arc<Mutex<BufWriter<Stream>>>;

/// `--keep-warm-pause`'s default: 100 us sleeps between 100 us spins kept
/// the latency of a full spin (plain/full 10/120/1,000, 300 ms between
/// keystrokes) at about half its CPU: 30-33 against 60 CPU s per minute of
/// typing (docs/evidence/p4-finish-2026-09-30/, `warm_cost.py`).
const DEFAULT_KEEP_WARM_PAUSE_US: u64 = 100;

/// Mark the calling thread as doing user-interactive work (macOS QoS
/// `USER_INTERACTIVE`): a keystroke's compile is what the user waits for.
/// Without it, the engine thread, idle between keystrokes, wakes up for a
/// COMPILE with the default QoS and runs its first milliseconds on an
/// efficiency core or a performance core still at a low clock: measured
/// (docs/evidence/p4-finish-2026-09-30/), the edited page took 18 ms of
/// thread CPU after 300 ms of idle against 6.5 ms back to back, every stage
/// alike. Threads the engine thread starts (the parallel restore's)
/// inherit the class. `FLASHTEX_HOST_QOS=default` leaves the class alone
/// (for A/B measurements). Elsewhere a no-op.
pub(crate) fn interactive_qos() {
    #[cfg(target_os = "macos")]
    {
        if std::env::var("FLASHTEX_HOST_QOS").as_deref() == Ok("default") {
            return;
        }
        extern "C" {
            fn pthread_set_qos_class_self_np(qos_class: u32, relative_priority: i32) -> i32;
        }
        // <sys/qos.h>: QOS_CLASS_USER_INTERACTIVE
        const QOS_CLASS_USER_INTERACTIVE: u32 = 0x21;
        // SAFETY: sets the calling thread's own scheduling class; no memory
        // is shared.
        unsafe {
            pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE, 0);
        }
    }
}

pub(crate) fn send(out: &Out, k: u8, body: &[u8]) -> bool {
    let mut w = out.lock().unwrap_or_else(|p| p.into_inner());
    write_frame(&mut *w, k, body).is_ok() && w.flush().is_ok()
}

pub(crate) fn send_json(out: &Out, k: u8, j: &Json) -> bool {
    send(out, k, j.to_string().as_bytes())
}

pub(crate) fn error(out: &Out, id: Option<i64>, code: &str, message: &str) {
    let mut kv = vec![
        ("code".to_string(), js(code)),
        ("message".to_string(), js(message)),
    ];
    if let Some(i) = id {
        kv.insert(0, ("id".into(), Json::Int(i)));
    }
    send_json(out, kind::ERROR, &Json::Obj(kv));
}

pub(crate) struct Config {
    /// The engine program for export runs and format preparation.
    pub engine: PathBuf,
    pub engine_version: String,
    /// What the engine reads (TeX Live or the bundle) and the formats made
    /// ready at start-up, for HELLO.
    pub texmf: Json,
    /// Where S₀ of each document persists (DESIGN.md §5.1), if anywhere.
    pub s0_cache: Option<PathBuf>,
    /// The resident engine's options; a performance mode's knobs (budget,
    /// checkpoint spacing) are applied over them (`crate::profile`).
    pub opts: crate::incr::Options,
    /// The performance mode the host starts in (`--profile`,
    /// `FLASHTEX_PROFILE`; Balanced by default): its knobs include the
    /// keep-warm window (`--keep-warm MS`: after a compile, the engine
    /// thread polls for the next request this long instead of sleeping,
    /// `resident::Engine::run`). A client's `HELLO` or `PROFILE` changes it.
    pub profile: crate::profile::Profile,
    /// Knobs the command line or the environment fixed: no profile changes
    /// them.
    pub pinned: crate::profile::Pinned,
    /// `--keep-warm-pause US`: while warm, alternate sleeps and spins of
    /// this length instead of spinning throughout (0: spin).
    pub keep_warm_pause: std::time::Duration,
    /// External tools: the programs found, the default policy, the timeout.
    pub tools: Arc<super::external::Config>,
}

/// One client connection, as the engine thread sees it.
pub(crate) struct Conn {
    pub id: u64,
    pub out: Out,
    /// COMPILE requests sent to the engine thread and not yet taken: when
    /// one is waiting, the running compile's later output is not sent (the
    /// newer compile sends what is current then).
    pub queued: AtomicU64,
    /// Compile ids the client cancelled.
    pub cancelled: Mutex<HashSet<i64>>,
    /// The client's protocol minor version (from its HELLO).
    pub minor: i64,
    /// The client accepted `diag-v1` (its HELLO's `accept`): it gets
    /// `DIAG` messages instead of `DIAGNOSTIC`s (spec §6.7).
    pub diag: bool,
    /// The client accepted `progress-v1`: it gets `PROGRESS` heartbeats (spec §6.8).
    pub progress: bool,
}

impl Conn {
    pub fn is_cancelled(&self, id: i64) -> bool {
        self.cancelled
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .contains(&id)
    }
}

/// What the connection threads ask of the engine thread.
pub(crate) enum Req {
    Compile {
        conn: Arc<Conn>,
        req: Json,
        t0: Instant,
    },
    Closed(u64),
    Warm(mpsc::Sender<Result<f64, String>>),
    /// A client chose a performance mode (`HELLO.profile` or `PROFILE`,
    /// capability `profile-v1`): applied between compiles; `reply`: send
    /// the client a `PROFILE` with the effective knobs once applied.
    Profile {
        conn: Arc<Conn>,
        profile: crate::profile::Profile,
        reply: bool,
    },
    /// The external tools' worker is done (`super::external`).
    ToolsDone {
        gen: u64,
        conn: Arc<Conn>,
        req: Json,
        id: i64,
        report: super::external::Report,
    },
}

static CONNECTIONS: AtomicU64 = AtomicU64::new(0);

/// `flashtex-host --socket PATH ...`: `args` is the whole command line;
/// returns the exit status.
pub fn main(args: Vec<String>) -> i32 {
    let mut socket = None;
    let mut engine = None;
    let mut once = false;
    // `--once`: the process that started this host, so that the host goes
    // when it goes (taken before the format is prepared, which takes time).
    #[cfg(unix)]
    let parent = std::os::unix::process::parent_id();
    let mut accept_timeout: Option<f64> = None;
    let mut warm = true;
    let mut s0_cache = std::env::var_os("FLASHTEX_S0_CACHE").map(PathBuf::from);
    let mut opts = crate::incr::Options::default();
    let mut pinned = crate::profile::Pinned::from_env();
    let mut mode = crate::profile::Mode::from_env();
    let mut keep_warm_pause_us: u64 = std::env::var("FLASHTEX_HOST_KEEP_WARM_PAUSE_US")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_KEEP_WARM_PAUSE_US);
    let mut formats: Vec<String> = Vec::new();
    let mut tools_default = super::external::Policy::Off;
    let mut tool_timeout = 120.0f64;
    let mut i = 1;
    while i < args.len() {
        let v = args.get(i + 1).cloned();
        match args[i].as_str() {
            "--socket" => {
                socket = v;
                i += 1;
            }
            "--engine" => {
                engine = v;
                i += 1;
            }
            "--once" => once = true,
            "--accept-timeout" => {
                match v.as_deref().and_then(|v| v.parse::<f64>().ok()) {
                    Some(t) if t > 0.0 => accept_timeout = Some(t),
                    _ => {
                        eprintln!("flashtex-host: --accept-timeout SECONDS");
                        return 2;
                    }
                }
                i += 1;
            }
            "--no-warm" => warm = false,
            "--s0-cache" => {
                s0_cache = v.map(PathBuf::from);
                i += 1;
            }
            "--profile" => {
                match v.as_deref().and_then(crate::profile::Mode::parse) {
                    Some(m) => mode = m,
                    None => {
                        eprintln!("flashtex-host: --profile low-memory|balanced|high-performance");
                        return 2;
                    }
                }
                i += 1;
            }
            "--budget" => {
                match v.and_then(|v| v.parse().ok()) {
                    Some(b) => pinned.budget = Some(b),
                    None => {
                        eprintln!("flashtex-host: --budget BYTES");
                        return 2;
                    }
                }
                i += 1;
            }
            "--timed" => {
                match v.and_then(|v| v.parse().ok()) {
                    Some(t) => pinned.timed_s = Some(t),
                    None => {
                        eprintln!("flashtex-host: --timed SECONDS");
                        return 2;
                    }
                }
                i += 1;
            }
            "--keep-warm" => {
                match v.and_then(|v| v.parse().ok()) {
                    Some(t) => pinned.keep_warm_ms = Some(t),
                    None => {
                        eprintln!("flashtex-host: --keep-warm MS");
                        return 2;
                    }
                }
                i += 1;
            }
            "--keep-warm-pause" => {
                match v.and_then(|v| v.parse().ok()) {
                    Some(t) => keep_warm_pause_us = t,
                    None => {
                        eprintln!("flashtex-host: --keep-warm-pause MICROSECONDS");
                        return 2;
                    }
                }
                i += 1;
            }
            "--format" => {
                if let Some(f) = v {
                    formats.push(f);
                }
                i += 1;
            }
            "--external-tools" => {
                match v.as_deref().and_then(super::external::Policy::parse) {
                    Some(p) => tools_default = p,
                    None => {
                        eprintln!("flashtex-host: --external-tools off|auto");
                        return 2;
                    }
                }
                i += 1;
            }
            "--tool-timeout" => {
                match v.and_then(|v| v.parse::<f64>().ok()).filter(|t| *t > 0.0) {
                    Some(t) => tool_timeout = t,
                    None => {
                        eprintln!("flashtex-host: --tool-timeout SECONDS");
                        return 2;
                    }
                }
                i += 1;
            }
            "--help" | "-h" => {
                println!("usage: flashtex-host --socket PATH [--engine PATH] [--format NAME]... [--once [--accept-timeout SECONDS]] [--no-warm] [--s0-cache DIR] [--profile low-memory|balanced|high-performance] [--budget BYTES] [--timed SECONDS] [--keep-warm MS] [--keep-warm-pause US] [--external-tools off|auto] [--tool-timeout SECONDS]");
                println!("       flashtex-host serve|iserve|bench|open|selftest|layout ... (see src/host/tools.rs)");
                return 0;
            }
            a => {
                eprintln!("flashtex-host: unknown argument {a}");
                return 2;
            }
        }
        i += 1;
    }
    let Some(socket) = socket else {
        eprintln!("flashtex-host: --socket PATH is required");
        return 2;
    };
    let engine = engine
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
        .unwrap_or_else(|| PathBuf::from("flashtex-initex"));
    // This program's own version needs no process (a process start costs
    // tens of milliseconds before the first compile); another engine's is
    // asked for.
    let engine_version = if is_this_program(&engine) {
        crate::system::version_text()
            .lines()
            .next()
            .unwrap_or_default()
            .to_string()
    } else {
        crate::os::engine_command(&engine)
            .arg("-version")
            .output()
            .ok()
            .and_then(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .next()
                    .map(str::to_string)
            })
            .unwrap_or_default()
    };
    if formats.is_empty() {
        formats.push("pdflatex".into());
    }
    let mut texmf = prepare(&engine, &formats);
    let tools = Arc::new(super::external::Config {
        programs: super::external::Programs::discover(),
        default: tools_default,
        timeout: std::time::Duration::from_secs_f64(tool_timeout),
    });
    if let Json::Obj(kv) = &mut texmf {
        kv.push(("tools".into(), tools.programs.json()));
        kv.push(("external_tools".into(), js(tools.default.name())));
    }
    let profile = crate::profile::Profile::new(mode, &pinned);
    opts.apply_profile(&profile);
    let cfg = Arc::new(Config {
        engine,
        engine_version,
        texmf,
        s0_cache,
        opts,
        profile,
        pinned,
        keep_warm_pause: std::time::Duration::from_micros(keep_warm_pause_us),
        tools,
    });
    // The resident engine: one thread, which owns every engine's state
    // (thread-local) and runs the compiles one at a time. A deep stack, as
    // TeX's recursion may need.
    let (tx, rx) = mpsc::channel::<Req>();
    let cfg2 = cfg.clone();
    let tx2 = tx.clone();
    let engine_thread = std::thread::Builder::new()
        .name("engine".into())
        .stack_size(512 << 20)
        .spawn(move || {
            interactive_qos();
            super::resident::Engine::new(cfg2, tx2).run(rx)
        });
    if let Err(e) = engine_thread {
        eprintln!("flashtex-host: cannot start the engine thread: {e}");
        return 1;
    }
    // Warm the resident engine up before saying what was prepared (one JSON
    // line, with `warm_ms`), then listen.
    let mut said = match &cfg.texmf {
        Json::Obj(kv) => kv.clone(),
        _ => vec![],
    };
    if warm && formats.iter().any(|f| f == "pdflatex") {
        let (dtx, drx) = mpsc::channel();
        let _ = tx.send(Req::Warm(dtx));
        match drx.recv() {
            Ok(Ok(s)) => said.push(("warm_ms".into(), Json::Num((s * 1e4).round() / 10.0))),
            Ok(Err(e)) => said.push(("warm_error".into(), js(e))),
            Err(_) => {}
        }
    }
    say(&format!("flashtex-host: {}", Json::Obj(said)));
    let _ = std::fs::remove_file(&socket);
    // Owner-only from the start, or not at all (`os::bind_owner_only`
    // removes a socket it could not restrict): the host never listens on a
    // socket another account could connect to.
    let listener = match crate::os::bind_owner_only(Path::new(&socket)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("flashtex-host: {socket}: cannot listen there, owner-only: {e}");
            return 1;
        }
    };
    // Ready: a supervisor may wait for this line.
    say(&format!("flashtex-host: listening on {socket}"));
    // `--once` serves the process that started it: when that process is
    // gone before it connected (killed, crashed), or no connection comes
    // within `--accept-timeout`, the host removes its socket and exits
    // instead of waiting for ever. Once connected, the connection's end
    // ends the host as before.
    // One lock decides between the two: the watcher exits only while it
    // holds it and `connected` is false, and an accepted connection is
    // marked under it, so a connection accepted is never dropped by the
    // watcher's exit (it either sees `connected`, or exits before the
    // accept loop can mark it).
    let connected = Arc::new(std::sync::Mutex::new(false));
    if once {
        let (connected, socket) = (connected.clone(), socket.clone());
        let t0 = Instant::now();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(200));
            let Ok(guard) = connected.lock() else { return };
            if *guard {
                return;
            }
            #[cfg(unix)]
            let orphaned = std::os::unix::process::parent_id() != parent;
            #[cfg(not(unix))]
            let orphaned = false;
            let late = accept_timeout.is_some_and(|t| t0.elapsed().as_secs_f64() > t);
            if orphaned || late {
                let _ = std::fs::remove_file(&socket);
                eprintln!(
                    "flashtex-host: {} before a connection (--once); exiting",
                    if orphaned {
                        "the parent process exited"
                    } else {
                        "--accept-timeout passed"
                    }
                );
                std::process::exit(0); // still holding the lock
            }
            drop(guard);
        });
    }
    for conn in listener.incoming() {
        let Ok(conn) = conn else { continue };
        if let Ok(mut c) = connected.lock() {
            *c = true;
        }
        let cfg = cfg.clone();
        let tx = tx.clone();
        let h = std::thread::spawn(move || {
            interactive_qos();
            connection(conn, &cfg, tx)
        });
        if once {
            let _ = h.join();
            break;
        }
    }
    let _ = std::fs::remove_file(&socket);
    super::crash::exit(if once {
        "the connection (--once) closed"
    } else {
        "the listener stopped"
    });
    0
}

/// Start-up (DESIGN.md 4.4): which TeX Live (or bundle) the engine will
/// read, and each format made ready, so that the first compile does not pay
/// for building it (about 4 s; a validated cache hit is milliseconds). The
/// format is readied by the engine itself, loading it exactly as a compile
/// will (`system.rs` `find_format`: `FLASHTEX_FORMATS`, else the format
/// cache, `formats::ensure_format`), so both use the same cache entry.
fn prepare(engine: &Path, formats: &[String]) -> Json {
    #[cfg(feature = "kpathsea")]
    let texlive = crate::resolver::discover_texlive()
        .map(|t| js(t.describe()))
        .unwrap_or(Json::Null);
    #[cfg(not(feature = "kpathsea"))]
    let texlive = Json::Null;
    // A bundle's fetches (opening it fetches its index and core, which can
    // take a while on a cold cache) are reported as they go, one line each
    // (`{"bundle_progress": ...}`), so the app can show them; the resident
    // engine's own on-demand fetches later are reported the same way.
    #[cfg(all(feature = "distribution", not(feature = "tex82")))]
    crate::bundle::set_progress(|p| {
        say(&format!(
            "flashtex-host: {}",
            obj([(
                "bundle_progress",
                obj([
                    ("what", js(p.what)),
                    ("name", js(p.name.as_str())),
                    ("done", Json::Int(p.done as i64)),
                    ("total", Json::Int(p.total as i64)),
                ]),
            )])
        ))
    });
    // The process's own resolver, started once: the first compile (run as
    // `pdflatex`) keeps it, so kpathsea's start-up is not paid twice.
    let resolver = crate::system::with_resolver_for("pdflatex", |r| r.describe());
    let bundle = bundle_json(&resolver);
    let dir = std::env::temp_dir().join(format!("flashtex-host-prepare-{}", std::process::id()));
    let mut ready = Vec::new();
    // `pdflatex` last, so that the resolver left running is the one its
    // compiles keep.
    let mut order: Vec<&String> = formats.iter().filter(|f| *f != "pdflatex").collect();
    order.extend(formats.iter().filter(|f| *f == "pdflatex"));
    for f in order {
        let t0 = Instant::now();
        if let Some(r) = ready_in_process(engine, f) {
            let mut kv = vec![
                ("name".to_string(), js(f.as_str())),
                (
                    "status".to_string(),
                    js(if r.is_ok() { "ready" } else { "failed" }),
                ),
                (
                    "ms".to_string(),
                    Json::Num((t0.elapsed().as_secs_f64() * 1e4).round() / 10.0),
                ),
            ];
            if let Err(why) = r {
                kv.push((
                    "error".into(),
                    js(why.chars().take(400).collect::<String>()),
                ));
            }
            ready.push((f, Json::Obj(kv)));
            continue;
        }
        let _ = std::fs::create_dir_all(&dir);
        // Load the format and stop at once (\@@end in LaTeX, \end in plain).
        let st = crate::os::engine_command(engine)
            .arg(format!("-fmt={f}"))
            .args(["-interaction=batchmode", "-jobname=flashtex-host-prepare"])
            .arg(format!("-output-directory={}", dir.display()))
            .arg("\\csname @@end\\endcsname\\end")
            .current_dir(&dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output();
        let (ok, why) = match st {
            Ok(o) if o.status.success() => (true, String::new()),
            Ok(o) => (
                false,
                String::from_utf8_lossy(&o.stderr)
                    .trim()
                    .chars()
                    .take(400)
                    .collect(),
            ),
            Err(e) => (false, e.to_string()),
        };
        let mut kv = vec![
            ("name".to_string(), js(f.as_str())),
            (
                "status".to_string(),
                js(if ok { "ready" } else { "failed" }),
            ),
            (
                "ms".to_string(),
                Json::Num((t0.elapsed().as_secs_f64() * 1e4).round() / 10.0),
            ),
        ];
        if !ok {
            kv.push(("error".into(), js(why)));
        }
        ready.push((f, Json::Obj(kv)));
    }
    let _ = std::fs::remove_dir_all(&dir);
    // In the order asked for.
    let ready: Vec<Json> = formats
        .iter()
        .filter_map(|f| ready.iter().find(|(g, _)| *g == f).map(|(_, j)| j.clone()))
        .collect();
    obj([
        ("texlive", texlive),
        ("resolver", js(resolver)),
        ("bundle", bundle),
        // Why typesetting cannot start (no TeX Live, bundle unusable), with
        // what to do; null when it can.
        (
            "setup",
            crate::resolver::setup_problem().map_or(Json::Null, |p| js(p.message())),
        ),
        ("formats", Json::Arr(ready)),
    ])
}

/// Whether `engine` is this program (the default `--engine`).
fn is_this_program(engine: &Path) -> bool {
    let canon = |p: &Path| std::fs::canonicalize(p).ok();
    std::env::current_exe()
        .ok()
        .and_then(|me| canon(&me))
        .is_some_and(|me| canon(engine).is_some_and(|e| e == me))
}

/// A format made ready in this process, exactly as a compile finds it
/// (system.rs `find_format`: the format cache, `formats::ensure_format`,
/// with the process's resolver), instead of in an engine process started
/// for it: that process's start, kpathsea start-up and format load were
/// about a quarter of a second before every host could listen. A build,
/// when the cache has no valid format, still runs INITEX in its own
/// process (`FormatCache::build`). `None` where a compile would not take
/// the cache path (another `--engine`, `FLASHTEX_FORMATS`, a format on the
/// search path, the cache turned off): there the engine process loads the
/// format as before.
#[cfg(feature = "distribution")]
fn ready_in_process(engine: &Path, f: &str) -> Option<Result<(), String>> {
    if !is_this_program(engine)
        || !crate::formats::cache_enabled()
        || std::env::var("FLASHTEX_FORMATS").is_ok_and(|d| !d.is_empty())
    {
        return None;
    }
    crate::system::with_resolver_for(f, |r| {
        let name = format!("{f}.fmt");
        if r.find(&name, crate::resolver::Format::Fmt).is_some() {
            return None;
        }
        Some(
            crate::formats::ensure_format(f, f, r)
                .map(|_| ())
                .map_err(|e| e.to_string()),
        )
    })
}

#[cfg(not(feature = "distribution"))]
fn ready_in_process(_engine: &Path, _f: &str) -> Option<Result<(), String>> {
    None
}

/// `HELLO.texmf.bundle`: the configured bundle (`bundle::BundleSpec::
/// configured`: the environment, else a `flashtex-bundle.lock`), whether
/// the engine reads it (`active`: no TeX Live, or `FLASHTEX_RESOLVER=
/// bundle`), and where its configuration came from; null when none is
/// configured. `error` when its configuration does not parse.
fn bundle_json(resolver: &str) -> Json {
    #[cfg(all(feature = "distribution", not(feature = "tex82")))]
    {
        return match crate::bundle::BundleSpec::configured() {
            None => Json::Null,
            Some(Err(e)) => obj([("error", js(e))]),
            Some(Ok((spec, origin))) => obj([
                ("digest", js(spec.digest.as_str())),
                ("url", js(spec.url.as_str())),
                ("origin", js(origin.describe())),
                ("offline", Json::Bool(spec.offline)),
                (
                    "active",
                    Json::Bool(resolver == format!("bundle {}", spec.digest)),
                ),
            ]),
        };
    }
    #[allow(unreachable_code)]
    {
        let _ = resolver;
        Json::Null
    }
}

/// A line on stdout for a supervisor, which may have stopped reading.
pub(crate) fn say(line: &str) {
    let mut o = std::io::stdout();
    let _ = writeln!(o, "{line}");
    let _ = o.flush();
}

/// One running export (a child engine process).
struct Running {
    id: i64,
    child: Arc<Mutex<Option<Child>>>,
    cancelled: Arc<Mutex<bool>>,
    thread: std::thread::JoinHandle<()>,
}

impl Running {
    fn cancel(self) {
        *self.cancelled.lock().unwrap() = true;
        if let Some(c) = self.child.lock().unwrap().as_mut() {
            let _ = c.kill();
        }
        let _ = self.thread.join();
    }
}

/// The capabilities this host announces in HELLO.
pub(crate) const CAPABILITIES: &[&str] = &[
    "compile",
    "cancel",
    "diagnostics",
    "font-programs",
    "font-formats",
    "have-fonts",
    "resident",
    "incremental",
    "buffers",
    "edits",
    "viewport",
    "pages-status",
    "export",
    "external-tools",
    // Every PAGE/FORM carries ORIGINS and RULE_GEOMETRY (spec §4.2, §4.4).
    "exact-geometry",
    // COMPILE `halt_on_error` is honoured (`Job::halt`): a client's strict mode.
    "halt-on-error",
    // COMPILE `includeonly` is honoured (`Job::includeonly`): a client's
    // chapter focus through LaTeX's own `\includeonly` (lane FOCUS-CHAPTER).
    "includeonly",
    flashtex_display_list::diag::CAPABILITY,
    flashtex_display_list::PROGRESS_CAPABILITY,
    // HELLO `profile` and the PROFILE message: performance modes (spec §6.9).
    flashtex_display_list::PROFILE_CAPABILITY,
];

fn connection(stream: Stream, cfg: &Config, tx: mpsc::Sender<Req>) {
    let id = CONNECTIONS.fetch_add(1, Ordering::Relaxed);
    flashtex_display_list::widen_socket_buffers(&stream);
    let Ok(wstream) = stream.try_clone() else {
        return;
    };
    let out: Out = Arc::new(Mutex::new(BufWriter::with_capacity(1 << 20, wstream)));
    let mut r = BufReader::new(stream);
    // HELLO
    let diag;
    let progress;
    let profile;
    let minor = match read_frame(&mut r) {
        Ok(Some((k, body))) if k == kind::C_HELLO => {
            let j = std::str::from_utf8(&body)
                .ok()
                .and_then(|t| Json::parse(t).ok())
                .unwrap_or(Json::Null);
            let version = j.get("version").and_then(Json::as_array);
            let major = version.and_then(|a| a.first()).and_then(Json::as_i64);
            if j.str_field("protocol") != Some(PROTOCOL) || major != Some(VERSION_MAJOR as i64) {
                error(
                    &out,
                    None,
                    "version",
                    &format!("this host speaks {PROTOCOL} {VERSION_MAJOR}.{VERSION_MINOR}"),
                );
                return;
            }
            diag = j.get("accept").and_then(Json::as_array).is_some_and(|a| {
                a.iter()
                    .any(|x| x.as_str() == Some(flashtex_display_list::diag::CAPABILITY))
            });
            progress = j.get("accept").and_then(Json::as_array).is_some_and(|a| {
                a.iter()
                    .any(|x| x.as_str() == Some(flashtex_display_list::PROGRESS_CAPABILITY))
            });
            // A mode the host does not know is ignored, as unknown names are
            // (the host's own then stays, and its HELLO says which).
            profile = j
                .str_field("profile")
                .and_then(crate::profile::Mode::parse)
                .map(|m| crate::profile::Profile::new(m, &cfg.pinned));
            version
                .and_then(|a| a.get(1))
                .and_then(Json::as_i64)
                .unwrap_or(0)
        }
        _ => {
            error(&out, None, "protocol", "expected HELLO");
            return;
        }
    };
    let hello = obj([
        ("protocol", js(PROTOCOL)),
        (
            "version",
            Json::Arr(vec![
                Json::Int(VERSION_MAJOR as i64),
                Json::Int(VERSION_MINOR as i64),
            ]),
        ),
        (
            "server",
            js(concat!("flashtex-host ", env!("CARGO_PKG_VERSION"))),
        ),
        ("engine", js(cfg.engine_version.as_str())),
        ("texmf", cfg.texmf.clone()),
        (
            "capabilities",
            Json::Arr(CAPABILITIES.iter().map(|c| js(*c)).collect()),
        ),
        ("profile", profile.as_ref().unwrap_or(&cfg.profile).json()),
    ]);
    if !send_json(&out, kind::HELLO, &hello) {
        return;
    }
    let conn = Arc::new(Conn {
        id,
        out: out.clone(),
        queued: AtomicU64::new(0),
        cancelled: Mutex::new(HashSet::new()),
        minor,
        diag,
        progress,
    });
    if let Some(p) = profile {
        let _ = tx.send(Req::Profile {
            conn: conn.clone(),
            profile: p,
            reply: false,
        });
    }
    let mut export: Option<Running> = None;
    loop {
        let (k, body) = match read_frame(&mut r) {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(e) => {
                error(&out, None, "protocol", &e.to_string());
                break;
            }
        };
        let t0 = Instant::now();
        let j = std::str::from_utf8(&body)
            .ok()
            .and_then(|t| Json::parse(t).ok());
        match k {
            kind::COMPILE => {
                let Some(req) = j else {
                    error(&out, None, "request", "COMPILE is not JSON");
                    continue;
                };
                if let Some(run) = export.take() {
                    run.cancel();
                }
                if req.get("export").and_then(Json::as_bool) == Some(true) {
                    match start_export(cfg, &out, &req, id, conn.diag) {
                        Ok(run) => export = Some(run),
                        Err(e) => error(&out, req.int_field("id"), "request", &e),
                    }
                } else {
                    // A compile already queued or running for this
                    // connection is superseded: `queued` tells it so.
                    conn.queued.fetch_add(1, Ordering::SeqCst);
                    if tx
                        .send(Req::Compile {
                            conn: conn.clone(),
                            req,
                            t0,
                        })
                        .is_err()
                    {
                        error(&out, None, "request", "the engine thread has stopped");
                        break;
                    }
                }
            }
            kind::CANCEL => {
                let cid = j.as_ref().and_then(|j| j.int_field("id"));
                match export.take() {
                    Some(run) if cid.is_none() || cid == Some(run.id) => run.cancel(),
                    other => export = other,
                }
                if let Some(c) = cid {
                    conn.cancelled
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .insert(c);
                }
            }
            kind::C_PROFILE => {
                let m = j.as_ref().and_then(|j| j.str_field("profile"));
                match m.and_then(crate::profile::Mode::parse) {
                    Some(m) => {
                        let _ = tx.send(Req::Profile {
                            conn: conn.clone(),
                            profile: crate::profile::Profile::new(m, &cfg.pinned),
                            reply: true,
                        });
                    }
                    None => error(
                        &out,
                        None,
                        "request",
                        "PROFILE: profile is low-memory, balanced or high-performance",
                    ),
                }
            }
            kind::BYE => break,
            _ => {} // unknown kinds from a later minor version are ignored
        }
    }
    if let Some(run) = export.take() {
        run.cancel();
    }
    let _ = tx.send(Req::Closed(id));
}

/// A `COMPILE`'s job: what pdflatex's command line is made of (§6.3).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Job {
    pub root: PathBuf,
    pub main: String,
    pub format: String,
    pub shell: Option<&'static str>,
    pub out_dir: PathBuf,
    pub jobname: String,
    /// `-halt-on-error` (`"halt_on_error": true`): TeX stops at the first
    /// error, as `pdflatex -halt-on-error` does (the app's strict mode).
    pub halt: bool,
    /// `"includeonly": ["chapters/03", …]` (lane FOCUS-CHAPTER): LaTeX's own
    /// `\includeonly`, given on pdflatex's command line before the main file
    /// (`pdflatex '\AtBeginDocument{\includeonly{chapters/03}}\input
    /// main.tex'`, see `argv`). The names as the document's `\include`s
    /// write them, comma-joined; the other chapters' pages and references
    /// come from their `.aux` files. None: the whole document (the main
    /// file alone).
    pub includeonly: Option<String>,
}

impl Job {
    /// The job a `COMPILE` asks for (`conn`: the connection, for the
    /// default output directory), checked, with the output directory made.
    pub fn parse(req: &Json, conn: u64) -> Result<Job, String> {
        let root = PathBuf::from(req.str_field("root").ok_or("COMPILE needs root")?);
        if !root.is_absolute() || !root.is_dir() {
            return Err(format!(
                "root {} is not an absolute directory",
                root.display()
            ));
        }
        let main = req
            .str_field("main")
            .ok_or("COMPILE needs main")?
            .to_string();
        let mp = Path::new(&main);
        if !inside(mp) {
            return Err("main must be a path inside root".into());
        }
        if !root.join(mp).is_file() {
            return Err(format!("{main} does not exist in root"));
        }
        let format = req.str_field("format").unwrap_or("pdflatex").to_string();
        if format.contains('/') {
            return Err("format is a name, not a path".into());
        }
        let out_dir = match req.str_field("output_dir") {
            Some(d) => PathBuf::from(d),
            None => {
                std::env::temp_dir().join(format!("flashtex-host-{}-{conn}", std::process::id()))
            }
        };
        std::fs::create_dir_all(&out_dir).map_err(|e| format!("output_dir: {e}"))?;
        let jobname = req
            .str_field("jobname")
            .map(str::to_string)
            .unwrap_or_else(|| {
                mp.file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "texput".into())
            });
        if let Some(p) = req.str_field("external_tools") {
            if super::external::Policy::parse(p).is_none() {
                return Err(format!("external_tools {p}: off or auto"));
            }
        }
        let shell = match req.str_field("shell_escape").unwrap_or("default") {
            "default" => None,
            "off" => Some("-no-shell-escape"),
            "restricted" => Some("-shell-restricted"),
            "on" => Some("-shell-escape"),
            other => {
                return Err(format!(
                    "shell_escape {other}: default, off, restricted or on"
                ))
            }
        };
        let halt = match req.get("halt_on_error") {
            None | Some(Json::Null) => false,
            Some(Json::Bool(b)) => *b,
            Some(_) => return Err("halt_on_error is a boolean".into()),
        };
        let includeonly = match req.get("includeonly") {
            None | Some(Json::Null) => None,
            Some(Json::Arr(names)) => Some(includeonly_list(names, &main)?),
            Some(_) => return Err("includeonly is an array of \\include names".into()),
        };
        Ok(Job {
            root,
            main,
            format,
            shell,
            out_dir,
            jobname,
            halt,
            includeonly,
        })
    }

    /// The output directory is the project directory itself: pdflatex then
    /// runs without `-output-directory`, as latexmk and a user run it
    /// (with it, pdfTeX finds an included figure through the output
    /// directory and writes its absolute path as `/PTEX.FileName`, where a
    /// plain run writes `./fig.pdf`: P-T2 differs).
    fn out_is_root(&self) -> bool {
        let canon = |p: &Path| std::fs::canonicalize(p).ok();
        let out = if self.out_dir.is_absolute() {
            self.out_dir.clone()
        } else {
            self.root.join(&self.out_dir)
        };
        canon(&out).is_some() && canon(&out) == canon(&self.root)
    }

    /// pdflatex's command line for the job (without argv[0]).
    pub fn argv(&self) -> Vec<String> {
        let mut argv = vec![
            format!("-fmt={}", self.format),
            "-interaction=nonstopmode".to_string(),
            "-file-line-error".to_string(),
        ];
        if self.halt {
            argv.push("-halt-on-error".to_string());
        }
        if !self.out_is_root() {
            argv.push(format!("-output-directory={}", self.out_dir.display()));
        }
        argv.push(format!("-jobname={}", self.jobname));
        if let Some(f) = self.shell {
            argv.push(f.to_string());
        }
        argv.push(match &self.includeonly {
            None => self.main.clone(),
            // `\includeonly` runs in the `begindocument` hook: after
            // `\document` has read the `.aux` and before it disables the
            // preamble commands, and before any `\include`, so its effect
            // is the preamble's (the same pages, streams and `.aux` as
            // `\includeonly{…}\input main.tex` under pdflatex, measured).
            // In the preamble its name lookup (l3's `\file_full_name:n`,
            // `\pdffilesize`) would put the chapter's content in S₀'s key,
            // and every keystroke in the focused chapter would be a cold
            // run from the format; after the `.aux` point it is a read of
            // the body (of the whole chapter: its size, #1724), and an edit
            // in the chapter restarts at S₀.
            // LaTeX's `\input` without a brace is the primitive (`\@@input`),
            // so the main file is read exactly as a bare first line reads it.
            Some(list) => format!(
                "\\AtBeginDocument{{\\includeonly{{{list}}}}}\\input {}",
                input_name(&self.main)
            ),
        });
        argv
    }
}

/// The `includeonly` names, checked and comma-joined. Each is an
/// `\include` argument as the document writes it: a relative path inside
/// the root with nothing TeX would read as markup on a first line (no
/// comma, brace, backslash, `%`, `#`, `"` or control character), so the
/// list means exactly the names given. `main` must be a name `\input` can
/// take (no `"`; one with a space is quoted).
fn includeonly_list(names: &[Json], main: &str) -> Result<String, String> {
    if names.is_empty() {
        return Err("includeonly names no \\include".into());
    }
    if main.contains('"') || main.chars().any(char::is_control) {
        return Err("includeonly needs a main file name without quotes".into());
    }
    let bad = |c: char| matches!(c, ',' | '{' | '}' | '\\' | '%' | '#' | '"') || c.is_control();
    let mut list = Vec::with_capacity(names.len());
    for n in names {
        let Some(name) = n.as_str() else {
            return Err("includeonly is an array of \\include names".into());
        };
        // Rooted on any platform, whatever `Path` says on this one: on
        // Windows `/abs` is not `is_absolute` (no drive), and `C:x` is
        // relative to drive C's current directory.
        let rooted = name.starts_with(['/', '\\'])
            || matches!(name.as_bytes(), [d, b':', ..] if d.is_ascii_alphabetic());
        if name.is_empty()
            || name.trim() != name
            || name.chars().any(bad)
            || rooted
            || !inside(Path::new(name))
        {
            return Err(format!(
                "includeonly: {name:?} is not an \\include name inside root"
            ));
        }
        list.push(name);
    }
    Ok(list.join(","))
}

/// The main file as `\input` names it on the first line: quoted when it
/// holds a space (web2c's quoted file names), else as given.
fn input_name(main: &str) -> String {
    if main.contains(' ') {
        format!("\"{main}\"")
    } else {
        main.to_string()
    }
}

/// A relative path that stays inside its base (no `..`, not absolute).
pub(crate) fn inside(p: &Path) -> bool {
    !p.is_absolute()
        && !p
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        && p.components().next().is_some()
}

/// The font keys a `COMPILE` says the client holds.
pub(crate) fn have_fonts(req: &Json) -> Vec<String> {
    req.get("have_fonts")
        .and_then(Json::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// The font formats beyond `type1` and `none` whose programs a `COMPILE`
/// says the client takes (`font_formats`, docs/protocol/display-list-v3.md
/// §5.1): `truetype`, `opentype`, `type3`.
pub(crate) fn font_formats(req: &Json) -> Vec<String> {
    req.get("font_formats")
        .and_then(Json::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Start an export: the engine as a child process, its frames relayed from
/// descriptor 3 (lane P3's host). `DONE.pdf` is then the compressed PDF,
/// as pdflatex writes it.
fn start_export(
    cfg: &Config,
    out: &Out,
    req: &Json,
    conn: u64,
    diag: bool,
) -> Result<Running, String> {
    let id = req.int_field("id").ok_or("COMPILE needs an integer id")?;
    let job = Job::parse(req, conn)?;
    let have_fonts = have_fonts(req);
    let font_formats = font_formats(req);
    let argv = job.argv();
    let (root, out_dir, jobname) = (job.root.clone(), job.out_dir.clone(), job.jobname.clone());

    let channel = crate::os::ExportChannel::open().map_err(|e| e.to_string())?;
    let mut cmd = crate::os::engine_command(&cfg.engine);
    cmd.args(&argv)
        .current_dir(&root)
        .env("FLASHTEX_DISPLAY_LIST_HAVE_FONTS", have_fonts.join(","))
        .env("FLASHTEX_DISPLAY_LIST_FONT_FORMATS", font_formats.join(","))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    channel.attach(&mut cmd);
    let t0 = Instant::now();
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("cannot start the engine: {e}"))?;
    let exited = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let pid = child.id();
    let ours = channel.reader(pid, exited.clone());
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    send_json(
        out,
        kind::STARTED,
        &obj([
            ("id", Json::Int(id)),
            ("pid", Json::Int(pid as i64)),
            (
                "argv",
                Json::Arr(argv.iter().map(|a| js(a.as_str())).collect()),
            ),
            ("output_dir", js(out_dir.display().to_string())),
            ("mode", js("export")),
        ]),
    );
    let child = Arc::new(Mutex::new(Some(child)));
    let cancelled = Arc::new(Mutex::new(false));
    let (out2, child2, cancelled2) = (out.clone(), child.clone(), cancelled.clone());
    let root2 = root.clone();
    let thread = std::thread::spawn(move || {
        // Frames from the engine, forwarded as they come.
        let out_r = out2.clone();
        let relay = std::thread::spawn(move || {
            let mut r = BufReader::with_capacity(1 << 20, ours);
            let mut pages = 0u64;
            let mut bytes = 0u64;
            let mut first_page: Option<f64> = None;
            while let Ok(Some((k, body))) = read_frame(&mut r) {
                bytes += body.len() as u64 + 5;
                if k == kind::PAGE {
                    pages += 1;
                    if first_page.is_none() {
                        first_page = Some(t0.elapsed().as_secs_f64() * 1000.0);
                    }
                }
                if !send(&out_r, k, &body) {
                    break;
                }
            }
            (pages, bytes, first_page)
        });
        // The terminal: diagnostics.
        let out_d = out2.clone();
        let diag = std::thread::spawn(move || {
            let mut n = 0u64;
            if let Some(so) = stdout {
                if diag {
                    // diag-v1 from the terminal alone (the export is
                    // another process: no side channel), `exact: false`.
                    use std::io::Read;
                    let mut term = vec![];
                    let _ = BufReader::new(so).read_to_end(&mut term);
                    for (k, (_, mut d)) in super::diag::scan_terminal(&term, &root2)
                        .into_iter()
                        .enumerate()
                    {
                        d.id = id;
                        d.seq = k as i64;
                        send(&out_d, kind::DIAG, &d.encode());
                        n += 1;
                    }
                } else {
                    n = diagnostics(BufReader::new(so), &out_d, id, &root2);
                }
            }
            n
        });
        let err_t = std::thread::spawn(move || {
            let mut s = String::new();
            if let Some(mut e) = stderr {
                use std::io::Read;
                let _ = e.read_to_string(&mut s);
            }
            s
        });
        let status = loop {
            let mut g = child2.lock().unwrap();
            match g.as_mut().map(|c| c.try_wait()) {
                Some(Ok(Some(st))) => {
                    *g = None;
                    break Some(st);
                }
                Some(Ok(None)) => {}
                _ => break None,
            }
            drop(g);
            std::thread::sleep(std::time::Duration::from_millis(2));
        };
        exited.store(true, Ordering::Release);
        let (pages, bytes, first_page) = relay.join().unwrap_or((0, 0, None));
        let ndiag = diag.join().unwrap_or(0);
        let stderr_text = err_t.join().unwrap_or_default();
        let code = status.and_then(|s| s.code());
        let was_cancelled = *cancelled2.lock().unwrap();
        let st = if was_cancelled {
            "cancelled"
        } else if code == Some(0) {
            "ok"
        } else if code.is_some() {
            "error"
        } else {
            "failed"
        };
        let pdf = out_dir.join(format!("{jobname}.pdf"));
        let log = out_dir.join(format!("{jobname}.log"));
        let mut kv = vec![
            ("id".to_string(), Json::Int(id)),
            ("status".to_string(), js(st)),
            (
                "exit_code".to_string(),
                code.map(|c| Json::Int(c as i64)).unwrap_or(Json::Null),
            ),
            ("pages".to_string(), Json::Int(pages as i64)),
            ("bytes".to_string(), Json::Int(bytes as i64)),
            ("diagnostics".to_string(), Json::Int(ndiag as i64)),
            (
                "elapsed_ms".to_string(),
                Json::Num((t0.elapsed().as_secs_f64() * 1e6).round() / 1e3),
            ),
            (
                "first_page_ms".to_string(),
                first_page
                    .map(|v| Json::Num((v * 1e3).round() / 1e3))
                    .unwrap_or(Json::Null),
            ),
            (
                "pdf".to_string(),
                if pdf.is_file() {
                    js(pdf.display().to_string())
                } else {
                    Json::Null
                },
            ),
            (
                "log".to_string(),
                if log.is_file() {
                    js(log.display().to_string())
                } else {
                    Json::Null
                },
            ),
        ];
        if !stderr_text.trim().is_empty() {
            kv.push((
                "stderr".into(),
                js(stderr_text.chars().take(4000).collect::<String>()),
            ));
        }
        send_json(&out2, kind::DONE, &Json::Obj(kv));
    });
    Ok(Running {
        id,
        child,
        cancelled,
        thread,
    })
}

/// Read the engine's terminal output and send what a user must see:
/// errors (`file:line: message`, with `-file-line-error`, or `! message`)
/// and LaTeX/package warnings. Returns how many were sent.
pub(crate) fn diagnostics(r: impl BufRead, out: &Out, id: i64, root: &Path) -> u64 {
    let mut n = 0;
    let mut lines = r
        .split(b'\n')
        .map_while(Result::ok)
        .map(|l| String::from_utf8_lossy(&l).into_owned());
    let send_d = |sev: &str, file: Option<String>, line: Option<i64>, msg: String| {
        let mut kv = vec![
            ("id".to_string(), Json::Int(id)),
            ("severity".to_string(), js(sev)),
            ("message".to_string(), js(msg)),
        ];
        if let Some(f) = file {
            let p = Path::new(&f);
            let abs = if p.is_absolute() {
                p.to_path_buf()
            } else {
                root.join(f.trim_start_matches("./"))
            };
            kv.push(("file".into(), js(abs.display().to_string())));
        }
        if let Some(l) = line {
            kv.push(("line".into(), Json::Int(l)));
        }
        send_json(out, kind::DIAGNOSTIC, &Json::Obj(kv));
    };
    while let Some(l) = lines.next() {
        // file:line: message
        if let Some((file, rest)) = file_line_error(&l) {
            let (line, msg) = rest;
            send_d("error", Some(file), Some(line), msg);
            n += 1;
            continue;
        }
        if let Some(msg) = l.strip_prefix("! ") {
            send_d("error", None, None, msg.to_string());
            n += 1;
            continue;
        }
        let warn = ["LaTeX Warning: ", "Package ", "Class "]
            .iter()
            .any(|p| l.starts_with(p))
            && l.contains("Warning:");
        if warn {
            // A warning's text can continue on following lines up to a blank.
            // TeX breaks terminal lines after max_print_line (texmf.cnf:
            // 79) bytes, wherever that falls: such a break is not a space.
            const MAX_PRINT_LINE: usize = 79;
            let mut msg = l.clone();
            let mut prev = l.len();
            if !msg.ends_with('.') {
                for more in lines.by_ref() {
                    if more.trim().is_empty() {
                        break;
                    }
                    if prev == MAX_PRINT_LINE {
                        msg.push_str(&more);
                    } else {
                        msg.push(' ');
                        msg.push_str(more.trim());
                    }
                    prev = more.len();
                    if more.ends_with('.') {
                        break;
                    }
                }
            }
            let line = input_line(&msg);
            send_d("warning", None, line, msg);
            n += 1;
        }
    }
    n
}

/// `./main.tex:12: Undefined control sequence.` -> (file, (12, message)).
fn file_line_error(l: &str) -> Option<(String, (i64, String))> {
    let mut parts = l.splitn(3, ':');
    let file = parts.next()?;
    let line = parts.next()?;
    let msg = parts.next()?;
    if file.is_empty() || file.contains(' ') || !(file.ends_with(".tex") || file.contains('.')) {
        return None;
    }
    let n: i64 = line.parse().ok()?;
    Some((file.to_string(), (n, msg.trim_start().to_string())))
}

/// "... on input line 12." -> 12.
fn input_line(msg: &str) -> Option<i64> {
    let p = msg.rfind("input line ")?;
    msg[p + 11..]
        .trim_end_matches('.')
        .trim()
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

#[cfg(test)]
mod job_tests {
    use super::*;

    /// A COMPILE on a scratch root, with `halt` as its `halt_on_error`. Built
    /// as values, not JSON text: a Windows path's `\` is no JSON escape.
    fn req(halt: Option<Json>) -> Json {
        let dir = std::env::temp_dir().join(format!("flashtex-job-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.tex"), "x").unwrap();
        let path = |p: &Path| js(p.to_string_lossy().into_owned());
        let mut kv = vec![
            ("id".to_string(), Json::Int(1)),
            ("root".to_string(), path(&dir)),
            ("main".to_string(), js("main.tex")),
            ("output_dir".to_string(), path(&dir.join("out"))),
        ];
        if let Some(h) = halt {
            kv.push(("halt_on_error".to_string(), h));
        }
        Json::Obj(kv)
    }

    /// Strict mode (lane ERROR-RECOVERY): `halt_on_error` is pdflatex's
    /// `-halt-on-error`, a different job; without it the command line is
    /// nonstopmode's, unchanged.
    #[test]
    fn halt_on_error_is_pdflatexs_flag() {
        let plain = Job::parse(&req(None), 1).unwrap();
        assert!(!plain.halt);
        assert!(!plain.argv().iter().any(|a| a == "-halt-on-error"));
        let off = Job::parse(&req(Some(Json::Bool(false))), 1).unwrap();
        assert_eq!(off, plain);
        let halt = Job::parse(&req(Some(Json::Bool(true))), 1).unwrap();
        assert!(halt.halt);
        assert_ne!(
            halt, plain,
            "another job: the resident document is replaced"
        );
        assert_eq!(
            &halt.argv()[..4],
            &[
                "-fmt=pdflatex",
                "-interaction=nonstopmode",
                "-file-line-error",
                "-halt-on-error"
            ]
        );
        assert!(Job::parse(&req(Some(js("yes"))), 1).is_err());
    }

    /// `req(None)` with `includeonly` set to `v`, and `main` replaced.
    fn focused(v: Json, main: Option<&str>) -> Json {
        let Json::Obj(mut kv) = req(None) else {
            unreachable!()
        };
        if let Some(m) = main {
            let dir =
                std::env::temp_dir().join(format!("flashtex-job-test-{}", std::process::id()));
            std::fs::write(dir.join(m), "x").unwrap();
            kv.retain(|(k, _)| k != "main");
            kv.push(("main".to_string(), js(m)));
        }
        kv.push(("includeonly".to_string(), v));
        Json::Obj(kv)
    }

    fn names(n: &[&str]) -> Json {
        Json::Arr(n.iter().map(|s| js(*s)).collect())
    }

    /// Chapter focus (lane FOCUS-CHAPTER): the first line is LaTeX's own
    /// `\includeonly{…}`, run at `\begin{document}`
    /// (`\AtBeginDocument{\includeonly{…}}\input main.tex`); LaTeX's `\input`
    /// without a brace is the primitive, so the main file is read as a bare
    /// first line reads it. Another job; without the field, unchanged.
    #[test]
    fn includeonly_is_latexs_own_first_line() {
        let plain = Job::parse(&req(None), 1).unwrap();
        assert_eq!(plain.includeonly, None);
        assert_eq!(plain.argv().last().unwrap(), "main.tex");
        let null = Job::parse(&focused(Json::Null, None), 1).unwrap();
        assert_eq!(null, plain);
        let one = Job::parse(&focused(names(&["chapters/03"]), None), 1).unwrap();
        assert_ne!(one, plain, "another job: the resident document is replaced");
        assert_eq!(
            one.argv().last().unwrap(),
            "\\AtBeginDocument{\\includeonly{chapters/03}}\\input main.tex"
        );
        // everything before the first line is the plain job's
        let (a, b) = (plain.argv(), one.argv());
        assert_eq!(a[..a.len() - 1], b[..b.len() - 1]);
        let two = Job::parse(&focused(names(&["ch1", "ch 2.tex"]), None), 1).unwrap();
        assert_eq!(
            two.argv().last().unwrap(),
            "\\AtBeginDocument{\\includeonly{ch1,ch 2.tex}}\\input main.tex"
        );
        let spaced = Job::parse(&focused(names(&["ch1"]), Some("my book.tex")), 1).unwrap();
        assert_eq!(
            spaced.argv().last().unwrap(),
            "\\AtBeginDocument{\\includeonly{ch1}}\\input \"my book.tex\""
        );
    }

    /// Only names that mean themselves on a first line, inside the root.
    #[test]
    fn includeonly_refuses_markup_and_escapes() {
        for bad in [
            names(&[]),
            names(&["a,b"]),
            names(&["a}\\input{x"]),
            names(&["a%"]),
            names(&["#1"]),
            names(&["\"q\""]),
            names(&["../outside"]),
            names(&["/abs"]),
            names(&["C:ch1"]),
            names(&["c:/abs"]),
            names(&[" lead"]),
            names(&[""]),
            names(&["line\nbreak"]),
            Json::Arr(vec![Json::Int(3)]),
            js("ch1"),
        ] {
            assert!(
                Job::parse(&focused(bad.clone(), None), 1).is_err(),
                "{bad:?} accepted"
            );
        }
        // A main file named with a quote: refused by the list itself (no file
        // needed), and through `Job::parse` where such a file can exist
        // (Windows refuses `"` in a file name: os error 123).
        assert!(includeonly_list(&[js("ch1")], "q\"uote.tex").is_err());
        assert!(includeonly_list(&[js("ch1")], "quote.tex").is_ok());
        if cfg!(not(windows)) {
            assert!(Job::parse(&focused(names(&["ch1"]), Some("q\"uote.tex")), 1).is_err());
        }
    }

    /// The host says it honours the field (an older one ignores it and
    /// compiles the whole document, so a client asks only when offered).
    #[test]
    fn includeonly_is_a_capability() {
        assert!(CAPABILITIES.contains(&"includeonly"));
    }
}
