//! The unified engine host end to end, through the MIT client
//! (`flashtex_display_list::client`): `flashtex-host --socket` keeps one
//! resident, incremental engine (DESIGN.md §5.1–§5.4) and streams
//! `display-list-v3` pages (§6.1).
//!
//! For each document: open it (compile until the `.aux` is stable), holding
//! every page as an incremental client does (`"incremental": true`); then,
//! for each edit, send it in a `COMPILE` (`edits`, with the edited page as
//! the `viewport`) and receive the edited page first, the later pages
//! re-typeset or kept, `PAGES` saying which are current, and `DONE`. After
//! every edit, **every page the client holds must equal the page a
//! from-scratch compile of the same inputs gives** (a second host, which has
//! never seen the document, on a copy of the directory as the compile found
//! it): the same content hash, items, links and destinations, with each
//! source span resolved to its file (relative to the project) and line.
//!
//! The edits: a word replaced (no reflow; the run converges), a comment
//! line inserted (the output is the same but every later line moves: pages
//! kept from before the edit must follow their lines, `SOURCES`), and a
//! paragraph inserted (the rest of the document reflows and gains a page).
//! Documents: a generated 12-page article, and the hyperref-toc parity
//! fixture (links, destinations, a table of contents).
//!
//! Skips where there is no TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_display_list::client::{Client, CompileRequest, Edit, Event};
use flashtex_display_list::json::Json;
use flashtex_display_list::page::{Item, Page};
use flashtex_engine::resolver::find_texlive_bin;
use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Host(Child, PathBuf);

impl Drop for Host {
    fn drop(&mut self) {
        // (cargo-llvm-cov: `SIGTERM` first, on which a coverage build
        // writes its profile, `host::crash`; a kill would lose it)
        #[cfg(coverage)]
        {
            let _ = Command::new("kill")
                .args(["-TERM", &self.0.id().to_string()])
                .status();
            let t = Instant::now();
            while t.elapsed() < Duration::from_secs(20) {
                if matches!(self.0.try_wait(), Ok(Some(_))) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
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
    // a directory of this process's own (common::fresh_dir), made once
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    let fmt = DIR
        .get_or_init(|| common::fresh_dir("flashtex-host-fmt"))
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
    start_host_env(name, &[])
}

fn start_host_env(name: &str, env: &[(&str, &str)]) -> Host {
    start_host_lines(name, env, None)
}

/// [`start_host_env`], the host's later stdout lines sent to `lines` (its
/// `flashtex-host: {...}` notes: `saved_s1`, `no_s1`).
fn start_host_lines(
    name: &str,
    env: &[(&str, &str)],
    lines: Option<std::sync::mpsc::Sender<String>>,
) -> Host {
    // Short: a Unix socket path must fit in sockaddr_un (104 bytes on macOS).
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let sock = PathBuf::from(format!("/tmp/fth-{}-{n}-{name}.sock", std::process::id()));
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-host"))
        .args(["--socket", sock.to_str().unwrap()])
        .env("FLASHTEX_POOL", pool())
        .env("FLASHTEX_FORMATS", fmt_dir())
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env_remove("FLASHTEX_S0_CACHE")
        .envs(env.iter().copied())
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
    // Keep draining the host's stdout (it may print more).
    std::thread::spawn(move || {
        for l in out.lines().map_while(Result::ok) {
            if let Some(tx) = &lines {
                let _ = tx.send(l);
            }
        }
    });
    Host(child, sock)
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let t = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &t);
        } else {
            std::fs::copy(e.path(), t).unwrap();
        }
    }
}

/// What a client holds: its pages, and what their ids mean.
#[derive(Default)]
struct View {
    pages: BTreeMap<u32, Page>,
    /// Font id -> key as bound when each page arrived.
    page_fonts: BTreeMap<u32, HashMap<u16, [u8; 32]>>,
    fonts: HashMap<u16, [u8; 32]>,
    spans: HashMap<u32, (u32, u32)>,
    files: HashMap<u32, String>,
    /// `IMAGE` messages, in order.
    images: Vec<Json>,
    /// `FORM` messages, by form id: the latest one of each.
    forms: HashMap<u32, Page>,
    count: usize,
}

struct Outcome {
    started: Json,
    done: Json,
    /// Indices of the PAGE messages, in order.
    order: Vec<u32>,
    pages_msgs: Vec<Json>,
    first_page: Option<Duration>,
    /// `PROGRESS` heartbeats received (`progress-v1`, spec §6.8).
    progress: usize,
    /// The `file`s the `PROGRESS` heartbeats named, in order.
    progress_files: Vec<String>,
    /// `SPAN_SHIFT` messages (`span-shift-v1`, spec §5.3.1), and the spans
    /// `SOURCES` declared (new or moved).
    span_shifts: usize,
    declared: usize,
}

fn compile(c: &mut Client, view: &mut View, req: &CompileRequest) -> Outcome {
    let t0 = Instant::now();
    c.compile(req).unwrap();
    let mut started = Json::Null;
    let mut order = vec![];
    let mut pages_msgs = vec![];
    let mut first_page = None;
    let mut progress = 0;
    let mut progress_files = vec![];
    let (mut span_shifts, mut declared) = (0, 0);
    let done = loop {
        match c.next_event().unwrap().expect("host closed the connection") {
            Event::Started(j) => {
                if j.get("keep").and_then(Json::as_bool) != Some(true) {
                    *view = View::default();
                }
                started = j;
            }
            Event::Font(f) => {
                assert!(
                    !f.program.is_empty() || f.info.str_field("format") != Some("type1"),
                    "font {} without its program",
                    f.id
                );
                view.fonts.insert(f.id, f.key);
            }
            Event::Sources(s) => {
                declared += s.spans.len();
                for (i, p) in s.files {
                    view.files.insert(i, p);
                }
                for (i, f, l) in s.spans {
                    assert!(
                        view.files.contains_key(&f),
                        "span {i} names unknown file {f}"
                    );
                    view.spans.insert(i, (f, l));
                }
            }
            Event::Page(p) => {
                first_page.get_or_insert(t0.elapsed());
                for it in &p.items {
                    match it {
                        Item::Glyph { font, .. } => {
                            assert!(view.fonts.contains_key(font), "font {font} not sent")
                        }
                        Item::Span(s) if *s != 0 => {
                            assert!(view.spans.contains_key(s), "span {s} not sent")
                        }
                        _ => {}
                    }
                }
                order.push(p.index);
                view.page_fonts.insert(p.index, view.fonts.clone());
                view.pages.insert(p.index, p);
            }
            // (a form by its id, as the client keys it: `resolve` compares
            // the forms a page draws, whose content its hash leaves out;
            // its fonts sent before it, as a page's, spec §5)
            Event::Form(f) => {
                for it in &f.items {
                    if let Item::Glyph { font, .. } = it {
                        assert!(
                            view.fonts.contains_key(font),
                            "form {}: font {font} not sent",
                            f.index
                        )
                    }
                }
                view.forms.insert(f.index, f);
            }
            Event::Pages(j) => pages_msgs.push(j),
            Event::Image(j) => view.images.push(j),
            Event::Done(d) => break d,
            Event::Error(e) => panic!("host error: {e}"),
            Event::Other(k, body) if k == flashtex_display_list::kind::SPAN_SHIFT => {
                span_shifts += 1;
                let j = Json::parse(std::str::from_utf8(&body).unwrap()).unwrap();
                let sh = flashtex_display_list::resource::SpanShifts::from_json(&j).unwrap();
                for at in view.spans.values_mut() {
                    *at = sh.apply(*at);
                }
            }
            Event::Other(k, body) if k == flashtex_display_list::kind::PROGRESS => {
                progress += 1;
                let j = Json::parse(std::str::from_utf8(&body).unwrap()).unwrap();
                if let Some(f) = j.str_field("file") {
                    progress_files.push(f.to_string());
                }
            }
            _ => {}
        }
    };
    let count = done.int_field("pages").unwrap_or(0) as usize;
    view.count = count;
    view.pages.retain(|&i, _| (i as usize) < count);
    Outcome {
        started,
        done,
        order,
        pages_msgs,
        first_page,
        progress,
        progress_files,
        span_shifts,
        declared,
    }
}

/// A page with its spans resolved to (file relative to `root`, line) and
/// its glyphs' fonts to keys, for comparing across hosts.
#[derive(Debug, PartialEq)]
struct Resolved {
    hash: [u8; 32],
    page: Page,
    spans: Vec<(String, u32)>,
    fonts: Vec<(u16, [u8; 32])>,
    /// The forms the page draws, directly or through other forms, in order
    /// of first use: id and content hash (lane TEST-GAPS: a page's hash
    /// names its forms by id only, so a stale form went unnoticed).
    forms: Vec<(u32, [u8; 32])>,
}

/// The forms `items` draw, and the forms those draw, in order of first
/// use, as `view` holds them; a form drawn but never sent fails.
fn forms_drawn(view: &View, items: &[Item], what: &str, into: &mut Vec<(u32, [u8; 32])>) {
    for it in items {
        if let Item::Form { id, .. } = it {
            if into.iter().any(|(k, _)| k == id) {
                continue;
            }
            let f = view
                .forms
                .get(id)
                .unwrap_or_else(|| panic!("{what} draws form {id}, which was never sent"));
            into.push((*id, f.hash));
            forms_drawn(view, &f.items, what, into);
        }
    }
}

fn resolve(view: &View, i: u32, root: &Path, out: &Path) -> Resolved {
    let mut page = view
        .pages
        .get(&i)
        .unwrap_or_else(|| panic!("page {i} missing"))
        .clone();
    // Files under the project or the output directory, by their place
    // there (the scratch compile's copies are elsewhere).
    let prefixes: Vec<(String, &str)> = [(root, ""), (out, "<out>/")]
        .iter()
        .flat_map(|(d, tag)| {
            let canon = std::fs::canonicalize(d).unwrap_or_else(|_| d.to_path_buf());
            [
                (format!("{}/", d.display()), *tag),
                (format!("{}/", canon.display()), *tag),
            ]
        })
        .collect();
    let loc = |s: u32| -> (String, u32) {
        if s == 0 {
            return (String::new(), 0);
        }
        let (f, l) = view.spans[&s];
        let p = view.files[&f].clone();
        for (pre, tag) in &prefixes {
            if let Some(rest) = p.strip_prefix(pre.as_str()) {
                return (format!("{tag}{rest}"), l);
            }
        }
        (p, l)
    };
    let mut spans = vec![];
    for it in page.items.iter_mut() {
        if let Item::Span(s) = it {
            spans.push(loc(*s));
            *s = 0;
        }
    }
    for l in page.links.iter_mut() {
        spans.push(loc(l.span));
        l.span = 0;
    }
    let bound = &view.page_fonts[&i];
    let mut fonts: Vec<(u16, [u8; 32])> = page
        .items
        .iter()
        .filter_map(|it| match it {
            Item::Glyph { font, .. } => Some((*font, bound[font])),
            _ => None,
        })
        .collect();
    let mut forms = vec![];
    forms_drawn(view, &page.items, &format!("page {i}"), &mut forms);
    // (and the fonts of the forms it draws: a form's font may be used
    // nowhere else, review of #2019)
    for (id, _) in &forms {
        for it in &view.forms[id].items {
            if let Item::Glyph { font, .. } = it {
                let key = bound.get(font).or_else(|| view.fonts.get(font));
                let key = *key.unwrap_or_else(|| panic!("form {id}: font {font} not sent"));
                fonts.push((*font, key));
            }
        }
    }
    fonts.sort();
    fonts.dedup();
    Resolved {
        hash: page.hash,
        page,
        spans,
        fonts,
        forms,
    }
}

/// What differs between two resolved pages (the first difference).
fn describe_difference(a: &Resolved, b: &Resolved) -> String {
    if a.hash != b.hash {
        return format!(
            "content hash {:02x?}.. vs {:02x?}..",
            &a.hash[..4],
            &b.hash[..4]
        );
    }
    if a.fonts != b.fonts {
        return format!("fonts {:?} vs {:?}", a.fonts, b.fonts);
    }
    if a.forms != b.forms {
        let short = |v: &[(u32, [u8; 32])]| -> Vec<String> {
            v.iter()
                .map(|(k, h)| format!("{k}:{:02x?}", &h[..4]))
                .collect()
        };
        return format!("forms {:?} vs {:?}", short(&a.forms), short(&b.forms));
    }
    if let Some(k) =
        (0..a.spans.len().max(b.spans.len())).find(|&k| a.spans.get(k) != b.spans.get(k))
    {
        return format!(
            "span #{k}: {:?} vs {:?} (of {} and {})",
            a.spans.get(k),
            b.spans.get(k),
            a.spans.len(),
            b.spans.len()
        );
    }
    let (p, q) = (&a.page, &b.page);
    if let Some(k) =
        (0..p.items.len().max(q.items.len())).find(|&k| p.items.get(k) != q.items.get(k))
    {
        return format!("item #{k}: {:?} vs {:?}", p.items.get(k), q.items.get(k));
    }
    if p.links != q.links {
        return format!("links {:?} vs {:?}", p.links, q.links);
    }
    if p.dests != q.dests {
        return format!("dests {:?} vs {:?}", p.dests, q.dests);
    }
    format!(
        "header/other: {:?} vs {:?}",
        (p.index, p.flags, p.counts),
        (q.index, q.flags, q.counts)
    )
}

fn req(id: i64, root: &Path, out: &Path, main: &str) -> CompileRequest {
    let mut r = CompileRequest::new(id, root.to_str().unwrap(), main);
    r.output_dir = Some(out.to_str().unwrap().into());
    r.incremental = true;
    r
}

/// A copy of the project and output directories as they are now.
fn snapshot(base: &Path, proj: &Path, out: &Path, tag: &str) -> (PathBuf, PathBuf) {
    let (p2, o2) = (
        base.join(format!("proj{tag}")),
        base.join(format!("out{tag}")),
    );
    copy_dir(proj, &p2);
    copy_dir(out, &o2);
    (p2, o2)
}

/// Compile the copy `p2` (output `o2`) from scratch in the host at `sock`
/// (which has never seen it) and compare every page with `view`'s.
#[allow(clippy::too_many_arguments)]
fn compare_with_scratch(
    sock: &Path,
    view: &View,
    proj: &Path,
    out: &Path,
    p2: &Path,
    o2: &Path,
    main: &str,
    what: &str,
) {
    compare_with_scratch_job(sock, view, proj, out, p2, o2, main, None, what)
}

/// `compare_with_scratch` for a job with `includeonly` (a chapter focus).
#[allow(clippy::too_many_arguments)]
fn compare_with_scratch_job(
    sock: &Path,
    view: &View,
    proj: &Path,
    out: &Path,
    p2: &Path,
    o2: &Path,
    main: &str,
    includeonly: Option<Vec<String>>,
    what: &str,
) {
    let mut s = Client::connect(sock).unwrap();
    let mut sv = View::default();
    let mut rs = CompileRequest::new(1, p2.to_str().unwrap(), main);
    rs.output_dir = Some(o2.to_str().unwrap().into());
    rs.includeonly = includeonly;
    let so = compile(&mut s, &mut sv, &rs);
    assert_eq!(
        so.done.str_field("mode"),
        Some("cold"),
        "{what}: {}",
        so.done
    );
    assert_eq!(sv.count, view.count, "{what}: page count");
    assert_eq!(view.pages.len(), view.count, "{what}: pages missing");
    for i in 0..view.count as u32 {
        let a = resolve(view, i, proj, out);
        let b = resolve(&sv, i, p2, o2);
        if a != b {
            panic!(
                "{what}: page {i} differs from the scratch compile: {}",
                describe_difference(&a, &b)
            );
        }
    }
    let _ = s.bye();
}

/// The page whose glyphs come from line `line` of `main`, if only one.
fn page_of_line(view: &View, line: u32, main: &str) -> Option<u32> {
    let main = format!("/{main}");
    let pages: Vec<u32> = view
        .pages
        .iter()
        .filter(|(_, p)| {
            p.items.iter().any(|it| {
                matches!(it, Item::Span(s) if *s != 0
                    && view.spans.get(s).is_some_and(|&(f, l)| l == line
                        && view.files[&f].ends_with(&main)))
            })
        })
        .map(|(i, _)| *i)
        .collect();
    (pages.len() == 1).then(|| pages[0])
}

enum EditKind {
    /// Replace the word at the middle of the line by another as long.
    Word,
    /// Insert a comment line before the line.
    Comment,
    /// Insert a new paragraph before the line.
    Paragraph,
}

/// The first plain lower-case word (3 letters or more) after the middle
/// of `line`: its byte offset and the word.
fn middle_word(line: &str) -> Option<(usize, &str)> {
    let mut at = line.len() / 2;
    while !line.is_char_boundary(at) {
        at += 1;
    }
    loop {
        at += line[at..].find(' ')? + 1;
        let w = line[at..].split(' ').next().unwrap();
        if w.len() >= 3 && w.bytes().all(|c| c.is_ascii_lowercase()) {
            return Some((at, w));
        }
    }
}

/// Open `src` (a directory with `main`) in a resident host, make each edit
/// on a prose line of the page `at` pages into the document, and compare
/// every page held after each compile (and each compile that settles the
/// `.aux` after it) with a from-scratch compile. `strict`: also require a
/// clean document of 3 pages or more, every edit to find its line, and the
/// edited page first. Returns the number of compiles compared.
fn check_document(
    name: &str,
    src: &Path,
    main: &str,
    edits: &[(EditKind, f64)],
    strict: bool,
) -> usize {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return 0;
    }
    let base = common::fresh_dir(&format!("flashtex-host-incr-{name}"));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let (proj, out) = (base.join("proj"), base.join("out"));
    copy_dir(src, &proj);
    std::fs::create_dir_all(&out).unwrap();
    let host = start_host("a");
    let scratch = start_host("b");
    let mut c = Client::connect(&host.1).unwrap();
    assert_eq!(
        c.hello
            .get("version")
            .and_then(Json::as_array)
            .map(|v| v.to_vec()),
        Some(vec![Json::Int(3), Json::Int(2)])
    );
    let mut view = View::default();
    let mut id = 0;
    let mut compared = 0;
    // Open: compile until the .aux is stable.
    for _ in 0..4 {
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
        if strict {
            assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
        }
        if o.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    if strict {
        assert!(view.count >= 3, "{name}: only {} pages", view.count);
    }
    assert_eq!(view.pages.len(), view.count, "{name}: pages missing");
    for (k, (kind, at)) in edits.iter().enumerate() {
        // A line whose glyphs are on one page, `at` into the document.
        let text = String::from_utf8_lossy(&std::fs::read(proj.join(main)).unwrap()).into_owned();
        let lines: Vec<&str> = text.split('\n').collect();
        let want = ((view.count as f64 * at) as u32).min((view.count as u32).max(1) - 1);
        let found = (1..=lines.len() as u32)
            .filter(|&l| {
                let t = lines[l as usize - 1];
                t.split(' ').count() > 6 && !t.trim_start().starts_with('%')
            })
            .filter(|&l| {
                !matches!(kind, EditKind::Word) || middle_word(lines[l as usize - 1]).is_some()
            })
            .filter_map(|l| page_of_line(&view, l, main).map(|p| (l, p)))
            .min_by_key(|&(_, p)| p.abs_diff(want));
        let Some((line, page)) = found else {
            assert!(!strict, "{name}: no prose line on one page");
            eprintln!("{name} edit {k}: no prose line on one page; skipped");
            continue;
        };
        let offset: usize = lines[..line as usize - 1].iter().map(|l| l.len() + 1).sum();
        let this = lines[line as usize - 1];
        let edit = match kind {
            EditKind::Word => {
                let (at, w) = middle_word(this).unwrap();
                Edit {
                    path: main.into(),
                    offset: (offset + at) as u64,
                    delete: w.len() as u64,
                    insert: "q".repeat(w.len()),
                }
            }
            EditKind::Comment => Edit {
                path: main.into(),
                offset: offset as u64,
                delete: 0,
                insert: "% a comment the output does not show\n".into(),
            },
            EditKind::Paragraph => Edit {
                path: main.into(),
                offset: offset as u64,
                delete: 0,
                insert: format!("{this}\n\n{this}\n\n"),
            },
        };
        // The directory as the compile will find it, for the scratch run.
        let (p2, o2) = snapshot(&base, &proj, &out, &format!("{k}"));
        let mut t = std::fs::read(p2.join(main)).unwrap();
        let (a, d) = (edit.offset as usize, edit.delete as usize);
        t.splice(a..a + d, edit.insert.bytes());
        std::fs::write(p2.join(main), t).unwrap();

        id += 1;
        let mut r = req(id, &proj, &out, main);
        r.edits = vec![edit];
        r.viewport = Some(page);
        let o = compile(&mut c, &mut view, &r);
        assert_eq!(o.started.get("keep").and_then(Json::as_bool), Some(true));
        // In page order: each pass goes forward; a later `.aux` pass
        // (DESIGN.md §5.5) sends the pages it typesets again from where it
        // restarts, which may be before the pages already sent (spec §6.4).
        let mut runs = vec![vec![]];
        for &i in &o.order {
            if runs.last().unwrap().last().is_some_and(|&l: &u32| i <= l) {
                runs.push(vec![]);
            }
            runs.last_mut().unwrap().push(i);
        }
        assert!(
            runs.len() <= 5,
            "{name} edit {k}: pages out of order: {:?}",
            o.order
        );
        // PAGES: all current at the end.
        let last = o.pages_msgs.last().expect("no PAGES");
        assert_eq!(last.get("complete").and_then(Json::as_bool), Some(true));
        assert_eq!(last.int_field("count"), Some(view.count as i64));
        let first = o.order.first().copied();
        if strict {
            assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
            assert_eq!(
                o.done.str_field("mode"),
                Some("incremental"),
                "{name} edit {k}: {}",
                o.done
            );
            // The edited page first: the run restarts at most one page before.
            let first = first.expect("no page re-sent");
            assert!(
                first == page || first + 1 == page,
                "{name} edit {k}: first page {first}, edited page {page}: {}",
                o.done
            );
            assert!(o.order.contains(&page), "the edited page was not re-sent");
            assert!(o.pages_msgs.len() >= 2, "no PAGES before the end");
        }
        eprintln!(
            "{name} edit {k}: first page {:?} (edited {page}) of {} after {:.1} ms; {}",
            first,
            view.count,
            o.first_page.map_or(0.0, |d| d.as_secs_f64() * 1e3),
            o.done
        );
        compare_with_scratch(
            &scratch.1,
            &view,
            &proj,
            &out,
            &p2,
            &o2,
            main,
            &format!("{name} edit {k}"),
        );
        compared += 1;
        // Settle: the edit may have changed the .aux (page numbers); compile
        // until nothing changes, each compile compared with scratch too.
        for n in 0..4 {
            let (p2, o2) = snapshot(&base, &proj, &out, &format!("{k}s{n}"));
            id += 1;
            let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
            if strict {
                assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
            }
            if o.done.str_field("mode") == Some("unchanged") {
                break;
            }
            eprintln!("{name} edit {k}, settling {n}: {}", o.done);
            compare_with_scratch(
                &scratch.1,
                &view,
                &proj,
                &out,
                &p2,
                &o2,
                main,
                &format!("{name} edit {k} settling {n}"),
            );
            compared += 1;
        }
    }
    let _ = c.bye();
    drop(host);
    drop(scratch);
    let _ = std::fs::remove_dir_all(&base);
    compared
}

