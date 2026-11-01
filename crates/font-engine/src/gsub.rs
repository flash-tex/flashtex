//! `GSUB` ligatures: the `liga` feature's lookups of type 4 (LigatureSubst),
//! including type 7 Extension wrappers. Other lookup types under `liga`
//! (contextual, chained, single, multiple, alternate) are recorded as
//! unsupported; no other feature (`dlig`, `clig`, `calt`, `smcp`, ...) is
//! read at all.

use crate::Unsupported;
use crate::otl::{self, Coverage};
use crate::reader::u16_at;
use crate::{Error, GlyphId};

#[derive(Debug, Clone)]
struct LigatureSubtable {
    coverage: Coverage,
    /// Per coverage index: (ligature glyph, component glyphs after the first),
    /// in font order (the font lists longer ligatures first when it wants
    /// them preferred; we honour font order).
    sets: Vec<Vec<(u16, Vec<u16>)>>,
}

/// Ligature lookups in LookupList order. Each lookup is one shaping pass
/// over the whole run (OpenType semantics), so a font can build `ffi` as
/// `f`+`f` -> `ff` in one lookup and `ff`+`i` -> `ffi` in the next.
#[derive(Debug, Clone, Default)]
pub struct GsubLigatures {
    lookups: Vec<Vec<LigatureSubtable>>,
    pub unsupported: Vec<Unsupported>,
}

impl GsubLigatures {
    pub fn parse(gsub: &[u8]) -> Result<GsubLigatures, Error> {
        let mut out = GsubLigatures::default();
        for index in otl::lookups_for_feature(gsub, b"liga")? {
            let lk = otl::lookup(gsub, index, 7)?;
            if lk.lookup_type != 4 {
                out.unsupported.push(Unsupported {
                    table: "GSUB",
                    feature: "liga",
                    detail: format!(
                        "liga lookup {} is type {} (only LigatureSubst type 4 is applied)",
                        lk.index, lk.lookup_type
                    ),
                });
                continue;
            }
            let mut subtables = Vec::with_capacity(lk.subtables.len());
            for st in lk.subtables {
                subtables.push(parse_ligature_subtable(gsub, st)?);
            }
            out.lookups.push(subtables);
        }
        Ok(out)
    }

    pub fn is_empty(&self) -> bool {
        self.lookups.is_empty()
    }

    /// Number of passes (lookups).
    pub fn passes(&self) -> usize {
        self.lookups.len()
    }

    /// Result of running every pass over exactly `components`; `Some` only
    /// when they collapse to a single glyph.
    pub fn ligature(&self, components: &[GlyphId]) -> Option<GlyphId> {
        let mut run: Vec<GlyphId> = components.to_vec();
        for pass in 0..self.passes() {
            let mut i = 0;
            while i < run.len() {
                if let Some((lig, len)) = self.longest_in_pass(pass, &run[i..]) {
                    run.splice(i..i + len, [lig]);
                }
                i += 1;
            }
        }
        (run.len() == 1 && components.len() > 1).then(|| run[0])
    }

    /// Longest ligature of pass `pass` starting at `glyphs[0]`; returns
    /// (ligature, component count).
    pub fn longest_in_pass(&self, pass: usize, glyphs: &[GlyphId]) -> Option<(GlyphId, usize)> {
        let first = glyphs.first()?;
        let mut best: Option<(GlyphId, usize)> = None;
        for sub in self.lookups.get(pass)? {
            let Some(ci) = sub.coverage.index(first.0) else {
                continue;
            };
            for (lig, rest) in &sub.sets[usize::from(ci)] {
                let len = rest.len() + 1;
                if len <= glyphs.len()
                    && len >= 2
                    && rest.iter().zip(&glyphs[1..]).all(|(a, b)| *a == b.0)
                    && best.is_none_or(|(_, l)| len > l)
                {
                    best = Some((GlyphId(*lig), len));
                }
            }
            if best.is_some() {
                // First subtable covering the glyph decides within a lookup.
                return best;
            }
        }
        best
    }
}

fn parse_ligature_subtable(b: &[u8], at: usize) -> Result<LigatureSubtable, Error> {
    let format = u16_at(b, at)?;
    if format != 1 {
        return Err(Error::Malformed(format!("LigatureSubst format {format}")));
    }
    let coverage = Coverage::parse(b, at + usize::from(u16_at(b, at + 2)?))?;
    let n = usize::from(u16_at(b, at + 4)?);
    let mut sets = Vec::with_capacity(n);
    for i in 0..n {
        let set = at + usize::from(u16_at(b, at + 6 + 2 * i)?);
        let count = usize::from(u16_at(b, set)?);
        let mut ligs = Vec::with_capacity(count);
        for j in 0..count {
            let lig = set + usize::from(u16_at(b, set + 2 + 2 * j)?);
            let lig_glyph = u16_at(b, lig)?;
            let comp_count = usize::from(u16_at(b, lig + 2)?);
            if comp_count == 0 {
                return Err(Error::Malformed("ligature with zero components".into()));
            }
            let mut rest = Vec::with_capacity(comp_count - 1);
            for k in 0..comp_count - 1 {
                rest.push(u16_at(b, lig + 4 + 2 * k)?);
            }
            ligs.push((lig_glyph, rest));
        }
        sets.push(ligs);
    }
    Ok(LigatureSubtable { coverage, sets })
}
