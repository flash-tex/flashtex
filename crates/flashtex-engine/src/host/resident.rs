//! The engine thread of `flashtex-host --socket` (`super::server`): one
//! resident, incremental engine per document (DESIGN.md §5.1–§5.4) serving
//! `display-list-v3` to the connected clients.
//!
//! * **One document at a time.** The engine's state is per thread and the
//!   engine keeps one job's state resident ([`crate::incr::Session`]): a
//!   `COMPILE` for another job (root, main file, format, output directory,
//!   job name, `\write18`) replaces it. The app runs one host per open
//!   document.
//! * **Edits.** A `COMPILE` may carry the editor's `buffers` (whole files)
//!   and `edits` (byte splices); the host writes them to their files under
//!   `root`, as saving would, and the engine finds what changed
//!   (`Session::compile`: restart before the change, converge after it).
//! * **Pages in order, edited page first.** The display-list writer
//!   ([`crate::displaylist`]) hands every page the engine ships out to the
//!   sink here ([`HostSink`]), which caches it by page index and sends it to
//!   the client at once. Pages the compile did not re-typeset (before the
//!   restart point, after convergence) are sent from the cache in page
//!   order, unless the client already holds them (`"incremental": true`:
//!   its pages persist across compiles). A client's resources are
//!   [`crate::displaylist::Peer`]'s: each font, image and span is sent
//!   before the first page that needs it, once per connection.
//! * **Stale pages** (`PAGES`, protocol 3.1, incremental clients): after
//!   the first re-typeset page, pages up to it are current and the client's
//!   later pages stale; after a `viewport` stop (L4) likewise; at the end,
//!   all `count` pages are current.
//! * **Superseded and cancelled compiles (preemption).** The engine runs one
//!   compile at a time. When a newer `COMPILE` from the same connection is
//!   waiting (typing), or the client cancelled, the running compile stops at
//!   its next page or segment checkpoint (`incr::Session::set_preempt`) --
//!   within a paragraph or two of engine time, in its first pass or in the
//!   `.aux` passes behind it -- and ends with `DONE` `cancelled`. The next
//!   compile keeps what the stopped run typeset when the new edit is behind
//!   it, and otherwise goes back to the complete run it was replacing
//!   (`incr::Session::compile`); either way its edited page comes first. A
//!   compile already superseded when it is taken only applies its edits.

use super::server::{self, Config, Conn, Job, Out, Req};
use crate::displaylist::{self, Emitted, Peer, Sink};
use crate::incr;
use flashtex_display_list::json::{obj, s as js, Json};
use flashtex_display_list::kind;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc};
use std::time::Instant;

/// A page (or form) the writer produced, as the cache keeps it.
struct Cached {
    e: Emitted,
    /// Bumped every time the index is produced again.
    version: u64,
}

/// What one client holds.
struct PeerState {
    peer: Peer,
    /// Page index -> the cached version the client has.
    held: HashMap<u32, u64>,
    /// Form id -> the content hash of the form the client has.
    forms: HashMap<u32, [u8; 32]>,
    /// The document generation `held` refers to.
    doc_gen: u64,
}

impl PeerState {
    fn new() -> PeerState {
        PeerState {
            peer: Peer::default(),
            held: HashMap::new(),
            forms: HashMap::new(),
            doc_gen: 0,
        }
    }

    fn reset(&mut self) {
        self.peer.reset();
        self.held.clear();
        self.forms.clear();
    }
}

/// The compile in progress: whom its pages go to.
struct Target {
    ps: PeerState,
    conn: Arc<Conn>,
    id: i64,
    t0: Instant,
    incremental: bool,
    /// Pages the document had before this compile.
    old_count: usize,
    /// Pages below this index have been delivered, in order, this compile.
    next: u32,
    first_index: Option<u32>,
    first_page_ms: Option<f64>,
    emitted: usize,
    bytes: u64,
    /// The client is gone.
    broken: bool,
    /// Output stopped (superseded or cancelled) at some point.
    went_quiet: bool,
    /// Stage timings (DONE's `stages`): the engine thread's CPU time and
    /// display-list time at the start of the compile, the time spent
    /// writing frames to the socket, and the first page's figures.
    cpu0: f64,
    emit0: u64,
    send_ns: u64,
    first_cpu_ms: Option<f64>,
    first_emit_ms: Option<f64>,
    first_send_ms: Option<f64>,
}

impl Target {
    /// Whether this compile's output is still wanted.
    fn quiet(&mut self) -> bool {
        let q = self.broken
            || self.conn.queued.load(Ordering::SeqCst) > 0
            || self.conn.is_cancelled(self.id);
        self.went_quiet |= q;
        q
    }

    fn out(&self) -> &Out {
        &self.conn.out
    }

