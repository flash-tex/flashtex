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
    /// Here's a procedure we'll need shortly.  It prints the name of the
    /// current \.{.bib} file, followed by a `newline`.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §113
    pub fn str_ends_with(&mut self, mut s: str_number, mut ext: str_number) -> bool {
        let mut str_ends_with: bool = false;
        let mut i: i32 = 0; // §113
        let mut str_idx: i32 = 0; // §113
        let mut ext_idx: i32 = 0; // §113
        let mut str_char: ASCII_code = 0; // §113
        let mut ext_char: ASCII_code = 0; // §113
        'l_exit_f: {
            str_ends_with = false;
            if ((self.str_start[((ext).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(ext) as usize]) > (self.str_start[((s).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(s) as usize])) {
                break 'l_exit_f;
            }
            str_idx = ((self.str_start[((s).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(s) as usize])).wrapping_sub(1i32);
            ext_idx = ((self.str_start[((ext).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(ext) as usize])).wrapping_sub(1i32);
            while (ext_idx >= 0i32) {
                {
                    str_char = self.str_pool[((self.str_start[(s) as usize]).wrapping_add(str_idx)) as usize];
                    ext_char = self.str_pool[((self.str_start[(ext) as usize]).wrapping_add(ext_idx)) as usize];
                    if (str_char != ext_char) {
                        break 'l_exit_f;
                    }
                    str_idx = (str_idx).wrapping_sub(1i32);
                    ext_idx = (ext_idx).wrapping_sub(1i32);
                }
            }
            str_ends_with = true;
        }
        str_ends_with
    }

    /// Here's a procedure we'll need shortly.  It prints the name of the
    /// current \.{.bib} file, followed by a `newline`.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §113
    pub fn print_bib_name(&mut self) {
        self.print_a_pool_str(self.bib_list[(self.bib_ptr) as usize]);
        if (!self.str_ends_with(self.bib_list[(self.bib_ptr) as usize], self.s_bib_extension)) {
            self.print_a_pool_str(self.s_bib_extension);
        }
        self.print_a_newline();
    }

    /// Here's a procedure we'll need shortly.  It prints the name of the
    /// current \.{.bib} file, followed by a `newline`.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §113
    pub fn log_pr_bib_name(&mut self) {
        {
            { let mut __f0 = ::core::mem::take(&mut self.log_file); let __r = self.out_pool_str(&mut __f0, self.bib_list[(self.bib_ptr) as usize]); self.log_file = __f0; __r };
        }
        if (!self.str_ends_with(self.bib_list[(self.bib_ptr) as usize], self.s_bib_extension)) {
            {
                { let mut __f0 = ::core::mem::take(&mut self.log_file); let __r = self.out_pool_str(&mut __f0, self.s_bib_extension); self.log_file = __f0; __r };
            }
        }
        {
            {
                crate::system::wr_ln(&mut self.log_file);
            }
        }
    }

    /// Print the name of the \.{.bst} file, followed by a `newline`.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §120
    pub fn print_bst_name(&mut self) {
        self.print_a_pool_str(self.bst_str);
        self.print_a_pool_str(self.s_bst_extension);
        self.print_a_newline();
    }

    /// Print the name of the \.{.bst} file, followed by a `newline`.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §120
    pub fn log_pr_bst_name(&mut self) {
        {
            { let mut __f0 = ::core::mem::take(&mut self.log_file); let __r = self.out_pool_str(&mut __f0, self.bst_str); self.log_file = __f0; __r };
        }
        {
            { let mut __f0 = ::core::mem::take(&mut self.log_file); let __r = self.out_pool_str(&mut __f0, self.s_bst_extension); self.log_file = __f0; __r };
        }
        {
            {
                crate::system::wr_ln(&mut self.log_file);
            }
        }
    }

    /// Here's a serious complaint (that is, a bug) concerning hash problems.
    /// This is the first of several similar bug-procedures that exist only
    /// because they save space.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §129
    pub fn hash_cite_confusion(&mut self) {
        {
            {
                {
                    crate::system::wr_str(&mut self.log_file, "Cite hash error");
                }
                {
                    crate::system::wr_str(&mut self.standard_output, "Cite hash error");
                }
            }
            self.print_confusion();
            crate::system::end_of_TEX(self);
        }
    }

    /// Complain if somebody's got a cite fetish.  This procedure is called
    /// when were about to add another cite key to `cite_list`.  It assumes
    /// that `cite_loc` gives the potential cite key's hash table location.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §130
    pub fn check_cite_overflow(&mut self, mut last_cite: cite_number) {
        if (last_cite == self.max_cites) {
            {
                self.log_realloc("cite_list", 4i32, (self.max_cites).wrapping_add(MAX_CITES), self.max_cites);
                self.cite_list.resize_len((((self.max_cites).wrapping_add(MAX_CITES)) as usize) + 1);
                self.log_realloc("type_list", 4i32, (self.max_cites).wrapping_add(MAX_CITES), self.max_cites);
                self.type_list.resize_len((((self.max_cites).wrapping_add(MAX_CITES)) as usize) + 1);
                self.log_realloc("entry_exists", 4i32, (self.max_cites).wrapping_add(MAX_CITES), self.max_cites);
                self.entry_exists.resize_len((((self.max_cites).wrapping_add(MAX_CITES)) as usize) + 1);
                self.log_realloc("cite_info", 4i32, (self.max_cites).wrapping_add(MAX_CITES), self.max_cites);
                self.cite_info.resize_len((((self.max_cites).wrapping_add(MAX_CITES)) as usize) + 1);
                self.max_cites = (self.max_cites).wrapping_add(MAX_CITES);
                while (last_cite < self.max_cites) {
                    {
                        self.type_list[(last_cite) as usize] = empty;
                        self.cite_info[(last_cite) as usize] = any_value;
                        last_cite = (last_cite).wrapping_add(1i32);
                    }
                }
            }
        }
    }

    /// We must complain if anything's amiss.
    // §136
    pub fn aux_end1_err_print(&mut self) {
        {
            {
                crate::system::wr_str(&mut self.log_file, "I found no ");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "I found no ");
            }
        }
    }

    /// We must complain if anything's amiss.
    // §136
    pub fn aux_end2_err_print(&mut self) {
        {
            {
                crate::system::wr_str(&mut self.log_file, "---while reading file ");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "---while reading file ");
            }
        }
        self.print_aux_name();
        self.mark_error();
    }

    /// This little procedure exists because it's used by at least two other
    /// procedures and thus saves some space.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §140
    pub fn bst_ln_num_print(&mut self) {
        {
            {
                let __w1 = self.bst_line_num;
                crate::system::wr_str(&mut self.log_file, "--line ");
                crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                crate::system::wr_str(&mut self.log_file, " of file ");
            }
            {
                let __w1 = self.bst_line_num;
                crate::system::wr_str(&mut self.standard_output, "--line ");
                crate::system::wr_int(&mut self.standard_output, __w1, 0i32);
                crate::system::wr_str(&mut self.standard_output, " of file ");
            }
        }
        self.print_bst_name();
    }

    /// When there's a serious error parsing the \.{.bst} file, we flush the
    /// rest of the current command; a blank line is assumed to mark the end
    /// of a command (but for the purposes of error recovery only).  Thus,
    /// error recovery will be better if style designers leave blank lines
    /// between \.{.bst} commands.  This macro must be called from within a
    /// procedure that has an `exit` label.
    // §141
    pub fn bst_err_print_and_look_for_blank_line(&mut self) {
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
        self.print_bad_input_line();
        while (self.last != 0i32) {
            if (!{ let mut __f0 = ::core::mem::take(&mut self.bst_file); let __r = self.input_ln(&mut __f0); self.bst_file = __f0; __r }) {
                self.jump_to_bst_done();
            } else {
                self.bst_line_num = (self.bst_line_num).wrapping_add(1i32);
            }
        }
        self.buf_ptr2 = self.last;
    }

    /// When there's a harmless error parsing the \.{.bst} file (harmless
    /// syntactically, at least) we give just a `warning_message`.
    // §142
    pub fn bst_warn_print(&mut self) {
        self.bst_ln_num_print();
        self.mark_warning();
    }

    /// It's often illegal to end a \.{.bst} command in certain places, and
    /// this is where we come to check.
    // §145
    pub fn eat_bst_print(&mut self) {
        {
            {
                crate::system::wr_str(&mut self.log_file, "Illegal end of style file in command: ");
            }
            {
                crate::system::wr_str(&mut self.standard_output, "Illegal end of style file in command: ");
            }
        }
    }

    /// Here's another bug report.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §149
    pub fn unknwn_function_class_confusion(&mut self) {
        {
            {
                {
                    crate::system::wr_str(&mut self.log_file, "Unknown function class");
                }
                {
                    crate::system::wr_str(&mut self.standard_output, "Unknown function class");
                }
            }
            self.print_confusion();
            crate::system::end_of_TEX(self);
        }
    }

    /// Occasionally we'll want to `print` the name of one of these function
    /// classes.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §150
    pub fn print_fn_class(&mut self, mut fn_loc: hash_loc) {
        match self.fn_type[(fn_loc) as usize] {
            built_in => {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "built-in");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "built-in");
                    }
                }
            }
            wiz_defined => {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "wizard-defined");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "wizard-defined");
                    }
                }
            }
            int_literal => {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "integer-literal");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "integer-literal");
                    }
                }
            }
            str_literal => {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "string-literal");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "string-literal");
                    }
                }
            }
            field => {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "field");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "field");
                    }
                }
            }
            int_entry_var => {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "integer-entry-variable");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "integer-entry-variable");
                    }
                }
            }
            str_entry_var => {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "string-entry-variable");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "string-entry-variable");
                    }
                }
            }
            int_global_var => {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "integer-global-variable");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "integer-global-variable");
                    }
                }
            }
            str_global_var => {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "string-global-variable");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "string-global-variable");
                    }
                }
            }
            _ => {
                self.unknwn_function_class_confusion();
            }
        }
    }

    /// Here's another bug.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §157
    pub fn id_scanning_confusion(&mut self) {
        {
            {
                {
                    crate::system::wr_str(&mut self.log_file, "Identifier scanning error");
                }
                {
                    crate::system::wr_str(&mut self.standard_output, "Identifier scanning error");
                }
            }
            self.print_confusion();
            crate::system::end_of_TEX(self);
        }
    }

    /// This macro is used to scan all \.{.bst} identifiers.  The argument
    /// supplies the \.{.bst} command name.  The associated procedure simply
    /// prints an error message.
    // §158
    pub fn bst_id_print(&mut self) {
        if (self.scan_result == id_null) {
            {
                {
                    let __w0 = b'"';
                    let __w1 = self.xchr[(self.buffer[(self.buf_ptr2) as usize]) as usize];
                    crate::system::wr_char(&mut self.log_file, __w0);
                    crate::system::wr_char(&mut self.log_file, __w1);
                    crate::system::wr_str(&mut self.log_file, "\" begins identifier, command: ");
                }
                {
                    let __w0 = b'"';
                    let __w1 = self.xchr[(self.buffer[(self.buf_ptr2) as usize]) as usize];
                    crate::system::wr_char(&mut self.standard_output, __w0);
                    crate::system::wr_char(&mut self.standard_output, __w1);
                    crate::system::wr_str(&mut self.standard_output, "\" begins identifier, command: ");
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
                        crate::system::wr_str(&mut self.log_file, "\" immediately follows identifier, command: ");
                    }
                    {
                        let __w0 = b'"';
                        let __w1 = self.xchr[(self.buffer[(self.buf_ptr2) as usize]) as usize];
                        crate::system::wr_char(&mut self.standard_output, __w0);
                        crate::system::wr_char(&mut self.standard_output, __w1);
                        crate::system::wr_str(&mut self.standard_output, "\" immediately follows identifier, command: ");
                    }
                }
            } else {
                self.id_scanning_confusion();
            }
        }
    }

    /// This macro just makes sure we're at a `left_brace`.
    // §159
    pub fn bst_left_brace_print(&mut self) {
        {
            {
                let __w0 = b'"';
                let __w1 = self.xchr[(left_brace) as usize];
                crate::system::wr_char(&mut self.log_file, __w0);
                crate::system::wr_char(&mut self.log_file, __w1);
                crate::system::wr_str(&mut self.log_file, "\" is missing in command: ");
            }
            {
                let __w0 = b'"';
                let __w1 = self.xchr[(left_brace) as usize];
                crate::system::wr_char(&mut self.standard_output, __w0);
                crate::system::wr_char(&mut self.standard_output, __w1);
                crate::system::wr_str(&mut self.standard_output, "\" is missing in command: ");
            }
        }
    }

    /// And this one, a `right_brace`.
    // §160
    pub fn bst_right_brace_print(&mut self) {
        {
            {
                let __w0 = b'"';
                let __w1 = self.xchr[(right_brace) as usize];
                crate::system::wr_char(&mut self.log_file, __w0);
                crate::system::wr_char(&mut self.log_file, __w1);
                crate::system::wr_str(&mut self.log_file, "\" is missing in command: ");
            }
            {
                let __w0 = b'"';
                let __w1 = self.xchr[(right_brace) as usize];
                crate::system::wr_char(&mut self.standard_output, __w0);
                crate::system::wr_char(&mut self.standard_output, __w1);
                crate::system::wr_str(&mut self.standard_output, "\" is missing in command: ");
            }
        }
    }

    /// This macro complains if we've already encountered a function to be
    /// inserted into the hash table.
    // §161
    pub fn already_seen_function_print(&mut self, mut seen_fn_loc: hash_loc) {
        'l_exit_f: {
            self.print_a_pool_str(self.hash_text[(seen_fn_loc) as usize]);
            {
                {
                    crate::system::wr_str(&mut self.log_file, " is already a type \"");
                }
                {
                    crate::system::wr_str(&mut self.standard_output, " is already a type \"");
                }
            }
            self.print_fn_class(seen_fn_loc);
            {
                {
                    crate::system::wr_str(&mut self.log_file, "\" function name");
                    crate::system::wr_ln(&mut self.log_file);
                }
                {
                    crate::system::wr_str(&mut self.standard_output, "\" function name");
                    crate::system::wr_ln(&mut self.standard_output);
                }
            }
            {
                self.bst_err_print_and_look_for_blank_line();
                break 'l_exit_f;
            }
        }
    }

    /// This little procedure exists because it's used by at least two other
    /// procedures and thus saves some space.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §211
    pub fn bib_ln_num_print(&mut self) {
        {
            {
                let __w1 = self.bib_line_num;
                crate::system::wr_str(&mut self.log_file, "--line ");
                crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                crate::system::wr_str(&mut self.log_file, " of file ");
            }
            {
                let __w1 = self.bib_line_num;
                crate::system::wr_str(&mut self.standard_output, "--line ");
                crate::system::wr_int(&mut self.standard_output, __w1, 0i32);
                crate::system::wr_str(&mut self.standard_output, " of file ");
            }
        }
        self.print_bib_name();
    }

    /// When there's a serious error parsing a \.{.bib} file, we flush
    /// everything up to the beginning of the next entry.
    // §212
    pub fn bib_err_print(&mut self) {
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
        self.bib_ln_num_print();
        self.print_bad_input_line();
        self.print_skipping_whatever_remains();
        if self.at_bib_command {
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
        } else {
            {
                {
                    crate::system::wr_str(&mut self.log_file, "entry");
                    crate::system::wr_ln(&mut self.log_file);
                }
                {
                    crate::system::wr_str(&mut self.standard_output, "entry");
                    crate::system::wr_ln(&mut self.standard_output);
                }
            }
        }
    }

    /// When there's a harmless error parsing a \.{.bib} file, we just give a
    /// warning message.  This is always called after other stuff has been
    /// printed out.
    // §213
    pub fn bib_warn_print(&mut self) {
        self.bib_ln_num_print();
        self.mark_warning();
    }

    /// Complain if somebody's got a field fetish.
    /// @<Procedures and functions for all file I/O, error messages, and such
    // §217
    pub fn check_field_overflow(&mut self, mut total_fields: i32) {
        let mut f_ptr: field_loc = 0; // §217
        let mut start_fields: field_loc = 0; // §217
        if (total_fields > self.max_fields) {
            {
                start_fields = self.max_fields;
                self.log_realloc("field_info", 4i32, (total_fields).wrapping_add(MAX_FIELDS), self.max_fields);
                self.field_info.resize_len((((total_fields).wrapping_add(MAX_FIELDS)) as usize) + 1);
                self.max_fields = (total_fields).wrapping_add(MAX_FIELDS);
                {
                    let __for_end_4 = (self.max_fields).wrapping_sub(1i32);
                    f_ptr = start_fields;
                    while f_ptr <= __for_end_4 {
                        {
                            self.field_info[(f_ptr) as usize] = missing;
                        }
                        f_ptr = f_ptr.wrapping_add(1);
                    }
                }
            }
        }
    }

}
