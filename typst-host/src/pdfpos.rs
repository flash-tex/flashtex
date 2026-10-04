//! Glyph positions from typst-pdf's own content stream, as the reference
//! PDF viewer computes them (DESIGN.md §15.5; display-list-v3 spec §4.2,
//! §11.2).
//!
//! Typst's frame positions miss the exported PDF's by up to 6·10⁻⁵ bp
//! (krilla writes f32), which moves pixels at 2× and 3×. So for every page
//! it sends, the host exports that page with typst-pdf (one export for all
//! the pages of a compile, untagged), reads the page's content stream and
//! computes each glyph's origin and glyph matrix in binary64 exactly as the
//! spec's §4.2 says the viewer does: an operand with k fraction digits is
//! its digits times the double nearest 10⁻ᵏ, a `TJ` adjustment and a width
//! the nearest double, products row by column, sums left to right.
//!
//! The derivation fails closed: an operator, font kind or filter it does
//! not know inside text is an error, never a guess, and the caller flags the
//! page INCOMPLETE.

use std::collections::HashMap;
use std::num::NonZeroUsize;

use typst_layout::PagedDocument;

use crate::pdf::{Dict, Lexer, Num, Obj, Pdf, Tok};

/// A 2-D affine matrix `[a b c d e f]` in binary64.
pub type F6 = [f64; 6];

const IDENTITY: F6 = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `m × n` (apply `m`, then `n`), each sum left to right, no fused
/// multiply-add (spec §4.2).
fn then(m: &F6, n: &F6) -> F6 {
    let [a, b, c, d, e, f] = *m;
    let [a2, b2, c2, d2, e2, f2] = *n;
    [
        a * a2 + b * c2,
        a * b2 + b * d2,
        c * a2 + d * c2,
        c * b2 + d * d2,
        e * a2 + f * c2 + e2,
        e * b2 + f * d2 + f2,
    ]
}

/// `[1 0 0 1 tx ty] × m`.
fn translate(m: &F6, tx: f64, ty: f64) -> F6 {
    let [a, b, c, d, e, f] = *m;
    [a, b, c, d, tx * a + ty * c + e, tx * b + ty * d + f]
}

/// One glyph as the viewer draws it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    /// The origin (X, Y) in stream space (bp, y up).
    pub origin: [f64; 2],
    /// The linear part of the text rendering matrix
    /// `[Tfs·Th 0 0 Tfs 0 Ts] × Tm × CTM` (the GLYPH's glyph matrix, §4.4).
    pub matrix: [f64; 4],
}

/// One page of the export.
#[derive(Clone, Debug, PartialEq)]
pub struct PagePos {
    /// The MediaBox, as the viewer reads its numbers.
    pub media_box: [f64; 4],
    /// Every glyph the content stream shows, in painting order (Form
    /// XObjects included, as drawn).
    pub glyphs: Vec<Glyph>,
}

/// Export `pages` (0-based, ascending) of `doc` with typst-pdf, untagged,
/// and derive each one's glyph positions. One export for all of them.
pub fn derive(doc: &PagedDocument, pages: &[usize]) -> Result<Vec<PagePos>, String> {
    if pages.is_empty() {
        return Ok(vec![]);
    }
    let ranges = pages
        .iter()
        .map(|&i| {
            let n = NonZeroUsize::new(i + 1);
            n..=n
        })
        .collect();
    let options = typst_pdf::PdfOptions {
        page_ranges: Some(typst::layout::PageRanges::new(ranges)),
        tagged: false,
        ..Default::default()
    };
    let bytes = typst_pdf::pdf(doc, &options).map_err(|errs| {
        let m: Vec<String> = errs.iter().map(|e| e.message.to_string()).collect();
        format!("typst-pdf export failed: {}", m.join("; "))
    })?;
    let out = derive_pdf(&bytes)?;
    if out.len() != pages.len() {
        return Err(format!(
            "the export has {} pages for {} requested",
            out.len(),
            pages.len()
        ));
    }
    Ok(out)
}

