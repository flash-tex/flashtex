//! The end-of-job "stack positions" line (tex.web §1334) with replays on,
//! against TeX Live's pdfTeX (changes/stackpeak.ch, `src/stackpeak.rs`).
//!
//! A replayed intrinsic or BOX-MEMO call pushes nothing, so a replay in a
//! deeper place than its recording used to leave `max_in_stack`,
//! `max_param_stack` and `max_save_stack` lower than pdfTeX's (`14i` for
//! `15i`, `21p` for `30p`, `44s` for `55s` on `deeper_replay.tex`).
//!
//! * Plain INITEX documents whose registered macros are recorded shallow
//!   and replayed deep, with each recorder on alone (the line must equal
//!   pdfTeX's, and a replay must have happened).
//! * `fixtures/beamer-v3/long-deck`, run as P-T1 runs a document (four
//!   passes, `\tracingstats=2`) through this engine's `pdflatex.fmt` with
//!   both recorders on; and an arXiv paper the same way when
//!   `FLASHTEX_STATS_ARXIV_DIR` names a directory with its `main.tex` (the
//!   parity corpus's `2501.06980v1`).
//!
//! Skips where there is no TeX Live (e.g. CI without it).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn pool() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool")
}

fn ours_in(d: &Path) -> PathBuf {
    let b = d.join("pdftex");
    if !b.exists() {
        common::link_engine(Path::new(env!("CARGO_BIN_EXE_flashtex-initex")), &b);
    }
    b
}

fn stack_line(log: &Path) -> String {
    let log = std::fs::read(log).unwrap_or_default();
    String::from_utf8_lossy(&log)
        .lines()
        .find(|l| l.contains("stack positions out of"))
        .unwrap_or("(no stack positions line)")
        .trim()
        .to_string()
}

/// The count `name: N` in a recorder's statistics file (its `Debug` form).
fn stat(file: &Path, name: &str) -> u64 {
    let s = std::fs::read_to_string(file).unwrap_or_default();
    s.lines()
        .filter_map(|l| l.trim().strip_prefix(&format!("{name}: ")))
        .filter_map(|v| v.trim_end_matches(',').parse().ok())
        .next()
        .unwrap_or(0)
}

const SHALLOW_AND_DEEP: &str = r"\catcode`\{=1 \catcode`\}=2 \catcode`\#=6 \tracingstats=2
\def\inner{\begingroup\count1=1 \begingroup\count1=2 \begingroup\count1=3
  \begingroup\count1=4 \relax\endgroup\endgroup\endgroup\endgroup}
\def\ia{\ib\relax}\def\ib{\ic\relax}\def\ic{\id\relax}\def\id{\inner\relax}
\def\foo{\ia\relax}
\def\bar#1{\begingroup\setbox1\hbox{\setbox2\hbox{\setbox3\hbox{#1}}}\count2=1 \begingroup\count2=2
  \begingroup\count2=3 \relax\endgroup\endgroup\endgroup\ia}
\def\deep#1{\da{#1}\relax}\def\da#1{\db{#1}\relax}\def\db#1{\dc{#1}\relax}
\def\dc#1{\begingroup\begingroup\begingroup\begingroup\begingroup\begingroup
  #1\endgroup\endgroup\endgroup\endgroup\endgroup\endgroup\relax}
\foo\deep{\foo}
\setbox5\hbox{\bar{x}\deep{\bar{x}}}
\end
";

fn deeper_replay() -> String {
    let relaxes = "\\relax".repeat(60);
    format!(
        r"\catcode`\{{=1 \catcode`\}}=2 \catcode`\#=6 \tracingstats=2
\def\inner{{\begingroup\count1=1 \begingroup\count1=2 \begingroup\count1=3
  \begingroup\count1=4 \relax\endgroup\endgroup\endgroup\endgroup}}
\def\ia{{\ib\relax}}\def\ib{{\ic\relax}}\def\ic{{\id\relax}}\def\id{{\inner\relax}}
\def\foo{{\ia\relax}}
\def\pa#1#2#3{{\pb{{#1}}{{#2}}{{#3}}\relax}}\def\pb#1#2#3{{\pc{{#1}}{{#2}}{{#3}}\relax}}
\def\pc#1#2#3{{\begingroup\count2=#1 \expandafter\ifx\csname anotherrathermuchlongercontrolsequencenamehere\endcsname\relax\fi\endgroup}}
\def\bar#1{{\begingroup\setbox1\hbox{{\setbox2\hbox{{\setbox3\hbox{{\setbox4\vbox{{#1}}}}}}}}\count2=1 \begingroup\count2=2
  \begingroup\count2=3 \pa{{1}}{{2}}{{3}}\relax\endgroup\endgroup\endgroup\ia}}
\def\deep#1{{\da{{#1}}{{x}}{{y}}\relax}}\def\da#1#2#3{{\db{{#1}}{{#2}}{{#3}}\relax}}\def\db#1#2#3{{\dc{{#1}}{{#2}}{{#3}}\relax}}
\def\dc#1#2#3{{\begingroup\begingroup\begingroup\begingroup\begingroup\begingroup
  #1\endgroup\endgroup\endgroup\endgroup\endgroup\endgroup\relax}}
