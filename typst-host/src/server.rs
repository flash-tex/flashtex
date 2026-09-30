//! The host side of the engine-host socket protocol (display-list-v3 spec
//! §6), for Typst documents, mirroring `flashtex-host`'s `COMPILE`
//! semantics (DESIGN.md §15.2):
//!
//! * `HELLO` first; a client of another major version is refused
//!   (`ERROR version`), anything else first is `ERROR protocol`.
//! * One document (job: `root` + `main`) per connection at a time; a
//!   `COMPILE` for another job replaces it. The app runs one host process
//!   per open Typst document (comemo's cache is process-global).
//! * Each `COMPILE`: apply `buffers`/`edits`, `STARTED`, compile with the
//!   standard `typst::compile`, then `DIAGNOSTIC`s, `FONT`/`SOURCES`/`PAGE`
//!   in page order, `PAGES` (incremental clients), exactly one `DONE`.
//!   `DONE.pdf` is typst-pdf's export of the same document: Typst's oracle.
//! * A `COMPILE` that arrives while an earlier one is still queued
//!   supersedes it: the earlier one only applies its edits and its `DONE`
//!   says `cancelled`. A Typst compile cannot be interrupted (Track A §6), so
//!   a `CANCEL` for the running compile has no effect and `cancel` is not
//!   offered as a capability.
//! * `comemo::evict(10)` after every compile's pages are sent (§15.2).
//!
//! Not in T0 (DESIGN.md §15.10): the seeded 1-pass loop and its idle
//! re-check, PDF-derived f64 positions, `viewport` ordering, packages, the
//! watchdog (the app's job), `lang-v1`.

use std::io::{self, BufReader, BufWriter, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Instant;

use flashtex_display_list::frame::{read_frame, write_frame};
use flashtex_display_list::json::Json;
use flashtex_display_list::{kind, PROTOCOL, VERSION_MAJOR};
use typst::diag::{Severity, SourceDiagnostic, Warned};
use typst::WorldExt;
use typst_layout::PagedDocument;

use crate::convert::{self, ClientCaps, Tables};
use crate::world::{confine, FontOptions, Fonts, HostWorld};
use crate::{v32, TYPST_VERSION};

pub struct Host {
    fonts: Fonts,
}

enum Msg {
    Frame(u8, Vec<u8>),
    Closed,
}

struct Job<'f> {
    root: PathBuf,
    main: String,
    world: HostWorld<'f>,
    tables: Tables,
    /// Page hashes of the last successful compile sent on this connection.
    hashes: Vec<u128>,
    /// Which of those pages the client holds INCOMPLETE.
    incomplete: Vec<bool>,
    compiled: bool,
}

/// A parsed `COMPILE`.
struct Request {
    id: i64,
    root: PathBuf,
    main: String,
    output_dir: Option<PathBuf>,
    jobname: String,
    have_fonts: Vec<String>,
    opentype: bool,
    incremental: bool,
    buffers: Vec<(String, String)>,
    edits: Vec<(String, u64, u64, String)>,
    export: bool,
}

fn err_json(id: Option<i64>, code: &str, message: &str) -> Json {
    let mut kv = Vec::new();
    if let Some(id) = id {
        kv.push(("id".to_string(), Json::Int(id)));
    }
    kv.push(("code".into(), Json::Str(code.into())));
    kv.push(("message".into(), Json::Str(message.into())));
    Json::Obj(kv)
}

struct Conn {
    w: BufWriter<UnixStream>,
}

impl Conn {
    fn send(&mut self, k: u8, body: &[u8]) -> io::Result<()> {
        write_frame(&mut self.w, k, body)
    }
    fn json(&mut self, k: u8, j: &Json) -> io::Result<()> {
        self.send(k, j.to_string().as_bytes())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}

impl Host {
    pub fn new(fonts: &FontOptions) -> Host {
        Host {
            fonts: Fonts::load(fonts),
        }
    }

    pub fn font_count(&self) -> usize {
        self.fonts.len()
    }

