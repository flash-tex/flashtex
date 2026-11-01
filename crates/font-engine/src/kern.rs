//! Legacy `kern` table, format 0 subtables only (horizontal, non-cross-stream).
//! Both the Microsoft (`version 0`, u16 count) and Apple (`version 1.0`,
//! u32 count) headers are recognised; format 1/2/3 subtables and vertical or
//! cross-stream subtables are recorded as unsupported.

use crate::Unsupported;
use crate::reader::{i16_at, u16_at, u32_at};
use crate::{Error, GlyphId};

#[derive(Debug, Clone, Default)]
pub struct KernTable {
    /// Sorted by (left, right); first occurrence wins across subtables.
    pairs: Vec<(u16, u16, i16)>,
    pub unsupported: Vec<Unsupported>,
}

impl KernTable {
    pub fn parse(b: &[u8]) -> Result<KernTable, Error> {
        let mut out = KernTable::default();
        let version = u16_at(b, 0)?;
        let (mut at, n_tables, apple) = if version == 0 {
            (4usize, usize::from(u16_at(b, 2)?), false)
        } else if version == 1 && u16_at(b, 2)? == 0 {
            (8usize, u32_at(b, 4)? as usize, true)
        } else {
            return Err(Error::Unsupported(format!("kern table version {version}")));
        };
        for _ in 0..n_tables {
            let (length, coverage, format, horizontal) = if apple {
                let length = u32_at(b, at)? as usize;
                let coverage = u16_at(b, at + 4)?;
                // Apple: bit 15 vertical, bit 14 cross-stream; format in low byte.
                let horizontal = coverage & 0x8000 == 0 && coverage & 0x4000 == 0;
                (length, coverage, (coverage & 0xFF) as u8, horizontal)
            } else {
                let length = usize::from(u16_at(b, at + 2)?);
                let coverage = u16_at(b, at + 4)?;
                // Microsoft: bit 0 horizontal, bit 2 cross-stream; format in high byte.
                let horizontal = coverage & 0x0001 != 0 && coverage & 0x0004 == 0;
                (length, coverage, (coverage >> 8) as u8, horizontal)
            };
            if length == 0 {
                return Err(Error::Malformed("kern subtable of length 0".into()));
            }
            let header = if apple { 8 } else { 6 };
            if format == 0 && horizontal {
                let n = usize::from(u16_at(b, at + header)?);
                let mut rec = at + header + 8;
                for _ in 0..n {
                    let l = u16_at(b, rec)?;
                    let r = u16_at(b, rec + 2)?;
                    let v = i16_at(b, rec + 4)?;
                    out.pairs.push((l, r, v));
                    rec += 6;
                }
            } else {
                out.unsupported.push(Unsupported {
                    table: "kern",
                    feature: "kern",
                    detail: format!(
                        "subtable format {format} coverage 0x{coverage:04X} not applied"
                    ),
                });
            }
            at += length;
        }
        out.pairs.sort_by_key(|(l, r, _)| (*l, *r));
        out.pairs.dedup_by_key(|(l, r, _)| (*l, *r));
        Ok(out)
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    pub fn kerning(&self, left: GlyphId, right: GlyphId) -> Option<i16> {
        self.pairs
            .binary_search_by_key(&(left.0, right.0), |(l, r, _)| (*l, *r))
            .ok()
            .map(|i| self.pairs[i].2)
    }
}
