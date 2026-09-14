//! `\includegraphics`: image header probing and graphicx sizing.
//!
//! Natural sizes follow pdfTeX (the oracle the fixtures are checked
//! against), never a TeX engine at run time:
//!
//! * PNG — `IHDR` pixels; resolution from `pHYs` when its unit is the metre,
//!   rounded to whole dpi as pdfTeX's `writepng.c` does; otherwise 72 dpi.
//! * JPEG — `SOFn` pixels; JFIF `APP0` density (unit 1 = dpi, 2 = dots per
//!   cm); otherwise 72 dpi.
//! * PDF — page 1 (or `page=`) CropBox (graphicx's default `pagebox`)
//!   clipped to the MediaBox, both inheritable through `/Parent`, with
//!   `/Rotate`. Only uncompressed page objects are read; a PDF whose page
//!   tree lives in compressed object streams is reported, never guessed.
//!
//! graphicx semantics (`graphicx.sty` `\Gin@esetsize`, `\Gin@ii`): keys
//! before the first `angle` request the size of the unrotated image
//! (`width`/`height` win over `scale`; both with `keepaspectratio` take the
//! smaller factor); `angle` rotates counter-clockwise about the reference
//! point and the box becomes the rotated bounding box; `width`/`height`/
//! `totalheight`/`scale` after an `angle` rescale that rotated box.

/// One TeX point in PDF big points.
pub const BP_PER_PT: f64 = 72.0 / 72.27;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Pdf,
}

impl ImageFormat {
    pub fn wire_name(self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpeg",
            ImageFormat::Pdf => "pdf",
        }
    }
}

/// What the header says about an image.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageInfo {
    pub format: ImageFormat,
    /// Natural size in PDF big points.
    pub width_bp: f64,
    pub height_bp: f64,
    /// Raster dimensions (PNG/JPEG).
    pub pixels: Option<(u32, u32)>,
    /// PDF only: the box's lower-left corner in the page's own space and
    /// the page's `/Rotate` (0/90/180/270), so a painter can map the page.
    pub pdf_box: Option<[f64; 4]>,
    pub pdf_rotate: i32,
    pub pdf_page: u32,
}

/// Probes `bytes` (any of the supported formats, sniffed by signature).
pub fn probe(bytes: &[u8], pdf_page: u32) -> Result<ImageInfo, String> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        probe_png(bytes)
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        probe_jpeg(bytes)
    } else if bytes.starts_with(b"%PDF-") {
        probe_pdf(bytes, pdf_page.max(1))
    } else {
        Err("not a PNG, JPEG or PDF file (unrecognised signature)".into())
    }
}

