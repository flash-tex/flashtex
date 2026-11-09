//! writettf.c keeps `upem` in a file static that only `ttf_read_head` sets,
//! and `writeotf` reads `head` only when the font has one. An OpenType font
//! without a `head` table therefore scales its `hhea` Ascent/Descent by the
//! units per em of the font written before it (#2273). Measured with
//! pdfTeX 1.40.29 (TeX Live 2026): lmroman10-regular.otf with its `head`
//! tag renamed, written after Arvo-Bold.ttf (upem 2048), gets
//! `/Ascent 550 /Descent -141` (1127 and -290 over 2048); written after
//! lmroman12-regular.otf (upem 1000), `/Ascent 1127 /Descent -290`. Found
//! by lane AUDIT-PDFC. Skips where there is no TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const HEAD: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\pdfoutput=1 \\pdfcompresslevel=0 \
    \\pdfobjcompresslevel=0\n";

fn run(bin: &Path, dir: &Path, job: &str, otf: &[u8], ours: bool) -> Vec<u8> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("xead.otf"), otf).unwrap();
    std::fs::write(dir.join("job.tex"), format!("{HEAD}{job}")).unwrap();
    let mut c = Command::new(bin);
    c.args(["-ini", "-interaction=batchmode", "job.tex"])
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "1700000000")
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
    std::fs::read(dir.join("job.pdf")).unwrap_or_default()
}

fn kpsewhich(texbin: &Path, name: &str) -> PathBuf {
    let o = Command::new(texbin.join("kpsewhich"))
        .arg(name)
        .output()
        .unwrap();
    String::from_utf8(o.stdout).unwrap().trim().into()
}

/// The font with its `head` table directory tag renamed to `xead`.
fn without_head(font: &[u8]) -> Vec<u8> {
    let mut f = font.to_vec();
    let n = u16::from_be_bytes([f[4], f[5]]) as usize;
    let mut renamed = false;
    for i in 0..n {
        let t = 12 + 16 * i;
        if &f[t..t + 4] == b"head" {
            f[t] = b'x';
            renamed = true;
        }
    }
    assert!(renamed, "the font has a head table");
    f
}

fn count(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len()).filter(|w| *w == needle).count()
}

#[test]
fn an_otf_without_head_uses_the_previous_fonts_upem() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let lm = kpsewhich(&texbin, "lmroman10-regular.otf");
    if lm.as_os_str().is_empty() || kpsewhich(&texbin, "Arvo-Bold.ttf").as_os_str().is_empty() {
        common::no_texlive();
        return;
    }
    let otf = without_head(&std::fs::read(lm).unwrap());
    let base = common::fresh_dir("flashtex-otf-stale-upem");
    let cases = [
        // a TrueType font (Arvo, upem 2048) written first
        (
            "\\pdfmapline{=cmr10 LMRoman10-Regular <<xead.otf}\
             \\pdfmapline{=Arvo-Bold-tlf-ot1 ArvoB <<Arvo-Bold.ttf}\n\
             \\font\\g=cmr10 \\font\\f=Arvo-Bold-tlf-ot1 \\shipout\\hbox{\\g AB\\f AB}\n\\end\n",
            &b"/Ascent 550\n/CapHeight"[..],
        ),
        // an OpenType font (upem 1000) written first
        (
            "\\pdfmapline{=cmr10 LMRoman10-Regular <<xead.otf}\
             \\pdfmapline{=cmr12 LMRoman12-Regular <<lmroman12-regular.otf}\n\
             \\font\\g=cmr10 \\font\\h=cmr12 \\shipout\\hbox{\\g AB\\h AB}\n\\end\n",
            &b"/Descent -290\n"[..],
        ),
    ];
    for (i, (job, mark)) in cases.iter().enumerate() {
        let want = run(&theirs, &base.join(format!("theirs{i}")), job, &otf, false);
        let got = run(ours, &base.join(format!("ours{i}")), job, &otf, true);
        assert!(!want.is_empty(), "case {i}: pdfTeX wrote no PDF");
        assert_eq!(
            count(&want, mark),
            1,
            "case {i}: pdfTeX's stale-upem metric"
        );
        assert!(got == want, "case {i}: the PDFs differ");
    }
    let _ = std::fs::remove_dir_all(&base);
}
