//! `-recorder`'s `.fls` for `flashtex-initex -ini` against a live oracle:
//! the installed TeX Live's own `pdftex -ini -recorder`, run at test time
//! in its own temp dir on the same inputs, with the same environment
//! (`SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, stdin null). Every line of
//! `t.fls`, in order, must match. A live comparison (like `recorder_fls.rs`,
//! which covers `-fmt=plain` runs and is not duplicated here), so the
//! install-specific set of `texmf.cnf` files kpathsea reads cannot fail the
//! test on a TeX Live laid out differently (a hardcoded oracle would).
//!
//! Normalization, applied to BOTH sides' `.fls` before comparing:
//! - the `PWD` line's directory becomes `PWD <PWD>` (each side runs in its
//!   own temp dir);
//! - the TeX Live install root (canonical `<root>/bin/<arch>/pdftex`,
//!   stripped to `<root>`) becomes `<TEXLIVE>`.
//!
//! Each test also keeps a small install-independent sanity assertion (the
//! oracle's `.fls` is non-empty, starts with the `PWD` line and contains
//! `INPUT t.tex`, plus one case-specific check run on both sides), so a
//! broken oracle cannot make a vacuous pass.
//!
//! Virgin `-ini` has virgin catcodes (`{`/`}` are "other", as where
//! plain.tex starts), so cases needing braces carry plain.tex's own first
//! line, `\catcode`\{=1 \catcode`\}=2 `, which reads no file.
//!
//! Not here: a `-recorder` run after a non-recorder run in the same
//! process must still list `texfonts.map`. No existing helper runs two
//! compiles with different recorder settings in one process
//! (`flashtex-initex` runs once per process; `flashtex-host iserve` takes
//! `-recorder` as a startup engine option with no per-compile toggle). That
//! ordering is `system.rs`'s `fontmaps_due`, which latches only once a
//! `-recorder` run lists the lines, and its unit test
//! `fontmaps_wait_for_a_recorder_run`.
#![cfg(all(unix, feature = "kpathsea"))]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

/// Run our engine on `t.tex` in `dir`, with `args` before the file name.
/// Returns exit success.
fn run_ours(dir: &Path, args: &[&str]) -> bool {
    Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
        .args(["-ini", "-recorder"])
        .args(args)
        .arg("t.tex")
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success()
}

/// Run the installed `pdftex -ini -recorder <args> t.tex` in `dir`.
/// Returns exit success.
fn run_theirs(pdftex: &Path, dir: &Path, args: &[&str]) -> bool {
    Command::new(pdftex)
        .args(["-ini", "-recorder"])
        .args(args)
        .arg("t.tex")
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success()
}

