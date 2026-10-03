//! Copy-on-write containers for pdfTeX's C-part state, so that a checkpoint
//! (`crate::checkpoint`, DESIGN.md §5.2) copies it in O(1).
//!
//! A checkpoint clones [`super::CState`]. Most of it is read-mostly -- the
//! font map (thousands of entries, read once per run), the encodings, the
//! glyph-to-Unicode table -- or grows by a few entries per page -- the
//! named-object maps of `avlstuff`. Cloned whole, the font map alone cost
//! 3.1 ms per checkpoint (docs/evidence/p4-l1-2026-09-29).
//!
//! * [`Shared<T>`] is an `Arc<T>`: cloning it is a reference-count increment,
//!   and the first write after a clone copies `T` (`Arc::make_mut`). Reads
//!   and writes look like those of a plain `T` (`Deref`, `DerefMut`), so the
//!   ported C code is unchanged.
//! * [`ShardMap<K, V>`] is a map of [`SHARDS`] `Shared` hash maps chosen by
//!   the key's hash: an insertion after a checkpoint copies one shard, a
//!   1/64th of the map, instead of all of it.
//!
//! A `Shared` value must not be written through a reference obtained before
//! a clone was taken; `DerefMut` takes `&mut self`, so the borrow checker
//! already rules that out.

use crate::persist::{Codec, Reader};
use std::collections::HashMap;
use std::hash::{BuildHasher, Hash};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// A copy-on-write value: see the module documentation.
#[derive(Default)]
pub struct Shared<T: Clone>(Arc<T>);

impl<T: Clone> Shared<T> {
    pub fn new(x: T) -> Shared<T> {
        Shared(Arc::new(x))
    }

    /// Whether `a` and `b` are the same copy (for tests and caches).
    pub fn ptr_eq(a: &Shared<T>, b: &Shared<T>) -> bool {
        Arc::ptr_eq(&a.0, &b.0)
    }
}

impl<T: Clone> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared(Arc::clone(&self.0))
    }
}

impl<T: Clone> Deref for Shared<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Clone> DerefMut for Shared<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        let _m = crate::memstat::scope(crate::memstat::tag::COW);
        Arc::make_mut(&mut self.0)
    }
}

impl<T: Clone + Codec> Codec for Shared<T> {
    fn enc(&self, w: &mut Vec<u8>) {
        (*self.0).enc(w)
    }
    fn dec(r: &mut Reader) -> Result<Self, String> {
        Ok(Shared::new(T::dec(r)?))
    }
}

impl<T: Clone + std::fmt::Debug> std::fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (*self.0).fmt(f)
    }
}

/// Shards of a [`ShardMap`].
pub const SHARDS: usize = 64;

/// A hash map split into [`SHARDS`] copy-on-write shards (lookups and
/// first-come insertions only need the one shard; iteration order is not
/// defined, as for a `HashMap`).
#[derive(Clone)]
pub struct ShardMap<K: Clone + Eq + Hash, V: Clone> {
    shards: Arc<[Shared<HashMap<K, V>>; SHARDS]>,
    len: usize,
}

impl<K: Clone + Eq + Hash, V: Clone> Default for ShardMap<K, V> {
    fn default() -> Self {
        ShardMap {
            shards: Arc::new(std::array::from_fn(|_| Shared::default())),
            len: 0,
        }
    }
}

/// A fixed hasher, so that a key lands in the same shard in every process
/// (the persisted state is decoded key by key, so this is only for speed).
fn shard_of<K: Hash>(k: &K) -> usize {
    let h = std::hash::BuildHasherDefault::<std::collections::hash_map::DefaultHasher>::default()
        .hash_one(k);
    (h as usize) % SHARDS
}

impl<K: Clone + Eq + Hash, V: Clone> ShardMap<K, V> {
    pub fn get(&self, k: &K) -> Option<&V> {
        self.shards[shard_of(k)].get(k)
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The value under `k`, inserting `v` first if there is none (the C
    /// code's `avl_probe`: an existing entry is never replaced).
    pub fn get_or_insert(&mut self, k: K, v: V) -> &V {
        let i = shard_of(&k);
        if self.shards[i].contains_key(&k) {
            return self.shards[i].get(&k).unwrap();
        }
        self.len += 1;
        let _m = crate::memstat::scope(crate::memstat::tag::COW);
        let shards = Arc::make_mut(&mut self.shards);
        shards[i].entry(k).or_insert(v)
    }

    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        let i = shard_of(&k);
        let _m = crate::memstat::scope(crate::memstat::tag::COW);
        let shards = Arc::make_mut(&mut self.shards);
        let old = shards[i].insert(k, v);
        if old.is_none() {
            self.len += 1;
        }
        old
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.shards.iter().flat_map(|s| s.iter())
    }
}

impl<K: Clone + Eq + Hash, V: Clone + PartialEq> ShardMap<K, V> {
    /// The same entries (shards that are the same copy without a look).
    pub fn same_as(&self, o: &ShardMap<K, V>) -> bool {
        self.len == o.len
            && self
                .shards
                .iter()
                .zip(o.shards.iter())
                .all(|(a, b)| Shared::ptr_eq(a, b) || **a == **b)
    }
}

impl<K: Clone + Eq + Hash + Codec, V: Clone + Codec> Codec for ShardMap<K, V> {
    fn enc(&self, w: &mut Vec<u8>) {
        self.len.enc(w);
        for (k, v) in self.iter() {
            k.enc(w);
            v.enc(w);
        }
    }
    fn dec(r: &mut Reader) -> Result<Self, String> {
        let n = usize::dec(r)?;
        if n > r.buf.len() {
            return Err("persisted state: bad length".into());
        }
        let mut m = ShardMap::default();
        for _ in 0..n {
            let k = K::dec(r)?;
            let v = V::dec(r)?;
            m.insert(k, v);
        }
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clone_is_unaffected_by_later_writes() {
        let mut a: Shared<Vec<i32>> = Shared::new(vec![1, 2]);
        let b = a.clone();
        assert!(Shared::ptr_eq(&a, &b));
        a.push(3);
        assert_eq!(*b, vec![1, 2]);
        assert_eq!(*a, vec![1, 2, 3]);
        assert!(!Shared::ptr_eq(&a, &b));
    }

    #[test]
    fn shard_map_copies_one_shard() {
        let mut m: ShardMap<i32, i32> = ShardMap::default();
        for i in 0..1000 {
            m.get_or_insert(i, i * 2);
        }
        let snap = m.clone();
        assert_eq!(*m.get_or_insert(5, 99), 10, "first insertion wins");
        m.insert(5000, 1);
        assert_eq!(snap.get(&5000), None);
        assert_eq!(m.get(&5000), Some(&1));
        let same = (0..SHARDS)
            .filter(|&i| Shared::ptr_eq(&m.shards[i], &snap.shards[i]))
            .count();
        assert_eq!(same, SHARDS - 1);
        let mut w = vec![];
        m.enc(&mut w);
        let back = ShardMap::<i32, i32>::dec(&mut Reader::new(&w)).unwrap();
        assert_eq!(back.len(), m.len());
        assert_eq!(back.get(&999), Some(&1998));
    }
}
