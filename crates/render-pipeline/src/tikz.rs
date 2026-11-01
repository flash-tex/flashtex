//! TikZ pictures: node text measured and shaped with the pipeline's Latin
//! Modern faces (TFM metrics, as pdfTeX lays them out), and a standalone
//! PDF writer used by the oracle harness (`tools/tikz-oracle`).
//!
//! The picture geometry comes from `flashtex-vector-graphics`' TikZ reader;
//! vector items go through its PDF content-stream serializer, text through
//! the glyph emission below (whole OpenType programs embedded as CID-keyed
//! `FontFile3 /OpenType` fonts, one `Tm` per glyph at the shaped position).
//! No TeX engine is involved.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::rc::Rc;

use flashtex_vector_graphics as vg;
use vg::geom::{Size, Transform};
use vg::tikz::{find_pictures, Picture, TextMeasurer, TextMetrics, TextStyle, Tikz};
use vg::{Color, DisplayList, Group, Item, ItemId};

use crate::fonts::{Family, FontSet, LoadedFace, Role};
use crate::tfm::Tfm;

const BP_PER_PT: f64 = 72.0 / 72.27;

/// One positioned glyph of node text.
#[derive(Clone, Debug)]
pub struct ShapedGlyph {
    pub gid: u16,
    /// Origin, TeX points from the start of the baseline.
    pub x_pt: f64,
    /// Advance in TeX points (kerning included).
    pub advance_pt: f64,
    /// Bytes of the text this glyph shows.
    pub text_range: std::ops::Range<usize>,
}

/// One shaped line of node text.
#[derive(Clone)]
pub struct ShapedText {
    pub face: Rc<LoadedFace>,
    pub glyphs: Vec<ShapedGlyph>,
    pub metrics: TextMetrics,
}

/// Shapes `text` as an `\hbox`: words through the TFM ligature/kern
/// program, interword glue at `\fontdimen2` (natural width).
pub fn shape_text(fonts: &FontSet, text: &str, style: &TextStyle) -> ShapedText {
    let size = style.size_pt;
    let face = fonts
        .resolve(
            Family::LatinModern,
            Role::Text {
                bold: style.bold,
                italic: style.italic,
            },
            size,
        )
        .face;
    let space = face.tfm.as_ref().and_then(|t| t.param(2)).map(|f| Tfm::pt(f, size)).unwrap_or(size / 3.0);
    let mut glyphs = Vec::new();
    let mut x = 0.0;
    let mut height: f64 = 0.0;
    let mut depth: f64 = 0.0;
    let mut word_start = 0usize;
    for (i, word) in text.split(' ').enumerate() {
        if i > 0 {
            x += space;
        }
        let this_start = word_start;
        word_start += word.len() + 1;
        if word.is_empty() {
            continue;
        }
        let shaped = fonts.shaper().shape(&face, word);
        let upem = shaped.units_per_em as f64;
        for cluster in &shaped.clusters {
            for g in &cluster.glyphs {
                let advance_pt = f64::from(g.advance) * size / upem;
                if !g.empty && g.gid.0 != 0 {
                    let dx = face.pt(i64::from(g.x_offset), size);
                    glyphs.push(ShapedGlyph {
                        gid: g.gid.0,
                        x_pt: x + dx,
                        advance_pt,
                        text_range: this_start + cluster.text_range.start..this_start + cluster.text_range.end,
                    });
                }
                x += advance_pt;
            }
        }
        height = height.max(shaped.height_pt(size));
        depth = depth.max(shaped.depth_pt(size));
    }
    ShapedText {
        face,
        glyphs,
        metrics: TextMetrics {
            width_pt: x,
            height_pt: height,
            depth_pt: depth,
        },
    }
}

/// [`TextMeasurer`] backed by the pipeline's fonts.
pub struct FontMeasurer<'a> {
    pub fonts: &'a FontSet,
}

impl TextMeasurer for FontMeasurer<'_> {
    fn measure(&self, text: &str, style: &TextStyle) -> TextMetrics {
        shape_text(self.fonts, text, style).metrics
    }
}