/// Every page of a typst-pdf PDF.
pub fn derive_pdf(bytes: &[u8]) -> Result<Vec<PagePos>, String> {
    let pdf = Pdf::parse(bytes)?;
    let mut out = Vec::new();
    for (n, page) in pdf.pages()? {
        out.push(derive_page(&pdf, &page).map_err(|e| format!("page object {n}: {e}"))?);
    }
    Ok(out)
}

fn nums(pdf: &Pdf, o: &Obj) -> Result<Vec<Num>, String> {
    match pdf.resolve(o)? {
        Obj::Arr(a) => a
            .iter()
            .map(|x| match pdf.resolve(x)? {
                Obj::Num(n) => Ok(n),
                o => Err(format!("number expected, got {o:?}")),
            })
            .collect(),
        o => Err(format!("array expected, got {o:?}")),
    }
}

fn derive_page(pdf: &Pdf, page: &Dict) -> Result<PagePos, String> {
    let mb = pdf
        .inherited(page, "MediaBox")?
        .ok_or("page without /MediaBox")?;
    let mb = nums(pdf, &mb)?;
    if mb.len() != 4 {
        return Err("a /MediaBox of other than 4 numbers".into());
    }
    let media_box = [
        mb[0].viewer(),
        mb[1].viewer(),
        mb[2].viewer(),
        mb[3].viewer(),
    ];
    let res = match pdf.inherited(page, "Resources")? {
        Some(r) => pdf.dict(&r)?,
        None => Dict::default(),
    };
    let mut content = Vec::new();
    match page.get("Contents") {
        None => {}
        Some(Obj::Ref(r)) => match pdf.get(*r)?.obj {
            Obj::Arr(parts) => {
                for p in parts {
                    let Obj::Ref(r) = p else {
                        return Err("/Contents entry is not a reference".into());
                    };
                    content.extend_from_slice(&pdf.stream(r)?.1);
                    content.push(b'\n');
                }
            }
            _ => content = pdf.stream(*r)?.1,
        },
        Some(Obj::Arr(parts)) => {
            for p in parts {
                let Obj::Ref(r) = p else {
                    return Err("/Contents entry is not a reference".into());
                };
                content.extend_from_slice(&pdf.stream(*r)?.1);
                content.push(b'\n');
            }
        }
        Some(o) => return Err(format!("bad /Contents {o:?}")),
    }
    let mut it = Interp {
        pdf,
        fonts: HashMap::new(),
        glyphs: Vec::new(),
        depth: 0,
    };
    let mut gs = Gs::default();
    it.run(&content, &res, &mut gs)?;
    Ok(PagePos {
        media_box,
        glyphs: it.glyphs,
    })
}

/// The text state and CTM: graphics state, saved by `q`, restored by `Q`.
#[derive(Clone)]
struct Gs {
    ctm: F6,
    tc: f64,
    tw: f64,
    tz: f64,
    tl: f64,
    ts: f64,
    fs: f64,
    font: Option<u32>,
}

impl Default for Gs {
    fn default() -> Gs {
        Gs {
            ctm: IDENTITY,
            tc: 0.0,
            tw: 0.0,
            tz: 100.0,
            tl: 0.0,
            ts: 0.0,
            fs: 0.0,
            font: None,
        }
    }
}

/// A font's widths: how a string splits into codes and each code's advance
/// at size 1 (the width W the viewer uses, in thousandths of text space).
enum Widths {
    /// Type0 with Identity-H: 2-byte CIDs, `/W` and `/DW`.
    Cid { w: HashMap<u32, f64>, dw: f64 },
    /// Type3: 1-byte codes; W is `/Widths[code − FirstChar]` times the
    /// FontMatrix's `a` times 1000, the double nearest that product.
    Type3 { first: u32, w: Vec<f64> },
}

struct Interp<'p, 'a> {
    pdf: &'p Pdf<'a>,
    /// Fonts by object number (resources map names to them).
    fonts: HashMap<u32, std::rc::Rc<Widths>>,
    glyphs: Vec<Glyph>,
    depth: u32,
}

/// The double nearest to the product of decimals `w × a × 1000`.
fn nearest_product(w: &Num, a: &Num) -> Option<f64> {
    let m = w.digits?.checked_mul(a.digits?)?;
    let exp = 3 - (w.k as i64 + a.k as i64);
    let neg = w.neg != a.neg;
    format!("{}{}e{}", if neg { "-" } else { "" }, m, exp)
        .parse()
        .ok()
}

