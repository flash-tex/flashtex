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

mod common;

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
    // one directory of this process's own (common::fresh_dir), made once
    static BASE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    let base = BASE
        .get_or_init(|| common::fresh_dir("flashtex-incr"))
        .clone();
    let fmt = base.join("fmt");
    std::fs::create_dir_all(&fmt).unwrap();
    let pdftex = fmt.join("pdftex");
    common::link_engine(initex, &pdftex);
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
        Host::start_env(e, dir, &[])
    }

    fn start_env(e: &Env, dir: &Path, env: &[(&str, &str)]) -> Host {
        let mut c = Command::new(env!("CARGO_BIN_EXE_flashtex-host"));
        c.arg("iserve").arg("--").args(ARGS).current_dir(dir);
        engine_env(&mut c, e);
        c.envs(env.iter().copied());
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
        let t = f.file_type().unwrap();
        if t.is_file() {
            std::fs::copy(f.path(), to.join(f.file_name())).unwrap();
        } else if t.is_symlink() {
            // (a link as it is: `a_file_under_two_names_shifts_once`)
            let target = std::fs::read_link(f.path()).unwrap();
            std::os::unix::fs::symlink(target, to.join(f.file_name())).unwrap();
        } else if t.is_dir() {
            copy_dir(&f.path(), &to.join(f.file_name()));
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
    check_against(e, dir, &reference, &report, what);
    report
}

/// Run the reference chain in `reference` (a copy of the directory the
/// compile should be equal to a from-scratch run on) and compare `dir`'s
/// outputs with it.
fn check_against(e: &Env, dir: &Path, reference: &Path, report: &str, what: &str) {
    let mut seen = vec![];
    for _ in 0..5 {
        seen.push(dir_state(reference));
        let mut c = Command::new(e.fmt.join("pdftex"));
        c.args(ARGS).current_dir(reference);
        engine_env(&mut c, e);
        c.env("FLASHTEX_PREVIEW", "1");
        c.stdin(Stdio::null()).stdout(Stdio::null());
        c.status().unwrap();
        if seen.contains(&dir_state(reference)) {
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
}

#[test]
fn a_failed_run_then_a_revert() {
    let Some(e) = env() else {
        common::no_texlive();
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
    // (the failed run's PDF is gone, as pdfTeX leaves it: `check_against`)
    assert!(!dir.join("doc.pdf").exists());
    let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc)], "the revert");
    // Lane ERROR-RECOVERY: the failed run's PDF was set aside, not lost
    // (`system::remove_output`), so the revert restarts from a checkpoint
    // instead of from the format.
    assert!(
        !field(&r, "mode").contains("cold"),
        "the revert compiled from scratch: {r}"
    );
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
        common::no_texlive();
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
        common::no_texlive();
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
/// The report's `diffs` (why each tested checkpoint did not converge) of
/// the pages before `page`, as `(page, reason)`.
fn diffs_before(report: &str, page: usize) -> Vec<(usize, String)> {
    let key = "\"diffs\":[";
    let Some(at) = report.find(key).map(|i| i + key.len()) else {
        return Vec::new();
    };
    // (a reason may hold `]`: `mem[...]`; the list ends at `"]`, or is `[]`)
    let end = if report[at..].starts_with(']') {
        0
    } else {
        report[at..].find("\"]").map_or(0, |i| i + 1)
    };
    let list = &report[at..at + end];
    list.split('"')
        .skip(1)
        .step_by(2)
        .filter_map(|d| {
            let (p, why) = d.strip_prefix("page ")?.split_once(": ")?;
            let p: usize = p.parse().ok()?;
            (p < page).then(|| (p, why.to_string()))
        })
        .collect()
}

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
///
/// The host keeps checkpoints at shipouts and the `.aux` points only (no
/// segment or timed ones, which are spaced in engine time): which ones a
/// pass keeps, and so whether one restarts past the `.aux` point, must not
/// depend on the machine's load.
#[test]
fn structural_edits_equal_scratch_runs() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("structural");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let base = refs_doc("", 8);
    let mut h = Host::start_env(
        &e,
        &dir,
        &[("FLASHTEX_SEGMENT_S", "off"), ("FLASHTEX_TIMED_S", "0")],
    );
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
/// after the last page's text never keeps the old run's last page (it read
/// the clock): the run converges with the old one before it and goes on
/// live from the last page checkpoint before the read (P6-HYPEROPT), or does
/// not converge; the same document without the read converges.
#[test]
fn elapsed_time_is_a_barrier() {
    let Some(e) = env() else {
        common::no_texlive();
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
            // the last page (which reads the clock) re-typeset live
            let pages: usize = field(&r, "pages").parse().unwrap();
            let kept = field(&r, "rerun_from");
            assert!(
                conv == "null" || kept.parse::<usize>().is_ok_and(|k| k < pages),
                "kept a page that read the clock: {r}"
            );
        } else {
            assert_ne!(conv, "null", "the control document did not converge: {r}");
        }
    }
}

/// A `\write18` in the body is not the preamble's: imakeidx runs makeindex
/// at `\printindex` through restricted `\write18` on every pass (a 592-page
/// textbook, docs/evidence/infdesc-2026-10-03), and that must leave S₀
/// usable, so an edit compiles incrementally and the passes stop once the
/// files they read repeat. S₀'s key held every barrier of the whole run, so
/// every compile of such a document was cold and ran `MAX_PASSES` passes.
/// A `\write18` in the preamble still makes S₀ unusable.
///
/// The command is `kpsewhich`, which every TeX Live has and texmf.cnf's
/// `shell_escape_commands` allows in the default restricted mode (makeindex
/// and imakeidx are not in every scheme: the CI image lacks them). It looks
/// up a file that does not exist, so it prints nothing on the stdout the
/// host's protocol shares with the command.
#[test]
fn a_write18_in_the_body_keeps_s0() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    const RUN: &str = "\\immediate\\write18{kpsewhich no-such-file.xyz}\n";
    let doc = |word: &str, in_preamble: bool| -> String {
        let mut s = String::from("\\documentclass{article}\n");
        if in_preamble {
            s.push_str(RUN);
        }
        s.push_str("\\begin{document}\n");
        for i in 0..60 {
            s.push_str(&para(i, if i == 5 { word } else { "lorem" }));
        }
        // at the end, as imakeidx's makeindex at \printindex
        s.push_str(RUN);
        s.push_str("\\end{document}\n");
        s
    };
    for in_preamble in [false, true] {
        let dir = e.dir.join(format!("write18-{in_preamble}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut h = Host::start(&e, &dir);
        let r = compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("doc.tex", &doc("lorem", in_preamble))],
            "cold",
        );
        assert_eq!(field(&r, "status"), "0", "{r}");
        let log = std::fs::read_to_string(dir.join("doc.log")).unwrap_or_default();
        let ran = log
            .lines()
            .filter(|l| l.starts_with("runsystem(kpsewhich") && l.contains("executed"))
            .count();
        assert_eq!(
            ran,
            if in_preamble { 2 } else { 1 },
            "the \\write18 did not run (restricted mode, kpsewhich): {r}\n{log}"
        );
        if !in_preamble {
            // the passes stop when nothing they read changed, as pdflatex's
            // reruns would (not at MAX_PASSES)
            assert_ne!(field(&r, "passes"), "5", "{r}");
        }
        let r = compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("doc.tex", &doc("lorme", in_preamble))],
            "an edit on page 1",
        );
        if in_preamble {
            assert!(
                r.contains("the preamble ran an external command (write18)"),
                "{r}"
            );
        } else {
            assert!(r.contains("\"mode\":\"incremental\""), "{r}");
        }
    }
}

/// `\pdfuniformdeviate`, `\pdfnormaldeviate`, `\pdfrandomseed` and
/// `\pdfcreationdate` come from the pinned clock and the state the
/// checkpoints hold: incremental compiles equal from-scratch runs.
#[test]
fn random_numbers_and_dates_equal_scratch_runs() {
    let Some(e) = env() else {
        common::no_texlive();
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
        common::no_texlive();
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
            s.push_str(&format!(
                "Filler sentence {i} that takes up room on the page. "
            ));
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

/// Lane P4-MULTIPASS, soundness cases 2030 and 2031: two `.aux` changes
/// L5's proof (`readset::aux_delta`, then `same_words` with the old
/// meanings put back) used to reject, re-reading the `.aux` from the `.aux`
/// point instead (the whole document again; seen on biblatex documents):
///
/// * 2030: an entry `\let` to a macro whose body other control sequences
///   share (an etoolbox toggle is `\@firstoftwo`/`\@secondoftwo`): putting
///   the old meaning back built a copy of the body, so the shared list's
///   reference count was one less than the old run's.
/// * 2031: a re-read `.aux` that nests groups deeper than the old one: the
///   high-water mark `max_save_stack` differs, a statistic only the log's
///   capacity block prints (DESIGN.md §1.1).
///
/// Each edit (on the last page, which alone reads the entry) must restart
/// the `.aux` pass at the entry's first read, and every compile equals
/// from-scratch runs.
#[test]
fn l5_shared_bodies_and_deeper_aux_reads_equal_scratch_runs() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("l5-2030-2031");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |which: &str, depth: usize| -> String {
        let mut s = String::from("\\documentclass{article}\n\\begin{document}\n\\makeatletter\n");
        for i in 0..40 {
            s.push_str(&para(i, "delta"));
        }
        s.push_str(
            "Toggle: \\@ifundefined{flagA}{unset}{\\flagA{first}{second}}; \
             depth \\@ifundefined{depthmark}{unset}{\\depthmark}.\n\n",
        );
        s.push_str(&format!(
            "\\immediate\\write\\@auxout{{\\string\\global\\string\\let\\string\\flagA\\string\\{which}}}\n"
        ));
        let open = "{".repeat(depth);
        let close = "}".repeat(depth);
        s.push_str(&format!(
            "\\immediate\\write\\@auxout{{\\unexpanded{{{open}\\gdef\\depthmark{{{depth}}}{close}}}}}\n"
        ));
        s.push_str("\\makeatother\n\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    for _ in 0..3 {
        let r = compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("doc.tex", &doc("@firstoftwo", 5))],
            "settle",
        );
        if r.contains("\"mode\":\"unchanged\"") {
            break;
        }
    }
    let cases = [
        (
            "2030: a toggle let to another shared body",
            "@secondoftwo",
            5,
        ),
        ("2030: the toggle back", "@firstoftwo", 5),
        ("2031: an .aux read nesting deeper", "@firstoftwo", 6),
        ("2031: and less deep again", "@firstoftwo", 5),
    ];
    for (what, which, depth) in cases {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(which, depth))], what);
        let l5 = r.split("\"l5\":").nth(1).unwrap_or("");
        assert!(
            l5.contains("restart at page") && !l5.contains("re-read from the .aux point"),
            "{what}: the .aux pass did not restart at the entry's first read: {l5}"
        );
    }
}

/// Lane P4-MULTIPASS, soundness case 2032: case 2030's shared body when the
/// only other control sequences sharing it live in tex.ch's `hash_extra`
/// region above `eqtb_size` (#1285). A 22,000-name preamble flood fills the
/// 15,000-slot hash, so `\xC` and `\xD`, defined after it, are allocated up
/// there. The `.aux` entry `\let\flagA\xC` toggles to `\xD` and back: putting
/// the old meaning back must find `\xC` as the list's other holder
/// (`readset::View::shared_list`) and add a reference, not build a copy whose
/// reference count differs, so each edit restarts the `.aux` pass at the
/// entry's first read. The Muse lead's alias shapes (`\let\xB\@firstoftwo`,
/// `\let\xA\xB`) run after the flood too and must equal scratch runs.
#[test]
fn l5_shared_bodies_in_the_hash_extra_region_equal_scratch_runs() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("l5-2032");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |which: &str, alias: &str| -> String {
        let mut s = String::from(
            "\\documentclass{article}\n\\makeatletter\n\
             \\count@=0\n\
             \\loop\\expandafter\\def\\csname flood\\the\\count@\\endcsname{}%\n\
             \\advance\\count@ 1 \\ifnum\\count@<22000 \\repeat\n\
             \\def\\xC#1#2{#1}\\def\\xD#1#2{#2}\n\
             \\makeatother\n\\begin{document}\n\\makeatletter\n",
        );
        for i in 0..40 {
            s.push_str(&para(i, "kappa"));
        }
        s.push_str(
            "Toggle: \\@ifundefined{flagA}{unset}{\\flagA{first}{second}}; \
             alias \\@ifundefined{xA}{unset}{\\xA{one}{two}}.\n\n",
        );
        s.push_str(&format!(
            "\\immediate\\write\\@auxout{{\\string\\global\\string\\let\\string\\flagA\\string\\{which}}}\n"
        ));
        s.push_str(&format!(
            "\\immediate\\write\\@auxout{{\\string\\global\\string\\let\\string\\xB\\string\\{alias}}}\n\
             \\immediate\\write\\@auxout{{\\string\\global\\string\\let\\string\\xA\\string\\xB}}\n"
        ));
        s.push_str("\\makeatother\n\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    for _ in 0..3 {
        let r = compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("doc.tex", &doc("xC", "@firstoftwo"))],
            "settle",
        );
        if r.contains("\"mode\":\"unchanged\"") {
            break;
        }
    }
    for (what, which) in [
        ("2032: a toggle let to a body shared in hash_extra", "xD"),
        ("2032: the toggle back", "xC"),
    ] {
        let r = compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("doc.tex", &doc(which, "@firstoftwo"))],
            what,
        );
        let l5 = r.split("\"l5\":").nth(1).unwrap_or("");
        assert!(
            l5.contains("restart at page") && !l5.contains("re-read from the .aux point"),
            "{what}: the .aux pass did not restart at the entry's first read: {l5}"
        );
    }
    // the alias chain's target changes: compared with scratch runs only
    for (what, alias) in [
        ("2032: the alias chain to another body", "@secondoftwo"),
        ("2032: the alias chain back", "@firstoftwo"),
    ] {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc("xC", alias))], what);
        let l5 = r.split("\"l5\":").nth(1).unwrap_or("");
        eprintln!(
            "{what}: {}",
            if l5.contains("re-read from the .aux point") {
                "fell back to the .aux point"
            } else if l5.contains("restart at page") {
                "restarted at the entry's first read"
            } else {
                "neither (see the report)"
            }
        );
    }
}

