//! ML\TeX (tex.ch, pdftex.ch; changes/tex-mltex.ch, changes/pdftex-mltex.ch)
//! and encTeX (enctexdir/enctex*.ch; changes/enctex1.ch, enctex-pdftex.ch,
//! enctex2.ch) against TeX Live's pdfTeX:
//!
//! - the string table a format holds is pdfTeX's, string for string: the
//!   pool strings of both features are there in every run (#2039);
//! - `-mltex`: `\charsubdef`, its tracing, the substitution `hlist_out` and
//!   `pdf_hlist_out` output and their `Missing character` messages, the
//!   `MLTeX v2.2 enabled` lines of a run from such a format;
//! - `-enc`: `\mubyte` (to a byte, to a control sequence, its errors),
//!   `\noconvert`, `\xordcode` and its kin, `\mubyteout`'s conversion in
//!   `\write`, `\specialout` in `\special`, the encTeX banner of a run from
//!   such a format;
//! - without `-ini` both options only print pdfTeX's warning.
//!
//! Both engines run as `pdftex` on the same input; terminal output, stderr,
//! the log (after the banner line), the DVI file and the exit status are
//! compared. Skips where there is no TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, PartialEq)]
struct Out {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    log: Option<String>,
    dvi: Option<Vec<u8>>,
}

/// Everything after the first line (the banner, whose date the log has).
fn after_banner(s: &str) -> String {
    s.split_once('\n')
        .map(|(_, b)| b.to_string())
        .unwrap_or_default()
}

/// The two sides of one test: this build's engine and TeX Live's pdfTeX,
/// each as `pdftex` in its own directory with the same files.
struct Sides {
    base: PathBuf,
    texbin: PathBuf,
}

impl Sides {
    fn new(name: &str, files: &[(&str, &[u8])]) -> Option<Sides> {
        let Some(texbin) = find_texlive_bin() else {
            common::no_texlive();
            return None;
        };
        let base = common::fresh_dir(&format!("flashtex-mltex-enctex-{name}"));
        for sub in ["ours", "tex"] {
            let d = base.join(sub);
            std::fs::create_dir_all(&d).unwrap();
            for (f, b) in files {
                std::fs::write(d.join(f), b).unwrap();
            }
        }
        common::link_engine(
            Path::new(env!("CARGO_BIN_EXE_flashtex-initex")),
            &base.join("ours").join("pdftex"),
        );
        Some(Sides { base, texbin })
    }

    /// Runs `args` on both sides; returns (ours, pdfTeX's).
    fn run(&self, args: &[&str], job: &str) -> (Out, Out) {
        let one = |ours: bool| {
            let d = self.base.join(if ours { "ours" } else { "tex" });
            let bin = if ours {
                d.join("pdftex")
            } else {
                self.texbin.join("pdftex")
            };
            let _ = std::fs::remove_file(d.join(format!("{job}.log")));
            let _ = std::fs::remove_file(d.join(format!("{job}.dvi")));
            let mut c = Command::new(bin);
            c.args(args)
                .current_dir(&d)
                .env("SOURCE_DATE_EPOCH", "0")
                .env("FORCE_SOURCE_DATE", "1")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            if ours {
                c.env("FLASHTEX_FORMATS", &d).env(
                    "FLASHTEX_POOL",
                    Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
                );
            }
            let out = c.output().unwrap();
            Out {
                code: out.status.code(),
                stdout: after_banner(&String::from_utf8_lossy(&out.stdout)),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
                // lossy: the logs hold bytes that are not UTF-8
                log: std::fs::read(d.join(format!("{job}.log")))
                    .ok()
                    .map(|l| after_banner(&String::from_utf8_lossy(&l))),
                dvi: std::fs::read(d.join(format!("{job}.dvi"))).ok(),
            }
        };
        (one(true), one(false))
    }

    fn file(&self, ours: bool, name: &str) -> Vec<u8> {
        std::fs::read(self.base.join(if ours { "ours" } else { "tex" }).join(name))
            .unwrap_or_else(|e| panic!("{name}: {e}"))
    }
}

impl Drop for Sides {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = std::fs::remove_dir_all(&self.base);
        }
    }
}

