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
    let fmt = std::env::temp_dir().join(format!("flashtex-host-fmt-{}", std::process::id()));
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
    std::thread::spawn(move || for _ in out.lines() {});
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
    count: usize,
}

struct Outcome {
    started: Json,
    done: Json,
    /// Indices of the PAGE messages, in order.
    order: Vec<u32>,
    pages_msgs: Vec<Json>,
    first_page: Option<Duration>,
}

fn compile(c: &mut Client, view: &mut View, req: &CompileRequest) -> Outcome {
    let t0 = Instant::now();
    c.compile(req).unwrap();
    let mut started = Json::Null;
    let mut order = vec![];
    let mut pages_msgs = vec![];
    let mut first_page = None;
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
            Event::Pages(j) => pages_msgs.push(j),
            Event::Image(j) => view.images.push(j),
            Event::Done(d) => break d,
            Event::Error(e) => panic!("host error: {e}"),
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
    fonts.sort();
    fonts.dedup();
    Resolved {
        hash: page.hash,
        page,
        spans,
        fonts,
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
    let mut s = Client::connect(sock).unwrap();
    let mut sv = View::default();
    let mut rs = CompileRequest::new(1, p2.to_str().unwrap(), main);
    rs.output_dir = Some(o2.to_str().unwrap().into());
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
    let base =
        std::env::temp_dir().join(format!("flashtex-host-incr-{}-{name}", std::process::id()));
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
    if !pdftex.exists() {
        let _ = std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_flashtex-initex"), &pdftex);
    }
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
    let base = std::env::temp_dir().join(format!("fth-mp-{}-{name}", std::process::id()));
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

#[test]
fn edits_stream_the_edited_page_and_equal_scratch_compiles_article() {
    let src = std::env::temp_dir().join(format!("flashtex-host-incr-src-{}", std::process::id()));
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
    let base = std::env::temp_dir().join(format!("flashtex-host-export-{}", std::process::id()));
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
    let base = std::env::temp_dir().join(format!("flashtex-host-same-{}", std::process::id()));
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
    let base = std::env::temp_dir().join(format!("flashtex-host-preempt-{}", std::process::id()));
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

/// #1295: pdfTeX frees an image's name once it has written the XObject; a
/// page that draws the image again after a restore (or in a later pass)
/// must still get an `IMAGE` naming the file.
#[test]
fn an_image_drawn_again_after_a_restore_names_its_file() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = std::env::temp_dir().join(format!("flashtex-host-image-{}", std::process::id()));
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
    let base = std::env::temp_dir().join(format!("flashtex-host-exp2-{}", std::process::id()));
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
