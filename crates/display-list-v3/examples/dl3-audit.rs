//! `dl3-audit`: an incremental client of an engine host, held to a fresh
//! run (lane AUDIT-DL). It drives one host through an edit script as the app
//! does (`"incremental": true`, the text in `buffers`), holding what the
//! frames say a client holds (spec §5, §6.4: `STARTED.keep`, pages replaced
//! by index, fonts, images and forms bound by id, spans, `PAGES`/`DONE`
//! counts). After every step it compiles the same text from scratch in a
//! second host (a job of its own: its own copy of the output directory as
//! the step found it) and compares every page, with each id resolved to what
//! it named when the page arrived: fonts and images by key, forms by their
//! content, spans by file and line, links and destinations.
//!
//!     dl3-audit (--host PATH [--host-arg A]... | --connect SOCK --scratch-connect SOCK)
//!         --work DIR [--format NAME] [--image FILE]... [--steps a,b,...]
//!         [--xetex] [--timeout SECS]
//!
//! A step passes only when both compiles end `DONE` `ok`, both have the
//! same page count and it is above 0, and every page compared equal: a step
//! whose compiles failed, or that had nothing to compare, fails. A run that
//! compared no page at all fails. `--connect` uses hosts already listening
//! (the self-tests' scripted hosts); `--timeout` (default 1800 s) bounds the
//! whole run. The hosts inherit the environment. Exit status 1 on any
//! failure.

use flashtex_display_list::client::{Client, CompileRequest, Event};
use flashtex_display_list::held::Model;
use flashtex_display_list::json::Json;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The steps, in order, when `--steps` is not given.
pub const ALL_STEPS: &str =
    "open,open2,same,comment,drop3,regrow,noimgA,url2,word,supersede,cancel,reconnect,shrink-cancel";

struct Host {
    /// None: a host already listening (`--connect`), not ours to stop.
    child: Option<Child>,
    sock: PathBuf,
}

impl Drop for Host {
    fn drop(&mut self) {
        if let Some(c) = &mut self.child {
            let _ = c.kill();
            let _ = c.wait();
            let _ = std::fs::remove_file(&self.sock);
        }
    }
}

