//! A `seac` glyph whose accent is not in the font, subset, against TeX
//! Live's own `pdftex` (lane REVIEW-PDF).
//!
//! writet1.c's `t1_mark_glyphs` walks `fd->gl_tree` with libavl's
//! traverser while `cs_mark`'s `seac` case inserts the base and the accent
//! into that same tree; after an insertion `avl_t_next` refreshes, so a
//! name added after the current glyph is visited too, and marking an
//! accent the font lacks warns a second time ("glyph `acute' undefined"
//! twice). The engine walked a copy of the set made before the walks and
//! warned once.
//!
//! The font is TeX Live's `cmr10.pfb` with its `acute` renamed `acutex`
//! and a glyph `Aacute` added (`32 750 hsbw 0 0 0 65 194 seac`: A with
//! StandardEncoding's acute); an encoding puts `Aacute` at 65 and `B` at
//! 66, and the job typesets "AB". The PDF and the log from the `**` line
//! on (lines joined, the program name normalised) must agree. Skips where
//! there is no TeX Live.
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The job; `enc` is the encoding file, named by its absolute path so that
/// both engines name the same file in their logs.
fn job(enc: &Path) -> String {
    format!(
        "\\pdfoutput=1 \\pdfcompresslevel=0 \\pdfobjcompresslevel=0 \
         \\pdfsuppressptexinfo=-1 \\pdfmapline{{=cmr10 CMRSEAC <{} <seac.pfb}}\
         \\font\\x=cmr10 \\x AB\\bye\n",
        enc.display()
    )
}

fn decrypt(data: &[u8], mut r: u16) -> Vec<u8> {
    data.iter()
        .map(|&c| {
            let p = c ^ (r >> 8) as u8;
            r = (c as u16)
                .wrapping_add(r)
                .wrapping_mul(52845)
                .wrapping_add(22719);
            p
        })
        .collect()
}

fn encrypt(data: &[u8], mut r: u16) -> Vec<u8> {
    data.iter()
        .map(|&p| {
            let c = p ^ (r >> 8) as u8;
            r = (c as u16)
                .wrapping_add(r)
                .wrapping_mul(52845)
                .wrapping_add(22719);
            c
        })
        .collect()
}

