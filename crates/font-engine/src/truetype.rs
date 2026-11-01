//! TrueType / OpenType (`glyf`) font parsing.
//!
//! Tables read: `head`, `hhea`, `hmtx`, `maxp`, `loca`, `glyf` (or `CFF `),
//! `cmap` (formats 4 and 12; optional so subset programs re-parse), `OS/2`,
//! `post`, `name`, `kern` (format 0), `GPOS` (PairPos under `kern`), `GSUB`
//! (LigatureSubst under `liga`), `MATH` (constants, italics correction, top
//! accent). TrueType collections (`ttcf`) are supported by face index.
//!
//! `CFF `-based OpenType (`OTTO`, e.g. Latin Modern) parses with the same
//! metric/cmap/layout tables; its outlines are exposed as the raw `CFF `
//! table for whole-program embedding ([`Outlines::Cff`]). CFF glyph parsing
//! and subsetting are not implemented. Variable fonts (`fvar`/`gvar`) and
//! bitmap-only fonts are rejected with [`Error::Unsupported`] rather than
//! partially parsed.

use std::collections::BTreeMap;

use crate::gpos::{GposKerning, MarkAttachment};
use crate::gsub::GsubLigatures;
use crate::kern::KernTable;
use crate::math::MathTable;
use crate::reader::{i16_at, i32_at, slice, u16_at, u32_at};
use crate::{
    Error, Face, FontId, FontSource, GlyphId, KerningSource, Style, Unsupported, VerticalMetrics,
};

/// Which outline format the program carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outlines {
    /// `glyf`/`loca` TrueType outlines; subsetting available.
    Glyf,
    /// `CFF ` (Type 2 charstrings); raw table exposed, whole-program embedding.
    Cff,
}

#[derive(Debug, Clone)]
pub struct TrueTypeFace {
    data: Vec<u8>,
    outlines: Outlines,
    /// Table tag -> (offset, length) into `data`.
    tables: BTreeMap<[u8; 4], (usize, usize)>,
    id: FontId,
    units_per_em: u16,
    num_glyphs: u16,
    bbox: [i16; 4],
    /// (advance, lsb) per glyph, expanded so every glyph has an entry.
    hmetrics: Vec<(u16, i16)>,
    loca: Vec<u32>,
    cmap: BTreeMap<u32, u16>,
    metrics: VerticalMetrics,
    italic_angle: f64,
    is_fixed_pitch: bool,
    postscript_name: String,
    /// OS/2 fsType embedding restrictions, raw.
    pub fs_type: u16,
    kern: KernTable,
    gpos: GposKerning,
    marks: MarkAttachment,
    gsub: GsubLigatures,
    math: Option<MathTable>,
    unsupported: Vec<Unsupported>,
}

const REQUIRED: [&[u8; 4]; 4] = [b"head", b"hhea", b"hmtx", b"maxp"];

impl TrueTypeFace {
    /// Parses face 0 of an in-memory font program.
    pub fn parse(data: Vec<u8>) -> Result<TrueTypeFace, Error> {
        TrueTypeFace::parse_with_source(data, FontSource::Memory { face_index: 0 })
    }

