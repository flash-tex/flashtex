//! `SOURCE_DATE_EPOCH` / `FORCE_SOURCE_DATE` against hard pdftex numbers.
//!
//! Every expected value below was measured from the installed TeX Live pdftex
//! (pdfTeX 3.141592653-2.6-1.40.29, TeX Live 2026) on macOS on 2026-11-01,
//! each case in a fresh dir under `$HOME/tmp` as
//! `perl -e 'alarm 20; exec @ARGV' /Library/TeX/texbin/pdftex -ini t.tex`
//! with stdin `/dev/null` (locale `en_US.UTF-8`, `TZ` unset, system zone PST).
//! Only FlashTeX runs here, with the same document bytes and arguments, so
//! the file needs no TeX Live and has no feature gate. Every run sets `TZ`
//! (`PST8PDT`, a POSIX rule that needs no zoneinfo files, unless the case
//! sets its own): a hosted runner's zone is UTC, where a clock date is
//! `D:...Z` with no numeric offset, so the measured zone is set, not assumed.
//!
//! The document prints `[Y \the\year-\the\month-\the\day \the\time]` (the
//! space after `\the\day` dies as the control-word terminator, so e.g.
//! `2023-11-141333`) and `[P \pdfcreationdate]`. Pinned runs assert both
//! strings exactly; clock runs assert their shape (a `20xx` year, a numeric
//! `D:...` offset). stderr is compared as RAW BYTES: pdftex echoes a
//! non-UTF-8 value's bytes, as does this PR's code.
//!
//! The fatal's `<argv[0]>` prefix is the engine path this test invoked, not
//! `/Library/TeX/texbin/pdftex`; everything after it is the measured text.
//! The PR's own `texch_source_date.rs` compares the engines live with other
//! documents and `-interaction=nonstopmode`; the environment values overlap
//! by design, but the past-`ULLONG_MAX` value (`...+16`, not its `999...9`)
//! and the `FORCE_SOURCE_DATE` sweep's epoch (`1700000000`, not its `0`)
//! differ, so no case here is asserted exactly there.

mod common;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The oracle document, byte for byte.
const DOC: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\message{[Y \\the\\year-\\the\\month-\\the\\day \\the\\time][P \\pdfcreationdate]}\\end\n";

/// A valid epoch that pins the date (2023-11-14 22:13:20 UTC).
const PINNED: &str = "1700000000";
/// What that epoch prints as: `Y 2023-11-14`, `\time` 1333, and creation date.
const PINNED_Y: &str = "2023-11-141333";
const PINNED_P: &str = "D:20231114221320Z";

struct Engine {
    dir: PathBuf,
    exe: PathBuf,
}

impl Engine {
    fn new(name: &str) -> Engine {
        let dir = common::fresh_dir(&format!("flashtex-sde-oracle-{name}"));
        let exe = dir.join("pdftex");
        common::link_engine(Path::new(env!("CARGO_BIN_EXE_flashtex-initex")), &exe);
        Engine { dir, exe }
    }

    /// `pdftex -ini t.tex </dev/null` with exactly the environment `env`
    /// (both date variables start unset): exit status, stdout, stderr.
    fn run(&self, env: &[(&str, &OsStr)]) -> (Option<i32>, Vec<u8>, Vec<u8>) {
        std::fs::write(self.dir.join("t.tex"), DOC).unwrap();
        let mut c = Command::new(&self.exe);
        c.args(["-ini", "t.tex"])
            .current_dir(&self.dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        c.env_remove("SOURCE_DATE_EPOCH")
            .env_remove("FORCE_SOURCE_DATE")
            .env("TZ", "PST8PDT");
        for (k, v) in env {
            c.env(k, v);
        }
        // `initex.rs`' hermetic setup: no TeX Live needed.
        c.env("FLASHTEX_RESOLVER", "cwd").env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        );
        let o = c.output().unwrap();
        (o.status.code(), o.stdout, o.stderr)
    }

    /// pdftex's fatal with our `argv[0]`: `<exe>: fatal: ... $NAME: <v>.\n`.
    fn fatal(&self, var: &str, value: &[u8]) -> Vec<u8> {
        let mut e = self.exe.as_os_str().as_encoded_bytes().to_vec();
        e.extend_from_slice(
            format!(
                ": fatal: invalid epoch-seconds-timezone value for environment variable ${var}: "
            )
            .as_bytes(),
        );
        e.extend_from_slice(value);
        e.extend_from_slice(b".\n");
        e
    }
}

