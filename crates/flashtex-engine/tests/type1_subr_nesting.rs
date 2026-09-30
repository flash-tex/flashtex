//! Nested `callsubr`s in a Type 1 font that is subset (#1237). pdfTeX's
//! `cs_mark` (writet1.c) follows every `callsubr` by recursing, with no
//! limit, so a subr that calls itself crashes TeX Live's pdfTeX 1.40.29
//! (SIGSEGV) and used to overflow this engine's stack. Here more than
//! `CS_SUBR_NEST_MAX` (1,000) nested calls are a pdfTeX font error, the
//! DESIGN.md 4.5 no-panic contract; up to it the output is pdfTeX's.
//!
//! The fonts are TeX Live's `cmr10.pfb` with subrs added after its own and
//! the glyph `a` calling the first of them. Skips where there is no TeX Live
//! (e.g. CI).
#![cfg(feature = "kpathsea")]

use flashtex_engine::resolver::find_texlive_bin;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

const LIMIT: usize = 1000; // src/pdftex/writet1.rs: CS_SUBR_NEST_MAX

const JOB: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\pdfoutput=1 \\pdfcompresslevel=0\n\
    \\pdfmapline{+nest nest <nest.pfb}\\font\\x=nest \\shipout\\hbox{\\x abc}\n\\end\n";

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

/// A charstring number (Type 1 spec 6.2), for 0..=1131.
fn num(v: usize) -> Vec<u8> {
    if v <= 107 {
        vec![(v + 139) as u8]
    } else {
        let w = v - 108;
        vec![((w >> 8) + 247) as u8, (w & 0xff) as u8]
    }
}

const CALLSUBR: u8 = 10;
const RETURN: u8 = 11;
const HSBW: u8 = 13;

/// An encrypted charstring with lenIV = 4.
fn charstring(code: &[u8]) -> Vec<u8> {
    let mut plain = vec![0u8; 4];
    plain.extend_from_slice(code);
    encrypt(&plain, 4330)
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

/// `cmr10` with subrs `base..base+n` added, subr `base+k` calling
/// `base+next(k)` (or returning where that is `None`), and `a` calling
/// `base`; and `base`, the number of subrs `cmr10` has.
fn font(cmr10: &[u8], n: usize, next: impl Fn(usize) -> Option<usize>) -> (Vec<u8>, usize) {
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

    let at = find(&plain, b"/Subrs ", 0).unwrap();
    let (base, end) = number(&plain, at + 7);
    let mut added = vec![];
    for k in 0..n {
        let code = match next(k) {
            Some(to) => [num(base + to), vec![CALLSUBR, RETURN]].concat(),
            None => vec![RETURN],
        };
        let cs = charstring(&code);
        added.extend_from_slice(format!("dup {} {} RD ", base + k, cs.len()).as_bytes());
        added.extend_from_slice(&cs);
        added.extend_from_slice(b" NP\n");
    }
    let mut out = plain[..at].to_vec();
    out.extend_from_slice(format!("/Subrs {}", base + n).as_bytes());
    out.extend_from_slice(&plain[end..]);
    plain = out;

    // after the line of the last subr
    let last = find(&plain, format!("dup {} ", base - 1).as_bytes(), 0).unwrap();
    let (len, e) = number(&plain, last + format!("dup {} ", base - 1).len());
    let nl = find(&plain, b"\n", e + " RD ".len() + len).unwrap() + 1;
    plain.splice(nl..nl, added);

    // `a`: call subr `base` right after its hsbw
    let a = find(&plain, b"/a ", 0).unwrap();
    let (len, e) = number(&plain, a + 3);
    let body_at = e + " RD ".len();
    let body = decrypt(&plain[body_at..body_at + len], 4330)[4..].to_vec();
    let mut i = 0;
    loop {
        let b = body[i];
        i += match b {
            255 => 5,
            247..=254 => 2,
            32..=246 => 1,
            12 => 2,
            _ => 1,
        };
        if b == HSBW {
            break;
        }
    }
    let code = [&body[..i], &num(base)[..], &[CALLSUBR], &body[i..]].concat();
    let cs = charstring(&code);
    let mut out = plain[..a].to_vec();
    out.extend_from_slice(format!("/a {} RD ", cs.len()).as_bytes());
    out.extend_from_slice(&cs);
    out.extend_from_slice(&plain[body_at + len..]);
    segs[i2].1 = encrypt(&out, 55665);

    let mut pfb = vec![];
    for (t, d) in segs {
        pfb.extend_from_slice(&[0x80, t]);
        pfb.extend_from_slice(&(d.len() as u32).to_le_bytes());
        pfb.extend_from_slice(&d);
    }
    pfb.extend_from_slice(&[0x80, 3]);
    (pfb, base)
}

fn kpsewhich(texbin: &Path, name: &str) -> PathBuf {
    let o = Command::new(texbin.join("kpsewhich"))
        .arg(name)
        .output()
        .unwrap();
    PathBuf::from(String::from_utf8(o.stdout).unwrap().trim())
}

/// Runs `JOB` with `pfb` as `nest.pfb` in a fresh directory.
fn run(bin: &Path, dir: &Path, pfb: &[u8], tfm: &Path, ours: bool) -> ExitStatus {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("nest.pfb"), pfb).unwrap();
    std::fs::copy(tfm, dir.join("nest.tfm")).unwrap();
    std::fs::write(dir.join("job.tex"), JOB).unwrap();
    let mut c = Command::new(bin);
    c.args(["-ini", "-interaction=nonstopmode", "job.tex"])
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
    c.status().unwrap()
}