/// Preemption: a compile interrupted by a newer edit (in its first pass, or
/// in the `.aux` pass that follows a label move) is not finished; the next
/// compile, of the newer edit, equals from-scratch runs on the directory as
/// the last finished compile left it with the newer sources (DESIGN.md
/// §5.5: the previous run's `.aux` is fixed input), whether it keeps the
/// interrupted run's pages (the edit is behind it) or goes back to the run
/// that was being replaced (the edit is ahead of it).
#[test]
fn interleaved_edits_equal_scratch_runs() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("interleaved");
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
    let moved = refs_doc(&"Words that move the labels. ".repeat(30), 8);
    let late = base.replacen("Paragraph 40 with", "Paragraph 40 now with", 1);
    let early = base.replacen("Paragraph 3 with", "Paragraph 3 now with", 1);
    // (first edit, interrupt as "PASS PAGES", second edit)
    let cases: Vec<(&str, &str, &str, &str)> = vec![
        (
            "a label move, then its revert, in pass 1",
            &moved,
            "1 1",
            &base,
        ),
        (
            "a label move, then its revert, in the .aux pass",
            &moved,
            "2 1",
            &base,
        ),
        ("an early edit, then a later one", &early, "1 1", &late),
        ("a label move, then an early edit", &moved, "1 2", &early),
        ("a late edit, then an early one", &late, "1 1", &early),
    ];
    let mut interrupted = 0;
    for (what, first, at, second) in cases {
        let reference = dir.with_extension("ref");
        copy_dir(&dir, &reference);
        std::fs::write(dir.join("doc.tex"), first).unwrap();
        let r = h.cmd(&format!("compile-interrupt {at}"));
        if r.contains("\"preempted\":true") {
            interrupted += 1;
            std::fs::write(dir.join("doc.tex"), second).unwrap();
            std::fs::write(reference.join("doc.tex"), second).unwrap();
            let r2 = h.cmd("compile");
            check_against(&e, &dir, &reference, &r2, what);
        } else {
            std::fs::write(reference.join("doc.tex"), first).unwrap();
            check_against(&e, &dir, &reference, &r, what);
        }
        compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("doc.tex", &base)],
            &format!("{what}: back"),
        );
    }
    assert!(
        interrupted >= 3,
        "only {interrupted} compiles were interrupted"
    );
}

/// Settle `dir` on `base`, write `first`, compile it interrupted at `at`
/// ("PASS PAGES"), which must preempt it, then write `second` and compile:
/// the result must equal from-scratch runs on the directory as the last
/// complete compile left it, with `second`.
fn interrupt_then(e: &Env, name: &str, base: &str, first: &str, at: &str, second: &str) {
    let dir = e.dir.join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = Host::start(e, &dir);
    for _ in 0..3 {
        let r = compile_and_check(e, &mut h, &dir, &[("doc.tex", base)], "settle");
        if r.contains("\"mode\":\"unchanged\"") {
            break;
        }
    }
    let reference = dir.with_extension("ref");
    copy_dir(&dir, &reference);
    std::fs::write(dir.join("doc.tex"), first).unwrap();
    let r = h.cmd(&format!("compile-interrupt {at}"));
    assert!(r.contains("\"preempted\":true"), "not interrupted: {r}");
    std::fs::write(dir.join("doc.tex"), second).unwrap();
    std::fs::write(reference.join("doc.tex"), second).unwrap();
    let r2 = h.cmd("compile");
    check_against(e, &dir, &reference, &r2, name);
}

/// P4-SOUNDNESS-D (sweep D, refs-30 `9:revert-after-interrupt`): a removed
/// label, interrupted in the `.aux` pass, then reverted. The interrupted
/// pass is abandoned; the revert's pass reads the `.aux` as the complete
/// run read it (a fixed input) and converges with that run, whose later
/// `.aux` bytes the convergence jump appends. Those bytes were taken from
/// the disk after the fixed input had been written there, so the `.aux`
/// got the read content's bytes at the old run's offsets: a broken `.aux`
/// and a false convergence. The fixed input is now written after the
/// restore has kept the old run's output.
#[test]
fn a_reverted_label_removal_interrupted_in_the_aux_pass() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let base = refs_doc("", 8);
    let unlabel = base.replacen("\\label{sec:2}", "", 1);
    interrupt_then(&e, "unlabel-revert", &base, &unlabel, "2 1", &base);
}

/// P4-SOUNDNESS-D (sweep D, min-float-table `11:second-after-interrupt`):
/// a new label, interrupted in the `.aux` pass after the first page, which
/// opened (truncated) the PDF; the next edit breaks `\end{document}`, so its
/// run ends on a fatal error before it ships a page and never opens the PDF.
/// The interrupted run's pages are kept (`settle_paused`), and its partial
/// PDF stayed on disk where a scratch run leaves the last complete run's.
/// The settled run's truncated files are now put back as that run left
/// them when a restart is before their truncation.
#[test]
fn a_fatal_edit_after_an_interrupted_aux_pass_keeps_the_pdf() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let base = "\\documentclass{article}\n\\begin{document}\n\nBody text before.\n\n\
                Body text after the float, up by the height of the table.\n\\end{document}\n";
    let label = base.replacen("the height", "the height\\label{lab:new}", 1);
    let fatal = label.replacen("\\end{document}", "\\jend{document}", 1);
    interrupt_then(&e, "fatal-after-interrupt", base, &label, "2 1", &fatal);
}

/// genvol.py's vol-closed, shorter: three blocks, each writes a file with
/// `\immediate\write`, closes it, ships a page or two and `\input`s it.
fn vol_closed_doc() -> String {
    let mut s = String::from("\\documentclass{article}\n\\newwrite\\tmp\n\\begin{document}\n\n");
    for k in 0..3 {
        s.push_str(&format!(
            "\\immediate\\openout\\tmp=\\jobname-tmp.tex\n\
             \\immediate\\write\\tmp{{Instance {k} says {}.}}\n\\immediate\\closeout\\tmp\n\n",
            "x".repeat(k + 1)
        ));
        for i in 0..25 {
            s.push_str(&para(k * 25 + i, "delta"));
        }
        s.push_str("\\input{\\jobname-tmp.tex}\n\n");
    }
    s.push_str("\\end{document}\n");
    s
}

/// Issue #1550 (soundness sweep D, vol-closed `3:second-after-interrupt`):
/// an edit breaks a `\closeout` (`\closeouet`), so the file is still open
/// for output when it is `\input` pages later. pdfTeX's `\write` line is
/// still in the stream's buffer then, and the `\input` reads an empty file;
/// a checkpoint between them had flushed the buffer, and the run read the
/// line. Such a read now redoes the run from the format, with that file
/// never flushed by a checkpoint (`system::no_flush`). Also on the
/// document's first compile, and back.
#[test]
fn a_file_read_while_open_for_output_is_read_as_from_scratch() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let base = vol_closed_doc();
    let lost =
        base.replacen("\\closeout", "\\closeouet", 2)
            .replacen("\\closeouet", "\\closeout", 1);
    assert_ne!(base, lost);
    let dir = e.dir.join("read-while-open");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = Host::start(&e, &dir);
    for _ in 0..3 {
        compile_and_check(&e, &mut h, &dir, &[("doc.tex", &base)], "settle");
    }
    compile_and_check(&e, &mut h, &dir, &[("doc.tex", &lost)], "closeout lost");
    compile_and_check(
        &e,
        &mut h,
        &dir,
        &[("doc.tex", &lost)],
        "closeout lost, again",
    );
    let edited = lost.replacen("Paragraph 40 with", "Paragraph 40 now with", 1);
    compile_and_check(
        &e,
        &mut h,
        &dir,
        &[("doc.tex", &edited)],
        "an edit after it",
    );
    compile_and_check(&e, &mut h, &dir, &[("doc.tex", &base)], "back");
    drop(h);
    // the first compile, from the format
    let dir = e.dir.join("read-while-open-first");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = Host::start(&e, &dir);
    compile_and_check(&e, &mut h, &dir, &[("doc.tex", &lost)], "first compile");
}

/// Issue #1550 as sweep D found it: the broken `\closeout` arrives while
/// the compile of an earlier edit is stopped.
#[test]
fn a_file_read_while_open_for_output_after_an_interrupt() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let base = vol_closed_doc();
    let first = base.replacen(
        "\\immediate\\closeout\\tmp\n\n",
        "\\immediate\\closeout\\tmp\n\n\\section{Inserted}\n\n",
        2,
    );
    let first = first.replacen(
        "\\immediate\\closeout\\tmp\n\n\\section{Inserted}\n\n",
        "\\immediate\\closeout\\tmp\n\n",
        1,
    );
    let second =
        first
            .replacen("\\closeout", "\\closeouet", 2)
            .replacen("\\closeouet", "\\closeout", 1);
    assert!(first != base && second != first);
    interrupt_then(
        &e,
        "read-while-open-interrupt",
        &base,
        &first,
        "1 2",
        &second,
    );
}

