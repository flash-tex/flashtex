//! Bounds-checked big-endian readers shared by every table parser.

use crate::Error;

pub fn u16_at(b: &[u8], at: usize) -> Result<u16, Error> {
    b.get(at..at + 2)
        .map(|s| u16::from_be_bytes([s[0], s[1]]))
        .ok_or_else(|| Error::Malformed(format!("read u16 at {at} past end")))
}

pub fn i16_at(b: &[u8], at: usize) -> Result<i16, Error> {
    u16_at(b, at).map(|v| v as i16)
}

pub fn u32_at(b: &[u8], at: usize) -> Result<u32, Error> {
    b.get(at..at + 4)
        .map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| Error::Malformed(format!("read u32 at {at} past end")))
}

pub fn i32_at(b: &[u8], at: usize) -> Result<i32, Error> {
    u32_at(b, at).map(|v| v as i32)
}

pub fn slice(b: &[u8], at: usize, len: usize) -> Result<&[u8], Error> {
    b.get(at..at + len)
        .ok_or_else(|| Error::Malformed(format!("slice {at}+{len} past end ({})", b.len())))
}

/// FNV-1a over arbitrary bytes; used for the deterministic subset tag only.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}