#[test]
fn subr_nesting() {
    let Some(texbin) = find_texlive_bin() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let cmr10 = std::fs::read(kpsewhich(&texbin, "cmr10.pfb")).unwrap();
    let tfm = kpsewhich(&texbin, "cmr10.tfm");
    let base = std::env::temp_dir().join(format!("flashtex-subr-nest-{}", std::process::id()));

    // LIMIT nested subrs: pdfTeX's PDF, byte for byte
    let chain = |n: usize| font(&cmr10, n, move |k| (k + 1 < n).then_some(k + 1));
    let (pfb, _) = chain(LIMIT);
    let (a, b) = (base.join("ours"), base.join("tex"));
    assert!(run(ours, &a, &pfb, &tfm, true).success());
    assert!(run(&theirs, &b, &pfb, &tfm, false).success());
    let (x, y) = (
        std::fs::read(a.join("job.pdf")).unwrap(),
        std::fs::read(b.join("job.pdf")).unwrap(),
    );
    assert!(x == y, "job.pdf: {} vs {} bytes", x.len(), y.len());

    // one more, a subr that calls itself and a cycle of two: a font error,
    // exit status 1, no PDF. pdfTeX has no limit and is not run here: the
    // last two crash it.
    // (name, font, the added subr that fails, the one it calls)
    let cases = [
        ("deeper", chain(LIMIT + 1), LIMIT - 1, LIMIT),
        ("self", font(&cmr10, 1, |_| Some(0)), 0, 0),
        ("cycle", font(&cmr10, 2, |k| Some(1 - k)), 1, 0),
    ];
    for (name, (pfb, first), at, to) in cases {
        let d = base.join(name);
        let status = run(ours, &d, &pfb, &tfm, true);
        assert_eq!(status.code(), Some(1), "{name}: {status:?}");
        // the log's lines are broken at max_print_line
        let log = std::fs::read_to_string(d.join("job.log"))
            .unwrap()
            .replace('\n', "");
        let want = format!(
            "(file ./nest.pfb): Subr ({}): cannot call subr ({}): \
             more than {LIMIT} nested subr calls",
            first + at,
            first + to
        );
        assert!(
            log.contains(&want),
            "{name}: no `{want}` in the log:\n{log}"
        );
        assert!(
            log.contains(" ==> Fatal error occurred, no output PDF file produced!"),
            "{name}: {log}"
        );
        assert!(!d.join("job.pdf").exists(), "{name}: a PDF was written");
    }
    let _ = std::fs::remove_dir_all(&base);
}
