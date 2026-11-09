//! `html:` specials (spc_html.c): `<a href>` links, `<a name>`
//! destinations, `</a>`, `<base href>`. TeX Live 2026's xetex + xdvipdfmx
//! on this input (measured) writes three links, each `/C [0 0 1]` with a
//! URI action: `https://example.com/x` over CD `[82.461 706.73 92.922
//! 720]`, `http://b.example/dir/#here` over E `[92.922 706.73 98.152 720]`
//! and `http://b.example/dir/page.html` over F `[98.152 706.73 103.382
//! 720]`.

use flashtex_pdf::reader::{Obj, PdfFile};
use std::path::PathBuf;
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2 \catcode`\#=12
\pdfpagewidth=612bp \pdfpageheight=792bp
\font\x="[lmmono10-regular.otf]" at 10pt
\shipout\hbox{\x A\special{html:<a name="here">}B\special{html:</a>}%
\special{html:<A HREF="https://example.com/x">}CD\special{html:</a>}%
\special{html:<base href="http://b.example/dir/">}%
\special{html:<a href='#here'>}E\special{html:</a>}%
\special{html:<a href="page.html">}F\special{html:</a>}}
\end
"#;

#[test]
fn html_anchors() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-spchtml-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
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
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains("unknown special"), "{stdout}");
    let pdf = PdfFile::parse(&std::fs::read(dir.join("t.pdf")).unwrap()).unwrap();
    let pages = pdf.pages().unwrap();
    let annots = pdf
        .page_attr(pages[0], "Annots")
        .and_then(|a| pdf.resolve(a).as_array())
        .unwrap();
    let want: [(&[u8], [f64; 4]); 3] = [
        (b"https://example.com/x", [82.461, 706.73, 92.922, 720.0]),
        (
            b"http://b.example/dir/#here",
            [92.922, 706.73, 98.152, 720.0],
        ),
        (
            b"http://b.example/dir/page.html",
            [98.152, 706.73, 103.382, 720.0],
        ),
    ];
    assert_eq!(annots.len(), 3);
    for (a, (uri, rect)) in annots.iter().zip(want) {
        let a = pdf.resolve(a).as_dict().unwrap();
        let act = pdf.resolve(a.get("A").unwrap()).as_dict().unwrap();
        assert_eq!(act.get("URI"), Some(&Obj::String(uri.to_vec())));
        let c: Vec<&str> = a
            .get("C")
            .and_then(Obj::as_array)
            .unwrap()
            .iter()
            .map(|o| o.as_number().unwrap())
            .collect();
        assert_eq!(c, ["0", "0", "1"]);
        let r: Vec<f64> = a
            .get("Rect")
            .and_then(Obj::as_array)
            .unwrap()
            .iter()
            .map(|o| o.as_number().unwrap().parse().unwrap())
            .collect();
        for (g, w) in r.iter().zip(rect) {
            assert!((g - w).abs() <= 0.0015, "{r:?} vs {rect:?}");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}