/// A multi-pass fixture's `PASSES` file (`fixtures/multipass/<name>/`):
/// line 1 the fixed-point count, line 2 the command sequence as run in the
/// fixture's directory, `;`-separated (`pdflatex … main; bibtex main;
/// makeindex -s dots.ist main; …`). Each step's argv, without the leading
/// `VAR=value` assignments (the tests pin the same environment).
fn read_passes(d: &Path) -> Option<Vec<Vec<String>>> {
    let t = std::fs::read_to_string(d.join("PASSES")).ok()?;
    let line = t.lines().nth(1)?;
    let steps: Vec<Vec<String>> = line
        .split(';')
        .map(|s| {
            s.split_whitespace()
                .skip_while(|w| w.contains('=') && !w.starts_with('-'))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.is_empty())
        .collect();
    let known = |p: &str| matches!(p, "pdflatex" | "bibtex" | "makeindex");
    assert!(
        steps.iter().all(|s| known(&s[0])),
        "{}: a PASSES step this test cannot run: {steps:?}",
        d.display()
    );
    Some(steps)
}

/// Run a `PASSES` tool step (`bibtex`, `makeindex`) in `dir` with TeX
/// Live's program, as the fixture's sequence runs it.
fn run_tool(argv: &[String], dir: &Path) {
    let bin = find_texlive_bin().expect("TeX Live");
    let st = Command::new(bin.join(&argv[0]))
        .args(&argv[1..])
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    // bibtex exits 1 on warnings (a key the .bib lacks) and still writes
    // its .bbl; 2 or more is a fatal error, as is any makeindex failure
    let code = st.code().unwrap_or(-1);
    let last = argv.last().unwrap();
    let (ok, out) = if argv[0] == "bibtex" {
        (code == 0 || code == 1, format!("{last}.bbl"))
    } else {
        let stem = last.strip_suffix(".idx").unwrap_or(last);
        (code == 0, format!("{stem}.ind"))
    };
    assert!(
        ok && dir.join(&out).is_file(),
        "{argv:?} in {}: exit {code}, {out} {}",
        dir.display(),
        if dir.join(&out).is_file() {
            "written"
        } else {
            "missing"
        }
    );
}

/// A `pdflatex` step from scratch: this engine (`flashtex-initex` as
/// `pdftex`, the format this test built) with the step's arguments, in
/// `dir`; it must succeed and write the `.aux`.
fn run_engine(argv: &[String], dir: &Path) {
    let fmt = fmt_dir();
    let pdftex = fmt.join("pdftex");
    common::link_engine(Path::new(env!("CARGO_BIN_EXE_flashtex-initex")), &pdftex);
    let st = Command::new(&pdftex)
        .arg("-fmt=pdflatex")
        .args(&argv[1..])
        .current_dir(dir)
        .env("FLASHTEX_POOL", pool())
        .env("FLASHTEX_FORMATS", &fmt)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env_remove("FLASHTEX_S0_CACHE")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    let job = argv.last().unwrap();
    let job = job.strip_suffix(".tex").unwrap_or(job);
    assert!(
        st.success() && dir.join(format!("{job}.aux")).is_file(),
        "{argv:?} in {}: {st}",
        dir.display()
    );
}

/// Every file of the directory with its contents, but the PDF and the
/// log (the pages are compared instead) and the tools' logs: the sources
/// (edited alike on both sides) and whatever the passes and the tools
/// write and read back (`.aux`, `.bbl`, `.ind`, `.toc`, a fixture's own
/// `.glsdef`, …).
fn pass_files(d: &Path) -> BTreeMap<String, Vec<u8>> {
    const SKIP: &[&str] = &["pdf", "log", "blg", "ilg"];
    std::fs::read_dir(d)
        .unwrap()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| {
            !n.rsplit_once('.')
                .is_some_and(|(_, ext)| SKIP.contains(&ext))
        })
        .map(|n| {
            let b = std::fs::read(d.join(&n)).unwrap();
            (n, b)
        })
        .collect()
}

/// Soundness of multi-pass documents (lane P4-MULTIPASS; the fixtures of
/// #1306): open `src` in a resident host, following its `PASSES` (each
/// `pdflatex` a `COMPILE`, which runs the engine's own `.aux` passes; each
/// tool run in the directory as the sequence runs it), then compile until
/// nothing changes. Then for each single-character edit: the edited
/// `COMPILE`, the rest of the sequence again, and compiles until nothing
/// changes; and from scratch, on a clean copy of the edited sources, the
/// whole sequence with this engine, runs until the files repeat, and a
/// fresh host's compile of the result. Every page the client holds and
/// every file the passes write (`pass_files`) must be the from-scratch
/// result's. Returns the number of edits compared.
fn check_multipass(name: &str, src: &Path, steps: &[Vec<String>], ats: &[f64]) -> usize {
    let main = "main.tex";
    let base = common::fresh_dir(&format!("fth-mp-{name}"));
    let _ = std::fs::remove_dir_all(&base);
    let proj = base.join("proj");
    copy_dir(src, &proj);
    let host = start_host("m");
    let scratch = start_host("n");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    let mut compile_here = |c: &mut Client, view: &mut View, edits: Vec<Edit>, vp: Option<u32>| {
        id += 1;
        let mut r = req(id, &proj, &proj, main);
        r.edits = edits;
        r.viewport = vp;
        let o = compile(c, view, &r);
        assert_eq!(o.done.str_field("status"), Some("ok"), "{name}: {}", o.done);
        o
    };
    // (compile until nothing changes: the engine's own passes settle the
    // `.aux` inside one compile, so this is one or two compiles)
    macro_rules! settle {
        () => {
            let mut settled = false;
            for _ in 0..5 {
                let o = compile_here(&mut c, &mut view, vec![], None);
                if o.done.str_field("mode") == Some("unchanged") {
                    settled = true;
                    break;
                }
            }
            assert!(settled, "{name}: the compiles did not settle");
        };
    }
    for s in steps {
        if s[0] == "pdflatex" {
            compile_here(&mut c, &mut view, vec![], None);
        } else {
            run_tool(s, &proj);
        }
    }
    settle!();
    assert!(view.count > 0, "{name}: no pages");
    let mut compared = 0;
    for (k, &at) in ats.iter().enumerate() {
        let text = String::from_utf8_lossy(&std::fs::read(proj.join(main)).unwrap()).into_owned();
        let (line, page) = edit_site(&view, &text, main, at, true)
            .unwrap_or_else(|| panic!("{name} edit {k}: no prose line on one page"));
        let lines: Vec<&str> = text.split('\n').collect();
        let offset: usize = lines[..line as usize - 1].iter().map(|l| l.len() + 1).sum();
        let (wat, w) = middle_word(lines[line as usize - 1]).unwrap();
        let ch = if w.starts_with('q') { "x" } else { "q" };
        let edit = Edit {
            path: main.into(),
            offset: (offset + wat) as u64,
            delete: 1,
            insert: ch.into(),
        };
        // from scratch: the fixture's sources, edited as the host's are
        let p2 = base.join(format!("scratch{k}"));
        copy_dir(src, &p2);
        std::fs::write(p2.join(main), std::fs::read(proj.join(main)).unwrap()).unwrap();
        let mut t = std::fs::read(p2.join(main)).unwrap();
        let a = edit.offset as usize;
        t.splice(a..a + 1, edit.insert.bytes());
        std::fs::write(p2.join(main), t).unwrap();
        // incremental: the edited compile, the rest of the sequence, settle
        let o = compile_here(&mut c, &mut view, vec![edit], Some(page));
        eprintln!("{name} edit {k} (line {line}, page {page}): {}", o.done);
        for s in &steps[1..] {
            if s[0] == "pdflatex" {
                compile_here(&mut c, &mut view, vec![], None);
            } else {
                run_tool(s, &proj);
            }
        }
        settle!();
        let pdflatex = steps.iter().find(|s| s[0] == "pdflatex").unwrap();
        for s in steps {
            if s[0] == "pdflatex" {
                run_engine(s, &p2);
            } else {
                run_tool(s, &p2);
            }
        }
        // to a fixed point: a run that changes nothing (an oscillating
        // document fails here)
        let mut last = pass_files(&p2);
        let mut fixed = false;
        for _ in 0..5 {
            run_engine(pdflatex, &p2);
            let now = pass_files(&p2);
            if now == last {
                fixed = true;
                break;
            }
            last = now;
        }
        assert!(
            fixed,
            "{name} edit {k}: the from-scratch runs reach no fixed point"
        );
        // (taken before the scratch host compiles `p2`)
        let fs = pass_files(&p2);
        compare_with_scratch(
            &scratch.1,
            &view,
            &proj,
            &proj,
            &p2,
            &p2,
            main,
            &format!("{name} edit {k}"),
        );
        // the scratch host's pages are of that fixed point: its compile
        // left the files as they were
        assert!(
            pass_files(&p2) == fs,
            "{name} edit {k}: the scratch host's compile changed the fixed point's files"
        );
        let fi = pass_files(&proj);
        let differ: Vec<&String> = fi
            .keys()
            .chain(fs.keys())
            .filter(|n| fi.get(*n) != fs.get(*n))
            .collect();
        assert!(
            differ.is_empty(),
            "{name} edit {k}: files differ from the from-scratch sequence: {differ:?}"
        );
        compared += 1;
    }
    let _ = c.bye();
    drop(host);
    drop(scratch);
    let _ = std::fs::remove_dir_all(&base);
    compared
}

/// A line of `text` (of `main`) whose glyphs are all on one page, nearest
/// to `at` of the way into the document: with `word`, one that has a
/// middle word to edit. (line, page)
fn edit_site(view: &View, text: &str, main: &str, at: f64, word: bool) -> Option<(u32, u32)> {
    let lines: Vec<&str> = text.split('\n').collect();
    let want = ((view.count as f64 * at) as u32).min((view.count as u32).max(1) - 1);
    (1..=lines.len() as u32)
        .filter(|&l| {
            let t = lines[l as usize - 1];
            t.split(' ').count() > 6 && !t.trim_start().starts_with('%')
        })
        .filter(|&l| !word || middle_word(lines[l as usize - 1]).is_some())
        .filter_map(|l| page_of_line(view, l, main).map(|p| (l, p)))
        .min_by_key(|&(_, p)| p.abs_diff(want))
}

/// A deterministic article of about `pages` pages: one paragraph per line.
fn article(pages: usize) -> String {
    const WORDS: &[&str] = &[
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
        "ad",
        "minim",
        "veniam",
        "quis",
        "nostrud",
        "exercitation",
        "ullamco",
        "laboris",
        "nisi",
        "aliquip",
        "ex",
        "ea",
        "commodo",
        "consequat",
    ];
    let mut x: u64 = 12345;
    let mut next = move || {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (x >> 33) as usize
    };
    let mut s = String::from(
        "\\documentclass{article}\n\\usepackage{amsmath}\n\\begin{document}\n\\section{Start}\n\n",
    );
    for k in 0..pages * 5 {
        if k % 9 == 8 {
            s.push_str(&format!("\\section{{Part {}}}\n\n", k / 9 + 1));
        }
        let n = 90 + next() % 30;
        let mut w: Vec<String> = (0..n)
            .map(|_| WORDS[next() % WORDS.len()].to_string())
            .collect();
        w[0] = format!("{}{}", w[0][..1].to_uppercase(), &w[0][1..]);
        let mid = n / 2;
        w[mid] = format!("{} with $x_{{{}}}^2+\\frac{{a}}{{b}}$", w[mid], k % 17);
        s.push_str(&w.join(" "));
        s.push_str(".\n\n");
        if k % 6 == 5 {
            s.push_str(&format!(
                "\\begin{{equation}}\n  y_{{{k}}} = \\sum_{{i=1}}^n c_i\n\\end{{equation}}\n\n"
            ));
        }
    }
    s.push_str("\\end{document}\n");
    s
}

/// The `progress-v1` heartbeat (incr::Session::set_progress) is told at
/// every page and segment checkpoint of every run. It must change nothing
/// the engine typesets: a client that accepted it, through a cold compile
/// and an edit's incremental compile, holds exactly the pages a scratch
/// compile (a client without it) gets; and only a client that accepted it
/// receives `PROGRESS`.
#[test]
fn the_progress_heartbeat_changes_no_page() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let host = start_host("progress");
    let base = common::fresh_dir("flashtex-host-progress");
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let text = article(12);
    std::fs::write(proj.join("main.tex"), &text).unwrap();
    let mut c =
        Client::connect_accepting(&host.1, &[flashtex_display_list::PROGRESS_CAPABILITY]).unwrap();
    let mut view = View::default();
    let first = compile(&mut c, &mut view, &req(1, &proj, &out, "main.tex"));
    assert!(
        first.progress > 0,
        "PROGRESS while typesetting: {}",
        first.done
    );
    // It names the file TeX reads (the innermost `\input`). (Integration
    // of #1551/#1684: a pass's first heartbeat now comes at its first
    // checkpoint in the preamble -- a `Point::PreambleLine` after a
    // package, or the `.aux` anchor -- so on this short document, done in
    // under 250 ms a pass, it may name the class or the `.aux`, not main.tex.)
    assert!(
        !first.progress_files.is_empty()
            && first
                .progress_files
                .iter()
                .all(|f| Path::new(f).is_file() || proj.join(f).is_file()),
        "PROGRESS names the file: {:?}",
        first.progress_files
    );
    // An edit near the middle: an incremental compile, with the heartbeat.
    let at = text.len() / 2;
    let at = (at..text.len())
        .find(|&i| text.as_bytes()[i] == b' ')
        .unwrap();
    let mut r2 = req(2, &proj, &out, "main.tex");
    r2.edits.push(Edit {
        path: "main.tex".into(),
        offset: at as u64,
        delete: 0,
        insert: " inserted".into(),
    });
    let edited = compile(&mut c, &mut view, &r2);
    assert_eq!(
        edited.done.str_field("mode"),
        Some("incremental"),
        "the edit compiles incrementally: {}",
        edited.done
    );
    assert!(
        edited.progress > 0,
        "PROGRESS in the incremental compile: {}",
        edited.done
    );
    let mut now = text.clone();
    now.insert_str(at, " inserted");
    std::fs::write(proj.join("main.tex"), &now).unwrap();
    let (p2, o2) = snapshot(&base, &proj, &out, "scratch");
    compare_with_scratch(
        &host.1,
        &view,
        &proj,
        &out,
        &p2,
        &o2,
        "main.tex",
        "with the progress heartbeat",
    );
    // A client that did not accept progress-v1 gets none.
    let mut plain = Client::connect(&host.1).unwrap();
    let mut pv = View::default();
    let o = compile(&mut plain, &mut pv, &req(3, &p2, &o2, "main.tex"));
    assert_eq!(o.progress, 0, "no PROGRESS without progress-v1");
    let _ = c.bye();
    let _ = plain.bye();
    let _ = std::fs::remove_dir_all(&base);
}

/// `span-shift-v1` (spec §5.3.1): a comment line inserted near the top of a
/// 40-page article moves every later span. A client that accepts it gets
/// one `SPAN_SHIFT` and holds the very spans a client sent every moved span
/// in `SOURCES` holds (another host, the same edits), and its pages resolve
/// as a from-scratch compile's.
#[test]
fn a_span_shift_client_holds_the_spans_a_full_resend_gives() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let text = article(40);
    let at = text
        .find("\\begin{document}")
        .map(|i| i + "\\begin{document}\n".len())
        .unwrap();
    let mut views = vec![];
    for (tag, accept) in [
        ("full", &[][..]),
        (
            "shifted",
            &[flashtex_display_list::SPAN_SHIFT_CAPABILITY][..],
        ),
    ] {
        let host = start_host(&format!("span-shift-{tag}"));
        let base = common::fresh_dir(&format!("flashtex-host-span-shift-{tag}"));
        let (proj, out) = (base.join("proj"), base.join("out"));
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(proj.join("main.tex"), &text).unwrap();
        let mut c = Client::connect_accepting(&host.1, accept).unwrap();
        let mut view = View::default();
        compile(&mut c, &mut view, &req(1, &proj, &out, "main.tex"));
        let held = view.spans.len();
        let mut r2 = req(2, &proj, &out, "main.tex");
        r2.edits.push(Edit {
            path: "main.tex".into(),
            offset: at as u64,
            delete: 0,
            insert: "% a comment line near the top\n".into(),
        });
        let o = compile(&mut c, &mut view, &r2);
        assert_eq!(
            o.done.str_field("mode"),
            Some("incremental"),
            "{tag}: {}",
            o.done
        );
        if accept.is_empty() {
            assert_eq!(o.span_shifts, 0, "no SPAN_SHIFT without span-shift-v1");
            assert!(
                o.declared * 2 > held,
                "{tag}: every later span re-declared ({} of {held})",
                o.declared
            );
        } else {
            assert_eq!(o.span_shifts, 1, "{tag}: one shift");
            assert!(
                o.declared * 20 < held,
                "{tag}: {} of {held} spans re-declared",
                o.declared
            );
        }
        let mut now = text.clone();
        now.insert_str(at, "% a comment line near the top\n");
        std::fs::write(proj.join("main.tex"), &now).unwrap();
        let (p2, o2) = snapshot(&base, &proj, &out, "scratch");
        compare_with_scratch(&host.1, &view, &proj, &out, &p2, &o2, "main.tex", tag);
        let rel: BTreeMap<u32, (String, u32)> = view
            .spans
            .iter()
            .map(|(&s, &(f, l))| {
                let p = view.files.get(&f).cloned().unwrap_or_default();
                // (by name: each host has its own project directory)
                let p = Path::new(&p)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or(p);
                (s, (p, l))
            })
            .collect();
        views.push(rel);
        let _ = c.bye();
        let _ = std::fs::remove_dir_all(&base);
    }
    assert_eq!(
        views[0], views[1],
        "the shifted client's spans are the full re-send's"
    );
}

/// Performance modes over the socket (`profile-v1`, spec §6.9): the client
/// names one in its `HELLO`, the host's `HELLO` says which knobs apply, a
/// `PROFILE` switches it live (answered once applied), and the pages after
/// each switch equal a from-scratch host's.
#[test]
fn a_performance_mode_switched_live_changes_no_page() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let host = start_host_env("profile", &[("FLASHTEX_MEMSTAT", "1")]);
    // (scratch compiles in their own host: the mode's host keeps its document)
    let scratch = start_host("profile-scratch");
    let base = common::fresh_dir("flashtex-host-profile");
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let mut text = article(12);
    std::fs::write(proj.join("main.tex"), &text).unwrap();
    let mut c = Client::connect_with(&host.1, &[], Some("balanced")).unwrap();
    let caps = c
        .hello
        .get("capabilities")
        .and_then(Json::as_array)
        .unwrap();
    assert!(
        caps.iter()
            .any(|x| x.as_str() == Some(flashtex_display_list::PROFILE_CAPABILITY)),
        "{}",
        c.hello
    );
    let hp = c.hello.get("profile").expect("HELLO.profile");
    assert_eq!(hp.str_field("mode"), Some("balanced"), "{}", c.hello);
    let mut view = View::default();
    let first = compile(&mut c, &mut view, &req(1, &proj, &out, "main.tex"));
    let cache = |done: &Json| {
        done.get("mem")
            .and_then(|m| m.int_field("page_cache"))
            .expect("DONE.mem.page_cache (FLASHTEX_MEMSTAT=1)")
    };
    let unpacked = cache(&first.done);
    let mut id = 1;
    for (mode, frac) in [
        ("high-performance", 0.8),
        ("low-memory", 0.3),
        ("balanced", 0.6),
        ("low-memory", 0.5),
    ] {
        c.set_profile(mode).unwrap();
        let applied = loop {
            match c.next_event().unwrap().expect("host closed") {
                Event::Profile(j) => break j,
                Event::Error(e) => panic!("host error: {e}"),
                _ => {}
            }
        };
        let p = applied.get("profile").expect("PROFILE.profile");
        assert_eq!(p.str_field("mode"), Some(mode), "{applied}");
        let at = (text.len() as f64 * frac) as usize;
        let at = (at..text.len())
            .find(|&i| text.as_bytes()[i] == b' ')
            .unwrap();
        id += 1;
        let mut r = req(id, &proj, &out, "main.tex");
        r.edits.push(Edit {
            path: "main.tex".into(),
            offset: at as u64,
            delete: 0,
            insert: " moded".into(),
        });
        let o = compile(&mut c, &mut view, &r);
        assert_eq!(o.done.str_field("mode"), Some("incremental"), "{}", o.done);
        text.insert_str(at, " moded");
        std::fs::write(proj.join("main.tex"), &text).unwrap();
        let (p2, o2) = snapshot(&base, &proj, &out, &format!("{mode}-{id}"));
        compare_with_scratch(
            &scratch.1,
            &view,
            &proj,
            &out,
            &p2,
            &o2,
            "main.tex",
            &format!("after switching to {mode}"),
        );
    }
    // Low Memory packs the page cache off the engine thread; a client that
    // lacks the pages (a second, 3.0-style connection) gets them unpacked,
    // equal to a scratch host's.
    std::thread::sleep(Duration::from_millis(1500));
    let mut other = Client::connect(&host.1).unwrap();
    let mut ov = View::default();
    let mut r = req(99, &proj, &out, "main.tex");
    r.incremental = false;
    let o = compile(&mut other, &mut ov, &r);
    let packed = cache(&o.done);
    assert!(
        packed * 10 < unpacked * 7,
        "Low Memory packs the page cache: {packed} of {unpacked} bytes"
    );
    let (p2, o2) = snapshot(&base, &proj, &out, "packed");
    compare_with_scratch(
        &scratch.1,
        &ov,
        &proj,
        &out,
        &p2,
        &o2,
        "main.tex",
        "pages delivered from the packed cache",
    );
    let _ = other.bye();
    // An unknown mode is refused, and the mode in effect stays.
    c.set_profile("turbo").unwrap();
    loop {
        match c.next_event().unwrap().expect("host closed") {
            Event::Error(_) => break,
            Event::Profile(j) => panic!("an unknown mode was applied: {j}"),
            _ => {}
        }
    }
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn edits_stream_the_edited_page_and_equal_scratch_compiles_article() {
    let src = common::fresh_dir("flashtex-host-incr-src");
    let _ = std::fs::remove_dir_all(&src);
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("main.tex"), article(12)).unwrap();
    check_document(
        "article",
        &src,
        "main.tex",
        &[
            (EditKind::Word, 0.5),
            (EditKind::Comment, 0.3),
            (EditKind::Paragraph, 0.6),
            (EditKind::Word, 0.9),
        ],
        true,
    );
    let _ = std::fs::remove_dir_all(&src);
}

#[test]
fn edits_stream_the_edited_page_and_equal_scratch_compiles_hyperref_toc() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/real-world/hyperref-toc");
    check_document(
        "hyperref-toc",
        &src,
        "main.tex",
        &[(EditKind::Word, 0.5), (EditKind::Comment, 0.5)],
        true,
    );
}

/// `"export": true`: a one-shot run of the engine as a child process (the
/// host itself, invoked as `pdftex`), streaming the same frames, with the
/// compressed PDF in `DONE`.
#[test]
fn export_runs_the_engine_as_a_child() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-export");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out) = (base.join("proj"), base.join("out"));
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/real-world/hyperref-toc"),
        &proj,
    );
    std::fs::create_dir_all(&out).unwrap();
    let host = start_host("x");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut r = CompileRequest::new(1, proj.to_str().unwrap(), "main.tex");
    r.output_dir = Some(out.to_str().unwrap().into());
    r.export = true;
    let o = compile(&mut c, &mut view, &r);
    assert_eq!(o.started.str_field("mode"), Some("export"), "{}", o.started);
    assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
    assert!(view.count >= 3);
    assert_eq!(o.order, (0..view.count as u32).collect::<Vec<_>>());
    let pdf = std::fs::read(out.join("main.pdf")).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    // Compressed, as pdflatex writes it (the preview's PDF stores streams).
    assert!(pdf.windows(12).any(|w| w == b"/FlateDecode"));
    let _ = c.bye();
    drop(host);
    let _ = std::fs::remove_dir_all(&base);
}

/// Every parity fixture through the socket host (run with `--ignored`; it
/// takes minutes): a word replaced at the middle of the document, a comment
/// line inserted early and a word replaced late; after every compile every
/// page the client holds must equal a from-scratch compile's.
/// `FLASHTEX_HOST_SWEEP_ONLY=name` runs one fixture.
#[test]
#[ignore]
fn every_fixture_edits_equal_scratch_compiles() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let only = std::env::var("FLASHTEX_HOST_SWEEP_ONLY").ok();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let (mut docs, mut compared) = (0, 0);
    for tier in ["real-world", "divergence-probes"] {
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(root.join(tier))
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        for d in dirs {
            let name = d.file_name().unwrap().to_string_lossy().into_owned();
            if only.as_deref().is_some_and(|o| o != name) {
                continue;
            }
            let texs: Vec<String> = std::fs::read_dir(&d)
                .unwrap()
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.ends_with(".tex"))
                .collect();
            let main = if texs.iter().any(|t| t == "main.tex") {
                "main.tex".to_string()
            } else if texs.len() == 1 {
                texs[0].clone()
            } else {
                continue;
            };
            let n = check_document(
                &name,
                &d,
                &main,
                &[
                    (EditKind::Word, 0.5),
                    (EditKind::Comment, 0.2),
                    (EditKind::Word, 0.9),
                ],
                false,
            );
            docs += 1;
            compared += n;
            eprintln!("SWEEP {name}: {n} compiles compared");
        }
    }
    // The multi-pass fixtures (#1306), each with the `PASSES` sequence it
    // reaches its fixed point with, tools included (`check_multipass`).
    let mp = root.join("multipass");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&mp)
        .unwrap_or_else(|e| panic!("{}: {e}", mp.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    assert!(!dirs.is_empty(), "{}: no fixtures", mp.display());
    dirs.sort();
    for d in dirs {
        let name = d.file_name().unwrap().to_string_lossy().into_owned();
        if only.as_deref().is_some_and(|o| o != name) {
            continue;
        }
        let steps =
            read_passes(&d).unwrap_or_else(|| panic!("multipass/{name}: no readable PASSES file"));
        let ats = [0.3, 0.8];
        let n = check_multipass(&name, &d, &steps, &ats);
        assert_eq!(n, ats.len(), "multipass/{name}: edits compared");
        docs += 1;
        compared += n;
        eprintln!("SWEEP multipass/{name}: {n} edits compared");
    }
    eprintln!("SWEEP: {docs} documents, {compared} compiles compared, 0 differences");
}

/// Lane P4-MULTIPASS regression (CI run 36767622154): an edit whose pages
/// come out exactly as the client holds them (a comment's text changed on
/// its own line: same output, no line moves) still sends the pages the
/// compile typeset, starting at the edited one. Only a *later* pass of the
/// same compile may skip a page it already delivered unchanged.
#[test]
fn an_edit_that_changes_no_page_still_sends_the_pages_it_typesets() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-same");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let doc = article(6);
    let mid = doc.len() / 2;
    let cut = mid + doc[mid..].find("\n\n").unwrap() + 2;
    let text = format!("{}% note A\n\n{}", &doc[..cut], &doc[cut..]);
    std::fs::write(proj.join("main.tex"), &text).unwrap();
    let host = start_host("s");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    for _ in 0..4 {
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, "main.tex"));
        if o.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    let at = text.find("% note A").unwrap() + "% note ".len();
    id += 1;
    let mut r = req(id, &proj, &out, "main.tex");
    r.edits = vec![Edit {
        path: "main.tex".into(),
        offset: at as u64,
        delete: 1,
        insert: "B".into(),
    }];
    let o = compile(&mut c, &mut view, &r);
    assert_eq!(o.done.str_field("mode"), Some("incremental"), "{}", o.done);
    assert!(
        !o.order.is_empty(),
        "no page sent for the pages the compile typeset: {}",
        o.done
    );
    let _ = c.bye();
    drop(host);
    let _ = std::fs::remove_dir_all(&base);
}

