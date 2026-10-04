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
                // §1386
                self.setup_bound_vars();
                if (self.error_line > ssup_error_line) {
                    self.error_line = ssup_error_line;
                }
                // §1716
                self.interaction_option = self.web2c_interaction_option();
                self.file_line_error_style_p = self.web2c_file_line_error_style_p();
                self.halt_on_error_p = self.web2c_halt_on_error_p();
                self.parse_first_line_p = self.web2c_parse_first_line_p();
                self.dump_line = self.web2c_dump_line();
                self.eight_bit_p = self.web2c_eight_bit_p();
                self.translate_filename_p = self.web2c_translate_filename_p();
                self.shellenabledp = self.web2c_shellenabledp();
                self.restrictedshell = self.web2c_restrictedshell();
                self.no_pdf_output = self.web2c_no_pdf_output();
                // §1386
                self.history = fatal_error_stop;
                crate::system::rewrite_char(&mut self.term_out, "TTY:", "/O");
                if (self.ready_already == 314159i32) {
                    break 'l_start_of_TEX_f;
                }
                // §14
                self.bad = 0i32;
                if ((self.half_error_line < 30i32)
                    || (self.half_error_line > (self.error_line).wrapping_sub(15i32)))
                {
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
                // §133
                if ((mem_min != mem_bot) || (mem_max != mem_top)) {
                    self.bad = 10i32;
                }
                if ((mem_min > mem_bot) || (mem_max < mem_top)) {
                    self.bad = 10i32;
                }
                if ((min_quarterword > 0i32) || (max_quarterword < 32767i32)) {
                    self.bad = 11i32;
                }
                if (((268435455i32).wrapping_neg() > 0i32) || (max_halfword < 1073741823i32)) {
                    self.bad = 12i32;
                }
                if ((min_quarterword < (268435455i32).wrapping_neg())
                    || (max_quarterword > max_halfword))
                {
                    self.bad = 13i32;
                }
                if (((mem_min < (268435455i32).wrapping_neg()) || (mem_max >= max_halfword))
                    || (((0i32).wrapping_neg()).wrapping_sub(mem_min) > 1073741824i32))
                {
                    self.bad = 14i32;
                }
                if ((max_font_max < (268435455i32).wrapping_neg()) || (max_font_max > max_halfword))
                {
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
                if (65535i32 < 65535i32) {
                    self.bad = 19i32;
                }
                // §320
                if (43161429i32 > max_halfword) {
                    self.bad = 21i32;
                }
                // §557
                if (format_default_length > file_name_size) {
                    self.bad = 31i32;
                }
                // §1303
                if ((2i32).wrapping_mul(max_halfword) < (mem_top).wrapping_sub(mem_min)) {
                    self.bad = 41i32;
                }
                // §1386
                if (self.bad > 0i32) {
                    {
                        {
                            let __w2 = self.bad;
                            crate::system::wr_str(
                                &mut self.term_out,
                                "Ouch---my internal constants have been clobbered!",
                            );
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
            self.selector = term_only;
            // §55
            self.tally = 0i32;
            self.term_offset = 0i32;
            self.file_offset = 0i32;
            // §65
            {
                crate::system::wr_str(&mut self.term_out, "This is XeTeX, Version 3.141592653");
                crate::system::wr_str(&mut self.term_out, "-2.6");
                crate::system::wr_str(&mut self.term_out, "-0.999998");
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
                    self.print(self.format_ident);
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
                        crate::system::wr_str(&mut self.term_out, " (WARNING: translate-file \"");
                    }
                    self.wterm_translate_filename();
                    {
                        crate::system::wr_str(&mut self.term_out, "\" ignored)");
                        crate::system::wr_ln(&mut self.term_out);
                    }
                }
            }
            crate::system::break_out(&mut self.term_out);
            // §563
            self.job_name = 0i32;
            self.name_in_progress = false;
            self.log_opened = false;
            // §568
            self.output_file_name = 0i32;
            if self.no_pdf_output {
                self.output_file_extension = 66178i32;
            } else {
                self.output_file_extension = 66179i32;
            }
            // §1391
            {
                // §361
                {
                    self.input_ptr = 0i32;
                    self.max_in_stack = 0i32;
                    self.full_source_filename_stack[crate::ix::U((0i32) as usize)] = 0i32;
                    self.in_open = 0i32;
                    self.open_parens = 0i32;
                    self.max_buf_stack = 0i32;
                    self.grp_stack[crate::ix::U((0i32) as usize)] = 0i32;
                    self.if_stack[crate::ix::U((0i32) as usize)] = (268435455i32).wrapping_neg();
                    self.param_ptr = 0i32;
                    self.max_param_stack = 0i32;
                    self.first = buf_size;
                    loop {
                        self.buffer[crate::ix::U((self.first) as usize)] = 0i32;
                        self.first = (self.first).wrapping_sub(1i32);
                        if (self.first == 0i32) {
                            break;
                        }
                    }
                    self.buffer[crate::ix::U((0i32) as usize)] = 0i32;
                    self.scanner_status = normal;
                    self.warning_index = (268435455i32).wrapping_neg();
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
                // §1451
                if ((self.etex_p()
                    || (self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)] == 42i32))
                    && (self.format_ident == 66704i32))
                {
                    {
                        self.no_new_control_sequence = false;
                        // §1399
                        self.primitive(66743i32, extension, pic_file_code);
                        self.primitive(66744i32, extension, pdf_file_code);
                        self.primitive(66745i32, extension, glyph_code);
                        self.primitive(66746i32, extension, XeTeX_linebreak_locale_extension_code);
                        self.primitive(66747i32, assign_toks, XeTeX_inter_char_loc);
                        self.primitive(66748i32, extension, pdf_save_pos_node);
                        // §1452
                        self.primitive(66804i32, last_item, last_node_type_code);
                        self.primitive(66805i32, last_item, eTeX_version_code);
                        self.primitive(66106i32, convert, eTeX_revision_code);
                        self.primitive(66806i32, last_item, XeTeX_version_code);
                        self.primitive(66807i32, convert, XeTeX_revision_code);
                        self.primitive(66808i32, last_item, XeTeX_count_glyphs_code);
                        self.primitive(66809i32, last_item, XeTeX_count_variations_code);
                        self.primitive(66810i32, last_item, XeTeX_variation_code);
                        self.primitive(66811i32, last_item, XeTeX_find_variation_by_name_code);
                        self.primitive(66812i32, last_item, XeTeX_variation_min_code);
                        self.primitive(66813i32, last_item, XeTeX_variation_max_code);
                        self.primitive(66814i32, last_item, XeTeX_variation_default_code);
                        self.primitive(66815i32, last_item, XeTeX_count_features_code);
                        self.primitive(66816i32, last_item, XeTeX_feature_code_code);
                        self.primitive(66817i32, last_item, XeTeX_find_feature_by_name_code);
                        self.primitive(66818i32, last_item, XeTeX_is_exclusive_feature_code);
                        self.primitive(66819i32, last_item, XeTeX_count_selectors_code);
                        self.primitive(66820i32, last_item, XeTeX_selector_code_code);
                        self.primitive(66821i32, last_item, XeTeX_find_selector_by_name_code);
                        self.primitive(66822i32, last_item, XeTeX_is_default_selector_code);
                        self.primitive(66823i32, convert, XeTeX_variation_name_code);
                        self.primitive(66824i32, convert, XeTeX_feature_name_code);
                        self.primitive(66825i32, convert, XeTeX_selector_name_code);
                        self.primitive(66826i32, last_item, XeTeX_OT_count_scripts_code);
                        self.primitive(66827i32, last_item, XeTeX_OT_count_languages_code);
                        self.primitive(66828i32, last_item, XeTeX_OT_count_features_code);
                        self.primitive(66829i32, last_item, XeTeX_OT_script_code);
                        self.primitive(66830i32, last_item, XeTeX_OT_language_code);
                        self.primitive(66831i32, last_item, XeTeX_OT_feature_code);
                        self.primitive(66832i32, last_item, XeTeX_map_char_to_glyph_code);
                        self.primitive(66833i32, last_item, XeTeX_glyph_index_code);
                        self.primitive(66834i32, last_item, XeTeX_glyph_bounds_code);
                        self.primitive(66835i32, convert, XeTeX_glyph_name_code);
                        self.primitive(66836i32, last_item, XeTeX_font_type_code);
                        self.primitive(66837i32, last_item, XeTeX_first_char_code);
                        self.primitive(66838i32, last_item, XeTeX_last_char_code);
                        self.primitive(66839i32, last_item, XeTeX_pdf_page_count_code);
                        // §1467
                        self.primitive(66849i32, assign_toks, every_eof_loc);
                        self.primitive(66850i32, assign_int, 7892325i32);
                        self.primitive(66851i32, assign_int, 7892326i32);
                        self.primitive(66852i32, assign_int, 7892327i32);
                        self.primitive(66853i32, assign_int, 7892328i32);
                        self.primitive(66854i32, assign_int, 7892329i32);
                        self.primitive(66855i32, assign_int, 7892330i32);
                        self.primitive(66856i32, assign_int, 7892331i32);
                        self.primitive(66857i32, assign_int, 7892332i32);
                        self.primitive(66858i32, assign_int, 7892333i32);
                        self.primitive(66859i32, assign_int, 7892335i32);
                        // §1473
                        self.primitive(66873i32, last_item, current_group_level_code);
                        self.primitive(66874i32, last_item, current_group_type_code);
                        // §1476
                        self.primitive(66875i32, last_item, current_if_level_code);
                        self.primitive(66876i32, last_item, current_if_type_code);
                        self.primitive(66877i32, last_item, current_if_branch_code);
                        // §1479
                        self.primitive(66878i32, last_item, font_char_wd_code);
                        self.primitive(66879i32, last_item, font_char_ht_code);
                        self.primitive(66880i32, last_item, font_char_dp_code);
                        self.primitive(66881i32, last_item, font_char_ic_code);
                        // §1482
                        self.primitive(66882i32, last_item, par_shape_length_code);
                        self.primitive(66883i32, last_item, par_shape_indent_code);
                        self.primitive(66884i32, last_item, par_shape_dimen_code);
                        // §1485
                        self.primitive(66885i32, xray, show_groups);
                        // §1494
                        self.primitive(66887i32, xray, show_tokens);
                        // §1496
                        self.primitive(66888i32, the, 1i32);
                        self.primitive(66889i32, the, show_tokens);
                        // §1499
                        self.primitive(66890i32, xray, show_ifs);
                        // §1502
                        self.primitive(66894i32, set_page_int, 2i32);
                        // §1507
                        self.primitive(66281i32, left_right, middle_noad);
                        // §1511
                        self.primitive(66898i32, assign_int, 7892334i32);
                        self.primitive(66899i32, assign_int, 7892339i32);
                        self.primitive(66900i32, assign_int, 7892341i32);
                        self.primitive(66901i32, assign_int, 7892342i32);
                        self.primitive(66902i32, assign_int, 7892343i32);
                        self.primitive(66903i32, assign_int, 7892340i32);
                        self.primitive(66904i32, assign_int, 7892344i32);
                        self.primitive(66905i32, assign_int, 7892347i32);
                        self.primitive(66906i32, assign_int, 7892348i32);
                        self.primitive(66907i32, assign_int, 7892349i32);
                        self.primitive(66908i32, assign_int, 7892350i32);
                        self.primitive(66749i32, extension, XeTeX_input_encoding_extension_code);
                        self.primitive(66750i32, extension, XeTeX_default_encoding_extension_code);
                        self.primitive(66909i32, valign, begin_L_code);
                        self.primitive(66910i32, valign, end_L_code);
                        self.primitive(66911i32, valign, begin_R_code);
                        self.primitive(66912i32, valign, end_R_code);
                        // §1558
                        self.primitive(66921i32, input, 2i32);
                        // §1570
                        self.primitive(66923i32, read_to_cs, 1i32);
                        // §1573
                        self.primitive(66155i32, expand_after, 1i32);
                        self.primitive(66924i32, if_test, if_def_code);
                        self.primitive(66925i32, if_test, if_cs_code);
                        self.primitive(66926i32, if_test, if_font_char_code);
                        self.primitive(66927i32, if_test, if_in_csname_code);
                        // §1581
                        self.primitive(66617i32, prefix, 8i32);
                        // §1589
                        self.primitive(66933i32, last_item, 67i32);
                        self.primitive(66934i32, last_item, 68i32);
                        self.primitive(66935i32, last_item, 69i32);
                        self.primitive(66936i32, last_item, 70i32);
                        // §1612
                        self.primitive(66940i32, last_item, glue_stretch_order_code);
                        self.primitive(66941i32, last_item, glue_shrink_order_code);
                        self.primitive(66942i32, last_item, glue_stretch_code);
                        self.primitive(66943i32, last_item, glue_shrink_code);
                        // §1616
                        self.primitive(66944i32, last_item, mu_to_glue_code);
                        self.primitive(66945i32, last_item, glue_to_mu_code);
                        // §1620
                        self.primitive(66946i32, mark, marks_code);
                        self.primitive(66947i32, top_bot_mark, 5i32);
                        self.primitive(66948i32, top_bot_mark, 6i32);
                        self.primitive(66949i32, top_bot_mark, 7i32);
                        self.primitive(66950i32, top_bot_mark, 8i32);
                        self.primitive(66951i32, top_bot_mark, 9i32);
                        // §1672
                        self.primitive(66956i32, un_vbox, last_box_code);
                        self.primitive(66957i32, un_vbox, vsplit_code);
                        // §1675
                        self.primitive(66958i32, set_shape, inter_line_penalties_loc);
                        self.primitive(66959i32, set_shape, club_penalties_loc);
                        self.primitive(66960i32, set_shape, widow_penalties_loc);
                        self.primitive(66961i32, set_shape, display_widow_penalties_loc);
                        // §1451
                        if (self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)] == 42i32)
                        {
                            self.cur_input.loc_field =
                                (self.cur_input.loc_field).wrapping_add(1i32);
                        }
                        self.eTeX_mode = 1i32;
                        // §1624
                        self.max_reg_num = 32767i32;
                        self.max_reg_help_line = 66953i32;
                    }
                }
                // §1451
                if (!self.no_new_control_sequence) {
                    self.no_new_control_sequence = true;
                } else {
                    // §1391
                    if (((self.format_ident == 0i32)
                        || (self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)]
                            == 38i32))
                        || self.dump_line)
                    {
                        {
                            if (self.format_ident != 0i32) {
                                self.initialize();
                            }
                            if (!self.open_fmt_file()) {
                                break 'l_final_end_f;
                            }
                            if (!self.load_fmt_file()) {
                                {
                                    {
                                        let mut __f0 = ::core::mem::take(&mut self.fmt_file);
                                        let __r = self.w_close(&mut __f0);
                                        self.fmt_file = __f0;
                                        __r
                                    };
                                    break 'l_final_end_f;
                                }
                            }
                            {
                                let mut __f0 = ::core::mem::take(&mut self.fmt_file);
                                let __r = self.w_close(&mut __f0);
                                self.fmt_file = __f0;
                                __r
                            };
                            while ((self.cur_input.loc_field < self.cur_input.limit_field)
                                && (self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)]
                                    == 32i32))
                            {
                                self.cur_input.loc_field =
                                    (self.cur_input.loc_field).wrapping_add(1i32);
                            }
                        }
                    }
                }
                if (self.eTeX_mode == 1i32) {
                    {
                        crate::system::wr_str(&mut self.term_out, "entering extended mode");
                        crate::system::wr_ln(&mut self.term_out);
                    }
                }
                if ((self.eqtb[crate::ix::U(((7892312i32) - 1) as usize)].int() < 0i32)
                    || (self.eqtb[crate::ix::U(((7892312i32) - 1) as usize)].int() > 255i32))
                {
                    self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(1i32);
                } else {
                    {
                        let __ix1967 = self.cur_input.limit_field;
                        let __v1968 = self.eqtb[crate::ix::U(((7892312i32) - 1) as usize)].int();
                        self.buffer[crate::ix::U((__ix1967) as usize)] = __v1968;
                    }
                }
                self.fix_date_and_time();
                self.random_seed = ((self.microseconds).wrapping_mul(1000i32))
                    .wrapping_add((self.epochseconds % 1000000i32));
                self.init_randoms(self.random_seed);
                // §813
                self.magic_offset = (self.str_start
                    [crate::ix::U(((math_spacing).wrapping_sub(65536i32)) as usize)])
                .wrapping_sub((9i32).wrapping_mul(ord_noad));
                // §79
                if (self.interaction == batch_mode) {
                    self.selector = no_print;
                } else {
                    self.selector = term_only;
                }
                // §1391
                if ((self.cur_input.loc_field < self.cur_input.limit_field)
                    && (self.eqtb[crate::ix::U(
                        (((cat_code_base).wrapping_add(
                            self.buffer[crate::ix::U((self.cur_input.loc_field) as usize)],
                        )) - 1) as usize,
                    )]
                    .hh()
                    .rh()
                        != escape))
                {
                    self.start_input();
                }
            }
            // §1386
            self.history = spotless;
            self.main_control();
            self.final_cleanup();
            self.close_files_and_terminate();
        }
        self.ready_already = 0i32;
    }
}
