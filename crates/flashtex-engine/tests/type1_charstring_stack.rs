//! The charstring operand stack of a subset Type 1 font, against TeX Live's
//! own `pdftex` (lane REVIEW-PDF).
//!
//! writet1.c's stack is a 24-entry array and `stack_ptr`: `cc_pop(N)` is
//! `stack_ptr -= N`, so a `callothersubr` whose count is below -2 (`a1 =
//! cc_get(-2) + 2 < 0`) makes the stack grow over what the array held, and
//! a later operator that checks its operands counts them. The engine's pop
//! of a negative count left the stack as it was, so for
//! `0 500 hsbw -5 7 callothersubr 0 0 rmoveto` pdfTeX stops with "more
//! arguments on stack (7) than required (2)" and the engine said 4.
//!
//! The font is TeX Live's `cmr10.pfb` with glyph `a` replaced; each job
//! typesets `a` (and `b`) without page numbers. Exit status, the PDF and the
//! log from the `**` line on (lines joined, the program name normalised)
//! must agree. Skips where there is no TeX Live.
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

/// A charstring number (Type 1 spec 6.2), for -107 to 1131.
fn num(v: i32) -> Vec<u8> {
    if (-107..=107).contains(&v) {
        vec![(v + 139) as u8]
    } else if (108..=1131).contains(&v) {
        let w = v - 108;
        vec![((w >> 8) + 247) as u8, (w & 0xff) as u8]
    } else {
        panic!("{v} is out of this helper's range")
    }
}

/// Charstring code: numbers and operators (`Op(13)` hsbw, `Esc(16)`
/// callothersubr, ...).
enum C {
    N(i32),
    Op(u8),
    Esc(u8),
}

