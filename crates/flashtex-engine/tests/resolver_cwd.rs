//! The working-directory resolver ([`CwdResolver`], `FLASHTEX_RESOLVER=cwd`
//! and `cwd-kpse`) against TeX Live's `kpsewhich` and `pdftex`, for TFM names.
//!
//! kpathsea's TFM format is suffix-only (`suffix_search_only`): a name that
//! does not end in `.tfm` is looked for only as `name.tfm`, never as given.
//! So a file `zzbare` (a TFM without the extension) in the working directory
//! is not found, and `\font\x=zzbare` is `nullfont` in pdfTeX. Every expected
//! answer here comes from running `kpsewhich` or `pdftex`; the test skips
//! (passes with a note) where there is no TeX Live.
//!
//! This file holds one test only, because it changes the process's working
//! directory: `CwdResolver` looks names up relative to it.

mod common;

use flashtex_engine::resolver::{find_texlive_bin, CwdResolver, FileResolver, Format};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Any small file will do for the resolver; pdfTeX needs a real TFM.
fn tfm_bytes(bin: &Path) -> Vec<u8> {
    let out = Command::new(bin.join("kpsewhich"))
        .arg("cmr10.tfm")
        .output()
        .expect("run kpsewhich");
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    std::fs::read(&p).unwrap_or_else(|e| panic!("reading cmr10.tfm at {p:?}: {e}"))
}

