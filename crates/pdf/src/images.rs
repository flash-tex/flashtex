//! Image XObjects for the exact route (`display-list-v2-images`).
//!
//! Contract: `protocol/proposals/display-list-v2-image.md` §3 and §5.4.
//! Bytes are read under the project root through [`read_rooted`] (every
//! component below the root must be a real directory or file, never a
//! symbolic link), and must match the item's `byte_length` and SHA-256
//! before anything is decoded.
//!
//! - PNG and JPEG become image XObjects ([`from_png`], [`from_jpeg`]) in the
//!   shape pdfTeX 1.40.29 writes (see `crate::raster`).
//! - A PDF page becomes a Form XObject ([`from_pdf_page`]): `/BBox` is the
//!   page's CropBox clipped to its MediaBox (graphicx's default
//!   `pagebox=cropbox`, `pdftex.def`: `\def\Gin@pagebox{cropbox}`), the page
//!   content stream is copied with its filters, and the page's `/Resources`
//!   and `/Group` are copied with every reachable object renumbered. A
//!   `/Rotate` of 90, 180 or 270 becomes the form's `/Matrix`, as pdfTeX
//!   does (`/Rotate 90` on a `[0 0 200 120]` page: `/Matrix [0 -1 1 0 0 200]`).
//!   pdfTeX's `/PTEX.FileName`, `/PTEX.PageNumber` and `/PTEX.InfoDict`
//!   keys are not written: the first would put an absolute local path into
//!   the export.
//!
//! Placement ([`ImageXObject::placement`]) takes the display list's unit
//! square transform already converted to PDF's y-up page space and returns
//! `q … cm /ImN Do Q`, the operator shape pdfTeX writes.

use crate::deflate;
use crate::exact::{Decimal, Op};
use crate::raster::{self, Space};
use crate::reader::{Obj, PdfFile};
use crate::sha256;
use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write as _;
use std::path::{Component, Path};

/// Largest image file accepted, in bytes.
pub const MAX_IMAGE_FILE_BYTES: u64 = 256 * 1024 * 1024;
/// Most objects imported from one PDF page.
pub const MAX_IMPORTED_OBJECTS: usize = 100_000;

/// Part of a serialised object: literal ASCII or a reference to another
/// object of the same [`ImageXObject`] (index into `objects`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    Text(String),
    Ref(usize),
}

/// One object of an image resource. For a stream, `dict` holds the
/// dictionary entries without `<<`, `>>` and `/Length` (the writer adds
/// them); otherwise it is the whole object body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalObject {
    pub dict: Vec<Piece>,
    pub stream: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Geometry {
    /// An image XObject: its unit square is the image.
    Raster { width: u32, height: u32 },
    /// A form XObject over `bbox` (verbatim tokens) with `/Rotate`.
    Form { bbox: [String; 4], rotate: u16 },
}

/// A self-contained image resource: `objects[0]` is the XObject; the rest
/// are its mask, palette or imported objects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageXObject {
    pub objects: Vec<LocalObject>,
    pub geometry: Geometry,
    /// An `/SMask` is present: pdfTeX gives the page a transparency group.
    pub needs_page_group: bool,
    /// One line for reports: format, size, colour space, filters.
    pub summary: String,
}

/// The page group pdfTeX writes on a page that paints a soft-masked image.
pub const PAGE_TRANSPARENCY_GROUP: &str =
    "<< /Type /Group /S /Transparency /CS /DeviceRGB /I true >>";

fn text(s: impl Into<String>) -> Piece {
    Piece::Text(s.into())
}

/// Serialises pieces with `base` added to every local reference.
pub fn resolve_pieces(pieces: &[Piece], base: usize) -> String {
    let mut out = String::new();
    for p in pieces {
        match p {
            Piece::Text(t) => out.push_str(t),
            Piece::Ref(i) => {
                let _ = write!(out, "{} 0 R", base + i);
            }
        }
    }
    out
}

