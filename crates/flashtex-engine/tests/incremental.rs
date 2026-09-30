//! Incremental compiles (DESIGN.md §5.2-§5.4, `flashtex-host iserve`)
//! against from-scratch runs, on the installed TeX Live's LaTeX. Skips where
//! there is no TeX Live (e.g. CI).
//!
//! Every compile of a session is compared, byte for byte (PDF, log, aux),
//! with `flashtex-initex` run once on a copy of the directory as the compile
//! found it, in preview mode like the host. The documents are the cases the
//! restart and convergence tests got wrong once:
//!
//! * `edits_equal_scratch_runs`: a file `\input` twice (a restart point in
//!   its second read must not skip an edit the first read saw), labels that
//!   move when a sentence is inserted (the next compile reads a changed
//!   `.aux`: it restarts at the `.aux` point), edits and reverts.
//! * `a_file_written_then_read_is_a_barrier`: a file written from a macro
//!   the edit changes and `\input` pages later; the run must not converge
//!   before that read (DESIGN.md §5.3's barriers).
#![cfg(feature = "kpathsea")]

use flashtex_engine::resolver::find_texlive_bin;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Env {
    dir: PathBuf,
    fmt: PathBuf,
    pool: PathBuf,
}

/// A pdflatex format made by this engine, once per test binary.
fn env() -> Option<Env> {
    static MADE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    find_texlive_bin()?;
    let _once = MADE.lock().unwrap_or_else(|p| p.into_inner());
    let initex = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    let base = std::env::temp_dir().join(format!("flashtex-incr-{}", std::process::id()));
    let fmt = base.join("fmt");
    std::fs::create_dir_all(&fmt).unwrap();
    let pdftex = fmt.join("pdftex");
    let _ = std::os::unix::fs::symlink(initex, &pdftex);
    if !fmt.join("pdflatex.fmt").exists() {
        let st = Command::new(&pdftex)
            .args([
                "-ini",
                "-jobname=pdflatex",
                "-progname=pdflatex",
                "-translate-file=cp227.tcx",
                "*pdflatex.ini",
            ])
            .current_dir(&fmt)
            .env("FLASHTEX_POOL", &pool)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(st.success() && fmt.join("pdflatex.fmt").exists());
    }
    Some(Env {
        dir: base,
        fmt,
        pool,
    })
}

const ARGS: &[&str] = &["-fmt=pdflatex", "-interaction=nonstopmode", "doc.tex"];

fn engine_env(c: &mut Command, e: &Env) {
    c.env("FLASHTEX_POOL", &e.pool)
        .env("FLASHTEX_FORMATS", &e.fmt)
        .env("SOURCE_DATE_EPOCH", "1700000000")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_PIN_CLOCK", "1700000000.123456")
        .env_remove("FLASHTEX_NO_AUX_POINT")
        .env_remove("FLASHTEX_NO_RELABEL");
}

struct Host {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Host {
    fn start(e: &Env, dir: &Path) -> Host {
        let mut c = Command::new(env!("CARGO_BIN_EXE_flashtex-host"));
        c.arg("iserve").arg("--").args(ARGS).current_dir(dir);
        engine_env(&mut c, e);
        let mut child = c
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Host {
            child,
            stdin,
            stdout,
        }
    }