    /// Send `e` (with what the client lacks before it).
    fn send(&mut self, e: &Emitted) -> bool {
        let t_send = Instant::now();
        let out = self.conn.out.clone();
        let mut bytes = 0u64;
        let ok = self.ps.peer.send(e, &mut |k, b| {
            bytes += b.len() as u64 + 5;
            server::send(&out, k, b)
        });
        self.bytes += bytes;
        self.send_ns += t_send.elapsed().as_nanos() as u64;
        if !ok {
            self.broken = true;
        }
        ok
    }

    fn pages_status(&mut self, count: usize, complete: bool) {
        if !self.incremental || self.quiet() {
            return;
        }
        let range = |a: usize, b: usize| Json::Arr(vec![Json::Int(a as i64), Json::Int(b as i64)]);
        let cur = self.next as usize;
        let mut current = vec![];
        let mut stale = vec![];
        if complete {
            if count > 0 {
                current.push(range(0, count - 1));
            }
        } else {
            if cur > 0 {
                current.push(range(0, cur - 1));
            }
            if count > cur {
                stale.push(range(cur, count - 1));
            }
        }
        let j = obj([
            ("id", Json::Int(self.id)),
            ("count", Json::Int(count as i64)),
            ("complete", Json::Bool(complete)),
            ("current", Json::Arr(current)),
            ("stale", Json::Arr(stale)),
        ]);
        if !server::send_json(self.out(), kind::PAGES, &j) {
            self.broken = true;
        }
    }
}

/// The page cache and the compile in progress, shared by the engine thread
/// and the display-list sink (both on the engine thread).
struct Live {
    pages: Vec<Option<Cached>>,
    forms: HashMap<u32, Cached>,
    next_version: u64,
    target: Option<Target>,
}

impl Live {
    fn new() -> Live {
        Live {
            pages: Vec::new(),
            forms: HashMap::new(),
            next_version: 1,
            target: None,
        }
    }

    fn clear(&mut self) {
        self.pages.clear();
        self.forms.clear();
    }

    /// Deliver page `j` from the cache to `t` if the client lacks it, with
    /// the forms it draws (`forms`: false for a page just shipped, whose
    /// forms the engine writes next).
    fn deliver(&self, t: &mut Target, j: u32, forms: bool) {
        let Some(Some(c)) = self.pages.get(j as usize) else {
            return;
        };
        if t.ps.held.get(&j) != Some(&c.version) {
            if !t.send(&c.e) {
                return;
            }
            t.ps.held.insert(j, c.version);
        }
        if forms {
            for n in &c.e.forms {
                if let Some(f) = self.forms.get(n) {
                    if t.ps.forms.get(n) != Some(&f.e.hash) {
                        if !t.send(&f.e) {
                            return;
                        }
                        t.ps.forms.insert(*n, f.e.hash);
                    }
                }
            }
        }
    }

    /// Deliver the cached pages from `t.next` up to (not including) `end`.
    fn catch_up(&self, t: &mut Target, end: u32) {
        while t.next < end && !t.quiet() {
            self.deliver(t, t.next, true);
            t.next += 1;
        }
    }

    fn emit(&mut self, e: Emitted) {
        let version = self.next_version;
        self.next_version += 1;
        if e.form {
            let mut t = self.target.take();
            if let Some(t) = t.as_mut() {
                if !t.quiet() && t.send(&e) {
                    t.ps.forms.insert(e.index, e.hash);
                }
            }
            self.forms.insert(e.index, Cached { e, version });
            self.target = t;
            return;
        }
        let i = e.index as usize;
        if self.pages.len() <= i {
            self.pages.resize_with(i + 1, || None);
        }
        self.pages[i] = Some(Cached { e, version });
        let Some(mut t) = self.target.take() else {
            return;
        };
        t.emitted += 1;
        self.catch_up(&mut t, i as u32);
        if !t.quiet() && t.next == i as u32 {
            self.deliver(&mut t, i as u32, false);
            t.next = i as u32 + 1;
            if t.first_index.is_none() {
                t.first_index = Some(i as u32);
                t.first_page_ms = Some(t.t0.elapsed().as_secs_f64() * 1e3);
                t.first_cpu_ms = Some((incr::thread_cpu_s() - t.cpu0) * 1e3);
                t.first_emit_ms = Some((displaylist::emit_ns() - t.emit0) as f64 * 1e-6);
                t.first_send_ms = Some(t.send_ns as f64 * 1e-6);
                let count = t.old_count.max(i + 1);
                t.pages_status(count, false);
            }
        }
        self.target = Some(t);
    }
}

/// The display-list writer's sink in the host.
struct HostSink(Rc<RefCell<Live>>);

impl Sink for HostSink {
    fn emit(&mut self, e: Emitted) {
        self.0.borrow_mut().emit(e);
    }
}

