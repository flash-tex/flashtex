//! A minimal sfnt header reader for indexing: the table directory, the
//! `name`, `OS/2` and `head` tables and the presence of `MATH`, `CFF ` and
//! `glyf`. It reads only those bytes (seeks, no whole-file reads), so a
//! directory of a few hundred fonts -- several hundred megabytes of CJK and
//! emoji collections on a Mac -- indexes in tens of milliseconds. Outlines,
//! metrics, layout tables and everything else are font-engine's job once a
//! face is actually selected.
//!
//! Layout references: OpenType 1.9 §5 (table directory, `ttcf` header),
//! `name` (§5.9), `OS/2` (§5.10), `head` (§5.4).

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Which outline format a face carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outlines {
    /// `glyf`/`loca` TrueType outlines.
    Glyf,
    /// `CFF ` (Type 2 charstrings) in an `OTTO` container.
    Cff,
}

impl Outlines {
    /// The display-list-v2 `fonts[].format` string the pipeline publishes.
    pub fn format_name(self) -> &'static str {
        match self {
            Outlines::Glyf => "static-truetype",
            Outlines::Cff => "opentype-cff",
        }
    }
}

/// What the header of one face says about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaceInfo {
    /// `name` id 16 (typographic family) when present, else id 1. This is
    /// the name fontspec and typst match on: `Helvetica Neue`, not
    /// `Helvetica Neue Light`.
    pub family: String,
    /// `name` id 1: the legacy (four-style) family, which may carry the
    /// weight (`Helvetica Neue Light`). Kept as a second match key.
    pub legacy_family: String,
    /// `name` id 17 when present, else id 2 (`Bold Italic`, `12 Regular`).
    pub subfamily: String,
    /// `name` id 4.
    pub full_name: String,
    /// `name` id 6.
    pub postscript_name: String,
    /// `OS/2` `usWeightClass`, 1..=1000 (400 regular, 700 bold); from
    /// `head.macStyle` when the font has no `OS/2`.
    pub weight: u16,
    /// `OS/2` `usWidthClass`, 1..=9 (5 normal).
    pub width: u16,
    /// `OS/2` `fsSelection` ITALIC or OBLIQUE (bits 0 and 9), else
    /// `head.macStyle` bit 1.
    pub italic: bool,
    /// `OS/2` `fsSelection` BOLD (bit 5), else `head.macStyle` bit 0.
    pub bold: bool,
    /// Whether an OpenType `MATH` table is present.
    pub has_math: bool,
    pub outlines: Outlines,
    pub units_per_em: u16,
    /// `head.fontRevision` as a 16.16 fixed, for cache keys and display.
    pub revision: u32,
}

/// Bounds on what is read from one file: a `name` table larger than this is
/// truncated (only the record headers and the English strings matter) and
/// a collection with more faces than this is cut off.
const MAX_NAME_TABLE: u64 = 1 << 20;
const MAX_TTC_FACES: u32 = 64;

/// Every face of the font file at `path` (one for a single-face font, each
/// member of a `.ttc`), in face-index order.
pub fn read_faces(path: &Path) -> Result<Vec<FaceInfo>, String> {
    let mut f = File::open(path).map_err(|e| e.to_string())?;
    let mut head = [0u8; 12];
    f.read_exact(&mut head).map_err(|e| e.to_string())?;
    let tag = u32::from_be_bytes([head[0], head[1], head[2], head[3]]);
    let offsets: Vec<u64> = if tag == 0x7474_6366 {
        // 'ttcf': numFonts at 8, then one u32 offset per face.
        let n = u32::from_be_bytes([head[8], head[9], head[10], head[11]]).min(MAX_TTC_FACES);
        let mut buf = vec![0u8; 4 * n as usize];
        f.read_exact(&mut buf).map_err(|e| e.to_string())?;
        buf.chunks(4).map(|c| u64::from(u32::from_be_bytes([c[0], c[1], c[2], c[3]]))).collect()
    } else {
        vec![0]
    };
    let mut faces = Vec::with_capacity(offsets.len());
    for off in offsets {
        faces.push(read_face(&mut f, off)?);
    }
    Ok(faces)
}

fn read_at(f: &mut File, off: u64, len: usize) -> Result<Vec<u8>, String> {
    f.seek(SeekFrom::Start(off)).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf).map_err(|e| format!("short read at {off}: {e}"))?;
    Ok(buf)
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*b.get(at)?, *b.get(at + 1)?]))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes([*b.get(at)?, *b.get(at + 1)?, *b.get(at + 2)?, *b.get(at + 3)?]))
}