    /// Parses the face named by `source` (its `face_index`) from `data`.
    pub fn parse_with_source(data: Vec<u8>, source: FontSource) -> Result<TrueTypeFace, Error> {
        let face_index = match &source {
            FontSource::File { face_index, .. } | FontSource::Memory { face_index } => *face_index,
            FontSource::Core14 { .. } => {
                return Err(Error::Unsupported("Core14 source has no program".into()));
            }
        };
        let b = &data[..];
        let tag = u32_at(b, 0)?;
        let dir = if tag == 0x7474_6366 {
            // 'ttcf'
            let n = u32_at(b, 8)?;
            if face_index >= n {
                return Err(Error::Malformed(format!(
                    "face index {face_index} but collection has {n} faces"
                )));
            }
            u32_at(b, 12 + 4 * face_index as usize)? as usize
        } else {
            if face_index != 0 {
                return Err(Error::Malformed("face index on a single-face font".into()));
            }
            0
        };
        let sfnt_version = u32_at(b, dir)?;
        let outlines = match sfnt_version {
            0x0001_0000 | 0x7472_7565 => Outlines::Glyf, // 1.0, 'true'
            0x4F54_544F => Outlines::Cff,                // 'OTTO'
            v => return Err(Error::Malformed(format!("sfnt version 0x{v:08X}"))),
        };
        let n_tables = usize::from(u16_at(b, dir + 4)?);
        let mut tables = BTreeMap::new();
        for i in 0..n_tables {
            let rec = dir + 12 + 16 * i;
            let tag: [u8; 4] = slice(b, rec, 4)?.try_into().unwrap();
            let off = u32_at(b, rec + 8)? as usize;
            let len = u32_at(b, rec + 12)? as usize;
            if off.checked_add(len).is_none_or(|end| end > b.len()) {
                return Err(Error::Malformed(format!(
                    "table {} overruns file",
                    String::from_utf8_lossy(&tag)
                )));
            }
            tables.insert(tag, (off, len));
        }
        for t in REQUIRED {
            if !tables.contains_key(t) {
                return Err(Error::MissingTable(String::from_utf8_lossy(t).into_owned()));
            }
        }
        match outlines {
            Outlines::Glyf => {
                for t in [b"loca", b"glyf"] {
                    if !tables.contains_key(t) {
                        return Err(Error::MissingTable(String::from_utf8_lossy(t).into_owned()));
                    }
                }
            }
            Outlines::Cff => {
                if !tables.contains_key(b"CFF ") {
                    return Err(Error::MissingTable("CFF ".into()));
                }
            }
        }
        if tables.contains_key(b"fvar") || tables.contains_key(b"gvar") {
            return Err(Error::Unsupported(
                "variable fonts (fvar/gvar) are not implemented".into(),
            ));
        }
        let table =
            |tag: &[u8; 4]| -> Option<&[u8]> { tables.get(tag).map(|(o, l)| &b[*o..*o + *l]) };

        let head = table(b"head").unwrap();
        if u32_at(head, 12)? != 0x5F0F_3CF5 {
            return Err(Error::Malformed("head magic".into()));
        }
        let units_per_em = u16_at(head, 18)?;
        if units_per_em == 0 {
            return Err(Error::Malformed("unitsPerEm is 0".into()));
        }
        let bbox = [
            i16_at(head, 36)?,
            i16_at(head, 38)?,
            i16_at(head, 40)?,
            i16_at(head, 42)?,
        ];
        let mac_style = u16_at(head, 44)?;
        let long_loca = i16_at(head, 50)? != 0;

        let maxp = table(b"maxp").unwrap();
        let num_glyphs = u16_at(maxp, 4)?;

        let hhea = table(b"hhea").unwrap();
        let hhea_asc = i16_at(hhea, 4)?;
        let hhea_desc = i16_at(hhea, 6)?;
        let hhea_gap = i16_at(hhea, 8)?;
        let num_hmetrics = usize::from(u16_at(hhea, 34)?);
        if num_hmetrics == 0 {
            return Err(Error::Malformed("numberOfHMetrics is 0".into()));
        }

        let hmtx = table(b"hmtx").unwrap();
        let mut hmetrics = Vec::with_capacity(usize::from(num_glyphs));
        let mut last_adv = 0u16;
        for g in 0..usize::from(num_glyphs) {
            if g < num_hmetrics {
                last_adv = u16_at(hmtx, 4 * g)?;
                hmetrics.push((last_adv, i16_at(hmtx, 4 * g + 2)?));
            } else {
                let lsb = i16_at(hmtx, 4 * num_hmetrics + 2 * (g - num_hmetrics))?;
                hmetrics.push((last_adv, lsb));
            }
        }

        let mut loca = Vec::new();
        if outlines == Outlines::Glyf {
            let loca_t = table(b"loca").unwrap();
            loca.reserve(usize::from(num_glyphs) + 1);
            for g in 0..=usize::from(num_glyphs) {
                loca.push(if long_loca {
                    u32_at(loca_t, 4 * g)?
                } else {
                    u32::from(u16_at(loca_t, 2 * g)?) * 2
                });
            }
            let glyf_len = tables[b"glyf"].1;
            for w in loca.windows(2) {
                if w[0] > w[1] || w[1] as usize > glyf_len {
                    return Err(Error::Malformed(
                        "loca offsets out of order or past glyf".into(),
                    ));
                }
            }
        }

        // `cmap` is optional so that glyph-addressed subset programs (which
        // deliberately omit it) can be re-parsed and verified.
        let cmap = match table(b"cmap") {
            Some(t) => parse_cmap(t)?,
            None => BTreeMap::new(),
        };

        // OS/2 and post are optional in spirit; use hhea when absent.
        let mut metrics = VerticalMetrics {
            ascender: hhea_asc,
            descender: hhea_desc,
            line_gap: hhea_gap,
            cap_height: 0,
            x_height: 0,
            cap_height_declared: false,
            x_height_declared: false,
        };
        let mut weight = 400u16;
        let mut fs_type = 0u16;
        let mut os2_italic = false;
        let mut os2_oblique = false;
        if let Some(os2) = table(b"OS/2") {
            let version = u16_at(os2, 0)?;
            weight = u16_at(os2, 4)?;
            fs_type = u16_at(os2, 8)?;
            let fs_selection = u16_at(os2, 62)?;
            os2_italic = fs_selection & 0x0001 != 0;
            os2_oblique = fs_selection & 0x0200 != 0;
            let typo_asc = i16_at(os2, 68)?;
            let typo_desc = i16_at(os2, 70)?;
            let typo_gap = i16_at(os2, 72)?;
            let use_typo = fs_selection & 0x0080 != 0;
            if use_typo {
                metrics.ascender = typo_asc;
                metrics.descender = typo_desc;
                metrics.line_gap = typo_gap;
            }
            if version >= 2 && os2.len() >= 90 {
                metrics.x_height = i16_at(os2, 86)?;
                metrics.cap_height = i16_at(os2, 88)?;
                metrics.x_height_declared = metrics.x_height != 0;
                metrics.cap_height_declared = metrics.cap_height != 0;
            }
        }
        let mut italic_angle = 0.0;
        let mut is_fixed_pitch = false;
        if let Some(post) = table(b"post") {
            italic_angle = f64::from(i32_at(post, 4)?) / 65536.0;
            is_fixed_pitch = u32_at(post, 12)? != 0;
        }
        // Derive undeclared heights from the 'H' / 'x' glyph yMax when possible
        // (glyf only; CFF charstrings are not parsed).
        if outlines == Outlines::Glyf
            && !metrics.cap_height_declared
            && let Some(&g) = cmap.get(&u32::from(b'H'))
            && let Some(y) = glyph_y_max(b, &tables, &loca, g)
        {
            metrics.cap_height = y;
        }
        if outlines == Outlines::Glyf
            && !metrics.x_height_declared
            && let Some(&g) = cmap.get(&u32::from(b'x'))
            && let Some(y) = glyph_y_max(b, &tables, &loca, g)
        {
            metrics.x_height = y;
        }

        let (family, postscript_name) = table(b"name")
            .map(parse_names)
            .transpose()?
            .unwrap_or((None, None));
        let family = family.unwrap_or_else(|| "unknown".to_string());
        let postscript_name = postscript_name.unwrap_or_else(|| family.replace(' ', ""));

        let style = if os2_oblique {
            Style::Oblique
        } else if os2_italic || mac_style & 0x0002 != 0 || italic_angle != 0.0 {
            Style::Italic
        } else {
            Style::Upright
        };

        let mut unsupported = Vec::new();
        let kern = match table(b"kern") {
            Some(t) => match KernTable::parse(t) {
                Ok(k) => k,
                Err(Error::Unsupported(d)) => {
                    unsupported.push(Unsupported {
                        table: "kern",
                        feature: "kern",
                        detail: d,
                    });
                    KernTable::default()
                }
                Err(e) => return Err(e),
            },
            None => KernTable::default(),
        };
        let (gpos, marks) = match table(b"GPOS") {
            Some(t) => (GposKerning::parse(t)?, MarkAttachment::parse(t)?),
            None => (GposKerning::default(), MarkAttachment::default()),
        };
        let gsub = match table(b"GSUB") {
            Some(t) => GsubLigatures::parse(t)?,
            None => GsubLigatures::default(),
        };
        unsupported.extend(kern.unsupported.iter().cloned());
        unsupported.extend(gpos.unsupported.iter().cloned());
        unsupported.extend(marks.unsupported.iter().cloned());
        unsupported.extend(gsub.unsupported.iter().cloned());
        let math = match table(b"MATH") {
            Some(t) => Some(MathTable::parse(t)?),
            None => None,
        };
        if outlines == Outlines::Cff {
            unsupported.push(Unsupported {
                table: "CFF ",
                feature: "outlines",
                detail: "CFF outlines are exposed raw; glyph parsing, bbox derivation and \
                         subsetting are not implemented (embed the whole program)"
                    .into(),
            });
        }

        let mut hash_input = data.clone();
        hash_input.extend_from_slice(&face_index.to_be_bytes());
        let content_sha256 = crate::sha256::digest(&hash_input);

        Ok(TrueTypeFace {
            id: FontId {
                family,
                weight,
                style,
                source,
                content_sha256,
            },
            data,
            outlines,
            tables,
            units_per_em,
            num_glyphs,
            bbox,
            hmetrics,
            loca,
            cmap,
            metrics,
            italic_angle,
            is_fixed_pitch,
            postscript_name,
            fs_type,
            kern,
            gpos,
            marks,
            gsub,
            math,
            unsupported,
        })
    }

