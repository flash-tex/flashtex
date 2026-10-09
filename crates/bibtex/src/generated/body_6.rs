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
    /// The `built_in` function {\.{int.to.chr\$}} pops the top (integer)
    /// literal, interpreted as the `ASCII_code` of a single character,
    /// converts it to the corresponding single-character string, and pushes
    /// this string.  If the literal isn't an appropriate integer, it
    /// complains and pushes the null string.
    /// @<`execute_fn`({\.{int.to.chr\$}})
    // §413
    pub fn x_int_to_chr(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        if (self.pop_typ1 != stk_int) {
            {
                self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_int);
                self.push_lit_stk(self.s_null, stk_str);
            }
        } else {
            if ((self.pop_lit1 < 0i32) || (self.pop_lit1 > 127i32)) {
                {
                    {
                        {
                            {
                                let __w0 = self.pop_lit1;
                                crate::system::wr_int(&mut self.log_file, __w0, 0i32);
                                crate::system::wr_str(&mut self.log_file, " isn't valid ASCII");
                            }
                            {
                                let __w0 = self.pop_lit1;
                                crate::system::wr_int(&mut self.standard_output, __w0, 0i32);
                                crate::system::wr_str(&mut self.standard_output, " isn't valid ASCII");
                            }
                        }
                        self.bst_ex_warn_print();
                    }
                    self.push_lit_stk(self.s_null, stk_str);
                }
            } else {
                {
                    {
                        while ((self.pool_ptr).wrapping_add(1i32) > self.pool_size) {
                            self.pool_overflow();
                        }
                    }
                    {
                        { let __ix141 = self.pool_ptr; let __v142 = self.pop_lit1; self.str_pool[(__ix141) as usize] = __v142; }
                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                    }
                    { let __a143_0 = self.make_string(); let __a143_1 = stk_str; self.push_lit_stk(__a143_0, __a143_1) };
                }
            }
        }
    }

    /// The `built_in` function {\.{int.to.str\$}} pops the top (integer)
    /// literal, converts it to its (unique) string equivalent, and pushes
    /// this string.  If the literal isn't an integer, it complains and pushes
    /// the null string.
    /// @<`execute_fn`({\.{int.to.str\$}})
    // §414
    pub fn x_int_to_str(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        if (self.pop_typ1 != stk_int) {
            {
                self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_int);
                self.push_lit_stk(self.s_null, stk_str);
            }
        } else {
            {
                { let mut __f1 = ::core::mem::take(&mut self.ex_buf); let mut __f3 = ::core::mem::take(&mut self.ex_buf_length); let __r = self.int_to_ASCII(self.pop_lit1, &mut __f1, 0i32, &mut __f3); self.ex_buf = __f1; self.ex_buf_length = __f3; __r };
                self.add_pool_buf_and_push();
            }
        }
    }

    /// The `built_in` function {\.{missing\$}} pops the top literal and
    /// pushes the integer 1 if it's a missing field, 0 otherwise.  If the
    /// literal isn't a missing field or a string, it complains and pushes 0.
    /// Unlike \.{empty\$}, this function should be called only when
    /// `mess_with_entries` is true.
    /// @<`execute_fn`({\.{missing\$}})
    // §415
    pub fn x_missing(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        if (!self.mess_with_entries) {
            self.bst_cant_mess_with_entries_print();
        } else {
            if ((self.pop_typ1 != stk_str) && (self.pop_typ1 != stk_field_missing)) {
                {
                    if (self.pop_typ1 != stk_empty) {
                        {
                            self.print_stk_lit(self.pop_lit1, self.pop_typ1);
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, ", not a string or missing field,");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, ", not a string or missing field,");
                                    }
                                }
                                self.bst_ex_warn_print();
                            }
                        }
                    }
                    self.push_lit_stk(0i32, stk_int);
                }
            } else {
                if (self.pop_typ1 == stk_field_missing) {
                    self.push_lit_stk(1i32, stk_int);
                } else {
                    self.push_lit_stk(0i32, stk_int);
                }
            }
        }
    }

    /// The `built_in` function {\.{num.names\$}} pops the top (string)
    /// literal; it pushes the number of names the string represents---one
    /// plus the number of occurrences of the substring ``and'' (ignoring case
    /// differences) surrounded by nonnull `white_space` at the top brace
    /// level.  If the literal isn't a string, it complains and pushes the
    /// value 0.
    /// @<`execute_fn`({\.{num.names\$}})
    // §417
    pub fn x_num_names(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        if (self.pop_typ1 != stk_str) {
            {
                self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_str);
                self.push_lit_stk(0i32, stk_int);
            }
        } else {
            {
                self.ex_buf_length = 0i32;
                self.add_buf_pool(self.pop_lit1);
                // §418
                {
                    self.ex_buf_ptr = 0i32;
                    self.num_names = 0i32;
                    while (self.ex_buf_ptr < self.ex_buf_length) {
                        {
                            self.name_scan_for_and(self.pop_lit1);
                            self.num_names = (self.num_names).wrapping_add(1i32);
                        }
                    }
                }
                // §417
                self.push_lit_stk(self.num_names, stk_int);
            }
        }
    }

    /// The `built_in` function {\.{preamble\$}} pushes onto the stack the
    /// concatenation of all the \.{preamble} strings read from the database
    /// files.
    /// @<`execute_fn`({\.{preamble\$}})
    // §420
    pub fn x_preamble(&mut self) {
        self.ex_buf_length = 0i32;
        self.preamble_ptr = 0i32;
        while (self.preamble_ptr < self.num_preamble_strings) {
            {
                self.add_buf_pool(self.s_preamble[(self.preamble_ptr) as usize]);
                self.preamble_ptr = (self.preamble_ptr).wrapping_add(1i32);
            }
        }
        self.add_pool_buf_and_push();
    }

    /// The `built_in` function {\.{purify\$}} pops the top (string) literal,
    /// removes nonalphanumeric characters except for `white_space` and
    /// `sep_char` characters (these get converted to a `space`) and removes
    /// certain alphabetic characters contained in the control sequences
    /// associated with a special character, and pushes the resulting string.
    /// If the literal isn't a string, it complains and pushes the null
    /// string.
    /// @<`execute_fn`({\.{purify\$}})
    // §421
    pub fn x_purify(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        if (self.pop_typ1 != stk_str) {
            {
                self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_str);
                self.push_lit_stk(self.s_null, stk_str);
            }
        } else {
            {
                self.ex_buf_length = 0i32;
                self.add_buf_pool(self.pop_lit1);
                // §422
                {
                    self.brace_level = 0i32;
                    self.ex_buf_xptr = 0i32;
                    self.ex_buf_ptr = 0i32;
                    while (self.ex_buf_ptr < self.ex_buf_length) {
                        {
                            match self.lex_class[(self.ex_buf[(self.ex_buf_ptr) as usize]) as usize] {
                                white_space | sep_char => {
                                    {
                                        self.ex_buf[(self.ex_buf_xptr) as usize] = space;
                                        self.ex_buf_xptr = (self.ex_buf_xptr).wrapping_add(1i32);
                                    }
                                }
                                alpha | numeric => {
                                    {
                                        { let __ix144 = self.ex_buf_xptr; let __v145 = self.ex_buf[(self.ex_buf_ptr) as usize]; self.ex_buf[(__ix144) as usize] = __v145; }
                                        self.ex_buf_xptr = (self.ex_buf_xptr).wrapping_add(1i32);
                                    }
                                }
                                _ => {
                                    if (self.ex_buf[(self.ex_buf_ptr) as usize] == left_brace) {
                                        {
                                            self.brace_level = (self.brace_level).wrapping_add(1i32);
                                            if ((self.brace_level == 1i32) && ((self.ex_buf_ptr).wrapping_add(1i32) < self.ex_buf_length)) {
                                                if (self.ex_buf[((self.ex_buf_ptr).wrapping_add(1i32)) as usize] == backslash) {
                                                    // §423
                                                    {
                                                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                        while ((self.ex_buf_ptr < self.ex_buf_length) && (self.brace_level > 0i32)) {
                                                            {
                                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                                self.ex_buf_yptr = self.ex_buf_ptr;
                                                                while ((self.ex_buf_ptr < self.ex_buf_length) && (self.lex_class[(self.ex_buf[(self.ex_buf_ptr) as usize]) as usize] == alpha)) {
                                                                    self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                                }
                                                                self.control_seq_loc = { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.str_lookup(&mut __f0, self.ex_buf_yptr, (self.ex_buf_ptr).wrapping_sub(self.ex_buf_yptr), control_seq_ilk, false); self.ex_buf = __f0; __r };
                                                                if self.hash_found {
                                                                    // §424
                                                                    {
                                                                        { let __ix146 = self.ex_buf_xptr; let __v147 = self.ex_buf[(self.ex_buf_yptr) as usize]; self.ex_buf[(__ix146) as usize] = __v147; }
                                                                        self.ex_buf_xptr = (self.ex_buf_xptr).wrapping_add(1i32);
                                                                        match self.ilk_info[(self.control_seq_loc) as usize] {
                                                                            n_oe | n_oe_upper | n_ae | n_ae_upper | n_ss => {
                                                                                {
                                                                                    { let __ix148 = self.ex_buf_xptr; let __v149 = self.ex_buf[((self.ex_buf_yptr).wrapping_add(1i32)) as usize]; self.ex_buf[(__ix148) as usize] = __v149; }
                                                                                    self.ex_buf_xptr = (self.ex_buf_xptr).wrapping_add(1i32);
                                                                                }
                                                                            }
                                                                            _ => {
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                // §423
                                                                while (((self.ex_buf_ptr < self.ex_buf_length) && (self.brace_level > 0i32)) && (self.ex_buf[(self.ex_buf_ptr) as usize] != backslash)) {
                                                                    {
                                                                        match self.lex_class[(self.ex_buf[(self.ex_buf_ptr) as usize]) as usize] {
                                                                            alpha | numeric => {
                                                                                {
                                                                                    { let __ix150 = self.ex_buf_xptr; let __v151 = self.ex_buf[(self.ex_buf_ptr) as usize]; self.ex_buf[(__ix150) as usize] = __v151; }
                                                                                    self.ex_buf_xptr = (self.ex_buf_xptr).wrapping_add(1i32);
                                                                                }
                                                                            }
                                                                            _ => {
                                                                                if (self.ex_buf[(self.ex_buf_ptr) as usize] == right_brace) {
                                                                                    self.brace_level = (self.brace_level).wrapping_sub(1i32);
                                                                                } else {
                                                                                    if (self.ex_buf[(self.ex_buf_ptr) as usize] == left_brace) {
                                                                                        self.brace_level = (self.brace_level).wrapping_add(1i32);
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_sub(1i32);
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        // §422
                                        if (self.ex_buf[(self.ex_buf_ptr) as usize] == right_brace) {
                                            if (self.brace_level > 0i32) {
                                                self.brace_level = (self.brace_level).wrapping_sub(1i32);
                                            }
                                        }
                                    }
                                }
                            }
                            self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                        }
                    }
                    self.ex_buf_length = self.ex_buf_xptr;
                }
                // §421
                self.add_pool_buf_and_push();
            }
        }
    }

    /// The `built_in` function {\.{quote\$}} pushes the string consisting of
    /// the `double_quote` character.
    /// @<`execute_fn`({\.{quote\$}})
    // §425
    pub fn x_quote(&mut self) {
        {
            while ((self.pool_ptr).wrapping_add(1i32) > self.pool_size) {
                self.pool_overflow();
            }
        }
        {
            self.str_pool[(self.pool_ptr) as usize] = double_quote;
            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
        }
        { let __a152_0 = self.make_string(); let __a152_1 = stk_str; self.push_lit_stk(__a152_0, __a152_1) };
    }

    /// The `built_in` function {\.{substring\$}} pops the top three literals
    /// (they are the two integers literals `pop_lit1` and `pop_lit2` and a
    /// string literal, in that order).  It pushes the substring of the (at
    /// most) `pop_lit1` consecutive characters starting at the `pop_lit2`th
    /// character (assuming 1-based indexing) if `pop_lit2` is positive, and
    /// ending at the `-pop_lit2`th character from the end if `pop_lit2` is
    /// negative (where the first character from the end is the last
    /// character).  If any of the types is incorrect, it complain and pushes
    /// the null string.
    /// @<`execute_fn`({\.{substring\$}})
    // §428
    pub fn x_substring(&mut self) {
        'l_exit_f: {
            { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
            { let mut __f0 = ::core::mem::take(&mut self.pop_lit2); let mut __f1 = ::core::mem::take(&mut self.pop_typ2); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit2 = __f0; self.pop_typ2 = __f1; __r };
            { let mut __f0 = ::core::mem::take(&mut self.pop_lit3); let mut __f1 = ::core::mem::take(&mut self.pop_typ3); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit3 = __f0; self.pop_typ3 = __f1; __r };
            if (self.pop_typ1 != stk_int) {
                {
                    self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_int);
                    self.push_lit_stk(self.s_null, stk_str);
                }
            } else {
                if (self.pop_typ2 != stk_int) {
                    {
                        self.print_wrong_stk_lit(self.pop_lit2, self.pop_typ2, stk_int);
                        self.push_lit_stk(self.s_null, stk_str);
                    }
                } else {
                    if (self.pop_typ3 != stk_str) {
                        {
                            self.print_wrong_stk_lit(self.pop_lit3, self.pop_typ3, stk_str);
                            self.push_lit_stk(self.s_null, stk_str);
                        }
                    } else {
                        {
                            self.sp_length = (self.str_start[((self.pop_lit3).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pop_lit3) as usize]);
                            if (self.pop_lit1 >= self.sp_length) {
                                if ((self.pop_lit2 == 1i32) || (self.pop_lit2 == (1i32).wrapping_neg())) {
                                    {
                                        {
                                            if (self.lit_stack[(self.lit_stk_ptr) as usize] >= self.cmd_str_ptr) {
                                                {
                                                    self.str_ptr = (self.str_ptr).wrapping_add(1i32);
                                                    self.pool_ptr = self.str_start[(self.str_ptr) as usize];
                                                }
                                            }
                                            self.lit_stk_ptr = (self.lit_stk_ptr).wrapping_add(1i32);
                                        }
                                        break 'l_exit_f;
                                    }
                                }
                            }
                            if ((((self.pop_lit1 <= 0i32) || (self.pop_lit2 == 0i32)) || (self.pop_lit2 > self.sp_length)) || (self.pop_lit2 < (self.sp_length).wrapping_neg())) {
                                {
                                    self.push_lit_stk(self.s_null, stk_str);
                                    break 'l_exit_f;
                                }
                            } else {
                                // §429
                                {
                                    if (self.pop_lit2 > 0i32) {
                                        {
                                            if (self.pop_lit1 > (self.sp_length).wrapping_sub((self.pop_lit2).wrapping_sub(1i32))) {
                                                self.pop_lit1 = (self.sp_length).wrapping_sub((self.pop_lit2).wrapping_sub(1i32));
                                            }
                                            self.sp_ptr = (self.str_start[(self.pop_lit3) as usize]).wrapping_add((self.pop_lit2).wrapping_sub(1i32));
                                            self.sp_end = (self.sp_ptr).wrapping_add(self.pop_lit1);
                                            if (self.pop_lit2 == 1i32) {
                                                if (self.pop_lit3 >= self.cmd_str_ptr) {
                                                    {
                                                        { let __ix153 = (self.pop_lit3).wrapping_add(1i32); let __v154 = self.sp_end; self.str_start[(__ix153) as usize] = __v154; }
                                                        {
                                                            self.str_ptr = (self.str_ptr).wrapping_add(1i32);
                                                            self.pool_ptr = self.str_start[(self.str_ptr) as usize];
                                                        }
                                                        self.lit_stk_ptr = (self.lit_stk_ptr).wrapping_add(1i32);
                                                        break 'l_exit_f;
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        {
                                            self.pop_lit2 = (self.pop_lit2).wrapping_neg();
                                            if (self.pop_lit1 > (self.sp_length).wrapping_sub((self.pop_lit2).wrapping_sub(1i32))) {
                                                self.pop_lit1 = (self.sp_length).wrapping_sub((self.pop_lit2).wrapping_sub(1i32));
                                            }
                                            self.sp_end = (self.str_start[((self.pop_lit3).wrapping_add(1i32)) as usize]).wrapping_sub((self.pop_lit2).wrapping_sub(1i32));
                                            self.sp_ptr = (self.sp_end).wrapping_sub(self.pop_lit1);
                                        }
                                    }
                                    {
                                        while (((self.pool_ptr).wrapping_add(self.sp_end)).wrapping_sub(self.sp_ptr) > self.pool_size) {
                                            self.pool_overflow();
                                        }
                                    }
                                    while (self.sp_ptr < self.sp_end) {
                                        {
                                            {
                                                { let __ix155 = self.pool_ptr; let __v156 = self.str_pool[(self.sp_ptr) as usize]; self.str_pool[(__ix155) as usize] = __v156; }
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                            self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                        }
                                    }
                                    { let __a157_0 = self.make_string(); let __a157_1 = stk_str; self.push_lit_stk(__a157_0, __a157_1) };
                                }
                            }
                        }
                    }
                }
            }
        }
        // §428
    }

    /// The `built_in` function {\.{swap\$}} pops the top two literals from
    /// the stack and pushes them back swapped.
    /// @<`execute_fn`({\.{swap\$}})
    // §430
    pub fn x_swap(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit2); let mut __f1 = ::core::mem::take(&mut self.pop_typ2); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit2 = __f0; self.pop_typ2 = __f1; __r };
        if ((self.pop_typ1 != stk_str) || (self.pop_lit1 < self.cmd_str_ptr)) {
            {
                self.push_lit_stk(self.pop_lit1, self.pop_typ1);
                if ((self.pop_typ2 == stk_str) && (self.pop_lit2 >= self.cmd_str_ptr)) {
                    {
                        self.str_ptr = (self.str_ptr).wrapping_add(1i32);
                        self.pool_ptr = self.str_start[(self.str_ptr) as usize];
                    }
                }
                self.push_lit_stk(self.pop_lit2, self.pop_typ2);
            }
        } else {
            if ((self.pop_typ2 != stk_str) || (self.pop_lit2 < self.cmd_str_ptr)) {
                {
                    {
                        self.str_ptr = (self.str_ptr).wrapping_add(1i32);
                        self.pool_ptr = self.str_start[(self.str_ptr) as usize];
                    }
                    self.push_lit_stk(self.pop_lit1, stk_str);
                    self.push_lit_stk(self.pop_lit2, self.pop_typ2);
                }
            } else {
                // §431
                {
                    self.ex_buf_length = 0i32;
                    self.add_buf_pool(self.pop_lit2);
                    self.sp_ptr = self.str_start[(self.pop_lit1) as usize];
                    self.sp_end = self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize];
                    while (self.sp_ptr < self.sp_end) {
                        {
                            {
                                { let __ix158 = self.pool_ptr; let __v159 = self.str_pool[(self.sp_ptr) as usize]; self.str_pool[(__ix158) as usize] = __v159; }
                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                            }
                            self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                        }
                    }
                    { let __a160_0 = self.make_string(); let __a160_1 = stk_str; self.push_lit_stk(__a160_0, __a160_1) };
                    self.add_pool_buf_and_push();
                }
            }
        }
    }

    /// The `built_in` function {\.{text.length\$}} pops the top (string)
    /// literal, and pushes the number of text characters it contains, where
    /// an accented character (more precisely, a ``special character''$\!$,
    /// defined earlier) counts as a single text character, even if it's
    /// missing its matching `right_brace`, and where braces don't count as
    /// text characters.  If the literal isn't a string, it complains and
    /// pushes the null string.
    /// @<`execute_fn`({\.{text.length\$}})
    // §432
    pub fn x_text_length(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        if (self.pop_typ1 != stk_str) {
            {
                self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_str);
                self.push_lit_stk(self.s_null, stk_str);
            }
        } else {
            {
                self.num_text_chars = 0i32;
                // §433
                {
                    self.sp_ptr = self.str_start[(self.pop_lit1) as usize];
                    self.sp_end = self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize];
                    self.sp_brace_level = 0i32;
                    while (self.sp_ptr < self.sp_end) {
                        {
                            self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                            if (self.str_pool[((self.sp_ptr).wrapping_sub(1i32)) as usize] == left_brace) {
                                {
                                    self.sp_brace_level = (self.sp_brace_level).wrapping_add(1i32);
                                    if ((self.sp_brace_level == 1i32) && (self.sp_ptr < self.sp_end)) {
                                        if (self.str_pool[(self.sp_ptr) as usize] == backslash) {
                                            {
                                                self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                                while ((self.sp_ptr < self.sp_end) && (self.sp_brace_level > 0i32)) {
                                                    {
                                                        if (self.str_pool[(self.sp_ptr) as usize] == right_brace) {
                                                            self.sp_brace_level = (self.sp_brace_level).wrapping_sub(1i32);
                                                        } else {
                                                            if (self.str_pool[(self.sp_ptr) as usize] == left_brace) {
                                                                self.sp_brace_level = (self.sp_brace_level).wrapping_add(1i32);
                                                            }
                                                        }
                                                        self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                                    }
                                                }
                                                self.num_text_chars = (self.num_text_chars).wrapping_add(1i32);
                                            }
                                        }
                                    }
                                }
                            } else {
                                if (self.str_pool[((self.sp_ptr).wrapping_sub(1i32)) as usize] == right_brace) {
                                    {
                                        if (self.sp_brace_level > 0i32) {
                                            self.sp_brace_level = (self.sp_brace_level).wrapping_sub(1i32);
                                        }
                                    }
                                } else {
                                    self.num_text_chars = (self.num_text_chars).wrapping_add(1i32);
                                }
                            }
                        }
                    }
                }
                // §432
                self.push_lit_stk(self.num_text_chars, stk_int);
            }
        }
    }

    /// The `built_in` function {\.{text.prefix\$}} pops the top two literals
    /// (the integer literal `pop_lit1` and a string literal, in that order).
    /// It pushes the substring of the (at most) `pop_lit1` consecutive text
    /// characters starting from the beginning of the string.  This function
    /// is similar to {\.{substring\$}}, but this one considers an accented
    /// character (or more precisely, a ``special character''$\!$, even if
    /// it's missing its matching `right_brace`) to be a single text character
    /// (rather than however many `ASCII_code` characters it actually
    /// comprises), and this function doesn't consider braces to be text
    /// characters; furthermore, this function appends any needed matching
    /// `right_brace`s.  If any of the types is incorrect, it complains and
    /// pushes the null string.
    /// @<`execute_fn`({\.{text.prefix\$}})
    // §434
    pub fn x_text_prefix(&mut self) {
        'l_exit_f: {
            { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
            { let mut __f0 = ::core::mem::take(&mut self.pop_lit2); let mut __f1 = ::core::mem::take(&mut self.pop_typ2); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit2 = __f0; self.pop_typ2 = __f1; __r };
            if (self.pop_typ1 != stk_int) {
                {
                    self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_int);
                    self.push_lit_stk(self.s_null, stk_str);
                }
            } else {
                if (self.pop_typ2 != stk_str) {
                    {
                        self.print_wrong_stk_lit(self.pop_lit2, self.pop_typ2, stk_str);
                        self.push_lit_stk(self.s_null, stk_str);
                    }
                } else {
                    if (self.pop_lit1 <= 0i32) {
                        {
                            self.push_lit_stk(self.s_null, stk_str);
                            break 'l_exit_f;
                        }
                    } else {
                        // §435
                        {
                            self.sp_ptr = self.str_start[(self.pop_lit2) as usize];
                            self.sp_end = self.str_start[((self.pop_lit2).wrapping_add(1i32)) as usize];
                            // §436
                            {
                                self.num_text_chars = 0i32;
                                self.sp_brace_level = 0i32;
                                self.sp_xptr1 = self.sp_ptr;
                                while ((self.sp_xptr1 < self.sp_end) && (self.num_text_chars < self.pop_lit1)) {
                                    {
                                        self.sp_xptr1 = (self.sp_xptr1).wrapping_add(1i32);
                                        if (self.str_pool[((self.sp_xptr1).wrapping_sub(1i32)) as usize] == left_brace) {
                                            {
                                                self.sp_brace_level = (self.sp_brace_level).wrapping_add(1i32);
                                                if ((self.sp_brace_level == 1i32) && (self.sp_xptr1 < self.sp_end)) {
                                                    if (self.str_pool[(self.sp_xptr1) as usize] == backslash) {
                                                        {
                                                            self.sp_xptr1 = (self.sp_xptr1).wrapping_add(1i32);
                                                            while ((self.sp_xptr1 < self.sp_end) && (self.sp_brace_level > 0i32)) {
                                                                {
                                                                    if (self.str_pool[(self.sp_xptr1) as usize] == right_brace) {
                                                                        self.sp_brace_level = (self.sp_brace_level).wrapping_sub(1i32);
                                                                    } else {
                                                                        if (self.str_pool[(self.sp_xptr1) as usize] == left_brace) {
                                                                            self.sp_brace_level = (self.sp_brace_level).wrapping_add(1i32);
                                                                        }
                                                                    }
                                                                    self.sp_xptr1 = (self.sp_xptr1).wrapping_add(1i32);
                                                                }
                                                            }
                                                            self.num_text_chars = (self.num_text_chars).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            if (self.str_pool[((self.sp_xptr1).wrapping_sub(1i32)) as usize] == right_brace) {
                                                {
                                                    if (self.sp_brace_level > 0i32) {
                                                        self.sp_brace_level = (self.sp_brace_level).wrapping_sub(1i32);
                                                    }
                                                }
                                            } else {
                                                self.num_text_chars = (self.num_text_chars).wrapping_add(1i32);
                                            }
                                        }
                                    }
                                }
                                self.sp_end = self.sp_xptr1;
                            }
                            // §435
                            {
                                while ((((self.pool_ptr).wrapping_add(self.sp_brace_level)).wrapping_add(self.sp_end)).wrapping_sub(self.sp_ptr) > self.pool_size) {
                                    self.pool_overflow();
                                }
                            }
                            if (self.pop_lit2 >= self.cmd_str_ptr) {
                                self.pool_ptr = self.sp_end;
                            } else {
                                while (self.sp_ptr < self.sp_end) {
                                    {
                                        {
                                            { let __ix161 = self.pool_ptr; let __v162 = self.str_pool[(self.sp_ptr) as usize]; self.str_pool[(__ix161) as usize] = __v162; }
                                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                        }
                                        self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                    }
                                }
                            }
                            while (self.sp_brace_level > 0i32) {
                                {
                                    {
                                        self.str_pool[(self.pool_ptr) as usize] = right_brace;
                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                    }
                                    self.sp_brace_level = (self.sp_brace_level).wrapping_sub(1i32);
                                }
                            }
                            { let __a163_0 = self.make_string(); let __a163_1 = stk_str; self.push_lit_stk(__a163_0, __a163_1) };
                        }
                    }
                }
            }
        }
        // §434
    }

    /// The `built_in` function {\.{type\$}} pushes the appropriate string
    /// from `type_list` onto the stack (unless either it's `undefined` or
    /// `empty`, in which case it pushes the null string).
    /// @<`execute_fn`({\.{type\$}})
    // §438
    pub fn x_type(&mut self) {
        if (!self.mess_with_entries) {
            self.bst_cant_mess_with_entries_print();
        } else {
            if ((self.type_list[(self.cite_ptr) as usize] == self.undefined) || (self.type_list[(self.cite_ptr) as usize] == empty)) {
                self.push_lit_stk(self.s_null, stk_str);
            } else {
                self.push_lit_stk(self.hash_text[(self.type_list[(self.cite_ptr) as usize]) as usize], stk_str);
            }
        }
    }

    /// The `built_in` function {\.{warning\$}} pops the top (string) literal
    /// and prints it following a warning message.  This is implemented as a
    /// special `built_in` function rather than using the {\.{top\$}} function
    /// so that it can `mark_warning`.
    /// @<`execute_fn`({\.{warning\$}})
    // §439
    pub fn x_warning(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        if (self.pop_typ1 != stk_str) {
            self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_str);
        } else {
            {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "Warning--");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "Warning--");
                    }
                }
                self.print_lit(self.pop_lit1, self.pop_typ1);
                self.mark_warning();
            }
        }
    }

    /// The `built_in` function {\.{width\$}} pops the top (string) literal
    /// and pushes the integer that represents its width in units specified by
    /// the `char_width` array.  This function takes the literal literally;
    /// that is, it assumes each character in the string is to be printed as
    /// is, regardless of whether the character has a special meaning to \TeX,
    /// except that special characters (even without their `right_brace`s) are
    /// handled specially.  If the literal isn't a string, it complains and
    /// pushes~0.
    /// @<`execute_fn`({\.{width\$}})
    // §441
    pub fn x_width(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        if (self.pop_typ1 != stk_str) {
            {
                self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_str);
                self.push_lit_stk(0i32, stk_int);
            }
        } else {
            {
                self.ex_buf_length = 0i32;
                self.add_buf_pool(self.pop_lit1);
                self.string_width = 0i32;
                // §442
                {
                    self.brace_level = 0i32;
                    self.ex_buf_ptr = 0i32;
                    while (self.ex_buf_ptr < self.ex_buf_length) {
                        {
                            if (self.ex_buf[(self.ex_buf_ptr) as usize] == left_brace) {
                                {
                                    self.brace_level = (self.brace_level).wrapping_add(1i32);
                                    if ((self.brace_level == 1i32) && ((self.ex_buf_ptr).wrapping_add(1i32) < self.ex_buf_length)) {
                                        if (self.ex_buf[((self.ex_buf_ptr).wrapping_add(1i32)) as usize] == backslash) {
                                            // §443
                                            {
                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                while ((self.ex_buf_ptr < self.ex_buf_length) && (self.brace_level > 0i32)) {
                                                    {
                                                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                        self.ex_buf_xptr = self.ex_buf_ptr;
                                                        while ((self.ex_buf_ptr < self.ex_buf_length) && (self.lex_class[(self.ex_buf[(self.ex_buf_ptr) as usize]) as usize] == alpha)) {
                                                            self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                        }
                                                        if ((self.ex_buf_ptr < self.ex_buf_length) && (self.ex_buf_ptr == self.ex_buf_xptr)) {
                                                            self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                        } else {
                                                            {
                                                                self.control_seq_loc = { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.str_lookup(&mut __f0, self.ex_buf_xptr, (self.ex_buf_ptr).wrapping_sub(self.ex_buf_xptr), control_seq_ilk, false); self.ex_buf = __f0; __r };
                                                                if self.hash_found {
                                                                    // §444
                                                                    {
                                                                        match self.ilk_info[(self.control_seq_loc) as usize] {
                                                                            n_ss => {
                                                                                self.string_width = (self.string_width).wrapping_add(500i32);
                                                                            }
                                                                            n_ae => {
                                                                                self.string_width = (self.string_width).wrapping_add(722i32);
                                                                            }
                                                                            n_oe => {
                                                                                self.string_width = (self.string_width).wrapping_add(778i32);
                                                                            }
                                                                            n_ae_upper => {
                                                                                self.string_width = (self.string_width).wrapping_add(903i32);
                                                                            }
                                                                            n_oe_upper => {
                                                                                self.string_width = (self.string_width).wrapping_add(1014i32);
                                                                            }
                                                                            _ => {
                                                                                self.string_width = (self.string_width).wrapping_add(self.char_width[(self.ex_buf[(self.ex_buf_xptr) as usize]) as usize]);
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        // §443
                                                        while ((self.ex_buf_ptr < self.ex_buf_length) && (self.lex_class[(self.ex_buf[(self.ex_buf_ptr) as usize]) as usize] == white_space)) {
                                                            self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                        }
                                                        while (((self.ex_buf_ptr < self.ex_buf_length) && (self.brace_level > 0i32)) && (self.ex_buf[(self.ex_buf_ptr) as usize] != backslash)) {
                                                            {
                                                                if (self.ex_buf[(self.ex_buf_ptr) as usize] == right_brace) {
                                                                    self.brace_level = (self.brace_level).wrapping_sub(1i32);
                                                                } else {
                                                                    if (self.ex_buf[(self.ex_buf_ptr) as usize] == left_brace) {
                                                                        self.brace_level = (self.brace_level).wrapping_add(1i32);
                                                                    } else {
                                                                        self.string_width = (self.string_width).wrapping_add(self.char_width[(self.ex_buf[(self.ex_buf_ptr) as usize]) as usize]);
                                                                    }
                                                                }
                                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                            }
                                                        }
                                                    }
                                                }
                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_sub(1i32);
                                            }
                                        } else {
                                            // §442
                                            self.string_width = (self.string_width).wrapping_add(self.char_width[(left_brace) as usize]);
                                        }
                                    } else {
                                        self.string_width = (self.string_width).wrapping_add(self.char_width[(left_brace) as usize]);
                                    }
                                }
                            } else {
                                if (self.ex_buf[(self.ex_buf_ptr) as usize] == right_brace) {
                                    {
                                        self.decr_brace_level(self.pop_lit1);
                                        self.string_width = (self.string_width).wrapping_add(self.char_width[(right_brace) as usize]);
                                    }
                                } else {
                                    self.string_width = (self.string_width).wrapping_add(self.char_width[(self.ex_buf[(self.ex_buf_ptr) as usize]) as usize]);
                                }
                            }
                            self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                        }
                    }
                    self.check_brace_level(self.pop_lit1);
                }
                // §441
                self.push_lit_stk(self.string_width, stk_int);
            }
        }
    }

    /// The `built_in` function {\.{write\$}} pops the top (string) literal
    /// and writes it onto the output buffer `out_buf` (which will result in
    /// stuff being written onto the \.{.bbl} file if the buffer fills up).  If
    /// the literal isn't a string, it complains but does nothing else.
    /// @<`execute_fn`({\.{write\$}})
    // §445
    pub fn x_write(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        if (self.pop_typ1 != stk_str) {
            self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_str);
        } else {
            self.add_out_pool(self.pop_lit1);
        }
    }

    /// This procedure executes a single specified function; it is the single
    /// execution-primitive that does everything (except windows, and it takes
    /// Tuesdays off).
    /// @<`execute_fn` itself
    // §316
    pub fn execute_fn(&mut self, mut ex_fn_loc: hash_loc) {
        let mut r_pop_lt1: i32 = 0; // §334
        let mut r_pop_lt2: i32 = 0; // §334
        let mut r_pop_tp1: stk_type = 0; // §334
        let mut r_pop_tp2: stk_type = 0; // §334
        let mut wiz_ptr: wiz_fn_loc = 0; // §316
        self.check_deadline();
        match self.fn_type[(ex_fn_loc) as usize] {
            built_in => {
                // §332
                {
                    { let __ix164 = self.ilk_info[(ex_fn_loc) as usize]; let __v165 = (self.execution_count[(self.ilk_info[(ex_fn_loc) as usize]) as usize]).wrapping_add(1i32); self.execution_count[(__ix164) as usize] = __v165; }
                    match self.ilk_info[(ex_fn_loc) as usize] {
                        n_equals => {
                            self.x_equals();
                        }
                        n_greater_than => {
                            self.x_greater_than();
                        }
                        n_less_than => {
                            self.x_less_than();
                        }
                        n_plus => {
                            self.x_plus();
                        }
                        n_minus => {
                            self.x_minus();
                        }
                        n_concatenate => {
                            self.x_concatenate();
                        }
                        n_gets => {
                            self.x_gets();
                        }
                        n_add_period => {
                            self.x_add_period();
                        }
                        n_call_type => {
                            // §354
                            {
                                if (!self.mess_with_entries) {
                                    self.bst_cant_mess_with_entries_print();
                                } else {
                                    if (self.type_list[(self.cite_ptr) as usize] == self.undefined) {
                                        self.execute_fn(self.b_default);
                                    } else {
                                        if (self.type_list[(self.cite_ptr) as usize] == empty) {
                                        } else {
                                            self.execute_fn(self.type_list[(self.cite_ptr) as usize]);
                                        }
                                    }
                                }
                            }
                        }
                        n_change_case => {
                            // §332
                            self.x_change_case();
                        }
                        n_chr_to_int => {
                            self.x_chr_to_int();
                        }
                        n_cite => {
                            self.x_cite();
                        }
                        n_duplicate => {
                            self.x_duplicate();
                        }
                        n_empty => {
                            self.x_empty();
                        }
                        n_format_name => {
                            self.x_format_name();
                        }
                        n_if => {
                            // §412
                            {
                                { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
                                { let mut __f0 = ::core::mem::take(&mut self.pop_lit2); let mut __f1 = ::core::mem::take(&mut self.pop_typ2); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit2 = __f0; self.pop_typ2 = __f1; __r };
                                { let mut __f0 = ::core::mem::take(&mut self.pop_lit3); let mut __f1 = ::core::mem::take(&mut self.pop_typ3); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit3 = __f0; self.pop_typ3 = __f1; __r };
                                if (self.pop_typ1 != stk_fn) {
                                    self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_fn);
                                } else {
                                    if (self.pop_typ2 != stk_fn) {
                                        self.print_wrong_stk_lit(self.pop_lit2, self.pop_typ2, stk_fn);
                                    } else {
                                        if (self.pop_typ3 != stk_int) {
                                            self.print_wrong_stk_lit(self.pop_lit3, self.pop_typ3, stk_int);
                                        } else {
                                            if (self.pop_lit3 > 0i32) {
                                                self.execute_fn(self.pop_lit2);
                                            } else {
                                                self.execute_fn(self.pop_lit1);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        n_int_to_chr => {
                            // §332
                            self.x_int_to_chr();
                        }
                        n_int_to_str => {
                            self.x_int_to_str();
                        }
                        n_missing => {
                            self.x_missing();
                        }
                        n_newline => {
                            // §416
                            {
                                self.output_bbl_line();
                            }
                        }
                        n_num_names => {
                            // §332
                            self.x_num_names();
                        }
                        n_pop => {
                            // §419
                            {
                                { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
                            }
                        }
                        n_preamble => {
                            // §332
                            self.x_preamble();
                        }
                        n_purify => {
                            self.x_purify();
                        }
                        n_quote => {
                            self.x_quote();
                        }
                        n_skip => {
                            // §426
                            {
                            }
                        }
                        n_stack => {
                            // §427
                            {
                                self.pop_whole_stack();
                            }
                        }
                        n_substring => {
                            // §332
                            self.x_substring();
                        }
                        n_swap => {
                            self.x_swap();
                        }
                        n_text_length => {
                            self.x_text_length();
                        }
                        n_text_prefix => {
                            self.x_text_prefix();
                        }
                        n_top_stack => {
                            // §437
                            {
                                self.pop_top_and_print();
                            }
                        }
                        n_type => {
                            // §332
                            self.x_type();
                        }
                        n_warning => {
                            self.x_warning();
                        }
                        n_while => {
                            // §440
                            {
                                'l_L51_f: {
                                    { let mut __f0 = ::core::mem::take(&mut r_pop_lt1); let mut __f1 = ::core::mem::take(&mut r_pop_tp1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); r_pop_lt1 = __f0; r_pop_tp1 = __f1; __r };
                                    { let mut __f0 = ::core::mem::take(&mut r_pop_lt2); let mut __f1 = ::core::mem::take(&mut r_pop_tp2); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); r_pop_lt2 = __f0; r_pop_tp2 = __f1; __r };
                                    if (r_pop_tp1 != stk_fn) {
                                        self.print_wrong_stk_lit(r_pop_lt1, r_pop_tp1, stk_fn);
                                    } else {
                                        if (r_pop_tp2 != stk_fn) {
                                            self.print_wrong_stk_lit(r_pop_lt2, r_pop_tp2, stk_fn);
                                        } else {
                                            while true {
                                                {
                                                    self.execute_fn(r_pop_lt2);
                                                    { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
                                                    if (self.pop_typ1 != stk_int) {
                                                        {
                                                            self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_int);
                                                            break 'l_L51_f;
                                                        }
                                                    } else {
                                                        if (self.pop_lit1 > 0i32) {
                                                            self.execute_fn(r_pop_lt1);
                                                        } else {
                                                            break 'l_L51_f;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        n_width => {
                            // §332
                            self.x_width();
                        }
                        n_write => {
                            self.x_write();
                        }
                        _ => {
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "Unknown built-in function");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "Unknown built-in function");
                                    }
                                }
                                self.print_confusion();
                                crate::system::end_of_TEX(self);
                            }
                        }
                    }
                }
            }
            wiz_defined => {
                // §317
                {
                    wiz_ptr = self.ilk_info[(ex_fn_loc) as usize];
                    while (self.wiz_functions[(wiz_ptr) as usize] != self.end_of_def) {
                        {
                            if (self.wiz_functions[(wiz_ptr) as usize] != quote_next_fn) {
                                self.execute_fn(self.wiz_functions[(wiz_ptr) as usize]);
                            } else {
                                {
                                    wiz_ptr = (wiz_ptr).wrapping_add(1i32);
                                    self.push_lit_stk(self.wiz_functions[(wiz_ptr) as usize], stk_fn);
                                }
                            }
                            wiz_ptr = (wiz_ptr).wrapping_add(1i32);
                        }
                    }
                }
            }
            int_literal => {
                // §316
                self.push_lit_stk(self.ilk_info[(ex_fn_loc) as usize], stk_int);
            }
            str_literal => {
                self.push_lit_stk(self.hash_text[(ex_fn_loc) as usize], stk_str);
            }
            field => {
                // §318
                {
                    if (!self.mess_with_entries) {
                        self.bst_cant_mess_with_entries_print();
                    } else {
                        {
                            self.field_ptr = ((self.cite_ptr).wrapping_mul(self.num_fields)).wrapping_add(self.ilk_info[(ex_fn_loc) as usize]);
                            if (self.field_ptr >= self.max_fields) {
                                {
                                    {
                                        {
                                            crate::system::wr_str(&mut self.log_file, "field_info index is out of range");
                                        }
                                        {
                                            crate::system::wr_str(&mut self.standard_output, "field_info index is out of range");
                                        }
                                    }
                                    self.print_confusion();
                                    crate::system::end_of_TEX(self);
                                }
                            }
                            if (self.field_info[(self.field_ptr) as usize] == missing) {
                                self.push_lit_stk(self.hash_text[(ex_fn_loc) as usize], stk_field_missing);
                            } else {
                                self.push_lit_stk(self.field_info[(self.field_ptr) as usize], stk_str);
                            }
                        }
                    }
                }
            }
            int_entry_var => {
                // §319
                {
                    if (!self.mess_with_entries) {
                        self.bst_cant_mess_with_entries_print();
                    } else {
                        self.push_lit_stk(self.entry_ints[(((self.cite_ptr).wrapping_mul(self.num_ent_ints)).wrapping_add(self.ilk_info[(ex_fn_loc) as usize])) as usize], stk_int);
                    }
                }
            }
            str_entry_var => {
                // §320
                {
                    if (!self.mess_with_entries) {
                        self.bst_cant_mess_with_entries_print();
                    } else {
                        {
                            self.str_ent_ptr = ((self.cite_ptr).wrapping_mul(self.num_ent_strs)).wrapping_add(self.ilk_info[(ex_fn_loc) as usize]);
                            self.ex_buf_ptr = 0i32;
                            while (self.entry_strs[(((self.str_ent_ptr).wrapping_mul((self.ent_str_size).wrapping_add(1i32))).wrapping_add(self.ex_buf_ptr)) as usize] != end_of_string) {
                                {
                                    { let __ix166 = self.ex_buf_ptr; let __v167 = self.entry_strs[(((self.str_ent_ptr).wrapping_mul((self.ent_str_size).wrapping_add(1i32))).wrapping_add(self.ex_buf_ptr)) as usize]; self.ex_buf[(__ix166) as usize] = __v167; }
                                    self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                }
                            }
                            self.ex_buf_length = self.ex_buf_ptr;
                            self.add_pool_buf_and_push();
                        }
                    }
                }
            }
            int_global_var => {
                // §316
                self.push_lit_stk(self.ilk_info[(ex_fn_loc) as usize], stk_int);
            }
            str_global_var => {
                // §321
                {
                    self.str_glb_ptr = self.ilk_info[(ex_fn_loc) as usize];
                    if (self.glb_str_ptr[(self.str_glb_ptr) as usize] > 0i32) {
                        self.push_lit_stk(self.glb_str_ptr[(self.str_glb_ptr) as usize], stk_str);
                    } else {
                        {
                            {
                                while ((self.pool_ptr).wrapping_add(self.glb_str_end[(self.str_glb_ptr) as usize]) > self.pool_size) {
                                    self.pool_overflow();
                                }
                            }
                            self.glob_chr_ptr = 0i32;
                            while (self.glob_chr_ptr < self.glb_str_end[(self.str_glb_ptr) as usize]) {
                                {
                                    {
                                        { let __ix168 = self.pool_ptr; let __v169 = self.global_strs[(((self.str_glb_ptr).wrapping_mul((self.glob_str_size).wrapping_add(1i32))).wrapping_add(self.glob_chr_ptr)) as usize]; self.str_pool[(__ix168) as usize] = __v169; }
                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                    }
                                    self.glob_chr_ptr = (self.glob_chr_ptr).wrapping_add(1i32);
                                }
                            }
                            { let __a170_0 = self.make_string(); let __a170_1 = stk_str; self.push_lit_stk(__a170_0, __a170_1) };
                        }
                    }
                }
            }
            _ => {
                // §316
                self.unknwn_function_class_confusion();
            }
        }
    }

    /// This module and the next two must be changed on those systems using
    /// command-line arguments.
    /// @<Procedures and functions for the reading and processing of input files
    // §93
    pub fn get_the_top_level_aux_file_name(&mut self) {
        'l_L41_f: {
            'l_L46_f: {
                self.set_aux_name_from_command_line();
                // §95
                {
                    if ((((self.aux_name_length).wrapping_add((self.str_start[((self.s_aux_extension).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.s_aux_extension) as usize])) > file_name_size) || ((self.aux_name_length).wrapping_add((self.str_start[((self.s_log_extension).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.s_log_extension) as usize])) > file_name_size)) || ((self.aux_name_length).wrapping_add((self.str_start[((self.s_bbl_extension).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.s_bbl_extension) as usize])) > file_name_size)) {
                        {
                            self.sam_too_long_file_name_print();
                            break 'l_L46_f;
                        }
                    }
                    // §98
                    {
                        self.name_length = self.aux_name_length;
                        if (((((self.name_length < 4i32) || (self.name_of_file[((self.name_length).wrapping_sub(3i32)) as usize] != b'.')) || (self.name_of_file[((self.name_length).wrapping_sub(2i32)) as usize] != b'a')) || (self.name_of_file[((self.name_length).wrapping_sub(1i32)) as usize] != b'u')) || (self.name_of_file[(self.name_length) as usize] != b'x')) {
                            self.add_extension(self.s_aux_extension);
                        } else {
                            self.aux_name_length = (self.aux_name_length).wrapping_sub(4i32);
                        }
                        self.aux_ptr = 0i32;
                        if ((!self.name_in_ok()) || (!{ let mut __f0 = ::core::mem::take(&mut self.aux_file[(self.aux_ptr) as usize]); let __r = self.a_open_in(&mut __f0, (1i32).wrapping_neg()); self.aux_file[(self.aux_ptr) as usize] = __f0; __r })) {
                            {
                                self.sam_wrong_file_name_print();
                                break 'l_L46_f;
                            }
                        }
                        self.name_length = self.aux_name_length;
                        self.add_extension(self.s_log_extension);
                        if ((!self.name_out_ok()) || (!{ let mut __f0 = ::core::mem::take(&mut self.log_file); let __r = self.a_open_out(&mut __f0); self.log_file = __f0; __r })) {
                            {
                                self.sam_wrong_file_name_print();
                                break 'l_L46_f;
                            }
                        }
                        self.name_length = self.aux_name_length;
                        self.add_extension(self.s_bbl_extension);
                        if ((!self.name_out_ok()) || (!{ let mut __f0 = ::core::mem::take(&mut self.bbl_file); let __r = self.a_open_out(&mut __f0); self.bbl_file = __f0; __r })) {
                            {
                                self.sam_wrong_file_name_print();
                                break 'l_L46_f;
                            }
                        }
                    }
                    // §99
                    {
                        self.name_length = self.aux_name_length;
                        self.add_extension(self.s_aux_extension);
                        self.name_ptr = 1i32;
                        while (self.name_ptr <= self.name_length) {
                            {
                                { let __ix171 = self.name_ptr; let __v172 = self.xord[(self.name_of_file[(self.name_ptr) as usize]) as usize]; self.buffer[(__ix171) as usize] = __v172; }
                                self.name_ptr = (self.name_ptr).wrapping_add(1i32);
                            }
                        }
                        self.top_lev_str = { let __s173 = ({ let mut __f0 = ::core::mem::take(&mut self.buffer); let __r = self.str_lookup(&mut __f0, 1i32, self.aux_name_length, text_ilk, true); self.buffer = __f0; __r }) as usize; self.hash_text[__s173] };
                        { let __ix174 = self.aux_ptr; let __v175 = { let __s176 = ({ let mut __f0 = ::core::mem::take(&mut self.buffer); let __r = self.str_lookup(&mut __f0, 1i32, self.name_length, aux_file_ilk, true); self.buffer = __f0; __r }) as usize; self.hash_text[__s176] }; self.aux_list[(__ix174) as usize] = __v175; }
                        if self.hash_found {
                            {
                                {
                                    {
                                        {
                                            crate::system::wr_str(&mut self.log_file, "Already encountered auxiliary file");
                                        }
                                        {
                                            crate::system::wr_str(&mut self.standard_output, "Already encountered auxiliary file");
                                        }
                                    }
                                    self.print_confusion();
                                    crate::system::end_of_TEX(self);
                                }
                            }
                        }
                        self.aux_ln_stack[(self.aux_ptr) as usize] = 0i32;
                    }
                    // §95
                    break 'l_L41_f;
                }
            }
            // §93
            self.uexit(1i32);
        }
    }

    /// A \.{\\bibdata} command will have its arguments between braces and
    /// separated by commas.  There must be exactly one such command in the
    /// \.{.aux} file(s).  All upper-case letters are converted to lower case.
    /// @<Procedures and functions for the reading and processing of input files
    // §112
    pub fn aux_bib_data_command(&mut self) {
        'l_exit_f: {
            if self.bib_seen {
                {
                    self.aux_err_illegal_another_print(n_aux_bibdata);
                    {
                        self.aux_err_print();
                        break 'l_exit_f;
                    }
                }
            }
            self.bib_seen = true;
            while (self.buffer[(self.buf_ptr2) as usize] != right_brace) {
                {
                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                    if (!self.scan2_white(right_brace, comma)) {
                        {
                            self.aux_err_no_right_brace_print();
                            {
                                self.aux_err_print();
                                break 'l_exit_f;
                            }
                        }
                    }
                    if (self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] == white_space) {
                        {
                            self.aux_err_white_space_in_argument_print();
                            {
                                self.aux_err_print();
                                break 'l_exit_f;
                            }
                        }
                    }
                    if ((self.last > (self.buf_ptr2).wrapping_add(1i32)) && (self.buffer[(self.buf_ptr2) as usize] == right_brace)) {
                        {
                            self.aux_err_stuff_after_right_brace_print();
                            {
                                self.aux_err_print();
                                break 'l_exit_f;
                            }
                        }
                    }
                    // §115
                    {
                        if (self.bib_ptr == self.max_bib_files) {
                            {
                                self.log_realloc("bib_list", 4i32, (self.max_bib_files).wrapping_add(MAX_BIB_FILES), self.max_bib_files);
                                self.bib_list.resize_len((((self.max_bib_files).wrapping_add(MAX_BIB_FILES)) as usize) + 1);
                                self.log_realloc("bib_file", 8i32, (self.max_bib_files).wrapping_add(MAX_BIB_FILES), self.max_bib_files);
                                { let __n177 = (((self.max_bib_files).wrapping_add(MAX_BIB_FILES)) as usize) + 1; self.bib_file.resize(__n177, Default::default()); }
                                self.log_realloc("s_preamble", 4i32, (self.max_bib_files).wrapping_add(MAX_BIB_FILES), self.max_bib_files);
                                self.s_preamble.resize_len((((self.max_bib_files).wrapping_add(MAX_BIB_FILES)) as usize) + 1);
                                self.max_bib_files = (self.max_bib_files).wrapping_add(MAX_BIB_FILES);
                            }
                        }
                        { let __ix178 = self.bib_ptr; let __v179 = { let __s180 = ({ let mut __f0 = ::core::mem::take(&mut self.buffer); let __r = self.str_lookup(&mut __f0, self.buf_ptr1, (self.buf_ptr2).wrapping_sub(self.buf_ptr1), bib_file_ilk, true); self.buffer = __f0; __r }) as usize; self.hash_text[__s180] }; self.bib_list[(__ix178) as usize] = __v179; }
                        if self.hash_found {
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "This database file appears more than once: ");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "This database file appears more than once: ");
                                    }
                                }
                                self.print_bib_name();
                                {
                                    self.aux_err_print();
                                    break 'l_exit_f;
                                }
                            }
                        }
                        self.start_name(self.bib_list[(self.bib_ptr) as usize]);
                        if ((!self.name_in_ok()) || (!{ let mut __f0 = ::core::mem::take(&mut self.bib_file[(self.bib_ptr) as usize]); let __r = self.a_open_in(&mut __f0, kpse_bib_format); self.bib_file[(self.bib_ptr) as usize] = __f0; __r })) {
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "I couldn't open database file ");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "I couldn't open database file ");
                                    }
                                }
                                self.print_bib_name();
                                {
                                    self.aux_err_print();
                                    break 'l_exit_f;
                                }
                            }
                        }
                        self.bib_ptr = (self.bib_ptr).wrapping_add(1i32);
                    }
                }
            }
        }
        // §112
    }

    /// A \.{\\bibstyle} command will have exactly one argument, and it will
    /// be between braces.  There must be exactly one such command in the
    /// \.{.aux} file(s).  All upper-case letters are converted to lower case.
    /// @<Procedures and functions for the reading and processing of input files
    // §118
    pub fn aux_bib_style_command(&mut self) {
        'l_exit_f: {
            if self.bst_seen {
                {
                    self.aux_err_illegal_another_print(n_aux_bibstyle);
                    {
                        self.aux_err_print();
                        break 'l_exit_f;
                    }
                }
            }
            self.bst_seen = true;
            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            if (!self.scan1_white(right_brace)) {
                {
                    self.aux_err_no_right_brace_print();
                    {
                        self.aux_err_print();
                        break 'l_exit_f;
                    }
                }
            }
            if (self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] == white_space) {
                {
                    self.aux_err_white_space_in_argument_print();
                    {
                        self.aux_err_print();
                        break 'l_exit_f;
                    }
                }
            }
            if (self.last > (self.buf_ptr2).wrapping_add(1i32)) {
                {
                    self.aux_err_stuff_after_right_brace_print();
                    {
                        self.aux_err_print();
                        break 'l_exit_f;
                    }
                }
            }
            // §119
            {
                self.bst_str = { let __s181 = ({ let mut __f0 = ::core::mem::take(&mut self.buffer); let __r = self.str_lookup(&mut __f0, self.buf_ptr1, (self.buf_ptr2).wrapping_sub(self.buf_ptr1), bst_file_ilk, true); self.buffer = __f0; __r }) as usize; self.hash_text[__s181] };
                if self.hash_found {
                    {
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "Already encountered style file");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "Already encountered style file");
                                }
                            }
                            self.print_confusion();
                            crate::system::end_of_TEX(self);
                        }
                    }
                }
                self.start_name(self.bst_str);
                if ((!self.name_in_ok()) || (!{ let mut __f0 = ::core::mem::take(&mut self.bst_file); let __r = self.a_open_in(&mut __f0, kpse_bst_format); self.bst_file = __f0; __r })) {
                    {
                        {
                            {
                                crate::system::wr_str(&mut self.log_file, "I couldn't open style file ");
                            }
                            {
                                crate::system::wr_str(&mut self.standard_output, "I couldn't open style file ");
                            }
                        }
                        self.print_bst_name();
                        self.bst_str = 0i32;
                        {
                            self.aux_err_print();
                            break 'l_exit_f;
                        }
                    }
                }
                if self.verbose {
                    {
                        {
                            {
                                crate::system::wr_str(&mut self.log_file, "The style file: ");
                            }
                            {
                                crate::system::wr_str(&mut self.standard_output, "The style file: ");
                            }
                        }
                        self.print_bst_name();
                    }
                } else {
                    {
                        {
                            {
                                crate::system::wr_str(&mut self.log_file, "The style file: ");
                            }
                        }
                        self.log_pr_bst_name();
                    }
                }
            }
        }
        // §118
    }

    /// A \.{\\citation} command will have its arguments between braces and
    /// separated by commas.  Upper/lower cases are considered to be different
    /// for \.{\\citation} arguments, which is the same as the rest of \LaTeX\
    /// but different from the rest of \BibTeX.  A cite key needn't exactly
    /// case-match its corresponding database key to work, although two cite
    /// keys that are case-mismatched will produce an error message.
    /// (A {\sl case mismatch\/} is a mismatch, but only because of a case
    /// difference.)
    /// A \.{\\citation} command having \.{*} as an argument indicates that
    /// the entire database will be included (almost as if a \.{\\nocite}
    /// command that listed every cite key in the database, in order, had been
    /// given at the corresponding spot in the \.{.tex} file).
    // §124
    pub fn aux_citation_command(&mut self) {
        'l_exit_f: {
            self.citation_seen = true;
            while (self.buffer[(self.buf_ptr2) as usize] != right_brace) {
                {
                    'l_L23_f: {
                        self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                        if (!self.scan2_white(right_brace, comma)) {
                            {
                                self.aux_err_no_right_brace_print();
                                {
                                    self.aux_err_print();
                                    break 'l_exit_f;
                                }
                            }
                        }
                        if (self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] == white_space) {
                            {
                                self.aux_err_white_space_in_argument_print();
                                {
                                    self.aux_err_print();
                                    break 'l_exit_f;
                                }
                            }
                        }
                        if ((self.last > (self.buf_ptr2).wrapping_add(1i32)) && (self.buffer[(self.buf_ptr2) as usize] == right_brace)) {
                            {
                                self.aux_err_stuff_after_right_brace_print();
                                {
                                    self.aux_err_print();
                                    break 'l_exit_f;
                                }
                            }
                        }
                        // §125
                        {
                            // §126
                            {
                                if ((self.buf_ptr2).wrapping_sub(self.buf_ptr1) == 1i32) {
                                    if (self.buffer[(self.buf_ptr1) as usize] == star) {
                                        {
                                            if self.all_entries {
                                                {
                                                    {
                                                        {
                                                            crate::system::wr_str(&mut self.log_file, "Multiple inclusions of entire database");
                                                            crate::system::wr_ln(&mut self.log_file);
                                                        }
                                                        {
                                                            crate::system::wr_str(&mut self.standard_output, "Multiple inclusions of entire database");
                                                            crate::system::wr_ln(&mut self.standard_output);
                                                        }
                                                    }
                                                    {
                                                        self.aux_err_print();
                                                        break 'l_exit_f;
                                                    }
                                                }
                                            } else {
                                                {
                                                    self.all_entries = true;
                                                    self.all_marker = self.cite_ptr;
                                                    break 'l_L23_f;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // §125
                            self.tmp_ptr = self.buf_ptr1;
                            while (self.tmp_ptr < self.buf_ptr2) {
                                {
                                    { let __ix182 = self.tmp_ptr; let __v183 = self.buffer[(self.tmp_ptr) as usize]; self.ex_buf[(__ix182) as usize] = __v183; }
                                    self.tmp_ptr = (self.tmp_ptr).wrapping_add(1i32);
                                }
                            }
                            { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.lower_case(&mut __f0, self.buf_ptr1, (self.buf_ptr2).wrapping_sub(self.buf_ptr1)); self.ex_buf = __f0; __r };
                            self.lc_cite_loc = { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.str_lookup(&mut __f0, self.buf_ptr1, (self.buf_ptr2).wrapping_sub(self.buf_ptr1), lc_cite_ilk, true); self.ex_buf = __f0; __r };
                            if self.hash_found {
                                // §127
                                {
                                    self.dummy_loc = { let mut __f0 = ::core::mem::take(&mut self.buffer); let __r = self.str_lookup(&mut __f0, self.buf_ptr1, (self.buf_ptr2).wrapping_sub(self.buf_ptr1), cite_ilk, false); self.buffer = __f0; __r };
                                    if (!self.hash_found) {
                                        {
                                            {
                                                {
                                                    crate::system::wr_str(&mut self.log_file, "Case mismatch error between cite keys ");
                                                }
                                                {
                                                    crate::system::wr_str(&mut self.standard_output, "Case mismatch error between cite keys ");
                                                }
                                            }
                                            self.print_a_token();
                                            {
                                                {
                                                    crate::system::wr_str(&mut self.log_file, " and ");
                                                }
                                                {
                                                    crate::system::wr_str(&mut self.standard_output, " and ");
                                                }
                                            }
                                            self.print_a_pool_str(self.cite_list[(self.ilk_info[(self.ilk_info[(self.lc_cite_loc) as usize]) as usize]) as usize]);
                                            self.print_a_newline();
                                            {
                                                self.aux_err_print();
                                                break 'l_exit_f;
                                            }
                                        }
                                    }
                                }
                            } else {
                                // §128
                                {
                                    self.cite_loc = { let mut __f0 = ::core::mem::take(&mut self.buffer); let __r = self.str_lookup(&mut __f0, self.buf_ptr1, (self.buf_ptr2).wrapping_sub(self.buf_ptr1), cite_ilk, true); self.buffer = __f0; __r };
                                    if self.hash_found {
                                        self.hash_cite_confusion();
                                    }
                                    self.check_cite_overflow(self.cite_ptr);
                                    { let __ix184 = self.cite_ptr; let __v185 = self.hash_text[(self.cite_loc) as usize]; self.cite_list[(__ix184) as usize] = __v185; }
                                    { let __ix186 = self.cite_loc; let __v187 = self.cite_ptr; self.ilk_info[(__ix186) as usize] = __v187; }
                                    { let __ix188 = self.lc_cite_loc; let __v189 = self.cite_loc; self.ilk_info[(__ix188) as usize] = __v189; }
                                    self.cite_ptr = (self.cite_ptr).wrapping_add(1i32);
                                }
                            }
                        }
                    }
                    // §124
                }
            }
        }
    }

    /// An \.{\\@@input} command will have exactly one argument, it will
    /// be between braces, and it must have the `s_aux_extension`.
    /// All upper-case letters are converted to lower case.
    /// @<Procedures and functions for the reading and processing of input files
    // §131
    pub fn aux_input_command(&mut self) {
        let mut aux_extension_ok: bool = false; // §131
        'l_exit_f: {
            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            if (!self.scan1_white(right_brace)) {
                {
                    self.aux_err_no_right_brace_print();
                    {
                        self.aux_err_print();
                        break 'l_exit_f;
                    }
                }
            }
            if (self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] == white_space) {
                {
                    self.aux_err_white_space_in_argument_print();
                    {
                        self.aux_err_print();
                        break 'l_exit_f;
                    }
                }
            }
            if (self.last > (self.buf_ptr2).wrapping_add(1i32)) {
                {
                    self.aux_err_stuff_after_right_brace_print();
                    {
                        self.aux_err_print();
                        break 'l_exit_f;
                    }
                }
            }
            // §132
            {
                self.aux_ptr = (self.aux_ptr).wrapping_add(1i32);
                if (self.aux_ptr == aux_stack_size) {
                    {
                        self.print_a_token();
                        {
                            {
                                crate::system::wr_str(&mut self.log_file, ": ");
                            }
                            {
                                crate::system::wr_str(&mut self.standard_output, ": ");
                            }
                        }
                        {
                            self.print_overflow();
                            {
                                {
                                    let __w1 = aux_stack_size;
                                    crate::system::wr_str(&mut self.log_file, "auxiliary file depth ");
                                    crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                                    crate::system::wr_ln(&mut self.log_file);
                                }
                                {
                                    let __w1 = aux_stack_size;
                                    crate::system::wr_str(&mut self.standard_output, "auxiliary file depth ");
                                    crate::system::wr_int(&mut self.standard_output, __w1, 0i32);
                                    crate::system::wr_ln(&mut self.standard_output);
                                }
                            }
                            crate::system::end_of_TEX(self);
                        }
                    }
                }
                aux_extension_ok = true;
                if ((self.buf_ptr2).wrapping_sub(self.buf_ptr1) < (self.str_start[((self.s_aux_extension).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.s_aux_extension) as usize])) {
                    aux_extension_ok = false;
                } else {
                    if (!{ let mut __f1 = ::core::mem::take(&mut self.buffer); let __r = self.str_eq_buf(self.s_aux_extension, &mut __f1, (self.buf_ptr2).wrapping_sub((self.str_start[((self.s_aux_extension).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.s_aux_extension) as usize])), (self.str_start[((self.s_aux_extension).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.s_aux_extension) as usize])); self.buffer = __f1; __r }) {
                        aux_extension_ok = false;
                    }
                }
                if (!aux_extension_ok) {
                    {
                        self.print_a_token();
                        {
                            {
                                crate::system::wr_str(&mut self.log_file, " has a wrong extension");
                            }
                            {
                                crate::system::wr_str(&mut self.standard_output, " has a wrong extension");
                            }
                        }
                        self.aux_ptr = (self.aux_ptr).wrapping_sub(1i32);
                        {
                            self.aux_err_print();
                            break 'l_exit_f;
                        }
                    }
                }
                { let __ix190 = self.aux_ptr; let __v191 = { let __s192 = ({ let mut __f0 = ::core::mem::take(&mut self.buffer); let __r = self.str_lookup(&mut __f0, self.buf_ptr1, (self.buf_ptr2).wrapping_sub(self.buf_ptr1), aux_file_ilk, true); self.buffer = __f0; __r }) as usize; self.hash_text[__s192] }; self.aux_list[(__ix190) as usize] = __v191; }
                if self.hash_found {
                    {
                        {
                            {
                                crate::system::wr_str(&mut self.log_file, "Already encountered file ");
                            }
                            {
                                crate::system::wr_str(&mut self.standard_output, "Already encountered file ");
                            }
                        }
                        self.print_aux_name();
                        self.aux_ptr = (self.aux_ptr).wrapping_sub(1i32);
                        {
                            self.aux_err_print();
                            break 'l_exit_f;
                        }
                    }
                }
                // §133
                {
                    self.start_name(self.aux_list[(self.aux_ptr) as usize]);
                    self.name_ptr = (self.name_length).wrapping_add(1i32);
                    self.name_of_file[(self.name_ptr) as usize] = ((0i32) as u8);
                    if ((!self.name_in_ok()) || ((!{ let mut __f0 = ::core::mem::take(&mut self.aux_file[(self.aux_ptr) as usize]); let __r = self.a_open_in(&mut __f0, (1i32).wrapping_neg()); self.aux_file[(self.aux_ptr) as usize] = __f0; __r }) && (!{ let mut __f0 = ::core::mem::take(&mut self.aux_file[(self.aux_ptr) as usize]); let __r = self.a_open_in_with_dirname(&mut __f0, (1i32).wrapping_neg(), self.top_lev_str); self.aux_file[(self.aux_ptr) as usize] = __f0; __r }))) {
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "I couldn't open auxiliary file ");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "I couldn't open auxiliary file ");
                                }
                            }
                            self.print_aux_name();
                            self.aux_ptr = (self.aux_ptr).wrapping_sub(1i32);
                            {
                                self.aux_err_print();
                                break 'l_exit_f;
                            }
                        }
                    }
                    {
                        {
                            let __w1 = self.aux_ptr;
                            crate::system::wr_str(&mut self.log_file, "A level-");
                            crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                            crate::system::wr_str(&mut self.log_file, " auxiliary file: ");
                        }
                    }
                    self.log_pr_aux_name();
                    self.aux_ln_stack[(self.aux_ptr) as usize] = 0i32;
                }
            }
        }
        // §131
    }

    /// Here we close the current-level \.{.aux} file and go back up a level,
    /// if possible, by decrementing `aux_ptr`.
    /// @<Procedures and functions for the reading and processing of input files
    // §134
    pub fn pop_the_aux_stack(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.aux_file[(self.aux_ptr) as usize]); let __r = self.a_close(&mut __f0); self.aux_file[(self.aux_ptr) as usize] = __f0; __r };
        if (self.aux_ptr == 0i32) {
            {
                self.lab31 = true;
                return;
            }
        } else {
            self.aux_ptr = (self.aux_ptr).wrapping_sub(1i32);
        }
    }

    /// We're not at the end of an \.{.aux} file, so we see if the current
    /// line might be a command of interest.  A command of interest will be a
    /// line without blanks, consisting of a command name, a `left_brace`, one
    /// or more arguments separated by commas, and a `right_brace`.
    /// @<Scan for and process an \.{.aux} command
    // §108
    pub fn get_aux_command_and_process(&mut self) {
        'l_exit_f: {
            self.buf_ptr2 = 0i32;
            if (!self.scan1(left_brace)) {
                break 'l_exit_f;
            }
            self.command_num = { let __s193 = ({ let mut __f0 = ::core::mem::take(&mut self.buffer); let __r = self.str_lookup(&mut __f0, self.buf_ptr1, (self.buf_ptr2).wrapping_sub(self.buf_ptr1), aux_command_ilk, false); self.buffer = __f0; __r }) as usize; self.ilk_info[__s193] };
            if self.hash_found {
                match self.command_num {
                    n_aux_bibdata => {
                        self.aux_bib_data_command();
                    }
                    n_aux_bibstyle => {
                        self.aux_bib_style_command();
                    }
                    n_aux_citation => {
                        self.aux_citation_command();
                    }
                    n_aux_input => {
                        self.aux_input_command();
                    }
                    _ => {
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "Unknown auxiliary-file command");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "Unknown auxiliary-file command");
                                }
                            }
                            self.print_confusion();
                            crate::system::end_of_TEX(self);
                        }
                    }
                }
            }
        }
    }

    /// Before proceeding, we see if we have any complaints.
    /// @<Procedures and functions for the reading and processing of input files
    // §137
    pub fn last_check_for_aux_errors(&mut self) {
        self.num_cites = self.cite_ptr;
        self.num_bib_files = self.bib_ptr;
        if (!self.citation_seen) {
            {
                self.aux_end1_err_print();
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "\\citation commands");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "\\citation commands");
                    }
                }
                self.aux_end2_err_print();
            }
        } else {
            if ((self.num_cites == 0i32) && (!self.all_entries)) {
                {
                    self.aux_end1_err_print();
                    {
                        {
                            crate::system::wr_str(&mut self.log_file, "cite keys");
                        }
                        {
                            crate::system::wr_str(&mut self.standard_output, "cite keys");
                        }
                    }
                    self.aux_end2_err_print();
                }
            }
        }
        if (!self.bib_seen) {
            {
                self.aux_end1_err_print();
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "\\bibdata command");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "\\bibdata command");
                    }
                }
                self.aux_end2_err_print();
            }
        } else {
            if (self.num_bib_files == 0i32) {
                {
                    self.aux_end1_err_print();
                    {
                        {
                            crate::system::wr_str(&mut self.log_file, "database files");
                        }
                        {
                            crate::system::wr_str(&mut self.standard_output, "database files");
                        }
                    }
                    self.aux_end2_err_print();
                }
            }
        }
        if (!self.bst_seen) {
            {
                self.aux_end1_err_print();
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "\\bibstyle command");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "\\bibstyle command");
                    }
                }
                self.aux_end2_err_print();
            }
        } else {
            if (self.bst_str == 0i32) {
                {
                    self.aux_end1_err_print();
                    {
                        {
                            crate::system::wr_str(&mut self.log_file, "style file");
                        }
                        {
                            crate::system::wr_str(&mut self.standard_output, "style file");
                        }
                    }
                    self.aux_end2_err_print();
                }
            }
        }
    }

}
