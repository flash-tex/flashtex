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
//!     dl3-audit --host PATH [--host-arg A]... --work DIR [--format NAME]
//!         [--image FILE]... [--steps a,b,...] [--xetex]
//!
//! The hosts inherit the environment. Exit status 1 on any difference.

use flashtex_display_list::client::{Client, CompileRequest, Event};
use flashtex_display_list::held::Model;
use flashtex_display_list::json::Json;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Host {
    child: Child,
    sock: PathBuf,
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.sock);
    }
}

fn start_host(exe: &str, args: &[String], sock: &Path, log: &Path) -> Host {
    let _ = std::fs::remove_file(sock);
    let err = std::fs::File::create(log).unwrap();
    let child = Command::new(exe)
        .arg("--socket")
        .arg(sock)
        .args(args)
        .stdin(Stdio::null())
        .stdout(err.try_clone().unwrap())
        .stderr(err)
        .spawn()
        .unwrap();
    let t = Instant::now();
    loop {
        if sock.exists() && flashtex_display_list::transport::Stream::connect(sock).is_ok() {
            break;
        }
        assert!(
            t.elapsed() < Duration::from_secs(900),
            "host did not listen"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    Host {
        child,
        sock: sock.to_path_buf(),
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

struct Audit {
    exe: String,
    args: Vec<String>,
    work: PathBuf,
    format: Option<String>,
    images: Vec<String>,
    xetex: bool,
    proj: PathBuf,
    out: PathBuf,
    next_id: i64,
    failures: usize,
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

    fn scratch_host(&mut self) -> PathBuf {
        if self.scratch.is_none() {
            let sock = self.work.join("scratch.sock");
            self.scratch = Some(start_host(
                &self.exe,
                &self.args,
                &sock,
                &self.work.join("scratch-host.log"),
            ));
        }
        self.scratch.as_ref().unwrap().sock.clone()
    }

    /// Compare `m` with a compile from scratch of the project as it is now,
    /// with `snap` (the output directory as the step found it).
    fn check(&mut self, step: &str, m: &Model, snap: &Path) {
        let sock = self.scratch_host();
        self.scratch_n += 1;
        let out2 = self.work.join(format!("scratch-out{}", self.scratch_n));
        copy_dir(snap, &out2);
        let mut c = Client::connect(&sock).unwrap();
        let mut s = Model::default();
        let mut r = CompileRequest::new(1, self.proj.to_str().unwrap(), "main.tex");
        r.output_dir = Some(out2.to_str().unwrap().into());
        if let Some(f) = &self.format {
            r.format = f.clone();
        }
        c.compile(&r).unwrap();
        let d = run_until(&mut c, &mut s, 1, |_| false);
        let _ = c.bye();
        let mut bad = vec![];
        bad.extend(m.violations.iter().map(|v| format!("client: {v}")));
        bad.extend(s.violations.iter().map(|v| format!("scratch: {v}")));
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
        let (o1, o2) = (
            self.out.to_str().unwrap().to_string(),
            out2.to_str().unwrap().to_string(),
        );
        for i in 0..m.count.max(s.count) as u32 {
            let (a, b) = (m.resolve(i, &o1), s.resolve(i, &o2));
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
        let status = d
            .as_ref()
            .and_then(|d| d.str_field("status").map(String::from));
        if bad.is_empty() {
            println!("OK   {step}: {} pages (scratch {status:?})", m.count);
        } else {
            self.failures += 1;
            println!("FAIL {step}: {} problems (scratch {status:?})", bad.len());
            for b in bad.iter().take(12) {
                println!("  {b}");
            }
            let dir = self.work.join(format!("fail-{step}"));
            let _ = std::fs::create_dir_all(&dir);
            for i in 0..m.count.max(s.count) as u32 {
                let _ = std::fs::write(dir.join(format!("client-{i}.txt")), m.resolve(i, &o1));
                let _ = std::fs::write(dir.join(format!("scratch-{i}.txt")), s.resolve(i, &o2));
            }
        }
    }
}

fn main() {
    let mut a = std::env::args().skip(1);
    let (mut exe, mut args, mut work, mut format, mut images, mut steps) =
        (None, vec![], None, None, vec![], None);
    let mut xetex = false;
    while let Some(x) = a.next() {
        match x.as_str() {
            "--host" => exe = a.next(),
            "--host-arg" => args.push(a.next().unwrap()),
            "--work" => work = a.next().map(PathBuf::from),
            "--format" => format = a.next(),
            "--image" => images.push(a.next().unwrap()),
            "--steps" => steps = a.next(),
            "--xetex" => xetex = true,
            _ => panic!("unknown argument {x}"),
        }
    }
    let work = work.expect("--work");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    let proj = work.join("proj");
    let out = work.join("out");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let mut names = vec![];
    for (k, f) in images.iter().enumerate() {
        let ext = Path::new(f).extension().unwrap().to_str().unwrap();
        let n = format!("fig{k}.{ext}");
        std::fs::copy(f, proj.join(&n)).unwrap();
        names.push(n);
    }
    let mut au = Audit {
        exe: exe.expect("--host"),
        args,
        work: work.clone(),
        format,
        images: names,
        xetex,
        proj: proj.clone(),
        out: out.clone(),
        next_id: 0,
        failures: 0,
        scratch: None,
        scratch_n: 0,
    };
    let doc = |v: &str, au: &Audit| document(v, &au.images, au.xetex);
    std::fs::write(proj.join("main.tex"), doc("base", &au)).unwrap();
    let host = start_host(
        &au.exe,
        &au.args,
        &work.join("host.sock"),
        &work.join("host.log"),
    );
    let mut c = Client::connect(&host.sock).unwrap();
    let mut m = Model::default();
    let all = "open,open2,same,comment,drop3,regrow,noimgA,url2,word,supersede,cancel,reconnect,shrink-cancel";
    let steps = steps.unwrap_or(all.to_string());
    let snap = work.join("snap");
    for step in steps.split(',') {
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
            _ => panic!("unknown step {step}"),
        };
        copy_dir(&out, &snap);
        let text = doc(variant, &au);
        let t = Instant::now();
        match how {
            "plain" => {
                let r = au.req(&text);
                c.compile(&r).unwrap();
                let d = run_until(&mut c, &mut m, r.id, |_| false);
                println!(
                    "     {step}: keep={:?} sent={:?} done={}",
                    m.keep,
                    m.sent,
                    d.map(|d| d.to_string())
                        .unwrap_or_default()
                        .chars()
                        .take(160)
                        .collect::<String>()
                );
            }
            "supersede" => {
                // a keystroke and, at once, another (the first superseded)
                let mid = doc("comment+noimgA+url2", &au);
                let r1 = au.req(&mid);
                let r2 = au.req(&text);
                c.compile(&r1).unwrap();
                // wait for the first page of r1, then send r2
                let mut seen = false;
                run_until(&mut c, &mut m, r1.id, |e| {
                    let p = matches!(e, Event::Page(_));
                    seen |= p;
                    p
                });
                c.compile(&r2).unwrap();
                let d = run_until(&mut c, &mut m, r2.id, |_| false);
                println!(
                    "     {step}: page before supersede={seen} keep={:?} sent={:?} done={}",
                    m.keep,
                    m.sent,
                    d.map(|d| d.to_string())
                        .unwrap_or_default()
                        .chars()
                        .take(160)
                        .collect::<String>()
                );
            }
            "cancel" => {
                // the edit cancelled after its first page; then compiled again
                let r1 = au.req(&text);
                c.compile(&r1).unwrap();
                run_until(&mut c, &mut m, r1.id, |e| matches!(e, Event::Page(_)));
                c.cancel(r1.id).unwrap();
                let d1 = run_until(&mut c, &mut m, r1.id, |_| false);
                let r2 = au.req(&text);
                c.compile(&r2).unwrap();
                let d = run_until(&mut c, &mut m, r2.id, |_| false);
                println!(
                    "     {step}: cancelled={:?} keep={:?} sent={:?} done={}",
                    d1.and_then(|d| d.str_field("status").map(String::from)),
                    m.keep,
                    m.sent,
                    d.map(|d| d.to_string())
                        .unwrap_or_default()
                        .chars()
                        .take(160)
                        .collect::<String>()
                );
            }
            "reconnect" => {
                // the connection drops after the edit's first page; a new
                // client (holding nothing) compiles the same text
                let r1 = au.req(&text);
                c.compile(&r1).unwrap();
                run_until(&mut c, &mut m, r1.id, |e| matches!(e, Event::Page(_)));
                drop(c);
                c = Client::connect(&host.sock).unwrap();
                m = Model::default();
                let r2 = au.req(&text);
                c.compile(&r2).unwrap();
                let d = run_until(&mut c, &mut m, r2.id, |_| false);
                println!(
                    "     {step}: keep={:?} sent={:?} done={}",
                    m.keep,
                    m.sent,
                    d.map(|d| d.to_string())
                        .unwrap_or_default()
                        .chars()
                        .take(160)
                        .collect::<String>()
                );
            }
            _ => unreachable!(),
        }
        let _ = t;
        // the scratch compile reads the project as the step left it
        au.check(step, &m, &snap);
        m.violations.clear();
    }
    let _ = c.bye();
    drop(host);
    println!("failures: {}", au.failures);
    std::process::exit(if au.failures > 0 { 1 } else { 0 });
}
