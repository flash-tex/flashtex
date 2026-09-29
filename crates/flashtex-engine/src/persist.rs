//! A small binary codec for the host state a persisted checkpoint carries
//! beside the word space: pdfTeX's C-part state (`pdftex::CState`) and the
//! external-state record (`checkpoint::ExtRecord`).
//!
//! It is deliberately dumb: little-endian fixed-width integers, a length
//! before every sequence, fields in declaration order. A persisted snapshot
//! is keyed by the engine build (`checkpoint::engine_build`), so the layout
//! never has to be read by another build and carries no version of its own.
//!
//! Each C-part module registers its state with [`codec_struct!`] next to the
//! struct, where its private fields are visible.

use std::collections::{BTreeMap, BTreeSet, HashMap};

pub struct Reader<'a> {
    pub buf: &'a [u8],
    pub pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Reader<'a> {
        Reader { buf, pos: 0 }
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.pos + n > self.buf.len() {
            return Err("persisted state is truncated".into());
        }
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
}

pub trait Codec: Sized {
    fn enc(&self, w: &mut Vec<u8>);
    fn dec(r: &mut Reader) -> Result<Self, String>;
}

macro_rules! codec_int {
    ($($t:ty),*) => {$(
        impl Codec for $t {
            fn enc(&self, w: &mut Vec<u8>) {
                w.extend_from_slice(&self.to_le_bytes());
            }
            fn dec(r: &mut Reader) -> Result<Self, String> {
                let b = r.take(std::mem::size_of::<$t>())?;
                Ok(<$t>::from_le_bytes(b.try_into().unwrap()))
            }
        }
    )*};
}
codec_int!(u8, i8, u16, i16, u32, i32, u64, i64, f64);

impl Codec for usize {
    fn enc(&self, w: &mut Vec<u8>) {
        (*self as u64).enc(w)
    }
    fn dec(r: &mut Reader) -> Result<Self, String> {
        Ok(u64::dec(r)? as usize)
    }
}

impl Codec for bool {
    fn enc(&self, w: &mut Vec<u8>) {
        w.push(*self as u8)
    }
    fn dec(r: &mut Reader) -> Result<Self, String> {
        Ok(r.take(1)?[0] != 0)
    }
}

impl Codec for String {
    fn enc(&self, w: &mut Vec<u8>) {
        self.as_bytes().to_vec().enc(w)
    }
    fn dec(r: &mut Reader) -> Result<Self, String> {
        String::from_utf8(Vec::<u8>::dec(r)?).map_err(|e| e.to_string())
    }
}

impl<T: Codec> Codec for Vec<T> {
    fn enc(&self, w: &mut Vec<u8>) {
        self.len().enc(w);
        for x in self {
            x.enc(w);
        }
    }
    fn dec(r: &mut Reader) -> Result<Self, String> {
        let n = usize::dec(r)?;
        if n > r.buf.len() {
            return Err("persisted state: bad length".into());
        }
        (0..n).map(|_| T::dec(r)).collect()
    }
}

impl<T: Codec> Codec for Option<T> {
    fn enc(&self, w: &mut Vec<u8>) {
        match self {
            None => w.push(0),
            Some(x) => {
                w.push(1);
                x.enc(w);
            }
        }
    }
    fn dec(r: &mut Reader) -> Result<Self, String> {
        Ok(match r.take(1)?[0] {
            0 => None,
            _ => Some(T::dec(r)?),
        })
    }
}

impl<T: Codec> Codec for Box<T> {
    fn enc(&self, w: &mut Vec<u8>) {
        (**self).enc(w)
    }
    fn dec(r: &mut Reader) -> Result<Self, String> {
        Ok(Box::new(T::dec(r)?))
    }
}

impl<T: Codec, const N: usize> Codec for [T; N] {
    fn enc(&self, w: &mut Vec<u8>) {
        for x in self {
            x.enc(w);
        }
    }
    fn dec(r: &mut Reader) -> Result<Self, String> {
        let v: Vec<T> = (0..N).map(|_| T::dec(r)).collect::<Result<_, _>>()?;
        v.try_into()
            .map_err(|_| "persisted state: array length".to_string())
    }
}

impl<A: Codec, B: Codec> Codec for (A, B) {
    fn enc(&self, w: &mut Vec<u8>) {
        self.0.enc(w);
        self.1.enc(w);
    }
    fn dec(r: &mut Reader) -> Result<Self, String> {
        Ok((A::dec(r)?, B::dec(r)?))
    }
}

impl<A: Codec, B: Codec, C: Codec> Codec for (A, B, C) {
    fn enc(&self, w: &mut Vec<u8>) {
        self.0.enc(w);
        self.1.enc(w);
        self.2.enc(w);
    }
    fn dec(r: &mut Reader) -> Result<Self, String> {
        Ok((A::dec(r)?, B::dec(r)?, C::dec(r)?))
    }
}

impl<A: Codec, B: Codec, C: Codec, D: Codec> Codec for (A, B, C, D) {
    fn enc(&self, w: &mut Vec<u8>) {
        self.0.enc(w);
        self.1.enc(w);
        self.2.enc(w);
        self.3.enc(w);
    }
    fn dec(r: &mut Reader) -> Result<Self, String> {
        Ok((A::dec(r)?, B::dec(r)?, C::dec(r)?, D::dec(r)?))
    }
}

