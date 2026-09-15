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
    let info = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        probe_png(bytes)
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        probe_jpeg(bytes)
    } else if bytes.starts_with(b"%PDF-") {
        probe_pdf(bytes, pdf_page.max(1))
    } else {
        Err("not a PNG, JPEG or PDF file (unrecognised signature)".into())
    }?;
    check_limits(info)
}

/// Largest raster side the exact PDF route embeds (`crates/pdf`
/// `raster::MAX_DIMENSION`).
pub const MAX_IMAGE_PIXELS: u32 = 1 << 16;
/// TeX's `\maxdimen` (16383.99998pt) in big points.
pub const MAX_DIMEN_BP: f64 = 16383.99998 * BP_PER_PT;

/// An image whose header claims more pixels than the PDF writer embeds, or
/// a natural size no TeX dimension holds (`! Dimension too large.` in
/// pdfTeX), is refused here, where the float reports an unreadable image,
/// instead of reaching the display list and failing the whole export.
fn check_limits(info: ImageInfo) -> Result<ImageInfo, String> {
    if let Some((w, h)) = info.pixels {
        if w > MAX_IMAGE_PIXELS || h > MAX_IMAGE_PIXELS {
            return Err(format!(
                "{} is {w}x{h} pixels, over the {MAX_IMAGE_PIXELS}-pixel limit per side",
                info.format.wire_name().to_uppercase()
            ));
        }
    }
    let fits = |v: f64| v.is_finite() && v.abs() <= MAX_DIMEN_BP;
    if !fits(info.width_bp) || !fits(info.height_bp) {
        return Err(format!(
            "Dimension too large: the natural size {}bp x {}bp is over \\maxdimen",
            info.width_bp, info.height_bp
        ));
    }
    Ok(info)
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
    /// `\textwidth`: the whole text block, both columns and `\columnsep`.
    pub text_width: f64,
    /// `\linewidth`/`\columnwidth`/`\hsize` where the key is read: the
    /// column in a two-column document, `\textwidth` inside a `figure*`
    /// (`\@xdblfloat`: `\hsize\textwidth \linewidth\textwidth`).
    pub line_width: f64,
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
        "\\textwidth" => env.text_width,
        "\\linewidth" | "\\columnwidth" | "\\hsize" => env.line_width,
        "\\textheight" | "\\vsize" => env.text_height,
        "\\paperwidth" => env.paper_width,
        "\\paperheight" => env.paper_height,
        _ => return None,
    };
    Some(factor * per)
}

/// The `graphics`/`graphicx` package options that decide whether an image
/// file is read at all.
///
/// LaTeX hands every global class option to each package, and `graphics.sty`
/// declares `draft`, `final` and `demo`, so `\documentclass[draft]{article}`
/// sets `\ifGin@draft` exactly as `\usepackage[draft]{graphicx}` does.
/// `\ProcessOptions` runs the declared options in *declaration* order
/// (`draft` on line 53, `final` on line 54 of `graphics.sty`), so a `final`
/// anywhere -- class options or package options -- always wins over a
/// `draft` anywhere. Measured on TeX Live 2025: `[draft,final]`,
/// `[final,draft]` and `\documentclass[final]` + `\usepackage[draft]` all
/// embed the file.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GraphicsMode {
    /// `\ifGin@draft`: the file is still looked up for its bounding box,
    /// but nothing is embedded -- `\Gin@setfile` sets a framed box of the
    /// requested size with the file name inside. A file that is missing is
    /// a `LaTeX Warning` and gets `pdftex.def`'s fallback bounding box
    /// ([`MISSING_NATURAL_BP`]) instead of the package error it would
    /// otherwise raise.
    pub draft: bool,
    /// `demo`: `\AtBeginDocument` replaces the whole of
    /// `\Ginclude@graphics` with a `\rule`, so no file is looked up at all
    /// and [`demo_box`] gives the size. It beats `draft`, whose branch
    /// lives in `\Gin@setfile`, which is never reached.
    pub demo: bool,
}

/// [`GraphicsMode`] of a document: its class options plus the options of
/// every `\usepackage` of `graphics` or `graphicx`.
pub fn mode(source: &str) -> GraphicsMode {
    let mut lists = vec![crate::adapter::class_options(source).unwrap_or_default()];
    lists.extend(["graphics", "graphicx"].iter().filter_map(|p| crate::adapter::package_options(source, p)));
    let has = |name: &str| lists.iter().any(|l| l.split(',').any(|o| o.trim() == name));
    GraphicsMode { draft: has("draft") && !has("final"), demo: has("demo") }
}

