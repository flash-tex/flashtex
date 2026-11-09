//! Lane AUDIT-HOST: an adversarial audit of how the resident host
//! (`flashtex-host --socket`, DESIGN.md §5.1–§5.4) handles realistic project
//! edits. After every edit sequence the host's PDF, its log and every file
//! its passes write (`.aux`, `.toc`, `.lof`, `.out`, a chapter's `.aux`)
//! must equal what a fresh run gives on the same inputs: a second host,
//! started for the comparison, on a copy of the sources (the files the
//! user and the external tools wrote, never the passes' own), compiled
//! until it changes nothing. (`host_incremental.rs` compares the
//! display-list pages; this compares the files on disk.)
//!
//! The copy lives at a path as long as the original (`<base>/a` against
//! `<base>/b`), so the log's lines break at the same places; the PDF's
//! `/ID`, made from the output file's full name, is left out. At the end of
//! each case the fresh run's log is also checked against TeX Live's
//! `pdflatex` (P-T1's log half, the capacity and output-size lines left out
//! as §1.1 rules).
//!
//! Skips where there is no TeX Live.
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_display_list::client::{Client, CompileRequest, Edit, Event};
use flashtex_display_list::json::Json;
use flashtex_engine::resolver::find_texlive_bin;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

struct Host(Child, PathBuf);

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
        let _ = std::fs::remove_file(&self.1);
    }
}

fn pool() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool")
}

/// pdflatex.fmt made by this engine, once per test binary.
fn fmt_dir() -> PathBuf {
    static MADE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _once = MADE.lock().unwrap_or_else(|p| p.into_inner());
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    let fmt = DIR
        .get_or_init(|| common::fresh_dir("flashtex-audit-fmt"))
        .clone();
    if fmt.join("pdflatex.fmt").is_file() {
        return fmt;
    }
    std::fs::create_dir_all(&fmt).unwrap();
    let st = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
        .args([
            "-ini",
            "-jobname=pdflatex",
            "-progname=pdflatex",
            "-etex",
            "-translate-file=cp227.tcx",
            "pdflatex.ini",
        ])
        .current_dir(&fmt)
        .env("FLASHTEX_POOL", pool())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(st.success(), "building pdflatex.fmt failed");
    fmt
}

fn start_host(name: &str) -> Host {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let sock = PathBuf::from(format!("/tmp/fta-{}-{n}-{name}.sock", std::process::id()));
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-host"))
        .args(["--socket", sock.to_str().unwrap()])
        .env("FLASHTEX_POOL", pool())
        .env("FLASHTEX_FORMATS", fmt_dir())
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env_remove("FLASHTEX_S0_CACHE")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut out = BufReader::new(child.stdout.take().unwrap());
    loop {
        let mut line = String::new();
        if out.read_line(&mut line).unwrap() == 0 {
            panic!("the host exited before listening");
        }
        if line.contains("listening") {
            break;
        }
    }
    std::thread::spawn(move || for _ in out.lines().map_while(Result::ok) {});
    Host(child, sock)
}

/// Read events up to `n` `DONE`s; the `DONE`s.
fn drain(c: &mut Client, n: usize) -> Vec<Json> {
    let mut dones = vec![];
    while dones.len() < n {
        match c.next_event().unwrap().expect("host closed the connection") {
            Event::Done(d) => dones.push(d),
            Event::Error(e) => panic!("host error: {e}"),
            _ => {}
        }
    }
    dones
}

/// What the passes and the tools write; never copied to the fresh run.
const WRITTEN: &[&str] = &[
    "pdf", "log", "aux", "toc", "lof", "lot", "out", "fls", "blg", "ilg", "synctex",
];

fn written(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, e)| WRITTEN.contains(&e))
}

/// Copy the sources under `from` to `to`: every file but the passes' own
/// (`WRITTEN`); `skip` names a subdirectory left out (the output one).
fn copy_sources(from: &Path, to: &Path, skip: Option<&str>) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        let t = to.join(&n);
        if e.file_type().unwrap().is_dir() {
            if Some(n.as_str()) != skip {
                copy_sources(&e.path(), &t, None);
            }
        } else if !written(&n) {
            std::fs::copy(e.path(), t).unwrap();
        }
    }
}

/// Every file under `d`, by its path there.
fn files(d: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(d: &Path, pre: &str, m: &mut BTreeMap<String, Vec<u8>>) {
        let Ok(rd) = std::fs::read_dir(d) else {
            return;
        };
        for e in rd.flatten() {
            let n = format!("{pre}{}", e.file_name().to_string_lossy());
            if e.file_type().unwrap().is_dir() {
                walk(&e.path(), &format!("{n}/"), m);
            } else {
                m.insert(n, std::fs::read(e.path()).unwrap());
            }
        }
    }
    let mut m = BTreeMap::new();
    walk(d, "", &mut m);
    m
}

