//! `avlstuff.c`, ported: PDF objects found by number or by name.
//!
//! The C code keeps one AVL tree per object type, keyed by `obj_tab[].int0`:
//! a nonnegative number, or minus a string number, compared by the string's
//! contents. Only lookups by key are ever made, and an insertion never
//! replaces an existing entry (`avl_probe`), so a map per type that keeps
//! the first object inserted under each key behaves identically.

use super::with_state;
use crate::generated::Globals;
use std::collections::HashMap;

#[derive(Clone, PartialEq, Eq, Hash)]
enum Key {
    Num(i32),
    Name(Vec<u8>),
}

#[derive(Default)]
pub struct State {
    trees: HashMap<i32, HashMap<Key, i32>>,
}

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
            s.avl
                .trees
                .entry(t)
                .or_default()
                .entry(key)
                .or_insert(objptr);
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
