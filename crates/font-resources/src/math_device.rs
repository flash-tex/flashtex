//! OpenType Device formats 1/2/3 only. Pixel deltas are not font-unit deltas.
use crate::math_variants::u16at;
#[derive(Debug)]
pub enum DeviceError {
    Invalid(crate::Error),
    UnsupportedVariationIndex,
    UnsupportedFormat(u16),
    InvalidPpem,
    InvalidRecord,
    NonMonotoneHeights,
    Bounds,
}
impl From<crate::Error> for DeviceError {
    fn from(e: crate::Error) -> Self {
        Self::Invalid(e)
    }
}
impl std::fmt::Display for DeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for DeviceError {}
/// Positive integer ppem is supplied explicitly, never inferred from point size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceContext {
    ppem: u16,
}
impl DeviceContext {
    pub fn new(ppem: u16) -> Result<Self, DeviceError> {
        if ppem == 0 {
            Err(DeviceError::InvalidPpem)
        } else {
            Ok(Self { ppem })
        }
    }
    pub fn ppem(self) -> u16 {
        self.ppem
    }
}
/// Zero-based index of one of the51 MathValueRecords, in OpenType MathConstants
/// order (mathLeading=0, axisHeight=1, radicalKernAfterDegree=50).
/// The two percentages, two unsigned heights and final percentage have no device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstantDeviceRecord(u8);
impl ConstantDeviceRecord {
    pub fn new(index: u8) -> Result<Self, DeviceError> {
        if index >= 51 {
            Err(DeviceError::InvalidRecord)
        } else {
            Ok(Self(index))
        }
    }
    pub fn index(self) -> u8 {
        self.0
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceTable {
    start: u16,
    end: u16,
    bits: u8,
    packed: Vec<u16>,
}
impl DeviceTable {
    /// Validates the complete declared payload, even for ppem outside its range.
    /// At most65536 deltas /32768 words (64KiB) are retained.
    pub fn parse(bytes: &[u8], offset: usize) -> Result<Self, DeviceError> {
        if offset > bytes.len().saturating_sub(6) {
            return Err(DeviceError::Bounds);
        }
        let start = u16at(bytes, offset)?;
        let end = u16at(bytes, offset + 2)?;
        let format = u16at(bytes, offset + 4)?;
        let bits = match format {
            1 => 2,
            2 => 4,
            3 => 8,
            0x8000 => return Err(DeviceError::UnsupportedVariationIndex),
            other => return Err(DeviceError::UnsupportedFormat(other)),
        };
        if start > end {
            return Err(DeviceError::Bounds);
        }
        let n = usize::from(end) - usize::from(start) + 1;
        let bit_count = n * bits as usize;
        let words = bit_count.div_ceil(16);
        let end_offset = offset
            .checked_add(6 + words * 2)
            .ok_or(DeviceError::Bounds)?;
        if end_offset > bytes.len() {
            return Err(DeviceError::Bounds);
        }
        let packed = (0..words)
            .map(|i| u16at(bytes, offset + 6 + i * 2).map_err(DeviceError::from))
            .collect::<Result<Vec<_>, _>>()?;
        let used = bit_count % 16;
        if used != 0 && packed.last().unwrap() & ((1u16 << (16 - used)) - 1) != 0 {
            return Err(DeviceError::Bounds);
        }
        Ok(Self {
            start,
            end,
            bits,
            packed,
        })
    }
    pub fn range(&self) -> std::ops::RangeInclusive<u16> {
        self.start..=self.end
    }
    pub fn byte_length(&self) -> usize {
        6 + self.packed.len() * 2
    }
    pub fn correction(&self, context: DeviceContext) -> i8 {
        if context.ppem < self.start || context.ppem > self.end {
            return 0;
        }
        let index = usize::from(context.ppem - self.start);
        let per_word = 16 / self.bits as usize;
        let shift = 16 - self.bits as usize * (index % per_word + 1);
        let raw = (self.packed[index / per_word] >> shift) & ((1 << self.bits) - 1);
        let sign = 1u16 << (self.bits - 1);
        if raw & sign != 0 {
            (raw as i16 - (1i16 << self.bits)) as i8
        } else {
            raw as i8
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantCorrection {
    pub record: ConstantDeviceRecord,
    pub context: DeviceContext,
    /// Pixel delta; caller must explicitly choose axis and pixel-to-page scaling.
    pub delta_pixels: i8,
    pub device_table_offset: Option<usize>,
    pub device_table_sha256: Option<String>,
}
pub(crate) fn constant_correction(
    math: &[u8],
    record: ConstantDeviceRecord,
    context: DeviceContext,
) -> Result<ConstantCorrection, DeviceError> {
    let base = crate::math_variants::offset(math, 0, 4)?;
    let pos = base + 8 + usize::from(record.0) * 4;
    u16at(math, pos)?;
    let relative = u16at(math, pos + 2)?;
    let (delta, offset, sha) = if relative == 0 {
        (0, None, None)
    } else {
        let offset = base + usize::from(relative);
        let table = DeviceTable::parse(math, offset)?;
        (
            table.correction(context),
            Some(offset),
            Some(crate::sha256(&math[offset..offset + table.byte_length()])),
        )
    };
    Ok(ConstantCorrection {
        record,
        context,
        delta_pixels: delta,
        device_table_offset: offset,
        device_table_sha256: sha,
    })
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphDeviceKind {
    ItalicCorrection,
    TopAccentAttachment,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordCorrection {
    pub design_units: i16,
    pub delta_pixels: i8,
    pub context: DeviceContext,
    pub device_table_offset: Option<usize>,
    pub device_table_sha256: Option<String>,
}
pub(crate) fn record_correction(
    math: &[u8],
    parent: usize,
    record: usize,
    context: DeviceContext,
) -> Result<RecordCorrection, DeviceError> {
    let design_units = u16at(math, record)? as i16;
    let relative = u16at(math, record + 2)?;
    let (delta_pixels, device_table_offset, device_table_sha256) = if relative == 0 {
        (0, None, None)
    } else {
        let offset = parent
            .checked_add(usize::from(relative))
            .ok_or(DeviceError::Bounds)?;
        let table = DeviceTable::parse(math, offset)?;
        (
            table.correction(context),
            Some(offset),
            Some(crate::sha256(&math[offset..offset + table.byte_length()])),
        )
    };
    Ok(RecordCorrection {
        design_units,
        delta_pixels,
        context,
        device_table_offset,
        device_table_sha256,
    })
}
pub(crate) fn glyph_record(
    math: &[u8],
    gid: u16,
    glyph_count: u16,
    kind: GlyphDeviceKind,
    context: DeviceContext,
) -> Result<Option<RecordCorrection>, DeviceError> {
    if gid >= glyph_count {
        return Err(DeviceError::InvalidRecord);
    }
    if u16at(math, 6)? == 0 {
        return Ok(None);
    }
    let info = crate::math_variants::offset(math, 0, 6)?;
    let pos = info
        + match kind {
            GlyphDeviceKind::ItalicCorrection => 0,
            GlyphDeviceKind::TopAccentAttachment => 2,
        };
    if u16at(math, pos)? == 0 {
        return Ok(None);
    }
    let base = crate::math_variants::offset(math, info, pos)?;
    let count = u16at(math, base + 2)? as usize;
    if count > 4096 {
        return Err(DeviceError::Bounds);
    }
    let coverage = crate::math_variants::coverage(
        math,
        crate::math_variants::offset(math, base, base)?,
        count,
        glyph_count,
    )?;
    match coverage.binary_search(&gid) {
        Ok(index) => Ok(Some(record_correction(
            math,
            base,
            base + 4 + index * 4,
            context,
        )?)),
        Err(_) => Ok(None),
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernDeviceContext {
    pub horizontal: DeviceContext,
    pub vertical: DeviceContext,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernCorrection {
    pub glyph_id: u16,
    pub corner: crate::math_kern::Corner,
    pub query_height: crate::cff::Rational,
    pub context: KernDeviceContext,
    pub selected_interval: usize,
    pub correction: RecordCorrection,
}
#[cfg(test)]
mod tests {
    use super::*;
    fn table(format: u16, start: u16, end: u16, words: &[u16]) -> Vec<u8> {
        [vec![start, end, format], words.to_vec()]
            .concat()
            .into_iter()
            .flat_map(u16::to_be_bytes)
            .collect()
    }
    #[test]
    fn signed_formats_and_word_boundaries() {
        for (format, end, words, expected) in [
            (
                1,
                18,
                vec![0xb1b1, 0x8000],
                vec![-2, -1, 0, 1, -2, -1, 0, 1, -2],
            ),
            (2, 14, vec![0x8f07, 0x1000], vec![-8, -1, 0, 7, 1]),
            (3, 12, vec![0x807f, 0xff00], vec![-128, 127, -1]),
        ] {
            let parsed = DeviceTable::parse(&table(format, 10, end, &words), 0).unwrap();
            for (i, value) in expected.into_iter().enumerate() {
                assert_eq!(
                    parsed.correction(DeviceContext::new(10 + i as u16).unwrap()),
                    value
                )
            }
            assert_eq!(parsed.correction(DeviceContext::new(9).unwrap()), 0);
            assert_eq!(parsed.correction(DeviceContext::new(end + 1).unwrap()), 0);
        }
    }
    #[test]
    fn variation_unknown_padding_truncation_and_bounds() {
        assert!(matches!(
            DeviceTable::parse(&table(0x8000, 50, 1, &[]), 0),
            Err(DeviceError::UnsupportedVariationIndex)
        ));
        assert!(matches!(
            DeviceTable::parse(&table(4, 1, 1, &[]), 0),
            Err(DeviceError::UnsupportedFormat(4))
        ));
        let b = table(1, 10, 10, &[0x8000]);
        for length in 0..b.len() {
            assert!(DeviceTable::parse(&b[..length], 0).is_err())
        }
        assert!(DeviceTable::parse(&table(1, 10, 10, &[0x8001]), 0).is_err());
        assert!(DeviceTable::parse(&b, usize::MAX).is_err());
        assert!(DeviceContext::new(0).is_err());
        assert!(ConstantDeviceRecord::new(51).is_err());
    }
    #[test]
    fn glyph_record_device_offsets_signed_and_absent() {
        let mut b = vec![0; 40];
        for (at, n) in [
            (6, 10u16),
            (10, 8),
            (12, 8),
            (18, 8),
            (20, 1),
            (22, 65516),
            (24, 14),
            (26, 1),
            (28, 1),
            (30, 2),
        ] {
            b[at..at + 2].copy_from_slice(&n.to_be_bytes())
        }
        b[32..40].copy_from_slice(&table(2, 12, 12, &[0xf000]));
        for kind in [
            GlyphDeviceKind::ItalicCorrection,
            GlyphDeviceKind::TopAccentAttachment,
        ] {
            let value = glyph_record(&b, 2, 5, kind, DeviceContext::new(12).unwrap())
                .unwrap()
                .unwrap();
            assert_eq!(value.design_units, -20);
            assert_eq!(value.delta_pixels, -1);
            assert_eq!(value.device_table_offset, Some(32));
            assert_eq!(
                glyph_record(&b, 2, 5, kind, DeviceContext::new(13).unwrap())
                    .unwrap()
                    .unwrap()
                    .delta_pixels,
                0
            );
        }
        assert!(glyph_record(
            &b,
            3,
            5,
            GlyphDeviceKind::ItalicCorrection,
            DeviceContext::new(12).unwrap()
        )
        .unwrap()
        .is_none());
        b[36..38].copy_from_slice(&0x8000u16.to_be_bytes());
        assert!(matches!(
            glyph_record(
                &b,
                2,
                5,
                GlyphDeviceKind::TopAccentAttachment,
                DeviceContext::new(13).unwrap()
            ),
            Err(DeviceError::UnsupportedVariationIndex)
        ));
    }
    #[test]
    fn constant_parent_offset_and_absence() {
        let mut math = vec![0; 232];
        math[4..6].copy_from_slice(&10u16.to_be_bytes());
        math[24..26].copy_from_slice(&214u16.to_be_bytes());
        math[224..232].copy_from_slice(&table(1, 12, 12, &[0x8000]));
        let context = DeviceContext::new(12).unwrap();
        let correction =
            constant_correction(&math, ConstantDeviceRecord::new(1).unwrap(), context).unwrap();
        assert_eq!(correction.delta_pixels, -2);
        assert_eq!(correction.device_table_offset, Some(224));
        assert_eq!(
            constant_correction(&math, ConstantDeviceRecord::new(0).unwrap(), context)
                .unwrap()
                .device_table_offset,
            None
        );
    }
}