fn replace_all(b: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    if from.is_empty() {
        return b.to_vec();
    }
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i..].starts_with(from) {
            out.extend_from_slice(to);
            i += from.len();
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

/// A PDF without its `/ID [<…> <…>]` (made from the file's full name).
fn without_id(b: &[u8]) -> Vec<u8> {
    let mut out = b.to_vec();
    while let Some(at) = out.windows(5).position(|w| w == b"/ID [") {
        let end = out[at..].iter().position(|&c| c == b']').unwrap() + at + 1;
        out.drain(at..end);
    }
    out
}

/// The first line where two texts differ, for the message.
fn first_difference(a: &[u8], b: &[u8]) -> String {
    let (a, b) = (String::from_utf8_lossy(a), String::from_utf8_lossy(b));
    let (la, lb): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
    for k in 0..la.len().max(lb.len()) {
        if la.get(k) != lb.get(k) {
            return format!(
                "line {}: host {:?} / fresh {:?}",
                k + 1,
                la.get(k),
                lb.get(k)
            );
        }
    }
    format!("same lines, {} vs {} bytes", a.len(), b.len())
}

/// Lorem prose: `n` paragraphs of about 90 words, deterministic in `seed`.
fn prose(seed: usize, n: usize) -> String {
    const W: &[&str] = &[
        "lorem",
        "ipsum",
        "dolor",
        "sit",
        "amet",
        "consectetur",
        "adipiscing",
        "elit",
        "sed",
        "do",
        "eiusmod",
        "tempor",
        "incididunt",
        "ut",
        "labore",
        "et",
        "dolore",
        "magna",
        "aliqua",
        "enim",
        "minim",
        "veniam",
        "quis",
        "nostrud",
        "exercitation",
        "ullamco",
        "laboris",
        "nisi",
        "aliquip",
        "commodo",
        "consequat",
    ];
    let mut s = String::new();
    let mut x = seed as u64 * 2654435761 + 1;
    for p in 0..n {
        let mut line = vec![];
        for _ in 0..90 {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            line.push(W[(x >> 33) as usize % W.len()]);
        }
        s.push_str(&format!("Paragraph {seed}.{p} {}.\n\n", line.join(" ")));
    }
    s
}

/// One case: a project at `<base>/a`, a resident host holding it.
struct Audit {
    name: String,
    base: PathBuf,
    a: PathBuf,
    /// The output directory's name under the project (`None`: the project).
    out_sub: Option<&'static str>,
    main: String,
    jobname: Option<String>,
    includeonly: Option<Vec<String>>,
    _host: Host,
    c: Client,
    id: i64,
}

impl Audit {
    fn new(name: &str, files: &[(&str, &[u8])], out_sub: Option<&'static str>) -> Option<Audit> {
        if find_texlive_bin().is_none() {
            common::no_texlive();
            return None;
        }
        let base = std::fs::canonicalize(common::fresh_dir(&format!("fta-{name}"))).unwrap();
        let a = base.join("a");
        std::fs::create_dir_all(&a).unwrap();
        if let Some(o) = out_sub {
            std::fs::create_dir_all(a.join(o)).unwrap();
        }
        for (p, b) in files {
            let f = a.join(p);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, b).unwrap();
        }
        let host = start_host(&name[..name.len().min(8)]);
        let c = Client::connect(&host.1).unwrap();
        Some(Audit {
            name: name.into(),
            base,
            a,
            out_sub,
            main: "main.tex".into(),
            jobname: None,
            includeonly: None,
            _host: host,
            c,
            id: 0,
        })
    }

    fn out_dir(&self, root: &Path) -> PathBuf {
        match self.out_sub {
            Some(o) => root.join(o),
            None => root.to_path_buf(),
        }
    }

    fn request(&mut self, root: &Path, edits: Vec<Edit>) -> CompileRequest {
        self.id += 1;
        let mut r = CompileRequest::new(self.id, root.to_str().unwrap(), &self.main);
        r.output_dir = Some(self.out_dir(root).to_str().unwrap().into());
        r.incremental = true;
        r.jobname = self.jobname.clone();
        r.includeonly = self.includeonly.clone();
        r.edits = edits;
        r
    }

    /// One compile of the project with `edits`; its `DONE`.
    fn compile(&mut self, edits: Vec<Edit>) -> Json {
        let a = self.a.clone();
        let r = self.request(&a, edits);
        self.c.compile(&r).unwrap();
        let d = drain(&mut self.c, 1).pop().unwrap();
        eprintln!("{}: compile {}: {d}", self.name, self.id);
        d
    }

    /// Compiles sent back to back, as typing sends them; the last `DONE`.
    fn burst(&mut self, edits: Vec<Vec<Edit>>) -> Vec<Json> {
        let a = self.a.clone();
        let n = edits.len();
        for e in edits {
            let r = self.request(&a, e);
            self.c.compile(&r).unwrap();
        }
        let d = drain(&mut self.c, n);
        for x in &d {
            eprintln!("{}: burst: {x}", self.name);
        }
        d
    }

    /// The splice replacing the first `find` of `path` (as it is on disk)
    /// by `with`.
    fn splice(&self, path: &str, find: &str, with: &str) -> Edit {
        let t = std::fs::read(self.a.join(path)).unwrap();
        let at = t
            .windows(find.len())
            .position(|w| w == find.as_bytes())
            .unwrap_or_else(|| panic!("{}: {find:?} not in {path}", self.name));
        Edit {
            path: path.into(),
            offset: at as u64,
            delete: find.len() as u64,
            insert: with.into(),
        }
    }

    /// Another program writes `path` (replacing the first `find` by `with`).
    fn rewrite(&self, path: &str, find: &str, with: &str) {
        let f = self.a.join(path);
        let t = String::from_utf8(std::fs::read(&f).unwrap()).unwrap();
        assert!(t.contains(find), "{}: {find:?} not in {path}", self.name);
        std::fs::write(&f, t.replacen(find, with, 1)).unwrap();
    }

    /// Run a TeX Live tool (`bibtex main`) in the output directory.
    fn tool(&self, argv: &[&str]) {
        let bin = find_texlive_bin().expect("TeX Live");
        let out = self.out_dir(&self.a);
        let st = Command::new(bin.join(argv[0]))
            .args(&argv[1..])
            .current_dir(&out)
            .env("BIBINPUTS", format!("{}:", self.a.display()))
            .env("SOURCE_DATE_EPOCH", "0")
            .env("FORCE_SOURCE_DATE", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(st.code().is_some_and(|c| c <= 1), "{argv:?}: {st}");
    }

    /// The fresh run: the sources copied to `<base>/b`, a new host,
    /// compiled until it changes nothing. Its files, by path.
    fn fresh(&mut self) -> BTreeMap<String, Vec<u8>> {
        let b = self.base.join("b");
        let _ = std::fs::remove_dir_all(&b);
        copy_sources(&self.a, &b, self.out_sub);
        if let Some(o) = self.out_sub {
            std::fs::create_dir_all(b.join(o)).unwrap();
            // the tools' own outputs in the output directory are sources
            // of the fresh run (bibtex wrote them; pdflatex reads them)
            for e in std::fs::read_dir(self.a.join(o)).unwrap().flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if e.file_type().unwrap().is_file() && !written(&n) {
                    std::fs::copy(e.path(), b.join(o).join(&n)).unwrap();
                }
            }
        }
        let host = start_host("fresh");
        let mut c = Client::connect(&host.1).unwrap();
        let mut settled = false;
        for k in 0..8 {
            let r = self.request(&b, vec![]);
            c.compile(&r).unwrap();
            let d = drain(&mut c, 1).pop().unwrap();
            if k == 0 {
                assert_eq!(
                    d.str_field("mode"),
                    Some("cold"),
                    "{}: fresh: {d}",
                    self.name
                );
            }
            if d.str_field("mode") == Some("unchanged") {
                settled = true;
                break;
            }
        }
        assert!(settled, "{}: the fresh run never settled", self.name);
        let _ = c.bye();
        drop(host);
        files(&b)
    }

    /// The host's files equal the fresh run's (`what` names the step).
    /// Err: the differences.
    fn try_check(&mut self) -> Result<(), String> {
        let fresh = self.fresh();
        let a = self.a.to_string_lossy().into_owned();
        let b = self.base.join("b").to_string_lossy().into_owned();
        let mut ours = files(&self.a);
        for (n, v) in ours.iter_mut() {
            *v = replace_all(v, a.as_bytes(), b.as_bytes());
            if n.ends_with(".pdf") {
                *v = without_id(v);
            }
        }
        let fresh: BTreeMap<String, Vec<u8>> = fresh
            .into_iter()
            .map(|(n, v)| {
                let v = if n.ends_with(".pdf") {
                    without_id(&v)
                } else {
                    v
                };
                (n, v)
            })
            .collect();
        let mut bad = vec![];
        for n in ours.keys().chain(fresh.keys()) {
            if bad.iter().any(|(m, _): &(String, String)| m == n) {
                continue;
            }
            match (ours.get(n), fresh.get(n)) {
                (Some(x), Some(y)) if x == y => {}
                (Some(x), Some(y)) => bad.push((n.clone(), first_difference(x, y))),
                (Some(_), None) => bad.push((n.clone(), "only the host's".into())),
                (None, _) => bad.push((n.clone(), "only the fresh run's".into())),
            }
        }
        if bad.is_empty() {
            Ok(())
        } else {
            Err(format!("{bad:#?}"))
        }
    }

    fn check(&mut self, what: &str) {
        if let Err(e) = self.try_check() {
            // classify: does one more compile (no edit) put it right?
            let d = self.compile(vec![]);
            let again = self.try_check();
            panic!(
                "{}: {what}: the host's files differ from a fresh run's: {e}\n\
                 one more compile ({}): {}",
                self.name,
                d.str_field("mode").unwrap_or("?"),
                match again {
                    Ok(()) => "then equal (the host stopped short of the fixed point)".into(),
                    Err(e2) => format!("still differ: {e2}"),
                }
            );
        }
        eprintln!("{}: {what}: equal", self.name);
    }

    /// The fresh run's log (in `<base>/b`, from the last `check`) against
    /// TeX Live's pdflatex run to a fixed point on the same sources in
    /// `<base>/c`.
    fn oracle(&mut self) {
        let bin = find_texlive_bin().expect("TeX Live");
        let c = self.base.join("c");
        let _ = std::fs::remove_dir_all(&c);
        copy_sources(&self.a, &c, self.out_sub);
        let job = self
            .jobname
            .clone()
            .unwrap_or_else(|| self.main.trim_end_matches(".tex").into());
        let out = self.out_dir(&c);
        std::fs::create_dir_all(&out).unwrap();
        if let Some(o) = self.out_sub {
            for e in std::fs::read_dir(self.a.join(o)).unwrap().flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if e.file_type().unwrap().is_file() && !written(&n) {
                    std::fs::copy(e.path(), out.join(&n)).unwrap();
                }
            }
        }
        let first = match &self.includeonly {
            Some(v) => format!(
                "\\AtBeginDocument{{\\includeonly{{{}}}}}\\input {}",
                v.join(","),
                self.main
            ),
            None => self.main.clone(),
        };
        let mut last = None;
        for _ in 0..6 {
            let st = Command::new(bin.join("pdflatex"))
                .args([
                    "-interaction=nonstopmode",
                    &format!("-jobname={job}"),
                    &format!("-output-directory={}", out.display()),
                    &first,
                ])
                .current_dir(&c)
                .env("SOURCE_DATE_EPOCH", "0")
                .env("FORCE_SOURCE_DATE", "1")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();
            let _ = st;
            let now: BTreeMap<String, Vec<u8>> = files(&out)
                .into_iter()
                .filter(|(n, _)| n.ends_with(".aux") || n.ends_with(".toc") || n.ends_with(".out"))
                .collect();
            if last.as_ref() == Some(&now) {
                break;
            }
            last = Some(now);
        }
        let norm = |b: &[u8], dir: &Path| -> Vec<String> {
            let b = replace_all(b, dir.to_string_lossy().as_bytes(), b"<dir>");
            let t = String::from_utf8_lossy(&b).into_owned();
            let mut v: Vec<String> = vec![];
            let mut skip = false;
            for (k, l) in t.lines().enumerate() {
                if k == 0 {
                    continue; // the banner: the format's date
                }
                if l.starts_with("Here is how much of TeX's memory you used:") {
                    skip = true;
                }
                if skip {
                    if l.starts_with("Output written on") {
                        skip = false;
                    } else {
                        continue;
                    }
                }
                if let Some(i) = l.find(" pages, ") {
                    v.push(l[..i].to_string());
                    continue;
                }
                v.push(l.to_string());
            }
            v
        };
        let b = self.base.join("b");
        let ob = std::fs::read(self.out_dir(&b).join(format!("{job}.log"))).unwrap();
        let oc = std::fs::read(out.join(format!("{job}.log"))).unwrap();
        let (x, y) = (norm(&ob, &b), norm(&oc, &c));
        if x != y {
            let k = (0..x.len().max(y.len()))
                .find(|&k| x.get(k) != y.get(k))
                .unwrap();
            panic!(
                "{}: the fresh run's log differs from pdflatex's at line {}: ours {:?} / pdflatex {:?}",
                self.name,
                k + 2,
                x.get(k),
                y.get(k)
            );
        }
        eprintln!("{}: oracle: log equal to pdflatex's", self.name);
    }
}

