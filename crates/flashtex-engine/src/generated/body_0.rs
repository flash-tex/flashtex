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
        let mut k: i32 = 0; // §181
        let mut z: hyph_pointer = 0; // §1104
        // §1869
        self.obj_tab.alloc_len(((inf_obj_tab_size) as usize) + 1);
        self.pdf_mem.alloc_len(((inf_pdf_mem_size) as usize) + 1);
        self.dest_names.alloc_len(((inf_dest_names_size) as usize) + 1);
        self.pdf_op_buf.alloc_len(((pdf_op_buf_size) as usize) + 1);
        self.pdf_os_buf.alloc_len(((inf_pdf_os_buf_size) as usize) + 1);
        self.pdf_os_objnum.alloc_len(((pdf_os_max_objs) as usize) + 1);
        self.pdf_os_objoff.alloc_len(((pdf_os_max_objs) as usize) + 1);
        self.pdf_char_used.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_size.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_num.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_map.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_type.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_attr.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_blink.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_elink.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_has_space_char.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_stretch.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_shrink.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_step.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_expand_ratio.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_auto_expand.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_lp_base.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_rp_base.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_ef_base.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_kn_bs_base.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_st_bs_base.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_sh_bs_base.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_kn_bc_base.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_kn_ac_base.alloc_len(((font_max) as usize) + 1);
        self.vf_packet_base.alloc_len(((font_max) as usize) + 1);
        self.vf_default_font.alloc_len(((font_max) as usize) + 1);
        self.vf_local_font_num.alloc_len(((font_max) as usize) + 1);
        self.vf_e_fnts.alloc_len(((font_max) as usize) + 1);
        self.vf_i_fnts.alloc_len(((font_max) as usize) + 1);
        self.pdf_font_nobuiltin_tounicode.alloc_len(((font_max) as usize) + 1);
        {
            let __for_end_2 = font_max;
            i = 0i32;
            while i <= __for_end_2 {
                {
                    {
                        let __for_end_5 = 31i32;
                        k = 0i32;
                        while k <= __for_end_5 {
                            self.pdf_char_used[(i) as usize][(k) as usize] = 0i32;
                            k = k.wrapping_add(1);
                        }
                    }
                    self.pdf_font_size[(i) as usize] = 0i32;
                    self.pdf_font_num[(i) as usize] = 0i32;
                    self.pdf_font_map[(i) as usize] = 0i32;
                    self.pdf_font_type[(i) as usize] = 0i32;
                    self.pdf_font_attr[(i) as usize] = 348i32;
                    self.pdf_font_blink[(i) as usize] = 0i32;
                    self.pdf_font_elink[(i) as usize] = 0i32;
                    { let __v0 = false; self.pdf_font_has_space_char[(i) as usize] = __v0; }
                    self.pdf_font_stretch[(i) as usize] = 0i32;
                    self.pdf_font_shrink[(i) as usize] = 0i32;
                    self.pdf_font_step[(i) as usize] = 0i32;
                    self.pdf_font_expand_ratio[(i) as usize] = 0i32;
                    { let __v1 = false; self.pdf_font_auto_expand[(i) as usize] = __v1; }
                    self.pdf_font_lp_base[(i) as usize] = 0i32;
                    self.pdf_font_rp_base[(i) as usize] = 0i32;
                    self.pdf_font_ef_base[(i) as usize] = 0i32;
                    self.pdf_font_kn_bs_base[(i) as usize] = 0i32;
                    self.pdf_font_st_bs_base[(i) as usize] = 0i32;
                    self.pdf_font_sh_bs_base[(i) as usize] = 0i32;
                    self.pdf_font_kn_bc_base[(i) as usize] = 0i32;
                    self.pdf_font_kn_ac_base[(i) as usize] = 0i32;
                    { let __v2 = false; self.pdf_font_nobuiltin_tounicode[(i) as usize] = __v2; }
                }
                i = i.wrapping_add(1);
            }
        }
        // §21
        self.xchr[(32i32) as usize] = b' ';
        self.xchr[(33i32) as usize] = b'!';
        self.xchr[(34i32) as usize] = b'"';
        self.xchr[(35i32) as usize] = b'#';
        self.xchr[(36i32) as usize] = b'$';
        self.xchr[(37i32) as usize] = b'%';
        self.xchr[(38i32) as usize] = b'&';
        self.xchr[(39i32) as usize] = b'\'';
        self.xchr[(40i32) as usize] = b'(';
        self.xchr[(41i32) as usize] = b')';
        self.xchr[(42i32) as usize] = b'*';
        self.xchr[(43i32) as usize] = b'+';
        self.xchr[(44i32) as usize] = b',';
        self.xchr[(45i32) as usize] = b'-';
        self.xchr[(46i32) as usize] = b'.';
        self.xchr[(47i32) as usize] = b'/';
        self.xchr[(48i32) as usize] = b'0';
        self.xchr[(49i32) as usize] = b'1';
        self.xchr[(50i32) as usize] = b'2';
        self.xchr[(51i32) as usize] = b'3';
        self.xchr[(52i32) as usize] = b'4';
        self.xchr[(53i32) as usize] = b'5';
        self.xchr[(54i32) as usize] = b'6';
        self.xchr[(55i32) as usize] = b'7';
        self.xchr[(56i32) as usize] = b'8';
        self.xchr[(57i32) as usize] = b'9';
        self.xchr[(58i32) as usize] = b':';
        self.xchr[(59i32) as usize] = b';';
        self.xchr[(60i32) as usize] = b'<';
        self.xchr[(61i32) as usize] = b'=';
        self.xchr[(62i32) as usize] = b'>';
        self.xchr[(63i32) as usize] = b'?';
        self.xchr[(64i32) as usize] = b'@';
        self.xchr[(65i32) as usize] = b'A';
        self.xchr[(66i32) as usize] = b'B';
        self.xchr[(67i32) as usize] = b'C';
        self.xchr[(68i32) as usize] = b'D';
        self.xchr[(69i32) as usize] = b'E';
        self.xchr[(70i32) as usize] = b'F';
        self.xchr[(71i32) as usize] = b'G';
        self.xchr[(72i32) as usize] = b'H';
        self.xchr[(73i32) as usize] = b'I';
        self.xchr[(74i32) as usize] = b'J';
        self.xchr[(75i32) as usize] = b'K';
        self.xchr[(76i32) as usize] = b'L';
        self.xchr[(77i32) as usize] = b'M';
        self.xchr[(78i32) as usize] = b'N';
        self.xchr[(79i32) as usize] = b'O';
        self.xchr[(80i32) as usize] = b'P';
        self.xchr[(81i32) as usize] = b'Q';
        self.xchr[(82i32) as usize] = b'R';
        self.xchr[(83i32) as usize] = b'S';
        self.xchr[(84i32) as usize] = b'T';
        self.xchr[(85i32) as usize] = b'U';
        self.xchr[(86i32) as usize] = b'V';
        self.xchr[(87i32) as usize] = b'W';
        self.xchr[(88i32) as usize] = b'X';
        self.xchr[(89i32) as usize] = b'Y';
        self.xchr[(90i32) as usize] = b'Z';
        self.xchr[(91i32) as usize] = b'[';
        self.xchr[(92i32) as usize] = b'\\';
        self.xchr[(93i32) as usize] = b']';
        self.xchr[(94i32) as usize] = b'^';
        self.xchr[(95i32) as usize] = b'_';
        self.xchr[(96i32) as usize] = b'`';
        self.xchr[(97i32) as usize] = b'a';
        self.xchr[(98i32) as usize] = b'b';
        self.xchr[(99i32) as usize] = b'c';
        self.xchr[(100i32) as usize] = b'd';
        self.xchr[(101i32) as usize] = b'e';
        self.xchr[(102i32) as usize] = b'f';
        self.xchr[(103i32) as usize] = b'g';
        self.xchr[(104i32) as usize] = b'h';
        self.xchr[(105i32) as usize] = b'i';
        self.xchr[(106i32) as usize] = b'j';
        self.xchr[(107i32) as usize] = b'k';
        self.xchr[(108i32) as usize] = b'l';
        self.xchr[(109i32) as usize] = b'm';
        self.xchr[(110i32) as usize] = b'n';
        self.xchr[(111i32) as usize] = b'o';
        self.xchr[(112i32) as usize] = b'p';
        self.xchr[(113i32) as usize] = b'q';
        self.xchr[(114i32) as usize] = b'r';
        self.xchr[(115i32) as usize] = b's';
        self.xchr[(116i32) as usize] = b't';
        self.xchr[(117i32) as usize] = b'u';
        self.xchr[(118i32) as usize] = b'v';
        self.xchr[(119i32) as usize] = b'w';
        self.xchr[(120i32) as usize] = b'x';
        self.xchr[(121i32) as usize] = b'y';
        self.xchr[(122i32) as usize] = b'z';
        self.xchr[(123i32) as usize] = b'{';
        self.xchr[(124i32) as usize] = b'|';
        self.xchr[(125i32) as usize] = b'}';
        self.xchr[(126i32) as usize] = b'~';
        // §23
        {
            let __for_end_2 = 31i32;
            i = 0i32;
            while i <= __for_end_2 {
                self.xchr[(i) as usize] = ((i) as u8);
                i = i.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 255i32;
            i = 127i32;
            while i <= __for_end_2 {
                self.xchr[(i) as usize] = ((i) as u8);
                i = i.wrapping_add(1);
            }
        }
        // §24
        {
            let __for_end_2 = 255i32;
            i = 0i32;
            while i <= __for_end_2 {
                self.xord[(((i) as u8)) as usize] = 127i32;
                i = i.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 255i32;
            i = 128i32;
            while i <= __for_end_2 {
                self.xord[(self.xchr[(i) as usize]) as usize] = i;
                i = i.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 126i32;
            i = 0i32;
            while i <= __for_end_2 {
                self.xord[(self.xchr[(i) as usize]) as usize] = i;
                i = i.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 255i32;
            i = 0i32;
            while i <= __for_end_2 {
                { let __v3 = (self.eight_bit_p || ((i >= 32i32) && (i <= 126i32))); self.xprn[(i) as usize] = __v3; }
                i = i.wrapping_add(1);
            }
        }
        if self.translate_filename_p {
            self.read_tcx_file();
        }
        // §74
        if (self.interaction_option == 4i32) {
            self.interaction = 3i32;
        } else {
            self.interaction = self.interaction_option;
        }
        // §77
        self.deletions_allowed = true;
        self.set_box_allowed = true;
        self.error_count = 0i32;
        // §80
        self.help_ptr = 0i32;
        self.use_err_help = false;
        // §97
        self.interrupt = 0i32;
        self.OK_to_interrupt = true;
        // §118
        self.two_to_the[(0i32) as usize] = 1i32;
        {
            let __for_end_2 = 30i32;
            k = 1i32;
            while k <= __for_end_2 {
                { let __v4 = (2i32).wrapping_mul(self.two_to_the[((k).wrapping_sub(1i32)) as usize]); self.two_to_the[(k) as usize] = __v4; }
                k = k.wrapping_add(1);
            }
        }
        self.spec_log[((1i32) - 1) as usize] = 93032640i32;
        self.spec_log[((2i32) - 1) as usize] = 38612034i32;
        self.spec_log[((3i32) - 1) as usize] = 17922280i32;
        self.spec_log[((4i32) - 1) as usize] = 8662214i32;
        self.spec_log[((5i32) - 1) as usize] = 4261238i32;
        self.spec_log[((6i32) - 1) as usize] = 2113709i32;
        self.spec_log[((7i32) - 1) as usize] = 1052693i32;
        self.spec_log[((8i32) - 1) as usize] = 525315i32;
        self.spec_log[((9i32) - 1) as usize] = 262400i32;
        self.spec_log[((10i32) - 1) as usize] = 131136i32;
        self.spec_log[((11i32) - 1) as usize] = 65552i32;
        self.spec_log[((12i32) - 1) as usize] = 32772i32;
        self.spec_log[((13i32) - 1) as usize] = 16385i32;
        {
            let __for_end_2 = 27i32;
            k = 14i32;
            while k <= __for_end_2 {
                { let __v5 = self.two_to_the[((27i32).wrapping_sub(k)) as usize]; self.spec_log[((k) - 1) as usize] = __v5; }
                k = k.wrapping_add(1);
            }
        }
        self.spec_log[((28i32) - 1) as usize] = 1i32;
        // §233
        self.nest_ptr = 0i32;
        self.max_nest_stack = 0i32;
        self.cur_list.mode_field = 1i32;
        self.cur_list.head_field = 4999998i32;
        self.cur_list.tail_field = 4999998i32;
        self.cur_list.eTeX_aux_field = 0i32;
        self.save_tail = 0i32;
        self.cur_list.aux_field.set_int((65536000i32).wrapping_neg());
        self.cur_list.ml_field = 0i32;
        self.cur_list.pg_field = 0i32;
        self.shown_mode = 0i32;
        // §1168
        self.page_contents = 0i32;
        self.page_tail = 4999997i32;
        self.mem[(4999997i32) as usize].set_hh_rh(0i32);
        self.last_glue = 268435455i32;
        self.last_penalty = 0i32;
        self.last_kern = 0i32;
        self.last_node_type = (1i32).wrapping_neg();
        self.page_so_far[(7i32) as usize] = 0i32;
        self.page_max_depth = 0i32;
        // §272
        {
            let __for_end_2 = 629929i32;
            k = 629018i32;
            while k <= __for_end_2 {
                self.xeq_level[((k) - 629018) as usize] = 1i32;
                k = k.wrapping_add(1);
            }
        }
        // §276
        self.no_new_control_sequence = true;
        self.prim[(0i32) as usize].set_lh(0i32);
        self.prim[(0i32) as usize].set_rh(0i32);
        {
            let __for_end_2 = 2100i32;
            k = 1i32;
            while k <= __for_end_2 {
                { let __v6 = self.prim[(0i32) as usize]; self.prim[(k) as usize] = __v6; }
                k = k.wrapping_add(1);
            }
        }
        self.hash[((514i32) - 514) as usize].set_lh(0i32);
        self.hash[((514i32) - 514) as usize].set_rh(0i32);
        {
            let __for_end_2 = 626626i32;
            k = 515i32;
            while k <= __for_end_2 {
                { let __v7 = self.hash[((514i32) - 514) as usize]; self.hash[((k) - 514) as usize] = __v7; }
                k = k.wrapping_add(1);
            }
        }
        // §294
        self.save_ptr = 0i32;
        self.cur_level = 1i32;
        self.cur_group = 0i32;
        self.cur_boundary = 0i32;
        self.max_save_stack = 0i32;
        // §309
        self.mag_set = 0i32;
        // §390
        self.is_in_csname = false;
        // §409
        self.cur_mark[(0i32) as usize] = 0i32;
        self.cur_mark[(1i32) as usize] = 0i32;
        self.cur_mark[(2i32) as usize] = 0i32;
        self.cur_mark[(3i32) as usize] = 0i32;
        self.cur_mark[(4i32) as usize] = 0i32;
        // §465
        self.cur_val = 0i32;
        self.cur_val_level = 0i32;
        self.radix = 0i32;
        self.cur_order = 0i32;
        // §507
        {
            let __for_end_2 = 16i32;
            k = 0i32;
            while k <= __for_end_2 {
                self.read_open[(k) as usize] = 2i32;
                k = k.wrapping_add(1);
            }
        }
        // §516
        self.cond_ptr = 0i32;
        self.if_limit = 0i32;
        self.cur_if = 0i32;
        self.if_line = 0i32;
        // §547
        crate::system::copy_str(&mut self.TEX_format_default, "TeXformats:plain.fmt");
        // §577
        {
            let __for_end_2 = font_max;
            k = 0i32;
            while k <= __for_end_2 {
                { let __v8 = false; self.font_used[(k) as usize] = __v8; }
                k = k.wrapping_add(1);
            }
        }
        // §582
        self.null_character.set_b0(0i32);
        self.null_character.set_b1(0i32);
        self.null_character.set_b2(0i32);
        self.null_character.set_b3(0i32);
        // §620
        self.total_pages = 0i32;
        self.max_v = 0i32;
        self.max_h = 0i32;
        self.max_push = 0i32;
        self.last_bop = (1i32).wrapping_neg();
        self.doing_leaders = false;
        self.dead_cycles = 0i32;
        self.cur_s = (1i32).wrapping_neg();
        // §623
        self.half_buf = (dvi_buf_size / 2i32);
        self.dvi_limit = dvi_buf_size;
        self.dvi_ptr = 0i32;
        self.dvi_offset = 0i32;
        self.dvi_gone = 0i32;
        // §633
        self.down_ptr = 0i32;
        self.right_ptr = 0i32;
        // §677
        self.pdf_mem_ptr = 1i32;
        self.pdf_mem_size = inf_pdf_mem_size;
        // §681
        self.pdf_gone = ((0i32) as i64);
        self.pdf_os_mode = false;
        self.pdf_ptr = 0i32;
        self.pdf_op_ptr = 0i32;
        self.pdf_os_ptr = 0i32;
        self.pdf_os_cur_objnum = 0i32;
        self.pdf_os_cntr = 0i32;
        self.pdf_buf_size = pdf_op_buf_size;
        self.pdf_os_buf_size = inf_pdf_os_buf_size;
        self.pdf_buf_is_os = false;
        self.pdf_seek_write_length = false;
        self.zip_write_state = 0i32;
        self.pdf_version_written = false;
        self.fixed_pdfoutput_set = false;
        self.fixed_pdf_draftmode_set = false;
        // §688
        self.one_bp = 65782i32;
        self.one_hundred_bp = 6578176i32;
        self.one_hundred_inch = 473628672i32;
        self.one_inch = 226i32;
        self.ten_pow[(0i32) as usize] = 1i32;
        {
            let __for_end_2 = 9i32;
            i = 1i32;
            while i <= __for_end_2 {
                { let __v9 = (10i32).wrapping_mul(self.ten_pow[((i).wrapping_sub(1i32)) as usize]); self.ten_pow[(i) as usize] = __v9; }
                i = i.wrapping_add(1);
            }
        }
        self.init_pdf_output = false;
        // §697
        self.obj_ptr = 0i32;
        self.sys_obj_ptr = 0i32;
        self.obj_tab_size = inf_obj_tab_size;
        self.dest_names_size = inf_dest_names_size;
        {
            let __for_end_2 = 10i32;
            k = 1i32;
            while k <= __for_end_2 {
                self.head_tab[((k) - 1) as usize] = 0i32;
                k = k.wrapping_add(1);
            }
        }
        self.pdf_box_spec_media = 1i32;
        self.pdf_box_spec_crop = 2i32;
        self.pdf_box_spec_bleed = 3i32;
        self.pdf_box_spec_trim = 4i32;
        self.pdf_box_spec_art = 5i32;
        self.pdf_dummy_font = 0i32;
        // §709
        self.pdf_resname_prefix = 0i32;
        self.last_tokens_string = 0i32;
        // §711
        self.vf_nf = 0i32;
        // §724
        self.vf_cur_s = 0i32;
        self.vf_stack_ptr = 0i32;
        // §820
        self.adjust_tail = 0i32;
        self.last_badness = 0i32;
        // §830
        self.pre_adjust_tail = 0i32;
        // §838
        self.pack_begin_line = 0i32;
        // §861
        self.empty_field.set_rh(0i32);
        self.empty_field.set_lh(0i32);
        self.null_delimiter.set_b0(0i32);
        self.null_delimiter.set_b1(0i32);
        self.null_delimiter.set_b2(0i32);
        self.null_delimiter.set_b3(0i32);
        // §947
        self.align_ptr = 0i32;
        self.cur_align = 0i32;
        self.cur_span = 0i32;
        self.cur_loop = 0i32;
        self.cur_head = 0i32;
        self.cur_tail = 0i32;
        self.cur_pre_head = 0i32;
        self.cur_pre_tail = 0i32;
        // §1105
        {
            let __for_end_2 = 8191i32;
            z = 0i32;
            while z <= __for_end_2 {
                {
                    self.hyph_word[(z) as usize] = 0i32;
                    self.hyph_list[(z) as usize] = 0i32;
                }
                z = z.wrapping_add(1);
            }
        }
        self.hyph_count = 0i32;
        // §1167
        self.output_active = false;
        self.output_can_end = false;
        self.insert_penalties = 0i32;
        // §1210
        self.ligature_present = false;
        self.cancel_boundary = false;
        self.lft_hit = false;
        self.rt_hit = false;
        self.ins_disc = false;
        // §1445
        self.after_token = 0i32;
        // §1460
        self.long_help_seen = false;
        // §1478
        self.format_ident = 0i32;
        // §1523
        {
            let __for_end_2 = 17i32;
            k = 0i32;
            while k <= __for_end_2 {
                { let __v10 = false; self.write_open[(k) as usize] = __v10; }
                k = k.wrapping_add(1);
            }
        }
        // §1551
        self.alt_rule = 0i32;
        self.warn_pdfpagebox = true;
        // §1571
        self.count_do_snapy = 0i32;
        // §1584
        { let mut __f0 = ::core::mem::take(&mut self.epochseconds); let mut __f1 = ::core::mem::take(&mut self.microseconds); let __r = self.seconds_and_micros(&mut __f0, &mut __f1); self.epochseconds = __f0; self.microseconds = __f1; __r };
        self.init_start_time();
        // §1629
        self.pdf_first_outline = 0i32;
        self.pdf_last_outline = 0i32;
        self.pdf_parent_outline = 0i32;
        self.pdf_obj_count = 0i32;
        self.pdf_xform_count = 0i32;
        self.pdf_ximage_count = 0i32;
        self.pdf_dest_names_ptr = 0i32;
        self.pdf_info_toks = 0i32;
        self.pdf_catalog_toks = 0i32;
        self.pdf_names_toks = 0i32;
        self.pdf_catalog_openaction = 0i32;
        self.pdf_trailer_toks = 0i32;
        self.pdf_trailer_id_toks = 0i32;
        self.gen_faked_interword_space = false;
        self.gen_running_link = true;
        self.pdf_space_font_name = 1918i32;
        // §1634
        self.pdf_link_stack_ptr = 0i32;
        // §1706
        self.LR_ptr = 0i32;
        self.LR_problems = 0i32;
        self.cur_dir = 0i32;
        // §1751
        self.pseudo_files = 0i32;
        // §1817
        self.sa_root[(6i32) as usize] = 0i32;
        self.sa_null.set_hh_lh(0i32);
        self.sa_null.set_hh_rh(0i32);
        // §1836
        self.sa_chain = 0i32;
        self.sa_level = 0i32;
        // §1860
        self.disc_ptr[((2i32) - 1) as usize] = 0i32;
        self.disc_ptr[((3i32) - 1) as usize] = 0i32;
        // §1871
        self.expand_depth_count = 0i32;
        self.pk_dpi = 72i32;
        // §1876
        self.stop_at_space = true;
        // §1880
        self.mltex_p = false;
        self.mltex_enabled_p = false;
        // §1886
        self.halting_on_error_p = false;
        // §182
        {
            let __for_end_2 = 19i32;
            k = 1i32;
            while k <= __for_end_2 {
                self.mem[(k) as usize].set_int(0i32);
                k = k.wrapping_add(1);
            }
        }
        k = 0i32;
        while (k <= 19i32) {
            {
                self.mem[(k) as usize].set_hh_rh(1i32);
                self.mem[(k) as usize].set_hh_b0(0i32);
                self.mem[(k) as usize].set_hh_b1(0i32);
                k = (k).wrapping_add(4i32);
            }
        }
        self.mem[(6i32) as usize].set_int(65536i32);
        self.mem[(4i32) as usize].set_hh_b0(1i32);
        self.mem[(10i32) as usize].set_int(65536i32);
        self.mem[(8i32) as usize].set_hh_b0(2i32);
        self.mem[(14i32) as usize].set_int(65536i32);
        self.mem[(12i32) as usize].set_hh_b0(1i32);
        self.mem[(15i32) as usize].set_int(65536i32);
        self.mem[(12i32) as usize].set_hh_b1(1i32);
        self.mem[(18i32) as usize].set_int((65536i32).wrapping_neg());
        self.mem[(16i32) as usize].set_hh_b0(1i32);
        self.rover = 20i32;
        { let __ix11 = self.rover; self.mem[(__ix11) as usize].set_hh_rh(268435455i32); }
        { let __ix12 = self.rover; self.mem[(__ix12) as usize].set_hh_lh(1000i32); }
        { let __ix13 = (self.rover).wrapping_add(1i32); let __v14 = self.rover; self.mem[(__ix13) as usize].set_hh_lh(__v14); }
        { let __ix15 = (self.rover).wrapping_add(1i32); let __v16 = self.rover; self.mem[(__ix15) as usize].set_hh_rh(__v16); }
        self.lo_mem_max = (self.rover).wrapping_add(1000i32);
        { let __ix17 = self.lo_mem_max; self.mem[(__ix17) as usize].set_hh_rh(0i32); }
        { let __ix18 = self.lo_mem_max; self.mem[(__ix18) as usize].set_hh_lh(0i32); }
        {
            let __for_end_2 = 4999999i32;
            k = 4999985i32;
            while k <= __for_end_2 {
                { let __v19 = self.mem[(self.lo_mem_max) as usize]; self.mem[(k) as usize] = __v19; }
                k = k.wrapping_add(1);
            }
        }
        // §966
        self.mem[(4999989i32) as usize].set_hh_lh(619614i32);
        // §973
        self.mem[(4999990i32) as usize].set_hh_rh(256i32);
        self.mem[(4999990i32) as usize].set_hh_lh(0i32);
        // §996
        self.mem[(4999992i32) as usize].set_hh_b0(1i32);
        self.mem[(4999993i32) as usize].set_hh_lh(268435455i32);
        self.mem[(4999992i32) as usize].set_hh_b1(0i32);
        // §1158
        self.mem[(4999999i32) as usize].set_hh_b1(255i32);
        self.mem[(4999999i32) as usize].set_hh_b0(1i32);
        self.mem[(4999999i32) as usize].set_hh_rh(4999999i32);
        // §1165
        self.mem[(4999997i32) as usize].set_hh_b0(10i32);
        self.mem[(4999997i32) as usize].set_hh_b1(0i32);
        // §182
        self.avail = 0i32;
        self.mem_end = 4999999i32;
        self.hi_mem_min = 4999985i32;
        self.var_used = 20i32;
        self.dyn_used = 15i32;
        // §240
        self.eqtb[((626627i32) - 1) as usize].set_hh_b0(104i32);
        self.eqtb[((626627i32) - 1) as usize].set_hh_rh(0i32);
        self.eqtb[((626627i32) - 1) as usize].set_hh_b1(0i32);
        {
            let __for_end_2 = 626626i32;
            k = 1i32;
            while k <= __for_end_2 {
                { let __v20 = self.eqtb[((626627i32) - 1) as usize]; self.eqtb[((k) - 1) as usize] = __v20; }
                k = k.wrapping_add(1);
            }
        }
        // §246
        self.eqtb[((626628i32) - 1) as usize].set_hh_rh(0i32);
        self.eqtb[((626628i32) - 1) as usize].set_hh_b1(1i32);
        self.eqtb[((626628i32) - 1) as usize].set_hh_b0(120i32);
        {
            let __for_end_2 = 627157i32;
            k = 626629i32;
            while k <= __for_end_2 {
                { let __v21 = self.eqtb[((626628i32) - 1) as usize]; self.eqtb[((k) - 1) as usize] = __v21; }
                k = k.wrapping_add(1);
            }
        }
        { let __v22 = (self.mem[(0i32) as usize].hh().rh()).wrapping_add(530i32); self.mem[(0i32) as usize].set_hh_rh(__v22); }
        // §250
        self.eqtb[((627158i32) - 1) as usize].set_hh_rh(0i32);
        self.eqtb[((627158i32) - 1) as usize].set_hh_b0(121i32);
        self.eqtb[((627158i32) - 1) as usize].set_hh_b1(1i32);
        {
            let __for_end_2 = 627432i32;
            k = 627429i32;
            while k <= __for_end_2 {
                { let __v23 = self.eqtb[((627158i32) - 1) as usize]; self.eqtb[((k) - 1) as usize] = __v23; }
                k = k.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 627428i32;
            k = 627159i32;
            while k <= __for_end_2 {
                { let __v24 = self.eqtb[((626627i32) - 1) as usize]; self.eqtb[((k) - 1) as usize] = __v24; }
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[((627433i32) - 1) as usize].set_hh_rh(0i32);
        self.eqtb[((627433i32) - 1) as usize].set_hh_b0(122i32);
        self.eqtb[((627433i32) - 1) as usize].set_hh_b1(1i32);
        {
            let __for_end_2 = 627688i32;
            k = 627434i32;
            while k <= __for_end_2 {
                { let __v25 = self.eqtb[((627433i32) - 1) as usize]; self.eqtb[((k) - 1) as usize] = __v25; }
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[((627689i32) - 1) as usize].set_hh_rh(0i32);
        self.eqtb[((627689i32) - 1) as usize].set_hh_b0(123i32);
        self.eqtb[((627689i32) - 1) as usize].set_hh_b1(1i32);
        {
            let __for_end_2 = 627737i32;
            k = 627690i32;
            while k <= __for_end_2 {
                { let __v26 = self.eqtb[((627689i32) - 1) as usize]; self.eqtb[((k) - 1) as usize] = __v26; }
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[((627738i32) - 1) as usize].set_hh_rh(0i32);
        self.eqtb[((627738i32) - 1) as usize].set_hh_b0(123i32);
        self.eqtb[((627738i32) - 1) as usize].set_hh_b1(1i32);
        {
            let __for_end_2 = 629017i32;
            k = 627739i32;
            while k <= __for_end_2 {
                { let __v27 = self.eqtb[((627738i32) - 1) as usize]; self.eqtb[((k) - 1) as usize] = __v27; }
                k = k.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 255i32;
            k = 0i32;
            while k <= __for_end_2 {
                {
                    self.eqtb[(((627738i32).wrapping_add(k)) - 1) as usize].set_hh_rh(12i32);
                    self.eqtb[(((628762i32).wrapping_add(k)) - 1) as usize].set_hh_rh((k).wrapping_add(0i32));
                    self.eqtb[(((628506i32).wrapping_add(k)) - 1) as usize].set_hh_rh(1000i32);
                }
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[((627751i32) - 1) as usize].set_hh_rh(5i32);
        self.eqtb[((627770i32) - 1) as usize].set_hh_rh(10i32);
        self.eqtb[((627830i32) - 1) as usize].set_hh_rh(0i32);
        self.eqtb[((627775i32) - 1) as usize].set_hh_rh(14i32);
        self.eqtb[((627865i32) - 1) as usize].set_hh_rh(15i32);
        self.eqtb[((627738i32) - 1) as usize].set_hh_rh(9i32);
        {
            let __for_end_2 = 57i32;
            k = 48i32;
            while k <= __for_end_2 {
                self.eqtb[(((628762i32).wrapping_add(k)) - 1) as usize].set_hh_rh((k).wrapping_add(28672i32));
                k = k.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 90i32;
            k = 65i32;
            while k <= __for_end_2 {
                {
                    self.eqtb[(((627738i32).wrapping_add(k)) - 1) as usize].set_hh_rh(11i32);
                    self.eqtb[((((627738i32).wrapping_add(k)).wrapping_add(32i32)) - 1) as usize].set_hh_rh(11i32);
                    self.eqtb[(((628762i32).wrapping_add(k)) - 1) as usize].set_hh_rh((k).wrapping_add(28928i32));
                    self.eqtb[((((628762i32).wrapping_add(k)).wrapping_add(32i32)) - 1) as usize].set_hh_rh((k).wrapping_add(28960i32));
                    self.eqtb[(((627994i32).wrapping_add(k)) - 1) as usize].set_hh_rh((k).wrapping_add(32i32));
                    self.eqtb[((((627994i32).wrapping_add(k)).wrapping_add(32i32)) - 1) as usize].set_hh_rh((k).wrapping_add(32i32));
                    self.eqtb[(((628250i32).wrapping_add(k)) - 1) as usize].set_hh_rh(k);
                    self.eqtb[((((628250i32).wrapping_add(k)).wrapping_add(32i32)) - 1) as usize].set_hh_rh(k);
                    self.eqtb[(((628506i32).wrapping_add(k)) - 1) as usize].set_hh_rh(999i32);
                }
                k = k.wrapping_add(1);
            }
        }
        // §258
        {
            let __for_end_2 = 629383i32;
            k = 629018i32;
            while k <= __for_end_2 {
                self.eqtb[((k) - 1) as usize].set_int(0i32);
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[((629073i32) - 1) as usize].set_int(256i32);
        self.eqtb[((629074i32) - 1) as usize].set_int((1i32).wrapping_neg());
        self.eqtb[((629035i32) - 1) as usize].set_int(1000i32);
        self.eqtb[((629019i32) - 1) as usize].set_int(10000i32);
        self.eqtb[((629059i32) - 1) as usize].set_int(1i32);
        self.eqtb[((629058i32) - 1) as usize].set_int(25i32);
        self.eqtb[((629063i32) - 1) as usize].set_int(92i32);
        self.eqtb[((629066i32) - 1) as usize].set_int(13i32);
        {
            let __for_end_2 = 255i32;
            k = 0i32;
            while k <= __for_end_2 {
                self.eqtb[(((629384i32).wrapping_add(k)) - 1) as usize].set_int((1i32).wrapping_neg());
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[((629430i32) - 1) as usize].set_int(0i32);
        self.eqtb[((629078i32) - 1) as usize].set_int((1i32).wrapping_neg());
        // §268
        {
            let __for_end_2 = 629929i32;
            k = 629640i32;
            while k <= __for_end_2 {
                self.eqtb[((k) - 1) as usize].set_int(0i32);
                k = k.wrapping_add(1);
            }
        }
        // §277
        self.prim_used = 2100i32;
        self.hash_used = 615514i32;
        self.cs_count = 0i32;
        self.eqtb[((615523i32) - 1) as usize].set_hh_b0(119i32);
        self.hash[((615523i32) - 514) as usize].set_rh(576i32);
        self.eqtb[((615525i32) - 1) as usize].set_hh_b0(39i32);
        self.eqtb[((615525i32) - 1) as usize].set_hh_rh(1i32);
        self.eqtb[((615525i32) - 1) as usize].set_hh_b1(1i32);
        self.hash[((615525i32) - 514) as usize].set_rh(577i32);
        // §578
        self.font_ptr = 0i32;
        self.fmem_ptr = 7i32;
        self.font_name[(0i32) as usize] = 959i32;
        self.font_area[(0i32) as usize] = 348i32;
        self.hyphen_char[(0i32) as usize] = 45i32;
        self.skew_char[(0i32) as usize] = (1i32).wrapping_neg();
        self.bchar_label[(0i32) as usize] = 0i32;
        self.font_bchar[(0i32) as usize] = 256i32;
        self.font_false_bchar[(0i32) as usize] = 256i32;
        self.font_bc[(0i32) as usize] = 1i32;
        self.font_ec[(0i32) as usize] = 0i32;
        self.font_size[(0i32) as usize] = 0i32;
        self.font_dsize[(0i32) as usize] = 0i32;
        self.char_base[(0i32) as usize] = 0i32;
        self.width_base[(0i32) as usize] = 0i32;
        self.height_base[(0i32) as usize] = 0i32;
        self.depth_base[(0i32) as usize] = 0i32;
        self.italic_base[(0i32) as usize] = 0i32;
        self.lig_kern_base[(0i32) as usize] = 0i32;
        self.kern_base[(0i32) as usize] = 0i32;
        self.exten_base[(0i32) as usize] = 0i32;
        self.font_glue[(0i32) as usize] = 0i32;
        self.font_params[(0i32) as usize] = 7i32;
        self.param_base[(0i32) as usize] = (1i32).wrapping_neg();
        {
            let __for_end_2 = 6i32;
            k = 0i32;
            while k <= __for_end_2 {
                self.font_info[(k) as usize].set_int(0i32);
                k = k.wrapping_add(1);
            }
        }
        // §672
        { let __v28 = ((self.one_hundred_inch).wrapping_add(50i32) / 100i32); self.eqtb[((629661i32) - 1) as usize].set_int(__v28); }
        { let __v29 = ((self.one_hundred_inch).wrapping_add(50i32) / 100i32); self.eqtb[((629662i32) - 1) as usize].set_int(__v29); }
        self.eqtb[((629080i32) - 1) as usize].set_int(9i32);
        self.eqtb[((629100i32) - 1) as usize].set_int(0i32);
        self.eqtb[((629081i32) - 1) as usize].set_int(3i32);
        self.eqtb[((629083i32) - 1) as usize].set_int(72i32);
        self.eqtb[((629088i32) - 1) as usize].set_int(1i32);
        self.eqtb[((629089i32) - 1) as usize].set_int(4i32);
        self.eqtb[((629093i32) - 1) as usize].set_int(1000i32);
        self.eqtb[((629094i32) - 1) as usize].set_int(2200i32);
        self.eqtb[((629095i32) - 1) as usize].set_int(1i32);
        self.eqtb[((629096i32) - 1) as usize].set_int(0i32);
        { let __v30 = self.one_bp; self.eqtb[((629673i32) - 1) as usize].set_int(__v30); }
        self.eqtb[((629105i32) - 1) as usize].set_int(0i32);
        // §1064
        self.eqtb[((629672i32) - 1) as usize].set_int((65536000i32).wrapping_neg());
        { let __v31 = self.eqtb[((629672i32) - 1) as usize].int(); self.eqtb[((629670i32) - 1) as usize].set_int(__v31); }
        { let __v32 = self.eqtb[((629672i32) - 1) as usize].int(); self.eqtb[((629671i32) - 1) as usize].set_int(__v32); }
        { let __v33 = self.eqtb[((629672i32) - 1) as usize].int(); self.eqtb[((629668i32) - 1) as usize].set_int(__v33); }
        { let __v34 = self.eqtb[((629672i32) - 1) as usize].int(); self.eqtb[((629669i32) - 1) as usize].set_int(__v34); }
        // §1123
        {
            let __for_end_2 = trie_op_size;
            k = (trie_op_size).wrapping_neg();
            while k <= __for_end_2 {
                self.trie_op_hash[((k) + 35111) as usize] = 0i32;
                k = k.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 255i32;
            k = 0i32;
            while k <= __for_end_2 {
                self.trie_used[(k) as usize] = 0i32;
                k = k.wrapping_add(1);
            }
        }
        self.trie_op_ptr = 0i32;
        // §1128
        self.trie_not_ready = true;
        self.trie_l[(0i32) as usize] = 0i32;
        self.trie_c[(0i32) as usize] = 0i32;
        self.trie_ptr = 0i32;
        // §1394
        self.hash[((615514i32) - 514) as usize].set_rh(1609i32);
        // §1479
        if self.ini_version() {
            self.format_ident = 1683i32;
        }
        // §1616
        self.hash[((615522i32) - 514) as usize].set_rh(1903i32);
        self.eqtb[((615522i32) - 1) as usize].set_hh_b1(1i32);
        self.eqtb[((615522i32) - 1) as usize].set_hh_b0(116i32);
        self.eqtb[((615522i32) - 1) as usize].set_hh_rh(0i32);
        // §1653
        self.eTeX_mode = 0i32;
        // §1812
        self.max_reg_num = 255i32;
        self.max_reg_help_line = 789i32;
        // §1818
        {
            let __for_end_2 = 5i32;
            i = 0i32;
            while i <= __for_end_2 {
                self.sa_root[(i) as usize] = 0i32;
                i = i.wrapping_add(1);
            }
        }
        // §1854
        self.trie_r[(0i32) as usize] = 0i32;
        self.hyph_start = 0i32;
    }

    /// The macros of \.{pdftex.h}, as \PASCAL\ routines. They come after the
    /// external declarations and before the basic printing procedures.
    /// @<Declare the routines of pdf\TeX's C parts
    // §1874
    pub fn pdf_char_bit(&mut self, mut c: eight_bits) -> i32 {
        let mut pdf_char_bit: i32 = 0;
        let mut k: i32 = 0; // §1874
        let mut b: i32 = 0; // §1874
        b = 1i32;
        {
            let __for_end_2 = (c % 8i32);
            k = 1i32;
            while k <= __for_end_2 {
                b = (b).wrapping_add(b);
                k = k.wrapping_add(1);
            }
        }
        pdf_char_bit = b;
        pdf_char_bit
    }

    /// The macros of \.{pdftex.h}, as \PASCAL\ routines. They come after the
    /// external declarations and before the basic printing procedures.
    /// @<Declare the routines of pdf\TeX's C parts
    // §1874
    pub fn pdf_char_marked(&mut self, mut f: internal_font_number, mut c: eight_bits) -> bool {
        let mut pdf_char_marked: bool = false;
        pdf_char_marked = ((((self.pdf_char_used[(f) as usize][((c / 8i32)) as usize] / self.pdf_char_bit(c))) % 2) != 0);
        pdf_char_marked
    }

    /// The macros of \.{pdftex.h}, as \PASCAL\ routines. They come after the
    /// external declarations and before the basic printing procedures.
    /// @<Declare the routines of pdf\TeX's C parts
    // §1874
    pub fn pdf_mark_char(&mut self, mut f: internal_font_number, mut c: eight_bits) {
        if (!self.pdf_char_marked(f, c)) {
            { let __v35 = (self.pdf_char_used[(f) as usize][((c / 8i32)) as usize]).wrapping_add(self.pdf_char_bit(c)); self.pdf_char_used[(f) as usize][((c / 8i32)) as usize] = __v35; }
        }
    }

    /// The macros of \.{pdftex.h}, as \PASCAL\ routines. They come after the
    /// external declarations and before the basic printing procedures.
    /// @<Declare the routines of pdf\TeX's C parts
    // §1874
    pub fn get_lp_code(&mut self, mut f: internal_font_number, mut c: eight_bits) -> i32 {
        let mut get_lp_code: i32 = 0;
        if (self.pdf_font_lp_base[(f) as usize] == 0i32) {
            get_lp_code = 0i32;
        } else {
            get_lp_code = self.pdf_mem[((self.pdf_font_lp_base[(f) as usize]).wrapping_add(c)) as usize];
        }
        get_lp_code
    }

    /// The macros of \.{pdftex.h}, as \PASCAL\ routines. They come after the
    /// external declarations and before the basic printing procedures.
    /// @<Declare the routines of pdf\TeX's C parts
    // §1874
    pub fn get_rp_code(&mut self, mut f: internal_font_number, mut c: eight_bits) -> i32 {
        let mut get_rp_code: i32 = 0;
        if (self.pdf_font_rp_base[(f) as usize] == 0i32) {
            get_rp_code = 0i32;
        } else {
            get_rp_code = self.pdf_mem[((self.pdf_font_rp_base[(f) as usize]).wrapping_add(c)) as usize];
        }
        get_rp_code
    }

    /// The macros of \.{pdftex.h}, as \PASCAL\ routines. They come after the
    /// external declarations and before the basic printing procedures.
    /// @<Declare the routines of pdf\TeX's C parts
    // §1874
    pub fn get_ef_code(&mut self, mut f: internal_font_number, mut c: eight_bits) -> i32 {
        let mut get_ef_code: i32 = 0;
        if (self.pdf_font_ef_base[(f) as usize] == 0i32) {
            get_ef_code = 1000i32;
        } else {
            get_ef_code = self.pdf_mem[((self.pdf_font_ef_base[(f) as usize]).wrapping_add(c)) as usize];
        }
        get_ef_code
    }

    /// The macros of \.{pdftex.h}, as \PASCAL\ routines. They come after the
    /// external declarations and before the basic printing procedures.
    /// @<Declare the routines of pdf\TeX's C parts
    // §1874
    pub fn get_kn_bs_code(&mut self, mut f: internal_font_number, mut c: i32) -> i32 {
        let mut get_kn_bs_code: i32 = 0;
        let mut i: i32 = 0; // §1874
        i = self.pdf_font_kn_bs_base[(f) as usize];
        if (i == 0i32) {
            get_kn_bs_code = 0i32;
        } else {
            {
                i = (i).wrapping_add(c);
                if ((i < 0i32) || (i > self.pdf_mem_size)) {
                    get_kn_bs_code = 0i32;
                } else {
                    get_kn_bs_code = self.pdf_mem[(i) as usize];
                }
            }
        }
        get_kn_bs_code
    }

    /// The macros of \.{pdftex.h}, as \PASCAL\ routines. They come after the
    /// external declarations and before the basic printing procedures.
    /// @<Declare the routines of pdf\TeX's C parts
    // §1874
    pub fn get_st_bs_code(&mut self, mut f: internal_font_number, mut c: i32) -> i32 {
        let mut get_st_bs_code: i32 = 0;
        let mut i: i32 = 0; // §1874
        i = self.pdf_font_st_bs_base[(f) as usize];
        if (i == 0i32) {
            get_st_bs_code = 0i32;
        } else {
            {
                i = (i).wrapping_add(c);
                if ((i < 0i32) || (i > self.pdf_mem_size)) {
                    get_st_bs_code = 0i32;
                } else {
                    get_st_bs_code = self.pdf_mem[(i) as usize];
                }
            }
        }
        get_st_bs_code
    }

    /// The macros of \.{pdftex.h}, as \PASCAL\ routines. They come after the
    /// external declarations and before the basic printing procedures.
    /// @<Declare the routines of pdf\TeX's C parts
    // §1874
    pub fn get_sh_bs_code(&mut self, mut f: internal_font_number, mut c: i32) -> i32 {
        let mut get_sh_bs_code: i32 = 0;
        let mut i: i32 = 0; // §1874
        i = self.pdf_font_sh_bs_base[(f) as usize];
        if (i == 0i32) {
            get_sh_bs_code = 0i32;
        } else {
            {
                i = (i).wrapping_add(c);
                if ((i < 0i32) || (i > self.pdf_mem_size)) {
                    get_sh_bs_code = 0i32;
                } else {
                    get_sh_bs_code = self.pdf_mem[(i) as usize];
                }
            }
        }
        get_sh_bs_code
    }

    /// The macros of \.{pdftex.h}, as \PASCAL\ routines. They come after the
    /// external declarations and before the basic printing procedures.
    /// @<Declare the routines of pdf\TeX's C parts
    // §1874
    pub fn get_kn_bc_code(&mut self, mut f: internal_font_number, mut c: eight_bits) -> i32 {
        let mut get_kn_bc_code: i32 = 0;
        if (self.pdf_font_kn_bc_base[(f) as usize] == 0i32) {
            get_kn_bc_code = 0i32;
        } else {
            get_kn_bc_code = self.pdf_mem[((self.pdf_font_kn_bc_base[(f) as usize]).wrapping_add(c)) as usize];
        }
        get_kn_bc_code
    }

    /// The macros of \.{pdftex.h}, as \PASCAL\ routines. They come after the
    /// external declarations and before the basic printing procedures.
    /// @<Declare the routines of pdf\TeX's C parts
    // §1874
    pub fn get_kn_ac_code(&mut self, mut f: internal_font_number, mut c: eight_bits) -> i32 {
        let mut get_kn_ac_code: i32 = 0;
        if (self.pdf_font_kn_ac_base[(f) as usize] == 0i32) {
            get_kn_ac_code = 0i32;
        } else {
            get_kn_ac_code = self.pdf_mem[((self.pdf_font_kn_ac_base[(f) as usize]).wrapping_add(c)) as usize];
        }
        get_kn_ac_code
    }

    /// The macros of \.{pdftex.h}, as \PASCAL\ routines. They come after the
    /// external declarations and before the basic printing procedures.
    /// @<Declare the routines of pdf\TeX's C parts
    // §1874
    pub fn pdf_buf_get(&mut self, mut i: i32) -> eight_bits {
        let mut pdf_buf_get: eight_bits = 0;
        if self.pdf_buf_is_os {
            pdf_buf_get = self.pdf_os_buf[(i) as usize];
        } else {
            pdf_buf_get = self.pdf_op_buf[(i) as usize];
        }
        pdf_buf_get
    }

    /// The macros of \.{pdftex.h}, as \PASCAL\ routines. They come after the
    /// external declarations and before the basic printing procedures.
    /// @<Declare the routines of pdf\TeX's C parts
    // §1874
    pub fn pdf_buf_set(&mut self, mut i: i32, mut b: eight_bits) {
        if self.pdf_buf_is_os {
            self.pdf_os_buf[(i) as usize] = b;
        } else {
            self.pdf_op_buf[(i) as usize] = b;
        }
    }

    /// tex.ch's `scan_file_name_braced` (its part \.{[54/web2c]}): when
    /// `scan_file_name` finds a `left_brace`, the file name is a balanced token
    /// list, expanded as it is read, converted into a string and fed to `more_name`
    /// character by character, with spaces allowed.
    /// @<Declare web2c's file-name procedures
    // §1877
    pub fn scan_file_name_braced(&mut self) {
        let mut save_scanner_status: small_number = 0; // §1877
        let mut save_def_ref: halfword = 0; // §1877
        let mut save_cur_cs: halfword = 0; // §1877
        let mut s: str_number = 0; // §1877
        let mut p: halfword = 0; // §1877
        let mut i: i32 = 0; // §1877
        let mut save_stop_at_space: bool = false; // §1877
        let mut dummy: bool = false; // §1877
        save_scanner_status = self.scanner_status;
        save_def_ref = self.def_ref;
        save_cur_cs = self.cur_cs;
        self.cur_cs = self.warning_index;
        if (self.scan_toks(false, true) != 0i32) {
        }
        self.old_setting = self.selector;
        self.selector = 21i32;
        self.show_token_list(self.mem[(self.def_ref) as usize].hh().rh(), 0i32, (pool_size).wrapping_sub(self.pool_ptr));
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
            let __for_end_2 = (self.str_start[((s).wrapping_add(1i32)) as usize]).wrapping_sub(1i32);
            i = self.str_start[(s) as usize];
            while i <= __for_end_2 {
                dummy = self.more_name(self.str_pool[(i) as usize]);
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
            19 => {
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
            18 => {
                {
                    {
                        crate::system::wr_ln(&mut self.log_file);
                    }
                    self.file_offset = 0i32;
                }
            }
            17 => {
                {
                    {
                        crate::system::wr_ln(&mut self.term_out);
                    }
                    self.term_offset = 0i32;
                }
            }
            16 | 20 | 21 => {
            }
            _ => {
                {
                    crate::system::wr_ln(&mut self.write_file[(self.selector) as usize]);
                }
            }
        }
    }

    /// The `print_char` procedure sends one character to the desired destination,
    /// using the `xchr` array to map it into an external character compatible with
    /// `input_ln`. All printing comes through `print_ln` or `print_char`.
    /// @<Basic printing...
    // §58
    pub fn print_char(&mut self, mut s: ASCII_code) {
        'l_exit_f: {
            if (s == self.eqtb[((629067i32) - 1) as usize].int()) {
                if (self.selector < 20i32) {
                    {
                        self.print_ln();
                        break 'l_exit_f;
                    }
                }
            }
            match self.selector {
                19 => {
                    {
                        {
                            let __w0 = self.xchr[(s) as usize];
                            crate::system::wr_char(&mut self.term_out, __w0);
                        }
                        {
                            let __w0 = self.xchr[(s) as usize];
                            crate::system::wr_char(&mut self.log_file, __w0);
                        }
                        self.term_offset = (self.term_offset).wrapping_add(1i32);
                        self.file_offset = (self.file_offset).wrapping_add(1i32);
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
                18 => {
                    {
                        {
                            let __w0 = self.xchr[(s) as usize];
                            crate::system::wr_char(&mut self.log_file, __w0);
                        }
                        self.file_offset = (self.file_offset).wrapping_add(1i32);
                        if (self.file_offset == self.max_print_line) {
                            self.print_ln();
                        }
                    }
                }
                17 => {
                    {
                        {
                            let __w0 = self.xchr[(s) as usize];
                            crate::system::wr_char(&mut self.term_out, __w0);
                        }
                        self.term_offset = (self.term_offset).wrapping_add(1i32);
                        if (self.term_offset == self.max_print_line) {
                            self.print_ln();
                        }
                    }
                }
                16 => {
                }
                20 => {
                    if (self.tally < self.trick_count) {
                        self.trick_buf[((self.tally % self.error_line)) as usize] = s;
                    }
                }
                21 => {
                    {
                        if (self.pool_ptr < pool_size) {
                            {
                                self.str_pool[(self.pool_ptr) as usize] = s;
                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                            }
                        }
                    }
                }
                _ => {
                    {
                        let __w0 = self.xchr[(s) as usize];
                        crate::system::wr_char(&mut self.write_file[(self.selector) as usize], __w0);
                    }
                }
            }
            self.tally = (self.tally).wrapping_add(1i32);
        }
    }

    /// An entire string is output by calling `print`. Note that if we are outputting
    /// the single standard ASCII character \.c, we could call `print("c")`, since
    /// `"c"=99` is the number of a single-character string, as explained above. But
    /// `print_char("c")` is quicker, so \TeX\ goes directly to the `print_char`
    /// routine when it knows that this is safe. (The present implementation
    /// assumes that it is always safe to print a visible ASCII character.)
    /// @<Basic print...
    // §59
    pub fn print(&mut self, mut s: i32) {
        let mut j: pool_pointer = 0; // §59
        let mut nl: i32 = 0; // §59
        'l_exit_f: {
            if (s >= self.str_ptr) {
                s = 261i32;
            } else {
                if (s < 256i32) {
                    if (s < 0i32) {
                        s = 261i32;
                    } else {
                        {
                            if (self.selector > 20i32) {
                                {
                                    self.print_char(s);
                                    break 'l_exit_f;
                                }
                            }
                            if (s == self.eqtb[((629067i32) - 1) as usize].int()) {
                                if (self.selector < 20i32) {
                                    {
                                        self.print_ln();
                                        break 'l_exit_f;
                                    }
                                }
                            }
                            nl = self.eqtb[((629067i32) - 1) as usize].int();
                            self.eqtb[((629067i32) - 1) as usize].set_int((1i32).wrapping_neg());
                            j = self.str_start[(s) as usize];
                            while (j < self.str_start[((s).wrapping_add(1i32)) as usize]) {
                                {
                                    self.print_char(self.str_pool[(j) as usize]);
                                    j = (j).wrapping_add(1i32);
                                }
                            }
                            self.eqtb[((629067i32) - 1) as usize].set_int(nl);
                            break 'l_exit_f;
                        }
                    }
                }
            }
            j = self.str_start[(s) as usize];
            while (j < self.str_start[((s).wrapping_add(1i32)) as usize]) {
                {
                    self.print_char(self.str_pool[(j) as usize]);
                    j = (j).wrapping_add(1i32);
                }
            }
        }
    }

    /// Control sequence names, file names, and strings constructed with
    /// \.{\\string} might contain `ASCII_code` values that can't
    /// be printed using `print_char`. Therefore we use `slow_print` for them:
    /// @<Basic print...
    // §60
    pub fn slow_print(&mut self, mut s: i32) {
        let mut j: pool_pointer = 0; // §60
        if ((s >= self.str_ptr) || (s < 256i32)) {
            self.print(s);
        } else {
            {
                j = self.str_start[(s) as usize];
                while (j < self.str_start[((s).wrapping_add(1i32)) as usize]) {
                    {
                        self.print(self.str_pool[(j) as usize]);
                        j = (j).wrapping_add(1i32);
                    }
                }
            }
        }
    }

    /// The procedure `print_nl` is like `print`, but it makes sure that the
    /// string appears at the beginning of a new line.
    /// @<Basic print...
    // §62
    pub fn print_nl(&mut self, mut s: str_number) {
        if (((self.selector < 16i32) || ((self.term_offset > 0i32) && (((self.selector) % 2) != 0))) || ((self.file_offset > 0i32) && (self.selector >= 18i32))) {
            self.print_ln();
        }
        self.print(s);
    }

    /// The procedure `print_esc` prints a string that is preceded by
    /// the user's escape character (which is usually a backslash).
    /// @<Basic print...
    // §63
    pub fn print_esc(&mut self, mut s: str_number) {
        let mut c: i32 = 0; // §63
        // §261
        c = self.eqtb[((629063i32) - 1) as usize].int();
        // §63
        if (c >= 0i32) {
            if (c < 256i32) {
                self.print(c);
            }
        }
        self.slow_print(s);
    }

    /// An array of digits in the range `0..15` is printed by `print_the_digs`.
    /// @<Basic print...
    // §64
    pub fn print_the_digs(&mut self, mut k: eight_bits) {
        while (k > 0i32) {
            {
                k = (k).wrapping_sub(1i32);
                if (self.dig[(k) as usize] < 10i32) {
                    self.print_char((48i32).wrapping_add(self.dig[(k) as usize]));
                } else {
                    self.print_char((55i32).wrapping_add(self.dig[(k) as usize]));
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
    // §65
    pub fn print_int(&mut self, mut n: longinteger) {
        let mut k: i32 = 0; // §65
        let mut m: longinteger = 0; // §65
        k = 0i32;
        if (n < ((0i32) as i64)) {
            {
                self.print_char(45i32);
                if (n > (((100000000i32).wrapping_neg()) as i64)) {
                    n = (n).wrapping_neg();
                } else {
                    {
                        m = ((((1i32).wrapping_neg()) as i64)).wrapping_sub(n);
                        n = (m / ((10i32) as i64));
                        m = ((m % ((10i32) as i64))).wrapping_add(((1i32) as i64));
                        k = 1i32;
                        if (m < ((10i32) as i64)) {
                            self.dig[(0i32) as usize] = ((m) as i32);
                        } else {
                            {
                                self.dig[(0i32) as usize] = 0i32;
                                n = (n).wrapping_add(((1i32) as i64));
                            }
                        }
                    }
                }
            }
        }
        loop {
            self.dig[(k) as usize] = (((n % ((10i32) as i64))) as i32);
            n = (n / ((10i32) as i64));
            k = (k).wrapping_add(1i32);
            if (n == ((0i32) as i64)) { break; }
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
    // §284
    pub fn print_cs(&mut self, mut p: i32) {
        if (p < 514i32) {
            if (p >= 257i32) {
                if (p == 513i32) {
                    {
                        self.print_esc(580i32);
                        self.print_esc(581i32);
                        self.print_char(32i32);
                    }
                } else {
                    {
                        self.print_esc((p).wrapping_sub(257i32));
                        if (self.eqtb[((((627738i32).wrapping_add(p)).wrapping_sub(257i32)) - 1) as usize].hh().rh() == 11i32) {
                            self.print_char(32i32);
                        }
                    }
                }
            } else {
                if (p < 1i32) {
                    self.print_esc(582i32);
                } else {
                    self.print((p).wrapping_sub(1i32));
                }
            }
        } else {
            if (p >= 626627i32) {
                self.print_esc(582i32);
            } else {
                if ((self.hash[((p) - 514) as usize].rh() < 0i32) || (self.hash[((p) - 514) as usize].rh() >= self.str_ptr)) {
                    self.print_esc(583i32);
                } else {
                    {
                        if ((p >= 615526i32) && (p < 617626i32)) {
                            self.print_esc((self.prim[((p).wrapping_sub(615526i32)) as usize].rh()).wrapping_sub(1i32));
                        } else {
                            self.print_esc(self.hash[((p) - 514) as usize].rh());
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
    // §285
    pub fn sprint_cs(&mut self, mut p: halfword) {
        if (p < 514i32) {
            if (p < 257i32) {
                self.print((p).wrapping_sub(1i32));
            } else {
                if (p < 513i32) {
                    self.print_esc((p).wrapping_sub(257i32));
                } else {
                    {
                        self.print_esc(580i32);
                        self.print_esc(581i32);
                    }
                }
            }
        } else {
            if ((p >= 615526i32) && (p < 617626i32)) {
                self.print_esc((self.prim[((p).wrapping_sub(615526i32)) as usize].rh()).wrapping_sub(1i32));
            } else {
                self.print_esc(self.hash[((p) - 514) as usize].rh());
            }
        }
    }

    /// Conversely, here is a routine that takes three strings and prints a file
    /// name that might have produced them. (The routine is system dependent, because
    /// some operating systems put the file area last instead of first.)
    /// @<Basic printing...
    // §544
    pub fn print_file_name(&mut self, mut n: i32, mut a: i32, mut e: i32) {
        let mut must_quote: bool = false; // §544
        let mut j: pool_pointer = 0; // §544
        must_quote = false;
        if (a != 0i32) {
            {
                j = self.str_start[(a) as usize];
                while ((!must_quote) && (j < self.str_start[((a).wrapping_add(1i32)) as usize])) {
                    {
                        must_quote = (self.str_pool[(j) as usize] == 32i32);
                        j = (j).wrapping_add(1i32);
                    }
                }
            }
        }
        if (n != 0i32) {
            {
                j = self.str_start[(n) as usize];
                while ((!must_quote) && (j < self.str_start[((n).wrapping_add(1i32)) as usize])) {
                    {
                        must_quote = (self.str_pool[(j) as usize] == 32i32);
                        j = (j).wrapping_add(1i32);
                    }
                }
            }
        }
        if (e != 0i32) {
            {
                j = self.str_start[(e) as usize];
                while ((!must_quote) && (j < self.str_start[((e).wrapping_add(1i32)) as usize])) {
                    {
                        must_quote = (self.str_pool[(j) as usize] == 32i32);
                        j = (j).wrapping_add(1i32);
                    }
                }
            }
        }
        if must_quote {
            self.print_char(34i32);
        }
        if (a != 0i32) {
            {
                let __for_end_3 = (self.str_start[((a).wrapping_add(1i32)) as usize]).wrapping_sub(1i32);
                j = self.str_start[(a) as usize];
                while j <= __for_end_3 {
                    if (self.str_pool[(j) as usize] != 34i32) {
                        self.print(self.str_pool[(j) as usize]);
                    }
                    j = j.wrapping_add(1);
                }
            }
        }
        if (n != 0i32) {
            {
                let __for_end_3 = (self.str_start[((n).wrapping_add(1i32)) as usize]).wrapping_sub(1i32);
                j = self.str_start[(n) as usize];
                while j <= __for_end_3 {
                    if (self.str_pool[(j) as usize] != 34i32) {
                        self.print(self.str_pool[(j) as usize]);
                    }
                    j = j.wrapping_add(1);
                }
            }
        }
        if (e != 0i32) {
            {
                let __for_end_3 = (self.str_start[((e).wrapping_add(1i32)) as usize]).wrapping_sub(1i32);
                j = self.str_start[(e) as usize];
                while j <= __for_end_3 {
                    if (self.str_pool[(j) as usize] != 34i32) {
                        self.print(self.str_pool[(j) as usize]);
                    }
                    j = j.wrapping_add(1);
                }
            }
        }
        if must_quote {
            self.print_char(34i32);
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
    // §875
    pub fn print_size(&mut self, mut s: i32) {
        if (s == 0i32) {
            self.print_esc(428i32);
        } else {
            if (s == 16i32) {
                self.print_esc(429i32);
            } else {
                self.print_esc(430i32);
            }
        }
    }

    /// Each new type of node that appears in our data structure must be capable
    /// of being displayed, copied, destroyed, and so on. The routines that we
    /// need for write-oriented whatsits are somewhat like those for mark nodes;
    /// other extensions might, of course, involve more subtlety here.
    /// @<Basic printing...
    // §1602
    pub fn print_write_whatsit(&mut self, mut s: str_number, mut p: halfword) {
        self.print_esc(s);
        if (self.mem[((p).wrapping_add(1i32)) as usize].hh().lh() < 16i32) {
            self.print_int(((self.mem[((p).wrapping_add(1i32)) as usize].hh().lh()) as i64));
        } else {
            if (self.mem[((p).wrapping_add(1i32)) as usize].hh().lh() == 16i32) {
                self.print_char(42i32);
            } else {
                self.print_char(45i32);
            }
        }
    }

    /// The `print_sa_num` procedure prints the register number corresponding
    /// to an array element.
    /// @<Basic print...
    // §1822
    pub fn print_sa_num(&mut self, mut q: halfword) {
        let mut n: halfword = 0; // §1822
        if (self.mem[(q) as usize].hh().b0() < 32i32) {
            n = self.mem[((q).wrapping_add(1i32)) as usize].hh().rh();
        } else {
            {
                n = (self.mem[(q) as usize].hh().b0() % 16i32);
                q = self.mem[(q) as usize].hh().rh();
                n = (n).wrapping_add((16i32).wrapping_mul(self.mem[(q) as usize].hh().b0()));
                q = self.mem[(q) as usize].hh().rh();
                n = (n).wrapping_add((256i32).wrapping_mul((self.mem[(q) as usize].hh().b0()).wrapping_add((16i32).wrapping_mul(self.mem[(self.mem[(q) as usize].hh().rh()) as usize].hh().b0()))));
            }
        }
        self.print_int(((n) as i64));
    }

    /// A helper for printing file:line:error style messages.  Look for a
    /// filename in `full_source_filename_stack`, and if we fail to find
    /// one fall back on the non-file:line:error style.
    /// @<Basic print...
    // §1888
    pub fn print_file_line(&mut self) {
        let mut level: i32 = 0; // §1888
        level = self.in_open;
        while ((level > 0i32) && (self.full_source_filename_stack[(level) as usize] == 0i32)) {
            level = (level).wrapping_sub(1i32);
        }
        if (level == 0i32) {
            self.print_nl(264i32);
        } else {
            {
                self.print_nl(348i32);
                self.print(self.full_source_filename_stack[(level) as usize]);
                self.print(58i32);
                if (level == self.in_open) {
                    self.print_int(((self.line) as i64));
                } else {
                    self.print_int(((self.line_stack[(((level).wrapping_add(1i32)) - 1) as usize]) as i64));
                }
                self.print(650i32);
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
    // §81
    pub fn jump_out(&mut self) {
        crate::system::end_of_TEX(self);
    }

    /// Here now is the general `error` routine.
    /// @<Error hand...
    // §82
    pub fn error(&mut self) {
        let mut c: ASCII_code = 0; // §82
        let mut s1: i32 = 0; // §82
        let mut s2: i32 = 0; // §82
        let mut s3: i32 = 0; // §82
        let mut s4: i32 = 0; // §82
        'l_exit_f: {
            if (self.history < 2i32) {
                self.history = 2i32;
            }
            self.print_char(46i32);
            self.show_context();
            if self.halt_on_error_p {
                {
                    if self.halting_on_error_p {
                        self.do_final_end();
                    }
                    self.halting_on_error_p = true;
                    // §90
                    if (self.interaction > 0i32) {
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
                                self.print_nl(self.help_line[(self.help_ptr) as usize]);
                            }
                        }
                    }
                    self.print_ln();
                    if (self.interaction > 0i32) {
                        self.selector = (self.selector).wrapping_add(1i32);
                    }
                    self.print_ln();
                    // §82
                    self.history = 3i32;
                    self.jump_out();
                }
            }
            if (self.interaction == 3i32) {
                // §83
                while true {
                    {
                        'l_continue_b: loop {
                            if (self.interaction != 3i32) {
                                break 'l_exit_f;
                            }
                            self.clear_for_error_prompt();
                            {
                                self.print(266i32);
                                self.term_input();
                            }
                            if (self.last == self.first) {
                                break 'l_exit_f;
                            }
                            c = self.buffer[(self.first) as usize];
                            if (c >= 97i32) {
                                c = (c).wrapping_sub(32i32);
                            }
                            // §84
                            match c {
                                48 | 49 | 50 | 51 | 52 | 53 | 54 | 55 | 56 | 57 => {
                                    if self.deletions_allowed {
                                        // §88
                                        {
                                            s1 = self.cur_tok;
                                            s2 = self.cur_cmd;
                                            s3 = self.cur_chr;
                                            s4 = self.align_state;
                                            self.align_state = 1000000i32;
                                            self.OK_to_interrupt = false;
                                            if (((self.last > (self.first).wrapping_add(1i32)) && (self.buffer[((self.first).wrapping_add(1i32)) as usize] >= 48i32)) && (self.buffer[((self.first).wrapping_add(1i32)) as usize] <= 57i32)) {
                                                c = (((c).wrapping_mul(10i32)).wrapping_add(self.buffer[((self.first).wrapping_add(1i32)) as usize])).wrapping_sub((48i32).wrapping_mul(11i32));
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
                                                self.help_line[(1i32) as usize] = 281i32;
                                                self.help_line[(0i32) as usize] = 282i32;
                                            }
                                            self.show_context();
                                            continue 'l_continue_b;
                                        }
                                    }
                                }
                                69 => {
                                    // §84
                                    if (self.base_ptr > 0i32) {
                                        if (self.input_stack[(self.base_ptr) as usize].name_field >= 256i32) {
                                            {
                                                self.print_nl(267i32);
                                                self.slow_print(self.input_stack[(self.base_ptr) as usize].name_field);
                                                self.print(268i32);
                                                self.print_int(((self.line) as i64));
                                                self.interaction = 2i32;
                                                self.jump_out();
                                            }
                                        }
                                    }
                                }
                                72 => {
                                    // §89
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
                                                        self.help_line[(1i32) as usize] = 283i32;
                                                        self.help_line[(0i32) as usize] = 284i32;
                                                    }
                                                }
                                                loop {
                                                    self.help_ptr = (self.help_ptr).wrapping_sub(1i32);
                                                    self.print(self.help_line[(self.help_ptr) as usize]);
                                                    self.print_ln();
                                                    if (self.help_ptr == 0i32) { break; }
                                                }
                                            }
                                        }
                                        {
                                            self.help_ptr = 4i32;
                                            self.help_line[(3i32) as usize] = 285i32;
                                            self.help_line[(2i32) as usize] = 284i32;
                                            self.help_line[(1i32) as usize] = 286i32;
                                            self.help_line[(0i32) as usize] = 287i32;
                                        }
                                        continue 'l_continue_b;
                                    }
                                }
                                73 => {
                                    // §87
                                    {
                                        self.begin_file_reading();
                                        if (self.last > (self.first).wrapping_add(1i32)) {
                                            {
                                                self.cur_input.loc_field = (self.first).wrapping_add(1i32);
                                                self.buffer[(self.first) as usize] = 32i32;
                                            }
                                        } else {
                                            {
                                                {
                                                    self.print(280i32);
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
                                    // §86
                                    {
                                        self.error_count = 0i32;
                                        self.interaction = ((0i32).wrapping_add(c)).wrapping_sub(81i32);
                                        self.print(275i32);
                                        match c {
                                            81 => {
                                                {
                                                    self.print_esc(276i32);
                                                    self.selector = (self.selector).wrapping_sub(1i32);
                                                }
                                            }
                                            82 => {
                                                self.print_esc(277i32);
                                            }
                                            83 => {
                                                self.print_esc(278i32);
                                            }
                                            _ => {}
                                        }
                                        self.print(279i32);
                                        self.print_ln();
                                        crate::system::break_out(&mut self.term_out);
                                        break 'l_exit_f;
                                    }
                                }
                                88 => {
                                    // §84
                                    {
                                        self.interaction = 2i32;
                                        self.jump_out();
                                    }
                                }
                                _ => {
                                }
                            }
                            // §85
                            {
                                self.print(269i32);
                                self.print_nl(270i32);
                                self.print_nl(271i32);
                                if (self.base_ptr > 0i32) {
                                    if (self.input_stack[(self.base_ptr) as usize].name_field >= 256i32) {
                                        self.print(272i32);
                                    }
                                }
                                if self.deletions_allowed {
                                    self.print_nl(273i32);
                                }
                                self.print_nl(274i32);
                            }
                            break 'l_continue_b;
                        }
                    }
                }
            }
            // §82
            self.error_count = (self.error_count).wrapping_add(1i32);
            if (self.error_count == 100i32) {
                {
                    self.print_nl(265i32);
                    self.history = 3i32;
                    self.jump_out();
                }
            }
            // §90
            if (self.interaction > 0i32) {
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
                        self.print_nl(self.help_line[(self.help_ptr) as usize]);
                    }
                }
            }
            self.print_ln();
            if (self.interaction > 0i32) {
                self.selector = (self.selector).wrapping_add(1i32);
            }
            self.print_ln();
        }
        // §82
    }

    /// The following procedure prints \TeX's last words before dying.
    // §93
    pub fn fatal_error(&mut self, mut s: str_number) {
        self.normalize_selector();
        {
            if (self.interaction == 3i32) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(264i32);
            }
            self.print(289i32);
        }
        {
            self.help_ptr = 1i32;
            self.help_line[(0i32) as usize] = s;
        }
        {
            if (self.interaction == 3i32) {
                self.interaction = 2i32;
            }
            if self.log_opened {
                self.error();
            }
            self.history = 3i32;
            self.jump_out();
        }
    }

    /// Here is the most dreaded error message.
    /// @<Error hand...
    // §94
    pub fn overflow(&mut self, mut s: str_number, mut n: i32) {
        self.normalize_selector();
        {
            if (self.interaction == 3i32) {
            }
            if self.file_line_error_style_p {
                self.print_file_line();
            } else {
                self.print_nl(264i32);
            }
            self.print(290i32);
        }
        self.print(s);
        self.print_char(61i32);
        self.print_int(((n) as i64));
        self.print_char(93i32);
        {
            self.help_ptr = 2i32;
            self.help_line[(1i32) as usize] = 291i32;
            self.help_line[(0i32) as usize] = 292i32;
        }
        {
            if (self.interaction == 3i32) {
                self.interaction = 2i32;
            }
            if self.log_opened {
                self.error();
            }
            self.history = 3i32;
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
    // §95
    pub fn confusion(&mut self, mut s: str_number) {
        self.normalize_selector();
        if (self.history < 2i32) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(293i32);
                }
                self.print(s);
                self.print_char(41i32);
                {
                    self.help_ptr = 1i32;
                    self.help_line[(0i32) as usize] = 294i32;
                }
            }
        } else {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(295i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[(1i32) as usize] = 296i32;
                    self.help_line[(0i32) as usize] = 297i32;
                }
            }
        }
        {
            if (self.interaction == 3i32) {
                self.interaction = 2i32;
            }
            if self.log_opened {
                self.error();
            }
            self.history = 3i32;
            self.jump_out();
        }
    }

    /// Once a sequence of characters has been appended to `str_pool`, it
    /// officially becomes a string when the function `make_string` is called.
    /// This function returns the identification number of the new string as its
    /// value.
    // §43
    pub fn make_string(&mut self) -> str_number {
        let mut make_string: str_number = 0;
        if (self.str_ptr == max_strings) {
            self.overflow(260i32, (max_strings).wrapping_sub(self.init_str_ptr));
        }
        self.str_ptr = (self.str_ptr).wrapping_add(1i32);
        { let __ix36 = self.str_ptr; let __v37 = self.pool_ptr; self.str_start[(__ix36) as usize] = __v37; }
        make_string = (self.str_ptr).wrapping_sub(1i32);
        make_string
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
            j = self.str_start[(s) as usize];
            while (j < self.str_start[((s).wrapping_add(1i32)) as usize]) {
                {
                    if (self.str_pool[(j) as usize] != self.buffer[(k) as usize]) {
                        {
                            result = false;
                            break 'l_not_found_f;
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
            if ((self.str_start[((s).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(s) as usize]) != (self.str_start[((t).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(t) as usize])) {
                break 'l_not_found_f;
            }
            j = self.str_start[(s) as usize];
            k = self.str_start[(t) as usize];
            while (j < self.str_start[((s).wrapping_add(1i32)) as usize]) {
                {
                    if (self.str_pool[(j) as usize] != self.str_pool[(k) as usize]) {
                        break 'l_not_found_f;
                    }
                    j = (j).wrapping_add(1i32);
                    k = (k).wrapping_add(1i32);
                }
            }
            result = true;
        }
        str_eq_str = result;
        str_eq_str
    }

    /// The initial values of `str_pool`, `str_start`, `pool_ptr`,
    /// and `str_ptr` are computed by the \.{INITEX} program, based in part
    /// on the information that \.{WEB} has output while processing \TeX.
    // §47
    pub fn get_strings_started(&mut self) -> bool {
        let mut get_strings_started: bool = false;
        let mut k: i32 = 0; // §47
        let mut l: i32 = 0; // §47
        let mut m: u8 = 0; // §47
        let mut n: u8 = 0; // §47
        let mut g: str_number = 0; // §47
        let mut a: i32 = 0; // §47
        let mut c: bool = false; // §47
        'l_exit_f: {
            self.pool_ptr = 0i32;
            self.str_ptr = 0i32;
            self.str_start[(0i32) as usize] = 0i32;
            // §48
            {
                let __for_end_3 = 255i32;
                k = 0i32;
                while k <= __for_end_3 {
                    {
                        if (!self.xprn[(k) as usize]) {
                            {
                                {
                                    self.str_pool[(self.pool_ptr) as usize] = 94i32;
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                }
                                {
                                    self.str_pool[(self.pool_ptr) as usize] = 94i32;
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                }
                                if (k < 64i32) {
                                    {
                                        self.str_pool[(self.pool_ptr) as usize] = (k).wrapping_add(64i32);
                                        self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                    }
                                } else {
                                    if (k < 128i32) {
                                        {
                                            self.str_pool[(self.pool_ptr) as usize] = (k).wrapping_sub(64i32);
                                            self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                        }
                                    } else {
                                        {
                                            l = (k / 16i32);
                                            if (l < 10i32) {
                                                {
                                                    self.str_pool[(self.pool_ptr) as usize] = (l).wrapping_add(48i32);
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                }
                                            } else {
                                                {
                                                    self.str_pool[(self.pool_ptr) as usize] = (l).wrapping_add(87i32);
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                }
                                            }
                                            l = (k % 16i32);
                                            if (l < 10i32) {
                                                {
                                                    self.str_pool[(self.pool_ptr) as usize] = (l).wrapping_add(48i32);
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                }
                                            } else {
                                                {
                                                    self.str_pool[(self.pool_ptr) as usize] = (l).wrapping_add(87i32);
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            {
                                self.str_pool[(self.pool_ptr) as usize] = k;
                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                            }
                        }
                        g = self.make_string();
                    }
                    k = k.wrapping_add(1);
                }
            }
            // §51
            crate::system::copy_str(&mut self.name_of_file, pool_name);
            if { let mut __f0 = ::core::mem::take(&mut self.pool_file); let __r = self.a_open_in(&mut __f0); self.pool_file = __f0; __r } {
                {
                    c = false;
                    loop {
                        // §52
                        {
                            if crate::system::eof(&self.pool_file) {
                                {
                                    {
                                        crate::system::wr_str(&mut self.term_out, "! TEX.POOL has no check sum.");
                                        crate::system::wr_ln(&mut self.term_out);
                                    }
                                    { let mut __f0 = ::core::mem::take(&mut self.pool_file); let __r = self.a_close(&mut __f0); self.pool_file = __f0; __r };
                                    get_strings_started = false;
                                    break 'l_exit_f;
                                }
                            }
                            m = crate::system::read_char(&mut self.pool_file);
                            n = crate::system::read_char(&mut self.pool_file);
                            if (m == b'*') {
                                // §53
                                {
                                    'l_done_f: {
                                        a = 0i32;
                                        k = 1i32;
                                        while true {
                                            {
                                                if ((self.xord[(n) as usize] < 48i32) || (self.xord[(n) as usize] > 57i32)) {
                                                    {
                                                        {
                                                            crate::system::wr_str(&mut self.term_out, "! TEX.POOL check sum doesn't have nine digits.");
                                                            crate::system::wr_ln(&mut self.term_out);
                                                        }
                                                        { let mut __f0 = ::core::mem::take(&mut self.pool_file); let __r = self.a_close(&mut __f0); self.pool_file = __f0; __r };
                                                        get_strings_started = false;
                                                        break 'l_exit_f;
                                                    }
                                                }
                                                a = (((10i32).wrapping_mul(a)).wrapping_add(self.xord[(n) as usize])).wrapping_sub(48i32);
                                                if (k == 9i32) {
                                                    break 'l_done_f;
                                                }
                                                k = (k).wrapping_add(1i32);
                                                n = crate::system::read_char(&mut self.pool_file);
                                            }
                                        }
                                    }
                                    if (a != 399034618i32) {
                                        {
                                            {
                                                crate::system::wr_str(&mut self.term_out, "! TEX.POOL doesn't match; TANGLE me again.");
                                                crate::system::wr_ln(&mut self.term_out);
                                            }
                                            { let mut __f0 = ::core::mem::take(&mut self.pool_file); let __r = self.a_close(&mut __f0); self.pool_file = __f0; __r };
                                            get_strings_started = false;
                                            break 'l_exit_f;
                                        }
                                    }
                                    c = true;
                                }
                            } else {
                                // §52
                                {
                                    if ((((self.xord[(m) as usize] < 48i32) || (self.xord[(m) as usize] > 57i32)) || (self.xord[(n) as usize] < 48i32)) || (self.xord[(n) as usize] > 57i32)) {
                                        {
                                            {
                                                crate::system::wr_str(&mut self.term_out, "! TEX.POOL line doesn't begin with two digits.");
                                                crate::system::wr_ln(&mut self.term_out);
                                            }
                                            { let mut __f0 = ::core::mem::take(&mut self.pool_file); let __r = self.a_close(&mut __f0); self.pool_file = __f0; __r };
                                            get_strings_started = false;
                                            break 'l_exit_f;
                                        }
                                    }
                                    l = (((self.xord[(m) as usize]).wrapping_mul(10i32)).wrapping_add(self.xord[(n) as usize])).wrapping_sub((48i32).wrapping_mul(11i32));
                                    if (((self.pool_ptr).wrapping_add(l)).wrapping_add(string_vacancies) > pool_size) {
                                        {
                                            {
                                                crate::system::wr_str(&mut self.term_out, "! You have to increase POOLSIZE.");
                                                crate::system::wr_ln(&mut self.term_out);
                                            }
                                            { let mut __f0 = ::core::mem::take(&mut self.pool_file); let __r = self.a_close(&mut __f0); self.pool_file = __f0; __r };
                                            get_strings_started = false;
                                            break 'l_exit_f;
                                        }
                                    }
                                    {
                                        let __for_end_9 = l;
                                        k = 1i32;
                                        while k <= __for_end_9 {
                                            {
                                                if crate::system::eoln(&self.pool_file) {
                                                    m = b' ';
                                                } else {
                                                    m = crate::system::read_char(&mut self.pool_file);
                                                }
                                                {
                                                    { let __ix38 = self.pool_ptr; let __v39 = self.xord[(m) as usize]; self.str_pool[(__ix38) as usize] = __v39; }
                                                    self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                                }
                                            }
                                            k = k.wrapping_add(1);
                                        }
                                    }
                                    crate::system::read_ln(&mut self.pool_file);
                                    g = self.make_string();
                                }
                            }
                        }
                        if c { break; }
                    }
                    // §51
                    { let mut __f0 = ::core::mem::take(&mut self.pool_file); let __r = self.a_close(&mut __f0); self.pool_file = __f0; __r };
                    get_strings_started = true;
                }
            } else {
                {
                    {
                        crate::system::wr_str(&mut self.term_out, "! I can't read TEX.POOL.");
                        crate::system::wr_ln(&mut self.term_out);
                    }
                    { let mut __f0 = ::core::mem::take(&mut self.pool_file); let __r = self.a_close(&mut __f0); self.pool_file = __f0; __r };
                    get_strings_started = false;
                    break 'l_exit_f;
                }
            }
        }
        // §47
        get_strings_started
    }

    /// Here is a trivial procedure to print two digits; it is usually called with
    /// a parameter in the range `0<=n<=99`.
    // §66
    pub fn print_two(&mut self, mut n: i32) {
        n = ((n).wrapping_abs() % 100i32);
        self.print_char((48i32).wrapping_add((n / 10i32)));
        self.print_char((48i32).wrapping_add((n % 10i32)));
    }

    /// Hexadecimal printing of nonnegative integers is accomplished by `print_hex`.
    // §67
    pub fn print_hex(&mut self, mut n: i32) {
        let mut k: i32 = 0; // §67
        k = 0i32;
        self.print_char(34i32);
        loop {
            self.dig[(k) as usize] = (n % 16i32);
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
    // §69
    pub fn print_roman_int(&mut self, mut n: i32) {
        let mut j: pool_pointer = 0; // §69
        let mut k: pool_pointer = 0; // §69
        let mut u: nonnegative_integer = 0; // §69
        let mut v: nonnegative_integer = 0; // §69
        'l_exit_f: {
            j = self.str_start[(262i32) as usize];
            v = 1000i32;
            while true {
                {
                    while (n >= v) {
                        {
                            self.print_char(self.str_pool[(j) as usize]);
                            n = (n).wrapping_sub(v);
                        }
                    }
                    if (n <= 0i32) {
                        break 'l_exit_f;
                    }
                    k = (j).wrapping_add(2i32);
                    u = (v / (self.str_pool[((k).wrapping_sub(1i32)) as usize]).wrapping_sub(48i32));
                    if (self.str_pool[((k).wrapping_sub(1i32)) as usize] == 50i32) {
                        {
                            k = (k).wrapping_add(2i32);
                            u = (u / (self.str_pool[((k).wrapping_sub(1i32)) as usize]).wrapping_sub(48i32));
                        }
                    }
                    if ((n).wrapping_add(u) >= v) {
                        {
                            self.print_char(self.str_pool[(k) as usize]);
                            n = (n).wrapping_add(u);
                        }
                    } else {
                        {
                            j = (j).wrapping_add(2i32);
                            v = (v / (self.str_pool[((j).wrapping_sub(1i32)) as usize]).wrapping_sub(48i32));
                        }
                    }
                }
            }
        }
    }

    /// The `print` subroutine will not print a string that is still being
    /// created. The following procedure will.
    // §70
    pub fn print_current_string(&mut self) {
        let mut j: pool_pointer = 0; // §70
        j = self.str_start[(self.str_ptr) as usize];
        while (j < self.pool_ptr) {
            {
                self.print_char(self.str_pool[(j) as usize]);
                j = (j).wrapping_add(1i32);
            }
        }
    }

    /// Here is a procedure that asks the user to type a line of input,
    /// assuming that the `selector` setting is either `term_only` or `term_and_log`.
    /// The input is placed into locations `first` through `last-1` of the
    /// `buffer` array, and echoed on the transcript file if appropriate.
    /// This procedure is never called when `interaction<scroll_mode`.
    // §71
    pub fn term_input(&mut self) {
        let mut k: i32 = 0; // §71
        crate::system::break_out(&mut self.term_out);
        if (!{ let mut __f0 = ::core::mem::take(&mut self.term_in); let __r = self.input_ln(&mut __f0, true); self.term_in = __f0; __r }) {
            {
                self.cur_input.limit_field = 0i32;
                self.fatal_error(263i32);
            }
        }
        self.term_offset = 0i32;
        self.selector = (self.selector).wrapping_sub(1i32);
        if (self.last != self.first) {
            {
                let __for_end_3 = (self.last).wrapping_sub(1i32);
                k = self.first;
                while k <= __for_end_3 {
                    self.print(self.buffer[(k) as usize]);
                    k = k.wrapping_add(1);
                }
            }
        }
        self.print_ln();
        self.selector = (self.selector).wrapping_add(1i32);
    }

    /// A dozen or so error messages end with a parenthesized integer, so we
    /// save a teeny bit of program space by declaring the following procedure:
    // §91
    pub fn int_error(&mut self, mut n: i32) {
        self.print(288i32);
        self.print_int(((n) as i64));
        self.print_char(41i32);
        self.error();
    }

    /// In anomalous cases, the print selector might be in an unknown state;
    /// the following subroutine is called to fix things just enough to keep
    /// running a bit longer.
    // §92
    pub fn normalize_selector(&mut self) {
        if self.log_opened {
            self.selector = 19i32;
        } else {
            self.selector = 17i32;
        }
        if (self.job_name == 0i32) {
            self.open_log_file();
        }
        if (self.interaction == 0i32) {
            self.selector = (self.selector).wrapping_sub(1i32);
        }
    }

    /// When an interrupt has been detected, the program goes into its
    /// highest interaction level and lets the user have nearly the full flexibility of
    /// the `error` routine.  \TeX\ checks for interrupts only at times when it is
    /// safe to do this.
    // §98
    pub fn pause_for_instructions(&mut self) {
        if self.OK_to_interrupt {
            {
                self.interaction = 3i32;
                if ((self.selector == 18i32) || (self.selector == 16i32)) {
                    self.selector = (self.selector).wrapping_add(1i32);
                }
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(298i32);
                }
                {
                    self.help_ptr = 3i32;
                    self.help_line[(2i32) as usize] = 299i32;
                    self.help_line[(1i32) as usize] = 300i32;
                    self.help_line[(0i32) as usize] = 301i32;
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
    // §100
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
    // §102
    pub fn round_decimals(&mut self, mut k: small_number) -> scaled {
        let mut round_decimals: scaled = 0;
        let mut a: i32 = 0; // §102
        a = 0i32;
        while (k > 0i32) {
            {
                k = (k).wrapping_sub(1i32);
                a = ((a).wrapping_add((self.dig[(k) as usize]).wrapping_mul(131072i32)) / 10i32);
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
    // §103
    pub fn print_scaled(&mut self, mut s: scaled) {
        let mut delta: scaled = 0; // §103
        if (s < 0i32) {
            {
                self.print_char(45i32);
                s = (s).wrapping_neg();
            }
        }
        self.print_int((((s / 65536i32)) as i64));
        self.print_char(46i32);
        s = ((10i32).wrapping_mul((s % 65536i32))).wrapping_add(5i32);
        delta = 10i32;
        loop {
            if (delta > 65536i32) {
                s = (s).wrapping_sub(17232i32);
            }
            self.print_char((48i32).wrapping_add((s / 65536i32)));
            s = (10i32).wrapping_mul((s % 65536i32));
            delta = (delta).wrapping_mul(10i32);
            if (s <= delta) { break; }
        }
    }

    /// The first arithmetical subroutine we need computes $nx+y$, where `x`
    /// and~`y` are `scaled` and `n` is an integer. We will also use it to
    /// multiply integers.
    // §105
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
    // §106
    pub fn x_over_n(&mut self, mut x: scaled, mut n: i32) -> scaled {
        let mut x_over_n: scaled = 0;
        let mut negative: bool = false; // §106
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
    // §107
    pub fn xn_over_d(&mut self, mut x: scaled, mut n: i32, mut d: i32) -> scaled {
        let mut xn_over_d: scaled = 0;
        let mut positive: bool = false; // §107
        let mut t: nonnegative_integer = 0; // §107
        let mut u: nonnegative_integer = 0; // §107
        let mut v: nonnegative_integer = 0; // §107
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
    // §108
    pub fn badness(&mut self, mut t: scaled, mut s: scaled) -> halfword {
        let mut badness: halfword = 0;
        let mut r: i32 = 0; // §108
        if (t == 0i32) {
            badness = 0i32;
        } else {
            if (s <= 0i32) {
                badness = 10000i32;
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
                        badness = 10000i32;
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
    // §112
    pub fn make_frac(&mut self, mut p: i32, mut q: i32) -> i32 {
        let mut make_frac: i32 = 0;
        let mut f: i32 = 0; // §112
        let mut n: i32 = 0; // §112
        let mut negative: bool = false; // §112
        let mut be_careful: i32 = 0; // §112
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
                    make_frac = 2147483647i32;
                }
            }
        } else {
            {
                n = ((n).wrapping_sub(1i32)).wrapping_mul(268435456i32);
                // §113
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
                    if (f >= 268435456i32) { break; }
                }
                be_careful = (p).wrapping_sub(q);
                if ((be_careful).wrapping_add(p) >= 0i32) {
                    f = (f).wrapping_add(1i32);
                }
                // §112
                if negative {
                    make_frac = ((f).wrapping_add(n)).wrapping_neg();
                } else {
                    make_frac = (f).wrapping_add(n);
                }
            }
        }
        make_frac
    }

    // §114
    pub fn take_frac(&mut self, mut q: i32, mut f: i32) -> i32 {
        let mut take_frac: i32 = 0;
        let mut p: i32 = 0; // §114
        let mut negative: bool = false; // §114
        let mut n: i32 = 0; // §114
        let mut be_careful: i32 = 0; // §114
        // §115
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
        // §114
        if (f < 268435456i32) {
            n = 0i32;
        } else {
            {
                n = (f / 268435456i32);
                f = (f % 268435456i32);
                if (q <= (2147483647i32 / n)) {
                    n = (n).wrapping_mul(q);
                } else {
                    {
                        self.arith_error = true;
                        n = 2147483647i32;
                    }
                }
            }
        }
        f = (f).wrapping_add(268435456i32);
        // §116
        p = 134217728i32;
        if (q < 1073741824i32) {
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
        // §114
        be_careful = (n).wrapping_sub(2147483647i32);
        if ((be_careful).wrapping_add(p) > 0i32) {
            {
                self.arith_error = true;
                n = (2147483647i32).wrapping_sub(p);
            }
        }
        if negative {
            take_frac = ((n).wrapping_add(p)).wrapping_neg();
        } else {
            take_frac = (n).wrapping_add(p);
        }
        take_frac
    }

    // §119
    pub fn m_log(&mut self, mut x: i32) -> i32 {
        let mut m_log: i32 = 0;
        let mut y: i32 = 0; // §119
        let mut z: i32 = 0; // §119
        let mut k: i32 = 0; // §119
        if (x <= 0i32) {
            // §121
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    if self.file_line_error_style_p {
                        self.print_file_line();
                    } else {
                        self.print_nl(264i32);
                    }
                    self.print(302i32);
                }
                self.print_scaled(x);
                self.print(303i32);
                {
                    self.help_ptr = 2i32;
                    self.help_line[(1i32) as usize] = 304i32;
                    self.help_line[(0i32) as usize] = 305i32;
                }
                self.error();
                m_log = 0i32;
            }
        } else {
            // §119
            {
                y = 1302456860i32;
                z = 6581195i32;
                while (x < 1073741824i32) {
                    {
                        x = (x).wrapping_add(x);
                        y = (y).wrapping_sub(93032639i32);
                        z = (z).wrapping_sub(48782i32);
                    }
                }
                y = (y).wrapping_add((z / 65536i32));
                k = 2i32;
                while (x > 1073741828i32) {
                    // §120
                    {
                        z = (((x).wrapping_sub(1i32) / self.two_to_the[(k) as usize])).wrapping_add(1i32);
                        while (x < (1073741824i32).wrapping_add(z)) {
                            {
                                z = ((z).wrapping_add(1i32) / 2i32);
                                k = (k).wrapping_add(1i32);
                            }
                        }
                        y = (y).wrapping_add(self.spec_log[((k) - 1) as usize]);
                        x = (x).wrapping_sub(z);
                    }
                }
                // §119
                m_log = (y / 8i32);
            }
        }
        m_log
    }

    /// The following somewhat different subroutine tests rigorously if $ab$ is
    /// greater than, equal to, or less than~$cd$,
    /// given integers $(a,b,c,d)$. In most cases a quick decision is reached.
    /// The result is $+1$, 0, or~$-1$ in the three respective cases.
    // §122
    pub fn ab_vs_cd(&mut self, mut a: i32, mut b: i32, mut c: i32, mut d: i32) -> i32 {
        let mut ab_vs_cd: i32 = 0;
        let mut q: i32 = 0; // §122
        let mut r: i32 = 0; // §122
        'l_exit_f: {
            // §123
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
            // §122
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
    // §124
    pub fn new_randoms(&mut self) {
        let mut k: i32 = 0; // §124
        let mut x: i32 = 0; // §124
        {
            let __for_end_2 = 23i32;
            k = 0i32;
            while k <= __for_end_2 {
                {
                    x = (self.randoms[(k) as usize]).wrapping_sub(self.randoms[((k).wrapping_add(31i32)) as usize]);
                    if (x < 0i32) {
                        x = (x).wrapping_add(268435456i32);
                    }
                    self.randoms[(k) as usize] = x;
                }
                k = k.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 54i32;
            k = 24i32;
            while k <= __for_end_2 {
                {
                    x = (self.randoms[(k) as usize]).wrapping_sub(self.randoms[((k).wrapping_sub(24i32)) as usize]);
                    if (x < 0i32) {
                        x = (x).wrapping_add(268435456i32);
                    }
                    self.randoms[(k) as usize] = x;
                }
                k = k.wrapping_add(1);
            }
        }
        self.j_random = 54i32;
    }

    /// To initialize the `randoms` table, we call the following routine.
    // §125
    pub fn init_randoms(&mut self, mut seed: i32) {
        let mut j: i32 = 0; // §125
        let mut jj: i32 = 0; // §125
        let mut k: i32 = 0; // §125
        let mut i: i32 = 0; // §125
        j = (seed).wrapping_abs();
        while (j >= 268435456i32) {
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
                    self.randoms[(((i).wrapping_mul(21i32) % 55i32)) as usize] = j;
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
    // §126
    pub fn unif_rand(&mut self, mut x: i32) -> i32 {
        let mut unif_rand: i32 = 0;
        let mut y: i32 = 0; // §126
        if (self.j_random == 0i32) {
            self.new_randoms();
        } else {
            self.j_random = (self.j_random).wrapping_sub(1i32);
        }
        y = self.take_frac((x).wrapping_abs(), self.randoms[(self.j_random) as usize]);
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
    // §127
    pub fn norm_rand(&mut self) -> i32 {
        let mut norm_rand: i32 = 0;
        let mut x: i32 = 0; // §127
        let mut u: i32 = 0; // §127
        let mut l: i32 = 0; // §127
        loop {
            loop {
                if (self.j_random == 0i32) {
                    self.new_randoms();
                } else {
                    self.j_random = (self.j_random).wrapping_sub(1i32);
                }
                x = self.take_frac(112429i32, (self.randoms[(self.j_random) as usize]).wrapping_sub(134217728i32));
                if (self.j_random == 0i32) {
                    self.new_randoms();
                } else {
                    self.j_random = (self.j_random).wrapping_sub(1i32);
                }
                u = self.randoms[(self.j_random) as usize];
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
    // §314
    pub fn show_token_list(&mut self, mut p: i32, mut q: i32, mut l: i32) {
        let mut m: i32 = 0; // §314
        let mut c: i32 = 0; // §314
        let mut match_chr: ASCII_code = 0; // §314
        let mut n: ASCII_code = 0; // §314
        'l_exit_f: {
            match_chr = 35i32;
            n = 48i32;
            self.tally = 0i32;
            while ((p != 0i32) && (self.tally < l)) {
                {
                    if (p == q) {
                        // §342
                        {
                            self.first_count = self.tally;
                            self.trick_count = (((self.tally).wrapping_add(1i32)).wrapping_add(self.error_line)).wrapping_sub(self.half_error_line);
                            if (self.trick_count < self.error_line) {
                                self.trick_count = self.error_line;
                            }
                        }
                    }
                    // §315
                    if ((p < self.hi_mem_min) || (p > self.mem_end)) {
                        {
                            self.print_esc(316i32);
                            break 'l_exit_f;
                        }
                    }
                    if (self.mem[(p) as usize].hh().lh() >= 4095i32) {
                        self.print_cs((self.mem[(p) as usize].hh().lh()).wrapping_sub(4095i32));
                    } else {
                        {
                            m = (self.mem[(p) as usize].hh().lh() / 256i32);
                            c = (self.mem[(p) as usize].hh().lh() % 256i32);
                            if (self.mem[(p) as usize].hh().lh() < 0i32) {
                                self.print_esc(637i32);
                            } else {
                                // §316
                                match m {
                                    1 | 2 | 3 | 4 | 7 | 8 | 10 | 11 | 12 => {
                                        self.print(c);
                                    }
                                    6 => {
                                        {
                                            self.print(c);
                                            self.print(c);
                                        }
                                    }
                                    5 => {
                                        {
                                            self.print(match_chr);
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
                                    13 => {
                                        {
                                            match_chr = c;
                                            self.print(c);
                                            n = (n).wrapping_add(1i32);
                                            self.print_char(n);
                                            if (n > 57i32) {
                                                break 'l_exit_f;
                                            }
                                        }
                                    }
                                    14 => {
                                        if (c == 0i32) {
                                            self.print(638i32);
                                        }
                                    }
                                    _ => {
                                        self.print_esc(637i32);
                                    }
                                }
                            }
                        }
                    }
                    // §314
                    p = self.mem[(p) as usize].hh().rh();
                }
            }
            if (p != 0i32) {
                self.print_esc(424i32);
            }
        }
    }

    /// Here is a procedure that uses `scanner_status` to print a warning message
    /// when a subfile has ended, and at certain other crucial times:
    /// @<Declare the procedure called `runaway`
    // §328
    pub fn runaway(&mut self) {
        let mut p: halfword = 0; // §328
        if (self.scanner_status > 1i32) {
            {
                self.print_nl(652i32);
                match self.scanner_status {
                    2 => {
                        {
                            self.print(653i32);
                            p = self.def_ref;
                        }
                    }
                    3 => {
                        {
                            self.print(654i32);
                            p = 4999996i32;
                        }
                    }
                    4 => {
                        {
                            self.print(655i32);
                            p = 4999995i32;
                        }
                    }
                    5 => {
                        {
                            self.print(656i32);
                            p = self.def_ref;
                        }
                    }
                    _ => {}
                }
                self.print_char(63i32);
                self.print_ln();
                self.show_token_list(self.mem[(p) as usize].hh().rh(), 0i32, (self.error_line).wrapping_sub(10i32));
            }
        }
    }

    /// The function `get_avail` returns a pointer to a new one-word node whose
    /// `link` field is null. However, \TeX\ will halt if there is no more room left.
    /// If the available-space list is empty, i.e., if `avail=null`,
    /// we try first to increase `mem_end`. If that cannot be done, i.e., if
    /// `mem_end=mem_max`, we try to decrease `hi_mem_min`. If that cannot be
    /// done, i.e., if `hi_mem_min=lo_mem_max+1`, we have to quit.
    // §138
    pub fn get_avail(&mut self) -> halfword {
        let mut get_avail: halfword = 0;
        let mut p: halfword = 0; // §138
        p = self.avail;
        if (p != 0i32) {
            self.avail = self.mem[(self.avail) as usize].hh().rh();
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
                            self.overflow(306i32, ((mem_max).wrapping_add(1i32)).wrapping_sub(mem_min));
                        }
                    }
                }
            }
        }
        self.mem[(p) as usize].set_hh_rh(0i32);
        self.dyn_used = (self.dyn_used).wrapping_add(1i32);
        self.dl_new_node(p);
        get_avail = p;
        get_avail
    }

    /// The procedure `flush_list(p)` frees an entire linked list of
    /// one-word nodes that starts at position `p`.
    // §141
    pub fn flush_list(&mut self, mut p: halfword) {
        let mut q: halfword = 0; // §141
        let mut r: halfword = 0; // §141
        if (p != 0i32) {
            {
                r = p;
                loop {
                    q = r;
                    r = self.mem[(r) as usize].hh().rh();
                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                    if (r == 0i32) { break; }
                }
                { let __v40 = self.avail; self.mem[(q) as usize].set_hh_rh(__v40); }
                self.avail = p;
            }
        }
    }

    /// A call to `get_node` with argument `s` returns a pointer to a new node
    /// of size~`s`, which must be 2~or more. The `link` field of the first word
    /// of this new node is set to null. An overflow stop occurs if no suitable
    /// space exists.
    /// If `get_node` is called with $s=2^{30}$, it simply merges adjacent free
    /// areas and returns the value `max_halfword`.
    // §143
    pub fn get_node(&mut self, mut s: i32) -> halfword {
        let mut get_node: halfword = 0;
        let mut p: halfword = 0; // §143
        let mut q: halfword = 0; // §143
        let mut r: i32 = 0; // §143
        let mut t: i32 = 0; // §143
        // goto labels: restart, found, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                p = self.rover;
                loop {
                    // §145
                    q = (p).wrapping_add(self.mem[(p) as usize].hh().lh());
                    while (self.mem[(q) as usize].hh().rh() == 268435455i32) {
                        {
                            t = self.mem[((q).wrapping_add(1i32)) as usize].hh().rh();
                            if (q == self.rover) {
                                self.rover = t;
                            }
                            { let __v41 = self.mem[((q).wrapping_add(1i32)) as usize].hh().lh(); self.mem[((t).wrapping_add(1i32)) as usize].set_hh_lh(__v41); }
                            { let __ix42 = (self.mem[((q).wrapping_add(1i32)) as usize].hh().lh()).wrapping_add(1i32); self.mem[(__ix42) as usize].set_hh_rh(t); }
                            q = (q).wrapping_add(self.mem[(q) as usize].hh().lh());
                        }
                    }
                    r = (q).wrapping_sub(s);
                    if (r > (p).wrapping_add(1i32)) {
                        // §146
                        {
                            self.mem[(p) as usize].set_hh_lh((r).wrapping_sub(p));
                            self.rover = p;
                            { __goto_1 = 1; continue 'l_dispatch_1; }
                        }
                    }
                    // §145
                    if (r == p) {
                        if (self.mem[((p).wrapping_add(1i32)) as usize].hh().rh() != p) {
                            // §147
                            {
                                self.rover = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh();
                                t = self.mem[((p).wrapping_add(1i32)) as usize].hh().lh();
                                { let __ix43 = (self.rover).wrapping_add(1i32); self.mem[(__ix43) as usize].set_hh_lh(t); }
                                { let __v44 = self.rover; self.mem[((t).wrapping_add(1i32)) as usize].set_hh_rh(__v44); }
                                { __goto_1 = 1; continue 'l_dispatch_1; }
                            }
                        }
                    }
                    // §145
                    self.mem[(p) as usize].set_hh_lh((q).wrapping_sub(p));
                    // §143
                    p = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh();
                    if (p == self.rover) { break; }
                }
                if (s == 1073741824i32) {
                    {
                        get_node = 268435455i32;
                        { __goto_1 = 2; continue 'l_dispatch_1; }
                    }
                }
                if ((self.lo_mem_max).wrapping_add(2i32) < self.hi_mem_min) {
                    if ((self.lo_mem_max).wrapping_add(2i32) <= 268435455i32) {
                        // §144
                        {
                            if ((self.hi_mem_min).wrapping_sub(self.lo_mem_max) >= 1998i32) {
                                t = (self.lo_mem_max).wrapping_add(1000i32);
                            } else {
                                t = ((self.lo_mem_max).wrapping_add(1i32)).wrapping_add(((self.hi_mem_min).wrapping_sub(self.lo_mem_max) / 2i32));
                            }
                            p = self.mem[((self.rover).wrapping_add(1i32)) as usize].hh().lh();
                            q = self.lo_mem_max;
                            self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(q);
                            { let __ix45 = (self.rover).wrapping_add(1i32); self.mem[(__ix45) as usize].set_hh_lh(q); }
                            if (t > 268435455i32) {
                                t = 268435455i32;
                            }
                            { let __v46 = self.rover; self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(__v46); }
                            self.mem[((q).wrapping_add(1i32)) as usize].set_hh_lh(p);
                            self.mem[(q) as usize].set_hh_rh(268435455i32);
                            { let __v47 = (t).wrapping_sub(self.lo_mem_max); self.mem[(q) as usize].set_hh_lh(__v47); }
                            self.lo_mem_max = t;
                            { let __ix48 = self.lo_mem_max; self.mem[(__ix48) as usize].set_hh_rh(0i32); }
                            { let __ix49 = self.lo_mem_max; self.mem[(__ix49) as usize].set_hh_lh(0i32); }
                            self.rover = q;
                            { __goto_1 = 0; continue 'l_dispatch_1; }
                        }
                    }
                }
                // §143
                self.overflow(306i32, ((mem_max).wrapping_add(1i32)).wrapping_sub(mem_min));
            }
            if __goto_1 <= 1 { // found
                self.mem[(r) as usize].set_hh_rh(0i32);
                self.dl_new_node(r);
                self.var_used = (self.var_used).wrapping_add(s);
                get_node = r;
            }
            if __goto_1 <= 2 { // exit
            }
            break 'l_dispatch_1;
        }
        get_node
    }

    /// Conversely, when some variable-size node `p` of size `s` is no longer needed,
    /// the operation `free_node(p,s)` will make its words available, by inserting
    /// `p` as a new empty node just before where `rover` now points.
    // §148
    pub fn free_node(&mut self, mut p: halfword, mut s: halfword) {
        let mut q: halfword = 0; // §148
        self.mem[(p) as usize].set_hh_lh(s);
        self.mem[(p) as usize].set_hh_rh(268435455i32);
        q = self.mem[((self.rover).wrapping_add(1i32)) as usize].hh().lh();
        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_lh(q);
        { let __v50 = self.rover; self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(__v50); }
        { let __ix51 = (self.rover).wrapping_add(1i32); self.mem[(__ix51) as usize].set_hh_lh(p); }
        self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(p);
        self.var_used = (self.var_used).wrapping_sub(s);
    }

    /// Just before \.{INITEX} writes out the memory, it sorts the doubly linked
    /// available space list. The list is probably very short at such times, so a
    /// simple insertion sort is used. The smallest available location will be
    /// pointed to by `rover`, the next-smallest by `rlink(rover)`, etc.
    // §149
    pub fn sort_avail(&mut self) {
        let mut p: halfword = 0; // §149
        let mut q: halfword = 0; // §149
        let mut r: halfword = 0; // §149
        let mut old_rover: halfword = 0; // §149
        p = self.get_node(1073741824i32);
        p = self.mem[((self.rover).wrapping_add(1i32)) as usize].hh().rh();
        { let __ix52 = (self.rover).wrapping_add(1i32); self.mem[(__ix52) as usize].set_hh_rh(268435455i32); }
        old_rover = self.rover;
        while (p != old_rover) {
            // §150
            if (p < self.rover) {
                {
                    q = p;
                    p = self.mem[((q).wrapping_add(1i32)) as usize].hh().rh();
                    { let __v53 = self.rover; self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(__v53); }
                    self.rover = q;
                }
            } else {
                {
                    q = self.rover;
                    while (self.mem[((q).wrapping_add(1i32)) as usize].hh().rh() < p) {
                        q = self.mem[((q).wrapping_add(1i32)) as usize].hh().rh();
                    }
                    r = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh();
                    { let __v54 = self.mem[((q).wrapping_add(1i32)) as usize].hh().rh(); self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(__v54); }
                    self.mem[((q).wrapping_add(1i32)) as usize].set_hh_rh(p);
                    p = r;
                }
            }
        }
        // §149
        p = self.rover;
        while (self.mem[((p).wrapping_add(1i32)) as usize].hh().rh() != 268435455i32) {
            {
                { let __ix55 = (self.mem[((p).wrapping_add(1i32)) as usize].hh().rh()).wrapping_add(1i32); self.mem[(__ix55) as usize].set_hh_lh(p); }
                p = self.mem[((p).wrapping_add(1i32)) as usize].hh().rh();
            }
        }
        { let __v56 = self.rover; self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(__v56); }
        { let __ix57 = (self.rover).wrapping_add(1i32); self.mem[(__ix57) as usize].set_hh_lh(p); }
    }

    /// The `new_null_box` function returns a pointer to an `hlist_node` in
    /// which all subfields have the values corresponding to `\.{\\hbox\{\}}'.
    /// (The `subtype` field is set to `min_quarterword`, for historic reasons
    /// that are no longer relevant.)
    // §154
    pub fn new_null_box(&mut self) -> halfword {
        let mut new_null_box: halfword = 0;
        let mut p: halfword = 0; // §154
        p = self.get_node(7i32);
        self.mem[(p) as usize].set_hh_b0(0i32);
        self.mem[(p) as usize].set_hh_b1(0i32);
        self.mem[((p).wrapping_add(1i32)) as usize].set_int(0i32);
        self.mem[((p).wrapping_add(2i32)) as usize].set_int(0i32);
        self.mem[((p).wrapping_add(3i32)) as usize].set_int(0i32);
        self.mem[((p).wrapping_add(4i32)) as usize].set_int(0i32);
        self.mem[((p).wrapping_add(5i32)) as usize].set_hh_rh(0i32);
        self.mem[((p).wrapping_add(5i32)) as usize].set_hh_b0(0i32);
        self.mem[((p).wrapping_add(5i32)) as usize].set_hh_b1(0i32);
        self.mem[((p).wrapping_add(6i32)) as usize].set_gr(0.0f64);
        new_null_box = p;
        new_null_box
    }

    /// A new rule node is delivered by the `new_rule` function. It
    /// makes all the dimensions ``running,'' so you have to change the
    /// ones that are not allowed to run.
    // §157
    pub fn new_rule(&mut self) -> halfword {
        let mut new_rule: halfword = 0;
        let mut p: halfword = 0; // §157
        p = self.get_node(4i32);
        self.mem[(p) as usize].set_hh_b0(2i32);
        self.mem[(p) as usize].set_hh_b1(0i32);
        self.mem[((p).wrapping_add(1i32)) as usize].set_int((1073741824i32).wrapping_neg());
        self.mem[((p).wrapping_add(2i32)) as usize].set_int((1073741824i32).wrapping_neg());
        self.mem[((p).wrapping_add(3i32)) as usize].set_int((1073741824i32).wrapping_neg());
        new_rule = p;
        new_rule
    }

    /// The `new_ligature` function creates a ligature node having given
    /// contents of the `font`, `character`, and `lig_ptr` fields. We also have
    /// a `new_lig_item` function, which returns a two-word node having a given
    /// `character` field. Such nodes are used for temporary processing as ligatures
    /// are being created.
    // §162
    pub fn new_ligature(&mut self, mut f: quarterword, mut c: quarterword, mut q: halfword) -> halfword {
        let mut new_ligature: halfword = 0;
        let mut p: halfword = 0; // §162
        p = self.get_node(2i32);
        self.mem[(p) as usize].set_hh_b0(6i32);
        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_b0(f);
        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_b1(c);
        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(q);
        self.mem[(p) as usize].set_hh_b1(0i32);
        new_ligature = p;
        new_ligature
    }

    /// The `new_ligature` function creates a ligature node having given
    /// contents of the `font`, `character`, and `lig_ptr` fields. We also have
    /// a `new_lig_item` function, which returns a two-word node having a given
    /// `character` field. Such nodes are used for temporary processing as ligatures
    /// are being created.
    // §162
    pub fn new_lig_item(&mut self, mut c: quarterword) -> halfword {
        let mut new_lig_item: halfword = 0;
        let mut p: halfword = 0; // §162
        p = self.get_node(2i32);
        self.mem[(p) as usize].set_hh_b1(c);
        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(0i32);
        new_lig_item = p;
        new_lig_item
    }

    /// A `disc_node`, which occurs only in horizontal lists, specifies a
    /// ``dis\-cretion\-ary'' line break. If such a break occurs at node `p`, the text
    /// that starts at `pre_break(p)` will precede the break, the text that starts at
    /// `post_break(p)` will follow the break, and text that appears in the next
    /// `replace_count(p)` nodes will be ignored. For example, an ordinary
    /// discretionary hyphen, indicated by `\.{\\-}', yields a `disc_node` with
    /// `pre_break` pointing to a `char_node` containing a hyphen, `post_break=null`,
    /// and `replace_count=0`. All three of the discretionary texts must be
    /// lists that consist entirely of character, kern, box, rule, and ligature nodes.
    /// If `pre_break(p)=null`, the `ex_hyphen_penalty` will be charged for this
    /// break.  Otherwise the `hyphen_penalty` will be charged.  The texts will
    /// actually be substituted into the list by the line-breaking algorithm if it
    /// decides to make the break, and the discretionary node will disappear at
    /// that time; thus, the output routine sees only discretionaries that were
    /// ...
    // §163
    pub fn new_disc(&mut self) -> halfword {
        let mut new_disc: halfword = 0;
        let mut p: halfword = 0; // §163
        p = self.get_node(2i32);
        self.mem[(p) as usize].set_hh_b0(7i32);
        self.mem[(p) as usize].set_hh_b1(0i32);
        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_lh(0i32);
        self.mem[((p).wrapping_add(1i32)) as usize].set_hh_rh(0i32);
        new_disc = p;
        new_disc
    }

    /// A `math_node`, which occurs only in horizontal lists, appears before and
    /// after mathematical formulas. The `subtype` field is `before` before the
    /// formula and `after` after it. There is a `width` field, which represents
    /// the amount of surrounding space inserted by \.{\\mathsurround}.
    /// In addition a `math_node` with `subtype>after` and `width=0` will be
    /// (ab)used to record a regular `math_node` reinserted after being
    /// discarded at a line break or one of the text direction primitives (
    /// \.{\\beginL}, \.{\\endL}, \.{\\beginR}, and \.{\\endR} ).
    // §165
    pub fn new_math(&mut self, mut w: scaled, mut s: small_number) -> halfword {
        let mut new_math: halfword = 0;
        let mut p: halfword = 0; // §165
        p = self.get_node(2i32);
        self.mem[(p) as usize].set_hh_b0(9i32);
        self.mem[(p) as usize].set_hh_b1(s);
        self.mem[((p).wrapping_add(1i32)) as usize].set_int(w);
        new_math = p;
        new_math
    }

}