/// P4-COLD-PREEMPT (Commander ruling, DESIGN.md §5.1/§5.3): a run from the
/// format -- a document's first compile, or one after a preamble edit --
/// stops for newer work once it has taken S₀, and keeps S₀ and the
/// checkpoints it took: the next compile restarts from them, not from the
/// format, and equals scratch runs on the directory as the last complete
/// compile left it (the stopped run's `.aux`, `.toc` and PDF are its own
/// partial output, not an input). A newer preamble edit starts from the
/// format again, as cleanly.
#[test]
fn a_cold_run_stopped_past_s0_is_kept() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let base = refs_doc("", 8);
    // (hyperref opens the PDF at `\begin{document}`, before S₀; without it
    // the first page does, after S₀)
    let plain = base.replacen("\\usepackage{hyperref}\n", "", 1);
    let mark = |s: &str| {
        s.replacen(
            "\\begin{document}",
            "% a preamble edit\n\\begin{document}",
            1,
        )
    };
    type Edit = fn(&str) -> String;
    // (the document, the edit behind the stopped run, the mode its compile
    // starts in)
    let cases: [(&str, &str, Edit, &str); 7] = [
        (
            "an edit before the stop",
            &base,
            |s| s.replacen("Paragraph 3 with", "Paragraph 3 now with", 1),
            "incremental",
        ),
        (
            "an edit after the stop",
            &base,
            |s| s.replacen("Paragraph 40 with", "Paragraph 40 now with", 1),
            "incremental",
        ),
        (
            "a new label",
            &base,
            |s| s.replacen("Paragraph 4 with", "Paragraph 4\\label{lab:new} with", 1),
            "incremental",
        ),
        (
            "a fatal edit",
            &base,
            |s| s.replacen("\\end{document}", "\\jend{document}", 1),
            "incremental",
        ),
        // a fatal error before the first page: the PDF stays as the last
        // complete run left it, not as the stopped run did
        (
            "a fatal edit before the first page",
            &plain,
            |s| {
                s.replacen(
                    "Paragraph 1 with",
                    "\\input{no-such-file}Paragraph 1 with",
                    1,
                )
            },
            "incremental",
        ),
        (
            "a fatal edit before the first page, with hyperref",
            &base,
            |s| {
                s.replacen(
                    "Paragraph 1 with",
                    "\\input{no-such-file}Paragraph 1 with",
                    1,
                )
            },
            "incremental",
        ),
        (
            "the preamble edit reverted",
            &base,
            |s| s.replacen("% a preamble edit\n", "", 1),
            "cold",
        ),
    ];
    for (i, (what, base, edit, mode)) in cases.iter().enumerate() {
        let marked = mark(base);
        let second = edit(&marked);
        let dir = e.dir.join(format!("cold-stop-{i}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut h = Host::start(&e, &dir);
        for _ in 0..3 {
            let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", base)], "settle");
            if r.contains("\"mode\":\"unchanged\"") {
                break;
            }
        }
        let reference = dir.with_extension("ref");
        copy_dir(&dir, &reference);
        std::fs::write(dir.join("doc.tex"), &marked).unwrap();
        let r = h.cmd("compile-interrupt 1 2");
        assert!(r.contains("\"preempted\":true"), "{what}: not stopped: {r}");
        assert_eq!(field(&r, "mode"), "\"cold\"", "{what}: {r}");
        std::fs::write(dir.join("doc.tex"), &second).unwrap();
        std::fs::write(reference.join("doc.tex"), &second).unwrap();
        let r2 = h.cmd("compile");
        assert_eq!(field(&r2, "mode"), format!("\"{mode}\""), "{what}: {r2}");
        check_against(&e, &dir, &reference, &r2, what);
        // and back
        compile_and_check(&e, &mut h, &dir, &[("doc.tex", base)], what);
    }
    // Opening a document compiled before (a new session's first compile is
    // from the format), and one never compiled (lane COLD-OPEN): with no
    // `.aux` its `.aux` point is where `\document` begins, before the lookup
    // that found none, so the `.aux` the stopped run wrote is not in S₀'s
    // key; the next compile keeps what the stopped run typeset and restarts
    // before the edit, taking the `.aux` as that run looked for it (not
    // there: `fixed_created`), and its next pass sees the `.aux`.
    for (i, compiled) in [true, false].into_iter().enumerate() {
        let dir = e.dir.join(format!("cold-stop-open-{i}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        if compiled {
            let mut h = Host::start(&e, &dir);
            for _ in 0..3 {
                let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &base)], "settle");
                if r.contains("\"mode\":\"unchanged\"") {
                    break;
                }
            }
        } else {
            std::fs::write(dir.join("doc.tex"), &base).unwrap();
        }
        let reference = dir.with_extension("ref");
        copy_dir(&dir, &reference);
        let mut h = Host::start(&e, &dir);
        let r = h.cmd("compile-interrupt 1 2");
        assert!(r.contains("\"preempted\":true"), "not stopped: {r}");
        let edited = base.replacen("Paragraph 3 with", "Paragraph 3 now with", 1);
        std::fs::write(dir.join("doc.tex"), &edited).unwrap();
        std::fs::write(reference.join("doc.tex"), &edited).unwrap();
        let r2 = h.cmd("compile");
        assert_eq!(field(&r2, "mode"), "\"incremental\"", "{r2}");
        check_against(&e, &dir, &reference, &r2, "opened and stopped");
    }
}

/// Lane COLD-OPEN: a first compile (no `.aux`) takes its `.aux` point where
/// `\document` begins, before it looks for the `.aux`, so the passes the new
/// `.aux` asks for restart there, not from the format; the output is a
/// scratch run's.
#[test]
fn a_first_compiles_later_passes_start_at_the_aux_point() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let base = refs_doc("", 8);
    let dir = e.dir.join("first-passes");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("doc.tex"), &base).unwrap();
    let reference = dir.with_extension("ref");
    copy_dir(&dir, &reference);
    let mut h = Host::start(&e, &dir);
    let r = h.cmd("compile");
    assert_eq!(field(&r, "mode"), "\"cold\"", "{r}");
    assert!(
        r.contains("\"pass_modes\":[\"cold\", \"incremental\""),
        "the second pass is from the format: {r}"
    );
    check_against(&e, &dir, &reference, &r, "a first compile");
}

/// P4-COLD-PREEMPT: newer work that arrives before a run from the format
/// has taken S₀ stops it at its first page or segment checkpoint after S₀,
/// not before (the preamble has checkpoints: each `\par` of a package runs
/// `build_page`). Stopped earlier, the next compile would start from the
/// format again -- while the user types, at every keystroke, and a long
/// preamble would never be passed; stopped there, the next compile restarts
/// from S₀.
#[test]
fn a_cold_run_stops_no_earlier_than_s0() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let mut doc = String::from("\\documentclass{article}\n\\usepackage{hyperref}\n");
    for k in 0..40 {
        doc.push_str(&format!(
            "\\newcommand\\macro{}{{word {k}}}\n\n",
            roman(k + 1)
        ));
    }
    doc.push_str("\\begin{document}\n");
    for i in 0..40 {
        doc.push_str(&para(i, "delta"));
    }
    doc.push_str("\\end{document}\n");
    let dir = e.dir.join("cold-before-s0");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    {
        let mut h = Host::start(&e, &dir);
        for _ in 0..3 {
            let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc)], "settle");
            if r.contains("\"mode\":\"unchanged\"") {
                break;
            }
        }
    }
    let reference = dir.with_extension("ref");
    copy_dir(&dir, &reference);
    // a new session: its first compile is from the format
    let mut h = Host::start(&e, &dir);
    // newer work at the first point the run could stop
    let r = h.cmd("compile-interrupt 1 0");
    assert!(r.contains("\"preempted\":true"), "not stopped: {r}");
    let edited = doc.replacen("Paragraph 30 with", "Paragraph 30 now with", 1);
    std::fs::write(dir.join("doc.tex"), &edited).unwrap();
    std::fs::write(reference.join("doc.tex"), &edited).unwrap();
    let r2 = h.cmd("compile");
    assert_eq!(field(&r2, "mode"), "\"incremental\"", "{r2}");
    check_against(&e, &dir, &reference, &r2, "stopped at S₀");
}

/// `k` in lower-case roman numerals (control sequence names of letters).
fn roman(mut k: usize) -> String {
    let mut s = String::new();
    for (v, r) in [(10, "x"), (9, "ix"), (5, "v"), (4, "iv"), (1, "i")] {
        while k >= v {
            s.push_str(r);
            k -= v;
        }
    }
    s
}

/// Issue #1294: `\tableofcontents` twice opens the `.toc` on two streams,
/// and the second writes it. An edit of a file `\input` right after a
/// `\write` to the `.toc` restarts at the checkpoint between the two
/// (restart points at every input line, `FLASHTEX_TIMED_S`), where the
/// line is still in the second stream's buffer. The restore cut the file
/// to the first stream's length (0) and extended it to the second's:
/// zeros. The `.toc` (and the PDF, log and `.aux`) must equal a scratch
/// run's.
#[test]
fn a_restart_after_a_toc_write_keeps_the_toc() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("twotocs");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut doc = String::from(
        "\\documentclass{article}\n\\begin{document}\n\\tableofcontents\n\\tableofcontents\n",
    );
    for (k, name) in ["One", "Two", "Three"].iter().enumerate() {
        doc.push_str(&format!("\\section{{{name}}}\n"));
        for i in 0..6 {
            doc.push_str(&para(10 * k + i, "alpha"));
        }
    }
    doc.push_str(
        "\\makeatletter\n\
         \\relax\\immediate\\write\\tf@toc{\\string\\contentsline{section}{Written}{9}{}}\\makeatother\n\
         \\input{tail}\n\
         \\end{document}\n",
    );
    let mut h = Host::start_env(&e, &dir, &[("FLASHTEX_TIMED_S", "0.0000001")]);
    let check_toc = |what: &str| {
        let reference = dir.with_extension("ref");
        let (a, b) = (
            std::fs::read(dir.join("doc.toc")).unwrap(),
            std::fs::read(reference.join("doc.toc")).unwrap(),
        );
        assert!(!a.contains(&0), "{what}: doc.toc holds NUL bytes");
        assert_eq!(a, b, "{what}: doc.toc differs from a scratch run");
    };
    let tail = |w: &str| format!("The tail says {w}.\n");
    for _ in 0..4 {
        let r = compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("doc.tex", &doc), ("tail.tex", &tail("alpha"))],
            "settle",
        );
        check_toc("settle");
        if r.contains("\"mode\":\"unchanged\"") {
            break;
        }
    }
    for w in ["beta", "gamma", "alpha"] {
        let what = format!("tail {w}");
        let r = compile_and_check(&e, &mut h, &dir, &[("tail.tex", &tail(w))], &what);
        check_toc(&what);
        assert!(r.contains("\"mode\":\"incremental\""), "{what}: {r}");
    }
}

/// P4-FINISH: a change that only lengthens the longest input line (a
/// comment at the end of a one-line paragraph) changes `max_buf_stack`,
/// which only the end-of-run statistics print: the run converges and equals
/// scratch runs (but for that accounting). hyperref puts `\pdfdest`s (xyz:
/// their dimensions left unset) into the pages the convergence test
/// compares.
#[test]
fn a_longer_longest_line_converges() {
    let Some(e) = env() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let dir = e.dir.join("longest");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |pad: usize| -> String {
        let mut s =
            String::from("\\documentclass{article}\n\\usepackage{hyperref}\n\\begin{document}\n");
        for k in 0..6 {
            s.push_str(&format!("\\section{{Part {k}}}\\label{{sec:{k}}}\n"));
            for i in 0..8 {
                let mut p = para(k * 8 + i, "delta");
                if k == 1 && i == 2 {
                    // the longest line of the file
                    p = format!("{}%{}\n\n", p.trim_end(), "c".repeat(300 + pad));
                }
                s.push_str(&p);
            }
        }
        s.push_str("\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    for k in 0..4 {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(0))], "settle");
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
    for (pad, what) in [(1, "one more byte"), (0, "the revert"), (40, "forty more")] {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(pad))], what);
        assert!(r.contains("\"mode\":\"incremental\""), "{what}: {r}");
        assert_ne!(field(&r, "converged_at"), "null", "{what}: {r}");
    }
}

/// BEAMER-V3: a form (`\pdfxform`) is shipped after the page that first
/// refers to it, which flushes its box and deletes (and nulls) its
/// attribute and resource token lists; `pdf_mem` keeps the box pointer,
/// dangling where the run allocated the box, and nothing reads it again.
/// An edit before the form is made allocates the box elsewhere. The
/// convergence test compared the pointer, so a beamer deck (a pgf shading is such a form, drawn on every
/// slide) never converged after an edit before its first shading: every
/// keystroke re-typeset the deck to its end. The run converges now, and
/// every compile equals scratch runs.
#[test]
fn a_written_form_converges() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("xform");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |word: &str| -> String {
        let mut s = String::from("\\documentclass{article}\n\\begin{document}\n");
        s.push_str(&para(0, word));
        s.push_str(
            "\\setbox0\\hbox{\\vrule width 1cm height 4mm}%\n\
             \\pdfxform attr{/FlashTeXTest 1} resources{/ProcSet [/PDF]} 0\n\
             \\xdef\\form{\\the\\pdflastxform}\\noindent\\pdfrefxform\\form\\par\n\n",
        );
        for k in 1..80 {
            s.push_str(&para(k, "omega"));
            if k % 4 == 0 {
                s.push_str("\\noindent\\pdfrefxform\\form\\par\n\n");
            }
        }
        s.push_str("\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    for k in 0..4 {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc("omega"))], "settle");
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
    for (word, what) in [
        ("ome ga", "a space before the form"),
        ("omega", "the revert"),
        ("omegb", "a letter replaced"),
    ] {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(word))], what);
        assert!(r.contains("\"mode\":\"incremental\""), "{what}: {r}");
        assert_ne!(field(&r, "converged_at"), "null", "{what}: {r}");
    }
}

/// BEAMER-V3: a form made after an edit and not yet shipped keeps its box
/// (and token lists) where the run allocated them, and `pdf_mem` points at
/// them. The structural comparison follows those pointers (`Iso::object`),
/// but the test never handed it the `pdf_mem` words: they differed
/// "outside what the structural comparison reads", so a beamer deck with
/// such a form ahead (an overlay frame made one) re-typeset to its end. The
/// run converges now, and every compile equals scratch runs.
#[test]
fn an_unwritten_form_converges() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("xform-late");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |word: &str| -> String {
        let mut s = String::from("\\documentclass{article}\n\\begin{document}\n");
        s.push_str(&para(0, word));
        // made on the first page, shipped after the last
        s.push_str(
            "\\setbox0\\hbox{\\vrule width 2cm height 1mm}%\n\
             \\pdfxform attr{/FlashTeXTest 2} resources{/ProcSet [/PDF]} 0\n\
             \\xdef\\late{\\the\\pdflastxform}\n\n",
        );
        for k in 1..80 {
            s.push_str(&para(k, "omega"));
        }
        s.push_str("\\noindent\\pdfrefxform\\late\\par\n\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    for k in 0..4 {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc("omega"))], "settle");
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
    for (word, what) in [
        ("ome ga", "a space before the form"),
        ("omega", "the revert"),
        ("omegb", "a letter replaced"),
    ] {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(word))], what);
        assert!(r.contains("\"mode\":\"incremental\""), "{what}: {r}");
        assert_ne!(field(&r, "converged_at"), "null", "{what}: {r}");
    }
}