    fn cmd(&mut self, c: &str) -> String {
        writeln!(self.stdin, "{c}").unwrap();
        self.stdin.flush().unwrap();
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        assert!(!line.contains("\"error\""), "{c}: {line}");
        line
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = writeln!(self.stdin, "quit");
        let _ = self.child.wait();
    }
}

fn copy_dir(from: &Path, to: &Path) {
    let _ = std::fs::remove_dir_all(to);
    std::fs::create_dir_all(to).unwrap();
    for f in std::fs::read_dir(from).unwrap() {
        let f = f.unwrap();
        if f.file_type().unwrap().is_file() {
            std::fs::copy(f.path(), to.join(f.file_name())).unwrap();
        }
    }
}

/// A log without DESIGN.md §1.1's capacity and output-size accounting (the
/// end-of-run memory block, the PDF statistics block, the byte count of
/// "Output written"), as `tools/parity`'s `split_accounting` removes it.
fn strict_log(log: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(log);
    let mut out = vec![];
    let mut in_block = false;
    for ln in text.split('\n') {
        if ln == "Here is how much of TeX's memory you used:" || ln == "PDF statistics:" {
            in_block = true;
            continue;
        }
        if in_block && ln.starts_with(' ') {
            continue;
        }
        in_block = false;
        if ln.starts_with("Output written on ") {
            if let Some(i) = ln.rfind(", ") {
                out.push(format!("{}, <BYTES> bytes).", &ln[..i]));
                continue;
            }
        }
        out.push(ln.to_string());
    }
    out.join("\n").into_bytes()
}

/// The directory's files but the PDF and the log, with their contents: what
/// the next from-scratch run may read.
fn dir_state(d: &Path) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = std::fs::read_dir(d)
        .unwrap()
        .filter_map(|f| f.ok())
        .filter(|f| f.file_type().is_ok_and(|t| t.is_file()))
        .map(|f| f.file_name().to_string_lossy().into_owned())
        .filter(|n| !n.ends_with(".pdf") && !n.ends_with(".log"))
        .map(|n| {
            let b = std::fs::read(d.join(&n)).unwrap();
            (n, b)
        })
        .collect();
    v.sort();
    v
}

/// Compile in the host after writing `files`, and compare with scratch runs
/// on the directory as the compile found it: repeated, like the host's
/// passes (DESIGN.md §5.5), while a run changes a file other than the PDF
/// and the log, up to five runs, stopping when the files repeat a state an
/// earlier run started from. Returns the host's report.
fn compile_and_check(
    e: &Env,
    h: &mut Host,
    dir: &Path,
    files: &[(&str, &str)],
    what: &str,
) -> String {
    for (name, text) in files {
        std::fs::write(dir.join(name), text).unwrap();
    }
    let reference = dir.with_extension("ref");
    copy_dir(dir, &reference);
    let report = h.cmd("compile");
    let mut seen = vec![];
    for _ in 0..5 {
        seen.push(dir_state(&reference));
        let mut c = Command::new(e.fmt.join("pdftex"));
        c.args(ARGS).current_dir(&reference);
        engine_env(&mut c, e);
        c.env("FLASHTEX_PREVIEW", "1");
        c.stdin(Stdio::null()).stdout(Stdio::null());
        c.status().unwrap();
        if seen.contains(&dir_state(&reference)) {
            break;
        }
    }
    for ext in ["pdf", "log", "aux"] {
        // (a run that fails produces no PDF: then neither may have one)
        let (x, y) = (
            std::fs::read(dir.join(format!("doc.{ext}"))).ok(),
            std::fs::read(reference.join(format!("doc.{ext}"))).ok(),
        );
        if ext == "log" && x != y {
            if let (Some(a), Some(b)) = (&x, &y) {
                if strict_log(a) == strict_log(b) {
                    // DESIGN.md §1.1 (ruling N2): the end-of-run capacity
                    // accounting is reported, not compared
                    eprintln!("{what}: the log differs in its accounting only");
                    continue;
                }
            }
        }
        assert!(
            x == y,
            "{what}: doc.{ext} differs from a scratch run ({:?} vs {:?} bytes)\n{report}",
            x.as_ref().map(|v| v.len()),
            y.as_ref().map(|v| v.len())
        );
    }
    report
}

#[test]
fn a_failed_run_then_a_revert() {
    let Some(e) = env() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let dir = e.dir.join("failed");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // an object written in the preamble opens the PDF before the .aux
    // point (beamer does that); a run that fails removes the PDF
    let body: String = (0..30).map(|i| para(i, "alpha")).collect();
    let doc = format!(
        "\\documentclass{{article}}\n\\usepackage{{hyperref}}\n\
         \\immediate\\pdfobj{{null}}\n\\begin{{document}}\n\
         \\section{{One}}\\label{{one}}\n{body}See page~\\pageref{{one}}.\n\\end{{document}}\n"
    );
    let broken = doc.replace("\\end{document}", "\\end{documen}");
    let mut h = Host::start(&e, &dir);
    for _ in 0..3 {
        compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc)], "settle");
    }
    compile_and_check(
        &e,
        &mut h,
        &dir,
        &[("doc.tex", &broken)],
        "a run that fails",
    );
    compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc)], "the revert");
    compile_and_check(&e, &mut h, &dir, &[], "settle again");
}

