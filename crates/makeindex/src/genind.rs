//! genind.c and genind.h: writing the `.ind` file.

use crate::locale::{c_isdigit, tolower_c, Env};
use crate::mkind::*;
use crate::Exit;

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    let mut v = Vec::new();
    for p in parts {
        v.extend_from_slice(cstr(p));
    }
    v
}

impl Mk<'_> {
    fn n(&self, k: Option<usize>) -> &Field {
        &self.nodes[k.expect("makeindex: an entry pointer is NULL")]
    }

    /// `IND_ERROR` and friends.
    fn ind_error(&mut self, msg: &[u8]) {
        if self.idx_dot {
            self.ilg(b"\n");
            self.idx_dot = false;
        }
        let (fname, lc) = {
            let c = self.n(self.curr);
            (c.fn_.clone(), c.lc)
        };
        let h = fmt(
            "## Warning (input = %s, line = %d; output = %s, line = %d):\n   -- ",
            &[
                P::S(&fname),
                lc.into(),
                P::S(&self.ind_fn.clone()),
                (self.ind_lc + 1).into(),
            ],
        );
        self.ilg(&h);
        self.ilg(msg);
        self.ind_ec += 1;
    }

    /// `PUTLN(S)`.
    fn putln(&mut self, s: &[u8]) {
        self.ind(s);
        self.ind(b"\n");
        self.ind_lc += 1;
    }

    /// `SAVE`.
    fn save(&mut self) {
        self.begin = self.curr;
        self.the_end = self.curr;
        self.prev_encap = Some(self.encap.clone());
    }

    pub fn gen_ind(&mut self) -> crate::R<()> {
        let m = fmt(
            "Generating output file %s...",
            &[P::S(&self.ind_fn.clone())],
        );
        self.message(&m);
        let pre = self.preamble.clone();
        self.ind(&pre);
        self.ind_lc += self.prelen;
        if self.init_page {
            self.insert_page();
        }

        // reset counters for putting out dots
        self.idx_dc = 0;
        for n in 0..self.idx_gt.max(0) as usize {
            if n >= self.idx_key.len() {
                break;
            }
            if self.nodes[self.idx_key[n]].type_ != DUPLICATE {
                if n != 0 && self.curr.is_none() {
                    // The first entry was a duplicate: make_entry compares
                    // with `prev`, a NULL pointer, and the C program dies
                    // of SIGSEGV.
                    return Err(Exit(crate::KILLED_BY_SIGSEGV));
                }
                if self.make_entry(n) {
                    self.idx_dot_(DOT_MAX);
                }
            }
        }
        if self.in_range {
            self.curr = self.range_ptr;
            let m = fmt(
                "Unmatched range opening operator %c.\n",
                &[P::C(self.idx_ropen)],
            );
            self.ind_error(&m);
        }
        self.prev = self.curr;
        self.flush_line(true);
        let t = self.delim_t.clone();
        self.ind(&t);
        let post = self.postamble.clone();
        self.ind(&post);
        let tmp_lc = self.ind_lc + self.postlen;
        if self.ind_ec == 1 {
            self.done(tmp_lc, "lines written", self.ind_ec, "warning");
        } else {
            self.done(tmp_lc, "lines written", self.ind_ec, "warnings");
        }
        Ok(())
    }

    fn make_entry(&mut self, n: usize) -> bool {
        // determine current and previous pointer
        self.prev = self.curr;
        self.curr = Some(self.idx_key[n]);
        // check if current entry is in range
        let cenc = self.n(self.curr).encap.clone();
        let c0 = at(&cenc, 0);
        if c0 == self.idx_ropen || c0 == self.idx_rclose {
            self.encap = cenc.get(1..).unwrap_or(&[]).to_vec();
        } else {
            self.encap = cenc.clone();
        }

        // determine the current nesting level
        if n == 0 {
            self.prev_level = 0;
            self.level = 0;
            let let_ = at(&self.n(self.curr).sf[0], 0) as i8 as i32;
            self.put_header(let_, None);
            self.make_item(b"");
        } else {
            self.prev_level = self.level;
            self.level = 0;
            while self.level < FIELD_MAX {
                let c = self.n(self.curr);
                let p = self.n(self.prev);
                if c.sf[self.level] != p.sf[self.level] || c.af[self.level] != p.af[self.level] {
                    break;
                }
                self.level += 1;
            }
            if self.level < FIELD_MAX {
                self.new_entry();
            }
            // Repeat test from just below to see if we are already in an
            // open range.  If so, we don't want to output anything.
            else if !(c0 == self.idx_ropen && self.in_range) {
                self.old_entry();
            }
        }

        if c0 == self.idx_ropen {
            if self.in_range {
                let m = fmt(
                    "Extra range opening operator %c.\n",
                    &[P::C(self.idx_ropen)],
                );
                self.ind_error(&m);
            } else {
                self.in_range = true;
                self.range_ptr = self.curr;
            }
        } else if c0 == self.idx_rclose {
            if self.in_range {
                self.in_range = false;
                let rest = cenc.get(1..).unwrap_or(&[]).to_vec();
                if !rest.is_empty() && self.prev_encap.as_deref() != Some(&rest[..]) {
                    let m = fmt(
                        "Range closing operator has an inconsistent encapsulator %s.\n",
                        &[P::S(&rest)],
                    );
                    self.ind_error(&m);
                }
            } else {
                let m = fmt(
                    "Unmatched range closing operator %c.\n",
                    &[P::C(self.idx_rclose)],
                );
                self.ind_error(&m);
            }
        } else if c0 != 0 && self.prev_encap.as_deref() != Some(&cenc[..]) && self.in_range {
            let m = fmt(
                "Inconsistent page encapsulator %s within range.\n",
                &[P::S(&cenc)],
            );
            self.ind_error(&m);
        }
        true
    }

    fn make_item(&mut self, term: &[u8]) {
        let level = self.level;
        let (sf, af) = {
            let c = self.n(self.curr);
            (c.sf.clone(), c.af.clone())
        };
        let text = if af[level].is_empty() {
            &sf[level]
        } else {
            &af[level]
        };
        if level > self.prev_level {
            // ascending level
            self.line = cat(&[term, &self.item_u[level], text]);
            self.ind_lc += self.ilen_u[level];
        } else {
            // same or descending level
            self.line = cat(&[term, &self.item_r[level], text]);
            self.ind_lc += self.ilen_r[level];
        }

        let mut i = level + 1;
        while i < FIELD_MAX && !sf[i].is_empty() {
            let l = self.line.clone();
            self.ind(&l);
            let text = if af[i].is_empty() { &sf[i] } else { &af[i] };
            self.line = cat(&[&self.item_x[i], text]);
            self.ind_lc += self.ilen_x[i];
            self.level = i; // Added at 2.11 <brosig@gmdzi.gmd.de>
            i += 1;
        }

        self.ind_indent = 0;
        let d = self.delim_p[self.level].clone();
        self.line.extend_from_slice(cstr(&d));
        self.save();
    }

    /// `first_letter`, in the environment's `LC_CTYPE`.
    fn first_letter(&self, ct: &Env, term: &[u8]) -> u8 {
        if self.thai_sort {
            let t0 = at(term, 0);
            return if t0 == 0 || b"\xe0\xe1\xe2\xe3\xe4".contains(&t0) {
                at(term, 1)
            } else {
                t0
            };
        }
        let c = at(term, 0) as i32;
        (if ct.isupper(c) { ct.tolower(c) } else { c }) as u8
    }

    fn new_entry(&mut self) {
        let mut let_: i32 = -1; // see comment below
                                // setlocale(LC_CTYPE, "") until the end of this function
        let ct = self.ctype.take().unwrap_or_else(Env::ctype);

        if self.in_range {
            let ptr = self.curr;
            self.curr = self.range_ptr;
            let m = fmt(
                "Unmatched range opening operator %c.\n",
                &[P::C(self.idx_ropen)],
            );
            self.ind_error(&m);
            self.in_range = false;
            self.curr = ptr;
        }
        self.flush_line(true);

        // beginning of a new group?
        let (cg, pg) = (self.n(self.curr).group, self.n(self.prev).group);
        let new_group = (cg != ALPHA && cg != pg && pg == SYMBOL)
            || (cg == ALPHA && {
                let c = self.first_letter(&ct, &self.n(self.curr).sf[0]);
                let_ = c as i32;
                c != self.first_letter(&ct, &self.n(self.prev).sf[0])
            })
            || (self.german_sort && cg != ALPHA && pg == ALPHA);
        if new_group {
            let t = self.delim_t.clone();
            self.ind(&t);
            let g = self.group_skip.clone();
            self.ind(&g);
            self.ind_lc += self.skiplen;
            // beginning of a new letter?
            self.put_header(let_, Some(&ct));
            self.make_item(b"");
        } else {
            let t = self.delim_t.clone();
            self.make_item(&t);
        }
        self.ctype = Some(ct);
    }

    fn old_entry(&mut self) {
        // current entry identical to previous one: append pages
        let diff = self.page_diff(self.the_end, self.curr);
        let (pt, ct) = (self.n(self.prev).type_, self.n(self.curr).type_);
        let same_encap = self.prev_encap.as_deref() == Some(&self.encap[..]);

        if pt == ct
            && diff != -1
            && ((diff == 0 && same_encap)
                || (self.merge_page && diff == 1 && same_encap)
                || self.in_range)
        {
            self.the_end = self.curr;
            // extract in-range encaps out
            let cenc = self.n(self.curr).encap.clone();
            if self.in_range
                && at(&cenc, 0) != 0
                && at(&cenc, 0) != self.idx_rclose
                && self.prev_encap.as_deref() != Some(&cenc[..])
            {
                let lpg = self.n(self.curr).lpg.clone();
                self.buff = cat(&[&self.encap_p, &cenc, &self.encap_i, &lpg, &self.encap_s]);
                self.wrap_line(false);
            }
            if self.in_range {
                self.encap_range = true;
            }
        } else {
            self.flush_line(false);
            if diff == 0 && pt == ct {
                self.ind_error(
                    b"Conflicting entries: multiple encaps for the same page under same key.\n",
                );
            } else if self.in_range && pt != ct {
                self.ind_error(
                    b"Illegal range formation: starting & ending pages are of different types.\n",
                );
            } else if self.in_range && diff == -1 {
                self.ind_error(
                    b"Illegal range formation: starting & ending pages cross chap/sec breaks.\n",
                );
            }
            self.save();
        }
    }

    fn page_diff(&self, a: Option<usize>, b: Option<usize>) -> i32 {
        let (a, b) = (self.n(a), self.n(b));
        if a.count != b.count {
            return -1;
        }
        let mut i: i16 = 0;
        while i < a.count - 1 {
            if a.npg[i as usize] != b.npg[i as usize] {
                return -1;
            }
            i += 1;
        }
        let la = (a.count - 1).max(0) as usize;
        let lb = (b.count - 1).max(0) as usize;
        b.npg[lb].wrapping_sub(a.npg[la])
    }

    /// `put_header(let)`: in the C locale from `make_entry`, in the
    /// environment's from `new_entry`.
    fn put_header(&mut self, let_: i32, ct: Option<&Env>) {
        if self.headings_flag != 0 {
            let p = self.heading_pre.clone();
            self.ind(&p);
            self.ind_lc += self.headprelen;
            let group = self.n(self.curr).group;
            match group {
                SYMBOL => {
                    let s = if self.headings_flag > 0 {
                        self.symhead_pos.clone()
                    } else {
                        self.symhead_neg.clone()
                    };
                    self.ind(&s);
                }
                ALPHA => {
                    let c = let_ as u8 as i32;
                    let out = match ct {
                        Some(e) => {
                            if self.headings_flag > 0 {
                                if e.isupper(c) {
                                    c
                                } else {
                                    e.toupper(c)
                                }
                            } else if e.isupper(c) {
                                e.tolower(c)
                            } else {
                                c
                            }
                        }
                        None => {
                            if self.headings_flag > 0 {
                                crate::locale::c_toupper(c)
                            } else {
                                tolower_c(c as u8) as i32
                            }
                        }
                    };
                    self.ind(&[out as u8]);
                }
                _ => {
                    let s = if self.headings_flag > 0 {
                        self.numhead_pos.clone()
                    } else {
                        self.numhead_neg.clone()
                    };
                    self.ind(&s);
                }
            }
            let s = self.heading_suf.clone();
            self.ind(&s);
            self.ind_lc += self.headsuflen;
        }
    }

    fn flush_line(&mut self, print: bool) {
        let begin_lpg = self.n(self.begin).lpg.clone();
        let end_lpg = self.n(self.the_end).lpg.clone();
        if self.page_diff(self.begin, self.the_end) != 0 {
            let thr = if self.suffix_2p.is_empty() { 1 } else { 0 };
            if self.encap_range || self.page_diff(self.begin, self.prev) > thr {
                let diff = self.page_diff(self.begin, self.the_end);

                self.buff = if diff == 1 && !self.suffix_2p.is_empty() {
                    cat(&[&begin_lpg, &self.suffix_2p])
                } else if diff == 2 && !self.suffix_3p.is_empty() {
                    cat(&[&begin_lpg, &self.suffix_3p])
                } else if diff >= 2 && !self.suffix_mp.is_empty() {
                    cat(&[&begin_lpg, &self.suffix_mp])
                } else {
                    cat(&[&begin_lpg, &self.delim_r, &end_lpg])
                };
                self.encap_range = false;
            } else {
                self.buff = cat(&[&begin_lpg, &self.delim_n, &end_lpg]);
            }
        } else {
            self.encap_range = false; // might be true from page range on same page
            self.buff = cstr(&begin_lpg).to_vec();
        }

        let pe = self.prev_encap.clone().unwrap_or_default();
        if !pe.is_empty() {
            let tmp = self.buff.clone();
            self.buff = cat(&[&self.encap_p, &pe, &self.encap_i, &tmp, &self.encap_s]);
        }
        self.wrap_line(print);
    }

    fn wrap_line(&mut self, print: bool) {
        let len = self.line.len() as i32 + self.buff.len() as i32 + self.ind_indent;
        if print {
            if len > self.linemax {
                let l = self.line.clone();
                self.putln(&l);
                let s = self.indent_space.clone();
                self.ind(&s);
                self.ind_indent = self.indent_length;
            } else {
                let l = self.line.clone();
                self.ind(&l);
            }
            let b = self.buff.clone();
            self.ind(&b);
        } else if len > self.linemax {
            let l = self.line.clone();
            self.putln(&l);
            self.line = cat(&[&self.indent_space, &self.buff, &self.delim_n]);
            self.ind_indent = self.indent_length;
        } else {
            let d = self.delim_n.clone();
            self.buff.extend_from_slice(&d);
            let b = self.buff.clone();
            self.line.extend_from_slice(&b);
        }
    }

    fn insert_page(&mut self) {
        if self.even_odd >= 0 {
            // pageno as the C array: its string, a NUL, room to grow
            let mut p = cstr(&self.pageno).to_vec();
            p.resize(p.len() + 4, 0);
            let get = |p: &[u8], k: isize| if k < 0 { 0 } else { p[k as usize] };
            // find the rightmost digit
            let mut i: isize = 0;
            loop {
                let c = get(&p, i);
                i += 1;
                if c == 0 {
                    break;
                }
            }
            i -= 1;
            let mut j = i;
            // find the leftmost digit
            loop {
                i -= 1;
                if !(c_isdigit(get(&p, i) as i32) && i > 0) {
                    break;
                }
            }
            if !c_isdigit(get(&p, i) as i32) {
                i += 1;
            }
            // convert page from literal to numeric
            let start = i.max(0) as usize;
            let mut page = strtoint(&p[start..]).wrapping_add(1);
            // check even-odd numbering
            if (self.even_odd == 1 && page % 2 == 0) || (self.even_odd == 2 && page % 2 == 1) {
                page += 1;
            }
            if (j + 1) as usize >= p.len() {
                p.resize(j as usize + 2, 0);
            }
            p[(j + 1) as usize] = 0;
            // convert page back to literal
            while page >= 10 {
                if j >= 0 {
                    p[j as usize] = (page % 10 + 48) as u8;
                }
                j -= 1;
                page /= 10;
            }
            if j >= 0 {
                p[j as usize] = (page + 48) as u8;
            }
            if i < j {
                let (mut i, mut j) = (i as usize, j as usize);
                while p[j] != 0 {
                    p[i] = p[j];
                    i += 1;
                    j += 1;
                }
                p[i] = 0;
            }
            self.pageno = cstr(&p).to_vec();
        }
        let o = self.setpage_open.clone();
        self.ind(&o);
        let p = self.pageno.clone();
        self.ind(&p);
        let c = self.setpage_close.clone();
        self.ind(&c);
        self.ind_lc += self.setpagelen;
    }
}
