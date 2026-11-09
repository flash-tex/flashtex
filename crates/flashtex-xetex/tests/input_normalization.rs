//! `\XeTeXinputnormalization`: XeTeX_ext.c's `input_line` passes each line
//! it reads through TECkit's normalizer (`apply_normalization`) when the
//! parameter is 1 (NFC) or 2 (NFD), and leaves it alone for any other value.
//! The port had the normalizer (`Globals::normalize_utf32`) but never called
//! it, so a document that set the parameter read its lines unnormalized
//! (found by tools/fuzz-unicode).
//!
//! The expected messages are TeX Live 2026's `xetex -ini -etex -no-pdf`
//! log for the same input (verified 2026-11-09, MacTeX 2026): `é` (U+00E9)
//! decomposes to `e` U+0301 under NFD, `ḕ` (U+1E15) to `e` U+0304 U+0300;
//! `e` U+0301 composes to U+00E9 under NFC and `e` U+0304 U+0300 to U+1E15;
//! the values 3 and 0 leave the line as it is.

use std::path::PathBuf;
use std::process::Command;

const INPUT: &str = "\\catcode`\\{=1 \\catcode`\\}=2
\\message{[\\number`\u{e9}]}
\\XeTeXinputnormalization=2
\\message{[\\number`\u{e9}][\\number`\u{1e15}]}
\\XeTeXinputnormalization=1
\\message{[\\number`e\u{301}][\\number`e\u{304}\u{300}]}
\\XeTeXinputnormalization=3
\\message{[\\number`\u{e9}][\\number`e\u{301}]}
\\XeTeXinputnormalization=0
\\message{[\\number`e\u{301}]}
\\end
";

/// xetex's log line holding the messages (after `(./norm.tex `; the run
/// here prints the name as the cwd resolver gives it, `(norm.tex `).
const WANT: &str =
    "[233] [101\u{301}][101\u{304}\u{300}] [233][7701] [233][101\u{301}] [101\u{301}] )";

fn run_dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("flashtex-xetex-norm-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn input_lines_are_normalized_as_xetex_does() {
    let dir = run_dir();
    std::fs::write(dir.join("norm.tex"), INPUT).unwrap();
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
            "norm.tex",
        ])
        .output()
        .expect("run flashtex-xetex");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "stderr:\n{stderr}");
    let log = std::fs::read_to_string(dir.join("norm.log")).unwrap();
    let line = log
        .lines()
        .find_map(|l| l.split_once("norm.tex ").map(|(_, rest)| rest))
        .unwrap_or_else(|| panic!("no input line in the log:\n{log}"));
    assert_eq!(line, WANT, "log:\n{log}");
    let _ = std::fs::remove_dir_all(&dir);
}
