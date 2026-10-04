//! Typing latency and memory of `flashtex-typst-host`, end to end over its
//! socket (DESIGN.md §15.3, §15.10 T1's latency and memory gates).
//!
//! ```sh
//! # The fixed corpus (Track A's generator: A4, 11 pt, outline, bibliography,
//! # numbered equations, references, figures; c300: a #pagebreak() every 14
//! # sections).
//! typing_bench gen DIR SECTIONS [CHAPTER_EVERY]
//! # Keystrokes: a 3.3 client (opentype, font-program-refs, incremental)
//! # types N characters before each marker (EDITSTART, EDITMID, EDITEND) and
//! # waits for each compile's DONE; one JSON line per location.
//! typing_bench run --host BIN --fonts DIR --doc DIR [--keys N] [--label L]
//!                  [--locs EDITSTART,EDITMID] [--host-arg ARG]... [--rss-every K]
//!                  [--accept font-program-refs,color-spaces,line-state,image-data]
//! ```
//!
//! Per keystroke it records the time from sending `COMPILE` to the first
//! `PAGE` frame (the edited page on the socket, which is the gated
//! number), to `DONE`, and the host's own `compile_ms`, `positions_ms`,
//! `first_page_ms` and `iterations`; with `--rss-every K` the host's
//! resident memory every K keystrokes.
//!
//! The generator is Track A's (`docs/evidence/typst-design-2026-09-30/
//! prototype`), our own MIT code, unchanged in what it writes.

use std::fmt::Write as _;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use flashtex_display_list::frame::{read_frame, write_frame};
use flashtex_display_list::json::Json;
use flashtex_display_list::kind;

const WORDS: &[&str] = &[
    "the",
    "of",
    "and",
    "a",
    "to",
    "in",
    "is",
    "that",
    "for",
    "it",
    "as",
    "was",
    "with",
    "be",
    "by",
    "on",
    "not",
    "this",
    "are",
    "which",
    "from",
    "or",
    "have",
    "an",
    "they",
    "one",
    "you",
    "were",
    "all",
    "we",
    "can",
    "their",
    "has",
    "there",
    "been",
    "if",
    "more",
    "when",
    "will",
    "would",
    "who",
    "so",
    "no",
    "compact",
    "operator",
    "manifold",
    "spectral",
    "sequence",
    "theorem",
    "estimate",
    "boundary",
    "convergence",
    "lattice",
    "invariant",
    "measure",
    "functional",
    "resolvent",
    "categorical",
    "homotopy",
    "typesetting",
    "incremental",
    "paragraph",
    "hyphenation",
    "justification",
];

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn word(&mut self) -> &'static str {
        WORDS[(self.next() % WORDS.len() as u64) as usize]
    }
}

fn para(rng: &mut Rng, words: usize, out: &mut String) {
    for i in 0..words {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(rng.word());
    }
}

