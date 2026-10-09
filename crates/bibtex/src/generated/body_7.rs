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
    /// An \.{entry} command has three arguments, each a (possibly empty) list
    /// of function names between braces (the names are separated by one or
    /// more `white_space` characters).  All function names in this and other
    /// commands must be legal \.{.bst} identifiers.  Upper/lower cases are
    /// considered to be the same for function names in these lists---all
    /// upper-case letters are converted to lower case.  These arguments give
    /// lists of `field`s, `int_entry_var`s, and `str_entry_var`s.
    /// @<Procedures and functions for the reading and processing of input files
    // §162
    pub fn bst_entry_command(&mut self) {
        'l_exit_f: {
            if self.entry_seen {
                {
                    {
                        {
                            crate::system::wr_str(
                                &mut self.log_file,
                                "Illegal, another entry command",
                            );
                        }
                        {
                            crate::system::wr_str(
                                &mut self.standard_output,
                                "Illegal, another entry command",
                            );
                        }
                    }
                    {
                        self.bst_err_print_and_look_for_blank_line();
                        break 'l_exit_f;
                    }
                }
            }
            self.entry_seen = true;
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "entry");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "entry");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            // §163
            {
                {
                    if (self.buffer[(self.buf_ptr2) as usize] != left_brace) {
                        {
                            self.bst_left_brace_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "entry");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "entry");
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                }
                {
                    if (!self.eat_bst_white_space()) {
                        {
                            self.eat_bst_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "entry");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "entry");
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                }
                while (self.buffer[(self.buf_ptr2) as usize] != right_brace) {
                    {
                        {
                            self.scan_identifier(right_brace, comment, comment);
                            if ((self.scan_result == white_adjacent)
                                || (self.scan_result == specified_char_adjacent))
                            {
                            } else {
                                {
                                    self.bst_id_print();
                                    {
                                        {
                                            {
                                                crate::system::wr_str(&mut self.log_file, "entry");
                                            }
                                            {
                                                crate::system::wr_str(
                                                    &mut self.standard_output,
                                                    "entry",
                                                );
                                            }
                                        }
                                        {
                                            self.bst_err_print_and_look_for_blank_line();
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                            }
                        }
                        // §164
                        {
                            {
                                let mut __f0 = ::core::mem::take(&mut self.buffer);
                                let __r = self.lower_case(
                                    &mut __f0,
                                    self.buf_ptr1,
                                    (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                                );
                                self.buffer = __f0;
                                __r
                            };
                            self.fn_loc = {
                                let mut __f0 = ::core::mem::take(&mut self.buffer);
                                let __r = self.str_lookup(
                                    &mut __f0,
                                    self.buf_ptr1,
                                    (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                                    bst_fn_ilk,
                                    true,
                                );
                                self.buffer = __f0;
                                __r
                            };
                            {
                                if self.hash_found {
                                    {
                                        self.already_seen_function_print(self.fn_loc);
                                        break 'l_exit_f;
                                    }
                                }
                            }
                            self.fn_type[(self.fn_loc) as usize] = field;
                            {
                                let __ix194 = self.fn_loc;
                                let __v195 = self.num_fields;
                                self.ilk_info[(__ix194) as usize] = __v195;
                            }
                            self.num_fields = (self.num_fields).wrapping_add(1i32);
                        }
                        // §163
                        {
                            if (!self.eat_bst_white_space()) {
                                {
                                    self.eat_bst_print();
                                    {
                                        {
                                            {
                                                crate::system::wr_str(&mut self.log_file, "entry");
                                            }
                                            {
                                                crate::system::wr_str(
                                                    &mut self.standard_output,
                                                    "entry",
                                                );
                                            }
                                        }
                                        {
                                            self.bst_err_print_and_look_for_blank_line();
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            }
            // §162
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "entry");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "entry");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            if (self.num_fields == self.num_pre_defined_fields) {
                {
                    {
                        {
                            crate::system::wr_str(
                                &mut self.log_file,
                                "Warning--I didn't find any fields",
                            );
                        }
                        {
                            crate::system::wr_str(
                                &mut self.standard_output,
                                "Warning--I didn't find any fields",
                            );
                        }
                    }
                    self.bst_warn_print();
                }
            }
            // §165
            {
                {
                    if (self.buffer[(self.buf_ptr2) as usize] != left_brace) {
                        {
                            self.bst_left_brace_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "entry");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "entry");
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                }
                {
                    if (!self.eat_bst_white_space()) {
                        {
                            self.eat_bst_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "entry");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "entry");
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                }
                while (self.buffer[(self.buf_ptr2) as usize] != right_brace) {
                    {
                        {
                            self.scan_identifier(right_brace, comment, comment);
                            if ((self.scan_result == white_adjacent)
                                || (self.scan_result == specified_char_adjacent))
                            {
                            } else {
                                {
                                    self.bst_id_print();
                                    {
                                        {
                                            {
                                                crate::system::wr_str(&mut self.log_file, "entry");
                                            }
                                            {
                                                crate::system::wr_str(
                                                    &mut self.standard_output,
                                                    "entry",
                                                );
                                            }
                                        }
                                        {
                                            self.bst_err_print_and_look_for_blank_line();
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                            }
                        }
                        // §166
                        {
                            {
                                let mut __f0 = ::core::mem::take(&mut self.buffer);
                                let __r = self.lower_case(
                                    &mut __f0,
                                    self.buf_ptr1,
                                    (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                                );
                                self.buffer = __f0;
                                __r
                            };
                            self.fn_loc = {
                                let mut __f0 = ::core::mem::take(&mut self.buffer);
                                let __r = self.str_lookup(
                                    &mut __f0,
                                    self.buf_ptr1,
                                    (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                                    bst_fn_ilk,
                                    true,
                                );
                                self.buffer = __f0;
                                __r
                            };
                            {
                                if self.hash_found {
                                    {
                                        self.already_seen_function_print(self.fn_loc);
                                        break 'l_exit_f;
                                    }
                                }
                            }
                            self.fn_type[(self.fn_loc) as usize] = int_entry_var;
                            {
                                let __ix196 = self.fn_loc;
                                let __v197 = self.num_ent_ints;
                                self.ilk_info[(__ix196) as usize] = __v197;
                            }
                            self.num_ent_ints = (self.num_ent_ints).wrapping_add(1i32);
                        }
                        // §165
                        {
                            if (!self.eat_bst_white_space()) {
                                {
                                    self.eat_bst_print();
                                    {
                                        {
                                            {
                                                crate::system::wr_str(&mut self.log_file, "entry");
                                            }
                                            {
                                                crate::system::wr_str(
                                                    &mut self.standard_output,
                                                    "entry",
                                                );
                                            }
                                        }
                                        {
                                            self.bst_err_print_and_look_for_blank_line();
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            }
            // §162
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "entry");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "entry");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            // §167
            {
                {
                    if (self.buffer[(self.buf_ptr2) as usize] != left_brace) {
                        {
                            self.bst_left_brace_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "entry");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "entry");
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                }
                {
                    if (!self.eat_bst_white_space()) {
                        {
                            self.eat_bst_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "entry");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "entry");
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                }
                while (self.buffer[(self.buf_ptr2) as usize] != right_brace) {
                    {
                        {
                            self.scan_identifier(right_brace, comment, comment);
                            if ((self.scan_result == white_adjacent)
                                || (self.scan_result == specified_char_adjacent))
                            {
                            } else {
                                {
                                    self.bst_id_print();
                                    {
                                        {
                                            {
                                                crate::system::wr_str(&mut self.log_file, "entry");
                                            }
                                            {
                                                crate::system::wr_str(
                                                    &mut self.standard_output,
                                                    "entry",
                                                );
                                            }
                                        }
                                        {
                                            self.bst_err_print_and_look_for_blank_line();
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                            }
                        }
                        // §168
                        {
                            {
                                let mut __f0 = ::core::mem::take(&mut self.buffer);
                                let __r = self.lower_case(
                                    &mut __f0,
                                    self.buf_ptr1,
                                    (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                                );
                                self.buffer = __f0;
                                __r
                            };
                            self.fn_loc = {
                                let mut __f0 = ::core::mem::take(&mut self.buffer);
                                let __r = self.str_lookup(
                                    &mut __f0,
                                    self.buf_ptr1,
                                    (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                                    bst_fn_ilk,
                                    true,
                                );
                                self.buffer = __f0;
                                __r
                            };
                            {
                                if self.hash_found {
                                    {
                                        self.already_seen_function_print(self.fn_loc);
                                        break 'l_exit_f;
                                    }
                                }
                            }
                            self.fn_type[(self.fn_loc) as usize] = str_entry_var;
                            {
                                let __ix198 = self.fn_loc;
                                let __v199 = self.num_ent_strs;
                                self.ilk_info[(__ix198) as usize] = __v199;
                            }
                            self.num_ent_strs = (self.num_ent_strs).wrapping_add(1i32);
                        }
                        // §167
                        {
                            if (!self.eat_bst_white_space()) {
                                {
                                    self.eat_bst_print();
                                    {
                                        {
                                            {
                                                crate::system::wr_str(&mut self.log_file, "entry");
                                            }
                                            {
                                                crate::system::wr_str(
                                                    &mut self.standard_output,
                                                    "entry",
                                                );
                                            }
                                        }
                                        {
                                            self.bst_err_print_and_look_for_blank_line();
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            }
        }
        // §162
    }

    /// A legal argument for an \.{execute}, \.{iterate}, or \.{reverse}
    /// command must exist and be `built_in` or `wiz_defined`.
    /// Here's where we check, returning `true` if the argument is illegal.
    /// @<Procedures and functions for the reading and processing of input files
    // §169
    pub fn bad_argument_token(&mut self) -> bool {
        let mut bad_argument_token: bool = false;
        'l_exit_f: {
            bad_argument_token = true;
            {
                let mut __f0 = ::core::mem::take(&mut self.buffer);
                let __r = self.lower_case(
                    &mut __f0,
                    self.buf_ptr1,
                    (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                );
                self.buffer = __f0;
                __r
            };
            self.fn_loc = {
                let mut __f0 = ::core::mem::take(&mut self.buffer);
                let __r = self.str_lookup(
                    &mut __f0,
                    self.buf_ptr1,
                    (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                    bst_fn_ilk,
                    false,
                );
                self.buffer = __f0;
                __r
            };
            if (!self.hash_found) {
                {
                    self.print_a_token();
                    {
                        {
                            {
                                crate::system::wr_str(
                                    &mut self.log_file,
                                    " is an unknown function",
                                );
                            }
                            {
                                crate::system::wr_str(
                                    &mut self.standard_output,
                                    " is an unknown function",
                                );
                            }
                        }
                        {
                            self.bst_err_print_and_look_for_blank_line();
                            break 'l_exit_f;
                        }
                    }
                }
            } else {
                if ((self.fn_type[(self.fn_loc) as usize] != built_in)
                    && (self.fn_type[(self.fn_loc) as usize] != wiz_defined))
                {
                    {
                        self.print_a_token();
                        {
                            {
                                crate::system::wr_str(
                                    &mut self.log_file,
                                    " has bad function type ",
                                );
                            }
                            {
                                crate::system::wr_str(
                                    &mut self.standard_output,
                                    " has bad function type ",
                                );
                            }
                        }
                        self.print_fn_class(self.fn_loc);
                        {
                            self.bst_err_print_and_look_for_blank_line();
                            break 'l_exit_f;
                        }
                    }
                }
            }
            bad_argument_token = false;
        }
        bad_argument_token
    }

    /// An \.{execute} command has one argument, a single `built_in` or
    /// `wiz_defined` function name between braces.  Upper/lower cases are
    /// considered to be the same---all upper-case letters are converted to
    /// lower case.  Also, we must make sure we've already seen a \.{read}
    /// command.
    /// This module reads a `left_brace`, a single function to be executed,
    /// and a `right_brace`.
    /// @<Procedures and functions for the reading and processing of input files
    // §170
    pub fn bst_execute_command(&mut self) {
        'l_exit_f: {
            if (!self.read_seen) {
                {
                    {
                        {
                            crate::system::wr_str(
                                &mut self.log_file,
                                "Illegal, execute command before read command",
                            );
                        }
                        {
                            crate::system::wr_str(
                                &mut self.standard_output,
                                "Illegal, execute command before read command",
                            );
                        }
                    }
                    {
                        self.bst_err_print_and_look_for_blank_line();
                        break 'l_exit_f;
                    }
                }
            }
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "execute");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "execute");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            {
                if (self.buffer[(self.buf_ptr2) as usize] != left_brace) {
                    {
                        self.bst_left_brace_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "execute");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "execute");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            }
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "execute");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "execute");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            {
                self.scan_identifier(right_brace, comment, comment);
                if ((self.scan_result == white_adjacent)
                    || (self.scan_result == specified_char_adjacent))
                {
                } else {
                    {
                        self.bst_id_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "execute");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "execute");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            // §171
            {
                if self.bad_argument_token() {
                    break 'l_exit_f;
                }
            }
            // §170
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "execute");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "execute");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            {
                if (self.buffer[(self.buf_ptr2) as usize] != right_brace) {
                    {
                        self.bst_right_brace_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "execute");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "execute");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            }
            // §287
            {
                self.init_command_execution();
                self.mess_with_entries = false;
                self.execute_fn(self.fn_loc);
                self.check_command_execution();
            }
        }
        // §170
    }

    /// A \.{function} command has two arguments; the first is a
    /// `wiz_defined` function name between braces.  Upper/lower cases are
    /// considered to be the same---all upper-case letters are converted to
    /// lower case.  The second argument defines this function.  It consists
    /// of a sequence of functions, between braces, separated by `white_space`
    /// characters.  Upper/lower cases are considered to be the same for
    /// function names but not for `str_literal`s.
    /// @<Procedures and functions for the reading and processing of input files
    // §172
    pub fn bst_function_command(&mut self) {
        'l_exit_f: {
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "function");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "function");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            // §173
            {
                {
                    if (self.buffer[(self.buf_ptr2) as usize] != left_brace) {
                        {
                            self.bst_left_brace_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "function");
                                    }
                                    {
                                        crate::system::wr_str(
                                            &mut self.standard_output,
                                            "function",
                                        );
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                }
                {
                    if (!self.eat_bst_white_space()) {
                        {
                            self.eat_bst_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "function");
                                    }
                                    {
                                        crate::system::wr_str(
                                            &mut self.standard_output,
                                            "function",
                                        );
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                }
                {
                    self.scan_identifier(right_brace, comment, comment);
                    if ((self.scan_result == white_adjacent)
                        || (self.scan_result == specified_char_adjacent))
                    {
                    } else {
                        {
                            self.bst_id_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "function");
                                    }
                                    {
                                        crate::system::wr_str(
                                            &mut self.standard_output,
                                            "function",
                                        );
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                }
                // §174
                {
                    {
                        let mut __f0 = ::core::mem::take(&mut self.buffer);
                        let __r = self.lower_case(
                            &mut __f0,
                            self.buf_ptr1,
                            (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                        );
                        self.buffer = __f0;
                        __r
                    };
                    self.wiz_loc = {
                        let mut __f0 = ::core::mem::take(&mut self.buffer);
                        let __r = self.str_lookup(
                            &mut __f0,
                            self.buf_ptr1,
                            (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                            bst_fn_ilk,
                            true,
                        );
                        self.buffer = __f0;
                        __r
                    };
                    {
                        if self.hash_found {
                            {
                                self.already_seen_function_print(self.wiz_loc);
                                break 'l_exit_f;
                            }
                        }
                    }
                    self.fn_type[(self.wiz_loc) as usize] = wiz_defined;
                    if (self.hash_text[(self.wiz_loc) as usize] == self.s_default) {
                        self.b_default = self.wiz_loc;
                    }
                }
                // §173
                {
                    if (!self.eat_bst_white_space()) {
                        {
                            self.eat_bst_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "function");
                                    }
                                    {
                                        crate::system::wr_str(
                                            &mut self.standard_output,
                                            "function",
                                        );
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                }
                {
                    if (self.buffer[(self.buf_ptr2) as usize] != right_brace) {
                        {
                            self.bst_right_brace_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "function");
                                    }
                                    {
                                        crate::system::wr_str(
                                            &mut self.standard_output,
                                            "function",
                                        );
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                }
            }
            // §172
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "function");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "function");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            {
                if (self.buffer[(self.buf_ptr2) as usize] != left_brace) {
                    {
                        self.bst_left_brace_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "function");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "function");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            }
            self.scan_fn_def(self.wiz_loc);
        }
    }

    /// An \.{integers} command has one argument, a list of function names
    /// between braces (the names are separated by one or more `white_space`
    /// characters).  Upper/lower cases are considered to be the same for
    /// function names in these lists---all upper-case letters are converted to
    /// lower case.  Each name in this list specifies an `int_global_var`.
    /// There may be several \.{integers} commands in the \.{.bst} file.
    /// This module reads a `left_brace`, a list of `int_global_var`s, and a
    /// `right_brace`.
    /// @<Procedures and functions for the reading and processing of input files
    // §193
    pub fn bst_integers_command(&mut self) {
        'l_exit_f: {
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "integers");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "integers");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            {
                if (self.buffer[(self.buf_ptr2) as usize] != left_brace) {
                    {
                        self.bst_left_brace_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "integers");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "integers");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            }
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "integers");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "integers");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            while (self.buffer[(self.buf_ptr2) as usize] != right_brace) {
                {
                    {
                        self.scan_identifier(right_brace, comment, comment);
                        if ((self.scan_result == white_adjacent)
                            || (self.scan_result == specified_char_adjacent))
                        {
                        } else {
                            {
                                self.bst_id_print();
                                {
                                    {
                                        {
                                            crate::system::wr_str(&mut self.log_file, "integers");
                                        }
                                        {
                                            crate::system::wr_str(
                                                &mut self.standard_output,
                                                "integers",
                                            );
                                        }
                                    }
                                    {
                                        self.bst_err_print_and_look_for_blank_line();
                                        break 'l_exit_f;
                                    }
                                }
                            }
                        }
                    }
                    // §194
                    {
                        {
                            let mut __f0 = ::core::mem::take(&mut self.buffer);
                            let __r = self.lower_case(
                                &mut __f0,
                                self.buf_ptr1,
                                (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                            );
                            self.buffer = __f0;
                            __r
                        };
                        self.fn_loc = {
                            let mut __f0 = ::core::mem::take(&mut self.buffer);
                            let __r = self.str_lookup(
                                &mut __f0,
                                self.buf_ptr1,
                                (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                                bst_fn_ilk,
                                true,
                            );
                            self.buffer = __f0;
                            __r
                        };
                        {
                            if self.hash_found {
                                {
                                    self.already_seen_function_print(self.fn_loc);
                                    break 'l_exit_f;
                                }
                            }
                        }
                        self.fn_type[(self.fn_loc) as usize] = int_global_var;
                        self.ilk_info[(self.fn_loc) as usize] = 0i32;
                    }
                    // §193
                    {
                        if (!self.eat_bst_white_space()) {
                            {
                                self.eat_bst_print();
                                {
                                    {
                                        {
                                            crate::system::wr_str(&mut self.log_file, "integers");
                                        }
                                        {
                                            crate::system::wr_str(
                                                &mut self.standard_output,
                                                "integers",
                                            );
                                        }
                                    }
                                    {
                                        self.bst_err_print_and_look_for_blank_line();
                                        break 'l_exit_f;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
        }
    }

    /// An \.{iterate} command has one argument, a single `built_in` or
    /// `wiz_defined` function name between braces.  Upper/lower cases are
    /// considered to be the same---all upper-case letters are converted to
    /// lower case.  Also, we must make sure we've already seen a \.{read}
    /// command.
    /// This module reads a `left_brace`, a single function to be iterated,
    /// and a `right_brace`.
    /// @<Procedures and functions for the reading and processing of input files
    // §195
    pub fn bst_iterate_command(&mut self) {
        'l_exit_f: {
            if (!self.read_seen) {
                {
                    {
                        {
                            crate::system::wr_str(
                                &mut self.log_file,
                                "Illegal, iterate command before read command",
                            );
                        }
                        {
                            crate::system::wr_str(
                                &mut self.standard_output,
                                "Illegal, iterate command before read command",
                            );
                        }
                    }
                    {
                        self.bst_err_print_and_look_for_blank_line();
                        break 'l_exit_f;
                    }
                }
            }
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "iterate");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "iterate");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            {
                if (self.buffer[(self.buf_ptr2) as usize] != left_brace) {
                    {
                        self.bst_left_brace_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "iterate");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "iterate");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            }
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "iterate");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "iterate");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            {
                self.scan_identifier(right_brace, comment, comment);
                if ((self.scan_result == white_adjacent)
                    || (self.scan_result == specified_char_adjacent))
                {
                } else {
                    {
                        self.bst_id_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "iterate");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "iterate");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            // §196
            {
                if self.bad_argument_token() {
                    break 'l_exit_f;
                }
            }
            // §195
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "iterate");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "iterate");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            {
                if (self.buffer[(self.buf_ptr2) as usize] != right_brace) {
                    {
                        self.bst_right_brace_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "iterate");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "iterate");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            }
            // §288
            {
                self.init_command_execution();
                self.mess_with_entries = true;
                self.sort_cite_ptr = 0i32;
                while (self.sort_cite_ptr < self.num_cites) {
                    {
                        self.cite_ptr = self.cite_info[(self.sort_cite_ptr) as usize];
                        self.execute_fn(self.fn_loc);
                        self.check_command_execution();
                        self.sort_cite_ptr = (self.sort_cite_ptr).wrapping_add(1i32);
                    }
                }
            }
        }
        // §195
    }

    /// A \.{macro} command, like a \.{function} command, has two arguments;
    /// the first is a macro name between braces.  The name must be a legal
    /// \.{.bst} identifier.  Upper/lower cases are considered to be the
    /// same---all upper-case letters are converted to lower case.  The second
    /// argument defines this macro.  It consists of a
    /// `double_quote`-delimited string (which must be on a single line)
    /// between braces, with optional `white_space` characters between the
    /// braces and the `double_quote`s.  This `double_quote`-delimited string
    /// is parsed exactly as a `str_literal` is for the \.{function} command.
    /// @<Procedures and functions for the reading and processing of input files
    // §197
    pub fn bst_macro_command(&mut self) {
        'l_exit_f: {
            if self.read_seen {
                {
                    {
                        {
                            crate::system::wr_str(
                                &mut self.log_file,
                                "Illegal, macro command after read command",
                            );
                        }
                        {
                            crate::system::wr_str(
                                &mut self.standard_output,
                                "Illegal, macro command after read command",
                            );
                        }
                    }
                    {
                        self.bst_err_print_and_look_for_blank_line();
                        break 'l_exit_f;
                    }
                }
            }
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "macro");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "macro");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            // §198
            {
                {
                    if (self.buffer[(self.buf_ptr2) as usize] != left_brace) {
                        {
                            self.bst_left_brace_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "macro");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "macro");
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                }
                {
                    if (!self.eat_bst_white_space()) {
                        {
                            self.eat_bst_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "macro");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "macro");
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                }
                {
                    self.scan_identifier(right_brace, comment, comment);
                    if ((self.scan_result == white_adjacent)
                        || (self.scan_result == specified_char_adjacent))
                    {
                    } else {
                        {
                            self.bst_id_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "macro");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "macro");
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                }
                // §199
                {
                    {
                        let mut __f0 = ::core::mem::take(&mut self.buffer);
                        let __r = self.lower_case(
                            &mut __f0,
                            self.buf_ptr1,
                            (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                        );
                        self.buffer = __f0;
                        __r
                    };
                    self.macro_name_loc = {
                        let mut __f0 = ::core::mem::take(&mut self.buffer);
                        let __r = self.str_lookup(
                            &mut __f0,
                            self.buf_ptr1,
                            (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                            macro_ilk,
                            true,
                        );
                        self.buffer = __f0;
                        __r
                    };
                    if self.hash_found {
                        {
                            self.print_a_token();
                            {
                                {
                                    {
                                        crate::system::wr_str(
                                            &mut self.log_file,
                                            " is already defined as a macro",
                                        );
                                    }
                                    {
                                        crate::system::wr_str(
                                            &mut self.standard_output,
                                            " is already defined as a macro",
                                        );
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                    {
                        let __ix200 = self.macro_name_loc;
                        let __v201 = self.hash_text[(self.macro_name_loc) as usize];
                        self.ilk_info[(__ix200) as usize] = __v201;
                    }
                }
                // §198
                {
                    if (!self.eat_bst_white_space()) {
                        {
                            self.eat_bst_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "macro");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "macro");
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                }
                {
                    if (self.buffer[(self.buf_ptr2) as usize] != right_brace) {
                        {
                            self.bst_right_brace_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "macro");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "macro");
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                }
            }
            // §197
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "macro");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "macro");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            // §200
            {
                {
                    if (self.buffer[(self.buf_ptr2) as usize] != left_brace) {
                        {
                            self.bst_left_brace_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "macro");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "macro");
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                }
                {
                    if (!self.eat_bst_white_space()) {
                        {
                            self.eat_bst_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "macro");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "macro");
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                }
                if (self.buffer[(self.buf_ptr2) as usize] != double_quote) {
                    {
                        {
                            {
                                let __w1 = self.xchr[(double_quote) as usize];
                                crate::system::wr_str(
                                    &mut self.log_file,
                                    "A macro definition must be ",
                                );
                                crate::system::wr_char(&mut self.log_file, __w1);
                                crate::system::wr_str(&mut self.log_file, "-delimited");
                            }
                            {
                                let __w1 = self.xchr[(double_quote) as usize];
                                crate::system::wr_str(
                                    &mut self.standard_output,
                                    "A macro definition must be ",
                                );
                                crate::system::wr_char(&mut self.standard_output, __w1);
                                crate::system::wr_str(&mut self.standard_output, "-delimited");
                            }
                        }
                        {
                            self.bst_err_print_and_look_for_blank_line();
                            break 'l_exit_f;
                        }
                    }
                }
                // §201
                {
                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                    if (!self.scan1(double_quote)) {
                        {
                            {
                                {
                                    let __w1 = self.xchr[(double_quote) as usize];
                                    crate::system::wr_str(&mut self.log_file, "There's no `");
                                    crate::system::wr_char(&mut self.log_file, __w1);
                                    crate::system::wr_str(
                                        &mut self.log_file,
                                        "' to end macro definition",
                                    );
                                }
                                {
                                    let __w1 = self.xchr[(double_quote) as usize];
                                    crate::system::wr_str(
                                        &mut self.standard_output,
                                        "There's no `",
                                    );
                                    crate::system::wr_char(&mut self.standard_output, __w1);
                                    crate::system::wr_str(
                                        &mut self.standard_output,
                                        "' to end macro definition",
                                    );
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                    self.macro_def_loc = {
                        let mut __f0 = ::core::mem::take(&mut self.buffer);
                        let __r = self.str_lookup(
                            &mut __f0,
                            self.buf_ptr1,
                            (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                            text_ilk,
                            true,
                        );
                        self.buffer = __f0;
                        __r
                    };
                    self.fn_type[(self.macro_def_loc) as usize] = str_literal;
                    {
                        let __ix202 = self.macro_name_loc;
                        let __v203 = self.hash_text[(self.macro_def_loc) as usize];
                        self.ilk_info[(__ix202) as usize] = __v203;
                    }
                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                }
                // §200
                {
                    if (!self.eat_bst_white_space()) {
                        {
                            self.eat_bst_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "macro");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "macro");
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                }
                {
                    if (self.buffer[(self.buf_ptr2) as usize] != right_brace) {
                        {
                            self.bst_right_brace_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "macro");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "macro");
                                    }
                                }
                                {
                                    self.bst_err_print_and_look_for_blank_line();
                                    break 'l_exit_f;
                                }
                            }
                        }
                    }
                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                }
            }
        }
        // §197
    }

    /// This module either reads a database entry, whose three main components
    /// are an entry type, a database key, and a list of fields, or it reads a
    /// \.{.bib} command, whose structure is command dependent and explained
    /// later.
    // §227
    pub fn get_bib_command_or_entry_and_process(&mut self) {
        'l_exit_f: {
            self.at_bib_command = false;
            // §228
            while (!self.scan1(at_sign)) {
                {
                    if (!{
                        let mut __f0 =
                            ::core::mem::take(&mut self.bib_file[(self.bib_ptr) as usize]);
                        let __r = self.input_ln(&mut __f0);
                        self.bib_file[(self.bib_ptr) as usize] = __f0;
                        __r
                    }) {
                        break 'l_exit_f;
                    }
                    self.bib_line_num = (self.bib_line_num).wrapping_add(1i32);
                    self.buf_ptr2 = 0i32;
                }
            }
            // §229
            {
                if (self.buffer[(self.buf_ptr2) as usize] != at_sign) {
                    {
                        {
                            {
                                let __w1 = self.xchr[(at_sign) as usize];
                                crate::system::wr_str(&mut self.log_file, "An \"");
                                crate::system::wr_char(&mut self.log_file, __w1);
                                crate::system::wr_str(&mut self.log_file, "\" disappeared");
                            }
                            {
                                let __w1 = self.xchr[(at_sign) as usize];
                                crate::system::wr_str(&mut self.standard_output, "An \"");
                                crate::system::wr_char(&mut self.standard_output, __w1);
                                crate::system::wr_str(&mut self.standard_output, "\" disappeared");
                            }
                        }
                        self.print_confusion();
                        crate::system::end_of_TEX(self);
                    }
                }
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                {
                    if (!self.eat_bib_white_space()) {
                        {
                            self.eat_bib_print();
                            break 'l_exit_f;
                        }
                    }
                }
                self.scan_identifier(left_brace, left_paren, left_paren);
                {
                    if ((self.scan_result == white_adjacent)
                        || (self.scan_result == specified_char_adjacent))
                    {
                    } else {
                        {
                            self.bib_id_print();
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "an entry type");
                                    }
                                    {
                                        crate::system::wr_str(
                                            &mut self.standard_output,
                                            "an entry type",
                                        );
                                    }
                                }
                                self.bib_err_print();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
                {
                    let mut __f0 = ::core::mem::take(&mut self.buffer);
                    let __r = self.lower_case(
                        &mut __f0,
                        self.buf_ptr1,
                        (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                    );
                    self.buffer = __f0;
                    __r
                };
                self.command_num = {
                    let __s204 = ({
                        let mut __f0 = ::core::mem::take(&mut self.buffer);
                        let __r = self.str_lookup(
                            &mut __f0,
                            self.buf_ptr1,
                            (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                            bib_command_ilk,
                            false,
                        );
                        self.buffer = __f0;
                        __r
                    }) as usize;
                    self.ilk_info[__s204]
                };
                if self.hash_found {
                    // §230
                    {
                        self.at_bib_command = true;
                        match self.command_num {
                            n_bib_comment => {
                                // §232
                                {
                                    break 'l_exit_f;
                                }
                            }
                            n_bib_preamble => {
                                // §233
                                {
                                    if (self.preamble_ptr == self.max_bib_files) {
                                        {
                                            self.log_realloc(
                                                "bib_list",
                                                4i32,
                                                (self.max_bib_files).wrapping_add(MAX_BIB_FILES),
                                                self.max_bib_files,
                                            );
                                            self.bib_list.resize_len(
                                                (((self.max_bib_files).wrapping_add(MAX_BIB_FILES))
                                                    as usize)
                                                    + 1,
                                            );
                                            self.log_realloc(
                                                "bib_file",
                                                8i32,
                                                (self.max_bib_files).wrapping_add(MAX_BIB_FILES),
                                                self.max_bib_files,
                                            );
                                            {
                                                let __n205 = (((self.max_bib_files)
                                                    .wrapping_add(MAX_BIB_FILES))
                                                    as usize)
                                                    + 1;
                                                self.bib_file.resize(__n205, Default::default());
                                            }
                                            self.log_realloc(
                                                "s_preamble",
                                                4i32,
                                                (self.max_bib_files).wrapping_add(MAX_BIB_FILES),
                                                self.max_bib_files,
                                            );
                                            self.s_preamble.resize_len(
                                                (((self.max_bib_files).wrapping_add(MAX_BIB_FILES))
                                                    as usize)
                                                    + 1,
                                            );
                                            self.max_bib_files =
                                                (self.max_bib_files).wrapping_add(MAX_BIB_FILES);
                                        }
                                    }
                                    {
                                        if (!self.eat_bib_white_space()) {
                                            {
                                                self.eat_bib_print();
                                                break 'l_exit_f;
                                            }
                                        }
                                    }
                                    if (self.buffer[(self.buf_ptr2) as usize] == left_brace) {
                                        self.right_outer_delim = right_brace;
                                    } else {
                                        if (self.buffer[(self.buf_ptr2) as usize] == left_paren) {
                                            self.right_outer_delim = right_paren;
                                        } else {
                                            {
                                                self.bib_one_of_two_print(left_brace, left_paren);
                                                break 'l_exit_f;
                                            }
                                        }
                                    }
                                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                    {
                                        if (!self.eat_bib_white_space()) {
                                            {
                                                self.eat_bib_print();
                                                break 'l_exit_f;
                                            }
                                        }
                                    }
                                    self.store_field = true;
                                    if (!self.scan_and_store_the_field_value_and_eat_white()) {
                                        break 'l_exit_f;
                                    }
                                    if (self.buffer[(self.buf_ptr2) as usize]
                                        != self.right_outer_delim)
                                    {
                                        {
                                            {
                                                {
                                                    let __w1 = self.xchr
                                                        [(self.right_outer_delim) as usize];
                                                    crate::system::wr_str(
                                                        &mut self.log_file,
                                                        "Missing \"",
                                                    );
                                                    crate::system::wr_char(
                                                        &mut self.log_file,
                                                        __w1,
                                                    );
                                                    crate::system::wr_str(
                                                        &mut self.log_file,
                                                        "\" in preamble command",
                                                    );
                                                }
                                                {
                                                    let __w1 = self.xchr
                                                        [(self.right_outer_delim) as usize];
                                                    crate::system::wr_str(
                                                        &mut self.standard_output,
                                                        "Missing \"",
                                                    );
                                                    crate::system::wr_char(
                                                        &mut self.standard_output,
                                                        __w1,
                                                    );
                                                    crate::system::wr_str(
                                                        &mut self.standard_output,
                                                        "\" in preamble command",
                                                    );
                                                }
                                            }
                                            self.bib_err_print();
                                            break 'l_exit_f;
                                        }
                                    }
                                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                    break 'l_exit_f;
                                }
                            }
                            n_bib_string => {
                                // §234
                                {
                                    {
                                        if (!self.eat_bib_white_space()) {
                                            {
                                                self.eat_bib_print();
                                                break 'l_exit_f;
                                            }
                                        }
                                    }
                                    // §235
                                    {
                                        if (self.buffer[(self.buf_ptr2) as usize] == left_brace) {
                                            self.right_outer_delim = right_brace;
                                        } else {
                                            if (self.buffer[(self.buf_ptr2) as usize] == left_paren)
                                            {
                                                self.right_outer_delim = right_paren;
                                            } else {
                                                {
                                                    self.bib_one_of_two_print(
                                                        left_brace, left_paren,
                                                    );
                                                    break 'l_exit_f;
                                                }
                                            }
                                        }
                                        self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                        {
                                            if (!self.eat_bib_white_space()) {
                                                {
                                                    self.eat_bib_print();
                                                    break 'l_exit_f;
                                                }
                                            }
                                        }
                                        self.scan_identifier(equals_sign, equals_sign, equals_sign);
                                        {
                                            if ((self.scan_result == white_adjacent)
                                                || (self.scan_result == specified_char_adjacent))
                                            {
                                            } else {
                                                {
                                                    self.bib_id_print();
                                                    {
                                                        {
                                                            {
                                                                crate::system::wr_str(
                                                                    &mut self.log_file,
                                                                    "a string name",
                                                                );
                                                            }
                                                            {
                                                                crate::system::wr_str(
                                                                    &mut self.standard_output,
                                                                    "a string name",
                                                                );
                                                            }
                                                        }
                                                        self.bib_err_print();
                                                        break 'l_exit_f;
                                                    }
                                                }
                                            }
                                        }
                                        // §236
                                        {
                                            {
                                                let mut __f0 = ::core::mem::take(&mut self.buffer);
                                                let __r = self.lower_case(
                                                    &mut __f0,
                                                    self.buf_ptr1,
                                                    (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                                                );
                                                self.buffer = __f0;
                                                __r
                                            };
                                            self.cur_macro_loc = {
                                                let mut __f0 = ::core::mem::take(&mut self.buffer);
                                                let __r = self.str_lookup(
                                                    &mut __f0,
                                                    self.buf_ptr1,
                                                    (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                                                    macro_ilk,
                                                    true,
                                                );
                                                self.buffer = __f0;
                                                __r
                                            };
                                            {
                                                let __ix206 = self.cur_macro_loc;
                                                let __v207 =
                                                    self.hash_text[(self.cur_macro_loc) as usize];
                                                self.ilk_info[(__ix206) as usize] = __v207;
                                            }
                                        }
                                    }
                                    // §234
                                    {
                                        if (!self.eat_bib_white_space()) {
                                            {
                                                self.eat_bib_print();
                                                break 'l_exit_f;
                                            }
                                        }
                                    }
                                    // §237
                                    {
                                        if (self.buffer[(self.buf_ptr2) as usize] != equals_sign) {
                                            {
                                                self.bib_equals_sign_print();
                                                break 'l_exit_f;
                                            }
                                        }
                                        self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                        {
                                            if (!self.eat_bib_white_space()) {
                                                {
                                                    self.eat_bib_print();
                                                    break 'l_exit_f;
                                                }
                                            }
                                        }
                                        self.store_field = true;
                                        if (!self.scan_and_store_the_field_value_and_eat_white()) {
                                            break 'l_exit_f;
                                        }
                                        if (self.buffer[(self.buf_ptr2) as usize]
                                            != self.right_outer_delim)
                                        {
                                            {
                                                {
                                                    {
                                                        let __w1 = self.xchr
                                                            [(self.right_outer_delim) as usize];
                                                        crate::system::wr_str(
                                                            &mut self.log_file,
                                                            "Missing \"",
                                                        );
                                                        crate::system::wr_char(
                                                            &mut self.log_file,
                                                            __w1,
                                                        );
                                                        crate::system::wr_str(
                                                            &mut self.log_file,
                                                            "\" in string command",
                                                        );
                                                    }
                                                    {
                                                        let __w1 = self.xchr
                                                            [(self.right_outer_delim) as usize];
                                                        crate::system::wr_str(
                                                            &mut self.standard_output,
                                                            "Missing \"",
                                                        );
                                                        crate::system::wr_char(
                                                            &mut self.standard_output,
                                                            __w1,
                                                        );
                                                        crate::system::wr_str(
                                                            &mut self.standard_output,
                                                            "\" in string command",
                                                        );
                                                    }
                                                }
                                                self.bib_err_print();
                                                break 'l_exit_f;
                                            }
                                        }
                                        self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                    }
                                    // §234
                                    break 'l_exit_f;
                                }
                            }
                            _ => {
                                // §230
                                self.bib_cmd_confusion();
                            }
                        }
                    }
                } else {
                    // §229
                    {
                        self.entry_type_loc = {
                            let mut __f0 = ::core::mem::take(&mut self.buffer);
                            let __r = self.str_lookup(
                                &mut __f0,
                                self.buf_ptr1,
                                (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                                bst_fn_ilk,
                                false,
                            );
                            self.buffer = __f0;
                            __r
                        };
                        if ((!self.hash_found)
                            || (self.fn_type[(self.entry_type_loc) as usize] != wiz_defined))
                        {
                            self.type_exists = false;
                        } else {
                            self.type_exists = true;
                        }
                    }
                }
            }
            // §227
            {
                if (!self.eat_bib_white_space()) {
                    {
                        self.eat_bib_print();
                        break 'l_exit_f;
                    }
                }
            }
            // §257
            {
                if (self.buffer[(self.buf_ptr2) as usize] == left_brace) {
                    self.right_outer_delim = right_brace;
                } else {
                    if (self.buffer[(self.buf_ptr2) as usize] == left_paren) {
                        self.right_outer_delim = right_paren;
                    } else {
                        {
                            self.bib_one_of_two_print(left_brace, left_paren);
                            break 'l_exit_f;
                        }
                    }
                }
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                {
                    if (!self.eat_bib_white_space()) {
                        {
                            self.eat_bib_print();
                            break 'l_exit_f;
                        }
                    }
                }
                if (self.right_outer_delim == right_paren) {
                    {
                        if self.scan1_white(comma) {}
                    }
                } else {
                    if self.scan2_white(comma, right_brace) {}
                }
                // §258
                {
                    self.tmp_ptr = self.buf_ptr1;
                    while (self.tmp_ptr < self.buf_ptr2) {
                        {
                            {
                                let __ix208 = self.tmp_ptr;
                                let __v209 = self.buffer[(self.tmp_ptr) as usize];
                                self.ex_buf[(__ix208) as usize] = __v209;
                            }
                            self.tmp_ptr = (self.tmp_ptr).wrapping_add(1i32);
                        }
                    }
                    {
                        let mut __f0 = ::core::mem::take(&mut self.ex_buf);
                        let __r = self.lower_case(
                            &mut __f0,
                            self.buf_ptr1,
                            (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                        );
                        self.ex_buf = __f0;
                        __r
                    };
                    if self.all_entries {
                        self.lc_cite_loc = {
                            let mut __f0 = ::core::mem::take(&mut self.ex_buf);
                            let __r = self.str_lookup(
                                &mut __f0,
                                self.buf_ptr1,
                                (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                                lc_cite_ilk,
                                true,
                            );
                            self.ex_buf = __f0;
                            __r
                        };
                    } else {
                        self.lc_cite_loc = {
                            let mut __f0 = ::core::mem::take(&mut self.ex_buf);
                            let __r = self.str_lookup(
                                &mut __f0,
                                self.buf_ptr1,
                                (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                                lc_cite_ilk,
                                false,
                            );
                            self.ex_buf = __f0;
                            __r
                        };
                    }
                    if self.hash_found {
                        {
                            self.entry_cite_ptr = self.ilk_info
                                [(self.ilk_info[(self.lc_cite_loc) as usize]) as usize];
                            // §259
                            {
                                'l_L26_f: {
                                    if (((!self.all_entries)
                                        || (self.entry_cite_ptr < self.all_marker))
                                        || (self.entry_cite_ptr >= self.old_num_cites))
                                    {
                                        {
                                            if (self.type_list[(self.entry_cite_ptr) as usize]
                                                == empty)
                                            {
                                                {
                                                    // §260
                                                    {
                                                        if ((!self.all_entries)
                                                            && (self.entry_cite_ptr
                                                                >= self.old_num_cites))
                                                        {
                                                            {
                                                                self.cite_loc = {
                                                                    let mut __f0 =
                                                                        ::core::mem::take(
                                                                            &mut self.buffer,
                                                                        );
                                                                    let __r = self.str_lookup(
                                                                        &mut __f0,
                                                                        self.buf_ptr1,
                                                                        (self.buf_ptr2)
                                                                            .wrapping_sub(
                                                                                self.buf_ptr1,
                                                                            ),
                                                                        cite_ilk,
                                                                        true,
                                                                    );
                                                                    self.buffer = __f0;
                                                                    __r
                                                                };
                                                                if (!self.hash_found) {
                                                                    {
                                                                        {
                                                                            let __ix210 =
                                                                                self.lc_cite_loc;
                                                                            let __v211 =
                                                                                self.cite_loc;
                                                                            self.ilk_info[(__ix210)
                                                                                as usize] = __v211;
                                                                        }
                                                                        {
                                                                            let __ix212 =
                                                                                self.cite_loc;
                                                                            let __v213 =
                                                                                self.entry_cite_ptr;
                                                                            self.ilk_info[(__ix212)
                                                                                as usize] = __v213;
                                                                        }
                                                                        {
                                                                            let __ix214 =
                                                                                self.entry_cite_ptr;
                                                                            let __v215 = self
                                                                                .hash_text
                                                                                [(self.cite_loc)
                                                                                    as usize];
                                                                            self.cite_list[(__ix214)
                                                                                as usize] = __v215;
                                                                        }
                                                                        self.hash_found = true;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    // §259
                                                    break 'l_L26_f;
                                                }
                                            }
                                        }
                                    } else {
                                        if (!self.entry_exists[(self.entry_cite_ptr) as usize]) {
                                            {
                                                // §261
                                                {
                                                    self.ex_buf_ptr = 0i32;
                                                    self.tmp_ptr = self.str_start[(self.cite_info
                                                        [(self.entry_cite_ptr) as usize])
                                                        as usize];
                                                    self.tmp_end_ptr = self.str_start[((self
                                                        .cite_info
                                                        [(self.entry_cite_ptr) as usize])
                                                        .wrapping_add(1i32))
                                                        as usize];
                                                    while (self.tmp_ptr < self.tmp_end_ptr) {
                                                        {
                                                            {
                                                                let __ix216 = self.ex_buf_ptr;
                                                                let __v217 = self.str_pool
                                                                    [(self.tmp_ptr) as usize];
                                                                self.ex_buf[(__ix216) as usize] =
                                                                    __v217;
                                                            }
                                                            self.ex_buf_ptr = (self.ex_buf_ptr)
                                                                .wrapping_add(1i32);
                                                            self.tmp_ptr =
                                                                (self.tmp_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                    {
                                                        let mut __f0 =
                                                            ::core::mem::take(&mut self.ex_buf);
                                                        let __r = self.lower_case(
                                                            &mut __f0,
                                                            0i32,
                                                            (self.str_start[((self.cite_info
                                                                [(self.entry_cite_ptr) as usize])
                                                                .wrapping_add(1i32))
                                                                as usize])
                                                                .wrapping_sub(
                                                                    self.str_start[(self.cite_info
                                                                        [(self.entry_cite_ptr)
                                                                            as usize])
                                                                        as usize],
                                                                ),
                                                        );
                                                        self.ex_buf = __f0;
                                                        __r
                                                    };
                                                    self.lc_xcite_loc = {
                                                        let mut __f0 =
                                                            ::core::mem::take(&mut self.ex_buf);
                                                        let __r = self.str_lookup(
                                                            &mut __f0,
                                                            0i32,
                                                            (self.str_start[((self.cite_info
                                                                [(self.entry_cite_ptr) as usize])
                                                                .wrapping_add(1i32))
                                                                as usize])
                                                                .wrapping_sub(
                                                                    self.str_start[(self.cite_info
                                                                        [(self.entry_cite_ptr)
                                                                            as usize])
                                                                        as usize],
                                                                ),
                                                            lc_cite_ilk,
                                                            false,
                                                        );
                                                        self.ex_buf = __f0;
                                                        __r
                                                    };
                                                    if (!self.hash_found) {
                                                        self.cite_key_disappeared_confusion();
                                                    }
                                                }
                                                // §259
                                                if (self.lc_xcite_loc == self.lc_cite_loc) {
                                                    break 'l_L26_f;
                                                }
                                            }
                                        }
                                    }
                                    if (self.type_list[(self.entry_cite_ptr) as usize] == empty) {
                                        {
                                            {
                                                {
                                                    crate::system::wr_str(
                                                        &mut self.log_file,
                                                        "The cite list is messed up",
                                                    );
                                                }
                                                {
                                                    crate::system::wr_str(
                                                        &mut self.standard_output,
                                                        "The cite list is messed up",
                                                    );
                                                }
                                            }
                                            self.print_confusion();
                                            crate::system::end_of_TEX(self);
                                        }
                                    }
                                    {
                                        {
                                            {
                                                crate::system::wr_str(
                                                    &mut self.log_file,
                                                    "Repeated entry",
                                                );
                                            }
                                            {
                                                crate::system::wr_str(
                                                    &mut self.standard_output,
                                                    "Repeated entry",
                                                );
                                            }
                                        }
                                        self.bib_err_print();
                                        break 'l_exit_f;
                                    }
                                }
                            }
                        }
                    }
                    // §258
                    self.store_entry = true;
                    if self.all_entries {
                        // §263
                        {
                            'l_L22_f: {
                                if self.hash_found {
                                    {
                                        if (self.entry_cite_ptr < self.all_marker) {
                                            break 'l_L22_f;
                                        } else {
                                            {
                                                {
                                                    let __ix218 = self.entry_cite_ptr;
                                                    let __v219 = true;
                                                    self.entry_exists[(__ix218) as usize] = __v219;
                                                }
                                                self.cite_loc =
                                                    self.ilk_info[(self.lc_cite_loc) as usize];
                                            }
                                        }
                                    }
                                } else {
                                    {
                                        self.cite_loc = {
                                            let mut __f0 = ::core::mem::take(&mut self.buffer);
                                            let __r = self.str_lookup(
                                                &mut __f0,
                                                self.buf_ptr1,
                                                (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                                                cite_ilk,
                                                true,
                                            );
                                            self.buffer = __f0;
                                            __r
                                        };
                                        if self.hash_found {
                                            self.hash_cite_confusion();
                                        }
                                    }
                                }
                                self.entry_cite_ptr = self.cite_ptr;
                                {
                                    let mut __f0 = ::core::mem::take(&mut self.cite_ptr);
                                    let __r = self.add_database_cite(&mut __f0);
                                    self.cite_ptr = __f0;
                                    __r
                                };
                            }
                        }
                    } else {
                        // §258
                        if (!self.hash_found) {
                            self.store_entry = false;
                        }
                    }
                    if self.store_entry {
                        // §264
                        {
                            if self.type_exists {
                                {
                                    let __ix220 = self.entry_cite_ptr;
                                    let __v221 = self.entry_type_loc;
                                    self.type_list[(__ix220) as usize] = __v221;
                                }
                            } else {
                                {
                                    {
                                        let __ix222 = self.entry_cite_ptr;
                                        let __v223 = self.undefined;
                                        self.type_list[(__ix222) as usize] = __v223;
                                    }
                                    {
                                        {
                                            crate::system::wr_str(
                                                &mut self.log_file,
                                                "Warning--entry type for \"",
                                            );
                                        }
                                        {
                                            crate::system::wr_str(
                                                &mut self.standard_output,
                                                "Warning--entry type for \"",
                                            );
                                        }
                                    }
                                    self.print_a_token();
                                    {
                                        {
                                            {
                                                crate::system::wr_str(
                                                    &mut self.log_file,
                                                    "\" isn't style-file defined",
                                                );
                                                crate::system::wr_ln(&mut self.log_file);
                                            }
                                            {
                                                crate::system::wr_str(
                                                    &mut self.standard_output,
                                                    "\" isn't style-file defined",
                                                );
                                                crate::system::wr_ln(&mut self.standard_output);
                                            }
                                        }
                                        self.bib_warn_print();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // §227
            {
                if (!self.eat_bib_white_space()) {
                    {
                        self.eat_bib_print();
                        break 'l_exit_f;
                    }
                }
            }
            // §265
            {
                'l_L15_f: {
                    while (self.buffer[(self.buf_ptr2) as usize] != self.right_outer_delim) {
                        {
                            if (self.buffer[(self.buf_ptr2) as usize] != comma) {
                                {
                                    self.bib_one_of_two_print(comma, self.right_outer_delim);
                                    break 'l_exit_f;
                                }
                            }
                            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                            {
                                if (!self.eat_bib_white_space()) {
                                    {
                                        self.eat_bib_print();
                                        break 'l_exit_f;
                                    }
                                }
                            }
                            if (self.buffer[(self.buf_ptr2) as usize] == self.right_outer_delim) {
                                break 'l_L15_f;
                            }
                            // §266
                            {
                                self.scan_identifier(equals_sign, equals_sign, equals_sign);
                                {
                                    if ((self.scan_result == white_adjacent)
                                        || (self.scan_result == specified_char_adjacent))
                                    {
                                    } else {
                                        {
                                            self.bib_id_print();
                                            {
                                                {
                                                    {
                                                        crate::system::wr_str(
                                                            &mut self.log_file,
                                                            "a field name",
                                                        );
                                                    }
                                                    {
                                                        crate::system::wr_str(
                                                            &mut self.standard_output,
                                                            "a field name",
                                                        );
                                                    }
                                                }
                                                self.bib_err_print();
                                                break 'l_exit_f;
                                            }
                                        }
                                    }
                                }
                                self.store_field = false;
                                if self.store_entry {
                                    {
                                        {
                                            let mut __f0 = ::core::mem::take(&mut self.buffer);
                                            let __r = self.lower_case(
                                                &mut __f0,
                                                self.buf_ptr1,
                                                (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                                            );
                                            self.buffer = __f0;
                                            __r
                                        };
                                        self.field_name_loc = {
                                            let mut __f0 = ::core::mem::take(&mut self.buffer);
                                            let __r = self.str_lookup(
                                                &mut __f0,
                                                self.buf_ptr1,
                                                (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                                                bst_fn_ilk,
                                                false,
                                            );
                                            self.buffer = __f0;
                                            __r
                                        };
                                        if self.hash_found {
                                            if (self.fn_type[(self.field_name_loc) as usize]
                                                == field)
                                            {
                                                self.store_field = true;
                                            }
                                        }
                                    }
                                }
                                {
                                    if (!self.eat_bib_white_space()) {
                                        {
                                            self.eat_bib_print();
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                                if (self.buffer[(self.buf_ptr2) as usize] != equals_sign) {
                                    {
                                        self.bib_equals_sign_print();
                                        break 'l_exit_f;
                                    }
                                }
                                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                            }
                            // §265
                            {
                                if (!self.eat_bib_white_space()) {
                                    {
                                        self.eat_bib_print();
                                        break 'l_exit_f;
                                    }
                                }
                            }
                            if (!self.scan_and_store_the_field_value_and_eat_white()) {
                                break 'l_exit_f;
                            }
                        }
                    }
                }
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            }
        }
        // §227
    }

    /// The \.{read} command has no arguments so there's no more parsing to
    /// do.  We must make sure we haven't seen a \.{read} command before and
    /// we've already seen an \.{entry} command.
    /// @<Procedures and functions for the reading and processing of input files
    // §203
    pub fn bst_read_command(&mut self) {
        'l_exit_f: {
            if self.read_seen {
                {
                    {
                        {
                            crate::system::wr_str(
                                &mut self.log_file,
                                "Illegal, another read command",
                            );
                        }
                        {
                            crate::system::wr_str(
                                &mut self.standard_output,
                                "Illegal, another read command",
                            );
                        }
                    }
                    {
                        self.bst_err_print_and_look_for_blank_line();
                        break 'l_exit_f;
                    }
                }
            }
            self.read_seen = true;
            if (!self.entry_seen) {
                {
                    {
                        {
                            crate::system::wr_str(
                                &mut self.log_file,
                                "Illegal, read command before entry command",
                            );
                        }
                        {
                            crate::system::wr_str(
                                &mut self.standard_output,
                                "Illegal, read command before entry command",
                            );
                        }
                    }
                    {
                        self.bst_err_print_and_look_for_blank_line();
                        break 'l_exit_f;
                    }
                }
            }
            self.sv_ptr1 = self.buf_ptr2;
            self.sv_ptr2 = self.last;
            self.tmp_ptr = self.sv_ptr1;
            while (self.tmp_ptr < self.sv_ptr2) {
                {
                    {
                        let __ix224 = self.tmp_ptr;
                        let __v225 = self.buffer[(self.tmp_ptr) as usize];
                        self.sv_buffer[(__ix224) as usize] = __v225;
                    }
                    self.tmp_ptr = (self.tmp_ptr).wrapping_add(1i32);
                }
            }
            // §214
            {
                // §215
                {
                    // §216
                    {
                        self.check_field_overflow((self.num_fields).wrapping_mul(self.num_cites));
                        self.field_ptr = 0i32;
                        while (self.field_ptr < self.max_fields) {
                            {
                                self.field_info[(self.field_ptr) as usize] = missing;
                                self.field_ptr = (self.field_ptr).wrapping_add(1i32);
                            }
                        }
                    }
                    // §218
                    {
                        self.cite_ptr = 0i32;
                        while (self.cite_ptr < self.max_cites) {
                            {
                                self.type_list[(self.cite_ptr) as usize] = empty;
                                self.cite_info[(self.cite_ptr) as usize] = any_value;
                                self.cite_ptr = (self.cite_ptr).wrapping_add(1i32);
                            }
                        }
                        self.old_num_cites = self.num_cites;
                        if self.all_entries {
                            {
                                self.cite_ptr = self.all_marker;
                                while (self.cite_ptr < self.old_num_cites) {
                                    {
                                        {
                                            let __ix226 = self.cite_ptr;
                                            let __v227 = self.cite_list[(self.cite_ptr) as usize];
                                            self.cite_info[(__ix226) as usize] = __v227;
                                        }
                                        {
                                            let __ix228 = self.cite_ptr;
                                            let __v229 = false;
                                            self.entry_exists[(__ix228) as usize] = __v229;
                                        }
                                        self.cite_ptr = (self.cite_ptr).wrapping_add(1i32);
                                    }
                                }
                                self.cite_ptr = self.all_marker;
                            }
                        } else {
                            {
                                self.cite_ptr = self.num_cites;
                                self.all_marker = any_value;
                            }
                        }
                    }
                }
                // §214
                self.read_performed = true;
                self.bib_ptr = 0i32;
                while (self.bib_ptr < self.num_bib_files) {
                    {
                        if self.verbose {
                            {
                                {
                                    {
                                        let __w1 = (self.bib_ptr).wrapping_add(1i32);
                                        crate::system::wr_str(
                                            &mut self.log_file,
                                            "Database file #",
                                        );
                                        crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                                        crate::system::wr_str(&mut self.log_file, ": ");
                                    }
                                    {
                                        let __w1 = (self.bib_ptr).wrapping_add(1i32);
                                        crate::system::wr_str(
                                            &mut self.standard_output,
                                            "Database file #",
                                        );
                                        crate::system::wr_int(
                                            &mut self.standard_output,
                                            __w1,
                                            0i32,
                                        );
                                        crate::system::wr_str(&mut self.standard_output, ": ");
                                    }
                                }
                                self.print_bib_name();
                            }
                        } else {
                            {
                                {
                                    {
                                        let __w1 = (self.bib_ptr).wrapping_add(1i32);
                                        crate::system::wr_str(
                                            &mut self.log_file,
                                            "Database file #",
                                        );
                                        crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                                        crate::system::wr_str(&mut self.log_file, ": ");
                                    }
                                }
                                self.log_pr_bib_name();
                            }
                        }
                        self.bib_line_num = 0i32;
                        self.buf_ptr2 = self.last;
                        while (!crate::system::eof(&self.bib_file[(self.bib_ptr) as usize])) {
                            self.get_bib_command_or_entry_and_process();
                        }
                        {
                            let mut __f0 =
                                ::core::mem::take(&mut self.bib_file[(self.bib_ptr) as usize]);
                            let __r = self.a_close(&mut __f0);
                            self.bib_file[(self.bib_ptr) as usize] = __f0;
                            __r
                        };
                        self.bib_ptr = (self.bib_ptr).wrapping_add(1i32);
                    }
                }
                self.reading_completed = true;
                // §267
                {
                    self.num_cites = self.cite_ptr;
                    self.num_preamble_strings = self.preamble_ptr;
                    // §268
                    {
                        if ((((self.num_cites).wrapping_sub(1i32)).wrapping_mul(self.num_fields))
                            .wrapping_add(self.crossref_num)
                            >= self.max_fields)
                        {
                            {
                                {
                                    {
                                        crate::system::wr_str(
                                            &mut self.log_file,
                                            "field_info index is out of range",
                                        );
                                    }
                                    {
                                        crate::system::wr_str(
                                            &mut self.standard_output,
                                            "field_info index is out of range",
                                        );
                                    }
                                }
                                self.print_confusion();
                                crate::system::end_of_TEX(self);
                            }
                        }
                        self.cite_ptr = 0i32;
                        while (self.cite_ptr < self.num_cites) {
                            {
                                self.field_ptr = ((self.cite_ptr).wrapping_mul(self.num_fields))
                                    .wrapping_add(self.crossref_num);
                                if (self.field_info[(self.field_ptr) as usize] != missing) {
                                    if self.find_cite_locs_for_this_cite_key(
                                        self.field_info[(self.field_ptr) as usize],
                                    ) {
                                        {
                                            self.cite_loc =
                                                self.ilk_info[(self.lc_cite_loc) as usize];
                                            {
                                                let __ix230 = self.field_ptr;
                                                let __v231 =
                                                    self.hash_text[(self.cite_loc) as usize];
                                                self.field_info[(__ix230) as usize] = __v231;
                                            }
                                            self.cite_parent_ptr =
                                                self.ilk_info[(self.cite_loc) as usize];
                                            self.field_ptr = ((self.cite_ptr)
                                                .wrapping_mul(self.num_fields))
                                            .wrapping_add(self.num_pre_defined_fields);
                                            self.field_end_ptr = ((self.field_ptr)
                                                .wrapping_sub(self.num_pre_defined_fields))
                                            .wrapping_add(self.num_fields);
                                            self.field_parent_ptr = ((self.cite_parent_ptr)
                                                .wrapping_mul(self.num_fields))
                                            .wrapping_add(self.num_pre_defined_fields);
                                            while (self.field_ptr < self.field_end_ptr) {
                                                {
                                                    if (self.field_info[(self.field_ptr) as usize]
                                                        == missing)
                                                    {
                                                        {
                                                            let __ix232 = self.field_ptr;
                                                            let __v233 = self.field_info
                                                                [(self.field_parent_ptr) as usize];
                                                            self.field_info[(__ix232) as usize] =
                                                                __v233;
                                                        }
                                                    }
                                                    self.field_ptr =
                                                        (self.field_ptr).wrapping_add(1i32);
                                                    self.field_parent_ptr =
                                                        (self.field_parent_ptr).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                    }
                                }
                                self.cite_ptr = (self.cite_ptr).wrapping_add(1i32);
                            }
                        }
                    }
                    // §270
                    {
                        if ((((self.num_cites).wrapping_sub(1i32)).wrapping_mul(self.num_fields))
                            .wrapping_add(self.crossref_num)
                            >= self.max_fields)
                        {
                            {
                                {
                                    {
                                        crate::system::wr_str(
                                            &mut self.log_file,
                                            "field_info index is out of range",
                                        );
                                    }
                                    {
                                        crate::system::wr_str(
                                            &mut self.standard_output,
                                            "field_info index is out of range",
                                        );
                                    }
                                }
                                self.print_confusion();
                                crate::system::end_of_TEX(self);
                            }
                        }
                        self.cite_ptr = 0i32;
                        while (self.cite_ptr < self.num_cites) {
                            {
                                self.field_ptr = ((self.cite_ptr).wrapping_mul(self.num_fields))
                                    .wrapping_add(self.crossref_num);
                                if (self.field_info[(self.field_ptr) as usize] != missing) {
                                    if (!self.find_cite_locs_for_this_cite_key(
                                        self.field_info[(self.field_ptr) as usize],
                                    )) {
                                        {
                                            if self.cite_hash_found {
                                                self.hash_cite_confusion();
                                            }
                                            self.nonexistent_cross_reference_error();
                                            self.field_info[(self.field_ptr) as usize] = missing;
                                        }
                                    } else {
                                        {
                                            if (self.cite_loc
                                                != self.ilk_info[(self.lc_cite_loc) as usize])
                                            {
                                                self.hash_cite_confusion();
                                            }
                                            self.cite_parent_ptr =
                                                self.ilk_info[(self.cite_loc) as usize];
                                            if (self.type_list[(self.cite_parent_ptr) as usize]
                                                == empty)
                                            {
                                                {
                                                    self.nonexistent_cross_reference_error();
                                                    self.field_info[(self.field_ptr) as usize] =
                                                        missing;
                                                }
                                            } else {
                                                {
                                                    self.field_parent_ptr = ((self
                                                        .cite_parent_ptr)
                                                        .wrapping_mul(self.num_fields))
                                                    .wrapping_add(self.crossref_num);
                                                    if (self.field_info
                                                        [(self.field_parent_ptr) as usize]
                                                        != missing)
                                                    {
                                                        // §273
                                                        {
                                                            {
                                                                {
                                                                    crate::system::wr_str(&mut self.log_file, "Warning--you've nested cross references");
                                                                }
                                                                {
                                                                    crate::system::wr_str(&mut self.standard_output, "Warning--you've nested cross references");
                                                                }
                                                            }
                                                            self.bad_cross_reference_print(
                                                                self.cite_list[(self
                                                                    .cite_parent_ptr)
                                                                    as usize],
                                                            );
                                                            {
                                                                {
                                                                    crate::system::wr_str(&mut self.log_file, "\", which also refers to something");
                                                                    crate::system::wr_ln(
                                                                        &mut self.log_file,
                                                                    );
                                                                }
                                                                {
                                                                    crate::system::wr_str(&mut self.standard_output, "\", which also refers to something");
                                                                    crate::system::wr_ln(
                                                                        &mut self.standard_output,
                                                                    );
                                                                }
                                                            }
                                                            self.mark_warning();
                                                        }
                                                    }
                                                    // §270
                                                    if (((!self.all_entries)
                                                        && (self.cite_parent_ptr
                                                            >= self.old_num_cites))
                                                        && (self.cite_info
                                                            [(self.cite_parent_ptr) as usize]
                                                            < self.min_crossrefs))
                                                    {
                                                        self.field_info
                                                            [(self.field_ptr) as usize] = missing;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                self.cite_ptr = (self.cite_ptr).wrapping_add(1i32);
                            }
                        }
                    }
                    // §274
                    {
                        self.cite_ptr = 0i32;
                        while (self.cite_ptr < self.num_cites) {
                            {
                                if (self.type_list[(self.cite_ptr) as usize] == empty) {
                                    self.print_missing_entry(
                                        self.cite_list[(self.cite_ptr) as usize],
                                    );
                                } else {
                                    if ((self.all_entries || (self.cite_ptr < self.old_num_cites))
                                        || (self.cite_info[(self.cite_ptr) as usize]
                                            >= self.min_crossrefs))
                                    {
                                        {
                                            if (self.cite_ptr > self.cite_xptr) {
                                                // §276
                                                {
                                                    if (((self.cite_xptr).wrapping_add(1i32))
                                                        .wrapping_mul(self.num_fields)
                                                        > self.max_fields)
                                                    {
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
                                                    {
                                                        let __ix234 = self.cite_xptr;
                                                        let __v235 = self.cite_list
                                                            [(self.cite_ptr) as usize];
                                                        self.cite_list[(__ix234) as usize] = __v235;
                                                    }
                                                    {
                                                        let __ix236 = self.cite_xptr;
                                                        let __v237 = self.type_list
                                                            [(self.cite_ptr) as usize];
                                                        self.type_list[(__ix236) as usize] = __v237;
                                                    }
                                                    if (!self.find_cite_locs_for_this_cite_key(
                                                        self.cite_list[(self.cite_ptr) as usize],
                                                    )) {
                                                        self.cite_key_disappeared_confusion();
                                                    }
                                                    if ((!self.cite_hash_found)
                                                        || (self.cite_loc
                                                            != self.ilk_info
                                                                [(self.lc_cite_loc) as usize]))
                                                    {
                                                        self.hash_cite_confusion();
                                                    }
                                                    {
                                                        let __ix238 = self.cite_loc;
                                                        let __v239 = self.cite_xptr;
                                                        self.ilk_info[(__ix238) as usize] = __v239;
                                                    }
                                                    self.field_ptr = (self.cite_xptr)
                                                        .wrapping_mul(self.num_fields);
                                                    self.field_end_ptr = (self.field_ptr)
                                                        .wrapping_add(self.num_fields);
                                                    self.tmp_ptr = (self.cite_ptr)
                                                        .wrapping_mul(self.num_fields);
                                                    while (self.field_ptr < self.field_end_ptr) {
                                                        {
                                                            {
                                                                let __ix240 = self.field_ptr;
                                                                let __v241 = self.field_info
                                                                    [(self.tmp_ptr) as usize];
                                                                self.field_info
                                                                    [(__ix240) as usize] = __v241;
                                                            }
                                                            self.field_ptr =
                                                                (self.field_ptr).wrapping_add(1i32);
                                                            self.tmp_ptr =
                                                                (self.tmp_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                            }
                                            // §274
                                            self.cite_xptr = (self.cite_xptr).wrapping_add(1i32);
                                        }
                                    }
                                }
                                self.cite_ptr = (self.cite_ptr).wrapping_add(1i32);
                            }
                        }
                        self.num_cites = self.cite_xptr;
                        if self.all_entries {
                            // §277
                            {
                                self.cite_ptr = self.all_marker;
                                while (self.cite_ptr < self.old_num_cites) {
                                    {
                                        if (!self.entry_exists[(self.cite_ptr) as usize]) {
                                            self.print_missing_entry(
                                                self.cite_info[(self.cite_ptr) as usize],
                                            );
                                        }
                                        self.cite_ptr = (self.cite_ptr).wrapping_add(1i32);
                                    }
                                }
                            }
                        }
                    }
                    // §278
                    {
                        self.entry_ints.alloc_len(
                            (((((self.num_ent_ints).wrapping_add(1i32))
                                .wrapping_mul((self.num_cites).wrapping_add(1i32)))
                            .wrapping_sub(1i32)) as usize)
                                + 1,
                        );
                        self.int_ent_ptr = 0i32;
                        while (self.int_ent_ptr < (self.num_ent_ints).wrapping_mul(self.num_cites))
                        {
                            {
                                self.entry_ints[(self.int_ent_ptr) as usize] = 0i32;
                                self.int_ent_ptr = (self.int_ent_ptr).wrapping_add(1i32);
                            }
                        }
                    }
                    // §279
                    {
                        self.entry_strs.alloc_len(
                            ((((((self.num_ent_strs).wrapping_add(1i32))
                                .wrapping_mul((self.num_cites).wrapping_add(1i32)))
                            .wrapping_mul((self.ent_str_size).wrapping_add(1i32)))
                            .wrapping_sub(1i32)) as usize)
                                + 1,
                        );
                        self.str_ent_ptr = 0i32;
                        while (self.str_ent_ptr < (self.num_ent_strs).wrapping_mul(self.num_cites))
                        {
                            {
                                self.entry_strs[(((self.str_ent_ptr)
                                    .wrapping_mul((self.ent_str_size).wrapping_add(1i32)))
                                .wrapping_add(0i32))
                                    as usize] = end_of_string;
                                self.str_ent_ptr = (self.str_ent_ptr).wrapping_add(1i32);
                            }
                        }
                    }
                    // §280
                    {
                        self.cite_ptr = 0i32;
                        while (self.cite_ptr < self.num_cites) {
                            {
                                {
                                    let __ix242 = self.cite_ptr;
                                    let __v243 = self.cite_ptr;
                                    self.cite_info[(__ix242) as usize] = __v243;
                                }
                                self.cite_ptr = (self.cite_ptr).wrapping_add(1i32);
                            }
                        }
                    }
                }
                // §214
                self.read_completed = true;
            }
            // §203
            self.buf_ptr2 = self.sv_ptr1;
            self.last = self.sv_ptr2;
            self.tmp_ptr = self.buf_ptr2;
            while (self.tmp_ptr < self.last) {
                {
                    {
                        let __ix244 = self.tmp_ptr;
                        let __v245 = self.sv_buffer[(self.tmp_ptr) as usize];
                        self.buffer[(__ix244) as usize] = __v245;
                    }
                    self.tmp_ptr = (self.tmp_ptr).wrapping_add(1i32);
                }
            }
        }
    }

    /// A \.{reverse} command has one argument, a single `built_in` or
    /// `wiz_defined` function name between braces.  Upper/lower cases are
    /// considered to be the same---all upper-case letters are converted to
    /// lower case.  Also, we must make sure we've already seen a \.{read}
    /// command.
    /// This module reads a `left_brace`, a single function to be iterated in
    /// reverse, and a `right_brace`.
    /// @<Procedures and functions for the reading and processing of input files
    // §204
    pub fn bst_reverse_command(&mut self) {
        'l_exit_f: {
            if (!self.read_seen) {
                {
                    {
                        {
                            crate::system::wr_str(
                                &mut self.log_file,
                                "Illegal, reverse command before read command",
                            );
                        }
                        {
                            crate::system::wr_str(
                                &mut self.standard_output,
                                "Illegal, reverse command before read command",
                            );
                        }
                    }
                    {
                        self.bst_err_print_and_look_for_blank_line();
                        break 'l_exit_f;
                    }
                }
            }
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "reverse");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "reverse");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            {
                if (self.buffer[(self.buf_ptr2) as usize] != left_brace) {
                    {
                        self.bst_left_brace_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "reverse");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "reverse");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            }
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "reverse");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "reverse");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            {
                self.scan_identifier(right_brace, comment, comment);
                if ((self.scan_result == white_adjacent)
                    || (self.scan_result == specified_char_adjacent))
                {
                } else {
                    {
                        self.bst_id_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "reverse");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "reverse");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            // §205
            {
                if self.bad_argument_token() {
                    break 'l_exit_f;
                }
            }
            // §204
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "reverse");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "reverse");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            {
                if (self.buffer[(self.buf_ptr2) as usize] != right_brace) {
                    {
                        self.bst_right_brace_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "reverse");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "reverse");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            }
            // §289
            {
                self.init_command_execution();
                self.mess_with_entries = true;
                if (self.num_cites > 0i32) {
                    {
                        self.sort_cite_ptr = self.num_cites;
                        loop {
                            self.sort_cite_ptr = (self.sort_cite_ptr).wrapping_sub(1i32);
                            self.cite_ptr = self.cite_info[(self.sort_cite_ptr) as usize];
                            self.execute_fn(self.fn_loc);
                            self.check_command_execution();
                            if (self.sort_cite_ptr == 0i32) {
                                break;
                            }
                        }
                    }
                }
            }
        }
        // §204
    }

    /// The \.{sort} command has no arguments so there's no more parsing to
    /// do, but we must make sure we've already seen a \.{read} command.
    /// @<Procedures and functions for the reading and processing of input files
    // §206
    pub fn bst_sort_command(&mut self) {
        'l_exit_f: {
            if (!self.read_seen) {
                {
                    {
                        {
                            crate::system::wr_str(
                                &mut self.log_file,
                                "Illegal, sort command before read command",
                            );
                        }
                        {
                            crate::system::wr_str(
                                &mut self.standard_output,
                                "Illegal, sort command before read command",
                            );
                        }
                    }
                    {
                        self.bst_err_print_and_look_for_blank_line();
                        break 'l_exit_f;
                    }
                }
            }
            // §290
            {
                if (self.num_cites > 1i32) {
                    self.quick_sort(0i32, (self.num_cites).wrapping_sub(1i32));
                }
            }
        }
        // §206
    }

    /// A \.{strings} command has one argument, a list of function names
    /// between braces (the names are separated by one or more `white_space`
    /// characters).  Upper/lower cases are considered to be the same for
    /// function names in these lists---all upper-case letters are converted to
    /// lower case.  Each name in this list specifies a `str_global_var`.
    /// There may be several \.{strings} commands in the \.{.bst} file.
    /// This module reads a `left_brace`, a list of `str_global_var`s,
    /// and a `right_brace`.
    /// @<Procedures and functions for the reading and processing of input files
    // §207
    pub fn bst_strings_command(&mut self) {
        'l_exit_f: {
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "strings");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "strings");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            {
                if (self.buffer[(self.buf_ptr2) as usize] != left_brace) {
                    {
                        self.bst_left_brace_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "strings");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "strings");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            }
            {
                if (!self.eat_bst_white_space()) {
                    {
                        self.eat_bst_print();
                        {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "strings");
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "strings");
                                }
                            }
                            {
                                self.bst_err_print_and_look_for_blank_line();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            while (self.buffer[(self.buf_ptr2) as usize] != right_brace) {
                {
                    {
                        self.scan_identifier(right_brace, comment, comment);
                        if ((self.scan_result == white_adjacent)
                            || (self.scan_result == specified_char_adjacent))
                        {
                        } else {
                            {
                                self.bst_id_print();
                                {
                                    {
                                        {
                                            crate::system::wr_str(&mut self.log_file, "strings");
                                        }
                                        {
                                            crate::system::wr_str(
                                                &mut self.standard_output,
                                                "strings",
                                            );
                                        }
                                    }
                                    {
                                        self.bst_err_print_and_look_for_blank_line();
                                        break 'l_exit_f;
                                    }
                                }
                            }
                        }
                    }
                    // §208
                    {
                        {
                            let mut __f0 = ::core::mem::take(&mut self.buffer);
                            let __r = self.lower_case(
                                &mut __f0,
                                self.buf_ptr1,
                                (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                            );
                            self.buffer = __f0;
                            __r
                        };
                        self.fn_loc = {
                            let mut __f0 = ::core::mem::take(&mut self.buffer);
                            let __r = self.str_lookup(
                                &mut __f0,
                                self.buf_ptr1,
                                (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                                bst_fn_ilk,
                                true,
                            );
                            self.buffer = __f0;
                            __r
                        };
                        {
                            if self.hash_found {
                                {
                                    self.already_seen_function_print(self.fn_loc);
                                    break 'l_exit_f;
                                }
                            }
                        }
                        self.fn_type[(self.fn_loc) as usize] = str_global_var;
                        {
                            let __ix246 = self.fn_loc;
                            let __v247 = self.num_glb_strs;
                            self.ilk_info[(__ix246) as usize] = __v247;
                        }
                        if (self.num_glb_strs == self.max_glob_strs) {
                            {
                                self.log_realloc(
                                    "glb_str_ptr",
                                    4i32,
                                    (self.max_glob_strs).wrapping_add(MAX_GLOB_STRS),
                                    self.max_glob_strs,
                                );
                                self.glb_str_ptr.resize_len(
                                    (((self.max_glob_strs).wrapping_add(MAX_GLOB_STRS)) as usize)
                                        + 1,
                                );
                                self.log_realloc(
                                    "global_strs",
                                    (self.glob_str_size).wrapping_add(1i32),
                                    (self.max_glob_strs).wrapping_add(MAX_GLOB_STRS),
                                    self.max_glob_strs,
                                );
                                self.global_strs.resize_len(
                                    (((((self.max_glob_strs).wrapping_add(MAX_GLOB_STRS))
                                        .wrapping_mul((self.glob_str_size).wrapping_add(1i32)))
                                    .wrapping_sub(1i32))
                                        as usize)
                                        + 1,
                                );
                                self.log_realloc(
                                    "glb_str_end",
                                    4i32,
                                    (self.max_glob_strs).wrapping_add(MAX_GLOB_STRS),
                                    self.max_glob_strs,
                                );
                                self.glb_str_end.resize_len(
                                    (((self.max_glob_strs).wrapping_add(MAX_GLOB_STRS)) as usize)
                                        + 1,
                                );
                                self.max_glob_strs =
                                    (self.max_glob_strs).wrapping_add(MAX_GLOB_STRS);
                                self.str_glb_ptr = self.num_glb_strs;
                                while (self.str_glb_ptr < self.max_glob_strs) {
                                    {
                                        self.glb_str_ptr[(self.str_glb_ptr) as usize] = 0i32;
                                        self.glb_str_end[(self.str_glb_ptr) as usize] = 0i32;
                                        self.str_glb_ptr = (self.str_glb_ptr).wrapping_add(1i32);
                                    }
                                }
                            }
                        }
                        self.num_glb_strs = (self.num_glb_strs).wrapping_add(1i32);
                    }
                    // §207
                    {
                        if (!self.eat_bst_white_space()) {
                            {
                                self.eat_bst_print();
                                {
                                    {
                                        {
                                            crate::system::wr_str(&mut self.log_file, "strings");
                                        }
                                        {
                                            crate::system::wr_str(
                                                &mut self.standard_output,
                                                "strings",
                                            );
                                        }
                                    }
                                    {
                                        self.bst_err_print_and_look_for_blank_line();
                                        break 'l_exit_f;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
        }
    }

    /// We must attend to a few details before getting to work on this
    /// \.{.bst} command.
    /// @<Scan for and process a \.{.bst} command
    // §146
    pub fn get_bst_command_and_process(&mut self) {
        'l_exit_f: {
            if (!self.scan_alpha()) {
                {
                    {
                        {
                            let __w0 = b'"';
                            let __w1 = self.xchr[(self.buffer[(self.buf_ptr2) as usize]) as usize];
                            crate::system::wr_char(&mut self.log_file, __w0);
                            crate::system::wr_char(&mut self.log_file, __w1);
                            crate::system::wr_str(
                                &mut self.log_file,
                                "\" can't start a style-file command",
                            );
                        }
                        {
                            let __w0 = b'"';
                            let __w1 = self.xchr[(self.buffer[(self.buf_ptr2) as usize]) as usize];
                            crate::system::wr_char(&mut self.standard_output, __w0);
                            crate::system::wr_char(&mut self.standard_output, __w1);
                            crate::system::wr_str(
                                &mut self.standard_output,
                                "\" can't start a style-file command",
                            );
                        }
                    }
                    {
                        self.bst_err_print_and_look_for_blank_line();
                        break 'l_exit_f;
                    }
                }
            }
            {
                let mut __f0 = ::core::mem::take(&mut self.buffer);
                let __r = self.lower_case(
                    &mut __f0,
                    self.buf_ptr1,
                    (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                );
                self.buffer = __f0;
                __r
            };
            self.command_num = {
                let __s248 = ({
                    let mut __f0 = ::core::mem::take(&mut self.buffer);
                    let __r = self.str_lookup(
                        &mut __f0,
                        self.buf_ptr1,
                        (self.buf_ptr2).wrapping_sub(self.buf_ptr1),
                        bst_command_ilk,
                        false,
                    );
                    self.buffer = __f0;
                    __r
                }) as usize;
                self.ilk_info[__s248]
            };
            if (!self.hash_found) {
                {
                    self.print_a_token();
                    {
                        {
                            {
                                crate::system::wr_str(
                                    &mut self.log_file,
                                    " is an illegal style-file command",
                                );
                            }
                            {
                                crate::system::wr_str(
                                    &mut self.standard_output,
                                    " is an illegal style-file command",
                                );
                            }
                        }
                        {
                            self.bst_err_print_and_look_for_blank_line();
                            break 'l_exit_f;
                        }
                    }
                }
            }
            // §147
            match self.command_num {
                n_bst_entry => {
                    self.bst_entry_command();
                }
                n_bst_execute => {
                    self.bst_execute_command();
                }
                n_bst_function => {
                    self.bst_function_command();
                }
                n_bst_integers => {
                    self.bst_integers_command();
                }
                n_bst_iterate => {
                    self.bst_iterate_command();
                }
                n_bst_macro => {
                    self.bst_macro_command();
                }
                n_bst_read => {
                    self.bst_read_command();
                }
                n_bst_reverse => {
                    self.bst_reverse_command();
                }
                n_bst_sort => {
                    self.bst_sort_command();
                }
                n_bst_strings => {
                    self.bst_strings_command();
                }
                _ => {
                    {
                        {
                            crate::system::wr_str(&mut self.log_file, "Unknown style-file command");
                        }
                        {
                            crate::system::wr_str(
                                &mut self.standard_output,
                                "Unknown style-file command",
                            );
                        }
                    }
                    self.print_confusion();
                    crate::system::end_of_TEX(self);
                }
            }
        }
        // §146
    }

    /// Determine `ent_str_size`, `glob_str_size`, and `max_strings` from the
    /// environment, configuration file, or default value.  Set
    /// `hash_size:=max_strings`, but not less than `HASH_SIZE`.
    /// {`setup_bound_var` stuff adapted from \.{tex.ch}.}
    // §468
    pub fn setup_params(&mut self) {
        let mut bound_default: i32 = 0; // §468
        let mut bound_name: &'static str = Default::default(); // §468
        bound_default = ENT_STR_SIZE;
        bound_name = "ent_str_size";
        self.ent_str_size = self.setup_bound_value(bound_name, bound_default);
        if (self.ent_str_size < bound_default) {
            self.ent_str_size = bound_default;
        }
        bound_default = GLOB_STR_SIZE;
        bound_name = "glob_str_size";
        self.glob_str_size = self.setup_bound_value(bound_name, bound_default);
        if (self.glob_str_size < bound_default) {
            self.glob_str_size = bound_default;
        }
        bound_default = MAX_STRINGS;
        bound_name = "max_strings";
        self.max_strings = self.setup_bound_value(bound_name, bound_default);
        if (self.max_strings < bound_default) {
            self.max_strings = bound_default;
        }
        bound_default = MAX_PRINT_LINE;
        bound_name = "max_print_line";
        self.max_print_line = self.setup_bound_value(bound_name, bound_default);
        if (self.max_print_line < bound_default) {
            self.max_print_line = bound_default;
        }
        self.hash_size = self.max_strings;
        if (self.hash_size < HASH_SIZE) {
            self.hash_size = HASH_SIZE;
        }
        self.hash_max = ((self.hash_size).wrapping_add(hash_base)).wrapping_sub(1i32);
        self.end_of_def = (self.hash_max).wrapping_add(1i32);
        self.undefined = (self.hash_max).wrapping_add(1i32);
    }

    /// We use the algorithm from Knuth's \.{primes.web} to compute `hash_prime`
    /// as the smallest prime number not less than 85\% of `hash_size` (and
    /// `>=128`).
    // §469
    pub fn compute_hash_prime(&mut self) {
        let mut hash_want: i32 = 0; // §469
        let mut k: i32 = 0; // §469
        let mut j: i32 = 0; // §469
        let mut o: i32 = 0; // §469
        let mut square: i32 = 0; // §469
        let mut n: i32 = 0; // §469
        let mut j_prime: bool = false; // §469
        hash_want = (self.hash_size / 20i32).wrapping_mul(17i32);
        j = 1i32;
        k = 1i32;
        self.hash_prime = 2i32;
        {
            let __v249 = self.hash_prime;
            self.hash_next[(k) as usize] = __v249;
        }
        o = 2i32;
        square = 9i32;
        while (self.hash_prime < hash_want) {
            {
                loop {
                    j = (j).wrapping_add(2i32);
                    if (j == square) {
                        {
                            self.hash_text[(o) as usize] = j;
                            j = (j).wrapping_add(2i32);
                            o = (o).wrapping_add(1i32);
                            square = (self.hash_next[(o) as usize])
                                .wrapping_mul(self.hash_next[(o) as usize]);
                        }
                    }
                    n = 2i32;
                    j_prime = true;
                    while ((n < o) && j_prime) {
                        {
                            while (self.hash_text[(n) as usize] < j) {
                                {
                                    let __v250 = (self.hash_text[(n) as usize]).wrapping_add(
                                        (2i32).wrapping_mul(self.hash_next[(n) as usize]),
                                    );
                                    self.hash_text[(n) as usize] = __v250;
                                }
                            }
                            if (self.hash_text[(n) as usize] == j) {
                                j_prime = false;
                            }
                            n = (n).wrapping_add(1i32);
                        }
                    }
                    if j_prime {
                        break;
                    }
                }
                k = (k).wrapping_add(1i32);
                self.hash_prime = j;
                {
                    let __v251 = self.hash_prime;
                    self.hash_next[(k) as usize] = __v251;
                }
            }
        }
    }

    /// The \.{.bst} loop of `main_part`, which `catch_bst_done` runs: a return is
    /// cvtbib.sed's `hack2` (`break`), `jump_to_bst_done` its `longjmp(jmp32,1)`.
    /// @<Procedures and functions for about everything
    // §470
    pub fn bst_loop(&mut self) {
        'l_exit_f: {
            while true {
                {
                    if (!self.eat_bst_white_space()) {
                        break 'l_exit_f;
                    }
                    self.get_bst_command_and_process();
                }
            }
        }
    }

    /// What the main program did between `hack0` and `close_up_shop`, which
    /// `catch_close_up_shop` runs: a nonlocal `goto close_up_shop` is the
    /// `longjmp(jmp9998,1)` that leaves it.
    /// @<Procedures and functions for about everything
    // §471
    pub fn main_part(&mut self) {
        'l_L9932_f: {
            // §102
            if self.verbose {
                {
                    {
                        {
                            crate::system::wr_str(
                                &mut self.log_file,
                                "The top-level auxiliary file: ",
                            );
                        }
                        {
                            crate::system::wr_str(
                                &mut self.standard_output,
                                "The top-level auxiliary file: ",
                            );
                        }
                    }
                    self.print_aux_name();
                }
            } else {
                {
                    {
                        {
                            crate::system::wr_str(
                                &mut self.log_file,
                                "The top-level auxiliary file: ",
                            );
                        }
                    }
                    self.log_pr_aux_name();
                }
            }
            while (!self.lab31) {
                {
                    {
                        let __ix252 = self.aux_ptr;
                        let __v253 =
                            (self.aux_ln_stack[(self.aux_ptr) as usize]).wrapping_add(1i32);
                        self.aux_ln_stack[(__ix252) as usize] = __v253;
                    }
                    if (!{
                        let mut __f0 =
                            ::core::mem::take(&mut self.aux_file[(self.aux_ptr) as usize]);
                        let __r = self.input_ln(&mut __f0);
                        self.aux_file[(self.aux_ptr) as usize] = __f0;
                        __r
                    }) {
                        self.pop_the_aux_stack();
                    } else {
                        self.get_aux_command_and_process();
                    }
                }
            }
            self.last_check_for_aux_errors();
            // §143
            if (self.bst_str == 0i32) {
                break 'l_L9932_f;
            }
            self.bst_line_num = 0i32;
            self.bbl_line_num = 1i32;
            self.buf_ptr2 = self.last;
            self.catch_bst_done();
            {
                let mut __f0 = ::core::mem::take(&mut self.bst_file);
                let __r = self.a_close(&mut __f0);
                self.bst_file = __f0;
                __r
            };
        }
        {
            let mut __f0 = ::core::mem::take(&mut self.bbl_file);
            let __r = self.a_close(&mut __f0);
            self.bbl_file = __f0;
            __r
        };
        // §471
    }

    /// This procedure gets things started properly.
    /// @<The procedure `initialize`
    // §9
    pub fn initialize(&mut self) {
        let mut i: i32 = 0; // §18
        let mut k: hash_loc = 0; // §59
                                 // §13
        self.bad = 0i32;
        if (min_print_line < 3i32) {
            self.bad = 1i32;
        }
        if (self.max_print_line <= min_print_line) {
            self.bad = ((10i32).wrapping_mul(self.bad)).wrapping_add(2i32);
        }
        if (self.max_print_line >= self.buf_size) {
            self.bad = ((10i32).wrapping_mul(self.bad)).wrapping_add(3i32);
        }
        if (self.hash_prime < 128i32) {
            self.bad = ((10i32).wrapping_mul(self.bad)).wrapping_add(4i32);
        }
        if (self.hash_prime > self.hash_size) {
            self.bad = ((10i32).wrapping_mul(self.bad)).wrapping_add(5i32);
        }
        if (hash_base != 1i32) {
            self.bad = ((10i32).wrapping_mul(self.bad)).wrapping_add(6i32);
        }
        if (self.max_strings > self.hash_size) {
            self.bad = ((10i32).wrapping_mul(self.bad)).wrapping_add(7i32);
        }
        if (self.max_cites > self.max_strings) {
            self.bad = ((10i32).wrapping_mul(self.bad)).wrapping_add(8i32);
        }
        // §293
        if (short_list < ((2i32).wrapping_mul(end_offset)).wrapping_add(2i32)) {
            self.bad = ((100i32).wrapping_mul(self.bad)).wrapping_add(22i32);
        }
        // §9
        if (self.bad > 0i32) {
            {
                {
                    let __w0 = self.bad;
                    crate::system::wr_int(&mut self.standard_output, __w0, 0i32);
                    crate::system::wr_str(&mut self.standard_output, " is a bad bad");
                    crate::system::wr_ln(&mut self.standard_output);
                }
                self.uexit(1i32);
            }
        }
        // §16
        self.history = spotless;
        // §20
        self.xchr[(32i32) as usize] = b' ';
        self.xchr[(33i32) as usize] = b'!';
        self.xchr[(34i32) as usize] = b'"';
        self.xchr[(35i32) as usize] = b'#';
        self.xchr[(36i32) as usize] = b'$';
        self.xchr[(37i32) as usize] = b'%';
        self.xchr[(38i32) as usize] = b'&';
        self.xchr[(39i32) as usize] = b'\'';
        self.xchr[(40i32) as usize] = b'(';
        self.xchr[(41i32) as usize] = b')';
        self.xchr[(42i32) as usize] = b'*';
        self.xchr[(43i32) as usize] = b'+';
        self.xchr[(44i32) as usize] = b',';
        self.xchr[(45i32) as usize] = b'-';
        self.xchr[(46i32) as usize] = b'.';
        self.xchr[(47i32) as usize] = b'/';
        self.xchr[(48i32) as usize] = b'0';
        self.xchr[(49i32) as usize] = b'1';
        self.xchr[(50i32) as usize] = b'2';
        self.xchr[(51i32) as usize] = b'3';
        self.xchr[(52i32) as usize] = b'4';
        self.xchr[(53i32) as usize] = b'5';
        self.xchr[(54i32) as usize] = b'6';
        self.xchr[(55i32) as usize] = b'7';
        self.xchr[(56i32) as usize] = b'8';
        self.xchr[(57i32) as usize] = b'9';
        self.xchr[(58i32) as usize] = b':';
        self.xchr[(59i32) as usize] = b';';
        self.xchr[(60i32) as usize] = b'<';
        self.xchr[(61i32) as usize] = b'=';
        self.xchr[(62i32) as usize] = b'>';
        self.xchr[(63i32) as usize] = b'?';
        self.xchr[(64i32) as usize] = b'@';
        self.xchr[(65i32) as usize] = b'A';
        self.xchr[(66i32) as usize] = b'B';
        self.xchr[(67i32) as usize] = b'C';
        self.xchr[(68i32) as usize] = b'D';
        self.xchr[(69i32) as usize] = b'E';
        self.xchr[(70i32) as usize] = b'F';
        self.xchr[(71i32) as usize] = b'G';
        self.xchr[(72i32) as usize] = b'H';
        self.xchr[(73i32) as usize] = b'I';
        self.xchr[(74i32) as usize] = b'J';
        self.xchr[(75i32) as usize] = b'K';
        self.xchr[(76i32) as usize] = b'L';
        self.xchr[(77i32) as usize] = b'M';
        self.xchr[(78i32) as usize] = b'N';
        self.xchr[(79i32) as usize] = b'O';
        self.xchr[(80i32) as usize] = b'P';
        self.xchr[(81i32) as usize] = b'Q';
        self.xchr[(82i32) as usize] = b'R';
        self.xchr[(83i32) as usize] = b'S';
        self.xchr[(84i32) as usize] = b'T';
        self.xchr[(85i32) as usize] = b'U';
        self.xchr[(86i32) as usize] = b'V';
        self.xchr[(87i32) as usize] = b'W';
        self.xchr[(88i32) as usize] = b'X';
        self.xchr[(89i32) as usize] = b'Y';
        self.xchr[(90i32) as usize] = b'Z';
        self.xchr[(91i32) as usize] = b'[';
        self.xchr[(92i32) as usize] = b'\\';
        self.xchr[(93i32) as usize] = b']';
        self.xchr[(94i32) as usize] = b'^';
        self.xchr[(95i32) as usize] = b'_';
        self.xchr[(96i32) as usize] = b'`';
        self.xchr[(97i32) as usize] = b'a';
        self.xchr[(98i32) as usize] = b'b';
        self.xchr[(99i32) as usize] = b'c';
        self.xchr[(100i32) as usize] = b'd';
        self.xchr[(101i32) as usize] = b'e';
        self.xchr[(102i32) as usize] = b'f';
        self.xchr[(103i32) as usize] = b'g';
        self.xchr[(104i32) as usize] = b'h';
        self.xchr[(105i32) as usize] = b'i';
        self.xchr[(106i32) as usize] = b'j';
        self.xchr[(107i32) as usize] = b'k';
        self.xchr[(108i32) as usize] = b'l';
        self.xchr[(109i32) as usize] = b'm';
        self.xchr[(110i32) as usize] = b'n';
        self.xchr[(111i32) as usize] = b'o';
        self.xchr[(112i32) as usize] = b'p';
        self.xchr[(113i32) as usize] = b'q';
        self.xchr[(114i32) as usize] = b'r';
        self.xchr[(115i32) as usize] = b's';
        self.xchr[(116i32) as usize] = b't';
        self.xchr[(117i32) as usize] = b'u';
        self.xchr[(118i32) as usize] = b'v';
        self.xchr[(119i32) as usize] = b'w';
        self.xchr[(120i32) as usize] = b'x';
        self.xchr[(121i32) as usize] = b'y';
        self.xchr[(122i32) as usize] = b'z';
        self.xchr[(123i32) as usize] = b'{';
        self.xchr[(124i32) as usize] = b'|';
        self.xchr[(125i32) as usize] = b'}';
        self.xchr[(126i32) as usize] = b'~';
        self.xchr[(0i32) as usize] = b' ';
        self.xchr[(127i32) as usize] = b' ';
        // §22
        {
            let __for_end_2 = 31i32;
            i = 0i32;
            while i <= __for_end_2 {
                {
                    let __v254 = ((i) as u8);
                    self.xchr[(i) as usize] = __v254;
                }
                i = i.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 255i32;
            i = 127i32;
            while i <= __for_end_2 {
                {
                    let __v255 = ((i) as u8);
                    self.xchr[(i) as usize] = __v255;
                }
                i = i.wrapping_add(1);
            }
        }
        // §23
        {
            let __for_end_2 = last_text_char;
            i = first_text_char;
            while i <= __for_end_2 {
                self.xord[(self.xchr[(i) as usize]) as usize] = i;
                i = i.wrapping_add(1);
            }
        }
        // §27
        {
            let __for_end_2 = 127i32;
            i = 0i32;
            while i <= __for_end_2 {
                self.lex_class[(i) as usize] = other_lex;
                i = i.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 255i32;
            i = 128i32;
            while i <= __for_end_2 {
                self.lex_class[(i) as usize] = alpha;
                i = i.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 31i32;
            i = 0i32;
            while i <= __for_end_2 {
                self.lex_class[(i) as usize] = illegal;
                i = i.wrapping_add(1);
            }
        }
        self.lex_class[(invalid_code) as usize] = illegal;
        self.lex_class[(tab) as usize] = white_space;
        self.lex_class[(13i32) as usize] = white_space;
        self.lex_class[(space) as usize] = white_space;
        self.lex_class[(tie) as usize] = sep_char;
        self.lex_class[(hyphen) as usize] = sep_char;
        {
            let __for_end_2 = 57i32;
            i = 48i32;
            while i <= __for_end_2 {
                self.lex_class[(i) as usize] = numeric;
                i = i.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 90i32;
            i = 65i32;
            while i <= __for_end_2 {
                self.lex_class[(i) as usize] = alpha;
                i = i.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 122i32;
            i = 97i32;
            while i <= __for_end_2 {
                self.lex_class[(i) as usize] = alpha;
                i = i.wrapping_add(1);
            }
        }
        // §28
        {
            let __for_end_2 = 255i32;
            i = 0i32;
            while i <= __for_end_2 {
                self.id_class[(i) as usize] = legal_id_char;
                i = i.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 31i32;
            i = 0i32;
            while i <= __for_end_2 {
                self.id_class[(i) as usize] = illegal_id_char;
                i = i.wrapping_add(1);
            }
        }
        self.id_class[(space) as usize] = illegal_id_char;
        self.id_class[(tab) as usize] = illegal_id_char;
        self.id_class[(double_quote) as usize] = illegal_id_char;
        self.id_class[(number_sign) as usize] = illegal_id_char;
        self.id_class[(comment) as usize] = illegal_id_char;
        self.id_class[(single_quote) as usize] = illegal_id_char;
        self.id_class[(left_paren) as usize] = illegal_id_char;
        self.id_class[(right_paren) as usize] = illegal_id_char;
        self.id_class[(comma) as usize] = illegal_id_char;
        self.id_class[(equals_sign) as usize] = illegal_id_char;
        self.id_class[(left_brace) as usize] = illegal_id_char;
        self.id_class[(right_brace) as usize] = illegal_id_char;
        // §30
        {
            let __for_end_2 = 127i32;
            i = 0i32;
            while i <= __for_end_2 {
                self.char_width[(i) as usize] = 0i32;
                i = i.wrapping_add(1);
            }
        }
        self.char_width[(32i32) as usize] = 278i32;
        self.char_width[(33i32) as usize] = 278i32;
        self.char_width[(34i32) as usize] = 500i32;
        self.char_width[(35i32) as usize] = 833i32;
        self.char_width[(36i32) as usize] = 500i32;
        self.char_width[(37i32) as usize] = 833i32;
        self.char_width[(38i32) as usize] = 778i32;
        self.char_width[(39i32) as usize] = 278i32;
        self.char_width[(40i32) as usize] = 389i32;
        self.char_width[(41i32) as usize] = 389i32;
        self.char_width[(42i32) as usize] = 500i32;
        self.char_width[(43i32) as usize] = 778i32;
        self.char_width[(44i32) as usize] = 278i32;
        self.char_width[(45i32) as usize] = 333i32;
        self.char_width[(46i32) as usize] = 278i32;
        self.char_width[(47i32) as usize] = 500i32;
        self.char_width[(48i32) as usize] = 500i32;
        self.char_width[(49i32) as usize] = 500i32;
        self.char_width[(50i32) as usize] = 500i32;
        self.char_width[(51i32) as usize] = 500i32;
        self.char_width[(52i32) as usize] = 500i32;
        self.char_width[(53i32) as usize] = 500i32;
        self.char_width[(54i32) as usize] = 500i32;
        self.char_width[(55i32) as usize] = 500i32;
        self.char_width[(56i32) as usize] = 500i32;
        self.char_width[(57i32) as usize] = 500i32;
        self.char_width[(58i32) as usize] = 278i32;
        self.char_width[(59i32) as usize] = 278i32;
        self.char_width[(60i32) as usize] = 278i32;
        self.char_width[(61i32) as usize] = 778i32;
        self.char_width[(62i32) as usize] = 472i32;
        self.char_width[(63i32) as usize] = 472i32;
        self.char_width[(64i32) as usize] = 778i32;
        self.char_width[(65i32) as usize] = 750i32;
        self.char_width[(66i32) as usize] = 708i32;
        self.char_width[(67i32) as usize] = 722i32;
        self.char_width[(68i32) as usize] = 764i32;
        self.char_width[(69i32) as usize] = 681i32;
        self.char_width[(70i32) as usize] = 653i32;
        self.char_width[(71i32) as usize] = 785i32;
        self.char_width[(72i32) as usize] = 750i32;
        self.char_width[(73i32) as usize] = 361i32;
        self.char_width[(74i32) as usize] = 514i32;
        self.char_width[(75i32) as usize] = 778i32;
        self.char_width[(76i32) as usize] = 625i32;
        self.char_width[(77i32) as usize] = 917i32;
        self.char_width[(78i32) as usize] = 750i32;
        self.char_width[(79i32) as usize] = 778i32;
        self.char_width[(80i32) as usize] = 681i32;
        self.char_width[(81i32) as usize] = 778i32;
        self.char_width[(82i32) as usize] = 736i32;
        self.char_width[(83i32) as usize] = 556i32;
        self.char_width[(84i32) as usize] = 722i32;
        self.char_width[(85i32) as usize] = 750i32;
        self.char_width[(86i32) as usize] = 750i32;
        self.char_width[(87i32) as usize] = 1028i32;
        self.char_width[(88i32) as usize] = 750i32;
        self.char_width[(89i32) as usize] = 750i32;
        self.char_width[(90i32) as usize] = 611i32;
        self.char_width[(91i32) as usize] = 278i32;
        self.char_width[(92i32) as usize] = 500i32;
        self.char_width[(93i32) as usize] = 278i32;
        self.char_width[(94i32) as usize] = 500i32;
        self.char_width[(95i32) as usize] = 278i32;
        self.char_width[(96i32) as usize] = 278i32;
        self.char_width[(97i32) as usize] = 500i32;
        self.char_width[(98i32) as usize] = 556i32;
        self.char_width[(99i32) as usize] = 444i32;
        self.char_width[(100i32) as usize] = 556i32;
        self.char_width[(101i32) as usize] = 444i32;
        self.char_width[(102i32) as usize] = 306i32;
        self.char_width[(103i32) as usize] = 500i32;
        self.char_width[(104i32) as usize] = 556i32;
        self.char_width[(105i32) as usize] = 278i32;
        self.char_width[(106i32) as usize] = 306i32;
        self.char_width[(107i32) as usize] = 528i32;
        self.char_width[(108i32) as usize] = 278i32;
        self.char_width[(109i32) as usize] = 833i32;
        self.char_width[(110i32) as usize] = 556i32;
        self.char_width[(111i32) as usize] = 500i32;
        self.char_width[(112i32) as usize] = 556i32;
        self.char_width[(113i32) as usize] = 528i32;
        self.char_width[(114i32) as usize] = 392i32;
        self.char_width[(115i32) as usize] = 394i32;
        self.char_width[(116i32) as usize] = 389i32;
        self.char_width[(117i32) as usize] = 556i32;
        self.char_width[(118i32) as usize] = 528i32;
        self.char_width[(119i32) as usize] = 722i32;
        self.char_width[(120i32) as usize] = 528i32;
        self.char_width[(121i32) as usize] = 528i32;
        self.char_width[(122i32) as usize] = 444i32;
        self.char_width[(123i32) as usize] = 500i32;
        self.char_width[(124i32) as usize] = 1000i32;
        self.char_width[(125i32) as usize] = 500i32;
        self.char_width[(126i32) as usize] = 500i32;
        // §60
        {
            let __for_end_2 = self.hash_max;
            k = hash_base;
            while k <= __for_end_2 {
                {
                    self.hash_next[(k) as usize] = empty;
                    self.hash_text[(k) as usize] = 0i32;
                }
                k = k.wrapping_add(1);
            }
        }
        self.hash_used = (self.hash_max).wrapping_add(1i32);
        // §65
        self.pool_ptr = 0i32;
        self.str_ptr = 1i32;
        {
            let __ix256 = self.str_ptr;
            let __v257 = self.pool_ptr;
            self.str_start[(__ix256) as usize] = __v257;
        }
        // §111
        self.bib_ptr = 0i32;
        self.bib_seen = false;
        // §117
        self.bst_str = 0i32;
        self.bst_seen = false;
        // §123
        self.cite_ptr = 0i32;
        self.citation_seen = false;
        self.all_entries = false;
        // §154
        self.wiz_def_ptr = 0i32;
        self.num_ent_ints = 0i32;
        self.num_ent_strs = 0i32;
        self.num_fields = 0i32;
        self.str_glb_ptr = 0i32;
        while (self.str_glb_ptr < self.max_glob_strs) {
            {
                self.glb_str_ptr[(self.str_glb_ptr) as usize] = 0i32;
                self.glb_str_end[(self.str_glb_ptr) as usize] = 0i32;
                self.str_glb_ptr = (self.str_glb_ptr).wrapping_add(1i32);
            }
        }
        self.num_glb_strs = 0i32;
        // §156
        self.entry_seen = false;
        self.read_seen = false;
        self.read_performed = false;
        self.reading_completed = false;
        self.read_completed = false;
        // §188
        self.impl_fn_num = 0i32;
        // §283
        self.out_buf_length = 0i32;
        // §9
        self.pre_def_certain_strings();
        self.get_the_top_level_aux_file_name();
    }

    /// System-dependent changes.
    /// @<Define \(p)`parse_arguments`
    // §458
    pub fn init_option_variables(&mut self) {
        // §461
        self.verbose = true;
        // §464
        self.min_crossrefs = 2i32;
    }
}