/// P4-FINISH: with a `\pdfsetmatrix` in effect, `\pdfdest` reads the
/// dimensions it left unset (pdftex.web's `set_rect_dimens`): the
/// convergence test compares them while the old run has such a read ahead.
/// Every compile equals scratch runs.
#[test]
fn destinations_under_a_matrix_equal_scratch_runs() {
    let Some(e) = env() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let dir = e.dir.join("matrix");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |word: &str| -> String {
        let mut s =
            String::from("\\documentclass{article}\n\\usepackage{hyperref}\n\\begin{document}\n");
        for k in 0..5 {
            s.push_str(&format!("\\section{{Part {k}}}\\label{{sec:{k}}}\n"));
            for i in 0..8 {
                let w = if k == 0 && i == 1 { word } else { "eta" };
                s.push_str(&para(k * 8 + i, w));
                if k >= 2 && i % 3 == 0 {
                    s.push_str(&format!(
                        "\\noindent\\pdfsave\\pdfsetmatrix{{1 0 0 1}}\\pdfdest name{{m{k}.{i}}} xyz\\pdfrestore\\pdfdest name{{n{k}.{i}}} fith\\par\n\n"
                    ));
                }
            }
        }
        s.push_str("\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    for k in 0..4 {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc("eta"))], "settle");
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
    for (word, what) in [
        ("etb", "a letter"),
        ("eta", "the revert"),
        ("etaa", "a letter more"),
    ] {
        compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(word))], what);
    }
}

/// P4-CONVERGENCE (T7, `docs/evidence/t7-latency-2026-10-02/`): hyperref's
/// `\pdfstringdefPreHook` (filled by siunitx) is a guarded intrinsic
/// (DESIGN.md §5.6 item 4). A letter added before the first page ships
/// makes the run allocate its token lists elsewhere, and the recordings
/// made after it hold pointers to them: watched meanings, pinned lists,
/// `\def` templates (`intr_data`). The convergence test compared those
/// words exactly, so such a run never converged and re-typeset every page
/// (full-100: 99 of 101 per keystroke). The structural comparison now
/// follows them (`Iso::intrinsics`): the run converges within a few pages,
/// and every compile still equals scratch runs.
#[test]
fn intrinsics_recorded_after_an_edit_converge() {
    let Some(e) = env() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let dir = e.dir.join("intrinsics");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |word: &str| -> String {
        let mut s = String::from(
            "\\documentclass{article}\n\\usepackage{siunitx}\n\\usepackage{hyperref}\n\
             \\begin{document}\n",
        );
        for k in 0..8 {
            s.push_str(&format!("\\section{{Part {k}}}\\label{{sec:{k}}}\n"));
            for i in 0..8 {
                let w = if k == 0 && i == 0 { word } else { "eta" };
                s.push_str(&para(k * 8 + i, w).repeat(3));
            }
        }
        s.push_str("\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    for k in 0..4 {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc("eta"))], "settle");
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
    for (word, what) in [("xeta", "a letter"), ("eta", "the revert")] {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(word))], what);
        assert!(r.contains("\"mode\":\"incremental\""), "{what}: {r}");
        assert_ne!(field(&r, "converged_at"), "null", "{what}: {r}");
    }
}

/// P4-EDIT-LATENCY: every converged compile adds a PDF-position correction
/// to each later checkpoint it keeps, and a restore of one of them applies
/// them all in order, rebuilding `rs_seen` once after the last. Letter
/// edits that converge in the middle, then edits near the end that restore
/// a checkpoint carrying all their corrections: every compile equals
/// scratch runs.
#[test]
fn a_restore_after_several_convergences_equals_scratch_runs() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("relocs");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |middle: &str, end: &str| -> String {
        let mut s =
            String::from("\\documentclass{article}\n\\usepackage{hyperref}\n\\begin{document}\n");
        for k in 0..6 {
            s.push_str(&format!(
                "\\section{{Part {k}}}\\label{{sec:{k}}}\nSee page~\\pageref{{sec:{}}}.\n\n",
                (k + 3) % 6
            ));
            for i in 0..10 {
                let word = match (k, i) {
                    (2, 4) => middle,
                    (5, 7) => end,
                    _ => "gamma",
                };
                s.push_str(&para(k * 10 + i, word));
            }
        }
        s.push_str("\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    for k in 0..4 {
        let r = compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("doc.tex", &doc("gamma", "gamma"))],
            "settle",
        );
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
    let mut converged = 0;
    for (i, w) in ["gammx", "gamma", "gammy", "gamma"].iter().enumerate() {
        let what = format!("middle edit {i}");
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(w, "gamma"))], &what);
        assert!(r.contains("\"mode\":\"incremental\""), "{what}: {r}");
        if field(&r, "converged_at") != "null" {
            converged += 1;
        }
    }
    assert!(
        converged >= 2,
        "only {converged} of 4 middle edits converged"
    );
    for (w, what) in [("gammz", "end edit"), ("gamma", "end revert")] {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc("gamma", w))], what);
        assert!(r.contains("\"mode\":\"incremental\""), "{what}: {r}");
    }
    compile_and_check(&e, &mut h, &dir, &[], "settle again");
}

/// P4-RESTART-PAGECOUNT (sweep D, min3-enumerate-only
/// `0:revert-after-interrupt`, then `1:label`): an interrupted compile is
/// abandoned by an edit before S₀, whose compile runs from scratch with a
/// new engine, whose checkpoint ids start again. The abandoned run's
/// restart point (its pages to be shipped again) was kept across that
/// cold run, and the next compile took it as a checkpoint of the new
/// engine: one before S₀ there (the longer preamble's restart points,
/// `FLASHTEX_TIMED_S`). That run restarted in the preamble and dropped S₀;
/// the compile after it failed with "restart point without a page count".
#[test]
fn an_abandoned_run_then_a_cold_run_keeps_no_stale_restart() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("abandoned-cold");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |preamble: usize, w: &str| {
        let mut d = String::from("\\documentclass{article}\n");
        d.push_str(&"\\relax\n".repeat(preamble));
        d.push_str("\\begin{document}\n");
        for i in 0..3 {
            d.push_str(&para(i, w));
        }
        d.push_str("\\end{document}\n");
        d
    };
    let mut h = Host::start_env(&e, &dir, &[("FLASHTEX_TIMED_S", "0.0000001")]);
    for _ in 0..4 {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(0, "alpha"))], "settle");
        if r.contains("\"mode\":\"unchanged\"") {
            break;
        }
    }
    // a body edit, interrupted after its page: paused with that page shipped
    let reference = dir.with_extension("ref");
    copy_dir(&dir, &reference);
    std::fs::write(dir.join("doc.tex"), doc(0, "beta")).unwrap();
    let r = h.cmd("compile-interrupt 1 1");
    assert!(r.contains("\"preempted\":true"), "not interrupted: {r}");
    // a longer preamble: the paused run is abandoned, and a run from scratch
    // takes more restart points before S₀ than the old engine had
    std::fs::write(reference.join("doc.tex"), doc(400, "alpha")).unwrap();
    std::fs::write(dir.join("doc.tex"), doc(400, "alpha")).unwrap();
    let r = h.cmd("compile");
    assert!(r.contains("\"mode\":\"cold\""), "not from scratch: {r}");
    check_against(&e, &dir, &reference, &r, "the longer preamble");
    for w in ["gamma", "delta", "alpha"] {
        let what = format!("then {w}");
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(400, w))], &what);
        assert!(r.contains("\"mode\":\"incremental\""), "{what}: {r}");
    }
}

/// BEAMER-V3: `scan_action` returns from a `user` action before it sets
/// the action's named flag, identifier, new-window flag and structure
/// identifier, so those keep what the node's memory held before; every
/// reader tests for `user` first. Beamer's navigation symbols are such
/// links (`/S/Named`), and their actions are still live at the next page
/// boundary; after an edit that moved the allocation they held other
/// leftovers than the old run's. The convergence test compared them, so a
/// deck re-typeset to its end after any such edit. The run converges now,
/// and every compile equals scratch runs.
#[test]
fn beamer_navigation_actions_converge() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("useraction");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // beamer's default theme: every slide has the navigation symbols, whose
    // links are `user` actions (`/S/Named`), and no shading
    let doc = |word: &str| -> String {
        let mut s = String::from("\\documentclass{beamer}\n\\begin{document}\n");
        for k in 0..12 {
            let w = if k == 1 { word } else { "omega" };
            s.push_str(&format!(
                "\\begin{{frame}}{{Frame {k}}}\nFrame {k} with the word {w}, and a sentence \
                 that wraps onto a second line of the slide.\n\\end{{frame}}\n"
            ));
        }
        s.push_str("\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    for k in 0..4 {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc("omega"))], "settle");
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
    for (word, what) in [
        ("ome ga", "a space in frame 1"),
        ("omega", "the revert"),
        ("omegb", "a letter replaced"),
    ] {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(word))], what);
        assert!(r.contains("\"mode\":\"incremental\""), "{what}: {r}");
        assert_ne!(field(&r, "converged_at"), "null", "{what}: {r}");
    }
}

/// BEAMER-V3 review (negative case for #1446): a form made before the edited
/// word's page and shipped pages later, whose attributes hold the edited
/// word. Its `obj_xform_attr` token list differs between the runs until the
/// form is shipped (`delete_toks` frees and nulls it then), so the run may
/// not converge before the page that ships it, and every compile must equal
/// scratch runs.
#[test]
fn an_unwritten_form_whose_attributes_change_does_not_converge_early() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("xform-attr");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // one paragraph per page; the form is made on page 1 and shipped with
    // page 8, the first page that refers to it
    const SHIPS_WITH: usize = 8;
    let doc = |word: &str| -> String {
        let mut s = String::from("\\documentclass{article}\n\\begin{document}\n");
        s.push_str(&para(0, "omega"));
        s.push_str(&format!(
            "\\setbox0\\hbox{{\\vrule width 2cm height 1mm}}%\n\
             \\pdfxform attr{{/FlashTeXWord ({word})}} 0\n\
             \\xdef\\late{{\\the\\pdflastxform}}\n\\clearpage\n"
        ));
        for k in 2..=14 {
            s.push_str(&para(k, "omega"));
            if k == SHIPS_WITH {
                s.push_str("\\noindent\\pdfrefxform\\late\\par\n");
            }
            s.push_str("\\clearpage\n");
        }
        s.push_str("\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    for k in 0..4 {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc("alpha"))], "settle");
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
    for (word, what) in [
        ("alphb", "the form's attribute"),
        ("alpha", "the revert"),
        ("alphc", "the attribute again"),
    ] {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(word))], what);
        assert!(r.contains("\"mode\":\"incremental\""), "{what}: {r}");
        let at = field(&r, "converged_at");
        if at != "null" {
            let at: usize = at.parse().unwrap();
            assert!(
                at >= SHIPS_WITH,
                "{what}: converged after page {at}, before the form shipped: {r}"
            );
        }
        // For the right reason: what kept each checkpoint before the form
        // shipped from converging is the attribute's tokens (the walk of
        // the form's `attr` chain), not a word the walk left uncompared.
        let before = diffs_before(&r, SHIPS_WITH);
        assert!(
            !before.is_empty(),
            "{what}: no checkpoint before the form shipped was tested: {r}"
        );
        for (p, why) in &before {
            assert!(
                why.contains("token differs") && why.contains("Chain"),
                "{what}: page {p} did not converge for another reason than the attribute ({why}): {r}"
            );
        }
    }
}

