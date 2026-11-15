//! `dvipdfmx:catch_phantom` and the advances (issue #2485): xdvipdfmx's
//! `dvi_set`, `do_setrule` and `do_glyphs` all end in `dvi_right(width)`,
//! which widens the link being tracked by the phantom box, so a glyph run
//! and a set rule widen a link as a `\kern` does.
//!
//! Measured with TeX Live 2026's `xetex -ini` + xdvipdfmx 20260317 on this
//! input (`/Rect` of the four links): a glyph run with a 20 pt high, 5 pt
//! deep phantom box `72 704.897 82.461 729.803`; a set rule alone
//! `92.423 701.748 102.386 726.655` (a link around a rule without the
//! phantom has no annotation); with `catch_phantom true` (the font's size
//! high) a rule then a glyph `112.349 706.73 127.542 720`; with it off a
//! glyph `137.504 706.73 142.735 720`.

use flashtex_pdf::reader::{Obj, PdfFile};
use std::path::PathBuf;
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2 \catcode`\#=6
\pdfpagewidth=612bp \pdfpageheight=792bp
\font\x="[lmmono10-regular.otf]" at 10pt
\shipout\hbox{\special{dvipdfmx:catch_phantom true height 20pt depth 5pt}%
\special{pdf:bann <</Type/Annot/Subtype/Link/Border[0 0 0]/A<</S/URI/URI(a)>>>>}\x ab\special{pdf:eann}%
\kern10pt\special{pdf:bann <</Type/Annot/Subtype/Link/Border[0 0 0]/A<</S/URI/URI(b)>>>>}\vrule width 10pt height 3pt\special{pdf:eann}%
\kern10pt\special{dvipdfmx:catch_phantom true}\special{pdf:bann <</Type/Annot/Subtype/Link/Border[0 0 0]/A<</S/URI/URI(c)>>>>}\vrule width 10pt height 3pt\x a\special{pdf:eann}%
\kern10pt\special{dvipdfmx:catch_phantom false}\special{pdf:bann <</Type/Annot/Subtype/Link/Border[0 0 0]/A<</S/URI/URI(d)>>>>}\x a\special{pdf:eann}}
\end
"#;

#[test]
fn advances_widen_links_by_the_phantom_box() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-phantom-{}", std::process::id()));
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
    let bytes = std::fs::read(dir.join("t.pdf")).unwrap_or_else(|_| {
        panic!(
            "no PDF\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    let pdf = PdfFile::parse(&bytes).unwrap();
    let page = pdf.pages().unwrap()[0];
    let annots = pdf
        .page_attr(page, "Annots")
        .and_then(Obj::as_array)
        .unwrap();
    let got: Vec<(String, Vec<String>)> = annots
        .iter()
        .map(|a| {
            let d = pdf.resolve(a).as_dict().unwrap();
            let uri = match pdf
                .get(d, "A")
                .and_then(Obj::as_dict)
                .and_then(|a| a.get("URI"))
            {
                Some(Obj::String(s)) => String::from_utf8_lossy(s).into_owned(),
                _ => String::new(),
            };
            let rect = d
                .get("Rect")
                .and_then(Obj::as_array)
                .unwrap()
                .iter()
                .map(|v| v.as_number().unwrap().to_string())
                .collect();
            (uri, rect)
        })
        .collect();
    let want = [
        ("a", ["72", "704.897", "82.461", "729.803"]),
        ("b", ["92.423", "701.748", "102.386", "726.655"]),
        ("c", ["112.349", "706.73", "127.542", "720"]),
        ("d", ["137.504", "706.73", "142.735", "720"]),
    ];
    let want: Vec<(String, Vec<String>)> = want
        .iter()
        .map(|(u, r)| (u.to_string(), r.iter().map(|s| s.to_string()).collect()))
        .collect();
    assert_eq!(got, want);
    let _ = std::fs::remove_dir_all(&dir);
}