fn generate(dir: &Path, sections: usize, chapter_every: usize) {
    std::fs::create_dir_all(dir).unwrap();
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let nbib = (sections / 2).max(8);
    let mut bib = String::new();
    for k in 0..nbib {
        writeln!(
            bib,
            "@article{{ref{k},\n  author = {{Author{k}, A. and Writer, B.}},\n  title = {{On the {} {} of {} {}}},\n  journal = {{Journal of {}}},\n  volume = {{{}}},\n  pages = {{{}--{}}},\n  year = {{{}}}\n}}\n",
            rng.word(), rng.word(), rng.word(), rng.word(), rng.word(),
            k % 40 + 1, k * 3 + 1, k * 3 + 17, 1950 + (k % 75)
        )
        .unwrap();
    }
    std::fs::write(dir.join("refs.bib"), bib).unwrap();
    std::fs::write(
        dir.join("fig.svg"),
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100" viewBox="0 0 200 100">
<rect x="5" y="5" width="190" height="90" fill="#eef" stroke="#336" stroke-width="2"/>
<circle cx="60" cy="50" r="30" fill="#c33"/><path d="M110 80 L150 20 L190 80 Z" fill="#3a3"/>
</svg>"##,
    )
    .unwrap();
    let mut s = String::new();
    s.push_str(
        r#"#set document(title: [Bench document])
#set page(paper: "a4", numbering: "1", header: context [_Bench_ #h(1fr) #counter(page).display()])
#set heading(numbering: "1.1")
#set math.equation(numbering: "(1)")
#set par(justify: true)
#set text(size: 11pt)
#outline()
#pagebreak()
"#,
    );
    let marks = [
        (sections / 20).max(1),
        sections / 2,
        sections.saturating_sub(2).max(1),
    ];
    for i in 1..=sections {
        if chapter_every > 0 && i > 1 && (i - 1) % chapter_every == 0 {
            s.push_str("#pagebreak()\n");
        }
        writeln!(s, "= Section {i} on {} {}\n", rng.word(), rng.word()).unwrap();
        if marks.contains(&i) {
            let tag = if i == marks[0] {
                "EDITSTART"
            } else if i == marks[1] {
                "EDITMID"
            } else {
                "EDITEND"
            };
            write!(s, "{tag} ").unwrap();
        }
        para(&mut rng, 70, &mut s);
        write!(s, " @ref{} and $x_{i}^2 + y^2 = z^2$ ", i % nbib).unwrap();
        para(&mut rng, 40, &mut s);
        write!(s, "#footnote[A note on {} {}.] ", rng.word(), rng.word()).unwrap();
        para(&mut rng, 30, &mut s);
        s.push_str(".\n\n");
        writeln!(
            s,
            "$ integral_0^oo e^(-{i} t^2) dif t = sqrt(pi) / (2 sqrt({i})), quad sum_(k=1)^n k^{i} <= n^({i}+1) $ <eq{i}>\n"
        )
        .unwrap();
        write!(s, "As shown in @eq{i}, ").unwrap();
        para(&mut rng, 90, &mut s);
        s.push_str(".\n\n");
        if i % 3 == 0 {
            writeln!(
                s,
                "#figure(image(\"fig.svg\", width: 45%), caption: [Figure for section {i}: {} {}.]) <fig{i}>\n",
                rng.word(),
                rng.word()
            )
            .unwrap();
        }
        if i % 5 == 0 {
            s.push_str("#figure(table(columns: 4, stroke: 0.5pt, [*A*], [*B*], [*C*], [*D*]");
            for r in 0..4 {
                write!(
                    s,
                    ", [{r}], [{}], [{}], [$alpha_{r}$]",
                    rng.word(),
                    rng.word()
                )
                .unwrap();
            }
            writeln!(s, "), caption: [Table {i}.])\n").unwrap();
        }
        para(&mut rng, 80, &mut s);
        if i > 3 && i % 3 == 0 {
            write!(s, " (see @fig{i})").unwrap();
        }
        s.push_str(".\n\n");
    }
    s.push_str("#bibliography(\"refs.bib\")\n");
    std::fs::write(dir.join("main.typ"), s).unwrap();
}

fn pct(v: &[f64], p: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    let mut v = v.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[((p / 100.0) * (v.len() as f64 - 1.0)).round() as usize]
}

fn rss_mb(pid: u32) -> f64 {
    let out = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<f64>()
        .unwrap_or(f64::NAN)
        / 1024.0
}

fn load() -> String {
    let out = Command::new("sysctl").args(["-n", "vm.loadavg"]).output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => String::new(),
    }
}

struct Conn {
    r: BufReader<UnixStream>,
    w: BufWriter<UnixStream>,
}

impl Conn {
    fn send(&mut self, k: u8, j: &str) {
        write_frame(&mut self.w, k, j.as_bytes()).unwrap();
        self.w.flush().unwrap();
    }
    fn frame(&mut self) -> (u8, Vec<u8>) {
        read_frame(&mut self.r).unwrap().expect("host closed")
    }
}

fn json(b: &[u8]) -> Json {
    Json::parse(std::str::from_utf8(b).unwrap()).unwrap()
}