    pub fn outlines(&self) -> Outlines {
        self.outlines
    }

    /// The raw `CFF ` table for CFF-based faces (`FontFile3` payload).
    pub fn cff_table(&self) -> Option<&[u8]> {
        match self.outlines {
            Outlines::Cff => self.table(b"CFF "),
            Outlines::Glyf => None,
        }
    }

    /// OpenType `MATH` table data, if the face has one.
    pub fn math(&self) -> Option<&MathTable> {
        self.math.as_ref()
    }

    /// Raw bytes of a table, if present.
    pub fn table(&self, tag: &[u8; 4]) -> Option<&[u8]> {
        self.tables.get(tag).map(|(o, l)| &self.data[*o..*o + *l])
    }

    pub fn has_table(&self, tag: &[u8; 4]) -> bool {
        self.tables.contains_key(tag)
    }

    /// Raw `glyf` data of one glyph (empty for glyphs without outlines).
    /// CFF faces return [`Error::Unsupported`].
    pub fn glyph_data(&self, gid: GlyphId) -> Result<&[u8], Error> {
        if self.outlines == Outlines::Cff {
            return Err(Error::Unsupported(
                "CFF glyph data is not parsed; use cff_table()".into(),
            ));
        }
        let g = usize::from(gid.0);
        if g >= usize::from(self.num_glyphs) {
            return Err(Error::GlyphOutOfRange(gid.0));
        }
        let (o, _) = self.tables[b"glyf"];
        Ok(&self.data[o + self.loca[g] as usize..o + self.loca[g + 1] as usize])
    }