    /// Serve connections one after another until the listener fails.
    pub fn serve(&self, listener: UnixListener) -> io::Result<()> {
        for stream in listener.incoming() {
            let stream = stream?;
            if let Err(e) = self.connection(stream) {
                eprintln!("flashtex-typst-host: connection ended: {e}");
            }
        }
        Ok(())
    }

    /// One client connection, until `BYE` or EOF. The socket is shut down
    /// on return, so the client sees EOF (the reader thread holds a clone).
    pub fn connection(&self, stream: UnixStream) -> io::Result<()> {
        let ctl = stream.try_clone()?;
        let r = self.session(stream);
        let _ = ctl.shutdown(std::net::Shutdown::Both);
        r
    }

    fn session(&self, stream: UnixStream) -> io::Result<()> {
        flashtex_display_list::widen_socket_buffers(&stream);
        let (tx, rx) = mpsc::channel();
        let rstream = stream.try_clone()?;
        std::thread::spawn(move || {
            let mut r = BufReader::with_capacity(1 << 20, rstream);
            loop {
                match read_frame(&mut r) {
                    Ok(Some((k, b))) => {
                        if tx.send(Msg::Frame(k, b)).is_err() {
                            return;
                        }
                    }
                    _ => {
                        let _ = tx.send(Msg::Closed);
                        return;
                    }
                }
            }
        });
        let mut c = Conn {
            w: BufWriter::with_capacity(1 << 20, stream),
        };

        // HELLO (spec §6.2).
        let minor = match rx.recv() {
            Ok(Msg::Frame(kind::C_HELLO, body)) => {
                let j = std::str::from_utf8(&body)
                    .ok()
                    .and_then(|t| Json::parse(t).ok());
                let proto = j.as_ref().and_then(|j| j.str_field("protocol"));
                let ver = j
                    .as_ref()
                    .and_then(|j| j.get("version"))
                    .and_then(Json::as_array);
                let major = ver.and_then(|v| v.first()).and_then(Json::as_i64);
                let minor = ver
                    .and_then(|v| v.get(1))
                    .and_then(Json::as_i64)
                    .unwrap_or(0);
                if proto != Some(PROTOCOL) {
                    c.json(
                        kind::ERROR,
                        &err_json(None, "protocol", "HELLO is not display-list-v3"),
                    )?;
                    return c.flush();
                }
                if major != Some(VERSION_MAJOR as i64) {
                    let m = format!("this host speaks display-list-v3 major {VERSION_MAJOR}, the client {major:?}");
                    c.json(kind::ERROR, &err_json(None, "version", &m))?;
                    return c.flush();
                }
                (minor.clamp(0, v32::MINOR as i64)) as u32
            }
            Ok(Msg::Frame(..)) => {
                c.json(
                    kind::ERROR,
                    &err_json(None, "protocol", "the client must send HELLO first"),
                )?;
                return c.flush();
            }
            _ => return Ok(()),
        };
        c.json(kind::HELLO, &hello(minor, self.fonts.len()))?;
        c.flush()?;

        let mut job: Option<Job> = None;
        let mut pending: std::collections::VecDeque<Msg> = Default::default();
        loop {
            let msg = match pending.pop_front() {
                Some(m) => m,
                None => match rx.recv() {
                    Ok(m) => m,
                    Err(_) => return Ok(()),
                },
            };
            match msg {
                Msg::Closed | Msg::Frame(kind::BYE, _) => return c.flush(),
                Msg::Frame(kind::COMPILE, body) => {
                    // Collect what else has arrived: a later COMPILE supersedes this one.
                    while let Ok(m) = rx.try_recv() {
                        pending.push_back(m);
                    }
                    let superseded = pending
                        .iter()
                        .any(|m| matches!(m, Msg::Frame(kind::COMPILE, _)));
                    let req = match parse_request(&body) {
                        Ok(r) => r,
                        Err((id, e)) => {
                            c.json(kind::ERROR, &err_json(id, "request", &e))?;
                            c.flush()?;
                            continue;
                        }
                    };
                    self.compile(&mut c, &mut job, req, minor, superseded)?;
                    c.flush()?;
                }
                // CANCEL: nothing is running between messages; C_HELLO again
                // and unknown kinds are ignored (spec §7).
                Msg::Frame(..) => {}
            }
        }
    }

