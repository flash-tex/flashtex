//! Minimal OpenType reader and TrueType subsetter, written by hand so the
//! crate keeps its zero-dependency, offline build.
//!
//! Reads `head`, `hhea`, `hmtx`, `maxp`, `cmap` (formats 4 and 12), and, when
//! present, `OS/2`, `post`, and `name`, for both flavours of OpenType:
//!
//! - **TrueType outlines** (`glyf`/`loca`, sfnt version 1.0 or `true`): the
//!   font can be subset. The subset contains the requested glyphs plus
//!   `.notdef` and every glyph a composite refers to, renumbered densely,
//!   with `cvt `, `fpgm`, and `prep` copied so hinting stays valid. Table
//!   checksums and `head.checkSumAdjustment` are computed and
//!   [`verify_checksums`] reads them back.
//! - **CFF outlines** (`CFF ` table, sfnt version `OTTO`), e.g. Latin Modern:
//!   the raw `CFF ` table is exposed by [`TrueTypeFont::cff_table`] for
//!   embedding whole. CFF subsetting is not implemented yet, so
//!   [`TrueTypeFont::subset`] reports that rather than guessing.
//!
//! TrueType collections (`ttcf`) and fonts missing a required table are
//! rejected with a message.

use std::collections::BTreeMap;

const REQUIRED: [&[u8; 4]; 4] = [b"head", b"hhea", b"hmtx", b"maxp"];

/// Which outline format the font carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outlines {
    /// `glyf`/`loca`; subsettable.
    TrueType,
    /// `CFF ` table; embedded whole.
    Cff,
}
const COPIED_IF_PRESENT: [&[u8; 4]; 3] = [b"cvt ", b"fpgm", b"prep"];

#[derive(Debug, Clone)]
pub struct TrueTypeFont {
    data: Vec<u8>,
    tables: BTreeMap<[u8; 4], (usize, usize)>,
    pub units_per_em: u16,
    num_glyphs: u16,
    pub outlines: Outlines,
    /// Glyph data offsets into `glyf`, `num_glyphs + 1` entries; empty for CFF.
    loca: Vec<u32>,
    /// (advance width, left side bearing) per glyph, expanded.
    metrics: Vec<(u16, i16)>,
    cmap: BTreeMap<u32, u16>,
    /// Font bounding box from `head`, in font units.
    pub bbox: [i16; 4],
    pub ascender: i16,
    pub descender: i16,
    /// `OS/2` sCapHeight when the table is new enough, else `None`.
    pub cap_height: Option<i16>,
    /// `post` italicAngle in degrees.
    pub italic_angle: f64,
    /// PostScript name from `name` id 6, sanitised to PDF name characters.
    pub postscript_name: String,
}

/// A subset font ready for embedding as `/FontFile2`.
#[derive(Debug, Clone)]
pub struct Subset {
    pub bytes: Vec<u8>,
    /// Old glyph id to new glyph id, including `.notdef` and composite parts.
    pub glyph_map: BTreeMap<u16, u16>,
    /// Advance widths per new glyph id, in font units.
    pub advances: Vec<u16>,
}

impl Subset {
    pub fn num_glyphs(&self) -> u16 {
        self.glyph_map.len() as u16
    }
}

fn rd_u16(b: &[u8], at: usize) -> Result<u16, String> {
    b.get(at..at + 2)
        .map(|s| u16::from_be_bytes([s[0], s[1]]))
        .ok_or_else(|| format!("font truncated at byte {at}"))
}

fn rd_i16(b: &[u8], at: usize) -> Result<i16, String> {
    rd_u16(b, at).map(|v| v as i16)
}

fn rd_u32(b: &[u8], at: usize) -> Result<u32, String> {
    b.get(at..at + 4)
        .map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| format!("font truncated at byte {at}"))
}

