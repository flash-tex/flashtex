//! `dl3-keys`: keystroke-to-page latency through a running `flashtex-host`
//! (protocol 3.1, `"incremental": true`), measured on the client side of the
//! socket, as the app sees it.
//!
//!     dl3-keys --socket PATH --root DIR --main main.tex [--output-dir DIR]
//!         [--keys N] [--at FRACTION] [--gap-ms MS] [--overlap] [--no-viewport]
//!         [--page INDEX] [--where start|middle|end] [--sentence]
//!
//! Opens the document (compiles until nothing changes), picks a prose line
//! whose glyphs are on one page about FRACTION (default 0.5) into the
//! document, then types N keystrokes there (default 40): alternately a
//! letter inserted into a word and the same letter deleted, each sent as a
//! `COMPILE` with the edit and that page as the `viewport`, the next one
//! after the previous `DONE` (plus MS) -- or, with `--overlap`, as soon as
//! the previous keystroke's edited page has arrived, while that compile's
//! background work goes on (the host preempts it). Per keystroke it prints one JSON
//! line: time to the first `PAGE`, to the edited page, to `DONE` (ms from
//! sending `COMPILE`), the first page's index, and the host's `DONE`; then a
//! summary with the median, p95 and maximum.
//!
//! `--page INDEX` picks the prose line on (0-based) page INDEX nearest to it
//! instead of `--at`; `--where` types in the line's second word (`start`),
//! the first plain word after its middle (`middle`, the default) or its
//! last plain word (`end`); `--sentence` inserts twelve words there (a
//! change that reflows the paragraph) and deletes them again, instead of
//! one letter.
//!
//! `--line N` (1-based, with `--page INDEX` as the watched page) gives the
//! line to type in instead of searching for one. The search needs the
//! line's glyphs on exactly one page, which a beamer deck never offers: a
//! frame's body is collected as an argument and typeset at `\end{frame}`,
//! so every glyph of the frame carries the `\end{frame}` line (as pdfTeX's
//! SyncTeX records it), and an overlay repeats the frame on several pages.
//! A bad `--line`/`--page` (out of range, or a line the display list places
//! on other pages than `--page`), or a line with no plain word for the edit,
//! exits 2 with a message.
//!
//! `--interval-ms MS` types on a clock instead (keystroke k sent at k * MS,
//! whatever the host is doing, as a user types): each keystroke's latency
//! is the time to the watched page's first `PAGE` from its own compile or a
//! later one (what the app paints), so compiles superseded before their
//! edited page show as waiting keystrokes; `unpainted` counts those never
//! shown.
//!
//! `--edit FILE` (relative to `--root`) types in another file of the project
//! than `--main`, e.g. a book chapter the main file `\input`s; the line search
//! and `--line` then look at that file.
//!
//! `--kind K` (DESIGN.md §8, T7's edit kinds) chooses what a keystroke
//! changes and the next changes back: `letter` (the default: a letter
//! inserted, then deleted), `sentence` (the same as `--sentence`; giving
//! both with another kind is an error), `newline` (the space after the word
//! becomes a line break and back: tools/incr-bench/edits.py's `newline`; the
//! output is the same, every later input line moves), `split` (that space
//! becomes a blank line, edits.py's `split`, and the next keystroke turns the
//! blank line back into the space, edits.py's `join` of that break) or
//! `preamble` (a `\newcommand` line after the `\documentclass` line: S₀
//! changes, a full run from the format; page 1 is the watched page and the
//! viewport). `newline` and `split` take the first plain word from the
//! chosen one on whose following space edits.py's conditions hold here (a
//! space then a letter, outside inline math, braces balanced in the line).
//!
//! Each keystroke's line also says how the compile kept the client's pages
//! current (DESIGN.md §1.2, §5.4): `later_pages` (pages after the watched
//! one that arrived after it: the background refresh), `stale_marked` (a
//! `PAGES` marking pages stale arrived before the first of those, or there
//! were none), and `complete` (the last `PAGES` before `DONE` says all of
//! `DONE`'s pages are current). Open compiles' lines carry `first_page_ms`
//! too (`--keys 0`: open only, e.g. a reopen from a persisted S₀).

use flashtex_display_list::client::{Client, CompileRequest, Edit, Event};
use flashtex_display_list::json::{s, Json};
use flashtex_display_list::page::Item;
use std::collections::HashMap;
use std::io::Write;
use std::time::{Duration, Instant};

