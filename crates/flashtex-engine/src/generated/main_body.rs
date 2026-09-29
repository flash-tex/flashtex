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
                // §1512
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
                // §129
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
                if ((9000i32 < 0i32) || (9000i32 > 268435455i32)) {
                    self.bad = 15i32;
                }
                if (font_max > 9000i32) {
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
                // §312
                if (630722i32 > 268435455i32) {
                    self.bad = 21i32;
                }
                // §548
                if (20i32 > file_name_size) {
                    self.bad = 31i32;
                }
                // §1427
                if ((2i32).wrapping_mul(268435455i32) < (4999999i32).wrapping_sub(mem_min)) {
                    self.bad = 41i32;
                }
                // §1512
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
                crate::system::wr_str(&mut self.term_out, "This is pdfTeX, Version 3.141592653");
                crate::system::wr_str(&mut self.term_out, "-2.6");
                crate::system::wr_str(&mut self.term_out, "-1.40.29");
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
            // §554
            self.job_name = 0i32;
            self.name_in_progress = false;
            self.log_opened = false;
            // §559
            self.output_file_name = 0i32;
            // §1517
            {
                // §353
                {
                    self.input_ptr = 0i32;
                    self.max_in_stack = 0i32;
                    self.in_open = 0i32;
                    self.open_parens = 0i32;
                    self.max_buf_stack = 0i32;
                    self.grp_stack[(0i32) as usize] = 0i32;
                    self.if_stack[(0i32) as usize] = 0i32;
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
                // §1648
                if ((self.buffer[(self.cur_input.loc_field) as usize] == 42i32) && (self.format_ident == 1673i32)) {
                    {
                        self.no_new_control_sequence = false;
                        // §1649
                        self.primitive(1944i32, 70i32, 3i32);
                        self.primitive(1945i32, 70i32, 20i32);
                        self.primitive(863i32, 110i32, 5i32);
                        // §1657
                        self.primitive(1947i32, 72i32, 627172i32);
                        self.primitive(1948i32, 73i32, 629110i32);
                        self.primitive(1949i32, 73i32, 629111i32);
                        self.primitive(1950i32, 73i32, 629112i32);
                        self.primitive(1951i32, 73i32, 629113i32);
                        self.primitive(1952i32, 73i32, 629114i32);
                        self.primitive(1953i32, 73i32, 629115i32);
                        self.primitive(1954i32, 73i32, 629116i32);
                        self.primitive(1955i32, 73i32, 629117i32);
                        self.primitive(1956i32, 73i32, 629118i32);
                        self.primitive(1957i32, 73i32, 629119i32);
                        // §1663
                        self.primitive(1971i32, 70i32, 21i32);
                        self.primitive(1972i32, 70i32, 22i32);
                        // §1666
                        self.primitive(1973i32, 70i32, 23i32);
                        self.primitive(1974i32, 70i32, 24i32);
                        self.primitive(1975i32, 70i32, 25i32);
                        // §1669
                        self.primitive(1976i32, 70i32, 28i32);
                        self.primitive(1977i32, 70i32, 29i32);
                        self.primitive(1978i32, 70i32, 30i32);
                        self.primitive(1979i32, 70i32, 31i32);
                        // §1672
                        self.primitive(1980i32, 70i32, 32i32);
                        self.primitive(1981i32, 70i32, 33i32);
                        self.primitive(1982i32, 70i32, 34i32);
                        // §1675
                        self.primitive(1983i32, 19i32, 4i32);
                        // §1684
                        self.primitive(1985i32, 19i32, 5i32);
                        // §1686
                        self.primitive(1986i32, 111i32, 1i32);
                        self.primitive(1987i32, 111i32, 5i32);
                        // §1689
                        self.primitive(1988i32, 19i32, 6i32);
                        // §1692
                        self.primitive(1992i32, 82i32, 2i32);
                        // §1697
                        self.primitive(1275i32, 49i32, 1i32);
                        // §1701
                        self.primitive(1996i32, 73i32, 629120i32);
                        self.primitive(1997i32, 33i32, 6i32);
                        self.primitive(1998i32, 33i32, 7i32);
                        self.primitive(1999i32, 33i32, 10i32);
                        self.primitive(2000i32, 33i32, 11i32);
                        // §1747
                        self.primitive(2009i32, 106i32, 2i32);
                        // §1759
                        self.primitive(2011i32, 96i32, 1i32);
                        // §1762
                        self.primitive(920i32, 104i32, 1i32);
                        self.primitive(2012i32, 107i32, 17i32);
                        self.primitive(2013i32, 107i32, 18i32);
                        self.primitive(2014i32, 107i32, 19i32);
                        self.primitive(2015i32, 107i32, 20i32);
                        self.primitive(2016i32, 107i32, 22i32);
                        self.primitive(2017i32, 107i32, 23i32);
                        // §1770
                        self.primitive(1591i32, 93i32, 8i32);
                        // §1778
                        self.primitive(2023i32, 70i32, 39i32);
                        self.primitive(2024i32, 70i32, 40i32);
                        self.primitive(2025i32, 70i32, 41i32);
                        self.primitive(2026i32, 70i32, 42i32);
                        // §1801
                        self.primitive(2030i32, 70i32, 26i32);
                        self.primitive(2031i32, 70i32, 27i32);
                        self.primitive(2032i32, 70i32, 35i32);
                        self.primitive(2033i32, 70i32, 36i32);
                        // §1805
                        self.primitive(2034i32, 70i32, 37i32);
                        self.primitive(2035i32, 70i32, 38i32);
                        // §1809
                        self.primitive(2036i32, 18i32, 5i32);
                        self.primitive(2037i32, 112i32, 5i32);
                        self.primitive(2038i32, 112i32, 6i32);
                        self.primitive(2039i32, 112i32, 7i32);
                        self.primitive(2040i32, 112i32, 8i32);
                        self.primitive(2041i32, 112i32, 9i32);
                        // §1861
                        self.primitive(2045i32, 24i32, 2i32);
                        self.primitive(2046i32, 24i32, 3i32);
                        // §1864
                        self.primitive(2047i32, 84i32, 627429i32);
                        self.primitive(2048i32, 84i32, 627430i32);
                        self.primitive(2049i32, 84i32, 627431i32);
                        self.primitive(2050i32, 84i32, 627432i32);
                        // §1648
                        self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(1i32);
                        self.eTeX_mode = 1i32;
                        // §1813
                        self.max_reg_num = 32767i32;
                        self.max_reg_help_line = 2042i32;
                    }
                }
                // §1648
                if (!self.no_new_control_sequence) {
                    self.no_new_control_sequence = true;
                } else {
                    // §1517
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
                                    { let mut __f0 = ::core::mem::take(&mut self.fmt_file); let __r = self.w_close(&mut __f0); self.fmt_file = __f0; __r };
                                    break 'l_final_end_f;
                                }
                            }
                            { let mut __f0 = ::core::mem::take(&mut self.fmt_file); let __r = self.w_close(&mut __f0); self.fmt_file = __f0; __r };
                            while ((self.cur_input.loc_field < self.cur_input.limit_field) && (self.buffer[(self.cur_input.loc_field) as usize] == 32i32)) {
                                self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(1i32);
                            }
                        }
                    }
                }
                if (self.pdf_output_option != 0i32) {
                    { let __v2307 = self.pdf_output_value; self.eqtb[((629073i32) - 1) as usize].set_int(__v2307); }
                }
                if (self.pdf_draftmode_option != 0i32) {
                    { let __v2308 = self.pdf_draftmode_value; self.eqtb[((629099i32) - 1) as usize].set_int(__v2308); }
                }
                self.pdf_init_map_file();
                if (self.eTeX_mode == 1i32) {
                    {
                        crate::system::wr_str(&mut self.term_out, "entering extended mode");
                        crate::system::wr_ln(&mut self.term_out);
                    }
                }
                if ((self.eqtb[((629066i32) - 1) as usize].int() < 0i32) || (self.eqtb[((629066i32) - 1) as usize].int() > 255i32)) {
                    self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(1i32);
                } else {
                    { let __ix2309 = self.cur_input.limit_field; let __v2310 = self.eqtb[((629066i32) - 1) as usize].int(); self.buffer[(__ix2309) as usize] = __v2310; }
                }
                self.fix_date_and_time();
                self.random_seed = ((self.microseconds).wrapping_mul(1000i32)).wrapping_add((self.epochseconds % 1000000i32));
                self.init_randoms(self.random_seed);
                // §941
                self.magic_offset = (self.str_start[(1290i32) as usize]).wrapping_sub((9i32).wrapping_mul(16i32));
                // §75
                if (self.interaction == 0i32) {
                    self.selector = 16i32;
                } else {
                    self.selector = 17i32;
                }
                // §1517
                if ((self.cur_input.loc_field < self.cur_input.limit_field) && (self.eqtb[(((627738i32).wrapping_add(self.buffer[(self.cur_input.loc_field) as usize])) - 1) as usize].hh().rh() != 0i32)) {
                    self.start_input();
                }
            }
            // §1512
            self.history = 0i32;
            self.main_control();
            self.final_cleanup();
            self.close_files_and_terminate();
        }
        self.ready_already = 0i32;
    }
}