impl TrueTypeFont {
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(data).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn parse(data: Vec<u8>) -> Result<Self, String> {
        let tag = rd_u32(&data, 0)?;
        let outlines = match tag {
            0x0001_0000 | 0x7472_7565 => Outlines::TrueType, // 1.0 or 'true'
            0x4F54_544F => Outlines::Cff,                    // 'OTTO'
            0x7474_6366 => {
                return Err(
                    "TrueType collections (.ttc) are not supported; extract one face".into(),
                );
            }
            other => return Err(format!("not an OpenType font (sfnt version {other:#010x})")),
        };
        let num_tables = rd_u16(&data, 4)? as usize;
        let mut tables = BTreeMap::new();
        for i in 0..num_tables {
            let rec = 12 + 16 * i;
            let tag: [u8; 4] = data
                .get(rec..rec + 4)
                .ok_or("table directory truncated")?
                .try_into()
                .expect("4 bytes");
            let offset = rd_u32(&data, rec + 8)? as usize;
            let length = rd_u32(&data, rec + 12)? as usize;
            if offset
                .checked_add(length)
                .is_none_or(|end| end > data.len())
            {
                return Err(format!(
                    "table {:?} lies outside the file",
                    String::from_utf8_lossy(&tag)
                ));
            }
            tables.insert(tag, (offset, length));
        }
        let required: Vec<&[u8; 4]> = match outlines {
            Outlines::TrueType => REQUIRED
                .iter()
                .chain([b"loca", b"glyf"].iter())
                .copied()
                .collect(),
            Outlines::Cff => REQUIRED.iter().chain([b"CFF "].iter()).copied().collect(),
        };
        for req in required {
            if !tables.contains_key(req) {
                return Err(format!(
                    "missing required table {:?}",
                    String::from_utf8_lossy(req)
                ));
            }
        }
        let table = |tag: &[u8; 4]| -> &[u8] {
            let (o, l) = tables[tag];
            &data[o..o + l]
        };

        let head = table(b"head");
        let units_per_em = rd_u16(head, 18)?;
        if units_per_em == 0 {
            return Err("unitsPerEm is zero".into());
        }
        let bbox = [
            rd_i16(head, 36)?,
            rd_i16(head, 38)?,
            rd_i16(head, 40)?,
            rd_i16(head, 42)?,
        ];
        let long_loca = match rd_i16(head, 50)? {
            0 => false,
            1 => true,
            v => return Err(format!("unknown indexToLocFormat {v}")),
        };

        let maxp = table(b"maxp");
        let num_glyphs = rd_u16(maxp, 4)?;

        let hhea = table(b"hhea");
        let ascender = rd_i16(hhea, 4)?;
        let descender = rd_i16(hhea, 6)?;
        let num_h_metrics = rd_u16(hhea, 34)? as usize;
        if num_h_metrics == 0 || num_h_metrics > num_glyphs as usize {
            return Err(format!(
                "numberOfHMetrics {num_h_metrics} inconsistent with {num_glyphs} glyphs"
            ));
        }

        let hmtx = table(b"hmtx");
        let mut metrics = Vec::with_capacity(num_glyphs as usize);
        let mut last_advance = 0;
        for g in 0..num_glyphs as usize {
            if g < num_h_metrics {
                last_advance = rd_u16(hmtx, 4 * g)?;
                metrics.push((last_advance, rd_i16(hmtx, 4 * g + 2)?));
            } else {
                let lsb_at = 4 * num_h_metrics + 2 * (g - num_h_metrics);
                metrics.push((last_advance, rd_i16(hmtx, lsb_at)?));
            }
        }

        let mut loca = Vec::new();
        if outlines == Outlines::TrueType {
            let loca_tbl = table(b"loca");
            loca.reserve(num_glyphs as usize + 1);
            for g in 0..=num_glyphs as usize {
                loca.push(if long_loca {
                    rd_u32(loca_tbl, 4 * g)?
                } else {
                    rd_u16(loca_tbl, 2 * g)? as u32 * 2
                });
            }
            let glyf_len = tables[b"glyf"].1 as u32;
            if loca.windows(2).any(|w| w[0] > w[1])
                || loca.last().is_some_and(|&end| end > glyf_len)
            {
                return Err("loca offsets are not monotonic or exceed glyf".into());
            }
        } else {
            let cff = table(b"CFF ");
            if cff.len() < 4 || cff[0] != 1 {
                return Err("CFF table is not a version 1 CFF".into());
            }
        }

        let cmap = match tables.get(b"cmap") {
            Some(_) => parse_cmap(table(b"cmap"))?,
            None => BTreeMap::new(),
        };

        let cap_height = tables.get(b"OS/2").and_then(|_| {
            let os2 = table(b"OS/2");
            (rd_u16(os2, 0).ok()? >= 2)
                .then(|| rd_i16(os2, 88).ok())
                .flatten()
        });
        let italic_angle = tables
            .get(b"post")
            .and_then(|_| rd_u32(table(b"post"), 4).ok())
            .map(|fixed| fixed as i32 as f64 / 65536.0)
            .unwrap_or(0.0);
        let postscript_name = tables
            .get(b"name")
            .and_then(|_| postscript_name(table(b"name")))
            .unwrap_or_else(|| "TrueTypeFont".into());

        Ok(TrueTypeFont {
            outlines,
            tables,
            units_per_em,
            num_glyphs,
            loca,
            metrics,
            cmap,
            bbox,
            ascender,
            descender,
            cap_height,
            italic_angle,
            postscript_name,
            data,
        })
    }