fn be32(b: &[u8]) -> u32 {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

fn probe_png(b: &[u8]) -> Result<ImageInfo, String> {
    let mut at = 8usize;
    let mut size = None;
    let mut dpi = None;
    while at + 12 <= b.len() {
        let len = be32(&b[at..]) as usize;
        let kind = &b[at + 4..at + 8];
        let data = b.get(at + 8..at + 8 + len).ok_or("PNG chunk runs past the end of the file")?;
        match kind {
            b"IHDR" if len >= 8 => size = Some((be32(data), be32(&data[4..]))),
            b"pHYs" if len >= 9 && data[8] == 1 => {
                // writepng.c: (int)(pixels_per_meter * 0.0254 + 0.5)
                let x = (f64::from(be32(data)) * 0.0254 + 0.5).floor();
                let y = (f64::from(be32(&data[4..])) * 0.0254 + 0.5).floor();
                if x > 0.0 && y > 0.0 {
                    dpi = Some((x, y));
                }
            }
            b"IDAT" | b"IEND" => break,
            _ => {}
        }
        at += 12 + len;
    }
    let (w, h) = size.ok_or("PNG has no IHDR chunk")?;
    if w == 0 || h == 0 {
        return Err("PNG has a zero dimension".into());
    }
    let (dx, dy) = dpi.unwrap_or((72.0, 72.0));
    Ok(ImageInfo {
        format: ImageFormat::Png,
        width_bp: f64::from(w) * 72.0 / dx,
        height_bp: f64::from(h) * 72.0 / dy,
        pixels: Some((w, h)),
        pdf_box: None,
        pdf_rotate: 0,
        pdf_page: 0,
    })
}

fn probe_jpeg(b: &[u8]) -> Result<ImageInfo, String> {
    let mut at = 2usize;
    let mut dpi = None;
    while at + 4 <= b.len() {
        if b[at] != 0xFF {
            return Err("JPEG marker stream is corrupt".into());
        }
        let marker = b[at + 1];
        if marker == 0xFF {
            at += 1;
            continue;
        }
        if marker == 0xD8 || (0xD0..=0xD7).contains(&marker) {
            at += 2;
            continue;
        }
        let len = usize::from(u16::from_be_bytes([b[at + 2], b[at + 3]]));
        let seg = b.get(at + 4..at + 2 + len).ok_or("JPEG segment runs past the end of the file")?;
        match marker {
            0xE0 if seg.len() >= 12 && seg.starts_with(b"JFIF\0") => {
                let unit = seg[7];
                let x = f64::from(u16::from_be_bytes([seg[8], seg[9]]));
                let y = f64::from(u16::from_be_bytes([seg[10], seg[11]]));
                if x > 0.0 && y > 0.0 {
                    dpi = match unit {
                        1 => Some((x, y)),
                        2 => Some((x * 2.54, y * 2.54)),
                        _ => None,
                    };
                }
            }
            0xC0..=0xCF if !matches!(marker, 0xC4 | 0xC8 | 0xCC) => {
                if seg.len() < 5 {
                    return Err("JPEG frame header is truncated".into());
                }
                let h = u32::from(u16::from_be_bytes([seg[1], seg[2]]));
                let w = u32::from(u16::from_be_bytes([seg[3], seg[4]]));
                if w == 0 || h == 0 {
                    return Err("JPEG has a zero dimension".into());
                }
                let (dx, dy) = dpi.unwrap_or((72.0, 72.0));
                return Ok(ImageInfo {
                    format: ImageFormat::Jpeg,
                    width_bp: f64::from(w) * 72.0 / dx,
                    height_bp: f64::from(h) * 72.0 / dy,
                    pixels: Some((w, h)),
                    pdf_box: None,
                    pdf_rotate: 0,
                    pdf_page: 0,
                });
            }
            _ => {}
        }
        at += 2 + len;
    }
    Err("JPEG has no frame header".into())
}

/// The dictionary text of `N 0 obj << ... >>` (uncompressed objects only).
fn pdf_object(b: &[u8], num: u32) -> Option<&[u8]> {
    let needle = format!("{num} 0 obj");
    let mut from = 0;
    while let Some(pos) = find(&b[from..], needle.as_bytes()) {
        let start = from + pos;
        let ok_before = start == 0 || !b[start - 1].is_ascii_digit();
        if ok_before {
            let body = &b[start + needle.len()..];
            let end = find(body, b"endobj").unwrap_or(body.len());
            return Some(&body[..end]);
        }
        from = start + needle.len();
    }
    None
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// The value text after `/Key` in a dictionary (not in a nested stream).
fn dict_value<'a>(dict: &'a [u8], key: &str) -> Option<&'a [u8]> {
    let k = format!("/{key}");
    let mut from = 0;
    while let Some(pos) = find(&dict[from..], k.as_bytes()) {
        let after = from + pos + k.len();
        // `/Type` must not match `/TypeX`.
        if dict.get(after).is_none_or(|c| !c.is_ascii_alphanumeric()) {
            let v = &dict[after..];
            let skip = v.iter().position(|c| !c.is_ascii_whitespace()).unwrap_or(v.len());
            return Some(&v[skip..]);
        }
        from = after;
    }
    None
}

fn ref_num(v: &[u8]) -> Option<u32> {
    let s = std::str::from_utf8(&v[..v.len().min(32)]).ok()?;
    let mut it = s.split_whitespace();
    let n = it.next()?.parse().ok()?;
    let _g: u32 = it.next()?.parse().ok()?;
    it.next()?.starts_with('R').then_some(n)
}

fn number_array(v: &[u8]) -> Option<[f64; 4]> {
    if v.first() != Some(&b'[') {
        return None;
    }
    let end = v.iter().position(|c| *c == b']')?;
    let s = std::str::from_utf8(&v[1..end]).ok()?;
    let n: Vec<f64> = s.split_whitespace().filter_map(|t| t.parse().ok()).collect();
    (n.len() == 4).then(|| [n[0].min(n[2]), n[1].min(n[3]), n[0].max(n[2]), n[1].max(n[3])])
}

