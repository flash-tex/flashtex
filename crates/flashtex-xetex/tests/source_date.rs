//! texmfmp.c's reading of `SOURCE_DATE_EPOCH` and `FORCE_SOURCE_DATE`, which
//! XeTeX shares with pdfTeX (the pdfTeX engine's is
//! `crates/flashtex-engine/tests/texch_source_date.rs`), against TeX Live's
//! `xetex`:
//!
//! - `get_date_and_time`: a `FORCE_SOURCE_DATE` other than empty, `0` or
//!   `1` is `warning: invalid value (expected 0 or 1) ...` on stderr, once
//!   per `fix_date_and_time` (twice in INITEX, once in a production run),
//!   and the clock is used;
//! - `init_start_time`, which every run calls as it starts: a
//!   `SOURCE_DATE_EPOCH` that `strtoull` does not read whole is
//!   `<argv[0]>: fatal: invalid epoch-seconds-timezone value ...` and exit
//!   status 1, with or without `FORCE_SOURCE_DATE`.
//!
//! Both engines run as `xetex -ini -no-pdf`; stderr, the exit status and
//! `\year` and `\time` as `\message` shows them are compared. Skips where
//! there is no TeX Live.

use std::path::PathBuf;
use std::process::{Command, Stdio};

/// What a run shows: exit status, stderr, and the `[...]` the document
/// printed on the terminal.
type Seen = (Option<i32>, String, Vec<String>);

struct Engines {
    base: PathBuf,
    ours: PathBuf,
    tex: PathBuf,
}

impl Engines {
    fn new(name: &str) -> Option<Engines> {
        let Some(texbin) = flashtex_engine::resolver::find_texlive_bin() else {
            eprintln!("skipped: no TeX Live");
            return None;
        };
        let base =
            std::env::temp_dir().join(format!("flashtex-xetex-sde-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        for d in ["ours", "tex", "bin"] {
            std::fs::create_dir_all(base.join(d)).unwrap();
        }
        // invoked as `xetex`, as TeX Live's is: the fatal message names it
        let ours = base.join("bin").join("xetex");
        std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_flashtex-xetex"), &ours).unwrap();
        Some(Engines {
            base,
            ours,
            tex: texbin.join("xetex"),
        })
    }

    /// Runs `args` in both engines' directories with the environment `env`
    /// (`None` removes the variable), after writing `files`.
    fn both(
        &self,
        files: &[(&str, &str)],
        args: &[&str],
        env: &[(&str, Option<&str>)],
    ) -> (Seen, Seen) {
        let run = |ours: bool| {
            let d = self.base.join(if ours { "ours" } else { "tex" });
            for (n, t) in files {
                std::fs::write(d.join(n), t).unwrap();
            }
            let mut c = Command::new(if ours { &self.ours } else { &self.tex });
            c.args(args)
                .current_dir(&d)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            for (k, v) in env {
                match v {
                    Some(v) => c.env(k, v),
                    None => c.env_remove(k),
                };
            }
            if ours {
                // the files are found in the working directory
                c.env("FLASHTEX_RESOLVER", "cwd");
            }
            let o = c.output().unwrap();
            let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
            let shown = stdout
                .split('[')
                .skip(1)
                .filter_map(|s| s.split_once(']').map(|(a, _)| a.to_string()))
                .collect();
            (
                o.status.code(),
                String::from_utf8_lossy(&o.stderr).into_owned(),
                shown,
            )
        };
        (run(true), run(false))
    }
}

const YEAR: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\message{[\\the\\year]}\\end\n";

#[test]
fn force_source_date_other_than_0_or_1_is_warned_about() {
    let Some(e) = Engines::new("force") else {
        return;
    };
    for v in ["yes", "2", " 1", "0", "", "1"] {
        let env = [
            ("SOURCE_DATE_EPOCH", Some("0")),
            ("FORCE_SOURCE_DATE", Some(v)),
        ];
        let (x, y) = e.both(
            &[("d.tex", YEAR)],
            &["-ini", "-no-pdf", "-interaction=nonstopmode", "d.tex"],
            &env,
        );
        if !["0", "", "1"].contains(&v) {
            assert!(
                y.1.contains("expected 0 or 1"),
                "{v:?}: xetex did not warn: {y:?}"
            );
        }
        assert_eq!(x, y, "FORCE_SOURCE_DATE={v:?} (INITEX)");
    }
    // A production run fixes the date once, so warns once.
    let (_, y) = e.both(&[("f.tex", "\\dump\n")], &["-ini", "-no-pdf", "f.tex"], &[]);
    assert_eq!(y.0, Some(0), "xetex made no format: {y:?}");
    let env = [
        ("SOURCE_DATE_EPOCH", Some("0")),
        ("FORCE_SOURCE_DATE", Some("yes")),
    ];
    let (x, y) = e.both(
        &[("d.tex", YEAR)],
        &["-fmt=f", "-no-pdf", "-interaction=nonstopmode", "d.tex"],
        &env,
    );
    assert_eq!(y.1.matches("expected 0 or 1").count(), 1, "xetex: {y:?}");
    assert_eq!(x, y, "FORCE_SOURCE_DATE=yes (production run)");
    let _ = std::fs::remove_dir_all(&e.base);
}

#[test]
fn an_invalid_source_date_epoch_is_fatal() {
    let Some(e) = Engines::new("epoch") else {
        return;
    };
    let time =
        "\\catcode`\\{=1 \\catcode`\\}=2 \\message{[\\the\\year.\\the\\month.\\the\\day.\\the\\time]}\\end\n";
    let values = [
        "abc",
        "7 ",
        "12x",
        "+",
        " ",
        "99999999999999999999999",
        " 7",
        "+7",
        "-5",
        "007",
        // macOS's strtoull sets EINVAL for an empty string; glibc's does not
        "",
    ];
    for v in values {
        for force in [None, Some("1")] {
            for doc in [YEAR, time] {
                let env = [("SOURCE_DATE_EPOCH", Some(v)), ("FORCE_SOURCE_DATE", force)];
                let (x, y) = e.both(
                    &[("d.tex", doc)],
                    &["-ini", "-no-pdf", "-interaction=nonstopmode", "d.tex"],
                    &env,
                );
                // The program name is argv[0], a path here: compare the rest.
                let strip = |s: &Seen| {
                    let err =
                        s.1.lines()
                            .map(|l| {
                                l.split_once(": fatal: ")
                                    .map_or(l.to_string(), |(_, r)| format!("fatal: {r}"))
                            })
                            .collect::<Vec<_>>();
                    (s.0, err, s.2.clone())
                };
                // (without FORCE_SOURCE_DATE the time is the clock's: the
                // minutes may tick between the two runs)
                if force.is_none() && doc == time {
                    assert_eq!(strip(&x).0, strip(&y).0, "SOURCE_DATE_EPOCH={v:?}");
                    assert_eq!(strip(&x).1, strip(&y).1, "SOURCE_DATE_EPOCH={v:?}");
                    continue;
                }
                assert_eq!(
                    strip(&x),
                    strip(&y),
                    "SOURCE_DATE_EPOCH={v:?} FORCE_SOURCE_DATE={force:?}"
                );
            }
        }
    }
    let (_, y) = e.both(
        &[("d.tex", YEAR)],
        &["-ini", "-no-pdf", "d.tex"],
        &[("SOURCE_DATE_EPOCH", Some("abc"))],
    );
    assert_eq!(y.0, Some(1), "xetex accepted SOURCE_DATE_EPOCH=abc: {y:?}");
    let _ = std::fs::remove_dir_all(&e.base);
}
