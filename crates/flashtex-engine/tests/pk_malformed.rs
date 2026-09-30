//! Malformed PK files (writet3.c/pkin.c, src/pdftex/writet3.rs) end in a
//! TeX fatal error, never a panic, a stack overflow or unbounded memory
//! (DESIGN.md §4.5). Where pdfTeX 1.40.29 itself ends in an error, the
//! message must be pdfTeX's, taken from TeX Live's own `pdftex` at run time
//! (DESIGN.md §8); where pdfTeX crashes, hangs or gets there only through
//! undefined behaviour, the case names the error wanted.
//!
//! Each case is a font `evil` (cmr10's TFM under another name, so it has no
//! map entry) with the PK file `evil.600pk` in the working directory.
//! Memory is capped with `ulimit -v` where the system allows lowering it
//! (Linux); everywhere, the peak resident size of the engine runs
//! (`getrusage(RUSAGE_CHILDREN)`) must stay under [`MAX_RSS`], and each run
//! gets 60 CPU seconds (`ulimit -t`). Skips where there is no TeX Live, unless
//! `FLASHTEX_REQUIRE_TEXLIVE=1`.
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The peak resident size allowed for an engine run on these files. pdfTeX
/// uses about 43 MB on the largest; the engine used 597 MB and 1.29 GB
/// before the raster was bounded.
const MAX_RSS: i64 = 256 << 20;

/// `ulimit -v` in KiB, where it can be set: 4 GiB of address space. The
/// engine's word space reserves address space it never touches (src/arena.rs),
/// so this cap is loose; [`MAX_RSS`] is the tight one.
const VM_CAP_KIB: u64 = 4 << 20;

/// The document: -ini, PDF output, one character of `evil`.
const DOC: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\pdfoutput=1 \\pdfpkresolution=600\n\
    \\font\\x=evil \\setbox0\\hbox{\\x A}\\shipout\\box0 \\end\n";

/// A PK file: the preamble, `chars`, and the postamble unless `truncated`.
fn pk(chars: &[u8], truncated: bool) -> Vec<u8> {
    let mut v = vec![247, 89, 0];
    for q in [10u32 << 20, 0, 544093, 544093] {
        v.extend_from_slice(&q.to_be_bytes()); // ds, cs, hppp, vppp
    }
    v.extend_from_slice(chars);
    if !truncated {
        v.push(245);
    }
    v
}

/// One extended-short-form character 'A' (flag bits 4..6): `dynf`, black
/// first run or not, width and height as 16-bit fields, then `raster`.
fn char_ext_short(dynf: u8, black: bool, w: u16, h: u16, raster: &[u8]) -> Vec<u8> {
    let pl = (13 + raster.len()) as u32;
    let mut v = vec![(dynf << 4) | ((black as u8) << 3) | 4 | ((pl >> 16) as u8 & 3)];
    v.extend_from_slice(&(pl as u16).to_be_bytes());
    v.push(b'A'); // cc
    v.extend_from_slice(&[0, 0x80, 0]); // tfm width
    for x in [w, w, h, 0, h] {
        v.extend_from_slice(&x.to_be_bytes()); // dm, w, h, hoff, voff
    }
    v.extend_from_slice(raster);
    v
}

/// One long-form character 'A' (flag bits 7): every field a signed quad.
fn char_long(dynf: u8, black: bool, w: i32, h: i32, raster: &[u8]) -> Vec<u8> {
    let mut v = vec![(dynf << 4) | ((black as u8) << 3) | 7];
    for q in [40 + raster.len() as i32, 65, 0x8000, 0, 0, w, h, 0, h] {
        v.extend_from_slice(&q.to_be_bytes()); // pl, cc, tfm, dx, dy, w, h, hoff, voff
    }
    v.extend_from_slice(raster);
    v
}

#[repr(C)]
struct RUsage {
    utime: [i64; 2],
    stime: [i64; 2],
    maxrss: i64,
    rest: [i64; 13],
}

extern "C" {
    fn getrusage(who: i32, usage: *mut RUsage) -> i32;
}

/// The largest resident size of any child waited for so far, in bytes.
fn children_maxrss() -> i64 {
    let mut u = RUsage {
        utime: [0; 2],
        stime: [0; 2],
        maxrss: 0,
        rest: [0; 13],
    };
    // SAFETY: `u` has the layout of `struct rusage` on macOS and Linux
    // (two 16-byte timevals, then 14 longs); RUSAGE_CHILDREN is -1 on both.
    unsafe { getrusage(-1, &mut u) };
    if cfg!(target_os = "macos") {
        u.maxrss
    } else {
        u.maxrss * 1024
    }
}