    /// (advance, left side bearing) in font units.
    pub fn hmetric(&self, gid: GlyphId) -> Result<(u16, i16), Error> {
        self.hmetrics
            .get(usize::from(gid.0))
            .copied()
            .ok_or(Error::GlyphOutOfRange(gid.0))
    }

    /// The legacy `kern` table's value for the pair even when GPOS is
    /// preferred; for diagnostics and cross-checks.
    pub fn legacy_kern_table_kerning(&self, left: GlyphId, right: GlyphId) -> Option<i16> {
        self.kern.kerning(left, right)
    }

    /// Whether the face has GPOS MarkToBase data.
    pub fn has_mark_attachment(&self) -> bool {
        !self.marks.is_empty()
    }

    pub fn has_gsub_ligatures(&self) -> bool {
        !self.gsub.is_empty()
    }

    /// The full character map (code point -> original glyph id).
    pub fn char_map(&self) -> &BTreeMap<u32, u16> {
        &self.cmap
    }

    /// Whole font program bytes (for hashing or re-embedding unsubsetted).
    pub fn program(&self) -> &[u8] {
        &self.data
    }
}

fn glyph_y_max(
    b: &[u8],
    tables: &BTreeMap<[u8; 4], (usize, usize)>,
    loca: &[u32],
    gid: u16,
) -> Option<i16> {
    let (o, _) = tables[b"glyf"];
    let g = usize::from(gid);
    let start = o + *loca.get(g)? as usize;
    let end = o + *loca.get(g + 1)? as usize;
    if end - start < 10 {
        return None;
    }
    i16_at(b, start + 8).ok()
}

