//! `dl3-dump FILE`: print a file of display-list-v3 frames as JSON lines, one
//! per frame (the positions checker and people read these). Pages list their
//! items in order as compact arrays:
//!
//!   ["g", font, code, x, y, col]   glyph        ["r", kind, x, y, w, h]  rule
//!   ["p", n] / ["clip", n]          path / clip  ["img", id, m] / ["form", id, m]
//!   ["q"] / ["Q"]                   save/restore ["fill", [..]] / ["stroke", [..]]
//!   ["m", n] matrix  ["span", n]  ["tr", mode]  ["unsupported", n]
//!
//! `--summary` prints one line per frame without the items. `--bench`
//! decodes the file's PAGE and FORM frames repeatedly and prints the decode
//! throughput (what a client spends turning bytes into items).

use flashtex_display_list::client::{decode_event, Event};
use flashtex_display_list::frame::read_frame;
use flashtex_display_list::json::{s, Json};
use flashtex_display_list::kind;
use flashtex_display_list::page::{Item, Page, StreamKind};
use flashtex_display_list::sha256::hex;
use std::io::{BufReader, BufWriter, Write};

fn ints(v: &[i64]) -> Json {
    Json::Arr(v.iter().map(|&i| Json::Int(i)).collect())
}

fn page_json(tag: &str, p: &Page, summary: bool) -> Json {
    let mut kv = vec![
        ("kind".to_string(), s(tag)),
        ("index".to_string(), Json::Int(p.index as i64)),
        ("flags".to_string(), Json::Int(p.flags as i64)),
        ("width".to_string(), Json::Int(p.width as i64)),
        ("height".to_string(), Json::Int(p.height as i64)),
        ("counts".to_string(), ints(&p.counts.map(|c| c as i64))),
        (
            "box".to_string(),
            Json::Arr(p.pdf_box.iter().map(|&v| Json::Num(v)).collect()),
        ),
        ("hash".to_string(), s(hex(&p.hash))),
        ("items_count".to_string(), Json::Int(p.items.len() as i64)),
        (
            "unsupported".to_string(),
            Json::Arr(p.unsupported.iter().map(|u| s(u.as_str())).collect()),
        ),
    ];
    if !summary {
        kv.push((
            "matrices".into(),
            Json::Arr(
                p.matrices
                    .iter()
                    .map(|m| Json::Arr(m.iter().map(|&v| Json::Num(v)).collect()))
                    .collect(),
            ),
        ));
        let items = p
            .items
            .iter()
            .map(|it| match it {
                Item::Glyph {
                    font,
                    code,
                    x,
                    y,
                    col,
                } => Json::Arr(vec![
                    s("g"),
                    Json::Int(*font as i64),
                    Json::Int(*code as i64),
                    Json::Int(*x as i64),
                    Json::Int(*y as i64),
                    Json::Int(*col as i64),
                ]),
                Item::Rule { kind, x, y, w, h } => Json::Arr(vec![
                    s("r"),
                    Json::Int(*kind as i64),
                    Json::Int(*x as i64),
                    Json::Int(*y as i64),
                    Json::Int(*w as i64),
                    Json::Int(*h as i64),
                ]),
                Item::Path(n) => Json::Arr(vec![s("p"), Json::Int(*n as i64)]),
                Item::Clip(n) => Json::Arr(vec![s("clip"), Json::Int(*n as i64)]),
                Item::Image { id, matrix } => Json::Arr(vec![
                    s("img"),
                    Json::Int(*id as i64),
                    Json::Int(*matrix as i64),
                ]),
                Item::Form { id, matrix } => Json::Arr(vec![
                    s("form"),
                    Json::Int(*id as i64),
                    Json::Int(*matrix as i64),
                ]),
                Item::Save => Json::Arr(vec![s("q")]),
                Item::Restore => Json::Arr(vec![s("Q")]),
                Item::FillColor(c) => Json::Arr(vec![
                    s("fill"),
                    Json::Arr(c.0.iter().map(|&v| Json::Num(v)).collect()),
                ]),
                Item::StrokeColor(c) => Json::Arr(vec![
                    s("stroke"),
                    Json::Arr(c.0.iter().map(|&v| Json::Num(v)).collect()),
                ]),
                Item::Matrix(n) => Json::Arr(vec![s("m"), Json::Int(*n as i64)]),
                Item::Span(n) => Json::Arr(vec![s("span"), Json::Int(*n as i64)]),
                Item::TextRender(m) => Json::Arr(vec![s("tr"), Json::Int(*m as i64)]),
                Item::Unsupported(n) => Json::Arr(vec![s("unsupported"), Json::Int(*n as i64)]),
            })
            .collect();
        kv.push(("items".into(), Json::Arr(items)));
        kv.push(("paths".into(), Json::Int(p.paths.len() as i64)));
        kv.push((
            "links".into(),
            Json::Arr(
                p.links
                    .iter()
                    .map(|l| {
                        Json::Arr(vec![
                            ints(&l.rect.map(|v| v as i64)),
                            Json::Int(l.kind.clone() as i64),
                            s(String::from_utf8_lossy(&l.data).into_owned()),
                        ])
                    })
                    .collect(),
            ),
        ));
        kv.push((
            "dests".into(),
            Json::Arr(
                p.dests
                    .iter()
                    .map(|d| {
                        Json::Arr(vec![
                            s(String::from_utf8_lossy(&d.name).into_owned()),
                            Json::Int(d.kind as i64),
                            ints(&d.rect.map(|v| v as i64)),
                        ])
                    })
                    .collect(),
            ),
        ));
    }
    Json::Obj(kv)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let summary = args.iter().any(|a| a == "--summary");
    let Some(path) = args.iter().find(|a| !a.starts_with("--")) else {
        eprintln!("usage: dl3-dump [--summary | --bench] FILE");
        std::process::exit(2);
    };
    let f = std::fs::File::open(path).unwrap_or_else(|e| {
        eprintln!("dl3-dump: {path}: {e}");
        std::process::exit(1)
    });
    let mut r = BufReader::with_capacity(1 << 20, f);
    if args.iter().any(|a| a == "--bench") {
        bench(&mut r);
        return;
    }
    let out = std::io::stdout();
    let mut w = BufWriter::new(out.lock());
    loop {
        let (k, body) = match read_frame(&mut r) {
            Ok(Some(x)) => x,
            Ok(None) => break,
            Err(e) => {
                eprintln!("dl3-dump: {e}");
                std::process::exit(1);
            }
        };
        let j = match decode_event(k, body) {
            Ok(Event::Page(p)) => page_json("page", &p, summary),
            Ok(Event::Form(p)) => page_json("form", &p, summary),
            Ok(Event::Font(f)) => Json::Obj(vec![
                ("kind".into(), s("font")),
                ("id".into(), Json::Int(f.id as i64)),
                ("key".into(), s(hex(&f.key))),
                ("program_bytes".into(), Json::Int(f.program.len() as i64)),
                (
                    "info".into(),
                    if summary { Json::Null } else { f.info.clone() },
                ),
            ]),
            Ok(Event::Sources(src)) => {
                let mut j = src.to_json();
                if let Json::Obj(kv) = &mut j {
                    kv.insert(0, ("kind".into(), s("sources")));
                }
                j
            }
            Ok(ev) => {
                let (name, body) = match ev {
                    Event::Started(j) => ("started", j),
                    Event::Image(j) => ("image", j),
                    Event::Diagnostic(j) => ("diagnostic", j),
                    Event::Diag(d) => ("diag", d.to_json()),
                    Event::Done(j) => ("done", j),
                    Event::Error(j) => ("error", j),
                    _ => (kind::name(k), Json::Null),
                };
                Json::Obj(vec![("kind".into(), s(name)), ("body".into(), body)])
            }
            Err(e) => {
                eprintln!("dl3-dump: frame {}: {e}", kind::name(k));
                std::process::exit(1);
            }
        };
        let _ = writeln!(w, "{}", j);
    }
}

fn bench(r: &mut impl std::io::Read) {
    let mut frames = vec![];
    while let Ok(Some((k, body))) = read_frame(r) {
        if k == kind::PAGE || k == kind::FORM {
            frames.push((k, body));
        }
    }
    let bytes: usize = frames.iter().map(|(_, b)| b.len() + 5).sum();
    let (mut items, mut best) = (0usize, f64::MAX);
    for _ in 0..20 {
        let t = std::time::Instant::now();
        items = 0;
        for (k, b) in &frames {
            let kind = if *k == kind::PAGE {
                StreamKind::Page
            } else {
                StreamKind::Form
            };
            items += Page::decode(kind, b).expect("decode").items.len();
        }
        best = best.min(t.elapsed().as_secs_f64());
    }
    println!(
        "{{\"frames\":{},\"bytes\":{},\"items\":{},\"best_seconds\":{:.6},\"mb_per_s\":{:.0},\"items_per_s\":{:.0}}}",
        frames.len(),
        bytes,
        items,
        best,
        bytes as f64 / 1e6 / best,
        items as f64 / best
    );
}