impl<K: Codec + Ord, V: Codec> Codec for BTreeMap<K, V> {
    fn enc(&self, w: &mut Vec<u8>) {
        self.len().enc(w);
        for (k, v) in self {
            k.enc(w);
            v.enc(w);
        }
    }
    fn dec(r: &mut Reader) -> Result<Self, String> {
        let n = usize::dec(r)?;
        let mut m = BTreeMap::new();
        for _ in 0..n {
            let k = K::dec(r)?;
            m.insert(k, V::dec(r)?);
        }
        Ok(m)
    }
}

impl<K: Codec + Ord> Codec for BTreeSet<K> {
    fn enc(&self, w: &mut Vec<u8>) {
        self.len().enc(w);
        for k in self {
            k.enc(w);
        }
    }
    fn dec(r: &mut Reader) -> Result<Self, String> {
        let n = usize::dec(r)?;
        (0..n).map(|_| K::dec(r)).collect()
    }
}

impl<K: Codec + Eq + std::hash::Hash, V: Codec> Codec for HashMap<K, V> {
    fn enc(&self, w: &mut Vec<u8>) {
        self.len().enc(w);
        for (k, v) in self {
            k.enc(w);
            v.enc(w);
        }
    }
    fn dec(r: &mut Reader) -> Result<Self, String> {
        let n = usize::dec(r)?;
        let mut m = HashMap::with_capacity(n);
        for _ in 0..n {
            let k = K::dec(r)?;
            m.insert(k, V::dec(r)?);
        }
        Ok(m)
    }
}

/// `codec_struct!(Type { field, field, ... })`: encode the fields in order.
/// Every field must be listed; a struct literal is built on decode, so a
/// missing one is a compile error.
#[macro_export]
macro_rules! codec_struct {
    ($t:ident { $($f:ident),* $(,)? }) => {
        impl $crate::persist::Codec for $t {
            fn enc(&self, w: &mut Vec<u8>) {
                $( $crate::persist::Codec::enc(&self.$f, w); )*
            }
            fn dec(r: &mut $crate::persist::Reader) -> Result<Self, String> {
                Ok($t { $( $f: $crate::persist::Codec::dec(r)?, )* })
            }
        }
    };
}

/// `codec_enum!(Type { Unit, Unit, ... })` for field-less enums.
#[macro_export]
macro_rules! codec_enum {
    ($t:ident { $($v:ident),* $(,)? }) => {
        impl $crate::persist::Codec for $t {
            fn enc(&self, w: &mut Vec<u8>) {
                let tags = [$($t::$v),*];
                let i = tags.iter().position(|x| x == self).unwrap() as u8;
                w.push(i);
            }
            fn dec(r: &mut $crate::persist::Reader) -> Result<Self, String> {
                let tags = [$($t::$v),*];
                let i = r.take(1)?[0] as usize;
                tags.get(i).copied().ok_or_else(|| format!("bad {}", stringify!($t)))
            }
        }
    };
}

/// A fast 128-bit content hash for read-sets and state comparison: not
/// cryptographic, but every byte feeds two independent 64-bit lanes with
/// full-avalanche mixing, and the length is folded in.
pub fn hash128(data: &[u8]) -> [u64; 2] {
    const K0: u64 = 0x9E37_79B9_7F4A_7C15;
    const K1: u64 = 0xC2B2_AE3D_27D4_EB4F;
    fn mix(x: u64) -> u64 {
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    let (mut a, mut b) = (
        K0 ^ data.len() as u64,
        K1.rotate_left(17) ^ data.len() as u64,
    );
    let mut chunks = data.chunks_exact(16);
    for c in &mut chunks {
        let x = u64::from_le_bytes(c[..8].try_into().unwrap());
        let y = u64::from_le_bytes(c[8..].try_into().unwrap());
        a = (a ^ x).wrapping_mul(K1).rotate_left(29).wrapping_add(y);
        b = (b ^ y).wrapping_mul(K0).rotate_left(31).wrapping_add(x);
    }
    let rest = chunks.remainder();
    let mut t = [0u8; 16];
    t[..rest.len()].copy_from_slice(rest);
    let x = u64::from_le_bytes(t[..8].try_into().unwrap());
    let y = u64::from_le_bytes(t[8..].try_into().unwrap());
    a = (a ^ x)
        .wrapping_mul(K1)
        .rotate_left(29)
        .wrapping_add(y ^ rest.len() as u64);
    b = (b ^ y).wrapping_mul(K0).rotate_left(31).wrapping_add(x);
    [mix(a ^ mix(b)), mix(b.wrapping_add(K0) ^ mix(a))]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let v: (Vec<Option<String>>, BTreeMap<Vec<u8>, (i32, bool)>) = (
            vec![None, Some("x".into())],
            [(b"k".to_vec(), (-3, true))].into_iter().collect(),
        );
        let mut w = vec![];
        v.enc(&mut w);
        let back =
            <(Vec<Option<String>>, BTreeMap<Vec<u8>, (i32, bool)>)>::dec(&mut Reader::new(&w))
                .unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn hash_differs_on_small_changes() {
        let a = hash128(b"hello world, this is a test");
        let b = hash128(b"hello world, this is a tesT");
        let c = hash128(b"hello world, this is a test ");
        assert!(a != b && a != c && b != c);
        assert_eq!(a, hash128(b"hello world, this is a test"));
    }
}
