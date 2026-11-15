//! `\pdffiledump` (texmfmp.c's `getfiledump`) against the installed TeX
//! Live's `pdftex -ini`, run at test time on the same `t.tex`: what the
//! document shows between `<<` and `>>`, the exit status, and whether the
//! run ran out of pool.
//!
//! The lengths from about 2^30 up are C's: `poolptr + 2 * length + 1` wraps
//! negative in an `int`, so the room test passes and `fread` gets an address
//! far past the string pool; it reads nothing there (EFAULT), and the result
//! is empty. Measured with pdfTeX 1.40.29 (TeX Live 2026, macOS arm64, the
//! §13 ruling) and 1.40.27 (NixOS): `<<>>` for 1073741823, 2000000000 and
//! 2147483647; 1000000000 does not wrap and is `TeX capacity exceeded`.
//! With an offset short of the file's end, macOS's pdfTeX crashes there
//! (exit 138/139, stdio copying its buffer to that address) and Linux's is
//! `<<>>`; a crash is not replicated (§13), so FlashTeX is `<<>>` too.
//! FlashTeX used to read the file in all of these (release) or panic on the
//! overflow (debug).
#![cfg(all(unix, feature = "kpathsea"))]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

/// What a run shows: exit status, the `<<...>>` texts, and whether the log
/// says the pool ran out.
type Seen = (Option<i32>, Vec<String>, bool);

fn run(ours: bool, pdftex: &Path, dir: &Path) -> Seen {
    let mut c = if ours {
        let mut c = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"));
        c.env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        );
        c
    } else {
        Command::new(pdftex)
    };
    let status = c
        .args(["-ini", "-interaction=nonstopmode", "t.tex"])
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    let log = std::fs::read_to_string(dir.join("t.log")).unwrap_or_default();
    let shown = log
        .split("<<")
        .skip(1)
        .filter_map(|s| s.split_once(">>").map(|(a, _)| a.to_string()))
        .collect();
    (status.code(), shown, log.contains("TeX capacity exceeded"))
}

/// `\pdffiledump` with `keys` on `file`, in both engines.
fn both(keys: &str, file: &str) -> (Seen, Seen) {
    let texbin = find_texlive_bin().unwrap();
    let base = common::fresh_dir("flashtex-filedump");
    let tex = format!(
        "\\catcode`\\{{=1 \\catcode`\\}}=2 \\message{{<<\\pdffiledump {keys}{{{file}}}>>}}\\end\n"
    );
    let mut seen = vec![];
    for ours in [true, false] {
        let d = base.join(if ours { "ours" } else { "tex" });
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("f.txt"), "hello\n").unwrap();
        std::fs::write(d.join("t.tex"), &tex).unwrap();
        seen.push(run(ours, &texbin.join("pdftex"), &d));
    }
    let _ = std::fs::remove_dir_all(&base);
    let tex = seen.pop().unwrap();
    (seen.pop().unwrap(), tex)
}

#[test]
fn file_dumps_match_pdftex() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let cases: &[(&str, &str, Option<&str>)] = &[
        // (keys, file, the result where it is fixed by measurement)
        ("length 3", "f.txt", Some("68656C")),
        ("offset 2 length 3", "f.txt", Some("6C6C6F")),
        ("offset 4 length 100", "f.txt", Some("6F0A")),
        ("offset 100 length 10", "f.txt", Some("")),
        ("length 0", "f.txt", Some("")),
        ("length 3", "nosuch.txt", Some("")),
        ("length 1000000000", "f.txt", None),
        ("length 1073741823", "f.txt", Some("")),
        ("offset 6 length 2000000000", "f.txt", Some("")),
        ("length 2147483647", "f.txt", Some("")),
        ("length 2147483647", "nosuch.txt", Some("")),
    ];
    for (keys, file, want) in cases {
        let (ours, tex) = both(keys, file);
        let what = format!("\\pdffiledump {keys}{{{file}}}");
        if let Some(w) = want {
            assert_eq!(tex.1, [w.to_string()], "{what}: pdftex shows {tex:?}");
            assert!(!tex.2 && tex.0 == Some(0), "{what}: pdftex {tex:?}");
        } else {
            assert!(tex.2 && tex.0 == Some(1), "{what}: pdftex {tex:?}");
        }
        assert_eq!(ours, tex, "{what}: ours vs pdftex");
    }
    // Where macOS's pdfTeX crashes: Linux's pdfTeX's result, and no crash.
    for keys in ["offset 1 length 2000000000", "offset 5 length 1073741823"] {
        let (ours, _) = both(keys, "f.txt");
        assert_eq!(ours, (Some(0), vec![String::new()], false), "{keys}");
    }
}