impl Face for TrueTypeFace {
    fn id(&self) -> &FontId {
        &self.id
    }

    fn units_per_em(&self) -> u16 {
        self.units_per_em
    }

    fn num_glyphs(&self) -> u16 {
        self.num_glyphs
    }

    fn vertical_metrics(&self) -> VerticalMetrics {
        self.metrics
    }

    fn bbox(&self) -> [i16; 4] {
        self.bbox
    }

    fn italic_angle(&self) -> f64 {
        self.italic_angle
    }

    fn advance(&self, gid: GlyphId) -> Result<u16, Error> {
        self.hmetric(gid).map(|(a, _)| a)
    }

    fn glyph_id(&self, ch: char) -> Option<GlyphId> {
        self.cmap
            .get(&(ch as u32))
            .copied()
            .filter(|g| *g != 0)
            .map(GlyphId)
    }

    fn kerning(&self, left: GlyphId, right: GlyphId) -> (i16, KerningSource) {
        // GPOS supersedes the legacy table when it carries a kern feature,
        // which is how OpenType-aware renderers behave.
        if !self.gpos.is_empty() {
            return (
                self.gpos.kerning(left, right).unwrap_or(0),
                KerningSource::Gpos,
            );
        }
        if let Some(v) = self.kern.kerning(left, right) {
            return (v, KerningSource::KernTable);
        }
        (0, KerningSource::None)
    }

    /// GPOS is preferred over the legacy table when it carries a kern feature.
    fn kerning_source(&self) -> KerningSource {
        if !self.gpos.is_empty() {
            KerningSource::Gpos
        } else if !self.kern.is_empty() {
            KerningSource::KernTable
        } else {
            KerningSource::None
        }
    }

    fn ligature(&self, components: &[GlyphId]) -> Option<GlyphId> {
        self.gsub.ligature(components)
    }

    fn mark_attachment(&self, base: GlyphId, mark: GlyphId) -> Option<(i16, i16)> {
        self.marks.attach(base, mark)
    }

    fn ligature_passes(&self) -> usize {
        self.gsub.passes()
    }

    fn longest_ligature(&self, pass: usize, glyphs: &[GlyphId]) -> Option<(GlyphId, usize)> {
        self.gsub.longest_in_pass(pass, glyphs)
    }

    fn unsupported(&self) -> &[Unsupported] {
        &self.unsupported
    }

    fn postscript_name(&self) -> &str {
        &self.postscript_name
    }

    fn is_fixed_pitch(&self) -> bool {
        self.is_fixed_pitch
    }
}

