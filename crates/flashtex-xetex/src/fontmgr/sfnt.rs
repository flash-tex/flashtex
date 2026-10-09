//! The sfnt data XeTeX's font manager reads through FreeType and HarfBuzz,
//! read here from the font file's bytes with the same rules, so the lookup
//! needs neither library (font loading and shaping do; they are not here).
//!
//! * The table directory as FreeType 2.14.1 validates it
//!   (`tt_face_load_font_dir`, `tt_face_lookup_table`).
//! * The `name` records as FreeType keeps them (`tt_face_load_name`) and
//!   hands them out (`FT_Get_Sfnt_Name`), and the PostScript name as
//!   `FT_Get_Postscript_Name` derives it (`sfnt_get_ps_name`).
//! * `OS/2`, `head` and `post` as `FT_Get_Sfnt_Table` returns them, for
//!   `XeTeXFontMgr::getOpSizeRecAndStyleFlags`.
//! * The GPOS `size` feature as HarfBuzz 12.3.2's
//!   `hb_ot_layout_get_size_params` reports it after its sanitizer.
//!
//! Everything reads through [`Bytes`], a file or a slice, a few bytes at a
//! time: a lookup touches only the faces it adds to its maps, and a CJK
//! collection's GPOS can be megabytes.

use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

/// Random access to a font file's bytes.
pub trait Bytes {
    /// The file's size.
    fn size(&self) -> u64;
    /// `len` bytes at `off`, or `None` when they are not all there.
    fn read(&self, off: u64, len: usize) -> Option<Vec<u8>>;
}

impl Bytes for Vec<u8> {
    fn size(&self) -> u64 {
        self.len() as u64
    }
    fn read(&self, off: u64, len: usize) -> Option<Vec<u8>> {
        let start = usize::try_from(off).ok()?;
        self.get(start..start.checked_add(len)?).map(<[u8]>::to_vec)
    }
}

/// A font file read with seeks.
pub struct FileBytes {
    file: RefCell<File>,
    size: u64,
}

impl FileBytes {
    pub fn open(path: &std::path::Path) -> std::io::Result<FileBytes> {
        let file = File::open(path)?;
        let size = file.metadata()?.len();
        Ok(FileBytes {
            file: RefCell::new(file),
            size,
        })
    }
}

impl Bytes for FileBytes {
    fn size(&self) -> u64 {
        self.size
    }
    fn read(&self, off: u64, len: usize) -> Option<Vec<u8>> {
        if off.checked_add(len as u64)? > self.size {
            return None;
        }
        let mut f = self.file.borrow_mut();
        f.seek(SeekFrom::Start(off)).ok()?;
        let mut buf = vec![0u8; len];
        f.read_exact(&mut buf).ok()?;
        Some(buf)
    }
}

pub(crate) fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*b.get(at)?, *b.get(at + 1)?]))
}

pub(crate) fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *b.get(at)?,
        *b.get(at + 1)?,
        *b.get(at + 2)?,
        *b.get(at + 3)?,
    ]))
}

const fn tag(t: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*t)
}

/// The number of faces in a font file: a `ttcf` collection's count, 1 for
/// a single sfnt, `None` for anything else (Type 1, a damaged file).
pub fn face_count(src: &dyn Bytes) -> Option<u32> {
    let head = src.read(0, 12)?;
    match u32_at(&head, 0)? {
        t if t == tag(b"ttcf") => u32_at(&head, 8),
        0x0001_0000 | 0x0002_0000 => Some(1),
        t if t == tag(b"OTTO") || t == tag(b"true") || t == tag(b"typ1") => Some(1),
        _ => None,
    }
}

/// One table directory entry that survived FreeType's checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Table {
    pub tag: u32,
    pub offset: u64,
    pub length: u64,
}

/// One face of an sfnt file: its validated table directory.
pub struct Face<'a> {
    pub src: &'a dyn Bytes,
    tables: Vec<Table>,
}

