//! Oracle tests around tex.ch's `Init`...`Tini` for `\patterns` ([49.1252])
//! and `\dump` ([51.1335]) in production runs, stacked on the PR that made a
//! production run print `! Patterns can be loaded only by INITEX.` and
//! `(\dump is performed only by INITEX)` instead of loading patterns or
//! writing a format.
//!
//! Each test makes a format with EACH engine's INITEX, then runs a small
//! production document with it as `pdftex -fmt=f`; the terminal output, the
//! log (after the banner), the exit status and whether `t.fmt` was written
//! are compared with TeX Live's pdfTeX, which is the LIVE oracle (not stored
//! text). Skips where there is no TeX Live (e.g. CI).
//!
//! What pdftex was observed to do while these tests were written (TeX Live
//! 2026): after its error, `\patterns` skips everything up to and including
//! the FIRST `}` token without counting nested `{`, so a nested group leaves
//! a stray `}` (`patterns_nested_braces`); `\dump` ends the job exactly like
//! `\end` and its message prints after the file closes. `\showhyphens` is not
//! covered: it is undefined in this minimal format (a plain/LaTeX macro, not
//! a primitive).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

/// Everything after the first line (the banner, whose date the log has).
fn after_banner(s: &str) -> String {
    s.split_once('\n')
        .map(|(_, b)| b.to_string())
        .unwrap_or_default()
}

struct Outcome {
    code: Option<i32>,
    term: String,
    log: String,
    fmt_written: bool,
}

/// Run `doc` as a production job on both engines. `None` where there is no
/// TeX Live (a skip that reports as a pass, failing only under
/// `FLASHTEX_REQUIRE_TEXLIVE=1`); otherwise `(ours, pdftex)`.
fn run_both(doc: &str) -> Option<(Outcome, Outcome)> {
    let texbin = find_texlive_bin()?;
    let base = common::fresh_dir("flashtex-initex-only-oracle");
    let fmt = "\\catcode`\\{=1 \\catcode`\\}=2 \\dump\n";
    let mut out = vec![];
    for (ours, sub) in [(true, "ours"), (false, "tex")] {
        let d = base.join(sub);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("f.tex"), fmt).unwrap();
        std::fs::write(d.join("t.tex"), doc).unwrap();
        let bin = if ours {
            let b = d.join("pdftex");
            common::link_engine(Path::new(env!("CARGO_BIN_EXE_flashtex-initex")), &b);
            b
        } else {
            texbin.join("pdftex")
        };
        let pass = |args: &[&str]| {
            let mut c = Command::new(&bin);
            c.args(args)
                .current_dir(&d)
                .env("SOURCE_DATE_EPOCH", "0")
                .env("FORCE_SOURCE_DATE", "1")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            if ours {
                c.env("FLASHTEX_FORMATS", &d).env(
                    "FLASHTEX_POOL",
                    Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
                );
            }
            c.output().unwrap()
        };
        let made = pass(&["-ini", "f.tex"]);
        assert!(d.join("f.fmt").is_file(), "{sub}: no format: {made:?}");
        let run = pass(&["-fmt=f", "-interaction=nonstopmode", "t.tex"]);
        let log = std::fs::read(d.join("t.log")).unwrap();
        out.push(Outcome {
            code: run.status.code(),
            term: after_banner(&String::from_utf8_lossy(&run.stdout)),
            log: after_banner(&String::from_utf8_lossy(&log)),
            fmt_written: d.join("t.fmt").exists(),
        });
    }
    let _ = std::fs::remove_dir_all(&base);
    let mut it = out.into_iter();
    Some((it.next().unwrap(), it.next().unwrap()))
}

/// The oracle must show every `want` in its log; then FlashTeX must match the
/// oracle's log, terminal, exit status and whether `t.fmt` was written.
fn check(ours: &Outcome, tex: &Outcome, wants: &[&str]) {
    for want in wants {
        assert!(
            tex.log.contains(want),
            "pdftex's log lacks {want:?}:\n{}",
            tex.log
        );
    }
    assert!(!tex.fmt_written, "pdftex wrote t.fmt in a production run");
    assert_eq!(ours.log, tex.log, "the log differs from pdftex's");
    assert_eq!(
        ours.term, tex.term,
        "the terminal output differs from pdftex's"
    );
    assert_eq!(ours.code, tex.code, "the exit status differs from pdftex's");
    assert_eq!(
        ours.fmt_written, tex.fmt_written,
        "whether t.fmt was written differs from pdftex"
    );
}

const PATTERNS_ERR: &str = "! Patterns can be loaded only by INITEX.";
const DUMP_MSG: &str = "(\\dump is performed only by INITEX)";