impl Drop for Audit {
    fn drop(&mut self) {
        let _ = self.c.bye();
        if !std::thread::panicking() {
            let _ = std::fs::remove_dir_all(&self.base);
        } else {
            eprintln!("{}: kept {}", self.name, self.base.display());
        }
    }
}

const PRE: &str = "\\documentclass{article}\n\\usepackage[T1]{fontenc}\n";

// ---------------------------------------------------------------- cases

/// An edit in an `\input` file: in the file read at the restart point, in
/// one opened only later (not open at the restart point), from the editor
/// and from another program, and one that reflows the rest.
#[test]
fn audit_input_files() {
    let main = format!(
        "{PRE}\\begin{{document}}\n\\section{{One}}\n\\input{{one}}\n\\section{{Two}}\n\
         \\input{{two}}\n\\section{{Three}}\n\\input{{three}}\n\\end{{document}}\n"
    );
    let (one, two, three) = (prose(1, 9), prose(2, 9), prose(3, 9));
    let Some(mut t) = Audit::new(
        "input",
        &[
            ("main.tex", main.as_bytes()),
            ("one.tex", one.as_bytes()),
            ("two.tex", two.as_bytes()),
            ("three.tex", three.as_bytes()),
        ],
        None,
    ) else {
        return;
    };
    t.compile(vec![]);
    t.check("open");
    let e = t.splice("two.tex", "Paragraph 2.4 ", "Paragraph 2.4 extra ");
    t.compile(vec![e]);
    t.check("an editor edit in two.tex");
    t.rewrite("three.tex", "Paragraph 3.6 ", "Paragraph 3.6 outside ");
    t.compile(vec![]);
    t.check("another program's edit in three.tex");
    let e = t.splice("one.tex", "Paragraph 1.2 ", "Inserted. \n\nParagraph 1.2 ");
    t.compile(vec![e]);
    t.check("a paragraph inserted in one.tex");
    t.oracle();
}