impl Interp<'_, '_> {
    fn font(&mut self, r: u32) -> Result<std::rc::Rc<Widths>, String> {
        if let Some(f) = self.fonts.get(&r) {
            return Ok(f.clone());
        }
        let pdf = self.pdf;
        let d = pdf.dict(&Obj::Ref(r))?;
        let w = match d.get("Subtype").and_then(Obj::name) {
            Some(b"Type0") => {
                match d.get("Encoding").and_then(Obj::name) {
                    Some(b"Identity-H") => {}
                    e => return Err(format!("Type0 font with encoding {e:?}")),
                }
                let desc =
                    match pdf.resolve(d.get("DescendantFonts").ok_or("no /DescendantFonts")?)? {
                        Obj::Arr(a) if a.len() == 1 => pdf.dict(&a[0])?,
                        _ => return Err("bad /DescendantFonts".into()),
                    };
                let dw = match desc.get("DW") {
                    Some(o) => match pdf.resolve(o)? {
                        Obj::Num(n) => n.nearest,
                        _ => return Err("bad /DW".into()),
                    },
                    None => 1000.0,
                };
                let mut w = HashMap::new();
                if let Some(wo) = desc.get("W") {
                    let Obj::Arr(a) = pdf.resolve(wo)? else {
                        return Err("bad /W".into());
                    };
                    let int = |o: &Obj| -> Result<u32, String> {
                        o.num()
                            .and_then(Num::as_i64)
                            .and_then(|v| u32::try_from(v).ok())
                            .ok_or_else(|| format!("bad /W entry {o:?}"))
                    };
                    let mut i = 0;
                    while i < a.len() {
                        let first = int(&a[i])?;
                        match a.get(i + 1) {
                            Some(Obj::Arr(ws)) => {
                                for (j, x) in ws.iter().enumerate() {
                                    let v = x.num().ok_or("bad /W width")?;
                                    w.insert(first + j as u32, v.nearest);
                                }
                                i += 2;
                            }
                            Some(Obj::Num(_)) => {
                                let last = int(&a[i + 1])?;
                                let v = a
                                    .get(i + 2)
                                    .and_then(Obj::num)
                                    .ok_or("bad /W range width")?;
                                for c in first..=last {
                                    w.insert(c, v.nearest);
                                }
                                i += 3;
                            }
                            o => return Err(format!("bad /W entry {o:?}")),
                        }
                    }
                }
                Widths::Cid { w, dw }
            }
            Some(b"Type3") => {
                let fm = nums(pdf, d.get("FontMatrix").ok_or("Type3 without /FontMatrix")?)?;
                if fm.len() != 6 {
                    return Err("bad /FontMatrix".into());
                }
                let first = d
                    .get("FirstChar")
                    .and_then(Obj::num)
                    .and_then(Num::as_i64)
                    .ok_or("Type3 without /FirstChar")? as u32;
                let widths = nums(pdf, d.get("Widths").ok_or("Type3 without /Widths")?)?;
                let w = widths
                    .iter()
                    .map(|x| nearest_product(x, &fm[0]).ok_or("Type3 width out of range"))
                    .collect::<Result<Vec<f64>, _>>()?;
                Widths::Type3 { first, w }
            }
            t => return Err(format!("font of subtype {t:?}")),
        };
        let w = std::rc::Rc::new(w);
        self.fonts.insert(r, w.clone());
        Ok(w)
    }

