//! Virtual fonts in PDF mode (`src/out/vf.rs`, issue #2438), without TeX
//! Live: INITEX ships a page in the fixture's virtual fonts (tests/vf),
//! and what the display list draws is compared with what TeX Live 2026's
//! xdvipdfmx drew for the same files (tests/vf/expected.json, written from
//! its PDF by tests/vf/make_fixture.py --oracle): every glyph (font, code,
//! size and slant, origin, colour), every rule and the link's rectangle.
//!
//! The fixture's packets use every command dvipdfm-x's vf.c reads: set and
//! put in every length, rules (stroked, filled, and ones that draw
//! nothing), moves in every register, font changes (one by a number vf.c
//! cannot select), specials, a character with no packet, and a virtual
//! font inside a virtual font at another size.
//!
//! Positions are held to the tolerance tools/xetex-pdfparity holds a PDF
//! to against xdvipdfmx's: 0.01 bp + 0.005 em (xdvipdfmx's own rounding of
//! a text line's kerns, measured there).

use flashtex_display_list::frame::read_frame;
use flashtex_display_list::json::Json;
use flashtex_display_list::kind;
use flashtex_display_list::page::{flags, Item, LinkKind, Page, StreamKind};
use flashtex_display_list::resource::Font;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

const FILES: &[&str] = &[
    "fxvfbase.pfb",
    "fxvfbase.tfm",
    "fxvfslant.tfm",
    "fxvf.vf",
    "fxvf.tfm",
    "fxvfn.vf",
    "fxvfn.tfm",
    "pdftex.map",
    "vf.tex",
];

/// sp per bp.
const SP: f64 = 65781.76;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/vf")
}

