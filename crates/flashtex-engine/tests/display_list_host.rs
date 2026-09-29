//! The engine host end to end, through the MIT client
//! (`flashtex_display_list::client`): start `flashtex-host`, connect,
//! compile a parity fixture (hyperref-toc: text, rules, links, destinations)
//! and receive every page as the engine ships it out; check that the pages
//! decode, that each one's fonts and source spans arrived before it, that
//! their number is the PDF's, that `DONE` reports the run; then cancel a
//! compile. Skips where there is no TeX Live (e.g. CI).
//!
//! Prints the socket round-trip timings (time to the first page, to DONE).
#![cfg(feature = "kpathsea")]

use flashtex_display_list::client::{Client, CompileRequest, Event};
use flashtex_display_list::page::{Item, LinkKind};
use flashtex_engine::resolver::find_texlive_bin;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Instant;

struct Host(Child, PathBuf);

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
        let _ = std::fs::remove_file(&self.1);
    }
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

#[test]
fn host_compiles_a_fixture_and_streams_every_page() {
    if find_texlive_bin().is_none() {
        eprintln!("no TeX Live found; skipping");
        return;
    }
    let engine = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let host_bin = Path::new(env!("CARGO_BIN_EXE_flashtex-host"));
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let pool = manifest.join("pdftex.pool");
    let base = std::env::temp_dir().join(format!("flashtex-dl-host-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (fmt, proj) = (base.join("fmt"), base.join("proj"));
    std::fs::create_dir_all(&fmt).unwrap();
    // pdflatex.fmt, the fmtutil way.
    let st = Command::new(engine)
        .args([
            "-ini",
            "-jobname=pdflatex",
            "-progname=pdflatex",
            "-etex",
            "-translate-file=cp227.tcx",
            "pdflatex.ini",
        ])
        .current_dir(&fmt)
        .env("FLASHTEX_POOL", &pool)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(st.success(), "building pdflatex.fmt failed");
    copy_dir(
        &manifest.join("../../fixtures/real-world/hyperref-toc"),
        &proj,
    );

    let sock = base.join("host.sock");
    let mut child = Command::new(host_bin)
        .args([
            "--socket",
            sock.to_str().unwrap(),
            "--engine",
            engine.to_str().unwrap(),
        ])
        .env("FLASHTEX_POOL", &pool)
        .env("FLASHTEX_FORMATS", &fmt)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    // The host prints one line when it listens.
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    assert!(line.contains("listening"), "{line}");
    let _host = Host(child, sock.clone());

    let mut c = Client::connect(&sock).unwrap();
    assert_eq!(c.hello.str_field("protocol"), Some("display-list-v3"));
    let out_dir = base.join("out");
    let mut timings = vec![];
    // Three compiles: the first writes the .aux/.toc files, after which the
    // document is complete (TOC, links) and stable.
    let mut last = None;
    for id in 1..=3 {
        let mut req = CompileRequest::new(id, proj.to_str().unwrap(), "main.tex");
        req.output_dir = Some(out_dir.to_str().unwrap().into());
        let t0 = Instant::now();
        c.compile(&req).unwrap();
        let mut fonts = HashSet::new();
        let mut spans = HashSet::new();
        let mut pages = vec![];
        let mut first_page = None;
        let done = loop {
            match c.next_event().unwrap().expect("host closed the connection") {
                Event::Font(f) => {
                    assert!(!f.program.is_empty(), "font {} without its program", f.id);
                    assert_eq!(f.info.str_field("format"), Some("type1"));
                    fonts.insert(f.id);
                }
                Event::Sources(s) => spans.extend(s.spans.iter().map(|x| x.0)),
                Event::Page(p) => {
                    first_page.get_or_insert(t0.elapsed());
                    for it in &p.items {
                        match it {
                            Item::Glyph { font, .. } => {
                                assert!(fonts.contains(font), "font {font} not sent")
                            }
                            Item::Span(s) if *s != 0 => {
                                assert!(spans.contains(s), "span {s} not sent")
                            }
                            _ => {}
                        }
                    }
                    assert_eq!(p.index as usize, pages.len());
                    pages.push(p);
                }
                Event::Done(d) => break d,
                Event::Error(e) => panic!("host error: {}", e),
                _ => {}
            }
        };
        let total = t0.elapsed();
        assert_eq!(done.str_field("status"), Some("ok"), "{}", done);
        assert_eq!(done.int_field("pages"), Some(pages.len() as i64));
        // As many pages as the PDF has.
        let log = std::fs::read_to_string(out_dir.join("main.log")).unwrap();
        let n: usize = log
            .split("Output written on ")
            .nth(1)
            .and_then(|t| t.split('(').nth(1))
            .and_then(|t| t.split(' ').next())
            .and_then(|t| t.parse().ok())
            .unwrap();
        assert_eq!(pages.len(), n);
        timings.push((first_page.unwrap(), total));
        last = Some(pages);
    }
    let pages = last.unwrap();
    // hyperref-toc: the table of contents links to its sections, which are
    // named destinations on later pages; body text has source spans.
    let links: Vec<_> = pages.iter().flat_map(|p| p.links.iter()).collect();
    let dests: HashMap<Vec<u8>, u32> = pages
        .iter()
        .flat_map(|p| p.dests.iter().map(move |d| (d.name.clone(), p.index)))
        .collect();
    assert!(
        links.iter().any(|l| l.kind == LinkKind::GotoName),
        "no named links"
    );
    for l in links.iter().filter(|l| l.kind == LinkKind::GotoName) {
        assert!(
            dests.contains_key(&l.data),
            "link to {} has no destination",
            String::from_utf8_lossy(&l.data)
        );
    }
    let glyphs_with_span = pages
        .iter()
        .map(|p| {
            let mut span = 0;
            p.items
                .iter()
                .filter(|it| {
                    if let Item::Span(s) = it {
                        span = *s;
                    }
                    matches!(it, Item::Glyph { .. }) && span != 0
                })
                .count()
        })
        .sum::<usize>();
    assert!(
        glyphs_with_span > 1000,
        "only {glyphs_with_span} glyphs have a source span"
    );
    for (i, (first, total)) in timings.iter().enumerate() {
        eprintln!(
            "compile {}: first page after {:.1} ms, DONE after {:.1} ms",
            i + 1,
            first.as_secs_f64() * 1e3,
            total.as_secs_f64() * 1e3
        );
    }

    // Cancel: DONE says so, and the connection stays usable.
    let mut req = CompileRequest::new(9, proj.to_str().unwrap(), "main.tex");
    req.output_dir = Some(out_dir.to_str().unwrap().into());
    c.compile(&req).unwrap();
    c.cancel(9).unwrap();
    let done = loop {
        if let Event::Done(d) = c.next_event().unwrap().unwrap() {
            break d;
        }
    };
    assert_eq!(done.int_field("id"), Some(9));
    assert!(
        matches!(done.str_field("status"), Some("cancelled") | Some("ok")),
        "{}",
        done.to_string()
    );
    c.bye().unwrap();
    let _ = std::fs::remove_dir_all(&base);
}