    fn compile<'f>(
        &'f self,
        c: &mut Conn,
        job: &mut Option<Job<'f>>,
        req: Request,
        minor: u32,
        superseded: bool,
    ) -> io::Result<()> {
        let t0 = Instant::now();
        let id = req.id;
        // The job: a new root or main file starts over.
        let same = job
            .as_ref()
            .is_some_and(|j| j.root == req.root && j.main == req.main);
        if !same {
            match HostWorld::new(&req.root, &req.main, &self.fonts) {
                Ok(world) => {
                    *job = Some(Job {
                        root: req.root.clone(),
                        main: req.main.clone(),
                        world,
                        tables: Tables::new(),
                        hashes: vec![],
                        incomplete: vec![],
                        compiled: false,
                    })
                }
                Err(e) => return c.json(kind::ERROR, &err_json(Some(id), "request", &e)),
            }
        }
        let j = job.as_mut().unwrap();
        if let Err(e) = apply_files(j.world.root(), &req) {
            return c.json(kind::ERROR, &err_json(Some(id), "request", &e));
        }
        let keep = req.incremental && j.compiled;
        if !keep {
            j.tables = Tables::new();
            j.hashes.clear();
            j.incomplete.clear();
        }
        let output_dir = match &req.output_dir {
            Some(d) => d.clone(),
            None => {
                std::env::temp_dir().join(format!("flashtex-typst-host-{}", std::process::id()))
            }
        };
        let mode = if req.export { "export" } else { "resident" };
        c.json(
            kind::STARTED,
            &Json::Obj(vec![
                ("id".into(), Json::Int(id)),
                ("pid".into(), Json::Int(std::process::id() as i64)),
                ("argv".into(), Json::Arr(vec![])),
                (
                    "output_dir".into(),
                    Json::Str(output_dir.to_string_lossy().into_owned()),
                ),
                ("mode".into(), Json::Str(mode.into())),
                ("keep".into(), Json::Bool(keep)),
                ("incremental".into(), Json::Bool(req.incremental)),
                ("engine".into(), Json::Str(format!("Typst {TYPST_VERSION}"))),
            ]),
        )?;
        if superseded {
            return c.json(
                kind::DONE,
                &Json::Obj(vec![
                    ("id".into(), Json::Int(id)),
                    ("status".into(), Json::Str("cancelled".into())),
                    ("exit_code".into(), Json::Int(0)),
                    ("pages".into(), Json::Int(0)),
                ]),
            );
        }

        j.world.reset();
        j.tables.begin_compile();
        let cold = !j.compiled;
        let Warned { output, warnings } = typst::compile::<PagedDocument>(&j.world);
        let compile_ms = ms(t0);

        let mut ndiag = 0;
        let mut errors = 0;
        let mut send_diag = |c: &mut Conn, d: &SourceDiagnostic, w: &HostWorld| -> io::Result<()> {
            ndiag += 1;
            if d.severity == Severity::Error {
                errors += 1;
            }
            c.json(kind::DIAGNOSTIC, &diagnostic(id, d, w))
        };
        for d in &warnings {
            send_diag(c, d, &j.world)?;
        }
        let (status, pages, typeset, first_page_ms, pdf) = match output {
            Err(errs) => {
                for d in &errs {
                    send_diag(c, d, &j.world)?;
                }
                ("error", 0usize, 0usize, None, None)
            }
            Ok(doc) => {
                let caps = ClientCaps {
                    minor,
                    opentype_programs: req.opentype,
                };
                let hashes: Vec<u128> = doc
                    .pages()
                    .iter()
                    .map(|p| typst::utils::hash128(&(&p.frame, &p.fill, &p.bleed, p.number)))
                    .collect();
                let mut first = None;
                let mut typeset = 0;
                let mut incomplete = vec![false; hashes.len()];
                for (i, h) in hashes.iter().enumerate() {
                    if keep && j.hashes.get(i) == Some(h) {
                        incomplete[i] = j.incomplete[i];
                        continue;
                    }
                    let out =
                        convert::page(&j.world, &doc, i, &mut j.tables, caps, &req.have_fonts);
                    for f in &out.fonts {
                        c.send(kind::FONT, f)?;
                    }
                    if let Some(s) = &out.sources {
                        c.json(kind::SOURCES, s)?;
                    }
                    c.send(kind::PAGE, &out.body)?;
                    incomplete[i] = out.flags & 1 != 0;
                    if first.is_none() {
                        c.flush()?;
                        first = Some(ms(t0));
                    }
                    typeset += 1;
                }
                let count = hashes.len();
                j.hashes = hashes;
                j.incomplete = incomplete;
                j.compiled = true;
                if req.incremental {
                    let current = if count == 0 {
                        vec![]
                    } else {
                        vec![Json::Arr(vec![Json::Int(0), Json::Int(count as i64 - 1)])]
                    };
                    c.json(
                        kind::PAGES,
                        &Json::Obj(vec![
                            ("id".into(), Json::Int(id)),
                            ("count".into(), Json::Int(count as i64)),
                            ("complete".into(), Json::Bool(true)),
                            ("current".into(), Json::Arr(current)),
                            ("stale".into(), Json::Arr(vec![])),
                        ]),
                    )?;
                }
                // DONE.pdf: typst-pdf's export of this compile (the oracle,
                // DESIGN.md §15), deterministic: no creation timestamp. A whole
                // export costs about 80 ms at 300 pages (Track C §5), so it is
                // written only when asked for (`export`) or when the client
                // may need it: a page it holds is INCOMPLETE, or it cannot
                // draw the pages at all (3.1, or no `opentype` programs).
                let need_pdf =
                    req.export || minor < 2 || !req.opentype || j.incomplete.iter().any(|&b| b);
                let exported = if need_pdf {
                    Some(typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()))
                } else {
                    None
                };
                let pdf = match exported {
                    None => None,
                    Some(Ok(bytes)) => {
                        let path = output_dir.join(format!("{}.pdf", req.jobname));
                        match std::fs::create_dir_all(&output_dir)
                            .and_then(|_| std::fs::write(&path, bytes))
                        {
                            Ok(()) => Some(path),
                            Err(e) => {
                                ndiag += 1;
                                errors += 1;
                                let m = format!("could not write {}: {e}", path.display());
                                c.json(kind::DIAGNOSTIC, &simple_diag(id, "error", &m))?;
                                None
                            }
                        }
                    }
                    Some(Err(errs)) => {
                        for d in &errs {
                            ndiag += 1;
                            errors += 1;
                            c.json(kind::DIAGNOSTIC, &diagnostic(id, d, &j.world))?;
                        }
                        None
                    }
                };
                (
                    if errors > 0 { "error" } else { "ok" },
                    count,
                    typeset,
                    first,
                    pdf,
                )
            }
        };
        let mut done = vec![
            ("id".to_string(), Json::Int(id)),
            ("status".into(), Json::Str(status.into())),
            (
                "exit_code".into(),
                Json::Int(if status == "ok" { 0 } else { 1 }),
            ),
            ("pages".into(), Json::Int(pages as i64)),
            ("diagnostics".into(), Json::Int(ndiag)),
            ("elapsed_ms".into(), Json::Num(ms(t0))),
            ("compile_ms".into(), Json::Num(compile_ms)),
            (
                "first_page_ms".into(),
                first_page_ms.map(Json::Num).unwrap_or(Json::Null),
            ),
            (
                "mode".into(),
                Json::Str(if cold { "cold" } else { "incremental" }.into()),
            ),
            ("typeset_pages".into(), Json::Int(typeset as i64)),
            ("keep".into(), Json::Bool(keep)),
            ("engine".into(), Json::Str("typst".into())),
        ];
        if let Some(p) = pdf {
            done.push(("pdf".into(), Json::Str(p.to_string_lossy().into_owned())));
        }
        c.json(kind::DONE, &Json::Obj(done))?;
        c.flush()?;
        // Mandatory eviction once the pages are out (DESIGN.md §15.2):
        // without it memory grows ~70 MB per keystroke at 300 pages.
        comemo::evict(10);
        Ok(())
    }
}

