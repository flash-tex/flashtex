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
    /// It's often illegal to end a \.{.bib} command in certain places, and
    /// this is where we come to check.
    // §220
    pub fn eat_bib_print(&mut self) {
        'l_exit_f: {
            {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "Illegal end of database file");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "Illegal end of database file");
                    }
                }
                self.bib_err_print();
                break 'l_exit_f;
            }
        }
    }

    /// And here are a bunch of error-message macros, each called more than
    /// once, that thus save space as implemented.  This one is for when one
    /// of two possible characters is expected while scanning.
    // §221
    pub fn bib_one_of_two_print(&mut self, mut char1: ASCII_code, mut char2: ASCII_code) {
        'l_exit_f: {
            {
                {
                    {
                        let __w1 = self.xchr[(char1) as usize];
                        let __w3 = self.xchr[(char2) as usize];
                        let __w4 = b'\'';
                        crate::system::wr_str(&mut self.log_file, "I was expecting a `");
                        crate::system::wr_char(&mut self.log_file, __w1);
                        crate::system::wr_str(&mut self.log_file, "' or a `");
                        crate::system::wr_char(&mut self.log_file, __w3);
                        crate::system::wr_char(&mut self.log_file, __w4);
                    }
                    {
                        let __w1 = self.xchr[(char1) as usize];
                        let __w3 = self.xchr[(char2) as usize];
                        let __w4 = b'\'';
                        crate::system::wr_str(&mut self.standard_output, "I was expecting a `");
                        crate::system::wr_char(&mut self.standard_output, __w1);
                        crate::system::wr_str(&mut self.standard_output, "' or a `");
                        crate::system::wr_char(&mut self.standard_output, __w3);
                        crate::system::wr_char(&mut self.standard_output, __w4);
                    }
                }
                self.bib_err_print();
                break 'l_exit_f;
            }
        }
    }

    /// This one's for an expected `equals_sign`.
    // §222
    pub fn bib_equals_sign_print(&mut self) {
        'l_exit_f: {
            {
                {
                    {
                        let __w1 = self.xchr[(equals_sign) as usize];
                        let __w2 = b'"';
                        crate::system::wr_str(&mut self.log_file, "I was expecting an \"");
                        crate::system::wr_char(&mut self.log_file, __w1);
                        crate::system::wr_char(&mut self.log_file, __w2);
                    }
                    {
                        let __w1 = self.xchr[(equals_sign) as usize];
                        let __w2 = b'"';
                        crate::system::wr_str(&mut self.standard_output, "I was expecting an \"");
                        crate::system::wr_char(&mut self.standard_output, __w1);
                        crate::system::wr_char(&mut self.standard_output, __w2);
                    }
                }
                self.bib_err_print();
                break 'l_exit_f;
            }
        }
    }

    /// This complains about unbalanced braces.
    // §223
    pub fn bib_unbalanced_braces_print(&mut self) {
        'l_exit_f: {
            {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "Unbalanced braces");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "Unbalanced braces");
                    }
                }
                self.bib_err_print();
                break 'l_exit_f;
            }
        }
    }

    /// And this one about an overly exuberant field.
    // §224
    pub fn bib_field_too_long_print(&mut self) {
        'l_exit_f: {
            {
                {
                    {
                        let __w1 = self.buf_size;
                        crate::system::wr_str(&mut self.log_file, "Your field is more than ");
                        crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                        crate::system::wr_str(&mut self.log_file, " characters");
                    }
                    {
                        let __w1 = self.buf_size;
                        crate::system::wr_str(&mut self.standard_output, "Your field is more than ");
                        crate::system::wr_int(&mut self.standard_output, __w1, 0i32);
                        crate::system::wr_str(&mut self.standard_output, " characters");
                    }
                }
                self.bib_err_print();
                break 'l_exit_f;
            }
        }
    }

    /// This one is just a warning, not an error.  It's for when something
    /// isn't (or might not be) quite right with a macro name.
    // §225
    pub fn macro_warn_print(&mut self) {
        {
            {
                crate::system::wr_str(&mut self.log_file, "Warning--string name \"");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "Warning--string name \"");
            }
        }
        self.print_a_token();
        {
            {
                crate::system::wr_str(&mut self.log_file, "\" is ");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "\" is ");
            }
        }
    }

    /// This macro is used to scan all \.{.bib} identifiers.  The argument
    /// tells what was happening at the time.  The associated procedure simply
    /// prints an error message.
    // §226
    pub fn bib_id_print(&mut self) {
        if (self.scan_result == id_null) {
            {
                {
                    crate::system::wr_str(&mut self.log_file, "You're missing ");
                }
                {
                    crate::system::wr_str(&mut self.standard_output, "You're missing ");
                }
            }
        } else {
            if (self.scan_result == other_char_adjacent) {
                {
                    {
                        let __w0 = b'"';
                        let __w1 = self.xchr[(self.buffer[(self.buf_ptr2) as usize]) as usize];
                        crate::system::wr_char(&mut self.log_file, __w0);
                        crate::system::wr_char(&mut self.log_file, __w1);
                        crate::system::wr_str(&mut self.log_file, "\" immediately follows ");
                    }
                    {
                        let __w0 = b'"';
                        let __w1 = self.xchr[(self.buffer[(self.buf_ptr2) as usize]) as usize];
                        crate::system::wr_char(&mut self.standard_output, __w0);
                        crate::system::wr_char(&mut self.standard_output, __w1);
                        crate::system::wr_str(&mut self.standard_output, "\" immediately follows ");
                    }
                }
            } else {
                self.id_scanning_confusion();
            }
        }
    }

    /// Here's another bug.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §231
    pub fn bib_cmd_confusion(&mut self) {
        {
            {
                {
                    crate::system::wr_str(&mut self.log_file, "Unknown database-file command");
                }
                {
                    crate::system::wr_str(&mut self.standard_output, "Unknown database-file command");
                }
            }
            self.print_confusion();
            crate::system::end_of_TEX(self);
        }
    }

    /// Here's another bug complaint.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §262
    pub fn cite_key_disappeared_confusion(&mut self) {
        {
            {
                {
                    crate::system::wr_str(&mut self.log_file, "A cite key disappeared");
                }
                {
                    crate::system::wr_str(&mut self.standard_output, "A cite key disappeared");
                }
            }
            self.print_confusion();
            crate::system::end_of_TEX(self);
        }
    }

    /// This procedure exists to save space, since it's used twice---once for
    /// each of the two succeeding modules.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §271
    pub fn bad_cross_reference_print(&mut self, mut s: str_number) {
        {
            {
                crate::system::wr_str(&mut self.log_file, "--entry \"");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "--entry \"");
            }
        }
        self.print_a_pool_str(self.cite_list[(self.cite_ptr) as usize]);
        {
            {
                let __w0 = b'"';
                crate::system::wr_char(&mut self.log_file, __w0);
                crate::system::wr_ln(&mut self.log_file);
            }
            {
                let __w0 = b'"';
                crate::system::wr_char(&mut self.standard_output, __w0);
                crate::system::wr_ln(&mut self.standard_output);
            }
        }
        {
            {
                crate::system::wr_str(&mut self.log_file, "refers to entry \"");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "refers to entry \"");
            }
        }
        self.print_a_pool_str(s);
    }

    /// When an entry being cross referenced doesn't exist on `cite_list`, we
    /// complain.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §272
    pub fn nonexistent_cross_reference_error(&mut self) {
        {
            {
                crate::system::wr_str(&mut self.log_file, "A bad cross reference-");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "A bad cross reference-");
            }
        }
        self.bad_cross_reference_print(self.field_info[(self.field_ptr) as usize]);
        {
            {
                crate::system::wr_str(&mut self.log_file, "\", which doesn't exist");
                crate::system::wr_ln(&mut self.log_file);
            }
            {
                crate::system::wr_str(&mut self.standard_output, "\", which doesn't exist");
                crate::system::wr_ln(&mut self.standard_output);
            }
        }
        self.mark_error();
    }

    /// When a cite key on the original `cite_list` (or added to `cite_list`
    /// because of cross~referencing) didn't appear in the database, complain.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §275
    pub fn print_missing_entry(&mut self, mut s: str_number) {
        {
            {
                crate::system::wr_str(&mut self.log_file, "Warning--I didn't find a database entry for \"");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "Warning--I didn't find a database entry for \"");
            }
        }
        self.print_a_pool_str(s);
        {
            {
                let __w0 = b'"';
                crate::system::wr_char(&mut self.log_file, __w0);
                crate::system::wr_ln(&mut self.log_file);
            }
            {
                let __w0 = b'"';
                crate::system::wr_char(&mut self.standard_output, __w0);
                crate::system::wr_ln(&mut self.standard_output);
            }
        }
        self.mark_warning();
    }

    /// When there's an error while executing \.{.bst} functions, what we do
    /// depends on whether the function is messing with the entries.
    /// Furthermore this error is serious enough to classify as an
    /// `error_message` instead of a `warning_message`.  These messages (that
    /// is, from `bst_ex_warn`) are meant both for the user and for the style
    /// designer while debugging.
    // §284
    pub fn bst_ex_warn_print(&mut self) {
        if self.mess_with_entries {
            {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, " for entry ");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, " for entry ");
                    }
                }
                self.print_a_pool_str(self.cite_list[(self.cite_ptr) as usize]);
            }
        }
        self.print_a_newline();
        {
            {
                crate::system::wr_str(&mut self.log_file, "while executing-");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "while executing-");
            }
        }
        self.bst_ln_num_print();
        self.mark_error();
    }

    /// When an error is so harmless, we print a `warning_message` instead of
    /// an `error_message`.
    // §285
    pub fn bst_mild_ex_warn_print(&mut self) {
        if self.mess_with_entries {
            {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, " for entry ");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, " for entry ");
                    }
                }
                self.print_a_pool_str(self.cite_list[(self.cite_ptr) as usize]);
            }
        }
        self.print_a_newline();
        {
            {
                {
                    crate::system::wr_str(&mut self.log_file, "while executing");
                }
                {
                    crate::system::wr_str(&mut self.standard_output, "while executing");
                }
            }
            self.bst_warn_print();
        }
    }

    /// It's illegal to mess with the entry information at certain times;
    /// here's a complaint for these times.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §286
    pub fn bst_cant_mess_with_entries_print(&mut self) {
        {
            {
                {
                    crate::system::wr_str(&mut self.log_file, "You can't mess with entries here");
                }
                {
                    crate::system::wr_str(&mut self.standard_output, "You can't mess with entries here");
                }
            }
            self.bst_ex_warn_print();
        }
    }

    /// More bug complaints, this time about bad literals.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §301
    pub fn illegl_literal_confusion(&mut self) {
        {
            {
                {
                    crate::system::wr_str(&mut self.log_file, "Illegal literal type");
                }
                {
                    crate::system::wr_str(&mut self.standard_output, "Illegal literal type");
                }
            }
            self.print_confusion();
            crate::system::end_of_TEX(self);
        }
    }

    /// More bug complaints, this time about bad literals.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §301
    pub fn unknwn_literal_confusion(&mut self) {
        {
            {
                {
                    crate::system::wr_str(&mut self.log_file, "Unknown literal type");
                }
                {
                    crate::system::wr_str(&mut self.standard_output, "Unknown literal type");
                }
            }
            self.print_confusion();
            crate::system::end_of_TEX(self);
        }
    }

    /// Occasionally we'll want to know what's on the literal stack.  Here we
    /// print out a stack literal, giving its type.  This procedure should
    /// never be called after popping an empty stack.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §302
    pub fn print_stk_lit(&mut self, mut stk_lt: i32, mut stk_tp: stk_type) {
        match stk_tp {
            stk_int => {
                {
                    {
                        let __w0 = stk_lt;
                        crate::system::wr_int(&mut self.log_file, __w0, 0i32);
                        crate::system::wr_str(&mut self.log_file, " is an integer literal");
                    }
                    {
                        let __w0 = stk_lt;
                        crate::system::wr_int(&mut self.standard_output, __w0, 0i32);
                        crate::system::wr_str(&mut self.standard_output, " is an integer literal");
                    }
                }
            }
            stk_str => {
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
                    self.print_a_pool_str(stk_lt);
                    {
                        {
                            crate::system::wr_str(&mut self.log_file, "\" is a string literal");
                        }
                        {
                            crate::system::wr_str(&mut self.standard_output, "\" is a string literal");
                        }
                    }
                }
            }
            stk_fn => {
                {
                    {
                        {
                            let __w0 = b'`';
                            crate::system::wr_char(&mut self.log_file, __w0);
                        }
                        {
                            let __w0 = b'`';
                            crate::system::wr_char(&mut self.standard_output, __w0);
                        }
                    }
                    self.print_a_pool_str(self.hash_text[(stk_lt) as usize]);
                    {
                        {
                            crate::system::wr_str(&mut self.log_file, "' is a function literal");
                        }
                        {
                            crate::system::wr_str(&mut self.standard_output, "' is a function literal");
                        }
                    }
                }
            }
            stk_field_missing => {
                {
                    {
                        {
                            let __w0 = b'`';
                            crate::system::wr_char(&mut self.log_file, __w0);
                        }
                        {
                            let __w0 = b'`';
                            crate::system::wr_char(&mut self.standard_output, __w0);
                        }
                    }
                    self.print_a_pool_str(stk_lt);
                    {
                        {
                            crate::system::wr_str(&mut self.log_file, "' is a missing field");
                        }
                        {
                            crate::system::wr_str(&mut self.standard_output, "' is a missing field");
                        }
                    }
                }
            }
            stk_empty => {
                self.illegl_literal_confusion();
            }
            _ => {
                self.unknwn_literal_confusion();
            }
        }
    }

    /// This is similar to `print_stk_lit`, but here we don't give the
    /// literal's type, and here we end with a new line.  This procedure
    /// should never be called after popping an empty stack.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §304
    pub fn print_lit(&mut self, mut stk_lt: i32, mut stk_tp: stk_type) {
        match stk_tp {
            stk_int => {
                {
                    {
                        let __w0 = stk_lt;
                        crate::system::wr_int(&mut self.log_file, __w0, 0i32);
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        let __w0 = stk_lt;
                        crate::system::wr_int(&mut self.standard_output, __w0, 0i32);
                        crate::system::wr_ln(&mut self.standard_output);
                    }
                }
            }
            stk_str => {
                {
                    self.print_a_pool_str(stk_lt);
                    self.print_a_newline();
                }
            }
            stk_fn => {
                {
                    self.print_a_pool_str(self.hash_text[(stk_lt) as usize]);
                    self.print_a_newline();
                }
            }
            stk_field_missing => {
                {
                    self.print_a_pool_str(stk_lt);
                    self.print_a_newline();
                }
            }
            stk_empty => {
                self.illegl_literal_confusion();
            }
            _ => {
                self.unknwn_literal_confusion();
            }
        }
    }

    /// This procedure actually writes onto the \.{.bbl}~file a line of output
    /// (the characters from `out_buf[0]` to `out_buf[out_buf_length-1]`,
    /// after removing trailing `white_space` characters).  It also updates
    /// `bbl_line_num`, the line counter.  It writes a blank line if and only
    /// if `out_buf` is empty.  The program uses this procedure in such a way
    /// that `out_buf` will be nonempty if there have been characters put in
    /// it since the most recent \.{newline\$}.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §312
    pub fn output_bbl_line(&mut self) {
        'l_exit_f: {
            if (self.out_buf_length != 0i32) {
                {
                    'l_L15_f: {
                        while (self.out_buf_length > 0i32) {
                            if (self.lex_class[(self.out_buf[((self.out_buf_length).wrapping_sub(1i32)) as usize]) as usize] == white_space) {
                                self.out_buf_length = (self.out_buf_length).wrapping_sub(1i32);
                            } else {
                                break 'l_L15_f;
                            }
                        }
                    }
                    if (self.out_buf_length == 0i32) {
                        break 'l_exit_f;
                    }
                    self.out_buf_ptr = 0i32;
                    while (self.out_buf_ptr < self.out_buf_length) {
                        {
                            {
                                let __w0 = self.xchr[(self.out_buf[(self.out_buf_ptr) as usize]) as usize];
                                crate::system::wr_char(&mut self.bbl_file, __w0);
                            }
                            self.out_buf_ptr = (self.out_buf_ptr).wrapping_add(1i32);
                        }
                    }
                }
            }
            {
                crate::system::wr_ln(&mut self.bbl_file);
            }
            self.bbl_line_num = (self.bbl_line_num).wrapping_add(1i32);
            self.out_buf_length = 0i32;
        }
    }

    /// It's time for a complaint if either of the two (entry or global)
    /// string lengths is exceeded.
    // §347
    pub fn bst_1print_string_size_exceeded(&mut self) {
        {
            {
                crate::system::wr_str(&mut self.log_file, "Warning--you've exceeded ");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "Warning--you've exceeded ");
            }
        }
    }

    /// It's time for a complaint if either of the two (entry or global)
    /// string lengths is exceeded.
    // §347
    pub fn bst_2print_string_size_exceeded(&mut self) {
        {
            {
                crate::system::wr_str(&mut self.log_file, "-string-size,");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "-string-size,");
            }
        }
        self.bst_mild_ex_warn_print();
        {
            {
                crate::system::wr_str(&mut self.log_file, "*Please notify the bibstyle designer*");
                crate::system::wr_ln(&mut self.log_file);
            }
            {
                crate::system::wr_str(&mut self.standard_output, "*Please notify the bibstyle designer*");
                crate::system::wr_ln(&mut self.standard_output);
            }
        }
    }

    /// This complaint often arises because the style designer has to type
    /// lots of braces.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §359
    pub fn braces_unbalanced_complaint(&mut self, mut pop_lit_var: str_number) {
        {
            {
                crate::system::wr_str(&mut self.log_file, "Warning--\"");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "Warning--\"");
            }
        }
        self.print_a_pool_str(pop_lit_var);
        {
            {
                {
                    crate::system::wr_str(&mut self.log_file, "\" isn't a brace-balanced string");
                }
                {
                    crate::system::wr_str(&mut self.standard_output, "\" isn't a brace-balanced string");
                }
            }
            self.bst_mild_ex_warn_print();
        }
    }

    /// Another bug complaint.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §364
    pub fn case_conversion_confusion(&mut self) {
        {
            {
                {
                    crate::system::wr_str(&mut self.log_file, "Unknown type of case conversion");
                }
                {
                    crate::system::wr_str(&mut self.standard_output, "Unknown type of case conversion");
                }
            }
            self.print_confusion();
            crate::system::end_of_TEX(self);
        }
    }

}
