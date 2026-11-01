//! CFF1 metadata and staged Type2 decoding. Feed cff_table() from the existing
//! font-engine/pdf OpenType reader; this module does not duplicate sfnt selection.
use crate::{invalid, u16_at, Error, Result};
use std::{collections::BTreeMap, ops::Range, sync::Arc};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DictNumber {
    Integer(i32),
    Decimal(String),
}
pub type Dictionary = BTreeMap<u16, Vec<DictNumber>>;
#[derive(Debug, Clone)]
pub struct Cff {
    bytes: Arc<[u8]>,
    pub sha256: String,
    pub name: Vec<u8>,
    pub top: Dictionary,
    pub private: Dictionary,
    pub charset: Vec<u16>,
    pub encoding: BTreeMap<u8, u16>,
    pub predefined_encoding: Option<u8>,
    strings: Vec<Range<usize>>,
    charstrings: Vec<Range<usize>>,
    global_subrs: Vec<Range<usize>>,
    local_subrs: Vec<Range<usize>>,
}
fn unsupported(message: &str) -> Error {
    Error::UnsupportedFont(message.into())
}
fn byte(bytes: &[u8], at: &mut usize) -> Result<u8> {
    let v = *bytes
        .get(*at)
        .ok_or_else(|| invalid("CFF truncated byte"))?;
    *at += 1;
    Ok(v)
}
fn integer(bytes: &[u8], at: &mut usize, first: u8) -> Result<i32> {
    Ok(match first {
        32..=246 => first as i32 - 139,
        247..=250 => (first as i32 - 247) * 256 + byte(bytes, at)? as i32 + 108,
        251..=254 => -(first as i32 - 251) * 256 - byte(bytes, at)? as i32 - 108,
        28 => {
            let v = u16_at(bytes, *at)? as i16 as i32;
            *at += 2;
            v
        }
        29 => {
            let v = crate::u32_at(bytes, *at)? as i32;
            *at += 4;
            v
        }
        _ => return Err(invalid("CFF invalid integer")),
    })
}
fn decimal(bytes: &[u8], at: &mut usize) -> Result<String> {
    let mut text = String::new();
    let mut done = false;
    for _ in 0..40 {
        let value = byte(bytes, at)?;
        for (i, nibble) in [value >> 4, value & 15].into_iter().enumerate() {
            match nibble {
                0..=9 => text.push((b'0' + nibble) as char),
                10 => text.push('.'),
                11 => text.push('E'),
                12 => text.push_str("E-"),
                14 => text.push('-'),
                15 => {
                    if i == 0 && value & 15 != 15 {
                        return Err(invalid("CFF real padding"));
                    }
                    done = true;
                    break;
                }
                _ => return Err(invalid("CFF reserved real nibble")),
            }
        }
        if done {
            break;
        }
    }
    if !done {
        return Err(invalid("CFF real operand budget"));
    }
    let mut parts = text.split('E');
    let mantissa = parts
        .next()
        .unwrap()
        .strip_prefix('-')
        .unwrap_or_else(|| text.split('E').next().unwrap());
    if mantissa.is_empty()
        || mantissa.bytes().filter(|b| *b == b'.').count() > 1
        || !mantissa.bytes().any(|b| b.is_ascii_digit())
        || mantissa.bytes().any(|b| !b.is_ascii_digit() && b != b'.')
    {
        return Err(invalid("CFF real mantissa"));
    }
    if let Some(exponent) = parts.next() {
        let exponent = exponent.strip_prefix('-').unwrap_or(exponent);
        if exponent.is_empty() || !exponent.bytes().all(|b| b.is_ascii_digit()) {
            return Err(invalid("CFF real exponent"));
        }
    }
    if parts.next().is_some() {
        return Err(invalid("CFF multiple exponents"));
    }
    Ok(text)
}
pub(crate) fn dict(bytes: &[u8]) -> Result<Dictionary> {
    let mut at = 0;
    let mut operands = Vec::new();
    let mut out = BTreeMap::new();
    while at < bytes.len() {
        let op = byte(bytes, &mut at)?;
        if op >= 32 || op == 28 || op == 29 {
            operands.push(DictNumber::Integer(integer(bytes, &mut at, op)?));
        } else if op == 30 {
            operands.push(DictNumber::Decimal(decimal(bytes, &mut at)?));
        } else if op <= 21 {
            let key = if op == 12 {
                0x0c00 | byte(bytes, &mut at)? as u16
            } else {
                op as u16
            };
            if out.insert(key, std::mem::take(&mut operands)).is_some() {
                return Err(invalid("duplicate CFF DICT operator"));
            }
        } else {
            return Err(invalid("CFF reserved DICT byte"));
        }
        if operands.len() > 48 || out.len() > 256 {
            return Err(invalid("CFF DICT operand/operator budget"));
        }
    }
    if !operands.is_empty() {
        return Err(invalid("CFF dangling DICT operands"));
    }
    Ok(out)
}
fn number(d: &Dictionary, key: u16, default: Option<i32>) -> Result<i32> {
    match d.get(&key) {
        None => default.ok_or_else(|| invalid("CFF required DICT key missing")),
        Some(v) => match v.as_slice() {
            [DictNumber::Integer(n)] => Ok(*n),
            _ => Err(unsupported("CFF noninteger/wrong-arity DICT field")),
        },
    }
}
fn offset(d: &Dictionary, key: u16, default: Option<i32>) -> Result<usize> {
    usize::try_from(number(d, key, default)?).map_err(|_| invalid("negative CFF offset"))
}
fn index(bytes: &[u8], at: &mut usize) -> Result<Vec<Range<usize>>> {
    let count = u16_at(bytes, *at)? as usize;
    *at += 2;
    if count == 0 {
        return Ok(Vec::new());
    }
    let size = byte(bytes, at)? as usize;
    if !(1..=4).contains(&size) {
        return Err(invalid("CFF INDEX offSize"));
    }
    let mut offsets = Vec::with_capacity(count + 1);
    for _ in 0..=count {
        let mut n = 0usize;
        for _ in 0..size {
            n = (n << 8) | byte(bytes, at)? as usize;
        }
        offsets.push(n);
    }
    if offsets[0] != 1 || offsets.windows(2).any(|p| p[0] > p[1]) {
        return Err(invalid("CFF INDEX offsets"));
    }
    let base = *at;
    let end = base
        .checked_add(offsets[count] - 1)
        .ok_or_else(|| invalid("CFF INDEX overflow"))?;
    if end > bytes.len() {
        return Err(invalid("CFF INDEX data bounds"));
    }
    *at = end;
    Ok(offsets
        .windows(2)
        .map(|p| base + p[0] - 1..base + p[1] - 1)
        .collect())
}
impl Cff {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 64 * 1024 * 1024 {
            return Err(invalid("CFF byte budget"));
        }
        if bytes.first() != Some(&1) {
            return Err(unsupported("CFF1 only; CFF2 unsupported"));
        }
        let mut at = 1;
        if byte(bytes, &mut at)? != 0 {
            return Err(unsupported("CFF minor version"));
        }
        let header = byte(bytes, &mut at)? as usize;
        let offsize = byte(bytes, &mut at)?;
        if header < 4 || header > bytes.len() || !(1..=4).contains(&offsize) {
            return Err(invalid("CFF header"));
        }
        at = header;
        let names = index(bytes, &mut at)?;
        let tops = index(bytes, &mut at)?;
        let strings = index(bytes, &mut at)?;
        let global_subrs = index(bytes, &mut at)?;
        if names.len() != 1 || tops.len() != 1 {
            return Err(unsupported("CFF multi-font set unsupported"));
        }
        if strings.len() > 64609 || names[0].is_empty() || names[0].len() > 127 {
            return Err(invalid("CFF name/string count"));
        }
        let top = dict(&bytes[tops[0].clone()])?;
        if top.contains_key(&0x0c1e) || top.contains_key(&0x0c24) || top.contains_key(&0x0c25) {
            return Err(unsupported("CID-keyed CFF unsupported"));
        }
        if number(&top, 0x0c06, Some(2))? != 2 {
            return Err(unsupported("CFF charstring type is not Type2"));
        }
        let mut char_at = offset(&top, 17, None)?;
        if char_at < at {
            return Err(invalid("CFF CharStrings overlaps mandatory INDEX data"));
        }
        let charstrings = index(bytes, &mut char_at)?;
        if charstrings.is_empty() {
            return Err(invalid("CFF has no .notdef charstring"));
        }
        let mut private = Dictionary::new();
        let mut local_subrs = Vec::new();
        if let Some(values) = top.get(&18) {
            let [DictNumber::Integer(size), DictNumber::Integer(start)] = values.as_slice() else {
                return Err(invalid("CFF Private DICT operands"));
            };
            let start = usize::try_from(*start).map_err(|_| invalid("CFF Private offset"))?;
            let size = usize::try_from(*size).map_err(|_| invalid("CFF Private size"))?;
            let end = start
                .checked_add(size)
                .ok_or_else(|| invalid("CFF Private overflow"))?;
            private = dict(
                bytes
                    .get(start..end)
                    .ok_or_else(|| invalid("CFF Private bounds"))?,
            )?;
            if private.contains_key(&19) {
                let relative = offset(&private, 19, None)?;
                if relative < size {
                    return Err(invalid("CFF Subrs overlaps Private DICT"));
                }
                let mut local = start
                    .checked_add(relative)
                    .ok_or_else(|| invalid("CFF Subrs offset overflow"))?;
                local_subrs = index(bytes, &mut local)?;
            }
        }
        let mut charset = vec![0u16];
        let charset_offset = offset(&top, 15, Some(0))?;
        if charset_offset == 0 {
            if charstrings.len() > 229 {
                return Err(invalid("CFF ISOAdobe charset glyph count"));
            }
            charset.extend(1..charstrings.len() as u16);
        } else if charset_offset <= 2 {
            return Err(unsupported("CFF predefined Expert charset unsupported"));
        } else {
            let mut cursor = charset_offset;
            let format = byte(bytes, &mut cursor)?;
            while charset.len() < charstrings.len() {
                let sid = u16_at(bytes, cursor)?;
                cursor += 2;
                let extra = match format {
                    0 => 0,
                    1 => byte(bytes, &mut cursor)? as usize,
                    2 => {
                        let n = u16_at(bytes, cursor)? as usize;
                        cursor += 2;
                        n
                    }
                    _ => return Err(invalid("CFF charset format")),
                };
                if charset.len() + extra + 1 > charstrings.len() {
                    return Err(invalid("CFF charset range exceeds glyphs"));
                }
                for i in 0..=extra {
                    let value = (sid as usize)
                        .checked_add(i)
                        .filter(|v| *v < 391 + strings.len())
                        .ok_or_else(|| invalid("CFF charset SID outside strings"))?;
                    charset.push(value as u16);
                }
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        if charset.iter().any(|sid| !seen.insert(*sid)) {
            return Err(invalid("duplicate CFF charset SID"));
        }
        let encoding_offset = offset(&top, 16, Some(0))?;
        let mut encoding = BTreeMap::new();
        let predefined_encoding = if encoding_offset <= 1 {
            Some(encoding_offset as u8)
        } else {
            let mut cursor = encoding_offset;
            let format = byte(bytes, &mut cursor)?;
            let count = byte(bytes, &mut cursor)? as usize;
            let mut gid = 1;
            for _ in 0..count {
                let first = byte(bytes, &mut cursor)?;
                let extra = match format & 127 {
                    0 => 0,
                    1 => byte(bytes, &mut cursor)? as usize,
                    _ => return Err(invalid("CFF encoding format")),
                };
                for i in 0..=extra {
                    let code = (first as usize + i)
                        .try_into()
                        .map_err(|_| invalid("CFF encoding code overflow"))?;
                    let sid = *charset
                        .get(gid)
                        .ok_or_else(|| invalid("CFF encoding glyph count"))?;
                    gid += 1;
                    if encoding.insert(code, sid).is_some() {
                        return Err(invalid("duplicate CFF encoding code"));
                    }
                }
            }
            if format & 128 != 0 {
                let count = byte(bytes, &mut cursor)?;
                for _ in 0..count {
                    let code = byte(bytes, &mut cursor)?;
                    let sid = u16_at(bytes, cursor)?;
                    cursor += 2;
                    if !seen.contains(&sid) || encoding.insert(code, sid).is_some() {
                        return Err(invalid("CFF encoding supplement"));
                    }
                }
            }
            None
        };
        Ok(Self {
            bytes: Arc::from(bytes),
            sha256: crate::sha256(bytes),
            name: bytes[names[0].clone()].to_vec(),
            top,
            private,
            charset,
            encoding,
            predefined_encoding,
            strings,
            charstrings,
            global_subrs,
            local_subrs,
        })
    }
    pub fn glyph_count(&self) -> usize {
        self.charstrings.len()
    }
    pub fn charstring(&self, gid: u16) -> Result<&[u8]> {
        self.charstrings
            .get(gid as usize)
            .map(|r| &self.bytes[r.clone()])
            .ok_or_else(|| invalid("CFF glyph ID outside font"))
    }
    pub fn custom_string(&self, sid: u16) -> Option<&[u8]> {
        sid.checked_sub(391)
            .and_then(|i| self.strings.get(i as usize))
            .map(|r| &self.bytes[r.clone()])
    }
    pub fn local_subr_count(&self) -> usize {
        self.local_subrs.len()
    }
    pub fn global_subr_count(&self) -> usize {
        self.global_subrs.len()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn fixture() -> Vec<u8> {
        vec![
            1, 0, 4, 4, 0, 1, 1, 1, 2, b'F', 0, 1, 1, 1, 3, 160, 17, 0, 0, 0, 0, 0, 1, 1, 1, 2, 14,
        ]
    }
    #[test]
    fn minimal_metadata() {
        let c = Cff::parse(&fixture()).unwrap();
        assert_eq!(c.name, b"F");
        assert_eq!(c.glyph_count(), 1);
        assert_eq!(c.charstring(0).unwrap(), [14]);
    }
    #[test]
    fn all_truncations_and_index_offsets() {
        let b = fixture();
        for n in 0..b.len() {
            assert!(Cff::parse(&b[..n]).is_err(), "{n}");
        }
        let mut b = fixture();
        b[7] = 0;
        assert!(Cff::parse(&b).is_err());
    }
    #[test]
    fn dict_exact_reals_and_operand_caps() {
        assert_eq!(
            dict(&[30, 0x0a, 0x12, 0x5f, 12, 7]).unwrap()[&0x0c07],
            vec![DictNumber::Decimal("0.125".into())]
        );
        assert!(dict(&[139; 49]).is_err());
        assert!(dict(&[30, 0xdf, 17]).is_err());
        assert!(dict(&[139, 17, 140, 17]).is_err());
    }
    #[test]
    fn unsupported_cff2_and_cid() {
        let mut b = fixture();
        b[0] = 2;
        assert!(matches!(Cff::parse(&b), Err(Error::UnsupportedFont(_))));
        let mut b = fixture();
        b[14] = 6;
        b.splice(17..17, [139, 12, 30]);
        b[15] = 163;
        assert!(matches!(Cff::parse(&b), Err(Error::UnsupportedFont(_))));
    }
}

mod type2;
pub use type2::{
    CubicCommand, CubicOutline, CubicPoint, HintMask, HintMetadata, HintPolicy, StemHint,
};

mod matrix;
pub use matrix::{MatrixCommand, MatrixOutline, Rational, RationalPoint};

mod cache;
pub use cache::{CacheLimits, CacheOutcome, CacheStatus, CffIdentity, CffOutlineCache};

mod names;
mod standard_strings;
pub use names::{
    BoundCffTfmFont, CffEncodingCache, CffEncodingManifest, CffGlyphNames, EncodingCacheOutcome,
    ResolvedCffEncoding,
};