fn hello(minor: u32, fonts: usize) -> Json {
    let mut caps = vec![
        "compile",
        "diagnostics",
        "font-programs",
        "font-formats",
        "have-fonts",
        "resident",
        "incremental",
        "buffers",
        "edits",
        "pages-status",
        "export",
    ];
    if minor >= 2 {
        caps.extend(["opentype-glyphs", "origins-f64", "page-meta"]);
    }
    Json::Obj(vec![
        ("protocol".into(), Json::Str(PROTOCOL.into())),
        (
            "version".into(),
            Json::Arr(vec![
                Json::Int(VERSION_MAJOR as i64),
                Json::Int(minor as i64),
            ]),
        ),
        (
            "server".into(),
            Json::Str(concat!("flashtex-typst-host ", env!("CARGO_PKG_VERSION")).into()),
        ),
        ("engine".into(), Json::Str(format!("Typst {TYPST_VERSION}"))),
        (
            "capabilities".into(),
            Json::Arr(caps.into_iter().map(|s| Json::Str(s.into())).collect()),
        ),
        (
            "typst".into(),
            Json::Obj(vec![
                ("version".into(), Json::Str(TYPST_VERSION.into())),
                ("fonts".into(), Json::Int(fonts as i64)),
                ("packages".into(), Json::Str("unavailable".into())),
            ]),
        ),
    ])
}

