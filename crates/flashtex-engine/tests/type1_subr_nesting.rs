//! Nested `callsubr`s in a Type 1 font that is subset (#1237). pdfTeX's
//! `cs_mark` (writet1.c) follows every `callsubr` by recursing, with no
//! limit: TeX Live's pdfTeX 1.40.29 embeds a font with 50,000 nested subrs,
//! and a subr that calls itself crashes it (SIGSEGV). This engine's
//! `cs_mark` keeps its own stack, so it gives pdfTeX's PDF for any depth,
//! and a subr called while it is being parsed is a font error (DESIGN.md
//! 4.5, no panics) where pdfTeX crashes.
//!
//! The fonts are TeX Live's `cmr10.pfb` with subrs added after its own and
//! the glyph `a` calling the first of them. Skips where there is no TeX Live
//! (e.g. CI).
#![cfg(feature = "kpathsea")]

use flashtex_engine::resolver::find_texlive_bin;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

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
/// `base+c` for each `c` of `calls(k)` in turn and then returning, and `a`
/// calling `base`; and `base`, the number of subrs `cmr10` has.
fn font(cmr10: &[u8], n: usize, calls: impl Fn(usize) -> Vec<usize>) -> (Vec<u8>, usize) {
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
        let mut code = vec![];
        for c in calls(k) {
            code.extend(num(base + c));
            code.push(CALLSUBR);
        }
        code.push(RETURN);
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
    // (the added subrs' binary data may hold the bytes "/a ")
    let dict = find(&plain, b"/CharStrings", 0).unwrap();
    let a = find(&plain, b"\n/a ", dict).unwrap() + 1;
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

    // pdfTeX's PDF, byte for byte: chains of 1,000 and 50,000 nested subrs,
    // and a binary tree of 4,095 subrs whose leaves all call one more
    let chain = |n: usize| {
        font(&cmr10, n, move |k| {
            (k + 1 < n).then_some(k + 1).into_iter().collect()
        })
    };
    let tree = |n: usize| {
        font(&cmr10, n + 1, move |k| {
            let kids: Vec<usize> = [2 * k + 1, 2 * k + 2]
                .into_iter()
                .filter(|&c| c < n)
                .collect();
            match (k == n, kids.is_empty()) {
                (true, _) => vec![],
                (false, true) => vec![n],
                (false, false) => kids,
            }
        })
    };
    for (name, (pfb, _)) in [
        ("chain1000", chain(1000)),
        ("chain50000", chain(50_000)),
        ("tree", tree(4095)),
    ] {
        let (a, b) = (base.join(name).join("ours"), base.join(name).join("tex"));
        assert!(
            run(ours, &a, &pfb, &tfm, true).success(),
            "{name}: ours failed"
        );
        assert!(
            run(&theirs, &b, &pfb, &tfm, false).success(),
            "{name}: pdfTeX failed"
        );
        let (x, y) = (
            std::fs::read(a.join("job.pdf")).unwrap(),
            std::fs::read(b.join("job.pdf")).unwrap(),
        );
        assert!(x == y, "{name}: job.pdf {} vs {} bytes", x.len(), y.len());
    }

    // a subr that calls itself, and a cycle of three: a font error, exit
    // status 1, no PDF. pdfTeX is not run on these: they crash it.
    // (name, font, the added subr that fails, the one it calls)
    let cases = [
        ("self", font(&cmr10, 1, |_| vec![0]), 0, 0),
        ("cycle", font(&cmr10, 3, |k| vec![(k + 1) % 3]), 2, 0),
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
            "(file ./nest.pfb): Subr ({}): cannot call subr ({}): it is already being called",
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