fn start_host(exe: &str, args: &[String], sock: &Path, log: &Path) -> Result<Host, String> {
    let _ = std::fs::remove_file(sock);
    let err = std::fs::File::create(log).map_err(|e| format!("{}: {e}", log.display()))?;
    let out = err.try_clone().map_err(|e| e.to_string())?;
    let child = Command::new(exe)
        .arg("--socket")
        .arg(sock)
        .args(args)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        .spawn()
        .map_err(|e| format!("cannot start host {exe}: {e}"))?;
    let mut h = Host {
        child: Some(child),
        sock: sock.to_path_buf(),
    };
    let t = Instant::now();
    loop {
        if sock.exists() && flashtex_display_list::transport::Stream::connect(sock).is_ok() {
            return Ok(h);
        }
        if let Ok(Some(st)) = h.child.as_mut().unwrap().try_wait() {
            return Err(format!(
                "host {exe} exited ({st}) before listening; see {}",
                log.display()
            ));
        }
        if t.elapsed() > Duration::from_secs(900) {
            return Err(format!("host {exe} did not listen in 900 s"));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Read events into `m` until a DONE for `id` (others' DONEs are applied
/// too); `stop` may end it early (returns None then).
fn run_until(
    c: &mut Client,
    m: &mut Model,
    id: i64,
    mut stop: impl FnMut(&Event) -> bool,
) -> Option<Json> {
    loop {
        let ev = match c.next_event() {
            Ok(Some(e)) => e,
            Ok(None) => {
                m.violations.push("host closed the connection".into());
                return None;
            }
            Err(e) => {
                m.violations.push(format!("decode error: {e}"));
                return None;
            }
        };
        if stop(&ev) {
            m.apply(ev);
            return None;
        }
        let done_id = match &ev {
            Event::Done(d) => d.int_field("id"),
            _ => None,
        };
        if let Some(d) = m.apply(ev) {
            if done_id == Some(id) {
                return Some(d);
            }
        }
    }
}

/// Send `r`; a write that fails is recorded on `m` (the step then fails).
fn send(c: &mut Client, r: &CompileRequest, m: &mut Model) -> bool {
    match c.compile(r) {
        Ok(()) => true,
        Err(e) => {
            m.violations.push(format!("COMPILE {} not sent: {e}", r.id));
            false
        }
    }
}

fn status(d: Option<&Json>) -> Option<String> {
    d.and_then(|d| d.str_field("status").map(String::from))
}

fn copy_dir(from: &Path, to: &Path) {
    let _ = std::fs::remove_dir_all(to);
    let st = Command::new("cp")
        .arg("-a")
        .arg(from)
        .arg(to)
        .status()
        .unwrap();
    assert!(st.success());
}

/// The document: sections of text, two images, links, a footnote, a form.
fn document(v: &str, images: &[String], xetex: bool) -> String {
    let has = |f: &str| v.split('+').any(|x| x == f);
    let para = "Words and more words that fill the page, again and again, until the \
                paragraph is long enough to break into several lines of text. ";
    let mut d = String::from("\\documentclass{article}\n\\usepackage{graphicx}\n");
    if xetex {
        d.push_str("\\usepackage{fontspec}\n");
    }
    d.push_str("\\usepackage{hyperref}\n");
    if !xetex {
        d.push_str("\\setbox0\\hbox{\\bfseries FORM box}\\immediate\\pdfxform0 \\edef\\myform{\\the\\pdflastxform}\n");
        d.push_str("\\newcommand\\drawform{\\pdfrefxform\\myform}\n");
    } else {
        d.push_str("\\newcommand\\drawform{}\n");
    }
    d.push_str("\\begin{document}\n");
    if has("comment") {
        d.push_str("% a comment line: every later line moves\n");
    }
    d.push_str("\\tableofcontents\n");
    for i in 1..=10 {
        if i == 3 && has("drop3") {
            continue;
        }
        let _ = writeln!(d, "\\section{{Part {i}}}\\label{{p{i}}}");
        if i == 2 && !has("noimgA") && !images.is_empty() {
            let _ = writeln!(
                d,
                "\\includegraphics[width=3cm]{{{}}}\n\\drawform\n",
                images[0]
            );
        }
        if i == 6 && images.len() > 1 {
            let _ = writeln!(d, "\\includegraphics[width=4cm]{{{}}}\n", images[1]);
        }
        if i == 7 {
            d.push_str("\\drawform\n\n");
        }
        if i == 3 {
            let url = if has("url2") {
                "https://example.org/b"
            } else {
                "https://example.org/a"
            };
            let _ = writeln!(d, "A link: \\href{{{url}}}{{the site}}.\n");
        }
        if i == 4 {
            d.push_str("Text with a note.\\footnote{The footnote.}\n\n");
        }
        for k in 0..3 {
            if i == 8 && k == 1 && has("word") {
                d.push_str("Changed ");
            }
            d.push_str(&para.repeat(3));
            d.push_str("\n\n");
        }
        let _ = writeln!(
            d,
            "See~\\ref{{p{}}} on page~\\pageref{{p{}}}.\n",
            11 - i,
            11 - i
        );
    }
    d.push_str("\\end{document}\n");
    d
}

/// What is wrong with one step: the client's state `m` (its compile ended
/// with `md`) against the scratch compile's `s` (ended with `sd`). Both
/// compiles must end `ok` with the same page count, above 0; every page is
/// compared. Returns the problems and the number of pages compared.
pub fn verdict(
    m: &Model,
    md: Option<&Json>,
    s: &Model,
    sd: Option<&Json>,
    o1: &str,
    o2: &str,
) -> (Vec<String>, usize) {
    let mut bad = vec![];
    bad.extend(m.violations.iter().map(|v| format!("client: {v}")));
    bad.extend(s.violations.iter().map(|v| format!("scratch: {v}")));
    for (who, d) in [("client", md), ("scratch", sd)] {
        match (d, status(d)) {
            (None, _) => bad.push(format!("{who}: no DONE")),
            (Some(_), Some(st)) if st == "ok" => {}
            (Some(d), st) => bad.push(format!(
                "{who}: DONE status {st:?}, not \"ok\": {}",
                d.to_string().chars().take(200).collect::<String>()
            )),
        }
    }
    if m.count == 0 || s.count == 0 {
        bad.push(format!(
            "nothing to compare: client {} pages, scratch {}",
            m.count, s.count
        ));
    }
    if m.count != s.count {
        bad.push(format!("page count {} vs scratch {}", m.count, s.count));
    }
    if m.pages.len() != m.count {
        bad.push(format!(
            "client holds {:?}, count {}",
            m.pages.keys().collect::<Vec<_>>(),
            m.count
        ));
    }
    let n = m.count.max(s.count);
    for i in 0..n as u32 {
        let (a, b) = (m.resolve(i, o1), s.resolve(i, o2));
        if a != b {
            let (la, lb): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
            let k = (0..la.len().max(lb.len()))
                .find(|&k| la.get(k) != lb.get(k))
                .unwrap_or(0);
            bad.push(format!(
                "page {i} line {k}:\n    client:  {}\n    scratch: {}",
                la.get(k).unwrap_or(&"<end>"),
                lb.get(k).unwrap_or(&"<end>")
            ));
        }
    }
    (bad, n)
}

/// How the audit reaches its hosts.
pub enum Hosts {
    /// Start `exe --socket S args...` (the incremental host and, lazily,
    /// the scratch host).
    Spawn { exe: String, args: Vec<String> },
    /// Hosts already listening.
    Connect { main: PathBuf, scratch: PathBuf },
}

pub struct Opts {
    pub hosts: Hosts,
    pub work: PathBuf,
    pub format: Option<String>,
    /// Image files, copied into the project.
    pub images: Vec<String>,
    pub steps: Option<String>,
    pub xetex: bool,
    pub timeout: Duration,
}

struct Audit {
    hosts: Hosts,
    work: PathBuf,
    format: Option<String>,
    images: Vec<String>,
    xetex: bool,
    proj: PathBuf,
    out: PathBuf,
    next_id: i64,
    failures: usize,
    compared: usize,
    scratch: Option<Host>,
    scratch_n: usize,
}

impl Audit {
    fn req(&mut self, text: &str) -> CompileRequest {
        self.next_id += 1;
        let mut r = CompileRequest::new(self.next_id, self.proj.to_str().unwrap(), "main.tex");
        r.output_dir = Some(self.out.to_str().unwrap().into());
        r.incremental = true;
        r.buffers = vec![("main.tex".into(), text.to_string())];
        if let Some(f) = &self.format {
            r.format = f.clone();
        }
        r
    }

    fn scratch_host(&mut self) -> Result<PathBuf, String> {
        if self.scratch.is_none() {
            self.scratch = Some(match &self.hosts {
                Hosts::Spawn { exe, args } => start_host(
                    exe,
                    args,
                    &self.work.join("scratch.sock"),
                    &self.work.join("scratch-host.log"),
                )?,
                Hosts::Connect { scratch, .. } => Host {
                    child: None,
                    sock: scratch.clone(),
                },
            });
        }
        Ok(self.scratch.as_ref().unwrap().sock.clone())
    }

    fn fail(&mut self, step: &str, bad: &[String]) {
        self.failures += 1;
        println!("FAIL {step}: {} problems", bad.len());
        for b in bad.iter().take(12) {
            println!("  {b}");
        }
    }

    /// The scratch compile of the project as it is now, with `snap` (the
    /// output directory as the step found it).
    fn scratch_compile(&mut self, snap: &Path) -> (Model, Option<Json>, PathBuf) {
        self.scratch_n += 1;
        let out2 = self.work.join(format!("scratch-out{}", self.scratch_n));
        copy_dir(snap, &out2);
        let mut s = Model::default();
        let sock = match self.scratch_host() {
            Ok(p) => p,
            Err(e) => {
                s.violations.push(e);
                return (s, None, out2);
            }
        };
        let mut c = match Client::connect(&sock) {
            Ok(c) => c,
            Err(e) => {
                s.violations.push(format!("connect {}: {e}", sock.display()));
                return (s, None, out2);
            }
        };
        let mut r = CompileRequest::new(1, self.proj.to_str().unwrap(), "main.tex");
        r.output_dir = Some(out2.to_str().unwrap().into());
        if let Some(f) = &self.format {
            r.format = f.clone();
        }
        let d = if send(&mut c, &r, &mut s) {
            run_until(&mut c, &mut s, 1, |_| false)
        } else {
            None
        };
        let _ = c.bye();
        (s, d, out2)
    }

    /// Compare `m` (its compile ended with `md`) with a compile from
    /// scratch; `extra` are problems the step found itself, `note` what the
    /// step did not exercise.
    fn check(
        &mut self,
        step: &str,
        m: &Model,
        md: Option<&Json>,
        snap: &Path,
        mut extra: Vec<String>,
        note: &str,
    ) {
        let (s, sd, out2) = self.scratch_compile(snap);
        let (o1, o2) = (
            self.out.to_str().unwrap().to_string(),
            out2.to_str().unwrap().to_string(),
        );
        let (mut bad, n) = verdict(m, md, &s, sd.as_ref(), &o1, &o2);
        bad.append(&mut extra);
        let smode = sd
            .as_ref()
            .and_then(|d| d.str_field("mode").map(String::from));
        if bad.is_empty() {
            self.compared += n;
            println!(
                "OK   {step}: {n} pages compared (scratch mode {smode:?}){}{note}",
                if self.xetex { " (no forms under --xetex)" } else { "" }
            );
        } else {
            self.fail(step, &bad);
            println!("  (client {:?}, scratch {:?} mode {smode:?})", status(md), status(sd.as_ref()));
            let dir = self.work.join(format!("fail-{step}"));
            let _ = std::fs::create_dir_all(&dir);
            for i in 0..m.count.max(s.count) as u32 {
                let _ = std::fs::write(dir.join(format!("client-{i}.txt")), m.resolve(i, &o1));
                let _ = std::fs::write(dir.join(format!("scratch-{i}.txt")), s.resolve(i, &o2));
            }
        }
    }
}

pub fn parse(mut a: impl Iterator<Item = String>) -> Result<Opts, String> {
    let (mut exe, mut args, mut work, mut format, mut images, mut steps) =
        (None, vec![], None, None, vec![], None);
    let (mut connect, mut scratch) = (None, None);
    let mut xetex = false;
    let mut timeout = Duration::from_secs(1800);
    while let Some(x) = a.next() {
        let mut val = || a.next().ok_or(format!("{x} needs a value"));
        match x.as_str() {
            "--host" => exe = Some(val()?),
            "--host-arg" => args.push(val()?),
            "--connect" => connect = Some(PathBuf::from(val()?)),
            "--scratch-connect" => scratch = Some(PathBuf::from(val()?)),
            "--work" => work = Some(PathBuf::from(val()?)),
            "--format" => format = Some(val()?),
            "--image" => images.push(val()?),
            "--steps" => steps = Some(val()?),
            "--timeout" => {
                timeout = Duration::from_secs(val()?.parse().map_err(|e| format!("--timeout: {e}"))?)
            }
            "--xetex" => xetex = true,
            _ => return Err(format!("unknown argument {x}")),
        }
    }
    let hosts = match (exe, connect, scratch) {
        (Some(exe), None, None) => Hosts::Spawn { exe, args },
        (None, Some(main), Some(scratch)) => Hosts::Connect { main, scratch },
        _ => return Err("give --host, or both --connect and --scratch-connect".into()),
    };
    Ok(Opts {
        hosts,
        work: work.ok_or("--work is required")?,
        format,
        images,
        steps,
        xetex,
        timeout,
    })
}

/// Run the audit; returns the number of failures (a step, or the run as a
/// whole when it compared nothing).
pub fn run(o: Opts) -> usize {
    let work = o.work.clone();
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    let proj = work.join("proj");
    let out = work.join("out");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let mut names = vec![];
    for (k, f) in o.images.iter().enumerate() {
        let ext = Path::new(f).extension().and_then(|e| e.to_str()).unwrap_or("png");
        let n = format!("fig{k}.{ext}");
        if let Err(e) = std::fs::copy(f, proj.join(&n)) {
            println!("FAIL run: --image {f}: {e}");
            return 1;
        }
        names.push(n);
    }
    // A wall-clock bound for unattended runs: a host that never answers
    // ends the run as a failure instead of holding a slot.
    let current = Arc::new(Mutex::new(String::from("start")));
    let (tx, rx) = mpsc::channel::<()>();
    {
        let current = current.clone();
        let limit = o.timeout;
        std::thread::spawn(move || {
            if let Err(mpsc::RecvTimeoutError::Timeout) = rx.recv_timeout(limit) {
                println!(
                    "FAIL timeout: no end after {} s (step {})",
                    limit.as_secs(),
                    current.lock().map(|c| c.clone()).unwrap_or_default()
                );
                std::process::exit(1);
            }
        });
    }
    let mut au = Audit {
        hosts: o.hosts,
        work: work.clone(),
        format: o.format,
        images: names,
        xetex: o.xetex,
        proj: proj.clone(),
        out: out.clone(),
        next_id: 0,
        failures: 0,
        compared: 0,
        scratch: None,
        scratch_n: 0,
    };
    let doc = |v: &str, au: &Audit| document(v, &au.images, au.xetex);
    std::fs::write(proj.join("main.tex"), doc("base", &au)).unwrap();
    let host = match &au.hosts {
        Hosts::Spawn { exe, args } => start_host(
            exe,
            args,
            &work.join("host.sock"),
            &work.join("host.log"),
        ),
        Hosts::Connect { main, .. } => Ok(Host {
            child: None,
            sock: main.clone(),
        }),
    };
    let host = match host {
        Ok(h) => h,
        Err(e) => {
            println!("FAIL run: {e}");
            println!("failures: 1");
            return 1;
        }
    };
    let mut c = match Client::connect(&host.sock) {
        Ok(c) => c,
        Err(e) => {
            println!("FAIL run: connect {}: {e}", host.sock.display());
            println!("failures: 1");
            return 1;
        }
    };
    let mut m = Model::default();
    let steps = o.steps.unwrap_or(ALL_STEPS.to_string());
    let snap = work.join("snap");
    let mut ran = 0;
    for step in steps.split(',') {
        *current.lock().unwrap() = step.to_string();
        let (variant, how) = match step {
            "open" | "open2" | "same" => ("base", "plain"),
            "comment" => ("comment", "plain"),
            "drop3" => ("comment+drop3", "plain"),
            "regrow" => ("comment", "plain"),
            "noimgA" => ("comment+noimgA", "plain"),
            "url2" => ("comment+noimgA+url2", "plain"),
            "word" => ("comment+noimgA+url2+word", "plain"),
            "supersede" => ("comment+url2", "supersede"),
            "cancel" => ("comment", "cancel"),
            "reconnect" => ("base", "reconnect"),
            "shrink-cancel" => ("base+drop3", "cancel"),
            _ => {
                au.fail(step, &[format!("unknown step {step}")]);
                continue;
            }
        };
        if step == "noimgA" && au.images.len() < 2 {
            // without the first image nothing differs from `regrow`, and no
            // image id shifts: the step would pass testing nothing
            au.fail(step, &["needs two --image (the step removes the first)".into()]);
            continue;
        }
        ran += 1;
        copy_dir(&out, &snap);
        let text = doc(variant, &au);
        let t = Instant::now();
        let mut extra = vec![];
        let mut note = String::new();
        let d = match how {
            "plain" => {
                let r = au.req(&text);
                if send(&mut c, &r, &mut m) {
                    run_until(&mut c, &mut m, r.id, |_| false)
                } else {
                    None
                }
            }
            "supersede" => {
                // a keystroke and, at once, another (the first superseded)
                let mid = doc("comment+noimgA+url2", &au);
                let r1 = au.req(&mid);
                let r2 = au.req(&text);
                // wait for the first page of r1, then send r2
                let early = if send(&mut c, &r1, &mut m) {
                    run_until(&mut c, &mut m, r1.id, |e| matches!(e, Event::Page(_)))
                } else {
                    None
                };
                if early.is_some() {
                    note = "; not exercised: the first compile ended before a page".into();
                }
                if send(&mut c, &r2, &mut m) {
                    run_until(&mut c, &mut m, r2.id, |_| false)
                } else {
                    None
                }
            }
            "cancel" => {
                // the edit cancelled after its first page; then compiled again
                let r1 = au.req(&text);
                let mut paged = false;
                let early = if send(&mut c, &r1, &mut m) {
                    run_until(&mut c, &mut m, r1.id, |e| {
                        paged = matches!(e, Event::Page(_));
                        paged
                    })
                } else {
                    None
                };
                // cancel only a compile still running: after its DONE the
                // CANCEL is ignored (spec §6.4) and no second DONE comes
                let d1 = if paged {
                    if let Err(e) = c.cancel(r1.id) {
                        m.violations.push(format!("CANCEL not sent: {e}"));
                    }
                    run_until(&mut c, &mut m, r1.id, |_| false)
                } else {
                    early
                };
                match status(d1.as_ref()).as_deref() {
                    Some("cancelled") => {}
                    Some("ok") => {
                        note = "; not exercised: the compile ended before the CANCEL".into()
                    }
                    st => extra.push(format!("cancelled compile: DONE status {st:?}")),
                }
                let r2 = au.req(&text);
                if send(&mut c, &r2, &mut m) {
                    run_until(&mut c, &mut m, r2.id, |_| false)
                } else {
                    None
                }
            }
            "reconnect" => {
                // the connection drops after the edit's first page; a new
                // client (holding nothing) compiles the same text
                let r1 = au.req(&text);
                if send(&mut c, &r1, &mut m)
                    && run_until(&mut c, &mut m, r1.id, |e| matches!(e, Event::Page(_))).is_some()
                {
                    note = "; not exercised: the compile ended before a page".into();
                }
                drop(c);
                c = match Client::connect(&host.sock) {
                    Ok(c) => c,
                    Err(e) => {
                        // the host is gone: no later step can run
                        println!("FAIL {step}: reconnect: {e}");
                        println!("failures: {}", au.failures + 1);
                        return au.failures + 1;
                    }
                };
                // a new client holds nothing; what went wrong before stays
                let v = std::mem::take(&mut m.violations);
                m = Model::default();
                m.violations = v;
                let r2 = au.req(&text);
                if send(&mut c, &r2, &mut m) {
                    run_until(&mut c, &mut m, r2.id, |_| false)
                } else {
                    None
                }
            }
            _ => unreachable!(),
        };
        println!(
            "     {step}: {:.1}s keep={:?} sent={:?} done={}",
            t.elapsed().as_secs_f64(),
            m.keep,
            m.sent,
            d.as_ref()
                .map(|d| d.to_string())
                .unwrap_or_default()
                .chars()
                .take(160)
                .collect::<String>()
        );
        // the scratch compile reads the project as the step left it
        au.check(step, &m, d.as_ref(), &snap, extra, &note);
        m.violations.clear();
    }
    let _ = c.bye();
    drop(host);
    drop(tx);
    if au.compared == 0 {
        println!("FAIL run: no page was compared ({ran} steps ran)");
        au.failures += 1;
    }
    println!(
        "steps: {ran}, pages compared: {}, failures: {}",
        au.compared, au.failures
    );
    au.failures
}

#[allow(dead_code)]
fn main() {
    let o = match parse(std::env::args().skip(1)) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("dl3-audit: {e}");
            std::process::exit(2);
        }
    };
    let failures = run(o);
    std::process::exit(if failures > 0 { 1 } else { 0 });
}