/// Reads `relative` under `root`, refusing absolute paths, `.`/`..`
/// components, symbolic links at any component below the root, and
/// anything but a regular file at the end. The opened file is checked to be
/// the one that was inspected (device and inode), so a component swapped
/// for a link between the check and the open is refused too.
pub fn read_rooted(root: &Path, relative: &str, expected_len: u64) -> Result<Vec<u8>, String> {
    if !root.is_absolute() {
        return Err(format!("project root {} is not absolute", root.display()));
    }
    if relative.is_empty() || relative.contains('\0') || relative.contains('\\') {
        return Err(format!(
            "image path {relative:?} is not a project-relative path"
        ));
    }
    let rel = Path::new(relative);
    let mut current = root.to_path_buf();
    let components: Vec<Component> = rel.components().collect();
    if components.is_empty() {
        return Err(format!("image path {relative:?} is empty"));
    }
    for (i, c) in components.iter().enumerate() {
        let Component::Normal(name) = c else {
            return Err(format!(
                "image path {relative:?} must stay under the project root (no absolute, '.' or '..' components)"
            ));
        };
        current.push(name);
        let meta = std::fs::symlink_metadata(&current)
            .map_err(|e| format!("image {relative:?}: {}: {e}", current.display()))?;
        if meta.file_type().is_symlink() {
            return Err(format!(
                "image {relative:?}: {} is a symbolic link; images are read without following links",
                current.display()
            ));
        }
        let last = i + 1 == components.len();
        if !last && !meta.is_dir() {
            return Err(format!(
                "image {relative:?}: {} is not a directory",
                current.display()
            ));
        }
        if last {
            if !meta.is_file() {
                return Err(format!(
                    "image {relative:?}: {} is not a regular file",
                    current.display()
                ));
            }
            if meta.len() > MAX_IMAGE_FILE_BYTES {
                return Err(format!("image {relative:?}: larger than 256 MiB"));
            }
            if meta.len() != expected_len {
                return Err(format!(
                    "image {relative:?} is stale: {} bytes on disk, the display list says {expected_len}",
                    meta.len()
                ));
            }
            let mut file =
                std::fs::File::open(&current).map_err(|e| format!("image {relative:?}: {e}"))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let opened = file
                    .metadata()
                    .map_err(|e| format!("image {relative:?}: {e}"))?;
                if opened.dev() != meta.dev() || opened.ino() != meta.ino() {
                    return Err(format!(
                        "image {relative:?} changed while it was being opened"
                    ));
                }
            }
            let mut bytes = Vec::with_capacity(expected_len as usize);
            use std::io::Read;
            file.by_ref()
                .take(MAX_IMAGE_FILE_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| format!("image {relative:?}: {e}"))?;
            if bytes.len() as u64 != expected_len {
                return Err(format!(
                    "image {relative:?} is stale: read {} bytes, the display list says {expected_len}",
                    bytes.len()
                ));
            }
            return Ok(bytes);
        }
    }
    unreachable!("the loop returns at the last component")
}

/// [`read_rooted`] plus the SHA-256 check.
pub fn read_verified(
    root: &Path,
    relative: &str,
    expected_len: u64,
    sha: &str,
) -> Result<Vec<u8>, String> {
    let bytes = read_rooted(root, relative, expected_len)?;
    let got = sha256::hex(&bytes);
    if got != sha {
        return Err(format!(
            "image {relative:?} is stale: its bytes hash to {got}, the display list says {sha}"
        ));
    }
    Ok(bytes)
}

fn image_dict(width: u32, height: u32, bits: u8, space: &str) -> String {
    format!(
        "/Type /XObject /Subtype /Image /Width {width} /Height {height} /BitsPerComponent {bits} /ColorSpace {space}"
    )
}