#[test]
fn patterns_several_patterns_then_text_after_brace() {
    let doc = "\\catcode`\\{=1 \\catcode`\\}=2 \\patterns{a1b c2d e3f}\nAFTER\n\\end\n";
    let Some((ours, tex)) = run_both(doc) else {
        common::no_texlive();
        return;
    };
    // The balanced group is gone whole: no stray-brace error follows, and the
    // page with AFTER still ships.
    assert!(
        !tex.log.contains("Too many }'s"),
        "pdftex reports a stray brace:\n{}",
        tex.log
    );
    assert_eq!(tex.code, Some(1), "pdftex's exit status:\n{}", tex.log);
    check(&ours, &tex, &[PATTERNS_ERR, "[0]"]);
}

#[test]
fn patterns_nested_braces() {
    let doc = "\\catcode`\\{=1 \\catcode`\\}=2 \\patterns{a1b c2d {x} e3f}\nAFTERBRACE\n\\end\n";
    let Some((ours, tex)) = run_both(doc) else {
        common::no_texlive();
        return;
    };
    // pdftex skips only to the FIRST `}`, leaving ` e3f}` behind: `e3f` is
    // typeset and the final `}` is stray.
    assert_eq!(tex.code, Some(1), "pdftex's exit status:\n{}", tex.log);
    check(&ours, &tex, &[PATTERNS_ERR, "! Too many }'s."]);
}

#[test]
fn two_patterns_in_a_row() {
    let doc = "\\catcode`\\{=1 \\catcode`\\}=2 \\patterns{a1b} \\patterns{c2d}\n\\end\n";
    let Some((ours, tex)) = run_both(doc) else {
        common::no_texlive();
        return;
    };
    assert_eq!(
        tex.log.matches(PATTERNS_ERR).count(),
        2,
        "pdftex reports the error twice:\n{}",
        tex.log
    );
    assert_eq!(tex.code, Some(1), "pdftex's exit status:\n{}", tex.log);
    check(&ours, &tex, &[PATTERNS_ERR]);
}

#[test]
fn patterns_inside_group_and_after_it_closes() {
    let doc = "\\catcode`\\{=1 \\catcode`\\}=2 {\\patterns{a1b}} AFTER \\patterns{c2d}\n\\end\n";
    let Some((ours, tex)) = run_both(doc) else {
        common::no_texlive();
        return;
    };
    assert_eq!(
        tex.log.matches(PATTERNS_ERR).count(),
        2,
        "pdftex reports the error twice:\n{}",
        tex.log
    );
    assert_eq!(tex.code, Some(1), "pdftex's exit status:\n{}", tex.log);
    check(&ours, &tex, &[PATTERNS_ERR, "[0]"]);
}

#[test]
fn dump_inside_group() {
    let doc = "\\catcode`\\{=1 \\catcode`\\}=2 {\\dump}\nAFTER\n\\end\n";
    let Some((ours, tex)) = run_both(doc) else {
        common::no_texlive();
        return;
    };
    assert_eq!(tex.code, Some(0), "pdftex's exit status:\n{}", tex.log);
    check(
        &ours,
        &tex,
        &[DUMP_MSG, "\\end occurred inside a group at level 1"],
    );
}

#[test]
fn dump_with_pending_paragraph_ends_job() {
    let doc =
        "\\catcode`\\{=1 \\catcode`\\}=2 Some pending text here\n\\dump\nMORE AFTER DUMP\n\\end\n";
    let Some((ours, tex)) = run_both(doc) else {
        common::no_texlive();
        return;
    };
    // The pending paragraph ships ([0]), then the job ends at `\dump`: the
    // lines after it are never read.
    assert_eq!(tex.code, Some(0), "pdftex's exit status:\n{}", tex.log);
    check(&ours, &tex, &[DUMP_MSG, "[0]"]);
}

#[test]
fn dump_after_patterns() {
    let doc = "\\catcode`\\{=1 \\catcode`\\}=2 \\patterns{a1b} \\dump MORE\n\\end\n";
    let Some((ours, tex)) = run_both(doc) else {
        common::no_texlive();
        return;
    };
    assert_eq!(tex.code, Some(1), "pdftex's exit status:\n{}", tex.log);
    check(&ours, &tex, &[PATTERNS_ERR, DUMP_MSG]);
}

#[test]
fn patterns_then_hyphenation() {
    let doc = "\\catcode`\\{=1 \\catcode`\\}=2 \\patterns{a1b} \\hyphenation{a-b} WORDS\n\\end\n";
    let Some((ours, tex)) = run_both(doc) else {
        common::no_texlive();
        return;
    };
    // Only `\patterns` is INITEX-only: `\hyphenation` runs fine and the job
    // ships its page.
    assert_eq!(
        tex.log.matches(PATTERNS_ERR).count(),
        1,
        "pdftex reports the error once:\n{}",
        tex.log
    );
    assert_eq!(tex.code, Some(1), "pdftex's exit status:\n{}", tex.log);
    check(&ours, &tex, &[PATTERNS_ERR, "[0]"]);
}