    pub fn num_glyphs(&self) -> u16 {
        self.num_glyphs
    }

    /// The raw `CFF ` table for a CFF-flavoured font, `None` for TrueType.
    pub fn cff_table(&self) -> Option<&[u8]> {
        let (o, l) = *self.tables.get(b"CFF ")?;
        Some(&self.data[o..o + l])
    }

    /// Glyph id for a character, or `None` when the font has no glyph for it
    /// (a mapping to glyph 0 counts as missing).
    pub fn glyph_id(&self, c: char) -> Option<u16> {
        self.cmap
            .get(&(c as u32))
            .copied()
            .filter(|&g| g != 0 && g < self.num_glyphs)
    }

    /// The lowest code point the `cmap` maps to `gid` (`.` rather than
    /// U+2024 for the period), or `None` for a glyph no character reaches
    /// (ligatures, alternates).
    pub fn char_for_glyph(&self, gid: u16) -> Option<char> {
        if gid == 0 {
            return None;
        }
        self.cmap
            .iter()
            .filter(|(_, g)| **g == gid)
            .find_map(|(cp, _)| char::from_u32(*cp))
    }

    pub fn advance(&self, gid: u16) -> u16 {
        self.metrics.get(gid as usize).map_or(0, |m| m.0)
    }

    fn glyph_data(&self, gid: u16) -> &[u8] {
        let (o, _) = self.tables[b"glyf"];
        let start = o + self.loca[gid as usize] as usize;
        let end = o + self.loca[gid as usize + 1] as usize;
        &self.data[start..end]
    }