/// A PNG as an image XObject (plus `/SMask` and palette objects).
pub fn from_png(bytes: &[u8]) -> Result<ImageXObject, String> {
    let png = raster::decode_png(bytes)?;
    let mut objects = vec![LocalObject {
        dict: vec![],
        stream: None,
    }];
    let mut dict = Vec::new();
    let space_desc;
    match &png.space {
        Space::Device(d) => {
            dict.push(text(image_dict(
                png.width,
                png.height,
                png.bits,
                &format!("/{}", d.name()),
            )));
            space_desc = d.name().to_string();
        }
        Space::IndexedRgb { lookup } => {
            let hival = lookup.len() / 3 - 1;
            dict.push(text(image_dict(png.width, png.height, png.bits, "")));
            dict.push(text(format!("[ /Indexed /DeviceRGB {hival} ")));
            dict.push(Piece::Ref(objects.len()));
            dict.push(text(" ]"));
            objects.push(LocalObject {
                dict: vec![text("/Filter /FlateDecode")],
                stream: Some(deflate::zlib_compress(lookup)),
            });
            space_desc = format!("Indexed/DeviceRGB {} entries", hival + 1);
        }
    }
    if let Some(alpha) = &png.alpha {
        dict.push(text(" /SMask "));
        dict.push(Piece::Ref(objects.len()));
        objects.push(LocalObject {
            dict: vec![
                text(image_dict(png.width, png.height, 8, "/DeviceGray")),
                text(" /Filter /FlateDecode"),
            ],
            stream: Some(deflate::zlib_compress(alpha)),
        });
    }
    dict.push(text(" /Filter /FlateDecode"));
    objects[0] = LocalObject {
        dict,
        stream: Some(deflate::zlib_compress(&png.samples)),
    };
    Ok(ImageXObject {
        summary: format!(
            "png {}x{} {}-bit {space_desc}{}, FlateDecode",
            png.width,
            png.height,
            png.bits,
            if png.alpha.is_some() { " + SMask" } else { "" }
        ),
        needs_page_group: png.alpha_channel,
        geometry: Geometry::Raster {
            width: png.width,
            height: png.height,
        },
        objects,
    })
}

/// A JPEG as a `/DCTDecode` image XObject (bytes unchanged).
pub fn from_jpeg(bytes: &[u8]) -> Result<ImageXObject, String> {
    let j = raster::parse_jpeg(bytes)?;
    let mut dict = image_dict(j.width, j.height, j.bits, &format!("/{}", j.space.name()));
    if j.invert_cmyk {
        dict.push_str(" /Decode [ 1 0 1 0 1 0 1 0 ]");
    }
    dict.push_str(" /Filter /DCTDecode");
    Ok(ImageXObject {
        summary: format!(
            "jpeg {}x{} {}{}{}, DCTDecode passthrough",
            j.width,
            j.height,
            j.space.name(),
            if j.progressive { " progressive" } else { "" },
            if j.invert_cmyk {
                " (Adobe, inverted Decode)"
            } else {
                ""
            }
        ),
        objects: vec![LocalObject {
            dict: vec![text(dict)],
            stream: Some(bytes.to_vec()),
        }],
        geometry: Geometry::Raster {
            width: j.width,
            height: j.height,
        },
        needs_page_group: false,
    })
}

fn escape_name(name: &str) -> String {
    let mut out = String::from("/");
    for b in name.bytes() {
        let regular = (0x21..=0x7e).contains(&b)
            && !matches!(
                b,
                b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%' | b'#'
            );
        if regular {
            out.push(b as char);
        } else {
            let _ = write!(out, "#{b:02X}");
        }
    }
    out
}

fn number(o: &Obj) -> Option<(f64, String)> {
    let t = o.as_number()?;
    let v: f64 = t.parse().ok()?;
    v.is_finite().then(|| (v, t.to_string()))
}

struct Importer<'a> {
    file: &'a PdfFile,
    map: BTreeMap<u32, usize>,
    objects: Vec<LocalObject>,
    queue: VecDeque<(u32, usize)>,
}

