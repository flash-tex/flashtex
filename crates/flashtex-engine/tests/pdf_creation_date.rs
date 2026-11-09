//! `\pdfcreationdate` (texmfmp.c's `makepdftime`) for years past 9999 and
//! before 1000, from `SOURCE_DATE_EPOCH` with `FORCE_SOURCE_DATE=1`.
//!
//! makepdftime's "correction for seconds" looks at index 14 of
//! `D:%Y%m%d%H%M%S` (the first digit of the seconds with a four-digit year)
//! and, finding a `6`, writes `59` and ends the string there; the time zone
//! then goes at the old end, after the new one. From the year 10000 that
//! index is a digit of the minutes. FlashTeX clamped the seconds instead.
//!
//! The years from 10000 are compared live with the installed pdftex, which
//! measured the same on TeX Live 2026 macOS arm64 and NixOS. The years before
//! 1000 are `%Y` as macOS's C library writes it, padded to four digits (the
//! §13 ruling; glibc's has no padding), so they are pinned to pdfTeX
//! 1.40.29's macOS arm64 output.
#![cfg(all(unix, feature = "kpathsea"))]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

const DOC: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\message{<<\\pdfcreationdate>>}\\end\n";

/// The `<<...>>` of `\pdfcreationdate` at `SOURCE_DATE_EPOCH=epoch`.
fn shown(ours: bool, pdftex: &Path, dir: &Path, epoch: &str) -> String {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("d.tex"), DOC).unwrap();
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
    let st = c
        .args(["-ini", "-interaction=nonstopmode", "d.tex"])
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", epoch)
        .env("FORCE_SOURCE_DATE", "1")
        .env("TZ", "UTC")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(st.success(), "{epoch}: exit {st:?} (ours: {ours})");
    let log = std::fs::read_to_string(dir.join("d.log")).unwrap_or_default();
    let a = log.find("<<").expect("no <<") + 2;
    let b = a + log[a..].find(">>").expect("no >>");
    log[a..b].to_string()
}

#[test]
fn creation_dates_past_9999_match_pdftex() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let pdftex = texbin.join("pdftex");
    let base = common::fresh_dir("flashtex-creationdate");
    let cases = [
        ("0", "D:19700101000000Z"),
        ("253402300800", "D:100000101000000Z"),
        // 10000-01-01 00:06:00 and 00:06:10: a minute ending in 6
        ("253402301160", "D:10000010100059"),
        ("253402301170", "D:10000010100059"),
        ("253402304760", "D:10000010101059"),
        ("99999999999999", "D:31708431107094639Z"),
    ];
    for (epoch, want) in cases {
        let tex = shown(false, &pdftex, &base.join(format!("tex-{epoch}")), epoch);
        assert_eq!(tex, want, "{epoch}: pdftex");
        let ours = shown(true, &pdftex, &base.join(format!("ours-{epoch}")), epoch);
        assert_eq!(ours, tex, "{epoch}: ours vs pdftex");
    }
    // `%Y` padded to four digits: macOS arm64 pdfTeX 1.40.29's output
    for (epoch, want) in [
        ("-60000000000", "D:00680903132000Z"),
        ("-62167219200", "D:00000101000000Z"),
        ("-62198755200", "D:-0010101000000Z"),
        ("-100000000000", "D:-11990215141320Z"),
    ] {
        let ours = shown(true, &pdftex, &base.join(format!("ours-{epoch}")), epoch);
        assert_eq!(ours, want, "{epoch}");
    }
    let _ = std::fs::remove_dir_all(&base);
}
