//! tex.ch's two checks in `read_font_info`, against TeX Live's pdfTeX:
//! a font scaled to 2048pt or more is an error that ignores the scaling
//! factor and carries on ([30.568]), and a TFM name of more than 255
//! characters is "file name too long", not "file not found" ([30.560-563]).
//! Both are errors, which the lockstep harness cannot hold (it runs with
//! `-halt-on-error` and fails any nonzero exit), so the two engines run here
//! without it, in nonstop mode, and their logs are compared after the banner.
//! Skips where there is no TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

fn run(bin: &Path, dir: &Path, job: &str, ours: bool) -> Option<i32> {
    let mut c = Command::new(bin);
    c.args(["-ini", "-interaction=nonstopmode", &format!("{job}.tex")])
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if ours {
        c.env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        );
    }
    c.status().unwrap().code()
}

/// The log after its banner line.
fn body(dir: &Path, job: &str) -> String {
    let log = std::fs::read_to_string(dir.join(format!("{job}.log"))).unwrap();
    log.split_once('\n')
        .map(|(_, b)| b.to_string())
        .unwrap_or_default()
}

fn compare(job: &str, tex: &str, expect: &[&str]) {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let base = common::fresh_dir(&format!("flashtex-texch-{job}"));
    let (a, b) = (base.join("ours"), base.join("tex"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
        std::fs::write(d.join(format!("{job}.tex")), tex).unwrap();
    }
    // Both run as `pdftex`, so the invocation name in messages is the same.
    let ours = a.join("pdftex");
    common::link_engine(Path::new(env!("CARGO_BIN_EXE_flashtex-initex")), &ours);
    let (x, y) = (
        run(&ours, &a, job, true),
        run(&texbin.join("pdftex"), &b, job, false),
    );
    let (ours_log, tex_log) = (body(&a, job), body(&b, job));
    for e in expect {
        assert!(tex_log.contains(e), "pdfTeX's log lacks {e:?}:\n{tex_log}");
    }
    assert_eq!(
        ours_log, tex_log,
        "{job}.log differs from TeX Live's pdftex"
    );
    assert_eq!(x, y, "exit codes differ");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn font_scaled_to_2048pt_is_ignored_and_the_run_goes_on() {
    compare(
        "scale",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\tracingonline=1\n\
         \\font\\x=cminch scaled 32000 \\message{[\\fontname\\x]}\n\
         \\font\\w=cminch scaled 32000 \\message{[\\fontname\\w]}\n\
         \\font\\y=cmr10 scaled 32000 \\message{[\\fontname\\y]}\n\
         \\font\\z=cmr10 scaled 2000 \\message{[\\fontname\\z]}\n\
         \\x \\setbox0\\hbox{A}\\message{[\\the\\wd0]}\n\
         \\end\n",
        &[
            "! Font \\x=cminch scaled 32000 scaled to 2048pt or higher.",
            "I will ignore the scaling factor.",
            "[cminch]",
            "[cmr10 at 20.0pt]",
        ],
    );
}

#[test]
fn tfm_name_over_255_characters_is_too_long() {
    let long = "f".repeat(300);
    compare(
        "long",
        &format!(
            "\\catcode`\\{{=1 \\catcode`\\}}=2 \\tracingonline=1\n\
             \\font\\x={long} \\message{{[\\fontname\\x]}}\n\
             \\font\\y=cmr10 \\message{{[\\fontname\\y]}}\n\
             \\end\n"
        ),
        &["Metric (TFM) file name too long.", "[cmr10]"],
    );
}
