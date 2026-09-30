//! The PDF backend (src/pdftex/: font map, Type 1 subsetting, encodings,
//! ToUnicode, zlib) against TeX Live's own `pdftex`: build plain.fmt with each,
//! typeset small documents in PDF mode, and require byte-identical PDFs and
//! the same backend log lines (`{...pdftex.map}`, `{...enc}`, `<...pfb>`,
//! "Output written on"). `\pdfsuppressptexinfo=-1` leaves out the
//! `/PTEX.Fullbanner`, the one PDF entry that names web2c's version string.
//! Both runs use `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, so the dates and
//! the `/ID` agree. Skips where there is no TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

const SETUP: &str = "\\pdfoutput=1 \\pdfsuppressptexinfo=-1 \\pdfminorversion=7 \
    \\pdfobjcompresslevel=2 \\pdfcompresslevel=9 \\pdfdecimaldigits=3 \
    \\pdfpagewidth=8.5truein \\pdfpageheight=11truein \
    \\pdfhorigin=1truein \\pdfvorigin=1truein\n";

/// (name, body): each is typeset after `SETUP` with plain TeX.
const CASES: &[(&str, &str)] = &[
    (
        "cm",
        "Hello, world! {\\bf Bold} {\\it italic} $x^2+\\alpha\\le\\sum_i y_i$\n\\bye\n",
    ),
    (
        "enc",
        "\\font\\t=ec-lmr10 \\t Caf\\'e na\\\"\\i ve ``quotes'' --- fi ffl\n\\bye\n",
    ),
    (
        "slant",
        "\\pdfmapline{=cmr10 CMR10 \"0.167 SlantFont\" <cmr10.pfb}\
         \\pdfmapline{=cmbx10 CMBX10 \"1.2 ExtendFont\" <cmbx10.pfb}\
         Slanted {\\bf extended}\n\\bye\n",
    ),
    (
        "full",
        "\\pdfmapline{=cmr10 CMR10 <<cmr10.pfb}Whole font\n\\bye\n",
    ),
    (
        "tounicode",
        "\\input glyphtounicode \\pdfgentounicode=1 \\font\\t=ec-lmr10 \
         Text {\\t Caf\\'e} $\\alpha$\n\\bye\n",
    ),
    (
        "plain-streams",
        "\\pdfcompresslevel=0 \\pdfobjcompresslevel=0 Uncompressed $x$\n\\bye\n",
    ),
];

fn run(bin: &Path, dir: &Path, args: &[&str], ours: bool) {
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
    c.status().unwrap();
}

/// The backend's lines of a log (lines joined first: TeX breaks them at 79).
fn backend_lines(log: &str) -> Vec<String> {
    let text: String = log.lines().collect();
    let mut out = vec![];
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let rest = &text[i..];
        let (open, close) = match b[i] {
            b'{' => ('{', '}'),
            b'<' => ('<', '>'),
            _ => {
                if rest.starts_with("Output written on ") {
                    out.push(rest[..rest.find(')').map_or(rest.len(), |j| j + 1)].to_string());
                }
                i += 1;
                continue;
            }
        };
        match rest[1..].find([close, open]) {
            Some(j) if rest.as_bytes()[j + 1] == close as u8 => {
                let item = &rest[..j + 2];
                if item.ends_with(".map}")
                    || item.ends_with(".enc}")
                    || item.ends_with(".pfb>")
                    || item.ends_with(".pfb>>")
                {
                    out.push(item.to_string());
                }
                i += j + 2;
            }
            _ => i += 1,
        }
    }
    out
}

#[test]
fn pdf_backend_matches_tex_live() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let base = std::env::temp_dir().join(format!("flashtex-pdf-{}", std::process::id()));
    let (a, b) = (base.join("ours"), base.join("tex"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
        for (name, body) in CASES {
            std::fs::write(d.join(format!("{name}.tex")), format!("{SETUP}{body}")).unwrap();
        }
    }
    run(ours, &a, &["-ini", "\\input plain \\dump"], true);
    run(&theirs, &b, &["-ini", "\\input plain \\dump"], false);
    let mut failures = vec![];
    for (name, _) in CASES {
        let arg = format!("&plain {name}");
        run(ours, &a, &[&arg], true);
        run(&theirs, &b, &[&arg], false);
        let pdf = format!("{name}.pdf");
        let (x, y) = (std::fs::read(a.join(&pdf)), std::fs::read(b.join(&pdf)));
        match (x, y) {
            (Ok(x), Ok(y)) if x == y => {}
            (Ok(x), Ok(y)) => {
                let at = x
                    .iter()
                    .zip(&y)
                    .position(|(p, q)| p != q)
                    .unwrap_or(x.len().min(y.len()));
                failures.push(format!(
                    "{pdf}: differs at byte {at} ({} vs {} bytes)",
                    x.len(),
                    y.len()
                ));
            }
            (x, y) => failures.push(format!(
                "{pdf}: ours {}, TeX Live's {}",
                if x.is_ok() { "written" } else { "missing" },
                if y.is_ok() { "written" } else { "missing" }
            )),
        }
        let log = format!("{name}.log");
        let lx = backend_lines(&std::fs::read_to_string(a.join(&log)).unwrap_or_default());
        let ly = backend_lines(&std::fs::read_to_string(b.join(&log)).unwrap_or_default());
        if lx != ly || lx.is_empty() {
            failures.push(format!("{log}: backend lines {lx:?} vs {ly:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    let _ = std::fs::remove_dir_all(&base);
}