/// Preemption (lane P4-L5-RESTART): a COMPILE sent while the previous one is
/// still re-typesetting (typing) stops it at its next checkpoint: its DONE is
/// `cancelled`, the newer compile's pages come, and after it every page the
/// client holds equals a from-scratch compile of the directory then.
#[test]
fn a_newer_compile_preempts_the_running_one() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-preempt");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let main = "main.tex";
    std::fs::write(proj.join(main), article(60)).unwrap();
    let host = start_host("p");
    let scratch = start_host("q");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    for _ in 0..4 {
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
        if o.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    // The first long prose line: a paragraph inserted there reflows the rest.
    let text = String::from_utf8_lossy(&std::fs::read(proj.join(main)).unwrap()).into_owned();
    let lines: Vec<&str> = text.split('\n').collect();
    let line = lines
        .iter()
        .position(|t| {
            t.split(' ').count() > 6 && !t.trim_start().starts_with('%') && !t.starts_with('\\')
        })
        .expect("a prose line");
    let offset: usize = lines[..line].iter().map(|l| l.len() + 1).sum();
    let this = lines[line];
    let mut cancelled = 0;
    for round in 0..3 {
        let first = Edit {
            path: main.into(),
            offset: offset as u64,
            delete: 0,
            insert: format!("{this}\n\n{this}\n\n"),
        };
        let second = Edit {
            path: main.into(),
            offset: (offset + 3 + round) as u64,
            delete: 0,
            insert: "q".into(),
        };
        let (id1, id2) = (id + 1, id + 2);
        id += 2;
        let mut r1 = req(id1, &proj, &out, main);
        r1.edits = vec![first];
        let mut r2 = req(id2, &proj, &out, main);
        r2.edits = vec![second];
        // the second right behind the first, as typing sends them
        c.compile(&r1).unwrap();
        c.compile(&r2).unwrap();
        let mut dones = vec![];
        while dones.len() < 2 {
            match c.next_event().unwrap().expect("host closed the connection") {
                Event::Started(j) => {
                    if j.get("keep").and_then(Json::as_bool) != Some(true) {
                        view = View::default();
                    }
                }
                Event::Font(f) => {
                    view.fonts.insert(f.id, f.key);
                }
                Event::Sources(s) => {
                    for (i, p) in s.files {
                        view.files.insert(i, p);
                    }
                    for (i, f, l) in s.spans {
                        view.spans.insert(i, (f, l));
                    }
                }
                Event::Page(p) => {
                    view.page_fonts.insert(p.index, view.fonts.clone());
                    view.pages.insert(p.index, p);
                }
                Event::Done(d) => dones.push(d),
                Event::Error(e) => panic!("host error: {e}"),
                _ => {}
            }
        }
        assert_eq!(dones[0].int_field("id"), Some(id1));
        assert_eq!(dones[1].int_field("id"), Some(id2));
        assert_eq!(dones[1].str_field("status"), Some("ok"), "{}", dones[1]);
        if dones[0].str_field("status") == Some("cancelled") {
            cancelled += 1;
        }
        let count = dones[1].int_field("pages").unwrap_or(0) as usize;
        view.count = count;
        view.pages.retain(|&i, _| (i as usize) < count);
        let (p2, o2) = snapshot(&base, &proj, &out, &format!("p{round}"));
        compare_with_scratch(
            &scratch.1,
            &view,
            &proj,
            &out,
            &p2,
            &o2,
            main,
            &format!("preempted round {round}"),
        );
    }
    assert!(cancelled > 0, "no compile was preempted");
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

/// LIVE-30MS: typing faster than the edited page arrives must not starve
/// the preview. After a compile that newer work stopped before it shipped a
/// changed page, the next one is not stopped before its changed page, and
/// sends it even when superseded (then stops, `cancelled`); and what the
/// client holds at the end still equals a from-scratch compile.
#[test]
fn a_superseded_compile_still_sends_its_edited_page() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-antistarve");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let main = "main.tex";
    std::fs::write(proj.join(main), article(40)).unwrap();
    let host = start_host("s");
    let scratch = start_host("t");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    for _ in 0..4 {
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
        if o.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    // A prose line about two thirds in.
    let text = String::from_utf8_lossy(&std::fs::read(proj.join(main)).unwrap()).into_owned();
    let lines: Vec<&str> = text.split('\n').collect();
    let prose: Vec<usize> = (0..lines.len())
        .filter(|&i| {
            let t = lines[i];
            t.split(' ').count() > 6 && !t.trim_start().starts_with('%') && !t.starts_with('\\')
        })
        .collect();
    let line = prose[prose.len() * 2 / 3];
    let (w, _) = middle_word(lines[line]).expect("a word");
    let offset: usize = lines[..line].iter().map(|l| l.len() + 1).sum::<usize>() + w;
    let mut sent_superseded = 0;
    for round in 0..5u64 {
        // three keystrokes, each sent as soon as the one before has started
        let ids: Vec<i64> = (1..=3).map(|n| id + n).collect();
        id += 3;
        let reqs: Vec<CompileRequest> = ids
            .iter()
            .enumerate()
            .map(|(n, &i)| {
                let mut r = req(i, &proj, &out, main);
                r.edits = vec![Edit {
                    path: main.into(),
                    offset: offset as u64 + (n as u64 + round) % 2,
                    delete: 0,
                    insert: ["xy", "z", "w"][n].into(),
                }];
                r
            })
            .collect();
        c.compile(&reqs[0]).unwrap();
        let (mut current, mut dones) = (0, vec![]);
        let mut pages: HashMap<i64, usize> = HashMap::new();
        while dones.len() < 3 {
            match c.next_event().unwrap().expect("host closed the connection") {
                Event::Started(j) => {
                    current = j.int_field("id").unwrap_or(0);
                    if let Some(n) = ids.iter().position(|&i| i == current) {
                        if n + 1 < reqs.len() {
                            c.compile(&reqs[n + 1]).unwrap();
                        }
                    }
                    if j.get("keep").and_then(Json::as_bool) != Some(true) {
                        view = View::default();
                    }
                }
                Event::Font(f) => {
                    view.fonts.insert(f.id, f.key);
                }
                Event::Sources(s) => {
                    for (i, p) in s.files {
                        view.files.insert(i, p);
                    }
                    for (i, f, l) in s.spans {
                        view.spans.insert(i, (f, l));
                    }
                }
                Event::Page(p) => {
                    *pages.entry(current).or_default() += 1;
                    view.page_fonts.insert(p.index, view.fonts.clone());
                    view.pages.insert(p.index, p);
                }
                Event::Done(d) => dones.push(d),
                Event::Error(e) => panic!("host error: {e}"),
                _ => {}
            }
        }
        assert_eq!(dones[2].int_field("id"), Some(ids[2]));
        assert_eq!(dones[2].str_field("status"), Some("ok"), "{}", dones[2]);
        // the first stopped before a page (starved): the second, superseded
        // too, still sent its changed page
        let cancelled = |d: &Json| d.str_field("status") == Some("cancelled");
        let started = |d: &Json| d.get("stages").is_some();
        if cancelled(&dones[0])
            && started(&dones[0])
            && pages.get(&ids[0]).copied().unwrap_or(0) == 0
            && cancelled(&dones[1])
            && started(&dones[1])
        {
            assert!(
                pages.get(&ids[1]).copied().unwrap_or(0) >= 1,
                "round {round}: the protected compile sent no page: {}",
                dones[1]
            );
            sent_superseded += 1;
        }
        let dones = [dones[0].clone(), dones[2].clone()];
        let count = dones[1].int_field("pages").unwrap_or(0) as usize;
        view.count = count;
        view.pages.retain(|&i, _| (i as usize) < count);
        let (p2, o2) = snapshot(&base, &proj, &out, &format!("s{round}"));
        compare_with_scratch(
            &scratch.1,
            &view,
            &proj,
            &out,
            &p2,
            &o2,
            main,
            &format!("superseded round {round}"),
        );
    }
    // The second keystroke is sent as soon as the first has started: the
    // first is stopped at its first checkpoint, long before its page, in
    // practice every round; one round at least, or the test proved nothing.
    eprintln!("{sent_superseded} of 5 rounds starved a compile and protected the next");
    assert!(
        sent_superseded >= 1,
        "no round starved a compile: the protection was not exercised"
    );
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

/// What a connection was sent of forms, and which forms its pages draw.
#[derive(Default)]
struct Forms {
    /// Form id -> its content hash, as last sent.
    sent: HashMap<u32, [u8; 32]>,
    /// Page index -> the forms its items draw (the last version sent).
    drawn: BTreeMap<u32, Vec<u32>>,
    /// Font ids sent, and the forms that arrived drawing a glyph of a font
    /// not sent before them (spec §5: every resource first).
    fonts: HashMap<u16, [u8; 32]>,
    unfonted: Vec<(u32, u16)>,
}

impl Forms {
    fn note(&mut self, ev: &Event) {
        let ids = |p: &Page| -> Vec<u32> {
            p.items
                .iter()
                .filter_map(|it| match it {
                    Item::Form { id, .. } => Some(*id),
                    _ => None,
                })
                .collect()
        };
        match ev {
            // (ids are valid until a STARTED says "keep": false, spec §5)
            Event::Started(j) if j.get("keep").and_then(Json::as_bool) != Some(true) => {
                *self = Forms::default()
            }
            Event::Font(f) => {
                self.fonts.insert(f.id, f.key);
            }
            Event::Form(f) => {
                self.sent.insert(f.index, f.hash);
                for it in &f.items {
                    if let Item::Glyph { font, .. } = it {
                        if !self.fonts.contains_key(font) {
                            self.unfonted.push((f.index, *font));
                        }
                    }
                }
            }
            Event::Page(p) => {
                self.drawn.insert(p.index, ids(p));
            }
            _ => {}
        }
    }

    /// Every form the pages below `count` draw was sent, and is the one a
    /// from-scratch compile sends under that id (`scratch`).
    fn check(&self, count: usize, scratch: &Forms, what: &str) {
        let mut drawn = 0;
        for (&page, ids) in self.drawn.range(..count as u32) {
            for id in ids {
                drawn += 1;
                let got = self.sent.get(id).unwrap_or_else(|| {
                    panic!("{what}: page {page} draws form {id}, which the host never sent")
                });
                assert_eq!(
                    Some(got),
                    scratch.sent.get(id),
                    "{what}: page {page}'s form {id} is not the scratch compile's"
                );
            }
        }
        assert!(drawn > 0, "{what}: no page draws a form");
        assert!(
            self.unfonted.is_empty(),
            "{what}: (form, font) drawn before the font was sent: {:?}",
            self.unfonted
        );
    }
}

/// One compile's events, noted in `forms`, up to its DONE.
fn compile_forms(c: &mut Client, forms: &mut Forms, req: &CompileRequest) -> Json {
    c.compile(req).unwrap();
    loop {
        let ev = c.next_event().unwrap().expect("host closed the connection");
        forms.note(&ev);
        match ev {
            Event::Done(d) => break d,
            Event::Error(e) => panic!("host error: {e}"),
            _ => {}
        }
    }
}

/// Spec §5: a `FORM` a page uses comes before that page or after it, but
/// before the next `PAGE` or `DONE`. A form the engine writes while no output
/// is wanted (its compile superseded) is cached, not sent; a page shipped
/// later that draws it must still get it. Here a run from the format is
/// superseded in its preamble, which writes a form at once
/// (`\immediate\pdfxform`, as pgf's shadings and fadings do); it stops past
/// S₀, and the next compile (nothing new) goes on with it and ships the
/// last page live: it draws the form, which no compile sent.
#[test]
fn a_page_shipped_after_a_superseded_preamble_gets_its_forms() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let mut doc = String::from(
        "\\documentclass{article}\n\
         % a while in the preamble before the form, so that the second\n\
         % COMPILE (sent at the first one's STARTED) supersedes the run first\n\
         \\newcount\\spin \\loop\\advance\\spin by 1 \\ifnum\\spin<400000 \\repeat\n\
         \\setbox0=\\hbox{\\vrule width 40pt height 12pt depth 3pt}\n\
         \\immediate\\pdfxform0\n\
         \\edef\\logo{\\noexpand\\pdfrefxform\\the\\pdflastxform\\relax}\n\
         \\begin{document}\n",
    );
    // (only the last page draws the form: a page the next compile delivers
    // from the cache, which the stopped run shipped, comes with its forms)
    for i in 0..3 {
        doc.push_str(&format!(
            "Page {i}: {}\\par\n{}\n\\newpage\n",
            if i == 2 { "\\logo" } else { "" },
            "Some words of the page, enough for a line or two. ".repeat(8)
        ));
    }
    doc.push_str("\\end{document}\n");
    let scratch = start_host("fs");
    let mut exercised = 0;
    for attempt in 0..3 {
        let base = common::fresh_dir(&format!("flashtex-host-forms-{attempt}"));
        let (proj, out) = (base.join("proj"), base.join("out"));
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(proj.join("main.tex"), &doc).unwrap();
        let host = start_host(&format!("f{attempt}"));
        let mut c = Client::connect(&host.1).unwrap();
        let mut forms = Forms::default();
        let (r1, r2) = (
            req(1, &proj, &out, "main.tex"),
            req(2, &proj, &out, "main.tex"),
        );
        c.compile(&r1).unwrap();
        let mut dones = vec![];
        while dones.len() < 2 {
            let ev = c.next_event().unwrap().expect("host closed the connection");
            forms.note(&ev);
            match ev {
                // the second as soon as the first has started (typing)
                Event::Started(j) if j.int_field("id") == Some(1) => c.compile(&r2).unwrap(),
                Event::Done(d) => dones.push(d),
                Event::Error(e) => panic!("host error: {e}"),
                _ => {}
            }
        }
        assert_eq!(dones[1].str_field("status"), Some("ok"), "{}", dones[1]);
        // The first ran (it has `stages`) and stopped before the last page:
        // the second shipped that page itself.
        if dones[0].str_field("status") == Some("cancelled")
            && dones[0].get("stages").is_some()
            && dones[0].int_field("typeset_pages").is_some_and(|n| n < 3)
        {
            exercised += 1;
        }
        let count = dones[1].int_field("pages").unwrap_or(0) as usize;
        assert_eq!(count, 3, "{}", dones[1]);
        let (p2, o2) = snapshot(&base, &proj, &out, "x");
        let mut s = Client::connect(&scratch.1).unwrap();
        let mut sf = Forms::default();
        let mut rs = CompileRequest::new(1, p2.to_str().unwrap(), "main.tex");
        rs.output_dir = Some(o2.to_str().unwrap().into());
        let sd = compile_forms(&mut s, &mut sf, &rs);
        assert_eq!(sd.str_field("mode"), Some("cold"), "{sd}");
        sf.check(count, &sf, "the scratch compile");
        forms.check(
            count,
            &sf,
            &format!("attempt {attempt} ({} then {})", dones[0], dones[1]),
        );
        let _ = s.bye();
        let _ = c.bye();
        let _ = std::fs::remove_dir_all(&base);
        if exercised > 0 {
            break;
        }
    }
    assert!(
        exercised > 0,
        "no attempt superseded the first compile before its last page: the case was not exercised"
    );
}

/// A persisted S₀ carries the forms its preamble wrote. pdfTeX writes an
/// `\immediate\pdfxform` at once (pgf's shadings and fadings: a beamer
/// theme, `\usetikzlibrary{fadings}`), so a host that opens the document
/// from a stored S₀, whose run starts after the preamble, never writes them;
/// its pages draw them all the same. It must send each before DONE (spec
/// §5), with the fonts its glyphs use, equal to a from-scratch compile's.
#[test]
fn a_reopen_from_a_persisted_s0_sends_the_preambles_forms() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-s0forms");
    let (proj, out, cache) = (base.join("proj"), base.join("out"), base.join("s0"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    // A rule, and a word in a font the body does not use, with its source
    // span (the preamble's line, named afresh by the new process).
    let mut doc = String::from(
        "\\documentclass{article}\n\
         \\setbox0=\\hbox{\\vrule width 40pt height 12pt depth 3pt}\n\
         \\immediate\\pdfxform0\n\
         \\edef\\rulemark{\\noexpand\\pdfrefxform\\the\\pdflastxform\\relax}\n\
         \\setbox0=\\hbox{\\normalfont\\bfseries Logo}\n\
         \\immediate\\pdfxform0\n\
         \\edef\\logo{\\noexpand\\pdfrefxform\\the\\pdflastxform\\relax}\n\
         \\begin{document}\n",
    );
    for i in 0..3 {
        doc.push_str(&format!(
            "Page {i}: \\rulemark\\ \\logo\\par\n{}\n\\newpage\n",
            "Some words of the page, enough for a line or two. ".repeat(8)
        ));
    }
    doc.push_str("\\end{document}\n");
    std::fs::write(proj.join("main.tex"), &doc).unwrap();
    let cache_env = [("FLASHTEX_S0_CACHE", cache.to_str().unwrap())];
    let stored = || {
        std::fs::read_dir(&cache).is_ok_and(|d| {
            d.flatten()
                .any(|e| e.path().extension().is_some_and(|x| x == "s0"))
        })
    };
    let mut cold = Forms::default();
    {
        let host = start_host_env("s0fa", &cache_env);
        let mut c = Client::connect(&host.1).unwrap();
        for id in 1..=4 {
            let d = compile_forms(&mut c, &mut cold, &req(id, &proj, &out, "main.tex"));
            assert_eq!(d.str_field("status"), Some("ok"), "{d}");
            if d.str_field("mode") == Some("unchanged") {
                break;
            }
        }
        // (the writer thread writes S₀ after DONE; a host that ends does
        // not wait for it)
        let deadline = Instant::now() + Duration::from_secs(120);
        while !stored() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = c.bye();
    }
    assert!(stored(), "no S0 was stored");
    cold.check(3, &cold, "the first host");
    assert_eq!(cold.sent.len(), 2, "the preamble's two forms");
    let host = start_host_env("s0fb", &cache_env);
    let mut c = Client::connect(&host.1).unwrap();
    let mut forms = Forms::default();
    let d = compile_forms(&mut c, &mut forms, &req(1, &proj, &out, "main.tex"));
    assert_eq!(d.str_field("status"), Some("ok"), "{d}");
    assert_eq!(
        d.str_field("mode"),
        Some("open"),
        "opened from the stored S0: {d}"
    );
    assert_eq!(d.int_field("pages"), Some(3), "{d}");
    forms.check(3, &cold, "opened from the stored S0");
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

/// Racy signatures (git's racy-clean rule; review of #1549): a source
/// changed to the same length within the modification-time tick in which
/// the run took its signature keeps the same signature. Simulated here on
/// any file system with a one-day tick (`FLASHTEX_RACY_MS`): the edit puts
/// the file's modification time back, as a coarse clock would leave it;
/// the host must still see the change (main said `unchanged`). The tick
/// must outlast the test: every signature the host takes meanwhile (a
/// content check keeps a fresh one) has to fall within it, as on a real
/// coarse clock, where the edit's own time is the tick it happens in. A
/// tick shorter than a slow (debug, loaded) run let a signature taken after
/// it hide the edit: a test artefact, not a racy case.
#[test]
fn a_same_size_edit_within_the_mtime_tick_is_seen() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-racy");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let main = "main.tex";
    let text = article(3);
    std::fs::write(proj.join(main), &text).unwrap();
    let tick = std::time::SystemTime::now();
    let set_mtime = |t: std::time::SystemTime| {
        std::fs::File::options()
            .write(true)
            .open(proj.join(main))
            .unwrap()
            .set_modified(t)
            .unwrap()
    };
    set_mtime(tick);
    let host = start_host_env("r", &[("FLASHTEX_RACY_MS", "86400000")]);
    let scratch = start_host("rs");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    for _ in 0..4 {
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
        if o.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    // the same length: one letter of a prose word replaced, in the body
    let at = text
        .rfind(" lorem ")
        .or_else(|| text.rfind(" the "))
        .expect("a word")
        + 1;
    let mut b = text.into_bytes();
    b[at] = if b[at] == b'q' { b'z' } else { b'q' };
    std::fs::write(proj.join(main), &b).unwrap();
    set_mtime(tick);
    id += 1;
    let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
    assert_ne!(
        o.done.str_field("mode"),
        Some("unchanged"),
        "the edit was not seen: {}",
        o.done
    );
    let (p2, o2) = snapshot(&base, &proj, &out, "r");
    compare_with_scratch(&scratch.1, &view, &proj, &out, &p2, &o2, main, "racy edit");
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

/// Another program rewrites the file after a compile that wrote it (review
/// of #1606): in place, the same length, the host's modification time put
/// back (a write within the file system's tick). The next compile, with no
/// edit, sees it (`system::KNOWN` holds the host's copy only within the
/// compile that wrote it): a letter replaced, and a space replaced by a LF.
#[test]
fn an_outside_rewrite_after_the_hosts_own_write_is_seen() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    for (variant, from, to) in [("letter", None, None), ("newline", Some(b' '), Some(b'\n'))] {
        let base = common::fresh_dir(&format!("flashtex-host-outside-{variant}"));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let (proj, out) = (base.join("proj"), base.join("out"));
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::create_dir_all(&out).unwrap();
        let main = "main.tex";
        let text = article(3);
        std::fs::write(proj.join(main), &text).unwrap();
        let host = start_host("o");
        let scratch = start_host("os");
        let mut c = Client::connect(&host.1).unwrap();
        let mut view = View::default();
        let mut id = 0;
        for _ in 0..4 {
            id += 1;
            let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
            if o.done.str_field("mode") == Some("unchanged") {
                break;
            }
        }
        // the host writes the file: one letter of a prose word replaced
        let at = text
            .find(" lorem ")
            .or_else(|| text.find(" the "))
            .expect("a word")
            + 1;
        id += 1;
        let mut r = req(id, &proj, &out, main);
        r.edits = vec![Edit {
            path: main.into(),
            offset: at as u64,
            delete: 1,
            insert: if text.as_bytes()[at] == b'q' {
                "z"
            } else {
                "q"
            }
            .into(),
        }];
        let o = compile(&mut c, &mut view, &r);
        assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
        // another program, within the tick: another byte, in place
        let mtime = std::fs::metadata(proj.join(main))
            .unwrap()
            .modified()
            .unwrap();
        let mut b = std::fs::read(proj.join(main)).unwrap();
        let w = text
            .rfind(" lorem ")
            .or_else(|| text.rfind(" the "))
            .expect("a word");
        assert!(w > at + 8);
        match (from, to) {
            (Some(f), Some(t)) => {
                assert_eq!(b[w], f);
                b[w] = t;
            }
            _ => b[w + 1] = if b[w + 1] == b'q' { b'z' } else { b'q' },
        }
        {
            use std::io::{Seek, Write};
            let mut f = std::fs::File::options()
                .write(true)
                .open(proj.join(main))
                .unwrap();
            f.seek(std::io::SeekFrom::Start(0)).unwrap();
            f.write_all(&b).unwrap();
            f.set_modified(mtime).unwrap();
        }
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
        assert_ne!(
            o.done.str_field("mode"),
            Some("unchanged"),
            "{variant}: the outside rewrite was not seen: {}",
            o.done
        );
        let (p2, o2) = snapshot(&base, &proj, &out, "o");
        compare_with_scratch(&scratch.1, &view, &proj, &out, &p2, &o2, main, variant);
        let _ = c.bye();
        let _ = std::fs::remove_dir_all(&base);
    }
}

/// FS-WATCH (lane T7-LAST12): the host's checks take the signatures of
/// files outside the document as the kernel's file events vouch for them
/// (`fswatch`). A package outside the document that changes during the
/// session must still be seen at the next compile: its content rewritten in
/// place to the same length with its time put back (in the preamble: S₀'s
/// key; in the body: the journal), and its directory replaced by another
/// holding another version (an event on a directory on the way, not on the
/// file).
#[test]
fn a_package_outside_the_document_that_changes_is_seen() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-fswatch");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let (proj, out, ext) = (base.join("proj"), base.join("out"), base.join("ext"));
    for d in [&proj, &out, &ext.join("tree")] {
        std::fs::create_dir_all(d).unwrap();
    }
    let main = "main.tex";
    let pre = ext.join("tree/pre.tex");
    let body = ext.join("tree/body.tex");
    std::fs::write(&pre, "\\def\\pre{AAAA}\n").unwrap();
    std::fs::write(&body, "Body: BBBB.\n").unwrap();
    let doc = format!(
        "\\documentclass{{article}}\n\\input{{{}}}\n\\begin{{document}}\nPreamble: \\pre.\n\n\\input{{{}}}\n\\end{{document}}\n",
        pre.display(),
        body.display()
    );
    std::fs::write(proj.join(main), doc).unwrap();
    let host = start_host("w");
    let scratch = start_host("ws");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    let settle = |c: &mut Client, view: &mut View, id: &mut i64| {
        for _ in 0..4 {
            *id += 1;
            let o = compile(c, view, &req(*id, &proj, &out, main));
            if o.done.str_field("mode") == Some("unchanged") {
                return;
            }
        }
    };
    settle(&mut c, &mut view, &mut id);
    // (the watcher serves the files outside the document: its counts)
    if cfg!(any(target_os = "linux", target_os = "macos")) {
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
        let served = o
            .done
            .get("stages")
            .and_then(|s| s.get("fswatch"))
            .and_then(|a| a.as_array())
            .and_then(|a| a.first())
            .and_then(|n| n.as_i64());
        assert!(served.is_some_and(|n| n > 0), "nothing served: {}", o.done);
    }
    // in place, the same length, the time put back
    let rewrite = |f: &Path, from: &str, to: &str| {
        let mtime = std::fs::metadata(f).unwrap().modified().unwrap();
        let t = std::fs::read_to_string(f).unwrap().replace(from, to);
        use std::io::Write;
        let mut h = std::fs::File::options().write(true).open(f).unwrap();
        h.write_all(t.as_bytes()).unwrap();
        h.set_modified(mtime).unwrap();
    };
    let steps: [(&str, &dyn Fn()); 3] = [
        ("the body's file rewritten", &|| {
            rewrite(&body, "BBBB", "CCCC")
        }),
        ("the preamble's file rewritten", &|| {
            rewrite(&pre, "AAAA", "DDDD")
        }),
        ("the directory replaced", &|| {
            std::fs::rename(ext.join("tree"), ext.join("old")).unwrap();
            std::fs::create_dir_all(ext.join("tree")).unwrap();
            std::fs::write(&pre, "\\def\\pre{EEEE}\n").unwrap();
            std::fs::write(&body, "Body: FFFF.\n").unwrap();
        }),
    ];
    for (k, (what, change)) in steps.iter().enumerate() {
        change();
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
        assert_ne!(
            o.done.str_field("mode"),
            Some("unchanged"),
            "{what}: not seen: {}",
            o.done
        );
        let (p2, o2) = snapshot(&base, &proj, &out, &format!("w{k}"));
        compare_with_scratch(&scratch.1, &view, &proj, &out, &p2, &o2, main, what);
        settle(&mut c, &mut view, &mut id);
    }
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

/// Lane FIND-PHASE: the document's own files, read relative to its
/// directory (`\input{chap/sec/body}`), are watched like TeX Live's
/// (`fswatch`), so that a keystroke does not `stat` and open each of them
/// again. A file under one of the document's directories that changes
/// between two compiles -- rewritten in place to the same length with its
/// time put back, another file renamed over it, its directory replaced, a
/// file looked for and missing made -- is still seen by the next compile,
/// with entries made in the document's directory meanwhile; with the
/// output directory apart, inside the document's directory, and with the
/// watcher off (`FLASHTEX_FSWATCH=off`: where there is none). No signature
/// is racy (`FLASHTEX_RACY_MS=0`), so a change is seen only if a check
/// looks at the file again: a stale signature served would miss it.
#[test]
fn a_file_under_the_documents_directory_that_changes_is_seen() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let scratch = start_host("dds");
    for (variant, off, out_in_proj) in [
        ("watched", false, false),
        ("output in the document's directory", false, true),
        ("watcher off", true, false),
    ] {
        let base = common::fresh_dir("flashtex-host-docdir");
        let _ = std::fs::remove_dir_all(&base);
        let proj = base.join("proj");
        let out = if out_in_proj {
            proj.clone()
        } else {
            base.join("out")
        };
        for d in [&proj, &out, &proj.join("chap/sec"), &proj.join("sty")] {
            std::fs::create_dir_all(d).unwrap();
        }
        let main = "main.tex";
        let (pre, body) = (proj.join("sty/pre.tex"), proj.join("chap/sec/body.tex"));
        let maybe = proj.join("chap/sec/maybe.tex");
        std::fs::write(&pre, "\\def\\pre{AAAA}\n").unwrap();
        std::fs::write(&body, "Body: BBBB.\n").unwrap();
        std::fs::write(
            proj.join(main),
            "\\documentclass{article}\n\\input{sty/pre}\n\\begin{document}\nPreamble: \\pre.\n\n\
             \\input{chap/sec/body}\n\n\\IfFileExists{chap/sec/maybe.tex}{\\input{chap/sec/maybe}}{No maybe.}\n\
             \\end{document}\n",
        )
        .unwrap();
        let mut env = vec![("FLASHTEX_RACY_MS", "0")];
        if off {
            env.push(("FLASHTEX_FSWATCH", "off"));
        }
        let host = start_host_env("dd", &env);
        let mut c = Client::connect(&host.1).unwrap();
        let mut view = View::default();
        let mut id = 0;
        let settle = |c: &mut Client, view: &mut View, id: &mut i64| {
            for _ in 0..4 {
                *id += 1;
                let o = compile(c, view, &req(*id, &proj, &out, main));
                if o.done.str_field("mode") == Some("unchanged") {
                    return;
                }
            }
        };
        settle(&mut c, &mut view, &mut id);
        // (the document's files are served by the watches: an unchanged
        // compile `stat`s fewer paths in its checks than it is served)
        let counts = |o: &Outcome| -> Option<(i64, i64)> {
            let a = o.done.get("stages")?.get("fswatch")?.as_array()?;
            Some((a.first()?.as_i64()?, a.get(1)?.as_i64()?))
        };
        if !off && cfg!(any(target_os = "linux", target_os = "macos")) {
            id += 1;
            let a = counts(&compile(&mut c, &mut view, &req(id, &proj, &out, main)));
            id += 1;
            let b = counts(&compile(&mut c, &mut view, &req(id, &proj, &out, main)));
            let (Some(a), Some(b)) = (a, b) else {
                panic!("{variant}: no fswatch counts");
            };
            let (served, statted) = (b.0 - a.0, b.1 - a.1);
            assert!(
                served > 0 && statted < served,
                "{variant}: an unchanged compile served {served} and made {statted} stats"
            );
        }
        let rewrite = |f: &Path, from: &str, to: &str| {
            let mtime = std::fs::metadata(f).unwrap().modified().unwrap();
            let t = std::fs::read_to_string(f).unwrap().replace(from, to);
            use std::io::Write;
            let mut h = std::fs::File::options().write(true).open(f).unwrap();
            h.write_all(t.as_bytes()).unwrap();
            h.set_modified(mtime).unwrap();
        };
        let steps: [(&str, &dyn Fn()); 5] = [
            ("the body's file rewritten in place", &|| {
                rewrite(&body, "BBBB", "CCCC")
            }),
            ("the preamble's file rewritten in place", &|| {
                rewrite(&pre, "AAAA", "DDDD")
            }),
            ("another file renamed over the body's", &|| {
                std::fs::write(proj.join("chap/sec/new.tex"), "Body: renamed over.\n").unwrap();
                std::fs::rename(proj.join("chap/sec/new.tex"), &body).unwrap();
            }),
            ("the body's directory replaced", &|| {
                std::fs::rename(proj.join("chap"), proj.join("chap-old")).unwrap();
                std::fs::create_dir_all(proj.join("chap/sec")).unwrap();
                std::fs::write(&body, "Body: EEEE.\n").unwrap();
            }),
            ("a file looked for made", &|| {
                std::fs::write(&maybe, "Maybe: FFFF.\n").unwrap();
            }),
        ];
        for (k, (what, change)) in steps.iter().enumerate() {
            // (entries made and removed in the document's directory, as
            // editors and the external tools make them)
            let junk = proj.join(format!("junk{k}.tmp"));
            std::fs::write(&junk, "j").unwrap();
            std::fs::remove_file(&junk).unwrap();
            change();
            id += 1;
            let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
            assert_ne!(
                o.done.str_field("mode"),
                Some("unchanged"),
                "{variant}: {what}: not seen: {}",
                o.done
            );
            let tag = format!("d{k}");
            let (p2, o2) = if out_in_proj {
                // (the scratch compile's output apart: its pages are the
                // same, the copied `.aux` read in either place)
                let (p2, o2) = (
                    base.join(format!("proj{tag}")),
                    base.join(format!("out{tag}")),
                );
                copy_dir(&proj, &p2);
                std::fs::create_dir_all(&o2).unwrap();
                (p2, o2)
            } else {
                snapshot(&base, &proj, &out, &tag)
            };
            compare_with_scratch(
                &scratch.1,
                &view,
                &proj,
                &out,
                &p2,
                &o2,
                main,
                &format!("{variant}: {what}"),
            );
            settle(&mut c, &mut view, &mut id);
        }
        let _ = c.bye();
        drop(host);
        let _ = std::fs::remove_dir_all(&base);
    }
}

/// BOX-MEMO revision 3 (BOX-MEMO.md §12.3): a call that enters control
/// sequences (`\csname` of names never made before) is recorded in an edit's
/// window and replayed at the next keystroke, which restored to before the
/// names were made: the replay enters them again, where the normal path
/// put them. Every page equals a scratch compile's, and the replay ran
/// (`stages.boxmemo`: hits and control sequences entered).
#[test]
fn a_replayed_call_enters_its_new_control_sequences_again() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-newcs");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let main = "main.tex";
    let text = "\\documentclass{article}\n\
        \\def\\mk#1{\\expandafter\\gdef\\csname mk@#1\\endcsname{#1!}%\n\
        \\expandafter\\global\\expandafter\\let\\csname mk@x@#1\\endcsname\\relax}\n\
        \\begin{document}\n\
        Words before the calls. \\mk{alpha}\\mk{beta}\\mk{gamma}Then\n\
        \\csname mk@alpha\\endcsname{} and \\csname mk@gamma\\endcsname.\n\
        \\end{document}\n";
    std::fs::write(proj.join(main), text).unwrap();
    // (on whatever the suite's `FLASHTEX_BOXMEMO` says: the test is of replays)
    let env = [("FLASHTEX_BOXMEMO", "on"), ("FLASHTEX_BOXMEMO_NAMES", "mk")];
    let host = start_host_env("nc", &env);
    let scratch = start_host_env("ncs", &env);
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    for _ in 0..4 {
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
        if o.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    let at = text.find("before").unwrap() as u64;
    let counts = |o: &Outcome| -> Vec<i64> {
        o.done
            .get("stages")
            .and_then(|s| s.get("boxmemo"))
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(|n| n.as_i64()).collect())
            .unwrap_or_default()
    };
    let mut last = vec![];
    for k in 0..4 {
        id += 1;
        let mut r = req(id, &proj, &out, main);
        r.edits = vec![Edit {
            path: main.into(),
            offset: at,
            delete: if k % 2 == 0 { 0 } else { 1 },
            insert: if k % 2 == 0 {
                "x".into()
            } else {
                String::new()
            },
        }];
        let o = compile(&mut c, &mut view, &r);
        assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
        let (p2, o2) = snapshot(&base, &proj, &out, &format!("nc{k}"));
        compare_with_scratch(
            &scratch.1,
            &view,
            &proj,
            &out,
            &p2,
            &o2,
            main,
            &format!("edit {k}"),
        );
        last = counts(&o);
    }
    // offers, hits, recordings, control sequences entered
    assert!(
        last.len() == 4 && last[1] > 0 && last[3] > 0,
        "no replay entered a control sequence: {last:?}"
    );
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

/// An absolute name the run looked for (review of #1549, (a)): its own
/// directory decides the answer, not the working directory, both before
/// S₀ (S₀'s key) and after it (the journal). A file appearing there is
/// seen (main: `unchanged`, the working directory being as it was).
#[test]
fn a_file_appearing_at_an_absolute_name_is_seen() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-absname");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let (proj, out, ext) = (base.join("proj"), base.join("out"), base.join("ext"));
    for d in [&proj, &out, &ext] {
        std::fs::create_dir_all(d).unwrap();
    }
    let main = "main.tex";
    let pre = ext.join("pre.tex");
    let body = ext.join("body.tex");
    let doc = format!(
        "\\documentclass{{article}}\n\\IfFileExists{{{}}}{{\\def\\pre{{yes}}}}{{\\def\\pre{{no}}}}\n\\begin{{document}}\nPreamble: \\pre.\n\n\\IfFileExists{{{}}}{{Body: yes.}}{{Body: no.}}\n\\end{{document}}\n",
        pre.display(),
        body.display()
    );
    std::fs::write(proj.join(main), doc).unwrap();
    let host = start_host("a");
    let scratch = start_host("as");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    for _ in 0..4 {
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
        if o.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    // the body's file appears (the journal's lookup), then the preamble's
    // (S₀'s key)
    for (k, f) in [&body, &pre].into_iter().enumerate() {
        std::fs::write(f, "% exists\n").unwrap();
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
        assert_ne!(
            o.done.str_field("mode"),
            Some("unchanged"),
            "{}: not seen: {}",
            f.display(),
            o.done
        );
        let (p2, o2) = snapshot(&base, &proj, &out, &format!("a{k}"));
        compare_with_scratch(
            &scratch.1,
            &view,
            &proj,
            &out,
            &p2,
            &o2,
            main,
            &format!("{} appeared", f.display()),
        );
    }
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

/// The directories of a LOOKUP-SKIP test (`lookup_steps`).
struct LookDirs {
    proj: PathBuf,
    out: PathBuf,
    /// Absolute names: `pre.tex` (the preamble's), `body.tex`.
    ext: PathBuf,
    /// `TEXMFHOME`'s `tex/latex/flashprobe`, and a deeper directory of the
    /// same `//` subtree.
    sub: PathBuf,
    deep: PathBuf,
}

impl LookDirs {
    /// Put `d`'s modification time back, well in the past and a different
    /// one each time: a changed signature that is not racy.
    fn set_back(&self, d: &Path) {
        static TICK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let k = TICK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let t = std::time::SystemTime::now() - Duration::from_secs(5000 - k);
        std::fs::File::open(d).unwrap().set_modified(t).unwrap();
    }
}

/// One step of `lookup_steps`: what it is, what it changes on disk, and an
/// edit of the document (from, to), if any.
type LookStep<'a> = (&'a str, &'a dyn Fn(&LookDirs), Option<(&'a str, &'a str)>);

/// LOOKUP-SKIP (`crate::lookupproof`): a document whose preamble and body
/// test for files in the places a lookup searches (`TEXMFHOME`'s `//`
/// subtree with no `ls-R`, absolute names, the project, the output
/// directory), compiled until it settles, then each step: its change on
/// disk, an entry more in the output and project directories (what a
/// compile's own writes do to them, so their signatures change at every
/// compile and the lookups are checked rather than skipped wholesale), a
/// compile, and every page compared with a scratch compile.
fn lookup_steps(tag: &str, steps: &[LookStep]) {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir(&format!("flashtex-host-look-{tag}"));
    let _ = std::fs::remove_dir_all(&base);
    let home = base.join("home");
    let d = LookDirs {
        proj: base.join("proj"),
        out: base.join("out"),
        ext: base.join("ext"),
        sub: home.join("tex/latex/flashprobe"),
        deep: home.join("tex/latex/deep/er"),
    };
    for x in [&d.proj, &d.out, &d.ext, &d.sub, &d.deep] {
        std::fs::create_dir_all(x).unwrap();
    }
    std::fs::write(d.sub.join("flashshadow.sty"), "Shadow: home.\n").unwrap();
    std::fs::write(d.sub.join("flashother.sty"), "% other\n").unwrap();
    for x in [
        &home,
        &home.join("tex"),
        &home.join("tex/latex"),
        &home.join("tex/latex/deep"),
        &d.sub,
        &d.deep,
    ] {
        d.set_back(x);
    }
    let filler: String = article(3)
        .split("\\section{Start}")
        .nth(1)
        .unwrap()
        .replace("\\end{document}\n", "");
    let doc = format!(
        "\\documentclass{{article}}\n\
         \\IfFileExists{{flashpre.sty}}{{\\def\\pre{{yes}}}}{{\\def\\pre{{no}}}}\n\
         \\IfFileExists{{{}}}{{\\def\\abspre{{yes}}}}{{\\def\\abspre{{no}}}}\n\
         \\begin{{document}}\nPreamble: \\pre, \\abspre.\n\n{filler}\n\
         \\IfFileExists{{flashprobe.sty}}{{Probe: yes.}}{{Probe: no.}}\n\n\
         \\IfFileExists{{flashdeep.sty}}{{Deep: yes.}}{{Deep: no.}}\n\n\
         \\InputIfFileExists{{flashshadow.sty}}{{}}{{Shadow: none.}}\n\n\
         \\IfFileExists{{{}}}{{Absolute: yes.}}{{Absolute: no.}}\n\n\
         \\InputIfFileExists{{flashout.tex}}{{}}{{Out: none.}}\n\n\
         \\IfFileExists{{flashcase.sty}}{{Case: yes.}}{{Case: no.}}\n\n\
         Closing words.\n\\end{{document}}\n",
        d.ext.join("pre.tex").display(),
        d.ext.join("body.tex").display()
    );
    let main = "main.tex";
    std::fs::write(d.proj.join(main), &doc).unwrap();
    let home_s = home.to_string_lossy().into_owned();
    let env = [("TEXMFHOME", home_s.as_str())];
    let host = start_host_env(tag, &env);
    let scratch = start_host_env(&format!("{tag}s"), &env);
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    for _ in 0..4 {
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &d.proj, &d.out, main));
        if o.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    let unrelated: LookStep = ("unrelated entries", &|_| {}, None);
    for (k, (what, change, edit)) in std::iter::once(&unrelated).chain(steps).enumerate() {
        change(&d);
        std::fs::write(d.out.join(format!("junk{k}.log")), "x").unwrap();
        std::fs::write(d.proj.join(format!("junk{k}.txt")), "x").unwrap();
        id += 1;
        let mut r = req(id, &d.proj, &d.out, main);
        if let Some((from, to)) = edit {
            let text = std::fs::read_to_string(d.proj.join(main)).unwrap();
            let at = text.find(from).unwrap();
            r.edits = vec![Edit {
                path: main.into(),
                offset: at as u64,
                delete: from.len() as u64,
                insert: (*to).into(),
            }];
        }
        let o = compile(&mut c, &mut view, &r);
        assert_eq!(o.done.str_field("status"), Some("ok"), "{what}: {}", o.done);
        let (p2, o2) = snapshot(&base, &d.proj, &d.out, &format!("{tag}{k}"));
        compare_with_scratch(&scratch.1, &view, &d.proj, &d.out, &p2, &o2, main, what);
        let _ = std::fs::remove_dir_all(&p2);
        let _ = std::fs::remove_dir_all(&o2);
    }
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

/// LOOKUP-SKIP, #1549's review case (a): a file appearing at an absolute
/// name, in the body (the journal's lookups) and the preamble (S₀'s key),
/// and going again. Its own directory decides the answer, not the working
/// or the output directory.
#[test]
fn lookup_skip_a_absolute_names() {
    lookup_steps(
        "la",
        &[
            (
                "(a) a file appeared at an absolute name",
                &|d| std::fs::write(d.ext.join("body.tex"), "% body\n").unwrap(),
                None,
            ),
            (
                "(a) one in the preamble",
                &|d| std::fs::write(d.ext.join("pre.tex"), "% pre\n").unwrap(),
                None,
            ),
            (
                "(a) both went",
                &|d| {
                    std::fs::remove_file(d.ext.join("body.tex")).unwrap();
                    std::fs::remove_file(d.ext.join("pre.tex")).unwrap();
                },
                None,
            ),
        ],
    );
}

/// LOOKUP-SKIP, #1549's review case (b): a file appearing in a `TEXMFHOME`
/// tree without `ls-R`, in the body and the preamble, an edit after the
/// lookups (the run restarts after them and keeps their answers), and the
/// files going again.
#[test]
fn lookup_skip_b_texmfhome() {
    lookup_steps(
        "lb",
        &[
            (
                "(b) a file appeared in TEXMFHOME",
                &|d| {
                    std::fs::write(d.sub.join("flashprobe.sty"), "% probe\n").unwrap();
                    d.set_back(&d.sub);
                },
                None,
            ),
            (
                "(b) an edit after the lookups",
                &|_| {},
                Some(("Closing words.", "Closing words, edited.")),
            ),
            (
                "(b) one in the preamble",
                &|d| {
                    std::fs::write(d.sub.join("flashpre.sty"), "% pre\n").unwrap();
                    d.set_back(&d.sub);
                },
                None,
            ),
            (
                "(b) both went",
                &|d| {
                    std::fs::remove_file(d.sub.join("flashprobe.sty")).unwrap();
                    std::fs::remove_file(d.sub.join("flashpre.sty")).unwrap();
                    d.set_back(&d.sub);
                },
                None,
            ),
        ],
    );
}

/// LOOKUP-SKIP, #1549's review case (c): a `//` subtree. A file appearing
/// in a deeper directory of it, and a file in the project (searched first)
/// shadowing the subtree's, and both going again.
#[test]
fn lookup_skip_c_subtrees() {
    lookup_steps(
        "lc",
        &[
            (
                "(c) a file appeared deep in the subtree",
                &|d| {
                    std::fs::write(d.deep.join("flashdeep.sty"), "% deep\n").unwrap();
                    d.set_back(&d.deep);
                },
                None,
            ),
            (
                "(c) the project shadows the subtree's file",
                &|d| std::fs::write(d.proj.join("flashshadow.sty"), "Shadow: project.\n").unwrap(),
                None,
            ),
            (
                "(c) both went",
                &|d| {
                    std::fs::remove_file(d.deep.join("flashdeep.sty")).unwrap();
                    d.set_back(&d.deep);
                    std::fs::remove_file(d.proj.join("flashshadow.sty")).unwrap();
                },
                None,
            ),
        ],
    );
}

/// LOOKUP-SKIP: the output directory, which `lookup_again` (texmfmp.c's
/// `open_input`) tries before kpathsea, and a name kpathsea finds only by
/// folding case; each appearing and going again.
#[test]
fn lookup_skip_output_directory_and_case() {
    lookup_steps(
        "lo",
        &[
            (
                "a file appeared in the output directory",
                &|d| std::fs::write(d.out.join("flashout.tex"), "Out: here.\n").unwrap(),
                None,
            ),
            (
                "a name differing in case appeared",
                &|d| std::fs::write(d.proj.join("FlashCase.STY"), "% case\n").unwrap(),
                None,
            ),
            (
                "both went",
                &|d| {
                    std::fs::remove_file(d.out.join("flashout.tex")).unwrap();
                    std::fs::remove_file(d.proj.join("FlashCase.STY")).unwrap();
                },
                None,
            ),
        ],
    );
}

/// LOOKUP-SKIP, #1549's review case (d): a directory's signature taken
/// within its modification-time tick (racy) proves nothing. A rename in the
/// project to a name a lookup tries keeps the directory's length and
/// identity, and the time is put back as a coarse clock would leave it
/// (`FLASHTEX_RACY_MS`, a one-day tick, as in
/// `a_same_size_edit_within_the_mtime_tick_is_seen`): only the racy rule
/// makes the check list the directory again. In the body (the journal) and
/// the preamble (S₀'s key).
#[test]
fn lookup_skip_d_a_rename_within_the_mtime_tick() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-look-ld");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(proj.join("flashrac0.sty"), "% body\n").unwrap();
    std::fs::write(proj.join("flashrap0.sty"), "% preamble\n").unwrap();
    let filler: String = article(2)
        .split("\\section{Start}")
        .nth(1)
        .unwrap()
        .replace("\\end{document}\n", "");
    let doc = format!(
        "\\documentclass{{article}}\n\
         \\IfFileExists{{flashrapy.sty}}{{\\def\\pre{{yes}}}}{{\\def\\pre{{no}}}}\n\
         \\begin{{document}}\nPreamble: \\pre.\n\n{filler}\n\
         \\IfFileExists{{flashracy.sty}}{{Body: yes.}}{{Body: no.}}\n\n\
         \\end{{document}}\n"
    );
    let main = "main.tex";
    std::fs::write(proj.join(main), &doc).unwrap();
    let host = start_host_env("ld", &[("FLASHTEX_RACY_MS", "86400000")]);
    let scratch = start_host("lds");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    for _ in 0..4 {
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
        if o.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    for (k, (from, to)) in [
        ("flashrac0.sty", "flashracy.sty"),
        ("flashrap0.sty", "flashrapy.sty"),
    ]
    .into_iter()
    .enumerate()
    {
        let before = std::fs::metadata(&proj).unwrap();
        std::fs::rename(proj.join(from), proj.join(to)).unwrap();
        std::fs::File::open(&proj)
            .unwrap()
            .set_modified(before.modified().unwrap())
            .unwrap();
        let after = std::fs::metadata(&proj).unwrap();
        assert_eq!(
            (before.len(), before.modified().unwrap()),
            (after.len(), after.modified().unwrap()),
            "the rename must keep the directory's length and time"
        );
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
        assert_ne!(
            o.done.str_field("mode"),
            Some("unchanged"),
            "{to}: not seen: {}",
            o.done
        );
        let (p2, o2) = snapshot(&base, &proj, &out, &format!("ld{k}"));
        compare_with_scratch(&scratch.1, &view, &proj, &out, &p2, &o2, main, to);
    }
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

/// #1295: pdfTeX frees an image's name once it has written the XObject; a
/// page that draws the image again after a restore (or in a later pass)
/// must still get an `IMAGE` naming the file.
#[test]
fn an_image_drawn_again_after_a_restore_names_its_file() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-image");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/images/png-rgb8.png"),
        proj.join("logo.png"),
    )
    .unwrap();
    // One XObject (a saved box), drawn on every page.
    let mut doc = String::from(
        "\\documentclass{article}\n\\usepackage{graphicx}\n\\newsavebox\\logo\n\
         \\begin{document}\n\\sbox\\logo{\\includegraphics[width=1cm]{logo}}\n",
    );
    for i in 1..=6 {
        doc.push_str(&format!("\\usebox\\logo\n\nPage {i} text.\n\n\\newpage\n"));
    }
    doc.push_str("\\end{document}\n");
    let main = "main.tex";
    std::fs::write(proj.join(main), &doc).unwrap();
    let host = start_host("img");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let o = compile(&mut c, &mut view, &req(1, &proj, &out, main));
    assert_eq!(o.done.str_field("status"), Some("ok"), "{:?}", o.done);
    // A word on page 4: the run restarts after page 3; the XObject was
    // written on page 1.
    let at = doc.find("Page 4 text").unwrap() + 5;
    let mut r = req(2, &proj, &out, main);
    r.edits = vec![Edit {
        path: main.into(),
        offset: at as u64,
        delete: 1,
        insert: "IV".into(),
    }];
    let o = compile(&mut c, &mut view, &r);
    assert_eq!(o.done.str_field("status"), Some("ok"), "{:?}", o.done);
    assert_ne!(o.done.str_field("mode"), Some("cold"), "{:?}", o.done);
    assert!(!view.images.is_empty(), "no IMAGE message");
    for m in &view.images {
        let file = m
            .str_field("file")
            .unwrap_or_else(|| panic!("IMAGE without a file: {m:?}"));
        assert!(file.ends_with("logo.png"), "{m:?}");
        assert_eq!(m.str_field("type"), Some("png"), "{m:?}");
    }
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

/// Protocol §5.2: a PDF image's `width`, `height`, `orig_x` and `orig_y` are
/// the included page's box in bp, as the PDF gives it, not pdfTeX's scaled
/// points (`bp2int`), which the host sent before (found by INFDESC-APP on
/// the textbook's title-page logo, #1407). `pdf-hand.pdf`'s CropBox, the
/// box `\includegraphics` takes by default, is `[5 5.5 195 95.25]`.
#[test]
fn a_pdf_image_box_is_sent_in_bp() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-pdfimg");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/images/pdf-hand.pdf"),
        proj.join("hand.pdf"),
    )
    .unwrap();
    let main = "main.tex";
    std::fs::write(
        proj.join(main),
        "\\documentclass{article}\n\\usepackage{graphicx}\n\\begin{document}\n\
         \\includegraphics{hand}\n\\end{document}\n",
    )
    .unwrap();
    let host = start_host("pdfimg");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let o = compile(&mut c, &mut view, &req(1, &proj, &out, main));
    assert_eq!(o.done.str_field("status"), Some("ok"), "{:?}", o.done);
    let m = view
        .images
        .iter()
        .find(|m| m.str_field("type") == Some("pdf"))
        .unwrap_or_else(|| panic!("no PDF IMAGE message: {:?}", view.images));
    let num = |k: &str| match m.get(k) {
        Some(Json::Num(f)) => *f,
        Some(Json::Int(i)) => *i as f64,
        other => panic!("{k}: {other:?} in {m:?}"),
    };
    for (k, want) in [
        ("orig_x", 5.0),
        ("orig_y", 5.5),
        ("width", 190.0),
        ("height", 89.75),
    ] {
        assert_eq!(num(k), want, "{k} in {m:?}");
    }
    assert_eq!(m.str_field("page_box"), Some("crop"), "{m:?}");
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

