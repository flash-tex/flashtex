//! `pdf:tounicode` (xdvipdfmx's spc_pdfm.c, `spc_handler_pdfm_tounicode`,
//! `modify_strings`, `reencode_string`; issue #2345): after it, the text
//! strings of `pdf:ann`, `pdf:bann`, `pdf:outline`, `pdf:docinfo` and
//! `pdf:docview` (ASCII ones too) are decoded through the CMap, the array's
//! names become text keys, and a Shift-JIS-like CMap's name makes the
//! dictionaries be read "tainted".
//!
//! Every expected value was measured with TeX Live 2026's `xetex -ini`
//! (xdvipdfmx 20260317) on these inputs, with the CMaps of
//! tests/data/tounicode in the working directory (a part of TeX Live's
//! `UTF8-UCS2`, which gives what the whole file gives for these strings,
//! and two small test CMaps, one using the other).

use flashtex_pdf::reader::{Obj, PdfFile};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

const HEAD: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6\n\
\\font\\x=\"[lmmono10-regular.otf]\" at 10pt\n";

fn run(name: &str, body: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!(
        "flashtex-xetex-tounicode-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    std::fs::copy(
        here.join("../../apps/mac/Fonts/lmmono10-regular.otf"),
        dir.join("lmmono10-regular.otf"),
    )
    .unwrap();
    for c in ["UTF8-UCS2", "T-RKSJ-H", "T-RKSJ-V"] {
        std::fs::copy(here.join("tests/data/tounicode").join(c), dir.join(c)).unwrap();
    }
    std::fs::write(dir.join("t.tex"), format!("{HEAD}{body}\n\\end\n")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(&dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_RESOLVER", "cwd")
        .args(["-ini", "-interaction=nonstopmode", "t.tex"])
        .output()
        .expect("run flashtex-xetex");
    let pdf = std::fs::read(dir.join("t.pdf")).unwrap_or_else(|_| {
        panic!(
            "no PDF\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    let _ = std::fs::remove_dir_all(&dir);
    pdf
}

fn hex(s: &str) -> Obj {
    Obj::String(
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect(),
    )
}

fn outline_titles(pdf: &PdfFile) -> Vec<BTreeMap<String, Obj>> {
    let cat = pdf.catalog().unwrap();
    let outlines = pdf.resolve(cat.get("Outlines").unwrap()).as_dict().unwrap();
    let mut out = Vec::new();
    let mut next = outlines.get("First");
    while let Some(o) = next {
        let d = pdf.resolve(o).as_dict().unwrap();
        out.push(d.clone());
        next = d.get("Next");
    }
    out
}

/// The issue's input with more: before `pdf:tounicode`, UTF-8 strings of
/// the default text keys only; after it, every string of a text key,
/// `/NM` and `/Foo` included, through UTF8-UCS2 (U+FFFD for what is
/// outside its codespace, one per byte).
#[test]
fn tounicode_utf8_ucs2() {
    let bytes = run(
        "utf8",
        "\\shipout\\hbox{\\x x\\special{pdf:docinfo <</Title (Café ∑) /Author (plain) /Foo (Café)>>}\
\\special{pdf:outline 1 <</Title (été)>>}\
\\special{pdf:ann width 10pt height 10pt depth 0pt <</Type/Annot/Subtype/Text/Contents (naïve)/NM (naïve)>>}\
\\special{pdf:tounicode UTF8-UCS2 [/Foo /NM]}\
\\special{pdf:ann width 10pt height 10pt depth 0pt <</Type/Annot/Subtype/Text/Contents (naïve)/NM (naïve)/T (ascii) /TU () /Subj [(a) (b) <</T (x𝔸)>>] /Foo <FEFF0041>>>}\
\\special{pdf:outline 1 <</Title (plain ascii)>>}\
\\special{pdf:docinfo <</Subject (ascii) /Keywords <ff41c3> /Creator (a)>>}}",
    );
    let pdf = PdfFile::parse(&bytes).unwrap();
    let info = pdf.info().unwrap();
    assert_eq!(
        info.get("Title"),
        Some(&hex("feff00430061006600e900202211"))
    );
    assert_eq!(info.get("Author"), Some(&Obj::String(b"plain".to_vec())));
    assert_eq!(info.get("Foo"), Some(&hex("436166c3a9")));
    assert_eq!(info.get("Subject"), Some(&hex("feff00610073006300690069")));
    assert_eq!(info.get("Keywords"), Some(&hex("fefffffd0041fffd")));
    assert_eq!(info.get("Creator"), Some(&hex("feff0061")));
    let t = outline_titles(&pdf);
    assert_eq!(t[0].get("Title"), Some(&hex("feff00e9007400e9")));
    assert_eq!(
        t[1].get("Title"),
        Some(&hex("feff0070006c00610069006e002000610073006300690069"))
    );
    let page = pdf.pages().unwrap()[0];
    let annots = pdf
        .page_attr(page, "Annots")
        .and_then(Obj::as_array)
        .unwrap();
    let a0 = pdf.resolve(&annots[0]).as_dict().unwrap();
    let a1 = pdf.resolve(&annots[1]).as_dict().unwrap();
    assert_eq!(a0.get("Contents"), Some(&hex("feff006e006100ef00760065")));
    assert_eq!(a0.get("NM"), Some(&hex("6e61c3af7665")));
    assert_eq!(a1.get("Contents"), Some(&hex("feff006e006100ef00760065")));
    assert_eq!(a1.get("NM"), Some(&hex("feff006e006100ef00760065")));
    assert_eq!(a1.get("T"), Some(&hex("feff00610073006300690069")));
    assert_eq!(a1.get("TU"), Some(&hex("feff")));
    assert_eq!(a1.get("Foo"), Some(&hex("feff0041")));
    let subj = a1.get("Subj").and_then(Obj::as_array).unwrap();
    assert_eq!(subj[0], hex("feff0061"));
    assert_eq!(subj[1], hex("feff0062"));
    assert_eq!(
        subj[2].as_dict().unwrap().get("T"),
        Some(&hex("feff0078fffdfffdfffdfffd"))
    );
}

/// A CMap that does not load adds no keys and leaves UTF-8 reading;
/// `Identity-H` copies the bytes; a key that is not in an array is not
/// added.
#[test]
fn tounicode_failed_identity_and_bad_keys() {
    let bytes = run(
        "ident",
        "\\shipout\\hbox{\\x x\\special{pdf:tounicode NoSuchCMap [/Foo]}\
\\special{pdf:docinfo <</Title (Café) /Foo (Café) /Author (ascii)>>}\
\\special{pdf:tounicode Identity-H [/Bar 3]}\
\\special{pdf:outline 1 <</Title (abcd) /Bar (wxyz)>>}\
\\special{pdf:tounicode UTF8-UCS2 /Baz}\
\\special{pdf:outline 1 <</Title <c3> /T <e2> /Baz (é)>>}}",
    );
    let pdf = PdfFile::parse(&bytes).unwrap();
    let info = pdf.info().unwrap();
    assert_eq!(info.get("Title"), Some(&hex("feff00430061006600e9")));
    assert_eq!(info.get("Foo"), Some(&hex("436166c3a9")));
    assert_eq!(info.get("Author"), Some(&Obj::String(b"ascii".to_vec())));
    let t = outline_titles(&pdf);
    assert_eq!(t[0].get("Title"), Some(&hex("feff61626364")));
    assert_eq!(t[0].get("Bar"), Some(&hex("feff7778797a")));
    assert_eq!(t[1].get("Title"), Some(&hex("fefffffd")));
    assert_eq!(t[1].get("T"), Some(&hex("fefffffd")));
    assert_eq!(t[1].get("Baz"), Some(&hex("c3a9")));
}

/// A CMap named like a Shift-JIS one (`RKSJ`) makes literal strings read
/// tainted: a byte above 127 takes the next byte as it is, so `∑\101`
/// (E2 88 91) keeps its backslash. The CMap uses another (`usecmap`).
/// After a `pdf:tounicode` that fails, strings are read plainly again.
#[test]
fn tounicode_tainted_and_usecmap() {
    let bytes = run(
        "rksj",
        "\\shipout\\hbox{\\x x\\special{pdf:tounicode T-RKSJ-V}\
{\\catcode`\\|=0 \\catcode`\\\\=12 |special{pdf:docinfo <</Title (ab∑\\101) /Author (é\\)) /Subject <c3a9915c>>>}\
|special{pdf:tounicode T-B5-X}|special{pdf:outline 1 <</Title (ab∑\\101)>>}}}",
    );
    let pdf = PdfFile::parse(&bytes).unwrap();
    let info = pdf.info().unwrap();
    assert_eq!(
        info.get("Title"),
        Some(&hex("feff03840043025802bc001200110012"))
    );
    assert_eq!(info.get("Author"), Some(&hex("feff01f4000a")));
    assert_eq!(info.get("Subject"), Some(&hex("feff01f402bc")));
    let t = outline_titles(&pdf);
    assert_eq!(t[0].get("Title"), Some(&hex("feff0061006222110041")));
}