/// Parses `cmap`, preferring a Unicode full-repertoire format 12 subtable,
/// then a BMP format 4 subtable. Other formats are ignored, but at least one
/// usable Unicode subtable must exist.
fn parse_cmap(cmap: &[u8]) -> Result<BTreeMap<u32, u16>, Error> {
    let n = usize::from(u16_at(cmap, 2)?);
    let mut best: Option<(u8, usize)> = None; // (rank, offset)
    for i in 0..n {
        let rec = 4 + 8 * i;
        let platform = u16_at(cmap, rec)?;
        let encoding = u16_at(cmap, rec + 2)?;
        let off = u32_at(cmap, rec + 4)? as usize;
        let format = u16_at(cmap, off)?;
        let rank = match (platform, encoding, format) {
            (3, 10, 12) => 5,
            (0, _, 12) => 4,
            (3, 1, 4) => 3,
            (0, _, 4) => 2,
            _ => continue,
        };
        if best.is_none_or(|(r, _)| rank > r) {
            best = Some((rank, off));
        }
    }
    let Some((_, off)) = best else {
        return Err(Error::Unsupported(
            "cmap has no Unicode format 4 or 12 subtable".into(),
        ));
    };
    let mut map = BTreeMap::new();
    match u16_at(cmap, off)? {
        4 => {
            let seg_x2 = usize::from(u16_at(cmap, off + 6)?);
            let segs = seg_x2 / 2;
            let ends = off + 14;
            let starts = ends + seg_x2 + 2;
            let deltas = starts + seg_x2;
            let range_offsets = deltas + seg_x2;
            for s in 0..segs {
                let end = u16_at(cmap, ends + 2 * s)?;
                let start = u16_at(cmap, starts + 2 * s)?;
                let delta = u16_at(cmap, deltas + 2 * s)?;
                let ro = u16_at(cmap, range_offsets + 2 * s)?;
                if start > end {
                    return Err(Error::Malformed("cmap segment start > end".into()));
                }
                for c in start..=end {
                    if c == 0xFFFF {
                        break;
                    }
                    let g = if ro == 0 {
                        c.wrapping_add(delta)
                    } else {
                        let addr =
                            range_offsets + 2 * s + usize::from(ro) + 2 * usize::from(c - start);
                        let raw = u16_at(cmap, addr)?;
                        if raw == 0 { 0 } else { raw.wrapping_add(delta) }
                    };
                    if g != 0 {
                        map.entry(u32::from(c)).or_insert(g);
                    }
                }
            }
        }
        12 => {
            let n_groups = u32_at(cmap, off + 12)? as usize;
            for i in 0..n_groups {
                let rec = off + 16 + 12 * i;
                let start = u32_at(cmap, rec)?;
                let end = u32_at(cmap, rec + 4)?;
                let start_gid = u32_at(cmap, rec + 8)?;
                if start > end || end > 0x10FFFF {
                    return Err(Error::Malformed("cmap group range".into()));
                }
                for c in start..=end {
                    let g = start_gid + (c - start);
                    if g != 0 && g <= 0xFFFF {
                        map.entry(c).or_insert(g as u16);
                    }
                }
            }
        }
        _ => unreachable!(),
    }
    Ok(map)
}

/// (family name id 1, PostScript name id 6), preferring Windows Unicode
/// records and falling back to Macintosh Roman.
fn parse_names(name: &[u8]) -> Result<(Option<String>, Option<String>), Error> {
    let count = usize::from(u16_at(name, 2)?);
    let storage = usize::from(u16_at(name, 4)?);
    let mut family: Option<(u8, String)> = None;
    let mut ps: Option<(u8, String)> = None;
    for i in 0..count {
        let rec = 6 + 12 * i;
        let platform = u16_at(name, rec)?;
        let encoding = u16_at(name, rec + 2)?;
        let id = u16_at(name, rec + 6)?;
        let len = usize::from(u16_at(name, rec + 8)?);
        let off = storage + usize::from(u16_at(name, rec + 10)?);
        if id != 1 && id != 6 {
            continue;
        }
        let Ok(bytes) = slice(name, off, len) else {
            continue;
        };
        let (rank, text) = match (platform, encoding) {
            (3, 1) | (3, 10) | (0, _) => {
                let units: Vec<u16> = bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| u16::from_be_bytes(*c))
                    .collect();
                (2u8, String::from_utf16_lossy(&units))
            }
            (1, 0) => (1u8, bytes.iter().map(|b| *b as char).collect()),
            _ => continue,
        };
        let slot = if id == 1 { &mut family } else { &mut ps };
        if slot.as_ref().is_none_or(|(r, _)| rank > *r) {
            *slot = Some((rank, text));
        }
    }
    Ok((family.map(|(_, s)| s), ps.map(|(_, s)| s)))
}

/// Component glyph references of a composite glyph; empty for simple glyphs.
/// Each entry is (component gid, byte offset of that gid within `data`).
pub(crate) fn composite_components(data: &[u8]) -> Result<Vec<(u16, usize)>, Error> {
    if data.len() < 10 || i16_at(data, 0)? >= 0 {
        return Ok(Vec::new());
    }
    let mut parts = Vec::new();
    let mut at = 10;
    loop {
        let flags = u16_at(data, at)?;
        parts.push((u16_at(data, at + 2)?, at + 2));
        at += 4;
        at += if flags & 0x0001 != 0 { 4 } else { 2 };
        at += if flags & 0x0008 != 0 {
            2
        } else if flags & 0x0040 != 0 {
            4
        } else if flags & 0x0080 != 0 {
            8
        } else {
            0
        };
        if flags & 0x0020 == 0 {
            break;
        }
    }
    Ok(parts)
}
