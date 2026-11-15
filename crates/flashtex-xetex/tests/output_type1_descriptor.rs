//! A TFM font drawn by its Type 1 program: the FontDescriptor's
//! `/CapHeight`, `/Ascent` and `/Descent` as xdvipdfmx's `get_font_attr`
//! (type1.c) takes them from the boxes `t1char_get_metrics` (t1_char.c)
//! gives glyphs `H`, `b` and `p` (issue #2401).
//!
//! The font is tests/data/type1's `ftxbox` (made by its mk.py): `H`'s top
//! is a `div` (683.5), `p`'s curve has control points below its end (the C
//! boxes the points, so -369), `b` is drawn in a Subr. Measured with TeX
//! Live 2026's `xetex -ini` + xdvipdfmx 20260317 on this input, with the
//! same files in the working directory: `/Ascent 694 /CapHeight 683.5
//! /Descent -369` (its FontBBox is `-10 -250 1000 900`).
//!
//! The program is embedded as xdvipdfmx embeds it: converted to a bare CFF
//! (`/FontFile3 /Subtype /Type1C`, type1.c `write_fontfile`) with the
//! descriptor's `/CharSet (/H/b/p)`. `ftxbox-Hpb.cff` is that stream from
//! the same run (subset tag `ZPECFV`).

use flashtex_pdf::reader::{Obj, PdfFile};
use std::path::PathBuf;
use std::process::Command;

const INPUT: &str =
    "\\catcode`\\{=1 \\catcode`\\}=2\n\\font\\x=ftxbox\n\\shipout\\hbox{\\x Hpb}\n\\end\n";

#[test]
fn type1_descriptor_metrics_are_xdvipdfmxs() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-t1desc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/type1");
    for f in ["ftxbox.pfb", "ftxbox.tfm", "pdftex.map"] {
        std::fs::copy(data.join(f), dir.join(f)).unwrap();
    }
    std::fs::write(dir.join("m.tex"), INPUT).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(&dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_RESOLVER", "cwd")
        .args(["-ini", "-interaction=nonstopmode", "m.tex"])
        .output()
        .expect("run flashtex-xetex");
    let bytes = std::fs::read(dir.join("m.pdf")).unwrap_or_else(|_| {
        panic!(
            "no PDF\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    let pdf = PdfFile::parse(&bytes).unwrap();
    let page = pdf.pages().unwrap()[0];
    let fonts = pdf.page_fonts(page);
    assert_eq!(fonts.len(), 1);
    let font = fonts.values().next().unwrap();
    assert_eq!(font.get("Subtype").and_then(Obj::as_name), Some("Type1"));
    let fd = pdf
        .resolve(font.get("FontDescriptor").unwrap())
        .as_dict()
        .unwrap();
    let num = |k: &str| fd.get(k).and_then(Obj::as_number).map(str::to_string);
    assert_eq!(num("Ascent").as_deref(), Some("694"));
    assert_eq!(num("CapHeight").as_deref(), Some("683.5"));
    assert_eq!(num("Descent").as_deref(), Some("-369"));
    // the program: xdvipdfmx's Type1C, byte for byte but for the tag
    let charset = match fd.get("CharSet") {
        Some(Obj::String(s)) => s.clone(),
        o => panic!("CharSet {o:?}"),
    };
    assert_eq!(charset, b"/H/b/p");
    assert_eq!(num("StemV").as_deref(), Some("80"));
    let ff = pdf.resolve(fd.get("FontFile3").expect("/FontFile3"));
    let Obj::Stream { dict, .. } = ff else {
        panic!("FontFile3 {ff:?}")
    };
    assert_eq!(dict.get("Subtype").and_then(Obj::as_name), Some("Type1C"));
    let got = pdf.decode_stream(ff).unwrap();
    let base = font.get("BaseFont").and_then(Obj::as_name).unwrap();
    let tag = base.split('+').next().unwrap().as_bytes();
    let mut got = got;
    if let Some(i) = got.windows(7).position(|w| w[..6] == *tag && w[6] == b'+') {
        got[i..i + 6].copy_from_slice(b"ZPECFV");
    }
    let want = std::fs::read(data.join("ftxbox-Hpb.cff")).unwrap();
    assert_eq!(got, want);
    let _ = std::fs::remove_dir_all(&dir);
}