/// The `\rule` `demo` sets in place of a graphic, in TeX points
/// (`graphics.sty`: `\rule{\@ifundefined{Gin@@ewidth}{150pt}{\Gin@@ewidth}}
/// {\@ifundefined{Gin@@eheight}{100pt}{\Gin@@eheight}}`).
pub const DEMO_WIDTH_PT: f64 = 150.0;
pub const DEMO_HEIGHT_PT: f64 = 100.0;

/// The bounding box `pdftex.def` gives a file it cannot find, in big
/// points: `\Gread@pdftex` leaves `\Gin@llx`..`\Gin@ury` at `0 0 72 72`, so
/// the natural size is one inch square and an image asked for by width
/// alone comes out square. Measured: `\includegraphics[draft]{nofile.png}`
/// is 72.26999pt wide and tall.
pub const MISSING_NATURAL_BP: f64 = 72.0;

/// One parsed `key=value` of the optional argument, in order.
#[derive(Debug, Clone, PartialEq)]
pub enum GKey {
    Width(f64),
    Height(f64),
    TotalHeight(f64),
    Scale(f64),
    Angle(f64),
    KeepAspectRatio(bool),
    /// `draft` / `draft=false`: `\ifGin@draft` for this one graphic, which
    /// overrides the package's own setting ([`GraphicsMode::draft`]).
    Draft(bool),
    Page(u32),
    /// Recognised but not honoured (`trim`, `clip`, `viewport`, ...):
    /// reported as a limitation.
    Unsupported(String),
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
            "draft" => Some(GKey::Draft(v.is_none_or(|v| v != "false"))),
            "page" => v.and_then(|v| v.parse().ok()).map(GKey::Page),
            "trim" | "viewport" | "clip" | "bb" | "natwidth" | "natheight" | "origin" | "pagebox" | "decodearray" | "interpolate" => Some(GKey::Unsupported(k.to_string())),
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
pub fn size_box(nat_w: f64, nat_h: f64, keys: &[GKey]) -> GraphicBox {
    place(nat_w, nat_h, keys, false)
}

/// The box `demo` sets: a [`DEMO_WIDTH_PT`] x [`DEMO_HEIGHT_PT`] rule, no
/// file involved.
///
/// `\Ginclude@graphics` is gone, so `\Gin@req@sizes` -- the only thing that
/// ever scales a natural size, applies `keepaspectratio` and honours the
/// first `scale=` -- is never used. All that survives of the keys before
/// the first `angle=`/`scale=` is `\Gin@@ewidth`/`\Gin@@eheight`, which
/// `\Gin@esetsize` fills from `width=`/`height=`/`totalheight=` and which
/// become the rule's dimensions *literally*: `[width=W]` gives a `W` x
/// 100pt rule, not a proportional one, and `[keepaspectratio,width=W,
/// height=H]` gives `W` x `H`. From the first `angle=`/`scale=` onwards the
/// rule is wrapped in `\Gin@erotate`/`\Gscale@@box` exactly as a real
/// graphic is, so those keys behave as usual -- including `keepaspectratio`,
/// which `\Gscale@@box` does honour.
pub fn demo_box(keys: &[GKey]) -> GraphicBox {
    place(DEMO_WIDTH_PT, DEMO_HEIGHT_PT, keys, true)
}

fn place(nat_w: f64, nat_h: f64, keys: &[GKey], demo: bool) -> GraphicBox {
    // Box so far: [a b c d e f] of the unit square, and its extents.
    let mut m = [nat_w, 0.0, 0.0, nat_h, 0.0, 0.0];
    let mut rotated = false;
    let (mut w, mut h, mut th, mut scale, mut iso) = (None, None, None, None, false);
    // `\Gin@esetsize` before the first angle: request the unrotated size.
    let apply_request = |m: &mut [f64; 6], w: Option<f64>, h: Option<f64>, th: Option<f64>, scale: Option<f64>, iso: bool, rotated: bool| {
        let h = h.or(th);
        if demo && !rotated {
            // The rule takes the requested lengths as they stand; there is
            // no natural size to scale, so `scale` and `keepaspectratio`
            // have nowhere to act.
            *m = [w.unwrap_or(nat_w), 0.0, 0.0, h.unwrap_or(nat_h), 0.0, 0.0];
        } else if !rotated {
            let (sx, sy) = match (w, h) {
                (None, None) => {
                    let s = scale.unwrap_or(1.0);
                    (s, s)
                }
                (Some(w), None) => (w / nat_w, w / nat_w),
                (None, Some(h)) => (h / nat_h, h / nat_h),
                (Some(w), Some(h)) => {
                    let (sx, sy) = (w / nat_w, h / nat_h);
                    if iso {
                        let s = sx.min(sy);
                        (s, s)
                    } else {
                        (sx, sy)
                    }
                }
            };
            *m = [nat_w * sx, 0.0, 0.0, nat_h * sy, 0.0, 0.0];
        } else {
            let (bw, bh, bd) = extents(m);
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
            for i in [0, 2, 4] {
                m[i] *= sx;
            }
            for i in [1, 3, 5] {
                m[i] *= sy;
            }
        }
    };
    for k in keys {
        match *k {
            GKey::Width(v) => w = Some(v),
            GKey::Height(v) => h = Some(v),
            GKey::TotalHeight(v) => th = Some(v),
            // A `scale=` sets `\if@tempswa`, which sends every later key
            // -- and every earlier one still pending -- through
            // `\Gscale@@box` around the finished graphic. Under `demo` that
            // finished graphic is the default rule and the factor itself is
            // lost with `\Gin@req@sizes`, so `[scale=2]` alone is a plain
            // 150x100pt rule and `[width=100pt,scale=2]` resizes that rule
            // to 100pt wide (measured: 99.99847pt x 66.66565pt).
            GKey::Scale(_) if demo && !rotated => rotated = true,
            GKey::Scale(v) => scale = Some(v),
            GKey::KeepAspectRatio(v) => iso = v,
            GKey::Draft(_) => {}
            GKey::Angle(deg) => {
                apply_request(&mut m, w, h, th, scale, iso, rotated);
                (w, h, th, scale) = (None, None, None, None);
                rotated = true;
                let (s, c) = deg.to_radians().sin_cos();
                // Exact quarter turns.
                let (s, c) = if (deg / 90.0).fract() == 0.0 { (s.round(), c.round()) } else { (s, c) };
                m = [c * m[0] - s * m[1], s * m[0] + c * m[1], c * m[2] - s * m[3], s * m[2] + c * m[3], c * m[4] - s * m[5], s * m[4] + c * m[5]];
                // \Grot@box: the new box's left edge is the bounding box's.
                let min_x = [0.0, m[0], m[2], m[0] + m[2]].into_iter().fold(f64::INFINITY, f64::min) + m[4];
                m[4] -= min_x;
            }
            GKey::Page(_) | GKey::Unsupported(_) => {}
        }
    }
    apply_request(&mut m, w, h, th, scale, iso, rotated);
    let (width, height, depth) = extents(&m);
    GraphicBox { width, height, depth, matrix: m }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> LengthEnv {
        LengthEnv { text_width: 469.75499, line_width: 469.75499, text_height: 650.43, paper_width: 614.295, paper_height: 794.96999, em: 11.74988, ex: 5.16 }
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

    /// `\usepackage[...]{graphicx}` and the class options, which LaTeX
    /// passes to it. Measured on TeX Live 2025 (`\ProcessOptions` runs the
    /// declared options in declaration order, and `final` is declared after
    /// `draft`, so `final` wins wherever either is written).
    #[test]
    fn package_and_class_options_decide_draft_and_demo() {
        let m = |p: &str| mode(&format!("{p}\n\\begin{{document}}x\\end{{document}}\n"));
        assert_eq!(m("\\documentclass{article}\n\\usepackage{graphicx}"), GraphicsMode { draft: false, demo: false });
        assert_eq!(m("\\documentclass{article}\n\\usepackage[draft]{graphicx}"), GraphicsMode { draft: true, demo: false });
        assert_eq!(m("\\documentclass{article}\n\\usepackage[demo]{graphicx}"), GraphicsMode { draft: false, demo: true });
        assert_eq!(m("\\documentclass{article}\n\\usepackage[demo]{graphics}"), GraphicsMode { draft: false, demo: true });
        // The class options reach every package.
        assert_eq!(m("\\documentclass[draft]{article}\n\\usepackage{graphicx}"), GraphicsMode { draft: true, demo: false });
        assert_eq!(m("\\documentclass[demo]{article}\n\\usepackage{graphicx}"), GraphicsMode { draft: false, demo: true });
        // `final` wins from either list and in either order.
        assert!(!m("\\documentclass[draft]{article}\n\\usepackage[final]{graphicx}").draft);
        assert!(!m("\\documentclass[final]{article}\n\\usepackage[draft]{graphicx}").draft);
        assert!(!m("\\documentclass{article}\n\\usepackage[draft,final]{graphicx}").draft);
        assert!(!m("\\documentclass{article}\n\\usepackage[final,draft]{graphicx}").draft);
        // The class option alone is enough: the mode is only consulted
        // for a graphic, and a graphic means the package was loaded.
        assert_eq!(m("\\documentclass[draft]{article}"), GraphicsMode { draft: true, demo: false });
    }

    /// `draft` is a boolean key, not a limitation: it changes no dimension,
    /// so the box is the one the same keys give a real file.
    #[test]
    fn draft_is_a_key_and_changes_no_dimension() {
        let e = env();
        let (k, p) = parse_keys("draft,width=100pt", &e);
        assert!(p.is_empty());
        assert!(!k.iter().any(|k| matches!(k, GKey::Unsupported(_))), "{k:?}");
        assert_eq!(k.first(), Some(&GKey::Draft(true)));
        assert_eq!(parse_keys("draft=false", &e).0, vec![GKey::Draft(false)]);
        // Measured: \includegraphics[draft,width=100pt]{nofile.png} is
        // 100pt x 100.00531pt -- `pdftex.def`'s 1 in square, scaled.
        let nat = MISSING_NATURAL_BP / BP_PER_PT;
        let b = size_box(nat, nat, &k);
        assert!((b.width - 100.0).abs() < 1e-6 && (b.height - 100.0).abs() < 0.01 && b.depth == 0.0, "{b:?}");
        // and with no keys at all, one inch square.
        let b = size_box(nat, nat, &parse_keys("draft", &e).0);
        assert!((b.width - 72.26999).abs() < 1e-4 && (b.height - 72.26999).abs() < 1e-4, "{b:?}");
    }

    /// Every row measured with pdflatex (TeX Live 2025) under
    /// `\usepackage[demo]{graphicx}`; `\linewidth` there was 345pt.
    #[test]
    fn demo_boxes_match_pdflatex() {
        let e = LengthEnv { text_width: 345.0, line_width: 345.0, ..env() };
        let case = |opts: &str| {
            let (k, p) = parse_keys(opts, &e);
            assert!(p.is_empty(), "{opts}: {p:?}");
            let b = demo_box(&k);
            (b.width, b.height, b.depth)
        };
        let near = |got: (f64, f64, f64), want: (f64, f64), opts: &str| {
            assert!((got.0 - want.0).abs() < 0.01 && (got.1 - want.1).abs() < 0.01 && got.2 == 0.0, "{opts}: {got:?} != {want:?}");
        };
        for (opts, want) in [
            ("", (150.0, 100.0)),
            // The height stays at 100pt however wide the rule is asked to be.
            ("width=100pt", (100.0, 100.0)),
            ("height=40pt", (150.0, 40.0)),
            ("totalheight=40pt", (150.0, 40.0)),
            ("width=100pt,height=40pt", (100.0, 40.0)),
            // `\Gin@req@sizes` is never used, so `keepaspectratio` and a
            // leading `scale` have nothing to act on.
            ("width=100pt,height=40pt,keepaspectratio", (100.0, 40.0)),
            ("keepaspectratio,width=100pt,height=40pt", (100.0, 40.0)),
            ("scale=2", (150.0, 100.0)),
            ("scale=0.5", (150.0, 100.0)),
            // ... but a `scale` makes every width/height a `\Gscale@@box`
            // around the default rule, in either order.
            ("scale=2,width=100pt", (99.99847, 66.66565)),
            ("width=100pt,scale=2", (99.99847, 66.66565)),
            ("width=0.5\\linewidth", (172.5, 100.0)),
            // From the first angle the rule rotates and rescales as usual.
            ("angle=90", (100.0, 150.0)),
            ("angle=90,width=100pt", (100.0, 150.0)),
            ("width=100pt,angle=90", (100.0, 100.0)),
            ("angle=30", (179.90036, 161.59897)),
            ("angle=90,height=40pt", (26.66626, 39.99939)),
            ("angle=45,width=100pt,height=40pt", (100.00398, 39.99889)),
            ("angle=90,scale=2", (200.0, 300.0)),
            ("width=100pt,angle=90,height=40pt", (39.99939, 39.99939)),
            ("scale=2,width=100pt,angle=90", (66.66565, 99.99847)),
            // `\Gscale@@box` does honour `keepaspectratio`.
            ("angle=45,keepaspectratio,width=100pt,height=40pt", (39.99889, 39.99889)),
            // `demo` beats `draft`: `\Gin@setfile` is never reached.
            ("draft", (150.0, 100.0)),
            ("draft,width=100pt", (100.0, 100.0)),
        ] {
            near(case(opts), want, opts);
        }
    }
}