    fn run(&mut self, content: &[u8], res: &Dict, gs: &mut Gs) -> Result<(), String> {
        self.depth += 1;
        if self.depth > 16 {
            return Err("Form XObjects nested too deep".into());
        }
        let pdf = self.pdf;
        let font_res = match res.get("Font") {
            Some(f) => pdf.dict(f)?,
            None => Dict::default(),
        };
        let mut stack: Vec<Gs> = Vec::new();
        let (mut tm, mut tlm) = (IDENTITY, IDENTITY);
        let mut in_text = false;
        let mut args: Vec<Tok> = Vec::new();
        let mut lx = Lexer::new(content);
        while let Some(t) = lx.token()? {
            let op = match t {
                Tok::Kw(op) => op,
                Tok::ArrOpen => {
                    // An array operand (TJ): collect it whole.
                    let mut v = Vec::new();
                    loop {
                        match lx.token()?.ok_or("unterminated array")? {
                            Tok::ArrClose => break,
                            t @ (Tok::Num(_) | Tok::Str(_)) => v.push(t),
                            t => return Err(format!("unexpected {t:?} in a TJ array")),
                        }
                    }
                    args.push(Tok::Kw(b"[".to_vec()));
                    args.extend(v);
                    args.push(Tok::Kw(b"]".to_vec()));
                    continue;
                }
                Tok::DictOpen => {
                    // An inline property list (BDC): skip it whole.
                    let mut depth = 1;
                    while depth > 0 {
                        match lx.token()?.ok_or("unterminated dictionary")? {
                            Tok::DictOpen => depth += 1,
                            Tok::DictClose => depth -= 1,
                            _ => {}
                        }
                    }
                    args.push(Tok::Kw(b"<<>>".to_vec()));
                    continue;
                }
                t => {
                    args.push(t);
                    continue;
                }
            };
            let n = |i: usize| -> Result<Num, String> {
                match args.get(i) {
                    Some(Tok::Num(v)) => Ok(*v),
                    a => Err(format!(
                        "{}: operand {i} is {a:?}",
                        String::from_utf8_lossy(&op)
                    )),
                }
            };
            let v = |i: usize| -> Result<f64, String> { Ok(n(i)?.viewer()) };
            match op.as_slice() {
                b"q" => stack.push(gs.clone()),
                b"Q" => *gs = stack.pop().ok_or("Q without q")?,
                b"cm" => {
                    let m = [v(0)?, v(1)?, v(2)?, v(3)?, v(4)?, v(5)?];
                    gs.ctm = then(&m, &gs.ctm);
                }
                b"BT" => {
                    in_text = true;
                    tm = IDENTITY;
                    tlm = IDENTITY;
                }
                b"ET" => in_text = false,
                b"Tf" => {
                    let Some(Tok::Name(name)) = args.first() else {
                        return Err("Tf without a font name".into());
                    };
                    let r = match font_res.0.iter().find(|(k, _)| k == name) {
                        Some((_, Obj::Ref(r))) => *r,
                        _ => {
                            return Err(format!(
                                "font /{} is not a resource",
                                String::from_utf8_lossy(name)
                            ))
                        }
                    };
                    self.font(r)?;
                    gs.font = Some(r);
                    gs.fs = v(1)?;
                }
                b"Tc" => gs.tc = v(0)?,
                b"Tw" => gs.tw = v(0)?,
                b"Tz" => gs.tz = v(0)?,
                b"TL" => gs.tl = v(0)?,
                b"Ts" => gs.ts = v(0)?,
                b"Tm" => {
                    tm = [v(0)?, v(1)?, v(2)?, v(3)?, v(4)?, v(5)?];
                    tlm = tm;
                }
                b"Td" | b"TD" => {
                    let (tx, ty) = (v(0)?, v(1)?);
                    if op == b"TD" {
                        gs.tl = -ty;
                    }
                    tlm = translate(&tlm, tx, ty);
                    tm = tlm;
                }
                b"T*" => {
                    tlm = translate(&tlm, 0.0, -gs.tl);
                    tm = tlm;
                }
                b"Tj" | b"TJ" | b"'" | b"\"" => {
                    if !in_text {
                        return Err("text shown outside BT/ET".into());
                    }
                    // The font in effect: `Tf` is graphics state, so after
                    // a `Q` it may be an earlier one.
                    let w = match gs.font {
                        Some(r) => self.font(r)?,
                        None => return Err("text without a font".into()),
                    };
                    if op == b"'" || op == b"\"" {
                        if op == b"\"" {
                            gs.tw = v(0)?;
                            gs.tc = v(1)?;
                        }
                        tlm = translate(&tlm, 0.0, -gs.tl);
                        tm = tlm;
                    }
                    let show: Vec<Tok> = match op.as_slice() {
                        b"TJ" => match (args.first(), args.last()) {
                            (Some(Tok::Kw(o)), Some(Tok::Kw(c))) if o == b"[" && c == b"]" => {
                                args[1..args.len() - 1].to_vec()
                            }
                            _ => return Err("TJ without an array".into()),
                        },
                        _ => match args.last() {
                            Some(t @ Tok::Str(_)) => vec![t.clone()],
                            _ => return Err("a text operator without a string".into()),
                        },
                    };
                    for e in show {
                        match e {
                            Tok::Str(s) => self.show(&s, &w, gs, &mut tm)?,
                            Tok::Num(adj) => {
                                let tx = -adj.nearest / 1000.0 * gs.fs;
                                let tx = tx * gs.tz / 100.0;
                                tm = translate(&tm, tx, 0.0);
                            }
                            _ => unreachable!(),
                        }
                    }
                }
                // An XObject is a frame image (raster, SVG or PDF): typst-pdf
                // draws each with one `Do`, and the glyphs an SVG or PDF image
                // shows are the image's, not the page's text runs. Page text is
                // never put in a Form XObject (checked over Typst's test suite
                // by examples/positions_suite.rs: a run drawn in one would
                // leave the walk short, and the page INCOMPLETE).
                b"Do" => {}
                b"BI" => return Err("inline image".into()),
                _ => {}
            }
            args.clear();
        }
        self.depth -= 1;
        Ok(())
    }