fn is_type(dict: &[u8], ty: &str) -> bool {
    dict_value(dict, "Type").is_some_and(|v| v.starts_with(format!("/{ty}").as_bytes()) && v.get(ty.len() + 1).is_none_or(|c| !c.is_ascii_alphanumeric()))
}

fn probe_pdf(b: &[u8], page: u32) -> Result<ImageInfo, String> {
    let compressed = find(b, b"/ObjStm").is_some();
    let trailer_root = find(b, b"/Root").and_then(|p| ref_num(dict_value(&b[p..], "Root")?));
    let fail = |what: &str| {
        if compressed {
            format!("PDF {what}: the page tree is in a compressed object stream, which is not read yet")
        } else {
            format!("PDF {what}")
        }
    };
    let root = trailer_root.ok_or_else(|| fail("has no /Root"))?;
    let catalog = pdf_object(b, root).ok_or_else(|| fail("catalog object not found"))?;
    let pages = dict_value(catalog, "Pages").and_then(ref_num).ok_or_else(|| fail("catalog has no /Pages"))?;
    // Walk the page tree to the requested page (1-based), depth-first.
    let mut remaining = page;
    let mut node = pages;
    let mut chain: Vec<u32> = Vec::new();
    'walk: for _ in 0..64 {
        let dict = pdf_object(b, node).ok_or_else(|| fail("page tree object not found"))?;
        chain.push(node);
        if is_type(dict, "Page") {
            if remaining == 1 {
                break 'walk;
            }
            return Err(format!("PDF has fewer than {page} pages"));
        }
        let kids = dict_value(dict, "Kids").ok_or_else(|| fail("page tree node has no /Kids"))?;
        let end = kids.iter().position(|c| *c == b']').unwrap_or(kids.len());
        let text = std::str::from_utf8(&kids[1.min(end)..end]).map_err(|_| fail("has unreadable /Kids"))?;
        let toks: Vec<&str> = text.split_whitespace().collect();
        let mut next = None;
        for t in toks.chunks(3) {
            let Some(n) = t.first().and_then(|n| n.parse::<u32>().ok()) else { continue };
            let kid = pdf_object(b, n).ok_or_else(|| fail("page object not found"))?;
            let count = if is_type(kid, "Page") {
                1
            } else {
                dict_value(kid, "Count").and_then(|v| std::str::from_utf8(&v[..v.len().min(12)]).ok()?.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()).unwrap_or(1)
            };
            if remaining <= count {
                next = Some(n);
                break;
            }
            remaining -= count;
        }
        match next {
            Some(n) => node = n,
            None => return Err(format!("PDF has fewer than {page} pages")),
        }
    }
    // Inheritable attributes: nearest ancestor wins.
    let inherited = |key: &str| -> Option<&[u8]> {
        chain.iter().rev().find_map(|n| pdf_object(b, *n).and_then(|d| dict_value(d, key)))
    };
    let media = inherited("MediaBox").and_then(number_array).ok_or_else(|| fail("page has no readable /MediaBox"))?;
    let crop = inherited("CropBox").and_then(number_array).map(|c| [c[0].max(media[0]), c[1].max(media[1]), c[2].min(media[2]), c[3].min(media[3])]).unwrap_or(media);
    let rotate = inherited("Rotate")
        .and_then(|v| std::str::from_utf8(&v[..v.len().min(8)]).ok()?.split(|c: char| !(c.is_ascii_digit() || c == '-')).next()?.parse::<i32>().ok())
        .unwrap_or(0)
        .rem_euclid(360);
    let (w, h) = (crop[2] - crop[0], crop[3] - crop[1]);
    if w <= 0.0 || h <= 0.0 {
        return Err("PDF page box is empty".into());
    }
    let (w, h) = if rotate % 180 == 90 { (h, w) } else { (w, h) };
    Ok(ImageInfo {
        format: ImageFormat::Pdf,
        width_bp: w,
        height_bp: h,
        pixels: None,
        pdf_box: Some(crop),
        pdf_rotate: rotate,
        pdf_page: page,
    })
}

/// Lengths a graphicx dimension may refer to, in TeX points.
#[derive(Debug, Clone, Copy)]
pub struct LengthEnv {
    pub text_width: f64,
    pub text_height: f64,
    pub paper_width: f64,
    pub paper_height: f64,
    /// `em`/`ex` of the current font.
    pub em: f64,
    pub ex: f64,
}

