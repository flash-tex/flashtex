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

// ---- tikz-cd commutative diagrams: minimal slice-1 reader ----
//
// `\begin{tikzcd}` matrices with `\arrow[r]`/`\arrow[d]`-style arrows
// between adjacent cells lower to stroked shafts, filled heads and one
// text placement per cell. Only single-step `r`/`l`/`u`/`d` arrows are
// drawn; everything else degrades to a warning diagnostic, never a silent
// drop (the `vg::tikz` reader's convention).
//
// This lives in the render pipeline rather than `flashtex-vector-graphics`
// because this crate builds against the frozen `vendor/` snapshot, which is
// read-only: the proposed next slice moves it to
// `crates/vector-graphics/src/tikz` at the vendor re-pin and routes
// `tikzcd` through the picture pipeline (`adapter`, `lib`, `typeset`).
// Plain `tikzpicture` rendering above is untouched.
const TIKZCD_BEGIN: &str = "\\begin{tikzcd}";
const TIKZCD_END: &str = "\\end{tikzcd}";

/// Finds the `tikzcd` environments in a document, skipping `%` comments.
///
/// This mirrors `vg::tikz::find_pictures` and reuses its `PictureSource`,
/// so both can feed one picture pipeline later.
pub fn find_tikzcds(doc: &str) -> Vec<vg::tikz::PictureSource> {
    let clean = vg::tikz::text::blank_comments(doc);
    let clean = clean.as_str();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(rel) = clean[from..].find(TIKZCD_BEGIN) {
        let start = from + rel;
        let mut body_start = start + TIKZCD_BEGIN.len();
        let mut options = None;
        let after = clean[body_start..].trim_start();
        let skipped = clean[body_start..].len() - after.len();
        if after.starts_with('[') {
            let open = body_start + skipped;
            if let Some(close) = vg::tikz::text::matching(clean, open) {
                options = Some((open + 1, close - 1));
                body_start = close;
            }
        }
        let Some(erel) = clean[body_start..].find(TIKZCD_END) else {
            break;
        };
        let body_end = body_start + erel;
        out.push(vg::tikz::PictureSource {
            start,
            end: body_end + TIKZCD_END.len(),
            body_start,
            body_end,
            options,
        });
        from = body_end + TIKZCD_END.len();
    }
    out
}

/// One `\arrow`/`\ar` of a cell: a single step in row/column deltas.
struct TikzcdArrow {
    dr: i32,
    dc: i32,
    span: (usize, usize),
}

/// One matrix cell: display text (arrow commands removed) and its arrows.
struct TikzcdCell {
    text: String,
    span: (usize, usize),
    arrows: Vec<TikzcdArrow>,
}

fn tikzcd_warn(diags: &mut Vec<vg::tikz::Diagnostic>, message: String, span: (usize, usize)) {
    diags.push(vg::tikz::Diagnostic {
        severity: vg::tikz::Severity::Warning,
        message,
        start: span.0,
        end: span.1,
    });
}

/// Byte ranges of the `\\`-separated rows, plus the `\\[...]` spacing
/// arguments skipped after each separator (the caller warns on those).
fn tikzcd_rows(body: &str) -> (Vec<(usize, usize)>, Vec<(usize, usize)>) {
    let b = body.as_bytes();
    let mut rows = Vec::new();
    let mut spacings = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && b.get(i + 1) == Some(&b'\\') {
            rows.push((start, i));
            i += 2;
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            if b.get(i) == Some(&b'[') {
                let open = i;
                let mut depth = 0;
                while i < b.len() {
                    if b[i] == b'[' {
                        depth += 1;
                    }
                    if b[i] == b']' {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    i += 1;
                }
                spacings.push((open, i));
            }
            start = i;
            continue;
        }
        i += 1;
    }
    rows.push((start, b.len()));
    (rows, spacings)
}

