//! PDF mode's PDF (docs/design/xetex/PLAN.md §3.2, `src/out/pdf.rs`),
//! without TeX Live: INITEX ships two pages with a native font, colour, a
//! link, a destination, an outline entry and document information, and the
//! PDF is read back (FlashTeX's own reader, `flashtex_pdf::reader`) and
//! checked. Whole documents against `xelatex`'s PDF are
//! tools/xetex-pdfparity's.

use flashtex_pdf::reader::{Obj, PdfFile};
use std::path::PathBuf;
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2 \catcode`\#=6
\pdfpagewidth=612bp \pdfpageheight=792bp
\font\x="[lmmono10-regular.otf]" at 10pt
\shipout\hbox{\special{pdf:docinfo <</Title(Test)/Subject()>>}%
\special{pdf:outline [] 1 <</Title(First)/A<</S/GoTo/D(here)>>>>}%
\special{pdf:dest (here) [@thispage /XYZ @xpos @ypos null]}%
\special{color push rgb 1 0 0}%
\special{pdf:bann <</Type/Annot/Subtype/Link/Border[0 0 1]/A<</S/URI/URI(https://example.com/)>>>>}%
\x AB\special{pdf:eann}\special{color pop}}
\shipout\hbox{\x C}
\end
"#;

fn run_dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("flashtex-xetex-outpdf-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn pdf_mode_writes_the_pdf_from_the_display_list() {
    let dir = run_dir();
    let font =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts/lmmono10-regular.otf");
    std::fs::copy(&font, dir.join("lmmono10-regular.otf")).unwrap();
    std::fs::write(dir.join("t.tex"), INPUT).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(&dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_RESOLVER", "cwd")
        .args(["-ini", "-interaction=nonstopmode", "t.tex"])
        .output()
        .expect("run flashtex-xetex");
    let log = std::fs::read_to_string(dir.join("t.log")).unwrap();
    assert!(
        log.contains("Output written on t.pdf (2 pages)."),
        "{log}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = std::fs::read(dir.join("t.pdf")).unwrap();
    assert!(bytes.starts_with(b"%PDF-1.7\n"));
    let pdf = PdfFile::parse(&bytes).unwrap();
    let pages = pdf.pages().unwrap();
    assert_eq!(pages.len(), 2);
    let mb: Vec<String> = pdf
        .page_attr(pages[0], "MediaBox")
        .and_then(Obj::as_array)
        .unwrap()
        .iter()
        .map(|o| o.as_number().unwrap().to_string())
        .collect();
    assert_eq!(mb, ["0", "0", "612", "792"]);

    // the native font: Type0 over a CID-keyed CFF subset (CID = glyph id)
    let fonts = pdf.page_fonts(pages[0]);
    assert_eq!(fonts.len(), 1);
    let f = *fonts.values().next().unwrap();
    assert_eq!(f.get("Subtype").and_then(Obj::as_name), Some("Type0"));
    assert_eq!(f.get("Encoding").and_then(Obj::as_name), Some("Identity-H"));
    let base = f.get("BaseFont").and_then(Obj::as_name).unwrap();
    assert!(
        base.ends_with("+LMMono10-Regular") && base.len() == 7 + 16,
        "{base}"
    );
    let cid = pdf
        .resolve(&f.get("DescendantFonts").and_then(Obj::as_array).unwrap()[0])
        .as_dict()
        .unwrap();
    assert_eq!(
        cid.get("Subtype").and_then(Obj::as_name),
        Some("CIDFontType0")
    );
    let fd = pdf
        .resolve(cid.get("FontDescriptor").unwrap())
        .as_dict()
        .unwrap();
    let ff = pdf.resolve(fd.get("FontFile3").unwrap());
    assert_eq!(
        ff.as_dict().unwrap().get("Subtype").and_then(Obj::as_name),
        Some("CIDFontType0C")
    );
    let program = pdf.decode_stream(ff).unwrap();
    let cff = flashtex_pdf::cff::CffFont::parse(&program);
    assert!(cff.is_ok(), "the subset is a valid CFF");

    // the glyphs, at TeX's positions (72 bp from the left), red
    let content = String::from_utf8(pdf.page_content(pages[0]).unwrap()).unwrap();
    let ttf = flashtex_pdf::truetype::TrueTypeFont::parse(std::fs::read(&font).unwrap()).unwrap();
    let a = ttf.glyph_id('A').unwrap();
    assert!(content.contains("1 0 0 1 72 "), "{content}");
    assert!(content.contains(&format!("<{a:04X}>Tj")), "{content}");
    assert!(content.contains("1 0 0 rg"), "{content}");

    // the link, the destination, the outline, the information
    let annots = pdf
        .page_attr(pages[0], "Annots")
        .and_then(Obj::as_array)
        .unwrap();
    assert_eq!(annots.len(), 1);
    let link = pdf.resolve(&annots[0]).as_dict().unwrap();
    assert_eq!(link.get("Subtype").and_then(Obj::as_name), Some("Link"));
    let action = link.get("A").and_then(Obj::as_dict).unwrap();
    assert_eq!(
        action.get("URI"),
        Some(&Obj::String(b"https://example.com/".to_vec()))
    );
    let cat = pdf.catalog().unwrap();
    let names = pdf.resolve(cat.get("Names").unwrap()).as_dict().unwrap();
    let dests = pdf.resolve(names.get("Dests").unwrap()).as_dict().unwrap();
    let list = dests.get("Names").and_then(Obj::as_array).unwrap();
    assert_eq!(list[0], Obj::String(b"here".to_vec()));
    let outlines = pdf.resolve(cat.get("Outlines").unwrap()).as_dict().unwrap();
    let first = pdf
        .resolve(outlines.get("First").unwrap())
        .as_dict()
        .unwrap();
    assert_eq!(first.get("Title"), Some(&Obj::String(b"First".to_vec())));
    let info = pdf.info().unwrap();
    assert_eq!(info.get("Title"), Some(&Obj::String(b"Test".to_vec())));
    // an empty /Subject is dropped, as xdvipdfmx drops it
    assert!(info.get("Subject").is_none());
    assert_eq!(
        info.get("CreationDate"),
        Some(&Obj::String(b"D:19700101000000Z".to_vec()))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The same input gives the same PDF, byte for byte (with
/// `SOURCE_DATE_EPOCH`): two fonts, a picture and a form, whose objects
/// the writer numbers and writes in a fixed order.
#[test]
fn the_pdf_is_reproducible() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-repro-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let fonts = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts");
    for f in ["lmmono10-regular.otf", "lmmono10-italic.otf"] {
        std::fs::copy(fonts.join(f), dir.join(f)).unwrap();
    }
    let pictures =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/xetex-lockstep/pictures");
    std::fs::copy(pictures.join("p-plain.png"), dir.join("p.png")).unwrap();
    std::fs::write(
        dir.join("r.tex"),
        r#"\catcode`\{=1 \catcode`\}=2 \catcode`\#=6
\pdfpagewidth=300bp \pdfpageheight=300bp
\font\x="[lmmono10-regular.otf]" at 10pt \font\y="[lmmono10-italic.otf]" at 12pt
\shipout\hbox{\special{pdf:bxobj @f width 20bp height 20bp}\x F\special{pdf:exobj}%
\x AB \y CD \XeTeXpicfile "p.png" width 20bp \special{pdf:uxobj @f}\special{color push rgb 0 0 1}\x E\special{color pop}}
\shipout\hbox{\y G \x H}
\end
"#,
    )
    .unwrap();
    let run = || {
        let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
            .current_dir(&dir)
            .env("SOURCE_DATE_EPOCH", "1700000000")
            .env("FLASHTEX_RESOLVER", "cwd")
            .args(["-ini", "-etex", "-interaction=nonstopmode", "r.tex"])
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        std::fs::read(dir.join("r.pdf")).unwrap()
    };
    let (a, b) = (run(), run());
    assert!(a.len() > 1000);
    assert!(a == b, "two runs gave different PDFs");
    let _ = std::fs::remove_dir_all(&dir);
}
