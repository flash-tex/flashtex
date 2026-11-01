//! `\XeTeXlinebreaklocale` without TeX Live: a native word cut by ICU's line
//! breaker (src/native/linebreak.rs), and the word merged with the one
//! before it, the path that panicked on polyglossia's manual (lane
//! M3-PANIC). With no line breaker, `do_locale_linebreaks` appended nothing:
//! the merge in `main_control` (xetex.web's "remove the preceding node from
//! the list") then flushed the previous word, left `tail` at the freed node
//! and walked `link`s from `head` past the list's end, reading `mem[null]`.
//!
//! The box dumps are TeX Live 2026's `xetex -ini -etex -no-pdf` log for the
//! same input (ICU 78.2, verified 2026-10-12). The font is Latin Modern
//! Roman, the copy apps/mac bundles (the same file as TeX Live's).

use std::path::{Path, PathBuf};
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2 \showboxdepth=100 \showboxbreadth=100
\hsize=100pt \parfillskip=0pt plus 1fil \tolerance=10000 \parindent=0pt
\font\x="[lmroman10-regular.otf]" \x
\setbox0\vbox{\noindent\hbox{}ab\XeTeXlinebreaklocale "zh" cd ef-gh/ij\par}
\showbox0
\setbox0\vbox{\XeTeXlinebreaklocale "ja" \XeTeXlinebreakpenalty=7
\XeTeXlinebreakskip=0pt plus 1pt \noindent ab-cd\par}
\showbox0
\end
"#;

/// xetex's two box dumps, each from `> \box0=` to its last node.
const WANT: [&[&str]; 2] = [
    &[
        "> \\box0=",
        "\\vbox(14.22313+0.0)x100.0",
        ".\\hbox(11.31227+2.91086)x100.0, glue set 43.38846fil",
        "..\\hbox(0.0+0.0)x0.0",
        "..\\x abcd",
        "..\\glue 3.34248 plus 1.67123 minus 1.11415",
        "..\\x ef-",
        "..\\penalty 0",
        "..\\x gh/",
        "..\\penalty 0",
        "..\\x ij",
        "..\\penalty 10000",
        "..\\glue(\\parfillskip) 0.0 plus 1.0fil",
        "..\\glue(\\rightskip) 0.0",
    ],
    &[
        "> \\box0=",
        "\\vbox(14.22313+0.0)x100.0",
        ".\\hbox(11.31227+2.91086)x100.0, glue set 76.0204fil",
        "..\\x ab-",
        "..\\penalty 7",
        "..\\glue(\\XeTeXlinebreakskip) 0.0 plus 1.0",
        "..\\x cd",
        "..\\penalty 10000",
        "..\\glue(\\parfillskip) 0.0 plus 1.0fil",
        "..\\glue(\\rightskip) 0.0",
    ],
];

fn run_dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("flashtex-xetex-linebreak-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn locale_breaks_and_the_merged_word_match_xetex() {
    let dir = run_dir();
    let font =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts/lmroman10-regular.otf");
    std::fs::copy(&font, dir.join("lmroman10-regular.otf")).unwrap();
    std::fs::write(dir.join("lb.tex"), INPUT).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(&dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_RESOLVER", "cwd")
        .args([
            "-ini",
            "-etex",
            "-no-pdf",
            "-interaction=nonstopmode",
            "lb.tex",
        ])
        .output()
        .expect("run flashtex-xetex");
    let stderr = String::from_utf8_lossy(&out.stderr);
    // \showbox in nonstop mode is an error: exit status 1, as xetex's (the
    // panic was 101).
    assert_eq!(out.status.code(), Some(1), "stderr:\n{stderr}");
    let log = std::fs::read_to_string(dir.join("lb.log")).unwrap();
    let lines: Vec<&str> = log.lines().collect();
    let starts: Vec<usize> = (0..lines.len())
        .filter(|&i| lines[i] == "> \\box0=")
        .collect();
    assert_eq!(starts.len(), 2, "log:\n{log}");
    for (k, want) in WANT.iter().enumerate() {
        let got = &lines[starts[k]..(starts[k] + want.len()).min(lines.len())];
        assert_eq!(got, *want, "box {k}; log:\n{log}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
