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
    /// This program uses the term `print` instead of `write` when writing on
    /// both the `log_file` and (system-dependent) `term_out` file, and it
    /// uses `trace_pr` when in `trace` mode, for which it writes on just the
    /// `log_file`.  If you want to change where either set of macros writes
    /// to, you should also change the other macros in this program for that
    /// set; each such macro begins with `print_` or `trace_pr_`.
    // §3
    pub fn print_a_newline(&mut self) {
        {
            crate::system::wr_ln(&mut self.log_file);
        }
        {
            crate::system::wr_ln(&mut self.standard_output);
        }
    }

    /// A global variable called `history` will contain one of four values at
    /// the end of every run: `spotless` means that no unusual messages were
    /// printed; `warning_message` means that a message of possible interest
    /// was printed but no serious errors were detected; `error_message` means
    /// that at least one error was found; `fatal_message` means that the
    /// program terminated abnormally. The value of `history` does not
    /// influence the behavior of the program; it is simply computed for the
    /// convenience of systems that might want to use such information.
    // §14
    pub fn mark_warning(&mut self) {
        if (self.history == warning_message) {
            self.err_count = (self.err_count).wrapping_add(1i32);
        } else {
            if (self.history == spotless) {
                {
                    self.history = warning_message;
                    self.err_count = 1i32;
                }
            }
        }
    }

    /// A global variable called `history` will contain one of four values at
    /// the end of every run: `spotless` means that no unusual messages were
    /// printed; `warning_message` means that a message of possible interest
    /// was printed but no serious errors were detected; `error_message` means
    /// that at least one error was found; `fatal_message` means that the
    /// program terminated abnormally. The value of `history` does not
    /// influence the behavior of the program; it is simply computed for the
    /// convenience of systems that might want to use such information.
    // §14
    pub fn mark_error(&mut self) {
        if (self.history < error_message) {
            {
                self.history = error_message;
                self.err_count = 1i32;
            }
        } else {
            self.err_count = (self.err_count).wrapping_add(1i32);
        }
    }

    /// A global variable called `history` will contain one of four values at
    /// the end of every run: `spotless` means that no unusual messages were
    /// printed; `warning_message` means that a message of possible interest
    /// was printed but no serious errors were detected; `error_message` means
    /// that at least one error was found; `fatal_message` means that the
    /// program terminated abnormally. The value of `history` does not
    /// influence the behavior of the program; it is simply computed for the
    /// convenience of systems that might want to use such information.
    // §14
    pub fn mark_fatal(&mut self) {
        self.history = fatal_message;
    }

    /// When something in the program wants to be bigger or something out
    /// there wants to be smaller, it's time to call it a run.  Here's the
    /// first of several macros that have associated procedures so that they
    /// produce less inline code.
    // §37
    pub fn print_overflow(&mut self) {
        {
            {
                crate::system::wr_str(&mut self.log_file, "Sorry---you've exceeded BibTeX's ");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "Sorry---you've exceeded BibTeX's ");
            }
        }
        self.mark_fatal();
    }

    /// When something happens that the program thinks is impossible,
    /// call the maintainer.
    // §38
    pub fn print_confusion(&mut self) {
        {
            {
                crate::system::wr_str(&mut self.log_file, "---this can't happen");
                crate::system::wr_ln(&mut self.log_file);
            }
            {
                crate::system::wr_str(&mut self.standard_output, "---this can't happen");
                crate::system::wr_ln(&mut self.standard_output);
            }
        }
        {
            {
                crate::system::wr_str(&mut self.log_file, "*Please notify the BibTeX maintainer*");
                crate::system::wr_ln(&mut self.log_file);
            }
            {
                crate::system::wr_str(&mut self.standard_output, "*Please notify the BibTeX maintainer*");
                crate::system::wr_ln(&mut self.standard_output);
            }
        }
        self.mark_fatal();
    }

    /// When a buffer overflows, it's time to complain (and then quit).
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §39
    pub fn buffer_overflow(&mut self) {
        self.log_realloc("buffer", 1i32, (self.buf_size).wrapping_add(BUF_SIZE), self.buf_size);
        self.buffer.resize_len((((self.buf_size).wrapping_add(BUF_SIZE)) as usize) + 1);
        self.log_realloc("sv_buffer", 1i32, (self.buf_size).wrapping_add(BUF_SIZE), self.buf_size);
        self.sv_buffer.resize_len((((self.buf_size).wrapping_add(BUF_SIZE)) as usize) + 1);
        self.log_realloc("ex_buf", 1i32, (self.buf_size).wrapping_add(BUF_SIZE), self.buf_size);
        self.ex_buf.resize_len((((self.buf_size).wrapping_add(BUF_SIZE)) as usize) + 1);
        self.log_realloc("out_buf", 1i32, (self.buf_size).wrapping_add(BUF_SIZE), self.buf_size);
        self.out_buf.resize_len((((self.buf_size).wrapping_add(BUF_SIZE)) as usize) + 1);
        self.log_realloc("name_tok", 4i32, (self.buf_size).wrapping_add(BUF_SIZE), self.buf_size);
        self.name_tok.resize_len((((self.buf_size).wrapping_add(BUF_SIZE)) as usize) + 1);
        self.log_realloc("name_sep_char", 1i32, (self.buf_size).wrapping_add(BUF_SIZE), self.buf_size);
        self.name_sep_char.resize_len((((self.buf_size).wrapping_add(BUF_SIZE)) as usize) + 1);
        self.buf_size = (self.buf_size).wrapping_add(BUF_SIZE);
    }

    /// The `input_ln` function brings the next line of input from the
    /// specified file into available positions of the buffer array and
    /// returns the value `true`, unless the file has already been entirely
    /// read, in which case it returns `false` and sets `last:=0`.  In
    /// general, the `ASCII_code` numbers that represent the next line of the
    /// file are input into `buffer[0]`, `buffer[1]`, \dots, `buffer[last-1]`;
    /// and the global variable `last` is set equal to the length of the line.
    /// Trailing `white_space` characters are removed from the line
    /// (`white_space` characters are explained in the character-set section%
    /// ---most likely they're blanks); thus, either `last=0` (in which case
    /// the line was entirely blank) or `lex_class[buffer[last-1]]<>white_space`.
    /// An overflow error is given if the normal actions of `input_ln` would
    /// make `last>buf_size`.
    /// Standard \PASCAL\ says that a file should have `eoln` immediately
    /// ...
    // §40
    pub fn input_ln(&mut self, f: &mut crate::system::AlphaFile) -> bool {
        let mut input_ln: bool = false;
        self.last = 0i32;
        if crate::system::eof(&(*f)) {
            input_ln = false;
        } else {
            {
                'l_L15_f: {
                    while (!crate::system::eoln(&(*f))) {
                        {
                            if (self.last >= self.buf_size) {
                                self.buffer_overflow();
                            }
                            { let __ix0 = self.last; let __v1 = { let __s2 = ({ let mut __f0 = ::core::mem::take(&mut (*f)); let __r = self.getc(&mut __f0); (*f) = __f0; __r }) as usize; self.xord[__s2] }; self.buffer[(__ix0) as usize] = __v1; }
                            self.last = (self.last).wrapping_add(1i32);
                        }
                    }
                    { let mut __f0 = ::core::mem::take(&mut (*f)); let __r = self.vgetc(&mut __f0); (*f) = __f0; __r };
                    while (self.last > 0i32) {
                        if (self.lex_class[(self.buffer[((self.last).wrapping_sub(1i32)) as usize]) as usize] == white_space) {
                            self.last = (self.last).wrapping_sub(1i32);
                        } else {
                            break 'l_L15_f;
                        }
                    }
                }
                input_ln = true;
            }
        }
        input_ln
    }

    /// And here are the associated procedures.  Note: The `term_out` file is
    /// system dependent.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §44
    pub fn out_pool_str(&mut self, f: &mut crate::system::AlphaFile, mut s: str_number) {
        let mut i: pool_pointer = 0; // §44
        if (((s < 0i32) || (s >= (self.str_ptr).wrapping_add(3i32))) || (s >= self.max_strings)) {
            {
                {
                    {
                        let __w1 = s;
                        crate::system::wr_str(&mut self.log_file, "Illegal string number:");
                        crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                    }
                    {
                        let __w1 = s;
                        crate::system::wr_str(&mut self.standard_output, "Illegal string number:");
                        crate::system::wr_int(&mut self.standard_output, __w1, 0i32);
                    }
                }
                self.print_confusion();
                crate::system::end_of_TEX(self);
            }
        }
        {
            let __for_end_2 = (self.str_start[((s).wrapping_add(1i32)) as usize]).wrapping_sub(1i32);
            i = self.str_start[(s) as usize];
            while i <= __for_end_2 {
                {
                    let __w0 = self.xchr[(self.str_pool[(i) as usize]) as usize];
                    crate::system::wr_char(&mut (*f), __w0);
                }
                i = i.wrapping_add(1);
            }
        }
    }

    /// And here are the associated procedures.  Note: The `term_out` file is
    /// system dependent.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §44
    pub fn print_a_pool_str(&mut self, mut s: str_number) {
        { let mut __f0 = ::core::mem::take(&mut self.standard_output); let __r = self.out_pool_str(&mut __f0, s); self.standard_output = __f0; __r };
        { let mut __f0 = ::core::mem::take(&mut self.log_file); let __r = self.out_pool_str(&mut __f0, s); self.log_file = __f0; __r };
    }

    /// Strings are created by appending character codes to `str_pool`.
    /// The macro called `append_char`, defined here, does not check to see if the
    /// value of `pool_ptr` has gotten too high; this test is supposed to be
    /// made before `append_char` is used.
    /// To test if there is room to append `l` more characters to `str_pool`,
    /// we shall write `str_room(l)`, which aborts \BibTeX\ and gives an
    /// error message if there isn't enough room.
    // §46
    pub fn pool_overflow(&mut self) {
        self.log_realloc("str_pool", 1i32, (self.pool_size).wrapping_add(POOL_SIZE), self.pool_size);
        self.str_pool.resize_len((((self.pool_size).wrapping_add(POOL_SIZE)) as usize) + 1);
        self.pool_size = (self.pool_size).wrapping_add(POOL_SIZE);
    }

    /// And here are the associated procedures.  Note: The `term_out` file is
    /// system dependent.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §75
    pub fn out_token(&mut self, f: &mut crate::system::AlphaFile) {
        let mut i: buf_pointer = 0; // §75
        i = self.buf_ptr1;
        while (i < self.buf_ptr2) {
            {
                {
                    let __w0 = self.xchr[(self.buffer[(i) as usize]) as usize];
                    crate::system::wr_char(&mut (*f), __w0);
                }
                i = (i).wrapping_add(1i32);
            }
        }
    }

    /// And here are the associated procedures.  Note: The `term_out` file is
    /// system dependent.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §75
    pub fn print_a_token(&mut self) {
        { let mut __f0 = ::core::mem::take(&mut self.standard_output); let __r = self.out_token(&mut __f0); self.standard_output = __f0; __r };
        { let mut __f0 = ::core::mem::take(&mut self.log_file); let __r = self.out_token(&mut __f0); self.log_file = __f0; __r };
    }

    /// The `print_bad_input_line` procedure prints the current input line,
    /// splitting it at the character being scanned: It prints `buffer[0]`,
    /// `buffer[1]`, \dots, `buffer[buf_ptr2-1]` on one line and
    /// `buffer[buf_ptr2]`, \dots, `buffer[last-1]` on the next (and both
    /// lines start with a colon between two `space`s).  Each `white_space`
    /// character is printed as a `space`.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §88
    pub fn print_bad_input_line(&mut self) {
        let mut bf_ptr: buf_pointer = 0; // §88
        {
            {
                crate::system::wr_str(&mut self.log_file, " : ");
            }
            {
                crate::system::wr_str(&mut self.standard_output, " : ");
            }
        }
        bf_ptr = 0i32;
        while (bf_ptr < self.buf_ptr2) {
            {
                if (self.lex_class[(self.buffer[(bf_ptr) as usize]) as usize] == white_space) {
                    {
                        {
                            let __w0 = self.xchr[(space) as usize];
                            crate::system::wr_char(&mut self.log_file, __w0);
                        }
                        {
                            let __w0 = self.xchr[(space) as usize];
                            crate::system::wr_char(&mut self.standard_output, __w0);
                        }
                    }
                } else {
                    {
                        {
                            let __w0 = self.xchr[(self.buffer[(bf_ptr) as usize]) as usize];
                            crate::system::wr_char(&mut self.log_file, __w0);
                        }
                        {
                            let __w0 = self.xchr[(self.buffer[(bf_ptr) as usize]) as usize];
                            crate::system::wr_char(&mut self.standard_output, __w0);
                        }
                    }
                }
                bf_ptr = (bf_ptr).wrapping_add(1i32);
            }
        }
        self.print_a_newline();
        {
            {
                crate::system::wr_str(&mut self.log_file, " : ");
            }
            {
                crate::system::wr_str(&mut self.standard_output, " : ");
            }
        }
        bf_ptr = 0i32;
        while (bf_ptr < self.buf_ptr2) {
            {
                {
                    {
                        let __w0 = self.xchr[(space) as usize];
                        crate::system::wr_char(&mut self.log_file, __w0);
                    }
                    {
                        let __w0 = self.xchr[(space) as usize];
                        crate::system::wr_char(&mut self.standard_output, __w0);
                    }
                }
                bf_ptr = (bf_ptr).wrapping_add(1i32);
            }
        }
        bf_ptr = self.buf_ptr2;
        while (bf_ptr < self.last) {
            {
                if (self.lex_class[(self.buffer[(bf_ptr) as usize]) as usize] == white_space) {
                    {
                        {
                            let __w0 = self.xchr[(space) as usize];
                            crate::system::wr_char(&mut self.log_file, __w0);
                        }
                        {
                            let __w0 = self.xchr[(space) as usize];
                            crate::system::wr_char(&mut self.standard_output, __w0);
                        }
                    }
                } else {
                    {
                        {
                            let __w0 = self.xchr[(self.buffer[(bf_ptr) as usize]) as usize];
                            crate::system::wr_char(&mut self.log_file, __w0);
                        }
                        {
                            let __w0 = self.xchr[(self.buffer[(bf_ptr) as usize]) as usize];
                            crate::system::wr_char(&mut self.standard_output, __w0);
                        }
                    }
                }
                bf_ptr = (bf_ptr).wrapping_add(1i32);
            }
        }
        self.print_a_newline();
        bf_ptr = 0i32;
        while ((bf_ptr < self.buf_ptr2) && (self.lex_class[(self.buffer[(bf_ptr) as usize]) as usize] == white_space)) {
            bf_ptr = (bf_ptr).wrapping_add(1i32);
        }
        if (bf_ptr == self.buf_ptr2) {
            {
                {
                    crate::system::wr_str(&mut self.log_file, "(Error may have been on previous line)");
                    crate::system::wr_ln(&mut self.log_file);
                }
                {
                    crate::system::wr_str(&mut self.standard_output, "(Error may have been on previous line)");
                    crate::system::wr_ln(&mut self.standard_output);
                }
            }
        }
        self.mark_error();
    }

    /// This little procedure exists because it's used by at least two other
    /// procedures and thus saves some space.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §89
    pub fn print_skipping_whatever_remains(&mut self) {
        {
            {
                crate::system::wr_str(&mut self.log_file, "I'm skipping whatever remains of this ");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "I'm skipping whatever remains of this ");
            }
        }
    }

    /// I mean, this is truly disgraceful.  A user has to type something in to
    /// the terminal just once during the entire run.  And it's not some
    /// complicated string where you have to get every last punctuation mark
    /// just right, and it's not some fancy list where you get nervous because
    /// if you forget one item you have to type the whole thing again; it's
    /// just a simple, ordinary, file name.  Now you'd think a five-year-old
    /// could do it; you'd think it's so simple a user should be able to do it
    /// in his sleep.  But noooooooooo.  He had to sit there droning on and on
    /// about who knows what until he exceeded the bounds of common sense, and
    /// he probably didn't even realize it.  Just pitiful.  What's this world
    /// coming to?  We should probably just delete all his files and be done
    /// with him.  Note: The `term_out` file is system dependent.
    // §91
    pub fn sam_too_long_file_name_print(&mut self) {
        {
            crate::system::wr_str(&mut self.standard_output, "File name `");
        }
        self.name_ptr = 1i32;
        while (self.name_ptr <= self.aux_name_length) {
            {
                {
                    let __w0 = self.name_of_file[(self.name_ptr) as usize];
                    crate::system::wr_char(&mut self.standard_output, __w0);
                }
                self.name_ptr = (self.name_ptr).wrapping_add(1i32);
            }
        }
        {
            crate::system::wr_str(&mut self.standard_output, "' is too long");
            crate::system::wr_ln(&mut self.standard_output);
        }
    }

    /// We've abused the user enough for one section; suffice it to
    /// say here that most of what we said last module still applies.
    /// Note: The `term_out` file is system dependent.
    // §92
    pub fn sam_wrong_file_name_print(&mut self) {
        {
            crate::system::wr_str(&mut self.standard_output, "I couldn't open file name `");
        }
        self.name_ptr = 1i32;
        while (self.name_ptr <= self.name_length) {
            {
                {
                    let __w0 = self.name_of_file[(self.name_ptr) as usize];
                    crate::system::wr_char(&mut self.standard_output, __w0);
                }
                self.name_ptr = (self.name_ptr).wrapping_add(1i32);
            }
        }
        {
            let __w0 = b'\'';
            crate::system::wr_char(&mut self.standard_output, __w0);
            crate::system::wr_ln(&mut self.standard_output);
        }
    }

    /// Print the name of the current \.{.aux} file, followed by a `newline`.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §100
    pub fn print_aux_name(&mut self) {
        self.print_a_pool_str(self.aux_list[(self.aux_ptr) as usize]);
        self.print_a_newline();
    }

    /// Print the name of the current \.{.aux} file, followed by a `newline`.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §100
    pub fn log_pr_aux_name(&mut self) {
        {
            { let mut __f0 = ::core::mem::take(&mut self.log_file); let __r = self.out_pool_str(&mut __f0, self.aux_list[(self.aux_ptr) as usize]); self.log_file = __f0; __r };
        }
        {
            {
                crate::system::wr_ln(&mut self.log_file);
            }
        }
    }

    /// When we find a bug, we print a message and flush the rest of the line.
    /// This macro must be called from within a procedure that has an `exit`
    /// label.
    // §103
    pub fn aux_err_print(&mut self) {
        {
            {
                let __w1 = self.aux_ln_stack[(self.aux_ptr) as usize];
                crate::system::wr_str(&mut self.log_file, "---line ");
                crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                crate::system::wr_str(&mut self.log_file, " of file ");
            }
            {
                let __w1 = self.aux_ln_stack[(self.aux_ptr) as usize];
                crate::system::wr_str(&mut self.standard_output, "---line ");
                crate::system::wr_int(&mut self.standard_output, __w1, 0i32);
                crate::system::wr_str(&mut self.standard_output, " of file ");
            }
        }
        self.print_aux_name();
        self.print_bad_input_line();
        self.print_skipping_whatever_remains();
        {
            {
                crate::system::wr_str(&mut self.log_file, "command");
                crate::system::wr_ln(&mut self.log_file);
            }
            {
                crate::system::wr_str(&mut self.standard_output, "command");
                crate::system::wr_ln(&mut self.standard_output);
            }
        }
    }

    /// Here are a bunch of macros whose print statements are used at least
    /// twice.  Thus we save space by making the statements procedures.  This
    /// macro complains when there's a repeated command that's to be used just
    /// once.
    // §104
    pub fn aux_err_illegal_another_print(&mut self, mut cmd_num: i32) {
        {
            {
                crate::system::wr_str(&mut self.log_file, "Illegal, another \\bib");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "Illegal, another \\bib");
            }
        }
        match cmd_num {
            n_aux_bibdata => {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "data");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "data");
                    }
                }
            }
            n_aux_bibstyle => {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "style");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "style");
                    }
                }
            }
            _ => {
                {
                    {
                        {
                            crate::system::wr_str(&mut self.log_file, "Illegal auxiliary-file command");
                        }
                        {
                            crate::system::wr_str(&mut self.standard_output, "Illegal auxiliary-file command");
                        }
                    }
                    self.print_confusion();
                    crate::system::end_of_TEX(self);
                }
            }
        }
        {
            {
                crate::system::wr_str(&mut self.log_file, " command");
            }
            {
                crate::system::wr_str(&mut self.standard_output, " command");
            }
        }
    }

    /// This one complains when a command is missing its `right_brace`.
    // §105
    pub fn aux_err_no_right_brace_print(&mut self) {
        {
            {
                let __w1 = self.xchr[(right_brace) as usize];
                let __w2 = b'"';
                crate::system::wr_str(&mut self.log_file, "No \"");
                crate::system::wr_char(&mut self.log_file, __w1);
                crate::system::wr_char(&mut self.log_file, __w2);
            }
            {
                let __w1 = self.xchr[(right_brace) as usize];
                let __w2 = b'"';
                crate::system::wr_str(&mut self.standard_output, "No \"");
                crate::system::wr_char(&mut self.standard_output, __w1);
                crate::system::wr_char(&mut self.standard_output, __w2);
            }
        }
    }

    /// This one complains when a command has stuff after its `right_brace`.
    // §106
    pub fn aux_err_stuff_after_right_brace_print(&mut self) {
        {
            {
                let __w1 = self.xchr[(right_brace) as usize];
                let __w2 = b'"';
                crate::system::wr_str(&mut self.log_file, "Stuff after \"");
                crate::system::wr_char(&mut self.log_file, __w1);
                crate::system::wr_char(&mut self.log_file, __w2);
            }
            {
                let __w1 = self.xchr[(right_brace) as usize];
                let __w2 = b'"';
                crate::system::wr_str(&mut self.standard_output, "Stuff after \"");
                crate::system::wr_char(&mut self.standard_output, __w1);
                crate::system::wr_char(&mut self.standard_output, __w2);
            }
        }
    }

    /// And this one complains when a command has `white_space` in its
    /// argument.
    // §107
    pub fn aux_err_white_space_in_argument_print(&mut self) {
        {
            {
                crate::system::wr_str(&mut self.log_file, "White space in argument");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "White space in argument");
            }
        }
    }

}
