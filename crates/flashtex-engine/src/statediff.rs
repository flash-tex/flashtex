//! Where two engine states differ, word by word, named (DESIGN.md §5.3).
//!
//! The convergence test (`crate::incr`) compares the live word space with an
//! old run's checkpoint (`Arena::diff_branch`). This module turns differing
//! chunks into differing words and names each one: the array and element,
//! or, in the scalar region, the scalar global -- the names come from the
//! generated `Globals::visit_scalars`, which lists them in the order the
//! spill lays them out.

use crate::arena::{ChunkDiff, Visit, CHUNK_BYTES, CHUNK_WORDS};
use crate::generated::Globals;

/// A scalar global's place in the scalar region.
#[derive(Clone, Debug)]
pub struct ScalarSlot {
    pub name: &'static str,
    pub off: usize,
    pub size: usize,
}

/// The scalar globals in spill order, with their offsets.
pub fn scalar_layout(g: &mut Globals) -> Vec<ScalarSlot> {
    static NAMES: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    let names = NAMES.get_or_init(|| {
        let src = include_str!("generated/globals.rs");
        let body = src
            .split("pub fn visit_scalars")
            .nth(1)
            .and_then(|b| b.split("\n    }\n").next())
            .unwrap_or("");
        body.lines()
            .filter_map(|l| {
                let l = l.trim();
                let rest = l
                    .strip_prefix("v.pod(&mut self.")
                    .or_else(|| l.strip_prefix("v.arr_len(&mut self."))?;
                rest.strip_suffix(");")
            })
            .collect()
    });
    struct Sizes(Vec<usize>);
    impl Visit for Sizes {
        fn pod<T: Copy>(&mut self, _x: &mut T) {
            self.0.push(std::mem::size_of::<T>());
        }
    }
    let mut v = Sizes(vec![]);
    g.visit_scalars(&mut v);
    let mut off = 0;
    let mut out = Vec::with_capacity(v.0.len());
    for (i, &size) in v.0.iter().enumerate() {
        out.push(ScalarSlot {
            name: names.get(i).copied().unwrap_or("?"),
            off,
            size,
        });
        // `Spill` pads every value to its slot (a multiple of 8 bytes).
        off += size.div_ceil(8) * 8;
    }
    out
}

/// One differing 8-byte word of the word space.
#[derive(Clone, Debug)]
pub struct WordDiff {
    /// Byte offset in the space.
    pub off: usize,
    pub old: u64,
    pub new: u64,
    /// The array (or `(scalars)`) holding it.
    pub region: &'static str,
    /// The element index in the array (the scalar's name for scalars).
    pub index: usize,
    pub scalar: Option<&'static str>,
}

impl std::fmt::Display for WordDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.scalar {
            Some(n) => write!(f, "{n}"),
            None => write!(f, "{}[{}]", self.region, self.index),
        }?;
        write!(f, ": {:#x} -> {:#x}", self.old, self.new)
    }
}

/// The differing words of `d`, named.
pub fn words(g: &mut Globals, d: &ChunkDiff) -> Vec<WordDiff> {
    let layout = scalar_layout(g);
    let mut out = vec![];
    for &(c, old, new) in &d.differing {
        // SAFETY: `ChunkDiff` points at CHUNK_WORDS words each, valid while
        // the arena and the pending branch are unchanged (they are: `g` is
        // only read here).
        let (a, b) = unsafe {
            (
                std::slice::from_raw_parts(old, CHUNK_WORDS),
                std::slice::from_raw_parts(new, CHUNK_WORDS),
            )
        };
        for w in 0..CHUNK_WORDS {
            if a[w] == b[w] {
                continue;
            }
            let off = c as usize * CHUNK_BYTES + w * 8;
            let (r, rel) = g.arena.region_at(off);
            let (index, scalar) = if r.name == "(scalars)" {
                let s = layout
                    .iter()
                    .rev()
                    .find(|s| s.off <= rel)
                    .map(|s| s.name)
                    .unwrap_or("?");
                (rel, Some(s))
            } else {
                (rel / r.elem.max(1), None)
            };
            out.push(WordDiff {
                off,
                old: a[w],
                new: b[w],
                region: r.name,
                index,
                scalar,
            });
        }
    }
    out
}

/// A summary line per array or scalar: how many words differ there.
pub fn summary(ws: &[WordDiff]) -> String {
    let mut m: std::collections::BTreeMap<String, usize> = Default::default();
    for w in ws {
        let k = match w.scalar {
            Some(n) => n.to_string(),
            None => w.region.to_string(),
        };
        *m.entry(k).or_default() += 1;
    }
    let mut v: Vec<_> = m.into_iter().collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.1));
    v.iter()
        .map(|(k, n)| format!("{k}:{n}"))
        .collect::<Vec<_>>()
        .join(" ")
}
