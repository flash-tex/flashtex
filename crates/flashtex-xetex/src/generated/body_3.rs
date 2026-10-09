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
    /// In case you are getting bored, here is a slightly less trivial routine:
    /// Given a string of lowercase letters, like `\.{pt}' or `\.{plus}' or
    /// `\.{width}', the `scan_keyword` routine checks to see whether the next
    /// tokens of input match this string. The match must be exact, except that
    /// uppercase letters will match their lowercase counterparts; uppercase
    /// equivalents are determined by subtracting `"a"-"A"`, rather than using the
    /// `uc_code` table, since \TeX\ uses this routine only for its own limited
    /// set of keywords.
    /// If a match is found, the characters are effectively removed from the input
    /// and `true` is returned. Otherwise `false` is returned, and the input
    /// is left essentially unchanged (except for the fact that some macros
    /// may have been expanded, etc.).
    // §441
    pub fn scan_keyword(&mut self, mut s: str_number) -> bool {
        let mut scan_keyword: bool = false;
        let mut p: halfword = 0; // §441
        let mut q: halfword = 0; // §441
        let mut k: pool_pointer = 0; // §441
        let mut save_cur_cs: halfword = 0; // §441
        'l_exit_f: {
            p = backup_head;
            self.mem[crate::ix::U((p) as usize)].set_hh_rh((268435455i32).wrapping_neg());
            if (s < too_big_char) {
                {
                    while true {
                        {
                            self.get_x_token();
                            if ((self.cur_cs == 0i32) && ((self.cur_chr == s) || (self.cur_chr == (s).wrapping_sub(32i32)))) {
                                {
                                    {
                                        q = self.get_avail();
                                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                        { let __v282 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v282); }
                                        p = q;
                                    }
                                    self.flush_list(self.mem[crate::ix::U((backup_head) as usize)].hh().rh());
                                    scan_keyword = true;
                                    break 'l_exit_f;
                                }
                            } else {
                                if ((self.cur_cmd != spacer) || (p != backup_head)) {
                                    {
                                        self.back_input();
                                        if (p != backup_head) {
                                            self.begin_token_list(self.mem[crate::ix::U((backup_head) as usize)].hh().rh(), backed_up);
                                        }
                                        scan_keyword = false;
                                        break 'l_exit_f;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            k = self.str_start[crate::ix::U(((s).wrapping_sub(65536i32)) as usize)];
            save_cur_cs = self.cur_cs;
            while (k < self.str_start[crate::ix::U((((s).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)]) {
                {
                    self.get_x_token();
                    if ((self.cur_cs == 0i32) && ((self.cur_chr == self.str_pool[crate::ix::U((k) as usize)]) || (self.cur_chr == (self.str_pool[crate::ix::U((k) as usize)]).wrapping_sub(32i32)))) {
                        {
                            {
                                q = self.get_avail();
                                self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                { let __v283 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v283); }
                                p = q;
                            }
                            k = (k).wrapping_add(1i32);
                        }
                    } else {
                        if ((self.cur_cmd != spacer) || (p != backup_head)) {
                            {
                                self.back_input();
                                if (p != backup_head) {
                                    self.begin_token_list(self.mem[crate::ix::U((backup_head) as usize)].hh().rh(), backed_up);
                                }
                                self.cur_cs = save_cur_cs;
                                scan_keyword = false;
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            self.flush_list(self.mem[crate::ix::U((backup_head) as usize)].hh().rh());
            scan_keyword = true;
        }
        scan_keyword
    }

    /// Here is a procedure that sounds an alarm when mu and non-mu units
    /// are being switched.
    // §442
    pub fn mu_error(&mut self) {
        {
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(65544i32);
            }
            self.print(65986i32);
        }
        {
            self.help_ptr = 1i32;
            self.help_line[crate::ix::U((0i32) as usize)] = 65987i32;
        }
        self.error();
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §467
    pub fn scan_glyph_number(&mut self, mut f: internal_font_number) {
        if self.scan_keyword(47i32) {
            {
                self.scan_and_pack_name();
                {
                    self.cur_val = self.map_glyph_to_index(f);
                    self.cur_val_level = int_val;
                }
            }
        } else {
            if self.scan_keyword(117i32) {
                {
                    self.scan_char_num();
                    {
                        self.cur_val = self.map_char_to_glyph(f, self.cur_val);
                        self.cur_val_level = int_val;
                    }
                }
            } else {
                self.scan_int();
            }
        }
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §467
    pub fn scan_char_class(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > char_class_limit)) {
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66027i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66028i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 65996i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §467
    pub fn scan_char_class_not_ignored(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > char_class_limit)) {
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66027i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66029i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 65996i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §467
    pub fn scan_eight_bit_int(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > 255i32)) {
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66030i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66031i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 65996i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §468
    pub fn scan_usv_num(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > biggest_usv)) {
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66032i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66033i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 65996i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §468
    pub fn scan_char_num(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > biggest_char)) {
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66032i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66034i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 65996i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// While we're at it, we might as well deal with similar routines that
    /// will be needed later.
    /// @<Declare procedures that scan restricted classes of integers
    // §469
    pub fn scan_xetex_math_char_int(&mut self) {
        self.scan_int();
        if (self.math_char_field(self.cur_val) == active_math_char) {
            {
                if (self.cur_val != active_math_char) {
                    {
                        {
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(65544i32);
                            }
                            self.print(66035i32);
                        }
                        {
                            self.help_ptr = 2i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 66036i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 66037i32;
                        }
                        self.int_error(self.cur_val);
                        self.cur_val = active_math_char;
                    }
                }
            }
        } else {
            if (self.math_char_field(self.cur_val) > biggest_usv) {
                {
                    {
                        if (self.interaction == error_stop_mode) {
                        }
                        if self.file_line_error_style_p {
                            self.print_file_line();
                        } else {
                            self.print_nl(65544i32);
                        }
                        self.print(66038i32);
                    }
                    {
                        self.help_ptr = 2i32;
                        self.help_line[crate::ix::U((1i32) as usize)] = 66039i32;
                        self.help_line[crate::ix::U((0i32) as usize)] = 65996i32;
                    }
                    self.int_error(self.cur_val);
                    self.cur_val = 0i32;
                }
            }
        }
    }

    /// While we're at it, we might as well deal with similar routines that
    /// will be needed later.
    /// @<Declare procedures that scan restricted classes of integers
    // §469
    pub fn scan_math_class_int(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > 7i32)) {
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66040i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66041i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 65996i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// While we're at it, we might as well deal with similar routines that
    /// will be needed later.
    /// @<Declare procedures that scan restricted classes of integers
    // §469
    pub fn scan_math_fam_int(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > 255i32)) {
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66042i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66043i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 65996i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// While we're at it, we might as well deal with similar routines that
    /// will be needed later.
    /// @<Declare procedures that scan restricted classes of integers
    // §469
    pub fn scan_four_bit_int(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > 15i32)) {
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66044i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66045i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 65996i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §470
    pub fn scan_fifteen_bit_int(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > 32767i32)) {
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66046i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66047i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 65996i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §471
    pub fn scan_delimiter_int(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > 134217727i32)) {
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66048i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66049i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 65996i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// @<Declare procedures that scan restricted classes of integers
    // §1622
    pub fn scan_register_num(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || (self.cur_val > self.max_reg_num)) {
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66030i32);
                }
                {
                    self.help_ptr = 2i32;
                    { let __v284 = self.max_reg_help_line; self.help_line[crate::ix::U((1i32) as usize)] = __v284; }
                    self.help_line[crate::ix::U((0i32) as usize)] = 65996i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// To be able to determine whether \.{\\write18} is enabled from within
    /// \TeX\ we also implement \.{\\eof18}.  We sort of cheat by having an
    /// additional route `scan_four_bit_int_or_18` which is the same as
    /// `scan_four_bit_int` except it also accepts the value 18.
    /// @<Declare procedures that scan restricted classes of integers
    // §1722
    pub fn scan_four_bit_int_or_18(&mut self) {
        self.scan_int();
        if ((self.cur_val < 0i32) || ((self.cur_val > 15i32) && (self.cur_val != 18i32))) {
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66044i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66045i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 65996i32;
                }
                self.int_error(self.cur_val);
                self.cur_val = 0i32;
            }
        }
    }

    /// The `get_x_or_protected` procedure is like `get_x_token` except that
    /// protected macros are not expanded.
    /// @<Declare \eTeX\ procedures for sc...
    // §1583
    pub fn get_x_or_protected(&mut self) {
        'l_exit_f: {
            while true {
                {
                    self.get_token();
                    if (self.cur_cmd <= max_command) {
                        break 'l_exit_f;
                    }
                    if ((self.cur_cmd >= call) && (self.cur_cmd < end_template)) {
                        if (self.mem[crate::ix::U((self.mem[crate::ix::U((self.cur_chr) as usize)].hh().rh()) as usize)].hh().lh() == protected_token) {
                            break 'l_exit_f;
                        }
                    }
                    self.expand();
                }
            }
        }
    }

    /// Before we forget about the format of these tables, let's deal with two
    /// of \TeX's basic scanning routines related to font information.
    /// @<Declare procedures that scan font-related stuff
    // §612
    pub fn scan_font_ident(&mut self) {
        let mut f: internal_font_number = 0; // §612
        let mut m: halfword = 0; // §612
        // §440
        loop {
            self.get_x_token();
            if (self.cur_cmd != spacer) { break; }
        }
        // §612
        if (self.cur_cmd == def_font) {
            f = self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh();
        } else {
            if (self.cur_cmd == set_font) {
                f = self.cur_chr;
            } else {
                if (self.cur_cmd == def_family) {
                    {
                        m = self.cur_chr;
                        self.scan_math_fam_int();
                        f = self.eqtb[crate::ix::U((((m).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                    }
                } else {
                    {
                        {
                            if (self.interaction == error_stop_mode) {
                            }
                            if self.file_line_error_style_p {
                                self.print_file_line();
                            } else {
                                self.print_nl(65544i32);
                            }
                            self.print(66208i32);
                        }
                        {
                            self.help_ptr = 2i32;
                            self.help_line[crate::ix::U((1i32) as usize)] = 66209i32;
                            self.help_line[crate::ix::U((0i32) as usize)] = 66210i32;
                        }
                        self.back_error();
                        f = null_font;
                    }
                }
            }
        }
        self.cur_val = f;
    }

    /// The following routine is used to implement `\.{\\fontdimen} `n` `f`'.
    /// The boolean parameter `writing` is set `true` if the calling program
    /// intends to change the parameter value.
    /// @<Declare procedures that scan font-related stuff
    // §613
    pub fn find_font_dimen(&mut self, mut writing: bool) {
        let mut f: internal_font_number = 0; // §613
        let mut n: i32 = 0; // §613
        self.scan_int();
        n = self.cur_val;
        self.scan_font_ident();
        f = self.cur_val;
        if (n <= 0i32) {
            self.cur_val = self.fmem_ptr;
        } else {
            {
                if (((writing && (n <= space_shrink_code)) && (n >= space_code)) && (self.font_glue[crate::ix::U((f) as usize)] != (268435455i32).wrapping_neg())) {
                    {
                        self.delete_glue_ref(self.font_glue[crate::ix::U((f) as usize)]);
                        self.font_glue[crate::ix::U((f) as usize)] = (268435455i32).wrapping_neg();
                    }
                }
                if (n > self.font_params[crate::ix::U((f) as usize)]) {
                    if (f < self.font_ptr) {
                        self.cur_val = self.fmem_ptr;
                    } else {
                        // §615
                        {
                            loop {
                                if (self.fmem_ptr == font_mem_size) {
                                    self.overflow(66215i32, font_mem_size);
                                }
                                { let __ix285 = self.fmem_ptr; self.font_info[crate::ix::U((__ix285) as usize)].set_int(0i32); }
                                self.fmem_ptr = (self.fmem_ptr).wrapping_add(1i32);
                                { let __v286 = (self.font_params[crate::ix::U((f) as usize)]).wrapping_add(1i32); self.font_params[crate::ix::U((f) as usize)] = __v286; }
                                if (n == self.font_params[crate::ix::U((f) as usize)]) { break; }
                            }
                            self.cur_val = (self.fmem_ptr).wrapping_sub(1i32);
                        }
                    }
                } else {
                    // §613
                    self.cur_val = (n).wrapping_add(self.param_base[crate::ix::U((f) as usize)]);
                }
            }
        }
        // §614
        if (self.cur_val == self.fmem_ptr) {
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66192i32);
                }
                self.print_esc(self.hash[crate::ix::U((((font_id_base).wrapping_add(f)) - 1179650) as usize)].rh());
                self.print(66211i32);
                self.print_int(self.font_params[crate::ix::U((f) as usize)]);
                self.print(66212i32);
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66213i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 66214i32;
                }
                self.error();
            }
        }
    }

    /// OK, we're ready for `scan_something_internal` itself. A second parameter,
    /// `negative`, is set `true` if the value that is found should be negated.
    /// It is assumed that `cur_cmd` and `cur_chr` represent the first token of
    /// the internal quantity to be scanned; an error will be signalled if
    /// `cur_cmd<min_internal` or `cur_cmd>max_internal`.
    // §447
    pub fn scan_something_internal(&mut self, mut level: small_number, mut negative: bool) {
        let mut m: halfword = 0; // §447
        let mut n: i32 = 0; // §447
        let mut k: i32 = 0; // §447
        let mut kk: i32 = 0; // §447
        let mut q: halfword = 0; // §447
        let mut r: halfword = 0; // §447
        let mut tx: halfword = 0; // §447
        let mut i: four_quarters = four_quarters::default(); // §447
        let mut p: i32 = 0; // §447
        // goto labels: restart, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                m = self.cur_chr;
                match self.cur_cmd {
                    def_code => {
                        // §448
                        {
                            self.scan_usv_num();
                            if (m == math_code_base) {
                                {
                                    self.cur_val1 = self.eqtb[crate::ix::U((((math_code_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                                    if (self.math_char_field(self.cur_val1) == active_math_char) {
                                        self.cur_val1 = 32768i32;
                                    } else {
                                        if (((self.math_class_field(self.cur_val1) > 7i32) || (self.math_fam_field(self.cur_val1) > 15i32)) || (self.math_char_field(self.cur_val1) > 255i32)) {
                                            {
                                                {
                                                    if (self.interaction == error_stop_mode) {
                                                    }
                                                    if self.file_line_error_style_p {
                                                        self.print_file_line();
                                                    } else {
                                                        self.print_nl(65544i32);
                                                    }
                                                    self.print(65994i32);
                                                }
                                                {
                                                    self.help_ptr = 2i32;
                                                    self.help_line[crate::ix::U((1i32) as usize)] = 65995i32;
                                                    self.help_line[crate::ix::U((0i32) as usize)] = 65996i32;
                                                }
                                                self.int_error(self.cur_val1);
                                                self.cur_val1 = 0i32;
                                            }
                                        }
                                    }
                                    self.cur_val1 = (((self.math_class_field(self.cur_val1)).wrapping_mul(4096i32)).wrapping_add((self.math_fam_field(self.cur_val1)).wrapping_mul(256i32))).wrapping_add(self.math_char_field(self.cur_val1));
                                    {
                                        self.cur_val = self.cur_val1;
                                        self.cur_val_level = int_val;
                                    }
                                }
                            } else {
                                if (m == del_code_base) {
                                    {
                                        self.cur_val1 = self.eqtb[crate::ix::U((((del_code_base).wrapping_add(self.cur_val)) - 1) as usize)].int();
                                        if (self.cur_val1 >= 1073741824i32) {
                                            {
                                                {
                                                    if (self.interaction == error_stop_mode) {
                                                    }
                                                    if self.file_line_error_style_p {
                                                        self.print_file_line();
                                                    } else {
                                                        self.print_nl(65544i32);
                                                    }
                                                    self.print(65997i32);
                                                }
                                                {
                                                    self.help_ptr = 2i32;
                                                    self.help_line[crate::ix::U((1i32) as usize)] = 65998i32;
                                                    self.help_line[crate::ix::U((0i32) as usize)] = 65996i32;
                                                }
                                                self.error();
                                                {
                                                    self.cur_val = 0i32;
                                                    self.cur_val_level = int_val;
                                                }
                                            }
                                        } else {
                                            {
                                                {
                                                    self.cur_val = self.cur_val1;
                                                    self.cur_val_level = int_val;
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    if (m < sf_code_base) {
                                        {
                                            self.cur_val = self.eqtb[crate::ix::U((((m).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                                            self.cur_val_level = int_val;
                                        }
                                    } else {
                                        if (m < math_code_base) {
                                            {
                                                self.cur_val = (self.eqtb[crate::ix::U((((m).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh() % 65536i32);
                                                self.cur_val_level = int_val;
                                            }
                                        } else {
                                            {
                                                self.cur_val = self.eqtb[crate::ix::U((((m).wrapping_add(self.cur_val)) - 1) as usize)].int();
                                                self.cur_val_level = int_val;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    XeTeX_def_code => {
                        // §447
                        {
                            self.scan_usv_num();
                            if (m == sf_code_base) {
                                {
                                    {
                                        self.cur_val = (self.eqtb[crate::ix::U((((sf_code_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh() / 65536i32);
                                        self.cur_val_level = int_val;
                                    }
                                }
                            } else {
                                if (m == math_code_base) {
                                    {
                                        {
                                            self.cur_val = self.eqtb[crate::ix::U((((math_code_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                                            self.cur_val_level = int_val;
                                        }
                                    }
                                } else {
                                    if (m == 5664041i32) {
                                        {
                                            {
                                                if (self.interaction == error_stop_mode) {
                                                }
                                                if self.file_line_error_style_p {
                                                    self.print_file_line();
                                                } else {
                                                    self.print_nl(65544i32);
                                                }
                                                self.print(65988i32);
                                            }
                                            {
                                                self.help_ptr = 2i32;
                                                self.help_line[crate::ix::U((1i32) as usize)] = 65989i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] = 65990i32;
                                            }
                                            self.error();
                                            {
                                                self.cur_val = 0i32;
                                                self.cur_val_level = int_val;
                                            }
                                        }
                                    } else {
                                        if (m == del_code_base) {
                                            {
                                                {
                                                    self.cur_val = self.eqtb[crate::ix::U((((del_code_base).wrapping_add(self.cur_val)) - 1) as usize)].int();
                                                    self.cur_val_level = int_val;
                                                }
                                            }
                                        } else {
                                            {
                                                {
                                                    if (self.interaction == error_stop_mode) {
                                                    }
                                                    if self.file_line_error_style_p {
                                                        self.print_file_line();
                                                    } else {
                                                        self.print_nl(65544i32);
                                                    }
                                                    self.print(65991i32);
                                                }
                                                {
                                                    self.help_ptr = 2i32;
                                                    self.help_line[crate::ix::U((1i32) as usize)] = 65992i32;
                                                    self.help_line[crate::ix::U((0i32) as usize)] = 65993i32;
                                                }
                                                self.error();
                                                {
                                                    self.cur_val = 0i32;
                                                    self.cur_val_level = int_val;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    toks_register | assign_toks | def_family | set_font | def_font => {
                        // §449
                        if (level != tok_val) {
                            {
                                {
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(65999i32);
                                }
                                {
                                    self.help_ptr = 3i32;
                                    self.help_line[crate::ix::U((2i32) as usize)] = 66000i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 66001i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66002i32;
                                }
                                self.back_error();
                                {
                                    self.cur_val = 0i32;
                                    self.cur_val_level = dimen_val;
                                }
                            }
                        } else {
                            if (self.cur_cmd <= assign_toks) {
                                {
                                    if (self.cur_cmd < assign_toks) {
                                        if (m == mem_bot) {
                                            {
                                                self.scan_register_num();
                                                if (self.cur_val < 256i32) {
                                                    self.cur_val = self.eqtb[crate::ix::U((((toks_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                                                } else {
                                                    {
                                                        self.find_sa_element(tok_val, self.cur_val, false);
                                                        if (self.cur_ptr == (268435455i32).wrapping_neg()) {
                                                            self.cur_val = (268435455i32).wrapping_neg();
                                                        } else {
                                                            self.cur_val = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            self.cur_val = self.mem[crate::ix::U(((m).wrapping_add(1i32)) as usize)].hh().rh();
                                        }
                                    } else {
                                        if (self.cur_chr == XeTeX_inter_char_loc) {
                                            {
                                                self.scan_char_class_not_ignored();
                                                self.cur_ptr = self.cur_val;
                                                self.scan_char_class_not_ignored();
                                                self.find_sa_element(inter_char_val, ((self.cur_ptr).wrapping_mul(char_class_limit)).wrapping_add(self.cur_val), false);
                                                if (self.cur_ptr == (268435455i32).wrapping_neg()) {
                                                    self.cur_val = (268435455i32).wrapping_neg();
                                                } else {
                                                    self.cur_val = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                                }
                                            }
                                        } else {
                                            self.cur_val = self.eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh();
                                        }
                                    }
                                    self.cur_val_level = tok_val;
                                }
                            } else {
                                {
                                    self.back_input();
                                    self.scan_font_ident();
                                    {
                                        self.cur_val = (font_id_base).wrapping_add(self.cur_val);
                                        self.cur_val_level = ident_val;
                                    }
                                }
                            }
                        }
                    }
                    assign_int => {
                        // §447
                        {
                            self.cur_val = self.eqtb[crate::ix::U(((m) - 1) as usize)].int();
                            self.cur_val_level = int_val;
                        }
                    }
                    assign_dimen => {
                        {
                            self.cur_val = self.eqtb[crate::ix::U(((m) - 1) as usize)].int();
                            self.cur_val_level = dimen_val;
                        }
                    }
                    assign_glue => {
                        {
                            self.cur_val = self.eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh();
                            self.cur_val_level = glue_val;
                        }
                    }
                    assign_mu_glue => {
                        {
                            self.cur_val = self.eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh();
                            self.cur_val_level = mu_val;
                        }
                    }
                    set_aux => {
                        // §452
                        if ((self.cur_list.mode_field).wrapping_abs() != m) {
                            {
                                {
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(66020i32);
                                }
                                self.print_cmd_chr(set_aux, m);
                                {
                                    self.help_ptr = 4i32;
                                    self.help_line[crate::ix::U((3i32) as usize)] = 66021i32;
                                    self.help_line[crate::ix::U((2i32) as usize)] = 66022i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 66023i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66024i32;
                                }
                                self.error();
                                if (level != tok_val) {
                                    {
                                        self.cur_val = 0i32;
                                        self.cur_val_level = dimen_val;
                                    }
                                } else {
                                    {
                                        self.cur_val = 0i32;
                                        self.cur_val_level = int_val;
                                    }
                                }
                            }
                        } else {
                            if (m == vmode) {
                                {
                                    self.cur_val = self.cur_list.aux_field.int();
                                    self.cur_val_level = dimen_val;
                                }
                            } else {
                                {
                                    self.cur_val = self.cur_list.aux_field.hh().lh();
                                    self.cur_val_level = int_val;
                                }
                            }
                        }
                    }
                    set_prev_graf => {
                        // §456
                        if (self.cur_list.mode_field == 0i32) {
                            {
                                self.cur_val = 0i32;
                                self.cur_val_level = int_val;
                            }
                        } else {
                            {
                                { let __ix287 = self.nest_ptr; let __v288 = self.cur_list; self.nest[crate::ix::U((__ix287) as usize)] = __v288; }
                                p = self.nest_ptr;
                                while ((self.nest[crate::ix::U((p) as usize)].mode_field).wrapping_abs() != vmode) {
                                    p = (p).wrapping_sub(1i32);
                                }
                                {
                                    self.cur_val = self.nest[crate::ix::U((p) as usize)].pg_field;
                                    self.cur_val_level = int_val;
                                }
                            }
                        }
                    }
                    set_page_int => {
                        // §453
                        {
                            if (m == 0i32) {
                                self.cur_val = self.dead_cycles;
                            } else {
                                // §1504
                                if (m == 2i32) {
                                    self.cur_val = self.interaction;
                                } else {
                                    // §453
                                    self.cur_val = self.insert_penalties;
                                }
                            }
                            self.cur_val_level = int_val;
                        }
                    }
                    set_page_dimen => {
                        // §455
                        {
                            if ((self.page_contents == empty) && (!self.output_active)) {
                                if (m == 0i32) {
                                    self.cur_val = max_dimen;
                                } else {
                                    self.cur_val = 0i32;
                                }
                            } else {
                                self.cur_val = self.page_so_far[crate::ix::U((m) as usize)];
                            }
                            self.cur_val_level = dimen_val;
                        }
                    }
                    set_shape => {
                        // §457
                        {
                            if (m > par_shape_loc) {
                                // §1677
                                {
                                    self.scan_int();
                                    if ((self.eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh() == (268435455i32).wrapping_neg()) || (self.cur_val < 0i32)) {
                                        self.cur_val = 0i32;
                                    } else {
                                        {
                                            if (self.cur_val > self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].int()) {
                                                self.cur_val = self.mem[crate::ix::U(((self.eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh()).wrapping_add(1i32)) as usize)].int();
                                            }
                                            self.cur_val = self.mem[crate::ix::U((((self.eqtb[crate::ix::U(((m) - 1) as usize)].hh().rh()).wrapping_add(self.cur_val)).wrapping_add(1i32)) as usize)].int();
                                        }
                                    }
                                }
                            } else {
                                // §457
                                if (self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh() == (268435455i32).wrapping_neg()) {
                                    self.cur_val = 0i32;
                                } else {
                                    self.cur_val = self.mem[crate::ix::U((self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()) as usize)].hh().lh();
                                }
                            }
                            self.cur_val_level = int_val;
                        }
                    }
                    set_box_dimen => {
                        // §454
                        {
                            self.scan_register_num();
                            if (self.cur_val < 256i32) {
                                q = self.eqtb[crate::ix::U((((box_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                            } else {
                                {
                                    self.find_sa_element(box_val, self.cur_val, false);
                                    if (self.cur_ptr == (268435455i32).wrapping_neg()) {
                                        q = (268435455i32).wrapping_neg();
                                    } else {
                                        q = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                    }
                                }
                            }
                            if (q == (268435455i32).wrapping_neg()) {
                                self.cur_val = 0i32;
                            } else {
                                self.cur_val = self.mem[crate::ix::U(((q).wrapping_add(m)) as usize)].int();
                            }
                            self.cur_val_level = dimen_val;
                        }
                    }
                    char_given | math_given | XeTeX_math_given => {
                        // §447
                        {
                            self.cur_val = self.cur_chr;
                            self.cur_val_level = int_val;
                        }
                    }
                    assign_font_dimen => {
                        // §459
                        {
                            self.find_font_dimen(false);
                            { let __ix289 = self.fmem_ptr; self.font_info[crate::ix::U((__ix289) as usize)].set_int(0i32); }
                            {
                                self.cur_val = self.font_info[crate::ix::U((self.cur_val) as usize)].int();
                                self.cur_val_level = dimen_val;
                            }
                        }
                    }
                    assign_font_int => {
                        // §460
                        {
                            self.scan_font_ident();
                            if (m == 0i32) {
                                {
                                    self.cur_val = self.hyphen_char[crate::ix::U((self.cur_val) as usize)];
                                    self.cur_val_level = int_val;
                                }
                            } else {
                                if (m == 1i32) {
                                    {
                                        self.cur_val = self.skew_char[crate::ix::U((self.cur_val) as usize)];
                                        self.cur_val_level = int_val;
                                    }
                                } else {
                                    {
                                        n = self.cur_val;
                                        if ((self.font_area[crate::ix::U((n) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((n) as usize)] == otgr_font_flag)) {
                                            self.scan_glyph_number(n);
                                        } else {
                                            self.scan_char_num();
                                        }
                                        k = self.cur_val;
                                        match m {
                                            lp_code_base => {
                                                {
                                                    self.cur_val = self.get_cp_code(n, k, left_side);
                                                    self.cur_val_level = int_val;
                                                }
                                            }
                                            rp_code_base => {
                                                {
                                                    self.cur_val = self.get_cp_code(n, k, right_side);
                                                    self.cur_val_level = int_val;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    register => {
                        // §461
                        {
                            if ((m < mem_bot) || (m > lo_mem_stat_max)) {
                                {
                                    self.cur_val_level = (self.mem[crate::ix::U((m) as usize)].hh().b0() / 64i32);
                                    if (self.cur_val_level < glue_val) {
                                        self.cur_val = self.mem[crate::ix::U(((m).wrapping_add(2i32)) as usize)].int();
                                    } else {
                                        self.cur_val = self.mem[crate::ix::U(((m).wrapping_add(1i32)) as usize)].hh().rh();
                                    }
                                }
                            } else {
                                {
                                    self.scan_register_num();
                                    self.cur_val_level = (m).wrapping_sub(0i32);
                                    if (self.cur_val > 255i32) {
                                        {
                                            self.find_sa_element(self.cur_val_level, self.cur_val, false);
                                            if (self.cur_ptr == (268435455i32).wrapping_neg()) {
                                                if (self.cur_val_level < glue_val) {
                                                    self.cur_val = 0i32;
                                                } else {
                                                    self.cur_val = zero_glue;
                                                }
                                            } else {
                                                if (self.cur_val_level < glue_val) {
                                                    self.cur_val = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(2i32)) as usize)].int();
                                                } else {
                                                    self.cur_val = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                                }
                                            }
                                        }
                                    } else {
                                        match self.cur_val_level {
                                            int_val => {
                                                self.cur_val = self.eqtb[crate::ix::U((((count_base).wrapping_add(self.cur_val)) - 1) as usize)].int();
                                            }
                                            dimen_val => {
                                                self.cur_val = self.eqtb[crate::ix::U((((scaled_base).wrapping_add(self.cur_val)) - 1) as usize)].int();
                                            }
                                            glue_val => {
                                                self.cur_val = self.eqtb[crate::ix::U((((skip_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                                            }
                                            mu_val => {
                                                self.cur_val = self.eqtb[crate::ix::U((((mu_skip_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    last_item => {
                        // §458
                        if (m >= input_line_no_code) {
                            if (m >= eTeX_glue) {
                                // §1591
                                {
                                    if (m < eTeX_mu) {
                                        {
                                            match m {
                                                mu_to_glue_code => {
                                                    // §1618
                                                    self.scan_mu_glue();
                                                }
                                                _ => {}
                                            }
                                            // §1591
                                            self.cur_val_level = glue_val;
                                        }
                                    } else {
                                        if (m < eTeX_expr) {
                                            {
                                                match m {
                                                    glue_to_mu_code => {
                                                        // §1619
                                                        self.scan_normal_glue();
                                                    }
                                                    _ => {}
                                                }
                                                // §1591
                                                self.cur_val_level = mu_val;
                                            }
                                        } else {
                                            {
                                                self.cur_val_level = (m).wrapping_sub(67i32);
                                                self.scan_expr();
                                            }
                                        }
                                    }
                                    while (self.cur_val_level > level) {
                                        {
                                            if (self.cur_val_level == glue_val) {
                                                {
                                                    m = self.cur_val;
                                                    self.cur_val = self.mem[crate::ix::U(((m).wrapping_add(1i32)) as usize)].int();
                                                    self.delete_glue_ref(m);
                                                }
                                            } else {
                                                if (self.cur_val_level == mu_val) {
                                                    self.mu_error();
                                                }
                                            }
                                            self.cur_val_level = (self.cur_val_level).wrapping_sub(1i32);
                                        }
                                    }
                                    if negative {
                                        if (self.cur_val_level >= glue_val) {
                                            {
                                                m = self.cur_val;
                                                self.cur_val = self.new_spec(m);
                                                self.delete_glue_ref(m);
                                                // §465
                                                {
                                                    { let __ix290 = (self.cur_val).wrapping_add(1i32); let __v291 = (self.mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int()).wrapping_neg(); self.mem[crate::ix::U((__ix290) as usize)].set_int(__v291); }
                                                    { let __ix292 = (self.cur_val).wrapping_add(2i32); let __v293 = (self.mem[crate::ix::U(((self.cur_val).wrapping_add(2i32)) as usize)].int()).wrapping_neg(); self.mem[crate::ix::U((__ix292) as usize)].set_int(__v293); }
                                                    { let __ix294 = (self.cur_val).wrapping_add(3i32); let __v295 = (self.mem[crate::ix::U(((self.cur_val).wrapping_add(3i32)) as usize)].int()).wrapping_neg(); self.mem[crate::ix::U((__ix294) as usize)].set_int(__v295); }
                                                }
                                            }
                                        } else {
                                            // §1591
                                            self.cur_val = (self.cur_val).wrapping_neg();
                                        }
                                    }
                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                }
                            } else {
                                // §458
                                if (m >= XeTeX_dim) {
                                    {
                                        match m {
                                            XeTeX_glyph_bounds_code => {
                                                // §1458
                                                {
                                                    if ((self.font_area[crate::ix::U((self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)] == otgr_font_flag)) {
                                                        {
                                                            self.scan_int();
                                                            n = self.cur_val;
                                                            if ((n < 1i32) || (n > 4i32)) {
                                                                {
                                                                    {
                                                                        if (self.interaction == error_stop_mode) {
                                                                        }
                                                                        if self.file_line_error_style_p {
                                                                            self.print_file_line();
                                                                        } else {
                                                                            self.print_nl(65544i32);
                                                                        }
                                                                        self.print(66846i32);
                                                                    }
                                                                    self.print_nl(66847i32);
                                                                    self.print_int(n);
                                                                    self.error();
                                                                    self.cur_val = 0i32;
                                                                }
                                                            } else {
                                                                {
                                                                    self.scan_int();
                                                                    self.cur_val = self.get_glyph_bounds(self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh(), n, self.cur_val);
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        {
                                                            self.not_native_font_error(last_item, m, self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh());
                                                            self.cur_val = 0i32;
                                                        }
                                                    }
                                                }
                                            }
                                            font_char_wd_code | font_char_ht_code | font_char_dp_code | font_char_ic_code => {
                                                // §1481
                                                {
                                                    self.scan_font_ident();
                                                    q = self.cur_val;
                                                    self.scan_usv_num();
                                                    if ((self.font_area[crate::ix::U((q) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((q) as usize)] == otgr_font_flag)) {
                                                        {
                                                            match m {
                                                                font_char_wd_code => {
                                                                    self.cur_val = self.getnativecharwd(q, self.cur_val);
                                                                }
                                                                font_char_ht_code => {
                                                                    self.cur_val = self.getnativecharht(q, self.cur_val);
                                                                }
                                                                font_char_dp_code => {
                                                                    self.cur_val = self.getnativechardp(q, self.cur_val);
                                                                }
                                                                font_char_ic_code => {
                                                                    self.cur_val = self.getnativecharic(q, self.cur_val);
                                                                }
                                                                _ => {}
                                                            }
                                                        }
                                                    } else {
                                                        {
                                                            if ((self.font_bc[crate::ix::U((q) as usize)] <= self.cur_val) && (self.font_ec[crate::ix::U((q) as usize)] >= self.cur_val)) {
                                                                {
                                                                    i = { let __s296 = ((self.char_base[crate::ix::U((q) as usize)]).wrapping_add(self.effective_char(true, q, self.cur_val))) as usize; self.font_info[crate::ix::U(__s296)] }.qqqq();
                                                                    match m {
                                                                        font_char_wd_code => {
                                                                            self.cur_val = self.font_info[crate::ix::U(((self.width_base[crate::ix::U((q) as usize)]).wrapping_add(i.b0())) as usize)].int();
                                                                        }
                                                                        font_char_ht_code => {
                                                                            self.cur_val = self.font_info[crate::ix::U(((self.height_base[crate::ix::U((q) as usize)]).wrapping_add((i.b1() / 16i32))) as usize)].int();
                                                                        }
                                                                        font_char_dp_code => {
                                                                            self.cur_val = self.font_info[crate::ix::U(((self.depth_base[crate::ix::U((q) as usize)]).wrapping_add((i.b1() % 16i32))) as usize)].int();
                                                                        }
                                                                        font_char_ic_code => {
                                                                            self.cur_val = self.font_info[crate::ix::U(((self.italic_base[crate::ix::U((q) as usize)]).wrapping_add((i.b2() / 4i32))) as usize)].int();
                                                                        }
                                                                        _ => {}
                                                                    }
                                                                }
                                                            } else {
                                                                self.cur_val = 0i32;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            par_shape_length_code | par_shape_indent_code | par_shape_dimen_code => {
                                                // §1484
                                                {
                                                    q = (self.cur_chr).wrapping_sub(60i32);
                                                    self.scan_int();
                                                    if ((self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh() == (268435455i32).wrapping_neg()) || (self.cur_val <= 0i32)) {
                                                        self.cur_val = 0i32;
                                                    } else {
                                                        {
                                                            if (q == 2i32) {
                                                                {
                                                                    q = (self.cur_val % 2i32);
                                                                    self.cur_val = ((self.cur_val).wrapping_add(q) / 2i32);
                                                                }
                                                            }
                                                            if (self.cur_val > self.mem[crate::ix::U((self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()) as usize)].hh().lh()) {
                                                                self.cur_val = self.mem[crate::ix::U((self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()) as usize)].hh().lh();
                                                            }
                                                            self.cur_val = self.mem[crate::ix::U((((self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].hh().rh()).wrapping_add((2i32).wrapping_mul(self.cur_val))).wrapping_sub(q)) as usize)].int();
                                                        }
                                                    }
                                                    self.cur_val_level = dimen_val;
                                                }
                                            }
                                            glue_stretch_code | glue_shrink_code => {
                                                // §1615
                                                {
                                                    self.scan_normal_glue();
                                                    q = self.cur_val;
                                                    if (m == glue_stretch_code) {
                                                        self.cur_val = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int();
                                                    } else {
                                                        self.cur_val = self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int();
                                                    }
                                                    self.delete_glue_ref(q);
                                                }
                                            }
                                            _ => {}
                                        }
                                        // §458
                                        self.cur_val_level = dimen_val;
                                    }
                                } else {
                                    {
                                        match m {
                                            input_line_no_code => {
                                                self.cur_val = self.line;
                                            }
                                            badness_code => {
                                                self.cur_val = self.last_badness;
                                            }
                                            elapsed_time_code => {
                                                self.cur_val = self.get_microinterval();
                                            }
                                            random_seed_code => {
                                                self.cur_val = self.random_seed;
                                            }
                                            pdf_shell_escape_code => {
                                                {
                                                    if self.shellenabledp {
                                                        {
                                                            if self.restrictedshell {
                                                                self.cur_val = 2i32;
                                                            } else {
                                                                self.cur_val = 1i32;
                                                            }
                                                        }
                                                    } else {
                                                        self.cur_val = 0i32;
                                                    }
                                                }
                                            }
                                            eTeX_version_code => {
                                                // §1454
                                                self.cur_val = eTeX_version;
                                            }
                                            XeTeX_version_code => {
                                                self.cur_val = XeTeX_version;
                                            }
                                            XeTeX_count_glyphs_code => {
                                                {
                                                    self.scan_font_ident();
                                                    n = self.cur_val;
                                                    if (self.font_area[crate::ix::U((n) as usize)] == aat_font_flag) {
                                                        self.cur_val = self.aat_font_get((m).wrapping_sub(27i32), self.font_layout_engine[crate::ix::U((n) as usize)]);
                                                    } else {
                                                        if (self.font_area[crate::ix::U((n) as usize)] == otgr_font_flag) {
                                                            self.cur_val = self.ot_font_get((m).wrapping_sub(27i32), self.font_layout_engine[crate::ix::U((n) as usize)]);
                                                        } else {
                                                            self.cur_val = 0i32;
                                                        }
                                                    }
                                                }
                                            }
                                            XeTeX_count_features_code => {
                                                {
                                                    self.scan_font_ident();
                                                    n = self.cur_val;
                                                    if (self.font_area[crate::ix::U((n) as usize)] == aat_font_flag) {
                                                        self.cur_val = self.aat_font_get((m).wrapping_sub(27i32), self.font_layout_engine[crate::ix::U((n) as usize)]);
                                                    } else {
                                                        if ((self.font_area[crate::ix::U((n) as usize)] == otgr_font_flag) && self.usingGraphite(self.font_layout_engine[crate::ix::U((n) as usize)])) {
                                                            self.cur_val = self.ot_font_get((m).wrapping_sub(27i32), self.font_layout_engine[crate::ix::U((n) as usize)]);
                                                        } else {
                                                            self.cur_val = 0i32;
                                                        }
                                                    }
                                                }
                                            }
                                            XeTeX_variation_code | XeTeX_variation_min_code | XeTeX_variation_max_code | XeTeX_variation_default_code | XeTeX_count_variations_code => {
                                                {
                                                    self.scan_font_ident();
                                                    n = self.cur_val;
                                                    self.cur_val = 0i32;
                                                }
                                            }
                                            XeTeX_feature_code_code | XeTeX_is_exclusive_feature_code | XeTeX_count_selectors_code => {
                                                {
                                                    self.scan_font_ident();
                                                    n = self.cur_val;
                                                    if (self.font_area[crate::ix::U((n) as usize)] == aat_font_flag) {
                                                        {
                                                            self.scan_int();
                                                            k = self.cur_val;
                                                            self.cur_val = self.aat_font_get_1((m).wrapping_sub(27i32), self.font_layout_engine[crate::ix::U((n) as usize)], k);
                                                        }
                                                    } else {
                                                        if ((self.font_area[crate::ix::U((n) as usize)] == otgr_font_flag) && self.usingGraphite(self.font_layout_engine[crate::ix::U((n) as usize)])) {
                                                            {
                                                                self.scan_int();
                                                                k = self.cur_val;
                                                                self.cur_val = self.ot_font_get_1((m).wrapping_sub(27i32), self.font_layout_engine[crate::ix::U((n) as usize)], k);
                                                            }
                                                        } else {
                                                            {
                                                                self.not_aat_gr_font_error(last_item, m, n);
                                                                self.cur_val = (1i32).wrapping_neg();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            XeTeX_selector_code_code | XeTeX_is_default_selector_code => {
                                                {
                                                    self.scan_font_ident();
                                                    n = self.cur_val;
                                                    if (self.font_area[crate::ix::U((n) as usize)] == aat_font_flag) {
                                                        {
                                                            self.scan_int();
                                                            k = self.cur_val;
                                                            self.scan_int();
                                                            self.cur_val = self.aat_font_get_2((m).wrapping_sub(27i32), self.font_layout_engine[crate::ix::U((n) as usize)], k, self.cur_val);
                                                        }
                                                    } else {
                                                        if ((self.font_area[crate::ix::U((n) as usize)] == otgr_font_flag) && self.usingGraphite(self.font_layout_engine[crate::ix::U((n) as usize)])) {
                                                            {
                                                                self.scan_int();
                                                                k = self.cur_val;
                                                                self.scan_int();
                                                                self.cur_val = self.ot_font_get_2((m).wrapping_sub(27i32), self.font_layout_engine[crate::ix::U((n) as usize)], k, self.cur_val);
                                                            }
                                                        } else {
                                                            {
                                                                self.not_aat_gr_font_error(last_item, m, n);
                                                                self.cur_val = (1i32).wrapping_neg();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            XeTeX_find_variation_by_name_code => {
                                                {
                                                    self.scan_font_ident();
                                                    n = self.cur_val;
                                                    if (self.font_area[crate::ix::U((n) as usize)] == aat_font_flag) {
                                                        {
                                                            self.scan_and_pack_name();
                                                            self.cur_val = self.aat_font_get_named((m).wrapping_sub(27i32), self.font_layout_engine[crate::ix::U((n) as usize)]);
                                                        }
                                                    } else {
                                                        {
                                                            self.not_aat_font_error(last_item, m, n);
                                                            self.cur_val = (1i32).wrapping_neg();
                                                        }
                                                    }
                                                }
                                            }
                                            XeTeX_find_feature_by_name_code => {
                                                {
                                                    self.scan_font_ident();
                                                    n = self.cur_val;
                                                    if (self.font_area[crate::ix::U((n) as usize)] == aat_font_flag) {
                                                        {
                                                            self.scan_and_pack_name();
                                                            self.cur_val = self.aat_font_get_named((m).wrapping_sub(27i32), self.font_layout_engine[crate::ix::U((n) as usize)]);
                                                        }
                                                    } else {
                                                        if ((self.font_area[crate::ix::U((n) as usize)] == otgr_font_flag) && self.usingGraphite(self.font_layout_engine[crate::ix::U((n) as usize)])) {
                                                            {
                                                                self.scan_and_pack_name();
                                                                self.cur_val = self.gr_font_get_named((m).wrapping_sub(27i32), self.font_layout_engine[crate::ix::U((n) as usize)]);
                                                            }
                                                        } else {
                                                            {
                                                                self.not_aat_gr_font_error(last_item, m, n);
                                                                self.cur_val = (1i32).wrapping_neg();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            XeTeX_find_selector_by_name_code => {
                                                {
                                                    self.scan_font_ident();
                                                    n = self.cur_val;
                                                    if (self.font_area[crate::ix::U((n) as usize)] == aat_font_flag) {
                                                        {
                                                            self.scan_int();
                                                            k = self.cur_val;
                                                            self.scan_and_pack_name();
                                                            self.cur_val = self.aat_font_get_named_1((m).wrapping_sub(27i32), self.font_layout_engine[crate::ix::U((n) as usize)], k);
                                                        }
                                                    } else {
                                                        if ((self.font_area[crate::ix::U((n) as usize)] == otgr_font_flag) && self.usingGraphite(self.font_layout_engine[crate::ix::U((n) as usize)])) {
                                                            {
                                                                self.scan_int();
                                                                k = self.cur_val;
                                                                self.scan_and_pack_name();
                                                                self.cur_val = self.gr_font_get_named_1((m).wrapping_sub(27i32), self.font_layout_engine[crate::ix::U((n) as usize)], k);
                                                            }
                                                        } else {
                                                            {
                                                                self.not_aat_gr_font_error(last_item, m, n);
                                                                self.cur_val = (1i32).wrapping_neg();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            XeTeX_OT_count_scripts_code => {
                                                {
                                                    self.scan_font_ident();
                                                    n = self.cur_val;
                                                    if ((self.font_area[crate::ix::U((n) as usize)] == otgr_font_flag) && self.usingOpenType(self.font_layout_engine[crate::ix::U((n) as usize)])) {
                                                        self.cur_val = self.ot_font_get((m).wrapping_sub(27i32), self.font_layout_engine[crate::ix::U((n) as usize)]);
                                                    } else {
                                                        {
                                                            self.cur_val = 0i32;
                                                        }
                                                    }
                                                }
                                            }
                                            XeTeX_OT_count_languages_code | XeTeX_OT_script_code => {
                                                {
                                                    self.scan_font_ident();
                                                    n = self.cur_val;
                                                    if ((self.font_area[crate::ix::U((n) as usize)] == otgr_font_flag) && self.usingOpenType(self.font_layout_engine[crate::ix::U((n) as usize)])) {
                                                        {
                                                            self.scan_int();
                                                            self.cur_val = self.ot_font_get_1((m).wrapping_sub(27i32), self.font_layout_engine[crate::ix::U((n) as usize)], self.cur_val);
                                                        }
                                                    } else {
                                                        {
                                                            self.not_ot_font_error(last_item, m, n);
                                                            self.cur_val = (1i32).wrapping_neg();
                                                        }
                                                    }
                                                }
                                            }
                                            XeTeX_OT_count_features_code | XeTeX_OT_language_code => {
                                                {
                                                    self.scan_font_ident();
                                                    n = self.cur_val;
                                                    if ((self.font_area[crate::ix::U((n) as usize)] == otgr_font_flag) && self.usingOpenType(self.font_layout_engine[crate::ix::U((n) as usize)])) {
                                                        {
                                                            self.scan_int();
                                                            k = self.cur_val;
                                                            self.scan_int();
                                                            self.cur_val = self.ot_font_get_2((m).wrapping_sub(27i32), self.font_layout_engine[crate::ix::U((n) as usize)], k, self.cur_val);
                                                        }
                                                    } else {
                                                        {
                                                            self.not_ot_font_error(last_item, m, n);
                                                            self.cur_val = (1i32).wrapping_neg();
                                                        }
                                                    }
                                                }
                                            }
                                            XeTeX_OT_feature_code => {
                                                {
                                                    self.scan_font_ident();
                                                    n = self.cur_val;
                                                    if ((self.font_area[crate::ix::U((n) as usize)] == otgr_font_flag) && self.usingOpenType(self.font_layout_engine[crate::ix::U((n) as usize)])) {
                                                        {
                                                            self.scan_int();
                                                            k = self.cur_val;
                                                            self.scan_int();
                                                            kk = self.cur_val;
                                                            self.scan_int();
                                                            self.cur_val = self.ot_font_get_3((m).wrapping_sub(27i32), self.font_layout_engine[crate::ix::U((n) as usize)], k, kk, self.cur_val);
                                                        }
                                                    } else {
                                                        {
                                                            self.not_ot_font_error(last_item, m, n);
                                                            self.cur_val = (1i32).wrapping_neg();
                                                        }
                                                    }
                                                }
                                            }
                                            XeTeX_map_char_to_glyph_code => {
                                                {
                                                    if ((self.font_area[crate::ix::U((self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)] == otgr_font_flag)) {
                                                        {
                                                            self.scan_int();
                                                            n = self.cur_val;
                                                            self.cur_val = self.map_char_to_glyph(self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh(), n);
                                                        }
                                                    } else {
                                                        {
                                                            self.not_native_font_error(last_item, m, self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh());
                                                            self.cur_val = 0i32;
                                                        }
                                                    }
                                                }
                                            }
                                            XeTeX_glyph_index_code => {
                                                {
                                                    if ((self.font_area[crate::ix::U((self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)] == otgr_font_flag)) {
                                                        {
                                                            self.scan_and_pack_name();
                                                            self.cur_val = self.map_glyph_to_index(self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh());
                                                        }
                                                    } else {
                                                        {
                                                            self.not_native_font_error(last_item, m, self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh());
                                                            self.cur_val = 0i32;
                                                        }
                                                    }
                                                }
                                            }
                                            XeTeX_font_type_code => {
                                                {
                                                    self.scan_font_ident();
                                                    n = self.cur_val;
                                                    if (self.font_area[crate::ix::U((n) as usize)] == aat_font_flag) {
                                                        self.cur_val = 1i32;
                                                    } else {
                                                        if ((self.font_area[crate::ix::U((n) as usize)] == otgr_font_flag) && self.usingOpenType(self.font_layout_engine[crate::ix::U((n) as usize)])) {
                                                            self.cur_val = 2i32;
                                                        } else {
                                                            if ((self.font_area[crate::ix::U((n) as usize)] == otgr_font_flag) && self.usingGraphite(self.font_layout_engine[crate::ix::U((n) as usize)])) {
                                                                self.cur_val = 3i32;
                                                            } else {
                                                                self.cur_val = 0i32;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            XeTeX_first_char_code | XeTeX_last_char_code => {
                                                {
                                                    self.scan_font_ident();
                                                    n = self.cur_val;
                                                    if ((self.font_area[crate::ix::U((n) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((n) as usize)] == otgr_font_flag)) {
                                                        self.cur_val = self.get_font_char_range(n, (m == XeTeX_first_char_code));
                                                    } else {
                                                        {
                                                            if (m == XeTeX_first_char_code) {
                                                                self.cur_val = self.font_bc[crate::ix::U((n) as usize)];
                                                            } else {
                                                                self.cur_val = self.font_ec[crate::ix::U((n) as usize)];
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            pdf_last_x_pos_code => {
                                                self.cur_val = self.pdf_last_x_pos;
                                            }
                                            pdf_last_y_pos_code => {
                                                self.cur_val = self.pdf_last_y_pos;
                                            }
                                            XeTeX_pdf_page_count_code => {
                                                {
                                                    self.scan_and_pack_name();
                                                    self.cur_val = self.count_pdf_file_pages();
                                                }
                                            }
                                            current_group_level_code => {
                                                // §1475
                                                self.cur_val = (self.cur_level).wrapping_sub(1i32);
                                            }
                                            current_group_type_code => {
                                                self.cur_val = self.cur_group;
                                            }
                                            current_if_level_code => {
                                                // §1478
                                                {
                                                    q = self.cond_ptr;
                                                    self.cur_val = 0i32;
                                                    while (q != (268435455i32).wrapping_neg()) {
                                                        {
                                                            self.cur_val = (self.cur_val).wrapping_add(1i32);
                                                            q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                        }
                                                    }
                                                }
                                            }
                                            current_if_type_code => {
                                                if (self.cond_ptr == (268435455i32).wrapping_neg()) {
                                                    self.cur_val = 0i32;
                                                } else {
                                                    if (self.cur_if < unless_code) {
                                                        self.cur_val = (self.cur_if).wrapping_add(1i32);
                                                    } else {
                                                        self.cur_val = ((self.cur_if).wrapping_sub(31i32)).wrapping_neg();
                                                    }
                                                }
                                            }
                                            current_if_branch_code => {
                                                if ((self.if_limit == or_code) || (self.if_limit == else_code)) {
                                                    self.cur_val = 1i32;
                                                } else {
                                                    if (self.if_limit == fi_code) {
                                                        self.cur_val = (1i32).wrapping_neg();
                                                    } else {
                                                        self.cur_val = 0i32;
                                                    }
                                                }
                                            }
                                            glue_stretch_order_code | glue_shrink_order_code => {
                                                // §1614
                                                {
                                                    self.scan_normal_glue();
                                                    q = self.cur_val;
                                                    if (m == glue_stretch_order_code) {
                                                        self.cur_val = self.mem[crate::ix::U((q) as usize)].hh().b0();
                                                    } else {
                                                        self.cur_val = self.mem[crate::ix::U((q) as usize)].hh().b1();
                                                    }
                                                    self.delete_glue_ref(q);
                                                }
                                            }
                                            _ => {}
                                        }
                                        // §458
                                        self.cur_val_level = int_val;
                                    }
                                }
                            }
                        } else {
                            {
                                if (self.cur_chr == glue_val) {
                                    self.cur_val = zero_glue;
                                } else {
                                    self.cur_val = 0i32;
                                }
                                tx = self.cur_list.tail_field;
                                if (!(tx >= self.hi_mem_min)) {
                                    if ((self.mem[crate::ix::U((tx) as usize)].hh().b0() == math_node) && (self.mem[crate::ix::U((tx) as usize)].hh().b1() == end_M_code)) {
                                        {
                                            r = self.cur_list.head_field;
                                            loop {
                                                q = r;
                                                r = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                if (r == tx) { break; }
                                            }
                                            tx = q;
                                        }
                                    }
                                }
                                if (self.cur_chr == last_node_type_code) {
                                    {
                                        self.cur_val_level = int_val;
                                        if ((tx == self.cur_list.head_field) || (self.cur_list.mode_field == 0i32)) {
                                            self.cur_val = (1i32).wrapping_neg();
                                        }
                                    }
                                } else {
                                    self.cur_val_level = self.cur_chr;
                                }
                                if ((!(tx >= self.hi_mem_min)) && (self.cur_list.mode_field != 0i32)) {
                                    match self.cur_chr {
                                        int_val => {
                                            if (self.mem[crate::ix::U((tx) as usize)].hh().b0() == penalty_node) {
                                                self.cur_val = self.mem[crate::ix::U(((tx).wrapping_add(1i32)) as usize)].int();
                                            }
                                        }
                                        dimen_val => {
                                            if (self.mem[crate::ix::U((tx) as usize)].hh().b0() == kern_node) {
                                                self.cur_val = self.mem[crate::ix::U(((tx).wrapping_add(1i32)) as usize)].int();
                                            }
                                        }
                                        glue_val => {
                                            if (self.mem[crate::ix::U((tx) as usize)].hh().b0() == glue_node) {
                                                {
                                                    self.cur_val = self.mem[crate::ix::U(((tx).wrapping_add(1i32)) as usize)].hh().lh();
                                                    if (self.mem[crate::ix::U((tx) as usize)].hh().b1() == mu_glue) {
                                                        self.cur_val_level = mu_val;
                                                    }
                                                }
                                            }
                                        }
                                        last_node_type_code => {
                                            if (self.mem[crate::ix::U((tx) as usize)].hh().b0() <= unset_node) {
                                                self.cur_val = (self.mem[crate::ix::U((tx) as usize)].hh().b0()).wrapping_add(1i32);
                                            } else {
                                                self.cur_val = 15i32;
                                            }
                                        }
                                        _ => {}
                                    }
                                } else {
                                    if ((self.cur_list.mode_field == vmode) && (tx == self.cur_list.head_field)) {
                                        match self.cur_chr {
                                            int_val => {
                                                self.cur_val = self.last_penalty;
                                            }
                                            dimen_val => {
                                                self.cur_val = self.last_kern;
                                            }
                                            glue_val => {
                                                if (self.last_glue != max_halfword) {
                                                    self.cur_val = self.last_glue;
                                                }
                                            }
                                            last_node_type_code => {
                                                self.cur_val = self.last_node_type;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    ignore_spaces => {
                        // §447
                        if (self.cur_chr == 1i32) {
                            // §403
                            {
                                self.get_token();
                                if (self.cur_cs < hash_base) {
                                    self.cur_cs = self.prim_lookup((self.cur_cs).wrapping_sub(1114113i32));
                                } else {
                                    self.cur_cs = self.prim_lookup(self.hash[crate::ix::U(((self.cur_cs) - 1179650) as usize)].rh());
                                }
                                if (self.cur_cs != undefined_primitive) {
                                    {
                                        self.cur_cmd = self.eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(self.cur_cs)) - 1) as usize)].hh().b0();
                                        self.cur_chr = self.eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(self.cur_cs)) - 1) as usize)].hh().rh();
                                        self.cur_cs = (prim_eqtb_base).wrapping_add(self.cur_cs);
                                        self.cur_tok = (cs_token_flag).wrapping_add(self.cur_cs);
                                    }
                                } else {
                                    {
                                        self.cur_cmd = relax;
                                        self.cur_chr = 0i32;
                                        self.cur_tok = 34749088i32;
                                        self.cur_cs = frozen_relax;
                                    }
                                }
                                { __goto_1 = 0; continue 'l_dispatch_1; }
                            }
                        }
                    }
                    _ => {
                        // §462
                        {
                            {
                                if (self.interaction == error_stop_mode) {
                                }
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(65544i32);
                                }
                                self.print(66025i32);
                            }
                            self.print_cmd_chr(self.cur_cmd, self.cur_chr);
                            self.print(66026i32);
                            self.print_esc(65853i32);
                            {
                                self.help_ptr = 1i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 66024i32;
                            }
                            self.error();
                            if (level != tok_val) {
                                {
                                    self.cur_val = 0i32;
                                    self.cur_val_level = dimen_val;
                                }
                            } else {
                                {
                                    self.cur_val = 0i32;
                                    self.cur_val_level = int_val;
                                }
                            }
                        }
                    }
                }
                // §447
                while (self.cur_val_level > level) {
                    // §463
                    {
                        if (self.cur_val_level == glue_val) {
                            self.cur_val = self.mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int();
                        } else {
                            if (self.cur_val_level == mu_val) {
                                self.mu_error();
                            }
                        }
                        self.cur_val_level = (self.cur_val_level).wrapping_sub(1i32);
                    }
                }
                // §464
                if negative {
                    if (self.cur_val_level >= glue_val) {
                        {
                            self.cur_val = self.new_spec(self.cur_val);
                            // §465
                            {
                                { let __ix297 = (self.cur_val).wrapping_add(1i32); let __v298 = (self.mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int()).wrapping_neg(); self.mem[crate::ix::U((__ix297) as usize)].set_int(__v298); }
                                { let __ix299 = (self.cur_val).wrapping_add(2i32); let __v300 = (self.mem[crate::ix::U(((self.cur_val).wrapping_add(2i32)) as usize)].int()).wrapping_neg(); self.mem[crate::ix::U((__ix299) as usize)].set_int(__v300); }
                                { let __ix301 = (self.cur_val).wrapping_add(3i32); let __v302 = (self.mem[crate::ix::U(((self.cur_val).wrapping_add(3i32)) as usize)].int()).wrapping_neg(); self.mem[crate::ix::U((__ix301) as usize)].set_int(__v302); }
                            }
                        }
                    } else {
                        // §464
                        self.cur_val = (self.cur_val).wrapping_neg();
                    }
                } else {
                    if ((self.cur_val_level >= glue_val) && (self.cur_val_level <= mu_val)) {
                        { let __ix303 = self.cur_val; let __v304 = (self.mem[crate::ix::U((self.cur_val) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((__ix303) as usize)].set_hh_rh(__v304); }
                    }
                }
            }
            if __goto_1 <= 1 { // exit
                // §447
            }
            break 'l_dispatch_1;
        }
    }

    /// The `scan_int` routine is used also to scan the integer part of a
    /// fraction; for example, the `\.3' in `\.{3.14159}' will be found by
    /// `scan_int`. The `scan_dimen` routine assumes that `cur_tok=point_token`
    /// after the integer part of such a fraction has been scanned by `scan_int`,
    /// and that the decimal point has been backed up to be scanned again.
    // §474
    pub fn scan_int(&mut self) {
        let mut negative: bool = false; // §474
        let mut m: i32 = 0; // §474
        let mut d: small_number = 0; // §474
        let mut vacuous: bool = false; // §474
        let mut OK_so_far: bool = false; // §474
        self.radix = 0i32;
        OK_so_far = true;
        // §475
        negative = false;
        loop {
            // §440
            loop {
                self.get_x_token();
                if (self.cur_cmd != spacer) { break; }
            }
            // §475
            if (self.cur_tok == 25165869i32) {
                {
                    negative = (!negative);
                    self.cur_tok = 25165867i32;
                }
            }
            if (self.cur_tok != 25165867i32) { break; }
        }
        'l_restart_b: loop {
            // §474
            if (self.cur_tok == alpha_token) {
                // §476
                {
                    self.get_token();
                    if (self.cur_tok < cs_token_flag) {
                        {
                            self.cur_val = self.cur_chr;
                            if (self.cur_cmd <= right_brace) {
                                if (self.cur_cmd == right_brace) {
                                    self.align_state = (self.align_state).wrapping_add(1i32);
                                } else {
                                    self.align_state = (self.align_state).wrapping_sub(1i32);
                                }
                            }
                        }
                    } else {
                        if (self.cur_tok < 34668544i32) {
                            self.cur_val = (self.cur_tok).wrapping_sub(33554432i32);
                        } else {
                            if (self.cur_tok < 34734080i32) {
                                self.cur_val = (self.cur_tok).wrapping_sub(34668544i32);
                            } else {
                                {
                                    m = self.hash[crate::ix::U((((self.cur_tok).wrapping_sub(33554431i32)) - 1179650) as usize)].rh();
                                    if (self.str_start[crate::ix::U((((m).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)] == (2i32).wrapping_add(self.str_start[crate::ix::U(((m).wrapping_sub(65536i32)) as usize)])) {
                                        {
                                            m = self.str_start[crate::ix::U(((m).wrapping_sub(65536i32)) as usize)];
                                            if ((((self.str_pool[crate::ix::U((m) as usize)] >= 55296i32) && (self.str_pool[crate::ix::U((m) as usize)] <= 56319i32)) && (self.str_pool[crate::ix::U(((m).wrapping_add(1i32)) as usize)] >= 56320i32)) && (self.str_pool[crate::ix::U(((m).wrapping_add(1i32)) as usize)] <= 57343i32)) {
                                                self.cur_val = (((65536i32).wrapping_add(((self.str_pool[crate::ix::U((m) as usize)]).wrapping_sub(55296i32)).wrapping_mul(1024i32))).wrapping_add(self.str_pool[crate::ix::U(((m).wrapping_add(1i32)) as usize)])).wrapping_sub(56320i32);
                                            } else {
                                                self.cur_val = too_big_usv;
                                            }
                                        }
                                    } else {
                                        self.cur_val = too_big_usv;
                                    }
                                }
                            }
                        }
                    }
                    if (self.cur_val > biggest_usv) {
                        {
                            {
                                if (self.interaction == error_stop_mode) {
                                }
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(65544i32);
                                }
                                self.print(66050i32);
                            }
                            {
                                self.help_ptr = 2i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 66051i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 66052i32;
                            }
                            self.cur_val = 48i32;
                            self.back_error();
                        }
                    } else {
                        // §477
                        {
                            self.get_x_token();
                            if (self.cur_cmd != spacer) {
                                self.back_input();
                            }
                        }
                    }
                }
            } else {
                // §474
                if (self.cur_tok == 34749092i32) {
                    // §403
                    {
                        self.get_token();
                        if (self.cur_cs < hash_base) {
                            self.cur_cs = self.prim_lookup((self.cur_cs).wrapping_sub(1114113i32));
                        } else {
                            self.cur_cs = self.prim_lookup(self.hash[crate::ix::U(((self.cur_cs) - 1179650) as usize)].rh());
                        }
                        if (self.cur_cs != undefined_primitive) {
                            {
                                self.cur_cmd = self.eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(self.cur_cs)) - 1) as usize)].hh().b0();
                                self.cur_chr = self.eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(self.cur_cs)) - 1) as usize)].hh().rh();
                                self.cur_cs = (prim_eqtb_base).wrapping_add(self.cur_cs);
                                self.cur_tok = (cs_token_flag).wrapping_add(self.cur_cs);
                            }
                        } else {
                            {
                                self.cur_cmd = relax;
                                self.cur_chr = 0i32;
                                self.cur_tok = 34749088i32;
                                self.cur_cs = frozen_relax;
                            }
                        }
                        continue 'l_restart_b;
                    }
                } else {
                    // §474
                    if ((self.cur_cmd >= min_internal) && (self.cur_cmd <= max_internal)) {
                        self.scan_something_internal(int_val, false);
                    } else {
                        // §478
                        {
                            'l_done_f: {
                                self.radix = 10i32;
                                m = 214748364i32;
                                if (self.cur_tok == octal_token) {
                                    {
                                        self.radix = 8i32;
                                        m = 268435456i32;
                                        self.get_x_token();
                                    }
                                } else {
                                    if (self.cur_tok == hex_token) {
                                        {
                                            self.radix = 16i32;
                                            m = 134217728i32;
                                            self.get_x_token();
                                        }
                                    }
                                }
                                vacuous = true;
                                self.cur_val = 0i32;
                                // §479
                                while true {
                                    {
                                        if (((self.cur_tok < (zero_token).wrapping_add(self.radix)) && (self.cur_tok >= zero_token)) && (self.cur_tok <= 25165881i32)) {
                                            d = (self.cur_tok).wrapping_sub(25165872i32);
                                        } else {
                                            if (self.radix == 16i32) {
                                                if ((self.cur_tok <= 23068742i32) && (self.cur_tok >= A_token)) {
                                                    d = (self.cur_tok).wrapping_sub(23068727i32);
                                                } else {
                                                    if ((self.cur_tok <= 25165894i32) && (self.cur_tok >= other_A_token)) {
                                                        d = (self.cur_tok).wrapping_sub(25165879i32);
                                                    } else {
                                                        break 'l_done_f;
                                                    }
                                                }
                                            } else {
                                                break 'l_done_f;
                                            }
                                        }
                                        vacuous = false;
                                        if ((self.cur_val >= m) && (((self.cur_val > m) || (d > 7i32)) || (self.radix != 10i32))) {
                                            {
                                                if OK_so_far {
                                                    {
                                                        {
                                                            if (self.interaction == error_stop_mode) {
                                                            }
                                                            if self.file_line_error_style_p {
                                                                self.print_file_line();
                                                            } else {
                                                                self.print_nl(65544i32);
                                                            }
                                                            self.print(66053i32);
                                                        }
                                                        {
                                                            self.help_ptr = 2i32;
                                                            self.help_line[crate::ix::U((1i32) as usize)] = 66054i32;
                                                            self.help_line[crate::ix::U((0i32) as usize)] = 66055i32;
                                                        }
                                                        self.error();
                                                        self.cur_val = infinity;
                                                        OK_so_far = false;
                                                    }
                                                }
                                            }
                                        } else {
                                            self.cur_val = ((self.cur_val).wrapping_mul(self.radix)).wrapping_add(d);
                                        }
                                        self.get_x_token();
                                    }
                                }
                            }
                            // §478
                            if vacuous {
                                // §480
                                {
                                    {
                                        if (self.interaction == error_stop_mode) {
                                        }
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(65544i32);
                                        }
                                        self.print(65999i32);
                                    }
                                    {
                                        self.help_ptr = 3i32;
                                        self.help_line[crate::ix::U((2i32) as usize)] = 66000i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 66001i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 66002i32;
                                    }
                                    self.back_error();
                                }
                            } else {
                                // §478
                                if (self.cur_cmd != spacer) {
                                    self.back_input();
                                }
                            }
                        }
                    }
                }
            }
            // §474
            if negative {
                self.cur_val = (self.cur_val).wrapping_neg();
            }
            break 'l_restart_b;
        }
    }

    /// Constructions like `\.{-\'77 pt}' are legal dimensions, so `scan_dimen`
    /// may begin with `scan_int`. This explains why it is convenient to use
    /// `scan_int` also for the integer part of a decimal fraction.
    /// Several branches of `scan_dimen` work with `cur_val` as an integer and
    /// with an auxiliary fraction `f`, so that the actual quantity of interest is
    /// $`cur_val`+`f`/2^{16}$. At the end of the routine, this ``unpacked''
    /// representation is put into the single word `cur_val`, which suddenly
    /// switches significance from `integer` to `scaled`.
    // §482
    pub fn xetex_scan_dimen(&mut self, mut mu: bool, mut inf: bool, mut shortcut: bool, mut requires_units: bool) {
        let mut negative: bool = false; // §482
        let mut f: i32 = 0; // §482
        let mut num: i32 = 0; // §485
        let mut denom: i32 = 0; // §485
        let mut k: small_number = 0; // §485
        let mut kk: small_number = 0; // §485
        let mut p: halfword = 0; // §485
        let mut q: halfword = 0; // §485
        let mut v: scaled = 0; // §485
        let mut save_cur_val: i32 = 0; // §485
        'l_L89_f: {
            f = 0i32;
            self.arith_error = false;
            self.cur_order = normal;
            negative = false;
            if (!shortcut) {
                {
                    // §475
                    negative = false;
                    loop {
                        // §440
                        loop {
                            self.get_x_token();
                            if (self.cur_cmd != spacer) { break; }
                        }
                        // §475
                        if (self.cur_tok == 25165869i32) {
                            {
                                negative = (!negative);
                                self.cur_tok = 25165867i32;
                            }
                        }
                        if (self.cur_tok != 25165867i32) { break; }
                    }
                    // §482
                    if ((self.cur_cmd >= min_internal) && (self.cur_cmd <= max_internal)) {
                        // §484
                        if mu {
                            {
                                self.scan_something_internal(mu_val, false);
                                if (self.cur_val_level != int_val) {
                                    {
                                        // §486
                                        if (self.cur_val_level >= glue_val) {
                                            {
                                                v = self.mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int();
                                                self.delete_glue_ref(self.cur_val);
                                                self.cur_val = v;
                                            }
                                        }
                                        // §484
                                        if (self.cur_val_level != mu_val) {
                                            self.mu_error();
                                        }
                                        break 'l_L89_f;
                                    }
                                }
                            }
                        } else {
                            {
                                self.scan_something_internal(dimen_val, false);
                                if (self.cur_val_level == dimen_val) {
                                    break 'l_L89_f;
                                }
                            }
                        }
                    } else {
                        // §482
                        {
                            self.back_input();
                            if (self.cur_tok == continental_point_token) {
                                self.cur_tok = point_token;
                            }
                            if (self.cur_tok != point_token) {
                                self.scan_int();
                            } else {
                                {
                                    self.radix = 10i32;
                                    self.cur_val = 0i32;
                                }
                            }
                            if (self.cur_tok == continental_point_token) {
                                self.cur_tok = point_token;
                            }
                            if ((self.radix == 10i32) && (self.cur_tok == point_token)) {
                                // §487
                                {
                                    'l_done1_f: {
                                        k = 0i32;
                                        p = (268435455i32).wrapping_neg();
                                        self.get_token();
                                        while true {
                                            {
                                                self.get_x_token();
                                                if ((self.cur_tok > 25165881i32) || (self.cur_tok < zero_token)) {
                                                    break 'l_done1_f;
                                                }
                                                if (k < 17i32) {
                                                    {
                                                        q = self.get_avail();
                                                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                                                        { let __v305 = (self.cur_tok).wrapping_sub(25165872i32); self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v305); }
                                                        p = q;
                                                        k = (k).wrapping_add(1i32);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    {
                                        let __for_end_9 = 1i32;
                                        kk = k;
                                        while kk >= __for_end_9 {
                                            {
                                                { let __v306 = self.mem[crate::ix::U((p) as usize)].hh().lh(); self.dig[crate::ix::U(((kk).wrapping_sub(1i32)) as usize)] = __v306; }
                                                q = p;
                                                p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                {
                                                    { let __v307 = self.avail; self.mem[crate::ix::U((q) as usize)].set_hh_rh(__v307); }
                                                    self.avail = q;
                                                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                }
                                            }
                                            kk = kk.wrapping_sub(1);
                                        }
                                    }
                                    f = self.round_decimals(k);
                                    if (self.cur_cmd != spacer) {
                                        self.back_input();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // §482
            if (self.cur_val < 0i32) {
                {
                    negative = (!negative);
                    self.cur_val = (self.cur_val).wrapping_neg();
                }
            }
            if requires_units {
                {
                    'l_done_f: {
                        'l_L88_f: {
                            'l_done2_f: {
                                'l_not_found_f: {
                                    'l_found_f: {
                                        // §488
                                        if inf {
                                            // §489
                                            if self.scan_keyword(65597i32) {
                                                {
                                                    self.cur_order = fil;
                                                    while self.scan_keyword(108i32) {
                                                        {
                                                            if (self.cur_order == filll) {
                                                                {
                                                                    {
                                                                        if (self.interaction == error_stop_mode) {
                                                                        }
                                                                        if self.file_line_error_style_p {
                                                                            self.print_file_line();
                                                                        } else {
                                                                            self.print_nl(65544i32);
                                                                        }
                                                                        self.print(66057i32);
                                                                    }
                                                                    self.print(66058i32);
                                                                    {
                                                                        self.help_ptr = 1i32;
                                                                        self.help_line[crate::ix::U((0i32) as usize)] = 66059i32;
                                                                    }
                                                                    self.error();
                                                                }
                                                            } else {
                                                                self.cur_order = (self.cur_order).wrapping_add(1i32);
                                                            }
                                                        }
                                                    }
                                                    break 'l_L88_f;
                                                }
                                            }
                                        }
                                        // §490
                                        save_cur_val = self.cur_val;
                                        // §440
                                        loop {
                                            self.get_x_token();
                                            if (self.cur_cmd != spacer) { break; }
                                        }
                                        // §490
                                        if ((self.cur_cmd < min_internal) || (self.cur_cmd > max_internal)) {
                                            self.back_input();
                                        } else {
                                            {
                                                if mu {
                                                    {
                                                        self.scan_something_internal(mu_val, false);
                                                        // §486
                                                        if (self.cur_val_level >= glue_val) {
                                                            {
                                                                v = self.mem[crate::ix::U(((self.cur_val).wrapping_add(1i32)) as usize)].int();
                                                                self.delete_glue_ref(self.cur_val);
                                                                self.cur_val = v;
                                                            }
                                                        }
                                                        // §490
                                                        if (self.cur_val_level != mu_val) {
                                                            self.mu_error();
                                                        }
                                                    }
                                                } else {
                                                    self.scan_something_internal(dimen_val, false);
                                                }
                                                v = self.cur_val;
                                                break 'l_found_f;
                                            }
                                        }
                                        if mu {
                                            break 'l_not_found_f;
                                        }
                                        if self.scan_keyword(66060i32) {
                                            v = self.font_info[crate::ix::U(((quad_code).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                                        } else {
                                            if self.scan_keyword(66061i32) {
                                                v = self.font_info[crate::ix::U(((x_height_code).wrapping_add(self.param_base[crate::ix::U((self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].hh().rh()) as usize)])) as usize)].int();
                                            } else {
                                                break 'l_not_found_f;
                                            }
                                        }
                                        // §477
                                        {
                                            self.get_x_token();
                                            if (self.cur_cmd != spacer) {
                                                self.back_input();
                                            }
                                        }
                                    }
                                    // §490
                                    self.cur_val = { let __a308_0 = save_cur_val; let __a308_1 = v; let __a308_2 = self.xn_over_d(v, f, 65536i32); let __a308_3 = 1073741823i32; self.mult_and_add(__a308_0, __a308_1, __a308_2, __a308_3) };
                                    break 'l_L89_f;
                                }
                                // §488
                                if mu {
                                    // §491
                                    if self.scan_keyword(65625i32) {
                                        break 'l_L88_f;
                                    } else {
                                        {
                                            {
                                                if (self.interaction == error_stop_mode) {
                                                }
                                                if self.file_line_error_style_p {
                                                    self.print_file_line();
                                                } else {
                                                    self.print_nl(65544i32);
                                                }
                                                self.print(66057i32);
                                            }
                                            self.print(66062i32);
                                            {
                                                self.help_ptr = 4i32;
                                                self.help_line[crate::ix::U((3i32) as usize)] = 66063i32;
                                                self.help_line[crate::ix::U((2i32) as usize)] = 66064i32;
                                                self.help_line[crate::ix::U((1i32) as usize)] = 66065i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] = 66066i32;
                                            }
                                            self.error();
                                            break 'l_L88_f;
                                        }
                                    }
                                }
                                // §488
                                if self.scan_keyword(66056i32) {
                                    // §492
                                    {
                                        self.prepare_mag();
                                        if (self.eqtb[crate::ix::U(((7892281i32) - 1) as usize)].int() != 1000i32) {
                                            {
                                                self.cur_val = self.xn_over_d(self.cur_val, 1000i32, self.eqtb[crate::ix::U(((7892281i32) - 1) as usize)].int());
                                                f = (((1000i32).wrapping_mul(f)).wrapping_add((65536i32).wrapping_mul(self.remainder)) / self.eqtb[crate::ix::U(((7892281i32) - 1) as usize)].int());
                                                self.cur_val = (self.cur_val).wrapping_add((f / 65536i32));
                                                f = (f % 65536i32);
                                            }
                                        }
                                    }
                                }
                                // §488
                                if self.scan_keyword(65689i32) {
                                    break 'l_L88_f;
                                }
                                // §493
                                if self.scan_keyword(66067i32) {
                                    {
                                        num = 7227i32;
                                        denom = 100i32;
                                    }
                                } else {
                                    if self.scan_keyword(66068i32) {
                                        {
                                            num = 12i32;
                                            denom = 1i32;
                                        }
                                    } else {
                                        if self.scan_keyword(66069i32) {
                                            {
                                                num = 7227i32;
                                                denom = 254i32;
                                            }
                                        } else {
                                            if self.scan_keyword(66070i32) {
                                                {
                                                    num = 7227i32;
                                                    denom = 2540i32;
                                                }
                                            } else {
                                                if self.scan_keyword(66071i32) {
                                                    {
                                                        num = 7227i32;
                                                        denom = 7200i32;
                                                    }
                                                } else {
                                                    if self.scan_keyword(66072i32) {
                                                        {
                                                            num = 1238i32;
                                                            denom = 1157i32;
                                                        }
                                                    } else {
                                                        if self.scan_keyword(66073i32) {
                                                            {
                                                                num = 14856i32;
                                                                denom = 1157i32;
                                                            }
                                                        } else {
                                                            if self.scan_keyword(66074i32) {
                                                                break 'l_done_f;
                                                            } else {
                                                                // §494
                                                                {
                                                                    {
                                                                        if (self.interaction == error_stop_mode) {
                                                                        }
                                                                        if self.file_line_error_style_p {
                                                                            self.print_file_line();
                                                                        } else {
                                                                            self.print_nl(65544i32);
                                                                        }
                                                                        self.print(66057i32);
                                                                    }
                                                                    self.print(66075i32);
                                                                    {
                                                                        self.help_ptr = 6i32;
                                                                        self.help_line[crate::ix::U((5i32) as usize)] = 66076i32;
                                                                        self.help_line[crate::ix::U((4i32) as usize)] = 66077i32;
                                                                        self.help_line[crate::ix::U((3i32) as usize)] = 66078i32;
                                                                        self.help_line[crate::ix::U((2i32) as usize)] = 66064i32;
                                                                        self.help_line[crate::ix::U((1i32) as usize)] = 66065i32;
                                                                        self.help_line[crate::ix::U((0i32) as usize)] = 66066i32;
                                                                    }
                                                                    self.error();
                                                                    break 'l_done2_f;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                // §493
                                self.cur_val = self.xn_over_d(self.cur_val, num, denom);
                                f = (((num).wrapping_mul(f)).wrapping_add((65536i32).wrapping_mul(self.remainder)) / denom);
                                self.cur_val = (self.cur_val).wrapping_add((f / 65536i32));
                                f = (f % 65536i32);
                            }
                        }
                        // §488
                        if (self.cur_val >= 16384i32) {
                            self.arith_error = true;
                        } else {
                            self.cur_val = ((self.cur_val).wrapping_mul(unity)).wrapping_add(f);
                        }
                    }
                    // §477
                    {
                        self.get_x_token();
                        if (self.cur_cmd != spacer) {
                            self.back_input();
                        }
                    }
                }
            } else {
                // §482
                {
                    if (self.cur_val >= 16384i32) {
                        self.arith_error = true;
                    } else {
                        self.cur_val = ((self.cur_val).wrapping_mul(unity)).wrapping_add(f);
                    }
                }
            }
        }
        if (self.arith_error || ((self.cur_val).wrapping_abs() >= 1073741824i32)) {
            // §495
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(66079i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 66080i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 66081i32;
                }
                self.error();
                self.cur_val = max_dimen;
                self.arith_error = false;
            }
        }
        // §482
        if negative {
            self.cur_val = (self.cur_val).wrapping_neg();
        }
    }

    /// Constructions like `\.{-\'77 pt}' are legal dimensions, so `scan_dimen`
    /// may begin with `scan_int`. This explains why it is convenient to use
    /// `scan_int` also for the integer part of a decimal fraction.
    /// Several branches of `scan_dimen` work with `cur_val` as an integer and
    /// with an auxiliary fraction `f`, so that the actual quantity of interest is
    /// $`cur_val`+`f`/2^{16}$. At the end of the routine, this ``unpacked''
    /// representation is put into the single word `cur_val`, which suddenly
    /// switches significance from `integer` to `scaled`.
    // §482
    pub fn scan_dimen(&mut self, mut mu: bool, mut inf: bool, mut shortcut: bool) {
        self.xetex_scan_dimen(mu, inf, shortcut, true);
    }

    /// For XeTeX, we have an additional version `scan_decimal`, like `scan_dimen`
    /// but without any scanning of units.
    // §483
    pub fn scan_decimal(&mut self) {
        self.xetex_scan_dimen(false, false, false, false);
    }

    /// The final member of \TeX's value-scanning trio is `scan_glue`, which
    /// makes `cur_val` point to a glue specification. The reference count of that
    /// glue spec will take account of the fact that `cur_val` is pointing to~it.
    /// The `level` parameter should be either `glue_val` or `mu_val`.
    /// Since `scan_dimen` was so much more complex than `scan_int`, we might expect
    /// `scan_glue` to be even worse. But fortunately, it is very simple, since
    /// most of the work has already been done.
    // §496
    pub fn scan_glue(&mut self, mut level: small_number) {
        let mut negative: bool = false; // §496
        let mut q: halfword = 0; // §496
        let mut mu: bool = false; // §496
        'l_exit_f: {
            mu = (level == mu_val);
            // §475
            negative = false;
            loop {
                // §440
                loop {
                    self.get_x_token();
                    if (self.cur_cmd != spacer) { break; }
                }
                // §475
                if (self.cur_tok == 25165869i32) {
                    {
                        negative = (!negative);
                        self.cur_tok = 25165867i32;
                    }
                }
                if (self.cur_tok != 25165867i32) { break; }
            }
            // §496
            if ((self.cur_cmd >= min_internal) && (self.cur_cmd <= max_internal)) {
                {
                    self.scan_something_internal(level, negative);
                    if (self.cur_val_level >= glue_val) {
                        {
                            if (self.cur_val_level != level) {
                                self.mu_error();
                            }
                            break 'l_exit_f;
                        }
                    }
                    if (self.cur_val_level == int_val) {
                        self.scan_dimen(mu, false, true);
                    } else {
                        if (level == mu_val) {
                            self.mu_error();
                        }
                    }
                }
            } else {
                {
                    self.back_input();
                    self.scan_dimen(mu, false, false);
                    if negative {
                        self.cur_val = (self.cur_val).wrapping_neg();
                    }
                }
            }
            // §497
            q = self.new_spec(zero_glue);
            { let __v309 = self.cur_val; self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v309); }
            if self.scan_keyword(66082i32) {
                {
                    self.scan_dimen(mu, true, false);
                    { let __v310 = self.cur_val; self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v310); }
                    { let __v311 = self.cur_order; self.mem[crate::ix::U((q) as usize)].set_hh_b0(__v311); }
                }
            }
            if self.scan_keyword(66083i32) {
                {
                    self.scan_dimen(mu, true, false);
                    { let __v312 = self.cur_val; self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v312); }
                    { let __v313 = self.cur_order; self.mem[crate::ix::U((q) as usize)].set_hh_b1(__v313); }
                }
            }
            self.cur_val = q;
        }
        // §496
    }

    /// The function `add_or_sub(x,y,max_answer,negative)` computes the sum
    /// (for `negative=false`) or difference (for `negative=true`) of `x` and
    /// `y`, provided the absolute value of the result does not exceed
    /// `max_answer`.
    /// @<Declare subprocedures for `scan_expr`
    // §1604
    pub fn add_or_sub(&mut self, mut x: i32, mut y: i32, mut max_answer: i32, mut negative: bool) -> i32 {
        let mut add_or_sub: i32 = 0;
        let mut a: i32 = 0; // §1604
        if negative {
            y = (y).wrapping_neg();
        }
        if (x >= 0i32) {
            if (y <= (max_answer).wrapping_sub(x)) {
                a = (x).wrapping_add(y);
            } else {
                {
                    self.arith_error = true;
                    a = 0i32;
                }
            }
        } else {
            if (y >= ((max_answer).wrapping_neg()).wrapping_sub(x)) {
                a = (x).wrapping_add(y);
            } else {
                {
                    self.arith_error = true;
                    a = 0i32;
                }
            }
        }
        add_or_sub = a;
        add_or_sub
    }

    /// The function `quotient(n,d)` computes the rounded quotient
    /// $q=\lfloor n/d+{1\over2}\rfloor$, when $n$ and $d$ are positive.
    /// @<Declare subprocedures for `scan_expr`
    // §1608
    pub fn quotient(&mut self, mut n: i32, mut d: i32) -> i32 {
        let mut quotient: i32 = 0;
        let mut negative: bool = false; // §1608
        let mut a: i32 = 0; // §1608
        if (d == 0i32) {
            {
                self.arith_error = true;
                a = 0i32;
            }
        } else {
            {
                if (d > 0i32) {
                    negative = false;
                } else {
                    {
                        d = (d).wrapping_neg();
                        negative = true;
                    }
                }
                if (n < 0i32) {
                    {
                        n = (n).wrapping_neg();
                        negative = (!negative);
                    }
                }
                a = (n / d);
                n = (n).wrapping_sub((a).wrapping_mul(d));
                d = (n).wrapping_sub(d);
                if ((d).wrapping_add(n) >= 0i32) {
                    a = (a).wrapping_add(1i32);
                }
                if negative {
                    a = (a).wrapping_neg();
                }
            }
        }
        quotient = a;
        quotient
    }

    /// Finally, the function `fract(x,n,d,max_answer)` computes the integer
    /// $q=\lfloor xn/d+{1\over2}\rfloor$, when $x$, $n$, and $d$ are positive
    /// and the result does not exceed `max_answer`.  We can't use floating
    /// point arithmetic since the routine must produce identical results in all
    /// cases; and it would be too dangerous to multiply by~`n` and then divide
    /// by~`d`, in separate operations, since overflow might well occur.  Hence
    /// this subroutine simulates double precision arithmetic, somewhat
    /// analogous to \MF's `make_fraction` and `take_fraction` routines.
    // §1610
    pub fn fract(&mut self, mut x: i32, mut n: i32, mut d: i32, mut max_answer: i32) -> i32 {
        let mut fract: i32 = 0;
        let mut negative: bool = false; // §1610
        let mut a: i32 = 0; // §1610
        let mut f: i32 = 0; // §1610
        let mut h: i32 = 0; // §1610
        let mut r: i32 = 0; // §1610
        let mut t: i32 = 0; // §1610
        'l_done_f: {
            'l_L88_f: {
                'l_found_f: {
                    'l_found1_f: {
                        if (d == 0i32) {
                            break 'l_L88_f;
                        }
                        a = 0i32;
                        if (d > 0i32) {
                            negative = false;
                        } else {
                            {
                                d = (d).wrapping_neg();
                                negative = true;
                            }
                        }
                        if (x < 0i32) {
                            {
                                x = (x).wrapping_neg();
                                negative = (!negative);
                            }
                        } else {
                            if (x == 0i32) {
                                break 'l_done_f;
                            }
                        }
                        if (n < 0i32) {
                            {
                                n = (n).wrapping_neg();
                                negative = (!negative);
                            }
                        }
                        t = (n / d);
                        if (t > (max_answer / x)) {
                            break 'l_L88_f;
                        }
                        a = (t).wrapping_mul(x);
                        n = (n).wrapping_sub((t).wrapping_mul(d));
                        if (n == 0i32) {
                            break 'l_found_f;
                        }
                        t = (x / d);
                        if (t > ((max_answer).wrapping_sub(a) / n)) {
                            break 'l_L88_f;
                        }
                        a = (a).wrapping_add((t).wrapping_mul(n));
                        x = (x).wrapping_sub((t).wrapping_mul(d));
                        if (x == 0i32) {
                            break 'l_found_f;
                        }
                        if (x < n) {
                            {
                                t = x;
                                x = n;
                                n = t;
                            }
                        }
                        // §1611
                        f = 0i32;
                        r = ((d / 2i32)).wrapping_sub(d);
                        h = (r).wrapping_neg();
                        while true {
                            {
                                if (((n) % 2) != 0) {
                                    {
                                        r = (r).wrapping_add(x);
                                        if (r >= 0i32) {
                                            {
                                                r = (r).wrapping_sub(d);
                                                f = (f).wrapping_add(1i32);
                                            }
                                        }
                                    }
                                }
                                n = (n / 2i32);
                                if (n == 0i32) {
                                    break 'l_found1_f;
                                }
                                if (x < h) {
                                    x = (x).wrapping_add(x);
                                } else {
                                    {
                                        t = (x).wrapping_sub(d);
                                        x = (t).wrapping_add(x);
                                        f = (f).wrapping_add(n);
                                        if (x < n) {
                                            {
                                                if (x == 0i32) {
                                                    break 'l_found1_f;
                                                }
                                                t = x;
                                                x = n;
                                                n = t;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if (f > (max_answer).wrapping_sub(a)) {
                        // §1610
                        break 'l_L88_f;
                    }
                    a = (a).wrapping_add(f);
                }
                if negative {
                    a = (a).wrapping_neg();
                }
                break 'l_done_f;
            }
            {
                self.arith_error = true;
                a = 0i32;
            }
        }
        fract = a;
        fract
    }

    /// The `scan_expr` procedure scans and evaluates an expression.
    /// @<Declare procedures needed for expressions
    // §1593
    pub fn scan_expr(&mut self) {
        let mut a: bool = false; // §1593
        let mut b: bool = false; // §1593
        let mut l: small_number = 0; // §1593
        let mut r: small_number = 0; // §1593
        let mut s: small_number = 0; // §1593
        let mut o: small_number = 0; // §1593
        let mut e: i32 = 0; // §1593
        let mut t: i32 = 0; // §1593
        let mut f: i32 = 0; // §1593
        let mut n: i32 = 0; // §1593
        let mut p: halfword = 0; // §1593
        let mut q: halfword = 0; // §1593
        l = self.cur_val_level;
        a = self.arith_error;
        b = false;
        p = (268435455i32).wrapping_neg();
        self.expand_depth_count = (self.expand_depth_count).wrapping_add(1i32);
        if (self.expand_depth_count >= self.expand_depth) {
            self.overflow(65942i32, self.expand_depth);
        }
        'l_restart_b: loop {
            // §1594
            r = expr_none;
            e = 0i32;
            s = expr_none;
            t = 0i32;
            n = 0i32;
            'l_continue_b: loop {
                if (s == expr_none) {
                    o = l;
                } else {
                    o = int_val;
                }
                // §440
                loop {
                    self.get_x_token();
                    if (self.cur_cmd != spacer) { break; }
                }
                // §1596
                if (self.cur_tok == 25165864i32) {
                    // §1599
                    {
                        q = self.get_node(expr_node_size);
                        self.mem[crate::ix::U((q) as usize)].set_hh_rh(p);
                        self.mem[crate::ix::U((q) as usize)].set_hh_b0(l);
                        self.mem[crate::ix::U((q) as usize)].set_hh_b1(((4i32).wrapping_mul(s)).wrapping_add(r));
                        self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(e);
                        self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(t);
                        self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(n);
                        p = q;
                        l = o;
                        continue 'l_restart_b;
                    }
                }
                // §1596
                self.back_input();
                if (o == int_val) {
                    self.scan_int();
                } else {
                    if (o == dimen_val) {
                        self.scan_dimen(false, false, false);
                    } else {
                        if (o == glue_val) {
                            self.scan_normal_glue();
                        } else {
                            self.scan_mu_glue();
                        }
                    }
                }
                f = self.cur_val;
                'l_found_b: loop {
                    // §1594
                    loop {
                        // §440
                        self.get_x_token();
                        if (self.cur_cmd != spacer) { break; }
                    }
                    // §1595
                    if (self.cur_tok == 25165867i32) {
                        o = expr_add;
                    } else {
                        if (self.cur_tok == 25165869i32) {
                            o = expr_sub;
                        } else {
                            if (self.cur_tok == 25165866i32) {
                                o = expr_mult;
                            } else {
                                if (self.cur_tok == 25165871i32) {
                                    o = expr_div;
                                } else {
                                    {
                                        o = expr_none;
                                        if (p == (268435455i32).wrapping_neg()) {
                                            {
                                                if (self.cur_cmd != relax) {
                                                    self.back_input();
                                                }
                                            }
                                        } else {
                                            if (self.cur_tok != 25165865i32) {
                                                {
                                                    {
                                                        if (self.interaction == error_stop_mode) {
                                                        }
                                                        if self.file_line_error_style_p {
                                                            self.print_file_line();
                                                        } else {
                                                            self.print_nl(65544i32);
                                                        }
                                                        self.print(66938i32);
                                                    }
                                                    {
                                                        self.help_ptr = 1i32;
                                                        self.help_line[crate::ix::U((0i32) as usize)] = 66939i32;
                                                    }
                                                    self.back_error();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // §1594
                    self.arith_error = b;
                    // §1601
                    if ((l == int_val) || (s > expr_sub)) {
                        {
                            if ((f > infinity) || (f < (2147483647i32).wrapping_neg())) {
                                {
                                    self.arith_error = true;
                                    f = 0i32;
                                }
                            }
                        }
                    } else {
                        if (l == dimen_val) {
                            {
                                if ((f).wrapping_abs() > max_dimen) {
                                    {
                                        self.arith_error = true;
                                        f = 0i32;
                                    }
                                }
                            }
                        } else {
                            {
                                if ((((self.mem[crate::ix::U(((f).wrapping_add(1i32)) as usize)].int()).wrapping_abs() > max_dimen) || ((self.mem[crate::ix::U(((f).wrapping_add(2i32)) as usize)].int()).wrapping_abs() > max_dimen)) || ((self.mem[crate::ix::U(((f).wrapping_add(3i32)) as usize)].int()).wrapping_abs() > max_dimen)) {
                                    {
                                        self.arith_error = true;
                                        self.delete_glue_ref(f);
                                        f = self.new_spec(zero_glue);
                                    }
                                }
                            }
                        }
                    }
                    // §1594
                    match s {
                        expr_none => {
                            // §1602
                            if ((l >= glue_val) && (o != expr_none)) {
                                {
                                    t = self.new_spec(f);
                                    self.delete_glue_ref(f);
                                    if (self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int() == 0i32) {
                                        self.mem[crate::ix::U((t) as usize)].set_hh_b0(normal);
                                    }
                                    if (self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int() == 0i32) {
                                        self.mem[crate::ix::U((t) as usize)].set_hh_b1(normal);
                                    }
                                }
                            } else {
                                t = f;
                            }
                        }
                        expr_mult => {
                            // §1606
                            if (o == expr_div) {
                                {
                                    n = f;
                                    o = expr_scale;
                                }
                            } else {
                                if (l == int_val) {
                                    t = self.mult_and_add(t, f, 0i32, 2147483647i32);
                                } else {
                                    if (l == dimen_val) {
                                        t = self.mult_and_add(t, f, 0i32, 1073741823i32);
                                    } else {
                                        {
                                            { let __v314 = self.mult_and_add(self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].int(), f, 0i32, 1073741823i32); self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_int(__v314); }
                                            { let __v315 = self.mult_and_add(self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int(), f, 0i32, 1073741823i32); self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].set_int(__v315); }
                                            { let __v316 = self.mult_and_add(self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int(), f, 0i32, 1073741823i32); self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].set_int(__v316); }
                                        }
                                    }
                                }
                            }
                        }
                        expr_div => {
                            // §1607
                            if (l < glue_val) {
                                t = self.quotient(t, f);
                            } else {
                                {
                                    { let __v317 = self.quotient(self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].int(), f); self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_int(__v317); }
                                    { let __v318 = self.quotient(self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int(), f); self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].set_int(__v318); }
                                    { let __v319 = self.quotient(self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int(), f); self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].set_int(__v319); }
                                }
                            }
                        }
                        expr_scale => {
                            // §1609
                            if (l == int_val) {
                                t = self.fract(t, n, f, infinity);
                            } else {
                                if (l == dimen_val) {
                                    t = self.fract(t, n, f, max_dimen);
                                } else {
                                    {
                                        { let __v320 = self.fract(self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].int(), n, f, max_dimen); self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].set_int(__v320); }
                                        { let __v321 = self.fract(self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int(), n, f, max_dimen); self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].set_int(__v321); }
                                        { let __v322 = self.fract(self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int(), n, f, max_dimen); self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].set_int(__v322); }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    // §1594
                    if (o > expr_sub) {
                        s = o;
                    } else {
                        // §1603
                        {
                            s = expr_none;
                            if (r == expr_none) {
                                e = t;
                            } else {
                                if (l == int_val) {
                                    e = self.add_or_sub(e, t, infinity, (r == expr_sub));
                                } else {
                                    if (l == dimen_val) {
                                        e = self.add_or_sub(e, t, max_dimen, (r == expr_sub));
                                    } else {
                                        // §1605
                                        {
                                            { let __v323 = self.add_or_sub(self.mem[crate::ix::U(((e).wrapping_add(1i32)) as usize)].int(), self.mem[crate::ix::U(((t).wrapping_add(1i32)) as usize)].int(), max_dimen, (r == expr_sub)); self.mem[crate::ix::U(((e).wrapping_add(1i32)) as usize)].set_int(__v323); }
                                            if (self.mem[crate::ix::U((e) as usize)].hh().b0() == self.mem[crate::ix::U((t) as usize)].hh().b0()) {
                                                { let __v324 = self.add_or_sub(self.mem[crate::ix::U(((e).wrapping_add(2i32)) as usize)].int(), self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int(), max_dimen, (r == expr_sub)); self.mem[crate::ix::U(((e).wrapping_add(2i32)) as usize)].set_int(__v324); }
                                            } else {
                                                if ((self.mem[crate::ix::U((e) as usize)].hh().b0() < self.mem[crate::ix::U((t) as usize)].hh().b0()) && (self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int() != 0i32)) {
                                                    {
                                                        { let __v325 = self.mem[crate::ix::U(((t).wrapping_add(2i32)) as usize)].int(); self.mem[crate::ix::U(((e).wrapping_add(2i32)) as usize)].set_int(__v325); }
                                                        { let __v326 = self.mem[crate::ix::U((t) as usize)].hh().b0(); self.mem[crate::ix::U((e) as usize)].set_hh_b0(__v326); }
                                                    }
                                                }
                                            }
                                            if (self.mem[crate::ix::U((e) as usize)].hh().b1() == self.mem[crate::ix::U((t) as usize)].hh().b1()) {
                                                { let __v327 = self.add_or_sub(self.mem[crate::ix::U(((e).wrapping_add(3i32)) as usize)].int(), self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int(), max_dimen, (r == expr_sub)); self.mem[crate::ix::U(((e).wrapping_add(3i32)) as usize)].set_int(__v327); }
                                            } else {
                                                if ((self.mem[crate::ix::U((e) as usize)].hh().b1() < self.mem[crate::ix::U((t) as usize)].hh().b1()) && (self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int() != 0i32)) {
                                                    {
                                                        { let __v328 = self.mem[crate::ix::U(((t).wrapping_add(3i32)) as usize)].int(); self.mem[crate::ix::U(((e).wrapping_add(3i32)) as usize)].set_int(__v328); }
                                                        { let __v329 = self.mem[crate::ix::U((t) as usize)].hh().b1(); self.mem[crate::ix::U((e) as usize)].set_hh_b1(__v329); }
                                                    }
                                                }
                                            }
                                            self.delete_glue_ref(t);
                                            if (self.mem[crate::ix::U(((e).wrapping_add(2i32)) as usize)].int() == 0i32) {
                                                self.mem[crate::ix::U((e) as usize)].set_hh_b0(normal);
                                            }
                                            if (self.mem[crate::ix::U(((e).wrapping_add(3i32)) as usize)].int() == 0i32) {
                                                self.mem[crate::ix::U((e) as usize)].set_hh_b1(normal);
                                            }
                                        }
                                    }
                                }
                            }
                            // §1603
                            r = o;
                        }
                    }
                    // §1594
                    b = self.arith_error;
                    if (o != expr_none) {
                        continue 'l_continue_b;
                    }
                    if (p != (268435455i32).wrapping_neg()) {
                        // §1600
                        {
                            f = e;
                            q = p;
                            e = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].int();
                            t = self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].int();
                            n = self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].int();
                            s = (self.mem[crate::ix::U((q) as usize)].hh().b1() / 4i32);
                            r = (self.mem[crate::ix::U((q) as usize)].hh().b1() % 4i32);
                            l = self.mem[crate::ix::U((q) as usize)].hh().b0();
                            p = self.mem[crate::ix::U((q) as usize)].hh().rh();
                            self.free_node(q, expr_node_size);
                            continue 'l_found_b;
                        }
                    }
                    // §1593
                    self.expand_depth_count = (self.expand_depth_count).wrapping_sub(1i32);
                    if b {
                        {
                            {
                                if (self.interaction == error_stop_mode) {
                                }
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(65544i32);
                                }
                                self.print(66654i32);
                            }
                            {
                                self.help_ptr = 2i32;
                                self.help_line[crate::ix::U((1i32) as usize)] = 66937i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 66656i32;
                            }
                            self.error();
                            if (l >= glue_val) {
                                {
                                    self.delete_glue_ref(e);
                                    e = zero_glue;
                                    { let __v330 = (self.mem[crate::ix::U((e) as usize)].hh().rh()).wrapping_add(1i32); self.mem[crate::ix::U((e) as usize)].set_hh_rh(__v330); }
                                }
                            } else {
                                e = 0i32;
                            }
                        }
                    }
                    self.arith_error = a;
                    self.cur_val = e;
                    self.cur_val_level = l;
                    break 'l_found_b;
                }
                break 'l_continue_b;
            }
            break 'l_restart_b;
        }
    }

    /// Here we declare two trivial procedures in order to avoid mutually
    /// recursive procedures with parameters.
    /// @<Declare procedures needed for expressions
    // §1598
    pub fn scan_normal_glue(&mut self) {
        self.scan_glue(glue_val);
    }

    /// Here we declare two trivial procedures in order to avoid mutually
    /// recursive procedures with parameters.
    /// @<Declare procedures needed for expressions
    // §1598
    pub fn scan_mu_glue(&mut self) {
        self.scan_glue(mu_val);
    }

    /// Here's a similar procedure that returns a pointer to a rule node. This
    /// routine is called just after \TeX\ has seen \.{\\hrule} or \.{\\vrule};
    /// therefore `cur_cmd` will be either `hrule` or `vrule`. The idea is to store
    /// the default rule dimensions in the node, then to override them if
    /// `\.{height}' or `\.{width}' or `\.{depth}' specifications are
    /// found (in any order).
    // §498
    pub fn scan_rule_spec(&mut self) -> halfword {
        let mut scan_rule_spec: halfword = 0;
        let mut q: halfword = 0; // §498
        q = self.new_rule();
        if (self.cur_cmd == vrule) {
            self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(default_rule);
        } else {
            {
                self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(default_rule);
                self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(0i32);
            }
        }
        'l_reswitch_b: loop {
            if self.scan_keyword(66084i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __v331 = self.cur_val; self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].set_int(__v331); }
                    continue 'l_reswitch_b;
                }
            }
            if self.scan_keyword(66085i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __v332 = self.cur_val; self.mem[crate::ix::U(((q).wrapping_add(3i32)) as usize)].set_int(__v332); }
                    continue 'l_reswitch_b;
                }
            }
            if self.scan_keyword(66086i32) {
                {
                    self.scan_dimen(false, false, false);
                    { let __v333 = self.cur_val; self.mem[crate::ix::U(((q).wrapping_add(2i32)) as usize)].set_int(__v333); }
                    continue 'l_reswitch_b;
                }
            }
            scan_rule_spec = q;
            break 'l_reswitch_b;
        }
        scan_rule_spec
    }

    /// The token list (balanced text) created by `scan_general_text` begins
    /// at `link(temp_head)` and ends at `cur_val`.  (If `cur_val=temp_head`,
    /// the list is empty.)
    /// @<Declare \eTeX\ procedures for tok...
    // §1493
    pub fn scan_general_text(&mut self) {
        let mut s: i32 = 0; // §1493
        let mut w: halfword = 0; // §1493
        let mut d: halfword = 0; // §1493
        let mut p: halfword = 0; // §1493
        let mut q: halfword = 0; // §1493
        let mut unbalance: halfword = 0; // §1493
        'l_found_f: {
            s = self.scanner_status;
            w = self.warning_index;
            d = self.def_ref;
            self.scanner_status = absorbing;
            self.warning_index = self.cur_cs;
            self.def_ref = self.get_avail();
            { let __ix334 = self.def_ref; self.mem[crate::ix::U((__ix334) as usize)].set_hh_lh((268435455i32).wrapping_neg()); }
            p = self.def_ref;
            self.scan_left_brace();
            unbalance = 1i32;
            while true {
                {
                    self.get_token();
                    if (self.cur_tok < right_brace_limit) {
                        if (self.cur_cmd < right_brace) {
                            unbalance = (unbalance).wrapping_add(1i32);
                        } else {
                            {
                                unbalance = (unbalance).wrapping_sub(1i32);
                                if (unbalance == 0i32) {
                                    break 'l_found_f;
                                }
                            }
                        }
                    }
                    {
                        q = self.get_avail();
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                        { let __v335 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v335); }
                        p = q;
                    }
                }
            }
        }
        q = self.mem[crate::ix::U((self.def_ref) as usize)].hh().rh();
        {
            { let __ix336 = self.def_ref; let __v337 = self.avail; self.mem[crate::ix::U((__ix336) as usize)].set_hh_rh(__v337); }
            self.avail = self.def_ref;
            self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
        }
        if (q == (268435455i32).wrapping_neg()) {
            self.cur_val = temp_head;
        } else {
            self.cur_val = p;
        }
        self.mem[crate::ix::U((temp_head) as usize)].set_hh_rh(q);
        self.scanner_status = s;
        self.warning_index = w;
        self.def_ref = d;
    }

    /// @<Declare \eTeX\ procedures for tok...
    // §1564
    pub fn pseudo_start(&mut self) {
        let mut old_setting: i32 = 0; // §1564
        let mut s: str_number = 0; // §1564
        let mut l: pool_pointer = 0; // §1564
        let mut m: pool_pointer = 0; // §1564
        let mut p: halfword = 0; // §1564
        let mut q: halfword = 0; // §1564
        let mut r: halfword = 0; // §1564
        let mut w: four_quarters = four_quarters::default(); // §1564
        let mut nl: i32 = 0; // §1564
        let mut sz: i32 = 0; // §1564
        self.scan_general_text();
        old_setting = self.selector;
        self.selector = new_string;
        self.token_show(temp_head);
        self.selector = old_setting;
        self.flush_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh());
        {
            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                self.overflow(65539i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        s = self.make_string();
        // §1565
        self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = 32i32;
        l = self.str_start[crate::ix::U(((s).wrapping_sub(65536i32)) as usize)];
        nl = self.eqtb[crate::ix::U(((7892313i32) - 1) as usize)].int();
        p = self.get_avail();
        q = p;
        while (l < self.pool_ptr) {
            {
                m = l;
                while ((l < self.pool_ptr) && (self.str_pool[crate::ix::U((l) as usize)] != nl)) {
                    l = (l).wrapping_add(1i32);
                }
                sz = (((l).wrapping_sub(m)).wrapping_add(7i32) / 4i32);
                if (sz == 1i32) {
                    sz = 2i32;
                }
                r = self.get_node(sz);
                self.mem[crate::ix::U((q) as usize)].set_hh_rh(r);
                q = r;
                self.mem[crate::ix::U((q) as usize)].set_hh_lh(sz);
                while (sz > 2i32) {
                    {
                        sz = (sz).wrapping_sub(1i32);
                        r = (r).wrapping_add(1i32);
                        { let __v338 = self.str_pool[crate::ix::U((m) as usize)]; w.set_b0(__v338); }
                        { let __v339 = self.str_pool[crate::ix::U(((m).wrapping_add(1i32)) as usize)]; w.set_b1(__v339); }
                        { let __v340 = self.str_pool[crate::ix::U(((m).wrapping_add(2i32)) as usize)]; w.set_b2(__v340); }
                        { let __v341 = self.str_pool[crate::ix::U(((m).wrapping_add(3i32)) as usize)]; w.set_b3(__v341); }
                        self.mem[crate::ix::U((r) as usize)].set_qqqq(w);
                        m = (m).wrapping_add(4i32);
                    }
                }
                w.set_b0(32i32);
                w.set_b1(32i32);
                w.set_b2(32i32);
                w.set_b3(32i32);
                if (l > m) {
                    {
                        { let __v342 = self.str_pool[crate::ix::U((m) as usize)]; w.set_b0(__v342); }
                        if (l > (m).wrapping_add(1i32)) {
                            {
                                { let __v343 = self.str_pool[crate::ix::U(((m).wrapping_add(1i32)) as usize)]; w.set_b1(__v343); }
                                if (l > (m).wrapping_add(2i32)) {
                                    {
                                        { let __v344 = self.str_pool[crate::ix::U(((m).wrapping_add(2i32)) as usize)]; w.set_b2(__v344); }
                                        if (l > (m).wrapping_add(3i32)) {
                                            { let __v345 = self.str_pool[crate::ix::U(((m).wrapping_add(3i32)) as usize)]; w.set_b3(__v345); }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                self.mem[crate::ix::U(((r).wrapping_add(1i32)) as usize)].set_qqqq(w);
                if (self.str_pool[crate::ix::U((l) as usize)] == nl) {
                    l = (l).wrapping_add(1i32);
                }
            }
        }
        { let __v346 = self.mem[crate::ix::U((p) as usize)].hh().rh(); self.mem[crate::ix::U((p) as usize)].set_hh_lh(__v346); }
        { let __v347 = self.pseudo_files; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v347); }
        self.pseudo_files = p;
        // §1564
        {
            self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
            self.pool_ptr = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
        }
        // §1566
        self.begin_file_reading();
        self.line = 0i32;
        self.cur_input.limit_field = self.cur_input.start_field;
        self.cur_input.loc_field = (self.cur_input.limit_field).wrapping_add(1i32);
        if (self.eqtb[crate::ix::U(((7892328i32) - 1) as usize)].int() > 0i32) {
            {
                if (self.term_offset > (self.max_print_line).wrapping_sub(3i32)) {
                    self.print_ln();
                } else {
                    if ((self.term_offset > 0i32) || (self.file_offset > 0i32)) {
                        self.print_char(32i32);
                    }
                }
                self.cur_input.name_field = 19i32;
                self.print(66922i32);
                self.open_parens = (self.open_parens).wrapping_add(1i32);
                crate::system::break_out(&mut self.term_out);
            }
        } else {
            {
                self.cur_input.name_field = 18i32;
                // §1707
                self.cur_input.synctex_tag_field = 0i32;
            }
        }
    }

    /// \[27] Building token lists.
    /// The token lists for macros and for other things like \.{\\mark} and \.{\\output}
    /// and \.{\\write} are produced by a procedure called `scan_toks`.
    /// Before we get into the details of `scan_toks`, let's consider a much
    /// simpler task, that of converting the current string into a token list.
    /// The `str_toks` function does this; it classifies spaces as type `spacer`
    /// and everything else as type `other_char`.
    /// The token list created by `str_toks` begins at `link(temp_head)` and ends
    /// at the value `p` that is returned. (If `p=temp_head`, the list is empty.)
    /// The `str_toks_cat` function is the same, except that the catcode `cat` is
    /// stamped on all the characters, unless zero is passed in which case it
    /// chooses `spacer` or `other_char` automatically.
    // §499
    pub fn str_toks_cat(&mut self, mut b: pool_pointer, mut cat: small_number) -> halfword {
        let mut str_toks_cat: halfword = 0;
        let mut p: halfword = 0; // §499
        let mut q: halfword = 0; // §499
        let mut t: halfword = 0; // §499
        let mut k: pool_pointer = 0; // §499
        {
            if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                self.overflow(65539i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        p = temp_head;
        self.mem[crate::ix::U((p) as usize)].set_hh_rh((268435455i32).wrapping_neg());
        k = b;
        while (k < self.pool_ptr) {
            {
                t = self.str_pool[crate::ix::U((k) as usize)];
                if ((t == 32i32) && (cat == 0i32)) {
                    t = space_token;
                } else {
                    {
                        if (((((t >= 55296i32) && (t <= 56319i32)) && ((k).wrapping_add(1i32) < self.pool_ptr)) && (self.str_pool[crate::ix::U(((k).wrapping_add(1i32)) as usize)] >= 56320i32)) && (self.str_pool[crate::ix::U(((k).wrapping_add(1i32)) as usize)] <= 57343i32)) {
                            {
                                k = (k).wrapping_add(1i32);
                                t = ((65536i32).wrapping_add(((t).wrapping_sub(55296i32)).wrapping_mul(1024i32))).wrapping_add((self.str_pool[crate::ix::U((k) as usize)]).wrapping_sub(56320i32));
                            }
                        }
                        if (cat == 0i32) {
                            t = (other_token).wrapping_add(t);
                        } else {
                            if (cat == active_char) {
                                t = (33554432i32).wrapping_add(t);
                            } else {
                                t = ((max_char_val).wrapping_mul(cat)).wrapping_add(t);
                            }
                        }
                    }
                }
                {
                    {
                        q = self.avail;
                        if (q == (268435455i32).wrapping_neg()) {
                            q = self.get_avail();
                        } else {
                            {
                                self.avail = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                self.mem[crate::ix::U((q) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                            }
                        }
                    }
                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                    self.mem[crate::ix::U((q) as usize)].set_hh_lh(t);
                    p = q;
                }
                k = (k).wrapping_add(1i32);
            }
        }
        self.pool_ptr = b;
        str_toks_cat = p;
        str_toks_cat
    }

    /// \[27] Building token lists.
    /// The token lists for macros and for other things like \.{\\mark} and \.{\\output}
    /// and \.{\\write} are produced by a procedure called `scan_toks`.
    /// Before we get into the details of `scan_toks`, let's consider a much
    /// simpler task, that of converting the current string into a token list.
    /// The `str_toks` function does this; it classifies spaces as type `spacer`
    /// and everything else as type `other_char`.
    /// The token list created by `str_toks` begins at `link(temp_head)` and ends
    /// at the value `p` that is returned. (If `p=temp_head`, the list is empty.)
    /// The `str_toks_cat` function is the same, except that the catcode `cat` is
    /// stamped on all the characters, unless zero is passed in which case it
    /// chooses `spacer` or `other_char` automatically.
    // §499
    pub fn str_toks(&mut self, mut b: pool_pointer) -> halfword {
        let mut str_toks: halfword = 0;
        str_toks = self.str_toks_cat(b, 0i32);
        str_toks
    }

    /// The main reason for wanting `str_toks` is the next function,
    /// `the_toks`, which has similar input/output characteristics.
    /// This procedure is supposed to scan something like `\.{\\skip\\count12}',
    /// i.e., whatever can follow `\.{\\the}', and it constructs a token list
    /// containing something like `\.{-3.0pt minus 0.5fill}'.
    // §500
    pub fn the_toks(&mut self) -> halfword {
        let mut the_toks: halfword = 0;
        let mut old_setting: i32 = 0; // §500
        let mut p: halfword = 0; // §500
        let mut q: halfword = 0; // §500
        let mut r: halfword = 0; // §500
        let mut b: pool_pointer = 0; // §500
        let mut c: small_number = 0; // §500
        'l_exit_f: {
            // §1498
            if (((self.cur_chr) % 2) != 0) {
                {
                    c = self.cur_chr;
                    self.scan_general_text();
                    if (c == 1i32) {
                        the_toks = self.cur_val;
                    } else {
                        {
                            old_setting = self.selector;
                            self.selector = new_string;
                            b = self.pool_ptr;
                            p = self.get_avail();
                            { let __v348 = self.mem[crate::ix::U((temp_head) as usize)].hh().rh(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v348); }
                            self.token_show(p);
                            self.flush_list(p);
                            self.selector = old_setting;
                            the_toks = self.str_toks(b);
                        }
                    }
                    break 'l_exit_f;
                }
            }
            // §500
            self.get_x_token();
            self.scan_something_internal(tok_val, false);
            if (self.cur_val_level >= ident_val) {
                // §501
                {
                    p = temp_head;
                    self.mem[crate::ix::U((p) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                    if (self.cur_val_level == ident_val) {
                        {
                            q = self.get_avail();
                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                            { let __v349 = (cs_token_flag).wrapping_add(self.cur_val); self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v349); }
                            p = q;
                        }
                    } else {
                        if (self.cur_val != (268435455i32).wrapping_neg()) {
                            {
                                r = self.mem[crate::ix::U((self.cur_val) as usize)].hh().rh();
                                while (r != (268435455i32).wrapping_neg()) {
                                    {
                                        {
                                            {
                                                q = self.avail;
                                                if (q == (268435455i32).wrapping_neg()) {
                                                    q = self.get_avail();
                                                } else {
                                                    {
                                                        self.avail = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                        self.mem[crate::ix::U((q) as usize)].set_hh_rh((268435455i32).wrapping_neg());
                                                        self.dyn_used = (self.dyn_used).wrapping_add(1i32);
                                                    }
                                                }
                                            }
                                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                            { let __v350 = self.mem[crate::ix::U((r) as usize)].hh().lh(); self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v350); }
                                            p = q;
                                        }
                                        r = self.mem[crate::ix::U((r) as usize)].hh().rh();
                                    }
                                }
                            }
                        }
                    }
                    the_toks = p;
                }
            } else {
                // §500
                {
                    old_setting = self.selector;
                    self.selector = new_string;
                    b = self.pool_ptr;
                    match self.cur_val_level {
                        int_val => {
                            self.print_int(self.cur_val);
                        }
                        dimen_val => {
                            {
                                self.print_scaled(self.cur_val);
                                self.print(65689i32);
                            }
                        }
                        glue_val => {
                            {
                                self.print_spec(self.cur_val, 65689i32);
                                self.delete_glue_ref(self.cur_val);
                            }
                        }
                        mu_val => {
                            {
                                self.print_spec(self.cur_val, 65625i32);
                                self.delete_glue_ref(self.cur_val);
                            }
                        }
                        _ => {}
                    }
                    self.selector = old_setting;
                    the_toks = self.str_toks(b);
                }
            }
        }
        the_toks
    }

    /// Here's part of the `expand` subroutine that we are now ready to complete:
    // §502
    pub fn ins_the_toks(&mut self) {
        { let __v351 = self.the_toks(); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v351); }
        self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
    }

    /// The procedure `conv_toks` uses `str_toks` to insert the token list
    /// for `convert` functions into the scanner; `\.{\\outer}' control sequences
    /// are allowed to follow `\.{\\string}' and `\.{\\meaning}'.
    /// The extra temp string `u` is needed because `pdf_scan_ext_toks` incorporates
    /// any pending string in its output. In order to save such a pending string,
    /// we have to create a temporary string that is destroyed immediately after.
    // §505
    pub fn conv_toks(&mut self) {
        let mut old_setting: i32 = 0; // §505
        let mut save_warning_index: halfword = 0; // §505
        let mut save_def_ref: halfword = 0; // §505
        let mut boolvar: bool = false; // §505
        let mut s: str_number = 0; // §505
        let mut u: str_number = 0; // §505
        let mut j: i32 = 0; // §505
        let mut c: small_number = 0; // §505
        let mut save_scanner_status: small_number = 0; // §505
        let mut b: pool_pointer = 0; // §505
        let mut fnt: i32 = 0; // §505
        let mut arg1: i32 = 0; // §505
        let mut arg2: i32 = 0; // §505
        let mut font_name_str: str_number = 0; // §505
        let mut i: small_number = 0; // §505
        let mut quote_char: UTF16_code = 0; // §505
        let mut cat: small_number = 0; // §505
        let mut saved_chr: UnicodeScalar = 0; // §505
        let mut p: halfword = 0; // §505
        let mut q: halfword = 0; // §505
        cat = 0i32;
        c = self.cur_chr;
        // §506
        match c {
            number_code | roman_numeral_code => {
                self.scan_int();
            }
            string_code | meaning_code => {
                {
                    save_scanner_status = self.scanner_status;
                    self.scanner_status = normal;
                    self.get_token();
                    self.scanner_status = save_scanner_status;
                }
            }
            font_name_code => {
                self.scan_font_ident();
            }
            eTeX_revision_code => {
            }
            expanded_code => {
                {
                    save_scanner_status = self.scanner_status;
                    save_warning_index = self.warning_index;
                    save_def_ref = self.def_ref;
                    if (self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)] < self.pool_ptr) {
                        u = self.make_string();
                    } else {
                        u = 0i32;
                    }
                    self.scan_pdf_ext_toks();
                    self.warning_index = save_warning_index;
                    self.scanner_status = save_scanner_status;
                    self.begin_token_list(self.mem[crate::ix::U((self.def_ref) as usize)].hh().rh(), inserted);
                    {
                        { let __ix352 = self.def_ref; let __v353 = self.avail; self.mem[crate::ix::U((__ix352) as usize)].set_hh_rh(__v353); }
                        self.avail = self.def_ref;
                        self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                    }
                    self.def_ref = save_def_ref;
                    if (u != 0i32) {
                        self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                    }
                    return;
                }
            }
            left_margin_kern_code | right_margin_kern_code => {
                {
                    self.scan_register_num();
                    if (self.cur_val < 256i32) {
                        p = self.eqtb[crate::ix::U((((box_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                    } else {
                        {
                            self.find_sa_element(box_val, self.cur_val, false);
                            if (self.cur_ptr == (268435455i32).wrapping_neg()) {
                                p = (268435455i32).wrapping_neg();
                            } else {
                                p = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                            }
                        }
                    }
                    if ((p == (268435455i32).wrapping_neg()) || (self.mem[crate::ix::U((p) as usize)].hh().b0() != hlist_node)) {
                        self.pdf_error(66107i32, 66108i32);
                    }
                }
            }
            pdf_creation_date_code => {
                {
                    b = self.pool_ptr;
                    self.getcreationdate();
                    { let __v354 = self.str_toks(b); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v354); }
                    self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                    return;
                }
            }
            pdf_file_mod_date_code => {
                {
                    save_scanner_status = self.scanner_status;
                    save_warning_index = self.warning_index;
                    save_def_ref = self.def_ref;
                    if (self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)] < self.pool_ptr) {
                        u = self.make_string();
                    } else {
                        u = 0i32;
                    }
                    self.scan_pdf_ext_toks();
                    if (self.selector == new_string) {
                        self.pdf_error(66109i32, 66110i32);
                    }
                    old_setting = self.selector;
                    self.selector = new_string;
                    self.show_token_list(self.mem[crate::ix::U((self.def_ref) as usize)].hh().rh(), (268435455i32).wrapping_neg(), (pool_size).wrapping_sub(self.pool_ptr));
                    self.selector = old_setting;
                    s = self.make_string();
                    self.delete_token_ref(self.def_ref);
                    self.def_ref = save_def_ref;
                    self.warning_index = save_warning_index;
                    self.scanner_status = save_scanner_status;
                    b = self.pool_ptr;
                    self.getfilemoddate(s);
                    { let __v355 = self.str_toks(b); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v355); }
                    if (s == (self.str_ptr).wrapping_sub(1i32)) {
                        {
                            self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                            self.pool_ptr = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
                        }
                    }
                    self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                    if (u != 0i32) {
                        self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                    }
                    return;
                }
            }
            pdf_file_size_code => {
                {
                    save_scanner_status = self.scanner_status;
                    save_warning_index = self.warning_index;
                    save_def_ref = self.def_ref;
                    if (self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)] < self.pool_ptr) {
                        u = self.make_string();
                    } else {
                        u = 0i32;
                    }
                    self.scan_pdf_ext_toks();
                    if (self.selector == new_string) {
                        self.pdf_error(66109i32, 66110i32);
                    }
                    old_setting = self.selector;
                    self.selector = new_string;
                    self.show_token_list(self.mem[crate::ix::U((self.def_ref) as usize)].hh().rh(), (268435455i32).wrapping_neg(), (pool_size).wrapping_sub(self.pool_ptr));
                    self.selector = old_setting;
                    s = self.make_string();
                    self.delete_token_ref(self.def_ref);
                    self.def_ref = save_def_ref;
                    self.warning_index = save_warning_index;
                    self.scanner_status = save_scanner_status;
                    b = self.pool_ptr;
                    self.getfilesize(s);
                    { let __v356 = self.str_toks(b); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v356); }
                    if (s == (self.str_ptr).wrapping_sub(1i32)) {
                        {
                            self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                            self.pool_ptr = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
                        }
                    }
                    self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                    if (u != 0i32) {
                        self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                    }
                    return;
                }
            }
            pdf_mdfive_sum_code => {
                {
                    save_scanner_status = self.scanner_status;
                    save_warning_index = self.warning_index;
                    save_def_ref = self.def_ref;
                    if (self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)] < self.pool_ptr) {
                        u = self.make_string();
                    } else {
                        u = 0i32;
                    }
                    boolvar = self.scan_keyword(66111i32);
                    self.scan_pdf_ext_toks();
                    if (self.selector == new_string) {
                        self.pdf_error(66109i32, 66110i32);
                    }
                    old_setting = self.selector;
                    self.selector = new_string;
                    self.show_token_list(self.mem[crate::ix::U((self.def_ref) as usize)].hh().rh(), (268435455i32).wrapping_neg(), (pool_size).wrapping_sub(self.pool_ptr));
                    self.selector = old_setting;
                    s = self.make_string();
                    self.delete_token_ref(self.def_ref);
                    self.def_ref = save_def_ref;
                    self.warning_index = save_warning_index;
                    self.scanner_status = save_scanner_status;
                    b = self.pool_ptr;
                    self.getmd5sum(s, boolvar);
                    { let __v357 = self.str_toks(b); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v357); }
                    if (s == (self.str_ptr).wrapping_sub(1i32)) {
                        {
                            self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                            self.pool_ptr = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
                        }
                    }
                    self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                    if (u != 0i32) {
                        self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                    }
                    return;
                }
            }
            pdf_file_dump_code => {
                {
                    save_scanner_status = self.scanner_status;
                    save_warning_index = self.warning_index;
                    save_def_ref = self.def_ref;
                    if (self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)] < self.pool_ptr) {
                        u = self.make_string();
                    } else {
                        u = 0i32;
                    }
                    self.cur_val = 0i32;
                    if self.scan_keyword(66112i32) {
                        {
                            self.scan_int();
                            if (self.cur_val < 0i32) {
                                {
                                    {
                                        if (self.interaction == error_stop_mode) {
                                        }
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(65544i32);
                                        }
                                        self.print(66113i32);
                                    }
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 66114i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 65996i32;
                                    }
                                    self.int_error(self.cur_val);
                                    self.cur_val = 0i32;
                                }
                            }
                        }
                    }
                    i = self.cur_val;
                    self.cur_val = 0i32;
                    if self.scan_keyword(66115i32) {
                        {
                            self.scan_int();
                            if (self.cur_val < 0i32) {
                                {
                                    {
                                        if (self.interaction == error_stop_mode) {
                                        }
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(65544i32);
                                        }
                                        self.print(66116i32);
                                    }
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 66117i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 65996i32;
                                    }
                                    self.int_error(self.cur_val);
                                    self.cur_val = 0i32;
                                }
                            }
                        }
                    }
                    j = self.cur_val;
                    self.scan_pdf_ext_toks();
                    if (self.selector == new_string) {
                        self.pdf_error(66109i32, 66110i32);
                    }
                    old_setting = self.selector;
                    self.selector = new_string;
                    self.show_token_list(self.mem[crate::ix::U((self.def_ref) as usize)].hh().rh(), (268435455i32).wrapping_neg(), (pool_size).wrapping_sub(self.pool_ptr));
                    self.selector = old_setting;
                    s = self.make_string();
                    self.delete_token_ref(self.def_ref);
                    self.def_ref = save_def_ref;
                    self.warning_index = save_warning_index;
                    self.scanner_status = save_scanner_status;
                    b = self.pool_ptr;
                    self.getfiledump(s, i, j);
                    { let __v358 = self.str_toks(b); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v358); }
                    if (s == (self.str_ptr).wrapping_sub(1i32)) {
                        {
                            self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                            self.pool_ptr = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
                        }
                    }
                    self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
                    if (u != 0i32) {
                        self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                    }
                    return;
                }
            }
            pdf_strcmp_code => {
                {
                    save_scanner_status = self.scanner_status;
                    save_warning_index = self.warning_index;
                    save_def_ref = self.def_ref;
                    if (self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)] < self.pool_ptr) {
                        u = self.make_string();
                    } else {
                        u = 0i32;
                    }
                    self.compare_strings();
                    self.def_ref = save_def_ref;
                    self.warning_index = save_warning_index;
                    self.scanner_status = save_scanner_status;
                    if (u != 0i32) {
                        self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                    }
                }
            }
            XeTeX_Uchar_code => {
                self.scan_usv_num();
            }
            XeTeX_Ucharcat_code => {
                {
                    self.scan_usv_num();
                    saved_chr = self.cur_val;
                    self.scan_int();
                    if ((((self.cur_val < left_brace) || (self.cur_val > active_char)) || (self.cur_val == out_param)) || (self.cur_val == ignore)) {
                        {
                            {
                                if (self.interaction == error_stop_mode) {
                                }
                                if self.file_line_error_style_p {
                                    self.print_file_line();
                                } else {
                                    self.print_nl(65544i32);
                                }
                                self.print(66118i32);
                            }
                            self.print_int(self.cur_val);
                            self.print(66119i32);
                            {
                                self.help_ptr = 1i32;
                                self.help_line[crate::ix::U((0i32) as usize)] = 66120i32;
                            }
                            self.error();
                            cat = 12i32;
                        }
                    } else {
                        cat = self.cur_val;
                    }
                    self.cur_val = saved_chr;
                }
            }
            XeTeX_revision_code => {
                // §1460
            }
            XeTeX_variation_name_code => {
                {
                    self.scan_font_ident();
                    fnt = self.cur_val;
                    if (self.font_area[crate::ix::U((fnt) as usize)] == aat_font_flag) {
                        {
                            self.scan_int();
                            arg1 = self.cur_val;
                            arg2 = 0i32;
                        }
                    } else {
                        self.not_aat_font_error(convert, c, fnt);
                    }
                }
            }
            XeTeX_feature_name_code => {
                {
                    self.scan_font_ident();
                    fnt = self.cur_val;
                    if ((self.font_area[crate::ix::U((fnt) as usize)] == aat_font_flag) || ((self.font_area[crate::ix::U((fnt) as usize)] == otgr_font_flag) && self.usingGraphite(self.font_layout_engine[crate::ix::U((fnt) as usize)]))) {
                        {
                            self.scan_int();
                            arg1 = self.cur_val;
                            arg2 = 0i32;
                        }
                    } else {
                        self.not_aat_gr_font_error(convert, c, fnt);
                    }
                }
            }
            XeTeX_selector_name_code => {
                {
                    self.scan_font_ident();
                    fnt = self.cur_val;
                    if ((self.font_area[crate::ix::U((fnt) as usize)] == aat_font_flag) || ((self.font_area[crate::ix::U((fnt) as usize)] == otgr_font_flag) && self.usingGraphite(self.font_layout_engine[crate::ix::U((fnt) as usize)]))) {
                        {
                            self.scan_int();
                            arg1 = self.cur_val;
                            self.scan_int();
                            arg2 = self.cur_val;
                        }
                    } else {
                        self.not_aat_gr_font_error(convert, c, fnt);
                    }
                }
            }
            XeTeX_glyph_name_code => {
                {
                    self.scan_font_ident();
                    fnt = self.cur_val;
                    if ((self.font_area[crate::ix::U((fnt) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((fnt) as usize)] == otgr_font_flag)) {
                        {
                            self.scan_int();
                            arg1 = self.cur_val;
                        }
                    } else {
                        self.not_native_font_error(convert, c, fnt);
                    }
                }
            }
            job_name_code => {
                // §506
                if (self.job_name == 0i32) {
                    self.open_log_file();
                }
            }
            uniform_deviate_code => {
                self.scan_int();
            }
            normal_deviate_code => {
            }
            _ => {}
        }
        // §505
        old_setting = self.selector;
        self.selector = new_string;
        b = self.pool_ptr;
        // §507
        match c {
            number_code => {
                self.print_int(self.cur_val);
            }
            roman_numeral_code => {
                self.print_roman_int(self.cur_val);
            }
            string_code => {
                if (self.cur_cs != 0i32) {
                    self.sprint_cs(self.cur_cs);
                } else {
                    self.print_char(self.cur_chr);
                }
            }
            meaning_code => {
                self.print_meaning();
            }
            font_name_code => {
                {
                    font_name_str = self.font_name[crate::ix::U((self.cur_val) as usize)];
                    if ((self.font_area[crate::ix::U((self.cur_val) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((self.cur_val) as usize)] == otgr_font_flag)) {
                        {
                            quote_char = 34i32;
                            {
                                let __for_end_7 = (self.length(font_name_str)).wrapping_sub(1i32);
                                i = 0i32;
                                while i <= __for_end_7 {
                                    if (self.str_pool[crate::ix::U(((self.str_start[crate::ix::U(((font_name_str).wrapping_sub(65536i32)) as usize)]).wrapping_add(i)) as usize)] == 34i32) {
                                        quote_char = 39i32;
                                    }
                                    i = i.wrapping_add(1);
                                }
                            }
                            self.print_char(quote_char);
                            self.print(font_name_str);
                            self.print_char(quote_char);
                        }
                    } else {
                        self.print(font_name_str);
                    }
                    if (self.font_size[crate::ix::U((self.cur_val) as usize)] != self.font_dsize[crate::ix::U((self.cur_val) as usize)]) {
                        {
                            self.print(66121i32);
                            self.print_scaled(self.font_size[crate::ix::U((self.cur_val) as usize)]);
                            self.print(65689i32);
                        }
                    }
                }
            }
            eTeX_revision_code => {
                self.print(eTeX_revision);
            }
            left_margin_kern_code => {
                {
                    p = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh();
                    while ((p != (268435455i32).wrapping_neg()) && (((!(p >= self.hi_mem_min)) && (((((((((self.mem[crate::ix::U((p) as usize)].hh().b0() == ins_node) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == mark_node)) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == adjust_node)) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == penalty_node)) || ((((self.mem[crate::ix::U((p) as usize)].hh().b0() == disc_node) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == (268435455i32).wrapping_neg())) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg())) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == 0i32))) || ((self.mem[crate::ix::U((p) as usize)].hh().b0() == math_node) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() == 0i32))) || ((self.mem[crate::ix::U((p) as usize)].hh().b0() == kern_node) && ((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() == 0i32) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == normal)))) || ((self.mem[crate::ix::U((p) as usize)].hh().b0() == glue_node) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == zero_glue))) || (((((self.mem[crate::ix::U((p) as usize)].hh().b0() == hlist_node) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg())))) || (((!(p >= self.hi_mem_min)) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == glue_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == 8i32)))) {
                        p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                    }
                    if ((((p != (268435455i32).wrapping_neg()) && (!(p >= self.hi_mem_min))) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == margin_kern_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == left_side)) {
                        self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                    } else {
                        self.print(48i32);
                    }
                    self.print(65689i32);
                }
            }
            right_margin_kern_code => {
                {
                    q = self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh();
                    p = self.prev_rightmost(q, (268435455i32).wrapping_neg());
                    while ((p != (268435455i32).wrapping_neg()) && (((!(p >= self.hi_mem_min)) && (((((((((self.mem[crate::ix::U((p) as usize)].hh().b0() == ins_node) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == mark_node)) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == adjust_node)) || (self.mem[crate::ix::U((p) as usize)].hh().b0() == penalty_node)) || ((((self.mem[crate::ix::U((p) as usize)].hh().b0() == disc_node) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == (268435455i32).wrapping_neg())) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg())) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == 0i32))) || ((self.mem[crate::ix::U((p) as usize)].hh().b0() == math_node) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() == 0i32))) || ((self.mem[crate::ix::U((p) as usize)].hh().b0() == kern_node) && ((self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() == 0i32) || (self.mem[crate::ix::U((p) as usize)].hh().b1() == normal)))) || ((self.mem[crate::ix::U((p) as usize)].hh().b0() == glue_node) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == zero_glue))) || (((((self.mem[crate::ix::U((p) as usize)].hh().b0() == hlist_node) && (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((p).wrapping_add(3i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((p).wrapping_add(2i32)) as usize)].int() == 0i32)) && (self.mem[crate::ix::U(((p).wrapping_add(5i32)) as usize)].hh().rh() == (268435455i32).wrapping_neg())))) || (((!(p >= self.hi_mem_min)) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == glue_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == 9i32)))) {
                        p = self.prev_rightmost(q, p);
                    }
                    if ((((p != (268435455i32).wrapping_neg()) && (!(p >= self.hi_mem_min))) && (self.mem[crate::ix::U((p) as usize)].hh().b0() == margin_kern_node)) && (self.mem[crate::ix::U((p) as usize)].hh().b1() == right_side)) {
                        self.print_scaled(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int());
                    } else {
                        self.print(48i32);
                    }
                    self.print(65689i32);
                }
            }
            pdf_strcmp_code => {
                self.print_int(self.cur_val);
            }
            uniform_deviate_code => {
                { let __a359_0 = self.unif_rand(self.cur_val); self.print_int(__a359_0) };
            }
            normal_deviate_code => {
                { let __a360_0 = self.norm_rand(); self.print_int(__a360_0) };
            }
            XeTeX_Uchar_code | XeTeX_Ucharcat_code => {
                self.print_char(self.cur_val);
            }
            XeTeX_revision_code => {
                // §1461
                self.print(XeTeX_revision);
            }
            XeTeX_variation_name_code => {
                if (self.font_area[crate::ix::U((fnt) as usize)] == aat_font_flag) {
                    self.aat_print_font_name(c, self.font_layout_engine[crate::ix::U((fnt) as usize)], arg1, arg2);
                }
            }
            XeTeX_feature_name_code | XeTeX_selector_name_code => {
                if (self.font_area[crate::ix::U((fnt) as usize)] == aat_font_flag) {
                    self.aat_print_font_name(c, self.font_layout_engine[crate::ix::U((fnt) as usize)], arg1, arg2);
                } else {
                    if ((self.font_area[crate::ix::U((fnt) as usize)] == otgr_font_flag) && self.usingGraphite(self.font_layout_engine[crate::ix::U((fnt) as usize)])) {
                        self.gr_print_font_name(c, self.font_layout_engine[crate::ix::U((fnt) as usize)], arg1, arg2);
                    }
                }
            }
            XeTeX_glyph_name_code => {
                if ((self.font_area[crate::ix::U((fnt) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((fnt) as usize)] == otgr_font_flag)) {
                    self.print_glyph_name(fnt, arg1);
                }
            }
            job_name_code => {
                // §507
                self.print_file_name(self.job_name, 0i32, 0i32);
            }
            _ => {}
        }
        // §505
        self.selector = old_setting;
        { let __v361 = self.str_toks_cat(b, cat); self.mem[crate::ix::U((garbage) as usize)].set_hh_rh(__v361); }
        self.begin_token_list(self.mem[crate::ix::U((temp_head) as usize)].hh().rh(), inserted);
    }

    /// Now we can't postpone the difficulties any longer; we must bravely tackle
    /// `scan_toks`. This function returns a pointer to the tail of a new token
    /// list, and it also makes `def_ref` point to the reference count at the
    /// head of that list.
    /// There are two boolean parameters, `macro_def` and `xpand`. If `macro_def`
    /// is true, the goal is to create the token list for a macro definition;
    /// otherwise the goal is to create the token list for some other \TeX\
    /// primitive: \.{\\mark}, \.{\\output}, \.{\\everypar}, \.{\\lowercase},
    /// \.{\\uppercase}, \.{\\message}, \.{\\errmessage}, \.{\\write}, or
    /// \.{\\special}. In the latter cases a left brace must be scanned next; this
    /// left brace will not be part of the token list, nor will the matching right
    /// brace that comes at the end. If `xpand` is false, the token list will
    /// simply be copied from the input using `get_token`. Otherwise all expandable
    /// tokens will be expanded until unexpandable tokens are left, except that
    /// ...
    // §508
    pub fn scan_toks(&mut self, mut macro_def: bool, mut xpand: bool) -> halfword {
        let mut scan_toks: halfword = 0;
        let mut t: halfword = 0; // §508
        let mut s: halfword = 0; // §508
        let mut p: halfword = 0; // §508
        let mut q: halfword = 0; // §508
        let mut unbalance: halfword = 0; // §508
        let mut hash_brace: halfword = 0; // §508
        'l_found_f: {
            if macro_def {
                self.scanner_status = defining;
            } else {
                self.scanner_status = absorbing;
            }
            self.warning_index = self.cur_cs;
            self.def_ref = self.get_avail();
            { let __ix362 = self.def_ref; self.mem[crate::ix::U((__ix362) as usize)].set_hh_lh((268435455i32).wrapping_neg()); }
            p = self.def_ref;
            hash_brace = 0i32;
            t = zero_token;
            if macro_def {
                // §509
                {
                    'l_done_f: {
                        'l_done1_f: {
                            while true {
                                {
                                    'l_continue_b: loop {
                                        self.get_token();
                                        if (self.cur_tok < right_brace_limit) {
                                            break 'l_done1_f;
                                        }
                                        if (self.cur_cmd == mac_param) {
                                            // §511
                                            {
                                                s = (match_token).wrapping_add(self.cur_chr);
                                                self.get_token();
                                                if (self.cur_tok < left_brace_limit) {
                                                    {
                                                        hash_brace = self.cur_tok;
                                                        {
                                                            q = self.get_avail();
                                                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                                            { let __v363 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v363); }
                                                            p = q;
                                                        }
                                                        {
                                                            q = self.get_avail();
                                                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                                            self.mem[crate::ix::U((q) as usize)].set_hh_lh(end_match_token);
                                                            p = q;
                                                        }
                                                        break 'l_done_f;
                                                    }
                                                }
                                                if (t == 25165881i32) {
                                                    {
                                                        {
                                                            if (self.interaction == error_stop_mode) {
                                                            }
                                                            if self.file_line_error_style_p {
                                                                self.print_file_line();
                                                            } else {
                                                                self.print_nl(65544i32);
                                                            }
                                                            self.print(66124i32);
                                                        }
                                                        {
                                                            self.help_ptr = 2i32;
                                                            self.help_line[crate::ix::U((1i32) as usize)] = 66125i32;
                                                            self.help_line[crate::ix::U((0i32) as usize)] = 66126i32;
                                                        }
                                                        self.error();
                                                        continue 'l_continue_b;
                                                    }
                                                } else {
                                                    {
                                                        t = (t).wrapping_add(1i32);
                                                        if (self.cur_tok != t) {
                                                            {
                                                                {
                                                                    if (self.interaction == error_stop_mode) {
                                                                    }
                                                                    if self.file_line_error_style_p {
                                                                        self.print_file_line();
                                                                    } else {
                                                                        self.print_nl(65544i32);
                                                                    }
                                                                    self.print(66127i32);
                                                                }
                                                                {
                                                                    self.help_ptr = 2i32;
                                                                    self.help_line[crate::ix::U((1i32) as usize)] = 66128i32;
                                                                    self.help_line[crate::ix::U((0i32) as usize)] = 66129i32;
                                                                }
                                                                self.back_error();
                                                            }
                                                        }
                                                        self.cur_tok = s;
                                                    }
                                                }
                                            }
                                        }
                                        // §509
                                        {
                                            q = self.get_avail();
                                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                            { let __v364 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v364); }
                                            p = q;
                                        }
                                        break 'l_continue_b;
                                    }
                                }
                            }
                        }
                        {
                            q = self.get_avail();
                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                            self.mem[crate::ix::U((q) as usize)].set_hh_lh(end_match_token);
                            p = q;
                        }
                        if (self.cur_cmd == right_brace) {
                            // §510
                            {
                                {
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(65981i32);
                                }
                                self.align_state = (self.align_state).wrapping_add(1i32);
                                {
                                    self.help_ptr = 2i32;
                                    self.help_line[crate::ix::U((1i32) as usize)] = 66122i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66123i32;
                                }
                                self.error();
                                break 'l_found_f;
                            }
                        }
                    }
                    // §509
                }
            } else {
                // §508
                self.scan_left_brace();
            }
            // §512
            unbalance = 1i32;
            while true {
                {
                    if xpand {
                        // §513
                        {
                            'l_done2_f: {
                                while true {
                                    {
                                        self.get_next();
                                        if (self.cur_cmd >= call) {
                                            if (self.mem[crate::ix::U((self.mem[crate::ix::U((self.cur_chr) as usize)].hh().rh()) as usize)].hh().lh() == protected_token) {
                                                {
                                                    self.cur_cmd = relax;
                                                    self.cur_chr = no_expand_flag;
                                                }
                                            }
                                        }
                                        if (self.cur_cmd <= max_command) {
                                            break 'l_done2_f;
                                        }
                                        if (self.cur_cmd != the) {
                                            self.expand();
                                        } else {
                                            {
                                                q = self.the_toks();
                                                if (self.mem[crate::ix::U((temp_head) as usize)].hh().rh() != (268435455i32).wrapping_neg()) {
                                                    {
                                                        { let __v365 = self.mem[crate::ix::U((temp_head) as usize)].hh().rh(); self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v365); }
                                                        p = q;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            self.x_token();
                        }
                    } else {
                        // §512
                        self.get_token();
                    }
                    if (self.cur_tok < right_brace_limit) {
                        if (self.cur_cmd < right_brace) {
                            unbalance = (unbalance).wrapping_add(1i32);
                        } else {
                            {
                                unbalance = (unbalance).wrapping_sub(1i32);
                                if (unbalance == 0i32) {
                                    break 'l_found_f;
                                }
                            }
                        }
                    } else {
                        if (self.cur_cmd == mac_param) {
                            if macro_def {
                                // §514
                                {
                                    s = self.cur_tok;
                                    if xpand {
                                        self.get_x_token();
                                    } else {
                                        self.get_token();
                                    }
                                    if (self.cur_cmd != mac_param) {
                                        if ((self.cur_tok <= zero_token) || (self.cur_tok > t)) {
                                            {
                                                {
                                                    if (self.interaction == error_stop_mode) {
                                                    }
                                                    if self.file_line_error_style_p {
                                                        self.print_file_line();
                                                    } else {
                                                        self.print_nl(65544i32);
                                                    }
                                                    self.print(66130i32);
                                                }
                                                self.sprint_cs(self.warning_index);
                                                {
                                                    self.help_ptr = 3i32;
                                                    self.help_line[crate::ix::U((2i32) as usize)] = 66131i32;
                                                    self.help_line[crate::ix::U((1i32) as usize)] = 66132i32;
                                                    self.help_line[crate::ix::U((0i32) as usize)] = 66133i32;
                                                }
                                                self.back_error();
                                                self.cur_tok = s;
                                            }
                                        } else {
                                            self.cur_tok = (10485712i32).wrapping_add(self.cur_chr);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // §512
                    {
                        q = self.get_avail();
                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                        { let __v366 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v366); }
                        p = q;
                    }
                }
            }
        }
        // §508
        self.scanner_status = normal;
        if (hash_brace != 0i32) {
            {
                q = self.get_avail();
                self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                self.mem[crate::ix::U((q) as usize)].set_hh_lh(hash_brace);
                p = q;
            }
        }
        scan_toks = p;
        scan_toks
    }

    /// The `read_toks` procedure constructs a token list like that for any
    /// macro definition, and makes `cur_val` point to it. Parameter `r` points
    /// to the control sequence that will receive this token list.
    // §517
    pub fn read_toks(&mut self, mut n: i32, mut r: halfword, mut j: halfword) {
        let mut p: halfword = 0; // §517
        let mut q: halfword = 0; // §517
        let mut s: i32 = 0; // §517
        let mut m: small_number = 0; // §517
        self.scanner_status = defining;
        self.warning_index = r;
        self.def_ref = self.get_avail();
        { let __ix367 = self.def_ref; self.mem[crate::ix::U((__ix367) as usize)].set_hh_lh((268435455i32).wrapping_neg()); }
        p = self.def_ref;
        {
            q = self.get_avail();
            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
            self.mem[crate::ix::U((q) as usize)].set_hh_lh(end_match_token);
            p = q;
        }
        if ((n < 0i32) || (n > 15i32)) {
            m = 16i32;
        } else {
            m = n;
        }
        s = self.align_state;
        self.align_state = 1000000i32;
        loop {
            'l_done_f: {
                // §518
                self.begin_file_reading();
                self.cur_input.name_field = (m).wrapping_add(1i32);
                if (self.read_open[crate::ix::U((m) as usize)] == closed) {
                    // §519
                    if (self.interaction > nonstop_mode) {
                        if (n < 0i32) {
                            {
                                self.print(65626i32);
                                self.term_input();
                            }
                        } else {
                            {
                                self.print_ln();
                                self.sprint_cs(r);
                                {
                                    self.print(61i32);
                                    self.term_input();
                                }
                                n = (1i32).wrapping_neg();
                            }
                        }
                    } else {
                        {
                            self.cur_input.limit_field = 0i32;
                            self.fatal_error(66134i32);
                        }
                    }
                } else {
                    // §518
                    if (self.read_open[crate::ix::U((m) as usize)] == just_open) {
                        // §520
                        if { let mut __f0 = ::core::mem::take(&mut self.read_file[crate::ix::U((m) as usize)]); let __r = self.input_ln(&mut __f0, false); self.read_file[crate::ix::U((m) as usize)] = __f0; __r } {
                            self.read_open[crate::ix::U((m) as usize)] = normal;
                        } else {
                            {
                                { let mut __f0 = ::core::mem::take(&mut self.read_file[crate::ix::U((m) as usize)]); let __r = self.u_close(&mut __f0); self.read_file[crate::ix::U((m) as usize)] = __f0; __r };
                                self.read_open[crate::ix::U((m) as usize)] = closed;
                            }
                        }
                    } else {
                        // §521
                        {
                            if (!{ let mut __f0 = ::core::mem::take(&mut self.read_file[crate::ix::U((m) as usize)]); let __r = self.input_ln(&mut __f0, true); self.read_file[crate::ix::U((m) as usize)] = __f0; __r }) {
                                {
                                    { let mut __f0 = ::core::mem::take(&mut self.read_file[crate::ix::U((m) as usize)]); let __r = self.u_close(&mut __f0); self.read_file[crate::ix::U((m) as usize)] = __f0; __r };
                                    self.read_open[crate::ix::U((m) as usize)] = closed;
                                    if (self.align_state != 1000000i32) {
                                        {
                                            self.runaway();
                                            {
                                                if (self.interaction == error_stop_mode) {
                                                }
                                                if self.file_line_error_style_p {
                                                    self.print_file_line();
                                                } else {
                                                    self.print_nl(65544i32);
                                                }
                                                self.print(66135i32);
                                            }
                                            self.print_esc(65850i32);
                                            {
                                                self.help_ptr = 1i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] = 66136i32;
                                            }
                                            self.align_state = 1000000i32;
                                            self.cur_input.limit_field = 0i32;
                                            self.error();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // §518
                self.cur_input.limit_field = self.last;
                if ((self.eqtb[crate::ix::U(((7892312i32) - 1) as usize)].int() < 0i32) || (self.eqtb[crate::ix::U(((7892312i32) - 1) as usize)].int() > 255i32)) {
                    self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(1i32);
                } else {
                    { let __ix368 = self.cur_input.limit_field; let __v369 = self.eqtb[crate::ix::U(((7892312i32) - 1) as usize)].int(); self.buffer[crate::ix::U((__ix368) as usize)] = __v369; }
                }
                self.first = (self.cur_input.limit_field).wrapping_add(1i32);
                self.cur_input.loc_field = self.cur_input.start_field;
                self.cur_input.state_field = new_line;
                // §1572
                if (j == 1i32) {
                    {
                        while (self.cur_input.loc_field <= self.cur_input.limit_field) {
                            {
                                self.cur_chr = self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)];
                                self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(1i32);
                                if (self.cur_chr == 32i32) {
                                    self.cur_tok = space_token;
                                } else {
                                    self.cur_tok = (self.cur_chr).wrapping_add(25165824i32);
                                }
                                {
                                    q = self.get_avail();
                                    self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                    { let __v370 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v370); }
                                    p = q;
                                }
                            }
                        }
                        break 'l_done_f;
                    }
                }
                // §518
                while true {
                    {
                        self.get_token();
                        if (self.cur_tok == 0i32) {
                            break 'l_done_f;
                        }
                        if (self.align_state < 1000000i32) {
                            {
                                loop {
                                    self.get_token();
                                    if (self.cur_tok == 0i32) { break; }
                                }
                                self.align_state = 1000000i32;
                                break 'l_done_f;
                            }
                        }
                        {
                            q = self.get_avail();
                            self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                            { let __v371 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v371); }
                            p = q;
                        }
                    }
                }
            }
            self.end_file_reading();
            if (self.align_state == 1000000i32) { break; }
        }
        // §517
        self.cur_val = self.def_ref;
        self.scanner_status = normal;
        self.align_state = s;
    }

    /// Here is a procedure that ignores text until coming to an \.{\\or},
    /// \.{\\else}, or \.{\\fi} at the current level of $\.{\\if}\ldots\.{\\fi}$
    /// nesting. After it has acted, `cur_chr` will indicate the token that
    /// was found, but `cur_tok` will not be set (because this makes the
    /// procedure run faster).
    // §529
    pub fn pass_text(&mut self) {
        let mut l: i32 = 0; // §529
        let mut save_scanner_status: small_number = 0; // §529
        'l_done_f: {
            save_scanner_status = self.scanner_status;
            self.scanner_status = skipping;
            l = 0i32;
            self.skip_line = self.line;
            while true {
                {
                    self.get_next();
                    if (self.cur_cmd == fi_or_else) {
                        {
                            if (l == 0i32) {
                                break 'l_done_f;
                            }
                            if (self.cur_chr == fi_code) {
                                l = (l).wrapping_sub(1i32);
                            }
                        }
                    } else {
                        if (self.cur_cmd == if_test) {
                            l = (l).wrapping_add(1i32);
                        }
                    }
                }
            }
        }
        self.scanner_status = save_scanner_status;
        if (self.eqtb[crate::ix::U(((7892327i32) - 1) as usize)].int() > 0i32) {
            self.show_cur_cmd_chr();
        }
    }

    /// Here's a procedure that changes the `if_limit` code corresponding to
    /// a given value of `cond_ptr`.
    // §532
    pub fn change_if_limit(&mut self, mut l: small_number, mut p: halfword) {
        let mut q: halfword = 0; // §532
        'l_exit_f: {
            if (p == self.cond_ptr) {
                self.if_limit = l;
            } else {
                {
                    q = self.cond_ptr;
                    while true {
                        {
                            if (q == (268435455i32).wrapping_neg()) {
                                self.confusion(66137i32);
                            }
                            if (self.mem[crate::ix::U((q) as usize)].hh().rh() == p) {
                                {
                                    self.mem[crate::ix::U((q) as usize)].set_hh_b0(l);
                                    break 'l_exit_f;
                                }
                            }
                            q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                        }
                    }
                }
            }
        }
    }

    /// A condition is started when the `expand` procedure encounters
    /// an `if_test` command; in that case `expand` reduces to `conditional`,
    /// which is a recursive procedure.
    // §533
    pub fn conditional(&mut self) {
        let mut b: bool = false; // §533
        let mut e: bool = false; // §533
        let mut r: i32 = 0; // §533
        let mut m: i32 = 0; // §533
        let mut n: i32 = 0; // §533
        let mut p: halfword = 0; // §533
        let mut q: halfword = 0; // §533
        let mut save_scanner_status: small_number = 0; // §533
        let mut save_cond_ptr: halfword = 0; // §533
        let mut this_if: small_number = 0; // §533
        let mut is_unless: bool = false; // §533
        'l_exit_f: {
            'l_common_ending_f: {
                if (self.eqtb[crate::ix::U(((7892327i32) - 1) as usize)].int() > 0i32) {
                    if (self.eqtb[crate::ix::U(((7892300i32) - 1) as usize)].int() <= 1i32) {
                        self.show_cur_cmd_chr();
                    }
                }
                // §530
                {
                    p = self.get_node(if_node_size);
                    { let __v372 = self.cond_ptr; self.mem[crate::ix::U((p) as usize)].set_hh_rh(__v372); }
                    { let __v373 = self.if_limit; self.mem[crate::ix::U((p) as usize)].set_hh_b0(__v373); }
                    { let __v374 = self.cur_if; self.mem[crate::ix::U((p) as usize)].set_hh_b1(__v374); }
                    { let __v375 = self.if_line; self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].set_int(__v375); }
                    self.cond_ptr = p;
                    self.cur_if = self.cur_chr;
                    self.if_limit = if_code;
                    self.if_line = self.line;
                }
                // §533
                save_cond_ptr = self.cond_ptr;
                is_unless = (self.cur_chr >= unless_code);
                this_if = (self.cur_chr % unless_code);
                // §536
                match this_if {
                    if_char_code | if_cat_code => {
                        // §541
                        {
                            {
                                self.get_x_token();
                                if (self.cur_cmd == relax) {
                                    if (self.cur_chr == no_expand_flag) {
                                        {
                                            self.cur_cmd = active_char;
                                            self.cur_chr = (self.cur_tok).wrapping_sub(33554432i32);
                                        }
                                    }
                                }
                            }
                            if ((self.cur_cmd > active_char) || (self.cur_chr > biggest_usv)) {
                                {
                                    m = relax;
                                    n = too_big_usv;
                                }
                            } else {
                                {
                                    m = self.cur_cmd;
                                    n = self.cur_chr;
                                }
                            }
                            {
                                self.get_x_token();
                                if (self.cur_cmd == relax) {
                                    if (self.cur_chr == no_expand_flag) {
                                        {
                                            self.cur_cmd = active_char;
                                            self.cur_chr = (self.cur_tok).wrapping_sub(33554432i32);
                                        }
                                    }
                                }
                            }
                            if ((self.cur_cmd > active_char) || (self.cur_chr > biggest_usv)) {
                                {
                                    self.cur_cmd = relax;
                                    self.cur_chr = too_big_usv;
                                }
                            }
                            if (this_if == if_char_code) {
                                b = (n == self.cur_chr);
                            } else {
                                b = (m == self.cur_cmd);
                            }
                        }
                    }
                    if_int_code | if_dim_code => {
                        // §538
                        {
                            if (this_if == if_int_code) {
                                self.scan_int();
                            } else {
                                self.scan_dimen(false, false, false);
                            }
                            n = self.cur_val;
                            // §440
                            loop {
                                self.get_x_token();
                                if (self.cur_cmd != spacer) { break; }
                            }
                            // §538
                            if ((self.cur_tok >= 25165884i32) && (self.cur_tok <= 25165886i32)) {
                                r = (self.cur_tok).wrapping_sub(25165824i32);
                            } else {
                                {
                                    {
                                        if (self.interaction == error_stop_mode) {
                                        }
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(65544i32);
                                        }
                                        self.print(66163i32);
                                    }
                                    self.print_cmd_chr(if_test, this_if);
                                    {
                                        self.help_ptr = 1i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 66164i32;
                                    }
                                    self.back_error();
                                    r = 61i32;
                                }
                            }
                            if (this_if == if_int_code) {
                                self.scan_int();
                            } else {
                                self.scan_dimen(false, false, false);
                            }
                            match r {
                                60 => {
                                    b = (n < self.cur_val);
                                }
                                61 => {
                                    b = (n == self.cur_val);
                                }
                                62 => {
                                    b = (n > self.cur_val);
                                }
                                _ => {}
                            }
                        }
                    }
                    if_odd_code => {
                        // §539
                        {
                            self.scan_int();
                            b = (((self.cur_val) % 2) != 0);
                        }
                    }
                    if_vmode_code => {
                        // §536
                        b = ((self.cur_list.mode_field).wrapping_abs() == vmode);
                    }
                    if_hmode_code => {
                        b = ((self.cur_list.mode_field).wrapping_abs() == hmode);
                    }
                    if_mmode_code => {
                        b = ((self.cur_list.mode_field).wrapping_abs() == mmode);
                    }
                    if_inner_code => {
                        b = (self.cur_list.mode_field < 0i32);
                    }
                    if_void_code | if_hbox_code | if_vbox_code => {
                        // §540
                        {
                            self.scan_register_num();
                            if (self.cur_val < 256i32) {
                                p = self.eqtb[crate::ix::U((((box_base).wrapping_add(self.cur_val)) - 1) as usize)].hh().rh();
                            } else {
                                {
                                    self.find_sa_element(box_val, self.cur_val, false);
                                    if (self.cur_ptr == (268435455i32).wrapping_neg()) {
                                        p = (268435455i32).wrapping_neg();
                                    } else {
                                        p = self.mem[crate::ix::U(((self.cur_ptr).wrapping_add(1i32)) as usize)].hh().rh();
                                    }
                                }
                            }
                            if (this_if == if_void_code) {
                                b = (p == (268435455i32).wrapping_neg());
                            } else {
                                if (p == (268435455i32).wrapping_neg()) {
                                    b = false;
                                } else {
                                    if (this_if == if_hbox_code) {
                                        b = (self.mem[crate::ix::U((p) as usize)].hh().b0() == hlist_node);
                                    } else {
                                        b = (self.mem[crate::ix::U((p) as usize)].hh().b0() == vlist_node);
                                    }
                                }
                            }
                        }
                    }
                    ifx_code => {
                        // §542
                        {
                            save_scanner_status = self.scanner_status;
                            self.scanner_status = normal;
                            self.get_next();
                            n = self.cur_cs;
                            p = self.cur_cmd;
                            q = self.cur_chr;
                            self.get_next();
                            if (self.cur_cmd != p) {
                                b = false;
                            } else {
                                if (self.cur_cmd < call) {
                                    b = (self.cur_chr == q);
                                } else {
                                    // §543
                                    {
                                        p = self.mem[crate::ix::U((self.cur_chr) as usize)].hh().rh();
                                        q = self.mem[crate::ix::U((self.eqtb[crate::ix::U(((n) - 1) as usize)].hh().rh()) as usize)].hh().rh();
                                        if (p == q) {
                                            b = true;
                                        } else {
                                            {
                                                while ((p != (268435455i32).wrapping_neg()) && (q != (268435455i32).wrapping_neg())) {
                                                    if (self.mem[crate::ix::U((p) as usize)].hh().lh() != self.mem[crate::ix::U((q) as usize)].hh().lh()) {
                                                        p = (268435455i32).wrapping_neg();
                                                    } else {
                                                        {
                                                            p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                            q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                                                        }
                                                    }
                                                }
                                                b = ((p == (268435455i32).wrapping_neg()) && (q == (268435455i32).wrapping_neg()));
                                            }
                                        }
                                    }
                                }
                            }
                            // §542
                            self.scanner_status = save_scanner_status;
                        }
                    }
                    if_eof_code => {
                        // §536
                        {
                            self.scan_four_bit_int_or_18();
                            if (self.cur_val == 18i32) {
                                b = (!self.shellenabledp);
                            } else {
                                b = (self.read_open[crate::ix::U((self.cur_val) as usize)] == closed);
                            }
                        }
                    }
                    if_true_code => {
                        b = true;
                    }
                    if_false_code => {
                        b = false;
                    }
                    if_def_code => {
                        // §1577
                        {
                            save_scanner_status = self.scanner_status;
                            self.scanner_status = normal;
                            self.get_next();
                            b = (self.cur_cmd != undefined_cs);
                            self.scanner_status = save_scanner_status;
                        }
                    }
                    if_cs_code => {
                        // §1578
                        {
                            n = self.get_avail();
                            p = n;
                            e = self.is_in_csname;
                            self.is_in_csname = true;
                            loop {
                                self.get_x_token();
                                if (self.cur_cs == 0i32) {
                                    {
                                        q = self.get_avail();
                                        self.mem[crate::ix::U((p) as usize)].set_hh_rh(q);
                                        { let __v376 = self.cur_tok; self.mem[crate::ix::U((q) as usize)].set_hh_lh(__v376); }
                                        p = q;
                                    }
                                }
                                if (self.cur_cs != 0i32) { break; }
                            }
                            if (self.cur_cmd != end_cs_name) {
                                // §407
                                {
                                    {
                                        if (self.interaction == error_stop_mode) {
                                        }
                                        if self.file_line_error_style_p {
                                            self.print_file_line();
                                        } else {
                                            self.print_nl(65544i32);
                                        }
                                        self.print(65949i32);
                                    }
                                    self.print_esc(65810i32);
                                    self.print(65950i32);
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[crate::ix::U((1i32) as usize)] = 65951i32;
                                        self.help_line[crate::ix::U((0i32) as usize)] = 65952i32;
                                    }
                                    self.back_error();
                                }
                            }
                            // §1579
                            m = self.first;
                            p = self.mem[crate::ix::U((n) as usize)].hh().rh();
                            while (p != (268435455i32).wrapping_neg()) {
                                {
                                    if (m >= self.max_buf_stack) {
                                        {
                                            self.max_buf_stack = (m).wrapping_add(1i32);
                                            if (self.max_buf_stack == buf_size) {
                                                self.overflow(65538i32, buf_size);
                                            }
                                        }
                                    }
                                    { let __v377 = (self.mem[crate::ix::U((p) as usize)].hh().lh() % max_char_val); self.buffer[crate::ix::U((m) as usize)] = __v377; }
                                    m = (m).wrapping_add(1i32);
                                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                }
                            }
                            if (m > (self.first).wrapping_add(1i32)) {
                                self.cur_cs = self.id_lookup(self.first, (m).wrapping_sub(self.first));
                            } else {
                                if (m == self.first) {
                                    self.cur_cs = null_cs;
                                } else {
                                    self.cur_cs = (single_base).wrapping_add(self.buffer[crate::ix::U((self.first) as usize)]);
                                }
                            }
                            // §1578
                            self.flush_list(n);
                            b = (self.eqtb[crate::ix::U(((self.cur_cs) - 1) as usize)].hh().b0() != undefined_cs);
                            self.is_in_csname = e;
                        }
                    }
                    if_in_csname_code => {
                        // §1580
                        b = self.is_in_csname;
                    }
                    if_font_char_code => {
                        {
                            self.scan_font_ident();
                            n = self.cur_val;
                            self.scan_usv_num();
                            if ((self.font_area[crate::ix::U((n) as usize)] == aat_font_flag) || (self.font_area[crate::ix::U((n) as usize)] == otgr_font_flag)) {
                                b = (self.map_char_to_glyph(n, self.cur_val) > 0i32);
                            } else {
                                {
                                    if ((self.font_bc[crate::ix::U((n) as usize)] <= self.cur_val) && (self.font_ec[crate::ix::U((n) as usize)] >= self.cur_val)) {
                                        b = ({ let __s378 = ((self.char_base[crate::ix::U((n) as usize)]).wrapping_add(self.effective_char(true, n, self.cur_val))) as usize; self.font_info[crate::ix::U(__s378)] }.qqqq().b0() > min_quarterword);
                                    } else {
                                        b = false;
                                    }
                                }
                            }
                        }
                    }
                    if_case_code => {
                        // §544
                        {
                            self.scan_int();
                            n = self.cur_val;
                            if (self.eqtb[crate::ix::U(((7892300i32) - 1) as usize)].int() > 1i32) {
                                {
                                    self.begin_diagnostic();
                                    self.print(66165i32);
                                    self.print_int(n);
                                    self.print_char(125i32);
                                    self.end_diagnostic(false);
                                }
                            }
                            while (n != 0i32) {
                                {
                                    self.pass_text();
                                    if (self.cond_ptr == save_cond_ptr) {
                                        if (self.cur_chr == or_code) {
                                            n = (n).wrapping_sub(1i32);
                                        } else {
                                            break 'l_common_ending_f;
                                        }
                                    } else {
                                        if (self.cur_chr == fi_code) {
                                            // §531
                                            {
                                                if (self.if_stack[crate::ix::U((self.in_open) as usize)] == self.cond_ptr) {
                                                    self.if_warning();
                                                }
                                                p = self.cond_ptr;
                                                self.if_line = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                                self.cur_if = self.mem[crate::ix::U((p) as usize)].hh().b1();
                                                self.if_limit = self.mem[crate::ix::U((p) as usize)].hh().b0();
                                                self.cond_ptr = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                                self.free_node(p, if_node_size);
                                            }
                                        }
                                    }
                                }
                            }
                            // §544
                            self.change_if_limit(or_code, save_cond_ptr);
                            break 'l_exit_f;
                        }
                    }
                    if_primitive_code => {
                        // §536
                        {
                            save_scanner_status = self.scanner_status;
                            self.scanner_status = normal;
                            self.get_next();
                            self.scanner_status = save_scanner_status;
                            if (self.cur_cs < hash_base) {
                                m = self.prim_lookup((self.cur_cs).wrapping_sub(1114113i32));
                            } else {
                                m = self.prim_lookup(self.hash[crate::ix::U(((self.cur_cs) - 1179650) as usize)].rh());
                            }
                            b = ((((self.cur_cmd != undefined_cs) && (m != undefined_primitive)) && (self.cur_cmd == self.eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(m)) - 1) as usize)].hh().b0())) && (self.cur_chr == self.eqtb[crate::ix::U((((prim_eqtb_base).wrapping_add(m)) - 1) as usize)].hh().rh()));
                        }
                    }
                    _ => {}
                }
                // §533
                if is_unless {
                    b = (!b);
                }
                if (self.eqtb[crate::ix::U(((7892300i32) - 1) as usize)].int() > 1i32) {
                    // §537
                    {
                        self.begin_diagnostic();
                        if b {
                            self.print(66161i32);
                        } else {
                            self.print(66162i32);
                        }
                        self.end_diagnostic(false);
                    }
                }
                // §533
                if b {
                    {
                        self.change_if_limit(else_code, save_cond_ptr);
                        break 'l_exit_f;
                    }
                }
                // §535
                while true {
                    {
                        self.pass_text();
                        if (self.cond_ptr == save_cond_ptr) {
                            {
                                if (self.cur_chr != or_code) {
                                    break 'l_common_ending_f;
                                }
                                {
                                    if (self.interaction == error_stop_mode) {
                                    }
                                    if self.file_line_error_style_p {
                                        self.print_file_line();
                                    } else {
                                        self.print_nl(65544i32);
                                    }
                                    self.print(66159i32);
                                }
                                self.print_esc(66157i32);
                                {
                                    self.help_ptr = 1i32;
                                    self.help_line[crate::ix::U((0i32) as usize)] = 66160i32;
                                }
                                self.error();
                            }
                        } else {
                            if (self.cur_chr == fi_code) {
                                // §531
                                {
                                    if (self.if_stack[crate::ix::U((self.in_open) as usize)] == self.cond_ptr) {
                                        self.if_warning();
                                    }
                                    p = self.cond_ptr;
                                    self.if_line = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                                    self.cur_if = self.mem[crate::ix::U((p) as usize)].hh().b1();
                                    self.if_limit = self.mem[crate::ix::U((p) as usize)].hh().b0();
                                    self.cond_ptr = self.mem[crate::ix::U((p) as usize)].hh().rh();
                                    self.free_node(p, if_node_size);
                                }
                            }
                        }
                    }
                }
            }
            // §533
            if (self.cur_chr == fi_code) {
                // §531
                {
                    if (self.if_stack[crate::ix::U((self.in_open) as usize)] == self.cond_ptr) {
                        self.if_warning();
                    }
                    p = self.cond_ptr;
                    self.if_line = self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].int();
                    self.cur_if = self.mem[crate::ix::U((p) as usize)].hh().b1();
                    self.if_limit = self.mem[crate::ix::U((p) as usize)].hh().b0();
                    self.cond_ptr = self.mem[crate::ix::U((p) as usize)].hh().rh();
                    self.free_node(p, if_node_size);
                }
            } else {
                // §533
                self.if_limit = fi_code;
            }
        }
    }

    /// Here now is the first of the system-dependent routines for file name scanning.
    // §550
    pub fn begin_name(&mut self) {
        self.area_delimiter = 0i32;
        self.ext_delimiter = 0i32;
        self.quoted_filename = false;
        self.file_name_quote_char = 0i32;
    }

    /// And here's the second. The string pool might change as the file name is
    /// being scanned, since a new \.{\\csname} might be entered; therefore we keep
    /// `area_delimiter` and `ext_delimiter` relative to the beginning of the current
    /// string, instead of assigning an absolute address like `pool_ptr` to them.
    // §551
    pub fn more_name(&mut self, mut c: UnicodeScalar) -> bool {
        let mut more_name: bool = false;
        if ((self.stop_at_space && (c == 32i32)) && (self.file_name_quote_char == 0i32)) {
            more_name = false;
        } else {
            if ((self.stop_at_space && (self.file_name_quote_char != 0i32)) && (c == self.file_name_quote_char)) {
                {
                    self.file_name_quote_char = 0i32;
                    more_name = true;
                }
            } else {
                if ((self.stop_at_space && (self.file_name_quote_char == 0i32)) && ((c == 34i32) || (c == 39i32))) {
                    {
                        self.file_name_quote_char = c;
                        self.quoted_filename = true;
                        more_name = true;
                    }
                } else {
                    {
                        if (c > 65535i32) {
                            {
                                if ((self.pool_ptr).wrapping_add(2i32) > pool_size) {
                                    self.overflow(65539i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                                }
                            }
                        } else {
                            {
                                if ((self.pool_ptr).wrapping_add(1i32) > pool_size) {
                                    self.overflow(65539i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                                }
                            }
                        }
                        {
                            if (c > 65535i32) {
                                {
                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = (((c).wrapping_sub(65536i32) / 1024i32)).wrapping_add(55296i32);
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = ((c % 1024i32)).wrapping_add(56320i32);
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                }
                            } else {
                                {
                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = c;
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                }
                            }
                        }
                        if (c == 47i32) {
                            {
                                self.area_delimiter = (self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]);
                                self.ext_delimiter = 0i32;
                            }
                        } else {
                            if (c == 46i32) {
                                self.ext_delimiter = (self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]);
                            }
                        }
                        more_name = true;
                    }
                }
            }
        }
        more_name
    }

    /// The third.
    // §552
    pub fn end_name(&mut self) {
        let mut temp_str: str_number = 0; // §552
        let mut j: pool_pointer = 0; // §552
        if ((self.str_ptr).wrapping_add(3i32) > max_strings) {
            self.overflow(65540i32, (max_strings).wrapping_sub(self.init_str_ptr));
        }
        if (self.area_delimiter == 0i32) {
            self.cur_area = 65626i32;
        } else {
            {
                self.cur_area = self.str_ptr;
                { let __ix379 = ((self.str_ptr).wrapping_add(1i32)).wrapping_sub(65536i32); let __v380 = (self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]).wrapping_add(self.area_delimiter); self.str_start[crate::ix::U((__ix379) as usize)] = __v380; }
                self.str_ptr = (self.str_ptr).wrapping_add(1i32);
                temp_str = self.search_string(self.cur_area);
                if (temp_str > 0i32) {
                    {
                        self.cur_area = temp_str;
                        self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                        {
                            let __for_end_6 = (self.pool_ptr).wrapping_sub(1i32);
                            j = self.str_start[crate::ix::U((((self.str_ptr).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)];
                            while j <= __for_end_6 {
                                {
                                    { let __ix381 = (j).wrapping_sub(self.area_delimiter); let __v382 = self.str_pool[crate::ix::U((j) as usize)]; self.str_pool[crate::ix::U((__ix381) as usize)] = __v382; }
                                }
                                j = j.wrapping_add(1);
                            }
                        }
                        self.pool_ptr = (self.pool_ptr).wrapping_sub(self.area_delimiter);
                    }
                }
            }
        }
        if (self.ext_delimiter == 0i32) {
            {
                self.cur_ext = 65626i32;
                self.cur_name = self.slow_make_string();
            }
        } else {
            {
                self.cur_name = self.str_ptr;
                { let __ix383 = ((self.str_ptr).wrapping_add(1i32)).wrapping_sub(65536i32); let __v384 = (((self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]).wrapping_add(self.ext_delimiter)).wrapping_sub(self.area_delimiter)).wrapping_sub(1i32); self.str_start[crate::ix::U((__ix383) as usize)] = __v384; }
                self.str_ptr = (self.str_ptr).wrapping_add(1i32);
                self.cur_ext = self.make_string();
                self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                temp_str = self.search_string(self.cur_name);
                if (temp_str > 0i32) {
                    {
                        self.cur_name = temp_str;
                        self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                        {
                            let __for_end_6 = (self.pool_ptr).wrapping_sub(1i32);
                            j = self.str_start[crate::ix::U((((self.str_ptr).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)];
                            while j <= __for_end_6 {
                                {
                                    { let __ix385 = (((j).wrapping_sub(self.ext_delimiter)).wrapping_add(self.area_delimiter)).wrapping_add(1i32); let __v386 = self.str_pool[crate::ix::U((j) as usize)]; self.str_pool[crate::ix::U((__ix385) as usize)] = __v386; }
                                }
                                j = j.wrapping_add(1);
                            }
                        }
                        self.pool_ptr = (((self.pool_ptr).wrapping_sub(self.ext_delimiter)).wrapping_add(self.area_delimiter)).wrapping_add(1i32);
                    }
                }
                self.cur_ext = self.slow_make_string();
            }
        }
    }

    /// Another system-dependent routine is needed to convert three internal
    /// \TeX\ strings
    /// into the `name_of_file` value that is used to open files. The present code
    /// allows both lowercase and uppercase letters in the file name.
    // §554
    pub fn pack_file_name(&mut self, mut n: str_number, mut a: str_number, mut e: str_number) {
        let mut k: i32 = 0; // §554
        let mut c: UnicodeScalar = 0; // §554
        let mut j: pool_pointer = 0; // §554
        k = 0i32;
        {
            let __for_end_2 = (self.str_start[crate::ix::U((((a).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)]).wrapping_sub(1i32);
            j = self.str_start[crate::ix::U(((a).wrapping_sub(65536i32)) as usize)];
            while j <= __for_end_2 {
                {
                    c = self.str_pool[crate::ix::U((j) as usize)];
                    k = (k).wrapping_add(1i32);
                    if (k <= file_name_size) {
                        {
                            if (c < 128i32) {
                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((c) as u8);
                            } else {
                                if (c < 2048i32) {
                                    {
                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((192i32).wrapping_add((c / 64i32))) as u8);
                                        k = (k).wrapping_add(1i32);
                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                    }
                                } else {
                                    if (c < 55296i32) {
                                        {
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((224i32).wrapping_add((c / 4096i32))) as u8);
                                            k = (k).wrapping_add(1i32);
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add(((c % 4096i32) / 64i32))) as u8);
                                            k = (k).wrapping_add(1i32);
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                        }
                                    } else {
                                        if ((c < 56320i32) && ((k).wrapping_add(3i32) < file_name_size)) {
                                            {
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((240i32).wrapping_add(((c).wrapping_sub(55232i32) / 4096i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((((c).wrapping_sub(55232i32) % 4096i32) / 4i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((((c).wrapping_sub(55232i32) % 4i32)).wrapping_mul(16i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((128i32) as u8);
                                            }
                                        } else {
                                            if ((c < 57344i32) && (k > 4i32)) {
                                                {
                                                    k = (k).wrapping_sub(1i32);
                                                    { let __v387 = (((((self.name_of_file[crate::ix::U((((k).wrapping_sub(1i32)) - 1) as usize)]) as i32)).wrapping_add(((c).wrapping_sub(56320i32) / 64i32))) as u8); self.name_of_file[crate::ix::U((((k).wrapping_sub(1i32)) - 1) as usize)] = __v387; }
                                                    { let __v388 = (((((self.name_of_file[crate::ix::U(((k) - 1) as usize)]) as i32)).wrapping_add(((c).wrapping_sub(56320i32) % 64i32))) as u8); self.name_of_file[crate::ix::U(((k) - 1) as usize)] = __v388; }
                                                }
                                            } else {
                                                if (c < 65536i32) {
                                                    {
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((224i32).wrapping_add((c / 4096i32))) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add(((c % 4096i32) / 64i32))) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                                    }
                                                } else {
                                                    {
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((239i32) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((191i32) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((189i32) as u8);
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
                j = j.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = (self.str_start[crate::ix::U((((n).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)]).wrapping_sub(1i32);
            j = self.str_start[crate::ix::U(((n).wrapping_sub(65536i32)) as usize)];
            while j <= __for_end_2 {
                {
                    c = self.str_pool[crate::ix::U((j) as usize)];
                    k = (k).wrapping_add(1i32);
                    if (k <= file_name_size) {
                        {
                            if (c < 128i32) {
                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((c) as u8);
                            } else {
                                if (c < 2048i32) {
                                    {
                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((192i32).wrapping_add((c / 64i32))) as u8);
                                        k = (k).wrapping_add(1i32);
                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                    }
                                } else {
                                    if (c < 55296i32) {
                                        {
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((224i32).wrapping_add((c / 4096i32))) as u8);
                                            k = (k).wrapping_add(1i32);
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add(((c % 4096i32) / 64i32))) as u8);
                                            k = (k).wrapping_add(1i32);
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                        }
                                    } else {
                                        if ((c < 56320i32) && ((k).wrapping_add(3i32) < file_name_size)) {
                                            {
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((240i32).wrapping_add(((c).wrapping_sub(55232i32) / 4096i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((((c).wrapping_sub(55232i32) % 4096i32) / 4i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((((c).wrapping_sub(55232i32) % 4i32)).wrapping_mul(16i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((128i32) as u8);
                                            }
                                        } else {
                                            if ((c < 57344i32) && (k > 4i32)) {
                                                {
                                                    k = (k).wrapping_sub(1i32);
                                                    { let __v389 = (((((self.name_of_file[crate::ix::U((((k).wrapping_sub(1i32)) - 1) as usize)]) as i32)).wrapping_add(((c).wrapping_sub(56320i32) / 64i32))) as u8); self.name_of_file[crate::ix::U((((k).wrapping_sub(1i32)) - 1) as usize)] = __v389; }
                                                    { let __v390 = (((((self.name_of_file[crate::ix::U(((k) - 1) as usize)]) as i32)).wrapping_add(((c).wrapping_sub(56320i32) % 64i32))) as u8); self.name_of_file[crate::ix::U(((k) - 1) as usize)] = __v390; }
                                                }
                                            } else {
                                                if (c < 65536i32) {
                                                    {
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((224i32).wrapping_add((c / 4096i32))) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add(((c % 4096i32) / 64i32))) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                                    }
                                                } else {
                                                    {
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((239i32) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((191i32) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((189i32) as u8);
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
                j = j.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = (self.str_start[crate::ix::U((((e).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)]).wrapping_sub(1i32);
            j = self.str_start[crate::ix::U(((e).wrapping_sub(65536i32)) as usize)];
            while j <= __for_end_2 {
                {
                    c = self.str_pool[crate::ix::U((j) as usize)];
                    k = (k).wrapping_add(1i32);
                    if (k <= file_name_size) {
                        {
                            if (c < 128i32) {
                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((c) as u8);
                            } else {
                                if (c < 2048i32) {
                                    {
                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((192i32).wrapping_add((c / 64i32))) as u8);
                                        k = (k).wrapping_add(1i32);
                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                    }
                                } else {
                                    if (c < 55296i32) {
                                        {
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((224i32).wrapping_add((c / 4096i32))) as u8);
                                            k = (k).wrapping_add(1i32);
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add(((c % 4096i32) / 64i32))) as u8);
                                            k = (k).wrapping_add(1i32);
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                        }
                                    } else {
                                        if ((c < 56320i32) && ((k).wrapping_add(3i32) < file_name_size)) {
                                            {
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((240i32).wrapping_add(((c).wrapping_sub(55232i32) / 4096i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((((c).wrapping_sub(55232i32) % 4096i32) / 4i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((((c).wrapping_sub(55232i32) % 4i32)).wrapping_mul(16i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((128i32) as u8);
                                            }
                                        } else {
                                            if ((c < 57344i32) && (k > 4i32)) {
                                                {
                                                    k = (k).wrapping_sub(1i32);
                                                    { let __v391 = (((((self.name_of_file[crate::ix::U((((k).wrapping_sub(1i32)) - 1) as usize)]) as i32)).wrapping_add(((c).wrapping_sub(56320i32) / 64i32))) as u8); self.name_of_file[crate::ix::U((((k).wrapping_sub(1i32)) - 1) as usize)] = __v391; }
                                                    { let __v392 = (((((self.name_of_file[crate::ix::U(((k) - 1) as usize)]) as i32)).wrapping_add(((c).wrapping_sub(56320i32) % 64i32))) as u8); self.name_of_file[crate::ix::U(((k) - 1) as usize)] = __v392; }
                                                }
                                            } else {
                                                if (c < 65536i32) {
                                                    {
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((224i32).wrapping_add((c / 4096i32))) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add(((c % 4096i32) / 64i32))) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                                    }
                                                } else {
                                                    {
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((239i32) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((191i32) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((189i32) as u8);
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
                j = j.wrapping_add(1);
            }
        }
        if (k <= file_name_size) {
            self.name_length = k;
        } else {
            self.name_length = file_name_size;
        }
        if (self.name_length < file_name_size) {
            self.name_of_file[crate::ix::U((((self.name_length).wrapping_add(1i32)) - 1) as usize)] = ((0i32) as u8);
        }
    }

    /// Here is the messy routine that was just mentioned. It sets `name_of_file`
    /// from the first `n` characters of `TEX_format_default`, followed by
    /// `buffer[a..b]`, followed by the last `format_ext_length` characters of
    /// `TEX_format_default`.
    /// We dare not give error messages here, since \TeX\ calls this routine before
    /// the `error` routine is ready to roll. Instead, we simply drop excess characters,
    /// since the error will be detected in another way when a strange file name
    /// isn't found.
    // §558
    pub fn pack_buffered_name(&mut self, mut n: small_number, mut a: i32, mut b: i32) {
        let mut k: i32 = 0; // §558
        let mut c: UTF16_code = 0; // §558
        let mut j: i32 = 0; // §558
        if ((((n).wrapping_add(b)).wrapping_sub(a)).wrapping_add(5i32) > file_name_size) {
            b = (((a).wrapping_add(file_name_size)).wrapping_sub(n)).wrapping_sub(5i32);
        }
        k = 0i32;
        {
            let __for_end_2 = n;
            j = 1i32;
            while j <= __for_end_2 {
                {
                    c = ((self.TEX_format_default[crate::ix::U(((j) - 1) as usize)]) as i32);
                    k = (k).wrapping_add(1i32);
                    if (k <= file_name_size) {
                        {
                            if (c < 128i32) {
                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((c) as u8);
                            } else {
                                if (c < 2048i32) {
                                    {
                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((192i32).wrapping_add((c / 64i32))) as u8);
                                        k = (k).wrapping_add(1i32);
                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                    }
                                } else {
                                    if (c < 55296i32) {
                                        {
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((224i32).wrapping_add((c / 4096i32))) as u8);
                                            k = (k).wrapping_add(1i32);
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add(((c % 4096i32) / 64i32))) as u8);
                                            k = (k).wrapping_add(1i32);
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                        }
                                    } else {
                                        if ((c < 56320i32) && ((k).wrapping_add(3i32) < file_name_size)) {
                                            {
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((240i32).wrapping_add(((c).wrapping_sub(55232i32) / 4096i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((((c).wrapping_sub(55232i32) % 4096i32) / 4i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((((c).wrapping_sub(55232i32) % 4i32)).wrapping_mul(16i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((128i32) as u8);
                                            }
                                        } else {
                                            if ((c < 57344i32) && (k > 4i32)) {
                                                {
                                                    k = (k).wrapping_sub(1i32);
                                                    { let __v393 = (((((self.name_of_file[crate::ix::U((((k).wrapping_sub(1i32)) - 1) as usize)]) as i32)).wrapping_add(((c).wrapping_sub(56320i32) / 64i32))) as u8); self.name_of_file[crate::ix::U((((k).wrapping_sub(1i32)) - 1) as usize)] = __v393; }
                                                    { let __v394 = (((((self.name_of_file[crate::ix::U(((k) - 1) as usize)]) as i32)).wrapping_add(((c).wrapping_sub(56320i32) % 64i32))) as u8); self.name_of_file[crate::ix::U(((k) - 1) as usize)] = __v394; }
                                                }
                                            } else {
                                                if (c < 65536i32) {
                                                    {
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((224i32).wrapping_add((c / 4096i32))) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add(((c % 4096i32) / 64i32))) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                                    }
                                                } else {
                                                    {
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((239i32) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((191i32) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((189i32) as u8);
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
                j = j.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = b;
            j = a;
            while j <= __for_end_2 {
                {
                    c = self.buffer[crate::ix::U((j) as usize)];
                    k = (k).wrapping_add(1i32);
                    if (k <= file_name_size) {
                        {
                            if (c < 128i32) {
                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((c) as u8);
                            } else {
                                if (c < 2048i32) {
                                    {
                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((192i32).wrapping_add((c / 64i32))) as u8);
                                        k = (k).wrapping_add(1i32);
                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                    }
                                } else {
                                    if (c < 55296i32) {
                                        {
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((224i32).wrapping_add((c / 4096i32))) as u8);
                                            k = (k).wrapping_add(1i32);
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add(((c % 4096i32) / 64i32))) as u8);
                                            k = (k).wrapping_add(1i32);
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                        }
                                    } else {
                                        if ((c < 56320i32) && ((k).wrapping_add(3i32) < file_name_size)) {
                                            {
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((240i32).wrapping_add(((c).wrapping_sub(55232i32) / 4096i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((((c).wrapping_sub(55232i32) % 4096i32) / 4i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((((c).wrapping_sub(55232i32) % 4i32)).wrapping_mul(16i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((128i32) as u8);
                                            }
                                        } else {
                                            if ((c < 57344i32) && (k > 4i32)) {
                                                {
                                                    k = (k).wrapping_sub(1i32);
                                                    { let __v395 = (((((self.name_of_file[crate::ix::U((((k).wrapping_sub(1i32)) - 1) as usize)]) as i32)).wrapping_add(((c).wrapping_sub(56320i32) / 64i32))) as u8); self.name_of_file[crate::ix::U((((k).wrapping_sub(1i32)) - 1) as usize)] = __v395; }
                                                    { let __v396 = (((((self.name_of_file[crate::ix::U(((k) - 1) as usize)]) as i32)).wrapping_add(((c).wrapping_sub(56320i32) % 64i32))) as u8); self.name_of_file[crate::ix::U(((k) - 1) as usize)] = __v396; }
                                                }
                                            } else {
                                                if (c < 65536i32) {
                                                    {
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((224i32).wrapping_add((c / 4096i32))) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add(((c % 4096i32) / 64i32))) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                                    }
                                                } else {
                                                    {
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((239i32) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((191i32) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((189i32) as u8);
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
                j = j.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = format_default_length;
            j = 17i32;
            while j <= __for_end_2 {
                {
                    c = ((self.TEX_format_default[crate::ix::U(((j) - 1) as usize)]) as i32);
                    k = (k).wrapping_add(1i32);
                    if (k <= file_name_size) {
                        {
                            if (c < 128i32) {
                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((c) as u8);
                            } else {
                                if (c < 2048i32) {
                                    {
                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((192i32).wrapping_add((c / 64i32))) as u8);
                                        k = (k).wrapping_add(1i32);
                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                    }
                                } else {
                                    if (c < 55296i32) {
                                        {
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((224i32).wrapping_add((c / 4096i32))) as u8);
                                            k = (k).wrapping_add(1i32);
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add(((c % 4096i32) / 64i32))) as u8);
                                            k = (k).wrapping_add(1i32);
                                            self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                        }
                                    } else {
                                        if ((c < 56320i32) && ((k).wrapping_add(3i32) < file_name_size)) {
                                            {
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((240i32).wrapping_add(((c).wrapping_sub(55232i32) / 4096i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((((c).wrapping_sub(55232i32) % 4096i32) / 4i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((((c).wrapping_sub(55232i32) % 4i32)).wrapping_mul(16i32))) as u8);
                                                k = (k).wrapping_add(1i32);
                                                self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((128i32) as u8);
                                            }
                                        } else {
                                            if ((c < 57344i32) && (k > 4i32)) {
                                                {
                                                    k = (k).wrapping_sub(1i32);
                                                    { let __v397 = (((((self.name_of_file[crate::ix::U((((k).wrapping_sub(1i32)) - 1) as usize)]) as i32)).wrapping_add(((c).wrapping_sub(56320i32) / 64i32))) as u8); self.name_of_file[crate::ix::U((((k).wrapping_sub(1i32)) - 1) as usize)] = __v397; }
                                                    { let __v398 = (((((self.name_of_file[crate::ix::U(((k) - 1) as usize)]) as i32)).wrapping_add(((c).wrapping_sub(56320i32) % 64i32))) as u8); self.name_of_file[crate::ix::U(((k) - 1) as usize)] = __v398; }
                                                }
                                            } else {
                                                if (c < 65536i32) {
                                                    {
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((224i32).wrapping_add((c / 4096i32))) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add(((c % 4096i32) / 64i32))) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = (((128i32).wrapping_add((c % 64i32))) as u8);
                                                    }
                                                } else {
                                                    {
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((239i32) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((191i32) as u8);
                                                        k = (k).wrapping_add(1i32);
                                                        self.name_of_file[crate::ix::U(((k) - 1) as usize)] = ((189i32) as u8);
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
                j = j.wrapping_add(1);
            }
        }
        if (k <= file_name_size) {
            self.name_length = k;
        } else {
            self.name_length = file_name_size;
        }
        if (self.name_length < file_name_size) {
            self.name_of_file[crate::ix::U((((self.name_length).wrapping_add(1i32)) - 1) as usize)] = ((0i32) as u8);
        }
    }

    /// Operating systems often make it possible to determine the exact name (and
    /// possible version number) of a file that has been opened. The following routine,
    /// which simply makes a \TeX\ string from the value of `name_of_file`, should
    /// ideally be changed to deduce the full name of file~`f`, which is the file
    /// most recently opened, if it is possible to do this in a \PASCAL\ program.
    /// This routine might be called after string memory has overflowed, hence
    /// we dare not use ``str_room`'.
    // §560
    pub fn make_name_string(&mut self) -> str_number {
        let mut make_name_string: str_number = 0;
        let mut k: i32 = 0; // §560
        let mut save_area_delimiter: pool_pointer = 0; // §560
        let mut save_ext_delimiter: pool_pointer = 0; // §560
        let mut save_name_in_progress: bool = false; // §560
        let mut save_stop_at_space: bool = false; // §560
        if ((((self.pool_ptr).wrapping_add(self.name_length) > pool_size) || (self.str_ptr == max_strings)) || ((self.pool_ptr).wrapping_sub(self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)]) > 0i32)) {
            make_name_string = 63i32;
        } else {
            {
                self.make_utf16_name();
                {
                    let __for_end_4 = (self.name_length16).wrapping_sub(1i32);
                    k = 0i32;
                    while k <= __for_end_4 {
                        {
                            if (self.name_of_file16[crate::ix::U((k) as usize)] > 65535i32) {
                                {
                                    { let __ix399 = self.pool_ptr; let __v400 = (((self.name_of_file16[crate::ix::U((k) as usize)]).wrapping_sub(65536i32) / 1024i32)).wrapping_add(55296i32); self.str_pool[crate::ix::U((__ix399) as usize)] = __v400; }
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                    { let __ix401 = self.pool_ptr; let __v402 = ((self.name_of_file16[crate::ix::U((k) as usize)] % 1024i32)).wrapping_add(56320i32); self.str_pool[crate::ix::U((__ix401) as usize)] = __v402; }
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                }
                            } else {
                                {
                                    { let __ix403 = self.pool_ptr; let __v404 = self.name_of_file16[crate::ix::U((k) as usize)]; self.str_pool[crate::ix::U((__ix403) as usize)] = __v404; }
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                }
                            }
                        }
                        k = k.wrapping_add(1);
                    }
                }
                make_name_string = self.make_string();
                save_area_delimiter = self.area_delimiter;
                save_ext_delimiter = self.ext_delimiter;
                save_name_in_progress = self.name_in_progress;
                save_stop_at_space = self.stop_at_space;
                self.name_in_progress = true;
                self.begin_name();
                self.stop_at_space = false;
                k = 0i32;
                while ((k < self.name_length16) && self.more_name(self.name_of_file16[crate::ix::U((k) as usize)])) {
                    k = (k).wrapping_add(1i32);
                }
                self.stop_at_space = save_stop_at_space;
                self.end_name();
                self.name_in_progress = save_name_in_progress;
                self.area_delimiter = save_area_delimiter;
                self.ext_delimiter = save_ext_delimiter;
            }
        }
        make_name_string
    }

    /// Operating systems often make it possible to determine the exact name (and
    /// possible version number) of a file that has been opened. The following routine,
    /// which simply makes a \TeX\ string from the value of `name_of_file`, should
    /// ideally be changed to deduce the full name of file~`f`, which is the file
    /// most recently opened, if it is possible to do this in a \PASCAL\ program.
    /// This routine might be called after string memory has overflowed, hence
    /// we dare not use ``str_room`'.
    // §560
    pub fn u_make_name_string(&mut self, f: &mut crate::system::AlphaFile) -> str_number {
        let mut u_make_name_string: str_number = 0;
        u_make_name_string = self.make_name_string();
        u_make_name_string
    }

    /// Now let's consider the ``driver''
    /// routines by which \TeX\ deals with file names
    /// in a system-independent manner.  First comes a procedure that looks for a
    /// file name in the input by calling `get_x_token` for the information.
    // §561
    pub fn scan_file_name(&mut self) {
        let mut save_warning_index: halfword = 0; // §561
        'l_done_f: {
            save_warning_index = self.warning_index;
            self.warning_index = self.cur_cs;
            // §438
            loop {
                self.get_x_token();
                if ((self.cur_cmd != spacer) && (self.cur_cmd != relax)) { break; }
            }
            // §561
            self.back_input();
            if (self.cur_cmd == left_brace) {
                self.scan_file_name_braced();
            } else {
                {
                    self.name_in_progress = true;
                    self.begin_name();
                    // §440
                    loop {
                        self.get_x_token();
                        if (self.cur_cmd != spacer) { break; }
                    }
                    // §561
                    while true {
                        {
                            if ((self.cur_cmd > other_char) || (self.cur_chr > biggest_usv)) {
                                {
                                    self.back_input();
                                    break 'l_done_f;
                                }
                            }
                            if (!self.more_name(self.cur_chr)) {
                                break 'l_done_f;
                            }
                            self.get_x_token();
                        }
                    }
                }
            }
        }
        self.end_name();
        self.name_in_progress = false;
        self.warning_index = save_warning_index;
    }

    /// Here is a routine that manufactures the output file names, assuming that
    /// `job_name<>0`. It ignores and changes the current settings of `cur_area`
    /// and `cur_ext`.
    // §564
    pub fn pack_job_name(&mut self, mut s: str_number) {
        self.cur_area = 65626i32;
        self.cur_ext = s;
        self.cur_name = self.job_name;
        self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
    }

    /// If some trouble arises when \TeX\ tries to open a file, the following
    /// routine calls upon the user to supply another file name. Parameter~`s`
    /// is used in the error message to identify the type of file; parameter~`e`
    /// is the default extension if none is given. Upon exit from the routine,
    /// variables `cur_name`, `cur_area`, `cur_ext`, and `name_of_file` are
    /// ready for another attempt at file opening.
    // §565
    pub fn prompt_file_name(&mut self, mut s: str_number, mut e: str_number) {
        let mut k: i32 = 0; // §565
        let mut saved_cur_name: str_number = 0; // §565
        let mut saved_cur_ext: str_number = 0; // §565
        let mut saved_cur_area: str_number = 0; // §565
        if (self.interaction == scroll_mode) {
        }
        if (s == 66169i32) {
            {
                if (self.interaction == error_stop_mode) {
                }
                if self.file_line_error_style_p {
                    self.print_file_line();
                } else {
                    self.print_nl(65544i32);
                }
                self.print(66170i32);
            }
        } else {
            {
                if (self.interaction == error_stop_mode) {
                }
                if self.file_line_error_style_p {
                    self.print_file_line();
                } else {
                    self.print_nl(65544i32);
                }
                self.print(66171i32);
            }
        }
        self.print_file_name(self.cur_name, self.cur_area, self.cur_ext);
        self.print(66172i32);
        if ((e == 66173i32) || (e == 65626i32)) {
            self.show_context();
        }
        self.print_ln();
        self.print_prompt_file_name_help_msg();
        if (e != 65626i32) {
            {
                self.print(66174i32);
                self.print(e);
                self.print(39i32);
            }
        }
        self.print(41i32);
        self.print_ln();
        self.print_nl(66175i32);
        self.print(s);
        if (self.interaction < scroll_mode) {
            self.fatal_error(66176i32);
        }
        saved_cur_name = self.cur_name;
        saved_cur_ext = self.cur_ext;
        saved_cur_area = self.cur_area;
        crate::system::break_in(&mut self.term_in, true);
        {
            self.print(65593i32);
            self.term_input();
        }
        // §566
        {
            'l_done_f: {
                self.begin_name();
                k = self.first;
                while ((self.buffer[crate::ix::U((k) as usize)] == 32i32) && (k < self.last)) {
                    k = (k).wrapping_add(1i32);
                }
                while true {
                    {
                        if (k == self.last) {
                            break 'l_done_f;
                        }
                        if (!self.more_name(self.buffer[crate::ix::U((k) as usize)])) {
                            break 'l_done_f;
                        }
                        k = (k).wrapping_add(1i32);
                    }
                }
            }
            self.end_name();
        }
        // §565
        if (((self.length(self.cur_name) == 0i32) && (self.cur_ext == 65626i32)) && (self.cur_area == 65626i32)) {
            {
                self.cur_name = saved_cur_name;
                self.cur_ext = saved_cur_ext;
                self.cur_area = saved_cur_area;
            }
        } else {
            if (self.cur_ext == 65626i32) {
                self.cur_ext = e;
            }
        }
        self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
    }

    /// The `open_log_file` routine is used to open the transcript file and to help
    /// it catch up to what has previously been printed on the terminal.
    // §569
    pub fn open_log_file(&mut self) {
        let mut old_setting: i32 = 0; // §569
        let mut k: i32 = 0; // §569
        let mut l: i32 = 0; // §569
        let mut months: [u8; 36] = [0u8; 36]; // §569
        old_setting = self.selector;
        if (self.job_name == 0i32) {
            self.job_name = self.get_job_name(66180i32);
        }
        self.pack_job_name(66181i32);
        self.recorder_change_filename();
        self.pack_job_name(66182i32);
        while (!{ let mut __f0 = ::core::mem::take(&mut self.log_file); let __r = self.a_open_out(&mut __f0); self.log_file = __f0; __r }) {
            // §570
            {
                self.selector = term_only;
                self.prompt_file_name(66184i32, 66182i32);
            }
        }
        // §569
        self.log_name = { let mut __f0 = ::core::mem::take(&mut self.log_file); let __r = self.a_make_name_string(&mut __f0); self.log_file = __f0; __r };
        self.selector = log_only;
        self.log_opened = true;
        // §571
        {
            {
                crate::system::wr_str(&mut self.log_file, "This is XeTeX, Version 3.141592653");
                crate::system::wr_str(&mut self.log_file, "-2.6");
                crate::system::wr_str(&mut self.log_file, "-0.999998");
            }
            self.wlog_version_string();
            self.print(self.format_ident);
            self.print(66185i32);
            self.print_int(self.sys_day);
            self.print_char(32i32);
            crate::system::copy_str(&mut months, "JANFEBMARAPRMAYJUNJULAUGSEPOCTNOVDEC");
            {
                let __for_end_3 = (3i32).wrapping_mul(self.sys_month);
                k = ((3i32).wrapping_mul(self.sys_month)).wrapping_sub(2i32);
                while k <= __for_end_3 {
                    {
                        let __w0 = months[crate::ix::U(((k) - 1) as usize)];
                        crate::system::wr_char(&mut self.log_file, __w0);
                    }
                    k = k.wrapping_add(1);
                }
            }
            self.print_char(32i32);
            self.print_int(self.sys_year);
            self.print_char(32i32);
            self.print_two((self.sys_time / 60i32));
            self.print_char(58i32);
            self.print_two((self.sys_time % 60i32));
            if (self.eTeX_mode == 1i32) {
                {
                    {
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        crate::system::wr_str(&mut self.log_file, "entering extended mode");
                    }
                }
            }
            if self.shellenabledp {
                {
                    {
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        let __w0 = b' ';
                        crate::system::wr_char(&mut self.log_file, __w0);
                    }
                    if self.restrictedshell {
                        {
                            {
                                crate::system::wr_str(&mut self.log_file, "restricted ");
                            }
                        }
                    }
                    {
                        crate::system::wr_str(&mut self.log_file, "\\write18 enabled.");
                    }
                }
            }
            if self.file_line_error_style_p {
                {
                    {
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        crate::system::wr_str(&mut self.log_file, " file:line:error style messages enabled.");
                    }
                }
            }
            if self.parse_first_line_p {
                {
                    {
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        crate::system::wr_str(&mut self.log_file, " %&-line parsing enabled.");
                    }
                }
            }
            if self.translate_filename_p {
                {
                    {
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        crate::system::wr_str(&mut self.log_file, " (WARNING: translate-file \"");
                    }
                    self.wlog_translate_filename();
                    {
                        crate::system::wr_str(&mut self.log_file, "\" ignored)");
                    }
                }
            }
        }
        // §569
        { let __ix405 = self.input_ptr; let __v406 = self.cur_input; self.input_stack[crate::ix::U((__ix405) as usize)] = __v406; }
        self.print_nl(66183i32);
        l = self.input_stack[crate::ix::U((0i32) as usize)].limit_field;
        if (self.buffer[crate::ix::U((l) as usize)] == self.eqtb[crate::ix::U(((7892312i32) - 1) as usize)].int()) {
            l = (l).wrapping_sub(1i32);
        }
        {
            let __for_end_2 = l;
            k = 1i32;
            while k <= __for_end_2 {
                self.print(self.buffer[crate::ix::U((k) as usize)]);
                k = k.wrapping_add(1);
            }
        }
        self.print_ln();
        self.selector = (old_setting).wrapping_add(2i32);
    }

    /// Let's turn now to the procedure that is used to initiate file reading
    /// when an `\.{\\input}' command is being processed.
    /// Beware: For historic reasons, this code foolishly conserves a tiny bit
    /// of string pool space; but that can confuse the interactive `\.E' option.
    // §572
    pub fn start_input(&mut self) {
        let mut temp_str: str_number = 0; // §572
        let mut v: halfword = 0; // §572
        let mut k: i32 = 0; // §572
        'l_done_f: {
            self.scan_file_name();
            self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
            while true {
                {
                    self.begin_file_reading();
                    self.set_tex_input_type(true);
                    if (self.kpse_in_name_ok() && { let mut __f0 = ::core::mem::take(&mut self.input_file[crate::ix::U((self.cur_input.index_field) as usize)]); let __r = self.u_open_in(&mut __f0, kpse_tex_format, self.eqtb[crate::ix::U(((7892345i32) - 1) as usize)].int(), self.eqtb[crate::ix::U(((7892346i32) - 1) as usize)].int()); self.input_file[crate::ix::U((self.cur_input.index_field) as usize)] = __f0; __r }) {
                        {
                            self.make_utf16_name();
                            self.name_in_progress = true;
                            self.begin_name();
                            self.stop_at_space = false;
                            k = 0i32;
                            while ((k < self.name_length16) && self.more_name(self.name_of_file16[crate::ix::U((k) as usize)])) {
                                k = (k).wrapping_add(1i32);
                            }
                            self.stop_at_space = true;
                            self.end_name();
                            self.name_in_progress = false;
                            break 'l_done_f;
                        }
                    }
                    self.end_file_reading();
                    self.prompt_file_name(66169i32, 65626i32);
                }
            }
        }
        self.cur_input.name_field = { let mut __f0 = ::core::mem::take(&mut self.input_file[crate::ix::U((self.cur_input.index_field) as usize)]); let __r = self.a_make_name_string(&mut __f0); self.input_file[crate::ix::U((self.cur_input.index_field) as usize)] = __f0; __r };
        { let __ix407 = self.in_open; let __v408 = self.make_full_name_string(); self.full_source_filename_stack[crate::ix::U((__ix407) as usize)] = __v408; }
        if (self.cur_input.name_field == (self.str_ptr).wrapping_sub(1i32)) {
            {
                temp_str = self.search_string(self.cur_input.name_field);
                if (temp_str > 0i32) {
                    {
                        self.cur_input.name_field = temp_str;
                        {
                            self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                            self.pool_ptr = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
                        }
                    }
                }
            }
        }
        if (self.job_name == 0i32) {
            {
                self.job_name = self.get_job_name(self.cur_name);
                self.open_log_file();
            }
        }
        if ((self.term_offset).wrapping_add(self.length(self.full_source_filename_stack[crate::ix::U((self.in_open) as usize)])) > (self.max_print_line).wrapping_sub(2i32)) {
            self.print_ln();
        } else {
            if ((self.term_offset > 0i32) || (self.file_offset > 0i32)) {
                self.print_char(32i32);
            }
        }
        self.print_char(40i32);
        self.open_parens = (self.open_parens).wrapping_add(1i32);
        self.print(self.full_source_filename_stack[crate::ix::U((self.in_open) as usize)]);
        crate::system::break_out(&mut self.term_out);
        if (self.eqtb[crate::ix::U(((7892322i32) - 1) as usize)].int() > 0i32) {
            {
                self.begin_diagnostic();
                self.print_ln();
                self.print_char(126i32);
                v = (self.input_ptr).wrapping_sub(1i32);
                if (v < self.eqtb[crate::ix::U(((7892322i32) - 1) as usize)].int()) {
                    while (v > 0i32) {
                        {
                            self.print_char(46i32);
                            v = (v).wrapping_sub(1i32);
                        }
                    }
                } else {
                    self.print_char(126i32);
                }
                self.print(66186i32);
                self.print(self.cur_name);
                self.print(self.cur_ext);
                self.print_ln();
                self.end_diagnostic(false);
            }
        }
        self.cur_input.state_field = new_line;
        // §1710
        {
            self.synctex_tag_counter = (self.synctex_tag_counter).wrapping_add(1i32);
            self.cur_input.synctex_tag_field = self.synctex_tag_counter;
        }
        // §573
        {
            self.line = 1i32;
            if { let mut __f0 = ::core::mem::take(&mut self.input_file[crate::ix::U((self.cur_input.index_field) as usize)]); let __r = self.input_ln(&mut __f0, false); self.input_file[crate::ix::U((self.cur_input.index_field) as usize)] = __f0; __r } {
            }
            self.firm_up_the_line();
            if ((self.eqtb[crate::ix::U(((7892312i32) - 1) as usize)].int() < 0i32) || (self.eqtb[crate::ix::U(((7892312i32) - 1) as usize)].int() > 255i32)) {
                self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(1i32);
            } else {
                { let __ix409 = self.cur_input.limit_field; let __v410 = self.eqtb[crate::ix::U(((7892312i32) - 1) as usize)].int(); self.buffer[crate::ix::U((__ix409) as usize)] = __v410; }
            }
            self.first = (self.cur_input.limit_field).wrapping_add(1i32);
            self.cur_input.loc_field = self.cur_input.start_field;
        }
    }

    /// The effective character of `c` in font `f` (tex.ch's ML\TeX, with
    /// xetex.ch's font mappings): `c` through the font's \.{TECkit} mapping, if it
    /// has one and `c` is not the character of a ligature, which is mapped
    /// already. ML\TeX's substitution of a missing character is not
    /// re-specified, because `mltex_enabled_p` is always false here; so `err_p`
    /// is not used.
    /// @<Declare additional functions for ML\TeX
    // §1715
    pub fn effective_char(&mut self, mut err_p: bool, mut f: internal_font_number, mut c: quarterword) -> i32 {
        let mut effective_char: i32 = 0;
        if ((!self.xtx_ligature_present) && (self.font_mapping[crate::ix::U((f) as usize)] != nil)) {
            c = self.apply_tfm_font_mapping(self.font_mapping[crate::ix::U((f) as usize)], c);
        }
        self.xtx_ligature_present = false;
        effective_char = c;
        effective_char
    }

}