impl<'a> Face<'a> {
    /// `FT_Open_Face(path, index)`'s sfnt part: the face `index` of a
    /// collection (0 for a single font), with the table directory as
    /// `tt_face_load_font_dir` keeps it: an entry starting past the end of
    /// the file or running past it is dropped (`hmtx`/`vmtx` are cut to fit
    /// instead), and of two entries with one tag the first wins.
    pub fn open(src: &'a dyn Bytes, index: u32) -> Option<Face<'a>> {
        let head = src.read(0, 12)?;
        let mut dir = 0u64;
        if u32_at(&head, 0)? == tag(b"ttcf") {
            let count = u32_at(&head, 8)?;
            if index >= count {
                return None;
            }
            dir = u64::from(u32_at(&src.read(12 + 4 * u64::from(index), 4)?, 0)?);
        } else if index != 0 {
            return None;
        }
        let header = src.read(dir, 12)?;
        match u32_at(&header, 0)? {
            0x0001_0000 | 0x0002_0000 => {}
            t if t == tag(b"OTTO") || t == tag(b"true") || t == tag(b"typ1") => {}
            _ => return None,
        }
        let n = usize::from(u16_at(&header, 4)?);
        let records = src.read(dir + 12, 16 * n)?;
        let size = src.size();
        let mut tables: Vec<Table> = Vec::with_capacity(n);
        for r in records.chunks(16) {
            let t = u32_at(r, 0)?;
            let offset = u64::from(u32_at(r, 8)?);
            let mut length = u64::from(u32_at(r, 12)?);
            if offset > size {
                continue;
            } else if length > size - offset {
                if t == tag(b"hmtx") || t == tag(b"vmtx") {
                    length = (size - offset) & !3;
                } else {
                    continue;
                }
            }
            if tables.iter().any(|e| e.tag == t) {
                continue;
            }
            tables.push(Table {
                tag: t,
                offset,
                length,
            });
        }
        Some(Face { src, tables })
    }

    /// `tt_face_lookup_table`: a zero-length table counts as missing.
    pub fn table(&self, t: &[u8; 4]) -> Option<Table> {
        let t = tag(t);
        self.tables
            .iter()
            .find(|e| e.tag == t && e.length != 0)
            .copied()
    }

    /// The whole of a table (for the small ones: `head`, `OS/2`, `post`).
    pub fn table_data(&self, t: &[u8; 4]) -> Option<Vec<u8>> {
        let e = self.table(t)?;
        self.src.read(e.offset, usize::try_from(e.length).ok()?)
    }

    /// `tt_face_load_name` then `FT_Get_Sfnt_Name` for every index: the
    /// records FreeType keeps, in table order, with their strings. A record
    /// is dropped when its string is empty or outside the storage area, or
    /// (format 1) when its language tag is invalid.
    pub fn name_records(&self) -> Vec<NameRecord> {
        let Some(e) = self.table(b"name") else {
            return Vec::new();
        };
        let Some(hdr) = self.src.read(e.offset, 6) else {
            return Vec::new();
        };
        let format = u16_at(&hdr, 0).unwrap_or(0);
        let count = u64::from(u16_at(&hdr, 2).unwrap_or(0));
        let storage_offset = u64::from(u16_at(&hdr, 4).unwrap_or(0));
        let mut storage_start = e.offset + 6 + 12 * count;
        let storage_limit = e.offset + e.length;
        if storage_start > storage_limit {
            return Vec::new();
        }
        let Some(recs) = self.src.read(e.offset + 6, 12 * count as usize) else {
            return Vec::new();
        };
        // Format 1's language-tag records follow the name records.
        let mut lang_tags: Vec<u16> = Vec::new();
        if format == 1 {
            let Some(n) = self.src.read(storage_start, 2).and_then(|b| u16_at(&b, 0)) else {
                return Vec::new();
            };
            let Some(lt) = self.src.read(storage_start + 2, 4 * usize::from(n)) else {
                return Vec::new();
            };
            lang_tags = lt.chunks(4).map(|c| u16_at(c, 0).unwrap_or(0)).collect();
            storage_start += 2 + 4 * u64::from(n);
        }
        let mut out = Vec::new();
        for r in recs.chunks(12) {
            let (
                Some(platform),
                Some(encoding),
                Some(language),
                Some(name_id),
                Some(len),
                Some(off),
            ) = (
                u16_at(r, 0),
                u16_at(r, 2),
                u16_at(r, 4),
                u16_at(r, 6),
                u16_at(r, 8),
                u16_at(r, 10),
            )
            else {
                continue;
            };
            if len == 0 {
                continue;
            }
            let start = u64::from(off) + e.offset + storage_offset;
            if start < storage_start || start + u64::from(len) > storage_limit {
                continue;
            }
            if format == 1 && language >= 0x8000 {
                match lang_tags.get(usize::from(language - 0x8000)) {
                    Some(&l) if l != 0 => {}
                    _ => continue,
                }
            }
            let Some(bytes) = self.src.read(start, usize::from(len)) else {
                continue;
            };
            out.push(NameRecord {
                platform,
                encoding,
                language,
                name_id,
                bytes,
            });
        }
        out
    }

    /// `head.macStyle`, as `TT_Header.Mac_Style`. FreeType refuses a face
    /// without a 54-byte `head` (`tt_face_load_head`), so does this.
    pub fn mac_style(&self) -> Option<u16> {
        let e = self.table(b"head")?;
        let head = self.src.read(e.offset, 54)?;
        u16_at(&head, 44)
    }

    /// `FT_Get_Sfnt_Table(face, ft_sfnt_os2)`: `(usWeightClass,
    /// usWidthClass, fsSelection)`, or `None` where FreeType sets the
    /// table's version to 0xFFFF (missing, or its fields could not be read:
    /// `tt_face_load_os2` reads 78 bytes, 8 more from version 1, 10 more
    /// from version 2 and 4 more from version 5, from the file -- not
    /// bounded by the table's own length).
    pub fn os2(&self) -> Option<(u16, u16, u16)> {
        let e = self.table(b"OS/2")?;
        let base = self.src.read(e.offset, 78)?;
        let version = u16_at(&base, 0)?;
        let need = match version {
            0 => 78,
            1 => 86,
            2..=4 => 96,
            _ => 100,
        };
        if need > 78 {
            self.src.read(e.offset, need)?;
        }
        Some((u16_at(&base, 4)?, u16_at(&base, 6)?, u16_at(&base, 62)?))
    }

    /// `TT_Postscript.italicAngle` (16.16): FreeType's `face->postscript`
    /// is always returned, zero when `post` is missing or shorter than its
    /// 32-byte header.
    pub fn italic_angle(&self) -> i32 {
        self.table(b"post")
            .and_then(|e| self.src.read(e.offset, 32))
            .and_then(|b| u32_at(&b, 4))
            .map_or(0, |v| v as i32)
    }

    /// `hb_ot_layout_get_size_params` (HarfBuzz 12.3.2, hb-ot-layout.cc)
    /// on the face's GPOS as HarfBuzz's sanitizer leaves it.
    pub fn size_params(&self) -> Option<SizeParams> {
        let e = self.table(b"GPOS")?;
        size_params(self.src, e.offset, e.length)
    }

    /// `FT_Get_Postscript_Name` for a (non-variation) sfnt face:
    /// `sfnt_get_ps_name`.
    pub fn postscript_name(&self, records: &[NameRecord]) -> Option<String> {
        postscript_name(records)
    }
}

/// One `name` record and its raw string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameRecord {
    pub platform: u16,
    pub encoding: u16,
    pub language: u16,
    pub name_id: u16,
    pub bytes: Vec<u8>,
}

