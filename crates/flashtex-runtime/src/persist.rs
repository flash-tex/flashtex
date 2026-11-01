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
    let (blocks, rest) = data.as_chunks::<16>();
    for c in blocks {
        let x = u64::from_le_bytes(c[..8].try_into().unwrap());
        let y = u64::from_le_bytes(c[8..].try_into().unwrap());
        a = (a ^ x).wrapping_mul(K1).rotate_left(29).wrapping_add(y);
        b = (b ^ y).wrapping_mul(K0).rotate_left(31).wrapping_add(x);
    }
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

/// [`hash128`] of the first `limit` bytes of the file at `path` (all of it
/// when `None`; the whole file when it is shorter), read through a fixed
/// buffer: the same value as `hash128` of what `fs::read` gives, truncated
/// to `limit`, without holding the file in memory (lane MEM-MODES: a 15 MB
/// format, read whole only to be hashed, left a 15 MB free block the macOS
/// allocator keeps). Also returns the number of bytes hashed. A file whose
/// length changes while it is read is read whole and hashed in one piece,
/// so the result is always the hash of bytes the file held.
pub fn hash128_file(path: &str, limit: Option<u64>) -> std::io::Result<([u64; 2], u64)> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let len = f.metadata()?.len();
    let n = limit.map_or(len, |l| l.min(len));
    let mut h = Hash128::new(n);
    let mut buf = [0u8; 64 << 10];
    let mut left = n;
    let mut exact = true;
    while left > 0 {
        let want = (left as usize).min(buf.len());
        let got = match f.read(&mut buf[..want]) {
            Ok(0) => {
                exact = false; // shorter than it was
                break;
            }
            Ok(k) => k,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        h.update(&buf[..got]);
        left -= got as u64;
    }
    if exact && limit.is_none_or(|l| l > n) {
        // all of it was wanted: it must end here
        exact = loop {
            match f.read(&mut buf[..1]) {
                Ok(k) => break k == 0,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        };
    }
    if exact {
        return Ok((h.finish(), n));
    }
    let mut d = std::fs::read(path)?;
    if let Some(l) = limit {
        d.truncate(l.min(d.len() as u64) as usize);
    }
    Ok((hash128(&d), d.len() as u64))
}

/// [`hash128`] fed in pieces: the total length is given first, as the hash
/// folds it in before any byte.
struct Hash128 {
    a: u64,
    b: u64,
    want: u64,
    fed: u64,
    tail: [u8; 16],
    tail_len: usize,
}

impl Hash128 {
    const K0: u64 = 0x9E37_79B9_7F4A_7C15;
    const K1: u64 = 0xC2B2_AE3D_27D4_EB4F;

    fn new(len: u64) -> Hash128 {
        Hash128 {
            a: Self::K0 ^ len,
            b: Self::K1.rotate_left(17) ^ len,
            want: len,
            fed: 0,
            tail: [0; 16],
            tail_len: 0,
        }
    }

    #[inline]
    fn block(&mut self, c: &[u8; 16]) {
        let x = u64::from_le_bytes(c[..8].try_into().unwrap());
        let y = u64::from_le_bytes(c[8..].try_into().unwrap());
        self.a = (self.a ^ x)
            .wrapping_mul(Self::K1)
            .rotate_left(29)
            .wrapping_add(y);
        self.b = (self.b ^ y)
            .wrapping_mul(Self::K0)
            .rotate_left(31)
            .wrapping_add(x);
    }

    fn update(&mut self, mut data: &[u8]) {
        self.fed += data.len() as u64;
        if self.tail_len > 0 {
            let k = (16 - self.tail_len).min(data.len());
            self.tail[self.tail_len..self.tail_len + k].copy_from_slice(&data[..k]);
            self.tail_len += k;
            data = &data[k..];
            if self.tail_len < 16 {
                return;
            }
            let t = self.tail;
            self.block(&t);
            self.tail_len = 0;
        }
        let (blocks, rest) = data.as_chunks::<16>();
        for c in blocks {
            self.block(c);
        }
        self.tail[..rest.len()].copy_from_slice(rest);
        self.tail_len = rest.len();
    }

    fn finish(self) -> [u64; 2] {
        debug_assert_eq!(self.fed, self.want);
        let (k0, k1) = (Self::K0, Self::K1);
        fn mix(x: u64) -> u64 {
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }
        let mut t = [0u8; 16];
        t[..self.tail_len].copy_from_slice(&self.tail[..self.tail_len]);
        let x = u64::from_le_bytes(t[..8].try_into().unwrap());
        let y = u64::from_le_bytes(t[8..].try_into().unwrap());
        let a = (self.a ^ x)
            .wrapping_mul(k1)
            .rotate_left(29)
            .wrapping_add(y ^ self.tail_len as u64);
        let b = (self.b ^ y)
            .wrapping_mul(k0)
            .rotate_left(31)
            .wrapping_add(x);
        [mix(a ^ mix(b)), mix(b.wrapping_add(k0) ^ mix(a))]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Sample = (Vec<Option<String>>, BTreeMap<Vec<u8>, (i32, bool)>);

    #[test]
    fn round_trip() {
        let v: Sample = (
            vec![None, Some("x".into())],
            [(b"k".to_vec(), (-3, true))].into_iter().collect(),
        );
        let mut w = vec![];
        v.enc(&mut w);
        let back = Sample::dec(&mut Reader::new(&w)).unwrap();
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

    /// The hash fed in pieces is `hash128`'s, for every length and split.
    #[test]
    fn hash_in_pieces_is_hash128() {
        let data: Vec<u8> = (0..300u32)
            .map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8)
            .collect();
        for n in 0..data.len() {
            let d = &data[..n];
            for cut in [0, 1, 7, 15, 16, 17, 33, n / 2, n] {
                let cut = cut.min(n);
                for cut2 in [cut, (cut + 5).min(n), (cut + 16).min(n)] {
                    let mut h = Hash128::new(n as u64);
                    h.update(&d[..cut]);
                    h.update(&d[cut..cut2]);
                    h.update(&d[cut2..]);
                    assert_eq!(h.finish(), hash128(d), "n={n} cut={cut},{cut2}");
                }
            }
        }
    }

    /// A file's hash read through the buffer is `hash128` of `fs::read`'s
    /// bytes, whole or a prefix, across the buffer's size.
    #[test]
    fn hash_of_a_file_is_hash128_of_its_bytes() {
        let dir = std::env::temp_dir().join(format!("ftx-h128-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for n in [0usize, 1, 15, 16, 17, 65_535, 65_536, 65_537, 200_003] {
            let data: Vec<u8> = (0..n).map(|i| (i * 31 % 251) as u8).collect();
            let p = dir.join(format!("f{n}"));
            std::fs::write(&p, &data).unwrap();
            let p = p.to_str().unwrap();
            assert_eq!(hash128_file(p, None).unwrap(), (hash128(&data), n as u64));
            for l in [0u64, 3, 16, 65_536, n as u64, n as u64 + 9] {
                let m = (l as usize).min(n);
                assert_eq!(
                    hash128_file(p, Some(l)).unwrap(),
                    (hash128(&data[..m]), m as u64),
                    "n={n} limit={l}"
                );
            }
        }
        assert!(hash128_file(dir.join("none").to_str().unwrap(), None).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