/// BEAMER-V3 review (negative case for #1448): a `user` link whose URI
/// holds the edited word, broken across a page: the action stays live on
/// `pdf_link_stack` past the page boundary, and the next page writes an
/// annotation with the same action again. The edit is a macro on page 1
/// that the link expands on page 2 and that is gone afterwards, so at the
/// checkpoint after page 2 the action's tokens are the only difference.
/// Comparing a `user` action's tokens must keep the run from converging
/// there (`converged_at` 2 would ship page 3's annotation with the old
/// URI); it converges once the link has ended, and every compile equals
/// scratch runs.
#[test]
fn a_user_link_whose_uri_changes_does_not_converge_early() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("useraction-uri");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |word: &str| -> String {
        // without object streams, and with more resources than the PDF
        // writer's 16 KiB buffer after each page's annotations (pdftex.web
        // writes the resources dictionary last), so that no buffer still
        // holds the URI at a page's end
        let mut s = String::from("\\documentclass{article}\n\\pdfobjcompresslevel=0\n");
        s.push_str(&format!(
            "\\pdfpageresources{{/FlashTeXPad ({})}}\n\\begin{{document}}\n",
            "x".repeat(20_000)
        ));
        s.push_str(&format!("\\gdef\\uri{{https://example.invalid/{word}}}\n"));
        s.push_str(&para(0, "omega"));
        s.push_str("\\clearpage\n");
        for k in 1..4 {
            s.push_str(&para(k, "omega"));
        }
        s.push_str(
            "\\noindent\\pdfstartlink user{/Subtype/Link/A<</S/URI/URI(\\uri)>>}%\n\
             \\global\\let\\uri\\relax\n",
        );
        // about 75 lines: from page 2 into page 3
        for i in 0..75 {
            s.push_str(&format!(
                "Line {i} of the linked paragraph, long enough to fill a line. "
            ));
        }
        s.push_str("\\pdfendlink\\par\n\n");
        for k in 4..120 {
            s.push_str(&para(k, "omega"));
        }
        s.push_str("\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    for k in 0..4 {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc("alpha"))], "settle");
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
    let mut converged = 0;
    for (word, what) in [
        ("alphb", "the URI"),
        ("alpha", "the revert"),
        ("alphc", "the URI again"),
    ] {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(word))], what);
        assert!(r.contains("\"mode\":\"incremental\""), "{what}: {r}");
        let at = field(&r, "converged_at");
        if at != "null" {
            let at: usize = at.parse().unwrap();
            assert!(
                at >= 3,
                "{what}: converged after page {at}, inside the link: {r}"
            );
            converged += 1;
        }
        // For the right reason: page 2's checkpoint differs in the action's
        // tokens (the walk of the `user` action), not in a word the walk
        // left uncompared.
        let before = diffs_before(&r, 3);
        assert!(
            before.iter().any(|(p, _)| *p == 2),
            "{what}: page 2's checkpoint was not tested: {r}"
        );
        for (p, why) in &before {
            assert!(
                why.contains("token differs") && why.contains("Tok"),
                "{what}: page {p} did not converge for another reason than the URI ({why}): {r}"
            );
        }
    }
    // the case reached the test past the link (else it proves nothing)
    assert!(converged >= 2, "only {converged} of 3 edits converged");
}

/// BEAMER-V3 review: a document whose first (cold) compile stops with an
/// error (`\pdfstartlink` in vertical mode) leaves the same log, aux and
/// (no) PDF as a scratch run.
#[test]
fn an_erroring_cold_compile_equals_scratch_runs() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("cold-error");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // pages shipped before the error, with hyperref (its .aux and .out)
    let doc = |preamble: &str, link: &str| -> String {
        let mut s = format!(
            "\\documentclass{{article}}\n\\usepackage{{hyperref}}\n{preamble}\\begin{{document}}\n"
        );
        for k in 0..40 {
            s.push_str(&para(k, "omega"));
            if k == 20 {
                s.push_str(link);
            }
        }
        s.push_str("\\end{document}\n");
        s
    };
    let bad = "\\pdfstartlink user{/S/URI/URI(x)}\\pdfendlink\n";
    let mut h = Host::start(&e, &dir);
    compile_and_check(
        &e,
        &mut h,
        &dir,
        &[("doc.tex", &doc("", bad))],
        "the first compile",
    );
    compile_and_check(&e, &mut h, &dir, &[], "again");
    for k in 0..3 {
        compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("doc.tex", &doc("", ""))],
            &format!("fixed {k}"),
        );
    }
    // a preamble edit and the error together: a cold compile that fails
    let r = compile_and_check(
        &e,
        &mut h,
        &dir,
        &[("doc.tex", &doc("\\relax\n", bad))],
        "cold and failing",
    );
    assert!(r.contains("\"mode\":\"cold\""), "not a cold compile: {r}");
    let log = std::fs::read_to_string(dir.join("doc.log")).unwrap();
    assert!(
        log.contains("\\pdfstartlink cannot be used in vertical mode"),
        "{log}"
    );
    compile_and_check(
        &e,
        &mut h,
        &dir,
        &[("doc.tex", &doc("\\relax\n", ""))],
        "fixed again",
    );
}

/// P6-HYPEROPT: `link(temp_head)` is left pointing at whatever list it
/// last held (a paragraph's line, inline math's translated hlist), and
/// nothing reads it before writing it again, so the convergence test does
/// not follow it (`crate::iso`, `roots`). It used to: after a one-letter
/// edit the old and new runs had different nodes there, page after page
/// (the edit moved where later nodes were allocated), so a keystroke in
/// the middle of this document (tools/incr-bench's plain-N kind: amsmath,
/// inline math in every paragraph, a display every sixth) re-typeset many
/// pages instead of converging on the page after the edited one. Every
/// compile equals scratch runs.
#[test]
fn a_stale_temp_head_does_not_block_convergence() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("temp-head");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    const WORDS: [&str; 30] = [
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
    ];
    let doc = |edit: &str| -> String {
        let mut s = String::from(
            "\\documentclass[11pt]{article}\n\\usepackage[margin=1in]{geometry}\n\
             \\usepackage{amsmath}\n\\begin{document}\n",
        );
        for k in 0..150usize {
            let w: Vec<&str> = (0..90)
                .map(|j| WORDS[(k * 7 + j * j * 3 + j) % 30])
                .collect();
            let (a, b) = w.split_at(45);
            let first = if k == 60 { edit } else { "" };
            s.push_str(&format!(
                "Text{first} {} with $x_{{{}}}^2+\\frac{{a}}{{b}}=\\sum_{{i=1}}^n c_i$ {}.\n\n",
                a.join(" "),
                k % 17,
                b.join(" ")
            ));
            if k % 6 == 5 {
                s.push_str(&format!(
                    "\\begin{{equation}}\\int_0^\\infty e^{{-x^2}}\\,dx=\
                     \\frac{{\\sqrt\\pi}}{{2}}+{k}\\end{{equation}}\n\n"
                ));
            }
        }
        s.push_str("\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    for k in 0..4 {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(""))], "settle");
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
    // (letters the text uses elsewhere: a glyph used nowhere else changes
    // `pdf_char_used`, which a removal never converges past)
    for (edit, what) in [("x", "a letter"), ("", "the revert"), ("xa", "two letters")] {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(edit))], what);
        assert!(r.contains("\"mode\":\"incremental\""), "{what}: {r}");
        let restart: usize = field(&r, "restart_pages").parse().unwrap();
        let conv: usize = field(&r, "converged_at")
            .parse()
            .unwrap_or_else(|_| panic!("{what}: no convergence: {r}"));
        assert!(
            conv <= restart + 4,
            "{what}: converged after page {conv}, restarted after {restart}: {r}"
        );
    }
}

/// #1502: a lookup the old run makes after the convergence point and whose
/// answer is different now (`\IfFileExists` of a file that appeared with
/// the edit) must not be skipped. The restart point is the edit (before
/// the lookup), and the convergence test did not look at the old run's
/// later lookups: the run converged on the page after the edit and kept
/// the old run's "MISSING FILE" page (found by the review of #1495-#1498).
/// Every compile equals scratch runs.
#[test]
fn a_later_lookup_whose_answer_changed_blocks_convergence() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("later-lookup");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |w: &str| -> String {
        let mut s = String::from("\\documentclass{article}\n\\begin{document}\n");
        for i in 0..120 {
            s.push_str(&para(i, if i == 10 { w } else { "lorem" }));
            if i == 80 {
                s.push_str(
                    "\\IfFileExists{extra-probe.tex}{\\input{extra-probe.tex}}{MISSING FILE}\n\n",
                );
            }
        }
        s.push_str("\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    for k in 0..4 {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc("lorem"))], "settle");
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
    // (the same letters: the fonts' used characters stay the same)
    compile_and_check(
        &e,
        &mut h,
        &dir,
        &[
            ("doc.tex", &doc("lorme")),
            ("extra-probe.tex", "THE EXTRA FILE IS HERE.\n"),
        ],
        "an edit and a new file a later page looks for",
    );
    std::fs::remove_file(dir.join("extra-probe.tex")).unwrap();
    compile_and_check(
        &e,
        &mut h,
        &dir,
        &[("doc.tex", &doc("lorem"))],
        "the revert and the file gone",
    );
}

/// #1502 (low, probed: not a bug, kept as a guard). After a convergence
/// the journal is the new run's reads up to the convergence point, then the
/// old run's after it, and the re-run of `\end{document}` cuts it at its
/// restart point's counts (`truncate_journal(&jn, rec_last.reads)`). Those
/// counts are in the spliced numbering: `redo_to_remapped` shifts every
/// kept record by the journal's length now minus the old run's at the
/// convergence point. Here the edit adds five whole-file reads (each in a
/// group, so it leaves no state) before the convergence point, the new run
/// has 14 journal entries more there, and the run converges; `tail.tex`,
/// read once pages later, stays in the journal, so an edit of it alone is
/// seen. Every compile equals scratch runs.
#[test]
fn a_spliced_journal_keeps_the_files_read_before_the_end() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("spliced-journal");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |extra: bool| -> String {
        let mut s = String::from("\\documentclass{article}\n\\begin{document}\n");
        for i in 0..200 {
            let p = para(i, "lorem");
            if i == 10 && extra {
                // on the paragraph's own line: no input line moves
                s.push_str(p.trim_end());
                s.push_str(
                    &" \\begingroup\\toks0=\\expandafter{\\pdfmdfivesum file{doc.tex}}\\endgroup"
                        .repeat(5),
                );
                s.push_str("\n\n");
            } else {
                s.push_str(&p);
            }
            if i == 150 {
                // the primitive: one open of the file (LaTeX's `\\input`
                // tests for it first, a second journal entry)
                s.push_str("\\csname @@input\\endcsname tail.tex\n\n");
            }
        }
        s.push_str("\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    for k in 0..4 {
        let r = compile_and_check(
            &e,
            &mut h,
            &dir,
            &[
                ("doc.tex", &doc(false)),
                ("tail.tex", "The tail, first version.\n"),
            ],
            "settle",
        );
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
    let r = compile_and_check(
        &e,
        &mut h,
        &dir,
        &[("doc.tex", &doc(true))],
        "more files read before the convergence point",
    );
    assert_ne!(field(&r, "converged_at"), "null", "{r}");
    compile_and_check(
        &e,
        &mut h,
        &dir,
        &[("tail.tex", "The tail, second version.\n")],
        "an edit of the last page's file alone",
    );
}

/// #1502 (review of #1507): the same lookup made twice, early and pages
/// later. The journal kept a lookup only at its first occurrence, so the
/// convergence test saw no changed lookup after the early one and kept the
/// old run's later page, which had looked the file up when it was missing.
/// For every kind of lookup a page can make -- `\pdffilesize`,
/// `\pdffilemoddate`, `\pdfmdfivesum file`, `\IfFileExists` with `\input`,
/// `\openin` -- the file appears with an edit before both lookups, goes
/// again with the revert, and appears once more with no edit: every compile
/// equals scratch runs.
#[test]
fn a_repeated_lookup_whose_answer_changed_blocks_convergence() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    const F: &str = "extra-probe.tex";
    let kinds: [(&str, &str, &str); 5] = [
        (
            "size",
            "\\begingroup\\edef\\x{\\pdffilesize{extra-probe.tex}}\\endgroup",
            "[size \\pdffilesize{extra-probe.tex}]",
        ),
        (
            "mtime",
            "\\begingroup\\edef\\x{\\pdffilemoddate{extra-probe.tex}}\\endgroup",
            "[date \\pdffilemoddate{extra-probe.tex}]",
        ),
        (
            "md5",
            "\\begingroup\\edef\\x{\\pdfmdfivesum file{extra-probe.tex}}\\endgroup",
            "[md5 \\pdfmdfivesum file{extra-probe.tex}]",
        ),
        (
            "iffileexists",
            "\\IfFileExists{extra-probe.tex}{}{}",
            "\\IfFileExists{extra-probe.tex}{\\input{extra-probe.tex}}{MISSING FILE}",
        ),
        (
            "openin",
            "\\openin15=extra-probe.tex \\ifeof15 \\else\\closein15 \\fi",
            "\\openin15=extra-probe.tex \\ifeof15 NO FILE\\else THERE\\closein15 \\fi",
        ),
    ];
    for (kind, early, late) in kinds {
        let dir = e.dir.join(format!("repeated-lookup-{kind}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let doc = |w: &str| -> String {
            let mut s = String::from("\\documentclass{article}\n\\begin{document}\n");
            for i in 0..120 {
                s.push_str(&para(i, if i == 10 { w } else { "lorem" }));
                if i == 5 {
                    s.push_str(early);
                    s.push_str("\n\n");
                }
                if i == 80 {
                    s.push_str(late);
                    s.push_str("\n\n");
                }
            }
            s.push_str("\\end{document}\n");
            s
        };
        let mut h = Host::start(&e, &dir);
        for k in 0..4 {
            let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc("lorem"))], "settle");
            if r.contains("\"mode\":\"unchanged\"") || k == 3 {
                break;
            }
        }
        // (the same letters: the fonts' used characters stay the same)
        let what = format!("{kind}: an edit and the file appearing");
        compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("doc.tex", &doc("lorme")), (F, "THE EXTRA FILE IS HERE.\n")],
            &what,
        );
        std::fs::remove_file(dir.join(F)).unwrap();
        let what = format!("{kind}: the revert and the file gone");
        compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc("lorem"))], &what);
        let what = format!("{kind}: the file appearing, no edit");
        compile_and_check(&e, &mut h, &dir, &[(F, "THE EXTRA FILE IS HERE.\n")], &what);
    }
}