/// pdftex's `FORCE_SOURCE_DATE` warning for the raw value, once.
fn warning(value: &[u8]) -> Vec<u8> {
    let mut w =
        b"warning: invalid value (expected 0 or 1) for environment variable $FORCE_SOURCE_DATE: "
            .to_vec();
    w.extend_from_slice(value);
    w.extend_from_slice(b".\n");
    w
}

/// The `[Y ...][P ...]` the document printed on the terminal.
fn yp(stdout: &[u8]) -> (String, String) {
    let s = String::from_utf8_lossy(stdout);
    let (_, rest) = s.split_once("[Y ").expect("no [Y marker on stdout");
    let (y, rest) = rest.split_once("][P ").expect("no ][P marker on stdout");
    let (p, _) = rest.split_once(']').expect("no P end on stdout");
    (y.to_string(), p.to_string())
}

/// `Y` from the living clock: a `20xx` year, two dashes, digits otherwise.
fn assert_clock_y(y: &str) {
    assert!(
        y.starts_with("20")
            && (9..=14).contains(&y.len())
            && y.chars().filter(|c| *c == '-').count() == 2
            && y.chars().all(|c| c.is_ascii_digit() || c == '-'),
        "clock Y shape: {y:?}"
    );
}

/// `P` from the living clock: `D:` + 14 digits + a numeric offset.
fn assert_local_p(p: &str) {
    let b = p.as_bytes();
    assert!(
        p.starts_with("D:20")
            && b.len() == 23
            && b[2..16].iter().all(|c| c.is_ascii_digit())
            && (b[16] == b'+' || b[16] == b'-')
            && b[17..19].iter().all(|c| c.is_ascii_digit())
            && b[19] == b'\''
            && b[20..22].iter().all(|c| c.is_ascii_digit())
            && b[22] == b'\'',
        "local P shape: {p:?}"
    );
}

/// `P` from the living clock in UTC: `D:` + 14 digits + `Z`, as pdfTeX
/// writes an offset of 0 (what a hosted runner with `TZ` unset prints).
fn assert_utc_p(p: &str) {
    let b = p.as_bytes();
    assert!(
        p.starts_with("D:20")
            && b.len() == 17
            && b[2..16].iter().all(|c| c.is_ascii_digit())
            && b[16] == b'Z',
        "UTC P shape: {p:?}"
    );
}

#[test]
fn sde_unset_in_utc_has_no_numeric_offset() {
    let e = Engine::new("utc");
    let (code, stdout, stderr) = e.run(&[("TZ", OsStr::new("UTC0"))]);
    assert_eq!(code, Some(0));
    assert_eq!(stderr, b"", "stderr");
    let (y, p) = yp(&stdout);
    assert_clock_y(&y);
    assert_utc_p(&p);
}

#[test]
fn sde_unset_uses_local_clock() {
    // pdftex: exit 0, empty stderr, `[Y 2026-11-11250][P D:20261101205023-08'00']`.
    let e = Engine::new("unset");
    let (code, stdout, stderr) = e.run(&[]);
    assert_eq!(code, Some(0));
    assert_eq!(stderr, b"", "stderr");
    let (y, p) = yp(&stdout);
    assert_clock_y(&y);
    assert_local_p(&p);
}

// macOS's `strtoull("")` is `EINVAL`; glibc's reads the epoch, so Linux
// pdftex exits 0 here. The PR's live-comparison test covers "" everywhere.
#[cfg(target_os = "macos")]
#[test]
fn sde_empty_is_fatal() {
    // pdftex: exit 1, `...$SOURCE_DATE_EPOCH: .\n` (119 bytes), empty stdout.
    let e = Engine::new("empty");
    let (code, stdout, stderr) = e.run(&[("SOURCE_DATE_EPOCH", OsStr::new(""))]);
    assert_eq!(code, Some(1));
    assert_eq!(stderr, e.fatal("SOURCE_DATE_EPOCH", b""), "stderr");
    assert!(!String::from_utf8_lossy(&stdout).contains("[Y "));
}