/// A charstring number (Type 1 spec 6.2), for 0 and up.
fn num(v: usize) -> Vec<u8> {
    if v <= 107 {
        vec![(v + 139) as u8]
    } else if v <= 1131 {
        let w = v - 108;
        vec![((w >> 8) + 247) as u8, (w & 0xff) as u8]
    } else {
        [&[255u8][..], &(v as u32).to_be_bytes()].concat()
    }
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

/// `cmr10` with `acute` renamed `acutex` and the `seac` glyph `Aacute`
/// added after it.
fn font(cmr10: &[u8]) -> Vec<u8> {
    // PFB segments: 0x80, type, 4-byte little-endian length
    let mut segs = vec![];
    let mut pos = 0;
    while pos + 6 <= cmr10.len() && cmr10[pos] == 0x80 && matches!(cmr10[pos + 1], 1 | 2) {
        let len = u32::from_le_bytes(cmr10[pos + 2..pos + 6].try_into().unwrap()) as usize;
        segs.push((cmr10[pos + 1], cmr10[pos + 6..pos + 6 + len].to_vec()));
        pos += 6 + len;
    }
    let i2 = segs.iter().position(|s| s.0 == 2).unwrap();
    let mut plain = decrypt(&segs[i2].1, 55665);

    // one more glyph in the dictionary's size
    let dict = find(&plain, b"/CharStrings ", 0).unwrap();
    let (n, e) = number(&plain, dict + b"/CharStrings ".len());
    plain.splice(dict..e, format!("/CharStrings {}", n + 1).into_bytes());

    // `/acute N RD <N bytes> ND`: rename it, and add `Aacute` after it
    // with the same separators (`RD`, and what ends the line)
    let dict = find(&plain, b"/CharStrings", 0).unwrap();
    let at = find(&plain, b"\n/acute ", dict).unwrap() + 1;
    let (len, e) = number(&plain, at + b"/acute ".len());
    let rd = &plain[e..e + 4].to_vec(); // " RD "
    let data_end = e + 4 + len;
    let nl = find(&plain, b"\n", data_end).unwrap();
    let tail = plain[data_end..nl].to_vec(); // " ND"
    let mut code = vec![0u8; 4]; // lenIV 4
    for v in [32, 750] {
        code.extend(num(v));
    }
    code.push(13); // hsbw
    for v in [0, 0, 0, 65, 194] {
        code.extend(num(v));
    }
    code.extend([12, 6]); // seac
    let cs = encrypt(&code, 4330);
    let mut glyph = format!("\n/Aacute {}", cs.len()).into_bytes();
    glyph.extend_from_slice(rd);
    glyph.extend_from_slice(&cs);
    glyph.extend_from_slice(&tail);
    plain.splice(nl..nl, glyph);
    plain.splice(at..at + b"/acute".len(), b"/acutex".iter().copied());
    segs[i2].1 = encrypt(&plain, 55665);

    let mut pfb = vec![];
    for (t, d) in segs {
        pfb.extend_from_slice(&[0x80, t]);
        pfb.extend_from_slice(&(d.len() as u32).to_le_bytes());
        pfb.extend_from_slice(&d);
    }
    pfb.extend_from_slice(&[0x80, 3]);
    pfb
}

/// An encoding with `Aacute` at 65, `B` at 66 and `.notdef` elsewhere.
fn encoding() -> String {
    let mut s = String::from("/SeacTest [\n");
    for c in 0..256 {
        s.push_str(match c {
            65 => "/Aacute\n",
            66 => "/B\n",
            _ => "/.notdef\n",
        });
    }
    s.push_str("] def\n");
    s
}

fn kpsewhich(texbin: &Path, name: &str) -> PathBuf {
    let o = Command::new(texbin.join("kpsewhich"))
        .arg(name)
        .output()
        .unwrap();
    PathBuf::from(String::from_utf8(o.stdout).unwrap().trim())
}

fn run(bin: &Path, dir: &Path, ours: bool) {
    let mut c = Command::new(bin);
    c.args([
        "-ini",
        "-jobname=job",
        "-interaction=nonstopmode",
        "\\input plain \\input job",
    ])
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
    c.status().unwrap();
}

/// The log from the `**` line on, lines joined, with the program name
/// after "pdfTeX warning: " replaced.
fn normalised_log(log: &str) -> String {
    let from = log.find("\n**").map_or(0, |i| i + 1);
    let s: String = log[from..].lines().collect();
    let key = "pdfTeX warning: ";
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
    out
}

#[test]
fn seac_with_a_missing_accent_matches_tex_live() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let pfb = font(&std::fs::read(kpsewhich(&texbin, "cmr10.pfb")).unwrap());
    let base = common::fresh_dir("flashtex-seac");
    let (a, b) = (base.join("ours"), base.join("tex"));
    let enc = base.join("seac.enc");
    std::fs::write(&enc, encoding()).unwrap();
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
        std::fs::write(d.join("seac.pfb"), &pfb).unwrap();
        std::fs::write(d.join("job.tex"), job(&enc)).unwrap();
    }
    run(ours, &a, true);
    run(&theirs, &b, false);
    let lx = normalised_log(&std::fs::read_to_string(a.join("job.log")).unwrap_or_default());
    let ly = normalised_log(&std::fs::read_to_string(b.join("job.log")).unwrap_or_default());
    // the font is what this test means: pdfTeX does warn about the accent
    assert!(
        ly.contains("glyph `acute' undefined"),
        "TeX Live's log has no warning about the accent: {ly:?}"
    );
    assert_eq!(lx, ly, "the logs differ");
    assert_eq!(
        std::fs::read(a.join("job.pdf")).ok(),
        std::fs::read(b.join("job.pdf")).ok(),
        "the PDFs differ"
    );
    if std::env::var_os("FLASHTEX_KEEP_TEST_DIR").is_none() {
        let _ = std::fs::remove_dir_all(&base);
    }
}
