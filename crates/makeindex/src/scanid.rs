//! scanid.c and scanid.h: reading the `.idx` file.

use crate::io::{mk_getc, EOF};
use crate::locale::c_isdigit;
use crate::mkind::*;
use std::rc::Rc;

fn is_roman_lower(c: u8) -> bool {
    matches!(c, b'i' | b'v' | b'x' | b'l' | b'c' | b'd' | b'm')
}
fn is_roman_upper(c: u8) -> bool {
    matches!(c, b'I' | b'V' | b'X' | b'L' | b'C' | b'D' | b'M')
}
fn roman_lower_val(c: u8) -> i32 {
    match c {
        b'i' => 1,
        b'v' => 5,
        b'x' => 10,
        b'l' => 50,
        b'c' => 100,
        b'd' => 500,
        b'm' => 1000,
        _ => 0,
    }
}
fn roman_upper_val(c: u8) -> i32 {
    roman_lower_val(c.wrapping_add(32))
}
fn is_alpha_lower(c: u8) -> bool {
    c.is_ascii_lowercase()
}
fn is_alpha_upper(c: u8) -> bool {
    c.is_ascii_uppercase()
}
fn alpha_val(c: u8) -> i32 {
    if c.is_ascii_uppercase() {
        (c - b'A') as i32
    } else if c.is_ascii_lowercase() {
        (c - b'a') as i32
    } else {
        0
    }
}

/// `strchr(s, c) != NULL` for a C string and a non-NUL `c`.
fn has(s: &[u8], c: u8) -> bool {
    s.contains(&c)
}

/// `strspn(s, set)`.
fn strspn(s: &[u8], set: &[u8]) -> usize {
    s.iter().take_while(|b| set.contains(b)).count()
}

/// mkind.h's `ISDIGIT` and `ISSYMBOL` on a (signed) `char`.
fn isdigit_ch(c: u8) -> bool {
    c.is_ascii_digit()
}
fn issymbol_ch(c: u8) -> bool {
    (b'!'..=b'@').contains(&c) || (b'['..=b'`').contains(&c) || (b'{'..=b'~').contains(&c)
}

/// `sscanf(str, "%d", &i)` on a string of digits: `strtoimax`, saturating,
/// stored truncated to `int`; nothing stored for an empty string.
fn sscanf_d(s: &[u8]) -> Option<i32> {
    if s.is_empty() {
        return None;
    }
    let mut v: i128 = 0;
    for &d in s {
        v = (v * 10 + (d - b'0') as i128).min(i64::MAX as i128);
    }
    Some(v as i64 as i32)
}

/// `group_type` (scanid.c).
pub fn group_type(s: &[u8]) -> i32 {
    let mut i = 0;
    while at(s, i) != 0 && isdigit_ch(at(s, i)) {
        i += 1;
    }
    if at(s, i) == 0 {
        sscanf_d(cstr(s)).unwrap_or(i as i32)
    } else if issymbol_ch(at(s, 0)) {
        SYMBOL
    } else {
        ALPHA
    }
}