fn para(i: usize, word: &str) -> String {
    format!(
        "Paragraph {i} with the word {word}, and enough text to fill a few lines \
         of the page so that the document ships several pages; math $a^{i}+b$.\n\n"
    )
}

#[test]
fn edits_equal_scratch_runs() {
    let Some(e) = env() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let dir = e.dir.join("edits");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = r"\documentclass{article}
\usepackage{hyperref}
\begin{document}
\section{One}\label{one}
See page~\pageref{two}.
\input{body}
\section{Two}\label{two}
Back to page~\pageref{one}.
\input{body}
\end{document}
";
    let body: String = (0..40).map(|i| para(i, "alpha")).collect();
    let mut h = Host::start(&e, &dir);
    // settle: the .aux appears, then the labels are stable
    for k in 0..4 {
        let r = compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("doc.tex", doc), ("body.tex", &body)],
            "settle",
        );
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
    // an edit late in body.tex: the second read has page checkpoints
    // before it, which the first read's pages already passed
    let e1 = body.replacen(
        "Paragraph 33 with the word alpha",
        "Paragraph 33 with the word alphb",
        1,
    );
    assert_ne!(e1, body);
    let r = compile_and_check(
        &e,
        &mut h,
        &dir,
        &[("body.tex", &e1)],
        "edit in a file read twice",
    );
    assert!(r.contains("\"mode\":\"incremental\""), "{r}");
    // a sentence that moves the second section to another page
    let extra = "More words to move the label. ".repeat(40);
    let e2 = e1.replacen("Paragraph 3 ", &format!("{extra}Paragraph 3 "), 1);
    compile_and_check(&e, &mut h, &dir, &[("body.tex", &e2)], "reflow");
    // the next compile reads the .aux the reflow rewrote
    let r = compile_and_check(&e, &mut h, &dir, &[], "after the .aux changed");
    assert!(
        !r.contains("\"mode\":\"cold\""),
        "a changed .aux re-ran from the format: {r}"
    );
    compile_and_check(&e, &mut h, &dir, &[("body.tex", &body)], "revert");
    compile_and_check(&e, &mut h, &dir, &[], "settle again");
}

#[test]
fn a_file_written_then_read_is_a_barrier() {
    let Some(e) = env() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let dir = e.dir.join("barrier");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |word: &str| -> String {
        let mut s = String::from("\\documentclass{article}\n\\newwrite\\tmp\n\\begin{document}\n");
        for i in 0..12 {
            s.push_str(&para(i, "lorem"));
        }
        s.push_str(&format!(
            "\\def\\x{{{word}}}\\immediate\\openout\\tmp=\\jobname.tmp \
             \\immediate\\write\\tmp{{Written \\x.}}\\immediate\\closeout\\tmp \\def\\x{{}}\n\n"
        ));
        for i in 12..60 {
            s.push_str(&para(i, "ipsum"));
        }
        s.push_str("\\input{\\jobname.tmp}\n\n");
        for i in 60..70 {
            s.push_str(&para(i, "dolor"));
        }
        s.push_str("\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    for _ in 0..3 {
        compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("doc.tex", &doc("alphaword"))],
            "settle",
        );
    }
    for w in ["alphawore", "alphaword", "betaword"] {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(w))], w);
        assert!(r.contains("\"mode\":\"incremental\""), "{r}");
    }
}

