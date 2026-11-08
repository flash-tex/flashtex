// GENERATED FILE -- DO NOT EDIT.
// The WEB main program, §1332 (`@p begin ... end.`).
// Regenerate with the command in tools/web2rust/README.md.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(unused_parens, unused_mut, unused_variables, unused_assignments)]
#![allow(unused_imports, unused_labels, while_true)]
#![allow(dead_code, unreachable_code, unused_comparisons, clippy::all)]

use super::consts::*;
use super::globals::Globals;
use super::types::*;

impl Globals {
    /// The body of WEB's outer block.
    pub fn tex_body(&mut self) {
        'l_final_end_f: {
            'l_start_of_TEX_f: {
                // §1513
                self.setup_bound_vars();
                if (self.error_line > ssup_error_line) {
                    self.error_line = ssup_error_line;
                }
                // §1923
                self.interaction_option = self.web2c_interaction_option();
                self.file_line_error_style_p = self.web2c_file_line_error_style_p();
                self.halt_on_error_p = self.web2c_halt_on_error_p();
                self.parse_first_line_p = self.web2c_parse_first_line_p();
                self.dump_line = self.web2c_dump_line();
                self.eight_bit_p = self.web2c_eight_bit_p();
                self.translate_filename_p = self.web2c_translate_filename_p();
                self.mltex_p = self.web2c_mltex_p();
                self.enctex_p = self.web2c_enctex_p();
                self.shellenabledp = self.web2c_shellenabledp();
                self.restrictedshell = self.web2c_restrictedshell();
                { let mut __f0 = ::core::mem::take(&mut self.pdf_output_option); let mut __f1 = ::core::mem::take(&mut self.pdf_output_value); let mut __f2 = ::core::mem::take(&mut self.pdf_draftmode_option); let mut __f3 = ::core::mem::take(&mut self.pdf_draftmode_value); let __r = self.web2c_pdf_options(&mut __f0, &mut __f1, &mut __f2, &mut __f3); self.pdf_output_option = __f0; self.pdf_output_value = __f1; self.pdf_draftmode_option = __f2; self.pdf_draftmode_value = __f3; __r };
                // §1513
                self.history = fatal_error_stop;
                crate::system::rewrite_char(&mut self.term_out, "TTY:", "/O");
                if (self.ready_already == 314159i32) {
                    break 'l_start_of_TEX_f;
                }
                // §14
                self.bad = 0i32;
                if ((self.half_error_line < 30i32) || (self.half_error_line > (self.error_line).wrapping_sub(15i32))) {
                    self.bad = 1i32;
                }
                if (self.max_print_line < 60i32) {
                    self.bad = 2i32;
                }
                if ((dvi_buf_size % 8i32) != 0i32) {
                    self.bad = 3i32;
                }
                if (1100i32 > mem_top) {
                    self.bad = 4i32;
                }
                if (hash_prime > hash_size) {
                    self.bad = 5i32;
                }
                if (max_in_open >= 128i32) {
                    self.bad = 6i32;
                }
                if (mem_top < 267i32) {
                    self.bad = 7i32;
                }
                // §129
                if ((mem_min != mem_bot) || (mem_max != mem_top)) {
                    self.bad = 10i32;
                }
                if ((mem_min > mem_bot) || (mem_max < mem_top)) {
                    self.bad = 10i32;
                }
                if ((min_quarterword > 0i32) || (max_quarterword < 127i32)) {
                    self.bad = 11i32;
                }
                if ((min_halfword > 0i32) || (max_halfword < 32767i32)) {
                    self.bad = 12i32;
                }
                if ((min_quarterword < min_halfword) || (max_quarterword > max_halfword)) {
                    self.bad = 13i32;
                }
                if (((mem_min < min_halfword) || (mem_max >= max_halfword)) || (((0i32).wrapping_neg()).wrapping_sub(mem_min) > 268435456i32)) {
                    self.bad = 14i32;
                }
                if ((max_font_max < min_halfword) || (max_font_max > max_halfword)) {
                    self.bad = 15i32;
                }
                if (font_max > 9000i32) {
                    self.bad = 16i32;
                }
                if ((save_size > max_halfword) || (max_strings > max_halfword)) {
                    self.bad = 17i32;
                }
                if (buf_size > max_halfword) {
                    self.bad = 18i32;
                }
                if (255i32 < 255i32) {
                    self.bad = 19i32;
                }
                // §312
                if (634287i32 > max_halfword) {
                    self.bad = 21i32;
                }
                // §549
                if (format_default_length > file_name_size) {
                    self.bad = 31i32;
                }
                // §1428
                if ((2i32).wrapping_mul(max_halfword) < (mem_top).wrapping_sub(mem_min)) {
                    self.bad = 41i32;
                }
                // §1513
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
                if self.ini_version() {
                    self.fix_date_and_time();
                }
                self.ready_already = 314159i32;
            }
            self.selector = term_only;
            // §55
            self.tally = 0i32;
            self.term_offset = 0i32;
            self.file_offset = 0i32;
            // §61
            {
                crate::system::wr_str(&mut self.term_out, "This is pdfTeX, Version 3.141592653");
                crate::system::wr_str(&mut self.term_out, "-2.6");
                crate::system::wr_str(&mut self.term_out, "-1.40.29");
            }
            self.wterm_version_string();
            if (self.format_ident == 0i32) {
                {
                    {
                        crate::system::wr_str(&mut self.term_out, " (preloaded format=");
                    }
                    self.wterm_dump_name();
                    {
                        let __w0 = b')';
                        crate::system::wr_char(&mut self.term_out, __w0);
                        crate::system::wr_ln(&mut self.term_out);
                    }
                }
            } else {
                {
                    self.slow_print(self.format_ident);
                    self.print_ln();
                }
            }
            if self.shellenabledp {
                {
                    {
                        let __w0 = b' ';
                        crate::system::wr_char(&mut self.term_out, __w0);
                    }
                    if self.restrictedshell {
                        {
                            {
                                crate::system::wr_str(&mut self.term_out, "restricted ");
                            }
                        }
                    }
                    {
                        crate::system::wr_str(&mut self.term_out, "\\write18 enabled.");
                        crate::system::wr_ln(&mut self.term_out);
                    }
                }
            }
            if self.translate_filename_p {
                {
                    {
                        crate::system::wr_str(&mut self.term_out, " (");
                    }
                    self.wterm_translate_filename();
                    {
                        let __w0 = b')';
                        crate::system::wr_char(&mut self.term_out, __w0);
                        crate::system::wr_ln(&mut self.term_out);
                    }
                }
            }
            crate::system::break_out(&mut self.term_out);
            // §555
            self.job_name = 0i32;
            self.name_in_progress = false;
            self.log_opened = false;
            // §560
            self.output_file_name = 0i32;
            // §1518
            {
                // §353
                {
                    self.input_ptr = 0i32;
                    self.max_in_stack = 0i32;
                    self.full_source_filename_stack[crate::ix::U((0i32) as usize)] = 0i32;
                    self.in_open = 0i32;
                    self.open_parens = 0i32;
                    self.max_buf_stack = 0i32;
                    self.grp_stack[crate::ix::U((0i32) as usize)] = 0i32;
                    self.if_stack[crate::ix::U((0i32) as usize)] = null;
                    self.param_ptr = 0i32;
                    self.max_param_stack = 0i32;
                    self.first = buf_size;
                    loop {
                        self.buffer[crate::ix::U((self.first) as usize)] = 0i32;
                        self.first = (self.first).wrapping_sub(1i32);
                        if (self.first == 0i32) { break; }
                    }
                    self.buffer[crate::ix::U((0i32) as usize)] = 0i32;
                    self.scanner_status = normal;
                    self.warning_index = null;
                    self.first = 1i32;
                    self.cur_input.state_field = new_line;
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
                // §1649
                if ((self.etex_p() || (self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)] == 42i32)) && (self.format_ident == 1710i32)) {
                    {
                        self.no_new_control_sequence = false;
                        // §1650
                        self.primitive(1993i32, last_item, last_node_type_code);
                        self.primitive(1994i32, last_item, eTeX_version_code);
                        self.primitive(883i32, convert, eTeX_revision_code);
                        // §1658
                        self.primitive(1996i32, assign_toks, every_eof_loc);
                        self.primitive(1997i32, assign_int, 29379i32);
                        self.primitive(1998i32, assign_int, 29380i32);
                        self.primitive(1999i32, assign_int, 29381i32);
                        self.primitive(2000i32, assign_int, 29382i32);
                        self.primitive(2001i32, assign_int, 29383i32);
                        self.primitive(2002i32, assign_int, 29384i32);
                        self.primitive(2003i32, assign_int, 29385i32);
                        self.primitive(2004i32, assign_int, 29386i32);
                        self.primitive(2005i32, assign_int, 29387i32);
                        self.primitive(2006i32, assign_int, 29388i32);
                        // §1664
                        self.primitive(2021i32, last_item, current_group_level_code);
                        self.primitive(2022i32, last_item, current_group_type_code);
                        // §1667
                        self.primitive(2023i32, last_item, current_if_level_code);
                        self.primitive(2024i32, last_item, current_if_type_code);
                        self.primitive(2025i32, last_item, current_if_branch_code);
                        // §1670
                        self.primitive(2026i32, last_item, font_char_wd_code);
                        self.primitive(2027i32, last_item, font_char_ht_code);
                        self.primitive(2028i32, last_item, font_char_dp_code);
                        self.primitive(2029i32, last_item, font_char_ic_code);
                        // §1673
                        self.primitive(2030i32, last_item, par_shape_length_code);
                        self.primitive(2031i32, last_item, par_shape_indent_code);
                        self.primitive(2032i32, last_item, par_shape_dimen_code);
                        // §1676
                        self.primitive(2033i32, xray, show_groups);
                        // §1685
                        self.primitive(2035i32, xray, show_tokens);
                        // §1687
                        self.primitive(2036i32, the, 1i32);
                        self.primitive(2037i32, the, show_tokens);
                        // §1690
                        self.primitive(2038i32, xray, show_ifs);
                        // §1693
                        self.primitive(2042i32, set_page_int, 2i32);
                        // §1698
                        self.primitive(1300i32, left_right, middle_noad);
                        // §1702
                        self.primitive(2046i32, assign_int, 29389i32);
                        self.primitive(2047i32, valign, begin_L_code);
                        self.primitive(2048i32, valign, end_L_code);
                        self.primitive(2049i32, valign, begin_R_code);
                        self.primitive(2050i32, valign, end_R_code);
                        // §1748
                        self.primitive(2059i32, input, 2i32);
                        // §1760
                        self.primitive(2061i32, read_to_cs, 1i32);
                        // §1763
                        self.primitive(940i32, expand_after, 1i32);
                        self.primitive(2062i32, if_test, if_def_code);
                        self.primitive(2063i32, if_test, if_cs_code);
                        self.primitive(2064i32, if_test, if_font_char_code);
                        self.primitive(2065i32, if_test, if_in_csname_code);
                        self.primitive(2066i32, if_test, if_pdfabs_num_code);
                        self.primitive(2067i32, if_test, if_pdfabs_dim_code);
                        // §1771
                        self.primitive(1615i32, prefix, 8i32);
                        // §1779
                        self.primitive(2073i32, last_item, 39i32);
                        self.primitive(2074i32, last_item, 40i32);
                        self.primitive(2075i32, last_item, 41i32);
                        self.primitive(2076i32, last_item, 42i32);
                        // §1802
                        self.primitive(2080i32, last_item, glue_stretch_order_code);
                        self.primitive(2081i32, last_item, glue_shrink_order_code);
                        self.primitive(2082i32, last_item, glue_stretch_code);
                        self.primitive(2083i32, last_item, glue_shrink_code);
                        // §1806
                        self.primitive(2084i32, last_item, mu_to_glue_code);
                        self.primitive(2085i32, last_item, glue_to_mu_code);
                        // §1810
                        self.primitive(2086i32, mark, marks_code);
                        self.primitive(2087i32, top_bot_mark, 5i32);
                        self.primitive(2088i32, top_bot_mark, 6i32);
                        self.primitive(2089i32, top_bot_mark, 7i32);
                        self.primitive(2090i32, top_bot_mark, 8i32);
                        self.primitive(2091i32, top_bot_mark, 9i32);
                        // §1862
                        self.primitive(2095i32, un_vbox, last_box_code);
                        self.primitive(2096i32, un_vbox, vsplit_code);
                        // §1865
                        self.primitive(2097i32, set_shape, inter_line_penalties_loc);
                        self.primitive(2098i32, set_shape, club_penalties_loc);
                        self.primitive(2099i32, set_shape, widow_penalties_loc);
                        self.primitive(2100i32, set_shape, display_widow_penalties_loc);
                        // §1649
                        if (self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)] == 42i32) {
                            self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(1i32);
                        }
                        self.eTeX_mode = 1i32;
                        // §1814
                        self.max_reg_num = 32767i32;
                        self.max_reg_help_line = 2092i32;
                    }
                }
                // §1649
                if (!self.no_new_control_sequence) {
                    self.no_new_control_sequence = true;
                } else {
                    // §1518
                    if (((self.format_ident == 0i32) || (self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)] == 38i32)) || self.dump_line) {
                        {
                            if (self.format_ident != 0i32) {
                                self.initialize();
                            }
                            if (!self.open_fmt_file()) {
                                break 'l_final_end_f;
                            }
                            if (!self.flashtex_fmt_restore()) {
                                {
                                    if (!self.load_fmt_file()) {
                                        {
                                            { let mut __f0 = ::core::mem::take(&mut self.fmt_file); let __r = self.w_close(&mut __f0); self.fmt_file = __f0; __r };
                                            break 'l_final_end_f;
                                        }
                                    }
                                    self.flashtex_fmt_loaded();
                                }
                            }
                            { let mut __f0 = ::core::mem::take(&mut self.fmt_file); let __r = self.w_close(&mut __f0); self.fmt_file = __f0; __r };
                            if self.intr_on {
                                self.flashtex_intr_loaded();
                            }
                            if self.bm_on {
                                self.flashtex_bm_loaded();
                            }
                            while ((self.cur_input.loc_field < self.cur_input.limit_field) && (self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)] == 32i32)) {
                                self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(1i32);
                            }
                        }
                    }
                }
                if (self.pdf_output_option != 0i32) {
                    { let __v2495 = self.pdf_output_value; self.eqtb[crate::ix::U(((29342i32) - 1) as usize)].set_int(__v2495); }
                }
                if (self.pdf_draftmode_option != 0i32) {
                    { let __v2496 = self.pdf_draftmode_value; self.eqtb[crate::ix::U(((29368i32) - 1) as usize)].set_int(__v2496); }
                }
                self.pdf_init_map_file();
                if (self.eTeX_mode == 1i32) {
                    {
                        crate::system::wr_str(&mut self.term_out, "entering extended mode");
                        crate::system::wr_ln(&mut self.term_out);
                    }
                }
                if ((self.eqtb[crate::ix::U(((29325i32) - 1) as usize)].int() < 0i32) || (self.eqtb[crate::ix::U(((29325i32) - 1) as usize)].int() > 255i32)) {
                    self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(1i32);
                } else {
                    { let __ix2497 = self.cur_input.limit_field; let __v2498 = self.eqtb[crate::ix::U(((29325i32) - 1) as usize)].int(); self.buffer[crate::ix::U((__ix2497) as usize)] = __v2498; }
                }
                if self.mltex_enabled_p {
                    {
                        {
                            crate::system::wr_str(&mut self.term_out, "MLTeX v2.2 enabled");
                            crate::system::wr_ln(&mut self.term_out);
                        }
                    }
                }
                if self.enctex_enabled_p {
                    {
                        {
                            crate::system::wr_str(&mut self.term_out, " encTeX v. Jun. 2004");
                        }
                        {
                            crate::system::wr_str(&mut self.term_out, ", reencoding enabled.");
                            crate::system::wr_ln(&mut self.term_out);
                        }
                        if self.translate_filename_p {
                            {
                                {
                                    crate::system::wr_str(&mut self.term_out, " (\\xordcode, \\xchrcode, \\xprncode overridden by TCX)");
                                    crate::system::wr_ln(&mut self.term_out);
                                }
                            }
                        }
                    }
                }
                self.fix_date_and_time();
                if self.trie_not_ready {
                    self.make_pdftex_banner();
                }
                self.random_seed = ((self.microseconds).wrapping_mul(1000i32)).wrapping_add((self.epochseconds % 1000000i32));
                self.init_randoms(self.random_seed);
                // §942
                self.magic_offset = (self.str_start[crate::ix::U((math_spacing) as usize)]).wrapping_sub((9i32).wrapping_mul(ord_noad));
                // §75
                if (self.interaction == batch_mode) {
                    self.selector = no_print;
                } else {
                    self.selector = term_only;
                }
                // §1518
                if ((self.cur_input.loc_field < self.cur_input.limit_field) && (self.eqtb[crate::ix::U((((cat_code_base).wrapping_add(self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)])) - 1) as usize)].hh().rh() != escape)) {
                    self.start_input();
                }
            }
            // §1513
            self.history = spotless;
            self.main_control();
            self.final_cleanup();
            self.close_files_and_terminate();
        }
        self.ready_already = 0i32;
    }
}
