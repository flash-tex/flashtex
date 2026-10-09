//! PDF mode's display list (docs/design/xetex/PLAN.md §3.2, `src/out`),
//! without TeX Live: INITEX ships a page with a native font, a colour, a
//! link, a destination, a rule and PDF operators, and the display list
//! `FLASHTEX_DISPLAY_LIST` receives is checked item by item. The parity of
//! whole documents with `xelatex`'s PDF is tools/xetex-pdfparity's.

use flashtex_display_list::frame::read_frame;
use flashtex_display_list::kind;
use flashtex_display_list::page::{Item, LinkKind, Page, RuleKind, StreamKind};
use flashtex_display_list::resource::Font;
use std::path::PathBuf;
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2 \catcode`\#=6
\pdfpagewidth=612bp \pdfpageheight=792bp
\font\x="[lmmono10-regular.otf]" at 10pt
\shipout\hbox{\special{pdf:dest (here) [@thispage /XYZ @xpos @ypos null]}%
\special{color push rgb 1 0 0}%
\special{pdf:bann <</Type/Annot/Subtype/Link/A<</S/URI/URI(https://example.com/)>>>>}%
\x AB\special{pdf:eann}\special{color pop}%
\vrule width 10bp height 2bp depth 0bp
\special{pdf:code q 0.5 w 0 0 m 10 10 l S Q}}
\end
"#;

fn run_dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("flashtex-xetex-outdl-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn pdf_mode_writes_the_display_list_of_a_page() {
    let dir = run_dir();
    let font =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts/lmmono10-regular.otf");
    std::fs::copy(&font, dir.join("lmmono10-regular.otf")).unwrap();
    std::fs::write(dir.join("t.tex"), INPUT).unwrap();
    let dl = dir.join("t.dl3");
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(&dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("FLASHTEX_DISPLAY_LIST", &dl)
        .args(["-ini", "-interaction=nonstopmode", "t.tex"])
        .output()
        .expect("run flashtex-xetex");
    let log = std::fs::read_to_string(dir.join("t.log")).unwrap();
    assert!(log.contains("[0]"), "no page shipped:\n{log}");

    let mut r = std::io::BufReader::new(std::fs::File::open(&dl).unwrap());
    let mut fonts = Vec::new();
    let mut pages = Vec::new();
    while let Some((k, body)) = read_frame(&mut r).unwrap() {
        match k {
            kind::FONT => fonts.push(Font::decode(&body).unwrap()),
            kind::PAGE => pages.push(Page::decode(StreamKind::Page, &body).unwrap()),
            _ => {}
        }
    }
    let _ = out;
    assert_eq!(fonts.len(), 1);
    let f = &fonts[0];
    assert_eq!(f.info.str_field("format"), Some("opentype"));
    assert_eq!(f.info.str_field("ps_name"), Some("LMMono10-Regular"));
    assert_eq!(f.program, std::fs::read(&font).unwrap());
    assert_eq!(pages.len(), 1);
    let p = &pages[0];
    assert_eq!(p.pdf_box, [0.0, 0.0, 612.0, 792.0]);

    // the glyphs of `AB`: their glyph ids, the first at TeX's origin (one
    // inch from the left: 72 bp is 4736286.72 sp), one em of LM Mono (0.525
    // em at 10 pt) apart
    let ttf = flashtex_pdf::truetype::TrueTypeFont::parse(std::fs::read(&font).unwrap()).unwrap();
    let glyphs: Vec<(u16, i32, i32)> = p
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Glyph { code, x, y, .. } => Some((*code, *x, *y)),
            _ => None,
        })
        .collect();
    assert_eq!(glyphs.len(), 2);
    assert_eq!(glyphs[0].0, ttf.glyph_id('A').unwrap());
    assert_eq!(glyphs[1].0, ttf.glyph_id('B').unwrap());
    assert_eq!(glyphs[0].1, 4_736_287);
    assert_eq!(glyphs[1].1 - glyphs[0].1, 344_064); // 0.525 em = 5.25 pt
                                                    // red, then black again for the rule
    let colors: Vec<&Vec<f64>> = p
        .items
        .iter()
        .filter_map(|i| match i {
            Item::FillColor(c) => Some(&c.0),
            _ => None,
        })
        .collect();
    assert_eq!(
        colors.first().map(|c| c.as_slice()),
        Some(&[1.0, 0.0, 0.0][..])
    );
    // the rule is 2 bp high: stroked, as dvipdfmx draws it
    let rule = p.items.iter().find_map(|i| match i {
        Item::Rule { kind, w, h, .. } => Some((*kind, *w, *h)),
        _ => None,
    });
    assert_eq!(rule.map(|r| r.0), Some(RuleKind::StrokeH));
    // the link around `AB`, and the destination at the box's start
    assert_eq!(p.links.len(), 1);
    assert_eq!(p.links[0].kind, LinkKind::Uri);
    assert_eq!(p.links[0].data, b"https://example.com/");
    assert_eq!(p.links[0].rect[0], 4_736_287);
    assert_eq!(p.links[0].rect[2] - p.links[0].rect[0], 2 * 344_064);
    assert_eq!(p.dests.len(), 1);
    assert_eq!(p.dests[0].name, b"here");
    // the path of `pdf:code`, under the page's CTM
    assert_eq!(p.paths.len(), 1);
    assert_eq!(p.paths[0].stroke.as_ref().unwrap().width, 0.5);
    let _ = std::fs::remove_dir_all(&dir);
}