/// #1294: an `export` of the same job in the same directory rewrites the
/// output files the resident engine's checkpoints hold (the PDF compressed,
/// the log). The next compile must not restore on top of them: its PDF and
/// log must be the ones the same compiles give without the export.
#[test]
fn an_export_in_the_same_directory_leaves_the_next_compile_exact() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-exp2");
    let (proj, out) = (base.join("proj"), base.join("out"));
    let main = "main.tex";
    // Numbered items on every page, nothing in the `.aux` but the page
    // count: one more item on page 3 renumbers every later one (no
    // convergence, no second pass), and the restart has the PDF open and
    // shorter than the exported one.
    let mut doc = String::from(
        "\\documentclass{article}\n\\newcounter{x}\n\
         \\newcommand\\X{\\stepcounter{x}[\\arabic{x}] }\n\\begin{document}\n",
    );
    for p in 0..10 {
        for i in 0..6 {
            doc.push_str(&format!(
                "\\X Item {p}.{i}: {}\n\n",
                "some words to fill the line and the page ".repeat(6)
            ));
        }
        doc.push_str("\\newpage\n");
    }
    doc.push_str("\\end{document}\n");
    let first = "Item 2.0: ";
    let at = doc.find(first).unwrap();
    let insert = "\\X ".to_string();
    let run = |export: bool| -> (Vec<u8>, Vec<u8>, Json) {
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(proj.join(main), &doc).unwrap();
        let host = start_host("exp2");
        let mut c = Client::connect(&host.1).unwrap();
        let mut view = View::default();
        let mut id = 0;
        for _ in 0..4 {
            id += 1;
            let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
            if o.done.str_field("mode") == Some("unchanged") {
                break;
            }
        }
        if export {
            id += 1;
            let mut r = CompileRequest::new(id, proj.to_str().unwrap(), main);
            r.output_dir = Some(out.to_str().unwrap().into());
            r.export = true;
            let o = compile(&mut c, &mut view, &r);
            assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
            let pdf = std::fs::read(out.join("main.pdf")).unwrap();
            assert!(pdf.windows(12).any(|w| w == b"/FlateDecode"));
        }
        id += 1;
        let mut r = req(id, &proj, &out, main);
        r.edits = vec![Edit {
            path: main.into(),
            offset: at as u64,
            delete: 0,
            insert: insert.clone(),
        }];
        let o = compile(&mut c, &mut view, &r);
        let _ = c.bye();
        (
            std::fs::read(out.join("main.pdf")).unwrap(),
            std::fs::read(out.join("main.log")).unwrap(),
            o.done,
        )
    };
    let (pdf1, log1, done1) = run(false);
    let (pdf2, log2, done2) = run(true);
    assert_eq!(done1.str_field("status"), Some("ok"), "{done1}");
    assert_eq!(done2.str_field("status"), Some("ok"), "{done2}");
    assert!(
        pdf1 == pdf2,
        "the preview PDF after an export differs\n{done1}\n{done2}"
    );
    assert!(
        log1 == log2,
        "the log after an export differs\n{done1}\n{done2}"
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// Lane P4-TYPING-200WPM: a keystroke that undoes the one before (a letter
/// typed and deleted, typed again) finds nothing new against the run the
/// keystrokes stopped: that run goes on as its compile. The pages it had
/// shipped, the edited one among them, are the compile's at once: `PAGES`
/// says they are current before the run ships its next page. Pages that run
/// shipped while the newer compile was queued (which a superseded compile
/// does not send) may come first, as the catch-up of what the client lacks;
/// no page typeset after it goes on may. The client ends equal to a
/// from-scratch compile.
#[test]
fn a_keystroke_that_changes_nothing_has_its_page_at_once() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-continued");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let main = "main.tex";
    std::fs::write(proj.join(main), article(60)).unwrap();
    let host = start_host("u");
    let scratch = start_host("v");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    for _ in 0..4 {
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
        if o.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    // The first long prose line: a paragraph inserted there reflows the
    // rest, so the run after it goes on long after its edited page.
    let text = String::from_utf8_lossy(&std::fs::read(proj.join(main)).unwrap()).into_owned();
    let lines: Vec<&str> = text.split('\n').collect();
    let line = lines
        .iter()
        .position(|t| {
            t.split(' ').count() > 6 && !t.trim_start().starts_with('%') && !t.starts_with('\\')
        })
        .expect("a prose line");
    let offset: usize = lines[..line].iter().map(|l| l.len() + 1).sum();
    let this = lines[line];
    let para = format!("{this}\n\n{this}\n\n");
    let insert = |at: usize| Edit {
        path: main.into(),
        offset: at as u64,
        delete: 0,
        insert: para.clone(),
    };
    let delete = Edit {
        path: main.into(),
        offset: offset as u64,
        delete: para.len() as u64,
        insert: String::new(),
    };
    // A round shows the continuation only when r1 was still running when
    // r2 came, and r2 when r3 came (both cancelled, by their DONEs): the
    // client sends r2 and r3 as soon as r1's edited page arrives, but under
    // load r1 may finish first. The rounds go on, at least 3, until one has
    // shown it (at most 12): the outcome is read from the protocol's DONEs,
    // not from the timing of one round.
    let mut seen_current_first = 0;
    for round in 0..12 {
        if round >= 3 && seen_current_first > 0 {
            break;
        }
        let (id1, id2, id3) = (id + 1, id + 2, id + 3);
        id += 3;
        let mut r1 = req(id1, &proj, &out, main);
        r1.edits = vec![insert(offset)];
        c.compile(&r1).unwrap();
        // r1's edited page: the first page frame of r1
        let mut edited = None;
        let mut dones = vec![];
        while edited.is_none() && dones.is_empty() {
            match c.next_event().unwrap().expect("host closed the connection") {
                Event::Font(f) => {
                    view.fonts.insert(f.id, f.key);
                }
                Event::Sources(s) => {
                    for (i, p) in s.files {
                        view.files.insert(i, p);
                    }
                    for (i, f, l) in s.spans {
                        view.spans.insert(i, (f, l));
                    }
                }
                Event::Page(p) => {
                    edited = Some(p.index);
                    view.page_fonts.insert(p.index, view.fonts.clone());
                    view.pages.insert(p.index, p);
                }
                Event::Done(d) => dones.push(d),
                Event::Error(e) => panic!("host error: {e}"),
                _ => {}
            }
        }
        let edited = edited.expect("r1's edited page");
        // the delete and the insert again, right behind, as typing sends them
        let mut r2 = req(id2, &proj, &out, main);
        r2.edits = vec![delete.clone()];
        let mut r3 = req(id3, &proj, &out, main);
        r3.edits = vec![insert(offset)];
        c.compile(&r2).unwrap();
        c.compile(&r3).unwrap();
        // r3's frames up to its first `PAGES`: the pages sent before it,
        // and the ranges it says are current
        let mut started = 0;
        let mut r3_pages_before: Vec<u32> = vec![];
        let mut r3_current: Option<Vec<(i64, i64)>> = None;
        while dones.len() < 3 {
            match c.next_event().unwrap().expect("host closed the connection") {
                Event::Started(j) => started = j.int_field("id").unwrap_or(0),
                Event::Font(f) => {
                    view.fonts.insert(f.id, f.key);
                }
                Event::Sources(s) => {
                    for (i, p) in s.files {
                        view.files.insert(i, p);
                    }
                    for (i, f, l) in s.spans {
                        view.spans.insert(i, (f, l));
                    }
                }
                Event::Page(p) => {
                    if started == id3 && r3_current.is_none() {
                        r3_pages_before.push(p.index);
                    }
                    view.page_fonts.insert(p.index, view.fonts.clone());
                    view.pages.insert(p.index, p);
                }
                Event::Pages(j) => {
                    if j.int_field("id") == Some(id3) && r3_current.is_none() {
                        let ranges = j.get("current").and_then(Json::as_array).map(|rs| {
                            rs.iter()
                                .filter_map(|r| {
                                    let a = r.as_array()?;
                                    Some((a.first()?.as_i64()?, a.get(1)?.as_i64()?))
                                })
                                .collect()
                        });
                        r3_current = Some(ranges.unwrap_or_default());
                    }
                }
                Event::Done(d) => dones.push(d),
                Event::Error(e) => panic!("host error: {e}"),
                _ => {}
            }
        }
        assert_eq!(dones[2].int_field("id"), Some(id3));
        assert_eq!(dones[2].str_field("status"), Some("ok"), "{}", dones[2]);
        // (when r1 had finished before r2 came, r3 is an ordinary compile)
        if dones[0].str_field("status") == Some("cancelled")
            && dones[1].str_field("status") == Some("cancelled")
        {
            // Before r3's first `PAGES`, the host may send pages the stopped
            // run shipped while a newer compile was queued (a superseded
            // compile sends no pages: `Target::quiet`), which the client
            // lacks (`Live::catch_up`): pages that run typeset, from the
            // edited page on, `typeset_pages` of them.
            let typeset = |d: &Json| d.int_field("typeset_pages").unwrap_or(0).max(0);
            let shipped = typeset(&dones[0]).max(typeset(&dones[1])) as u32;
            let current = r3_current.clone().unwrap_or_default();
            let covers = |i: u32| {
                current
                    .iter()
                    .any(|&(lo, hi)| lo <= i as i64 && i as i64 <= hi)
            };
            let caught_up = r3_current.is_some()
                && covers(edited)
                && r3_pages_before
                    .iter()
                    .all(|&i| i >= edited && i < edited + shipped && covers(i));
            if dones[2].str_field("mode") == Some("continued") {
                // The run r1 started goes on: no page it typesets after going
                // on comes before the `PAGES` that says the edited page is
                // current.
                assert!(
                    caught_up,
                    "round {round}: r3 sent pages {r3_pages_before:?} before its first \
                     `PAGES` {:?}; r1 had shipped {shipped} from page {edited}",
                    r3_current
                );
                seen_current_first += 1;
            } else {
                // An ordinary compile (the stopped run settled or abandoned):
                // its first page frame is the edited page, unless it caught up
                // first as above.
                assert!(
                    caught_up || r3_pages_before.first() == Some(&edited),
                    "round {round}: r3 ({}) sent pages {r3_pages_before:?} before its first \
                     `PAGES` {:?}",
                    dones[2].str_field("mode").unwrap_or("?"),
                    r3_current
                );
            }
        }
        let count = dones[2].int_field("pages").unwrap_or(0) as usize;
        view.count = count;
        view.pages.retain(|&i, _| (i as usize) < count);
        let (p2, o2) = snapshot(&base, &proj, &out, &format!("c{round}"));
        compare_with_scratch(
            &scratch.1,
            &view,
            &proj,
            &out,
            &p2,
            &o2,
            main,
            &format!("continued round {round}"),
        );
        // back to the base text for the next round
        let mut r4 = req(id + 1, &proj, &out, main);
        id += 1;
        r4.edits = vec![delete.clone()];
        compile(&mut c, &mut view, &r4);
    }
    assert!(
        seen_current_first > 0,
        "no keystroke found its page current at once (in 12 rounds)"
    );
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

/// The display list's side table at a convergence (BEAMER-LATENCY, #1319):
/// one source line long enough to fill three pages, and an edit at its
/// start that changes no glyph (a second space after its first word) but
/// moves the column of every later character of the line. The pages it
/// re-typesets equal the old ones, and the convergence test compares the
/// state, where the rest of the paragraph still waits to be shipped. Those
/// nodes' source positions (in `dl_side`) were dead for the test: the run
/// converged and kept the old run's next pages, whose glyphs named the old
/// columns. A live node's position is compared now, so the run goes on
/// until the line is shipped, and every page equals a from-scratch compile.
#[test]
fn a_kept_page_never_names_a_moved_column() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-cols");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let doc = article(8);
    let cut = doc.find("\n\n").unwrap() + 2;
    // one line, about three pages of words
    let words: Vec<String> = (0..1800).map(|i| format!("word{}", i % 97)).collect();
    let long = format!("Begin {}.\n\n", words.join(" "));
    let text = format!("{}{}{}", &doc[..cut], long, &doc[cut..]);
    std::fs::write(proj.join("main.tex"), &text).unwrap();
    let host = start_host("c");
    let scratch = start_host("cs");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    for _ in 0..4 {
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, "main.tex"));
        if o.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    let at = text.find("Begin ").unwrap() + "Begin ".len();
    for (k, (delete, insert)) in [(0, " "), (1, "")].into_iter().enumerate() {
        id += 1;
        let mut r = req(id, &proj, &out, "main.tex");
        r.edits = vec![Edit {
            path: "main.tex".into(),
            offset: at as u64,
            delete,
            insert: insert.into(),
        }];
        let o = compile(&mut c, &mut view, &r);
        let what = if k == 0 {
            "a second space"
        } else {
            "its revert"
        };
        assert_eq!(
            o.done.str_field("mode"),
            Some("incremental"),
            "{what}: {}",
            o.done
        );
        let (p2, o2) = snapshot(&base, &proj, &out, &k.to_string());
        compare_with_scratch(&scratch.1, &view, &proj, &out, &p2, &o2, "main.tex", what);
    }
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

/// A chapter of the focus test's book: a `\chapter` with a label, a
/// reference to the next chapter (its number and page come from the
/// `.aux`), and text for a few pages.
fn focus_chapter(n: usize, of: usize) -> String {
    let words = [
        "alpha", "beta", "gamma", "delta", "kernel", "glue", "penalty", "boxes", "rules", "marks",
    ];
    let mut s = format!(
        "\\chapter{{Chapter {n}}}\\label{{ch:{n}}}\nSee Chapter~\\ref{{ch:{m}}} on page~\\pageref{{ch:{m}}}.\n\n",
        m = n % of + 1
    );
    for p in 0..18 {
        let line: Vec<&str> = (0..80)
            .map(|w| words[(n * 7 + p * 3 + w) % words.len()])
            .collect();
        s.push_str(&line.join(" "));
        s.push_str(".\n\n");
    }
    s
}

/// A drawn page: `\count0` and every glyph as (font key, code, x, y).
type Drawn = (i32, Vec<([u8; 32], u16, i32, i32)>);

/// The pages whose spans come from `file`, in page order, as a reader sees them.
fn pages_of(view: &View, file: &str) -> Vec<Drawn> {
    let suffix = format!("/{file}");
    view.pages
        .iter()
        .filter(|(_, p)| {
            p.items.iter().any(|it| {
                matches!(it, Item::Span(s) if *s != 0
                    && view.spans.get(s).is_some_and(|&(f, _)| view.files[&f].ends_with(&suffix)))
            })
        })
        .map(|(i, p)| {
            let fonts = &view.page_fonts[i];
            let glyphs = p
                .items
                .iter()
                .filter_map(|it| match it {
                    Item::Glyph {
                        font, code, x, y, ..
                    } => Some((fonts[font], *code, *x, *y)),
                    _ => None,
                })
                .collect();
            (p.counts[0], glyphs)
        })
        .collect()
}

/// Chapter focus (lane FOCUS-CHAPTER): a COMPILE with `includeonly` is
/// pdflatex's run of `\AtBeginDocument{\includeonly{chapters/ch2}}\input
/// main.tex`, in an output folder started from the whole document's (the
/// app's `out-focus`): only that chapter is typeset, and its pages are the
/// whole document's own (page numbers, references and positions from the
/// other chapters' `.aux`). An edit in the focused chapter is incremental
/// (S₀ does not depend on the chapter: `\includeonly`'s lookup is after
/// it) and equals a from-scratch compile of the focused job, and dropping
/// the focus gives the whole document again, equal to a from-scratch
/// compile.
#[test]
fn a_focused_compile_is_includeonly_and_keeps_the_documents_pages() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-focus");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out, outf) = (base.join("proj"), base.join("out"), base.join("out-focus"));
    std::fs::create_dir_all(proj.join("chapters")).unwrap();
    // The project's folders in the output folder, as the app's copy makes
    // them (and latexmk): pdfTeX cannot create `chapters/` for `ch1.aux`.
    std::fs::create_dir_all(out.join("chapters")).unwrap();
    let of = 3;
    let mut main = String::from("\\documentclass{report}\n\\begin{document}\n\\tableofcontents\n");
    for n in 1..=of {
        std::fs::write(
            proj.join(format!("chapters/ch{n}.tex")),
            focus_chapter(n, of),
        )
        .unwrap();
        main.push_str(&format!("\\include{{chapters/ch{n}}}\n"));
    }
    main.push_str("\\end{document}\n");
    std::fs::write(proj.join("main.tex"), &main).unwrap();
    let host = start_host("f");
    let scratch = start_host("fs");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();

    // The whole document, settled.
    let o = compile(&mut c, &mut view, &req(1, &proj, &out, "main.tex"));
    assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
    let full_count = view.count;
    let full_ch2 = pages_of(&view, "chapters/ch2.tex");
    assert!(
        full_ch2.len() >= 2,
        "chapter 2 spans {} pages",
        full_ch2.len()
    );
    assert!(!pages_of(&view, "chapters/ch1.tex").is_empty());

    // Focused on chapter 2, from a copy of the whole document's output.
    copy_dir(&out, &outf);
    let focus = Some(vec!["chapters/ch2".to_string()]);
    let mut r = req(2, &proj, &outf, "main.tex");
    r.includeonly = focus.clone();
    let o = compile(&mut c, &mut view, &r);
    assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
    assert_eq!(
        o.started.get("keep").and_then(Json::as_bool),
        Some(false),
        "another job"
    );
    let argv = o.started.get("argv").and_then(Json::as_array).unwrap();
    assert_eq!(
        argv.last().and_then(Json::as_str),
        Some("\\AtBeginDocument{\\includeonly{chapters/ch2}}\\input main.tex"),
        "{}",
        o.started
    );
    assert!(
        view.count < full_count,
        "{} pages focused, {full_count} whole",
        view.count
    );
    assert!(
        pages_of(&view, "chapters/ch1.tex").is_empty(),
        "chapter 1 typeset"
    );
    assert!(
        pages_of(&view, "chapters/ch3.tex").is_empty(),
        "chapter 3 typeset"
    );
    assert!(
        pages_of(&view, "chapters/ch2.tex") == full_ch2,
        "the focused chapter's pages are not the whole document's"
    );

    // An edit in the focused chapter equals a from-scratch focused compile.
    let ch2 = std::fs::read_to_string(proj.join("chapters/ch2.tex")).unwrap();
    let at = ch2.find("kernel").unwrap();
    let mut r = req(3, &proj, &outf, "main.tex");
    r.includeonly = focus.clone();
    r.edits = vec![Edit {
        path: "chapters/ch2.tex".into(),
        offset: at as u64,
        delete: "kernel".len() as u64,
        insert: "colonel".into(),
    }];
    let o = compile(&mut c, &mut view, &r);
    assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
    assert_eq!(
        o.done.str_field("mode"),
        Some("incremental"),
        "an edit in the focused chapter keeps S0: {}",
        o.done
    );
    let (p2, o2) = snapshot(&base, &proj, &outf, "e");
    compare_with_scratch_job(
        &scratch.1,
        &view,
        &proj,
        &outf,
        &p2,
        &o2,
        "main.tex",
        focus,
        "an edit in the focused chapter",
    );

    // The whole document again, with the edit.
    let o = compile(&mut c, &mut view, &req(4, &proj, &out, "main.tex"));
    assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
    assert_eq!(
        o.started.get("keep").and_then(Json::as_bool),
        Some(false),
        "another job"
    );
    assert_eq!(view.count, full_count);
    let (p3, o3) = snapshot(&base, &proj, &out, "w");
    compare_with_scratch(
        &scratch.1,
        &view,
        &proj,
        &out,
        &p3,
        &o3,
        "main.tex",
        "the whole document after a focus",
    );
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

/// A file tested with `\openin`/`\closein` on page 1 and `\input` pages
/// later (`incr::Session::restart_point`): the checkpoints between the test
/// and the `\input` are not restart points for an edit in the file, those in
/// it before the edit are. Each edit restarts after the gap, not at S₀, and
/// every page equals a from-scratch compile.
#[test]
fn an_edit_after_an_early_openin_test_restarts_near_it() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-openin");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let words = [
        "alpha", "beta", "gamma", "delta", "kernel", "glue", "penalty", "boxes", "rules", "marks",
    ];
    let paras = |seed: usize, n: usize| -> String {
        (0..n)
            .map(|p| {
                let l: Vec<&str> = (0..70)
                    .map(|w| words[(seed * 7 + p * 3 + w) % words.len()])
                    .collect();
                l.join(" ") + "."
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    std::fs::write(
        proj.join("main.tex"),
        format!(
            "\\documentclass{{article}}\n\\newread\\probe\n\\begin{{document}}\n\
             \\openin\\probe=ch2.tex \\ifeof\\probe No chapter.\\else Chapter found.\\fi \\closein\\probe\n\
             {}\n\\input ch2.tex\n{}\n\\end{{document}}\n",
            paras(1, 30),
            paras(3, 6)
        ),
    )
    .unwrap();
    let ch2 = format!("\\section{{Two}}\n{}\n", paras(2, 30));
    std::fs::write(proj.join("ch2.tex"), &ch2).unwrap();
    let host = start_host("o");
    let scratch = start_host("os");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let o = compile(&mut c, &mut view, &req(1, &proj, &out, "main.tex"));
    assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
    // the pages before ch2.tex's first one: the gap after the \openin test
    let before = pages_of(&view, "ch2.tex").first().map(|p| p.0).unwrap();
    assert!(before > 3, "ch2.tex starts on page {before}");
    let mut id = 1;
    // Each edit and its revert, at positions across the chapter (where the
    // search lands depends on how many checkpoints the gap and the chapter
    // hold; near the chapter's start it landed in the gap without the
    // second search).
    for (k, at) in [
        ch2.len() / 12,
        ch2.len() / 6,
        ch2.len() / 2,
        ch2.len() * 5 / 6,
    ]
    .into_iter()
    .enumerate()
    {
        let at = at + ch2[at..].find(" kernel ").unwrap() + 1;
        for (step, (delete, insert)) in [("kernel", "colonel"), ("colonel", "kernel")]
            .into_iter()
            .enumerate()
        {
            id += 1;
            let mut r = req(id, &proj, &out, "main.tex");
            r.edits = vec![Edit {
                path: "ch2.tex".into(),
                offset: at as u64,
                delete: delete.len() as u64,
                insert: insert.into(),
            }];
            let o = compile(&mut c, &mut view, &r);
            assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
            let restart = o.done.int_field("restart_page").unwrap_or(0);
            assert!(
                restart >= before as i64 - 1,
                "edit {k}.{step}: restarted on page {restart}, ch2.tex starts on {before}: {}",
                o.done
            );
            let (p2, o2) = snapshot(&base, &proj, &out, &format!("o{k}{step}"));
            compare_with_scratch(
                &scratch.1,
                &view,
                &proj,
                &out,
                &p2,
                &o2,
                "main.tex",
                "an edit after an early \\openin test",
            );
        }
    }
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

/// A package installed with no host running, which the preamble's
/// `\IfFileExists` did not find before, is found by a full run; a host
/// opened from the stored S₀ (whose key's lookups are made again only
/// where something they depend on changed) must find it too. Into
/// `TEXMFHOME` (searched on disk) and into `TEXMFLOCAL` (an `!!` tree, found
/// through its `ls-R`, which the installer updates).
#[test]
fn a_persisted_s0_finds_a_package_installed_since() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let ls_r = |files: &[&str]| {
        format!(
            "% ls-R -- filename database for kpathsea; do not change this line.\n\
             ./:\ntex\n\n./tex:\nlatex\n\n./tex/latex:\n{}",
            files.iter().map(|f| format!("{f}\n")).collect::<String>()
        )
    };
    for (case, var, db) in [("home", "TEXMFHOME", false), ("local", "TEXMFLOCAL", true)] {
        let base = common::fresh_dir(&format!("flashtex-host-s0-lookup-{case}"));
        let _ = std::fs::remove_dir_all(&base);
        let (proj, out, cache, tree) = (
            base.join("proj"),
            base.join("out"),
            base.join("s0"),
            base.join("texmf"),
        );
        for d in [&proj, &out, &tree.join("tex/latex")] {
            std::fs::create_dir_all(d).unwrap();
        }
        std::fs::write(tree.join("tex/latex/flashtexother.sty"), "\\endinput\n").unwrap();
        if db {
            std::fs::write(tree.join("ls-R"), ls_r(&["flashtexother.sty"])).unwrap();
        }
        std::fs::write(
            proj.join("main.tex"),
            "\\documentclass{article}\n\
             \\IfFileExists{flashtexprobe.sty}{\\def\\probe{found}}{\\def\\probe{missing}}\n\
             \\begin{document}\nThe probe package is \\probe.\n\\end{document}\n",
        )
        .unwrap();
        let env = [
            ("FLASHTEX_S0_CACHE", cache.to_str().unwrap()),
            (var, tree.to_str().unwrap()),
        ];
        {
            let host = start_host_env(&format!("a{case}"), &env);
            let mut c = Client::connect(&host.1).unwrap();
            let mut view = View::default();
            for id in 1..=4 {
                let o = compile(&mut c, &mut view, &req(id, &proj, &out, "main.tex"));
                if o.done.str_field("mode") == Some("unchanged") {
                    break;
                }
            }
            let deadline = Instant::now() + Duration::from_secs(120);
            while !stored(&cache, "s0") && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
            let _ = c.bye();
        }
        assert!(stored(&cache, "s0"), "{case}: no S0 was stored");
        // installed with no host running
        std::fs::write(
            tree.join("tex/latex/flashtexprobe.sty"),
            "\\ProvidesPackage{flashtexprobe}\n",
        )
        .unwrap();
        if db {
            std::fs::write(
                tree.join("ls-R"),
                ls_r(&["flashtexother.sty", "flashtexprobe.sty"]),
            )
            .unwrap();
        }
        let host = start_host_env(&format!("b{case}"), &env);
        let scratch = start_host_env(&format!("s{case}"), &env[1..]);
        let mut c = Client::connect(&host.1).unwrap();
        let mut view = View::default();
        for id in 1..=4 {
            let o = compile(&mut c, &mut view, &req(id, &proj, &out, "main.tex"));
            if o.done.str_field("mode") == Some("unchanged") {
                break;
            }
        }
        let (p2, o2) = snapshot(&base, &proj, &out, "x");
        compare_with_scratch(
            &scratch.1,
            &view,
            &proj,
            &out,
            &p2,
            &o2,
            "main.tex",
            &format!("{case}: a package installed since S0 was stored"),
        );
        let _ = c.bye();
        let _ = std::fs::remove_dir_all(&base);
    }
}

/// Review of #1739: a host opened from a persisted S₀ (`--s0-cache`) has
/// no checkpoint before S₀, so the whole reads between `\document`'s start
/// and the anchor (`host::whole_after_arm`, left out of S₀'s key) are
/// checked when it opens (`Key::check_arm`). A `begindocument/before` hook
/// keeps (A) the `.aux`'s size, a `\label` added; (B) the main file's size,
/// a letter typed; (D) a file's date, the file touched; (F) a file's MD5, the
/// file edited -- each with no host running. A new host from the cache
/// equals a scratch compile; for B, D and F it does not open the stored S₀.
#[test]
fn a_persisted_s0_checks_the_reads_at_the_document_start() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let body: String = (0..14)
        .map(|i| {
            format!(
                "Paragraph {i} with enough words to fill a few lines of the page, so that \
                 the document ships more than one page; math $a^{i}+b$.\n\n"
            )
        })
        .collect();
    let doc = |hook: &str, extra: &str| {
        format!(
            "\\documentclass{{article}}\n\\AddToHook{{begindocument/before}}{{{hook}}}\n\
             \\begin{{document}}\n\\section{{One}}\\label{{one}}\n{body}{extra}\
             Kept: \\x. See~\\ref{{one}}.\n\\end{{document}}\n"
        )
    };
    let t0 = std::time::UNIX_EPOCH + Duration::from_secs(1_600_000_000);
    let t1 = std::time::UNIX_EPOCH + Duration::from_secs(1_600_000_500);
    let touch = |p: &Path, t: std::time::SystemTime| {
        std::fs::File::options()
            .write(true)
            .open(p)
            .unwrap()
            .set_modified(t)
            .unwrap()
    };
    // (case, the hook, the body's edit, a side file and its new text, the
    // stored S₀ refused)
    type Case<'a> = (&'a str, &'a str, &'a str, Option<(&'a str, &'a str)>, bool);
    let cases: [Case; 4] = [
        (
            "A",
            "\\xdef\\x{\\pdffilesize{\\jobname.aux}}",
            "\\section{Two}\\label{two}\n",
            None,
            false,
        ),
        (
            "B",
            "\\xdef\\x{\\pdffilesize{\\jobname.tex}}",
            "A letter: x.\n",
            None,
            true,
        ),
        (
            "D",
            "\\xdef\\x{\\pdffilemoddate{stamp.tex}}",
            "",
            Some(("stamp.tex", "% a file whose date is read\n")),
            true,
        ),
        (
            "F",
            "\\xdef\\x{\\pdfmdfivesum file {stamp.tex}}",
            "",
            Some(("stamp.tex", "% a file whose digest is read, edited\n")),
            true,
        ),
    ];
    for (case, hook, extra, side, refused) in cases {
        let base = common::fresh_dir(&format!("flashtex-host-arm-{case}"));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let (proj, out, cache) = (base.join("proj"), base.join("out"), base.join("s0"));
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(proj.join("main.tex"), doc(hook, "")).unwrap();
        std::fs::write(proj.join("stamp.tex"), "% a file whose date is read\n").unwrap();
        touch(&proj.join("stamp.tex"), t0);
        let cache_env = [("FLASHTEX_S0_CACHE", cache.to_str().unwrap())];
        // a whole stored S0 (not the writer's `.tmp`)
        let stored = || {
            std::fs::read_dir(&cache).is_ok_and(|d| {
                d.flatten()
                    .any(|e| e.path().extension().is_some_and(|x| x == "s0"))
            })
        };
        {
            let host = start_host_env(&format!("a{case}"), &cache_env);
            let mut c = Client::connect(&host.1).unwrap();
            let mut view = View::default();
            for id in 1..=4 {
                let o = compile(&mut c, &mut view, &req(id, &proj, &out, "main.tex"));
                if o.done.str_field("mode") == Some("unchanged") {
                    break;
                }
            }
            // The writer thread (`host::s0write`) writes S0 after DONE, and
            // a host that ends does not wait for it; so wait here (a
            // generous bound, for a loaded runner) before the host is killed.
            let deadline = Instant::now() + Duration::from_secs(120);
            while !stored() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
            let _ = c.bye();
        }
        assert!(stored(), "{case}: no S0 was stored");
        // the edit, with no host running
        std::fs::write(proj.join("main.tex"), doc(hook, extra)).unwrap();
        if let Some((name, text)) = side {
            std::fs::write(proj.join(name), text).unwrap();
            touch(&proj.join(name), t1);
        }
        let host = start_host_env(&format!("b{case}"), &cache_env);
        let scratch = start_host(&format!("s{case}"));
        let mut c = Client::connect(&host.1).unwrap();
        let mut view = View::default();
        let o = compile(&mut c, &mut view, &req(1, &proj, &out, "main.tex"));
        if refused {
            assert_ne!(o.done.str_field("mode"), Some("open"), "{case}: {}", o.done);
        }
        for id in 2..=4 {
            let o = compile(&mut c, &mut view, &req(id, &proj, &out, "main.tex"));
            if o.done.str_field("mode") == Some("unchanged") {
                break;
            }
        }
        let (p2, o2) = snapshot(&base, &proj, &out, "x");
        touch(&p2.join("stamp.tex"), if side.is_some() { t1 } else { t0 });
        let what = format!("{case}: opened from the stored S0");
        compare_with_scratch(&scratch.1, &view, &proj, &out, &p2, &o2, "main.tex", &what);
        let _ = c.bye();
        let _ = std::fs::remove_dir_all(&base);
    }
}