/// `<factor><unit>` or `<factor>\textwidth`-style dimension, in TeX points.
pub fn parse_dimen(raw: &str, env: &LengthEnv) -> Option<f64> {
    let s: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    let split = s.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == ',')).unwrap_or(s.len());
    let (num, unit) = s.split_at(split);
    let factor = if num.is_empty() || num == "-" || num == "+" {
        if num == "-" { -1.0 } else { 1.0 }
    } else {
        num.replace(',', ".").parse::<f64>().ok()?
    };
    let per = match unit {
        "pt" => 1.0,
        "bp" => 72.27 / 72.0,
        "in" => 72.27,
        "cm" => 72.27 / 2.54,
        "mm" => 72.27 / 25.4,
        "pc" => 12.0,
        "dd" => 1238.0 / 1157.0,
        "cc" => 12.0 * 1238.0 / 1157.0,
        "sp" => 1.0 / 65536.0,
        "em" => env.em,
        "ex" => env.ex,
        "\\textwidth" | "\\linewidth" | "\\columnwidth" | "\\hsize" => env.text_width,
        "\\textheight" | "\\vsize" => env.text_height,
        "\\paperwidth" => env.paper_width,
        "\\paperheight" => env.paper_height,
        _ => return None,
    };
    Some(factor * per)
}

/// One parsed `key=value` of the optional argument, in order.
#[derive(Debug, Clone, PartialEq)]
pub enum GKey {
    Width(f64),
    Height(f64),
    TotalHeight(f64),
    Scale(f64),
    Angle(f64),
    KeepAspectRatio(bool),
    Page(u32),
    /// `viewport=<llx> <lly> <urx> <ury>` in TeX points, relative to the
    /// image's natural bounding-box origin. The compiler records graphics.sty's
    /// two-bracket `\includegraphics[llx,lly][urx,ury]{..}` in this form, as
    /// `pdftex.def`'s `\Gin@iii@vp` does.
    Viewport([f64; 4]),
    /// `trim=<left> <bottom> <right> <top>` in TeX points: insets from the
    /// four edges of the natural bounding box.
    Trim([f64; 4]),
    /// `clip` (and graphics.sty's `\includegraphics*`).
    Clip(bool),
    /// Recognised but not honoured (`bb`, `natwidth`, `origin`, ...):
    /// reported as a limitation.
    Unsupported(String),
}

/// Four graphicx dimensions separated by spaces or commas, defaulting to big
/// points when a number carries no unit (`\Gin@defaultbp`), in TeX points.
fn parse_bp_quad(raw: &str, env: &LengthEnv) -> Option<[f64; 4]> {
    let mut out = [0.0; 4];
    let mut n = 0;
    for field in raw.split(|c: char| c.is_whitespace() || c == ',').filter(|f| !f.is_empty()) {
        if n == 4 {
            return None;
        }
        let bare = field.bytes().all(|b| b.is_ascii_digit() || b == b'.' || b == b'-' || b == b'+');
        let value = if bare { parse_dimen(&format!("{field}bp"), env) } else { parse_dimen(field, env) };
        out[n] = value?;
        n += 1;
    }
    (n == 4).then_some(out)
}

/// Parses the optional argument. Errors name the offending entry.
pub fn parse_keys(options: &str, env: &LengthEnv) -> (Vec<GKey>, Vec<String>) {
    let mut keys = Vec::new();
    let mut problems = Vec::new();
    for entry in split_top_level(options) {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (k, v) = match entry.split_once('=') {
            Some((k, v)) => (k.trim(), Some(v.trim().trim_matches(|c| c == '{' || c == '}'))),
            None => (entry, None),
        };
        let dimen = |v: Option<&str>| v.and_then(|v| parse_dimen(v, env));
        let number = |v: Option<&str>| v.and_then(|v| v.parse::<f64>().ok());
        let key = match k {
            "width" => dimen(v).map(GKey::Width),
            "height" => dimen(v).map(GKey::Height),
            "totalheight" => dimen(v).map(GKey::TotalHeight),
            "scale" => number(v).map(GKey::Scale),
            "angle" => number(v).map(GKey::Angle),
            "keepaspectratio" => Some(GKey::KeepAspectRatio(v.is_none_or(|v| v != "false"))),
            "page" => v.and_then(|v| v.parse().ok()).map(GKey::Page),
            "viewport" => v.and_then(|v| parse_bp_quad(v, env)).map(GKey::Viewport),
            "trim" => v.and_then(|v| parse_bp_quad(v, env)).map(GKey::Trim),
            "clip" => Some(GKey::Clip(v.is_none_or(|v| v != "false"))),
            "bb" | "natwidth" | "natheight" | "origin" | "draft" | "pagebox" | "decodearray" | "interpolate" => Some(GKey::Unsupported(k.to_string())),
            "alt" | "actualtext" | "artifact" | "quiet" => None,
            _ => {
                problems.push(format!("unknown \\includegraphics key '{k}'"));
                continue;
            }
        };
        match key {
            Some(key) => keys.push(key),
            None if matches!(k, "alt" | "actualtext" | "artifact" | "quiet") => {}
            None => problems.push(format!("could not read \\includegraphics key '{entry}'")),
        }
    }
    (keys, problems)
}