/// Byte ranges of the `&`-separated cells of one row, honouring `\` escapes
/// (`\&` stays literal). Ranges are relative to `row`.
fn tikzcd_cells(row: &str) -> Vec<(usize, usize)> {
    let b = row.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' {
            i += 2;
            continue;
        }
        if b[i] == b'&' {
            out.push((start, i));
            start = i + 1;
        }
        i += 1;
    }
    out.push((start, b.len()));
    out
}

/// Start and end (exclusive) of the arrow command name (`\arrow` before
/// `\ar`, with a non-letter boundary) at or after `from`, if any.
fn tikzcd_command(cell: &str, from: usize) -> Option<(usize, usize)> {
    let b = cell.as_bytes();
    let mut i = from;
    while i < b.len() {
        if b[i] == b'\\' {
            for name in ["\\arrow", "\\ar"] {
                if cell[i..].starts_with(name) {
                    let end = i + name.len();
                    if end >= b.len() || !b[end].is_ascii_alphabetic() {
                        return Some((i, end));
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// Parses one `\arrow[...]` option list: the single direction plus a warning
/// for everything else (labels, styles, `from=`/`to=`) or for no direction.
fn tikzcd_arrow(opts: &str, span: (usize, usize), diags: &mut Vec<vg::tikz::Diagnostic>) -> Option<TikzcdArrow> {
    let mut dir: Option<(i32, i32)> = None;
    let mut unsupported: Vec<&str> = Vec::new();
    for opt in vg::tikz::text::split_top(opts, b',') {
        let o = opt.trim();
        let d = match o {
            "r" => Some((0, 1)),
            "l" => Some((0, -1)),
            "u" => Some((-1, 0)),
            "d" => Some((1, 0)),
            _ => None,
        };
        match d {
            Some(v) if dir.is_none() => dir = Some(v),
            _ if o.is_empty() => {}
            _ => unsupported.push(o),
        }
    }
    if !unsupported.is_empty() {
        let s = if unsupported.len() == 1 { "" } else { "s" };
        tikzcd_warn(
            diags,
            format!("unsupported tikzcd arrow option{s} ({}); ignored", unsupported.join(", ")),
            span,
        );
    }
    match dir {
        Some((dr, dc)) => Some(TikzcdArrow { dr, dc, span }),
        None => {
            tikzcd_warn(diags, "tikzcd arrow without a direction (expected one of r, l, u, d); dropped".to_string(), span);
            None
        }
    }
}

/// Splits one cell into display text and arrows. `cell` is already trimmed;
/// `abs_start` is its offset in the source document.
fn tikzcd_cell(cell: &str, abs_start: usize, diags: &mut Vec<vg::tikz::Diagnostic>) -> TikzcdCell {
    let mut kept = String::new();
    let mut arrows = Vec::new();
    let mut pos = 0;
    let mut cursor = 0;
    while let Some((s, e)) = tikzcd_command(cell, pos) {
        let mut end = e;
        let tail = &cell[e..];
        let trimmed = tail.trim_start();
        if trimmed.starts_with('[') {
            let open = e + (tail.len() - trimmed.len());
            match vg::tikz::text::matching(cell, open) {
                Some(close) => {
                    end = close;
                    if let Some(a) = tikzcd_arrow(&cell[open + 1..close - 1], (abs_start + s, abs_start + end), diags) {
                        arrows.push(a);
                    }
                }
                None => end = cell.len(),
            }
        } else {
            tikzcd_arrow("", (abs_start + s, abs_start + end), diags);
        }
        kept.push_str(&cell[cursor..s]);
        cursor = end;
        pos = end;
    }
    kept.push_str(&cell[cursor..]);
    TikzcdCell {
        text: kept.trim().to_string(),
        span: (abs_start, abs_start + cell.len()),
        arrows,
    }
}

/// Compiles one `tikzcd` environment into a `Picture` in picture space (PDF
/// points, top-left origin, y down), like `Tikz::render`. `font_size_pt` is
/// the document body size; cells are set at that size, unstyled.
pub fn render_tikzcd(source: &str, picture: &vg::tikz::PictureSource, measurer: &dyn TextMeasurer, font_size_pt: f64) -> Picture {
    let base = picture.start;
    let clean = vg::tikz::text::blank_comments(&source[base..picture.end]);
    let clean = clean.as_str();
    let body = &clean[picture.body_start - base..picture.body_end - base];
    let mut diags = Vec::new();
    let opts = picture.options.map(|(a, b)| &clean[a - base..b - base]).unwrap_or("");
    if !opts.trim().is_empty() {
        tikzcd_warn(&mut diags, format!("tikzcd picture options [{opts}] are not supported; ignored"), (picture.start, picture.body_start));
    }
    let (rows, spacings) = tikzcd_rows(body);
    for (s, e) in spacings {
        tikzcd_warn(&mut diags, "tikzcd row spacing is not supported; ignored".to_string(), (picture.body_start + s, picture.body_start + e));
    }
    let mut grid: Vec<Vec<TikzcdCell>> = Vec::new();
    for (rs, re) in rows {
        let mut row = Vec::new();
        for (cs, ce) in tikzcd_cells(&body[rs..re]) {
            let raw = &body[rs + cs..rs + ce];
            let lead = raw.len() - raw.trim_start().len();
            row.push(tikzcd_cell(raw.trim(), picture.body_start + rs + cs + lead, &mut diags));
        }
        grid.push(row);
    }
    let style = TextStyle { size_pt: font_size_pt, bold: false, italic: false };
    let nrows = grid.len();
    let ncols = grid.iter().map(Vec::len).max().unwrap_or(0);
    let mut w: Vec<Vec<f64>> = Vec::new();
    let mut h: Vec<Vec<f64>> = Vec::new();
    let mut d: Vec<Vec<f64>> = Vec::new();
    for row in &grid {
        let (mut ww, mut hh, mut dd) = (Vec::new(), Vec::new(), Vec::new());
        for cell in row {
            let m = measurer.measure(&cell.text, &style);
            ww.push(m.width_pt);
            hh.push(m.height_pt);
            dd.push(m.depth_pt);
        }
        w.push(ww);
        h.push(hh);
        d.push(dd);
    }
    // Provisional separations (2.5em): real tikz-cd defaults are larger and
    // configurable; only the matrix-with-visible-arrows shape is locked here.
    let col_sep = 2.5 * font_size_pt;
    let row_sep = 2.5 * font_size_pt;
    let mut col_w: Vec<f64> = vec![0.0; ncols];
    let mut row_above: Vec<f64> = vec![0.0; nrows];
    let mut row_below: Vec<f64> = vec![0.0; nrows];
    for (r, row) in grid.iter().enumerate() {
        for (c, _) in row.iter().enumerate() {
            col_w[c] = col_w[c].max(w[r][c]);
            row_above[r] = row_above[r].max(h[r][c]);
            row_below[r] = row_below[r].max(d[r][c]);
        }
    }
    let mut col_x = vec![0.0; ncols + 1];
    for c in 0..ncols {
        col_x[c + 1] = col_x[c] + col_w[c] + col_sep;
    }
    let mut row_y = vec![0.0; nrows + 1];
    for r in 0..nrows {
        row_y[r + 1] = row_y[r] + row_above[r] + row_below[r] + row_sep;
    }
    let total_w = col_x[ncols] - if ncols > 0 { col_sep } else { 0.0 };
    let total_h = row_y[nrows] - if nrows > 0 { row_sep } else { 0.0 };
    let k = BP_PER_PT;
    const TIP_LEN_PT: f64 = 3.0;
    const TIP_HALF_PT: f64 = 1.2;
    let mut items: Vec<Item> = Vec::new();
    let mut id: u64 = 0;
    for (r, row) in grid.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            for a in &cell.arrows {
                let tr = r as i32 + a.dr;
                let tc = c as i32 + a.dc;
                if tr < 0 || tc < 0 || grid.get(tr as usize).map(|t| (tc as usize) >= t.len()).unwrap_or(true) {
                    tikzcd_warn(&mut diags, "tikzcd arrow points outside the diagram; dropped".to_string(), a.span);
                    continue;
                }
                let (tr, tc) = (tr as usize, tc as usize);
                let sbx = col_x[c] + (col_w[c] - w[r][c]) / 2.0;
                let tbx = col_x[tc] + (col_w[tc] - w[tr][tc]) / 2.0;
                let smy = row_y[r] + row_above[r] - h[r][c] + (h[r][c] + d[r][c]) / 2.0;
                let tmy = row_y[tr] + row_above[tr] - h[tr][tc] + (h[tr][tc] + d[tr][tc]) / 2.0;
                let (x1, y1, x2, y2) = if a.dc != 0 {
                    let (lx, rx) = if a.dc > 0 { (sbx + w[r][c], tbx) } else { (sbx, tbx + w[tr][tc]) };
                    (lx, smy, rx, tmy)
                } else {
                    let (ey1, ey2) = if a.dr > 0 {
                        (row_y[r] + row_above[r] + row_below[r], row_y[tr])
                    } else {
                        (row_y[r], row_y[tr] + row_above[tr] + row_below[tr])
                    };
                    (sbx + w[r][c] / 2.0, ey1, tbx + w[tr][tc] / 2.0, ey2)
                };
                let (dx, dy) = (x2 - x1, y2 - y1);
                let len = dx.hypot(dy);
                if len <= 0.0 {
                    continue;
                }
                let (ux, uy) = (dx / len, dy / len);
                let p1 = vg::Point::new(x1 * k, y1 * k);
                let p2 = vg::Point::new(x2 * k, y2 * k);
                let q = if len > TIP_LEN_PT {
                    vg::Point::new((x2 - ux * TIP_LEN_PT) * k, (y2 - uy * TIP_LEN_PT) * k)
                } else {
                    p2
                };
                let mut shaft = vg::Path::new();
                shaft.move_to(p1);
                shaft.line_to(q);
                id += 1;
                items.push(Item::PathStroke(vg::PathStroke {
                    id: ItemId(id),
                    path: shaft,
                    style: vg::StrokeStyle::with_width(0.4 * k),
                    paint: vg::Paint::BLACK,
                    source: None,
                }));
                if len > TIP_LEN_PT {
                    let (bx, by) = (x2 - ux * TIP_LEN_PT, y2 - uy * TIP_LEN_PT);
                    let (hx, hy) = (-uy * TIP_HALF_PT, ux * TIP_HALF_PT);
                    let mut head = vg::Path::new();
                    head.move_to(p2);
                    head.line_to(vg::Point::new((bx + hx) * k, (by + hy) * k));
                    head.line_to(vg::Point::new((bx - hx) * k, (by - hy) * k));
                    head.close();
                    id += 1;
                    items.push(Item::PathFill(vg::PathFill {
                        id: ItemId(id),
                        path: head,
                        rule: vg::FillRule::NonZero,
                        paint: vg::Paint::BLACK,
                        source: None,
                    }));
                }
            }
        }
    }
    let mut texts = Vec::new();
    for (r, row) in grid.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            if cell.text.is_empty() {
                continue;
            }
            texts.push(vg::tikz::PictureText {
                text: cell.text.clone(),
                style,
                transform: Transform::translate((col_x[c] + (col_w[c] - w[r][c]) / 2.0) * k, (row_y[r] + row_above[r]) * k),
                paint: vg::Paint::BLACK,
                after_item: items.len(),
                source: cell.span,
            });
        }
    }
    Picture {
        width_bp: total_w.max(0.0) * k,
        height_bp: total_h.max(0.0) * k,
        items,
        texts,
        diagnostics: diags,
    }
}