    /// Builds a subset containing `gids` (in any order, duplicates allowed),
    /// `.notdef`, and every component of every composite glyph reached.
    pub fn subset(&self, gids: &[u16]) -> Result<Subset, String> {
        if self.outlines != Outlines::TrueType {
            return Err("CFF outlines cannot be subset yet; embed the CFF table whole".into());
        }
        let mut wanted = std::collections::BTreeSet::new();
        wanted.insert(0u16);
        let mut stack: Vec<u16> = gids.to_vec();
        while let Some(g) = stack.pop() {
            if g >= self.num_glyphs {
                return Err(format!("glyph {g} out of range"));
            }
            if !wanted.insert(g) {
                continue;
            }
            for part in composite_components(self.glyph_data(g))? {
                stack.push(part.gid);
            }
        }
        let glyph_map: BTreeMap<u16, u16> = wanted
            .iter()
            .enumerate()
            .map(|(new, &old)| (old, new as u16))
            .collect();
        let n = glyph_map.len();

        let mut glyf = Vec::new();
        let mut loca = Vec::with_capacity(n + 1);
        let mut advances = Vec::with_capacity(n);
        let mut hmtx = Vec::with_capacity(4 * n);
        for &old in &wanted {
            loca.push(glyf.len() as u32);
            let mut data = self.glyph_data(old).to_vec();
            for part in composite_components(&data)? {
                let new = glyph_map[&part.gid];
                data[part.index_at..part.index_at + 2].copy_from_slice(&new.to_be_bytes());
            }
            glyf.extend_from_slice(&data);
            while glyf.len() % 4 != 0 {
                glyf.push(0);
            }
            let (adv, lsb) = self.metrics[old as usize];
            advances.push(adv);
            hmtx.extend_from_slice(&adv.to_be_bytes());
            hmtx.extend_from_slice(&lsb.to_be_bytes());
        }
        loca.push(glyf.len() as u32);
        let loca_bytes: Vec<u8> = loca.iter().flat_map(|o| o.to_be_bytes()).collect();

        let src = |tag: &[u8; 4]| -> Vec<u8> {
            let (o, l) = self.tables[tag];
            self.data[o..o + l].to_vec()
        };
        let mut head = src(b"head");
        head[8..12].copy_from_slice(&[0; 4]); // checkSumAdjustment, filled below
        head[50..52].copy_from_slice(&1i16.to_be_bytes()); // long loca
        let mut hhea = src(b"hhea");
        hhea[34..36].copy_from_slice(&(n as u16).to_be_bytes());
        let mut maxp = src(b"maxp");
        maxp[4..6].copy_from_slice(&(n as u16).to_be_bytes());

        let mut out_tables: Vec<([u8; 4], Vec<u8>)> = vec![
            (*b"head", head),
            (*b"hhea", hhea),
            (*b"maxp", maxp),
            (*b"hmtx", hmtx),
            (*b"loca", loca_bytes),
            (*b"glyf", glyf),
        ];
        for tag in COPIED_IF_PRESENT {
            if self.tables.contains_key(tag) {
                out_tables.push((*tag, src(tag)));
            }
        }
        out_tables.sort_by_key(|t| t.0);

        Ok(Subset {
            bytes: write_sfnt(&out_tables),
            glyph_map,
            advances,
        })
    }
}

