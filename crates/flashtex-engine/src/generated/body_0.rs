// GENERATED FILE -- DO NOT EDIT.
// Translated WEB procedures and functions.
// Regenerate with the command in tools/web2rust/README.md.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(unused_parens, unused_mut, unused_variables, unused_assignments)]
#![allow(unused_imports, unused_labels, while_true)]
#![allow(dead_code, unreachable_code, clippy::all)]

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
        let mut k: i32 = 0; // §163
        let mut z: hyph_pointer = 0; // §927
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
                self.xchr[(i) as usize] = b' ';
                i = i.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 255i32;
            i = 127i32;
            while i <= __for_end_2 {
                self.xchr[(i) as usize] = b' ';
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
        // §74
        self.interaction = 3i32;
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
        // §215
        self.nest_ptr = 0i32;
        self.max_nest_stack = 0i32;
        self.cur_list.mode_field = 1i32;
        self.cur_list.head_field = 4999998i32;
        self.cur_list.tail_field = 4999998i32;
        self.cur_list.aux_field.set_int((65536000i32).wrapping_neg());
        self.cur_list.ml_field = 0i32;
        self.cur_list.pg_field = 0i32;
        self.shown_mode = 0i32;
        // §991
        self.page_contents = 0i32;
        self.page_tail = 4999997i32;
        self.mem[(4999997i32) as usize].set_hh_rh(0i32);
        self.last_glue = 268435455i32;
        self.last_penalty = 0i32;
        self.last_kern = 0i32;
        self.page_so_far[(7i32) as usize] = 0i32;
        self.page_max_depth = 0i32;
        // §254
        {
            let __for_end_2 = 619006i32;
            k = 618163i32;
            while k <= __for_end_2 {
                self.xeq_level[((k) - 618163) as usize] = 1i32;
                k = k.wrapping_add(1);
            }
        }
        // §257
        self.no_new_control_sequence = true;
        self.hash[((514i32) - 514) as usize].set_lh(0i32);
        self.hash[((514i32) - 514) as usize].set_rh(0i32);
        {
            let __for_end_2 = 615780i32;
            k = 515i32;
            while k <= __for_end_2 {
                { let __v0 = self.hash[((514i32) - 514) as usize]; self.hash[((k) - 514) as usize] = __v0; }
                k = k.wrapping_add(1);
            }
        }
        // §272
        self.save_ptr = 0i32;
        self.cur_level = 1i32;
        self.cur_group = 0i32;
        self.cur_boundary = 0i32;
        self.max_save_stack = 0i32;
        // §287
        self.mag_set = 0i32;
        // §383
        self.cur_mark[(0i32) as usize] = 0i32;
        self.cur_mark[(1i32) as usize] = 0i32;
        self.cur_mark[(2i32) as usize] = 0i32;
        self.cur_mark[(3i32) as usize] = 0i32;
        self.cur_mark[(4i32) as usize] = 0i32;
        // §439
        self.cur_val = 0i32;
        self.cur_val_level = 0i32;
        self.radix = 0i32;
        self.cur_order = 0i32;
        // §481
        {
            let __for_end_2 = 16i32;
            k = 0i32;
            while k <= __for_end_2 {
                self.read_open[(k) as usize] = 2i32;
                k = k.wrapping_add(1);
            }
        }
        // §490
        self.cond_ptr = 0i32;
        self.if_limit = 0i32;
        self.cur_if = 0i32;
        self.if_line = 0i32;
        // §521
        crate::system::copy_str(&mut self.TEX_format_default, "TeXformats:plain.fmt");
        // §551
        {
            let __for_end_2 = font_max;
            k = 0i32;
            while k <= __for_end_2 {
                { let __v1 = false; self.font_used[(k) as usize] = __v1; }
                k = k.wrapping_add(1);
            }
        }
        // §556
        self.null_character.set_b0(0i32);
        self.null_character.set_b1(0i32);
        self.null_character.set_b2(0i32);
        self.null_character.set_b3(0i32);
        // §593
        self.total_pages = 0i32;
        self.max_v = 0i32;
        self.max_h = 0i32;
        self.max_push = 0i32;
        self.last_bop = (1i32).wrapping_neg();
        self.doing_leaders = false;
        self.dead_cycles = 0i32;
        self.cur_s = (1i32).wrapping_neg();
        // §596
        self.half_buf = (dvi_buf_size / 2i32);
        self.dvi_limit = dvi_buf_size;
        self.dvi_ptr = 0i32;
        self.dvi_offset = 0i32;
        self.dvi_gone = 0i32;
        // §606
        self.down_ptr = 0i32;
        self.right_ptr = 0i32;
        // §648
        self.adjust_tail = 0i32;
        self.last_badness = 0i32;
        // §662
        self.pack_begin_line = 0i32;
        // §685
        self.empty_field.set_rh(0i32);
        self.empty_field.set_lh(0i32);
        self.null_delimiter.set_b0(0i32);
        self.null_delimiter.set_b1(0i32);
        self.null_delimiter.set_b2(0i32);
        self.null_delimiter.set_b3(0i32);
        // §771
        self.align_ptr = 0i32;
        self.cur_align = 0i32;
        self.cur_span = 0i32;
        self.cur_loop = 0i32;
        self.cur_head = 0i32;
        self.cur_tail = 0i32;
        // §928
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
        // §990
        self.output_active = false;
        self.insert_penalties = 0i32;
        // §1033
        self.ligature_present = false;
        self.cancel_boundary = false;
        self.lft_hit = false;
        self.rt_hit = false;
        self.ins_disc = false;
        // §1267
        self.after_token = 0i32;
        // §1282
        self.long_help_seen = false;
        // §1300
        self.format_ident = 0i32;
        // §1343
        {
            let __for_end_2 = 17i32;
            k = 0i32;
            while k <= __for_end_2 {
                { let __v2 = false; self.write_open[(k) as usize] = __v2; }
                k = k.wrapping_add(1);
            }
        }
        // §164
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
        { let __ix3 = self.rover; self.mem[(__ix3) as usize].set_hh_rh(268435455i32); }
        { let __ix4 = self.rover; self.mem[(__ix4) as usize].set_hh_lh(1000i32); }
        { let __ix5 = (self.rover).wrapping_add(1i32); let __v6 = self.rover; self.mem[(__ix5) as usize].set_hh_lh(__v6); }
        { let __ix7 = (self.rover).wrapping_add(1i32); let __v8 = self.rover; self.mem[(__ix7) as usize].set_hh_rh(__v8); }
        self.lo_mem_max = (self.rover).wrapping_add(1000i32);
        { let __ix9 = self.lo_mem_max; self.mem[(__ix9) as usize].set_hh_rh(0i32); }
        { let __ix10 = self.lo_mem_max; self.mem[(__ix10) as usize].set_hh_lh(0i32); }
        {
            let __for_end_2 = 4999999i32;
            k = 4999986i32;
            while k <= __for_end_2 {
                { let __v11 = self.mem[(self.lo_mem_max) as usize]; self.mem[(k) as usize] = __v11; }
                k = k.wrapping_add(1);
            }
        }
        // §790
        self.mem[(4999989i32) as usize].set_hh_lh(619614i32);
        // §797
        self.mem[(4999990i32) as usize].set_hh_rh(256i32);
        self.mem[(4999990i32) as usize].set_hh_lh(0i32);
        // §820
        self.mem[(4999992i32) as usize].set_hh_b0(1i32);
        self.mem[(4999993i32) as usize].set_hh_lh(268435455i32);
        self.mem[(4999992i32) as usize].set_hh_b1(0i32);
        // §981
        self.mem[(4999999i32) as usize].set_hh_b1(255i32);
        self.mem[(4999999i32) as usize].set_hh_b0(1i32);
        self.mem[(4999999i32) as usize].set_hh_rh(4999999i32);
        // §988
        self.mem[(4999997i32) as usize].set_hh_b0(10i32);
        self.mem[(4999997i32) as usize].set_hh_b1(0i32);
        // §164
        self.avail = 0i32;
        self.mem_end = 4999999i32;
        self.hi_mem_min = 4999986i32;
        self.var_used = 20i32;
        self.dyn_used = 14i32;
        // §222
        self.eqtb[((615781i32) - 1) as usize].set_hh_b0(101i32);
        self.eqtb[((615781i32) - 1) as usize].set_hh_rh(0i32);
        self.eqtb[((615781i32) - 1) as usize].set_hh_b1(0i32);
        {
            let __for_end_2 = 615780i32;
            k = 1i32;
            while k <= __for_end_2 {
                { let __v12 = self.eqtb[((615781i32) - 1) as usize]; self.eqtb[((k) - 1) as usize] = __v12; }
                k = k.wrapping_add(1);
            }
        }
        // §228
        self.eqtb[((615782i32) - 1) as usize].set_hh_rh(0i32);
        self.eqtb[((615782i32) - 1) as usize].set_hh_b1(1i32);
        self.eqtb[((615782i32) - 1) as usize].set_hh_b0(117i32);
        {
            let __for_end_2 = 616311i32;
            k = 615783i32;
            while k <= __for_end_2 {
                { let __v13 = self.eqtb[((615782i32) - 1) as usize]; self.eqtb[((k) - 1) as usize] = __v13; }
                k = k.wrapping_add(1);
            }
        }
        { let __v14 = (self.mem[(0i32) as usize].hh().rh()).wrapping_add(530i32); self.mem[(0i32) as usize].set_hh_rh(__v14); }
        // §232
        self.eqtb[((616312i32) - 1) as usize].set_hh_rh(0i32);
        self.eqtb[((616312i32) - 1) as usize].set_hh_b0(118i32);
        self.eqtb[((616312i32) - 1) as usize].set_hh_b1(1i32);
        {
            let __for_end_2 = 616577i32;
            k = 616313i32;
            while k <= __for_end_2 {
                { let __v15 = self.eqtb[((615781i32) - 1) as usize]; self.eqtb[((k) - 1) as usize] = __v15; }
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[((616578i32) - 1) as usize].set_hh_rh(0i32);
        self.eqtb[((616578i32) - 1) as usize].set_hh_b0(119i32);
        self.eqtb[((616578i32) - 1) as usize].set_hh_b1(1i32);
        {
            let __for_end_2 = 616833i32;
            k = 616579i32;
            while k <= __for_end_2 {
                { let __v16 = self.eqtb[((616578i32) - 1) as usize]; self.eqtb[((k) - 1) as usize] = __v16; }
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[((616834i32) - 1) as usize].set_hh_rh(0i32);
        self.eqtb[((616834i32) - 1) as usize].set_hh_b0(120i32);
        self.eqtb[((616834i32) - 1) as usize].set_hh_b1(1i32);
        {
            let __for_end_2 = 616882i32;
            k = 616835i32;
            while k <= __for_end_2 {
                { let __v17 = self.eqtb[((616834i32) - 1) as usize]; self.eqtb[((k) - 1) as usize] = __v17; }
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[((616883i32) - 1) as usize].set_hh_rh(0i32);
        self.eqtb[((616883i32) - 1) as usize].set_hh_b0(120i32);
        self.eqtb[((616883i32) - 1) as usize].set_hh_b1(1i32);
        {
            let __for_end_2 = 618162i32;
            k = 616884i32;
            while k <= __for_end_2 {
                { let __v18 = self.eqtb[((616883i32) - 1) as usize]; self.eqtb[((k) - 1) as usize] = __v18; }
                k = k.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 255i32;
            k = 0i32;
            while k <= __for_end_2 {
                {
                    self.eqtb[(((616883i32).wrapping_add(k)) - 1) as usize].set_hh_rh(12i32);
                    self.eqtb[(((617907i32).wrapping_add(k)) - 1) as usize].set_hh_rh((k).wrapping_add(0i32));
                    self.eqtb[(((617651i32).wrapping_add(k)) - 1) as usize].set_hh_rh(1000i32);
                }
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[((616896i32) - 1) as usize].set_hh_rh(5i32);
        self.eqtb[((616915i32) - 1) as usize].set_hh_rh(10i32);
        self.eqtb[((616975i32) - 1) as usize].set_hh_rh(0i32);
        self.eqtb[((616920i32) - 1) as usize].set_hh_rh(14i32);
        self.eqtb[((617010i32) - 1) as usize].set_hh_rh(15i32);
        self.eqtb[((616883i32) - 1) as usize].set_hh_rh(9i32);
        {
            let __for_end_2 = 57i32;
            k = 48i32;
            while k <= __for_end_2 {
                self.eqtb[(((617907i32).wrapping_add(k)) - 1) as usize].set_hh_rh((k).wrapping_add(28672i32));
                k = k.wrapping_add(1);
            }
        }
        {
            let __for_end_2 = 90i32;
            k = 65i32;
            while k <= __for_end_2 {
                {
                    self.eqtb[(((616883i32).wrapping_add(k)) - 1) as usize].set_hh_rh(11i32);
                    self.eqtb[((((616883i32).wrapping_add(k)).wrapping_add(32i32)) - 1) as usize].set_hh_rh(11i32);
                    self.eqtb[(((617907i32).wrapping_add(k)) - 1) as usize].set_hh_rh((k).wrapping_add(28928i32));
                    self.eqtb[((((617907i32).wrapping_add(k)).wrapping_add(32i32)) - 1) as usize].set_hh_rh((k).wrapping_add(28960i32));
                    self.eqtb[(((617139i32).wrapping_add(k)) - 1) as usize].set_hh_rh((k).wrapping_add(32i32));
                    self.eqtb[((((617139i32).wrapping_add(k)).wrapping_add(32i32)) - 1) as usize].set_hh_rh((k).wrapping_add(32i32));
                    self.eqtb[(((617395i32).wrapping_add(k)) - 1) as usize].set_hh_rh(k);
                    self.eqtb[((((617395i32).wrapping_add(k)).wrapping_add(32i32)) - 1) as usize].set_hh_rh(k);
                    self.eqtb[(((617651i32).wrapping_add(k)) - 1) as usize].set_hh_rh(999i32);
                }
                k = k.wrapping_add(1);
            }
        }
        // §240
        {
            let __for_end_2 = 618473i32;
            k = 618163i32;
            while k <= __for_end_2 {
                self.eqtb[((k) - 1) as usize].set_int(0i32);
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[((618180i32) - 1) as usize].set_int(1000i32);
        self.eqtb[((618164i32) - 1) as usize].set_int(10000i32);
        self.eqtb[((618204i32) - 1) as usize].set_int(1i32);
        self.eqtb[((618203i32) - 1) as usize].set_int(25i32);
        self.eqtb[((618208i32) - 1) as usize].set_int(92i32);
        self.eqtb[((618211i32) - 1) as usize].set_int(13i32);
        {
            let __for_end_2 = 255i32;
            k = 0i32;
            while k <= __for_end_2 {
                self.eqtb[(((618474i32).wrapping_add(k)) - 1) as usize].set_int((1i32).wrapping_neg());
                k = k.wrapping_add(1);
            }
        }
        self.eqtb[((618520i32) - 1) as usize].set_int(0i32);
        // §250
        {
            let __for_end_2 = 619006i32;
            k = 618730i32;
            while k <= __for_end_2 {
                self.eqtb[((k) - 1) as usize].set_int(0i32);
                k = k.wrapping_add(1);
            }
        }
        // §258
        self.hash_used = 615514i32;
        self.cs_count = 0i32;
        self.eqtb[((615523i32) - 1) as usize].set_hh_b0(116i32);
        self.hash[((615523i32) - 514) as usize].set_rh(502i32);
        // §552
        self.font_ptr = 0i32;
        self.fmem_ptr = 7i32;
        self.font_name[(0i32) as usize] = 801i32;
        self.font_area[(0i32) as usize] = 338i32;
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
        // §946
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
        // §951
        self.trie_not_ready = true;
        self.trie_l[(0i32) as usize] = 0i32;
        self.trie_c[(0i32) as usize] = 0i32;
        self.trie_ptr = 0i32;
        // §1216
        self.hash[((615514i32) - 514) as usize].set_rh(1190i32);
        // §1301
        self.format_ident = 1257i32;
        // §1369
        self.hash[((615522i32) - 514) as usize].set_rh(1296i32);
        self.eqtb[((615522i32) - 1) as usize].set_hh_b1(1i32);
        self.eqtb[((615522i32) - 1) as usize].set_hh_b0(113i32);
        self.eqtb[((615522i32) - 1) as usize].set_hh_rh(0i32);
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
            if (s == self.eqtb[((618212i32) - 1) as usize].int()) {
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
                        if (self.term_offset == max_print_line) {
                            {
                                {
                                    crate::system::wr_ln(&mut self.term_out);
                                }
                                self.term_offset = 0i32;
                            }
                        }
                        if (self.file_offset == max_print_line) {
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
                        if (self.file_offset == max_print_line) {
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
                        if (self.term_offset == max_print_line) {
                            self.print_ln();
                        }
                    }
                }
                16 => {
                }
                20 => {
                    if (self.tally < self.trick_count) {
                        self.trick_buf[((self.tally % error_line)) as usize] = s;
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
                s = 259i32;
            } else {
                if (s < 256i32) {
                    if (s < 0i32) {
                        s = 259i32;
                    } else {
                        {
                            if (self.selector > 20i32) {
                                {
                                    self.print_char(s);
                                    break 'l_exit_f;
                                }
                            }
                            if (s == self.eqtb[((618212i32) - 1) as usize].int()) {
                                if (self.selector < 20i32) {
                                    {
                                        self.print_ln();
                                        break 'l_exit_f;
                                    }
                                }
                            }
                            nl = self.eqtb[((618212i32) - 1) as usize].int();
                            self.eqtb[((618212i32) - 1) as usize].set_int((1i32).wrapping_neg());
                            j = self.str_start[(s) as usize];
                            while (j < self.str_start[((s).wrapping_add(1i32)) as usize]) {
                                {
                                    self.print_char(self.str_pool[(j) as usize]);
                                    j = (j).wrapping_add(1i32);
                                }
                            }
                            self.eqtb[((618212i32) - 1) as usize].set_int(nl);
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
        if (((self.term_offset > 0i32) && (((self.selector) % 2) != 0)) || ((self.file_offset > 0i32) && (self.selector >= 18i32))) {
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
        // §243
        c = self.eqtb[((618208i32) - 1) as usize].int();
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
    pub fn print_int(&mut self, mut n: i32) {
        let mut k: i32 = 0; // §65
        let mut m: i32 = 0; // §65
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
                            self.dig[(0i32) as usize] = m;
                        } else {
                            {
                                self.dig[(0i32) as usize] = 0i32;
                                n = (n).wrapping_add(1i32);
                            }
                        }
                    }
                }
            }
        }
        loop {
            self.dig[(k) as usize] = (n % 10i32);
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
    // §262
    pub fn print_cs(&mut self, mut p: i32) {
        if (p < 514i32) {
            if (p >= 257i32) {
                if (p == 513i32) {
                    {
                        self.print_esc(504i32);
                        self.print_esc(505i32);
                        self.print_char(32i32);
                    }
                } else {
                    {
                        self.print_esc((p).wrapping_sub(257i32));
                        if (self.eqtb[((((616883i32).wrapping_add(p)).wrapping_sub(257i32)) - 1) as usize].hh().rh() == 11i32) {
                            self.print_char(32i32);
                        }
                    }
                }
            } else {
                if (p < 1i32) {
                    self.print_esc(506i32);
                } else {
                    self.print((p).wrapping_sub(1i32));
                }
            }
        } else {
            if (p >= 615781i32) {
                self.print_esc(506i32);
            } else {
                if ((self.hash[((p) - 514) as usize].rh() < 0i32) || (self.hash[((p) - 514) as usize].rh() >= self.str_ptr)) {
                    self.print_esc(507i32);
                } else {
                    {
                        self.print_esc(self.hash[((p) - 514) as usize].rh());
                        self.print_char(32i32);
                    }
                }
            }
        }
    }

    /// Here is a similar procedure; it avoids the error checks, and it never
    /// prints a space after the control sequence.
    /// @<Basic printing procedures
    // §263
    pub fn sprint_cs(&mut self, mut p: halfword) {
        if (p < 514i32) {
            if (p < 257i32) {
                self.print((p).wrapping_sub(1i32));
            } else {
                if (p < 513i32) {
                    self.print_esc((p).wrapping_sub(257i32));
                } else {
                    {
                        self.print_esc(504i32);
                        self.print_esc(505i32);
                    }
                }
            }
        } else {
            self.print_esc(self.hash[((p) - 514) as usize].rh());
        }
    }

    /// Conversely, here is a routine that takes three strings and prints a file
    /// name that might have produced them. (The routine is system dependent, because
    /// some operating systems put the file area last instead of first.)
    /// @<Basic printing...
    // §518
    pub fn print_file_name(&mut self, mut n: i32, mut a: i32, mut e: i32) {
        self.slow_print(a);
        self.slow_print(n);
        self.slow_print(e);
    }

    /// \[35] Subroutines for math mode.
    /// In order to convert mlists to hlists, i.e., noads to nodes, we need several
    /// subroutines that are conveniently dealt with now.
    /// Let us first introduce the macros that make it easy to get at the parameters and
    /// other font information. A size code, which is a multiple of 16, is added to a
    /// family number to get an index into the table of internal font numbers
    /// for each combination of family and size.  (Be alert: Size codes get
    /// larger as the type gets smaller.)
    // §699
    pub fn print_size(&mut self, mut s: i32) {
        if (s == 0i32) {
            self.print_esc(412i32);
        } else {
            if (s == 16i32) {
                self.print_esc(413i32);
            } else {
                self.print_esc(414i32);
            }
        }
    }

    /// Each new type of node that appears in our data structure must be capable
    /// of being displayed, copied, destroyed, and so on. The routines that we
    /// need for write-oriented whatsits are somewhat like those for mark nodes;
    /// other extensions might, of course, involve more subtlety here.
    /// @<Basic printing...
    // §1355
    pub fn print_write_whatsit(&mut self, mut s: str_number, mut p: halfword) {
        self.print_esc(s);
        if (self.mem[((p).wrapping_add(1i32)) as usize].hh().lh() < 16i32) {
            self.print_int(self.mem[((p).wrapping_add(1i32)) as usize].hh().lh());
        } else {
            if (self.mem[((p).wrapping_add(1i32)) as usize].hh().lh() == 16i32) {
                self.print_char(42i32);
            } else {
                self.print_char(45i32);
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
                                self.print(264i32);
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
                                                self.help_line[(1i32) as usize] = 279i32;
                                                self.help_line[(0i32) as usize] = 280i32;
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
                                                self.print_nl(265i32);
                                                self.slow_print(self.input_stack[(self.base_ptr) as usize].name_field);
                                                self.print(266i32);
                                                self.print_int(self.line);
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
                                                        self.help_line[(1i32) as usize] = 281i32;
                                                        self.help_line[(0i32) as usize] = 282i32;
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
                                            self.help_line[(3i32) as usize] = 283i32;
                                            self.help_line[(2i32) as usize] = 282i32;
                                            self.help_line[(1i32) as usize] = 284i32;
                                            self.help_line[(0i32) as usize] = 285i32;
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
                                                    self.print(278i32);
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
                                        self.print(273i32);
                                        match c {
                                            81 => {
                                                {
                                                    self.print_esc(274i32);
                                                    self.selector = (self.selector).wrapping_sub(1i32);
                                                }
                                            }
                                            82 => {
                                                self.print_esc(275i32);
                                            }
                                            83 => {
                                                self.print_esc(276i32);
                                            }
                                            _ => {}
                                        }
                                        self.print(277i32);
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
                                self.print(267i32);
                                self.print_nl(268i32);
                                self.print_nl(269i32);
                                if (self.base_ptr > 0i32) {
                                    if (self.input_stack[(self.base_ptr) as usize].name_field >= 256i32) {
                                        self.print(270i32);
                                    }
                                }
                                if self.deletions_allowed {
                                    self.print_nl(271i32);
                                }
                                self.print_nl(272i32);
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
                    self.print_nl(263i32);
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
            self.print_nl(262i32);
            self.print(287i32);
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
            self.print_nl(262i32);
            self.print(288i32);
        }
        self.print(s);
        self.print_char(61i32);
        self.print_int(n);
        self.print_char(93i32);
        {
            self.help_ptr = 2i32;
            self.help_line[(1i32) as usize] = 289i32;
            self.help_line[(0i32) as usize] = 290i32;
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
                    self.print_nl(262i32);
                    self.print(291i32);
                }
                self.print(s);
                self.print_char(41i32);
                {
                    self.help_ptr = 1i32;
                    self.help_line[(0i32) as usize] = 292i32;
                }
            }
        } else {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(293i32);
                }
                {
                    self.help_ptr = 2i32;
                    self.help_line[(1i32) as usize] = 294i32;
                    self.help_line[(0i32) as usize] = 295i32;
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
            self.overflow(258i32, (max_strings).wrapping_sub(self.init_str_ptr));
        }
        self.str_ptr = (self.str_ptr).wrapping_add(1i32);
        { let __ix19 = self.str_ptr; let __v20 = self.pool_ptr; self.str_start[(__ix19) as usize] = __v20; }
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
                        if ((k < 32i32) || (k > 126i32)) {
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
            if { let mut __f = ::core::mem::take(&mut self.pool_file); let __r = self.a_open_in(&mut __f); self.pool_file = __f; __r } {
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
                                    { let mut __f = ::core::mem::take(&mut self.pool_file); let __r = self.a_close(&mut __f); self.pool_file = __f; __r };
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
                                                        { let mut __f = ::core::mem::take(&mut self.pool_file); let __r = self.a_close(&mut __f); self.pool_file = __f; __r };
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
                                    if (a != 504454778i32) {
                                        {
                                            {
                                                crate::system::wr_str(&mut self.term_out, "! TEX.POOL doesn't match; TANGLE me again.");
                                                crate::system::wr_ln(&mut self.term_out);
                                            }
                                            { let mut __f = ::core::mem::take(&mut self.pool_file); let __r = self.a_close(&mut __f); self.pool_file = __f; __r };
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
                                            { let mut __f = ::core::mem::take(&mut self.pool_file); let __r = self.a_close(&mut __f); self.pool_file = __f; __r };
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
                                            { let mut __f = ::core::mem::take(&mut self.pool_file); let __r = self.a_close(&mut __f); self.pool_file = __f; __r };
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
                                                    { let __ix21 = self.pool_ptr; let __v22 = self.xord[(m) as usize]; self.str_pool[(__ix21) as usize] = __v22; }
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
                    { let mut __f = ::core::mem::take(&mut self.pool_file); let __r = self.a_close(&mut __f); self.pool_file = __f; __r };
                    get_strings_started = true;
                }
            } else {
                {
                    {
                        crate::system::wr_str(&mut self.term_out, "! I can't read TEX.POOL.");
                        crate::system::wr_ln(&mut self.term_out);
                    }
                    { let mut __f = ::core::mem::take(&mut self.pool_file); let __r = self.a_close(&mut __f); self.pool_file = __f; __r };
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
            j = self.str_start[(260i32) as usize];
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
        if (!{ let mut __f = ::core::mem::take(&mut self.term_in); let __r = self.input_ln(&mut __f, true); self.term_in = __f; __r }) {
            self.fatal_error(261i32);
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
        self.print(286i32);
        self.print_int(n);
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
                    self.print_nl(262i32);
                    self.print(296i32);
                }
                {
                    self.help_ptr = 3i32;
                    self.help_line[(2i32) as usize] = 297i32;
                    self.help_line[(1i32) as usize] = 298i32;
                    self.help_line[(0i32) as usize] = 299i32;
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
        self.print_int((s / 65536i32));
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
    // §292
    pub fn show_token_list(&mut self, mut p: i32, mut q: i32, mut l: i32) {
        let mut m: i32 = 0; // §292
        let mut c: i32 = 0; // §292
        let mut match_chr: ASCII_code = 0; // §292
        let mut n: ASCII_code = 0; // §292
        'l_exit_f: {
            match_chr = 35i32;
            n = 48i32;
            self.tally = 0i32;
            while ((p != 0i32) && (self.tally < l)) {
                {
                    if (p == q) {
                        // §320
                        {
                            self.first_count = self.tally;
                            self.trick_count = (((self.tally).wrapping_add(1i32)).wrapping_add(error_line)).wrapping_sub(half_error_line);
                            if (self.trick_count < error_line) {
                                self.trick_count = error_line;
                            }
                        }
                    }
                    // §293
                    if ((p < self.hi_mem_min) || (p > self.mem_end)) {
                        {
                            self.print_esc(309i32);
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
                                self.print_esc(555i32);
                            } else {
                                // §294
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
                                        self.print(556i32);
                                    }
                                    _ => {
                                        self.print_esc(555i32);
                                    }
                                }
                            }
                        }
                    }
                    // §292
                    p = self.mem[(p) as usize].hh().rh();
                }
            }
            if (p != 0i32) {
                self.print_esc(554i32);
            }
        }
    }

    /// Here is a procedure that uses `scanner_status` to print a warning message
    /// when a subfile has ended, and at certain other crucial times:
    /// @<Declare the procedure called `runaway`
    // §306
    pub fn runaway(&mut self) {
        let mut p: halfword = 0; // §306
        if (self.scanner_status > 1i32) {
            {
                self.print_nl(569i32);
                match self.scanner_status {
                    2 => {
                        {
                            self.print(570i32);
                            p = self.def_ref;
                        }
                    }
                    3 => {
                        {
                            self.print(571i32);
                            p = 4999996i32;
                        }
                    }
                    4 => {
                        {
                            self.print(572i32);
                            p = 4999995i32;
                        }
                    }
                    5 => {
                        {
                            self.print(573i32);
                            p = self.def_ref;
                        }
                    }
                    _ => {}
                }
                self.print_char(63i32);
                self.print_ln();
                self.show_token_list(self.mem[(p) as usize].hh().rh(), 0i32, (error_line).wrapping_sub(10i32));
            }
        }
    }

}