/// `\include` with `\includeonly` changing: from the request's
/// `includeonly`, and from a preamble line, with cross-chapter references.
#[test]
fn audit_includeonly() {
    let main = format!(
        "{PRE}\\begin{{document}}\n\\include{{ch1}}\n\\include{{ch2}}\n\\include{{ch3}}\n\
         \\end{{document}}\n"
    );
    let ch = |n: usize| {
        format!(
            "\\section{{Chapter {n}}}\\label{{s{n}}}\nSee section~\\ref{{s{}}} on page~\\pageref{{s{}}}.\n\n{}",
            n % 3 + 1,
            n % 3 + 1,
            prose(n, 5)
        )
    };
    let (c1, c2, c3) = (ch(1), ch(2), ch(3));
    let Some(mut t) = Audit::new(
        "include",
        &[
            ("main.tex", main.as_bytes()),
            ("ch1.tex", c1.as_bytes()),
            ("ch2.tex", c2.as_bytes()),
            ("ch3.tex", c3.as_bytes()),
        ],
        None,
    ) else {
        return;
    };
    t.compile(vec![]);
    t.check("open");
    t.includeonly = Some(vec!["ch2".into()]);
    t.compile(vec![]);
    t.check("includeonly ch2");
    let e = t.splice("ch2.tex", "Paragraph 2.1 ", "Paragraph 2.1 more words ");
    t.compile(vec![e]);
    t.check("an edit in ch2 under includeonly");
    t.includeonly = Some(vec!["ch1".into(), "ch3".into()]);
    t.compile(vec![]);
    t.check("includeonly ch1,ch3");
    t.includeonly = None;
    t.compile(vec![]);
    t.check("includeonly dropped");
    let e = t.splice(
        "main.tex",
        "\\begin{document}",
        "\\includeonly{ch3}\n\\begin{document}",
    );
    t.compile(vec![e]);
    t.check("\\includeonly{ch3} added to the preamble");
    let e = t.splice("main.tex", "\\includeonly{ch3}\n", "");
    t.compile(vec![e]);
    t.check("\\includeonly removed again");
    t.oracle();
}