impl TrueTypeFont {
    /// Builds a subset that **keeps glyph ids in place**: glyphs `0..=max`
    /// (where `max` is the highest requested or component glyph id) are
    /// all present, the requested ones and their composite components with
    /// their original outlines and metrics, every other one as an empty
    /// glyph. Composite references therefore need no renumbering and a
    /// `/CIDToGIDMap /Identity` CID font selects glyphs by the font's own
    /// ids. The trailing unused glyphs are dropped, so `maxp.numGlyphs`
    /// becomes `max + 1`.
    pub fn subset_keep_gids(
        &self,
        gids: &std::collections::BTreeSet<u16>,
    ) -> Result<Vec<u8>, String> {
        if self.outlines != Outlines::TrueType {
            return Err("only glyf outlines can be subset in place".into());
        }
        let mut wanted = std::collections::BTreeSet::new();
        wanted.insert(0u16);
        let mut stack: Vec<u16> = gids.iter().copied().collect();
        while let Some(g) = stack.pop() {
            if g >= self.num_glyphs {
                return Err(format!("glyph {g} out of range"));
            }
            if !wanted.insert(g) {
                continue;
            }
            for part in composite_components(self.glyph_data(g))? {
                stack.push(part.gid);
            }
        }
        let n = *wanted.iter().max().expect("has .notdef") as usize + 1;
        let mut glyf = Vec::new();
        let mut loca = Vec::with_capacity(n + 1);
        let mut hmtx = Vec::with_capacity(4 * n);
        for g in 0..n as u16 {
            loca.push(glyf.len() as u32);
            let (adv, lsb) = self.metrics[g as usize];
            if wanted.contains(&g) {
                glyf.extend_from_slice(self.glyph_data(g));
                while glyf.len() % 4 != 0 {
                    glyf.push(0);
                }
            }
            // Metrics are kept for every glyph so /W stays truthful; an
            // unused glyph simply has an empty outline.
            hmtx.extend_from_slice(&adv.to_be_bytes());
            hmtx.extend_from_slice(&lsb.to_be_bytes());
        }
        loca.push(glyf.len() as u32);
        let loca_bytes: Vec<u8> = loca.iter().flat_map(|o| o.to_be_bytes()).collect();
        let src = |tag: &[u8; 4]| -> Vec<u8> {
            let (o, l) = self.tables[tag];
            self.data[o..o + l].to_vec()
        };
        let mut head = src(b"head");
        head[8..12].copy_from_slice(&[0; 4]);
        head[50..52].copy_from_slice(&1i16.to_be_bytes());
        let mut hhea = src(b"hhea");
        hhea[34..36].copy_from_slice(&(n as u16).to_be_bytes());
        let mut maxp = src(b"maxp");
        maxp[4..6].copy_from_slice(&(n as u16).to_be_bytes());
        let mut out_tables: Vec<([u8; 4], Vec<u8>)> = vec![
            (*b"head", head),
            (*b"hhea", hhea),
            (*b"maxp", maxp),
            (*b"hmtx", hmtx),
            (*b"loca", loca_bytes),
            (*b"glyf", glyf),
        ];
        for tag in COPIED_IF_PRESENT {
            if self.tables.contains_key(tag) {
                out_tables.push((*tag, src(tag)));
            }
        }
        out_tables.sort_by_key(|t| t.0);
        Ok(write_sfnt(&out_tables))
    }
}

struct Component {
    gid: u16,
    /// Byte offset of the component's glyph index within the glyph data.
    index_at: usize,
}

/// Lists the components of a composite glyph; empty for simple/empty glyphs.
fn composite_components(data: &[u8]) -> Result<Vec<Component>, String> {
    if data.len() < 10 || rd_i16(data, 0)? >= 0 {
        return Ok(Vec::new());
    }
    let mut parts = Vec::new();
    let mut at = 10;
    loop {
        let flags = rd_u16(data, at)?;
        parts.push(Component {
            gid: rd_u16(data, at + 2)?,
            index_at: at + 2,
        });
        at += 4;
        at += if flags & 0x0001 != 0 { 4 } else { 2 }; // ARG_1_AND_2_ARE_WORDS
        at += if flags & 0x0008 != 0 {
            2 // WE_HAVE_A_SCALE
        } else if flags & 0x0040 != 0 {
            4 // WE_HAVE_AN_X_AND_Y_SCALE
        } else if flags & 0x0080 != 0 {
            8 // WE_HAVE_A_TWO_BY_TWO
        } else {
            0
        };
        if flags & 0x0020 == 0 {
            break; // no MORE_COMPONENTS
        }
    }
    Ok(parts)
}