/// A second pdflatex.fmt, another format than [`fmt_dir`]'s: made from an
/// ini file that lets `\flashtexformatmark` be `\relax` and then reads pdflatex.ini
/// (a format rebuilt after a TeX Live update, as the app's format cache
/// stores it: under another name, `flashtex_runtime::formats`).
fn fmt_dir_marked() -> PathBuf {
    let fmt = common::fresh_dir("flashtex-host-fmt-marked");
    let _ = std::fs::remove_dir_all(&fmt);
    std::fs::create_dir_all(&fmt).unwrap();
    std::fs::write(
        fmt.join("flashtexmarked.ini"),
        // (INITEX has no braces yet: `\let`)
        "\\let\\flashtexformatmark\\relax \\input pdflatex.ini\n",
    )
    .unwrap();
    let st = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
        .args([
            "-ini",
            "-jobname=pdflatex",
            "-progname=pdflatex",
            "-etex",
            "-translate-file=cp227.tcx",
            "flashtexmarked.ini",
        ])
        .current_dir(&fmt)
        .env("FLASHTEX_POOL", pool())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(st.success(), "building the marked pdflatex.fmt failed");
    fmt
}

/// The format is not among what a persisted S₀ is checked for: a host
/// started with another pdflatex.fmt (TeX Live updated, the format cache
/// rebuilt it under a new name and kept the old file) opened the stored
/// S₀ of the old format and showed what the old format typesets. A host
/// with the new format refuses the stored S₀ and equals a scratch compile
/// with the new format.
#[test]
fn a_persisted_s0_is_refused_under_another_format() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-s0-format");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out, cache) = (base.join("proj"), base.join("out"), base.join("s0"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(
        proj.join("main.tex"),
        "\\documentclass{article}\n\\begin{document}\n\
         Made with format \\ifdefined\\flashtexformatmark B\\else A\\fi.\n\\end{document}\n",
    )
    .unwrap();
    let cache_env = [("FLASHTEX_S0_CACHE", cache.to_str().unwrap())];
    {
        let host = start_host_env("fa", &cache_env);
        let mut c = Client::connect(&host.1).unwrap();
        let mut view = View::default();
        for id in 1..=4 {
            let o = compile(&mut c, &mut view, &req(id, &proj, &out, "main.tex"));
            if o.done.str_field("mode") == Some("unchanged") {
                break;
            }
        }
        // (the writer thread writes S0 after DONE; a host that ends does
        // not wait for it)
        let deadline = Instant::now() + Duration::from_secs(120);
        while !stored(&cache, "s0") && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = c.bye();
    }
    assert!(stored(&cache, "s0"), "no S0 was stored");
    let marked = fmt_dir_marked();
    let fmt_env = [
        ("FLASHTEX_S0_CACHE", cache.to_str().unwrap()),
        ("FLASHTEX_FORMATS", marked.to_str().unwrap()),
    ];
    let host = start_host_env("fb", &fmt_env);
    let scratch = start_host_env("fs", &fmt_env[1..]);
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let o = compile(&mut c, &mut view, &req(1, &proj, &out, "main.tex"));
    assert_ne!(
        o.done.str_field("mode"),
        Some("open"),
        "the stored S0 of another format was opened: {}",
        o.done
    );
    for id in 2..=4 {
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, "main.tex"));
        if o.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    let (p2, o2) = snapshot(&base, &proj, &out, "x");
    compare_with_scratch(
        &scratch.1,
        &view,
        &proj,
        &out,
        &p2,
        &o2,
        "main.tex",
        "opened under another format",
    );
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
    let _ = std::fs::remove_dir_all(&marked);
}