fn code(ops: &[C]) -> Vec<u8> {
    let mut v = vec![0u8; 4]; // lenIV 4
    for op in ops {
        match *op {
            C::N(n) => v.extend(num(n)),
            C::Op(o) => v.push(o),
            C::Esc(o) => v.extend([12, o]),
        }
    }
    v
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

/// Which charstring of the font to replace.
enum Key {
    Glyph(&'static str),
    Subr(u32),
}

/// `cmr10` with the given glyphs' and subrs' charstrings replaced.
fn font(cmr10: &[u8], entries: &[(Key, Vec<C>)]) -> Vec<u8> {
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
    for (key, ops) in entries {
        // `/a 123 RD <bytes>` in /CharStrings, `dup 5 27 RD <bytes>` in /Subrs
        let (dict, head) = match key {
            Key::Glyph(g) => (&b"/CharStrings"[..], format!("/{g} ")),
            Key::Subr(k) => (&b"/Subrs"[..], format!("dup {k} ")),
        };
        let dict = find(&plain, dict, 0).unwrap();
        let at = find(&plain, format!("\n{head}").as_bytes(), dict).unwrap() + 1;
        let (len, e) = number(&plain, at + head.len());
        let data = e + b" RD ".len();
        let cs = encrypt(&code(ops), 4330);
        let mut entry = format!("{head}{} RD ", cs.len()).into_bytes();
        entry.extend_from_slice(&cs);
        plain.splice(at..data + len, entry);
    }
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

const HSBW: C = C::Op(13);
const RMOVETO: C = C::Op(21);
const ENDCHAR: C = C::Op(14);
const CALLOTHERSUBR: C = C::Esc(16);

const CALLSUBR: C = C::Op(10);
const RETURN: C = C::Op(11);

/// Glyphs `a` and `b`, and subrs 10 to 15, for REVIEW-A's finding on #1970:
/// a subr entered again, nested, with the same live stack (empty) and the
/// same marks, where only an entry above the stack differs. That entry is
/// what the subr's negative pop uncovers and calls, so the walk is not a
/// loop: S (10) grows the stack by one and calls the uncovered entry; T11,
/// T12 and T13 leave 12, 13 and `last` there and call S again. For `b`
/// every entry is marked already, so S is entered three times in states
/// that differ only in that entry. With `last` 14, R (14) empties the
/// stack and the font is fine; with 15, subr 15 sees three operands for
/// `rmoveto`, which shows that pdfTeX's walk follows the uncovered values.
fn uncovered_subrs(last: i32) -> Vec<(Key, Vec<C>)> {
    use C::N;
    let t = |next: i32| vec![N(next), N(3), N(0), CALLOTHERSUBR, N(10), CALLSUBR, RETURN];
    let glyph = || vec![N(0), N(500), HSBW, N(-3), N(0), N(11), CALLSUBR, ENDCHAR];
    vec![
        (Key::Glyph("a"), glyph()),
        (Key::Glyph("b"), glyph()),
        (
            Key::Subr(10),
            vec![N(-3), N(0), CALLOTHERSUBR, CALLSUBR, RETURN],
        ),
        (Key::Subr(11), t(12)),
        (Key::Subr(12), t(13)),
        (Key::Subr(13), t(last)),
        (Key::Subr(14), vec![N(2), N(0), CALLOTHERSUBR, RETURN]),
        (Key::Subr(15), vec![N(0), RMOVETO, RETURN]),
    ]
}

/// (name, the charstrings replaced, text).
type Case = (&'static str, Vec<(Key, Vec<C>)>, &'static str);

fn cases() -> Vec<Case> {
    use C::N;
    let a = |glyph: Vec<C>| vec![(Key::Glyph("a"), glyph)];
    vec![
        // a1 = -5 + 2 = -3: the stack grows by three, to 5, then 7
        (
            "negative-count",
            a(vec![
                N(0),
                N(500),
                HSBW,
                N(-5),
                N(7),
                CALLOTHERSUBR,
                N(0),
                N(0),
                RMOVETO,
                ENDCHAR,
            ]),
            "a",
        ),
        // a1 = 0: nothing is popped, and the font is fine
        (
            "zero-count",
            a(vec![
                N(0),
                N(500),
                HSBW,
                N(-2),
                N(7),
                CALLOTHERSUBR,
                RMOVETO,
                ENDCHAR,
            ]),
            "a",
        ),
        // two operands left after `a`'s walk: `b`'s hsbw sees four
        ("leftover", a(vec![N(0), N(500), HSBW, N(1), N(2)]), "a b"),
        // a subr entered again where only an uncovered entry differs: not a
        // loop, and the font is fine
        ("uncovered-entry", uncovered_subrs(14), "a b"),
        // the same, with the walk following the uncovered entry to subr 15
        ("uncovered-entry-followed", uncovered_subrs(15), "a b"),
    ]
}

fn kpsewhich(texbin: &Path, name: &str) -> PathBuf {
    let o = Command::new(texbin.join("kpsewhich"))
        .arg(name)
        .output()
        .unwrap();
    PathBuf::from(String::from_utf8(o.stdout).unwrap().trim())
}

fn run(bin: &Path, dir: &Path, ours: bool) -> Option<i32> {
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
fn charstring_stack_matches_tex_live() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let cmr10 = std::fs::read(kpsewhich(&texbin, "cmr10.pfb")).unwrap();
    let base = common::fresh_dir("flashtex-csstack");
    let mut failures = vec![];
    for (name, entries, text) in cases() {
        let pfb = font(&cmr10, &entries);
        let job = format!(
            "\\nopagenumbers \\pdfoutput=1 \\pdfcompresslevel=0 \\pdfobjcompresslevel=0 \
             \\pdfsuppressptexinfo=-1 \\pdfmapline{{=cmr10 CMR10 <stk.pfb}}{text}\\bye\n"
        );
        let (a, b) = (base.join(name).join("ours"), base.join(name).join("tex"));
        for d in [&a, &b] {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(d.join("stk.pfb"), &pfb).unwrap();
            std::fs::write(d.join("job.tex"), &job).unwrap();
        }
        let (rx, ry) = (run(ours, &a, true), run(&theirs, &b, false));
        if rx != ry {
            failures.push(format!("{name}: exit {rx:?} vs TeX Live's {ry:?}"));
        }
        if std::fs::read(a.join("job.pdf")).ok() != std::fs::read(b.join("job.pdf")).ok() {
            failures.push(format!(
                "{name}: the PDFs differ (or one engine wrote none)"
            ));
        }
        let lx = normalised_log(&std::fs::read_to_string(a.join("job.log")).unwrap_or_default());
        let ly = normalised_log(&std::fs::read_to_string(b.join("job.log")).unwrap_or_default());
        if lx != ly || lx.is_empty() {
            failures.push(format!("{name}: log\n  ours {lx:?}\n  tex  {ly:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    if std::env::var_os("FLASHTEX_KEEP_TEST_DIR").is_none() {
        let _ = std::fs::remove_dir_all(&base);
    }
}
