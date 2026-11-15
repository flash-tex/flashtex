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
    /// PDF: xdvipdfmx's form `/Matrix` (`set_transform_matrix`, from
    /// `/Rotate`) and the box `pdf_ximage_scale_image` fits
    /// (`I->attr.bbox`, [`pic::dpx_form_bounds`]). Raster images:
    /// the identity and `bbox`.
    pub matrix: [f64; 6],
    pub attr_bbox: [f64; 4],
    pub info: Json,
}

#[derive(Clone, Default)]
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

    /// Whether every image's key is still its file's: a document carried
    /// over from another run (a converged run's, `crate::conv`) whose image
    /// file changed since (the same XDV, a picture replaced in place by one
    /// of the same size) is not this run's (#2444).
    pub fn fresh(&self) -> bool {
        self.by.iter().all(|((path, page, pagebox), &id)| {
            self.get(id)
                .is_some_and(|i| i.key == image_key(path, *page, *pagebox))
        })
    }

    /// The image of file `path` (found already), PDF page `page` (0: the
    /// first) and box `pagebox` (0: none given).
    pub fn load(&mut self, path: &str, page: i32, pagebox: i32) -> Result<u32, String> {
        let page = if page == 0 { 1 } else { page };
        let k = (path.to_string(), page, pagebox);
        // A document carried over from another run (a page checkpoint's,
        // a converged run's, `crate::conv`) holds the images that run
        // loaded: one whose file changed since is loaded again, under its
        // id, so that its key and data are the file's as a fresh run's are
        // (#2444)
        let mut again = None;
        // (the key from the file's identity before its bytes are read: a
        // file written in between is then seen as changed at the next
        // check, not kept under the new identity with the old bytes)
        let key = image_key(path, page, pagebox);
        if let Some(&id) = self.by.get(&k) {
            match self.get(id) {
                Some(i) if i.key == key => return Ok(id),
                _ => again = Some(id),
            }
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
        let id = again.unwrap_or(self.res.len() as u32 + 1);
        let image = if kind == Kind::Pdf {
            // xdvipdfmx's page, box and /Rotate (pdf_doc_get_page), not
            // XeTeX's own (pic::pdf_page sizes the box TeX sees)
            let p = pic::dvipdfmx_page(path, page, pagebox)
                .ok_or_else(|| format!("{path}: cannot read page {page} of the PDF file"))?;
            let [lx, ly, rx, ry] = p.bbox;
            let [a, b, ..] = p.matrix;
            let rotate = if b == -1.0 {
                90
            } else if a == -1.0 {
                180
            } else if b == 1.0 {
                270
            } else {
                0
            };
            let info = Json::Obj(vec![
                ("id".into(), Json::Int(id as i64)),
                ("key".into(), Json::Str(hex(&key))),
                ("type".into(), Json::Str("pdf".into())),
                ("file".into(), Json::Str(path.into())),
                ("width".into(), Json::Num(rx - lx)),
                ("height".into(), Json::Num(ry - ly)),
                ("rotate".into(), Json::Int(rotate as i64)),
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
                rotate,
                matrix: p.matrix,
                attr_bbox: p.attr_bbox,
                info,
            }
        } else {
            let s = size.ok_or_else(|| format!("{path}: cannot read the image's size"))?;
            let (w, h) = (s.width * 72.0 / s.xdpi, s.height * 72.0 / s.ydpi);
            let mut kv = vec![
                ("id".into(), Json::Int(id as i64)),
                ("key".into(), Json::Str(hex(&key))),
                ("type".into(), Json::Str(kind.name().into())),
                ("file".into(), Json::Str(path.into())),
                ("width".into(), Json::Int(s.width as i64)),
                ("height".into(), Json::Int(s.height as i64)),
                ("rotate".into(), Json::Int(0)),
                ("x_res".into(), Json::Num(s.xdpi)),
                ("y_res".into(), Json::Num(s.ydpi)),
            ];
            // The space the PDF writer gives the samples (dvipdfmx's
            // CalGray/CalRGB/ICCBased, not pdfTeX's Device spaces): the
            // preview draws them in it (spec §5.2 `color_space`).
            if let Some(cs) = super::pdf::image_color_space(kind, &data) {
                kv.push(("color_space".into(), cs));
            }
            let info = Json::Obj(kv);
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
                matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                attr_bbox: [0.0, 0.0, w, h],
                info,
            }
        };
        match again {
            Some(id) => self.res[id as usize - 1] = image,
            None => self.res.push(image),
        }
        self.by.insert(k, id);
        Ok(id)
    }
}

