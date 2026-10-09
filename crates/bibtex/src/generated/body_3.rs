// GENERATED FILE -- DO NOT EDIT.
// Translated WEB procedures and functions.
// Regenerate with the command in tools/web2rust/README.md.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(unused_parens, unused_mut, unused_variables, unused_assignments)]
#![allow(unused_imports, unused_labels, while_true)]
#![allow(dead_code, unreachable_code, unused_comparisons, clippy::all)]

use super::consts::*;
use super::globals::Globals;
use super::types::*;

impl Globals {
    /// Here we print `trace` and/or `stat` information, if desired.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §447
    pub fn trace_and_stat_printing(&mut self) {
        // §456
        {
            {
                {
                    let __w1 = self.num_cites;
                    crate::system::wr_str(&mut self.log_file, "You've used ");
                    crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                }
            }
            if (self.num_cites == 1i32) {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, " entry,");
                        crate::system::wr_ln(&mut self.log_file);
                    }
                }
            } else {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, " entries,");
                        crate::system::wr_ln(&mut self.log_file);
                    }
                }
            }
            {
                {
                    let __w1 = self.wiz_def_ptr;
                    crate::system::wr_str(&mut self.log_file, "            ");
                    crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                    crate::system::wr_str(&mut self.log_file, " wiz_defined-function locations,");
                    crate::system::wr_ln(&mut self.log_file);
                }
            }
            {
                {
                    let __w1 = self.str_ptr;
                    let __w3 = self.str_start[(self.str_ptr) as usize];
                    crate::system::wr_str(&mut self.log_file, "            ");
                    crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                    crate::system::wr_str(&mut self.log_file, " strings with ");
                    crate::system::wr_int(&mut self.log_file, __w3, 0i32);
                    crate::system::wr_str(&mut self.log_file, " characters,");
                    crate::system::wr_ln(&mut self.log_file);
                }
            }
            self.blt_in_ptr = 0i32;
            self.total_ex_count = 0i32;
            while (self.blt_in_ptr < num_blt_in_fns) {
                {
                    self.total_ex_count = (self.total_ex_count).wrapping_add(self.execution_count[(self.blt_in_ptr) as usize]);
                    self.blt_in_ptr = (self.blt_in_ptr).wrapping_add(1i32);
                }
            }
            {
                {
                    let __w1 = self.total_ex_count;
                    crate::system::wr_str(&mut self.log_file, "and the built_in function-call counts, ");
                    crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                    crate::system::wr_str(&mut self.log_file, " in all, are:");
                    crate::system::wr_ln(&mut self.log_file);
                }
            }
            self.blt_in_ptr = 0i32;
            while (self.blt_in_ptr < num_blt_in_fns) {
                {
                    {
                        { let mut __f0 = ::core::mem::take(&mut self.log_file); let __r = self.out_pool_str(&mut __f0, self.hash_text[(self.blt_in_loc[(self.blt_in_ptr) as usize]) as usize]); self.log_file = __f0; __r };
                    }
                    {
                        {
                            let __w1 = self.execution_count[(self.blt_in_ptr) as usize];
                            crate::system::wr_str(&mut self.log_file, " -- ");
                            crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                            crate::system::wr_ln(&mut self.log_file);
                        }
                    }
                    self.blt_in_ptr = (self.blt_in_ptr).wrapping_add(1i32);
                }
            }
        }
    }

    /// This procedure copies file name `file_name` into the beginning of
    /// `name_of_file`, if it will fit.  It also sets the global variable
    /// `name_length` to the appropriate value.
    /// @<Procedures and functions for file-system interacting
    // §51
    pub fn start_name(&mut self, mut file_name: str_number) {
        let mut p_ptr: pool_pointer = 0; // §51
        self.name_of_file.alloc_len(((((self.str_start[((file_name).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(file_name) as usize])).wrapping_add(1i32)) as usize) + 1);
        self.name_ptr = 1i32;
        p_ptr = self.str_start[(file_name) as usize];
        while (p_ptr < self.str_start[((file_name).wrapping_add(1i32)) as usize]) {
            {
                { let __ix3 = self.name_ptr; let __v4 = ((self.str_pool[(p_ptr) as usize]) as u8); self.name_of_file[(__ix3) as usize] = __v4; }
                self.name_ptr = (self.name_ptr).wrapping_add(1i32);
                p_ptr = (p_ptr).wrapping_add(1i32);
            }
        }
        self.name_length = (self.str_start[((file_name).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(file_name) as usize]);
        self.name_of_file[((self.name_length).wrapping_add(1i32)) as usize] = ((0i32) as u8);
    }

    /// This procedure copies file extension `ext` into the array
    /// `name_of_file` starting at position `name_length+1`.  It also sets the
    /// global variable `name_length` to the appropriate value.
    /// @<Procedures and functions for file-system interacting
    // §53
    pub fn add_extension(&mut self, mut ext: str_number) {
        let mut p_ptr: pool_pointer = 0; // §53
        self.name_ptr = (self.name_length).wrapping_add(1i32);
        p_ptr = self.str_start[(ext) as usize];
        while (p_ptr < self.str_start[((ext).wrapping_add(1i32)) as usize]) {
            {
                { let __ix5 = self.name_ptr; let __v6 = ((self.str_pool[(p_ptr) as usize]) as u8); self.name_of_file[(__ix5) as usize] = __v6; }
                self.name_ptr = (self.name_ptr).wrapping_add(1i32);
                p_ptr = (p_ptr).wrapping_add(1i32);
            }
        }
        self.name_length = (self.name_length).wrapping_add((self.str_start[((ext).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(ext) as usize]));
        self.name_of_file[((self.name_length).wrapping_add(1i32)) as usize] = ((0i32) as u8);
    }

    /// Once a sequence of characters has been appended to `str_pool`, it
    /// officially becomes a string when the function `make_string` is called.
    /// It returns the string number of the string it just made.
    /// @<Procedures and functions for handling numbers, characters, and strings
    // §47
    pub fn make_string(&mut self) -> str_number {
        let mut make_string: str_number = 0;
        if (self.str_ptr == self.max_strings) {
            {
                self.print_overflow();
                {
                    {
                        let __w1 = self.max_strings;
                        crate::system::wr_str(&mut self.log_file, "number of strings ");
                        crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        let __w1 = self.max_strings;
                        crate::system::wr_str(&mut self.standard_output, "number of strings ");
                        crate::system::wr_int(&mut self.standard_output, __w1, 0i32);
                        crate::system::wr_ln(&mut self.standard_output);
                    }
                }
                crate::system::end_of_TEX(self);
            }
        }
        self.str_ptr = (self.str_ptr).wrapping_add(1i32);
        { let __ix7 = self.str_ptr; let __v8 = self.pool_ptr; self.str_start[(__ix7) as usize] = __v8; }
        make_string = (self.str_ptr).wrapping_sub(1i32);
        make_string
    }

    /// This subroutine compares string `s` with another string that appears
    /// in the buffer `buf` between positions `bf_ptr` and `bf_ptr+len-1`; the
    /// result is `true` if and only if the strings are equal.
    /// @<Procedures and functions for handling numbers, characters, and strings
    // §49
    pub fn str_eq_buf(&mut self, mut s: str_number, buf: &mut buf_type, mut bf_ptr: buf_pointer, mut len: buf_pointer) -> bool {
        let mut str_eq_buf: bool = false;
        let mut i: buf_pointer = 0; // §49
        let mut j: pool_pointer = 0; // §49
        'l_exit_f: {
            if ((self.str_start[((s).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(s) as usize]) != len) {
                {
                    str_eq_buf = false;
                    break 'l_exit_f;
                }
            }
            i = bf_ptr;
            j = self.str_start[(s) as usize];
            while (j < self.str_start[((s).wrapping_add(1i32)) as usize]) {
                {
                    if (self.str_pool[(j) as usize] != (*buf)[(i) as usize]) {
                        {
                            str_eq_buf = false;
                            break 'l_exit_f;
                        }
                    }
                    i = (i).wrapping_add(1i32);
                    j = (j).wrapping_add(1i32);
                }
            }
            str_eq_buf = true;
        }
        str_eq_buf
    }

    /// This subroutine compares two `str_pool` strings and returns true
    /// `true` if and only if the strings are equal.
    /// @<Procedures and functions for handling numbers, characters, and strings
    // §50
    pub fn str_eq_str(&mut self, mut s1: str_number, mut s2: str_number) -> bool {
        let mut str_eq_str: bool = false;
        'l_exit_f: {
            if ((self.str_start[((s1).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(s1) as usize]) != (self.str_start[((s2).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(s2) as usize])) {
                {
                    str_eq_str = false;
                    break 'l_exit_f;
                }
            }
            self.p_ptr1 = self.str_start[(s1) as usize];
            self.p_ptr2 = self.str_start[(s2) as usize];
            while (self.p_ptr1 < self.str_start[((s1).wrapping_add(1i32)) as usize]) {
                {
                    if (self.str_pool[(self.p_ptr1) as usize] != self.str_pool[(self.p_ptr2) as usize]) {
                        {
                            str_eq_str = false;
                            break 'l_exit_f;
                        }
                    }
                    self.p_ptr1 = (self.p_ptr1).wrapping_add(1i32);
                    self.p_ptr2 = (self.p_ptr2).wrapping_add(1i32);
                }
            }
            str_eq_str = true;
        }
        str_eq_str
    }

    /// This system-independent procedure converts upper-case characters to
    /// lower case for the specified part of `buf`.  It is system independent
    /// because it uses only the internal representation for characters.
    // §55
    pub fn lower_case(&mut self, buf: &mut buf_type, mut bf_ptr: buf_pointer, mut len: buf_pointer) {
        let mut i: buf_pointer = 0; // §55
        if (len > 0i32) {
            {
                let __for_end_3 = ((bf_ptr).wrapping_add(len)).wrapping_sub(1i32);
                i = bf_ptr;
                while i <= __for_end_3 {
                    if (((*buf)[(i) as usize] >= 65i32) && ((*buf)[(i) as usize] <= 90i32)) {
                        (*buf)[(i) as usize] = ((*buf)[(i) as usize]).wrapping_add(32i32);
                    }
                    i = i.wrapping_add(1);
                }
            }
        }
    }

    /// This system-independent procedure is the same as the previous except
    /// that it converts lower- to upper-case letters.
    /// @<Procedures and functions for handling numbers, characters, and strings
    // §56
    pub fn upper_case(&mut self, buf: &mut buf_type, mut bf_ptr: buf_pointer, mut len: buf_pointer) {
        let mut i: buf_pointer = 0; // §56
        if (len > 0i32) {
            {
                let __for_end_3 = ((bf_ptr).wrapping_add(len)).wrapping_sub(1i32);
                i = bf_ptr;
                while i <= __for_end_3 {
                    if (((*buf)[(i) as usize] >= 97i32) && ((*buf)[(i) as usize] <= 122i32)) {
                        (*buf)[(i) as usize] = ((*buf)[(i) as usize]).wrapping_sub(32i32);
                    }
                    i = i.wrapping_add(1);
                }
            }
        }
    }

    /// Here is the subroutine that searches the hash table for a
    /// (string,~`str_ilk`) pair, where the string is of length `l>=0` and
    /// appears in `buffer[j..(j+l-1)]`.  If it finds the pair, it returns the
    /// corresponding hash-table location and sets the global variable
    /// `hash_found` to `true`.  Otherwise it sets `hash_found` to `false`,
    /// and if the parameter `insert_it` is `true`, it inserts the pair into
    /// the hash table, inserts the string into `str_pool` if not previously
    /// encountered, and returns its location.  Note that two different pairs
    /// can have the same string but different `str_ilk`s, in which case the
    /// second pair encountered, if `insert_it` were `true`, would be inserted
    /// into the hash table though its string wouldn't be inserted into
    /// `str_pool` because it would already be there.
    // §61
    pub fn str_lookup(&mut self, buf: &mut buf_type, mut j: buf_pointer, mut l: buf_pointer, mut ilk: str_ilk, mut insert_it: bool) -> hash_loc {
        let mut str_lookup: hash_loc = 0;
        let mut h: i32 = 0; // §61
        let mut p: hash_loc = 0; // §61
        let mut k: buf_pointer = 0; // §61
        let mut str_num: str_number = 0; // §61
        'l_L40_f: {
            'l_L45_f: {
                // §62
                {
                    h = 0i32;
                    k = j;
                    while (k < (j).wrapping_add(l)) {
                        {
                            h = ((h).wrapping_add(h)).wrapping_add((*buf)[(k) as usize]);
                            while (h >= self.hash_prime) {
                                h = (h).wrapping_sub(self.hash_prime);
                            }
                            k = (k).wrapping_add(1i32);
                        }
                    }
                }
                // §61
                p = (h).wrapping_add(hash_base);
                self.hash_found = false;
                str_num = 0i32;
                while true {
                    {
                        // §63
                        {
                            if (self.hash_text[(p) as usize] > 0i32) {
                                if { let mut __f1 = ::core::mem::take(&mut (*buf)); let __r = self.str_eq_buf(self.hash_text[(p) as usize], &mut __f1, j, l); (*buf) = __f1; __r } {
                                    if (self.hash_ilk[(p) as usize] == ilk) {
                                        {
                                            self.hash_found = true;
                                            break 'l_L40_f;
                                        }
                                    } else {
                                        {
                                            str_num = self.hash_text[(p) as usize];
                                        }
                                    }
                                }
                            }
                        }
                        // §61
                        if (self.hash_next[(p) as usize] == empty) {
                            {
                                if (!insert_it) {
                                    break 'l_L45_f;
                                }
                                // §64
                                {
                                    if (self.hash_text[(p) as usize] > 0i32) {
                                        {
                                            loop {
                                                if (self.hash_used == hash_base) {
                                                    {
                                                        self.print_overflow();
                                                        {
                                                            {
                                                                let __w1 = self.hash_size;
                                                                crate::system::wr_str(&mut self.log_file, "hash size ");
                                                                crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                                                                crate::system::wr_ln(&mut self.log_file);
                                                            }
                                                            {
                                                                let __w1 = self.hash_size;
                                                                crate::system::wr_str(&mut self.standard_output, "hash size ");
                                                                crate::system::wr_int(&mut self.standard_output, __w1, 0i32);
                                                                crate::system::wr_ln(&mut self.standard_output);
                                                            }
                                                        }
                                                        crate::system::end_of_TEX(self);
                                                    }
                                                }
                                                self.hash_used = (self.hash_used).wrapping_sub(1i32);
                                                if (self.hash_text[(self.hash_used) as usize] == 0i32) { break; }
                                            }
                                            { let __v9 = self.hash_used; self.hash_next[(p) as usize] = __v9; }
                                            p = self.hash_used;
                                        }
                                    }
                                    if (str_num > 0i32) {
                                        self.hash_text[(p) as usize] = str_num;
                                    } else {
                                        {
                                            {
                                                while ((self.pool_ptr).wrapping_add(l) > self.pool_size) {
                                                    self.pool_overflow();
                                                }
                                            }
                                            k = j;
                                            while (k < (j).wrapping_add(l)) {
                                                {
                                                    {
                                                        self.str_pool[(self.pool_ptr) as usize] = (*buf)[(k) as usize];
                                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                    }
                                                    k = (k).wrapping_add(1i32);
                                                }
                                            }
                                            { let __v10 = self.make_string(); self.hash_text[(p) as usize] = __v10; }
                                        }
                                    }
                                    self.hash_ilk[(p) as usize] = ilk;
                                }
                                // §61
                                break 'l_L40_f;
                            }
                        }
                        p = self.hash_next[(p) as usize];
                    }
                }
            }
        }
        str_lookup = p;
        str_lookup
    }

    /// This procedure initializes a pre-defined string of length at most
    /// `longest_pds`.
    /// @<Procedures and functions for handling numbers, characters, and strings
    // §70
    pub fn pre_define(&mut self, mut pds: pds_type, mut len: pds_len, mut ilk: str_ilk) {
        let mut i: pds_len = 0; // §70
        {
            let __for_end_2 = len;
            i = 1i32;
            while i <= __for_end_2 {
                { let __v11 = { let __s12 = (((self.c_char(pds, (i).wrapping_sub(1i32))) as u8)) as usize; self.xord[__s12] }; self.buffer[(i) as usize] = __v11; }
                i = i.wrapping_add(1);
            }
        }
        self.pre_def_loc = { let mut __f0 = ::core::mem::take(&mut self.buffer); let __r = self.str_lookup(&mut __f0, 1i32, len, ilk, true); self.buffer = __f0; __r };
    }

    /// This procedure takes the integer `int`, copies the appropriate
    /// `ASCII_code` string into `int_buf` starting at `int_begin`, and sets
    /// the `var` parameter `int_end` to the first unused `int_buf` location.
    /// The ASCII string will consist of decimal digits, the first of which
    /// will be not be a~0 if the integer is nonzero, with a prepended minus
    /// sign if the integer is negative.
    // §190
    pub fn int_to_ASCII(&mut self, mut the_int: i32, int_buf: &mut buf_type, mut int_begin: buf_pointer, int_end: &mut buf_pointer) {
        let mut int_ptr: buf_pointer = 0; // §190
        let mut int_xptr: buf_pointer = 0; // §190
        let mut int_tmp_val: ASCII_code = 0; // §190
        int_ptr = int_begin;
        if (the_int < 0i32) {
            {
                {
                    if (int_ptr == self.buf_size) {
                        self.buffer_overflow();
                    }
                    (*int_buf)[(int_ptr) as usize] = minus_sign;
                    int_ptr = (int_ptr).wrapping_add(1i32);
                }
                the_int = (the_int).wrapping_neg();
            }
        }
        int_xptr = int_ptr;
        loop {
            {
                if (int_ptr == self.buf_size) {
                    self.buffer_overflow();
                }
                (*int_buf)[(int_ptr) as usize] = (48i32).wrapping_add((the_int % 10i32));
                int_ptr = (int_ptr).wrapping_add(1i32);
            }
            the_int = (the_int / 10i32);
            if (the_int == 0i32) { break; }
        }
        (*int_end) = int_ptr;
        int_ptr = (int_ptr).wrapping_sub(1i32);
        while (int_xptr < int_ptr) {
            {
                int_tmp_val = (*int_buf)[(int_xptr) as usize];
                (*int_buf)[(int_xptr) as usize] = (*int_buf)[(int_ptr) as usize];
                (*int_buf)[(int_ptr) as usize] = int_tmp_val;
                int_ptr = (int_ptr).wrapping_sub(1i32);
                int_xptr = (int_xptr).wrapping_add(1i32);
            }
        }
    }

    /// This procedure adds (or restores) to `cite_list` a cite key; it is
    /// called only when `all_entries` is `true` or when adding
    /// cross~references, and it assumes that `cite_loc` and `lc_cite_loc` are
    /// set.  It also increments its argument.
    /// @<Procedures and functions for handling numbers, characters, and strings
    // §256
    pub fn add_database_cite(&mut self, new_cite: &mut cite_number) {
        self.check_cite_overflow((*new_cite));
        self.check_field_overflow((self.num_fields).wrapping_mul(((*new_cite)).wrapping_add(1i32)));
        { let __v13 = self.hash_text[(self.cite_loc) as usize]; self.cite_list[((*new_cite)) as usize] = __v13; }
        self.ilk_info[(self.cite_loc) as usize] = (*new_cite);
        { let __ix14 = self.lc_cite_loc; let __v15 = self.cite_loc; self.ilk_info[(__ix14) as usize] = __v15; }
        (*new_cite) = ((*new_cite)).wrapping_add(1i32);
    }

    /// Occasionally we need to figure out the hash-table location of a given
    /// cite-key string and its lower-case equivalent.  This function does
    /// that.  To perform the task it needs to borrow a buffer, a need that
    /// gives rise to the alias kludge---it helps make the stack space not
    /// overflow on some machines (and while it's at it, it'll borrow a
    /// pointer, too).  Finally, the function returns `true` if the cite key
    /// exists on `cite_list`, and its sets `cite_hash_found` according to
    /// whether or not it found the actual version (before `lower_case`ing) of
    /// the cite key; however, its {\sl raison d'\^$\mkern-8mu$etre\/}
    /// (literally, ``to eat a raisin'') is to compute `cite_loc` and
    /// `lc_cite_loc`.
    // §269
    pub fn find_cite_locs_for_this_cite_key(&mut self, mut cite_str: str_number) -> bool {
        let mut find_cite_locs_for_this_cite_key: bool = false;
        self.ex_buf_ptr = 0i32;
        self.tmp_ptr = self.str_start[(cite_str) as usize];
        self.tmp_end_ptr = self.str_start[((cite_str).wrapping_add(1i32)) as usize];
        while (self.tmp_ptr < self.tmp_end_ptr) {
            {
                { let __ix16 = self.ex_buf_ptr; let __v17 = self.str_pool[(self.tmp_ptr) as usize]; self.ex_buf[(__ix16) as usize] = __v17; }
                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                self.tmp_ptr = (self.tmp_ptr).wrapping_add(1i32);
            }
        }
        self.cite_loc = { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.str_lookup(&mut __f0, 0i32, (self.str_start[((cite_str).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(cite_str) as usize]), cite_ilk, false); self.ex_buf = __f0; __r };
        self.cite_hash_found = self.hash_found;
        { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.lower_case(&mut __f0, 0i32, (self.str_start[((cite_str).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(cite_str) as usize])); self.ex_buf = __f0; __r };
        self.lc_cite_loc = { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.str_lookup(&mut __f0, 0i32, (self.str_start[((cite_str).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(cite_str) as usize]), lc_cite_ilk, false); self.ex_buf = __f0; __r };
        if self.hash_found {
            find_cite_locs_for_this_cite_key = true;
        } else {
            find_cite_locs_for_this_cite_key = false;
        }
        find_cite_locs_for_this_cite_key
    }

    /// These next two procedures (actually, one procedures and one function,
    /// but who's counting) are subroutines for `quick_sort`, which follows.
    /// The `swap` procedure exchanges the two elements its arguments point
    /// to.
    /// @<Procedures and functions for handling numbers, characters, and strings
    // §291
    pub fn swap(&mut self, mut swap1: cite_number, mut swap2: cite_number) {
        let mut innocent_bystander: cite_number = 0; // §291
        innocent_bystander = self.cite_info[(swap2) as usize];
        { let __v18 = self.cite_info[(swap1) as usize]; self.cite_info[(swap2) as usize] = __v18; }
        self.cite_info[(swap1) as usize] = innocent_bystander;
    }

    /// The function `less_than` compares the two \.{sort.key\$}s indirectly
    /// pointed to by its arguments and returns `true` if the first argument's
    /// \.{sort.key\$} is lexicographically less than the second's (that is,
    /// alphabetically earlier).  In case of ties the function compares the
    /// indices `arg1` and `arg2`, which are assumed to be different, and
    /// returns `true` if the first is smaller.  This function uses
    /// `ASCII_code`s to compare, so it might give ``interesting'' results
    /// when handling nonletters.
    // §292
    pub fn less_than(&mut self, mut arg1: cite_number, mut arg2: cite_number) -> bool {
        let mut less_than: bool = false;
        let mut char_ptr: i32 = 0; // §292
        let mut ptr1: str_ent_loc = 0; // §292
        let mut ptr2: str_ent_loc = 0; // §292
        let mut char1: ASCII_code = 0; // §292
        let mut char2: ASCII_code = 0; // §292
        'l_exit_f: {
            ptr1 = ((arg1).wrapping_mul(self.num_ent_strs)).wrapping_add(self.sort_key_num);
            ptr2 = ((arg2).wrapping_mul(self.num_ent_strs)).wrapping_add(self.sort_key_num);
            char_ptr = 0i32;
            while true {
                {
                    char1 = self.entry_strs[(((ptr1).wrapping_mul((self.ent_str_size).wrapping_add(1i32))).wrapping_add(char_ptr)) as usize];
                    char2 = self.entry_strs[(((ptr2).wrapping_mul((self.ent_str_size).wrapping_add(1i32))).wrapping_add(char_ptr)) as usize];
                    if (char1 == end_of_string) {
                        if (char2 == end_of_string) {
                            if (arg1 < arg2) {
                                {
                                    less_than = true;
                                    break 'l_exit_f;
                                }
                            } else {
                                if (arg1 > arg2) {
                                    {
                                        less_than = false;
                                        break 'l_exit_f;
                                    }
                                } else {
                                    {
                                        {
                                            {
                                                crate::system::wr_str(&mut self.log_file, "Duplicate sort key");
                                            }
                                            {
                                                crate::system::wr_str(&mut self.standard_output, "Duplicate sort key");
                                            }
                                        }
                                        self.print_confusion();
                                        crate::system::end_of_TEX(self);
                                    }
                                }
                            }
                        } else {
                            {
                                less_than = true;
                                break 'l_exit_f;
                            }
                        }
                    } else {
                        if (char2 == end_of_string) {
                            {
                                less_than = false;
                                break 'l_exit_f;
                            }
                        } else {
                            if (char1 < char2) {
                                {
                                    less_than = true;
                                    break 'l_exit_f;
                                }
                            } else {
                                if (char1 > char2) {
                                    {
                                        less_than = false;
                                        break 'l_exit_f;
                                    }
                                }
                            }
                        }
                    }
                    char_ptr = (char_ptr).wrapping_add(1i32);
                }
            }
        }
        less_than
    }

    /// Here's the actual procedure.
    // §294
    pub fn quick_sort(&mut self, mut left_end: cite_number, mut right_end: cite_number) {
        let mut left: cite_number = 0; // §294
        let mut right: cite_number = 0; // §294
        let mut insert_ptr: cite_number = 0; // §294
        let mut middle: cite_number = 0; // §294
        let mut partition: cite_number = 0; // §294
        if ((right_end).wrapping_sub(left_end) < short_list) {
            // §295
            {
                {
                    let __for_end_4 = right_end;
                    insert_ptr = (left_end).wrapping_add(1i32);
                    while insert_ptr <= __for_end_4 {
                        {
                            'l_L24_f: {
                                {
                                    let __for_end_8 = (left_end).wrapping_add(1i32);
                                    right = insert_ptr;
                                    while right >= __for_end_8 {
                                        {
                                            if self.less_than(self.cite_info[((right).wrapping_sub(1i32)) as usize], self.cite_info[(right) as usize]) {
                                                break 'l_L24_f;
                                            }
                                            self.swap((right).wrapping_sub(1i32), right);
                                        }
                                        right = right.wrapping_sub(1);
                                    }
                                }
                            }
                        }
                        insert_ptr = insert_ptr.wrapping_add(1);
                    }
                }
            }
        } else {
            // §294
            {
                // §296
                {
                    left = (left_end).wrapping_add(4i32);
                    middle = ((left_end).wrapping_add(right_end) / 2i32);
                    right = (right_end).wrapping_sub(4i32);
                    if self.less_than(self.cite_info[(left) as usize], self.cite_info[(middle) as usize]) {
                        if self.less_than(self.cite_info[(middle) as usize], self.cite_info[(right) as usize]) {
                            self.swap(left_end, middle);
                        } else {
                            if self.less_than(self.cite_info[(left) as usize], self.cite_info[(right) as usize]) {
                                self.swap(left_end, right);
                            } else {
                                self.swap(left_end, left);
                            }
                        }
                    } else {
                        if self.less_than(self.cite_info[(right) as usize], self.cite_info[(middle) as usize]) {
                            self.swap(left_end, middle);
                        } else {
                            if self.less_than(self.cite_info[(right) as usize], self.cite_info[(left) as usize]) {
                                self.swap(left_end, right);
                            } else {
                                self.swap(left_end, left);
                            }
                        }
                    }
                }
                // §297
                {
                    partition = self.cite_info[(left_end) as usize];
                    left = (left_end).wrapping_add(1i32);
                    right = right_end;
                    loop {
                        while self.less_than(self.cite_info[(left) as usize], partition) {
                            left = (left).wrapping_add(1i32);
                        }
                        while self.less_than(partition, self.cite_info[(right) as usize]) {
                            right = (right).wrapping_sub(1i32);
                        }
                        if (left < right) {
                            {
                                self.swap(left, right);
                                left = (left).wrapping_add(1i32);
                                right = (right).wrapping_sub(1i32);
                            }
                        }
                        if (left == (right).wrapping_add(1i32)) { break; }
                    }
                    self.swap(left_end, right);
                    self.quick_sort(left_end, (right).wrapping_sub(1i32));
                    self.quick_sort(left, right_end);
                }
            }
        }
    }

    /// This procedure inserts a `built_in` function into the hash table and
    /// initializes the corresponding pre-defined string (of length at most
    /// `longest_pds`).  The array `fn_info` contains a number from 0 through
    /// the number of `built_in` functions minus 1 (i.e., `num_blt_in_fns - 1`
    /// if we're keeping statistics); this number is used by a `case`
    /// statement to execute this function and is used for keeping execution
    /// counts when keeping statistics.
    /// @<Procedures and functions for handling numbers, characters, and strings
    // §326
    pub fn build_in(&mut self, mut pds: pds_type, mut len: pds_len, fn_hash_loc: &mut hash_loc, mut blt_in_num: blt_in_range) {
        self.pre_define(pds, len, bst_fn_ilk);
        (*fn_hash_loc) = self.pre_def_loc;
        self.fn_type[((*fn_hash_loc)) as usize] = built_in;
        self.ilk_info[((*fn_hash_loc)) as usize] = blt_in_num;
        self.blt_in_loc[(blt_in_num) as usize] = (*fn_hash_loc);
        self.execution_count[(blt_in_num) as usize] = 0i32;
    }

    /// This is a procedure so that `initialize` is smaller.
    /// @<Procedures and functions for handling numbers, characters, and strings
    // §327
    pub fn pre_def_certain_strings(&mut self) {
        // §68
        self.pre_define(".aux        ", 4i32, file_ext_ilk);
        self.s_aux_extension = self.hash_text[(self.pre_def_loc) as usize];
        self.pre_define(".bbl        ", 4i32, file_ext_ilk);
        self.s_bbl_extension = self.hash_text[(self.pre_def_loc) as usize];
        self.pre_define(".blg        ", 4i32, file_ext_ilk);
        self.s_log_extension = self.hash_text[(self.pre_def_loc) as usize];
        self.pre_define(".bst        ", 4i32, file_ext_ilk);
        self.s_bst_extension = self.hash_text[(self.pre_def_loc) as usize];
        self.pre_define(".bib        ", 4i32, file_ext_ilk);
        self.s_bib_extension = self.hash_text[(self.pre_def_loc) as usize];
        self.pre_define("texinputs:  ", 10i32, file_area_ilk);
        self.s_bst_area = self.hash_text[(self.pre_def_loc) as usize];
        self.pre_define("texbib:     ", 7i32, file_area_ilk);
        self.s_bib_area = self.hash_text[(self.pre_def_loc) as usize];
        // §72
        self.pre_define("\\citation   ", 9i32, aux_command_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_aux_citation;
        self.pre_define("\\bibdata    ", 8i32, aux_command_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_aux_bibdata;
        self.pre_define("\\bibstyle   ", 9i32, aux_command_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_aux_bibstyle;
        self.pre_define("\\@input     ", 7i32, aux_command_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_aux_input;
        self.pre_define("entry       ", 5i32, bst_command_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_bst_entry;
        self.pre_define("execute     ", 7i32, bst_command_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_bst_execute;
        self.pre_define("function    ", 8i32, bst_command_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_bst_function;
        self.pre_define("integers    ", 8i32, bst_command_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_bst_integers;
        self.pre_define("iterate     ", 7i32, bst_command_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_bst_iterate;
        self.pre_define("macro       ", 5i32, bst_command_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_bst_macro;
        self.pre_define("read        ", 4i32, bst_command_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_bst_read;
        self.pre_define("reverse     ", 7i32, bst_command_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_bst_reverse;
        self.pre_define("sort        ", 4i32, bst_command_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_bst_sort;
        self.pre_define("strings     ", 7i32, bst_command_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_bst_strings;
        self.pre_define("comment     ", 7i32, bib_command_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_bib_comment;
        self.pre_define("preamble    ", 8i32, bib_command_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_bib_preamble;
        self.pre_define("string      ", 6i32, bib_command_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_bib_string;
        // §325
        { let mut __f2 = ::core::mem::take(&mut self.b_equals); let __r = self.build_in("=           ", 1i32, &mut __f2, n_equals); self.b_equals = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_greater_than); let __r = self.build_in(">           ", 1i32, &mut __f2, n_greater_than); self.b_greater_than = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_less_than); let __r = self.build_in("<           ", 1i32, &mut __f2, n_less_than); self.b_less_than = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_plus); let __r = self.build_in("+           ", 1i32, &mut __f2, n_plus); self.b_plus = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_minus); let __r = self.build_in("-           ", 1i32, &mut __f2, n_minus); self.b_minus = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_concatenate); let __r = self.build_in("*           ", 1i32, &mut __f2, n_concatenate); self.b_concatenate = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_gets); let __r = self.build_in(":=          ", 2i32, &mut __f2, n_gets); self.b_gets = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_add_period); let __r = self.build_in("add.period$ ", 11i32, &mut __f2, n_add_period); self.b_add_period = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_call_type); let __r = self.build_in("call.type$  ", 10i32, &mut __f2, n_call_type); self.b_call_type = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_change_case); let __r = self.build_in("change.case$", 12i32, &mut __f2, n_change_case); self.b_change_case = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_chr_to_int); let __r = self.build_in("chr.to.int$ ", 11i32, &mut __f2, n_chr_to_int); self.b_chr_to_int = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_cite); let __r = self.build_in("cite$       ", 5i32, &mut __f2, n_cite); self.b_cite = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_duplicate); let __r = self.build_in("duplicate$  ", 10i32, &mut __f2, n_duplicate); self.b_duplicate = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_empty); let __r = self.build_in("empty$      ", 6i32, &mut __f2, n_empty); self.b_empty = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_format_name); let __r = self.build_in("format.name$", 12i32, &mut __f2, n_format_name); self.b_format_name = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_if); let __r = self.build_in("if$         ", 3i32, &mut __f2, n_if); self.b_if = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_int_to_chr); let __r = self.build_in("int.to.chr$ ", 11i32, &mut __f2, n_int_to_chr); self.b_int_to_chr = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_int_to_str); let __r = self.build_in("int.to.str$ ", 11i32, &mut __f2, n_int_to_str); self.b_int_to_str = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_missing); let __r = self.build_in("missing$    ", 8i32, &mut __f2, n_missing); self.b_missing = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_newline); let __r = self.build_in("newline$    ", 8i32, &mut __f2, n_newline); self.b_newline = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_num_names); let __r = self.build_in("num.names$  ", 10i32, &mut __f2, n_num_names); self.b_num_names = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_pop); let __r = self.build_in("pop$        ", 4i32, &mut __f2, n_pop); self.b_pop = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_preamble); let __r = self.build_in("preamble$   ", 9i32, &mut __f2, n_preamble); self.b_preamble = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_purify); let __r = self.build_in("purify$     ", 7i32, &mut __f2, n_purify); self.b_purify = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_quote); let __r = self.build_in("quote$      ", 6i32, &mut __f2, n_quote); self.b_quote = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_skip); let __r = self.build_in("skip$       ", 5i32, &mut __f2, n_skip); self.b_skip = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_stack); let __r = self.build_in("stack$      ", 6i32, &mut __f2, n_stack); self.b_stack = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_substring); let __r = self.build_in("substring$  ", 10i32, &mut __f2, n_substring); self.b_substring = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_swap); let __r = self.build_in("swap$       ", 5i32, &mut __f2, n_swap); self.b_swap = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_text_length); let __r = self.build_in("text.length$", 12i32, &mut __f2, n_text_length); self.b_text_length = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_text_prefix); let __r = self.build_in("text.prefix$", 12i32, &mut __f2, n_text_prefix); self.b_text_prefix = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_top_stack); let __r = self.build_in("top$        ", 4i32, &mut __f2, n_top_stack); self.b_top_stack = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_type); let __r = self.build_in("type$       ", 5i32, &mut __f2, n_type); self.b_type = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_warning); let __r = self.build_in("warning$    ", 8i32, &mut __f2, n_warning); self.b_warning = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_while); let __r = self.build_in("while$      ", 6i32, &mut __f2, n_while); self.b_while = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_width); let __r = self.build_in("width$      ", 6i32, &mut __f2, n_width); self.b_width = __f2; __r };
        { let mut __f2 = ::core::mem::take(&mut self.b_write); let __r = self.build_in("write$      ", 6i32, &mut __f2, n_write); self.b_write = __f2; __r };
        // §330
        self.pre_define("            ", 0i32, text_ilk);
        self.s_null = self.hash_text[(self.pre_def_loc) as usize];
        self.fn_type[(self.pre_def_loc) as usize] = str_literal;
        self.pre_define("default.type", 12i32, text_ilk);
        self.s_default = self.hash_text[(self.pre_def_loc) as usize];
        self.fn_type[(self.pre_def_loc) as usize] = str_literal;
        self.b_default = self.b_skip;
        self.preamble_ptr = 0i32;
        self.pre_define("i           ", 1i32, control_seq_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_i;
        self.pre_define("j           ", 1i32, control_seq_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_j;
        self.pre_define("oe          ", 2i32, control_seq_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_oe;
        self.pre_define("OE          ", 2i32, control_seq_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_oe_upper;
        self.pre_define("ae          ", 2i32, control_seq_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_ae;
        self.pre_define("AE          ", 2i32, control_seq_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_ae_upper;
        self.pre_define("aa          ", 2i32, control_seq_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_aa;
        self.pre_define("AA          ", 2i32, control_seq_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_aa_upper;
        self.pre_define("o           ", 1i32, control_seq_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_o;
        self.pre_define("O           ", 1i32, control_seq_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_o_upper;
        self.pre_define("l           ", 1i32, control_seq_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_l;
        self.pre_define("L           ", 1i32, control_seq_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_l_upper;
        self.pre_define("ss          ", 2i32, control_seq_ilk);
        self.ilk_info[(self.pre_def_loc) as usize] = n_ss;
        // §331
        self.pre_define("crossref    ", 8i32, bst_fn_ilk);
        self.fn_type[(self.pre_def_loc) as usize] = field;
        { let __ix19 = self.pre_def_loc; let __v20 = self.num_fields; self.ilk_info[(__ix19) as usize] = __v20; }
        self.crossref_num = self.num_fields;
        self.num_fields = (self.num_fields).wrapping_add(1i32);
        self.num_pre_defined_fields = self.num_fields;
        self.pre_define("sort.key$   ", 9i32, bst_fn_ilk);
        self.fn_type[(self.pre_def_loc) as usize] = str_entry_var;
        { let __ix21 = self.pre_def_loc; let __v22 = self.num_ent_strs; self.ilk_info[(__ix21) as usize] = __v22; }
        self.sort_key_num = self.num_ent_strs;
        self.num_ent_strs = (self.num_ent_strs).wrapping_add(1i32);
        self.pre_define("entry.max$  ", 10i32, bst_fn_ilk);
        self.fn_type[(self.pre_def_loc) as usize] = int_global_var;
        { let __ix23 = self.pre_def_loc; let __v24 = self.ent_str_size; self.ilk_info[(__ix23) as usize] = __v24; }
        self.pre_define("global.max$ ", 11i32, bst_fn_ilk);
        self.fn_type[(self.pre_def_loc) as usize] = int_global_var;
        { let __ix25 = self.pre_def_loc; let __v26 = self.glob_str_size; self.ilk_info[(__ix25) as usize] = __v26; }
    }

    /// This function scans the `buffer` for the next token, starting at the
    /// global variable `buf_ptr2` and ending just before either the single
    /// specified stop-character or the end of the current line, whichever
    /// comes first, respectively returning `true` or `false`; afterward,
    /// `scan_char` is the first character following this token.
    /// @<Procedures and functions for input scanning
    // §76
    pub fn scan1(&mut self, mut char1: ASCII_code) -> bool {
        let mut scan1: bool = false;
        self.buf_ptr1 = self.buf_ptr2;
        while ((self.buffer[(self.buf_ptr2) as usize] != char1) && (self.buf_ptr2 < self.last)) {
            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
        }
        if (self.buf_ptr2 < self.last) {
            scan1 = true;
        } else {
            scan1 = false;
        }
        scan1
    }

    /// This function is the same but stops at `white_space` characters as well.
    /// @<Procedures and functions for input scanning
    // §77
    pub fn scan1_white(&mut self, mut char1: ASCII_code) -> bool {
        let mut scan1_white: bool = false;
        self.buf_ptr1 = self.buf_ptr2;
        while (((self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] != white_space) && (self.buffer[(self.buf_ptr2) as usize] != char1)) && (self.buf_ptr2 < self.last)) {
            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
        }
        if (self.buf_ptr2 < self.last) {
            scan1_white = true;
        } else {
            scan1_white = false;
        }
        scan1_white
    }

    /// This function is similar to `scan1`, but stops at either of two
    /// stop-characters as well as the end of the current line.
    /// @<Procedures and functions for input scanning
    // §78
    pub fn scan2(&mut self, mut char1: ASCII_code, mut char2: ASCII_code) -> bool {
        let mut scan2: bool = false;
        self.buf_ptr1 = self.buf_ptr2;
        while (((self.buffer[(self.buf_ptr2) as usize] != char1) && (self.buffer[(self.buf_ptr2) as usize] != char2)) && (self.buf_ptr2 < self.last)) {
            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
        }
        if (self.buf_ptr2 < self.last) {
            scan2 = true;
        } else {
            scan2 = false;
        }
        scan2
    }

    /// This function is the same but stops at `white_space` characters as well.
    /// @<Procedures and functions for input scanning
    // §79
    pub fn scan2_white(&mut self, mut char1: ASCII_code, mut char2: ASCII_code) -> bool {
        let mut scan2_white: bool = false;
        self.buf_ptr1 = self.buf_ptr2;
        while ((((self.buffer[(self.buf_ptr2) as usize] != char1) && (self.buffer[(self.buf_ptr2) as usize] != char2)) && (self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] != white_space)) && (self.buf_ptr2 < self.last)) {
            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
        }
        if (self.buf_ptr2 < self.last) {
            scan2_white = true;
        } else {
            scan2_white = false;
        }
        scan2_white
    }

    /// This function is similar to `scan2`, but stops at either of three
    /// stop-characters as well as the end of the current line.
    /// @<Procedures and functions for input scanning
    // §80
    pub fn scan3(&mut self, mut char1: ASCII_code, mut char2: ASCII_code, mut char3: ASCII_code) -> bool {
        let mut scan3: bool = false;
        self.buf_ptr1 = self.buf_ptr2;
        while ((((self.buffer[(self.buf_ptr2) as usize] != char1) && (self.buffer[(self.buf_ptr2) as usize] != char2)) && (self.buffer[(self.buf_ptr2) as usize] != char3)) && (self.buf_ptr2 < self.last)) {
            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
        }
        if (self.buf_ptr2 < self.last) {
            scan3 = true;
        } else {
            scan3 = false;
        }
        scan3
    }

    /// This function scans for letters, stopping at the first nonletter; it
    /// returns `true` if there is at least one letter.
    /// @<Procedures and functions for input scanning
    // §81
    pub fn scan_alpha(&mut self) -> bool {
        let mut scan_alpha: bool = false;
        self.buf_ptr1 = self.buf_ptr2;
        while ((self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] == alpha) && (self.buf_ptr2 < self.last)) {
            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
        }
        if ((self.buf_ptr2).wrapping_sub(self.buf_ptr1) == 0i32) {
            scan_alpha = false;
        } else {
            scan_alpha = true;
        }
        scan_alpha
    }

}