#[test]
fn sde_zero_and_seven_forms_pin_creation_date() {
    // pdftex: exit 0, empty stderr; `0` -> `D:19700101000000Z`, the three
    // readings of 7 (` 7`, `+7`, `007`) -> `D:19700101000007Z`; `Y` is clock.
    let e = Engine::new("small");
    for (v, p) in [
        ("0", "D:19700101000000Z"),
        (" 7", "D:19700101000007Z"),
        ("+7", "D:19700101000007Z"),
        ("007", "D:19700101000007Z"),
    ] {
        let (code, stdout, stderr) = e.run(&[("SOURCE_DATE_EPOCH", OsStr::new(v))]);
        assert_eq!(code, Some(0), "SDE={v:?}");
        assert_eq!(stderr, b"", "SDE={v:?} stderr");
        let (y, got) = yp(&stdout);
        assert_clock_y(&y);
        assert_eq!(got, p, "SDE={v:?}");
    }
}

#[test]
fn sde_1700000000_pins_creation_date() {
    // pdftex: exit 0, empty stderr, `P D:20231114221320Z`, `Y` is clock:
    // the epoch alone does not pin `\year` (that needs `FORCE_SOURCE_DATE=1`).
    let e = Engine::new("pinned");
    let (code, stdout, stderr) = e.run(&[("SOURCE_DATE_EPOCH", OsStr::new(PINNED))]);
    assert_eq!(code, Some(0));
    assert_eq!(stderr, b"", "stderr");
    let (y, p) = yp(&stdout);
    assert_clock_y(&y);
    assert_eq!(p, PINNED_P);
}

#[test]
fn sde_negative_and_ullong_max_wrap() {
    // pdftex: exit 0, empty stderr; `-5` wraps to -5 (`D:19691231235955Z`)
    // and `ULLONG_MAX` to -1 (`D:19691231235959Z`); `Y` is clock.
    let e = Engine::new("wrap");
    for (v, p) in [
        ("-5", "D:19691231235955Z"),
        ("18446744073709551615", "D:19691231235959Z"),
    ] {
        let (code, stdout, stderr) = e.run(&[("SOURCE_DATE_EPOCH", OsStr::new(v))]);
        assert_eq!(code, Some(0), "SDE={v:?}");
        assert_eq!(stderr, b"", "SDE={v:?} stderr");
        let (y, got) = yp(&stdout);
        assert_clock_y(&y);
        assert_eq!(got, p, "SDE={v:?}");
    }
}

#[test]
fn sde_trailing_text_and_overflow_are_fatal() {
    // pdftex: exit 1, `...$SOURCE_DATE_EPOCH: 12x.\n` (122 bytes) and
    // `...: 18446744073709551616.\n` (139 bytes); empty stdout.
    let e = Engine::new("fatal");
    for v in ["12x", "18446744073709551616"] {
        let (code, stdout, stderr) = e.run(&[("SOURCE_DATE_EPOCH", OsStr::new(v))]);
        assert_eq!(code, Some(1), "SDE={v:?}");
        assert_eq!(
            stderr,
            e.fatal("SOURCE_DATE_EPOCH", v.as_bytes()),
            "SDE={v:?}"
        );
        assert!(!String::from_utf8_lossy(&stdout).contains("[Y "));
    }
}

#[cfg(unix)]
#[test]
fn sde_non_utf8_bytes_are_fatal_and_echoed_raw() {
    // pdftex: exit 1, `...$SOURCE_DATE_EPOCH: \xff\xfe.\n` with the RAW
    // bytes ff fe (121 bytes total), empty stdout.
    use std::os::unix::ffi::OsStrExt;
    let e = Engine::new("sde-bytes");
    let v = OsStr::from_bytes(b"\xff\xfe");
    let (code, stdout, stderr) = e.run(&[("SOURCE_DATE_EPOCH", v)]);
    assert_eq!(code, Some(1));
    assert_eq!(stderr, e.fatal("SOURCE_DATE_EPOCH", b"\xff\xfe"), "stderr");
    assert!(!String::from_utf8_lossy(&stdout).contains("[Y "));
}

