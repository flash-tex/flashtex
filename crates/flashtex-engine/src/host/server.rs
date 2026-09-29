//! `flashtex-host`: the engine host (DESIGN.md §3). It listens on a Unix
//! socket and speaks `display-list-v3` (docs/protocol/display-list-v3.md §6):
//! a client connects, says `HELLO`, asks to `COMPILE` a project, and receives
//! the display list of every page as the engine ships it out, the fonts and
//! images those pages use, the source spans, diagnostics, and `DONE`.
//!
//! Each compile runs the engine (`flashtex-initex`, next to this binary
//! unless `--engine` says otherwise) as a child process with pdfTeX's command
//! line, in the project directory, with `FLASHTEX_DISPLAY_LIST=fd:3`: the
//! engine writes its frames to descriptor 3, a socket pair whose other end
//! this host reads and forwards to the client frame by frame. The child's
//! terminal output becomes `DIAGNOSTIC` messages. `CANCEL` (or a new
//! `COMPILE`) kills the child.
//!
//! GPL-2.0-or-later like the engine. The app never links this program; it
//! runs it and talks to its socket.
//!
//!     flashtex-host --socket /tmp/flashtex.sock [--engine PATH] [--format NAME]... [--once]
//!
//! At start-up it reports which TeX Live (or bundle) the engine reads and
//! makes each `--format` ready (default `pdflatex`), building it into the
//! format cache if needed (see [`prepare`]); HELLO carries both.

use flashtex_display_list::frame::{read_frame, write_frame};
use flashtex_display_list::json::{obj, s as js, Json};
use flashtex_display_list::{kind, PROTOCOL, VERSION_MAJOR, VERSION_MINOR};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

extern "C" {
    fn dup2(oldfd: i32, newfd: i32) -> i32;
}

type Out = Arc<Mutex<BufWriter<UnixStream>>>;

fn send(out: &Out, k: u8, body: &[u8]) -> bool {
    let mut w = out.lock().unwrap();
    write_frame(&mut *w, k, body).is_ok() && w.flush().is_ok()
}

fn send_json(out: &Out, k: u8, j: &Json) -> bool {
    send(out, k, j.to_string().as_bytes())
}

fn error(out: &Out, id: Option<i64>, code: &str, message: &str) {
    let mut kv = vec![
        ("code".to_string(), js(code)),
        ("message".to_string(), js(message)),
    ];
    if let Some(i) = id {
        kv.insert(0, ("id".into(), Json::Int(i)));
    }
    send_json(out, kind::ERROR, &Json::Obj(kv));
}

struct Config {
    engine: PathBuf,
    engine_version: String,
    /// What the engine reads (TeX Live or the bundle) and the formats made
    /// ready at start-up, for HELLO.
    texmf: Json,
}

static CONNECTIONS: AtomicU64 = AtomicU64::new(0);

