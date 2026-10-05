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
    /// The program begins with a normal \PASCAL\ program heading, whose
    /// components will be filled in later, using the conventions of \.{WEB}.
    /// For example, the portion of the program called `\X\glob:Global
    /// variables\X' below will be replaced by a sequence of variable declarations
    /// that starts in $\section\glob$ of this documentation. In this way, we are able
    /// to define each individual global variable when we are prepared to
    /// understand what it means; we do not have to define all of the globals at
    /// once.  Cross references in $\section\glob$, where it says ``See also
    /// sections \gglob, \dots,'' also make it possible to look at the set of
    /// all global variables, if desired.  Similar remarks apply to the other
    /// portions of the program heading.
    /// Actually the heading shown here is not quite normal: The `program` line
    /// does not mention any `output` file, because \ph\ would ask the \TeX\ user
    /// to specify a file name if `output` were specified here.
    /// ...
    // §4
    pub fn initialize(&mut self) {
        let mut i: i32 = 0; // §19
        let mut k: i32 = 0; // §188
        let mut z: hyph_pointer = 0; // §981
        // §1682
        {
            let __for_end_2 = font_max;
            i = font_base;
            while i <= __for_end_2 {
                {
                    self.font_layout_engine[crate::ix::U((i) as usize)] = 0i32;
                    self.font_mapping[crate::ix::U((i) as usize)] = 0i32;
                    self.font_flags[crate::ix::U((i) as usize)] = 0i32;
                    self.font_letter_space[crate::ix::U((i) as usize)] = 0i32;
                }
                i = i.wrapping_add(1);
            }
        }
        // §1691
        self.mapped_text.alloc_len(((mapped_text_size) as usize) + 1);
        self.xdv_buffer.alloc_len(((xdv_buffer_size) as usize) + 1);
        // §23
        {
            let __for_end_2 = 255i32;
            i = 0i32;
            while i <= __for_end_2 {
                self.xchr[crate::ix::U((i) as usize)] = ((i) as u8);
                i = i.wrapping_add(1);
            }
        }
        // §62
        self.doing_special = false;
        self.native_text_size = 128i32;
        self.native_text.alloc_len(((self.native_text_size) as usize) + 1);
        // §78
        if (self.interaction_option == unspecified_mode) {
            self.interaction = error_stop_mode;
        } else {
            self.interaction = self.interaction_option;
        }
        // §81
        self.deletions_allowed = true;
        self.set_box_allowed = true;
        self.error_count = 0i32;
        // §84
        self.help_ptr = 0i32;
        self.use_err_help = false;
        // §101
        self.interrupt = 0i32;
        self.OK_to_interrupt = true;
        // §122
        self.two_to_the[crate::ix::U((0i32) as usize)] = 1i32;
        {
            let __for_end_2 = 30i32;
            k = 1i32;
            while k <= __for_end_2 {
                { let __v0 = (2i32).wrapping_mul(self.two_to_the[crate::ix::U(((k).wrapping_sub(1i32)) as usize)]); self.two_to_the[crate::ix::U((k) as usize)] = __v0; }
                k = k.wrapping_add(1);
            }
        }
        self.spec_log[crate::ix::U(((1i32) - 1) as usize)] = 93032640i32;
        self.spec_log[crate::ix::U(((2i32) - 1) as usize)] = 38612034i32;
        self.spec_log[crate::ix::U(((3i32) - 1) as usize)] = 17922280i32;
        self.spec_log[crate::ix::U(((4i32) - 1) as usize)] = 8662214i32;
        self.spec_log[crate::ix::U(((5i32) - 1) as usize)] = 4261238i32;
        self.spec_log[crate::ix::U(((6i32) - 1) as usize)] = 2113709i32;
        self.spec_log[crate::ix::U(((7i32) - 1) as usize)] = 1052693i32;
        self.spec_log[crate::ix::U(((8i32) - 1) as usize)] = 525315i32;
        self.spec_log[crate::ix::U(((9i32) - 1) as usize)] = 262400i32;
        self.spec_log[crate::ix::U(((10i32) - 1) as usize)] = 131136i32;
        self.spec_log[crate::ix::U(((11i32) - 1) as usize)] = 65552i32;
        self.spec_log[crate::ix::U(((12i32) - 1) as usize)] = 32772i32;
        self.spec_log[crate::ix::U(((13i32) - 1) as usize)] = 16385i32;
        {
            let __for_end_2 = 27i32;
            k = 14i32;
            while k <= __for_end_2 {
                { let __v1 = self.two_to_the[crate::ix::U(((27i32).wrapping_sub(k)) as usize)]; self.spec_log[crate::ix::U(((k) - 1) as usize)] = __v1; }
                k = k.wrapping_add(1);
            }
        }
        self.spec_log[crate::ix::U(((28i32) - 1) as usize)] = 1i32;
        // §241
        self.nest_ptr = 0i32;
        self.max_nest_stack = 0i32;
        self.cur_list.mode_field = vmode;
        self.cur_list.head_field = contrib_head;
        self.cur_list.tail_field = contrib_head;
        self.cur_list.eTeX_aux_field = (268435455i32).wrapping_neg();
        self.cur_list.aux_field.set_int((65536000i32).wrapping_neg());
        self.cur_list.ml_field = 0i32;
        self.cur_list.pg_field = 0i32;
        self.shown_mode = 0i32;
        // §1045
        self.page_contents = empty;
        self.page_tail = page_head;
        self.mem[crate::ix::U((page_head) as usize)].set_hh_rh((268435455i32).wrapping_neg());
        self.last_glue = max_halfword;
        self.last_penalty = 0i32;
        self.last_kern = 0i32;
        self.last_node_type = (1i32).wrapping_neg();
        self.page_so_far[crate::ix::U((7i32) as usize)] = 0i32;
        self.page_max_depth = 0i32;
        // §280
        {
            let __for_end_2 = eqtb_size;
            k = int_base;
            while k <= __for_end_2 {
                self.xeq_level[crate::ix::U(((k) - 7892264) as usize)] = level_one;
                k = k.wrapping_add(1);
            }
        }
        // §284
        self.no_new_control_sequence = true;
        self.prim[crate::ix::U((0i32) as usize)].set_lh(0i32);
        self.prim[crate::ix::U((0i32) as usize)].set_rh(0i32);
        {
            let __for_end_2 = prim_size;
            k = 1i32;
            while k <= __for_end_2 {
                { let __v2 = self.prim[crate::ix::U((0i32) as usize)]; self.prim[crate::ix::U((k) as usize)] = __v2; }
                k = k.wrapping_add(1);
            }
        }
        self.hash[crate::ix::U(((hash_base) - 1179650) as usize)].set_lh(0i32);
        self.hash[crate::ix::U(((hash_base) - 1179650) as usize)].set_rh(0i32);
        {
            let __for_end_2 = hash_top;
            k = 1179651i32;
            while k <= __for_end_2 {
                { let __v3 = self.hash[crate::ix::U(((hash_base) - 1179650) as usize)]; self.hash[crate::ix::U(((k) - 1179650) as usize)] = __v3; }
                k = k.wrapping_add(1);
            }
        }
        // §302
        self.save_ptr = 0i32;
        self.cur_level = level_one;
        self.cur_group = bottom_level;
        self.cur_boundary = 0i32;
        self.max_save_stack = 0i32;
        // §317
        self.mag_set = 0i32;
        // §398
        self.is_in_csname = false;
        // §417
        self.cur_mark[crate::ix::U((top_mark_code) as usize)] = (268435455i32).wrapping_neg();
        self.cur_mark[crate::ix::U((first_mark_code) as usize)] = (268435455i32).wrapping_neg();
        self.cur_mark[crate::ix::U((bot_mark_code) as usize)] = (268435455i32).wrapping_neg();
        self.cur_mark[crate::ix::U((split_first_mark_code) as usize)] = (268435455i32).wrapping_neg();
        self.cur_mark[crate::ix::U((split_bot_mark_code) as usize)] = (268435455i32).wrapping_neg();
        // §473
        self.cur_val = 0i32;
        self.cur_val_level = int_val;
        self.radix = 0i32;
        self.cur_order = normal;
        // §516
        {
            let __for_end_2 = 16i32;
            k = 0i32;
            while k <= __for_end_2 {
                self.read_open[crate::ix::U((k) as usize)] = closed;
                k = k.wrapping_add(1);
            }
        }
        // §525
        self.cond_ptr = (268435455i32).wrapping_neg();
        self.if_limit = normal;
        self.cur_if = 0i32;
        self.if_line = 0i32;
        // §556
        crate::system::copy_str(&mut self.TEX_format_default, "TeXformats:plain.fmt");
        // §586
        {
            let __for_end_2 = font_max;
            k = font_base;
            while k <= __for_end_2 {
                { let __v4 = false; self.font_used[crate::ix::U((k) as usize)] = __v4; }
                k = k.wrapping_add(1);
            }
        }
        // §591
        self.null_character.set_b0(min_quarterword);
        self.null_character.set_b1(min_quarterword);
        self.null_character.set_b2(min_quarterword);
        self.null_character.set_b3(min_quarterword);
        // §629
        self.total_pages = 0i32;
        self.max_v = 0i32;
        self.max_h = 0i32;
        self.max_push = 0i32;
        self.last_bop = (1i32).wrapping_neg();
        self.doing_leaders = false;
        self.dead_cycles = 0i32;
        self.cur_s = (1i32).wrapping_neg();
        // §632
        self.half_buf = (dvi_buf_size / 2i32);
        self.dvi_limit = dvi_buf_size;
        self.dvi_ptr = 0i32;
        self.dvi_offset = 0i32;
        self.dvi_gone = 0i32;
        // §642
        self.down_ptr = (268435455i32).wrapping_neg();
        self.right_ptr = (268435455i32).wrapping_neg();
        // §687
        self.adjust_tail = (268435455i32).wrapping_neg();
        self.last_badness = 0i32;
        // §696
        self.pre_adjust_tail = (268435455i32).wrapping_neg();
        // §704
        self.pack_begin_line = 0i32;
        // §727
        self.empty_field.set_rh(empty);
        self.empty_field.set_lh((268435455i32).wrapping_neg());
        self.null_delimiter.set_b0(0i32);
        self.null_delimiter.set_b1(min_quarterword);
        self.null_delimiter.set_b2(0i32);
        self.null_delimiter.set_b3(min_quarterword);
        // §819
        self.align_ptr = (268435455i32).wrapping_neg();
        self.cur_align = (268435455i32).wrapping_neg();
        self.cur_span = (268435455i32).wrapping_neg();
        self.cur_loop = (268435455i32).wrapping_neg();
        self.cur_head = (268435455i32).wrapping_neg();
        self.cur_tail = (268435455i32).wrapping_neg();
        self.cur_pre_head = (268435455i32).wrapping_neg();
        self.cur_pre_tail = (268435455i32).wrapping_neg();
        // §941
        self.max_hyph_char = too_big_lang;
        // §982
        {
            let __for_end_2 = hyph_size;
            z = 0i32;
            while z <= __for_end_2 {
                {
                    self.hyph_word[crate::ix::U((z) as usize)] = 0i32;
                    self.hyph_list[crate::ix::U((z) as usize)] = (268435455i32).wrapping_neg();
                }
                z = z.wrapping_add(1);
            }
        }
        self.hyph_count = 0i32;
        // §1044
        self.output_active = false;
        self.output_can_end = false;
        self.insert_penalties = 0i32;
        // §1087
        self.ligature_present = false;
        self.cancel_boundary = false;
        self.lft_hit = false;
        self.rt_hit = false;
        self.ins_disc = false;
        // §1321
        self.after_token = 0i32;
        // §1336
        self.long_help_seen = false;
        // §1354
        self.format_ident = 0i32;
        // §1397
        {
            let __for_end_2 = 17i32;
            k = 0i32;
            while k <= __for_end_2 {
                { let __v5 = false; self.write_open[crate::ix::U((k) as usize)] = __v5; }
                k = k.wrapping_add(1);
            }
        }
        // §1412
        { let mut __f0 = ::core::mem::take(&mut self.epochseconds); let mut __f1 = ::core::mem::take(&mut self.microseconds); let __r = self.seconds_and_micros(&mut __f0, &mut __f1); self.epochseconds = __f0; self.microseconds = __f1; __r };
        self.init_start_time();
        // §1516
        self.LR_ptr = (268435455i32).wrapping_neg();
        self.LR_problems = 0i32;
        self.cur_dir = left_to_right;
        // §1562
        self.pseudo_files = (268435455i32).wrapping_neg();
        // §1628
        self.sa_root[crate::ix::U((mark_val) as usize)] = (268435455i32).wrapping_neg();
        self.sa_null.set_hh_lh((268435455i32).wrapping_neg());
        self.sa_null.set_hh_rh((268435455i32).wrapping_neg());
        // §1647
        self.sa_chain = (268435455i32).wrapping_neg();
        self.sa_level = level_zero;
        // §1671
        self.disc_ptr[crate::ix::U(((last_box_code) - 1) as usize)] = (268435455i32).wrapping_neg();
        self.disc_ptr[crate::ix::U(((vsplit_code) - 1) as usize)] = (268435455i32).wrapping_neg();
        // §1684
        self.expand_depth_count = 0i32;
        // §1686
        self.edit_name_start = 0i32;
        // §1695
        self.stop_at_space = true;
        // §1701
        self.mltex_p = false;
        self.mltex_enabled_p = false;
        // §1709
        self.synctex_tag_counter = 0i32;
        // §1717
        self.halting_on_error_p = false;
        // §189
        {
            let __for_end_2 = lo_mem_stat_max;
            k = 1i32;
            while k <= __for_end_2 {
                self.mem[crate::ix::U((k) as usize)].set_int(0i32);
                k = k.wrapping_add(1);
            }
        }
        k = mem_bot;
        while (k <= lo_mem_stat_max) {
            {
                self.mem[crate::ix::U((k) as usize)].set_hh_rh((268435454i32).wrapping_neg());
                self.mem[crate::ix::U((k) as usize)].set_hh_b0(normal);
                self.mem[crate::ix::U((k) as usize)].set_hh_b1(normal);
                k = (k).wrapping_add(4i32);
            }
        }
        self.mem[crate::ix::U((6i32) as usize)].set_int(unity);
        self.mem[crate::ix::U((fil_glue) as usize)].set_hh_b0(fil);
        self.mem[crate::ix::U((10i32) as usize)].set_int(unity);
        self.mem[crate::ix::U((fill_glue) as usize)].set_hh_b0(fill);
        self.mem[crate::ix::U((14i32) as usize)].set_int(unity);
        self.mem[crate::ix::U((ss_glue) as usize)].set_hh_b0(fil);
        self.mem[crate::ix::U((15i32) as usize)].set_int(unity);
        self.mem[crate::ix::U((ss_glue) as usize)].set_hh_b1(fil);
        self.mem[crate::ix::U((18i32) as usize)].set_int((65536i32).wrapping_neg());
        self.mem[crate::ix::U((fil_neg_glue) as usize)].set_hh_b0(fil);
        self.rover = 20i32;
        { let __ix6 = self.rover; self.mem[crate::ix::U((__ix6) as usize)].set_hh_rh(empty_flag); }
        { let __ix7 = self.rover; self.mem[crate::ix::U((__ix7) as usize)].set_hh_lh(1000i32); }
        { let __ix8 = (self.rover).wrapping_add(1i32); let __v9 = self.rover; self.mem[crate::ix::U((__ix8) as usize)].set_hh_lh(__v9); }
        { let __ix10 = (self.rover).wrapping_add(1i32); let __v11 = self.rover; self.mem[crate::ix::U((__ix10) as usize)].set_hh_rh(__v11); }
        self.lo_mem_max = (self.rover).wrapping_add(1000i32);
        { let __ix12 = self.lo_mem_max; self.mem[crate::ix::U((__ix12) as usize)].set_hh_rh((268435455i32).wrapping_neg()); }
        { let __ix13 = self.lo_mem_max; self.mem[crate::ix::U((__ix13) as usize)].set_hh_lh((268435455i32).wrapping_neg()); }
        {
            let __for_end_2 = mem_top;
            k = hi_mem_stat_min;
            while k <= __for_end_2 {
                { let __v14 = self.mem[crate::ix::U((self.lo_mem_max) as usize)]; self.mem[crate::ix::U((k) as usize)] = __v14; }
                k = k.wrapping_add(1);
            }
        }
        // §838
        self.mem[crate::ix::U((omit_template) as usize)].set_hh_lh(end_template_token);
        // §845
        self.mem[crate::ix::U((end_span) as usize)].set_hh_rh(65536i32);
        self.mem[crate::ix::U((end_span) as usize)].set_hh_lh((268435455i32).wrapping_neg());
        // §868
        self.mem[crate::ix::U((last_active) as usize)].set_hh_b0(hyphenated);
        self.mem[crate::ix::U((4999993i32) as usize)].set_hh_lh(max_halfword);
        self.mem[crate::ix::U((last_active) as usize)].set_hh_b1(0i32);
        // §1035
        self.mem[crate::ix::U((page_ins_head) as usize)].set_hh_b1(255i32);
        self.mem[crate::ix::U((page_ins_head) as usize)].set_hh_b0(split_up);
        self.mem[crate::ix::U((page_ins_head) as usize)].set_hh_rh(page_ins_head);
        // §1042
        self.mem[crate::ix::U((page_head) as usize)].set_hh_b0(glue_node);
        self.mem[crate::ix::U((page_head) as usize)].set_hh_b1(normal);
        // §189
        self.avail = (268435455i32).wrapping_neg();
        self.mem_end = mem_top;
        self.hi_mem_min = hi_mem_stat_min;
        self.var_used = 20i32;
        self.dyn_used = hi_mem_stat_usage;
        // §248
        self.eqtb[crate::ix::U(((undefined_control_sequence) - 1) as usize)].set_hh_b0(undefined_cs);
        self.eqtb[crate::ix::U(((undefined_control_sequence) - 1) as usize)].set_hh_rh((268435455i32).wrapping_neg());
        self.eqtb[crate::ix::U(((undefined_control_sequence) - 1) as usize)].set_hh_b1(level_zero);
        {
            let __for_end_2 = eqtb_top;
            k = active_base;
            while k <= __for_end_2 {
                { let __v15 = self.eqtb[crate::ix::U(((undefined_control_sequence) - 1) as usize)]; self.eqtb[crate::ix::U(((k) - 1) as usize)] = __v15; }
                k = k.wrapping_add(1);
            }
        }
        // §254
        self.eqtb[crate::ix::U(((glue_base) - 1) as usize)].set_hh_rh(zero_glue);
        self.eqtb[crate::ix::U(((glue_base) - 1) as usize)].set_hh_b1(level_one);
        self.eqtb[crate::ix::U(((glue_base) - 1) as usize)].set_hh_b0(glue_ref);
        {
            let __for_end_2 = 1206294i32;
            k = 1205765i32;
            while k <= __for_end_2 {
                { let __v16 = self.eqtb[crate::ix::U(((glue_base) - 1) as usize)]; self.eqtb[crate::ix::U(((k) - 1) as usize)] = __v16; }
                k = k.wrapping_add(1);
            }
        }
        { let __v17 = (self.mem[crate::ix::U((zero_glue) as usize)].hh().rh()).wrapping_add(531i32); self.mem[crate::ix::U((zero_glue) as usize)].set_hh_rh(__v17); }
        // §258
        self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].set_hh_rh((268435455i32).wrapping_neg());
        self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].set_hh_b0(shape_ref);
        self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)].set_hh_b1(level_one);
        {
            let __for_end_2 = 1206566i32;
            k = etex_pen_base;
            while k <= __for_end_2 {
                { let __v18 = self.eqtb[crate::ix::U(((par_shape_loc) - 1) as usize)]; self.eqtb[crate::ix::U(((k) - 1) as usize)] = __v18; }
                k = k.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 1206562i32;
            k = output_routine_loc;
            while k <= __for_end_2 {
                { let __v19 = self.eqtb[crate::ix::U(((undefined_control_sequence) - 1) as usize)]; self.eqtb[crate::ix::U(((k) - 1) as usize)] = __v19; }
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[crate::ix::U(((1206567i32) - 1) as usize)].set_hh_rh((268435455i32).wrapping_neg());
        self.eqtb[crate::ix::U(((box_base) - 1) as usize)].set_hh_b0(box_ref);
        self.eqtb[crate::ix::U(((box_base) - 1) as usize)].set_hh_b1(level_one);
        {
            let __for_end_2 = 1206822i32;
            k = 1206568i32;
            while k <= __for_end_2 {
                { let __v20 = self.eqtb[crate::ix::U(((box_base) - 1) as usize)]; self.eqtb[crate::ix::U(((k) - 1) as usize)] = __v20; }
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].set_hh_rh(null_font);
        self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].set_hh_b0(data);
        self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)].set_hh_b1(level_one);
        {
            let __for_end_2 = 1207591i32;
            k = math_font_base;
            while k <= __for_end_2 {
                { let __v21 = self.eqtb[crate::ix::U(((cur_font_loc) - 1) as usize)]; self.eqtb[crate::ix::U(((k) - 1) as usize)] = __v21; }
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[crate::ix::U(((cat_code_base) - 1) as usize)].set_hh_rh(0i32);
        self.eqtb[crate::ix::U(((cat_code_base) - 1) as usize)].set_hh_b0(data);
        self.eqtb[crate::ix::U(((cat_code_base) - 1) as usize)].set_hh_b1(level_one);
        {
            let __for_end_2 = 7892263i32;
            k = 1207593i32;
            while k <= __for_end_2 {
                { let __v22 = self.eqtb[crate::ix::U(((cat_code_base) - 1) as usize)]; self.eqtb[crate::ix::U(((k) - 1) as usize)] = __v22; }
                k = k.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 1114111i32;
            k = 0i32;
            while k <= __for_end_2 {
                {
                    self.eqtb[crate::ix::U((((cat_code_base).wrapping_add(k)) - 1) as usize)].set_hh_rh(other_char);
                    self.eqtb[crate::ix::U((((math_code_base).wrapping_add(k)) - 1) as usize)].set_hh_rh(k);
                    self.eqtb[crate::ix::U((((sf_code_base).wrapping_add(k)) - 1) as usize)].set_hh_rh(1000i32);
                }
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[crate::ix::U(((1207605i32) - 1) as usize)].set_hh_rh(car_ret);
        self.eqtb[crate::ix::U(((1207624i32) - 1) as usize)].set_hh_rh(spacer);
        self.eqtb[crate::ix::U(((1207684i32) - 1) as usize)].set_hh_rh(escape);
        self.eqtb[crate::ix::U(((1207629i32) - 1) as usize)].set_hh_rh(comment);
        self.eqtb[crate::ix::U(((1207719i32) - 1) as usize)].set_hh_rh(invalid_char);
        self.eqtb[crate::ix::U(((1207592i32) - 1) as usize)].set_hh_rh(ignore);
        {
            let __for_end_2 = 57i32;
            k = 48i32;
            while k <= __for_end_2 {
                { let __v23 = (k).wrapping_add(self.set_class_field(var_fam_class)); self.eqtb[crate::ix::U((((math_code_base).wrapping_add(k)) - 1) as usize)].set_hh_rh(__v23); }
                k = k.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 90i32;
            k = 65i32;
            while k <= __for_end_2 {
                {
                    self.eqtb[crate::ix::U((((cat_code_base).wrapping_add(k)) - 1) as usize)].set_hh_rh(letter);
                    self.eqtb[crate::ix::U(((((cat_code_base).wrapping_add(k)).wrapping_add(32i32)) - 1) as usize)].set_hh_rh(letter);
                    { let __v24 = ((k).wrapping_add(self.set_family_field(1i32))).wrapping_add(self.set_class_field(var_fam_class)); self.eqtb[crate::ix::U((((math_code_base).wrapping_add(k)) - 1) as usize)].set_hh_rh(__v24); }
                    { let __v25 = (((k).wrapping_add(32i32)).wrapping_add(self.set_family_field(1i32))).wrapping_add(self.set_class_field(var_fam_class)); self.eqtb[crate::ix::U(((((math_code_base).wrapping_add(k)).wrapping_add(32i32)) - 1) as usize)].set_hh_rh(__v25); }
                    self.eqtb[crate::ix::U((((lc_code_base).wrapping_add(k)) - 1) as usize)].set_hh_rh((k).wrapping_add(32i32));
                    self.eqtb[crate::ix::U(((((lc_code_base).wrapping_add(k)).wrapping_add(32i32)) - 1) as usize)].set_hh_rh((k).wrapping_add(32i32));
                    self.eqtb[crate::ix::U((((uc_code_base).wrapping_add(k)) - 1) as usize)].set_hh_rh(k);
                    self.eqtb[crate::ix::U(((((uc_code_base).wrapping_add(k)).wrapping_add(32i32)) - 1) as usize)].set_hh_rh(k);
                    self.eqtb[crate::ix::U((((sf_code_base).wrapping_add(k)) - 1) as usize)].set_hh_rh(999i32);
                }
                k = k.wrapping_add(1);
            }
        }
        // §266
        {
            let __for_end_2 = 7892607i32;
            k = int_base;
            while k <= __for_end_2 {
                self.eqtb[crate::ix::U(((k) - 1) as usize)].set_int(0i32);
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[crate::ix::U(((7892319i32) - 1) as usize)].set_int(256i32);
        self.eqtb[crate::ix::U(((7892320i32) - 1) as usize)].set_int((1i32).wrapping_neg());
        self.eqtb[crate::ix::U(((7892281i32) - 1) as usize)].set_int(1000i32);
        self.eqtb[crate::ix::U(((7892265i32) - 1) as usize)].set_int(10000i32);
        self.eqtb[crate::ix::U(((7892305i32) - 1) as usize)].set_int(1i32);
        self.eqtb[crate::ix::U(((7892304i32) - 1) as usize)].set_int(25i32);
        self.eqtb[crate::ix::U(((7892309i32) - 1) as usize)].set_int(92i32);
        self.eqtb[crate::ix::U(((7892312i32) - 1) as usize)].set_int(carriage_return);
        {
            let __for_end_2 = 1114111i32;
            k = 0i32;
            while k <= __for_end_2 {
                self.eqtb[crate::ix::U((((del_code_base).wrapping_add(k)) - 1) as usize)].set_int((1i32).wrapping_neg());
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[crate::ix::U(((7892654i32) - 1) as usize)].set_int(0i32);
        self.eqtb[crate::ix::U(((7892324i32) - 1) as usize)].set_int((1i32).wrapping_neg());
        // §276
        {
            let __for_end_2 = eqtb_size;
            k = dimen_base;
            while k <= __for_end_2 {
                self.eqtb[crate::ix::U(((k) - 1) as usize)].set_int(0i32);
                k = k.wrapping_add(1);
            }
        }
        // §285
        self.prim_used = prim_size;
        self.hash_used = frozen_control_sequence;
        self.hash_high = 0i32;
        self.cs_count = 0i32;
        self.eqtb[crate::ix::U(((frozen_dont_expand) - 1) as usize)].set_hh_b0(dont_expand);
        self.hash[crate::ix::U(((frozen_dont_expand) - 1179650) as usize)].set_rh(65805i32);
        self.eqtb[crate::ix::U(((frozen_primitive) - 1) as usize)].set_hh_b0(ignore_spaces);
        self.eqtb[crate::ix::U(((frozen_primitive) - 1) as usize)].set_hh_rh(1i32);
        self.eqtb[crate::ix::U(((frozen_primitive) - 1) as usize)].set_hh_b1(level_one);
        self.hash[crate::ix::U(((frozen_primitive) - 1179650) as usize)].set_rh(65806i32);
        // §587
        self.font_ptr = null_font;
        self.fmem_ptr = 7i32;
        self.font_name[crate::ix::U((null_font) as usize)] = 66187i32;
        self.font_area[crate::ix::U((null_font) as usize)] = 65626i32;
        self.hyphen_char[crate::ix::U((null_font) as usize)] = 45i32;
        self.skew_char[crate::ix::U((null_font) as usize)] = (1i32).wrapping_neg();
        self.bchar_label[crate::ix::U((null_font) as usize)] = non_address;
        self.font_bchar[crate::ix::U((null_font) as usize)] = non_char;
        self.font_false_bchar[crate::ix::U((null_font) as usize)] = non_char;
        self.font_bc[crate::ix::U((null_font) as usize)] = 1i32;
        self.font_ec[crate::ix::U((null_font) as usize)] = 0i32;
        self.font_size[crate::ix::U((null_font) as usize)] = 0i32;
        self.font_dsize[crate::ix::U((null_font) as usize)] = 0i32;
        self.char_base[crate::ix::U((null_font) as usize)] = 0i32;
        self.width_base[crate::ix::U((null_font) as usize)] = 0i32;
        self.height_base[crate::ix::U((null_font) as usize)] = 0i32;
        self.depth_base[crate::ix::U((null_font) as usize)] = 0i32;
        self.italic_base[crate::ix::U((null_font) as usize)] = 0i32;
        self.lig_kern_base[crate::ix::U((null_font) as usize)] = 0i32;
        self.kern_base[crate::ix::U((null_font) as usize)] = 0i32;
        self.exten_base[crate::ix::U((null_font) as usize)] = 0i32;
        self.font_glue[crate::ix::U((null_font) as usize)] = (268435455i32).wrapping_neg();
        self.font_params[crate::ix::U((null_font) as usize)] = 7i32;
        self.param_base[crate::ix::U((null_font) as usize)] = (1i32).wrapping_neg();
        {
            let __for_end_2 = 6i32;
            k = 0i32;
            while k <= __for_end_2 {
                self.font_info[crate::ix::U((k) as usize)].set_int(0i32);
                k = k.wrapping_add(1);
            }
        }
        // §1000
        {
            let __for_end_2 = trie_op_size;
            k = (trie_op_size).wrapping_neg();
            while k <= __for_end_2 {
                self.trie_op_hash[crate::ix::U(((k) + 35111) as usize)] = 0i32;
                k = k.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = biggest_lang;
            k = 0i32;
            while k <= __for_end_2 {
                self.trie_used[crate::ix::U((k) as usize)] = min_quarterword;
                k = k.wrapping_add(1);
            }
        }
        self.trie_op_ptr = 0i32;
        // §1005
        self.trie_not_ready = true;
        self.trie_l[crate::ix::U((0i32) as usize)] = 0i32;
        self.trie_c[crate::ix::U((0i32) as usize)] = 0i32;
        self.trie_ptr = 0i32;
        // §1270
        self.hash[crate::ix::U(((frozen_protection) - 1179650) as usize)].set_rh(66625i32);
        // §1355
        if self.ini_version() {
            self.format_ident = 66704i32;
        }
        // §1432
        self.hash[crate::ix::U(((end_write) - 1179650) as usize)].set_rh(66760i32);
        self.eqtb[crate::ix::U(((end_write) - 1) as usize)].set_hh_b1(level_one);
        self.eqtb[crate::ix::U(((end_write) - 1) as usize)].set_hh_b0(outer_call);
        self.eqtb[crate::ix::U(((end_write) - 1) as usize)].set_hh_rh((268435455i32).wrapping_neg());
        // §1463
        self.eTeX_mode = 0i32;
        // §1623
        self.max_reg_num = 255i32;
        self.max_reg_help_line = 66952i32;
        // §1629
        {
            let __for_end_2 = inter_char_val;
            i = int_val;
            while i <= __for_end_2 {
                self.sa_root[crate::ix::U((i) as usize)] = (268435455i32).wrapping_neg();
                i = i.wrapping_add(1);
            }
        }
        // §1665
        self.eqtb[crate::ix::U(((7892350i32) - 1) as usize)].set_int(63i32);
    }

    /// tex.ch's `scan_file_name_braced` (its part \.{[54/web2c]}): when
    /// `scan_file_name` finds a `left_brace`, the file name is a balanced token
    /// list, expanded as it is read, converted into a string and fed to `more_name`
    /// character by character, with spaces allowed.
    /// @<Declare web2c's file-name procedures
    // §1696
    pub fn scan_file_name_braced(&mut self) {
        let mut save_scanner_status: small_number = 0; // §1696
        let mut save_def_ref: halfword = 0; // §1696
        let mut save_cur_cs: halfword = 0; // §1696
        let mut s: str_number = 0; // §1696
        let mut p: halfword = 0; // §1696
        let mut i: i32 = 0; // §1696
        let mut save_stop_at_space: bool = false; // §1696
        let mut dummy: bool = false; // §1696
        save_scanner_status = self.scanner_status;
        save_def_ref = self.def_ref;
        save_cur_cs = self.cur_cs;
        self.cur_cs = self.warning_index;
        if (self.scan_toks(false, true) != 0i32) {
        }
        self.old_setting = self.selector;
        self.selector = new_string;
        self.show_token_list(self.mem[crate::ix::U((self.def_ref) as usize)].hh().rh(), (268435455i32).wrapping_neg(), (pool_size).wrapping_sub(self.pool_ptr));
        self.selector = self.old_setting;
        s = self.make_string();
        self.delete_token_ref(self.def_ref);
        self.def_ref = save_def_ref;
        self.cur_cs = save_cur_cs;
        self.scanner_status = save_scanner_status;
        save_stop_at_space = self.stop_at_space;
        self.stop_at_space = false;
        self.begin_name();
        {
            let __for_end_2 = (self.str_start[crate::ix::U((((s).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)]).wrapping_sub(1i32);
            i = self.str_start[crate::ix::U(((s).wrapping_sub(65536i32)) as usize)];
            while i <= __for_end_2 {
                dummy = self.more_name(self.str_pool[crate::ix::U((i) as usize)]);
                i = i.wrapping_add(1);
            }
        }
        self.stop_at_space = save_stop_at_space;
    }

    /// To end a line of text output, we call `print_ln`.
    /// @<Basic print...
    // §57
    pub fn print_ln(&mut self) {
        match self.selector {
            term_and_log => {
                {
                    {
                        crate::system::wr_ln(&mut self.term_out);
                    }
                    {
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    self.term_offset = 0i32;
                    self.file_offset = 0i32;
                }
            }
            log_only => {
                {
                    {
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    self.file_offset = 0i32;
                }
            }
            term_only => {
                {
                    {
                        crate::system::wr_ln(&mut self.term_out);
                    }
                    self.term_offset = 0i32;
                }
            }
            no_print | pseudo | new_string => {
            }
            _ => {
                {
                    crate::system::wr_ln(&mut self.write_file[crate::ix::U((self.selector) as usize)]);
                }
            }
        }
    }

    /// The `print_raw_char` procedure sends one character to the desired destination,
    /// using the `xchr` array to map it into an external character compatible with
    /// `input_ln`. All printing comes through `print_ln`, `print_char` or
    /// `print_visible_char`. When printing a multi-byte character, the boolean
    /// parameter `incr_offset` is set `false` except for the very last byte, to avoid
    /// calling `print_ln` in the middle of such character.
    /// @<Basic printing...
    // §58
    pub fn print_raw_char(&mut self, mut s: UnicodeScalar, mut incr_offset: bool) {
        match self.selector {
            term_and_log => {
                {
                    {
                        let __w0 = self.xchr[crate::ix::U((s) as usize)];
                        crate::system::wr_char(&mut self.term_out, __w0);
                    }
                    {
                        let __w0 = self.xchr[crate::ix::U((s) as usize)];
                        crate::system::wr_char(&mut self.log_file, __w0);
                    }
                    if incr_offset {
                        {
                            self.term_offset = (self.term_offset).wrapping_add(1i32);
                            self.file_offset = (self.file_offset).wrapping_add(1i32);
                        }
                    }
                    if (self.term_offset == self.max_print_line) {
                        {
                            {
                                crate::system::wr_ln(&mut self.term_out);
                            }
                            self.term_offset = 0i32;
                        }
                    }
                    if (self.file_offset == self.max_print_line) {
                        {
                            {
                                crate::system::wr_ln(&mut self.log_file);
                            }
                            self.file_offset = 0i32;
                        }
                    }
                }
            }
            log_only => {
                {
                    {
                        let __w0 = self.xchr[crate::ix::U((s) as usize)];
                        crate::system::wr_char(&mut self.log_file, __w0);
                    }
                    if incr_offset {
                        self.file_offset = (self.file_offset).wrapping_add(1i32);
                    }
                    if (self.file_offset == self.max_print_line) {
                        self.print_ln();
                    }
                }
            }
            term_only => {
                {
                    {
                        let __w0 = self.xchr[crate::ix::U((s) as usize)];
                        crate::system::wr_char(&mut self.term_out, __w0);
                    }
                    if incr_offset {
                        self.term_offset = (self.term_offset).wrapping_add(1i32);
                    }
                    if (self.term_offset == self.max_print_line) {
                        self.print_ln();
                    }
                }
            }
            no_print => {
            }
            pseudo => {
                if (self.tally < self.trick_count) {
                    self.trick_buf[crate::ix::U(((self.tally % self.error_line)) as usize)] = s;
                }
            }
            new_string => {
                {
                    if (self.pool_ptr < pool_size) {
                        {
                            if (s > 65535i32) {
                                {
                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = (((s).wrapping_sub(65536i32) / 1024i32)).wrapping_add(55296i32);
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = ((s % 1024i32)).wrapping_add(56320i32);
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                }
                            } else {
                                {
                                    self.str_pool[crate::ix::U((self.pool_ptr) as usize)] = s;
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                {
                    let __w0 = self.xchr[crate::ix::U((s) as usize)];
                    crate::system::wr_char(&mut self.write_file[crate::ix::U((self.selector) as usize)], __w0);
                }
            }
        }
        self.tally = (self.tally).wrapping_add(1i32);
    }

    /// The `print_char` procedure sends one character to the desired destination.
    /// Control sequence names, file names and string constructed with
    /// \.{\\string} might contain `ASCII_code` values that can't
    /// be printed using `print_raw_char`.  These characters will be printed
    /// in three- or four-symbol form like `\.{\^\^A}' or `\.{\^\^e4}',
    /// unless the -8bit option is enabled.
    /// Output that goes to the terminal and/or log file is treated differently
    /// when it comes to determining whether a character is printable.
    // §59
    pub fn print_char(&mut self, mut s: i32) {
        let mut l: small_number = 0; // §59
        'l_exit_f: {
            if ((self.selector > pseudo) && (!self.doing_special)) {
                {
                    if (s >= 65536i32) {
                        {
                            self.print_raw_char((55296i32).wrapping_add(((s).wrapping_sub(65536i32) / 1024i32)), true);
                            self.print_raw_char((56320i32).wrapping_add(((s).wrapping_sub(65536i32) % 1024i32)), true);
                        }
                    } else {
                        self.print_raw_char(s, true);
                    }
                    break 'l_exit_f;
                }
            }
            if (s == self.eqtb[crate::ix::U(((7892313i32) - 1) as usize)].int()) {
                if (self.selector < pseudo) {
                    {
                        self.print_ln();
                        break 'l_exit_f;
                    }
                }
            }
            if (((s < 32i32) && (((self.eight_bit_p) as i32) == 0i32)) && (!self.doing_special)) {
                {
                    self.print_raw_char(94i32, true);
                    self.print_raw_char(94i32, true);
                    self.print_raw_char((s).wrapping_add(64i32), true);
                }
            } else {
                if (s < 127i32) {
                    self.print_raw_char(s, true);
                } else {
                    if (s == 127i32) {
                        {
                            if ((((self.eight_bit_p) as i32) == 0i32) && (!self.doing_special)) {
                                {
                                    self.print_raw_char(94i32, true);
                                    self.print_raw_char(94i32, true);
                                    self.print_raw_char(63i32, true);
                                }
                            } else {
                                self.print_raw_char(s, true);
                            }
                        }
                    } else {
                        if (((s < 160i32) && (((self.eight_bit_p) as i32) == 0i32)) && (!self.doing_special)) {
                            {
                                self.print_raw_char(94i32, true);
                                self.print_raw_char(94i32, true);
                                l = ((s % 256i32) / 16i32);
                                if (l < 10i32) {
                                    self.print_raw_char((l).wrapping_add(48i32), true);
                                } else {
                                    self.print_raw_char((l).wrapping_add(87i32), true);
                                }
                                l = (s % 16i32);
                                if (l < 10i32) {
                                    self.print_raw_char((l).wrapping_add(48i32), true);
                                } else {
                                    self.print_raw_char((l).wrapping_add(87i32), true);
                                }
                            }
                        } else {
                            if (self.selector == pseudo) {
                                self.print_raw_char(s, true);
                            } else {
                                {
                                    if (s < 2048i32) {
                                        {
                                            self.print_raw_char((192i32).wrapping_add((s / 64i32)), false);
                                            self.print_raw_char((128i32).wrapping_add((s % 64i32)), true);
                                        }
                                    } else {
                                        if (s < 65536i32) {
                                            {
                                                self.print_raw_char((224i32).wrapping_add((s / 4096i32)), false);
                                                self.print_raw_char((128i32).wrapping_add(((s % 4096i32) / 64i32)), false);
                                                self.print_raw_char((128i32).wrapping_add((s % 64i32)), true);
                                            }
                                        } else {
                                            {
                                                self.print_raw_char((240i32).wrapping_add((s / 262144i32)), false);
                                                self.print_raw_char((128i32).wrapping_add(((s % 262144i32) / 4096i32)), false);
                                                self.print_raw_char((128i32).wrapping_add(((s % 4096i32) / 64i32)), false);
                                                self.print_raw_char((128i32).wrapping_add((s % 64i32)), true);
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

    /// An entire string is output by calling `print`. Note that if we are outputting
    /// the single standard ASCII character \.c, we could call `print("c")`, since
    /// `"c"=99` is the number of a single-character string, as explained above. But
    /// `print_char("c")` is quicker, so \TeX\ goes directly to the `print_char`
    /// routine when it knows that this is safe. (The present implementation
    /// assumes that it is always safe to print a visible ASCII character.)
    /// @<Basic print...
    // §63
    pub fn print(&mut self, mut s: i32) {
        let mut j: pool_pointer = 0; // §63
        let mut nl: i32 = 0; // §63
        'l_exit_f: {
            if (s >= self.str_ptr) {
                s = 65541i32;
            } else {
                if (s < biggest_char) {
                    if (s < 0i32) {
                        s = 65541i32;
                    } else {
                        {
                            if (self.selector > pseudo) {
                                {
                                    self.print_char(s);
                                    break 'l_exit_f;
                                }
                            }
                            if (s == self.eqtb[crate::ix::U(((7892313i32) - 1) as usize)].int()) {
                                if (self.selector < pseudo) {
                                    {
                                        self.print_ln();
                                        break 'l_exit_f;
                                    }
                                }
                            }
                            nl = self.eqtb[crate::ix::U(((7892313i32) - 1) as usize)].int();
                            self.eqtb[crate::ix::U(((7892313i32) - 1) as usize)].set_int((1i32).wrapping_neg());
                            self.print_char(s);
                            self.eqtb[crate::ix::U(((7892313i32) - 1) as usize)].set_int(nl);
                            break 'l_exit_f;
                        }
                    }
                }
            }
            j = self.str_start[crate::ix::U(((s).wrapping_sub(65536i32)) as usize)];
            while (j < self.str_start[crate::ix::U((((s).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)]) {
                {
                    if (((((self.str_pool[crate::ix::U((j) as usize)] >= 55296i32) && (self.str_pool[crate::ix::U((j) as usize)] <= 56319i32)) && ((j).wrapping_add(1i32) < self.str_start[crate::ix::U((((s).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)])) && (self.str_pool[crate::ix::U(((j).wrapping_add(1i32)) as usize)] >= 56320i32)) && (self.str_pool[crate::ix::U(((j).wrapping_add(1i32)) as usize)] <= 57343i32)) {
                        {
                            self.print_char((((65536i32).wrapping_add(((self.str_pool[crate::ix::U((j) as usize)]).wrapping_sub(55296i32)).wrapping_mul(1024i32))).wrapping_add(self.str_pool[crate::ix::U(((j).wrapping_add(1i32)) as usize)])).wrapping_sub(56320i32));
                            j = (j).wrapping_add(2i32);
                        }
                    } else {
                        {
                            self.print_char(self.str_pool[crate::ix::U((j) as usize)]);
                            j = (j).wrapping_add(1i32);
                        }
                    }
                }
            }
        }
    }

    /// The procedure `print_nl` is like `print`, but it makes sure that the
    /// string appears at the beginning of a new line.
    /// @<Basic print...
    // §66
    pub fn print_nl(&mut self, mut s: str_number) {
        if (((self.selector < no_print) || ((self.term_offset > 0i32) && (((self.selector) % 2) != 0))) || ((self.file_offset > 0i32) && (self.selector >= log_only))) {
            self.print_ln();
        }
        self.print(s);
    }

    /// The procedure `print_esc` prints a string that is preceded by
    /// the user's escape character (which is usually a backslash).
    /// @<Basic print...
    // §67
    pub fn print_esc(&mut self, mut s: str_number) {
        let mut c: i32 = 0; // §67
        // §269
        c = self.eqtb[crate::ix::U(((7892309i32) - 1) as usize)].int();
        // §67
        if (c >= 0i32) {
            if (c <= biggest_usv) {
                self.print_char(c);
            }
        }
        self.print(s);
    }

    /// An array of digits in the range `0..15` is printed by `print_the_digs`.
    /// @<Basic print...
    // §68
    pub fn print_the_digs(&mut self, mut k: eight_bits) {
        while (k > 0i32) {
            {
                k = (k).wrapping_sub(1i32);
                if (self.dig[crate::ix::U((k) as usize)] < 10i32) {
                    self.print_char((48i32).wrapping_add(self.dig[crate::ix::U((k) as usize)]));
                } else {
                    self.print_char((55i32).wrapping_add(self.dig[crate::ix::U((k) as usize)]));
                }
            }
        }
    }

    /// The following procedure, which prints out the decimal representation of a
    /// given integer `n`, has been written carefully so that it works properly
    /// if `n=0` or if `(-n)` would cause overflow. It does not apply `mod` or `div`
    /// to negative arguments, since such operations are not implemented consistently
    /// by all \PASCAL\ compilers.
    /// @<Basic print...
    // §69
    pub fn print_int(&mut self, mut n: i32) {
        let mut k: i32 = 0; // §69
        let mut m: i32 = 0; // §69
        k = 0i32;
        if (n < 0i32) {
            {
                self.print_char(45i32);
                if (n > (100000000i32).wrapping_neg()) {
                    n = (n).wrapping_neg();
                } else {
                    {
                        m = ((1i32).wrapping_neg()).wrapping_sub(n);
                        n = (m / 10i32);
                        m = ((m % 10i32)).wrapping_add(1i32);
                        k = 1i32;
                        if (m < 10i32) {
                            self.dig[crate::ix::U((0i32) as usize)] = m;
                        } else {
                            {
                                self.dig[crate::ix::U((0i32) as usize)] = 0i32;
                                n = (n).wrapping_add(1i32);
                            }
                        }
                    }
                }
            }
        }
        loop {
            self.dig[crate::ix::U((k) as usize)] = (n % 10i32);
            n = (n / 10i32);
            k = (k).wrapping_add(1i32);
            if (n == 0i32) { break; }
        }
        self.print_the_digs(k);
    }

    /// Single-character control sequences do not need to be looked up in a hash
    /// table, since we can use the character code itself as a direct address.
    /// The procedure `print_cs` prints the name of a control sequence, given
    /// a pointer to its address in `eqtb`. A space is printed after the name
    /// unless it is a single nonletter or an active character. This procedure
    /// might be invoked with invalid data, so it is ``extra robust.'' The
    /// individual characters must be printed one at a time using `print`, since
    /// they may be unprintable.
    /// @<Basic printing...
    // §292
    pub fn print_cs(&mut self, mut p: i32) {
        if (p < hash_base) {
            if (p >= single_base) {
                if (p == null_cs) {
                    {
                        self.print_esc(65809i32);
                        self.print_esc(65810i32);
                        self.print_char(32i32);
                    }
                } else {
                    {
                        self.print_esc((p).wrapping_sub(1114113i32));
                        if (self.eqtb[crate::ix::U(((((cat_code_base).wrapping_add(p)).wrapping_sub(1114113i32)) - 1) as usize)].hh().rh() == letter) {
                            self.print_char(32i32);
                        }
                    }
                }
            } else {
                if (p < active_base) {
                    self.print_esc(65811i32);
                } else {
                    self.print_char((p).wrapping_sub(1i32));
                }
            }
        } else {
            if (((p >= undefined_control_sequence) && (p <= eqtb_size)) || (p > eqtb_top)) {
                self.print_esc(65811i32);
            } else {
                if ((self.hash[crate::ix::U(((p) - 1179650) as usize)].rh() < 0i32) || (self.hash[crate::ix::U(((p) - 1179650) as usize)].rh() >= self.str_ptr)) {
                    self.print_esc(65812i32);
                } else {
                    {
                        if ((p >= prim_eqtb_base) && (p < frozen_null_font)) {
                            self.print_esc((self.prim[crate::ix::U(((p).wrapping_sub(1194662i32)) as usize)].rh()).wrapping_sub(1i32));
                        } else {
                            self.print_esc(self.hash[crate::ix::U(((p) - 1179650) as usize)].rh());
                        }
                        self.print_char(32i32);
                    }
                }
            }
        }
    }

    /// Here is a similar procedure; it avoids the error checks, and it never
    /// prints a space after the control sequence.
    /// @<Basic printing procedures
    // §293
    pub fn sprint_cs(&mut self, mut p: halfword) {
        if (p < hash_base) {
            if (p < single_base) {
                self.print_char((p).wrapping_sub(1i32));
            } else {
                if (p < null_cs) {
                    self.print_esc((p).wrapping_sub(1114113i32));
                } else {
                    {
                        self.print_esc(65809i32);
                        self.print_esc(65810i32);
                    }
                }
            }
        } else {
            if ((p >= prim_eqtb_base) && (p < frozen_null_font)) {
                self.print_esc((self.prim[crate::ix::U(((p).wrapping_sub(1194662i32)) as usize)].rh()).wrapping_sub(1i32));
            } else {
                self.print_esc(self.hash[crate::ix::U(((p) - 1179650) as usize)].rh());
            }
        }
    }

    /// Conversely, here is a routine that takes three strings and prints a file
    /// name that might have produced them. (The routine is system dependent, because
    /// some operating systems put the file area last instead of first.)
    /// @<Basic printing...
    // §553
    pub fn print_file_name(&mut self, mut n: i32, mut a: i32, mut e: i32) {
        let mut must_quote: bool = false; // §553
        let mut quote_char: i32 = 0; // §553
        let mut j: pool_pointer = 0; // §553
        must_quote = false;
        quote_char = 0i32;
        if (a != 0i32) {
            {
                j = self.str_start[crate::ix::U(((a).wrapping_sub(65536i32)) as usize)];
                while (((!must_quote) || (quote_char == 0i32)) && (j < self.str_start[crate::ix::U((((a).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)])) {
                    {
                        if (self.str_pool[crate::ix::U((j) as usize)] == 32i32) {
                            must_quote = true;
                        } else {
                            if ((self.str_pool[crate::ix::U((j) as usize)] == 34i32) || (self.str_pool[crate::ix::U((j) as usize)] == 39i32)) {
                                {
                                    must_quote = true;
                                    quote_char = (73i32).wrapping_sub(self.str_pool[crate::ix::U((j) as usize)]);
                                }
                            }
                        }
                        j = (j).wrapping_add(1i32);
                    }
                }
            }
        }
        if (n != 0i32) {
            {
                j = self.str_start[crate::ix::U(((n).wrapping_sub(65536i32)) as usize)];
                while (((!must_quote) || (quote_char == 0i32)) && (j < self.str_start[crate::ix::U((((n).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)])) {
                    {
                        if (self.str_pool[crate::ix::U((j) as usize)] == 32i32) {
                            must_quote = true;
                        } else {
                            if ((self.str_pool[crate::ix::U((j) as usize)] == 34i32) || (self.str_pool[crate::ix::U((j) as usize)] == 39i32)) {
                                {
                                    must_quote = true;
                                    quote_char = (73i32).wrapping_sub(self.str_pool[crate::ix::U((j) as usize)]);
                                }
                            }
                        }
                        j = (j).wrapping_add(1i32);
                    }
                }
            }
        }
        if (e != 0i32) {
            {
                j = self.str_start[crate::ix::U(((e).wrapping_sub(65536i32)) as usize)];
                while (((!must_quote) || (quote_char == 0i32)) && (j < self.str_start[crate::ix::U((((e).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)])) {
                    {
                        if (self.str_pool[crate::ix::U((j) as usize)] == 32i32) {
                            must_quote = true;
                        } else {
                            if ((self.str_pool[crate::ix::U((j) as usize)] == 34i32) || (self.str_pool[crate::ix::U((j) as usize)] == 39i32)) {
                                {
                                    must_quote = true;
                                    quote_char = (73i32).wrapping_sub(self.str_pool[crate::ix::U((j) as usize)]);
                                }
                            }
                        }
                        j = (j).wrapping_add(1i32);
                    }
                }
            }
        }
        if must_quote {
            {
                if (quote_char == 0i32) {
                    quote_char = 34i32;
                }
                self.print_char(quote_char);
            }
        }
        if (a != 0i32) {
            {
                let __for_end_3 = (self.str_start[crate::ix::U((((a).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)]).wrapping_sub(1i32);
                j = self.str_start[crate::ix::U(((a).wrapping_sub(65536i32)) as usize)];
                while j <= __for_end_3 {
                    {
                        if (self.str_pool[crate::ix::U((j) as usize)] == quote_char) {
                            {
                                self.print(quote_char);
                                quote_char = (73i32).wrapping_sub(quote_char);
                                self.print(quote_char);
                            }
                        }
                        if (((((self.str_pool[crate::ix::U((j) as usize)] >= 55296i32) && (self.str_pool[crate::ix::U((j) as usize)] <= 56319i32)) && ((j).wrapping_add(1i32) < self.str_start[crate::ix::U((((a).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)])) && (self.str_pool[crate::ix::U(((j).wrapping_add(1i32)) as usize)] >= 56320i32)) && (self.str_pool[crate::ix::U(((j).wrapping_add(1i32)) as usize)] <= 57343i32)) {
                            {
                                self.print_char((((65536i32).wrapping_add(((self.str_pool[crate::ix::U((j) as usize)]).wrapping_sub(55296i32)).wrapping_mul(1024i32))).wrapping_add(self.str_pool[crate::ix::U(((j).wrapping_add(1i32)) as usize)])).wrapping_sub(56320i32));
                                j = (j).wrapping_add(1i32);
                            }
                        } else {
                            self.print(self.str_pool[crate::ix::U((j) as usize)]);
                        }
                    }
                    j = j.wrapping_add(1);
                }
            }
        }
        if (n != 0i32) {
            {
                let __for_end_3 = (self.str_start[crate::ix::U((((n).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)]).wrapping_sub(1i32);
                j = self.str_start[crate::ix::U(((n).wrapping_sub(65536i32)) as usize)];
                while j <= __for_end_3 {
                    {
                        if (self.str_pool[crate::ix::U((j) as usize)] == quote_char) {
                            {
                                self.print(quote_char);
                                quote_char = (73i32).wrapping_sub(quote_char);
                                self.print(quote_char);
                            }
                        }
                        if (((((self.str_pool[crate::ix::U((j) as usize)] >= 55296i32) && (self.str_pool[crate::ix::U((j) as usize)] <= 56319i32)) && ((j).wrapping_add(1i32) < self.str_start[crate::ix::U((((n).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)])) && (self.str_pool[crate::ix::U(((j).wrapping_add(1i32)) as usize)] >= 56320i32)) && (self.str_pool[crate::ix::U(((j).wrapping_add(1i32)) as usize)] <= 57343i32)) {
                            {
                                self.print_char((((65536i32).wrapping_add(((self.str_pool[crate::ix::U((j) as usize)]).wrapping_sub(55296i32)).wrapping_mul(1024i32))).wrapping_add(self.str_pool[crate::ix::U(((j).wrapping_add(1i32)) as usize)])).wrapping_sub(56320i32));
                                j = (j).wrapping_add(1i32);
                            }
                        } else {
                            self.print(self.str_pool[crate::ix::U((j) as usize)]);
                        }
                    }
                    j = j.wrapping_add(1);
                }
            }
        }
        if (e != 0i32) {
            {
                let __for_end_3 = (self.str_start[crate::ix::U((((e).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)]).wrapping_sub(1i32);
                j = self.str_start[crate::ix::U(((e).wrapping_sub(65536i32)) as usize)];
                while j <= __for_end_3 {
                    {
                        if (self.str_pool[crate::ix::U((j) as usize)] == quote_char) {
                            {
                                self.print(quote_char);
                                quote_char = (73i32).wrapping_sub(quote_char);
                                self.print(quote_char);
                            }
                        }
                        if (((((self.str_pool[crate::ix::U((j) as usize)] >= 55296i32) && (self.str_pool[crate::ix::U((j) as usize)] <= 56319i32)) && ((j).wrapping_add(1i32) < self.str_start[crate::ix::U((((e).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)])) && (self.str_pool[crate::ix::U(((j).wrapping_add(1i32)) as usize)] >= 56320i32)) && (self.str_pool[crate::ix::U(((j).wrapping_add(1i32)) as usize)] <= 57343i32)) {
                            {
                                self.print_char((((65536i32).wrapping_add(((self.str_pool[crate::ix::U((j) as usize)]).wrapping_sub(55296i32)).wrapping_mul(1024i32))).wrapping_add(self.str_pool[crate::ix::U(((j).wrapping_add(1i32)) as usize)])).wrapping_sub(56320i32));
                                j = (j).wrapping_add(1i32);
                            }
                        } else {
                            self.print(self.str_pool[crate::ix::U((j) as usize)]);
                        }
                    }
                    j = j.wrapping_add(1);
                }
            }
        }
        if (quote_char != 0i32) {
            self.print_char(quote_char);
        }
    }

    /// \[35] Subroutines for math mode.
    /// In order to convert mlists to hlists, i.e., noads to nodes, we need several
    /// subroutines that are conveniently dealt with now.
    /// Let us first introduce the macros that make it easy to get at the parameters and
    /// other font information. A size code, which is a multiple of 16, is added to a
    /// family number to get an index into the table of internal font numbers
    /// for each combination of family and size.  (Be alert: Size codes get
    /// larger as the type gets smaller.)
    /// @<Basic printing procedures
    // §741
    pub fn print_size(&mut self, mut s: i32) {
        if (s == text_size) {
            self.print_esc(65704i32);
        } else {
            if (s == script_size) {
                self.print_esc(65705i32);
            } else {
                self.print_esc(65706i32);
            }
        }
    }

    /// Each new type of node that appears in our data structure must be capable
    /// of being displayed, copied, destroyed, and so on. The routines that we
    /// need for write-oriented whatsits are somewhat like those for mark nodes;
    /// other extensions might, of course, involve more subtlety here.
    /// @<Basic printing...
    // §1415
    pub fn print_write_whatsit(&mut self, mut s: str_number, mut p: halfword) {
        self.print_esc(s);
        if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() < 16i32) {
            self.print_int(self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh());
        } else {
            if (self.mem[crate::ix::U(((p).wrapping_add(1i32)) as usize)].hh().lh() == 16i32) {
                self.print_char(42i32);
            } else {
                self.print_char(45i32);
            }
        }
    }

    /// Each new type of node that appears in our data structure must be capable
    /// of being displayed, copied, destroyed, and so on. The routines that we
    /// need for write-oriented whatsits are somewhat like those for mark nodes;
    /// other extensions might, of course, involve more subtlety here.
    /// @<Basic printing...
    // §1415
    pub fn print_native_word(&mut self, mut p: halfword) {
        let mut i: i32 = 0; // §1415
        let mut c: i32 = 0; // §1415
        let mut cc: i32 = 0; // §1415
        {
            let __for_end_2 = (self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2()).wrapping_sub(1i32);
            i = 0i32;
            while i <= __for_end_2 {
                {
                    c = self.get_native_char(p, i);
                    if ((c >= 55296i32) && (c <= 56319i32)) {
                        {
                            if (i < (self.mem[crate::ix::U(((p).wrapping_add(4i32)) as usize)].qqqq().b2()).wrapping_sub(1i32)) {
                                {
                                    cc = self.get_native_char(p, (i).wrapping_add(1i32));
                                    if ((cc >= 56320i32) && (cc <= 57343i32)) {
                                        {
                                            c = ((65536i32).wrapping_add(((c).wrapping_sub(55296i32)).wrapping_mul(1024i32))).wrapping_add((cc).wrapping_sub(56320i32));
                                            self.print_char(c);
                                            i = (i).wrapping_add(1i32);
                                        }
                                    } else {
                                        self.print(46i32);
                                    }
                                }
                            } else {
                                self.print(46i32);
                            }
                        }
                    } else {
                        self.print_char(c);
                    }
                }
                i = i.wrapping_add(1);
            }
        }
    }

    /// The `print_sa_num` procedure prints the register number corresponding
    /// to an array element.
    /// @<Basic print...
    // §1633
    pub fn print_sa_num(&mut self, mut q: halfword) {
        let mut n: halfword = 0; // §1633
        if (self.mem[crate::ix::U((q) as usize)].hh().b0() < dimen_val_limit) {
            n = self.mem[crate::ix::U(((q).wrapping_add(1i32)) as usize)].hh().rh();
        } else {
            {
                n = (self.mem[crate::ix::U((q) as usize)].hh().b0() % 64i32);
                q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                n = (n).wrapping_add((64i32).wrapping_mul(self.mem[crate::ix::U((q) as usize)].hh().b0()));
                q = self.mem[crate::ix::U((q) as usize)].hh().rh();
                n = (n).wrapping_add(((64i32).wrapping_mul(64i32)).wrapping_mul((self.mem[crate::ix::U((q) as usize)].hh().b0()).wrapping_add((64i32).wrapping_mul(self.mem[crate::ix::U((self.mem[crate::ix::U((q) as usize)].hh().rh()) as usize)].hh().b0()))));
            }
        }
        self.print_int(n);
    }

    /// A helper for printing file:line:error style messages.  Look for a
    /// filename in `full_source_filename_stack`, and if we fail to find
    /// one fall back on the non-file:line:error style.
    /// @<Basic print...
    // §1719
    pub fn print_file_line(&mut self) {
        let mut level: i32 = 0; // §1719
        level = self.in_open;
        while ((level > 0i32) && (self.full_source_filename_stack[crate::ix::U((level) as usize)] == 0i32)) {
            level = (level).wrapping_sub(1i32);
        }
        if (level == 0i32) {
            self.print_nl(65544i32);
        } else {
            {
                self.print_nl(65626i32);
                self.print(self.full_source_filename_stack[crate::ix::U((level) as usize)]);
                self.print(58i32);
                if (level == self.in_open) {
                    self.print_int(self.line);
                } else {
                    self.print_int(self.line_stack[crate::ix::U((((level).wrapping_add(1i32)) - 1) as usize)]);
                }
                self.print(65593i32);
            }
        }
    }

    /// The `jump_out` procedure just cuts across all active procedure levels and
    /// goes to `end_of_TEX`. This is the only nontrivial `@!goto` statement in the
    /// whole program. It is used when there is no recovery from a particular error.
    /// Some \PASCAL\ compilers do not implement non-local `goto` statements.
    /// In such cases the body of `jump_out` should simply be
    /// ``close_files_and_terminate`;\thinspace' followed by a call on some system
    /// procedure that quietly terminates the program.
    /// @<Error hand...
    // §85
    pub fn jump_out(&mut self) {
        crate::system::end_of_TEX(self);
    }

    /// Here now is the general `error` routine.
    /// @<Error hand...
    // §86
    pub fn error(&mut self) {
        let mut c: UnicodeScalar = 0; // §86
        let mut s1: i32 = 0; // §86
        let mut s2: i32 = 0; // §86
        let mut s3: i32 = 0; // §86
        let mut s4: i32 = 0; // §86
        'l_exit_f: {
            if (self.history < error_message_issued) {
                self.history = error_message_issued;
            }
            self.print_char(46i32);
            self.show_context();
            if self.halt_on_error_p {
                {
                    if self.halting_on_error_p {
                        self.do_final_end();
                    }
                    self.halting_on_error_p = true;
                    // §94
                    if (self.interaction > batch_mode) {
                        self.selector = (self.selector).wrapping_sub(1i32);
                    }
                    if self.use_err_help {
                        {
                            self.print_ln();
                            self.give_err_help();
                        }
                    } else {
                        while (self.help_ptr > 0i32) {
                            {
                                self.help_ptr = (self.help_ptr).wrapping_sub(1i32);
                                self.print_nl(self.help_line[crate::ix::U((self.help_ptr) as usize)]);
                            }
                        }
                    }
                    self.print_ln();
                    if (self.interaction > batch_mode) {
                        self.selector = (self.selector).wrapping_add(1i32);
                    }
                    self.print_ln();
                    // §86
                    self.history = fatal_error_stop;
                    self.jump_out();
                }
            }
            if (self.interaction == error_stop_mode) {
                // §87
                while true {
                    {
                        'l_continue_b: loop {
                            if (self.interaction != error_stop_mode) {
                                break 'l_exit_f;
                            }
                            self.clear_for_error_prompt();
                            {
                                self.print(65546i32);
                                self.term_input();
                            }
                            if (self.last == self.first) {
                                break 'l_exit_f;
                            }
                            c = self.buffer[crate::ix::U((self.first) as usize)];
                            if (c >= 97i32) {
                                c = (c).wrapping_sub(32i32);
                            }
                            // §88
                            match c {
                                48 | 49 | 50 | 51 | 52 | 53 | 54 | 55 | 56 | 57 => {
                                    if self.deletions_allowed {
                                        // §92
                                        {
                                            s1 = self.cur_tok;
                                            s2 = self.cur_cmd;
                                            s3 = self.cur_chr;
                                            s4 = self.align_state;
                                            self.align_state = 1000000i32;
                                            self.OK_to_interrupt = false;
                                            if (((self.last > (self.first).wrapping_add(1i32)) && (self.buffer[crate::ix::U(((self.first).wrapping_add(1i32)) as usize)] >= 48i32)) && (self.buffer[crate::ix::U(((self.first).wrapping_add(1i32)) as usize)] <= 57i32)) {
                                                c = (((c).wrapping_mul(10i32)).wrapping_add(self.buffer[crate::ix::U(((self.first).wrapping_add(1i32)) as usize)])).wrapping_sub((48i32).wrapping_mul(11i32));
                                            } else {
                                                c = (c).wrapping_sub(48i32);
                                            }
                                            while (c > 0i32) {
                                                {
                                                    self.get_token();
                                                    c = (c).wrapping_sub(1i32);
                                                }
                                            }
                                            self.cur_tok = s1;
                                            self.cur_cmd = s2;
                                            self.cur_chr = s3;
                                            self.align_state = s4;
                                            self.OK_to_interrupt = true;
                                            {
                                                self.help_ptr = 2i32;
                                                self.help_line[crate::ix::U((1i32) as usize)] = 65559i32;
                                                self.help_line[crate::ix::U((0i32) as usize)] = 65560i32;
                                            }
                                            self.show_context();
                                            continue 'l_continue_b;
                                        }
                                    }
                                }
                                69 => {
                                    // §88
                                    if (self.base_ptr > 0i32) {
                                        if (self.input_stack[crate::ix::U((self.base_ptr) as usize)].name_field >= 256i32) {
                                            {
                                                self.edit_name_start = self.str_start[crate::ix::U(((self.input_stack[crate::ix::U((self.base_ptr) as usize)].name_field).wrapping_sub(65536i32)) as usize)];
                                                self.edit_name_length = self.length(self.input_stack[crate::ix::U((self.base_ptr) as usize)].name_field);
                                                self.edit_line = self.line;
                                                self.jump_out();
                                            }
                                        }
                                    }
                                }
                                72 => {
                                    // §93
                                    {
                                        if self.use_err_help {
                                            {
                                                self.give_err_help();
                                                self.use_err_help = false;
                                            }
                                        } else {
                                            {
                                                if (self.help_ptr == 0i32) {
                                                    {
                                                        self.help_ptr = 2i32;
                                                        self.help_line[crate::ix::U((1i32) as usize)] = 65561i32;
                                                        self.help_line[crate::ix::U((0i32) as usize)] = 65562i32;
                                                    }
                                                }
                                                loop {
                                                    self.help_ptr = (self.help_ptr).wrapping_sub(1i32);
                                                    self.print(self.help_line[crate::ix::U((self.help_ptr) as usize)]);
                                                    self.print_ln();
                                                    if (self.help_ptr == 0i32) { break; }
                                                }
                                            }
                                        }
                                        {
                                            self.help_ptr = 4i32;
                                            self.help_line[crate::ix::U((3i32) as usize)] = 65563i32;
                                            self.help_line[crate::ix::U((2i32) as usize)] = 65562i32;
                                            self.help_line[crate::ix::U((1i32) as usize)] = 65564i32;
                                            self.help_line[crate::ix::U((0i32) as usize)] = 65565i32;
                                        }
                                        continue 'l_continue_b;
                                    }
                                }
                                73 => {
                                    // §91
                                    {
                                        self.begin_file_reading();
                                        if (self.last > (self.first).wrapping_add(1i32)) {
                                            {
                                                self.cur_input.loc_field = (self.first).wrapping_add(1i32);
                                                self.buffer[crate::ix::U((self.first) as usize)] = 32i32;
                                            }
                                        } else {
                                            {
                                                {
                                                    self.print(65558i32);
                                                    self.term_input();
                                                }
                                                self.cur_input.loc_field = self.first;
                                            }
                                        }
                                        self.first = self.last;
                                        self.cur_input.limit_field = (self.last).wrapping_sub(1i32);
                                        break 'l_exit_f;
                                    }
                                }
                                81 | 82 | 83 => {
                                    // §90
                                    {
                                        self.error_count = 0i32;
                                        self.interaction = ((batch_mode).wrapping_add(c)).wrapping_sub(81i32);
                                        self.print(65553i32);
                                        match c {
                                            81 => {
                                                {
                                                    self.print_esc(65554i32);
                                                    self.selector = (self.selector).wrapping_sub(1i32);
                                                }
                                            }
                                            82 => {
                                                self.print_esc(65555i32);
                                            }
                                            83 => {
                                                self.print_esc(65556i32);
                                            }
                                            _ => {}
                                        }
                                        self.print(65557i32);
                                        self.print_ln();
                                        crate::system::break_out(&mut self.term_out);
                                        break 'l_exit_f;
                                    }
                                }
                                88 => {
                                    // §88
                                    {
                                        self.interaction = scroll_mode;
                                        self.jump_out();
                                    }
                                }
                                _ => {
                                }
                            }
                            // §89
                            {
                                self.print(65547i32);
                                self.print_nl(65548i32);
                                self.print_nl(65549i32);
                                if (self.base_ptr > 0i32) {
                                    if (self.input_stack[crate::ix::U((self.base_ptr) as usize)].name_field >= 256i32) {
                                        self.print(65550i32);
                                    }
                                }
                                if self.deletions_allowed {
                                    self.print_nl(65551i32);
                                }
                                self.print_nl(65552i32);
                            }
                            break 'l_continue_b;
                        }
                    }
                }
            }
            // §86
            self.error_count = (self.error_count).wrapping_add(1i32);
            if (self.error_count == 100i32) {
                {
                    self.print_nl(65545i32);
                    self.history = fatal_error_stop;
                    self.jump_out();
                }
            }
            // §94
            if (self.interaction > batch_mode) {
                self.selector = (self.selector).wrapping_sub(1i32);
            }
            if self.use_err_help {
                {
                    self.print_ln();
                    self.give_err_help();
                }
            } else {
                while (self.help_ptr > 0i32) {
                    {
                        self.help_ptr = (self.help_ptr).wrapping_sub(1i32);
                        self.print_nl(self.help_line[crate::ix::U((self.help_ptr) as usize)]);
                    }
                }
            }
            self.print_ln();
            if (self.interaction > batch_mode) {
                self.selector = (self.selector).wrapping_add(1i32);
            }
            self.print_ln();
        }
        // §86
    }

    /// The following procedure prints \TeX's last words before dying.
    // §97
    pub fn fatal_error(&mut self, mut s: str_number) {
        self.normalize_selector();
        {
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(65544i32);
            }
            self.print(65567i32);
        }
        {
            self.help_ptr = 1i32;
            self.help_line[crate::ix::U((0i32) as usize)] = s;
        }
        {
            if (self.interaction == error_stop_mode) {
                self.interaction = scroll_mode;
            }
            if self.log_opened {
                self.error();
            }
            self.history = fatal_error_stop;
            self.jump_out();
        }
    }

    /// Here is the most dreaded error message.
    /// @<Error hand...
    // §98
    pub fn overflow(&mut self, mut s: str_number, mut n: i32) {
        self.normalize_selector();
        {
            if (self.interaction == error_stop_mode) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(65544i32);
            }
            self.print(65568i32);
        }
        self.print(s);
        self.print_char(61i32);
        self.print_int(n);
        self.print_char(93i32);
        {
            self.help_ptr = 2i32;
            self.help_line[crate::ix::U((1i32) as usize)] = 65569i32;
            self.help_line[crate::ix::U((0i32) as usize)] = 65570i32;
        }
        {
            if (self.interaction == error_stop_mode) {
                self.interaction = scroll_mode;
            }
            if self.log_opened {
                self.error();
            }
            self.history = fatal_error_stop;
            self.jump_out();
        }
    }

    /// The program might sometime run completely amok, at which point there is
    /// no choice but to stop. If no previous error has been detected, that's bad
    /// news; a message is printed that is really intended for the \TeX\
    /// maintenance person instead of the user (unless the user has been
    /// particularly diabolical).  The index entries for `this can't happen' may
    /// help to pinpoint the problem.
    /// @<Error hand...
    // §99
    pub fn confusion(&mut self, mut s: str_number) {
        self.normalize_selector();
        if (self.history < error_message_issued) {
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(65571i32);
                }
                self.print(s);
                self.print_char(41i32);
                {
                    self.help_ptr = 1i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 65572i32;
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
                    self.print(65573i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 65574i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 65575i32;
                }
            }
        }
        {
            if (self.interaction == error_stop_mode) {
                self.interaction = scroll_mode;
            }
            if self.log_opened {
                self.error();
            }
            self.history = fatal_error_stop;
            self.jump_out();
        }
    }

    /// Several of the elementary string operations are performed using \.{WEB}
    /// macros instead of \PASCAL\ procedures, because many of the
    /// operations are done quite frequently and we want to avoid the
    /// overhead of procedure calls. For example, here is
    /// a simple macro that computes the length of a string.
    // §40
    pub fn length(&mut self, mut s: str_number) -> i32 {
        let mut length: i32 = 0;
        if (s >= 65536i32) {
            length = (self.str_start[crate::ix::U((((s).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)]).wrapping_sub(self.str_start[crate::ix::U(((s).wrapping_sub(65536i32)) as usize)]);
        } else {
            if ((s >= 32i32) && (s < 127i32)) {
                length = 1i32;
            } else {
                if (s <= 127i32) {
                    length = 3i32;
                } else {
                    if (s < 256i32) {
                        length = 4i32;
                    } else {
                        length = 8i32;
                    }
                }
            }
        }
        length
    }

    /// Once a sequence of characters has been appended to `str_pool`, it
    /// officially becomes a string when the function `make_string` is called.
    /// This function returns the identification number of the new string as its
    /// value.
    // §43
    pub fn make_string(&mut self) -> str_number {
        let mut make_string: str_number = 0;
        if (self.str_ptr == max_strings) {
            self.overflow(65540i32, (max_strings).wrapping_sub(self.init_str_ptr));
        }
        self.str_ptr = (self.str_ptr).wrapping_add(1i32);
        { let __ix26 = (self.str_ptr).wrapping_sub(65536i32); let __v27 = self.pool_ptr; self.str_start[crate::ix::U((__ix26) as usize)] = __v27; }
        make_string = (self.str_ptr).wrapping_sub(1i32);
        make_string
    }

    /// To destroy the most recently made string, we say `flush_string`.
    // §44
    pub fn append_str(&mut self, mut s: str_number) {
        let mut i: i32 = 0; // §44
        let mut j: pool_pointer = 0; // §44
        i = self.length(s);
        {
            if ((self.pool_ptr).wrapping_add(i) > pool_size) {
                self.overflow(65539i32, (pool_size).wrapping_sub(self.init_pool_ptr));
            }
        }
        j = self.str_start[crate::ix::U(((s).wrapping_sub(65536i32)) as usize)];
        while (i > 0i32) {
            {
                {
                    if (self.str_pool[crate::ix::U((j) as usize)] > 65535i32) {
                        {
                            { let __ix28 = self.pool_ptr; let __v29 = (((self.str_pool[crate::ix::U((j) as usize)]).wrapping_sub(65536i32) / 1024i32)).wrapping_add(55296i32); self.str_pool[crate::ix::U((__ix28) as usize)] = __v29; }
                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                            { let __ix30 = self.pool_ptr; let __v31 = ((self.str_pool[crate::ix::U((j) as usize)] % 1024i32)).wrapping_add(56320i32); self.str_pool[crate::ix::U((__ix30) as usize)] = __v31; }
                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                        }
                    } else {
                        {
                            { let __ix32 = self.pool_ptr; let __v33 = self.str_pool[crate::ix::U((j) as usize)]; self.str_pool[crate::ix::U((__ix32) as usize)] = __v33; }
                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                        }
                    }
                }
                j = (j).wrapping_add(1i32);
                i = (i).wrapping_sub(1i32);
            }
        }
    }

    /// The following subroutine compares string `s` with another string of the
    /// same length that appears in `buffer` starting at position `k`;
    /// the result is `true` if and only if the strings are equal.
    /// Empirical tests indicate that `str_eq_buf` is used in such a way that
    /// it tends to return `true` about 80 percent of the time.
    // §45
    pub fn str_eq_buf(&mut self, mut s: str_number, mut k: i32) -> bool {
        let mut str_eq_buf: bool = false;
        let mut j: pool_pointer = 0; // §45
        let mut result: bool = false; // §45
        'l_not_found_f: {
            j = self.str_start[crate::ix::U(((s).wrapping_sub(65536i32)) as usize)];
            while (j < self.str_start[crate::ix::U((((s).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)]) {
                {
                    if (self.buffer[crate::ix::U((k) as usize)] >= 65536i32) {
                        if (self.str_pool[crate::ix::U((j) as usize)] != (55296i32).wrapping_add(((self.buffer[crate::ix::U((k) as usize)]).wrapping_sub(65536i32) / 1024i32))) {
                            {
                                result = false;
                                break 'l_not_found_f;
                            }
                        } else {
                            if (self.str_pool[crate::ix::U(((j).wrapping_add(1i32)) as usize)] != (56320i32).wrapping_add(((self.buffer[crate::ix::U((k) as usize)]).wrapping_sub(65536i32) % 1024i32))) {
                                {
                                    result = false;
                                    break 'l_not_found_f;
                                }
                            } else {
                                j = (j).wrapping_add(1i32);
                            }
                        }
                    } else {
                        if (self.str_pool[crate::ix::U((j) as usize)] != self.buffer[crate::ix::U((k) as usize)]) {
                            {
                                result = false;
                                break 'l_not_found_f;
                            }
                        }
                    }
                    j = (j).wrapping_add(1i32);
                    k = (k).wrapping_add(1i32);
                }
            }
            result = true;
        }
        str_eq_buf = result;
        str_eq_buf
    }

    /// Here is a similar routine, but it compares two strings in the string pool,
    /// and it does not assume that they have the same length.
    // §46
    pub fn str_eq_str(&mut self, mut s: str_number, mut t: str_number) -> bool {
        let mut str_eq_str: bool = false;
        let mut j: pool_pointer = 0; // §46
        let mut k: pool_pointer = 0; // §46
        let mut result: bool = false; // §46
        'l_not_found_f: {
            result = false;
            if (self.length(s) != self.length(t)) {
                break 'l_not_found_f;
            }
            if (self.length(s) == 1i32) {
                {
                    if (s < 65536i32) {
                        {
                            if (t < 65536i32) {
                                {
                                    if (s != t) {
                                        break 'l_not_found_f;
                                    }
                                }
                            } else {
                                {
                                    if (s != self.str_pool[crate::ix::U((self.str_start[crate::ix::U(((t).wrapping_sub(65536i32)) as usize)]) as usize)]) {
                                        break 'l_not_found_f;
                                    }
                                }
                            }
                        }
                    } else {
                        {
                            if (t < 65536i32) {
                                {
                                    if (self.str_pool[crate::ix::U((self.str_start[crate::ix::U(((s).wrapping_sub(65536i32)) as usize)]) as usize)] != t) {
                                        break 'l_not_found_f;
                                    }
                                }
                            } else {
                                {
                                    if (self.str_pool[crate::ix::U((self.str_start[crate::ix::U(((s).wrapping_sub(65536i32)) as usize)]) as usize)] != self.str_pool[crate::ix::U((self.str_start[crate::ix::U(((t).wrapping_sub(65536i32)) as usize)]) as usize)]) {
                                        break 'l_not_found_f;
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                {
                    j = self.str_start[crate::ix::U(((s).wrapping_sub(65536i32)) as usize)];
                    k = self.str_start[crate::ix::U(((t).wrapping_sub(65536i32)) as usize)];
                    while (j < self.str_start[crate::ix::U((((s).wrapping_add(1i32)).wrapping_sub(65536i32)) as usize)]) {
                        {
                            if (self.str_pool[crate::ix::U((j) as usize)] != self.str_pool[crate::ix::U((k) as usize)]) {
                                break 'l_not_found_f;
                            }
                            j = (j).wrapping_add(1i32);
                            k = (k).wrapping_add(1i32);
                        }
                    }
                }
            }
            result = true;
        }
        str_eq_str = result;
        str_eq_str
    }

    /// tex.ch's string recycling routines (its part \.{[54/web2c-string]}).
    /// \TeX{} uses 2 upto 4 {\it new\/} strings when scanning a filename in an
    /// \.{\\input}, \.{\\openin}, or \.{\\openout} operation.  These strings are
    /// normally lost because the reference to them are not saved after finishing
    /// the operation.  `search_string` searches through the string pool for the
    /// given string and returns either 0 or the found string number.
    /// @<Declare additional routines for string recycling
    // §1697
    pub fn search_string(&mut self, mut search: str_number) -> str_number {
        let mut search_string: str_number = 0;
        let mut result: str_number = 0; // §1697
        let mut s: str_number = 0; // §1697
        let mut len: i32 = 0; // §1697
        'l_found_f: {
            result = 0i32;
            len = self.length(search);
            if (len == 0i32) {
                {
                    result = 65626i32;
                    break 'l_found_f;
                }
            } else {
                {
                    s = (search).wrapping_sub(1i32);
                    while (s > 65535i32) {
                        {
                            if (self.length(s) == len) {
                                if self.str_eq_str(s, search) {
                                    {
                                        result = s;
                                        break 'l_found_f;
                                    }
                                }
                            }
                            s = (s).wrapping_sub(1i32);
                        }
                    }
                }
            }
        }
        search_string = result;
        search_string
    }

    /// The following routine is a variant of `make_string`.  It searches
    /// the whole string pool for a string equal to the string currently built
    /// and returns a found string.  Otherwise a new string is created and
    /// returned.  Be cautious, you can not apply `flush_string` to a replaced
    /// string!
    /// @<Declare additional routines for string recycling
    // §1698
    pub fn slow_make_string(&mut self) -> str_number {
        let mut slow_make_string: str_number = 0;
        let mut s: str_number = 0; // §1698
        let mut t: str_number = 0; // §1698
        'l_exit_f: {
            t = self.make_string();
            s = self.search_string(t);
            if (s > 0i32) {
                {
                    {
                        self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                        self.pool_ptr = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
                    }
                    slow_make_string = s;
                    break 'l_exit_f;
                }
            }
            slow_make_string = t;
        }
        slow_make_string
    }

    /// The initial values of `str_pool`, `str_start`, `pool_ptr`,
    /// and `str_ptr` are computed by the \.{INITEX} program, based in part
    /// on the information that \.{WEB} has output while processing \TeX.
    // §47
    pub fn get_strings_started(&mut self) -> bool {
        let mut get_strings_started: bool = false;
        let mut g: str_number = 0; // §47
        'l_exit_f: {
            self.pool_ptr = 0i32;
            self.str_ptr = 0i32;
            self.str_start[crate::ix::U((0i32) as usize)] = 0i32;
            // §48
            {
                self.str_ptr = too_big_char;
                { let __ix34 = (self.str_ptr).wrapping_sub(65536i32); let __v35 = self.pool_ptr; self.str_start[crate::ix::U((__ix34) as usize)] = __v35; }
            }
            // §51
            g = self.load_pool_strings((pool_size).wrapping_sub(string_vacancies));
            if (g == 0i32) {
                {
                    {
                        crate::system::wr_str(&mut self.term_out, "! You have to increase POOLSIZE.");
                        crate::system::wr_ln(&mut self.term_out);
                    }
                    get_strings_started = false;
                    break 'l_exit_f;
                }
            }
            get_strings_started = true;
        }
        // §47
        get_strings_started
    }

    /// Here is a trivial procedure to print two digits; it is usually called with
    /// a parameter in the range `0<=n<=99`.
    // §70
    pub fn print_two(&mut self, mut n: i32) {
        n = ((n).wrapping_abs() % 100i32);
        self.print_char((48i32).wrapping_add((n / 10i32)));
        self.print_char((48i32).wrapping_add((n % 10i32)));
    }

    /// Hexadecimal printing of nonnegative integers is accomplished by `print_hex`.
    // §71
    pub fn print_hex(&mut self, mut n: i32) {
        let mut k: i32 = 0; // §71
        k = 0i32;
        self.print_char(34i32);
        loop {
            self.dig[crate::ix::U((k) as usize)] = (n % 16i32);
            n = (n / 16i32);
            k = (k).wrapping_add(1i32);
            if (n == 0i32) { break; }
        }
        self.print_the_digs(k);
    }

    /// Roman numerals are produced by the `print_roman_int` routine.  Readers
    /// who like puzzles might enjoy trying to figure out how this tricky code
    /// works; therefore no explanation will be given. Notice that 1990 yields
    /// \.{mcmxc}, not \.{mxm}.
    // §73
    pub fn print_roman_int(&mut self, mut n: i32) {
        let mut j: pool_pointer = 0; // §73
        let mut k: pool_pointer = 0; // §73
        let mut u: nonnegative_integer = 0; // §73
        let mut v: nonnegative_integer = 0; // §73
        'l_exit_f: {
            j = self.str_start[crate::ix::U(((65542i32).wrapping_sub(65536i32)) as usize)];
            v = 1000i32;
            while true {
                {
                    while (n >= v) {
                        {
                            self.print_char(self.str_pool[crate::ix::U((j) as usize)]);
                            n = (n).wrapping_sub(v);
                        }
                    }
                    if (n <= 0i32) {
                        break 'l_exit_f;
                    }
                    k = (j).wrapping_add(2i32);
                    u = (v / (self.str_pool[crate::ix::U(((k).wrapping_sub(1i32)) as usize)]).wrapping_sub(48i32));
                    if (self.str_pool[crate::ix::U(((k).wrapping_sub(1i32)) as usize)] == 50i32) {
                        {
                            k = (k).wrapping_add(2i32);
                            u = (u / (self.str_pool[crate::ix::U(((k).wrapping_sub(1i32)) as usize)]).wrapping_sub(48i32));
                        }
                    }
                    if ((n).wrapping_add(u) >= v) {
                        {
                            self.print_char(self.str_pool[crate::ix::U((k) as usize)]);
                            n = (n).wrapping_add(u);
                        }
                    } else {
                        {
                            j = (j).wrapping_add(2i32);
                            v = (v / (self.str_pool[crate::ix::U(((j).wrapping_sub(1i32)) as usize)]).wrapping_sub(48i32));
                        }
                    }
                }
            }
        }
    }

    /// The `print` subroutine will not print a string that is still being
    /// created. The following procedure will.
    // §74
    pub fn print_current_string(&mut self) {
        let mut j: pool_pointer = 0; // §74
        j = self.str_start[crate::ix::U(((self.str_ptr).wrapping_sub(65536i32)) as usize)];
        while (j < self.pool_ptr) {
            {
                self.print_char(self.str_pool[crate::ix::U((j) as usize)]);
                j = (j).wrapping_add(1i32);
            }
        }
    }

    /// Here is a procedure that asks the user to type a line of input,
    /// assuming that the `selector` setting is either `term_only` or `term_and_log`.
    /// The input is placed into locations `first` through `last-1` of the
    /// `buffer` array, and echoed on the transcript file if appropriate.
    /// This procedure is never called when `interaction<scroll_mode`.
    // §75
    pub fn term_input(&mut self) {
        let mut k: i32 = 0; // §75
        crate::system::break_out(&mut self.term_out);
        if (!{ let mut __f0 = ::core::mem::take(&mut self.term_in); let __r = self.input_ln(&mut __f0, true); self.term_in = __f0; __r }) {
            {
                self.cur_input.limit_field = 0i32;
                self.fatal_error(65543i32);
            }
        }
        self.term_offset = 0i32;
        self.selector = (self.selector).wrapping_sub(1i32);
        if (self.last != self.first) {
            {
                let __for_end_3 = (self.last).wrapping_sub(1i32);
                k = self.first;
                while k <= __for_end_3 {
                    self.print(self.buffer[crate::ix::U((k) as usize)]);
                    k = k.wrapping_add(1);
                }
            }
        }
        self.print_ln();
        self.selector = (self.selector).wrapping_add(1i32);
    }

    /// A dozen or so error messages end with a parenthesized integer, so we
    /// save a teeny bit of program space by declaring the following procedure:
    // §95
    pub fn int_error(&mut self, mut n: i32) {
        self.print(65566i32);
        self.print_int(n);
        self.print_char(41i32);
        self.error();
    }

    /// In anomalous cases, the print selector might be in an unknown state;
    /// the following subroutine is called to fix things just enough to keep
    /// running a bit longer.
    // §96
    pub fn normalize_selector(&mut self) {
        if self.log_opened {
            self.selector = term_and_log;
        } else {
            self.selector = term_only;
        }
        if (self.job_name == 0i32) {
            self.open_log_file();
        }
        if (self.interaction == batch_mode) {
            self.selector = (self.selector).wrapping_sub(1i32);
        }
    }

    /// When an interrupt has been detected, the program goes into its
    /// highest interaction level and lets the user have nearly the full flexibility of
    /// the `error` routine.  \TeX\ checks for interrupts only at times when it is
    /// safe to do this.
    // §102
    pub fn pause_for_instructions(&mut self) {
        if self.OK_to_interrupt {
            {
                self.interaction = error_stop_mode;
                if ((self.selector == log_only) || (self.selector == no_print)) {
                    self.selector = (self.selector).wrapping_add(1i32);
                }
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(65576i32);
                }
                {
                    self.help_ptr = 3i32;
                    self.help_line[crate::ix::U((2i32) as usize)] = 65577i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 65578i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 65579i32;
                }
                self.deletions_allowed = false;
                self.error();
                self.deletions_allowed = true;
                self.interrupt = 0i32;
            }
        }
    }

    /// Here is a routine that calculates half of an integer, using an
    /// unambiguous convention with respect to signed odd numbers.
    // §104
    pub fn half(&mut self, mut x: i32) -> i32 {
        let mut half: i32 = 0;
        if (((x) % 2) != 0) {
            half = ((x).wrapping_add(1i32) / 2i32);
        } else {
            half = (x / 2i32);
        }
        half
    }

    /// The following function is used to create a scaled integer from a given decimal
    /// fraction $(.d_0d_1\ldots d_{k-1})$, where `0<=k<=17`. The digit $d_i$ is
    /// given in `dig[i]`, and the calculation produces a correctly rounded result.
    // §106
    pub fn round_decimals(&mut self, mut k: small_number) -> scaled {
        let mut round_decimals: scaled = 0;
        let mut a: i32 = 0; // §106
        a = 0i32;
        while (k > 0i32) {
            {
                k = (k).wrapping_sub(1i32);
                a = ((a).wrapping_add((self.dig[crate::ix::U((k) as usize)]).wrapping_mul(two)) / 10i32);
            }
        }
        round_decimals = ((a).wrapping_add(1i32) / 2i32);
        round_decimals
    }

    /// Conversely, here is a procedure analogous to `print_int`. If the output
    /// of this procedure is subsequently read by \TeX\ and converted by the
    /// `round_decimals` routine above, it turns out that the original value will
    /// be reproduced exactly; the ``simplest'' such decimal number is output,
    /// but there is always at least one digit following the decimal point.
    /// The invariant relation in the \&{repeat} loop is that a sequence of
    /// decimal digits yet to be printed will yield the original number if and only if
    /// they form a fraction~$f$ in the range $s-\delta\L10\cdot2^{16}f<s$.
    /// We can stop if and only if $f=0$ satisfies this condition; the loop will
    /// terminate before $s$ can possibly become zero.
    // §107
    pub fn print_scaled(&mut self, mut s: scaled) {
        let mut delta: scaled = 0; // §107
        if (s < 0i32) {
            {
                self.print_char(45i32);
                s = (s).wrapping_neg();
            }
        }
        self.print_int((s / unity));
        self.print_char(46i32);
        s = ((10i32).wrapping_mul((s % unity))).wrapping_add(5i32);
        delta = 10i32;
        loop {
            if (delta > unity) {
                s = (s).wrapping_sub(17232i32);
            }
            self.print_char((48i32).wrapping_add((s / unity)));
            s = (10i32).wrapping_mul((s % unity));
            delta = (delta).wrapping_mul(10i32);
            if (s <= delta) { break; }
        }
    }

    /// The first arithmetical subroutine we need computes $nx+y$, where `x`
    /// and~`y` are `scaled` and `n` is an integer. We will also use it to
    /// multiply integers.
    // §109
    pub fn mult_and_add(&mut self, mut n: i32, mut x: scaled, mut y: scaled, mut max_answer: scaled) -> scaled {
        let mut mult_and_add: scaled = 0;
        if (n < 0i32) {
            {
                x = (x).wrapping_neg();
                n = (n).wrapping_neg();
            }
        }
        if (n == 0i32) {
            mult_and_add = y;
        } else {
            if ((x <= ((max_answer).wrapping_sub(y) / n)) && ((x).wrapping_neg() <= ((max_answer).wrapping_add(y) / n))) {
                mult_and_add = ((n).wrapping_mul(x)).wrapping_add(y);
            } else {
                {
                    self.arith_error = true;
                    mult_and_add = 0i32;
                }
            }
        }
        mult_and_add
    }

    /// We also need to divide scaled dimensions by integers.
    // §110
    pub fn x_over_n(&mut self, mut x: scaled, mut n: i32) -> scaled {
        let mut x_over_n: scaled = 0;
        let mut negative: bool = false; // §110
        negative = false;
        if (n == 0i32) {
            {
                self.arith_error = true;
                x_over_n = 0i32;
                self.remainder = x;
            }
        } else {
            {
                if (n < 0i32) {
                    {
                        x = (x).wrapping_neg();
                        n = (n).wrapping_neg();
                        negative = true;
                    }
                }
                if (x >= 0i32) {
                    {
                        x_over_n = (x / n);
                        self.remainder = (x % n);
                    }
                } else {
                    {
                        x_over_n = (((x).wrapping_neg() / n)).wrapping_neg();
                        self.remainder = (((x).wrapping_neg() % n)).wrapping_neg();
                    }
                }
            }
        }
        if negative {
            self.remainder = (self.remainder).wrapping_neg();
        }
        x_over_n
    }

    /// Then comes the multiplication of a scaled number by a fraction `n/d`,
    /// where `n` and `d` are nonnegative integers `<=@t$2^{16}$@>` and `d` is
    /// positive. It would be too dangerous to multiply by~`n` and then divide
    /// by~`d`, in separate operations, since overflow might well occur; and it
    /// would be too inaccurate to divide by `d` and then multiply by `n`. Hence
    /// this subroutine simulates 1.5-precision arithmetic.
    // §111
    pub fn xn_over_d(&mut self, mut x: scaled, mut n: i32, mut d: i32) -> scaled {
        let mut xn_over_d: scaled = 0;
        let mut positive: bool = false; // §111
        let mut t: nonnegative_integer = 0; // §111
        let mut u: nonnegative_integer = 0; // §111
        let mut v: nonnegative_integer = 0; // §111
        if (x >= 0i32) {
            positive = true;
        } else {
            {
                x = (x).wrapping_neg();
                positive = false;
            }
        }
        t = ((x % 32768i32)).wrapping_mul(n);
        u = (((x / 32768i32)).wrapping_mul(n)).wrapping_add((t / 32768i32));
        v = (((u % d)).wrapping_mul(32768i32)).wrapping_add((t % 32768i32));
        if ((u / d) >= 32768i32) {
            self.arith_error = true;
        } else {
            u = ((32768i32).wrapping_mul((u / d))).wrapping_add((v / d));
        }
        if positive {
            {
                xn_over_d = u;
                self.remainder = (v % d);
            }
        } else {
            {
                xn_over_d = (u).wrapping_neg();
                self.remainder = ((v % d)).wrapping_neg();
            }
        }
        xn_over_d
    }

    /// The next subroutine is used to compute the ``badness'' of glue, when a
    /// total~`t` is supposed to be made from amounts that sum to~`s`.  According
    /// to {\sl The \TeX book}, the badness of this situation is $100(t/s)^3$;
    /// however, badness is simply a heuristic, so we need not squeeze out the
    /// last drop of accuracy when computing it. All we really want is an
    /// approximation that has similar properties.
    /// The actual method used to compute the badness is easier to read from the
    /// program than to describe in words. It produces an integer value that is a
    /// reasonably close approximation to $100(t/s)^3$, and all implementations
    /// of \TeX\ should use precisely this method. Any badness of $2^{13}$ or more is
    /// treated as infinitely bad, and represented by 10000.
    /// It is not difficult to prove that $$\hbox{`badness(t+1,s)>=badness(t,s)
    /// >=badness(t,s+1)`}.$$ The badness function defined here is capable of
    /// computing at most 1095 distinct values, but that is plenty.
    /// ...
    // §112
    pub fn badness(&mut self, mut t: scaled, mut s: scaled) -> halfword {
        let mut badness: halfword = 0;
        let mut r: i32 = 0; // §112
        if (t == 0i32) {
            badness = 0i32;
        } else {
            if (s <= 0i32) {
                badness = inf_bad;
            } else {
                {
                    if (t <= 7230584i32) {
                        r = ((t).wrapping_mul(297i32) / s);
                    } else {
                        if (s >= 1663497i32) {
                            r = (t / (s / 297i32));
                        } else {
                            r = t;
                        }
                    }
                    if (r > 1290i32) {
                        badness = inf_bad;
                    } else {
                        badness = ((((r).wrapping_mul(r)).wrapping_mul(r)).wrapping_add(131072i32) / 262144i32);
                    }
                }
            }
        }
        badness
    }

    /// The `make_frac` routine produces the `fraction` equivalent of
    /// `p/q`, given integers `p` and~`q`; it computes the integer
    /// $f=\lfloor2^{28}p/q+{1\over2}\rfloor$, when $p$ and $q$ are
    /// positive. If `p` and `q` are both of the same scaled type `t`,
    /// the ``type relation'' `make_frac(t,t)=fraction` is valid;
    /// and it's also possible to use the subroutine ``backwards,'' using
    /// the relation `make_frac(t,fraction)=t` between scaled types.
    /// If the result would have magnitude $2^{31}$ or more, `make_frac`
    /// sets `arith_error:=true`. Most of \MP's internal computations have
    /// been designed to avoid this sort of error.
    /// If this subroutine were programmed in assembly language on a typical
    /// machine, we could simply compute `(@t$2^{28}$@>*p)div q`, since a
    /// double-precision product can often be input to a fixed-point division
    /// instruction. But when we are restricted to \PASCAL\ arithmetic it
    /// ...
    // §116
    pub fn make_frac(&mut self, mut p: i32, mut q: i32) -> i32 {
        let mut make_frac: i32 = 0;
        let mut f: i32 = 0; // §116
        let mut n: i32 = 0; // §116
        let mut negative: bool = false; // §116
        let mut be_careful: i32 = 0; // §116
        if (p >= 0i32) {
            negative = false;
        } else {
            {
                p = (p).wrapping_neg();
                negative = true;
            }
        }
        if (q <= 0i32) {
            {
                q = (q).wrapping_neg();
                negative = (!negative);
            }
        }
        n = (p / q);
        p = (p % q);
        if (n >= 8i32) {
            {
                self.arith_error = true;
                if negative {
                    make_frac = (2147483647i32).wrapping_neg();
                } else {
                    make_frac = el_gordo;
                }
            }
        } else {
            {
                n = ((n).wrapping_sub(1i32)).wrapping_mul(fraction_one);
                // §117
                f = 1i32;
                loop {
                    be_careful = (p).wrapping_sub(q);
                    p = (be_careful).wrapping_add(p);
                    if (p >= 0i32) {
                        f = ((f).wrapping_add(f)).wrapping_add(1i32);
                    } else {
                        {
                            f = (f).wrapping_add(f);
                            p = (p).wrapping_add(q);
                        }
                    }
                    if (f >= fraction_one) { break; }
                }
                be_careful = (p).wrapping_sub(q);
                if ((be_careful).wrapping_add(p) >= 0i32) {
                    f = (f).wrapping_add(1i32);
                }
                // §116
                if negative {
                    make_frac = ((f).wrapping_add(n)).wrapping_neg();
                } else {
                    make_frac = (f).wrapping_add(n);
                }
            }
        }
        make_frac
    }

    // §118
    pub fn take_frac(&mut self, mut q: i32, mut f: i32) -> i32 {
        let mut take_frac: i32 = 0;
        let mut p: i32 = 0; // §118
        let mut negative: bool = false; // §118
        let mut n: i32 = 0; // §118
        let mut be_careful: i32 = 0; // §118
        // §119
        if (f >= 0i32) {
            negative = false;
        } else {
            {
                f = (f).wrapping_neg();
                negative = true;
            }
        }
        if (q < 0i32) {
            {
                q = (q).wrapping_neg();
                negative = (!negative);
            }
        }
        // §118
        if (f < fraction_one) {
            n = 0i32;
        } else {
            {
                n = (f / fraction_one);
                f = (f % fraction_one);
                if (q <= (el_gordo / n)) {
                    n = (n).wrapping_mul(q);
                } else {
                    {
                        self.arith_error = true;
                        n = el_gordo;
                    }
                }
            }
        }
        f = (f).wrapping_add(268435456i32);
        // §120
        p = fraction_half;
        if (q < fraction_four) {
            loop {
                if (((f) % 2) != 0) {
                    p = ((p).wrapping_add(q) / 2i32);
                } else {
                    p = (p / 2i32);
                }
                f = (f / 2i32);
                if (f == 1i32) { break; }
            }
        } else {
            loop {
                if (((f) % 2) != 0) {
                    p = (p).wrapping_add(((q).wrapping_sub(p) / 2i32));
                } else {
                    p = (p / 2i32);
                }
                f = (f / 2i32);
                if (f == 1i32) { break; }
            }
        }
        // §118
        be_careful = (n).wrapping_sub(2147483647i32);
        if ((be_careful).wrapping_add(p) > 0i32) {
            {
                self.arith_error = true;
                n = (el_gordo).wrapping_sub(p);
            }
        }
        if negative {
            take_frac = ((n).wrapping_add(p)).wrapping_neg();
        } else {
            take_frac = (n).wrapping_add(p);
        }
        take_frac
    }

    // §123
    pub fn m_log(&mut self, mut x: i32) -> i32 {
        let mut m_log: i32 = 0;
        let mut y: i32 = 0; // §123
        let mut z: i32 = 0; // §123
        let mut k: i32 = 0; // §123
        if (x <= 0i32) {
            // §125
            {
                {
                    if (self.interaction == error_stop_mode) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(65544i32);
                    }
                    self.print(65580i32);
                }
                self.print_scaled(x);
                self.print(65581i32);
                {
                    self.help_ptr = 2i32;
                    self.help_line[crate::ix::U((1i32) as usize)] = 65582i32;
                    self.help_line[crate::ix::U((0i32) as usize)] = 65583i32;
                }
                self.error();
                m_log = 0i32;
            }
        } else {
            // §123
            {
                y = 1302456860i32;
                z = 6581195i32;
                while (x < fraction_four) {
                    {
                        x = (x).wrapping_add(x);
                        y = (y).wrapping_sub(93032639i32);
                        z = (z).wrapping_sub(48782i32);
                    }
                }
                y = (y).wrapping_add((z / unity));
                k = 2i32;
                while (x > 1073741828i32) {
                    // §124
                    {
                        z = (((x).wrapping_sub(1i32) / self.two_to_the[crate::ix::U((k) as usize)])).wrapping_add(1i32);
                        while (x < (fraction_four).wrapping_add(z)) {
                            {
                                z = ((z).wrapping_add(1i32) / 2i32);
                                k = (k).wrapping_add(1i32);
                            }
                        }
                        y = (y).wrapping_add(self.spec_log[crate::ix::U(((k) - 1) as usize)]);
                        x = (x).wrapping_sub(z);
                    }
                }
                // §123
                m_log = (y / 8i32);
            }
        }
        m_log
    }

    /// The following somewhat different subroutine tests rigorously if $ab$ is
    /// greater than, equal to, or less than~$cd$,
    /// given integers $(a,b,c,d)$. In most cases a quick decision is reached.
    /// The result is $+1$, 0, or~$-1$ in the three respective cases.
    // §126
    pub fn ab_vs_cd(&mut self, mut a: i32, mut b: i32, mut c: i32, mut d: i32) -> i32 {
        let mut ab_vs_cd: i32 = 0;
        let mut q: i32 = 0; // §126
        let mut r: i32 = 0; // §126
        'l_exit_f: {
            // §127
            if (a < 0i32) {
                {
                    a = (a).wrapping_neg();
                    b = (b).wrapping_neg();
                }
            }
            if (c < 0i32) {
                {
                    c = (c).wrapping_neg();
                    d = (d).wrapping_neg();
                }
            }
            if (d <= 0i32) {
                {
                    if (b >= 0i32) {
                        if (((a == 0i32) || (b == 0i32)) && ((c == 0i32) || (d == 0i32))) {
                            {
                                ab_vs_cd = 0i32;
                                break 'l_exit_f;
                            }
                        } else {
                            {
                                ab_vs_cd = 1i32;
                                break 'l_exit_f;
                            }
                        }
                    }
                    if (d == 0i32) {
                        if (a == 0i32) {
                            {
                                ab_vs_cd = 0i32;
                                break 'l_exit_f;
                            }
                        } else {
                            {
                                ab_vs_cd = (1i32).wrapping_neg();
                                break 'l_exit_f;
                            }
                        }
                    }
                    q = a;
                    a = c;
                    c = q;
                    q = (b).wrapping_neg();
                    b = (d).wrapping_neg();
                    d = q;
                }
            } else {
                if (b <= 0i32) {
                    {
                        if (b < 0i32) {
                            if (a > 0i32) {
                                {
                                    ab_vs_cd = (1i32).wrapping_neg();
                                    break 'l_exit_f;
                                }
                            }
                        }
                        if (c == 0i32) {
                            {
                                ab_vs_cd = 0i32;
                                break 'l_exit_f;
                            }
                        } else {
                            {
                                ab_vs_cd = (1i32).wrapping_neg();
                                break 'l_exit_f;
                            }
                        }
                    }
                }
            }
            // §126
            while true {
                {
                    q = (a / d);
                    r = (c / b);
                    if (q != r) {
                        if (q > r) {
                            {
                                ab_vs_cd = 1i32;
                                break 'l_exit_f;
                            }
                        } else {
                            {
                                ab_vs_cd = (1i32).wrapping_neg();
                                break 'l_exit_f;
                            }
                        }
                    }
                    q = (a % d);
                    r = (c % b);
                    if (r == 0i32) {
                        if (q == 0i32) {
                            {
                                ab_vs_cd = 0i32;
                                break 'l_exit_f;
                            }
                        } else {
                            {
                                ab_vs_cd = 1i32;
                                break 'l_exit_f;
                            }
                        }
                    }
                    if (q == 0i32) {
                        {
                            ab_vs_cd = (1i32).wrapping_neg();
                            break 'l_exit_f;
                        }
                    }
                    a = b;
                    b = q;
                    c = d;
                    d = r;
                }
            }
        }
        ab_vs_cd
    }

    /// To consume a random integer, the program below will say ``next_random`'
    /// and then it will fetch `randoms[j_random]`.
    // §128
    pub fn new_randoms(&mut self) {
        let mut k: i32 = 0; // §128
        let mut x: i32 = 0; // §128
        {
            let __for_end_2 = 23i32;
            k = 0i32;
            while k <= __for_end_2 {
                {
                    x = (self.randoms[crate::ix::U((k) as usize)]).wrapping_sub(self.randoms[crate::ix::U(((k).wrapping_add(31i32)) as usize)]);
                    if (x < 0i32) {
                        x = (x).wrapping_add(268435456i32);
                    }
                    self.randoms[crate::ix::U((k) as usize)] = x;
                }
                k = k.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 54i32;
            k = 24i32;
            while k <= __for_end_2 {
                {
                    x = (self.randoms[crate::ix::U((k) as usize)]).wrapping_sub(self.randoms[crate::ix::U(((k).wrapping_sub(24i32)) as usize)]);
                    if (x < 0i32) {
                        x = (x).wrapping_add(268435456i32);
                    }
                    self.randoms[crate::ix::U((k) as usize)] = x;
                }
                k = k.wrapping_add(1);
            }
        }
        self.j_random = 54i32;
    }

    /// To initialize the `randoms` table, we call the following routine.
    // §129
    pub fn init_randoms(&mut self, mut seed: i32) {
        let mut j: i32 = 0; // §129
        let mut jj: i32 = 0; // §129
        let mut k: i32 = 0; // §129
        let mut i: i32 = 0; // §129
        j = (seed).wrapping_abs();
        while (j >= fraction_one) {
            j = (j / 2i32);
        }
        k = 1i32;
        {
            let __for_end_2 = 54i32;
            i = 0i32;
            while i <= __for_end_2 {
                {
                    jj = k;
                    k = (j).wrapping_sub(k);
                    j = jj;
                    if (k < 0i32) {
                        k = (k).wrapping_add(268435456i32);
                    }
                    self.randoms[crate::ix::U((((i).wrapping_mul(21i32) % 55i32)) as usize)] = j;
                }
                i = i.wrapping_add(1);
            }
        }
        self.new_randoms();
        self.new_randoms();
        self.new_randoms();
    }

    /// To produce a uniform random number in the range `0<=u<x` or `0>=u>x`
    /// or `0=u=x`, given a `scaled` value~`x`, we proceed as shown here.
    /// Note that the call of `take_frac` will produce the values 0 and~`x`
    /// with about half the probability that it will produce any other particular
    /// values between 0 and~`x`, because it rounds its answers.
    // §130
    pub fn unif_rand(&mut self, mut x: i32) -> i32 {
        let mut unif_rand: i32 = 0;
        let mut y: i32 = 0; // §130
        if (self.j_random == 0i32) {
            self.new_randoms();
        } else {
            self.j_random = (self.j_random).wrapping_sub(1i32);
        }
        y = self.take_frac((x).wrapping_abs(), self.randoms[crate::ix::U((self.j_random) as usize)]);
        if (y == (x).wrapping_abs()) {
            unif_rand = 0i32;
        } else {
            if (x > 0i32) {
                unif_rand = y;
            } else {
                unif_rand = (y).wrapping_neg();
            }
        }
        unif_rand
    }

    /// Finally, a normal deviate with mean zero and unit standard deviation
    /// can readily be obtained with the ratio method (Algorithm 3.4.1R in
    /// {\sl The Art of Computer Programming\/}).
    // §131
    pub fn norm_rand(&mut self) -> i32 {
        let mut norm_rand: i32 = 0;
        let mut x: i32 = 0; // §131
        let mut u: i32 = 0; // §131
        let mut l: i32 = 0; // §131
        loop {
            loop {
                if (self.j_random == 0i32) {
                    self.new_randoms();
                } else {
                    self.j_random = (self.j_random).wrapping_sub(1i32);
                }
                x = self.take_frac(112429i32, (self.randoms[crate::ix::U((self.j_random) as usize)]).wrapping_sub(134217728i32));
                if (self.j_random == 0i32) {
                    self.new_randoms();
                } else {
                    self.j_random = (self.j_random).wrapping_sub(1i32);
                }
                u = self.randoms[crate::ix::U((self.j_random) as usize)];
                if ((x).wrapping_abs() < u) { break; }
            }
            x = self.make_frac(x, u);
            l = (139548960i32).wrapping_sub(self.m_log(u));
            if (self.ab_vs_cd(1024i32, l, x, x) >= 0i32) { break; }
        }
        norm_rand = x;
        norm_rand
    }

    /// The procedure `show_token_list`, which prints a symbolic form of
    /// the token list that starts at a given node `p`, illustrates these
    /// conventions. The token list being displayed should not begin with a reference
    /// count. However, the procedure is intended to be robust, so that if the
    /// memory links are awry or if `p` is not really a pointer to a token list,
    /// nothing catastrophic will happen.
    /// An additional parameter `q` is also given; this parameter is either null
    /// or it points to a node in the token list where a certain magic computation
    /// takes place that will be explained later. (Basically, `q` is non-null when
    /// we are printing the two-line context information at the time of an error
    /// message; `q` marks the place corresponding to where the second line
    /// should begin.)
    /// For example, if `p` points to the node containing the first \.a in the
    /// token list above, then `show_token_list` will print the string
    /// ...
    // §322
    pub fn show_token_list(&mut self, mut p: i32, mut q: i32, mut l: i32) {
        let mut m: i32 = 0; // §322
        let mut c: i32 = 0; // §322
        let mut match_chr: i32 = 0; // §322
        let mut n: UTF16_code = 0; // §322
        'l_exit_f: {
            match_chr = 35i32;
            n = 48i32;
            self.tally = 0i32;
            while ((p != (268435455i32).wrapping_neg()) && (self.tally < l)) {
                {
                    if (p == q) {
                        // §350
                        {
                            self.first_count = self.tally;
                            self.trick_count = (((self.tally).wrapping_add(1i32)).wrapping_add(self.error_line)).wrapping_sub(self.half_error_line);
                            if (self.trick_count < self.error_line) {
                                self.trick_count = self.error_line;
                            }
                        }
                    }
                    // §323
                    if ((p < self.hi_mem_min) || (p > self.mem_end)) {
                        {
                            self.print_esc(65595i32);
                            break 'l_exit_f;
                        }
                    }
                    if (self.mem[crate::ix::U((p) as usize)].hh().lh() >= cs_token_flag) {
                        self.print_cs((self.mem[crate::ix::U((p) as usize)].hh().lh()).wrapping_sub(33554431i32));
                    } else {
                        {
                            m = (self.mem[crate::ix::U((p) as usize)].hh().lh() / max_char_val);
                            c = (self.mem[crate::ix::U((p) as usize)].hh().lh() % max_char_val);
                            if (self.mem[crate::ix::U((p) as usize)].hh().lh() < 0i32) {
                                self.print_esc(65874i32);
                            } else {
                                // §324
                                match m {
                                    left_brace | right_brace | math_shift | tab_mark | sup_mark | sub_mark | spacer | letter | other_char => {
                                        self.print_char(c);
                                    }
                                    mac_param => {
                                        {
                                            self.print_char(c);
                                            self.print_char(c);
                                        }
                                    }
                                    out_param => {
                                        {
                                            self.print_char(match_chr);
                                            if (c <= 9i32) {
                                                self.print_char((c).wrapping_add(48i32));
                                            } else {
                                                {
                                                    self.print_char(33i32);
                                                    break 'l_exit_f;
                                                }
                                            }
                                        }
                                    }
                                    match_ => {
                                        {
                                            match_chr = c;
                                            self.print_char(c);
                                            n = (n).wrapping_add(1i32);
                                            self.print_char(n);
                                            if (n > 57i32) {
                                                break 'l_exit_f;
                                            }
                                        }
                                    }
                                    end_match => {
                                        if (c == 0i32) {
                                            self.print(65875i32);
                                        }
                                    }
                                    _ => {
                                        self.print_esc(65874i32);
                                    }
                                }
                            }
                        }
                    }
                    // §322
                    p = self.mem[crate::ix::U((p) as usize)].hh().rh();
                }
            }
            if (p != (268435455i32).wrapping_neg()) {
                self.print_esc(65700i32);
            }
        }
    }

    /// Here is a procedure that uses `scanner_status` to print a warning message
    /// when a subfile has ended, and at certain other crucial times:
    /// @<Declare the procedure called `runaway`
    // §336
    pub fn runaway(&mut self) {
        let mut p: halfword = 0; // §336
        if (self.scanner_status > skipping) {
            {
                self.print_nl(65888i32);
                match self.scanner_status {
                    defining => {
                        {
                            self.print(65889i32);
                            p = self.def_ref;
                        }
                    }
                    matching => {
                        {
                            self.print(65890i32);
                            p = temp_head;
                        }
                    }
                    aligning => {
                        {
                            self.print(65891i32);
                            p = hold_head;
                        }
                    }
                    absorbing => {
                        {
                            self.print(65892i32);
                            p = self.def_ref;
                        }
                    }
                    _ => {}
                }
                self.print_char(63i32);
                self.print_ln();
                self.show_token_list(self.mem[crate::ix::U((p) as usize)].hh().rh(), (268435455i32).wrapping_neg(), (self.error_line).wrapping_sub(10i32));
            }
        }
    }

    /// The function `get_avail` returns a pointer to a new one-word node whose
    /// `link` field is null. However, \TeX\ will halt if there is no more room left.
    /// If the available-space list is empty, i.e., if `avail=null`,
    /// we try first to increase `mem_end`. If that cannot be done, i.e., if
    /// `mem_end=mem_max`, we try to decrease `hi_mem_min`. If that cannot be
    /// done, i.e., if `hi_mem_min=lo_mem_max+1`, we have to quit.
    // §142
    pub fn get_avail(&mut self) -> halfword {
        let mut get_avail: halfword = 0;
        let mut p: halfword = 0; // §142
        p = self.avail;
        if (p != (268435455i32).wrapping_neg()) {
            self.avail = self.mem[crate::ix::U((self.avail) as usize)].hh().rh();
        } else {
            if (self.mem_end < mem_max) {
                {
                    self.mem_end = (self.mem_end).wrapping_add(1i32);
                    p = self.mem_end;
                }
            } else {
                {
                    self.hi_mem_min = (self.hi_mem_min).wrapping_sub(1i32);
                    p = self.hi_mem_min;
                    if (self.hi_mem_min <= self.lo_mem_max) {
                        {
                            self.runaway();
                            self.overflow(65584i32, ((mem_max).wrapping_add(1i32)).wrapping_sub(mem_min));
                        }
                    }
                }
            }
        }
        self.mem[crate::ix::U((p) as usize)].set_hh_rh((268435455i32).wrapping_neg());
        self.dyn_used = (self.dyn_used).wrapping_add(1i32);
        get_avail = p;
        get_avail
    }

}