\expandafter\let\csname anotherrathermuchlongercontrolsequencenamehere\endcsname\relax
\setbox7\vbox{{\bar{{x}}}}
\setbox7\vbox{{\deep{{\deep{{\bar{{x}}}}}}}}{relaxes}
\foo
\deep{{\deep{{\foo}}}}{relaxes}
\end
"
    )
}

#[test]
fn a_deeper_replay_raises_the_stack_marks_as_the_expansion_would() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let base = common::fresh_dir("flashtex-stack-positions");
    for (name, doc) in [
        ("shallow-and-deep", SHALLOW_AND_DEEP.to_string()),
        ("deeper-replay", deeper_replay()),
    ] {
        let d = base.join(name);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("p.tex"), &doc).unwrap();
        let tl = Command::new(texbin.join("pdftex"))
            .args(["-ini", "-etex", "-interaction=nonstopmode", "p.tex"])
            .current_dir(&d)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(tl.code().is_some(), "{name}: pdftex did not run");
        let want = stack_line(&d.join("p.log"));
        assert!(want.contains("stack positions"), "{name}: pdftex: {want}");
        let ours = ours_in(&d);
        for (what, env, stats, counter) in [
            (
                "the intrinsics",
                [
                    ("FLASHTEX_INTRINSIC_NAMES", "foo"),
                    ("FLASHTEX_BOXMEMO", "off"),
                ],
                "FLASHTEX_INTRINSICS_STATS",
                "replays",
            ),
            (
                "BOX-MEMO",
                [("FLASHTEX_INTRINSICS", "off"), ("FLASHTEX_BOXMEMO", "on")],
                "FLASHTEX_BOXMEMO_STATS",
                "hits",
            ),
        ] {
            let sf = d.join(format!("{counter}.txt"));
            let _ = std::fs::remove_file(&sf);
            Command::new(&ours)
                .args(["-ini", "-etex", "-interaction=nonstopmode", "p.tex"])
                .current_dir(&d)
                .envs(env)
                .env("FLASHTEX_BOXMEMO_NAMES", "bar")
                .env(stats, &sf)
                .env("FLASHTEX_POOL", pool())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .status()
                .unwrap();
            assert!(stat(&sf, counter) > 0, "{name}, {what}: no replay ({sf:?})");
            assert_eq!(stack_line(&d.join("p.log")), want, "{name}, {what}");
        }
    }
    let _ = std::fs::remove_dir_all(&base);
}

/// P-T1's runs of `src`'s `main.tex` with each engine (four passes); the
/// stack positions line of the last.
fn pt1_stack_lines(base: &Path, texbin: &Path, src: &Path, tag: &str) -> (String, String) {
    let fmt = base.join("fmt");
    if !fmt.join("pdflatex.fmt").exists() {
        std::fs::create_dir_all(&fmt).unwrap();
        let st = Command::new(ours_in(&fmt))
            .args([
                "-ini",
                "-jobname=pdflatex",
                "-progname=pdflatex",
                "-translate-file=cp227.tcx",
                "*pdflatex.ini",
            ])
            .current_dir(&fmt)
            .env("FLASHTEX_POOL", pool())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(
            st.success() && fmt.join("pdflatex.fmt").is_file(),
            "no pdflatex.fmt"
        );
    }
    let first = r"\pdfsetrandomseed 1\relax\tracingstats=2 \input{main.tex}";
    let mut lines = vec![];
    for ours in [true, false] {
        let d = base.join(format!("{tag}-{}", if ours { "ours" } else { "tex" }));
        let _ = std::fs::remove_dir_all(&d);
        copy_dir(src, &d);
        for _ in 0..4 {
            let mut c = if ours {
                let mut c = Command::new(ours_in(&fmt));
                c.env("FLASHTEX_POOL", pool())
                    .env("FLASHTEX_FORMATS", &fmt)
                    .env("FLASHTEX_INTRINSICS", "on")
                    .env("FLASHTEX_BOXMEMO", "on");
                c
            } else {
                Command::new(texbin.join("pdftex"))
            };
            c.args(["-fmt=pdflatex", "-interaction=nonstopmode", first])
                .current_dir(&d)
                .env("SOURCE_DATE_EPOCH", "0")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();
        }
        lines.push(stack_line(&d.join("main.log")));
    }
    (lines[0].clone(), lines[1].clone())
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let t = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &t);
        } else {
            std::fs::copy(e.path(), t).unwrap();
        }
    }
}

#[test]
fn long_deck_and_a_paper_print_pdftex_s_stack_positions_with_replays_on() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let base = common::fresh_dir("flashtex-stack-positions-pt1");
    let deck = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/beamer-v3/long-deck");
    let mut docs = vec![("long-deck", deck)];
    if let Some(p) = std::env::var_os("FLASHTEX_STATS_ARXIV_DIR") {
        docs.push(("arxiv", PathBuf::from(p)));
    }
    for (tag, src) in docs {
        let (ours, tex) = pt1_stack_lines(&base, &texbin, &src, tag);
        assert!(tex.contains("stack positions"), "{tag}: pdftex: {tex}");
        assert_eq!(ours, tex, "{tag}: the stack positions line");
    }
    let _ = std::fs::remove_dir_all(&base);
}
