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
    /// This procedure scans for an identifier, stopping at the first
    /// `illegal_id_char`, or stopping at the first character if it's
    /// `numeric`.  It sets the global variable `scan_result` to `id_null` if
    /// the identifier is null, else to `white_adjacent` if it ended at a
    /// `white_space` character or an end-of-line, else to
    /// `specified_char_adjacent` if it ended at one of `char1` or `char2` or
    /// `char3`, else to `other_char_adjacent` if it ended at a nonspecified,
    /// non`white_space` `illegal_id_char`.  By convention, when some calling
    /// code really wants just one or two ``specified'' characters, it merely
    /// repeats one of the characters.
    /// @<Procedures and functions for input scanning
    // §83
    pub fn scan_identifier(&mut self, mut char1: ASCII_code, mut char2: ASCII_code, mut char3: ASCII_code) {
        self.buf_ptr1 = self.buf_ptr2;
        if (self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] != numeric) {
            while ((self.id_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] == legal_id_char) && (self.buf_ptr2 < self.last)) {
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            }
        }
        if ((self.buf_ptr2).wrapping_sub(self.buf_ptr1) == 0i32) {
            self.scan_result = id_null;
        } else {
            if ((self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] == white_space) || (self.buf_ptr2 == self.last)) {
                self.scan_result = white_adjacent;
            } else {
                if (((self.buffer[(self.buf_ptr2) as usize] == char1) || (self.buffer[(self.buf_ptr2) as usize] == char2)) || (self.buffer[(self.buf_ptr2) as usize] == char3)) {
                    self.scan_result = specified_char_adjacent;
                } else {
                    self.scan_result = other_char_adjacent;
                }
            }
        }
    }

    /// This function scans for a nonnegative integer, stopping at the first
    /// nondigit; it sets the value of `token_value` accordingly.  It returns
    /// `true` if the token was a legal nonnegative integer (i.e., consisted
    /// of one or more digits).
    /// @<Procedures and functions for input scanning
    // §85
    pub fn scan_nonneg_integer(&mut self) -> bool {
        let mut scan_nonneg_integer: bool = false;
        self.buf_ptr1 = self.buf_ptr2;
        self.token_value = 0i32;
        while ((self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] == numeric) && (self.buf_ptr2 < self.last)) {
            {
                self.token_value = ((self.token_value).wrapping_mul(10i32)).wrapping_add((self.buffer[(self.buf_ptr2) as usize]).wrapping_sub(48i32));
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            }
        }
        if ((self.buf_ptr2).wrapping_sub(self.buf_ptr1) == 0i32) {
            scan_nonneg_integer = false;
        } else {
            scan_nonneg_integer = true;
        }
        scan_nonneg_integer
    }

    /// This procedure scans for an integer, stopping at the first nondigit;
    /// it sets the value of `token_value` accordingly.  It returns `true` if
    /// the token was a legal integer (i.e., consisted of an optional
    /// `minus_sign` followed by one or more digits).
    // §86
    pub fn scan_integer(&mut self) -> bool {
        let mut scan_integer: bool = false;
        let mut sign_length: i32 = 0; // §86
        self.buf_ptr1 = self.buf_ptr2;
        if (self.buffer[(self.buf_ptr2) as usize] == minus_sign) {
            {
                sign_length = 1i32;
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            }
        } else {
            sign_length = 0i32;
        }
        self.token_value = 0i32;
        while ((self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] == numeric) && (self.buf_ptr2 < self.last)) {
            {
                self.token_value = ((self.token_value).wrapping_mul(10i32)).wrapping_add((self.buffer[(self.buf_ptr2) as usize]).wrapping_sub(48i32));
                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            }
        }
        if (sign_length == 1i32) {
            self.token_value = (self.token_value).wrapping_neg();
        }
        if ((self.buf_ptr2).wrapping_sub(self.buf_ptr1) == sign_length) {
            scan_integer = false;
        } else {
            scan_integer = true;
        }
        scan_integer
    }

    /// This function scans over `white_space` characters, stopping either at
    /// the first nonwhite character or the end of the line, respectively
    /// returning `true` or `false`.
    /// @<Procedures and functions for input scanning
    // §87
    pub fn scan_white_space(&mut self) -> bool {
        let mut scan_white_space: bool = false;
        while ((self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] == white_space) && (self.buf_ptr2 < self.last)) {
            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
        }
        if (self.buf_ptr2 < self.last) {
            scan_white_space = true;
        } else {
            scan_white_space = false;
        }
        scan_white_space
    }

    /// This \.{.bst}-specific scanning function skips over `white_space`
    /// characters (and comments) until hitting a nonwhite character or the
    /// end of the file, respectively returning `true` or `false`.  It also
    /// updates `bst_line_num`, the line counter.
    /// @<Procedures and functions for input scanning
    // §144
    pub fn eat_bst_white_space(&mut self) -> bool {
        let mut eat_bst_white_space: bool = false;
        'l_exit_f: {
            while true {
                {
                    if self.scan_white_space() {
                        if (self.buffer[(self.buf_ptr2) as usize] != comment) {
                            {
                                eat_bst_white_space = true;
                                break 'l_exit_f;
                            }
                        }
                    }
                    if (!{ let mut __f0 = ::core::mem::take(&mut self.bst_file); let __r = self.input_ln(&mut __f0); self.bst_file = __f0; __r }) {
                        {
                            eat_bst_white_space = false;
                            break 'l_exit_f;
                        }
                    }
                    self.bst_line_num = (self.bst_line_num).wrapping_add(1i32);
                    self.buf_ptr2 = 0i32;
                }
            }
        }
        eat_bst_white_space
    }

    /// We're about to start scanning tokens in a function definition.  When a
    /// function token is illegal, we skip until it ends; a `white_space`
    /// character, an end-of-line, a `right_brace`, or a `comment` marks the
    /// end of the current token.
    // §175
    pub fn skip_token_print(&mut self) {
        {
            {
                let __w0 = b'-';
                crate::system::wr_char(&mut self.log_file, __w0);
            }
            {
                let __w0 = b'-';
                crate::system::wr_char(&mut self.standard_output, __w0);
            }
        }
        self.bst_ln_num_print();
        self.mark_error();
        if self.scan2_white(right_brace, comment) {
        }
    }

    /// This macro is similar to the last one but is specifically for
    /// recursion in a `wiz_defined` function, which is illegal; it helps save
    /// space.
    // §176
    pub fn print_recursion_illegal(&mut self) {
        {
            {
                crate::system::wr_str(&mut self.log_file, "Curse you, wizard, before you recurse me:");
                crate::system::wr_ln(&mut self.log_file);
            }
            {
                crate::system::wr_str(&mut self.standard_output, "Curse you, wizard, before you recurse me:");
                crate::system::wr_ln(&mut self.standard_output);
            }
        }
        {
            {
                crate::system::wr_str(&mut self.log_file, "function ");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "function ");
            }
        }
        self.print_a_token();
        {
            {
                crate::system::wr_str(&mut self.log_file, " is illegal in its own definition");
                crate::system::wr_ln(&mut self.log_file);
            }
            {
                crate::system::wr_str(&mut self.standard_output, " is illegal in its own definition");
                crate::system::wr_ln(&mut self.standard_output);
            }
        }
        self.skip_token_print();
    }

    /// Here's another macro for saving some space when there's a problem with
    /// a token.
    // §177
    pub fn skp_token_unknown_function_print(&mut self) {
        self.print_a_token();
        {
            {
                crate::system::wr_str(&mut self.log_file, " is an unknown function");
            }
            {
                crate::system::wr_str(&mut self.standard_output, " is an unknown function");
            }
        }
        self.skip_token_print();
    }

    /// And another.
    // §178
    pub fn skip_illegal_stuff_after_token_print(&mut self) {
        {
            {
                let __w0 = b'"';
                let __w1 = self.xchr[(self.buffer[(self.buf_ptr2) as usize]) as usize];
                crate::system::wr_char(&mut self.log_file, __w0);
                crate::system::wr_char(&mut self.log_file, __w1);
                crate::system::wr_str(&mut self.log_file, "\" can't follow a literal");
            }
            {
                let __w0 = b'"';
                let __w1 = self.xchr[(self.buffer[(self.buf_ptr2) as usize]) as usize];
                crate::system::wr_char(&mut self.standard_output, __w0);
                crate::system::wr_char(&mut self.standard_output, __w1);
                crate::system::wr_str(&mut self.standard_output, "\" can't follow a literal");
            }
        }
        self.skip_token_print();
    }

    /// This recursive function reads and stores the list of functions
    /// (separated by `white_space` characters or ends-of-line) that define
    /// this new function, and reads a `right_brace`.
    /// @<Procedures and functions for input scanning
    // §179
    pub fn scan_fn_def(&mut self, mut fn_hash_loc: hash_loc) {
        let mut singl_function: Vec<hash_ptr2> = Vec::new(); // §179
        let mut single_fn_space: i32 = 0; // §179
        let mut single_ptr: fn_def_loc = 0; // §179
        let mut copy_ptr: fn_def_loc = 0; // §179
        let mut end_of_num: buf_pointer = 0; // §179
        let mut impl_fn_loc: hash_loc = 0; // §179
        'l_exit_f: {
            single_fn_space = SINGLE_FN_SPACE;
            singl_function = vec![0; ((single_fn_space) as usize) + 1];
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
            single_ptr = 0i32;
            while (self.buffer[(self.buf_ptr2) as usize] != right_brace) {
                {
                    'l_L25_f: {
                        // §181
                        match self.buffer[(self.buf_ptr2) as usize] {
                            number_sign => {
                                // §182
                                {
                                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                    if (!self.scan_integer()) {
                                        {
                                            {
                                                {
                                                    crate::system::wr_str(&mut self.log_file, "Illegal integer in integer literal");
                                                }
                                                {
                                                    crate::system::wr_str(&mut self.standard_output, "Illegal integer in integer literal");
                                                }
                                            }
                                            self.skip_token_print();
                                            break 'l_L25_f;
                                        }
                                    }
                                    self.literal_loc = { let mut __f0 = ::core::mem::take(&mut self.buffer); let __r = self.str_lookup(&mut __f0, self.buf_ptr1, (self.buf_ptr2).wrapping_sub(self.buf_ptr1), integer_ilk, true); self.buffer = __f0; __r };
                                    if (!self.hash_found) {
                                        {
                                            self.fn_type[(self.literal_loc) as usize] = int_literal;
                                            { let __ix27 = self.literal_loc; let __v28 = self.token_value; self.ilk_info[(__ix27) as usize] = __v28; }
                                        }
                                    }
                                    if ((((self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] != white_space) && (self.buf_ptr2 < self.last)) && (self.buffer[(self.buf_ptr2) as usize] != right_brace)) && (self.buffer[(self.buf_ptr2) as usize] != comment)) {
                                        {
                                            self.skip_illegal_stuff_after_token_print();
                                            break 'l_L25_f;
                                        }
                                    }
                                    {
                                        { let __v29 = self.literal_loc; singl_function[(single_ptr) as usize] = __v29; }
                                        if (single_ptr == single_fn_space) {
                                            {
                                                self.log_realloc("singl_function", 4i32, (single_fn_space).wrapping_add(SINGLE_FN_SPACE), single_fn_space);
                                                { let __n30 = (((single_fn_space).wrapping_add(SINGLE_FN_SPACE)) as usize) + 1; singl_function.resize(__n30, 0); }
                                                single_fn_space = (single_fn_space).wrapping_add(SINGLE_FN_SPACE);
                                            }
                                        }
                                        single_ptr = (single_ptr).wrapping_add(1i32);
                                    }
                                }
                            }
                            double_quote => {
                                // §183
                                {
                                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                    if (!self.scan1(double_quote)) {
                                        {
                                            {
                                                {
                                                    let __w1 = self.xchr[(double_quote) as usize];
                                                    crate::system::wr_str(&mut self.log_file, "No `");
                                                    crate::system::wr_char(&mut self.log_file, __w1);
                                                    crate::system::wr_str(&mut self.log_file, "' to end string literal");
                                                }
                                                {
                                                    let __w1 = self.xchr[(double_quote) as usize];
                                                    crate::system::wr_str(&mut self.standard_output, "No `");
                                                    crate::system::wr_char(&mut self.standard_output, __w1);
                                                    crate::system::wr_str(&mut self.standard_output, "' to end string literal");
                                                }
                                            }
                                            self.skip_token_print();
                                            break 'l_L25_f;
                                        }
                                    }
                                    self.literal_loc = { let mut __f0 = ::core::mem::take(&mut self.buffer); let __r = self.str_lookup(&mut __f0, self.buf_ptr1, (self.buf_ptr2).wrapping_sub(self.buf_ptr1), text_ilk, true); self.buffer = __f0; __r };
                                    self.fn_type[(self.literal_loc) as usize] = str_literal;
                                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                    if ((((self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] != white_space) && (self.buf_ptr2 < self.last)) && (self.buffer[(self.buf_ptr2) as usize] != right_brace)) && (self.buffer[(self.buf_ptr2) as usize] != comment)) {
                                        {
                                            self.skip_illegal_stuff_after_token_print();
                                            break 'l_L25_f;
                                        }
                                    }
                                    {
                                        { let __v31 = self.literal_loc; singl_function[(single_ptr) as usize] = __v31; }
                                        if (single_ptr == single_fn_space) {
                                            {
                                                self.log_realloc("singl_function", 4i32, (single_fn_space).wrapping_add(SINGLE_FN_SPACE), single_fn_space);
                                                { let __n32 = (((single_fn_space).wrapping_add(SINGLE_FN_SPACE)) as usize) + 1; singl_function.resize(__n32, 0); }
                                                single_fn_space = (single_fn_space).wrapping_add(SINGLE_FN_SPACE);
                                            }
                                        }
                                        single_ptr = (single_ptr).wrapping_add(1i32);
                                    }
                                }
                            }
                            single_quote => {
                                // §184
                                {
                                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                    if self.scan2_white(right_brace, comment) {
                                    }
                                    { let mut __f0 = ::core::mem::take(&mut self.buffer); let __r = self.lower_case(&mut __f0, self.buf_ptr1, (self.buf_ptr2).wrapping_sub(self.buf_ptr1)); self.buffer = __f0; __r };
                                    self.fn_loc = { let mut __f0 = ::core::mem::take(&mut self.buffer); let __r = self.str_lookup(&mut __f0, self.buf_ptr1, (self.buf_ptr2).wrapping_sub(self.buf_ptr1), bst_fn_ilk, false); self.buffer = __f0; __r };
                                    if (!self.hash_found) {
                                        {
                                            self.skp_token_unknown_function_print();
                                            break 'l_L25_f;
                                        }
                                    } else {
                                        // §185
                                        {
                                            if (self.fn_loc == self.wiz_loc) {
                                                {
                                                    self.print_recursion_illegal();
                                                    break 'l_L25_f;
                                                }
                                            } else {
                                                {
                                                    {
                                                        { let __v33 = quote_next_fn; singl_function[(single_ptr) as usize] = __v33; }
                                                        if (single_ptr == single_fn_space) {
                                                            {
                                                                self.log_realloc("singl_function", 4i32, (single_fn_space).wrapping_add(SINGLE_FN_SPACE), single_fn_space);
                                                                { let __n34 = (((single_fn_space).wrapping_add(SINGLE_FN_SPACE)) as usize) + 1; singl_function.resize(__n34, 0); }
                                                                single_fn_space = (single_fn_space).wrapping_add(SINGLE_FN_SPACE);
                                                            }
                                                        }
                                                        single_ptr = (single_ptr).wrapping_add(1i32);
                                                    }
                                                    {
                                                        { let __v35 = self.fn_loc; singl_function[(single_ptr) as usize] = __v35; }
                                                        if (single_ptr == single_fn_space) {
                                                            {
                                                                self.log_realloc("singl_function", 4i32, (single_fn_space).wrapping_add(SINGLE_FN_SPACE), single_fn_space);
                                                                { let __n36 = (((single_fn_space).wrapping_add(SINGLE_FN_SPACE)) as usize) + 1; singl_function.resize(__n36, 0); }
                                                                single_fn_space = (single_fn_space).wrapping_add(SINGLE_FN_SPACE);
                                                            }
                                                        }
                                                        single_ptr = (single_ptr).wrapping_add(1i32);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            left_brace => {
                                // §186
                                {
                                    self.ex_buf[(0i32) as usize] = single_quote;
                                    { let mut __f1 = ::core::mem::take(&mut self.ex_buf); let mut __f3 = ::core::mem::take(&mut end_of_num); let __r = self.int_to_ASCII(self.impl_fn_num, &mut __f1, 1i32, &mut __f3); self.ex_buf = __f1; end_of_num = __f3; __r };
                                    impl_fn_loc = { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.str_lookup(&mut __f0, 0i32, end_of_num, bst_fn_ilk, true); self.ex_buf = __f0; __r };
                                    if self.hash_found {
                                        {
                                            {
                                                {
                                                    crate::system::wr_str(&mut self.log_file, "Already encountered implicit function");
                                                }
                                                {
                                                    crate::system::wr_str(&mut self.standard_output, "Already encountered implicit function");
                                                }
                                            }
                                            self.print_confusion();
                                            crate::system::end_of_TEX(self);
                                        }
                                    }
                                    self.impl_fn_num = (self.impl_fn_num).wrapping_add(1i32);
                                    self.fn_type[(impl_fn_loc) as usize] = wiz_defined;
                                    {
                                        { let __v37 = quote_next_fn; singl_function[(single_ptr) as usize] = __v37; }
                                        if (single_ptr == single_fn_space) {
                                            {
                                                self.log_realloc("singl_function", 4i32, (single_fn_space).wrapping_add(SINGLE_FN_SPACE), single_fn_space);
                                                { let __n38 = (((single_fn_space).wrapping_add(SINGLE_FN_SPACE)) as usize) + 1; singl_function.resize(__n38, 0); }
                                                single_fn_space = (single_fn_space).wrapping_add(SINGLE_FN_SPACE);
                                            }
                                        }
                                        single_ptr = (single_ptr).wrapping_add(1i32);
                                    }
                                    {
                                        singl_function[(single_ptr) as usize] = impl_fn_loc;
                                        if (single_ptr == single_fn_space) {
                                            {
                                                self.log_realloc("singl_function", 4i32, (single_fn_space).wrapping_add(SINGLE_FN_SPACE), single_fn_space);
                                                { let __n39 = (((single_fn_space).wrapping_add(SINGLE_FN_SPACE)) as usize) + 1; singl_function.resize(__n39, 0); }
                                                single_fn_space = (single_fn_space).wrapping_add(SINGLE_FN_SPACE);
                                            }
                                        }
                                        single_ptr = (single_ptr).wrapping_add(1i32);
                                    }
                                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                    self.scan_fn_def(impl_fn_loc);
                                }
                            }
                            _ => {
                                // §191
                                {
                                    if self.scan2_white(right_brace, comment) {
                                    }
                                    { let mut __f0 = ::core::mem::take(&mut self.buffer); let __r = self.lower_case(&mut __f0, self.buf_ptr1, (self.buf_ptr2).wrapping_sub(self.buf_ptr1)); self.buffer = __f0; __r };
                                    self.fn_loc = { let mut __f0 = ::core::mem::take(&mut self.buffer); let __r = self.str_lookup(&mut __f0, self.buf_ptr1, (self.buf_ptr2).wrapping_sub(self.buf_ptr1), bst_fn_ilk, false); self.buffer = __f0; __r };
                                    if (!self.hash_found) {
                                        {
                                            self.skp_token_unknown_function_print();
                                            break 'l_L25_f;
                                        }
                                    } else {
                                        if (self.fn_loc == self.wiz_loc) {
                                            {
                                                self.print_recursion_illegal();
                                                break 'l_L25_f;
                                            }
                                        } else {
                                            {
                                                {
                                                    { let __v40 = self.fn_loc; singl_function[(single_ptr) as usize] = __v40; }
                                                    if (single_ptr == single_fn_space) {
                                                        {
                                                            self.log_realloc("singl_function", 4i32, (single_fn_space).wrapping_add(SINGLE_FN_SPACE), single_fn_space);
                                                            { let __n41 = (((single_fn_space).wrapping_add(SINGLE_FN_SPACE)) as usize) + 1; singl_function.resize(__n41, 0); }
                                                            single_fn_space = (single_fn_space).wrapping_add(SINGLE_FN_SPACE);
                                                        }
                                                    }
                                                    single_ptr = (single_ptr).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // §179
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
                }
            }
            // §192
            {
                {
                    { let __v42 = self.end_of_def; singl_function[(single_ptr) as usize] = __v42; }
                    if (single_ptr == single_fn_space) {
                        {
                            self.log_realloc("singl_function", 4i32, (single_fn_space).wrapping_add(SINGLE_FN_SPACE), single_fn_space);
                            { let __n43 = (((single_fn_space).wrapping_add(SINGLE_FN_SPACE)) as usize) + 1; singl_function.resize(__n43, 0); }
                            single_fn_space = (single_fn_space).wrapping_add(SINGLE_FN_SPACE);
                        }
                    }
                    single_ptr = (single_ptr).wrapping_add(1i32);
                }
                while ((single_ptr).wrapping_add(self.wiz_def_ptr) > self.wiz_fn_space) {
                    {
                        self.log_realloc("wiz_functions", 4i32, (self.wiz_fn_space).wrapping_add(WIZ_FN_SPACE), self.wiz_fn_space);
                        self.wiz_functions.resize_len((((self.wiz_fn_space).wrapping_add(WIZ_FN_SPACE)) as usize) + 1);
                        self.wiz_fn_space = (self.wiz_fn_space).wrapping_add(WIZ_FN_SPACE);
                    }
                }
                { let __v44 = self.wiz_def_ptr; self.ilk_info[(fn_hash_loc) as usize] = __v44; }
                copy_ptr = 0i32;
                while (copy_ptr < single_ptr) {
                    {
                        self.wiz_functions[(self.wiz_def_ptr) as usize] = singl_function[(copy_ptr) as usize];
                        copy_ptr = (copy_ptr).wrapping_add(1i32);
                        self.wiz_def_ptr = (self.wiz_def_ptr).wrapping_add(1i32);
                    }
                }
            }
            // §179
            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
        }
    }

    /// Before we actually start the code for reading a database file, we must
    /// define this \.{.bib}-specific scanning function.  It skips over
    /// `white_space` characters until hitting a nonwhite character or the end
    /// of the file, respectively returning `true` or `false`.  It also
    /// updates `bib_line_num`, the line counter.
    /// @<Procedures and functions for input scanning
    // §219
    pub fn eat_bib_white_space(&mut self) -> bool {
        let mut eat_bib_white_space: bool = false;
        'l_exit_f: {
            while (!self.scan_white_space()) {
                {
                    if (!{ let mut __f0 = ::core::mem::take(&mut self.bib_file[(self.bib_ptr) as usize]); let __r = self.input_ln(&mut __f0); self.bib_file[(self.bib_ptr) as usize] = __f0; __r }) {
                        {
                            eat_bib_white_space = false;
                            break 'l_exit_f;
                        }
                    }
                    self.bib_line_num = (self.bib_line_num).wrapping_add(1i32);
                    self.buf_ptr2 = 0i32;
                }
            }
            eat_bib_white_space = true;
        }
        eat_bib_white_space
    }

    /// The \.{.bib}-specific scanning function `compress_bib_white` skips
    /// over `white_space` characters within a string until hitting a nonwhite
    /// character; in fact, it does everything `eat_bib_white_space` does, but
    /// it also adds a `space` to `field_vl_str`.  This function is never
    /// called if there are no `white_space` characters (or ends-of-line) to
    /// be scanned (though the associated macro might be).  The function
    /// returns `false` if there is a serious syntax error.
    // §243
    pub fn compress_bib_white(&mut self) -> bool {
        let mut compress_bib_white: bool = false;
        'l_exit_f: {
            compress_bib_white = false;
            {
                if (self.ex_buf_ptr >= self.buf_size) {
                    {
                        {
                            {
                                let __w1 = space;
                                crate::system::wr_str(&mut self.log_file, "Field filled up at ");
                                crate::system::wr_int(&mut self.log_file, __w1, 1);
                                crate::system::wr_str(&mut self.log_file, ", reallocating.");
                            }
                        }
                        {
                            {
                                crate::system::wr_ln(&mut self.log_file);
                            }
                        }
                        self.buffer_overflow();
                    }
                }
                self.ex_buf[(self.ex_buf_ptr) as usize] = space;
                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
            }
            while (!self.scan_white_space()) {
                {
                    if (!{ let mut __f0 = ::core::mem::take(&mut self.bib_file[(self.bib_ptr) as usize]); let __r = self.input_ln(&mut __f0); self.bib_file[(self.bib_ptr) as usize] = __f0; __r }) {
                        {
                            self.eat_bib_print();
                            break 'l_exit_f;
                        }
                    }
                    self.bib_line_num = (self.bib_line_num).wrapping_add(1i32);
                    self.buf_ptr2 = 0i32;
                }
            }
            compress_bib_white = true;
        }
        compress_bib_white
    }

    /// This \.{.bib}-specific function scans a string with balanced braces,
    /// stopping just past the matching `right_str_delim`.  How much work it
    /// does depends on whether `store_field = true`.  It returns `false` if
    /// there was a serious syntax error.
    /// @<The scanning function `scan_balanced_braces`
    // §244
    pub fn scan_balanced_braces(&mut self) -> bool {
        let mut scan_balanced_braces: bool = false;
        'l_exit_f: {
            scan_balanced_braces = false;
            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            {
                if ((self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] == white_space) || (self.buf_ptr2 == self.last)) {
                    if (!self.compress_bib_white()) {
                        break 'l_exit_f;
                    }
                }
            }
            if (self.ex_buf_ptr > 1i32) {
                if (self.ex_buf[((self.ex_buf_ptr).wrapping_sub(1i32)) as usize] == space) {
                    if (self.ex_buf[((self.ex_buf_ptr).wrapping_sub(2i32)) as usize] == space) {
                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_sub(1i32);
                    }
                }
            }
            self.bib_brace_level = 0i32;
            if self.store_field {
                // §247
                {
                    while (self.buffer[(self.buf_ptr2) as usize] != self.right_str_delim) {
                        match self.buffer[(self.buf_ptr2) as usize] {
                            left_brace => {
                                {
                                    self.bib_brace_level = (self.bib_brace_level).wrapping_add(1i32);
                                    {
                                        if (self.ex_buf_ptr >= self.buf_size) {
                                            {
                                                {
                                                    {
                                                        let __w1 = left_brace;
                                                        crate::system::wr_str(&mut self.log_file, "Field filled up at ");
                                                        crate::system::wr_int(&mut self.log_file, __w1, 1);
                                                        crate::system::wr_str(&mut self.log_file, ", reallocating.");
                                                    }
                                                }
                                                {
                                                    {
                                                        crate::system::wr_ln(&mut self.log_file);
                                                    }
                                                }
                                                self.buffer_overflow();
                                            }
                                        }
                                        self.ex_buf[(self.ex_buf_ptr) as usize] = left_brace;
                                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                    }
                                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                    {
                                        if ((self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] == white_space) || (self.buf_ptr2 == self.last)) {
                                            if (!self.compress_bib_white()) {
                                                break 'l_exit_f;
                                            }
                                        }
                                    }
                                    // §248
                                    {
                                        'l_L15_f: {
                                            while true {
                                                match self.buffer[(self.buf_ptr2) as usize] {
                                                    right_brace => {
                                                        {
                                                            self.bib_brace_level = (self.bib_brace_level).wrapping_sub(1i32);
                                                            {
                                                                if (self.ex_buf_ptr >= self.buf_size) {
                                                                    {
                                                                        {
                                                                            {
                                                                                let __w1 = right_brace;
                                                                                crate::system::wr_str(&mut self.log_file, "Field filled up at ");
                                                                                crate::system::wr_int(&mut self.log_file, __w1, 1);
                                                                                crate::system::wr_str(&mut self.log_file, ", reallocating.");
                                                                            }
                                                                        }
                                                                        {
                                                                            {
                                                                                crate::system::wr_ln(&mut self.log_file);
                                                                            }
                                                                        }
                                                                        self.buffer_overflow();
                                                                    }
                                                                }
                                                                self.ex_buf[(self.ex_buf_ptr) as usize] = right_brace;
                                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                            }
                                                            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                                            {
                                                                if ((self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] == white_space) || (self.buf_ptr2 == self.last)) {
                                                                    if (!self.compress_bib_white()) {
                                                                        break 'l_exit_f;
                                                                    }
                                                                }
                                                            }
                                                            if (self.bib_brace_level == 0i32) {
                                                                break 'l_L15_f;
                                                            }
                                                        }
                                                    }
                                                    left_brace => {
                                                        {
                                                            self.bib_brace_level = (self.bib_brace_level).wrapping_add(1i32);
                                                            {
                                                                if (self.ex_buf_ptr >= self.buf_size) {
                                                                    {
                                                                        {
                                                                            {
                                                                                let __w1 = left_brace;
                                                                                crate::system::wr_str(&mut self.log_file, "Field filled up at ");
                                                                                crate::system::wr_int(&mut self.log_file, __w1, 1);
                                                                                crate::system::wr_str(&mut self.log_file, ", reallocating.");
                                                                            }
                                                                        }
                                                                        {
                                                                            {
                                                                                crate::system::wr_ln(&mut self.log_file);
                                                                            }
                                                                        }
                                                                        self.buffer_overflow();
                                                                    }
                                                                }
                                                                self.ex_buf[(self.ex_buf_ptr) as usize] = left_brace;
                                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                            }
                                                            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                                            {
                                                                if ((self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] == white_space) || (self.buf_ptr2 == self.last)) {
                                                                    if (!self.compress_bib_white()) {
                                                                        break 'l_exit_f;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    _ => {
                                                        {
                                                            {
                                                                if (self.ex_buf_ptr >= self.buf_size) {
                                                                    {
                                                                        {
                                                                            {
                                                                                let __w1 = self.buffer[(self.buf_ptr2) as usize];
                                                                                crate::system::wr_str(&mut self.log_file, "Field filled up at ");
                                                                                crate::system::wr_int(&mut self.log_file, __w1, 1);
                                                                                crate::system::wr_str(&mut self.log_file, ", reallocating.");
                                                                            }
                                                                        }
                                                                        {
                                                                            {
                                                                                crate::system::wr_ln(&mut self.log_file);
                                                                            }
                                                                        }
                                                                        self.buffer_overflow();
                                                                    }
                                                                }
                                                                { let __ix45 = self.ex_buf_ptr; let __v46 = self.buffer[(self.buf_ptr2) as usize]; self.ex_buf[(__ix45) as usize] = __v46; }
                                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                            }
                                                            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                                            {
                                                                if ((self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] == white_space) || (self.buf_ptr2 == self.last)) {
                                                                    if (!self.compress_bib_white()) {
                                                                        break 'l_exit_f;
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
                            right_brace => {
                                // §247
                                {
                                    self.bib_unbalanced_braces_print();
                                    break 'l_exit_f;
                                }
                            }
                            _ => {
                                {
                                    {
                                        if (self.ex_buf_ptr >= self.buf_size) {
                                            {
                                                {
                                                    {
                                                        let __w1 = self.buffer[(self.buf_ptr2) as usize];
                                                        crate::system::wr_str(&mut self.log_file, "Field filled up at ");
                                                        crate::system::wr_int(&mut self.log_file, __w1, 1);
                                                        crate::system::wr_str(&mut self.log_file, ", reallocating.");
                                                    }
                                                }
                                                {
                                                    {
                                                        crate::system::wr_ln(&mut self.log_file);
                                                    }
                                                }
                                                self.buffer_overflow();
                                            }
                                        }
                                        { let __ix47 = self.ex_buf_ptr; let __v48 = self.buffer[(self.buf_ptr2) as usize]; self.ex_buf[(__ix47) as usize] = __v48; }
                                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                    }
                                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                    {
                                        if ((self.lex_class[(self.buffer[(self.buf_ptr2) as usize]) as usize] == white_space) || (self.buf_ptr2 == self.last)) {
                                            if (!self.compress_bib_white()) {
                                                break 'l_exit_f;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // §245
                {
                    while (self.buffer[(self.buf_ptr2) as usize] != self.right_str_delim) {
                        if (self.buffer[(self.buf_ptr2) as usize] == left_brace) {
                            {
                                self.bib_brace_level = (self.bib_brace_level).wrapping_add(1i32);
                                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                {
                                    if (!self.eat_bib_white_space()) {
                                        {
                                            self.eat_bib_print();
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                                while (self.bib_brace_level > 0i32) {
                                    // §246
                                    {
                                        if (self.buffer[(self.buf_ptr2) as usize] == right_brace) {
                                            {
                                                self.bib_brace_level = (self.bib_brace_level).wrapping_sub(1i32);
                                                self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                                {
                                                    if (!self.eat_bib_white_space()) {
                                                        {
                                                            self.eat_bib_print();
                                                            break 'l_exit_f;
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            if (self.buffer[(self.buf_ptr2) as usize] == left_brace) {
                                                {
                                                    self.bib_brace_level = (self.bib_brace_level).wrapping_add(1i32);
                                                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                                    {
                                                        if (!self.eat_bib_white_space()) {
                                                            {
                                                                self.eat_bib_print();
                                                                break 'l_exit_f;
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                {
                                                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                                    if (!self.scan2(right_brace, left_brace)) {
                                                        {
                                                            if (!self.eat_bib_white_space()) {
                                                                {
                                                                    self.eat_bib_print();
                                                                    break 'l_exit_f;
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
                        } else {
                            // §245
                            if (self.buffer[(self.buf_ptr2) as usize] == right_brace) {
                                {
                                    self.bib_unbalanced_braces_print();
                                    break 'l_exit_f;
                                }
                            } else {
                                {
                                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                                    if (!self.scan3(self.right_str_delim, left_brace, right_brace)) {
                                        {
                                            if (!self.eat_bib_white_space()) {
                                                {
                                                    self.eat_bib_print();
                                                    break 'l_exit_f;
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
            // §244
            self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
            scan_balanced_braces = true;
        }
        scan_balanced_braces
    }

    /// Each field token is either a nonnegative number, a macro name (like
    /// `jan'), or a brace-balanced string delimited by either `double_quote`s
    /// or braces.  Thus there are four possibilities for the first character
    /// of the field token: If it's a `left_brace` or a `double_quote`, the
    /// token (with balanced braces, up to the matching `right_str_delim`) is
    /// a string; if it's `numeric`, the token is a number; if it's anything
    /// else, the token is a macro name (and should thus have been defined by
    /// either the \.{.bst}-file's \.{macro} command or the \.{.bib}-file's
    /// \.{string} command).  This function returns `false` if there was a
    /// serious syntax error.
    /// @<The scanning function `scan_a_field_token_and_eat_white`
    // §241
    pub fn scan_a_field_token_and_eat_white(&mut self) -> bool {
        let mut scan_a_field_token_and_eat_white: bool = false;
        'l_exit_f: {
            scan_a_field_token_and_eat_white = false;
            match self.buffer[(self.buf_ptr2) as usize] {
                left_brace => {
                    {
                        self.right_str_delim = right_brace;
                        if (!self.scan_balanced_braces()) {
                            break 'l_exit_f;
                        }
                    }
                }
                double_quote => {
                    {
                        self.right_str_delim = double_quote;
                        if (!self.scan_balanced_braces()) {
                            break 'l_exit_f;
                        }
                    }
                }
                48 | 49 | 50 | 51 | 52 | 53 | 54 | 55 | 56 | 57 => {
                    // §249
                    {
                        if (!self.scan_nonneg_integer()) {
                            {
                                {
                                    {
                                        crate::system::wr_str(&mut self.log_file, "A digit disappeared");
                                    }
                                    {
                                        crate::system::wr_str(&mut self.standard_output, "A digit disappeared");
                                    }
                                }
                                self.print_confusion();
                                crate::system::end_of_TEX(self);
                            }
                        }
                        if self.store_field {
                            {
                                self.tmp_ptr = self.buf_ptr1;
                                while (self.tmp_ptr < self.buf_ptr2) {
                                    {
                                        {
                                            if (self.ex_buf_ptr >= self.buf_size) {
                                                {
                                                    {
                                                        {
                                                            let __w1 = self.buffer[(self.tmp_ptr) as usize];
                                                            crate::system::wr_str(&mut self.log_file, "Field filled up at ");
                                                            crate::system::wr_int(&mut self.log_file, __w1, 1);
                                                            crate::system::wr_str(&mut self.log_file, ", reallocating.");
                                                        }
                                                    }
                                                    {
                                                        {
                                                            crate::system::wr_ln(&mut self.log_file);
                                                        }
                                                    }
                                                    self.buffer_overflow();
                                                }
                                            }
                                            { let __ix49 = self.ex_buf_ptr; let __v50 = self.buffer[(self.tmp_ptr) as usize]; self.ex_buf[(__ix49) as usize] = __v50; }
                                            self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                        }
                                        self.tmp_ptr = (self.tmp_ptr).wrapping_add(1i32);
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {
                    // §250
                    {
                        self.scan_identifier(comma, self.right_outer_delim, concat_char);
                        {
                            if ((self.scan_result == white_adjacent) || (self.scan_result == specified_char_adjacent)) {
                            } else {
                                {
                                    self.bib_id_print();
                                    {
                                        {
                                            {
                                                crate::system::wr_str(&mut self.log_file, "a field part");
                                            }
                                            {
                                                crate::system::wr_str(&mut self.standard_output, "a field part");
                                            }
                                        }
                                        self.bib_err_print();
                                        break 'l_exit_f;
                                    }
                                }
                            }
                        }
                        if self.store_field {
                            {
                                { let mut __f0 = ::core::mem::take(&mut self.buffer); let __r = self.lower_case(&mut __f0, self.buf_ptr1, (self.buf_ptr2).wrapping_sub(self.buf_ptr1)); self.buffer = __f0; __r };
                                self.macro_name_loc = { let mut __f0 = ::core::mem::take(&mut self.buffer); let __r = self.str_lookup(&mut __f0, self.buf_ptr1, (self.buf_ptr2).wrapping_sub(self.buf_ptr1), macro_ilk, false); self.buffer = __f0; __r };
                                self.store_token = true;
                                if self.at_bib_command {
                                    if (self.command_num == n_bib_string) {
                                        if (self.macro_name_loc == self.cur_macro_loc) {
                                            {
                                                self.store_token = false;
                                                {
                                                    self.macro_warn_print();
                                                    {
                                                        {
                                                            {
                                                                crate::system::wr_str(&mut self.log_file, "used in its own definition");
                                                                crate::system::wr_ln(&mut self.log_file);
                                                            }
                                                            {
                                                                crate::system::wr_str(&mut self.standard_output, "used in its own definition");
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
                                if (!self.hash_found) {
                                    {
                                        self.store_token = false;
                                        {
                                            self.macro_warn_print();
                                            {
                                                {
                                                    {
                                                        crate::system::wr_str(&mut self.log_file, "undefined");
                                                        crate::system::wr_ln(&mut self.log_file);
                                                    }
                                                    {
                                                        crate::system::wr_str(&mut self.standard_output, "undefined");
                                                        crate::system::wr_ln(&mut self.standard_output);
                                                    }
                                                }
                                                self.bib_warn_print();
                                            }
                                        }
                                    }
                                }
                                if self.store_token {
                                    // §251
                                    {
                                        self.tmp_ptr = self.str_start[(self.ilk_info[(self.macro_name_loc) as usize]) as usize];
                                        self.tmp_end_ptr = self.str_start[((self.ilk_info[(self.macro_name_loc) as usize]).wrapping_add(1i32)) as usize];
                                        if (self.ex_buf_ptr == 0i32) {
                                            if ((self.lex_class[(self.str_pool[(self.tmp_ptr) as usize]) as usize] == white_space) && (self.tmp_ptr < self.tmp_end_ptr)) {
                                                {
                                                    {
                                                        if (self.ex_buf_ptr >= self.buf_size) {
                                                            {
                                                                {
                                                                    {
                                                                        let __w1 = space;
                                                                        crate::system::wr_str(&mut self.log_file, "Field filled up at ");
                                                                        crate::system::wr_int(&mut self.log_file, __w1, 1);
                                                                        crate::system::wr_str(&mut self.log_file, ", reallocating.");
                                                                    }
                                                                }
                                                                {
                                                                    {
                                                                        crate::system::wr_ln(&mut self.log_file);
                                                                    }
                                                                }
                                                                self.buffer_overflow();
                                                            }
                                                        }
                                                        self.ex_buf[(self.ex_buf_ptr) as usize] = space;
                                                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                    }
                                                    self.tmp_ptr = (self.tmp_ptr).wrapping_add(1i32);
                                                    while ((self.lex_class[(self.str_pool[(self.tmp_ptr) as usize]) as usize] == white_space) && (self.tmp_ptr < self.tmp_end_ptr)) {
                                                        self.tmp_ptr = (self.tmp_ptr).wrapping_add(1i32);
                                                    }
                                                }
                                            }
                                        }
                                        while (self.tmp_ptr < self.tmp_end_ptr) {
                                            {
                                                if (self.lex_class[(self.str_pool[(self.tmp_ptr) as usize]) as usize] != white_space) {
                                                    {
                                                        if (self.ex_buf_ptr >= self.buf_size) {
                                                            {
                                                                {
                                                                    {
                                                                        let __w1 = self.str_pool[(self.tmp_ptr) as usize];
                                                                        crate::system::wr_str(&mut self.log_file, "Field filled up at ");
                                                                        crate::system::wr_int(&mut self.log_file, __w1, 1);
                                                                        crate::system::wr_str(&mut self.log_file, ", reallocating.");
                                                                    }
                                                                }
                                                                {
                                                                    {
                                                                        crate::system::wr_ln(&mut self.log_file);
                                                                    }
                                                                }
                                                                self.buffer_overflow();
                                                            }
                                                        }
                                                        { let __ix51 = self.ex_buf_ptr; let __v52 = self.str_pool[(self.tmp_ptr) as usize]; self.ex_buf[(__ix51) as usize] = __v52; }
                                                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                    }
                                                } else {
                                                    if (self.ex_buf[((self.ex_buf_ptr).wrapping_sub(1i32)) as usize] != space) {
                                                        {
                                                            if (self.ex_buf_ptr >= self.buf_size) {
                                                                {
                                                                    {
                                                                        {
                                                                            let __w1 = space;
                                                                            crate::system::wr_str(&mut self.log_file, "Field filled up at ");
                                                                            crate::system::wr_int(&mut self.log_file, __w1, 1);
                                                                            crate::system::wr_str(&mut self.log_file, ", reallocating.");
                                                                        }
                                                                    }
                                                                    {
                                                                        {
                                                                            crate::system::wr_ln(&mut self.log_file);
                                                                        }
                                                                    }
                                                                    self.buffer_overflow();
                                                                }
                                                            }
                                                            self.ex_buf[(self.ex_buf_ptr) as usize] = space;
                                                            self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                        }
                                                    }
                                                }
                                                self.tmp_ptr = (self.tmp_ptr).wrapping_add(1i32);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // §241
            {
                if (!self.eat_bib_white_space()) {
                    {
                        self.eat_bib_print();
                        break 'l_exit_f;
                    }
                }
            }
            scan_a_field_token_and_eat_white = true;
        }
        scan_a_field_token_and_eat_white
    }

    /// This function scans the list of field tokens that define the field
    /// value string.  If `store_field` is `true` it accumulates (indirectly)
    /// in `field_vl_str` the concatenation of all the field tokens,
    /// compressing nonnull `white_space` to a single `space` and, if the
    /// field value is for a field (rather than a string definition), removing
    /// any leading or trailing `white_space`; when it's finished it puts the
    /// string into the hash table.  It returns `false` if there was a serious
    /// syntax error.
    /// @<Procedures and functions for input scanning
    // §240
    pub fn scan_and_store_the_field_value_and_eat_white(&mut self) -> bool {
        let mut scan_and_store_the_field_value_and_eat_white: bool = false;
        'l_exit_f: {
            scan_and_store_the_field_value_and_eat_white = false;
            self.ex_buf_ptr = 0i32;
            if (!self.scan_a_field_token_and_eat_white()) {
                break 'l_exit_f;
            }
            while (self.buffer[(self.buf_ptr2) as usize] == concat_char) {
                {
                    self.buf_ptr2 = (self.buf_ptr2).wrapping_add(1i32);
                    {
                        if (!self.eat_bib_white_space()) {
                            {
                                self.eat_bib_print();
                                break 'l_exit_f;
                            }
                        }
                    }
                    if (!self.scan_a_field_token_and_eat_white()) {
                        break 'l_exit_f;
                    }
                }
            }
            if self.store_field {
                // §252
                {
                    if (!self.at_bib_command) {
                        if (self.ex_buf_ptr > 0i32) {
                            if (self.ex_buf[((self.ex_buf_ptr).wrapping_sub(1i32)) as usize] == space) {
                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_sub(1i32);
                            }
                        }
                    }
                    if (((!self.at_bib_command) && (self.ex_buf[(0i32) as usize] == space)) && (self.ex_buf_ptr > 0i32)) {
                        self.ex_buf_xptr = 1i32;
                    } else {
                        self.ex_buf_xptr = 0i32;
                    }
                    self.field_val_loc = { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.str_lookup(&mut __f0, self.ex_buf_xptr, (self.ex_buf_ptr).wrapping_sub(self.ex_buf_xptr), text_ilk, true); self.ex_buf = __f0; __r };
                    self.fn_type[(self.field_val_loc) as usize] = str_literal;
                    if self.at_bib_command {
                        // §253
                        {
                            match self.command_num {
                                n_bib_preamble => {
                                    {
                                        { let __ix53 = self.preamble_ptr; let __v54 = self.hash_text[(self.field_val_loc) as usize]; self.s_preamble[(__ix53) as usize] = __v54; }
                                        self.preamble_ptr = (self.preamble_ptr).wrapping_add(1i32);
                                    }
                                }
                                n_bib_string => {
                                    { let __ix55 = self.cur_macro_loc; let __v56 = self.hash_text[(self.field_val_loc) as usize]; self.ilk_info[(__ix55) as usize] = __v56; }
                                }
                                _ => {
                                    self.bib_cmd_confusion();
                                }
                            }
                        }
                    } else {
                        // §254
                        {
                            self.field_ptr = ((self.entry_cite_ptr).wrapping_mul(self.num_fields)).wrapping_add(self.ilk_info[(self.field_name_loc) as usize]);
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
                            if (self.field_info[(self.field_ptr) as usize] != missing) {
                                {
                                    {
                                        {
                                            crate::system::wr_str(&mut self.log_file, "Warning--I'm ignoring ");
                                        }
                                        {
                                            crate::system::wr_str(&mut self.standard_output, "Warning--I'm ignoring ");
                                        }
                                    }
                                    self.print_a_pool_str(self.cite_list[(self.entry_cite_ptr) as usize]);
                                    {
                                        {
                                            crate::system::wr_str(&mut self.log_file, "'s extra \"");
                                        }
                                        {
                                            crate::system::wr_str(&mut self.standard_output, "'s extra \"");
                                        }
                                    }
                                    self.print_a_pool_str(self.hash_text[(self.field_name_loc) as usize]);
                                    {
                                        {
                                            {
                                                crate::system::wr_str(&mut self.log_file, "\" field");
                                                crate::system::wr_ln(&mut self.log_file);
                                            }
                                            {
                                                crate::system::wr_str(&mut self.standard_output, "\" field");
                                                crate::system::wr_ln(&mut self.standard_output);
                                            }
                                        }
                                        self.bib_warn_print();
                                    }
                                }
                            } else {
                                {
                                    { let __ix57 = self.field_ptr; let __v58 = self.hash_text[(self.field_val_loc) as usize]; self.field_info[(__ix57) as usize] = __v58; }
                                    if ((self.ilk_info[(self.field_name_loc) as usize] == self.crossref_num) && (!self.all_entries)) {
                                        // §255
                                        {
                                            self.tmp_ptr = self.ex_buf_xptr;
                                            while (self.tmp_ptr < self.ex_buf_ptr) {
                                                {
                                                    { let __ix59 = self.tmp_ptr; let __v60 = self.ex_buf[(self.tmp_ptr) as usize]; self.out_buf[(__ix59) as usize] = __v60; }
                                                    self.tmp_ptr = (self.tmp_ptr).wrapping_add(1i32);
                                                }
                                            }
                                            { let mut __f0 = ::core::mem::take(&mut self.out_buf); let __r = self.lower_case(&mut __f0, self.ex_buf_xptr, (self.ex_buf_ptr).wrapping_sub(self.ex_buf_xptr)); self.out_buf = __f0; __r };
                                            self.lc_cite_loc = { let mut __f0 = ::core::mem::take(&mut self.out_buf); let __r = self.str_lookup(&mut __f0, self.ex_buf_xptr, (self.ex_buf_ptr).wrapping_sub(self.ex_buf_xptr), lc_cite_ilk, true); self.out_buf = __f0; __r };
                                            if self.hash_found {
                                                {
                                                    self.cite_loc = self.ilk_info[(self.lc_cite_loc) as usize];
                                                    if (self.ilk_info[(self.cite_loc) as usize] >= self.old_num_cites) {
                                                        { let __ix61 = self.ilk_info[(self.cite_loc) as usize]; let __v62 = (self.cite_info[(self.ilk_info[(self.cite_loc) as usize]) as usize]).wrapping_add(1i32); self.cite_info[(__ix61) as usize] = __v62; }
                                                    }
                                                }
                                            } else {
                                                {
                                                    self.cite_loc = { let mut __f0 = ::core::mem::take(&mut self.ex_buf); let __r = self.str_lookup(&mut __f0, self.ex_buf_xptr, (self.ex_buf_ptr).wrapping_sub(self.ex_buf_xptr), cite_ilk, true); self.ex_buf = __f0; __r };
                                                    if self.hash_found {
                                                        self.hash_cite_confusion();
                                                    }
                                                    { let mut __f0 = ::core::mem::take(&mut self.cite_ptr); let __r = self.add_database_cite(&mut __f0); self.cite_ptr = __f0; __r };
                                                    self.cite_info[(self.ilk_info[(self.cite_loc) as usize]) as usize] = 1i32;
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
            // §240
            scan_and_store_the_field_value_and_eat_white = true;
        }
        scan_and_store_the_field_value_and_eat_white
    }

    /// This procedure complains if the just-encountered `right_brace` would
    /// make `brace_level` negative.
    /// @<Procedures and functions for name-string processing
    // §358
    pub fn decr_brace_level(&mut self, mut pop_lit_var: str_number) {
        if (self.brace_level == 0i32) {
            self.braces_unbalanced_complaint(pop_lit_var);
        } else {
            self.brace_level = (self.brace_level).wrapping_sub(1i32);
        }
    }

    /// This one makes sure that `brace_level=0` (it's called at a point in a
    /// string where braces must be balanced).
    /// @<Procedures and functions for name-string processing
    // §360
    pub fn check_brace_level(&mut self, mut pop_lit_var: str_number) {
        if (self.brace_level > 0i32) {
            self.braces_unbalanced_complaint(pop_lit_var);
        }
    }

    /// This module, starting at `ex_buf_ptr`, looks in `ex_buf` for an
    /// ``and'' surrounded by nonnull `white_space`.  It stops either at
    /// `ex_buf_length` or just past the ``and'', whichever comes first,
    /// setting `ex_buf_ptr` accordingly.  Its parameter `pop_lit_var` is
    /// either `pop_lit3` or `pop_lit1`, depending on whether
    /// {\.{format.name\$}} or {\.{num.names\$}} calls it.
    /// @<Procedures and functions for name-string processing
    // §375
    pub fn name_scan_for_and(&mut self, mut pop_lit_var: str_number) {
        self.brace_level = 0i32;
        self.preceding_white = false;
        self.and_found = false;
        while ((!self.and_found) && (self.ex_buf_ptr < self.ex_buf_length)) {
            match self.ex_buf[(self.ex_buf_ptr) as usize] {
                97 | 65 => {
                    {
                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                        if self.preceding_white {
                            // §377
                            {
                                if (self.ex_buf_ptr <= (self.ex_buf_length).wrapping_sub(3i32)) {
                                    if ((self.ex_buf[(self.ex_buf_ptr) as usize] == 110i32) || (self.ex_buf[(self.ex_buf_ptr) as usize] == 78i32)) {
                                        if ((self.ex_buf[((self.ex_buf_ptr).wrapping_add(1i32)) as usize] == 100i32) || (self.ex_buf[((self.ex_buf_ptr).wrapping_add(1i32)) as usize] == 68i32)) {
                                            if (self.lex_class[(self.ex_buf[((self.ex_buf_ptr).wrapping_add(2i32)) as usize]) as usize] == white_space) {
                                                {
                                                    self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(2i32);
                                                    self.and_found = true;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // §375
                        self.preceding_white = false;
                    }
                }
                left_brace => {
                    {
                        self.brace_level = (self.brace_level).wrapping_add(1i32);
                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                        // §376
                        while ((self.brace_level > 0i32) && (self.ex_buf_ptr < self.ex_buf_length)) {
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
                        // §375
                        self.preceding_white = false;
                    }
                }
                right_brace => {
                    {
                        self.decr_brace_level(pop_lit_var);
                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                        self.preceding_white = false;
                    }
                }
                _ => {
                    if (self.lex_class[(self.ex_buf[(self.ex_buf_ptr) as usize]) as usize] == white_space) {
                        {
                            self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                            self.preceding_white = true;
                        }
                    } else {
                        {
                            self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                            self.preceding_white = false;
                        }
                    }
                }
            }
        }
        self.check_brace_level(pop_lit_var);
    }

    /// It's a von token if there exists a first brace-level-0 letter (or
    /// brace-level-1 special character), and it's in lower case; in this case
    /// we return `true`.  The token is in `name_buf`, starting at
    /// `name_bf_ptr` and ending just before `name_bf_xptr`.
    // §388
    pub fn von_token_found(&mut self) -> bool {
        let mut von_token_found: bool = false;
        'l_exit_f: {
            self.nm_brace_level = 0i32;
            von_token_found = false;
            while (self.name_bf_ptr < self.name_bf_xptr) {
                if ((self.sv_buffer[(self.name_bf_ptr) as usize] >= 65i32) && (self.sv_buffer[(self.name_bf_ptr) as usize] <= 90i32)) {
                    break 'l_exit_f;
                } else {
                    if ((self.sv_buffer[(self.name_bf_ptr) as usize] >= 97i32) && (self.sv_buffer[(self.name_bf_ptr) as usize] <= 122i32)) {
                        {
                            von_token_found = true;
                            break 'l_exit_f;
                        }
                    } else {
                        if (self.sv_buffer[(self.name_bf_ptr) as usize] == left_brace) {
                            {
                                self.nm_brace_level = (self.nm_brace_level).wrapping_add(1i32);
                                self.name_bf_ptr = (self.name_bf_ptr).wrapping_add(1i32);
                                if (((self.name_bf_ptr).wrapping_add(2i32) < self.name_bf_xptr) && (self.sv_buffer[(self.name_bf_ptr) as usize] == backslash)) {
                                    // §389
                                    {
                                        self.name_bf_ptr = (self.name_bf_ptr).wrapping_add(1i32);
                                        self.name_bf_yptr = self.name_bf_ptr;
                                        while ((self.name_bf_ptr < self.name_bf_xptr) && (self.lex_class[(self.sv_buffer[(self.name_bf_ptr) as usize]) as usize] == alpha)) {
                                            self.name_bf_ptr = (self.name_bf_ptr).wrapping_add(1i32);
                                        }
                                        self.control_seq_loc = { let mut __f0 = ::core::mem::take(&mut self.sv_buffer); let __r = self.str_lookup(&mut __f0, self.name_bf_yptr, (self.name_bf_ptr).wrapping_sub(self.name_bf_yptr), control_seq_ilk, false); self.sv_buffer = __f0; __r };
                                        if self.hash_found {
                                            // §390
                                            {
                                                match self.ilk_info[(self.control_seq_loc) as usize] {
                                                    n_oe_upper | n_ae_upper | n_aa_upper | n_o_upper | n_l_upper => {
                                                        break 'l_exit_f;
                                                    }
                                                    n_i | n_j | n_oe | n_ae | n_aa | n_o | n_l | n_ss => {
                                                        {
                                                            von_token_found = true;
                                                            break 'l_exit_f;
                                                        }
                                                    }
                                                    _ => {
                                                        {
                                                            {
                                                                {
                                                                    crate::system::wr_str(&mut self.log_file, "Control-sequence hash error");
                                                                }
                                                                {
                                                                    crate::system::wr_str(&mut self.standard_output, "Control-sequence hash error");
                                                                }
                                                            }
                                                            self.print_confusion();
                                                            crate::system::end_of_TEX(self);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        // §389
                                        while ((self.name_bf_ptr < self.name_bf_xptr) && (self.nm_brace_level > 0i32)) {
                                            {
                                                if ((self.sv_buffer[(self.name_bf_ptr) as usize] >= 65i32) && (self.sv_buffer[(self.name_bf_ptr) as usize] <= 90i32)) {
                                                    break 'l_exit_f;
                                                } else {
                                                    if ((self.sv_buffer[(self.name_bf_ptr) as usize] >= 97i32) && (self.sv_buffer[(self.name_bf_ptr) as usize] <= 122i32)) {
                                                        {
                                                            von_token_found = true;
                                                            break 'l_exit_f;
                                                        }
                                                    } else {
                                                        if (self.sv_buffer[(self.name_bf_ptr) as usize] == right_brace) {
                                                            self.nm_brace_level = (self.nm_brace_level).wrapping_sub(1i32);
                                                        } else {
                                                            if (self.sv_buffer[(self.name_bf_ptr) as usize] == left_brace) {
                                                                self.nm_brace_level = (self.nm_brace_level).wrapping_add(1i32);
                                                            }
                                                        }
                                                    }
                                                }
                                                self.name_bf_ptr = (self.name_bf_ptr).wrapping_add(1i32);
                                            }
                                        }
                                        break 'l_exit_f;
                                    }
                                } else {
                                    // §391
                                    while ((self.nm_brace_level > 0i32) && (self.name_bf_ptr < self.name_bf_xptr)) {
                                        {
                                            if (self.sv_buffer[(self.name_bf_ptr) as usize] == right_brace) {
                                                self.nm_brace_level = (self.nm_brace_level).wrapping_sub(1i32);
                                            } else {
                                                if (self.sv_buffer[(self.name_bf_ptr) as usize] == left_brace) {
                                                    self.nm_brace_level = (self.nm_brace_level).wrapping_add(1i32);
                                                }
                                            }
                                            self.name_bf_ptr = (self.name_bf_ptr).wrapping_add(1i32);
                                        }
                                    }
                                }
                            }
                        } else {
                            // §388
                            self.name_bf_ptr = (self.name_bf_ptr).wrapping_add(1i32);
                        }
                    }
                }
            }
        }
        von_token_found
    }

    /// The last name starts just past the last token, before the first
    /// `comma` (if there is no `comma`, there is deemed to be one at the end
    /// of the string), for which there exists a first brace-level-0 letter
    /// (or brace-level-1 special character), and it's in lower case, unless
    /// this last token is also the last token before the `comma`, in which
    /// case the last name starts with this token (unless this last token is
    /// connected by a `sep_char` other than a `tie` to the previous token, in
    /// which case the last name starts with as many tokens earlier as are
    /// connected by non`tie`s to this last one (except on Tuesdays
    /// $\ldots\,$), although this module never sees such a case).  Note that
    /// if there are any tokens in either the von or last names, then the last
    /// name has at least one, even if it starts with a lower-case letter.
    /// @<Procedures and functions for name-string processing
    // §392
    pub fn von_name_ends_and_last_name_starts_stuff(&mut self) {
        'l_exit_f: {
            self.von_end = (self.last_end).wrapping_sub(1i32);
            while (self.von_end > self.von_start) {
                {
                    self.name_bf_ptr = self.name_tok[((self.von_end).wrapping_sub(1i32)) as usize];
                    self.name_bf_xptr = self.name_tok[(self.von_end) as usize];
                    if self.von_token_found() {
                        break 'l_exit_f;
                    }
                    self.von_end = (self.von_end).wrapping_sub(1i32);
                }
            }
        }
    }

    /// When we come here `sp_ptr` is just past the `left_brace`, and when we
    /// leave it's either at `sp_end` or just past the matching `right_brace`.
    /// @<Procedures and functions for name-string processing
    // §395
    pub fn skip_stuff_at_sp_brace_level_greater_than_one(&mut self) {
        while ((self.sp_brace_level > 1i32) && (self.sp_ptr < self.sp_end)) {
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
    }

    /// At most one of the important letters, perhaps doubled, may appear at
    /// `sp_brace_level = 1`.
    /// @<Procedures and functions for name-string processing
    // §397
    pub fn brace_lvl_one_letters_complaint(&mut self) {
        {
            {
                crate::system::wr_str(&mut self.log_file, "The format string \"");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "The format string \"");
            }
        }
        self.print_a_pool_str(self.pop_lit1);
        {
            {
                {
                    crate::system::wr_str(&mut self.log_file, "\" has an illegal brace-level-1 letter");
                }
                {
                    crate::system::wr_str(&mut self.standard_output, "\" has an illegal brace-level-1 letter");
                }
            }
            self.bst_ex_warn_print();
        }
    }

    /// This function looks at the string in `ex_buf`, starting at
    /// `ex_buf_xptr` and ending just before `ex_buf_ptr`, and it returns
    /// `true` if there are `enough_chars`, where a special character (even if
    /// it's missing its matching `right_brace`) counts as a single character.
    /// This procedure is called only for strings that don't have too many
    /// `right_brace`s.
    /// @<Procedures and functions for name-string processing
    // §409
    pub fn enough_text_chars(&mut self, mut enough_chars: buf_pointer) -> bool {
        let mut enough_text_chars: bool = false;
        self.num_text_chars = 0i32;
        self.ex_buf_yptr = self.ex_buf_xptr;
        while ((self.ex_buf_yptr < self.ex_buf_ptr) && (self.num_text_chars < enough_chars)) {
            {
                self.ex_buf_yptr = (self.ex_buf_yptr).wrapping_add(1i32);
                if (self.ex_buf[((self.ex_buf_yptr).wrapping_sub(1i32)) as usize] == left_brace) {
                    {
                        self.brace_level = (self.brace_level).wrapping_add(1i32);
                        if ((self.brace_level == 1i32) && (self.ex_buf_yptr < self.ex_buf_ptr)) {
                            if (self.ex_buf[(self.ex_buf_yptr) as usize] == backslash) {
                                {
                                    self.ex_buf_yptr = (self.ex_buf_yptr).wrapping_add(1i32);
                                    while ((self.ex_buf_yptr < self.ex_buf_ptr) && (self.brace_level > 0i32)) {
                                        {
                                            if (self.ex_buf[(self.ex_buf_yptr) as usize] == right_brace) {
                                                self.brace_level = (self.brace_level).wrapping_sub(1i32);
                                            } else {
                                                if (self.ex_buf[(self.ex_buf_yptr) as usize] == left_brace) {
                                                    self.brace_level = (self.brace_level).wrapping_add(1i32);
                                                }
                                            }
                                            self.ex_buf_yptr = (self.ex_buf_yptr).wrapping_add(1i32);
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    if (self.ex_buf[((self.ex_buf_yptr).wrapping_sub(1i32)) as usize] == right_brace) {
                        self.brace_level = (self.brace_level).wrapping_sub(1i32);
                    }
                }
                self.num_text_chars = (self.num_text_chars).wrapping_add(1i32);
            }
        }
        if (self.num_text_chars < enough_chars) {
            enough_text_chars = false;
        } else {
            enough_text_chars = true;
        }
        enough_text_chars
    }

    /// This is a procedure so that `x_format_name` is smaller.
    /// @<Procedures and functions for name-string processing
    // §411
    pub fn figure_out_the_formatted_name(&mut self) {
        // §393
        {
            self.ex_buf_ptr = 0i32;
            self.sp_brace_level = 0i32;
            self.sp_ptr = self.str_start[(self.pop_lit1) as usize];
            self.sp_end = self.str_start[((self.pop_lit1).wrapping_add(1i32)) as usize];
            while (self.sp_ptr < self.sp_end) {
                if (self.str_pool[(self.sp_ptr) as usize] == left_brace) {
                    {
                        self.sp_brace_level = (self.sp_brace_level).wrapping_add(1i32);
                        self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                        // §394
                        {
                            self.sp_xptr1 = self.sp_ptr;
                            self.alpha_found = false;
                            self.double_letter = false;
                            self.end_of_group = false;
                            self.to_be_written = true;
                            while ((!self.end_of_group) && (self.sp_ptr < self.sp_end)) {
                                if (self.lex_class[(self.str_pool[(self.sp_ptr) as usize]) as usize] == alpha) {
                                    {
                                        self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                        // §396
                                        {
                                            if self.alpha_found {
                                                {
                                                    self.brace_lvl_one_letters_complaint();
                                                    self.to_be_written = false;
                                                }
                                            } else {
                                                {
                                                    match self.str_pool[((self.sp_ptr).wrapping_sub(1i32)) as usize] {
                                                        102 | 70 => {
                                                            // §398
                                                            {
                                                                self.cur_token = self.first_start;
                                                                self.last_token = self.first_end;
                                                                if (self.cur_token == self.last_token) {
                                                                    self.to_be_written = false;
                                                                }
                                                                if ((self.str_pool[(self.sp_ptr) as usize] == 102i32) || (self.str_pool[(self.sp_ptr) as usize] == 70i32)) {
                                                                    self.double_letter = true;
                                                                }
                                                            }
                                                        }
                                                        118 | 86 => {
                                                            // §399
                                                            {
                                                                self.cur_token = self.von_start;
                                                                self.last_token = self.von_end;
                                                                if (self.cur_token == self.last_token) {
                                                                    self.to_be_written = false;
                                                                }
                                                                if ((self.str_pool[(self.sp_ptr) as usize] == 118i32) || (self.str_pool[(self.sp_ptr) as usize] == 86i32)) {
                                                                    self.double_letter = true;
                                                                }
                                                            }
                                                        }
                                                        108 | 76 => {
                                                            // §400
                                                            {
                                                                self.cur_token = self.von_end;
                                                                self.last_token = self.last_end;
                                                                if (self.cur_token == self.last_token) {
                                                                    self.to_be_written = false;
                                                                }
                                                                if ((self.str_pool[(self.sp_ptr) as usize] == 108i32) || (self.str_pool[(self.sp_ptr) as usize] == 76i32)) {
                                                                    self.double_letter = true;
                                                                }
                                                            }
                                                        }
                                                        106 | 74 => {
                                                            // §401
                                                            {
                                                                self.cur_token = self.last_end;
                                                                self.last_token = self.jr_end;
                                                                if (self.cur_token == self.last_token) {
                                                                    self.to_be_written = false;
                                                                }
                                                                if ((self.str_pool[(self.sp_ptr) as usize] == 106i32) || (self.str_pool[(self.sp_ptr) as usize] == 74i32)) {
                                                                    self.double_letter = true;
                                                                }
                                                            }
                                                        }
                                                        _ => {
                                                            // §396
                                                            {
                                                                self.brace_lvl_one_letters_complaint();
                                                                self.to_be_written = false;
                                                            }
                                                        }
                                                    }
                                                    if self.double_letter {
                                                        self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                                    }
                                                }
                                            }
                                            self.alpha_found = true;
                                        }
                                    }
                                } else {
                                    // §394
                                    if (self.str_pool[(self.sp_ptr) as usize] == right_brace) {
                                        {
                                            self.sp_brace_level = (self.sp_brace_level).wrapping_sub(1i32);
                                            self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                            self.end_of_group = true;
                                        }
                                    } else {
                                        if (self.str_pool[(self.sp_ptr) as usize] == left_brace) {
                                            {
                                                self.sp_brace_level = (self.sp_brace_level).wrapping_add(1i32);
                                                self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                                self.skip_stuff_at_sp_brace_level_greater_than_one();
                                            }
                                        } else {
                                            self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                        }
                                    }
                                }
                            }
                            if (self.end_of_group && self.to_be_written) {
                                // §402
                                {
                                    self.ex_buf_xptr = self.ex_buf_ptr;
                                    self.sp_ptr = self.sp_xptr1;
                                    self.sp_brace_level = 1i32;
                                    while (self.sp_brace_level > 0i32) {
                                        if ((self.lex_class[(self.str_pool[(self.sp_ptr) as usize]) as usize] == alpha) && (self.sp_brace_level == 1i32)) {
                                            {
                                                self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                                // §403
                                                {
                                                    if self.double_letter {
                                                        self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                                    }
                                                    self.use_default = true;
                                                    self.sp_xptr2 = self.sp_ptr;
                                                    if (self.str_pool[(self.sp_ptr) as usize] == left_brace) {
                                                        {
                                                            self.use_default = false;
                                                            self.sp_brace_level = (self.sp_brace_level).wrapping_add(1i32);
                                                            self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                                            self.sp_xptr1 = self.sp_ptr;
                                                            self.skip_stuff_at_sp_brace_level_greater_than_one();
                                                            self.sp_xptr2 = (self.sp_ptr).wrapping_sub(1i32);
                                                        }
                                                    }
                                                    // §404
                                                    while (self.cur_token < self.last_token) {
                                                        {
                                                            if self.double_letter {
                                                                // §405
                                                                {
                                                                    self.name_bf_ptr = self.name_tok[(self.cur_token) as usize];
                                                                    self.name_bf_xptr = self.name_tok[((self.cur_token).wrapping_add(1i32)) as usize];
                                                                    if ((self.ex_buf_length).wrapping_add((self.name_bf_xptr).wrapping_sub(self.name_bf_ptr)) > self.buf_size) {
                                                                        self.buffer_overflow();
                                                                    }
                                                                    while (self.name_bf_ptr < self.name_bf_xptr) {
                                                                        {
                                                                            {
                                                                                { let __ix63 = self.ex_buf_ptr; let __v64 = self.sv_buffer[(self.name_bf_ptr) as usize]; self.ex_buf[(__ix63) as usize] = __v64; }
                                                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                                            }
                                                                            self.name_bf_ptr = (self.name_bf_ptr).wrapping_add(1i32);
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                // §406
                                                                {
                                                                    'l_L15_f: {
                                                                        self.name_bf_ptr = self.name_tok[(self.cur_token) as usize];
                                                                        self.name_bf_xptr = self.name_tok[((self.cur_token).wrapping_add(1i32)) as usize];
                                                                        while (self.name_bf_ptr < self.name_bf_xptr) {
                                                                            {
                                                                                if (self.lex_class[(self.sv_buffer[(self.name_bf_ptr) as usize]) as usize] == alpha) {
                                                                                    {
                                                                                        {
                                                                                            if (self.ex_buf_ptr == self.buf_size) {
                                                                                                self.buffer_overflow();
                                                                                            }
                                                                                            {
                                                                                                { let __ix65 = self.ex_buf_ptr; let __v66 = self.sv_buffer[(self.name_bf_ptr) as usize]; self.ex_buf[(__ix65) as usize] = __v66; }
                                                                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                                                            }
                                                                                        }
                                                                                        break 'l_L15_f;
                                                                                    }
                                                                                } else {
                                                                                    if ((self.sv_buffer[(self.name_bf_ptr) as usize] == left_brace) && ((self.name_bf_ptr).wrapping_add(1i32) < self.name_bf_xptr)) {
                                                                                        if (self.sv_buffer[((self.name_bf_ptr).wrapping_add(1i32)) as usize] == backslash) {
                                                                                            // §407
                                                                                            {
                                                                                                if ((self.ex_buf_ptr).wrapping_add(2i32) > self.buf_size) {
                                                                                                    self.buffer_overflow();
                                                                                                }
                                                                                                {
                                                                                                    self.ex_buf[(self.ex_buf_ptr) as usize] = left_brace;
                                                                                                    self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                                                                }
                                                                                                {
                                                                                                    self.ex_buf[(self.ex_buf_ptr) as usize] = backslash;
                                                                                                    self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                                                                }
                                                                                                self.name_bf_ptr = (self.name_bf_ptr).wrapping_add(2i32);
                                                                                                self.nm_brace_level = 1i32;
                                                                                                while ((self.name_bf_ptr < self.name_bf_xptr) && (self.nm_brace_level > 0i32)) {
                                                                                                    {
                                                                                                        if (self.sv_buffer[(self.name_bf_ptr) as usize] == right_brace) {
                                                                                                            self.nm_brace_level = (self.nm_brace_level).wrapping_sub(1i32);
                                                                                                        } else {
                                                                                                            if (self.sv_buffer[(self.name_bf_ptr) as usize] == left_brace) {
                                                                                                                self.nm_brace_level = (self.nm_brace_level).wrapping_add(1i32);
                                                                                                            }
                                                                                                        }
                                                                                                        {
                                                                                                            if (self.ex_buf_ptr == self.buf_size) {
                                                                                                                self.buffer_overflow();
                                                                                                            }
                                                                                                            {
                                                                                                                { let __ix67 = self.ex_buf_ptr; let __v68 = self.sv_buffer[(self.name_bf_ptr) as usize]; self.ex_buf[(__ix67) as usize] = __v68; }
                                                                                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                                                                            }
                                                                                                        }
                                                                                                        self.name_bf_ptr = (self.name_bf_ptr).wrapping_add(1i32);
                                                                                                    }
                                                                                                }
                                                                                                break 'l_L15_f;
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                                // §406
                                                                                self.name_bf_ptr = (self.name_bf_ptr).wrapping_add(1i32);
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            // §404
                                                            self.cur_token = (self.cur_token).wrapping_add(1i32);
                                                            if (self.cur_token < self.last_token) {
                                                                // §408
                                                                {
                                                                    if self.use_default {
                                                                        {
                                                                            if (!self.double_letter) {
                                                                                {
                                                                                    if (self.ex_buf_ptr == self.buf_size) {
                                                                                        self.buffer_overflow();
                                                                                    }
                                                                                    {
                                                                                        self.ex_buf[(self.ex_buf_ptr) as usize] = period;
                                                                                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                                                    }
                                                                                }
                                                                            }
                                                                            if (self.lex_class[(self.name_sep_char[(self.cur_token) as usize]) as usize] == sep_char) {
                                                                                {
                                                                                    if (self.ex_buf_ptr == self.buf_size) {
                                                                                        self.buffer_overflow();
                                                                                    }
                                                                                    {
                                                                                        { let __ix69 = self.ex_buf_ptr; let __v70 = self.name_sep_char[(self.cur_token) as usize]; self.ex_buf[(__ix69) as usize] = __v70; }
                                                                                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                                                    }
                                                                                }
                                                                            } else {
                                                                                if ((self.cur_token == (self.last_token).wrapping_sub(1i32)) || (!self.enough_text_chars(long_token))) {
                                                                                    {
                                                                                        if (self.ex_buf_ptr == self.buf_size) {
                                                                                            self.buffer_overflow();
                                                                                        }
                                                                                        {
                                                                                            self.ex_buf[(self.ex_buf_ptr) as usize] = tie;
                                                                                            self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                                                        }
                                                                                    }
                                                                                } else {
                                                                                    {
                                                                                        if (self.ex_buf_ptr == self.buf_size) {
                                                                                            self.buffer_overflow();
                                                                                        }
                                                                                        {
                                                                                            self.ex_buf[(self.ex_buf_ptr) as usize] = space;
                                                                                            self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    } else {
                                                                        {
                                                                            if ((self.ex_buf_length).wrapping_add((self.sp_xptr2).wrapping_sub(self.sp_xptr1)) > self.buf_size) {
                                                                                self.buffer_overflow();
                                                                            }
                                                                            self.sp_ptr = self.sp_xptr1;
                                                                            while (self.sp_ptr < self.sp_xptr2) {
                                                                                {
                                                                                    {
                                                                                        { let __ix71 = self.ex_buf_ptr; let __v72 = self.str_pool[(self.sp_ptr) as usize]; self.ex_buf[(__ix71) as usize] = __v72; }
                                                                                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                                                    }
                                                                                    self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    // §403
                                                    if (!self.use_default) {
                                                        self.sp_ptr = (self.sp_xptr2).wrapping_add(1i32);
                                                    }
                                                }
                                            }
                                        } else {
                                            // §402
                                            if (self.str_pool[(self.sp_ptr) as usize] == right_brace) {
                                                {
                                                    self.sp_brace_level = (self.sp_brace_level).wrapping_sub(1i32);
                                                    self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                                    if (self.sp_brace_level > 0i32) {
                                                        {
                                                            if (self.ex_buf_ptr == self.buf_size) {
                                                                self.buffer_overflow();
                                                            }
                                                            {
                                                                self.ex_buf[(self.ex_buf_ptr) as usize] = right_brace;
                                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                if (self.str_pool[(self.sp_ptr) as usize] == left_brace) {
                                                    {
                                                        self.sp_brace_level = (self.sp_brace_level).wrapping_add(1i32);
                                                        self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                                        {
                                                            if (self.ex_buf_ptr == self.buf_size) {
                                                                self.buffer_overflow();
                                                            }
                                                            {
                                                                self.ex_buf[(self.ex_buf_ptr) as usize] = left_brace;
                                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    {
                                                        {
                                                            if (self.ex_buf_ptr == self.buf_size) {
                                                                self.buffer_overflow();
                                                            }
                                                            {
                                                                { let __ix73 = self.ex_buf_ptr; let __v74 = self.str_pool[(self.sp_ptr) as usize]; self.ex_buf[(__ix73) as usize] = __v74; }
                                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                            }
                                                        }
                                                        self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if (self.ex_buf_ptr > 0i32) {
                                        if (self.ex_buf[((self.ex_buf_ptr).wrapping_sub(1i32)) as usize] == tie) {
                                            // §410
                                            {
                                                self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_sub(1i32);
                                                if (self.ex_buf[((self.ex_buf_ptr).wrapping_sub(1i32)) as usize] == tie) {
                                                } else {
                                                    if (!self.enough_text_chars(long_name)) {
                                                        self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                                    } else {
                                                        {
                                                            self.ex_buf[(self.ex_buf_ptr) as usize] = space;
                                                            self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
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
                } else {
                    // §393
                    if (self.str_pool[(self.sp_ptr) as usize] == right_brace) {
                        {
                            self.braces_unbalanced_complaint(self.pop_lit1);
                            self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                        }
                    } else {
                        {
                            {
                                if (self.ex_buf_ptr == self.buf_size) {
                                    self.buffer_overflow();
                                }
                                {
                                    { let __ix75 = self.ex_buf_ptr; let __v76 = self.str_pool[(self.sp_ptr) as usize]; self.ex_buf[(__ix75) as usize] = __v76; }
                                    self.ex_buf_ptr = (self.ex_buf_ptr).wrapping_add(1i32);
                                }
                            }
                            self.sp_ptr = (self.sp_ptr).wrapping_add(1i32);
                        }
                    }
                }
            }
            if (self.sp_brace_level > 0i32) {
                self.braces_unbalanced_complaint(self.pop_lit1);
            }
            self.ex_buf_length = self.ex_buf_ptr;
        }
    }

}