fn assert_same(name: &str, x: &Out, y: &Out) {
    assert_eq!(x.log, y.log, "{name}: log differs from pdftex");
    assert_eq!(
        x.stdout, y.stdout,
        "{name}: terminal output differs from pdftex"
    );
    assert_eq!(x.stderr, y.stderr, "{name}: stderr differs from pdftex");
    assert_eq!(x.dvi, y.dvi, "{name}: DVI file differs from pdftex");
    assert_eq!(x.code, y.code, "{name}: exit status differs from pdftex");
}

/// `n strings of total length m`, as the log of a `\dump` says.
fn string_counts(log: &str) -> (usize, usize) {
    let l = log
        .lines()
        .find(|l| l.contains(" strings of total length "))
        .unwrap_or_else(|| panic!("no string count in the log:\n{log}"));
    let mut w = l.split_whitespace();
    let n = w.next().unwrap().parse().unwrap();
    let m = w.last().unwrap().parse().unwrap();
    (n, m)
}

/// The strings of a format: `str_ptr` strings of `pool_ptr` characters,
/// found after the pair (`pool_ptr`, `str_ptr`) and `str_start[0..=str_ptr]`,
/// in integers of `width` bytes read by `int`, then the pool characters,
/// which `chars` reads from there.
fn strings_after(
    data: &[u8],
    pool_ptr: usize,
    str_ptr: usize,
    width: usize,
    int: impl Fn(&[u8]) -> i64,
    chars: impl Fn(&[u8], usize) -> Vec<u8>,
) -> Vec<Vec<u8>> {
    let at = (0..data.len().saturating_sub(2 * width))
        .find(|&i| {
            int(&data[i..i + width]) == pool_ptr as i64
                && int(&data[i + width..i + 2 * width]) == str_ptr as i64
                && int(&data[i + 2 * width..i + 3 * width]) == 0
        })
        .expect("no string pool in the format");
    let starts: Vec<usize> = (0..=str_ptr)
        .map(|k| int(&data[at + (2 + k) * width..at + (3 + k) * width]) as usize)
        .collect();
    assert_eq!(starts[str_ptr], pool_ptr);
    let pool = chars(&data[at + (3 + str_ptr) * width..], pool_ptr);
    (0..str_ptr)
        .map(|k| pool[starts[k]..starts[k + 1]].to_vec())
        .collect()
}

/// TeX Live's format: gzip (zlib-fmt.ch), big-endian 4-byte integers,
/// `str_pool` as bytes (texmfmp.h's `dump_things`).
fn texlive_strings(fmt: &[u8], pool_ptr: usize, str_ptr: usize) -> Vec<Vec<u8>> {
    let mut gz = Command::new("gzip")
        .arg("-dc")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("gzip");
    let mut stdin = gz.stdin.take().unwrap();
    let input = fmt.to_vec();
    let feed = std::thread::spawn(move || {
        use std::io::Write;
        stdin.write_all(&input).unwrap();
    });
    let out = gz.wait_with_output().unwrap();
    feed.join().unwrap();
    assert!(out.status.success(), "gzip -dc failed");
    strings_after(
        &out.stdout,
        pool_ptr,
        str_ptr,
        4,
        |b| i32::from_be_bytes(b.try_into().unwrap()) as i64,
        |b, n| b[..n].to_vec(),
    )
}

/// This engine's format: pdftex.web's `dump_int` of 8-byte memory words
/// (little-endian), the pool four characters to a word
/// (`dump_four_ASCII`, the quarterwords |b0|..|b3|), the last word holding
/// `str_pool[pool_ptr-4..pool_ptr]`.
fn our_strings(fmt: &[u8], pool_ptr: usize, str_ptr: usize) -> Vec<Vec<u8>> {
    strings_after(
        fmt,
        pool_ptr,
        str_ptr,
        8,
        |b| i64::from_le_bytes(b.try_into().unwrap()),
        |b, n| {
            // which bytes of a word are b0..b3: the pool starts with `^^@^`
            let lay = [[0, 2, 4, 6], [0, 1, 2, 3], [6, 4, 2, 0], [3, 2, 1, 0]]
                .into_iter()
                .find(|l| l.map(|j| b[j]) == *b"^^@^")
                .expect("no four_quarters layout");
            let word = |w: usize| lay.map(|j| b[8 * w + j]);
            let mut pool = vec![];
            let mut k = 0;
            while k + 4 < n {
                pool.extend(word(k / 4));
                k += 4;
            }
            pool.truncate(n - 4);
            pool.extend(word(k / 4));
            pool
        },
    )
}