impl Mk<'_> {
    fn idx_getc(&mut self) -> i32 {
        mk_getc(&mut self.lookahead, self.idx_fp.as_mut().unwrap())
    }

    /// `IDX_ERROR` and friends.
    fn idx_error(&mut self, msg: &[u8]) {
        if self.idx_dot {
            self.ilg(b"\n");
            self.idx_dot = false;
        }
        let h = fmt(
            "!! Input index error (file = %s, line = %d):\n   -- ",
            &[P::S(&self.idx_fn.clone()), self.idx_lc.into()],
        );
        self.ilg(&h);
        self.ilg(msg);
        self.idx_ec += 1;
    }

    /// `IDX_SKIPLINE` (without its `arg_count = -1`).
    fn idx_skipline(&mut self) {
        loop {
            let tmp = self.idx_getc();
            if tmp == LFD || tmp == EOF {
                break;
            }
        }
        self.idx_lc += 1;
    }

    pub fn scan_idx(&mut self) {
        let mut keyword = vec![0u8; ARRAY_MAX + 1];
        let mut i = 0usize;
        let mut not_eof = true;
        let mut arg_count: i32 = -1;

        let m = fmt("Scanning input file %s...", &[P::S(&self.idx_fn.clone())]);
        self.message(&m);
        self.idx_lc = 0;
        self.idx_tc = 0;
        self.idx_ec = 0;
        self.idx_dc = 0;
        self.comp_len = self.page_comp.len();
        while not_eof {
            let c = self.idx_getc();
            if c == EOF {
                if arg_count == 2 {
                    self.idx_lc += 1;
                    if self.make_key() {
                        self.idx_dot_(DOT_MAX);
                    }
                    arg_count = -1;
                } else {
                    if arg_count > -1 {
                        self.idx_lc += 1;
                        self.idx_error(b"Missing arguments -- need two (premature EOF).\n");
                        arg_count = -2;
                    }
                    not_eof = false;
                }
            } else if c == LFD {
                self.idx_lc += 1;
                if arg_count == 2 {
                    if self.make_key() {
                        self.idx_dot_(DOT_MAX);
                    }
                    arg_count = -1;
                } else if arg_count > -1 {
                    self.idx_error(b"Missing arguments -- need two (premature LFD).\n");
                    arg_count = -1;
                }
            } else if c == TAB || c == SPC {
            } else {
                match arg_count {
                    -1 => {
                        i = 0;
                        keyword[i] = c as u8;
                        i += 1;
                        arg_count += 1;
                        self.idx_tc += 1;
                    }
                    0 => {
                        if c == sc(self.idx_aopen) {
                            arg_count += 1;
                            keyword[i] = 0;
                            if cstr(&keyword) == cstr(&self.idx_keyword) {
                                if !self.scan_arg1() {
                                    arg_count = -1;
                                }
                            } else {
                                self.idx_skipline();
                                arg_count = -1;
                                let m = fmt("Unknown index keyword %s.\n", &[P::S(cstr(&keyword))]);
                                self.idx_error(&m);
                            }
                        } else if i < ARRAY_MAX {
                            keyword[i] = c as u8;
                            i += 1;
                        } else {
                            self.idx_skipline();
                            arg_count = -1;
                            let m = fmt(
                                "Index keyword %s too long (max %d).\n",
                                &[P::S(cstr(&keyword[..ARRAY_MAX])), ARRAY_MAX.into()],
                            );
                            self.idx_error(&m);
                        }
                    }
                    1 => {
                        if c == sc(self.idx_aopen) {
                            arg_count += 1;
                            if !self.scan_arg2() {
                                arg_count = -1;
                            }
                        } else {
                            self.idx_skipline();
                            arg_count = -1;
                            let m = fmt(
                                "No opening delimiter for second argument (illegal character `%c').\n",
                                &[P::C(c as u8)],
                            );
                            self.idx_error(&m);
                        }
                    }
                    2 => {
                        self.idx_skipline();
                        arg_count = -1;
                        let m = fmt(
                            "No closing delimiter for second argument (illegal character `%c').\n",
                            &[P::C(c as u8)],
                        );
                        self.idx_error(&m);
                    }
                    _ => {}
                }
            }
        }

        // fixup the total counts
        self.idx_tt += self.idx_tc;
        self.idx_et += self.idx_ec;

        self.done(
            self.idx_tc - self.idx_ec,
            "entries accepted",
            self.idx_ec,
            "rejected",
        );
        self.idx_fp = None;
    }

    fn flush_to_eol(&mut self) {
        loop {
            let a = self.idx_getc();
            if a == LFD || a == EOF {
                break;
            }
        }
    }

    fn make_key(&mut self) -> bool {
        let mut data = Field {
            sf: [vec![], vec![], vec![]],
            af: [vec![], vec![], vec![]],
            group: 0,
            lpg: vec![],
            npg: [0; PAGEFIELD_MAX],
            count: 0,
            type_: EMPTY,
            encap: vec![],
            fn_: Rc::new(vec![]),
            lc: 0,
        };

        // process index key
        let key = std::mem::take(&mut self.key);
        let ok = self.scan_key(&key, &mut data);
        self.key = key;
        if !ok {
            return false;
        }

        // determine group type
        data.group = group_type(&data.sf[0]);

        // process page number
        let no = cstr(&self.no).to_vec();
        data.lpg = no.clone();
        let mut ty = EMPTY;
        if !self.scan_no(&no, &mut data.npg, &mut data.count, &mut ty) {
            return false;
        }
        data.type_ = ty;

        data.lc = self.idx_lc;
        data.fn_ = self.idx_fn.clone();
        self.nodes.push(data);
        true
    }

    fn scan_key(&mut self, key: &[u8], data: &mut Field) -> bool {
        let mut i = 0usize; // current level
        let mut n = 0usize; // index to the key[] array
        let mut second_round = false;
        let last = FIELD_MAX - 1;

        loop {
            if at(key, n) == 0 {
                break;
            }
            let len = cstr(key).len();
            if at(key, n) == self.idx_encap {
                n += 1;
                match self.scan_field(key, &mut n, len, false, false, false) {
                    Some(f) => {
                        data.encap = f;
                        break;
                    }
                    None => return false,
                }
            }
            if at(key, n) == self.idx_actual {
                n += 1;
                match self.scan_field(key, &mut n, len, i != last, true, false) {
                    Some(f) => data.af[i] = f,
                    None => return false,
                }
            } else {
                // Next nesting level
                if second_round {
                    i += 1;
                    n += 1;
                }
                match self.scan_field(key, &mut n, len, i != last, true, true) {
                    Some(f) => data.sf[i] = f,
                    None => return false,
                }
                second_round = true;
                if self.german_sort && data.sf[i].contains(&b'"') {
                    search_quote(&mut data.sf[i], &mut data.af[i]);
                }
            }
        }

        // check for empty fields which shouldn't be empty
        if data.sf[0].is_empty() {
            self.idx_error(b"Illegal null field.\n");
            return false;
        }
        for i in 1..FIELD_MAX - 1 {
            if data.sf[i].is_empty() && (!data.af[i].is_empty() || !data.sf[i + 1].is_empty()) {
                self.idx_error(b"Illegal null field.\n");
                return false;
            }
        }
        // i == FIELD_MAX-1
        let i = FIELD_MAX - 1;
        if data.sf[i].is_empty() && !data.af[i].is_empty() {
            self.idx_error(b"Illegal null field.\n");
            return false;
        }
        true
    }

    /// `scan_field`: the field read (a C string), or `None` after an
    /// error.
    fn scan_field(
        &mut self,
        key: &[u8],
        n: &mut usize,
        len_field: usize,
        ck_level: bool,
        ck_encap: bool,
        ck_actual: bool,
    ) -> Option<Vec<u8>> {
        let mut field = vec![0u8; len_field + 2];
        let mut i = 0usize;
        let k = |n: usize| at(key, n);

        if self.compress_blanks && (k(*n) as i32 == SPC || k(*n) as i32 == TAB) {
            *n += 1;
        }

        loop {
            let mut nbsh = 0;
            let mut overflow = false;
            while k(*n) == self.idx_escape {
                nbsh += 1;
                field[i] = k(*n);
                i += 1;
                if i > len_field {
                    overflow = true;
                    break;
                }
                *n += 1;
            }

            if !overflow {
                if k(*n) == self.idx_quote {
                    if nbsh % 2 == 0 {
                        *n += 1;
                        field[i] = k(*n);
                    } else {
                        field[i] = k(*n);
                    }
                    i += 1;
                    if i > len_field {
                        overflow = true;
                    }
                } else if (ck_level && k(*n) == self.idx_level)
                    || (ck_encap && k(*n) == self.idx_encap)
                    || (ck_actual && k(*n) == self.idx_actual)
                    || k(*n) == 0
                {
                    if i > 0 && self.compress_blanks && field[i - 1] as i32 == SPC {
                        field[i - 1] = 0;
                    } else {
                        field[i] = 0;
                    }
                    return Some(cstr(&field).to_vec());
                } else {
                    field[i] = k(*n);
                    i += 1;
                    if i > len_field {
                        overflow = true;
                    } else if !ck_level && k(*n) == self.idx_level {
                        let m = fmt(
                            "Extra `%c' at position %d of first argument.\n",
                            &[P::C(self.idx_level), (*n + 1).into()],
                        );
                        self.idx_error(&m);
                        return None;
                    } else if !ck_encap && k(*n) == self.idx_encap {
                        let m = fmt(
                            "Extra `%c' at position %d of first argument.\n",
                            &[P::C(self.idx_encap), (*n + 1).into()],
                        );
                        self.idx_error(&m);
                        return None;
                    } else if !ck_actual && k(*n) == self.idx_actual {
                        let m = fmt(
                            "Extra `%c' at position %d of first argument.\n",
                            &[P::C(self.idx_actual), (*n + 1).into()],
                        );
                        self.idx_error(&m);
                        return None;
                    }
                }
            }
            // check if max field length is reached
            if overflow || i > len_field {
                let m = if !ck_encap {
                    fmt(
                        "Encapsulator of page number too long (max. %d).\n",
                        &[len_field.into()],
                    )
                } else if ck_actual {
                    fmt("Index sort key too long (max. %d).\n", &[len_field.into()])
                } else {
                    fmt(
                        "Text of key entry too long (max. %d).\n",
                        &[len_field.into()],
                    )
                };
                self.idx_error(&m);
                return None;
            }
            *n += 1;
        }
    }

    /// `IS_COMPOSITOR` at `no[i]`.
    fn is_compositor(&self, no: &[u8], i: usize) -> bool {
        for k in 0..self.comp_len {
            let a = at(no, i + k);
            let b = at(&self.page_comp, k);
            if a != b {
                return false;
            }
            if a == 0 {
                return true;
            }
        }
        true
    }

    /// `ENTER(V)`: false after the error.
    fn enter(
        &mut self,
        no: &[u8],
        npg: &mut [i32; PAGEFIELD_MAX],
        count: &mut i16,
        v: i32,
    ) -> bool {
        if *count as usize >= PAGEFIELD_MAX {
            let m = fmt(
                "Page number %s has too many fields (max. %d).",
                &[P::S(no), PAGEFIELD_MAX.into()],
            );
            self.idx_error(&m);
            return false;
        }
        npg[*count as usize] = v;
        *count += 1;
        true
    }

    fn scan_no(
        &mut self,
        no: &[u8],
        npg: &mut [i32; PAGEFIELD_MAX],
        count: &mut i16,
        type_: &mut i16,
    ) -> bool {
        let pp = cstr(&self.page_prec).to_vec();
        let c0 = at(no, 0);
        let cnt = (*count as usize).min(PAGEFIELD_MAX);
        let (roml, romu, arab, alpl, alpu, empty) = (
            ROML as i32,
            ROMU as i32,
            ARAB as i32,
            ALPL as i32,
            ALPU as i32,
            EMPTY as i32,
        );
        // heuristic detection if a letter is Roman or Alpha
        if c0.is_ascii_digit() {
            self.tg_set(cnt, arab);
        } else if is_roman_lower(c0) && is_alpha_lower(c0) && has(&pp, b'r') && has(&pp, b'a') {
            let g = self.tg_get(cnt);
            if strspn(no, b"ivxlcdm") == 1 && g != roml && g != alpl {
                self.tg_set(cnt, if strspn(no, b"ivx") == 1 { roml } else { alpl });
            }
            if strspn(no, b"ivxlcdm") > 1 {
                self.tg_set(cnt, roml);
            }
        } else if is_roman_upper(c0) && is_alpha_upper(c0) && has(&pp, b'R') && has(&pp, b'A') {
            let g = self.tg_get(cnt);
            if strspn(no, b"IVXLCDM") == 1 && g != romu && g != alpu {
                self.tg_set(cnt, if strspn(no, b"IVX") == 1 { romu } else { alpu });
            }
            if strspn(no, b"IVXLCDM") > 1 {
                self.tg_set(cnt, romu);
            }
        } else if is_roman_lower(c0) && has(&pp, b'r') {
            self.tg_set(cnt, roml);
        } else if is_roman_upper(c0) && has(&pp, b'R') {
            self.tg_set(cnt, romu);
        } else if is_alpha_lower(c0) && has(&pp, b'a') {
            self.tg_set(cnt, alpl);
        } else if is_alpha_upper(c0) && has(&pp, b'A') {
            self.tg_set(cnt, alpu);
        } else {
            self.tg_set(cnt, empty);
        }
        let tg = self.tg_get(cnt);

        if c0.is_ascii_digit() {
            *type_ = ARAB;
            self.scan_arabic(no, npg, count)
        } else if is_roman_lower(c0) && has(&pp, b'r') && (!has(&pp, b'a') || tg == roml) {
            *type_ = ROML;
            self.scan_roman(no, npg, count, false)
        } else if is_roman_upper(c0) && has(&pp, b'R') && (!has(&pp, b'A') || tg == romu) {
            *type_ = ROMU;
            self.scan_roman(no, npg, count, true)
        } else if is_alpha_lower(c0) && has(&pp, b'a') {
            *type_ = ALPL;
            self.scan_alpha(no, npg, count, ALPL)
        } else if is_alpha_upper(c0) && has(&pp, b'A') {
            *type_ = ALPU;
            self.scan_alpha(no, npg, count, ALPU)
        } else {
            let m = fmt(
                "Illegal page number %s or page_precedence %s.\n",
                &[P::S(no), P::S(&pp)],
            );
            self.idx_error(&m);
            false
        }
    }

    fn scan_arabic(&mut self, no: &[u8], npg: &mut [i32; PAGEFIELD_MAX], count: &mut i16) -> bool {
        let mut i = 0usize;
        let mut s: Vec<u8> = vec![];
        while at(no, i) != 0 && i <= ARABIC_MAX && !self.is_compositor(no, i) {
            if c_isdigit(at(no, i) as i32) {
                s.push(at(no, i));
                i += 1;
            } else {
                let m = fmt(
                    "Illegal Arabic digit: position %d in %s.\n",
                    &[(i + 1).into(), P::S(no)],
                );
                self.idx_error(&m);
                return false;
            }
        }
        if i > ARABIC_MAX {
            let m = fmt(
                "Arabic page number %s too big (max %d digits).\n",
                &[P::S(no), ARABIC_MAX.into()],
            );
            self.idx_error(&m);
            return false;
        }

        let v = strtoint(&s).wrapping_add(self.page_offset[ARAB as usize]);
        if !self.enter(no, npg, count, v) {
            return false;
        }

        if self.is_compositor(no, i) {
            let mut t = 0;
            let rest = no.get(i + self.comp_len..).unwrap_or(&[]).to_vec();
            self.scan_no(&rest, npg, count, &mut t)
        } else {
            true
        }
    }

    fn scan_roman(
        &mut self,
        no: &[u8],
        npg: &mut [i32; PAGEFIELD_MAX],
        count: &mut i16,
        upper: bool,
    ) -> bool {
        let mut i = 0usize;
        let mut inp: i32 = 0;
        let mut prev: i32 = 0;
        while at(no, i) != 0 && i < ROMAN_MAX && !self.is_compositor(no, i) {
            let c = at(no, i);
            let (is, val) = if upper {
                (is_roman_upper(c), roman_upper_val(c))
            } else {
                (is_roman_lower(c), roman_lower_val(c))
            };
            if is && val != 0 {
                let mut the_new = val;
                if prev == 0 {
                    prev = the_new;
                } else {
                    if prev < the_new {
                        prev = the_new - prev;
                        the_new = 0;
                    }
                    inp += prev;
                    prev = the_new;
                }
            } else {
                let m = fmt(
                    "Illegal Roman number: position %d in %s.\n",
                    &[(i + 1).into(), P::S(no)],
                );
                self.idx_error(&m);
                return false;
            }
            i += 1;
        }
        if i == ROMAN_MAX {
            let m = fmt(
                "Roman page number %s too big (max %d digits).\n",
                &[P::S(no), ROMAN_MAX.into()],
            );
            self.idx_error(&m);
            return false;
        }
        inp += prev;

        let off = self.page_offset[if upper { ROMU } else { ROML } as usize];
        if !self.enter(no, npg, count, inp.wrapping_add(off)) {
            return false;
        }

        if self.is_compositor(no, i) {
            let mut t = 0;
            let rest = no.get(i + self.comp_len..).unwrap_or(&[]).to_vec();
            self.scan_no(&rest, npg, count, &mut t)
        } else {
            true
        }
    }

    fn scan_alpha(
        &mut self,
        no: &[u8],
        npg: &mut [i32; PAGEFIELD_MAX],
        count: &mut i16,
        ty: i16,
    ) -> bool {
        let v = alpha_val(at(no, 0)).wrapping_add(self.page_offset[ty as usize]);
        if !self.enter(no, npg, count, v) {
            return false;
        }
        if self.is_compositor(no, 1) {
            let mut t = 0;
            let rest = no.get(self.comp_len + 1..).unwrap_or(&[]).to_vec();
            self.scan_no(&rest, npg, count, &mut t)
        } else {
            true
        }
    }

    fn scan_arg1(&mut self) -> bool {
        let mut i = 0usize;
        let mut n = 0i32; // delimiter count
        let mut a;

        if self.compress_blanks {
            loop {
                a = self.idx_getc();
                if a != SPC && a != TAB {
                    break;
                }
            }
        } else {
            a = self.idx_getc();
        }

        while i < ARGUMENT_MAX && a != EOF {
            if a == sc(self.idx_quote) || a == sc(self.idx_escape) {
                // take next character literally, but preserve quote or escape
                self.key[i] = a as u8;
                i += 1;
                a = self.idx_getc();
                self.key[i] = a as u8; // save literal character
                i += 1;
            } else if a == sc(self.idx_aopen) {
                // opening delimiters within the argument list
                self.key[i] = a as u8;
                i += 1;
                n += 1;
            } else if a == sc(self.idx_aclose) {
                if n == 0 {
                    // end of argument
                    if self.compress_blanks && i > 0 && self.key[i - 1] as i32 == SPC {
                        self.key[i - 1] = 0;
                    } else {
                        self.key[i] = 0;
                    }
                    return true;
                } else {
                    // nested delimiters
                    self.key[i] = a as u8;
                    i += 1;
                    n -= 1;
                }
            } else if a == LFD {
                self.idx_lc += 1;
                self.idx_error(b"Incomplete first argument (premature LFD).\n");
                return false;
            } else if (a == TAB || a == SPC) && self.compress_blanks {
                // compress successive SPC's to one SPC
                if i > 0 && self.key[i - 1] as i32 != SPC && self.key[i - 1] as i32 != TAB {
                    self.key[i] = SPC as u8;
                    i += 1;
                }
            } else {
                self.key[i] = a as u8;
                i += 1;
            }
            a = self.idx_getc();
        }

        self.flush_to_eol(); // Skip to end of line
        self.idx_lc += 1;
        let m = fmt(
            "First argument too long (max %d).\n",
            &[ARGUMENT_MAX.into()],
        );
        self.idx_error(&m);
        false
    }

    fn scan_arg2(&mut self) -> bool {
        let mut i = 0usize;
        let mut hit_blank = false;
        let mut a;
        loop {
            a = self.idx_getc();
            if a != SPC && a != TAB {
                break;
            }
        }

        while i < NUMBER_MAX {
            if a == sc(self.idx_aclose) {
                self.no[i] = 0;
                return true;
            } else if a == LFD {
                self.idx_lc += 1;
                self.idx_error(b"Incomplete second argument (premature LFD).\n");
                return false;
            } else if a == TAB || a == SPC {
                hit_blank = true;
            } else {
                if hit_blank {
                    self.flush_to_eol(); // Skip to end of line
                    self.idx_lc += 1;
                    self.idx_error(b"Illegal space within numerals in second argument.\n");
                    return false;
                }
                self.no[i] = a as u8;
                i += 1;
            }
            a = self.idx_getc();
        }
        self.flush_to_eol(); // Skip to end of line
        self.idx_lc += 1;
        let m = fmt("Second argument too long (max %d).\n", &[NUMBER_MAX.into()]);
        self.idx_error(&m);
        false
    }
}

/// `search_quote` (german sorting): `"a` sorts as `ae` and so on.
fn search_quote(sort_key: &mut [u8], actual_key: &mut Vec<u8>) {
    *actual_key = sort_key.to_vec();
    let mut char_found = false;
    let mut ptr = sort_key.iter().position(|&b| b == b'"');
    while let Some(p) = ptr {
        let next = at(sort_key, p + 1);
        let sort: Option<&[u8; 2]> = match next {
            b'a' | b'A' => Some(if next.is_ascii_uppercase() {
                b"Ae"
            } else {
                b"ae"
            }),
            b'o' | b'O' => Some(if next.is_ascii_uppercase() {
                b"Oe"
            } else {
                b"oe"
            }),
            b'u' | b'U' => Some(if next.is_ascii_uppercase() {
                b"Ue"
            } else {
                b"ue"
            }),
            b's' => Some(b"ss"),
            _ => None,
        };
        if let Some(s) = sort {
            char_found = true;
            sort_key[p] = s[0];
            sort_key[p + 1] = s[1];
        }
        ptr = sort_key
            .iter()
            .skip(p + 1)
            .position(|&b| b == b'"')
            .map(|q| q + p + 1);
    }
    if !char_found {
        actual_key.clear();
    }
}
