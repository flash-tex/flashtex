//! `embolden=` in PDF mode: xdvipdfmx strokes the glyphs (`2 Tr`) with a
//! line as wide as the XDV font record's `embolden`, which XeTeX has
//! already scaled to the font (the option's value times the size in TeX
//! points over 100). The writer multiplied it by the size over 10 pt once
//! more, so only 10 pt fonts were right (found by tools/fuzz-unicode
//! `--pdf`).
//!
//! The widths are TeX Live 2026's (`xetex` through xdvipdfmx, MacTeX 2026,
//! 2026-11-09) for the same input: `0.199997 w` at 20 pt, `0.100006 w` at
//! 10 pt and, for `embolden=3` at 4 pt, `0.119995 w`.

use flashtex_pdf::reader::PdfFile;
use std::path::PathBuf;
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2
\font\fa="[lmmono10-regular.otf]:embolden=1" at 20pt \fa
\font\fb="[lmmono10-regular.otf]:embolden=1" at 10pt
\font\fc="[lmmono10-regular.otf]:embolden=3" at 4pt
\shipout\hbox{AB\fb AB\fc AB}
\end
"#;

#[test]
fn embolden_strokes_with_the_xdv_width() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-embolden-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let font =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts/lmmono10-regular.otf");
    std::fs::copy(&font, dir.join("lmmono10-regular.otf")).unwrap();
    std::fs::write(dir.join("e.tex"), INPUT).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(&dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_RESOLVER", "cwd")
        .args(["-ini", "-interaction=nonstopmode", "e.tex"])
        .output()
        .expect("run flashtex-xetex");
    let bytes = std::fs::read(dir.join("e.pdf")).unwrap_or_else(|_| {
        panic!("no PDF: {}", String::from_utf8_lossy(&out.stderr));
    });
    let pdf = PdfFile::parse(&bytes).unwrap();
    let pages = pdf.pages().unwrap();
    let content = String::from_utf8(pdf.page_content(pages[0]).unwrap()).unwrap();
    let widths: Vec<&str> = content
        .lines()
        .filter_map(|l| l.split_once(" w ").map(|(w, _)| w))
        .collect();
    assert_eq!(widths, ["0.199997", "0.100006", "0.119995"], "{content}");
    assert!(content.contains("2 Tr"), "{content}");
    let _ = std::fs::remove_dir_all(&dir);
}