fn parse_cmap(cmap: &[u8]) -> Result<BTreeMap<u32, u16>, String> {
    let n = rd_u16(cmap, 2)? as usize;
    // Prefer a full-Unicode format 12 table, then a BMP format 4 table.
    let mut best: Option<(u8, usize)> = None;
    for i in 0..n {
        let rec = 4 + 8 * i;
        let platform = rd_u16(cmap, rec)?;
        let encoding = rd_u16(cmap, rec + 2)?;
        let offset = rd_u32(cmap, rec + 4)? as usize;
        let unicode = matches!((platform, encoding), (0, _) | (3, 1) | (3, 10));
        if !unicode {
            continue;
        }
        let format = rd_u16(cmap, offset)?;
        let rank = match format {
            12 => 2,
            4 => 1,
            _ => continue,
        };
        if best.is_none_or(|(r, _)| rank > r) {
            best = Some((rank, offset));
        }
    }
    let Some((rank, at)) = best else {
        return Err("no Unicode cmap subtable of format 4 or 12".into());
    };
    let mut map = BTreeMap::new();
    if rank == 2 {
        let groups = rd_u32(cmap, at + 12)? as usize;
        for g in 0..groups {
            let rec = at + 16 + 12 * g;
            let start = rd_u32(cmap, rec)?;
            let end = rd_u32(cmap, rec + 4)?;
            let start_gid = rd_u32(cmap, rec + 8)?;
            if end < start || end - start > 0x10FFFF {
                return Err("malformed cmap format 12 group".into());
            }
            for (i, cp) in (start..=end).enumerate() {
                let gid = start_gid as usize + i;
                if gid != 0 && gid <= u16::MAX as usize {
                    map.insert(cp, gid as u16);
                }
            }
        }
    } else {
        let seg_x2 = rd_u16(cmap, at + 6)? as usize;
        let segs = seg_x2 / 2;
        let ends = at + 14;
        let starts = ends + seg_x2 + 2;
        let deltas = starts + seg_x2;
        let range_offsets = deltas + seg_x2;
        for s in 0..segs {
            let end = rd_u16(cmap, ends + 2 * s)? as u32;
            let start = rd_u16(cmap, starts + 2 * s)? as u32;
            let delta = rd_u16(cmap, deltas + 2 * s)?;
            let range_offset = rd_u16(cmap, range_offsets + 2 * s)? as usize;
            if start > end {
                continue;
            }
            for cp in start..=end {
                if cp == 0xFFFF {
                    continue;
                }
                let gid = if range_offset == 0 {
                    (cp as u16).wrapping_add(delta)
                } else {
                    let addr = range_offsets + 2 * s + range_offset + 2 * (cp - start) as usize;
                    match rd_u16(cmap, addr) {
                        Ok(0) => 0,
                        Ok(g) => g.wrapping_add(delta),
                        Err(_) => 0,
                    }
                };
                if gid != 0 {
                    map.insert(cp, gid);
                }
            }
        }
    }
    Ok(map)
}

fn postscript_name(name: &[u8]) -> Option<String> {
    let count = rd_u16(name, 2).ok()? as usize;
    let storage = rd_u16(name, 4).ok()? as usize;
    let mut fallback = None;
    for i in 0..count {
        let rec = 6 + 12 * i;
        let platform = rd_u16(name, rec).ok()?;
        let name_id = rd_u16(name, rec + 6).ok()?;
        if name_id != 6 {
            continue;
        }
        let len = rd_u16(name, rec + 8).ok()? as usize;
        let off = storage + rd_u16(name, rec + 10).ok()? as usize;
        let raw = name.get(off..off + len)?;
        let decoded = match platform {
            3 | 0 => {
                let units: Vec<u16> = raw
                    .chunks(2)
                    .map(|c| u16::from_be_bytes([c[0], *c.get(1).unwrap_or(&0)]))
                    .collect();
                String::from_utf16_lossy(&units)
            }
            _ => raw.iter().map(|&b| b as char).collect(),
        };
        let clean: String = decoded
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        if clean.is_empty() {
            continue;
        }
        if platform == 3 {
            return Some(clean);
        }
        fallback.get_or_insert(clean);
    }
    fallback
}

fn checksum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    for chunk in data.chunks(4) {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum = sum.wrapping_add(u32::from_be_bytes(word));
    }
    sum
}

/// Serialises tables (already sorted by tag) into an sfnt file with correct
/// directory fields, table checksums, and `head.checkSumAdjustment`.
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

