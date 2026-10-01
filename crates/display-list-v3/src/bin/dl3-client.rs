//! `dl3-client`: compile a project through a running `flashtex-host` and
//! report what arrived and how fast (the app lane's reference client, and
//! the socket round-trip measurement).
//!
//!     dl3-client --socket /tmp/flashtex.sock --root /path/to/project --main main.tex \
//!         [--repeat N] [--reuse-fonts] [--save out.dl3] [--output-dir DIR] [--quiet]
//!         [--diag FILE]
//!
//! Per compile it prints one JSON line: time to STARTED, to the first PAGE
//! and to DONE (ms, measured here from sending COMPILE), pages, forms,
//! fonts, bytes received, and the decode throughput (MB/s of frames decoded
//! into pages, fonts and resources by this client).
//!
//! `--diag FILE` accepts `diag-v1` (spec §6.7) and writes every `DIAG` of
//! every compile to FILE, one JSON object a line.

use flashtex_display_list::client::{decode_event, Client, CompileRequest, Event};
use flashtex_display_list::frame::{read_frame, write_frame};
use flashtex_display_list::json::{s, Json};
use std::io::Write;
use std::time::Instant;

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
        eprintln!("usage: dl3-client --socket PATH --root DIR --main FILE [--repeat N] [--save FILE] [--quiet]");
        std::process::exit(2);
    };
    let repeat: usize = arg("--repeat").and_then(|v| v.parse().ok()).unwrap_or(1);
    let save = arg("--save");
    let quiet = a.iter().any(|x| x == "--quiet");
    // --reuse-fonts: after the first compile, tell the host which font
    // programs this client holds (as the app would), so they are not resent.
    let reuse = a.iter().any(|x| x == "--reuse-fonts");
    let mut held: Vec<String> = Vec::new();
    let diag_out = arg("--diag");
    let accept: &[&str] = if diag_out.is_some() {
        &[flashtex_display_list::diag::CAPABILITY]
    } else {
        &[]
    };
    let mut diag_w = diag_out
        .as_ref()
        .map(|p| std::io::BufWriter::new(std::fs::File::create(p).unwrap()));
    let mut c = Client::connect_accepting(socket.as_ref(), accept).unwrap_or_else(|e| {
        eprintln!("dl3-client: {socket}: {e}");
        std::process::exit(1)
    });
    if !quiet {
        eprintln!("host: {}", c.hello);
    }
    let mut failed = false;
    for run in 0..repeat {
        let id = run as i64 + 1;
        let mut req = CompileRequest::new(id, &root, &main);
        req.output_dir = arg("--output-dir");
        if reuse {
            req.have_fonts = held.clone();
        }
        let t0 = Instant::now();
        c.compile(&req).expect("send COMPILE");
        let (mut started, mut first_page, mut pages, mut forms, mut fonts, mut bytes) =
            (None, None, 0, 0, 0, 0u64);
        let mut decode = 0f64;
        let mut saved = save
            .as_ref()
            .map(|p| std::io::BufWriter::new(std::fs::File::create(p).unwrap()));
        let done = loop {
            // Read the raw frame, then decode it, timing the decode alone.
            let frame = read_frame(c.reader()).expect("read");
            let Some((k, body)) = frame else {
                eprintln!("dl3-client: host closed the connection");
                std::process::exit(1)
            };
            bytes += body.len() as u64 + 5;
            if let Some(w) = saved.as_mut() {
                write_frame(w, k, &body).unwrap();
            }
            let td = Instant::now();
            let ev = decode_event(k, body).expect("decode");
            decode += td.elapsed().as_secs_f64();
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            match ev {
                Event::Started(_) => started = Some(ms),
                Event::Page(_) => {
                    pages += 1;
                    first_page.get_or_insert(ms);
                }
                Event::Form(_) => forms += 1,
                Event::Font(f) => {
                    fonts += 1;
                    let k = flashtex_display_list::sha256::hex(&f.key);
                    if !held.contains(&k) {
                        held.push(k);
                    }
                }
                Event::Diagnostic(d) if !quiet => eprintln!("diagnostic: {}", d),
                Event::Diag(d) => {
                    if let Some(w) = diag_w.as_mut() {
                        writeln!(w, "{}", d.to_json()).unwrap();
                        w.flush().unwrap();
                    }
                }
                Event::Error(e) => {
                    eprintln!("error: {}", e);
                    failed = true;
                    break Json::Null;
                }
                Event::Done(d) => break d,
                _ => {}
            }
        };
        let total = t0.elapsed().as_secs_f64() * 1000.0;
        let r = |v: f64| Json::Num((v * 1000.0).round() / 1000.0);
        let line = Json::Obj(vec![
            ("run".into(), Json::Int(id)),
            ("status".into(), s(done.str_field("status").unwrap_or("?"))),
            ("started_ms".into(), started.map(r).unwrap_or(Json::Null)),
            (
                "first_page_ms".into(),
                first_page.map(r).unwrap_or(Json::Null),
            ),
            ("done_ms".into(), r(total)),
            ("pages".into(), Json::Int(pages)),
            ("forms".into(), Json::Int(forms)),
            ("fonts".into(), Json::Int(fonts)),
            ("bytes".into(), Json::Int(bytes as i64)),
            (
                "decode_mb_per_s".into(),
                r(bytes as f64 / 1e6 / decode.max(1e-9)),
            ),
            ("host".into(), done),
        ]);
        println!("{}", line);
        let _ = std::io::stdout().flush();
        if pages == 0 {
            failed = true;
        }
    }
    let _ = c.bye();
    std::process::exit(if failed { 1 } else { 0 });
}
