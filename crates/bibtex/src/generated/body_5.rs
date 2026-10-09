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
    /// Ok, that's it for sorting; now we'll play with the literal stack.
    /// This procedure pushes a literal onto the stack, checking for stack
    /// overflow.
    /// @<Procedures and functions for style-file function execution
    // §298
    pub fn push_lit_stk(&mut self, mut push_lt: i32, mut push_type: stk_type) {
        self.lit_stack[(self.lit_stk_ptr) as usize] = push_lt;
        self.lit_stk_type[(self.lit_stk_ptr) as usize] = push_type;
        if (self.lit_stk_ptr == self.lit_stk_size) {
            {
                self.log_realloc("lit_stack", 4i32, (self.lit_stk_size).wrapping_add(LIT_STK_SIZE), self.lit_stk_size);
                self.lit_stack.resize_len((((self.lit_stk_size).wrapping_add(LIT_STK_SIZE)) as usize) + 1);
                self.log_realloc("lit_stk_type", 1i32, (self.lit_stk_size).wrapping_add(LIT_STK_SIZE), self.lit_stk_size);
                self.lit_stk_type.resize_len((((self.lit_stk_size).wrapping_add(LIT_STK_SIZE)) as usize) + 1);
                self.lit_stk_size = (self.lit_stk_size).wrapping_add(LIT_STK_SIZE);
            }
        }
        self.lit_stk_ptr = (self.lit_stk_ptr).wrapping_add(1i32);
    }

    /// This procedure pops the stack, checking for, and trying to recover
    /// from, stack underflow.  (Actually, this procedure is really a
    /// function, since it returns the two values through its `var`
    /// parameters.)  Also, if the literal being popped is a `stk_str` that's
    /// been created during the execution of the current \.{.bst} command, pop
    /// it from `str_pool` as well (it will be the string corresponding to
    /// `str_ptr-1`).  Note that when this happens, the string is no longer
    /// `officially' available so that it must be used before anything else is
    /// added to `str_pool`.
    /// @<Procedures and functions for style-file function execution
    // §300
    pub fn pop_lit_stk(&mut self, pop_lit: &mut i32, pop_type: &mut stk_type) {
        if (self.lit_stk_ptr == 0i32) {
            {
                {
                    {
                        {
                            crate::system::wr_str(&mut self.log_file, "You can't pop an empty literal stack");
                        }
                        {
                            crate::system::wr_str(&mut self.standard_output, "You can't pop an empty literal stack");
                        }
                    }
                    self.bst_ex_warn_print();
                }
                (*pop_type) = stk_empty;
            }
        } else {
            {
                self.lit_stk_ptr = (self.lit_stk_ptr).wrapping_sub(1i32);
                (*pop_lit) = self.lit_stack[(self.lit_stk_ptr) as usize];
                (*pop_type) = self.lit_stk_type[(self.lit_stk_ptr) as usize];
                if ((*pop_type) == stk_str) {
                    if ((*pop_lit) >= self.cmd_str_ptr) {
                        {
                            if ((*pop_lit) != (self.str_ptr).wrapping_sub(1i32)) {
                                {
                                    {
                                        {
                                            crate::system::wr_str(&mut self.log_file, "Nontop top of string stack");
                                        }
                                        {
                                            crate::system::wr_str(&mut self.standard_output, "Nontop top of string stack");
                                        }
                                    }
                                    self.print_confusion();
                                    crate::system::end_of_TEX(self);
                                }
                            }
                            {
                                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                                self.pool_ptr = self.str_start[(self.str_ptr) as usize];
                            }
                        }
                    }
                }
            }
        }
    }

    /// This procedure appropriately chastises the style designer; however, if
    /// the wrong literal came from popping an empty stack, the procedure
    /// `pop_lit_stack` will have already done the chastising (because this
    /// procedure is called only after popping the stack) so there's no need
    /// for more.
    /// @<Procedures and functions for style-file function execution
    // §303
    pub fn print_wrong_stk_lit(&mut self, mut stk_lt: i32, mut stk_tp1: stk_type, mut stk_tp2: stk_type) {
        if (stk_tp1 != stk_empty) {
            {
                self.print_stk_lit(stk_lt, stk_tp1);
                match stk_tp2 {
                    stk_int => {
                        {
                            {
                                crate::system::wr_str(&mut self.log_file, ", not an integer,");
                            }
                            {
                                crate::system::wr_str(&mut self.standard_output, ", not an integer,");
                            }
                        }
                    }
                    stk_str => {
                        {
                            {
                                crate::system::wr_str(&mut self.log_file, ", not a string,");
                            }
                            {
                                crate::system::wr_str(&mut self.standard_output, ", not a string,");
                            }
                        }
                    }
                    stk_fn => {
                        {
                            {
                                crate::system::wr_str(&mut self.log_file, ", not a function,");
                            }
                            {
                                crate::system::wr_str(&mut self.standard_output, ", not a function,");
                            }
                        }
                    }
                    stk_field_missing | stk_empty => {
                        self.illegl_literal_confusion();
                    }
                    _ => {
                        self.unknwn_literal_confusion();
                    }
                }
                self.bst_ex_warn_print();
            }
        }
    }

    /// This procedure pops and prints the top of the stack; when the stack is
    /// empty the procedure `pop_lit_stk` complains.
    /// @<Procedures and functions for style-file function execution
    // §305
    pub fn pop_top_and_print(&mut self) {
        let mut stk_lt: i32 = 0; // §305
        let mut stk_tp: stk_type = 0; // §305
        { let mut __f0 = ::core::mem::take(&mut stk_lt); let mut __f1 = ::core::mem::take(&mut stk_tp); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); stk_lt = __f0; stk_tp = __f1; __r };
        if (stk_tp == stk_empty) {
            {
                {
                    crate::system::wr_str(&mut self.log_file, "Empty literal");
                    crate::system::wr_ln(&mut self.log_file);
                }
                {
                    crate::system::wr_str(&mut self.standard_output, "Empty literal");
                    crate::system::wr_ln(&mut self.standard_output);
                }
            }
        } else {
            self.print_lit(stk_lt, stk_tp);
        }
    }

    /// This procedure pops and prints the whole stack.
    /// @<Procedures and functions for style-file function execution
    // §306
    pub fn pop_whole_stack(&mut self) {
        while (self.lit_stk_ptr > 0i32) {
            self.pop_top_and_print();
        }
    }

    /// At the beginning of a \.{.bst}-command execution we make the stack
    /// empty and record how much of `str_pool` has been used.
    /// @<Procedures and functions for style-file function execution
    // §307
    pub fn init_command_execution(&mut self) {
        self.lit_stk_ptr = 0i32;
        self.cmd_str_ptr = self.str_ptr;
    }

    /// At the end of a \.{.bst} command-execution we check that the stack and
    /// `str_pool` are still in good shape.
    /// @<Procedures and functions for style-file function execution
    // §308
    pub fn check_command_execution(&mut self) {
        if (self.lit_stk_ptr != 0i32) {
            {
                {
                    {
                        let __w1 = self.lit_stk_ptr;
                        crate::system::wr_str(&mut self.log_file, "ptr=");
                        crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                        crate::system::wr_str(&mut self.log_file, ", stack=");
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        let __w1 = self.lit_stk_ptr;
                        crate::system::wr_str(&mut self.standard_output, "ptr=");
                        crate::system::wr_int(&mut self.standard_output, __w1, 0i32);
                        crate::system::wr_str(&mut self.standard_output, ", stack=");
                        crate::system::wr_ln(&mut self.standard_output);
                    }
                }
                self.pop_whole_stack();
                {
                    {
                        {
                            crate::system::wr_str(&mut self.log_file, "---the literal stack isn't empty");
                        }
                        {
                            crate::system::wr_str(&mut self.standard_output, "---the literal stack isn't empty");
                        }
                    }
                    self.bst_ex_warn_print();
                }
            }
        }
        if (self.cmd_str_ptr != self.str_ptr) {
            {
                {
                    {
                        {
                            crate::system::wr_str(&mut self.log_file, "Nonempty empty string stack");
                        }
                        {
                            crate::system::wr_str(&mut self.standard_output, "Nonempty empty string stack");
                        }
                    }
                    self.print_confusion();
                    crate::system::end_of_TEX(self);
                }
            }
        }
    }

    /// This procedure adds to `str_pool` the string from `ex_buf[0]` through
    /// `ex_buf[ex_buf_length-1]` if it will fit.  It assumes the global
    /// variable `ex_buf_length` gives the length of the current string in
    /// `ex_buf`.  It then pushes this string onto the literal stack.
    /// @<Procedures and functions for style-file function execution
    // §309
    pub fn add_pool_buf_and_push(&mut self) {
        {
            while ((self.pool_ptr).wrapping_add(self.ex_buf_length) > self.pool_size) {
                self.pool_overflow();
            }
        }
        self.ex_buf_ptr = 0i32;
        while (self.ex_buf_ptr < self.ex_buf_length) {
            {
                {
                    { let __ix77 = self.pool_ptr; let __v78 = self.ex_buf[(self.ex_buf_ptr) as usize]; self.str_pool[(__ix77) as usize] = __v78; }
                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                }
                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
            }
        }
        { let __a79_0 = self.make_string(); let __a79_1 = stk_str; self.push_lit_stk(__a79_0, __a79_1) };
    }

    /// This procedure adds to the execution buffer the given string in
    /// `str_pool` if it will fit.  It assumes the global variable
    /// `ex_buf_length` gives the length of the current string in `ex_buf`,
    /// and thus also gives the location of the next character.
    /// @<Procedures and functions for style-file function execution
    // §311
    pub fn add_buf_pool(&mut self, mut p_str: str_number) {
        self.p_ptr1 = self.str_start[(p_str) as usize];
        self.p_ptr2 = self.str_start[((p_str).wrapping_add(1i32)) as usize];
        if ((self.ex_buf_length).wrapping_add((self.p_ptr2).wrapping_sub(self.p_ptr1)) > self.buf_size) {
            self.buffer_overflow();
        }
        self.ex_buf_ptr = self.ex_buf_length;
        while (self.p_ptr1 < self.p_ptr2) {
            {
                {
                    { let __ix80 = self.ex_buf_ptr; let __v81 = self.str_pool[(self.p_ptr1) as usize]; self.ex_buf[(__ix80) as usize] = __v81; }
                    self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                }
                self.p_ptr1 = (self.p_ptr1).wrapping_add(1i32);
            }
        }
        self.ex_buf_length = self.ex_buf_ptr;
    }

    /// This procedure adds to the output buffer the given string in
    /// `str_pool`.  It assumes the global variable `out_buf_length` gives the
    /// length of the current string in `out_buf`, and thus also gives the
    /// location for the next character.  If there are enough characters
    /// present in the output buffer, it writes one or more lines out to the
    /// \.{.bbl} file.  It breaks a line only at a `white_space` character,
    /// and when it does, it adds two `space`s to the next output line.
    /// @<Procedures and functions for style-file function execution
    // §313
    pub fn add_out_pool(&mut self, mut p_str: str_number) {
        let mut break_ptr: buf_pointer = 0; // §313
        let mut end_ptr: buf_pointer = 0; // §313
        let mut break_pt_found: bool = false; // §313
        let mut unbreakable_tail: bool = false; // §313
        self.p_ptr1 = self.str_start[(p_str) as usize];
        self.p_ptr2 = self.str_start[((p_str).wrapping_add(1i32)) as usize];
        while ((self.out_buf_length).wrapping_add((self.p_ptr2).wrapping_sub(self.p_ptr1)) > self.buf_size) {
            self.buffer_overflow();
        }
        self.out_buf_ptr = self.out_buf_length;
        while (self.p_ptr1 < self.p_ptr2) {
            {
                { let __ix82 = self.out_buf_ptr; let __v83 = self.str_pool[(self.p_ptr1) as usize]; self.out_buf[(__ix82) as usize] = __v83; }
                self.p_ptr1 = (self.p_ptr1).wrapping_add(1i32);
                self.out_buf_ptr = (self.out_buf_ptr).wrapping_add(1i32);
            }
        }
        self.out_buf_length = self.out_buf_ptr;
        unbreakable_tail = false;
        while ((self.out_buf_length > self.max_print_line) && (!unbreakable_tail)) {
            // §314
            {
                end_ptr = self.out_buf_length;
                self.out_buf_ptr = self.max_print_line;
                break_pt_found = false;
                while ((self.lex_class[(self.out_buf[(self.out_buf_ptr) as usize]) as usize] != white_space) && (self.out_buf_ptr >= min_print_line)) {
                    self.out_buf_ptr = (self.out_buf_ptr).wrapping_sub(1i32);
                }
                if (self.out_buf_ptr == (min_print_line).wrapping_sub(1i32)) {
                    // §315
                    {
                        'l_L16_f: {
                            self.out_buf_ptr = (self.max_print_line).wrapping_add(1i32);
                            while (self.out_buf_ptr < end_ptr) {
                                if (self.lex_class[(self.out_buf[(self.out_buf_ptr) as usize]) as usize] != white_space) {
                                    self.out_buf_ptr = (self.out_buf_ptr).wrapping_add(1i32);
                                } else {
                                    break 'l_L16_f;
                                }
                            }
                        }
                        if (self.out_buf_ptr == end_ptr) {
                            unbreakable_tail = true;
                        } else {
                            {
                                'l_L17_f: {
                                    break_pt_found = true;
                                    while ((self.out_buf_ptr).wrapping_add(1i32) < end_ptr) {
                                        if (self.lex_class[(self.out_buf[((self.out_buf_ptr).wrapping_add(1i32)) as usize]) as usize] == white_space) {
                                            self.out_buf_ptr = (self.out_buf_ptr).wrapping_add(1i32);
                                        } else {
                                            break 'l_L17_f;
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // §314
                    break_pt_found = true;
                }
                if break_pt_found {
                    {
                        self.out_buf_length = self.out_buf_ptr;
                        break_ptr = (self.out_buf_length).wrapping_add(1i32);
                        self.output_bbl_line();
                        self.out_buf[(0i32) as usize] = space;
                        self.out_buf[(1i32) as usize] = space;
                        self.out_buf_ptr = 2i32;
                        self.tmp_ptr = break_ptr;
                        while (self.tmp_ptr < end_ptr) {
                            {
                                { let __ix84 = self.out_buf_ptr; let __v85 = self.out_buf[(self.tmp_ptr) as usize]; self.out_buf[(__ix84) as usize] = __v85; }
                                self.out_buf_ptr = (self.out_buf_ptr).wrapping_add(1i32);
                                self.tmp_ptr = (self.tmp_ptr).wrapping_add(1i32);
                            }
                        }
                        self.out_buf_length = ((end_ptr).wrapping_sub(break_ptr)).wrapping_add(2i32);
                    }
                }
            }
        }
    }

    /// The `built_in` function {\.{=}} pops the top two (integer or string)
    /// literals, compares them, and pushes the integer 1 if they're equal, 0
    /// otherwise.  If they're not either both string or both integer, it
    /// complains and pushes the integer 0.
    /// @<`execute_fn`({\.{=}})
    // §336
    pub fn x_equals(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit2); let mut __f1 = ::core::mem::take(&mut self.pop_typ2); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit2 = __f0; self.pop_typ2 = __f1; __r };
        if (self.pop_typ1 != self.pop_typ2) {
            {
                if ((self.pop_typ1 != stk_empty) && (self.pop_typ2 != stk_empty)) {
                    {
                        self.print_stk_lit(self.pop_lit1, self.pop_typ1);
                        {
                            {
                                crate::system::wr_str(&mut self.log_file, ", ");
                            }
                            {
                                crate::system::wr_str(&mut self.standard_output, ", ");
                            }
                        }
                        self.print_stk_lit(self.pop_lit2, self.pop_typ2);
                        self.print_a_newline();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "---they aren't the same literal types");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "---they aren't the same literal types");
                                }
                            }
                            self.bst_ex_warn_print();
                        }
                    }
                }
                self.push_lit_stk(0i32, stk_int);
            }
        } else {
            if ((self.pop_typ1 != stk_int) && (self.pop_typ1 != stk_str)) {
                {
                    if (self.pop_typ1 != stk_empty) {
                        {
                            self.print_stk_lit(self.pop_lit1, self.pop_typ1);
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, ", not an integer or a string,");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, ", not an integer or a string,");
                                    }
                                }
                                self.bst_ex_warn_print();
                            }
                        }
                    }
                    self.push_lit_stk(0i32, stk_int);
                }
            } else {
                if (self.pop_typ1 == stk_int) {
                    if (self.pop_lit2 == self.pop_lit1) {
                        self.push_lit_stk(1i32, stk_int);
                    } else {
                        self.push_lit_stk(0i32, stk_int);
                    }
                } else {
                    if self.str_eq_str(self.pop_lit2, self.pop_lit1) {
                        self.push_lit_stk(1i32, stk_int);
                    } else {
                        self.push_lit_stk(0i32, stk_int);
                    }
                }
            }
        }
    }

    /// The `built_in` function {\.{>}} pops the top two (integer) literals,
    /// compares them, and pushes the integer 1 if the second is greater than
    /// the first, 0 otherwise.  If either isn't an integer literal, it
    /// complains and pushes the integer 0.
    /// @<`execute_fn`({\.{>}})
    // §337
    pub fn x_greater_than(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit2); let mut __f1 = ::core::mem::take(&mut self.pop_typ2); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit2 = __f0; self.pop_typ2 = __f1; __r };
        if (self.pop_typ1 != stk_int) {
            {
                self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_int);
                self.push_lit_stk(0i32, stk_int);
            }
        } else {
            if (self.pop_typ2 != stk_int) {
                {
                    self.print_wrong_stk_lit(self.pop_lit2, self.pop_typ2, stk_int);
                    self.push_lit_stk(0i32, stk_int);
                }
            } else {
                if (self.pop_lit2 > self.pop_lit1) {
                    self.push_lit_stk(1i32, stk_int);
                } else {
                    self.push_lit_stk(0i32, stk_int);
                }
            }
        }
    }

    /// The `built_in` function {\.{<}} pops the top two (integer) literals,
    /// compares them, and pushes the integer 1 if the second is less than the
    /// first, 0 otherwise.  If either isn't an integer literal, it complains
    /// and pushes the integer 0.
    /// @<`execute_fn`({\.{<}})
    // §338
    pub fn x_less_than(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit2); let mut __f1 = ::core::mem::take(&mut self.pop_typ2); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit2 = __f0; self.pop_typ2 = __f1; __r };
        if (self.pop_typ1 != stk_int) {
            {
                self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_int);
                self.push_lit_stk(0i32, stk_int);
            }
        } else {
            if (self.pop_typ2 != stk_int) {
                {
                    self.print_wrong_stk_lit(self.pop_lit2, self.pop_typ2, stk_int);
                    self.push_lit_stk(0i32, stk_int);
                }
            } else {
                if (self.pop_lit2 < self.pop_lit1) {
                    self.push_lit_stk(1i32, stk_int);
                } else {
                    self.push_lit_stk(0i32, stk_int);
                }
            }
        }
    }

    /// The `built_in` function {\.{+}} pops the top two (integer) literals
    /// and pushes their sum.  If either isn't an integer literal, it
    /// complains and pushes the integer 0.
    /// @<`execute_fn`({\.{+}})
    // §339
    pub fn x_plus(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit2); let mut __f1 = ::core::mem::take(&mut self.pop_typ2); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit2 = __f0; self.pop_typ2 = __f1; __r };
        if (self.pop_typ1 != stk_int) {
            {
                self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_int);
                self.push_lit_stk(0i32, stk_int);
            }
        } else {
            if (self.pop_typ2 != stk_int) {
                {
                    self.print_wrong_stk_lit(self.pop_lit2, self.pop_typ2, stk_int);
                    self.push_lit_stk(0i32, stk_int);
                }
            } else {
                self.push_lit_stk((self.pop_lit2).wrapping_add(self.pop_lit1), stk_int);
            }
        }
    }

    /// The `built_in` function {\.{-}} pops the top two (integer) literals
    /// and pushes their difference (the first subtracted from the second).
    /// If either isn't an integer literal, it complains and pushes the
    /// integer 0.
    /// @<`execute_fn`({\.{-}})
    // §340
    pub fn x_minus(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit2); let mut __f1 = ::core::mem::take(&mut self.pop_typ2); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit2 = __f0; self.pop_typ2 = __f1; __r };
        if (self.pop_typ1 != stk_int) {
            {
                self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_int);
                self.push_lit_stk(0i32, stk_int);
            }
        } else {
            if (self.pop_typ2 != stk_int) {
                {
                    self.print_wrong_stk_lit(self.pop_lit2, self.pop_typ2, stk_int);
                    self.push_lit_stk(0i32, stk_int);
                }
            } else {
                self.push_lit_stk((self.pop_lit2).wrapping_sub(self.pop_lit1), stk_int);
            }
        }
    }

    /// The `built_in` function {\.{*}} pops the top two (string) literals,
    /// concatenates them (in reverse order, that is, the order in which
    /// pushed), and pushes the resulting string back onto the stack.  If
    /// either isn't a string literal, it complains and pushes the null
    /// string.
    /// @<`execute_fn`({\.{*}})
    // §341
    pub fn x_concatenate(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit2); let mut __f1 = ::core::mem::take(&mut self.pop_typ2); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit2 = __f0; self.pop_typ2 = __f1; __r };
        if (self.pop_typ1 != stk_str) {
            {
                self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_str);
                self.push_lit_stk(self.s_null, stk_str);
            }
        } else {
            if (self.pop_typ2 != stk_str) {
                {
                    self.print_wrong_stk_lit(self.pop_lit2, self.pop_typ2, stk_str);
                    self.push_lit_stk(self.s_null, stk_str);
                }
            } else {
                // §342
                {
                    if (self.pop_lit2 >= self.cmd_str_ptr) {
                        if (self.pop_lit1 >= self.cmd_str_ptr) {
                            {
                                { let __ix86 = self.pop_lit1; let __v87 = self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize]; self.str_start[(__ix86) as usize] = __v87; }
                                {
                                    self.str_ptr = (self.str_ptr).wrapping_add(1i32);
                                    self.pool_ptr = self.str_start[(self.str_ptr) as usize];
                                }
                                self.lit_stk_ptr = (self.lit_stk_ptr).wrapping_add(1i32);
                            }
                        } else {
                            if ((self.str_start[((self.pop_lit2).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pop_lit2) as usize]) == 0i32) {
                                self.push_lit_stk(self.pop_lit1, stk_str);
                            } else {
                                {
                                    self.pool_ptr = self.str_start[((self.pop_lit2).wrapping_add(1i32)) as usize];
                                    {
                                        while ((self.pool_ptr).wrapping_add((self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pop_lit1) as usize])) > self.pool_size) {
                                            self.pool_overflow();
                                        }
                                    }
                                    self.sp_ptr = self.str_start[(self.pop_lit1) as usize];
                                    self.sp_end = self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize];
                                    while (self.sp_ptr < self.sp_end) {
                                        {
                                            {
                                                { let __ix88 = self.pool_ptr; let __v89 = self.str_pool[(self.sp_ptr) as usize]; self.str_pool[(__ix88) as usize] = __v89; }
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                            self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                        }
                                    }
                                    { let __a90_0 = self.make_string(); let __a90_1 = stk_str; self.push_lit_stk(__a90_0, __a90_1) };
                                }
                            }
                        }
                    } else {
                        // §343
                        {
                            if (self.pop_lit1 >= self.cmd_str_ptr) {
                                if ((self.str_start[((self.pop_lit2).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pop_lit2) as usize]) == 0i32) {
                                    {
                                        {
                                            self.str_ptr = (self.str_ptr).wrapping_add(1i32);
                                            self.pool_ptr = self.str_start[(self.str_ptr) as usize];
                                        }
                                        { let __ix91 = self.lit_stk_ptr; let __v92 = self.pop_lit1; self.lit_stack[(__ix91) as usize] = __v92; }
                                        self.lit_stk_ptr = (self.lit_stk_ptr).wrapping_add(1i32);
                                    }
                                } else {
                                    if ((self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pop_lit1) as usize]) == 0i32) {
                                        self.lit_stk_ptr = (self.lit_stk_ptr).wrapping_add(1i32);
                                    } else {
                                        {
                                            self.sp_length = (self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pop_lit1) as usize]);
                                            self.sp2_length = (self.str_start[((self.pop_lit2).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pop_lit2) as usize]);
                                            {
                                                while (((self.pool_ptr).wrapping_add(self.sp_length)).wrapping_add(self.sp2_length) > self.pool_size) {
                                                    self.pool_overflow();
                                                }
                                            }
                                            self.sp_ptr = self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize];
                                            self.sp_end = self.str_start[(self.pop_lit1) as usize];
                                            self.sp_xptr1 = (self.sp_ptr).wrapping_add(self.sp2_length);
                                            while (self.sp_ptr > self.sp_end) {
                                                {
                                                    self.sp_ptr = (self.sp_ptr).wrapping_sub(1i32);
                                                    self.sp_xptr1 = (self.sp_xptr1).wrapping_sub(1i32);
                                                    { let __ix93 = self.sp_xptr1; let __v94 = self.str_pool[(self.sp_ptr) as usize]; self.str_pool[(__ix93) as usize] = __v94; }
                                                }
                                            }
                                            self.sp_ptr = self.str_start[(self.pop_lit2) as usize];
                                            self.sp_end = self.str_start[((self.pop_lit2).wrapping_add(1i32)) as usize];
                                            while (self.sp_ptr < self.sp_end) {
                                                {
                                                    {
                                                        { let __ix95 = self.pool_ptr; let __v96 = self.str_pool[(self.sp_ptr) as usize]; self.str_pool[(__ix95) as usize] = __v96; }
                                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                    }
                                                    self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                                }
                                            }
                                            self.pool_ptr = (self.pool_ptr).wrapping_add(self.sp_length);
                                            { let __a97_0 = self.make_string(); let __a97_1 = stk_str; self.push_lit_stk(__a97_0, __a97_1) };
                                        }
                                    }
                                }
                            } else {
                                // §344
                                {
                                    if ((self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pop_lit1) as usize]) == 0i32) {
                                        self.lit_stk_ptr = (self.lit_stk_ptr).wrapping_add(1i32);
                                    } else {
                                        if ((self.str_start[((self.pop_lit2).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pop_lit2) as usize]) == 0i32) {
                                            self.push_lit_stk(self.pop_lit1, stk_str);
                                        } else {
                                            {
                                                {
                                                    while (((self.pool_ptr).wrapping_add((self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pop_lit1) as usize]))).wrapping_add((self.str_start[((self.pop_lit2).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pop_lit2) as usize])) > self.pool_size) {
                                                        self.pool_overflow();
                                                    }
                                                }
                                                self.sp_ptr = self.str_start[(self.pop_lit2) as usize];
                                                self.sp_end = self.str_start[((self.pop_lit2).wrapping_add(1i32)) as usize];
                                                while (self.sp_ptr < self.sp_end) {
                                                    {
                                                        {
                                                            { let __ix98 = self.pool_ptr; let __v99 = self.str_pool[(self.sp_ptr) as usize]; self.str_pool[(__ix98) as usize] = __v99; }
                                                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                        }
                                                        self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                                    }
                                                }
                                                self.sp_ptr = self.str_start[(self.pop_lit1) as usize];
                                                self.sp_end = self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize];
                                                while (self.sp_ptr < self.sp_end) {
                                                    {
                                                        {
                                                            { let __ix100 = self.pool_ptr; let __v101 = self.str_pool[(self.sp_ptr) as usize]; self.str_pool[(__ix100) as usize] = __v101; }
                                                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                        }
                                                        self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                                    }
                                                }
                                                { let __a102_0 = self.make_string(); let __a102_1 = stk_str; self.push_lit_stk(__a102_0, __a102_1) };
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// The `built_in` function {\.{:=}} pops the top two literals and assigns
    /// to the first (which must be an `int_entry_var`, a `str_entry_var`, an
    /// `int_global_var`, or a `str_global_var`) the value of the second;
    /// it complains if the value isn't of the appropriate type.
    /// @<`execute_fn`({\.{:=}})
    // §345
    pub fn x_gets(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit2); let mut __f1 = ::core::mem::take(&mut self.pop_typ2); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit2 = __f0; self.pop_typ2 = __f1; __r };
        if (self.pop_typ1 != stk_fn) {
            self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_fn);
        } else {
            if ((!self.mess_with_entries) && ((self.fn_type[(self.pop_lit1) as usize] == str_entry_var) || (self.fn_type[(self.pop_lit1) as usize] == int_entry_var))) {
                self.bst_cant_mess_with_entries_print();
            } else {
                match self.fn_type[(self.pop_lit1) as usize] {
                    int_entry_var => {
                        // §346
                        if (self.pop_typ2 != stk_int) {
                            self.print_wrong_stk_lit(self.pop_lit2, self.pop_typ2, stk_int);
                        } else {
                            { let __ix103 = ((self.cite_ptr).wrapping_mul(self.num_ent_ints)).wrapping_add(self.ilk_info[(self.pop_lit1) as usize]); let __v104 = self.pop_lit2; self.entry_ints[(__ix103) as usize] = __v104; }
                        }
                    }
                    str_entry_var => {
                        // §348
                        {
                            if (self.pop_typ2 != stk_str) {
                                self.print_wrong_stk_lit(self.pop_lit2, self.pop_typ2, stk_str);
                            } else {
                                {
                                    self.str_ent_ptr = ((self.cite_ptr).wrapping_mul(self.num_ent_strs)).wrapping_add(self.ilk_info[(self.pop_lit1) as usize]);
                                    self.ent_chr_ptr = 0i32;
                                    self.sp_ptr = self.str_start[(self.pop_lit2) as usize];
                                    self.sp_xptr1 = self.str_start[((self.pop_lit2).wrapping_add(1i32)) as usize];
                                    if ((self.sp_xptr1).wrapping_sub(self.sp_ptr) > self.ent_str_size) {
                                        {
                                            {
                                                self.bst_1print_string_size_exceeded();
                                                {
                                                    {
                                                        let __w0 = self.ent_str_size;
                                                        crate::system::wr_int(&mut self.log_file, __w0, 0i32);
                                                        crate::system::wr_str(&mut self.log_file, ", the entry");
                                                    }
                                                    {
                                                        let __w0 = self.ent_str_size;
                                                        crate::system::wr_int(&mut self.standard_output, __w0, 0i32);
                                                        crate::system::wr_str(&mut self.standard_output, ", the entry");
                                                    }
                                                }
                                                self.bst_2print_string_size_exceeded();
                                            }
                                            self.sp_xptr1 = (self.sp_ptr).wrapping_add(self.ent_str_size);
                                        }
                                    }
                                    while (self.sp_ptr < self.sp_xptr1) {
                                        {
                                            { let __ix105 = ((self.str_ent_ptr).wrapping_mul((self.ent_str_size).wrapping_add(1i32))).wrapping_add(self.ent_chr_ptr); let __v106 = self.str_pool[(self.sp_ptr) as usize]; self.entry_strs[(__ix105) as usize] = __v106; }
                                            self.ent_chr_ptr = (self.ent_chr_ptr).wrapping_add(1i32);
                                            self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                        }
                                    }
                                    self.entry_strs[(((self.str_ent_ptr).wrapping_mul((self.ent_str_size).wrapping_add(1i32))).wrapping_add(self.ent_chr_ptr)) as usize] = end_of_string;
                                }
                            }
                        }
                    }
                    int_global_var => {
                        // §349
                        if (self.pop_typ2 != stk_int) {
                            self.print_wrong_stk_lit(self.pop_lit2, self.pop_typ2, stk_int);
                        } else {
                            { let __ix107 = self.pop_lit1; let __v108 = self.pop_lit2; self.ilk_info[(__ix107) as usize] = __v108; }
                        }
                    }
                    str_global_var => {
                        // §350
                        {
                            if (self.pop_typ2 != stk_str) {
                                self.print_wrong_stk_lit(self.pop_lit2, self.pop_typ2, stk_str);
                            } else {
                                {
                                    self.str_glb_ptr = self.ilk_info[(self.pop_lit1) as usize];
                                    if (self.pop_lit2 < self.cmd_str_ptr) {
                                        { let __ix109 = self.str_glb_ptr; let __v110 = self.pop_lit2; self.glb_str_ptr[(__ix109) as usize] = __v110; }
                                    } else {
                                        {
                                            self.glb_str_ptr[(self.str_glb_ptr) as usize] = 0i32;
                                            self.glob_chr_ptr = 0i32;
                                            self.sp_ptr = self.str_start[(self.pop_lit2) as usize];
                                            self.sp_end = self.str_start[((self.pop_lit2).wrapping_add(1i32)) as usize];
                                            if ((self.sp_end).wrapping_sub(self.sp_ptr) > self.glob_str_size) {
                                                {
                                                    {
                                                        self.bst_1print_string_size_exceeded();
                                                        {
                                                            {
                                                                let __w0 = self.glob_str_size;
                                                                crate::system::wr_int(&mut self.log_file, __w0, 0i32);
                                                                crate::system::wr_str(&mut self.log_file, ", the global");
                                                            }
                                                            {
                                                                let __w0 = self.glob_str_size;
                                                                crate::system::wr_int(&mut self.standard_output, __w0, 0i32);
                                                                crate::system::wr_str(&mut self.standard_output, ", the global");
                                                            }
                                                        }
                                                        self.bst_2print_string_size_exceeded();
                                                    }
                                                    self.sp_end = (self.sp_ptr).wrapping_add(self.glob_str_size);
                                                }
                                            }
                                            while (self.sp_ptr < self.sp_end) {
                                                {
                                                    { let __ix111 = ((self.str_glb_ptr).wrapping_mul((self.glob_str_size).wrapping_add(1i32))).wrapping_add(self.glob_chr_ptr); let __v112 = self.str_pool[(self.sp_ptr) as usize]; self.global_strs[(__ix111) as usize] = __v112; }
                                                    self.glob_chr_ptr = (self.glob_chr_ptr).wrapping_add(1i32);
                                                    self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                                }
                                            }
                                            { let __ix113 = self.str_glb_ptr; let __v114 = self.glob_chr_ptr; self.glb_str_end[(__ix113) as usize] = __v114; }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        // §345
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "You can't assign to type ");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "You can't assign to type ");
                                }
                            }
                            self.print_fn_class(self.pop_lit1);
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, ", a nonvariable function class");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, ", a nonvariable function class");
                                    }
                                }
                                self.bst_ex_warn_print();
                            }
                        }
                    }
                }
            }
        }
    }

    /// The `built_in` function {\.{add.period\$}} pops the top (string)
    /// literal, adds a `period` to a nonnull string if its last
    /// non`right_brace` character isn't a `period`, `question_mark`, or
    /// `exclamation_mark`, and pushes this resulting string back onto the
    /// stack.  If the literal isn't a string, it complains and pushes the
    /// null string.
    /// @<`execute_fn`({\.{add.period\$}})
    // §351
    pub fn x_add_period(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        if (self.pop_typ1 != stk_str) {
            {
                self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_str);
                self.push_lit_stk(self.s_null, stk_str);
            }
        } else {
            if ((self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pop_lit1) as usize]) == 0i32) {
                self.push_lit_stk(self.s_null, stk_str);
            } else {
                // §352
                {
                    'l_L15_f: {
                        self.sp_ptr = self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize];
                        self.sp_end = self.str_start[(self.pop_lit1) as usize];
                        while (self.sp_ptr > self.sp_end) {
                            {
                                self.sp_ptr = (self.sp_ptr).wrapping_sub(1i32);
                                if (self.str_pool[(self.sp_ptr) as usize] != right_brace) {
                                    break 'l_L15_f;
                                }
                            }
                        }
                    }
                    match self.str_pool[(self.sp_ptr) as usize] {
                        period | question_mark | exclamation_mark => {
                            {
                                if (self.lit_stack[(self.lit_stk_ptr) as usize] >= self.cmd_str_ptr) {
                                    {
                                        self.str_ptr = (self.str_ptr).wrapping_add(1i32);
                                        self.pool_ptr = self.str_start[(self.str_ptr) as usize];
                                    }
                                }
                                self.lit_stk_ptr = (self.lit_stk_ptr).wrapping_add(1i32);
                            }
                        }
                        _ => {
                            // §353
                            {
                                if (self.pop_lit1 < self.cmd_str_ptr) {
                                    {
                                        {
                                            while (((self.pool_ptr).wrapping_add((self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pop_lit1) as usize]))).wrapping_add(1i32) > self.pool_size) {
                                                self.pool_overflow();
                                            }
                                        }
                                        self.sp_ptr = self.str_start[(self.pop_lit1) as usize];
                                        self.sp_end = self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize];
                                        while (self.sp_ptr < self.sp_end) {
                                            {
                                                {
                                                    { let __ix115 = self.pool_ptr; let __v116 = self.str_pool[(self.sp_ptr) as usize]; self.str_pool[(__ix115) as usize] = __v116; }
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                }
                                                self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                            }
                                        }
                                    }
                                } else {
                                    {
                                        self.pool_ptr = self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize];
                                        {
                                            while ((self.pool_ptr).wrapping_add(1i32) > self.pool_size) {
                                                self.pool_overflow();
                                            }
                                        }
                                    }
                                }
                                {
                                    self.str_pool[(self.pool_ptr) as usize] = period;
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                }
                                { let __a117_0 = self.make_string(); let __a117_1 = stk_str; self.push_lit_stk(__a117_0, __a117_1) };
                            }
                        }
                    }
                }
            }
        }
    }

    /// The `built_in` function {\.{change.case\$}} pops the top two (string)
    /// literals; it changes the case of the second according to the
    /// specifications of the first, as follows.  (Note: The word `letters' in
    /// the next sentence refers only to those at brace-level~0, the top-most
    /// brace level; no other characters are changed, except perhaps for
    /// special characters, described shortly.)  If the first literal is the
    /// string~\.{t}, it converts to lower case all letters except the very
    /// first character in the string, which it leaves alone, and except the
    /// first character following any `colon` and then nonnull `white_space`,
    /// which it also leaves alone; if it's the string~\.{l}, it converts all
    /// letters to lower case; if it's the string~\.{u}, it converts all
    /// letters to upper case; and if it's anything else, it complains and
    /// does no conversion.  It then pushes this resulting string.  If either
    /// type is incorrect, it complains and pushes the null string; however,
    /// ...
    // §355
    pub fn x_change_case(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit2); let mut __f1 = ::core::mem::take(&mut self.pop_typ2); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit2 = __f0; self.pop_typ2 = __f1; __r };
        if (self.pop_typ1 != stk_str) {
            {
                self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_str);
                self.push_lit_stk(self.s_null, stk_str);
            }
        } else {
            if (self.pop_typ2 != stk_str) {
                {
                    self.print_wrong_stk_lit(self.pop_lit2, self.pop_typ2, stk_str);
                    self.push_lit_stk(self.s_null, stk_str);
                }
            } else {
                {
                    // §357
                    {
                        match self.str_pool[(self.str_start[(self.pop_lit1) as usize]) as usize] {
                            116 | 84 => {
                                self.conversion_type = title_lowers;
                            }
                            108 | 76 => {
                                self.conversion_type = all_lowers;
                            }
                            117 | 85 => {
                                self.conversion_type = all_uppers;
                            }
                            _ => {
                                self.conversion_type = bad_conversion;
                            }
                        }
                        if (((self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pop_lit1) as usize]) != 1i32) || (self.conversion_type == bad_conversion)) {
                            {
                                self.conversion_type = bad_conversion;
                                self.print_a_pool_str(self.pop_lit1);
                                {
                                    {
                                        {
                                            crate::system::wr_str(&mut self.log_file, " is an illegal case-conversion string");
                                        }
                                        {
                                            crate::system::wr_str(&mut self.standard_output, " is an illegal case-conversion string");
                                        }
                                    }
                                    self.bst_ex_warn_print();
                                }
                            }
                        }
                    }
                    // §355
                    self.ex_buf_length = 0i32;
                    self.add_buf_pool(self.pop_lit2);
                    // §361
                    {
                        self.brace_level = 0i32;
                        self.ex_buf_ptr = 0i32;
                        while (self.ex_buf_ptr < self.ex_buf_length) {
                            {
                                if (self.ex_buf[(self.ex_buf_ptr) as usize] == left_brace) {
                                    {
                                        'l_L21_f: {
                                            self.brace_level = (self.brace_level).wrapping_add(1i32);
                                            if (self.brace_level != 1i32) {
                                                break 'l_L21_f;
                                            }
                                            if ((self.ex_buf_ptr).wrapping_add(4i32) > self.ex_buf_length) {
                                                break 'l_L21_f;
                                            } else {
                                                if (self.ex_buf[((self.ex_buf_ptr).wrapping_add(1i32)) as usize] != backslash) {
                                                    break 'l_L21_f;
                                                }
                                            }
                                            if (self.conversion_type == title_lowers) {
                                                if (self.ex_buf_ptr == 0i32) {
                                                    break 'l_L21_f;
                                                } else {
                                                    if (self.prev_colon && (self.lex_class[(self.ex_buf[((self.ex_buf_ptr).wrapping_sub(1i32)) as usize]) as usize] == white_space)) {
                                                        break 'l_L21_f;
                                                    }
                                                }
                                            }
                                            // §362
                                            {
                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                while ((self.ex_buf_ptr < self.ex_buf_length) && (self.brace_level > 0i32)) {
                                                    {
                                                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                        self.ex_buf_xptr = self.ex_buf_ptr;
                                                        while ((self.ex_buf_ptr < self.ex_buf_length) && (self.lex_class[(self.ex_buf[(self.ex_buf_ptr) as usize]) as usize] == alpha)) {
                                                            self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                        }
                                                        self.control_seq_loc = { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.str_lookup(&mut __f0, self.ex_buf_xptr, (self.ex_buf_ptr).wrapping_sub(self.ex_buf_xptr), control_seq_ilk, false); self.ex_buf = __f0; __r };
                                                        if self.hash_found {
                                                            // §363
                                                            {
                                                                match self.conversion_type {
                                                                    title_lowers | all_lowers => {
                                                                        match self.ilk_info[(self.control_seq_loc) as usize] {
                                                                            n_l_upper | n_o_upper | n_oe_upper | n_ae_upper | n_aa_upper => {
                                                                                { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.lower_case(&mut __f0, self.ex_buf_xptr, (self.ex_buf_ptr).wrapping_sub(self.ex_buf_xptr)); self.ex_buf = __f0; __r };
                                                                            }
                                                                            _ => {
                                                                            }
                                                                        }
                                                                    }
                                                                    all_uppers => {
                                                                        match self.ilk_info[(self.control_seq_loc) as usize] {
                                                                            n_l | n_o | n_oe | n_ae | n_aa => {
                                                                                { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.upper_case(&mut __f0, self.ex_buf_xptr, (self.ex_buf_ptr).wrapping_sub(self.ex_buf_xptr)); self.ex_buf = __f0; __r };
                                                                            }
                                                                            n_i | n_j | n_ss => {
                                                                                // §365
                                                                                {
                                                                                    { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.upper_case(&mut __f0, self.ex_buf_xptr, (self.ex_buf_ptr).wrapping_sub(self.ex_buf_xptr)); self.ex_buf = __f0; __r };
                                                                                    while (self.ex_buf_xptr < self.ex_buf_ptr) {
                                                                                        {
                                                                                            { let __ix118 = (self.ex_buf_xptr).wrapping_sub(1i32); let __v119 = self.ex_buf[(self.ex_buf_xptr) as usize]; self.ex_buf[(__ix118) as usize] = __v119; }
                                                                                            self.ex_buf_xptr = (self.ex_buf_xptr).wrapping_add(1i32);
                                                                                        }
                                                                                    }
                                                                                    self.ex_buf_xptr = (self.ex_buf_xptr).wrapping_sub(1i32);
                                                                                    while ((self.ex_buf_ptr < self.ex_buf_length) && (self.lex_class[(self.ex_buf[(self.ex_buf_ptr) as usize]) as usize] == white_space)) {
                                                                                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                                                    }
                                                                                    self.tmp_ptr = self.ex_buf_ptr;
                                                                                    while (self.tmp_ptr < self.ex_buf_length) {
                                                                                        {
                                                                                            { let __ix120 = (self.tmp_ptr).wrapping_sub((self.ex_buf_ptr).wrapping_sub(self.ex_buf_xptr)); let __v121 = self.ex_buf[(self.tmp_ptr) as usize]; self.ex_buf[(__ix120) as usize] = __v121; }
                                                                                            self.tmp_ptr = (self.tmp_ptr).wrapping_add(1i32);
                                                                                        }
                                                                                    }
                                                                                    self.ex_buf_length = (self.tmp_ptr).wrapping_sub((self.ex_buf_ptr).wrapping_sub(self.ex_buf_xptr));
                                                                                    self.ex_buf_ptr = self.ex_buf_xptr;
                                                                                }
                                                                            }
                                                                            _ => {
                                                                                // §363
                                                                            }
                                                                        }
                                                                    }
                                                                    bad_conversion => {
                                                                    }
                                                                    _ => {
                                                                        self.case_conversion_confusion();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        // §362
                                                        self.ex_buf_xptr = self.ex_buf_ptr;
                                                        while (((self.ex_buf_ptr < self.ex_buf_length) && (self.brace_level > 0i32)) && (self.ex_buf[(self.ex_buf_ptr) as usize] != backslash)) {
                                                            {
                                                                if (self.ex_buf[(self.ex_buf_ptr) as usize] == right_brace) {
                                                                    self.brace_level = (self.brace_level).wrapping_sub(1i32);
                                                                } else {
                                                                    if (self.ex_buf[(self.ex_buf_ptr) as usize] == left_brace) {
                                                                        self.brace_level = (self.brace_level).wrapping_add(1i32);
                                                                    }
                                                                }
                                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                            }
                                                        }
                                                        // §366
                                                        {
                                                            match self.conversion_type {
                                                                title_lowers | all_lowers => {
                                                                    { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.lower_case(&mut __f0, self.ex_buf_xptr, (self.ex_buf_ptr).wrapping_sub(self.ex_buf_xptr)); self.ex_buf = __f0; __r };
                                                                }
                                                                all_uppers => {
                                                                    { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.upper_case(&mut __f0, self.ex_buf_xptr, (self.ex_buf_ptr).wrapping_sub(self.ex_buf_xptr)); self.ex_buf = __f0; __r };
                                                                }
                                                                bad_conversion => {
                                                                }
                                                                _ => {
                                                                    self.case_conversion_confusion();
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                // §362
                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_sub(1i32);
                                            }
                                        }
                                        // §361
                                        self.prev_colon = false;
                                    }
                                } else {
                                    if (self.ex_buf[(self.ex_buf_ptr) as usize] == right_brace) {
                                        {
                                            self.decr_brace_level(self.pop_lit2);
                                            self.prev_colon = false;
                                        }
                                    } else {
                                        if (self.brace_level == 0i32) {
                                            // §367
                                            {
                                                match self.conversion_type {
                                                    title_lowers => {
                                                        {
                                                            if (self.ex_buf_ptr == 0i32) {
                                                            } else {
                                                                if (self.prev_colon && (self.lex_class[(self.ex_buf[((self.ex_buf_ptr).wrapping_sub(1i32)) as usize]) as usize] == white_space)) {
                                                                } else {
                                                                    { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.lower_case(&mut __f0, self.ex_buf_ptr, 1i32); self.ex_buf = __f0; __r };
                                                                }
                                                            }
                                                            if (self.ex_buf[(self.ex_buf_ptr) as usize] == colon) {
                                                                self.prev_colon = true;
                                                            } else {
                                                                if (self.lex_class[(self.ex_buf[(self.ex_buf_ptr) as usize]) as usize] != white_space) {
                                                                    self.prev_colon = false;
                                                                }
                                                            }
                                                        }
                                                    }
                                                    all_lowers => {
                                                        { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.lower_case(&mut __f0, self.ex_buf_ptr, 1i32); self.ex_buf = __f0; __r };
                                                    }
                                                    all_uppers => {
                                                        { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.upper_case(&mut __f0, self.ex_buf_ptr, 1i32); self.ex_buf = __f0; __r };
                                                    }
                                                    bad_conversion => {
                                                    }
                                                    _ => {
                                                        self.case_conversion_confusion();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                // §361
                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                            }
                        }
                        self.check_brace_level(self.pop_lit2);
                    }
                    // §355
                    self.add_pool_buf_and_push();
                }
            }
        }
    }

    /// The `built_in` function {\.{chr.to.int\$}} pops the top (string)
    /// literal, makes sure it's a single character, converts it to the
    /// corresponding `ASCII_code` integer, and pushes this integer.  If the
    /// literal isn't an appropriate string, it complains and pushes the
    /// integer~0.
    /// @<`execute_fn`({\.{chr.to.int\$}})
    // §368
    pub fn x_chr_to_int(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        if (self.pop_typ1 != stk_str) {
            {
                self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_str);
                self.push_lit_stk(0i32, stk_int);
            }
        } else {
            if ((self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pop_lit1) as usize]) != 1i32) {
                {
                    {
                        {
                            let __w0 = b'"';
                            crate::system::wr_char(&mut self.log_file, __w0);
                        }
                        {
                            let __w0 = b'"';
                            crate::system::wr_char(&mut self.standard_output, __w0);
                        }
                    }
                    self.print_a_pool_str(self.pop_lit1);
                    {
                        {
                            {
                                crate::system::wr_str(&mut self.log_file, "\" isn't a single character");
                            }
                            {
                                crate::system::wr_str(&mut self.standard_output, "\" isn't a single character");
                            }
                        }
                        self.bst_ex_warn_print();
                    }
                    self.push_lit_stk(0i32, stk_int);
                }
            } else {
                self.push_lit_stk(self.str_pool[(self.str_start[(self.pop_lit1) as usize]) as usize], stk_int);
            }
        }
    }

    /// The `built_in` function {\.{cite\$}} pushes the appropriate string
    /// from `cite_list` onto the stack.
    /// @<`execute_fn`({\.{cite\$}})
    // §369
    pub fn x_cite(&mut self) {
        if (!self.mess_with_entries) {
            self.bst_cant_mess_with_entries_print();
        } else {
            self.push_lit_stk(self.cite_list[(self.cite_ptr) as usize], stk_str);
        }
    }

    /// The `built_in` function {\.{duplicate\$}} pops the top literal from
    /// the stack and pushes two copies of it.
    /// @<`execute_fn`({\.{duplicate\$}})
    // §370
    pub fn x_duplicate(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        if (self.pop_typ1 != stk_str) {
            {
                self.push_lit_stk(self.pop_lit1, self.pop_typ1);
                self.push_lit_stk(self.pop_lit1, self.pop_typ1);
            }
        } else {
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
                if (self.pop_lit1 < self.cmd_str_ptr) {
                    self.push_lit_stk(self.pop_lit1, self.pop_typ1);
                } else {
                    {
                        {
                            while ((self.pool_ptr).wrapping_add((self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.pop_lit1) as usize])) > self.pool_size) {
                                self.pool_overflow();
                            }
                        }
                        self.sp_ptr = self.str_start[(self.pop_lit1) as usize];
                        self.sp_end = self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize];
                        while (self.sp_ptr < self.sp_end) {
                            {
                                {
                                    { let __ix122 = self.pool_ptr; let __v123 = self.str_pool[(self.sp_ptr) as usize]; self.str_pool[(__ix122) as usize] = __v123; }
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                }
                                self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                            }
                        }
                        { let __a124_0 = self.make_string(); let __a124_1 = stk_str; self.push_lit_stk(__a124_0, __a124_1) };
                    }
                }
            }
        }
    }

    /// The `built_in` function {\.{empty\$}} pops the top literal and pushes
    /// the integer 1 if it's a missing field or a string having no
    /// non`white_space` characters, 0 otherwise.  If the literal isn't a
    /// missing field or a string, it complains and pushes 0.
    /// @<`execute_fn`({\.{empty\$}})
    // §371
    pub fn x_empty(&mut self) {
        'l_exit_f: {
            { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
            match self.pop_typ1 {
                stk_str => {
                    // §372
                    {
                        self.sp_ptr = self.str_start[(self.pop_lit1) as usize];
                        self.sp_end = self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize];
                        while (self.sp_ptr < self.sp_end) {
                            {
                                if (self.lex_class[(self.str_pool[(self.sp_ptr) as usize]) as usize] != white_space) {
                                    {
                                        self.push_lit_stk(0i32, stk_int);
                                        break 'l_exit_f;
                                    }
                                }
                                self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                            }
                        }
                        self.push_lit_stk(1i32, stk_int);
                    }
                }
                stk_field_missing => {
                    // §371
                    self.push_lit_stk(1i32, stk_int);
                }
                stk_empty => {
                    self.push_lit_stk(0i32, stk_int);
                }
                _ => {
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
                        self.push_lit_stk(0i32, stk_int);
                    }
                }
            }
        }
    }

    /// The `built_in` function {\.{format.name\$}} pops the top three
    /// literals (they are a string, an integer, and a string literal, in that
    /// order).  The last string literal represents a name list (each name
    /// corresponding to a person), the integer literal specifies which name
    /// to pick from this list, and the first string literal specifies how to
    /// format this name, as described in the \BibTeX\ documentation.
    /// Finally, this function pushes the formatted name.  If any of the types
    /// is incorrect, it complains and pushes the null string.
    // §373
    pub fn x_format_name(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit1); let mut __f1 = ::core::mem::take(&mut self.pop_typ1); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit1 = __f0; self.pop_typ1 = __f1; __r };
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit2); let mut __f1 = ::core::mem::take(&mut self.pop_typ2); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit2 = __f0; self.pop_typ2 = __f1; __r };
        { let mut __f0 = ::core::mem::take(&mut self.pop_lit3); let mut __f1 = ::core::mem::take(&mut self.pop_typ3); let __r = self.pop_lit_stk(&mut __f0, &mut __f1); self.pop_lit3 = __f0; self.pop_typ3 = __f1; __r };
        if (self.pop_typ1 != stk_str) {
            {
                self.print_wrong_stk_lit(self.pop_lit1, self.pop_typ1, stk_str);
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
                        self.ex_buf_length = 0i32;
                        self.add_buf_pool(self.pop_lit3);
                        // §374
                        {
                            self.ex_buf_ptr = 0i32;
                            self.num_names = 0i32;
                            while ((self.num_names < self.pop_lit2) && (self.ex_buf_ptr < self.ex_buf_length)) {
                                {
                                    self.num_names = (self.num_names).wrapping_add(1i32);
                                    self.ex_buf_xptr = self.ex_buf_ptr;
                                    self.name_scan_for_and(self.pop_lit3);
                                }
                            }
                            if (self.ex_buf_ptr < self.ex_buf_length) {
                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_sub(4i32);
                            }
                            if (self.num_names < self.pop_lit2) {
                                {
                                    if (self.pop_lit2 == 1i32) {
                                        {
                                            {
                                                crate::system::wr_str(&mut self.log_file, "There is no name in \"");
                                            }
                                            {
                                                crate::system::wr_str(&mut self.standard_output, "There is no name in \"");
                                            }
                                        }
                                    } else {
                                        {
                                            {
                                                let __w1 = self.pop_lit2;
                                                crate::system::wr_str(&mut self.log_file, "There aren't ");
                                                crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                                                crate::system::wr_str(&mut self.log_file, " names in \"");
                                            }
                                            {
                                                let __w1 = self.pop_lit2;
                                                crate::system::wr_str(&mut self.standard_output, "There aren't ");
                                                crate::system::wr_int(&mut self.standard_output, __w1, 0i32);
                                                crate::system::wr_str(&mut self.standard_output, " names in \"");
                                            }
                                        }
                                    }
                                    self.print_a_pool_str(self.pop_lit3);
                                    {
                                        {
                                            {
                                                let __w0 = b'"';
                                                crate::system::wr_char(&mut self.log_file, __w0);
                                            }
                                            {
                                                let __w0 = b'"';
                                                crate::system::wr_char(&mut self.standard_output, __w0);
                                            }
                                        }
                                        self.bst_ex_warn_print();
                                    }
                                }
                            }
                        }
                        // §378
                        {
                            // §379
                            {
                                'l_L16_f: {
                                    while (self.ex_buf_ptr > self.ex_buf_xptr) {
                                        match self.lex_class[(self.ex_buf[((self.ex_buf_ptr).wrapping_sub(1i32)) as usize]) as usize] {
                                            white_space | sep_char => {
                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_sub(1i32);
                                            }
                                            _ => {
                                                if (self.ex_buf[((self.ex_buf_ptr).wrapping_sub(1i32)) as usize] == comma) {
                                                    {
                                                        {
                                                            {
                                                                let __w1 = self.pop_lit2;
                                                                crate::system::wr_str(&mut self.log_file, "Name ");
                                                                crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                                                                crate::system::wr_str(&mut self.log_file, " in \"");
                                                            }
                                                            {
                                                                let __w1 = self.pop_lit2;
                                                                crate::system::wr_str(&mut self.standard_output, "Name ");
                                                                crate::system::wr_int(&mut self.standard_output, __w1, 0i32);
                                                                crate::system::wr_str(&mut self.standard_output, " in \"");
                                                            }
                                                        }
                                                        self.print_a_pool_str(self.pop_lit3);
                                                        {
                                                            {
                                                                crate::system::wr_str(&mut self.log_file, "\" has a comma at the end");
                                                            }
                                                            {
                                                                crate::system::wr_str(&mut self.standard_output, "\" has a comma at the end");
                                                            }
                                                        }
                                                        self.bst_ex_warn_print();
                                                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_sub(1i32);
                                                    }
                                                } else {
                                                    break 'l_L16_f;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // §378
                            self.name_bf_ptr = 0i32;
                            self.num_commas = 0i32;
                            self.num_tokens = 0i32;
                            self.token_starting = true;
                            while (self.ex_buf_xptr < self.ex_buf_ptr) {
                                match self.ex_buf[(self.ex_buf_xptr) as usize] {
                                    comma => {
                                        // §380
                                        {
                                            if (self.num_commas == 2i32) {
                                                {
                                                    {
                                                        {
                                                            let __w1 = self.pop_lit2;
                                                            crate::system::wr_str(&mut self.log_file, "Too many commas in name ");
                                                            crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                                                            crate::system::wr_str(&mut self.log_file, " of \"");
                                                        }
                                                        {
                                                            let __w1 = self.pop_lit2;
                                                            crate::system::wr_str(&mut self.standard_output, "Too many commas in name ");
                                                            crate::system::wr_int(&mut self.standard_output, __w1, 0i32);
                                                            crate::system::wr_str(&mut self.standard_output, " of \"");
                                                        }
                                                    }
                                                    self.print_a_pool_str(self.pop_lit3);
                                                    {
                                                        {
                                                            let __w0 = b'"';
                                                            crate::system::wr_char(&mut self.log_file, __w0);
                                                        }
                                                        {
                                                            let __w0 = b'"';
                                                            crate::system::wr_char(&mut self.standard_output, __w0);
                                                        }
                                                    }
                                                    self.bst_ex_warn_print();
                                                }
                                            } else {
                                                {
                                                    self.num_commas = (self.num_commas).wrapping_add(1i32);
                                                    if (self.num_commas == 1i32) {
                                                        self.comma1 = self.num_tokens;
                                                    } else {
                                                        self.comma2 = self.num_tokens;
                                                    }
                                                    self.name_sep_char[(self.num_tokens) as usize] = comma;
                                                }
                                            }
                                            self.ex_buf_xptr = (self.ex_buf_xptr).wrapping_add(1i32);
                                            self.token_starting = true;
                                        }
                                    }
                                    left_brace => {
                                        // §381
                                        {
                                            self.brace_level = (self.brace_level).wrapping_add(1i32);
                                            if self.token_starting {
                                                {
                                                    { let __ix125 = self.num_tokens; let __v126 = self.name_bf_ptr; self.name_tok[(__ix125) as usize] = __v126; }
                                                    self.num_tokens = (self.num_tokens).wrapping_add(1i32);
                                                }
                                            }
                                            { let __ix127 = self.name_bf_ptr; let __v128 = self.ex_buf[(self.ex_buf_xptr) as usize]; self.sv_buffer[(__ix127) as usize] = __v128; }
                                            self.name_bf_ptr = (self.name_bf_ptr).wrapping_add(1i32);
                                            self.ex_buf_xptr = (self.ex_buf_xptr).wrapping_add(1i32);
                                            while ((self.brace_level > 0i32) && (self.ex_buf_xptr < self.ex_buf_ptr)) {
                                                {
                                                    if (self.ex_buf[(self.ex_buf_xptr) as usize] == right_brace) {
                                                        self.brace_level = (self.brace_level).wrapping_sub(1i32);
                                                    } else {
                                                        if (self.ex_buf[(self.ex_buf_xptr) as usize] == left_brace) {
                                                            self.brace_level = (self.brace_level).wrapping_add(1i32);
                                                        }
                                                    }
                                                    { let __ix129 = self.name_bf_ptr; let __v130 = self.ex_buf[(self.ex_buf_xptr) as usize]; self.sv_buffer[(__ix129) as usize] = __v130; }
                                                    self.name_bf_ptr = (self.name_bf_ptr).wrapping_add(1i32);
                                                    self.ex_buf_xptr = (self.ex_buf_xptr).wrapping_add(1i32);
                                                }
                                            }
                                            self.token_starting = false;
                                        }
                                    }
                                    right_brace => {
                                        // §382
                                        {
                                            if self.token_starting {
                                                {
                                                    { let __ix131 = self.num_tokens; let __v132 = self.name_bf_ptr; self.name_tok[(__ix131) as usize] = __v132; }
                                                    self.num_tokens = (self.num_tokens).wrapping_add(1i32);
                                                }
                                            }
                                            {
                                                {
                                                    let __w1 = self.pop_lit2;
                                                    crate::system::wr_str(&mut self.log_file, "Name ");
                                                    crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                                                    crate::system::wr_str(&mut self.log_file, " of \"");
                                                }
                                                {
                                                    let __w1 = self.pop_lit2;
                                                    crate::system::wr_str(&mut self.standard_output, "Name ");
                                                    crate::system::wr_int(&mut self.standard_output, __w1, 0i32);
                                                    crate::system::wr_str(&mut self.standard_output, " of \"");
                                                }
                                            }
                                            self.print_a_pool_str(self.pop_lit3);
                                            {
                                                {
                                                    {
                                                        crate::system::wr_str(&mut self.log_file, "\" isn't brace balanced");
                                                    }
                                                    {
                                                        crate::system::wr_str(&mut self.standard_output, "\" isn't brace balanced");
                                                    }
                                                }
                                                self.bst_ex_warn_print();
                                            }
                                            self.ex_buf_xptr = (self.ex_buf_xptr).wrapping_add(1i32);
                                            self.token_starting = false;
                                        }
                                    }
                                    _ => {
                                        // §378
                                        match self.lex_class[(self.ex_buf[(self.ex_buf_xptr) as usize]) as usize] {
                                            white_space => {
                                                // §383
                                                {
                                                    if (!self.token_starting) {
                                                        self.name_sep_char[(self.num_tokens) as usize] = space;
                                                    }
                                                    self.ex_buf_xptr = (self.ex_buf_xptr).wrapping_add(1i32);
                                                    self.token_starting = true;
                                                }
                                            }
                                            sep_char => {
                                                // §384
                                                {
                                                    if (!self.token_starting) {
                                                        { let __ix133 = self.num_tokens; let __v134 = self.ex_buf[(self.ex_buf_xptr) as usize]; self.name_sep_char[(__ix133) as usize] = __v134; }
                                                    }
                                                    self.ex_buf_xptr = (self.ex_buf_xptr).wrapping_add(1i32);
                                                    self.token_starting = true;
                                                }
                                            }
                                            _ => {
                                                // §385
                                                {
                                                    if self.token_starting {
                                                        {
                                                            { let __ix135 = self.num_tokens; let __v136 = self.name_bf_ptr; self.name_tok[(__ix135) as usize] = __v136; }
                                                            self.num_tokens = (self.num_tokens).wrapping_add(1i32);
                                                        }
                                                    }
                                                    { let __ix137 = self.name_bf_ptr; let __v138 = self.ex_buf[(self.ex_buf_xptr) as usize]; self.sv_buffer[(__ix137) as usize] = __v138; }
                                                    self.name_bf_ptr = (self.name_bf_ptr).wrapping_add(1i32);
                                                    self.ex_buf_xptr = (self.ex_buf_xptr).wrapping_add(1i32);
                                                    self.token_starting = false;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // §378
                            { let __ix139 = self.num_tokens; let __v140 = self.name_bf_ptr; self.name_tok[(__ix139) as usize] = __v140; }
                        }
                        // §386
                        {
                            if (self.num_commas == 0i32) {
                                {
                                    self.first_start = 0i32;
                                    self.last_end = self.num_tokens;
                                    self.jr_end = self.last_end;
                                    // §387
                                    {
                                        'l_L52_f: {
                                            'l_L17_f: {
                                                self.von_start = 0i32;
                                                while (self.von_start < (self.last_end).wrapping_sub(1i32)) {
                                                    {
                                                        self.name_bf_ptr = self.name_tok[(self.von_start) as usize];
                                                        self.name_bf_xptr = self.name_tok[((self.von_start).wrapping_add(1i32)) as usize];
                                                        if self.von_token_found() {
                                                            {
                                                                self.von_name_ends_and_last_name_starts_stuff();
                                                                break 'l_L52_f;
                                                            }
                                                        }
                                                        self.von_start = (self.von_start).wrapping_add(1i32);
                                                    }
                                                }
                                                while (self.von_start > 0i32) {
                                                    {
                                                        if ((self.lex_class[(self.name_sep_char[(self.von_start) as usize]) as usize] != sep_char) || (self.name_sep_char[(self.von_start) as usize] == tie)) {
                                                            break 'l_L17_f;
                                                        }
                                                        self.von_start = (self.von_start).wrapping_sub(1i32);
                                                    }
                                                }
                                            }
                                            self.von_end = self.von_start;
                                        }
                                        self.first_end = self.von_start;
                                    }
                                }
                            } else {
                                // §386
                                if (self.num_commas == 1i32) {
                                    {
                                        self.von_start = 0i32;
                                        self.last_end = self.comma1;
                                        self.jr_end = self.last_end;
                                        self.first_start = self.jr_end;
                                        self.first_end = self.num_tokens;
                                        self.von_name_ends_and_last_name_starts_stuff();
                                    }
                                } else {
                                    if (self.num_commas == 2i32) {
                                        {
                                            self.von_start = 0i32;
                                            self.last_end = self.comma1;
                                            self.jr_end = self.comma2;
                                            self.first_start = self.jr_end;
                                            self.first_end = self.num_tokens;
                                            self.von_name_ends_and_last_name_starts_stuff();
                                        }
                                    } else {
                                        {
                                            {
                                                {
                                                    crate::system::wr_str(&mut self.log_file, "Illegal number of comma,s");
                                                }
                                                {
                                                    crate::system::wr_str(&mut self.standard_output, "Illegal number of comma,s");
                                                }
                                            }
                                            self.print_confusion();
                                            crate::system::end_of_TEX(self);
                                        }
                                    }
                                }
                            }
                        }
                        // §373
                        self.ex_buf_length = 0i32;
                        self.add_buf_pool(self.pop_lit1);
                        self.figure_out_the_formatted_name();
                        self.add_pool_buf_and_push();
                    }
                }
            }
        }
    }

}
