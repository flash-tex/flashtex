//! Deterministic TrueType subsetting.
//!
//! The subset keeps `.notdef`, every requested glyph, and every component of
//! every composite glyph reached (transitively), renumbered in ascending
//! original-id order. Renumbering exists ONLY here; [`Subset::old_to_new`]
//! and [`Subset::new_to_old`] are the explicit maps. Tables written: `head`,
//! `hhea`, `maxp`, `hmtx`, `loca` (long), `glyf`, plus `cvt `, `fpgm`, `prep`
//! copied verbatim when present so hinting still works. `cmap`, `name`,
//! `OS/2`, `post`, `kern`, `GPOS`, `GSUB` are deliberately omitted: the
//! embedded program is addressed by glyph id (CIDFontType2 / Identity-H)
//! and shaping has already happened. Same input yields byte-identical output.

use std::collections::{BTreeMap, BTreeSet};

use crate::reader::{fnv1a, u16_at, u32_at};
use crate::truetype::{TrueTypeFace, composite_components};
use crate::{Error, GlyphId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subset {
    /// The subset font program (sfnt with correct checksums).
    pub program: Vec<u8>,
    /// Original glyph id -> subset glyph id.
    pub old_to_new: BTreeMap<u16, u16>,
    /// Subset glyph id -> original glyph id (index = new id).
    pub new_to_old: Vec<u16>,
    /// Advances in font units, indexed by subset glyph id.
    pub advances: Vec<u16>,
    pub units_per_em: u16,
    /// Six uppercase letters derived from the glyph set (PDF subset tag).
    pub tag: String,
}

impl Subset {
    pub fn num_glyphs(&self) -> u16 {
        self.new_to_old.len() as u16
    }

    pub fn new_id(&self, old: GlyphId) -> Option<GlyphId> {
        self.old_to_new.get(&old.0).map(|g| GlyphId(*g))
    }
}

const COPIED_IF_PRESENT: [&[u8; 4]; 3] = [b"cvt ", b"fpgm", b"prep"];

/// Builds the subset containing `gids` (any order, duplicates allowed).
pub fn subset(face: &TrueTypeFace, gids: &[GlyphId]) -> Result<Subset, Error> {
    if face.outlines() == crate::truetype::Outlines::Cff {
        return Err(Error::Unsupported(
            "CFF subsetting is not implemented; embed the whole program (FontFile3)".into(),
        ));
    }
    let mut wanted: BTreeSet<u16> = BTreeSet::new();
    wanted.insert(0);
    let mut stack: Vec<u16> = gids.iter().map(|g| g.0).collect();
    while let Some(g) = stack.pop() {
        if !wanted.insert(g) {
            continue;
        }
        for (component, _) in composite_components(face.glyph_data(GlyphId(g))?)? {
            stack.push(component);
        }
    }
    let new_to_old: Vec<u16> = wanted.iter().copied().collect();
    let old_to_new: BTreeMap<u16, u16> = new_to_old
        .iter()
        .enumerate()
        .map(|(new, old)| (*old, new as u16))
        .collect();
    let n = new_to_old.len();

    let mut glyf = Vec::new();
    let mut loca = Vec::with_capacity(n + 1);
    let mut advances = Vec::with_capacity(n);
    let mut hmtx = Vec::with_capacity(4 * n);
    let mut x_min = i16::MAX;
    let mut y_min = i16::MAX;
    let mut x_max = i16::MIN;
    let mut y_max = i16::MIN;
    for &old in &new_to_old {
        loca.push(glyf.len() as u32);
        let mut data = face.glyph_data(GlyphId(old))?.to_vec();
        for (component, at) in composite_components(&data)? {
            data[at..at + 2].copy_from_slice(&old_to_new[&component].to_be_bytes());
        }
        if data.len() >= 10 {
            x_min = x_min.min(u16_at(&data, 2)? as i16);
            y_min = y_min.min(u16_at(&data, 4)? as i16);
            x_max = x_max.max(u16_at(&data, 6)? as i16);
            y_max = y_max.max(u16_at(&data, 8)? as i16);
        }
        glyf.extend_from_slice(&data);
        while glyf.len() % 4 != 0 {
            glyf.push(0);
        }
        let (adv, lsb) = face.hmetric(GlyphId(old))?;
        advances.push(adv);
        hmtx.extend_from_slice(&adv.to_be_bytes());
        hmtx.extend_from_slice(&lsb.to_be_bytes());
    }
    loca.push(glyf.len() as u32);
    let loca_bytes: Vec<u8> = loca.iter().flat_map(|o| o.to_be_bytes()).collect();

    let src =
        |tag: &[u8; 4]| -> Vec<u8> { face.table(tag).map(<[u8]>::to_vec).unwrap_or_default() };
    let mut head = src(b"head");
    if head.len() < 54 {
        return Err(Error::Malformed("head table too short".into()));
    }
    head[8..12].copy_from_slice(&[0; 4]); // checkSumAdjustment, set by write_sfnt
    if x_min <= x_max {
        head[36..38].copy_from_slice(&x_min.to_be_bytes());
        head[38..40].copy_from_slice(&y_min.to_be_bytes());
        head[40..42].copy_from_slice(&x_max.to_be_bytes());
        head[42..44].copy_from_slice(&y_max.to_be_bytes());
    }
    head[50..52].copy_from_slice(&1i16.to_be_bytes()); // indexToLocFormat: long
    let mut hhea = src(b"hhea");
    if hhea.len() < 36 {
        return Err(Error::Malformed("hhea table too short".into()));
    }
    hhea[34..36].copy_from_slice(&(n as u16).to_be_bytes());
    let mut maxp = src(b"maxp");
    if maxp.len() < 6 {
        return Err(Error::Malformed("maxp table too short".into()));
    }
    maxp[4..6].copy_from_slice(&(n as u16).to_be_bytes());

    let mut tables: Vec<([u8; 4], Vec<u8>)> = vec![
        (*b"head", head),
        (*b"hhea", hhea),
        (*b"maxp", maxp),
        (*b"hmtx", hmtx),
        (*b"loca", loca_bytes),
        (*b"glyf", glyf),
    ];
    for tag in COPIED_IF_PRESENT {
        if face.has_table(tag) {
            tables.push((*tag, src(tag)));
        }
    }
    tables.sort_by_key(|t| t.0);

    let tag_seed: Vec<u8> = new_to_old.iter().flat_map(|g| g.to_be_bytes()).collect();
    Ok(Subset {
        program: write_sfnt(&tables),
        old_to_new,
        new_to_old,
        advances,
        units_per_em: crate::Face::units_per_em(face),
        tag: subset_tag(&tag_seed),
    })
}

/// Six uppercase letters derived deterministically from the glyph set.
pub fn subset_tag(seed: &[u8]) -> String {
    let h = fnv1a(seed);
    (0..6)
        .map(|i| (b'A' + ((h >> (8 * i)) % 26) as u8) as char)
        .collect()
}

pub fn checksum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    for chunk in data.chunks(4) {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum = sum.wrapping_add(u32::from_be_bytes(word));
    }
    sum
}