pub struct StandalonePdf {
    pub bytes: Vec<u8>,
    pub picture: Picture,
    pub width_bp: f64,
    pub height_bp: f64,
}

fn num(v: f64) -> String {
    vg::pdf::num(v)
}

fn color_op(c: Color) -> String {
    match c.clamped() {
        Color::Gray(g) => format!("{} g", num(g)),
        Color::Rgb(r, g, b) => format!("{} {} {} rg", num(r), num(g), num(b)),
        Color::Cmyk(c, m, y, k) => format!("{} {} {} {} k", num(c), num(m), num(y), num(k)),
    }
}

/// Compiles the first `tikzpicture` of a `standalone`-style document into a
/// one-page PDF sized to the picture plus `border_pt` on every side.
pub fn standalone_pdf(fonts: &FontSet, doc: &str, border_pt: f64, font_size_pt: f64) -> Result<StandalonePdf, String> {
    let preamble_end = doc.find("\\begin{document}").unwrap_or(0);
    let mut tikz = Tikz::new(font_size_pt);
    let mut pre_diags = tikz.read_preamble(&doc[..preamble_end]);
    let pics = find_pictures(doc);
    let pic_src = pics.first().ok_or("no tikzpicture in the document")?;
    let measurer = FontMeasurer { fonts };
    let mut picture = tikz.render(doc, pic_src, &measurer);
    pre_diags.append(&mut picture.diagnostics);
    picture.diagnostics = pre_diags;

    let b = border_pt * BP_PER_PT;
    let w = picture.width_bp + 2.0 * b;
    let h = picture.height_bp + 2.0 * b;
    let place = Transform::translate(b, b);
    let flip = Transform::new(1.0, 0.0, 0.0, -1.0, 0.0, h);

    let mut content = String::new();
    let mut ext_states: Vec<(String, String)> = Vec::new();
    let mut font_objs: BTreeMap<String, (String, Rc<LoadedFace>)> = BTreeMap::new();
    let mut items_done = 0usize;
    let mut chunk = 0usize;
    let emit_items = |upto: usize, content: &mut String, ext_states: &mut Vec<(String, String)>, chunk: &mut usize, done: &mut usize| -> Result<(), String> {
        if upto <= *done {
            return Ok(());
        }
        let mut g = Group::new(ItemId(u64::MAX));
        g.transform = place;
        g.items = picture.items[*done..upto].to_vec();
        let list = DisplayList {
            page_size: Size::new(w, h),
            items: vec![Item::Group(g)],
        };
        let frag = vg::pdf::content_stream(&list).map_err(|e| format!("{e:?}"))?;
        let prefix = format!("C{}", *chunk);
        let mut text = frag.content;
        for e in frag.ext_g_states.iter().rev() {
            text = text.replace(&format!("/{} gs", e.name), &format!("/{prefix}{} gs", e.name));
            ext_states.push((format!("{prefix}{}", e.name), e.dictionary()));
        }
        content.push_str("q\n");
        content.push_str(&text);
        content.push_str("Q\n");
        *chunk += 1;
        *done = upto;
        Ok(())
    };
    let texts = picture.texts.clone();
    for (ti, t) in texts.iter().enumerate() {
        emit_items(t.after_item.min(picture.items.len()), &mut content, &mut ext_states, &mut chunk, &mut items_done)?;
        let shaped = shape_text(fonts, &t.text, &t.style);
        let Some(_program) = shaped.face.program() else {
            return Err(format!("face {} has no program to embed", shaped.face.name));
        };
        let key = shaped.face.font_id.to_string();
        let next = format!("F{}", font_objs.len());
        let fname = font_objs.entry(key).or_insert((next, shaped.face.clone())).0.clone();
        let size_bp = t.style.size_pt * BP_PER_PT;
        let _ = write!(content, "q\n{}\n", color_op(t.paint.color));
        if t.paint.alpha < 1.0 {
            let name = format!("TA{ti}");
            ext_states.push((name.clone(), format!("<< /Type /ExtGState /ca {} /CA {} >>", num(t.paint.alpha), num(t.paint.alpha))));
            let _ = writeln!(content, "/{name} gs");
        }
        let _ = writeln!(content, "BT\n/{fname} {} Tf", num(size_bp));
        for g in &shaped.glyphs {
            let (gid, x_pt) = (g.gid, g.x_pt);
            let m = Transform::new(1.0, 0.0, 0.0, -1.0, x_pt * BP_PER_PT, 0.0).then(&t.transform).then(&place).then(&flip);
            let c = m.coefficients();
            // Tm takes the text matrix scaled by Tf: normalise by nothing,
            // the size is applied through Tf.
            let _ = writeln!(
                content,
                "{} {} {} {} {} {} Tm <{:04X}> Tj",
                vg::pdf::num_matrix(c[0]),
                vg::pdf::num_matrix(c[1]),
                vg::pdf::num_matrix(c[2]),
                vg::pdf::num_matrix(c[3]),
                num(c[4]),
                num(c[5]),
                gid
            );
        }
        content.push_str("ET\nQ\n");
    }
    let total = picture.items.len();
    emit_items(total, &mut content, &mut ext_states, &mut chunk, &mut items_done)?;

    // ---- objects
    let mut objs: Vec<Vec<u8>> = Vec::new();
    let mut add = |o: Vec<u8>| {
        objs.push(o);
        objs.len()
    };
    let catalog = add(Vec::new());
    let pages = add(Vec::new());
    let page = add(Vec::new());
    let contents = add(stream(&[], content.as_bytes()));
    let mut font_refs = String::new();
    for (name, face) in font_objs.values() {
        let program = face.program().unwrap_or(&[]);
        let ps = face.postscript_name.replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "");
        let file = add(stream(&[("Subtype", "/OpenType".to_string())], program));
        let desc = add(
            format!(
                "<< /Type /FontDescriptor /FontName /{ps} /Flags 32 /FontBBox [-500 -300 1500 1000] /ItalicAngle 0 /Ascent 900 /Descent -300 /CapHeight 700 /StemV 80 /FontFile3 {file} 0 R >>"
            )
            .into_bytes(),
        );
        let cid = add(
            format!(
                "<< /Type /Font /Subtype /CIDFontType0 /BaseFont /{ps} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {desc} 0 R /DW 1000 >>"
            )
            .into_bytes(),
        );
        let t0 = add(format!("<< /Type /Font /Subtype /Type0 /BaseFont /{ps} /Encoding /Identity-H /DescendantFonts [{cid} 0 R] >>").into_bytes());
        let _ = write!(font_refs, "/{name} {t0} 0 R ");
    }
    let mut gs_refs = String::new();
    for (name, dict) in &ext_states {
        let o = add(dict.clone().into_bytes());
        let _ = write!(gs_refs, "/{name} {o} 0 R ");
    }
    objs[catalog - 1] = format!("<< /Type /Catalog /Pages {pages} 0 R >>").into_bytes();
    objs[pages - 1] = format!("<< /Type /Pages /Kids [{page} 0 R] /Count 1 >>").into_bytes();
    objs[page - 1] = format!(
        "<< /Type /Page /Parent {pages} 0 R /MediaBox [0 0 {} {}] /Resources << /Font << {font_refs}>> /ExtGState << {gs_refs}>> >> /Contents {contents} 0 R >>",
        num(w),
        num(h)
    )
    .into_bytes();
    let mut out: Vec<u8> = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for off in offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root {catalog} 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes());
    Ok(StandalonePdf {
        bytes: out,
        picture,
        width_bp: w,
        height_bp: h,
    })
}

fn stream(extra: &[(&str, String)], data: &[u8]) -> Vec<u8> {
    let mut dict = format!("<< /Length {}", data.len());
    for (k, v) in extra {
        let _ = write!(dict, " /{k} {v}");
    }
    dict.push_str(" >>\nstream\n");
    let mut o = dict.into_bytes();
    o.extend_from_slice(data);
    o.extend_from_slice(b"\nendstream");
    o
}