/// A field of a host report (the text after `"name":` up to `,` or `]`).
fn field<'a>(report: &'a str, name: &str) -> &'a str {
    let key = format!("\"{name}\":");
    let at = report.find(&key).map_or(report.len(), |i| i + key.len());
    let rest = &report[at..];
    let end = rest.find([',', '}']).unwrap_or(rest.len());
    &rest[..end]
}

/// A document with sections, labels, forward and backward references
/// (`\ref`, `\pageref`), footnotes, a table of contents and a
/// bibliography: the edits of `structural_edits_equal_scratch_runs` change
/// the `.aux` and `.toc` in every way a pass can see.
fn refs_doc(extra: &str, sections: usize) -> String {
    let mut s = String::from(
        "\\documentclass{article}\n\\usepackage{hyperref}\n\\begin{document}\n\
         \\tableofcontents\n",
    );
    for k in 0..sections {
        s.push_str(&format!("\\section{{Part {k}}}\\label{{sec:{k}}}\n"));
        if k == 2 {
            s.push_str(extra);
        }
        for i in 0..6 {
            s.push_str(&para(k * 6 + i, "gamma"));
        }
        s.push_str(&format!(
            "See section~\\ref{{sec:{}}} on page~\\pageref{{sec:{}}} and \\cite{{key{}}}.\\footnote{{Note {k}.}}\n\n",
            (k + 3) % sections,
            (k + sections - 1) % sections,
            k % 3
        ));
    }
    s.push_str("\\begin{thebibliography}{9}\n");
    for b in 0..3 {
        s.push_str(&format!("\\bibitem{{key{b}}} Author {b}. Title {b}.\n"));
    }
    s.push_str("\\end{thebibliography}\n\\end{document}\n");
    s
}

/// DESIGN.md §5.5: edits that move labels, add and remove sections,
/// labels, citations and footnotes change the `.aux` and the `.toc`; each
/// compile, with its further passes, equals from-scratch runs repeated by
/// the same rule, and a pass whose `.aux` changed restarts at the first
/// read of a changed entry rather than at the `.aux` point.
#[test]
fn structural_edits_equal_scratch_runs() {
    let Some(e) = env() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let dir = e.dir.join("structural");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let base = refs_doc("", 8);
    let mut h = Host::start(&e, &dir);
    for _ in 0..3 {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &base)], "settle");
        if r.contains("\"mode\":\"unchanged\"") {
            break;
        }
    }
    let edits: Vec<(&str, String)> = vec![
        (
            "a sentence that moves labels",
            refs_doc(&"Words that move the labels. ".repeat(30), 8),
        ),
        (
            "a new section",
            refs_doc("\\section{Inserted}\\label{sec:new}\n", 8),
        ),
        (
            "a new label and a reference to it",
            refs_doc("\\label{lab:x}See \\pageref{lab:x}.\n", 8),
        ),
        ("a new citation", refs_doc("As \\cite{key2} says.\n", 8)),
        (
            "a new footnote",
            refs_doc("Text.\\footnote{Inserted.}\n", 8),
        ),
        ("a section fewer", refs_doc("", 7)),
    ];
    let mut l5_restarts = 0;
    let mut notes = vec![];
    for (what, doc) in &edits {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", doc)], what);
        if r.contains("restart at page") {
            l5_restarts += 1;
        }
        notes.push(r.split("\"l5\":").nth(1).unwrap_or("").to_string());
        let r = compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("doc.tex", &base)],
            &format!("{what}, reverted"),
        );
        if r.contains("restart at page") {
            l5_restarts += 1;
        }
        notes.push(r.split("\"l5\":").nth(1).unwrap_or("").to_string());
    }
    eprintln!("L5 notes: {notes:#?}");
    assert!(
        l5_restarts > 0,
        "no pass restarted at the first read of a changed .aux entry: {notes:#?}"
    );
}

