//! External tools through the engine host (protocol 3.2, lane
//! P5-EXTERNAL-TOOLS; `src/host/external.rs`): with `"external_tools":
//! "auto"` the host runs bibtex and makeindex from the user's TeX Live when
//! latexmk would, after the compile's `DONE`, and compiles again by itself
//! (`"cause": "tools"`) until the bibliography and index are current; with
//! `"off"` it runs nothing and says what it would have run.
//!
//! Each case checks the files the tools made against the same programs
//! run by hand as latexmk runs them (`bibtex main` in the output directory,
//! `makeindex -o main.ind main.idx`), and that the resident engine's final
//! `.aux` resolves every citation. The corpus-level P-T2 comparison with
//! `latexmk -pdf` is `tools/external-tools/xtools.py`.
//!
//! Skips where there is no TeX Live with bibtex and makeindex (e.g. CI).
#![cfg(feature = "kpathsea")]

use flashtex_display_list::client::{Client, CompileRequest, Event};
use flashtex_display_list::json::Json;
use flashtex_engine::resolver::find_texlive_bin;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

struct Host(Child, PathBuf);

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
        let _ = std::fs::remove_file(&self.1);
    }
}

fn pool() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool")
}

fn tex_bin() -> Option<PathBuf> {
    let b = find_texlive_bin()?;
    (b.join("bibtex").is_file() && b.join("makeindex").is_file()).then_some(b)
}

/// pdflatex.fmt made by this engine, once per test binary.
fn fmt_dir() -> PathBuf {
    static MADE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _once = MADE.lock().unwrap_or_else(|p| p.into_inner());
    let fmt = std::env::temp_dir().join(format!("flashtex-tools-fmt-{}", std::process::id()));
    if fmt.join("pdflatex.fmt").is_file() {
        return fmt;
    }
    std::fs::create_dir_all(&fmt).unwrap();
    let st = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
        .args([
            "-ini",
            "-jobname=pdflatex",
            "-progname=pdflatex",
            "-etex",
            "-translate-file=cp227.tcx",
            "pdflatex.ini",
        ])
        .current_dir(&fmt)
        .env("FLASHTEX_POOL", pool())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(st.success(), "building pdflatex.fmt failed");
    fmt
}

fn start_host(name: &str, extra: &[&str]) -> Host {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let sock = PathBuf::from(format!("/tmp/ftt-{}-{n}-{name}.sock", std::process::id()));
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-host"))
        .args(["--socket", sock.to_str().unwrap(), "--no-warm"])
        .args(extra)
        .env("FLASHTEX_POOL", pool())
        .env("FLASHTEX_FORMATS", fmt_dir())
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env_remove("FLASHTEX_S0_CACHE")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut out = BufReader::new(child.stdout.take().unwrap());
    loop {
        let mut line = String::new();
        if out.read_line(&mut line).unwrap() == 0 {
            panic!("the host exited before listening");
        }
        if line.contains("listening") {
            break;
        }
    }
    std::thread::spawn(move || for _ in out.lines() {});
    Host(child, sock)
}

/// What one compile cycle (the compile and its follow-ups) said.
#[derive(Default, Debug)]
struct Cycle {
    dones: Vec<Json>,
    tools: Vec<Json>,
    diagnostics: Vec<Json>,
    /// `diag-v1` (a client that accepted it).
    diags: Vec<flashtex_display_list::diag::Diag>,
}

impl Cycle {
    fn events(&self, event: &str) -> Vec<&Json> {
        self.tools
            .iter()
            .filter(|t| t.str_field("event") == Some(event))
            .collect()
    }
}

/// Compile and read until the cycle is `settled`.
fn cycle(c: &mut Client, req: &CompileRequest) -> Cycle {
    c.compile(req).unwrap();
    let mut cy = Cycle::default();
    loop {
        match c.next_event().unwrap().expect("host closed the connection") {
            Event::Done(d) => cy.dones.push(d),
            Event::Diagnostic(d) => cy.diagnostics.push(d),
            Event::Diag(d) => cy.diags.push(d),
            Event::Tool(t) => {
                let settled =
                    t.str_field("event") == Some("settled") && t.int_field("id") == Some(req.id);
                cy.tools.push(t);
                if settled {
                    return cy;
                }
            }
            Event::Error(e) => panic!("host error: {e}"),
            _ => {}
        }
    }
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("flashtex-tools-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("out")).unwrap();
    d
}

fn req(id: i64, root: &Path, tools: &str) -> CompileRequest {
    let mut r = CompileRequest::new(id, root.to_str().unwrap(), "main.tex");
    r.output_dir = Some(root.join("out").to_str().unwrap().into());
    r.incremental = true;
    r.external_tools = Some(tools.into());
    r
}

const BIB_DOC: &str = r"\documentclass{article}
\begin{document}
Knuth wrote \cite{knuth84}; Lamport~\cite{lamport94} followed.
\bibliographystyle{plain}
\bibliography{refs}
\end{document}
";

const BIB: &str = r"@book{knuth84, author = {Donald E. Knuth}, title = {The {\TeX}book},
  publisher = {Addison-Wesley}, year = {1984}}
