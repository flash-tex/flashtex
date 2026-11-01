//! Original bounded MATH Variants decoder. Constants are handled by font-engine.
//! Coverage is local because the peer's coverage module is private.
use crate::{invalid, Result};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Vertical,
    Horizontal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Variant {
    pub glyph_id: u16,
    pub advance: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Part {
    pub glyph_id: u16,
    pub start_connector: u16,
    pub end_connector: u16,
    pub full_advance: u16,
    pub extender: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assembly {
    pub italic_correction: i16,
    /// Present device data is not evaluated or validated.
    pub device_adjustment_present: bool,
    pub parts: Vec<Part>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Construction {
    pub variants: Vec<Variant>,
    pub assembly: Option<Assembly>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection<'a> {
    Variant(Variant),
    AssemblyRequired(&'a Assembly),
    Unavailable,
}
/// Immutable parse, bounded to 4096 constructions and 65536 aggregate records.
#[derive(Debug)]
pub struct MathVariants {
    min_overlap: u16,
    constructions: BTreeMap<(Direction, u16), Construction>,
}
pub(crate) fn u16at(b: &[u8], p: usize) -> Result<u16> {
    Ok(u16::from_be_bytes(
        b.get(p..p + 2)
            .ok_or_else(|| invalid("MATH variants truncated"))?
            .try_into()
            .unwrap(),
    ))
}
pub(crate) fn offset(b: &[u8], base: usize, p: usize) -> Result<usize> {
    let n = u16at(b, p)? as usize;
    if n == 0 {
        return Err(invalid("MATH required offset is zero"));
    }
    let at = base
        .checked_add(n)
        .ok_or_else(|| invalid("MATH offset overflow"))?;
    if at >= b.len() {
        return Err(invalid("MATH offset outside table"));
    }
    Ok(at)
}
fn gid(n: u16, count: u16) -> Result<u16> {
    if n >= count {
        Err(invalid("MATH variant GID out of bounds"))
    } else {
        Ok(n)
    }
}
pub(crate) fn coverage(b: &[u8], at: usize, expected: usize, glyph_count: u16) -> Result<Vec<u16>> {
    let count = u16at(b, at + 2)? as usize;
    if count > 4096 {
        return Err(crate::Error::SizeLimit);
    }
    let mut out = Vec::new();
    match u16at(b, at)? {
        1 => {
            if count != expected {
                return Err(invalid("MATH coverage count"));
            }
            for i in 0..count {
                out.push(gid(u16at(b, at + 4 + i * 2)?, glyph_count)?);
            }
        }
        2 => {
            for i in 0..count {
                let p = at + 4 + i * 6;
                let start = u16at(b, p)?;
                let end = gid(u16at(b, p + 2)?, glyph_count)?;
                if start > end
                    || u16at(b, p + 4)? as usize != out.len()
                    || out.len() + usize::from(end - start) + 1 > expected
                {
                    return Err(invalid("MATH coverage range/index"));
                }
                out.extend(start..=end);
            }
        }
        _ => return Err(invalid("MATH coverage format")),
    }
    if out.len() != expected || out.windows(2).any(|w| w[0] >= w[1]) {
        return Err(invalid("MATH coverage order/count"));
    }
    Ok(out)
}
impl MathVariants {
    pub fn parse(math: &[u8], glyph_count: u16) -> Result<Self> {
        if math.len() > 4 * 1024 * 1024 {
            return Err(crate::Error::SizeLimit);
        }
        if u16at(math, 0)? != 1 {
            return Err(crate::Error::UnsupportedFont("MATH major version".into()));
        }
        let raw = u16at(math, 8)? as usize;
        let mut result = Self {
            min_overlap: 0,
            constructions: BTreeMap::new(),
        };
        if raw == 0 {
            return Ok(result);
        }
        let base = offset(math, 0, 8)?;
        result.min_overlap = u16at(math, base)?;
        let vertical = u16at(math, base + 6)? as usize;
        let horizontal = u16at(math, base + 8)? as usize;
        if vertical + horizontal > 4096 {
            return Err(crate::Error::SizeLimit);
        }
        let mut records = 0usize;
        for (direction, count, coverage_pos, first) in [
            (Direction::Vertical, vertical, base + 2, 0),
            (Direction::Horizontal, horizontal, base + 4, vertical),
        ] {
            if count == 0 {
                if u16at(math, coverage_pos)? != 0 {
                    let at = offset(math, base, coverage_pos)?;
                    coverage(math, at, 0, glyph_count)?;
                }
                continue;
            }
            let covered = coverage(math, offset(math, base, coverage_pos)?, count, glyph_count)?;
            for (i, original) in covered.into_iter().enumerate() {
                let at = offset(math, base, base + 10 + (first + i) * 2)?;
                let n = u16at(math, at + 2)? as usize;
                records += n;
                if records > 65536 {
                    return Err(crate::Error::SizeLimit);
                }
                let mut variants = Vec::with_capacity(n);
                for j in 0..n {
                    let p = at + 4 + j * 4;
                    variants.push(Variant {
                        glyph_id: gid(u16at(math, p)?, glyph_count)?,
                        advance: u16at(math, p + 2)?,
                    });
                }
                if variants.windows(2).any(|w| w[0].advance > w[1].advance) {
                    return Err(invalid("MATH variants advance order"));
                }
                let assembly = if u16at(math, at)? == 0 {
                    None
                } else {
                    let a = offset(math, at, at)?;
                    let n = u16at(math, a + 4)? as usize;
                    records += n;
                    if records > 65536 {
                        return Err(crate::Error::SizeLimit);
                    }
                    if n == 0 {
                        return Err(invalid("MATH empty assembly"));
                    }
                    let device = u16at(math, a + 2)?;
                    if device != 0 {
                        offset(math, a, a + 2)?;
                    }
                    let mut parts = Vec::with_capacity(n);
                    for j in 0..n {
                        let p = a + 6 + j * 10;
                        let flags = u16at(math, p + 8)?;
                        if flags & !1 != 0 {
                            return Err(crate::Error::UnsupportedFont(
                                "MATH reserved part flags".into(),
                            ));
                        }
                        let part = Part {
                            glyph_id: gid(u16at(math, p)?, glyph_count)?,
                            start_connector: u16at(math, p + 2)?,
                            end_connector: u16at(math, p + 4)?,
                            full_advance: u16at(math, p + 6)?,
                            extender: flags == 1,
                        };
                        parts.push(part);
                    }
                    Some(Assembly {
                        italic_correction: u16at(math, a)? as i16,
                        device_adjustment_present: device != 0,
                        parts,
                    })
                };
                result
                    .constructions
                    .insert((direction, original), Construction { variants, assembly });
            }
        }
        Ok(result)
    }
    pub fn min_connector_overlap(&self) -> u16 {
        self.min_overlap
    }
    pub fn constructions(&self) -> &BTreeMap<(Direction, u16), Construction> {
        &self.constructions
    }
    /// Exact first declared variant meeting the requested design-unit advance.
    /// An assembly requirement is explicit; this does not synthesize geometry.
    pub fn select(&self, direction: Direction, glyph_id: u16, target: u32) -> Selection<'_> {
        let Some(c) = self.constructions.get(&(direction, glyph_id)) else {
            return Selection::Unavailable;
        };
        match c.variants.iter().find(|v| u32::from(v.advance) >= target) {
            Some(v) => Selection::Variant(*v),
            None => c
                .assembly
                .as_ref()
                .map(Selection::AssemblyRequired)
                .unwrap_or(Selection::Unavailable),
        }
    }
}

/// Exact caller-selected assembly. No automatic typography or device adjustment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedPart {
    pub glyph_id: u16,
    pub part_index: usize,
    pub instance: usize,
    pub offset: i64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblyPlacement {
    pub parts: Vec<PlacedPart>,
    pub advance: i64,
    pub italic_correction: i16,
}
impl MathVariants {
    /// Repeat every extender equally (zero removes them), applying one explicitly
    /// chosen overlap to each join. Caller controls the target-size policy.
    /// Units and order are unchanged: bottom-to-top or left-to-right.
    pub fn assemble(
        &self,
        direction: Direction,
        glyph_id: u16,
        extender_repetitions: usize,
        overlaps: &[u16],
    ) -> Result<AssemblyPlacement> {
        if extender_repetitions > 1024 || overlaps.len() > 4095 {
            return Err(crate::Error::SizeLimit);
        }
        let assembly = self
            .constructions
            .get(&(direction, glyph_id))
            .and_then(|c| c.assembly.as_ref())
            .ok_or_else(|| invalid("MATH assembly unavailable"))?;
        let mut selected = Vec::new();
        for (part_index, part) in assembly.parts.iter().enumerate() {
            let count = if part.extender {
                extender_repetitions
            } else {
                1
            };
            if selected.len() + count > 4096 {
                return Err(crate::Error::SizeLimit);
            }
            for instance in 0..count {
                selected.push((part_index, instance, part));
            }
        }
        if selected.is_empty() || overlaps.len() != selected.len() - 1 {
            return Err(invalid("MATH assembly join count"));
        }
        let mut parts = Vec::with_capacity(selected.len());
        let mut position = 0i64;
        for (i, &(part_index, instance, part)) in selected.iter().enumerate() {
            parts.push(PlacedPart {
                glyph_id: part.glyph_id,
                part_index,
                instance,
                offset: position,
            });
            position = position
                .checked_add(i64::from(part.full_advance))
                .ok_or_else(|| invalid("MATH assembly overflow"))?;
            if let Some(&overlap) = overlaps.get(i) {
                if overlap < self.min_overlap
                    || overlap > part.end_connector
                    || overlap > selected[i + 1].2.start_connector
                {
                    return Err(invalid("MATH assembly overlap bounds"));
                }
                position -= i64::from(overlap);
            }
        }
        Ok(AssemblyPlacement {
            parts,
            advance: position,
            italic_correction: assembly.italic_correction,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn put(b: &mut [u8], at: usize, n: u16) {
        b[at..at + 2].copy_from_slice(&n.to_be_bytes())
    }
    fn fixture() -> Vec<u8> {
        let mut b = vec![0; 66];
        put(&mut b, 0, 1);
        put(&mut b, 8, 10);
        // variants at10; vertical coverage22; construction28; assembly40.
        for (at, n) in [
            (10, 5),
            (12, 12),
            (16, 1),
            (20, 18),
            (22, 1),
            (24, 1),
            (26, 2),
            (28, 12),
            (30, 2),
            (32, 3),
            (34, 100),
            (36, 4),
            (38, 200),
            (40, 65535),
            (44, 2),
            (46, 5),
            (48, 0),
            (50, 20),
            (52, 100),
            (56, 6),
            (58, 20),
            (60, 0),
            (62, 100),
            (64, 1),
        ] {
            put(&mut b, at, n)
        }
        b
    }
    #[test]
    fn variants_and_assembly_exact() {
        let p = MathVariants::parse(&fixture(), 10).unwrap();
        assert_eq!(p.min_connector_overlap(), 5);
        assert_eq!(
            p.select(Direction::Vertical, 2, 150),
            Selection::Variant(Variant {
                glyph_id: 4,
                advance: 200
            })
        );
        let Selection::AssemblyRequired(a) = p.select(Direction::Vertical, 2, 201) else {
            panic!()
        };
        assert_eq!(a.italic_correction, -1);
        assert!(a.parts[1].extender);
        assert_eq!(a.parts[0].end_connector, 20);
        assert_eq!(
            p.select(Direction::Horizontal, 2, 1),
            Selection::Unavailable
        );
    }
    #[test]
    fn exact_assembly_overlap_and_caps() {
        let p = MathVariants::parse(&fixture(), 10).unwrap();
        let placed = p.assemble(Direction::Vertical, 2, 1, &[10]).unwrap();
        assert_eq!(placed.advance, 190);
        assert_eq!(placed.parts[1].offset, 90);
        assert_eq!(
            p.assemble(Direction::Vertical, 2, 0, &[]).unwrap().advance,
            100
        );
        assert!(p.assemble(Direction::Vertical, 2, 1, &[4]).is_err());
        assert!(p.assemble(Direction::Vertical, 2, 1, &[21]).is_err());
        assert!(p.assemble(Direction::Vertical, 2, 1025, &[]).is_err());
        assert!(p.assemble(Direction::Vertical, 2, 1, &[]).is_err());
    }
    #[test]
    fn exact_target_fit_rational_overlap_and_refusals() {
        use crate::{cff::Rational, math_fit::*};
        let mut b = fixture();
        put(&mut b, 60, 20);
        let p = MathVariants::parse(&b, 10).unwrap();
        let fit = |target, limits| {
            p.fit(
                Direction::Vertical,
                2,
                target,
                FitStrategy::EqualExtendersProportionalConnectorFlexibility,
                limits,
            )
        };
        assert!(
            matches!(fit(Rational::new(190,1).unwrap(),FitLimits::default()).unwrap().shape,FittedShape::Variant(v) if v.advance==200)
        );
        for (target, overlap) in [
            (
                Rational::new(260, 1).unwrap(),
                Rational::new(20, 1).unwrap(),
            ),
            (Rational::new(290, 1).unwrap(), Rational::new(5, 1).unwrap()),
            (
                Rational::new(551, 2).unwrap(),
                Rational::new(49, 4).unwrap(),
            ),
        ] {
            let FittedShape::Assembly(a) = fit(target, FitLimits::default()).unwrap().shape else {
                panic!()
            };
            assert_eq!(a.advance, target);
            assert_eq!(a.extender_repetitions, 2);
            assert_eq!(a.overlaps, vec![overlap; 2]);
            assert_eq!(a.parts[2].instance, 1);
        }
        assert!(matches!(
            fit(
                Rational::new(275, 1).unwrap(),
                FitLimits {
                    max_parts: 2,
                    ..FitLimits::default()
                }
            ),
            Err(FitError::Budget)
        ));
        assert!(fit(Rational::new(250, 1).unwrap(), FitLimits::default()).is_err());
        assert!(matches!(
            fit(Rational::new(0, 1).unwrap(), FitLimits::default()),
            Err(FitError::InvalidTarget)
        ));
        assert!(matches!(
            fit(Rational::new(1, i128::MAX).unwrap(), FitLimits::default()),
            Err(FitError::Arithmetic(_))
        ));
        assert!(matches!(
            fit(
                Rational::new(250, 1).unwrap(),
                FitLimits {
                    max_parts: 4097,
                    ..FitLimits::default()
                }
            ),
            Err(FitError::InvalidLimits)
        ));
        let mut unequal = b.clone();
        put(&mut unequal, 60, 10);
        let unequal = MathVariants::parse(&unequal, 10).unwrap();
        let FittedShape::Assembly(a) = unequal
            .fit(
                Direction::Vertical,
                2,
                Rational::new(280, 1).unwrap(),
                FitStrategy::EqualExtendersProportionalConnectorFlexibility,
                FitLimits::default(),
            )
            .unwrap()
            .shape
        else {
            panic!()
        };
        assert_eq!(
            a.overlaps,
            vec![Rational::new(25, 2).unwrap(), Rational::new(15, 2).unwrap()]
        );
        let mut impossible = b;
        put(&mut impossible, 58, 0);
        assert!(MathVariants::parse(&impossible, 10)
            .unwrap()
            .fit(
                Direction::Vertical,
                2,
                Rational::new(275, 1).unwrap(),
                FitStrategy::EqualExtendersProportionalConnectorFlexibility,
                FitLimits::default()
            )
            .is_err());
    }
    #[test]
    fn malformed_offsets_records_and_gids() {
        let b = fixture();
        for length in 0..b.len() {
            assert!(
                MathVariants::parse(&b[..length], 10).is_err(),
                "prefix {length}"
            );
        }
        for (at, value) in [
            (12, 65535),
            (16, 4097),
            (26, 10),
            (32, 10),
            (38, 90),
            (64, 2),
            (24, 2),
            (20, 0),
        ] {
            let mut bad = b.clone();
            put(&mut bad, at, value);
            assert!(MathVariants::parse(&bad, 10).is_err(), "field {at}");
        }
    }
    #[test]
    fn coverage_ranges_strict() {
        let b = [0, 2, 0, 1, 0, 2, 0, 4, 0, 0];
        assert_eq!(coverage(&b, 0, 3, 5).unwrap(), vec![2, 3, 4]);
        let mut bad = b;
        bad[9] = 1;
        assert!(coverage(&bad, 0, 3, 5).is_err());
        assert!(coverage(&b, 0, 2, 5).is_err());
    }
}