/// sfdriver.c's `sfnt_ps_map`: the ASCII characters allowed in a
/// PostScript name (bit `c & 7` of byte `c >> 3`).
const SFNT_PS_MAP: [u8; 16] = [
    0x00, 0x00, 0x00, 0x00, 0xDE, 0x7C, 0xFF, 0xAF, 0xFF, 0xFF, 0xFF, 0xD7, 0xFF, 0xFF, 0xFF, 0x57,
];

/// sfdriver.c's `sfnt_is_postscript`.
fn sfnt_is_postscript(c: u8) -> bool {
    c < 0x80 && SFNT_PS_MAP[usize::from(c >> 3)] & (1 << (c & 7)) != 0
}

/// sfdriver.c's `sfnt_get_ps_name` (no variation instance): name id 6,
/// the Windows record (platform 3, encoding 0 or 1; US English preferred,
/// else the first) before the Macintosh Roman one (language 0 preferred),
/// keeping only the characters `sfnt_is_postscript` allows; a string left
/// empty counts as absent (`get_win_string`, `get_apple_string`).
pub fn postscript_name(records: &[NameRecord]) -> Option<String> {
    let (mut win, mut apple) = (None, None);
    for (n, r) in records.iter().enumerate() {
        if r.name_id == 6 && !r.bytes.is_empty() {
            if r.platform == 3
                && (r.encoding == 1 || r.encoding == 0)
                && (r.language == 0x409 || win.is_none())
            {
                win = Some(n);
            }
            if r.platform == 1 && r.encoding == 0 && (r.language == 0 || apple.is_none()) {
                apple = Some(n);
            }
        }
    }
    let win_string = |r: &NameRecord| {
        let s: String = r
            .bytes
            .as_chunks::<2>()
            .0
            .iter()
            .filter(|p| p[0] == 0 && sfnt_is_postscript(p[1]))
            .map(|p| p[1] as char)
            .collect();
        (!s.is_empty()).then_some(s)
    };
    let apple_string = |r: &NameRecord| {
        let s: String = r
            .bytes
            .iter()
            .filter(|&&c| sfnt_is_postscript(c))
            .map(|&c| c as char)
            .collect();
        (!s.is_empty()).then_some(s)
    };
    win.and_then(|n| win_string(&records[n]))
        .or_else(|| apple.and_then(|n| apple_string(&records[n])))
}