/// #1514 (the re-review of #1507): the same as
/// `a_later_lookup_whose_answer_changed_blocks_convergence`, but the file
/// appears while the edit's compile is preempted. The next compile
/// continues the stopped run (nothing it has read changed), which had
/// checked the old run's lookups when the file was still missing: it
/// converged on the page after the edit and kept the old run's "MISSING
/// FILE" page, and the compile after it said `unchanged`. For a lookup
/// by `\IfFileExists` and by `\pdffilesize`, and then for the file going
/// again with the revert, every compile equals scratch runs.
#[test]
fn a_lookup_whose_answer_changed_during_a_preempted_run_blocks_convergence() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    const F: &str = "extra-probe.tex";
    let kinds: [(&str, &str); 2] = [
        (
            "iffileexists",
            "\\IfFileExists{extra-probe.tex}{\\input{extra-probe.tex}}{MISSING FILE}",
        ),
        ("size", "[size \\pdffilesize{extra-probe.tex}]"),
    ];
    for (kind, late) in kinds {
        let dir = e.dir.join(format!("preempted-lookup-{kind}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let doc = |w: &str| -> String {
            let mut s = String::from("\\documentclass{article}\n\\begin{document}\n");
            for i in 0..200 {
                s.push_str(&para(i, if i == 10 { w } else { "lorem" }));
                if i == 160 {
                    s.push_str(late);
                    s.push_str("\n\n");
                }
            }
            s.push_str("\\end{document}\n");
            s
        };
        let mut h = Host::start(&e, &dir);
        for k in 0..4 {
            let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc("lorem"))], "settle");
            if r.contains("\"mode\":\"unchanged\"") || k == 3 {
                break;
            }
        }
        // A control: a file nothing looks up appears while the compile is
        // stopped. The continued run still converges (the `.aux` it is
        // rewriting itself is not taken as changed).
        std::fs::write(dir.join("doc.tex"), doc("lorme")).unwrap();
        let r = h.cmd("compile-interrupt 1 2");
        assert!(r.contains("\"preempted\":true"), "{kind}: not stopped: {r}");
        std::fs::write(dir.join("unrelated.txt"), "nobody reads this\n").unwrap();
        let reference = dir.with_extension("ref");
        copy_dir(&dir, &reference);
        let r2 = h.cmd("compile");
        check_against(
            &e,
            &dir,
            &reference,
            &r2,
            &format!("{kind}: an unrelated file"),
        );
        assert_eq!(field(&r2, "mode"), "\"continued\"", "{kind}: {r2}");
        assert_ne!(field(&r2, "converged_at"), "null", "{kind}: {r2}");
        std::fs::remove_file(dir.join("unrelated.txt")).unwrap();
        compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("doc.tex", &doc("lorem"))],
            "the revert",
        );
        for (step, (word, file)) in [("lorme", true), ("lorem", false)].into_iter().enumerate() {
            // the edit's compile is preempted after two pages, then the
            // file appears (or goes)
            std::fs::write(dir.join("doc.tex"), doc(word)).unwrap();
            let r = h.cmd("compile-interrupt 1 2");
            assert!(
                r.contains("\"preempted\":true"),
                "{kind} {step}: not stopped: {r}"
            );
            if file {
                std::fs::write(dir.join(F), "THE EXTRA FILE IS HERE.\n").unwrap();
            } else {
                std::fs::remove_file(dir.join(F)).unwrap();
            }
            let reference = dir.with_extension("ref");
            copy_dir(&dir, &reference);
            let r2 = h.cmd("compile");
            eprintln!("{kind} {step}: {}", &r2[..r2.len().min(300)]);
            let what = format!("{kind} {step}: the file changed during a preempted compile");
            check_against(&e, &dir, &reference, &r2, &what);
            // and the next compile, with nothing changed, keeps it
            copy_dir(&dir, &reference);
            let r3 = h.cmd("compile");
            let what = format!("{kind} {step}: the compile after it");
            check_against(&e, &dir, &reference, &r3, &what);
        }
    }
}

/// P6-HYPEROPT: DESIGN.md §5.3's barriers "block reuse past the point where
/// they are read". A document that writes a file through its body and reads
/// it back near its end, after a `\write18` (imakeidx's `\index` entries,
/// makeindex at `\printindex`, then its `.ind`: the 592-page *Infinite
/// Descent*, docs/evidence/infdesc-2026-10-03), never converged after an
/// edit, because the old run read both later: every keystroke re-typeset
/// the book from the edit to its end. Now the run converges, keeps the old
/// run's pages up to the last page checkpoint before the first of them,
/// and runs on live from there, which re-does both. Every compile equals
/// scratch runs.
#[test]
fn a_late_barrier_keeps_the_pages_before_it() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("late-barrier");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |word: &str| -> String {
        let mut s = String::from(
            "\\documentclass{article}\n\\newwrite\\idx\n\\begin{document}\n\
             \\immediate\\openout\\idx=\\jobname.idx\n",
        );
        for i in 0..150 {
            s.push_str(&para(i, if i == 10 { word } else { "lorem" }));
            if i % 10 == 0 {
                s.push_str(&format!("\\immediate\\write\\idx{{Entry {i}.}}\n"));
            }
        }
        s.push_str(
            "\\immediate\\closeout\\idx\n\
             \\immediate\\write18{kpsewhich no-such-file.xyz}\n\
             \\clearpage\\input{\\jobname.idx}\n\\end{document}\n",
        );
        s
    };
    let mut h = Host::start(&e, &dir);
    for k in 0..4 {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc("lorem"))], "settle");
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
    // (the same letters: the fonts' used characters stay the same)
    for (word, what) in [("lorme", "an edit on page 2"), ("lorem", "the revert")] {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(word))], what);
        assert!(r.contains("\"mode\":\"incremental\""), "{what}: {r}");
        let conv: usize = field(&r, "converged_at")
            .parse()
            .unwrap_or_else(|_| panic!("{what}: no convergence before the barrier: {r}"));
        let kept: usize = field(&r, "rerun_from")
            .parse()
            .unwrap_or_else(|_| panic!("{what}: no live run from before the barrier: {r}"));
        let pages: usize = field(&r, "pages").parse().unwrap();
        // the pages up to the barrier are the old run's; the last ones (the
        // `\write18` and the read of the written file) were re-typeset
        assert!(conv < kept && kept < pages, "{what}: {r}");
    }
}

/// A file written through the body and read back at the end, opened at
/// `\begin{document}` (*Infinite Descent*'s hints and solutions: `\hint`s
/// written to `\jobname.hnt` in every chapter, `\input` in the appendix).
/// `word` is in paragraph 30, `hint` in the hint written at paragraph 60,
/// `tail` in paragraph 145, after the read.
fn hints_doc(word: &str, hint: &str, tail: &str) -> String {
    let mut s = String::from(
        "\\documentclass{article}\n\\newwrite\\hnt\n\
         \\AtBeginDocument{\\immediate\\openout\\hnt=\\jobname.hnt}\n\\begin{document}\n",
    );
    for i in 0..140 {
        s.push_str(&para(i, if i == 30 { word } else { "lorem" }));
        if i % 20 == 0 {
            let h = if i == 60 { hint } else { "lorem" };
            s.push_str(&format!("\\immediate\\write\\hnt{{Hint {i}: {h}.}}\n"));
        }
    }
    s.push_str("\\immediate\\closeout\\hnt\n\\clearpage\\input{\\jobname.hnt}\n\n");
    for i in 140..150 {
        s.push_str(&para(i, if i == 145 { tail } else { "lorem" }));
    }
    s.push_str("\\end{document}\n");
    s
}

/// A compile with page `stop` requested first (the viewport: the app's
/// request), continued (`finish`) when it stopped there, checked against
/// scratch runs: the report of each.
fn paused_then_finished(
    e: &Env,
    h: &mut Host,
    dir: &Path,
    text: &str,
    stop: usize,
    what: &str,
) -> (String, String) {
    std::fs::write(dir.join("doc.tex"), text).unwrap();
    let reference = dir.with_extension("ref");
    copy_dir(dir, &reference);
    let first = h.cmd(&format!("compile {stop}"));
    // (a run that converged before the page, or restarted after it, did
    // not stop)
    let r = if first.contains("\"paused\":true") {
        h.cmd("finish")
    } else {
        first.clone()
    };
    check_against(e, dir, &reference, &r, what);
    (first, r)
}

/// A run stopped at the requested page and continued compared the old
/// run's journal with the files again (#1514), and took a file it writes
/// itself and reads back later, opened before the restart (the hints of
/// *Infinite Descent*), for a changed input: on disk it held the stopped
/// run's part so far. The old run "read the changed file later", so no
/// test after the edited page could pass, and every keystroke re-typeset
/// the book to its end. Now such a file is the stopped run's own output,
/// and the old run's read of it is no change. Where the file holds the
/// same bytes in both runs at the convergence point, the old run's read of
/// it is kept too (`written_same`): the run converges and keeps the old
/// run's pages to the end (i). Where it does not (an edit inside a hint,
/// ii), the read is a barrier (a file one of the runs writes): the run
/// keeps the old run's pages before the read and runs on live from there.
/// Every compile equals scratch runs, when the edit leaves the hints alone
/// (i), changes what is written to them (ii), lies after the read (iii),
/// and when a second keystroke comes before the first compile is continued
/// (iv).
#[test]
fn a_file_written_and_read_back_converges_after_a_stopped_run() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("hints");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let base = hints_doc("lorem", "lorem", "lorem");
    let mut h = Host::start(&e, &dir);
    for k in 0..4 {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &base)], "settle");
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
    // (i) an edit that leaves the written file alone: converges, and keeps
    // the old run's pages to the end, the read of the file too, which holds
    // the same bytes in both runs (the same letters: the fonts' used
    // characters stay the same)
    for (word, what) in [
        ("lorme", "(i) an edit on page 2"),
        ("lorem", "(i) its revert"),
    ] {
        let doc = hints_doc(word, "lorem", "lorem");
        let (first, r) = paused_then_finished(&e, &mut h, &dir, &doc, 2, what);
        assert!(
            first.contains("\"paused\":true"),
            "{what}: not stopped: {first}"
        );
        let conv: usize = field(&r, "converged_at")
            .parse()
            .unwrap_or_else(|_| panic!("{what}: no convergence before the read: {r}"));
        assert!(
            conv <= 3 && field(&r, "rerun_from") == "null",
            "{what}: re-typeset from the read of the unchanged hints: {r}"
        );
    }
    // (ii) an edit of what is written: converges, keeps the old run's pages
    // up to the read, and typesets the pages from there again from the new
    // hints
    for (hint, what) in [
        ("lorme", "(ii) an edit of a hint"),
        ("lorem", "(ii) its revert"),
    ] {
        let doc = hints_doc("lorem", hint, "lorem");
        let (_, r) = paused_then_finished(&e, &mut h, &dir, &doc, 5, what);
        let conv: usize = field(&r, "converged_at")
            .parse()
            .unwrap_or_else(|_| panic!("{what}: no convergence before the read: {r}"));
        let kept: usize = field(&r, "rerun_from")
            .parse()
            .unwrap_or_else(|_| panic!("{what}: no live run from before the read: {r}"));
        let pages: usize = field(&r, "pages").parse().unwrap();
        assert!(conv < kept && kept < pages, "{what}: {r}");
    }
    // (iii) an edit after the read
    for (tail, what) in [
        ("lorme", "(iii) an edit after the read"),
        ("lorem", "(iii) its revert"),
    ] {
        let doc = hints_doc("lorem", "lorem", tail);
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc)], what);
        assert!(r.contains("\"mode\":\"incremental\""), "{what}: {r}");
    }
    // (iv) a second keystroke before the first compile is continued: the
    // stopped run is settled or abandoned, and the second compile, stopped
    // too, is continued
    std::fs::write(dir.join("doc.tex"), hints_doc("lorme", "lorem", "lorem")).unwrap();
    let first = h.cmd("compile 2");
    assert!(
        first.contains("\"paused\":true"),
        "(iv) not stopped: {first}"
    );
    for (doc, what) in [
        (
            hints_doc("loerm", "lorem", "lorem"),
            "(iv) a second keystroke",
        ),
        (base.clone(), "(iv) the revert"),
    ] {
        let (first, _) = paused_then_finished(&e, &mut h, &dir, &doc, 2, what);
        assert!(
            first.contains("\"paused\":true"),
            "{what}: not stopped: {first}"
        );
    }
}