/// Re-reads an sfnt and checks every table checksum in the directory, that
/// tables do not overlap or overrun, and that the whole-file checksum equals
/// the magic `0xB1B0AFBA` once `head.checkSumAdjustment` is honoured.
pub fn verify_checksums(data: &[u8]) -> Result<(), String> {
    let n = rd_u16(data, 4)? as usize;
    let mut head_at = None;
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for i in 0..n {
        let rec = 12 + 16 * i;
        let tag = &data[rec..rec + 4];
        let want = rd_u32(data, rec + 4)?;
        let off = rd_u32(data, rec + 8)? as usize;
        let len = rd_u32(data, rec + 12)? as usize;
        let table = data
            .get(off..off + len)
            .ok_or_else(|| format!("table {:?} overruns file", String::from_utf8_lossy(tag)))?;
        let mut got = checksum(table);
        if tag == b"head" {
            head_at = Some(off);
            got = got.wrapping_sub(rd_u32(data, off + 8)?);
        }
        if got != want {
            return Err(format!(
                "table {:?} checksum {got:#010x} != directory {want:#010x}",
                String::from_utf8_lossy(tag)
            ));
        }
        spans.push((off, off + len));
    }
    spans.sort();
    if spans.windows(2).any(|w| w[0].1 > w[1].0) {
        return Err("tables overlap".into());
    }
    let head = head_at.ok_or("no head table")?;
    let adjustment = rd_u32(data, head + 8)?;
    let whole = checksum(data);
    // The file checksum includes the adjustment field itself, so the sum of
    // everything else must be magic - adjustment.
    if whole != 0xB1B0_AFBA {
        return Err(format!(
            "whole-file checksum {whole:#010x} != 0xB1B0AFBA (checkSumAdjustment {adjustment:#010x})"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_truetype() {
        assert!(
            TrueTypeFont::parse(b"OTTO\0\0\0\0\0\0\0\0".to_vec())
                .unwrap_err()
                .contains("head")
        );
        assert!(
            TrueTypeFont::parse(b"ttcf\0\0\0\0\0\0\0\0".to_vec())
                .unwrap_err()
                .contains("collection")
        );
        assert!(TrueTypeFont::parse(vec![0; 3]).is_err());
        assert!(
            TrueTypeFont::parse(b"\0\x01\0\0\0\0\0\0\0\0\0\0".to_vec())
                .unwrap_err()
                .contains("head")
        );
    }

    #[test]
    fn sfnt_writer_produces_verifiable_checksums() {
        let mut head = vec![0u8; 54];
        head[50..52].copy_from_slice(&1i16.to_be_bytes());
        let tables = vec![
            (*b"glyf", vec![1, 2, 3, 4, 5]),
            (*b"head", head),
            (*b"loca", vec![0, 0, 0, 0, 0, 0, 0, 8]),
        ];
        let bytes = write_sfnt(&tables);
        verify_checksums(&bytes).unwrap();
        assert_eq!(rd_u16(&bytes, 4).unwrap(), 3);
        assert_eq!(rd_u16(&bytes, 6).unwrap(), 32); // searchRange: 2 tables * 16
        assert_eq!(rd_u16(&bytes, 8).unwrap(), 1); // entrySelector
        assert_eq!(rd_u16(&bytes, 10).unwrap(), 16); // rangeShift
    }

    #[test]
    fn composite_parsing_walks_flags() {
        // Two components: first with word args and a 2x2 scale, MORE_COMPONENTS;
        // second with byte args and no scale.
        let mut g = vec![0u8; 10];
        g[0..2].copy_from_slice(&(-1i16).to_be_bytes());
        g.extend_from_slice(&(0x0001u16 | 0x0080 | 0x0020).to_be_bytes());
        g.extend_from_slice(&7u16.to_be_bytes());
        g.extend_from_slice(&[0; 4 + 8]);
        g.extend_from_slice(&0u16.to_be_bytes());
        g.extend_from_slice(&9u16.to_be_bytes());
        g.extend_from_slice(&[0; 2]);
        let parts = composite_components(&g).unwrap();
        assert_eq!(parts.iter().map(|p| p.gid).collect::<Vec<_>>(), vec![7, 9]);
        assert_eq!(parts[0].index_at, 12);
        assert_eq!(parts[1].index_at, 12 + 2 + 4 + 8 + 2);
    }
}
