//! XeTeX's command line, not pdfTeX's: `-help` is texmfmp-help.h's
//! XETEXHELP (`cli-xetex-help.txt` is TeX Live 2026's `xetex --help`,
//! byte for byte; the port prints it without the options it refuses, as
//! the pdfTeX engine prints pdfTeX's), `-version` the port's own text, and pdfTeX's options
//! that XeTeX lacks (`-enc`, `-draftmode`, `-output-format`, ...) are
//! unrecognized, as TeX Live's xetex reports them (x86_64-linux:
//! `xetex: unrecognized option '-enc'`, and the run goes on). The port
//! printed pdfTeX's help and version and refused `-enc` as "not
//! implemented". No TeX installation needed.

use std::process::{Command, Stdio};

fn run(args: &[&str]) -> (Option<i32>, String, Vec<String>) {
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .args(args)
        .env("FLASHTEX_RESOLVER", "cwd")
        .stdin(Stdio::null())
        .output()
        .expect("run flashtex-xetex");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr)
            .lines()
            .map(|l| l.split_once(": ").map_or(l, |(_, t)| t).to_string())
            .collect(),
    )
}

/// TeX Live's text as the engines print theirs (the pdfTeX engine's `HELP`
/// is pdfTeX's so): without the options this engine refuses, whose names
/// replace the last line (the address for bug reports).
fn expected_help() -> String {
    let tl = include_str!("cli-xetex-help.txt");
    let refused = [
        "-kpathsea-debug",
        "[-no]-mktex",
        "-mltex",
        "-src-specials",
        "-synctex",
    ];
    let mut out = String::new();
    let mut skipping = false;
    for line in tl.lines() {
        if line.starts_with('-') || line.starts_with('[') || line.is_empty() {
            skipping = refused.iter().any(|r| {
                line.starts_with(r) && line[r.len()..].starts_with(|c: char| c == ' ' || c == '=')
            });
        }
        if skipping || line.starts_with("Email bug reports") {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(
        "Not implemented by this engine yet: -kpathsea-debug, -mktex, -no-mktex,\n\
         -mltex, -src-specials, -synctex.\n",
    );
    out
}

#[test]
fn help_is_xetexs() {
    let (code, out, err) = run(&["--help"]);
    assert_eq!(code, Some(0), "{err:?}");
    assert_eq!(out, expected_help());
}

#[test]
fn version_is_xetexs() {
    let (code, out, _) = run(&["-version"]);
    assert_eq!(code, Some(0));
    assert!(out.starts_with("XeTeX 3.141592653-2.6-0.999998 "), "{out}");
}

#[test]
fn pdftex_only_options_are_unrecognized() {
    // xetex -enc -draftmode -output-format=pdf -version
    let (code, out, err) = run(&["-enc", "-draftmode", "-output-format=pdf", "-version"]);
    assert_eq!(code, Some(0), "{err:?}");
    assert_eq!(
        err,
        [
            "unrecognized option '-enc'",
            "unrecognized option '-draftmode'",
            "unrecognized option '-output-format=pdf'",
        ]
    );
    assert!(out.starts_with("XeTeX "), "{out}");
}