struct Held {
    /// Page index -> the (file, line) of every span its items name.
    lines: HashMap<u32, Vec<(u32, u32)>>,
    spans: HashMap<u32, (u32, u32)>,
    files: HashMap<u32, String>,
    count: usize,
}

struct Result {
    first_page: Option<f64>,
    target_page: Option<f64>,
    first_index: Option<u32>,
    done_ms: f64,
    done: Json,
    /// Pages after the target that arrived after it (the background refresh).
    later_pages: u32,
    /// A `PAGES` with stale pages came before the first of `later_pages`
    /// (true when there were none).
    stale_marked: bool,
    /// The last `PAGES` before `DONE` was complete, with `DONE`'s count.
    complete: Option<bool>,
}

fn compile(c: &mut Client, held: &mut Held, req: &CompileRequest, target: Option<u32>) -> Result {
    let t0 = Instant::now();
    c.compile(req).expect("send COMPILE");
    let (mut first_page, mut target_page, mut first_index) = (None, None, None);
    let (mut later_pages, mut stale_seen, mut stale_marked) = (0u32, false, true);
    let mut last_pages: Option<Json> = None;
    let done = loop {
        let ev = c
            .next_event()
            .expect("read")
            .expect("host closed the connection");
        let ms = t0.elapsed().as_secs_f64() * 1e3;
        match ev {
            Event::Started(j) => {
                if j.get("keep").and_then(Json::as_bool) != Some(true) {
                    held.lines.clear();
                }
            }
            Event::Sources(src) => {
                for (i, p) in src.files {
                    held.files.insert(i, p);
                }
                for (i, f, l) in src.spans {
                    held.spans.insert(i, (f, l));
                }
            }
            Event::Page(p) => {
                first_page.get_or_insert(ms);
                first_index.get_or_insert(p.index);
                if Some(p.index) == target {
                    target_page.get_or_insert(ms);
                } else if target_page.is_some() && target.is_some_and(|t| p.index > t) {
                    if later_pages == 0 && !stale_seen {
                        stale_marked = false;
                    }
                    later_pages += 1;
                }
                let lines = p
                    .items
                    .iter()
                    .filter_map(|it| match it {
                        Item::Span(s) if *s != 0 => held.spans.get(s).copied(),
                        _ => None,
                    })
                    .collect();
                held.lines.insert(p.index, lines);
            }
            Event::Error(e) => {
                eprintln!("dl3-keys: host error: {e}");
                std::process::exit(1)
            }
            Event::Pages(j) => {
                if j.get("stale")
                    .and_then(Json::as_array)
                    .is_some_and(|a| !a.is_empty())
                {
                    stale_seen = true;
                }
                last_pages = Some(j);
            }
            Event::Done(d) => break d,
            _ => {}
        }
    };
    let complete = last_pages.map(|j| {
        j.get("complete").and_then(Json::as_bool) == Some(true)
            && j.int_field("count") == done.int_field("pages")
    });
    held.count = done.int_field("pages").unwrap_or(0) as usize;
    let n = held.count as u32;
    held.lines.retain(|&i, _| i < n);
    Result {
        first_page,
        target_page,
        first_index,
        done_ms: t0.elapsed().as_secs_f64() * 1e3,
        done,
        later_pages,
        stale_marked,
        complete,
    }
}