fn read_face(f: &mut File, dir: u64) -> Result<FaceInfo, String> {
    let header = read_at(f, dir, 12)?;
    let sfnt = u32_at(&header, 0).unwrap();
    let outlines = match sfnt {
        0x0001_0000 | 0x7472_7565 => Outlines::Glyf, // 1.0, 'true'
        0x4F54_544F => Outlines::Cff,                // 'OTTO'
        v => return Err(format!("sfnt version 0x{v:08X} is not a font")),
    };
    let n_tables = usize::from(u16_at(&header, 4).unwrap());
    if n_tables == 0 || n_tables > 512 {
        return Err(format!("{n_tables} tables"));
    }
    let records = read_at(f, dir + 12, 16 * n_tables)?;
    let mut name = None;
    let mut os2 = None;
    let mut head = None;
    let mut has_math = false;
    for r in records.chunks(16) {
        let tag = &r[0..4];
        let off = u64::from(u32_at(r, 8).unwrap());
        let len = u64::from(u32_at(r, 12).unwrap());
        match tag {
            b"name" => name = Some((off, len.min(MAX_NAME_TABLE))),
            b"OS/2" => os2 = Some((off, len)),
            b"head" => head = Some((off, len)),
            b"MATH" => has_math = true,
            _ => {}
        }
    }
    let (name_off, name_len) = name.ok_or("no name table")?;
    let (head_off, head_len) = head.ok_or("no head table")?;
    if head_len < 54 {
        return Err("head table too short".into());
    }
    let head = read_at(f, head_off, 54)?;
    let revision = u32_at(&head, 4).unwrap();
    let units_per_em = u16_at(&head, 18).unwrap();
    let mac_style = u16_at(&head, 44).unwrap();
    let names = read_at(f, name_off, name_len as usize)?;
    let strings = NameTable::parse(&names);
    let family1 = strings.get(1).unwrap_or_default();
    let family16 = strings.get(16);
    let sub2 = strings.get(2).unwrap_or_default();
    let sub17 = strings.get(17);
    let (weight, width, italic, bold) = match os2 {
        Some((off, len)) if len >= 64 => {
            let t = read_at(f, off, 64)?;
            let mut w = u16_at(&t, 4).unwrap();
            // Some old fonts write 1..9 where the spec says 100..900.
            if (1..=9).contains(&w) {
                w *= 100;
            }
            let weight = w.clamp(1, 1000);
            let width = u16_at(&t, 6).unwrap().clamp(1, 9);
            let sel = u16_at(&t, 62).unwrap();
            (weight, width, sel & 0x0001 != 0 || sel & 0x0200 != 0, sel & 0x0020 != 0)
        }
        _ => (if mac_style & 1 != 0 { 700 } else { 400 }, 5, mac_style & 2 != 0, mac_style & 1 != 0),
    };
    Ok(FaceInfo {
        family: family16.clone().unwrap_or_else(|| family1.clone()),
        legacy_family: family1,
        subfamily: sub17.unwrap_or(sub2),
        full_name: strings.get(4).unwrap_or_default(),
        postscript_name: strings.get(6).unwrap_or_default(),
        weight,
        width,
        italic,
        bold,
        has_math,
        outlines,
        units_per_em,
        revision,
    })
}

/// The `name` table's records, decoded on demand.
struct NameTable<'a> {
    data: &'a [u8],
    storage: usize,
    /// (platform, encoding, language, name id, length, offset)
    records: Vec<(u16, u16, u16, u16, usize, usize)>,
}

impl<'a> NameTable<'a> {
    fn parse(data: &'a [u8]) -> NameTable<'a> {
        let count = u16_at(data, 2).unwrap_or(0) as usize;
        let storage = u16_at(data, 4).unwrap_or(0) as usize;
        let mut records = Vec::with_capacity(count);
        for i in 0..count {
            let r = 6 + 12 * i;
            let (Some(p), Some(e), Some(l), Some(id), Some(len), Some(off)) =
                (u16_at(data, r), u16_at(data, r + 2), u16_at(data, r + 4), u16_at(data, r + 6), u16_at(data, r + 8), u16_at(data, r + 10))
            else {
                break;
            };
            records.push((p, e, l, id, usize::from(len), usize::from(off)));
        }
        NameTable { data, storage, records }
    }

    /// The string for `id` in the most useful encoding available: Windows
    /// Unicode (platform 3) in US English, then any Windows Unicode, then
    /// Macintosh Roman (platform 1), then Unicode (platform 0). Windows
    /// symbol-encoded fonts (3/0) count as Unicode too.
    fn get(&self, id: u16) -> Option<String> {
        let pick = |pred: &dyn Fn(u16, u16, u16) -> bool| {
            self.records.iter().find(|(p, e, l, i, _, _)| *i == id && pred(*p, *e, *l))
        };
        let rec = pick(&|p, e, l| p == 3 && (e == 1 || e == 10 || e == 0) && l == 0x409)
            .or_else(|| pick(&|p, e, _| p == 3 && (e == 1 || e == 10 || e == 0)))
            .or_else(|| pick(&|p, e, l| p == 1 && e == 0 && l == 0))
            .or_else(|| pick(&|p, _, _| p == 1))
            .or_else(|| pick(&|p, _, _| p == 0))?;
        let (platform, _, _, _, len, off) = *rec;
        let start = self.storage.checked_add(off)?;
        let bytes = self.data.get(start..start.checked_add(len)?)?;
        let s = if platform == 1 {
            // Mac Roman: ASCII plus a high half this index does not need
            // exactly (family names in system fonts are ASCII). Bytes above
            // 0x7F are mapped as Latin-1, which is right for the accented
            // Latin letters that do occur.
            bytes.iter().map(|&b| b as char).collect::<String>()
        } else {
            let units: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
            String::from_utf16_lossy(&units)
        };
        let s = s.trim().to_string();
        (!s.is_empty()).then_some(s)
    }
}