#[test]
fn force_valid_values_with_pinned_epoch() {
    // pdftex with SDE=1700000000: exit 0, empty stderr, `P` pinned; `Y` is
    // the clock except for `FORCE_SOURCE_DATE=1`, which pins UTC (`Y
    // 2023-11-141333`). Unset, `0`, `1` and empty all behave.
    let e = Engine::new("force-valid");
    for f in [None, Some("0"), Some("1"), Some("")] {
        let mut env: Vec<(&str, &OsStr)> = vec![("SOURCE_DATE_EPOCH", OsStr::new(PINNED))];
        if let Some(f) = f {
            env.push(("FORCE_SOURCE_DATE", OsStr::new(f)));
        }
        let (code, stdout, stderr) = e.run(&env);
        assert_eq!(code, Some(0), "FORCE={f:?}");
        assert_eq!(stderr, b"", "FORCE={f:?} stderr");
        let (y, p) = yp(&stdout);
        assert_eq!(p, PINNED_P, "FORCE={f:?}");
        if f == Some("1") {
            assert_eq!(y, PINNED_Y, "FORCE={f:?}");
        } else {
            assert_clock_y(&y);
        }
    }
}

#[test]
fn force_invalid_values_warn_twice_and_use_clock() {
    // pdftex with SDE=1700000000: exit 0, the warning TWICE on stderr (89
    // bytes each: `...$FORCE_SOURCE_DATE: 2.\n`), `P` still pinned, `Y` clock.
    let e = Engine::new("force-warn");
    for v in ["2", "abc"] {
        let (code, stdout, stderr) = e.run(&[
            ("SOURCE_DATE_EPOCH", OsStr::new(PINNED)),
            ("FORCE_SOURCE_DATE", OsStr::new(v)),
        ]);
        assert_eq!(code, Some(0), "FORCE={v:?}");
        assert_eq!(
            stderr,
            [warning(v.as_bytes()), warning(v.as_bytes())].concat()
        );
        let (y, p) = yp(&stdout);
        assert_clock_y(&y);
        assert_eq!(p, PINNED_P, "FORCE={v:?}");
    }
}

#[cfg(unix)]
#[test]
fn force_non_utf8_byte_warns_and_echoes_raw() {
    // pdftex: exit 0, the warning twice with the RAW byte ff (89 bytes
    // each: `...$FORCE_SOURCE_DATE: \xff.\n`), whether SDE pins the date
    // (`P D:20231114221320Z`) or is unset (`P` local); `Y` is clock.
    use std::os::unix::ffi::OsStrExt;
    let e = Engine::new("force-bytes");
    let f = OsStr::from_bytes(b"\xff");
    let pinned = [
        ("SOURCE_DATE_EPOCH", OsStr::new(PINNED)),
        ("FORCE_SOURCE_DATE", f),
    ];
    let (code, stdout, stderr) = e.run(&pinned);
    assert_eq!(code, Some(0));
    assert_eq!(
        stderr,
        [warning(b"\xff"), warning(b"\xff")].concat(),
        "stderr"
    );
    let (y, p) = yp(&stdout);
    assert_clock_y(&y);
    assert_eq!(p, PINNED_P);
    let (code, stdout, stderr) = e.run(&[("FORCE_SOURCE_DATE", f)]);
    assert_eq!(code, Some(0));
    assert_eq!(
        stderr,
        [warning(b"\xff"), warning(b"\xff")].concat(),
        "stderr"
    );
    let (y, p) = yp(&stdout);
    assert_clock_y(&y);
    assert_local_p(&p);
}

#[test]
fn force_with_sde_unset() {
    // pdftex with SDE unset: exit 0; empty stderr for `0`, `1` and empty,
    // the warning twice for `2` and `abc`. `P` is local clock throughout;
    // `Y` is local clock except `FORCE_SOURCE_DATE=1`, which takes UTC now.
    let e = Engine::new("force-unset");
    for f in ["0", "1", "", "2", "abc"] {
        let (code, stdout, stderr) = e.run(&[("FORCE_SOURCE_DATE", OsStr::new(f))]);
        assert_eq!(code, Some(0), "FORCE={f:?}");
        if f == "2" || f == "abc" {
            assert_eq!(
                stderr,
                [warning(f.as_bytes()), warning(f.as_bytes())].concat(),
                "FORCE={f:?}"
            );
        } else {
            assert_eq!(stderr, b"", "FORCE={f:?} stderr");
        }
        let (y, p) = yp(&stdout);
        assert_clock_y(&y);
        assert_local_p(&p);
    }
}
