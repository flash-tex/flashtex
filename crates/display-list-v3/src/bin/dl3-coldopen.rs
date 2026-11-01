//! `dl3-coldopen`: a document's first open through a running `flashtex-host`
//! (protocol 3.1, `"incremental": true`, `progress-v1`), measured on the
//! client side of the socket, as the app sees it (lane COLD-OPEN, DESIGN.md
//! §1.2 "Opening").
//!
//!     dl3-coldopen --socket PATH --root DIR --main FILE [--output-dir DIR]
//!         [--viewport INDEX] [--external-tools auto|off]
//!         [--edit-at-ms MS --edit-line N [--edit-file FILE] [--edit-viewport]]
//!
//! Sends one `COMPILE` (the open: no `.aux` unless the output directory has
//! one) and prints one JSON line when it is done: the time from sending it to
//! the first `PAGE` (`first_page_ms`), to the `viewport` page's first `PAGE`
//! (`viewport_ms`), to the moment every page of the final count had arrived
//! at least once (`all_seen_ms`), to `DONE` (`done_ms`), when each pass
//! started (`PROGRESS`), how many `PAGE` frames came (`page_frames`: pages
//! sent again in later passes count again), and the host's `DONE`.
//!
//! `--edit-at-ms MS --edit-line N`: MS milliseconds after the open's
//! `COMPILE`, a keystroke: a letter inserted into the first plain word
//! (4+ lowercase letters) after the start of line N (1-based) of FILE
//! (`--edit-file`, default the main file), sent as a second `COMPILE`, as the
//! app sends a keystroke during the initial compile. Its line says how far
//! the open had got then (`open_pages_then`: the highest page index seen + 1,
//! `open_pass_then`), the time from sending it to the first `PAGE` of the
//! edit's compile that shows line N (`edited_page_ms`; the page is found from
//! the spans, so the edit may be ahead of the open's progress), and to its
//! `DONE` (`edit_done_ms`). With `--edit-viewport` the keystroke's `COMPILE`
//! carries the page line N was seen on, if it was (as the app sends the page
//! on screen).
//!
//! With `--external-tools auto` (a trusted project, as the app sends it) the
//! open lasts until the host's `TOOL` `settled` for it: makeindex, bibtex or
//! biber and the follow-up compiles they cause are part of it (`dones`: every
//! `DONE`: id, ms, cause, passes, their modes, instructions; `done_ms`: the last; `settled_ms`; `tool_runs`).