fn kpsewhich(bin: &Path, cwd: &Path, path_dir: &Path, name: &str, f: Format) -> String {
    let var = match f {
        Format::Tfm => "TFMFONTS",
        _ => "TEXINPUTS",
    };
    let out = Command::new(bin.join("kpsewhich"))
        .current_dir(cwd)
        // The working directory, then the one directory FLASHTEX_TFM_PATH /
        // FLASHTEX_INPUTS names for the resolver: nothing from texmf trees.
        .env(var, format!(".:{}", path_dir.display()))
        .arg("-no-mktex=tfm")
        .arg(format!("-format={}", f.kpse_name()))
        .arg(name)
        .output()
        .expect("run kpsewhich");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn show(p: Option<PathBuf>) -> String {
    p.map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// `\font\fntN=NAME \message{<<NAME=\fontname\fntN>>}` for each name, run by
/// `pdftex -ini` and by our INITEX; the `<<...>>` lines of the log.
fn font_messages(dir: &Path, names: &[&str], program: &mut Command) -> Vec<String> {
    let mut src = String::from("\\catcode`\\{=1 \\catcode`\\}=2 \\batchmode\n");
    for (i, n) in names.iter().enumerate() {
        let id: String = (i as u32)
            .to_string()
            .chars()
            .map(|c| (b'a' + c.to_digit(10).unwrap() as u8) as char)
            .collect();
        src.push_str(&format!(
            "\\font\\fnt{id}={n} \\message{{<<{n}=\\fontname\\fnt{id}>>}}\n"
        ));
    }
    src.push_str("\\end\n");
    std::fs::write(dir.join("t.tex"), src).unwrap();
    let _ = std::fs::remove_file(dir.join("t.log"));
    let st = program
        .current_dir(dir)
        .args(["-ini", "t.tex"])
        // kpathsea would try mktextfm for each missing font; it cannot make
        // one from no METAFONT source, and the answer is nullfont either way.
        .env("MKTEXTFM", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run TeX");
    let _ = st;
    let log = std::fs::read_to_string(dir.join("t.log")).expect("t.log");
    let joined = log.replace('\n', "");
    joined
        .split("<<")
        .skip(1)
        .filter_map(|s| s.split(">>").next())
        // An error's context lines show the source, `\fontname` unexpanded.
        .filter(|s| !s.contains("\\fontname"))
        .map(str::to_string)
        .collect()
}

#[test]
fn tfm_names_agree_with_kpsewhich_and_pdftex() {
    let Some(bin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let root = common::fresh_dir("flashtex-engine-resolver-cwd");
    let d = root.join("work");
    let p = root.join("tfmpath");
    std::fs::create_dir_all(d.join("sub")).unwrap();
    std::fs::create_dir_all(&p).unwrap();
    let tfm = tfm_bytes(&bin);
    for f in [
        "zzbare",
        "plain.tfm",
        "foo.bar",
        "baz.qux.tfm",
        "both",
        "both.tfm",
        "pair.bar",
        "pair.bar.tfm",
        "noext.tfm.tfm",
        "dot.",
        "sub/subbare",
        "sub/subt.tfm",
        // TeX input, whose rule (name.tex, then name) is unchanged.
        "tin.tex",
        "tbare",
        "tdot.bar",
        "sub/tsub.tex",
    ] {
        std::fs::write(d.join(f), &tfm).unwrap();
    }
    std::fs::create_dir_all(p.join("psub")).unwrap();
    for f in ["inpath", "inpath2.tfm", "tpath", "psub/q.tfm"] {
        std::fs::write(p.join(f), &tfm).unwrap();
    }
    // Names that are absolute or explicitly relative only on Windows
    // (`kpse_absolute_p` under DOSISH, absolute.c lines 38-41, 61-62): on
    // Unix they are plain names, searched along the path. Not valid file
    // names on Windows, where this test never compares with kpsewhich anyway
    // (its path separator is `:` here).
    let unix_only = ["c:tx.tex", "\\tb.tex", ".\\tdot.tex"];
    if cfg!(unix) {
        for f in unix_only {
            std::fs::write(d.join(f), &tfm).unwrap();
        }
    }

    let abs = |n: &str| d.join(n).to_string_lossy().into_owned();
    let tfm_names: Vec<String> = [
        "zzbare",
        "plain",
        "plain.tfm",
        "foo.bar",
        "foo",
        "baz.qux",
        "baz.qux.tfm",
        "both",
        "both.tfm",
        "pair.bar",
        "noext.tfm",
        "dot.",
        "dot",
        "./zzbare",
        "./plain",
        "./plain.tfm",
        "sub/subbare",
        "sub/subt",
        "sub/subt.tfm",
        "./sub/subbare",
        "./sub/subt",
        "../work/zzbare",
        "../work/plain",
        "inpath",
        "inpath2",
        "inpath2.tfm",
        // Explicitly relative: kpathsea never searches the path for them
        // (`kpse_absolute_p` is true, pathsearch.c `search`), so a file
        // only in the path directory is not found.
        "./inpath2",
        "./inpath2.tfm",
        "./inpath",
        "psub/q",
        "./psub/q",
        "../tfmpath/inpath2",
    ]
    .iter()
    .map(|s| s.to_string())
    .chain(
        [
            "zzbare",
            "plain",
            "plain.tfm",
            "foo.bar",
            "both",
            "sub/subbare",
            "sub/subt",
        ]
        .iter()
        .map(|n| abs(n)),
    )
    .collect();
    let mut tex_names = vec![
        "tin", "tin.tex", "tbare", "tdot.bar", "tdot", "./tbare", "sub/tsub", "tpath", "./tpath",
    ];
    if cfg!(unix) {
        tex_names.extend(unix_only);
    }

    std::env::set_current_dir(&d).unwrap();
    std::env::set_var("FLASHTEX_TFM_PATH", &p);
    std::env::set_var("FLASHTEX_INPUTS", &p);
    let mut bad = Vec::new();
    let cases = tfm_names
        .iter()
        .map(|n| (n.as_str(), Format::Tfm))
        .chain(tex_names.iter().map(|n| (*n, Format::Tex)));
    for (name, f) in cases {
        let want = kpsewhich(&bin, &d, &p, name, f);
        // `cwd-kpse`: kpathsea's own spelling of the path.
        let got = show(CwdResolver { dot: true }.find(name, f));
        if got != want {
            bad.push(format!(
                "cwd-kpse {} {name:?}: got {got:?}, kpsewhich {want:?}",
                f.kpse_name()
            ));
        }
        // `cwd`: the same file, spelled as given.
        let plain = CwdResolver { dot: false }.find(name, f);
        let same = match (&plain, want.is_empty()) {
            (None, true) => true,
            (Some(g), false) => g.canonicalize().ok() == Path::new(&want).canonicalize().ok(),
            _ => false,
        };
        if !same {
            bad.push(format!(
                "cwd {} {name:?}: got {:?}, kpsewhich {want:?}",
                f.kpse_name(),
                show(plain)
            ));
        }
    }

    // The engine itself: `\font` under both resolvers, as pdfTeX loads it.
    let font_names = [
        "zzbare",
        "plain",
        "plain.tfm",
        "foo.bar",
        "both",
        "pair.bar",
        "./zzbare",
        "./plain",
        "sub/subbare",
        "sub/subt",
        "inpath",
        "inpath2",
        "./inpath2",
        "psub/q",
        "./psub/q",
    ];
    let mut tex = Command::new(bin.join("pdftex"));
    tex.env("TFMFONTS", format!(".:{}", p.display()));
    let want = font_messages(&d, &font_names, &mut tex);
    assert_eq!(want.len(), font_names.len(), "pdftex messages: {want:?}");
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    for resolver in ["cwd", "cwd-kpse"] {
        let mut ours = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"));
        ours.env("FLASHTEX_POOL", &pool)
            .env("FLASHTEX_RESOLVER", resolver)
            .env("FLASHTEX_TFM_PATH", &p);
        let got = font_messages(&d, &font_names, &mut ours);
        if got != want {
            bad.push(format!(
                "\\font, FLASHTEX_RESOLVER={resolver}: got {got:?}, pdftex {want:?}"
            ));
        }
    }

    std::env::set_current_dir(std::env::temp_dir()).unwrap();
    let _ = std::fs::remove_dir_all(&root);
    assert!(
        bad.is_empty(),
        "{} differences:\n{}",
        bad.len(),
        bad.join("\n")
    );
}
