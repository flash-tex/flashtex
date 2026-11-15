//! A NUL (`^^@`) in a file name ends the name web2c gives kpathsea and
//! `fopen` (`nameoffile + 1` is a C string), while XeTeX keeps and prints
//! the whole name. TeX Live's XeTeX (0.999998, TeX Live 2026, macOS arm64
//! and x86_64-linux alike): `\openin` of `zz^^@b.tex` finds nothing, of
//! `b.tex^^@zz` or `b^^@.tex` opens `b.tex`; `\input b^^@zz` reads `b.tex`;
//! `\openout3={w^^@x.tex}` writes a file `w` and logs the whole name (with
//! XeTeX's own `I` after each `^^@` and at the end; in INITEX `\newlinechar`
//! is 0, so `^^@` prints as a line end). Found by lane PARITY-HUNT in TeX
//! Live's ftc-notebook example under xelatex (#2520). No TeX installation needed:
//! the expected log is XeTeX's for the same input.

use std::process::Command;

const DOC: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\^=7 \\catcode`\\^^@=12
\\immediate\\openin1={zz^^@b.tex} \\ifeof1 \\message{[C eof]}\\else\\message{[C open]}\\fi \\closein1
\\immediate\\openin1={b.tex^^@zz} \\ifeof1 \\message{[D eof]}\\else\\message{[D open]}\\fi \\closein1
\\immediate\\openin1={b^^@.tex} \\ifeof1 \\message{[E eof]}\\else\\message{[E open]}\\fi \\closein1
\\input b^^@zz \\message{[G]}
\\immediate\\openout3={w^^@x.tex}\\immediate\\write3{w}\\immediate\\closeout3
\\immediate\\openin1={w} \\ifeof1 \\message{[H eof]}\\else\\message{[H open]}\\fi \\closein1
\\end
";

/// TeX Live's XeTeX on `DOC`, from `**t.tex` on (the lines before it are
/// the banner and texmf.cnf's settings).
const XETEX_LOG: &str = "**t.tex
(./t.tex [C eof] [D open] [E open] (./b.tex [in b]) [G]
\\openout3 = `w
I
x.texI'.

 [H open] )
No pages of output.
";

#[test]
fn a_nul_ends_the_file_name_the_system_sees() {
    let d = std::env::temp_dir().join(format!(
        "flashtex-xetex-nul-file-name-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.tex"), DOC).unwrap();
    std::fs::write(d.join("b.tex"), "\\message{[in b]}\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .args(["-ini", "-no-pdf", "-interaction=nonstopmode", "t.tex"])
        .current_dir(&d)
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .output()
        .expect("run flashtex-xetex");
    let log = std::fs::read_to_string(d.join("t.log")).unwrap();
    let from = log.find("**t.tex").unwrap_or(0);
    assert_eq!(
        &log[from..],
        XETEX_LOG,
        "exit {:?}; terminal:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(std::fs::read(d.join("w")).ok().as_deref(), Some(&b"w\n"[..]));
    let _ = std::fs::remove_dir_all(&d);
}