/// The TeX Live install root: the canonical `pdftex` is
/// `<root>/bin/<arch>/pdftex`; strip three components.
fn texlive_root(texbin: &Path) -> String {
    let pdftex = std::fs::canonicalize(texbin.join("pdftex")).unwrap();
    pdftex
        .ancestors()
        .nth(3)
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

/// The `.fls` file `fls` (in `dir`)'s lines, normalized: `PWD <PWD>`, the
/// install root `<TEXLIVE>`.
fn normalized(dir: &Path, fls: &str, root: &str) -> Vec<String> {
    std::fs::read_to_string(dir.join(fls))
        .unwrap_or_default()
        .lines()
        .map(|l| {
            if l.starts_with("PWD ") {
                "PWD <PWD>".to_string()
            } else {
                l.replace(root, "<TEXLIVE>")
            }
        })
        .collect()
}

/// First line mentioning `needle`, if any.
fn pos(fls: &[String], needle: &str) -> Option<usize> {
    fls.iter().position(|l| l.contains(needle))
}

/// Write `files` into a fresh temp dir per side, run both engines, and
/// compare their normalized `t.fls` line by line, in order. `extra` is a
/// case-specific install-independent sanity check, run on BOTH sides.
/// Skips where there is no TeX Live.
fn check(files: &[(&str, &str)], extra: impl Fn(&[String])) {
    check_with(files, &[], &[], "t.fls", extra)
}

/// [`check`], with copies of TeX Live's own files (`kpsewhich`'s path of
/// the second name, put at the first), engine options `args`, and the
/// `.fls` at `fls`.
fn check_with(
    files: &[(&str, &str)],
    copies: &[(&str, &str)],
    args: &[&str],
    fls: &str,
    extra: impl Fn(&[String]),
) {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let base = common::fresh_dir("flashtex-fls-oracle");
    let (a, b) = (base.join("ours"), base.join("tex"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
        for (name, body) in files {
            let p = d.join(name);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(p, body).unwrap();
        }
        for (to, name) in copies {
            let o = Command::new(texbin.join("kpsewhich"))
                .arg(name)
                .output()
                .unwrap();
            let from = String::from_utf8_lossy(&o.stdout).trim().to_string();
            assert!(!from.is_empty(), "kpsewhich finds no {name}");
            let p = d.join(to);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::copy(&from, p).unwrap();
        }
    }
    assert!(
        run_ours(&a, args),
        "our engine failed; see {}/t.log",
        a.display()
    );
    let theirs = texbin.join("pdftex");
    assert!(
        run_theirs(&theirs, &b, args),
        "the pdftex oracle failed; see {}/t.log",
        b.display()
    );
    let root = texlive_root(&texbin);
    let (got, want) = (normalized(&a, fls, &root), normalized(&b, fls, &root));
    // The oracle must look like a real run, or the comparison is vacuous.
    assert!(!want.is_empty(), "the pdftex oracle wrote an empty t.fls");
    assert_eq!(want[0], "PWD <PWD>", "oracle t.fls starts oddly: {want:?}");
    assert!(
        want.contains(&"INPUT t.tex".to_string()),
        "oracle t.fls lacks INPUT t.tex: {want:?}"
    );
    extra(&want);
    extra(&got);
    assert_eq!(got, want, "t.fls differs from the live pdftex oracle");
    if std::env::var_os("FLASHTEX_KEEP_TEST_DIR").is_none() {
        let _ = std::fs::remove_dir_all(&base);
    }
}

// pdftex lists `texfonts.map` (kpathsea's `read_all_maps` at the first TFM
// lookup) BEFORE the `.tfm` itself.
#[test]
fn font_lists_texfonts_map_before_the_tfm() {
    check(&[("t.tex", "\\font\\x=cmr10 \\end\n")], |fls| {
        let (map, tfm) = (pos(fls, "texfonts.map"), pos(fls, "cmr10.tfm"));
        assert!(
            map.is_some() && tfm.is_some() && map < tfm,
            "texfonts.map must precede cmr10.tfm: {fls:?}"
        );
    });
}

// A TFM in `-output-directory`: openclose.c's `open_input` tries the name
// there as it is (`cmr10`: tex.ch packs TFM names without `.tfm`) before
// any kpathsea search, so it is listed as `out/cmr10` and reads no
// `texfonts.map`; the first TFM kpathsea looks for (`cmr12`) does. Measured
// with pdfTeX 1.40.29: `INPUT out/cmr10`, then `INPUT .../texfonts.map`,
// then `INPUT .../cmr12.tfm`. (REVIEW-B's question on #2083: every TFM
// lookup is `input_path`'s, and this is its other branch.)
#[test]
fn a_tfm_in_the_output_directory_reads_no_font_map() {
    check_with(
        &[
            ("t.tex", "\\font\\x=cmr10 \\font\\y=cmr12 \\end\n"),
            ("out/.keep", ""),
        ],
        &[("out/cmr10", "cmr10.tfm")],
        &["-output-directory=out"],
        "out/t.fls",
        |fls| {
            let (here, map, kpse) = (
                pos(fls, "INPUT out/cmr10"),
                pos(fls, "texfonts.map"),
                pos(fls, "cmr12.tfm"),
            );
            assert!(
                here.is_some() && map.is_some() && kpse.is_some() && here < map && map < kpse,
                "want out/cmr10, then texfonts.map, then cmr12.tfm: {fls:?}"
            );
        },
    );
}

// `\input` of a file in a subdirectory: one line, the name as asked for
// (no kpathsea `./`, unlike the C lookups below).
#[test]
fn input_in_a_subdirectory() {
    check(
        &[
            ("t.tex", "\\input sub/foo \\end\n"),
            ("sub/foo.tex", "\\relax\n"),
        ],
        |fls| {
            assert!(
                fls.contains(&"INPUT sub/foo.tex".to_string()),
                "no INPUT sub/foo.tex: {fls:?}"
            );
        },
    );
}

// Each of the three C-part queries records its own lookup, spelled as the
// lookup returned it (`./f.txt`); three primitives, three lines.
#[test]
fn pdf_queries_list_the_file_thrice_with_dot_slash() {
    check(
        &[
            (
                "t.tex",
                "\\catcode`\\{=1 \\catcode`\\}=2 \
                 \\message{\\pdffilesize{f.txt}|\\pdffilemoddate{f.txt}|\
                 \\pdfmdfivesum file{f.txt}}\\end\n",
            ),
            ("f.txt", "hello\n"),
        ],
        |fls| {
            assert_eq!(
                fls.iter().filter(|l| l.as_str() == "INPUT ./f.txt").count(),
                3,
                "want three INPUT ./f.txt: {fls:?}"
            );
        },
    );
}

// A missing file with `\openin`: looked up, not found, not listed.
#[test]
fn missing_openin_lists_nothing() {
    check(&[("t.tex", "\\openin1=nosuch.tex \\end\n")], |fls| {
        assert!(
            pos(fls, "nosuch").is_none(),
            "a missing file must not be listed: {fls:?}"
        );
    });
}

// `\openout` (announced when the log-side name is set), `\write`,
// `\closeout`: one `OUTPUT` line for the file.
#[test]
fn openout_write_closeout_lists_one_output() {
    check(
        &[(
            "t.tex",
            "\\catcode`\\{=1 \\catcode`\\}=2 \\immediate\\openout1=o.txt \
             \\immediate\\write1{hi} \\immediate\\closeout1 \\end\n",
        )],
        |fls| {
            assert!(
                fls.contains(&"OUTPUT o.txt".to_string()),
                "no OUTPUT o.txt: {fls:?}"
            );
        },
    );
}