/// T7-CONV (T7's plain-10 letter@start, #1850): a letter typed on the line
/// of the document's first inline math, after the math. amsmath's
/// `\every@math@size` sets `\strutbox@` globally there, so the box lives
/// in every later state with that line's source position, and an edited
/// line has a new span in every run: compared as span ids, the old run's
/// entry never equalled the new run's and no compile converged. Compared
/// as places (file, line, column) they are equal, the run converges after
/// the edited page, and every page still equals a from-scratch compile;
/// a letter typed before the math (which moves the box's column) and its
/// revert, too.
#[test]
fn an_edited_line_converges_with_a_box_set_on_it() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-strutbox");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let text = article(8);
    std::fs::write(proj.join("main.tex"), &text).unwrap();
    let host = start_host("sb");
    let scratch = start_host("sbs");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    for _ in 0..4 {
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, "main.tex"));
        if o.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    let math = text.find("\\frac{a}{b}$ ").unwrap() + "\\frac{a}{b}$ ".len();
    let (after, before) = (math, text.find(" with $x_").unwrap());
    for (k, (at, delete, insert, what, converges)) in [
        (after, 0, "x", "a letter after the math", true),
        (after, 1, "", "its revert", true),
        (before, 0, "x", "a letter before the math", false),
        (before, 1, "", "its revert", false),
    ]
    .into_iter()
    .enumerate()
    {
        id += 1;
        let mut r = req(id, &proj, &out, "main.tex");
        r.edits = vec![Edit {
            path: "main.tex".into(),
            offset: at as u64,
            delete,
            insert: insert.into(),
        }];
        let o = compile(&mut c, &mut view, &r);
        assert_eq!(
            o.done.str_field("mode"),
            Some("incremental"),
            "{what}: {}",
            o.done
        );
        if converges {
            let pages = o.done.int_field("pages").unwrap_or(0);
            let at = o.done.int_field("converged_at");
            assert!(
                at.is_some_and(|p| p < pages),
                "{what}: no convergence before the last page: {}",
                o.done
            );
        }
        let (p2, o2) = snapshot(&base, &proj, &out, &format!("sb{k}"));
        compare_with_scratch(&scratch.1, &view, &proj, &out, &p2, &o2, "main.tex", what);
    }
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

// ---------------------------------------------------------------------------
// S₁: a persisted body checkpoint at the last viewed page (lane COLD-OPEN,
// `host::s1`)

/// The viewport of the S₁ tests (0-based): S₁ is taken a page or two before.
const S1_VIEW: u32 = 12;

/// A document for the S₁ tests: a table of contents, a forward reference,
/// and a file read (and closed) on the first page.
fn s1_doc() -> (String, String) {
    let main = article(18)
        .replacen(
            "\\begin{document}\n",
            "\\begin{document}\n\\tableofcontents\nSee page~\\pageref{last}.\n\\input{side}\n",
            1,
        )
        .replacen(
            "\\end{document}\n",
            "\\section{Last}\\label{last}\nThe end.\n\\end{document}\n",
            1,
        );
    let side = "\\section{Side}\nA side file, read and closed on the first page; see \
                Section~\\ref{last}.\n\n"
        .to_string();
    (main, side)
}

/// The cache holds a whole stored file with extension `ext` (not a
/// writer's `.tmp`).
fn stored(cache: &Path, ext: &str) -> bool {
    std::fs::read_dir(cache).is_ok_and(|d| {
        d.flatten()
            .any(|e| e.path().extension().is_some_and(|x| x == ext))
    })
}

/// Compile until a compile finds nothing changed, at the viewport; the
/// first compile's outcome.
fn s1_compile_until_unchanged(
    c: &mut Client,
    view: &mut View,
    proj: &Path,
    out: &Path,
    id: &mut i64,
) -> Outcome {
    let mut first = None;
    for _ in 0..6 {
        *id += 1;
        let mut r = req(*id, proj, out, "main.tex");
        r.viewport = Some(S1_VIEW);
        let o = compile(c, view, &r);
        assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
        let unchanged = o.done.str_field("mode") == Some("unchanged");
        if first.is_none() {
            first = Some(o);
        }
        if unchanged {
            break;
        }
    }
    first.unwrap()
}

/// Store S₀ and S₁: a host with the cache opens the document at the
/// viewport, compiles until nothing changes, and is left idle until both
/// are written.
fn s1_store(proj: &Path, out: &Path, cache: &Path, name: &str) {
    s1_store_env(proj, out, cache, name, &[]);
}

/// [`s1_store`] with more of the host's environment.
fn s1_store_env(proj: &Path, out: &Path, cache: &Path, name: &str, env: &[(&str, &str)]) {
    if let Err(why) = s1_try_store(proj, out, cache, name, env) {
        panic!("no S1 was stored: {why}");
    }
}

/// [`s1_store_env`], or why the host stored no S₁ (`no_s1`, DONE's
/// `s1_save`).
fn s1_try_store(
    proj: &Path,
    out: &Path,
    cache: &Path,
    name: &str,
    env: &[(&str, &str)],
) -> Result<(), String> {
    let mut cache_env = vec![("FLASHTEX_S0_CACHE", cache.to_str().unwrap())];
    cache_env.extend_from_slice(env);
    let (tx, notes) = std::sync::mpsc::channel();
    let host = start_host_lines(name, &cache_env, Some(tx));
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    s1_compile_until_unchanged(&mut c, &mut view, proj, out, &mut id);
    // S₀ after the first (cold) compile, S₁ once the host is idle (2 s): the
    // host says when S₁ is written (`saved_s1`) or why it is not (`no_s1`),
    // so the wait is on that, not on a time (a loaded runner took over the
    // 120 s once allowed here)
    let deadline = Instant::now() + Duration::from_secs(900);
    let mut said = None;
    while said.is_none() && Instant::now() < deadline {
        match notes.recv_timeout(Duration::from_millis(200)) {
            Ok(l) if l.contains("\"saved_s1\"") => said = Some(Ok(l)),
            Ok(l) if l.contains("\"no_s1\"") => said = Some(Err(l)),
            Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    if let Some(Err(l)) = said {
        let _ = c.bye();
        return Err(l);
    }
    while !stored(cache, "s0") && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    if !stored(cache, "s1") {
        // (why: the next DONE says how the last save went, `s1_save`)
        id += 1;
        let mut r = req(id, proj, out, "main.tex");
        r.viewport = Some(S1_VIEW);
        let o = compile(&mut c, &mut view, &r);
        let _ = c.bye();
        return Err(o
            .done
            .get("s1_save")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "no save was tried".into()));
    }
    let _ = c.bye();
    Ok(())
}

/// The PDF and log a from-scratch compile gives in the same directories
/// (so that the PDF's /ID, made from its name, is the same): a host without
/// a cache, the output directory emptied first.
fn s1_scratch_pdf_log(proj: &Path, out: &Path) -> (Vec<u8>, Vec<u8>) {
    let _ = std::fs::remove_dir_all(out);
    std::fs::create_dir_all(out).unwrap();
    let host = start_host("s1p");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    for _ in 0..6 {
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, proj, out, "main.tex"));
        if o.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    let _ = c.bye();
    (
        std::fs::read(out.join("main.pdf")).unwrap(),
        std::fs::read(out.join("main.log")).unwrap(),
    )
}

/// `compare_with_scratch`, for the pages from `from` on only (a reopen
/// from S₁ with the fill off has not typeset the ones before).
#[allow(clippy::too_many_arguments)]
fn s1_compare_from(
    sock: &Path,
    view: &View,
    proj: &Path,
    out: &Path,
    p2: &Path,
    o2: &Path,
    from: u32,
    what: &str,
) {
    let mut s = Client::connect(sock).unwrap();
    let mut sv = View::default();
    let mut rs = CompileRequest::new(1, p2.to_str().unwrap(), "main.tex");
    rs.output_dir = Some(o2.to_str().unwrap().into());
    let so = compile(&mut s, &mut sv, &rs);
    assert_eq!(
        so.done.str_field("mode"),
        Some("cold"),
        "{what}: {}",
        so.done
    );
    assert_eq!(sv.count, view.count, "{what}: page count");
    for i in from..view.count as u32 {
        let a = resolve(view, i, proj, out);
        let b = resolve(&sv, i, p2, o2);
        if a != b {
            panic!(
                "{what}: page {i} differs from the scratch compile: {}",
                describe_difference(&a, &b)
            );
        }
    }
    let _ = s.bye();
}

/// What a case does to the stored state before the reopen.
#[derive(Clone, Copy, Debug)]
enum S1Change {
    Nothing,
    /// A file read and closed before S₁.
    Side,
    /// The main file before S₁'s position in it, and after.
    MainEarly,
    MainLate,
    /// The `.aux` and the `.toc` the run read before S₁.
    Aux,
    Toc,
    /// The PDF on disk: a byte of its first page changed, or cut there.
    PdfByte,
    PdfShort,
    /// S₁ saved by another engine build (its key's build).
    Build,
    /// The stored S₁ file cut short, or its magic changed (lane TEST-GAPS:
    /// `host::s1::File::read`'s refusals).
    S1Short,
    S1Magic,
}