/// The string table of a format INITEX makes is pdfTeX's, string for
/// string: pdftex.web's and the change files' strings in tangle's order,
/// ML\TeX's and encTeX's among them, then the run's (#2039).
#[test]
fn a_format_s_string_table_is_pdftex_s() {
    let tex = b"\\catcode`\\{=1 \\catcode`\\}=2 \\font\\f=cmr10 \\def\\x{y}\\dump\n";
    let Some(s) = Sides::new("pool", &[("t.tex", tex)]) else {
        return;
    };
    let (x, y) = s.run(&["-ini", "-etex", "-interaction=nonstopmode", "t.tex"], "t");
    let (xl, yl) = (x.log.unwrap(), y.log.unwrap());
    let (n, m) = string_counts(&yl);
    assert_eq!(string_counts(&xl), (n, m), "the string counts differ");
    let ours = our_strings(&s.file(true, "t.fmt"), m, n);
    let theirs = texlive_strings(&s.file(false, "t.fmt"), m, n);
    for (k, (a, b)) in ours.iter().zip(&theirs).enumerate() {
        assert_eq!(
            String::from_utf8_lossy(a),
            String::from_utf8_lossy(b),
            "string {k} differs from pdftex's"
        );
    }
    assert_eq!(ours.len(), theirs.len());
}

const MLTEX_FMT: &[u8] = b"\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\catcode`\\^=7
\\tracingonline=1 \\tracingcharsubdef=1
\\showthe\\charsubdefmax
\\charsubdef 128=127 `a  % a dieresis: accent 127, base a
\\charsubdef 129 18 101 % e grave
\\charsubdef 130=200 `o % no accent 200 in cmr10
\\charsubdef 131=127 200 % nor a base 200
\\showthe\\charsubdefmax \\charsubdefmin % not a primitive (tex.ch's `if false')
\\let\\c\\charsubdef \\show\\c \\showthe\\tracingcharsubdef
\\font\\f=cmr10 \\f \\tracinglostchars=100 \\pdfoutput=0
\\setbox0\\hbox{^^80^^81^^82^^83^^84 x\\char128}
\\showbox0 \\showboxdepth=5 \\showboxbreadth=100 \\showbox0
\\shipout\\copy0
{\\tracinglostchars=1 \\charsubdef 128=0 0 \\shipout\\copy0}
\\shipout\\vbox{\\hrule\\hbox{\\f ^^80^^81}\\leaders\\hbox{^^81}\\hskip 30pt}
\\dump
";