impl Importer<'_> {
    /// Allocates a local slot for object `n` (or reuses it).
    fn reference(&mut self, n: u32) -> Result<Piece, String> {
        if let Some(&i) = self.map.get(&n) {
            return Ok(Piece::Ref(i));
        }
        let target = self.file.objects.get(&n);
        // Never follow a link back into the page tree: that would copy
        // every page of the source document.
        if let Some(d) = target.and_then(Obj::as_dict)
            && matches!(d.get("Type").and_then(Obj::as_name), Some("Page" | "Pages"))
        {
            return Ok(text("null"));
        }
        if target.is_none() {
            return Ok(text("null"));
        }
        if self.objects.len() >= MAX_IMPORTED_OBJECTS {
            return Err(format!(
                "PDF page references more than {MAX_IMPORTED_OBJECTS} objects"
            ));
        }
        let i = self.objects.len();
        self.objects.push(LocalObject {
            dict: vec![],
            stream: None,
        });
        self.map.insert(n, i);
        self.queue.push_back((n, i));
        Ok(Piece::Ref(i))
    }

    fn value(&mut self, o: &Obj, out: &mut Vec<Piece>, depth: usize) -> Result<(), String> {
        if depth > 64 {
            return Err("PDF object nesting deeper than 64".into());
        }
        let push = |out: &mut Vec<Piece>, s: &str| match out.last_mut() {
            Some(Piece::Text(t)) => t.push_str(s),
            _ => out.push(text(s)),
        };
        match o {
            Obj::Null => push(out, "null"),
            Obj::Bool(b) => push(out, if *b { "true" } else { "false" }),
            Obj::Number(n) => push(out, n),
            Obj::String(s) => {
                let mut h = String::from("<");
                for b in s {
                    let _ = write!(h, "{b:02X}");
                }
                h.push('>');
                push(out, &h);
            }
            Obj::Name(n) => push(out, &escape_name(n)),
            Obj::Array(a) => {
                push(out, "[");
                for x in a {
                    push(out, " ");
                    self.value(x, out, depth + 1)?;
                }
                push(out, " ]");
            }
            Obj::Dict(d) => {
                push(out, "<<");
                for (k, v) in d {
                    push(out, " ");
                    push(out, &escape_name(k));
                    push(out, " ");
                    self.value(v, out, depth + 1)?;
                }
                push(out, " >>");
            }
            Obj::Ref(n, _) => {
                let p = self.reference(*n)?;
                match p {
                    Piece::Text(t) => push(out, &t),
                    r => out.push(r),
                }
            }
            Obj::Stream { .. } => return Err("a stream cannot be a direct object".into()),
        }
        Ok(())
    }

    /// Stream dictionary entries except `/Length`.
    fn stream_entries(
        &mut self,
        dict: &BTreeMap<String, Obj>,
        skip: &[&str],
    ) -> Result<Vec<Piece>, String> {
        let mut out = Vec::new();
        for (k, v) in dict {
            if k == "Length" || skip.contains(&k.as_str()) {
                continue;
            }
            if !out.is_empty() {
                out.push(text(" "));
            }
            out.push(text(format!("{} ", escape_name(k))));
            self.value(v, &mut out, 1)?;
        }
        Ok(out)
    }

    fn drain(&mut self) -> Result<(), String> {
        while let Some((n, i)) = self.queue.pop_front() {
            let obj = self.file.objects[&n].clone();
            self.objects[i] = match &obj {
                Obj::Stream { dict, raw } => LocalObject {
                    dict: self.stream_entries(dict, &[])?,
                    stream: Some(raw.clone()),
                },
                other => {
                    let mut body = Vec::new();
                    self.value(other, &mut body, 0)?;
                    LocalObject {
                        dict: body,
                        stream: None,
                    }
                }
            };
        }
        Ok(())
    }
}