/// ICU's `macintosh` converter (Apple's Mac OS Roman, `macos-0_2-10.2`)
/// for bytes 0x80..=0xFF.
const MAC_ROMAN_HIGH: [u16; 128] = [
    0x00C4, 0x00C5, 0x00C7, 0x00C9, 0x00D1, 0x00D6, 0x00DC, 0x00E1, 0x00E0, 0x00E2, 0x00E4, 0x00E3,
    0x00E5, 0x00E7, 0x00E9, 0x00E8, 0x00EA, 0x00EB, 0x00ED, 0x00EC, 0x00EE, 0x00EF, 0x00F1, 0x00F3,
    0x00F2, 0x00F4, 0x00F6, 0x00F5, 0x00FA, 0x00F9, 0x00FB, 0x00FC, 0x2020, 0x00B0, 0x00A2, 0x00A3,
    0x00A7, 0x2022, 0x00B6, 0x00DF, 0x00AE, 0x00A9, 0x2122, 0x00B4, 0x00A8, 0x2260, 0x00C6, 0x00D8,
    0x221E, 0x00B1, 0x2264, 0x2265, 0x00A5, 0x00B5, 0x2202, 0x2211, 0x220F, 0x03C0, 0x222B, 0x00AA,
    0x00BA, 0x03A9, 0x00E6, 0x00F8, 0x00BF, 0x00A1, 0x00AC, 0x221A, 0x0192, 0x2248, 0x2206, 0x00AB,
    0x00BB, 0x2026, 0x00A0, 0x00C0, 0x00C3, 0x00D5, 0x0152, 0x0153, 0x2013, 0x2014, 0x201C, 0x201D,
    0x2018, 0x2019, 0x00F7, 0x25CA, 0x00FF, 0x0178, 0x2044, 0x20AC, 0x2039, 0x203A, 0xFB01, 0xFB02,
    0x2021, 0x00B7, 0x201A, 0x201E, 0x2030, 0x00C2, 0x00CA, 0x00C1, 0x00CB, 0x00C8, 0x00CD, 0x00CE,
    0x00CF, 0x00CC, 0x00D3, 0x00D4, 0xF8FF, 0x00D2, 0x00DA, 0x00DB, 0x00D9, 0x0131, 0x02C6, 0x02DC,
    0x00AF, 0x02D8, 0x02D9, 0x02DA, 0x00B8, 0x02DD, 0x02DB, 0x02C7,
];

/// `convertToUtf8(macRomanConv, ...)` as a `std::string`: the string ends
/// at its first NUL, as the C string it is copied from does.
pub fn decode_mac_roman(bytes: &[u8]) -> String {
    let s: String = bytes
        .iter()
        .map(|&b| {
            if b < 0x80 {
                b as char
            } else {
                char::from_u32(u32::from(MAC_ROMAN_HIGH[usize::from(b - 0x80)]))
                    .unwrap_or('\u{FFFD}')
            }
        })
        .collect();
    cut_at_nul(s)
}

