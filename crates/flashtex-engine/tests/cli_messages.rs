//! texmfmp.c's command-line messages, against TeX Live 2026's pdfTeX
//! (x86_64-linux; the same words on macOS arm64, where getopt quotes an
//! option `like this'): a bad `-interaction` or `-output-format` value is
//! kpathsea's WARNING1 (`warning: `, the message, `.`), not a message
//! after the program's name; an unrecognized option is named as it was
//! given, its `=value` included. No TeX installation needed.

use std::path::Path;
use std::process::{Command, Stdio};

fn stderr_of(args: &[&str]) -> (Option<i32>, Vec<String>) {
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
        .args(args)
        .env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        )
        .env("FLASHTEX_RESOLVER", "cwd")
        .stdin(Stdio::null())
        .output()
        .expect("run flashtex-initex");
    let err = String::from_utf8_lossy(&out.stderr)
        .lines()
        .map(str::to_string)
        .collect();
    (out.status.code(), err)
}

#[test]
fn bad_values_are_kpathsea_warnings() {
    // pdftex -interaction=bogus -output-format=bogus -version
    let (code, err) = stderr_of(&["-interaction=bogus", "-output-format=bogus", "-version"]);
    assert_eq!(code, Some(0));
    assert_eq!(
        err,
        [
            "warning: Ignoring unknown argument `bogus' to --interaction.",
            "warning: Ignoring unknown value `bogus' for --output-format.",
        ]
    );
}

#[test]
fn an_unrecognized_option_is_named_as_given() {
    // pdftex -foo=bar --foo -version
    let (code, err) = stderr_of(&["-foo=bar", "--foo", "-version"]);
    assert_eq!(code, Some(0));
    let tails: Vec<&str> = err
        .iter()
        .map(|l| l.split_once(": ").map_or(l.as_str(), |(_, t)| t))
        .collect();
    assert_eq!(
        tails,
        [
            "unrecognized option '-foo=bar'",
            "unrecognized option '--foo'"
        ]
    );
}