/// A file created between compiles, then changed, then deleted.
#[test]
fn audit_file_created_and_deleted() {
    let main = format!(
        "{PRE}\\begin{{document}}\n{}\\IfFileExists{{extra.tex}}{{\\input{{extra}}}}{{No extra.}}\n\n{}\
         \\InputIfFileExists{{late.tex}}{{}}{{No late.}}\n\\end{{document}}\n",
        prose(1, 6),
        prose(2, 6)
    );
    let Some(mut t) = Audit::new("created", &[("main.tex", main.as_bytes())], None) else {
        return;
    };
    t.compile(vec![]);
    t.check("open");
    std::fs::write(t.a.join("extra.tex"), "An extra file \\emph{here}.\n").unwrap();
    t.compile(vec![]);
    t.check("extra.tex created");
    std::fs::write(t.a.join("late.tex"), prose(9, 2)).unwrap();
    t.compile(vec![]);
    t.check("late.tex created");
    t.rewrite("extra.tex", "here", "there, now longer");
    t.compile(vec![]);
    t.check("extra.tex changed");
    std::fs::remove_file(t.a.join("extra.tex")).unwrap();
    t.compile(vec![]);
    t.check("extra.tex deleted");
    std::fs::remove_file(t.a.join("late.tex")).unwrap();
    t.compile(vec![]);
    t.check("late.tex deleted");
    t.oracle();
}

const BIB: &str =
    "@book{knuth,author={Donald Knuth},title={The {\\TeX}book},year=1984,publisher={AW}}\n\
@article{lamport,author={Leslie Lamport},title={LaTeX},journal={J},year=1986}\n";