fn simple_diag(id: i64, severity: &str, message: &str) -> Json {
    Json::Obj(vec![
        ("id".into(), Json::Int(id)),
        ("severity".into(), Json::Str(severity.into())),
        ("message".into(), Json::Str(message.into())),
    ])
}

/// `DIAGNOSTIC` (spec §6.4) with the 3.2-draft additions `column` (0-based
/// byte column) and `hints`.
fn diagnostic(id: i64, d: &SourceDiagnostic, w: &HostWorld) -> Json {
    let sev = match d.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };
    let mut kv = vec![
        ("id".to_string(), Json::Int(id)),
        ("severity".into(), Json::Str(sev.into())),
        ("message".into(), Json::Str(d.message.to_string())),
    ];
    if let Some(fid) = d.span.id() {
        if let (Some(path), Some(range)) = (w.path_of(fid), w.range(d.span)) {
            kv.push((
                "file".into(),
                Json::Str(path.to_string_lossy().into_owned()),
            ));
            if let Ok(src) = typst::World::source(w, fid) {
                if let Some((line, col)) = src
                    .lines()
                    .byte_to_line(range.start)
                    .and_then(|l| Some((l, range.start - src.lines().line_to_byte(l)?)))
                {
                    kv.push(("line".into(), Json::Int(line as i64 + 1)));
                    kv.push(("column".into(), Json::Int(col as i64)));
                }
            }
        }
    }
    if !d.hints.is_empty() {
        kv.push((
            "hints".into(),
            Json::Arr(d.hints.iter().map(|h| Json::Str(h.v.to_string())).collect()),
        ));
    }
    Json::Obj(kv)
}

fn ms(t: Instant) -> f64 {
    (t.elapsed().as_secs_f64() * 1e6).round() / 1e3
}

/// A relative, `/`-separated path inside the project (spec §1).
fn relative(p: &str) -> Result<&str, String> {
    if p.is_empty() || p.starts_with('/') || p.contains('\\') || p.split('/').any(|c| c == "..") {
        return Err(format!("{p:?} is not a relative path inside root"));
    }
    Ok(p)
}