/// DESIGN.md §5.3 rule (c), the line half (`crate::lineshift`): a document
/// of `n` paragraphs with `extra` text before some of them, and `pre` in
/// the preamble.
fn lines_doc(pre: &str, n: usize, extra: &[(usize, &str)]) -> String {
    let mut s = format!("\\documentclass{{article}}\n{pre}\\begin{{document}}\n");
    for i in 0..n {
        for (k, t) in extra {
            if *k == i {
                s.push_str(t);
                s.push_str("\n\n");
            }
        }
        s.push_str(&para(i, "lorem"));
    }
    s.push_str("\\end{document}\n");
    s
}

/// The edits of the line kinds at paragraph 5 (`tools/incr-bench/edits.py`'s
/// `newline`, `split`, `join`), each followed by its revert.
fn line_edits(doc: &str) -> Vec<(String, &'static str)> {
    let at = "Paragraph 5 with the word";
    assert!(doc.contains(at));
    let nl = doc.replacen(at, "Paragraph 5 with\nthe word", 1);
    let split = doc.replacen(at, "Paragraph 5 with\n\nthe word", 1);
    // two paragraphs joined: the break before paragraph 6 becomes a space
    let join = doc.replacen(".\n\nParagraph 6 ", ". Paragraph 6 ", 1);
    assert!(nl != *doc && split != *doc && join != *doc);
    vec![
        (nl, "a newline"),
        (doc.to_string(), "its revert"),
        (split, "a paragraph split"),
        (doc.to_string(), "its revert"),
        (join, "a paragraph join"),
        (doc.to_string(), "its revert"),
    ]
}

/// The 1-based line of the first occurrence of `what` in `doc`.
fn line_of(doc: &str, what: &str) -> usize {
    doc[..doc.find(what).unwrap()].matches('\n').count() + 1
}

fn settle(e: &Env, h: &mut Host, dir: &Path, doc: &str) {
    for k in 0..4 {
        let r = compile_and_check(e, h, dir, &[("doc.tex", doc)], "settle");
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
}

#[test]
fn line_edits_converge_and_equal_scratch_runs() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    // Plain prose, and LaTeX environments (`\begin` reads `\inputlineno`
    // into `\@currenvline`, a marked list): a newline before them converges
    // at the first page tested after the edited one.
    let envs: Vec<(usize, String)> = (10..120)
        .step_by(7)
        .map(|i| (i, format!("\\begin{{equation}}a_{i}=b\\end{{equation}}")))
        .collect();
    let envs: Vec<(usize, &str)> = envs.iter().map(|(i, s)| (*i, s.as_str())).collect();
    for (name, doc) in [
        ("lines-plain", lines_doc("", 120, &[])),
        ("lines-envs", lines_doc("", 120, &envs)),
    ] {
        let dir = e.dir.join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut h = Host::start(&e, &dir);
        settle(&e, &mut h, &dir, &doc);
        for (k, (text, what)) in line_edits(&doc).into_iter().enumerate() {
            let what = format!("{name}: {what}");
            let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &text)], &what);
            assert!(r.contains("\"mode\":\"incremental\""), "{what}: {r}");
            if k < 2 {
                assert_ne!(field(&r, "converged_at"), "null", "{what}: {r}");
                assert_eq!(field(&r, "rerun_from"), "null", "{what}: {r}");
            }
        }
    }
}

/// Each case reads or prints a line number pages after a line edit (about
/// page 9 of 12; the edit is on page 1): the old run's pages from there on
/// would show the old number. The newline converges, and the run goes on
/// live from before that page. Every compile equals a scratch run; each
/// case fails without its rule (`crate::lineshift`).
#[test]
fn moved_line_numbers_are_barriers() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    const N: usize = 240;
    const AT: usize = 180;
    // `\ifx` on and expansion of `\@currenvline` (a marked list): the
    // environment's line compared with the line it had before the edit
    let probe = "\\begin{center}\\expandafter\\ifx\\csname @currenvline\\endcsname\\expected \
                 SAME\\else DIFFERENT\\fi\\end{center}";
    let base = lines_doc("\\def\\expected{ on input line LINE}\n", N, &[(AT, probe)]);
    let l = line_of(&base, "\\begin{center}");
    let ifx = base.replace("LINE", &l.to_string());
    let typeset = lines_doc(
        "",
        N,
        &[(
            AT,
            "\\begin{center}\\csname @currenvline\\endcsname\\end{center}",
        )],
    );
    let cases: Vec<(&str, String)> = vec![
        // `\inputlineno` into a message (the log)
        (
            "lines-message",
            lines_doc("", N, &[(AT, "\\message{[line \\the\\inputlineno]}")]),
        ),
        // ... into a register in a group, typeset
        (
            "lines-count",
            lines_doc(
                "",
                N,
                &[(
                    AT,
                    "\\begingroup\\count255=\\inputlineno Line \\the\\count255.\\endgroup",
                )],
            ),
        ),
        // a box report's line (the log)
        (
            "lines-underfull",
            lines_doc("", N, &[(AT, "\\noindent\\hbox to 10cm{a b}")]),
        ),
        // the context of a `\show` (`l.<n>`, the log)
        ("lines-show", lines_doc("", N, &[(AT, "\\show\\par")])),
        // `\showgroups` in an open group (the log)
        (
            "lines-groups",
            lines_doc("", N, &[(AT, "\\begingroup\\showgroups\\endgroup")]),
        ),
        // `\showlists` (the modes' lines, the log)
        ("lines-lists", lines_doc("", N, &[(AT, "\\showlists")])),
        ("lines-ifx", ifx),
        ("lines-typeset", typeset),
    ];
    // (every case runs, and the failures are named together)
    let mut bad = vec![];
    for (name, doc) in &cases {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let dir = e.dir.join(name);
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let mut h = Host::start(&e, &dir);
            settle(&e, &mut h, &dir, doc);
            for (k, (text, what)) in line_edits(doc).into_iter().enumerate() {
                let what = format!("{name}: {what}");
                let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &text)], &what);
                if k == 0 {
                    // the old run's pages are kept up to the barrier
                    assert_ne!(field(&r, "converged_at"), "null", "{what}: {r}");
                    assert_ne!(field(&r, "rerun_from"), "null", "{what}: {r}");
                }
            }
        }));
        if r.is_err() {
            bad.push(*name);
        }
    }
    assert!(bad.is_empty(), "failed: {bad:?}");
}

/// After a line edit converges, later edits restart from the old run's
/// checkpoints. Those inside an environment open across pages keep a marked
/// `\@currenvline`: they are dropped, so that an edit ending the environment
/// with the wrong `\end` shows its true line. The others are corrected as
/// they are restored (`Reloc`): a `\message` of `\inputlineno` after the
/// restart point shows the new number.
#[test]
fn restores_after_a_line_edit_equal_scratch_runs() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let items: String = (0..40)
        .map(|i| format!("\\item {}", para(1000 + i, "ipsum")))
        .collect();
    let list = format!("\\begin{{itemize}}\n{items}\\end{{itemize}}");
    let doc = lines_doc(
        "",
        150,
        &[(60, &list), (140, "\\message{[line \\the\\inputlineno]}")],
    );
    let dir = e.dir.join("lines-restore");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = Host::start(&e, &dir);
    settle(&e, &mut h, &dir, &doc);
    let nl = doc.replacen("Paragraph 5 with the word", "Paragraph 5 with\nthe word", 1);
    let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &nl)], "a newline");
    assert_ne!(field(&r, "converged_at"), "null", "a newline: {r}");
    // the list's end made wrong: a restart inside the list; LaTeX's error
    // shows the list's line
    let wrong = nl.replacen("\\end{itemize}", "\\end{enumerate}", 1);
    assert_ne!(wrong, nl);
    compile_and_check(&e, &mut h, &dir, &[("doc.tex", &wrong)], "the wrong end");
    compile_and_check(&e, &mut h, &dir, &[("doc.tex", &nl)], "its revert");
    // a letter after the list and before the message: a restart from a
    // checkpoint the convergence kept
    let letter = nl.replacen(
        "Paragraph 130 with the word lorem",
        "Paragraph 130 with the word lorme",
        1,
    );
    assert_ne!(letter, nl);
    compile_and_check(&e, &mut h, &dir, &[("doc.tex", &letter)], "a letter later");
    compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc)], "all reverted");
}

/// The independent review of #1570 (probe 1): `\the\inputlineno` confined
/// into an `\edef` body, and the definition runs away (an `\outer` macro
/// read from another file). TeX's `runaway` prints the unfinished body, the
/// moved line's digits included, through `show_token_list(link(def_ref))`
/// before the definition is done and its list marked; the error's context
/// is the other file's line. The print is a read of the moved line.
#[test]
fn a_runaway_definition_prints_a_moved_line() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    const N: usize = 240;
    const AT: usize = 180;
    let doc = lines_doc(
        "\\outer\\def\\foo{}\n",
        N,
        &[
            (
                AT,
                "\\edef\\x{\\the\\inputlineno\\csname @@input\\endcsname rvsub ",
            ),
            (AT + 3, "\\let\\x\\relax"),
        ],
    );
    let dir = e.dir.join("lines-runaway");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("rvsub.tex"), "\\foo\n").unwrap();
    let mut h = Host::start(&e, &dir);
    settle(&e, &mut h, &dir, &doc);
    for (text, what) in line_edits(&doc).into_iter().take(2) {
        let what = format!("lines-runaway: {what}");
        compile_and_check(&e, &mut h, &dir, &[("doc.tex", &text)], &what);
    }
}

/// The review's probe 2: a group begun in an `\input` file that is closed
/// again while the group stays open, after a newline in that file. The old
/// run's later checkpoints hold the group's line in the old numbering;
/// correcting them must know the group is the edited file's although no
/// level reads it any more. e-TeX's end of job prints the line ("entered
/// at line N").
#[test]
fn a_group_line_of_a_closed_inclusion_moves() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("lines-closed-group");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut chap: String = (0..60).map(|i| para(1000 + i, "ipsum")).collect();
    chap.push_str("\\begingroup\n");
    let doc = lines_doc("", 120, &[(20, "\\input{chapx}")]);
    std::fs::write(dir.join("chapx.tex"), &chap).unwrap();
    let mut h = Host::start(&e, &dir);
    settle(&e, &mut h, &dir, &doc);
    let nl = chap.replacen(
        "Paragraph 1005 with the word",
        "Paragraph 1005 with\nthe word",
        1,
    );
    assert_ne!(nl, chap);
    compile_and_check(
        &e,
        &mut h,
        &dir,
        &[("chapx.tex", &nl)],
        "lines-closed-group: a newline in chapx.tex",
    );
    compile_and_check(
        &e,
        &mut h,
        &dir,
        &[("chapx.tex", &chap)],
        "lines-closed-group: its revert",
    );
}

/// The review's probe 3: a file `sub/doc.tex`, read by its absolute name,
/// is not the edited `doc.tex`. After a newline in `doc.tex` converges, a
/// read of `\inputlineno` is added in `sub/doc.tex`: the restart from a
/// checkpoint the convergence kept must not move that file's line.
#[test]
fn an_absolute_namesake_is_not_the_edited_file() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("lines-namesake");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    let abs = dir.canonicalize().unwrap().join("sub").join("doc.tex");
    let sub: String = (0..80).map(|i| para(1000 + i, "ipsum")).collect();
    std::fs::write(&abs, &sub).unwrap();
    let inc = format!("\\input{{{}}}", abs.display());
    let doc = lines_doc("", 150, &[(60, &inc)]);
    let mut h = Host::start(&e, &dir);
    settle(&e, &mut h, &dir, &doc);
    let nl = doc.replacen("Paragraph 5 with the word", "Paragraph 5 with\nthe word", 1);
    compile_and_check(
        &e,
        &mut h,
        &dir,
        &[("doc.tex", &nl)],
        "lines-namesake: a newline",
    );
    // a line read in the middle of sub/doc.tex, no line moved
    let sub2 = sub.replacen(
        "Paragraph 1050 with",
        "\\message{[sub line \\the\\inputlineno]}Paragraph 1050 with",
        1,
    );
    assert_ne!(sub2, sub);
    compile_and_check(
        &e,
        &mut h,
        &dir,
        &[("sub/doc.tex", &sub2)],
        "lines-namesake: a read in sub/doc.tex",
    );
}

