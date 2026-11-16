//! `-recorder` (lib/openclose.c's recorder): `JOBNAME.fls` lists `PWD`,
//! kpathsea's `texmf.cnf` files, then every file `open_input` opened
//! (`nameoffile + 1` after the open: `sub.tex` for `\input sub`,
//! `./sub.tex` for `\input ./sub`) and every file `open_output` wrote, in
//! order. The port wrote no `.fls` ("not supported in phase S0"), so a
//! `xelatex -recorder` run (latexmk's default) had none. The expected lines
//! are TeX Live 2026 XeTeX's (MacTeX 2026, macOS arm64) for the same input,
//! `xetex -ini -recorder t.tex`, without its `texmf.cnf` lines (the `cwd`
//! resolver reads none). The file primitives (`\filesize`,
//! `\filemoddate`, `\filedump`, `\mdfivesum file`) list the path their
//! lookup returned, as texmfmp.c's do (they listed nothing: LaTeX's
//! `\IfFileExists` asks `\filesize`). No TeX installation needed.

use std::process::Command;

const DOC: &str = "\\catcode`\\{=1 \\catcode`\\}=2
\\input sub
\\openin1=sub \\closein1
\\immediate\\openout2=w \\immediate\\write2{x}\\immediate\\closeout2
\\openin1=w \\closein1
\\openin1=nosuch \\closein1
\\input ./sub
\\end
";

const XETEX_FLS: &[&str] = &[
    "INPUT t.tex",
    "OUTPUT t.log",
    "INPUT sub.tex",
    "INPUT sub.tex",
    "OUTPUT w.tex",
    "INPUT w.tex",
    "INPUT ./sub.tex",
];

/// The file primitives, with names the `cwd` resolver returns as kpathsea
/// does (`./sub.tex`); a file that is not there, and `\filedump` of 0
/// bytes, list nothing.
const QUERIES: &str = "\\catcode`\\{=1 \\catcode`\\}=2
\\message{[\\filesize{./sub.tex}][\\filesize{./nosuch}]}
\\edef\\x{\\filemoddate{./sub.tex}}
\\edef\\x{\\filedump length 2{./sub.tex}}
\\edef\\x{\\filedump length 0{./sub.tex}}
\\edef\\x{\\mdfivesum file{./sub}}
\\end
";

const QUERIES_FLS: &[&str] = &[
    "INPUT t.tex",
    "OUTPUT t.log",
    "INPUT ./sub.tex",
    "INPUT ./sub.tex",
    "INPUT ./sub.tex",
    "INPUT ./sub.tex",
];

#[test]
fn the_fls_lists_what_was_opened() {
    check("open", DOC, XETEX_FLS);
}

#[test]
fn the_fls_lists_what_the_file_primitives_found() {
    check("queries", QUERIES, QUERIES_FLS);
}

fn check(case: &str, doc: &str, expected: &[&str]) {
    let d = std::env::temp_dir().join(format!(
        "flashtex-xetex-recorder-{case}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.tex"), doc).unwrap();
    std::fs::write(d.join("sub.tex"), "\\relax\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .args(["-ini", "-no-pdf", "-recorder", "t.tex"])
        .current_dir(&d)
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("SOURCE_DATE_EPOCH", "0")
        .output()
        .expect("run flashtex-xetex");
    let fls = std::fs::read_to_string(d.join("t.fls")).unwrap_or_else(|e| {
        panic!(
            "no t.fls ({e}); files: {:?}; terminal:\n{}",
            std::fs::read_dir(&d)
                .map(|r| r.flatten().map(|e| e.file_name()).collect::<Vec<_>>())
                .unwrap_or_default(),
            String::from_utf8_lossy(&out.stdout)
        )
    });
    let mut lines = fls.lines();
    let pwd = std::fs::canonicalize(&d).unwrap();
    assert_eq!(
        lines.next(),
        Some(format!("PWD {}", pwd.display()).as_str()),
        "{fls}"
    );
    let rest: Vec<&str> = lines.filter(|l| !l.ends_with("/texmf.cnf")).collect();
    assert_eq!(rest, expected, "{fls}");
    // no `xetex<pid>.fls` is left behind: it was renamed
    let left: Vec<_> = std::fs::read_dir(&d)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".fls") && n != "t.fls")
        .collect();
    assert!(left.is_empty(), "{left:?}");
    let _ = std::fs::remove_dir_all(&d);
}
