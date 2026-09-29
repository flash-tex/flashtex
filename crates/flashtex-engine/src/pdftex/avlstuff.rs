//! `avlstuff.c`, ported: PDF objects found by number or by name.
//!
//! The C code keeps one AVL tree per object type, keyed by `obj_tab[].int0`:
//! a nonnegative number, or minus a string number, compared by the string's
//! contents. Only lookups by key are ever made, and an insertion never
//! replaces an existing entry (`avl_probe`), so a map per type that keeps
//! the first object inserted under each key behaves identically.

use super::shared::ShardMap;
use super::with_state;
use crate::generated::Globals;
use std::collections::HashMap;

#[derive(Clone, PartialEq, Eq, Hash)]
enum Key {
    Num(i32),
    Name(Vec<u8>),
}

impl crate::persist::Codec for Key {
    fn enc(&self, w: &mut Vec<u8>) {
        match self {
            Key::Num(n) => {
                w.push(0);
                n.enc(w);
            }
            Key::Name(b) => {
                w.push(1);
                b.enc(w);
            }
        }
    }
    fn dec(r: &mut crate::persist::Reader) -> Result<Self, String> {
        Ok(match r.take(1)?[0] {
            0 => Key::Num(i32::dec(r)?),
            _ => Key::Name(Vec::<u8>::dec(r)?),
        })
    }
}

/// One map per object type, each a [`ShardMap`]: named destinations grow by
/// a few per page (hyperref), and a checkpoint after every page must not
/// copy them all (`super::shared`).
#[derive(Default, Clone)]
pub struct State {
    trees: HashMap<i32, ShardMap<Key, i32>>,
}

// Checkpoint registration (crate::checkpoint): the state is cloned at a
// checkpoint and persisted with a snapshot.
crate::codec_struct!(State { trees });

impl Globals {
    fn avl_key(&self, int0: i32) -> Key {
        if int0 < 0 {
            Key::Name(self.str_bytes(-int0))
        } else {
            Key::Num(int0)
        }
    }

    /// `avlputobj`.
    pub fn avl_put_obj(&mut self, objptr: i32, t: i32) {
        let key = self.avl_key(self.obj_tab[objptr as usize].int0);
        with_state(|s| {
            s.avl.trees.entry(t).or_default().get_or_insert(key, objptr);
        });
    }

    /// `avlfindobj`: the object of type `t` with number or name `i`, or 0.
    pub fn avl_find_obj(&mut self, t: i32, i: i32, byname: i32) -> i32 {
        let key = self.avl_key(if byname > 0 { -i } else { i });
        with_state(|s| {
            s.avl
                .trees
                .get(&t)
                .and_then(|m| m.get(&key))
                .copied()
                .unwrap_or(0)
        })
    }
}

impl State {
    /// The same objects under the same keys (`CState::same_as`).
    pub fn same_as(&self, o: &State) -> bool {
        self.trees.len() == o.trees.len()
            && self
                .trees
                .iter()
                .all(|(t, m)| o.trees.get(t).is_some_and(|n| m.same_as(n)))
    }
}