/// `\immediate` writes of `\jobname-tmp.tex` (`Instance K says S.`), closed
/// at once: a temporary file written and read back (genvol.py's
/// vol-closed).
fn write_tmp(k: usize, s: &str) -> String {
    format!(
        "\\immediate\\openout\\tmp=\\jobname-tmp.tex\n\
         \\immediate\\write\\tmp{{Instance {k} says {s}.}}\n\\immediate\\closeout\\tmp\n\n"
    )
}

fn paras(from: usize, to: usize, word: &str) -> String {
    (from..to).map(|i| para(i, word)).collect()
}

/// #1348 (1): an edit adds a write of a temporary file that the document
/// wrote and closed before the edit's restart point and reads later, and
/// its compile is preempted after that write. The next compile abandons
/// it and gets the complete run back (`reattach_pending`), but no tail of
/// that run held the file (the run never opened it after the restart
/// point), so the disk kept the abandoned run's instance, and the record
/// of that open was dropped: a later restart between the document's own
/// write and its `\input` read the abandoned instance.
#[test]
fn an_abandoned_run_that_rewrote_a_closed_file_leaves_it_as_the_old_run() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("abandoned-rewrite");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |slot: &str, word: &str| {
        format!(
            "\\documentclass{{article}}\n\\newwrite\\tmp\n\\begin{{document}}\n{}{}\\clearpage\n\
             {}\\clearpage\n{slot}{}\\clearpage\n{}\\clearpage\n\\input{{\\jobname-tmp.tex}}\n\n{}\
             \\end{{document}}\n",
            paras(0, 3, "alpha"),
            write_tmp(0, "x"),
            paras(3, 6, "alpha"),
            paras(6, 9, "alpha"),
            paras(9, 12, word),
            paras(12, 15, "alpha"),
        )
    };
    let base = doc("", "alpha");
    let mut h = Host::start(&e, &dir);
    for k in 0..4 {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &base)], "settle");
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
    // The new write, on the page after the restart point: preempted once
    // that page is out.
    std::fs::write(dir.join("doc.tex"), doc(&write_tmp(1, "yy"), "alpha")).unwrap();
    let r = h.cmd("compile-interrupt 1 1");
    assert!(r.contains("\"preempted\":true"), "not interrupted: {r}");
    compile_and_check(&e, &mut h, &dir, &[("doc.tex", &base)], "the revert");
    // A restart after the document's write and before its read.
    compile_and_check(
        &e,
        &mut h,
        &dir,
        &[("doc.tex", &doc("", "gamma"))],
        "an edit between the write and the read",
    );
    compile_and_check(&e, &mut h, &dir, &[("doc.tex", &base)], "its revert");
}

/// #1348 (2): a temporary file the preamble writes and the body reads, then
/// writes again and reads, in a host that opened the document from a
/// stored S₀ (a host restart). The opens before S₀ were not stored, so
/// the new process did not know the preamble had written the file: a
/// restart at S₀ or a checkpoint before the body's write (`rewritten_since`)
/// read the body's instance.
#[test]
fn a_file_the_preamble_wrote_is_rewritten_after_a_host_restart() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("preamble-rewrite");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |word: &str| {
        format!(
            "\\documentclass{{article}}\n\\newwrite\\tmp\n{}\\begin{{document}}\n{}\\clearpage\n\
             {}\\clearpage\n\\input{{\\jobname-tmp.tex}}\n\n{}\\clearpage\n{}{}\
             \\input{{\\jobname-tmp.tex}}\n\n\\end{{document}}\n",
            write_tmp(0, "x"),
            paras(0, 3, "alpha"),
            paras(3, 6, word),
            paras(6, 9, "alpha"),
            write_tmp(1, "yy"),
            paras(9, 12, "alpha"),
        )
    };
    let base = doc("alpha");
    let s0 = dir.with_extension("s0");
    let _ = std::fs::remove_file(&s0);
    {
        let mut h = Host::start(&e, &dir);
        for k in 0..4 {
            let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &base)], "settle");
            if r.contains("\"mode\":\"unchanged\"") || k == 3 {
                break;
            }
        }
        let r = h.cmd(&format!("save {}", s0.display()));
        assert!(r.contains("\"saved\""), "{r}");
    }
    // A new process, from the stored S₀.
    let mut h = Host::start(&e, &dir);
    let reference = dir.with_extension("ref");
    copy_dir(&dir, &reference);
    let r = h.cmd(&format!("open {}", s0.display()));
    assert!(r.contains("\"mode\":\"open\""), "not opened from S0: {r}");
    check_against(&e, &dir, &reference, &r, "the open");
    for (word, what) in [
        ("gamma", "an edit before the first read"),
        ("alpha", "its revert"),
    ] {
        compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(word))], what);
    }
}

/// #1591: one file under two names (here `alias/chapx.tex`, through a link
/// `alias` to the project's own directory), each opening
/// leaving a group open to the end. The edit is one shift: counted once per
/// name, it moved the groups' lines twice, every later test failed, and the
/// convergence came at the document's end.
#[test]
fn a_file_under_two_names_shifts_once() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("lines-alias");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::os::unix::fs::symlink(".", dir.join("alias")).unwrap();
    let mut chap: String = (0..12).map(|i| para(1000 + i, "ipsum")).collect();
    chap.push_str("\\begingroup\n");
    let doc = lines_doc(
        "",
        150,
        &[(10, "\\input{chapx}"), (30, "\\input{alias/chapx}")],
    );
    std::fs::write(dir.join("chapx.tex"), &chap).unwrap();
    let mut h = Host::start(&e, &dir);
    settle(&e, &mut h, &dir, &doc);
    let nl = chap.replacen(
        "Paragraph 1005 with the word",
        "Paragraph 1005 with\nthe word",
        1,
    );
    assert_ne!(nl, chap);
    for (text, what) in [(&nl, "a newline in chapx.tex"), (&chap, "its revert")] {
        let what = format!("lines-alias: {what}");
        let r = compile_and_check(&e, &mut h, &dir, &[("chapx.tex", text)], &what);
        let pages: usize = field(&r, "pages").parse().unwrap();
        let conv: usize = field(&r, "converged_at")
            .parse()
            .unwrap_or_else(|_| panic!("{what}: no convergence: {r}"));
        assert!(conv + 3 < pages, "{what}: converged late: {r}");
    }
}

/// The #1595 review's probe: `alias` is a link to the project directory
/// while names are resolved, then becomes a real directory with its own
/// `chapx.tex`. One compile then edits both files with the same line delta:
/// a late newline in the file read first, an early one in the file read
/// second, which reads `\inputlineno` below the first file's edit. The two
/// files' shifts are both kept: a resolution remembered from before the
/// swap took them for one file, dropped the second's shift, and the read
/// was no barrier.
#[test]
fn an_alias_replaced_by_a_directory_is_another_file() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    for (first, second) in [("chapx", "alias/chapx"), ("alias/chapx", "chapx")] {
        let tag = if first == "chapx" { "a" } else { "b" };
        let dir = e.dir.join(format!("lines-alias-swap-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(dir.with_extension("ref"));
        std::fs::create_dir_all(&dir).unwrap();
        std::os::unix::fs::symlink(".", dir.join("alias")).unwrap();
        let mut chap = String::new();
        chap.push_str(&para(1000, "ipsum"));
        chap.push_str(&para(1001, "ipsum"));
        chap.push_str("\\input{other}\n");
        chap.push_str("\\message{[chap line \\the\\inputlineno]}\n");
        for i in 1002..1020 {
            chap.push_str(&para(i, "ipsum"));
        }
        let other: String = (2000..2040).map(|i| para(i, "dolor")).collect();
        std::fs::write(dir.join("chapx.tex"), &chap).unwrap();
        std::fs::write(dir.join("other.tex"), &other).unwrap();
        let doc = lines_doc(
            "",
            40,
            &[
                (10, &format!("\\input{{{first}}}")),
                (30, &format!("\\input{{{second}}}")),
            ],
        );
        let mut h = Host::start(&e, &dir);
        settle(&e, &mut h, &dir, &doc);
        // names resolved: a newline through both names, and its revert
        let nl = chap.replacen(
            "Paragraph 1015 with the word",
            "Paragraph 1015 with\nthe word",
            1,
        );
        for (text, what) in [(&nl, "a newline"), (&chap, "its revert")] {
            let what = format!("lines-alias-swap-{tag}: {what}");
            compile_and_check(&e, &mut h, &dir, &[("chapx.tex", text)], &what);
        }
        // the link becomes a directory with its own copy
        std::fs::remove_file(dir.join("alias")).unwrap();
        std::fs::create_dir_all(dir.join("alias")).unwrap();
        std::fs::write(dir.join("alias").join("chapx.tex"), &chap).unwrap();
        compile_and_check(
            &e,
            &mut h,
            &dir,
            &[],
            &format!("lines-alias-swap-{tag}: swap"),
        );
        // a late newline in the file read first, an early one in the second
        let late = chap.replacen(
            "Paragraph 1015 with the word",
            "Paragraph 1015 with\nthe word",
            1,
        );
        let early = chap.replacen(
            "Paragraph 1001 with the word",
            "Paragraph 1001 with\nthe word",
            1,
        );
        let file = |n: &str| format!("{n}.tex");
        compile_and_check(
            &e,
            &mut h,
            &dir,
            &[(&file(first), &late), (&file(second), &early)],
            &format!("lines-alias-swap-{tag}: two files, one delta"),
        );
    }
}

/// The second #1595 review's probe: two different files, byte for byte the
/// same, edited the same way in one compile. Their edits are one shift
/// (`Pending::same_edit`), which must keep both names: with the second
/// file's name dropped, its openings' lines were never shifted and the run
/// did not converge.
#[test]
fn twin_files_edited_alike_both_shift() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("lines-twins");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut chap: String = (0..12).map(|i| para(1000 + i, "ipsum")).collect();
    chap.push_str("\\begingroup\n");
    let doc = lines_doc("", 150, &[(10, "\\input{chapx}"), (30, "\\input{chapy}")]);
    std::fs::write(dir.join("chapx.tex"), &chap).unwrap();
    std::fs::write(dir.join("chapy.tex"), &chap).unwrap();
    let mut h = Host::start(&e, &dir);
    settle(&e, &mut h, &dir, &doc);
    let nl = chap.replacen(
        "Paragraph 1005 with the word",
        "Paragraph 1005 with\nthe word",
        1,
    );
    assert_ne!(nl, chap);
    for (text, what) in [(&nl, "a newline in both"), (&chap, "its revert")] {
        let what = format!("lines-twins: {what}");
        let r = compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("chapx.tex", text), ("chapy.tex", text)],
            &what,
        );
        let pages: usize = field(&r, "pages").parse().unwrap();
        let conv: usize = field(&r, "converged_at")
            .parse()
            .unwrap_or_else(|_| panic!("{what}: no convergence: {r}"));
        assert!(conv + 3 < pages, "{what}: converged late: {r}");
    }
}

/// Performance modes (lane PERF-MODES, `crate::profile`): switching the mode
/// between compiles, live, changes only which checkpoints are kept (the
/// budget, the dense window), never the output. A tiny pinned budget keeps
/// retention thinning at every compile, so the dense window of each mode
/// (4, 16, 512 pages) decides what survives; the edits land near and far
/// from the last cursor.
#[test]
fn edits_across_performance_mode_switches_equal_scratch_runs() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let dir = e.dir.join("modes");
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
    let body: String = (0..60).map(|i| para(i, "alpha")).collect();
    let mut h = Host::start_env(
        &e,
        &dir,
        &[("FLASHTEX_BUDGET", "65536"), ("FLASHTEX_TIMED_S", "0.0002")],
    );
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
    let modes = ["low-memory", "high-performance", "balanced", "low-memory"];
    let mut text = body.clone();
    for (k, (m, at)) in modes.iter().zip([55, 3, 30, 58]).enumerate() {
        let p = h.cmd(&format!("profile {m}"));
        assert!(p.contains(&format!("\"mode\":\"{m}\"")), "{p}");
        assert!(p.contains("\"budget\":65536"), "the pinned budget: {p}");
        let from = format!("Paragraph {at} with the word alpha");
        let to = format!(
            "Paragraph {at} with the word alph{}",
            (b'b' + k as u8) as char
        );
        text = text.replacen(&from, &to, 1);
        compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("body.tex", &text)],
            &format!("{m}: edit"),
        );
    }
    h.cmd("profile high-performance");
    compile_and_check(&e, &mut h, &dir, &[("body.tex", &body)], "revert");
    compile_and_check(&e, &mut h, &dir, &[], "settle again");
}
