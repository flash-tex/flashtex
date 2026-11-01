//! sortid.c: sorting the entries.

use crate::locale::{c_isupper, strcmp, tolower_c, Env};
use crate::mkind::*;
use crate::qsort::qqsort;
use crate::scanid::group_type;

impl Mk<'_> {
    pub fn sort_idx(&mut self) {
        self.message(b"Sorting entries...");
        // setlocale(LC_COLLATE, ""): only strcoll (-L, -T) reads it.
        if self.locale_sort {
            self.collate = Some(Env::collate());
        }
        self.idx_dc = 0;
        self.idx_gc = 0;
        let mut keys = std::mem::take(&mut self.idx_key);
        qqsort(&mut keys, &mut |a, b| self.compare(a, b));
        self.idx_key = keys;
        let m = fmt("done (%ld comparisons).\n", &[self.idx_gc.into()]);
        self.message(&m);
    }

    fn compare(&mut self, a: usize, b: usize) -> i32 {
        self.idx_gc += 1;
        self.idx_dot_(CMP_MAX);

        let mut dif = 0;
        let mut i = 0;
        while i < FIELD_MAX {
            // compare the sort fields
            dif = self.compare_one(a, b, true, i);
            if dif != 0 {
                break;
            }
            // compare the actual fields
            dif = self.compare_one(a, b, false, i);
            if dif != 0 {
                break;
            }
            i += 1;
        }

        // both key aggregates are identical, compare page numbers
        if i == FIELD_MAX {
            dif = self.compare_page(a, b);
        }
        dif
    }

    #[allow(clippy::if_same_then_else)] // the C's branches, kept
    fn compare_one(&self, a: usize, b: usize, sort: bool, i: usize) -> i32 {
        let (x, y) = if sort {
            (&self.nodes[a].sf[i], &self.nodes[b].sf[i])
        } else {
            (&self.nodes[a].af[i], &self.nodes[b].af[i])
        };

        if x.is_empty() && y.is_empty() {
            return 0;
        }
        if x.is_empty() {
            return -1;
        }
        if y.is_empty() {
            return 1;
        }

        let m = group_type(x);
        let n = group_type(y);

        // both pure digits
        if m >= 0 && n >= 0 {
            return m.wrapping_sub(n);
        }

        // x digit, y non-digit
        if m >= 0 {
            return if self.german_sort {
                1
            } else if n == -1 {
                1
            } else {
                -1
            };
        }
        // x non-digit, y digit
        if n >= 0 {
            return if self.german_sort {
                -1
            } else if m == -1 {
                -1
            } else {
                1
            };
        }
        // strings started with a symbol (including digit)
        if m == SYMBOL && n == SYMBOL {
            return self.check_mixsym(x, y);
        }

        // x symbol, y non-symbol
        if m == SYMBOL {
            return -1;
        }

        // x non-symbol, y symbol
        if n == SYMBOL {
            return 1;
        }

        // strings with a leading letter, the ALPHA type
        self.compare_string(x, y)
    }

    fn strcoll(&self, x: &[u8], y: &[u8]) -> i32 {
        match &self.collate {
            Some(e) => e.strcoll(x, y),
            None => strcmp(x, y),
        }
    }

    fn check_mixsym(&self, x: &[u8], y: &[u8]) -> i32 {
        let m = at(x, 0).is_ascii_digit();
        let n = at(y, 0).is_ascii_digit();

        if m && !n {
            return 1;
        }
        if !m && n {
            return -1;
        }
        if self.locale_sort {
            self.strcoll(x, y)
        } else {
            strcmp(x, y)
        }
    }

    /// `compare_string`. Indices may run past a string's NUL (with
    /// `-l`, a skipped blank); the bytes there read as NUL.
    pub fn compare_string(&self, a: &[u8], b: &[u8]) -> i32 {
        let mut i = 0;
        let mut j = 0;

        if self.locale_sort {
            return self.strcoll(a, b);
        }

        while at(a, i) != 0 || at(b, j) != 0 {
            if at(a, i) == 0 {
                return -1;
            }
            if at(b, j) == 0 {
                return 1;
            }
            if self.letter_ordering {
                if at(a, i) as i32 == SPC {
                    i += 1;
                }
                if at(b, j) as i32 == SPC {
                    j += 1;
                }
            }
            let al = tolower_c(at(a, i)) as i32;
            let bl = tolower_c(at(b, j)) as i32;

            if al != bl {
                return al - bl;
            }
            i += 1;
            j += 1;
        }
        if self.german_sort {
            new_strcmp(a, b)
        } else {
            strcmp(a, b)
        }
    }

    fn compare_page(&mut self, a: usize, b: usize) -> i32 {
        let mut m: i32 = 0;
        let mut i: i16 = 0;
        let (ca, cb) = (self.nodes[a].count, self.nodes[b].count);
        while i < ca && i < cb {
            m = self.nodes[a].npg[i as usize].wrapping_sub(self.nodes[b].npg[i as usize]);
            if m != 0 {
                break;
            }
            i += 1;
        }
        if m == 0 {
            // common leading page numbers match
            if i == ca && i == cb {
                // all page numbers match
                let ea = at(&self.nodes[a].encap, 0);
                let eb = at(&self.nodes[b].encap, 0);
                let isrange = |c: u8| c == self.idx_ropen || c == self.idx_rclose;
                if isrange(ea) && isrange(eb) {
                    // Order two range values by input line number
                    m = self.nodes[a].lc.wrapping_sub(self.nodes[b].lc);
                } else if self.nodes[a].encap == self.nodes[b].encap {
                    // If neither are yet marked duplicate, mark the
                    // second of them to be ignored.
                    if self.nodes[a].type_ != DUPLICATE && self.nodes[b].type_ != DUPLICATE {
                        self.nodes[b].type_ = DUPLICATE;
                    }
                    // leave m == 0 to show equality
                } else if isrange(ea) || isrange(eb) {
                    m = self.nodes[a].lc.wrapping_sub(self.nodes[b].lc);
                } else {
                    let (x, y) = (self.nodes[a].encap.clone(), self.nodes[b].encap.clone());
                    m = self.compare_string(&x, &y);
                }
            } else if i == ca && i < cb {
                m = -1;
            } else if i < ca && i == cb {
                m = 1;
            }
        }
        m
    }
}

/// `new_strcmp(s1, s2, GERMAN)`.
fn new_strcmp(s1: &[u8], s2: &[u8]) -> i32 {
    let mut i = 0;
    while at(s1, i) == at(s2, i) {
        if at(s1, i) == 0 {
            return 0;
        }
        i += 1;
    }
    if c_isupper(at(s1, i) as i32) {
        1
    } else {
        -1
    }
}