fn split_top_level(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut depth, mut start) = (0i32, 0usize);
    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

/// The placed box of one graphic, in TeX points, plus how the image maps
/// into it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphicBox {
    pub width: f64,
    pub height: f64,
    pub depth: f64,
    /// Affine map from the image's unit square (u right, v up, origin at
    /// the image's lower-left) to box coordinates (x right from the box's
    /// left edge, y UP from the baseline), in TeX points:
    /// `(x, y) = (e + a*u + c*v, f + b*u + d*v)`.
    pub matrix: [f64; 6],
}

/// graphicx sizing of an image whose natural size is `nat_w` x `nat_h`
/// TeX points.
///
/// Two affine maps are carried side by side, in the same box coordinates:
/// `m` places the *image*'s unit square and `r` places the *box*'s. They are
/// equal unless a `viewport`/`trim` moves the box off the image, which is
/// exactly what those keys do (`pdftex.def` `\Gin@viewport`): the box becomes
/// the requested rectangle and the image keeps its natural size, shifted so
/// that the rectangle's lower-left corner sits at the box origin. Every later
/// operation — `\Gin@esetsize`'s scaling and `\Grot@box`'s rotation — acts on
/// both, so the box is always the *requested* rectangle's image and never the
/// picture's.
pub fn size_box(nat_w: f64, nat_h: f64, keys: &[GKey]) -> GraphicBox {
    // Image placement and box placement: [a b c d e f] of a unit square.
    let mut m = [nat_w, 0.0, 0.0, nat_h, 0.0, 0.0];
    let mut r = m;
    // `viewport` and `trim` both set the box rectangle (last one wins, as in
    // `\Gin@ii`'s key loop); `trim` states insets from the four edges.
    if let Some(vp) = keys.iter().rev().find_map(|k| match *k {
        GKey::Viewport(v) => Some(v),
        GKey::Trim([l, b, right, t]) => Some([l, b, nat_w - right, nat_h - t]),
        _ => None,
    }) {
        m[4] = -vp[0];
        m[5] = -vp[1];
        r = [vp[2] - vp[0], 0.0, 0.0, vp[3] - vp[1], 0.0, 0.0];
    }
    let (mut w, mut h, mut th, mut scale, mut iso) = (None, None, None, None, false);
    // `\Gin@esetsize`: the request is against the box as it stands — the
    // natural (or viewport) rectangle before the first `angle`, the rotated
    // bounding box after it.
    let apply_request = |m: &mut [f64; 6], r: &mut [f64; 6], w: Option<f64>, h: Option<f64>, th: Option<f64>, scale: Option<f64>, iso: bool| {
        let h = h.or(th);
        let (bw, bh, bd) = extents(r);
        let (sx, sy) = match (w, h) {
            (None, None) => match scale {
                Some(s) => (s, s),
                None => return,
            },
            (Some(w), None) => (w / bw, w / bw),
            (None, Some(hh)) => {
                let total = if th.is_some() { bh + bd } else { bh };
                (hh / total, hh / total)
            }
            (Some(w), Some(hh)) => {
                let total = if th.is_some() { bh + bd } else { bh };
                let (sx, sy) = (w / bw, hh / total);
                if iso {
                    let s = sx.min(sy);
                    (s, s)
                } else {
                    (sx, sy)
                }
            }
        };
        for t in [&mut *m, r] {
            for i in [0, 2, 4] {
                t[i] *= sx;
            }
            for i in [1, 3, 5] {
                t[i] *= sy;
            }
        }
    };
    for k in keys {
        match *k {
            GKey::Width(v) => w = Some(v),
            GKey::Height(v) => h = Some(v),
            GKey::TotalHeight(v) => th = Some(v),
            GKey::Scale(v) => scale = Some(v),
            GKey::KeepAspectRatio(v) => iso = v,
            GKey::Angle(deg) => {
                apply_request(&mut m, &mut r, w, h, th, scale, iso);
                (w, h, th, scale) = (None, None, None, None);
                let (s, c) = deg.to_radians().sin_cos();
                // Exact quarter turns.
                let (s, c) = if (deg / 90.0).fract() == 0.0 { (s.round(), c.round()) } else { (s, c) };
                let rot = |t: &[f64; 6]| [c * t[0] - s * t[1], s * t[0] + c * t[1], c * t[2] - s * t[3], s * t[2] + c * t[3], c * t[4] - s * t[5], s * t[4] + c * t[5]];
                m = rot(&m);
                r = rot(&r);
                // \Grot@box: the new box's left edge is the bounding box's.
                // The image rides along, so the same shift applies to both.
                let min_x = [0.0, r[0], r[2], r[0] + r[2]].into_iter().fold(f64::INFINITY, f64::min) + r[4];
                m[4] -= min_x;
                r[4] -= min_x;
            }
            GKey::Page(_) | GKey::Clip(_) | GKey::Viewport(_) | GKey::Trim(_) | GKey::Unsupported(_) => {}
        }
    }
    apply_request(&mut m, &mut r, w, h, th, scale, iso);
    let (width, height, depth) = extents(&r);
    GraphicBox { width, height, depth, matrix: m }
}

