//! `dl3-keys`: keystroke-to-page latency through a running `flashtex-host`
//! (protocol 3.1, `"incremental": true`), measured on the client side of the
//! socket, as the app sees it.
//!
//!     dl3-keys --socket PATH --root DIR --main main.tex [--output-dir DIR]
//!         [--keys N] [--at FRACTION] [--gap-ms MS]
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
}

fn compile(c: &mut Client, held: &mut Held, req: &CompileRequest, target: Option<u32>) -> Result {
    let t0 = Instant::now();
    c.compile(req).expect("send COMPILE");
    let (mut first_page, mut target_page, mut first_index) = (None, None, None);
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
            Event::Done(d) => break d,
            _ => {}
        }
    };
    held.count = done.int_field("pages").unwrap_or(0) as usize;
    let n = held.count as u32;
    held.lines.retain(|&i, _| i < n);
    Result {
        first_page,
        target_page,
        first_index,
        done_ms: t0.elapsed().as_secs_f64() * 1e3,
        done,
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
        eprintln!("usage: dl3-keys --socket PATH --root DIR --main FILE [--output-dir DIR] [--keys N] [--at FRACTION] [--gap-ms MS]");
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
                ("done_ms".into(), Json::Num(r.done_ms)),
                ("host".into(), r.done.clone()),
            ])
        );
        if r.done.str_field("mode") == Some("unchanged") {
            break;
        }
    }
    // A prose line on one page, about `at` into the document.
    let path = std::path::Path::new(&root).join(&main);
    let text = std::fs::read_to_string(&path).expect("read the main file");
    let lines: Vec<&str> = text.split('\n').collect();
    let want = ((held.count as f64 * at) as u32).min(held.count.saturating_sub(1) as u32);
    let main_file = |f: u32| {
        held.files
            .get(&f)
            .is_some_and(|p| p.ends_with(&format!("/{main}")))
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
    let (line, page) = (1..=lines.len() as u32)
        .filter(|&l| lines[l as usize - 1].split(' ').count() > 40)
        .filter_map(|l| {
            let p = on_pages(l);
            (p.len() == 1).then(|| (l, p[0]))
        })
        .min_by_key(|&(_, p)| p.abs_diff(want))
        .unwrap_or_else(|| {
            eprintln!("dl3-keys: no prose line on one page");
            std::process::exit(1)
        });
    let offset: usize = lines[..line as usize - 1].iter().map(|l| l.len() + 1).sum();
    let this = lines[line as usize - 1];
    // Inside the first plain word after the middle of the line.
    let mut pos = this.len() / 2;
    loop {
        pos += this[pos..].find(' ').expect("a word") + 1;
        let w = this[pos..].split(' ').next().unwrap();
        if w.len() >= 4 && w.bytes().all(|c| c.is_ascii_lowercase()) {
            break;
        }
    }
    let at_byte = (offset + pos + 2) as u64;
    eprintln!(
        "dl3-keys: {} pages; typing on line {line} (page {page}), byte {at_byte}",
        held.count
    );
    let (mut firsts, mut targets, mut dones) = (vec![], vec![], vec![]);
    let overlap = a.iter().any(|x| x == "--overlap");
    let mut cancelled = 0;
    // (overlap) DONEs still to come for keystrokes already answered
    let mut owed: Vec<i64> = vec![];
    for k in 0..keys {
        if overlap {
            id += 1;
            let mut r = req(id);
            r.viewport = Some(page);
            r.edits = vec![if k % 2 == 0 {
                Edit {
                    path: main.clone(),
                    offset: at_byte,
                    delete: 0,
                    insert: "x".into(),
                }
            } else {
                Edit {
                    path: main.clone(),
                    offset: at_byte,
                    delete: 1,
                    insert: String::new(),
                }
            }];
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
        r.viewport = Some(page);
        r.edits = vec![if k % 2 == 0 {
            Edit {
                path: main.clone(),
                offset: at_byte,
                delete: 0,
                insert: "x".into(),
            }
        } else {
            Edit {
                path: main.clone(),
                offset: at_byte,
                delete: 1,
                insert: String::new(),
            }
        }];
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
