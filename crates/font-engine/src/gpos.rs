//! `GPOS` pair adjustment: the `kern` feature's lookups of type 2 (PairPos
//! format 1 and 2), including type 9 Extension wrappers around type 2.
//! Every other lookup type under `kern` is recorded as unsupported.
//!
//! Only the first glyph's X advance adjustment (`valueFormat1` bit 0x0004)
//! is applied, which is what horizontal Latin kerning uses. Placement
//! adjustments and second-glyph records are parsed past but ignored, and
//! that is stated in the README.

use crate::Unsupported;
use crate::otl::{self, ClassDef, Coverage};
use crate::reader::{i16_at, u16_at};
use crate::{Error, GlyphId};

#[derive(Debug, Clone)]
enum PairSubtable {
    /// Format 1: coverage on the first glyph; per-first-glyph sorted pair sets
    /// of (second glyph, x_advance_1).
    Format1 {
        coverage: Coverage,
        pair_sets: Vec<Vec<(u16, i16)>>,
    },
    /// Format 2: class-based matrix of x_advance_1.
    Format2 {
        coverage: Coverage,
        class1: ClassDef,
        class2: ClassDef,
        class2_count: u16,
        values: Vec<i16>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct GposKerning {
    subtables: Vec<PairSubtable>,
    pub unsupported: Vec<Unsupported>,
}

fn value_record_len(format: u16) -> usize {
    2 * usize::from(format.count_ones() as u16)
}

/// Reads the XAdvance field of a value record with `format` at `at`.
fn x_advance(b: &[u8], at: usize, format: u16) -> Result<i16, Error> {
    if format & 0x0004 == 0 {
        return Ok(0);
    }
    let mut off = at;
    if format & 0x0001 != 0 {
        off += 2;
    }
    if format & 0x0002 != 0 {
        off += 2;
    }
    i16_at(b, off)
}

impl GposKerning {
    /// Parses the `kern` feature of a `GPOS` table. Returns an empty set (not
    /// an error) when the table has no `kern` feature.
    pub fn parse(gpos: &[u8]) -> Result<GposKerning, Error> {
        let mut out = GposKerning::default();
        let indices = otl::lookups_for_feature(gpos, b"kern")?;
        for index in indices {
            let lk = otl::lookup(gpos, index, 9)?;
            if lk.lookup_type != 2 {
                out.unsupported.push(Unsupported {
                    table: "GPOS",
                    feature: "kern",
                    detail: format!(
                        "kern lookup {} is type {} (only PairPos type 2 is applied)",
                        lk.index, lk.lookup_type
                    ),
                });
                continue;
            }
            for st in lk.subtables {
                match parse_pair_subtable(gpos, st) {
                    Ok(sub) => out.subtables.push(sub),
                    Err(Error::Unsupported(detail)) => out.unsupported.push(Unsupported {
                        table: "GPOS",
                        feature: "kern",
                        detail,
                    }),
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(out)
    }

    pub fn is_empty(&self) -> bool {
        self.subtables.is_empty()
    }

    /// First applicable subtable wins, as OpenType lookup processing does.
    pub fn kerning(&self, left: GlyphId, right: GlyphId) -> Option<i16> {
        for sub in &self.subtables {
            match sub {
                PairSubtable::Format1 {
                    coverage,
                    pair_sets,
                } => {
                    let Some(ci) = coverage.index(left.0) else {
                        continue;
                    };
                    let set = pair_sets.get(usize::from(ci))?;
                    if let Ok(i) = set.binary_search_by_key(&right.0, |(g, _)| *g) {
                        return Some(set[i].1);
                    }
                }
                PairSubtable::Format2 {
                    coverage,
                    class1,
                    class2,
                    class2_count,
                    values,
                } => {
                    if coverage.index(left.0).is_none() {
                        continue;
                    }
                    let c1 = usize::from(class1.class(left.0));
                    let c2 = usize::from(class2.class(right.0));
                    let v = values.get(c1 * usize::from(*class2_count) + c2).copied();
                    // Class 0 means "not in any class"; a zero record there is
                    // no match, so a later subtable may still apply (this is
                    // how mainstream shapers treat it).
                    if (c1 == 0 || c2 == 0) && v == Some(0) {
                        continue;
                    }
                    return v;
                }
            }
        }
        None
    }
}

fn parse_pair_subtable(b: &[u8], at: usize) -> Result<PairSubtable, Error> {
    let format = u16_at(b, at)?;
    let coverage = Coverage::parse(b, at + usize::from(u16_at(b, at + 2)?))?;
    let vf1 = u16_at(b, at + 4)?;
    let vf2 = u16_at(b, at + 6)?;
    let rec_len = value_record_len(vf1) + value_record_len(vf2);
    match format {
        1 => {
            let n = usize::from(u16_at(b, at + 8)?);
            let mut pair_sets = Vec::with_capacity(n);
            for i in 0..n {
                let ps = at + usize::from(u16_at(b, at + 10 + 2 * i)?);
                let count = usize::from(u16_at(b, ps)?);
                let mut set = Vec::with_capacity(count);
                for j in 0..count {
                    let rec = ps + 2 + j * (2 + rec_len);
                    set.push((u16_at(b, rec)?, x_advance(b, rec + 2, vf1)?));
                }
                // PairValueRecords are specified sorted by second glyph; sort
                // defensively so the binary search is valid regardless.
                set.sort_by_key(|(g, _)| *g);
                pair_sets.push(set);
            }
            Ok(PairSubtable::Format1 {
                coverage,
                pair_sets,
            })
        }
        2 => {
            let class1 = ClassDef::parse(b, at + usize::from(u16_at(b, at + 8)?))?;
            let class2 = ClassDef::parse(b, at + usize::from(u16_at(b, at + 10)?))?;
            let class1_count = usize::from(u16_at(b, at + 12)?);
            let class2_count = u16_at(b, at + 14)?;
            let mut values = Vec::with_capacity(class1_count * usize::from(class2_count));
            let mut rec = at + 16;
            for _ in 0..class1_count * usize::from(class2_count) {
                values.push(x_advance(b, rec, vf1)?);
                rec += rec_len;
            }
            Ok(PairSubtable::Format2 {
                coverage,
                class1,
                class2,
                class2_count,
                values,
            })
        }
        f => Err(Error::Unsupported(format!("PairPos format {f}"))),
    }
}

// ------------------------------------------------------------ MarkToBase

#[derive(Debug, Clone)]
struct MarkBaseSubtable {
    mark_coverage: Coverage,
    base_coverage: Coverage,
    class_count: u16,
    /// Per mark coverage index: (class, anchor x, anchor y).
    marks: Vec<(u16, i16, i16)>,
    /// Per base coverage index: per class `Some((x, y))` or `None`.
    bases: Vec<Vec<Option<(i16, i16)>>>,
}

/// The `mark` feature's MarkToBase (lookup type 4) attachment data.
/// MarkToLigature (5) and MarkToMark (6) are recorded as unsupported.
#[derive(Debug, Clone, Default)]
pub struct MarkAttachment {
    subtables: Vec<MarkBaseSubtable>,
    pub unsupported: Vec<Unsupported>,
}

fn anchor_xy(b: &[u8], at: usize) -> Result<(i16, i16), Error> {
    // Anchor formats 1, 2 and 3 all start with format, xCoordinate, yCoordinate.
    let format = u16_at(b, at)?;
    if !(1..=3).contains(&format) {
        return Err(Error::Malformed(format!("anchor format {format}")));
    }
    Ok((i16_at(b, at + 2)?, i16_at(b, at + 4)?))
}

impl MarkAttachment {
    pub fn parse(gpos: &[u8]) -> Result<MarkAttachment, Error> {
        let mut out = MarkAttachment::default();
        for index in otl::lookups_for_feature(gpos, b"mark")? {
            let lk = otl::lookup(gpos, index, 9)?;
            if lk.lookup_type != 4 {
                out.unsupported.push(Unsupported {
                    table: "GPOS",
                    feature: "mark",
                    detail: format!(
                        "mark lookup {} is type {} (only MarkToBase type 4 is applied)",
                        lk.index, lk.lookup_type
                    ),
                });
                continue;
            }
            for st in lk.subtables {
                out.subtables.push(parse_mark_base(gpos, st)?);
            }
        }
        Ok(out)
    }

    pub fn is_empty(&self) -> bool {
        self.subtables.is_empty()
    }

    /// Offset (dx, dy) in font units from the BASE glyph's origin at which
    /// `mark` should be drawn, from the first subtable covering both.
    pub fn attach(&self, base: GlyphId, mark: GlyphId) -> Option<(i16, i16)> {
        for sub in &self.subtables {
            let (Some(mi), Some(bi)) = (
                sub.mark_coverage.index(mark.0),
                sub.base_coverage.index(base.0),
            ) else {
                continue;
            };
            let (class, mx, my) = *sub.marks.get(usize::from(mi))?;
            if class >= sub.class_count {
                continue;
            }
            let Some((bx, by)) = sub.bases.get(usize::from(bi))?.get(usize::from(class))? else {
                continue;
            };
            return Some((bx - mx, by - my));
        }
        None
    }
}

fn parse_mark_base(b: &[u8], at: usize) -> Result<MarkBaseSubtable, Error> {
    if u16_at(b, at)? != 1 {
        return Err(Error::Malformed("MarkBasePos format".into()));
    }
    let mark_coverage = Coverage::parse(b, at + usize::from(u16_at(b, at + 2)?))?;
    let base_coverage = Coverage::parse(b, at + usize::from(u16_at(b, at + 4)?))?;
    let class_count = u16_at(b, at + 6)?;
    let mark_array = at + usize::from(u16_at(b, at + 8)?);
    let base_array = at + usize::from(u16_at(b, at + 10)?);
    let n_marks = usize::from(u16_at(b, mark_array)?);
    let mut marks = Vec::with_capacity(n_marks);
    for i in 0..n_marks {
        let rec = mark_array + 2 + 4 * i;
        let class = u16_at(b, rec)?;
        let anchor = mark_array + usize::from(u16_at(b, rec + 2)?);
        let (x, y) = anchor_xy(b, anchor)?;
        marks.push((class, x, y));
    }
    let n_bases = usize::from(u16_at(b, base_array)?);
    let mut bases = Vec::with_capacity(n_bases);
    for i in 0..n_bases {
        let rec = base_array + 2 + 2 * usize::from(class_count) * i;
        let mut anchors = Vec::with_capacity(usize::from(class_count));
        for c in 0..usize::from(class_count) {
            let off = u16_at(b, rec + 2 * c)?;
            anchors.push(if off == 0 {
                None
            } else {
                Some(anchor_xy(b, base_array + usize::from(off))?)
            });
        }
        bases.push(anchors);
    }
    Ok(MarkBaseSubtable {
        mark_coverage,
        base_coverage,
        class_count,
        marks,
        bases,
    })
}