/// (name, files, whether pdfTeX ends in an error to compare with).
/// What the engine's error must be.
enum Want {
    /// pdfTeX's own, from TeX Live's `pdftex` on the same files (it ends in
    /// that error without undefined behaviour).
    Oracle,
    /// This text: pdfTeX crashes, hangs or gets there only through
    /// undefined behaviour, so it is no reference.
    Text(&'static str),
}

/// (name, files, the error wanted).
type Case = (&'static str, Vec<(&'static str, Vec<u8>)>, Want);

const TOO_MANY: &str = "error while unpacking; more bits than required";
const EOF: &str = "unexpected eof in pk file";

/// Packed nybbles, padded with a zero.
fn nybbles(n: &[u8]) -> Vec<u8> {
    n.chunks(2)
        .map(|c| c[0] << 4 | c.get(1).copied().unwrap_or(0))
        .collect()
}

struct Run {
    code: Option<i32>,
    log: String,
}

/// Run `bin` on `doc.tex` in `dir`, CPU time and (where allowed) virtual
/// memory capped.
fn run(bin: &Path, dir: &Path, ours: bool) -> Run {
    let mut c = Command::new("/bin/sh");
    c.arg("-c")
        .arg(format!(
            "ulimit -t 60; ulimit -v {VM_CAP_KIB} 2>/dev/null; exec \"$0\" \"$@\""
        ))
        .arg(bin)
        .args(["-ini", "-interaction=nonstopmode", "doc.tex"])
        .current_dir(dir)
        .env("TEXMFVAR", dir.join("texmf-var"))
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
    let st = c.status().unwrap();
    Run {
        code: st.code(),
        log: std::fs::read_to_string(dir.join("doc.log")).unwrap_or_default(),
    }
}

/// The text of the `!pdfTeX error:` after `(file evil): `, lines joined.
fn pdftex_error(log: &str) -> Option<String> {
    let text: String = log.lines().collect();
    let i = text.find("!pdfTeX error: ")?;
    let rest = &text[i..];
    let rest = &rest[rest.find("): ").map_or(0, |j| j + 3)..];
    Some(rest[..rest.find(" ==> Fatal").unwrap_or(rest.len())].to_string())
}

fn setup(base: &Path, name: &str, tfm: &Path, files: &[(&str, &[u8])]) -> PathBuf {
    let d = base.join(name);
    std::fs::create_dir_all(d.join("texmf-var")).unwrap();
    std::fs::copy(tfm, d.join("evil.tfm")).unwrap();
    std::fs::write(d.join("doc.tex"), DOC).unwrap();
    for (f, data) in files {
        std::fs::write(d.join(f), data).unwrap();
    }
    d
}