/// Page `page` (1-based) of a PDF as a Form XObject.
pub fn from_pdf_page(bytes: &[u8], page: u32) -> Result<ImageXObject, String> {
    let file = PdfFile::parse(bytes).map_err(|e| format!("PDF: {e}"))?;
    if file.trailer.contains_key("Encrypt") {
        return Err("PDF: encrypted documents are not imported".into());
    }
    let pages = file.pages().map_err(|e| format!("PDF: {e}"))?;
    let count = pages.len();
    let p = *pages
        .get((page as usize).wrapping_sub(1))
        .ok_or_else(|| format!("PDF: page {page} requested, the document has {count}"))?;
    let read_box = |key: &str| -> Option<[(f64, String); 4]> {
        let a = file.page_attr(p, key)?.as_array()?;
        if a.len() != 4 {
            return None;
        }
        let v: Vec<(f64, String)> = a
            .iter()
            .map(|x| number(file.resolve(x)))
            .collect::<Option<_>>()?;
        let (x0, x1) = if v[0].0 <= v[2].0 {
            (v[0].clone(), v[2].clone())
        } else {
            (v[2].clone(), v[0].clone())
        };
        let (y0, y1) = if v[1].0 <= v[3].0 {
            (v[1].clone(), v[3].clone())
        } else {
            (v[3].clone(), v[1].clone())
        };
        Some([x0, y0, x1, y1])
    };
    let media = read_box("MediaBox").ok_or("PDF: page has no readable /MediaBox")?;
    let bbox = match read_box("CropBox") {
        None => media.clone(),
        Some(c) => {
            let hi = |a: &(f64, String), b: &(f64, String)| {
                if a.0 >= b.0 { a.clone() } else { b.clone() }
            };
            let lo = |a: &(f64, String), b: &(f64, String)| {
                if a.0 <= b.0 { a.clone() } else { b.clone() }
            };
            [
                hi(&c[0], &media[0]),
                hi(&c[1], &media[1]),
                lo(&c[2], &media[2]),
                lo(&c[3], &media[3]),
            ]
        }
    };
    if bbox[2].0 <= bbox[0].0 || bbox[3].0 <= bbox[1].0 {
        return Err("PDF: the page's CropBox and MediaBox do not overlap".into());
    }
    let rotate = match file.page_attr(p, "Rotate").map(|o| file.resolve(o)) {
        None => 0,
        Some(o) => {
            let r: i64 = o
                .as_number()
                .and_then(|t| t.parse().ok())
                .ok_or("PDF: /Rotate is not an integer")?;
            if r % 90 != 0 {
                return Err(format!("PDF: /Rotate {r} is not a multiple of 90"));
            }
            r.rem_euclid(360) as u16
        }
    };

    let mut imp = Importer {
        file: &file,
        map: BTreeMap::new(),
        objects: vec![LocalObject {
            dict: vec![],
            stream: None,
        }],
        queue: VecDeque::new(),
    };
    let mut dict = vec![text("/Type /XObject /Subtype /Form /FormType 1")];
    let [llx, lly, urx, ury] = [&bbox[0].1, &bbox[1].1, &bbox[2].1, &bbox[3].1];
    let matrix = match rotate {
        90 => Some(format!("0 -1 1 0 {} {urx}", neg(lly))),
        180 => Some(format!("-1 0 0 -1 {urx} {ury}")),
        270 => Some(format!("0 1 -1 0 {ury} {}", neg(llx))),
        _ => None,
    };
    if let Some(m) = matrix {
        dict.push(text(format!(" /Matrix [ {m} ]")));
    }
    dict.push(text(format!(
        " /BBox [ {llx} {lly} {urx} {ury} ] /Resources "
    )));
    match file.page_attr(p, "Resources") {
        Some(r) => imp.value(r, &mut dict, 1)?,
        None => dict.push(text("<< >>")),
    }
    if let Some(g) = p.get("Group") {
        dict.push(text(" /Group "));
        imp.value(g, &mut dict, 1)?;
    }
    let content = match p.get("Contents").map(|c| (c, file.resolve(c))) {
        None => Vec::new(),
        Some((_, Obj::Stream { dict: sd, raw })) => {
            let entries =
                imp.stream_entries(sd, &["Type", "Subtype", "BBox", "Resources", "Matrix"])?;
            if !entries.is_empty() {
                dict.push(text(" "));
                dict.extend(entries);
            }
            raw.clone()
        }
        Some((_, Obj::Array(parts))) => {
            let mut joined = Vec::new();
            for (i, part) in parts.iter().enumerate() {
                if i > 0 {
                    joined.push(b'\n');
                }
                joined.extend(
                    file.decode_stream(part)
                        .map_err(|e| format!("PDF: content stream: {e}"))?,
                );
            }
            dict.push(text(" /Filter /FlateDecode"));
            deflate::zlib_compress(&joined)
        }
        Some(_) => return Err("PDF: page /Contents is neither a stream nor an array".into()),
    };
    imp.objects[0] = LocalObject {
        dict,
        stream: Some(content),
    };
    imp.drain()?;
    let imported = imp.objects.len() - 1;
    Ok(ImageXObject {
        summary: format!(
            "pdf page {page} of {count}, BBox [{llx} {lly} {urx} {ury}]{}, {imported} object(s) imported",
            if rotate != 0 {
                format!(", /Rotate {rotate}")
            } else {
                String::new()
            }
        ),
        objects: imp.objects,
        geometry: Geometry::Form {
            bbox: [
                bbox[0].1.clone(),
                bbox[1].1.clone(),
                bbox[2].1.clone(),
                bbox[3].1.clone(),
            ],
            rotate,
        },
        needs_page_group: false,
    })
}

