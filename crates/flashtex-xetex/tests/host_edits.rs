//! Edit sequences against the Unicode host (lane AUDIT-UHOST): after every
//! compile of a sequence of edits, the resident host (runs in its own
//! process, from S₀, a page checkpoint or a converged run) writes what
//! compiles in processes of their own write (`FLASHTEX_UNICODE_INPROCESS=0`,
//! no spares, a fresh font-name cache for every compile: the oracle) after
//! the same edits: the PDF, the log and the `.aux` byte for byte, the
//! display lists of the pages and the fonts sent. Each case aims at a way
//! the restart point could be wrong: edits in `\input` files, fonts changed
//! in the preamble or on disk, XeTeX's inter-character tokens and
//! inter-word shaping toggled across the restart point, right-to-left text,
//! polyglossia's languages, images replaced in place, an undo back to the
//! first text, cancelled compiles and a document reopened from its
//! persisted S₀. Needs TeX Live; skipped without.

use flashtex_display_list::client::{Client, CompileRequest, Event};
use flashtex_display_list::json::Json;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

fn texlive() -> bool {
    flashtex_engine::resolver::discover_texlive().is_some()
}

struct Host {
    child: Child,
    sock: PathBuf,
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.sock);
    }
}

fn start_with(dir: &Path, envs: &[(String, String)], err: &Path) -> Host {
    let sock = dir.join("h.sock");
    let _ = std::fs::remove_file(&sock);
    let child = Command::new(env!("CARGO_BIN_EXE_flashtex-host-unicode"))
        .args([
            "--socket",
            sock.to_str().unwrap(),
            "--once",
            "--accept-timeout",
            "600",
        ])
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .envs(envs.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdout(Stdio::null())
        .stderr(
            std::fs::File::options()
                .create(true)
                .append(true)
                .open(err)
                .unwrap(),
        )
        .spawn()
        .unwrap();
    let t = Instant::now();
    while !sock.exists() {
        assert!(
            t.elapsed() < Duration::from_secs(900),
            "the host did not listen"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    Host { child, sock }
}

/// Ends a host the way the app does (BYE) and waits for it to exit (it
/// writes its persisted S₀ on the way out).
fn end(mut c: Client, mut h: Host) {
    c.bye().unwrap();
    let t = Instant::now();
    while h.child.try_wait().unwrap().is_none() {
        assert!(
            t.elapsed() < Duration::from_secs(120),
            "the host did not end"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The events of compile `id` up to its `DONE`.
fn events(c: &mut Client, id: i64) -> (Vec<Event>, Json) {
    let mut ev = vec![];
    loop {
        let e = c.next_event().unwrap().expect("the host closed");
        if let Event::Done(j) = &e {
            if j.int_field("id") == Some(id) {
                return (ev, j.clone());
            }
        }
        ev.push(e);
    }
}

/// A change to the project (or to the font directory, `@fonts/…`).
#[derive(Clone)]
enum Op {
    Write(String, Vec<u8>),
    /// Written in place, its old modification time put back (a copy with
    /// `cp -p`, an archive unpacked over it).
    WriteKeepTime(String, Vec<u8>),
    Remove(String),
}

fn w(name: &str, text: impl AsRef<[u8]>) -> Op {
    Op::Write(name.into(), text.as_ref().to_vec())
}

#[derive(Clone, Default)]
struct Step {
    ops: Vec<Op>,
    /// Cancel the compile this long after it is sent.
    cancel_ms: Option<u64>,
    /// End the host before this compile and start a new one (the document
    /// closed and reopened; its persisted S₀ is kept).
    reopen: bool,
    /// Changes saved this long after the compile is sent, while it runs
    /// (the oracle makes them after its compile); that compile is not
    /// compared (what it read depends on when), the next ones are.
    during: Option<(u64, Vec<Op>)>,
}

fn step(ops: Vec<Op>) -> Step {
    Step {
        ops,
        ..Step::default()
    }
}

#[derive(Debug, PartialEq)]
struct Out {
    status: String,
    pdf: Vec<u8>,
    log: String,
    aux: String,
    pages: Vec<Vec<u8>>,
    fonts: Vec<String>,
    /// The `IMAGE` messages, in order.
    images: Vec<String>,
}

/// The log without what P-T1 normalises as accounting (DESIGN.md §1.1):
/// the end-of-run memory block, whose high-water marks (`stack
/// positions`) a converged run takes from the old run, and the output's
/// byte count. (`check` reports an accounting-only difference without
/// failing, as the harnesses' accounting check does.)
fn typesetting(log: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for l in log.lines() {
        if l.starts_with("Here is how much of TeX's memory you used:") {
            skipping = true;
            continue;
        }
        if skipping {
            if l.starts_with(' ') {
                continue;
            }
            skipping = false;
        }
        if let Some(rest) = l.strip_prefix("Output written on ") {
            if let Some(k) = rest.rfind(", ") {
                out.push_str(&rest[..k]);
                out.push('\n');
                continue;
            }
        }
        out.push_str(l);
        out.push('\n');
    }
    out
}

/// The modification time every file written by step `i` gets: the same in
/// both runs of a case (the display list's image keys hold it).
fn stamp(i: usize) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000 + 60 * i as u64)
}

fn apply(proj: &Path, fonts: &Path, i: usize, ops: &[Op]) {
    let at = |n: &str| match n.strip_prefix("@fonts/") {
        Some(f) => fonts.join(f),
        None => proj.join(n),
    };
    for op in ops {
        match op {
            Op::Write(n, b) => {
                let p = at(n);
                std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                std::fs::write(&p, b).unwrap();
                std::fs::File::options()
                    .write(true)
                    .open(&p)
                    .unwrap()
                    .set_modified(stamp(i))
                    .unwrap();
            }
            Op::WriteKeepTime(n, b) => {
                let p = at(n);
                let t = std::fs::metadata(&p).unwrap().modified().unwrap();
                std::fs::write(&p, b).unwrap();
                std::fs::File::options()
                    .write(true)
                    .open(&p)
                    .unwrap()
                    .set_modified(t)
                    .unwrap();
            }
            Op::Remove(n) => {
                let _ = std::fs::remove_file(at(n));
            }
        }
    }
}

/// What the run said each compile started from, joined.
fn starts(done: &Json) -> String {
    done.get("starts")
        .and_then(Json::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default()
}

/// Runs `steps` in `dir/run` (the same paths both times: the log names
/// them). `resident`: one host for the sequence (a new one at a reopen),
/// with an S₀ cache; else the oracle, a host per compile, each with a
/// font-name cache of its own, compiles in processes of their own. Returns
/// each compile's outputs, `None` for one that was cancelled, and what each
/// started from.
fn run(
    dir: &Path,
    resident: bool,
    steps: &[Step],
    skip: &[bool],
) -> (Vec<Option<Out>>, Vec<String>) {
    let sub = dir.join("run");
    let _ = std::fs::remove_dir_all(&sub);
    let proj = sub.join("proj");
    let fonts = sub.join("fonts");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&fonts).unwrap();
    let out = sub.join("out");
    let err = sub.join("host.err");
    let envs = |i: usize| -> Vec<(String, String)> {
        let mut e = vec![
            ("FLASHTEX_PIN_CLOCK".to_string(), "1760000000.5".to_string()),
            (
                "FLASHTEX_FONT_DIRS".to_string(),
                fonts.to_string_lossy().into_owned(),
            ),
        ];
        if resident {
            e.push((
                "FLASHTEX_S0_CACHE".into(),
                sub.join("s0").to_string_lossy().into_owned(),
            ));
            e.push((
                "FLASHTEX_CACHE_DIR".into(),
                sub.join("cache").to_string_lossy().into_owned(),
            ));
        } else {
            e.push(("FLASHTEX_UNICODE_INPROCESS".into(), "0".into()));
            e.push(("FLASHTEX_UNICODE_SPARES".into(), "0".into()));
            e.push((
                "FLASHTEX_CACHE_DIR".into(),
                sub.join(format!("cache{i}")).to_string_lossy().into_owned(),
            ));
        }
        e
    };
    let mut host: Option<(Client, Host)> = None;
    let mut outs = vec![];
    let mut said = vec![];
    for (i, s) in steps.iter().enumerate() {
        apply(&proj, &fonts, i, &s.ops);
        if skip[i] {
            outs.push(None);
            said.push("skipped".into());
            continue;
        }
        if s.reopen || !resident {
            if let Some((c, h)) = host.take() {
                end(c, h);
            }
        }
        if host.is_none() {
            let h = start_with(&sub, &envs(i), &err);
            let c = Client::connect(&h.sock).unwrap();
            host = Some((c, h));
        }
        let (c, _) = host.as_mut().unwrap();
        let id = i as i64 + 1;
        let mut r = CompileRequest::new(id, proj.to_str().unwrap(), "main.tex");
        r.output_dir = Some(out.to_string_lossy().into_owned());
        c.compile(&r).unwrap();
        if let Some(ms) = s.cancel_ms.filter(|_| resident) {
            std::thread::sleep(Duration::from_millis(ms));
            c.cancel(id).unwrap();
        }
        if let Some((ms, ops)) = s.during.as_ref().filter(|_| resident) {
            std::thread::sleep(Duration::from_millis(*ms));
            apply(&proj, &fonts, 1000 + i, ops);
        }
        let (ev, done) = events(c, id);
        if let Some((_, ops)) = s.during.as_ref().filter(|_| !resident) {
            apply(&proj, &fonts, 1000 + i, ops);
        }
        let status = done.str_field("status").unwrap_or("").to_string();
        said.push(format!("{status}:{}", starts(&done)));
        if status == "cancelled" || s.during.is_some() {
            outs.push(None);
            continue;
        }
        let pages = ev
            .iter()
            .filter_map(|e| match e {
                Event::Page(p) => Some(p.encode()),
                _ => None,
            })
            .collect();
        let fonts = ev
            .iter()
            .filter_map(|e| match e {
                Event::Font(f) => Some(f.info.str_field("ps_name").unwrap_or("").to_string()),
                _ => None,
            })
            .collect();
        let images = ev
            .iter()
            .filter_map(|e| match e {
                Event::Image(j) => Some(format!("{j:?}")),
                _ => None,
            })
            .collect();
        outs.push(Some(Out {
            status,
            pdf: std::fs::read(out.join("main.pdf")).unwrap_or_default(),
            log: std::fs::read_to_string(out.join("main.log")).unwrap_or_default(),
            aux: std::fs::read_to_string(out.join("main.aux")).unwrap_or_default(),
            pages,
            fonts,
            images,
        }));
    }
    if let Some((c, h)) = host.take() {
        end(c, h);
    }
    (outs, said)
}

/// Runs `steps` resident and as the oracle and compares every compile.
/// A cancelled compile is left out of the oracle's sequence (a cancelled
/// compile leaves nothing behind). On a difference the outputs are kept
/// under `dir/diff/`.
fn check(name: &str, steps: &[Step]) {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-edits-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (got, said) = run(&dir, true, steps, &vec![false; steps.len()]);
    let skip: Vec<bool> = got
        .iter()
        .zip(steps)
        .map(|(g, s)| g.is_none() && s.during.is_none())
        .collect();
    let (want, _) = run(&dir, false, steps, &skip);
    let mut bad = vec![];
    let keep = dir.join("diff");
    for (i, (a, b)) in got.iter().zip(&want).enumerate() {
        let (Some(a), Some(b)) = (a, b) else { continue };
        let mut what = vec![];
        if a.status != b.status {
            what.push(format!("status {} / {}", a.status, b.status));
        }
        if a.pdf != b.pdf {
            what.push("PDF".into());
        }
        if typesetting(&a.log) != typesetting(&b.log) {
            what.push("log".into());
        } else if a.log != b.log {
            eprintln!(
                "{name}: compile {}: the log's accounting differs (not gating, DESIGN.md §1.1)",
                i + 1
            );
        }
        if a.images != b.images {
            what.push("images".into());
        }
        if a.aux != b.aux {
            what.push(".aux".into());
        }
        if a.pages != b.pages {
            let n = a
                .pages
                .iter()
                .zip(&b.pages)
                .position(|(x, y)| x != y)
                .unwrap_or(a.pages.len().min(b.pages.len()));
            what.push(format!(
                "pages ({} / {}, first differing {})",
                a.pages.len(),
                b.pages.len(),
                n + 1
            ));
        }
        if a.fonts != b.fonts {
            what.push(format!("fonts {:?} / {:?}", a.fonts, b.fonts));
        }
        if !what.is_empty() {
            std::fs::create_dir_all(&keep).unwrap();
            for (tag, o) in [("got", a), ("want", b)] {
                std::fs::write(keep.join(format!("{}-{tag}.pdf", i + 1)), &o.pdf).unwrap();
                std::fs::write(keep.join(format!("{}-{tag}.log", i + 1)), &o.log).unwrap();
                std::fs::write(keep.join(format!("{}-{tag}.aux", i + 1)), &o.aux).unwrap();
                std::fs::write(
                    keep.join(format!("{}-{tag}.images", i + 1)),
                    o.images.join("\n"),
                )
                .unwrap();
                for (k, p) in o.pages.iter().enumerate() {
                    std::fs::write(keep.join(format!("{}-{tag}-page{}.dl", i + 1, k + 1)), p)
                        .unwrap();
                }
            }
            bad.push(format!(
                "compile {} ({}): {}",
                i + 1,
                said[i],
                what.join(", ")
            ));
        }
    }
    // the final state against xelatex, when the run names one: the text of
    // the pages (`pdftotext`), a belief about the engine, not the host, so
    // reported and not failed
    if let (Ok(xl), Some(Some(last))) = (std::env::var("AUDIT_XELATEX"), want.last()) {
        eprintln!("{name}: xelatex: {}", against_xelatex(&dir, &xl, last));
    }
    assert!(
        bad.is_empty(),
        "{name}: the resident host differs from runs of their own\n  {}\n(started: {said:?}; kept in {})",
        bad.join("\n  "),
        keep.display()
    );
    eprintln!("{name}: {said:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// xelatex run on the project as the last step left it (to a fixed point
/// of its `.aux`, at most four runs), its pages' text against ours.
fn against_xelatex(dir: &Path, xelatex: &str, ours: &Out) -> String {
    let src = dir.join("run");
    let xl = dir.join("xl");
    let _ = std::fs::remove_dir_all(&xl);
    let cp = Command::new("cp")
        .arg("-R")
        .arg(src.join("proj"))
        .arg(&xl)
        .status();
    if !cp.is_ok_and(|s| s.success()) {
        return "could not copy the project".into();
    }
    let mut aux = None;
    for _ in 0..4 {
        let _ = Command::new(xelatex)
            .args(["-interaction=nonstopmode", "-halt-on-error", "main.tex"])
            .current_dir(&xl)
            .env("OSFONTDIR", src.join("fonts"))
            .env("SOURCE_DATE_EPOCH", "0")
            .env("FORCE_SOURCE_DATE", "1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let a = std::fs::read(xl.join("main.aux")).ok();
        if a == aux {
            break;
        }
        aux = a;
    }
    let text = |pdf: &Path| -> Option<String> {
        let o = Command::new("pdftotext").arg(pdf).arg("-").output().ok()?;
        o.status
            .success()
            .then(|| String::from_utf8_lossy(&o.stdout).into_owned())
    };
    let mine = dir.join("ours.pdf");
    std::fs::write(&mine, &ours.pdf).unwrap();
    match (text(&mine), text(&xl.join("main.pdf"))) {
        (Some(a), Some(b)) if a == b => format!("same text ({} bytes)", a.len()),
        (Some(a), Some(b)) => {
            let (la, lb): (Vec<_>, Vec<_>) = (a.lines().collect(), b.lines().collect());
            let k = la
                .iter()
                .zip(&lb)
                .position(|(x, y)| x != y)
                .unwrap_or(la.len().min(lb.len()));
            format!(
                "XELATEX-DIFF at line {}: ours {:?} / xelatex {:?} ({} / {} lines)",
                k + 1,
                la.get(k),
                lb.get(k),
                la.len(),
                lb.len()
            )
        }
        (_, None) => "XELATEX-DIFF: xelatex made no PDF".into(),
        (None, _) => "XELATEX-DIFF: no text from our PDF".into(),
    }
}

const PARA: &str = "Words and more words that fill the page, again and again, until the \
                    paragraph is long enough to break into several lines of text. ";

/// `n` paragraphs of filler, each three times [`PARA`], tagged.
fn filler(tag: &str, n: usize) -> String {
    let mut s = String::new();
    for k in 0..n {
        s.push_str(&format!("{tag} {k}. "));
        s.push_str(&PARA.repeat(3));
        s.push_str("\n\n");
    }
    s
}

fn tl_file(n: &str) -> Vec<u8> {
    let f = flashtex_engine::system::find_file(n, flashtex_engine::resolver::Format::Tex)
        .or_else(|| flashtex_engine::system::find_file(n, flashtex_engine::resolver::Format::Pict))
        .or_else(|| {
            flashtex_engine::system::find_file(n, flashtex_engine::resolver::Format::OpenType)
        })
        .or_else(|| {
            flashtex_engine::system::find_file(n, flashtex_engine::resolver::Format::TrueType)
        })
        .unwrap_or_else(|| panic!("{n} (TeX Live)"));
    std::fs::read(f).unwrap()
}

fn bundled_font(n: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/mac/Fonts")
            .join(n),
    )
    .unwrap()
}

/// `font` with `from` in its name table replaced by `to` (as long): the same
/// glyphs, the same size, another name.
fn renamed_font(font: &[u8], from: &str, to: &str) -> Vec<u8> {
    assert_eq!(from.len(), to.len());
    let mut d = font.to_vec();
    let u16_at = |d: &[u8], i: usize| u16::from_be_bytes([d[i], d[i + 1]]) as usize;
    let u32_at =
        |d: &[u8], i: usize| u32::from_be_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]) as usize;
    let (off, len) = (0..u16_at(&d, 4))
        .map(|k| 12 + 16 * k)
        .find(|&r| &d[r..r + 4] == b"name")
        .map(|r| (u32_at(&d, r + 8), u32_at(&d, r + 12)))
        .expect("a name table");
    let utf16 = |s: &str| -> Vec<u8> { s.encode_utf16().flat_map(u16::to_be_bytes).collect() };
    let mut n = 0;
    for (a, b) in [
        (from.as_bytes().to_vec(), to.as_bytes().to_vec()),
        (utf16(from), utf16(to)),
    ] {
        let mut i = off;
        while i + a.len() <= off + len {
            if d[i..i + a.len()] == a[..] {
                d[i..i + a.len()].copy_from_slice(&b);
                n += 1;
                i += a.len();
            } else {
                i += 1;
            }
        }
    }
    assert!(n > 0, "{from} is not in the name table");
    d
}

/// Edits in `\input` files: late, early, two files at once, a preamble
/// file, a file that appears (`\InputIfFileExists`) and goes, a new
/// `\input`, and back to the first text.
#[test]
fn edits_in_input_files() {
    let main = "\\documentclass{article}\n\\usepackage{fontspec}\n\\input{macros}\n\\begin{document}\n\\input{ch1}\n\\input{ch2}\n\\InputIfFileExists{extra}{}{}\n\\input{ch3}\nSee~\\ref{s3} and \\thing.\n\\end{document}\n";
    let macros = "\\newcommand\\thing{the thing}\n";
    let ch = |k: usize| {
        format!(
            "\\section{{Chapter {k}}}\\label{{s{k}}}\n{}",
            filler(&format!("Ch{k}"), 10)
        )
    };
    let (c1, c2, c3) = (ch(1), ch(2), ch(3));
    let steps = vec![
        step(vec![
            w("main.tex", main),
            w("macros.tex", macros),
            w("ch1.tex", &c1),
            w("ch2.tex", &c2),
            w("ch3.tex", &c3),
        ]),
        step(vec![w("ch3.tex", format!("{c3}A late edit.\n"))]),
        step(vec![w(
            "ch1.tex",
            c1.replacen("Words and more", "Wards and more", 1),
        )]),
        step(vec![
            w("ch1.tex", c1.replacen("Ch1 3.", "Ch1 3, edited,", 1)),
            w("ch3.tex", c3.replacen("Ch3 8.", "Ch3 8, edited too,", 1)),
        ]),
        step(vec![w(
            "macros.tex",
            "\\newcommand\\thing{another thing, longer, so that the last line changes}\n",
        )]),
        step(vec![w(
            "extra.tex",
            format!("\\section{{Extra}}\\label{{x}}\n{}", filler("Extra", 3)),
        )]),
        step(vec![w(
            "extra.tex",
            format!("\\section{{Extra}}\\label{{x}}\n{}", filler("Extra", 5)),
        )]),
        step(vec![Op::Remove("extra.tex".into())]),
        step(vec![
            w(
                "ch2b.tex",
                "\\subsection{Two b}\\label{s2b}A new file, see~\\ref{s1}.\n",
            ),
            w("ch2.tex", format!("{c2}\\input{{ch2b}}\n")),
        ]),
        step(vec![w(
            "ch2b.tex",
            "\\subsection{Two b}\\label{s2b}A new file, edited, see~\\ref{s3}.\n",
        )]),
        step(vec![
            w("main.tex", main),
            w("macros.tex", macros),
            w("ch1.tex", &c1),
            w("ch2.tex", &c2),
            w("ch3.tex", &c3),
            Op::Remove("ch2b.tex".into()),
        ]),
        step(vec![]),
    ];
    check("input", &steps);
}

/// fontspec: the main font changed in the preamble (by file, by name, and
/// back), and a project font used in the body replaced on disk: by another
/// font, by a font of the same size with its old modification time, and
/// back.
#[test]
fn fontspec_font_changes() {
    let doc = |mainfont: &str| {
        format!(
            "\\documentclass{{article}}\n\\usepackage{{fontspec}}\n\\setmainfont{{{mainfont}}}\n\\newfontfamily\\myf{{myfont.otf}}\n\\begin{{document}}\n\\section{{One}}\\label{{one}}\n{}\\section{{Two}}\n{}{{\\myf Our own font on page three, ffi.}}\n{}\\section{{Three}}\n{}{{\\fontspec{{TeX Gyre Heros}}A lookup by name in the body.}}\n{}See~\\ref{{one}}.\n\\end{{document}}\n",
            filler("A", 8),
            filler("B", 8),
            filler("C", 6),
            filler("D", 6),
            filler("E", 4)
        )
    };
    let pagella = doc("texgyrepagella-regular.otf");
    let cursor = bundled_font("texgyrecursor-regular.otf");
    let steps = vec![
        step(vec![
            w("main.tex", &pagella),
            w("myfont.otf", tl_file("lmroman10-regular.otf")),
        ]),
        step(vec![w("main.tex", doc("texgyretermes-regular.otf"))]),
        step(vec![w("main.tex", doc("TeX Gyre Heros"))]),
        step(vec![w("main.tex", &pagella)]),
        step(vec![w("myfont.otf", tl_file("lmroman12-regular.otf"))]),
        step(vec![w(
            "main.tex",
            pagella.replacen("E 2.", "E 2, edited late,", 1),
        )]),
        step(vec![w(
            "myfont.otf",
            renamed_font(&cursor, "Cursor", "Prob3x"),
        )]),
        step(vec![Op::WriteKeepTime(
            "myfont.otf".into(),
            renamed_font(&cursor, "Cursor", "Prob3y"),
        )]),
        step(vec![w(
            "main.tex",
            pagella.replacen("D 1.", "D 1, edited,", 1),
        )]),
        step(vec![
            w("main.tex", &pagella),
            w("myfont.otf", tl_file("lmroman10-regular.otf")),
        ]),
    ];
    check("fontspec", &steps);
}

/// `\XeTeXinterchartokenstate`, `\XeTeXinterchartoks` and
/// `\XeTeXinterwordspaceshaping` set in the body, moved before and after
/// the restart point, and taken out.
#[test]
fn interchar_and_interword_shaping_toggled() {
    let pre = "\\documentclass{article}\n\\usepackage{fontspec}\n\\setmainfont{texgyrepagella-regular.otf}\n\\XeTeXinterchartokenstate=1\n\\newXeTeXintercharclass\\bang\n\\XeTeXcharclass`\\!=\\bang\n\\XeTeXinterchartoks 0 \\bang = {\\kern3pt}\n\\XeTeXinterchartoks \\bang 0 = {\\kern1pt\\relax}\n\\begin{document}\n";
    let sec = |k: usize, extra: &str| {
        format!(
            "\\section{{Part {k}}}\n{extra}Hello! World! Bang!bang! {}",
            filler(&format!("P{k}! ok"), 6)
        )
    };
    let doc = |extras: &[(usize, &str)]| {
        let mut s = pre.to_string();
        for k in 1..=6 {
            let e: String = extras
                .iter()
                .filter(|(j, _)| *j == k)
                .map(|(_, t)| *t)
                .collect();
            s.push_str(&sec(k, e.as_str()));
        }
        s.push_str("The end!\n\\end{document}\n");
        s
    };
    let off = "\\XeTeXinterchartokenstate=0 ";
    let shaping = "\\XeTeXinterwordspaceshaping=2 ";
    let base = doc(&[]);
    let steps = vec![
        step(vec![w("main.tex", &base)]),
        step(vec![w("main.tex", doc(&[(4, off)]))]),
        step(vec![w(
            "main.tex",
            doc(&[(4, off)]).replacen("P1! ok 2.", "P1! ok 2, edited!", 1),
        )]),
        step(vec![w("main.tex", doc(&[(2, off)]))]),
        step(vec![w("main.tex", doc(&[(3, shaping)]))]),
        step(vec![w(
            "main.tex",
            doc(&[(3, shaping)]).replacen("P1! ok 1.", "P1! ok 1, edited!", 1),
        )]),
        step(vec![w(
            "main.tex",
            doc(&[
                (3, shaping),
                (5, "\\XeTeXinterchartoks 0 \\bang = {\\kern6pt}"),
            ]),
        )]),
        step(vec![w(
            "main.tex",
            doc(&[(5, "\\XeTeXinterchartoks 0 \\bang = {\\kern6pt}")]).replacen(
                "P6! ok 3.",
                "P6! ok 3, edited!",
                1,
            ),
        )]),
        step(vec![w("main.tex", &base)]),
    ];
    check("interchar", &steps);
}

/// Right-to-left text (#1777, #2160): Arabic and Hebrew with numbers,
/// edited inside the RTL runs, `\XeTeXinterwordspaceshaping` set before
/// them, an early edit, and back.
#[test]
fn bidi_text_edits() {
    let doc = |ar: &str, he: &str, extra: &str| {
        format!(
            "\\documentclass{{article}}\n\\usepackage{{polyglossia}}\n\\setmainlanguage{{english}}\n\\setotherlanguage{{arabic}}\n\\setotherlanguage{{hebrew}}\n\\newfontfamily\\arabicfont[Script=Arabic]{{Amiri-Regular.ttf}}\n\\newfontfamily\\hebrewfont[Script=Hebrew]{{DavidCLM-Medium.otf}}\n\\begin{{document}}\n\\section{{One}}\n{}\\section{{Two}}\n{extra}\\textarabic{{{ar}}}\n\n\\begin{{hebrew}}{he}\\end{{hebrew}}\n\n{}\\section{{Three}}\n\\textarabic{{{ar}}} and \\texthebrew{{{he}}}.\n{}\\end{{document}}\n",
            filler("A", 8),
            filler("B", 6),
            filler("C", 6)
        )
    };
    let (ar, he) = ("مرحبا بالعالم 123", "שלום עולם 2024");
    let base = doc(ar, he, "");
    let steps = vec![
        step(vec![w("main.tex", &base)]),
        step(vec![w("main.tex", doc("مرحبا بالعالم الجميل 123", he, ""))]),
        step(vec![w(
            "main.tex",
            doc("مرحبا بالعالم الجميل 123", "שלום עולם 2024 (3.5%)", ""),
        )]),
        step(vec![w(
            "main.tex",
            doc(ar, he, "\\XeTeXinterwordspaceshaping=2 "),
        )]),
        step(vec![w(
            "main.tex",
            doc(ar, he, "\\XeTeXinterwordspaceshaping=2 ").replacen("A 1.", "A 1, early,", 1),
        )]),
        step(vec![w("main.tex", doc("١٢٣ مرحبا", "123 שלום", ""))]),
        step(vec![w("main.tex", &base)]),
    ];
    check("bidi", &steps);
}

/// polyglossia: the main language switched in the preamble, French (whose
/// punctuation is active) and German selected in the body before and after
/// the restart point, and back.
#[test]
fn polyglossia_language_switches() {
    let words = "Donaudampfschifffahrtsgesellschaftskapitaen internationalization \
                 incomprehensibilities Rindfleischetikettierungsueberwachung; really: yes! no? ";
    let doc = |main: &str, s2: &str, s4: &str| {
        let mut s = format!(
            "\\documentclass{{article}}\n\\usepackage{{polyglossia}}\n\\setmainlanguage{{{main}}}\n\\setotherlanguage{{german}}\n\\setotherlanguage{{french}}\n\\begin{{document}}\n"
        );
        for k in 1..=5 {
            s.push_str(&format!("\\section{{Part {k}}}\n"));
            if k == 2 {
                s.push_str(s2);
            }
            if k == 4 {
                s.push_str(s4);
            }
            for j in 0..5 {
                s.push_str(&format!("P{k} {j}. {}\n\n", words.repeat(4)));
            }
        }
        s.push_str("\\end{document}\n");
        s
    };
    let base = doc("english", "", "");
    let steps = vec![
        step(vec![w("main.tex", &base)]),
        step(vec![w("main.tex", doc("german", "", ""))]),
        step(vec![w(
            "main.tex",
            doc("english", "\\selectlanguage{french}\n", ""),
        )]),
        step(vec![w(
            "main.tex",
            doc("english", "\\selectlanguage{french}\n", "").replacen("P4 2.", "P4 2; edited:", 1),
        )]),
        step(vec![w(
            "main.tex",
            doc(
                "english",
                "\\selectlanguage{german}\n",
                "\\selectlanguage{english}\n",
            ),
        )]),
        step(vec![w(
            "main.tex",
            doc(
                "english",
                "",
                "\\textfrench{Bonjour ! Ça va ? Oui : bien ; merci.}\n",
            ),
        )]),
        step(vec![w(
            "main.tex",
            doc(
                "english",
                "",
                "\\textfrench{Bonjour ! Ça va ? Oui : bien ; merci.}\n",
            )
            .replacen("P1 0.", "P1 0; early:", 1),
        )]),
        step(vec![w("main.tex", &base)]),
    ];
    check("polyglossia", &steps);
}

/// Images replaced in place: a PNG by one of the same size and by one of
/// another shape, a PDF by another, an edit before them, and back.
#[test]
fn images_replaced_in_place() {
    let doc = format!(
        "\\documentclass{{article}}\n\\usepackage{{fontspec}}\n\\usepackage{{graphicx}}\n\\begin{{document}}\n\\section{{One}}\n{}\\section{{Two}}\n\\includegraphics[width=.4\\linewidth]{{img.png}}\n{}\\section{{Three}}\n\\includegraphics[width=.3\\linewidth]{{fig.pdf}}\n{}\\section{{Four}}\n\\includegraphics[height=3cm]{{img.png}}\n{}\\end{{document}}\n",
        filler("A", 8),
        filler("B", 6),
        filler("C", 6),
        filler("D", 3)
    );
    let steps = vec![
        step(vec![
            w("main.tex", &doc),
            w("img.png", tl_file("example-image-a.png")),
            w("fig.pdf", tl_file("example-image-a.pdf")),
        ]),
        step(vec![w("img.png", tl_file("example-image-b.png"))]),
        step(vec![w("img.png", tl_file("example-image-16x9.png"))]),
        step(vec![w("fig.pdf", tl_file("example-image-b.pdf"))]),
        step(vec![w("main.tex", doc.replacen("A 1.", "A 1, early,", 1))]),
        step(vec![Op::WriteKeepTime(
            "img.png".into(),
            tl_file("example-image-c.png"),
        )]),
        step(vec![
            w("main.tex", &doc),
            w("img.png", tl_file("example-image-a.png")),
            w("fig.pdf", tl_file("example-image-a.pdf")),
        ]),
    ];
    check("images", &steps);
}

/// Compiles cancelled mid-run, at several points, with the next edits
/// early and late, and an undo at the end.
#[test]
fn cancelled_compiles_between_edits() {
    let mut body = String::new();
    for i in 1..=30 {
        body.push_str(&format!(
            "\\section{{Part {i}}}\\label{{p{i}}}\n{}",
            filler(&format!("S{i}"), 4)
        ));
    }
    let doc = format!(
        "\\documentclass{{article}}\n\\usepackage{{fontspec}}\n\\begin{{document}}\n{body}See~\\ref{{p2}}.\n\\end{{document}}\n"
    );
    let cancel = |ops: Vec<Op>, ms: u64| Step {
        ops,
        cancel_ms: Some(ms),
        ..Step::default()
    };
    let steps = vec![
        step(vec![w("main.tex", &doc)]),
        cancel(vec![w("main.tex", doc.replace("See~", "Look at~"))], 30),
        step(vec![w(
            "main.tex",
            doc.replace("See~", "Look at~")
                .replacen("S1 1.", "S1 1, early,", 1),
        )]),
        cancel(
            vec![w("main.tex", doc.replacen("S20 1.", "S20 1, mid,", 1))],
            80,
        ),
        cancel(
            vec![w("main.tex", doc.replacen("S3 1.", "S3 1, early,", 1))],
            15,
        ),
        step(vec![w(
            "main.tex",
            doc.replacen("S25 1.", "S25 1, late,", 1)
                .replace("\\label{p2}", "\\label{p2}\\label{q}"),
        )]),
        cancel(vec![w("main.tex", &doc)], 200),
        step(vec![w("main.tex", &doc)]),
    ];
    check("cancel", &steps);
}

/// Edits saved while a compile runs (not cancelled), in the main file and
/// in an `\input` file, early and late: the next compile sees them.
#[test]
fn edits_saved_while_a_compile_runs() {
    let mut body = String::new();
    for i in 1..=20 {
        body.push_str(&format!(
            "\\section{{Part {i}}}\n{}",
            filler(&format!("S{i}"), 4)
        ));
    }
    let doc = format!(
        "\\documentclass{{article}}\n\\usepackage{{fontspec}}\n\\begin{{document}}\n{body}\\input{{tail}}\n\\end{{document}}\n"
    );
    let tail = filler("T", 3);
    let during = |ops: Vec<Op>, ms: u64, saved: Vec<Op>| Step {
        ops,
        during: Some((ms, saved)),
        ..Step::default()
    };
    let steps = vec![
        step(vec![w("main.tex", &doc), w("tail.tex", &tail)]),
        during(
            vec![w("main.tex", doc.replacen("S15 1.", "S15 1, late,", 1))],
            30,
            vec![w(
                "main.tex",
                doc.replacen("S2 1.", "S2 1, early,", 1)
                    .replacen("S15 1.", "S15 1, late,", 1),
            )],
        ),
        step(vec![]),
        during(
            vec![w("main.tex", doc.replacen("S3 1.", "S3 1, again,", 1))],
            60,
            vec![w("tail.tex", format!("{tail}Saved while it ran.\n"))],
        ),
        step(vec![]),
        during(vec![w("main.tex", &doc)], 10, vec![w("tail.tex", &tail)]),
        step(vec![]),
    ];
    check("during", &steps);
}

/// The main font looked up by name, `TeX Gyre Prob3x`, as the font
/// directories change: its file rewritten in place with other glyphs (the
/// same family), the font removed, installed under another file name, and
/// put back; with body edits between. `reopen`: the document is closed and
/// reopened (from its persisted S₀, the app's `s0-unicode` cache, #2107)
/// before every compile after the first.
fn main_font_changes(name: &str, reopen: bool) {
    let doc = format!(
        "\\documentclass{{article}}\n\\usepackage{{fontspec}}\n\\setmainfont{{TeX Gyre Prob3x}}\n\\begin{{document}}\n\\section{{One}}\n{}See this.\n\\end{{document}}\n",
        filler("A", 8)
    );
    let x = renamed_font(
        &bundled_font("texgyrecursor-regular.otf"),
        "Cursor",
        "Prob3x",
    );
    let t = renamed_font(
        &bundled_font("texgyretermes-regular.otf"),
        "Termes",
        "Prob3x",
    );
    let s = |ops: Vec<Op>| Step {
        ops,
        reopen,
        ..Step::default()
    };
    let steps = vec![
        step(vec![w("main.tex", &doc), w("@fonts/prob3x.otf", &x)]),
        s(vec![w("main.tex", doc.replace("See this", "Look at this"))]),
        s(vec![w("@fonts/prob3x.otf", &t)]),
        s(vec![w("main.tex", doc.replace("See this", "See that"))]),
        s(vec![Op::Remove("@fonts/prob3x.otf".into())]),
        s(vec![w("main.tex", &doc)]),
        s(vec![w("@fonts/other-name.otf", &x)]),
        s(vec![w("main.tex", doc.replace("See this", "See it"))]),
        s(vec![
            Op::Remove("@fonts/other-name.otf".into()),
            w("@fonts/prob3x.otf", &x),
        ]),
        s(vec![w("main.tex", &doc)]),
    ];
    check(name, &steps);
}

#[test]
fn a_reopened_document_after_its_fonts_changed() {
    main_font_changes("reopen-fonts", true);
}

#[test]
fn the_main_font_changes_on_disk_while_open() {
    main_font_changes("open-fonts", false);
}

/// A font file rewritten in place (`cp` over it) with a font of another
/// family: the directory's listing is unchanged, and the resident host's
/// catalog and a persisted S₀'s key (both kept while the font directories
/// are) still answer as before the change, where a fresh run reads the
/// file's names again. Ignored: a known gap (#2296); fontconfig's cache, which
/// xelatex reads on Linux, is kept by directory too.
#[test]
#[ignore]
fn a_font_file_rewritten_in_place_with_another_family() {
    let doc = format!(
        "\\documentclass{{article}}\n\\usepackage{{fontspec}}\n\\setmainfont{{TeX Gyre Prob3x}}\n\\begin{{document}}\n\\section{{One}}\n{}See this.\n\\end{{document}}\n",
        filler("A", 8)
    );
    let x = renamed_font(
        &bundled_font("texgyrecursor-regular.otf"),
        "Cursor",
        "Prob3x",
    );
    let y = renamed_font(
        &bundled_font("texgyrecursor-regular.otf"),
        "Cursor",
        "Prob3y",
    );
    for reopen in [false, true] {
        let s = |ops: Vec<Op>| Step {
            ops,
            reopen,
            ..Step::default()
        };
        let steps = vec![
            step(vec![w("main.tex", &doc), w("@fonts/prob3x.otf", &x)]),
            s(vec![w("@fonts/prob3x.otf", &y)]),
            s(vec![w("main.tex", doc.replace("See this", "See that"))]),
            s(vec![w("@fonts/prob3x.otf", &x)]),
            s(vec![w("main.tex", &doc)]),
        ];
        check(
            if reopen {
                "inplace-reopen"
            } else {
                "inplace-open"
            },
            &steps,
        );
    }
}

/// A font looked up by name in the body (after S₀, on page three) whose
/// file is rewritten in place with other glyphs under the same family
/// name, with edits after it: the page checkpoints after the lookup must
/// not stand.
#[test]
fn a_body_font_found_by_name_rewritten_in_place() {
    let doc = format!(
        "\\documentclass{{article}}\n\\usepackage{{fontspec}}\n\\begin{{document}}\n\\section{{One}}\n{}\\section{{Two}}\n{}{{\\fontspec{{TeX Gyre Prob3x}}Found by name on page three.}}\n{}\\section{{Three}}\n{}See this.\n\\end{{document}}\n",
        filler("A", 8),
        filler("B", 8),
        filler("C", 6),
        filler("D", 6)
    );
    let x = renamed_font(
        &bundled_font("texgyrecursor-regular.otf"),
        "Cursor",
        "Prob3x",
    );
    let t = renamed_font(
        &bundled_font("texgyretermes-regular.otf"),
        "Termes",
        "Prob3x",
    );
    let steps = vec![
        step(vec![w("main.tex", &doc), w("@fonts/prob3x.otf", &x)]),
        step(vec![w("main.tex", doc.replace("See this", "Look at this"))]),
        step(vec![w("@fonts/prob3x.otf", &t)]),
        step(vec![w("main.tex", doc.replace("See this", "See that"))]),
        step(vec![Op::WriteKeepTime(
            "@fonts/prob3x.otf".into(),
            x.clone(),
        )]),
        step(vec![w("main.tex", doc.replacen("D 2.", "D 2, edited,", 1))]),
        step(vec![w("main.tex", &doc)]),
    ];
    check("body-font", &steps);
}