/// `convertToUtf8(utf16beConv, ...)`: UTF-16BE, a lone surrogate or a
/// trailing odd byte becoming U+FFFD (ICU's substitution), cut at the
/// first NUL.
pub fn decode_utf16be(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&c| u16::from_be_bytes(c))
        .collect();
    let mut s = String::from_utf16_lossy(&units);
    if bytes.len() % 2 == 1 {
        s.push('\u{FFFD}');
    }
    cut_at_nul(s)
}

fn cut_at_nul(mut s: String) -> String {
    if let Some(k) = s.find('\0') {
        s.truncate(k);
    }
    s
}

/// The `size` feature's parameters (OpenType `FeatureParamsSize`), in
/// decipoints as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeParams {
    pub design_size: u16,
    pub subfamily_id: u16,
    pub subfamily_name_id: u16,
    pub range_start: u16,
    pub range_end: u16,
}

/// `hb_ot_layout_get_size_params` over the GPOS table at `off` (`len`
/// bytes) of `src`, with what HarfBuzz's sanitizer does to the structures
/// it walks on the way:
///
/// * only a version 1.x GPOS has features (`GSUBGPOS::get_feature_list`);
/// * a FeatureList whose record array runs past the table is neutered
///   (offset set to 0: no features); a Feature table that does not fit,
///   or whose lookup-index array does not, likewise (a Null feature);
/// * a `size` FeatureParams that does not fit or fails
///   `FeatureParamsSize::sanitize` is neutered (a Null params, design size
///   0, so the search goes on to the next `size` feature).
///
/// The first `size` feature with a non-zero design size wins. Not modelled
/// (a belief: none occurs in a font that loads): damage elsewhere in GPOS
/// that makes HarfBuzz drop the whole table (more than 32 neuterings, a
/// broken ScriptList or LookupList that cannot be neutered).
pub fn size_params(src: &dyn Bytes, off: u64, len: u64) -> Option<SizeParams> {
    let rd = |at: u64, n: usize| -> Option<Vec<u8>> {
        if at.checked_add(n as u64)? > len {
            return None;
        }
        src.read(off + at, n)
    };
    let header = rd(0, 10)?;
    if u16_at(&header, 0)? != 1 {
        return None;
    }
    let list = u64::from(u16_at(&header, 6)?);
    if list == 0 {
        return None;
    }
    let count = usize::from(u16_at(&rd(list, 2)?, 0)?);
    let records = rd(list + 2, 6 * count)?;
    for r in records.chunks(6) {
        if r[0..4] != *b"size" {
            continue;
        }
        let feature = u16_at(r, 4)?;
        if feature == 0 {
            continue;
        }
        let feature = list + u64::from(feature);
        let Some(fh) = rd(feature, 4) else { continue };
        let lookups = u64::from(u16_at(&fh, 2)?);
        if rd(feature + 4, 2 * lookups as usize).is_none() {
            continue;
        }
        let params = u16_at(&fh, 0)?;
        if params == 0 {
            continue;
        }
        let Some(p) = rd(feature + u64::from(params), 10) else {
            continue;
        };
        let sp = SizeParams {
            design_size: u16_at(&p, 0)?,
            subfamily_id: u16_at(&p, 2)?,
            subfamily_name_id: u16_at(&p, 4)?,
            range_start: u16_at(&p, 6)?,
            range_end: u16_at(&p, 8)?,
        };
        if !feature_params_size_sane(&sp) {
            continue;
        }
        if sp.design_size != 0 {
            return Some(sp);
        }
    }
    None
}

/// `FeatureParamsSize::sanitize` (hb-ot-layout-common.hh): a design size,
/// and either no range at all or a range around it with a menu name id in
/// 256..=32767.
fn feature_params_size_sane(p: &SizeParams) -> bool {
    if p.design_size == 0 {
        false
    } else if p.subfamily_id == 0
        && p.subfamily_name_id == 0
        && p.range_start == 0
        && p.range_end == 0
    {
        true
    } else {
        !(p.design_size < p.range_start
            || p.design_size > p.range_end
            || p.subfamily_name_id < 256
            || p.subfamily_name_id > 32767)
    }
}
