//! Font map and encoding file edge cases (src/pdftex/mapfile.rs, subfont.rs,
//! writet1/encoding.rs) against TeX Live's own `pdftex`: SlantFont and
//! ExtendFont values that saturate C's `int`, `/Flags` numbers too big for
//! `atoi`, and every `check_buf` overflow of a map line, a map file line
//! and an `.enc` line, whose fatal message names the C file and line as
//! TeX Live's build has them (lane REVIEW-PDF). Each case is a plain-TeX job;
//! exit status, the PDF (bytes, or none from either) and the log from the
//! `**` line on (lines joined, the program name normalised) must agree.
//! Skips where there is no TeX Live.
//!
//! Three cases hinge on undefined C behaviour: a double out of `int`'s range
//! cast to `int`, then `abs` of the result. DESIGN §13 (2026-11-01) rules
//! that FlashTeX matches TeX Live's macOS arm64 build there ([`ARM64_RULED`]).
//! On macOS arm64 they are compared with the local TeX Live like the rest
//! (which must also still give the recorded result); elsewhere, with that
//! build's result as recorded in `map_edges/arm64-oracle.txt`.
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

const SETUP: &str = "\\pdfoutput=1 \\pdfcompresslevel=0 \\pdfobjcompresslevel=0 \
                     \\pdfsuppressptexinfo=-1 ";

/// The cases whose expected result is TeX Live macOS arm64's, by the §13
/// ruling on undefined C behaviour. mapfile.c rounds `d * 1000` with a cast
/// to `integer` that is out of range here, then tests `abs(x) > 1000`
/// (`> 2000` for ExtendFont). Measured: TeX Live 2026 on macOS arm64 rejects
/// all three entries with a warning, as FlashTeX does (run 42501522185);
/// on x86_64 Linux it accepts them without one and sets the font with the
/// wrapped value, so its PDFs are 62-63 bytes longer (runs 41499342620 and
/// 41503402654). Why (belief, not measured): arm64's cast saturates to
/// INT_MIN or INT_MAX and clang folds the `abs` test into a range test, while
/// x86_64 gives INT_MIN for both and gcc's `abs(INT_MIN)` stays negative.
const ARM64_RULED: [&str; 3] = ["slant-min", "extend-min", "extend-max"];

/// Whether this host's TeX Live is the build the §13 ruling names.
const ARM64_HOST: bool = cfg!(all(target_os = "macos", target_arch = "aarch64"));

/// TeX Live macOS arm64's result on each [`ARM64_RULED`] case, measured on
/// GitHub's hosted macos-15 with MacTeX 2026 (see the file's header).
const ARM64_ORACLE: &str = include_str!("map_edges/arm64-oracle.txt");

/// One recorded result: exit status, the PDF's length and FNV-1a 64 hash (or
/// none), and the normalised log with the paths of [`tl_files`] replaced.
#[derive(Debug, PartialEq)]
struct Outcome {
    exit: Option<i32>,
    pdf: Option<(usize, u64)>,
    log: String,
}

impl Outcome {
    fn new(exit: Option<i32>, pdf: Option<&[u8]>, log: &str, tl: &[(String, &str)]) -> Outcome {
        Outcome {
            exit,
            pdf: pdf.map(|b| (b.len(), fnv1a(b))),
            log: tl
                .iter()
                .fold(log.to_string(), |l, (path, var)| l.replace(path, var)),
        }
    }

    /// The record's lines for case `name`, as `arm64-oracle.txt` has them.
    fn record(&self, name: &str) -> String {
        let exit = self.exit.map_or("none".to_string(), |c| c.to_string());
        let pdf = self
            .pdf
            .map_or("none".to_string(), |(n, h)| format!("{n} {h:016x}"));
        format!("case {name}\nexit {exit}\npdf {pdf}\nlog {}\n", self.log)
    }
}

