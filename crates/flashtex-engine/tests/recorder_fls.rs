//! `-recorder`'s `.fls` against TeX Live's own `pdftex`, every line after
//! `PWD`, in order:
//! - kpathsea's own reads: the `texmf.cnf` files when it starts, every
//!   `texfonts.map` at the first TFM lookup;
//! - a file lib/openclose.c's `open_input` opened (`\input`, `\openin`,
//!   the format, TFM, map and encoding files), as `nameoffile + 1`: without
//!   the `./` kpathsea puts in front of a file in the current directory
//!   unless the name asked for had one (`INPUT sub.tex` for `\input sub`,
//!   `INPUT ./sub.tex` for `\input ./sub`);
//! - the C parts' own lines, the path as the lookup returned it
//!   (`./sub.tex`): `\pdffilesize`, `\pdffilemoddate`, `\pdffiledump`,
//!   `\pdfmdfivesum file` (texmfmp.c), `\pdfximage` (writeimg.c), a Type 1
//!   font file (writet1.c).
#![cfg(all(unix, feature = "kpathsea"))]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

/// (job, extra options, document).
const CASES: &[(&str, &[&str], &str)] = &[
    (
        "plain-name",
        &[],
        "\\input sub \\openin1=sub \\closein1 \\bye\n",
    ),
    (
        "dot-name",
        &[],
        "\\input ./sub \\openin1=./sub.tex \\closein1 \\bye\n",
    ),
    (
        "sub-dir",
        &[],
        "\\input dir/sub \\input ./dir/sub \\openin1=dir/sub \\closein1 \\bye\n",
    ),
    (
        "written",
        &[],
        "\\immediate\\openout1=out.tex \\immediate\\write1{\\relax}\
         \\immediate\\closeout1 \\input out \\input ./out \\bye\n",
    ),
    // (found in -output-directory first: openclose.c records that path)
    (
        "outdir",
        &["-output-directory=od"],
        "\\input sub2.tex \\input sub \\bye\n",
    ),
    // (a file that is not there is not listed; \pdffiledump of 0 bytes
    // looks nothing up)
    (
        "queries",
        &[],
        "\\message{\\pdffilesize{sub.tex}\\pdffilesize{./sub}\\pdfmdfivesum file{sub.tex}\
         \\pdffiledump length 4{sub.tex}\\pdffiledump length 0{dir/sub.tex}\
         \\pdffilemoddate{dir/sub.tex}\\pdffilesize{nothere.tex}}\\bye\n",
    ),
    (
        "page",
        &[],
        "\\pdfoutput=1 \\font\\big=cmss12 \\big \\pdfximage{img.png}\
         \\pdfximage page 2{img.pdf}\\pdfrefximage\\pdflastximage\\ A\\par\\vfill\\eject\
         \\pdfximage{img.png}\\pdfrefximage\\pdflastximage\\ $x$\\bye\n",
    ),
];

fn run(bin: &Path, dir: &Path, args: &[&str], ours: bool) {
    let mut c = Command::new(bin);
    c.args(args)
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
    c.status().unwrap();
}

/// The `.fls` lines after `PWD`, without kpathsea's `texmf.cnf` reads.
fn fls(dir: &Path, name: &str) -> Vec<String> {
    std::fs::read_to_string(dir.join(name).with_extension("fls"))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with("PWD "))
        .map(str::to_string)
        .collect()
}

#[test]
fn recorder_names_match_tex_live() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let base = common::fresh_dir("flashtex-fls");
    let (a, b) = (base.join("ours"), base.join("tex"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d.join("dir")).unwrap();
        std::fs::create_dir_all(d.join("od")).unwrap();
        std::fs::write(d.join("sub.tex"), "\\message{sub}\n").unwrap();
        std::fs::write(d.join("dir/sub.tex"), "\\message{dir/sub}\n").unwrap();
        std::fs::write(d.join("od/sub2.tex"), "\\message{od/sub2}\n").unwrap();
        let images = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/images");
        std::fs::copy(images.join("png-rgb8.png"), d.join("img.png")).unwrap();
        std::fs::copy(images.join("pdf-hand.pdf"), d.join("img.pdf")).unwrap();
        for (name, _, body) in CASES {
            std::fs::write(d.join(format!("{name}.tex")), body).unwrap();
        }
    }
    run(ours, &a, &["-ini", "\\input plain \\dump"], true);
    run(&theirs, &b, &["-ini", "\\input plain \\dump"], false);
    let mut failures = vec![];
    for (name, opts, _) in CASES {
        let mut args = vec!["-fmt=plain", "-recorder"];
        args.extend_from_slice(opts);
        args.push(name);
        run(ours, &a, &args, true);
        run(&theirs, &b, &args, false);
        let job = match opts.first() {
            Some(o) => format!("{}/{name}", o.trim_start_matches("-output-directory=")),
            None => name.to_string(),
        };
        let (x, y) = (fls(&a, &job), fls(&b, &job));
        if x != y || y.is_empty() {
            failures.push(format!("{name}.fls:\n  ours {x:?}\n  tex  {y:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    if std::env::var_os("FLASHTEX_KEEP_TEST_DIR").is_none() {
        let _ = std::fs::remove_dir_all(&base);
    }
}