/// Negates a PDF number token without going through `f64`.
fn neg(t: &str) -> String {
    match t.strip_prefix('-') {
        Some(rest) => rest.to_string(),
        None => format!("-{}", t.strip_prefix('+').unwrap_or(t)),
    }
}

/// `v` as a decimal token with at most `digits` fractional digits.
pub fn decimal_rounded(v: f64, digits: usize) -> Result<Decimal, String> {
    if !v.is_finite() {
        return Err(format!("{v} is not finite"));
    }
    let mut s = format!("{v:.digits$}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".into();
    }
    Decimal::new(&s).map_err(|e| e.to_string())
}

impl ImageXObject {
    /// `q unit cm [form normalisation] /name Do Q`, where `unit` maps the
    /// image's unit square to PDF page space (y up).
    pub fn placement(&self, name: &str, unit: [Decimal; 6]) -> Result<Vec<Op>, String> {
        let mut ops = vec![Op::Save, Op::Concat(unit)];
        if let Geometry::Form { bbox, rotate } = &self.geometry {
            let v: Vec<f64> = bbox
                .iter()
                .map(|t| t.parse::<f64>().map_err(|_| format!("bad BBox token {t}")))
                .collect::<Result<_, _>>()?;
            let (w, h) = (v[2] - v[0], v[3] - v[1]);
            let (dw, dh) = if rotate % 180 == 90 { (h, w) } else { (w, h) };
            let zero = Decimal::from_i64(0);
            let one = Decimal::from_i64(1);
            ops.push(Op::Concat([
                decimal_rounded(1.0 / dw, 12)?,
                zero.clone(),
                zero.clone(),
                decimal_rounded(1.0 / dh, 12)?,
                zero.clone(),
                zero.clone(),
            ]));
            if *rotate == 0 {
                ops.push(Op::Concat([
                    one.clone(),
                    zero.clone(),
                    zero.clone(),
                    one,
                    Decimal::new(&neg(&bbox[0])).map_err(|e| e.to_string())?,
                    Decimal::new(&neg(&bbox[1])).map_err(|e| e.to_string())?,
                ]));
            }
        }
        ops.push(Op::Do(name.to_string()));
        ops.push(Op::Restore);
        Ok(ops)
    }

    /// Records alternate text for accessibility (`\includegraphics`
    /// `alt=...`): appends `/Alt (...)` to the XObject dictionary, encoded
    /// as a PDF text string (ASCII verbatim, anything else UTF-16BE with a
    /// BOM). Call once per XObject; the entry is serialised with the
    /// dictionary by the exact writer, so the text reaches the written PDF
    /// bytes unchanged.
    pub fn set_alt(&mut self, alt: &str) {
        if let Some(obj) = self.objects.first_mut() {
            obj.dict.push(Piece::Text(format!(
                " /Alt {}",
                crate::navigation::text_string(alt)
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negation_and_rounding_are_textual() {
        assert_eq!(neg("5"), "-5");
        assert_eq!(neg("-10.5"), "10.5");
        assert_eq!(neg("+3"), "-3");
        assert_eq!(
            decimal_rounded(1.0 / 180.0, 12).unwrap().as_str(),
            "0.005555555556"
        );
        assert_eq!(decimal_rounded(-0.0000000000001, 12).unwrap().as_str(), "0");
        assert_eq!(decimal_rounded(2.5, 12).unwrap().as_str(), "2.5");
    }

    #[test]
    fn names_are_escaped() {
        assert_eq!(escape_name("F1"), "/F1");
        assert_eq!(escape_name("A B#(x)"), "/A#20B#23#28x#29");
    }
}