/// Serialises tables (sorted by tag) into an sfnt with table checksums and
/// `head.checkSumAdjustment`.
fn write_sfnt(tables: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let n = tables.len();
    let mut entry_selector = 0u16;
    while (2usize << entry_selector) <= n {
        entry_selector += 1;
    }
    let search_range = (1u16 << entry_selector) * 16;
    let range_shift = n as u16 * 16 - search_range;

    let mut out = Vec::new();
    out.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    out.extend_from_slice(&(n as u16).to_be_bytes());
    out.extend_from_slice(&search_range.to_be_bytes());
    out.extend_from_slice(&entry_selector.to_be_bytes());
    out.extend_from_slice(&range_shift.to_be_bytes());

    let mut offset = 12 + 16 * n;
    let mut body = Vec::new();
    let mut head_offset = None;
    for (tag, data) in tables {
        let padded = data.len().div_ceil(4) * 4;
        out.extend_from_slice(tag);
        out.extend_from_slice(&checksum(data).to_be_bytes());
        out.extend_from_slice(&(offset as u32).to_be_bytes());
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        if tag == b"head" {
            head_offset = Some(offset);
        }
        body.extend_from_slice(data);
        body.resize(body.len() + (padded - data.len()), 0);
        offset += padded;
    }
    out.extend_from_slice(&body);
    if let Some(h) = head_offset {
        let adjustment = 0xB1B0_AFBAu32.wrapping_sub(checksum(&out));
        out[h + 8..h + 12].copy_from_slice(&adjustment.to_be_bytes());
    }
    out
}

/// Verifies every table checksum, that tables do not overlap or overrun, and
/// that the whole-file checksum honours `head.checkSumAdjustment`.
pub fn verify_checksums(data: &[u8]) -> Result<(), Error> {
    let n = usize::from(u16_at(data, 4)?);
    let mut head_at = None;
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for i in 0..n {
        let rec = 12 + 16 * i;
        let tag = &data[rec..rec + 4];
        let want = u32_at(data, rec + 4)?;
        let off = u32_at(data, rec + 8)? as usize;
        let len = u32_at(data, rec + 12)? as usize;
        let table = data.get(off..off + len).ok_or_else(|| {
            Error::Malformed(format!(
                "table {} overruns file",
                String::from_utf8_lossy(tag)
            ))
        })?;
        let mut got = checksum(table);
        if tag == b"head" {
            head_at = Some(off);
            got = got.wrapping_sub(u32_at(table, 8)?);
        }
        if got != want {
            return Err(Error::Malformed(format!(
                "table {} checksum {got:08X} != {want:08X}",
                String::from_utf8_lossy(tag)
            )));
        }
        spans.push((off, off + len));
    }
    spans.sort_unstable();
    for w in spans.windows(2) {
        if w[0].1 > w[1].0 {
            return Err(Error::Malformed("tables overlap".into()));
        }
    }
    let Some(h) = head_at else {
        return Err(Error::MissingTable("head".into()));
    };
    let adjustment = u32_at(data, h + 8)?;
    let whole = checksum(data).wrapping_sub(adjustment);
    if 0xB1B0_AFBAu32.wrapping_sub(whole) != adjustment {
        return Err(Error::Malformed("head.checkSumAdjustment mismatch".into()));
    }
    Ok(())
}