/// Lane COLD-OPEN: a reopen from a persisted S₁ (`host::s1`). A host with
/// an S₀ cache opens the document at a viewport, compiles it until nothing
/// changes and, idle, stores S₀ and S₁ (the page checkpoint before the
/// viewport). Then, with no host running, each case changes something (or
/// nothing) and a new host reopens the document at the viewport: from S₁
/// where S₁ is still what a full run reaches (nothing changed, or the main
/// file after S₁'s position), else from S₀ (`s1_error` says why). Every page
/// the client holds equals a from-scratch compile's, and so do the PDF and
/// the log, byte for byte; an unchanged reopen ships the viewport page
/// first, and its fill of the pages before S₁ converges with S₁'s run.
#[test]
fn a_reopen_from_a_stored_s1_equals_a_scratch_compile() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-s1");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out, cache) = (base.join("proj"), base.join("out"), base.join("s0"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let (main, side) = s1_doc();
    std::fs::write(proj.join("main.tex"), &main).unwrap();
    std::fs::write(proj.join("side.tex"), &side).unwrap();
    s1_store(&proj, &out, &cache, "s1a");
    let cache_env = [("FLASHTEX_S0_CACHE", cache.to_str().unwrap())];
    // the stored state, put back before each case (the same directories:
    // the cache's names and the PDF's /ID depend on them)
    let bak = base.join("bak");
    for d in ["proj", "out", "s0"] {
        copy_dir(&base.join(d), &bak.join(d));
    }
    let scratch = start_host("s1s");
    let early = main.find("Start}\n\n").unwrap() + "Start}\n\n".len();
    let late = main.find("\\section{Last}").unwrap();
    let cases: [(S1Change, bool, Option<&str>); 11] = [
        (S1Change::Nothing, true, None),
        (S1Change::MainLate, true, None),
        (S1Change::Side, false, Some("side.tex changed")),
        (S1Change::MainEarly, false, Some("main.tex changed")),
        (S1Change::Aux, false, Some("main.aux changed")),
        (S1Change::Toc, false, Some("main.toc changed")),
        (
            S1Change::PdfByte,
            false,
            Some("is not the one S1 was taken with"),
        ),
        (
            S1Change::PdfShort,
            false,
            Some("is not the one S1 was taken with"),
        ),
        (S1Change::Build, false, Some("another engine build")),
        (S1Change::S1Short, false, Some("S1 file truncated")),
        (S1Change::S1Magic, false, Some("not an S1 file")),
    ];
    for (k, (change, from_s1, why)) in cases.into_iter().enumerate() {
        let what = format!("{change:?}");
        for d in ["proj", "out", "s0"] {
            let _ = std::fs::remove_dir_all(base.join(d));
            copy_dir(&bak.join(d), &base.join(d));
        }
        let s1_file = std::fs::read_dir(&cache)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|x| x == "s1"))
            .unwrap();
        let edit = |at: usize, text: &str| {
            let mut m = main.clone();
            m.insert_str(at, text);
            std::fs::write(proj.join("main.tex"), m).unwrap();
        };
        let append = |p: &Path, text: &str| {
            let mut d = std::fs::read(p).unwrap();
            d.extend_from_slice(text.as_bytes());
            std::fs::write(p, d).unwrap();
        };
        match change {
            S1Change::Nothing => {}
            S1Change::Side => std::fs::write(
                proj.join("side.tex"),
                side.replace("A side file", "A side file, edited"),
            )
            .unwrap(),
            S1Change::MainEarly => edit(early, "An early sentence, typed with no host. "),
            S1Change::MainLate => edit(late, "A late paragraph, typed with no host.\n\n"),
            // (the same length: S₀'s check of the `.aux`'s size at the
            // document's start holds, S₁'s of its content does not)
            S1Change::Aux => {
                let p = out.join("main.aux");
                let mut d = std::fs::read(&p).unwrap();
                let tag = b"\\newlabel{last}{{";
                let i = d.windows(tag.len()).position(|w| w == tag).unwrap() + tag.len();
                d[i] = if d[i] == b'9' { b'1' } else { b'9' };
                std::fs::write(&p, d).unwrap();
            }
            S1Change::Toc => append(
                &out.join("main.toc"),
                "\\contentsline {section}{\\numberline {99}Fake}{1}%\n",
            ),
            S1Change::PdfByte => {
                let p = out.join("main.pdf");
                let mut d = std::fs::read(&p).unwrap();
                let i = 1500;
                d[i] = if d[i] == b' ' { b'\n' } else { b' ' };
                std::fs::write(&p, d).unwrap();
            }
            S1Change::PdfShort => {
                let p = out.join("main.pdf");
                let d = std::fs::read(&p).unwrap();
                std::fs::write(&p, &d[..2000]).unwrap();
            }
            S1Change::Build => {
                // the key's build, after the header length, the magic and
                // the S₀ tag (`host::s1::Head`)
                let mut d = std::fs::read(&s1_file).unwrap();
                let at = 8 + 8 + b"flashtex S1 v1".len() + 16;
                d[at] ^= 1;
                std::fs::write(&s1_file, d).unwrap();
            }
            S1Change::S1Short => {
                // (the header whole, the chunks after it not)
                let d = std::fs::read(&s1_file).unwrap();
                let hlen = u64::from_le_bytes(d[..8].try_into().unwrap()) as usize;
                std::fs::write(&s1_file, &d[..8 + hlen]).unwrap();
            }
            S1Change::S1Magic => {
                // (a letter of the magic, after the header length and the
                // magic's own length)
                let mut d = std::fs::read(&s1_file).unwrap();
                d[8 + 8] ^= 0x20;
                std::fs::write(&s1_file, d).unwrap();
            }
        }
        let host = start_host_env(&format!("s1b{k}"), &cache_env);
        let mut c = Client::connect(&host.1).unwrap();
        let mut view = View::default();
        let mut id = 0;
        let o = s1_compile_until_unchanged(&mut c, &mut view, &proj, &out, &mut id);
        let s1_pages = o.done.int_field("s1_pages");
        assert_eq!(o.done.str_field("mode"), Some("open"), "{what}: {}", o.done);
        if from_s1 {
            let n = s1_pages.unwrap_or_else(|| panic!("{what}: S1 not used: {}", o.done));
            assert!(
                (4..=S1_VIEW as i64).contains(&n),
                "{what}: S1 at {n}: {}",
                o.done
            );
            // S₁'s pages first, up to the viewport page, the rest after
            let at = o.order.iter().position(|&p| p == S1_VIEW);
            assert!(
                o.order.first() == Some(&(n as u32)) && at == Some((S1_VIEW as i64 - n) as usize),
                "{what}: order {:?}",
                o.order
            );
            if matches!(change, S1Change::Nothing) {
                assert!(
                    o.done.int_field("s1_fill_converged").is_some(),
                    "{what}: the fill did not converge: {}",
                    o.done
                );
            }
        } else {
            assert_eq!(s1_pages, None, "{what}: S1 used: {}", o.done);
            let e = o.done.str_field("s1_error").unwrap_or_default();
            assert!(
                why.is_some_and(|w| e.contains(w)),
                "{what}: refused for another reason: {e:?}"
            );
        }
        let (p2, o2) = snapshot(&base, &proj, &out, &format!("s1x{k}"));
        compare_with_scratch(&scratch.1, &view, &proj, &out, &p2, &o2, "main.tex", &what);
        let (pdf, log) = (
            std::fs::read(out.join("main.pdf")).unwrap(),
            std::fs::read(out.join("main.log")).unwrap(),
        );
        let _ = c.bye();
        drop(host);
        let (pdf0, log0) = s1_scratch_pdf_log(&proj, &out);
        assert!(
            pdf == pdf0,
            "{what}: the PDF differs from a scratch compile's"
        );
        assert!(
            log == log0,
            "{what}: the log differs from a scratch compile's"
        );
        let _ = std::fs::remove_dir_all(&p2);
        let _ = std::fs::remove_dir_all(&o2);
    }
    let _ = std::fs::remove_dir_all(&base);
}

/// Lane COLD-OPEN: BOX-MEMO records only in an edit's window, in a session
/// reopened from S₁ too. Its open from S₀ left the window unset, which
/// records every registered call, and its fill ran as an edit's first pass,
/// whose window stays open until a changed page (none, in a fill). The
/// replays of those recordings made the fill's state another than S₁'s
/// writer's (cells allocated where it had them free), so on *Infinite
/// Descent ×2* the fill of an S₁ stored by a cold session never converged
/// and ran to the end, and its recording made it half as slow again. With a
/// registered macro called before S₁ and after it, the reopen records
/// nothing, its fill converges, and the pages, the PDF and the log equal a
/// scratch compile's.
#[test]
fn a_reopen_from_s1_records_no_calls_and_its_fill_converges() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-s1m");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out, cache) = (base.join("proj"), base.join("out"), base.join("s0"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let (main, side) = s1_doc();
    let main = main
        .replacen(
            "\\begin{document}\n",
            "\\def\\mk#1{\\expandafter\\gdef\\csname mk@#1\\endcsname{#1!}}\n\
             \\begin{document}\n\\mk{alpha}\n",
            1,
        )
        .replacen("The end.\n", "\\mk{omega}The end.\n", 1);
    std::fs::write(proj.join("main.tex"), &main).unwrap();
    std::fs::write(proj.join("side.tex"), &side).unwrap();
    let memo = [("FLASHTEX_BOXMEMO", "on"), ("FLASHTEX_BOXMEMO_NAMES", "mk")];
    s1_store_env(&proj, &out, &cache, "s1ma", &memo);
    let mut env = vec![("FLASHTEX_S0_CACHE", cache.to_str().unwrap())];
    env.extend_from_slice(&memo);
    let host = start_host_env("s1mb", &env);
    let scratch = start_host("s1ms");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    let o = s1_compile_until_unchanged(&mut c, &mut view, &proj, &out, &mut id);
    assert!(
        o.done.int_field("s1_pages").is_some(),
        "S1 not used: {}",
        o.done
    );
    assert!(
        o.done.int_field("s1_fill_converged").is_some(),
        "the fill did not converge: {}",
        o.done
    );
    // (offers, hits, recordings committed, control sequences entered)
    let recorded = o
        .done
        .get("stages")
        .and_then(|s| s.get("boxmemo"))
        .and_then(|a| a.as_array())
        .and_then(|a| a.get(2).and_then(|n| n.as_i64()));
    assert_eq!(recorded, Some(0), "the reopen recorded calls: {}", o.done);
    let (p2, o2) = snapshot(&base, &proj, &out, "m1");
    compare_with_scratch(
        &scratch.1,
        &view,
        &proj,
        &out,
        &p2,
        &o2,
        "main.tex",
        "the reopen",
    );
    let (pdf, log) = (
        std::fs::read(out.join("main.pdf")).unwrap(),
        std::fs::read(out.join("main.log")).unwrap(),
    );
    let _ = c.bye();
    drop(host);
    let (pdf0, log0) = s1_scratch_pdf_log(&proj, &out);
    assert!(pdf == pdf0, "the PDF differs from a scratch compile's");
    assert!(log == log0, "the log differs from a scratch compile's");
    let _ = std::fs::remove_dir_all(&base);
}

/// Lane COLD-OPEN with lane BEAMER's kept instances: a file written before
/// S₁, read after it and written again later holds on disk, when S₁ is
/// stored, the later instance (beamer's `.vrb` is rewritten so at each
/// fragile frame). The reopen must put back the one S₁ stands for, which
/// the run kept at its next `\openout` (`system::instances_at`), or not use
/// S₁. Either way, the pages, the PDF and the log equal a scratch compile's.
#[test]
fn a_reopen_from_s1_puts_back_a_file_rewritten_after_it() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-s1w");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out, cache) = (base.join("proj"), base.join("out"), base.join("s0"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let (main, side) = s1_doc();
    let main = main
        .replacen(
            "\\begin{document}\n",
            "\\newwrite\\tw\n\
             \\def\\stamp#1{\\immediate\\openout\\tw=\\jobname.stp \
             \\immediate\\write\\tw{Stamp #1.}\\immediate\\closeout\\tw}\n\
             \\begin{document}\n\\stamp{early}\n",
            1,
        )
        .replacen(
            "\\section{Last}",
            "\\InputIfFileExists{\\jobname.stp}{}{No stamp.}\\stamp{late}\n\\section{Last}",
            1,
        );
    std::fs::write(proj.join("main.tex"), &main).unwrap();
    std::fs::write(proj.join("side.tex"), &side).unwrap();
    // The writer's own checkpoints between the two opens are restorable only
    // with the instance at them kept (#1907), which a pass whose history
    // came through a reattach and a jump can lack (timing: where the pass
    // restarted). Then no S₁ is stored, for that reason, and the reopen runs
    // from S₀; its pages must equal scratch all the same.
    let stored_s1 = match s1_try_store(&proj, &out, &cache, "s1wa", &[]) {
        Ok(()) => true,
        Err(why) => {
            assert!(
                why.contains("instance there not kept"),
                "no S1 was stored, for another reason: {why}"
            );
            false
        }
    };
    assert_eq!(
        std::fs::read_to_string(out.join("main.stp"))
            .unwrap()
            .trim(),
        "Stamp late.",
        "the stored state's file is the later instance"
    );
    let cache_env = [("FLASHTEX_S0_CACHE", cache.to_str().unwrap())];
    let host = start_host_env("s1wb", &cache_env);
    let scratch = start_host("s1ws");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    let o = s1_compile_until_unchanged(&mut c, &mut view, &proj, &out, &mut id);
    if o.done.int_field("s1_pages").is_some() {
        assert!(
            o.done.int_field("s1_fill_converged").is_some(),
            "the fill did not converge: {}",
            o.done
        );
    } else if stored_s1 {
        let e = o.done.str_field("s1_error").unwrap_or_default();
        assert!(
            e.contains("opened for output again") || e.contains("rewritten after it"),
            "S1 refused for another reason: {}",
            o.done
        );
    }
    let (p2, o2) = snapshot(&base, &proj, &out, "w1");
    compare_with_scratch(
        &scratch.1,
        &view,
        &proj,
        &out,
        &p2,
        &o2,
        "main.tex",
        "the reopen",
    );
    let (pdf, log) = (
        std::fs::read(out.join("main.pdf")).unwrap(),
        std::fs::read(out.join("main.log")).unwrap(),
    );
    let _ = c.bye();
    drop(host);
    let (pdf0, log0) = s1_scratch_pdf_log(&proj, &out);
    assert!(pdf == pdf0, "the PDF differs from a scratch compile's");
    assert!(log == log0, "the log differs from a scratch compile's");
    let _ = std::fs::remove_dir_all(&base);
}

/// Lane COLD-OPEN: while a session reopened from S₁ has not filled the
/// pages before it (`FLASHTEX_S1_FILL=off` here: never), its history before
/// S₁ is not the process's. An edit after S₁ runs incrementally from a
/// checkpoint at or after it; one in a file read before S₁ runs from S₀
/// with nothing kept. Each equals a from-scratch compile (the pages from S₁
/// on, then all of them, the PDF and the log).
#[test]
fn edits_after_a_reopen_from_s1_equal_scratch_compiles() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-s1e");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out, cache) = (base.join("proj"), base.join("out"), base.join("s0"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let (main, side) = s1_doc();
    std::fs::write(proj.join("main.tex"), &main).unwrap();
    std::fs::write(proj.join("side.tex"), &side).unwrap();
    s1_store(&proj, &out, &cache, "s1ea");
    let env = [
        ("FLASHTEX_S0_CACHE", cache.to_str().unwrap()),
        ("FLASHTEX_S1_FILL", "off"),
    ];
    let host = start_host_env("s1eb", &env);
    let scratch = start_host("s1es");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 1;
    let mut r = req(id, &proj, &out, "main.tex");
    r.viewport = Some(S1_VIEW);
    let o = compile(&mut c, &mut view, &r);
    let n = o
        .done
        .int_field("s1_pages")
        .unwrap_or_else(|| panic!("S1 not used: {}", o.done)) as u32;
    // an edit after S₁: from a checkpoint at or after it
    let late = main.find("\\section{Last}").unwrap();
    id += 1;
    let mut r = req(id, &proj, &out, "main.tex");
    r.viewport = Some(S1_VIEW);
    r.edits = vec![Edit {
        path: "main.tex".into(),
        offset: late as u64,
        delete: 0,
        insert: "A late paragraph, typed after the reopen.\n\n".into(),
    }];
    let o = compile(&mut c, &mut view, &r);
    assert_eq!(o.done.str_field("mode"), Some("incremental"), "{}", o.done);
    let rp = o.done.int_field("restart_page").unwrap_or(-1);
    assert!(rp >= n as i64, "restart before S1 ({n}): {}", o.done);
    let (p2, o2) = snapshot(&base, &proj, &out, "e1");
    s1_compare_from(
        &scratch.1,
        &view,
        &proj,
        &out,
        &p2,
        &o2,
        n,
        "an edit after S1",
    );
    // a file read before S₁: from S₀, every page again
    std::fs::write(
        proj.join("side.tex"),
        side.replace("A side file", "A side file, edited after the reopen"),
    )
    .unwrap();
    for _ in 0..5 {
        id += 1;
        let mut r = req(id, &proj, &out, "main.tex");
        r.viewport = Some(S1_VIEW);
        let o = compile(&mut c, &mut view, &r);
        if o.done.str_field("mode") == Some("unchanged") {
            break;
        }
        assert_eq!(o.done.int_field("restart_page"), Some(0), "{}", o.done);
    }
    let (p2, o2) = snapshot(&base, &proj, &out, "e2");
    compare_with_scratch(
        &scratch.1,
        &view,
        &proj,
        &out,
        &p2,
        &o2,
        "main.tex",
        "an edit before S1",
    );
    let (pdf, log) = (
        std::fs::read(out.join("main.pdf")).unwrap(),
        std::fs::read(out.join("main.log")).unwrap(),
    );
    let _ = c.bye();
    drop(host);
    let (pdf0, log0) = s1_scratch_pdf_log(&proj, &out);
    assert!(pdf == pdf0, "the PDF differs from a scratch compile's");
    assert!(log == log0, "the log differs from a scratch compile's");
    let _ = std::fs::remove_dir_all(&base);
}

// ---------------------------------------------------------------------------
// Lane TEST-GAPS: host/server.rs's error and cancellation paths that the CI
// coverage run (coverage-engine.yml, 2026-11-01) found untaken.
// ---------------------------------------------------------------------------

/// A raw connection to the host: frames written and read by hand.
fn raw_connect(sock: &Path) -> std::os::unix::net::UnixStream {
    std::os::unix::net::UnixStream::connect(sock).unwrap()
}

fn raw_hello(major: i64) -> Vec<u8> {
    format!(
        "{{\"protocol\":\"{}\",\"version\":[{major},{}]}}",
        flashtex_display_list::PROTOCOL,
        flashtex_display_list::LATEST_MINOR
    )
    .into_bytes()
}

/// The next frame, decoded as JSON (`Null`: a binary one, PAGE or FONT).
fn raw_read(s: &mut std::os::unix::net::UnixStream) -> (u8, Json) {
    let (k, body) = flashtex_display_list::frame::read_frame(s)
        .unwrap()
        .expect("the host closed the connection");
    let j = std::str::from_utf8(&body)
        .ok()
        .and_then(|t| Json::parse(t).ok())
        .unwrap_or(Json::Null);
    (k, j)
}

/// `server::connection`'s refusals: a first frame that is not HELLO
/// ("expected HELLO"), a HELLO of another major version ("this host
/// speaks ..."), each an ERROR before the host closes the connection; and
/// a COMPILE whose body is not JSON ("COMPILE is not JSON"), after which
/// the connection goes on to a compile that runs.
#[test]
fn a_connection_refuses_a_bad_hello_and_a_compile_that_is_not_json() {
    use flashtex_display_list::frame::write_frame;
    use flashtex_display_list::kind;
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let host = start_host("proto");
    // (a) COMPILE first
    let mut s = raw_connect(&host.1);
    write_frame(&mut s, kind::COMPILE, b"{}").unwrap();
    let (k, j) = raw_read(&mut s);
    assert_eq!(k, kind::ERROR, "{j}");
    assert_eq!(j.str_field("message"), Some("expected HELLO"), "{j}");
    // (b) another major version
    let mut s = raw_connect(&host.1);
    write_frame(&mut s, kind::C_HELLO, &raw_hello(99)).unwrap();
    let (k, j) = raw_read(&mut s);
    assert_eq!(k, kind::ERROR, "{j}");
    assert_eq!(j.str_field("code"), Some("version"), "{j}");
    assert!(
        j.str_field("message")
            .is_some_and(|m| m.starts_with("this host speaks")),
        "{j}"
    );
    // (c) a COMPILE that is not JSON, then one that is
    let mut s = raw_connect(&host.1);
    write_frame(
        &mut s,
        kind::C_HELLO,
        &raw_hello(flashtex_display_list::VERSION_MAJOR as i64),
    )
    .unwrap();
    assert_eq!(raw_read(&mut s).0, kind::HELLO);
    write_frame(&mut s, kind::COMPILE, b"{not json").unwrap();
    let (k, j) = raw_read(&mut s);
    assert_eq!(k, kind::ERROR, "{j}");
    assert_eq!(j.str_field("message"), Some("COMPILE is not JSON"), "{j}");
    let base = common::fresh_dir("flashtex-host-proto");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(proj.join("main.tex"), article(1)).unwrap();
    let r = req(1, &proj, &out, "main.tex");
    write_frame(&mut s, kind::COMPILE, r.to_json().to_string().as_bytes()).unwrap();
    let done = loop {
        let (k, j) = raw_read(&mut s);
        if k == kind::DONE {
            break j;
        }
        assert_ne!(k, kind::ERROR, "{j}");
    };
    assert_eq!(done.str_field("status"), Some("ok"), "{done}");
    drop(host);
    let _ = std::fs::remove_dir_all(&base);
}

/// An export (`server::start_export`) of a document with an error: DONE's
/// status is "error", and the DIAGNOSTIC names the error the oracle's log
/// has (TeX Live's pdflatex on the same file: its message and `l.N`).
#[test]
fn an_export_with_an_error_reports_it_as_pdflatex_does() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-xerr");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out, oracle) = (base.join("proj"), base.join("out"), base.join("oracle"));
    for d in [&proj, &out, &oracle] {
        std::fs::create_dir_all(d).unwrap();
    }
    let doc = article(2).replacen(
        "\\begin{document}\n",
        "\\begin{document}\nA sentence, then \\undefinedmacro here.\n",
        1,
    );
    std::fs::write(proj.join("main.tex"), &doc).unwrap();
    std::fs::write(oracle.join("main.tex"), &doc).unwrap();
    // the oracle: its first error's message and line
    let bin = find_texlive_bin().unwrap();
    let _ = Command::new(bin.join("pdflatex"))
        .args(["-interaction=nonstopmode", "main.tex"])
        .current_dir(&oracle)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    let log = std::fs::read_to_string(oracle.join("main.log")).unwrap();
    let mut lines = log.lines().skip_while(|l| !l.starts_with("! "));
    let message = lines.next().expect("the oracle reports an error")[2..].to_string();
    let line: i64 = lines
        .find_map(|l| l.strip_prefix("l.")?.split(' ').next()?.parse().ok())
        .expect("the oracle's l.N");
    let host = start_host("xerr");
    let mut c = Client::connect(&host.1).unwrap();
    let mut r = CompileRequest::new(1, proj.to_str().unwrap(), "main.tex");
    r.output_dir = Some(out.to_str().unwrap().into());
    r.export = true;
    c.compile(&r).unwrap();
    let mut diags = vec![];
    let done = loop {
        match c
            .next_event()
            .unwrap()
            .expect("the host closed the connection")
        {
            Event::Diagnostic(d) => diags.push(d),
            Event::Done(d) => break d,
            Event::Error(e) => panic!("host error: {e}"),
            _ => {}
        }
    };
    assert_eq!(done.str_field("status"), Some("error"), "{done}");
    assert!(
        diags
            .iter()
            .any(|d| d.str_field("severity") == Some("error")
                && d.str_field("message")
                    .is_some_and(|m| m.contains(message.trim_end_matches('.')))
                && d.int_field("line") == Some(line)),
        "no diagnostic for the oracle's {message:?} at l.{line}: {diags:?}"
    );
    let _ = c.bye();
    drop(host);
    let _ = std::fs::remove_dir_all(&base);
}