fn parse_request(body: &[u8]) -> Result<Request, (Option<i64>, String)> {
    let j = std::str::from_utf8(body)
        .map_err(|e| e.to_string())
        .and_then(Json::parse)
        .map_err(|e| (None, format!("COMPILE is not JSON: {e}")))?;
    let id = j
        .int_field("id")
        .ok_or((None, "COMPILE without an integer id".to_string()))?;
    let bad = |m: String| (Some(id), m);
    let root = j
        .str_field("root")
        .ok_or_else(|| bad("COMPILE without root".into()))?;
    let root = PathBuf::from(root);
    if !root.is_absolute() || !root.is_dir() {
        return Err(bad(format!(
            "root {} is not an absolute directory",
            root.display()
        )));
    }
    let main = j
        .str_field("main")
        .ok_or_else(|| bad("COMPILE without main".into()))?;
    let main = relative(main).map_err(bad)?.to_string();
    let output_dir = j.str_field("output_dir").map(PathBuf::from);
    if let Some(d) = &output_dir {
        if !d.is_absolute() {
            return Err(bad(format!("output_dir {} is not absolute", d.display())));
        }
    }
    let jobname = match j.str_field("jobname") {
        Some(s) if !s.is_empty() && !s.contains('/') => s.to_string(),
        Some(s) => return Err(bad(format!("jobname {s:?} is not a file name"))),
        None => Path::new(&main)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or("main".into()),
    };
    let strs = |k: &str| -> Vec<String> {
        j.get(k)
            .and_then(Json::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut buffers = Vec::new();
    for b in j.get("buffers").and_then(Json::as_array).unwrap_or(&[]) {
        let (Some(p), Some(t)) = (b.str_field("path"), b.str_field("text")) else {
            return Err(bad("a buffer needs path and text".into()));
        };
        buffers.push((relative(p).map_err(bad)?.to_string(), t.to_string()));
    }
    let mut edits = Vec::new();
    for e in j.get("edits").and_then(Json::as_array).unwrap_or(&[]) {
        let p = e
            .str_field("path")
            .ok_or_else(|| bad("an edit needs path".into()))?;
        let off = e
            .int_field("offset")
            .filter(|v| *v >= 0)
            .ok_or_else(|| bad("an edit needs offset".into()))?;
        let del = e.int_field("delete").unwrap_or(0).max(0);
        let ins = e.str_field("insert").unwrap_or("").to_string();
        edits.push((
            relative(p).map_err(bad)?.to_string(),
            off as u64,
            del as u64,
            ins,
        ));
    }
    Ok(Request {
        id,
        root,
        main,
        output_dir,
        jobname,
        have_fonts: strs("have_fonts"),
        opentype: strs("font_formats").iter().any(|f| f == "opentype"),
        incremental: j
            .get("incremental")
            .and_then(Json::as_bool)
            .unwrap_or(false),
        buffers,
        edits,
        export: j.get("export").and_then(Json::as_bool).unwrap_or(false),
    })
}

/// `buffers` and `edits` (spec §6.3): written to their files, as saving
/// would, confined to the project root.
fn apply_files(root: &Path, req: &Request) -> Result<(), String> {
    for (p, text) in &req.buffers {
        let path = confine(root, Path::new(p))?;
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    for (p, off, del, ins) in &req.edits {
        let path = confine(root, Path::new(p))?;
        let mut data = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let (off, del) = (*off as usize, *del as usize);
        if off > data.len() || off + del > data.len() {
            return Err(format!(
                "edit {off}+{del} is outside {} ({} bytes)",
                p,
                data.len()
            ));
        }
        data.splice(off..off + del, ins.bytes());
        std::fs::write(&path, data).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

/// Bind the socket (mode 0600, spec §6.1) after removing a stale one.
pub fn bind(path: &Path) -> io::Result<UnixListener> {
    use std::os::unix::fs::PermissionsExt;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    let l = UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(l)
}