const MLTEX_RUN: &[u8] = b"\\tracingonline=1 \\tracinglostchars=100 \\pdfoutput=1
\\pdfcompresslevel=0 \\pdfobjcompresslevel=0
\\setbox0\\hbox{\\f ^^80^^81^^82^^83 \\charsubdef 132=127 `u ^^84}
\\showbox0
\\shipout\\box0
\\end
";

#[test]
fn mltex_substitutes_as_pdftex_does() {
    let Some(s) = Sides::new("mltex", &[("f.tex", MLTEX_FMT), ("t.tex", MLTEX_RUN)]) else {
        return;
    };
    let (x, y) = s.run(
        &["-ini", "-mltex", "-interaction=nonstopmode", "f.tex"],
        "f",
    );
    let yl = y.log.as_deref().unwrap_or_default();
    for m in [
        "MLTeX v2.2 enabled",
        "New character substitution: ^^80 = ^^? a",
        "Using character substitution: ^^80 = ^^? a in font cmr10.",
        "Missing character: Incomplete substitution ^^82 = ^^c8 o in font cmr10!",
    ] {
        assert!(yl.contains(m), "pdftex's log lacks {m:?}:\n{yl}");
    }
    assert_same("-ini -mltex", &x, &y);

    // a run from the format: the banner lines, pdf_hlist_out's substitution
    let (x, y) = s.run(&["-fmt=f", "-interaction=nonstopmode", "t.tex"], "t");
    assert!(
        y.stdout.contains("MLTeX v2.2 enabled"),
        "pdftex's terminal lacks the MLTeX line:\n{}",
        y.stdout
    );
    assert_same("-fmt (MLTeX)", &x, &y);
    assert_eq!(
        s.file(true, "t.pdf"),
        s.file(false, "t.pdf"),
        "-fmt (MLTeX): the PDF file differs from pdftex's"
    );

    // without -ini: only the warning, and no \charsubdef
    let (x, y) = s.run(
        &[
            "-mltex",
            "-enc",
            "-fmt=f",
            "-interaction=nonstopmode",
            "t.tex",
        ],
        "t",
    );
    assert!(
        y.stderr.contains("-mltex only works with -ini") && y.stderr.contains("-enc only works"),
        "pdftex printed no warning: {:?}",
        y.stderr
    );
    assert_same("-mltex -enc without -ini", &x, &y);
}

/// UTF-8's bytes of `é`, `à`, `—`, `“` and `§`: to a character, to a control
/// sequence, to one with a prefix (`#2`), to one that is only written (`\relax`).
const ENCTEX_FMT: &[u8] = b"\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\catcode`\\^=7
\\tracingonline=1 \\showthe\\mubytein
\\mubyte ^^e9 \xc3\xa9\\endmubyte
\\mubyte ^^e0 \xc3\xa0\\endmubyte
\\mubyte \\dash \xe2\x80\x94\\endmubyte
\\mubyte \\pfx #2 \xe2\x80\x9c\\endmubyte
\\mubyte \\out \\relax \xc2\xa7\\endmubyte
\\mubyte ^^ff ^^c3^^bf\\endmubyte % to the bytes ^^c3^^bf, by ^^ notation
\\def\\dash{[DASH]} \\def\\pfx#1{(#1)} \\def\\out{OUT}
\\mubyte ^^e0 \\endmubyte
\\mubyte x\\endmubyte
\\mubyte ^^e8 \xc3\\relax\\endmubyte
\\endmubyte
\\let\\m\\mubyte \\show\\m \\show\\noconvert \\show\\endmubyte \\show\\mubytein
\\mubytein=1 \\mubyteout=3 \\mubytelog=1 \\specialout=1
\\message{[\xc3\xa9][\xc3\xa0][\xe2\x80\x94][\xe2\x80\x9cab]}
\\showthe\\mubytein
\\xordcode`A=`B \\xchrcode`B=`C \\xprncode 200=2
\\showthe\\xordcode`A \\showthe\\xchrcode`B \\showthe\\xprncode200 \\showthe\\xprncode`a
\\immediate\\write-1{\\out\\noconvert\\out ^^e9\\noconvert^^e9}
{\\mubyteout=0 \\immediate\\write-1{\\out ^^e9}}
{\\mubytelog=0 \\message{^^e9}}
\\pdfoutput=0 \\font\\f=cmr10
\\shipout\\hbox{\\f x\\special{\\out ^^e9}\\write-1{\\out ^^e9}}
{\\specialout=0 \\shipout\\hbox{\\special{\\out ^^e9}}}
\\dump
";

const ENCTEX_RUN: &[u8] = b"\\tracingonline=1
\\message{[\xc3\xa9][\xe2\x80\x94]}
\\showthe\\mubytein \\show\\mubyte
\\end
";

#[test]
fn enctex_converts_as_pdftex_does() {
    let Some(s) = Sides::new("enctex", &[("f.tex", ENCTEX_FMT), ("t.tex", ENCTEX_RUN)]) else {
        return;
    };
    let (x, y) = s.run(&["-ini", "-enc", "-interaction=nonstopmode", "f.tex"], "f");
    let yl = y.log.as_deref().unwrap_or_default();
    for m in ["encTeX v. Jun. 2004", "\\mubyte", "[DASH]"] {
        assert!(yl.contains(m), "pdftex's log lacks {m:?}:\n{yl}");
    }
    assert_same("-ini -enc", &x, &y);

    // a run from the format: its tables were dumped
    let (x, y) = s.run(&["-fmt=f", "-interaction=nonstopmode", "t.tex"], "t");
    assert!(
        y.stdout
            .contains("encTeX v. Jun. 2004, reencoding enabled."),
        "pdftex's terminal lacks the encTeX line:\n{}",
        y.stdout
    );
    assert_same("-fmt (encTeX)", &x, &y);
}