/// The resident document.
struct Doc {
    job: Job,
    session: incr::Session,
    /// Generation: peers holding pages of an older one start afresh.
    gen: u64,
    compiles: u64,
    /// The user's files as the last compile read them (to move source
    /// spans with their lines when they are edited).
    texts: HashMap<String, Arc<Vec<u8>>>,
}

pub(crate) struct Engine {
    cfg: Arc<Config>,
    doc: Option<Doc>,
    peers: HashMap<u64, PeerState>,
    live: Rc<RefCell<Live>>,
    gens: u64,
    /// The files the host last wrote (`apply_changes`), with their stat
    /// signature then: the next edit splices into these bytes instead of
    /// reading the file again (a 1,000-page source is 4 MB) while the file
    /// is as the host left it.
    written: Written,
}

type Written = HashMap<PathBuf, (crate::system::StatSig, Arc<Vec<u8>>)>;

impl Engine {
    pub fn new(cfg: Arc<Config>) -> Engine {
        Engine {
            cfg,
            doc: None,
            peers: HashMap::new(),
            live: Rc::new(RefCell::new(Live::new())),
            gens: 0,
            written: HashMap::new(),
        }
    }

    pub fn run(mut self, rx: mpsc::Receiver<Req>) {
        // `--keep-warm`: after a compile, poll (a busy core) until then.
        let mut hot_until: Option<Instant> = None;
        // The heap's free pages go back to the system once the host has
        // been idle for TRIM_AFTER (`give_back_free_memory`): a trim takes
        // up to ~0.1 s on 1,000 pages and cannot be interrupted, so it never
        // runs while keystrokes are coming (review of #1300).
        let mut trim_due = false;
        loop {
            let pause = self.cfg.keep_warm_pause;
            let req = match hot_until {
                Some(t) if Instant::now() < t && pause.is_zero() => match rx.try_recv() {
                    Ok(r) => r,
                    Err(mpsc::TryRecvError::Empty) => {
                        std::hint::spin_loop();
                        continue;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => break,
                },
                // (`--keep-warm-pause`: short sleeps instead of a spin)
                Some(t) if Instant::now() < t => match rx.recv_timeout(pause) {
                    Ok(r) => r,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // a little work between the sleeps
                        let w = Instant::now();
                        while w.elapsed() < pause {
                            std::hint::spin_loop();
                        }
                        continue;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                },
                _ if trim_due => match rx.recv_timeout(TRIM_AFTER) {
                    Ok(r) => r,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        trim_due = false;
                        give_back_free_memory();
                        continue;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                },
                _ => match rx.recv() {
                    Ok(r) => r,
                    Err(_) => break,
                },
            };
            let compiled = matches!(req, Req::Compile { .. });
            match req {
                Req::Warm(done) => {
                    let _ = done.send(self.warm());
                }
                Req::Closed(id) => {
                    self.peers.remove(&id);
                }
                Req::Compile { conn, req, t0 } => {
                    conn.queued.fetch_sub(1, Ordering::SeqCst);
                    let c = conn.clone();
                    self.compile(conn, req, t0);
                    // DONE is out: prepare the next keystroke's restore
                    // while nothing waits (`incr::Session::prepare_next`)
                    // (FLASHTEX_NO_PREPARE=1 leaves it out, for A/B)
                    let prepare = std::env::var_os("FLASHTEX_NO_PREPARE").is_none();
                    if let Some(d) = self.doc.as_mut().filter(|_| prepare) {
                        d.session
                            .prepare_next(&mut || c.queued.load(Ordering::SeqCst) > 0);
                    }
                    trim_due = true;
                }
            }
            if compiled && !self.cfg.keep_warm.is_zero() {
                hot_until = Some(Instant::now() + self.cfg.keep_warm);
            }
        }
    }

    /// Warm the process up: a one-page LaTeX document in a scratch
    /// directory starts kpathsea, reads the font map and loads the format,
    /// so that a document's first compile (or its reopening from S₀) does
    /// not pay for them.
    fn warm(&mut self) -> Result<f64, String> {
        let t = Instant::now();
        let dir = std::env::temp_dir().join(format!("flashtex-host-warm-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        std::fs::write(
            dir.join("flashtex-warm.tex"),
            "\\documentclass{article}\\begin{document}Warm.\\end{document}\n",
        )
        .map_err(|e| e.to_string())?;
        let here = std::env::current_dir().ok();
        std::env::set_current_dir(&dir).map_err(|e| e.to_string())?;
        let argv: Vec<String> = [
            "pdftex",
            "-fmt=pdflatex",
            "-interaction=batchmode",
            "flashtex-warm.tex",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let o = crate::cli::parse(&argv);
        let r = {
            let mut s = incr::Session::new(o, None, self.cfg.opts.clone());
            s.compile(None).map(|_| ())
        };
        if let Some(h) = here {
            let _ = std::env::set_current_dir(h);
        }
        let _ = std::fs::remove_dir_all(&dir);
        r?;
        Ok(t.elapsed().as_secs_f64())
    }

    fn s0_path(&self, job: &Job) -> Option<PathBuf> {
        let dir = self.cfg.s0_cache.as_ref()?;
        let key = format!(
            "{}\0{}\0{}\0{:?}\0{}\0{}",
            job.root.display(),
            job.main,
            job.format,
            job.shell,
            job.out_dir.display(),
            job.jobname
        );
        let h = crate::persist::hash128(key.as_bytes());
        Some(dir.join(format!("{:016x}{:016x}.s0", h[0], h[1])))
    }

    /// Make `job` the resident document.
    fn open(&mut self, job: Job) -> Result<(), String> {
        self.doc = None;
        std::env::set_current_dir(&job.root).map_err(|e| format!("{}: {e}", job.root.display()))?;
        let mut argv = vec!["pdftex".to_string()];
        argv.extend(job.argv());
        let o = crate::cli::parse(&argv);
        let session = incr::Session::new(o, None, self.cfg.opts.clone());
        displaylist::init_with_sink(Box::new(HostSink(self.live.clone())));
        self.live.borrow_mut().clear();
        self.gens += 1;
        self.doc = Some(Doc {
            job,
            session,
            gen: self.gens,
            compiles: 0,
            texts: HashMap::new(),
        });
        Ok(())
    }

    fn compile(&mut self, conn: Arc<Conn>, req: Json, t0: Instant) {
        let queue_ms = t0.elapsed().as_secs_f64() * 1e3;
        super::crash::serving(&format!(
            "COMPILE id {} main {} ({} edits, {} buffers) from connection {}",
            req.int_field("id").unwrap_or(-1),
            req.str_field("main").unwrap_or("?"),
            req.get("edits")
                .and_then(Json::as_array)
                .map_or(0, |a| a.len()),
            req.get("buffers")
                .and_then(Json::as_array)
                .map_or(0, |a| a.len()),
            conn.id
        ));
        let cpu0 = incr::thread_cpu_s();
        let out = conn.out.clone();
        let Some(id) = req.int_field("id") else {
            server::error(&out, None, "request", "COMPILE needs an integer id");
            return;
        };
        let job = match Job::parse(&req, conn.id) {
            Ok(j) => j,
            Err(e) => return server::error(&out, Some(id), "request", &e),
        };
        let t_apply = Instant::now();
        if let Err(e) = apply_changes(&job.root, &req, &mut self.written) {
            return server::error(&out, Some(id), "request", &e);
        }
        let apply_ms = t_apply.elapsed().as_secs_f64() * 1e3;
        let started = |mode: &str, keep: bool, extra: Vec<(String, Json)>| {
            let mut kv = vec![
                ("id".to_string(), Json::Int(id)),
                ("pid".to_string(), Json::Int(std::process::id() as i64)),
                (
                    "argv".to_string(),
                    Json::Arr(job.argv().iter().map(|a| js(a.as_str())).collect()),
                ),
                (
                    "output_dir".to_string(),
                    js(job.out_dir.display().to_string()),
                ),
                ("mode".to_string(), js(mode)),
                ("keep".to_string(), Json::Bool(keep)),
            ];
            kv.extend(extra);
            server::send_json(&out, kind::STARTED, &Json::Obj(kv));
        };
        // Superseded (a newer COMPILE is waiting) or cancelled before it
        // started: its edits are on disk; the next compile compiles them.
        if conn.queued.load(Ordering::SeqCst) > 0 || conn.is_cancelled(id) {
            started("resident", true, vec![]);
            server::send_json(
                &out,
                kind::DONE,
                &obj([
                    ("id", Json::Int(id)),
                    ("status", js("cancelled")),
                    ("pages", Json::Int(self.live.borrow().pages.len() as i64)),
                ]),
            );
            return;
        }
        // Pages persist across compiles only for a 3.1 client that asks.
        let incremental =
            conn.minor >= 1 && req.get("incremental").and_then(Json::as_bool) == Some(true);
        let s0_path = self.s0_path(&job);
        if self.doc.as_ref().map(|d| &d.job) != Some(&job) {
            if let Err(e) = self.open(job.clone()) {
                return server::error(&out, Some(id), "request", &e);
            }
        }
        let doc = self.doc.as_mut().unwrap();
        // Source spans follow their lines through the edits.
        let t_moved = Instant::now();
        move_spans(doc);
        let move_ms = t_moved.elapsed().as_secs_f64() * 1e3;
        let mut ps = self.peers.remove(&conn.id).unwrap_or_else(PeerState::new);
        let keep = incremental && ps.doc_gen == doc.gen;
        if !keep {
            ps.reset();
            ps.doc_gen = doc.gen;
        }
        ps.peer.have_fonts = displaylist::parse_font_keys(&server::have_fonts(&req).join(","));
        ps.peer.font_formats = Some(displaylist::parse_font_formats(
            &server::font_formats(&req).join(","),
        ));
        let reopen = doc.compiles == 0 && s0_path.as_ref().is_some_and(|p| p.is_file());
        started(
            "resident",
            keep,
            vec![("incremental".to_string(), Json::Bool(incremental))],
        );
        if keep {
            if let Some(src) = ps.peer.moved_spans() {
                server::send(&out, kind::SOURCES, &src);
            }
        }
        let old_count = self.live.borrow().pages.len();
        self.live.borrow_mut().target = Some(Target {
            ps,
            conn: conn.clone(),
            id,
            t0,
            incremental,
            old_count,
            next: 0,
            first_index: None,
            first_page_ms: None,
            emitted: 0,
            bytes: 0,
            broken: false,
            went_quiet: false,
            cpu0,
            emit0: displaylist::emit_ns(),
            send_ns: 0,
            first_cpu_ms: None,
            first_emit_ms: None,
            first_send_ms: None,
        });
        let stop_at = req
            .int_field("viewport")
            .filter(|v| *v >= 0)
            .map(|v| v as usize + 1);
        let doc = self.doc.as_mut().unwrap();
        // Preemption: a newer COMPILE from this client (typing) or its
        // CANCEL stops the run at its next page or segment checkpoint; the
        // next compile keeps what it typeset, or goes back to the run it was
        // replacing (`incr::Session::compile`).
        {
            let c = conn.clone();
            doc.session
                .set_preempt(Some(std::rc::Rc::new(move |_pass, _pages| {
                    c.queued.load(Ordering::SeqCst) > 0 || c.is_cancelled(id)
                })));
        }
        let t_run = Instant::now();
        let mut open_error = None;
        let first = if reopen {
            match doc
                .session
                .open_s0(&s0_path.as_ref().unwrap().to_string_lossy(), stop_at)
            {
                Ok(r) => Ok(r),
                Err(e) => {
                    open_error = Some(e);
                    doc.session.compile(stop_at)
                }
            }
        } else {
            doc.session.compile(stop_at)
        };
        let mut viewport_ms = None;
        let result = match first {
            Ok(mut rep) if rep.paused && !rep.preempted => {
                // L4: the requested page is there. Say which pages are
                // current, then typeset the rest.
                viewport_ms = Some(t0.elapsed().as_secs_f64() * 1e3);
                {
                    let mut live = self.live.borrow_mut();
                    let mut t = live.target.take().unwrap();
                    t.pages_status(old_count.max(t.next as usize), false);
                    live.target = Some(t);
                }
                match doc.session.finish() {
                    Ok(r2) => {
                        rep.status = r2.status;
                        rep.pages = r2.pages;
                        rep.converged_at = r2.converged_at;
                        rep.rerun_pages = r2.rerun_pages;
                        rep.paused = r2.paused;
                        rep.preempted = r2.preempted;
                        Ok(rep)
                    }
                    Err(e) => Err(e),
                }
            }
            other => other,
        };
        let run_ms = t_run.elapsed().as_secs_f64() * 1e3;
        doc.session.set_preempt(None);
        let mut live = self.live.borrow_mut();
        let mut t = live.target.take().unwrap();
        let (status, exit_code, count, mode, mut extra) = match &result {
            Ok(rep) => {
                let count = rep.pages;
                live.pages.truncate(count);
                live.catch_up(&mut t, count as u32);
                let mut extra = vec![
                    (
                        "restart_page".to_string(),
                        Json::Int(rep.restart_pages as i64),
                    ),
                    // a checkpoint between pages (a segment's), and how
                    // many bytes before the edit its input position is
                    (
                        "restart_mid_page".to_string(),
                        Json::Bool(rep.restart_mid_page),
                    ),
                    (
                        "restart_next_gap".to_string(),
                        rep.restart_next_gap.map(Json::Int).unwrap_or(Json::Null),
                    ),
                    (
                        "restart_gap".to_string(),
                        if rep.restart_gap == u64::MAX {
                            Json::Null
                        } else {
                            Json::Int(rep.restart_gap as i64)
                        },
                    ),
                    (
                        "converged_at".to_string(),
                        rep.converged_at
                            .map(|c| Json::Int(c as i64))
                            .unwrap_or(Json::Null),
                    ),
                    ("rerun_pages".to_string(), Json::Int(rep.rerun_pages as i64)),
                ];
                if let Some(r) = &rep.cold_reason {
                    extra.push(("cold_reason".to_string(), js(r.as_str())));
                }
                if let Some(e) = &open_error {
                    extra.push(("s0_error".to_string(), js(e.as_str())));
                }
                let st = if rep.status == 0 { "ok" } else { "error" };
                (st, Some(rep.status), count, rep.mode.clone(), extra)
            }
            Err(e) => (
                "failed",
                None,
                live.pages.len(),
                "failed".to_string(),
                vec![("message".to_string(), js(e.as_str()))],
            ),
        };
        // Where the time to the first page went (ms): waiting for the
        // engine thread, applying the edits, moving spans, finding the
        // restart point (of which the S0 key check and finding what
        // changed), restoring it, then the engine to the first page's
        // shipout (of which building display lists), and writing frames.
        {
            let m = |v: f64| Json::Num((v * 1e3).round() / 1e3);
            let o = |v: Option<f64>| v.map(m).unwrap_or(Json::Null);
            let mut st = vec![
                ("queue".to_string(), m(queue_ms)),
                ("apply".to_string(), m(apply_ms)),
                ("move_spans".to_string(), m(move_ms)),
                ("first_page".to_string(), o(t.first_page_ms)),
                ("first_page_cpu".to_string(), o(t.first_cpu_ms)),
                ("first_page_dl".to_string(), o(t.first_emit_ms)),
                ("first_page_send".to_string(), o(t.first_send_ms)),
            ];
            if let Ok(rep) = &result {
                st.push(("find".to_string(), m(rep.find_s * 1e3)));
                st.push(("key".to_string(), m(rep.key_s * 1e3)));
                st.push(("changes".to_string(), m(rep.changes_s * 1e3)));
                st.push(("restore".to_string(), m(rep.restore_s * 1e3)));
                st.push(("tests".to_string(), Json::Int(rep.tests as i64)));
                st.push(("test".to_string(), m(rep.test_s * 1e3)));
                if let Some((p, w, c)) = rep.edited {
                    st.push(("edited_page".to_string(), Json::Int(p as i64)));
                    st.push(("edited_wall".to_string(), m(w * 1e3)));
                    st.push(("edited_cpu".to_string(), m(c * 1e3)));
                }
            }
            st.push((
                "dl".to_string(),
                m((displaylist::emit_ns() - t.emit0) as f64 * 1e-6),
            ));
            st.push(("send".to_string(), m(t.send_ns as f64 * 1e-6)));
            st.push(("cpu".to_string(), m((incr::thread_cpu_s() - t.cpu0) * 1e3)));
            extra.push(("stages".to_string(), Json::Obj(st)));
        }
        let cancelled = t.quiet() || t.went_quiet;
        if !cancelled {
            t.pages_status(count, true);
        }
        drop(live);
        let mut ndiag = 0;
        if !cancelled {
            let term = doc.session.terminal();
            ndiag = server::diagnostics(std::io::BufReader::new(&term[..]), &out, id, &job.root);
        }
        let pdf = job.out_dir.join(format!("{}.pdf", job.jobname));
        let log = job.out_dir.join(format!("{}.log", job.jobname));
        let path_or_null = |p: &Path| {
            if p.is_file() {
                js(p.display().to_string())
            } else {
                Json::Null
            }
        };
        let ms = |v: f64| Json::Num((v * 1e3).round() / 1e3);
        let mut kv = vec![
            ("id".to_string(), Json::Int(id)),
            (
                "status".to_string(),
                js(if cancelled && status != "failed" {
                    "cancelled"
                } else {
                    status
                }),
            ),
            (
                "exit_code".to_string(),
                exit_code.map(|c| Json::Int(c as i64)).unwrap_or(Json::Null),
            ),
            ("pages".to_string(), Json::Int(count as i64)),
            ("bytes".to_string(), Json::Int(t.bytes as i64)),
            ("diagnostics".to_string(), Json::Int(ndiag as i64)),
            (
                "elapsed_ms".to_string(),
                ms(t0.elapsed().as_secs_f64() * 1e3),
            ),
            (
                "first_page_ms".to_string(),
                t.first_page_ms.map(ms).unwrap_or(Json::Null),
            ),
            (
                "viewport_ms".to_string(),
                viewport_ms.map(ms).unwrap_or(Json::Null),
            ),
            ("run_ms".to_string(), ms(run_ms)),
            ("move_spans_ms".to_string(), ms(move_ms)),
            ("mode".to_string(), js(mode.as_str())),
            ("keep".to_string(), Json::Bool(keep)),
            ("typeset_pages".to_string(), Json::Int(t.emitted as i64)),
            ("pdf".to_string(), path_or_null(&pdf)),
            ("log".to_string(), path_or_null(&log)),
        ];
        kv.extend(extra);
        // Memory accounting (FLASHTEX_MEMSTAT=1; lane P4-MEMORY): the
        // session's parts (`incr::Session::mem_stats`) and the page cache.
        if std::env::var_os("FLASHTEX_MEMSTAT").is_some() {
            let live = self.live.borrow();
            let body = |e: &Emitted| e.body.len() as i64;
            let pages: i64 = live.pages.iter().flatten().map(|c| body(&c.e)).sum();
            let forms: i64 = live.forms.values().map(|c| body(&c.e)).sum();
            let mut m: Vec<(String, Json)> = doc
                .session
                .mem_stats()
                .into_iter()
                .map(|(k, v)| (k, Json::Int(v)))
                .collect();
            m.push(("page_cache".into(), Json::Int(pages)));
            m.push(("form_cache".into(), Json::Int(forms)));
            m.push((
                "texts".into(),
                Json::Int(doc.texts.values().map(|t| t.len() as i64).sum()),
            ));
            m.push((
                "written".into(),
                Json::Int(self.written.values().map(|(_, t)| t.len() as i64).sum()),
            ));
            kv.push(("mem".into(), Json::Obj(m)));
        }
        server::send_json(&out, kind::DONE, &Json::Obj(kv));
        let failed = result.is_err();
        let cold = matches!(mode.as_str(), "cold");
        self.peers.insert(conn.id, t.ps);
        if failed {
            // The engine's state is unknown: start the document afresh.
            self.doc = None;
            self.live.borrow_mut().clear();
            return;
        }
        let doc = self.doc.as_mut().unwrap();
        doc.compiles += 1;
        remember_texts(doc);
        // Persist S₀ after a full run (off the keystroke path: DONE is out).
        if cold {
            if let Some(p) = &s0_path {
                if let Some(d) = p.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                let t = Instant::now();
                match doc.session.save_s0(&p.to_string_lossy()) {
                    // One line for a supervisor (and the measurements).
                    Ok((bytes, _)) => server::say(&format!(
                        "flashtex-host: {}",
                        obj([
                            ("saved_s0", js(p.display().to_string())),
                            ("bytes", Json::Int(bytes as i64)),
                            (
                                "ms",
                                Json::Num((t.elapsed().as_secs_f64() * 1e4).round() / 10.0)
                            ),
                        ])
                    )),
                    Err(e) => eprintln!("flashtex-host: saving S0: {e}"),
                }
            }
        }
    }
}

/// Idle time after which the host trims its heap (`give_back_free_memory`).
/// Keep-warm (2 s by default) counts towards it.
const TRIM_AFTER: std::time::Duration = std::time::Duration::from_secs(2);

/// While the engine has been idle for `TRIM_AFTER`: hand the heap's free pages
/// back to the system. glibc keeps what a compile freed (the logs a
/// retention pass merged, a detached branch, the convergence test's
/// buffers) mapped, so the host's resident memory stayed at its peak: on
/// full-1000, 1.5 GB resident for a 0.47 GB heap
/// (docs/evidence/p4-memory-2026-09-30/). macOS's allocator returns free
/// pages itself. FLASHTEX_NO_TRIM=1 leaves it out (for A/B).
fn give_back_free_memory() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        extern "C" {
            fn malloc_trim(pad: usize) -> i32;
        }
        if std::env::var_os("FLASHTEX_NO_TRIM").is_none() {
            let t = Instant::now();
            // SAFETY: no preconditions; it only releases free memory.
            unsafe { malloc_trim(0) };
            if std::env::var_os("FLASHTEX_MEMSTAT").is_some() {
                eprintln!(
                    "flashtex-host: malloc_trim {:.2} ms",
                    t.elapsed().as_secs_f64() * 1e3
                );
            }
        }
    }
}

/// Write the `buffers` (whole files) and `edits` (byte splices) of a
/// `COMPILE` to their files under `root`, as saving them would: the files
/// end up holding exactly these bytes, which the engine then reads like
/// any file. `written` keeps what the host wrote last: while a file's stat
/// signature is still the one the host left, its bytes come from there
/// instead of a read, and an edit rewrites the file from the edit on
/// (truncated or extended to the new length) rather than all of it.
fn apply_changes(root: &Path, req: &Json, written: &mut Written) -> Result<(), String> {
    use crate::system::StatSig;
    use std::os::unix::fs::FileExt;
    let target = |p: &str| -> Result<PathBuf, String> {
        let rel = Path::new(p);
        if !server::inside(rel) {
            return Err(format!("{p}: a path inside root"));
        }
        Ok(root.join(rel))
    };
    let sig = |path: &Path| StatSig::of(&path.to_string_lossy());
    // The file's bytes now: the host's copy while the file is as it left
    // it, else read.
    let current = |path: &Path, written: &mut Written| -> std::io::Result<Vec<u8>> {
        if let Some((s, d)) = written.remove(path) {
            if sig(path) == Some(s) {
                return Ok(Arc::try_unwrap(d).unwrap_or_else(|d| (*d).clone()));
            }
        }
        std::fs::read(path)
    };
    // Write `data` to `path`, whose bytes before `from` are already these.
    let write_from = |path: &Path, data: Vec<u8>, from: usize, written: &mut Written| {
        let r = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .and_then(|f| {
                f.set_len(data.len() as u64)?;
                f.write_all_at(&data[from..], from as u64)
            });
        r.map_err(|e| format!("{}: {e}", path.display()))?;
        if let Some(s) = sig(path) {
            written.insert(path.to_path_buf(), (s, Arc::new(data)));
        }
        Ok::<(), String>(())
    };
    for b in req.get("buffers").and_then(Json::as_array).unwrap_or(&[]) {
        let p = b.str_field("path").ok_or("a buffer needs path")?;
        let text = b.str_field("text").ok_or("a buffer needs text")?;
        let path = target(p)?;
        match current(&path, written) {
            Ok(d) if d.as_slice() == text.as_bytes() => {
                if let Some(s) = sig(&path) {
                    written.insert(path, (s, Arc::new(d)));
                }
            }
            _ => write_from(&path, text.as_bytes().to_vec(), 0, written)?,
        }
    }
    for e in req.get("edits").and_then(Json::as_array).unwrap_or(&[]) {
        let p = e.str_field("path").ok_or("an edit needs path")?;
        let path = target(p)?;
        let mut d = current(&path, written).map_err(|x| format!("{p}: {x}"))?;
        let at = e.int_field("offset").ok_or("an edit needs offset")?;
        let del = e.int_field("delete").unwrap_or(0);
        let ins = e.str_field("insert").unwrap_or("");
        if at < 0 || del < 0 || (at + del) as usize > d.len() {
            return Err(format!("{p}: edit outside the file"));
        }
        let (at, del) = (at as usize, del as usize);
        if d[at..at + del] == *ins.as_bytes() {
            // nothing changes (the file is left alone, as before)
            if let Some(s) = sig(&path) {
                written.insert(path, (s, Arc::new(d)));
            }
            continue;
        }
        d.splice(at..at + del, ins.bytes());
        write_from(&path, d, at, written)?;
    }
    Ok(())
}

/// The user's files the display list names, as they are now, compared
/// with what the last compile read: spans after a change move with their
/// lines ([`displaylist::move_lines`]).
fn move_spans(doc: &mut Doc) {
    for (path, old) in doc.texts.iter_mut() {
        let Ok(new) = std::fs::read(path) else {
            continue;
        };
        if new.as_slice() == old.as_slice() {
            continue;
        }
        let (from, old_end, new_end) = line_change(old, &new);
        displaylist::move_lines(path, from, old_end, new_end);
        *old = Arc::new(new);
    }
}

/// Record the user's files the display list names that the host has not
/// seen yet (as the compile just read them).
fn remember_texts(doc: &mut Doc) {
    let root = doc.job.root.clone();
    for (_, p) in displaylist::files() {
        if doc.texts.contains_key(&p) || !Path::new(&p).starts_with(&root) {
            continue;
        }
        if let Ok(d) = std::fs::read(&p) {
            doc.texts.insert(p, Arc::new(d));
        }
    }
}

/// The lines that differ between `old` and `new`: (first changed line,
/// end of the change in `old`, end in `new`), 1-based, ends exclusive.
fn line_change(old: &[u8], new: &[u8]) -> (u32, u32, u32) {
    let a: Vec<&[u8]> = old.split(|&c| c == b'\n').collect();
    let b: Vec<&[u8]> = new.split(|&c| c == b'\n').collect();
    let p = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let max_s = a.len().min(b.len()) - p;
    let s = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take(max_s)
        .take_while(|(x, y)| x == y)
        .count();
    (
        p as u32 + 1,
        (a.len() - s) as u32 + 1,
        (b.len() - s) as u32 + 1,
    )
}

#[cfg(test)]
mod tests {
    use super::line_change;

    #[test]
    fn line_changes() {
        // One line edited in place.
        assert_eq!(line_change(b"a\nb\nc\n", b"a\nB\nc\n"), (2, 3, 3));
        // A line inserted.
        assert_eq!(line_change(b"a\nb\nc\n", b"a\nb\nx\nc\n"), (3, 3, 4));
        // A line removed.
        assert_eq!(line_change(b"a\nb\nc\n", b"a\nc\n"), (2, 3, 2));
        // A character inside a line.
        assert_eq!(line_change(b"hello\nworld", b"hello\nworlds"), (2, 3, 3));
    }
}