/// Whether `keys` (with graphics.sty's `\includegraphics*`, which is `clip`)
/// ask for a crop this pipeline cannot paint: the display list places an
/// image by an affine transform and carries no clip path, so a cropped image
/// gets the right *box* and is drawn whole. `None` when nothing is cropped —
/// `\includegraphics*` on its own clips to the natural bounding box, which
/// removes nothing.
pub fn clip_limitation(keys: &[GKey], starred: bool, nat_w: f64, nat_h: f64) -> Option<String> {
    let clip = starred || keys.iter().rev().find_map(|k| if let GKey::Clip(v) = k { Some(*v) } else { None }).unwrap_or(false);
    if !clip {
        return None;
    }
    let vp = keys.iter().rev().find_map(|k| match *k {
        GKey::Viewport(v) => Some(v),
        GKey::Trim([l, b, r, t]) => Some([l, b, nat_w - r, nat_h - t]),
        _ => None,
    })?;
    let crops = vp[0] > 1e-9 || vp[1] > 1e-9 || vp[2] < nat_w - 1e-9 || vp[3] < nat_h - 1e-9;
    crops.then(|| "the image is drawn whole: this display list places an image by an affine transform and has no clip path, so `clip` (and `\\includegraphics*`) set the box but do not cut the picture".to_string())
}

/// (width, height above the baseline, depth below it) of the unit square's
/// image under `m`.
fn extents(m: &[f64; 6]) -> (f64, f64, f64) {
    let xs = [m[4], m[4] + m[0], m[4] + m[2], m[4] + m[0] + m[2]];
    let ys = [m[5], m[5] + m[1], m[5] + m[3], m[5] + m[1] + m[3]];
    let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min).min(0.0);
    let max_x = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let max_y = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max).max(0.0);
    let min_y = ys.iter().copied().fold(f64::INFINITY, f64::min).min(0.0);
    (max_x - min_x, max_y, -min_y)
}

/// graphicx's extension search for a file named without one (pdfTeX
/// `\Gin@extensions` order, restricted to the formats read here).
pub const EXTENSIONS: [&str; 7] = [".pdf", ".png", ".jpg", ".jpeg", ".PDF", ".PNG", ".JPG"];

