//! The images of the output (`pdf:image`, and `\XeTeXpicfile` and
//! `\XeTeXpdffile` pictures, which XeTeX writes as `pdf:image`): one
//! display-list `IMAGE` resource per file, page and box (spec §5.2).

use crate::pic;
use flashtex_display_list::json::Json;
use flashtex_display_list::sha256::{hex, Sha256};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Png,
    Jpeg,
    Bmp,
    Pdf,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Png => "png",
            Kind::Jpeg => "jpeg",
            Kind::Bmp => "bmp",
            Kind::Pdf => "pdf",
        }
    }
}

/// An image resource.
#[derive(Clone, Debug)]
pub struct Image {
    pub id: u32,
    pub key: [u8; 32],
    pub kind: Kind,
    pub path: String,
    /// The natural box in bp: a raster image's size at its resolution
    /// (`[0, 0, w, h]`), a PDF page's selected box as the file has it.
    pub bbox: [f64; 4],
    /// Raster images: pixels and resolution (dpi).
    pub pixels: (u32, u32),
    pub res: (f64, f64),
    /// PDF: the page (1-based), the box's name, the page's `/Rotate`.
    pub page: i32,
    pub page_box: &'static str,
    pub rotate: i32,
    pub info: Json,
}

#[derive(Default)]
pub struct Images {
    /// By id - 1.
    pub res: Vec<Image>,
    by: HashMap<(String, i32, i32), u32>,
}

/// dvipdfmx's page box keywords, as XeTeX's `pdfbox_*` numbers.
pub fn pagebox_code(w: &[u8]) -> Option<i32> {
    Some(match w {
        b"cropbox" => 1,
        b"mediabox" => 2,
        b"bleedbox" => 3,
        b"trimbox" => 4,
        b"artbox" => 5,
        _ => return None,
    })
}

fn pagebox_name(code: i32) -> &'static str {
    match code {
        2 => "media",
        3 => "bleed",
        4 => "trim",
        5 => "art",
        _ => "crop",
    }
}

impl Images {
    pub fn get(&self, id: u32) -> Option<&Image> {
        self.res.get(id.checked_sub(1)? as usize)
    }

    /// The image of file `path` (found already), PDF page `page` (0: the
    /// first) and box `pagebox` (0: crop).
    pub fn load(&mut self, path: &str, page: i32, pagebox: i32) -> Result<u32, String> {
        let page = page.max(1);
        let k = (path.to_string(), page, pagebox);
        if let Some(&id) = self.by.get(&k) {
            return Ok(id);
        }
        let data = std::fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?;
        let (kind, size) = if pic::is_png(&data) {
            (Kind::Png, pic::png_size(&data))
        } else if pic::is_jpeg(&data) {
            (Kind::Jpeg, pic::jpeg_size(&data))
        } else if pic::is_bmp(&data) {
            match pic::bmp_size(&data) {
                pic::Bmp::Size(s) => (Kind::Bmp, Some(s)),
                _ => (Kind::Bmp, None),
            }
        } else if data.starts_with(b"%PDF") || data.windows(5).take(1024).any(|w| w == b"%PDF-") {
            (Kind::Pdf, None)
        } else {
            return Err(format!("{path}: not a PNG, JPEG, BMP or PDF file"));
        };
        let id = self.res.len() as u32 + 1;
        let meta = std::fs::metadata(path).ok();
        let mut h = Sha256::new();
        h.update(b"display-list-v3 image\0");
        h.update(path.as_bytes());
        h.update(&[0]);
        h.update(&(data.len() as u64).to_le_bytes());
        if let Some(t) = meta
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        {
            h.update(&t.as_nanos().to_le_bytes());
        }
        h.update(&page.to_le_bytes());
        h.update(&pagebox.to_le_bytes());
        let key = h.finish();
        let image = if kind == Kind::Pdf {
            let p = pic::pdf_page(path, page, pagebox)
                .ok_or_else(|| format!("{path}: cannot read page {page} of the PDF file"))?;
            let [lx, ly, rx, ry] = p.bbox;
            let info = Json::Obj(vec![
                ("id".into(), Json::Int(id as i64)),
                ("key".into(), Json::Str(hex(&key))),
                ("type".into(), Json::Str("pdf".into())),
                ("file".into(), Json::Str(path.into())),
                ("width".into(), Json::Num(rx - lx)),
                ("height".into(), Json::Num(ry - ly)),
                ("rotate".into(), Json::Int(p.rotate as i64)),
                ("page".into(), Json::Int(p.page as i64)),
                ("page_box".into(), Json::Str(pagebox_name(pagebox).into())),
                ("orig_x".into(), Json::Num(lx)),
                ("orig_y".into(), Json::Num(ly)),
            ]);
            Image {
                id,
                key,
                kind,
                path: path.into(),
                bbox: p.bbox,
                pixels: (0, 0),
                res: (72.0, 72.0),
                page: p.page,
                page_box: pagebox_name(pagebox),
                rotate: p.rotate,
                info,
            }
        } else {
            let s = size.ok_or_else(|| format!("{path}: cannot read the image's size"))?;
            let (w, h) = (s.width * 72.0 / s.xdpi, s.height * 72.0 / s.ydpi);
            let info = Json::Obj(vec![
                ("id".into(), Json::Int(id as i64)),
                ("key".into(), Json::Str(hex(&key))),
                ("type".into(), Json::Str(kind.name().into())),
                ("file".into(), Json::Str(path.into())),
                ("width".into(), Json::Int(s.width as i64)),
                ("height".into(), Json::Int(s.height as i64)),
                ("rotate".into(), Json::Int(0)),
                ("x_res".into(), Json::Num(s.xdpi)),
                ("y_res".into(), Json::Num(s.ydpi)),
            ]);
            Image {
                id,
                key,
                kind,
                path: path.into(),
                bbox: [0.0, 0.0, w, h],
                pixels: (s.width as u32, s.height as u32),
                res: (s.xdpi, s.ydpi),
                page: 1,
                page_box: "crop",
                rotate: 0,
                info,
            }
        };
        self.res.push(image);
        self.by.insert(k, id);
        Ok(id)
    }
}
