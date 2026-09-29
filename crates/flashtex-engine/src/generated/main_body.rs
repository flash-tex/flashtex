// GENERATED FILE -- DO NOT EDIT.
// The WEB main program, §1332 (`@p begin ... end.`).
// Regenerate with the command in tools/web2rust/README.md.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(unused_parens, unused_mut, unused_variables, unused_assignments, unused_imports)]
#![allow(dead_code, unreachable_code, unused_labels, while_true, clippy::all)]

use super::consts::*;
use super::globals::Globals;
use super::types::*;

impl Globals {
    /// The body of WEB's outer block.
    pub fn tex_body(&mut self) {
        'l_final_end_f: {
            'l_start_of_TEX_f: {
                // §1332
                self.history = 3i32;
                crate::system::rewrite_char(&mut self.term_out, "TTY:", "/O");
                if (self.ready_already == 314159i32) {
                    break 'l_start_of_TEX_f;
                }
                // §14
                self.bad = 0i32;
                if ((half_error_line < 30i32) || (half_error_line > (error_line).wrapping_sub(15i32))) {
                    self.bad = 1i32;
                }
                if (max_print_line < 60i32) {
                    self.bad = 2i32;
                }
                if ((dvi_buf_size % 8i32) != 0i32) {
                    self.bad = 3i32;
                }
                if (1100i32 > 4999999i32) {
                    self.bad = 4i32;
                }
                if (522749i32 > 615000i32) {
                    self.bad = 5i32;
                }
                if (max_in_open >= 128i32) {
                    self.bad = 6i32;
                }
                if (4999999i32 < 267i32) {
                    self.bad = 7i32;
                }
                // §111
                if ((mem_min != 0i32) || (mem_max != 4999999i32)) {
                    self.bad = 10i32;
                }
                if ((mem_min > 0i32) || (mem_max < 4999999i32)) {
                    self.bad = 10i32;
                }
                if ((0i32 > 0i32) || (255i32 < 127i32)) {
                    self.bad = 11i32;
                }
                if ((0i32 > 0i32) || (268435455i32 < 32767i32)) {
                    self.bad = 12i32;
                }
                if ((0i32 < 0i32) || (255i32 > 268435455i32)) {
                    self.bad = 13i32;
                }
                if (((mem_min < 0i32) || (mem_max >= 268435455i32)) || (((0i32).wrapping_neg()).wrapping_sub(mem_min) > 268435456i32)) {
                    self.bad = 14i32;
                }
                if ((0i32 < 0i32) || (font_max > 255i32)) {
                    self.bad = 15i32;
                }
                if (font_max > 256i32) {
                    self.bad = 16i32;
                }
                if ((save_size > 268435455i32) || (max_strings > 268435455i32)) {
                    self.bad = 17i32;
                }
                if (buf_size > 268435455i32) {
                    self.bad = 18i32;
                }
                if (255i32 < 255i32) {
                    self.bad = 19i32;
                }
                // §290
                if (619876i32 > 268435455i32) {
                    self.bad = 21i32;
                }
                // §522
                if (20i32 > file_name_size) {
                    self.bad = 31i32;
                }
                // §1249
                if ((2i32).wrapping_mul(268435455i32) < (4999999i32).wrapping_sub(mem_min)) {
                    self.bad = 41i32;
                }
                // §1332
                if (self.bad > 0i32) {
                    {
                        {
                            let __w2 = self.bad;
                            crate::system::wr_str(&mut self.term_out, "Ouch---my internal constants have been clobbered!");
                            crate::system::wr_str(&mut self.term_out, "---case ");
                            crate::system::wr_int(&mut self.term_out, __w2, 1i32);
                            crate::system::wr_ln(&mut self.term_out);
                        }
                        break 'l_final_end_f;
                    }
                }
                self.initialize();
                if (!self.get_strings_started()) {
                    break 'l_final_end_f;
                }
                self.init_prim();
                self.init_str_ptr = self.str_ptr;
                self.init_pool_ptr = self.pool_ptr;
                self.fix_date_and_time();
                self.ready_already = 314159i32;
            }
            self.selector = 17i32;
            // §55
            self.tally = 0i32;
            self.term_offset = 0i32;
            self.file_offset = 0i32;
            // §61
            {
                crate::system::wr_str(&mut self.term_out, "This is TeX, Version 3.141592653");
            }
            if (self.format_ident == 0i32) {
                {
                    crate::system::wr_str(&mut self.term_out, " (no format preloaded)");
                    crate::system::wr_ln(&mut self.term_out);
                }
            } else {
                {
                    self.slow_print(self.format_ident);
                    self.print_ln();
                }
            }
            crate::system::break_out(&mut self.term_out);
            // §528
            self.job_name = 0i32;
            self.name_in_progress = false;
            self.log_opened = false;
            // §533
            self.output_file_name = 0i32;
            // §1337
            {
                // §331
                {
                    self.input_ptr = 0i32;
                    self.max_in_stack = 0i32;
                    self.in_open = 0i32;
                    self.open_parens = 0i32;
                    self.max_buf_stack = 0i32;
                    self.param_ptr = 0i32;
                    self.max_param_stack = 0i32;
                    self.first = buf_size;
                    loop {
                        self.buffer[(self.first) as usize] = 0i32;
                        self.first = (self.first).wrapping_sub(1i32);
                        if (self.first == 0i32) { break; }
                    }
                    self.scanner_status = 0i32;
                    self.warning_index = 0i32;
                    self.first = 1i32;
                    self.cur_input.state_field = 33i32;
                    self.cur_input.start_field = 1i32;
                    self.cur_input.index_field = 0i32;
                    self.line = 0i32;
                    self.cur_input.name_field = 0i32;
                    self.force_eof = false;
                    self.align_state = 1000000i32;
                    if (!self.init_terminal()) {
                        break 'l_final_end_f;
                    }
                    self.cur_input.limit_field = self.last;
                    self.first = (self.last).wrapping_add(1i32);
                }
                // §1337
                if ((self.format_ident == 0i32) || (self.buffer[(self.cur_input.loc_field) as usize] == 38i32)) {
                    {
                        if (self.format_ident != 0i32) {
                            self.initialize();
                        }
                        if (!self.open_fmt_file()) {
                            break 'l_final_end_f;
                        }
                        if (!self.load_fmt_file()) {
                            {
                                { let mut __f = ::core::mem::take(&mut self.fmt_file); let __r = self.w_close(&mut __f); self.fmt_file = __f; __r };
                                break 'l_final_end_f;
                            }
                        }
                        { let mut __f = ::core::mem::take(&mut self.fmt_file); let __r = self.w_close(&mut __f); self.fmt_file = __f; __r };
                        while ((self.cur_input.loc_field < self.cur_input.limit_field) && (self.buffer[(self.cur_input.loc_field) as usize] == 32i32)) {
                            self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(1i32);
                        }
                    }
                }
                if ((self.eqtb[((618211i32) - 1) as usize].int() < 0i32) || (self.eqtb[((618211i32) - 1) as usize].int() > 255i32)) {
                    self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(1i32);
                } else {
                    { let __ix1244 = self.cur_input.limit_field; let __v1245 = self.eqtb[((618211i32) - 1) as usize].int(); self.buffer[(__ix1244) as usize] = __v1245; }
                }
                self.fix_date_and_time();
                // §765
                self.magic_offset = (self.str_start[(892i32) as usize]).wrapping_sub((9i32).wrapping_mul(16i32));
                // §75
                if (self.interaction == 0i32) {
                    self.selector = 16i32;
                } else {
                    self.selector = 17i32;
                }
                // §1337
                if ((self.cur_input.loc_field < self.cur_input.limit_field) && (self.eqtb[(((616883i32).wrapping_add(self.buffer[(self.cur_input.loc_field) as usize])) - 1) as usize].hh().rh() != 0i32)) {
                    self.start_input();
                }
            }
            // §1332
            self.history = 0i32;
            self.main_control();
            self.final_cleanup();
            self.close_files_and_terminate();
        }
        self.ready_already = 0i32;
    }
}
