//! `dvipdfmx:config g` (hyperref's `pdflinkmargin`, l3backend's link
//! margin): dvipdfmx reads the first page's config specials before it sets
//! the document up, and grows every `pdf:bann` link's rectangle by the
//! margin (pdfdoc.c's pdf_doc_break_annot); a later page's `g` is read too
//! late. TeX Live 2026's xetex + xdvipdfmx on this input (measured):
//! `/Rect [74.74 704.239 90.182 722.491]` and `[69.509 704.239 84.951
//! 722.491]`.

use flashtex_pdf::reader::{Obj, PdfFile};
use std::path::PathBuf;
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2
\pdfpagewidth=612bp \pdfpageheight=792bp
\font\x="[lmmono10-regular.otf]" at 10pt
\shipout\hbox{\x A\special{pdf:bann <</Type/Annot/Subtype/Link/Border[0 0 1]/A<</S/URI/URI(https://a.example/)>>>>}BC\special{pdf:eann}%
\special{dvipdfmx:config g 2.5pt}}
\shipout\hbox{\x \special{dvipdfmx:config g 10}\special{pdf:bann <</Type/Annot/Subtype/Link/Border[0 0 1]/A<</S/URI/URI(https://b.example/)>>>>}DE\special{pdf:eann}}
\end
"#;

#[test]
fn first_page_config_grows_links() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-spcgrow-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let font =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts/lmmono10-regular.otf");
    std::fs::copy(&font, dir.join("lmmono10-regular.otf")).unwrap();
    std::fs::write(dir.join("t.tex"), INPUT).unwrap();
    Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(&dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_RESOLVER", "cwd")
        .args(["-ini", "-interaction=nonstopmode", "t.tex"])
        .output()
        .expect("run flashtex-xetex");
    let pdf = PdfFile::parse(&std::fs::read(dir.join("t.pdf")).unwrap()).unwrap();
    let pages = pdf.pages().unwrap();
    let want = [
        [74.74, 704.239, 90.182, 722.491],
        [69.509, 704.239, 84.951, 722.491],
    ];
    for (pg, w) in pages.iter().zip(want) {
        let annots = pdf
            .page_attr(*pg, "Annots")
            .and_then(Obj::as_array)
            .unwrap();
        let a = pdf.resolve(&annots[0]).as_dict().unwrap();
        let r: Vec<f64> = a
            .get("Rect")
            .and_then(Obj::as_array)
            .unwrap()
            .iter()
            .map(|o| o.as_number().unwrap().parse().unwrap())
            .collect();
        for (g, w) in r.iter().zip(w) {
            assert!((g - w).abs() <= 0.0015, "{r:?} vs {w:?}");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}