#[test]
fn malformed_pk_files_end_in_a_tex_error() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    // cmr10's TFM, which every TeX Live scheme has, under a name with no
    // map entry.
    let tfm = Command::new(texbin.join("kpsewhich"))
        .arg("cmr10.tfm")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|p| !p.is_empty());
    let Some(tfm) = tfm else {
        common::no_texlive();
        return;
    };
    let tfm = PathBuf::from(tfm);
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let base = std::env::temp_dir().join(format!("flashtex-pk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);

    let long_line = {
        let mut p = b"\\pdffontscale 0.001\n".to_vec();
        p.extend(std::iter::repeat_n(b'x', 1100));
        p.push(b'\n');
        p
    };
    let big: Vec<u8> = pk(&char_ext_short(13, false, 32752, 32767, &[0x11; 5]), true);
    assert_eq!(big.len(), 41);
    // Packed numbers with dyn_f 13: 32767 and 32766 (huge-count form).
    let n32767: &[u8] = &[0, 0, 0, 8, 0, 0, 1];
    let n32766: &[u8] = &[0, 0, 0, 8, 0, 0, 0];
    // A repeat count of 32766, then a white run of 32767.
    let fill = nybbles(&[&[14], n32766, n32767].concat());
    // A huge count whose remainder is the most negative `long`.
    let min_remainder = nybbles(&[&[0u8; 16][..], &[1, 8], &[0; 14], &[2]].concat());
    let cases: Vec<Case> = vec![
        // width -1, first run black: C indexes gpower[17].
        (
            "negative-width-black",
            vec![(
                "evil.600pk",
                pk(&char_ext_short(13, true, 0xffff, 1, &[0x11; 4]), false),
            )],
            Want::Text(TOO_MANY),
        ),
        // width -1, first run white: C writes past its raster forever.
        (
            "negative-width-white",
            vec![(
                "evil.600pk",
                pk(&char_ext_short(13, false, 0xffff, 1, &[0x11; 4]), false),
            )],
            Want::Text(TOO_MANY),
        ),
        // A 32752x32767 character in 41 bytes: pdfTeX reaches the end of
        // the file with 43 MB.
        ("huge-glyph", vec![("evil.600pk", big)], Want::Oracle),
        // A million bytes of repeat-count nybbles: C recurses once per
        // nybble and overflows its stack.
        (
            "repeat-counts-1m",
            vec![(
                "evil.600pk",
                pk(&char_long(13, false, 16, 16, &vec![0xee; 1_000_000]), true),
            )],
            Want::Text(EOF),
        ),
        // The same with 20000 bytes, which pdfTeX survives.
        (
            "repeat-counts-20k",
            vec![(
                "evil.600pk",
                pk(&char_long(13, false, 16, 16, &vec![0xee; 20_000]), true),
            )],
            Want::Oracle,
        ),
        // A million bytes after an extended-short header whose length
        // spills into the flag byte: read as a long form, it asks for more
        // words than its raster holds, which pdfTeX reports (the engine
        // grew the raster to 6.6 GB for it before it was bounded).
        (
            "raster-overrun-1m",
            vec![(
                "evil.600pk",
                pk(
                    &char_ext_short(13, false, 16, 16, &vec![0xee; 1_000_000]),
                    true,
                ),
            )],
            Want::Oracle,
        ),
        // A .pgc line longer than writet3.c's buffer.
        ("pgc-long-line", vec![("evil.pgc", long_line)], Want::Oracle),
        // A remainder of LONG_MIN: C negates it and returns 0 forever.
        (
            "remainder-long-min",
            vec![(
                "evil.600pk",
                pk(&char_long(13, false, 16, 16, &min_remainder), false),
            )],
            Want::Text(TOO_MANY),
        ),
        // cheight 0x7FF10001 is 1 as a C short, and the file ends inside
        // that row: pdfTeX reaches its end of file.
        (
            "tall-truncated",
            vec![(
                "evil.600pk",
                pk(&char_long(13, false, 32767, 0x7ff1_0001, &[0x11; 8]), false),
            )],
            Want::Oracle,
        ),
        // The same with the row complete: C then draws 2^31 rows from a
        // one-row raster (the engine wrote gigabytes of zeros for it).
        (
            "tall-one-row",
            vec![(
                "evil.600pk",
                pk(
                    &char_long(13, false, 32767, 0x7ff1_0001, &nybbles(n32767)),
                    false,
                ),
            )],
            Want::Text(TOO_MANY),
        ),
        (
            "tall-one-narrow-row",
            vec![(
                "evil.600pk",
                pk(
                    &char_long(10, false, 16, 0x7fff_0001, &nybbles(&[11, 5])),
                    false,
                ),
            )],
            Want::Text(TOO_MANY),
        ),
        // Width 491504 is 32752 as a C short; one repeat count fills
        // gigabytes (pdfTeX: SIGBUS; the engine: 3.9 GB).
        (
            "wide-repeat",
            vec![(
                "evil.600pk",
                pk(&char_long(13, false, 491_504, 32767, &fill), false),
            )],
            Want::Text(TOO_MANY),
        ),
        // Bitmaps the file cannot hold: pdfTeX writes past its raster, then
        // reaches its end of file.
        (
            "bitmap-wide",
            vec![(
                "evil.600pk",
                pk(&char_long(14, false, 0x7fff_ffff, 1, &[0x55; 16]), true),
            )],
            Want::Text(EOF),
        ),
        (
            "bitmap-tall",
            vec![(
                "evil.600pk",
                pk(&char_long(14, false, 8, 0x7fff_ffff, &[0x55; 16]), true),
            )],
            Want::Text(EOF),
        ),
    ];
    let mut failures = vec![];
    for (name, files, want) in &cases {
        let files: Vec<(&str, &[u8])> = files.iter().map(|(f, d)| (*f, d.as_slice())).collect();
        let a = setup(&base, &format!("{name}-ours"), &tfm, &files);
        let r = run(ours, &a, true);
        let got = pdftex_error(&r.log);
        if r.code != Some(1) || got.is_none() {
            failures.push(format!(
                "{name}: exit {:?}, error {got:?} (want exit 1 and a pdfTeX error)",
                r.code
            ));
            continue;
        }
        let want = match want {
            Want::Text(t) => Some(t.to_string()),
            Want::Oracle => {
                let b = setup(&base, &format!("{name}-tex"), &tfm, &files);
                let t = run(&theirs, &b, false);
                match pdftex_error(&t.log) {
                    Some(e) if t.code == Some(1) => Some(e),
                    e => {
                        failures.push(format!(
                            "{name}: pdfTeX exit {:?}, error {e:?}: no reference",
                            t.code
                        ));
                        continue;
                    }
                }
            }
        };
        if got != want {
            failures.push(format!("{name}: error {got:?}, want {want:?}"));
        }
    }
    let rss = children_maxrss();
    if rss > MAX_RSS {
        failures.push(format!("peak resident size {rss} bytes > {MAX_RSS}"));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    let _ = std::fs::remove_dir_all(&base);
}