/// bibtex between compiles: a `.bib` edited, a citation added, the `.bbl`
/// rewritten by the tool; the `.aux` files deleted by a clean.
#[test]
fn audit_bibtex_between_runs() {
    let main = format!(
        "{PRE}\\begin{{document}}\n{}As \\cite{{knuth}} says.\n\n{}\\bibliographystyle{{plain}}\n\
         \\bibliography{{refs}}\n\\end{{document}}\n",
        prose(1, 4),
        prose(2, 4)
    );
    for out in [None, Some("out")] {
        let Some(mut t) = Audit::new(
            if out.is_some() { "bib-out" } else { "bib" },
            &[("main.tex", main.as_bytes()), ("refs.bib", BIB.as_bytes())],
            out,
        ) else {
            return;
        };
        t.compile(vec![]);
        t.tool(&["bibtex", "main"]);
        t.compile(vec![]);
        t.check("bibtex after the first compile");
        let e = t.splice("main.tex", "As \\cite{knuth}", "As \\cite{knuth,lamport}");
        t.compile(vec![e]);
        t.check("a citation added, before bibtex");
        t.tool(&["bibtex", "main"]);
        t.compile(vec![]);
        t.check("bibtex again");
        t.rewrite("refs.bib", "{Donald Knuth}", "{Donald E. Knuth}");
        t.tool(&["bibtex", "main"]);
        t.compile(vec![]);
        t.check("refs.bib changed, bibtex again");
        // the .bbl rewritten by hand to the same length (another tool)
        let bbl = t.out_dir(&t.a).join("main.bbl");
        let b = std::fs::read_to_string(&bbl).unwrap();
        std::fs::write(&bbl, b.replacen("Lamport", "Lampert", 1)).unwrap();
        t.compile(vec![]);
        t.check("the .bbl rewritten in place");
        // a clean: the .aux deleted
        std::fs::remove_file(t.out_dir(&t.a).join("main.aux")).unwrap();
        t.compile(vec![]);
        t.check("the .aux deleted");
        t.oracle();
    }
}

/// LaTeX's rerun cycle: labels whose page numbers an edit changes, and a
/// table of contents that grows a line.
#[test]
fn audit_rerun_cycle() {
    let mut body = String::new();
    for s in 1..=6 {
        body.push_str(&format!(
            "\\section{{Part {s}}}\\label{{p{s}}}\nPart {s} is on page~\\pageref{{p{s}}}; part {} on page~\\pageref{{p{}}}.\n\n{}",
            7 - s,
            7 - s,
            prose(s, 3)
        ));
    }
    let main = format!("{PRE}\\begin{{document}}\n\\tableofcontents\n{body}\\end{{document}}\n");
    let Some(mut t) = Audit::new("rerun", &[("main.tex", main.as_bytes())], None) else {
        return;
    };
    t.compile(vec![]);
    t.check("open");
    // a paragraph before part 2: the later labels move a page
    let e = t.splice(
        "main.tex",
        "\\section{Part 2}",
        &format!("{}\\section{{Part 2}}", prose(7, 3)),
    );
    t.compile(vec![e]);
    t.check("paragraphs before part 2");
    let e = t.splice(
        "main.tex",
        "\\section{Part 4}",
        "\\section{Part 3b}\\label{p3b}See \\pageref{p3b}.\n\n\\section{Part 4}",
    );
    t.compile(vec![e]);
    t.check("a section added (the toc grows)");
    let e = t.splice(
        "main.tex",
        "\\section{Part 3b}\\label{p3b}See \\pageref{p3b}.\n\n",
        "",
    );
    t.compile(vec![e]);
    t.check("the section removed again");
    t.oracle();
}

/// hyperref's `.out`, `.toc` and `.lof`, under another `\jobname` too.
#[test]
fn audit_jobname_files() {
    let mut body = String::new();
    for s in 1..=4 {
        body.push_str(&format!(
            "\\section{{Section {s}}}\n{}\\begin{{figure}}[h]\\centering\\rule{{2cm}}{{1cm}}\\caption{{Figure {s}}}\\end{{figure}}\n\n",
            prose(s, 3)
        ));
    }
    let main = format!(
        "{PRE}\\usepackage{{hyperref}}\n\\begin{{document}}\n\\tableofcontents\n\\listoffigures\n{body}\\end{{document}}\n"
    );
    for job in [None, Some("paper")] {
        let Some(mut t) = Audit::new(
            if job.is_some() { "job-paper" } else { "job" },
            &[("main.tex", main.as_bytes())],
            Some("out"),
        ) else {
            return;
        };
        t.jobname = job.map(String::from);
        t.compile(vec![]);
        t.check("open");
        let e = t.splice(
            "main.tex",
            "\\section{Section 3}",
            "\\section{Section three, renamed}",
        );
        t.compile(vec![e]);
        t.check("a section title changed (.toc, .out)");
        let e = t.splice(
            "main.tex",
            "\\caption{Figure 2}",
            "\\caption{Figure two, renamed}",
        );
        t.compile(vec![e]);
        t.check("a caption changed (.lof)");
        let e = t.splice(
            "main.tex",
            "\\section{Section 4}",
            "\\section{Section 3b}\\begin{figure}[h]\\rule{1cm}{1cm}\\caption{New}\\end{figure}\n\\section{Section 4}",
        );
        t.compile(vec![e]);
        t.check("a section and a figure added");
        t.oracle();
    }
}