/// DESIGN.md §5.3's barriers: a document that reads `\pdfelapsedtime`
/// after the last page's text never converges (the old run's later pages
/// read the clock); the same document without the read does.
#[test]
fn elapsed_time_is_a_barrier() {
    let Some(e) = env() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let dir = e.dir.join("elapsed");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |word: &str, read: bool| -> String {
        let mut s = String::from("\\documentclass{article}\n\\begin{document}\n");
        for i in 0..150 {
            s.push_str(&para(i, if i == 5 { word } else { "lorem" }));
        }
        if read {
            // read, and print nothing: the output stays reproducible
            s.push_str("\\ifnum\\pdfelapsedtime<0 never\\fi\n");
        }
        s.push_str("\\end{document}\n");
        s
    };
    for read in [false, true] {
        let mut h = Host::start(&e, &dir);
        for _ in 0..3 {
            compile_and_check(
                &e,
                &mut h,
                &dir,
                &[("doc.tex", &doc("lorem", read))],
                "settle",
            );
        }
        // (the same letters: the fonts' used characters stay the same)
        let r = compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("doc.tex", &doc("lorme", read))],
            "an edit on page 1",
        );
        let conv = field(&r, "converged_at");
        if read {
            assert_eq!(conv, "null", "converged past a read of the clock: {r}");
        } else {
            assert_ne!(conv, "null", "the control document did not converge: {r}");
        }
    }
}

/// `\pdfuniformdeviate`, `\pdfnormaldeviate`, `\pdfrandomseed` and
/// `\pdfcreationdate` come from the pinned clock and the state the
/// checkpoints hold: incremental compiles equal from-scratch runs.
#[test]
fn random_numbers_and_dates_equal_scratch_runs() {
    let Some(e) = env() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let dir = e.dir.join("random");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |word: &str| -> String {
        let mut s = String::from(
            "\\documentclass{article}\n\\begin{document}\nCreated \\pdfcreationdate, seed \\the\\pdfrandomseed.\n\n",
        );
        for i in 0..40 {
            s.push_str(&para(i, if i == 12 { word } else { "lorem" }));
            s.push_str("Drawn \\pdfuniformdeviate 1000\\ and \\pdfnormaldeviate.\n\n");
            if i == 25 {
                s.push_str("\\pdfsetrandomseed 4242 Reseeded \\pdfuniformdeviate 77.\n\n");
            }
        }
        s.push_str("\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    for _ in 0..3 {
        compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc("alpha"))], "settle");
    }
    for w in ["alphb", "alpha", "a longer word here"] {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(w))], w);
        assert!(r.contains("\"mode\":\"incremental\""), "{r}");
    }
}

/// DESIGN.md §5.5: passes stop on a repeated state. A label whose page
/// decides how much text precedes it (long text when it was on page 1 last
/// time, which pushes it to page 2, and the other way round) never settles:
/// the host stops when the `.aux` repeats one a pass already read, as the
/// from-scratch runs repeated by the same rule do, and both end equal.
#[test]
fn oscillating_labels_stop_on_a_repeated_state() {
    let Some(e) = env() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let dir = e.dir.join("oscillation");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |word: &str| -> String {
        let mut s = String::from(
            "\\documentclass{article}\n\\makeatletter\n\
             \\def\\secondof#1#2#3\\relax{#2}\n\
             \\newcommand\\pageof[1]{\\expandafter\\ifx\\csname r@#1\\endcsname\\relax 0\\else\
             \\expandafter\\expandafter\\expandafter\\secondof\\csname r@#1\\endcsname\\relax\\fi}\n\
             \\makeatother\n\\begin{document}\n",
        );
        for i in 0..16 {
            s.push_str(&para(i, if i == 2 { word } else { "lorem" }));
        }
        s.push_str("\\ifnum\\pageof{x}>1 Short.\\else ");
        for i in 0..30 {
            s.push_str(&format!("Filler sentence {i} that takes up room on the page. "));
        }
        s.push_str("\\fi\n\n\\label{x}The label.\n\n");
        for i in 16..21 {
            s.push_str(&para(i, "lorem"));
        }
        s.push_str("\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    let mut stopped = false;
    for w in ["alpha", "alpha", "alphb", "alpha"] {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(w))], w);
        stopped |= r.contains("\"oscillation\":true");
    }
    assert!(stopped, "no compile stopped on a repeated state");
}