fn run_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("flashtex-xetex-outvf-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Run the fixture's `vf.tex` and read the display list: the fonts and
/// the page.
fn run(tag: &str) -> (Vec<Font>, Page, PathBuf) {
    let dir = run_dir(tag);
    for f in FILES {
        std::fs::copy(fixture().join(f), dir.join(f)).unwrap();
    }
    let dl = dir.join("vf.dl3");
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(&dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("FLASHTEX_DISPLAY_LIST", &dl)
        .args(["-ini", "-interaction=nonstopmode", "vf.tex"])
        .output()
        .expect("run flashtex-xetex");
    let log = std::fs::read_to_string(dir.join("vf.log")).unwrap_or_default();
    assert!(
        log.contains("Output written on vf.pdf (1 page"),
        "{log}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
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
    assert_eq!(pages.len(), 1);
    (fonts, pages.remove(0), dir)
}

fn num(j: &Json) -> f64 {
    match j {
        Json::Int(i) => *i as f64,
        Json::Num(f) => *f,
        _ => panic!("not a number: {j:?}"),
    }
}

fn nums(j: &Json) -> Vec<f64> {
    j.as_array().unwrap().iter().map(num).collect()
}

#[derive(Debug)]
struct Glyph {
    font: String,
    code: u16,
    /// The text matrix's linear part (bp per em).
    m: [f64; 4],
    x: f64,
    y: f64,
    fill: Vec<f64>,
}

/// The page's glyphs as the PDF draws them: bp, y up.
fn glyphs_of(fonts: &[Font], p: &Page) -> Vec<Glyph> {
    let by_id: HashMap<u16, &Font> = fonts.iter().map(|f| (f.id, f)).collect();
    let height = p.pdf_box[3];
    let mut m = [1.0, 0.0, 0.0, 1.0];
    let mut fill = vec![0.0];
    let mut out = Vec::new();
    for it in &p.items {
        match it {
            Item::Matrix(n) => {
                let t = p.matrices[*n as usize - 1];
                m = [t[0], t[1], t[2], t[3]];
            }
            Item::FillColor(c) => fill = c.0.clone(),
            Item::Glyph {
                font, code, x, y, ..
            } => {
                let f = by_id[font];
                // a map entry's SlantFont is in the resource, not the
                // glyph matrix (spec §5.1): the PDF's text matrix has it
                let slant = f.info.int_field("slant").unwrap_or(0) as f64 / 1000.0;
                out.push(Glyph {
                    font: f.info.str_field("ps_name").unwrap_or("?").to_string(),
                    code: *code,
                    m: [m[0], m[1], m[2] + slant * m[0], m[3]],
                    x: *x as f64 / SP,
                    y: height - *y as f64 / SP,
                    fill: fill.clone(),
                });
            }
            _ => {}
        }
    }
    out
}

#[test]
fn virtual_fonts_are_drawn_as_xdvipdfmx_draws_them() {
    let (fonts, p, _dir) = run("parity");
    let expected = std::fs::read_to_string(fixture().join("expected.json")).unwrap();
    let expected = Json::parse(&expected).unwrap();
    assert_eq!(nums(expected.get("media_box").unwrap()), p.pdf_box);

    // nothing left undrawn
    assert_eq!(p.unsupported, Vec::<String>::new());
    assert_eq!(p.flags & flags::INCOMPLETE, 0);
    // the virtual fonts have no resource: the two map entries have
    for f in &fonts {
        assert_eq!(f.info.str_field("format"), Some("type1"), "{:?}", f.info);
        assert_eq!(f.info.str_field("ps_name"), Some("FxVfBase"));
        assert_eq!(
            f.program,
            std::fs::read(fixture().join("fxvfbase.pfb")).unwrap()
        );
    }
    let mut slants: Vec<i64> = fonts
        .iter()
        .map(|f| f.info.int_field("slant").unwrap_or(0))
        .collect();
    slants.sort();
    assert_eq!(slants, [0, 250]);

    // the glyphs, in the order drawn
    let got = glyphs_of(&fonts, &p);
    let want = expected.get("glyphs").and_then(Json::as_array).unwrap();
    assert_eq!(got.len(), want.len(), "glyph count");
    let mut worst = 0.0f64;
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        let what = format!("glyph {i}: {g:?} against {w:?}");
        assert_eq!(Some(g.font.as_str()), w.str_field("font"), "{what}");
        assert_eq!(Some(g.code as i64), w.int_field("code"), "{what}");
        // expected.json has the matrix per 1000 units (a glyph-space unit
        // of the Type 1 font), to four decimals
        let wm = nums(w.get("m").unwrap());
        for k in 0..4 {
            assert!((g.m[k] * 1000.0 - wm[k]).abs() <= 0.11, "{what}");
        }
        let em = g.m[0].abs();
        let tol = 0.01 + 0.005 * em;
        let (dx, dy) = (
            g.x - num(w.get("x").unwrap()),
            g.y - num(w.get("y").unwrap()),
        );
        assert!(
            dx.abs() <= tol && dy.abs() <= tol,
            "{what}: off by {dx}, {dy}"
        );
        worst = worst.max(dx.abs() / em).max(dy.abs() / em);
        let fill = nums(&w.get("fill").unwrap().as_array().unwrap()[1]);
        assert_eq!(g.fill, fill, "{what}");
    }
    eprintln!("largest glyph deviation from xdvipdfmx: {worst:.6} em");

    // the rules, in the order drawn: left, bottom, right, top
    let height = p.pdf_box[3];
    let rules: Vec<[f64; 4]> = p
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Rule { x, y, w, h, .. } => Some([
                *x as f64 / SP,
                height - (*y as f64 + *h as f64) / SP,
                (*x as f64 + *w as f64) / SP,
                height - *y as f64 / SP,
            ]),
            _ => None,
        })
        .collect();
    let want = expected.get("rules").and_then(Json::as_array).unwrap();
    assert_eq!(rules.len(), want.len(), "rule count: {rules:?}");
    assert!(p.paths.is_empty());
    for (i, (r, w)) in rules.iter().zip(want).enumerate() {
        let wr = nums(w.get("rect").unwrap());
        for k in 0..4 {
            assert!(
                (r[k] - wr[k]).abs() <= 0.01,
                "rule {i}: {r:?} against {wr:?}"
            );
        }
    }

    // the link around characters of virtual fonts: the box of what their
    // packets draw
    let want = expected.get("links").and_then(Json::as_array).unwrap();
    assert_eq!(p.links.len(), want.len());
    assert_eq!(p.links.len(), 1);
    assert_eq!(p.links[0].kind, LinkKind::Uri);
    let l = p.links[0].rect.map(|v| v as f64 / SP);
    let got = [l[0], height - l[3], l[2], height - l[1]];
    let wr = nums(want[0].get("rect").unwrap());
    for k in 0..4 {
        assert!(
            (got[k] - wr[k]).abs() <= 0.01,
            "link: {got:?} against {wr:?}"
        );
    }
}

/// The PDF itself: the page's fonts are the two Type 1 fonts of the map
/// entries (the upright and the slanted one), with the fixture's program.
#[test]
fn the_pdf_has_the_fonts_the_virtual_fonts_name() {
    use flashtex_pdf::reader::{Obj, PdfFile};
    let (_fonts, _p, dir) = run("pdf");
    let bytes = std::fs::read(dir.join("vf.pdf")).unwrap();
    let pdf = PdfFile::parse(&bytes).unwrap();
    let pages = pdf.pages().unwrap();
    assert_eq!(pages.len(), 1);
    let fonts = pdf.page_fonts(pages[0]);
    assert_eq!(fonts.len(), 2, "the upright and the slanted font");
    for f in fonts.values() {
        let base = f.get("BaseFont").and_then(Obj::as_name).unwrap();
        assert!(base.ends_with("FxVfBase"), "{base}");
    }
}