/// The display list's key of an image (spec §5.2): it changes when the
/// file's path or identity, or the selection, changes. The identity is the
/// file's size, modification and status-change times, inode and device: a
/// file rewritten in place with bytes of the same size and its old
/// modification time put back (`cp -p`, an archive unpacked over it) has a
/// new status-change time, which no user call can set (#2444, the #2296
/// ruling); one renamed over it (an editor's atomic save) has a new inode.
fn image_key(path: &str, page: i32, pagebox: i32) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"display-list-v3 image\0");
    h.update(path.as_bytes());
    h.update(&[0]);
    match std::fs::metadata(path) {
        Ok(m) => {
            let s = flashtex_engine::os::file_stat(&m);
            h.update(&[1]);
            h.update(&s.size.to_le_bytes());
            h.update(&s.mtime_ns.to_le_bytes());
            h.update(&s.ctime_ns.to_le_bytes());
            h.update(&s.ino.to_le_bytes());
            h.update(&s.dev.to_le_bytes());
        }
        Err(_) => h.update(&[0]),
    }
    h.update(&page.to_le_bytes());
    h.update(&pagebox.to_le_bytes());
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    /// A PNG's signature, its `IHDR` (`w` × 1, grey) and an empty `IDAT`:
    /// enough for its size; the same length for every `w`.
    fn png(w: u32) -> Vec<u8> {
        let mut d = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        d.extend_from_slice(&13u32.to_be_bytes());
        d.extend_from_slice(b"IHDR");
        d.extend_from_slice(&w.to_be_bytes());
        d.extend_from_slice(&1u32.to_be_bytes());
        d.extend_from_slice(&[8, 0, 0, 0, 0]);
        d.extend_from_slice(&[0; 4]);
        d.extend_from_slice(&0u32.to_be_bytes());
        d.extend_from_slice(b"IDAT");
        d.extend_from_slice(&[0; 4]);
        d
    }

    fn width(i: &Images, id: u32) -> Option<i64> {
        match &i.get(id)?.info {
            Json::Obj(kv) => kv.iter().find(|(k, _)| k == "width").and_then(|(_, v)| match v {
                Json::Int(n) => Some(*n),
                _ => None,
            }),
            _ => None,
        }
    }

    /// A picture rewritten in place with bytes of the same size and its old
    /// modification time put back (`cp -p`, an archive unpacked over it):
    /// its path, size, modification time and inode are all the same, and
    /// only the status-change time says it changed. The images loaded
    /// before (a carried-over document's) are no longer fresh, and the
    /// picture is loaded again under its id with a new key (#2444, the
    /// #2296 ruling).
    #[test]
    fn a_picture_rewritten_with_its_size_and_modification_time_is_seen() {
        let dir = std::env::temp_dir().join(format!("flashtex-images-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("p.png");
        let path = f.to_str().unwrap();
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let write = |w: u32| {
            std::fs::write(&f, png(w)).unwrap();
            std::fs::File::options()
                .write(true)
                .open(&f)
                .unwrap()
                .set_modified(t)
                .unwrap();
        };
        write(1);
        let mut imgs = Images::default();
        let id = imgs.load(path, 0, 0).unwrap();
        assert_eq!(width(&imgs, id), Some(1));
        let key = imgs.get(id).unwrap().key;
        let before = flashtex_engine::os::file_stat(&std::fs::metadata(&f).unwrap());
        assert!(imgs.fresh());
        // (past the file system's timestamp granularity: Linux takes the
        // status-change time from a coarse clock)
        std::thread::sleep(Duration::from_millis(50));
        write(2);
        let after = flashtex_engine::os::file_stat(&std::fs::metadata(&f).unwrap());
        // the case: nothing but the status-change time differs
        assert_eq!(
            (before.size, before.mtime_ns, before.ino, before.dev),
            (after.size, after.mtime_ns, after.ino, after.dev)
        );
        assert_ne!(before.ctime_ns, after.ctime_ns);
        assert!(!imgs.fresh(), "the rewritten picture's entry is not fresh");
        assert_eq!(imgs.load(path, 0, 0).unwrap(), id, "loaded again under its id");
        assert_eq!(width(&imgs, id), Some(2), "the new picture's data");
        assert_ne!(imgs.get(id).unwrap().key, key, "a new key");
        assert!(imgs.fresh());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