/// A CANCEL of a running export (`server::connection`, `ExportRun::cancel`)
/// ends it: DONE's status is "cancelled", and the connection goes on to a
/// compile that runs.
#[test]
fn a_cancelled_export_says_so_and_the_connection_goes_on() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-xcancel");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(proj.join("main.tex"), article(400)).unwrap();
    let host = start_host("xcancel");
    let mut c = Client::connect(&host.1).unwrap();
    let mut r = CompileRequest::new(1, proj.to_str().unwrap(), "main.tex");
    r.output_dir = Some(out.to_str().unwrap().into());
    r.export = true;
    c.compile(&r).unwrap();
    let done = loop {
        match c
            .next_event()
            .unwrap()
            .expect("the host closed the connection")
        {
            Event::Started(_) => c.cancel(1).unwrap(),
            Event::Done(d) => break d,
            Event::Error(e) => panic!("host error: {e}"),
            _ => {}
        }
    };
    // (CANCEL follows STARTED's COMPILE in order, so it cannot precede the
    // export; but a reader stalled for the whole export sees it finish
    // first: not a failure of cancelling, review of #2019)
    if done.str_field("status") == Some("ok") {
        eprintln!("skipped: the export finished before the CANCEL was read ({done})");
    } else {
        assert_eq!(done.str_field("status"), Some("cancelled"), "{done}");
    }
    let mut view = View::default();
    std::fs::write(proj.join("main.tex"), article(1)).unwrap();
    let o = compile(&mut c, &mut view, &req(2, &proj, &out, "main.tex"));
    assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
    let _ = c.bye();
    drop(host);
    let _ = std::fs::remove_dir_all(&base);
}

/// Every page of `view` draws at least one form.
fn every_page_draws_a_form(view: &View, proj: &Path, out: &Path, what: &str) {
    for i in 0..view.count as u32 {
        assert!(
            !resolve(view, i, proj, out).forms.is_empty(),
            "{what}: page {i} draws no form"
        );
    }
}

/// Lane TEST-GAPS: forms (`\pdfxform`) through the resident host
/// (`host::resident::emit`'s and `deliver`'s form paths, never taken under
/// the CI coverage run; and the tests' views ignored FORM, so a stale form
/// went unnoticed). A form made on the first page, drawn on every page,
/// and its content edited: after each compile every page and every form it
/// draws equals a scratch compile's. Then a second connection to the same
/// document gets the kept pages from the cache, with their forms.
#[test]
fn an_edited_form_drawn_on_every_page_equals_a_scratch_compile() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let doc = |logo: &str| -> String {
        let mut s = format!(
            "\\documentclass{{article}}\n\\begin{{document}}\n\
             \\setbox0\\hbox{{\\bfseries Logo {logo}}}\\immediate\\pdfxform0\n\
             \\edef\\logo{{\\the\\pdflastxform}}\n"
        );
        for i in 0..60 {
            s.push_str(&format!(
                "Paragraph {i} with the logo \\pdfrefxform\\logo{{}} and enough text to \
                 fill a few lines of the page so that the document ships several pages.\n\n"
            ));
        }
        s.push_str("\\end{document}\n");
        s
    };
    let base = common::fresh_dir("flashtex-host-forms");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(proj.join("main.tex"), doc("alpha")).unwrap();
    let host = start_host("forms");
    let scratch = start_host("formss");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    for (k, logo) in ["alpha", "alpha", "omega", "omega", "alpha"]
        .iter()
        .enumerate()
    {
        std::fs::write(proj.join("main.tex"), doc(logo)).unwrap();
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, "main.tex"));
        assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
        assert!(view.count >= 3, "{} pages", view.count);
        assert!(!view.forms.is_empty(), "compile {k}: no FORM sent");
        // (each page draws the logo: if emission dropped every form draw,
        // the scratch side would lose them too, review of #2019)
        every_page_draws_a_form(&view, &proj, &out, &format!("compile {k}"));
        let (p2, o2) = snapshot(&base, &proj, &out, &format!("f{k}"));
        compare_with_scratch(
            &scratch.1,
            &view,
            &proj,
            &out,
            &p2,
            &o2,
            "main.tex",
            &format!("compile {k}, logo {logo}"),
        );
    }
    // a second client of the same document: the pages from the cache,
    // with the forms they draw
    let mut c2 = Client::connect(&host.1).unwrap();
    let mut v2 = View::default();
    let mut r = req(1, &proj, &out, "main.tex");
    r.incremental = true;
    let o = compile(&mut c2, &mut v2, &r);
    assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
    every_page_draws_a_form(&v2, &proj, &out, "a second connection");
    let (p2, o2) = snapshot(&base, &proj, &out, "fsecond");
    compare_with_scratch(
        &scratch.1,
        &v2,
        &proj,
        &out,
        &p2,
        &o2,
        "main.tex",
        "a second connection",
    );
    let _ = c.bye();
    let _ = c2.bye();
    drop(host);
    let _ = std::fs::remove_dir_all(&base);
}

/// A `COMPILE` the host may refuse: the ERROR it sent, or the DONE of the
/// compile it ran (its pages not kept).
fn compile_or_error(c: &mut Client, r: &CompileRequest) -> Result<Json, Json> {
    c.compile(r).unwrap();
    loop {
        match c
            .next_event()
            .unwrap()
            .expect("the host closed the connection")
        {
            Event::Error(e) => return Err(e),
            Event::Done(d) => return Ok(d),
            _ => {}
        }
    }
}

/// Lane TEST-GAPS: a preamble edit after a reopen from S₁ (`incr::
/// Session::compile_pass`): S₀'s key fails, and with no checkpoint before
/// S₀ and no history before S₁ the compile runs from the format (`mode`
/// cold, `cold_reason` naming the file). Its pages, the PDF and the log
/// equal a scratch compile's.
#[test]
fn a_preamble_edit_after_a_reopen_from_s1_runs_cold() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-s1p");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out, cache) = (base.join("proj"), base.join("out"), base.join("s0"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let (main, side) = s1_doc();
    std::fs::write(proj.join("main.tex"), &main).unwrap();
    std::fs::write(proj.join("side.tex"), &side).unwrap();
    s1_store(&proj, &out, &cache, "s1pa");
    let env = [
        ("FLASHTEX_S0_CACHE", cache.to_str().unwrap()),
        ("FLASHTEX_S1_FILL", "off"),
    ];
    let host = start_host_env("s1pb", &env);
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut r = req(1, &proj, &out, "main.tex");
    r.viewport = Some(S1_VIEW);
    let o = compile(&mut c, &mut view, &r);
    assert!(
        o.done.int_field("s1_pages").is_some(),
        "S1 not used: {}",
        o.done
    );
    let at = main.find("\\begin{document}").unwrap();
    let mut r = req(2, &proj, &out, "main.tex");
    r.viewport = Some(S1_VIEW);
    r.edits = vec![Edit {
        path: "main.tex".into(),
        offset: at as u64,
        delete: 0,
        insert: "\\newcommand\\gapsx{x}\n".into(),
    }];
    let o = compile(&mut c, &mut view, &r);
    assert_eq!(o.done.str_field("mode"), Some("cold"), "{}", o.done);
    let why = o.done.str_field("cold_reason").unwrap_or_default();
    assert!(
        why.contains("main.tex"),
        "cold for another reason: {}",
        o.done
    );
    for id in 3..8 {
        let mut r = req(id, &proj, &out, "main.tex");
        r.viewport = Some(S1_VIEW);
        if compile(&mut c, &mut view, &r).done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    let (p2, o2) = snapshot(&base, &proj, &out, "s1px");
    let scratch = start_host("s1ps");
    compare_with_scratch(
        &scratch.1,
        &view,
        &proj,
        &out,
        &p2,
        &o2,
        "main.tex",
        "a preamble edit after S1",
    );
    let (pdf, log) = (
        std::fs::read(out.join("main.pdf")).unwrap(),
        std::fs::read(out.join("main.log")).unwrap(),
    );
    let _ = c.bye();
    drop(host);
    let (pdf0, log0) = s1_scratch_pdf_log(&proj, &out);
    assert!(pdf == pdf0, "the PDF differs from a scratch compile's");
    assert!(log == log0, "the log differs from a scratch compile's");
    let _ = std::fs::remove_dir_all(&base);
}

/// Lane TEST-GAPS: `host::server::Job::parse`'s refusals: a root that is
/// not an absolute directory, a main file outside it or missing, a format
/// given as a path. Each is an ERROR naming it, the connection goes on,
/// and a valid compile after them runs.
#[test]
fn a_compile_of_a_bad_root_main_or_format_is_refused() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-job");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(proj.join("main.tex"), article(1)).unwrap();
    std::fs::write(base.join("outside.tex"), article(1)).unwrap();
    let host = start_host("job");
    let mut c = Client::connect(&host.1).unwrap();
    let rel = |id: i64, root: &str, main: &str| {
        let mut r = CompileRequest::new(id, root, main);
        r.output_dir = Some(out.to_str().unwrap().into());
        r
    };
    let proj_s = proj.to_str().unwrap();
    let mut fmt = rel(5, proj_s, "main.tex");
    fmt.format = "../pdflatex".into();
    let cases = [
        (
            "a relative root",
            rel(1, "proj", "main.tex"),
            "root proj is not an absolute directory".to_string(),
        ),
        (
            "a root that is a file",
            rel(2, proj.join("main.tex").to_str().unwrap(), "main.tex"),
            format!(
                "root {} is not an absolute directory",
                proj.join("main.tex").display()
            ),
        ),
        (
            "a main file outside the root",
            rel(3, proj_s, "../outside.tex"),
            "main must be a path inside root".to_string(),
        ),
        (
            "a missing main file",
            rel(4, proj_s, "missing.tex"),
            "missing.tex does not exist in root".to_string(),
        ),
        (
            "a format path",
            fmt,
            "format is a name, not a path".to_string(),
        ),
    ];
    for (what, r, why) in cases {
        let e = compile_or_error(&mut c, &r)
            .map(|d| d.to_string())
            .expect_err(what)
            .to_string();
        assert!(e.contains(&why), "{what}: {e}");
    }
    let mut view = View::default();
    let o = compile(&mut c, &mut view, &req(6, &proj, &out, "main.tex"));
    assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
    let _ = c.bye();
    drop(host);
    let _ = std::fs::remove_dir_all(&base);
}

/// Lane TEST-GAPS: `host::resident::apply_changes`'s refusals. A buffer
/// whose path leaves the project (`..`, absolute) and an edit past the
/// file's end are refused with an ERROR that names them, and nothing is
/// written; the next compile, with a valid edit, equals a scratch compile.
#[test]
fn buffers_and_edits_outside_the_project_or_the_file_are_refused() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-apply");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let main = article(3);
    std::fs::write(proj.join("main.tex"), &main).unwrap();
    let host = start_host("apply");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let o = compile(&mut c, &mut view, &req(1, &proj, &out, "main.tex"));
    assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
    let outside = base.join("escape.tex");
    let abs = outside.to_str().unwrap().to_string();
    // (what, the buffers, the edits, the refusal)
    type Case = (&'static str, Vec<(String, String)>, Vec<Edit>, String);
    let cases: [Case; 3] = [
        (
            "a buffer in the parent directory",
            vec![("../escape.tex".into(), "x".into())],
            vec![],
            "../escape.tex: a path inside root".into(),
        ),
        (
            "a buffer at an absolute path",
            vec![(abs.clone(), "x".into())],
            vec![],
            format!("{abs}: a path inside root"),
        ),
        (
            "an edit past the file's end",
            vec![],
            vec![Edit {
                path: "main.tex".into(),
                offset: main.len() as u64,
                delete: 1,
                insert: "x".into(),
            }],
            "main.tex: edit outside the file".into(),
        ),
    ];
    for (k, (what, buffers, edits, why)) in cases.into_iter().enumerate() {
        let mut r = req(2 + k as i64, &proj, &out, "main.tex");
        r.buffers = buffers;
        r.edits = edits;
        let e = compile_or_error(&mut c, &r)
            .map(|d| d.to_string())
            .expect_err(what)
            .to_string();
        assert!(e.contains(&why), "{what}: {e}");
        assert!(
            !outside.exists(),
            "{what}: a file outside the project was written"
        );
        assert_eq!(
            std::fs::read_to_string(proj.join("main.tex")).unwrap(),
            main,
            "{what}: main.tex changed"
        );
    }
    // a valid edit still applies, and equals a scratch compile
    let at = main.find("\\end{document}").unwrap() as u64;
    let mut r = req(9, &proj, &out, "main.tex");
    r.edits = vec![Edit {
        path: "main.tex".into(),
        offset: at,
        delete: 0,
        insert: "An appended sentence.\n".into(),
    }];
    let d = compile(&mut c, &mut view, &r).done;
    assert_eq!(d.str_field("status"), Some("ok"), "{d}");
    assert!(
        std::fs::read_to_string(proj.join("main.tex"))
            .unwrap()
            .contains("An appended sentence."),
        "the edit was not applied"
    );
    let (p2, o2) = snapshot(&base, &proj, &out, "applyx");
    let scratch = start_host("applys");
    compare_with_scratch(
        &scratch.1, &view, &proj, &out, &p2, &o2, "main.tex", "the edit",
    );
    let _ = c.bye();
    drop(host);
    let _ = std::fs::remove_dir_all(&base);
}

/// Lane TEST-GAPS: S₁'s save refusals (`incr::Session::prepare_s1`), which
/// the next DONE reports (`s1_save`). A host with an S₀ cache compiles the
/// document at `viewport` until nothing changes; `between` runs right after
/// (before the host is idle); once S₀ is stored and the host has been idle,
/// a compile's DONE says why no S₁ was stored, and none is. That compile's
/// pages equal a scratch compile's.
fn s1_refused(tag: &str, viewport: u32, between: &dyn Fn(&Path), why: &str) {
    let base = common::fresh_dir(&format!("flashtex-host-s1r-{tag}"));
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out, cache) = (base.join("proj"), base.join("out"), base.join("s0"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let (main, side) = s1_doc();
    std::fs::write(proj.join("main.tex"), &main).unwrap();
    std::fs::write(proj.join("side.tex"), &side).unwrap();
    let host = start_host_env(
        &format!("s1r{tag}"),
        &[("FLASHTEX_S0_CACHE", cache.to_str().unwrap())],
    );
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    let at = |id: i64| {
        let mut r = req(id, &proj, &out, "main.tex");
        r.viewport = Some(viewport);
        r
    };
    for _ in 0..6 {
        id += 1;
        let o = compile(&mut c, &mut view, &at(id));
        if o.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    between(&proj);
    let deadline = Instant::now() + Duration::from_secs(120);
    while !stored(&cache, "s0") && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    let mut said = None;
    while said.is_none() && Instant::now() < deadline {
        // (idle: the host tries S₁ 2 s after it stops keeping warm, 2 s
        // after the last request)
        std::thread::sleep(Duration::from_secs(7));
        id += 1;
        let o = compile(&mut c, &mut view, &at(id));
        said = o.done.get("s1_save").map(|s| s.to_string());
    }
    let said = said.unwrap_or_else(|| panic!("{tag}: no S1 save was tried"));
    assert!(said.contains(why), "{tag}: the save went otherwise: {said}");
    assert!(!stored(&cache, "s1"), "{tag}: an S1 was stored");
    let (p2, o2) = snapshot(&base, &proj, &out, &format!("s1r{tag}"));
    let scratch = start_host(&format!("s1rs{tag}"));
    compare_with_scratch(&scratch.1, &view, &proj, &out, &p2, &o2, "main.tex", tag);
    let _ = c.bye();
    drop(host);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn an_s1_save_near_the_document_start_is_refused() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    // (S₁ at least `S1_MIN_PAGES` = 4 pages in, `S1_MARGIN` = 2 before the
    // viewport)
    s1_refused("near", 5, &|_| {}, "the viewport is near the start");
}

#[test]
fn an_s1_save_after_an_input_changed_is_refused() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    // (side.tex changed after the last compile, before the host was idle:
    // the last run is not what the files now give)
    let edit = |proj: &Path| {
        let p = proj.join("side.tex");
        let t = std::fs::read_to_string(&p).unwrap();
        std::fs::write(&p, t.replace("A side file", "A side file, edited")).unwrap();
    };
    s1_refused("dirty", S1_VIEW, &edit, "the last run's inputs changed");
}

/// A stored S₁ is refused for a reopen at a viewport before it ("the
/// viewport is before S1", `host::s1::File::check`): the reopen runs from
/// S₀, and its pages, PDF and log equal a scratch compile's.
#[test]
fn a_reopen_at_a_viewport_before_s1_runs_from_s0() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-s1v");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out, cache) = (base.join("proj"), base.join("out"), base.join("s0"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let (main, side) = s1_doc();
    std::fs::write(proj.join("main.tex"), &main).unwrap();
    std::fs::write(proj.join("side.tex"), &side).unwrap();
    s1_store(&proj, &out, &cache, "s1va");
    let host = start_host_env("s1vb", &[("FLASHTEX_S0_CACHE", cache.to_str().unwrap())]);
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    // (S₁ is at least 4 pages in, `S1_MIN_PAGES`)
    let mut r = req(1, &proj, &out, "main.tex");
    r.viewport = Some(2);
    let o = compile(&mut c, &mut view, &r);
    assert_eq!(o.done.str_field("mode"), Some("open"), "{}", o.done);
    assert_eq!(o.done.int_field("s1_pages"), None, "S1 used: {}", o.done);
    let e = o.done.str_field("s1_error").unwrap_or_default();
    assert!(
        e.contains("the viewport is before S1"),
        "refused for another reason: {e:?}"
    );
    for id in 2..6 {
        let mut r = req(id, &proj, &out, "main.tex");
        r.viewport = Some(2);
        if compile(&mut c, &mut view, &r).done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    let (p2, o2) = snapshot(&base, &proj, &out, "s1vx");
    let scratch = start_host("s1vs");
    compare_with_scratch(
        &scratch.1,
        &view,
        &proj,
        &out,
        &p2,
        &o2,
        "main.tex",
        "viewport 2",
    );
    let (pdf, log) = (
        std::fs::read(out.join("main.pdf")).unwrap(),
        std::fs::read(out.join("main.log")).unwrap(),
    );
    let _ = c.bye();
    drop(host);
    let (pdf0, log0) = s1_scratch_pdf_log(&proj, &out);
    assert!(pdf == pdf0, "the PDF differs from a scratch compile's");
    assert!(log == log0, "the log differs from a scratch compile's");
    let _ = std::fs::remove_dir_all(&base);
}

/// A replayed call's color stack must exist where it is replayed (BOX-MEMO).
/// `\pdfcolorstack <n>` checks `n` against the stacks `\pdfcolorstackinit`
/// made, which no key held: a draw call recorded while stack 1 existed was
/// replayed by the compile after an edit that took the stack away, which
/// pdfTeX answers with "Unknown color stack number 1" and stack 0; the
/// replay rebuilt the whatsit for stack 1, which the shipout then used.
#[test]
fn a_replayed_call_needs_its_color_stack() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-host-colstack");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let main = "main.tex";
    let init = "\\pdfcolorstackinit page direct{0 g}";
    let text = format!(
        "\\documentclass{{article}}\n\
         \\edef\\stk{{{init}}}\n\
         \\def\\mk{{\\pdfcolorstack 1 push{{1 0 0 rg}}\\vrule width 2pt height 2pt\\pdfcolorstack 1 pop}}\n\
         \\begin{{document}}\n\
         Words before the calls. \\mbox{{\\mk}} and \\mbox{{\\mk}} then.\n\
         \\end{{document}}\n"
    );
    std::fs::write(proj.join(main), &text).unwrap();
    let env = [("FLASHTEX_BOXMEMO", "on"), ("FLASHTEX_BOXMEMO_NAMES", "mk")];
    let host = start_host_env("cs", &env);
    let scratch = start_host_env("css", &env);
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let mut id = 0;
    for _ in 0..4 {
        id += 1;
        let o = compile(&mut c, &mut view, &req(id, &proj, &out, main));
        if o.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    let hits = |o: &Outcome| -> i64 {
        o.done
            .get("stages")
            .and_then(|s| s.get("boxmemo"))
            .and_then(|a| a.as_array())
            .and_then(|a| a.get(1).and_then(|n| n.as_i64()))
            .unwrap_or(0)
    };
    // edits on the calls' page: recorded in the first edit's window, then
    // replayed
    let at = text.find("before").unwrap() as u64;
    let mut replayed = 0;
    for k in 0..4 {
        id += 1;
        let mut r = req(id, &proj, &out, main);
        r.edits = vec![Edit {
            path: main.into(),
            offset: at,
            delete: if k % 2 == 0 { 0 } else { 1 },
            insert: if k % 2 == 0 {
                "x".into()
            } else {
                String::new()
            },
        }];
        let o = compile(&mut c, &mut view, &r);
        assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
        replayed += hits(&o);
    }
    assert!(replayed > 0, "the calls were never replayed");
    // the stack goes: `\stk` keeps its meaning (`1`), so only the stack
    // count tells the states apart
    id += 1;
    let mut r = req(id, &proj, &out, main);
    r.edits = vec![Edit {
        path: main.into(),
        offset: text.find(init).unwrap() as u64,
        delete: init.len() as u64,
        insert: "1".into(),
    }];
    let o = compile(&mut c, &mut view, &r);
    assert!(o.done.str_field("status").is_some(), "{}", o.done);
    let (p2, o2) = snapshot(&base, &proj, &out, "cs");
    compare_with_scratch(&scratch.1, &view, &proj, &out, &p2, &o2, main, "stack gone");
    let log = std::fs::read_to_string(out.join("main.log")).unwrap_or_default();
    assert!(
        log.contains("Unknown color stack number 1"),
        "the normal path's error is missing: {log}"
    );
    let _ = c.bye();
    let _ = std::fs::remove_dir_all(&base);
}

/// A `COMPILE`'s buffers and edits are written only inside the project
/// (`host::resident::apply_changes`, through the root's descriptor, no link
/// followed below it, #2094): through a linked directory, a linked file or a
/// dangling link that leads out of the root, the request is refused and
/// nothing outside is written or created. A link that stays inside the root
/// is refused too (failing closed), and its file left as it was.
#[cfg(unix)]
#[test]
fn buffers_through_a_symlink_out_of_the_project_are_refused() {
    use std::os::unix::fs::symlink;
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    fn refusal(c: &mut Client, r: &CompileRequest) -> Option<String> {
        c.compile(r).unwrap();
        loop {
            match c
                .next_event()
                .unwrap()
                .expect("the host closed the connection")
            {
                Event::Error(e) => return Some(e.to_string()),
                Event::Done(_) => return None,
                _ => {}
            }
        }
    }
    let base = common::fresh_dir("flashtex-host-symlink");
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out, away) = (base.join("proj"), base.join("out"), base.join("away"));
    for d in [&proj, &out, &away] {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(proj.join("main.tex"), article(1)).unwrap();
    std::fs::write(away.join("kept.tex"), "outside\n").unwrap();
    symlink(&away, proj.join("linked")).unwrap();
    symlink(away.join("kept.tex"), proj.join("kept.tex")).unwrap();
    symlink(away.join("new.tex"), proj.join("dangling.tex")).unwrap();
    std::fs::write(proj.join("real.tex"), "inside\n").unwrap();
    symlink(proj.join("real.tex"), proj.join("alias.tex")).unwrap();
    let host = start_host("symlink");
    let mut c = Client::connect(&host.1).unwrap();
    let mut view = View::default();
    let o = compile(&mut c, &mut view, &req(1, &proj, &out, "main.tex"));
    assert_eq!(o.done.str_field("status"), Some("ok"), "{}", o.done);
    let edit = |p: &str| Edit {
        path: p.into(),
        offset: 0,
        delete: 0,
        insert: "x".into(),
    };
    let cases: [(&str, Option<&str>, Option<Edit>); 5] = [
        (
            "a buffer in a linked directory",
            Some("linked/evil.tex"),
            None,
        ),
        ("a buffer over a linked file", Some("kept.tex"), None),
        ("a buffer at a dangling link", Some("dangling.tex"), None),
        (
            "an edit of a linked directory's file",
            None,
            Some(edit("linked/kept.tex")),
        ),
        ("an edit of a linked file", None, Some(edit("kept.tex"))),
    ];
    for (k, (what, buffer, e)) in cases.into_iter().enumerate() {
        let mut r = req(2 + k as i64, &proj, &out, "main.tex");
        if let Some(p) = buffer {
            r.buffers = vec![(p.into(), "written\n".into())];
        }
        r.edits = e.into_iter().collect();
        let why = refusal(&mut c, &r).unwrap_or_else(|| panic!("{what}: written"));
        assert!(why.contains("\"request\""), "{what}: {why}");
        assert!(
            !away.join("evil.tex").exists(),
            "{what}: away/evil.tex made"
        );
        assert!(!away.join("new.tex").exists(), "{what}: away/new.tex made");
        assert_eq!(
            std::fs::read_to_string(away.join("kept.tex")).unwrap(),
            "outside\n",
            "{what}: away/kept.tex changed"
        );
    }
    // a link inside the project is refused as well
    let mut r = req(9, &proj, &out, "main.tex");
    r.buffers = vec![("alias.tex".into(), "inside, edited\n".into())];
    assert!(refusal(&mut c, &r).is_some(), "a link inside the root");
    assert_eq!(
        std::fs::read_to_string(proj.join("real.tex")).unwrap(),
        "inside\n"
    );
    let _ = c.bye();
    drop(host);
    let _ = std::fs::remove_dir_all(&base);
}
