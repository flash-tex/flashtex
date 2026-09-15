//! Sorted index over a `&[(char, T)]` glyph table (issue #65).
//!
//! The metric and encoding tables are looked up once per character of every
//! math atom, and a character none of them carries (a digit, `+`, `x`) used to
//! walk all of them linearly. The index answers exactly what
//! `table.iter().find(|(c, _)| *c == ch)` answers, first occurrence included,
//! with a binary search.

use std::sync::OnceLock;

pub(crate) struct CharTable<T: 'static> {
    source: &'static [(char, T)],
    sorted: OnceLock<Vec<(char, T)>>,
}

impl<T: Copy + Send + Sync + 'static> CharTable<T> {
    pub(crate) const fn new(source: &'static [(char, T)]) -> Self {
        Self {
            source,
            sorted: OnceLock::new(),
        }
    }

    /// The value of the FIRST entry for `ch` in the source table.
    pub(crate) fn get(&self, ch: char) -> Option<T> {
        let sorted = self.sorted.get_or_init(|| {
            let mut sorted = self.source.to_vec();
            // Stable sort keeps duplicates in source order, and `dedup_by_key`
            // keeps the first of each run: the entry `find` would return.
            sorted.sort_by_key(|(c, _)| *c);
            sorted.dedup_by_key(|(c, _)| *c);
            sorted
        });
        sorted
            .binary_search_by_key(&ch, |(c, _)| *c)
            .ok()
            .map(|index| sorted[index].1)
    }
}

#[cfg(test)]
mod tests {
    use super::CharTable;

    const TABLE: &[(char, u16)] = &[('b', 2), ('a', 1), ('b', 3), ('z', 26)];

    #[test]
    fn matches_linear_find_including_duplicates() {
        static INDEX: CharTable<u16> = CharTable::new(TABLE);
        for ch in ['a', 'b', 'c', 'z', '\u{0}', '\u{10FFFF}'] {
            let linear = TABLE.iter().find(|(c, _)| *c == ch).map(|(_, v)| *v);
            assert_eq!(INDEX.get(ch), linear, "{ch:?}");
        }
    }

    #[test]
    fn crate_tables_index_like_linear_find() {
        let tables: [&'static [(char, u16)]; 4] = [
            crate::lm_math::ADVANCES,
            crate::newcm_math::ADVANCES,
            crate::amssymb::LM_ADVANCES,
            crate::amssymb::NEWCM_ADVANCES,
        ];
        for table in tables {
            let index = CharTable::new(table);
            for code in (0..0x3_0000u32).filter_map(char::from_u32) {
                let linear = table.iter().find(|(c, _)| *c == code).map(|(_, v)| *v);
                assert_eq!(index.get(code), linear, "U+{:04X}", code as u32);
            }
        }
        for table in [crate::export::SYMBOL_ENCODING, crate::export::WINANSI_HIGH] {
            let index = CharTable::new(table);
            for code in (0..0x3_0000u32).filter_map(char::from_u32) {
                let linear = table.iter().find(|(c, _)| *c == code).map(|(_, v)| *v);
                assert_eq!(index.get(code), linear, "U+{:04X}", code as u32);
            }
        }
    }
}