    fn show(&mut self, s: &[u8], w: &Widths, gs: &Gs, tm: &mut F6) -> Result<(), String> {
        let codes: Vec<(u32, bool)> = match w {
            Widths::Cid { .. } => {
                if !s.len().is_multiple_of(2) {
                    return Err("a CID string of odd length".into());
                }
                s.chunks(2)
                    .map(|c| (u16::from_be_bytes([c[0], c[1]]) as u32, false))
                    .collect()
            }
            Widths::Type3 { .. } => s.iter().map(|&c| (c as u32, c == 32)).collect(),
        };
        for (code, single_space) in codes {
            let trm = then(tm, &gs.ctm);
            let th = gs.tz / 100.0;
            let origin = [gs.ts * trm[2] + trm[4], gs.ts * trm[3] + trm[5]];
            let matrix = [
                gs.fs * th * trm[0],
                gs.fs * th * trm[1],
                gs.fs * trm[2],
                gs.fs * trm[3],
            ];
            self.glyphs.push(Glyph { origin, matrix });
            let wv = match w {
                Widths::Cid { w, dw } => *w.get(&code).unwrap_or(dw),
                Widths::Type3 { first, w } => *code
                    .checked_sub(*first)
                    .and_then(|i| w.get(i as usize))
                    .ok_or_else(|| format!("Type3 code {code} has no width"))?,
            };
            let mut tx = wv / 1000.0 * gs.fs + gs.tc;
            if single_space {
                tx += gs.tw;
            }
            let tx = tx * gs.tz / 100.0;
            *tm = translate(tm, tx, 0.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PDF with the given objects (1-based) and a cross-reference table.
    fn pdf(objs: &[&str]) -> Vec<u8> {
        let mut out = b"%PDF-1.7\n".to_vec();
        let mut offs = vec![];
        for (i, o) in objs.iter().enumerate() {
            offs.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
        }
        let x = out.len();
        out.extend_from_slice(
            format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes(),
        );
        for o in offs {
            out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!(
                "trailer\n<</Size {}/Root 1 0 R>>\nstartxref\n{x}\n%%EOF\n",
                objs.len() + 1
            )
            .as_bytes(),
        );
        out
    }

    fn doc(content: &str, fonts: &str, extra: &[&str]) -> Vec<u8> {
        let mut objs = vec![
            "<</Type/Catalog/Pages 2 0 R>>".to_string(),
            "<</Type/Pages/Kids[3 0 R]/Count 1>>".to_string(),
            format!(
                "<</Type/Page/Parent 2 0 R/MediaBox[0 0 595.2756 841.8898]/Contents 4 0 R\
                 /Resources<</Font<<{fonts}>>>>>>"
            ),
            format!(
                "<</Length {}>>\nstream\n{content}\nendstream",
                content.len()
            ),
        ];
        objs.extend(extra.iter().map(|s| s.to_string()));
        let refs: Vec<&str> = objs.iter().map(|s| s.as_str()).collect();
        pdf(&refs)
    }

    /// The text operators typst-pdf does not write today (TD, T*, `"`, Tz,
    /// Ts, Tc, Tw, Type 3 widths) follow spec §4.2 too.
    #[test]
    fn text_operators_follow_the_viewer() {
        let content = "q 1 0 0 1 10.5 20.25 cm BT /F1 12 Tf 2 Tz 0.5 Tc 3 Tw 1.5 Ts \
                       0 14 TD (A ) Tj T* [(B) -250 (C)] TJ ET Q \
                       BT /F0 10 Tf 1 0 0 1 100 200 Tm 7 TL 1 2 <0044> \" ET";
        let mut widths = vec!["0"; 36];
        widths[0] = "250";
        widths[33] = "600";
        widths[34] = "500";
        widths[35] = "400";
        let t3 = format!(
            "<</Type/Font/Subtype/Type3/FirstChar 32/Widths[{}]/FontMatrix[0.001 0 0 0.001 0 0]>>",
            widths.join(" ")
        );
        let bytes = doc(
            content,
            "/F0 5 0 R/F1 6 0 R",
            &[
                "<</Type/Font/Subtype/Type0/Encoding/Identity-H/DescendantFonts[7 0 R]>>",
                &t3,
                "<</Type/Font/Subtype/CIDFontType0/DW 1000/W[68[555.5]]>>",
            ],
        );
        let pages = derive_pdf(&bytes).unwrap();
        let p = &pages[0];
        assert_eq!(p.media_box, [0.0, 0.0, 5952756.0 * 1e-4, 8418898.0 * 1e-4]);
        let g = &p.glyphs;
        assert_eq!(g.len(), 5);
        let ctm = [1.0, 0.0, 0.0, 1.0, 105.0 * 0.1, 2025.0 * 0.01];
        let o = |tm: &F6| {
            let t = then(tm, &ctm);
            [1.5 * t[2] + t[4], 1.5 * t[3] + t[5]]
        };
        // TD: TL = -14, the line moves up 14; Ts lifts the origin.
        let tm0 = translate(&IDENTITY, 0.0, 14.0);
        assert_eq!(g[0].origin, o(&tm0));
        assert_eq!(g[0].matrix, [12.0 * 2.0 / 100.0, 0.0, 0.0, 12.0]);
        // 'A' (code 65, width 600 × 0.001 × 1000), Tc, then ' ' (code 32: Tw too).
        let adv = |w: f64, tw: f64| (w / 1000.0 * 12.0 + 0.5 + tw) * 2.0 / 100.0;
        let tm1 = translate(&tm0, adv(600.0, 0.0), 0.0);
        assert_eq!(g[1].origin, o(&tm1));
        // T* moves down TL from the line start: B, an adjustment, C.
        let tm2 = translate(&tm0, 0.0, 14.0);
        assert_eq!(g[2].origin, o(&tm2));
        let tm3 = translate(&tm2, adv(500.0, 0.0), 0.0);
        let tm3 = translate(&tm3, 250.0 / 1000.0 * 12.0 * 2.0 / 100.0, 0.0);
        assert_eq!(g[3].origin, o(&tm3));
        // `"` sets Tw and Tc and moves to the next line (TL 7): CID 68.
        let tm = translate(&[1.0, 0.0, 0.0, 1.0, 100.0, 200.0], 0.0, -7.0);
        assert_eq!(g[4].origin, [tm[4], tm[5]]);
        assert_eq!(g[4].matrix, [10.0, 0.0, 0.0, 10.0]);
    }

    #[test]
    fn unknown_fonts_and_inline_images_fail_closed() {
        let t1 = ["<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>"];
        assert!(derive_pdf(&doc("BT /F0 1 Tf (a) Tj ET", "/F0 5 0 R", &t1)).is_err());
        assert!(derive_pdf(&doc("BI /W 1 /H 1 ID x EI", "/F0 5 0 R", &t1)).is_err());
        assert!(derive_pdf(&doc("(a) Tj", "/F0 5 0 R", &t1)).is_err());
        let ok = derive_pdf(&doc("q Q", "/F0 5 0 R", &t1)).unwrap();
        assert!(ok[0].glyphs.is_empty());
    }
}