/// A `w`×`h` 8-bit gray PNG of one `shade`, its deflate stream stored
/// (so two shades give files of the same length).
fn gray_png(w: u32, h: u32, shade: u8) -> Vec<u8> {
    fn crc(b: &[u8]) -> u32 {
        let mut c = 0xffff_ffffu32;
        for &x in b {
            c ^= x as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xedb8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
        }
        !c
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc(&body).to_be_bytes());
    }
    let mut raw = vec![];
    for _ in 0..h {
        raw.push(0); // filter: none
        raw.extend(std::iter::repeat_n(shade, w as usize));
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    let mut z = vec![0x78, 0x01, 0x01];
    z.extend_from_slice(&(raw.len() as u16).to_le_bytes());
    z.extend_from_slice(&(!(raw.len() as u16)).to_le_bytes());
    z.extend_from_slice(&raw);
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = vec![];
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 0, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

fn image(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/images")
            .join(name),
    )
    .unwrap()
}

/// A graphics file replaced in place under the same name: other content
/// and size (PNG, JPEG, PDF), and the same size with other pixels.
#[test]
fn audit_graphics_replaced() {
    let main = format!(
        "{PRE}\\usepackage{{graphicx}}\n\\begin{{document}}\n{}\\includegraphics{{pic}}\n\n{}\
         \\includegraphics[width=3cm]{{fig.pdf}}\n\n\\includegraphics{{photo.jpg}}\n\\end{{document}}\n",
        prose(1, 5),
        prose(2, 5)
    );
    let Some(mut t) = Audit::new(
        "graphics",
        &[
            ("main.tex", main.as_bytes()),
            ("pic.png", &image("png-rgb8.png")),
            ("fig.pdf", &image("pdf-hand.pdf")),
            ("photo.jpg", &image("jpg-rgb.jpg")),
        ],
        None,
    ) else {
        return;
    };
    t.compile(vec![]);
    t.check("open");
    std::fs::write(t.a.join("pic.png"), image("png-pal8.png")).unwrap();
    t.compile(vec![]);
    t.check("pic.png replaced");
    std::fs::write(t.a.join("fig.pdf"), image("pdf-17.pdf")).unwrap();
    t.compile(vec![]);
    t.check("fig.pdf replaced");
    std::fs::write(t.a.join("photo.jpg"), image("jpg-gray.jpg")).unwrap();
    t.compile(vec![]);
    t.check("photo.jpg replaced");
    // back to the first image, then an 8x6 gray one
    let p = t.a.join("pic.png");
    std::fs::write(&p, image("png-rgb8.png")).unwrap();
    t.compile(vec![]);
    t.check("pic.png back");
    std::fs::write(&p, gray_png(8, 6, 200)).unwrap();
    t.compile(vec![]);
    t.check("pic.png an 8x6 gray image");
    // other pixels, the same length, the modification time put back
    let mt = std::fs::metadata(&p).unwrap().modified().unwrap();
    let px = gray_png(8, 6, 40);
    assert_eq!(px.len(), gray_png(8, 6, 200).len());
    std::fs::write(&p, &px).unwrap();
    std::fs::File::options()
        .write(true)
        .open(&p)
        .unwrap()
        .set_modified(mt)
        .unwrap();
    t.compile(vec![]);
    t.check("pic.png, same size, other bytes, time put back");
    t.oracle();
}

/// A file's mtime changes without its content (a checkout, `touch`).
#[test]
fn audit_mtime_only() {
    let main = format!(
        "{PRE}\\begin{{document}}\n{}\\input{{part}}\n{}\\end{{document}}\n",
        prose(1, 6),
        prose(2, 6)
    );
    let part = prose(3, 6);
    let Some(mut t) = Audit::new(
        "mtime",
        &[("main.tex", main.as_bytes()), ("part.tex", part.as_bytes())],
        None,
    ) else {
        return;
    };
    t.compile(vec![]);
    t.check("open");
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
    for f in ["part.tex", "main.tex"] {
        std::fs::File::options()
            .write(true)
            .open(t.a.join(f))
            .unwrap()
            .set_modified(later)
            .unwrap();
    }
    t.compile(vec![]);
    t.check("both files touched");
    // the same bytes written again (an editor's save)
    std::fs::write(t.a.join("part.tex"), part.as_bytes()).unwrap();
    t.compile(vec![]);
    t.check("part.tex saved unchanged");
    // the aux touched
    std::fs::File::options()
        .write(true)
        .open(t.a.join("main.aux"))
        .unwrap()
        .set_modified(later)
        .unwrap();
    t.compile(vec![]);
    t.check("main.aux touched");
    t.oracle();
}