use flashtex_display_list::client::{Client, CompileRequest, Edit, Event};
use flashtex_display_list::json::{s, Json};
use flashtex_display_list::page::Item;
use flashtex_display_list::{kind, PROGRESS_CAPABILITY};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::time::{Duration, Instant};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arg = |k: &str| {
        a.iter()
            .position(|x| x == k)
            .and_then(|i| a.get(i + 1))
            .cloned()
    };
    let flag = |k: &str| a.iter().any(|x| x == k);
    let (Some(socket), Some(root), Some(main)) = (arg("--socket"), arg("--root"), arg("--main"))
    else {
        eprintln!("usage: dl3-coldopen --socket PATH --root DIR --main FILE [--output-dir DIR] [--viewport INDEX] [--external-tools auto|off] [--edit-at-ms MS --edit-line N [--edit-file FILE] [--edit-viewport]]");
        std::process::exit(2);
    };
    let num = |k: &str| -> Option<u64> {
        arg(k).map(|v| {
            v.parse().unwrap_or_else(|_| {
                eprintln!("dl3-coldopen: {k} {v}: not a number");
                std::process::exit(2)
            })
        })
    };
    let viewport = num("--viewport").map(|v| v as u32);
    let edit_at = num("--edit-at-ms").map(Duration::from_millis);
    let edit_line = num("--edit-line").map(|v| v as u32);
    if edit_at.is_some() != edit_line.is_some() {
        eprintln!("dl3-coldopen: --edit-at-ms and --edit-line go together");
        std::process::exit(2);
    }
    let efile = arg("--edit-file").unwrap_or_else(|| main.clone());
    // The keystroke: a letter after the first two letters of the first plain
    // word after the start of line N.
    let edit = edit_line.map(|line| {
        let path = std::path::Path::new(&root).join(&efile);
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("dl3-coldopen: {}: {e}", path.display());
            std::process::exit(2)
        });
        let lines: Vec<&str> = text.split('\n').collect();
        if line == 0 || line as usize > lines.len() {
            eprintln!(
                "dl3-coldopen: --edit-line {line}: {efile} has {} lines",
                lines.len()
            );
            std::process::exit(2)
        }
        let offset: usize = lines[..line as usize - 1].iter().map(|l| l.len() + 1).sum();
        let this = lines[line as usize - 1];
        let plain = |w: &str| w.len() >= 4 && w.bytes().all(|c| c.is_ascii_lowercase());
        let at = this
            .match_indices(' ')
            .map(|(i, _)| i + 1)
            .find(|&i| plain(this[i..].split(' ').next().unwrap_or("")))
            .unwrap_or_else(|| {
                eprintln!("dl3-coldopen: line {line} has no plain lowercase word after its start");
                std::process::exit(2)
            });
        Edit {
            path: efile.clone(),
            offset: (offset + at + 2) as u64,
            delete: 0,
            insert: "x".into(),
        }
    });
    let mut c =
        Client::connect_accepting(socket.as_ref(), &[PROGRESS_CAPABILITY]).unwrap_or_else(|e| {
            eprintln!("dl3-coldopen: {socket}: {e}");
            std::process::exit(1)
        });
    let req = |id: i64| {
        let mut r = CompileRequest::new(id, &root, &main);
        r.output_dir = arg("--output-dir");
        r.incremental = true;
        r.external_tools = arg("--external-tools");
        r
    };
    let mut open = req(1);
    open.viewport = viewport;
    let t0 = Instant::now();
    let ms = |t: Instant| (t - t0).as_secs_f64() * 1e3;
    c.compile(&open).expect("send COMPILE");
    // The keystroke goes from another thread at its time, whatever the host
    // is doing; `sent` is when it went, `then` how far the open had got.
    let progress: std::sync::Arc<std::sync::Mutex<(u32, i64)>> = Default::default();
    let sent: std::sync::Arc<std::sync::Mutex<Option<(Instant, u32, i64)>>> = Default::default();
    // (the page line N was seen on, for `--edit-viewport`)
    let line_page: std::sync::Arc<std::sync::Mutex<Option<u32>>> = Default::default();
    let typist = edit.clone().zip(edit_at).map(|(e, at)| {
        let mut sender = c.canceller().expect("a sending handle");
        let (progress, sent, line_page) = (progress.clone(), sent.clone(), line_page.clone());
        let mut r = req(2);
        r.edits = vec![e];
        let with_viewport = flag("--edit-viewport");
        std::thread::spawn(move || {
            if let Some(d) = (t0 + at).checked_duration_since(Instant::now()) {
                std::thread::sleep(d);
            }
            if with_viewport {
                r.viewport = *line_page.lock().unwrap();
            }
            let (pages, pass) = *progress.lock().unwrap();
            *sent.lock().unwrap() = Some((Instant::now(), pages, pass));
            sender.compile(&r).expect("send COMPILE");
        })
    });
    let mut spans: HashMap<u32, (u32, u32)> = HashMap::new();
    let mut files: HashMap<u32, String> = HashMap::new();
    let mut started = 0i64;
    let (mut first_page, mut viewport_ms, mut frames) = (None, None, 0u64);
    let mut seen: HashSet<u32> = HashSet::new();
    let mut arrival: HashMap<u32, f64> = HashMap::new();
    let mut passes: Vec<(i64, f64)> = vec![];
    let (mut edited_ms, mut edited_index) = (None, None);
    let mut dones: Vec<(i64, f64, Json)> = vec![];
    let want_done = if edit.is_some() { 2 } else { 1 };
    let shows_line = |p: &flashtex_display_list::page::Page,
                      spans: &HashMap<u32, (u32, u32)>,
                      files: &HashMap<u32, String>,
                      line: u32| {
        p.items.iter().any(|it| match it {
            Item::Span(sp) if *sp != 0 => spans.get(sp).is_some_and(|&(f, l)| {
                l == line
                    && files
                        .get(&f)
                        .is_some_and(|n| n == &efile || n.ends_with(&format!("/{efile}")))
            }),
            _ => false,
        })
    };
    // With the external tools on, the open goes on until they settle
    // (`TOOL` `settled`): their follow-up compiles are part of it.
    let tools = arg("--external-tools").as_deref() == Some("auto");
    let mut settled: Option<(f64, Json)> = None;
    let mut tool_runs = 0i64;
    while dones.iter().all(|d| d.0 != want_done) || (tools && settled.is_none()) {
        let ev = c
            .next_event()
            .expect("read")
            .expect("host closed the connection");
        let now = Instant::now();
        match ev {
            Event::Started(j) => started = j.int_field("id").unwrap_or(0),
            Event::Sources(src) => {
                for (i, p) in src.files {
                    files.insert(i, p);
                }
                for (i, f, l) in src.spans {
                    spans.insert(i, (f, l));
                }
            }
            Event::Page(p) => {
                frames += 1;
                let t = ms(now);
                if started == 1 {
                    first_page.get_or_insert(t);
                    if Some(p.index) == viewport {
                        viewport_ms.get_or_insert(t);
                    }
                    let mut pr = progress.lock().unwrap();
                    pr.0 = pr.0.max(p.index + 1);
                }
                if seen.insert(p.index) {
                    arrival.insert(p.index, t);
                }
                if let Some(line) = edit_line {
                    if shows_line(&p, &spans, &files, line) {
                        let mut lp = line_page.lock().unwrap();
                        lp.get_or_insert(p.index);
                        if started == 2 && edited_ms.is_none() {
                            if let Some((at, _, _)) = *sent.lock().unwrap() {
                                edited_ms = Some((now - at).as_secs_f64() * 1e3);
                                edited_index = Some(p.index);
                            }
                        }
                    }
                }
            }
            Event::Other(k, body) if k == kind::PROGRESS => {
                let j = std::str::from_utf8(&body)
                    .ok()
                    .and_then(|t| Json::parse(t).ok());
                if let Some(pass) = j.as_ref().and_then(|j| j.int_field("pass")) {
                    if j.as_ref().and_then(|j| j.int_field("id")) == Some(started)
                        && passes.last().is_none_or(|p| p.0 != pass)
                    {
                        passes.push((pass, ms(now)));
                    }
                    if started == 1 {
                        progress.lock().unwrap().1 = pass;
                    }
                }
            }
            Event::Error(e) => {
                eprintln!("dl3-coldopen: host error: {e}");
                std::process::exit(1)
            }
            Event::Tool(j) => {
                let id = j.int_field("id");
                match j.str_field("event") {
                    Some("run") => tool_runs += 1,
                    Some("settled") if id == Some(want_done) => settled = Some((ms(now), j)),
                    _ => {}
                }
            }
            Event::Done(d) => {
                let id = d.int_field("id").unwrap_or(-1);
                dones.push((id, ms(now), d));
            }
            _ => {}
        }
    }
    if let Some(t) = typist {
        t.join().expect("the typist");
    }
    let (last_id, done_ms, done) = dones.last().cloned().unwrap();
    let count = done.int_field("pages").unwrap_or(0) as u32;
    let all_seen = (0..count)
        .map(|i| arrival.get(&i).copied())
        .collect::<Option<Vec<f64>>>()
        .map(|v| v.into_iter().fold(0.0, f64::max));
    let n = |v: Option<f64>| v.map(Json::Num).unwrap_or(Json::Null);
    let mut kv: Vec<(String, Json)> = vec![
        ("coldopen".into(), s(main.as_str())),
        ("pages".into(), Json::Int(count as i64)),
        ("first_page_ms".into(), n(first_page)),
        (
            "viewport".into(),
            viewport.map(|v| Json::Int(v as i64)).unwrap_or(Json::Null),
        ),
        ("viewport_ms".into(), n(viewport_ms)),
        ("all_seen_ms".into(), n(all_seen)),
        ("done_ms".into(), Json::Num(done_ms)),
        ("done_id".into(), Json::Int(last_id)),
        ("page_frames".into(), Json::Int(frames as i64)),
        // every DONE: (id, ms, cause, passes, pass_modes, engine-thread
        // instructions in thousands)
        (
            "dones".into(),
            Json::Arr(
                dones
                    .iter()
                    .map(|(id, t, d)| {
                        Json::Arr(vec![
                            Json::Int(*id),
                            Json::Num(*t),
                            d.get("cause").cloned().unwrap_or(Json::Null),
                            d.get("passes").cloned().unwrap_or(Json::Null),
                            d.get("pass_modes").cloned().unwrap_or(Json::Null),
                            d.get("stages")
                                .and_then(|s| s.get("instr_k"))
                                .cloned()
                                .unwrap_or(Json::Null),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("tool_runs".into(), Json::Int(tool_runs)),
        ("settled_ms".into(), n(settled.as_ref().map(|s| s.0))),
        (
            "pass_starts_ms".into(),
            Json::Arr(
                passes
                    .iter()
                    .map(|(p, t)| Json::Arr(vec![Json::Int(*p), Json::Num(*t)]))
                    .collect(),
            ),
        ),
    ];
    if let (Some(line), Some(at)) = (edit_line, edit_at) {
        let sent = *sent.lock().unwrap();
        let open_done = dones.iter().find(|d| d.0 == 1);
        kv.extend([
            ("edit_line".into(), Json::Int(line as i64)),
            ("edit_at_ms".into(), Json::Num(at.as_secs_f64() * 1e3)),
            ("edit_sent_ms".into(), n(sent.map(|x| ms(x.0)))),
            (
                "open_pages_then".into(),
                Json::Int(sent.map_or(0, |x| x.1 as i64)),
            ),
            ("open_pass_then".into(), Json::Int(sent.map_or(0, |x| x.2))),
            (
                "edited_page".into(),
                edited_index
                    .map(|i| Json::Int(i as i64))
                    .unwrap_or(Json::Null),
            ),
            ("edited_page_ms".into(), n(edited_ms)),
            ("edit_done_ms".into(), n(sent.map(|x| done_ms - ms(x.0)))),
            (
                "open_status".into(),
                open_done
                    .and_then(|d| d.2.str_field("status"))
                    .map(s)
                    .unwrap_or(Json::Null),
            ),
            (
                "open_host".into(),
                open_done.map(|d| d.2.clone()).unwrap_or(Json::Null),
            ),
        ]);
    }
    kv.push(("host".into(), done));
    println!("{}", Json::Obj(kv));
    let _ = std::io::stdout().flush();
    let _ = c.bye();
}