fn pct(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if v.is_empty() {
        return f64::NAN;
    }
    v[((v.len() - 1) as f64 * p).round() as usize]
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arg = |k: &str| {
        a.iter()
            .position(|x| x == k)
            .and_then(|i| a.get(i + 1))
            .cloned()
    };
    let (Some(socket), Some(root), Some(main)) = (arg("--socket"), arg("--root"), arg("--main"))
    else {
        eprintln!("usage: dl3-keys --socket PATH --root DIR --main FILE [--output-dir DIR] [--keys N] [--at FRACTION] [--gap-ms MS] [--edit FILE] [--page INDEX [--line N]] [--where start|middle|end] [--kind letter|sentence|newline|split|preamble | --sentence] [--overlap | --interval-ms MS] [--no-viewport]");
        std::process::exit(2);
    };
    let keys: usize = arg("--keys").and_then(|v| v.parse().ok()).unwrap_or(40);
    let at: f64 = arg("--at").and_then(|v| v.parse().ok()).unwrap_or(0.5);
    let gap = Duration::from_millis(arg("--gap-ms").and_then(|v| v.parse().ok()).unwrap_or(0));
    let mut c = Client::connect(socket.as_ref()).unwrap_or_else(|e| {
        eprintln!("dl3-keys: {socket}: {e}");
        std::process::exit(1)
    });
    let mut held = Held {
        lines: HashMap::new(),
        spans: HashMap::new(),
        files: HashMap::new(),
        count: 0,
    };
    let mut id = 0;
    let req = |id: i64| {
        let mut r = CompileRequest::new(id, &root, &main);
        r.output_dir = arg("--output-dir");
        r.incremental = true;
        r
    };
    // Open: until nothing changes.
    for _ in 0..5 {
        id += 1;
        let r = compile(&mut c, &mut held, &req(id), None);
        println!(
            "{}",
            Json::Obj(vec![
                ("open".into(), Json::Int(id)),
                (
                    "first_page_ms".into(),
                    r.first_page.map(Json::Num).unwrap_or(Json::Null),
                ),
                ("done_ms".into(), Json::Num(r.done_ms)),
                ("host".into(), r.done.clone()),
            ])
        );
        if r.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    if keys == 0 {
        let _ = std::io::stdout().flush();
        let _ = c.bye();
        return;
    }
    // `--edit FILE`: type in another file of the project (a book's chapter
    // under `\input`), relative to the root; the main file by default.
    let efile = arg("--edit").unwrap_or_else(|| main.clone());
    // A prose line on one page, about `at` into the document.
    let path = std::path::Path::new(&root).join(&efile);
    let text = std::fs::read_to_string(&path).expect("read the main file");
    let lines: Vec<&str> = text.split('\n').collect();
    let want = match arg("--page").and_then(|v| v.parse::<u32>().ok()) {
        Some(p) => p,
        None => (held.count as f64 * at) as u32,
    }
    .min(held.count.saturating_sub(1) as u32);
    let main_file = |f: u32| {
        held.files
            .get(&f)
            .is_some_and(|p| p.ends_with(&format!("/{efile}")))
    };
    let on_pages = |line: u32| -> Vec<u32> {
        let mut v: Vec<u32> = held
            .lines
            .iter()
            .filter(|(_, ls)| ls.iter().any(|&(f, l)| l == line && main_file(f)))
            .map(|(i, _)| *i)
            .collect();
        v.sort();
        v
    };
    // `--line N --page P`: the line and the watched page are given (see the
    // module comment: a beamer frame's glyphs all carry its `\end{frame}`
    // line, so no prose line is found on a page).
    let given = arg("--line").map(|v| {
        let page = arg("--page").and_then(|p| p.parse::<u32>().ok());
        let (Ok(l), Some(p)) = (v.parse::<u32>(), page) else {
            eprintln!("dl3-keys: --line {v} wants a line number and --page INDEX");
            std::process::exit(2)
        };
        if !(1..=lines.len() as u32).contains(&l) {
            eprintln!("dl3-keys: --line {l}: {main} has {} lines", lines.len());
            std::process::exit(2)
        }
        if p >= held.count as u32 {
            eprintln!(
                "dl3-keys: --page {p}: the document has {} pages (0-based)",
                held.count
            );
            std::process::exit(2)
        }
        // A line whose glyphs the display list places must be on the
        // watched page; a beamer frame's prose line has none (its glyphs
        // carry the `\end{frame}` line), so any page passes there.
        let on = on_pages(l);
        if !on.is_empty() && !on.contains(&p) {
            eprintln!("dl3-keys: --line {l} is on pages {on:?}, not on --page {p}");
            std::process::exit(2)
        }
        (l, p)
    });
    let (line, page) = given.unwrap_or_else(|| {
        (1..=lines.len() as u32)
            .filter(|&l| lines[l as usize - 1].split(' ').count() > 40)
            .filter_map(|l| {
                // On one page, or (a file input more than once, as in a
                // book typeset twice) on pages at least two apart, each a
                // whole occurrence: the one nearest the wanted page.
                let p = on_pages(l);
                let whole = !p.is_empty() && p.windows(2).all(|w| w[1] > w[0] + 1);
                whole.then(|| (l, *p.iter().min_by_key(|&&q| q.abs_diff(want)).unwrap()))
            })
            .min_by_key(|&(_, p)| p.abs_diff(want))
            .unwrap_or_else(|| {
                eprintln!("dl3-keys: no prose line on one page");
                std::process::exit(1)
            })
    });
    let offset: usize = lines[..line as usize - 1].iter().map(|l| l.len() + 1).sum();
    let this = lines[line as usize - 1];
    // Inside a plain word: the first after the start of the line (its
    // second word), after its middle, or the last one.
    let plain = |w: &str| w.len() >= 4 && w.bytes().all(|c| c.is_ascii_lowercase());
    let starts: Vec<usize> = std::iter::once(0)
        .chain(this.match_indices(' ').map(|(i, _)| i + 1))
        .filter(|&i| plain(this[i..].split(' ').next().unwrap_or("")))
        .collect();
    let found = match arg("--where").as_deref().unwrap_or("middle") {
        "start" => starts.iter().copied().find(|&i| i > 0),
        "end" => starts.last().copied(),
        _ => starts.iter().copied().find(|&i| i > this.len() / 2),
    };
    // (a preamble edit types in no word: only the other kinds need one)
    let pos = || {
        found.unwrap_or_else(|| {
            eprintln!(
                "dl3-keys: line {line} has no plain lowercase word (4+ letters) where \
                 --where asks; give --line a prose line"
            );
            std::process::exit(2)
        })
    };
    let kind = match (a.iter().any(|x| x == "--sentence"), arg("--kind")) {
        (true, Some(k)) if k != "sentence" => {
            eprintln!("dl3-keys: --sentence and --kind {k} conflict");
            std::process::exit(2)
        }
        (true, _) => "sentence".to_string(),
        (false, k) => k.unwrap_or_else(|| "letter".into()),
    };
    // The space after the plain word at `from` or a later one where
    // edits.py's newline/split apply: a space then a letter, outside inline
    // math, braces balanced in the line up to it.
    let space_after = |from: usize| -> usize {
        starts
            .iter()
            .copied()
            .filter(|&i| i >= from)
            .map(|i| i + this[i..].split(' ').next().unwrap_or("").len())
            .find(|&q| {
                let before = &this[..q];
                this.as_bytes().get(q) == Some(&b' ')
                    && this
                        .as_bytes()
                        .get(q + 1)
                        .is_some_and(|c| c.is_ascii_alphabetic())
                    && before.matches('$').count().is_multiple_of(2)
                    && before.matches('{').count() == before.matches('}').count()
            })
            .unwrap_or_else(|| {
                eprintln!("dl3-keys: no space for a {kind} edit in line {line}");
                std::process::exit(1)
            })
    };
    // What a keystroke changes (`old` at `at_byte` becomes `new`; the next
    // keystroke changes it back), and the watched page.
    let (at_byte, old, new, page): (u64, String, String, u32) = match kind.as_str() {
        "letter" => ((offset + pos() + 2) as u64, String::new(), "x".into(), page),
        "sentence" => (
            (offset + pos() + 2) as u64,
            String::new(),
            "lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor ".into(),
            page,
        ),
        "newline" => (
            (offset + space_after(pos())) as u64,
            " ".into(),
            "\n".into(),
            page,
        ),
        "split" => (
            (offset + space_after(pos())) as u64,
            " ".into(),
            "\n\n".into(),
            page,
        ),
        "preamble" => {
            let dc = text.find("\\documentclass").unwrap_or_else(|| {
                eprintln!("dl3-keys: no \\documentclass for a preamble edit");
                std::process::exit(1)
            });
            let eol = text[dc..]
                .find('\n')
                .map(|i| dc + i + 1)
                .unwrap_or(text.len());
            (
                eol as u64,
                String::new(),
                "\\newcommand\\flashtexTsevenProbe{}\n".into(),
                0,
            )
        }
        k => {
            eprintln!("dl3-keys: unknown --kind {k}");
            std::process::exit(2)
        }
    };
    let edit = |k: usize| {
        let (from, to) = if k.is_multiple_of(2) {
            (&old, &new)
        } else {
            (&new, &old)
        };
        Edit {
            path: efile.clone(),
            offset: at_byte,
            delete: from.len() as u64,
            insert: to.clone(),
        }
    };
    eprintln!(
        "dl3-keys: {} pages; {kind} on line {line} (page {page}), byte {at_byte}",
        held.count
    );
    let (mut firsts, mut targets, mut dones) = (vec![], vec![], vec![]);
    let overlap = a.iter().any(|x| x == "--overlap");
    // `--no-viewport`: no `viewport` in the COMPILE (as the app sends it
    // while page 1 is on screen).
    let viewport = !a.iter().any(|x| x == "--no-viewport");
    let mut cancelled = 0;
    // (overlap) DONEs still to come for keystrokes already answered
    let mut owed: Vec<i64> = vec![];
    // `--interval-ms MS`: a typist on a clock, as the app's user: keystroke
    // k is sent at k * MS whatever the host is doing, and its latency is
    // the time to the watched page's first `PAGE` from its compile or a
    // later one (the app's measure: the first commit that shows it). A
    // keystroke whose compile is superseded before that page waits for
    // the next compile's.
    if let Some(ms) = arg("--interval-ms").and_then(|v| v.parse::<u64>().ok()) {
        let reqs: Vec<CompileRequest> = (0..keys)
            .map(|k| {
                let mut r = req(id + 1 + k as i64);
                r.viewport = viewport.then_some(page);
                r.edits = vec![edit(k)];
                r
            })
            .collect();
        let first_id = id + 1;
        let last_id = id + keys as i64;
        let mut sender = c.canceller().expect("a sending handle");
        let t_start = Instant::now();
        let sent: std::sync::Arc<std::sync::Mutex<Vec<Instant>>> = Default::default();
        let sent2 = sent.clone();
        let typist = std::thread::spawn(move || {
            for (k, r) in reqs.iter().enumerate() {
                let due = t_start + Duration::from_millis(ms * k as u64);
                if let Some(d) = due.checked_duration_since(Instant::now()) {
                    std::thread::sleep(d);
                }
                sent2.lock().unwrap().push(Instant::now());
                sender.compile(r).expect("send COMPILE");
            }
        });
        let (mut started, mut covered, mut lat) = (0i64, 0usize, vec![]);
        let (mut cancelled, mut done_last) = (0, false);
        while !done_last {
            let ev = c
                .next_event()
                .expect("read")
                .expect("host closed the connection");
            let now = Instant::now();
            match ev {
                Event::Started(j) => started = j.int_field("id").unwrap_or(0),
                Event::Page(p) if p.index == page && started >= first_id => {
                    let upto = (started - first_id + 1) as usize;
                    let s = sent.lock().unwrap();
                    while covered < upto.min(s.len()) {
                        let l = (now - s[covered]).as_secs_f64() * 1e3;
                        println!(
                            "{}",
                            Json::Obj(vec![
                                ("key".into(), Json::Int(covered as i64)),
                                ("edited_page_ms".into(), Json::Num(l)),
                                ("by".into(), Json::Int(started)),
                            ])
                        );
                        lat.push(l);
                        covered += 1;
                    }
                }
                Event::Done(d) => {
                    println!("{}", Json::Obj(vec![("done".into(), d.clone())]));
                    if d.str_field("status") == Some("cancelled") {
                        cancelled += 1;
                    }
                    done_last = d.int_field("id") == Some(last_id);
                }
                Event::Error(e) => {
                    eprintln!("dl3-keys: host error: {e}");
                    std::process::exit(1)
                }
                _ => {}
            }
        }
        typist.join().expect("the typist");
        let unpainted = keys - covered;
        let sum = |v: &mut Vec<f64>| {
            Json::Obj(vec![
                ("n".into(), Json::Int(v.len() as i64)),
                ("p50".into(), Json::Num(pct(v, 0.5))),
                ("p95".into(), Json::Num(pct(v, 0.95))),
                ("max".into(), Json::Num(pct(v, 1.0))),
            ])
        };
        println!(
            "{}",
            Json::Obj(vec![
                ("summary".into(), s(main.as_str())),
                ("kind".into(), s(kind.as_str())),
                ("interval_ms".into(), Json::Int(ms as i64)),
                ("pages".into(), Json::Int(held.count as i64)),
                ("edited_page".into(), Json::Int(page as i64)),
                ("edited_page_ms".into(), sum(&mut lat)),
                ("unpainted".into(), Json::Int(unpainted as i64)),
                ("cancelled".into(), Json::Int(cancelled)),
            ])
        );
        let _ = std::io::stdout().flush();
        let _ = c.bye();
        return;
    }
    for k in 0..keys {
        if overlap {
            id += 1;
            let mut r = req(id);
            r.viewport = viewport.then_some(page);
            r.edits = vec![edit(k)];
            let t0 = Instant::now();
            c.compile(&r).expect("send COMPILE");
            owed.push(id);
            let (mut mine, mut first, mut target) = (false, None, None);
            loop {
                let ev = c
                    .next_event()
                    .expect("read")
                    .expect("host closed the connection");
                let ms = t0.elapsed().as_secs_f64() * 1e3;
                match ev {
                    Event::Started(j) => {
                        mine = j.int_field("id") == Some(id);
                        if mine && j.get("keep").and_then(Json::as_bool) != Some(true) {
                            held.lines.clear();
                        }
                    }
                    Event::Page(p) => {
                        if mine {
                            first.get_or_insert(ms);
                            if p.index == page {
                                target = Some(ms);
                                break;
                            }
                        }
                    }
                    Event::Done(d) => {
                        let did = d.int_field("id").unwrap_or(-1);
                        println!("{}", Json::Obj(vec![("done".into(), d.clone())]));
                        owed.retain(|&x| x != did);
                        if d.str_field("status") == Some("cancelled") {
                            cancelled += 1;
                        }
                        if did == id {
                            dones.push(ms);
                            break;
                        }
                    }
                    Event::Error(e) => {
                        eprintln!("dl3-keys: host error: {e}");
                        std::process::exit(1)
                    }
                    _ => {}
                }
            }
            let n = |v: Option<f64>| v.map(Json::Num).unwrap_or(Json::Null);
            println!(
                "{}",
                Json::Obj(vec![
                    ("key".into(), Json::Int(k as i64)),
                    ("first_page_ms".into(), n(first)),
                    ("edited_page_ms".into(), n(target)),
                ])
            );
            if let Some(v) = first {
                firsts.push(v);
            }
            if let Some(v) = target {
                targets.push(v);
            }
            std::thread::sleep(gap);
            continue;
        }
        id += 1;
        let mut r = req(id);
        r.viewport = viewport.then_some(page);
        r.edits = vec![edit(k)];
        let res = compile(&mut c, &mut held, &r, Some(page));
        let n = |v: Option<f64>| v.map(Json::Num).unwrap_or(Json::Null);
        println!(
            "{}",
            Json::Obj(vec![
                ("key".into(), Json::Int(k as i64)),
                ("first_page_ms".into(), n(res.first_page)),
                ("edited_page_ms".into(), n(res.target_page)),
                ("done_ms".into(), Json::Num(res.done_ms)),
                (
                    "first_index".into(),
                    res.first_index
                        .map(|i| Json::Int(i as i64))
                        .unwrap_or(Json::Null)
                ),
                ("edited_page".into(), Json::Int(page as i64)),
                ("kind".into(), s(kind.as_str())),
                ("later_pages".into(), Json::Int(res.later_pages as i64)),
                ("stale_marked".into(), Json::Bool(res.stale_marked)),
                (
                    "complete".into(),
                    res.complete.map(Json::Bool).unwrap_or(Json::Null)
                ),
                ("host".into(), res.done.clone()),
            ])
        );
        if let Some(v) = res.first_page {
            firsts.push(v);
        }
        if let Some(v) = res.target_page {
            targets.push(v);
        }
        dones.push(res.done_ms);
        std::thread::sleep(gap);
    }
    let sum = |v: &mut Vec<f64>| {
        Json::Obj(vec![
            ("n".into(), Json::Int(v.len() as i64)),
            ("p50".into(), Json::Num(pct(v, 0.5))),
            ("p95".into(), Json::Num(pct(v, 0.95))),
            ("max".into(), Json::Num(pct(v, 1.0))),
        ])
    };
    println!(
        "{}",
        Json::Obj(vec![
            ("summary".into(), s(main.as_str())),
            ("kind".into(), s(kind.as_str())),
            ("pages".into(), Json::Int(held.count as i64)),
            ("edited_page".into(), Json::Int(page as i64)),
            ("first_page_ms".into(), sum(&mut firsts)),
            ("edited_page_ms".into(), sum(&mut targets)),
            ("done_ms".into(), sum(&mut dones)),
        ])
    );
    if overlap {
        // the last keystroke's DONE, and any other still owed
        while !owed.is_empty() {
            if let Some(Event::Done(d)) = c.next_event().expect("read") {
                let did = d.int_field("id").unwrap_or(-1);
                owed.retain(|&x| x != did);
                if d.str_field("status") == Some("cancelled") {
                    cancelled += 1;
                }
            }
        }
        println!(
            "{}",
            Json::Obj(vec![("cancelled".into(), Json::Int(cancelled))])
        );
    }
    let _ = std::io::stdout().flush();
    let _ = c.bye();
}