/// An edit undone back to the original text: in the body, in an input
/// file and in the preamble; by splices and by another program.
#[test]
fn audit_undo() {
    let main = format!(
        "{PRE}\\newcommand\\thing{{widget}}\n\\begin{{document}}\n{}The \\thing.\n\n\\input{{part}}\n{}\\end{{document}}\n",
        prose(1, 6),
        prose(2, 6)
    );
    let part = prose(3, 6);
    let Some(mut t) = Audit::new(
        "undo",
        &[("main.tex", main.as_bytes()), ("part.tex", part.as_bytes())],
        None,
    ) else {
        return;
    };
    t.compile(vec![]);
    t.check("open");
    let e = t.splice(
        "main.tex",
        "Paragraph 2.2 ",
        "Paragraph 2.2 typed then undone ",
    );
    t.compile(vec![e]);
    let e = t.splice(
        "main.tex",
        "Paragraph 2.2 typed then undone ",
        "Paragraph 2.2 ",
    );
    t.compile(vec![e]);
    t.check("a body edit undone");
    let e = t.splice("main.tex", "{widget}", "{gadget}");
    t.compile(vec![e]);
    let e = t.splice("main.tex", "{gadget}", "{widget}");
    t.compile(vec![e]);
    t.check("a preamble edit undone");
    t.rewrite("part.tex", "Paragraph 3.3 ", "Paragraph 3.3 more\n\n");
    t.compile(vec![]);
    std::fs::write(t.a.join("part.tex"), part.as_bytes()).unwrap();
    t.compile(vec![]);
    t.check("an outside edit undone");
    // undone within one burst (the edit never compiled to the end)
    let e1 = t.splice("main.tex", "Paragraph 1.4 ", "Paragraph 1.4 x\n\n");
    let e2 = Edit {
        delete: e1.insert.len() as u64,
        insert: "Paragraph 1.4 ".into(),
        ..e1.clone()
    };
    t.burst(vec![vec![e1], vec![e2]]);
    t.check("an edit undone in the next keystroke");
    t.oracle();
}

/// Edits arriving while a compile runs (it is cancelled or superseded).
#[test]
fn audit_rapid_edits() {
    let main = format!(
        "{PRE}\\begin{{document}}\n\\tableofcontents\n\\section{{A}}\\label{{a}}\n{}\\section{{B}}\\label{{b}}\nSee page~\\pageref{{a}}.\n\n{}\\end{{document}}\n",
        prose(1, 20),
        prose(2, 20)
    );
    let Some(mut t) = Audit::new("rapid", &[("main.tex", main.as_bytes())], None) else {
        return;
    };
    t.compile(vec![]);
    t.check("open");
    for round in 0..3 {
        let e1 = t.splice(
            "main.tex",
            "Paragraph 1.3 ",
            &format!("{}Paragraph 1.3 ", prose(10 + round, 2)),
        );
        // the second splice is against the text after the first
        let shift = e1.insert.len() as i64 - e1.delete as i64;
        let e2 = t.splice("main.tex", "Paragraph 2.5 ", "Paragraph 2.5 q ");
        let e2 = Edit {
            offset: (e2.offset as i64 + shift) as u64,
            ..e2
        };
        let e3 = Edit {
            offset: e2.offset,
            delete: 0,
            insert: "z".into(),
            path: "main.tex".into(),
        };
        t.burst(vec![vec![e1], vec![e2], vec![e3]]);
        t.check(&format!("a burst of three edits, round {round}"));
    }
    // a preamble edit right behind a body edit
    let e1 = t.splice("main.tex", "Paragraph 2.9 ", "Paragraph 2.9 w ");
    let e2 = t.splice(
        "main.tex",
        "{fontenc}\n",
        "{fontenc}\n\\usepackage{parskip}\n",
    );
    t.burst(vec![vec![e1], vec![e2]]);
    t.check("a preamble edit behind a body edit");
    t.oracle();
}

/// Edits before the first restart point and in the preamble.
#[test]
fn audit_preamble_and_document_start() {
    let main = format!(
        "{PRE}\\title{{A title}}\\author{{Someone}}\\date{{}}\n\\newcommand\\word{{alpha}}\n\
         \\begin{{document}}\n\\maketitle\nFirst \\word.\n\n{}\\end{{document}}\n",
        prose(1, 12)
    );
    let Some(mut t) = Audit::new("preamble", &[("main.tex", main.as_bytes())], None) else {
        return;
    };
    t.compile(vec![]);
    t.check("open");
    let e = t.splice("main.tex", "First ", "Very first ");
    t.compile(vec![e]);
    t.check("an edit on the first body line");
    let e = t.splice("main.tex", "{A title}", "{Another, longer title}");
    t.compile(vec![e]);
    t.check("the title changed in the preamble");
    let e = t.splice("main.tex", "{alpha}", "{beta}");
    t.compile(vec![e]);
    t.check("a macro the body uses changed in the preamble");
    let e = t.splice(
        "main.tex",
        "\\begin{document}",
        "\\usepackage{lmodern}\n\\begin{document}",
    );
    t.compile(vec![e]);
    t.check("a package added to the preamble");
    let e = t.splice(
        "main.tex",
        "\\documentclass{article}",
        "\\documentclass[twocolumn]{article}",
    );
    t.compile(vec![e]);
    t.check("the class option changed");
    t.oracle();
}
