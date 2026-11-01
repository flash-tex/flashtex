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
        // §6
        self.open_standard_files();
        self.pool_size = POOL_SIZE;
        self.buf_size = BUF_SIZE;
        self.max_bib_files = MAX_BIB_FILES;
        self.max_glob_strs = MAX_GLOB_STRS;
        self.max_fields = MAX_FIELDS;
        self.max_cites = MAX_CITES;
        self.wiz_fn_space = WIZ_FN_SPACE;
        self.lit_stk_size = LIT_STK_SIZE;
        // §94
        self.parse_arguments();
        // §6
        self.setup_params();
        self.bib_file = vec![Default::default(); ((self.max_bib_files) as usize) + 1];
        self.bib_list.alloc_len(((self.max_bib_files) as usize) + 1);
        self.wiz_functions.alloc_len(((self.wiz_fn_space) as usize) + 1);
        self.field_info.alloc_len(((self.max_fields) as usize) + 1);
        self.s_preamble.alloc_len(((self.max_bib_files) as usize) + 1);
        self.str_pool.alloc_len(((self.pool_size) as usize) + 1);
        self.buffer.alloc_len(((self.buf_size) as usize) + 1);
        self.sv_buffer.alloc_len(((self.buf_size) as usize) + 1);
        self.ex_buf.alloc_len(((self.buf_size) as usize) + 1);
        self.out_buf.alloc_len(((self.buf_size) as usize) + 1);
        self.name_tok.alloc_len(((self.buf_size) as usize) + 1);
        self.name_sep_char.alloc_len(((self.buf_size) as usize) + 1);
        self.glb_str_ptr.alloc_len((((self.max_glob_strs).wrapping_sub(1i32)) as usize) + 1);
        self.global_strs.alloc_len(((((self.max_glob_strs).wrapping_mul((self.glob_str_size).wrapping_add(1i32))).wrapping_sub(1i32)) as usize) + 1);
        self.glb_str_end.alloc_len((((self.max_glob_strs).wrapping_sub(1i32)) as usize) + 1);
        self.cite_list.alloc_len(((self.max_cites) as usize) + 1);
        self.type_list.alloc_len(((self.max_cites) as usize) + 1);
        self.entry_exists.alloc_len(((self.max_cites) as usize) + 1);
        self.cite_info.alloc_len(((self.max_cites) as usize) + 1);
        self.str_start.alloc_len(((self.max_strings) as usize) + 1);
        self.hash_next.alloc_len(((self.hash_max) as usize) + 1);
        self.hash_text.alloc_len(((self.hash_max) as usize) + 1);
        self.hash_ilk.alloc_len(((self.hash_max) as usize) + 1);
        self.ilk_info.alloc_len(((self.hash_max) as usize) + 1);
        self.fn_type.alloc_len(((self.hash_max) as usize) + 1);
        self.lit_stack.alloc_len(((self.lit_stk_size) as usize) + 1);
        self.lit_stk_type.alloc_len(((self.lit_stk_size) as usize) + 1);
        self.compute_hash_prime();
        self.initialize();
        if self.verbose {
            {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "This is BibTeX, Version 0.99e");
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, "This is BibTeX, Version 0.99e");
                    }
                }
                {
                    {
                        crate::system::wr_str(&mut self.log_file, " (TeX Live 2026)");
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    {
                        crate::system::wr_str(&mut self.standard_output, " (TeX Live 2026)");
                        crate::system::wr_ln(&mut self.standard_output);
                    }
                }
            }
        } else {
            {
                {
                    {
                        crate::system::wr_str(&mut self.log_file, "This is BibTeX, Version 0.99e");
                    }
                }
                {
                    {
                        crate::system::wr_str(&mut self.log_file, " (TeX Live 2026)");
                        crate::system::wr_ln(&mut self.log_file);
                    }
                }
            }
        }
        {
            {
                let __w1 = self.max_strings;
                let __w3 = self.hash_size;
                let __w5 = self.hash_prime;
                crate::system::wr_str(&mut self.log_file, "Capacity: max_strings=");
                crate::system::wr_int(&mut self.log_file, __w1, 1i32);
                crate::system::wr_str(&mut self.log_file, ", hash_size=");
                crate::system::wr_int(&mut self.log_file, __w3, 1i32);
                crate::system::wr_str(&mut self.log_file, ", hash_prime=");
                crate::system::wr_int(&mut self.log_file, __w5, 1i32);
                crate::system::wr_ln(&mut self.log_file);
            }
        }
        self.catch_close_up_shop();
        {
            // §446
            if (self.read_performed && (!self.reading_completed)) {
                {
                    {
                        {
                            let __w1 = self.bib_line_num;
                            crate::system::wr_str(&mut self.log_file, "Aborted at line ");
                            crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                            crate::system::wr_str(&mut self.log_file, " of file ");
                        }
                        {
                            let __w1 = self.bib_line_num;
                            crate::system::wr_str(&mut self.standard_output, "Aborted at line ");
                            crate::system::wr_int(&mut self.standard_output, __w1, 0i32);
                            crate::system::wr_str(&mut self.standard_output, " of file ");
                        }
                    }
                    self.print_bib_name();
                }
            }
            self.trace_and_stat_printing();
            // §457
            match self.history {
                spotless => {
                }
                warning_message => {
                    {
                        if (self.err_count == 1i32) {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "(There was 1 warning)");
                                    crate::system::wr_ln(&mut self.log_file);
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "(There was 1 warning)");
                                    crate::system::wr_ln(&mut self.standard_output);
                                }
                            }
                        } else {
                            {
                                {
                                    let __w1 = self.err_count;
                                    crate::system::wr_str(&mut self.log_file, "(There were ");
                                    crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                                    crate::system::wr_str(&mut self.log_file, " warnings)");
                                    crate::system::wr_ln(&mut self.log_file);
                                }
                                {
                                    let __w1 = self.err_count;
                                    crate::system::wr_str(&mut self.standard_output, "(There were ");
                                    crate::system::wr_int(&mut self.standard_output, __w1, 0i32);
                                    crate::system::wr_str(&mut self.standard_output, " warnings)");
                                    crate::system::wr_ln(&mut self.standard_output);
                                }
                            }
                        }
                    }
                }
                error_message => {
                    {
                        if (self.err_count == 1i32) {
                            {
                                {
                                    crate::system::wr_str(&mut self.log_file, "(There was 1 error message)");
                                    crate::system::wr_ln(&mut self.log_file);
                                }
                                {
                                    crate::system::wr_str(&mut self.standard_output, "(There was 1 error message)");
                                    crate::system::wr_ln(&mut self.standard_output);
                                }
                            }
                        } else {
                            {
                                {
                                    let __w1 = self.err_count;
                                    crate::system::wr_str(&mut self.log_file, "(There were ");
                                    crate::system::wr_int(&mut self.log_file, __w1, 0i32);
                                    crate::system::wr_str(&mut self.log_file, " error messages)");
                                    crate::system::wr_ln(&mut self.log_file);
                                }
                                {
                                    let __w1 = self.err_count;
                                    crate::system::wr_str(&mut self.standard_output, "(There were ");
                                    crate::system::wr_int(&mut self.standard_output, __w1, 0i32);
                                    crate::system::wr_str(&mut self.standard_output, " error messages)");
                                    crate::system::wr_ln(&mut self.standard_output);
                                }
                            }
                        }
                    }
                }
                fatal_message => {
                    {
                        {
                            crate::system::wr_str(&mut self.log_file, "(That was a fatal error)");
                            crate::system::wr_ln(&mut self.log_file);
                        }
                        {
                            crate::system::wr_str(&mut self.standard_output, "(That was a fatal error)");
                            crate::system::wr_ln(&mut self.standard_output);
                        }
                    }
                }
                _ => {
                    {
                        {
                            {
                                crate::system::wr_str(&mut self.log_file, "History is bunk");
                            }
                            {
                                crate::system::wr_str(&mut self.standard_output, "History is bunk");
                            }
                        }
                        self.print_confusion();
                    }
                }
            }
            // §446
            { let mut __f0 = ::core::mem::take(&mut self.log_file); let __r = self.a_close(&mut __f0); self.log_file = __f0; __r };
        }
        // §6
        if (self.history > 1i32) {
            self.uexit(self.history);
        }
    }
}
