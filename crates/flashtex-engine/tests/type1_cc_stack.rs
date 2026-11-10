//! writet1.c's charstring operand stack outlives a font (#2408). In pdfTeX
//! 1.40.29 (TeX Live 2026, `texk/web2c/pdftexdir/writet1.c`) `cc_stack` and
//! `stack_ptr` are file statics: only `cc_clear` empties the stack (on a
//! clearing operator, `seac`, and `cs_mark`'s `cs_error` exit), never the
//! start of a font. An operand a font's last walk leaves on the stack is
//! therefore under the next font's first walk, `.notdef`, whose `hsbw` takes
//! its two operands from the bottom of the stack and fails.
//!
//! The probe is REVIEW-D's on #2286: `cm0.pfb` is TeX Live's `cmr10.pfb`
//! with one operand (7) after `/A`'s `endchar`; it sorts before `cmr12.pfb`
//! in `fd_tree`, so it is written first. With `\a A\b B` set, TeX Live
//! 2026's pdfTeX stops while writing cmr12 with
//! `CharString (/.notdef): more arguments on stack (3) than required (2)`,
//! writes no PDF and exits 1. The expected result is pdfTeX's, run here.
//! Skips where there is no TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const JOB: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\pdfoutput=1 \\pdfcompresslevel=0\n\
    \\pdfmapline{=cmr10 CMR10 <cm0.pfb}\\pdfmapline{=cmr12 CMR12 <cmr12.pfb}\n\
    \\font\\a=cmr10 \\font\\b=cmr12 \\shipout\\hbox{\\a A\\b B}\n\\end\n";

const MESSAGE: &str = "CharString (/.notdef): more arguments on stack (3) than required (2)";

fn crypt(data: &[u8], mut r: u16, decrypting: bool) -> Vec<u8> {
    data.iter()
        .map(|&b| {
            let out = b ^ (r >> 8) as u8;
            let c = if decrypting { b } else { out };
            r = (c as u16)
                .wrapping_add(r)
                .wrapping_mul(52845)
                .wrapping_add(22719);
            out
        })
        .collect()
}

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    hay[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|i| i + from)
}

/// The decimal number at `at`, and the offset after it.
fn number(b: &[u8], at: usize) -> (usize, usize) {
    let mut e = at;
    while b[e].is_ascii_digit() {
        e += 1;
    }
    (std::str::from_utf8(&b[at..e]).unwrap().parse().unwrap(), e)
}

/// `cmr10.pfb` with the operand 7 (charstring byte 7 + 139) after `/A`'s
/// `endchar`.
fn operand_after_endchar(cmr10: &[u8]) -> Vec<u8> {
    // PFB segments: 0x80, type, 4-byte little-endian length
    let mut segs = vec![];
    let mut pos = 0;
    while pos + 6 <= cmr10.len() && cmr10[pos] == 0x80 && matches!(cmr10[pos + 1], 1 | 2) {
        let len = u32::from_le_bytes(cmr10[pos + 2..pos + 6].try_into().unwrap()) as usize;
        segs.push((cmr10[pos + 1], cmr10[pos + 6..pos + 6 + len].to_vec()));
        pos += 6 + len;
    }
    let i2 = segs.iter().position(|s| s.0 == 2).unwrap();
    let plain = crypt(&segs[i2].1, 55665, true);
    let dict = find(&plain, b"/CharStrings", 0).unwrap();
    let a = find(&plain, b"\n/A ", dict).unwrap() + 1;
    let (len, e) = number(&plain, a + 3);
    let body_at = e + " RD ".len();
    let mut cs = crypt(&plain[body_at..body_at + len], 4330, true);
    assert_eq!(cs.last(), Some(&14), "/A ends with endchar");
    cs.push(7 + 139);
    let cs = crypt(&cs, 4330, false);
    let mut out = plain[..a].to_vec();
    out.extend_from_slice(format!("/A {} RD ", cs.len()).as_bytes());
    out.extend_from_slice(&cs);
    out.extend_from_slice(&plain[body_at + len..]);
    segs[i2].1 = crypt(&out, 55665, false);
    let mut pfb = vec![];
    for (t, d) in segs {
        pfb.extend_from_slice(&[0x80, t]);
        pfb.extend_from_slice(&(d.len() as u32).to_le_bytes());
        pfb.extend_from_slice(&d);
    }
    pfb.extend_from_slice(&[0x80, 3]);
    pfb
}

fn kpsewhich(texbin: &Path, name: &str) -> PathBuf {
    let o = Command::new(texbin.join("kpsewhich"))
        .arg(name)
        .output()
        .unwrap();
    PathBuf::from(String::from_utf8(o.stdout).unwrap().trim())
}

/// What a run gives: its exit code, the PDF (empty for none), and the log.
struct Run {
    code: Option<i32>,
    pdf: Vec<u8>,
    log: String,
}

/// Runs `JOB` with `cm0` as `cm0.pfb` in a fresh directory.
fn run(bin: &Path, dir: &Path, texbin: &Path, cm0: &[u8], ours: bool) -> Run {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("cm0.pfb"), cm0).unwrap();
    for f in ["cmr12.pfb", "cmr10.tfm", "cmr12.tfm"] {
        std::fs::copy(kpsewhich(texbin, f), dir.join(f)).unwrap();
    }
    std::fs::write(dir.join("job.tex"), JOB).unwrap();
    let mut c = Command::new(bin);
    c.args(["-ini", "-interaction=batchmode", "job.tex"])
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
    let code = c.status().unwrap().code();
    Run {
        code,
        pdf: std::fs::read(dir.join("job.pdf")).unwrap_or_default(),
        // unwrapped: the error line breaks at `max_print_line`, after the
        // program's path
        log: String::from_utf8_lossy(&std::fs::read(dir.join("job.log")).unwrap_or_default())
            .replace('\n', ""),
    }
}

#[test]
fn a_fonts_leftover_operand_is_under_the_next_fonts_notdef() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let cmr10 = std::fs::read(kpsewhich(&texbin, "cmr10.pfb")).unwrap();
    let base = common::fresh_dir("flashtex-t1-cc-stack");

    // the probe: pdfTeX fails while writing cmr12
    let cm0 = operand_after_endchar(&cmr10);
    let want = run(&theirs, &base.join("theirs"), &texbin, &cm0, false);
    let got = run(ours, &base.join("ours"), &texbin, &cm0, true);
    assert_eq!(want.code, Some(1), "pdfTeX's exit code");
    assert!(want.pdf.is_empty(), "pdfTeX wrote a PDF");
    assert!(want.log.contains(MESSAGE), "pdfTeX's log:\n{}", want.log);
    assert_eq!(got.code, want.code, "exit code: ours vs pdfTeX");
    assert!(
        got.pdf.is_empty(),
        "ours wrote a {}-byte PDF",
        got.pdf.len()
    );
    assert!(got.log.contains(MESSAGE), "our log:\n{}", got.log);

    // the control: cmr10 as it is writes the same PDF in both
    let want = run(&theirs, &base.join("theirs0"), &texbin, &cmr10, false);
    let got = run(ours, &base.join("ours0"), &texbin, &cmr10, true);
    assert_eq!(want.code, Some(0), "pdfTeX's exit code, unmodified cmr10");
    assert!(!want.pdf.is_empty(), "pdfTeX wrote no PDF");
    assert_eq!(got.code, want.code, "exit code: ours vs pdfTeX, unmodified");
    assert!(got.pdf == want.pdf, "the PDFs differ, unmodified cmr10");
    let _ = std::fs::remove_dir_all(&base);
}