/// `flashtex-host --socket PATH ...`: `args` is the whole command line;
/// returns the exit status.
pub fn main(args: Vec<String>) -> i32 {
    let mut socket = None;
    let mut engine = None;
    let mut once = false;
    let mut formats: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--socket" => {
                socket = args.get(i + 1).cloned();
                i += 1;
            }
            "--engine" => {
                engine = args.get(i + 1).cloned();
                i += 1;
            }
            "--once" => once = true,
            "--format" => {
                if let Some(f) = args.get(i + 1) {
                    formats.push(f.clone());
                }
                i += 1;
            }
            "--help" | "-h" => {
                println!("usage: flashtex-host --socket PATH [--engine PATH] [--format NAME]... [--once]");
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
    let engine = engine.map(PathBuf::from).unwrap_or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("flashtex-initex")))
            .unwrap_or_else(|| PathBuf::from("flashtex-initex"))
    });
    let engine_version = Command::new(&engine)
        .arg("-version")
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .map(str::to_string)
        })
        .unwrap_or_default();
    if formats.is_empty() {
        formats.push("pdflatex".into());
    }
    let texmf = prepare(&engine, &formats);
    say(&format!("flashtex-host: {texmf}"));
    let cfg = Arc::new(Config {
        engine,
        engine_version,
        texmf,
    });
    let _ = std::fs::remove_file(&socket);
    let listener = match UnixListener::bind(&socket) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("flashtex-host: {socket}: {e}");
            return 1;
        }
    };
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600));
    }
    // Ready: a supervisor may wait for this line.
    say(&format!("flashtex-host: listening on {socket}"));
    for conn in listener.incoming() {
        let Ok(conn) = conn else { continue };
        let cfg = cfg.clone();
        let h = std::thread::spawn(move || connection(conn, &cfg));
        if once {
            let _ = h.join();
            break;
        }
    }
    let _ = std::fs::remove_file(&socket);
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
    let resolver = crate::resolver::default_resolver(
        "pdflatex",
        crate::system::ENGINE_NAME,
    )
    .describe();
    let dir = std::env::temp_dir().join(format!("flashtex-host-prepare-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let mut ready = Vec::new();
    for f in formats {
        let t0 = Instant::now();
        // Load the format and stop at once (\@@end in LaTeX, \end in plain).
        let st = Command::new(engine)
            .arg0("pdftex")
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
        ready.push(Json::Obj(kv));
    }
    let _ = std::fs::remove_dir_all(&dir);
    obj([
        ("texlive", texlive),
        ("resolver", js(resolver)),
        ("formats", Json::Arr(ready)),
    ])
}

/// A line on stdout for a supervisor, which may have stopped reading.
fn say(line: &str) {
    let mut o = std::io::stdout();
    let _ = writeln!(o, "{line}");
    let _ = o.flush();
}

/// One running compile.
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

fn connection(stream: UnixStream, cfg: &Config) {
    let conn = CONNECTIONS.fetch_add(1, Ordering::Relaxed);
    flashtex_display_list::widen_socket_buffers(&stream);
    let Ok(wstream) = stream.try_clone() else {
        return;
    };
    let out: Out = Arc::new(Mutex::new(BufWriter::with_capacity(1 << 20, wstream)));
    let mut r = BufReader::new(stream);
    // HELLO
    match read_frame(&mut r) {
        Ok(Some((k, body))) if k == kind::C_HELLO => {
            let j = std::str::from_utf8(&body)
                .ok()
                .and_then(|t| Json::parse(t).ok())
                .unwrap_or(Json::Null);
            let major = j
                .get("version")
                .and_then(Json::as_array)
                .and_then(|a| a.first())
                .and_then(Json::as_i64);
            if j.str_field("protocol") != Some(PROTOCOL) || major != Some(VERSION_MAJOR as i64) {
                error(
                    &out,
                    None,
                    "version",
                    &format!("this host speaks {PROTOCOL} {VERSION_MAJOR}.{VERSION_MINOR}"),
                );
                return;
            }
        }
        _ => {
            error(&out, None, "protocol", "expected HELLO");
            return;
        }
    }
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
            Json::Arr(
                [
                    "compile",
                    "cancel",
                    "diagnostics",
                    "font-programs",
                    "have-fonts",
                ]
                .iter()
                .map(|c| js(*c))
                .collect(),
            ),
        ),
    ]);
    if !send_json(&out, kind::HELLO, &hello) {
        return;
    }
    let mut running: Option<Running> = None;
    loop {
        let (k, body) = match read_frame(&mut r) {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(e) => {
                error(&out, None, "protocol", &e.to_string());
                break;
            }
        };
        let j = std::str::from_utf8(&body)
            .ok()
            .and_then(|t| Json::parse(t).ok());
        match k {
            kind::COMPILE => {
                let Some(req) = j else {
                    error(&out, None, "request", "COMPILE is not JSON");
                    continue;
                };
                if let Some(run) = running.take() {
                    run.cancel();
                }
                match start(cfg, &out, &req, conn) {
                    Ok(run) => running = Some(run),
                    Err(e) => error(&out, req.int_field("id"), "request", &e),
                }
            }
            kind::CANCEL => {
                let id = j.as_ref().and_then(|j| j.int_field("id"));
                match running.take() {
                    Some(run) if id.is_none() || id == Some(run.id) => run.cancel(),
                    other => running = other,
                }
            }
            kind::BYE => break,
            _ => {} // unknown kinds from a later minor version are ignored
        }
    }
    if let Some(run) = running.take() {
        run.cancel();
    }
}

fn start(cfg: &Config, out: &Out, req: &Json, conn: u64) -> Result<Running, String> {
    let id = req.int_field("id").ok_or("COMPILE needs an integer id")?;
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
    if mp.is_absolute()
        || mp
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
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
        None => std::env::temp_dir().join(format!("flashtex-host-{}-{conn}", std::process::id())),
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
    let have_fonts: Vec<String> = req
        .get("have_fonts")
        .and_then(Json::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let mut argv = vec![
        format!("-fmt={format}"),
        "-interaction=nonstopmode".to_string(),
        "-file-line-error".to_string(),
        format!("-output-directory={}", out_dir.display()),
        format!("-jobname={jobname}"),
    ];
    if let Some(f) = shell {
        argv.push(f.to_string());
    }
    argv.push(main.clone());

    let (ours, theirs) = UnixStream::pair().map_err(|e| e.to_string())?;
    flashtex_display_list::widen_socket_buffers(&ours);
    flashtex_display_list::widen_socket_buffers(&theirs);
    let fd = {
        use std::os::fd::AsRawFd;
        theirs.as_raw_fd()
    };
    let mut cmd = Command::new(&cfg.engine);
    cmd.arg0("pdftex")
        .args(&argv)
        .current_dir(&root)
        .env("FLASHTEX_DISPLAY_LIST", "fd:3")
        .env("FLASHTEX_DISPLAY_LIST_HAVE_FONTS", have_fonts.join(","))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Descriptor 3 in the child is our socket pair's other end.
    unsafe {
        cmd.pre_exec(move || {
            if dup2(fd, 3) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let t0 = Instant::now();
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("cannot start the engine: {e}"))?;
    drop(theirs);
    let pid = child.id();
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
                n = diagnostics(BufReader::new(so), &out_d, id, &root2);
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
fn diagnostics(r: impl BufRead, out: &Out, id: i64, root: &Path) -> u64 {
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
            let mut msg = l.clone();
            let line = input_line(&msg);
            if !msg.ends_with('.') {
                for more in lines.by_ref() {
                    if more.trim().is_empty() {
                        break;
                    }
                    msg.push(' ');
                    msg.push_str(more.trim());
                    if more.ends_with('.') {
                        break;
                    }
                }
            }
            let line = line.or_else(|| input_line(&msg));
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