/// One compile: (ms to the first PAGE, ms to DONE, DONE).
fn compile(c: &mut Conn, req: &str) -> (Option<f64>, f64, Json) {
    let t = Instant::now();
    c.send(kind::COMPILE, req);
    let mut first = None;
    loop {
        let (k, b) = c.frame();
        if k == kind::PAGE && first.is_none() {
            first = Some(t.elapsed().as_secs_f64() * 1e3);
        }
        if k == kind::DONE {
            return (first, t.elapsed().as_secs_f64() * 1e3, json(&b));
        }
        if k == kind::ERROR {
            panic!("host error: {}", String::from_utf8_lossy(&b));
        }
    }
}

fn run(args: &[String]) {
    let mut host = None;
    let mut fonts = None;
    let mut doc = None;
    let mut keys = 40usize;
    let mut label = String::new();
    let mut locs = vec!["EDITSTART".to_string(), "EDITMID".into(), "EDITEND".into()];
    let mut host_args: Vec<String> = vec![];
    let mut rss_every = 0usize;
    // The client's HELLO `accept` (spec §11.7); a 3.3 app client also says
    // color-spaces, line-state and image-data.
    let mut accept = vec!["font-program-refs".to_string()];
    let mut i = 0;
    while i < args.len() {
        let v = args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--host" => host = Some(PathBuf::from(v)),
            "--fonts" => fonts = Some(PathBuf::from(v)),
            "--doc" => doc = Some(PathBuf::from(v)),
            "--keys" => keys = v.parse().unwrap(),
            "--label" => label = v,
            "--locs" => locs = v.split(',').map(String::from).collect(),
            "--host-arg" => host_args.push(v),
            "--rss-every" => rss_every = v.parse().unwrap(),
            "--accept" => accept = v.split(',').map(String::from).collect(),
            a => panic!("unknown argument {a}"),
        }
        i += 2;
    }
    let doc = doc.expect("--doc").canonicalize().unwrap();
    let sockdir = std::env::temp_dir().join(format!("ftb-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&sockdir);
    std::fs::create_dir_all(&sockdir).unwrap();
    let sock = sockdir.join("h.sock");
    let mut child = Command::new(host.expect("--host"))
        .arg("--socket")
        .arg(&sock)
        .arg("--font-path")
        .arg(fonts.expect("--fonts"))
        .arg("--no-system-fonts")
        .args(&host_args)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let pid = child.id();
    let mut out = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    while !line.contains("listening") {
        line.clear();
        out.read_line(&mut line).unwrap();
    }
    let s = UnixStream::connect(&sock).unwrap();
    let mut c = Conn {
        r: BufReader::new(s.try_clone().unwrap()),
        w: BufWriter::new(s),
    };
    let accept: Vec<String> = accept.iter().map(|a| format!("{a:?}")).collect();
    c.send(
        kind::C_HELLO,
        &format!(
            r#"{{"protocol":"display-list-v3","version":[3,3],"client":"typing_bench","accept":[{}]}}"#,
            accept.join(",")
        ),
    );
    let _ = c.frame();
    let mut text = std::fs::read_to_string(doc.join("main.typ")).unwrap();
    let out_dir = sockdir.join("out");
    let req = |id: i64, edits: &str| {
        format!(
            r#"{{"id":{id},"root":{:?},"main":"main.typ","output_dir":{:?},"font_formats":["opentype"],"incremental":true,"edits":[{edits}]}}"#,
            doc.to_string_lossy(),
            out_dir.to_string_lossy()
        )
    };
    let mut id = 1;
    let (f, d, done) = compile(&mut c, &req(id, ""));
    println!(
        "{{\"label\":{label:?},\"cold\":true,\"first_page_ms\":{:.2},\"done_ms\":{d:.2},\"pages\":{},\"rss_mb\":{:.1},\"load\":{:?}}}",
        f.unwrap_or(f64::NAN),
        done.int_field("pages").unwrap_or(0),
        rss_mb(pid),
        load()
    );
    let original = text.clone();
    for loc in &locs {
        let mut first = vec![];
        let mut total = vec![];
        let mut compile_ms = vec![];
        let mut pos_ms = vec![];
        let mut hash_ms = vec![];
        let mut host_first = vec![];
        let mut iters = vec![];
        let mut pages_sent = vec![];
        let mut rss = vec![];
        let mut not_ok = 0;
        let (mut checked_same, mut checked_differ, mut seeded) = (0, 0, 0);
        for k in 0..keys {
            let off = text.find(loc.as_str()).expect("marker");
            let ch = if k % 7 == 6 {
                ' '
            } else {
                (b'a' + ((k * 7 + 3) % 26) as u8) as char
            };
            text.insert(off, ch);
            id += 1;
            let edit =
                format!(r#"{{"path":"main.typ","offset":{off},"delete":0,"insert":"{ch}"}}"#);
            let (f, d, done) = compile(&mut c, &req(id, &edit));
            if done.str_field("status") != Some("ok") {
                not_ok += 1;
            }
            match done.get("verified").and_then(Json::as_bool) {
                Some(true) => checked_same += 1,
                Some(false) => checked_differ += 1,
                None => {}
            }
            if done.get("seeded").and_then(Json::as_bool) == Some(true) {
                seeded += 1;
            }
            if k >= 2 {
                if let Some(f) = f {
                    first.push(f);
                }
                total.push(d);
                let num = |k: &str| match done.get(k) {
                    Some(Json::Num(v)) => *v,
                    Some(Json::Int(v)) => *v as f64,
                    _ => f64::NAN,
                };
                compile_ms.push(num("compile_ms"));
                pos_ms.push(num("positions_ms"));
                hash_ms.push(num("hash_ms"));
                host_first.push(num("first_page_ms"));
                iters.push(num("iterations"));
                pages_sent.push(num("typeset_pages"));
            }
            if rss_every > 0 && (k + 1) % rss_every == 0 {
                rss.push(format!("[{},{:.1}]", k + 1, rss_mb(pid)));
            }
        }
        println!(
            "{{\"label\":{label:?},\"loc\":{loc:?},\"keys\":{keys},\"first_page_p50\":{:.2},\"first_page_p95\":{:.2},\"first_page_max\":{:.2},\"done_p50\":{:.2},\"done_p95\":{:.2},\"compile_p50\":{:.2},\"compile_p95\":{:.2},\"host_first_page_p95\":{:.2},\"positions_p50\":{:.3},\"positions_p95\":{:.3},\"hash_p50\":{:.3},\"iterations_max\":{},\"pages_sent_max\":{},\"not_ok\":{not_ok},\"seeded\":{seeded},\"verified_same\":{checked_same},\"verified_differ\":{checked_differ},\"rss_mb\":{:.1},\"rss\":[{}],\"load\":{:?}}}",
            pct(&first, 50.0),
            pct(&first, 95.0),
            pct(&first, 100.0),
            pct(&total, 50.0),
            pct(&total, 95.0),
            pct(&compile_ms, 50.0),
            pct(&compile_ms, 95.0),
            pct(&host_first, 95.0),
            pct(&pos_ms, 50.0),
            pct(&pos_ms, 95.0),
            pct(&hash_ms, 50.0),
            pct(&iters, 100.0),
            pct(&pages_sent, 100.0),
            rss_mb(pid),
            rss.join(","),
            load()
        );
    }
    // Restore the document for the next run.
    std::fs::write(doc.join("main.typ"), original).unwrap();
    c.send(kind::BYE, "{}");
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&sockdir);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("gen") => generate(
            Path::new(&args[1]),
            args[2].parse().unwrap(),
            args.get(3).map(|s| s.parse().unwrap()).unwrap_or(0),
        ),
        Some("run") => run(&args[1..]),
        _ => eprintln!("usage: typing_bench gen DIR SECTIONS [CHAPTER_EVERY] | run ..."),
    }
}