fn fnv1a(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf2_9ce4_8422_2325, |h, &x| {
        (h ^ u64::from(x)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// The recorded arm64 result of case `name`.
fn recorded(name: &str) -> Outcome {
    let mut lines = ARM64_ORACLE
        .lines()
        .skip_while(|l| *l != format!("case {name}"))
        .skip(1);
    let mut field = |key: &str| {
        let l = lines.next().unwrap_or_default();
        l.strip_prefix(key)
            .and_then(|r| r.strip_prefix(' '))
            .unwrap_or_else(|| panic!("arm64-oracle.txt: case {name}: no `{key}` line"))
            .to_string()
    };
    let exit = field("exit");
    let pdf = field("pdf");
    let log = field("log");
    Outcome {
        exit: (exit != "none").then(|| exit.parse().unwrap()),
        pdf: (pdf != "none").then(|| {
            let (n, h) = pdf.split_once(' ').unwrap();
            (n.parse().unwrap(), u64::from_str_radix(h, 16).unwrap())
        }),
        log,
    }
}

/// The two TeX Live files these logs name, as the local `kpsewhich` finds
/// them, and the names the record writes for them (so the record does not
/// depend on where TeX Live is installed).
fn tl_files(texbin: &Path) -> Vec<(String, &'static str)> {
    [("pdftex.map", "$PDFTEX_MAP"), ("cmr10.pfb", "$CMR10_PFB")]
        .into_iter()
        .map(|(file, var)| {
            let out = Command::new(texbin.join("kpsewhich"))
                .arg(file)
                .output()
                .unwrap();
            let path = String::from_utf8(out.stdout).unwrap().trim().to_string();
            assert!(!path.is_empty(), "kpsewhich found no {file}");
            (path, var)
        })
        .collect()
}

/// (name, document). `files` is the directory of [`support_files`], named by
/// its absolute path (shared by both engines, so their logs name the same
/// file).
fn cases(files: &Path) -> Vec<(String, String)> {
    let f = files.display();
    let x = "x".repeat(1100);
    let mut v: Vec<(String, String)> = [
        // (integer)(d - 0.5) of -1e33 is INT_MIN, which `abs(x) > 1000`
        // rejects (the entry is dropped with a warning)
        (
            "slant-min",
            "\\pdfmapline{=cmr10 CMR10 \"-1e30 SlantFont\" <cmr10.pfb}".to_string(),
        ),
        (
            "extend-min",
            "\\pdfmapline{=cmr10 CMR10 \"-1e30 ExtendFont\" <cmr10.pfb}".to_string(),
        ),
        (
            "extend-max",
            "\\pdfmapline{=cmr10 CMR10 \"1e30 ExtendFont\" <cmr10.pfb}".to_string(),
        ),
        // atoi: strtol saturates, then the cast to int
        (
            "flags-huge",
            "\\pdfmapline{=cmr10 CMR10 123456789012345678901234 <cmr10.pfb}".to_string(),
        ),
        (
            "flags-wrap",
            "\\pdfmapline{=cmr10 CMR10 4294967297 <cmr10.pfb}".to_string(),
        ),
        (
            "flags-2p31",
            "\\pdfmapline{=cmr10 CMR10 2147483648 <cmr10.pfb}".to_string(),
        ),
        // read_field's check_buf at mapfile.c's lines 455, 460 and 531
        ("over-tfm", format!("\\pdfmapline{{={x}}}")),
        ("over-ps", format!("\\pdfmapline{{=cmr10 {x}}}")),
        (
            "over-fontfile",
            format!("\\pdfmapline{{=cmr10 CMR10 <<{x}}}"),
        ),
        (
            "over-enc",
            format!("\\pdfmapline{{=cmr10 CMR10 <{x}.enc <cmr10.pfb}}"),
        ),
        // a map file's line: append_char_to_buf at line 440
        ("mapfile-1023", format!("\\pdfmapfile{{{f}/l1023.map}}")),
        ("mapfile-1024", format!("\\pdfmapfile{{{f}/l1024.map}}")),
    ]
    .into_iter()
    .map(|(n, b)| (n.to_string(), b))
    .collect();
    // an .enc line of N bytes: append_char_to_buf (writet1.c line 223) from
    // 4096, append_eol (line 225) from 4094
    for n in 4092..=4097 {
        v.push((
            format!("enc-{n}"),
            format!("\\pdfmapline{{=cmr10 CMR10 <{f}/e{n}.enc <cmr10.pfb}}"),
        ));
    }
    v.into_iter()
        .map(|(n, b)| (n, format!("{SETUP}{b}Ab\\bye\n")))
        .collect()
}

/// The support files: `.enc` files with one long line, map files whose
/// one line is 1023 and 1024 bytes long (blanks are not doubled, since a
/// map file's blanks collapse).
fn support_files(dir: &Path) {
    for n in 4092..=4097usize {
        let name = format!("/{}", "a".repeat(n - 1));
        let rest = " /a".repeat(255);
        std::fs::write(
            dir.join(format!("e{n}.enc")),
            format!("/E [\n{name}\n{rest} ] def\n"),
        )
        .unwrap();
    }
    for n in [1023usize, 1024] {
        let base = "cmr10 CMR10 <cmr10.pfb";
        let line = format!("{base} {}", "y".repeat(n - base.len() - 1));
        std::fs::write(dir.join(format!("l{n}.map")), format!("{line}\n")).unwrap();
    }
}

fn run(bin: &Path, dir: &Path, args: &[&str], ours: bool) -> Option<i32> {
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
    c.status().unwrap().code()
}

/// The log from the `**` line on, lines joined, with the program name
/// after "pdfTeX warning: " and "!pdfTeX error: " replaced.
fn normalised_log(log: &str) -> String {
    let from = log.find("\n**").map_or(0, |i| i + 1);
    let mut s: String = log[from..].lines().collect();
    for key in ["pdfTeX warning: ", "!pdfTeX error: "] {
        let mut out = String::new();
        let mut rest = s.as_str();
        while let Some(i) = rest.find(key) {
            out.push_str(&rest[..i + key.len()]);
            rest = &rest[i + key.len()..];
            let end = rest.find([' ', ':']).unwrap_or(rest.len());
            out.push_str("PROG");
            rest = &rest[end..];
        }
        out.push_str(rest);
        s = out;
    }
    s
}

#[test]
fn map_edges_match_tex_live() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let base = common::fresh_dir("flashtex-mapedges");
    let (a, b) = (base.join("ours"), base.join("tex"));
    let files = base.join("files");
    std::fs::create_dir_all(&files).unwrap();
    support_files(&files);
    let cases = cases(&files);
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
        for (name, body) in &cases {
            std::fs::write(d.join(format!("{name}.tex")), body).unwrap();
        }
    }
    let tl = tl_files(&texbin);
    run(ours, &a, &["-ini", "\\input plain \\dump"], true);
    run(&theirs, &b, &["-ini", "\\input plain \\dump"], false);
    let mut failures = vec![];
    for (name, _) in &cases {
        let arg = format!("&plain {name}");
        let rx = run(ours, &a, &["-fmt=plain", &arg], true);
        let ry = run(&theirs, &b, &[&arg], false);
        if ARM64_RULED.contains(&name.as_str()) {
            let read = |d: &Path| {
                let pdf = std::fs::read(d.join(format!("{name}.pdf"))).ok();
                let log = std::fs::read_to_string(d.join(format!("{name}.log")));
                (pdf, normalised_log(&log.unwrap_or_default()))
            };
            let ((px, lx), (py, ly)) = (read(&a), read(&b));
            let x = Outcome::new(rx, px.as_deref(), &lx, &tl);
            let want = recorded(name);
            if ARM64_HOST {
                // the record must still be this TeX Live's result
                let y = Outcome::new(ry, py.as_deref(), &ly, &tl);
                if y != want {
                    failures.push(format!(
                        "{name}: TeX Live (macOS arm64) no longer gives the recorded result; \
                         its record is now:\n{}",
                        y.record(name)
                    ));
                }
            }
            if x != want {
                failures.push(format!(
                    "{name}: ours differs from TeX Live macOS arm64's (§13):\n  ours {}\n  want {}",
                    x.record(name),
                    want.record(name)
                ));
            }
            continue;
        }
        if rx != ry {
            failures.push(format!("{name}: exit {rx:?} vs TeX Live's {ry:?}"));
        }
        let pdf = format!("{name}.pdf");
        let (x, y) = (
            std::fs::read(a.join(&pdf)).ok(),
            std::fs::read(b.join(&pdf)).ok(),
        );
        if x != y {
            failures.push(format!(
                "{pdf}: ours {:?} bytes, TeX Live's {:?}",
                x.map(|v| v.len()),
                y.map(|v| v.len())
            ));
        }
        let log = format!("{name}.log");
        let lx = normalised_log(&std::fs::read_to_string(a.join(&log)).unwrap_or_default());
        let ly = normalised_log(&std::fs::read_to_string(b.join(&log)).unwrap_or_default());
        if lx != ly || lx.is_empty() {
            let at = lx
                .chars()
                .zip(ly.chars())
                .position(|(p, q)| p != q)
                .unwrap_or(lx.len().min(ly.len()));
            let ctx = |s: &str| {
                s.chars()
                    .skip(at.saturating_sub(40))
                    .take(160)
                    .collect::<String>()
            };
            failures.push(format!(
                "{log}: differs at {at}:\n  ours {:?}\n  tex  {:?}",
                ctx(&lx),
                ctx(&ly)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    if std::env::var_os("FLASHTEX_KEEP_TEST_DIR").is_none() {
        let _ = std::fs::remove_dir_all(&base);
    }
}
