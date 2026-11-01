//! Original bounded MathKernInfo parser; unhinted design values only.
use crate::{
    cff::Rational,
    invalid,
    math_variants::{coverage, offset, u16at},
    Result,
};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Corner {
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}
impl Corner {
    pub const ALL: [Self; 4] = [
        Self::TopRight,
        Self::TopLeft,
        Self::BottomRight,
        Self::BottomLeft,
    ];
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Value {
    pub design_units: i16,
    pub device_adjustment_present: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernTable {
    parent_offset: usize,
    heights: Vec<Value>,
    kerns: Vec<Value>,
}
impl KernTable {
    pub(crate) fn retained_value_bytes(&self) -> usize {
        (self.heights.capacity() + self.kerns.capacity()) * std::mem::size_of::<Value>()
    }

    pub fn correction_heights(&self) -> &[Value] {
        &self.heights
    }
    pub fn kern_values(&self) -> &[Value] {
        &self.kerns
    }
    /// Upper-bound search: equality advances to the following kern interval.
    pub fn lookup(&self, height: Rational) -> Result<Value> {
        let mut low = 0;
        let mut high = self.heights.len();
        while low < high {
            let mid = low + (high - low) / 2;
            let boundary = i128::from(self.heights[mid].design_units)
                .checked_mul(height.denominator())
                .ok_or_else(|| invalid("MATH kern comparison overflow"))?;
            if height.numerator() >= boundary {
                low = mid + 1
            } else {
                high = mid
            }
        }
        Ok(self.kerns[low])
    }
}
#[derive(Debug)]
pub struct MathKern {
    glyph_count: u16,
    records: BTreeMap<(u16, Corner), KernTable>,
}
fn value(b: &[u8], base: usize, at: usize) -> Result<Value> {
    let device = u16at(b, at + 2)?;
    if device != 0 {
        offset(b, base, at + 2)?;
    }
    Ok(Value {
        design_units: u16at(b, at)? as i16,
        device_adjustment_present: device != 0,
    })
}
impl MathKern {
    pub fn parse(math: &[u8], glyph_count: u16) -> Result<Self> {
        if math.len() > 4 * 1024 * 1024 {
            return Err(crate::Error::SizeLimit);
        }
        if u16at(math, 0)? != 1 {
            return Err(crate::Error::UnsupportedFont("MATH major version".into()));
        }
        let mut result = Self {
            glyph_count,
            records: BTreeMap::new(),
        };
        // Require the complete header even when this optional data is absent.
        u16at(math, 8)?;
        if u16at(math, 6)? == 0 {
            return Ok(result);
        }
        let info = offset(math, 0, 6)?;
        if u16at(math, info + 6)? == 0 {
            return Ok(result);
        }
        let base = offset(math, info, info + 6)?;
        let count = u16at(math, base + 2)? as usize;
        if count > 4096 {
            return Err(crate::Error::SizeLimit);
        }
        let gids = if count == 0 && u16at(math, base)? == 0 {
            Vec::new()
        } else {
            coverage(math, offset(math, base, base)?, count, glyph_count)?
        };
        let mut total = 0usize;
        for (i, gid) in gids.into_iter().enumerate() {
            for (j, corner) in Corner::ALL.into_iter().enumerate() {
                let pos = base + 4 + i * 8 + j * 2;
                if u16at(math, pos)? == 0 {
                    continue;
                }
                let at = offset(math, base, pos)?;
                let n = u16at(math, at)? as usize;
                total += n * 2 + 1;
                if total > 65536 {
                    return Err(crate::Error::SizeLimit);
                }
                let heights = (0..n)
                    .map(|k| value(math, at, at + 2 + k * 4))
                    .collect::<Result<Vec<_>>>()?;
                if heights
                    .windows(2)
                    .any(|w| w[0].design_units >= w[1].design_units)
                {
                    return Err(invalid("MATH correction heights not strictly increasing"));
                }
                let kerns = (0..=n)
                    .map(|k| value(math, at, at + 2 + n * 4 + k * 4))
                    .collect::<Result<Vec<_>>>()?;
                result.records.insert(
                    (gid, corner),
                    KernTable {
                        parent_offset: at,
                        heights,
                        kerns,
                    },
                );
            }
        }
        Ok(result)
    }
    pub(crate) fn device_lookup(
        &self,
        math: &[u8],
        gid: u16,
        corner: Corner,
        height: Rational,
        units_per_em: u16,
        context: crate::math_device::KernDeviceContext,
    ) -> std::result::Result<crate::math_device::KernCorrection, crate::math_device::DeviceError>
    {
        use crate::math_device::*;
        if gid >= self.glyph_count || units_per_em == 0 {
            return Err(DeviceError::InvalidRecord);
        }
        let Some(table) = self.records.get(&(gid, corner)) else {
            return Ok(KernCorrection {
                glyph_id: gid,
                corner,
                query_height: height,
                context,
                selected_interval: 0,
                correction: RecordCorrection {
                    design_units: 0,
                    delta_pixels: 0,
                    context: context.horizontal,
                    device_table_offset: None,
                    device_table_sha256: None,
                },
            });
        };
        if table.heights.len() > 4096 {
            return Err(DeviceError::Bounds);
        }
        let mut selected = 0;
        let mut previous = None;
        for i in 0..table.heights.len() {
            let correction = record_correction(
                math,
                table.parent_offset,
                table.parent_offset + 2 + i * 4,
                context.vertical,
            )?;
            let boundary = Rational::new(i128::from(correction.design_units), 1)?.checked_add(
                Rational::new(
                    i128::from(correction.delta_pixels) * i128::from(units_per_em),
                    i128::from(context.vertical.ppem()),
                )?,
            )?;
            let compare = |a: Rational,
                           b: Rational|
             -> std::result::Result<std::cmp::Ordering, DeviceError> {
                let l = a
                    .numerator()
                    .checked_mul(b.denominator())
                    .ok_or(DeviceError::Bounds)?;
                let r = b
                    .numerator()
                    .checked_mul(a.denominator())
                    .ok_or(DeviceError::Bounds)?;
                Ok(l.cmp(&r))
            };
            if let Some(p) = previous {
                if compare(boundary, p)?.is_lt() {
                    return Err(DeviceError::NonMonotoneHeights);
                }
            }
            if !compare(height, boundary)?.is_lt() {
                selected = i + 1;
            }
            previous = Some(boundary);
        }
        let correction = record_correction(
            math,
            table.parent_offset,
            table.parent_offset + 2 + table.heights.len() * 4 + selected * 4,
            context.horizontal,
        )?;
        Ok(KernCorrection {
            glyph_id: gid,
            corner,
            query_height: height,
            context,
            selected_interval: selected,
            correction,
        })
    }
    pub fn records(&self) -> &BTreeMap<(u16, Corner), KernTable> {
        &self.records
    }
    /// Absent per-glyph/corner data yields the specified zero adjustment.
    pub fn lookup(&self, glyph_id: u16, corner: Corner, height: Rational) -> Result<Value> {
        if glyph_id >= self.glyph_count {
            return Err(invalid("MATH kern GID outside font"));
        }
        match self.records.get(&(glyph_id, corner)) {
            Some(t) => t.lookup(height),
            None => Ok(Value {
                design_units: 0,
                device_adjustment_present: false,
            }),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn put(b: &mut [u8], at: usize, n: u16) {
        b[at..at + 2].copy_from_slice(&n.to_be_bytes())
    }
    fn fixture() -> Vec<u8> {
        let mut b = vec![0; 58];
        for (at, n) in [
            (0, 1),
            (6, 10),
            (16, 8),
            (18, 12),
            (20, 1),
            (22, 18),
            (30, 1),
            (32, 1),
            (34, 2),
            (36, 2),
            (38, 65526),
            (42, 20),
            (46, 65533),
            (50, 5),
            (54, 8),
        ] {
            put(&mut b, at, n)
        }
        b
    }
    #[test]
    fn exact_signed_ties_and_absence() {
        let k = MathKern::parse(&fixture(), 5).unwrap();
        for (n, d, expected) in [
            (-11, 1, -3),
            (-10, 1, 5),
            (39, 2, 5),
            (20, 1, 8),
            (100, 1, 8),
        ] {
            assert_eq!(
                k.lookup(2, Corner::TopRight, Rational::new(n, d).unwrap())
                    .unwrap()
                    .design_units,
                expected
            )
        }
        assert_eq!(
            k.lookup(2, Corner::BottomLeft, Rational::new(0, 1).unwrap())
                .unwrap()
                .design_units,
            0
        );
        assert!(k
            .lookup(5, Corner::TopLeft, Rational::new(0, 1).unwrap())
            .is_err());
    }
    #[test]
    fn device_height_ties_and_kern_pixels() {
        use crate::math_device::*;
        let mut b = fixture();
        b.extend([0; 16]);
        for (at, n) in [
            (40, 22),
            (52, 30),
            (58, 100),
            (60, 100),
            (62, 1),
            (64, 0x4000),
            (66, 100),
            (68, 100),
            (70, 1),
            (72, 0x8000),
        ] {
            put(&mut b, at, n)
        }
        let k = MathKern::parse(&b, 5).unwrap();
        let context = KernDeviceContext {
            horizontal: DeviceContext::new(100).unwrap(),
            vertical: DeviceContext::new(100).unwrap(),
        };
        let before = k
            .device_lookup(
                &b,
                2,
                Corner::TopRight,
                Rational::new(-1, 1).unwrap(),
                1000,
                context,
            )
            .unwrap();
        assert_eq!(before.correction.design_units, -3);
        let tie = k
            .device_lookup(
                &b,
                2,
                Corner::TopRight,
                Rational::new(0, 1).unwrap(),
                1000,
                context,
            )
            .unwrap();
        assert_eq!(tie.selected_interval, 1);
        assert_eq!(tie.correction.design_units, 5);
        assert_eq!(tie.correction.delta_pixels, -2);
        let outside = KernDeviceContext {
            horizontal: DeviceContext::new(101).unwrap(),
            vertical: DeviceContext::new(101).unwrap(),
        };
        assert_eq!(
            k.device_lookup(
                &b,
                2,
                Corner::TopRight,
                Rational::new(0, 1).unwrap(),
                1000,
                outside
            )
            .unwrap()
            .correction
            .delta_pixels,
            0
        );
        let mut variation = b.clone();
        put(&mut variation, 70, 0x8000);
        assert!(matches!(
            k.device_lookup(
                &variation,
                2,
                Corner::TopRight,
                Rational::new(0, 1).unwrap(),
                1000,
                outside
            ),
            Err(DeviceError::UnsupportedVariationIndex)
        ));
        put(&mut b, 62, 3);
        put(&mut b, 64, 0x0800);
        assert!(matches!(
            k.device_lookup(
                &b,
                2,
                Corner::TopRight,
                Rational::new(0, 1).unwrap(),
                1000,
                context
            ),
            Err(DeviceError::NonMonotoneHeights)
        ));
    }
    #[test]
    fn malformed_and_device_limits() {
        let b = fixture();
        for length in 0..b.len() {
            assert!(MathKern::parse(&b[..length], 5).is_err())
        }
        for (at, n) in [
            (16, 65535),
            (20, 4097),
            (34, 5),
            (36, 65535),
            (42, 65526),
            (22, 65535),
            (40, 65535),
        ] {
            let mut bad = b.clone();
            put(&mut bad, at, n);
            assert!(MathKern::parse(&bad, 5).is_err(), "{at}");
        }
        let mut device = b;
        device.extend([0; 6]);
        put(&mut device, 48, 22);
        let k = MathKern::parse(&device, 5).unwrap();
        assert!(
            k.lookup(2, Corner::TopRight, Rational::new(-11, 1).unwrap())
                .unwrap()
                .device_adjustment_present
        );
        assert!(k
            .lookup(2, Corner::TopRight, Rational::new(1, i128::MAX).unwrap())
            .is_err());
    }
}