@book{lamport94, author = {Leslie Lamport}, title = {{\LaTeX}: A Document Preparation System},
  publisher = {Addison-Wesley}, year = {1994}}
@article{liang83, author = {Franklin M. Liang}, title = {Word Hy-phen-a-tion by Com-put-er},
  journal = {Stanford}, year = {1983}}
";

/// `bibtex main` as latexmk runs it: in the output directory, with the
/// project first on BIBINPUTS/BSTINPUTS. Returns the `.bbl`.
fn bibtex_by_hand(bin: &Path, root: &Path) -> Vec<u8> {
    let d = std::env::temp_dir().join(format!("flashtex-tools-hand-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::copy(root.join("out/main.aux"), d.join("main.aux")).unwrap();
    let st = Command::new(bin.join("bibtex"))
        .arg("main")
        .current_dir(&d)
        .env("BIBINPUTS", format!("{}:", root.display()))
        .env("BSTINPUTS", format!("{}:", root.display()))
        .stdout(Stdio::null())
        .status()
        .unwrap();
    assert!(st.code().is_some());
    let bbl = std::fs::read(d.join("main.bbl")).unwrap();
    let _ = std::fs::remove_dir_all(&d);
    bbl
}

#[test]
fn bibtex_runs_and_the_host_compiles_again() {
    let Some(bin) = tex_bin() else {
        eprintln!("skipped: no TeX Live with bibtex");
        return;
    };
    let host = start_host("bib", &[]);
    let d = scratch("bib");
    std::fs::write(d.join("main.tex"), BIB_DOC).unwrap();
    std::fs::write(d.join("refs.bib"), BIB).unwrap();
    let mut c = Client::connect(&host.1).unwrap();
    let cy = cycle(&mut c, &req(1, &d, "auto"));
    let runs = cy.events("run");
    assert_eq!(runs.len(), 1, "{cy:?}");
    assert_eq!(runs[0].str_field("tool"), Some("bibtex"));
    assert_eq!(runs[0].str_field("file"), Some("main.aux"));
    let done = cy.events("done");
    assert_eq!(done[0].str_field("status"), Some("ok"), "{done:?}");
    assert_eq!(done[0].get("changed").and_then(Json::as_bool), Some(true));
    // The client's compile, then the host's own.
    assert!(cy.dones.len() >= 2, "{cy:?}");
    assert_eq!(cy.dones[0].str_field("cause"), None);
    assert_eq!(cy.dones[1].str_field("cause"), Some("tools"));
    let settled = cy.events("settled");
    assert_eq!(settled[0].get("ran").and_then(Json::as_bool), Some(true));
    // The .bbl is bibtex's, and the .aux resolves both citations.
    let bbl = std::fs::read(d.join("out/main.bbl")).unwrap();
    assert_eq!(bbl, bibtex_by_hand(&bin, &d));
    let aux = std::fs::read_to_string(d.join("out/main.aux")).unwrap();
    assert!(aux.contains("\\bibcite{knuth84}") && aux.contains("\\bibcite{lamport94}"));

    // A compile that changes nothing: bibtex is up to date.
    let cy = cycle(&mut c, &req(2, &d, "auto"));
    assert!(cy.events("run").is_empty(), "{cy:?}");
    assert_eq!(cy.dones.len(), 1);
    assert_eq!(
        cy.events("settled")[0].get("ran").and_then(Json::as_bool),
        Some(false)
    );

    // A new \cite: bibtex again, then the host's compile.
    let mut r = req(3, &d, "auto");
    r.buffers = vec![(
        "main.tex".into(),
        BIB_DOC.replace("followed.", "followed, as did Liang~\\cite{liang83}."),
    )];
    let cy = cycle(&mut c, &r);
    assert_eq!(cy.events("run").len(), 1, "{cy:?}");
    assert_eq!(cy.dones.last().unwrap().str_field("cause"), Some("tools"));
    let aux = std::fs::read_to_string(d.join("out/main.aux")).unwrap();
    assert!(aux.contains("\\bibcite{liang83}"), "{aux}");
    assert_eq!(
        std::fs::read(d.join("out/main.bbl")).unwrap(),
        bibtex_by_hand(&bin, &d)
    );

    // An edited .bib entry (the engine never reads the .bib: the compile
    // itself changes nothing, the tools see it).
    let mut r = req(4, &d, "auto");
    r.buffers = vec![(
        "refs.bib".into(),
        BIB.replace("The {\\TeX}book", "The New {\\TeX}book"),
    )];
    let cy = cycle(&mut c, &r);
    assert_eq!(cy.events("run").len(), 1, "{cy:?}");
    assert!(cy.events("run")[0]
        .str_field("reason")
        .is_some_and(|s| s.contains("changed")));
    let bbl = String::from_utf8(std::fs::read(d.join("out/main.bbl")).unwrap()).unwrap();
    assert!(bbl.contains("The New"), "{bbl}");
    assert_eq!(cy.dones.last().unwrap().str_field("cause"), Some("tools"));
    let _ = std::fs::remove_dir_all(&d);
}

/// A tool's warnings: `DIAGNOSTIC`s with `source`, or for a client that
/// accepted `diag-v1` (spec §6.7) `DIAG`s with `origin`, never both.
#[test]
fn a_tools_warnings_reach_both_kinds_of_client() {
    if tex_bin().is_none() {
        eprintln!("skipped: no TeX Live with bibtex");
        return;
    }
    let host = start_host("bibwarn", &[]);
    let bib = BIB.replace(
        "  publisher = {Addison-Wesley}, year = {1984}}",
        "  year = {1984}}",
    );
    for accept in [false, true] {
        let d = scratch(&format!("bibwarn-{accept}"));
        std::fs::write(d.join("main.tex"), BIB_DOC).unwrap();
        std::fs::write(d.join("refs.bib"), &bib).unwrap();
        let mut c = if accept {
            Client::connect_accepting(&host.1, &[flashtex_display_list::diag::CAPABILITY]).unwrap()
        } else {
            Client::connect(&host.1).unwrap()
        };
        let cy = cycle(&mut c, &req(1, &d, "auto"));
        let done = cy.events("done");
        assert_eq!(done[0].str_field("status"), Some("warnings"), "{cy:?}");
        let from_bibtex = cy
            .diagnostics
            .iter()
            .filter(|j| j.str_field("source") == Some("bibtex"))
            .count();
        let tool_diags: Vec<_> = cy.diags.iter().filter(|d| d.origin == "bibtex").collect();
        if accept {
            assert!(cy.diagnostics.is_empty(), "{cy:?}");
            assert!(!tool_diags.is_empty(), "{cy:?}");
            for (k, t) in tool_diags.iter().enumerate() {
                assert_eq!((t.id, t.seq, t.exact), (1, k as i64, false));
                assert!(t.code.starts_with("bibtex/"), "{t:?}");
                assert!(t.message.contains("publisher"), "{t:?}");
            }
        } else {
            assert!(cy.diags.is_empty(), "{cy:?}");
            assert!(from_bibtex > 0, "{cy:?}");
        }
        let _ = std::fs::remove_dir_all(&d);
    }
}

#[test]
fn off_runs_nothing_and_says_so() {
    if tex_bin().is_none() {
        eprintln!("skipped: no TeX Live with bibtex");
        return;
    }
    let host = start_host("off", &[]);
    let d = scratch("off");
    std::fs::write(d.join("main.tex"), BIB_DOC).unwrap();
    std::fs::write(d.join("refs.bib"), BIB).unwrap();
    let mut c = Client::connect(&host.1).unwrap();
    // No external_tools key: the host's default, off.
    let mut r = req(1, &d, "off");
    r.external_tools = None;
    let cy = cycle(&mut c, &r);
    assert!(cy.events("run").is_empty(), "{cy:?}");
    let skip = cy.events("skip");
    assert_eq!(skip.len(), 1, "{cy:?}");
    assert!(skip[0]
        .str_field("reason")
        .is_some_and(|s| s.contains("off for this project")));
    assert_eq!(cy.dones.len(), 1);
    assert!(!d.join("out/main.bbl").exists());
    // Said once per state: the same compile again says nothing more.
    let cy = cycle(&mut c, &req(2, &d, "off"));
    assert!(cy.events("skip").is_empty(), "{cy:?}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_missing_bib_file_is_not_run() {
    if tex_bin().is_none() {
        eprintln!("skipped: no TeX Live with bibtex");
        return;
    }
    let host = start_host("nobib", &["--external-tools", "auto"]);
    let d = scratch("nobib");
    std::fs::write(d.join("main.tex"), BIB_DOC).unwrap();
    let mut c = Client::connect(&host.1).unwrap();
    let mut r = req(1, &d, "auto");
    r.external_tools = None; // the host's default: auto
    let cy = cycle(&mut c, &r);
    assert!(cy.events("run").is_empty(), "{cy:?}");
    assert!(cy.events("skip")[0]
        .str_field("reason")
        .is_some_and(|s| s.contains("refs")));
    let _ = std::fs::remove_dir_all(&d);
}

const IDX_DOC: &str = r"\documentclass{article}
\usepackage{makeidx}
\makeindex
\begin{document}
Alpha\index{alpha} and beta\index{beta}.
\newpage
Gamma\index{gamma} and alpha again\index{alpha}.
\printindex
\end{document}
";

#[test]
fn makeindex_runs_and_the_index_is_typeset() {
    let Some(bin) = tex_bin() else {
        eprintln!("skipped: no TeX Live with makeindex");
        return;
    };
    let host = start_host("idx", &[]);
    let d = scratch("idx");
    std::fs::write(d.join("main.tex"), IDX_DOC).unwrap();
    let mut c = Client::connect(&host.1).unwrap();
    let cy = cycle(&mut c, &req(1, &d, "auto"));
    let runs = cy.events("run");
    assert_eq!(runs.len(), 1, "{cy:?}");
    assert_eq!(runs[0].str_field("tool"), Some("makeindex"));
    assert_eq!(cy.dones.last().unwrap().str_field("cause"), Some("tools"));
    // makeindex by hand on the final .idx gives the .ind the host holds.
    let h = std::env::temp_dir().join(format!("flashtex-tools-idx-hand-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&h);
    std::fs::create_dir_all(&h).unwrap();
    std::fs::copy(d.join("out/main.idx"), h.join("main.idx")).unwrap();
    let st = Command::new(bin.join("makeindex"))
        .args(["-q", "-o", "main.ind", "main.idx"])
        .current_dir(&h)
        .status()
        .unwrap();
    assert!(st.success());
    assert_eq!(
        std::fs::read(h.join("main.ind")).unwrap(),
        std::fs::read(d.join("out/main.ind")).unwrap()
    );
    let ind = String::from_utf8(std::fs::read(d.join("out/main.ind")).unwrap()).unwrap();
    assert!(ind.contains("\\item alpha, 1, 2"), "{ind}");
    let _ = std::fs::remove_dir_all(&h);
    let _ = std::fs::remove_dir_all(&d);
}

/// A document's first compile runs a second pass for its `.aux` (DESIGN.md
/// §5.5), which typesets the pages again with the references resolved:
/// the client must end up holding those pages, not the first pass's (the
/// host's follow-up compiles after the tools rely on it too).
#[test]
fn later_passes_reach_the_client() {
    if tex_bin().is_none() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let host = start_host("passes", &[]);
    let d = scratch("passes");
    std::fs::write(
        d.join("main.tex"),
        "\\documentclass{article}\\begin{document}\\section{One}\\label{s}See section~\\ref{s} \
         on page~\\pageref{s}.\\newpage Again \\ref{s}.\\end{document}\n",
    )
    .unwrap();
    let mut c = Client::connect(&host.1).unwrap();
    let held = |c: &mut Client, id: i64, incremental: bool| {
        let mut r = req(id, &d, "off");
        r.incremental = incremental;
        c.compile(&r).unwrap();
        let mut pages = std::collections::BTreeMap::new();
        loop {
            match c.next_event().unwrap().expect("host closed the connection") {
                Event::Page(p) => {
                    pages.insert(p.index, p.hash);
                }
                Event::Done(d) => return (pages, d),
                _ => {}
            }
        }
    };
    let (first, _) = held(&mut c, 1, true);
    // Every page again, from the host's cache (a non-incremental compile).
    let (again, done2) = held(&mut c, 2, false);
    assert_eq!(done2.str_field("mode"), Some("unchanged"), "{done2}");
    assert_eq!(first, again, "the client holds pages of an earlier pass");
    let _ = std::fs::remove_dir_all(&d);
}

/// Lane P4-MULTIPASS: a compile whose `.aux` passes stopped for the tools
/// (`DONE.deferred`) gets its follow-up even when the tools change nothing
/// (bibtex on a `\cite` of a key the `.bib` lacks leaves the `.bbl` as it
/// was), with and without a viewport (the L4 path's `finish`): the new
/// label's `\ref` is resolved by that follow-up, as latexmk's next
/// pdflatex run resolves it.
#[test]
fn deferred_passes_run_after_tools_that_change_nothing() {
    if tex_bin().is_none() {
        eprintln!("skipped: no TeX Live with bibtex");
        return;
    }
    for viewport in [None, Some(0)] {
        let host = start_host("deferred", &[]);
        let d = scratch("deferred");
        let doc = |extra: &str| {
            format!(
                "\\documentclass{{article}}\n\\begin{{document}}\n\\section{{One}}\\label{{s}}\
                 Knuth wrote \\cite{{knuth84}}, see \\ref{{s}}.{extra}\n\\bibliographystyle{{plain}}\n\
                 \\bibliography{{refs}}\n\\end{{document}}\n"
            )
        };
        std::fs::write(d.join("main.tex"), doc("")).unwrap();
        std::fs::write(d.join("refs.bib"), BIB).unwrap();
        let mut c = Client::connect(&host.1).unwrap();
        cycle(&mut c, &req(1, &d, "auto"));
        let bbl = std::fs::read(d.join("out/main.bbl")).unwrap();
        std::fs::write(
            d.join("main.tex"),
            doc("\n\\section{Two}\\label{t}See \\ref{t} and \\cite{nosuchkey}."),
        )
        .unwrap();
        let mut r = req(2, &d, "auto");
        r.viewport = viewport;
        let cy = cycle(&mut c, &r);
        let bib = cy
            .events("done")
            .into_iter()
            .find(|t| t.str_field("tool") == Some("bibtex"))
            .cloned();
        assert!(bib.is_some(), "{viewport:?}: bibtex did not run: {cy:?}");
        assert_eq!(std::fs::read(d.join("out/main.bbl")).unwrap(), bbl);
        assert_eq!(
            cy.dones[0].get("deferred").and_then(Json::as_bool),
            Some(true),
            "{viewport:?}: the passes did not wait for bibtex: {cy:?}"
        );
        assert!(
            cy.dones.len() >= 2,
            "{viewport:?}: no follow-up for the deferred passes: {cy:?}"
        );
        let aux = std::fs::read_to_string(d.join("out/main.aux")).unwrap();
        assert!(aux.contains("\\newlabel{t}"), "{aux}");
        let log = std::fs::read_to_string(d.join("out/main.log")).unwrap();
        assert!(
            !log.contains("Reference `t'"),
            "{viewport:?}: the new label's reference is still undefined: {log}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}

/// A tool that runs out of time is killed, reported (`status: timeout`),
/// and its output is not used; the cycle still settles.
#[test]
fn a_tool_out_of_time_is_reported_and_its_output_unused() {
    if tex_bin().is_none() {
        eprintln!("skipped: no TeX Live with bibtex");
        return;
    }
    let host = start_host("timeout", &["--tool-timeout", "0.000001"]);
    let d = scratch("timeout");
    std::fs::write(d.join("main.tex"), BIB_DOC).unwrap();
    std::fs::write(d.join("refs.bib"), BIB).unwrap();
    let mut c = Client::connect(&host.1).unwrap();
    let cy = cycle(&mut c, &req(1, &d, "auto"));
    let done = cy.events("done");
    assert_eq!(done.len(), 1, "{cy:?}");
    assert_eq!(done[0].str_field("status"), Some("timeout"), "{cy:?}");
    assert_eq!(done[0].get("changed").and_then(Json::as_bool), Some(false));
    assert!(!d.join("out/main.bbl").exists());
    // The only follow-up is the `.aux` passes the first compile left for
    // after the tools (lane P4-MULTIPASS, `incr::Session::set_defer`),
    // not a compile of an output the tool did not make.
    let deferred = cy.dones[0].get("deferred").and_then(Json::as_bool) == Some(true);
    assert_eq!(
        cy.dones.len(),
        1 + deferred as usize,
        "no follow-up compile but the deferred passes: {cy:?}"
    );
    if deferred {
        assert_eq!(
            cy.dones[1].get("deferred").and_then(Json::as_bool),
            Some(false)
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}