/// The directory list of the project's last `\graphicspath{{a/}{b/}}`, in
/// order.
///
/// The compiler consumes `\graphicspath` without leaving a node (its own
/// module says so: the list "is re-read from the source by a consumer that
/// loads files"), so it is read from the bytes here — which is also what the
/// float path needs, since a `figure` is blanked before the compiler ever
/// sees it. A later `\graphicspath` replaces an earlier one, as the macro's
/// own `\def` does; a commented-out one is ignored.
pub fn search_dirs(texts: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for text in texts {
        let b = text.as_bytes();
        let mut i = 0usize;
        while i < b.len() {
            match b[i] {
                b'%' => i = text[i..].find('\n').map_or(b.len(), |n| i + n + 1),
                b'\\' => {
                    // A control word, or an escaped character (`\%`): either
                    // way the next byte is never the start of a comment.
                    let rest = &text[i + 1..];
                    let name_end = rest.find(|c: char| !c.is_ascii_alphabetic()).map_or(text.len(), |n| i + 1 + n);
                    if &text[i + 1..name_end] == "graphicspath" {
                        let mut j = name_end;
                        while j < b.len() && (b[j] as char).is_ascii_whitespace() {
                            j += 1;
                        }
                        if let Some((start, end)) = brace_group(text, j) {
                            found.clear();
                            found.extend(flashtex_compiler::graphics::graphics_path_entries(&text[start..end]).into_iter().map(|d| d.trim().to_string()).filter(|d| !d.is_empty()));
                            i = end + 1;
                            continue;
                        }
                    }
                    i = name_end.max(i + 2).min(b.len());
                }
                _ => i += 1,
            }
        }
    }
    found
}

/// The interior of the balanced `{...}` starting at `at`, if one does.
fn brace_group(text: &str, at: usize) -> Option<(usize, usize)> {
    let b = text.as_bytes();
    if b.get(at) != Some(&b'{') {
        return None;
    }
    let mut depth = 0usize;
    for (k, c) in text[at..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((at + 1, at + k));
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> LengthEnv {
        LengthEnv { text_width: 469.75499, text_height: 650.43, paper_width: 614.295, paper_height: 794.96999, em: 11.74988, ex: 5.16 }
    }

    #[test]
    fn png_resolution_rounds_like_pdftex() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/floats/images/green-144dpi.png");
        let info = probe(&std::fs::read(path).unwrap(), 1).unwrap();
        assert_eq!(info.pixels, Some((300, 200)));
        assert_eq!((info.width_bp, info.height_bp), (150.0, 100.0));
    }

    #[test]
    fn pdf_cropbox_is_used() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/floats/images/box-crop.pdf");
        let info = probe(&std::fs::read(path).unwrap(), 1).unwrap();
        assert_eq!((info.width_bp, info.height_bp), (200.0, 120.0));
    }

    #[test]
    fn jpeg_density_is_read() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/floats/images/blue-96dpi.jpg");
        let info = probe(&std::fs::read(path).unwrap(), 1).unwrap();
        assert_eq!(info.pixels, Some((192, 96)));
        // sips wrote JFIF units 0 (aspect ratio only, density 96) plus an
        // EXIF XResolution of 96; pdfTeX 1.40.29 ignores both and sizes the
        // image at 72 dpi (measured: \wd = 192.71951pt = 192bp).
        assert!((info.width_bp - 192.0).abs() < 1e-9, "{info:?}");
    }

    #[test]
    fn keys_follow_graphicx_order() {
        let e = env();
        let (k, p) = parse_keys("width=4in,height=1in,keepaspectratio", &e);
        assert!(p.is_empty());
        let b = size_box(200.0 / BP_PER_PT, 120.0 / BP_PER_PT, &k);
        assert!((b.height - 72.27).abs() < 1e-6 && (b.width - 120.45).abs() < 1e-6, "{b:?}");
        // angle first, then height scales the rotated box.
        let (k, _) = parse_keys("angle=90,height=1in", &e);
        let b = size_box(144.0 / BP_PER_PT, 72.0 / BP_PER_PT, &k);
        assert!((b.height - 72.27).abs() < 1e-6 && (b.width - 36.135).abs() < 1e-6 && b.depth.abs() < 1e-9, "{b:?}");
        let (k, _) = parse_keys("width=0.5\\textwidth", &e);
        assert!((size_box(100.0, 50.0, &k).width - 234.877495).abs() < 1e-6);
    }

    /// `viewport`/`trim` make the box the requested rectangle while the image
    /// keeps its natural size, shifted so the rectangle's lower-left corner
    /// sits at the box origin. Measured against pdfTeX 1.40.27 (TeX Live
    /// 2025) through `fixtures/graphics/oracle.py`:
    /// `\includegraphics[10,10][40,50]{box-crop.pdf}` sets a 30bp x 40bp box
    /// (`\wd` 30.1125pt, `\ht` 40.15pt, `\dp` 0pt).
    #[test]
    fn a_viewport_sets_the_box_and_moves_the_image() {
        let e = env();
        let bp = |v: f64| v / BP_PER_PT;
        let (k, p) = parse_keys("viewport=10 10 40 50", &e);
        assert!(p.is_empty(), "{p:?}");
        assert_eq!(k, vec![GKey::Viewport([bp(10.0), bp(10.0), bp(40.0), bp(50.0)])]);
        // A 200bp x 120bp natural size, as `box-crop.pdf` has.
        let b = size_box(bp(200.0), bp(120.0), &k);
        assert!((b.width - bp(30.0)).abs() < 1e-9 && (b.height - bp(40.0)).abs() < 1e-9 && b.depth.abs() < 1e-9, "{b:?}");
        // The image is still 200bp x 120bp, moved down and left by the
        // viewport's lower-left corner.
        assert!((b.matrix[0] - bp(200.0)).abs() < 1e-9 && (b.matrix[3] - bp(120.0)).abs() < 1e-9, "{b:?}");
        assert!((b.matrix[4] + bp(10.0)).abs() < 1e-9 && (b.matrix[5] + bp(10.0)).abs() < 1e-9, "{b:?}");
        // `trim` states insets from the four edges, so it is the same box
        // (the two routes reach it by different subtractions, so the last
        // bits of the mantissa differ).
        let (t, _) = parse_keys("trim=10 10 160 70", &e);
        let tb = size_box(bp(200.0), bp(120.0), &t);
        assert!((tb.width - b.width).abs() < 1e-9 && (tb.height - b.height).abs() < 1e-9, "{tb:?} vs {b:?}");
        assert_eq!(tb.matrix, b.matrix);
        // A later request scales the *box*, not the picture: half the width
        // of the 30bp viewport, so the image halves too.
        let (k, _) = parse_keys("viewport=10 10 40 50,width=15bp", &e);
        let h = size_box(bp(200.0), bp(120.0), &k);
        assert!((h.width - bp(15.0)).abs() < 1e-9 && (h.height - bp(20.0)).abs() < 1e-9, "{h:?}");
        assert!((h.matrix[0] - bp(100.0)).abs() < 1e-9, "{h:?}");
    }

    /// `clip` (and `\includegraphics*`) cannot be painted: this display list
    /// has no clip path. Saying nothing would be worse than saying so, but a
    /// star that crops nothing must not cry wolf.
    #[test]
    fn only_a_crop_that_removes_something_is_reported() {
        let e = env();
        let (none, _) = parse_keys("width=2in", &e);
        assert_eq!(clip_limitation(&none, true, 100.0, 50.0), None, "a bare star clips to the natural box");
        let (vp, _) = parse_keys("viewport=0 0 100 50", &e);
        assert_eq!(clip_limitation(&vp, true, 100.0 / BP_PER_PT, 50.0 / BP_PER_PT), None, "a viewport equal to the natural box removes nothing");
        let (vp, _) = parse_keys("viewport=10 0 100 50", &e);
        assert!(clip_limitation(&vp, true, 100.0 / BP_PER_PT, 50.0 / BP_PER_PT).is_some());
        let (vp, _) = parse_keys("viewport=10 0 90 50", &e);
        assert_eq!(clip_limitation(&vp, false, 100.0 / BP_PER_PT, 50.0 / BP_PER_PT), None, "without clip, graphicx draws the whole image");
    }

    #[test]
    fn graphicspath_is_read_from_the_source() {
        assert_eq!(search_dirs(&["\\graphicspath{{figs/}{images/png/}}\n"]), vec!["figs/", "images/png/"]);
        // A later one replaces an earlier one, and a commented-out one is not
        // one at all.
        assert_eq!(search_dirs(&["\\graphicspath{{a/}}\n% \\graphicspath{{b/}}\n\\graphicspath{{c/}}\n"]), vec!["c/"]);
        assert!(search_dirs(&["no graphics path here\n"]).is_empty());
        // `\%` is an escaped character, not the start of a comment.
        assert_eq!(search_dirs(&["100\\% \\graphicspath{{d/}}\n"]), vec!["d/"]);
    }
}
